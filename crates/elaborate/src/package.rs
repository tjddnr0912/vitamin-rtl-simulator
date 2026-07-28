//! packages / imports — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// IEEE §26.3 (round-7): is `func` a SELF-CONTAINED, straight-line function safe to
/// FRAME-lower for a package-scoped call `pkg::f(args)`? True iff its body has no
/// control flow and every bare identifier / call it references is one of its own
/// formals or a body-local declaration (no free / package-internal reference that could
/// mis-resolve or collide with a net in the caller's module scope, since the frame body
/// is lowered in that scope). See `inline_pkg_function`. Multiple / non-final `return`s
/// are fine — the frame path models the first-return exit faithfully (unlike the inline
/// fold, which is why this feature routes through the frame path).
pub(crate) fn pkg_func_self_contained(
    func: &ast::FunctionDef,
    pkg_const_names: &std::collections::BTreeSet<String>,
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
    pkg_stmt_pure(&func.body, &names, &read_names)
}

/// Straight-line + free-name-closed check for one statement (see
/// `pkg_func_self_contained`). `write` = assignable names; `read` = readable
/// names (write set ∪ same-package consts).
pub(crate) fn pkg_stmt_pure(
    s: &ast::Stmt,
    write: &std::collections::BTreeSet<String>,
    read: &std::collections::BTreeSet<String>,
) -> bool {
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
    ) -> (
        Vec<(String, Option<i64>)>,
        Vec<(String, Option<(u32, bool)>)>,
    ) {
        let mut saved_p = Vec::new();
        let mut saved_m = Vec::new();
        let consts = match self.pkg_consts.get(pkg) {
            Some(c) => c.clone(),
            None => return (saved_p, saved_m),
        };
        // Skip-set = formals + body-local decls + the function's own name (same
        // construction as `pkg_func_self_contained`).
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
        let metas = self.pkg_const_meta.get(pkg).cloned().unwrap_or_default();
        for (cname, &v) in &consts {
            if skip.contains(cname) {
                continue;
            }
            let key = self.fq(cname);
            saved_p.push((key.clone(), self.params.insert(key.clone(), v)));
            if let Some(&m) = metas.get(cname) {
                saved_m.push((key.clone(), self.param_meta.insert(key, m)));
            }
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
        let mut consts: BTreeMap<String, i64> = BTreeMap::new();
        // Declared `(width, signed)` per PARAM const (flushed to
        // `pkg_const_meta`) so a `pkg::x` / bare-imported read gets its true
        // self-width in a concat/replication (see the field doc).
        let mut const_meta: BTreeMap<String, (u32, bool)> = BTreeMap::new();
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
                    let v = self.const_eval_in_scope(&p.value).unwrap_or_else(|| {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "package parameter `{}` value is not a foldable constant",
                                p.name.name
                            ),
                        );
                        0
                    });
                    // A2b-prereq: params and variables share the package's single
                    // name space (IEEE §26.3) — a duplicate is loud, never a
                    // silent double-binding (`add_net` only guards net-vs-net).
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
                    saved.push((key.clone(), self.params.insert(key.clone(), v)));
                    consts.insert(p.name.name.clone(), v);
                    if let Some(m) = self.param_decl_width(p) {
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
                        // DECLARED sign (`|| v < 0` graceful) so a positive label of a
                        // signed enum stays signed in a comparison — §4.5.158.
                        // §4.5.158: a base-less `enum {…}` is `int` (32-bit) — give its labels
                        // an explicit 32-bit `param_meta` so the sign fix below reaches them too
                        // (`enum_base_width` returns None for a rangeless base). An unfoldable
                        // range stays None (unknown width, value-inferred as before).
                        let base_w = self
                            .enum_base_width(base)
                            .or_else(|| base.is_none().then_some(32u32));
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
                            // wrapping: an explicit label at i64::MAX must not
                            // panic the auto-increment (twin of the module-scope
                            // and body-local enum loops).
                            next = v.wrapping_add(1);
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
                            let key = self.fq(&l.name.name);
                            saved.push((key.clone(), self.params.insert(key, v)));
                            consts.insert(l.name.name.clone(), v);
                            if let Some(w) = base_w {
                                const_meta.insert(l.name.name.clone(), (w, *signed || v < 0));
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
                    self.elaborate_pkg_netvar(d, &consts, &mut vars);
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
                    self.params.insert(k, v);
                }
                None => {
                    self.params.remove(&k);
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
        self.cur_prefix = saved_prefix;
        self.pkg_consts.insert(pkg.clone(), consts);
        if !types.is_empty() {
            self.pkg_types.insert(pkg.clone(), types);
        }
        if !const_meta.is_empty() {
            self.pkg_const_meta.insert(pkg.clone(), const_meta);
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
    pub(crate) fn elaborate_pkg_netvar(
        &mut self,
        d: &ast::NetVarDecl,
        consts: &BTreeMap<String, i64>,
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
            if consts.contains_key(&decl.name.name) {
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
                        (k.clone(), v, m)
                    })
                    .collect::<Vec<_>>();
                for (name, v, meta) in all {
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
                            let prev_val = self.params.remove(&key);
                            saved_params.push((key.clone(), prev_val));
                            if self.pkg_var_aliases.remove(&key).is_some() {
                                self.symbols.remove(&key);
                            }
                            wc_origin.insert(key, String::new());
                        }
                        // Same package re-import: idempotent, keep the binding.
                        Some(_) => {}
                        None => {
                            saved_params.push((key.clone(), self.params.insert(key.clone(), v)));
                            if let Some(m) = meta {
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
                            let prev_val = self.params.remove(&key);
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
                if let Some(&v) = consts.get(&sym.name) {
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
                    saved_params.push((key.clone(), self.params.insert(key, v)));
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
                            let prev = self.params.remove(&key);
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
    pub(crate) fn apply_import_routines(
        &mut self,
        imp: &ast::ImportDecl,
        wc_rtn: &mut BTreeMap<String, String>,
        explicit_rtn: &mut std::collections::BTreeSet<String>,
    ) {
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
                }
                if let Some(t) = tasks.get(&sym.name) {
                    self.task_table.insert(sym.name.clone(), t.clone());
                }
            }
        }
    }
}
