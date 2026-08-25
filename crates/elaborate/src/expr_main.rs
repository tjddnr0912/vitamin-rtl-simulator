//! lower_expr — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    // ── expression lowering: post-order arena append, returns ExprId ──
    pub(crate) fn lower_expr(&mut self, e: &ast::Expr) -> u32 {
        // §11.6: an expression whose operands carry an unsized FILL literal is
        // lowered width-aware (self-determined context = 0 here, so a bare fill in
        // a self-determined position stays minimal). Only context-propagating node
        // types are scanned, so a fill-free design pays an O(1) type check.
        if is_ctx_node(e) && expr_contains_fill(e) {
            return self.lower_expr_ctx(e, 0);
        }
        self.lower_expr_ungated(e)
    }

    /// `lower_expr` with the fill/context front gate ALREADY DECIDED — the entry
    /// `lower_expr_ctx` uses to hand a whole node back for ordinary lowering.
    ///
    /// ⚠️ It exists because the two were MUTUAL TAIL CALLS. `lower_expr_ctx`'s
    /// `Concat` arm — since deleted as redundant, see the note where it used to be —
    /// handed a string concat back to `lower_expr`, whose front gate
    /// (`is_ctx_node` ∧ `expr_contains_fill` — both still true of that same node)
    /// sent it straight back, so `wire [11:0] a = {s, '1};` never terminated. In
    /// release that is 100% CPU at a FLAT RSS — both calls sit in tail position, so
    /// the stack never grows to tell you what is happening; in debug it overflows
    /// the stack and aborts (SIGABRT, rc=134).
    ///
    /// ⭐ With this entry the termination argument is STRUCTURAL instead of a
    /// by-hand audit of which arms happen to exist: every call out of
    /// `lower_expr_ctx` is either on a STRICT SUB-EXPRESSION or to this entry, and
    /// this entry never re-enters the gate with the node it was handed. Adding a
    /// kind to `is_ctx_node`, or deleting an arm from `lower_expr_ctx`, can no
    /// longer resurrect the cycle.
    pub(crate) fn lower_expr_ungated(&mut self, e: &ast::Expr) -> u32 {
        match &e.kind {
            // ── leaves ──────────────────────────────────────────────
            ast::ExprKind::IntLit { kind, raw } => {
                // §5.7.1/§11.6: a fill literal reaching here is in a SELF-determined
                // position (a bare `$display` arg, a `$signed`/`$unsigned` arg, a
                // select index, …) → 1 bit, minimal. Context-determined positions
                // widen it through `lower_expr_ctx` before reaching this arm.
                if literal::is_fill_literal(raw, *kind) {
                    let cv = literal::fill_literal_const(raw, *kind, 1)
                        .unwrap_or_else(|| make_const_u32(0, 1));
                    let cid = self.intern_const(cv);
                    return self.push_expr(ir::Expr::Const { val: cid });
                }
                let cid = self.lower_int_literal(*kind, raw);
                self.push_expr(ir::Expr::Const { val: cid })
            }
            // G11: a time literal folds to a Const in the current module's time unit
            // (reuses the `const_eval_in_scope` fold). Loud on sub-precision / real /
            // non-constant (correct-or-loud).
            ast::ExprKind::TimeLit { .. } => match self.const_eval_in_scope(e) {
                Some(d) => {
                    let cid = self.intern_const(make_const_i64(d, 64, false));
                    self.push_expr(ir::Expr::Const { val: cid })
                }
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a time literal that is non-constant, real, negative, or finer \
                         than the module's time unit/precision is unsupported",
                    );
                    self.placeholder_expr()
                }
            },
            // G10: a `NamedArg` is consumed by the call reorder (`resolve_named_args`);
            // reaching `lower_expr` means it was used outside a user-subroutine call
            // (e.g. a named arg to a system task) — loud (correct-or-loud).
            ast::ExprKind::NamedArg { .. } => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a named argument `.formal(...)` is only valid in a user function / \
                     task call",
                );
                self.placeholder_expr()
            }
            // G8: a method chained on a call/method RESULT (`s.substr(a,b).atoi()`). Lower
            // the receiver, then dispatch the method on its handle — a string method
            // chains as a nested SysFunc (a `.substr()`/`.toupper()`/`.tolower()` result
            // IS a string handle, per `ir_expr_is_string`). A non-string receiver (e.g.
            // chaining on `.atoi()`, which returns an int) is loud (correct-or-loud).
            ast::ExprKind::MethodCall { recv, method, args } => {
                let h = self.lower_expr(recv);
                if self.ir_expr_is_string(h) {
                    self.lower_string_method_expr_handle(h, &method.name, args)
                } else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a chained method call is supported only on a string-returning \
                         method result (e.g. `s.substr(a,b).atoi()`)",
                    );
                    self.placeholder_expr()
                }
            }
            // v7 P2-D: explicit `pkg::name` — folds through the package
            // const map (sees the PACKAGE value even when a local declaration
            // shadows an import, iverilog-pinned). Function references need
            // call syntax, which is outside the v7 scope — loud.
            ast::ExprKind::PkgScoped { pkg, name } => {
                // §3 ⑨: a string / real package parameter has no i64 value, so it is
                // kept out of `pkg_consts` and answered here — BEFORE the numeric map,
                // for the same reason the bare-`Ident` arm below resolves its string and
                // real side maps first. A `real` param with a wholly integral
                // initializer is in BOTH maps; the real answer wins here because that is
                // the parameter's DECLARED domain, and folding it as an integer is what
                // made `P::PR / 2` answer 1 where both oracles say 1.5.
                if let Some(rv) = self
                    .pkg_real_val
                    .get(&pkg.name)
                    .and_then(|m| m.get(&name.name))
                    .copied()
                {
                    let cid = self.intern_const(make_const_real(rv));
                    return self.push_expr(ir::Expr::Const { val: cid });
                }
                if let Some(raw) = self
                    .pkg_str_raw
                    .get(&pkg.name)
                    .and_then(|m| m.get(&name.name))
                    .cloned()
                {
                    let cid = self.intern_const(parse_str_literal(&raw));
                    return self.push_expr(ir::Expr::Const { val: cid });
                }
                match self
                    .pkg_consts
                    .get(&pkg.name)
                    .and_then(|c| c.get(&name.name))
                {
                    // Materialize at the package const's DECLARED width (`logic
                    // [3:0] x` → 4-bit const), not the value-inferred 32 bits, so
                    // it carries the right self-width inside a concat/replication.
                    Some(&v) => {
                        let meta = self
                            .pkg_const_meta
                            .get(&pkg.name)
                            .and_then(|m| m.get(&name.name))
                            .copied();
                        self.const_param_expr_w(v, meta)
                    }
                    None => {
                        // A2b-prereq: `pkg::var` reads the package-level
                        // VARIABLE net (sees the package storage even when a
                        // local declaration shadows an import — same rule as
                        // the const branch above, iverilog-pinned).
                        if let Some(&net) =
                            self.pkg_vars.get(&pkg.name).and_then(|m| m.get(&name.name))
                        {
                            if self.net_is_static_array(net) {
                                // mirror the Ident arm's whole-array guard: a
                                // whole unpacked array has no value here.
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    "a whole unpacked array has no value in this \
                                     context (v1: arrays are copied by array \
                                     assignment; index an element elsewhere — \
                                     import the name and select on it)",
                                );
                            }
                            return self.push_expr(ir::Expr::Signal { net, word: None });
                        }
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "`{}::{}` does not name a package constant or \
                                 variable (v7 supports param/enum-label and \
                                 plain-variable references)",
                                pkg.name, name.name
                            ),
                        );
                        self.placeholder_expr()
                    }
                }
            }
            ast::ExprKind::Ident(path) => {
                // ⓑ-breadth (v17): inside an array-method `with` clause, the
                // iterator name (`item`/named) is the per-element value; `iter.index`
                // is its 0-based position. Checked before all other name resolution.
                if self.array_iter.is_some() {
                    let iter = self.array_iter.clone().unwrap();
                    match path.segments.as_slice() {
                        [s] if s.name == iter => {
                            let (width, signed) = self.array_iter_elem.unwrap_or((32, true));
                            return self.push_expr(ir::Expr::ArrayItem {
                                index: false,
                                width,
                                signed,
                            });
                        }
                        [s, ix] if s.name == iter && ix.name == "index" => {
                            return self.push_expr(ir::Expr::ArrayItem {
                                index: true,
                                width: 32,
                                signed: true,
                            });
                        }
                        _ => {}
                    }
                }
                // INLINE substitution (function/task formals). A single-segment name
                // bound to an actual-arg ExprId lowers to that ExprId directly — no
                // new IR node, exactly like `Paren` unwrapping. Innermost wins.
                if path.segments.len() == 1 {
                    let seg = &path.segments[0].name;
                    if let Some(eid) = self.subst_lookup(seg) {
                        return eid;
                    }
                    // output/inout task formal: resolves to the caller's net.
                    if let Some(net) = self.out_subst_lookup(seg) {
                        if self.net_is_static_array(net) {
                            // Phase-1.x ②: an out-actual bound to a whole
                            // array would otherwise read word 0 SILENTLY
                            // through the formal (adversarial find #2).
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a task output formal bound to a whole unpacked \
                                 array has no value (v1: arrays cannot pass \
                                 through task ports)",
                            );
                        }
                        return self.push_expr(ir::Expr::Signal { net, word: None });
                    }
                    // N5: a `string`-valued parameter/localparam folds to the SAME
                    // StrUtf8 const its raw literal would (so `S == "abc"` is byte-
                    // identical). It has no i64 value (kept out of `self.params`), so it
                    // is resolved before the numeric param path — but ONLY when the
                    // string param is the INNERMOST binding of the name (IEEE §6.21
                    // innermost-wins). An independent `walk_scopes(&str_param_raw)` would
                    // match an OUTER module-scope string param even when an inner net /
                    // numeric param / frame-local (`$func$f.S`) shadows it, resolving the
                    // name two different ways (const-eval finds the inner via
                    // `lookup_scoped`, this pass found the outer string) — a silent-wrong.
                    // Re-derive the innermost key over the COMBINED binding set and only
                    // fold the string when that exact key is the string param.
                    let mut local_shadows_param = false;
                    if let Some(key) = self.walk_scopes_key(seg, |k| {
                        self.str_param_raw.contains_key(k)
                            || self.wide_param_bits.contains_key(k)
                            || self.real_param_val.contains_key(k)
                            || self.params.contains_key(k)
                            || self.symbols.contains_key(k)
                    }) {
                        if self.str_param_raw.contains_key(&key) {
                            let raw = self.str_param_raw[&key].clone();
                            let cid = self.intern_const(parse_str_literal(&raw));
                            return self.push_expr(ir::Expr::Const { val: cid });
                        }
                        // A parameter wider than the i64 constant domain — same
                        // side-map shape as the string and real cases, and it rides
                        // the same innermost-wins key derivation.
                        if let Some(cv) = self.wide_param_bits.get(&key).cloned() {
                            let cid = self.intern_const(cv);
                            return self.push_expr(ir::Expr::Const { val: cid });
                        }
                        // r19: a REAL param folds to the SAME const its raw literal
                        // would, and rides the SAME innermost-wins re-derivation as the
                        // string case — an independent walk over `real_param_raw` alone
                        // would match an OUTER real param even when an inner net /
                        // numeric param / frame-local shadows it (the silent-wrong the
                        // string comment above describes).
                        if let Some(&v) = self.real_param_val.get(&key) {
                            let cid = self.intern_const(make_const_real(v));
                            return self.push_expr(ir::Expr::Const { val: cid });
                        }
                        // An inner NET wins over an outer parameter (§23.9 name
                        // resolution). The fall-through below calls `lookup_scoped`,
                        // which runs its OWN params-only walk and therefore ignores
                        // the innermost key just derived — so an outer `localparam W`
                        // beat a function/task/block-local `int W` and the local's
                        // value silently vanished (`W = 9; return W;` returned the
                        // param's 4). Skip the parameter branch when the innermost
                        // binding is a net that is NOT itself a parameter; resolution
                        // then falls to `resolve_net`, which is what the comment above
                        // always intended.
                        // ⚠️ The `!params` clause alone is not the question, because the
                        // v1 flatten publishes a module-level block-local under the BARE
                        // name — landing on the very key the constant already occupies,
                        // so `symbols` AND `params` both hold it and the net (which the
                        // VCD shows, and which the block's own `N = 3` writes to) could
                        // never win. A reader INSIDE the declaring block must resolve to
                        // that net even then; one outside must still see the constant.
                        if self.symbols.contains_key(&key)
                            && (self.block_local_declared_at(&key, e.span)
                                || (!self.params.contains_key(&key)
                                    && self.block_local_covers(&key, e.span)))
                        {
                            local_shadows_param = true;
                        }
                        // else: an inner numeric param wins — fall through to the
                        // normal resolution below (which resolves that innermost binding).
                    }
                    // parameter / localparam / genvar: a constant in THIS scope (or
                    // an enclosing generate scope) folds to a Const, NOT a net read.
                    // Resolved before `resolve_net` so a param never errors as an
                    // undeclared net (mirrors `const_eval_in_scope`'s lookup_scoped).
                    if let Some(v) = self.lookup_scoped(seg).filter(|_| !local_shadows_param) {
                        // A TYPED param (`logic [63:0] P`, `int W`) materializes at
                        // its DECLARED width, not the value-inferred 32 bits — so
                        // `$display("%h", P)` of a 64-bit param shows all 16 nibbles.
                        let meta = self.walk_scopes(seg, &self.param_meta);
                        return self.const_param_expr_w(v, meta);
                    }
                    // SVA-REST `let NAME = expr;` (0 formals): substitute the declared
                    // body. Resolved AFTER nets/params/formals (a real net/param of the
                    // same name always wins — a `let` never shadows hardware) and only
                    // when no net AND no function/task binds the name (the genuine
                    // callable wins — review: illegal co-declaration must not silent-shadow).
                    if self.let_table.contains_key(seg)
                        && self.lookup_net_scoped(seg).is_none()
                        && !self.func_table.contains_key(seg)
                        && !self.task_table.contains_key(seg)
                    {
                        return self.lower_let_use(seg, &[], e.span);
                    }
                }
                // N7: a class field read (`obj.field` / `this.field` / bare member
                // inside a method) → `Signal{handle, word: field-id}`. Checked
                // BEFORE the hierarchical-reference path (which would otherwise
                // treat `obj.field` as a cross-instance ref).
                if let Some(eid) = self.try_class_field_read(path) {
                    return eid;
                }
                // N3: a multi-segment path that is NOT already a known dotted symbol
                // (an interface member alias `i.sig`, inserted at port-binding BEFORE
                // this body lowers) is a hierarchical cross-instance READ — the child
                // instance's net may not exist yet (created in pass 8, after this pass-7
                // lowering), so emit a PLACEHOLDER `Signal` and DEFER resolution to
                // `resolve_deferred_hier` (run once all instances are elaborated). A
                // hierarchical WHOLE-net WRITE is the symmetric deferred lvalue path
                // (`collect_lval_chunks` → `defer_hier_write`); an element/part-select
                // write is still a loud follow-on.
                if path.segments.len() > 1 {
                    let joined = path
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    // A2b-prereq F2: a dotted hit on a package-var import alias
                    // is NOT a known dotted symbol — defer, so the hierarchical
                    // resolver (which skips aliases per §26.3) stays loud.
                    if self.lookup_net_scoped(&joined).is_none()
                        || self.dotted_hit_is_pkg_alias(&joined)
                    {
                        let eid = self.push_expr(ir::Expr::Signal {
                            net: POISON_NET,
                            word: None,
                        });
                        self.deferred_hier.push(DeferredHier {
                            span: self.cur_span,
                            eid,
                            prefix: self.cur_prefix.clone(),
                            path: path.segments.iter().map(|s| s.name.clone()).collect(),
                        });
                        return eid;
                    }
                    // else: a known dotted symbol (interface member) — fall through.
                }
                let net = self.resolve_net(path);
                if self.event_nets.contains(&net) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a named event has no value: it cannot be read in an \
                         expression (only `->e` and `@(e)` touch it)",
                    );
                }
                if self.is_dyn_handle_net(net) {
                    // v5 ⑥: whole-handle reads (incl. handle copy `d2 = d`)
                    // are outside the MVP — elements/methods only.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a dynamic-storage handle has no whole-value surface (read elements or call methods)",
                    );
                }
                if self.net_is_static_array(net) {
                    // Phase-1.x ②: a whole unpacked array is only a value in
                    // ARRAY ASSIGNMENT (intercepted before lowering reaches
                    // here). Anywhere else it used to read word 0 SILENTLY.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a whole unpacked array has no value in this context \
                         (v1: arrays are copied by array assignment; index an \
                         element elsewhere)",
                    );
                }
                // §13.3 UARR: an unpacked-array FORMAL is an md-packed frame slot; a
                // WHOLE read (`arr`, not `arr[i]`) reaches this choke point (an element
                // read `arr[i]` diverts through `expr_packed_chain` earlier) and would
                // return the flat vector — silently wrong for a scalar context. Only
                // element reads are supported, so loud-reject the whole use.
                if self.frame_arr_formal_meta.contains_key(&net) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a whole unpacked-array formal has no value here — index an \
                         element (`arr[i]`); passing / comparing / displaying the whole \
                         array formal is unsupported",
                    );
                }
                self.push_expr(ir::Expr::Signal { net, word: None })
            }

            // ── operators (1:1 name-map; children lowered first) ────
            ast::ExprKind::Unary { op, operand } => {
                let operand = self.lower_expr(operand);
                let irop = map_unop(*op);
                // §6.2: bitwise `~` / reductions on a real are illegal (`+`/`-`/`!`
                // are legal: unary +/- are real-preserving, `!` is logical).
                if self.expr_is_real(operand)
                    && matches!(
                        irop,
                        ir::UnOp::BitNot
                            | ir::UnOp::RedAnd
                            | ir::UnOp::RedNand
                            | ir::UnOp::RedOr
                            | ir::UnOp::RedNor
                            | ir::UnOp::RedXor
                            | ir::UnOp::RedXnor
                    )
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "bitwise/shift/reduction not defined on real operand",
                    );
                }
                self.push_expr(ir::Expr::Unary { op: irop, operand })
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                // §11.4.6 wildcard equality intercept — BEFORE every map_binop
                // route (the string-compare and generic arms below never see
                // WildEq/WildNe). Comparison operands size each other, not the
                // outer context, so both lower_expr paths land here.
                if matches!(op, ast::BinOp::WildEq | ast::BinOp::WildNe) {
                    return self.lower_wildcard_eq(lhs, rhs, matches!(op, ast::BinOp::WildNe));
                }
                // N7 handle type gate (IEEE §8.4/§11.4): a class handle / `null`
                // is only a legal binary operand of `==`/`!=` against ANOTHER
                // handle/null. Arithmetic/relational on a handle, or `==` with a
                // mismatched integral, is loud — not silent object-id math.
                {
                    let lk = self.ast_handle_kind(lhs);
                    let rk = self.ast_handle_kind(rhs);
                    let any_handle = matches!(lk, HKind::Handle | HKind::Null)
                        || matches!(rk, HKind::Handle | HKind::Null);
                    if any_handle {
                        let is_eq = matches!(op, ast::BinOp::Eq | ast::BinOp::Ne);
                        // both sides handle/null/unknown ⇒ a legal handle compare.
                        let both_ok = matches!(lk, HKind::Handle | HKind::Null | HKind::Unknown)
                            && matches!(rk, HKind::Handle | HKind::Null | HKind::Unknown);
                        if !is_eq || !both_ok {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a class handle / `null` is only a legal operand of \
                                 `==`/`!=` against another handle or `null` (IEEE §8.4)",
                            );
                            return self.placeholder_expr();
                        }
                    }
                }
                // v7 P2-C: a comparison with a STRING-domain operand routes
                // through StrCmp (packed compare zero-extends MSB-side, which
                // is NOT lexicographic for unequal lengths; sizing the
                // dynamic widths statically would truncate). `cmp <op> 0`
                // with a SIGNED zero keeps the relational signed.
                if matches!(
                    op,
                    ast::BinOp::Eq
                        | ast::BinOp::Ne
                        | ast::BinOp::Lt
                        | ast::BinOp::Le
                        | ast::BinOp::Gt
                        | ast::BinOp::Ge
                ) && (self.expr_is_string_ast(lhs) || self.expr_is_string_ast(rhs))
                {
                    let l = self.lower_expr(lhs);
                    let r = self.lower_expr(rhs);
                    let cmp = self.push_expr(ir::Expr::SysFunc {
                        which: ir::SysFuncId::StrCmp,
                        args: vec![l, r],
                    });
                    let zero = {
                        let cid = self.intern_const(make_const_i64(0, 32, true));
                        self.push_expr(ir::Expr::Const { val: cid })
                    };
                    return self.push_expr(ir::Expr::Binary {
                        op: map_binop(*op),
                        lhs: cmp,
                        rhs: zero,
                    });
                }
                let lhs = self.lower_expr(lhs); // POST-ORDER: lhs, then rhs, then self
                let rhs = self.lower_expr(rhs);
                let irop = map_binop(*op);
                if let Some(id) = self.binary_real_operand_route(irop, lhs, rhs) {
                    return id;
                }
                self.push_expr(ir::Expr::Binary { op: irop, lhs, rhs })
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                let cond = self.lower_expr(cond);
                let then_e = self.lower_expr(then_e);
                let else_e = self.lower_expr(else_e);
                self.push_expr(ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                })
            }

            // ── selects → Select{base,offset,width,kind} (all ExprIds) ──
            ast::ExprKind::BitSelect { base, index } => {
                // N3.4 follow-on: a RANGE select cannot be further indexed —
                // `x[3:2][0]` / `x[c+:w][0]`. iverilog rejects this universally
                // ("All but the final index in a chain of indices must be a single
                // value, not a range."), on plain vectors too. vita used to silently
                // bit-select bit `index` of the already-narrowed range result (a
                // silent-wrong value), so this is a loud E3009, not a fall-through.
                if matches!(
                    &base.kind,
                    ast::ExprKind::PartSelect { .. } | ast::ExprKind::IndexedPart { .. }
                ) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "all but the final index in a chain of indices must be a \
                         single value, not a range",
                    );
                    let _ = index;
                    return self.placeholder_expr();
                }
                // N6: a fixed string-ARRAY element read `files[K]` (CONST K) → the K-th
                // scalar string element net (checked before the dyn/string-byte chains —
                // a string array is a set of scalar string nets, not a handle). A runtime
                // index is loud (a dynamic string array is a deeper follow-on).
                if self.is_string_array_base(base) {
                    if let Some(net) = self.string_array_elem_net(base, index) {
                        return self.push_expr(ir::Expr::Signal { net, word: None });
                    }
                    if self.const_eval_in_scope(index).is_none() {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a string-array element requires a constant index (a runtime \
                             index / dynamic string array is a deeper follow-on)",
                        );
                    }
                    return self.placeholder_expr();
                }
                // v5 ⑥: dyn-handle element read (`d[i]`, `q[$]`, `a[k]`) —
                // BEFORE the static array/packed chains (handles have
                // `array_len 0`, so those would mis-route to bit-select).
                if let Some(eid) = self.dyn_select_read(base, index) {
                    return eid;
                }
                // String element READ `s[i]` (IEEE §6.16.2): a string variable is a
                // dynamic byte SEQUENCE, not a packed vector — `s[i]` is the front-
                // indexed CHARACTER (8-bit byte) at i, 0 if out of range, reusing the
                // `.getc(i)` primitive. A packed-vector base (`logic[7:0] x; x[0]`)
                // returns None here and falls through to the plain bit-select.
                if let Some(eid) = self.string_index_read(base, index) {
                    return eid;
                }
                // SYMMETRY with the LHS (`collect_lval_chunks`): a `base[i]…[k]`
                // chain rooted at an ARRAY net is a WORD select (the first D indices
                // flatten row-major to the element word `i0*s0+…+iD`), with any
                // trailing indices becoming bit-selects INTO that word. The single-
                // dim `mem[i]` and `mem[i][j]` cases are the D==1 specialisation —
                // lowered byte-identically to the old path. A scalar base falls
                // through to the plain bit-select below.
                if let Some((net, idxs)) = self.expr_array_chain(base, index) {
                    return self.lower_array_read(net, &idxs);
                }
                if let Some((net, idxs)) = self.expr_packed_chain(base, index) {
                    return self.lower_packed_read(net, &idxs);
                }
                // N3.1 (+ multi-dim follow-on): a hierarchical INDEXED read
                // `dut.mem[i]` / `dut.x[i]` / `dut.grid[i][j]` / packed `dut.pm[i]` —
                // the base chain bottoms out at a 2-segment hierarchical ref whose net
                // does not exist yet (created in pass 8). The SELECT KIND and arity
                // (single-/multi-dim array element word vs packed bit-slice vs vector
                // bit) depend on the resolved net's shape, so defer the whole indexed
                // read and resolve it in `resolve_deferred_hier_sel`. (A known dotted
                // symbol — an interface member — keeps the normal path below.)
                if let Some((path, idx_asts)) = self.hier_sel_chain(base, index) {
                    // Lower EVERY index NOW, with the full lowering context
                    // (params/genvars/function-formal `subst`) — re-lowering at fixup
                    // would lose that (review N3.1 HIGH).
                    let idx_eids: Vec<u32> =
                        idx_asts.iter().map(|e| self.lower_index_expr(e)).collect();
                    let eid = self.push_expr(ir::Expr::Signal {
                        net: POISON_NET,
                        word: None,
                    });
                    self.deferred_hier_sel.push(DeferredHierSelect {
                        eid,
                        prefix: self.cur_prefix.clone(),
                        path,
                        idx_eids,
                        part: None,
                    });
                    return eid;
                }
                let base_id = self.lower_expr(base);
                if self.expr_is_real(base_id) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "bit/part-select not defined on real operand",
                    );
                }
                let raw_off = self.lower_index_expr(index);
                let offset = self.norm_offset_if_net(base, raw_off);
                let width = self.const_u32_expr(1, 32);
                self.push_expr(ir::Expr::Select {
                    base: base_id,
                    offset,
                    width,
                    kind: ir::SelKind::Bit,
                })
            }
            ast::ExprKind::PartSelect { base, msb, lsb } => {
                // §13.3 UARR: `arr[hi:lo]` on a whole unpacked-array formal is an
                // unpacked SLICE, not a packed value — the md-packed rep would
                // silently return `{arr[hi],…,arr[lo]}`. Loud (index an element).
                if self.is_array_formal_whole(base) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a part-select of a whole unpacked-array formal \
                         (`arr[hi:lo]`) is an unpacked slice, not a value — index a \
                         single element (`arr[i]`)",
                    );
                    let _ = (msb, lsb);
                    return self.placeholder_expr();
                }
                // A part-select of a package ARRAY ELEMENT (`pkg::mem[i][m:l]`) is
                // loud (v1 does not normalize nested non-zero-LSB packed elements)
                // — the whole element `pkg::mem[i]` and a DIRECT `pkg::vec[m:l]`
                // both lower fine.
                if self.is_pkg_array_elem_subselect(base) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a part-select of a package array element \
                         (`pkg::arr[i][m:l]`) is not supported in v1 — read the \
                         whole element `pkg::arr[i]`, or import the name and \
                         part-select on it",
                    );
                    let _ = (msb, lsb);
                    return self.placeholder_expr();
                }
                // N3.4: a part-select on a bare multi-dim PACKED net selects
                // whole outer-dim elements, not flat bits.
                if let Some(sel) = self.try_packed_part_select(base, msb, lsb) {
                    return sel;
                }
                // HIER-REST-PS (read): a hierarchical part-select `dut.mem[i][m:l]` /
                // scalar `dut.v[m:l]` — the target net does not exist yet, so its
                // declared LSB (the offset normalization) is unknown. Defer with a
                // HierPart, mirroring the write side (`collect_lval_chunks` PartSelect):
                // resolution normalizes the offset against the element/net LSB. Without
                // it a non-zero-LSB hierarchical net read the raw offset → silent X.
                if let Some((path, idx_asts)) = self.hier_chain(base) {
                    let idx_eids: Vec<u32> =
                        idx_asts.iter().map(|e| self.lower_index_expr(e)).collect();
                    let lsb_id = self.lower_index_expr(lsb);
                    let msb_id = self.lower_index_expr(msb);
                    let width = self.width_from_msb_lsb_checked(msb, lsb, msb_id, lsb_id);
                    let eid = self.push_expr(ir::Expr::Signal {
                        net: POISON_NET,
                        word: None,
                    });
                    self.deferred_hier_sel.push(DeferredHierSelect {
                        eid,
                        prefix: self.cur_prefix.clone(),
                        path,
                        idx_eids,
                        part: Some(HierPart {
                            raw_off: lsb_id,
                            width,
                            kind: ir::SelKind::PartConst,
                        }),
                    });
                    return eid;
                }
                let base_id = self.lower_expr(base);
                if self.expr_is_real(base_id) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "bit/part-select not defined on real operand",
                    );
                }
                let lsb_id = self.lower_index_expr(lsb);
                let msb_id = self.lower_index_expr(msb);
                let asc = self.base_net_ascending(base);
                if self.base_has_neg_decl_lsb(base) {
                    self.error_neg_lsb_part_select();
                }
                let width = self.width_from_msb_lsb_dir(msb, lsb, msb_id, lsb_id, asc);
                // Ascending: normalize via the root net (handles array elements,
                // whose base is not a single-segment ident). Descending: the classic
                // single-seg path (raw for array elements) — byte-identical.
                let offset = if asc {
                    self.norm_offset_ascending(base, lsb_id)
                } else {
                    self.norm_offset_if_net(base, lsb_id)
                };
                self.push_expr(ir::Expr::Select {
                    base: base_id,
                    offset,
                    width,
                    kind: ir::SelKind::PartConst,
                })
            }
            ast::ExprKind::IndexedPart {
                base,
                offset,
                width,
                dir,
            } => {
                // §13.3 UARR: `arr[b+:w]` on a whole unpacked-array formal is an
                // unpacked slice, not a value — loud (see the PartSelect twin).
                if self.is_array_formal_whole(base) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an indexed part-select of a whole unpacked-array formal \
                         (`arr[b+:w]`) is an unpacked slice, not a value — index a \
                         single element (`arr[i]`)",
                    );
                    let _ = (offset, width, dir);
                    return self.placeholder_expr();
                }
                // Loud twin of the PartSelect guard: an indexed part-select of a
                // package array element (`pkg::mem[i][b+:w]`) is unsupported in v1.
                if self.is_pkg_array_elem_subselect(base) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an indexed part-select of a package array element \
                         (`pkg::arr[i][b+:w]`) is not supported in v1 — read the \
                         whole element `pkg::arr[i]`, or import the name and \
                         select on it",
                    );
                    let _ = (offset, width, dir);
                    return self.placeholder_expr();
                }
                // N3.4 follow-on: a constant indexed part-select on a bare/array
                // multi-dim packed net selects whole outer elements (`x[2+:2]` ≡
                // `x[3:2]`); a variable offset is loud (iverilog 13.0 aborts).
                if let Some(sel) = self.try_packed_indexed_part(base, offset, width, dir) {
                    return sel;
                }
                // HIER-REST-PS (read): a hierarchical indexed part-select
                // `dut.mem[i][b+:w]` / `dut.v[b+:w]` — defer with a HierPart (offset
                // normalized against the net LSB at fixup), the `+:`/`-:` twin of the
                // PartSelect arm above and the write side. `false` (descending) kind
                // mirrors the write side; an ascending hier net is a rare follow-on.
                if let Some((path, idx_asts)) = self.hier_chain(base) {
                    let idx_eids: Vec<u32> =
                        idx_asts.iter().map(|e| self.lower_index_expr(e)).collect();
                    let raw_off = self.lower_index_expr(offset);
                    let w = self.lower_const_width_expr(width);
                    let eid = self.push_expr(ir::Expr::Signal {
                        net: POISON_NET,
                        word: None,
                    });
                    self.deferred_hier_sel.push(DeferredHierSelect {
                        eid,
                        prefix: self.cur_prefix.clone(),
                        path,
                        idx_eids,
                        part: Some(HierPart {
                            raw_off,
                            width: w,
                            kind: indexed_sel_kind(dir, false),
                        }),
                    });
                    return eid;
                }
                let base_id = self.lower_expr(base);
                if self.expr_is_real(base_id) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "bit/part-select not defined on real operand",
                    );
                }
                let raw_off = self.lower_index_expr(offset);
                let asc = self.base_net_ascending(base);
                // Ascending net: a source-index range maps onto DECREASING internal
                // bits, so `+:` becomes a downward internal select and `-:` upward;
                // the offset is the base index normalized via the root net.
                let off = if asc {
                    self.norm_offset_ascending(base, raw_off)
                } else {
                    self.norm_offset_if_net(base, raw_off)
                };
                let width = self.lower_const_width_expr(width);
                let kind = indexed_sel_kind(dir, asc);
                self.push_expr(ir::Expr::Select {
                    base: base_id,
                    offset: off,
                    width,
                    kind,
                })
            }

            // ── structural ─────────────────────────────────────────
            ast::ExprKind::Concat { parts } => {
                // G1 (IEEE §6.16): a string concat in ANY context (display arg,
                // comparison, function/task arg, nested concat) lowers — like the
                // statement-level concat-ASSIGN desugar — to a `$sformatf("%s…",
                // parts…)` whose dynamic-width string result the static Concat node
                // cannot carry. Shared with `string_concat_special` so both paths
                // render byte-identically.
                if parts.iter().any(|p| self.expr_is_string_ast(p)) {
                    let part_refs: Vec<&ast::Expr> = parts.iter().collect();
                    return self.lower_string_concat_parts(&part_refs);
                }
                let part_ids: Vec<u32> = parts
                    .iter()
                    .map(|p| {
                        // A DIRECT Replicate operand may carry a zero count
                        // (§11.4.12.1); the Replicate arm takes the flag on entry.
                        if matches!(p.kind, ast::ExprKind::Replicate { .. }) {
                            self.repl_zero_ok = true;
                        }
                        self.lower_expr(p)
                    })
                    .collect();
                if part_ids.iter().any(|&p| self.expr_is_real(p)) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "real may not appear in concatenation (use $realtobits)",
                    );
                }
                self.push_expr(ir::Expr::Concat { parts: part_ids })
            }
            ast::ExprKind::Replicate { count, value } => {
                // Take (and clear) the concat-operand permission BEFORE lowering
                // anything below — see the field doc: it must not leak inward.
                let zero_ok = std::mem::take(&mut self.repl_zero_ok);
                // The count AST, kept because `count` is shadowed below by its
                // LOWERED id and the negative-count rule reads the AST.
                let e_count: &ast::Expr = count;
                // G1 (IEEE §6.16): a string replicate `{N{str}}` in any context
                // flattens to N copies of the value list, then lowers through the
                // shared `$sformatf("%s…")` desugar (mirrors `string_concat_special`).
                if value.iter().any(|p| self.expr_is_string_ast(p)) {
                    let Some(n) = self.const_eval_in_scope(count) else {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a non-constant replication count on a string is unsupported",
                        );
                        return self.placeholder_expr();
                    };
                    // §11.4.12.2 reaches this arm too, and it is the one the generic guard
                    // below cannot see: a string VARIABLE operand returns from here. A
                    // negative count used to render an empty string at exit 0 while BOTH
                    // oracles reject it. ZERO keeps its own meaning (legal as a direct
                    // concatenation operand, §11.4.12.1).
                    if n < 0 {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a replication count may not be negative (IEEE §11.4.12.2)",
                        );
                    }
                    let n = n.max(0) as usize;
                    let mut flat: Vec<&ast::Expr> =
                        Vec::with_capacity(n.saturating_mul(value.len()));
                    for _ in 0..n {
                        flat.extend(value.iter());
                    }
                    return self.lower_string_concat_parts(&flat);
                }
                // A replication count that reads an unpacked-array ELEMENT
                // (`{CNT[0]{…}}`, an arithmetic wrapper `{CNT[0]+1{…}}`, or inside
                // `$clog2(CNT[i])`) is NOT a runtime net the engine's
                // `const_u32_of_expr` can fold — it would read 0 (→ silent 0-width).
                // A replication count MUST be a foldable, non-negative constant
                // (IEEE §11.4.12.2), so here it is correct-or-loud: fold it to a
                // literal, else LOUD. The union of the two array detectors covers
                // BOTH a GAP-G-capturable element (module / generate / package
                // zero-based ascending array — folds) AND a shape GAP-G cannot fold
                // (descending / non-zero-based / multi-dimensional) or a RUNTIME
                // array or an out-of-range / negative index — none of which fold, so
                // they go loud instead of the old silent 0. Any NON-array count
                // (literal, param, genvar, packed bit/part-select, plain expression)
                // fails both gates → keeps its existing lowering, byte-identical
                // golden IR.
                let count = if self.count_lowers_real_param(count) {
                    // r19: a REAL param has no integer value and is not in `params`, so
                    // the count would fold to a silent 0 here (a 0-width replication)
                    // rather than the intended value. IEEE §11.4.12.2 wants a constant
                    // integral count — loud, never a silent 0.
                    //
                    // NOT `lower_index_expr`: tried, measured, reverted. `$clog2(R)`
                    // lowers to a SysFunc that `expr_is_real` does not call real, so
                    // the wrapper passed it through unchanged and the count folded to a
                    // silent 0 — strictly worse than the loud reject here. The IR-based
                    // decision only works where the operand reaches the boundary as a
                    // real `Const`.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a replication count that reads a real parameter is unsupported \
                         (a real has no integral constant value)",
                    );
                    self.placeholder_expr()
                } else if self.count_reads_const_array_elem(count)
                    || self.count_reads_array_param_elem(count)
                {
                    match self.const_eval_in_scope(count) {
                        Some(n) if n >= 0 => self.const_u32_expr(n as u32, 32),
                        _ => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a replication count that reads an unpacked-array \
                                 element must be a foldable, non-negative constant \
                                 (an out-of-range / negative / non-constant index, \
                                 or a descending / non-zero-based / multi-dimensional \
                                 array shape, is unsupported)",
                            );
                            self.placeholder_expr()
                        }
                    }
                } else if self.count_reads_runtime_net(count) {
                    // A replication count MUST be a constant expression (IEEE
                    // §11.4.12.2). A runtime-variable count previously lowered to
                    // a net the engine folded to 0 → silent 0-width (`{n{4'hA}}`
                    // with a `logic`/`int` `n` printed `000`, not iverilog's
                    // "reference to a variable in a constant expression" reject);
                    // make it LOUD instead. A constant count (param / genvar /
                    // $clog2 / const-function / literal) reads no net and keeps
                    // its existing lowering, byte-identical.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a replication count must be a constant expression (IEEE \
                         §11.4.12.2); a count that reads a runtime variable is \
                         unsupported",
                    );
                    self.placeholder_expr()
                } else {
                    // r19/B2: decide on the LOWERED count. The AST predicates above
                    // have to enumerate `ExprKind` arms and that does not converge —
                    // `MinTypMax` fell through every one of them, reached here as a
                    // real `Const`, and `{(R:R:R){1'b1}}` then span forever building a
                    // replication of the f64 bit pattern (RSS stays at 16 MB, so a
                    // memory cap does not catch it). Requiring a folded constant here
                    // is complete by construction: IEEE §11.4.12.2 wants a constant
                    // integral count, so anything that did not fold has no business
                    // reaching `Replicate`, where a non-constant silently became 0.
                    // Deciding on the lowered node is complete by construction where
                    // the AST walk is not; and because the wrapper converts an
                    // exactly-integral real at this boundary, a real count with an i64
                    // twin becomes correct rather than merely loud. NOT a "must be a
                    // folded Const" check — tried, and it false-rejected `$clog2(P)`
                    // on an INTEGER param, which the engine evaluates as a constant
                    // without folding at elaborate time.
                    // …and because the engine's fold is SHALLOW, a count it cannot
                    // reduce became `unwrap_or(0)` = a SILENT 0-width replication:
                    // `{P*2{…}}`, `{int'(3){…}}`, `{f(3){…}}`, `{(P>1?3:2){…}}` and
                    // `{$bits(v)/4{…}}` all printed nothing while iverilog replicated.
                    // §11.4.12.2 requires a constant count, so the shared width/count
                    // funnel hands the engine a `Const` when the const domain can
                    // prove one (and changes nothing otherwise).
                    self.lower_const_width_expr(count)
                };
                // §11.4.12.2: the count is a NON-NEGATIVE constant expression, and
                // BOTH oracles reject a negative one ("Concatenation repeat may not
                // be negative" / "Replication value of < 0 … not legal"). vita used
                // to hand the engine the two's-complement bit pattern instead:
                // `{W{1'b1}}` with `W = -56` replicated 4294967240 times and the
                // result was truncated to the target, printing 255 at exit 0.
                // Decided on the AST, self-determined. NOT on the lowered node: its
                // u32 read SATURATES an `Add`/`Sub` to 0, so `{(2-3){…}}` and
                // `{(W-4){…}}` reached the zero check below and it reported "a
                // replication count of zero" about a count of -1. And NOT in the
                // width-unlimited const domain: `{(4'd0-4'd1){1'b1}}` is 15
                // replications in both oracles while `{(4'sd0-4'sd1){1'b1}}` is
                // rejected — same bit pattern, and the unlimited fold calls both of
                // them -1, so reading the sign there would false-reject a correct
                // design. The sign only exists at the count's own width, which is
                // exactly what `const_bound_signed` walks.
                //
                // A second reading off the lowered `Const` was tried and removed: a
                // mutation battery could not find a cell it answered that this one
                // did not, including the shape it was added for (a count naming an
                // inlined function's formal — the substitution rewrites the AST, so
                // this walk sees the literal).
                if self.const_bound_signed(e_count).is_some_and(|n| n < 0) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a replication count may not be negative (IEEE §11.4.12.2)",
                    );
                    return self.placeholder_expr();
                }
                // §11.4.12.1: a zero replication count is legal ONLY as a direct
                // concatenation operand (where it contributes nothing — that path
                // already works). Anywhere else it is an error, not a silent
                // 0-width value: `r = {(0){1'b1}}` printed 0 with exit 0 while
                // iverilog rejects, and the width-honest count fold now routes
                // `{(4'd15+4'd1){1'b1}}` (a 4-bit wrap) into this same position.
                if !zero_ok && self.const_of_expr_u32(count) == Some(0) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a replication count of zero is only legal as a direct \
                         operand of a concatenation (IEEE §11.4.12.1)",
                    );
                    return self.placeholder_expr();
                }
                // hdl-ast `value: Vec<Expr>` is the element LIST (no wrapper
                // Concat); sim-ir Replicate wants ONE `value: u32` → wrap in a
                // Concat node. (For a single element this is a 1-part Concat,
                // kept for shape-uniformity / determinism.)
                let part_ids: Vec<u32> = value.iter().map(|p| self.lower_expr(p)).collect();
                if part_ids.iter().any(|&p| self.expr_is_real(p)) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "real may not appear in concatenation (use $realtobits)",
                    );
                }
                let value = self.push_expr(ir::Expr::Concat { parts: part_ids });
                self.push_expr(ir::Expr::Replicate { count, value })
            }

            // ── SV static cast `casting_type'(expr)` (§6.24) ───────
            ast::ExprKind::Cast { target, expr } => self.lower_cast(target, expr),

            // ── calls ──────────────────────────────────────────────
            ast::ExprKind::SysCall { name, args } => {
                // v7: `$bits` is a TYPE function — its argument is NOT
                // evaluated (IEEE §20.6.2) — and folds to a const at
                // elaborate (IR-0). Array views bypass lowering entirely
                // (whole-array reads are loud by design).
                if name.name == "$bits" && args.len() == 1 {
                    return self.lower_bits_fold(&args[0]);
                }
                // SYS-INTRO: array-query / dimension introspection const-folds
                // (IR-0, like $bits). A TYPE/net reference, not evaluated.
                if let Some(folded) = self.try_introspect_fold(&name.name, args) {
                    return folded;
                }
                // SYS-INTRO잔여: `$countbits(expr, ctrl…)` desugars to a per-bit
                // case-eq sum (IR-0, no new SysFuncId). iverilog 13.0 crashes on it.
                if name.name == "$countbits" {
                    return self.lower_countbits(args);
                }
                // v7: a SEEDED $random updates its ref argument — legal ONLY
                // as the direct rhs of a blocking assign (the special form
                // bypasses this arm). Any other placement is loud, never a
                // silent unseeded draw.
                if name.name == "$random" && !args.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "seeded $random is supported only as the direct rhs of \
                         a blocking assignment (v7)",
                    );
                    return self.placeholder_expr();
                }
                // v7: $fopen mutates the file table — direct-rhs only.
                if name.name == "$fopen" {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "$fopen is supported only as the direct rhs of a \
                         blocking assignment (v7)",
                    );
                    return self.placeholder_expr();
                }
                // v7 P2-C: $sformatf renders through the kernel-side format
                // engine — direct-rhs only (the dominant TB pattern is
                // `msg = $sformatf(...); $display("%s", msg);`).
                if name.name == "$sformatf" {
                    // §4.5.252: normally loud — a `$sformatf` node reaching the generic
                    // `eval` is rendered by a DEGENERATE arm that cannot see its format
                    // string (`eval/sysfunc.rs`), so it would silently produce the wrong
                    // bytes. `sformatf_expr_ok` is set only where the node is guaranteed to
                    // reach a FORMAT-AWARE evaluator instead: the direct rhs of a string
                    // blocking assign and a string `return`, both of which render through
                    // `format_args_str` (`k_sformatf` / `frame_rhs_value`). Measured, one
                    // shape at a time — a ternary arm and a task argument both go through
                    // `eval` and were blank / garbage, so neither sets the flag.
                    if self.sformatf_expr_ok {
                        if !matches!(
                            args.first().map(|a| &a.kind),
                            Some(ast::ExprKind::StrLit { .. })
                        ) {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "$sformatf needs a string-literal format (v7)",
                            );
                            return self.placeholder_expr();
                        }
                        let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
                        return self.push_expr(ir::Expr::SysFunc {
                            which: ir::SysFuncId::Sformatf,
                            args: arg_ids,
                        });
                    }
                    self.error(
                        MsgCode::ElabUnsupported,
                        "$sformatf is supported only as the direct rhs of a \
                         blocking assignment (v7)",
                    );
                    return self.placeholder_expr();
                }
                // ⚠️ Same stale rule as the file-read family below: §4.5.374's hoist
                // opened `if` conditions, `case` scrutinees and the rest for this call
                // too, so "direct rhs only (v7)" stopped being true and reading it
                // literally sent users to rewrite working code. What is refused is
                // narrower, and it is a different reason from the file-read family's:
                // this call WRITES its second argument, and the hoist evaluates it
                // ahead of the surrounding expression, so any other read of that same
                // variable in the same statement would see the post-call value where
                // both oracles see the pre-call one.
                if name.name == "$value$plusargs" {
                    let saved = self.cur_span;
                    self.cur_span = Some(e.span);
                    self.error(
                        MsgCode::ElabUnsupported,
                        "`$value$plusargs` writes the variable passed as its second \
                         argument, and vita evaluates the call into a temporary before \
                         the surrounding statement runs. This position is refused \
                         because the statement would then observe that write out of \
                         order — or, in a position that may be skipped or repeated (the \
                         right operand of `&&` / `||`, an arm of `?:`, a loop condition, \
                         a `$monitor` argument), would perform it a different number of \
                         times than written. Call it in its own statement first, then \
                         use the result and the written variable here",
                    );
                    self.cur_span = saved;
                    return self.placeholder_expr();
                }
                // v9 SYS-READ: the fd-ADVANCING file-read functions mutate the fd
                // read state (and some write ref/dest args), so they are
                // direct-rhs-only special forms (the legal placements intercept in
                // lower_stmt BEFORE lower_expr is reached). Reaching here = an
                // illegal nested placement. This guard MUST stay in sync with
                // `file_read_int_special`. Round-9 FIO: `$feof` is PURE (reads the
                // EOF flag, no mutation) and is intentionally NOT in this list —
                // it maps through `map_sysfunc` like any pure value sysfunc, so
                // `while (!$feof(fd))` and other expression/condition placements
                // work (re-evaluation per iteration is exactly correct).
                if matches!(
                    name.name.as_str(),
                    "$fgetc" | "$ungetc" | "$fgets" | "$fread" | "$fscanf" | "$sscanf"
                ) {
                    // ⚠️ This message used to state the pre-§4.5.374 rule — "supported
                    // only as the direct rhs of a blocking assignment (v9)" — which had
                    // been FALSE since the hoist opened NBA right-hand sides, `if`
                    // conditions, `case` scrutinees, `repeat` counts, task arguments,
                    // nested expressions and lvalue indices. Taken at face value it told
                    // users to rewrite working code, and the `(v9)` tag pointed at the
                    // rule of the day rather than today's. Say what is actually true.
                    //
                    // The caret goes on the CALL, not the enclosing statement: the
                    // statement can span several lines and the reader needs the operand
                    // that is illegal, which is what the neighbouring queue-pop
                    // diagnostic already does.
                    let saved = self.cur_span;
                    self.cur_span = Some(e.span);
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "`{}` advances the file position, so vita evaluates it into a \
                             temporary once, before the statement runs. This position is \
                             refused because the call would then happen a DIFFERENT NUMBER \
                             OF TIMES than written: the right operand of `&&` / `||` and an \
                             arm of `?:` may be skipped, a loop condition is re-evaluated \
                             per iteration, and a `$monitor` / `$strobe` argument is \
                             re-rendered on every later change and would show the frozen \
                             temporary. Read it into a variable in its own statement first, \
                             then use that variable here. Placements evaluated exactly once \
                             per execution are supported — a blocking or nonblocking rhs, \
                             an `if` condition, a `case` scrutinee, a `repeat` count, a \
                             `$display`-style argument, an lvalue index",
                            name.name
                        ),
                    );
                    self.cur_span = saved;
                    return self.placeholder_expr();
                }
                // v9 rank 6: $dist_uniform advances the ref seed, $cast (func form)
                // writes the dst ref — both direct-rhs-only (the legal placements
                // intercept in lower_stmt before lower_expr). Reaching here is an
                // illegal nested placement — loud, never a silent unseeded draw /
                // dropped cast. MUST stay in sync with dist_uniform_special /
                // cast_special.
                if matches!(
                    name.name.as_str(),
                    "$dist_uniform"
                        | "$dist_normal"
                        | "$dist_exponential"
                        | "$dist_poisson"
                        | "$dist_chi_square"
                        | "$dist_t"
                        | "$dist_erlang"
                        | "$cast"
                ) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "$dist_* / $cast (function form) are supported only as \
                         the direct rhs of a blocking assignment (v9/v19)",
                    );
                    return self.placeholder_expr();
                }
                // v7: the $test$plusargs query must be a string literal (the
                // full string type lands with P2-C).
                if name.name == "$test$plusargs"
                    && !matches!(
                        args.first().map(|a| &a.kind),
                        Some(ast::ExprKind::StrLit { .. })
                    )
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "$test$plusargs needs a string-literal query (v7)",
                    );
                    return self.placeholder_expr();
                }
                let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
                match map_sysfunc(&name.name) {
                    Some(which) => {
                        // v7 bit-vector predicates are integral-only; a real
                        // argument would silently count IEEE-754 mantissa
                        // bits — loud instead (IEEE: illegal operand type).
                        if matches!(
                            which,
                            ir::SysFuncId::CountOnes
                                | ir::SysFuncId::OneHot
                                | ir::SysFuncId::OneHot0
                                | ir::SysFuncId::IsUnknown
                        ) && arg_ids.iter().any(|&a| self.expr_is_real(a))
                        {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "real operand is not legal for a bit-vector system function",
                            );
                            return self.placeholder_expr();
                        }
                        self.push_expr(ir::Expr::SysFunc {
                            which,
                            args: arg_ids,
                        })
                    }
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "unsupported system function in expression",
                        );
                        self.placeholder_expr()
                    }
                }
            }
            // N7-REST B-CRV final: a bare `obj.randomize() with {…}` value-expression
            // (e.g. inside `if (...)`) is outside v1 — only the statement form and the
            // direct-assign rhs `r = obj.randomize() with {…}` are lowered (both route
            // through `try_emit_randomize` before reaching here). Loud, never silent.
            ast::ExprKind::RandomizeWith(_) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "inline `randomize() with {…}` is supported as a statement or a direct \
                     assignment rhs (`r = obj.randomize() with {…};`), not as a nested \
                     value-expression",
                );
                self.placeholder_expr()
            }
            // ⓑ-breadth (v17): `arr.METHOD() with (expr)`. A REDUCTION yields a
            // scalar and lowers here; a LOCATOR yields a queue (statement-level
            // only, routed through `dyn_blocking_special`) → loud in expr position.
            ast::ExprKind::ArrayMethodWith(amw) => self.lower_reduction_with_expr(amw),
            ast::ExprKind::Call { name, args } => {
                // SVA-REST `let NAME(formals) = expr;` call: substitute the body with
                // positional formal→actual binding. A real FUNCTION/TASK of the same
                // name WINS (it is the genuine callable) — IEEE 1800 §11.13 requires a
                // `let` name to be unique in scope, but if a design illegally co-declares
                // both, the function must not be silently shadowed by the let (review).
                if name.segments.len() == 1
                    && self.let_table.contains_key(&name.segments[0].name)
                    && !self.func_table.contains_key(&name.segments[0].name)
                    && !self.task_table.contains_key(&name.segments[0].name)
                {
                    return self.lower_let_use(&name.segments[0].name.clone(), args, e.span);
                }
                // N7: a class method call (`obj.m(args)` / `this.m()` / `super.m()`)
                // → `Expr::Call{method_fid, [this, …args]}`. Checked before the
                // ordinary function-inline path (which expects a free function).
                if let Some(eid) = self.try_class_method_call(name, args) {
                    return eid;
                }
                self.inline_function(name, args)
            }

            // ── transparent / placeholder ──────────────────────────
            ast::ExprKind::Paren { inner } => self.lower_expr(inner), // unwrap, no IR node
            ast::ExprKind::MinTypMax { typ, .. } => self.lower_expr(typ), // pick typ branch
            // v2: a string literal interns as a `StrUtf8` const. Used by $systask
            // format/args ($display("...", x), $dumpfile("dump.vcd")). Escapes are
            // processed by `parse_str_literal`; the const pool dedups StrUtf8 vs
            // Numeric via the repr tag (intern_const ConstKey).
            ast::ExprKind::StrLit { raw } => {
                let cid = self.intern_str_literal(raw, e.span);
                self.push_expr(ir::Expr::Const { val: cid })
            }
            ast::ExprKind::RealLit { raw, .. } => {
                let cid = self.intern_const(parse_real_literal(raw));
                self.push_expr(ir::Expr::Const { val: cid })
            }
            // v5 ⑥: `new[n]` reached OUTSIDE `d = new[n]` (its only legal
            // placement, intercepted in `dyn_blocking_special`).
            ast::ExprKind::New { size, src } => {
                // V2005 compat: a net actually named `new` — re-lower as the
                // indexed read the source meant (`new[i]`).
                if src.is_none() && self.lookup_net_scoped("new").is_some() {
                    let span = e.span;
                    let fake = ast::Expr {
                        kind: ast::ExprKind::BitSelect {
                            base: Box::new(ast::Expr {
                                kind: ast::ExprKind::Ident(ast::HierPath {
                                    segments: vec![ast::Ident {
                                        name: "new".to_string(),
                                        span,
                                    }],
                                    span,
                                }),
                                span,
                            }),
                            index: size.clone(),
                        },
                        span,
                    };
                    return self.lower_expr(&fake);
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    "`new[n]` is only valid as the rhs of a blocking assignment to a dynamic-array handle",
                );
                self.placeholder_expr()
            }
            // v5 ⑥: bare `$` — meaningful only inside a queue element select
            // (`lower_dyn_index` pins the substitution).
            ast::ExprKind::Dollar => match self.dollar_subst {
                Some(eid) => eid,
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "`$` is only valid inside a queue element select (`q[$]`)",
                    );
                    self.placeholder_expr()
                }
            },
            // N7: `null` — the null class handle (object-id 0, 32-bit).
            ast::ExprKind::Null => self.const_u32_expr(0, 32),
            // N7: `new`/`new(args)` (class allocation) is only valid as the
            // DIRECT rhs of a blocking assignment to a class handle, where it is
            // intercepted (`lower_class_new_assign`). Any other position is loud.
            ast::ExprKind::ClassNew { .. } => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "`new` (class object allocation) is only valid as the rhs of an assignment to a class handle",
                );
                self.placeholder_expr()
            }
            ast::ExprKind::Dist { .. } => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "`dist` is only valid inside a constraint",
                );
                self.placeholder_expr()
            }
            ast::ExprKind::AssignPattern(_) => {
                // An assignment pattern is only valid bound to an unpacked-array
                // assignment (handled in `array_assign_special` / the decl-init path;
                // 1-D or multi-dim). Anywhere else (a sub-expression, a packed/struct
                // target) is unsupported — loud.
                self.error(
                    MsgCode::ElabUnsupported,
                    "an assignment pattern `'{…}` is supported only as the whole \
                     right-hand side of an unpacked array assignment",
                );
                self.placeholder_expr()
            }
            ast::ExprKind::Error => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "cannot lower parse-error expression",
                );
                self.placeholder_expr()
            }
        }
    }

    /// What a REAL operand does to a binary operator once BOTH sides are lowered —
    /// IEEE §6.2 permanent illegalities plus the §11.4.9 `**` route.
    ///
    /// Returns `Some(id)` when the operator is REPLACED (only `**`, which becomes the
    /// `$pow` sysfunc) and `None` when the caller should build its own `Binary` node.
    /// A permanently illegal operator is reported here and still returns `None`, so
    /// the caller's node is well formed while the elaboration is poisoned.
    ///
    /// ⚠️ THIS IS A FUNCTION BECAUSE IT HAS TWO CALLERS. It used to be inline in
    /// `lower_expr`'s `Binary` arm, and `lower_expr_ctx`'s `Binary` arm — the
    /// width-aware twin a fill literal diverts to — never did any of it. So a fill
    /// anywhere in the node turned every rule here off: `r & '1` and `r << '1` printed
    /// a silent `0` at exit 0 where the `1'b1` spellings beside them were E3009
    /// (iverilog rejects both), and `r ** '1` lost the `$pow` desugar entirely and
    /// read `0` where BOTH oracles — and its own `r ** 1'b1` twin — read `3`.
    pub(crate) fn binary_real_operand_route(
        &mut self,
        irop: ir::BinOp,
        lhs: u32,
        rhs: u32,
    ) -> Option<u32> {
        if !(self.expr_is_real(lhs) || self.expr_is_real(rhs)) {
            return None;
        }
        match irop {
            ir::BinOp::Mod => self.error(
                MsgCode::ElabUnsupported,
                "modulo (%) not defined on real operand",
            ),
            // IEEE 1800-2017 §11.4.9: `**` with a real operand yields a REAL result =
            // pow(base, exp). Desugar to the `$pow` sysfunc (libm::pow) instead of
            // loud-rejecting — both operands read as real (`real_arg` converts an
            // integral operand to f64), so `2.0**3`, `r**2`, and `2**2.0` all fold.
            ir::BinOp::Pow => {
                return Some(self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Pow,
                    args: vec![lhs, rhs],
                }));
            }
            ir::BinOp::BitAnd
            | ir::BinOp::BitOr
            | ir::BinOp::BitXor
            | ir::BinOp::BitXnor
            | ir::BinOp::Shl
            | ir::BinOp::Shr
            | ir::BinOp::AShl
            | ir::BinOp::AShr => self.error(
                MsgCode::ElabUnsupported,
                "bitwise/shift/reduction not defined on real operand",
            ),
            _ => {}
        }
        None
    }
}
