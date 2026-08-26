//! Multi-word (`&[u64]` limb) integer kernels — ONE spelling, shared by the
//! runtime evaluator and the elaborate-time constant domain.
//!
//! These are pure functions over little-endian `u64` word vectors: no engine
//! state, no `Value`, no `SimIr`. They lived in `sim-engine::eval` until the
//! elaborate-time wide constant domain needed `/`, `%` and `**`, at which point
//! the choice was to copy them or to move them somewhere both crates can reach.
//!
//! ⚠️⚠️ Copying was not an option, and `const_wide.rs` had already written down
//! why: implementing division there "would be a second spelling of the engine's
//! arithmetic, and a subtly wrong one produces a silent wrong PARAMETER", which
//! is P0-5. So they moved DOWN to `sim-ir`, which both `elaborate` and
//! `sim-engine` already depend on. Nothing here is serialized and nothing here
//! is a type, so the `module_path!()`-keyed `SchemaHash` freeze does not apply
//! (verified: no `SchemaHash` derive in this file, and the golden root hash is
//! computed over the frozen TYPES in `lib.rs`).
//!
//! Every function is width-agnostic: the caller decides the word count and
//! masks the result to the target width with [`mw_mask`].

/// Word count for `width` bits.
#[inline]
fn nwords(width: u32) -> usize {
    ((width as usize) + 63) / 64
}

/// Low mask over `width` bits in a single u64 (width ≤ 64 usage).
#[inline]
fn low_mask(width: u32) -> u64 {
    if width >= 64 {
        u64::MAX
    } else if width == 0 {
        0
    } else {
        (1u64 << width) - 1
    }
}

pub fn mw_add(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut carry = 0u64;
    for k in 0..a.len() {
        let (s1, c1) = a[k].overflowing_add(b.get(k).copied().unwrap_or(0));
        let (s2, c2) = s1.overflowing_add(carry);
        out[k] = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
    out
}

/// In-place `dest += b` on the word grid; bit-identical to
/// `dest = mw_add(&dest, b)` but reuses `dest`'s allocation (hot in the
/// restoring-division loop, where `b` is the pre-negated divisor).
pub fn mw_add_inplace(dest: &mut [u64], b: &[u64]) {
    let mut carry = 0u64;
    for (k, d) in dest.iter_mut().enumerate() {
        let (s1, c1) = d.overflowing_add(b.get(k).copied().unwrap_or(0));
        let (s2, c2) = s1.overflowing_add(carry);
        *d = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
}

/// Two's complement on the word grid (`!a + 1`); caller masks to width.
pub fn mw_neg(a: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut carry = 1u64;
    for k in 0..a.len() {
        let (s, c) = (!a[k]).overflowing_add(carry);
        out[k] = s;
        carry = c as u64;
    }
    out
}

pub fn mw_mask(mut a: Vec<u64>, w: u32) -> Vec<u64> {
    let n = nwords(w).max(1);
    a.truncate(n);
    a.resize(n, 0);
    let top = w - 64 * (n as u32 - 1);
    a[n - 1] &= low_mask(top);
    a
}

pub fn mw_is_zero(a: &[u64]) -> bool {
    a.iter().all(|&x| x == 0)
}

pub fn mw_one(n: usize) -> Vec<u64> {
    let mut v = vec![0u64; n];
    v[0] = 1;
    v
}

/// School multiplication, LOW `n` words (mod 2^(64n)).
pub fn mw_mul(a: &[u64], b: &[u64], n: usize) -> Vec<u64> {
    let mut out = vec![0u64; n];
    for i in 0..n.min(a.len()) {
        if a[i] == 0 {
            continue;
        }
        let mut carry = 0u128;
        for j in 0..n - i {
            let bj = b.get(j).copied().unwrap_or(0);
            let cur = (a[i] as u128) * (bj as u128) + (out[i + j] as u128) + carry;
            out[i + j] = cur as u64;
            carry = cur >> 64;
        }
    }
    out
}

pub fn mw_cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    for k in (0..a.len().max(b.len())).rev() {
        let av = a.get(k).copied().unwrap_or(0);
        let bv = b.get(k).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            o => return o,
        }
    }
    std::cmp::Ordering::Equal
}

/// Unsigned divmod; `b != 0` (caller-gated). One-word divisors take the O(n)
/// short path; otherwise classic restoring long division over the dividend
/// bits (O(bits·n) word ops).
pub fn mw_divmod(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
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
pub fn mw_decimal(words: &[u64]) -> String {
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

/// Index of the highest set bit of `v`, or `None` when `v` is zero.
///
/// Extracted from [`mw_pow`]'s exponent scan because the elaborate-time caller
/// needs the SAME number before it runs the loop: it is the iteration count,
/// and therefore the work estimate a constant fold budgets against.
pub fn mw_top_set_bit(v: &[u64]) -> Option<usize> {
    (0..v.len() * 64)
        .rev()
        .find(|&i| (v[i / 64] >> (i % 64)) & 1 == 1)
}

/// Square-multiply power mod 2^w (exponent ≥ 0, X-free).
pub fn mw_pow(base: &[u64], exp: &[u64], w: u32) -> Vec<u64> {
    let n = base.len();
    let top = match mw_top_set_bit(exp) {
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
