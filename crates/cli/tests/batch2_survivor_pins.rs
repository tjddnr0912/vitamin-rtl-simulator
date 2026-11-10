//! Pins for the batch 2 boundary's mutation survivors (W/batch2/battery: P10 P12 P19 P22 E07 E13
//! E18 E19 E21 SURVIVED narrow, workspace and lane cells) and for the four site cutoffs the
//! soundness lens named (N1: P16 P25 E11 killed only by the lens's own cells, E01 by none):
//! each cell is an input that tells the site's mutant apart, with POST's whole stdout, stderr
//! (code, text, line:col) and exit code. Every expected text was measured on POST (a392795c,
//! md5 73ea7532) and on PRE (4a8f086b, md5 56c997be), which print the same, with `t.sv` as the
//! argument, so positions print as `t.sv:<line>:<col>`. P15 and P27 are equivalent mutants
//! (W/batch2/pins/EQUIV.txt) and have no pin.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_b2_survivor_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn check(id: &str, src: &str, want_stdout: &str, want_stderr: &str, want_rc: i32) {
    let (so, se, rc) = run(src);
    assert_eq!(so, want_stdout, "{id}: stdout");
    assert_eq!(se, want_stderr, "{id}: stderr");
    assert_eq!(rc, Some(want_rc), "{id}: exit code");
}

/// P10 (const_fold.rs cw_eval `leaf`): a signed 4-bit leaf `4'sb1111` (-1) in an unsigned 8-bit context zero-extends its own 4-bit pattern to 15, so the member is `[15:0]` (w=17); sign-extending it to the context (the mutant) gives 255 (w=257).
#[test]
fn p10_a_signed_leaf_in_an_unsigned_context_zero_extends() {
    check(
        "p10_a_signed_leaf_in_an_unsigned_context_zero_extends",
        r#"module top;
  typedef struct packed { logic [4'sb1111 + 8'd0:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#"L w=17
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// P12 (const_fold.rs cw_eval binary arm): `(4'd15 + 4'd1) / 4'd2` at 4 bits: the sum wraps to 0 before the division reads it, so the member is `[0:0]` (w=2); an unmasked sum (the mutant) divides 16 and gives `[8:0]` (w=10).
#[test]
fn p12_a_division_reads_the_wrapped_sum() {
    check(
        "p12_a_division_reads_the_wrapped_sum",
        r#"module top;
  typedef struct packed { logic [(4'd15 + 4'd1) / 4'd2:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#"L w=2
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// P16 (lens N1 c05, const_fold.rs based_lit_pattern): a size `4294967297` is past u32, so the parse-time fold declines and the member width is refused; `as u32` (the mutant) reads size 1 and lays out `[1:0]`.
#[test]
fn p16_a_based_size_past_u32_declines() {
    check(
        "p16_a_based_size_past_u32_declines",
        r#"module top;
  typedef struct packed { logic [4294967297'd1:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:2:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found keyword 'initial'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// P19 (const_fold.rs const_param_fits): `logic [63:0] P = -1` does not hold -1, so P is not recorded and `[P + 3:0]` is refused; admitting a negative into an unsigned 64-bit type (the mutant) records -1 and lays out `[2:0]`.
#[test]
fn p19_a_negative_value_is_not_recorded_for_an_unsigned_64_bit_param() {
    check(
        "p19_a_negative_value_is_not_recorded_for_an_unsigned_64_bit_param",
        r#"module top;
  localparam logic [63:0] P = -1;
  typedef struct packed { logic [P + 3:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:3:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found keyword 'initial'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// P22 (const_fold.rs index_literal): `2147483647 - i` is an admitted index shape (`L-V`, L within int), so the out-of-range writes are dropped and the run prints; `<` (the mutant) refuses the index.
#[test]
fn p22_an_index_literal_of_i32_max_is_within_int() {
    check(
        "p22_an_index_literal_of_i32_max_is_within_int",
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { m1_t [3:0] m; logic [3:0] z; } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v;
  always_comb for (int i = 0; i < 4; i++) s.m[i].q = v[1:0];
  always_comb for (int i = 0; i < 4; i++) s.m[2147483647 - i].qe = v[i];
  assign s.z = v;
  initial #900 $finish;
  initial begin v = 4'b0101; #1 $display("A s=%h", s); $finish; end
endmodule
"#,
        r#"A s=XXX5
simulation ended (Finish) at time 1
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// P25 (lens N1 c02, expr_primary.rs parse_small_const): `##4294967296` is past u32 and refused; `as u32` (the mutant) reads it as `##0`.
#[test]
fn p25_an_sva_delay_past_u32_is_refused() {
    check(
        "p25_an_sva_delay_past_u32_is_refused",
        r#"module top; logic clk = 0, a = 1, b = 1;
  always #1 clk = ~clk;
  property p; @(posedge clk) a |-> ##4294967296 b; endproperty
  assert property (p);
  initial begin #50 $display("L ok"); $finish; end
endmodule
"#,
        r#""#,
        r#"t.sv:3:49: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected a constant cycle delay after `##`, found identifier 'b'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// E01 (lens N1 p1, block_local/proofs.rs fold_i32): `repeat (-4294967295)` has a leaf past i32, so the trip count is not proven and the read stays loud; `as i32` (the mutant) folds -(-1) = 1, proves one trip, and the engine runs zero (the previous activation's leftover is read).
#[test]
fn e01_a_repeat_count_past_i32_is_not_proven() {
    check(
        "e01_a_repeat_count_past_i32_is_not_proven",
        r#"module t;
  task automatic fill (output byte o []); o = new[2]; o[0] = 8'h61; o[1] = 8'h62; endtask
  initial begin
    for (int a = 0; a < 2; a++) begin
      automatic byte cur [];
      repeat (-4294967295) fill(cur);
      $display("a=%0d size=%0d", a, cur.size());
      cur = new[7];
    end
    #1 $finish;
  end
endmodule
"#,
        r#""#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:5:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: an `automatic` block-local `cur` whose per-entry lifetime differs from static (an initializer, or a read before its first write) is unsupported in a procedural block (v1 flattens block-locals to one static net); assign it before use, or drop `automatic` [in t]
t.sv:7:7: note[VITA-E3009] E-ELAB-UNSUPPORTED: definite-assignment for `cur` stopped here: this statement form is not modelled by the walk, and it references the local before any write on this path. Everything after this point is treated as if `cur` were still unwritten [in t]
errors=1 warnings=1 notes=1
"#,
        1,
    );
}

/// E07 (const_fn.rs eval_const_env binary arm): a formal whose range holds a call has no target shape, so the argument `10 - 3` folds in the plain walk: 7; swapped operands (the mutant) give -7.
#[test]
fn e07_a_subtraction_in_an_unknown_target_keeps_its_operand_order() {
    check(
        "e07_a_subtraction_in_an_unknown_target_keeps_its_operand_order",
        r#"module top;
  function automatic int g(input int x); return x; endfunction
  function automatic int f(input logic [g(31):0] a); return a; endfunction
  localparam int P = f(10 - 3);
  initial begin
    $display("L P=%0d", P);
    $finish;
  end
endmodule
"#,
        r#"L P=7
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// E11 (lens N1 c04, literal.rs): a size `4294967297` is past u32 and the literal is refused; `as u32` (the mutant) reads `1'd1`.
#[test]
fn e11_a_literal_size_past_u32_is_malformed() {
    check(
        "e11_a_literal_size_past_u32_is_malformed",
        r#"module top;
  initial begin
    $display("L %0d", 4294967297'd1);
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: malformed integer literal `4294967297'd1` [in top]
errors=1 warnings=1 notes=0
"#,
        1,
    );
}

/// E13 (packed.rs const_index_value): `64'hFFFF_FFFF_FFFF_FFFF` as a slice base is i64::MAX (a huge positive, never negative), as the error text prints; i64::MIN (the mutant) prints a negative window.
#[test]
fn e13_an_unsigned_64_bit_constant_index_saturates_high() {
    check(
        "e13_an_unsigned_64_bit_constant_index_saturates_high",
        r#"module top;
  logic [7:0] a [0:3];
  logic [7:0] b [0:3];
  initial begin
    b = '{8'h11, 8'h22, 8'h33, 8'h44};
    a[64'hFFFF_FFFF_FFFF_FFFF +: 2] = b[0:1];
    $display("L a=%h", a[1]);
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:6:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: an unpacked-array slice [9223372036854775807..9223372036854775807] outside the dimension's range [0..3] [in top]
errors=1 warnings=1 notes=0
"#,
        1,
    );
}

/// E18 (edge_gate.rs is_other_sys_call): `$rtoi` reads text of its own, so a `P[0 +: $rtoi(N)]` width of 0 is not claimed and the value is refused as PRE refuses it; reading `real_conversion` (the mutant) claims it and prints the zero-width text.
#[test]
fn e18_a_rtoi_width_is_not_claimed_zero() {
    check(
        "e18_a_rtoi_width_is_not_claimed_zero",
        r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] X = P[0 +: $rtoi(N)];
  initial #1 $display("A zlr X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
        r#""#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `X` value is not a constant: the part-select `P[…]` has no constant-fold arm [in top.u]
errors=1 warnings=1 notes=0
"#,
        1,
    );
}

/// E19 (param_query.rs real_conversion_is_folded): `$bits(R)` is not a real-to-integral conversion, so the replication count reads the real parameter and is refused, as PRE refuses it; `folds_integer` (the mutant, which holds `$bits`) stands down where `$bits(R)` folds (64, for a real `R = 4` with an exact integer twin) and replicates. A non-integral `R = 1.5` does not tell them apart: `$bits(R)` does not fold there and neither stands down.
#[test]
fn e19_a_bits_of_a_real_is_not_a_real_conversion() {
    check(
        "e19_a_bits_of_a_real_is_not_a_real_conversion",
        r#"module top;
  parameter real R = 4;
  logic [63:0] v;
  initial begin
    v = {$bits(R){1'b1}};
    $display("L v=%h", v);
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:5:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: a replication count that reads a real parameter is unsupported (a real has no integral constant value) [in top]
t.sv:5:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: a replication count of zero is only legal as a direct operand of a concatenation (IEEE §11.4.12.1) [in top]
errors=2 warnings=1 notes=0
"#,
        1,
    );
}

/// E21 (override_type.rs expr_result_is_real): `0 ? $itor(3) : 5` has a real arm, so the override is real 5.0 and `P/2` is 2.5; reading `sysfunc_id` without the `$` (the mutant) knows no system function, the override stays integral and `P/2` is 2.
#[test]
fn e21_a_real_system_function_in_a_dead_arm_makes_the_override_real() {
    check(
        "e21_a_real_system_function_in_a_dead_arm_makes_the_override_real",
        r#"module sub #(parameter P = 3) ();
  initial #1 $display("T %m %f", P/2);
endmodule
module t;
  sub #(.P(0 ? $itor(3) : 5)) a();
  sub #(.P(0 ? $sqrt(4.0) : 5)) b();
  sub #(.P(0 ? 7 : 5)) c();
  initial #3 $finish;
endmodule
"#,
        r#"T t.a 2.500000
T t.b 2.500000
T t.c 2.000000
simulation ended (Finish) at time 3
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}
