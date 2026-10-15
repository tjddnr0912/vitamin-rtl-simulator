//! procedural block-local hoisting — split out of the original `elaborate` lib.rs.
//!
//! R17 split this file again (it had grown to 1248 lines, past the 1000-line module
//! cap) along the line the code already divided on:
//!   * [`gate`] — may this block-local be flattened? The callee-effect resolver, the
//!     definite-assignment entry point, the scope-leak check, and their diagnostics.
//!   * [`hoist`] — flatten it. Net creation and declaration-initializer collection.
//!
//! What stays here is the block-walking free functions both need, plus the unrelated
//! module-scope `gather_local_decl_names`.

use super::*;

mod gate;
mod hoist;
mod proofs;
mod timed_task;

pub(crate) use timed_task::{source_name, timing_nested_decls, RoutineView};

/// The statements nested DIRECTLY in `s` that can hold a declaring `begin … end` /
/// `fork … join` — a block's or fork's own statements, both arms of an `if`, every
/// `case` item, a loop's body (`f(child, true)`), and the body of a timing control:
/// `#d`, `@(e)`, `wait (c)`.
///
/// §5.2 row 72: this is the ONE descent every walker that finds block-local
/// declarations takes — the census (`gather_nested_block_locals`, which feeds the
/// row-21 `select_scope` census, the scope-leak gate and the coalesce set), the
/// collectors (`collect_block_local_decls_spanned`, which feeds every subroutine
/// reserver), the Nets-phase hoist (`hoist_block_local_nets`), the scoping and
/// per-entry classifiers (`gather_auto_block_locals`,
/// `compute_per_entry_block_locals`), and the multidriver shadow guard. Each used to
/// spell its own `match`, and three had no timing-control arm while the census had
/// one: a declaration in `initial #1 begin logic [7:0] x; … end` was never hoisted,
/// so the block's `x` resolved to a same-named module variable (its writes landed
/// there; with no module twin it was E3010) while the census counted the block as a
/// scope. A walker that takes its descent from here cannot disagree with another on
/// which blocks exist.
///
/// Exhaustive on purpose: a future statement that holds a statement is a compile
/// error here, not a silently skipped scope. Not entered, each for a stated reason:
/// * a `for` header's `init` / `step` — an assignment, never a declaring block (the
///   loop variable is the header's own declaration, not a block-local);
/// * an assertion's action blocks (`DeferredAssert` arms, `ConcurrentAssert`
///   pass/fail): a deferred assertion's action is one subroutine call (IEEE 1800-2017
///   §16.4), and a concurrent one's is lowered by the SVA synthesis, whose action
///   locals are ROADMAP §3.b `mdrv-assert-local`. The census takes their spans
///   separately (`SelectScope::actions`), and the multidriver guard enters them only
///   for its opted-in rule.
pub(crate) fn for_each_nested_stmt<'a>(s: &'a ast::Stmt, mut f: impl FnMut(&'a ast::Stmt, bool)) {
    use ast::Stmt::*;
    match s {
        Block { stmts, .. } | Fork { stmts, .. } => stmts.iter().for_each(|st| f(st, false)),
        If { then_s, else_s, .. } => {
            f(then_s, false);
            if let Some(e) = else_s {
                f(e, false);
            }
        }
        Case { items, .. } => {
            for it in items {
                let (ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. }) = it;
                f(body, false);
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            f(body, true)
        }
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => {
            if let Some(b) = body {
                f(b, false)
            }
        }
        DeferredAssert { .. } | ConcurrentAssert { .. } => {}
        Blocking { .. }
        | NonBlocking { .. }
        | Return { .. }
        | SysTaskCall { .. }
        | UserTaskCall { .. }
        | RandomizeWith { .. }
        | EventTrigger { .. }
        | Disable { .. }
        | WaitFork { .. }
        | Assign { .. }
        | Deassign { .. }
        | Force { .. }
        | Release { .. }
        | CoverProperty { .. }
        | Null(_)
        | Error(_) => {}
    }
}

/// Collect every NESTED `begin…end`/`fork` block (i.e. not the top-level body
/// block) together with the names it declares as locals. `is_top` is true only for
/// the outermost body statement, whose own decls are function/task-scoped (not
/// block-scoped) and so are skipped.
pub(crate) fn gather_nested_block_locals(
    s: &ast::Stmt,
    is_top: bool,
    out: &mut Vec<(ast::Span, Vec<(String, ast::Span)>)>,
) {
    if let ast::Stmt::Block { decls, span, .. } | ast::Stmt::Fork { decls, span, .. } = s {
        if !is_top && !decls.is_empty() {
            // R17 §4.1: each name carries its own DECLARATION span, so the
            // scope-leak diagnostic can be anchored at the declaration the user
            // has to rename rather than at nothing at all.
            let names: Vec<(String, ast::Span)> = decls
                .iter()
                .flat_map(|d| d.names.iter().map(|n| (n.name.name.clone(), d.span)))
                .collect();
            if !names.is_empty() {
                out.push((*span, names));
            }
        }
    }
    for_each_nested_stmt(s, |st, _| gather_nested_block_locals(st, false, out));
}

/// Is `s` a timing control whose body [`for_each_nested_stmt`] enters?
pub(crate) fn is_timing_control(s: &ast::Stmt) -> bool {
    matches!(
        s,
        ast::Stmt::Wait { .. } | ast::Stmt::DelayCtrl { .. } | ast::Stmt::EventCtrl { .. }
    )
}

/// Can the statements of a declaring block SUSPEND — wait in a timing control (`#d`,
/// `@(e)`, `wait (c)`, `wait fork`), an intra-assignment delay or event, or a call of a
/// user task (which may wait)? A block that cannot runs from entry to exit without
/// another process running in between, so two activations of it never overlap in time
/// (§5.2 row 72, review round 2: p20, `x=1 x=2` = verilator).
pub(crate) fn stmts_may_suspend(stmts: &[ast::Stmt]) -> bool {
    fn walk(s: &ast::Stmt) -> bool {
        let here = match s {
            ast::Stmt::DelayCtrl { .. }
            | ast::Stmt::EventCtrl { .. }
            | ast::Stmt::Wait { .. }
            | ast::Stmt::WaitFork { .. }
            | ast::Stmt::UserTaskCall { .. } => true,
            ast::Stmt::Blocking { delay, event, .. }
            | ast::Stmt::NonBlocking { delay, event, .. } => delay.is_some() || event.is_some(),
            _ => false,
        };
        let mut hit = here;
        for_each_nested_stmt(s, |st, _| hit = hit || walk(st));
        hit
    }
    stmts.iter().any(walk)
}

/// [`collect_block_local_decls`] without the timing-control bodies — the walk every
/// collector took before §5.2 row 72.
///
/// Kept for `expr_size_hier`'s census of the names a bare hierarchical segment can
/// bind to: counting a timing-body local made a module variable of the same name look
/// declared twice, and a declined name takes that lane's 32-bit fallback (row 72's
/// round-1 review: `16'(u.hs * q8)` went `00000120` → `0000xx20`). The same fallback
/// for a block-local with no timing control above it is pre-existing (ROADMAP §2).
pub(crate) fn collect_block_local_decls_outside_timing(
    s: &ast::Stmt,
    out: &mut Vec<ast::NetVarDecl>,
) {
    if let ast::Stmt::Block { decls, .. } | ast::Stmt::Fork { decls, .. } = s {
        out.extend(decls.iter().cloned());
    }
    if is_timing_control(s) {
        return;
    }
    for_each_nested_stmt(s, |st, _| collect_block_local_decls_outside_timing(st, out));
}

/// Collect every `begin`/`fork` block-local declaration in `s`, in source order
/// (recursing into nested blocks and control-flow bodies). Used to reserve a
/// frame body's block-locals under its `$func$<name>` scope so a local declared
/// inside a `begin … end` resolves (previously unresolved → E3010, for frame
/// functions AND tasks alike).
pub(crate) fn collect_block_local_decls(s: &ast::Stmt, out: &mut Vec<ast::NetVarDecl>) {
    let mut spanned = Vec::new();
    collect_block_local_decls_spanned(s, &mut Vec::new(), &mut spanned);
    out.extend(spanned.into_iter().map(|(_, d)| d));
}

/// §2 Scoping (subroutine block-locals): the span-carrying twin of
/// [`collect_block_local_decls`]. Each decl is returned with the CHAIN of enclosing
/// `begin…end`/`fork` spans, outermost first, the declaring block last.
///
/// The bare-name key is exactly what made two same-named sibling block-locals inside
/// one subroutine share a net (`hoist_inline_task_locals` / `reserve_frame_block_locals`
/// both key on `self.fq(&decl.name.name)` under ONE per-subroutine segment). The
/// reservers need the declaring block to reproduce the `$blk$<lo>` segment the
/// Logic-phase `Stmt::Block` arm wraps the body in; the CHAIN — not just the declaring
/// span — because that arm nests its segments, so a decl two levels down sits at
/// `…$blk$<outer>.$blk$<inner>` and a flat single segment would miss it.
///
/// The descent is [`for_each_nested_stmt`]'s, so a block under a timing control
/// (`#d begin … end` in a task body) is reserved like any other; the outermost body
/// block's own decls are included (the parser leaves them empty, and
/// `gather_auto_block_locals` treats a process's top block the same way).
pub(crate) fn collect_block_local_decls_spanned(
    s: &ast::Stmt,
    chain: &mut Vec<ast::Span>,
    out: &mut Vec<(Vec<ast::Span>, ast::NetVarDecl)>,
) {
    let declaring = match s {
        ast::Stmt::Block { decls, span, .. } | ast::Stmt::Fork { decls, span, .. } => {
            chain.push(*span);
            out.extend(decls.iter().map(|d| (chain.clone(), d.clone())));
            true
        }
        _ => false,
    };
    for_each_nested_stmt(s, |st, _| collect_block_local_decls_spanned(st, chain, out));
    if declaring {
        chain.pop();
    }
}

/// Does a `$strobe` / `$monitor`-family task ([`is_deferred_print_task`]) anywhere in
/// `stmts` (the declaring block's own statements, nested blocks included) take `name`
/// in its arguments?
///
/// §5.2 row 72: such a task renders its arguments LATER — at the end of the time step,
/// or on every later change — so on a net two blocks share (the bare-name flatten's
/// coalesce, `coalesced_block_locals`) it shows whatever the other block wrote by then.
/// The coalesce gate's definite-assignment walk proves the WRITE comes first, which
/// says nothing about when such a read happens. Conservative on purpose: any argument
/// that mentions the name counts (`expr_no_ref`), a nested redeclaration included.
pub(crate) fn stmts_defer_read(stmts: &[ast::Stmt], name: &str) -> bool {
    fn walk(s: &ast::Stmt, name: &str) -> bool {
        if let ast::Stmt::SysTaskCall {
            name: task, args, ..
        } = s
        {
            if is_deferred_print_task(&task.name) && args.iter().any(|a| !expr_no_ref(a, name)) {
                return true;
            }
        }
        let mut hit = false;
        for_each_nested_stmt(s, |st, _| hit = hit || walk(st, name));
        hit
    }
    stmts.iter().any(|s| walk(s, name))
}

impl Elaborator<'_> {
    /// Param-aware const-eval in a SIGNED 64-bit domain (P0-6, 2026-06-10).
    /// Folds: literals (sign-aware), params/genvars in scope, unary `+ - ~ !`,
    /// the binary operator set with i64 semantics (so a descending genvar
    /// condition `i >= 0` actually terminates), ternary `?:` and `$clog2`
    /// (P0-5 — the `localparam AW = $clog2(DEPTH)` / `W = M ? a : b` idioms).
    /// Overflow and ill-defined folds return None — param-binding callers
    /// escalate None to an ERROR (never a silent 0), width callers clamp
    /// loudly. NOTE: this is a width-less mathematical-integer model; a
    /// logical `>>` of a NEGATIVE value is width-dependent and folds None.
    /// GAP-G shadow guard: the bare names the module declares locally — header
    /// (`#(...)`) params, ports, and top-level body nets/params. A name in this
    /// set SHADOWS a same-named wildcard-imported package array, so a const
    /// element read of it must not fold the imported array. See `local_decl_names`.
    /// Earliest declaration position of every module-scope net/variable name —
    /// the ordering half of [`Self::gather_local_decl_names`].
    ///
    /// ⚠️ NOT the same walk: this one also visits `ModuleItem::PortDecl`, which the
    /// name gatherer does not. The direction is benign (a non-ANSI body port maps to
    /// 0, so it is never "later"), but the two sets are not identical and an earlier
    /// note claimed they were.
    ///
    /// Ports and header params map to 0: they are declared in the header, ahead of
    /// every body item, so no body use can precede them. Body nets and params map to
    /// their own `span.lo`. A name declared twice keeps the EARLIEST position — the
    /// duplicate is someone else's diagnostic, and taking the earliest is the
    /// conservative choice for this one (it can only fail to flag).
    /// Every name declared inside a PROCEDURAL block of this module.
    ///
    /// The use-before-declaration check needs it because vita's v1 flatten model
    /// publishes a plain-static block-local as a module net under its BARE name: the
    /// use then lands on the module-scope declaration's position and an ordinary
    ///     initial begin : blk integer i; … end   …   integer i;
    /// testbench is rejected. `hoisted_block_local` cannot answer this — the hoist
    /// skips a name the module already declares, so the very collision that needs
    /// excluding is the one it does not record. Taken from the AST instead, which is
    /// order-free and cannot depend on which pass has run.
    pub(crate) fn gather_block_local_names(
        &self,
        module: &ast::ModuleDecl,
    ) -> std::collections::BTreeSet<String> {
        let mut out = std::collections::BTreeSet::new();
        for item in &module.body {
            let ast::ModuleItem::Proc(p) = item else {
                continue;
            };
            let mut decls = Vec::new();
            collect_block_local_decls(&p.body, &mut decls);
            for d in &decls {
                for n in &d.names {
                    out.insert(n.name.name.clone());
                }
            }
        }
        out
    }

    pub(crate) fn gather_decl_positions(&self, module: &ast::ModuleDecl) -> BTreeMap<String, u32> {
        let mut pos: BTreeMap<String, u32> = BTreeMap::new();
        let mut note = |n: &str, lo: u32| match pos.entry(n.to_string()) {
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(lo);
            }
            std::collections::btree_map::Entry::Occupied(mut o) => {
                if lo < *o.get() {
                    o.insert(lo);
                }
            }
        };
        for p in &module.params {
            note(&p.name.name, 0);
        }
        match &module.ports {
            ast::PortList::Ansi(ports) => {
                for pt in ports {
                    note(&pt.name.name, 0);
                }
            }
            ast::PortList::NonAnsi(idents) => {
                for id in idents {
                    note(&id.name, 0);
                }
            }
            ast::PortList::None => {}
        }
        for item in &module.body {
            match item {
                ast::ModuleItem::NetVar(d) => {
                    for n in &d.names {
                        note(&n.name.name, n.name.span.lo);
                    }
                }
                ast::ModuleItem::Param(p) => note(&p.name.name, p.name.span.lo),
                // A NON-ANSI body port declaration (`input logic a;`) re-declares a
                // header port name; the header already put it at 0, and a name that
                // is ONLY here is still a port — so 0, not the decl position.
                ast::ModuleItem::PortDecl(pd) => {
                    for n in &pd.names {
                        note(&n.name, 0);
                    }
                }
                _ => {}
            }
        }
        pos
    }

    pub(crate) fn gather_local_decl_names(
        &self,
        module: &ast::ModuleDecl,
    ) -> std::collections::BTreeSet<String> {
        let mut names = std::collections::BTreeSet::new();
        for p in &module.params {
            names.insert(p.name.name.clone());
        }
        match &module.ports {
            ast::PortList::Ansi(ports) => {
                for pt in ports {
                    names.insert(pt.name.name.clone());
                }
            }
            ast::PortList::NonAnsi(idents) => {
                for id in idents {
                    names.insert(id.name.clone());
                }
            }
            ast::PortList::None => {}
        }
        for item in &module.body {
            match item {
                ast::ModuleItem::NetVar(d) => {
                    for n in &d.names {
                        names.insert(n.name.name.clone());
                    }
                }
                ast::ModuleItem::Param(p) => {
                    names.insert(p.name.name.clone());
                }
                _ => {}
            }
        }
        names
    }

    /// `names` ∪ the bare names bound into THIS scope by a package VARIABLE import
    /// (`import pkg::*` / `import pkg::name`). For the block-local SCOPING
    /// computations only — see the two traps below.
    ///
    /// Three-lifetime census (§2 Scoping row 3, measured against iverilog 13 and
    /// verilator 5.052, which agree on every cell):
    /// - DECLARATION: `package.rs:1288-1291` (wildcard) and `:1406-1408` (explicit)
    ///   insert the `$pkg$<pkg>` net id VERBATIM into `symbols`, so an imported bare
    ///   name is an ALIAS to the package's single shared net (`lib.rs:1180`), not a
    ///   per-module copy.
    /// - SHADOW: `gather_local_decl_names` above returns the module's OWN
    ///   declarations only, so `shadows_module` (`block_local_class.rs:398`,
    ///   `frames_reserve.rs:180`, `hoist.rs`'s twin) was false for such a name. On the
    ///   module-PROCESS feed that term is the SOLE one a plain, initializer-free,
    ///   non-`automatic` block-local can satisfy (`admit_static_plain` is false there,
    ///   `block_local_class.rs:284-286`), so the declaration took the bare flatten —
    ///   onto `symbols[<scope>.<name>]`, i.e. the package's own storage.
    /// - EXPORT: measured, both spellings name ONE net, so the block-local's write was
    ///   visible to `pkg::name` INSIDE the block, to a continuous assign, to a sibling
    ///   instance, and to a DIFFERENT module importing the same package.
    ///
    /// An imported name occupies this scope's bare-name namespace exactly as a
    /// declared net does — the precondition `shadows_module` exists to detect ("the net
    /// it would flatten onto is the shadowed one", `frames_reserve.rs:171-177`).
    /// Mechanism proof: adding a module-level declaration of the same name (which
    /// touches no package storage) already makes the whole shape correct, and the only
    /// thing it changes is this set.
    ///
    /// Two traps this signature is shaped around:
    /// 1. It returns a SEPARATE set; callers must not augment `names` in place. That
    ///    same set is `apply_import_consts`'s `local_names` (`package.rs:1258`), where
    ///    membership SUPPRESSES the import binding, so feeding it back its own product
    ///    would unbind every wildcard-imported variable. It is also read by
    ///    `compute_coalesced_block_locals` and `const_array.rs:125`.
    /// 2. It must NOT reach `compute_per_entry_block_locals`
    ///    (`block_local_class.rs:532`), which reads `module_names` with the OPPOSITE
    ///    polarity (`contains` -> `continue`): widening it there would remove per-entry
    ///    re-initialisation from an `automatic`-with-init block-local shadowing an
    ///    import.
    ///
    /// Keyed on `pkg_var_aliases` (`lib.rs:1187`), never on `symbols`: an
    /// interface-port alias also lives in `symbols` and is not a package variable
    /// (`package.rs:1284-1287` keeps the two apart for exactly this reason).
    pub(crate) fn names_with_pkg_var_aliases(
        &self,
        names: &std::collections::BTreeSet<String>,
    ) -> std::collections::BTreeSet<String> {
        if self.pkg_var_aliases.is_empty() {
            return names.clone();
        }
        let mut out = names.clone();
        for key in self.pkg_var_aliases.keys() {
            // Keys are `self.fq(name)`, so only the aliases bound at the CURRENT scope
            // are this scope's bare names (a nested instance's alias is not visible
            // here by bare name).
            let bare = match key.rsplit_once('.') {
                Some((scope, bare)) if scope == self.cur_prefix => bare,
                None if self.cur_prefix.is_empty() => key.as_str(),
                _ => continue,
            };
            out.insert(bare.to_string());
        }
        out
    }
}
