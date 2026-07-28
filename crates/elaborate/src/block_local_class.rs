//! Which block-locals earn their own `$blk$` scope, and which get per-entry
//! (§6.21) initialization — the two PURE AST classifiers behind
//! `hoist_block_local_nets`, split out of `block_local.rs` for the ≤1000-line policy.
//!
//! Both are functions of the AST alone, so the Nets-phase hoist and the Logic-phase
//! lowering cannot disagree about which locals were scoped or re-initialized.

use super::*;

impl Elaborator<'_> {
    /// DUP (round-5): decide which `automatic` block-locals need a `$blk$<span>`
    /// scope segment. Returns block `span.lo` → the set of local NAMES to scope in
    /// that block. A name is scoped IFF it is declared `automatic` in ≥ 2 procedural
    /// blocks that are all MUTUALLY DISJOINT (no span containment ⇒ no shadowing /
    /// nesting), AND it does not also name a module-scope net/param (`module_names`).
    /// A candidate block nested inside ANOTHER candidate block is then dropped (a
    /// single-level hoist would not match nested segments). Every excluded case
    /// falls through to the pre-existing loud E3009 — correct-or-loud. Pure function
    /// of the AST, so the Nets-phase hoist and the Logic-phase lowering agree.
    pub(crate) fn compute_scoped_block_locals(
        module: &ast::ModuleDecl,
        module_names: &std::collections::BTreeSet<String>,
    ) -> BTreeMap<u32, std::collections::BTreeSet<String>> {
        // `outer` strictly contains `inner` (properly nested AST blocks never
        // partially overlap, so containment ⇒ nesting).
        fn contains(outer: (u32, u32), inner: (u32, u32)) -> bool {
            outer.0 <= inner.0 && inner.1 <= outer.1 && outer != inner
        }
        // (1) gather automatic block-locals across all procedural blocks.
        let mut per_name: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
        for item in &module.body {
            if let ast::ModuleItem::Proc(p) = item {
                Self::gather_auto_block_locals(&p.body, &mut per_name);
            }
        }
        // (2) candidate (span, name): declared in ≥2 blocks, no module-net
        //     collision, and no two declaring spans nested (shadowing).
        let mut cand: Vec<(u32, u32, String)> = Vec::new();
        for (name, spans) in &per_name {
            if spans.len() < 2 || module_names.contains(name) {
                continue;
            }
            let shadowed = spans.iter().enumerate().any(|(i, &a)| {
                spans
                    .iter()
                    .enumerate()
                    .any(|(j, &b)| i != j && contains(a, b))
            });
            if shadowed {
                continue;
            }
            for &(lo, hi) in spans {
                cand.push((lo, hi, name.clone()));
            }
        }
        // (3) drop any candidate block nested inside ANOTHER candidate block.
        let cand_spans: Vec<(u32, u32)> = cand.iter().map(|&(lo, hi, _)| (lo, hi)).collect();
        let mut out: BTreeMap<u32, std::collections::BTreeSet<String>> = BTreeMap::new();
        for (lo, hi, name) in cand {
            let nested = cand_spans.iter().any(|&s| contains(s, (lo, hi)));
            if nested {
                continue;
            }
            out.entry(lo).or_default().insert(name);
        }
        out
    }

    /// r18 (family D): the module-process block-locals that are `automatic` WITH AN
    /// INITIALIZER and safe to give per-entry (IEEE §6.21) semantics on the single flattened
    /// net — the initializer re-runs at block entry instead of once at t0. Pure function of
    /// the AST (mirrors `compute_scoped_block_locals`), keyed by the declaring block's
    /// `span.lo` → qualifying NAMES. SOUND because a single flattened net is correct iff at
    /// most one activation of the block is live at a time: a module process's loops are
    /// sequential (iteration N completes before N+1), so ONLY a `fork` ancestor can spawn a
    /// concurrent copy — `under_fork` blocks are EXCLUDED (they keep the loud reject). Also
    /// excludes module-net collisions (handled by the coalesce guards). Records TWO shapes:
    /// (a) a plain scalar VAR with an initializer (family D), and (b) BL2/BL3 (round-19) a
    /// DYNAMIC-STORAGE local — one `Dim::Dyn`/`Dim::Queue` unpacked dim (dyn array / string
    /// dyn array / queue) with a `'{…}`/`{…}`/`new[]` init, whose decl-init EXPANSION re-runs
    /// at block entry (self-resetting). Assoc, multi-dim, and non-pattern dyn inits are NOT
    /// recorded. Anything not recorded here falls through to the existing E3009
    /// (correct-or-loud). A module process cannot recurse, so recursion is a non-issue.
    pub(crate) fn compute_per_entry_block_locals(
        module: &ast::ModuleDecl,
        module_names: &std::collections::BTreeSet<String>,
    ) -> BTreeMap<u32, std::collections::BTreeSet<String>> {
        /// `fork_multi` — this statement sits under a `fork` arm that can be SPAWNED
        /// more than once, so two activations of the same block may be live at once and
        /// the one flattened net cannot represent both. `repeatable` — this statement
        /// itself can execute more than once (a repeating process, or a loop ancestor).
        fn walk(
            s: &ast::Stmt,
            fork_multi: bool,
            repeatable: bool,
            module_names: &std::collections::BTreeSet<String>,
            out: &mut BTreeMap<u32, std::collections::BTreeSet<String>>,
        ) {
            let under_fork = fork_multi;
            match s {
                ast::Stmt::Block {
                    decls, stmts, span, ..
                } => {
                    if !under_fork {
                        for d in decls {
                            if d.lifetime != Some(true) {
                                continue;
                            }
                            for n in &d.names {
                                if module_names.contains(&n.name.name) {
                                    continue;
                                }
                                // A plain scalar VAR (not a net, not a string, no unpacked
                                // dims) with an initializer — its init re-runs at block entry
                                // (emitted as a plain `x = init` blocking).
                                let scalar_var = netvar_kind_is_var(d.kind)
                                    && !matches!(d.kind, ast::NetVarKind::String)
                                    && n.init.is_some()
                                    && n.unpacked.is_empty();
                                // A scalar `string` local with an initializer. Its re-init is
                                // the SAME plain `s = init` blocking the scalar case emits (a
                                // string net holds one whole value; there is no handle to
                                // reallocate), which is why `begin automatic string s; s = "a";
                                // … end` already worked — only the decl-init spelling was left
                                // out of family D, for no reason the emission path shares.
                                let string_var = matches!(d.kind, ast::NetVarKind::String)
                                    && n.init.is_some()
                                    && n.unpacked.is_empty();
                                // BL2/BL3 (round-19): a DYNAMIC-STORAGE block-local — a single
                                // `Dim::Dyn`/`Dim::Queue` unpacked dim (a dyn array, a string
                                // dyn array, or a queue) — declared `automatic` with a
                                // `'{…}` / `{…}` (§10.10 unpacked-concat) initializer. Its
                                // decl-init EXPANSION (`d = new[N]; d[i] = e;` for a dyn array /
                                // a `q.push_back(e)` sequence for a queue) is re-emitted at
                                // BLOCK ENTRY by `emit_per_entry_block_inits`, giving §6.21
                                // per-entry semantics on the one flattened handle net
                                // (self-resetting: `new[N]` re-allocates; a queue is cleared
                                // first). EXCLUDED and left loud: associative arrays
                                // (`Dim::Assoc` — no `'{…}` expansion), MULTI-dim dyn
                                // (`unpacked.len() != 1`), and a non-pattern init. (A `new[]`
                                // decl-init is separately rejected at the decl for any dyn
                                // handle — vita supports only a `'{…}` pattern there.)
                                //
                                // §4.5.248: a `new[N]` init joins them — it re-allocates, so it
                                // is self-resetting in exactly the way this gate requires, and
                                // it emits as the plain `d = new[N]` statement.
                                let dyn_pattern = n.unpacked.len() == 1
                                    && matches!(n.unpacked[0], ast::Dim::Dyn | ast::Dim::Queue(_))
                                    && n.init.as_ref().is_some_and(|init| {
                                        matches!(
                                            init.kind,
                                            ast::ExprKind::AssignPattern(_)
                                                | ast::ExprKind::Concat { .. }
                                        ) || (matches!(init.kind, ast::ExprKind::New { .. })
                                            && matches!(n.unpacked[0], ast::Dim::Dyn))
                                    });
                                if scalar_var || string_var || dyn_pattern {
                                    out.entry(span.lo).or_default().insert(n.name.name.clone());
                                }
                            }
                        }
                    }
                    for st in stmts {
                        walk(st, fork_multi, repeatable, module_names, out);
                    }
                }
                // A fork's children run CONCURRENTLY and a `join_any`/`join_none` arm
                // OUTLIVES the fork point. What breaks the one-flattened-net model is not
                // concurrency as such — it is TWO LIVE ACTIVATIONS OF THE SAME BLOCK, which
                // needs the fork itself to be spawned more than once. So the arms inherit
                // `fork_multi = fork_multi || repeatable`: a fork reached once (an `initial`
                // with no loop above it) gives each arm exactly one activation, and the
                // flattened net is then as correct for the arm as for any straight-line
                // block. A fork under a loop, or in a repeating process, keeps the loud.
                //
                // A loop INSIDE an arm does not reintroduce the hazard — one arm is one
                // thread, so its iterations are sequential — which is why `repeatable`
                // (about this statement) and `fork_multi` (about the spawning) are two
                // flags and not one.
                //
                // This is what left the standard watchdog — `fork begin automatic int
                // timeout = D; void'($value$plusargs(…, timeout)); #(timeout*1ns); end
                // join_none` — loud in essentially every testbench.
                ast::Stmt::Fork { stmts, .. } => {
                    let arm_multi = fork_multi || repeatable;
                    for st in stmts {
                        walk(st, arm_multi, repeatable, module_names, out);
                    }
                }
                ast::Stmt::If { then_s, else_s, .. } => {
                    walk(then_s, fork_multi, repeatable, module_names, out);
                    if let Some(e) = else_s {
                        walk(e, fork_multi, repeatable, module_names, out);
                    }
                }
                ast::Stmt::Case { items, .. } => {
                    for it in items {
                        walk(
                            case_item_body(it),
                            fork_multi,
                            repeatable,
                            module_names,
                            out,
                        );
                    }
                }
                ast::Stmt::For { body, .. }
                | ast::Stmt::While { body, .. }
                | ast::Stmt::Repeat { body, .. }
                | ast::Stmt::Forever { body, .. } => {
                    walk(body, fork_multi, true, module_names, out)
                }
                ast::Stmt::DelayCtrl { body, .. }
                | ast::Stmt::EventCtrl { body, .. }
                | ast::Stmt::Wait { body, .. } => {
                    if let Some(b) = body {
                        walk(b, fork_multi, repeatable, module_names, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = BTreeMap::new();
        for item in &module.body {
            if let ast::ModuleItem::Proc(p) = item {
                // An `initial` / `final` process body executes ONCE; every `always*` form
                // re-runs, so a `join_none` arm it spawned can still be live when the next
                // iteration spawns another.
                let repeats = !matches!(p.kind, ast::ProcKind::Initial | ast::ProcKind::Final);
                walk(&p.body, false, repeats, module_names, &mut out);
            }
        }
        out
    }

    /// DUP (round-5): the `$blk$<span>` scope segment for a decl `d` in the block at
    /// `span`, if ANY name it declares is marked for scoping. `None` ⇒ not scoped
    /// (pre-existing behavior).
    ///
    /// ANY (not ALL) is load-bearing for soundness: `compute_scoped_block_locals`
    /// marks scoping PER-NAME, and the block body is lowered under `$blk$<span>`
    /// whenever the block has ANY scoped name. If a MULTI-name decl
    /// (`automatic int idx, jdx;`) had only `idx` scoped and we left the whole decl
    /// BARE (the old ALL check), `idx` would keep the bare `top.idx` net while the
    /// block IS wrapped — breaking the invariant "every colliding occurrence is
    /// scoped, so no bare `top.idx` exists". A later same-named static block-local
    /// then coalesces onto that bare net and aliases block `idx` (silent-wrong,
    /// found by adversarial review). Scoping the WHOLE decl on ANY hit keeps the
    /// invariant; the non-colliding sibling (`jdx`) is a block-local referenced only
    /// within this block, so giving it a `$blk$` net too is harmless (it resolves
    /// under the same scope wrap, and an outside reference is already loud).
    pub(crate) fn block_local_scope_seg(
        &self,
        span: ast::Span,
        d: &ast::NetVarDecl,
    ) -> Option<String> {
        let set = self.scoped_block_locals.get(&span.lo)?;
        if d.names.iter().any(|n| set.contains(&n.name.name)) {
            Some(format!("$blk${}", span.lo))
        } else {
            None
        }
    }
}
