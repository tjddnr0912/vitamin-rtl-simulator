//! value_rules -- the 64-bit operator / literal / width kernels of PARSE-1 (+ OPS) (owner 2026-11-09, O2).
//!
//! History (ROADMAP §5.2 PARSE-1): elaborate's constant-domain kernels (`const_binop`,
//! `const_shl_i64`, `const_pow`, from `const_fn.rs`) and the parser's spellings of the same rules
//! (its op-set arms, its context-width arm, its `$clog2` formula, its decimal read and its
//! sign extension `wrap_i128`, from `expr_primary.rs`) were collected into this one file, where
//! the parser and elaborate both reach them; the parser's parse-time fold itself (the fns that
//! read its constant tables) went to hdl-parser `const_fold.rs` ("an evaluator stays where its
//! state is"). STRUCTURE ONLY: each kernel is the text of the spellings it replaced; no answer
//! changed. Each caller keeps its own operand order, its `$clog2` `n <= 0` handling and its
//! cutoffs (an i32 / u32 `try_from`, a trim, a zero-width test) at its site.
//!
//! Lanes L1-L15 are named answers of their own, never merged (L1 Full = [`const_binop`]; L2
//! Parser4 and L3 Lit3 = [`OpSet`] through [`binop_in`]; L4 the parser's context-width arm =
//! [`ctx5_binop`], masked once by its caller; L5-L8 the four `$clog2` readers over
//! [`clog2_u64`]; L9 the decimal read [`decimal_i64`] with each site's cutoff; L10-L15 the
//! literal readers, bit-container reads, the elaboration-task renderer, the cast and the type
//! ranges that stay at their sites). O1-O15 are outside by rule (another function, domain,
//! width, phase or crate edge), each with its differing input (docs/roadmap/5.2/PARSE-1.md).
//!
//! ENFORCED by test -- exactly these token shapes, counted per (file, fn), each exception named
//! with its reason:
//!   elaborate/tests/value_rules_home.rs: T1 K1's arms `=> a.checked_{add,sub,mul,div,rem}(b)`
//!     and `=> Some((a <cmp> b) as i64)` (S3: 3 keys, 8) ; T2 the bit-length identifiers (every
//!     crate: 11 keys, 13) ; T2b `clog2`-named fns (6) ; T3 the literal-read tokens (S3: 9 keys,
//!     17) ; T4 one definition of each kernel, here ; T5 the 21 folds only in hdl-parser
//!     const_fold.rs ; T6 constant-table reads only there (2 named) ; T7 the sign-extension
//!     tokens (S3: 4 keys, 4) ; T7b bare masks (S3: 24 keys, 28, each with its edge convention).
//!   hdl-ast/tests/value_rules_equality.rs: each re-pointed spelling's PRE text against its
//!     kernel call over the site's domain (EQ1-EQ7).
//!   hdl-ast/tests/value_rules_lanes.rs: the op sets, ctx5_binop unmasked, the clog2_u64 /
//!     decimal_i64 / wrap_i128 edges.
//!
//! REVIEW-ONLY -- no token test sees these ; a change to one is a reviewed edit and re-runs the
//! PARSE-1 class census (tool, method and rows: docs/roadmap/5.2/PARSE-1.md). The census and
//! its rows are complete AT the tree it was run on (its method, its 123 controls and the
//! premise round 3 absence scans), not complete by method in general: its measured blind
//! spots are op variants reached through a type alias, a cross-file `pub use .. as`, a braced
//! glob `use E::{BinOp::*, ..}` or `Self::` in `impl .. for path::BinOp` ; raw-identifier
//! variants, const patterns and macro_rules arms ; `a.add(b)` ; i128 products / sums then
//! `try_from` ; shift pairs through other names, `i64::from(v as i32)`, a narrowing `let` ; a
//! ceiling log by a loop over `1 << k` or `p *= 2` ; shift-add digit accumulation ;
//! string-keyed op arms ; closure arm bodies with 2+ params, generic-argument commas and a
//! computing scrutinee. None of these occurred when the census ran ; a future one is
//! REVIEW-ONLY, never a pin's claim.
//!   R1 operator arms in general -- any arm computing from operands other than T1's two
//!      shapes, at any layout, through any path, alias or glob: 451 census arms (here 21 ;
//!      re-pointed calls 13 ; lane L4 1 ; OUT 167: O1 14, O2 19, O6 24, O9 94, O14 8, O15 8 ;
//!      tests 5 ; non-computing 244).
//!   R2 values computed before a match and selected by an arm (lowered_truth's `ord`, O14 ;
//!      the census's pre-computed values).
//!   R3 extensions not spelled with T7's tokens: bit loops, IR-node builders (O12, 8 fns),
//!      wide / 4-state extensions (O4, 3 fns).
//!   R4 a bit length computed without a T2 token (a shift loop: the engine's pow loop,
//!      binops.rs:207, O4 ; none in S3).
//!   R5 a literal read without a T3 token (a char table) -- none at the census.
//!   R6 arms keyed on a token or char that compute (50 census arms, none an operator fold).
//!   R7 a kernel computed by a helper with another name, or through an op enum or re-export
//!      the census does not list, or in one of the blind-spot spellings named above.
//!   R8 a constant-table read T6 does not see: hdl-parser module_items.rs:872
//!      `self.const_locals.iter()`, the package-export copy into `pkg_const_scoped` (the twin of
//!      O7, imports.rs).

// The kernels below moved from elaborate `const_fn.rs` with their text; `ast::` is this
// crate, so each body reads as it did there.
use crate as ast;
use crate::BinOp;

/// `**` in the i64 const domain. Negative exponents follow the IEEE integer
/// table (1**n=1, (-1)**n=±1, 0**neg undefined → None, else 0); overflow → None.
#[inline]
pub fn const_pow(a: i64, b: i64, exp_signed: bool) -> Option<i64> {
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
#[inline]
pub fn const_shl_i64(a: i64, b: i64) -> Option<i64> {
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
#[inline]
pub fn const_binop(op: ast::BinOp, a: i64, b: i64) -> Option<i64> {
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
        ast::BinOp::Shl | ast::BinOp::AShl => const_shl_i64(a, b),
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

/// The parser's two restricted op sets over [`const_binop`]'s arms (L2, L3). Neither admits
/// `**` (K1's Pow arm is a `debug_assert`): a set that admits more is a value row's edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpSet {
    /// `try_const_index`'s `+ - * /` (hdl-parser `const_fold.rs`, L2).
    Parser4,
    /// `const_lit`'s `+ - *` (hdl-parser `const_fold.rs`, L3).
    Lit3,
}

impl OpSet {
    /// Whether this set folds `op` at all.
    #[inline]
    pub fn admits(self, op: BinOp) -> bool {
        match self {
            OpSet::Parser4 => matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div),
            OpSet::Lit3 => matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul),
        }
    }
}

/// [`const_binop`] restricted to `set`: `None` for an operator outside it.
#[inline]
pub fn binop_in(set: OpSet, op: BinOp, a: i64, b: i64) -> Option<i64> {
    // Each admitted operator reaches `const_binop` as a constant, so the inliner folds its
    // `match` to that one arm in line, as each PRE site's own `match` was (an `op` passed
    // through left `const_binop` out of line at both parser sites: +0.09 % retired
    // instructions on the parse, PARSE-1 fix round 1). Every operator the two sets admit
    // has its arm here (the `debug_assert` holds it when a set grows); any other answers
    // `None`, as `set.admits(op)` false did.
    match op {
        BinOp::Add if set.admits(op) => const_binop(BinOp::Add, a, b),
        BinOp::Sub if set.admits(op) => const_binop(BinOp::Sub, a, b),
        BinOp::Mul if set.admits(op) => const_binop(BinOp::Mul, a, b),
        BinOp::Div if set.admits(op) => const_binop(BinOp::Div, a, b),
        _ => {
            debug_assert!(!set.admits(op), "binop_in: {op:?} admitted without an arm");
            None
        }
    }
}

/// The parser's context-width arm (L4): `x` and `y` are `w`-bit patterns (non-negative,
/// below 2^w), `+ - * / %` computed over i128 with the operands read back under `signed` for
/// `* / %`; returns the arm value before the caller's w-bit mask (the caller masks once).
/// A zero divisor and every other operator decline. Precondition `1 <= w <= 64` (its one
/// caller refuses `w == 0 || w > 64` first). `*` is plain i128 (an operand pair whose product
/// overflows i128 panics in a dev build, as it did at its PRE site).
#[inline]
pub fn ctx5_binop(op: BinOp, x: i128, y: i128, w: u32, signed: bool) -> Option<i128> {
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
        BinOp::Mul => read(x) * read(y),
        BinOp::Div if y != 0 => read(x) / read(y),
        BinOp::Mod if y != 0 => read(x) % read(y),
        _ => return None,
    };
    Some(v)
}

/// `$clog2` of a non-negative `n` (IEEE 1800-2017 §20.8.1): the number of address bits for
/// `n` items, 0 for 0 and 1. Each caller keeps its own `n <= 0` lane (L5-L8).
#[inline]
pub fn clog2_u64(n: u64) -> u32 {
    64 - n.saturating_sub(1).leading_zeros()
}

/// A decimal literal's text as an i64: digit separators removed, then Rust's integer grammar
/// (an optional sign, ASCII digits); `None` past i64. Each caller keeps its own trim and its
/// narrower cutoff (L9).
#[inline]
pub fn decimal_i64(raw: &str) -> Option<i64> {
    raw.chars()
        .filter(|c| *c != '_')
        .collect::<String>()
        .parse::<i64>()
        .ok()
}

/// Wrap a two's-complement value to `w` bits and read it back under `signed`.
/// Declines an unknown or over-64-bit width, and an unsigned 64-bit pattern no i64
/// holds.
///
/// The answer reads only the low `w <= 64` bits of `v`, so the body works in `u64`. The
/// constant-function interpreter narrows every assignment and operator result through
/// `coerce_int_width` -> here; a `u128` mask and shift made each of those uses 41
/// instructions where PRE's `u64` mask-and-fill is 14 (+1.75 % retired instructions on
/// the `cost27` loop cells, PARSE-1 fix round 1). Value-equal to that `u128` body on
/// every input (PIN7 `eq6b_wrap_dense`).
#[inline]
pub fn wrap_i128(v: i128, w: u32, signed: bool) -> Option<i64> {
    if w == 0 || w > 64 {
        return None;
    }
    let lo = v as u64;
    if w == 64 {
        return if !signed && lo > i64::MAX as u64 {
            None
        } else {
            Some(lo as i64)
        };
    }
    let mask = (1u64 << w) - 1;
    let bits = lo & mask;
    if signed && (bits >> (w - 1)) & 1 == 1 {
        Some((bits | (!0u64 << w)) as i64)
    } else {
        Some(bits as i64)
    }
}
