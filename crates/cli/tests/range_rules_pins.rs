//! §5.2 Q-VALRULES fix (d) slice A (premise judge round 4, K4-2): value pins on the cells that broke this row in
//! earlier rounds, each on the value the pre-change binary (PRE, d59629fa) prints -- slice A moves every range rule into
//! `hdl_ast::range_rules` and must move no value.  md64n_sel (design2 D2-1: a bound past 32 bits narrowed),
//! q2b_idx_signed_unsstride / m_pkg_imp_signed / a_l4_z0 (plan4 F19: a lane read through another lane's rule),
//! w40 (premise3: a 40-bit unsigned bound), c_l8_s16sh8000 (plan4: loud -> value), vs2k1 (premise4 K7: the enum base
//! lane vs the struct member lane past 64 bits).  Oracle readings are in the slice's commit message.
//! Design review 3 (D2-10) adds: f1a_wrap32 (the bound value rule F1a, PRE's value) and w1_tdbits, w1_gen, n08 (a
//! typedef `$bits` the parser answers itself) on PRE's value.  Slice A is structure only (owner 2026-11-07): the
//! refusal pins of R1w / R1x / C1 / R1c went to Q-PARAM2STATE S1v with the refusals.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design: (stdout minus the trailer, stderr + stdout).  A unique per-test dir keeps the parallel runners apart.
fn run_full(src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rrpin_{}_{n}", std::process::id()));
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
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    (
        so.lines()
            .filter(|l| !l.contains("simulation ended"))
            .collect::<Vec<_>>()
            .join("\n"),
        format!("{}\n{so}", String::from_utf8_lossy(&out.stderr)),
    )
}

/// Every `want` line is printed, and no error.
fn assert_lines(src: &str, want: &[&str]) {
    let (out, err) = run_full(src);
    for w in want {
        assert!(
            out.lines().any(|l| l == *w),
            "expected `{w}`:\n{out}\n--\n{err}"
        );
    }
    assert!(!err.contains("error["), "unexpected error:\n{err}");
}

#[test]
fn keeps_md64n_sel() {
    assert_lines(
        r#"module c #(parameter longint L = -64'sd4294967296) ();
  localparam logic [L+1:L][1:0] P = 4'b1001;
  longint i; logic [1:0] a, b, r0, r1;
  initial begin #1 i = L+1; a = P[i]; i = L; b = P[i]; r1 = P[L+1]; r0 = P[L];
    $display("A rt1=%b rt0=%b c1=%b c0=%b hi=%0d lo=%0d inc=%0d", a, b, r1, r0, $high(P), $low(P), $increment(P)); end
endmodule
module top; c u(); initial #2 $finish; endmodule
"#,
        &["A rt1=10 rt0=01 c1=10 c0=01 hi=-4294967295 lo=-4294967296 inc=1"],
    );
}

#[test]
fn keeps_q2b_idx_signed_unsstride() {
    assert_lines(
        r#"module top;
  parameter L3 = 4'd15;
  parameter logic [3:0] N = 4'd3;
  parameter logic [N:0][L3 - 5'sd14 : 0] P = 8'b11_10_01_00;
  logic signed [1:0] k;
  initial begin k = 2'sb11; #1 $display("A p=%b bp=%0d", P[k], $bits(P)); $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A p=11 bp=8"],
    );
}

#[test]
fn keeps_m_pkg_imp_signed() {
    assert_lines(
        r#"package p; parameter bit signed [3:0] S = -2; parameter logic [3:0] U = 1; localparam logic [U:S][1:0] Q = 8'b11_10_01_00; endpackage
module top; import p::*;
  logic [U:S][1:0] n = 8'b11_10_01_00;
  initial begin #1 $display("A bq=%0d l=%0d r=%0d inc=%0d bn=%0d nl=%0d nr=%0d", $bits(Q), $left(Q), $right(Q), $increment(Q), $bits(n), $left(n), $right(n)); $finish; end
endmodule
"#,
        &["A bq=28 l=1 r=-2 inc=-1 bn=8 nl=1 nr=-2"],
    );
}

#[test]
fn keeps_w40() {
    assert_lines(
        r#"module top; localparam logic [39:0] U = 0; logic [U-1:U-2] n;
  initial begin n = '1; #1 $display("A bn=%0d sz=%0d n=%b", $bits(n), $size(n), n); $finish; end endmodule
"#,
        &["A bn=2 sz=2 n=11"],
    );
}

#[test]
fn keeps_a_l4_z0() {
    assert_lines(
        r#"module c #(parameter logic [3:0] X = 2) ();
  typedef logic [X-1:0] t_t;
  t_t v; logic [X-1:0] n;
  initial begin #1 v = '1; n = '1;
    $display("A bt=%0d bn=%0d sz=%0d l=%0d r=%0d", $bits(t_t), $bits(n), $size(v), $left(v), $right(v));
    $display("A v=%0d n=%0d", v, n); end
endmodule
module top; c #(.X(0)) u(); initial #2 $finish; endmodule
"#,
        &["A bt=0 bn=2 sz=2 l=-1 r=0", "A v=3 n=3"],
    );
}

#[test]
fn keeps_c_l8_s16sh8000_refused() {
    let (out, err) = run_full(
        r#"module tb;
  localparam logic [7:0] X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
    );
    assert!(!out.contains("DIGEST"), "a value was bound: {out}");
    assert!(
        err.contains("error[VITA-E2002]")
            && err.contains("struct member width must be a named integer type or a constant-literal range in v1"),
        "expected the member-width refusal: {err}"
    );
}

#[test]
fn keeps_vs2k1_enum_base_past_64() {
    assert_lines(
        r#"module top;
  typedef enum logic [127:0] {A = 5, B} e_t;
  e_t e;
  initial begin e = B; $display("A e=%0d a=%0d b=%0d", e, A, $bits(e)); end
endmodule
"#,
        &["A e=6 a=5 b=128"],
    );
}

// design review 3 (D2-10): a value pin on f1a_wrap32 -- the bound value rule F1a (PRE's)
// wraps a 32-bit sum (5e9 mod 2^32 = 705032704), and the battery's F1A mutant moved it.
#[test]
fn keeps_f1a_wrap32() {
    assert_lines(
        r#"module top;
  logic [32'd4000000000 + 32'd1000000000 : 705032700] n;
  initial begin n = '1; #1 $display("A bn=%0d l=%0d n=%b", $bits(n), $left(n), n); $finish; end
endmodule
"#,
        &["A bn=5 l=705032704 n=11111"],
    );
}

// OPEN lanes (F14): R1c does not reach the parser's own typedef `$bits` lane, so these
// keep PRE's value -- pinned on it so a change to them is a visible edit (owner S1v).
#[test]
fn keeps_open_lane_w1_tdbits() {
    assert_lines(
        r#"module top; typedef logic [2147483648:0] t_t; initial begin #1 $display("A bt=%0d", $bits(t_t)); $finish; end endmodule
"#,
        &["A bt=2147483649"],
    );
}

#[test]
fn keeps_open_lane_w1_gen() {
    assert_lines(
        r#"module top;
  typedef logic [2147483648:0] t_t;
  genvar g; for (g = 0; g < $bits(t_t) - 2147483647; g++) begin : gb logic n; end
  initial begin #2 $display("A ok"); $finish; end
endmodule
"#,
        &["A ok"],
    );
}

#[test]
fn keeps_open_lane_n08() {
    assert_lines(
        r#"module top #(parameter P = 32'h7FFF_FFFF);
  typedef logic [P:0] t_t;
  localparam L = $bits(t_t);
  initial begin #1 $display("A l=%0d", L); $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A l=2147483648"],
    );
}

// Fix round r3 (lens2 B1): a part-select span inside `$bits` counts by its own lane
// (`Lane::StructMember`, PRE's spelling), not the record's `integer` rule. Each is PRE's value
// and iverilog's (verilator: "Width of bit extract must be positive"); the record lane
// printed "parameter `W` value is not a constant" / "a `$bits` that is not a constant" /
// "generate-if condition is not a constant".
#[test]
fn keeps_es2_select_span_in_param() {
    assert_lines(
        r#"module top;
  logic [7:0] x;
  localparam int W = $bits(x[32'h8000_0000:0]);
  initial begin $display("A w=%0d", W); #1 $finish; end
endmodule
"#,
        &["A w=-2147483647"],
    );
}

#[test]
fn keeps_es4_select_span_in_range_bound() {
    assert_lines(
        r#"module top;
  logic [7:0] x;
  logic [$bits(x[32'h8000_0000:0]) > 5 ? 3 : 1 : 0] y;
  initial begin $display("A y=%0d", $bits(y)); #1 $finish; end
endmodule
"#,
        &["A y=2"],
    );
}

#[test]
fn keeps_es5_select_span_in_generate_if() {
    assert_lines(
        r#"module top;
  logic [7:0] x;
  if ($bits(x[32'h8000_0000:0]) != 0) begin : g
    initial $display("A g=1");
  end
  initial #1 $finish;
endmodule
"#,
        &["A g=1"],
    );
}

// KNOWN-WRONG on PRE's value (iverilog `A w=1`, verilator `A w=3`): the override's span
// count wraps; slice A keeps PRE's value, the bound range rule is Q-PARAM2STATE S1v's.
#[test]
fn keeps_es6_select_span_in_override_known_wrong() {
    assert_lines(
        r#"module c #(parameter int W = 1) ();
  initial $display("A w=%0d", W);
endmodule
module top;
  logic [7:0] x;
  c #(.W($bits(x[32'hFFFF_FFFE:0]))) u();
  initial #1 $finish;
endmodule
"#,
        &["A w=-1"],
    );
}

// Fix round r3 (lens2 F-D): a package function's range folds in the package (`Z = 3`,
// IEEE 1800-2017 §26.3) beside a caller `Z` = 2^64 - 1 (a bound S1v's R1w will refuse); the
// caller-scope half of the split takes PRE's step, so the declaring-scope half decides. PRE = iverilog =
// sv2v = verilator `A v=13` / `A v=14`.
#[test]
fn keeps_r01_package_range_beside_a_caller_constant_past_i64() {
    assert_lines(
        r#"package q; localparam int Z = 3; function automatic logic [Z:0] h(int a); return a; endfunction endpackage
module top;
  localparam longint unsigned Z = 64'hFFFF_FFFF_FFFF_FFFF;
  initial $display("A v=%0d", q::h(13));
  initial #20 $finish;
endmodule
"#,
        &["A v=13"],
    );
}

#[test]
fn keeps_r14_nested_package_call_beside_a_caller_constant_past_i64() {
    assert_lines(
        r#"package q; localparam int Z = 3; function automatic logic [Z:0] h(int a); return a; endfunction function automatic int g(int a); return h(a) + 1; endfunction endpackage
module top;
  localparam longint unsigned Z = 64'hFFFF_FFFF_FFFF_FFFF;
  initial $display("A v=%0d", q::g(13));
  initial #20 $finish;
endmodule
"#,
        &["A v=14"],
    );
}

// Fix round r3 (F3 census K6): a SELECTED bound folds with PRE's step (no R1w): r2 refused
// `x[U:0]` with U = 2^64 - 1 as "undefined name `x` is not a constant". KNOWN-WRONG on PRE's
// value (iverilog `A w=0`, verilator "Value too wide for 32-bits"): slice A keeps PRE's value,
// the bound range rule is Q-PARAM2STATE S1v's.
#[test]
fn keeps_s1_select_bound_past_i64_known_wrong() {
    assert_lines(
        r#"module top;
  localparam longint unsigned U = 64'hFFFF_FFFF_FFFF_FFFF;
  logic [7:0] x;
  localparam int W = $bits(x[U:0]);
  initial begin $display("A w=%0d", W); #1 $finish; end
endmodule
"#,
        &["A w=2"],
    );
}

// Premise7 C-b (class EVALUATION-ORDER): a module routine whose return range's LSB calls the
// routine itself, behind an msb that does not fold (`[v : h(1)]`, v a variable). PRE folds the
// msb first, fails and never folds the lsb: rc 1 with E3009. Folding the lsb as well (an eager
// record, premise6 B1) recursed until the stack overflowed (SIGABRT). Pinned: rc 1, an E3009
// line, no signal and no overflow text -- not PRE's full wording.
#[test]
fn keeps_lsb_self_call_behind_an_unfolded_msb_loud() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rrpin_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        r#"module top;
  int v;
  function automatic logic [v : h(1)] h(input int a);
    h = a;
  endfunction
  localparam int K = h(5);
  initial begin #1 $display("A k=%0d", K); $finish; end
endmodule
"#,
    )
    .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected rc 1 (no signal): {:?}\n{all}",
        out.status
    );
    assert!(
        all.lines().any(|l| l.contains("error[VITA-E3009]")),
        "expected an E3009 line:\n{all}"
    );
    assert!(
        !all.contains("overflowed its stack") && !all.contains("fatal runtime error"),
        "no stack overflow:\n{all}"
    );
}
