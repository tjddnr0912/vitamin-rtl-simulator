//! Which block-locals earn their own `$blk$` scope, and which get per-entry
//! (§6.21) initialization — the two PURE AST classifiers behind
//! `hoist_block_local_nets`, split out of `block_local.rs` for the ≤1000-line policy.
//!
//! Both are functions of the AST alone, so the Nets-phase hoist and the Logic-phase
//! lowering cannot disagree about which locals were scoped or re-initialized.

use super::*;

/// Every procedural block in a module, INCLUDING the ones inside generate constructs.
///
/// §4.5.258: the two classifiers below used to walk `module.body` for `ModuleItem::Proc`
/// only, so a process inside a `generate` was invisible to them — no block-local of one
/// could ever earn a `$blk$` scope, and the whole same-name family (queues, dynamic and
/// associative arrays, strings and string arrays) stayed loud there while the identical
/// code at module scope worked. Both classifiers are pure functions of the AST and must
/// see the same set, so they share this walk.
///
/// A generate-for body is one AST subtree however many times it unrolls, and each unroll
/// elaborates under its own prefix, so a name declared once inside the loop body is
/// declared in ONE block here — which is correct: the copies cannot collide with each
/// other, only two distinct blocks can.
/// §4.5.259: each process also carries the BRANCH PATH it sits under — one
/// `(decision, arm)` pair per enclosing generate `if`/`case`, keyed by the construct's own
/// span so two different constructs at the same depth are never confused. Exactly ONE arm
/// of a decision is elaborated, so two blocks whose paths disagree at a shared decision
/// can never both exist, and letting them interact turned a supported pair back into a
/// loud one: an `if (0)` arm carrying a nested same-name local made the live pair look
/// shadowed. Comparing paths keeps the classifier a pure function of the AST — no
/// condition is evaluated, so it cannot depend on parameter binding or pass order.
pub(crate) type BranchPath = Vec<(u32, u32)>;

/// Can two blocks with these branch paths BOTH be elaborated?
pub(crate) fn branches_coexist(a: &BranchPath, b: &BranchPath) -> bool {
    !a.iter()
        .any(|(da, arm_a)| b.iter().any(|(db, arm_b)| da == db && arm_a != arm_b))
}

pub(crate) fn for_each_proc(
    items: &[ast::ModuleItem],
    f: &mut impl FnMut(&ast::ProceduralBlock, &BranchPath),
) {
    fn gen_items(
        items: &[ast::GenItem],
        path: &mut BranchPath,
        f: &mut impl FnMut(&ast::ProceduralBlock, &BranchPath),
    ) {
        for it in items {
            match it {
                ast::GenItem::For { body, .. } | ast::GenItem::Block { items: body, .. } => {
                    gen_items(body, path, f)
                }
                ast::GenItem::If {
                    then_b,
                    else_b,
                    span,
                    ..
                } => {
                    for (arm, b) in [(0u32, then_b), (1, else_b)] {
                        path.push((span.lo, arm));
                        gen_items(b, path, f);
                        path.pop();
                    }
                }
                ast::GenItem::Case { items, span, .. } => {
                    for (arm, ci) in items.iter().enumerate() {
                        let body = match ci {
                            ast::GenCaseItem::Match { body, .. }
                            | ast::GenCaseItem::Default { body, .. } => body,
                        };
                        path.push((span.lo, arm as u32));
                        gen_items(body, path, f);
                        path.pop();
                    }
                }
                ast::GenItem::Item(mi) => walk_items(std::slice::from_ref(mi), path, f),
            }
        }
    }
    fn walk_items(
        items: &[ast::ModuleItem],
        path: &mut BranchPath,
        f: &mut impl FnMut(&ast::ProceduralBlock, &BranchPath),
    ) {
        for item in items {
            match item {
                ast::ModuleItem::Proc(p) => f(p, path),
                ast::ModuleItem::Generate(g) => gen_items(&g.items, path, f),
                _ => {}
            }
        }
    }
    walk_items(items, &mut Vec::new(), f);
}

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
        // (1) gather automatic block-locals across all procedural blocks, each carrying
        //     the generate branch it sits under.
        let mut per_name: BTreeMap<String, Vec<(u32, u32, bool)>> = BTreeMap::new();
        let mut branch_of: BTreeMap<(u32, u32), BranchPath> = BTreeMap::new();
        for_each_proc(&module.body, &mut |p, path| {
            let before: BTreeMap<String, usize> =
                per_name.iter().map(|(k, v)| (k.clone(), v.len())).collect();
            Self::gather_auto_block_locals(&p.body, &mut per_name);
            for (name, spans) in per_name.iter() {
                for &(lo, hi, _) in &spans[before.get(name).copied().unwrap_or(0)..] {
                    branch_of.entry((lo, hi)).or_insert_with(|| path.clone());
                }
            }
        });
        let coexist = |a: (u32, u32), b: (u32, u32)| match (branch_of.get(&a), branch_of.get(&b)) {
            (Some(pa), Some(pb)) => branches_coexist(pa, pb),
            _ => true,
        };
        // (2) candidate (span, name): declared in ≥2 blocks, no module-net
        //     collision, and no two declaring spans nested (shadowing).
        let mut cand: Vec<(u32, u32, String)> = Vec::new();
        for (name, all) in &per_name {
            // Review S3: a WIDENED span (admitted by the dynamic-storage rule, not by
            // `automatic`) that ENCLOSES another declaring span of this name is dropped
            // rather than counted. Otherwise merely gathering it makes the name look
            // shadowed and withdraws scoping that already worked — a correct design
            // turned loud. It is only ever dropped from CANDIDACY; its declaration still
            // takes the ordinary flattened path, exactly as before the widening.
            let spans: Vec<(u32, u32)> = all
                .iter()
                .filter(|&&(lo, hi, widened)| {
                    !widened || !all.iter().any(|&(l2, h2, _)| contains((lo, hi), (l2, h2)))
                })
                .map(|&(lo, hi, _)| (lo, hi))
                .collect();
            if spans.len() < 2 || module_names.contains(name) {
                continue;
            }
            // §4.5.259: a span in a NESTING relation with another declaring span of this
            // name is dropped from candidacy — it is not a reason to disqualify the NAME.
            // Globally disqualifying it meant an `if (0)` generate arm carrying its own
            // nested `k` withdrew the scoping of a live, disjoint pair elsewhere. A
            // dropped span keeps the flattened net exactly as before, and the survivors
            // get distinct `$blk$` nets, so the two cannot alias. This generalizes the
            // review-S3 rule, which dropped exactly one shape of the same thing.
            //
            // Two declarations in different arms of ONE generate `if`/`case` never both
            // exist, so they do not shadow each other either.
            let spans: Vec<(u32, u32)> = spans
                .iter()
                .copied()
                .filter(|&a| {
                    !spans
                        .iter()
                        .any(|&b| a != b && (contains(a, b) || contains(b, a)) && coexist(a, b))
                })
                .collect();
            if spans.len() < 2 {
                continue;
            }
            for &(lo, hi) in &spans {
                cand.push((lo, hi, name.clone()));
            }
        }
        // (3) drop any candidate block nested inside ANOTHER candidate block.
        let cand_spans: Vec<(u32, u32)> = cand.iter().map(|&(lo, hi, _)| (lo, hi)).collect();
        let mut out: BTreeMap<u32, std::collections::BTreeSet<String>> = BTreeMap::new();
        for (lo, hi, name) in cand {
            let nested = cand_spans
                .iter()
                .any(|&s| contains(s, (lo, hi)) && coexist(s, (lo, hi)));
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
        for_each_proc(&module.body, &mut |p, _branch| {
            // An `initial` / `final` process body executes ONCE; every `always*` form
            // re-runs, so a `join_none` arm it spawned can still be live when the next
            // iteration spawns another. The branch path is irrelevant here: this
            // classifier is per-BLOCK (it never compares two blocks), so a dead arm's
            // block can only mark itself.
            let repeats = !matches!(p.kind, ast::ProcKind::Initial | ast::ProcKind::Final);
            walk(&p.body, false, repeats, module_names, &mut out);
        });
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
