//! 4-state integer-literal parser (IEEE 1364-2005 §3.5 / §3.5.1).
//!
//! Lowers a verbatim IntLit lexeme (`raw`, including the apostrophe) into a
//! `sim_ir::ConstVal`. The two output planes follow the FROZEN encoding
//! `(v,u)`: `00→0`, `10→1`, `01→X`, `11→Z`; word0 bit0 = LSB.
//!
//! Forms handled (lexer-validated upstream — see hdl-lexer IntSized/
//! IntUnsizedBased/IntDecimal):
//!   `42`, `1_000`                         → Decimal, 32-bit signed
//!   `'b1101`, `'hFF`, `'sd9`              → UnsizedBased, 32-bit
//!   `8'hAB`, `4'b1010`, `4'sd5`           → Sized
//!   `8'b1010_xxxx`, `4'bx`, `8'hzz`       → x/z digits
//!   `'0 '1 'x 'z`                         → SV single-char fills (UnsizedBased)
//!   `?` is an alias for `z`.
//!
//! ## Sign/extension rule (§3.5.1 — decided and pinned here)
//! When the supplied digits yield fewer bits than `width`, the literal is
//! LEFT-extended to `width`. The extension bit is the *state* of the supplied
//! MSB: `x` → x-extend, `z` → z-extend, else (`0`/`1`) → **0**-extend. This is
//! NOT sign-extension — a `1` MSB never replicates. (Sign-extension into a wider
//! *expression context* is a separate concern handled at expr-context sizing,
//! deferred past v1.) Excess bits beyond `width` are truncated (MSBs dropped).

use hdl_ast::IntLitKind;
use sim_ir::{BitPacked, ConstRepr, ConstVal};

/// One parsed 4-state bit, in (val, unk) plane form.
#[derive(Clone, Copy)]
struct Bit {
    v: bool,
    u: bool,
}
const B0: Bit = Bit { v: false, u: false };
const B1: Bit = Bit { v: true, u: false };
const BX: Bit = Bit { v: false, u: true };
const BZ: Bit = Bit { v: true, u: true };

/// v1 width cap, mirrored from `elaborate::MAX_NET_WIDTH` (`1 << 20`). A literal
/// whose RESOLVED width exceeds this is rejected by `lower_int_literal`; the same
/// bound clamps the `pack_bits` allocation here so a tiny source like
/// `4294967295'h1` (or `4294967295'0`) cannot size a multi-gigabyte `BitPacked`
/// BEFORE that reject runs. In-cap widths are never touched (byte-identical).
const LITERAL_WIDTH_CAP: u32 = 1 << 20;

/// Max decimal digits accepted before the base conversion runs. A magnitude with
/// MORE digits than this can never fit `LITERAL_WIDTH_CAP` bits, so it is rejected
/// here in O(n) (a digit scan) rather than running the (still O(n²)) conversion to
/// completion and only THEN hitting the width cap — i.e. this is a TIME bound, not
/// just the width-cap boundary. `floor(cap·log10 2)` is the digit count of the
/// widest in-cap value; `30103/100000 ≥ log10 2` and the `+4` margin keep the bound
/// on the LOOSE side so no in-cap literal is ever wrongly rejected (the authoritative
/// width-cap reject still lives in `lower_int_literal`). For `cap = 1 << 20` this is
/// 315 656 — a literal of 315 657+ digits rejects in microseconds.
const MAX_DECIMAL_DIGITS: usize = (LITERAL_WIDTH_CAP as usize * 30103) / 100_000 + 4;

/// Words actually allocated for a literal whose REPORTED width may exceed the v1
/// cap. Clamping the allocation (NOT the reported width) means an over-cap literal
/// — which `lower_int_literal` rejects loud — cannot size a giant `BitPacked`
/// first. Any in-cap width passes through unchanged.
fn alloc_width(width: u32) -> u32 {
    width.min(LITERAL_WIDTH_CAP.saturating_add(64))
}

/// Pack an LSB-first `Bit` slice into `width` bits. Missing bits use `fill`;
/// bits beyond `width` are dropped. Result has `ceil(width/64)` words (≥1).
fn pack_bits(bits: &[Bit], width: u32, fill: Bit) -> BitPacked {
    let nwords = (((width as usize) + 63) / 64).max(1);
    let mut val = vec![0u64; nwords];
    let mut unk = vec![0u64; nwords];
    for i in 0..(width as usize) {
        let b = if i < bits.len() { bits[i] } else { fill };
        let w = i / 64;
        let off = i % 64;
        if b.v {
            val[w] |= 1u64 << off;
        }
        if b.u {
            unk[w] |= 1u64 << off;
        }
    }
    BitPacked { val, unk }
}

/// Map one based digit (`b`/`o`/`h`) to `k` LSB-first bits. `x`/`z`/`?` fill the
/// whole digit. Returns `None` for an out-of-radix digit.
fn digit_bits(c: char, base: u32) -> Option<Vec<Bit>> {
    let k = match base {
        2 => 1,
        8 => 3,
        16 => 4,
        _ => return None,
    };
    match c {
        'x' | 'X' => return Some(vec![BX; k]),
        'z' | 'Z' | '?' => return Some(vec![BZ; k]),
        _ => {}
    }
    let d = c.to_digit(base)?;
    let mut out = Vec::with_capacity(k);
    for i in 0..k {
        out.push(if (d >> i) & 1 == 1 { B1 } else { B0 });
    }
    Some(out)
}

/// Build the LSB-first bit vector for a based digit string (`b`/`o`/`h`).
fn based_bits(digits: &str, base: u32) -> Option<Vec<Bit>> {
    // Accumulate MSB-first (source order), then reverse to LSB-first.
    let mut msb_first: Vec<Bit> = Vec::new();
    for c in digits.chars() {
        let db = digit_bits(c, base)?;
        for b in db.iter().rev() {
            msb_first.push(*b);
        }
    }
    if msb_first.is_empty() {
        return None; // a based literal with no digits is malformed
    }
    msb_first.reverse();
    Some(msb_first)
}

/// Decimal magnitude → LSB-first, TRIMMED (`len = msb+1`) bit vector. Overflow-proof
/// and width-agnostic. `None` on a non-digit char, an empty string, or a digit count
/// past `MAX_DECIMAL_DIGITS` (DoS guard — such a value can never fit the v1 width cap).
///
/// Conversion is Horner in base 10¹⁹ over little-endian base-2⁶⁴ limbs: each ≤19-digit
/// chunk does `limbs = limbs*10ⁿ + chunk` in O(limbs). The previous version emitted
/// one output bit per full-width division pass (plus a fresh `Vec` per bit), making it
/// O(n²) with a large constant — a ~40 000-digit literal took tens of seconds. This is
/// still O(n²) in the worst accepted case, but with a ~700× smaller constant; the
/// `MAX_DECIMAL_DIGITS` guard bounds n so the wall-clock stays well in hand.
fn decimal_bits(digits: &str) -> Option<Vec<Bit>> {
    // Validate + collect digit VALUES (0..=9). O(n); rejects any non-digit.
    let mut dv: Vec<u8> = Vec::with_capacity(digits.len());
    for b in digits.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        dv.push(b - b'0');
    }
    if dv.is_empty() {
        return None;
    }
    if dv.len() > MAX_DECIMAL_DIGITS {
        return None; // exceeds the v1 width cap regardless of digit content
    }

    // 10ⁿ for n ∈ 0..=19; 10¹⁹ < 2⁶⁴, so every ≤19-digit chunk value fits one u64.
    const POW10: [u64; 20] = [
        1,
        10,
        100,
        1_000,
        10_000,
        100_000,
        1_000_000,
        10_000_000,
        100_000_000,
        1_000_000_000,
        10_000_000_000,
        100_000_000_000,
        1_000_000_000_000,
        10_000_000_000_000,
        100_000_000_000_000,
        1_000_000_000_000_000,
        10_000_000_000_000_000,
        100_000_000_000_000_000,
        1_000_000_000_000_000_000,
        10_000_000_000_000_000_000,
    ];
    const CHUNK: usize = 19;
    // Horner base-10¹⁹ into little-endian base-2⁶⁴ limbs (no leading-zero limbs;
    // value 0 → empty `limbs`). The most-significant chunk may be 1..=19 digits.
    let mut limbs: Vec<u64> = Vec::new();
    let n = dv.len();
    let lead = if n % CHUNK == 0 { CHUNK } else { n % CHUNK };
    let mut i = 0;
    while i < n {
        let end = if i == 0 { lead } else { i + CHUNK };
        let mut chunk = 0u64;
        for &d in &dv[i..end] {
            chunk = chunk * 10 + d as u64; // ≤19 digits → ≤10¹⁹−1 < 2⁶⁴
        }
        // limbs = limbs * 10^(end-i) + chunk. `carry` stays ≤ 10¹⁹ < 2⁶⁴ throughout
        // (limb<2⁶⁴, m≤10¹⁹, so cur ≤ 10¹⁹·2⁶⁴ < u128::MAX; cur>>64 ≤ 10¹⁹).
        let m = POW10[end - i] as u128;
        let mut carry = chunk as u128;
        for limb in limbs.iter_mut() {
            let cur = (*limb as u128) * m + carry;
            *limb = cur as u64;
            carry = cur >> 64;
        }
        while carry != 0 {
            limbs.push(carry as u64);
            carry >>= 64;
        }
        i = end;
    }

    // LSB-first bit image, then trim trailing zero bits (keep ≥1) so the result is
    // the minimal `len = msb+1` vector the callers (width = msb+1/+2) expect.
    let mut bits: Vec<Bit> = Vec::with_capacity(limbs.len() * 64);
    for &limb in &limbs {
        for b in 0..64 {
            bits.push(if (limb >> b) & 1 == 1 { B1 } else { B0 });
        }
    }
    while bits.len() > 1 && !bits.last().unwrap().v {
        bits.pop();
    }
    if bits.is_empty() {
        bits.push(B0); // the value 0
    }
    Some(bits)
}

/// The extension bit per §3.5.1: x/z-extend if the supplied MSB is x/z, else 0.
fn ext_fill(bits: &[Bit]) -> Bit {
    match bits.last().copied().unwrap_or(B0) {
        Bit { v: false, u: true } => BX, // X
        Bit { v: true, u: true } => BZ,  // Z
        _ => B0,                         // 0 or 1 → zero-extend
    }
}

/// The fill `Bit` of an unsized single-bit fill literal (`'0`/`'1`/`'x`/`'z`,
/// IEEE §5.7.1) — `None` for any other literal. A fill is unsized (no size
/// prefix), an optional `s`, then exactly one of `0 1 x X z Z ?`.
fn fill_bit(raw: &str, kind: IntLitKind) -> Option<Bit> {
    if kind != IntLitKind::UnsizedBased {
        return None;
    }
    let tick = raw.find('\'')?;
    if tick != 0 {
        return None; // a fill literal carries no explicit width
    }
    let mut rest = raw[1..].chars();
    let mut c = rest.next()?;
    if matches!(c, 's' | 'S') {
        c = rest.next()?;
    }
    let fill = match c {
        '0' => B0,
        '1' => B1,
        'x' | 'X' => BX,
        'z' | 'Z' | '?' => BZ,
        _ => return None,
    };
    if rest.next().is_some() {
        return None; // trailing junk ⇒ not a bare fill
    }
    Some(fill)
}

/// Is `raw`/`kind` an unsized single-bit fill literal (`'0`/`'1`/`'x`/`'z`)?
/// A cheap predicate the caller uses to decide whether to apply context-width
/// sizing (IEEE §5.7.1 / §11.4.4) before computing the assignment target width.
pub fn is_fill_literal(raw: &str, kind: IntLitKind) -> bool {
    fill_bit(raw, kind).is_some()
}

/// Build the `ConstVal` of a fill literal at the CONTEXT width `width` (the fill
/// bit replicated across all `width` bits). `None` if `raw`/`kind` is not a fill
/// literal. Fills are UNSIGNED (iverilog parity: `'1 > 0` is true); the engine
/// therefore does not sign-extend them in narrower-than-context uses.
pub fn fill_literal_const(raw: &str, kind: IntLitKind, width: u32) -> Option<ConstVal> {
    let fill = fill_bit(raw, kind)?;
    Some(ConstVal {
        width,
        signed: false,
        repr: ConstRepr::Numeric,
        bits: pack_bits(&[], alloc_width(width), fill),
    })
}

/// Parse a raw IntLit lexeme into a `ConstVal`. `None` ⇒ malformed (caller emits
/// a diagnostic and substitutes a zero const).
pub fn parse_int_literal(raw: &str, kind: IntLitKind) -> Option<ConstVal> {
    match kind {
        // ── plain decimal: 42 → 32-bit signed ──────────────────────────
        IntLitKind::Decimal => {
            let digits: String = raw.chars().filter(|&c| c != '_').collect();
            let bits = decimal_bits(&digits)?;
            // IEEE §3.5.1: an unsized literal is "at least 32 bits", grown to
            // hold its value (iverilog parity). A plain decimal is SIGNED, so a
            // positive value needs one extra (sign) bit above its MSB:
            // width = max(32, nbits+1). `decimal_bits` is trimmed (len = msb+1),
            // so for any value < 2^31 this stays 32 (pre-P0-10 byte-identical).
            let width = (bits.len() as u32 + 1).max(32);
            let bits_packed = pack_bits(&bits, alloc_width(width), B0);
            Some(ConstVal {
                width,
                signed: true,
                repr: ConstRepr::Numeric,
                bits: bits_packed,
            })
        }

        // ── sized / unsized based: [W]'[s]B digits ─────────────────────
        IntLitKind::Sized | IntLitKind::UnsizedBased => {
            let tick = raw.find('\'')?;
            let size_part = &raw[..tick];
            let mut rest = raw[tick + 1..].chars().peekable();

            // optional signed marker
            let mut signed = false;
            if matches!(rest.peek(), Some('s') | Some('S')) {
                signed = true;
                rest.next();
            }

            // explicit width for Sized; None for unsized (computed from the
            // digits below, P0-10 — IEEE §3.5.1 "at least 32", grown to hold
            // the value, iverilog parity).
            let explicit_width: Option<u32> = if size_part.is_empty() {
                None
            } else {
                let sd: String = size_part.chars().filter(|&c| c != '_').collect();
                Some(sd.parse::<u32>().ok()?)
            };
            if explicit_width == Some(0) {
                return None;
            }

            let base_char = rest.peek().copied().unwrap_or('\0');
            let is_base = matches!(base_char, 'd' | 'D' | 'b' | 'B' | 'o' | 'O' | 'h' | 'H');

            if !is_base {
                // SV single-char fill: '0 '1 'x 'z (no base letter, no digits).
                // Self-determined width is context-sized; keep the v1 default 32.
                let fill = match base_char {
                    '0' => B0,
                    '1' => B1,
                    'x' | 'X' => BX,
                    'z' | 'Z' | '?' => BZ,
                    _ => return None,
                };
                rest.next();
                if rest.next().is_some() {
                    return None; // trailing junk after a single fill char
                }
                let width = explicit_width.unwrap_or(32);
                return Some(ConstVal {
                    width,
                    signed,
                    repr: ConstRepr::Numeric,
                    bits: pack_bits(&[], alloc_width(width), fill),
                });
            }

            rest.next(); // consume base letter
            let digits: String = rest.filter(|&c| c != '_').collect();
            let base: u32 = match base_char {
                'd' | 'D' => 10,
                'b' | 'B' => 2,
                'o' | 'O' => 8,
                'h' | 'H' => 16,
                _ => return None,
            };

            let bits: Vec<Bit> = if base == 10 {
                // decimal-based: x/z legal ONLY as the whole single-token value.
                let lc = digits.to_ascii_lowercase();
                if lc == "x" {
                    vec![BX]
                } else if lc == "z" || lc == "?" {
                    vec![BZ]
                } else {
                    decimal_bits(&digits)?
                }
            } else {
                based_bits(&digits, base)?
            };

            // Unsized width (P0-10): based h/b/o use the DIGIT span itself
            // (`bits.len()` = digits×bits-per-digit — `'h1FFFFFFFF`→36, NOT the
            // value MSB 33). Based decimal uses the trimmed value width; a SIGNED
            // `'sd` needs one extra sign bit (like plain decimal), `'d`/h/b/o do
            // not. ≤32-bit literals stay 32 (pre-P0-10 byte-identical).
            let width = match explicit_width {
                Some(w) => w,
                None => {
                    let natural = if base == 10 && signed {
                        bits.len() as u32 + 1
                    } else {
                        bits.len() as u32
                    };
                    natural.max(32)
                }
            };

            let fill = ext_fill(&bits);
            Some(ConstVal {
                width,
                signed,
                repr: ConstRepr::Numeric,
                bits: pack_bits(&bits, alloc_width(width), fill),
            })
        }
    }
}

/// Parse a raw `StrLit` lexeme (e.g. `"hello\n"`, WITH the surrounding quotes,
/// escapes unprocessed by the parser) into a `StrUtf8` `ConstVal`.
///
/// The surrounding double-quotes are stripped (recovery-safe if one is missing),
/// C-style escapes (`\n \t \r \\ \" \0`) are processed, and the resulting UTF-8
/// bytes are packed in IEEE §5.9 order: the FIRST character is the MOST
/// significant byte (byte `k` of `n` occupies bits `[(n-1-k)*8 .. (n-k)*8)`),
/// so `"ab"` evaluates numerically to 16'h6162 (iverilog live: 24930).
/// `width = nbytes*8`. Strings are 2-state, so the `unk` plane is all zero.
/// (`\ddd` octal / `\xhh` hex are deferred — recovered by literal copy.)
/// (v6 fix: the pre-v6 packing was LSB-first — a latent numeric-surface
/// divergence that string-keyed assoc arrays were the first to expose.)
/// Unescape a raw string-literal lexeme (quotes stripped, C escapes processed)
/// into its byte vector. Shared by `parse_str_literal` and the elaborate-time
/// format-specifier scan (§4.1a).
pub fn unescape_str_literal_bytes(raw: &str) -> Vec<u8> {
    let inner = raw.strip_prefix('"').unwrap_or(raw);
    let inner = inner.strip_suffix('"').unwrap_or(inner);

    let mut bytes: Vec<u8> = Vec::with_capacity(inner.len());
    let mut cs = inner.chars();
    while let Some(c) = cs.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match cs.next() {
            Some('n') => bytes.push(b'\n'),
            Some('t') => bytes.push(b'\t'),
            Some('r') => bytes.push(b'\r'),
            Some('\\') => bytes.push(b'\\'),
            Some('"') => bytes.push(b'"'),
            Some('0') => bytes.push(0),
            Some(other) => {
                bytes.push(b'\\');
                let mut buf = [0u8; 4];
                bytes.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
            }
            None => bytes.push(b'\\'),
        }
    }
    bytes
}

/// The unescaped UTF-8 text of a string literal (lossy for any non-UTF-8 bytes,
/// which never occur in a Verilog format string). Used by the §4.1a static gate.
pub fn parse_str_literal_text(raw: &str) -> String {
    String::from_utf8_lossy(&unescape_str_literal_bytes(raw)).into_owned()
}

pub fn parse_str_literal(raw: &str) -> ConstVal {
    str_const_from_bytes(&unescape_str_literal_bytes(raw))
}

/// Build a `StrUtf8` ConstVal from plain bytes (no quotes/escapes). Used for
/// SYNTHETIC strings too (e.g. the $dumpvars scope-path encoding, ⑤b).
pub fn str_const_from_bytes(bytes: &[u8]) -> ConstVal {
    let width = (bytes.len() as u32).saturating_mul(8);
    let nwords = (((width as usize) + 63) / 64).max(1);
    let mut val = vec![0u64; nwords];
    let unk = vec![0u64; nwords]; // strings are 2-state
    for (k, &b) in bytes.iter().enumerate() {
        // IEEE §5.9: first character = MOST significant byte.
        let bit = (bytes.len() - 1 - k) * 8;
        // bit % 64 ∈ {0,8,..,56} (8 | 64) → a byte never straddles a word.
        val[bit / 64] |= (b as u64) << (bit % 64);
    }
    ConstVal {
        width,
        signed: false,
        repr: ConstRepr::StrUtf8,
        bits: BitPacked { val, unk },
    }
}

/// IEEE 1364 real literal → `ConstVal { repr: Real }`. Underscores are stripped;
/// round-to-nearest-even is Rust's f64 parse default. The f64 is stored as
/// `to_bits()` in `bits.val[0]`, `unk = [0]`, `width = 64`, `signed = true`.
///
/// OVERFLOW: an out-of-range literal (e.g. `1e400`) parses to `Ok(±inf)`, NOT
/// `Err`, so it interns as `±inf` with the canonical IEEE bit pattern
/// (deterministic, §0). `unwrap_or(0.0)` only covers a truly unparseable string,
/// which the validated grammar should never deliver.
pub fn parse_real_literal(raw: &str) -> ConstVal {
    let cleaned: String = raw.chars().filter(|&c| c != '_').collect();
    let x = cleaned.parse::<f64>().unwrap_or(0.0);
    ConstVal {
        width: 64,
        signed: true,
        repr: ConstRepr::Real,
        bits: BitPacked {
            val: vec![x.to_bits()],
            unk: vec![0],
        },
    }
}

/// Parse a raw real literal lexeme to its f64 value (for real delays `#1.5`).
pub fn parse_real_f64(raw: &str) -> f64 {
    let cleaned: String = raw.chars().filter(|&c| c != '_').collect();
    cleaned.parse::<f64>().unwrap_or(0.0)
}

/// Synthesize a 2-state `ConstVal` from an i64 (two's-complement image masked
/// to `width`). Param values outside the legacy u32 range bind through this
/// (negative → 32-bit signed; beyond 32 bits → 64-bit), P0-6.
pub fn make_const_i64(v: i64, width: u32, signed: bool) -> ConstVal {
    let nwords = ((width as usize).div_ceil(64)).max(1);
    let mut val = vec![0u64; nwords];
    let unk = vec![0u64; nwords];
    val[0] = v as u64;
    if width < 64 {
        val[0] &= (1u64 << width) - 1;
    }
    ConstVal {
        width,
        signed,
        repr: ConstRepr::Numeric,
        bits: BitPacked { val, unk },
    }
}

/// Synthesize a small unsigned `ConstVal` of `n` in `width` bits (used for
/// select widths / single-bit selects). Always 2-state (`unk` all zero).
pub fn make_const_u32(n: u32, width: u32) -> ConstVal {
    let nwords = (((width as usize) + 63) / 64).max(1);
    let mut val = vec![0u64; nwords];
    let unk = vec![0u64; nwords];
    for i in 0..(width.min(32) as usize) {
        if (n >> i) & 1 == 1 {
            val[i / 64] |= 1u64 << (i % 64);
        }
    }
    ConstVal {
        width,
        signed: false,
        repr: ConstRepr::Numeric,
        bits: BitPacked { val, unk },
    }
}
