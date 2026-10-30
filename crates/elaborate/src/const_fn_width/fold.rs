//! The width-aware constant walk as a TRI-STATE fold (§5.2 Q-XFALLBACK).
//!
//! [`Elaborator::eval_const_env_at`], [`Elaborator::eval_const_env_self`],
//! [`Elaborator::eval_const_shift_count`] and [`Elaborator::eval_const_assign`] stay the crate's
//! entries in `const_fn_width.rs`; the first, second and fourth are `fold_*(..).legacy()` over
//! the bodies here, so every existing caller reads exactly the `Option` it read before.
//!
//! ⭐ THE CONTRACT (the interface Q-AESINK builds on). [`CFold`] is `Val(v)` exactly where
//! the old walk answered `Some(v)`, and `Four` or `No` exactly where it answered `None` (G1,
//! G2). `Four { v, x, why }` says more than `None`: bit `i` of `x` is IEEE `x` at the region
//! width `why.ctx_w` (G3) — an x ORIGIN is a `/` or `%` whose divisor is 0 at the region width,
//! or a `**` whose base is 0 there under a negative exponent (IEEE 1800-2017 §11.4.3, Table
//! 11-4), and the mask travels only along CONTEXT-DETERMINED positions: unary `+ - ~`, the
//! arithmetic operators (any x bit makes every bit x, §11.4.3), `& | ^ ~^` bit by bit (§11.4.8:
//! a known 0 under `&`, a known 1 under `|` clears that bit), the shifted operand of a shift by
//! a known count (§11.4.10: the bits move; a count at or past the region width moves them all
//! out), the `**` base and the taken arm of a known condition. `x == 0` is an ABSORBED x: the
//! value `v` is then exact (an operator over an x-free `Four` computes as over a `Val`), but the
//! node stays `Four` — sticky — so the legacy entries never gain a `Some`. `v` under an x bit
//! is 0 and means nothing: no consumer may bind `Four.v` (G4 — Q-XCORE's value arm owns that).
//! Every SELF-determined position is a boundary that calls the legacy entry (G5): a comparison,
//! `!`, a reduction, `$clog2`, a ternary condition, a shift count, a `**` exponent, a cast, a
//! call, a concatenation, a select, every leaf. A consumer refuses only with `x_in(<its
//! width>)` and [`Elaborator::x_certified`] (G6) — never on `No`.

use super::*;

/// What made a region `x` (the only two origins this walk proves).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XKind {
    DivZero,
    ModZero,
    PowZeroNeg,
}

/// Where and at which width an x origin was decided: the `/` `%` `**` node's span, the
/// region width (1..=64) its divisor / base was 0 at, and `own` — the x bits of the node
/// carrying this `XWhy` that this origin is known to make x.
///
/// ⚠️ `own` is what lets a cause clause NAME the origin: an origin whose x was absorbed
/// (`(X % 0) & 16'h0`), shifted out or truncated away by the target no longer owns a bit, and
/// naming it would name an operation that did not make the x the consumer refused on
/// (§5.2 Q-XFALLBACK lens C-W1). `own` is a SUBSET of the node's x mask — one origin is kept
/// per node, so bits another origin made are not in it — and `own == 0` means "no origin is
/// known to reach a surviving x bit": [`Elaborator::x_reason`] then gives the clause without
/// an origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct XWhy {
    pub(crate) kind: XKind,
    pub(crate) at: ast::Span,
    pub(crate) ctx_w: u32,
    pub(crate) own: u64,
}

impl XWhy {
    /// The same origin owning `own` instead.
    pub(crate) fn owning(self, own: u64) -> Self {
        XWhy { own, ..self }
    }
}

/// The cause a binary node keeps from its operands: the first operand (left, then right)
/// whose origin still owns one of the node's x bits (`own_a` / `own_b`, already reduced to
/// the bits that survive the operator), else the left one's with nothing owned.
fn pick_why(a: Option<(XWhy, u64)>, b: Option<(XWhy, u64)>) -> XWhy {
    match (a, b) {
        (Some((w, o)), _) if o != 0 => w.owning(o),
        (_, Some((w, o))) if o != 0 => w.owning(o),
        (Some((w, _)), _) | (None, Some((w, _))) => w.owning(0),
        (None, None) => unreachable!("a Four operand carries a why"),
    }
}

/// The width-aware walk's answer — see the module doc for the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CFold {
    /// Exactly the legacy entry's `Some(v)`.
    Val(i64),
    /// The legacy entry's `None`, with x evidence: bit `i` of `x` is x at `why.ctx_w`.
    Four { v: i64, x: u64, why: XWhy },
    /// The legacy entry's `None`, no x evidence.
    No,
}

impl From<Option<i64>> for CFold {
    fn from(o: Option<i64>) -> Self {
        o.map_or(CFold::No, CFold::Val)
    }
}

impl CFold {
    /// The legacy entries' answer: `Some` only for `Val`.
    pub(crate) fn legacy(self) -> Option<i64> {
        match self {
            CFold::Val(v) => Some(v),
            _ => None,
        }
    }

    /// Does an x bit lie inside the low `w` bits?
    pub(crate) fn x_in(self, w: u32) -> bool {
        matches!(self, CFold::Four { x, .. } if x & ones(w) != 0)
    }

    /// The x mask of a `Four` (0 for `Val` / `No`).
    pub(crate) fn x_bits(self) -> u64 {
        match self {
            CFold::Four { x, .. } => x,
            _ => 0,
        }
    }

    pub(crate) fn why(self) -> Option<XWhy> {
        match self {
            CFold::Four { why, .. } => Some(why),
            _ => None,
        }
    }
}

/// The low `w` bits set (`ones(0)` = 0, `ones(64+)` = all).
pub(crate) fn ones(w: u32) -> u64 {
    if w >= 64 {
        u64::MAX
    } else {
        (1u64 << w) - 1
    }
}

/// Where the x bits of a shifted `Four` land (§11.4.10), mirroring the value branch the
/// shift takes. ⚠️ The count test is NAMED: a count at or past the region width moves every
/// bit out, the x bits included (`(X << 32) + 5` into an `int` is 5 in all three tools).
fn shift_x(op: ast::BinOp, x: u64, b: i64, ctx_w: u32, ctx_signed: bool) -> u64 {
    use ast::BinOp as B;
    let m = ones(ctx_w);
    let x = x & m;
    let all_out = b < 0 || b >= i64::from(ctx_w);
    match op {
        B::Shl | B::AShl => {
            if all_out {
                0
            } else {
                (x << b) & m
            }
        }
        B::AShr if ctx_signed && ctx_w >= 1 && (x >> (ctx_w - 1)) & 1 == 1 => {
            // A signed region's `>>>` refills from its sign bit, which is x.
            if all_out {
                m
            } else {
                (x >> b) | (m & !(m >> b))
            }
        }
        _ => {
            if all_out {
                0
            } else {
                x >> b
            }
        }
    }
}

impl Elaborator<'_> {
    /// The body of [`Self::eval_const_env_at`], tri-state (see the module doc).
    pub(crate) fn fold_ctx(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
        ctx_w: u32,
        ctx_signed: bool,
    ) -> CFold {
        use ast::ExprKind as K;
        use CFold::{Four, No, Val};
        // At ≥64 bits the i64 domain IS the SV width, and with no known context
        // there is nothing to narrow to — so masking becomes the identity. The walk
        // still has to happen: a SELF-determined sub-position (a `$clog2` argument,
        // a shift count) is sized by itself no matter how wide the context is, and
        // an early return here left `longint t = $clog2(4'd15 + 4'd15)` at 5 while
        // every narrower target already gave iverilog's 4.
        let masking = ctx_w > 0 && ctx_w < 64;
        // §11.8.1: the signedness of a context-determined expression is decided
        // ONCE for the whole context and pushed down — if ANY operand is unsigned,
        // every operand is reinterpreted as unsigned. Recomputing it per node
        // sign-extended a signed sub-expression under an unsigned parent, so
        // `bit [7:0] r = (b + b) / u` with a signed `byte b = 100` divided −56 by 2
        // and stored 228 where SV (and vita's own runtime) give 100.
        let mask = |v: i64| {
            if masking {
                Self::const_mask(v, ctx_w, ctx_signed)
            } else {
                v
            }
        };
        // The region mask. A `Four` exists only where an origin fired, i.e. at
        // 1 <= ctx_w <= 64, and every context-determined child shares the root's
        // `ctx_w`, so `m` is the same for every `Four` this node can see.
        let m = ones(ctx_w);
        let origin_width = (1..=64).contains(&ctx_w);
        // A `Four` in canonical form: 0 under every x bit, masked like a value.
        let four = |v: i64, x: u64, why: XWhy| Four {
            v: mask(((v as u64) & !x) as i64),
            x,
            why,
        };
        // An arithmetic result is all-x when any operand has an x bit (§11.4.3), so an
        // origin that owns a live bit owns every bit of the result; one that owns none
        // still owns none (the x came from an origin this node does not name).
        let all_if_live = |why: XWhy| why.owning(if why.own & m != 0 { m } else { 0 });
        // G-EVAL: the old walk stopped at the first operand that declined (`?`); this one
        // continues after a `Four` to combine masks, so the sibling is a NEW evaluation. A
        // call can reach the interpreter's interior-mutable state (`const_call_fn`'s
        // save / restore, the decl window's `decl_enter` / `decl_reentered`), so a sibling
        // holding one is not evaluated: the node is `No` (= the old `None`, no evidence).
        let unevaluable_after = |a: CFold, sib: &ast::Expr| {
            matches!(a, No) || (matches!(a, Four { .. }) && Self::subtree_has_call(sib))
        };
        match &e.kind {
            K::Paren { inner } => self.fold_ctx(inner, env, envw, depth, ctx_w, ctx_signed),
            // §5.7.1: a fill is the fill bit replicated to the CONTEXT width — `ctx_w`
            // here is the region width the parent computed as `max(ctx, every
            // self-determined sibling)`, so `4'd8 - '1` sees a 4-bit all-ones (both
            // oracles 9) and a lone fill in a self-determined position (`$clog2('1)`,
            // `'1 >> 1`, `localparam U = '1;`) is ONE bit. Before this arm the leaf
            // fell to the plain twin's `const_eval_i64_lit`, whose fill is the i64
            // lane's hard 32 (ROADMAP §2 🆕 C). An x/z fill declines (no unknown
            // plane here), which is the loud the callers already had — an x LITERAL is
            // not one of this walk's x origins.
            K::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => {
                crate::const_eval::fill_to_i64(*kind, raw, ctx_w.max(1)).into()
            }
            K::Unary {
                op: op @ (ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot),
                operand,
            } => match (
                op,
                self.fold_ctx(operand, env, envw, depth, ctx_w, ctx_signed),
            ) {
                (_, No) => No,
                (ast::UnOp::Plus, Val(v)) => Val(mask(v)),
                (ast::UnOp::Minus, Val(v)) => v.checked_neg().map(mask).into(),
                (_, Val(v)) => Val(mask(!v)),
                (ast::UnOp::Plus, Four { v, x, why }) => four(v, x, why),
                // Arithmetic: an x bit anywhere makes every bit x (§11.4.3); an absorbed
                // operand negates exactly.
                (ast::UnOp::Minus, Four { v, x, why }) => {
                    if x & m != 0 {
                        four(0, m, all_if_live(why))
                    } else {
                        v.checked_neg().map_or(No, |n| four(n, 0, why.owning(0)))
                    }
                }
                // `~` is bitwise (§11.4.8): an x bit stays x, a known bit flips.
                (_, Four { v, x, why }) => four(!v, x, why),
            },
            K::Binary { op, lhs, rhs } => {
                use ast::BinOp as B;
                match op {
                    B::Add
                    | B::Sub
                    | B::Mul
                    | B::Div
                    | B::Mod
                    | B::BitAnd
                    | B::BitOr
                    | B::BitXor
                    | B::BitXnor => {
                        let a = self.fold_ctx(lhs, env, envw, depth, ctx_w, ctx_signed);
                        if unevaluable_after(a, rhs) {
                            return No;
                        }
                        let b = self.fold_ctx(rhs, env, envw, depth, ctx_w, ctx_signed);
                        // ⚠️ `/` and `%` are the two SIGN-sensitive members of this arm
                        // (the rest are bit-pattern ops), so at 64+ unsigned bits they
                        // divide in u64: both oracles fold
                        // `64'hFFFF_FFFF_FFFF_FFFF % 64'd10` to 5, and the signed
                        // reading answered −1.
                        let exact = |a: i64, b: i64| -> Option<i64> {
                            if const_i64_is_unsigned_at(ctx_w, ctx_signed) && b != 0 {
                                match op {
                                    B::Div => return Some(mask(((a as u64) / (b as u64)) as i64)),
                                    B::Mod => return Some(mask(((a as u64) % (b as u64)) as i64)),
                                    _ => {}
                                }
                            }
                            Some(mask(const_binop(*op, a, b)?))
                        };
                        let ((va, xa), (vb, xb), (wa, wb)) = match (a, b) {
                            (No, _) | (_, No) => return No,
                            (Val(a), Val(b)) => {
                                // ORIGIN D1: a divisor that is 0 at the region width makes
                                // the quotient / remainder x (IEEE 1800-2017 §11.4.3). The
                                // old walk declined right here (`const_binop` refuses a
                                // zero divisor), so this is a `None` it already returned.
                                if matches!(op, B::Div | B::Mod) && b == 0 && origin_width {
                                    let kind = if matches!(op, B::Div) {
                                        XKind::DivZero
                                    } else {
                                        XKind::ModZero
                                    };
                                    return four(
                                        0,
                                        m,
                                        XWhy {
                                            kind,
                                            at: e.span,
                                            ctx_w,
                                            own: m,
                                        },
                                    );
                                }
                                return exact(a, b).into();
                            }
                            (Four { v, x, why }, Val(b)) => ((v, x), (b, 0), (Some(why), None)),
                            (Val(a), Four { v, x, why }) => ((a, 0), (v, x), (None, Some(why))),
                            (
                                Four { v, x, why },
                                Four {
                                    v: v2,
                                    x: x2,
                                    why: why2,
                                },
                            ) => ((v, x), (v2, x2), (Some(why), Some(why2))),
                        };
                        // The cause follows the x bits that SURVIVE this operator: each
                        // operand's origin keeps only the bits of its `own` the result is
                        // still x at (C-W1: `(X % 0) & 16'h0 | (Y / 0)` is the `/`'s x).
                        let own_of = |w: Option<XWhy>, keep: u64| w.map(|w| (w, w.own & keep));
                        match op {
                            // §11.4.8 bit by bit: a known 0 under `&`, a known 1 under `|`
                            // decides the bit whatever the other side holds.
                            B::BitAnd | B::BitOr => {
                                let (ua, ub) = (va as u64, vb as u64);
                                let (ka, kb) = if matches!(op, B::BitAnd) {
                                    (ua & !xa & m, ub & !xb & m) // known 1s
                                } else {
                                    (!ua & !xa & m, !ub & !xb & m) // known 0s
                                };
                                let x = (xa & xb) | (xa & kb) | (xb & ka);
                                let v = if matches!(op, B::BitAnd) {
                                    va & vb
                                } else {
                                    va | vb
                                };
                                // A bit of `a`'s x survives where `b` is x or does not decide
                                // the bit (not a known 0 under `&`, not a known 1 under `|`).
                                let why = pick_why(own_of(wa, xb | kb), own_of(wb, xa | ka));
                                four(v, x, why)
                            }
                            B::BitXor | B::BitXnor => {
                                let why = pick_why(own_of(wa, m), own_of(wb, m));
                                let v = if matches!(op, B::BitXor) {
                                    va ^ vb
                                } else {
                                    !(va ^ vb)
                                };
                                four(v, xa | xb, why)
                            }
                            // + - * / %: any x bit inside the region makes every bit x
                            // (§11.4.3) — `* 0`, `X - X` and `0 / X` are NOT absorbers.
                            // Operands with no x bit (absorbed) compute exactly, and the
                            // node stays `Four` (sticky).
                            _ => {
                                let why = pick_why(own_of(wa, m), own_of(wb, m));
                                if (xa | xb) & m != 0 {
                                    four(0, m, all_if_live(why))
                                } else {
                                    exact(va, vb).map_or(No, |v| four(v, 0, why.owning(0)))
                                }
                            }
                        }
                    }
                    // LEFT operand takes the context; the shift COUNT / exponent is
                    // self-determined (`(8'd200+8'd100) >> 2` is 11 at 8 bits and
                    // 75 at 32 — the count itself is 2 either way).
                    B::Shl | B::Shr | B::AShl | B::AShr | B::Pow => {
                        let a = self.fold_ctx(lhs, env, envw, depth, ctx_w, ctx_signed);
                        if unevaluable_after(a, rhs) {
                            return No;
                        }
                        // Pow goes through the one shared exponent helper — which
                        // reports the exponent's SIGNEDNESS with its value, because
                        // that is what decides whether the IEEE negative-exponent
                        // table applies at all. A shift count needs no such thing.
                        if matches!(op, B::Pow) {
                            let Some((b, sg)) =
                                self.const_pow_exponent_selfdet(rhs, env, envw, depth)
                            else {
                                return No;
                            };
                            return match a {
                                // ORIGIN D2: a base that is 0 at the region width under a
                                // negative exponent is x (§11.4.3, Table 11-4). The old
                                // walk declined here (`const_pow` answers None).
                                Val(0) if sg && b < 0 && origin_width => four(
                                    0,
                                    m,
                                    XWhy {
                                        kind: XKind::PowZeroNeg,
                                        at: e.span,
                                        ctx_w,
                                        own: m,
                                    },
                                ),
                                Val(a) => const_pow(a, b, sg).map(mask).into(),
                                // `X ** 0` and `X ** 1` are x too: §11.4.3's any-x rule
                                // over Table 11-4. An absorbed base computes exactly.
                                Four { v, x, why } => {
                                    if x & m != 0 {
                                        four(0, m, all_if_live(why))
                                    } else {
                                        const_pow(v, b, sg)
                                            .map_or(No, |r| four(r, 0, why.owning(0)))
                                    }
                                }
                                No => No,
                            };
                        }
                        let Some(b) = self.eval_const_shift_count(rhs, env, envw, depth) else {
                            return No;
                        };
                        // With a KNOWN context width a shift is exact on the bit
                        // pattern, so it no longer has to decline. `const_binop`
                        // refuses a logical `>>` of a negative value because the
                        // result depends on the operand width — but that width is
                        // right here, and refusing turned `bit [7:0] t =
                        // (8'sd100 + 8'sd100) >> 1` (iverilog 100) into a LOUD
                        // reject once the operands started masking to −56.
                        // ⚠️ A logical `>>` of a 64+ unsigned value: `const_binop`
                        // DECLINES for a negative `a` (its result depends on the
                        // operand width, which it cannot see) and the caller then
                        // goes LOUD — `localparam L = 64'hFFFFFFFF00000000 >> 32;`
                        // was rejected where both oracles fold it. Here the width IS
                        // known, so the shift is exact on the bit pattern.
                        // ⚠️ BOTH shifts. §11.4.10 makes `>>>` arithmetic only when its
                        // LEFT operand is signed, and §11.6.1 has already converted that
                        // operand to the expression's type — which is unsigned here. Both
                        // oracles agree even when the left operand is a DECLARED-signed
                        // leaf: `(byte signed P = -100) >>> 60` compared against 64'd100
                        // takes the unsigned branch. Leaving `>>>` out did not merely miss
                        // a fix — the comparison route above UNMASKED it, turning 14
                        // measured cells from correct to silently wrong.
                        let shifted = |a: i64| -> Option<i64> {
                            if const_i64_is_unsigned_at(ctx_w, ctx_signed)
                                && matches!(op, B::Shr | B::AShr)
                            {
                                if !(0..64).contains(&b) {
                                    return Some(0);
                                }
                                return Some(mask(((a as u64) >> b) as i64));
                            }
                            if masking && matches!(op, B::Shl | B::AShl | B::Shr) {
                                if !(0..64).contains(&b) {
                                    return Some(0); // every bit shifted out
                                }
                                let wmask = (1u64 << ctx_w) - 1;
                                let bits = a as u64 & wmask;
                                let r = if matches!(op, B::Shr) {
                                    bits >> b
                                } else {
                                    bits << b
                                };
                                return Some(mask((r & wmask) as i64));
                            }
                            Some(mask(const_binop(*op, a, b)?))
                        };
                        match a {
                            Val(a) => shifted(a).into(),
                            // The mask moves with the bits; a value branch that declines
                            // leaves no evidence.
                            // The origin's own bits move with the bits (a refill from an x
                            // sign bit is the sign bit's origin's).
                            Four { v, x, why } => shifted(v).map_or(No, |v| {
                                let own = shift_x(*op, why.own, b, ctx_w, ctx_signed);
                                four(v, shift_x(*op, x, b, ctx_w, ctx_signed), why.owning(own))
                            }),
                            No => No,
                        }
                    }
                    // A comparison's operands size against EACH OTHER, not against
                    // the surrounding context, and the result is a 1-bit 0/1 that
                    // no masking may touch. (`binop_result_is_context_determined`
                    // is the shared statement of that split; this arm is its
                    // `false` side and the two above are its `true` side.) A
                    // BOUNDARY: its operands go through the legacy entry.
                    _ => {
                        debug_assert!(!binop_result_is_context_determined(*op));
                        (|| -> Option<i64> {
                            // ⚠️ Two comparison folds are WHOLE-NODE facts that this walk
                            // would shadow by recursing into the operands — a `string`
                            // equality and an x/z wildcard pattern. With NO local bindings
                            // there is nothing a module-scope resolution could shadow, so
                            // consult the shared owner first; the same conjunct
                            // `eval_const_env`'s delegation arm uses.
                            // ⚠️ Only the SPECIAL cases delegate, never the whole node: a
                            // plain `(4'd15 + 4'd1) > 0` must keep this walk's width-honest
                            // 0 and not the unlimited domain's 1.
                            if env.is_empty() && envw.is_empty() {
                                if let Some(v) = self.const_compare_special(*op, lhs, rhs) {
                                    return Some(v);
                                }
                                // An operand carrying x/z bits has no i64 reading; the
                                // wide domain answers the equalities and logical operators
                                // that are definite regardless (`4'b1x10 === 4'b1x10`,
                                // `4'b110x == 4'b0000`, `4'b1x1x && 1'b0`) and declines
                                // the rest, so the loud stays where the answer is x.
                                if crate::param_query::ast_holds_unknown_literal(lhs)
                                    || crate::param_query::ast_holds_unknown_literal(rhs)
                                {
                                    return self.selfdet_bits_i64(e);
                                }
                            }
                            // An UNKNOWN operand width means the pair cannot be sized,
                            // so nothing is masked (`w = 0`) rather than guessing 32.
                            let w = match (
                                self.const_self_width(lhs, envw),
                                self.const_self_width(rhs, envw),
                            ) {
                                (Some(a), Some(b)) => a.max(b),
                                _ => 0,
                            };
                            // The operands form their OWN context — they unify width
                            // AND sign with each other, not with the enclosing one.
                            let cs = self.const_signed_env(lhs, envw)
                                && self.const_signed_env(rhs, envw);
                            let a = self.eval_const_env_at(lhs, env, envw, depth, w, cs)?;
                            let b = self.eval_const_env_at(rhs, env, envw, depth, w, cs)?;
                            // ⚠️ ORDERING at 64+ unsigned bits must read u64. `const_binop`
                            // compares signed i64, which is right for every width masking
                            // can normalize and wrong for the one it cannot.
                            if const_i64_is_unsigned_at(w, cs) {
                                let (ua, ub) = (a as u64, b as u64);
                                match op {
                                    B::Lt => return Some((ua < ub) as i64),
                                    B::Le => return Some((ua <= ub) as i64),
                                    B::Gt => return Some((ua > ub) as i64),
                                    B::Ge => return Some((ua >= ub) as i64),
                                    _ => {}
                                }
                            }
                            const_binop(*op, a, b)
                        })()
                        .into()
                    }
                }
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // The condition is self-determined (a BOUNDARY: the legacy entry); the
                // arms take the context, and only the taken one is evaluated. One
                // carrying x/z bits is read for its truth in the wide domain (as the
                // `!` arm does), unless it names a constant-function local.
                let c = match self.eval_const_env_self(cond, env, envw, depth) {
                    Some(v) => v != 0,
                    None => {
                        let local = |n: &str| env.contains_key(n) || envw.contains_key(n);
                        if ast_names_any(cond, &local) {
                            return No;
                        }
                        let Some(t) = self.selfdet_truth(cond) else {
                            return No;
                        };
                        t
                    }
                };
                if c {
                    self.fold_ctx(then_e, env, envw, depth, ctx_w, ctx_signed)
                } else {
                    self.fold_ctx(else_e, env, envw, depth, ctx_w, ctx_signed)
                }
            }
            // `!x` (and the reductions) yield a 1-BIT result from a
            // self-determined operand — the operand must not take the surrounding
            // width, or `!(4'd15 + 4'd1)` sees 16 instead of the 4-bit 0. A BOUNDARY.
            K::Unary {
                op: op @ ast::UnOp::LogNot,
                operand,
            } => {
                let _ = op;
                match self.eval_const_env_self(operand, env, envw, depth) {
                    Some(v) => Val((v == 0) as i64),
                    // An operand carrying x/z bits: its TRUTH, through the wide
                    // domain, unless it names a constant-function local (whose
                    // value lives only in `env`).
                    None => {
                        let local = |n: &str| env.contains_key(n) || envw.contains_key(n);
                        if ast_names_any(operand, &local) {
                            return No;
                        }
                        self.selfdet_truth(operand).map(|t| !t as i64).into()
                    }
                }
            }
            // §11.4.14 REDUCTION — the arm this walk never had. It is what a declared
            // range bound holding a parameter SELECT reaches (`const_range_bound_fold`
            // redirects such a bound here), and what `!`, a ternary condition and a
            // `**` exponent reach for their self-determined operand — so
            // `wire [(|P[3:1])+2:0]` was ONE bit against both oracles' three, and
            // `wire [(!(|4'b1010))+2:0]` the same, at exit 0. A BOUNDARY.
            //
            // ⚠️ The operand's WIDTH decides `&`, `~&`, `^` and `~^`, and this walk sizes
            // a module parameter from `param_meta`, where value-INFERRED widths live —
            // the exact reading §4.5.373 built and reverted (`localparam W = 4'hF |
            // 4'h0;` is 32 bits there and 4 in both oracles). So a reduction with no
            // constant-function local in it takes the wide bit domain, whose names
            // carry DECLARED provenance only, and gives the same answer
            // `const_eval_in_scope` gives for the same text. Only an operand that
            // names a local (`env` / `envw` — a formal or a body local, whose width IS
            // its declaration) folds here, at that width.
            K::Unary {
                op:
                    op @ (ast::UnOp::RedAnd
                    | ast::UnOp::RedOr
                    | ast::UnOp::RedXor
                    | ast::UnOp::RedNand
                    | ast::UnOp::RedNor
                    | ast::UnOp::RedXnor),
                operand,
            } => (|| -> Option<i64> {
                let local = |n: &str| env.contains_key(n) || envw.contains_key(n);
                if !ast_names_any(operand, &local) {
                    return self.selfdet_bits_i64(e);
                }
                let w = self.const_self_width(operand, envw)?;
                if w == 0 || w > 64 {
                    return None;
                }
                let v = self.eval_const_env_self(operand, env, envw, depth)?;
                let all = if w == 64 { u64::MAX } else { (1u64 << w) - 1 };
                let bits = (v as u64) & all;
                let r = match op {
                    ast::UnOp::RedAnd | ast::UnOp::RedNand => bits == all,
                    ast::UnOp::RedOr | ast::UnOp::RedNor => bits != 0,
                    _ => bits.count_ones() % 2 == 1,
                };
                let inv = matches!(
                    op,
                    ast::UnOp::RedNand | ast::UnOp::RedNor | ast::UnOp::RedXnor
                );
                Some((r != inv) as i64)
            })()
            .into(),
            // A system function's ARGUMENT is self-determined — it does not take the
            // surrounding context — so it evaluates at its OWN width:
            // `$clog2(4'd15 + 4'd15)` is `$clog2(14)` = 4, not `$clog2(30)` = 5,
            // whatever the assignment target is. A BOUNDARY.
            K::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                self.const_clog2_selfdet(&args[0], env, envw, depth).into()
            }
            // A leaf enters the context at ITS OWN declared width: §11.6.1 extends
            // an operand to the context size, sign-extending only when the operand
            // is signed AND the whole expression is signed — otherwise ZERO. `env`
            // stores an 8-bit signed local already sign-extended into i64, so
            // without this a `byte a = -100` reached a 32-bit UNSIGNED context as
            // 0xFFFF_FF9C instead of 0x0000_009C, and `(a * 8'sd1) > LIMIT` answered
            // 1 where iverilog and vita's own runtime answer 0. A BOUNDARY (casts,
            // calls, concatenations, replications, selects and system calls are leaves).
            _ => (|| -> Option<i64> {
                let v = self.eval_const_env(e, env, envw, depth)?;
                // ⚠️ An exactly-64-bit UNSIGNED context normalizes its leaves too, even
                // though `masking` is off there. Without this the walk never establishes
                // the invariant the unsigned reading asserts: a NARROW SIGNED leaf
                // arrives sign-extended and the u64 route then reads the extension as
                // magnitude — `(logic signed [7:0] P = -100) / 8'sd3` compared at 64 bits
                // divided 0xFFFF_FFFF_FFFF_FF9C instead of the 156 both oracles use, and
                // an indexed part-select width built on one went from LOUD to a silently
                // wrong 22. `leaf_into_ctx` is exactly the §11.6.1 reinterpretation
                // (mask to the leaf's own width, sign-extend only if BOTH the leaf and
                // the context are signed), and it is a no-op for a leaf already ≥64 bits.
                if !masking && !const_i64_is_unsigned_at(ctx_w, ctx_signed) {
                    return Some(v);
                }
                let Some(lw) = self.const_self_width(e, envw) else {
                    return Some(v); // unknown leaf width ⇒ do not reinterpret
                };
                let ls = self.const_signed_env(e, envw);
                Some(Self::leaf_into_ctx(v, lw, ls, ctx_signed))
            })()
            .into(),
        }
    }

    /// G-EVAL's test: does `e` hold a call node anywhere (every child, concatenations and
    /// replications included — not `ast_contains_call`, which walks the const-fold children
    /// only)?
    fn subtree_has_call(e: &ast::Expr) -> bool {
        if matches!(e.kind, ast::ExprKind::Call { .. }) {
            return true;
        }
        let mut found = false;
        e.for_each_child(|_, c| found = found || Self::subtree_has_call(c));
        found
    }

    /// Reinterpret a leaf value that was produced at its own declared width `lw`
    /// for use in a context whose signedness is `ctx_signed`. The bit pattern is
    /// what the language carries: mask to `lw`, then sign-extend only when the leaf
    /// is signed AND the context is, else leave it zero-extended.
    fn leaf_into_ctx(v: i64, lw: u32, ls: bool, ctx_signed: bool) -> i64 {
        if lw == 0 || lw >= 64 {
            return v;
        }
        Self::const_mask(v, lw, ls && ctx_signed)
    }

    /// The body of [`Self::eval_const_env_self`]: [`Self::fold_ctx`] at `e`'s own width.
    pub(crate) fn fold_self(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> CFold {
        // `Some(0)` is a region made ONLY of unsized fills (`'1 + '1`): §5.7.1 gives
        // each fill its context, and with no sibling and no context that is one bit —
        // both oracles fold `{('1 + '1){8'hA5}}` to a zero count and reject it. `None`
        // is an UNKNOWN width and keeps the unlimited walk (ctx 0), as before.
        let w = match self.const_self_width(e, envw) {
            Some(0) => 1,
            Some(w) => w,
            None => 0,
        };
        let sg = self.const_signed_env(e, envw);
        self.fold_ctx(e, env, envw, depth, w, sg)
    }

    /// The body of [`Self::eval_const_shift_count`] (moved, unchanged: a shift count is a
    /// self-determined boundary and stays an `Option`).
    pub(crate) fn fold_shift_count(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        let v = match self.eval_const_env_self(e, env, envw, depth) {
            Some(v) => v,
            // A count the i64 lane cannot hold — a >64-bit literal tree (`32'd1 <<
            // 128'd70`, both oracles 0; `8'd1 << 128'h1_0000_0000_0000_0000`, both 0).
            // §11.4.10 reads it unsigned; one whose magnitude passes u64 shifts every
            // bit out, which a saturated count expresses. Literal trees only (the
            // evident-width rule below), and only what the wide domain folds.
            None => {
                if !Self::ctx_width_names_are_evident(e, envw) || !self.wide_domain_folds(e) {
                    return None;
                }
                return Some(
                    self.selfdet_bits_unsigned(e)
                        .map_or(i64::MAX, |u| i64::try_from(u).unwrap_or(i64::MAX)),
                );
            }
        };
        // ⚠️⚠️ THE WIDTH HAS TO BE A FACT. `const_self_width`'s name arm reads
        // `param_meta`, and for an untyped `parameter C = 3'sd1;` that records the
        // DEFAULT literal's 3 bits — which §6.20.2 replaces with the final override's
        // type the moment `#(.C(-3))` arrives (both oracles report `$bits(C)` = 32,
        // vita reports 3, and that is pre-existing). Reading the count SIGNED was
        // ACCIDENTALLY IMMUNE to it: −3 is out of `0..64` at any width, so the shift
        // collapsed to 0, which is the right answer for a 32-bit count. Reading it
        // unsigned at a stale 3 bits makes it **5**, and the shift really happens —
        // review measured 21 cells going correct → silent-wrong across all four
        // override channels, one of them a declared net width.
        //
        // So the mask is applied only where the width is EVIDENT: a literal and any
        // operator tree over literals, plus a name whose width came from `envw` (a
        // subprogram local's declared range, which no override can retype). A name that
        // has to be looked up in `param_meta` keeps the pre-slice behaviour. That costs
        // the module-scope NAMED count spelling and keeps every literal one, which is
        // where the row's reachability lives (a net width, a `generate` condition).
        //
        // ⚠️ This is [[a-default-is-not-a-fact]] again, and it is the third slice on
        // this axis to meet it. Widening past `envw` needs the declared-vs-inferred
        // provenance §2 row 14 stopped at — not a guess about which map is fresher.
        if !Self::ctx_width_names_are_evident(e, envw) {
            return Some(v);
        }
        // An unknown or >=64-bit self width leaves the value alone: `const_mask` is the
        // identity there, and inventing a width is what this domain must never do.
        match self.const_self_width(e, envw) {
            Some(w) if w > 0 => Some(Self::const_mask(v, w, false)),
            _ => Some(v),
        }
    }

    /// The body of [`Self::eval_const_assign`]: the RHS at `max(self, target)`, then the
    /// target's mask — on a `Four` too, whose x bits outside the target are dropped (the
    /// assignment truncates them away, so `(X << 8)` into 8 bits holds no x).
    pub(crate) fn fold_assign(
        &self,
        rhs: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
        target: Option<(u32, bool)>,
    ) -> CFold {
        let Some((tw, ts)) = target else {
            // An unknown target shape keeps the previous unlimited behavior.
            return self.eval_const_env(rhs, env, envw, depth).into();
        };
        // ⚠️ A RECORDED width of 0 (UNKNOWN) must still flow into `ctx` below, where
        // `max(self, 0)` leaves the RHS at its OWN self width. Degrading it to the
        // unlimited domain instead was measured to be a REGRESSION: the self width
        // is the right context whenever the true target is no wider than the RHS,
        // and dropping it turned `bit [1:0][3:0] tt; tt = 8'd100 * 8'd100;` from
        // iverilog's 16 into 10000. The defect the width-0 record actually caused
        // was upstream — a computable width being declined — and `const_decl_wsign`
        // is where that is fixed.
        // Unknown self-width ⇒ ctx 0 ⇒ no masking at all (the pre-slice behavior).
        // The final coercion to the target still applies: that width IS known.
        let ctx = match self.const_self_width(rhs, envw) {
            Some(w) => w.max(tw).min(64),
            None => 0,
        };
        // The evaluation context's sign comes from the OPERANDS (§11.8.1); the
        // TARGET's sign applies only to the final store, which is why
        // `bit signed [3:0] t = 4'd7 + 4'd1` computes 8 unsigned and stores −8.
        let cs = self.const_signed_env(rhs, envw);
        match self.fold_ctx(rhs, env, envw, depth, ctx, cs) {
            CFold::Val(v) => CFold::Val(Self::const_mask(v, tw, ts)),
            // The origin's own bits are truncated with the x bits: an origin whose x lies
            // only above the target no longer owns one (C-W1's truncation spelling).
            CFold::Four { v, x, why } => CFold::Four {
                v: Self::const_mask(v, tw, ts),
                x: x & ones(tw),
                why: why.owning(why.own & ones(tw)),
            },
            CFold::No => CFold::No,
        }
    }
}
