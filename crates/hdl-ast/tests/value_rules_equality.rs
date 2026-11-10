//! PRE-equality pin of `value_rules` (ROADMAP §5.2 PARSE-1, PIN7): every spelling a kernel
//! replaced is copied here VERBATIM from the PRE tree (batch2 bea52457), with its file:line, a
//! receiver or field read becoming a parameter, and run against the kernel call that replaced
//! it over the site's domain grid: equal results, `None` for `None`. Where the two part
//! outside the domain, the row names the first out-of-domain input where they differ, so a
//! producer change that widens a domain is a reviewed edit. Run in the dev profile (a panic
//! in either column fails the row). A value row that changes a kernel edits this pin.

use std::fmt::Debug;
use std::panic::{catch_unwind, AssertUnwindSafe};

use hdl_ast as ast;
use hdl_ast::value_rules::{
    binop_in, clog2_u64, const_binop, const_pow, const_shl_i64, ctx5_binop, decimal_i64, wrap_i128,
    OpSet,
};
use hdl_ast::BinOp;

const ALL: [BinOp; 27] = [
    BinOp::Add,
    BinOp::Sub,
    BinOp::Mul,
    BinOp::Div,
    BinOp::Mod,
    BinOp::Pow,
    BinOp::Shl,
    BinOp::Shr,
    BinOp::AShl,
    BinOp::AShr,
    BinOp::Lt,
    BinOp::Le,
    BinOp::Gt,
    BinOp::Ge,
    BinOp::Eq,
    BinOp::Ne,
    BinOp::CaseEq,
    BinOp::CaseNe,
    BinOp::WildEq,
    BinOp::WildNe,
    BinOp::BitAnd,
    BinOp::BitXor,
    BinOp::BitXnor,
    BinOp::BitOr,
    BinOp::LogAnd,
    BinOp::LogOr,
    BinOp::InsideEq,
];

/// G-INT.
const G_INT: [i64; 15] = [
    0,
    1,
    -1,
    2,
    7,
    -8,
    i32::MIN as i64,
    i32::MAX as i64,
    i64::MIN,
    i64::MIN + 1,
    i64::MAX,
    3037000500,
    -3037000500,
    1 << 62,
    i64::MAX,
];
/// Shift amounts.
const G_SH: [i64; 7] = [0, 1, 62, 63, 64, 65, -1];
/// G-N.
const G_N: [i64; 11] = [
    i64::MIN,
    -1,
    0,
    1,
    2,
    3,
    4,
    5,
    1 << 32,
    (1 << 32) + 1,
    i64::MAX,
];
/// G-STR.
const G_STR: [&str; 31] = [
    "",
    "0",
    "00",
    "007",
    "-0",
    "+5",
    "-5",
    "5",
    "1_0",
    "_",
    "_1",
    "1_",
    "1__0",
    " 5",
    "5 ",
    " 5 ",
    "\t5",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "2147483647",
    "2147483648",
    "4294967295",
    "4294967296",
    "000000000000000000000000005",
    "12345678901234567890",
    "1e3",
    "0x10",
    "5'd3",
    "abc",
];
/// G-WRAP widths.
const G_W: [u32; 12] = [0, 1, 2, 4, 31, 32, 33, 62, 63, 64, 65, 128];

fn int_pairs() -> Vec<(i64, i64)> {
    let mut v = Vec::new();
    for a in G_INT {
        for b in G_INT {
            v.push((a, b));
        }
    }
    v
}

/// The G-WRAP values for width `w` that an i64 holds.
fn wrap_vals(w: u32) -> Vec<i64> {
    let mut v: Vec<i128> = vec![0, 1, -1, i128::from(i64::MIN), i128::from(i64::MAX)];
    // 2^(w-1) - 1, 2^(w-1), 2^w - 1, 2^w where an i64 holds them.
    for k in [w.checked_sub(1), Some(w)].into_iter().flatten() {
        if k < 127 {
            let p = 1i128 << k;
            v.extend([p - 1, p]);
        }
    }
    v.push(i128::from(u64::MAX as i64)); // u64::MAX as a pattern
    v.into_iter()
        .filter_map(|x| i64::try_from(x).ok())
        .collect()
}

/// One value of `f(i)`, a panic read as `Err`.
fn run<I: Copy, O>(f: &impl Fn(I) -> O, i: I) -> Result<O, ()> {
    catch_unwind(AssertUnwindSafe(|| f(i))).map_err(|_| ())
}

/// Asserts PRE == POST on every input of the domain grid; returns the number compared.
fn same<I: Copy + Debug, O: PartialEq + Debug>(
    row: &str,
    inputs: impl IntoIterator<Item = I>,
    pre: impl Fn(I) -> O,
    post: impl Fn(I) -> O,
) -> usize {
    let mut n = 0;
    for i in inputs {
        let (a, b) = (run(&pre, i), run(&post, i));
        assert!(
            a.is_ok() && b.is_ok(),
            "{row} {i:?}: a panic (pre {a:?}, post {b:?})"
        );
        assert_eq!(a, b, "{row} {i:?}: PRE != kernel");
        n += 1;
    }
    assert!(n > 0, "{row}: an empty grid");
    n
}

/// The first input of an out-of-domain grid where PRE and POST part (a panic counts).
fn first_diff<I: Copy + Debug, O: PartialEq>(
    inputs: impl IntoIterator<Item = I>,
    pre: impl Fn(I) -> O,
    post: impl Fn(I) -> O,
) -> Option<I> {
    inputs.into_iter().find(|i| run(&pre, *i) != run(&post, *i))
}

// ───────────────────────── PRE columns, verbatim ─────────────────────────

/// EQ1 PRE: hdl-parser expr_primary.rs:79-91 (try_const_index's Binary arm, after `a` / `b`).
fn pre_try_const_index_binary(op: &BinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        // §3 ⑤ ⓓ: integer division truncates toward zero in both SV
        // (§11.4.3) and Rust; a zero divisor declines (elaborate gives x).
        BinOp::Div if b != 0 => a.checked_div(b),
        _ => None,
    }
}

/// EQ2 PRE: hdl-parser expr_primary.rs:776-786 (const_lit's Binary arm, after `a` / `b`).
fn pre_const_lit_binary(op: &BinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        _ => None,
    }
}

/// EQ3 PRE: hdl-parser expr_primary.rs:660 (the mask), :672-678 (`read`), :732-745 (cw_eval's
/// Binary arm, after `x` / `y`).
fn pre_cw_eval_binary(op: &BinOp, x: i128, y: i128, w: u32, signed: bool) -> Option<i128> {
    let mask: i128 = (1i128 << w) - 1;
    // Read a `w`-bit pattern back as the value `signed` says it is.
    let read = |p: i128| -> i128 {
        if signed && (p >> (w - 1)) & 1 == 1 {
            p - (1i128 << w)
        } else {
            p
        }
    };
    let v = match op {
        BinOp::Add => x + y,
        BinOp::Sub => x - y,
        BinOp::Mul => (read(x) * read(y)) & mask,
        BinOp::Div if y != 0 => read(x) / read(y),
        BinOp::Mod if y != 0 => read(x) % read(y),
        _ => return None,
    };
    Some(v & mask)
}

/// EQ4 (a) PRE: hdl-parser expr_primary.rs:55-61 (try_const_index's `$clog2`, after `n`).
fn pre_clog2_tci(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    Some(i64::from(64 - (n as u64).saturating_sub(1).leading_zeros()))
}

/// EQ4 (b) PRE: hdl-parser expr_primary.rs:725-731 (cw_eval's `$clog2`, `n.v` = `n`, before
/// `leaf`).
fn pre_clog2_cw(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    let v = i64::from(64 - (n as u64).saturating_sub(1).leading_zeros());
    Some(v)
}

/// EQ4 (c) PRE: elaborate const_fn.rs:675-679 (const_clog2_selfdet's tail, `n: u64`).
fn pre_clog2_selfdet(n: u64) -> Option<i64> {
    Some(if n <= 1 {
        0
    } else {
        (64 - (n - 1).leading_zeros()) as i64
    })
}

/// EQ4 (d) PRE: elaborate expr_size_hier.rs:750-758 (env_fold's `$clog2`, after `a`).
fn pre_clog2_env_fold(a: i64) -> Option<i64> {
    if a < 0 {
        return None;
    }
    let v = if a <= 1 {
        0
    } else {
        i64::from(64 - (a - 1).leading_zeros())
    };
    Some(v)
}

/// EQ5 A01-A04 PRE: hdl-parser expr_primary.rs:38-43 (try_const_index), :602-607 (cw_shape),
/// :684-689 (cw_eval), :763-768 (const_lit): one text, byte-identical at the four sites.
fn pre_decimal(raw: &str) -> Option<i64> {
    raw.chars()
        .filter(|c| *c != '_')
        .collect::<String>()
        .parse::<i64>()
        .ok()
}

/// EQ5 A02 PRE: hdl-parser expr_primary.rs:602-608 (cw_shape's decimal leaf + its i32 cutoff).
fn pre_cw_shape_decimal(raw: &str) -> Option<(u32, bool)> {
    let v = raw
        .chars()
        .filter(|c| *c != '_')
        .collect::<String>()
        .parse::<i64>()
        .ok()?;
    i32::try_from(v).ok()?;
    Some((32, true))
}

/// EQ5 A05 PRE: hdl-parser expr_primary.rs:876-880 (based_lit_pattern's size, `raw[..tick]` =
/// `size`).
fn pre_based_size(size: &str) -> Option<u32> {
    let w: String = size.chars().filter(|c| *c != '_').collect();
    let w = w.parse::<u32>().ok()?;
    if w == 0 || w > 64 {
        return None;
    }
    Some(w)
}

/// EQ5 A07 PRE: hdl-parser expr_primary.rs:983 (parse_small_const, `self.cur_text()` = `t`).
fn pre_small_const(t: &str) -> Option<u32> {
    t.replace('_', "").parse::<u32>().ok()
}

/// EQ5 A09 PRE: elaborate block_local/proofs.rs:241 (fold_i32's decimal leaf).
fn pre_fold_i32_decimal(raw: &str) -> Option<i32> {
    raw.trim().replace('_', "").parse::<i32>().ok()
}

/// EQ5 A10 PRE: elaborate const_eval.rs:477 (ast_decimal_lit_i64's leaf).
fn pre_ast_decimal(raw: &str) -> Option<i64> {
    raw.trim().replace('_', "").parse::<i64>().ok()
}

/// EQ5 A11 PRE: elaborate da/mod.rs:814 (the DA constant index's decimal leaf).
fn pre_da_decimal(raw: &str) -> Option<i64> {
    raw.replace('_', "").parse::<i64>().ok()
}

/// EQ5 A12 PRE: elaborate literal.rs:383-392 (parse_int_literal's explicit size + its
/// zero-width test).
fn pre_literal_size(size_part: &str) -> Option<Option<u32>> {
    let explicit_width: Option<u32> = if size_part.is_empty() {
        None
    } else {
        let sd: String = size_part.chars().filter(|&c| c != '_').collect();
        Some(sd.parse::<u32>().ok()?)
    };
    if explicit_width == Some(0) {
        return None;
    }
    Some(explicit_width)
}

/// EQ6 B05 PRE: hdl-parser params.rs:1037-1047 (based_lit_param_value's tail).
fn pre_based_param_tail(pat: u64, w: u32, signed: bool) -> Option<i64> {
    let v = if w == 64 {
        if signed || pat <= i64::MAX as u64 {
            pat as i64
        } else {
            return None;
        }
    } else if signed && (pat >> (w - 1)) & 1 == 1 {
        (pat | (!0u64 << w)) as i64
    } else {
        i64::try_from(pat).ok()?
    };
    Some(v)
}

/// EQ6 B06 PRE: elaborate const_eval.rs:40-56 (coerce_i64_to_width).
fn pre_coerce_i64_to_width(v: i64, w: u32, signed: bool) -> i64 {
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

/// EQ6 B07 PRE: elaborate const_eval.rs:98-112 (coerce_int_width).
fn pre_coerce_int_width(v: i64, width: u32, signed: bool) -> i64 {
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

/// EQ6 B10 PRE: elaborate dead_arm.rs:56-63 (sext).
fn pre_sext(v: u64, w: u32) -> i64 {
    if w >= 64 {
        v as i64
    } else {
        let sh = 64 - w;
        ((v << sh) as i64) >> sh
    }
}

/// EQ6 B12 PRE: elaborate packed.rs:604-609 (const_index_sign_extended's tail, `raw` after its
/// guards).
fn pre_index_sign_extended(raw: u64, w: u32) -> Option<u32> {
    if w == 0 || w > 64 {
        return None;
    }
    let sh = 64 - w;
    Some((((raw << sh) as i64) >> sh) as u32)
}

/// EQ6 B13 PRE: elaborate packed.rs:719-731 (const_index_value's tail; `c.width` = `width`
/// after its `> 64` guard, `c.signed` = `signed`).
fn pre_index_value(raw: u64, width: u32, signed: bool) -> Option<i64> {
    let w = width.max(1);
    let masked = if w >= 64 {
        raw
    } else {
        raw & ((1u64 << w) - 1)
    };
    Some(if signed && w < 64 && (masked >> (w - 1)) & 1 == 1 {
        (masked | (!0u64 << w)) as i64
    } else if signed {
        masked as i64
    } else {
        i64::try_from(masked).unwrap_or(i64::MAX)
    })
}

// EQ7 PRE: elaborate const_fn.rs:36-176 (const_pow, const_shl_i64, const_binop), verbatim but
// for the `pre_` names.
/// `**` in the i64 const domain. Negative exponents follow the IEEE integer
/// table (1**n=1, (-1)**n=±1, 0**neg undefined → None, else 0); overflow → None.
fn pre_const_pow(a: i64, b: i64, exp_signed: bool) -> Option<i64> {
    // The IEEE negative-exponent table applies only when the exponent REALLY is
    // negative. `64'd0 - 64'd8` is an UNSIGNED subtraction — 18446744073709551608,
    // not −8 — and reading the i64 container's sign instead of the expression's
    // made `3 ** (64'd0 - 64'd8)` answer 0 where iverilog, verilator AND vita's own
    // runtime all answer 926288481.
    let neg = exp_signed && b < 0;
    // A base of 0 or ±1 does not depend on the exponent's MAGNITUDE — only on its
    // parity, which is the low bit of the pattern either way. So these answer for
    // ANY exponent, including the huge unsigned ones the i64 domain cannot carry;
    // gating them behind the domain check below turned `1 ** (64'd0 - 64'd8)` and
    // `(-1) ** (64'd0 - 64'd8)` from both oracles' 1 into a decline.
    match a {
        1 => return Some(1),
        -1 => return Some(if b % 2 == 0 { 1 } else { -1 }),
        // `0 ** 0` is 1; `0 ** positive` is 0; `0 ** negative` is undefined.
        0 => {
            return if b == 0 {
                Some(1)
            } else if neg {
                None
            } else {
                Some(0)
            }
        }
        _ => {}
    }
    if neg {
        return Some(0); // |base| ≥ 2 with a genuinely negative exponent
    }
    // ⚠️⚠️ An "unsigned negative" exponent is a HUGE positive one, so the exact
    // result does not fit the i64 domain — DECLINE, the same discipline `+` and
    // `*` keep here (this kernel declines `3037000500 * 3037000500` ; a parameter
    // of that value still binds 145474192 through the width-aware path, as both
    // oracles print).
    //
    // Folding it MODULARLY was tried and reverted. Square-and-multiply mod 2^64
    // gives the right answer at every context of 64 bits or fewer — and both
    // oracles confirmed six such cells — but the module-scope fold has no context
    // width, and a `localparam [127:0] P = 3 ** 41` then zero-extends an
    // ALREADY-TRUNCATED 64-bit value: `resize_bits` cannot restore what the
    // wrapping discarded, so a loud reject became a silent wrong (and at 96 bits,
    // one silent wrong became a different one). Both adversarial lenses landed on
    // it independently. vita's own ENGINE is right there because it works mod
    // 2^128 and switches to exact multi-word kernels past its wide cap.
    //
    // ⇒ the wrapping answer needs a KNOWN context width, which is the width-aware
    // module-scope fold this domain does not have yet — one class with the
    // `+`/`*` overflow, tracked in ROADMAP §2.
    // `b as u64` reads the exponent as the BIT PATTERN it is. (Measured equivalent
    // to `try_from(b)`: anything that fits `u32` is non-negative as an i64 too, so
    // the two decline on exactly the same inputs. Written this way because the
    // pattern reading is the rule, and the next change to this line should start
    // from the rule, not from a coincidence.)
    a.checked_pow(u32::try_from(b as u64).ok()?)
}

/// `<<`/`<<<` in the i64 const domain: value-preserving or None. A shift that
/// loses bits (or lands in the sign bit) would be a silently wrong param value
/// — the round-trip check rejects it loudly. `0 << anything` stays 0.
fn pre_const_shl_i64(a: i64, b: i64) -> Option<i64> {
    if a == 0 {
        return Some(0);
    }
    if !(0..64).contains(&b) {
        return None; // every bit of a non-zero value shifted out / negative amount
    }
    let r = a.checked_shl(b as u32)?;
    if (r >> b) == a {
        Some(r)
    } else {
        None
    }
}

/// Fold one binary operator over two i64 const operands (SIGNED i64 semantics; a
/// folded const carries no x/z, so `===`/`!==`/`==?`/`!=?` collapse to `==`/`!=`).
/// `checked_*` overflow / div-or-mod by zero / a logical `>>` of a negative value
/// (width-dependent) → None → LOUD at the caller. Shared by `const_eval_in_scope`
/// (the plain const domain) and `eval_const_env` (the constant-function interpreter)
/// so both fold identically.
fn pre_const_binop(op: ast::BinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        ast::BinOp::Add => a.checked_add(b),
        ast::BinOp::Sub => a.checked_sub(b),
        ast::BinOp::Mul => a.checked_mul(b),
        ast::BinOp::Div if b != 0 => a.checked_div(b),
        ast::BinOp::Mod if b != 0 => a.checked_rem(b),
        ast::BinOp::Lt => Some((a < b) as i64),
        ast::BinOp::Le => Some((a <= b) as i64),
        ast::BinOp::Gt => Some((a > b) as i64),
        ast::BinOp::Ge => Some((a >= b) as i64),
        // `InsideEq`: an x/z literal never reaches the i64 domain (its fold declines),
        // so the element here has no don't-care bit and §11.4.13's `==?` is `==`.
        ast::BinOp::Eq | ast::BinOp::CaseEq | ast::BinOp::WildEq | ast::BinOp::InsideEq => {
            Some((a == b) as i64)
        }
        ast::BinOp::Ne | ast::BinOp::CaseNe | ast::BinOp::WildNe => Some((a != b) as i64),
        ast::BinOp::BitAnd => Some(a & b),
        ast::BinOp::BitOr => Some(a | b),
        ast::BinOp::BitXor => Some(a ^ b),
        ast::BinOp::BitXnor => Some(!(a ^ b)),
        ast::BinOp::LogAnd => Some(((a != 0) && (b != 0)) as i64),
        ast::BinOp::LogOr => Some(((a != 0) || (b != 0)) as i64),
        // `**` needs the EXPONENT's signedness, which this signature cannot carry —
        // every reachable site folds it through `const_pow` directly. Declining is
        // fail-closed: a site added later goes loud rather than silently reading an
        // unsigned exponent as a negative one.
        // (A mutant that restores the old signed-reading path here SURVIVES the
        // whole suite — which is the proof that the three sites are exhaustive:
        // nothing reaches this arm. The decline stays as the fail-closed guard for
        // a fourth site added later.)
        ast::BinOp::Pow => {
            debug_assert!(false, "Pow must fold through const_pow");
            None
        }
        // `<<`/`<<<`: value-preserving or None (a shifted-out/overflowing value would
        // be silently wrong). `1<<32` folds wide (4294967296), matching iverilog.
        ast::BinOp::Shl | ast::BinOp::AShl => pre_const_shl_i64(a, b),
        // `>>` (logical): well-defined here only for a ≥ 0 (a negative value's logical
        // shift depends on the operand WIDTH, which this domain does not model).
        ast::BinOp::Shr if a >= 0 => {
            if !(0..64).contains(&b) {
                Some(0)
            } else {
                Some(((a as u64) >> b) as i64)
            }
        }
        // `>>>` (arithmetic): sign-extending; an over-width / negative amount saturates.
        ast::BinOp::AShr => {
            if !(0..64).contains(&b) {
                Some(if a < 0 { -1 } else { 0 })
            } else {
                Some(a >> b)
            }
        }
        // Div/Mod by zero, negative-operand `>>` → non-constant.
        _ => None,
    }
}

// ───────────────────────── the rows ─────────────────────────

#[test]
fn eq1_try_const_index_binary() {
    let g = ALL
        .iter()
        .flat_map(|op| int_pairs().into_iter().map(move |(a, b)| (*op, a, b)));
    same(
        "EQ1",
        g,
        |(op, a, b)| pre_try_const_index_binary(&op, a, b),
        |(op, a, b)| binop_in(OpSet::Parser4, op, a, b),
    );
}

#[test]
fn eq2_const_lit_binary() {
    let g = ALL
        .iter()
        .flat_map(|op| int_pairs().into_iter().map(move |(a, b)| (*op, a, b)));
    same(
        "EQ2",
        g,
        |(op, a, b)| pre_const_lit_binary(&op, a, b),
        |(op, a, b)| binop_in(OpSet::Lit3, op, a, b),
    );
}

#[test]
fn eq3_cw_eval_binary() {
    let mut g = Vec::new();
    for w in [1u32, 4, 32, 63, 64] {
        let mask: i128 = (1i128 << w) - 1;
        for signed in [false, true] {
            let read = |p: i128| -> i128 {
                if signed && (p >> (w - 1)) & 1 == 1 {
                    p - (1i128 << w)
                } else {
                    p
                }
            };
            for (a, b) in int_pairs() {
                let (x, y) = (i128::from(a) & mask, i128::from(b) & mask);
                for op in ALL {
                    // E1: a Mul pair whose i128 product overflows panics in both columns
                    // (dev) -- outside the domain this pin measures.
                    if op == BinOp::Mul && read(x).checked_mul(read(y)).is_none() {
                        continue;
                    }
                    g.push((op, x, y, w, signed));
                }
            }
        }
    }
    same(
        "EQ3",
        g,
        |(op, x, y, w, s)| pre_cw_eval_binary(&op, x, y, w, s),
        |(op, x, y, w, s)| {
            let mask: i128 = (1i128 << w) - 1;
            Some(ctx5_binop(op, x, y, w, s)? & mask)
        },
    );
}

#[test]
fn eq4_clog2() {
    same("EQ4a", G_N, pre_clog2_tci, |n| {
        if n < 0 {
            return None;
        }
        Some(i64::from(clog2_u64(n as u64)))
    });
    same("EQ4b", G_N, pre_clog2_cw, |n| {
        if n < 0 {
            return None;
        }
        let v = i64::from(clog2_u64(n as u64));
        Some(v)
    });
    let gu = G_N.iter().map(|n| *n as u64).chain([1u64 << 63, u64::MAX]);
    same("EQ4c", gu, pre_clog2_selfdet, |n| {
        Some(i64::from(clog2_u64(n)))
    });
    same("EQ4d", G_N, pre_clog2_env_fold, |a| {
        if a < 0 {
            return None;
        }
        let v = i64::from(clog2_u64(a as u64));
        Some(v)
    });
}

#[test]
fn eq5_decimal() {
    same("EQ5 A01-A04", G_STR, pre_decimal, decimal_i64);
    same("EQ5 A02", G_STR, pre_cw_shape_decimal, |raw| {
        let v = decimal_i64(raw)?;
        i32::try_from(v).ok()?;
        Some((32, true))
    });
    same("EQ5 A05", G_STR, pre_based_size, |size| {
        let w = decimal_i64(size).and_then(|v| u32::try_from(v).ok())?;
        if w == 0 || w > 64 {
            return None;
        }
        Some(w)
    });
    // A07: an IntDecimal token's text (`\d[\d_]*`, hdl-lexer) -- no sign, no space.
    let int_decimal = |t: &str| {
        t.as_bytes().first().is_some_and(u8::is_ascii_digit)
            && t.bytes().all(|b| b.is_ascii_digit() || b == b'_')
    };
    let post_small = |t: &str| decimal_i64(t).and_then(|v| u32::try_from(v).ok());
    same(
        "EQ5 A07",
        G_STR.into_iter().filter(|t| int_decimal(t)),
        pre_small_const,
        post_small,
    );
    let differ: Vec<&str> = G_STR
        .into_iter()
        .filter(|t| pre_small_const(t) != post_small(t))
        .collect();
    assert_eq!(
        differ,
        vec!["-0"],
        "EQ5 A07: the G-STR texts where the two part"
    );
    assert!(!int_decimal("-0"), "EQ5 A07: `-0` is no IntDecimal text");
    same("EQ5 A09", G_STR, pre_fold_i32_decimal, |raw| {
        decimal_i64(raw.trim()).and_then(|v| i32::try_from(v).ok())
    });
    same("EQ5 A10", G_STR, pre_ast_decimal, |raw| {
        decimal_i64(raw.trim())
    });
    same("EQ5 A11", G_STR, pre_da_decimal, decimal_i64);
    same("EQ5 A12", G_STR, pre_literal_size, |size_part| {
        let explicit_width: Option<u32> = if size_part.is_empty() {
            None
        } else {
            Some(decimal_i64(size_part).and_then(|v| u32::try_from(v).ok())?)
        };
        if explicit_width == Some(0) {
            return None;
        }
        Some(explicit_width)
    });
}

#[test]
fn eq6_wraps() {
    // B05: w 1..=64, pat < 2^w (based_lit_pattern masks to w and declines truncation).
    let mut g = Vec::new();
    for w in 1u32..=64 {
        let full: u64 = if w == 64 { u64::MAX } else { (1u64 << w) - 1 };
        let half: u64 = 1u64 << (w - 1);
        for pat in [0, 1, half - 1, half, full - 1, full, 5 & full] {
            for signed in [false, true] {
                g.push((pat, w, signed));
            }
        }
    }
    let post_b05 = |(pat, w, signed): (u64, u32, bool)| wrap_i128(pat as i128, w, signed);
    same(
        "EQ6 B05",
        g,
        |(pat, w, signed)| pre_based_param_tail(pat, w, signed),
        post_b05,
    );
    let out: Vec<(u64, u32, bool)> = (1u32..=63)
        .flat_map(|w| [false, true].map(move |s| (1u64 << w, w, s)))
        .collect();
    assert_eq!(
        first_diff(
            out,
            |(pat, w, signed): (u64, u32, bool)| pre_based_param_tail(pat, w, signed),
            post_b05
        ),
        Some((2, 1, false)),
        "EQ6 B05: the first out-of-domain input (pat = 2^w)"
    );
    // B06 / B07: every (v, w, signed) of G-WRAP.
    let mut g = Vec::new();
    for w in G_W {
        for v in wrap_vals(w) {
            for signed in [false, true] {
                g.push((v, w, signed));
            }
        }
    }
    same(
        "EQ6 B06",
        g.clone(),
        |(v, w, s)| pre_coerce_i64_to_width(v, w, s),
        |(v, w, s)| wrap_i128(i128::from(v), w, s).unwrap_or(v),
    );
    same(
        "EQ6 B07",
        g,
        |(v, w, s)| pre_coerce_int_width(v, w, s),
        |(v, w, s)| wrap_i128(i128::from(v), w, s).unwrap_or(v),
    );
    // B10 / B12 / B13: w 1..=64 over u64 patterns.
    let pats = |w: u32| -> Vec<u64> {
        let mut v = vec![0u64, 1, u64::MAX, 1 << 63, (1 << 63) - 1, 5];
        if w < 64 {
            v.extend([1u64 << (w - 1), (1u64 << w) - 1, 1u64 << w]);
        }
        v
    };
    let g: Vec<(u64, u32)> = (1u32..=64)
        .flat_map(|w| pats(w).into_iter().map(move |p| (p, w)))
        .collect();
    let post_b10 = |(v, w): (u64, u32)| wrap_i128(i128::from(v), w, true).unwrap_or(v as i64);
    same("EQ6 B10", g.clone(), |(v, w)| pre_sext(v, w), post_b10);
    if cfg!(debug_assertions) {
        assert_eq!(
            first_diff([(5u64, 0u32), (5, 65)], |(v, w)| pre_sext(v, w), post_b10),
            Some((5, 0)),
            "EQ6 B10: w = 0 (PRE shifts by 64: a dev panic) is the first out-of-domain input"
        );
    }
    same(
        "EQ6 B12",
        g.clone(),
        |(raw, w)| pre_index_sign_extended(raw, w),
        |(raw, w)| {
            if w == 0 || w > 64 {
                return None;
            }
            Some(wrap_i128(i128::from(raw), w, true)? as u32)
        },
    );
    let g13: Vec<(u64, u32, bool)> = (0u32..=64)
        .flat_map(|w| {
            pats(w.max(1))
                .into_iter()
                .flat_map(move |p| [false, true].map(move |s| (p, w, s)))
        })
        .collect();
    same(
        "EQ6 B13",
        g13,
        |(raw, width, s)| pre_index_value(raw, width, s),
        |(raw, width, s)| {
            let w = width.max(1);
            Some(wrap_i128(i128::from(raw), w, s).unwrap_or(i64::MAX))
        },
    );
}

#[test]
fn eq7_k1_k3() {
    // K1 over every operator but `**` (its arm is a debug_assert: every site folds `**`
    // through const_pow).
    let g = ALL
        .iter()
        .filter(|op| **op != BinOp::Pow)
        .flat_map(|op| int_pairs().into_iter().map(move |(a, b)| (*op, a, b)))
        .chain(
            [BinOp::Shl, BinOp::AShl, BinOp::Shr, BinOp::AShr]
                .into_iter()
                .flat_map(|op| {
                    G_INT
                        .into_iter()
                        .flat_map(move |a| G_SH.map(move |s| (op, a, s)))
                }),
        );
    same(
        "EQ7 K1",
        g,
        |(op, a, b)| pre_const_binop(op, a, b),
        |(op, a, b)| const_binop(op, a, b),
    );
    let g = G_INT.into_iter().flat_map(|a| G_SH.map(move |s| (a, s)));
    same(
        "EQ7 K2",
        g.chain(int_pairs()),
        |(a, b)| pre_const_shl_i64(a, b),
        |(a, b)| const_shl_i64(a, b),
    );
    let g = int_pairs()
        .into_iter()
        .chain(G_INT.into_iter().flat_map(|a| G_SH.map(move |s| (a, s))))
        .flat_map(|(a, b)| [false, true].map(move |s| (a, b, s)));
    same(
        "EQ7 K3",
        g,
        |(a, b, s)| pre_const_pow(a, b, s),
        |(a, b, s)| const_pow(a, b, s),
    );
}

// ───────────── fix round 1: the kernels whose bodies changed for cost ─────────────
//
// PARSE-1 fix round 1 rewrote two kernel bodies for per-use cost (K7 `wrap_i128` in u64,
// K4 `binop_in` passing each admitted operator as a constant). These rows compare the
// new bodies against every PRE spelling they serve, over dense grids: every width
// 0..=64 plus 65, 128 and u32::MAX, both signs, the edge values and 10^4 seeded random
// values.

/// The parser's PRE kernel, verbatim: hdl-parser expr_primary.rs:5-25 at bea52457 (the
/// `wrap_i128` K7 was moved from; its callers there are the cw_eval leaf, based-literal
/// and parameter sites).
fn pre_parser_wrap_i128(v: i128, w: u32, signed: bool) -> Option<i64> {
    if w == 0 || w > 64 {
        return None;
    }
    let mask: u128 = if w == 64 {
        u64::MAX as u128
    } else {
        (1u128 << w) - 1
    };
    let bits = (v as u128) & mask;
    if signed && w < 64 && (bits >> (w - 1)) & 1 == 1 {
        Some((bits | (!0u128 << w)) as i64)
    } else if !signed && w == 64 && bits > i64::MAX as u128 {
        None
    } else {
        Some(bits as i64)
    }
}

/// A seeded xorshift64 stream (the grid is the same on every run).
fn rand_u64(n: usize, mut s: u64) -> Vec<u64> {
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        })
        .collect()
}

/// Every width 0..=64, then 65, 128 and u32::MAX.
fn dense_widths() -> Vec<u32> {
    (0u32..=64).chain([65, 128, u32::MAX]).collect()
}

/// The i64 edge values at width `w`: 0, +-1, i64::MIN, i64::MAX, 2^(w-1) +-1,
/// -(2^(w-1)) +-1 and 2^w +-1 where an i64 holds them, and the shared random values.
fn dense_vals(w: u32, rnd: &[u64]) -> Vec<i64> {
    let mut v: Vec<i128> = vec![0, 1, -1, i128::from(i64::MIN), i128::from(i64::MAX)];
    for k in [w.checked_sub(1), Some(w)].into_iter().flatten() {
        if k < 127 {
            let p = 1i128 << k;
            v.extend([p - 1, p, p + 1, -p - 1, -p, -p + 1]);
        }
    }
    let mut out: Vec<i64> = v
        .into_iter()
        .filter_map(|x| i64::try_from(x).ok())
        .collect();
    out.extend(rnd.iter().map(|r| *r as i64));
    out
}

#[test]
fn eq6b_wrap_dense() {
    let rnd = rand_u64(10_000, 0x9E37_79B9_7F4A_7C15);
    let mut n = 0;
    // The kernel itself against the parser's PRE kernel, over i128 values wider than 64 bits
    // too (the low 64 bits are all either body reads).
    let mut wide: Vec<i128> = vec![
        i128::MIN,
        i128::MAX,
        1i128 << 64,
        (1i128 << 64) - 1,
        (1i128 << 64) + 1,
    ];
    wide.extend(
        rnd.iter()
            .zip(rnd.iter().rev())
            .map(|(h, l)| ((u128::from(*h) << 64) | u128::from(*l)) as i128),
    );
    for w in dense_widths() {
        let vals: Vec<i128> = dense_vals(w, &rnd)
            .into_iter()
            .map(i128::from)
            .chain(wide.iter().copied())
            .collect();
        for s in [false, true] {
            n += same(
                "EQ6b K7",
                vals.iter().map(|v| (*v, w, s)),
                |(v, w, s)| pre_parser_wrap_i128(v, w, s),
                |(v, w, s)| wrap_i128(v, w, s),
            );
            // B06 / B07: every (v, w, signed).
            let g: Vec<(i64, u32, bool)> =
                dense_vals(w, &rnd).into_iter().map(|v| (v, w, s)).collect();
            n += same(
                "EQ6b B06",
                g.iter().copied(),
                |(v, w, s)| pre_coerce_i64_to_width(v, w, s),
                |(v, w, s)| wrap_i128(i128::from(v), w, s).unwrap_or(v),
            );
            n += same(
                "EQ6b B07",
                g.iter().copied(),
                |(v, w, s)| pre_coerce_int_width(v, w, s),
                |(v, w, s)| wrap_i128(i128::from(v), w, s).unwrap_or(v),
            );
            // B05 on its domain: w 1..=64, pat < 2^w (the pattern masked to w).
            if (1..=64).contains(&w) {
                let m = if w == 64 { u64::MAX } else { (1u64 << w) - 1 };
                n += same(
                    "EQ6b B05",
                    g.iter().map(|(v, w, s)| ((*v as u64) & m, *w, *s)),
                    |(pat, w, s)| pre_based_param_tail(pat, w, s),
                    |(pat, w, s)| wrap_i128(pat as i128, w, s),
                );
            }
            // B13: width 0..=64 (its caller refuses > 64 first).
            if w <= 64 {
                n += same(
                    "EQ6b B13",
                    g.iter().map(|(v, w, s)| (*v as u64, *w, *s)),
                    |(raw, width, s)| pre_index_value(raw, width, s),
                    |(raw, width, s)| {
                        let w = width.max(1);
                        Some(wrap_i128(i128::from(raw), w, s).unwrap_or(i64::MAX))
                    },
                );
            }
        }
        // B10 (w 1..=64: its producer's guard ; PRE panics at w = 0 in dev, EQ6 names it) and
        // B12 (every w: its own guard first).
        let g: Vec<(u64, u32)> = dense_vals(w, &rnd)
            .into_iter()
            .map(|v| (v as u64, w))
            .collect();
        if (1..=64).contains(&w) {
            n += same(
                "EQ6b B10",
                g.iter().copied(),
                |(v, w)| pre_sext(v, w),
                |(v, w)| wrap_i128(i128::from(v), w, true).unwrap_or(v as i64),
            );
        }
        n += same(
            "EQ6b B12",
            g.iter().copied(),
            |(raw, w)| pre_index_sign_extended(raw, w),
            |(raw, w)| {
                if w == 0 || w > 64 {
                    return None;
                }
                Some(wrap_i128(i128::from(raw), w, true)? as u32)
            },
        );
    }
    assert!(n > 4_000_000, "EQ6b: the grid shrank ({n})");
}

#[test]
fn eq8_binop_in_dense() {
    // K4 against both PRE parser spellings, over every operator and G-INT x G-INT plus
    // 10^4 seeded random pairs (each operand also tried at small magnitudes, so the
    // checked arms meet both their value and their overflow lanes).
    let rnd = rand_u64(20_000, 0xD1B5_4A32_D192_ED03);
    let mut pairs = int_pairs();
    for c in rnd.chunks(2) {
        let (a, b) = (c[0] as i64, c[1] as i64);
        pairs.extend([(a, b), (a >> 32, b >> 32), (a >> 60, b >> 60), (a, b >> 62)]);
    }
    let mut n = 0;
    for op in ALL {
        n += same(
            "EQ8 Parser4",
            pairs.iter().map(|(a, b)| (op, *a, *b)),
            |(op, a, b)| pre_try_const_index_binary(&op, a, b),
            |(op, a, b)| binop_in(OpSet::Parser4, op, a, b),
        );
        n += same(
            "EQ8 Lit3",
            pairs.iter().map(|(a, b)| (op, *a, *b)),
            |(op, a, b)| pre_const_lit_binary(&op, a, b),
            |(op, a, b)| binop_in(OpSet::Lit3, op, a, b),
        );
    }
    assert!(n > 2_000_000, "EQ8: the grid shrank ({n})");
}
