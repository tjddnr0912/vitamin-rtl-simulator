//! deferred hierarchical READ resolution — whole-net reads, element/bit/part selects,
//! and hierarchical FUNCTION calls.
//!
//! Split from `hier_defer.rs` (R17) to hold every module under the 1000-line cap.
//! The record types live in `mod.rs`; the write side and its guards in `write.rs`.

use super::*;

impl Elaborator<'_> {
    /// T1-6/7/8: the flat WORD index for a hierarchical element access on a DYNAMIC
    /// container (`u.d[i]`, `u.q[i]`, `u.s[i]`, `u.s[i][j]`), or `None` to decline.
    ///
    /// ONE addressing rule shared by the deferred READ and the deferred WRITE. They are
    /// two separate resolution passes over two separate sentinel spaces, and the whole
    /// hazard of admitting the write is that it addresses a DIFFERENT element than the
    /// read of the same source text — so the rule is stated once, here, and both call it.
    ///
    /// A ROUTED string array maps its DECLARED index space onto the flat container
    /// through `flatten_word_eids`, the pre-lowered-eid twin of the `flatten_word` the
    /// LOCAL funnel uses. Same arithmetic, so `u.s[i][j]` and a local `s[i][j]` cannot
    /// select different elements. A partial index (fewer eids than dims) declines rather
    /// than reading a row number as an element number.
    ///
    /// A plain dyn array / queue takes the ONE-index positional rule: its position IS
    /// its index and it has no other geometry.
    ///
    /// T1-10: an ASSOC array takes the same ONE-index shape for a different reason — its
    /// single index is a KEY, and the engine already dispatches on that. The chunk /
    /// `Signal` carries a word EID either way; `resolve_lvalue_offsets` re-reads it as an
    /// `AssocKey` when `is_assoc(net)`, and the element read does the same. So the
    /// keyed-vs-positional distinction is settled downstream by the NET, never by how the
    /// name was reached — which is why the local `a[k]` and this share one spelling.
    /// (No oracle: iverilog 13.0 cannot parse `int a[int]` at all. The pin is the
    /// vita-internal equivalence — `u.a[k]` must read what the child's own `a[k]` reads.)
    pub(crate) fn hier_dyn_container_word(&mut self, net: u32, idx_eids: &[u32]) -> Option<u32> {
        if !self.is_dyn_handle_net(net) {
            return None;
        }
        match self.fixed_string_dyn_geom(net).map(|g| g.extents.clone()) {
            Some(ext) if ext.len() == idx_eids.len() => Some(self.flatten_word_eids(
                &ext,
                idx_eids,
                &[],
                crate::array_geom::IndexDomain::ArrayWord,
            )),
            Some(_) => None,
            None => (idx_eids.len() == 1).then(|| idx_eids[0]),
        }
    }

    /// Resolve the N3.1 deferred hierarchical INDEXED reads (`dut.mem[i]`). The index
    /// is already lowered (`idx_eid`, with the original lowering context); here we
    /// resolve the base net and build the correct select FROM ITS SHAPE around that
    /// index — a single-dim unpacked array → element word (`net[idx-lo]`), a scalar/
    /// vector → bit-select — and overwrite the placeholder. A multi-dim packed net, a
    /// dynamic handle / event source, a multi-dim array (single index = partial
    /// slice), or an unresolved name is loud-rejected (those element selects are
    /// deferred follow-ons).
    pub(crate) fn resolve_deferred_hier_sel(&mut self) {
        let pending = std::mem::take(&mut self.deferred_hier_sel);
        for d in pending {
            // T1-6: a ROUTED string array is registered under a mangled net name, so the
            // symbol table cannot find it by the declared one — the same side map the
            // local resolver consults is checked here, with the same commit-to-scope
            // walk. Second, so an ordinary net of that name still wins.
            let Some(net) = self
                .hier_lookup(&d.prefix, &d.path)
                .or_else(|| self.hier_resolve(&d.prefix, &d.path, &self.fixed_string_dyn_key))
            else {
                self.error(
                    MsgCode::ElabUnresolvedName,
                    &format!(
                        "undeclared hierarchical name `{}` (no such cross-instance net)",
                        d.path.join(".")
                    ),
                );
                continue;
            };
            // T1-6/7: an indexed dynamic container (`u.s[0]` on a dyn array, a queue, or
            // a routed string array) reads as the word-indexed `Signal` the local
            // `dyn_select_read` builds — the engine's element read does not care how the
            // name was reached. `hier_dyn_container_word` is the shared addressing rule;
            // the WRITE twin calls the SAME function.
            if d.part.is_none() {
                if let Some(word) = self.hier_dyn_container_word(net, &d.idx_eids) {
                    self.exprs[d.eid as usize] = ir::Expr::Signal {
                        net,
                        word: Some(word),
                    };
                    continue;
                }
            }
            // Events and dynamic handles have no indexable readable value here (a dyn
            // element read routes through `dyn_select_read` at lowering, on a 1-seg
            // base — never a hierarchical ref). Loud-reject.
            if self.event_nets.contains(&net) || self.is_dyn_handle_net(net) {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "a hierarchical indexed read of `{}` is unsupported (an event or a \
                         dynamic-storage handle has no plain indexable value)",
                        d.path.join(".")
                    ),
                );
                continue;
            }
            let path = d.path.join(".");
            // HIER-REST-PS (read): a trailing PART-select whose offset is normalized
            // against the (element/net) LSB — the READ twin of the write-side `part`
            // branch. Handled first; the bit/whole lanes below run only when absent.
            if let Some(p) = d.part {
                if let Some(e) = self.build_hier_read_part(net, &d.idx_eids, p, &path) {
                    let ex = self.exprs[e as usize].clone();
                    self.exprs[d.eid as usize] = ex;
                }
                continue;
            }
            let built = if self.net_is_static_array(net) {
                // Unpacked array element — single- OR multi-dim, mirroring the local
                // `lower_array_read` path so `dut.grid[i][j]` reads exactly what a local
                // `grid[i][j]` would. A partial slice (< D indices) and a bit-of-bit
                // (> D+1) stay loud.
                match self.build_hier_array_read(net, &d.idx_eids, &path) {
                    Some(e) => e,
                    None => continue,
                }
            } else if self.packed_dims.contains_key(&net) {
                // Multi-dim PACKED element → a bit-slice (mirrors `lower_packed_read`).
                match self.build_hier_packed_read(net, &d.idx_eids, &path) {
                    Some(e) => e,
                    None => continue,
                }
            } else {
                // scalar / vector → a plain bit-select (mirrors the BitSelect arm). A
                // multi-index chain on a scalar/vector is a bit-of-bit → loud.
                if d.idx_eids.len() != 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "too many indices in hierarchical read of `{path}` (a scalar/vector \
                             takes a single bit-select)"
                        ),
                    );
                    continue;
                }
                let base_sig = self.push_expr(ir::Expr::Signal { net, word: None });
                let offset = self.norm_offset_for_net(net, d.idx_eids[0]);
                let width = self.const_u32_expr(1, 32);
                self.push_expr(ir::Expr::Select {
                    base: base_sig,
                    offset,
                    width,
                    kind: ir::SelKind::Bit,
                })
            };
            let e = self.exprs[built as usize].clone();
            self.exprs[d.eid as usize] = e;
        }
    }

    /// Resolve the N3 deferred hierarchical READ references against the completed
    /// `symbols` table and patch each placeholder `Signal` expr to the real NetId.
    /// Resolution walks the lowering-time scope prefix DOWNWARD (`prefix.path`) then
    /// strips it OUTWARD (sibling / ancestor scopes) and finally tries the path as an
    /// ABSOLUTE root-relative name — the first hit wins (downward preferred, the common
    /// `tb` → `dut.x` case). An unresolved name, or a resolved net that has no readable
    /// whole value (a named event, a dynamic-storage handle, or a whole unpacked array),
    /// is loud-rejected; the placeholder stays `POISON_NET`.
    pub(crate) fn resolve_deferred_hier(&mut self) {
        let deferred = std::mem::take(&mut self.deferred_hier);
        let ambient = self.cur_span;
        for d in deferred {
            self.cur_span = d.span.or(ambient);
            let Some(net) = self.hier_lookup(&d.prefix, &d.path) else {
                // Not a net — maybe a hierarchical PARAMETER read (`dut.WIDTH`),
                // which folds to a const in this runtime/expression context (a
                // const-context hierarchical param is loud — the sibling instance
                // is not yet elaborated when the const-eval needs the value).
                // ⚠️ A `parameter real` is NOT served here. Patching this placeholder
                // with a real constant makes the REAL-domain reads right (`a.P/2`) and
                // the INTEGRAL consumers silently wrong: `lower_cast` already decided
                // `int'(a.P)` was integral when the operand was still a placeholder, so
                // the patched const is read as bits (`int'` → 0, `longint'` → the IEEE-754
                // word, while `integer'` happens to survive). That is the leaf-vs-context
                // conversion trap, it is pre-existing for a hierarchical real VARIABLE
                // (`a.rv`, identical in PRE), and trading the loud below for it would
                // swap one silent-wrong for another. ROADMAP §2 owns the axis.
                if let Some(v) = self.hier_lookup_param(&d.prefix, &d.path) {
                    let meta = self.hier_lookup_param_meta(&d.prefix, &d.path);
                    self.patch_expr_param_const_w(d.eid, v, meta);
                } else {
                    self.error(
                        MsgCode::ElabUnresolvedName,
                        &format!(
                            "undeclared hierarchical name `{}` (no such cross-instance \
                             net or parameter)",
                            d.path.join(".")
                        ),
                    );
                }
                continue;
            };
            // Read-context guards (mirror the single-net Ident arm): these constructs
            // have no plain readable whole value, AND — critically — element/array
            // selects on a hierarchical base do NOT route through `expr_array_chain`/
            // `expr_packed_chain` (those require a single-segment Ident), so a
            // `dut.mem[i]` or multi-dim packed `dut.pm[i]` select would mis-lower to a
            // flat bit-select (review N3 HIGH: silent wrong value/width on packed
            // multi-dim). Loud-reject the whole net here; a hierarchical element select
            // is a deferred follow-on lane.
            // HIER-REST-MP: a WHOLE multi-dim packed net IS a plain flat vector — its
            // whole-net read (`dut.pm`) reads the flat value, so it is NOT rejected
            // here. (An element select `dut.pm[i]` takes the deferred-sel lane, never
            // this whole-net path.) Events / dyn handles / whole unpacked arrays still
            // have no plain whole value.
            // R17 (IEEE 1800 §23.9): an `automatic` variable has no static address, so
            // it cannot be named hierarchically at all. v1's flatten gives it a module
            // net, which is exactly how a cross-module `tb.a` came to resolve to per-
            // entry storage and read it from outside.
            if self.hier_ref_to_automatic_local(net, &d.path.join("."), "read") {
                continue;
            }
            // §3 ④: in a `$readmem*`/`$writemem*` MEMORY position a whole unpacked array
            // is the OPERAND, so the whole-array arm below does not apply — the task
            // wants the array, not a value. Events and dynamic handles stay rejected in
            // every position: those have no array to hand over either.
            let mem_arg = self.hier_mem_args.get(&d.eid).copied();
            // V34-5: in a `%p` ARGUMENT position the aggregate is the operand, the
            // same exemption `$readmem*`'s memory position gets one line above and
            // for the same reason. The dyn-handle arm is exempt too here (unlike
            // the readmem one) because `builtins::pattern` renders a queue / dyn
            // array / assoc from `dyn_heap`, which is keyed by NET id and therefore
            // already per-instance — a hierarchical handle needs nothing extra.
            let pattern_arg = self.hier_pattern_args.contains(&d.eid);
            let bad = self.event_nets.contains(&net)
                || (self.is_dyn_handle_net(net) && !pattern_arg)
                || (self.net_is_static_array(net) && mem_arg.is_none() && !pattern_arg);
            if bad {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "hierarchical read of `{}` is unsupported (a named event, a \
                         dynamic handle, or a whole unpacked array has no plain readable \
                         value; a hierarchical element select is a deferred follow-on)",
                        d.path.join(".")
                    ),
                );
                continue;
            }
            // A2a twin: `$readmem*` WRITES the memory, so a desugared const
            // array-parameter target is loud here exactly as it is on the local arm.
            // `$writemem*` only READS it and passes — the local arm restricts the check
            // the same way, and running it for the whole family would false-loud a
            // legitimate `$writememh(f, dut.some_param_array)`.
            if mem_arg == Some(true) {
                self.deny_const_param_write(net, "$readmem into");
            }
            // The one-element-array refusal `lower_pattern_arg` makes locally, made
            // here instead — the local site cannot make it, because a cross-instance
            // net does not exist until this pass. Same reason, same message: the
            // engine sees only `array_len`, which is 1 for a scalar too, so rendering
            // it would drop the assignment-pattern braces at exit 0.
            if pattern_arg
                && self.net_is_static_array(net)
                && self.nets[net as usize].array_len <= 1
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "`%p` of the ONE-ELEMENT unpacked array `{}` is unsupported: \
                         `sim_ir::NetVar` records only `array_len`, which is 1 for a \
                         scalar too, so the renderer cannot tell the two apart and would \
                         print the element without its assignment-pattern braces (index \
                         the element instead)",
                        d.path.join(".")
                    ),
                );
                continue;
            }
            if let Some(ir::Expr::Signal { net: slot, .. }) = self.exprs.get_mut(d.eid as usize) {
                *slot = net;
            }
        }
        self.cur_span = ambient;
    }

    /// Family D (r17): patch each deferred hierarchical function call `u1.f(x)` to the
    /// callee's per-instance FuncId, once every instance's frame funcs are reserved.
    /// `hier_resolve` commits the leading segments to the callee instance scope (the same
    /// §23.6 walk the net/param resolvers use) and looks up `<inst>.<fname>` in
    /// `hier_funcs` (which holds ONLY hier-callable framed functions). An unresolved
    /// target — a plain/inlined function, an output/array/string formal, or a bad
    /// instance path — is loud (correct-or-loud); the POISON_FID placeholder never
    /// survives (the whole IR is discarded on `had_error`).
    pub(crate) fn resolve_deferred_hier_call(&mut self) {
        let deferred = std::mem::take(&mut self.deferred_hier_calls);
        for d in deferred {
            let Some(fid) = self.hier_resolve(&d.prefix, &d.path, &self.hier_funcs) else {
                // ⭐ An ENUM METHOD lands here, and the generic wording described a
                // DIFFERENT feature: `x.name()` is not a hierarchical call at all, and
                // telling its author about "framed functions with input-only scalar
                // formals reached through an instance path" sent them looking at the
                // wrong construct entirely. It arrives here because `x`'s enum type was
                // never registered, so the parser's method desugar did not fire and the
                // two-segment name fell through to this resolver.
                //
                // V33-3: and the receiver test had to be widened, because it asked
                // `symbols` — which holds NETS ONLY. An enum LABEL is a constant, never
                // a net, so `LA.name()` (label declared in this module, or wildcard-
                // imported from a package) missed this arm entirely and took the
                // hierarchical-call wording. The label case also needs its OWN message:
                // the "enum type was not registered" sentence is FALSE for a label —
                // the enum IS registered, which is exactly why `mv.name()` works on a
                // variable of that type. Measured: both oracles reject the label form
                // (iverilog 13 aborts in `elab_expr.cc:3297`, verilator 5.050 says
                // "Can't find definition of task/function: 'name'"), so this stays
                // loud; the job is to describe the right construct.
                let enum_method = d.path.len() == 2
                    && matches!(
                        d.path[1].as_str(),
                        "name" | "next" | "prev" | "first" | "last" | "num"
                    );
                // The receiver must resolve in the scope the CALL was lowered in —
                // `cur_prefix` has moved on by the time this pass runs, so keys are
                // rebuilt from the saved prefix rather than looked up ambiently.
                // Without that a genuine `u1.next(x)` could take these messages.
                let recv_key = if d.prefix.is_empty() {
                    d.path[0].clone()
                } else {
                    format!("{}.{}", d.prefix, d.path[0])
                };
                let recv_is_net = enum_method && self.symbols.contains_key(&recv_key);
                // A label binds under the scope that DECLARED it, which may enclose the
                // one the call was lowered in (a label declared in the module body, a
                // call inside a generate block). Walk outward over the saved prefix the
                // way `walk_scopes_key` walks over `cur_prefix`.
                let label_ty = if enum_method && !recv_is_net {
                    self.scoped_key_at_or_above(&d.prefix, &d.path[0], |k| {
                        self.enum_label_types.contains_key(k)
                    })
                    .and_then(|k| self.enum_label_types.get(&k).cloned())
                } else {
                    None
                };
                let msg = if let Some(ty) = label_ty {
                    format!(
                        "`{}()` calls an enum method on the enum LABEL `{}` (a named \
                         constant of `{ty}`), which vita does not support — and neither \
                         oracle accepts it either (iverilog aborts, verilator reports no \
                         such function). vita's enum methods (`name`/`next`/`prev`/\
                         `first`/`last`/`num`) work on a VARIABLE of the enum type: \
                         declare one and assign the label first — `{ty} v = {}; v.{}()`",
                        d.path.join("."),
                        d.path[0],
                        d.path[0],
                        d.path[1]
                    )
                } else if recv_is_net {
                    format!(
                        "enum method `{}` is unavailable: the enum type of `{}` was not \
                         registered, which happens when a label's value is not a \
                         parse-time constant (an overridable `parameter`, or a sized \
                         literal). Give the labels plain decimal `localparam` / literal \
                         values — the VALUES themselves are correct either way, it is \
                         only the methods that are lost",
                        d.path.join("."),
                        d.path[0]
                    )
                } else {
                    format!(
                        "unsupported hierarchical function call `{}` (the callee must be a \
                         framed function with input-only scalar formals and a non-string \
                         return, reached through an instance path)",
                        d.path.join(".")
                    )
                };
                self.error_at(MsgCode::ElabUnsupported, d.span, &msg);
                continue;
            };
            // Arity guard: the engine coerces actuals to formal widths BY INDEX, so a
            // wrong count would read past / drop formals (silent-wrong) — loud instead.
            let n_params = self.func_metas[fid as usize].n_params as usize;
            if n_params != d.argc {
                self.error_at(
                    MsgCode::ElabUnsupported,
                    d.span,
                    &format!(
                        "hierarchical call `{}` passes {} argument(s) but the function \
                         takes {}",
                        d.path.join("."),
                        d.argc,
                        n_params
                    ),
                );
                continue;
            }
            if let Some(ir::Expr::Call { func, .. }) = self.exprs.get_mut(d.eid as usize) {
                *func = fid;
            }
        }
    }
}
