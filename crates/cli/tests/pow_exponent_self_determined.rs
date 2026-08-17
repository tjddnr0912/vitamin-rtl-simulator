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

// ═══════════════════════════════════════════════════════════════════════════
// The ELABORATE CONST FOLD (the sixth spelling). §4.5.319 closed five spellings
// of "the exponent is self-determined"; `const_eval_in_scope`'s width-unlimited
// i64 walk was the last: `-4'sd8` (the 4-bit pattern 1000 = −8) widened to +8,
// so a localparam folded 6561 where the RUNTIME lowering of the same text — and
// iverilog — answer 0. These anchors pin the fold through every channel that
// consumes it (localparam, generate-if, `#()` override) and the shapes that
// discriminate the self-determined walk (pattern reads, mixed-sign extension,
// concat widths). The fold is SHARED code: a native-vs-vm differential is
// structurally blind here (§5.1-e), so these absolute pins are the teeth.
// ═══════════════════════════════════════════════════════════════════════════

/// Run one design and return (stdout minus trailer, stderr) — for the pins that
/// assert a LOUD outcome rather than a value.
fn run_full(src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_powsd_f_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.contains("simulation ended"))
            .collect::<Vec<_>>()
            .join("\n"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The headline: a negative self-determined exponent in a localparam. The plain
/// fold read `-4'sd8` as +8 and bound 6561; Table 11-6 (|base| > 1, negative
/// exponent) and iverilog answer 0 — and the RUNTIME lane of the very same text
/// already answered 0, so one source produced two values (param vs runtime).
#[test]
fn a_const_fold_exponent_is_self_determined() {
    let out = run(r#"module t;
  localparam signed [15:0] P = 4'sd3 ** -4'sd8;
  logic signed [15:0] r;
  initial begin
    r = 4'sd3 ** -4'sd8;
    $display("%0d %0d", P, r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog: 0 0   (PRE: 6561 0 — the SAME text, two values)
    assert_eq!(
        out.trim(),
        "0 0",
        "const fold disagrees with the runtime lane"
    );
}

/// `~4'd0` is the 4-bit pattern 1111 = 15 UNSIGNED; the unlimited walk computed
/// `!0 = −1` instead, and Table 11-6 turned that into 0. 3^15 = 14348907, which
/// is −3477 in a signed 16-bit localparam. Kills a mutation that drops the
/// width masking from the self-determined walk (−1 and 15 differ only there).
#[test]
fn a_const_fold_bitnot_exponent_reads_the_pattern() {
    let out = run(r#"module t;
  localparam signed [15:0] P = 4'sd3 ** ~4'd0;
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#);
    // iverilog: -3477   (PRE: 0)
    assert_eq!(out.trim(), "-3477", "bitnot exponent lost its pattern");
}

/// A MIXED-sign exponent (`4'sd8 - 5'd1`) is unsigned as a whole (§11.8.1), so
/// the signed operand's 4-bit pattern 1000 ZERO-extends to 01000 = 8, and
/// 8 − 1 = 7 → 3^7 = 2187. The unlimited walk sign-extended it (−8 − 1 = −9)
/// and answered 0. Discriminates the extension rule, not just the wrap.
#[test]
fn a_const_fold_mixed_sign_exponent_zero_extends() {
    let out = run(r#"module t;
  localparam signed [15:0] P = 4'sd3 ** (4'sd8 - 5'd1);
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#);
    // iverilog: 2187   (PRE: 0)
    assert_eq!(out.trim(), "2187", "mixed-sign exponent extension wrong");
}

/// A concatenation's self width is the SUM of its parts (§11.8.1) and its value
/// is unsigned. `{2'b10,2'b01}` = 4-bit 9; 9 − 8 = 1 → 3^1 = 3. Without the
/// `Concat` arm in `const_self_width` the Pow helper's width-unknown refusal
/// turned this correctly-folding cell loud.
#[test]
fn a_const_fold_concat_exponent_width_is_the_sum() {
    let out = run(r#"module t;
  localparam signed [15:0] P = 4'sd3 ** ({2'b10, 2'b01} - 4'd8);
  localparam [31:0] Q = 4'sd3 ** ({1'b1,1'b1,1'b1,1'b1} - 2'd1);
  initial begin $display("%0d %0d", P, Q); #1 $finish; end
endmodule
"#);
    // iverilog: 3 4782969.  Q discriminates SUM from MAX: four 1-bit parts sum
    // to 4 (15 − 1 = 14 → 3^14) while their max is 1 (a 2-bit context reads 15
    // as 3 → 3^2 = 9).
    assert_eq!(out.trim(), "3 4782969", "concat width not summed");
}

/// The fold feeds GENERATE-IF conditions and the `#()` override channel too, so
/// a wrong exponent selects the wrong generate arm / installs the wrong child
/// param. Both consumers through one pin each.
#[test]
fn a_const_fold_exponent_reaches_generate_and_override() {
    let out = run(r#"module t;
  generate if ((4'sd3 ** -4'sd8) == 0) begin : g
    initial begin #1 $display("arm zero"); #1 $finish; end
  end else begin : h
    initial begin #1 $display("arm nonzero"); #1 $finish; end
  end endgenerate
endmodule
"#);
    // iverilog: arm zero   (PRE: arm nonzero — the wrong generate arm ELABORATED)
    assert!(out.contains("arm zero"), "generate arm chosen by +8: {out}");
    let out2 = run(r#"module s #(parameter signed [15:0] P = 1);
  initial begin $display("%0d", P); #1 $finish; end
endmodule
module t;
  s #(.P(4'sd3 ** -4'sd8)) u();
endmodule
"#);
    // iverilog: 0   (PRE: 6561)
    assert_eq!(
        out2.trim(),
        "0",
        "override channel folded the wide exponent"
    );
}

/// POSITIVE-exponent folds are byte-identical before/after: the self-determined
/// walk answers the same value the unlimited one did wherever no wrap and no
/// mixed-sign extension occur. The regression guard for the whole class.
#[test]
fn a_const_fold_positive_exponent_is_unchanged() {
    let out = run(r#"module t;
  localparam [15:0] Q = 8'd3 ** 4'd11;
  localparam [31:0] R = 8'd2 ** (4'd1 << 2);
  localparam [31:0] S = 8'd3 ** (4'd2 + 4'd3);
  initial begin $display("%0d %0d %0d", Q, R, S); #1 $finish; end
endmodule
"#);
    // iverilog: 46075 16 243
    assert_eq!(out.trim(), "46075 16 243", "positive exponent fold moved");
}

/// `RPS'(2)` — the Named spelling of a size cast — has a knowable width
/// (`cast_size_bits` resolves both spellings), so the exponent sizes and the
/// whole cell FOLDS: `2 − 9` wraps at 4 bits to 9 → 3^9 = 19683 = iverilog.
/// PRE silently bound 0 (the unlimited walk computed 2 − 9 = −7 → Table 11-6).
/// An earlier draft REFUSED this shape instead of sizing it; the adversarial
/// round measured the refusal as correct→silent-wrong in range-bound position
/// (the bound's decline path substitutes a default at exit 0), so the width
/// model was completed and the refusal removed.
#[test]
fn a_named_size_cast_exponent_folds_at_its_width() {
    let out = run(r#"module t;
  parameter RPS = 4;
  localparam [15:0] P = 4'sd3 ** (RPS'(2) - 4'd9);
  localparam [15:0] Q = 4'sd3 ** ((RPS'(2) - 4'd9));
  initial begin $display("%0d %0d", P, Q); #1 $finish; end
endmodule
"#);
    // iverilog: 19683 19683   (PRE: silently 0 0; the paren twin pins that a
    // wrapper does not change the sizing).
    assert_eq!(out.trim(), "19683 19683", "Named-cast exponent width lost");
}

/// A WIDTH-UNKNOWN exponent shape (a multi-packed body local is recorded with
/// width 0 = unknown) keeps the pre-slice unlimited fold — it must NOT be
/// refused: the refusal is only as loud as its caller, and in RANGE-BOUND
/// position the decline path silently substitutes a default (a 1-bit net at
/// exit 0). Both consumers pinned: the bound builds 10 bits and the localparam
/// folds 9, exactly as PRE and iverilog do. (The WRAPPING twin of this shape —
/// `m - 8'd250` — stays imprecise, the interpreter's recorded width residual.)
#[test]
fn a_width_unknown_exponent_keeps_the_old_fold() {
    let out = run(r#"module t;
  function automatic integer f3();
    logic [1:0][3:0] m;
    integer r;
    m = 8'h02;
    r = 3 ** (m + 0);
    return r;
  endfunction
  logic [f3():0] v;
  localparam integer P = f3();
  initial begin $display("%0d %0d", $bits(v), P); #1 $finish; end
endmodule
"#);
    // iverilog: 10 9   (a refusal here made $bits(v) = 1 at exit 0)
    assert_eq!(out.trim(), "10 9", "width-unknown exponent was refused");
}

/// The env twin's OWN Binary arm is reachable only when `eval_const_assign`
/// has NO target shape. A multi-packed local does NOT take it (its width-0
/// record is a `Some` target, so the value routes through `eval_const_env_at`
/// — measured); a formal whose declared range CONTAINS A CALL does
/// (`const_decl_wsign` declines call-bearing bounds, so the ARG folds with
/// target None). The mutation that reverts that arm to the plain walk binds
/// 6561 here and passes every other test in this file.
#[test]
fn the_shape_unknown_target_path_shares_the_exponent_rule() {
    let out = run(r#"module t;
  function automatic int fw();
    fw = 15;
  endfunction
  function automatic [15:0] f(input [fw():0] p);
    f = p;
  endfunction
  localparam [15:0] P = f(4'sd3 ** -4'sd8);
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#);
    // iverilog: 0   (the unlimited walk would bind 6561)
    assert_eq!(
        out.trim(),
        "0",
        "the env twin's Pow arm widened the exponent"
    );
}

/// The module-scope delegation in `eval_const_env`'s catch-all is gated on the
/// binding maps being EMPTY — with any local binding a module-scope resolution
/// could shadow. This design keeps the gate honest: the function formal `i`
/// shadows the module net `i`, and the body's `$bits(i)` is a form only the
/// delegation could fold — folding it would resolve the MODULE net (4 bits) and
/// silently bind 4 where iverilog answers 32. The honest outcome while the
/// interpreter cannot see formal widths in `$bits` is the foldability reject.
/// ⚠️ The conjunct with teeth here is `envw` (the formal is recorded before any
/// body expression folds); dropping `env.is_empty()` alone is equivalent, as an
/// adversarial probe measured — the two maps move in lockstep.
#[test]
fn a_function_local_shadow_stays_loud_under_delegation() {
    let (out, err) = run_full(
        r#"module t;
  logic [3:0] i;
  function automatic int f(input int i);
    f = $bits(i);
  endfunction
  localparam int P = f(2);
  initial begin $display("V=%0d", P); #1 $finish; end
endmodule
"#,
    );
    // iverilog: 32 (correct-support would need env-aware $bits). Dropping the
    // gate binds the SHADOWED net's 4 silently.
    assert!(!out.contains("V="), "a value was bound: {out}");
    assert!(
        err.contains("E3009"),
        "expected the foldability reject: {err}"
    );
}

/// The module-scope delegation is gated on `depth == 0`, NOT on "contains a
/// call" — a call-bearing subtree at the true module-scope entry terminates
/// exactly where the plain fold always did, so this cell must keep folding
/// (an earlier draft gated on the call and turned it loud).
#[test]
fn a_call_bearing_exponent_still_folds_at_module_scope() {
    let out = run(r#"module t;
  function automatic int cf(input int x);
    cf = x + 3;
  endfunction
  localparam [15:0] P = 2 ** (8'(cf(2)) + 1);
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#);
    // iverilog: 64   (2^(5+1); PRE folds it too — the pin is anti-regression)
    assert_eq!(out.trim(), "64", "call-bearing exponent went loud");
}

/// A SELF-REFERENTIAL default argument must hit the depth cap (E3009), not the
/// stack: argument/default folding runs at `depth + 1` and the module-scope
/// delegation refuses call-bearing subtrees (it would restart the depth at 0).
/// The bare spelling crashed PRE too; the cast spelling is the one the
/// delegation newly exposed. iverilog aborts internally on both — the pin is
/// vita's own ladder (loud, never a crash).
#[test]
fn a_self_referential_default_is_loud_not_a_crash() {
    for src in [
        r#"module t;
  function automatic int f(input int k = f());
    f = k;
  endfunction
  localparam int P = f();
  initial begin $display("V=%0d", P); #1 $finish; end
endmodule
"#,
        r#"module t;
  function automatic int f(input int k = 8'(f()));
    f = k;
  endfunction
  localparam int P = f();
  initial begin $display("V=%0d", P); #1 $finish; end
endmodule
"#,
    ] {
        let (out, err) = run_full(src);
        assert!(!out.contains("V="), "a value was bound: {out}");
        assert!(
            err.contains("E3009") && !err.contains("overflowed"),
            "expected a loud reject, not a crash: {err}"
        );
    }
}

/// `0 ** negative` is `x` (Table 11-6) — a value the i64 const domain cannot
/// carry. PRE silently bound 0 (it saw exponent +8); the honest answer is the
/// foldability reject. iverilog binds `x`.
#[test]
fn a_zero_base_negative_exponent_goes_loud_not_zero() {
    let (out, err) = run_full(
        r#"module t;
  localparam signed [15:0] P = 4'd0 ** -4'sd8;
  initial begin $display("V=%0d", P); #1 $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V="), "a value was bound: {out}");
    assert!(
        err.contains("E3009"),
        "expected the foldability reject: {err}"
    );
}

/// The CONSTANT-FUNCTION interpreter lane (`eval_const_env_at`) already
/// self-determined its Pow exponent; it now shares the one helper. This pins
/// that the shared spelling kept the interpreter's correct answer, and that a
/// leaf exponent (a param) folds identically through both lanes.
#[test]
fn the_const_fn_lane_and_a_param_leaf_agree() {
    let out = run(r#"module t;
  function automatic signed [15:0] f();
    f = 4'sd3 ** -4'sd8;
  endfunction
  parameter signed [3:0] E = -4'sd2;
  localparam signed [15:0] A = f();
  localparam signed [15:0] B = 4'sd3 ** E;
  localparam signed [15:0] C = 4'sd3 ** (E - 4'sd1);
  initial begin $display("%0d %0d %0d", A, B, C); #1 $finish; end
endmodule
"#);
    // iverilog: 0 0 0  (E−1 = −3 all-signed: sign-extension IS correct there)
    assert_eq!(out.trim(), "0 0 0", "lanes disagree on the exponent");
}

/// An explicit ARGUMENT descends a finite AST, so folding it must not consume a
/// level of the call-depth budget — charging it one (an earlier draft did, to
/// bound a self-referential DEFAULT) capped legitimate argument nesting at 32
/// and turned this 65-deep chain from a folded 65 into E3009. Only the DEFAULT
/// position is cyclic, and only it is charged.
#[test]
fn deep_argument_nesting_still_folds() {
    let mut expr = String::from("0");
    for _ in 0..65 {
        expr = format!("g({expr})");
    }
    let out = run(&format!(
        r#"module t;
  function automatic int g(input int x);
    g = x + 1;
  endfunction
  localparam int P = {expr};
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#
    ));
    // iverilog: 65 (and the pre-slice binary folds it too — this is the
    // anti-regression pin for the depth accounting).
    assert_eq!(out.trim(), "65", "argument nesting lost depth budget");
}

/// A CALL-FREE default folds even though it is evaluated inside a call: the
/// module-scope delegation is allowed for a subtree that cannot restart the
/// call depth. `8'(3)` is a cast — a form the env twin does not model itself —
/// so without the delegation it is E3009 (the pre-slice answer), and with a
/// depth-only gate it stays E3009 for the crash's sake. Both are worse than
/// folding it.
#[test]
fn a_call_free_default_folds_inside_the_call() {
    let out = run(r#"module t;
  function automatic int f(input int k = 8'(3));
    f = k;
  endfunction
  localparam int P = f();
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#);
    // iverilog: 3   (PRE: E3009)
    assert_eq!(out.trim(), "3", "call-free default did not fold");
}

/// The Named spelling of a size cast inherits its OPERAND's signedness, and
/// that sign decides the enclosing context's (§11.8.1) — which for an exponent
/// decides Table 11-6. `RPS'(2) - 4'sd9` is 9 in four bits: read signed it is
/// −7 (|base| > 1, negative exponent ⇒ 0), read unsigned it is 9 (3^9 = 19683).
/// Answering "unsigned" for the Named arm — the pre-slice behavior, and a
/// surviving mutation until this pin — silently binds 19683/21459.
/// The third field is the anti-vacuity control: it folds the same either way.
#[test]
fn a_named_size_cast_exponent_carries_its_operand_sign() {
    let out = run(r#"module t;
  parameter RPS = 4;
  localparam signed [15:0] A = 4'sd3 ** (RPS'(2) - 4'sd9);
  localparam signed [15:0] B = 4'sd3 ** (RPS'(6) + 4'sd7);
  localparam signed [15:0] C = 4'sd3 ** (RPS'(2) + 4'sd1);
  initial begin $display("%0d %0d %0d", A, B, C); #1 $finish; end
endmodule
"#);
    // iverilog: 0 0 27   (PRE: 19683 21459 27)
    assert_eq!(out.trim(), "0 0 27", "Named-cast exponent sign lost");
}

/// The Named arm's SIGN reaches beyond the exponent position: a COMPARISON
/// sizes and signs its operands against each other (§11.8.1), so the compare
/// `RPS'(4'sd2) > (4'sd7 + 4'sd7)` is signed (2 > −2, true, exponent 2, 9)
/// when the cast carries its operand's sign and unsigned (2 > 14, false,
/// exponent 5, 243) when it does not. Supplied by the adversarial round as the
/// teeth for the comparison arm, which the Pow-context pin above cannot reach.
#[test]
fn a_named_cast_sign_reaches_the_comparison_arm() {
    let out = run(r#"module t;
  parameter RPS = 4;
  localparam signed [15:0] P = 3 ** ((RPS'(4'sd2) > (4'sd7 + 4'sd7)) ? 2 : 5);
  initial begin $display("%0d", P); #1 $finish; end
endmodule
"#);
    // iverilog: 9   (PRE: 243)
    assert_eq!(
        out.trim(),
        "9",
        "Named-cast sign lost in the comparison arm"
    );
}

/// A genvar IS a signed 32-bit integer (IEEE 1800 §27.4). Its shape is recorded
/// alongside its value, because the const domain's sign model reads that record:
/// with no entry it answered UNSIGNED, so `2 ** (g - 1)` at g = 0 masked −1 into
/// 4294967295 and the fold overflowed into a LOUD reject where every oracle says
/// 0 — a regression this slice introduced by starting to mask at all.
#[test]
fn a_genvar_exponent_is_signed() {
    let out = run(r#"module t;
  genvar g;
  generate for (g = 0; g < 2; g = g + 1) begin : blk
    localparam signed [31:0] P = 2 ** (g - 1);
    localparam signed [31:0] Q = 4'sd3 ** (g - 5);
    localparam signed [31:0] R = 2 ** (g + 3);
    initial begin $display("g%0d: %0d %0d %0d", g, P, Q, R); end
  end endgenerate
  initial begin #1 $finish; end
endmodule
"#);
    // iverilog: "g0: 0 0 8" and "g1: 1 0 16" (R is the anti-vacuity control —
    // a non-negative intermediate folds the same under either sign).
    assert!(out.contains("g0: 0 0 8"), "genvar exponent at g=0: {out}");
    assert!(out.contains("g1: 1 0 16"), "genvar exponent at g=1: {out}");
}
