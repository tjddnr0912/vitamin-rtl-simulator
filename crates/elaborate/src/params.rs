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
    /// The wide channel — see `ResolvedOverride::bits`. It carries the override's
    /// WIDTH as well as its value, which is what §6.20.2 gives an untyped parameter.
    pub(crate) bits: BTreeMap<String, ir::ConstVal>,
    /// Is the override EXPRESSION signed — see `ResolvedOverride::signed`. Read only
    /// when the value has to be EXTENDED past the 64-bit integer lane, where the sign
    /// of the i64 is not enough to decide (three expressions with the same i64 extend
    /// two different ways).
    pub(crate) signed: BTreeMap<String, bool>,
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
        self.bits.remove(name);
        self.signed.remove(name);
    }
}

impl Elaborator<'_> {
    pub(crate) fn param_decl_width(&self, p: &ast::ParamDecl) -> Option<(u32, bool)> {
        self.param_decl_width_opt(p, false, false)
    }

    /// [`Self::param_decl_width`] for a declaration whose DEFAULT is what binds — no
    /// override reached it.
    ///
    /// Only then may a concatenation initializer supply the width. IEEE §6.20.2 gives
    /// an untyped parameter the range of its FINAL override value, so keying the width
    /// on the declared expression truncates the override: `#(parameter P = {2{8'h1}})`
    /// overridden with `32'hDEADBEEF` came out 16 bits holding `beef`, where both
    /// oracles keep 32 bits and `deadbeef`. A DECLARED TYPE legitimately survives an
    /// override; a self-determined initializer expression does not, because the value
    /// it was determined from has been replaced.
    /// Whether a concatenation's width comes from its operands' OWN stated widths,
    /// with nothing inferred anywhere in the tree.
    ///
    /// Only a SIZED literal qualifies as a leaf. An unsized decimal is sized from its
    /// value, and a NAME is sized from `param_meta`, which is where inferred widths
    /// live — this predicate cannot see which kind a given name got, so it declines
    /// rather than guess. The replication COUNT is deliberately not examined: it
    /// scales the width but contributes none of its own bits.
    fn concat_width_is_declared(e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::IntLit { kind, .. } => matches!(kind, ast::IntLitKind::Sized),
            ast::ExprKind::Paren { inner } => Self::concat_width_is_declared(inner),
            ast::ExprKind::Concat { parts } => parts.iter().all(Self::concat_width_is_declared),
            ast::ExprKind::Replicate { value, .. } => {
                value.iter().all(Self::concat_width_is_declared)
            }
            _ => false,
        }
    }

    pub(crate) fn param_decl_width_unoverridden(&self, p: &ast::ParamDecl) -> Option<(u32, bool)> {
        self.param_decl_width_opt(p, false, true)
    }

    /// [`Self::param_decl_width_unoverridden`] restricted to DECLARED provenance — the
    /// `(width, signed)` twin of what `param_decl_range` records, and the only meta the
    /// wide bit domain may resize against.
    ///
    /// ⚠️ The unrestricted wrapper is NOT interchangeable here even on the decline path
    /// it is reached from. Three of its value-inferring arms are already fenced off by
    /// needing a folded value, but the concatenation arm is not: it sizes a leaf NAME
    /// through `param_meta`, where inferred widths live, so a concatenation could hand
    /// back a width nothing declared. Truncating a folded value to such a width is the
    /// §4.5.363 263-bit-net shape with the sign flipped.
    pub(crate) fn param_decl_width_declared(&self, p: &ast::ParamDecl) -> Option<(u32, bool)> {
        self.param_decl_width_opt(p, true, true)
    }

    /// [`Self::param_decl_width_declared`] for a declaration an OVERRIDE reached — the
    /// declared range / type / sized literal only, with nothing the initializer's own
    /// value could have supplied.
    pub(crate) fn param_decl_width_declared_overridden(
        &self,
        p: &ast::ParamDecl,
    ) -> Option<(u32, bool)> {
        self.param_decl_width_opt(p, true, false)
    }

    /// The string value of a parameter's declared default: its LITERAL first, and only
    /// then the widened constant-domain fold.
    ///
    /// The order and the gate are both load-bearing.
    ///
    /// The literal question comes first so that every shape which bound a string
    /// before this existed still takes byte-identically the same route.
    ///
    /// The gate exists because `str_param_raw` carries **no width**, and the string
    /// route runs BEFORE the width-carrying numeric and wide paths at every consumer.
    /// A declaration that states a width or an integral type therefore loses it:
    /// `localparam [95:0] X = {"A","B"};` came out as 16 bits where both oracles say
    /// 96, and `localparam [95:0] Z = 1 ? "AB" : "CD";` came out 16706 where iverilog
    /// says 0. Those shapes were LOUD before the widening, so folding them into the
    /// width-free side map is loud → silent-wrong — the one move the ladder forbids.
    /// Declining keeps them exactly as loud as they were.
    ///
    /// An untyped, unranged declaration has no width to lose: `localparam Q =
    /// {"A","B"}` measures 16706 at 16 bits on vita *and* on both oracles.
    /// The override text to apply to `p`, or None to leave the declared default.
    ///
    /// A LITERAL override applies as it always has. A FOLDED one applies only when the
    /// child's declaration has no width to lose — see [`Self::param_str_or_folded`]
    /// for why, and `ResolvedOverride::str_is_literal` for what the flag records.
    pub(crate) fn override_text_for(
        p: &ast::ParamDecl,
        ov: &crate::toplevel::ResolvedOverride,
    ) -> Option<String> {
        let t = ov.str.as_ref()?;
        if ov.str_is_literal || (p.range.is_none() && matches!(p.ty, ast::ParamType::Implicit)) {
            Some(t.clone())
        } else {
            None
        }
    }

    /// `overridden` = a NUMERIC override targets this parameter, so the declared
    /// default is not what binds. The folded fallback must stand down then: an untyped
    /// `parameter W = {"A","B"}` is an ordinary numeric parameter whose default happens
    /// to be a string expression, and both oracles apply `#(.W(9))` to it and print 9.
    /// Folding the default into the width-free string map instead makes the override
    /// vanish and the design run at 16706 — a silently different design at exit 0,
    /// which is worse than the false-loud it replaced.
    pub(crate) fn param_str_or_folded(
        &self,
        p: &ast::ParamDecl,
        overridden: bool,
    ) -> Option<String> {
        Self::param_str_literal(&p.value).or_else(|| {
            (!overridden && p.range.is_none() && matches!(p.ty, ast::ParamType::Implicit))
                .then(|| self.const_str_in_scope(&p.value))
                .flatten()
        })
    }

    /// [`Self::param_decl_width`] with an OPT-IN provenance filter.
    ///
    /// `declared_only` = answer only when the width came from a DECLARED RANGE, a
    /// TYPE, or a LITERAL — never from inference over the folded value. Every
    /// existing caller passes a literal `false` and is byte-identical.
    ///
    /// ⚠️ The distinction is not cosmetic, and it was measured the hard way. The
    /// final fallthrough sizes an untyped expression initializer as
    /// `min_signed_bits(v).max(32)`, so `localparam W = ~8'hCB;` records 32 for a
    /// value whose real self width is 8. A consumer that EXTRACTS BITS from that
    /// width invents bits that do not exist: `logic [(W[15:8])+8-1:0] v;` declared a
    /// **263-bit** net where iverilog declares 1. The bound was correct before the
    /// select fold existed, so trusting the inferred width is a correct→silent-wrong
    /// regression, not a residue — which is why the select path opts in here instead
    /// of reading `param_meta` directly.
    ///
    /// The ALIAS arms (`localparam C = D;`, `localparam C = p::D;`) decline under
    /// the flag as well: they inherit the source's recorded meta, and this predicate
    /// cannot see whether THAT width was itself inferred. Fail-closed; the alias of
    /// a declared param is recorded residue, not a wrong answer.
    fn param_decl_width_opt(
        &self,
        p: &ast::ParamDecl,
        declared_only: bool,
        default_binds: bool,
    ) -> Option<(u32, bool)> {
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
                        if declared_only {
                            return None; // inherited meta — provenance unknown here
                        }
                        return self
                            .param_meta
                            .get(&self.fq(&pth.segments[0].name))
                            .copied();
                    }
                }
                // Same for a bare `pkg::X` alias — inherit the package constant's
                // full `(width, signed)` (MISS → value-inferred, as above).
                if let ast::ExprKind::PkgScoped { pkg, name } = &e.kind {
                    if declared_only {
                        return None; // inherited meta — provenance unknown here
                    }
                    return self
                        .pkg_const_meta
                        .get(&pkg.name)
                        .and_then(|m| m.get(&name.name))
                        .copied();
                }
                // §11.4.12 / §11.4.12.1: a concatenation and a replication are
                // SELF-DETERMINED — their width is the sum of their operands' own
                // widths, and they are unsigned. That width is not recoverable from
                // the value (`{2{32'd2}}` is 64 bits wide and 34 bits of magnitude),
                // so without this arm the fallthrough below sized it from the folded
                // i64 as `min_signed_bits(v).max(32)` and recorded 35 where both
                // oracles say 64 — and 32 where they say 4 for `{2{2'd1}}`.
                //
                // It belongs with the type-determined family around it rather than
                // with the value-inferred tail, and it answers under `declared_only`
                // for the same reason a SIZED literal does: the width comes from the
                // operands' own declared widths, never from the value.
                // ⚠️ Under `declared_only` this must prove provenance LEAF BY LEAF,
                // because the resolver it calls cannot: that resolver sizes a NAME from
                // `param_meta` — exactly where value-INFERRED widths are recorded — and
                // guesses `(32, false)` when there is none. Answering unconditionally
                // made a concatenation a laundering wrapper around the very provenance
                // the flag fences off: `localparam W = ~8'hCB; localparam Q = {W};
                // logic [(Q[15:8])+8-1:0] v;` declared a **263-bit** net where iverilog
                // declares 1 — the identical §4.5.363 regression through the concat
                // door.
                //
                // But refusing outright is not the answer either: `S_THREADS[m*32 +:
                // 32]` — how `axi_crossbar` forwards one port's slice of a per-port
                // vector — IS a select over a concatenation, and denying it a width put
                // the whole design back to loud. A concatenation of SIZED LITERALS
                // states its width as plainly as a sized literal does; a leaf that is a
                // name does not, and only that leaf has to decline.
                // Parens only: the loop above also peels unary `+`/`-`, and whether a
                // negated concatenation keeps the operand's width is a separate
                // question that wants its own measurement. Without this, `{2{8'h1}}`
                // recorded 16 and `({2{8'h1}})` recorded 32 — one value, two answers.
                let mut cat = &p.value;
                while let ast::ExprKind::Paren { inner } = &cat.kind {
                    cat = inner;
                }
                if default_binds
                    && (!declared_only || Self::concat_width_is_declared(cat))
                    && matches!(
                        cat.kind,
                        ast::ExprKind::Concat { .. } | ast::ExprKind::Replicate { .. }
                    )
                {
                    if let Some((_, w, _)) = self.const_placement_wide(
                        cat,
                        &std::collections::BTreeMap::new(),
                        &ConstWidths::new(),
                    ) {
                        if w > 0 {
                            return Some((w, false));
                        }
                    }
                }
                // §11.4.11: a CONDITIONAL is not value-determined either — its
                // result is as wide as its WIDER ARM, and both arms then widen to
                // that. Like the concatenation above, that width is not recoverable
                // from the value: `Z ? Z : 64'h0100000000000000` is 64 bits wide and
                // 57 bits of magnitude, so the value-inferred tail recorded 58 where
                // both oracles say 64 — the parameter's top bits then read `x` and
                // `axi_crossbar`'s address decode died on them.
                //
                // ⭐ The RULE is not new here: `const_self_width` has carried the
                // §11.4.11 arm (`max(then, else)`) since the `**` exponent work. What
                // was missing is that this inference never ASKED it — the same shape
                // as the concatenation arm above, one operator over.
                //
                // ⚠️ Declines under `declared_only`, for exactly the reason the
                // value-inferred tail does: `const_self_width` sizes a NAME from
                // `param_meta`, which is where value-INFERRED widths are recorded, so
                // answering there would launder the provenance that flag fences off —
                // the §4.5.363 regression the concatenation arm documents, through a
                // different door.
                if default_binds && !declared_only {
                    let mut tern = &p.value;
                    while let ast::ExprKind::Paren { inner } = &tern.kind {
                        tern = inner;
                    }
                    if matches!(tern.kind, ast::ExprKind::Ternary { .. }) {
                        if let Some(w) = self.const_self_width(tern, &ConstWidths::new()) {
                            if w > 0 {
                                return Some((w, self.const_expr_signed(tern)));
                            }
                        }
                    }
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
                if declared_only {
                    return None; // VALUE-inferred — see the doc on this parameter
                }
                if let Some(v) = self.const_eval_in_scope(&p.value) {
                    return Some((min_signed_bits(v).max(32), self.const_expr_signed(&p.value)));
                }
            }
            None // `time` (64-bit) / unfoldable: keep the full i64
        }
    }

    /// The param's `(lo, width, ascending)` **whenever that shape is known from a
    /// DECLARATION rather than inferred from the value** — recorded in `param_range`
    /// and read back by [`Self::param_sel_range`]. It answers three groups:
    ///
    /// 1. an explicit packed range (`[15:8]`, `[0:31]`, `[31:0]`);
    /// 2. no range but a TYPE- or LITERAL-determined width (`int`, `localparam
    ///    U = 300`, `byte'(-1)`, a constant-function call) → `(0, w, false)`;
    /// 3. nothing else — notably an untyped EXPRESSION initializer, whose recorded
    ///    width is a value inference (see [`Self::param_decl_width_opt`]).
    ///
    /// ⚠️ Group 2 and the descending zero-LSB half of group 1 are OFFSET NO-OPS at
    /// runtime — `norm_offset_for_range(raw, 0, w, false)` returns `raw` unchanged —
    /// so recording them costs nothing on the lowering path and the common shape
    /// stays byte-identical. What they buy is PROVENANCE: this map becomes the one
    /// place that answers "is this param's width a declared fact?", which is exactly
    /// the question the constant-domain select fold has to ask before it extracts
    /// bits. Group 3 declining is what keeps `localparam W = ~8'hCB; W[15:8]` from
    /// inventing a 263-bit net out of a value-inferred 32.
    ///
    /// ⚠️ The ASCENDING case was missing and `lo == 0` swallowed it: `[0:31]` has
    /// LSB 0 like `[31:0]` does, so an ascending param recorded nothing and every
    /// consumer read the RAW offset. `localparam logic [0:31] A = 32'h34; A[26]`
    /// answered 0 where both oracles answer 1 (ascending index 26 is internal bit
    /// `hi − 26` = 5). Direction, not LSB, is what `norm_offset_for_range` needs.
    ///
    /// Reads the SAME `p.range` as [`Self::param_decl_width`].
    /// [`Self::param_decl_range`] told whether this declaration's DEFAULT is what binds.
    ///
    /// ⚠️ The distinction is §6.20.2's, and it is the same one `param_decl_width_opt`
    /// draws: a width taken from a self-determined INITIALIZER (a concatenation of
    /// sized literals) is a declared fact only while that initializer is the value. An
    /// override replaces it, and then the width came from somewhere that no longer
    /// exists. Passing `false` unconditionally — which is what this did — was safe but
    /// blind: it also refused the un-overridden case, so `parameter M_ISSUE =
    /// {M_COUNT{32'd4}}` recorded no range and `M_ISSUE[n*32 +: 32]`, the way every AXI
    /// generator slices a per-port vector, had nothing to select out of.
    pub(crate) fn param_decl_range_opt(
        &self,
        p: &ast::ParamDecl,
        default_binds: bool,
    ) -> Option<(u32, u32, bool)> {
        if matches!(p.ty, ast::ParamType::Real | ast::ParamType::Realtime) {
            return None;
        }
        let Some(r) = p.range.as_ref() else {
            // No declared range: the width is a fact only when a TYPE or a LITERAL
            // states it. `lo = 0`, descending — an offset no-op.
            let (w, _) = self.param_decl_width_opt(p, true, default_binds)?;
            return Some((0, w, false));
        };
        let m = self.const_eval_in_scope(&r.msb)?;
        let l = self.const_eval_in_scope(&r.lsb)?;
        // ⚠️⚠️ A NEGATIVE declared bound (`[3:-2]`, `[-2:3]`) DECLINES. `param_range`'s
        // value type cannot hold a negative `lo`, so the `min(l).max(0)` this used to
        // write recorded a LIE — and while `lo == 0` filtered every such range out the
        // lie never reached a consumer. It does now, and it is not inert: an ascending
        // `[-2:3]` recorded `(0, 6, true)` turned `A[0]`/`A[3]` from the correct 0/0
        // into 1/1 against both oracles = correct→silent-wrong. Declining leaves the
        // pre-existing (separately tracked) negative-bound behaviour exactly as it was.
        if m < 0 || l < 0 {
            return None;
        }
        let lo = m.min(l) as u32;
        Some((lo, m.abs_diff(l) as u32 + 1, m < l))
    }

    /// Bind a constant VALUE at `key`, and CLEAR any declared range recorded for the
    /// binding it replaces. **Every binder of `self.params` goes through here** — the
    /// header, body, generate, enum-label, genvar, real-twin, package and import
    /// binders alike — so the side map cannot be left describing a declaration that is
    /// no longer bound. A binder that HAS a range calls [`Self::bind_param_range`]
    /// immediately after; the debug assert there is what keeps that order.
    ///
    /// ⚠️ The class this closes was created by this slice and measured by the review.
    /// `param_range` is keyed exactly like `params`, and before imports bound a range
    /// no OTHER writer could rebind a ranged key — the FQ key made every scope
    /// disjoint. A wildcard `import pk::*` breaks that: it binds a package
    /// declaration at the module's OWN key, and then a local enum label, a genvar, a
    /// real parameter's integer twin or a body parameter rebinds the same key one
    /// phase later. Two of those were live correct→silent-wrong:
    /// `import pk::*` over `parameter [39:8] W` plus a local
    /// `enum { W = 32'hDEADBEEF }` answered `W[15:8]` = 239 where all three tools had
    /// said 190, and the genvar spelling answered 8 where all three had said 0.
    /// Clearing at 3 of ~13 binders was the wrong shape: the fix is that there is only
    /// one binder.
    pub(crate) fn bind_param_value(&mut self, key: String, v: i64) -> Option<i64> {
        self.param_range.remove(&key);
        self.param_type_guessed.remove(&key);
        self.params.insert(key, v)
    }

    /// Unbind `key` — the value and the range together, for the same reason.
    pub(crate) fn unbind_param(&mut self, key: &str) -> Option<i64> {
        self.param_range.remove(key);
        self.params.remove(key)
    }

    /// Bind (or CLEAR) the declared range of the param now bound at `key`.
    ///
    /// ⚠️ The clear is the point. `param_range` answers *"is this param's width a
    /// declared fact?"*, and that question is about the declaration currently bound
    /// at the key — so every writer of `self.params` that could REBIND a key another
    /// writer already ranged has to say which it is. Imports made that reachable: a
    /// wildcard `import pk::*` binds `pk`'s `parameter [31:0] W` at `top.W`, and a
    /// module-body `localparam W = ~8'hCB;` then rebinds the VALUE two phases later
    /// (3a.5 imports, 3b body params). Leaving the package's range behind would let
    /// `W[15:8]` extract bits of a 32-bit declaration out of an 8-bit value — the
    /// §4.5.363 "263-bit net" regression, arriving through the import door.
    pub(crate) fn bind_param_range(&mut self, key: &str, r: Option<(u32, u32, bool)>) {
        // The range describes the binding that is THERE. Writing it before the value
        // would be silently undone by `bind_param_value`'s clear, so the order is part
        // of the contract and this is what enforces it across every call site.
        debug_assert!(
            self.params.contains_key(key),
            "bind_param_range({key}) before the value was bound"
        );
        match r {
            Some(r) => {
                self.param_range.insert(key.to_string(), r);
            }
            None => {
                self.param_range.remove(key);
            }
        }
    }

    /// If `base` is a bare param/localparam Ident with a recorded non-zero-LSB
    /// declared range, return `(lo, width, ascending)` — resolved by the SAME
    /// `walk_scopes` as the param's value (`lookup_scoped`) and meta (`param_meta`),
    /// so the offset range can never drift from the value lookup. Drives the
    /// offset-normalization param arms in `norm_offset_if_net` / `base_net_ascending`
    /// / `norm_offset_ascending`, mirroring a net's `norm_offset_for_net`.
    /// Strip parentheses from a select's base: `(pk::B)[15:8]` names the object
    /// `pk::B[15:8]` names. See [`Self::param_sel_range`] for why that matters.
    pub(crate) fn peel_parens(e: &ast::Expr) -> &ast::Expr {
        match &e.kind {
            ast::ExprKind::Paren { inner } => Self::peel_parens(inner),
            _ => e,
        }
    }

    pub(crate) fn param_sel_range(&self, base: &ast::Expr) -> Option<(u32, u32, bool)> {
        // A parenthesised base names the same object. Both oracles REJECT the spelling
        // outright (vita keeps the value and says so — `W-PARSE-SELECT-BASE`), so there
        // is no oracle to move toward here; what there is, is vita's own answer to the
        // same select two characters away. Without the peel this slice made a
        // parenthesis change the value: `pk::B[15:8]` normalized to `34` while
        // `(pk::B)[15:8]` still read the raw `ab`, in one file.
        let base = Self::peel_parens(base);
        // `pkg::W` names a package CONSTANT, and its declaration is in the package's
        // own table. Answering here is what makes the three spellings of one select
        // — `pkg::W[m:l]`, the bare name after `import pkg::*`, and the bare name
        // after `import pkg::W` — normalize identically for a NARROW constant: the two
        // bare ones arrive as `Ident` and are answered by the walk below, because
        // `apply_import_consts` binds the range alongside the value.
        //
        // ⚠️ NARROW is the whole claim, and the review measured the boundary. A >64-bit
        // package parameter is bound by the import into `wide_param_bits`, which the
        // walk below does not look in, so `pkg::M[15:8]` normalizes and the
        // bare-imported `M[15:8]` still reads raw internal bits (silent→silent, no
        // ladder move — recorded in ROADMAP §2). Widening the walk is not a two-line
        // change: it would put a range on a key whose value lives in a SECOND map with
        // its own set of binders, and that is exactly the staleness `bind_param_value`
        // exists to make unrepresentable for `params`.
        if let ast::ExprKind::PkgScoped { pkg, name } = &base.kind {
            return self
                .pkg_const_range
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
                .copied();
        }
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

    /// Coerce a folded parameter value to its declared width, with that width
    /// ALREADY decided by the caller.
    ///
    /// The caller is the only one who knows whether an override bound, and that
    /// changes the width (see [`Self::param_decl_width_unoverridden`]). Recomputing it
    /// here threw that away: a concatenation default whose width the caller had just
    /// resolved to 64 was re-derived as the value-inferred 35 and the value coerced to
    /// it, which put `axi_crossbar`'s per-port vectors back to loud.
    pub(crate) fn coerce_param_value_with(&mut self, v: i64, meta: Option<(u32, bool)>) -> i64 {
        match meta {
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

    /// The declared default of a parameter whose DECLARED TYPE is integral but whose
    /// initializer mentions a real (`localparam int M = R*2.0;` — both oracles 6).
    ///
    /// A declaration IS a context boundary: §6.24.1 converts the value to the
    /// declared type, so the initializer folds whole in the real domain and only the
    /// rounded result becomes the parameter's value. Reached only after
    /// `param_real_value` has declined, which is what keeps a genuinely real
    /// parameter in the real domain.
    ///
    /// ⚠️ `meta` is the gate, and it is the right one for a REASON, not by luck. An
    /// UNTYPED parameter takes its type from its value (§6.20.2), so `localparam M =
    /// R*2.0;` is a REAL parameter and rounding it here would be a silent-wrong of
    /// the exact family §4.5.232 withdrew over — and `param_decl_width_unoverridden`
    /// answers None for precisely that shape, because a non-literal initializer gives
    /// it no width to infer. So `meta.is_some()` on a real-mentioning initializer
    /// means the declaration STATED a range or an integral type. (That untyped
    /// spelling stays loud; ROADMAP §2 owns it.)
    pub(crate) fn param_value_via_real(
        &self,
        meta: Option<(u32, bool)>,
        value: &ast::Expr,
    ) -> Option<i64> {
        meta?;
        self.const_int_via_real(value)
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
    /// The loud line for a parameter whose value the constant domain cannot produce.
    ///
    /// ⭐ Two things about it were wrong at once, and they are the same defect seen
    /// from two sides: it named the DECLARED thing and not the REJECTED one. The
    /// caret came from `cur_span`, which during module elaboration is the module
    /// header — so N rejections in one module printed the same `file:line:col`, at a
    /// line with no parameter on it. And the text stopped at "not a foldable constant
    /// expression", so three declarations rejected for three unrelated reasons were
    /// indistinguishable. Both are fixed here: the caret goes on the INITIALIZER, and
    /// [`Self::unfoldable_reason`] names the innermost sub-expression that failed.
    pub(crate) fn param_value_unfoldable(&mut self, what: &str, name: &str, value: &ast::Expr) {
        let unknown_fill = const_eval::fill_literal_ast(value)
            .map(|(raw, kind)| (raw.to_string(), kind))
            .filter(|(raw, kind)| literal::fill_is_unknown(raw, *kind));
        let msg = match unknown_fill {
            Some((raw, _)) => format!(
                "{what} `{name}` is declared `{raw}`, which this parameter model cannot \
                 hold — a parameter value has no x/z plane"
            ),
            None => match self.unfoldable_reason(value) {
                Some(why) => format!("{what} `{name}` value is not a constant: {why}"),
                None => format!("{what} `{name}` value is not a foldable constant expression"),
            },
        };
        self.error_at(MsgCode::ElabUnsupported, value.span, &msg);
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
                    // A `-G` fill override carries no text at all, so the flag is
                    // never read — `false` is the honest value.
                    str_is_literal: false,
                    str: None,
                    // A fill has no width of its own — it takes the target's, which is
                    // exactly what the `fill` channel above exists to do.
                    bits: None,
                    // A fill is unsigned and is re-folded at the target width, so the
                    // extension channel never reads this.
                    signed: Some(false),
                });
                continue;
            }
            // The SIZED-literal form is parsed once and kept whole: `wide` carries the
            // value at its declared width, which is what a >64-bit `-G` needs and what
            // §6.20.2 wants even when the value fits.
            let sized = crate::literal::parse_int_literal(t, ast::IntLitKind::Sized);
            // A bare decimal is signed; a sized literal states its own signedness.
            let sized_signed = sized.as_ref().map(|c| c.signed).unwrap_or(true);
            let mut wide: Option<ir::ConstVal> = None;
            let (value, text) = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
                (None, Some(t[1..t.len() - 1].to_string()))
            } else if let Ok(v) = t.parse::<i64>() {
                (Some(v), None)
            } else if let Some(c) = sized {
                wide = Some(c.clone());
                // The i64 channel still takes it when it fits, so every override that
                // applied before applies by the same route.
                let fits = !c.bits.val.iter().skip(1).any(|&w| w != 0);
                (
                    fits.then(|| c.bits.val.first().copied().unwrap_or(0) as i64),
                    None,
                )
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
                // `-G NAME="text"` is a literal by construction — the CLI parses the
                // quotes itself, there is no expression to fold.
                str_is_literal: true,
                str: text,
                bits: wide,
                // A bare decimal on the command line is a SIGNED integer; a sized
                // literal carries its own `s`, which `wide` above already holds.
                signed: Some(sized_signed),
            });
        }
        out
    }

    /// An override's value RESIZED to the parameter's declared width, in the wide bit
    /// domain — the one place that can carry a value the 64-bit integer lane cannot.
    ///
    /// Channels in precedence order, which is `chosen_val`'s own order minus the
    /// declared default (this runs only when an override is in flight):
    ///  1. `bits` — the override folded at its own width WITH its signedness.
    ///  2. `fill` — `'1`/`'0` re-folded at the target width (`fill_to_i64` cannot
    ///     represent 128 ones, so the i64 lane answered `…ffffffffffffffff`).
    ///  3. the i64, extended with the EXPRESSION's signedness. `None` signedness (a
    ///     `defparam`) declines, keeping that channel on the route it took before.
    ///
    /// A parameter with no declared width keeps the override's own range (§6.20.2).
    fn override_at_declared_width(
        &self,
        decl: Option<(u32, bool)>,
        ovr_bits: Option<&ir::ConstVal>,
        ovr_fill: Option<&(ast::IntLitKind, String)>,
        chosen_val: Option<i64>,
        ovr_signed: Option<bool>,
    ) -> Option<ir::ConstVal> {
        let (bits, from_w, from_sg) = if let Some(c) = ovr_bits {
            (c.bits.clone(), c.width, c.signed)
        } else if let Some((k, raw)) = ovr_fill {
            let w = decl.map(|(w, _)| w)?;
            if literal::fill_is_unknown(raw, *k) {
                return None;
            }
            let cv = literal::fill_literal_const(raw, *k, w)?;
            (cv.bits, w, false)
        } else {
            let v = chosen_val?;
            (bp_from_limbs(vec![v as u64], 64), 64, ovr_signed?)
        };
        let (w, sg) = decl.unwrap_or((from_w, from_sg));
        Some(ir::ConstVal {
            width: w,
            signed: sg,
            repr: ir::ConstRepr::Numeric,
            bits: resize_bits(&bits, from_w, w, from_sg),
        })
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
                        if let Some(b) = &ov.bits {
                            o.bits.insert(p.name.name.clone(), b.clone());
                        }
                        if let Some(sg) = ov.signed {
                            o.signed.insert(p.name.name.clone(), sg);
                        }
                        if let Some(t) = Self::override_text_for(p, ov) {
                            o.text.insert(p.name.name.clone(), t);
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
                        if let Some(b) = &ov.bits {
                            o.bits.insert(p.name.name.clone(), b.clone());
                        }
                        if let Some(sg) = ov.signed {
                            o.signed.insert(p.name.name.clone(), sg);
                        }
                        if let Some(t) = Self::override_text_for(p, ov) {
                            o.text.insert(p.name.name.clone(), t);
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
            // ⚠️ The FOURTH channel is `bits`, and it must be here for the same reason
            // it must be in `keeps_default_of`: a wide-literal override IS applied now,
            // so escalating it to *"not a constant, so the declared default would be
            // used instead"* describes the opposite of what happens. The two
            // conjunctions are one rule and drifted apart once already.
            let has_applied_override = ovr_by_name
                .get(p.name.name.as_str())
                .copied()
                .flatten()
                .is_some()
                || ovr_fill.contains_key(p.name.name.as_str())
                || ovr_str.contains_key(p.name.name.as_str())
                || ovr.bits.contains_key(p.name.name.as_str());
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
                    // ⚠️ `param_str_literal`, NOT the widened resolver. This guard asks
                    // *"was this declared as a string?"*, and it approximates that with
                    // *"is its default a string literal?"*. Asking the VALUE domain
                    // instead makes an ordinary untyped parameter whose default happens
                    // to be a string EXPRESSION — `parameter W = {"A","B"}` — refuse a
                    // perfectly legal numeric override: both oracles run `#(.W(9))` and
                    // print 9, and so did this simulator before the widening.
                    // correct-support → loud is a fall down the ladder.
                    //
                    // (The approximation is already too broad for a LITERAL default —
                    // iverilog accepts `#(parameter W="AB")` + `#(.W(9))` — but that
                    // false-loud is pre-existing and recorded in ROADMAP §3; growing it
                    // is what this slice must not do. Same distinction that keeps
                    // `systask.rs` on this helper.)
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
                    self.param_str_or_folded(p, ovr_by_name.contains_key(p.name.name.as_str()))
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
                        saved.push((key.clone(), self.bind_param_value(key, ov)));
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
                    saved.push((key.clone(), self.bind_param_value(key, i)));
                }
                return;
            }
            // The default binds only when nothing overrode it — on ANY channel.
            let default_binds = !ovr_by_name.contains_key(p.name.name.as_str())
                && !ovr_fill.contains_key(p.name.name.as_str())
                && !ovr_str.contains_key(p.name.name.as_str())
                && !ovr_unfoldable.contains(p.name.name.as_str());
            let ovr_bits = ovr.bits.get(p.name.name.as_str());
            let meta = if default_binds {
                self.param_decl_width_unoverridden(p)
            } else {
                // §6.20.2: an UNTYPED parameter takes the range of its FINAL override
                // value. Only the wide channel carries one — `by_name` is a bare i64 —
                // so without this the child re-derived a width from the magnitude:
                // `#(.M_ISSUE(M_ISSUE))` forwarding `{2{32'd4}}` arrived as 35 bits
                // where every other tool has 64, and the per-port slice `M_ISSUE[32 +:
                // 32]` then read past the end. A DECLARED type still wins — it survives
                // an override, and `param_decl_width` answers for it first.
                // Order matters and is §6.20.2's: a DECLARED type survives an override,
                // the override's own width comes next, and only then the width inferred
                // from the folded value. Asking `param_decl_width` first put the
                // inference ahead of the override — it answers 35 for `{2{32'd4}}` (the
                // value's minimal signed width) and never reached the override's 64.
                self.param_decl_width_declared_overridden(p)
                    .or_else(|| ovr_bits.map(|c| (c.width, c.signed)))
                    .or_else(|| self.param_decl_width(p))
            };
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
            let mut chosen_val: Option<i64> = ovr_fill_v
                .or_else(|| ovr_by_name.get(p.name.name.as_str()).copied().flatten())
                .or_else(|| self.eval_param_init(&p.value, pw))
                // The WIDE bit domain, read back as an i64 because the declaration
                // fits one. Last in the chain: it fires only where every integer arm
                // declined, so a reduction (`^A`), a select (`A[7:4]`) or a wide
                // comparison becomes a value instead of an error, and nothing that
                // folded before changes route.
                .or_else(|| {
                    let dm = self.param_decl_width_declared(p);
                    self.param_i64_at_declared(&p.value, dm)
                });
            // Wider than the i64 constant domain — see `wide_param_bits`. Reached
            // ONLY when the numeric fold above already declined, so a wide DECLARATION
            // whose value happens to fit (`localparam logic [255:0] K = 256'h1`) keeps
            // its integer identity and stays usable as a width, a bound and a
            // generate condition. Gating on the declared WIDTH instead took four
            // declaration scopes from correct to loud — the field doc claimed the
            // opposite ("the boundary is the VALUE, not the declared width") and the
            // code was the counterexample.
            // An override whose value is WIDER than the i64 channel: install it in the
            // wide side map, which is where a >64-bit parameter lives. Before this,
            // `-G K=128'hdead…` and `#(.K(128'h…))` could only be REFUSED — correctly,
            // since installing the low 64 bits would be a different design — so a
            // sweep over 128-bit keys had to go through a file instead.
            // ⚠️⚠️ The override AT THE DECLARED WIDTH — the whole silent lane the
            // aes_top round-34 census found, and the report named only its LOUD half.
            //
            // On `parameter logic [127:0] K = 128'hAAAA…`, `#(.K(-1))`, `#(.K(8'shFF))`
            // and `#(.K('1))` printed `0000000000000000ffffffffffffffff` where both
            // oracles print all ones: the i64 lane holds the value and ZERO-extends it
            // on a read past bit 63, so a sign that has to reach bit 127 is lost. The
            // old gate here only admitted an override whose LITERAL was already wider
            // than 64 bits, so every narrow one fell through to that lane.
            //
            // ⚠️ Extending needs the OVERRIDE's signedness, not the parameter's and not
            // the sign of the i64: `64'hFFFF_FFFF_FFFF_FFFF + 64'd0`, `-(64'sd1)` and
            // `32'd0 - 32'd1` fold to the same i64 and the oracles extend the first
            // with zeros and the other two with ones. That is what `ovr.signed` is for.
            //
            // ⚠️ And the result goes wide ONLY when it must. `#(.K(32'h5))` on the same
            // declaration resizes to 128 bits of which the top 64 are zero — the i64
            // lane reproduces that exactly, and a parameter parked in `wide_param_bits`
            // stops being usable as a width or a bound (measured: `logic [K-1:0]` went
            // correct → E3009 the moment a wide-literal override appeared). So the
            // install is keyed on "are there bits the i64 lane cannot carry", which
            // also repairs that pre-existing regression.
            if !default_binds {
                let cv = self.override_at_declared_width(
                    self.param_decl_width(p),
                    ovr_bits,
                    ovr_fill.get(p.name.name.as_str()),
                    chosen_val,
                    ovr.signed.get(p.name.name.as_str()).copied(),
                );
                if let Some(cv) = cv {
                    // ⚠️⚠️ AN UNKNOWN-PLANE GUARD BELONGS HERE AND CANNOT BE WRITTEN YET
                    // — built, measured, reverted (ROADMAP §2 row 15).
                    //
                    // The test below reads only the VALUE bits, so an override carrying
                    // x or z fits the i64 lane and its plane is dropped:
                    // `#(.K(8'b1010_010x))` binds `10100100` at exit 0 where both
                    // oracles keep the x, and `8'bzzzzz1z0` binds `11111110` because z's
                    // value bit is 1. Refusing with `bp_any_unknown` moves those five
                    // cells silent-wrong → loud, and the sibling `fill` arm twelve lines
                    // up already refuses the same way.
                    //
                    // ⚠️ It also refuses **76 cells that were CORRECT**. A 2-STATE
                    // declaration (`bit`/`byte`/`shortint`/`int`/`longint`) converts x
                    // and z to 0 on assignment, so dropping the plane IS that conversion
                    // and the i64 lane was exactly right — `parameter bit [7:0] K` with
                    // `#(.K(8'b1010_010x))` is `10100100` in vita and in iverilog.
                    // Review measured every channel, including `-G`.
                    //
                    // ⭐ So the guard needs "is this declaration 4-state", and THAT IS NOT
                    // IN THE AST: `hdl-parser/src/params.rs` computes `var_kind` for
                    // exactly these keywords and drops it, because `ParamDecl` has no
                    // such field — the same gap its own comment documents for the
                    // 1-bit-`logic` range. Recording it is an `hdl-ast` field and a
                    // SchemaHash re-pin, not a conditional here.
                    //
                    // ⚠️ The same field closes a sibling silent-wrong this arm cannot see
                    // either: on a 2-state declaration z must also become 0, and today it
                    // becomes 1.
                    if (64..cv.width as usize).any(|i| bp_get(&cv.bits, i).0) {
                        let key = self.fq(&p.name.name);
                        self.wide_param_bits.insert(key, cv);
                        return;
                    }
                    // ⚠️⚠️ It fits the i64 lane — but `chosen_val` may not HOLD it.
                    // `#(.K({64'h0, 64'h5}))` folds to a 128-bit constant whose value
                    // is 5; the i64 walk declines the concatenation, so `by_name` is
                    // `Some(None)` and the chain fell through to `eval_param_init` on
                    // the DECLARED DEFAULT. Before the four-channel conjunction above
                    // that shape was loud; letting it fall through here would have
                    // traded that loud for a silent default — which is exactly the
                    // event this whole block exists to remove. The wide fold IS the
                    // override, so read it back.
                    chosen_val = Some(cv.bits.val.first().copied().unwrap_or(0) as i64);
                }
            }
            {
                // ⚠️⚠️ `p.value` is the DECLARED DEFAULT's expression, and this helper
                // asks *"do the two domains AGREE about it?"* — a question that only
                // means anything when the default is what binds. With an override in
                // flight the two are DIFFERENT expressions, so they disagree by
                // construction and this arm installed the DEFAULT and returned,
                // throwing the override away at `errors=0`.
                //
                // Measured on `parameter logic [127:0] K = 128'hAAAA…1111`:
                // `#(.K(5))`, `#(.K(32'hDEADBEEF))` and `#(.K(-1))` all printed the
                // default where both oracles print the override. NINETEEN cells,
                // every channel (`#()`, positional, `defparam`, `-G`, generate scope),
                // every storage class — the whole silent lane was this one call
                // folding the wrong expression.
                //
                // ⚠️ The flip is at declared width 65, not at the override's width: a
                // declaration of 64 or less never reaches here.
                let wide = if default_binds {
                    self.wide_disagreeing_value(&p.value, meta, chosen_val)
                } else {
                    None
                };
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
            let v = self.coerce_param_value_with(v, meta);
            let key = self.fq(&p.name.name);
            // Persistent copy for hierarchical reads (`dut.WIDTH`) — `self.params`
            // is restored after the instance, so the read side needs this.
            self.hier_params.insert(key.clone(), v);
            if let Some(m) = meta {
                self.param_meta.insert(key.clone(), m);
            }
            // `default_binds` is the same flag `meta` was computed with a few lines up:
            // an overridden header parameter's initializer no longer supplies its width.
            // An OVERRIDE supplies one instead — that is what §6.20.2 says the range of
            // an untyped parameter is, and the wide channel is the only one carrying it.
            let range = self.param_decl_range_opt(p, default_binds).or_else(|| {
                (!default_binds)
                    .then(|| ovr.bits.get(p.name.name.as_str()))
                    .flatten()
                    .map(|c| (0, c.width, false))
            });
            let prev = self.bind_param_value(key.clone(), v);
            self.bind_param_range(&key, range);
            // An override reached an UNTYPED declaration: the meta recorded above is
            // the default literal's, not the override's (§2 row 25) — mark the type a
            // guess so a consumer that must not extend by a guessed sign declines.
            // …and a header-list SIBLING that is untyped and derives from one
            // (`#(parameter P = 5, parameter Q = P + 1)`) inherits the guess: its
            // value was folded from the guessed binding and its meta inferred from
            // that value. Typed declarations carry a declared type and are facts.
            if matches!(p.ty, ast::ParamType::Implicit | ast::ParamType::Time)
                && (!default_binds || self.ast_reads_guessed_param(&p.value))
            {
                self.param_type_guessed.insert(key.clone());
            }
            saved.push((key, prev));
        }
    }

    /// Restore the param map to the snapshot taken before this instance bound its
    /// params (so sibling instances of the same module re-bind cleanly).
    pub(crate) fn restore_params(&mut self, saved: Vec<(String, Option<i64>)>) {
        for (k, prev) in saved.into_iter().rev() {
            // Through the funnel in both directions: the scope being unwound is dead,
            // and a range left behind for one of its keys would outlive the value it
            // describes.
            match prev {
                Some(v) => {
                    self.bind_param_value(k, v);
                }
                None => {
                    self.unbind_param(&k);
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
