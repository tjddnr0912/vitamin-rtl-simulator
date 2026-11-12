//! value/arms -- the constant-fold arms that two or more of elaborate's cores spelled (ROADMAP §5.2 VAL-1, owner 2026-11-09).
//! Cores: IN `const_eval_in_scope`, ENV `eval_const_env`, CTX `fold_ctx`, REG `fold_region`, HIER `env_fold`. STRUCTURE ONLY:
//! each body is the text it replaced ; a core's difference is a lane ([`ArmLane`]) or stays at the core, named below.
//! Members: `selfdet_truth_arm` (`!` and the `?:` condition: IN, ENV, CTX), [`unary_i64`] (`+ - ~` on a folded operand: IN,
//! ENV, CTX), the size / prim / signing cast (`const_eval_cast`, `const_size_cast`: IN, ENV ; the signing-over-size-cast
//! body, spelled twice in `const_eval_cast` before, is one `signing_over_size_cast`).
//! Members by reference (already one fn each, in const_fn.rs / const_wide.rs ; VAL-2 P-a did not move them, the move is
//! deferred to P-d, W/val2/plan/PLAN.txt:104): `const_clog2_selfdet`, `const_placement_env`, `selfdet_bits_i64` (reductions),
//! `const_pow_exponent_selfdet`, `eval_const_shift_count`.
//! Ratchet blind spot: structure_ratchet.rs key b (`const_option_fns`) counts the Option fns of top-level
//! `elaborate/src/const_*.rs` only, so the Option fns of value/ (here `selfdet_truth_arm`, `const_size_cast`,
//! `const_eval_cast` ; chains.rs `param_decl_fold`, lanes.rs `fold_self_wide`) are outside its count, and a counted fn
//! moved from a const_*.rs file into value/ lowers b with no structural gain (b's re-aim is deferred with the move, P-d).
//! Lanes that stay at their core (PRE differences, measured: docs/roadmap/5.2/VAL-1.md): REG's truth, unary, cast, ternary
//! (both arms folded), `$clog2` and system-call set ; HIER's `$clog2` (a value, negative declines) and `+ -` only ; ENV's
//! missing arms (prim and signing cast, `$bits`, `$rtoi`), which reach IN only from an empty frame. Each core still folds
//! the `?:` arm it takes and the unary operand itself, where and when it did.
//! ENFORCED by test (elaborate/tests/value_arms_home.rs), exactly these token shapes per (file, fn), `#[cfg(test)]` items
//! skipped, over elaborate/src: A1 `selfdet_truth (` calls ; A2 `eval_const_env_self | const_int_selfdet (..) ? ==|!= 0` ;
//! A2b the same call then `{ Some ( v ) =>` then `v cmp 0`, `Some ( v cmp 0`, `Some ( ( v cmp 0` or `Val ( ( v cmp 0` ;
//! A3 `checked_neg (` ; A4 one `fn` each of `const_eval_cast`, `const_size_cast`, `selfdet_truth_arm`, `unary_i64`,
//! `signing_over_size_cast`, here. The PRE-text equality of the two kernels is this file's `#[cfg(test)]` module.
//! REVIEW-ONLY (no test sees them): a truth spelled another way (a `let` then a compare, `.is_some_and`, `matches!`, a fold
//! reached through another name) ; a negation written `0 - v` or `!v + 1` ; macro bodies ; the same rules in sibling
//! evaluators (const_real.rs `const_eval_real_in_scope` and `const_truth_in_scope`, const_str.rs `const_str_at`'s ternary,
//! strings.rs `real_literal_value`'s unary minus, const_eval.rs `delay_units_in_scope`, block_local/proofs.rs `fold_i32`,
//! `const_eval_u32`, dead_arm.rs `lowered_truth`, hdl-parser const_fold.rs's unary minus in `try_const_index`, `const_lit`,
//! `const_lit_enum`, `plain_int`, `cw_shape`, `cw_eval`, read_set.rs `exact_const`'s `go`, an IR u64 fold whose `?:` reads
//! its condition `go(..)? != 0`) ; exec_const_stmt's three loop / branch conditions ; the IN / ENV
//! Binary and Call twins ; every REG and HIER arm.

use crate::*;

/// Which core asks an arm: the lane difference the arm keeps (the REG and HIER lanes stay at
/// their cores, see the head note).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ArmLane {
    /// IN, `const_eval_in_scope`: an empty frame, the wide-domain fallback always.
    Unlimited,
    /// ENV, `eval_const_env`: no wide-domain fallback.
    Env,
    /// CTX, `fold_ctx`: the wide-domain fallback unless the operand names a frame local.
    Ctx,
}

impl Elaborator<'_> {
    /// The TRUTH of a self-determined operand -- what `!e` and a `?:` condition ask -- in
    /// each core's lane. It takes the UNFOLDED operand: the i64 self-determined walk first
    /// (`v != 0`), then, where that declines, the wide bit domain's truth (`4'b110x` is true,
    /// `4'b000x` declines), except in the Env lane (none) and in the Ctx lane past an operand
    /// that names a constant-function local (whose value lives only in `env`). It folds no
    /// arm of a ternary.
    #[inline]
    pub(crate) fn selfdet_truth_arm(
        &self,
        lane: ArmLane,
        e: &ast::Expr,
        fr: &ConstFrame,
        depth: u32,
    ) -> Option<bool> {
        match self.eval_const_env_self(e, fr, depth) {
            Some(v) => Some(v != 0),
            None => {
                if lane == ArmLane::Env {
                    return None;
                }
                if lane == ArmLane::Ctx {
                    let local = |n: &str| fr.env().contains_key(n) || fr.widths().contains_key(n);
                    if ast_names_any(e, &local) {
                        return None;
                    }
                }
                self.selfdet_truth(e)
            }
        }
    }

    /// Fold a SIZE cast `N'(e)` / `W'(e)` — the ONE spelling the module-scope arm
    /// (`const_eval_cast`) and the constant-function arm (`eval_const_env`) share.
    ///
    /// §6.24.1 / §11.8.1: the operand runs at `max(its self width, N)` with the
    /// OPERAND's signedness and the result is delivered in N bits — which is exactly
    /// what the assignment funnel computes for a target of `(N, sign)`, so no second
    /// copy of the width rule exists.
    ///
    /// Declines outside the i64 constant domain, and both halves of that boundary were
    /// measured the hard way:
    ///
    ///   * `N > 64` — the operand still evaluates at 64 bits, so a carry past bit 63 is
    ///     lost. `65'(64'hFFFF_FFFF_FFFF_FFFF + 64'd1) >> 64` is 1 for iverilog AND
    ///     verilator and 0 here, so answering at all is a silent wrong.
    ///   * `N == 64` with an UNSIGNED result whose top bit is set — `coerce_int_width`
    ///     is the identity at 64, so the value escapes as a NEGATIVE i64 and
    ///     `(64'(64'hFFFF_FFFF_FFFF_FFFF) > 0)` answers 0 for iverilog's 1. This is the
    ///     same fit rule `const_placement_env` applies, and the same pre-existing
    ///     64-bit class ROADMAP §2 records for a bare literal — extending it through a
    ///     new syntax is not licensed by its already existing.
    pub(crate) fn const_size_cast(
        &self,
        target: &ast::CastTarget,
        operand: &ast::Expr,
        fr: &ConstFrame,
        depth: u32,
    ) -> Option<i64> {
        let n = u32::try_from(self.cast_size_bits(target)?).ok()?;
        if !(1..=64).contains(&n) {
            return None;
        }
        let sg = self.const_signed_env(operand, fr.widths());
        let v = self.eval_const_assign(operand, fr, depth, Some((n, sg)))?;
        if n == 64 && !sg && v < 0 {
            return None;
        }
        Some(v)
    }

    pub(crate) fn const_eval_cast(
        &self,
        target: &ast::CastTarget,
        operand: &ast::Expr,
    ) -> Option<i64> {
        match target {
            // `int'(e)`, `byte'(e)`, … — a fixed (width, signedness) target, so the
            // runtime cast's resize-then-sign-stamp is exactly `coerce_int_width`.
            // `real'` yields None from the shared table (no integral value here).
            // ⚠️ Routing this through `const_size_cast` too was tried and measured
            // EQUIVALENT — 13 discriminators (division, modulo and shift under a
            // narrowing `byte'`/`shortint'`, a 64-bit operand, a wrapping sum) all
            // agree, because the final coercion to a FIXED width already reduces
            // whatever the wider evaluation produced. Left alone because a prim cast
            // also changes the 4-state-ness and the domain (`real'` has no integral
            // value at all), and unifying on the strength of "no cell separates them"
            // would be a claim this domain has not earned.
            ast::CastTarget::Prim(p) => {
                let (w, s, _) = cast_prim_wsign(*p)?;
                let v = match self.const_eval_in_scope(operand) {
                    Some(v) => v,
                    // `int'(<real>)` IS the conversion the source asked for, and this
                    // cast node is the context boundary §6.24.1 names — so the operand
                    // folds WHOLE in the real domain and only the rounded result
                    // crosses into the integer one. The integer domain is asked first
                    // and this is a FALLBACK, so a wholly integral operand keeps its
                    // own width semantics (`int'(4'd15 + 4'd1)` stays the 4-bit 0).
                    // `real'` never reaches here: `cast_prim_wsign` has no integral
                    // (width, sign) for it and returned above.
                    None => self.const_int_via_real(operand)?,
                };
                Some(coerce_int_width(v, w, s))
            }
            // `N'(e)`: the operand runs at `max(its self width, N)` and the result is
            // delivered in N bits with the operand's signedness (IEEE §6.24.1 /
            // §11.8.1 — §4.5.316 pinned that rule against iverilog). That is exactly
            // what the assignment funnel computes for a target of `(N, sign)`, and it
            // is the SAME routing `eval_const_env`'s Cast arm already uses, so the
            // two spellings of one construct cannot answer differently.
            //
            // This used to fold the operand in the width-UNLIMITED domain and then
            // truncate, which is unsound on top of an un-narrowed operand
            // (`4'((4'd8+4'd8)/4'd3)` divided 16 by 3 and answered 5 where the 4-bit
            // sum is 0, so SV — and iverilog — answer 0). It compensated by folding
            // only where the signed and unsigned readings AGREE, i.e. a non-negative
            // value with the target's sign bit clear, which declined every ordinary
            // narrowing cast (`8'(255)`, `4'(9)`, `8'(P)`, `64'(-1)`, `1'(3)` — all
            // values iverilog prints). Sizing the operand first removes both: the
            // truncation is no longer an approximation, so the sign no longer has to
            // be guessed away.
            ast::CastTarget::Size(_) | ast::CastTarget::Named(_) => {
                self.const_size_cast(target, operand, &ConstFrame::new(), 0)
            }
            // `signed'`/`unsigned'` PRESERVE the operand's width, which this domain
            // does not track (`signed'(4'hF)` is −1 at 4 bits and 15 at 32), so it
            // stays loud — EXCEPT over a size cast, whose width the cast itself
            // names (§3 ⑤ ⓓ: the parser spells a typedef cast `t'(e)` / `cap_t'(e)`
            // as `signed'|unsigned'(W'(e))`, so `parameter cap_t ROOT_CAP_TX =
            // cap_t'(ROOT_DECODED_CAP_TX)` is this shape). The inner cast delivers
            // W bits with the operand's sign; the outer re-reads those W bits with
            // the named sign. `const_expr_signed` already answers `signed` for it.
            ast::CastTarget::Signing { signed } => self.signing_over_size_cast(operand, *signed),
            // §3 ⑤ⓕ: the per-instance twin of the arm above — `T'(e)` where `T` is
            // an overridable `parameter type`, which the parser spells
            // `signing_param'(T$w'(e))`. Same body with the sign folded from `T$s`
            // instead of read off the node; `?` DECLINES the whole fold when `T$s`
            // does not resolve here, which is where this domain was before.
            ast::CastTarget::SigningParam { shape_param } => {
                let signed = self.cast_shape_signed(shape_param)?;
                self.signing_over_size_cast(operand, signed)
            }
        }
    }

    /// The one body of a signing cast over a size cast (`Signing` and `SigningParam` above,
    /// PARSE-1 L14): the inner size cast's W bits re-read with `signed`. `(1i64 << w) - 1`
    /// is PRE's text, so its dev-profile overflow at w = 63 is PRE's too (release wraps).
    fn signing_over_size_cast(&self, operand: &ast::Expr, signed: bool) -> Option<i64> {
        match &operand.kind {
            ast::ExprKind::Cast {
                target: inner @ (ast::CastTarget::Size(_) | ast::CastTarget::Named(_)),
                expr: inner_e,
            } => {
                let w = u32::try_from(self.cast_size_bits(inner)?).ok()?;
                if !(1..=63).contains(&w) {
                    return None;
                }
                let v = self.const_size_cast(inner, inner_e, &ConstFrame::new(), 0)?;
                let mask = (1i64 << w) - 1;
                let u = v & mask;
                Some(if signed && (u >> (w - 1)) & 1 == 1 {
                    u | !mask
                } else {
                    u
                })
            }
            _ => None,
        }
    }
}

/// Unary `+ - ~` on a FOLDED operand (each core folds the operand itself, as it did):
/// `-` declines at the 64-bit minimum (`checked_neg`). CTX masks the result to its context.
#[inline]
pub(crate) fn unary_i64(op: ast::UnOp, v: i64) -> Option<i64> {
    debug_assert!(matches!(
        op,
        ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot
    ));
    match op {
        ast::UnOp::Plus => Some(v),
        ast::UnOp::Minus => v.checked_neg(),
        _ => Some(!v),
    }
}

#[cfg(test)]
mod tests {
    //! PRE-text equality (PLAN §5 ; premise C6: these replicas are `#[cfg(test)]`, which
    //! `value_arms_home`, `value_rules_home` and pins.py skip ; nothing here calls a
    //! structure-ratchet callee).
    use super::*;
    use diag::LogEvent;

    struct Quiet;
    impl LogSink for Quiet {
        fn emit(&self, _: LogEvent) {}
    }

    const SP: ast::Span = ast::Span { lo: 0, hi: 0 };

    fn grid() -> Vec<i64> {
        let mut g = vec![
            0,
            1,
            -1,
            2,
            -2,
            i64::MIN,
            i64::MAX,
            i64::MIN + 1,
            i64::MAX - 1,
        ];
        for k in 0..63 {
            g.push(1i64 << k);
            g.push(-(1i64 << k));
            g.push((1i64 << k) - 1);
        }
        g.push(0x5555_5555_5555_5555);
        g.push(0xAAAA_AAAA_AAAA_AAAAu64 as i64);
        g
    }

    /// PRE's three-arm text (const_fn.rs :209-213 and :829-833 at 257c4990 ; fold.rs
    /// :242-244 before its mask).
    fn pre_unary(op: ast::UnOp, v: i64) -> Option<i64> {
        match op {
            ast::UnOp::Plus => Some(v),
            ast::UnOp::Minus => v.checked_neg(),
            _ => Some(!v),
        }
    }

    #[test]
    fn unary_i64_is_pre_text() {
        let mut n = 0;
        for op in [ast::UnOp::Plus, ast::UnOp::Minus, ast::UnOp::BitNot] {
            for v in grid() {
                assert_eq!(unary_i64(op, v), pre_unary(op, v), "{op:?} {v}");
                n += 1;
            }
        }
        assert_eq!(unary_i64(ast::UnOp::Minus, i64::MIN), None);
        assert!(n > 500, "{n}");
    }

    fn lit(kind: ast::IntLitKind, raw: String) -> ast::Expr {
        ast::Expr {
            kind: ast::ExprKind::IntLit { kind, raw },
            span: SP,
        }
    }

    /// `<w>'(<operand literal>)`.
    fn size_cast(w: u32, operand: ast::Expr) -> ast::Expr {
        ast::Expr {
            kind: ast::ExprKind::Cast {
                target: ast::CastTarget::Size(Box::new(lit(
                    ast::IntLitKind::Decimal,
                    w.to_string(),
                ))),
                expr: Box::new(operand),
            },
            span: SP,
        }
    }

    /// PRE's twin body (const_fn.rs :669-694 / :702-727 at 257c4990), `signed` read as a value.
    fn pre_signing(el: &Elaborator<'_>, operand: &ast::Expr, signed: bool) -> Option<i64> {
        match &operand.kind {
            ast::ExprKind::Cast {
                target: inner @ (ast::CastTarget::Size(_) | ast::CastTarget::Named(_)),
                expr: inner_e,
            } => {
                let w = u32::try_from(el.cast_size_bits(inner)?).ok()?;
                if !(1..=63).contains(&w) {
                    return None;
                }
                let v = el.const_size_cast(inner, inner_e, &ConstFrame::new(), 0)?;
                let mask = (1i64 << w) - 1;
                let u = v & mask;
                Some(if signed && (u >> (w - 1)) & 1 == 1 {
                    u | !mask
                } else {
                    u
                })
            }
            _ => None,
        }
    }

    fn signing_rows(widths: &[u32]) -> (usize, usize) {
        let sink = Quiet;
        let el = Elaborator::new(&sink);
        let (mut rows, mut some) = (0, 0);
        for &w in widths {
            for v in grid() {
                for raw in [
                    format!("64'h{:x}", v as u64),
                    format!("64'sh{:x}", v as u64),
                ] {
                    let op = size_cast(w, lit(ast::IntLitKind::Sized, raw.clone()));
                    for signed in [false, true] {
                        let got = el.signing_over_size_cast(&op, signed);
                        assert_eq!(
                            got,
                            pre_signing(&el, &op, signed),
                            "w={w} {raw} signed={signed}"
                        );
                        rows += 1;
                        some += usize::from(got.is_some());
                    }
                }
            }
        }
        (rows, some)
    }

    /// w = 63 is out: `(1i64 << 63) - 1` overflows in the dev profile, in PRE's text and here
    /// alike (PARSE-1 L14 ; the release lane cells k_sg_s63 / k_sg_u63 / k_sp_s63 hold it).
    #[test]
    fn signing_over_size_cast_is_pre_text() {
        let widths: Vec<u32> = (0..=62).chain([64, 65]).collect();
        let (rows, some) = signing_rows(&widths);
        assert!(rows > 30_000 && some > 15_000, "rows={rows} some={some}");
        // The fixture folds: `signed'(4'(64'hf))` is -1, `unsigned'(..)` 15 ; a non-cast
        // operand declines.
        let sink = Quiet;
        let el = Elaborator::new(&sink);
        let op = size_cast(4, lit(ast::IntLitKind::Sized, "64'hf".into()));
        assert_eq!(el.signing_over_size_cast(&op, true), Some(-1));
        assert_eq!(el.signing_over_size_cast(&op, false), Some(15));
        let bare = lit(ast::IntLitKind::Sized, "4'hf".into());
        assert_eq!(el.signing_over_size_cast(&bare, true), None);
    }

    /// Compiled only without debug assertions, and the gate never builds it: ci.yml and
    /// CONTRIBUTING run `cargo nextest run --workspace` in the dev profile and nothing runs the
    /// tests with `--release`, so this test cannot fail there (a compile error in it would pass
    /// too). What holds w = 63 is the release lane cells k_sg_s63, k_sg_u63 and k_sp_s63
    /// (W/homes lanes val1_lane).
    #[cfg(not(debug_assertions))]
    #[test]
    fn signing_over_size_cast_is_pre_text_w63() {
        let (rows, some) = signing_rows(&[63]);
        assert!(rows > 500 && some > 200, "rows={rows} some={some}");
    }
}
