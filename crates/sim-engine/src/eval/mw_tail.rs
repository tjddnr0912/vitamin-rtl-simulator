//! split part of `eval` (mechanical move).

use super::*;

/// Unsigned divmod; `b != 0` (caller-gated). One-word divisors take the O(n)
/// short path; otherwise classic restoring long division over the dividend
/// bits (O(bits·n) word ops).
pub(crate) fn mw_divmod(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let n = a.len();
    if b.iter().skip(1).all(|&x| x == 0) {
        let d = b[0] as u128;
        let mut q = vec![0u64; n];
        let mut rem = 0u128;
        for k in (0..n).rev() {
            let cur = (rem << 64) | a[k] as u128;
            q[k] = (cur / d) as u64;
            rem = cur % d;
        }
        let mut r = vec![0u64; n];
        r[0] = rem as u64;
        return (q, r);
    }
    // rem gets one extra word so the shift-in never clips.
    let mut rem = vec![0u64; n + 1];
    let mut bx = b.to_vec();
    bx.push(0);
    // `bx` is loop-invariant, so its two's complement is too — negate once and
    // subtract in place (was `mw_neg(&bx)` + a fresh `mw_add` Vec every bit).
    let neg_bx = mw_neg(&bx);
    let mut q = vec![0u64; n];
    for i in (0..n as u32 * 64).rev() {
        // rem = (rem << 1) | bit i of a
        let mut carry = (a[(i / 64) as usize] >> (i % 64)) & 1;
        for word in rem.iter_mut() {
            let top = *word >> 63;
            *word = (*word << 1) | carry;
            carry = top;
        }
        if mw_cmp(&rem, &bx) != std::cmp::Ordering::Less {
            mw_add_inplace(&mut rem, &neg_bx);
            q[(i / 64) as usize] |= 1 << (i % 64);
        }
    }
    rem.truncate(n);
    (q, rem)
}

/// Exact decimal rendering of an arbitrary-width unsigned word vector:
/// repeated short division by 10^19 (the largest power of ten in a u64),
/// emitting 19-digit chunks. Phase-1.x ⑥ — `%d` used to truncate past 128.
pub(crate) fn mw_decimal(words: &[u64]) -> String {
    const D: u128 = 10_000_000_000_000_000_000; // 10^19
    if mw_is_zero(words) {
        return "0".to_string();
    }
    let mut w = words.to_vec();
    let mut chunks: Vec<u64> = Vec::new();
    while !mw_is_zero(&w) {
        let mut rem: u128 = 0;
        for k in (0..w.len()).rev() {
            let cur = (rem << 64) | w[k] as u128;
            w[k] = (cur / D) as u64;
            rem = cur % D;
        }
        chunks.push(rem as u64);
    }
    let mut out = chunks.pop().unwrap().to_string();
    for c in chunks.into_iter().rev() {
        out.push_str(&format!("{c:019}"));
    }
    out
}

/// Square-multiply power mod 2^w (exponent ≥ 0, X-free).
pub(crate) fn mw_pow(base: &[u64], exp: &[u64], w: u32) -> Vec<u64> {
    let n = base.len();
    let top = match (0..n * 64)
        .rev()
        .find(|&i| (exp[i / 64] >> (i % 64)) & 1 == 1)
    {
        None => return mw_one(n), // exp == 0
        Some(t) => t,
    };
    let mut acc = mw_one(n);
    let mut sq = base.to_vec();
    for i in 0..=top {
        if (exp[i / 64] >> (i % 64)) & 1 == 1 {
            acc = mw_mask(mw_mul(&acc, &sq, n), w);
        }
        if i < top {
            sq = mw_mask(mw_mul(&sq, &sq, n), w);
        }
    }
    acc
}
