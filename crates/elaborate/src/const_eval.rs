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
        return Some(real_delay_ticks(parse_real_f64(raw), mult, prec_mult));
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

/// TWO-STAGE (doc-08) real → tick conversion: round to the MODULE's own precision
/// first (P = M/S), then scale by S = 10^(prec − global) to global ticks. S == 1
/// (single-timescale designs / legacy entry) ⇒ round(d × M).
///
/// Factored out of `const_delay_ticks`'s real-LITERAL branch so the scope-resolved
/// twin (`Elaborator::delay_ticks_in_scope`, for `parameter real RD = 2.5`) rounds
/// through the SAME two stages. Two spellings of this is how `#2.5` and `#(RD)`
/// would land on different ticks under a `10ns/1ns` module.
fn real_delay_ticks(x: f64, mult: u64, prec_mult: u64) -> u32 {
    let s_mult = prec_mult.max(1);
    let p_mult = (mult / s_mult).max(1);
    let stage1 = ((x * p_mult as f64).round() as i64).clamp(0, u32::MAX as i64) as u64;
    stage1.saturating_mul(s_mult).min(u32::MAX as u64) as u32
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
/// A fill literal (`'0`/`'1`) as an i64 at `width`. **Declines an `'x`/`'z` fill**: the
/// i64 domain has no unknown plane, and `fill_literal_const`'s packed VALUE word for
/// one is a plausible 0 (`'x`) or all-ones (`'z`) with the mask discarded — a wrong
/// number, not a missing one, and every caller here (both are parameter binding) then
/// installed it silently. The 4-state callers of `fill_literal_const` keep the full
/// `ConstVal` and are unaffected.
pub(crate) fn fill_to_i64(kind: ast::IntLitKind, raw: &str, width: u32) -> Option<i64> {
    if literal::fill_is_unknown(raw, kind) {
        return None;
    }
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
        // §4.5.353: `lower_expr` lowers `MinTypMax` as a TRANSPARENT pass-through
        // (`lower_expr(typ)`), so the assignment context does reach the chosen branch —
        // but this walk did not look inside, and the walk is what gates the whole
        // context lowering. `wire [7:0] a = (1:'1:2);` therefore kept a 1-bit fill at
        // EVERY assignment site, old and new alike, against both oracles' 11111111.
        // The arm mirrors the lowering's choice exactly: only the `typ` branch is
        // lowered, so only the `typ` branch can carry a fill that matters.
        MinTypMax { typ, .. } => expr_contains_fill(typ),
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

/// A parameter value too wide for the i64 constant domain, as a full `ConstVal`.
///
/// `Some` only when the DECLARED width exceeds 64 and the initializer is a literal
/// that `fold_init` can size exactly. Everything else — a wide expression, an
/// unsized name — stays `None` so the caller's ordinary fold (and its loud reject)
/// still runs. Callers must reach it only AFTER the numeric fold has declined: the
/// boundary is the VALUE, not the declared width, so `logic [255:0] K = 256'h1` keeps
/// its integer identity and stays usable as a width or a bound.
pub(crate) fn wide_param_const(e: &ast::Expr, width: u32, signed: bool) -> Option<ir::ConstVal> {
    if width <= 64 {
        return None;
    }
    Some(ir::ConstVal {
        width,
        // ⚠️ This was hard-coded `false` with the note "a wide packed parameter has no
        // sign in this domain, and claiming one would flip a comparison". Measurement
        // says the opposite: `localparam signed [127:0] K = 128'sh8000…0001` compared
        // POSITIVE and `K >>> 4` shifted in zeros, where iverilog says NEG and sign-
        // extends. The declaration is where the sign lives, so it is carried.
        signed,
        repr: ir::ConstRepr::Numeric,
        bits: fold_init(e, width)?,
    })
}

impl Elaborator<'_> {
    /// [`wide_param_const`] with NAMES resolved — a wide declaration may be written in
    /// terms of another wide parameter (`localparam [127:0] MASK = ~POLY;`), which the
    /// free function cannot see.
    ///
    /// ⚠️ Only `wide_param_bits` is consulted, and deliberately: it is the one table
    /// that carries bits AND width AND sign, so a name found there needs no width
    /// guess. A narrow parameter declines — fail-closed, and the caller's loud reject
    /// is what it was. The key derivation is `walk_scopes_key` over the SAME combined
    /// binding set `lower_expr` uses, so innermost-wins is one spelling; a second scope
    /// walk here is precisely the §4.5.218 shape (an order-dependent decision that
    /// silently deleted a generate body).
    pub(crate) fn wide_param_const_in_scope(
        &self,
        e: &ast::Expr,
        width: u32,
        signed: bool,
    ) -> Option<ir::ConstVal> {
        if width <= 64 {
            return None;
        }
        let resolve = |path: &ast::HierPath| -> Option<WideBits> {
            let [seg] = path.segments.as_slice() else {
                // A hierarchical name is not a constant here. UNREACHABLE today and
                // measured so (`panic!` probe, 0 hits across the whole suite): both
                // this elaborator and iverilog already refuse `localparam [127:0] B =
                // ~u1.K;` upstream, so a multi-segment path never arrives. Kept
                // fail-closed rather than widened to `segments.last()`, which would
                // resolve the LAST segment in the CURRENT scope — a same-named local
                // would then be folded for a reference that named something else.
                return None;
            };
            let key = self.walk_scopes_key(&seg.name, |k| {
                self.wide_param_bits.contains_key(k)
                    || self.params.contains_key(k)
                    || self.symbols.contains_key(k)
            })?;
            let cv = self.wide_param_bits.get(&key)?;
            Some((cv.bits.clone(), cv.width, cv.signed))
        };
        let (b, w, sg) = match &e.kind {
            // Keep the literal/fill arms exactly where they were — `fold_init` is the
            // only place that knows the CONTEXT width a fill literal needs.
            ast::ExprKind::IntLit { .. } | ast::ExprKind::Paren { .. } => {
                return wide_param_const(e, width, signed)
            }
            _ => fold_self_bits(e, &resolve)?,
        };
        Some(ir::ConstVal {
            width,
            signed,
            repr: ir::ConstRepr::Numeric,
            bits: resize_bits(&b, w, width, sg),
        })
    }
}

// ── wide (>64-bit) CARRY-FREE constant folding ───────────────────────────────
//
// `wide_param_bits` has always been able to REPRESENT a >64-bit parameter; what it
// could not do is compute one. `fold_init` handled a literal and a parenthesised
// literal, so `localparam logic [127:0] K = 128'h…` worked and
// `localparam logic [127:0] K = {8'he1, 120'h0}` was `E3009 … not a foldable
// constant expression` — the spelling every crypto IP actually uses.

/// One folded value in the wide bit domain: `(bits, self width, signed)`.
pub(crate) type WideBits = (ir::BitPacked, u32, bool);

/// Resolves a NAME to an already-folded wide constant. See `fold_self_bits`.
pub(crate) type WideNameFn<'a> = &'a dyn Fn(&ast::HierPath) -> Option<WideBits>;

/// A zeroed bit vector wide enough for `width` bits.
fn bp_zero(width: u32) -> ir::BitPacked {
    let n = ((width as usize).div_ceil(64)).max(1);
    ir::BitPacked {
        val: vec![0; n],
        unk: vec![0; n],
    }
}

fn bp_get(bp: &ir::BitPacked, i: usize) -> (bool, bool) {
    let g = |p: &[u64]| {
        p.get(i / 64)
            .map(|w| (w >> (i % 64)) & 1 == 1)
            .unwrap_or(false)
    };
    (g(&bp.val), g(&bp.unk))
}

fn bp_set(bp: &mut ir::BitPacked, i: usize, v: bool, u: bool) {
    if i / 64 >= bp.val.len() {
        return;
    }
    if v {
        bp.val[i / 64] |= 1u64 << (i % 64);
    }
    if u {
        bp.unk[i / 64] |= 1u64 << (i % 64);
    }
}

fn bp_any_unknown(bp: &ir::BitPacked, width: u32) -> bool {
    (0..width as usize).any(|i| bp_get(bp, i).1)
}

/// Fold an expression at its OWN (self-determined) width in the WIDE bit domain.
/// Returns `(bits, width, signed)`.
///
/// ⚠️⚠️ **CARRY-FREE ONLY, and that is the admission rule.** Concatenation,
/// replication, a size cast, a constant logical shift, the bitwise operators and
/// `~` all decide each result bit from operand bits at KNOWN positions. `+`, `-`,
/// `*`, `/` and the comparisons need a carry chain across 128+ bits; implementing
/// one here would be a second spelling of the engine's arithmetic, and a subtly
/// wrong one produces a silent wrong PARAMETER, which is P0-5. They decline, and
/// the caller stays loud — the same boundary this domain has always had. What
/// widens is which expressions reach it, not what it can represent.
///
/// ⚠️ Unknown (x/z) bits ride through the PLACEMENT arms (concat/replicate/shift/
/// cast MOVE bits without reading them) and DECLINE the value-reading ones, for the
/// same reason: a 4-state `&`/`|`/`^`/`~` table belongs in one place and it is not
/// here.
pub(crate) fn fold_self_bits(
    e: &ast::Expr,
    // Resolve a NAME to an already-folded constant. `None` = names decline, which is
    // what a caller without an elaborator (a class field default) must pass — it has
    // no parameter table to consult and inventing one here would be a second scope
    // walk beside `walk_scopes_key`, the exact shape §4.5.218 turned into a silent
    // generate-body deletion.
    name: WideNameFn,
) -> Option<WideBits> {
    // A width this domain will not build. `MAX_NET_WIDTH` is the declared-width cap
    // a net already lives under, so an intermediate wider than that cannot land
    // anywhere legal — and it keeps `{1000000{8'hAB}}` from allocating before the
    // caller ever sees it.
    let cap = |w: u64| -> Option<u32> {
        if w == 0 || w > MAX_NET_WIDTH {
            None
        } else {
            u32::try_from(w).ok()
        }
    };
    match &e.kind {
        ast::ExprKind::Paren { inner } => fold_self_bits(inner, name),
        // A fill literal is context-determined (§5.7.1) and therefore has NO self
        // width — only `fold_init`, which knows the target, can fold one.
        ast::ExprKind::IntLit { kind, raw } if literal::is_fill_literal(raw, *kind) => None,
        ast::ExprKind::IntLit { kind, raw } => {
            let cv = parse_int_literal(raw, *kind)?;
            Some((cv.bits, cv.width, cv.signed))
        }
        // §6.24.1 size cast: the result is `n` bits; signedness is INHERITED.
        ast::ExprKind::Cast {
            target: ast::CastTarget::Size(w),
            expr,
        } => {
            let n = cap(u64::from(const_eval_u32(w)?))?;
            let (b, bw, sg) = fold_self_bits(expr, name)?;
            Some((resize_bits(&b, bw, n, sg), n, sg))
        }
        // §11.4.12 concatenation: unsigned, leftmost part most significant.
        ast::ExprKind::Concat { parts } => {
            let (b, w) = fold_concat_parts(parts, &cap, name)?;
            Some((b, w, false))
        }
        // §11.4.12.1 replication: unsigned; the count is a constant.
        ast::ExprKind::Replicate { count, value } => {
            let n = const_eval_u32(count)?;
            if n == 0 {
                return None; // a zero replication has no width in this position
            }
            let (one, ow) = fold_concat_parts(value, &cap, name)?;
            let total = cap(u64::from(ow) * u64::from(n))?;
            let mut out = bp_zero(total);
            for k in 0..n as usize {
                for i in 0..ow as usize {
                    let (v, u) = bp_get(&one, i);
                    bp_set(&mut out, k * ow as usize + i, v, u);
                }
            }
            Some((out, total, false))
        }
        // §11.4.10 LOGICAL shift by a constant: the result keeps the LEFT operand's
        // self width and signedness, and the vacated bits are ZERO for both
        // directions. `<<<`/`>>>` are deliberately NOT admitted — the arithmetic
        // right shift reads the sign bit, which is the value-reading class above.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(op, ast::BinOp::Shl | ast::BinOp::Shr) =>
        {
            let k = const_eval_u32(rhs)? as usize;
            let (b, w, sg) = fold_self_bits(lhs, name)?;
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                // source index for result bit i
                let src = match op {
                    ast::BinOp::Shl => i.checked_sub(k),
                    _ => i.checked_add(k).filter(|s| *s < w as usize),
                };
                if let Some(si) = src {
                    let (v, u) = bp_get(&b, si);
                    bp_set(&mut out, i, v, u);
                }
            }
            Some((out, w, sg))
        }
        // §11.4.8 bitwise: width is the max of the operands, each extended in its own
        // signedness; the result is signed only if BOTH operands are.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(
                op,
                ast::BinOp::BitAnd | ast::BinOp::BitOr | ast::BinOp::BitXor
            ) =>
        {
            let (lb, lw, ls) = fold_self_bits(lhs, name)?;
            let (rb, rw, rs) = fold_self_bits(rhs, name)?;
            if bp_any_unknown(&lb, lw) || bp_any_unknown(&rb, rw) {
                return None;
            }
            let w = lw.max(rw);
            let (la, ra) = (resize_bits(&lb, lw, w, ls), resize_bits(&rb, rw, w, rs));
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                let (a, _) = bp_get(&la, i);
                let (c, _) = bp_get(&ra, i);
                let v = match op {
                    ast::BinOp::BitAnd => a && c,
                    ast::BinOp::BitOr => a || c,
                    _ => a ^ c,
                };
                bp_set(&mut out, i, v, false);
            }
            Some((out, w, ls && rs))
        }
        ast::ExprKind::Unary {
            op: ast::UnOp::BitNot,
            operand,
        } => {
            let (b, w, sg) = fold_self_bits(operand, name)?;
            if bp_any_unknown(&b, w) {
                return None;
            }
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                bp_set(&mut out, i, !bp_get(&b, i).0, false);
            }
            Some((out, w, sg))
        }
        ast::ExprKind::Ident(path) => name(path),
        _ => None,
    }
}

/// Fold a concat PART LIST (also the body of a replication, which holds the parts
/// directly rather than a `Concat` wrapper) into one unsigned bit vector.
fn fold_concat_parts(
    parts: &[ast::Expr],
    cap: &dyn Fn(u64) -> Option<u32>,
    name: WideNameFn,
) -> Option<(ir::BitPacked, u32)> {
    let mut folded = Vec::with_capacity(parts.len());
    let mut total: u64 = 0;
    for p in parts {
        let (b, w, _) = fold_self_bits(p, name)?;
        total += u64::from(w);
        cap(total)?;
        folded.push((b, w));
    }
    let total = cap(total)?;
    let mut out = bp_zero(total);
    let mut pos = total as usize;
    for (b, w) in folded {
        pos -= w as usize;
        for i in 0..w as usize {
            let (v, u) = bp_get(&b, i);
            bp_set(&mut out, pos + i, v, u);
        }
    }
    Some((out, total))
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
        // Everything the CARRY-FREE wide folder admits: fold at the expression's own
        // width, then size it to the target the way an assignment would. Additive —
        // every shape that folded before still takes an arm above, and one this
        // declines still returns None, so the caller's loud reject is unchanged.
        _ => {
            let (b, w, sg) = fold_self_bits(e, &|_| None)?;
            Some(resize_bits(&b, w, width, sg))
        }
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
                // `N'(e)` INHERITS the operand's signedness — and so does the
                // `RPS'(e)` spelling of the same cast (see `cast_size_bits`); a
                // `Named` that is NOT a constant is a type cast and stays unsigned.
                ast::CastTarget::Size(_) => self.const_expr_signed(expr),
                ast::CastTarget::Named(_) => {
                    self.cast_size_bits(target).is_some() && self.const_expr_signed(expr)
                }
            },
            _ => false, // Call / select / concat / unmodeled: conservatively unsigned
        }
    }

    /// GAP-G: resolve the element-value table of a const array `base` used in a
    /// constant-context element read (`base[i]`). Handles, in local-wins order:
    /// (1) a module-local / generate-scope array by bare name (the same scope walk a
    /// bare param Ident takes); (2) a package array named by its bare name made visible
    /// via `import p::*` / `import p::ROT` — resolved through the var-alias the import
    /// machinery bound (`pkg_var_aliases`) to its origin package; (3) an explicitly
    /// package-qualified array `p::ROT`. Any other base shape (hierarchical,
    /// multi-segment, a non-captured array) → None → the read stays loud at the binding
    /// site (correct-or-loud).
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
        // A >64-bit parameter is deliberately kept out of `params` too (see
        // `wide_param_bits`), so a bound reading one also folds to None — but calling
        // it "undefined" sends the user looking for a typo in a name that is right
        // there. Say what it actually is.
        if let Some(n) = self.wide_param_name_in(e) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "`{n}` is wider than 64 bits, so it has no integral constant value \
                     for a width / range bound (select the bits you need, or declare a \
                     narrower localparam)"
                ),
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

    /// The first name in `e` bound to a >64-bit parameter, if any.
    ///
    /// Walks [`Self::const_fold_children`] — the const domain's own traversal —
    /// rather than `collect_bare_idents`. ⚠️ That collector is deliberately
    /// SELECT-BLIND because the implicit-net pass shares it, where a missed arm
    /// costs a loud E3010 and an added one could conjure a silent implicit net;
    /// widening it for a diagnostic would trade a rung on an unrelated path. The
    /// cost of the blindness was here: `localparam [127:0] K = …; logic [K[7:0]-1:0] v;`
    /// never saw `K` through the select, so the message fell through to
    /// `nonconst_bound_reason` and said *"undefined name `K`"* about a name declared
    /// two lines up — while the message four lines above tells the user to "select
    /// the bits you need", which is precisely what they did.
    fn wide_param_name_in(&self, e: &ast::Expr) -> Option<String> {
        if let ast::ExprKind::Ident(p) = &e.kind {
            if let [seg] = p.segments.as_slice() {
                if self
                    .walk_scopes_key(&seg.name, |k| self.wide_param_bits.contains_key(k))
                    .is_some()
                {
                    return Some(seg.name.clone());
                }
            }
        }
        Self::const_fold_children(e)
            .into_iter()
            .find_map(|c| self.wide_param_name_in(c))
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
                // §3 ⑨: a STRING or REAL package parameter. This is the hole the comment
                // above predicted — "an UNKNOWN `pkg::name` keeps the pre-existing
                // silent-unfoldable behavior" — and routing those two domains out of
                // `pkg_consts` walked straight into it: `logic [P::S-1:0] v;` with
                // `parameter S = "RED"` silently clamped to ONE BIT at exit 0 where both
                // oracles give 5391684. The MODULE-scope twin is loud for the same text,
                // so this is branch parity, not a new rule — a string in an integral
                // context is one recorded gap for both scopes, and it stays loud in both.
                // A `real` param with a wholly integral initializer is also in
                // `pkg_consts`, so its bound folds and never reaches here.
                // ⚠️ NOUN PHRASES, like the sibling arms: the caller appends "… is not
                // allowed in a constant range bound", so a sentence here reads as two
                // clauses jammed together ("has no integral value is not allowed in …").
                if self
                    .pkg_str_raw
                    .get(&pkg.name)
                    .is_some_and(|m| m.contains_key(&name.name))
                {
                    return Some(format!(
                        "a string package parameter (`{}::{}`)",
                        pkg.name, name.name
                    ));
                }
                if self
                    .pkg_real_val
                    .get(&pkg.name)
                    .is_some_and(|m| m.contains_key(&name.name))
                {
                    return Some(format!(
                        "a real package parameter (`{}::{}`)",
                        pkg.name, name.name
                    ));
                }
                None
            }
            // A SYSTEM CALL that did not fold. Reaching here means the bound was not
            // constant, so this is the last chance to say why — and the catch-all below
            // used to swallow it into a SILENT width-1 net. `wire [$bits(X)-1:0] c;`
            // declared a ONE-BIT net whenever the `$bits` argument was a shape the
            // constant domain could not see, truncating an 8-bit assignment to 1 at
            // exit 0 in every backend, and doing it across a module boundary when the
            // bound was on a port. It did that even for `$bits(<undeclared name>)`.
            // The width-folding half of this is fixed (`bits_of_selfdet`), but a bound
            // must not degrade silently for the shapes that remain, so name the call.
            K::SysCall { name, .. } => {
                Some(format!("a `{}` that is not a constant here", name.name))
            }
            // A CAST is the same hole one node over, and the same detector found it:
            // `logic [int'(NOPE)-1:0] v;` declared a SILENT one-bit net at exit 0
            // while the bare `[NOPE-1:0]` twin was loud about that very name. Descend
            // first, so the reason names what is actually wrong; then name the cast
            // itself, because reaching this fn at all means the bound did not fold and
            // a bound that does not fold must never degrade quietly. (A cast the const
            // domain CAN fold — `int'(f(3))` through the const-function interpreter,
            // or `int'(R)` through the real domain — returns before this fn is called,
            // so naming it here cannot false-loud a bound that has a value.)
            K::Cast { expr, .. } => {
                r(self, expr).or_else(|| Some("a cast that is not a constant here".to_string()))
            }
            // literals · Call · New · Dollar · Error: no bare net/hier
            // ref of their own (function calls are not descended — see doc).
            _ => None,
        }
    }

    /// The SCOPE-RESOLVED half of a structural (continuous-assign / net-declaration
    /// / gate-primitive) delay value. `const_delay_ticks` — the shared, scope-free
    /// spelling — is asked FIRST by every caller, so this only ever adds answers
    /// where that returned `None`; nothing it already folded changes.
    ///
    /// It had to exist, because the caller consumes `None` as a SILENT default: a
    /// delay that does not fold becomes NO delay. Measured against both oracles,
    /// `parameter D = 7; assign #(D) y = a;` propagated at t=1 instead of t=8, at
    /// exit 0 with no diagnostic — and `#(2+3)`, `#($clog2(32))`, `#(5ns)` and
    /// `#(RD)` for a `parameter real RD` were the same silence, because the
    /// scope-free fold is literal-only (`IntLit` / `(…)` / unary ±, then `_ => None`).
    ///
    /// Three lanes:
    ///
    ///   * A TIME LITERAL is scaled to global ticks HERE. `const_eval_in_scope`'s
    ///     `TimeLit` arm answers in MODULE units and declines when the literal is
    ///     not a whole multiple of the unit — and `#(5ns)` inside a `10ns/1ns`
    ///     module is half a unit, so that decline became the silent no-delay this
    ///     fn exists to remove (both oracles: 5 ns). The whole-multiple cells are
    ///     unchanged: that arm's answer × `cur_time_mult` IS this product.
    ///   * REAL when the expression mentions a real — asked with the SHADOW-CORRECT
    ///     resolver, and BEFORE the integer domain, but FALLING BACK to it rather
    ///     than returning (see the two ⚠️ notes in the body).
    ///     ⚠️⚠️ The reverse order is a measured silent-wrong and `param_real_value`
    ///     already records why: an exactly-integral `parameter real R = 11` keeps an
    ///     i64 TWIN in `params`, so the integer walk finds it and `#(R/2)` folds
    ///     INTEGER division — 5 where both oracles, and vita's own procedural
    ///     `#(R/2)`, say 5.5 ⇒ 6 ticks. A delay is a MAGNITUDE, so it needs
    ///     `param_real_value`'s order, not the one `const_truth_in_scope` can afford
    ///     (a truth test cannot see a truncation).
    ///   * INTEGER otherwise, through `const_unsigned_selfdet`. A delay value is a
    ///     SELF-DETERMINED position read as UNSIGNED — both oracles delay 0 for
    ///     `#(4'd15 + 4'd1)` (the 4-bit sum wraps) and 255 for a
    ///     `parameter signed [7:0] D = -8'sd1`. Folding it width-unlimited would
    ///     answer 16 and 4294967295, so this shares `$clog2`'s helper rather than
    ///     the plain `const_eval_in_scope`. A wholly integral `#(D/2)` therefore
    ///     keeps integer division (5 units for `D = 11`, both oracles).
    ///
    /// ⚠️ Deliberately NOT wired into `lower_delay` (the procedural `#delay`), which
    /// calls the scope-free `const_delay_ticks` to decide `Inactive` vs `Active`
    /// ONLY. That path lowers the amount as an expression the engine evaluates at
    /// suspension time, so it was never literal-limited; widening its region test
    /// would move `#(ZERO_PARAM)` from `Active` (with the engine's runtime
    /// `ticks == 0` nudge) into `Inactive` — a scheduling change with no defect
    /// behind it. Keeping the new rule opt-in at the one consumer that needs it is
    /// the shared-machinery rule in ENGINEERING_RULES.
    fn delay_ticks_in_scope(&self, e: &ast::Expr) -> Option<u32> {
        let mult = self.cur_time_mult;
        let pmult = self.cur_prec_mult;
        // min:typ:max picks typ — the same branch `const_delay_ticks` took before
        // handing the rest of the expression to its literal-only fold.
        let pick = match &e.kind {
            ast::ExprKind::MinTypMax { typ, .. } => typ.as_ref(),
            _ => e,
        };
        // Saturate, never wrap: the integer branch of `const_delay_ticks` records
        // why (a wrapped delay is a silent EARLY fire).
        let ticks = |v: u64| Some(v.saturating_mul(mult).min(u32::MAX as u64) as u32);
        if let ast::ExprKind::TimeLit { num, unit_exp } = &pick.kind {
            let val = self.const_unsigned_selfdet(
                num,
                &std::collections::BTreeMap::new(),
                &ConstWidths::new(),
                0,
            )?;
            // Sub-precision (finer than the design's global precision) declines, as
            // `const_eval_in_scope`'s arm does — there is no tick to round it to.
            let e = *unit_exp as i32 - self.global_prec_exp as i32;
            if e < 0 {
                return None;
            }
            // SATURATE on overflow, like every sibling lane — declining here would
            // hand the caller its silent no-delay, and a dropped delay fires EARLIER
            // than a clamped one. (`10^15 × 20000` overflows u64 under a `1s` unit at
            // `fs` precision; both oracles never fire it, and `min(u32::MAX)` doesn't
            // either, while `None` fires it at once. Both review lenses found this.)
            let t = 10u64
                .checked_pow(e as u32)
                .and_then(|m| m.checked_mul(val))
                .unwrap_or(u64::MAX);
            return Some(t.min(u32::MAX as u64) as u32);
        }
        // ⚠️ SHADOW-CORRECT realness (`shadow_correct = true`): this predicate is
        // CHOOSING a domain here, not widening one, so the blind `real_param_val`
        // walk is not good enough — an inner `localparam R = 9;` shadowing an outer
        // `parameter real R = 5;` sent `assign #(R)` into the real lane, which folded
        // the outer 5 where both oracles delay 9.
        // ⚠️ And the real lane FALLS BACK rather than returning: the real domain has
        // no `%`, no bit-ops, no shifts and no call/`$clog2` arm, so `#(RD % 4)`,
        // `#($clog2(RD))` and `#(half(RD))` over an integral `parameter real RD`
        // decline there — and a bare `return` on that decline is the silent no-delay
        // this whole fn exists to remove (measured: all three were correct before the
        // real lane was put first).
        if self.expr_mentions_real_opt(pick, true) {
            if let Some(x) = self.const_eval_real_in_scope(pick) {
                return Some(real_delay_ticks(x, mult, pmult));
            }
        }
        self.const_unsigned_selfdet(
            pick,
            &std::collections::BTreeMap::new(),
            &ConstWidths::new(),
            0,
        )
        .and_then(ticks)
    }

    /// One delay value → ticks: the scope-free fold, then the scope-resolved one.
    fn ca_delay_value(&self, e: &ast::Expr) -> Option<u32> {
        const_delay_ticks(e, self.cur_time_mult, self.cur_prec_mult)
            .or_else(|| self.delay_ticks_in_scope(e))
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
        let uniform = delay.and_then(|d| {
            let e = d.values.first()?;
            // ⚠️ A SCOPE-resolved RISE of 0 keeps the pre-slice shape (`None` = no
            // delay) instead of becoming `Some(0)`. This is the one value where the
            // silent default was already BOTH oracles' answer, and `Some(0)` is not
            // the same thing in this engine: it routes the assign onto the delayed
            // path, where a zero-tick write lands a delta LATER than either oracle
            // (measured: `assign #0 y = a;` still reads 0 after two `#0` hops where
            // iverilog and verilator both read 1). Turning `#(ZERO_PARAM)` into
            // `Some(0)` would therefore trade a correct answer for that pre-existing
            // lag — a rung DOWN the ladder. The lag itself (and the resulting
            // `#0` vs `#(ZERO_PARAM)` split) is the literal spelling's, untouched
            // here and recorded in ROADMAP §2.
            match const_delay_ticks(e, self.cur_time_mult, self.cur_prec_mult) {
                Some(t) => Some(t),
                None => self.delay_ticks_in_scope(e).filter(|&t| t != 0),
            }
        });
        // The sidecar is only ever CONSULTED on the delayed path, which the engine
        // enters on `delay.is_some()` — so an rft triple under a `None` uniform is
        // dead weight, and computing it would be the only way this fn could emit
        // one. Pre-slice this was implicit (both used the same fold, so values[0]
        // failing meant the `folded?` below failed too); the zero-rise rule above
        // makes the two able to disagree, so it is now spelled out.
        let rft = uniform.and(delay).and_then(|d| {
            // Only 2- or 3-value specs can carry a distinct fall/turnoff.
            if d.values.len() < 2 {
                return None;
            }
            // NOT the zero-suppressing form: a FALL or TURNOFF of 0 is a real,
            // distinct edge delay (`#(5,0)` — both oracles fall immediately) and
            // reaches the engine through the sidecar, not through `ContAssign.delay`.
            // ⚠️ The converse does NOT hold: a scope-folded RISE of 0 suppresses the
            // uniform above, which kills this whole triple — so `#(ZERO_PARAM, 9)`
            // keeps the pre-slice no-delay and its fall stays wrong, while the
            // literal twin `#(0, 9)` is correct. Both review lenses found it; it is
            // PRE-identical (no regression) and it is a TRADE, not an oversight —
            // emitting `Some(0)` + sidecar fixes the fall and breaks the rise on the
            // `#0` lag above, and both halves are 2-oracle-agreed. ROADMAP §2 owns
            // it, with the lag named as the root that unblocks both.
            let folded: Option<Vec<u32>> =
                d.values.iter().map(|e| self.ca_delay_value(e)).collect();
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

    /// The SIGNEDNESS a parameter read materializes with — the ONE spelling, used by
    /// the lowering ([`Self::const_param_expr_w`], which builds the const) and by the
    /// size-cast classifier (`ast_ctx_signed`, which decides how every leaf of the
    /// cast operand is extended). Two spellings would let the classifier extend a leaf
    /// one way while the lowering builds it the other.
    ///
    /// ⚠️ It has to be USED by both, not merely documented as shared: §4.5.318 first
    /// added it as a second copy of the rule and a reviewer's `always false` mutation
    /// passed the ENTIRE 5,291-test workspace gate — the sign rule had no teeth
    /// anywhere. `const_param_expr_w` now derives its `signed` from here, so a
    /// mutation moves the lowering too.
    ///
    /// Declared meta wins; without it the value decides, exactly as
    /// [`Self::const_param_expr`] does (`v >= 0` lands in the unsigned 32/64-bit
    /// arms, `v < 0` in the signed ones).
    pub(crate) fn param_const_signed(v: i64, meta: Option<(u32, bool)>) -> bool {
        match meta {
            Some((w, signed)) if w >= 1 => signed,
            _ => v < 0,
        }
    }

    /// Materialize a param read at its DECLARED `(width, signed)` when known
    /// (`param_meta`), else fall back to the value-inferred width
    /// ([`Self::const_param_expr`]). A typed param's const therefore carries its
    /// real width: `localparam logic [63:0] P = '1` reads as a 64-bit all-ones
    /// const, and `logic [3:0] x = 5` reads as a 4-bit const, matching iverilog.
    pub(crate) fn const_param_expr_w(&mut self, v: i64, meta: Option<(u32, bool)>) -> u32 {
        let signed = Self::param_const_signed(v, meta);
        match meta {
            Some((w, _)) if (1..=64).contains(&w) => {
                let cv = make_const_i64(v, w, signed);
                let cid = self.intern_const(cv);
                self.push_expr(ir::Expr::Const { val: cid })
            }
            // WIDER than 64 and still an i64 value — `localparam logic [95:0] K = 7`,
            // and every wide parameter whose initializer folds. This used to fall
            // through to the value-inferred width, so a 96-bit parameter read back as
            // 32 bits: `%h` printed 8 digits where iverilog prints 24, and the same
            // width fed concats and comparisons. Widen the i64 to the declared width
            // through the shared `resize_bits`, which sign-extends across words.
            Some((w, _)) if w > 64 => {
                let base = make_const_i64(v, 64, signed);
                let cv = ir::ConstVal {
                    width: w,
                    signed,
                    repr: ir::ConstRepr::Numeric,
                    bits: resize_bits(&base.bits, 64, w, signed),
                };
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
