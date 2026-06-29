//! Runtime 4-state value + the per-bit IEEE-1364 truth-table primitives.
//!
//! Encoding (canonical, identical to `sim_ir::BitPacked` and `vcd-writer`):
//! per-bit two planes `(v,u)`: `(0,0)=0`, `(1,0)=1`, `(0,1)=X`, `(1,1)=Z`.
//! word0 bit0 = LSB. This is load-bearing — a `Value` round-trips to the VCD
//! writer for free via [`Value::into_bitpacked`].
//!
//! Z is treated as X in every operator except `===`/`!==` (and net storage);
//! the per-bit primitives below state their tables over `{0,1,X}` and a Z input
//! (`(1,1)`) folds to X because it has `u=1`.

use sim_ir::BitPacked;

#[inline]
pub(crate) fn nwords(width: u32) -> usize {
    ((width as usize) + 63) / 64
}

/// Mask of valid bits in the most-significant word (handles `width % 64 == 0`).
#[inline]
pub(crate) fn top_mask(width: u32) -> u64 {
    let r = width % 64;
    if width == 0 {
        0
    } else if r == 0 {
        u64::MAX
    } else {
        (1u64 << r) - 1
    }
}

/// [C3] Bit-plane word store for a [`Value`]: **inline** for the ≤128-bit case (two
/// `u64` words, NO heap allocation — the overwhelmingly common RTL width), spilling to a
/// `Vec` only for >128 bits. Once the bit-serial net I/O + shift paths were word-ized,
/// profiling showed `Value` heap allocation became the single dominant cost (~25-30%):
/// every eval intermediate + net read (`from_packed`) allocs two `Vec<u64>`. This rep
/// keeps the ≤128-bit case alloc-free.
///
/// Runtime-only: a `Value` is NEVER serialized or part of the golden `SimIr`/SchemaHash
/// (it bridges to the frozen `BitPacked` via [`Value::into_bitpacked`]/[`Value::from_packed`]),
/// so the representation is free. It `Deref`s to `[u64]`, so the 4-state ops index /
/// slice / iterate it exactly like the old `Vec<u64>`; mutation is via `resize` and
/// `IndexMut` (through `DerefMut`). Equality is BY VALUE (slice contents), NOT by
/// inline-vs-heap variant — `Value` derives `PartialEq`/`Eq` and is compared for
/// `$monitor` change detection, so two equal-valued planes MUST compare equal regardless
/// of representation.
#[derive(Clone)]
pub enum Words {
    /// ≤128 bits: `len ∈ {0,1,2}` valid words held inline (no allocation).
    Inline { w: [u64; 2], len: usize },
    /// >128 bits: heap-backed (the rare wide-arithmetic case).
    Heap(Vec<u64>),
}

impl Words {
    /// `n` words of zero (inline if `n ≤ 2`).
    #[inline]
    pub fn zeros(n: usize) -> Self {
        if n <= 2 {
            Words::Inline { w: [0, 0], len: n }
        } else {
            Words::Heap(vec![0; n])
        }
    }

    /// `n` words of all-ones (caller masks the top partial word). Inline if `n ≤ 2`.
    #[inline]
    pub fn ones(n: usize) -> Self {
        if n <= 2 {
            Words::Inline {
                w: [u64::MAX, u64::MAX],
                len: n,
            }
        } else {
            Words::Heap(vec![u64::MAX; n])
        }
    }

    /// A single-word inline store `[x]` (for the real lane / 1-word constants).
    #[inline]
    pub fn from_word(x: u64) -> Self {
        Words::Inline { w: [x, 0], len: 1 }
    }

    /// Copy `s` into an `n`-word store, zero-padding/truncating to `n` — alloc-free for
    /// `n ≤ 2` (the hot `from_packed` net-read path).
    #[inline]
    pub fn from_slice_padded(s: &[u64], n: usize) -> Self {
        let c = s.len().min(n);
        if n <= 2 {
            let mut w = [0u64; 2];
            w[..c].copy_from_slice(&s[..c]);
            Words::Inline { w, len: n }
        } else {
            let mut v = vec![0u64; n];
            v[..c].copy_from_slice(&s[..c]);
            Words::Heap(v)
        }
    }

    // NOTE: `len()` / `is_empty()` / `get()` / `first()` / `iter()` / indexing are all
    // provided by the `Deref`/`DerefMut` to `[u64]` below — callers use them unchanged.

    /// `Vec::resize` semantics (grow appends `fill`, shrink truncates), promoting inline
    /// → heap when growing past 2 words.
    pub fn resize(&mut self, new_len: usize, fill: u64) {
        match self {
            Words::Inline { w, len } => {
                if new_len <= 2 {
                    for slot in w.iter_mut().take(new_len).skip(*len) {
                        *slot = fill; // grow: write fill into the newly-valid words
                    }
                    *len = new_len;
                } else {
                    let mut v = Vec::with_capacity(new_len);
                    v.extend_from_slice(&w[..*len]);
                    v.resize(new_len, fill);
                    *self = Words::Heap(v);
                }
            }
            Words::Heap(v) => v.resize(new_len, fill),
        }
    }

    /// Consume into a `Vec<u64>` (the `BitPacked` bridge in `into_bitpacked`).
    #[inline]
    pub fn into_vec(self) -> Vec<u64> {
        match self {
            Words::Inline { w, len } => w[..len].to_vec(),
            Words::Heap(v) => v,
        }
    }
}

impl std::ops::Deref for Words {
    type Target = [u64];
    #[inline]
    fn deref(&self) -> &[u64] {
        match self {
            Words::Inline { w, len } => &w[..*len],
            Words::Heap(v) => v,
        }
    }
}

impl std::ops::DerefMut for Words {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u64] {
        match self {
            Words::Inline { w, len } => &mut w[..*len],
            Words::Heap(v) => v,
        }
    }
}

// Equality BY VALUE (slice contents): an inline `[1]` MUST equal a heap `[1]` so
// `Value`'s derived `PartialEq` (used for `$monitor` change detection) is
// representation-independent.
impl PartialEq for Words {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for Words {}

impl std::fmt::Debug for Words {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug as the word slice — identical to the old `Vec<u64>` rendering.
        std::fmt::Debug::fmt(&**self, f)
    }
}

/// A runtime 4-state vector. `val`/`unk` are bit-parallel planes, word0 bit0 =
/// LSB, identical encoding to `sim_ir::BitPacked`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub val: Words,
    pub unk: Words,
    pub width: u32,
    /// Result signedness (drives sign-extension, `>>>`, relational, `$signed`).
    pub signed: bool,
    /// When true, `val[0]` is `f64::to_bits(x)` and this Value is an IEEE-754
    /// real (64-bit, 2-state, `unk == [0]`). All 4-state paths keep this false.
    pub is_real: bool,
    /// v7 P2-C: a STRING-domain value (width = 8×len packed ASCII, MSB-first).
    /// Sizing bypasses context resizing (like `is_real`) — a dynamic-length
    /// string must never be truncated by a static width; the write funnel
    /// owns the §6.16 string↔packed conversion instead.
    pub is_str: bool,
}

impl Value {
    pub fn zeros(width: u32, signed: bool) -> Self {
        let n = nwords(width).max(1);
        Value {
            val: Words::zeros(n),
            unk: Words::zeros(n),
            width,
            signed,
            is_real: false,
            is_str: false,
        }
    }

    /// All-X of a given width (the canonical "poison" for arith/relational when
    /// an operand has any X/Z).
    pub fn xs(width: u32, signed: bool) -> Self {
        let n = nwords(width).max(1);
        let mut v = Value {
            val: Words::zeros(n),
            unk: Words::ones(n),
            width,
            signed,
            is_real: false,
            is_str: false,
        };
        v.mask_top();
        v
    }

    pub fn one1() -> Self {
        let mut v = Value::zeros(1, false);
        v.val[0] = 1;
        v
    }

    pub fn x1() -> Self {
        let mut v = Value::zeros(1, false);
        v.unk[0] = 1; // (v=0,u=1) = X
        v
    }

    pub fn logic(b: bool) -> Self {
        if b {
            Value::one1()
        } else {
            Value::zeros(1, false)
        }
    }

    /// Build a `Value` from a `BitPacked` of the given width (net/const read). Alloc-free
    /// for ≤128 bits — `from_slice_padded` copies the frozen `BitPacked` words straight
    /// into the inline store (this is the hot net-read path).
    pub fn from_packed(b: &BitPacked, width: u32, signed: bool) -> Self {
        let n = nwords(width).max(1);
        let mut v = Value {
            val: Words::from_slice_padded(&b.val, n),
            unk: Words::from_slice_padded(&b.unk, n),
            width,
            signed,
            is_real: false,
            is_str: false,
        };
        v.mask_top();
        v
    }

    /// Clear bits above `width` in the top word (keep planes canonical). The plane
    /// resizes are GUARDED on length: a `Value` is almost always already exactly
    /// `nwords(width)` words (every constructor / word-parallel op builds it so), and
    /// `mask_top` is on the tail of nearly every Value operation, so skipping the
    /// no-op `resize` (its match + bookkeeping) in the common case is a measured win.
    pub(crate) fn mask_top(&mut self) {
        if self.is_real {
            return; // a real is 64 IEEE bits; never bit-mask (would corrupt it).
        }
        let n = nwords(self.width).max(1);
        if self.val.len() != n {
            self.val.resize(n, 0);
        }
        if self.unk.len() != n {
            self.unk.resize(n, 0);
        }
        let m = top_mask(self.width);
        if let Some(last) = self.val.get_mut(n - 1) {
            *last &= m;
        }
        if let Some(last) = self.unk.get_mut(n - 1) {
            *last &= m;
        }
    }

    #[inline]
    pub fn get_vu(&self, i: u32) -> (u64, u64) {
        let w = (i / 64) as usize;
        let s = i % 64;
        let v = self.val.get(w).map_or(0, |x| (x >> s) & 1);
        let u = self.unk.get(w).map_or(0, |x| (x >> s) & 1);
        (v, u)
    }

    #[inline]
    pub fn set_vu(&mut self, i: u32, v: u64, u: u64) {
        let w = (i / 64) as usize;
        let s = i % 64;
        if w >= self.val.len() {
            self.val.resize(w + 1, 0);
            self.unk.resize(w + 1, 0);
        }
        self.val[w] = (self.val[w] & !(1 << s)) | ((v & 1) << s);
        self.unk[w] = (self.unk[w] & !(1 << s)) | ((u & 1) << s);
    }

    /// Any X/Z anywhere within width? Drives arith/relational/`==` poisoning.
    pub fn has_xz(&self) -> bool {
        let n = nwords(self.width);
        for w in 0..n {
            let mask = if w == n - 1 {
                top_mask(self.width)
            } else {
                u64::MAX
            };
            if self.unk.get(w).copied().unwrap_or(0) & mask != 0 {
                return true;
            }
        }
        false
    }

    /// Clean integer value IF IT FITS in u64; `None` on any X/Z (caller
    /// poisons) or when set bits exist above bit 63. Returning the truncated
    /// low word for a wider value let relational/shift/index sites silently
    /// use a wrong small number (P0-4, 2026-06-10 audit) — overflow is now
    /// `None` and every caller decides (saturate, OOR sentinel, or X).
    pub fn to_u64(&self) -> Option<u64> {
        if self.has_xz() {
            return None;
        }
        if self.val.iter().skip(1).any(|&w| w != 0) {
            return None;
        }
        Some(self.val.first().copied().unwrap_or(0) & low_mask(self.width))
    }

    /// Clean integer value IF IT FITS in u128; `None` on any X/Z or when set
    /// bits exist above bit 127 (same no-silent-truncation contract as
    /// `to_u64`). Unsigned arithmetic lane: 128 bits.
    pub fn to_u128(&self) -> Option<u128> {
        if self.has_xz() {
            return None;
        }
        if self.val.iter().skip(2).any(|&w| w != 0) {
            return None;
        }
        let lo = self.val.first().copied().unwrap_or(0) as u128;
        let hi = self.val.get(1).copied().unwrap_or(0) as u128;
        let raw = lo | (hi << 64);
        if self.width >= 128 {
            Some(raw)
        } else {
            Some(raw & ((1u128 << self.width) - 1))
        }
    }

    /// Sign-aware i128 view of the low bits (for signed arith/relational).
    pub fn to_i128_signed(&self) -> Option<i128> {
        let u = self.to_u64()? as i128;
        if self.signed && self.width >= 1 && self.width <= 64 {
            let sign = (self.val.first().copied().unwrap_or(0) >> (self.width - 1)) & 1;
            if sign == 1 {
                return Some(u - (1i128 << self.width));
            }
        }
        Some(u)
    }

    /// Pack into a `BitPacked` of `out_width`, zero/sign-/X-extending or
    /// truncating. Bridge to net writes and the VCD writer.
    pub fn into_bitpacked(self, out_width: u32) -> BitPacked {
        let r = self.resize(out_width);
        BitPacked {
            val: r.val.into_vec(),
            unk: r.unk.into_vec(),
        }
    }

    /// Resize to `new_width`: signed → sign-extend the MSB *bit value* (X MSB →
    /// X fill); unsigned → zero-extend; shrink → truncate. WORD-PARALLEL: copy the
    /// overlapping low words wholesale (the source is canonical — bits ≥ width are 0 —
    /// so no per-bit boundary clear is needed), sign-fill the high region word-parallel
    /// only when extending a signed value with a set sign bit, then mask the top word.
    pub fn resize(mut self, new_width: u32) -> Value {
        if self.is_real {
            return self; // a real is dimensionless 64-bit; width context is a no-op.
        }
        if new_width == self.width {
            self.mask_top();
            return self;
        }
        let n = nwords(new_width).max(1);
        let copy_w = nwords(self.width.min(new_width)).min(n);
        let mut val = Words::zeros(n);
        let mut unk = Words::zeros(n);
        for k in 0..copy_w {
            val[k] = self.val.get(k).copied().unwrap_or(0);
            unk[k] = self.unk.get(k).copied().unwrap_or(0);
        }
        if new_width > self.width && self.signed && self.width > 0 {
            let (fv, fu) = self.get_vu(self.width - 1);
            if fv != 0 || fu != 0 {
                fill_bits(&mut val, &mut unk, self.width, new_width, fv, fu);
            }
        }
        let mut out = Value {
            val,
            unk,
            width: new_width,
            signed: self.signed,
            is_real: false,
            is_str: false,
        };
        out.mask_top();
        out
    }

    /// WORD-PARALLEL logical/arithmetic right shift by `amt`, preserving width `w`; the
    /// vacated top `min(amt,w)` bits are filled with `(fv,fu)` (arith → MSB sign bit,
    /// logical → 0). Both planes shift identically; bit-exact with the per-bit form
    /// (locked by `shift_word_vs_bit_parity`).
    pub fn shr_fill(&self, amt: u64, w: u32, fv: u64, fu: u64) -> Value {
        let n = nwords(w).max(1);
        let mut val = Words::zeros(n);
        let mut unk = Words::zeros(n);
        if amt < w as u64 {
            let q = (amt / 64) as usize;
            let r = (amt % 64) as u32;
            for k in 0..n {
                let lo_v = self.val.get(k + q).copied().unwrap_or(0);
                let lo_u = self.unk.get(k + q).copied().unwrap_or(0);
                if r == 0 {
                    val[k] = lo_v;
                    unk[k] = lo_u;
                } else {
                    let hi_v = self.val.get(k + q + 1).copied().unwrap_or(0);
                    let hi_u = self.unk.get(k + q + 1).copied().unwrap_or(0);
                    val[k] = (lo_v >> r) | (hi_v << (64 - r));
                    unk[k] = (lo_u >> r) | (hi_u << (64 - r));
                }
            }
        }
        // Fill the vacated top bits [w-amt, w) (or all of [0,w) when amt ≥ w). Logical
        // (fv=fu=0) needs no fill — the shift already brought zeros in.
        if fv != 0 || fu != 0 {
            let lo = (w as u64).saturating_sub(amt).min(w as u64) as u32;
            fill_bits(&mut val, &mut unk, lo, w, fv, fu);
        }
        let mut out = Value {
            val,
            unk,
            width: w,
            signed: self.signed,
            is_real: false,
            is_str: false,
        };
        out.mask_top();
        out
    }

    /// WORD-PARALLEL left shift by `amt`, growing the result to width `grow_w` (the
    /// caller's lossless-growth policy); vacated low bits are 0. Bit-exact with the
    /// per-bit form (locked by `shift_word_vs_bit_parity`).
    pub fn shl_grow(&self, amt: u64, grow_w: u32) -> Value {
        let n = nwords(grow_w).max(1);
        let mut val = Words::zeros(n);
        let mut unk = Words::zeros(n);
        if amt < grow_w as u64 {
            let q = (amt / 64) as usize;
            let r = (amt % 64) as u32;
            for k in 0..n {
                let lo_v = if k >= q {
                    self.val.get(k - q).copied().unwrap_or(0)
                } else {
                    0
                };
                let lo_u = if k >= q {
                    self.unk.get(k - q).copied().unwrap_or(0)
                } else {
                    0
                };
                if r == 0 {
                    val[k] = lo_v;
                    unk[k] = lo_u;
                } else {
                    let bv = if k > q {
                        self.val.get(k - q - 1).copied().unwrap_or(0)
                    } else {
                        0
                    };
                    let bu = if k > q {
                        self.unk.get(k - q - 1).copied().unwrap_or(0)
                    } else {
                        0
                    };
                    val[k] = (lo_v << r) | (bv >> (64 - r));
                    unk[k] = (lo_u << r) | (bu >> (64 - r));
                }
            }
        }
        let mut out = Value {
            val,
            unk,
            width: grow_w,
            signed: self.signed,
            is_real: false,
            is_str: false,
        };
        out.mask_top();
        out
    }

    /// `resize` but extension policy follows the *combined* signedness (only
    /// sign-extend when context-signed, per IEEE 1364-2001 §4.5).
    pub fn resize_keep_sign(mut self, w: u32, ctx_signed: bool) -> Value {
        if self.is_real {
            return self; // FIRST: before any `.signed` mutation — a real is width/sign-free.
        }
        if self.is_str {
            // v7: a string's width is its DYNAMIC length — context sizing
            // never truncates it; the write funnel owns §6.16 conversion.
            return self;
        }
        self.signed = self.signed && ctx_signed;
        let mut out = self.resize(w);
        out.signed = ctx_signed;
        out
    }

    /// v7 P2-C: the byte string a value DENOTES (§6.16 conversion): the
    /// packed bytes MSB-first with LEADING NULs stripped (interior NULs
    /// kept). X/Z bits read through the value plane (2-state collapse).
    pub fn to_str_bytes(&self) -> Vec<u8> {
        let nbytes = (self.width as usize).div_ceil(8);
        let mut out = Vec::with_capacity(nbytes);
        let mut started = false;
        for bi in (0..nbytes).rev() {
            let bit = bi * 8;
            let byte = (self.val.get(bit / 64).copied().unwrap_or(0) >> (bit % 64)) as u8;
            if !started && byte == 0 {
                continue;
            }
            started = true;
            out.push(byte);
        }
        out
    }

    /// v7 P2-C: build a STRING-domain value from raw bytes (8×len packed,
    /// MSB-first; empty ⇒ a width-8 zero that renders/strips back to "").
    pub fn from_str_bytes(bytes: &[u8]) -> Value {
        let w = ((bytes.len() as u32) * 8).max(8);
        let mut v = Value::zeros(w, false);
        for (i, &b) in bytes.iter().rev().enumerate() {
            let bit = i * 8;
            v.val[bit / 64] |= (b as u64) << (bit % 64);
        }
        v.is_str = true;
        v
    }

    /// Build a real Value from an f64. width=64, signed=true, unk=0, is_real.
    pub fn from_f64(x: f64) -> Value {
        Value {
            val: Words::from_word(x.to_bits()),
            unk: Words::zeros(1),
            width: 64,
            signed: true,
            is_real: true,
            is_str: false,
        }
    }

    /// Decode to f64. If already real, reinterpret val[0]. Otherwise coerce the
    /// 4-state integer value to f64 (IEEE 1364 §4.3 int→real promotion), honoring
    /// signedness. Returns None only if an integer operand is X/Z (caller decides
    /// poison vs 0.0).
    pub fn to_f64(&self) -> Option<f64> {
        if self.is_real {
            return Some(f64::from_bits(self.val[0]));
        }
        if self.has_xz() {
            return None;
        }
        if self.signed {
            self.to_i128_signed().map(|i| i as f64)
        } else {
            self.to_u128().map(|u| u as f64)
        }
    }

    /// Build an INTEGER (is_real=false) Value of `width` bits from an i128,
    /// masked to width with two's-complement wrap. Constructor the real→int
    /// coercion and `$rtoi` need. Keeps the low `width` bits of `i`'s two's-
    /// complement image; `signed` only stamps the result's sign flag.
    pub fn from_i128(i: i128, width: u32, signed: bool) -> Value {
        let mut v = Value::zeros(width.max(1), signed);
        let bits = i as u128; // reinterpret two's-complement bit image
        let words = (width as usize).div_ceil(64);
        for w in 0..words.min(v.val.len()) {
            v.val[w] = (bits >> (w * 64)) as u64;
        }
        v.width = width;
        v.mask_top(); // is_real=false so mask_top applies (clears bits above width)
        v
    }
}

/// real → int assignment coercion: ROUND half-away-from-zero, then build an
/// integer Value masked to the target width. Saturation/NaN handling: large |x|
/// saturates to i128 extremes; NaN.round() as i128 == 0.
pub(crate) fn real_to_int_round(x: f64, width: u32, signed: bool) -> Value {
    let r = x.round(); // Rust f64::round = round-half-away-from-zero
    let i = r as i128; // large |x| SATURATES to i128 extremes; NaN → 0
    Value::from_i128(i, width, signed)
}

/// Set bits `[lo, hi)` of the plane pair to the constant 4-state bit `(fv,fu)`,
/// word-parallel (used for sign-extension fill in `resize`/`shr_fill`). `fv`/`fu` are
/// 0/1 flags; each is broadcast across the affected lanes.
fn fill_bits(val: &mut [u64], unk: &mut [u64], lo: u32, hi: u32, fv: u64, fu: u64) {
    if lo >= hi {
        return;
    }
    let fvw = if fv != 0 { u64::MAX } else { 0 };
    let fuw = if fu != 0 { u64::MAX } else { 0 };
    let mut i = lo;
    while i < hi {
        let w = (i / 64) as usize;
        let s = i % 64;
        let end = (((w as u32) + 1) * 64).min(hi); // end of this word's fill region
        let bits = end - i;
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            ((1u64 << bits) - 1) << s
        };
        if let Some(slot) = val.get_mut(w) {
            *slot = (*slot & !mask) | (fvw & mask);
        }
        if let Some(slot) = unk.get_mut(w) {
            *slot = (*slot & !mask) | (fuw & mask);
        }
        i = end;
    }
}

/// Low mask over `width` bits in a single u64 (width ≤ 64 usage).
#[inline]
pub(crate) fn low_mask(width: u32) -> u64 {
    if width >= 64 {
        u64::MAX
    } else if width == 0 {
        0
    } else {
        (1u64 << width) - 1
    }
}

// ── word-parallel 4-state primitives (64 bits at once) ─────────────────────
//
// These compute the result (val, unk) plane WORDS for 64 independent 4-state bits
// in one branchless pass, replacing the per-bit `and1`/`or1`/… fold inside the hot
// element-wise paths. Encoding (per bit): 0=(v0,u0), 1=(v1,u0), X=(v0,u1),
// Z=(v1,u1); `unk` (u) is the "not definite" mask, so Z behaves as X in
// expressions (IEEE 1364 §4). Derivation is verified bit-for-bit against the
// per-bit tables by `word_vs_bit_parity` below.
//
// NOTE on width: callers MUST mask the result's last partial word to the valid
// bit count — `not_w`/`xnor_w` set the high "0&0" region to 1 (`!0 & !0`), which
// would corrupt bits ≥ width if left unmasked. `and_w`/`or_w`/`xor_w` already
// yield 0 there, but masking uniformly is cheap and keeps the value canonical.

/// Word AND: 0 where either is definite-0; 1 where both definite-1; else X.
#[inline]
pub(crate) fn and_w(av: u64, au: u64, bv: u64, bu: u64) -> (u64, u64) {
    let known0 = (!au & !av) | (!bu & !bv);
    let known1 = (!au & av) & (!bu & bv);
    (known1, !known0 & !known1)
}

/// Word OR: 1 where either is definite-1; 0 where both definite-0; else X.
#[inline]
pub(crate) fn or_w(av: u64, au: u64, bv: u64, bu: u64) -> (u64, u64) {
    let known1 = (!au & av) | (!bu & bv);
    let known0 = (!au & !av) & (!bu & !bv);
    (known1, !known1 & !known0)
}

/// Word XOR: X where either is unknown; else val xor.
#[inline]
pub(crate) fn xor_w(av: u64, au: u64, bv: u64, bu: u64) -> (u64, u64) {
    let ru = au | bu;
    ((av ^ bv) & !ru, ru)
}

/// Word XNOR: X where either is unknown; else NOT(val xor).
#[inline]
pub(crate) fn xnor_w(av: u64, au: u64, bv: u64, bu: u64) -> (u64, u64) {
    let ru = au | bu;
    (!(av ^ bv) & !ru, ru)
}

/// Word NOT: X where unknown; else bit-complement of the definite value.
#[inline]
pub(crate) fn not_w(av: u64, au: u64) -> (u64, u64) {
    (!av & !au, au)
}

// ── per-bit 4-state reference truth tables (test oracles) ──────────────────
//
// The hot paths use the word-parallel `*_w` forms above; these explicit per-bit
// tables remain as the readable spec that `word_vs_bit_parity` checks `*_w`
// against (so a future formula edit can't silently drift). `not1` is also the
// non-test result inverter for the N-form reductions.

/// One-bit AND: any definite 0 → 0; both definite 1 → 1; else (x/z, no 0) → X.
#[cfg(test)]
#[inline]
pub(crate) fn and1(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    let a0 = a.1 == 0 && a.0 == 0;
    let b0 = b.1 == 0 && b.0 == 0;
    if a0 || b0 {
        return (0, 0);
    }
    if a.1 != 0 || b.1 != 0 {
        return (0, 1);
    }
    (1, 0)
}

/// One-bit OR: any definite 1 → 1; both definite 0 → 0; else (x/z, no 1) → X.
#[cfg(test)]
#[inline]
pub(crate) fn or1(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    let a1 = a.1 == 0 && a.0 == 1;
    let b1 = b.1 == 0 && b.0 == 1;
    if a1 || b1 {
        return (1, 0);
    }
    if a.1 != 0 || b.1 != 0 {
        return (0, 1);
    }
    (0, 0)
}

/// One-bit XOR: any x/z operand → X; else val xor.
#[cfg(test)]
#[inline]
pub(crate) fn xor1(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    if a.1 != 0 || b.1 != 0 {
        return (0, 1);
    }
    ((a.0 ^ b.0) & 1, 0)
}

/// One-bit NOT: ~x = x; ~0 = 1; ~1 = 0.
#[inline]
pub(crate) fn not1(a: (u64, u64)) -> (u64, u64) {
    if a.1 != 0 {
        return (0, 1);
    }
    ((!a.0) & 1, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn and_or_xz_tables() {
        let zero = (0, 0);
        let one = (1, 0);
        let x = (0, 1);
        let z = (1, 1);
        // AND: 0&x=0, 1&x=x, x&x=x, 0&z=0
        assert_eq!(and1(zero, x), (0, 0));
        assert_eq!(and1(one, x), (0, 1));
        assert_eq!(and1(x, x), (0, 1));
        assert_eq!(and1(zero, z), (0, 0));
        assert_eq!(and1(one, one), (1, 0));
        // OR: 1|x=1, 0|x=x, x|x=x
        assert_eq!(or1(one, x), (1, 0));
        assert_eq!(or1(zero, x), (0, 1));
        assert_eq!(or1(x, x), (0, 1));
        assert_eq!(or1(zero, zero), (0, 0));
        // NOT
        assert_eq!(not1(x), (0, 1));
        assert_eq!(not1(zero), (1, 0));
        assert_eq!(not1(one), (0, 0));
    }

    /// The word-parallel `*_w` primitives must agree bit-for-bit with the per-bit
    /// `*1` tables for every 4-state input pair, across all 64 lane positions.
    #[test]
    fn word_vs_bit_parity() {
        type Bit = (u64, u64);
        let states: [Bit; 4] = [(0, 0), (1, 0), (0, 1), (1, 1)]; // 0,1,X,Z
        for &(a0, a1) in &states {
            for &(b0, b1) in &states {
                // place the same bit in all 64 lanes
                let (av, au) = (a0 * u64::MAX, a1 * u64::MAX);
                let (bv, bu) = (b0 * u64::MAX, b1 * u64::MAX);
                let expect = |f: fn(Bit, Bit) -> Bit| {
                    let (v, u) = f((a0, a1), (b0, b1));
                    (v * u64::MAX, u * u64::MAX)
                };
                assert_eq!(and_w(av, au, bv, bu), expect(and1), "AND {a0}{a1} {b0}{b1}");
                assert_eq!(or_w(av, au, bv, bu), expect(or1), "OR {a0}{a1} {b0}{b1}");
                assert_eq!(xor_w(av, au, bv, bu), expect(xor1), "XOR {a0}{a1} {b0}{b1}");
                // XNOR = NOT(XOR), bit-exact
                let (xv, xu) = not1(xor1((a0, a1), (b0, b1)));
                assert_eq!(
                    xnor_w(av, au, bv, bu),
                    (xv * u64::MAX, xu * u64::MAX),
                    "XNOR {a0}{a1} {b0}{b1}"
                );
            }
            let (av, au) = (a0 * u64::MAX, a1 * u64::MAX);
            let (nv, nu) = not1((a0, a1));
            assert_eq!(
                not_w(av, au),
                (nv * u64::MAX, nu * u64::MAX),
                "NOT {a0}{a1}"
            );
        }
    }

    /// The word-parallel `shr_fill`/`shl_grow` must agree bit-for-bit with the prior
    /// per-bit shift forms, across widths spanning word boundaries, shift amounts
    /// (0 / sub-word / word-multiple / ≥ width), and X/Z-laced operands + arith sign fill.
    #[test]
    fn shift_word_vs_bit_parity() {
        fn ref_shr(l: &Value, amt: u64, w: u32, fv: u64, fu: u64) -> Value {
            let mut out = Value::zeros(w.max(1), l.signed);
            out.width = w;
            for i in 0..w {
                let src = i as u64 + amt;
                if src < w as u64 {
                    let (v, u) = l.get_vu(src as u32);
                    out.set_vu(i, v, u);
                } else {
                    out.set_vu(i, fv, fu);
                }
            }
            out.mask_top();
            out
        }
        fn ref_shl(l: &Value, amt: u64, grow: u32) -> Value {
            let mut out = Value::zeros(grow.max(1), l.signed);
            out.width = grow;
            for i in 0..grow {
                if (i as u64) >= amt {
                    let src = i as u64 - amt;
                    if src < l.width as u64 {
                        let (v, u) = l.get_vu(src as u32);
                        out.set_vu(i, v, u);
                    }
                }
            }
            out.mask_top();
            out
        }
        fn mk(width: u32) -> Value {
            let mut v = Value::zeros(width.max(1), true);
            v.width = width;
            for i in 0..width {
                let (val, unk) = match i % 4 {
                    0 => (0, 0),
                    1 => (1, 0),
                    2 => (0, 1),
                    _ => (1, 1),
                }; // 0,1,X,Z by bit index
                v.set_vu(i, val, unk);
            }
            v.mask_top();
            v
        }
        for &width in &[1u32, 7, 64, 65, 96, 128, 130] {
            let l = mk(width);
            let (mv, mu) = if width > 0 {
                l.get_vu(width - 1)
            } else {
                (0, 0)
            };
            for &amt in &[0u64, 1, 3, 63, 64, 65, 100, width as u64, width as u64 + 5] {
                for &(fv, fu) in &[(0u64, 0u64), (mv, mu)] {
                    assert_eq!(
                        l.shr_fill(amt, width, fv, fu),
                        ref_shr(&l, amt, width, fv, fu),
                        "shr width={width} amt={amt} fill=({fv},{fu})"
                    );
                }
                let grow = (width as u64).saturating_add(amt).min(4096) as u32;
                let w = grow.max(width).max(1);
                assert_eq!(
                    l.shl_grow(amt, w),
                    ref_shl(&l, amt, w),
                    "shl width={width} amt={amt} grow={w}"
                );
            }
        }
    }

    #[test]
    fn resize_signext() {
        let mut v = Value::zeros(4, true);
        // 0b1000 = -8 as signed 4-bit
        v.set_vu(3, 1, 0);
        let w = v.resize(8);
        // sign-extend: bits 4..8 should be 1
        for i in 4..8 {
            assert_eq!(w.get_vu(i), (1, 0));
        }
    }

    #[test]
    fn packed_roundtrip() {
        let b = BitPacked {
            val: vec![0b1010],
            unk: vec![0b0100],
        };
        let v = Value::from_packed(&b, 4, false);
        assert_eq!(v.get_vu(1), (1, 0));
        assert_eq!(v.get_vu(2), (0, 1)); // X
        let back = v.into_bitpacked(4);
        assert_eq!(back.val[0] & 0xF, 0b1010);
        assert_eq!(back.unk[0] & 0xF, 0b0100);
    }
}
