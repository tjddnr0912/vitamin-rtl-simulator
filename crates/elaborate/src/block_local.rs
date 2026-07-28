//! procedural block-local hoisting — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Collect every NESTED `begin…end`/`fork` block (i.e. not the top-level body
/// block) together with the names it declares as locals. `is_top` is true only for
/// the outermost body statement, whose own decls are function/task-scoped (not
/// block-scoped) and so are skipped.
pub(crate) fn gather_nested_block_locals(
    s: &ast::Stmt,
    is_top: bool,
    out: &mut Vec<(ast::Span, Vec<String>)>,
) {
    use ast::Stmt::*;
    match s {
        Block {
            stmts, decls, span, ..
        }
        | Fork {
            stmts, decls, span, ..
        } => {
            if !is_top && !decls.is_empty() {
                let names: Vec<String> = decls
                    .iter()
                    .flat_map(|d| d.names.iter().map(|n| n.name.name.clone()))
                    .collect();
                if !names.is_empty() {
                    out.push((*span, names));
                }
            }
            for st in stmts {
                gather_nested_block_locals(st, false, out);
            }
        }
        If { then_s, else_s, .. } => {
            gather_nested_block_locals(then_s, false, out);
            if let Some(e) = else_s {
                gather_nested_block_locals(e, false, out);
            }
        }
        Case { items, .. } => {
            for it in items {
                match it {
                    ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                        gather_nested_block_locals(body, false, out)
                    }
                }
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            gather_nested_block_locals(body, false, out)
        }
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => {
            if let Some(b) = body {
                gather_nested_block_locals(b, false, out)
            }
        }
        _ => {}
    }
}

/// Collect every `begin`/`fork` block-local declaration in `s`, in source order
/// (recursing into nested blocks and control-flow bodies). Used to reserve a
/// frame body's block-locals under its `$func$<name>` scope so a local declared
/// inside a `begin … end` resolves (previously unresolved → E3010, for frame
/// functions AND tasks alike).
pub(crate) fn collect_block_local_decls(s: &ast::Stmt, out: &mut Vec<ast::NetVarDecl>) {
    use ast::Stmt::*;
    match s {
        Block { decls, stmts, .. } | Fork { decls, stmts, .. } => {
            out.extend(decls.iter().cloned());
            for st in stmts {
                collect_block_local_decls(st, out);
            }
        }
        If { then_s, else_s, .. } => {
            collect_block_local_decls(then_s, out);
            if let Some(e) = else_s {
                collect_block_local_decls(e, out);
            }
        }
        Case { items, .. } => {
            for it in items {
                match it {
                    ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                        collect_block_local_decls(body, out)
                    }
                }
            }
        }
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            collect_block_local_decls(body, out)
        }
        _ => {}
    }
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

    /// BL4 (round-19): resolve a callee NAME to its formal port DIRECTIONS, in port
    /// order, for the block-local definite-assignment gate. A single-segment name is
    /// looked up in `func_table` then `task_table` (both hold the per-module AST def
    /// with `ports: Vec<TfPort>`). Returns `None` for a hierarchical `u.f` (the child
    /// module's ports are out of scope here), an unknown name, or a `$system` call —
    /// leaving the DA walk to treat the reference conservatively as a read.
    fn callee_port_dirs(&self, callee: &ast::HierPath) -> Option<Vec<ast::PortDir>> {
        if callee.segments.len() != 1 {
            return None;
        }
        let nm = callee.segments[0].name.as_str();
        if let Some(f) = self.func_table.get(nm) {
            return Some(f.ports.iter().map(|p| p.dir).collect());
        }
        if let Some(t) = self.task_table.get(nm) {
            return Some(t.ports.iter().map(|p| p.dir).collect());
        }
        None
    }

    /// BL4 (round-19): true iff the call `callee(args)` DEFINITELY (whole-var) WRITES
    /// `name` through a PURE OUTPUT actual and READS `name` at NO position of the same
    /// call. This is the resolver threaded into `automatic_local_definitely_assigned` /
    /// `da_stmt` (see [`crate::da::OutActualWrites`]). Soundness (this is an ACCEPT gate
    /// — a wrong "assigned" reads a leftover/X value instead of erroring = silent-wrong):
    ///   * ONLY a bare whole-var `Ident(name)` at an OUTPUT formal position counts as a
    ///     write (copy-out only, no copy-in). A SELECT actual `name[i]` is a PARTIAL
    ///     write (the other bits stay unwritten) → NOT a definite whole assignment; it
    ///     is caught as a READ below and blocks the accept.
    ///   * An INOUT actual is NOT a write here: its copy-IN reads `name`'s current value
    ///     (the leftover on the v1 flatten ≠ a fresh automatic's default), so an inout
    ///     `name` is a READ (blocks the accept unless `name` is already assigned).
    ///   * `name` at ANY input actual, or inside a select index of any actual, or at an
    ///     arg beyond the resolved formals, is a READ.
    ///   * Named args (can't map positionally) or an unresolvable callee → `false`.
    ///
    /// The verdict is `writes && !reads`: a genuine output-write with no shadow read.
    fn call_out_actual_writes(
        &self,
        callee: &ast::HierPath,
        args: &[ast::Expr],
        name: &str,
    ) -> bool {
        // Named args defeat positional formal mapping → conservative (stays loud).
        if args
            .iter()
            .any(|a| matches!(a.kind, ast::ExprKind::NamedArg { .. }))
        {
            return false;
        }
        let Some(dirs) = self.callee_port_dirs(callee) else {
            return false;
        };
        let mut writes = false;
        let mut reads = false;
        for (i, arg) in args.iter().enumerate() {
            // A clean whole-var OUTPUT actual `name` at an OUTPUT formal — copy-out only.
            let clean_output_whole = matches!(dirs.get(i), Some(ast::PortDir::Output))
                && matches!(&arg.kind, ast::ExprKind::Ident(p)
                    if p.segments.len() == 1 && p.segments[0].name == name);
            if clean_output_whole {
                writes = true;
            } else if !expr_no_ref(arg, name) {
                // Any OTHER reference to `name` (input actual, inout copy-in, select
                // index, partial-write select base, arg beyond arity, OR a same-call
                // member / method read `name.field` / `name.size()`) is a read that
                // observes the copy-IN value before the output copy-OUT. Use the
                // CONSERVATIVE `expr_no_ref` (any unvetted form ⇒ "may reference"), NOT
                // `expr_reads_ident` — the latter is the known under-detecting walker
                // (single-seg only, no `MethodCall`/`Dist`/… arm), so a co-arg
                // `name.field` (a multi-seg ident) / `name.method()` would slip through
                // and let the accept gate skip the loud → the flattened net's leftover
                // is read on re-entry = silent-wrong (BL4 adversarial-review finding;
                // latent today since member/method-bearing OUTPUT-formal types are
                // otherwise loud, but this keeps the gate sound as support widens).
                reads = true;
            }
        }
        writes && !reads
    }

    /// BL4 (round-19): [`automatic_local_definitely_assigned`] with this Elaborator's
    /// output-actual resolver ([`Self::call_out_actual_writes`]) bound in — so a call
    /// whose OUTPUT actual is `name` (unconditionally evaluated) is seen as a definite
    /// assignment, not a read-before-write. `&self` (immutable) — the returned `bool`
    /// releases the borrow before the surrounding `&mut self` hoist continues.
    fn block_local_definitely_assigned(&self, stmts: &[ast::Stmt], name: &str) -> bool {
        automatic_local_definitely_assigned(stmts, name, &|cn, args, nm| {
            self.call_out_actual_writes(cn, args, nm)
        })
    }

    /// §6.21: a STATIC block-local's initializer runs ONCE at time zero, before any
    /// process starts; an `automatic` sibling's runs on each BLOCK ENTRY. So a static
    /// initializer that reads a per-entry sibling reads it at a time the automatic has
    /// not been initialized — and worse, anything the static initializer WRITES back
    /// into that sibling (an `inout`/`output` copy-out: `int z = f(c);`) is then clobbered
    /// by the entry re-init. Both values are wrong and neither is recoverable, so this
    /// combination is loud.
    ///
    /// Found while lifting the fork restriction (§4.5.248): the whole fork arm used to be
    /// loud for a different reason, which masked this. The identical NON-fork shape was
    /// already silently printing `c=5` where the copy-out said 6 — so this is a
    /// silent-wrong being raised to loud, not a capability being taken away.
    fn deny_static_init_reading_per_entry(&mut self, decls: &[ast::NetVarDecl], span: ast::Span) {
        let mut per_entry = self.per_entry_in_scope.clone();
        if let Some(here) = self.per_entry_block_locals.get(&span.lo) {
            per_entry.extend(here.iter().cloned());
        }
        if per_entry.is_empty() {
            return;
        }
        let per_entry = &per_entry;
        // POLARITY (review F4): this is a REJECT gate, so it uses the POSITIVE walker.
        // `expr_no_ref` answers "may reference" for anything it has not vetted, which is
        // right for an accept gate and turns every unvetted initializer into a rejection
        // here — `int idx = pkg::BASE;` beside any `automatic` sibling was rejected, with
        // a message naming a variable it never mentions.
        // Carry each hit's own DECL span so the diagnostic points at the declaration
        // rather than at nothing (review F9 — this gate runs at block level, before the
        // per-decl `cur_span` anchor in the hoist loop).
        let hits: Vec<(String, String, ast::Span)> = decls
            .iter()
            .filter(|d| d.lifetime != Some(true))
            .flat_map(|d| d.names.iter().map(move |n| (n, d.span)))
            .filter(|(n, _)| !per_entry.contains(&n.name.name))
            .filter_map(|(n, sp)| {
                let init = n.init.as_ref()?;
                let hit = per_entry.iter().find(|pn| expr_definitely_refs(init, pn))?;
                Some((n.name.name.clone(), hit.clone(), sp))
            })
            .collect();
        for (stat, auto, sp) in hits {
            let saved = self.cur_span.replace(sp);
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "the STATIC block-local `{stat}`'s initializer reads the `automatic` \
                     block-local `{auto}` in scope here; a static initializer \
                     runs once at time 0 while an `automatic` is initialized on each block \
                     entry, so `{auto}` has no value yet there (and a write back into it \
                     through an output/inout formal is overwritten by the entry \
                     initialization) — declare `{stat}` `automatic`, or move the \
                     initialization into the block body"
                ),
            );
            self.cur_span = saved;
        }
    }

    /// §6.8/§6.21: collect a block-local declaration's non-constant initializers into
    /// the t0 var-init sweep. `scope` is the `$blk$<lo>` segment when the declaration
    /// was given its own scope, `None` for the flattened path.
    ///
    /// §4.5.251: the scoped path used to skip this entirely, which is why a scoped
    /// `byte m[] = '{…}` came back EMPTY and why the same-name widening had to exclude
    /// every initializer-bearing declaration. The push target is the only difference —
    /// a scoped init is recorded under its own prefix and replayed there.
    fn collect_block_local_decl_inits(
        &mut self,
        d: &ast::NetVarDecl,
        span: ast::Span,
        scope: Option<&str>,
    ) {
        // §6.8/§6.21: a NON-constant PROCESS block-local initializer
        // (`begin logic x = g+1; …`) is STATIC-lifetime — applied ONCE
        // at time 0 (the block-local net is module-flattened), NOT on
        // each block entry. So it rides the SAME synthesized var-init
        // `initial` as a module-scope non-const var-init (matches
        // iverilog for an `always`/`for` body, which freezes the t0
        // value). A constant init already folded into net.init (skip).
        // A scalar `string s = expr;` block-local has no foldable
        // net.init field, so it always rides this t0 pre-sweep (a
        // dimensioned string was loud-rejected in `elaborate_netvar_decl`).
        let scalar_string =
            matches!(d.kind, ast::NetVarKind::String) && d.range.is_none() && d.packed.is_empty();
        if netvar_kind_is_var(d.kind) || scalar_string {
            for name in &d.names {
                let Some(init) = &name.init else { continue };
                let push = if scalar_string {
                    // A scalar string (no dims) rides the t0 pre-sweep. A
                    // string DYNAMIC array (`string s[] = '{…}`) rides it too:
                    // its `'{…}` is expanded by the flush (`new[N]` + element
                    // writes) exactly like the module-scope path
                    // (`collect_var_init_drivers`'s `is_dyn_str_init`). Without
                    // this the dimensioned-string branch dropped the init here
                    // (`name.unpacked` is non-empty → `push` false), leaving the
                    // block-local array silently EMPTY while the identical
                    // module-scope decl worked (a pre-existing silent-wrong).
                    // Other string dims (fixed / multi / non-`'{…}`) were
                    // loud-rejected at the decl, so they never reach here.
                    name.unpacked.is_empty()
                        || crate::string_array_route::is_dyn_string_container_init(
                            &name.unpacked,
                            init,
                        )
                } else {
                    // Mirror `collect_var_init_drivers`: a non-constant
                    // initializer rides the t0 pre-sweep. This INCLUDES an
                    // unpacked-array pattern (`int a[4] = '{1,2,3,4}`),
                    // whose synthesized `a = '{…}` is routed through
                    // `array_assign_special` — previously a bare
                    // `name.unpacked.is_empty()` guard silently dropped it.
                    let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                    fold_init(init, w).is_none() && self.const_eval_in_scope(init).is_none()
                };
                // r18 (family D): a per-entry local's initializer is emitted at
                // BLOCK ENTRY (the Logic-phase Block arm), so it must NOT also
                // ride the t0 static pre-sweep — that would double-init (and a
                // loop-var-reading init reads X at t0). Skip the push here.
                let per_entry = self
                    .per_entry_block_locals
                    .get(&span.lo)
                    .is_some_and(|s| s.contains(&name.name.name));
                if push && !per_entry {
                    let path = ast::HierPath {
                        segments: vec![name.name.clone()],
                        span: name.name.span,
                    };
                    // A block-local STRING init goes to the deferred
                    // list so it is assigned AFTER module-scope string
                    // inits (it may read one); a non-string keeps its
                    // existing `pending_var_inits` slot (byte-identical).
                    if scalar_string {
                        let key = self.scoped_init_key(scope);
                        self.pending_scoped_bl_strings
                            .entry(key)
                            .or_default()
                            .push((ast::Lvalue::Ident(path), init.clone()));
                    } else if let Some(seg) = scope {
                        let key = self.scoped_init_key(Some(seg));
                        self.pending_blk_inits
                            .entry(key)
                            .or_default()
                            .push((ast::Lvalue::Ident(path), init.clone()));
                    } else {
                        self.pending_var_inits
                            .push((ast::Lvalue::Ident(path), init.clone()));
                    }
                } else if scalar_string
                    && !name.unpacked.is_empty()
                    && self.has_fixed_string_array_storage(&name.name.name)
                {
                    // r19: a block-local FIXED string array (`string s[2] =
                    // '{…}`) — `push` is false for it (the gate above admits
                    // only a scalar or a `string s[]`), so expand it to one
                    // `s[k] = <elem>` per declared index here, into the same
                    // deferred string list so it lands after the module-scope
                    // string inits it may read.
                    //
                    // Gated on the decl having created the element storage,
                    // exactly like the module-scope collector — that is what
                    // keeps the two scopes from drifting (the class of bug
                    // that silently emptied a block-local `string s[] = '{…}`
                    // once before). Deliberately NOT gated on `per_entry`:
                    // that set only ever holds scalars and dyn/queue locals,
                    // so excluding it here would be dead code that silently
                    // opts INTO dropping the init if that set ever widens.
                    //
                    // T1-4: the FULL `unpacked`, exactly as the module-scope
                    // collector passes it — the two scopes share ONE expansion
                    // and must hand it the same shape, or a nested pattern
                    // expands here and not there.
                    if let Some(pairs) =
                        self.string_array_init_pairs(&name.name, &name.unpacked, init)
                    {
                        let key = self.scoped_init_key(scope);
                        self.pending_scoped_bl_strings
                            .entry(key)
                            .or_default()
                            .extend(pairs);
                    }
                }
            }
        }
    }

    /// One block-local declaration's Nets-phase hoist (extracted so the caller can
    /// wrap it in a `cur_span` anchor).
    #[allow(clippy::too_many_arguments)]
    fn hoist_one_block_local(
        &mut self,
        d: &ast::NetVarDecl,
        decls: &[ast::NetVarDecl],
        stmts: &[ast::Stmt],
        span: ast::Span,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) {
        // DUP (round-5): a colliding `automatic` block-local that the
        // pure pre-scan marked (disjoint blocks, no module-net collision,
        // no nesting) gets its OWN `$blk$<span>` scope so two blocks'
        // same-named locals become DISTINCT nets instead of aliasing
        // (was E3009). It still must pass per-entry definite-assignment
        // (each scoped block independently) to be byte-identical to the
        // static flatten — same gate as the non-colliding automatic path
        // below. `Fork` spans are never marked (gather skips them), so a
        // fork local always falls through. Everything unmarked keeps the
        // pre-existing behavior.
        //
        // §4.5.249: a STATIC dynamic-storage local qualifies too. Two
        // same-named dynamic locals in disjoint blocks are two distinct
        // variables that cannot share one flattened handle, so the pair was
        // loud whenever either side was static — scoping it is a pure
        // loud → support move. The per-entry-lifetime gate inside is keyed on
        // `d.lifetime`, so a static decl skips it exactly as it does on the
        // unscoped path; only the NET it gets changes.
        // Must match `gather_auto_block_locals` exactly — a decl scoped here but not
        // gathered there (or the reverse) breaks the invariant that EVERY colliding
        // occurrence of a name is scoped.
        //
        // §4.5.251: a scalar `string` qualifies too, and an initializer no longer
        // disqualifies either — the scoped path records its initializers under its own
        // prefix now and replays them there.
        let dyn_storage = matches!(d.kind, ast::NetVarKind::String)
            || d.names.iter().any(|n| {
                n.unpacked.iter().any(|dim| {
                    matches!(dim, ast::Dim::Dyn | ast::Dim::Queue(_) | ast::Dim::Assoc(_))
                })
            });
        if d.lifetime == Some(true) || dyn_storage {
            if let Some(seg) = self.block_local_scope_seg(span, d) {
                for n in &d.names {
                    let nm = &n.name.name;
                    let read_in_sibling_init = decls
                        .iter()
                        .flat_map(|dd| dd.names.iter())
                        .any(|nn| nn.init.as_ref().is_some_and(|e| !expr_no_ref(e, nm)));
                    // r18 (family D): a per-entry-safe local (its init re-runs at
                    // block entry) — supported even on a `$blk$`-scoped net.
                    let per_entry = self
                        .per_entry_block_locals
                        .get(&span.lo)
                        .is_some_and(|s| s.contains(nm));
                    // BL1 (round-19): a const-folding, never-reassigned local is
                    // byte-identical to the static flatten (the constant rides
                    // `net.init`; a never-written net holds it forever) — skip the
                    // loud (see the non-scoped gate below for the full rationale).
                    let const_immune = n.init.as_ref().is_some_and(|init| {
                        let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                        fold_init(init, w).is_some() || self.const_eval_in_scope(init).is_some()
                    }) && stmt_never_assigns_ident(stmts, nm);
                    // The gate is about the `automatic` PER-ENTRY lifetime, so
                    // it applies only to an `automatic` decl — the same
                    // condition the unscoped path below wraps it in. A STATIC
                    // decl reaches this arm only since §4.5.249 (a static
                    // dynamic-storage local now earns a `$blk$` net too), and
                    // for it there is no per-entry reset to be unfaithful to.
                    if d.lifetime == Some(true)
                        && !per_entry
                        && !const_immune
                        && (n.init.is_some()
                            || read_in_sibling_init
                            || !self.block_local_definitely_assigned(stmts, nm))
                    {
                        self.error(
                                        MsgCode::ElabUnsupported,
                                        &format!(
                                            "an `automatic` block-local `{nm}` whose per-entry \
                                             lifetime differs from static (an initializer, or a \
                                             read before its first write) is unsupported in a \
                                             procedural block (v1 flattens block-locals to one \
                                             static net); assign it before use, or drop `automatic`",
                                        ),
                                    );
                    }
                }
                // r19: this `$blk$`-scoped arm `continue`s past the decl-init collector
                // below, and it registers the decl under a `$blk$<lo>.` prefix that the
                // collector does not compute — so an init-bearing FIXED string array must
                // never reach here, or its init would be silently dropped. Today the
                // `automatic`-lifetime loud above guarantees that (an `AssignPattern` folds
                // neither via `fold_init` nor `const_eval_in_scope`, and `per_entry` never
                // holds a string array). Pinned by `automatic_block_local_init_stays_loud`.
                self.with_scope(&seg, |s| s.elaborate_netvar_decl(d, ports, body, true));
                self.collect_block_local_decl_inits(d, span, Some(&seg));
                return;
            }
        }
        // v1 flattens block-locals into the module namespace (no
        // per-block scope). If a local name was already created by an
        // EARLIER block, skip re-creating it rather than erroring
        // "redeclared" — two SEQUENTIAL named blocks reusing the same
        // temp name (`integer local_v;`) then share one net, which is
        // correct since they never overlap in time.
        // r19: a module-scope FIXED string array (`string sa[2]`) occupies its
        // bare NAME while registering NO net under it — only the per-element
        // `<name>$sae$<i>` nets — so the `existing` probe below cannot see it
        // and a block-local `string` of that name silently flattened onto it.
        // Every module-scope `sa[i]` then resolved to the block-local scalar
        // instead: `sa[0]="zz"` became a `putc` byte-write and read back "".
        //
        // Gated on the STRING kind, deliberately — the same discipline as the
        // `new_str_read` gate below, and for the same reason. A NON-string
        // local of that name mostly coalesces harmlessly (it occupies `t.sa`
        // while the array's storage stays `t.sa$sae$i`), so rejecting on the
        // bare name collision alone turned a dozen byte-correct designs loud —
        // `logic [7:0] sa;`, `logic [7:0] sa[2];`, `int`/`real sa;`, multi-name
        // decls — every one of which iverilog runs and vita already got right.
        // The residual non-string shapes that ARE wrong (a scalar local whose
        // name the block index-selects; some named-block array cases) are
        // pre-existing and recorded in ROADMAP §2: separating them from the
        // correct ones needs a per-shape hazard model, not a name match.
        //
        // T1: keyed on `has_fixed_string_array_storage`, which covers BOTH
        // representations. Keying it on `string_array_elems` alone stopped
        // firing once zero-based arrays routed to the dyn form, and the
        // alias came straight back in a NEW shape — the module's own
        // `sa[0]="zz"` and its read-back resolved through DIFFERENT
        // resolvers (the write reached the block-local scalar, the read the
        // routed array), so `R=zz,yy` became a silent `R=,` at exit 0.
        let shadowed_string_array = if matches!(d.kind, ast::NetVarKind::String) {
            d.names
                .iter()
                .find(|n| self.has_fixed_string_array_storage(&n.name.name))
                .map(|n| n.name.name.clone())
        } else {
            None
        };
        if let Some(nm) = shadowed_string_array {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a block-local `{nm}` collides with a string ARRAY of the same \
                                 name declared at module scope; v1 flattens block-locals into \
                                 the module namespace and cannot give this one its own scope \
                                 (the two would alias) — rename it",
                ),
            );
        }
        let existing = d
            .names
            .first()
            .and_then(|n| self.symbols.get(&self.fq(&n.name.name)).copied());
        if let Some(net) = existing {
            // ⓑ-breadth fix: a SCALAR local safely coalesces (the net
            // is just overwritten in time), but a DYNAMIC-STORAGE local
            // (queue/dyn-array/assoc/string) is backed by a persistent
            // heap that is NOT reset on block entry — sharing one net
            // across two blocks leaks the first block's elements into
            // the second (a silent-wrong the array reductions/`size()`
            // then compute over). v1 has no per-block scope to give them
            // distinct heaps, so reject loudly rather than miscompute.
            // Also fire when the EXISTING net is a plain scalar but THIS
            // (later) block's decl is a `string` that the block READS
            // (`begin logic s; end  begin string s; …=s[0]; end`): the
            // string coalesces onto the packed net and reading it is
            // silent-wrong. A WRITE-ONLY string coalesces harmlessly (its
            // truncated write is discarded), so gate on a READ to avoid
            // over-rejecting the common param-gated / dead-store shape.
            let new_str_read = matches!(d.kind, ast::NetVarKind::String)
                && d.names.first().is_some_and(|n| {
                    let nm = &n.name.name;
                    stmts.iter().any(|st| stmt_reads_ident(st, nm))
                                    // A SIBLING decl in THIS block can read the string
                                    // in its own initializer (`begin string s; int x =
                                    // s[0]; end`) — that read lives in a decl, not in
                                    // `stmts`, so scan the block's decl inits too.
                                    || decls.iter().flat_map(|dd| dd.names.iter()).any(|nn| {
                                        nn.init
                                            .as_ref()
                                            .is_some_and(|e| rvalue_reads_ident(e, nm))
                                    })
                });
            if self.is_dyn_handle_net(net) || self.is_string_net(net) || new_str_read {
                // Name the identifier and say what makes THIS one different
                // from the same-named locals that are fine: the two decls
                // must both be `automatic` (and neither nested inside the
                // other) to earn distinct `$blk$` nets. Without the name, N
                // of these in one run are indistinguishable — which is
                // exactly why the round-20 report could not narrow its 81.
                let nm = d
                    .names
                    .first()
                    .map(|n| n.name.name.as_str())
                    .unwrap_or("<unnamed>");
                let has_init = d.names.iter().any(|n| n.init.is_some());
                let lifetime = match (d.lifetime == Some(true), has_init) {
                    (true, _) => "this one is `automatic`, so the OTHER is not",
                    (false, true) => {
                        "this one is static AND has an initializer, which the scoped                          path would drop"
                    }
                    (false, false) => "this one is static and the other is not eligible",
                };
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "the dynamic-storage local `{nm}` (queue / dynamic array / \
                         associative array / string) is declared under the same name in \
                         another block, and this pair cannot be given distinct storage: \
                         v1 does that for two `automatic` locals, or for two dynamic \
                         locals with NO initializer, and only when neither block encloses \
                         the other — {lifetime}. As written both would share one flattened \
                         handle and one heap; declare them `automatic`, drop the \
                         initializer, or rename one"
                    ),
                );
            }
            // GAP-D completeness (adversarial find): an `automatic`
            // block-local whose name COLLIDES with an existing net (a
            // module-scope net, or an earlier sibling block-local) is
            // ALIASED onto that net by the v1 flatten — this both defeats
            // the automatic's required distinct per-entry storage AND
            // BYPASSES the definite-assignment gate below (this `continue`
            // skips it, so a read-before-write colliding automatic would
            // be silently accepted with the shared/persisted value).
            // v1 has no per-block scope to give the shadowing automatic
            // its own storage, so reject LOUD rather than silently alias
            // (correct-or-loud) — the workaround is a distinct name.
            if d.lifetime == Some(true) {
                for n in &d.names {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "an `automatic` block-local `{}` collides with an \
                                         existing net of the same name; v1 flattens \
                                         block-locals into the module namespace and cannot \
                                         give the `automatic` its own per-entry storage (it \
                                         would alias the shadowed net) — rename it",
                            n.name.name
                        ),
                    );
                }
            } else {
                // A STATIC scalar block-local coalescing onto an EXISTING
                // same-named net (the v1 flatten "reuse the net in time" path)
                // matches iverilog's distinct-per-scope variable ONLY when
                // (a) its TYPE equals the shared net's AND (b) it is definitely
                // assigned before any read here. A type mismatch (different
                // width or signedness) makes THIS block read/write the shared
                // net with the WRONG type — wrong `%d` sign, wrong `>>>`
                // arithmetic, wrong `%h`/`$bits` width. A read-before-write
                // observes the PRIOR block's leftover value instead of the X a
                // fresh variable would hold. Both are silent-wrong; v1 has no
                // per-block scope to give this local distinct typed storage, so
                // loud (correct-or-loud). The SAFE same-type + definitely-
                // assigned coalesce (common `for`/`tmp` name reuse) is
                // unaffected. dyn/string collisions were already loud above;
                // `range_to_dims` is packed-only, so skip them here.
                let (nw, _, _, nsig) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                for n in &d.names {
                    let nm = &n.name.name;
                    // A name ALSO declared at MODULE scope is a legitimate
                    // SHADOW (`outer_s x; … begin inner_s x; … end`), handled
                    // by the struct/enum/typedef shadow-scoping — NOT a
                    // sibling block-local coalesce. Skip it: this guard targets
                    // block-vs-block (two disjoint blocks reusing one flattened
                    // net), where the colliding name is a pure block-local.
                    if self.local_decl_names.contains(nm) {
                        return;
                    }
                    let Some(ex) = self.symbols.get(&self.fq(nm)).copied() else {
                        return;
                    };
                    if self.is_dyn_handle_net(ex)
                        || self.is_string_net(ex)
                        || matches!(d.kind, ast::NetVarKind::String)
                    {
                        return;
                    }
                    let (ew, esig) = {
                        let e = &self.nets[ex as usize];
                        (e.width, e.signed)
                    };
                    if nw != ew || nsig != esig {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "block-local `{nm}` is declared with a different \
                                             width/signedness than a same-named block-local in \
                                             another block; v1 flattens both to one module net \
                                             and cannot hold two types (the shared net would be \
                                             read/written with the wrong sign or width) — rename \
                                             one"
                            ),
                        );
                    } else if !self.block_local_definitely_assigned(stmts, nm) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "block-local `{nm}` shares one flattened net with a \
                                             same-named block-local in another block but is READ \
                                             before it is assigned here — it would observe the \
                                             other block's leftover value, not a fresh variable's \
                                             default; assign it before use, or rename one"
                            ),
                        );
                    }
                }
            }
            return;
        }
        // GAP-D soundness: a procedural block-local `automatic` is
        // byte-identical to the static flattening ONLY when its
        // per-entry reset/re-init is dead. An initializer (must re-run
        // each entry) or a read-before-write use (observes the reset
        // value) cannot be honored without a per-block frame, so reject
        // LOUD rather than silently give static semantics. The
        // static-equivalent case (`automatic t x; x = e; … x …`) is
        // accepted — it flattens correctly.
        if d.lifetime == Some(true) {
            for n in &d.names {
                let nm = &n.name.name;
                // A sibling block-local's initializer that reads this
                // automatic var (`automatic int v; int w = v;`) observes
                // its entry value too — not just the block statements.
                // Use the CONSERVATIVE `expr_no_ref` (not the shared
                // under-detecting `expr_reads_ident`, which `_ => false`s
                // on `ArrayMethodWith`/`Dist`/… and could miss a hidden
                // read) so this gate is sound like the statement scan.
                let read_in_sibling_init = decls
                    .iter()
                    .flat_map(|dd| dd.names.iter())
                    .any(|nn| nn.init.as_ref().is_some_and(|e| !expr_no_ref(e, nm)));
                // r18 (family D): a per-entry-safe automatic-with-init local
                // (not under a fork, no collision — see `compute_per_entry_
                // block_locals`) is now supported: its initializer re-runs at
                // BLOCK ENTRY (emitted by the Logic-phase Block arm), so skip
                // the loud reject here.
                let per_entry = self
                    .per_entry_block_locals
                    .get(&span.lo)
                    .is_some_and(|s| s.contains(nm));
                // BL1 (round-19): an `automatic` block-local whose initializer
                // FOLDS TO A CONSTANT and which is NEVER reassigned in the block is
                // byte-identical to the static flatten — the folded constant already
                // rides `net.init` (a never-written net holds it forever), so it is
                // CONCURRENCY-IMMUNE even under a `fork` (module-process forks have no
                // frame arena, but every activation reads the SAME constant off one
                // shared net). Skip the loud for it; do NOT mark per-entry — a
                // constant needs no re-init, and the const `net.init` handles t0.
                let const_immune = n.init.as_ref().is_some_and(|init| {
                    let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                    fold_init(init, w).is_some() || self.const_eval_in_scope(init).is_some()
                }) && stmt_never_assigns_ident(stmts, nm);
                if !per_entry
                    && !const_immune
                    && (n.init.is_some()
                        || read_in_sibling_init
                        || !self.block_local_definitely_assigned(stmts, nm))
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "an `automatic` block-local `{nm}` whose per-entry \
                                         lifetime differs from static (an initializer, or a \
                                         read before its first write) is unsupported in a \
                                         procedural block (v1 flattens block-locals to one \
                                         static net); assign it before use, or drop `automatic`",
                        ),
                    );
                }
            }
        }
        self.elaborate_netvar_decl(d, ports, body, true);
        // Record the FLATTENED key. This net belongs to ONE process, but
        // the v1 flatten publishes it under the enclosing prefix's bare
        // name — inside a generate block that is `t.g.W`, a DIFFERENT key
        // from the module constant `t.W`. Name resolution's inner-net-wins
        // rule must not treat it as a legitimate shadow of an outer
        // constant, or every OTHER reader in that generate scope (a
        // sibling `initial`, a continuous assign, an inner generate) picks
        // up one process's private variable instead of the constant.
        for n in &d.names {
            let k = self.fq(&n.name.name);
            self.hoisted_block_local.insert(k);
        }
        self.collect_block_local_decl_inits(d, span, None);
    }

    /// Recursively create nets for every `begin…end`/`fork…join` block-local
    /// declaration reachable from a procedural-block body. v1 flattens these to
    /// module-scope nets (no per-process frame). Called in the Nets phase.
    pub(crate) fn hoist_block_local_nets(
        &mut self,
        s: &ast::Stmt,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) {
        match s {
            ast::Stmt::Block {
                decls, stmts, span, ..
            }
            | ast::Stmt::Fork {
                decls, stmts, span, ..
            } => {
                self.deny_static_init_reading_per_entry(decls, *span);
                for d in decls {
                    // §4.5.249: the Nets-phase hoist never passes through `lower_stmt`,
                    // so anchor each declaration's diagnostics here. This is exactly the
                    // family the report could not narrow — 81 identical messages with no
                    // position and, for the same-name class, no identifier either.
                    let saved_span = self.cur_span.replace(d.span);
                    self.hoist_one_block_local(d, decls, stmts, *span, ports, body);
                    self.cur_span = saved_span;
                }
                // This block's per-entry locals are in scope for every NESTED block's
                // static initializers too (`begin automatic int c = 5; begin int z =
                // f(c); … end end`), so publish them for the recursion and take them
                // back on the way out.
                let pushed: Vec<String> = self
                    .per_entry_block_locals
                    .get(&span.lo)
                    .map(|s| {
                        s.iter()
                            .filter(|n| self.per_entry_in_scope.insert((*n).clone()))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                for st in stmts {
                    self.hoist_block_local_nets(st, ports, body);
                }
                for n in pushed {
                    self.per_entry_in_scope.remove(&n);
                }
            }
            ast::Stmt::If { then_s, else_s, .. } => {
                self.hoist_block_local_nets(then_s, ports, body);
                if let Some(e) = else_s {
                    self.hoist_block_local_nets(e, ports, body);
                }
            }
            ast::Stmt::Case { items, .. } => {
                for it in items {
                    let inner = match it {
                        ast::CaseItem::Match { body: b, .. } => b,
                        ast::CaseItem::Default { body: b, .. } => b,
                    };
                    self.hoist_block_local_nets(inner, ports, body);
                }
            }
            ast::Stmt::For { body: b, .. }
            | ast::Stmt::While { body: b, .. }
            | ast::Stmt::Repeat { body: b, .. }
            | ast::Stmt::Forever { body: b, .. } => {
                self.hoist_block_local_nets(b, ports, body);
            }
            _ => {}
        }
    }

    /// Allocate this frame function's nets (formals, return-var, body_decls — in
    /// that slot order) under a synthetic `$func$<name>` scope, push a placeholder
    /// `FuncDef` + the complete `FuncMeta`, and record the name→FuncId divert.
    /// Reserve a frame body's block-local declarations (`begin int tmp; …`) under
    /// the current `$func$<name>` scope, in source order, AFTER the formals /
    /// return / top-level `body_decls`. A name already reserved (a formal, the
    /// return var, a top-level local, or an earlier same-named block) is COALESCED
    /// (shared net) — like the process-body hoist for two sequential blocks reusing
    /// IEEE §6.21: a local declared in a nested `begin…end` is visible only within
    /// that block. vita keeps a function/task/procedural body's block-locals in a
    /// FLAT per-body table, so a reference OUTSIDE the declaring block would
    /// silently resolve to the block-local instead of the lexically-correct outer
    /// binding (a module variable, a formal, or an enclosing local). Proper
    /// per-block scope lowering is a large follow-on (ROADMAP §4.5.18); until then,
    /// detect that exact leak and reject it LOUD (correct-or-loud) rather than
    /// produce a silent-wrong. A block-local referenced only inside its own block
    /// resolves correctly and is left untouched (no diagnostic, byte-identical).
    pub(crate) fn check_block_local_scope_leaks(&mut self, body: &ast::Stmt) {
        let mut nested = Vec::new();
        gather_nested_block_locals(body, true, &mut nested);
        for (bspan, names) in &nested {
            for n in names {
                if stmt_refs_ident_outside(body, *bspan, n) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "block-local `{n}` is referenced outside its `begin…end` block; \
                             vita cannot resolve this to the outer `{n}` yet (it would silently \
                             read the block-local) — rename the block-local or hoist its \
                             declaration to the enclosing scope"
                        ),
                    );
                }
            }
        }
    }
}
