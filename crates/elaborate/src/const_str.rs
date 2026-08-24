//! The STRING constant domain.
//!
//! Split out of `const_fn.rs` because it is a coherent unit and because that file had
//! grown past the repository's 1000-line ceiling. Nothing here is serialized, so the
//! move cannot touch a schema hash.
//!
//! The domain answers ONE question — *what string is this constant expression?* — and
//! two callers ask it: the parameter-binding consumers (`params.rs`, `instance.rs`,
//! `generate.rs`, `iface_inst.rs`, through `param_str_or_folded`) and the string
//! equality fold below. For a long time only the second existed, which is why the
//! resolver could already handle names and package scopes while every parameter
//! binding in the tree went through a literal-only twin.

use super::*;

impl Elaborator<'_> {
    /// The STRING value of a constant expression, or None.
    ///
    /// A string literal, a parenthesised one, a `pkg::NAME`, a bare name bound in
    /// `str_param_raw`, a `?:` over two of those, or a concatenation of them —
    /// names resolved through the SAME innermost-wins
    /// `walk_scopes_key` over the SAME combined binding set that the lowering path
    /// uses (`expr_main`'s Ident arm), so a local net or an integer param that shadows
    /// a string param wins here exactly as it does there. Two spellings of that rule
    /// is how a generate-if would decide one way and the body lower the other.
    pub(crate) fn const_str_in_scope(&self, e: &ast::Expr) -> Option<String> {
        self.const_str_at(e, 0)
    }

    /// Maximum nesting this domain will walk.
    ///
    /// `const_compare_special` asks this on BOTH operands of every binary node it
    /// sees, so an unbounded walk over a left-deep chain is Θ(n²) — the same shape as
    /// the four-minute elaboration this file already documents for the numeric fold.
    /// The bound is far above anything an author writes: the deepest form in the
    /// corpus is a two-level `?:` chain.
    const STR_DEPTH_MAX: u32 = 64;

    fn const_str_at(&self, e: &ast::Expr, depth: u32) -> Option<String> {
        if depth > Self::STR_DEPTH_MAX {
            return None;
        }
        match &e.kind {
            ast::ExprKind::StrLit { raw } => Some(raw.clone()),
            ast::ExprKind::Paren { inner } => self.const_str_at(inner, depth + 1),
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                let seg = &p.segments[0].name;
                // ⚠️ `wide_param_bits` belongs in this predicate even though it is
                // never READ here: the predicate's job is to stop the outward walk at
                // the innermost binding of the name, and a key bound ONLY there would
                // otherwise let the walk reach PAST an integer shadow to an outer
                // string parameter — a value the lowering would never produce. It
                // happens to be unreachable today because generate-scope params also
                // land in `symbols`, i.e. the invariant held by an accident of a
                // different map. Spell it out instead of relying on that.
                let key = self.walk_scopes_key(seg, |k| {
                    self.str_param_raw.contains_key(k)
                        || self.real_param_val.contains_key(k)
                        || self.params.contains_key(k)
                        || self.wide_param_bits.contains_key(k)
                        || self.symbols.contains_key(k)
                })?;
                self.str_param_raw.get(&key).cloned()
            }
            // §3 ⑨: read the PACKAGE's own map. ⚠️ This arm used to look `str_param_raw`
            // up under the key `"P::S"` — a spelling NO producer ever writes (the package
            // fold keys by `$pkg$P.S`, module scope by `module.name`), so it could never
            // hit and the arm read as supported while being unreachable. That is the same
            // shape §4.5.370 found: a reference implementation with no producer. It was
            // also moot until now, because the fold did not route strings at all.
            ast::ExprKind::PkgScoped { pkg, name } => self
                .pkg_str_raw
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
                .cloned(),
            // §11.4.11. The canonical way an IP family switches an implementation on a
            // string parameter — `parameter STYLE_INT = (STYLE == "AUTO") ? "REDUCTION"
            // : STYLE;` is `lfsr.v`, which every one of Alex Forencich's cores
            // instantiates. The CONDITION is self-determined, spelled with the same
            // `const_int_selfdet` the integer domain's Ternary arm uses, so one source
            // line cannot get two answers from the two domains.
            //
            // BOTH arms must resolve as strings, not just the selected one. Requiring
            // only the taken arm would make the fold depend on the condition's value
            // for whether the expression is a string at all, and `C ? "A" : 5` would
            // be a string on one override and an integer on another — the same
            // expression changing domain under a parameter. Fail closed instead: the
            // mixed form stays loud, exactly as it is today.
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                let t = self.const_str_at(then_e, depth + 1)?;
                let f = self.const_str_at(else_e, depth + 1)?;
                let c = self.const_int_selfdet(cond)?;
                Some(if c != 0 { t } else { f })
            }
            // §11.4.12 on string operands. `{"RE", "D"}` is a bit concatenation whose
            // operands happen to be string literals, and laying their TEXT end to end
            // gives the same bytes — but only while every operand is itself a string.
            // A mixed `{"AB", 8'h43}` would need the integer domain's width rules to
            // place the numeric operand, so it declines here and stays loud rather
            // than guessing a byte.
            ast::ExprKind::Concat { parts } => {
                // ⚠️ Every value in this domain is the RAW literal, quotes INCLUDED —
                // that is what the lexer hands over and what `str_param_raw` stores.
                // Joining the raws directly produced `"RE""D"`, i.e. a four-character
                // string with two quotes in the middle, which printed as `RE""D` and
                // read as 1092756034 in an integer context. Join the CONTENTS and
                // re-quote, so the result is in the same representation as every other
                // value the domain produces.
                //
                // ⚠️⚠️ And DECLINE on an empty operand. `{"ab", ""}` is a BIT
                // concatenation in which `""` is 8'h00 (§5.9 / §11.4.12) — both
                // oracles read it as 0x616200 — but a text join represents that byte
                // as no characters at all and yields 0x6162. The string domain simply
                // cannot carry an interior NUL, so it must not pretend to: staying
                // loud here is the honest answer, and folding it was loud turning into
                // silent-wrong.
                let mut out = String::new();
                for p in parts {
                    let raw = self.const_str_at(p, depth + 1)?;
                    let content = str_raw_content(&raw)?;
                    if content.is_empty() {
                        return None;
                    }
                    out.push_str(content);
                }
                Some(format!("\"{out}\""))
            }
            _ => None,
        }
    }

    /// The two WHOLE-NODE comparison folds that do not live in the numeric domain:
    /// a `string` parameter equality (§6.16 compares TEXT, and neither operand has an
    /// i64 value at all) and `==?`/`!=?` against an x/z PATTERN literal (§11.4.6 makes
    /// the pattern's unknown bits don't-cares).
    ///
    /// Both are MODULE-SCOPE facts — they resolve names through `const_str_in_scope` /
    /// `const_eval_in_scope` — so a caller with local bindings must not consult them.
    /// They live here rather than inline because the WIDTH-AWARE walk
    /// (`eval_const_env_at`) recurses into a comparison's OPERANDS and would otherwise
    /// shadow them: routing a size cast through that walk turned `8'(MODE == "Y")` —
    /// the canonical way to switch an implementation on a string parameter — from
    /// iverilog's 1 into a decline, which a width consumer swallows as a 1-bit net.
    ///
    /// `None` means "no special case applies", not "loud": every fail-closed exit here
    /// leads to a generic fold that declines on the same operand anyway.
    pub(crate) fn const_compare_special(
        &self,
        op: ast::BinOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
    ) -> Option<i64> {
        let op = &op;
        // A STRING comparison folds in the string domain, before the i64 one —
        // `MODE == "Y"` on a `parameter string MODE` is the canonical way to
        // switch an implementation in a generate-if, and neither operand has an
        // i64 value, so the numeric fold below returned None and the generate-if
        // was loud ("condition is not a constant"). IEEE 1800 §6.16 compares
        // string VALUES, so equality is exact text equality.
        if matches!(
            op,
            ast::BinOp::Eq | ast::BinOp::Ne | ast::BinOp::CaseEq | ast::BinOp::CaseNe
        ) {
            if let (Some(x), Some(y)) = (self.const_str_in_scope(lhs), self.const_str_in_scope(rhs))
            {
                let want_eq = matches!(op, ast::BinOp::Eq | ast::BinOp::CaseEq);
                return Some(i64::from((x == y) == want_eq));
            }
        }
        // ⚠️ Everything below is the WILDCARD case only, and the LHS fold must not
        // happen before that check. This helper runs on EVERY binary node, and the
        // arm it was extracted from folds the LHS again for its generic path — so
        // computing it here unconditionally evaluates each left operand TWICE, which
        // is 2^depth on a left-deep chain. Measured: a 200-deep `(~r5) - 5'd0 - …`
        // index went from milliseconds to over four minutes (a suite timeout).
        if !matches!(op, ast::BinOp::WildEq | ast::BinOp::WildNe) {
            return None;
        }
        // ⚠️ SELF-DETERMINED, like every other operand of a comparison: this helper is
        // consulted from inside the width-aware arm, so reading the LHS with the
        // width-unlimited walk would make the wildcard the one member of its own
        // operator family still answering from the evaluator that family redirects
        // away from (`(4'd15 + 4'd1) ==? 4'b000x` is 1 for both oracles and was 0).
        let a = self.const_int_selfdet(lhs)?;
        // `==?`/`!=?` against a wildcard LITERAL (`P ==? 4'b1x1x`): the x/z bits
        // of the PATTERN (rhs) are don't-cares (§11.4.6). const_eval carries no
        // x/z, so the generic `const_eval_in_scope(rhs)` below returns None on
        // the pattern; pull the pattern's value + x/z mask straight from the
        // literal and masked-compare. The pattern zero-extends, so `a & !mask`
        // at full width matches iverilog for a narrower pattern too. Fail-closed:
        // only a NON-NEGATIVE const `a` (an i64 sign bit would corrupt the
        // full-width compare) and a single-word, bit-63-clear pattern; otherwise
        // fall through to None (loud). An x/z-free pattern is NOT intercepted —
        // it folds via the `WildEq`/`WildNe` collapse arm below.
        if matches!(op, ast::BinOp::WildEq | ast::BinOp::WildNe) {
            if let ast::ExprKind::IntLit { kind, raw } = &rhs.kind {
                // Only a SIZED pattern is safe: bits ABOVE its declared width
                // zero-extend, so the masked compare's "the LHS high bits must
                // be 0" is correct. An UNSIZED x/z literal (`'hx`) x-FILLS to the
                // context width — but parse_int_literal sizes it to its 32-bit
                // self-width, so an LHS wider than 32 bits would wrongly require
                // its high bits to be 0 (silent-wrong). Leave unsized x/z patterns
                // loud (fall through → the generic rhs fold returns None).
                if matches!(kind, ast::IntLitKind::Sized) {
                    if let Some(cv) = parse_int_literal(raw, *kind) {
                        if cv.bits.unk.iter().any(|&u| u != 0) {
                            let pat = cv.bits.val.first().copied().unwrap_or(0);
                            let mask = cv.bits.unk.first().copied().unwrap_or(0);
                            if a >= 0
                                && cv.bits.val.len() <= 1
                                && cv.bits.unk.len() <= 1
                                && (pat >> 63) == 0
                                && (mask >> 63) == 0
                            {
                                let eq = (a & !(mask as i64)) == (pat as i64 & !(mask as i64));
                                return Some(if matches!(op, ast::BinOp::WildEq) {
                                    eq
                                } else {
                                    !eq
                                } as i64);
                            }
                            return None; // negative LHS / wide / bit-63 pattern → loud
                        }
                    }
                }
            }
        }
        None
    }
}

/// The text INSIDE a raw string literal's quotes.
///
/// Values in the string constant domain carry their delimiters — `str_param_raw`
/// stores what the lexer read, and every consumer downstream expects that shape. Only
/// the concatenation arm needs to look inside, and it re-quotes what it builds.
fn str_raw_content(raw: &str) -> Option<&str> {
    // Fails CLOSED. An earlier version ended in `.unwrap_or(raw)`, which passed an
    // unquoted value through verbatim — and on a lone `"` the prefix strip succeeds
    // and the suffix strip then fails on the empty remainder, so the open form was
    // both unreachable-by-construction and wrong if ever reached. Every producer in
    // this domain emits a quoted value; anything else is a bug in a producer, and the
    // caller should decline rather than build on it.
    raw.strip_prefix('"').and_then(|r| r.strip_suffix('"'))
}
