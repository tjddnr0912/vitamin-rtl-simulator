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
            let n = cap(u64::from(fold_count(w, name)?))?;
            let (b, bw, sg) = fold_self_bits(expr, name)?;
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
        // directions. `<<<` is not admitted (it is `<<` with a signed result, and no
        // caller folds it here); `>>>` has its OWN arm below — the sentence that used
        // to say both were "deliberately NOT admitted" was written before it.
        ast::ExprKind::Binary { op, lhs, rhs }
            if matches!(op, ast::BinOp::Shl | ast::BinOp::Shr) =>
        {
            let k = fold_shift_count(rhs, name)?;
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
            let (b, w, sg) = fold_self_bits(lhs, name)?;
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
            let (b, w, _) = fold_self_bits(operand, name)?;
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
            let (b, w, _) = fold_self_bits(operand, name)?;
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
            let l = fold_self_bits(lhs, name)?;
            let r = fold_self_bits(rhs, name)?;
            let (a, b, w, sg) = bp_operands(&l, &r)?;
            let v = match op {
                ast::BinOp::Add => limbs_add(&a, &b, w),
                ast::BinOp::Sub => limbs_add(&a, &limbs_neg(&b, w), w),
                _ => limbs_mul(&a, &b, w),
            };
            Some((bp_from_limbs(v, w), w, sg))
        }
        // §11.4.5 unary minus: the two's complement at the operand's own width.
        ast::ExprKind::Unary {
            op: ast::UnOp::Minus,
            operand,
        } => {
            let (b, w, sg) = fold_self_bits(operand, name)?;
            if bp_any_unknown(&b, w) {
                return None;
            }
            Some((bp_from_limbs(limbs_neg(&b.val, w), w), w, sg))
        }
        ast::ExprKind::Unary {
            op: ast::UnOp::Plus,
            operand,
        } => fold_self_bits(operand, name),
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
            let l = fold_self_bits(lhs, name)?;
            let r = fold_self_bits(rhs, name)?;
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
                let (b, w, _) = fold_self_bits(e, name)?;
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
            let (cb, cw, _) = fold_self_bits(cond, name)?;
            if bp_any_unknown(&cb, cw) {
                return None;
            }
            let c = (0..cw as usize).any(|i| bp_get(&cb, i).0);
            let (tb, tw, ts) = fold_self_bits(then_e, name)?;
            let (eb, ew, es) = fold_self_bits(else_e, name)?;
            let w = tw.max(ew);
            let sg = ts && es;
            let (b, from, fs) = if c { (tb, tw, ts) } else { (eb, ew, es) };
            Some((resize_bits(&b, from, w, fs), w, sg))
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
            let (b, w, _) = fold_self_bits(base, name)?;
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
            let (b, w, _) = fold_self_bits(base, name)?;
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
            let (b, w, _) = fold_self_bits(base, name)?;
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
            let arg = || fold_self_bits(&args[0], name);
            match f.name.as_str() {
                "$signed" => arg().map(|(b, w, _)| (b, w, true)),
                "$unsigned" => arg().map(|(b, w, _)| (b, w, false)),
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
                        _ => {
                            let mut b = bp_zero(32);
                            for i in 0..32 {
                                bp_set(&mut b, i, (n >> i) & 1 == 1, false);
                            }
                            Some((b, 32, true))
                        }
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
    pub(crate) fn param_bits_at_declared(
        &self,
        e: &ast::Expr,
        width: u32,
        signed: bool,
    ) -> Option<ir::ConstVal> {
        let (b, w, sg) = fold_self_bits(e, &|n, _| self.wide_name_bits(n))?;
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
