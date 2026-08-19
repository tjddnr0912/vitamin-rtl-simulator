//! §2 clearance 1–3 — SELF-DETERMINED positions in the const domain.
//!
//! One root: the width-unlimited i64 fold (`const_eval_in_scope`) was answering
//! positions that IEEE sizes BY THEMSELVES (§11.6.1 Table 11-21), so a value
//! that wraps at its self width folded un-wrapped:
//!
//!   1. a `$clog2` ARGUMENT — `$clog2(4'd15 + 4'd1)` folded 4 where the 4-bit
//!      sum wraps to 0 (iverilog 0 — and vita's OWN constant-function
//!      interpreter already answered 0 for the same text: two answers for one
//!      source line);
//!   2. a size cast's SIZE expression — `(4'd9+4'd8)'(2)` sized a 17-bit cast
//!      where the 4-bit sum wraps to 1 (iverilog: `1'(2)` = 0);
//!   3. an integral subtree CONVERTING TO REAL (§11.8.1 — the real side gives
//!      it no width context) — `2.0 ** -4'sd8` promoted the exponent to +8
//!      (iverilog 0.003906), and the same widening hit every real operator's
//!      integral operand (`1.0 + -4'sd8` → 9.0 instead of −7.0) and a
//!      declared-real parameter's wholly integral initializer.
//!
//! All three now go through the §4.5.339 self-determined walk
//! (`eval_const_env_self`, degrading to the unlimited domain where the width is
//! unknown — a refusal is only as loud as its caller). Oracle: iverilog 13.0 for
//! every cell except the treated-as-unsigned `$clog2` cells, where iverilog
//! converts the argument to a 32-bit integer first and answers 32; Verilator
//! 5.050, vita's runtime engine, and §20.8.1's own text ("treated as an
//! unsigned value" — a reading of the argument's bit pattern at its own width)
//! all answer 3, so those cells are pinned to 3.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_selfdet_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

fn loud(src: &str, needle: &str) {
    let (_, ok, err) = run_raw(src);
    assert!(!ok, "expected a loud reject");
    assert!(err.contains(needle), "unexpected diagnostic:\n{err}");
}

/// #1 headline: the module-scope fold, the constant-function interpreter and
/// the runtime engine all answer 0 for one source expression — the defect was
/// exactly that the first said 4 while the other two said 0.
#[test]
fn clog2_selfdet_arg_three_evaluators_agree() {
    let out = run("module top;\n\
         localparam L = $clog2(4'd15 + 4'd1);\n\
         function automatic integer f();\n\
           f = $clog2(4'd15 + 4'd1);\n\
         endfunction\n\
         localparam LF = f();\n\
         integer r;\n\
         initial begin\n\
           r = $clog2(4'd15 + 4'd1);\n\
           $display(\"L=%0d LF=%0d R=%0d\", L, LF, r);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("L=0 LF=0 R=0"), "got:\n{out}");
}

/// The non-wrapping neighbours keep their values (iverilog-pinned): a sum that
/// stays inside its width, parameters, an unsized literal, a concatenation,
/// and the tiny-argument edge cells.
#[test]
fn clog2_selfdet_arg_matrix() {
    let out = run("module top;\n\
         localparam W = 16;\n\
         localparam A2 = $clog2(4'd15 + 4'd15);\n\
         localparam A3 = $clog2(8'd255 + 8'd1);\n\
         localparam A6 = $clog2(W);\n\
         localparam A7 = $clog2(W + 1);\n\
         localparam A8 = $clog2(4'd8 * 4'd4);\n\
         localparam A9 = $clog2(2'd3 << 1);\n\
         localparam A13 = $clog2(16);\n\
         localparam A15 = $clog2({2'b10, 2'b01});\n\
         localparam E0 = $clog2(0);\n\
         localparam E1 = $clog2(1);\n\
         localparam E2 = $clog2(2);\n\
         initial $display(\"%0d %0d %0d %0d %0d %0d %0d %0d%0d%0d\",\n\
                          A2, A3, A6, A7, A8, A9, A13, E0, E1, E2);\n\
         localparam A15C = A15;\n\
         initial $display(\"A15=%0d\", A15C);\n\
         endmodule\n");
    assert!(out.contains("4 0 4 5 0 1 4 001"), "got:\n{out}");
    assert!(out.contains("A15=4"), "got:\n{out}");
}

/// §20.8.1 "treated as an unsigned value": the argument's bit pattern at its
/// own width. `4'sd7 + 4'sd1` wraps to the 4-bit pattern 1000 = unsigned 8 ⇒ 3.
/// Verilator 5.050, vita's runtime, and the LRM text agree on 3; iverilog 13.0
/// answers 32 (it converts the −8 to a 32-bit integer first) and is the
/// recorded divergence. `$clog2(-1)` is a 32-bit all-ones pattern ⇒ 32 — BOTH
/// oracles agree there, and the old `n < 0 → None` refusal kept it loud.
#[test]
fn clog2_arg_treated_as_unsigned_at_self_width() {
    let out = run("module top;\n\
         localparam C1 = $clog2(4'sd7 + 4'sd1);\n\
         localparam C2 = $clog2(-4'sd8);\n\
         localparam C3 = $clog2(-1);\n\
         localparam signed [3:0] SP = -8;\n\
         localparam C4 = $clog2(SP);\n\
         integer r;\n\
         initial begin\n\
           r = $clog2(4'sd7 + 4'sd1);\n\
           $display(\"C1=%0d C2=%0d C3=%0d C4=%0d R=%0d\", C1, C2, C3, C4, r);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("C1=3 C2=3 C3=32 C4=3 R=3"), "got:\n{out}");
}

/// A `**` inside the argument is still self-determined arithmetic: `2 ** 33`
/// wraps to 0 at the unsized 32-bit width, so `$clog2` of it is 0 (iverilog 0;
/// the unlimited fold answered 33).
#[test]
fn clog2_wide_pow_argument_wraps() {
    let out = run("module top;\n\
         localparam A = $clog2(2 ** 33);\n\
         initial $display(\"A=%0d\", A);\n\
         endmodule\n");
    assert!(out.contains("A=0"), "got:\n{out}");
}

/// The wrap must also happen at positions NARROWER than the whole argument —
/// a shift COUNT inside the argument is its own self-determined region, so the
/// final treated-as-unsigned masking of the argument cannot rescue an inner
/// widening: `12'd100 >> (4'd15 + 4'd1)` is `100 >> 0` = 100 ⇒ 7 (both
/// oracles; a walk that widened internally would compute `100 >> 16` = 0, and
/// 0 and 7 both fit the argument's 12 bits). Three routes hit three arms:
/// module scope, the plain env twin (via a multi-packed body local whose
/// declaration shape the interpreter cannot size — the shape-unknown-target
/// path), and the width-aware twin (via a signed wrap in a const-fn body,
/// verilator-pinned like the module-scope cells).
#[test]
fn clog2_inner_selfdet_region_all_three_arms() {
    let out = run("module top;\n\
         localparam A18 = $clog2(12'd100 >> (4'd15 + 4'd1));\n\
         function automatic integer g();\n\
           logic [1:0][3:0] t;\n\
           t = $clog2(12'd100 >> (4'd15 + 4'd1));\n\
           g = t;\n\
         endfunction\n\
         localparam G = g();\n\
         function automatic integer f2();\n\
           f2 = $clog2(4'sd7 + 4'sd1);\n\
         endfunction\n\
         localparam LF2 = f2();\n\
         initial $display(\"A18=%0d G=%0d LF2=%0d\", A18, G, LF2);\n\
         endmodule\n");
    assert!(out.contains("A18=7 G=7 LF2=3"), "got:\n{out}");
}

/// The PLAIN env twin's `$clog2` arm (the shape-unknown-target walk) — reached
/// when a call ARGUMENT binds to a formal whose range the interpreter must not
/// fold (a constant-function call in the bound declines `const_decl_wsign`, so
/// the argument folds with NO target shape). Both oracles answer 7; a
/// plain-walk widening of the inner shift count computes `100 >> 16` = 0
/// silently (measured on the mutant build — exit 0, G2=0).
#[test]
fn clog2_plain_twin_via_unfoldable_formal_range() {
    let out = run("module top;\n\
         function automatic integer w4();\n\
           w4 = 3;\n\
         endfunction\n\
         function automatic integer g2(input logic [w4():0] x);\n\
           g2 = x;\n\
         endfunction\n\
         localparam G2 = g2($clog2(12'd100 >> (4'd15 + 4'd1)));\n\
         initial $display(\"G2=%0d\", G2);\n\
         endmodule\n");
    assert!(out.contains("G2=7"), "got:\n{out}");
}

/// #2 headline (runtime lane): the size expression of a cast is its own
/// context. `(4'd9+4'd8)` wraps to 1, so the cast truncates 2 to a 1-bit 0 and
/// `4'd3 ** 0` is 1 (iverilog 1; the unlimited fold sized 17 bits and answered
/// 9). Non-wrapping sizes keep their values.
#[test]
fn cast_size_expression_selfdet_runtime() {
    let out = run("module top;\n\
         localparam W = 8;\n\
         logic [31:0] r;\n\
         logic [15:0] v;\n\
         initial begin\n\
           v = 4'd3 ** ((4'd9+4'd8)'(2)); $display(\"B6=%0d\", v);\n\
           r = (4'd2+4'd2)'(200);         $display(\"B3=%0d\", r);\n\
           r = W'(511);                   $display(\"B4=%0d\", r);\n\
           r = (2'd3<<1)'(255);           $display(\"B9=%0d\", r);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("B6=1"), "got:\n{out}");
    assert!(out.contains("B3=4294967288"), "got:\n{out}");
    assert!(out.contains("B4=4294967295"), "got:\n{out}");
    assert!(out.contains("B9=4294967295"), "got:\n{out}");
}

/// #2, const lane: this used to DECLINE (loud) — the const domain folded a size
/// cast only where truncation could not occur — and the pin recorded iverilog's
/// 1 as what full correct-support would give. §4.5.345 supplied the width-aware
/// operand fold that arm was waiting for: the exponent's `(4'd9+4'd8)'(2)` is a
/// 1-bit cast of 2, i.e. 0, so `4'd3 ** 0` is iverilog's 1.
///
/// ⚠️ This is the SELF-DETERMINED (exponent) lane, which routes through the
/// interpreter's width-aware walk. A size cast reached through `const_eval_cast`
/// — a plain `localparam P = 4'((4'd8+4'd8)/4'd3);` — still folds its operand in
/// the width-UNLIMITED domain and answers 5 for iverilog's 0 (ROADMAP §2).
#[test]
fn cast_size_selfdet_const_folds_at_the_cast_width() {
    let out = run("module top;\n\
         localparam P = 4'd3 ** ((4'd9+4'd8)'(2));\n\
         initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n");
    assert!(out.contains("P=1"), "got:\n{out}");
}

/// A size expression that wraps to ZERO is rejected by both simulators
/// (iverilog: "Cast size expression must be constant and greater than zero").
#[test]
fn cast_size_selfdet_zero_is_loud() {
    loud(
        "module top;\n\
         logic [31:0] r;\n\
         initial begin r = (3'd4+3'd4)'(7); $display(\"B8=%0d\", r); end\n\
         endmodule\n",
        "size cast width must be a positive constant expression",
    );
}

/// #3 headline: every real operator's integral operand converts at its OWN
/// width (§11.8.1). 18 cells, byte-compared against iverilog 13.0 during the
/// slice; the interesting ones — a wrapping negate, a wrapping sum, integer
/// division kept integral, a shift, a concat, a wide signed literal, and the
/// two `**` shapes (integer exponent wraps; real exponent untouched).
#[test]
fn real_integral_operand_selfdet_matrix() {
    let out = run("module top;\n\
         localparam real C01 = 1.0 + -4'sd8;\n\
         localparam real C03 = 2.0 * (4'd15 + 4'd1);\n\
         localparam real C05 = 2.0 - (3'd7 + 3'd1);\n\
         localparam real C07 = 2.0 ** -4'sd8;\n\
         localparam real C08 = 2.0 ** (4'd15 + 4'd1);\n\
         localparam real C09 = 1.0 + 3/2;\n\
         localparam real C10 = 1.0 + (2'd3 << 1);\n\
         localparam real C16 = 2.0 ** 0.5;\n\
         localparam real C17 = 1.0 + {2'b10, 2'b01};\n\
         localparam real C18 = 1.0 + 16'shFFFF;\n\
         initial begin\n\
           $display(\"C01=%f C03=%f C05=%f\", C01, C03, C05);\n\
           $display(\"C07=%f C08=%f C09=%f\", C07, C08, C09);\n\
           $display(\"C10=%f C16=%f\", C10, C16);\n\
           $display(\"C17=%f C18=%f\", C17, C18);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("C01=-7.000000 C03=0.000000 C05=2.000000"),
        "got:\n{out}"
    );
    assert!(
        out.contains("C07=0.003906 C08=1.000000 C09=2.000000"),
        "got:\n{out}"
    );
    assert!(out.contains("C10=3.000000 C16=1.414214"), "got:\n{out}");
    assert!(out.contains("C17=10.000000 C18=0.000000"), "got:\n{out}");
}

/// #3, declared-real parameter with a WHOLLY integral initializer: the value
/// converts at its self-determined size, and the i64 twin registers the SAME
/// value (`P` sizes a range below to prove the twin moved with it).
#[test]
fn real_param_integral_init_selfdet() {
    let out = run("module top;\n\
         parameter real P = 4'd15 + 4'd1;\n\
         parameter real Q = -4'sd8;\n\
         parameter real S = 1.0 + 3/2;\n\
         parameter real T = 2.0 + 4'd9 * 4'd9;\n\
         initial $display(\"P=%f Q=%f S=%f T=%f\", P, Q, S, T);\n\
         endmodule\n");
    assert!(
        out.contains("P=0.000000 Q=-8.000000 S=2.000000 T=3.000000"),
        "got:\n{out}"
    );
}

/// #3, the ternary CONDITION is a self-determined position too (§11.4.11):
/// `(4'd15 + 4'd1) ? 1.5 : 2.5` takes the ELSE arm (the 4-bit sum is 0).
#[test]
fn real_ternary_cond_selfdet() {
    let out = run("module top;\n\
         localparam real T1 = (4'd15 + 4'd1) ? 1.5 : 2.5;\n\
         localparam real T2 = (-4'sd8 + 4'sd8) ? 1.5 : 2.5;\n\
         localparam real T3 = (4'd2 + 4'd2) ? 1.5 : 2.5;\n\
         initial $display(\"T1=%f T2=%f T3=%f\", T1, T2, T3);\n\
         endmodule\n");
    assert!(
        out.contains("T1=2.500000 T2=2.500000 T3=1.500000"),
        "got:\n{out}"
    );
}
