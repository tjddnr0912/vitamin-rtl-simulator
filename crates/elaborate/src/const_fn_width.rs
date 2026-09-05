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

/// Does this operator's RESULT take the surrounding context width?
///
/// The context-determined ones widen with their neighbours; the rest — the
/// comparisons, the equalities and the two logical operators — deliver ONE bit and
/// size their operands against EACH OTHER (IEEE §11.6.1 Table 11-21), so the whole
/// node is a self-determined position and the width-UNLIMITED module-scope fold is
/// the wrong evaluator for it.
///
/// Written as an exhaustive match on purpose: a new `BinOp` must not silently join
/// whichever side the compiler defaults to. This is the one spelling of that split
/// *inside the constant domain* — `eval_const_env_at`'s three-way match,
/// `const_eval_in_scope`'s redirect and `eval_const_env`'s guard all stand on it.
/// ⚠️ Four hand-written copies of the same list live outside it (`expr_ctx.rs` ×2,
/// `hoist/arms.rs`, `sim-ir::selfwidth`); they were read and they AGREE, and the
/// const-vs-runtime split they could have caused was measured closed rather than
/// opened by this rule — but they are not derived from this function.
pub(crate) fn binop_result_is_context_determined(op: ast::BinOp) -> bool {
    use ast::BinOp as B;
    match op {
        B::Add
        | B::Sub
        | B::Mul
        | B::Div
        | B::Mod
        | B::Pow
        | B::Shl
        | B::Shr
        | B::AShl
        | B::AShr
        | B::BitAnd
        | B::BitXor
        | B::BitXnor
        | B::BitOr => true,
        B::Lt
        | B::Le
        | B::Gt
        | B::Ge
        | B::Eq
        | B::Ne
        | B::CaseEq
        | B::CaseNe
        | B::WildEq
        | B::WildNe
        | B::LogAnd
        | B::LogOr => false,
    }
}

/// Is this i64 EXACTLY an unsigned 64-bit value — a bit pattern whose top bit is
/// magnitude, not sign?
///
/// Below 64 bits `const_mask` zero-extends an unsigned value, so the sign bit is
/// clear and every signed i64 operation happens to agree with the unsigned reading.
/// At exactly 64 masking is the identity (`masking = ctx_w > 0 && ctx_w < 64`), the
/// i64 IS the width, and the SIGN-SENSITIVE operations — ordering comparisons, `/`,
/// `%`, and both shifts (§11.4.10: `>>>` is arithmetic only when its left operand is
/// signed, and §11.6.1 makes it unsigned here) — read the top bit as a sign.
///
/// Measured, both oracles: `localparam L = ((64'd1 - 64'd2) > 64'd0) ? 111 : 222;`
/// is 111 and vita folded 222, while the 63-bit twin and the RUNTIME spelling of the
/// same text were both already right — the defect is the constant domain's alone.
/// `==`/`!=` and `+`/`-`/`*`/`<<`/bit-ops are sign-agnostic on the bit pattern and
/// are deliberately NOT routed here.
///
/// ⚠️ `== 64`, NOT `>= 64`. Above 64 bits the i64 has ALREADY truncated the value, so
/// neither reading is the language's: measured, `(64'hFFFF_FFFF_FFFF_FFFF + 65'd1) >
/// 64'hFFFF_FFFF_FFFF_FFFF` is 1 on both oracles and the unsigned reading of the
/// truncation answers 2, while `64'hFFFF_FFFF_FFFF_FFFF > 65'd1` goes the other way.
/// Neither dominates, which is the definition of a guess — so >64 keeps the pre-slice
/// answer and stays ROADMAP §2's. The sibling `const_unsigned_selfdet` already draws
/// the line in exactly this place.
fn const_i64_is_unsigned_at(w: u32, signed: bool) -> bool {
    !signed && w == 64
}

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
            // §5.7.1: an unsized fill (`'0`/`'1`/`'x`/`'z`) has NO width of its own —
            // it is replicated to the context's. Zero is what every consumer of this
            // table reads as "takes the context" (`max` with a sibling, `w > 0` guards,
            // `ctx_w == 0` ⇒ no masking); `parse_int_literal`'s 32 is the i64 lane's
            // container size and reported `$clog2('1)` as 32 where both oracles say 0
            // (ROADMAP §2 🆕 C). A lone fill evaluates at one bit — see the leaf arm
            // in `eval_const_env_at`.
            K::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => Some(0),
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
            // §11.4.12.1: a replication is `count` copies of its operand list — also
            // self-determined and unsigned. Without this arm a replication operand made
            // the whole enclosing width UNKNOWN, so the width-aware walk degraded to the
            // unlimited domain and `({2{4'd15}} + 4'd1) > 4'd0` kept answering 1 where
            // both oracles answer 0 — the same defect the `Concat` twin above was added
            // for, one operator over.
            K::Replicate { count, value } => {
                let n = const_eval_u32(count)?;
                let one = value.iter().try_fold(0u32, |acc, p| {
                    acc.checked_add(self.const_self_width(p, envw)?)
                })?;
                n.checked_mul(one)
            }
            // §11.5.1: a select is as wide as the bits it NAMES and is unsigned —
            // it takes nothing from the base's width, so `W[7:0]` is 8 bits even
            // though `W` is 32. Without this arm the width-aware walk had no self
            // width for a select and DEGRADED to the unlimited domain, which is the
            // §4.5.339 contract but leaves `(W[3:0] + 4'd1) > 4'd0` answering the
            // unwrapped 16 where both oracles wrap to 0 at 4 bits.
            K::BitSelect { .. } | K::PartSelect { .. } | K::IndexedPart { .. } => {
                self.const_select_self_width(e)
            }
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
            // §5.7.1: a fill is the fill bit replicated to the CONTEXT width — `ctx_w`
            // here is the region width the parent computed as `max(ctx, every
            // self-determined sibling)`, so `4'd8 - '1` sees a 4-bit all-ones (both
            // oracles 9) and a lone fill in a self-determined position (`$clog2('1)`,
            // `'1 >> 1`, `localparam U = '1;`) is ONE bit. Before this arm the leaf
            // fell to the plain twin's `const_eval_i64_lit`, whose fill is the i64
            // lane's hard 32 (ROADMAP §2 🆕 C). An x/z fill declines (no unknown
            // plane here), which is the loud the callers already had.
            K::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => {
                crate::const_eval::fill_to_i64(*kind, raw, ctx_w.max(1))
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
                        // ⚠️ `/` and `%` are the two SIGN-sensitive members of this arm
                        // (the rest are bit-pattern ops), so at 64+ unsigned bits they
                        // divide in u64: both oracles fold
                        // `64'hFFFF_FFFF_FFFF_FFFF % 64'd10` to 5, and the signed
                        // reading answered −1.
                        if const_i64_is_unsigned_at(ctx_w, ctx_signed) && b != 0 {
                            match op {
                                B::Div => return Some(mask(((a as u64) / (b as u64)) as i64)),
                                B::Mod => return Some(mask(((a as u64) % (b as u64)) as i64)),
                                _ => {}
                            }
                        }
                        Some(mask(const_binop(*op, a, b)?))
                    }
                    // LEFT operand takes the context; the shift COUNT / exponent is
                    // self-determined (`(8'd200+8'd100) >> 2` is 11 at 8 bits and
                    // 75 at 32 — the count itself is 2 either way).
                    B::Shl | B::Shr | B::AShl | B::AShr | B::Pow => {
                        let a = self.eval_const_env_at(lhs, env, envw, depth, ctx_w, ctx_signed)?;
                        // Pow goes through the one shared exponent helper — which
                        // reports the exponent's SIGNEDNESS with its value, because
                        // that is what decides whether the IEEE negative-exponent
                        // table applies at all. A shift count needs no such thing.
                        if matches!(op, B::Pow) {
                            let (b, sg) = self.const_pow_exponent_selfdet(rhs, env, envw, depth)?;
                            return Some(mask(const_pow(a, b, sg)?));
                        }
                        let b = self.eval_const_shift_count(rhs, env, envw, depth)?;
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
                    }
                    // A comparison's operands size against EACH OTHER, not against
                    // the surrounding context, and the result is a 1-bit 0/1 that
                    // no masking may touch. (`binop_result_is_context_determined`
                    // is the shared statement of that split; this arm is its
                    // `false` side and the two above are its `true` side.)
                    _ => {
                        debug_assert!(!binop_result_is_context_determined(*op));
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
                        let cs =
                            self.const_signed_env(lhs, envw) && self.const_signed_env(rhs, envw);
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
            // §11.4.14 REDUCTION — the arm this walk never had. It is what a declared
            // range bound holding a parameter SELECT reaches (`const_range_bound_fold`
            // redirects such a bound here), and what `!`, a ternary condition and a
            // `**` exponent reach for their self-determined operand — so
            // `wire [(|P[3:1])+2:0]` was ONE bit against both oracles' three, and
            // `wire [(!(|4'b1010))+2:0]` the same, at exit 0.
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
            } => {
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
        self.eval_const_env_at(e, env, envw, depth, w, sg)
    }

    /// A shift's COUNT — self-determined, and §11.4.10 adds that it "shall be treated
    /// as an unsigned number". [`Self::eval_const_env_self`] gives an operand its OWN
    /// signedness, which is right for a ternary condition and a select index and wrong
    /// here, so the count arrived NEGATIVE and every shift by it collapsed to 0.
    ///
    /// ⭐⭐ It needs no function and no name to reach: `logic [(16'h0100 >> 3'sb101)-1:0]
    /// bus;` declared `$bits` **2** where both oracles say 8 — a silently wrong bus
    /// width — and `generate if (16'hFF01 << 3'sb101)` took the WRONG BRANCH and deleted
    /// its body at exit 0. A 158-cell census found 66 silent-wrong (and 32 more latent:
    /// the defect only shows when the count's unsigned value is below the target width).
    /// All four shift operators; `**` is unaffected (`const_pow_exponent_selfdet` is its
    /// own helper), and no OTHER self-determined position is — an index, a replication
    /// count, a ternary condition, a `$clog2` argument and a generate-for bound all fold
    /// correctly, which is what makes this the shift arm's rule rather than a sizing bug.
    ///
    /// ⭐ The reference implementation was already in the tree and right: the >64-bit
    /// lane's `const_wide::fold_shift_count` reads the count's bits at its own width and
    /// treats them as unsigned, citing this clause. Three lanes disagreed inside one
    /// binary — runtime correct, wide constant lane correct, this one wrong — so every
    /// cell was a vita self-contradiction before any oracle was consulted.
    ///
    /// ⚠️ This is a POST-STEP on the value, not a different evaluator, and it belongs
    /// HERE rather than inside `eval_const_env_self`: that function has 17 callers and
    /// exactly one of them is a shift count. `C + 0` is 32 bits wide, so its unsigned
    /// reading is 4294967293 and the shift yields 0 — which is what both oracles answer
    /// too, and why widening the rule past the count itself would be wrong.
    pub(crate) fn eval_const_shift_count(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        let v = self.eval_const_env_self(e, env, envw, depth)?;
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
        if !Self::shift_count_width_is_evident(e, envw) {
            return Some(v);
        }
        // An unknown or >=64-bit self width leaves the value alone: `const_mask` is the
        // identity there, and inventing a width is what this domain must never do.
        match self.const_self_width(e, envw) {
            Some(w) if w > 0 => Some(Self::const_mask(v, w, false)),
            _ => Some(v),
        }
    }

    /// Is every NAME in a shift count's expression one whose width `envw` records?
    ///
    /// Conservative by construction — a kind this does not enumerate answers `false`, so
    /// a new expression form cannot inherit the mask by falling through a catch-all.
    /// See [`Self::eval_const_shift_count`] for the measurement that made it necessary.
    fn shift_count_width_is_evident(e: &ast::Expr, envw: &ConstWidths) -> bool {
        use ast::ExprKind as K;
        match &e.kind {
            K::IntLit { .. } => true,
            K::Paren { inner } => Self::shift_count_width_is_evident(inner, envw),
            // A subprogram local's declared range, and nothing else.
            //
            // ⚠️ `param_range` was tried as a second admission — the reasoning was that
            // `param_decl_range_opt` takes `default_binds`, so an OVERRIDDEN untyped
            // parameter would have no entry. It has one, measured: the blocking design
            // went straight back to wrong while the census went back to 30 fixed. The
            // maps that could answer this are the ones §2 row 14 is stuck on, so the
            // gate stays at the one map whose widths are declared locals.
            //
            // ⚠️ The cost is the module-scope NAMED count spelling: 16 of 168 census
            // cells keep the pre-slice answer. Every position the row is actually about
            // — a net's declared width, a `generate` condition, an unpacked dimension, a
            // `repeat`, a `-:` width — uses a LITERAL count and is fixed.
            K::Ident(p) if p.segments.len() == 1 => {
                matches!(envw.get(&p.segments[0].name), Some((w, _)) if *w > 0)
            }
            K::Unary { operand, .. } => Self::shift_count_width_is_evident(operand, envw),
            K::Binary { lhs, rhs, .. } => {
                Self::shift_count_width_is_evident(lhs, envw)
                    && Self::shift_count_width_is_evident(rhs, envw)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                Self::shift_count_width_is_evident(cond, envw)
                    && Self::shift_count_width_is_evident(then_e, envw)
                    && Self::shift_count_width_is_evident(else_e, envw)
            }
            K::Cast {
                target: ast::CastTarget::Size(_),
                expr,
            } => Self::shift_count_width_is_evident(expr, envw),
            K::Concat { parts } => parts
                .iter()
                .all(|q| Self::shift_count_width_is_evident(q, envw)),
            _ => false,
        }
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
                let hi = self.const_range_bound_fold(&r.msb)?;
                let lo = self.const_range_bound_fold(&r.lsb)?;
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
            let hi = self.const_range_bound_fold(&r.msb)?;
            let lo = self.const_range_bound_fold(&r.lsb)?;
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
