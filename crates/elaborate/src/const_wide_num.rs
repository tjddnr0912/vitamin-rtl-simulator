//! Fixed-width limb arithmetic for the wide constant domain — see the banner below.

use super::*;

// ── VALUE-READING wide arms: arithmetic, comparison, select, reduction ───────
//
// The section above admitted only CARRY-FREE operators, and said so as a rule:
// "`+`, `-`, `*`, `/` and the comparisons need a carry chain across 128+ bits;
// implementing one here would be a second spelling of the engine's arithmetic".
//
// That reasoning was about a RISK, not an impossibility, and the risk is payable:
// the operations below are implemented once, on the same `BitPacked` limbs the
// engine uses, with the operand-extension rule (§11.6.1) written out at the single
// place both operands are brought to a common width. Every one of them DECLINES on
// an unknown bit rather than inventing a 2-state answer, and division/modulus stay
// out (a wide divide is a different algorithm, and `x/0` has no value here).
//
// What this buys is the crypto-constant idiom the wide domain was built for:
// `localparam [127:0] R = A + W;` and `localparam P = (A == W);` were `E3009` while
// `A ^ W` folded, so which operator you wrote decided whether the parameter existed.

/// Limb count for `w` bits.
pub(crate) fn nlimb(w: u32) -> usize {
    ((w as usize).div_ceil(64)).max(1)
}

/// Clear every bit at or above `w` in `p` (the limbs are 64-bit; the top one may
/// hold slack). Keeps a value CANONICAL at its width, which is what makes the
/// comparisons below a plain limb-wise scan.
pub(crate) fn bp_mask_to(p: &mut [u64], w: u32) {
    let n = nlimb(w);
    let rem = w as usize % 64;
    if rem != 0 && n <= p.len() {
        p[n - 1] &= (1u64 << rem) - 1;
    }
    for limb in p.iter_mut().skip(n) {
        *limb = 0;
    }
}

/// The two operands of a binary operator brought to their COMMON width (§11.6.1:
/// the max of the two self widths, each extended in its OWN signedness), with the
/// result's signedness (§11.8.1: signed only if both are).
///
/// Declines when either side carries an unknown bit — every caller here READS bit
/// values, and a 4-state carry chain belongs in the engine, not in a second copy.
pub(crate) fn bp_operands(l: &WideBits, r: &WideBits) -> Option<(Vec<u64>, Vec<u64>, u32, bool)> {
    let (lb, lw, ls) = l;
    let (rb, rw, rs) = r;
    if bp_any_unknown(lb, *lw) || bp_any_unknown(rb, *rw) {
        return None;
    }
    let w = (*lw).max(*rw);
    let a = resize_bits(lb, *lw, w, *ls);
    let b = resize_bits(rb, *rw, w, *rs);
    Some((a.val, b.val, w, *ls && *rs))
}

/// Wrap a 2-state limb vector back into a `BitPacked` of width `w`.
pub(crate) fn bp_from_limbs(mut v: Vec<u64>, w: u32) -> ir::BitPacked {
    v.resize(nlimb(w), 0);
    bp_mask_to(&mut v, w);
    ir::BitPacked {
        unk: vec![0; v.len()],
        val: v,
    }
}

/// `a + b` on 2-state limbs, truncated to `w` bits (§11.4.3 — the result is the
/// context width and the carry out of it is DISCARDED, which is what makes
/// `8'hFF + 8'h2` equal 1).
pub(crate) fn limbs_add(a: &[u64], b: &[u64], w: u32) -> Vec<u64> {
    let n = nlimb(w);
    let mut out = vec![0u64; n];
    let mut carry = 0u64;
    for (i, slot) in out.iter_mut().enumerate() {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        let (s1, c1) = x.overflowing_add(y);
        let (s2, c2) = s1.overflowing_add(carry);
        *slot = s2;
        carry = u64::from(c1) + u64::from(c2);
    }
    bp_mask_to(&mut out, w);
    out
}

/// Two's-complement negation at `w` bits.
pub(crate) fn limbs_neg(a: &[u64], w: u32) -> Vec<u64> {
    let n = nlimb(w);
    let inv: Vec<u64> = (0..n).map(|i| !a.get(i).copied().unwrap_or(0)).collect();
    let mut one = vec![0u64; n];
    one[0] = 1;
    limbs_add(&inv, &one, w)
}

/// Schoolbook `a * b`, truncated to `w` bits. `w` is capped by `MAX_NET_WIDTH`
/// upstream, so the O(n²) limb loop is bounded by the same cap every other wide
/// operation lives under.
pub(crate) fn limbs_mul(a: &[u64], b: &[u64], w: u32) -> Vec<u64> {
    let n = nlimb(w);
    let mut out = vec![0u64; n];
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0) as u128;
        if x == 0 {
            continue;
        }
        let mut carry = 0u128;
        for j in 0..(n - i) {
            let y = b.get(j).copied().unwrap_or(0) as u128;
            let acc = x * y + out[i + j] as u128 + carry;
            out[i + j] = acc as u64;
            carry = acc >> 64;
        }
    }
    bp_mask_to(&mut out, w);
    out
}

/// `a <=> b` at `w` bits, read as SIGNED when `signed` (§11.4.4: the comparison is
/// signed only when BOTH operands are).
pub(crate) fn limbs_cmp(a: &[u64], b: &[u64], w: u32, signed: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let n = nlimb(w);
    if signed {
        let top = (w as usize - 1) % 64;
        let sa = (a.get(n - 1).copied().unwrap_or(0) >> top) & 1;
        let sb = (b.get(n - 1).copied().unwrap_or(0) >> top) & 1;
        if sa != sb {
            // The negative one is the smaller.
            return if sa == 1 {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
    }
    for i in (0..n).rev() {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x.cmp(&y);
        }
    }
    Ordering::Equal
}

/// A 32-bit SIGNED `WideBits` — the shape §20.6.2 / §20.8.1 / §20.9 give the
/// system functions that return an `int`/`integer`: `$bits`, `$clog2`,
/// `$countones`. One spelling so the three cannot drift on the sign.
pub(crate) fn int32(v: u64) -> WideBits {
    let mut b = bp_zero(32);
    for i in 0..32 {
        bp_set(&mut b, i, (v >> i) & 1 == 1, false);
    }
    (b, 32, true)
}

/// A 1-bit unsigned `WideBits` — every self-determined predicate's result shape.
pub(crate) fn bp_bit(v: bool) -> WideBits {
    let mut b = bp_zero(1);
    bp_set(&mut b, 0, v, false);
    (b, 1, false)
}

// ── the SUPER-LINEAR wide arms: `/`, `%`, `**`, `$clog2` ─────────────────────
//
// The banner above says the carry-chain risk "is payable ... implemented once,
// on the same `BitPacked` limbs the engine uses", and then kept division out
// because "a wide divide is a different algorithm". It is — so this file does
// not write one. `sim_ir::mw::mw_divmod` and `mw_pow` are the kernels
// `EvalCtx::arith` calls, moved down into `sim-ir` so BOTH domains call the one
// function. There is no second spelling here to be subtly wrong.

/// `v` resized to exactly `nlimb(w)` limbs and masked canonical at `w`.
///
/// Every kernel below indexes limbs positionally, so an operand that arrived
/// with a different limb count (a narrower fold, a `mw_*` return) has to be
/// normalised before a bit index means what it says.
fn limbs_at(v: &[u64], w: u32) -> Vec<u64> {
    let mut out = v.to_vec();
    out.resize(nlimb(w), 0);
    bp_mask_to(&mut out, w);
    out
}

/// Bit `i` of a limb vector.
fn limb_bit(v: &[u64], i: u32) -> bool {
    v.get(i as usize / 64)
        .is_some_and(|l| (l >> (i % 64)) & 1 == 1)
}

/// Word-op budget for the kernels whose cost grows FASTER than the operand
/// width, evaluated at ELABORATE time where there is no `$finish` to interrupt.
///
/// ⚠️ The other wide arms are O(n) or O(n²) at `n = w/64` limbs, and the
/// declarable width cap (`MAX_NET_WIDTH` = 2²⁰ bits = 16384 limbs) keeps even
/// the multiply near a second. Restoring division is O(w·n) — at 2²⁰ bits that
/// is 2³⁴ word operations, tens of seconds, for a `localparam` nobody writes.
/// The runtime lane answers that shape by POISONING to X above
/// `WIDE_ARITH_CAP`; a constant fold has the better option of DECLINING, which
/// leaves the caller loud (E3009) instead of silently X. 2²⁶ word ops admits a
/// 65536-bit `/` and `%` pair — measured at **52 ms** end to end in a release
/// build, and four times the widest constant in the workload corpus — while a
/// 131072-bit one stays loud in 6 ms instead of running for minutes.
const WIDE_CONST_WORK_CAP: u64 = 1 << 26;

/// §11.4.3 `/` and `%` at the operands' common width.
///
/// The sign handling is `EvalCtx::arith`'s, line for line: take each operand to
/// its MAGNITUDE, divide unsigned, then negate the quotient iff the signs differ
/// and the remainder iff the DIVIDEND was negative. Measured against both
/// oracles: `-128'sd17 / 128'sd5` = −3 and `-128'sd17 % 128'sd5` = −2, while
/// `-128'sd17 / 128'd5` (one unsigned operand ⇒ the whole operation unsigned,
/// §11.4.3) is `3333…332f`.
pub(crate) fn wide_divmod(is_div: bool, l: &WideBits, r: &WideBits) -> Option<WideBits> {
    let (a, b, w, sg) = bp_operands(l, r)?;
    // ⚠️ `x / 0` has NO constant value: IEEE §11.4.3 makes it X, iverilog and
    // vita's own runtime both produce X, and verilator's 0 is a 2-state
    // artifact. This domain holds 2-state limbs, so it declines and the caller
    // stays loud rather than installing either answer in a parameter.
    if b.iter().all(|&x| x == 0) {
        return None;
    }
    if u64::from(w) * nlimb(w) as u64 > WIDE_CONST_WORK_CAP {
        return None;
    }
    let (sa, sb) = (sg && limb_bit(&a, w - 1), sg && limb_bit(&b, w - 1));
    let ma = if sa {
        limbs_neg(&a, w)
    } else {
        limbs_at(&a, w)
    };
    let mb = if sb {
        limbs_neg(&b, w)
    } else {
        limbs_at(&b, w)
    };
    let (q, rem) = ir::mw::mw_divmod(&ma, &mb);
    let v = if is_div {
        if sa != sb {
            limbs_neg(&q, w)
        } else {
            q
        }
    } else if sa {
        limbs_neg(&rem, w)
    } else {
        rem
    };
    Some((bp_from_limbs(v, w), w, sg))
}

/// §11.4.10 `**`.
///
/// ⚠️⚠️ [`bp_operands`] is the WRONG prep for this operator, which is the whole
/// reason it has its own function. Table 11-21 makes the EXPONENT
/// self-determined while the base is context-determined, so bringing both to a
/// common width would size `8'd200 ** 2` at 32 bits — both oracles answer `40`
/// there, the 8-bit truncation of 40000. The BASE's width is the result's
/// width, and §11.4.10 makes the result signed iff the base is.
///
/// The negative-exponent rows are IEEE Table 11-6, mirrored from the engine:
/// 1 → 1; −1 → ±1 by exponent parity; 0 → X (declines here); |base| > 1 → 0.
pub(crate) fn wide_pow(l: &WideBits, r: &WideBits) -> Option<WideBits> {
    let (bb, bw, bs) = l;
    let (eb, ew, es) = r;
    if bp_any_unknown(bb, *bw) || bp_any_unknown(eb, *ew) {
        return None;
    }
    let (w, sg) = (*bw, *bs);
    let n = nlimb(w);
    let a = limbs_at(&bb.val, w);
    let e = limbs_at(&eb.val, *ew);
    if *es && limb_bit(&e, *ew - 1) {
        let one = limbs_at(&[1], w);
        let minus_one = limbs_neg(&one, w);
        // The parity of a two's-complement negative equals the parity of its
        // magnitude, so the raw low bit answers the ±1 row without negating.
        let odd = e.first().copied().unwrap_or(0) & 1 == 1;
        let v = if a == one {
            one
        } else if sg && a == minus_one {
            if odd {
                minus_one
            } else {
                one
            }
        } else if a.iter().all(|&x| x == 0) {
            return None; // 0 ** negative is X — no constant value here
        } else {
            vec![0u64; n]
        };
        return Some((bp_from_limbs(v, w), w, sg));
    }
    // Square-multiply runs one or two n²-limb multiplies per exponent bit up to
    // the highest set one, so the work is bounded BEFORE the loop rather than
    // discovered inside it.
    let top = ir::mw::mw_top_set_bit(&e).map_or(0u64, |t| t as u64 + 1);
    if top * 2 * (n as u64) * (n as u64) > WIDE_CONST_WORK_CAP {
        return None;
    }
    Some((bp_from_limbs(ir::mw::mw_pow(&a, &e, w), w), w, sg))
}

/// §20.8.1 `$clog2`: the smallest `n` with `2ⁿ ≥ arg`, the argument read as
/// UNSIGNED at its own width.
///
/// In the bit domain that is a bit index, which is why it answers where the
/// integral domain cannot: `selfdet_bits_unsigned` declines the moment the
/// value's MAGNITUDE passes 64 bits, and `localparam int AW = $clog2(MAX);`
/// over a 128-bit `MAX` is the standard width idiom. Measured on both oracles:
/// `$clog2(128'he1000…001)` = 128, `$clog2(2¹²⁷)` = 127 (an exact power of two
/// costs no extra bit), `$clog2(2¹²⁷+1)` = 128, `$clog2(1)` = `$clog2(0)` = 0.
pub(crate) fn wide_clog2(b: &ir::BitPacked, w: u32) -> Option<u64> {
    if bp_any_unknown(b, w) {
        return None;
    }
    // An all-zero argument has no top bit, and `$clog2(0)` is 0 in both oracles.
    let Some(top) = (0..w as usize).rev().find(|&i| bp_get(b, i).0) else {
        return Some(0);
    };
    let exact = !(0..top).any(|i| bp_get(b, i).0);
    Some(if exact { top as u64 } else { top as u64 + 1 })
}
