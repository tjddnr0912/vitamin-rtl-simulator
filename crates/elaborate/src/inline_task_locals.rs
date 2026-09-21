//! The inline-task lane's BODY-LOCAL reservation — split out of `inline_task.rs` at
//! the repo's 1000-line policy.
//!
//! One unit because the three functions are one decision: which nets a static task's
//! body-locals get (`hoist_inline_task_locals`), how each declaration is reserved
//! (`hoist_one_inline_local`), and when their initializers run (`emit_inline_local_inits`).
//! The §3.13 per-SV-scope duplicate test lives with them for the same reason.

use super::*;

impl Elaborator<'_> {
    /// Reserve an inline-task's body-local declarations as nets under the current
    /// (unique per-call) scope and run their declaration initializers at entry. A name
    /// already bound in this scope is skipped; 2-state locals register for X/Z→0
    /// coercion, mirroring the formal-local path.
    pub(crate) fn hoist_inline_task_locals(
        &mut self,
        b: &mut ProcessBuilder,
        decls: &[(Vec<ast::Span>, ast::NetVarDecl)],
    ) {
        // The scope is shared per-task (static retention), so a SECOND call finds every
        // local already bound: it allocates nothing and re-runs no initializer;
        // `first_call` is true only for the FIRST inline, gating the one-time init.
        let mut first_call = false;
        // §3.13: the names THIS inlining hoisted, keyed by the SV SCOPE that declared them
        // — the `chain` of enclosing `begin…end`/`fork` spans, empty for a body-top
        // declarator. The skip below makes a static task's local ONE net across N call sites
        // (§6.21) and swallowed a SECOND DECLARATOR of one declaration list with it
        // (`int x = 1; int x = 3;` answered `x=43`, both oracles reject — p26_f / p26_t).
        // ⚠️ The key is the CHAIN, never the flattened net name: `block_local_scope_prefix`
        // returns `None` for a task-body declarator AND for a nested block's declarator that
        // earns no `$blk$` segment, so both landed on one `$itask$<name>$L.x` key and a LEGAL
        // nested shadow (`task tk; integer x; begin integer x; … end endtask`) read as a
        // duplicate — rc=1 where both oracles print `inner x=3`.
        let mut hoisted_here: BTreeSet<(Vec<u32>, String)> = BTreeSet::new();
        for (chain, d) in decls {
            let scope_key: Vec<u32> = chain.iter().map(|sp| sp.lo).collect();
            // §2 Scoping (subroutine block-locals): reserve the decl under the same
            // `$blk$…` prefix the Logic-phase `Stmt::Block` arm lowers its block body in,
            // so two same-named sibling block-locals get two nets instead of sharing one
            // under `$itask$<name>$L`. `None` = the pre-existing path. Taken BEFORE the
            // duplicate report so the reported key is the one the net actually lives
            // under: reporting from the outer prefix printed `…$L.x` for a declarator
            // whose net is `…$L.$blk$<lo>.x` — a key no lookup can reach.
            let seg = self.block_local_scope_prefix(chain, d);
            // One report per declaration; the declaration is then skipped, exactly as
            // `add_net`'s guard skips a redeclared net. Span-keyed: N sites, one report.
            let dup = d
                .names
                .iter()
                .find(|n| !hoisted_here.insert((scope_key.clone(), n.name.name.clone())));
            if let Some(n) = dup {
                let sp = n.name.span;
                if self.reported_decl_collisions.insert((sp.lo, sp.hi)) {
                    // Reported from INSIDE the declaring block's scope, so the key and
                    // the `[in …]` frame both name where the net lives.
                    let nm = n.name.name.clone();
                    let report = |s: &mut Self| {
                        let key = s.fq(&nm);
                        let m = format!("net/variable `{key}` redeclared (duplicate declaration)");
                        s.error_at(MsgCode::ElabUnsupported, sp, &m);
                    };
                    match &seg {
                        Some(sg) => {
                            let sg = sg.clone();
                            self.with_scope(&sg, report)
                        }
                        None => report(self),
                    }
                }
                continue;
            }
            let allocated = match seg {
                Some(sg) => self.with_scope(&sg, |s| s.hoist_one_inline_local(d)),
                None => self.hoist_one_inline_local(d),
            };
            first_call |= allocated;
        }
        // §13.4.1/§6.21: a static local's initializer runs ONCE (before time 0),
        // not on each call — so emit the inits only at the first call site.
        if first_call {
            // The inline route owns its own once-gating (`first_call`) and needs no
            // guard against the frame prologue's skip: that skip is membership in
            // `frame_hoisted_decls`, keyed by the DECLARATOR's own source offset, and
            // an inlined task's declarators are never in a frame's set.
            self.emit_inline_local_inits(b, decls);
        }
    }

    /// One inline-task local DECLARATION, reserved under the CURRENT prefix.
    /// `true` when it allocated at least one net (i.e. this is the first inline of
    /// the task) — the caller gates the once-only initializer emission on it.
    fn hoist_one_inline_local(&mut self, d: &ast::NetVarDecl) -> bool {
        let mut allocated = false;
        for decl in &d.names {
            let key = self.fq(&decl.name.name);
            if self.symbols.contains_key(&key) {
                continue;
            }
            // An UNPACKED-ARRAY body-local (`int arr [0:1];`) gets real element
            // storage, mirroring a module-level array (`array_len` + the
            // addressing sidecars below) — a static-lifetime local for a
            // non-automatic task, so it persists across calls.
            let dim_extents = self.array_dim_extents(&decl.unpacked);
            let array_len = dim_extents
                .iter()
                .fold(1u32, |acc, &(_, n)| acc.saturating_mul(n.max(1)));
            if (array_len as u64) > MAX_ARRAY_LEN {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "unpacked-array local `{}` has {} elements (cap {MAX_ARRAY_LEN})",
                        decl.name.name, array_len
                    ),
                );
                continue;
            }
            allocated = true;
            // §7.4.2 / §4.5.359: the STATIC task's body-local, the twin of the
            // framed one in `frames_reserve`. Opt-in and record are one unit.
            let odd_bound = self.declared_odd_bound(d.range.as_ref()).is_some();
            let (w, msb, lsb, signed) = self.range_to_dims_opt(
                self.shape_kind(d.kind, &d.shape_param),
                d.range.as_ref(),
                self.shape_signed(d.signed, &d.shape_param),
                odd_bound,
            );
            let odd_net = self.nets.len() as u32;
            if odd_bound {
                self.record_declared_bounds_for(odd_net, d.range.as_ref());
            }
            self.add_net(
                &decl.name.name,
                ir::NetVar {
                    // R22: `frame_local_net_kind`, NOT `map_net_kind_or_wire` — a body-local
                    // `string` is WRITTEN by the body, so it needs a heap-backed
                    // `NetKind::String` slot; `map_net_kind_or_wire` has no String arm and
                    // dropped it to `_ => Wire`. This is the THIRD collector for the same
                    // concept (module scope, frame body-locals, inline/static task
                    // body-locals) and it was the one that never got the String arm, so a
                    // `string` local was correct in a `task automatic` and broken in the
                    // otherwise-identical static `task`. Two failure modes came out of the
                    // one Wire: a plain `s = "hi"` was loud (E3018 procedural assignment to
                    // a net), while `$fgets(s, fd)` — whose destination write does not go
                    // through the lvalue check that raises E3018 — was SILENT, returning 0
                    // and leaving `s` untouched at exit 0.
                    kind: frame_local_net_kind(self.shape_kind(d.kind, &d.shape_param)),
                    width: w,
                    msb,
                    lsb,
                    signed,
                    array_len,
                    dir: ir::PortDir::Internal,
                    init: default_init(self.shape_kind(d.kind, &d.shape_param), w),
                },
            );
            let Some(&id) = self.symbols.get(&key) else {
                continue;
            };
            if net_kind_is_two_state(self.shape_kind(d.kind, &d.shape_param)) {
                self.intro_kind
                    .insert(id, self.shape_kind(d.kind, &d.shape_param));
            }
            // Element-addressing sidecars (only for an actual unpacked array),
            // mirroring the module-level decl path so `arr[i]` resolves.
            if !decl.unpacked.is_empty() {
                if dim_extents.len() >= 2 || dim_extents.iter().any(|&(lo, _)| lo != 0) {
                    self.array_dims.insert(id, dim_extents);
                }
                let desc: Vec<bool> = decl
                    .unpacked
                    .iter()
                    .map(|dm| match dm {
                        ast::Dim::Range(r) => {
                            let m = self.const_range_bound_fold(&r.msb);
                            let l = self.const_range_bound_fold(&r.lsb);
                            matches!((m, l), (Some(m), Some(l)) if m > l)
                        }
                        _ => false,
                    })
                    .collect();
                if desc.iter().any(|&x| x) {
                    self.array_dim_desc.insert(id, desc);
                }
                self.record_dim_desc(
                    id,
                    self.shape_kind(d.kind, &d.shape_param),
                    d.range.as_ref(),
                    &d.packed,
                    &decl.unpacked,
                );
                self.unpacked_array_nets.insert(id);
            }
        }
        allocated
    }

    /// The declaration initializers of an inline task's locals, each emitted under the
    /// `$blk$…` prefix its own declaring block was reserved in. Without the wrap the
    /// initializer would resolve the bare name in the TASK prefix while the net lives
    /// in the block one — the write on the new slot, the read on the old.
    fn emit_inline_local_inits(
        &mut self,
        b: &mut ProcessBuilder,
        decls: &[(Vec<ast::Span>, ast::NetVarDecl)],
    ) {
        for (chain, d) in decls {
            match self.block_local_scope_prefix(chain, d) {
                Some(seg) => self.with_scope(&seg, |s| {
                    s.emit_frame_local_inits(b, std::slice::from_ref(d))
                }),
                None => self.emit_frame_local_inits(b, std::slice::from_ref(d)),
            }
        }
    }
}
