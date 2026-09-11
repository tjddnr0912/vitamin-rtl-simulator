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

/// WHY one declaring span of a block-local was admitted by
/// [`Elaborator::gather_auto_block_locals`], carried alongside the span.
///
/// FIVE independent rules admit a span — `automatic` (per-entry storage),
/// DYNAMIC storage (§4.5.249: a `Dim::Dyn`/`Dim::Queue`/`Dim::Assoc` dim or a
/// scalar `string`), SHADOWING a module-scope port/param/net, a STATIC
/// declarator carrying an INITIALIZER, and (SUBROUTINE BODIES ONLY, opt-in) a
/// STATIC declarator carrying NO initializer — and the candidacy filters below
/// must tell them apart. This used to be ONE bool
/// (`widened = d.lifetime != Some(true)`), which is true for a static shadow and
/// for a dynamic-storage widening alike, so filter A could not distinguish them
/// and dropped both. Dropping a widened dynamic-storage span is harmless (its
/// flatten target is a fresh net of its own); dropping a SHADOW span routes the
/// declaration's writes onto the shadowed MODULE net (`block_local/hoist.rs`),
/// which was 12 measured silent-wrong shapes (§2 Scoping queue row 1).
///
/// In-memory only: this rides a `BTreeMap` local to `elaborate`, never a
/// `SchemaHash` / `sim-ir` / `hdl-ast` type, so it cannot move the golden root.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct AdmitReason {
    /// The declaration is NOT `automatic`, so this span was admitted by the
    /// §4.5.249 widening or by the shadow rule rather than by a per-entry lifetime.
    pub(crate) widened: bool,
    /// The name also names a module-scope port, parameter or net. The flatten
    /// this span would otherwise take targets THAT net.
    pub(crate) shadows_module: bool,
    /// A STATIC (non-`automatic`) declarator carrying an initializer. Its write
    /// happens ONCE at t0, not on block entry — measured three-tool identical:
    /// `int s = 5; s = s + 1;` in an `always` prints `6,7,8,9`. So when two
    /// same-named static initialised declarations flatten onto one net, the LAST
    /// initializer of the name is the only one that ever runs and every sharer
    /// reads it; each such declaration therefore owns storage a flatten cannot
    /// share, exactly as `automatic`, dynamic storage and a shadow do.
    pub(crate) static_init: bool,
    /// A STATIC (non-`automatic`) declarator carrying NO initializer, admitted
    /// only inside a SUBROUTINE body (§2 Scoping: the opt-in `admit_static_plain`
    /// feed of [`Elaborator::gather_auto_block_locals`]).
    ///
    /// Measured, 2-oracle: two same-named initializer-free sibling block-locals in
    /// one `task`/`function` body are TWO variables. `task t; begin int x = 44;
    /// $display("A=%0d",x); end begin int x; $display("B=%0d",x); end endtask`
    /// prints `A=44 B=0` on iverilog 13 and verilator 5.052, and `A=44 B=44` at
    /// HEAD — the second block read the first block's leftover. Each such
    /// declaration is its OWN static variable and RETAINS independently across
    /// calls (`P=1 Q=10` then `P=2 Q=20`, both oracles), so the storage a flatten
    /// would share is not shareable and the span earns a `$blk$` scope for the
    /// same reason the four rules above do.
    ///
    /// It is OPT-IN and subroutine-only because the MODULE-PROCESS path answers
    /// this shape differently and already answers it LOUDLY: the R18-X1
    /// read-before-assign guard (`block_local/hoist.rs:606-613`, keyed on
    /// `coalesced_block_locals`) reports E3009 for the same pair written in an
    /// `initial` block. Admitting it there would turn a loud into a value on a
    /// path this slice did not measure, so every module-process feed passes
    /// `false` and that path is byte-identical.
    pub(crate) static_plain: bool,
}

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

/// §2 Scoping (subroutine block-locals): every module-scope `task`/`function` BODY,
/// including the ones declared inside a `generate`, with the same branch path
/// [`for_each_proc`] carries.
///
/// The two classifiers below used to see procedural blocks only, so
/// `scoped_block_locals` never held a span from a subroutine body and
/// `block_local_scope_seg` answered `None` for every block inside one. That is the
/// whole reason two same-named sibling block-locals in a task/function shared one net
/// while the identical shape in an `initial` was correct (§4.5.475).
///
/// PACKAGE / CLASS / INTERFACE subroutines are deliberately NOT here: they reach the
/// reservers through their own callers with their own name sets, were not measured,
/// and stay exactly as they are.
pub(crate) fn for_each_subroutine_body(
    items: &[ast::ModuleItem],
    f: &mut impl FnMut(&ast::Stmt, &BranchPath),
) {
    fn gen_items(
        items: &[ast::GenItem],
        path: &mut BranchPath,
        f: &mut impl FnMut(&ast::Stmt, &BranchPath),
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
        f: &mut impl FnMut(&ast::Stmt, &BranchPath),
    ) {
        for item in items {
            match item {
                ast::ModuleItem::Task(t) => f(&t.body, path),
                ast::ModuleItem::Func(fd) => f(&fd.body, path),
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
        let mut per_name: BTreeMap<String, Vec<(u32, u32, AdmitReason)>> = BTreeMap::new();
        let mut branch_of: BTreeMap<(u32, u32), BranchPath> = BTreeMap::new();
        let gather = |body: &ast::Stmt,
                      path: &BranchPath,
                      admit_static_plain: bool,
                      per_name: &mut BTreeMap<String, Vec<(u32, u32, AdmitReason)>>,
                      branch_of: &mut BTreeMap<(u32, u32), BranchPath>| {
            let before: BTreeMap<String, usize> =
                per_name.iter().map(|(k, v)| (k.clone(), v.len())).collect();
            Self::gather_auto_block_locals(body, module_names, admit_static_plain, per_name);
            for (name, spans) in per_name.iter() {
                for &(lo, hi, _) in &spans[before.get(name).copied().unwrap_or(0)..] {
                    branch_of.entry((lo, hi)).or_insert_with(|| path.clone());
                }
            }
        };
        // MODULE PROCESS bodies: `admit_static_plain = false`. This path answers an
        // initializer-free same-named sibling pair with the R18-X1 read-before-assign
        // LOUD (`block_local/hoist.rs:606-613`), measured at HEAD as two E3009s for
        // the pair written in an `initial` block. Admitting the fifth rule here would
        // silently convert that loud into a value on a path this slice did not
        // measure, so the module-process flatten stays byte-identical.
        for_each_proc(&module.body, &mut |p, path| {
            gather(&p.body, path, false, &mut per_name, &mut branch_of);
        });
        // §2 Scoping (subroutine block-locals): the module's task/function bodies feed
        // the SAME gatherer with the SAME `module_names` set the proc walk passes, so a
        // block-local inside a subroutine earns a `$blk$<lo>` segment under exactly the
        // admission rules a module-process one does — no new `AdmitReason`. A subroutine
        // FORMAL or a body-top local is NOT in `module_names`, so a block-local that
        // shadows one is not admitted by the shadow rule; that shape is loud today
        // (E3009 "referenced outside its `begin…end` block") and stays loud.
        //
        // §2 Scoping row 2 adds ONE admission rule that is exclusive to this feed:
        // `admit_static_plain = true` (a STATIC, initializer-free declarator). It is
        // opt-in rather than global because the R18-X1 loud that covers the shape on
        // the module-process path cannot fire here — `compute_coalesced_block_locals`
        // below deliberately walks `for_each_proc` ONLY, so no subroutine local is in
        // `coalesced_block_locals` and `block_local/hoist.rs:606` skips it. The shape
        // is therefore SILENT on this path and LOUD on that one; only the silent half
        // is this slice's to close.
        for_each_subroutine_body(&module.body, &mut |body, path| {
            gather(body, path, true, &mut per_name, &mut branch_of);
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
            //
            // §2 Scoping queue row 1: that drop is WRONG for a span admitted because it
            // SHADOWS a module-scope name. `widened` conflated the two admission rules
            // (it is `d.lifetime != Some(true)`, true for a static shadow as well as for
            // a dynamic-storage widening), and the S3 rationale does not carry over: it
            // is about not WITHDRAWING scoping a pair already had, and a shadowing name
            // qualifies on ONE span (the `shadows_module` floor below), so there is
            // nothing to withdraw. What the drop does instead is send the declaration
            // down the flatten, whose target for a shadow IS the shadowed module net —
            // measured as 12 silent-wrong shapes (an enclosing block's write landing on
            // the module net).
            //
            // The exemption is taken only when EVERY declaring span of the name is a
            // STATIC shadow. A span that is also `automatic` or dynamic-storage carries a
            // per-entry lifetime requirement of its own, which is what these two filters
            // were built to protect and which the loud E3009 gate is the authority on;
            // leaving those names on the pre-existing path keeps every shape with an
            // `automatic` span exactly as measured (unchanged: loud where a span's use
            // crosses its block, and the pre-existing flatten otherwise — a static shadow
            // pair beside a DISJOINT `automatic` span of the same name is still the old
            // silent route, 1-oracle, recorded in ROADMAP §2).
            //
            // §3.b `blocal-flatten`: the same exemption is taken when EVERY declaring
            // span of the name is a STATIC declarator carrying an INITIALIZER and
            // nothing else (not `automatic`, not a module shadow). The S3 rationale
            // does not carry over there either: a static initializer runs once at t0,
            // so two same-named initialised declarations on one flattened net leave
            // only the LAST initializer running and every sharer reads it. The drop
            // is what sends the outer declaration of a widened NESTING back down the
            // flatten, which costs the name its candidacy entirely (the lone survivor
            // then falls below the two-span bar below). Homogeneity is what makes it
            // safe: a name with even one `automatic`, dynamic-storage or shadow span
            // has a mixed reason set, `static_init_only` is false, and the whole name
            // takes the pre-existing path byte for byte.
            //
            // §2 Scoping row 2: the exemption must be taken for a MIXED static set as
            // well — `begin int x = 44; end` beside `begin int x; end` has one
            // `static_init` span and one `static_plain` span, so a term testing
            // `static_init` alone is false and the pair takes the pre-existing flatten
            // that is the bug. The two reasons share the one property this filter
            // needs: neither span carries a per-entry lifetime (`d.lifetime !=
            // Some(true)` for both, which is exactly `r.widened`), so there is no
            // per-entry requirement to protect. Hence `static_only` = every span is
            // static and non-shadowing, whichever of the two static rules admitted it.
            let shadow_static_only = all.iter().all(|&(_, _, r)| r.shadows_module && r.widened);
            let static_init_only = all
                .iter()
                .all(|&(_, _, r)| r.static_init && r.widened && !r.shadows_module);
            let static_only = all.iter().all(|&(_, _, r)| {
                (r.static_init || r.static_plain) && r.widened && !r.shadows_module
            });
            let spans: Vec<(u32, u32)> = all
                .iter()
                .filter(|&&(lo, hi, r)| {
                    shadow_static_only
                        || static_only
                        || !r.widened
                        || !all.iter().any(|&(l2, h2, _)| contains((lo, hi), (l2, h2)))
                })
                .map(|&(lo, hi, _)| (lo, hi))
                .collect();
            // A name that SHADOWS a module-scope port/param/net qualifies on ONE
            // declaring block: there is nothing for it to coalesce WITH, and the net
            // it would flatten onto is the shadowed one — which is the bug, not the
            // baseline. Every other name still needs two declaring blocks, because
            // a lone non-shadowing block-local has a net of its own already.
            //
            // ⚠️ This condition used to read `|| module_names.contains(name)`, i.e.
            // the exact opposite: a shadow was DISQUALIFIED from scoping. That is
            // what routed all 22 shapes onto the module net.
            let shadows_module = module_names.contains(name);
            if spans.len() < 2 && !shadows_module {
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
            //
            // §2 Scoping queue row 1: `shadow_static_only` is exempt here too, and it has
            // to be — this filter drops BOTH members of a nesting pair, so with filter A
            // fixed but this one unchanged an outer/inner static shadow pair loses BOTH
            // scopes and the INNER write leaks onto the module net as well (measured: c24
            // of the census, correct today, would regress). The exemption is sound for
            // the same reason the R16 §3.4 drop at (3) below could go: the Nets-phase
            // hoist now nests its `$blk$` segments exactly as the Logic-phase lowering
            // does, so two nested candidates of the SAME name get `…$blk$<outer>.s` and
            // `…$blk$<outer>.$blk$<inner>.s` and each block body resolves its own. There
            // is no aliasing left for this filter to prevent between two static shadows —
            // it only prevented them from being scoped at all. Names with an `automatic`
            // or dynamic-storage span keep the drop (see filter A's note).
            //
            // §3.b `blocal-flatten`: `static_init_only` is exempt here for the same
            // reason and with the same nesting argument — the Nets-phase hoist nests
            // `$blk$` segments the way the Logic-phase lowering does, so an outer and
            // an inner static initialised declaration of one name get
            // `…$blk$<outer>.s` and `…$blk$<outer>.$blk$<inner>.s` and cannot alias.
            // Without the exemption this filter drops BOTH members of such a nesting
            // and the pair loses every scope it just earned.
            //
            // §2 Scoping row 2: this filter deliberately keeps `static_init_only` and
            // is NOT widened to `static_only`. The two filters answer different
            // questions and the §3.b nesting argument does not carry to a
            // `static_plain` span. Filter A above is about SIBLINGS (disjoint spans,
            // which this filter never drops), and that is the whole of the measured
            // row. What widening THIS one would newly admit is a NESTING — an outer
            // `begin int x = 44; … begin int x; … end end` — and that shape is LOUD
            // today (E3009 "block-local `x` is referenced outside its `begin…end`
            // block"), because `block_local/gate.rs:596` skips only names that are in
            // `scoped_block_locals`. Scoping the nesting would silently convert that
            // loud into a value, which is a loud → value move on a shape this slice
            // measured only at HEAD (vita loud; both oracles `OUT=44 IN=0`). Loud is
            // the higher rung of the ladder, so it stays until that shape is its own
            // measured row. Keeping `static_init_only` here makes the mixed nesting
            // drop BOTH spans, fall below the two-span bar, earn no scope, and reach
            // the same E3009 it reaches today.
            let spans: Vec<(u32, u32)> = spans
                .iter()
                .copied()
                .filter(|&a| {
                    shadow_static_only
                        || static_init_only
                        || !spans
                            .iter()
                            .any(|&b| a != b && (contains(a, b) || contains(b, a)) && coexist(a, b))
                })
                .collect();
            if spans.len() < 2 && !shadows_module {
                continue;
            }
            for &(lo, hi) in &spans {
                cand.push((lo, hi, name.clone()));
            }
        }
        // (3) R16 §3.4: a candidate block nested inside ANOTHER candidate block used to
        // be dropped here, because the Nets-phase hoist recursed FLAT while the Logic
        // phase lowered a scoped block's body inside its own segment — so with both
        // levels scoped the inner block's nets sat at `$blk$<inner>` while its body
        // resolved under `$blk$<outer>.$blk$<inner>`, missed them, and fell through to
        // the module net. Dropping the inner candidate avoided the mismatch at the cost
        // of the whole two-level shape: a name reused at ONE level worked, the same name
        // reused at TWO levels was loud, and the standard table-driven `.rsp` walker
        // (`foreach (files[fi]) begin automatic int fd = …; … begin <inner locals> end
        // end`, repeated in sibling blocks) sits exactly on that shape.
        //
        // The hoist now nests its scopes the same way the lowering does, so the two
        // agree at any depth and there is nothing left to drop. Note this is NOT the
        // same rule as the same-NAME nesting filter in (2) above, which is about
        // shadowing — an inner block redeclaring a name an enclosing block also
        // declares — and stays.
        let mut out: BTreeMap<u32, std::collections::BTreeSet<String>> = BTreeMap::new();
        for (lo, _hi, name) in cand {
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
                                // R16 §3.3: a FIXED-size unpacked array with a `'{…}` /
                                // `{…}` initializer. The report measured that the
                                // identical declaration with a DYNAMIC `[]` dim passes
                                // while `[4]` is loud, with the same contents and the
                                // same element type — the only thing missing was this
                                // arm, not any emission capability. Re-initializing one
                                // is the whole-array assign `a = '{…}`, a statement form
                                // that already works today INCLUDING under `automatic`
                                // (measured: `automatic logic [4:0] m[4]; m = '{…};` is
                                // accepted and prints correctly), so the emission path is
                                // reused verbatim.
                                //
                                // `Dim::Size(N)` (`a[4]`) and `Dim::Range(hi:lo)`
                                // (`a[3:0]`) are the same fixed array spelled two ways.
                                let fixed_pattern = n.unpacked.len() == 1
                                    && matches!(
                                        n.unpacked[0],
                                        ast::Dim::Range(_) | ast::Dim::Size(_)
                                    )
                                    && n.init.as_ref().is_some_and(|init| {
                                        matches!(
                                            init.kind,
                                            ast::ExprKind::AssignPattern(_)
                                                | ast::ExprKind::Concat { .. }
                                        )
                                    });
                                // R16 §3.3 (the report's side case) is deliberately NOT
                                // here. Marking an initializer-FREE local per-entry
                                // would reset it at every block entry, and that is not
                                // what a conforming simulator does: automatic storage is
                                // created per ACTIVATION, not per block entry. Measured
                                // in iverilog with a block inside an `automatic` task —
                                // three loop iterations print `xx, 10, 11` (the leftover
                                // survives), while three separate CALLS print `xx, xx,
                                // xx`. An initializer, by contrast, does re-run on each
                                // entry (`w=11` every iteration), which is exactly what
                                // the arms above emit. The element-write case is closed
                                // in the definite-assignment walk instead, where it can
                                // be proven rather than assumed.
                                if scalar_var || string_var || dyn_pattern || fixed_pattern {
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

    /// R18-X1: bare names that end up sharing ONE flattened net because they are
    /// declared as block-locals in two or more DISJOINT blocks and are not given
    /// `$blk$` scoping.
    ///
    /// Why this has to be a pure AST pre-pass, like its two siblings above. The
    /// coalesce guard in `hoist` fires when a name's net ALREADY EXISTS, so it only
    /// ever examines the SECOND and later declaring blocks — the first one is checked
    /// as if its net were private, because at that moment it is. That asymmetry is
    /// order, not semantics: both blocks write the same net. It was a silent-wrong
    /// (measured identical at `c8ad2b4` and `46b9816`): the first block wrote its
    /// local, called a task that suspends, and read the local back — observing the
    /// second block's write. vita printed `A v=99` where iverilog prints `A v=1`, at
    /// exit 0, because nothing ever asked the first block the shared-net question.
    ///
    /// Conservative on every axis, since the only consumer is a REJECT gate:
    /// * a name declared in ≥2 distinct blocks counts, whatever the lifetimes;
    /// * `module_names` is excluded to match the hoist guard, which treats a
    ///   module-scope collision as a legitimate SHADOW and hands it to the
    ///   struct/enum/typedef shadow-scoping instead;
    /// * only blocks that put the name on the FLAT net are counted — a block whose
    ///   copy earned a `$blk$<lo>` scope has its own net and does not participate in
    ///   the sharing, so two `automatic` locals that both got scoped are two
    ///   variables, not one. Counting them was a false-loud on a shape that already
    ///   worked (`block_scope_two_level::struct_member_static_branch…`).
    ///
    /// ⚠️ §2 Scoping (subroutine block-locals): this one deliberately keeps walking
    /// `for_each_proc` ONLY, while `compute_scoped_block_locals` above now also walks
    /// the module's subroutine bodies. The set is a module-wide set of bare NAMES with
    /// no span, and its only readers are in the module-process hoist
    /// (`block_local/hoist.rs`), which is not a path any subroutine local takes — so
    /// adding subroutine names buys that path nothing and can only make it louder.
    /// Measured (cell R5: a task declaring `v` in two blocks beside an `initial` block
    /// that declares its own `v` and reads it unassigned): both oracles print
    /// `p=0 t1=1 t2=2`, and feeding the subroutine bodies in here turns the module
    /// process's own `v` — a lone block-local with a net of its own — into
    /// `error[VITA-E3009] … block-local `v` shares one flattened net with a same-named
    /// block-local in another block but is READ before it is assigned here`. A
    /// correct → loud regression, so the walk stays as it is.
    pub(crate) fn compute_coalesced_block_locals(
        module: &ast::ModuleDecl,
        module_names: &std::collections::BTreeSet<String>,
        scoped: &BTreeMap<u32, std::collections::BTreeSet<String>>,
    ) -> std::collections::BTreeSet<String> {
        let mut per_name: BTreeMap<String, std::collections::BTreeSet<u32>> = BTreeMap::new();
        for_each_proc(&module.body, &mut |p, _branch| {
            let mut blocks: Vec<(ast::Span, Vec<(String, ast::Span)>)> = Vec::new();
            crate::block_local::gather_nested_block_locals(&p.body, true, &mut blocks);
            for (blk, names) in blocks {
                for (nm, _) in names {
                    // A block that gives this name its own scope segment owns a
                    // distinct net; only the FLAT declarations share one.
                    if scoped.get(&blk.lo).is_some_and(|s| s.contains(&nm)) {
                        continue;
                    }
                    per_name.entry(nm).or_default().insert(blk.lo);
                }
            }
        });
        per_name
            .into_iter()
            .filter(|(nm, blocks)| blocks.len() >= 2 && !module_names.contains(nm))
            .map(|(nm, _)| nm)
            .collect()
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

    /// §2 Scoping (subroutine block-locals): the `$blk$…` prefix a block-local decl
    /// must be RESERVED under, given the chain of enclosing block spans
    /// [`crate::block_local::collect_block_local_decls_spanned`] returns.
    ///
    /// It has to mirror the Logic-phase `Stmt::Block` arm (`stmt_main.rs:604`) exactly,
    /// or the write lands on one net and the read on another:
    /// * an ENCLOSING block contributes a segment whenever it is scoped AT ALL
    ///   (`scoped_block_locals.contains_key`) — that arm wraps a scoped block's whole
    ///   body, scoped names or not;
    /// * the DECLARING block contributes one only when this decl itself is selected by
    ///   [`Self::block_local_scope_seg`]'s deliberate ANY rule — an unselected decl in a
    ///   scoped block keeps the outer net, and reads from inside still reach it because
    ///   `walk_scopes_key` treats `$blk$` as transparent.
    ///
    /// `None` ⇒ no wrap, i.e. the pre-existing bare-name reservation, byte for byte.
    pub(crate) fn block_local_scope_prefix(
        &self,
        chain: &[ast::Span],
        d: &ast::NetVarDecl,
    ) -> Option<String> {
        let mut segs: Vec<String> = Vec::new();
        for (i, sp) in chain.iter().enumerate() {
            if i + 1 == chain.len() {
                if let Some(seg) = self.block_local_scope_seg(*sp, d) {
                    segs.push(seg);
                }
            } else if self.scoped_block_locals.contains_key(&sp.lo) {
                segs.push(format!("$blk${}", sp.lo));
            }
        }
        (!segs.is_empty()).then(|| segs.join("."))
    }
}
