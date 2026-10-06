//! Real-domain constant folding (`localparam real R = 2.0 + 3.0`).
//!
//! `param_real_value` folded a real LITERAL, and for a declared-`real` parameter
//! it fell back to the INTEGER const domain (so `parameter real R = 3;` binds
//! 3.0 and keeps its i64 twin). Real ARITHMETIC reached neither and the parameter
//! went loud — `2.0+3.0`, `*`, `/`, `-`, `**` all E3009 — even though vita's own
//! runtime computes them. `localparam real` is a common idiom, so this was a
//! visible hole rather than an exotic one.
//!
//! This evaluator stays in f64 from end to end. It never converts a real leaf to
//! an integer: doing that at the leaf was tried before and destroyed the value
//! before the enclosing operator could pick its domain (a real `R` compared with
//! `R > 2` took the wrong generate branch). Conversion belongs at the CONTEXT
//! boundary — which here is the parameter binding, where an exactly-integral
//! result additionally registers its i64 twin so the integral capabilities of
//! `parameter real R = 4;` are not lost.
//!
//! The reverse promotion IS correct and is done: an INTEGER operand inside a real
//! expression widens to f64 (§11.8.1 — an operation with any real operand is
//! evaluated in the real domain), so `10 / 4.0` is 2.5, not 2.

use super::*;

impl Elaborator<'_> {
    /// The TRUTH of a constant control expression (a generate-if condition, a
    /// generate-for condition). Folds in the integer domain first; if that
    /// declines and the expression mentions a real, folds in the real domain and
    /// converts to a truth value THERE.
    ///
    /// This is the context boundary for a real control expression: the comparison
    /// `R/2 > 2` is evaluated wholly in the real domain (§11.8.1) and only its
    /// 1-bit RESULT crosses into the integer world. Converting `R` to an integer
    /// first would decide the branch on the wrong value — the exact leaf-conversion
    /// mistake this domain exists to avoid.
    ///
    /// `top_level` says the condition is a generate-if written at the module's top level,
    /// the one position [`Self::selfdet_truth_reading_str`] reads a string literal in.
    pub(crate) fn const_truth_in_scope(&self, e: &ast::Expr, top_level: bool) -> Option<bool> {
        // A CONDITION is a self-determined position (§11.6.1 Table 11-21): nothing
        // around it supplies a width. `generate if (4'd15 + 4'd1)` therefore tests the
        // 4-bit 0 and takes the `else` — which is what iverilog AND verilator do, and
        // what the width-unlimited fold got wrong by elaborating the other branch,
        // silently, at exit 0.
        if let Some(v) = self.const_int_selfdet(e) {
            return Some(v != 0);
        }
        // …then the WIDE bit domain, which is the only one that can weigh a value the
        // i64 walk cannot hold: `generate if ({1'b1, 63'd0} > 0)` is 64 bits with the
        // top one set, and both oracles take the THEN branch. Reading it as a truth
        // value is safe for the same reason the branch above is — §11.6.1 Table 11-21
        // makes a condition SELF-DETERMINED, so there is no context width it should
        // have been evaluated at instead.
        if let Some(v) = self.selfdet_bits_unsigned(e) {
            return Some(v != 0);
        }
        if let Some(t) = top_level
            .then(|| self.selfdet_truth_reading_str(e))
            .flatten()
        {
            return Some(t);
        }
        if !self.expr_mentions_real(e) {
            return None;
        }
        Some(self.const_eval_real_in_scope(e)? != 0.0)
    }

    /// The truth of a control expression that holds a STRING LITERAL, read as §5.9's
    /// unsigned constant of eight bits per character — `localparam int UseDsp = "no";`
    /// then `if (UseDsp == "yes")` is how `ibex_counter` picks its DSP flop, and all three
    /// oracles take the `else` (`00006e6f` against `00796573`). The two domains above
    /// decline any literal, since their name hooks answer names only (the literal
    /// reading is opt-in per consumer, [`Self::param_leaf_bits`]); a condition may opt
    /// in because §11.6.1 makes it self-determined, so nothing around it could have made
    /// the literal a `string`.
    ///
    /// Only the literal is new, and only where a name cannot be read wrong: the caller
    /// asks for a generate-if at the module's top level, and every name must be one the
    /// module declares once, at its top level, as a parameter with a written integral
    /// type ([`Self::cond_names_ok`], `cond_names.rs` — the review rounds that found each
    /// other name reading an outer object). A literal with an escape Table 5-1 does not
    /// define declines too
    /// ([`crate::const_str::std_str_lit_bits`]). The truth is "some bit is 1", so a
    /// literal past 64 bits needs no u64 reading; an x bit declines as it does above.
    fn selfdet_truth_reading_str(&self, e: &ast::Expr) -> Option<bool> {
        if !holds_str_lit(e) || !self.cond_names_ok(e) {
            return None;
        }
        let lit: crate::const_str::LitBits = crate::const_str::std_str_lit_bits;
        let (b, w, _) = fold_self_bits(e, &|n, _| self.leaf_bits_reading(lit, n))?;
        if bp_any_unknown(&b, w) {
            return None;
        }
        Some((0..w as usize).any(|i| bp_get(&b, i).0))
    }

    /// The REAL value of a constant subtree with no real in it — the §11.8.1 crossing
    /// of its self-determined integral value into the real domain, converted at its own
    /// width and sign by the engine's conversion ([`sim_ir::mw::int_to_real`]: set bits
    /// added LSB first, as iverilog does). Every elaborate-time int→real crossing of an
    /// EXPRESSION reads this, through [`Self::const_eval_real_in_scope`]'s real-free arm: a
    /// real-free operand of a real operator, a declared-real parameter's integral
    /// initializer (`param_real_value`).
    ///
    /// The width and sign come from the wide bit domain, which keeps both; the i64 walk
    /// keeps neither, and reading its value as a signed 64-bit number made every
    /// unsigned 64-bit constant with the top bit set NEGATIVE: `localparam real R =
    /// 64'hC000_0000_0000_0401;` was -4611686018427386880.0 where iverilog, verilator and
    /// sv2v → iverilog bind 13835058055282163712.0 (so `generate if (… > 1.0e19)` took
    /// the other branch), and `65'd5 - 65'd7` was the i64 wrap -2.0 for the oracles'
    /// 36893488147419103232.0. An x/z bit declines here as everywhere else in this
    /// domain (§6.12.2 reads it as 0 and the oracles agree, `4'bx011` is 3.0; the run-time
    /// conversion does that), so such a constant stays loud.
    ///
    /// A shape the wide domain declines keeps the i64 walk's value, read signed — except
    /// a top whose DECLARED type is unsigned, whose value the i64 holds as its
    /// two's-complement image: a constant function call's return type (§13.4.1;
    /// `function logic [63:0] f(…)` returning `64'hC000_0000_0000_0401` binds
    /// 13835058055282163712.0 in all three oracles, not -4611686018427386880.0) and a
    /// primitive cast's type (§6.24.1; `time'(-5) + 0.0` is 18446744073709551616.0 in
    /// iverilog, not -5.0).
    fn const_selfdet_real(&self, e: &ast::Expr) -> Option<f64> {
        if let Some((b, w, sg)) = fold_self_bits(e, &|n, _| self.wide_name_bits(n)) {
            if bp_any_unknown(&b, w) {
                return None;
            }
            return Some(sim_ir::mw::int_to_real(&b.val, &b.unk, w, sg));
        }
        let v = self.const_int_selfdet(e)?;
        if v < 0 {
            let declared = match &Self::peel_parens(e).kind {
                ast::ExprKind::Call { name, .. } => self
                    .const_fn_def(name)
                    .and_then(|(f, p)| self.const_fn_ret_wsign_in(f, p.as_deref())),
                ast::ExprKind::Cast {
                    target: ast::CastTarget::Prim(p),
                    ..
                } => crate::expr_cast::cast_prim_wsign(*p).map(|(w, s, _)| (w, s)),
                _ => None,
            };
            if let Some((w @ 1..=64, false)) = declared {
                let mask = if w == 64 { u64::MAX } else { (1u64 << w) - 1 };
                return Some(u64_to_real(v as u64 & mask));
            }
        }
        Some(i64_to_real(v))
    }

    /// Fold `e` in the REAL domain. `None` (⇒ the caller stays loud) for anything
    /// this domain cannot evaluate exactly: an unmodeled node, an unbound name, a
    /// division by zero, or a non-finite result.
    ///
    /// Callers must reach this only AFTER the integer domain has declined, so a
    /// wholly integral expression keeps its integer value and its exact binding —
    /// `parameter real R = 3/2;` stays 1.0 (integer division), matching iverilog,
    /// rather than becoming 1.5.
    pub(crate) fn const_eval_real_in_scope(&self, e: &ast::Expr) -> Option<f64> {
        use ast::ExprKind as K;
        let fin = |v: f64| v.is_finite().then_some(v);
        // §11.8.1: a real operator CONVERTS its integral operand, and the
        // conversion reads the integral subtree's SELF-DETERMINED value — the real
        // side gives it no width context. Recursing into the subtree with f64
        // arithmetic instead re-implemented integer arithmetic width-unlimited:
        // `1.0 + -4'sd8` folded 9.0 (the 4-bit negate wraps to −8 ⇒ −7.0, iverilog
        // agrees), `1.0 + 3/2` folded 2.5 (integer division ⇒ 2.0), and the `**`
        // exponent cell `2.0 ** -4'sd8` promoted −8 to +8. So a real-free subtree
        // folds in the INTEGER domain at its own width and only its RESULT crosses
        // into f64. Declining integral shapes stay loud here (falling back to the
        // f64 re-walk would revive exactly the widening this gate closes).
        // `expr_mentions_real` is the same conservative discriminator
        // `param_real_value` orders the two domains with — on the SHADOW-CORRECT
        // resolver, which is the one the name arm below reads: an integral
        // `localparam N = 3` in a generate block over a module-scope `real N = 2.5`
        // is the integer 3, so `N / 2` beside a real folds 1 (integer division), where
        // the blind walk saw the outer real and folded 1.5.
        if !self.expr_mentions_real_opt(e, true) {
            return self.const_selfdet_real(e);
        }
        match &e.kind {
            K::RealLit { raw, .. } => Some(parse_real_f64(raw)),
            // An integer literal inside a real expression promotes (§11.8.1).
            // (Reached only for a literal the gate above declined to claim — kept
            // for the day `expr_mentions_real` learns a form this arm models.)
            K::IntLit { .. } => self.const_selfdet_real(e),
            K::Paren { inner } => self.const_eval_real_in_scope(inner),
            K::Unary { op, operand } => {
                let v = self.const_eval_real_in_scope(operand)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => Some(-v),
                    // `!r` is defined (§11.4.7) but a real has no bit operators;
                    // keep the rest loud rather than inventing a bit pattern.
                    ast::UnOp::LogNot => Some((v == 0.0) as i64 as f64),
                    _ => None,
                }
            }
            // A single-segment name: its INNERMOST binding, real or integral — the
            // binding the lowering reads (`bare_ident_route`) and the override's domain
            // was classified by (`override_domain`). Walking the real map alone first
            // read an outer `real N = 2.5` through an inner `localparam N = 3`: `N + X`
            // with `real X = 5` folded 7.5 where both oracles give 8.0.
            K::Ident(p) if p.segments.len() == 1 => {
                let n = &p.segments[0].name;
                if self.real_param_lowers_real(n) {
                    self.walk_scopes(n, &self.real_param_val)
                } else {
                    self.lookup_scoped(n).map(i64_to_real)
                }
            }
            // The package twin, in the same precedence order (real map first, then the
            // integer one promoted). Its absence is what made a MODULE-LOCAL
            // `localparam real` fold and a byte-identical `pkg::` one go loud — the
            // value was in `pkg_real_val` the whole time and this walk had no arm to
            // read it with.
            K::PkgScoped { pkg, name } => self
                .pkg_real_val
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
                .copied()
                .or_else(|| {
                    self.pkg_consts
                        .get(&pkg.name)
                        .and_then(|c| c.get(&name.name))
                        .map(|&v| i64_to_real(v))
                }),
            K::Binary { op, lhs, rhs } => {
                let a = self.const_eval_real_in_scope(lhs)?;
                let b = self.const_eval_real_in_scope(rhs)?;
                use ast::BinOp as B;
                match op {
                    B::Add => fin(a + b),
                    B::Sub => fin(a - b),
                    B::Mul => fin(a * b),
                    // A real division by zero is ±inf / NaN in IEEE-754, which is
                    // not a usable parameter value — stay loud.
                    B::Div if b != 0.0 => fin(a / b),
                    B::Pow => fin(a.powf(b)),
                    // Relational / equality operators yield a 1-bit INTEGER result,
                    // which the real domain carries as 0.0 / 1.0 so a ternary or a
                    // generate condition built from reals still folds.
                    B::Lt => Some((a < b) as i64 as f64),
                    B::Le => Some((a <= b) as i64 as f64),
                    B::Gt => Some((a > b) as i64 as f64),
                    B::Ge => Some((a >= b) as i64 as f64),
                    // An `inside` element on a real is §11.4.13's `==` (non-integral).
                    B::Eq | B::CaseEq | B::InsideEq => Some((a == b) as i64 as f64),
                    B::Ne | B::CaseNe => Some((a != b) as i64 as f64),
                    B::LogAnd => Some(((a != 0.0) && (b != 0.0)) as i64 as f64),
                    B::LogOr => Some(((a != 0.0) || (b != 0.0)) as i64 as f64),
                    // Bit-wise / shift / modulus are not defined on a real operand
                    // (§11.4) — loud, never a silently coerced bit pattern.
                    _ => None,
                }
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // The condition is a truth value in a SELF-DETERMINED position
                // (§11.4.11) — integer first (the common `P > 2 ? …` spelling),
                // but at the condition's own width: the unlimited fold read
                // `(4'd15 + 4'd1) ? 1.5 : 2.5` as 16 ⇒ 1.5 where the 4-bit sum
                // wraps to 0 ⇒ 2.5 (iverilog agrees). A real-mentioning
                // condition declines in the integer walk and folds here.
                let c = match self.const_int_selfdet(cond) {
                    Some(v) => v != 0,
                    None => self.const_eval_real_in_scope(cond)? != 0.0,
                };
                if c {
                    self.const_eval_real_in_scope(then_e)
                } else {
                    self.const_eval_real_in_scope(else_e)
                }
            }
            _ => None,
        }
    }
}

/// Convert a real constant to the integer domain AT A CONTEXT BOUNDARY (§6.24.1).
///
/// The conversion ROUNDS, and rounds a `.5` AWAY FROM ZERO: `2.5`→3, `3.5`→4,
/// `−2.5`→−3, `−0.5`→−1. Rust's `f64::round` is exactly that rule, which is why
/// this is a cast and not the `e + (e >= 0 ? 0.5 : −0.5)` construction the LOWERED
/// form has to use — that spelling is tie-to-EVEN for odd values in [2^52, 2^53)
/// and cost §4.5.365 a silent-wrong.
///
/// `$rtoi` TRUNCATES instead and has its own arm; the two are not interchangeable
/// (`$rtoi(2.9)` is 2 where `int'(2.9)` is 3 — measured on both oracles).
///
/// Declines a non-finite value and anything outside the i64 walk, so a caller that
/// cannot represent the result stays loud rather than wrapping.
pub(crate) fn real_round_to_i64(x: f64) -> Option<i64> {
    let r = x.round();
    // 2^63 exactly: `i64::MAX as f64` rounds UP to it, so comparing against the
    // cast bound would admit a value that overflows on the way back.
    const LIM: f64 = 9_223_372_036_854_775_808.0;
    (r.is_finite() && (-LIM..LIM).contains(&r)).then_some(r as i64)
}

/// A u64 constant (an unsigned reading, `const_unsigned_selfdet`) converted by the
/// engine's conversion ([`sim_ir::mw::int_to_real`]). Equal to `v as f64` below 2^53.
pub(crate) fn u64_to_real(v: u64) -> f64 {
    sim_ir::mw::int_to_real(&[v], &[0], 64, false)
}

/// An i64 constant read as a signed 64-bit integer, converted by the engine's conversion
/// ([`sim_ir::mw::int_to_real`]) — the reading for a value whose width and sign the
/// domain did not keep. Equal to `v as f64` below 2^53.
pub(crate) fn i64_to_real(v: i64) -> f64 {
    sim_ir::mw::int_to_real(&[v as u64], &[0], 64, true)
}

impl Elaborator<'_> {
    /// Fold `e` in the REAL domain and hand back its INTEGER reading (§6.24.1).
    ///
    /// This is the context-boundary converter the real domain's header calls for.
    /// It is deliberately NOT reachable from a leaf: callers must be a position the
    /// language itself defines as integral — an `int'()`/`byte'()` cast, a `$clog2`
    /// argument, a replication count, or a parameter whose DECLARED type is
    /// integral. Registering an i64 twin at the leaf instead was tried in §4.5.232
    /// and opened five silent-wrongs, because it let the INTEGER domain answer an
    /// expression that mentions a real, and only the real domain applies §11.8.1's
    /// "any real operand ⇒ evaluate in the real domain" ordering: `generate if
    /// (R/2 > 2)` with R = 5.0 then took the ELSE branch. Here the whole expression
    /// is folded in the real domain first and only its RESULT crosses over.
    ///
    /// Guarded on `expr_mentions_real` so a wholly integral expression can never be
    /// re-folded through f64 — the integer domain owns that case, at its own width.
    pub(crate) fn const_int_via_real(&self, e: &ast::Expr) -> Option<i64> {
        if !self.expr_mentions_real(e) {
            return None;
        }
        real_round_to_i64(self.const_eval_real_in_scope(e)?)
    }

    /// `$rtoi(e)` in a const domain: §20.10 TRUNCATES toward zero, unlike the
    /// rounding a cast performs. Kept beside its rounding sibling so the pair
    /// cannot drift apart.
    pub(crate) fn const_rtoi_via_real(&self, e: &ast::Expr) -> Option<i64> {
        // ⚠️ Integer FIRST, and the order is load-bearing rather than stylistic. A
        // wholly integral argument is already its own truncation, and asking the real
        // domain for it routes an exact i64 through f64: `const_eval_real_in_scope`
        // converts a real-free subtree to f64 (`const_selfdet_real`), which above 2^53
        // rounds.
        // Measured — `$rtoi(64'd9007199254740993)` came back 9007199254740992, and
        // since PRE had no `$rtoi` const arm at all that was a loud → silently
        // off-by-one. The real domain can only ADD answers here, never correct one.
        if !self.expr_mentions_real(e) {
            return self.const_int_selfdet(e);
        }
        real_round_to_i64(self.const_eval_real_in_scope(e)?.trunc())
    }
}

/// Does `e` hold a string literal anywhere the constant domain reads?
pub(crate) fn holds_str_lit(e: &ast::Expr) -> bool {
    crate::param_query::ast_any(e, &|x| matches!(x.kind, ast::ExprKind::StrLit { .. }))
}
