//! P0-5/6/7: elaborate constant-domain semantics (REMAINING_WORK 2026-06-10).
//!
//! The elaboration const evaluator was an unsigned u32 domain with silent-0
//! defaults: unfoldable params bound 0 with no diagnostic, `1<<32` folded to
//! 0 (iverilog: 4294967296), signed `>>>` used a logical shift, and a
//! descending `for (i=N; i>=0; i=i-1)` generate loop never terminated
//! (unsigned `i>=0` is always true → unroll-cap error). These tests pin the
//! signed-i64 domain + loud-error semantics.

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

/// lex → parse → elaborate → simulate; returns (stdout, all diagnostics).
fn run_with_diags(src: &str) -> (String, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow().clone();
    let out = match ir {
        Some(ir) => simulate_capture(&ir, SimOpts::default()).1,
        None => String::new(),
    };
    (out, diags)
}

/// Happy-path runner: asserts elaboration emitted no Error/Fatal.
fn run(src: &str) -> String {
    let (out, diags) = run_with_diags(src);
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    out
}

/// P0-5: a ternary `?:` in a parameter value must fold (was: silent 0).
#[test]
fn ternary_param_folds() {
    let out = run(r#"
module t;
  parameter MODE = 1;
  parameter W = MODE ? 16 : 8;
  initial $display("W=%0d", W);
endmodule
"#);
    assert_eq!(out.trim(), "W=16");
}

/// P0-5: `$clog2` in a parameter value must fold (the ubiquitous
/// `localparam AW = $clog2(DEPTH)` idiom; was: silent 0).
#[test]
fn clog2_param_folds() {
    let out = run(r#"
module t;
  parameter DEPTH = 300;
  localparam AW = $clog2(DEPTH);
  localparam A1 = $clog2(1);
  localparam A2 = $clog2(2);
  initial $display("AW=%0d A1=%0d A2=%0d", AW, A1, A2);
endmodule
"#);
    assert_eq!(out.trim(), "AW=9 A1=0 A2=1");
}

/// P0-6: `1 << 32` folds to 4294967296 in the i64 domain (iverilog parity);
/// the old u32 domain gave 1 (wrapping) then 0 (checked).
#[test]
fn param_shift_beyond_32_bits_folds_wide() {
    let out = run(r#"
module t;
  parameter [63:0] F = 1 << 32;
  parameter A = 1 << 32;
  initial $display("F=%0d A=%0d", F, A);
endmodule
"#);
    assert_eq!(out.trim(), "F=4294967296 A=4294967296");
}

/// P0-6: `>>>` on a negative parameter sign-extends (was: logical shift on
/// the wrapped u32 image → 0x7FFFFFFE).
#[test]
fn signed_param_ashr_sign_extends() {
    let out = run(r#"
module t;
  parameter S = -4;
  parameter T = S >>> 1;
  initial $display("T=%0d", T);
endmodule
"#);
    assert_eq!(out.trim(), "T=-2");
}

/// P0-6: a negative parameter prints as its signed value (iverilog parity);
/// the u32 domain bound it as an unsigned 32-bit image (4294967292).
#[test]
fn negative_param_displays_signed() {
    let out = run(r#"
module t;
  parameter N = -4;
  initial $display("N=%0d", N);
endmodule
"#);
    assert_eq!(out.trim(), "N=-4");
}

/// P0-7: a descending generate-for (`i >= 0; i = i - 1`) terminates and
/// produces the expected instances (was: unsigned `i>=0` always true →
/// E3009 unroll-cap error, design rejected).
#[test]
fn descending_genvar_loop_terminates() {
    let out = run(r#"
module t;
  wire [3:0] in_w, out_w;
  assign in_w = 4'b1010;
  genvar i;
  generate
    for (i = 3; i >= 0; i = i - 1) begin: g
      assign out_w[i] = in_w[i];
    end
  endgenerate
  initial begin
    #1 $display("out=%b", out_w);
  end
endmodule
"#);
    assert_eq!(out.trim(), "out=1010");
}

/// P0-7: a generate-for whose condition is false on entry (negative start)
/// unrolls zero times — no instances, no diagnostics.
#[test]
fn zero_trip_descending_genvar() {
    let out = run(r#"
module t;
  genvar i;
  generate
    for (i = -1; i >= 0; i = i - 1) begin: g
      wire dead;
    end
  endgenerate
  initial $display("ok");
endmodule
"#);
    assert_eq!(out.trim(), "ok");
}

/// P0-5: an unfoldable parameter value is a LOUD error — never a silent 0
/// (the 2026-06-05 BLOCKER#2 조치 that was specified but not implemented).
#[test]
fn unfoldable_param_is_error() {
    let (_out, diags) = run_with_diags(
        r#"
module t;
  parameter W = undeclared_xyz;
  initial $display("W=%0d", W);
endmodule
"#,
    );
    assert!(
        diags.iter().any(|d| d.starts_with("Error")),
        "expected an Error diagnostic for the unfoldable parameter, got: {diags:?}"
    );
}

/// P0-6: i64 overflow in a parameter expression is loud, not a wrapped value.
#[test]
fn param_overflow_is_error() {
    let (_out, diags) = run_with_diags(
        r#"
module t;
  parameter W = 64'd3037000500 * 64'd3037000500; // ~9.2233720368e18 > i64::MAX
  initial $display("W=%0d", W);
endmodule
"#,
    );
    assert!(
        diags.iter().any(|d| d.starts_with("Error")),
        "expected an Error diagnostic for the overflowing parameter, got: {diags:?}"
    );
}

/// Keep-green: enum label chains still fold (explicit value resets the
/// running counter).
#[test]
fn enum_label_chain_folds() {
    let out = run(r#"
module t;
  typedef enum { A = 5, B } e_t;
  initial $display("B=%0d", B);
endmodule
"#);
    assert_eq!(out.trim(), "B=6");
}
