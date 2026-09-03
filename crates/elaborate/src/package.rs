//! packages / imports — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// IEEE §26.3: is `func` FREE-NAME CLOSED — safe to frame-lower for a package-scoped
/// call `pkg::f(args)`? True iff every bare identifier it reads is one of its own
/// formals or body-locals or a same-package CONSTANT, every name it writes is a formal
/// or body-local, and every call it makes is to a same-package SUBROUTINE.
///
/// ⚠️ This used to say "SELF-CONTAINED, straight-line … its body has no control flow",
/// and both halves are now false. The no-control-flow clause was never a
/// name-resolution property — the frame path this routes through lowers arbitrary
/// CFGs, exactly as it does for a bare-name imported function — and the nested-call
/// clause fell once a package routine's body began resolving in its own package
/// (`resolve_rtn_key`). What survives is the free-name closure, which is the whole
/// reason the check exists: a free name would be resolved in the CALLER's module scope.
///
/// Applied to the ROOT is not enough — `inject_pkg_callees` walks the TRANSITIVE
/// callee set, so it applies this to every callee it injects and refuses the ones that
/// fail (a sibling with a free name bound that name to a caller net, silently).
pub(crate) fn pkg_task_self_contained(
    task: &ast::TaskDef,
    pkg_const_names: &std::collections::BTreeSet<String>,
    pkg_rtn_names: &std::collections::BTreeSet<String>,
) -> bool {
    Elaborator::task_self_contained_impl(task, pkg_const_names, pkg_rtn_names)
}

pub(crate) fn pkg_func_self_contained(
    func: &ast::FunctionDef,
    pkg_const_names: &std::collections::BTreeSet<String>,
    pkg_rtn_names: &std::collections::BTreeSet<String>,
) -> bool {
    // WRITE set: names a statement may assign to — the function's own
    // formals / locals plus its return-by-name. A package const / enum label is
    // NOT writable, so it stays out of this set (writing one is loud).
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for p in &func.ports {
        names.insert(p.name.name.clone());
    }
    let mut decls = func.body_decls.clone();
    collect_block_local_decls(&func.body, &mut decls);
    for d in &decls {
        for n in &d.names {
            names.insert(n.name.name.clone());
        }
    }
    // A value function may assign its own return value by name (`f = expr;`).
    names.insert(func.name.name.clone());
    // READ set: the write set PLUS same-package constants (enum labels +
    // localparams). Round-9 PKG2: a body may READ a bare `C`/`D` that is an
    // enum label of the SAME package as the function (frame-body lowering
    // injects those consts under the `$func$pkg::name` scope so the bare name
    // resolves to the package version — see `push_pkg_consts_scoped`). A local
    // that shadows a same-name pkg const still wins (it is in `names`).
    let read_names: std::collections::BTreeSet<String> =
        names.union(pkg_const_names).cloned().collect();
    pkg_stmt_pure_with(&func.body, &names, &read_names, pkg_rtn_names)
}

/// Free-name-closed check for one statement (see `pkg_func_self_contained`).
/// `write` = assignable names; `read` = readable names (write set ∪ same-package
/// consts).
/// Free-name-closed check over the FULL statement set, plus the same-package routines a
/// nested call may target.
///
/// ⚠️ The straight-line restriction this replaces was never about name resolution.
/// It read "no control flow", and the message told users to `import pkg::*` instead
/// — but the frame path this routes through lowers arbitrary CFGs (it is the same
/// path a bare-name imported function takes, `if`/`for` and all). What actually
/// mattered was the FREE-NAME closure, and that is checked here as before.
pub(crate) fn pkg_stmt_pure_with(
    s: &ast::Stmt,
    write: &std::collections::BTreeSet<String>,
    read: &std::collections::BTreeSet<String>,
    rtns: &std::collections::BTreeSet<String>,
) -> bool {
    let ep = |e: &ast::Expr| pkg_expr_pure_with(e, read, rtns);
    let sp = |st: &ast::Stmt| pkg_stmt_pure_with(st, write, read, rtns);
    use ast::Stmt as S;
    match s {
        S::If {
            cond,
            then_s,
            else_s,
            ..
        } => ep(cond) && sp(then_s) && else_s.as_ref().is_none_or(|e| sp(e)),
        S::Case {
            scrutinee, items, ..
        } => {
            ep(scrutinee)
                && items.iter().all(|it| match it {
                    ast::CaseItem::Match { labels, body, .. } => labels.iter().all(&ep) && sp(body),
                    ast::CaseItem::Default { body, .. } => sp(body),
                })
        }
        S::For {
            init,
            cond,
            step,
            body,
            ..
        } => sp(init) && ep(cond) && sp(step) && sp(body),
        S::While { cond, body, .. } => ep(cond) && sp(body),
        S::Repeat { count, body, .. } => ep(count) && sp(body),
        S::UserTaskCall { name, args, .. } => {
            name.segments.len() == 1
                && rtns.contains(&name.segments[0].name)
                && args.iter().all(&ep)
        }
        // Everything else keeps the conservative original rule (a `Block`, a
        // `Return`, a blocking `=`, and nothing more).
        _ => pkg_stmt_pure_orig(s, write, read, rtns),
    }
}

fn pkg_stmt_pure_orig(
    s: &ast::Stmt,
    write: &std::collections::BTreeSet<String>,
    read: &std::collections::BTreeSet<String>,
    rtns: &std::collections::BTreeSet<String>,
) -> bool {
    let pkg_expr_pure =
        |e: &ast::Expr, n: &std::collections::BTreeSet<String>| pkg_expr_pure_with(e, n, rtns);
    let pkg_stmt_pure =
        |st: &ast::Stmt,
         w: &std::collections::BTreeSet<String>,
         r: &std::collections::BTreeSet<String>| { pkg_stmt_pure_with(st, w, r, rtns) };
    use ast::Stmt::*;
    match s {
        Block { stmts, .. } => stmts.iter().all(|st| pkg_stmt_pure(st, write, read)),
        Return { value: Some(e), .. } => pkg_expr_pure(e, read),
        Return { value: None, .. } => true,
        // Only a BLOCKING `=` (a local / function-name assignment) is foldable; a
        // non-blocking `<=` in a function is illegal, and delay / intra-event is not
        // straight-line.
        Blocking {
            lhs,
            rhs,
            delay,
            event,
            ..
        } => {
            delay.is_none()
                && event.is_none()
                && pkg_lvalue_pure(lhs, write, read)
                && pkg_expr_pure(rhs, read)
        }
        // Control flow, `<=`, task calls, timing, force / assert, etc. → NOT
        // pure-inlinable (conservative: a construct not listed here is loud).
        _ => false,
    }
}

/// Free-name-closed check for an expression (see `pkg_func_self_contained`). A bare
/// name must be a formal / body-local; nested USER calls and exotic nodes are rejected.
pub(crate) fn pkg_expr_pure(e: &ast::Expr, names: &std::collections::BTreeSet<String>) -> bool {
    pkg_expr_pure_with(e, names, &std::collections::BTreeSet::new())
}

/// [`pkg_expr_pure`], plus a set of same-package ROUTINE names a nested call may
/// target. A call used to be rejected outright because the callee would have been
/// resolved in the CALLER's module scope; `resolve_rtn_key` now resolves it in the
/// package, so a nested same-package call is as safe as a bare-name one.
pub(crate) fn pkg_expr_pure_with(
    e: &ast::Expr,
    names: &std::collections::BTreeSet<String>,
    rtns: &std::collections::BTreeSet<String>,
) -> bool {
    if let ast::ExprKind::Call { name, args } = &e.kind {
        return name.segments.len() == 1
            && rtns.contains(&name.segments[0].name)
            && args.iter().all(|a| pkg_expr_pure_with(a, names, rtns));
    }
    pkg_expr_pure_inner(e, names, rtns)
}

fn pkg_expr_pure_inner(
    e: &ast::Expr,
    names: &std::collections::BTreeSet<String>,
    rtns: &std::collections::BTreeSet<String>,
) -> bool {
    let pkg_expr_pure = |e: &ast::Expr, names: &std::collections::BTreeSet<String>| {
        pkg_expr_pure_with(e, names, rtns)
    };
    use ast::ExprKind::*;
    match &e.kind {
        IntLit { .. } | RealLit { .. } | StrLit { .. } => true,
        // A package-scoped `pkg::CONST` / `pkg::var` read resolves to a single global
        // net — unambiguous and collision-free in any caller scope.
        PkgScoped { .. } => true,
        Ident(p) => p.segments.len() == 1 && names.contains(&p.segments[0].name),
        Unary { operand, .. } => pkg_expr_pure(operand, names),
        Binary { lhs, rhs, .. } => pkg_expr_pure(lhs, names) && pkg_expr_pure(rhs, names),
        Ternary {
            cond,
            then_e,
            else_e,
        } => {
            pkg_expr_pure(cond, names)
                && pkg_expr_pure(then_e, names)
                && pkg_expr_pure(else_e, names)
        }
        BitSelect { base, index } => pkg_expr_pure(base, names) && pkg_expr_pure(index, names),
        PartSelect { base, msb, lsb } => {
            pkg_expr_pure(base, names) && pkg_expr_pure(msb, names) && pkg_expr_pure(lsb, names)
        }
        IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            pkg_expr_pure(base, names)
                && pkg_expr_pure(offset, names)
                && pkg_expr_pure(width, names)
        }
        Concat { parts } => parts.iter().all(|p| pkg_expr_pure(p, names)),
        Replicate { count, value } => {
            pkg_expr_pure(count, names) && value.iter().all(|v| pkg_expr_pure(v, names))
        }
        Paren { inner } => pkg_expr_pure(inner, names),
        MinTypMax { typ, .. } => pkg_expr_pure(typ, names),
        Cast { expr, .. } => pkg_expr_pure(expr, names),
        // A `$sys` call ($signed/$clog2/$bits/…) is a pure function of its args.
        SysCall { args, .. } => args.iter().all(|a| pkg_expr_pure(a, names)),
        // A nested USER call, `new`, randomize, dist, `$`-in-queue, etc. make the
        // function non-self-contained (or need a frame) → NOT pure-inlinable.
        _ => false,
    }
}

/// Free-name-closed check for an assignment target (see `pkg_func_self_contained`).
/// The write TARGET (base ident) must be a formal/local (`write`); index /
/// select sub-expressions are ordinary reads (`read` = write ∪ same-pkg consts,
/// so e.g. `arr[C] = ..` with `C` a package const is allowed).
pub(crate) fn pkg_lvalue_pure(
    lv: &ast::Lvalue,
    write: &std::collections::BTreeSet<String>,
    read: &std::collections::BTreeSet<String>,
) -> bool {
    match lv {
        ast::Lvalue::Ident(p) => p.segments.len() == 1 && write.contains(&p.segments[0].name),
        ast::Lvalue::BitSelect { base, index, .. } => {
            pkg_lvalue_pure(base, write, read) && pkg_expr_pure(index, read)
        }
        ast::Lvalue::PartSelect { base, msb, lsb, .. } => {
            pkg_lvalue_pure(base, write, read)
                && pkg_expr_pure(msb, read)
                && pkg_expr_pure(lsb, read)
        }
        ast::Lvalue::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            pkg_lvalue_pure(base, write, read)
                && pkg_expr_pure(offset, read)
                && pkg_expr_pure(width, read)
        }
        ast::Lvalue::Concat { .. } | ast::Lvalue::Error(_) => false,
    }
}

/// What `push_pkg_consts_*` hands back for the unwind: the previous `params` and
/// `param_meta` entries of every constant it injected, newest last.
type SavedPkgConsts = (
    Vec<(String, Option<i64>)>,
    Vec<(String, Option<(u32, bool)>)>,
);

impl Elaborator<'_> {
    /// Round-9 PKG2: register a package's constants (enum labels + localparams)
    /// as params under the CURRENT scope so a frame-lowered `pkg::fn` body can
    /// READ a bare same-package name (`C`, `D`) that resolves to `pkg::C` /
    /// `pkg::D`. Mirrors `push_body_enum_labels` (scoped registration +
    /// `restore_params` unwind) and `apply_import_consts` (also seeds
    /// `param_meta` for the declared width, so the framed body matches the
    /// bare-import call byte-for-byte). Skips any name that is one of the
    /// function's own formals / locals / return-name: a formal is a NET,
    /// resolved AFTER params, so an injected same-name const would shadow it —
    /// skipping keeps the local winning. Returns (param save-list, param_meta
    /// save-list) for the caller to unwind in reverse.
    #[allow(clippy::type_complexity)]
    pub(crate) fn push_pkg_consts_scoped(
        &mut self,
        pkg: &str,
        func: &ast::FunctionDef,
    ) -> SavedPkgConsts {
        // Skip-set = formals + body-local decls + the function's own name (return by
        // name), the same construction `pkg_func_self_contained` uses.
        let mut skip: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in &func.ports {
            skip.insert(p.name.name.clone());
        }
        let mut decls = func.body_decls.clone();
        collect_block_local_decls(&func.body, &mut decls);
        for d in &decls {
            for n in &d.names {
                skip.insert(n.name.name.clone());
            }
        }
        skip.insert(func.name.name.clone());
        self.push_pkg_consts_skipping(pkg, &skip)
    }

    /// [`Self::push_pkg_consts_scoped`] for a TASK — a task has no return-by-name, so
    /// its own name is not in the skip set.
    pub(crate) fn push_pkg_consts_task(
        &mut self,
        pkg: &str,
        task: &ast::TaskDef,
    ) -> SavedPkgConsts {
        let mut skip: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in &task.ports {
            skip.insert(p.name.name.clone());
        }
        let mut decls = task.body_decls.clone();
        collect_block_local_decls(&task.body, &mut decls);
        for d in &decls {
            for n in &d.names {
                skip.insert(n.name.name.clone());
            }
        }
        self.push_pkg_consts_skipping(pkg, &skip)
    }

    fn push_pkg_consts_skipping(
        &mut self,
        pkg: &str,
        skip: &std::collections::BTreeSet<String>,
    ) -> SavedPkgConsts {
        let mut saved_p = Vec::new();
        let mut saved_m = Vec::new();
        let consts = match self.pkg_consts.get(pkg) {
            Some(c) => c.clone(),
            None => return (saved_p, saved_m),
        };
        let metas = self.pkg_const_meta.get(pkg).cloned().unwrap_or_default();
        for (cname, &v) in &consts {
            if skip.contains(cname) {
                continue;
            }
            let key = self.fq(cname);
            saved_p.push((key.clone(), self.bind_param_value(key.clone(), v)));
            if let Some(&m) = metas.get(cname) {
                saved_m.push((key.clone(), self.param_meta.insert(key, m)));
            }
        }
        // The wide side map gets the same scoped registration. `wide_param_bits` is
        // FQ-keyed and a package function's prefix is its own, so no unwind list is
        // needed — but the skip set still applies: a formal of the same name is a NET
        // and must keep winning.
        let wides: Vec<(String, ir::ConstVal)> = self
            .pkg_wide_bits
            .get(pkg)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        for (cname, cv) in wides {
            if skip.contains(&cname) {
                continue;
            }
            let key = self.fq(&cname);
            self.wide_param_bits.insert(key, cv);
        }
        (saved_p, saved_m)
    }

    /// A2b-prereq (adversarial diff F2): true iff a DOTTED (multi-segment)
    /// name's symbols hit lands on a package-variable IMPORT alias. An import
    /// is a lexical binding for the BARE name only (IEEE §26.3) — a dotted
    /// path resolving through one (possible only at the outermost bare-key
    /// candidate of the scope walk) is an illegal hierarchical access and
    /// must be treated as a MISS by the dotted-name consumers, so the
    /// reference stays loud. Single-segment lookups never call this.
    pub(crate) fn dotted_hit_is_pkg_alias(&self, joined: &str) -> bool {
        !self.pkg_var_aliases.is_empty()
            && self
                .walk_scopes_key(joined, |k| self.symbols.contains_key(k))
                .is_some_and(|k| self.pkg_var_aliases.contains_key(&k))
    }

    /// A2b-prereq (adversarial sound S1/S2): true iff a BARE name's innermost
    /// symbols hit is a package-variable import alias that a local CONSTANT
    /// shadows — a param/localparam/enum label (live `params` binding) or a
    /// declared genvar (persistent record; its `params` binding is
    /// unroll-transient). Such a name must never resolve to the package net:
    /// reads see the constant, so a write/array-view resolving the alias
    /// would be a SILENT divergence. Callers turn a `true` into a loud path.
    pub(crate) fn bare_hit_is_shadowed_pkg_alias(&self, name: &str) -> bool {
        !self.pkg_var_aliases.is_empty()
            && self
                .walk_scopes_key(name, |k| self.symbols.contains_key(k))
                .is_some_and(|k| self.pkg_var_aliases.contains_key(&k))
            && (self.lookup_scoped(name).is_some()
                || self
                    .walk_scopes_key(name, |k| self.genvar_decls.contains(k))
                    .is_some())
    }

    /// Does `base` (a part/indexed-select base) peel through ≥1 array-element
    /// `BitSelect` to an explicit `pkg::name` root? Such a package array-element
    /// SUB-select (`pkg::mem[i][m:l]`) has offset-normalization edge cases —
    /// nested non-zero-LSB packed dims resolve against the whole-net range, not
    /// the residual element range — that v1 does not handle correctly. It is LOUD
    /// (exactly as on the base before package array elements could lower at all),
    /// never silently mis-normalized. A DIRECT `pkg::vec[m:l]` (0 peels) or a
    /// LOCAL base yields false and lowers normally.
    pub(crate) fn is_pkg_array_elem_subselect(&self, base: &ast::Expr) -> bool {
        let mut cur = base;
        let mut peeled = false;
        loop {
            match &cur.kind {
                ast::ExprKind::Paren { inner } => cur = inner,
                ast::ExprKind::BitSelect { base, .. } => {
                    peeled = true;
                    cur = base;
                }
                ast::ExprKind::PkgScoped { .. } => {
                    return peeled && self.pkg_scoped_var_net(cur).is_some();
                }
                _ => return false,
            }
        }
    }

    /// Resolve an explicitly package-scoped base `pkg::name` to its package-level
    /// VARIABLE net, if it names one. Mirrors the const-vs-var precedence of the
    /// `PkgScoped` expr arm (`lower_expr`): a package CONSTANT (param/enum-label)
    /// is a value, not a net, so it is NOT resolved here (returns `None` → the
    /// select chain bails and the scalar lane const-folds it). Explicit `::`
    /// qualification carries no shadowing ambiguity, so — unlike the bare/dotted
    /// `Ident` arms — there is no alias guard. Read-only: `pkg::arr[i]` is a legal
    /// rvalue select (iverilog-pinned), while writing a package variable
    /// (`pkg::arr[i] = …`) is rejected at parse (matches iverilog).
    pub(crate) fn pkg_scoped_var_net(&self, e: &ast::Expr) -> Option<u32> {
        let ast::ExprKind::PkgScoped { pkg, name } = &e.kind else {
            return None;
        };
        if self
            .pkg_consts
            .get(&pkg.name)
            .and_then(|c| c.get(&name.name))
            .is_some()
        {
            return None;
        }
        self.pkg_vars
            .get(&pkg.name)
            .and_then(|m| m.get(&name.name))
            .copied()
    }

    // ── v7 P2-D packages (IR-0 — elaborate-side symbol flattening) ──
    /// Fold one package body: params/localparams + enum labels (decl order,
    /// package-local visibility) into `pkg_consts`; clone funcs/tasks into
    /// `pkg_funcs`/`pkg_tasks`; lower VARIABLE declarations (incl. desugared
    /// array parameters) to nets under the reserved `$pkg$<pkg>` scope and
    /// flush their collected decl-inits as the package's own §6.8 pre-sweep
    /// `initial` (A2b — see `elaborate_pkg_netvar`). Anything else
    /// (cont-assigns / procs / package-internal imports) is loud.
    pub(crate) fn elaborate_package(&mut self, pm: &ast::ModuleDecl) {
        let pkg = pm.name.name.clone();
        // fold under a synthetic scope so the package's own params resolve
        // while folding later ones (`localparam L2 = W * 2`).
        let saved_prefix = std::mem::replace(&mut self.cur_prefix, format!("$pkg${pkg}"));
        let mut saved: Vec<(String, Option<i64>)> = Vec::new();
        // Parallel to `saved`: param_meta entries for THIS package's params, made
        // live during the fold so an intra-package alias/expression resolves a
        // sibling param's (width, signed), then restored (no cross-scope pollution).
        let mut saved_meta: Vec<(String, Option<(u32, bool)>)> = Vec::new();
        // §3 ⑨: the string / real twins of `saved`+`consts`. Same two jobs — made LIVE
        // during this package's fold so an intra-package sibling reference resolves
        // (`parameter real R2 = R*2.0;`, `parameter SI = (S=="AUTO") ? "RED" : S;`), then
        // restored so a module-scope name of the same spelling is untouched; and
        // collected for the flush into the per-package maps the readers consult.
        let mut saved_real: Vec<(String, Option<f64>)> = Vec::new();
        let mut saved_str: Vec<(String, Option<String>)> = Vec::new();
        let mut saved_wide: Vec<(String, Option<ir::ConstVal>)> = Vec::new();
        let mut consts: BTreeMap<String, i64> = BTreeMap::new();
        let mut real_vals: BTreeMap<String, f64> = BTreeMap::new();
        let mut str_vals: BTreeMap<String, String> = BTreeMap::new();
        let mut wide_vals: BTreeMap<String, ir::ConstVal> = BTreeMap::new();
        // Every parameter name in this body, whatever domain it folded into — see
        // `elaborate_pkg_netvar`. Kept beside the three value maps rather than derived
        // from them, so a future fourth domain has one obvious place to register.
        let mut param_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        // V33-3 (DIAGNOSTIC ONLY): which of this package's consts are enum LABELS, and
        // which enum declared each — flushed to `pkg_enum_labels` so the `pk::LA.name()`
        // message can say what the author actually wrote (and name the type to declare)
        // instead of describing a chained string method.
        let mut enum_labels: BTreeMap<String, String> = BTreeMap::new();
        // Declared `(width, signed)` per PARAM const (flushed to
        // `pkg_const_meta`) so a `pkg::x` / bare-imported read gets its true
        // self-width in a concat/replication (see the field doc).
        let mut const_meta: BTreeMap<String, (u32, bool)> = BTreeMap::new();
        // The DECLARED range per PARAM const (flushed to `pkg_const_range`), so a
        // select of this constant — in any scope, by any spelling — can normalize
        // against the declaration instead of reading raw internal bits.
        let mut const_range: BTreeMap<String, (u32, u32, bool)> = BTreeMap::new();
        // …and made LIVE during this package's own fold, exactly as `param_meta` is
        // above, so an intra-package sibling (`parameter Q = W[7:0];`) folds. Same
        // save/restore discipline: a module-scope name of the same spelling must not
        // inherit this package's declared range.
        let mut saved_range: Vec<(String, Option<DeclRange>)> = Vec::new();
        let mut vars: BTreeMap<String, u32> = BTreeMap::new();
        // GAP-G: const array-parameter element values for this package (name →
        // elements), the package-scope twin of `array_const_vals`. Flushed into
        // `pkg_array_const_vals` below so an element read `p::ROT[i]` (or a bare
        // `ROT[i]` from `import p::*`) folds in a constant context.
        let mut array_vals: BTreeMap<String, Vec<i64>> = BTreeMap::new();
        let mut funcs: BTreeMap<String, ast::FunctionDef> = BTreeMap::new();
        let mut tasks: BTreeMap<String, ast::TaskDef> = BTreeMap::new();
        let mut types: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for item in &pm.body {
            match item {
                ast::ModuleItem::Param(p) => {
                    // A2b-prereq: params and variables share the package's single
                    // name space (IEEE §26.3) — a duplicate is loud, never a
                    // silent double-binding (`add_net` only guards net-vs-net).
                    // Hoisted above the fold so it fires for every domain, not just
                    // the integer one the fold used to be.
                    if vars.contains_key(&p.name.name) {
                        self.error(
                            MsgCode::DupUnit,
                            &format!(
                                "package symbol `{}` declared more than once (a \
                                 parameter and a variable share one name space)",
                                p.name.name
                            ),
                        );
                    }
                    let key = self.fq(&p.name.name);
                    param_names.insert(p.name.name.clone());
                    // §3 ⑨: this was the FOURTH copy of the parameter-declaration fold
                    // and the only one that never learned the string / real domains, so
                    // the same source text folded differently inside and outside a
                    // package — `parameter S = "RED";` was loud on its own bare LITERAL,
                    // and `parameter real PR = 3;` folded into the integer domain and
                    // made `P::PR / 2` answer 1 where both oracles say 1.5 (silent, at
                    // exit 0). The three arms below mirror the GENERATE copy verbatim,
                    // which is the right reference because it shares the property that
                    // decides the shape: a package constant, like a generate-scope one,
                    // HAS NO OVERRIDE CHANNEL, so its declared default is always what
                    // binds and the width may be taken from the declaration.
                    //
                    // Order is real → string → integer, and the first two FALL THROUGH
                    // rather than return-or-error: `param_real_value` applies §11.8.1
                    // (a real operand puts the expression in the real domain) and hands
                    // back an i64 twin only when the initializer was wholly integral,
                    // which is what keeps `localparam real R = 4;` usable where an
                    // integer is wanted. Reversing the two is a measured silent-wrong
                    // (§4.5.364).
                    if let Some((rv, exact)) = self.param_real_value(&p.ty, &p.value) {
                        // Live for the rest of THIS package's fold, so a sibling
                        // `parameter real R2 = R*2.0;` resolves; restored below with the
                        // integer and meta entries so nothing leaks into module scope.
                        saved_real.push((key.clone(), self.real_param_val.insert(key.clone(), rv)));
                        real_vals.insert(p.name.name.clone(), rv);
                        if let Some(i) = exact {
                            saved.push((key.clone(), self.bind_param_value(key.clone(), i)));
                            consts.insert(p.name.name.clone(), i);
                        }
                        continue;
                    }
                    if let Some(raw) = self.param_str_or_folded(p, false) {
                        saved_str.push((
                            key.clone(),
                            self.str_param_raw.insert(key.clone(), raw.clone()),
                        ));
                        str_vals.insert(p.name.name.clone(), raw);
                        continue;
                    }
                    // Branch parity with the module-body and generate twins: a
                    // declared-integral package parameter whose initializer mentions a
                    // real converts at the declaration (§6.24.1). The meta is
                    // recomputed a few lines below for `const_meta`; asking twice is
                    // cheaper than reordering a block whose restore list is positional.
                    let pmeta = self.param_decl_width_unoverridden(p);
                    let folded = (!self.param_init_kept_loud(p))
                        .then(|| self.const_eval_in_scope(&p.value))
                        .flatten()
                        .or_else(|| self.param_value_via_real(pmeta, &p.value))
                        .or_else(|| {
                            let dm = self.param_decl_width_declared(p);
                            self.param_i64_at_declared(&p.value, dm)
                        });
                    // The FOURTH domain, and the one the package fold never grew: a
                    // value too wide for i64. Reached only after the numeric fold
                    // declined, exactly like the module-body twin, so a wide
                    // DECLARATION whose value happens to fit keeps its integer
                    // identity and stays usable as a width and a bound.
                    {
                        if let Some(cv) = self.wide_disagreeing_value(&p.value, pmeta, folded) {
                            saved_wide.push((
                                key.clone(),
                                self.wide_param_bits.insert(key.clone(), cv.clone()),
                            ));
                            wide_vals.insert(p.name.name.clone(), cv);
                            if let Some(r) = self.param_decl_range_opt(p, true) {
                                const_range.insert(p.name.name.clone(), r);
                            }
                            if let Some(m) = pmeta {
                                const_meta.insert(p.name.name.clone(), m);
                                saved_meta.push((key.clone(), self.param_meta.insert(key, m)));
                            }
                            continue;
                        }
                    }
                    let v = folded.unwrap_or_else(|| {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "package parameter `{}` value is not a foldable constant",
                                p.name.name
                            ),
                        );
                        0
                    });
                    saved.push((key.clone(), self.bind_param_value(key.clone(), v)));
                    consts.insert(p.name.name.clone(), v);
                    // A package constant has no override channel.
                    if let Some(r) = self.param_decl_range_opt(p, true) {
                        const_range.insert(p.name.name.clone(), r);
                        saved_range.push((key.clone(), self.param_range.insert(key.clone(), r)));
                    } else {
                        // Set-or-CLEAR, the same discipline `param_meta` follows two
                        // lines down: a stale same-name entry from another scope must
                        // not answer for this declaration.
                        saved_range.push((key.clone(), self.param_range.remove(&key)));
                    }
                    if let Some(m) = self.param_decl_width_unoverridden(p) {
                        const_meta.insert(p.name.name.clone(), m);
                        // Make this param's meta visible to a LATER intra-package
                        // alias/expression (the ident/`const_expr_signed` arms read
                        // `param_meta`). Set-or-CLEAR so a stale same-name entry from
                        // another scope can't leak in on the no-meta path.
                        saved_meta.push((key.clone(), self.param_meta.insert(key, m)));
                    } else {
                        saved_meta.push((key.clone(), self.param_meta.remove(&key)));
                    }
                }
                ast::ModuleItem::Typedef(td) => {
                    // Every typedef name is a package TYPE (the parser resolves the type
                    // itself; this only lets an `import p::<name>` be recognized as a
                    // legal type import rather than an unknown symbol).
                    types.insert(td.name.name.clone());
                    #[allow(irrefutable_let_patterns)]
                    if let ast::TypedefKind::Enum {
                        base,
                        signed,
                        labels,
                    } = &td.kind
                    {
                        // Base width so an imported / `pkg::`-read label carries its
                        // self-width in a concat (twin of the module path); the enum's
                        // DECLARED sign so a positive label of a signed enum stays signed
                        // in a comparison — §4.5.158. ⚠️ DECLARED sign only; the old
                        // `|| v < 0` is gone, and `instance.rs` records what both oracles
                        // say instead.
                        // §4.5.158: a base-less `enum {…}` is `int` (32-bit) — give its labels
                        // an explicit 32-bit `param_meta` so the sign fix below reaches them too
                        // (`enum_base_width` returns None for a rangeless base). An unfoldable
                        // range stays None (unknown width, value-inferred as before).
                        let base_w = self
                            .enum_base_width(base)
                            .or_else(|| base.is_none().then_some(32u32));
                        let base_range = self.enum_base_range(base);
                        let mut next: i64 = 0;
                        for l in labels {
                            let v = match &l.value {
                                Some(e) => self.const_eval_in_scope(e).unwrap_or_else(|| {
                                    self.error(
                                        MsgCode::ElabUnsupported,
                                        &format!(
                                            "enum label `{}` value is not a foldable constant",
                                            l.name.name
                                        ),
                                    );
                                    0
                                }),
                                None => next,
                            };

                            // A2b-prereq: same single-name-space rule as params.
                            if vars.contains_key(&l.name.name) {
                                self.error(
                                    MsgCode::DupUnit,
                                    &format!(
                                        "package symbol `{}` declared more than \
                                         once (an enum label and a variable share \
                                         one name space)",
                                        l.name.name
                                    ),
                                );
                            }
                            // §6.19 on the value as WRITTEN — see `instance.rs`.
                            self.check_enum_label_fits(
                                base,
                                *signed,
                                &l.name.name,
                                v,
                                l.value.is_some(),
                            );
                            // ⭐ Canonical at the declared width — see `instance.rs`.
                            let v = match base_w {
                                Some(w) => Self::const_mask(v, w, *signed),
                                None => v,
                            };
                            // ⚠️ AFTER the mask, exactly as the module-scope and
                            // body-local twins do it. It used to be taken from the RAW
                            // fold, immediately after `const_eval_in_scope`, and that one
                            // line made the twins disagree: with
                            // `enum logic [7:0] { PA = -8'sd1, PB }` the module scope
                            // auto-increments 255 → 256 and is correctly rejected, while
                            // the package auto-incremented −1 → 0 and accepted a design
                            // both oracles reject. Found by a review lens reading the
                            // three "see `instance.rs`" comments and checking they were
                            // true.
                            // wrapping: an explicit label at i64::MAX must not panic the
                            // auto-increment (twin of the module-scope and body-local
                            // enum loops).
                            next = v.wrapping_add(1);
                            let key = self.fq(&l.name.name);
                            // Capture the prior range BEFORE binding — `bind_param_value`
                            // clears it, so this is the only moment it can be saved, and
                            // the restore list must hold the prior state and not a blanket
                            // "remove" (the same discipline as the parameter arm above).
                            let prev_range = self.param_range.get(&key).copied();
                            let prev = self.bind_param_value(key.clone(), v);
                            // The declared range travels with the label the way it
                            // travels with a package parameter, so `pk::EA[7:0]` and the
                            // bare-imported `EA[7:0]` fold through the same table.
                            self.bind_param_range(&key, base_range);
                            saved_range.push((key.clone(), prev_range));
                            consts.insert(l.name.name.clone(), v);
                            enum_labels.insert(l.name.name.clone(), td.name.name.clone());
                            if let Some(w) = base_w {
                                const_meta.insert(l.name.name.clone(), (w, *signed));
                                // …and make it LIVE for the rest of this package body,
                                // exactly as the parameter arm above does ("Make this
                                // param's meta visible to a LATER intra-package alias /
                                // expression"). Without it a label had a declared width
                                // for every consumer OUTSIDE the package and none inside
                                // it, so `localparam logic [15:0] WIDE = {A, B};` in the
                                // same package folded the concat at the wrong width —
                                // `0003`, losing the high label, where both oracles give
                                // `fe03`. Save/restore like every other entry here, so
                                // nothing leaks past the package.
                                saved_meta.push((
                                    key.clone(),
                                    self.param_meta.insert(key.clone(), (w, *signed)),
                                ));
                            }
                            saved.push((key, prev));
                            if let Some(r) = base_range {
                                const_range.insert(l.name.name.clone(), r);
                            }
                        }
                    }
                    // Alias/Struct typedefs ride the parser's unit-global
                    // typedef map (type NAMES are parse-resolved) — no
                    // elaborate-side symbol needed.
                }
                ast::ModuleItem::Func(f) => {
                    funcs.insert(f.name.name.clone(), f.clone());
                }
                ast::ModuleItem::Task(t) => {
                    tasks.insert(t.name.name.clone(), t.clone());
                }
                ast::ModuleItem::Import(imp) => {
                    // Family D (r17): apply an imported package's CONSTANTS into this
                    // package's fold scope (`base` is already fully elaborated — packages
                    // elaborate in declaration order — so `pkg_consts[base]` exists). TYPES
                    // are already resolved by the parser's unit-global typedef map, which is
                    // why a `base::byte8_t`-typed decl in `derived` parses. The
                    // wildcard-origin maps are package-loop-local; imported consts ride
                    // `saved` for restore at the package's end. A package-INTERNAL call to
                    // an imported ROUTINE stays a follow-on (routine resolution runs at the
                    // external call site, not here) → loud (correct-or-loud); imported types
                    // + consts now work.
                    let mut wc_origin: BTreeMap<String, String> = BTreeMap::new();
                    let mut explicit: std::collections::BTreeSet<String> =
                        std::collections::BTreeSet::new();
                    self.apply_import_consts(imp, &mut saved, &mut wc_origin, &mut explicit);
                }
                // A2b-prereq/A2b: package-level VARIABLE declaration — one
                // storage instance per elaboration (IEEE §26), lowered as an
                // ordinary net under the reserved `$pkg$<pkg>` prefix
                // (`cur_prefix` is already synthetic here). Plain vector
                // variable kinds only in v1. A const-foldable init rides the
                // NetVar `init` field; any other init (array `'{…}`,
                // non-constant scalar) is collected and flushed below as the
                // package's own §6.8 pre-sweep `initial` (before-t0 for every
                // module process — module parity, iverilog-pinned).
                ast::ModuleItem::NetVar(d) => {
                    self.elaborate_pkg_netvar(d, &param_names, &mut vars);
                    // GAP-G: capture a const array param's foldable element values
                    // (same shape rules as the module-scope `capture_const_array_vals`)
                    // keyed by the package-local name, for `p::ROT[i]` const folds.
                    if d.const_param {
                        for decl in &d.names {
                            if let Some(vals) = self.const_array_elem_vals(d, decl) {
                                array_vals.insert(decl.name.name.clone(), vals);
                            }
                        }
                    }
                }
                _ => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "only parameters/typedefs/functions/tasks/variables are \
                         supported in a package body (v7/A2b-prereq)",
                    );
                }
            }
        }
        // A2b: emit this package's collected decl-inits (array `'{…}`,
        // non-constant scalars) as the package's own §6.8 pre-sweep `initial`
        // — INSIDE the `$pkg$<pkg>` scope (lvalues/RHS resolve against the
        // package nets and still-live package params) and BEFORE the param
        // restore. Empty collection = no-op (flush early-returns), so init-free
        // packages stay byte-identical.
        //
        // §4.5.259: RANKED, like every other initializer. "Packages elaborate before any
        // instance, so this process's ProcId precedes every module process" stopped being
        // enough the moment initialization became a pre-arm PHASE — an unranked process
        // is not in that phase at all, so it ran after every module's initializers (a
        // module `int m = p::pv + 100;` read 0) and its writes produced events an
        // `always @(p::pv)` could see. `RANK_PACKAGE` sorts below every root instance.
        self.flush_ranked(Self::RANK_PACKAGE);
        for (k, prev) in saved.into_iter().rev() {
            match prev {
                Some(v) => {
                    self.bind_param_value(k, v);
                }
                None => {
                    self.unbind_param(&k);
                }
            }
        }
        // §3 ⑨: same unwind for the string / real side maps, in reverse push order and
        // set-or-REMOVE, so a package param can never outlive its package under a bare
        // key that a module-scope read would walk into.
        for (k, prev) in saved_real.into_iter().rev() {
            match prev {
                Some(v) => {
                    self.real_param_val.insert(k, v);
                }
                None => {
                    self.real_param_val.remove(&k);
                }
            }
        }
        for (k, prev) in saved_str.into_iter().rev() {
            match prev {
                Some(v) => {
                    self.str_param_raw.insert(k, v);
                }
                None => {
                    self.str_param_raw.remove(&k);
                }
            }
        }
        // Restore param_meta — the package's params were made live only for
        // intra-package alias resolution above; module-scope reads use
        // `pkg_const_meta` (persisted below), so these entries must not linger.
        for (k, prev) in saved_meta.into_iter().rev() {
            match prev {
                Some(m) => {
                    self.param_meta.insert(k, m);
                }
                None => {
                    self.param_meta.remove(&k);
                }
            }
        }
        for (k, prev) in saved_range.into_iter().rev() {
            match prev {
                Some(r) => {
                    self.param_range.insert(k, r);
                }
                None => {
                    self.param_range.remove(&k);
                }
            }
        }
        // The wide entries were made live for intra-package siblings (`localparam
        // [127:0] M = ~K;`) exactly as the string and real ones are; module-scope
        // reads go through `pkg_wide_bits`, so these must not linger either.
        for (k, prev) in saved_wide.into_iter().rev() {
            match prev {
                Some(cv) => {
                    self.wide_param_bits.insert(k, cv);
                }
                None => {
                    self.wide_param_bits.remove(&k);
                }
            }
        }
        self.cur_prefix = saved_prefix;
        self.pkg_consts.insert(pkg.clone(), consts);
        if !types.is_empty() {
            self.pkg_types.insert(pkg.clone(), types);
        }
        if !enum_labels.is_empty() {
            self.pkg_enum_labels.insert(pkg.clone(), enum_labels);
        }
        if !const_meta.is_empty() {
            self.pkg_const_meta.insert(pkg.clone(), const_meta);
        }
        if !const_range.is_empty() {
            self.pkg_const_range.insert(pkg.clone(), const_range);
        }
        if !real_vals.is_empty() {
            self.pkg_real_val.insert(pkg.clone(), real_vals);
        }
        if !str_vals.is_empty() {
            self.pkg_str_raw.insert(pkg.clone(), str_vals);
        }
        if !wide_vals.is_empty() {
            self.pkg_wide_bits.insert(pkg.clone(), wide_vals);
        }
        self.pkg_vars.insert(pkg.clone(), vars);
        if !array_vals.is_empty() {
            self.pkg_array_const_vals.insert(pkg.clone(), array_vals);
        }
        self.pkg_funcs.insert(pkg.clone(), funcs);
        self.pkg_tasks.insert(pkg, tasks);
    }

    /// A2b-prereq/A2b: lower ONE package-body variable declaration. v1 subset:
    /// plain vector VARIABLE kinds (`reg`/`logic`/`integer`/`time`/2-state
    /// atoms) with optional unpacked array dims — including a desugared array
    /// PARAMETER (`const_param`, A2a mechanism: the net registers into
    /// `const_param_nets`, so every write path stays loud). A const-foldable
    /// init rides `NetVar.init`; any other init (array `'{…}`, non-constant
    /// scalar — iverilog-supported) is COLLECTED for the package's own §6.8
    /// pre-sweep initial, flushed by `elaborate_package` (its ProcId precedes
    /// every module process, so module code sees the values at t0). Loud line:
    /// wire kinds are not package items (IEEE §26.2); event/string/real/
    /// class/dynamic storage are an unverified scope in v1 (scope-gate, §3).
    /// `param_names` is the package's PARAMETER NAME SPACE — every name declared as a
    /// parameter in this body, in ANY domain. ⚠️ It used to be the i64 `consts` map, which
    /// was the same set only while the package fold was integer-only: once §3 ⑨ routed
    /// string and real parameters out of `consts`, a `parameter S = "RED"; int S;`
    /// collision stopped being reported and ran at exit 0 (both oracles reject it), while
    /// the integer twin stayed loud. The check is about the NAME SPACE (IEEE §26.3), so
    /// it takes the name space.
    pub(crate) fn elaborate_pkg_netvar(
        &mut self,
        d: &ast::NetVarDecl,
        param_names: &std::collections::BTreeSet<String>,
        vars: &mut BTreeMap<String, u32>,
    ) {
        if d.kind.is_net() {
            self.error(
                MsgCode::ElabUnsupported,
                "a net declaration is not a package item (IEEE §26.2: a package \
                 holds variables — use a variable kind)",
            );
            return;
        }
        if !matches!(
            d.kind,
            ast::NetVarKind::Reg
                | ast::NetVarKind::Logic
                | ast::NetVarKind::Integer
                | ast::NetVarKind::Time
                | ast::NetVarKind::Bit
                | ast::NetVarKind::Byte
                | ast::NetVarKind::Shortint
                | ast::NetVarKind::Int
                | ast::NetVarKind::Longint
        ) {
            self.error(
                MsgCode::ElabUnsupported,
                "this variable kind in a package body is outside the v1 subset \
                 (plain vector variables only — event/string/real/class/virtual \
                 are a follow-on)",
            );
            return;
        }
        for decl in &d.names {
            if self.dyn_dim_kind(&decl.unpacked).is_some() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "package variable `{}`: dynamic storage (queue/dynamic/\
                         associative) in a package body is outside the v1 subset",
                        decl.name.name
                    ),
                );
                return;
            }
            if param_names.contains(&decl.name.name) {
                self.error(
                    MsgCode::DupUnit,
                    &format!(
                        "package symbol `{}` declared more than once (a parameter \
                         and a variable share one name space)",
                        decl.name.name
                    ),
                );
            }
        }
        // The shared decl path registers width/sign/2-state/array/const-param
        // sidecars exactly like a module-body variable; PortList::None + empty
        // body ⇒ every name is `Internal`. A non-foldable init defaults inside
        // and is collected below for the package pre-sweep (module parity).
        self.elaborate_netvar_decl(d, &ast::PortList::None, &[], false);
        self.collect_var_init_drivers(d);
        for decl in &d.names {
            if let Some(&id) = self.symbols.get(&self.fq(&decl.name.name)) {
                vars.insert(decl.name.name.clone(), id);
            }
        }
    }

    /// Bind one import's CONST symbols into the current module scope.
    pub(crate) fn apply_import_consts(
        &mut self,
        imp: &ast::ImportDecl,
        saved_params: &mut Vec<(String, Option<i64>)>,
        wc_origin: &mut BTreeMap<String, String>,
        explicit_imports: &mut std::collections::BTreeSet<String>,
    ) {
        let pkg = imp.pkg.name.as_str();
        let Some(consts) = self.pkg_consts.get(pkg) else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("import from unknown package `{pkg}`"),
            );
            return;
        };
        // V33-3 (DIAGNOSTIC ONLY): which of the imported names are enum LABELS, and of
        // which enum. Cloned up front because both binding arms below need it while
        // `self` is borrowed mutably; empty for every package without a `typedef enum`.
        let pkg_labels: BTreeMap<String, String> =
            self.pkg_enum_labels.get(pkg).cloned().unwrap_or_default();
        match &imp.item {
            None => {
                // Carry each const's declared `(width, signed)` alongside its
                // value so a bare-imported read materializes at the right width
                // (the pkg twin of a local param's `param_meta` entry).
                let all = consts
                    .iter()
                    .map(|(k, &v)| {
                        let m = self
                            .pkg_const_meta
                            .get(pkg)
                            .and_then(|mm| mm.get(k))
                            .copied();
                        let r = self
                            .pkg_const_range
                            .get(pkg)
                            .and_then(|mm| mm.get(k))
                            .copied();
                        (k.clone(), v, m, r)
                    })
                    .collect::<Vec<_>>();
                for (name, v, meta, rng) in all {
                    let key = self.fq(&name);
                    // An explicit import of this name always wins — skip the wildcard.
                    if explicit_imports.contains(&key) {
                        continue;
                    }
                    match wc_origin.get(&key).map(String::as_str) {
                        // "" sentinel = already ambiguous: stays unbound.
                        Some("") => continue,
                        // A different package already wildcard-bound this name ⇒
                        // ambiguous. Unbind it (save the prior value for restore) so a
                        // reference is loud-undefined, and mark it ambiguous.
                        // A2b-prereq: `wc_origin` is shared by the CONST and the
                        // VARIABLE namespaces (§26.8 ambiguity is per NAME) — the
                        // prior binding may live in either map; unbind both sides
                        // (the alias-guarded symbols removal never touches a real
                        // net: only keys this import machinery inserted).
                        Some(prev) if prev != pkg => {
                            let prev_val = self.unbind_param(&key);
                            saved_params.push((key.clone(), prev_val));
                            if self.pkg_var_aliases.remove(&key).is_some() {
                                self.symbols.remove(&key);
                            }
                            wc_origin.insert(key, String::new());
                        }
                        // Same package re-import: idempotent, keep the binding.
                        Some(_) => {}
                        None => {
                            saved_params.push((key.clone(), self.bind_param_value(key.clone(), v)));
                            if let Some(m) = meta {
                                self.param_meta.insert(key.clone(), m);
                            }
                            // The DECLARED range travels with the value, so a select
                            // of the bare-imported name normalizes against the
                            // package's declaration exactly as `pkg::W[m:l]` does.
                            // Set-or-CLEAR: a name with no declared-range entry must
                            // not inherit a stale one from another binding of the
                            // same key.
                            self.bind_param_range(&key, rng);
                            if let Some(ty) = pkg_labels.get(&name) {
                                self.enum_label_types.insert(key.clone(), ty.clone());
                            }
                            wc_origin.insert(key, pkg.to_string());
                        }
                    }
                }
                // The WIDE consts ride the same wildcard, from their own side map:
                // they are not in `consts` (no i64 value), so without this pass a
                // `import p::*;` bound every narrow parameter of the package and
                // silently skipped the 128-bit ones.
                let wides: Vec<(String, ir::ConstVal)> = self
                    .pkg_wide_bits
                    .get(pkg)
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();
                for (name, cv) in wides {
                    let key = self.fq(&name);
                    if explicit_imports.contains(&key) {
                        continue;
                    }
                    match wc_origin.get(&key).map(String::as_str) {
                        Some("") => continue,
                        Some(prev) if prev != pkg => {
                            self.wide_param_bits.remove(&key);
                            wc_origin.insert(key, String::new());
                        }
                        Some(_) => {}
                        None => {
                            self.wide_param_bits.insert(key.clone(), cv);
                            if let Some(m) = self
                                .pkg_const_meta
                                .get(pkg)
                                .and_then(|mm| mm.get(&name))
                                .copied()
                            {
                                self.param_meta.insert(key.clone(), m);
                            }
                            wc_origin.insert(key, pkg.to_string());
                        }
                    }
                }
                // A2b-prereq: wildcard-bind the package's VARIABLES as symbol
                // aliases (interface-alias precedent — one insertion covers
                // every read/write name→net funnel). Same origin/ambiguity
                // rules as the consts above, shared `wc_origin`.
                let vars: Vec<(String, u32)> = self
                    .pkg_vars
                    .get(pkg)
                    .map(|m| m.iter().map(|(k, &n)| (k.clone(), n)).collect())
                    .unwrap_or_default();
                for (name, net) in vars {
                    let key = self.fq(&name);
                    if explicit_imports.contains(&key) {
                        continue;
                    }
                    match wc_origin.get(&key).map(String::as_str) {
                        Some("") => continue,
                        Some(prev) if prev != pkg => {
                            let prev_val = self.unbind_param(&key);
                            saved_params.push((key.clone(), prev_val));
                            if self.pkg_var_aliases.remove(&key).is_some() {
                                self.symbols.remove(&key);
                            }
                            wc_origin.insert(key, String::new());
                        }
                        Some(_) => {}
                        None => {
                            // Never clobber a symbols entry this machinery did
                            // not create (e.g. an interface-port alias): the
                            // existing binding wins, like a local declaration.
                            // Same for a HEADER parameter already bound (3a
                            // runs before this 3a.5): local constant wins the
                            // wildcard (iverilog local-wins pin).
                            if self.params.contains_key(&key)
                                || (self.symbols.contains_key(&key)
                                    && !self.pkg_var_aliases.contains_key(&key))
                            {
                                continue;
                            }
                            self.symbols.insert(key.clone(), net);
                            self.pkg_var_aliases
                                .insert(key.clone(), (pkg.to_string(), false));
                            wc_origin.insert(key, pkg.to_string());
                        }
                    }
                }
            }
            Some(sym) => {
                if let Some(cv) = self
                    .pkg_wide_bits
                    .get(pkg)
                    .and_then(|m| m.get(&sym.name))
                    .cloned()
                {
                    let key = self.fq(&sym.name);
                    explicit_imports.insert(key.clone());
                    if self.pkg_var_aliases.remove(&key).is_some() {
                        self.symbols.remove(&key);
                    }
                    if let Some(m) = self
                        .pkg_const_meta
                        .get(pkg)
                        .and_then(|mm| mm.get(&sym.name))
                        .copied()
                    {
                        self.param_meta.insert(key.clone(), m);
                    }
                    self.wide_param_bits.insert(key, cv);
                } else if let Some(&v) = consts.get(&sym.name) {
                    let key = self.fq(&sym.name);
                    explicit_imports.insert(key.clone());
                    // A2b-prereq S4 (symmetric): an explicit CONST import wins
                    // over a prior wildcard VARIABLE binding (§26.8) — drop the
                    // now-dead alias so no write path can still reach it.
                    if self.pkg_var_aliases.remove(&key).is_some() {
                        self.symbols.remove(&key);
                    }
                    // Bare-imported read materializes at the declared width (the
                    // pkg twin of a local param's `param_meta` entry).
                    if let Some(m) = self
                        .pkg_const_meta
                        .get(pkg)
                        .and_then(|mm| mm.get(&sym.name))
                        .copied()
                    {
                        self.param_meta.insert(key.clone(), m);
                    }
                    // Same pairing as the wildcard arm: the declared range binds with
                    // the value it describes, or the key is cleared.
                    let rng = self
                        .pkg_const_range
                        .get(pkg)
                        .and_then(|mm| mm.get(&sym.name))
                        .copied();
                    let prev = self.bind_param_value(key.clone(), v);
                    self.bind_param_range(&key, rng);
                    if let Some(ty) = pkg_labels.get(&sym.name) {
                        self.enum_label_types.insert(key.clone(), ty.clone());
                    }
                    saved_params.push((key, prev));
                } else if let Some(&net) = self.pkg_vars.get(pkg).and_then(|m| m.get(&sym.name)) {
                    // A2b-prereq: explicit VARIABLE import — bind the alias and
                    // mark it explicit (a later local declaration of this name
                    // is a loud conflict in `add_net`, iverilog-pinned). A name
                    // ALREADY locally bound (header param / earlier net) is the
                    // same conflict, just discovered in the other order — but a
                    // prior WILDCARD binding (const or var, S4) is not a local
                    // declaration: the explicit import wins (§26.8).
                    let key = self.fq(&sym.name);
                    let wildcard_bound = wc_origin.get(&key).is_some_and(|p| !p.is_empty());
                    if (self.params.contains_key(&key) && !wildcard_bound)
                        || (self.symbols.contains_key(&key)
                            && !self.pkg_var_aliases.contains_key(&key))
                    {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "explicit import of `{}` from package `{pkg}` \
                                 conflicts with a local declaration of the same \
                                 name",
                                sym.name
                            ),
                        );
                    } else {
                        if wildcard_bound {
                            // unbind the losing wildcard (either namespace).
                            let prev = self.unbind_param(&key);
                            saved_params.push((key.clone(), prev));
                            self.pkg_var_aliases.remove(&key);
                        }
                        explicit_imports.insert(key.clone());
                        self.symbols.insert(key.clone(), net);
                        self.pkg_var_aliases.insert(key, (pkg.to_string(), true));
                    }
                } else if !self
                    .pkg_funcs
                    .get(pkg)
                    .is_some_and(|f| f.contains_key(&sym.name))
                    && !self
                        .pkg_tasks
                        .get(pkg)
                        .is_some_and(|t| t.contains_key(&sym.name))
                    && !self
                        .pkg_types
                        .get(pkg)
                        .is_some_and(|t| t.contains(&sym.name))
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("package `{pkg}` has no symbol `{}`", sym.name),
                    );
                }
                // A type import (`import p::my_t`) binds nothing in elaborate — the
                // parser already copied the scoped type twin to the bare name.
            }
        }
    }

    /// Bind one import's FUNCTION/TASK symbols (local definitions win —
    /// skip-if-present, called after the module's own (3.5) collection).
    /// The `func_table`/`task_table` key a BARE callee name resolves to.
    ///
    /// Normally the name itself. Inside the body of a routine declared in package
    /// `p`, `p::name` wins when it exists — that is the whole point of the scoped
    /// injection in `apply_import_routines`: a package routine's body must see its
    /// OWN siblings, not whatever the importing module gave the same name. Without
    /// this, `import p::f2;` in a module that also declares its own `helper` made
    /// `f2`'s body call the module's `helper` at exit 0 (measured: 1002 where
    /// iverilog says 4 — an earlier note said 2002, which the design as written does
    /// not produce).
    pub(crate) fn resolve_rtn_key(&self, bare: &str) -> String {
        if let Some(pkg) = self.cur_rtn_pkg.last() {
            let scoped = format!("{pkg}::{bare}");
            if self.func_table.contains_key(&scoped) || self.task_table.contains_key(&scoped) {
                return scoped;
            }
        }
        bare.to_string()
    }

    /// The package a routine KEY belongs to: `pkg::name` carries it in the key, an
    /// explicitly imported bare name carries it in `rtn_pkg`, everything else is a
    /// plain module routine.
    pub(crate) fn rtn_key_pkg(&self, key: &str) -> Option<String> {
        match key.split_once("::") {
            Some((pkg, _)) => Some(pkg.to_string()),
            None => self.rtn_pkg.get(key).cloned(),
        }
    }

    /// Make every same-package routine that `root` transitively CALLS reachable
    /// under its scoped key `pkg::name`.
    ///
    /// A routine's body resolves in its own declaring scope (IEEE 1800 §26.3), but
    /// vita lowers every body with the caller module's flat tables live. Scoped keys
    /// are how the body still finds its own siblings: `resolve_rtn_key` prefers
    /// `pkg::n` while that body is being lowered, and nothing else can see the key,
    /// so a module-local `n` is untouched and two packages can never collide.
    ///
    /// Shared by both entry points — an explicit `import pkg::f;` and a scoped
    /// `pkg::f()` call — because they need exactly the same thing.
    pub(crate) fn inject_pkg_callees(&mut self, pkg: &str, root: &str) -> Vec<String> {
        let (funcs, tasks) = match (self.pkg_funcs.get(pkg), self.pkg_tasks.get(pkg)) {
            (Some(f), Some(t)) => (f.clone(), t.clone()),
            _ => return Vec::new(),
        };
        // ⚠️ Every injected callee must be FREE-NAME CLOSED, not just the root.
        // The admission (`pkg_func_self_contained`) ran on the root only while this
        // walk is TRANSITIVE, so a sibling whose body names something free was
        // injected anyway and then lowered with the caller module's tables live —
        // binding that free name to the CALLER's net, silently:
        //   package p; function h(x); return x + zz; endfunction   // zz free
        //              function g(m); return h(m);   endfunction
        //   module t; int zz; … p::g(1) → 101, iverilog: "Unable to bind zz in p.h"
        // A callee that fails the check is NOT injected and its name is returned, so
        // the caller can say why instead of leaving a bare "undeclared function".
        let const_names: std::collections::BTreeSet<String> = self
            .pkg_consts
            .get(pkg)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let rtn_names: std::collections::BTreeSet<String> =
            funcs.keys().chain(tasks.keys()).cloned().collect();
        let mut refused: Vec<String> = Vec::new();
        let pkg = pkg.to_string();
        let mut want: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        if let Some(f) = funcs.get(root) {
            collect_callee_stmt(&f.body, &mut want);
        }
        if let Some(t) = tasks.get(root) {
            collect_callee_stmt(&t.body, &mut want);
        }
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        seen.insert(root.to_string());
        while let Some(n) = want.iter().next().cloned() {
            want.remove(&n);
            if !seen.insert(n.clone()) {
                continue;
            }
            let f = funcs.get(&n).cloned();
            let t = tasks.get(&n).cloned();
            if f.is_none() && t.is_none() {
                continue; // not this package's — a local, or another import
            }
            let key = format!("{pkg}::{n}");
            if let Some(f) = f {
                collect_callee_stmt(&f.body, &mut want);
                if pkg_func_self_contained(&f, &const_names, &rtn_names) {
                    self.func_table.entry(key.clone()).or_insert(f);
                } else {
                    refused.push(n.clone());
                }
            }
            if let Some(t) = t {
                collect_callee_stmt(&t.body, &mut want);
                if pkg_task_self_contained(&t, &const_names, &rtn_names) {
                    self.task_table.entry(key).or_insert(t);
                } else {
                    refused.push(n.clone());
                }
            }
        }
        refused
    }

    /// [`pkg_func_self_contained`] for a TASK — same write/read sets, same walk.
    /// A task has no return-by-name, so its own name is not writable.
    fn task_self_contained_impl(
        task: &ast::TaskDef,
        pkg_const_names: &std::collections::BTreeSet<String>,
        pkg_rtn_names: &std::collections::BTreeSet<String>,
    ) -> bool {
        let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in &task.ports {
            names.insert(p.name.name.clone());
        }
        let mut decls = task.body_decls.clone();
        collect_block_local_decls(&task.body, &mut decls);
        for d in &decls {
            for n in &d.names {
                names.insert(n.name.name.clone());
            }
        }
        let read_names: std::collections::BTreeSet<String> =
            names.union(pkg_const_names).cloned().collect();
        pkg_stmt_pure_with(&task.body, &names, &read_names, pkg_rtn_names)
    }

    pub(crate) fn apply_import_routines(
        &mut self,
        imp: &ast::ImportDecl,
        wc_rtn: &mut BTreeMap<String, String>,
        explicit_rtn: &mut std::collections::BTreeSet<String>,
    ) {
        // Same-package siblings a WILDCARD-imported routine needs; injected after the
        // loop below (the borrow of `funcs` ends there).
        let mut wildcard_roots: Vec<String> = Vec::new();
        let pkg = imp.pkg.name.to_string();
        let (funcs, tasks) = match (self.pkg_funcs.get(&pkg), self.pkg_tasks.get(&pkg)) {
            (Some(f), Some(t)) => (f.clone(), t.clone()),
            _ => return, // unknown package already diagnosed in the const pass
        };
        match &imp.item {
            None => {
                for (n, f) in funcs {
                    if explicit_rtn.contains(&n) {
                        continue; // explicit import wins
                    }
                    // a LOCAL definition (present, not from a wildcard import) wins.
                    if self.func_table.contains_key(&n) && !wc_rtn.contains_key(&n) {
                        continue;
                    }
                    match wc_rtn.get(&n).map(String::as_str) {
                        Some("") => {
                            self.func_table.remove(&n); // already ambiguous: stay unbound
                        }
                        Some(prev) if prev != pkg => {
                            self.func_table.remove(&n); // two wildcard imports ⇒ ambiguous
                            wc_rtn.insert(n, String::new());
                        }
                        Some(_) => {} // same package, idempotent
                        None => {
                            self.func_table.insert(n.clone(), f);
                            self.rtn_pkg.insert(n.clone(), pkg.clone());
                            // ★ The wildcard arm needs the SCOPED injection too. It
                            // looked like it did not — a wildcard puts every sibling in
                            // under its bare name — but a module-local routine of the
                            // same name wins that bare slot, and the imported body then
                            // called the MODULE's version: 1002 where iverilog says 4,
                            // silently, on PRE and POST alike. Only the explicit-import
                            // spelling of the same design was fixed.
                            wildcard_roots.push(n.clone());
                            wc_rtn.insert(n, pkg.clone());
                        }
                    }
                }
                for (n, t) in tasks {
                    if explicit_rtn.contains(&n) {
                        continue;
                    }
                    if self.task_table.contains_key(&n) && !wc_rtn.contains_key(&n) {
                        continue;
                    }
                    match wc_rtn.get(&n).map(String::as_str) {
                        Some("") => {
                            self.task_table.remove(&n);
                        }
                        Some(prev) if prev != pkg => {
                            self.task_table.remove(&n);
                            wc_rtn.insert(n, String::new());
                        }
                        Some(_) => {}
                        None => {
                            self.task_table.insert(n.clone(), t);
                            self.rtn_pkg.insert(n.clone(), pkg.clone());
                            wildcard_roots.push(n.clone());
                            wc_rtn.insert(n, pkg.clone());
                        }
                    }
                }
            }
            Some(sym) => {
                // explicit import always wins (override + protect from wildcards).
                explicit_rtn.insert(sym.name.clone());
                if let Some(f) = funcs.get(&sym.name) {
                    self.func_table.insert(sym.name.clone(), f.clone());
                    self.rtn_pkg.insert(sym.name.clone(), pkg.clone());
                }
                if let Some(t) = tasks.get(&sym.name) {
                    self.task_table.insert(sym.name.clone(), t.clone());
                    self.rtn_pkg.insert(sym.name.clone(), pkg.clone());
                }
                // ★ A routine's body resolves in ITS OWN declaring scope (IEEE 1800
                // §26.3: an import controls what the IMPORTING scope may name, not how
                // the imported item's already-declared body resolves). vita lowers every
                // body with the caller module's flat `func_table` live, so
                // `import p::f2;` used to leave `f2`'s own call to same-package `f1`
                // unresolvable (E3010) while `import p::*` worked BY ACCIDENT — it
                // happens to put every sibling in the table under its bare name.
                //
                // Pull in the same-package routines the imported one actually needs
                // (transitive callees) under the SCOPED key `pkg::name` only. Scoped
                // keys are an existing shape (`inline_pkg_function` builds them for
                // `p::f()` calls), and keying this way is both simpler and MORE
                // conformant than injecting bare names: the module still cannot call
                // `f1` by its bare name, a module-local `f1` is untouched, and two
                // packages can never collide — so no ambiguity bookkeeping is needed.
                // `resolve_rtn_key` is the lookup half: while a package routine's body
                // is being lowered, a bare callee tries `pkg::name` first.
                // A sibling that is not free-name closed stays out (it would bind its
                // free name to a caller net); the call site is then loud, which is the
                // pre-existing behaviour for that shape. Not escalated here because an
                // import is not a use — the module may never call it.
                let _ = self.inject_pkg_callees(&pkg, &sym.name);
            }
        }
        for r in wildcard_roots {
            let _ = self.inject_pkg_callees(&pkg, &r);
        }
    }
}
