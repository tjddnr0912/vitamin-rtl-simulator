//! parameter binding / overrides — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// One instance's parameter overrides, already resolved against the module's
/// parameter port list by `resolve_param_overrides` — name-keyed, so the ANSI header
/// loop and the module-BODY loop in `instance.rs` bind from the SAME decision about
/// which override targets which declaration.
///
/// The four maps are not interchangeable and each answers a different question:
/// `by_name` = the override folded to an i64 (`Some(None)` = written but did not
/// fold); `fill` = a `'0`/`'1` literal re-folded at the target's declared width;
/// `text` = a string override (`value` is i64-only, so dropping it ran the child with
/// its default at exit 0); `unfoldable` = written and did not fold, which must be
/// loud rather than a silent fallback to the declared default.
#[derive(Default, Clone)]
pub(crate) struct ParamOverrides {
    pub(crate) by_name: BTreeMap<String, Option<i64>>,
    pub(crate) fill: BTreeMap<String, (ast::IntLitKind, String)>,
    pub(crate) text: BTreeMap<String, String>,
    pub(crate) unfoldable: std::collections::BTreeSet<String>,
}

impl ParamOverrides {
    /// Does any channel target this declaration? The module-body loop asks this to
    /// decide whether a body parameter is bound by `bind_one_param` (override
    /// applied, or a loud rejection owed) or folded by its own decl-order pass.
    ///
    /// All four terms are checked, but `text` can never decide it alone: a named string
    /// override always sets `unfoldable` too (its `value` is None and `had_value` is
    /// true), and the positional arm always sets `by_name`. `fill` IS sole — for a
    /// NAMED `'x`/`'z`, which `bind_one_param` refuses. (Measured, both ways: dropping
    /// `text` from this list changes no answer in the whole suite; dropping `fill`
    /// used to change none either, until that refusal existed.)
    pub(crate) fn targets(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
            || self.fill.contains_key(name)
            || self.text.contains_key(name)
            || self.unfoldable.contains(name)
    }

    /// Drop every channel's entry for `name` before a later override writes its own.
    ///
    /// Last write wins ACROSS channels, not merely within one. The overrides arrive in
    /// one list with `defparam` appended after the instance's `#()` (IEEE §23.10.1 —
    /// a `defparam` supersedes the parameter assignment), and each channel used to
    /// insert independently: `sub #(.K('0)) u(); defparam u.K = 6;` wrote `fill` then
    /// `by_name`, and since `bind_one_param` prefers the declared-width fill re-fold,
    /// the `'0` won and the `defparam` was silently ignored (iverilog: 6, vita: 0).
    ///
    /// `had_value` gates it because `.W()` with no expression is not an override at
    /// all — it legally means "keep the default" and must not erase a real one.
    fn clear_target(&mut self, name: &str, had_value: bool) {
        if !had_value {
            return;
        }
        self.by_name.remove(name);
        self.fill.remove(name);
        self.text.remove(name);
        self.unfoldable.remove(name);
    }
}

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
                            // `RPS'(e)` is the same cast under the other spelling —
                            // `cast_size_bits` owns that rule for every consumer.
                            ast::CastTarget::Size(_) | ast::CastTarget::Named(_) => {
                                if let Some(n) = self.cast_size_bits(target) {
                                    if let Ok(w) = u32::try_from(n) {
                                        return Some((w, self.const_expr_signed(expr)));
                                    }
                                }
                            }
                            // Not folded by `const_eval_cast`, so unreachable with a
                            // Some value — fall through to value-inference anyway.
                            ast::CastTarget::Signing { .. } => {}
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

    /// The module's OVERRIDABLE parameter list — IEEE 1364-2005 §12.2 / IEEE
    /// 1800-2017 §23.2.2.1.
    ///
    /// A module written with an ANSI header (`module m #(parameter W = 8);`) has an
    /// explicit parameter port list, and that list is the whole of it: a `parameter`
    /// declared in the BODY of such a module is not overridable (iverilog agrees —
    /// "Parameter cannot be overridden in the scope it has been declared in"), so
    /// the header list is returned verbatim.
    ///
    /// A module written without one (`module m; parameter W = 8;`, the Verilog-2005
    /// spelling) has an IMPLICIT parameter port list instead: its top-level body
    /// parameter declarations, in declaration order. A `parameter` declared inside a
    /// `generate` block is in a different scope and is not overridable (iverilog:
    /// "parameter `GP` not found") — only `module.body` top level is walked, so
    /// those are never reached.
    ///
    /// `localparam`s are IN this list, so naming one is the precise "cannot override
    /// localparam" and not a misleading "unknown parameter" (iverilog reports the
    /// same). They are not POSITIONAL slots though — see `positional_param_ports`.
    ///
    /// Every override channel resolves names through THIS one list: `#()` named and
    /// positional (`bind_params`), `defparam` (merged into `bind_params` as named
    /// overrides), and `-G` (`cli_overrides_for`). A second spelling would let the
    /// channels disagree about what a module's parameters are. (`defparam` does not
    /// reach an INTERFACE instance at all — it warns and keeps the default, in PRE and
    /// POST alike; that is a separate gap, recorded in ROADMAP §3, not a disagreement
    /// about this list.)
    pub(crate) fn param_ports(module: &ast::ModuleDecl) -> Vec<&ast::ParamDecl> {
        if !module.params.is_empty() {
            return module.params.iter().collect();
        }
        module
            .body
            .iter()
            .filter_map(|it| match it {
                ast::ModuleItem::Param(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// The subset of `param_ports` that a POSITIONAL `#(v1, v2)` override binds to,
    /// in order. A `localparam` is not overridable, so it does not occupy a slot —
    /// `parameter A; localparam L; parameter B;` + `#(10, 30)` binds A=10 and B=30
    /// (oracle: iverilog, for the body list AND for an SV header list that mixes the
    /// two). Counting it made the second value land on the localparam and the design
    /// went loud.
    pub(crate) fn positional_param_ports(module: &ast::ModuleDecl) -> Vec<&ast::ParamDecl> {
        Self::param_ports(module)
            .into_iter()
            .filter(|p| matches!(p.kind, ast::ParamKind::Parameter))
            .collect()
    }

    /// The loud refusal for a parameter whose DECLARED value did not fold into the i64
    /// parameter domain. One spelling for the three folds that reach it — the shared
    /// binder here, `instance.rs`'s module-body copy, and `generate.rs`'s — because the
    /// reason is a property of the domain, not of which copy happened to run.
    ///
    /// An `'x`/`'z` fill gets its own sentence: it IS a foldable constant (iverilog
    /// folds `parameter [7:0] A = 'x;` to `xx`), so calling it unfoldable sends the
    /// reader hunting a syntax problem that is not there. What it lacks is a
    /// REPRESENTATION. A declaration wider than 64 bits takes the `wide_param_bits`
    /// route instead and does carry the x's — which is why the narrow case is the only
    /// one that has to say this.
    pub(crate) fn param_value_unfoldable(&mut self, what: &str, name: &str, value: &ast::Expr) {
        let unknown_fill = const_eval::fill_literal_ast(value)
            .map(|(raw, kind)| (raw.to_string(), kind))
            .filter(|(raw, kind)| literal::fill_is_unknown(raw, *kind));
        let msg = match unknown_fill {
            Some((raw, _)) => format!(
                "{what} `{name}` is declared `{raw}`, which this parameter model cannot \
                 hold — a parameter value has no x/z plane"
            ),
            None => format!("{what} `{name}` value is not a foldable constant expression"),
        };
        self.error(MsgCode::ElabUnsupported, &msg);
    }

    /// Turn `-G NAME=VALUE` into overrides for one top module.
    ///
    /// A value is a decimal integer (`-G W=8`, `-G N=-1`), a Verilog sized literal
    /// (`-G K=8'hFF`), or a quoted string (`-G MODE="lut"`). Anything else is loud —
    /// a CLI override that silently did not apply would be the same failure mode as
    /// the dropped `#(.W(sig))` this slice made loud.
    pub(crate) fn cli_overrides_for(
        &mut self,
        module: &ast::ModuleDecl,
        used: &mut std::collections::BTreeSet<String>,
    ) -> Vec<ResolvedOverride> {
        let mut out = Vec::new();
        let ports = Self::param_ports(module);
        for (name, raw) in self.top_param_overrides.clone() {
            let Some(p) = ports.iter().find(|p| p.name.name == name) else {
                continue; // another root may declare it; reported once by the caller
            };
            used.insert(name.clone());
            if !matches!(p.kind, ast::ParamKind::Parameter) {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!("`-G {name}=…` targets a localparam, which cannot be overridden"),
                );
                continue;
            }
            let t = raw.trim();
            // EVERY fill literal goes to the `fill` channel — the same one `#(.K('1))`
            // uses — because a fill has no width of its own and takes the target's.
            // `parse_int_literal` below would size it here instead, and it gets both
            // halves wrong: it drops the unknown mask, so `-G K='x` ran the child with
            // `K=0` (`'z`: all-ones) at exit 0; and it sizes `'1` to 32 bits, so
            // `-G K='1` installed `0000_0000_ffff_ffff` in a 64-bit parameter while
            // `#(.K('1))` on the same declaration installed all ones. `bind_one_param`
            // is the only place the target's DECLARED width is known — it re-folds
            // `'0`/`'1` there and refuses `'x`/`'z` outright.
            if literal::is_fill_literal(t, ast::IntLitKind::UnsizedBased) {
                let kind = ast::IntLitKind::UnsizedBased;
                // `value` carries the SAME self-determined 32-bit fold that
                // `const_eval_in_scope` gives the `#(.K('1))` spelling, so the two
                // channels present one shape to `bind_one_param` and take the same
                // arms. It is a fallback, not the answer — `fill` above wins wherever
                // a declared width exists. Sending `None` here instead (as the first
                // cut did) emptied `by_name`, and EVERY guard that decides a fill
                // cannot apply reads `by_name`: `-G S='1` on a `parameter string`,
                // `-G R='1` on a `real`, and `-G T='1` on a width-less `time` all
                // became silent no-ops at exit 0 — output byte-identical to passing
                // no flag at all, where PRE was loud about the string.
                // An UNKNOWN fill deliberately keeps `value: None`: there is no i64 it
                // could fall back to, and the refusal in `bind_one_param` owns it.
                let value = (!literal::fill_is_unknown(t, kind)).then(|| fill_to_i64(kind, t, 32));
                out.push(ResolvedOverride {
                    name: Some(name),
                    value: value.flatten(),
                    is_named: true,
                    fill: Some((kind, t.to_string())),
                    had_value: true,
                    str: None,
                });
                continue;
            }
            let (value, text) = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
                (None, Some(t[1..t.len() - 1].to_string()))
            } else if let Ok(v) = t.parse::<i64>() {
                (Some(v), None)
            } else if let Some(cv) = crate::literal::parse_int_literal(t, ast::IntLitKind::Sized)
                .and_then(|c| {
                    // Same i64 domain the override channel uses everywhere else: a
                    // value with bits above word 0 does not fit and is loud, not
                    // silently truncated.
                    (!c.bits.val.iter().skip(1).any(|&w| w != 0))
                        .then(|| c.bits.val.first().copied().unwrap_or(0) as i64)
                })
            {
                (Some(cv), None)
            } else {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!(
                        "`-G {name}={raw}`: the value must be a decimal integer, a sized \
                         literal like 8'hFF, or a quoted string"
                    ),
                );
                continue;
            };
            out.push(ResolvedOverride {
                name: Some(name),
                value,
                is_named: true,
                fill: None,
                had_value: true,
                str: text,
            });
        }
        out
    }

    /// Resolve `#()` / `defparam` / `-G` overrides against a module's parameter port
    /// list (`param_ports`) — the name/position half of `bind_params`, split out so
    /// the module-BODY parameter loop in `instance.rs` binds an overridden body
    /// parameter through the SAME resolution and the same `bind_one_param` below.
    /// A second spelling there would let `#(.W(8))` and `module m; parameter W;`
    /// disagree about which override applies.
    pub(crate) fn resolve_param_overrides(
        &mut self,
        module: &ast::ModuleDecl,
        overrides: &[ResolvedOverride],
    ) -> ParamOverrides {
        // Build name→value from the resolved overrides. Positional binds to the
        // i-th overridable declaration. A fill-literal override (`#(.P('1))`) is
        // carried as `(kind, raw)` and re-folded at the CHILD param's declared width.
        let mut o = ParamOverrides::default();
        let mut pos_i = 0usize;
        // IEEE 1364-2005 §12.2: the overridable list is the ANSI header when there is
        // one, and the top-level body parameter declarations when there is not (see
        // `param_ports`). Positional binding skips localparams (`positional_param_ports`).
        let ports = Self::param_ports(module);
        let pos_ports = Self::positional_param_ports(module);
        for ov in overrides {
            if ov.is_named {
                let Some(n) = ov.name.as_deref() else {
                    continue;
                };
                // Fix 2 (mirror): a named override naming no real param is an error.
                match ports.iter().find(|p| p.name.name == n) {
                    Some(p) => {
                        o.clear_target(&p.name.name, ov.had_value);
                        if let Some(v) = ov.value {
                            o.by_name.insert(p.name.name.clone(), Some(v));
                        } else if ov.fill.is_none() && ov.had_value {
                            // r19: the override was WRITTEN but did not fold (a real
                            // expression, a signal, …). `by_name` only holds folded
                            // values, so record the attempt — a REAL-typed target must
                            // reject rather than silently run with its declared default.
                            o.unfoldable.insert(p.name.name.clone());
                        }
                        if let Some(f) = &ov.fill {
                            o.fill.insert(p.name.name.clone(), f.clone());
                        }
                        if let Some(t) = &ov.str {
                            o.text.insert(p.name.name.clone(), t.clone());
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
                match pos_ports.get(pos_i) {
                    Some(p) => {
                        o.clear_target(&p.name.name, ov.had_value);
                        o.by_name.insert(p.name.name.clone(), ov.value);
                        if let Some(f) = &ov.fill {
                            o.fill.insert(p.name.name.clone(), f.clone());
                        }
                        if let Some(t) = &ov.str {
                            o.text.insert(p.name.name.clone(), t.clone());
                        }
                        if ov.value.is_none()
                            && ov.fill.is_none()
                            && ov.str.is_none()
                            && ov.had_value
                        {
                            o.unfoldable.insert(p.name.name.clone());
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
        o
    }

    pub(crate) fn bind_params(
        &mut self,
        module: &ast::ModuleDecl,
        overrides: &[ResolvedOverride],
    ) -> (Vec<(String, Option<i64>)>, ParamOverrides) {
        let ovr = self.resolve_param_overrides(module, overrides);
        let mut saved = Vec::new();
        for p in &module.params {
            self.bind_one_param(p, &ovr, &mut saved);
        }
        // A module with no ANSI header binds its body parameters in `instance.rs`
        // (decl order, after imports and net prescan) and an interface binds its own
        // in `iface_inst.rs`, so an override that targets one is applied THERE —
        // through `bind_one_param`, with this same set.
        (saved, ovr)
    }

    /// Bind ONE parameter declaration: apply an override if one targets it, else fold
    /// its declared default, and register the result in every map a parameter lives in
    /// (`params`, `hier_params`, `param_meta`, `param_range`, or the string / real /
    /// wide side maps). Three callers: `bind_params` for each ANSI header parameter,
    /// the module-body loop in `instance.rs` for an OVERRIDDEN body parameter, and the
    /// interface body loop in `iface_inst.rs` for EVERY interface body parameter.
    pub(crate) fn bind_one_param(
        &mut self,
        p: &ast::ParamDecl,
        ovr: &ParamOverrides,
        saved: &mut Vec<(String, Option<i64>)>,
    ) {
        // A `localparam` is IN `param_ports` on purpose — naming one must give the
        // precise "cannot override localparam" and not a misleading "unknown
        // parameter" (iverilog reports the same). It is not OVERRIDABLE, though, so
        // answer that HERE, once, and bind the declared value with every channel
        // ignored. Every diagnostic below describes what an override would DO (it did
        // not fold; it is a string where a number is wanted; it has no x/z plane) and
        // none of them is the reason a localparam refuses one. Deciding it lower, per
        // channel, is what reported `#(.L(sig))` as "the override of parameter `L` is
        // not a constant" — the wrong noun AND the wrong reason — while iverilog says
        // "Cannot override localparam `L`".
        let empty = ParamOverrides::default();
        let ovr = if matches!(p.kind, ast::ParamKind::Parameter) {
            ovr
        } else {
            if ovr.targets(&p.name.name) {
                self.error(
                    MsgCode::ElabPortMismatch,
                    &format!("cannot override localparam `{}`", p.name.name),
                );
            }
            &empty
        };
        let ovr_by_name = &ovr.by_name;
        let ovr_fill = &ovr.fill;
        let ovr_unfoldable = &ovr.unfoldable;
        let ovr_str = &ovr.text;
        {
            // ★ ORDERING RULE — this block runs for EVERY parameter, before any
            // side-map early `return` below.
            //
            // An override that was WRITTEN but did not fold used to be a warning
            // ("default kept") and the child then ran with the wrong parameter at
            // exit 0. Escalating it is the point of this slice — but the escalation
            // was first placed after the string / wide / real routes, each of which
            // `continue`s, so the very types most likely to carry an unfoldable
            // override skipped it. Two adversarial lenses found that independently:
            // `#(.MODE(sig))` on a `parameter string` kept its default silently, and
            // a NAMED `#(.K(128'h…))` on a >64-bit parameter installed the declared
            // default while the POSITIONAL spelling of the same override was loud.
            //
            // `ovr_by_name` only holds values that folded to i64, so "an override was
            // written and did not fold" is `ovr_unfoldable` (numeric), `ovr_str`
            // covers the string case, and `ovr_fill` the fill case. `.W()` with no
            // expression is untouched: it legally means "keep the default" and never
            // enters `ovr_unfoldable`.
            // An `'x`/`'z` FILL override cannot be carried at all: this channel is i64
            // and has no unknown plane, so there is no width and no declaration type at
            // which it becomes representable. That is why the test is the fill BIT and
            // not `ovr_fill_v.is_none()` — the value-level test only fires on the arm
            // the NAMED spelling happens to take, and the round-1 spelling of this
            // check sat there. `#(.K('x))` was loud while the positional `#('x)` of the
            // same override silently installed `K=0` and `#('z)` installed `K=255`
            // (`fill_to_i64` used to drop the unknown mask; it now declines, so nothing
            // downstream can fold one either). Placed HERE, above the string / real /
            // wide returns, it also covers `#(.K('x))` on a `parameter real`.
            if let Some((k, raw)) = ovr_fill.get(p.name.name.as_str()) {
                if literal::fill_is_unknown(raw, *k) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "the `{raw}` override of parameter `{}` cannot be applied \
                             — a parameter override carries an integer value and has \
                             no x/z plane, so the declared default would be used \
                             silently",
                            p.name.name
                        ),
                    );
                }
            }
            let has_applied_override = ovr_by_name
                .get(p.name.name.as_str())
                .copied()
                .flatten()
                .is_some()
                || ovr_fill.contains_key(p.name.name.as_str())
                || ovr_str.contains_key(p.name.name.as_str());
            if ovr_unfoldable.contains(&p.name.name) && !has_applied_override {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "the override of parameter `{}` is not a constant, so the \
                         declared default would be used instead — that is a different \
                         design, not a smaller one",
                        p.name.name
                    ),
                );
            }
            // A `string` header parameter has no i64 value, so the numeric fold below
            // reports E3009 on its own declared DEFAULT — measured with no override at
            // all. The module-BODY parameter loop has always routed strings to
            // `str_param_raw` before folding (`instance.rs`); the header path never
            // did, which is why `#(parameter string MODE="X")` was unusable. Do the
            // same here, and apply a string OVERRIDE while we are at it: it is carried
            // in `ResolvedOverride::str` because `value` is i64-only and dropping it
            // ran the child with its default at exit 0.
            let str_val = match ovr_str.get(p.name.name.as_str()) {
                Some(t) => Some(t.to_string()),
                None => {
                    // A `string` parameter overridden with a NUMBER: the override
                    // folded, so `ovr_by_name` has it and the escalation above stays
                    // quiet, and then this route installed the declared DEFAULT — a
                    // silently different design. iverilog rejects the assignment.
                    if Self::param_str_literal(&p.value).is_some()
                        && ovr_by_name.contains_key(p.name.name.as_str())
                    {
                        self.error(
                            MsgCode::ElabPortMismatch,
                            &format!(
                                "parameter `{}` is a string, so a numeric override \
                                 cannot be applied to it",
                                p.name.name
                            ),
                        );
                    }
                    Self::param_str_literal(&p.value)
                }
            };
            if let Some(raw) = str_val {
                let key = self.fq(&p.name.name);
                self.str_param_raw.insert(key, raw);
                return;
            }
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
                        // `real_param_val` is the real view; it serves the BARE read
                        // (`lower_expr` prefers it over `params`). A hierarchical read
                        // is not served at all — it is honestly loud, because neither
                        // available representation survives the trip: publishing the
                        // i64 to `hier_params` makes `a.P/2` divide in the INTEGER
                        // domain, and patching the resolved placeholder with a real
                        // constant lands after `lower_cast` has already committed
                        // `int'(a.P)` to the integral path. Both were built and
                        // measured; each swaps one silent-wrong for another. ROADMAP §2.
                        self.real_param_val.insert(key.clone(), ov as f64);
                        // The LOCAL i64 view. ⚠️ Its necessity is NOT established:
                        // removing this insert entirely leaves every integral context
                        // measured (packed range, unpacked dim, `$bits`, genvar bound,
                        // `generate if`, case label, array index, `repeat`, delay, cast,
                        // derived localparam) still correct — `real_param_val` serves
                        // them — and no discriminator was found. It is kept because
                        // "no discriminator" is not "dead"; do not cite it as the
                        // mechanism that makes an exact-integer real usable as a width.
                        saved.push((key.clone(), self.params.insert(key, ov)));
                        return;
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
                    // Twin of the override arm above: the LOCAL integer view, with the
                    // same measured caveat. Nothing here is published for a hierarchical
                    // read — a module/ANSI real parameter is not readable across an
                    // instance boundary at all (the interface fold keeps its own i64
                    // twin; see `iface_inst.rs` for the measured reason).
                    saved.push((key.clone(), self.params.insert(key, i)));
                }
                return;
            }
            let meta = self.param_decl_width(p);
            let pw = meta.map(|(w, _)| w);
            // A fill-literal override re-folds at THIS param's declared width.
            let ovr_fill_v = ovr_fill
                .get(p.name.name.as_str())
                .and_then(|(k, raw)| pw.and_then(|w| fill_to_i64(*k, raw, w)));
            // A fill override re-folded at THIS param's declared width wins over the
            // same override's parent-side fold, and is consulted INDEPENDENTLY of
            // `by_name`: the `-G` channel carries a fill with no i64 beside it, so
            // keying the decision on `by_name` membership — as this did — dropped it
            // and installed the declared default silently. `by_name` holding
            // `Some(None)` (written, did not fold) also falls through to the declared
            // default; the escalation above has already made that loud.
            let chosen_val: Option<i64> = ovr_fill_v
                .or_else(|| ovr_by_name.get(p.name.name.as_str()).copied().flatten())
                .or_else(|| self.eval_param_init(&p.value, pw));
            // Wider than the i64 constant domain — see `wide_param_bits`. Reached
            // ONLY when the numeric fold above already declined, so a wide DECLARATION
            // whose value happens to fit (`localparam logic [255:0] K = 256'h1`) keeps
            // its integer identity and stays usable as a width, a bound and a
            // generate condition. Gating on the declared WIDTH instead took four
            // declaration scopes from correct to loud — the field doc claimed the
            // opposite ("the boundary is the VALUE, not the declared width") and the
            // code was the counterexample.
            if chosen_val.is_none() {
                let wide = meta.and_then(|(w, sg)| self.wide_param_const_in_scope(&p.value, w, sg));
                if let Some(cv) = wide {
                    let key = self.fq(&p.name.name);
                    self.wide_param_bits.insert(key, cv);
                    return;
                }
            }
            // Unfoldable param value = LOUD error, never a silent 0 (P0-5);
            // 0 is only the post-error recovery value.
            let v = chosen_val.unwrap_or_else(|| {
                self.param_value_unfoldable("parameter", &p.name.name, &p.value);
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
}
