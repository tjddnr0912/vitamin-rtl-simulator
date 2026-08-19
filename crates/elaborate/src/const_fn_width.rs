//! Width-aware evaluation for the constant-function interpreter (§4.5.186).
//!
//! The interpreter used to evaluate every body expression in the width-UNLIMITED
//! i64 domain, so a narrow assignment TARGET never truncated: `bit [3:0] t; t =
//! 4'd15 + 4'd15;` produced 30 where SystemVerilog — and vita's OWN runtime, and
//! iverilog — produce 14. Six of nine measured shapes disagreed with the runtime
//! executing the very same function, which made `localparam W = f()` a silently
//! wrong parameter value at exit 0.
//!
//! IEEE 1800 §11.6: an assignment evaluates its right-hand side at
//! `max(self-determined width of the RHS, width of the target)`, and the operands
//! of a context-determined operator are all widened to that width; the result is
//! then assigned (truncating / sign-extending) to the target. Self-determined
//! positions — a shift's right operand, a ternary's condition, the operands of a
//! comparison — do NOT take the surrounding width.
//!
//! Everything here is elaborate-local: it changes const-function VALUES only, and
//! nothing about the IR's shape. The blast radius is wider than "narrow types",
//! though — ANY target of 63 bits or less now masks per assignment, so an
//! all-`int` function whose intermediate exceeds 32 bits also changes (and
//! becomes correct: `int r = 100000 * 100000` is 1410065408, not 10000000000).

use super::*;

/// `(width, signed)` of every name visible to the interpreter — the parallel twin
/// of its value env, so an assignment can find its target's declared shape.
pub(crate) type ConstWidths = std::collections::BTreeMap<String, (u32, bool)>;

impl Elaborator<'_> {
    /// Mask `v` into `w` bits, sign-extending when `signed` — the single place the
    /// interpreter narrows a value, shared by the operator masking and the final
    /// assignment so the two cannot disagree.
    pub(crate) fn const_mask(v: i64, w: u32, signed: bool) -> i64 {
        coerce_int_width(v, w, signed)
    }

    /// The SELF-determined width of `e` (IEEE Table 11-21). `None` when a leaf's
    /// width is unknown, which makes the caller keep the unlimited i64 behavior
    /// rather than invent a truncation.
    pub(crate) fn const_self_width(&self, e: &ast::Expr, envw: &ConstWidths) -> Option<u32> {
        use ast::ExprKind as K;
        match &e.kind {
            K::IntLit { kind, raw } => parse_int_literal(raw, *kind).map(|c| c.width),
            K::Paren { inner } => self.const_self_width(inner, envw),
            // A local/formal's declared width, else a module param's, else the
            // value-inferred 32 that `param_decl_width` never goes below.
            // A name the interpreter bound but whose shape it could NOT determine
            // is recorded with width 0 = UNKNOWN, and unknown must propagate as
            // None so no masking happens at all. Falling back to "32, unsigned"
            // invented a width for a 64-bit multi-packed local and silently
            // truncated it.
            K::Ident(p) if p.segments.len() == 1 => match envw.get(&p.segments[0].name).copied() {
                Some((0, _)) => None,
                Some((w, _)) => Some(w),
                None => Some(
                    self.walk_scopes(&p.segments[0].name, &self.param_meta)
                        .map_or(32, |(w, _)| w),
                ),
            },
            K::PkgScoped { pkg, name } => Some(
                self.pkg_const_meta
                    .get(&pkg.name)
                    .and_then(|m| m.get(&name.name))
                    .map_or(32, |(w, _)| *w),
            ),
            K::Unary { op, operand } => match op {
                // Context-determined unary: the operand's width.
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    self.const_self_width(operand, envw)
                }
                // Reductions and `!` are 1 bit.
                _ => Some(1),
            },
            K::Binary { op, lhs, rhs } => {
                use ast::BinOp as B;
                match op {
                    // Context-determined pair: the wider of the two.
                    B::Add
                    | B::Sub
                    | B::Mul
                    | B::Div
                    | B::Mod
                    | B::BitAnd
                    | B::BitOr
                    | B::BitXor
                    | B::BitXnor => Some(
                        self.const_self_width(lhs, envw)?
                            .max(self.const_self_width(rhs, envw)?),
                    ),
                    // Shifts and `**` take the LEFT operand's width; the right one
                    // is self-determined and does not widen the result.
                    B::Shl | B::Shr | B::AShl | B::AShr | B::Pow => {
                        self.const_self_width(lhs, envw)
                    }
                    // Comparisons / equality / logical: a 1-bit result.
                    _ => Some(1),
                }
            }
            K::Ternary { then_e, else_e, .. } => Some(
                self.const_self_width(then_e, envw)?
                    .max(self.const_self_width(else_e, envw)?),
            ),
            // A concatenation is SELF-determined: the sum of its operands' self
            // widths (§11.8.1), unsigned (`const_signed_env`'s fallthrough already
            // answers false). Without this arm a `**` exponent like
            // `({2'b10,2'b01} - 4'd8)` had no self width, and the Pow helper's
            // width-unknown refusal turned a correctly-folding cell LOUD.
            K::Concat { parts } => parts.iter().try_fold(0u32, |acc, p| {
                acc.checked_add(self.const_self_width(p, envw)?)
            }),
            // `$clog2`/`$bits` are 32-bit integers.
            K::SysCall { .. } => Some(32),
            K::Cast { target, expr } => match target {
                ast::CastTarget::Prim(p) => cast_prim_wsign(*p).map(|(w, _, _)| w),
                // `4'(e)` and `RPS'(e)` are two spellings of ONE construct —
                // `cast_size_bits` resolves both (a typedef/class Named still
                // yields None). Leaving `Named` unanswered made the Pow helper's
                // walk degrade on `3 ** (RPS'(2) - 4'd9)` where the width is
                // perfectly knowable.
                ast::CastTarget::Size(_) | ast::CastTarget::Named(_) => {
                    u32::try_from(self.cast_size_bits(target)?).ok()
                }
                ast::CastTarget::Signing { .. } => self.const_self_width(expr, envw),
            },
            // A call is as wide as its declared return type.
            K::Call { name, .. } if name.segments.len() == 1 => self
                .const_func_table
                .get(&name.segments[0].name)
                .and_then(|f| self.const_fn_ret_wsign(f))
                .map(|(w, _)| w),
            _ => None,
        }
    }

    /// Does `e` contain a function call anywhere? Used to keep a declared-range
    /// fold out of the constant-function interpreter's own recursion, and to let
    /// the env twin delegate a call-free subtree from inside a call (the
    /// delegation target restarts the call depth, so only a call-free subtree is
    /// safe there). Conservative by construction: `const_fold_children` descends
    /// exactly the kinds `const_eval_in_scope` folds, and a kind it does not list
    /// cannot reach a call in the delegation target either (a `Concat` part folds
    /// through the literal-only `const_eval_sized`).
    pub(crate) fn ast_contains_call(e: &ast::Expr) -> bool {
        matches!(&e.kind, ast::ExprKind::Call { .. })
            || Self::const_fold_children(e)
                .iter()
                .any(|c| Self::ast_contains_call(c))
    }

    /// Is `e` SIGNED? (IEEE §11.8.1: a context-determined operation is signed only
    /// when every one of its context-determined operands is.) Used to decide
    /// whether masking sign-extends, so `byte t = 8'sd100 + 8'sd100` is −56 rather
    /// than 200. The env twin of `const_expr_signed`.
    pub(crate) fn const_signed_env(&self, e: &ast::Expr, envw: &ConstWidths) -> bool {
        use ast::ExprKind as K;
        match &e.kind {
            K::IntLit { kind, raw } => parse_int_literal(raw, *kind).is_some_and(|c| c.signed),
            K::Paren { inner } => self.const_signed_env(inner, envw),
            K::Ident(p) if p.segments.len() == 1 => envw
                .get(&p.segments[0].name)
                .copied()
                .or_else(|| self.walk_scopes(&p.segments[0].name, &self.param_meta))
                .is_some_and(|(_, s)| s),
            K::PkgScoped { .. } => self.const_expr_signed(e),
            K::Unary {
                op: ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot,
                operand,
            } => self.const_signed_env(operand, envw),
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
                        self.const_signed_env(lhs, envw) && self.const_signed_env(rhs, envw)
                    }
                    // Shifts / `**`: the LEFT operand alone decides.
                    B::Shl | B::Shr | B::AShl | B::AShr | B::Pow => {
                        self.const_signed_env(lhs, envw)
                    }
                    _ => false, // 1-bit unsigned result
                }
            }
            K::Ternary { then_e, else_e, .. } => {
                self.const_signed_env(then_e, envw) && self.const_signed_env(else_e, envw)
            }
            K::Cast { target, expr } => match target {
                ast::CastTarget::Prim(p) => cast_prim_wsign(*p).is_some_and(|(_, s, _)| s),
                ast::CastTarget::Signing { signed } => *signed,
                ast::CastTarget::Size(_) => self.const_signed_env(expr, envw),
                // `RPS'(e)` — the Named spelling of a size cast inherits the
                // operand's sign exactly like `Size` when the name IS a constant
                // (`const_expr_signed`'s canonical rule; the two sign models must
                // not drift). A typedef/class Named stays unsigned.
                ast::CastTarget::Named(_) => {
                    self.cast_size_bits(target).is_some() && self.const_signed_env(expr, envw)
                }
            },
            K::Call { name, .. } if name.segments.len() == 1 => self
                .const_func_table
                .get(&name.segments[0].name)
                .and_then(|f| self.const_fn_ret_wsign(f))
                .is_some_and(|(_, s)| s),
            // `$clog2`/`$bits` yield a signed 32-bit int.
            K::SysCall { .. } => true,
            _ => false,
        }
    }

    /// Evaluate `e` in a context `ctx_w` bits wide: every context-determined
    /// operator computes and MASKS at that width, while self-determined positions
    /// (a shift count, a ternary condition, comparison operands) evaluate on their
    /// own. `ctx_w == 0` means "no known context" and degrades to the plain
    /// unlimited evaluation, so nothing is invented where a width is unknown.
    pub(crate) fn eval_const_env_at(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
        ctx_w: u32,
        ctx_signed: bool,
    ) -> Option<i64> {
        use ast::ExprKind as K;
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
        match &e.kind {
            K::Paren { inner } => {
                self.eval_const_env_at(inner, env, envw, depth, ctx_w, ctx_signed)
            }
            K::Unary {
                op: op @ (ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot),
                operand,
            } => {
                let v = self.eval_const_env_at(operand, env, envw, depth, ctx_w, ctx_signed)?;
                Some(mask(match op {
                    ast::UnOp::Plus => v,
                    ast::UnOp::Minus => v.checked_neg()?,
                    _ => !v,
                }))
            }
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
                        let a = self.eval_const_env_at(lhs, env, envw, depth, ctx_w, ctx_signed)?;
                        let b = self.eval_const_env_at(rhs, env, envw, depth, ctx_w, ctx_signed)?;
                        Some(mask(const_binop(*op, a, b)?))
                    }
                    // LEFT operand takes the context; the shift COUNT / exponent is
                    // self-determined (`(8'd200+8'd100) >> 2` is 11 at 8 bits and
                    // 75 at 32 — the count itself is 2 either way).
                    B::Shl | B::Shr | B::AShl | B::AShr | B::Pow => {
                        let a = self.eval_const_env_at(lhs, env, envw, depth, ctx_w, ctx_signed)?;
                        // Pow goes through the one shared exponent helper —
                        // today that is the same self-determined walk a shift
                        // count takes, but a rule change to the exponent (its
                        // §11.4.10-vs-Table-11-21 story differs from a shift
                        // count's) must happen in ONE place.
                        let b = if matches!(op, B::Pow) {
                            self.const_pow_exponent_selfdet(rhs, env, envw, depth)?
                        } else {
                            self.eval_const_env_self(rhs, env, envw, depth)?
                        };
                        // With a KNOWN context width a shift is exact on the bit
                        // pattern, so it no longer has to decline. `const_binop`
                        // refuses a logical `>>` of a negative value because the
                        // result depends on the operand width — but that width is
                        // right here, and refusing turned `bit [7:0] t =
                        // (8'sd100 + 8'sd100) >> 1` (iverilog 100) into a LOUD
                        // reject once the operands started masking to −56.
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
                    }
                    // A comparison's operands size against EACH OTHER, not against
                    // the surrounding context, and the result is a 1-bit 0/1 that
                    // no masking may touch.
                    _ => {
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
                        let cs =
                            self.const_signed_env(lhs, envw) && self.const_signed_env(rhs, envw);
                        let a = self.eval_const_env_at(lhs, env, envw, depth, w, cs)?;
                        let b = self.eval_const_env_at(rhs, env, envw, depth, w, cs)?;
                        const_binop(*op, a, b)
                    }
                }
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // The condition is self-determined; the arms take the context.
                if self.eval_const_env_self(cond, env, envw, depth)? != 0 {
                    self.eval_const_env_at(then_e, env, envw, depth, ctx_w, ctx_signed)
                } else {
                    self.eval_const_env_at(else_e, env, envw, depth, ctx_w, ctx_signed)
                }
            }
            // `!x` (and the reductions) yield a 1-BIT result from a
            // self-determined operand — the operand must not take the surrounding
            // width, or `!(4'd15 + 4'd1)` sees 16 instead of the 4-bit 0.
            K::Unary {
                op: op @ ast::UnOp::LogNot,
                operand,
            } => {
                let v = self.eval_const_env_self(operand, env, envw, depth)?;
                let _ = op;
                Some((v == 0) as i64)
            }
            // A system function's ARGUMENT is self-determined — it does not take the
            // surrounding context — so it evaluates at its OWN width:
            // `$clog2(4'd15 + 4'd15)` is `$clog2(14)` = 4, not `$clog2(30)` = 5,
            // whatever the assignment target is.
            K::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                self.const_clog2_selfdet(&args[0], env, envw, depth)
            }
            // A leaf enters the context at ITS OWN declared width: §11.6.1 extends
            // an operand to the context size, sign-extending only when the operand
            // is signed AND the whole expression is signed — otherwise ZERO. `env`
            // stores an 8-bit signed local already sign-extended into i64, so
            // without this a `byte a = -100` reached a 32-bit UNSIGNED context as
            // 0xFFFF_FF9C instead of 0x0000_009C, and `(a * 8'sd1) > LIMIT` answered
            // 1 where iverilog and vita's own runtime answer 0.
            _ => {
                let v = self.eval_const_env(e, env, envw, depth)?;
                if !masking {
                    return Some(v);
                }
                let Some(lw) = self.const_self_width(e, envw) else {
                    return Some(v); // unknown leaf width ⇒ do not reinterpret
                };
                let ls = self.const_signed_env(e, envw);
                Some(Self::leaf_into_ctx(v, lw, ls, ctx_signed))
            }
        }
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

    /// Evaluate `e` in a SELF-determined position — a shift count, a ternary
    /// condition, a system-function argument. Such an operand is sized by itself,
    /// never by the surrounding assignment, so it runs at its own self-width.
    pub(crate) fn eval_const_env_self(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        let w = self.const_self_width(e, envw).unwrap_or(0);
        let sg = self.const_signed_env(e, envw);
        self.eval_const_env_at(e, env, envw, depth, w, sg)
    }

    /// Evaluate `rhs` for an assignment whose target is `(w, signed)`: the RHS runs
    /// at `max(its self-determined width, w)` and the result is then coerced to the
    /// target. This is the one entry point the interpreter's assignment paths use.
    pub(crate) fn eval_const_assign(
        &self,
        rhs: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
        target: Option<(u32, bool)>,
    ) -> Option<i64> {
        let Some((tw, ts)) = target else {
            // An unknown target shape keeps the previous unlimited behavior.
            return self.eval_const_env(rhs, env, envw, depth);
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
        let v = self.eval_const_env_at(rhs, env, envw, depth, ctx, cs)?;
        Some(Self::const_mask(v, tw, ts))
    }

    /// The declared `(width, signed)` of a body declaration / formal, or None when
    /// it is not an integral shape this domain models (the caller then keeps the
    /// unlimited behavior rather than guessing a width).
    pub(crate) fn const_decl_wsign(
        &self,
        kind: ast::NetVarKind,
        range: Option<&ast::Range>,
        packed: &[ast::Range],
        signed: bool,
    ) -> Option<(u32, bool)> {
        // `ast_kind_range_width` folds only a BARE DECIMAL bound, so the commonest
        // parameterized form (`bit [PW-1:0]`) fell through and kept the old
        // unlimited behavior. Fold the bounds in the const domain first — the same
        // thing the sibling `const_fn_ret_wsign` already does for a return type.
        let w = match range {
            // A bound that CALLS a constant function must not fold here: this runs
            // while the interpreter is already inside a call, and
            // `const_eval_in_scope` restarts the call depth at 0, so
            // `bit [f()-1:0]` inside `f` recursed until the stack overflowed (PRE
            // simply did not fold and was fine). Declining keeps that behavior and
            // removes the recursion by construction rather than by a depth cap.
            Some(r)
                if ast_kind_range_width(kind, range).is_none()
                    && !Self::ast_contains_call(&r.msb)
                    && !Self::ast_contains_call(&r.lsb) =>
            {
                let hi = self.const_eval_in_scope(&r.msb)?;
                let lo = self.const_eval_in_scope(&r.lsb)?;
                u32::try_from(hi.abs_diff(lo).checked_add(1)?).ok()?
            }
            _ => ast_kind_range_width(kind, range)?,
        };
        // A MULTI-packed declaration (`logic [3:0][7:0] m`) is as wide as the
        // PRODUCT of its dimensions, and `ast_kind_range_width` only ever reports
        // the first one. This used to decline outright and record UNKNOWN — but an
        // unknown target contributes nothing to §11.6's `max(self, target)`, so
        // `bit [1:0][3:0] tt; tt = 4'd13 ** 4'd2;` evaluated at the RHS's own 4 bits
        // and stored 9 where iverilog stores 169. The extra dimensions are ordinary
        // constant ranges — the same fold the first one just got — so multiply them
        // in. A dimension this domain cannot fold still declines, and a product past
        // 64 bits masks as the identity, which IS the unlimited behavior it had.
        let mut w = w;
        for r in packed {
            if Self::ast_contains_call(&r.msb) || Self::ast_contains_call(&r.lsb) {
                return None; // same recursion guard the first dimension takes
            }
            let hi = self.const_eval_in_scope(&r.msb)?;
            let lo = self.const_eval_in_scope(&r.lsb)?;
            let d = u32::try_from(hi.abs_diff(lo).checked_add(1)?).ok()?;
            w = w.checked_mul(d)?;
        }
        // `time` is unsigned by definition; every other integral kind carries the
        // signedness the DECLARATION resolved. Hard-coding the atom keywords signed
        // was wrong: the parser already applies each atom's default, so `int
        // unsigned u` arrives as `Int` + `signed = false` and forcing it signed
        // sign-extended 32'hFFFF_FFFF into −1.
        let s = match kind {
            ast::NetVarKind::Time => false,
            _ => signed,
        };
        Some((w, s))
    }
}
