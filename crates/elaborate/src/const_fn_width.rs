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

mod fold;
mod fold_x;
pub(crate) use fold::{CFold, XWhy};

/// `(width, signed)` of every name visible to the interpreter — the parallel twin
/// of its value env, so an assignment can find its target's declared shape.
pub(crate) type ConstWidths = std::collections::BTreeMap<String, (u32, bool)>;

/// The key `envw` records a `pkg::name` leaf under.
///
/// ⚠️ QUALIFIED, and the qualification is load-bearing. `ConstWidths` is keyed by a
/// plain `String` shared with bare `Ident` leaves, and a module parameter may carry
/// the same identifier as a package constant: `parameter [7:0] PW = 8'haa;` in the
/// module beside `pk::PW` at 36 bits, in the one expression `pk::PW + PW`, is a
/// buildable design (both oracles run it and print `b4`). A bare `PW` key would hand
/// the module's 8-bit parameter the package constant's 36 bits — a correct→wrong
/// trade for a silent→value one. One spelling, here, so the writer
/// (`declared_override_widths`) and the two readers (`ctx_width_names_are_evident`,
/// `const_self_width`) cannot drift.
pub(crate) fn pkg_envw_key(pkg: &str, name: &str) -> String {
    format!("{pkg}::{name}")
}

/// Does this system function return a 32-bit SIGNED INTEGER *and* fold in the
/// constant domain?
///
/// An explicit list, never a blanket `SysCall`: the width rule below
/// (`const_self_width`'s `Some(32)` and `const_signed_env`'s `true`) is a claim about
/// INTEGER-returning calls only, and a real-returning one (`$itor`, `$bitstoreal`,
/// `$realtime`) would be sized wrong by it the day the const domain learns to fold it.
/// These three are exactly the ones `const_fn.rs` folds into this domain — `$clog2`
/// (`:377`), `$rtoi` (`:387`), `$bits` (`:408`). The dimension-query family
/// (`$size`/`$high`/…) is integer-returning too but is NOT in this list, so a
/// consumer that also needs the family must spell `|| is_dim_query_name(..)` —
/// `const_eval_in_scope` DOES fold the family (`const_fn.rs:393`), which is why
/// `const_expr_signed` (`const_eval.rs`) asks both lists. Widening this list
/// itself is a different (loud→value) move for the consumers that gate on it —
/// `ctx_width_names_are_evident` and `param_decl_width_opt` — and belongs to its
/// own row.
pub(crate) fn sys_fn_is_integer(name: &str) -> bool {
    matches!(name, "$clog2" | "$bits" | "$rtoi")
}

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
        | B::InsideEq
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

    /// Inside a PACKAGE function the constant interpreter is running (`const_call_pkg`),
    /// the declared `(width, signed)` of that package's own numeric constant `name` —
    /// which the interpreter seeds into `env` without an `envw` twin, so a width or sign
    /// read of it fell to the CALLER's same-named parameter: `return C;` over a package
    /// `[15:0] C = 16'h0123` beside a module `[7:0] C` folded `23` (both oracles `123`).
    /// Asked only on an `envw` miss, so a formal or local of the name keeps its own.
    fn pkg_fn_const_meta(&self, name: &str) -> Option<(u32, bool)> {
        let pkg = self.pkg_fn_own()?;
        self.pkg_consts.get(&pkg)?.get(name)?;
        self.pkg_const_meta.get(&pkg)?.get(name).copied()
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
            K::IntLit { kind, raw } => int_literal_shape(raw, *kind).map(|(w, _)| w),
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
                    self.pkg_fn_const_meta(&p.segments[0].name)
                        .or_else(|| self.walk_scopes(&p.segments[0].name, &self.param_meta))
                        .map_or(32, |(w, _)| w),
                ),
            },
            // The CERTIFIED width first, under the qualified key, exactly as the
            // single-segment `Ident` arm above prefers `envw` over `param_meta`: the
            // env entry is the one `pkg_const_narrow_bits` proved is a declared fact,
            // and reading the un-certified map beside it would let the two halves of
            // one answer come from different declarations.
            K::PkgScoped { pkg, name } => Some(
                match envw.get(&pkg_envw_key(&pkg.name, &name.name)).copied() {
                    Some((0, _)) => 32,
                    Some((w, _)) => w,
                    None => self
                        .pkg_const_meta
                        .get(&pkg.name)
                        .and_then(|m| m.get(&name.name))
                        .map_or(32, |(w, _)| *w),
                },
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
                // §3 ⑤ⓕ: a signing cast PRESERVES width whichever way its sign is
                // decided, so the per-instance twin answers identically.
                ast::CastTarget::Signing { .. } | ast::CastTarget::SigningParam { .. } => {
                    self.const_self_width(expr, envw)
                }
            },
            // A call is as wide as its declared return type.
            K::Call { name, .. } => self
                .const_fn_def(name)
                .and_then(|(f, p)| self.const_fn_ret_wsign_in(f, p.as_deref()))
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
            K::IntLit { kind, raw } => int_literal_shape(raw, *kind).is_some_and(|(_, s)| s),
            K::Paren { inner } => self.const_signed_env(inner, envw),
            K::Ident(p) if p.segments.len() == 1 => envw
                .get(&p.segments[0].name)
                .copied()
                .or_else(|| self.pkg_fn_const_meta(&p.segments[0].name))
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
                // §3 ⑤ⓕ: the per-instance twin; unresolvable declines to unsigned,
                // the catch-all's answer.
                ast::CastTarget::SigningParam { shape_param } => {
                    self.cast_shape_signed(shape_param).unwrap_or(false)
                }
                ast::CastTarget::Size(_) => self.const_signed_env(expr, envw),
                // `RPS'(e)` — the Named spelling of a size cast inherits the
                // operand's sign exactly like `Size` when the name IS a constant
                // (`const_expr_signed`'s canonical rule; the two sign models must
                // not drift). A typedef/class Named stays unsigned.
                ast::CastTarget::Named(_) => {
                    self.cast_size_bits(target).is_some() && self.const_signed_env(expr, envw)
                }
            },
            K::Call { name, .. } => self
                .const_fn_def(name)
                .and_then(|(f, p)| self.const_fn_ret_wsign_in(f, p.as_deref()))
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
        self.fold_ctx(e, env, envw, depth, ctx_w, ctx_signed)
            .legacy()
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
        self.fold_self(e, env, envw, depth).legacy()
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
        self.fold_shift_count(e, env, envw, depth)
    }

    /// Is every NAME in this expression one whose width `envw` records — i.e. is the
    /// evaluation context this expression states EVIDENT from the expression itself?
    ///
    /// Conservative by construction — a kind this does not enumerate answers `false`, so
    /// a new expression form cannot inherit the mask by falling through a catch-all.
    /// See [`Self::eval_const_shift_count`] for the measurement that made it necessary.
    ///
    /// ⚠️ TWO callers now, and they ask the same question for the same reason: a shift
    /// COUNT (`eval_const_shift_count`) and an untyped parameter INITIALIZER
    /// (`param_init_width_aware_ok`) both mask at a width, and both would read that
    /// width off `param_meta` for a bare NAME — which records the DEFAULT literal's
    /// width for an untyped parameter and is replaced by the final override's type the
    /// moment `#(.C(…))` arrives (§6.20.2). Masking at a stale width turns correct
    /// values into silent-wrong ones; declining keeps the pre-slice answer. The gate is
    /// therefore one helper, not a predicate copied into a second place.
    pub(crate) fn ctx_width_names_are_evident(e: &ast::Expr, envw: &ConstWidths) -> bool {
        use ast::ExprKind as K;
        match &e.kind {
            K::IntLit { .. } => true,
            K::Paren { inner } => Self::ctx_width_names_are_evident(inner, envw),
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
            K::Unary { operand, .. } => Self::ctx_width_names_are_evident(operand, envw),
            K::Binary { lhs, rhs, .. } => {
                Self::ctx_width_names_are_evident(lhs, envw)
                    && Self::ctx_width_names_are_evident(rhs, envw)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                Self::ctx_width_names_are_evident(cond, envw)
                    && Self::ctx_width_names_are_evident(then_e, envw)
                    && Self::ctx_width_names_are_evident(else_e, envw)
            }
            K::Cast {
                target: ast::CastTarget::Size(_),
                expr,
            } => Self::ctx_width_names_are_evident(expr, envw),
            K::Concat { parts } => parts
                .iter()
                .all(|q| Self::ctx_width_names_are_evident(q, envw)),
            // §2 "Index sealing" ⓑ: an integer-returning system function's width is a
            // TYPE fact, not an inference — `const_self_width` has answered 32 for it
            // since the `**` exponent work and `const_signed_env` has answered SIGNED.
            // This gate simply never asked, so `localparam A = $clog2(300); ~A` and
            // `#(.P(~$clog2(300)))` fell back to the leaf's own default: ONE bit / 0
            // where both oracles say 32 / `fffffff6`, and `logic [(W[15:8])+8-1:0] v`
            // declared 1 bit where both declare 263.
            //
            // ⚠️ NAMED list, not a blanket arm — see `sys_fn_is_integer` for why a
            // real-returning call must not join it, and note that admitting a name
            // here cannot by itself manufacture a value: the consumers all require
            // `const_eval_in_scope` to fold the expression first.
            K::SysCall { name, .. } if sys_fn_is_integer(&name.name) => true,
            // §2 "Index sealing" ⓒ: a `pkg::` leaf, admitted only when
            // `declared_override_widths` has already CERTIFIED it under the qualified
            // key (`pkg_const_narrow_bits` — the package twin of `narrow_param_bits`,
            // declining a value-inferred width, a non-zero LSB and an ascending
            // declaration alike). This is an associated fn with no `&self`, so `envw`
            // is the only channel the certification can arrive through — which is why
            // the key, not a receiver, is the fix.
            K::PkgScoped { pkg, name } => {
                matches!(envw.get(&pkg_envw_key(&pkg.name, &name.name)), Some((w, _)) if *w > 0)
            }
            // A SELECT of a CERTIFIED name: its width is structural (§11.5.1) and it is
            // unsigned, so it is evident whenever its base is a name `envw` certifies
            // and its bounds / index are themselves evident. Opt-in through `envw`: a
            // consumer that passes an empty map (the parameter-initializer gate, a
            // module-scope shift count) certifies no base and is unchanged. Measured on
            // the override lane (`override_self_meta`), where the select used to refuse
            // the whole tree back to the DEFAULT literal's type: `#(.P(~W8[3:0]))` onto
            // an untyped `parameter P = 5` bound 32 signed bits `fffffffa` where both
            // oracles bind 4 bits `a`.
            K::PartSelect { base, msb, lsb } => {
                Self::select_base_is_certified(base, envw)
                    && Self::ctx_width_names_are_evident(msb, envw)
                    && Self::ctx_width_names_are_evident(lsb, envw)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                Self::select_base_is_certified(base, envw)
                    && Self::ctx_width_names_are_evident(offset, envw)
                    && Self::ctx_width_names_are_evident(width, envw)
            }
            K::BitSelect { base, index } => {
                Self::select_base_is_certified(base, envw)
                    && Self::ctx_width_names_are_evident(index, envw)
            }
            _ => false,
        }
    }

    /// A select's BASE for [`Self::ctx_width_names_are_evident`]: a bare or `pkg::` name
    /// the certified map records, and nothing else (a select of an expression is not a
    /// declared-width vector this gate can vouch for).
    fn select_base_is_certified(base: &ast::Expr, envw: &ConstWidths) -> bool {
        matches!(
            base.kind,
            ast::ExprKind::Ident(_) | ast::ExprKind::PkgScoped { .. }
        ) && Self::ctx_width_names_are_evident(base, envw)
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
        self.fold_assign(rhs, env, envw, depth, target).legacy()
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
        // Every integral kind carries the signedness the DECLARATION resolved.
        // Hard-coding the atom keywords signed was wrong: the parser already applies
        // each atom's default, so `int unsigned u` arrives as `Int` + `signed =
        // false` and forcing it signed sign-extended 32'hFFFF_FFFF into −1. `time`
        // used to be forced unsigned here as well — the same discarded-qualifier
        // defect `array_geom::kind_signedness` carried; both now honour `signed`, so
        // the const-expression width/sign container cannot drift from the net one.
        Some((w, signed))
    }
}
