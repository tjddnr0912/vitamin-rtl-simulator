//! IEEE 1800-2017 Table 11-21: the EXPONENT of `**` is SELF-DETERMINED.
//!
//! The generic `arith` kernel read both operands under ONE collective sign
//! (§11.8.1), and `**` is the single arithmetic operator that breaks that
//! contract. The engine used to reconcile the two by restamping the exponent
//! with the BASE's sign, which reinterpreted its bits whenever it had no spare
//! high bit: `4'sd3 ** 4'd11` read the exponent 11 as -5 and IEEE Table 11-6
//! answered 0. A 3,584-cell base-sign x exp-sign x exp-width x all-values sweep
//! measured **336 wrong cells**, all silent (exit 0).
//!
//! The fix splits that one boolean into the three questions it was standing in
//! for — how to read the LEFT operand, how to read the RIGHT one, and what the
//! RESULT's sign is. For every other operator the three coincide, so the split is
//! mechanically identity there.
//!
//! Every expected value below is pinned to BOTH oracles (iverilog 13.0 and
//! verilator 5.050) unless the test says otherwise; the two cells where they
//! disagree are pinned to IEEE Table 11-6 and say which oracle that follows.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design and return stdout minus the trailer. A unique per-test dir keeps
/// the parallel runners off each other's files.
fn run_args(src: &str, args: &[&str]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_powsd_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_vita"));
    for a in args {
        c.arg(a);
    }
    let out = c
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let r = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn run(src: &str) -> String {
    run_args(src, &[])
}

/// The headline: a SIGNED base with an UNSIGNED exponent whose top bit is set.
/// `3 ** 11 = 177147`, and 177147 mod 16 = 11 = `1011`. Reading 11 as -5 makes
/// Table 11-6 return 0 for |base| > 1 — the shape that was silently wrong.
#[test]
fn an_unsigned_exponent_is_never_reinterpreted_as_negative() {
    let out = run(r#"module t;
  logic signed [3:0] s3;
  logic        [3:0] u11;
  initial begin
    s3 = 4'sd3; u11 = 4'd11;
    $display("%b %b %0d", s3 ** u11, s3 ** 4'd11, $signed(s3 ** u11));
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 1011 1011 -5   (PRE: 0000 0000 0)
    assert!(
        out.contains("1011 1011 -5"),
        "unsigned exponent misread as negative: {out}"
    );
}

/// The mirror: an UNSIGNED base with a SIGNED NEGATIVE exponent. The exponent
/// must stay negative so Table 11-6 fires (|13| > 1 => 0); demoting it to
/// unsigned read -2 as 14 and computed 13^14 mod 16 = 9.
#[test]
fn a_signed_negative_exponent_survives_an_unsigned_base() {
    let out = run(r#"module t;
  logic        [3:0] u13;
  logic signed [3:0] sm2;
  initial begin
    u13 = 4'd13; sm2 = -4'sd2;
    $display("%b", u13 ** sm2);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 0000   (PRE: 1001)
    assert!(out.contains("0000"), "negative exponent demoted: {out}");
}

/// Table 11-6 keys the negative-exponent rules on the base's VALUE. A signed
/// -1 base alternates by parity; an UNSIGNED all-ones base is 15, not -1, so it
/// takes the |base| > 1 row. ORACLES SPLIT on the second line: iverilog reads
/// the unsigned all-ones as -1 (`0001`), verilator and this pin read it as 15.
/// A base of 0 with a negative exponent is `x` — there iverilog is the one that
/// matches the LRM and verilator prints `0000`. ⚠️ That last call is WIDTH-DEPENDENT:
/// iverilog only follows Table 11-6 up to width 32, and collapses every negative
/// exponent to 0 from width 33 up (see `the_wide_lane_reads_the_base_sign_for_table_11_6`).
#[test]
fn table_11_6_reads_the_base_under_its_own_sign() {
    let out = run(r#"module t;
  logic signed [3:0] sm1;
  logic        [3:0] u15, u0;
  logic signed [3:0] sm2;
  initial begin
    sm1 = -4'sd1; u15 = 4'd15; u0 = 4'd0; sm2 = -4'sd2;
    $display("%b %b %b", sm1 ** sm2, u15 ** sm2, u0 ** sm2);
    #1 $finish;
  end
endmodule
"#);
    // (-1)**-2 = 1 [both oracles] | 15**-2 = 0 [verilator; iverilog says 0001]
    // | 0**-2 = x [iverilog; verilator says 0000]
    assert!(
        out.contains("0001 0000 xxxx"),
        "Table 11-6 base rows: {out}"
    );
}

/// A SAME-sign pair must be byte-identical to the pre-slice engine — splitting one
/// collective sign into three collapses back to the original boolean when the two
/// operands agree. `2 ** 1'b1` = 2 (a 1-bit unsigned exponent is +1, never -1) and
/// `a ** 18` must not truncate the exponent to its 4-bit result width
/// (18 mod 16 = 2 would give 4 instead of 2^18 mod 16 = 0). Both rows pass on the
/// PRE binary too — that is the point of them.
#[test]
fn a_same_sign_pair_keeps_its_exponent_value_exactly() {
    let out = run(r#"module t;
  logic [3:0] a, r1, r2;
  logic signed [3:0] s2;
  logic signed [7:0] e18;
  initial begin
    a = 4'd2; s2 = 4'sd2; e18 = 8'sd18;
    r1 = a ** 1'b1;
    r2 = a ** e18;
    $display("%0d %0d %0d", r1, r2, s2 ** 4'sd3);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 2 0 -8   (2**18 = 262144, mod 16 = 0; 2**3 = 8 as signed [3:0] = -8)
    assert!(out.contains("2 0 -8"), "same-sign pair regressed: {out}");
}

/// A mixed-sign `**` picks its KERNEL by whether the exponent can be negative, so
/// these rows straddle the i128 lane and the multi-word one: `(2^63+3) ** 2` keeps
/// only the low 64 bits (= 9) on the narrow lane, and a 68-bit operand goes wide.
/// `arith_wide` reads each side under its own sign now, so both must still agree
/// with the oracles.
#[test]
fn the_mixed_sign_widening_agrees_across_the_arithmetic_lanes() {
    let out = run(r#"module t;
  logic        [63:0] u64hi;
  logic        [67:0] w68;
  logic signed [63:0] s64;
  initial begin
    u64hi = 64'h8000_0000_0000_0003; w68 = 68'd3; s64 = -64'sd3;
    $display("%0h %0h %0h", u64hi ** 2, w68 ** 3, s64 ** 3);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 9 1b ffffffffffffffe5
    assert!(
        out.contains("9 1b ffffffffffffffe5"),
        "wide/mixed lane disagreement: {out}"
    );
}

/// `N'(base ** exp)` narrows through the width-probe branch, where `**` now
/// sits (a negative exponent asks whether the base is +/-1 — a question about
/// the WHOLE base, so the low-bit-closure argument that puts `<<` on the other
/// branch does not cover it). §4.5.318 measured that move as a net loss while
/// the exponent's sign was still taken from the base; with that fixed it is a
/// pure gain — the cast sweep goes 0 regressions / 72 fixed.
#[test]
fn a_size_cast_narrows_a_power_through_the_width_probe() {
    let out = run(r#"module t;
  logic signed [3:0] b;
  logic signed [3:0] e;
  logic [4:0] u5;
  initial begin
    b = 4'sd3; e = -4'sd5;
    u5 = 5'd9;
    $display("%b %b %b", 2'(b ** e), 3'(b ** e), 2'(u5 + (b ** e)));
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 00 000 01   (3 ** -5 = 0; 9 + 0 = 9, low 2 bits = 01)
    assert!(out.contains("00 000 01"), "size-cast power branch: {out}");
}

/// The three backends must not disagree — `native_eval` lowers a small CONST
/// exponent to a Mul chain and bails on a signed base, so it is a second
/// spelling of the same operator that could drift.
#[test]
fn every_backend_agrees_on_the_operand_signs() {
    let src = r#"module t;
  logic signed [3:0] s3;
  logic        [3:0] u11, u13;
  logic signed [3:0] sm2;
  initial begin
    s3 = 4'sd3; u11 = 4'd11; u13 = 4'd13; sm2 = -4'sd2;
    $display("%b %b %b", s3 ** u11, u13 ** sm2, u13 ** 3);
    #1 $finish;
  end
endmodule
"#;
    let base = run_args(src, &["--backend", "interp"]);
    for be in ["bytecode", "native"] {
        let other = run_args(src, &["--backend", be]);
        assert_eq!(base, other, "backend {be} diverged");
    }
    assert!(base.contains("1011 0000 0101"), "interp baseline: {base}");
}

/// `arith` stamps a `**` result with the BASE's sign (§11.4.10), not the pair's.
/// This row exists because an adversarial mutation deleting that rule survived the
/// rest of this file: every other mixed-sign row here has a SIGNED base (where the
/// stamp is a no-op) or a `0000`/`xxxx` result (where `%b` cannot show a sign). An
/// UNSIGNED base with a signed exponent is the only shape that observes it — 13
/// printed as `-3` is the mutant.
#[test]
fn an_unsigned_base_keeps_an_unsigned_result() {
    let out = run(r#"module t;
  logic        [3:0] u13;
  logic signed [3:0] e1, e2;
  logic signed [7:0] w1;
  initial begin
    u13 = 4'd13; e1 = 4'sd1; e2 = 4'sd2;
    w1 = u13 ** e1;
    $display("%0d %0d %0d", u13 ** e1, u13 ** e2, w1);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 13 9 13   (a signed result would widen 13 into -3)
    assert!(
        out.contains("13 9 13"),
        "result sign follows the base: {out}"
    );
}

/// A `**` at the WIDEST legal net width must not be pushed over `WIDE_ARITH_CAP`
/// by anything this evaluator does internally. An earlier cut of this slice widened
/// a mixed-sign pair by one bit unconditionally; at `MAX_NET_WIDTH` that landed one
/// bit past the cap, where `arith_wide` X-poisons — and the loud `W-RUN-WIDE-ARITH`
/// gate keys on the DECLARED width, so it could not fire. correct -> silently `x`,
/// exit 0. The exponent here is an unsized decimal literal, i.e. SIGNED, so the pair
/// really is mixed.
#[test]
fn the_widest_legal_operand_does_not_cross_the_wide_arith_cap() {
    let out = run(r#"module t;
  logic [1048575:0] big, res;
  initial begin
    big = 3;
    res = big ** 0;
    $display("%b", res[3:0]);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 0001   (the +1 widening printed xxxx)
    assert!(out.contains("0001"), "cap boundary poisoned: {out}");
}

/// A `string` base cannot be resized to the context width — `resize_keep_sign`
/// returns it at its own width — so `base.width == w` is NOT an invariant and the
/// exponent's widen to `max(w, own width)` is what keeps the arithmetic at the
/// CONTEXT width. Dropping that widen computed `"ab" ** 3` at 32 bits instead of 64.
/// No external oracle: iverilog aborts on `string ** int` and verilator raises an
/// internal error, so this is pinned to `24930 ** 3` computed by hand (and to vita's
/// own pre-slice answer).
#[test]
fn a_string_base_still_powers_at_the_context_width() {
    let out = run(r#"module t;
  string s;
  logic [63:0] a, b;
  initial begin
    s = "ab";
    a = s ** 3;
    b = s ** 32'd3;
    $display("%0d %0d", a, b);
    #1 $finish;
  end
endmodule
"#);
    // 24930^3 = 15494117157000, and the two spellings must not disagree.
    assert!(
        out.contains("15494117157000 15494117157000"),
        "string base lost the context width: {out}"
    );
}

/// IEEE Table 11-21 puts a power's exponent in the same row as a shift's amount, so
/// `lower_expr_ctx`'s "right operand self-determined" list must name `Pow`. It did
/// not, and the assignment context leaked into the exponent: `q ** '1` sized the
/// fill to the lvalue's 8 bits and computed `2 ** 255` instead of `2 ** 1`.
#[test]
fn an_assignment_context_does_not_reach_the_exponent() {
    let out = run(r#"module t;
  logic [3:0] q;
  logic [7:0] r8;
  initial begin
    q = 4'd2;
    r8 = q ** '1;
    $display("%0d", r8);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: 2   (PRE: 0 — the fill became 8 ones). `contains` would be
    // near-vacuous here: 94 of the 256 values an 8-bit result can print contain a
    // '2', so this pins the whole line.
    assert_eq!(out.trim(), "2", "context leaked into the exponent");
}

/// The DEFAULT backend has a second spelling of `**`: `native_eval` lowers a small
/// CONST exponent to a Mul chain. `const_pow_exponent` read that constant with
/// `to_u128`, which masks to the constant's own WIDTH — so `4'sb1110` (-2) came back
/// as 14 and compiled a 14-step chain. Before this slice all three backends were
/// wrong together; fixing the generic evaluator alone would have left the DEFAULT
/// backend silently disagreeing with the reference interpreter. The process must be
/// clocked: an `initial` block is not codegen-able, so the Mul-chain lane never runs
/// there and the shape looks fine.
#[test]
fn a_narrow_negative_const_exponent_is_not_read_as_positive() {
    let src = r#"module t;
  logic clk = 0;
  logic [3:0] u13 = 4'd13, u7 = 4'd7;
  logic [3:0] r1, r2, r3, r4, r5;
  always @(posedge clk) begin
    r1 <= u13 ** 4'sb1110;
    r2 <= u13 ** 4'sb1111;
    r3 <= u13 ** 3'sb111;
    r4 <= u7  ** 2'sb11;
    r5 <= u13 ** 5'sb10000;
  end
  initial begin
    #1 clk = 1; #1 clk = 0; #1;
    $display("%b %b %b %b %b", r1, r2, r3, r4, r5);
    #1 $finish;
  end
endmodule
"#;
    // both oracles: every one is 0 (Table 11-6, |base| > 1 and a negative exponent)
    let want = "0000 0000 0000 0000 0000";
    let base = run_args(src, &["--backend", "interp"]);
    assert!(base.contains(want), "interp: {base}");
    for be in ["bytecode", "native"] {
        let other = run_args(src, &["--backend", be]);
        assert_eq!(base, other, "backend {be} diverged");
    }
}

/// Anti-vacuity for "the result is signed iff the BASE is". A mutation replacing
/// that rule with the pair's collective sign survived the whole suite, because the
/// only shape that observes it is a SIGNED base with an UNSIGNED exponent rendered
/// as a decimal — an unsigned base cannot tell `l.signed` from `l.signed &&
/// r.signed`, and `%b` never shows a sign at all.
#[test]
fn a_signed_base_keeps_a_signed_result_under_an_unsigned_exponent() {
    let out = run(r#"module t;
  logic signed [3:0] s3;
  logic        [3:0] u1;
  logic signed [7:0] w;
  initial begin
    s3 = -4'sd3; u1 = 4'd1;
    w = s3 ** u1;
    $display("%0d %0d %0d", s3 ** u1, w, s3 ** u1 + 0);
    #1 $finish;
  end
endmodule
"#);
    // both oracles: -3 -3 -3   (an unsigned result would render 13)
    assert!(out.contains("-3 -3 -3"), "result sign lost: {out}");
}

/// The wide (multi-word) lane has its own copy of Table 11-6's negative-exponent
/// rows, and mutations of its sign plumbing survived the suite because every wide
/// row above uses a NON-negative exponent. `sg.lhs` gates the -1 row, so an UNSIGNED
/// all-ones base takes the |base| > 1 row (0) while a SIGNED one is genuinely -1 and
/// alternates by parity.
///
/// This row kills two of those mutations (dropping the `sg.lhs` gate; reading `sb`
/// off the LEFT sign). A third — passing `sg.lhs` to the right operand's
/// `resize_keep_sign` — is EQUIVALENT, not untested: at this point `r.width == w`
/// (the caller pre-extends the exponent to `max(exp.width, w)`), so that call only
/// restamps a flag, and nothing downstream reads `re.signed` — only `re.val` and
/// `re.get_vu(w-1)`. Don't go looking for a killer; there isn't one.
///
/// ORACLE NOTE: iverilog 13.0 diverges here in TWO independent ways, and the two
/// fields below sit on different ones.
///   (a) It reads an UNSIGNED all-ones base as -1. Width-independent — it does this
///       at 4 bits too. So field 1 is pinned to verilator + IEEE (op1's VALUE is
///       2^68-1, not -1), where iverilog prints all-ones.
///   (b) Its wide `**` collapses EVERY negative exponent to 0 from width 33 up.
///       Measured at the exact boundary: `(-1) ** -2` is 1 at widths 16/31/32 and 0
///       at 33/34/40/64/68/100. So field 2 is pinned to verilator + IEEE, where
///       iverilog prints 0.
/// Field 3 is a plain |base| > 1 row and both oracles agree on it.
#[test]
fn the_wide_lane_reads_the_base_sign_for_table_11_6() {
    let out = run(r#"module t;
  logic        [67:0] uall;
  logic signed [67:0] sall;
  logic        [99:0] u100;
  logic signed [3:0] e3;
  logic signed [7:0] e2;
  initial begin
    uall = {68{1'b1}}; sall = {68{1'b1}}; u100 = 100'd13;
    e3 = -4'sd3; e2 = -8'sd2;
    $display("%h %h %h", uall ** e3, sall ** e3, u100 ** e2);
    #1 $finish;
  end
endmodule
"#);
    // unsigned all-ones is 2^68-1, not -1  -> 0            [both oracles]
    // signed all-ones IS -1, odd exponent  -> -1           [verilator; iverilog 0]
    // 13 ** -2                             -> 0            [both oracles]
    assert!(
        out.contains("00000000000000000 fffffffffffffffff 0000000000000000000000000"),
        "wide Table 11-6 rows: {out}"
    );
}
