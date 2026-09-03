//! The WIDE (bit-vector) constant domain — see the section banner below.

use super::*;

// ── wide (>64-bit) CARRY-FREE constant folding ───────────────────────────────
//
// `wide_param_bits` has always been able to REPRESENT a >64-bit parameter; what it
// could not do is compute one. `fold_init` handled a literal and a parenthesised
// literal, so `localparam logic [127:0] K = 128'h…` worked and
// `localparam logic [127:0] K = {8'he1, 120'h0}` was `E3009 … not a foldable
// constant expression` — the spelling every crypto IP actually uses.

/// One folded value in the wide bit domain: `(bits, self width, signed)`.
pub(crate) type WideBits = (ir::BitPacked, u32, bool);

/// Resolves a NAME NODE (a bare `Ident` or a `pkg::name`) to an already-folded
/// constant. Takes the whole expression rather than a `HierPath` so the two
/// spellings share one hook — a package-scoped name is not a `HierPath` at all, and
/// giving it a second resolver would be a second scope walk (§4.5.218's shape).
/// The `bool` is `is_count`: true in a COUNT / SIZE / INDEX position (a replication
/// count, a size cast's width, a select bound), where only a value that survives
/// elaboration may answer.
///
/// ⚠️⚠️ The flag is not decoration. Without it, folding a count through the ordinary
/// resolver let a const-function's runtime LOCAL supply one: `int n = 2; int x =
/// {n{4'hA}};` folded 170 where iverilog rejects the function outright — §4.5.371's
/// blocking defect ⓶, reproduced exactly. A local that is merely SHADOWING must also
/// stop the walk rather than fall through to a same-named module parameter (that
/// slice's ⓷, which installed 43690 in place of 170).
pub(crate) type WideNameFn<'a> = &'a dyn Fn(&ast::Expr, bool) -> Option<WideBits>;

/// A zeroed bit vector wide enough for `width` bits.
pub(crate) fn bp_zero(width: u32) -> ir::BitPacked {
    let n = ((width as usize).div_ceil(64)).max(1);
    ir::BitPacked {
        val: vec![0; n],
        unk: vec![0; n],
    }
}

pub(crate) fn bp_get(bp: &ir::BitPacked, i: usize) -> (bool, bool) {
    let g = |p: &[u64]| {
        p.get(i / 64)
            .map(|w| (w >> (i % 64)) & 1 == 1)
            .unwrap_or(false)
    };
    (g(&bp.val), g(&bp.unk))
}

pub(crate) fn bp_set(bp: &mut ir::BitPacked, i: usize, v: bool, u: bool) {
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

pub(crate) fn bp_any_unknown(bp: &ir::BitPacked, width: u32) -> bool {
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
/// A COUNT / SIZE inside the wide domain — a replication count, a size cast's width,
/// a select index — folded through the domain's OWN name resolver.
///
/// ⭐ These used to be `const_eval_u32` alone, which is literal-only, so
/// `localparam logic [5:0] PV = {N{2'b01}};` was `E3009` for a `localparam int N = 3`
/// two lines up — the per-port-vector idiom every AXI/Ethernet generator emits.
///
/// ⚠️⚠️ §4.5.371 built a wider count and REVERTED it, and its four blocking defects
/// are exactly what routing through `name` avoids, one for one. ⓵ *"the width twin was
/// not widened, so the width-aware walk went width-unlimited"* — there is no twin to
/// forget here: `const_placement_wide` (the width consumer) and the value consumers
/// call THIS function, so one edit widens both. ⓶/⓷ *"the name resolver caught a
/// const-function LOCAL / skipped past a shadowing local to the module parameter"* —
/// `name` is the resolver the surrounding fold already uses, and both of its
/// implementations consult the innermost binding first. ⓸ *"the evaluator restarted
/// the call depth at 0 and a call inside the count overflowed the stack"* — this walk
/// calls no evaluator; it recurses over the AST it was given.
///
/// The literal fold is asked FIRST so every count that folded before takes exactly the
/// route it did.
fn fold_count(e: &ast::Expr, name: WideNameFn) -> Option<u32> {
    if let Some(n) = const_eval_u32(e) {
        return Some(n);
    }
    let (b, w, _) = fold_self_bits(e, &|n, _| name(n, true))?;
    if bp_any_unknown(&b, w) {
        return None;
    }
    // §11.4.12.1: a replication count is UNSIGNED. Anything that does not fit a u32
    // cannot be a count this domain will build (`MAX_NET_WIDTH` caps the product).
    let mut v: u64 = 0;
    for i in 0..(w as usize).min(64) {
        if bp_get(&b, i).0 {
            v |= 1u64 << i;
        }
    }
    if (w as usize) > 64 && (64..w as usize).any(|i| bp_get(&b, i).0) {
        return None;
    }
    u32::try_from(v).ok()
}

/// A SHIFT AMOUNT in the wide domain — the one count position where an amount past
/// `u32::MAX` has a CORRECT answer instead of a loud one.
///
/// ⚠️ [`fold_count`] is fail-closed above `u32::MAX` because a replication count, a
/// size cast's width and a select index all become nonsense there. A shift does not:
/// §11.4.10 vacates with zeros (with the sign bit for `>>>`), so any amount at or
/// above the left operand's width gives the same answer whatever its exact value is,
/// and `MAX_NET_WIDTH` (2**20) is far below the 2**32 this saturates at. Both oracles
/// agree — `64'hDEAD_BEEF_1234_5678 >> 64'h1_0000_0000` is `0` in iverilog and in
/// verilator, with no diagnostic from either, and vita answered the operand UNSHIFTED
/// until the `const_eval_u32` truncation this saturation replaces.
///
/// `usize::MAX` is safe to hand both shift loops: it drives `i.checked_add(k)` /
/// `i.checked_sub(k)` to `None` for every bit, which is the all-vacated result.
fn fold_shift_count(e: &ast::Expr, name: WideNameFn) -> Option<usize> {
    // ⚠️⚠️ A shift amount is SELF-determined (§11.4.10), and an unsized fill in a
    // self-determined position is ONE BIT (§5.7.1) — so `'1` is a shift by 1 and `'0` a
    // shift by 0, which is what both oracles do. `const_eval_u32` below has no fill arm
    // and sizes one at a hard 32, so `'1` came back `0xFFFFFFFF` and the saturation two
    // lines down turned every such shift into zero: `localparam logic [39:0] R =
    // 40'hFF << '1;` was `0000000000` at exit 0 where iverilog and verilator both give
    // `00000001fe`. Its `>>` twin was `0000000000` for their `000000007f`.
    //
    // ⭐ This is a value→value correction on its own: every cell it moves already had a
    // (wrong) number, because the fill sits in the COUNT and the walk folds the left
    // operand as it always did. Nothing that was loud becomes a value.
    //
    // ⚠️ An x/z fill still declines — there is no shift amount it could name — and
    // `fill_to_i64` is the helper that says so.
    if let Some((raw, kind)) = const_eval::fill_literal_ast(e) {
        return const_eval::fill_to_i64(kind, raw, 1).map(|v| v as usize);
    }
    if let Some(n) = const_eval_u32(e) {
        return Some(n as usize);
    }
    let (b, w, _) = fold_self_bits(e, &|n, _| name(n, true))?;
    if bp_any_unknown(&b, w) {
        return None;
    }
    // §11.4.10: the amount is UNSIGNED. A set bit at or above 32 puts it past
    // `u32::MAX`, hence past every buildable width — saturate.
    if (32..w as usize).any(|i| bp_get(&b, i).0) {
        return Some(usize::MAX);
    }
    let mut v: usize = 0;
    for i in 0..(w as usize).min(32) {
        if bp_get(&b, i).0 {
            v |= 1usize << i;
        }
    }
    Some(v)
}

pub(crate) fn fold_self_bits(
    e: &ast::Expr,
    // Resolve a NAME to an already-folded constant. `None` = names decline, which is
    // what a caller without an elaborator (a class field default) must pass — it has
    // no parameter table to consult and inventing one here would be a second scope
    // walk beside `walk_scopes_key`, the exact shape §4.5.218 turned into a silent
    // generate-body deletion.
    name: WideNameFn,
) -> Option<WideBits> {
    fold_bits_at(e, 0, name)
}

/// [`fold_self_bits`] with a CONTEXT width (`0` = none), which is the whole of §11.6.1
/// this domain used to be missing.
///
/// A context-determined operator takes its width from the surrounding assignment, not
/// from its operands, and folding it at the operands' width and extending afterwards is
/// a different answer whenever the operation carries information past their top bit.
/// `localparam logic [127:0] C = ~32'd0;` was `0000000000000000ffffffffffffffff` at exit
/// 0 where both oracles give 128 ones — the complement ran at 32 bits and the extension
/// could not put back what was never computed. A 108-cell census found **48 such cells**;
/// the `**` shapes the row was filed from are the rare LOUD corner of the same gate.
///
/// ⭐ Nothing new installs the answer: `wide_disagreeing_value` already prefers the wide
/// domain whenever it disagrees with the i64 one, so the cells become correct the moment
/// this stops folding narrow.
///
/// ⚠️⚠️ THE CLASSIFICATION IS THE FIFTH COPY OF ITSELF, and saying otherwise was the
/// first thing review corrected. [`crate::binop_result_is_context_determined`] is the
/// canonical list and its own docstring already records four hand-written copies living
/// outside it; the arms below are a sixth reading, because each one needs the operand
/// STRUCTURE and not just a yes/no. The drift is not hypothetical — the bitwise arm was
/// missing `BitXnor`, which the list has, and `96'hF0 ~^ 96'h0F` at 128 bits was
/// consequently wrong in both PRE and POST. Adding a context-determined operator means
/// editing here too, and a `_`-free match is what makes that a compile error rather than
/// a silent default.
///
/// The two positions the LRM carves OUT of that list — a shift's RIGHT operand
/// (§11.4.10) and `**`'s exponent (Table 11-21) — are self-determined and recurse with
/// no context, which is the same rule §2 row 27 established for the ≤64-bit walk.
///
/// ⚠️ A size cast is its own context (§11.6.1), so `ctx` stops there rather than passing
/// through: in `localparam logic [255:0] C = 128'(~32'd0);` the complement runs at the
/// CAST's 128 bits and the result is then widened to 256 — not complemented at 256.
fn fold_bits_at0(e: &ast::Expr, name: WideNameFn) -> Option<WideBits> {
    fold_bits_at(e, 0, name)
}

/// Extend an already-folded value to at least `ctx` bits, filling per `sg`.
///
/// ⚠️⚠️ `sg` is the EXPRESSION's signedness, not the value's own. §11.8.2 decides an
/// expression's sign FIRST — unsigned if any context-determined operand is unsigned —
/// and then each operand is reinterpreted at that sign, at its own width, before being
/// extended. Filling with the operand's own sign made
/// `localparam signed [95:0] S = 96'sh8000…; localparam logic [127:0] C = S + 96'd16;`
/// come out `ffffffff8000…0010` where both oracles give `000000008000…0010` — and where
/// **vita's own runtime already gave the oracles' answer**, so the constant domain was
/// contradicting the runtime. Both review lenses found it independently; 72 cells.
///
/// ⚠️ An UNKNOWN value is not widened at all. `resize_bits` replicates the top bit, so
/// an x in the MSB became an x in every added bit — `8'bxxxx_0000 << 2` at 128 bits
/// printed 120 x's where both oracles print zeros above bit 7. That `resize_bits`
/// behaviour is pre-existing and visible without this slice (a concat shows it), so the
/// fix here is to decline rather than to widen, which restores the pre-slice loud.
fn widen_to(v: WideBits, ctx: u32, sg: bool) -> Option<WideBits> {
    let (b, w, own) = v;
    if w >= ctx {
        return Some((b, w, own));
    }
    if bp_any_unknown(&b, w) {
        return None;
    }
    Some((resize_bits(&b, w, ctx, sg), ctx, own))
}

/// [`bp_operands`] with a floor on the common width — §11.6.1's context.
///
/// The common SIGN is computed first (`ls && rs`, §11.8.2) and both operands are
/// extended with it.
fn bp_operands_at(l: &WideBits, r: &WideBits, ctx: u32) -> Option<(Vec<u64>, Vec<u64>, u32, bool)> {
    let sg = l.2 && r.2;
    bp_operands(
        &widen_to(l.clone(), ctx, sg)?,
        &widen_to(r.clone(), ctx, sg)?,
    )
}

pub(crate) fn fold_bits_at(e: &ast::Expr, ctx: u32, name: WideNameFn) -> Option<WideBits> {
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
        ast::ExprKind::Paren { inner } => fold_bits_at(inner, ctx, name),
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
            let n = cap(u64::from(fold_count(w, name)?))?;
            // A cast IS a context (§11.6.1) — `ctx` stops here and `n` takes over.
            let (b, bw, sg) = fold_bits_at(expr, n, name)?;
            // ⚠️⚠️ A size cast is a CONTEXT for its operand (§11.6.1: the operand is
            // evaluated at `max(its self width, N)`), not a truncation applied
            // afterwards. Folding at the operand's own width and resizing is the same
            // answer only when the operand decides its own width, or when it is already
            // at least as wide as the cast. Measured the hard way: `65'(64'd18446744073709551615
            // + 64'd1) >> 64` wrapped the 64-bit addition to 0 and shifted 0, where both
            // oracles carry the sum into bit 64 and answer 1. This walk cannot widen a
            // context, so it declines — which is exactly where that expression was.
            if bw < n && !wide_top_is_self_determined(expr) {
                return None;
            }
            Some((resize_bits(&b, bw, n, sg), n, sg))
        }
        // §11.4.12 concatenation: unsigned, leftmost part most significant.
        ast::ExprKind::Concat { parts } => {
            let (b, w) = fold_concat_parts(parts, &cap, name)?;
            Some((b, w, false))
        }
        // §11.4.12.1 replication: unsigned; the count is a constant.
        ast::ExprKind::Replicate { count, value } => {
            let n = fold_count(count, name)?;
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
        // directions. `>>>` is not here — the arithmetic RIGHT shift reads the sign
        // bit, and has its own arm below.
        //
        // ⭐ `<<<` rides this arm because at a FIXED width it IS the logical shift:
        // §11.4.10 gives an arithmetic LEFT shift the same zero fill `<<` has, and
        // only the right shift differs. It used to fall to the catch-all, so
        // `A <<< 4` was E3009 where `A << 4` folded — which of two spellings of one
        // operation you wrote decided whether the parameter existed.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(op, ast::BinOp::Shl | ast::BinOp::AShl | ast::BinOp::Shr) =>
        {
            let k = fold_shift_count(rhs, name)?;
            // §11.4.10: the LEFT operand takes the context, the count does not.
            let (b0, w0, sg) = fold_bits_at(lhs, ctx, name)?;
            let w = w0.max(ctx);
            let (b, _, _) = widen_to((b0, w0, sg), w, sg)?;
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                // source index for result bit i
                let src = match op {
                    ast::BinOp::Shl | ast::BinOp::AShl => i.checked_sub(k),
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
                ast::BinOp::BitAnd | ast::BinOp::BitOr | ast::BinOp::BitXor | ast::BinOp::BitXnor
            ) =>
        {
            let (lb, lw, ls) = fold_bits_at(lhs, ctx, name)?;
            let (rb, rw, rs) = fold_bits_at(rhs, ctx, name)?;
            if bp_any_unknown(&lb, lw) || bp_any_unknown(&rb, rw) {
                return None;
            }
            let w = lw.max(rw).max(ctx);
            // §11.8.2: the expression's sign decides the fill for BOTH operands.
            let cs = ls && rs;
            let (la, ra) = (resize_bits(&lb, lw, w, cs), resize_bits(&rb, rw, w, cs));
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                let (a, _) = bp_get(&la, i);
                let (c, _) = bp_get(&ra, i);
                let v = match op {
                    ast::BinOp::BitAnd => a && c,
                    ast::BinOp::BitOr => a || c,
                    ast::BinOp::BitXnor => !(a ^ c),
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
            let (b0, w0, sg) = fold_bits_at(operand, ctx, name)?;
            if bp_any_unknown(&b0, w0) {
                return None;
            }
            // ⭐ THE CENSUS'S BIGGEST CELL. `~32'd0` at a 128-bit target complements at
            // 128, not at 32 and then extends — the extension cannot put back bits the
            // complement never computed.
            let w = w0.max(ctx);
            let (b, _, _) = widen_to((b0, w0, sg), w, sg)?;
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                bp_set(&mut out, i, !bp_get(&b, i).0, false);
            }
            Some((out, w, sg))
        }
        // §11.4.10 ARITHMETIC right shift: the vacated bits copy the LEFT operand's
        // sign bit when it is signed, zero otherwise. It reads the sign bit, which is
        // why it sat outside the carry-free set — but a sign bit is a bit at a KNOWN
        // position, and reading it here is the same act `resize_bits` already performs
        // on every signed extension in this file.
        ast::ExprKind::Binary {
            op: ast::BinOp::AShr,
            lhs,
            rhs,
        } => {
            let k = fold_shift_count(rhs, name)?;
            let (b0, w0, sg) = fold_bits_at(lhs, ctx, name)?;
            let w = w0.max(ctx);
            let (b, _, _) = widen_to((b0, w0, sg), w, sg)?;
            let hi = w as usize - 1;
            let (fill_v, fill_u) = if sg { bp_get(&b, hi) } else { (false, false) };
            let mut out = bp_zero(w);
            for i in 0..w as usize {
                let (v, u) = match i.checked_add(k).filter(|s| *s < w as usize) {
                    Some(si) => bp_get(&b, si),
                    None => (fill_v, fill_u),
                };
                bp_set(&mut out, i, v, u);
            }
            Some((out, w, sg))
        }
        // §11.4.14 REDUCTION: one bit out of any width in, and SELF-DETERMINED — the
        // surrounding context supplies nothing. This is the axis §4.5.373 built,
        // measured and reverted, and the reason it failed then was never the operator:
        // it read the operand's width out of `param_meta`, where INFERRED widths live,
        // so `localparam W = 4'hF | 4'h0;` reduced a 32-bit value that both oracles
        // hold at 4 bits. Here the operand's bits arrive already canonical at a
        // DECLARED width (see `narrow_param_bits`), so there is nothing left to guess.
        ast::ExprKind::Unary { op, operand }
            if matches!(
                op,
                ast::UnOp::RedAnd
                    | ast::UnOp::RedOr
                    | ast::UnOp::RedXor
                    | ast::UnOp::RedNand
                    | ast::UnOp::RedNor
                    | ast::UnOp::RedXnor
            ) =>
        {
            let (b, w, _) = fold_bits_at0(operand, name)?;
            if bp_any_unknown(&b, w) {
                return None;
            }
            let bits = (0..w as usize).map(|i| bp_get(&b, i).0);
            let mut bits = bits;
            let r = match op {
                ast::UnOp::RedAnd | ast::UnOp::RedNand => bits.all(|x| x),
                ast::UnOp::RedOr | ast::UnOp::RedNor => bits.any(|x| x),
                _ => bits.fold(false, |a, x| a ^ x),
            };
            let inv = matches!(
                op,
                ast::UnOp::RedNand | ast::UnOp::RedNor | ast::UnOp::RedXnor
            );
            Some(bp_bit(r != inv))
        }
        // §11.4.7 logical negation — one bit, self-determined, and it reads the whole
        // operand rather than a bit position, so it belongs with the reductions.
        ast::ExprKind::Unary {
            op: ast::UnOp::LogNot,
            operand,
        } => {
            let (b, w, _) = fold_bits_at0(operand, name)?;
            if bp_any_unknown(&b, w) {
                return None;
            }
            Some(bp_bit(!(0..w as usize).any(|i| bp_get(&b, i).0)))
        }
        // §11.4.3 arithmetic. Truncating to the common width is not a shortcut — it
        // IS the rule: `8'hFF + 8'h2` is 1 in every tool, because the carry out of
        // the context width has nowhere to go.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(op, ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul) =>
        {
            let l = fold_bits_at(lhs, ctx, name)?;
            let r = fold_bits_at(rhs, ctx, name)?;
            let (a, b, w, sg) = bp_operands_at(&l, &r, ctx)?;
            let v = match op {
                ast::BinOp::Add => limbs_add(&a, &b, w),
                ast::BinOp::Sub => limbs_add(&a, &limbs_neg(&b, w), w),
                _ => limbs_mul(&a, &b, w),
            };
            Some((bp_from_limbs(v, w), w, sg))
        }
        // §11.4.3 division and modulus — the two the arm above deliberately left
        // out ("a wide divide is a different algorithm, and `x/0` has no value
        // here"). Both halves of that are still true, and neither is a reason to
        // decline any more: the algorithm is `sim_ir::mw::mw_divmod`, the SAME
        // function the runtime evaluator calls, and `x/0` still declines.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) =>
        {
            let l0 = fold_bits_at(lhs, ctx, name)?;
            let r0 = fold_bits_at(rhs, ctx, name)?;
            let sg = l0.2 && r0.2;
            let l = widen_to(l0, ctx, sg)?;
            let r = widen_to(r0, ctx, sg)?;
            wide_divmod(matches!(op, ast::BinOp::Div), &l, &r)
        }
        // §11.4.10 power. ⚠️ NOT folded through `bp_operands` like its arithmetic
        // siblings: Table 11-21 makes the exponent SELF-determined while the base
        // takes the context, so each side folds at its own width and the RESULT is
        // the base's. See `wide_pow`.
        ast::ExprKind::Binary {
            op: ast::BinOp::Pow,
            lhs,
            rhs,
        } => {
            // Table 11-21: the BASE takes the context, the exponent is
            // self-determined — the same carve-out §2 row 27 made for a shift count.
            // The result's sign is the BASE's, so the base extends in its own.
            let l0 = fold_bits_at(lhs, ctx, name)?;
            let ls = l0.2;
            let l = widen_to(l0, ctx, ls)?;
            let r = fold_bits_at(rhs, 0, name)?;
            wide_pow(&l, &r)
        }
        // §11.4.5 unary minus: the two's complement at the operand's own width.
        ast::ExprKind::Unary {
            op: ast::UnOp::Minus,
            operand,
        } => {
            let (b0, w0, sg) = fold_bits_at(operand, ctx, name)?;
            if bp_any_unknown(&b0, w0) {
                return None;
            }
            let w = w0.max(ctx);
            let (b, _, _) = widen_to((b0, w0, sg), w, sg)?;
            Some((bp_from_limbs(limbs_neg(&b.val, w), w), w, sg))
        }
        ast::ExprKind::Unary {
            op: ast::UnOp::Plus,
            operand,
        } => fold_bits_at(operand, ctx, name),
        // §11.4.4 / §11.4.5: relational and equality operators deliver ONE UNSIGNED
        // BIT and size their operands against EACH OTHER. That makes the whole node
        // self-determined, which is what lets a caller extend the result to a wider
        // declaration without re-folding at the wider width.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(
                op,
                ast::BinOp::Lt
                    | ast::BinOp::Le
                    | ast::BinOp::Gt
                    | ast::BinOp::Ge
                    | ast::BinOp::Eq
                    | ast::BinOp::Ne
                    | ast::BinOp::CaseEq
                    | ast::BinOp::CaseNe
            ) =>
        {
            let l = fold_bits_at0(lhs, name)?;
            let r = fold_bits_at0(rhs, name)?;
            let (a, b, w, sg) = bp_operands(&l, &r)?;
            let ord = limbs_cmp(&a, &b, w, sg);
            use std::cmp::Ordering::*;
            Some(bp_bit(match op {
                ast::BinOp::Lt => ord == Less,
                ast::BinOp::Le => ord != Greater,
                ast::BinOp::Gt => ord == Greater,
                ast::BinOp::Ge => ord != Less,
                ast::BinOp::Eq | ast::BinOp::CaseEq => ord == Equal,
                _ => ord != Equal,
            }))
        }
        // §11.4.7 logical AND/OR — one bit, operands read as truth values, and each
        // operand is SELF-determined (no common width), so `bp_operands` is not used.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr) =>
        {
            let truth = |e: &ast::Expr| -> Option<bool> {
                let (b, w, _) = fold_bits_at0(e, name)?;
                if bp_any_unknown(&b, w) {
                    return None;
                }
                Some((0..w as usize).any(|i| bp_get(&b, i).0))
            };
            let (a, b) = (truth(lhs)?, truth(rhs)?);
            Some(bp_bit(if matches!(op, ast::BinOp::LogAnd) {
                a && b
            } else {
                a || b
            }))
        }
        // §11.4.11 conditional: the CONDITION is self-determined; the two arms size
        // against each other, so the node's own width is the max of theirs. Both arms
        // are folded because the unselected one still contributes its width.
        ast::ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            // §11.4.11: the CONDITION is self-determined; both arms take the context.
            let (cb, cw, _) = fold_bits_at(cond, 0, name)?;
            if bp_any_unknown(&cb, cw) {
                return None;
            }
            let c = (0..cw as usize).any(|i| bp_get(&cb, i).0);
            let (tb, tw, ts) = fold_bits_at(then_e, ctx, name)?;
            let (eb, ew, es) = fold_bits_at(else_e, ctx, name)?;
            let w = tw.max(ew).max(ctx);
            let sg = ts && es;
            let (b, from, _) = if c { (tb, tw, ts) } else { (eb, ew, es) };
            // §11.8.2 again: the chosen arm extends with the EXPRESSION's sign.
            let (b, _, _) = widen_to((b, from, sg), w, sg)?;
            Some((b, w, sg))
        }
        // §11.5.1 / §11.5.2 BIT and PART select: PLACEMENT, like the concat above —
        // each result bit is an operand bit at a known position. The selected range
        // is read against the operand's OWN width, which is exactly what the wide
        // domain carries, so this needs no declared-range side table.
        ast::ExprKind::BitSelect { base, index } => {
            // `const_eval_u32` is the same folder the size cast and the replication
            // count use here; a NEGATIVE index declines through it, which is the
            // fail-closed answer (an out-of-range select is `x`, and this 2-state
            // domain will not claim one).
            let i = fold_count(index, name)?;
            let (b, w, _) = fold_bits_at0(base, name)?;
            if i >= w {
                return None;
            }
            let (v, u) = bp_get(&b, i as usize);
            if u {
                return None;
            }
            Some(bp_bit(v))
        }
        ast::ExprKind::PartSelect { base, msb, lsb } => {
            let (m, l) = (fold_count(msb, name)?, fold_count(lsb, name)?);
            let (b, w, _) = fold_bits_at0(base, name)?;
            // A DESCENDING select only. An ascending one (`A[0:7]`) is legal against
            // an ascending declaration, and the declared DIRECTION is exactly what
            // this domain does not carry — so it declines rather than read the range
            // backwards (the §4.5.363 rule, at the bit domain's own boundary).
            if m < l || m >= w {
                return None;
            }
            let n = m - l + 1;
            let mut out = bp_zero(n);
            for i in 0..n as usize {
                let (v, u) = bp_get(&b, l as usize + i);
                bp_set(&mut out, i, v, u);
            }
            Some((out, n, false))
        }
        // §11.5.1 INDEXED part-select `A[base +: n]` / `A[base -: n]`. The width is a
        // constant and the base need not be — but here it must fold, because the domain
        // has no runtime. This is the shape `axi_crossbar` uses to slice one port's
        // field out of a per-port vector (`M_ISSUE[n*32 +: 32]`), and it is why a wide
        // parameter needed a select arm at all.
        ast::ExprKind::IndexedPart {
            base,
            offset,
            width,
            dir,
        } => {
            let n = fold_count(width, name)?;
            let off = fold_count(offset, name)?;
            let (b, w, _) = fold_bits_at0(base, name)?;
            // `-:` counts DOWN from the offset, so its low bit is `off - n + 1`.
            let lo = match dir {
                ast::PartDir::PlusColon => off,
                ast::PartDir::MinusColon => off.checked_sub(n.checked_sub(1)?)?,
            };
            if n == 0 || lo.checked_add(n)? > w {
                return None;
            }
            let mut out = bp_zero(n);
            for i in 0..n as usize {
                let (v, u) = bp_get(&b, lo as usize + i);
                bp_set(&mut out, i, v, u);
            }
            Some((out, n, false))
        }
        // §20.9 bit-vector system functions, in the domain that actually holds the
        // bits. All are self-determined; `$signed`/`$unsigned` only RELABEL.
        ast::ExprKind::SysCall { name: f, args } if args.len() == 1 => {
            let arg = || fold_bits_at0(&args[0], name);
            match f.name.as_str() {
                "$signed" => arg().map(|(b, w, _)| (b, w, true)),
                "$unsigned" => arg().map(|(b, w, _)| (b, w, false)),
                // §20.8.1: the argument is self-determined and read UNSIGNED; the
                // result is an `integer` — 32 bits, signed. The value is a bit
                // INDEX, so it always fits, which is the point: the integral
                // domain's `$clog2` route (`selfdet_bits_unsigned`) declines the
                // moment the argument's magnitude passes 64 bits, and
                // `localparam int AW = $clog2(MAX);` over a crypto-width `MAX` is
                // the standard width idiom.
                "$clog2" => {
                    let (b, w, _) = arg()?;
                    Some(int32(wide_clog2(&b, w)?))
                }
                // §20.6.2: the number of bits the expression needs — which is the
                // SELF width this whole walk computes, so the arm is one line.
                //
                // ⚠️ The width is trustworthy here for the reason `narrow_param_bits`
                // spends a paragraph on: a NAME only answers from `param_range` /
                // `pkg_const_range`, the DECLARED-provenance maps. §4.5.373 measured
                // what happens when a width-relative operator reads `param_meta`
                // instead (an INFERRED width picked the opposite generate branch at
                // exit 0), and that is why `bits_of_selfdet` deliberately excludes
                // `const_self_width`. This domain is not that map. Verified against
                // iverilog on twelve shapes — name, select, concat, `+`, `>`, `/`,
                // `**`, reduction, `<<`, `?:`, a signed name, `$clog2` — all
                // identical, and identical to vita's own RUNTIME `$bits` for the
                // same text.
                "$bits" => {
                    let (_, w, _) = arg()?;
                    Some(int32(u64::from(w)))
                }
                // §20.9: 1 if ANY bit is x/z. The placement arms carry unknown bits
                // through, so this domain can answer it exactly — and it is the one
                // question here that WANTS an unknown rather than declining on it.
                "$isunknown" => {
                    let (b, w, _) = arg()?;
                    Some(bp_bit(bp_any_unknown(&b, w)))
                }
                "$countones" | "$onehot" | "$onehot0" => {
                    let (b, w, _) = arg()?;
                    if bp_any_unknown(&b, w) {
                        return None;
                    }
                    let n = (0..w as usize).filter(|&i| bp_get(&b, i).0).count() as u64;
                    match f.name.as_str() {
                        "$onehot" => Some(bp_bit(n == 1)),
                        "$onehot0" => Some(bp_bit(n <= 1)),
                        // §20.9: `$countones` returns an `int` — 32 bits, signed.
                        _ => Some(int32(n)),
                    }
                }
                _ => None,
            }
        }
        ast::ExprKind::Ident(_) | ast::ExprKind::PkgScoped { .. } => name(e, false),
        _ => None,
    }
}

/// Is `e`'s TOP node SELF-DETERMINED — does it fix its own width without asking the
/// context (§11.6.1 Table 11-21)?
///
/// This is the gate that lets a caller fold at the self width and then EXTEND to a
/// wider declaration. For a context-determined top (`+`, `~`, `<<`, a bitwise op, a
/// conditional) the operands would have been extended FIRST and the answer differs:
/// `localparam logic [127:0] Q = B << 4;` with an 8-bit `B` shifts inside 128 bits in
/// every tool, and inside 8 here — the bits `<<` pushes past bit 7 are gone before
/// the extension can save them. Such a top is admitted only when the fold's own
/// width already covers the declaration, where extending is a no-op.
pub(crate) fn wide_top_is_self_determined(e: &ast::Expr) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Paren { inner } => wide_top_is_self_determined(inner),
        K::IntLit { .. }
        | K::Ident(_)
        | K::Concat { .. }
        | K::Replicate { .. }
        | K::BitSelect { .. }
        | K::PartSelect { .. }
        | K::IndexedPart { .. }
        | K::SysCall { .. }
        | K::Cast { .. } => true,
        K::Unary { op, .. } => matches!(
            op,
            ast::UnOp::RedAnd
                | ast::UnOp::RedOr
                | ast::UnOp::RedXor
                | ast::UnOp::RedNand
                | ast::UnOp::RedNor
                | ast::UnOp::RedXnor
                | ast::UnOp::LogNot
        ),
        K::Binary { op, .. } => matches!(
            op,
            ast::BinOp::Lt
                | ast::BinOp::Le
                | ast::BinOp::Gt
                | ast::BinOp::Ge
                | ast::BinOp::Eq
                | ast::BinOp::Ne
                | ast::BinOp::CaseEq
                | ast::BinOp::CaseNe
                | ast::BinOp::LogAnd
                | ast::BinOp::LogOr
        ),
        _ => false,
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
            let (b, w, sg) = fold_self_bits(e, &|_, _| None)?;
            Some(resize_bits(&b, w, width, sg))
        }
    }
}

impl Elaborator<'_> {
    /// Resolve a bare NAME to its bits at its **declared** width, for the wide domain.
    ///
    /// ⭐ This is the piece whose absence made the operator you wrote decide whether a
    /// parameter existed: `A ^ 128'h1` folded and `A ^ B` did not, for the same value,
    /// because the name resolver consulted `wide_param_bits` ONLY — a table that holds
    /// nothing under 65 bits. Every narrow name therefore declined, and a wide operand
    /// beside it had nothing to combine with.
    ///
    /// ⚠️⚠️ The width comes from `param_range`, and that source is the whole soundness
    /// argument. §4.5.373 built the reduction operators on `param_meta`, where widths
    /// INFERRED from a folded value are recorded next to declared ones, and measured
    /// `localparam W = 4'hF | 4'h0;` reducing 32 bits where both oracles hold 4 —
    /// picking the opposite generate branch at exit 0. `param_range` is the map that
    /// answers "is this width a DECLARED fact?" (`param_decl_range` records only a
    /// declared range or a declared type/literal), and the value stored beside it is
    /// coerced to that width at binding, so the pair is canonical. Measured on the
    /// exact counterexample: with the range declared, `localparam logic [3:0] W = A<<4;`
    /// is 0 in vita and 0 in iverilog.
    ///
    /// DECLINES a non-zero LSB or an ASCENDING declaration: the bit domain indexes
    /// positionally from 0 and carries no direction, so reading `[7:4]`'s or `[0:31]`'s
    /// bits through it would read the range backwards (§4.5.363's rule, applied at this
    /// domain's own boundary).
    pub(crate) fn narrow_param_bits(&self, path: &ast::HierPath) -> Option<WideBits> {
        let [seg] = path.segments.as_slice() else {
            return None;
        };
        let n = &seg.name;
        // A frame-local (inline-function formal / local, or a task output formal)
        // SHADOWS the param — the VALUE resolves to it first, so the bits must not come
        // from the param. Same guard, same order, as `param_sel_range`.
        if self.subst_lookup(n).is_some() || self.out_subst_lookup(n).is_some() {
            return None;
        }
        // Re-derive the innermost binding key over the SAME combined set the value
        // lookup walks, so a shadowing net wins here exactly as it does there.
        let key = self.walk_scopes_key(n, |k| {
            self.params.contains_key(k) || self.symbols.contains_key(k)
        })?;
        let (lo, w, ascending) = self.param_range.get(&key).copied()?;
        if lo != 0 || ascending {
            return None;
        }
        let v = self.params.get(&key).copied()?;
        // The SIGN travels with the width and must be the SAME declaration's. Requiring
        // `param_meta`'s width to agree is what proves they are: a disagreement means
        // one of the two was inferred, and this resolver refuses to mix them.
        let (mw, signed) = self.param_meta.get(&key).copied()?;
        if mw != w {
            return None;
        }
        let cv = ir::ConstVal {
            width: 64,
            signed,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val: vec![v as u64],
                unk: vec![0],
            },
        };
        Some((resize_bits(&cv.bits, 64, w, signed), w, signed))
    }

    /// A NARROW package constant's bits at its DECLARED width - the package twin of
    /// [`Self::narrow_param_bits`], declining for the same three reasons: an
    /// unrecorded (value-inferred) width, a non-zero declared LSB, and an ascending
    /// declaration. This bit domain indexes positionally from 0 and carries no
    /// direction, so reading `[7:4]`'s or `[0:31]`'s bits through it would read the
    /// declared range backwards.
    fn pkg_const_narrow_bits(&self, pkg: &str, name: &str) -> Option<WideBits> {
        let (lo, w, ascending) = self.pkg_const_range.get(pkg)?.get(name).copied()?;
        if lo != 0 || ascending {
            return None;
        }
        let v = self.pkg_consts.get(pkg)?.get(name).copied()?;
        // The SIGN must come from the SAME declaration as the width; a disagreement
        // means one of the two was inferred. Same proof as the module twin.
        let (mw, signed) = self.pkg_const_meta.get(pkg)?.get(name).copied()?;
        if mw != w {
            return None;
        }
        let cv = ir::ConstVal {
            width: 64,
            signed,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val: vec![v as u64],
                unk: vec![0],
            },
        };
        Some((resize_bits(&cv.bits, 64, w, signed), w, signed))
    }

    /// The wide domain's NAME resolver: an already-wide parameter first, then a narrow
    /// one at its declared width.
    pub(crate) fn wide_name_bits(&self, e: &ast::Expr) -> Option<WideBits> {
        // `pkg::K` — answered from the package's own wide side map and NOWHERE else.
        // A NARROW package constant deliberately declines: its declared width lives in
        // `pkg_const_meta`, which (unlike `param_range`) is not provenance-filtered, so
        // reading bits out of it would be the `param_meta` mistake in a second scope.
        if let ast::ExprKind::PkgScoped { pkg, name } = &e.kind {
            if let Some(cv) = self
                .pkg_wide_bits
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
            {
                return Some((cv.bits.clone(), cv.width, cv.signed));
            }
            // ...and a NARROW package constant, on the footing a narrow MODULE
            // parameter got one slice ago. That case declined here with the note
            // "its declared width lives in `pkg_const_meta`, which (unlike
            // `param_range`) is not provenance-filtered" - true when it was written,
            // and no longer: `pkg_const_range` is the provenance-filtered twin,
            // filled by the same `param_decl_range_opt`.
            //
            // It is what closes the last asymmetry between the three spellings of one
            // select: a NESTED select `pk::W[15:0][7:0]` declared ONE BIT while the
            // bare-imported spelling of the same text declared 52, because only this
            // domain folds a select of a select and only a bare name could reach it.
            return self.pkg_const_narrow_bits(&pkg.name, &name.name);
        }
        let ast::ExprKind::Ident(path) = &e.kind else {
            return None;
        };
        if let [seg] = path.segments.as_slice() {
            if let Some(key) = self.walk_scopes_key(&seg.name, |k| {
                self.wide_param_bits.contains_key(k)
                    || self.params.contains_key(k)
                    || self.symbols.contains_key(k)
            }) {
                if let Some(cv) = self.wide_param_bits.get(&key) {
                    return Some((cv.bits.clone(), cv.width, cv.signed));
                }
            }
        }
        self.narrow_param_bits(path)
    }

    /// Fold a parameter INITIALIZER in the wide bit domain at its DECLARED width.
    ///
    /// The declaration is a context boundary (§6.20.2 / §10.7): the value is folded at
    /// the expression's own width and then extended or truncated to the declared one.
    /// That last step is only equivalent to evaluating at the context width when the
    /// top node is SELF-DETERMINED, or when the fold already covers the declaration —
    /// see [`wide_top_is_self_determined`] for the shift that proves it.
    ///
    /// Reached only AFTER the integer domain declines, so nothing that folds today
    /// changes route.
    ///
    /// ⭐ The declared width is now handed to the fold as a CONTEXT rather than applied
    /// after it, which is what §11.6.1 asks for and is why the decline below is only a
    /// backstop: a context-determined top computes at `width` in the first place.
    /// `localparam logic [127:0] C = ~32'd0;` was `00…00ffffffffffffffff` at exit 0
    /// against both oracles' 128 ones — 48 of 108 census cells were that shape, and the
    /// `**` examples the row was filed from are its rare LOUD corner.
    pub(crate) fn param_bits_at_declared(
        &self,
        e: &ast::Expr,
        width: u32,
        signed: bool,
    ) -> Option<ir::ConstVal> {
        let (b, w, sg) = fold_bits_at(e, width, &|n, _| self.wide_name_bits(n))?;
        // ⚠️ Kept as a backstop, not as the mechanism. With the context threaded, a
        // context-determined top already returns `w >= width`; what can still land here
        // is a node kind the threading does not reach, and for those the pre-slice
        // refusal is still the right answer.
        if w < width && !wide_top_is_self_determined(e) {
            return None;
        }
        Some(ir::ConstVal {
            width,
            signed,
            repr: ir::ConstRepr::Numeric,
            bits: resize_bits(&b, w, width, sg),
        })
    }

    /// [`Self::param_bits_at_declared`] for a declaration that FITS the i64 domain —
    /// the answer goes back as an ordinary parameter value.
    ///
    /// This is what closes the reduction and select rows: `localparam logic P = ^A;`
    /// and `localparam logic [63:0] X = A[127:64];` are 1- and 64-bit declarations, so
    /// the wide side map never held them and the i64 walk has no arm for either.
    pub(crate) fn param_i64_at_declared(
        &self,
        e: &ast::Expr,
        meta: Option<(u32, bool)>,
    ) -> Option<i64> {
        let (width, signed) = meta?;
        if width > 64 {
            return None;
        }
        let cv = self.param_bits_at_declared(e, width, signed)?;
        if bp_any_unknown(&cv.bits, width) {
            return None;
        }
        let raw = cv.bits.val.first().copied().unwrap_or(0);
        Some(coerce_i64_to_width(raw as i64, width, signed))
    }
}

impl Elaborator<'_> {
    /// The wide fold of a parameter initializer, but ONLY when it disagrees with the
    /// i64 answer the caller already has.
    ///
    /// ⭐ A declaration wider than 64 bits has TWO folds available and they are not
    /// interchangeable. The i64 walk is width-UNLIMITED, so `localparam logic [127:0]
    /// R = 128'h1 - 128'h3;` folds −2, and materializing that in an UNSIGNED 128-bit
    /// parameter zero-extends it: vita printed `0000000000000000fffffffffffffffe`
    /// where both oracles print `fffffffffffffffffffffffffffffffe`. The wide domain
    /// computes the same subtraction inside 128 bits and gets it right.
    ///
    /// ⚠️ But preferring the wide fold outright is a measured regression, and the
    /// field doc says so: a wide DECLARATION whose value fits (`localparam logic
    /// [255:0] K = 256'h1;`) must keep its integer identity, or it stops being usable
    /// as a width, a bound or a generate condition. So the test is neither "how wide
    /// is the declaration" nor "how big is the value" — it is whether the two domains
    /// AGREE. When they do, nothing changes; when they do not, the i64 answer is the
    /// one that lost bits, and the wide value is installed instead.
    pub(crate) fn wide_disagreeing_value(
        &self,
        e: &ast::Expr,
        meta: Option<(u32, bool)>,
        i64_val: Option<i64>,
    ) -> Option<ir::ConstVal> {
        let (width, signed) = meta?;
        if width <= 64 {
            return None;
        }
        let cv = self.wide_param_const_in_scope(e, width, signed)?;
        let Some(v) = i64_val else {
            return Some(cv); // nothing to disagree with — the ordinary wide path
        };
        let from_i64 = resize_bits(
            &ir::BitPacked {
                val: vec![v as u64],
                unk: vec![0],
            },
            64,
            width,
            signed,
        );
        (from_i64.val != cv.bits.val || from_i64.unk != cv.bits.unk).then_some(cv)
    }
}

impl Elaborator<'_> {
    /// A SELF-DETERMINED expression folded through the wide bit domain and read back as
    /// an i64 — the bridge that lets the integer constant domain answer a shape only
    /// the bit domain can compute.
    ///
    /// ⚠️ Gated on [`wide_top_is_self_determined`] for the reason that gate exists: a
    /// context-determined top would have been evaluated at the CONSUMER's width, and
    /// this entry point has no consumer to ask. A select, a concatenation, a reduction
    /// and a comparison all decide their own width, so their i64 reading is the same
    /// number any consumer would see.
    /// The UNSIGNED reading of `e` folded in the wide bit domain at its OWN width,
    /// for a caller whose POSITION the LRM declares self-determined.
    ///
    /// ⚠️ No `wide_top_is_self_determined` gate, and that is a statement about the
    /// CALLER, not a relaxation: §20.8.1 makes a `$clog2` argument self-determined and
    /// unsigned outright, so there is no surrounding context whose width the operands
    /// should have taken instead. Only call it from a position the LRM says that about.
    pub(crate) fn selfdet_bits_unsigned(&self, e: &ast::Expr) -> Option<u64> {
        let (b, w, _) = fold_self_bits(e, &|n, _| self.wide_name_bits(n))?;
        if bp_any_unknown(&b, w) {
            return None;
        }
        if w > 64 && (64..w as usize).any(|i| bp_get(&b, i).0) {
            return None; // more magnitude than a u64 reading can carry
        }
        let mut v: u64 = 0;
        for i in 0..(w as usize).min(64) {
            if bp_get(&b, i).0 {
                v |= 1u64 << i;
            }
        }
        Some(v)
    }

    /// Does the WIDE bit domain have bits for `e`? A diagnostic-only question.
    ///
    /// ⭐ It exists because "this name is wider than 64 bits" stopped being a REASON
    /// the moment the wide domain learned to read narrow and wide names alike. The
    /// unfoldable-reason walk used the name's width as a proxy for the failure and
    /// so blamed `A` for `A / 0` — a name it can read perfectly, in an expression
    /// that fails for an entirely different reason (§4.5.384's shape: a proxy goes
    /// stale silently when the thing it stood for is routed elsewhere).
    ///
    /// ⚠️ An x/z bit does NOT count as folding. The placement arms carry unknowns
    /// through (a concat moves bits without reading them) while every value-reading
    /// arm declines on one, so a child that folds to `128'hx` is a perfectly good
    /// culprit — and saying so ("`128'hx` has no constant-fold arm") is sharper than
    /// naming the operator above it.
    pub(crate) fn wide_domain_folds(&self, e: &ast::Expr) -> bool {
        matches!(
            fold_self_bits(e, &|n, _| self.wide_name_bits(n)),
            Some((b, w, _)) if !bp_any_unknown(&b, w)
        )
    }

    /// Is `e` a KNOWN zero in the wide bit domain? Diagnostic-only, and false
    /// whenever the answer is not certain (it declines, or it carries an x bit).
    pub(crate) fn wide_domain_is_zero(&self, e: &ast::Expr) -> bool {
        match fold_self_bits(e, &|n, _| self.wide_name_bits(n)) {
            Some((b, w, _)) => !bp_any_unknown(&b, w) && !(0..w as usize).any(|i| bp_get(&b, i).0),
            None => false,
        }
    }

    /// The SELF width of `e` as the wide bit domain computes it — `$bits`'s answer
    /// for an operand the integral domain's two width sources both decline.
    ///
    /// ⚠️ `bits_of_selfdet` deliberately excludes `const_self_width` because that map
    /// mixes DECLARED widths with ones INFERRED from a folded value (§4.5.373 measured
    /// an inferred width picking the opposite generate branch at exit 0). This walk is
    /// not that map: a NAME reaches it only through `param_range` / `pkg_const_range`,
    /// which record a declared range or a declared type and nothing else — so the
    /// width it returns has the provenance the exclusion was protecting.
    pub(crate) fn wide_selfdet_width(&self, e: &ast::Expr) -> Option<u32> {
        fold_self_bits(e, &|n, _| self.wide_name_bits(n)).map(|(_, w, _)| w)
    }

    /// `$clog2(e)` computed entirely in the wide bit domain — the finished ceiling,
    /// not the argument.
    ///
    /// ⚠️ This exists because the ceiling is REPRESENTABLE where its argument is not.
    /// [`Self::selfdet_bits_unsigned`] must decline a value whose magnitude passes
    /// 64 bits (a u64 cannot carry it), and the integral `$clog2` route has nothing
    /// after that — so `localparam int AW = $clog2(A);` over a 128-bit `A` was
    /// E3009 while the runtime lane answered 128. The answer is a bit INDEX, and
    /// the bit domain has those.
    ///
    /// No `wide_top_is_self_determined` gate, for the same reason the sibling
    /// documents: §20.8.1 makes a `$clog2` argument self-determined outright.
    pub(crate) fn selfdet_clog2_wide(&self, e: &ast::Expr) -> Option<i64> {
        let (b, w, _) = fold_self_bits(e, &|n, _| self.wide_name_bits(n))?;
        // A ceiling is at most `MAX_NET_WIDTH` (2²⁰), so the i64 is never near an edge.
        wide_clog2(&b, w).map(|n| n as i64)
    }

    pub(crate) fn selfdet_bits_i64(&self, e: &ast::Expr) -> Option<i64> {
        if !wide_top_is_self_determined(e) {
            return None;
        }
        let (b, w, sg) = fold_self_bits(e, &|n, _| self.wide_name_bits(n))?;
        if w > 64 || bp_any_unknown(&b, w) {
            return None;
        }
        let raw = b.val.first().copied().unwrap_or(0);
        // ⚠️ Exactly 64 UNSIGNED bits with the top one SET has no i64 reading — the
        // same edge `const_placement_env` documents. Without this the bound
        // `[L64[63:0]:0]` over `64'hFFFF_FFFF_FFFF_FF34` wrapped to a negative and
        // sized a 205-bit net, where the pinned answer (and iverilog's own
        // "verinum::as_long() truncated 64 bits to 63") is to decline.
        if w == 64 && !sg && raw >> 63 == 1 {
            return None;
        }
        Some(coerce_i64_to_width(raw as i64, w, sg))
    }
}

impl Elaborator<'_> {
    /// An override EXPRESSION folded in the wide bit domain, at its own width.
    ///
    /// §6.20.2 gives an untyped parameter the range of its FINAL override value, so
    /// the width is part of the override, not a property the child can re-derive. See
    /// `ResolvedOverride::bits`.
    ///
    /// Only a SELF-DETERMINED top folds: an override expression is evaluated in the
    /// parent with no context from the child (the child's declared width is not known
    /// yet, and for an untyped child there is none), so a context-determined top has
    /// no width to be given and the i64 channel remains the answer for it.
    pub(crate) fn override_bits(&self, e: &ast::Expr) -> Option<ir::ConstVal> {
        if !wide_top_is_self_determined(e) {
            return None;
        }
        let (b, w, sg) = fold_self_bits(e, &|n, _| self.wide_name_bits(n))?;
        Some(ir::ConstVal {
            width: w,
            signed: sg,
            repr: ir::ConstRepr::Numeric,
            bits: b,
        })
    }
}
