//! deferred hier resolution — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// A concurrent assertion (`assert property(@(clk) ante |-> cons)`) collected
/// A hierarchical READ reference (`tb.dut.x`) deferred during expression lowering
/// (N3). The placeholder `Signal` at `eid` is patched to the resolved NetId after
/// all instances are elaborated; `prefix` is the scope path in effect at lowering
/// (so the resolver can walk downward → outward → absolute), `path` the dotted
/// segments.
pub(crate) struct DeferredHier {
    pub(crate) eid: u32,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
}

/// Family D (r17): a hierarchical FUNCTION call `u1.f(args)` whose callee lives in a
/// child instance not yet elaborated when the call site is lowered. Mirrors
/// [`DeferredHier`] (the net-read defer): the placeholder `Expr::Call { func: POISON_FID }`
/// at `eid` is patched to the callee's per-instance FuncId after all instances exist.
/// `prefix` = the caller's scope at lowering, `path` = the dotted segments (`[u1, f]`);
/// the last segment is the function name, the rest resolve to the callee instance scope.
pub(crate) struct DeferredHierCall {
    pub(crate) eid: u32,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    pub(crate) argc: usize,
}

/// Family D (r18): a hierarchical TASK enable `u1.tk(args);` whose callee lives in a
/// child instance not yet elaborated when the call site is lowered. The statement path
/// twin of [`DeferredHierCall`]. At the call site a placeholder `Terminator::Call` seals
/// the process block `(proc, call_block)` and the args are lowered in the CALLER scope;
/// `resolve_deferred_hier_task_call` builds the `TaskCallInfo{callee: per-instance fid}`
/// into `task_calls_proc[(proc, call_block)]` once every instance's frame tasks are in
/// `hier_tasks`. Restricted to SCALAR formals. §4.5.201: each arg is lowered BOTH as a
/// value (`arg_ids[i]`, the copy-IN for an input/inout formal) AND — when it is an lvalue
/// (a bare var / select) — as a caller-side lvalue (`arg_lvals[i]`, the copy-OUT target for
/// an output/inout formal); the callee port DIRECTION is unknown at the call site (the
/// instance isn't elaborated yet), so `resolve_deferred_hier_task_call` picks between them
/// per port using `hier_task_port_dirs[fid]`.
pub(crate) struct DeferredHierTaskCall {
    pub(crate) proc: u32,
    pub(crate) call_block: u32,
    /// §4.5.208: `Some(func_block)` when the enable is NESTED inside a frame TASK body (the
    /// placeholder `Call` lives in `func_blocks`, not a process) — the resolver patches
    /// `func_blocks[func_block].term` and inserts the `TaskCallInfo` into `task_calls_func`
    /// (not `task_calls_proc`). `None` ⇒ a top-level process enable (`proc`/`call_block`).
    /// The block id is process-LOCAL at emit and rebased by `+base` at frame-body finish
    /// (`lower_frame_task_body`), exactly like `pending_task_calls`.
    pub(crate) func_block: Option<u32>,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    pub(crate) arg_ids: Vec<u32>,
    pub(crate) arg_lvals: Vec<Option<ir::Lvalue>>,
    /// §4.5.207: per-arg, `Some(net)` when the actual is a bare whole-array Ident (a static
    /// array net, resolved in the CALLER scope at defer time — a whole array cannot be lowered
    /// to a value). At resolve, if the callee formal is an INPUT array (`frame_arr_formal_meta`)
    /// the net is packed into the md-packed slot (`pack_hier_array_actual`); `None` otherwise.
    /// An array formal fed a non-array actual (or vice versa), or an OUTPUT/INOUT array formal,
    /// stays loud there (correct-or-loud).
    pub(crate) arg_arrays: Vec<Option<u32>>,
}

/// A hierarchical WRITE target (`tb.dut.x = …`) whose net does not exist when the
/// lvalue is lowered. The `LvalChunk` is emitted with the sentinel net
/// `HIER_WRITE_SENTINEL_BASE + index-in-this-Vec`; `resolve_deferred_hier_write`
/// resolves `prefix`/`path` (same IEEE §23.6 walk as the read side, via
/// `hier_lookup`), applies the write-context guards (no event/dyn/array/packed
/// whole-net, no procedural write to a `wire`), and patches every matching chunk
/// in the statement arena to the real NetId (or `POISON_NET` on error). Whole-net
/// only — a hierarchical element/part-select write stays a loud follow-on.
pub(crate) struct DeferredHierWrite {
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
}

pub(crate) struct DeferredHierSelWrite {
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    pub(crate) idx_eids: Vec<u32>,
    /// `Some` for a hierarchical PART-select write (`dut.v[3:0]=…`, `dut.v[o+:w]=…`,
    /// or array-element `dut.mem[i][3:0]=…`): the (already-lowered) raw offset edge,
    /// the width edge, and the select kind. `None` for an element / bit-select write.
    pub(crate) part: Option<HierPart>,
}

/// A hierarchical INDEXED read `base[i]…[k]` whose `base` is a 2-segment hierarchical
/// reference (slice N3.1 + multi-dim follow-on). Deferred like [`DeferredHier`] — the
/// base net does not exist at lowering time — but resolution must choose the SELECT
/// KIND and arity from the resolved net's shape (a single-/multi-dim unpacked array
/// element word, a multi-dim packed bit-slice, or a vector bit-select), which is only
/// known after elaboration. EVERY index is LOWERED AT LOWERING TIME (with the full
/// param/genvar/function-formal context) into `idx_eids`; the fixup only builds the
/// flat-word/offset arithmetic and select around those eids (review N3.1: re-lowering
/// an index at fixup lost that context — a function-formal index silently resolved to
/// a shadowing outer net).
pub(crate) struct DeferredHierSelect {
    pub(crate) eid: u32,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    /// Indices in SOURCE order (`grid[i][j]` → `[i, j]`). Length 1 = the N3.1 scalar/
    /// vector/single-dim-array case; ≥2 = a multi-dim unpacked/packed element select.
    pub(crate) idx_eids: Vec<u32>,
    /// A trailing PART-select (`dut.mem[i][m:l]`, scalar `dut.v[m:l]`, `[b+:w]`) whose
    /// offset is normalized against the (element/net) LSB at resolution time — the
    /// READ twin of [`DeferredHierSelWrite`]'s `part`. `None` = a bit-select / whole
    /// element read (the pre-existing lanes), so those stay byte-identical.
    pub(crate) part: Option<HierPart>,
}

impl Elaborator<'_> {
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
            let Some(net) = self.hier_lookup(&d.prefix, &d.path) else {
                self.error(
                    MsgCode::ElabUnresolvedName,
                    &format!(
                        "undeclared hierarchical name `{}` (no such cross-instance net)",
                        d.path.join(".")
                    ),
                );
                continue;
            };
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
        for d in deferred {
            let Some(net) = self.hier_lookup(&d.prefix, &d.path) else {
                // Not a net — maybe a hierarchical PARAMETER read (`dut.WIDTH`),
                // which folds to a const in this runtime/expression context (a
                // const-context hierarchical param is loud — the sibling instance
                // is not yet elaborated when the const-eval needs the value).
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
            let bad = self.event_nets.contains(&net)
                || self.is_dyn_handle_net(net)
                || self.net_is_static_array(net);
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
            if let Some(ir::Expr::Signal { net: slot, .. }) = self.exprs.get_mut(d.eid as usize) {
                *slot = net;
            }
        }
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
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "unsupported hierarchical function call `{}` (the callee must be a \
                         framed function with input-only scalar formals and a non-string \
                         return, reached through an instance path)",
                        d.path.join(".")
                    ),
                );
                continue;
            };
            // Arity guard: the engine coerces actuals to formal widths BY INDEX, so a
            // wrong count would read past / drop formals (silent-wrong) — loud instead.
            let n_params = self.func_metas[fid as usize].n_params as usize;
            if n_params != d.argc {
                self.error(
                    MsgCode::ElabUnsupported,
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

    /// Family D (r18): build each deferred hierarchical TASK enable's `TaskCallInfo` — the
    /// callee's per-instance frame-TASK FuncId + positional input binds — into
    /// `task_calls_proc`, once every instance's frame tasks are in `hier_tasks`.
    /// `hier_resolve` commits the leading segments to the callee instance scope (the same
    /// §23.6 walk the net/param/func resolvers use) and looks up `<inst>.<tname>`. An
    /// unresolved target — a non-framed (static) task, an output/inout/array/string
    /// formal, or a bad instance path — is loud (correct-or-loud). The placeholder
    /// `Terminator::Call.target` is patched to the callee entry (faithful IR; the process
    /// executor resolves the callee via `task_calls_proc`, so `target` is otherwise
    /// unread for a process-body call).
    pub(crate) fn resolve_deferred_hier_task_call(&mut self) {
        let deferred = std::mem::take(&mut self.deferred_hier_task_calls);
        for d in deferred {
            let Some(fid) = self.hier_resolve(&d.prefix, &d.path, &self.hier_tasks) else {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "unsupported hierarchical task call `{}` (the callee must be a framed \
                         task reached through an instance path, with scalar or fixed \
                         unpacked-array formals — no string / dynamic-array formal)",
                        d.path.join(".")
                    ),
                );
                continue;
            };
            // Arity guard: the engine binds actuals to formal slots BY INDEX, so a wrong
            // count would read past / drop formals (silent-wrong) — loud instead.
            let n_params = self.func_metas[fid as usize].n_params as usize;
            if n_params != d.arg_ids.len() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "hierarchical task call `{}` passes {} argument(s) but the task takes {}",
                        d.path.join("."),
                        d.arg_ids.len(),
                        n_params
                    ),
                );
                continue;
            }
            // §4.5.201: route each arg by the callee port DIRECTION (`hier_task_port_dirs`,
            // stored when the task was registered — the callee def is not in scope here).
            // INPUT → copy-in (value); OUTPUT → copy-out (caller lvalue); INOUT → both. A
            // non-lvalue output/inout actual is loud (correct-or-loud). `dirs.len()` ==
            // `arg_ids.len()` after the arity guard (both are the callee's formal count).
            let dirs = self
                .hier_task_port_dirs
                .get(&fid)
                .cloned()
                .unwrap_or_default();
            let base_net = self.func_metas[fid as usize].base_net;
            let mut in_binds: Vec<(u32, u32)> = Vec::new();
            let mut out_binds: Vec<(u32, ir::Lvalue)> = Vec::new();
            // item 1: (caller-array net, packed-temp net, formal shape) for every OUTPUT/INOUT
            // array formal — the copy-OUT is UNPACKED into the caller-array elements at the
            // front of the ret block after the loop (the deferred twin of the local §4.5.204).
            let mut out_array_unpacks: Vec<(u32, u32, ArrayFormal)> = Vec::new();
            let mut bind_err = false;
            for (i, dir) in dirs.iter().enumerate() {
                // §4.5.207: the callee formal `i` is the per-instance net `base_net + i`. If it
                // is an md-packed ARRAY slot (`frame_arr_formal_meta`), the arg must be a bare
                // whole-array actual (`arg_arrays[i]`) packed into the slot — INPUT only (an
                // output/inout array copy-out over a hier enable is a deferred follow-on).
                let callee_af = self
                    .frame_arr_formal_meta
                    .get(&(base_net + i as u32))
                    .cloned();
                if let Some(af) = callee_af {
                    // An array formal needs a bare whole-array actual — a static array net
                    // resolved in the caller scope at defer time (`arg_arrays[i]`). A frame-LOCAL
                    // array actual is md-packed (not a static array) ⇒ `None` ⇒ loud (ROADMAP
                    // follow-on #2: forwarding a frame formal array through a nested enable).
                    let Some(caller_net) = d.arg_arrays[i] else {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "hierarchical task call `{}`: an array formal needs a bare \
                                 whole-array actual",
                                d.path.join(".")
                            ),
                        );
                        bind_err = true;
                        continue;
                    };
                    // Shared shape gate — the copy-IN and copy-OUT agree on "matching".
                    if !self.hier_array_shape_ok(caller_net, &af) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "hierarchical task call `{}`: the array argument's shape does \
                                 not match the formal",
                                d.path.join(".")
                            ),
                        );
                        bind_err = true;
                        continue;
                    }
                    match dir {
                        // INPUT (§4.5.207): pack the whole-array actual into the callee's
                        // md-packed slot value (copy-in, IEEE §13.5.1 pass-by-value).
                        ast::PortDir::Input => {
                            if let Some(eid) = self.pack_hier_array_actual(caller_net, &af) {
                                in_binds.push((i as u32, eid));
                            }
                        }
                        // OUTPUT / INOUT (item 1): the body writes the md-packed slot; at exit the
                        // engine copies the WHOLE slot to a fresh packed temp (a scalar out-bind),
                        // then the ret-block UNPACK (built after this loop) writes it into the
                        // caller-array elements — the resolve-time twin of the local §4.5.193/204
                        // copy-out. INOUT additionally PACKS the caller array INTO the slot at
                        // entry (a normal in-bind, §4.5.207). Hand-IEEE §13.5.2 pass-by-value-
                        // result (iverilog rejects unpacked subroutine array ports).
                        ast::PortDir::Output | ast::PortDir::Inout => {
                            self.deny_const_param_write(
                                caller_net,
                                "connect an output/inout array to",
                            );
                            if matches!(dir, ast::PortDir::Inout) {
                                if let Some(eid) = self.pack_hier_array_actual(caller_net, &af) {
                                    in_binds.push((i as u32, eid));
                                }
                            }
                            let w = af.count.saturating_mul(af.elem_w).max(1);
                            let packed_net = self.nets.len() as u32;
                            let pname = format!("__houtpack${}${packed_net}", d.path.join("$"));
                            self.add_net(
                                &pname,
                                ir::NetVar {
                                    kind: ir::NetKind::Reg,
                                    width: w,
                                    msb: w.saturating_sub(1),
                                    lsb: 0,
                                    signed: false,
                                    array_len: 1,
                                    dir: ir::PortDir::Internal,
                                    init: default_init(ast::NetVarKind::Reg, w),
                                },
                            );
                            out_binds.push((i as u32, whole_net_lvalue(packed_net)));
                            out_array_unpacks.push((caller_net, packed_net, af));
                        }
                    }
                    continue;
                }
                // Scalar formal — a whole-array actual here is a type mismatch (loud).
                if d.arg_arrays[i].is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "hierarchical task call `{}`: a whole-array actual was passed to a \
                             scalar formal",
                            d.path.join(".")
                        ),
                    );
                    bind_err = true;
                    continue;
                }
                if matches!(dir, ast::PortDir::Input | ast::PortDir::Inout) {
                    in_binds.push((i as u32, d.arg_ids[i]));
                }
                if matches!(dir, ast::PortDir::Output | ast::PortDir::Inout) {
                    match &d.arg_lvals[i] {
                        Some(lv) => out_binds.push((i as u32, lv.clone())),
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "hierarchical task call `{}`: an output/inout argument must \
                                     be a writable net or select",
                                    d.path.join(".")
                                ),
                            );
                            bind_err = true;
                        }
                    }
                }
            }
            if bind_err {
                continue;
            }
            // item 1: build the copy-OUT unpack (`caller[i] = packed[i*ew +: ew]`) for every
            // OUTPUT/INOUT array formal — direct IR, no scope/`lower_stmt`. Position `i` ↔ the
            // caller's flat word `i` is the EXACT reverse of `pack_hier_array_actual` (which
            // packs the caller's flat word `i` at slot position `i`), so the round-trip is
            // correct by construction. Element signedness is the caller array's, applied on
            // later reads (bit-faithful copy here). Prepended to the ret block below.
            let mut unpack_sids: Vec<u32> = Vec::new();
            for (caller_net, packed_net, af) in &out_array_unpacks {
                for pos in 0..af.count {
                    let base_sig = self.push_expr(ir::Expr::Signal {
                        net: *packed_net,
                        word: None,
                    });
                    let off = self.const_u32_expr(pos.saturating_mul(af.elem_w), 32);
                    let wid = self.const_u32_expr(af.elem_w, 32);
                    let rhs = self.push_expr(ir::Expr::Select {
                        base: base_sig,
                        offset: off,
                        width: wid,
                        kind: ir::SelKind::PartIdxUp,
                    });
                    let word = self.const_u32_expr(pos, 32);
                    let lhs = ir::Lvalue {
                        chunks: vec![ir::LvalChunk {
                            net: *caller_net,
                            word: Some(word),
                            offset: None,
                            width: None,
                            kind: ir::SelKind::Bit,
                        }],
                    };
                    unpack_sids.push(self.push_stmt(ir::Stmt::BlockingAssign { lhs, rhs }));
                }
            }
            let info = TaskCallInfo {
                callee: fid,
                in_binds,
                out_binds,
            };
            let entry = self.funcs[fid as usize].entry;
            // §4.5.208: a NESTED-in-frame-body enable's placeholder `Call` lives in
            // `func_blocks` and is keyed into `task_calls_func`; a top-level process enable
            // uses `processes[proc]` + `task_calls_proc`. The engine reads the same two tables.
            // item 1: capture the `ret_bb` while patching the terminator, then PREPEND the
            // copy-out unpack to that block so it runs AFTER the task's exit (the out-bind wrote
            // the packed temp) and BEFORE any statement following the enable.
            if let Some(fb) = d.func_block {
                self.task_calls_func.insert(fb, info);
                let mut ret_bb = None;
                if let Some(blk) = self.func_blocks.get_mut(fb as usize) {
                    if let ir::Terminator::Call { target, ret_bb: rb } = &mut blk.term {
                        *target = entry;
                        ret_bb = Some(*rb);
                    }
                }
                if let (Some(rb), false) = (ret_bb, unpack_sids.is_empty()) {
                    if let Some(retblk) = self.func_blocks.get_mut(rb as usize) {
                        retblk.stmts.splice(0..0, unpack_sids);
                    }
                }
            } else {
                self.task_calls_proc.insert((d.proc, d.call_block), info);
                let mut ret_bb = None;
                if let Some(blk) = self
                    .processes
                    .get_mut(d.proc as usize)
                    .and_then(|p| p.body.get_mut(d.call_block as usize))
                {
                    if let ir::Terminator::Call { target, ret_bb: rb } = &mut blk.term {
                        *target = entry;
                        ret_bb = Some(*rb);
                    }
                }
                if let (Some(rb), false) = (ret_bb, unpack_sids.is_empty()) {
                    if let Some(retblk) = self
                        .processes
                        .get_mut(d.proc as usize)
                        .and_then(|p| p.body.get_mut(rb as usize))
                    {
                        retblk.stmts.splice(0..0, unpack_sids);
                    }
                }
            }
        }
    }

    /// Record a deferred hierarchical WRITE target and return its sentinel net id
    /// (`HIER_WRITE_SENTINEL_BASE + index`). The chunk carries this sentinel until
    /// `resolve_deferred_hier_write` patches it. Falls back to a loud `resolve_net`
    /// (→ POISON) only if the sentinel range is exhausted (≈16M deferred writes —
    /// unreachable in practice).
    pub(crate) fn defer_hier_write(&mut self, path: &ast::HierPath) -> u32 {
        let idx = self.deferred_hier_write.len() as u32;
        if idx >= POISON_NET - HIER_WRITE_SENTINEL_BASE {
            return self.resolve_net(path);
        }
        self.deferred_hier_write.push(DeferredHierWrite {
            prefix: self.cur_prefix.clone(),
            path: path.segments.iter().map(|s| s.name.clone()).collect(),
        });
        HIER_WRITE_SENTINEL_BASE + idx
    }

    /// Resolve the deferred hierarchical WRITE targets (`tb.dut.x = …`) once every
    /// instance's nets are in `symbols`. Mirrors `resolve_deferred_hier` (the read
    /// side) but patches `LvalChunk.net` across the statement arena: build a
    /// `sentinel → NetId` map (applying the write-context guards on the resolved
    /// net), then scan every lvalue-bearing statement and replace any sentinel
    /// chunk net. Whole-net only — a hierarchical element/part-select write never
    /// reaches here (it stays loud in the generic lvalue path).
    pub(crate) fn resolve_deferred_hier_write(&mut self) {
        let pending = std::mem::take(&mut self.deferred_hier_write);
        if pending.is_empty() {
            return;
        }
        let mut patch: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        for (i, d) in pending.iter().enumerate() {
            let sentinel = HIER_WRITE_SENTINEL_BASE + i as u32;
            let real = match self.hier_lookup(&d.prefix, &d.path) {
                None => {
                    self.error(
                        MsgCode::ElabUnresolvedName,
                        &format!(
                            "undeclared hierarchical write target `{}` (no such cross-instance net)",
                            d.path.join(".")
                        ),
                    );
                    POISON_NET
                }
                Some(net) => {
                    // N4 §14.3: a clocking INPUT is read-only — a HIERARCHICAL drive
                    // from a parent (`dut.cb.sig = v`) must be loud, NOT a silent write
                    // to the holding reg (the in-module guard at `collect_lval_chunks`
                    // cannot see a cross-instance name, so it is caught HERE on the
                    // resolved net).
                    if self.clocking_hold_nets.contains(&net) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "cannot drive a clocking INPUT (`cb.sig` is read-only, §14.3)",
                        );
                        POISON_NET
                    } else if self.const_param_nets.contains_key(&net) {
                        // A2a: a hierarchical write resolves AFTER lower_lvalue
                        // (sentinel chunk), so the funnel deny never saw the real
                        // net — enforce it here (`u1.RHO = …` incl. the [0:0]
                        // single-element shape that passes the static-array guard).
                        self.deny_const_param_write(net, "assign to");
                        POISON_NET
                    } else if self.event_nets.contains(&net)
                        || self.is_dyn_handle_net(net)
                        || self.net_is_static_array(net)
                    {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "hierarchical write of `{}` is unsupported (a named event, a \
                                 dynamic handle, or a whole unpacked array is not a plain \
                                 whole-net write target; a hierarchical element select is a \
                                 deferred follow-on)",
                                d.path.join(".")
                            ),
                        );
                        POISON_NET
                    } else if matches!(
                        self.nets.get(net as usize).map(|nv| &nv.kind),
                        Some(ir::NetKind::Wire)
                    ) {
                        // P1-9 (E3018) for the deferred path: a procedural hierarchical
                        // write may not target a `wire` (iverilog rejects it too).
                        self.error(
                            MsgCode::ElabLvalueKind,
                            &format!(
                                "procedural hierarchical write to net `{}` (declare it reg/logic)",
                                d.path.join(".")
                            ),
                        );
                        POISON_NET
                    } else {
                        net
                    }
                }
            };
            patch.insert(sentinel, real);
        }
        for s in &mut self.stmts {
            let chunks = match s {
                ir::Stmt::BlockingAssign { lhs, .. }
                | ir::Stmt::NonblockingAssign { lhs, .. }
                | ir::Stmt::Force { lhs, .. }
                | ir::Stmt::Release { lhs, .. } => &mut lhs.chunks,
                _ => continue,
            };
            for c in chunks {
                if let Some(&real) = patch.get(&c.net) {
                    c.net = real;
                }
            }
        }
    }

    /// Record a deferred hierarchical element/bit-select WRITE; returns the sentinel
    /// net the placeholder `LvalChunk` carries until `resolve_deferred_hier_sel_write`
    /// rebuilds it. Falls back to `POISON_NET` (loud downstream) if the sentinel range
    /// is exhausted (~16M deferred element writes — unreachable in practice).
    pub(crate) fn defer_hier_sel_write(
        &mut self,
        path: Vec<String>,
        idx_eids: Vec<u32>,
        part: Option<HierPart>,
    ) -> u32 {
        let idx = self.deferred_hier_sel_write.len() as u32;
        if idx >= HIER_WRITE_SENTINEL_BASE - HIER_SEL_WRITE_SENTINEL_BASE {
            return POISON_NET;
        }
        self.deferred_hier_sel_write.push(DeferredHierSelWrite {
            prefix: self.cur_prefix.clone(),
            path,
            idx_eids,
            part,
        });
        HIER_SEL_WRITE_SENTINEL_BASE + idx
    }

    /// Resolve the deferred hierarchical ELEMENT/bit-select WRITE targets
    /// (`dut.mem[i] = …`, `m.l.v[3] <= …`) once every instance's nets exist. The write
    /// twin of [`Self::resolve_deferred_hier_sel`]: resolve the base net, apply the
    /// write-context guards (no event / dynamic-handle source, no procedural write to a
    /// `wire`), REBUILD the full `LvalChunk` from the net's shape (array element word /
    /// packed bit-slice / vector bit), and replace every matching sentinel chunk in the
    /// statement arena. Runs BEFORE the multidriver scan so it sees real net ids.
    pub(crate) fn resolve_deferred_hier_sel_write(&mut self) {
        let pending = std::mem::take(&mut self.deferred_hier_sel_write);
        if pending.is_empty() {
            return;
        }
        let mut patch: std::collections::BTreeMap<u32, (ir::LvalChunk, String)> =
            std::collections::BTreeMap::new();
        for (i, d) in pending.into_iter().enumerate() {
            let sentinel = HIER_SEL_WRITE_SENTINEL_BASE + i as u32;
            let path = d.path.join(".");
            let chunk = match self.hier_lookup(&d.prefix, &d.path) {
                None => {
                    self.error(
                        MsgCode::ElabUnresolvedName,
                        &format!(
                            "undeclared hierarchical write target `{path}` (no such cross-instance net)"
                        ),
                    );
                    poison_chunk()
                }
                Some(net) => {
                    if self.clocking_hold_nets.contains(&net) {
                        // N4 §14.3: a clocking INPUT is read-only — a hierarchical
                        // SELECT drive (`dut.cb.sig[i] = v`) is loud too.
                        self.error(
                            MsgCode::ElabUnsupported,
                            "cannot drive a clocking INPUT (`cb.sig` is read-only, §14.3)",
                        );
                        poison_chunk()
                    } else if self.const_param_nets.contains_key(&net) {
                        // A2a: the deferred element/bit/part-select write lane —
                        // the funnel deny saw only the sentinel, so enforce it on
                        // the resolved net (`u1.RHO[0] = …` was a silent mutation).
                        self.deny_const_param_write(net, "assign to");
                        poison_chunk()
                    } else if self.event_nets.contains(&net) || self.is_dyn_handle_net(net) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "a hierarchical indexed write of `{path}` is unsupported (an event \
                                 or a dynamic-storage handle has no plain indexable write target)"
                            ),
                        );
                        poison_chunk()
                    } else if matches!(
                        self.nets.get(net as usize).map(|nv| &nv.kind),
                        Some(ir::NetKind::Wire)
                    ) {
                        // P1-9 (E3018): a procedural hierarchical write may not target a
                        // `wire` — iverilog rejects it too (whole-net precedent).
                        self.error(
                            MsgCode::ElabLvalueKind,
                            &format!(
                                "procedural hierarchical write to net `{path}` (declare it reg/logic)"
                            ),
                        );
                        poison_chunk()
                    } else {
                        self.build_hier_sel_write_chunk(net, &d.idx_eids, d.part, &path)
                    }
                }
            };
            patch.insert(sentinel, (chunk, path));
        }
        // A hierarchical element/bit-select is NOT a legal force/release target — the
        // local path rejects a bit/part-select force (`is_whole_single_net`), but the
        // placeholder sel-write chunk LOOKS whole at that check, so enforce parity here.
        let mut force_release_paths: Vec<String> = Vec::new();
        for s in &mut self.stmts {
            let force_release = matches!(s, ir::Stmt::Force { .. } | ir::Stmt::Release { .. });
            let chunks = match s {
                ir::Stmt::BlockingAssign { lhs, .. }
                | ir::Stmt::NonblockingAssign { lhs, .. }
                | ir::Stmt::Force { lhs, .. }
                | ir::Stmt::Release { lhs, .. } => &mut lhs.chunks,
                _ => continue,
            };
            for c in chunks {
                if let Some((rebuilt, path)) = patch.get(&c.net) {
                    if force_release {
                        force_release_paths.push(path.clone());
                        *c = poison_chunk();
                    } else {
                        *c = rebuilt.clone();
                    }
                }
            }
        }
        for path in force_release_paths {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "force/release target must be a whole net/variable (a hierarchical \
                     element/bit-select `{path}` is not a legal force/release target)"
                ),
            );
        }
    }

    // ── lvalue lowering ────────────────────────────────────────────
    /// SYS-READ dest guard: the just-lowered dest `eid` is a deferred
    /// hierarchical ELEMENT-select placeholder (`Signal{POISON_NET}` with a
    /// pending `deferred_hier_sel` record). The fixup rebuilds a READ select
    /// — never a write chunk — so the engine write silently VANISHES for
    /// these dests (measured: rc=1, value unchanged; iverilog writes it).
    /// Every SYS-READ special loud-rejects them; a WHOLE hierarchical dest
    /// (`deferred_hier`) keeps its working patched path.
    pub(crate) fn is_deferred_hier_sel_dest(&self, eid: u32) -> bool {
        self.deferred_hier_sel.iter().any(|d| d.eid == eid)
    }
}
