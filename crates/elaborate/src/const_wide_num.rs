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

/// A 1-bit unsigned `WideBits` — every self-determined predicate's result shape.
pub(crate) fn bp_bit(v: bool) -> WideBits {
    let mut b = bp_zero(1);
    bp_set(&mut b, 0, v, false);
    (b, 1, false)
}
