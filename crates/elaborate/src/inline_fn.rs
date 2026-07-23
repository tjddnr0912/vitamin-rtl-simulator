//! inline function expansion — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// A function's declared RETURN bit-vector width. `None` = a `real`/`realtime`
/// return (handled by the caller: NOT a bit-width context) OR an unfoldable
/// (param-width) integral return (the caller treats that as unknown-width ⇒ route).
pub(crate) fn ast_func_return_width(f: &ast::FunctionDef) -> Option<u32> {
    match f.ret_type {
        ast::ParamType::Integer => Some(32),
        ast::ParamType::Time => Some(64),
        ast::ParamType::Real | ast::ParamType::Realtime => None,
        ast::ParamType::Implicit => ast_kind_range_width(ast::NetVarKind::Reg, f.range.as_ref()),
    }
}

/// Does a STATIC function's inline lowering miscompute a widening context-sensitive
/// assignment? If so `build_frame_set` routes it to the correct frame path (㊁).
/// `func_widths` maps a callee name to its return width (a call operand's self-width).
pub(crate) fn body_needs_context_width(
    f: &ast::FunctionDef,
    func_widths: &std::collections::HashMap<String, u32>,
) -> bool {
    let mut widths: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut unknown_bv: std::collections::HashSet<String> = std::collections::HashSet::new();
    // A bit-vector target with a foldable width → `widths` (precise check); with an
    // unfoldable (param-width) width → `unknown_bv` (fail-closed route). A non-bit-
    // vector (`real`/`string`/handle) target is in NEITHER set (never routed on).
    let mut classify = |name: &str, kind: ast::NetVarKind, range: Option<&ast::Range>| {
        if !ast_kind_is_bit_vector(kind) {
            return;
        }
        match ast_kind_range_width(kind, range) {
            Some(w) => {
                widths.insert(name.to_string(), w);
            }
            None => {
                unknown_bv.insert(name.to_string());
            }
        }
    };
    for p in &f.ports {
        classify(
            &p.name.name,
            p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
            p.range.as_ref(),
        );
    }
    let mut decls = f.body_decls.clone();
    collect_block_local_decls(&f.body, &mut decls);
    for d in &decls {
        for n in &d.names {
            classify(&n.name.name, d.kind, d.range.as_ref());
        }
    }
    // The return var: `real`/`realtime` is not a bit-width context (skip); an integral
    // return with a foldable width → `widths`, an unfoldable (param-width) one →
    // `unknown_bv` (fail-closed).
    match f.ret_type {
        ast::ParamType::Real | ast::ParamType::Realtime => {}
        _ => match ast_func_return_width(f) {
            Some(w) => {
                widths.insert(f.name.name.clone(), w);
            }
            None => {
                unknown_bv.insert(f.name.name.clone());
            }
        },
    }
    assign_needs_context_width(&f.body, &widths, &unknown_bv, func_widths)
}

impl Elaborator<'_> {
    /// The declared return `(width, signed)` of a const function, or None if the
    /// return type is outside the integer domain (real/realtime/string).
    pub(crate) fn const_fn_ret_wsign(&self, f: &ast::FunctionDef) -> Option<(u32, bool)> {
        if f.ret_string || matches!(f.ret_type, ast::ParamType::Real | ast::ParamType::Realtime) {
            return None;
        }
        if let Some(r) = &f.range {
            let hi = self.const_eval_in_scope(&r.msb)?;
            let lo = self.const_eval_in_scope(&r.lsb)?;
            return Some((u32::try_from((hi - lo).unsigned_abs() + 1).ok()?, f.signed));
        }
        match f.ret_type {
            ast::ParamType::Integer => Some((32, true)), // `int`/`integer` are 32-bit signed
            ast::ParamType::Time => Some((64, false)),
            ast::ParamType::Implicit => Some((1, false)), // a bare `function f;` is 1-bit
            _ => None,
        }
    }

    pub(crate) fn inline_function(&mut self, name: &ast::HierPath, args: &[ast::Expr]) -> u32 {
        // v5 ⑥: `handle.method(args)` — a 2-segment call whose head is a dyn
        // handle is a METHOD, not a hierarchical call.
        if name.segments.len() == 2 {
            // N5: covergroup method in expression position (`c.get_coverage()`).
            if self.cover_inst(&name.segments[0].name).is_some() {
                return self
                    .synth_cover_method_expr(&name.segments[0].name, &name.segments[1].name);
            }
            // R2: `dyn_handle_read` so a read-only `input` dyn-array formal (aliased
            // to the caller net via `dyn_subst`) resolves `b.size()` / `b.num()`.
            if let Some((net, kind)) = self.dyn_handle_read(&name.segments[0].name) {
                return self.lower_dyn_method_expr(net, kind, &name.segments[1].name, args);
            }
            // v7 P2-C: string methods. A frame-local string slot is also a
            // `NetKind::String` net now (round-14 V1); its receiver handle reads via
            // the engine's frame-aware `str_bytes` (mirrors `read_net`), so this
            // dispatch is correct for both module-scope and frame-local strings.
            if let Some(net) = self.string_handle(&name.segments[0].name) {
                return self.lower_string_method_expr(net, &name.segments[1].name, args);
            }
            // G1: a string method on a string-domain FORMAL / inline LOCAL. A frame
            // string formal is a 1-bit wire slot and an inline string local is a subst
            // value — neither is a bare `NetKind::String` net, so `string_handle` above
            // misses them and the call would fall to the misleading "hierarchical
            // function call (deferred)" error. But the receiver IS string-domain
            // (`expr_is_string_ast`), and when its lowered handle is byte-readable
            // (frame-slot `Signal` / literal `Const`, exactly what `string_index_read`
            // routes `s[i]` through), dispatch the method on it — `.len()`, `.getc()`,
            // `.atoi()`, … all read via the engine's `handle_str_bytes`.
            {
                let base = ast::Expr {
                    kind: ast::ExprKind::Ident(ast::HierPath {
                        segments: vec![name.segments[0].clone()],
                        span: name.segments[0].span,
                    }),
                    span: name.segments[0].span,
                };
                if self.expr_is_string_ast(&base) {
                    let h = self.lower_expr(&base);
                    if self.handle_is_str_readable(h) {
                        return self.lower_string_method_expr_handle(
                            h,
                            &name.segments[1].name,
                            args,
                        );
                    }
                }
            }
            // IEEE §26.3 (round-7): a package-scoped function call `pkg::name(args)`.
            // Reached only when the head is not a handle / cover / class object above
            // (the object-method and package-scope namespaces are disjoint here), and
            // the head names a known package. Resolve the callee in that package.
            if self.pkg_funcs.contains_key(&name.segments[0].name) {
                let pkg = name.segments[0].name.clone();
                let fname = name.segments[1].name.clone();
                return self.inline_pkg_function(&pkg, &fname, args);
            }
        }
        if name.segments.len() >= 2 {
            // Family D (r17): DEFER a hierarchical function call `u1.f(x)`. The callee
            // lives in a child instance not yet elaborated at pass 7 (its FuncId does not
            // exist yet), so — mirroring the hier-NET read defer (`DeferredHier`) — lower
            // the args in the CALLER scope now and patch the placeholder
            // `Expr::Call{func:POISON_FID}` to the callee's per-instance FuncId in
            // `resolve_deferred_hier_call` after every instance exists. A target that is
            // not a HIER-CALLABLE framed function (see `reserve_frame_func`) resolves to
            // no `hier_funcs` entry → loud there (correct-or-loud). Args lower
            // positionally; a `.named(v)` actual can't be reordered without the callee's
            // formals here, so it falls through to the loud reject below.
            if args
                .iter()
                .all(|a| !matches!(a.kind, ast::ExprKind::NamedArg { .. }))
            {
                let arg_ids: Vec<u32> = args.iter().map(|a| self.lower_expr(a)).collect();
                let argc = arg_ids.len();
                let eid = self.push_expr(ir::Expr::Call {
                    func: POISON_FID,
                    args: arg_ids,
                });
                self.deferred_hier_calls.push(DeferredHierCall {
                    eid,
                    prefix: self.cur_prefix.clone(),
                    path: name.segments.iter().map(|s| s.name.clone()).collect(),
                    argc,
                });
                return eid;
            }
            self.error(
                MsgCode::ElabUnsupported,
                "hierarchical function call with named arguments (deferred)",
            );
            return self.placeholder_expr();
        }
        if name.segments.is_empty() {
            self.error(MsgCode::ElabUnsupported, "empty function call name");
            return self.placeholder_expr();
        }
        let fname = name.segments[0].name.clone();

        // Clone the def out of the table so we can mutate `self` while lowering.
        let func = match self.func_table.get(fname.as_str()) {
            Some(f) => f.clone(),
            None => {
                self.error(
                    MsgCode::ElabUnresolvedName,
                    &format!("call to undeclared function `{fname}`"),
                );
                return self.placeholder_expr();
            }
        };

        // G10: reorder named args (`.formal(v)` / `.formal()`) to positional using the
        // callee's formals (if any named arg is present), so the frame and inline paths
        // below both see a plain positional list.
        let reordered_args;
        let args: &[ast::Expr] = if args
            .iter()
            .any(|a| matches!(a.kind, ast::ExprKind::NamedArg { .. }))
        {
            match self.resolve_named_args(&fname, &func.ports, args) {
                Some(v) => {
                    reordered_args = v;
                    &reordered_args
                }
                None => return self.placeholder_expr(),
            }
        } else {
            args
        };

        // R4: a `string` return type (IEEE §6.16) is materialized through the FRAME
        // path — a `NetKind::String` return slot the body writes (`return $sformatf(...)`
        // renders via the engine's `frame_rhs_value`) and `run_frame_call` copies out as
        // a string Value; the call reads as a string operand (`ir_expr_is_string`).
        // `build_frame_set` forces every string-returning function into the frame set, so
        // `frame_idx` is populated below. A missing `fid` (defensive) stays loud.
        if func.ret_string {
            if let Some(&fid) = self.frame_idx.get(fname.as_str()) {
                return self.emit_frame_call(fid, &func, args);
            }
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "function `{fname}`: a `string` return type is not yet supported \
                     (return the text via a `string` output/ref formal instead)"
                ),
            );
            return self.placeholder_expr();
        }
        // B1 frame-call: a function in the frame set (automatic OR on a recursion
        // cycle) is LOWERED to the func arena (reserved in step 6.5), not inlined.
        // Emit an `Expr::Call` to its FuncId — the args are lowered in the CALLER
        // scope (they read caller nets / outer subst, never the callee formals).
        if let Some(&fid) = self.frame_idx.get(fname.as_str()) {
            return self.emit_frame_call(fid, &func, args);
        }
        // A non-framed recursive function reaching here means the recursion cycle
        // was missed by `build_frame_set` (it should have been framed) — the inline
        // path's `inline_stack` guard still catches it loud (in `inline_resolved_func`).
        self.inline_resolved_func(&func, args)
    }

    /// Inline an ALREADY-RESOLVED function definition as a straight-line fold, formals
    /// bound to actuals. Shared by the bare-name inline path (`inline_function`, after
    /// the frame dispatch) and the package-scoped pure-inline path
    /// (`inline_pkg_function`). Does NOT consult the frame set — the caller decides
    /// frame vs inline. The `inline_stack` guard still catches any missed recursion
    /// cycle loud.
    pub(crate) fn inline_resolved_func(
        &mut self,
        func: &ast::FunctionDef,
        args: &[ast::Expr],
    ) -> u32 {
        let fname = func.name.name.clone();
        if self.inline_stack.iter().any(|n| n == &fname) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "recursive function `{fname}` (frame-call deferred; cycle: {} -> {fname})",
                    self.inline_stack.join(" -> ")
                ),
            );
            return self.placeholder_expr();
        }
        // Functions take INPUT formals only; an output/inout formal is illegal.
        if func
            .ports
            .iter()
            .any(|p| !matches!(p.dir, ast::PortDir::Input))
        {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("function `{fname}` has output/inout formal (illegal)"),
            );
            return self.placeholder_expr();
        }
        let inputs: Vec<ast::TfPort> = func.ports.clone();
        let Some(eff_args) = self.fill_default_args(&fname, &inputs, args) else {
            return self.placeholder_expr();
        };

        // (1) Lower each ACTUAL arg (incl. filled defaults) in the CALLER scope
        //     FIRST (before pushing the substitution frame) so args see the caller's
        //     nets and any OUTER substitution (nested inlining), never the function's
        //     own formals. §11.6: an arg is in the context of its FORMAL's width, so a
        //     fill grows to that width (non-fill ⇒ byte-identical via lower_expr).
        let mut actual_ids: Vec<u32> = Vec::with_capacity(eff_args.len());
        let mut dyn_binds: Vec<(String, u32)> = Vec::new();
        for (i, &a) in eff_args.iter().enumerate() {
            let p = &inputs[i];
            // R2: a read-only `input` dyn-array formal ALIASES the caller's DynArray
            // net (no value copy) so `b.size()`/`b[i]` read the caller's heap. The
            // alias rides `dyn_subst` (read-only); the actual_id slot is a placeholder
            // (never read — `b` is only used via the dyn read paths). A non-bare /
            // mismatched actual is loud (correct-or-loud).
            if self.is_input_dyn_array_formal(p) {
                match self.dyn_array_actual_net(a, p) {
                    Some(net) => dyn_binds.push((p.name.name.clone(), net)),
                    None => self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "function `{fname}`: dynamic-array formal `{}` needs a bare \
                             matching dynamic-array actual",
                            p.name.name
                        ),
                    ),
                }
                actual_ids.push(self.placeholder_expr());
                continue;
            }
            let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
            let (w, _, _, _) = self.range_to_dims(kind, p.range.as_ref(), p.signed);
            actual_ids.push(self.lower_ctx_or_plain(a, w));
        }

        // (2) Reduce the straight-line body → an ExprId, formals bound to actuals. The
        //     R2 dyn aliases are pushed for the body's read paths and popped after.
        let n_dyn = dyn_binds.len();
        self.dyn_subst.extend(dyn_binds);
        self.inline_stack.push(fname);
        let result = self.reduce_function_body(func, &inputs, &actual_ids);
        self.inline_stack.pop();
        self.dyn_subst.truncate(self.dyn_subst.len() - n_dyn);
        // R2 / §6.11.3: a 2-state return (`int`/`byte`/`bit`/…) must coerce X/Z→0. The
        // frame RETURN SLOT does this, but the R2 carve-out forced this (`ret_two_state`)
        // function onto the INLINE path, which has no slot — so a 4-state-element read
        // (`return b[i]` on `logic b[]`) or an OOB read would leak X/Z. Coerce the folded
        // return value here. GATED on an R2 call (`n_dyn > 0`), so every non-R2 inline
        // function stays byte-identical (they were the only `ret_two_state` inline path).
        if n_dyn > 0 && func.ret_two_state {
            let rw = ast_func_return_width(func).unwrap_or(32).max(1);
            self.coerce_two_state(result, rw)
        } else {
            result
        }
    }

    /// IEEE §26.3 (round-7): resolve + frame-lower a package-scoped function call
    /// `pkg::name(args)`. Supported only for a SELF-CONTAINED, straight-line function
    /// — one whose body has no control flow and references only its own formals /
    /// locals (plus literals, operators, `$sys` calls, and unambiguous package-scoped
    /// `pkg::CONST` reads). Such a function computes purely from its actuals, so lowering
    /// it in the CALLER's module scope can never mis-resolve a bare name or collide with
    /// a module net (provably correct-or-loud, whether or not the package is imported).
    /// It is routed through the FRAME path (not the inline fold) so the return
    /// expression gets its IEEE §11.6.1 assignment-context width — byte-for-byte the same
    /// result as calling the function by its bare imported name. A stateful /
    /// control-flow / externally-referencing package function is loud: the workaround is
    /// to `import pkg::*` and call by the bare name (which brings the whole package scope
    /// into the module, so the ordinary inline / frame path resolves the body).
    pub(crate) fn inline_pkg_function(&mut self, pkg: &str, name: &str, args: &[ast::Expr]) -> u32 {
        let func = match self.pkg_funcs.get(pkg).and_then(|m| m.get(name)) {
            Some(f) => f.clone(),
            None => {
                self.error(
                    MsgCode::ElabUnresolvedName,
                    &format!("package `{pkg}` has no function `{name}`"),
                );
                return self.placeholder_expr();
            }
        };
        // G4: same loud guard as the module-function path (`inline_function`) — a
        // `string` return needs frame heap String-slot storage the engine does not yet
        // provide (a frame return slot is a bit-vector; a String return reads back empty
        // = silent-wrong). Reject the CALL rather than silently return an empty string.
        if func.ret_string {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "function `{pkg}::{name}`: a `string` return type is not yet supported \
                     (return the text via a `string` output/ref formal instead)"
                ),
            );
            return self.placeholder_expr();
        }
        // Round-9 PKG2: same-package constants (enum labels + localparams) are
        // readable in the body — the frame-body lowering injects them under the
        // `$func$pkg::name` scope. `pkg_consts[pkg]` already holds params +
        // localparams + enum labels (see `elaborate_package`).
        let pkg_const_names: std::collections::BTreeSet<String> = self
            .pkg_consts
            .get(pkg)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        if !pkg_func_self_contained(&func, &pkg_const_names) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "package-scoped call `{pkg}::{name}(...)` is supported only for a \
                     self-contained, straight-line function (its body must reference only \
                     its own formals/locals and same-package constants, and have no control \
                     flow); `import {pkg}::*` and call `{name}` by its bare name instead",
                ),
            );
            return self.placeholder_expr();
        }
        // Route through the FRAME path, NOT the inline fold. The inline fold
        // (`fold_straight_line`) evaluates the return expression in its SELF-determined
        // operand width and only THEN resizes to the return type — silently dropping an
        // overflowing top-level operator's carry/high bits (8-bit `a + b` bound to a
        // 16-bit return keeps only 8 bits, `255+255` → 254 not 510). The frame path
        // lowers the body to real return-slot assignments, so the ENGINE applies the
        // IEEE §11.6.1 assignment-context width — identical to calling the same function
        // by its bare imported name, which `build_frame_set` already frames. Reserve +
        // lower the frame on first use, keyed as `pkg::name` (distinct from any
        // module-local bare name), then divert the call to it. A self-contained function
        // makes no other user call, so there is no mutual recursion → reserving AND
        // lowering it on demand (rather than at the step-6.5 barrier) is safe.
        let key = format!("{pkg}::{name}");
        let fid = match self.frame_idx.get(&key) {
            Some(&fid) => fid,
            None => {
                self.reserve_frame_func(&key, &func);
                let fid = self.frame_idx[&key];
                self.lower_frame_func_body(&key, &func, fid);
                fid
            }
        };
        self.emit_frame_call(fid, &func, args)
    }

    pub(crate) fn reduce_function_body(
        &mut self,
        func: &ast::FunctionDef,
        inputs: &[ast::TfPort],
        actual_ids: &[u32],
    ) -> u32 {
        // A multi-dim packed local element-select (`p[k]`) folds as a BIT-select on
        // the inline subst value (which carries no packed dims) — silently wrong.
        // A SELF-CONTAINED such function was routed to the (correct) frame path by
        // `build_frame_set`, so reaching the inline path here means the body reads a
        // NON-local (not routable without dropping that read from an `always_comb`
        // sensitivity list) — loud-reject rather than fold the wrong element.
        if body_selects_mdpacked_local(func) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "function `{}`: a multi-dim packed local element-select in a \
                     non-`automatic` function that also reads a non-local signal is \
                     unsupported (declare the function `automatic`)",
                    func.name.name
                ),
            );
            return self.placeholder_expr();
        }
        let frame_base = self.subst.len();
        let fs_base = self.formal_str.len();
        // (a) bind each input formal NAME → actual ExprId. §13.5.3/§11.6.2: an actual
        // is ASSIGNED to its formal, so resize it to the formal's WIDTH — a narrow
        // actual zero/sign-extends to the port width (not left with X in the high bits),
        // a wide actual truncates. The extension direction is the ACTUAL's own sign
        // (§11.6.1), and the formal's declared SIGNEDNESS is deliberately NOT re-stamped
        // onto the value here: stamping it is IEEE-correct in isolation, but combined
        // with the inline path's pre-existing SELF-width (not context-width) evaluation
        // of body sub-expressions it can flip a signed-context overflow to the wrong
        // sign (silent-wrong). Applying the formal signedness to body arithmetic is a
        // separate deferred gap (needs context-width propagation into the inline SSA).
        for (p, &eid) in inputs.iter().zip(actual_ids) {
            // R2: a dyn-array formal is aliased via `dyn_subst` (read paths), NOT bound
            // in `subst` — so a stray WHOLE-array read of `b` misses `subst` and falls
            // to its normal (loud) resolution instead of the placeholder ExprId.
            if self.is_input_dyn_array_formal(p) {
                continue;
            }
            let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
            let (w, _, _, _) = self.range_to_dims(kind, p.range.as_ref(), p.signed);
            // A heap-HANDLE formal (string/class/event) or any width-0 type is NOT a
            // bit-vector — bind it verbatim; a bit-resize would corrupt the handle (the
            // NetKind discriminator must precede the width-based resize). `resize_inline_
            // assign` already guards real internally.
            let is_handle = matches!(
                kind,
                ast::NetVarKind::String | ast::NetVarKind::ClassHandle | ast::NetVarKind::Event
            );
            // Only EXTEND a NARROW actual (the ㊀ bug: its high bits were X). A wide or
            // exact actual is bound VERBATIM — TRUNCATING it here would, combined with
            // the inline path's deferred self-width body evaluation, flip a wider-context
            // shift (`f = c >> 1` with a wide actual) to the wrong value vs the pre-change
            // baseline; the IEEE assignment truncation is instead left to the body's own
            // context (a part-select / the return assignment re-truncates). This keeps
            // the change strictly additive over the prior behavior for wide/exact actuals.
            let rw = self.ir_bits_of(eid).unwrap_or(w);
            let bound = if is_handle || w == 0 || rw >= w {
                eid
            } else {
                // Extend by the actual's OWN sign (§11.6.1); no formal-sign re-stamp.
                let actual_signed = self.expr_self_signed(eid);
                self.resize_inline_assign(eid, w, actual_signed)
            };
            self.subst.push((p.name.name.clone(), bound));
            self.formal_str
                .push((p.name.name.clone(), matches!(kind, ast::NetVarKind::String)));
        }
        // §11.6 + §10.7: width/sign context for each body assignment — the return
        // var (return-type width/sign) and each declared body/block local. Used to
        // size fill literals AND to resize the assigned value to the LHS-declared
        // (width, sign), which the inline SSA path otherwise misses (see
        // `resize_inline_assign`).
        let (ret_w, ret_signed) = self.func_return_dims(func);
        // Sibling-block same-name string/handle collision (INLINE path): two
        // block-local decls sharing a NAME but differing in string/handle-ness
        // flatten into one name-keyed `formal_str`/`subst` binding (innermost-wins),
        // so one block's local silently adopts the other's classification (its
        // `s[i]` folds as a byte where the other wants a bit, or vice versa). v1 has
        // no per-block-scope inline namespace — loud-reject (rename one) rather than
        // miscompute. Only BLOCK locals are compared (a block local shadowing a
        // function-TOP-level local is separately caught by the block-scope-leak
        // E3009). Packed-WIDTH-only collisions are a separate deferred axis;
        // identical-typed reuse (`for` temps etc.) is safe and unaffected.
        {
            let mut block_locals: Vec<ast::NetVarDecl> = Vec::new();
            collect_block_local_decls(&func.body, &mut block_locals);
            let mut seen: BTreeMap<String, bool> = BTreeMap::new();
            for d in &block_locals {
                let is_str = matches!(d.kind, ast::NetVarKind::String);
                for n in &d.names {
                    match seen.get(&n.name.name) {
                        Some(&prev) if prev != is_str => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "block-local `{}` in function `{}` is declared \
                                     `string` in one block and non-`string` in another \
                                     — vita flattens inline block-locals to one binding \
                                     (no per-block scope); rename one",
                                    n.name.name, func.name.name
                                ),
                            );
                            return self.placeholder_expr();
                        }
                        _ => {
                            seen.insert(n.name.name.clone(), is_str);
                        }
                    }
                }
            }
        }
        let mut local_dims: BTreeMap<String, (u32, bool)> = BTreeMap::new();
        let mut decls = func.body_decls.clone();
        collect_block_local_decls(&func.body, &mut decls);
        for d in &decls {
            // v7 P2-C: a `string`/handle (`class`/`event`) LOCAL must NOT be bit-resized
            // — omitting it from `local_dims` makes `fold_straight_line` take its
            // `ctx_w == 0` path (`lower_expr` + no `resize_inline_assign`), preserving the
            // heap value (a resize to the 1-bit `range_to_dims(String)` width corrupts it,
            // so `s < t` on two string locals compared as 1-bit packed = silent-wrong).
            let is_handle = matches!(
                d.kind,
                ast::NetVarKind::String | ast::NetVarKind::ClassHandle | ast::NetVarKind::Event
            );
            let (mut w, _, _, signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
            // A multi-dim PACKED local (`logic [1:0][7:0] p`) has its FULL flat width
            // = product of all packed dims; `range_to_dims` returns only the OUTER dim
            // (`[1:0]` ⇒ 2), so a whole-value assign (`p = 16'hABCD`) would truncate to
            // 2 bits (silent-wrong). Use the full packed width. (An element-select of
            // such a local is separately routed to the frame path / loud-rejected.)
            if let Some((pw, _, _)) = self.frame_packed_width(d) {
                w = pw;
            }
            for n in &d.names {
                if !is_handle {
                    local_dims.insert(n.name.name.clone(), (w, signed));
                }
                // Record each local's declared `string`-ness (innermost-wins, so a local
                // shadowing a formal resolves to the local) so a `string` relational
                // compare (`s < t`) on two string LOCALS routes to `StrCmp` (§4.5.81).
                self.formal_str.push((
                    n.name.name.clone(),
                    matches!(d.kind, ast::NetVarKind::String),
                ));
            }
        }
        // (b) walk the straight-line body, recording the return-var assignment.
        let fname = func.name.name.clone();
        let mut ret: Option<u32> = None;
        // Gap B: body-local enum labels → constants under the caller prefix, bounded
        // to this reduction (restored below) so a body `= A` folds without leaking.
        let (saved_labels, saved_meta) = self.push_body_enum_labels(&func.body_enums, &func.body);
        let ok =
            self.fold_straight_line(&func.body, &fname, ret_w, ret_signed, &local_dims, &mut ret);
        self.restore_params(saved_labels);
        self.restore_param_meta(saved_meta);
        // restore the substitution stack to its pre-call depth.
        self.subst.truncate(frame_base);
        self.formal_str.truncate(fs_base);

        if !ok {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "function `{fname}` body is not reducible to an expression (control flow)"
                ),
            );
            return self.placeholder_expr();
        }
        match ret {
            Some(eid) => eid,
            None => {
                // body never assigned the return var → X (the function's default).
                self.warn(&format!(
                    "function `{fname}` never assigns its return value; X approximated"
                ));
                self.placeholder_expr()
            }
        }
    }

    /// Fold a straight-line function body. Returns false (caller emits the error)
    /// on the first non-foldable construct. Each `local = expr;` pushes a
    /// substitution binding (SSA-by-substitution); `fname = expr;` records the
    /// return ExprId. Lowering happens with the CURRENT substitution scope active.
    pub(crate) fn fold_straight_line(
        &mut self,
        s: &ast::Stmt,
        fname: &str,
        ret_w: u32,
        ret_signed: bool,
        local_dims: &BTreeMap<String, (u32, bool)>,
        ret: &mut Option<u32>,
    ) -> bool {
        match s {
            ast::Stmt::Null(_) => true,
            ast::Stmt::Block { stmts, .. } => {
                // begin-end local decls need NO nets: each local lives only as a
                // substitution binding (combinational). Fold each stmt in order.
                stmts.iter().all(|st| {
                    self.fold_straight_line(st, fname, ret_w, ret_signed, local_dims, ret)
                })
            }
            ast::Stmt::Blocking {
                lhs, delay, rhs, ..
            } => {
                if delay.is_some() {
                    self.warn("intra-assignment delay in inlined function dropped");
                }
                // LHS must be a bare single-segment Ident (a local var or func name).
                let ast::Lvalue::Ident(p) = lhs else {
                    return false;
                };
                if p.segments.len() != 1 {
                    return false;
                }
                let target = p.segments[0].name.clone();
                // §11.6: a fill rhs is sized to the LHS width — the return-type width
                // for `fname = …`, the declared width for a body/block local (non-
                // fill ⇒ byte-identical via lower_expr).
                let (ctx_w, ctx_signed) = if target == fname {
                    (ret_w, ret_signed)
                } else {
                    local_dims.get(&target).copied().unwrap_or((0, false))
                };
                let rhs_id0 = if ctx_w == 0 {
                    self.lower_expr(rhs)
                } else {
                    self.lower_ctx_or_plain(rhs, ctx_w)
                };
                // §10.7: apply the LHS-declared width/sign the inline SSA path
                // otherwise misses (no net write). ctx_w==0 = unknown/implicit
                // width ⇒ leave untouched (byte-identical). A real-valued rhs —
                // including a call to a real-returning FRAME function, whose
                // `Expr::Call` node carries no real flag so the helper's IR-level
                // `expr_is_real` cannot see it — must NOT be bit-resized/sign-
                // stamped; guard with the AST-aware check `lower_prim_cast` uses.
                let rhs_id = if ctx_w == 0 || self.cast_operand_is_real(rhs, rhs_id0) {
                    rhs_id0
                } else {
                    self.resize_inline_assign(rhs_id0, ctx_w, ctx_signed)
                };
                if target == fname {
                    *ret = Some(rhs_id); // return assignment
                } else {
                    self.subst.push((target, rhs_id)); // local: innermost-wins binding
                }
                true
            }
            // R2: an explicit `return e;` in a straight-line inline body — same as the
            // `fname = e` return assignment (size the value to the return width/sign).
            // Behavior-preserving for existing code: every Return-bodied function is
            // pre-framed by `body_needs_frame`, so no function reaching the inline fold
            // today contains a `Return` — this arm is exercised only by the new R2
            // read-only-dyn carve-out.
            ast::Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    let rhs_id0 = if ret_w == 0 {
                        self.lower_expr(e)
                    } else {
                        self.lower_ctx_or_plain(e, ret_w)
                    };
                    let rhs_id = if ret_w == 0 || self.cast_operand_is_real(e, rhs_id0) {
                        rhs_id0
                    } else {
                        self.resize_inline_assign(rhs_id0, ret_w, ret_signed)
                    };
                    *ret = Some(rhs_id);
                }
                true
            }
            // if/case/loop/nonblocking/task-call/etc. ⇒ not reducible to one expr.
            _ => false,
        }
    }

    /// R2: can `f` be routed to the INLINE path with a read-only dyn-array-input alias
    /// (bypassing the frame forces)? Requires ≥1 `input` dyn-array formal, ALL formals
    /// `input`, and a STRAIGHT-LINE body (Null/Block/Blocking/Return only) that never
    /// WRITES a dyn-array formal. A recursive such function is caught by the inline
    /// path's `inline_stack` (loud), so no recursion check is needed here. Any function
    /// outside this exact shape stays FRAMED → loud at `classify_array_formal`
    /// (correct-or-loud).
    pub(crate) fn func_r2_inlinable(&self, f: &ast::FunctionDef) -> bool {
        f.ports.iter().any(|p| self.is_input_dyn_array_formal(p))
            && f.ports.iter().all(|p| matches!(p.dir, ast::PortDir::Input))
            && self.body_readonly_dyn_ok(&f.body, f)
    }
}
