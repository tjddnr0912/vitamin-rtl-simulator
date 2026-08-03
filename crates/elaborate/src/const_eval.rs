//! constant expression evaluation — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Canonical dedup key for the const pool. Cloning the `Vec<u64>` planes keeps
/// the compare total and order-independent (used only for lookup, never to drive
/// arena order — see determinism note).
pub(crate) type ConstKey = (u32, bool, u8, Vec<u64>, Vec<u64>);

/// Sign-aware i64 fold of an integer literal: an EXPLICITLY signed based
/// literal with its sign bit set (`8'shFF`) folds negative. A plain decimal
/// (`4294967295`) is the positive value as written — IEEE marks unsized
/// decimals signed, but the written magnitude is the value (iverilog folds it
/// positive), so sign-extending on the bit image would turn it into -1. The
/// image must fit i64 (else None → loud at the param sites). X/Z bits → None.
/// Truncate / sign-extend `v` to a `w`-bit value of the given signedness — the
/// coercion `coerce_param_value` applies to a scalar param, factored out so a
/// const ARRAY element (GAP-G) is coerced to its ELEMENT type the SAME way (so
/// a narrow element such as `bit[3:0]` / `byte` truncates its init literal to
/// match the runtime net and iverilog). `w == 0` or `w >= 64` keeps the full i64.
/// Minimal width (in bits) to hold `v` as a TWO'S-COMPLEMENT SIGNED value: the
/// magnitude's bit-length plus one sign bit. `0` → 1, `5` → 4, `-1` → 1,
/// `2^31` → 33, but `-2^31` → 32 (a negative power of two needs one fewer bit
/// than its positive twin). Used to size an untyped decimal param to its VALUE
/// (IEEE §6.20.2) rather than the magnitude literal's width. Always ≤ 64 for a
/// foldable `i64` (`i64::MIN` gives 64).
pub(crate) fn min_signed_bits(v: i64) -> u32 {
    let mag = if v < 0 { !v as u64 } else { v as u64 };
    (64 - mag.leading_zeros()) + 1
}

pub(crate) fn coerce_i64_to_width(v: i64, w: u32, signed: bool) -> i64 {
    if w == 0 || w >= 64 {
        return v;
    }
    // Compute the low-`w`-bit mask in u64: at w == 63 the i64 form `(1i64 << 63) - 1`
    // is `i64::MIN - 1`, which panics on subtract-overflow in a debug build (release
    // wraps to the same i64::MAX). u64 has no such edge for w ≤ 63.
    let mask = ((1u64 << w) - 1) as i64;
    let trunc = v & mask;
    if signed && (trunc & (1i64 << (w - 1))) != 0 {
        // sign-extend: trunc − 2^w. `1i64 << w` is `i64::MIN` at w == 63, so subtract
        // with wrapping — the two's-complement result is exact for any w in 1..=63.
        trunc.wrapping_sub(1i64.wrapping_shl(w))
    } else {
        trunc
    }
}

/// IEEE 1800 §11.4.2 / §9.4.5: a `repeat`/replication count whose CONSTANT value
/// carries X/Z bits evaluates to 0. [`Self::const_eval_in_scope`] is X/Z-blind — it
/// returns `None` for an X/Z literal exactly as it does for a genuinely runtime
/// value — so an intra-assignment count fold must use THIS to distinguish a constant
/// literal that folded to X/Z (⇒ 0 iterations / immediate write) from a runtime
/// count (⇒ loud). Peels `(…)` and unary `+`/`-`/`~` down to the int-literal operand.
/// (Review finding: `repeat(2'bx1) @(ev)` was wrongly rejected as runtime; iverilog
/// treats it as 0 iterations.)
pub(crate) fn count_lit_is_xz(e: &ast::Expr) -> bool {
    match &e.kind {
        ast::ExprKind::IntLit { kind, raw } => parse_int_literal(raw, *kind)
            .map(|cv| cv.bits.unk.iter().any(|&w| w != 0))
            .unwrap_or(false),
        ast::ExprKind::Paren { inner } => count_lit_is_xz(inner),
        ast::ExprKind::Unary { operand, .. } => count_lit_is_xz(operand),
        _ => false,
    }
}

/// §4.5.186: is a net/var kind an INTEGER kind a const function may use for a formal
/// / local / return (fits the i64 const domain)? Excludes real/realtime/string/event/
/// class-handle/etc. — those make a const-function call loud (correct-or-loud).
pub(crate) fn netvar_kind_is_int_const(kind: ast::NetVarKind) -> bool {
    matches!(
        kind,
        ast::NetVarKind::Reg
            | ast::NetVarKind::Logic
            | ast::NetVarKind::Integer
            | ast::NetVarKind::Time
            | ast::NetVarKind::Bit
            | ast::NetVarKind::Byte
            | ast::NetVarKind::Shortint
            | ast::NetVarKind::Int
            | ast::NetVarKind::Longint
    )
}

/// §4.5.186: truncate/sign-extend an i64 to a `width`-bit value (the SV self-width
/// coercion of a const-function return). `width >= 64` is the identity; a narrower
/// width masks to the low bits and sign-extends when `signed` and the sign bit is set.
pub(crate) fn coerce_int_width(v: i64, width: u32, signed: bool) -> i64 {
    if width == 0 || width >= 64 {
        return v;
    }
    // u64 shift: at width 63 the i64 form `(1i64 << 63) - 1` overflows (a debug
    // panic, a wrap in release). The mask is a bit pattern, so build it unsigned
    // and reinterpret — exact for every width in 1..=63.
    let mask = ((1u64 << width) - 1) as i64;
    let m = v & mask;
    if signed && (m >> (width - 1)) & 1 == 1 {
        m | !mask
    } else {
        m
    }
}

/// Const-fold a `#delay` value to integer ticks on the GLOBAL precision timeline.
/// `mult` is the module's delay multiplier `M = 10^(unit_exp − global_prec_exp)`:
/// a delay of `d` module-units becomes `round(d × M)` precision ticks (IEEE 1364 §9
/// round-half-away). The multiply happens INSIDE the rounding so a fractional
/// `#2.5` with `M=1000` is the exact `2500`, not `round(2.5)×1000`. With `M=1` (the
/// 1ns/1ns base) this is byte-identical to the prior `round(d)` behavior.
pub(crate) fn const_delay_ticks(e: &ast::Expr, mult: u64, prec_mult: u64) -> Option<u32> {
    let pick = match &e.kind {
        ast::ExprKind::MinTypMax { typ, .. } => typ.as_ref(),
        _ => e,
    };
    let real = match &pick.kind {
        ast::ExprKind::RealLit { raw, .. } => Some(raw),
        ast::ExprKind::Paren { inner } => match &inner.kind {
            ast::ExprKind::RealLit { raw, .. } => Some(raw),
            _ => None,
        },
        _ => None,
    };
    if let Some(raw) = real {
        // TWO-STAGE (doc-08): round to the MODULE's own precision first
        // (P = M/S), then scale by S = 10^(prec − global) to global ticks.
        // S == 1 (single-timescale designs / legacy entry) ⇒ round(d × M),
        // byte-identical to the prior behavior.
        let s_mult = prec_mult.max(1);
        let p_mult = (mult / s_mult).max(1);
        let x = parse_real_f64(raw) * p_mult as f64;
        let stage1 = (x.round() as i64).clamp(0, u32::MAX as i64) as u64;
        return Some(stage1.saturating_mul(s_mult).min(u32::MAX as u64) as u32);
    }
    // integer delay: exact `d × M` (saturating into u32) — a whole-unit count is
    // already an exact multiple of the module precision (unit ≥ prec), so
    // stage-1 rounding is the identity here.
    //
    // Folded at 64 bits, NOT through `const_eval_u32`. That helper takes the low
    // 32 bits (`const_fn.rs`: `… .first() … as u32`), which for a delay is a WRAP,
    // and a wrap on a delay is a silent early fire — measured, `assign
    // #5000000000` under `1ns/1ns` fired at t=705032704 (= 5e9 mod 2^32, 7.09×
    // early) with `errors=0`, while iverilog never fired it. The other two branches
    // of this function already saturate; the integer branch was the one that
    // escaped its own policy.
    const_delay_u64(pick).map(|d| d.saturating_mul(mult).min(u32::MAX as u64) as u32)
}

/// The delay path's own integer fold: the full low-64-bit value of a 2-state
/// literal, so the caller can SATURATE rather than wrap.
///
/// Deliberately not a widening of `const_eval_u32`, which serves indices, bounds
/// and widths as well — each of those has its own out-of-range policy, and giving
/// them all this one silently would be the shared-machinery mistake
/// ENGINEERING_RULES records ("adding a rule to a shared walk must be opt-in").
///
/// ⚠️ REMAINING GAP (ROADMAP §2): a delay that exceeds `u32::MAX` precision ticks
/// is still CLAMPED rather than reported. The IR field is `u32`, so representing
/// it needs a frozen-type change; reporting it needs a new W-code. Until then the
/// clamp is wrong only for a run that actually reaches 4.29e9 ticks, instead of
/// wrong for every such delay.
fn const_delay_u64(e: &ast::Expr) -> Option<u64> {
    match &e.kind {
        ast::ExprKind::IntLit { kind, raw } => {
            let cv = literal::parse_int_literal(raw, *kind)?;
            // x/z in a delay is not a constant — same rule as `const_eval_u32`.
            if cv.bits.unk.iter().any(|&w| w != 0) {
                return None;
            }
            Some(cv.bits.val.first().copied().unwrap_or(0))
        }
        ast::ExprKind::Paren { inner } => const_delay_u64(inner),
        // Anything else (a param ref, an expression) keeps the pre-existing
        // 32-bit fold: widening those is a separate question about the constant
        // evaluator, not about this wrap.
        _ => const_eval_u32(e).map(|v| v as u64),
    }
}

/// If `e` (looking through `Paren`) is an unsized single-bit fill literal
/// (`'0`/`'1`/`'x`/`'z`, IEEE §5.7.1), return its `(raw, kind)` so the caller can
/// size it to the context width. `None` for any other expression.
pub(crate) fn fill_literal_ast(e: &ast::Expr) -> Option<(&str, ast::IntLitKind)> {
    match &e.kind {
        ast::ExprKind::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => {
            Some((raw.as_str(), *kind))
        }
        ast::ExprKind::Paren { inner } => fill_literal_ast(inner),
        _ => None,
    }
}

/// Fold a fill literal `(kind, raw)` to its i64 value at `width` (low 64 bits;
/// a >64-bit param is already outside the i64 param model). `'1`@64 → all ones
/// (i64 `-1`), `'1`@48 → `0xFFFFFFFFFFFF`, `'0` → 0.
pub(crate) fn fill_to_i64(kind: ast::IntLitKind, raw: &str, width: u32) -> Option<i64> {
    literal::fill_literal_const(raw, kind, width)
        .map(|cv| cv.bits.val.first().copied().unwrap_or(0) as i64)
}

/// If `e` (peeling `(…)`) IS an unsized fill literal, return its `(kind, raw)`.
/// Used by the parameter-init path to size a bare `'1`/`'0`/`'x`/`'z` to the
/// declared param width before const-folding.
pub(crate) fn expr_as_fill(e: &ast::Expr) -> Option<(ast::IntLitKind, &str)> {
    match &e.kind {
        ast::ExprKind::Paren { inner } => expr_as_fill(inner),
        ast::ExprKind::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => {
            Some((*kind, raw.as_str()))
        }
        _ => None,
    }
}

/// Does `e` contain an unsized fill literal in a CONTEXT-PROPAGATING position
/// (the operands of arith/bitwise/shift/ternary/unary, or a concat/replication
/// element)? Recurses only into those node types — a fill buried in a select
/// index or call arg is not width-propagated by the binary/concat lowering
/// (those self-determined contexts are handled at their own sites). Bounded by
/// the parser's expr depth cap, so this stays near-linear over a design.
pub(crate) fn expr_contains_fill(e: &ast::Expr) -> bool {
    use ast::ExprKind::*;
    if fill_literal_ast(e).is_some() {
        return true;
    }
    match &e.kind {
        Paren { inner } => expr_contains_fill(inner),
        Unary { operand, .. } => expr_contains_fill(operand),
        Binary { lhs, rhs, .. } => expr_contains_fill(lhs) || expr_contains_fill(rhs),
        Ternary {
            cond,
            then_e,
            else_e,
        } => expr_contains_fill(cond) || expr_contains_fill(then_e) || expr_contains_fill(else_e),
        Concat { parts } => parts.iter().any(expr_contains_fill),
        Replicate { count, value } => {
            expr_contains_fill(count) || value.iter().any(expr_contains_fill)
        }
        _ => false,
    }
}

/// Is `e` a node type whose lowering propagates a context width to its operands
/// (so a contained fill must be sized contextually)? Used to gate the width-aware
/// path in `lower_expr` cheaply — a non-context node skips the fill scan.
pub(crate) fn is_ctx_node(e: &ast::Expr) -> bool {
    matches!(
        e.kind,
        ast::ExprKind::Binary { .. }
            | ast::ExprKind::Unary { .. }
            | ast::ExprKind::Ternary { .. }
            | ast::ExprKind::Concat { .. }
            | ast::ExprKind::Replicate { .. }
            | ast::ExprKind::Paren { .. }
    )
}

pub(crate) fn fold_init(e: &ast::Expr, width: u32) -> Option<ir::BitPacked> {
    match &e.kind {
        // A fill literal is context-determined: replicate the fill bit across the
        // full target `width` (§5.7.1), not the self-determined 32-bit default.
        ast::ExprKind::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => {
            let cv = literal::fill_literal_const(raw, *kind, width)?;
            Some(cv.bits)
        }
        ast::ExprKind::IntLit { kind, raw } => {
            let cv = parse_int_literal(raw, *kind)?;
            Some(resize_bits(&cv.bits, cv.width, width, cv.signed))
        }
        ast::ExprKind::Paren { inner } => fold_init(inner, width),
        _ => None,
    }
}

// ── ㊁ inline-function context-width frame-routing detection (pure AST) ───────
//
// A STATIC simple-body function is INLINED (fold-by-substitution). The inline path
// lowers each assignment RHS at its SELF width, then resizes to the target — so a
// WIDENING context-sensitive op (`f = s + u` into a wider `f`, `f = c << 4`, …)
// loses the §11.6 context-width extension of its operands and is silently wrong
// (the frame/module net-assignment path evaluates at the context width correctly).
// `body_needs_context_width` detects this so `build_frame_set` routes such a function
// to the (correct) frame path. CONSERVATIVE: an unfoldable width routes to frame
// (safe — the frame path is always correct; over-routing only changes a golden).

/// Fold a pure DECIMAL integer literal (a range bound / replication count) to i64;
/// `None` for a sized/based literal or any non-literal (a param/expr) ⇒ unknown.
pub(crate) fn ast_decimal_lit_i64(e: &ast::Expr) -> Option<i64> {
    match &e.kind {
        ast::ExprKind::Paren { inner } => ast_decimal_lit_i64(inner),
        ast::ExprKind::IntLit {
            kind: ast::IntLitKind::Decimal,
            raw,
        } => raw.trim().replace('_', "").parse::<i64>().ok(),
        _ => None,
    }
}

impl Elaborator<'_> {
    /// Self-determined signedness of a CONSTANT expression at the AST level
    /// (§5.4.1/§11.8.1) — the const-param analogue of the IR-level
    /// [`Self::expr_self_signed`], for a param initializer that has not been
    /// lowered. A signed operand chain keeps an untyped param signed (IEEE
    /// §6.20.2): `localparam D = 7; localparam C = D;` and `E = 3 + 4` are signed
    /// (like their operands), so a comparison against a negative value is signed.
    /// An in-scope param reference inherits its recorded signedness; anything
    /// unmodeled is conservatively unsigned.
    pub(crate) fn const_expr_signed(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.const_expr_signed(inner),
            ast::ExprKind::IntLit { kind, raw } => {
                literal::parse_int_literal(raw, *kind).is_some_and(|cv| cv.signed)
            }
            ast::ExprKind::Ident(pth) if pth.segments.len() == 1 => self
                .param_meta
                .get(&self.fq(&pth.segments[0].name))
                .is_some_and(|&(_, s)| s),
            // a `pkg::X` reference inherits the package constant's signedness.
            ast::ExprKind::PkgScoped { pkg, name } => self
                .pkg_const_meta
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
                .is_some_and(|&(_, s)| s),
            // context-determined unary +/-/~ follow the operand's sign.
            ast::ExprKind::Unary {
                op: ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot,
                operand,
            } => self.const_expr_signed(operand),
            ast::ExprKind::Binary { op, lhs, rhs } => match op {
                ast::BinOp::Add
                | ast::BinOp::Sub
                | ast::BinOp::Mul
                | ast::BinOp::Div
                | ast::BinOp::Mod
                | ast::BinOp::BitAnd
                | ast::BinOp::BitOr
                | ast::BinOp::BitXor
                | ast::BinOp::BitXnor => self.const_expr_signed(lhs) && self.const_expr_signed(rhs),
                // power & shifts: sign follows the LEFT (base) operand only.
                ast::BinOp::Pow
                | ast::BinOp::Shl
                | ast::BinOp::Shr
                | ast::BinOp::AShl
                | ast::BinOp::AShr => self.const_expr_signed(lhs),
                _ => false, // comparisons / equality / logical: 1-bit unsigned
            },
            ast::ExprKind::Ternary { then_e, else_e, .. } => {
                self.const_expr_signed(then_e) && self.const_expr_signed(else_e)
            }
            // A cast carries the CASTING TYPE's signedness (§6.24). This arm must
            // exist for every cast form `const_eval_cast` can fold, or an untyped
            // `localparam P = int'(-300)` binds the folded −300 as UNSIGNED and
            // materializes 4294966996 — the fold made it reachable, so the two must
            // stay in step. `Named` is not folded there, so it stays unsigned here.
            // A constant-function call carries its DECLARED RETURN type's sign
            // (§13.4.1). Without this arm a `function int f(); f = -56;` bound its
            // folded −56 as UNSIGNED and materialized 4294967240 — the same
            // three-predicates-must-agree trap the `Cast` arm below closed, reached
            // once width-aware evaluation made a negative return value possible.
            ast::ExprKind::Call { name, .. } if name.segments.len() == 1 => self
                .const_func_table
                .get(&name.segments[0].name)
                .and_then(|f| self.const_fn_ret_wsign(f))
                .is_some_and(|(_, s)| s),
            ast::ExprKind::Cast { target, expr } => match target {
                ast::CastTarget::Prim(p) => cast_prim_wsign(*p).is_some_and(|(_, s, _)| s),
                ast::CastTarget::Signing { signed } => *signed,
                // `N'(e)` INHERITS the operand's signedness.
                ast::CastTarget::Size(_) => self.const_expr_signed(expr),
                ast::CastTarget::Named(_) => false,
            },
            _ => false, // Call / select / concat / unmodeled: conservatively unsigned
        }
    }

    pub(crate) fn const_array_vals_of_base(&self, base: &ast::Expr) -> Option<&Vec<i64>> {
        match &base.kind {
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                let n = path.segments[0].name.as_str();
                if let Some(key) =
                    self.walk_scopes_key(n, |k| self.array_const_vals.contains_key(k))
                {
                    return self.array_const_vals.get(&key);
                }
                // A module-local declaration of `n` SHADOWS a wildcard-imported
                // package array of the same name (IEEE §26.3, iverilog-pinned
                // local-wins). A GAP-G-capturable local array would have hit
                // `array_const_vals` above; reaching here with a local net means
                // the local is a shape GAP-G does NOT capture (descending /
                // non-zero-base / multi-dim / plain variable / scalar), so the
                // correct result is LOUD — never silently fold the IMPORTED array
                // in its place. `add_net` drops the stale import alias when the
                // local net is created, but that is a pass LATER than this const
                // read, so consult the decl-order `bits_prescan` (populated for
                // every body net before the params/generates that follow it).
                // A local declaration of `n` (array net, scalar param, port, or a
                // forward-declared one) SHADOWS the wildcard-imported array —
                // `local_decl_names` was gathered upfront from the AST, so it
                // catches every declaration form regardless of order. A local
                // genvar / header param bound only in `self.params` is caught by
                // `lookup_scoped`. A pure import is NOT a local declaration →
                // absent from both → the fold below proceeds (the GAP-G support).
                if self.local_decl_names.contains(n) || self.lookup_scoped(n).is_some() {
                    return None;
                }
                // Imported (wildcard/explicit) package array parameter read by its
                // bare name: the import bound a var-alias `key → (pkg, _)`; fold
                // from that package's captured element values.
                let akey = self.walk_scopes_key(n, |k| self.pkg_var_aliases.contains_key(k))?;
                let (pkg, _) = self.pkg_var_aliases.get(&akey)?;
                self.pkg_array_const_vals.get(pkg)?.get(n)
            }
            ast::ExprKind::PkgScoped { pkg, name } => {
                self.pkg_array_const_vals.get(&pkg.name)?.get(&name.name)
            }
            _ => None,
        }
    }

    /// `Expr::Const` → its u64 value (None for non-const / X-bearing) — used to
    /// turn a static part-select's `(offset, width)` ExprId edges into a bit
    /// interval for the multi-driver scan.
    pub(crate) fn const_expr_u64(&self, eid: u32) -> Option<u64> {
        match self.exprs.get(eid as usize)? {
            ir::Expr::Const { val } => {
                let c = self.consts.get(*val as usize)?;
                if c.bits.unk.iter().any(|&u| u != 0) {
                    return None;
                }
                Some(c.bits.val.first().copied().unwrap_or(0))
            }
            // A static part-select's width edge is the unfolded `(msb - lsb) + 1`
            // tree (`width_from_msb_lsb_checked`); fold the two arithmetic ops.
            ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs,
                rhs,
            } => Some(
                self.const_expr_u64(*lhs)?
                    .wrapping_add(self.const_expr_u64(*rhs)?),
            ),
            ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs,
                rhs,
            } => Some(
                self.const_expr_u64(*lhs)?
                    .wrapping_sub(self.const_expr_u64(*rhs)?),
            ),
            _ => None,
        }
    }

    // ── PASS 1b: body NetVarDecl → nets ────────────────────────────
    /// GAP-G: for each DESUGARED array parameter name in `d`, capture its
    /// const-folded element values under its fq name so `const_eval_in_scope`
    /// can fold an element read `NAME[i]` in a constant context (a generate-scope
    /// `localparam R = ROT[g]`, or a module-scope `localparam X = ROT[2]`).
    /// Restricted to a 0-based ASCENDING single-dim array (`[0:N]` / `[N]`) whose
    /// `'{…}` init is ALL foldable scalars — the only shape whose positional
    /// pattern maps element i → index i directly. Any other shape (descending /
    /// non-zero base / multi-dim / non-foldable element / count mismatch) is left
    /// absent, so its element reads stay LOUD (correct-or-loud). Idempotent:
    /// called both in the decl-order body-param walk and at net elaboration.
    /// GAP-G: the const-folded element values of ONE declarator `decl` of a
    /// const array parameter `d` (`ROT` in `localparam int ROT[0:3]='{0,1,3,5}`),
    /// each coerced to the ELEMENT type — the same shape the RUNTIME net stores
    /// each element at (and that `coerce_param_value` applies to a scalar), so a
    /// narrow element (`bit[3:0]`, `byte`, `logic signed [N]`) truncates /
    /// sign-extends its init literal instead of storing the raw i64. Without this
    /// a const read `ROT[i]` disagreed with both iverilog AND vita's own runtime
    /// read (adversarial find). `None` unless `decl` is a 0-based ascending
    /// single-dim unpacked array with an all-foldable `'{…}` init of the declared
    /// length (descending / non-zero base / multi-dim / non-foldable → None → the
    /// element read stays loud, correct-or-loud). Shared by the module-scope
    /// capture below and the package-scope capture in `elaborate_package`.
    pub(crate) fn const_array_elem_vals(
        &mut self,
        d: &ast::NetVarDecl,
        decl: &ast::DeclName,
    ) -> Option<Vec<i64>> {
        let (base_w, _, _, elem_signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
        let elem_w = if d.packed.is_empty() {
            base_w
        } else {
            self.packed_extents(d.range.as_ref(), &d.packed)
                .iter()
                .fold(1u32, |a, &(_, w, _)| a.saturating_mul(w.max(1)))
        };
        let init = decl.init.as_ref()?;
        let ast::ExprKind::AssignPattern(parts) = &init.kind else {
            return None;
        };
        let zero_based_asc = decl.unpacked.len() == 1
            && match &decl.unpacked[0] {
                ast::Dim::Size(_) => true,
                ast::Dim::Range(r) => {
                    self.const_eval_in_scope(&r.msb) == Some(0)
                        && self.const_eval_in_scope(&r.lsb).is_some_and(|l| l >= 0)
                }
                _ => false,
            };
        if !zero_based_asc {
            return None;
        }
        let vals = parts
            .iter()
            .map(|p| {
                self.const_eval_in_scope(p)
                    .map(|v| coerce_i64_to_width(v, elem_w, elem_signed))
            })
            .collect::<Option<Vec<i64>>>()?;
        let expected = self
            .array_dim_extents(&decl.unpacked)
            .iter()
            .fold(1u32, |a, &(_, n)| a.saturating_mul(n.max(1)));
        (vals.len() as u32 == expected).then_some(vals)
    }

    pub(crate) fn capture_const_array_vals(&mut self, d: &ast::NetVarDecl) {
        if !d.const_param {
            return;
        }
        for decl in &d.names {
            if let Some(vals) = self.const_array_elem_vals(d, decl) {
                let key = self.fq(&decl.name.name);
                self.array_const_vals.insert(key, vals);
            }
        }
    }

    /// Read back a const width edge this elaboration pushed. A constant `[msb:lsb]`
    /// part-select width is the tree `Add(Sub(msb,lsb), 1)` (see
    /// [`Self::width_from_msb_lsb_checked`]), NOT a bare `Const`, so this MUST fold
    /// `Add`/`Sub` recursively — exactly mirroring the engine's
    /// `sim_engine::width::const_u32_of_expr`. (Folding only `Const` here sized a
    /// part-select capture temp at width 1, silently dropping all but the LSB of an
    /// intra-assignment `a[msb:lsb] = / <= [@ev/#d] rhs` — review finding, also
    /// affected the blocking twins.)
    pub(crate) fn const_of_expr_u32(&self, eid: u32) -> Option<u32> {
        match self.exprs.get(eid as usize)? {
            ir::Expr::Const { val } => {
                let c = self.consts.get(*val as usize)?;
                if c.bits.unk.iter().any(|&u| u != 0) {
                    return None;
                }
                u32::try_from(c.bits.val.first().copied().unwrap_or(0)).ok()
            }
            // OUTER `(msb - lsb) + 1`.
            ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs,
                rhs,
            } => Some(
                self.const_of_expr_u32(*lhs)?
                    .saturating_add(self.const_of_expr_u32(*rhs)?),
            ),
            // INNER `msb - lsb`.
            ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs,
                rhs,
            } => Some(
                self.const_of_expr_u32(*lhs)?
                    .saturating_sub(self.const_of_expr_u32(*rhs)?),
            ),
            _ => None,
        }
    }

    /// Resolve declared range → (width, msb, lsb, signed). `Integer` is a fixed
    /// 32-bit signed type regardless of any range.
    ///
    /// Width arithmetic is overflow-guarded: `abs_diff(..) + 1` is computed in
    /// `u64` and rejected above [`MAX_NET_WIDTH`] with `ElabUnsupported` (the net
    /// is then clamped to width 1 so the arena stays valid). A `[N:0]` with
    /// `N = u32::MAX` no longer panics. (COVERAGE verdict HIGH.)
    /// P0-NCW: a declared range bound that fails to const-fold AND references a
    /// net/variable or a hierarchical name is NOT a constant expression — iverilog
    /// rejects it ("not allowed in a constant expression"). Emit a loud E3009
    /// instead of the OLD silent width-1 (the bound clamped to 0). The caller then
    /// proceeds with a degenerate extent, but the run is already loud (exit 1).
    /// A const-but-unfoldable bound vita simply cannot fold yet (e.g. a constant
    /// function call `f(3)`, which iverilog DOES accept) carries no net/hier ref,
    /// so it is left silent — unchanged behavior, NOT a new false-loud.
    pub(crate) fn check_const_range_bound(&mut self, e: &ast::Expr, folded: Option<i64>) {
        if folded.is_some() {
            return;
        }
        // r19: a REAL parameter has no i64 value and is deliberately kept out of
        // `params`, so a bound reading one folds to None. `nonconst_bound_reason`
        // does not descend into system-call args (a false-loud guard for `$bits`),
        // so `$clog2(R)` produced NO diagnostic and `clamp_bound_u32(None)` silently
        // gave a 1-bit width. Converting the param at the const-eval leaf instead
        // was worse: it destroyed the real value before the enclosing expression
        // chose its context, so `if (R > 2)` with R=2.4 took the wrong generate
        // branch and `R == 2` folded TRUE. Loud here, converted nowhere.
        if self.count_reads_real_param(e) {
            self.error(
                MsgCode::ElabUnsupported,
                "a real parameter is not an integral constant and cannot be used in a \
                 width / range bound (assign it to an integer localparam first)",
            );
            return;
        }
        if let Some(reason) = self.nonconst_bound_reason(e) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("{reason} is not allowed in a constant range bound"),
            );
        }
    }

    /// First sub-expression that makes a range bound non-constant: a reference to a
    /// net/variable (single-segment name that is a net, not a param/genvar), or any
    /// hierarchical (multi-segment) name. Mirrors iverilog's constant-expression
    /// rule. Returns a human message fragment, or None when nothing runtime/
    /// hierarchical is present (const-but-unfoldable — left as-is). System/user
    /// function CALLS are NOT descended into: `$bits(net)` is a legal constant and
    /// a constant function `f(x)` is accepted by iverilog, so flagging their args
    /// would be a false-loud.
    pub(crate) fn nonconst_bound_reason(&self, e: &ast::Expr) -> Option<String> {
        use ast::ExprKind as K;
        let r = |s: &Self, x: &ast::Expr| s.nonconst_bound_reason(x);
        match &e.kind {
            K::Ident(path) => {
                if path.segments.len() > 1 {
                    let joined = path
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    return Some(format!("a hierarchical reference (`{joined}`)"));
                }
                let name = &path.segments[0].name;
                if self.lookup_scoped(name).is_none() {
                    if self.lookup_net_scoped(name).is_some() {
                        return Some(format!("a reference to net/variable `{name}`"));
                    }
                    // Neither a param/genvar NOR a net — an undefined name, or a
                    // wildcard-AMBIGUOUS one (IEEE §26.8 unbinds it). This fn is
                    // only reached when the bound did NOT fold, so a resolvable
                    // param/genvar never lands here; an unresolved bare name is
                    // genuinely undefined → loud, NOT a silent width-1 (`[UNDEF-1:0]`
                    // used to clamp to 1 bit with no error). The expression path
                    // already errors on the same name (E3010), so this restores
                    // parity for the range/width context.
                    return Some(format!("undefined name `{name}`"));
                }
                None
            }
            K::Unary { operand, .. } => r(self, operand),
            K::Binary { lhs, rhs, .. } => r(self, lhs).or_else(|| r(self, rhs)),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => r(self, cond)
                .or_else(|| r(self, then_e))
                .or_else(|| r(self, else_e)),
            K::Paren { inner } => r(self, inner),
            K::BitSelect { base, index } => r(self, base).or_else(|| r(self, index)),
            K::PartSelect { base, msb, lsb } => r(self, base)
                .or_else(|| r(self, msb))
                .or_else(|| r(self, lsb)),
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => r(self, base)
                .or_else(|| r(self, offset))
                .or_else(|| r(self, width)),
            K::Concat { parts } => parts.iter().find_map(|p| r(self, p)),
            K::Replicate { count, value } => {
                r(self, count).or_else(|| value.iter().find_map(|p| r(self, p)))
            }
            K::MinTypMax { min, typ, max } => r(self, min)
                .or_else(|| r(self, typ))
                .or_else(|| r(self, max)),
            // A2b-prereq: `pkg::name` naming a package VARIABLE is a net
            // reference — not a constant (adversarial diff F1: it used to fall
            // into the const-but-unfoldable catch-all and clamp to a SILENT
            // width-1). A `pkg::name` naming a package CONSTANT never reaches
            // this fn (the bound folds); an UNKNOWN `pkg::name` keeps the
            // pre-existing silent-unfoldable behavior (unchanged from before
            // packages had variables at all).
            K::PkgScoped { pkg, name } => {
                if self
                    .pkg_vars
                    .get(&pkg.name)
                    .is_some_and(|m| m.contains_key(&name.name))
                {
                    return Some(format!(
                        "a reference to package variable `{}::{}`",
                        pkg.name, name.name
                    ));
                }
                None
            }
            // literals · SysCall · Call · New · Dollar · Error: no bare net/hier
            // ref of their own (function calls are not descended — see doc).
            _ => None,
        }
    }

    /// Fold an AST continuous-assign / net-declaration delay into the two engine
    /// forms: the uniform `ContAssign.delay` (= `Some(rise)` from `values[0]`,
    /// preserving the frozen "has delay" fast-path) and, ONLY when rise/fall/turnoff
    /// are NOT all equal, a `(rise, fall, turnoff)` sidecar triple. `#5`, `#(3,3)`,
    /// `#(3,3,3)`, and no-delay all yield `(uniform, None)` → byte-identical. A
    /// value that fails to const-fold drops to `None` (no delay) — the pre-existing
    /// `assign #d` behavior, now shared so `wire #d w = e` desugars identically.
    pub(crate) fn fold_ca_delay(
        &self,
        delay: Option<&ast::Delay>,
    ) -> (Option<u32>, Option<(u32, u32, u32)>) {
        let mult = self.cur_time_mult;
        let pmult = self.cur_prec_mult;
        let uniform = delay.and_then(|d| {
            d.values
                .first()
                .and_then(|e| const_delay_ticks(e, mult, pmult))
        });
        let rft = delay.and_then(|d| {
            // Only 2- or 3-value specs can carry a distinct fall/turnoff.
            if d.values.len() < 2 {
                return None;
            }
            let folded: Option<Vec<u32>> = d
                .values
                .iter()
                .map(|e| const_delay_ticks(e, mult, pmult))
                .collect();
            let folded = folded?;
            let rise = folded[0];
            let fall = folded[1];
            // turnoff: explicit 3rd value, else default min(rise, fall).
            let turnoff = if folded.len() >= 3 {
                folded[2]
            } else {
                rise.min(fall)
            };
            // All equal ⇒ uniform ⇒ no sidecar (byte-identical to today).
            if rise == fall && fall == turnoff {
                None
            } else {
                Some((rise, fall, turnoff))
            }
        });
        (uniform, rft)
    }

    /// A2a: loud-reject a WRITE targeting a desugared array parameter (a
    /// parameter is an elaboration constant — silently mutating one would be
    /// a silent-wrong). `how` is the verb phrase for the message ("assign
    /// to" / "$readmem into" / …). Returns whether it fired.
    pub(crate) fn deny_const_param_write(&mut self, net: u32, how: &str) -> bool {
        if self.lowering_decl_init {
            return false;
        }
        let Some(name) = self.const_param_nets.get(&net) else {
            return false;
        };
        let name = name.clone();
        self.error(
            MsgCode::ElabUnsupported,
            &format!("cannot {how} parameter `{name}` (a parameter is an elaboration constant)"),
        );
        true
    }

    /// Dedup-or-append a const; returns its ConstId. The dedup map is lookup-only
    /// and never reorders the arena (first-seen wins, driven by traversal order).
    pub(crate) fn intern_const(&mut self, cv: ir::ConstVal) -> u32 {
        let key: ConstKey = (
            cv.width,
            cv.signed,
            match cv.repr {
                ir::ConstRepr::Numeric => 0,
                ir::ConstRepr::StrUtf8 => 1,
                ir::ConstRepr::Real => 2,
            },
            cv.bits.val.clone(),
            cv.bits.unk.clone(),
        );
        if let Some(&id) = self.const_dedup.get(&key) {
            return id;
        }
        let id = self.consts.len() as u32;
        self.consts.push(cv);
        self.const_dedup.insert(key, id);
        id
    }

    /// Append a `Const` expr of literal `n` (width `w`); returns its ExprId.
    pub(crate) fn const_u32_expr(&mut self, n: u32, w: u32) -> u32 {
        let cid = self.intern_const(make_const_u32(n, w));
        self.push_expr(ir::Expr::Const { val: cid })
    }

    /// Append a SIGNED 32-bit `Const` expr of value `v`; returns its ExprId. Used
    /// where a comparison/arithmetic must be signed (e.g. a descending foreach walk
    /// whose index transiently goes below 0).
    pub(crate) fn const_s32_expr(&mut self, v: i32) -> u32 {
        let cid = self.intern_const(make_const_i64(v as i64, 32, true));
        self.push_expr(ir::Expr::Const { val: cid })
    }

    /// Lower an i64-domain param/genvar VALUE to a Const expr (P0-6). The
    /// legacy `0..=u32::MAX` range keeps the exact old shape (unsigned 32-bit,
    /// byte-identical golden bytes for every pre-existing design); a negative
    /// value in i32 range becomes a 32-bit SIGNED const (so `%0d` prints `-4`,
    /// iverilog parity); anything wider binds as a 64-bit const.
    pub(crate) fn const_param_expr(&mut self, v: i64) -> u32 {
        if let Ok(u) = u32::try_from(v) {
            return self.const_u32_expr(u, 32);
        }
        let cv = if i32::try_from(v).is_ok() {
            make_const_i64(v, 32, true)
        } else {
            make_const_i64(v, 64, v < 0)
        };
        let cid = self.intern_const(cv);
        self.push_expr(ir::Expr::Const { val: cid })
    }

    /// Materialize a param read at its DECLARED `(width, signed)` when known
    /// (`param_meta`), else fall back to the value-inferred width
    /// ([`Self::const_param_expr`]). A typed param's const therefore carries its
    /// real width: `localparam logic [63:0] P = '1` reads as a 64-bit all-ones
    /// const, and `logic [3:0] x = 5` reads as a 4-bit const, matching iverilog.
    pub(crate) fn const_param_expr_w(&mut self, v: i64, meta: Option<(u32, bool)>) -> u32 {
        match meta {
            Some((w, signed)) if (1..=64).contains(&w) => {
                let cv = make_const_i64(v, w, signed);
                let cid = self.intern_const(cv);
                self.push_expr(ir::Expr::Const { val: cid })
            }
            _ => self.const_param_expr(v),
        }
    }

    /// A 32-bit all-X Const expr (the iverilog result of an introspection dim
    /// query on a 0-dimension object).
    pub(crate) fn const_x32_expr(&mut self) -> u32 {
        let cv = ir::ConstVal {
            width: 32,
            signed: false,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val: vec![0],
                unk: vec![0xFFFF_FFFF],
            },
        };
        let cid = self.intern_const(cv);
        self.push_expr(ir::Expr::Const { val: cid })
    }
}
