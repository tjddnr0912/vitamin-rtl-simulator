//! parameter binding / overrides — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    pub(crate) fn param_decl_width(&self, p: &ast::ParamDecl) -> Option<(u32, bool)> {
        if matches!(p.ty, ast::ParamType::Real | ast::ParamType::Realtime) {
            return None;
        }
        if let Some(r) = &p.range {
            match (
                self.const_eval_in_scope(&r.msb),
                self.const_eval_in_scope(&r.lsb),
            ) {
                (Some(m), Some(l)) => Some((m.abs_diff(l) as u32 + 1, p.signed)),
                _ => None, // unfoldable bound: leave it value-inferred (loud elsewhere)
            }
        } else if matches!(p.ty, ast::ParamType::Integer) {
            // `int`/`integer` are 32-bit; signedness comes from the decl (signed by
            // default, `int unsigned` / `unsigned integer` flip it — the parser sets
            // `p.signed` accordingly).
            Some((32, p.signed))
        } else {
            // Untyped/implicit param — IEEE §6.20.2: the type follows the VALUE.
            // A LITERAL initializer carries its own `(width, signedness)`, which
            // `parse_int_literal` computes exactly: a SIZED literal (`8'hAB`) its
            // declared width; a plain DECIMAL (`42`, `3000000000`) a SIGNED width
            // grown to hold value+sign (§3.5.1 — 32, or 33 for ≥2^31); an
            // UNSIZED-BASED literal (`'hFF` unsigned, `'shFF` signed) its base's
            // sign. Without this an untyped decimal fell through to the
            // value-inferred fallback, which makes a NON-NEGATIVE value UNSIGNED
            // (`const_u32_expr`) — so `localparam A=-1, B=2; A < B` compared
            // UNSIGNED (B unsigned) instead of signed (both signed decimals).
            // The SIZED case applies to any reaching param-type (unchanged); the
            // value-determined DECIMAL/UNSIZED-BASED case is restricted to a
            // genuinely untyped (`Implicit`) param — a `time` param's width is its
            // declared 64-bit type, not the literal's. A non-literal EXPRESSION
            // initializer keeps the value-inferred width — None.
            // Peel `(...)` and a leading unary `-`/`+` to reach the inner literal —
            // that tells us the SIGNEDNESS (a `-`/`+` preserves the operand's sign).
            let mut e = &p.value;
            loop {
                match &e.kind {
                    ast::ExprKind::Paren { inner } => e = inner,
                    ast::ExprKind::Unary {
                        op: ast::UnOp::Minus | ast::UnOp::Plus,
                        operand,
                    } => e = operand,
                    _ => break,
                }
            }
            if let ast::ExprKind::IntLit { kind, raw } = &e.kind {
                let value_determined = matches!(kind, ast::IntLitKind::Sized)
                    || matches!(p.ty, ast::ParamType::Implicit);
                if value_determined {
                    let cv = literal::parse_int_literal(raw, *kind)?;
                    // A plain unsized DECIMAL is signed and sized to its FOLDED
                    // value's minimal signed width (≥32) — NOT the magnitude
                    // literal's, because `-2^k` needs one fewer bit than `+2^k`
                    // (`-2^31` is 32-bit, `+2^31` 33-bit). SIZED / UNSIZED-BASED
                    // literals carry an explicit width — keep parse_int_literal's.
                    if matches!(kind, ast::IntLitKind::Decimal) {
                        if let Some(v) = self.const_eval_in_scope(&p.value) {
                            return Some((min_signed_bits(v).max(32), true));
                        }
                    }
                    return Some((cv.width, cv.signed));
                }
            }
            // The value-determined ident/expression cases below apply ONLY to a
            // genuinely untyped (`Implicit`) param — a `time` param keeps its
            // declared 64-bit UNSIGNED type, so a bare `time C = D;` must NOT
            // inherit D's sign/width through the ident path (the ident-inherit and
            // the expression path share this one `Implicit` gate).
            if matches!(p.ty, ast::ParamType::Implicit) {
                // A bare in-scope param reference (`localparam C = D;`, or a unary
                // `-D`/`+D`/`~D` peeled above) inherits that param's full `(width,
                // signed)` — so an alias of a narrow/typed param keeps its width.
                // On a MISS (the source has no recorded meta — e.g. a `time` param,
                // or an unfoldable-width source), fall to the value-inferred default
                // (`None`) rather than value-sizing the folded i64 below: a bare
                // alias must keep the SOURCE's width, not shrink to its value's.
                if let ast::ExprKind::Ident(pth) = &e.kind {
                    if pth.segments.len() == 1 {
                        return self
                            .param_meta
                            .get(&self.fq(&pth.segments[0].name))
                            .copied();
                    }
                }
                // Same for a bare `pkg::X` alias — inherit the package constant's
                // full `(width, signed)` (MISS → value-inferred, as above).
                if let ast::ExprKind::PkgScoped { pkg, name } = &e.kind {
                    return self
                        .pkg_const_meta
                        .get(&pkg.name)
                        .and_then(|m| m.get(&name.name))
                        .copied();
                }
                // A constant-function CALL is type-determined too: the parameter
                // takes the function's declared RETURN type (§13.4.1), so
                // `localparam X = fb()` with `function byte fb()` is 8 bits signed,
                // not the value-inferred 32 unsigned. Same trio as the cast below.
                if let ast::ExprKind::Call { name, .. } = &p.value.kind {
                    if name.segments.len() == 1 && self.const_eval_in_scope(&p.value).is_some() {
                        if let Some(m) = self
                            .const_func_table
                            .get(&name.segments[0].name)
                            .and_then(|f| self.const_fn_ret_wsign(f))
                        {
                            return Some(m);
                        }
                    }
                }
                // A CAST initializer is type-determined, not value-determined: the
                // casting type states the width outright (§6.24), so `localparam
                // PL = longint'(-1)` is 64 bits and `byte'(-1)` is 8 — the
                // value-inferred `.max(32)` below would call all of them 32, which
                // shows up as a wrong `$bits`, a wrong `%h`, and a concatenation of
                // the wrong LENGTH. This is the third predicate that reads a folded
                // value (after `const_eval_in_scope` and `const_expr_signed`), and
                // all three have to widen together or the trio disagrees.
                if let ast::ExprKind::Cast { target, expr } = &p.value.kind {
                    if self.const_eval_in_scope(&p.value).is_some() {
                        match target {
                            ast::CastTarget::Prim(pr) => {
                                if let Some((w, s, _)) = cast_prim_wsign(*pr) {
                                    return Some((w, s));
                                }
                            }
                            // `N'(e)`: N bits, signedness inherited from the operand.
                            ast::CastTarget::Size(n) => {
                                if let Some(n) = self.const_eval_in_scope(n) {
                                    if let Ok(w) = u32::try_from(n) {
                                        return Some((w, self.const_expr_signed(expr)));
                                    }
                                }
                            }
                            // Not folded by `const_eval_cast`, so unreachable with a
                            // Some value — fall through to value-inference anyway.
                            ast::CastTarget::Signing { .. } | ast::CastTarget::Named(_) => {}
                        }
                    }
                }
                // Any other constant EXPRESSION initializer (`localparam E = 3 + 4;`)
                // is value-determined (§6.20.2): signedness from the expression
                // (§11.8.1), width from the folded value's minimal signed width.
                // Without this a positive such param was UNSIGNED — inconsistent
                // with the same value written as a bare literal.
                if let Some(v) = self.const_eval_in_scope(&p.value) {
                    return Some((min_signed_bits(v).max(32), self.const_expr_signed(&p.value)));
                }
            }
            None // `time` (64-bit) / unfoldable: keep the full i64
        }
    }

    /// The param's DECLARED packed range `(lo, width, ascending)` — recorded in
    /// `param_range` ONLY when the LSB is non-zero (`localparam [15:8] P`), so a
    /// bit/part-select `P[15:12]` normalizes its offset against the declared LSB.
    /// `None` for a bare/atom param, a zero-LSB range (`[N:0]` — raw is already
    /// correct), or an unfoldable bound. Reads the SAME `p.range` as
    /// [`Self::param_decl_width`].
    pub(crate) fn param_decl_range(&self, p: &ast::ParamDecl) -> Option<(u32, u32, bool)> {
        if matches!(p.ty, ast::ParamType::Real | ast::ParamType::Realtime) {
            return None;
        }
        let r = p.range.as_ref()?;
        let m = self.const_eval_in_scope(&r.msb)?;
        let l = self.const_eval_in_scope(&r.lsb)?;
        let lo = m.min(l).max(0) as u32;
        if lo == 0 {
            return None; // zero-LSB `[N:0]`/`[0:N]` — the raw offset is already correct
        }
        Some((lo, m.abs_diff(l) as u32 + 1, m < l))
    }

    /// If `base` is a bare param/localparam Ident with a recorded non-zero-LSB
    /// declared range, return `(lo, width, ascending)` — resolved by the SAME
    /// `walk_scopes` as the param's value (`lookup_scoped`) and meta (`param_meta`),
    /// so the offset range can never drift from the value lookup. Drives the
    /// offset-normalization param arms in `norm_offset_if_net` / `base_net_ascending`
    /// / `norm_offset_ascending`, mirroring a net's `norm_offset_for_net`.
    pub(crate) fn param_sel_range(&self, base: &ast::Expr) -> Option<(u32, u32, bool)> {
        let ast::ExprKind::Ident(path) = &base.kind else {
            return None;
        };
        if path.segments.len() != 1 {
            return None;
        }
        let seg = &path.segments[0].name;
        // A frame-local (inline-function input formal / local, or task output formal)
        // SHADOWS the param — the VALUE resolves to it FIRST, so the offset must not
        // use the param range. Decline when either substitution stack binds the name.
        if self.subst_lookup(seg).is_some() || self.out_subst_lookup(seg).is_some() {
            return None;
        }
        // Re-derive the SAME innermost binding key the VALUE resolves to — a shadowing
        // inner net/local (`symbols`) or generate/zero-LSB param (`params`) must WIN —
        // then use the declared range ONLY if that exact key is a recorded non-zero-LSB
        // param. An independent `walk_scopes(&param_range)` would silently skip a shadow
        // (`param_range` is a SUBSET of `params`, populated only for non-zero-LSB params
        // at a couple of decl sites) and drift outward to an OUTER param, normalizing
        // the offset against the wrong LSB while the value came from the inner binding.
        let key = self.walk_scopes_key(seg, |k| {
            self.params.contains_key(k) || self.symbols.contains_key(k)
        })?;
        self.param_range.get(&key).copied()
    }

    pub(crate) fn coerce_param_value(&mut self, v: i64, p: &ast::ParamDecl) -> i64 {
        // `param_decl_width` already reports the declared signedness (incl. `int`/
        // `integer` via `p.signed`), so coerce with THAT — an `int unsigned` must
        // NOT be force-signed here.
        match self.param_decl_width(p) {
            Some((w, signed)) => coerce_i64_to_width(v, w, signed),
            None => v,
        }
    }

    /// Evaluate a parameter/localparam INITIALIZER to its i64 value, sizing an
    /// unsized fill literal (`'0`/`'1`/`'x`/`'z`) to the DECLARED width (IEEE
    /// §5.7.1 / §11.6 — the fill is context-determined, here by the param type).
    /// Without a fill, this is exactly `const_eval_in_scope`. `'1` into a 64-bit
    /// param therefore yields all-64-ones, not the 32-bit `0xFFFFFFFF`.
    pub(crate) fn eval_param_init(&self, e: &ast::Expr, width: Option<u32>) -> Option<i64> {
        if let (Some(w), Some((kind, raw))) = (width, expr_as_fill(e)) {
            return fill_to_i64(kind, raw, w);
        }
        self.const_eval_in_scope(e)
    }

    pub(crate) fn bind_params(
        &mut self,
        module: &ast::ModuleDecl,
        overrides: &[ResolvedOverride],
    ) -> Vec<(String, Option<i64>)> {
        // Build name→value from the resolved overrides. Positional binds to the
        // i-th declaration index (matches module.params order). A fill-literal
        // override (`#(.P('1))`) is carried as `(kind, raw)` and re-folded at the
        // CHILD param's declared width below.
        let mut ovr_by_name: BTreeMap<&str, Option<i64>> = BTreeMap::new();
        let mut ovr_fill: BTreeMap<&str, &(ast::IntLitKind, String)> = BTreeMap::new();
        let mut ovr_unfoldable: std::collections::BTreeSet<String> = Default::default();
        let mut pos_i = 0usize;
        for ov in overrides {
            if ov.is_named {
                let Some(n) = ov.name.as_deref() else {
                    continue;
                };
                // Fix 2 (mirror): a named override naming no real param is an error.
                match module.params.iter().find(|p| p.name.name == n) {
                    Some(p) => {
                        if let Some(v) = ov.value {
                            ovr_by_name.insert(p.name.name.as_str(), Some(v));
                        } else if ov.fill.is_none() && ov.had_value {
                            // r19: the override was WRITTEN but did not fold (a real
                            // expression, a signal, …). `ovr_by_name` only holds folded
                            // values, so record the attempt — a REAL-typed target must
                            // reject rather than silently run with its declared default.
                            ovr_unfoldable.insert(p.name.name.clone());
                        }
                        if let Some(f) = &ov.fill {
                            ovr_fill.insert(p.name.name.as_str(), f);
                        }
                        // `.W()` with no value ⇒ keep default (no insert).
                    }
                    None => {
                        self.error(
                            MsgCode::ElabPortMismatch,
                            &format!("override of unknown parameter `{n}`"),
                        );
                    }
                }
            } else {
                match module.params.get(pos_i) {
                    Some(p) => {
                        ovr_by_name.insert(p.name.name.as_str(), ov.value);
                        if let Some(f) = &ov.fill {
                            ovr_fill.insert(p.name.name.as_str(), f);
                        }
                    }
                    None => {
                        self.error(
                            MsgCode::ElabPortMismatch,
                            "more positional parameter overrides than module parameters",
                        );
                    }
                }
                pos_i += 1;
            }
        }

        let mut saved = Vec::new();
        for p in &module.params {
            // r19: a REAL-valued header parameter (`#(parameter real R = 1.5)`) has no
            // i64 value — route it to the side map before the numeric fold, exactly as
            // the module-body path does.
            //
            // An OVERRIDE of a real param is NOT supported: the override machinery is
            // i64 throughout, so `#(.R(2.5))` cannot be folded. Falling through to the
            // numeric path there is not safe — it only WARNS (W3056 "override is not a
            // constant; default kept") and runs with the declared default, i.e. the
            // wrong value with exit 0, where before this slice the whole design was
            // loud. Reject explicitly instead (correct-or-loud): a parameter bound to
            // the wrong value poisons everything downstream with no trace.
            if let Some((v, exact)) = self.param_real_value(&p.ty, &p.value) {
                let key = self.fq(&p.name.name);
                // An override that FOLDED to an i64 applies exactly — `#(.R(i+2))` on a
                // real formal is legal and iverilog answers it. Rejecting it took a
                // byte-correct design loud. Only an override that was WRITTEN and did
                // not fold (a real expression, a signal) is still unsupported, because
                // the override machinery is i64 throughout and falling through would
                // merely WARN and run with the declared default = the wrong value at
                // exit 0.
                match ovr_by_name.get(p.name.name.as_str()).copied() {
                    Some(Some(ov)) => {
                        self.real_param_val.insert(key.clone(), ov as f64);
                        self.params.insert(key, ov);
                        continue;
                    }
                    Some(None) => self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "the override of real parameter `{}` is not a constant (a \
                             real override cannot be folded) — the declared default \
                             would be used silently",
                            p.name.name
                        ),
                    ),
                    None if ovr_unfoldable.contains(&p.name.name) => self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "the override of real parameter `{}` is not a constant (a \
                             real override cannot be folded) — the declared default \
                             would be used silently",
                            p.name.name
                        ),
                    ),
                    None => {}
                }
                // Bind the declared default anyway: on the error paths the run already
                // fails, and leaving the name unbound raised a second, misleading
                // "undeclared net/variable" at every downstream read.
                self.real_param_val.insert(key.clone(), v);
                if let Some(i) = exact {
                    self.params.insert(key, i);
                }
                continue;
            }
            let meta = self.param_decl_width(p);
            let pw = meta.map(|(w, _)| w);
            // A fill-literal override re-folds at THIS param's declared width.
            let ovr_fill_v = ovr_fill
                .get(p.name.name.as_str())
                .and_then(|(k, raw)| pw.and_then(|w| fill_to_i64(*k, raw, w)));
            let chosen_val: Option<i64> = match ovr_by_name.get(p.name.name.as_str()) {
                // override present + param is overridable → use it (None = fold-fail
                // → fall back to the declared default). A fill override wins.
                Some(ovr) if matches!(p.kind, ast::ParamKind::Parameter) => ovr_fill_v
                    .or(*ovr)
                    .or_else(|| self.eval_param_init(&p.value, pw)),
                // override targeting a localparam → error, keep declared value.
                Some(_) => {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        &format!("cannot override localparam `{}`", p.name.name),
                    );
                    self.eval_param_init(&p.value, pw)
                }
                None => self.eval_param_init(&p.value, pw),
            };
            // Unfoldable param value = LOUD error, never a silent 0 (P0-5);
            // 0 is only the post-error recovery value.
            let v = chosen_val.unwrap_or_else(|| {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "parameter `{}` value is not a foldable constant expression",
                        p.name.name
                    ),
                );
                0
            });
            let v = self.coerce_param_value(v, p);
            let key = self.fq(&p.name.name);
            // Persistent copy for hierarchical reads (`dut.WIDTH`) — `self.params`
            // is restored after the instance, so the read side needs this.
            self.hier_params.insert(key.clone(), v);
            if let Some(m) = meta {
                self.param_meta.insert(key.clone(), m);
            }
            if let Some(r) = self.param_decl_range(p) {
                self.param_range.insert(key.clone(), r);
            }
            saved.push((key.clone(), self.params.insert(key, v)));
        }
        saved
    }

    /// Restore the param map to the snapshot taken before this instance bound its
    /// params (so sibling instances of the same module re-bind cleanly).
    pub(crate) fn restore_params(&mut self, saved: Vec<(String, Option<i64>)>) {
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
    }

    /// §4.5.158: twin of `restore_params` for the `param_meta` side-map — unwinds a
    /// scoped body-local enum-label width/sign registration so it does not pollute
    /// later scopes (`param_meta` is otherwise persistent).
    pub(crate) fn restore_param_meta(&mut self, saved: Vec<(String, Option<(u32, bool)>)>) {
        for (k, prev) in saved.into_iter().rev() {
            match prev {
                Some(v) => {
                    self.param_meta.insert(k, v);
                }
                None => {
                    self.param_meta.remove(&k);
                }
            }
        }
    }

    /// GAP-G: resolve the element-value table of a const array `base` used in a
    /// constant-context element read (`base[i]`). Handles, in local-wins order:
    /// (1) a module-local / generate-scope array by bare name (the same scope
    /// walk a bare param Ident takes); (2) a package array named by its bare name
    /// made visible via `import p::*` / `import p::ROT` — resolved through the
    /// var-alias the import machinery bound (`pkg_var_aliases`) to its origin
    /// package; (3) an explicitly package-qualified array `p::ROT`. Any other
    /// base shape (hierarchical, multi-segment, a non-captured array) → None →
    /// the read stays loud at the binding site (correct-or-loud).
    /// True if a (constant) replication-count expression reads a const-array
    /// ELEMENT (`CNT[i]`) anywhere — directly or inside an arithmetic wrapper
    /// (`CNT[0]+1`, `-CNT[0]`, `c ? CNT[0] : 1`). Such an element read is not a
    /// runtime net the engine can fold, so a foldable count containing one must
    /// be materialized as a literal (else it reads 0 → 0-width). Recurses only
    /// the node kinds a constant count uses; `const_array_vals_of_base` gates the
    /// `BitSelect` on a genuine const array (a packed-vector bit-select or a
    /// runtime array read is NOT one → left to the ordinary lowering).
    pub(crate) fn count_reads_const_array_elem(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::BitSelect { base, .. } => self.const_array_vals_of_base(base).is_some(),
            ast::ExprKind::Paren { inner } => self.count_reads_const_array_elem(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_const_array_elem(operand),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_const_array_elem(lhs) || self.count_reads_const_array_elem(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_const_array_elem(cond)
                    || self.count_reads_const_array_elem(then_e)
                    || self.count_reads_const_array_elem(else_e)
            }
            // `$clog2(CNT[i])` etc. — the element read hides inside a system-call
            // arg (`const_eval_in_scope` folds `$clog2`/`$bits`).
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_reads_const_array_elem(a))
            }
            _ => false,
        }
    }

    /// r19: is `name` a REAL parameter with NO exact integer twin? A real param whose
    /// initializer const-folded to an i64 is registered in BOTH `real_param_val` and
    /// `params` — both representations are exact and agree — so it keeps every integral
    /// capability it had before this slice (`logic [R-1:0]`, `generate if (R > 2)`, …)
    /// while `R/2` still divides in the real domain. Only a param with no i64 twin
    /// (`= 1.5`) is non-integral and must go loud in an integral context.
    ///
    /// Resolves over the COMBINED binding set — an independent walk of `real_param_val` alone
    /// would match an OUTER real param even when an inner net / numeric param shadows
    /// it, resolving one name two different ways.
    pub(crate) fn real_param_is_non_integral(&self, name: &str) -> bool {
        let Some(key) = self.walk_scopes_key(name, |k| {
            self.real_param_val.contains_key(k)
                || self.params.contains_key(k)
                || self.symbols.contains_key(k)
        }) else {
            return false;
        };
        self.real_param_val.contains_key(&key) && !self.params.contains_key(&key)
    }

    /// r19: does `e` read a REAL-valued parameter? A real param is deliberately kept
    /// out of `params` (it has no i64 value), so `const_eval_in_scope` returns None
    /// for it and a constant-required context that lacks its own loud gate silently
    /// folded to 0 — `{int'(R){1'b1}}` printed `0` instead of `11`. The loud twin of
    /// the array-element / runtime-net count detectors, same recursive shape.
    /// r19/B2: does `name` lower to a REAL value? `lower_expr`'s Ident arm prefers
    /// `real_param_val` over `params`, so a real param WITH an exact i64 twin still
    /// lowers to a real `Const`. A consumer that goes through `lower_expr` must ask
    /// this, not `real_param_is_non_integral` — that one models the const-FOLD
    /// resolver (`params`), and a `parameter real R = 4;` answers the two questions
    /// differently. Asking the wrong one let a real count reach `ir::Expr::Replicate`
    /// and emit 2^24 bits at exit 0. Same predicate, two resolvers: pick by consumer.
    pub(crate) fn real_param_lowers_real(&self, name: &str) -> bool {
        self.walk_scopes_key(name, |k| {
            self.real_param_val.contains_key(k)
                || self.params.contains_key(k)
                || self.symbols.contains_key(k)
        })
        .is_some_and(|k| self.real_param_val.contains_key(&k))
    }

    /// r19: the `lower_expr`-resolver twin of [`Self::count_reads_real_param`], for
    /// consumers that lower their operand rather than const-folding it.
    pub(crate) fn count_lowers_real_param(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.real_param_lowers_real(&p.segments[0].name)
            }
            ast::ExprKind::Call { args, .. } => {
                args.iter().any(|a| self.count_lowers_real_param(a))
            }
            ast::ExprKind::Paren { inner } => self.count_lowers_real_param(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_lowers_real_param(operand),
            ast::ExprKind::Cast { expr, .. } => self.count_lowers_real_param(expr),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_lowers_real_param(lhs) || self.count_lowers_real_param(rhs)
            }
            // These two MUST be mirrored rather than delegated: the `_` fallback below
            // reaches `count_reads_real_param`, i.e. back to the const-FOLD resolver,
            // which answers `false` for a real param that has an exact i64 twin. That
            // is how `{$clog2(R){1'b1}}` with `parameter real R = 4;` still folded to a
            // silent 0 — one syntactic layer was enough to re-enter the wrong resolver.
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_lowers_real_param(cond)
                    || self.count_lowers_real_param(then_e)
                    || self.count_lowers_real_param(else_e)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_lowers_real_param(a))
            }
            _ => self.count_reads_real_param(e),
        }
    }

    pub(crate) fn count_reads_real_param(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.real_param_is_non_integral(&p.segments[0].name)
            }
            // A const-FUNCTION call is the hole the bound guard was meant to be the only
            // net for: neither this walk nor `nonconst_bound_reason` descended into call
            // args, so `logic [f(R)-1:0]` folded to None and `clamp_bound_u32` silently
            // gave width 1 on a design iverilog answers.
            ast::ExprKind::Call { args, .. } => args.iter().any(|a| self.count_reads_real_param(a)),
            ast::ExprKind::Paren { inner } => self.count_reads_real_param(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_real_param(operand),
            ast::ExprKind::Cast { expr, .. } => self.count_reads_real_param(expr),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_real_param(lhs) || self.count_reads_real_param(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_real_param(cond)
                    || self.count_reads_real_param(then_e)
                    || self.count_reads_real_param(else_e)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_reads_real_param(a))
            }
            _ => false,
        }
    }

    /// True if a replication-count expression reads an UNPACKED-ARRAY element of
    /// ANY shape — including shapes `const_array_vals_of_base` cannot fold
    /// (descending, non-zero-based, multi-dimensional) and a RUNTIME array. Uses
    /// the array net directly (`net_is_static_array`), so it is the loud-gate
    /// twin of [`Self::count_reads_const_array_elem`]: a count that reads such an
    /// element but does NOT const-fold is an invalid/unsupported constant count
    /// and must be LOUD (the engine would otherwise read 0 → silent 0-width),
    /// mirroring the loud `localparam R = ROT[i]` binding site. A scalar
    /// (packed-vector) net has `array_len == 1` → NOT flagged, so a packed
    /// bit/part-select count is left to the ordinary lowering (byte-identical).
    pub(crate) fn count_reads_array_param_elem(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::BitSelect { base, .. } => {
                self.base_is_array_net(base) || self.count_reads_array_param_elem(base)
            }
            ast::ExprKind::Paren { inner } => self.count_reads_array_param_elem(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_array_param_elem(operand),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_array_param_elem(lhs) || self.count_reads_array_param_elem(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_array_param_elem(cond)
                    || self.count_reads_array_param_elem(then_e)
                    || self.count_reads_array_param_elem(else_e)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.count_reads_array_param_elem(a))
            }
            _ => false,
        }
    }

    /// Overwrite the deferred placeholder at `eid` (a `Signal`) with a `Const`
    /// folding the i64 hierarchical-param value `v` — same width/sign as
    /// [`Self::const_param_expr`] (byte-identical to how a bare param folds), but
    /// written IN PLACE so the existing arena edge keeps pointing at it.
    pub(crate) fn patch_expr_param_const(&mut self, eid: u32, v: i64) {
        let cv = if let Ok(u) = u32::try_from(v) {
            make_const_u32(u, 32)
        } else if i32::try_from(v).is_ok() {
            make_const_i64(v, 32, true)
        } else {
            make_const_i64(v, 64, v < 0)
        };
        let cid = self.intern_const(cv);
        if let Some(slot) = self.exprs.get_mut(eid as usize) {
            *slot = ir::Expr::Const { val: cid };
        }
    }

    /// Width-aware [`Self::patch_expr_param_const`]: a hierarchical read of a TYPED
    /// param (`dut.W` where `W` is `logic [63:0]`) materializes at its DECLARED
    /// width, mirroring the bare-param [`Self::const_param_expr_w`]. `None` meta
    /// (untyped param / no recorded width) falls back to value-inference.
    pub(crate) fn patch_expr_param_const_w(&mut self, eid: u32, v: i64, meta: Option<(u32, bool)>) {
        let cv = match meta {
            Some((w, signed)) if (1..=64).contains(&w) => make_const_i64(v, w, signed),
            _ => {
                self.patch_expr_param_const(eid, v);
                return;
            }
        };
        let cid = self.intern_const(cv);
        if let Some(slot) = self.exprs.get_mut(eid as usize) {
            *slot = ir::Expr::Const { val: cid };
        }
    }
}
