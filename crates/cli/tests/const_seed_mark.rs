//! §5.2 Q-AESINK: the constant interpreter records what it fabricates.  A read of a 4-state local
//! or return variable no statement wrote answers a fabricated 0 (the i64 domain has no x), and
//! that read is what `Sidecars::const_seed_reads` lists.  The set is one fact about one
//! elaboration: empty means no fabricated 0 was read, so no constant of this design rests on one.
//! It is a fact about vita's own run, over the declarations as vita folded them, not about a
//! value or an IEEE run: `marks_over_*` are designs whose value is right, `unmarked_nonclaim_*` are
//! what it cannot say.  Only a declaration whose text says `[N:0]` lets a select write clear a
//! bit, and a live record re-declared in the flat env is cleared by no write.
//! No user-visible output moves: every pin here also fixes the exit code and the design's own
//! lines, which are PRE's.  The accessor is read here and by no product code (RULINGS-dr1 B2: the
//! W3056 reader was false on run-time calls and on right values).
//!
//! Teeth: `marks_*` are red on a tree with the accessor and no recording (K/r1/stub.patch),
//! `unmarked_*` green there.  Each doc comment carries the raw oracle lines: iverilog 13.0 +
//! vvp, sv2v v0.0.13 + iverilog, verilator 5.052.  `vita` = PRE = this tree.

use diag::{LogEvent, LogSink};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Quiet;
impl LogSink for Quiet {
    fn emit(&self, _: LogEvent) {}
}

/// The accessor, through the parse and the elaboration `vita` runs.  A refused design has
/// `(None, sidecars)`: the sidecars still answer.
fn seed_reads(src: &str) -> BTreeSet<elaborate::ConstSeedRead> {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let unit = su.expect("source unit");
    elaborate::elaborate_with_timescale(&unit, &Quiet, &BTreeMap::new(), -9)
        .1
        .const_seed_reads
}

/// `vita` on one design: the exit code and the design's own `V ` lines.
fn run(src: &str) -> (i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_csm_{}_{n}", std::process::id()));
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
    let lines: Vec<&str> = so.lines().filter(|l| l.starts_with("V ")).collect();
    (out.status.code().expect("exit code"), lines.join("\n"))
}

/// One design: exit code and value lines first (they are PRE's on both sides of a pin), then the
/// accessor, every entry's span naming its own declaration in `src`.
#[track_caller]
fn check(src: &str, rc: i32, value: &str, want: &[(&str, &str)]) {
    assert_eq!(run(src), (rc, value.to_string()), "rc and value lines");
    let reads = seed_reads(src);
    let got: Vec<(&str, &str)> = reads
        .iter()
        .map(|r| (r.function.as_str(), r.variable.as_str()))
        .collect();
    assert_eq!(got, want, "the accessor");
    for r in &reads {
        let (lo, hi) = r.decl_span;
        assert_eq!(&src[lo as usize..hi as usize], r.variable, "{r:?}");
    }
}

/// x2a: a path that leaves the return variable unwritten.
/// iverilog `refuses: error: Unable to evaluate parameter P value: top.fx(32'sd2)`
/// sv2v `refuses: error: Unable to evaluate parameter P value: top.fx(32'sd2)`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_x2a_return_fall_off() {
    check(
        r#"module top;
  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  localparam logic [3:0] P = fx(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("fx", "fx")],
    );
}

/// x2c: a 4-state local a path leaves unwritten.
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_x2c_local() {
    check(
        r#"module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// x2i: `integer` is 4-state.
/// iverilog `V P=x`
/// sv2v `V P=x`
/// verilator `V P=x`
/// vita `V P=0`
#[test]
fn marks_x2i_integer() {
    check(
        r#"module top;
  function automatic integer f(input int a);
    integer t;
    if (a == 1) t = 10;
    f = t;
  endfunction
  localparam integer P = f(2);
  initial begin #1 $display("V P=%0d", P); $finish; end
endmodule
"#,
        0,
        "V P=0",
        &[("f", "t")],
    );
}

/// x2l: one bit written, three still fabricated.
/// iverilog `V P=xxx1`
/// sv2v `V P=xxx1`
/// verilator `V P=xxx1`
/// vita `V P=0001`
#[test]
fn marks_x2l_bit_written() {
    check(
        r#"module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "t")],
    );
}

/// d10: an untyped parameter binds the fabricated return value.
/// iverilog `refuses: error: Unable to evaluate parameter P value: top.fx(32'sd2)`
/// sv2v `refuses: error: Unable to evaluate parameter P value: top.fx(32'sd2)`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_d10_untyped_parameter() {
    check(
        r#"module top;
  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  parameter P = fx(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("fx", "fx")],
    );
}

/// m19: the fabricated 0 decides a branch and a run-time repeat count.
/// iverilog `V n=2`
/// sv2v `V n=2`
/// verilator `V n=1`
/// vita `V n=1`
#[test]
fn marks_m19_compare() {
    check(
        r#"module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  int n;
  initial begin n = 0; repeat (fd(2)) n = n + 1; #1 $display("V n=%0d", n); $finish; end
endmodule
"#,
        0,
        "V n=1",
        &[("fd", "t")],
    );
}

/// a41: an initializer that declines, then a select write merges into a fabricated 0.
/// iverilog `V P=2`
/// sv2v `V P=2`
/// verilator `V P=2`
/// vita `V P=0`
#[test]
fn marks_a41_declined_initializer() {
    check(
        r#"module top;
  function automatic int f(); int t = int'(2.5); t[0] = 1'b0; f = t; endfunction
  localparam int P = f();
  initial begin #1 $display("V P=%0d", P); $finish; end
endmodule
"#,
        0,
        "V P=0",
        &[("f", "t")],
    );
}

/// q5_formal: the seeded value is stored in `A` and read again by a width fold.
/// iverilog `V A=xxxx Y=01`
/// sv2v `V A=xxxx Y=01`
/// verilator `refuses: %Error: left side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.`
/// vita `V A=0000 Y=03`
#[test]
fn marks_q5_stored_binding_in_a_formal_range() {
    check(
        r#"module top;
  function automatic logic [3:0] g(); logic [3:0] t; g = t; endfunction
  localparam logic [3:0] A = g();
  function automatic logic [7:0] h(input logic [A+1:0] a); return a; endfunction
  localparam logic [7:0] Y = h(8'hFF);
  initial begin #1 $display("V A=%b Y=%h", A, Y); $finish; end
endmodule
"#,
        0,
        "V A=0000 Y=03",
        &[("g", "t")],
    );
}

/// p01: two fabricated values cancel, the difference is what the design prints.
/// iverilog `V Q=0 P=2 R=0`
/// sv2v `V Q=x P=2 R=x`
/// verilator `V Q=x P=2 R=x`
/// vita `V Q=0 P=1 R=0`
#[test]
fn marks_p01_cancelling_pair() {
    check(
        r#"module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    t[0] = 1'b1;
    fx1 = t;
  endfunction
  localparam int Q = fd(2) - fx1(2);
  localparam int P = fd(2);
  localparam int R = P - fx1(2);
  initial begin #1 $display("V Q=%0d P=%0d R=%0d", Q, P, R); $finish; end
endmodule
"#,
        0,
        "V Q=0 P=1 R=0",
        &[("fd", "t"), ("fx1", "t")],
    );
}

/// A read of a 4-state local no statement wrote (M01, M14).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_local() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// The implicit return variable read at the end of the body (M04, M16).
/// iverilog `refuses: error: Unable to evaluate parameter P value: top.f()`
/// sv2v `refuses: error: Unable to evaluate parameter P value: top.f(1'sd0)`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_return_fall_off() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "f")],
    );
}

/// A select write clears its own bits and no others (M11).
/// iverilog `V P=xxx1`
/// sv2v `V P=xxx1`
/// verilator `V P=xxx1`
/// vita `V P=0001`
#[test]
fn marks_after_bit_write() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; t[0] = 1'b1; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "t")],
    );
}

/// A call restores the caller's record (M13).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0010`
#[test]
fn marks_caller_after_call() {
    check(
        r#"module top;
  function automatic int h(input int a); return a; endfunction
  function automatic logic [3:0] f(); logic [3:0] t; logic [3:0] u; u = h(2); f = t + u; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[("f", "t")],
    );
}

/// A read through the placement resolver (M15).
/// iverilog `V P=xx01`
/// sv2v `V P=xx01`
/// verilator `V P=xx01`
/// vita `V P=0001`
#[test]
fn marks_concat_read() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [1:0] t; f = {t, 2'd1}; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "t")],
    );
}

/// A loop body's declaration re-arms every bit: the second pass reads what the first pass's write no longer covers (M21).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_loop_redeclaration() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] r; r = 1; for (int i = 0; i < 2; i++) begin logic [3:0] t; if (i == 1) r = t; t[0 +: 4] = 4'd5; end f = r; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// Each activation has its own record (v7).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_recursion() {
    check(
        r#"module top;
  function automatic logic [3:0] f(input int n); logic [3:0] t; if (n > 0) t = f(n-1); return t; endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// `t += 1` reads `t` before it writes it (v13).
/// iverilog `refuses: assert: net_func_eval.cc:408: failed assertion lval_const`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0001`
#[test]
fn marks_compound_assignment() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; t += 1; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "t")],
    );
}

/// A package function's frame (v16).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_package_function() {
    check(
        r#"package p;
  function automatic logic [3:0] g(); logic [3:0] t; return t; endfunction
endpackage
module top;
  localparam logic [3:0] P = p::g();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("g", "t")],
    );
}

/// A local rebinding a package constant's name is the local (c66).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_package_constant_rebound_by_a_local() {
    check(
        r#"package p;
  localparam int t = 7;
  function automatic logic [3:0] f(); logic [3:0] t; return t; endfunction
endpackage
module top;
  localparam logic [3:0] P = p::f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// The class tables run the interpreter after `el.run`, on an accepted design: the snapshot comes after them (N1, M22).  A bare module function in a constraint is E3009 (h1), a package one runs.
/// iverilog `refuses: sorry: Constraint declarations not supported.`
/// sv2v `refuses: Parse error: missing expected 'endmodule'`
/// verilator `V ok=0 x=0`
/// vita `V ok=1 x=0`
#[test]
fn marks_a_read_in_a_class_table() {
    check(
        r#"package p;
  function automatic logic [3:0] g(input int a); logic [3:0] t; return t; endfunction
endpackage
module top;
  class C; rand bit [3:0] x; constraint c { x == p::g(1); } endclass
  initial begin C c; int ok; c = new; ok = c.randomize(); #1 $display("V ok=%0d x=%0d", ok, c.x); $finish; end
endmodule
"#,
        0,
        "V ok=1 x=0",
        &[("g", "t")],
    );
}

/// d1: provenance through a net width, no stored binding between the seed and the reader.
/// iverilog `refuses: error: Unable to evaluate parameter A value: top.g()`
/// sv2v `refuses: error: Unable to evaluate parameter A value: top.g(1'sd0)`
/// verilator `refuses: %Error: left side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.`
/// vita `V W=4`
#[test]
fn marks_net_width() {
    check(
        r#"module top;
  function automatic logic [3:0] g(); endfunction
  localparam logic [3:0] A = g();
  logic [A+3:0] d;
  localparam int W = $bits(d);
  initial begin #1 $display("V W=%0d", W); $finish; end
endmodule
"#,
        0,
        "V W=4",
        &[("g", "g")],
    );
}

/// d2: a control dependency, nothing the arm reads is marked and the mark is already set.
/// iverilog `refuses: error: Unable to evaluate parameter A value: top.g()`
/// sv2v `refuses: error: Unable to evaluate parameter A value: top.g(1'sd0)`
/// verilator `V then`
/// vita `V then`
#[test]
fn marks_generate_arm() {
    check(
        r#"module top;
  function automatic logic [3:0] g(); endfunction
  localparam logic [3:0] A = g();
  if (A == 0) begin : b
    initial begin #1 $display("V then"); $finish; end
  end else begin : c
    initial begin #1 $display("V else"); $finish; end
  end
endmodule
"#,
        0,
        "V then",
        &[("g", "g")],
    );
}

/// d12: a loop count.
/// iverilog `refuses: error: Unable to evaluate parameter A value: top.g()`
/// sv2v `refuses: error: Unable to evaluate parameter A value: top.g(1'sd0)`
/// verilator `rc=0, no line`
/// vita `V i=0`
#[test]
fn marks_generate_loop_count() {
    check(
        r#"module top;
  function automatic logic [3:0] g(); endfunction
  localparam logic [3:0] A = g();
  for (genvar i = 0; i < A + 1; i++) begin : u
    initial begin #1 $display("V i=%0d", i); end
  end
  initial #2 $finish;
endmodule
"#,
        0,
        "V i=0",
        &[("g", "g")],
    );
}

/// A local whose declared range does not fold has width 0 in `envw`: every bit is unwritten, and the read still answers 0 (MX3).
/// iverilog `V P=0xxx`
/// sv2v `V P=0xxx`
/// verilator `V P=0xxx`
/// vita `V P=0000`
#[test]
fn marks_unknown_width_local() {
    check(
        r#"module top;
  function automatic int gk(); return 2; endfunction
  function automatic logic [3:0] f(); logic [gk():0] t; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// The mask's top edge: 64 bits is every bit of the word (MX4).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_64_bit_local() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [63:0] t; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// Two reads of one declaration are one entry, two declarations of one name are two (M19).
/// iverilog `V P=xxxx`
/// sv2v `V P=xxxx`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_each_declaration_once() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] r; logic [3:0] u; r = u + u; begin logic [3:0] t; r = t; end begin logic [3:0] t; r = r + t; end f = r; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t"), ("f", "t"), ("f", "u")],
    );
}

/// The twin of `unmarked_type_parameter_local`: an override to a 4-state type.
/// iverilog `V P=xxxxxxxx`
/// sv2v `V P=xxxxxxxx`
/// verilator `V P=xxxxxxxx`
/// vita `V P=0`
#[test]
fn marks_type_parameter_override() {
    check(
        r#"module s #(parameter type T = int);
  function automatic logic [31:0] f(); T t; return t; endfunction
  localparam logic [31:0] P = f();
  initial begin #1 $display("V P=%0h", P); $finish; end
endmodule
module top;
  s #(.T(logic [31:0])) u();
endmodule
"#,
        0,
        "V P=0",
        &[("f", "t")],
    );
}

/// v1_loop0_w1: `t[0]` names no bit of `[8:1]`, so the top bit stays unwritten where the select write fills bit 0.
/// iverilog `V P=x1111111`
/// sv2v `V P=x1111111`
/// verilator `V P=x1111111`
/// vita `V P=11111111`
#[test]
fn marks_v1_offset_lsb_loop() {
    check(
        r#"module top;
  function automatic logic [7:0] f(); logic [8:1] t; for (int i = 0; i < 8; i++) t[i] = 1'b1; return t; endfunction
  localparam logic [7:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=11111111",
        &[("f", "t")],
    );
}

/// nz_mod_off: the same at row 40's value, a stored binding read by a concat.
/// iverilog `V q=xx11 l=xx111010`
/// sv2v `V q=xx11 l=xx111010`
/// verilator `V q=xx11 l=xx111010`
/// vita `V q=1111 l=11111010`
#[test]
fn marks_nz_offset_lsb_in_a_stored_binding() {
    check(
        r#"module top;
  function automatic logic [3:0] g(input int a); logic [5:2] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] Q = g(1);
  localparam logic [7:0] L = { Q, 4'hA };
  initial begin #1 $display("V q=%b l=%b", Q, L); $finish; end
endmodule
"#,
        0,
        "V q=1111 l=11111010",
        &[("g", "t")],
    );
}

/// k_off_ret: the return variable's declared range.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_k_offset_lsb_return() {
    check(
        r#"module top;
  function automatic logic [5:2] f(input int a);
    f[0] = 1'b1; f[1] = 1'b1; f[2] = 1'b1; f[3] = 1'b1;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "f")],
    );
}

/// s_lab_off: the same at row 76's sink: the arm depends on the fabricated bits.
/// iverilog `V arm=default`
/// sv2v `V arm=default`
/// verilator `V arm=default`
/// vita `V arm=label`
#[test]
fn marks_s_offset_lsb_at_a_case_label() {
    check(
        r#"module top;
  function automatic logic [3:0] g(input int a); logic [5:2] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  case (4'hF)
    g(1): initial $display("V arm=label");
    default: initial $display("V arm=default");
  endcase
  initial #1 $finish;
endmodule
"#,
        0,
        "V arm=label",
        &[("g", "t")],
    );
}

/// An ascending range: `t[0]` names no bit of `[1:4]`.
/// iverilog `V P=111x`
/// sv2v `V P=111x`
/// verilator `V P=111x`
/// vita `V P=1111`
#[test]
fn marks_ascending_range() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [1:4] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// `[-1:0]` has LSB index 0 and is ascending: `-1` is no decimal literal, so the text does not say `[N:0]`.
/// iverilog `V P=x1`
/// sv2v `V P=x1`
/// verilator `V P=11`
/// vita `V P=11`
#[test]
fn marks_negative_msb() {
    check(
        r#"module top;
  function automatic logic [1:0] f(); logic [-1:0] t; t[0] = 1'b1; t[1] = 1'b1; return t; endfunction
  localparam logic [1:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=11",
        &[("f", "t")],
    );
}

/// A folded bound: the LSB is a parameter.
/// iverilog `V P=xx11`
/// sv2v `V P=xx11`
/// verilator `V P=xx11`
/// vita `V P=1111`
#[test]
fn marks_parameter_lsb() {
    check(
        r#"module top;
  localparam int L = 2;
  function automatic logic [3:0] f(); logic [L+3:L] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// td_off: a typedef keeps its range.
/// iverilog `V P=xx11`
/// sv2v `V P=xx11`
/// verilator `V P=xx11`
/// vita `V P=1111`
#[test]
fn marks_typedef_offset_lsb() {
    check(
        r#"module top;
  typedef logic [5:2] o_t;
  function automatic logic [3:0] f(input int a); o_t t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// td_ret_off: a typedef return.
/// iverilog `V P=xx11`
/// sv2v `V P=xx11`
/// verilator `V P=xx11`
/// vita `V P=1111`
#[test]
fn marks_typedef_offset_lsb_return() {
    check(
        r#"module top;
  typedef logic [5:2] o_t;
  function automatic o_t f(input int a); f[0] = 1'b1; f[1] = 1'b1; f[2] = 1'b1; f[3] = 1'b1; endfunction
  localparam logic [3:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "f")],
    );
}

/// en_off: an enum base type.
/// iverilog `V P=xx11`
/// sv2v `V P=xx11`
/// verilator `V P=xx11`
/// vita `V P=1111`
#[test]
fn marks_enum_offset_lsb() {
    check(
        r#"module top;
  typedef enum logic [5:2] {A, B} e_t;
  function automatic logic [3:0] f(input int a); e_t t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// tp_off_ov: an override's LSB reaches no field: the declaration reads `[T$w-1:0]`, so the carrier name is the only sign.
/// iverilog `V P=xx11`
/// sv2v `V P=xx11`
/// verilator `V P=xx11`
/// vita `V P=1111`
#[test]
fn marks_type_parameter_offset_lsb() {
    check(
        r#"module s #(parameter type T = logic [3:0]);
  function automatic logic [3:0] f(input int a); T t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
module top; s #(.T(logic [5:2])) u(); endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// tp_ret_off: a type-parameter return.
/// iverilog `V P=xx11`
/// sv2v `V P=xx11`
/// verilator `V P=xx11`
/// vita `V P=1111`
#[test]
fn marks_type_parameter_offset_lsb_return() {
    check(
        r#"module s #(parameter type T = logic [5:2]);
  function automatic T f(input int a); f[0] = 1'b1; f[1] = 1'b1; f[2] = 1'b1; f[3] = 1'b1; endfunction
  localparam logic [3:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
module top; s u(); endmodule
"#,
        0,
        "V P=1111",
        &[("f", "f")],
    );
}

/// u6: an unpacked local acts as one vector, so the select writes cover a mask no element has.
/// iverilog `V P=000000001111xxxx`
/// sv2v `V P=000000001111xxxx`
/// verilator `V P=0000000011111111`
/// vita `V P=0000000000000011`
#[test]
fn marks_unpacked_local() {
    check(
        r#"module top;
  function automatic logic [15:0] f(); logic [3:0] m [2]; m[0] = 4'd15; m[1] = 4'd15; m[2] = 4'd15; m[3] = 4'd15; return {m[0], m[3]}; endfunction
  localparam logic [15:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000000000000011",
        &[("f", "m")],
    );
}

/// Extra packed dimensions: the select writes number the product, not the elements (k_md_b's shape, seeded).
/// iverilog `refuses: assert: elab_lval.cc:611: failed assertion rcl`
/// sv2v `V P=0101`
/// verilator `V P=0101`
/// vita `V P=1111`
#[test]
fn marks_packed_dimensions() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [1:0][1:0] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// gl_pkg_shad: a 2-state shadow zeroes nothing it can vouch for: row 76's sink.
/// iverilog `V arm=default`
/// sv2v `V arm=default`
/// verilator `V arm=default`
/// vita `V arm=label`
#[test]
fn marks_gl_shadow_at_a_case_label() {
    check(
        r#"package q;
  function automatic logic [3:0] g(input int a); logic [3:0] t; begin int t; end return t; endfunction
endpackage
module top;
  case (4'd0)
    q::g(1): initial $display("V arm=label");
    default: initial $display("V arm=default");
  endcase
  initial #1 $finish;
endmodule
"#,
        0,
        "V arm=label",
        &[("g", "t")],
    );
}

/// nz_pkg_shad: the same at row 40's value.
/// iverilog `V q=xxxx l=xxxx1010`
/// sv2v `V q=xxxx l=xxxx1010`
/// verilator `V q=xxxx l=xxxx1010`
/// vita `V q=0000 l=00001010`
#[test]
fn marks_nz_shadow_in_a_stored_binding() {
    check(
        r#"package q;
  function automatic logic [3:0] g(input int a); logic [3:0] t; begin int t; end return t; endfunction
endpackage
module top;
  localparam logic [3:0] Q = q::g(1);
  localparam logic [7:0] L = { Q, 4'hA };
  initial begin #1 $display("V q=%b l=%b", Q, L); $finish; end
endmodule
"#,
        0,
        "V q=0000 l=00001010",
        &[("g", "t")],
    );
}

/// j_pkg_in2s_after.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000000`
#[test]
fn marks_j_two_state_shadow() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t; begin int t; end f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("f", "t")],
    );
}

/// j_pkg_in4s_w_after: the write lands on the shared name and ends nothing.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000101`
#[test]
fn marks_j_shadow_written() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t; begin logic [3:0] t; t = 5; end f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000101",
        &[("f", "t")],
    );
}

/// j_pkg_in_init_after: a folded initializer in the shadow.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000011`
#[test]
fn marks_j_shadow_initializer() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t; begin logic [3:0] t = 4'd3; end f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000011",
        &[("f", "t")],
    );
}

/// A declined initializer in the shadow: the arm that seeds.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000101`
#[test]
fn marks_j_shadow_declined_initializer() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t; begin logic [3:0] t = int'(2.5); t = 5; end f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000101",
        &[("f", "t")],
    );
}

/// j_pkg_ret_int: the return variable's record, in a package.
/// iverilog `refuses: error: Unable to evaluate parameter P value`
/// sv2v `refuses: error: Unable to evaluate parameter P`
/// verilator `V P=xxxxxxxx`
/// vita `V P=00000000`
#[test]
fn marks_j_return_variable_shadow() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    begin int f; end
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("f", "f")],
    );
}

/// s12: the same in a module.
/// iverilog `refuses: error: Unable to evaluate parameter P val`
/// sv2v `refuses: error: Unable to evaluate parameter`
/// verilator `V P=xxxx`
/// vita `V P=0000`
#[test]
fn marks_s12_return_variable_shadow() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); begin int f; end endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "f")],
    );
}

/// c80_shadow_ret: the shadow is written, the return variable is not.
/// iverilog `refuses: error: Unable to evaluate paramet`
/// sv2v `V P=0001`
/// verilator `V P=xxxx`
/// vita `V P=0001`
#[test]
fn marks_c80_return_variable_shadow_written() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); begin logic [3:0] f; f = 4'd1; end endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "f")],
    );
}

/// A shadowed record stays shadowed through a same-span re-seed: pass 1 moves the record to the inner declaration, pass 2 is a re-entry of it.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000101`
#[test]
fn marks_shadow_survives_a_loop_re_entry() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t; for (int i = 0; i < 2; i++) begin logic [3:0] t; t = 5; end f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000101",
        &[("f", "t")],
    );
}

/// w25: an argument folds against the caller's record, so the frame swap stays below it (sC2).
/// iverilog `V P=00000101`
/// sv2v `V P=00000101`
/// verilator `V P=00000101`
/// vita `V P=00000101`
#[test]
fn marks_seed_as_an_actual_argument() {
    check(
        r#"module top;
  function automatic logic [3:0] h(input logic [3:0] a); return 4'd5; endfunction
  function automatic logic [7:0] f(); logic [3:0] t; return h(t); endfunction
  localparam logic [7:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000101",
        &[("f", "t")],
    );
}

/// a41 on a declaration whose text is not `[N:0]`: the seed is not `numbered` in the declined arm either.
/// iverilog `V P=0011`
/// sv2v `V P=0011`
/// verilator `V P=0011`
/// vita `V P=1111`
#[test]
fn marks_declined_initializer_offset_lsb() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [5:2] t = int'(2.5); t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1111",
        &[("f", "t")],
    );
}

/// Over-mark: the select's constant probe runs the interpreter on a run-time call and discards the answer (a01, RULINGS-dr1 B2).  This is why the accessor is a fact about the elaboration and not about a value.
/// iverilog `V x`
/// sv2v `V x`
/// verilator `V 0`
/// vita `V x`
#[test]
fn marks_over_a_run_time_select_probe() {
    check(
        r#"module top;
  function automatic logic [3:0] g(input int a); logic [3:0] t; return t; endfunction
  logic [15:0] v = 16'h1234;
  initial begin #1 $display("V %b", v[g(1)]); $finish; end
endmodule
"#,
        0,
        "V x",
        &[("g", "t")],
    );
}

/// Over-mark: a whole-variable mask reads a written bit as unwritten (c39).
/// iverilog `V P=0001`
/// sv2v `V P=0001`
/// verilator `V P=0001`
/// vita `V P=0001`
#[test]
fn marks_over_a_written_bit() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; t[0] = 1'b1; return {3'b0, t[0]}; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "t")],
    );
}

/// Over-mark: `t & 0` is 0 by IEEE (e7): the mark says a seed was read, not that the value is wrong.
/// iverilog `V P=0000`
/// sv2v `V P=0000`
/// verilator `V P=0000`
/// vita `V P=0000`
#[test]
fn marks_over_a_masked_read() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; return t & 4'd0; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// A whole write clears every bit whatever the numbering: vita's word is written (was `marks_over_offset_whole_write`).
/// iverilog `V P=0101`
/// sv2v `V P=0101`
/// verilator `V P=0101`
/// vita `V P=0101`
#[test]
fn unmarked_offset_whole_write() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [4:1] t; t = 4'd5; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0101",
        &[],
    );
}

/// The same through a type parameter: `T` hides the numbering, a whole write needs none (was `marks_over_type_parameter_whole_write`).
/// iverilog `V P=0101`
/// sv2v `V P=0101`
/// verilator `V P=0101`
/// vita `V P=0101`
#[test]
fn unmarked_type_parameter_whole_write() {
    check(
        r#"module s #(parameter type T = logic [3:0]);
  function automatic logic [3:0] f(input int a); T t; t = 4'd5; return t; endfunction
  localparam logic [3:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
module top; s u(); endmodule
"#,
        0,
        "V P=0101",
        &[],
    );
}

/// Over-mark: a live record re-declared by a sibling block, then a folded initializer (was `unmarked_initializer_after_a_sibling_seed`).
/// iverilog `V P=0001`
/// sv2v `V P=0001`
/// verilator `V P=0001`
/// vita `V P=0001`
#[test]
fn marks_over_initializer_after_a_live_sibling() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); begin logic [3:0] t; end begin logic [3:0] t = 4'd1; f = t; end endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[("f", "t")],
    );
}

/// Over-mark: a live record re-declared 2-state (MX1 of r1, reversed by RULINGS-lens-r1 R1): a miss sits at a sink, an over-mark costs a reader one loud.
/// iverilog `V P=0000`
/// sv2v `V P=xxxx`
/// verilator `V P=0000`
/// vita `V P=0000`
#[test]
fn marks_over_two_state_redeclaration() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] r; r = 1; begin logic [3:0] t; end begin int t; r = t; end f = r; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[("f", "t")],
    );
}

/// Over-mark: `enum bit` is recorded as 4-state `logic` (k42).
/// iverilog `V P=00000000000000000000000000000000`
/// sv2v `V P=000000000000000000000000000000xx`
/// verilator `V P=00000000000000000000000000000000`
/// vita `V P=00000000000000000000000000000000`
#[test]
fn marks_over_enum_bit_local() {
    check(
        r#"module top;
  typedef enum bit [1:0] {A, B} e_t;
  function automatic logic [31:0] f(); e_t t; return t; endfunction
  localparam logic [31:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000000000000000000000000000",
        &[("f", "t")],
    );
}

/// Over-mark: the same on a return (k45).
/// iverilog `refuses: error: Unable to evaluate parameter P value: <s`
/// sv2v `refuses: error: Unable to evaluate parameter P valu`
/// verilator `V P=00000000000000000000000000000000`
/// vita `V P=00000000000000000000000000000000`
#[test]
fn marks_over_enum_bit_return() {
    check(
        r#"module top;
  typedef enum bit [1:0] {A, B} e_t;
  function automatic e_t f(); endfunction
  localparam logic [31:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000000000000000000000000000",
        &[("f", "f")],
    );
}

/// Over-mark: `(0 && g(1))`: IEEE does not evaluate the operand, the fold runs it (e_discard_land).
/// iverilog `V P=0010`
/// sv2v `V P=0010`
/// verilator `V P=0010`
/// vita `V P=0010`
#[test]
fn marks_over_a_discarded_operand() {
    check(
        r#"module top;
  function automatic logic [3:0] g(input int a); logic [3:0] t; g = t; endfunction
  localparam logic [3:0] P = (0 && g(1)) ? 4'd1 : 4'd2;
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[("g", "t")],
    );
}

/// Over-mark: a return with packed dimensions, every element written.
/// iverilog `V P=1001`
/// sv2v `V P=1001`
/// verilator `V P=1001`
/// vita `V P=1001`
#[test]
fn marks_over_packed_return() {
    check(
        r#"module top;
  function automatic logic [1:0][1:0] f(); f[0] = 2'b01; f[1] = 2'b10; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1001",
        &[("f", "f")],
    );
}

/// A bound holding a call: a whole write needs no numbering, so it ends the record (was `marks_over_call_bound_whole_write`, the family of 28 of the 4,820 rows).
/// iverilog `V P=0101`
/// sv2v `V P=0101`
/// verilator `V P=0101`
/// vita `V P=0101`
#[test]
fn unmarked_call_bound_whole_write() {
    check(
        r#"module top;
  function automatic int w(input int a); return 3; endfunction
  function automatic logic [3:0] f(); logic [w(2):0] t; t = 4'd5; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0101",
        &[],
    );
}

/// IEEE's own 0: a 2-state local (M02).
/// iverilog `V P=0000`
/// sv2v `V P=xxxx`
/// verilator `V P=0000`
/// vita `V P=0000`
#[test]
fn unmarked_int_local() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); int t; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[],
    );
}

/// IEEE's own 0: a 2-state return variable (M05).
/// iverilog `refuses: error: Unable to evaluate parameter P value: top.f()`
/// sv2v `refuses: error: Unable to evaluate parameter P value: top.f(1'sd0)`
/// verilator `V P=0000`
/// vita `V P=0000`
#[test]
fn unmarked_int_return() {
    check(
        r#"module top;
  function automatic int f(); endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[],
    );
}

/// A 2-state packed local.
/// iverilog `V P=0000`
/// sv2v `V P=xxxx`
/// verilator `V P=0000`
/// vita `V P=0000`
#[test]
fn unmarked_bit_local() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); bit [3:0] t; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[],
    );
}

/// A typedef'd 2-state return (v2).
/// iverilog `refuses: error: Unable to evaluate parameter P value: top.f()`
/// sv2v `refuses: error: Unable to evaluate parameter P value: top.f(1'sd0)`
/// verilator `V P=0`
/// vita `V P=0`
#[test]
fn unmarked_two_state_typedef() {
    check(
        r#"module top;
  typedef int myint_t;
  function automatic myint_t f(); endfunction
  localparam int P = f();
  initial begin #1 $display("V P=%0d", P); $finish; end
endmodule
"#,
        0,
        "V P=0",
        &[],
    );
}

/// A typedef'd 2-state local (v3).
/// iverilog `V P=0000`
/// sv2v `V P=xxxx`
/// verilator `V P=0000`
/// vita `V P=0000`
#[test]
fn unmarked_typedef_bit_local() {
    check(
        r#"module top;
  typedef bit [3:0] b4_t;
  function automatic logic [3:0] f(); b4_t t; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0000",
        &[],
    );
}

/// A type parameter's default is 2-state: the kind is the shape's, not the keyword's (v6, MX5).
/// iverilog `V P=0`
/// sv2v `V P=xxxxxxxx`
/// verilator `V P=0`
/// vita `V P=0`
#[test]
fn unmarked_type_parameter_local() {
    check(
        r#"module s #(parameter type T = int);
  function automatic logic [31:0] f(); T t; return t; endfunction
  localparam logic [31:0] P = f();
  initial begin #1 $display("V P=%0h", P); $finish; end
endmodule
module top;
  s u();
endmodule
"#,
        0,
        "V P=0",
        &[],
    );
}

/// s_partial_cover_idx: two select writes cover every bit (M03, M10, M17, M20).  381 of 1,339 RTL functions select-write this way.
/// iverilog `V P=1001`
/// sv2v `V P=1001`
/// verilator `V P=1001`
/// vita `V P=1001`
#[test]
fn unmarked_covering_bit_builder() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; t[0 +: 2] = 2'b01; t[2 +: 2] = 2'b10; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1001",
        &[],
    );
}

/// A whole write (M09).
/// iverilog `V P=0001`
/// sv2v `V P=0001`
/// verilator `V P=0001`
/// vita `V P=0001`
#[test]
fn unmarked_whole_write() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] t; t = 4'd1; f = t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[],
    );
}

/// A callee does not see its caller's record (M12).
/// iverilog `V P=0010`
/// sv2v `V P=0010`
/// verilator `V P=0010`
/// vita `V P=0010`
#[test]
fn unmarked_frame_isolated() {
    check(
        r#"module top;
  function automatic int h(input int t); return t; endfunction
  function automatic logic [3:0] f(); logic [3:0] t; f = h(2); endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[],
    );
}

/// A local of the function's own name, folded: the return variable was not fabricated (C1, MX2).
/// iverilog `V P=0001`
/// sv2v `V P=0001`
/// verilator `V P=0001`
/// vita `rc=1, no V line`
#[test]
fn unmarked_return_name_bound() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] f = 4'd1; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        1,
        "",
        &[],
    );
}

/// A function nothing calls (v19).  The guard is the interpreter's entry, a call: no line of this
/// row can run an uncalled function, so no mutant of it dies here.  It keeps a declaration-time
/// seed read out.
/// iverilog `V P=0010`
/// sv2v `V P=0010`
/// verilator `V P=0010`
/// vita `V P=0010`
#[test]
fn unmarked_uncalled() {
    check(
        r#"module top;
  function automatic logic [3:0] g(); logic [3:0] t; return t; endfunction
  localparam logic [3:0] P = 4'd2;
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[],
    );
}

/// A constant `?:` does not evaluate its dead arm (v17): it keeps a dead-arm seed read out.
/// iverilog `V P=0010`
/// sv2v `V P=0010`
/// verilator `V P=0010`
/// vita `V P=0010`
#[test]
fn unmarked_discarded_arm() {
    check(
        r#"module top;
  function automatic logic [3:0] g(); logic [3:0] t; return t; endfunction
  localparam logic [3:0] P = 1 ? 4'd2 : g();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[],
    );
}

/// A call only the run-time evaluates (v20): it keeps a run-time call out of the interpreter's record.
/// iverilog `V r=x`
/// sv2v `V r=x`
/// verilator `V r=0`
/// vita `V r=x`
#[test]
fn unmarked_run_time_call() {
    check(
        r#"module top;
  function automatic logic [3:0] g(input int a); logic [3:0] t; return t; endfunction
  int k = 1;
  initial begin #1 $display("V r=%h", g(k)); $finish; end
endmodule
"#,
        0,
        "V r=x",
        &[],
    );
}

/// A loop body's declaration is a re-entry, not a shadow: pass 1 leaves its record live, pass 2 writes before it reads.
/// iverilog `V P=0011`
/// sv2v `V P=0011`
/// verilator `V P=0011`
/// vita `V P=0011`
#[test]
fn unmarked_same_span_loop_keep() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] r; r = 0; for (int i = 0; i < 2; i++) begin logic [3:0] t; if (i == 1) begin t = 4'd3; r = t; end end f = r; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0011",
        &[],
    );
}

/// A whole write ended the record, so the next block's 2-state declaration has nothing to shadow.
/// iverilog `V P=0010`
/// sv2v `V P=0010`
/// verilator `V P=0010`
/// vita `V P=0010`
#[test]
fn unmarked_redeclaration_no_live_record() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] r; begin logic [3:0] t; t = 4'd1; end begin int t; t = 2; r = t; end f = r; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[],
    );
}

/// Every bit written: the record has mask 0, it is not live, so the sibling declaration does not shadow it and its own whole write ends its seed.
/// iverilog `V P=0010`
/// sv2v `V P=0010`
/// verilator `V P=0010`
/// vita `V P=0010`
#[test]
fn unmarked_redeclaration_of_a_covered_record() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); begin logic [3:0] t; t[0 +: 4] = 4'd1; end begin logic [3:0] t; t = 4'd2; f = t; end endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[],
    );
}

/// Over-mark: a name in a bound proves no numbering, whatever it folds to: a select write clears nothing, value right (was `unmarked_parameter_range_cover`).
/// iverilog `V P=1001`
/// sv2v `V P=1001`
/// verilator `V P=1001`
/// vita `V P=1001`
#[test]
fn marks_over_parameter_range_cover() {
    check(
        r#"module top;
  localparam int W = 4;
  function automatic logic [3:0] f(); logic [W-1:0] t; t[0 +: 2] = 2'b01; t[2 +: 2] = 2'b10; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1001",
        &[("f", "t")],
    );
}

/// Over-mark: the same through `$clog2`: the text is not `[N:0]` (was `unmarked_clog2_range_cover`).
/// iverilog `V P=1001`
/// sv2v `V P=1001`
/// verilator `V P=1001`
/// vita `V P=1001`
#[test]
fn marks_over_clog2_range_cover() {
    check(
        r#"module top;
  localparam int N = 16;
  function automatic logic [3:0] f(); logic [$clog2(N)-1:0] t; t[0 +: 2] = 2'b01; t[2 +: 2] = 2'b10; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1001",
        &[("f", "t")],
    );
}

/// No range is a scalar's own numbering.
/// iverilog `V P=0001`
/// sv2v `V P=0001`
/// verilator `V P=0001`
/// vita `V P=0001`
#[test]
fn unmarked_scalar_whole_write() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic t; t = 1'b1; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0001",
        &[],
    );
}

/// An atom has no range either.
/// iverilog `V P=0101`
/// sv2v `V P=0101`
/// verilator `V P=0101`
/// vita `V P=0101`
#[test]
fn unmarked_integer_cover() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); integer t; t[0 +: 32] = 32'd5; return t; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0101",
        &[],
    );
}

/// A `[3:0]` return built bit by bit.
/// iverilog `V P=1001`
/// sv2v `V P=1001`
/// verilator `V P=1001`
/// vita `V P=1001`
#[test]
fn unmarked_return_cover() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); f[0 +: 2] = 2'b01; f[2 +: 2] = 2'b10; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=1001",
        &[],
    );
}

/// k18: the shape's kind, not the keyword: the real killer of MX5 (sC3).
/// iverilog `V P=00000000`
/// sv2v `V P=xxxxxxxx`
/// verilator `V P=00000000`
/// vita `V P=00000000`
#[test]
fn unmarked_type_parameter_override_to_two_state() {
    check(
        r#"module s #(parameter type T = logic [31:0]);
  function automatic logic [63:0] f(); T t; return t; endfunction
  localparam logic [31:0] P = f();
  initial begin #1 $display("V P=%h", P); $finish; end
endmodule
module top;
  s #(.T(int)) u();
endmodule
"#,
        0,
        "V P=00000000",
        &[],
    );
}

/// A range bound holding a call is not folded by the numbering test either: `f` inside `f` would recurse past the stack.
/// iverilog `refuses: assert: elab_expr.cc:2927: failed assertion def`
/// sv2v `refuses: assert: elab_expr.cc:2927: failed assertion def`
/// verilator `refuses: %Error: internal fault, sorry. Suggest trying --debug --gdbbt`
/// vita `V P=0010`
#[test]
fn unmarked_range_calling_its_own_function() {
    check(
        r#"module top;
  function automatic logic [3:0] f(input int n); bit [f(1):0] t; return 4'd2; endfunction
  localparam logic [3:0] P = f(0);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0010",
        &[],
    );
}

/// A loop body's initializer declines on pass 1 (a41, a record) and folds on pass 2: the same declaration, so the fold is a write and ends the record.
/// iverilog `V P=0101`
/// sv2v `V P=0101`
/// verilator `V P=0101`
/// vita `V P=0101`
#[test]
fn unmarked_initializer_folds_on_a_later_pass() {
    check(
        r#"module top;
  function automatic logic [3:0] f(); logic [3:0] r; r = 4'd0; for (int i = 0; i < 2; i++) begin logic [3:0] t = i == 0 ? int'(2.5) : 4'd5; if (i == 1) r = t; end return r; endfunction
  localparam logic [3:0] P = f();
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=0101",
        &[],
    );
}

/// A select-built local, LSB a name: the fold answers at the call (`L = 0`), the declaration's scope says `[5:2]`.  The text is not `[N:0]`, so the select writes clear nothing.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_named_lsb_at_the_call() {
    check(
        r#"module top;
  localparam L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t;
    t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; f = t;
  endfunction
  if (1) begin : g
    localparam L = 0;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// A select-built local, MSB a name: `[W-1:0]`, home `W = 8`, call `W = 4`: vita's word is 4 bits and fully written, the numbering is not the text's, so the select writes clear nothing.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_named_width_at_the_call() {
    check(
        r#"module top;
  localparam W = 8;
  function automatic logic [7:0] f(input int a);
    logic [W-1:0] t;
    t[0 +: 4] = 4'hF; f = t;
  endfunction
  if (1) begin : g
    localparam W = 4;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// The return variable, LSB a name: Z2's numbering is the text too.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_return_named_lsb_at_the_call() {
    check(
        r#"module top;
  localparam L = 2;
  function automatic logic [L+3:L] f(input int a); f[0] = 1'b1; f[1] = 1'b1; f[2] = 1'b1; f[3] = 1'b1; endfunction
  if (1) begin : g
    localparam L = 0;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "f")],
    );
}

/// The return variable, MSB a name.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_return_named_width_at_the_call() {
    check(
        r#"module top;
  localparam W = 8;
  function automatic logic [W-1:0] f(input int a); f[0 +: 4] = 4'hF; endfunction
  if (1) begin : g
    localparam W = 4;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "f")],
    );
}

/// A typedef's LSB: the range the local takes holds a name.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_typedef_lsb_at_the_call() {
    check(
        r#"module top;
  localparam L = 2; typedef logic [L+3:L] n_t;
  function automatic logic [7:0] f(input int a);
    n_t t;
    t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; f = t;
  endfunction
  if (1) begin : g
    localparam L = 0;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// A typedef's MSB.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_typedef_width_at_the_call() {
    check(
        r#"module top;
  localparam W = 8; typedef logic [W-1:0] n_t;
  function automatic logic [7:0] f(input int a);
    n_t t;
    t[0 +: 4] = 4'hF; f = t;
  endfunction
  if (1) begin : g
    localparam W = 4;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// An imported name in the LSB, a generate shadow at the call.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_imported_lsb_at_the_call() {
    check(
        r#"package p; localparam L = 2; endpackage
module top;
  import p::*;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t;
    t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; f = t;
  endfunction
  if (1) begin : g
    localparam L = 0;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// `$clog2` in the MSB, a generate shadow at the call.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_clog2_width_at_the_call() {
    check(
        r#"module top;
  localparam D = 256;
  function automatic logic [7:0] f(input int a);
    logic [$clog2(D)-1:0] t;
    t[0 +: 4] = 4'hF; f = t;
  endfunction
  if (1) begin : g
    localparam D = 16;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// A `$unit` routine, LSB a name, a module shadow at the call.  sv2v's flattened `$unit` reads the module's name.
/// iverilog `V P=0000xx11`
/// sv2v `V P=00001111`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_unit_lsb() {
    check(
        r#"localparam L = 2;
function automatic logic [7:0] f(input int a);
    logic [L+3:L] t;
    t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; f = t;
  endfunction
module top;
  localparam L = 0;
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// A `$unit` routine, MSB a name.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=00001111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_unit_width() {
    check(
        r#"localparam W = 8;
function automatic logic [7:0] f(input int a);
    logic [W-1:0] t;
    t[0 +: 4] = 4'hF; f = t;
  endfunction
module top;
  localparam W = 4;
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// A `$unit` routine's return variable, MSB a name.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=00001111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_unit_return_width() {
    check(
        r#"localparam W = 8;
function automatic logic [W-1:0] f(input int a); f[0 +: 4] = 4'hF; endfunction
module top;
  localparam W = 4;
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "f")],
    );
}

/// An instance override folds the call at the generate's scope.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=00000000`
/// vita `V P=00001111`
#[test]
fn marks_instance_override_at_the_call() {
    check(
        r#"module c #(parameter logic [7:0] Q = 0) ();
  initial begin #1 $display("V P=%b", Q); $finish; end
endmodule
module top;
  localparam L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t;
    t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; f = t;
  endfunction
  if (1) begin : g
    localparam L = 0;
    c #(.Q(f(2))) u();
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// The call at a generate case label.
/// iverilog `V arm=default`
/// sv2v `V arm=default`
/// verilator `V arm=default`
/// vita `V arm=label`
#[test]
fn marks_label_at_the_call() {
    check(
        r#"module top;
  localparam int L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t; t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1; return t;
  endfunction
  if (1) begin : g
    localparam int L = 0;
    case (8'h0f)
      f(1): initial begin $display("V arm=label"); $finish; end
      default: initial begin $display("V arm=default"); $finish; end
    endcase
  end
endmodule
"#,
        0,
        "V arm=label",
        &[("f", "t")],
    );
}

/// The call in a generate condition.
/// iverilog `V arm=else`
/// sv2v `V arm=else`
/// verilator `V arm=then`
/// vita `V arm=then`
#[test]
fn marks_generate_condition_at_the_call() {
    check(
        r#"module top;
  localparam int L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t; t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1; return t;
  endfunction
  if (1) begin : g
    localparam int L = 0;
    if (f(1) == 8'h0f) initial begin $display("V arm=then"); $finish; end
    else initial begin $display("V arm=else"); $finish; end
  end
endmodule
"#,
        0,
        "V arm=then",
        &[("f", "t")],
    );
}

/// The call in a net's width.
/// iverilog `V P=00000001`
/// sv2v `V P=xxxxxxxx`
/// verilator `refuses: %Error: wd_gen_shad.sv:8:12: left side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1`
/// vita `V P=00010000`
#[test]
fn marks_width_sink_at_the_call() {
    check(
        r#"module top;
  localparam int L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t; t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1; return t;
  endfunction
  if (1) begin : g
    localparam int L = 0;
    logic [f(1):0] w;
    localparam logic [7:0] P = $bits(w);
    initial begin $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00010000",
        &[("f", "t")],
    );
}

/// A case label's call, MSB a name.
/// iverilog `V arm=default`
/// sv2v `V arm=default`
/// verilator `V arm=default`
/// vita `V arm=label`
#[test]
fn marks_label_named_width() {
    check(
        r#"module top;
  localparam W = 8;
  function automatic logic [7:0] g(input int a); logic [W-1:0] t; t[0 +: 4] = 4'hF; return t; endfunction
  if (1) begin : b
    localparam W = 4;
    case (8'h0F)
      g(1): initial $display("V arm=label");
      default: initial $display("V arm=default");
    endcase
  end
  initial #1 $finish;
endmodule
"#,
        0,
        "V arm=label",
        &[("g", "t")],
    );
}

/// A net width's call, LSB a name.
/// iverilog `V w=1`
/// sv2v `V w=x`
/// verilator `refuses: %Error: s_w.sv:6:12: left side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1)`
/// vita `V w=16`
#[test]
fn marks_bound_sink_named_lsb() {
    check(
        r#"module top;
  localparam L = 2;
  function automatic logic [7:0] g(input int a); logic [L+3:L] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction
  if (1) begin : b
    localparam L = 0;
    logic [g(1):0] v;
    initial begin #1 $display("V w=%0d", $bits(v)); $finish; end
  end
endmodule
"#,
        0,
        "V w=16",
        &[("g", "t")],
    );
}

/// A negative MSB by name, `[H:0]`, home `H = -3`, call `H = 3`.
/// iverilog `V P=0000xxx1`
/// sv2v `V P=0000xxx1`
/// verilator `V P=00001111`
/// vita `V P=00001111`
#[test]
fn marks_negative_msb_at_the_call() {
    check(
        r#"module top;
  localparam int H = -3;
  function automatic logic [7:0] f(input int a);
    logic [H:0] t; t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1; return t;
  endfunction
  if (1) begin : g
    localparam int H = 3;
    localparam logic [7:0] P = f(1);
    initial begin $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// C2: the return variable's LSB holds a call: no window, no fold: the text is not `[N:0]`.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_return_lsb_call() {
    check(
        r#"module top;
  function automatic int g(); return 2; endfunction
  function automatic logic [g()+3:g()] f(input int a);
    f[0]=1'b1; f[1]=1'b1; f[2]=1'b1; f[3]=1'b1;
  endfunction
  localparam logic [7:0] P = f(0);
  initial begin $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "f")],
    );
}

/// C2: a package routine's LSB a name beside a same-named module parameter.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_package_lsb_shadow() {
    check(
        r#"package q;
  localparam int L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t; t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1; return t;
  endfunction
endpackage
module top;
  localparam int L = 0;
  localparam logic [7:0] P = q::f(1);
  initial begin $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// C2: the same on the return variable.
/// iverilog `V P=0000xx11`
/// sv2v `V P=0000xx11`
/// verilator `V P=0000xx11`
/// vita `V P=00001111`
#[test]
fn marks_package_return_lsb_shadow() {
    check(
        r#"package q;
  localparam int L = 2;
  function automatic logic [L+3:L] f(input int a);
    f[0]=1'b1; f[1]=1'b1; f[2]=1'b1; f[3]=1'b1;
  endfunction
endpackage
module top;
  localparam int L = 0;
  localparam logic [7:0] P = q::f(1);
  initial begin $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "f")],
    );
}

/// R14: `r.shadowed |=`: a re-declaration over a written outer: the record is the inner's, a select write of an offset declaration.  A true mark of vita's run: the flat env.
/// iverilog `V P=00000001`
/// sv2v `V P=00000001`
/// verilator `V P=00000001`
/// vita `V P=00001111`
#[test]
fn marks_redeclared_offset_over_a_written_outer() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    t = 8'd1;
    begin logic [5:2] t; t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; end
    f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// A shadowed record: a select cover clears nothing.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000101`
#[test]
fn marks_shadow_select_written() {
    check(
        r#"package q;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t;
    begin logic [3:0] t; t[0 +: 4] = 4'd5; end
    f = t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000101",
        &[("f", "t")],
    );
}

/// The accounting: a bit written twice is one write of one bit, the record stays live.
/// iverilog `V P=0000xxx0`
/// sv2v `V P=0000xxx0`
/// verilator `V P=0000xxx0`
/// vita `V P=00000000`
#[test]
fn marks_repeated_select_write() {
    check(
        r#"module top;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t;
    t[0] = 1'b1; t[0] = 1'b0;
    f = t;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("f", "t")],
    );
}

/// The accounting: a double whole write counts once, the sibling stays live.
/// iverilog `V P=0000xxxx`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000000`
#[test]
fn marks_second_local_after_double_write() {
    check(
        r#"module top;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t, u;
    t = 4'd1; t = 4'd2;
    f = u;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("f", "u")],
    );
}

/// The return variable (a record with no name) read as an argument, in its own frame.
/// iverilog `V P=xxxxxxxx`
/// sv2v `V P=xxxxxxxx`
/// verilator `V P=xxxxxxxx`
/// vita `V P=00000000`
#[test]
fn marks_return_variable_in_an_argument() {
    check(
        r#"module top;
  function automatic logic [7:0] g(input logic [7:0] x); g = x; endfunction
  function automatic logic [7:0] f(input int a);
    f = g(f);
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("f", "f")],
    );
}

/// A caller's local as an argument: after a call, the caller's frame reads again.
/// iverilog `V P=xxxxxxxx`
/// sv2v `V P=xxxxxxxx`
/// verilator `V P=xxxxxxxx`
/// vita `V P=00000001`
#[test]
fn marks_caller_local_in_an_argument() {
    check(
        r#"module top;
  function automatic logic [3:0] g(input logic [3:0] t); logic [3:0] u; u = t; g = u; endfunction
  function automatic logic [7:0] f(input int a);
    logic [3:0] t, u;
    u = 4'd1;
    f = g(u) + g(t);
  endfunction
  localparam logic [7:0] P = f(1);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000001",
        &[("f", "t")],
    );
}

/// A frame: the callee has a live record, its formal `t` is the caller's seeded name: the callee's read looks in its own frame only.
/// iverilog `V P=00000010`
/// sv2v `V P=00000010`
/// verilator `V P=00000010`
/// vita `V P=00000010`
#[test]
fn unmarked_frame_isolated_live_callee() {
    check(
        r#"module top;
  function automatic int h(input int t); logic [3:0] u; return t; endfunction
  function automatic logic [7:0] f(input int a);
    logic [3:0] t;
    f = h(2);
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000010",
        &[],
    );
}

/// A dead frame: the callee's seeded `t` ended with its call, the caller's own `t` is a 2-state local.
/// iverilog `V P=00000001`
/// sv2v `V P=00000001`
/// verilator `V P=00000001`
/// vita `V P=00000001`
#[test]
fn unmarked_callee_record_gone() {
    check(
        r#"module top;
  function automatic logic [3:0] h(input int a); logic [3:0] t; h = 4'd1; endfunction
  function automatic logic [7:0] f(input int a);
    logic [3:0] u; int t;
    t = h(2);
    f = t;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000001",
        &[],
    );
}

/// A live sibling opens the gate: a read of the written record finds it cleared.
/// iverilog `V P=00000001`
/// sv2v `V P=00000001`
/// verilator `V P=00000001`
/// vita `V P=00000001`
#[test]
fn unmarked_written_beside_a_live_sibling() {
    check(
        r#"module top;
  function automatic logic [7:0] f(input int a);
    logic [3:0] t, u;
    t = 4'd1;
    f = t;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000001",
        &[],
    );
}

/// `[0:0]`: both bounds literals, the LSB 0: a select write clears.
/// iverilog `V P=00000001`
/// sv2v `V P=00000001`
/// verilator `V P=00000001`
/// vita `V P=00000001`
#[test]
fn unmarked_literal_single_bit() {
    check(
        r#"module top;
  function automatic logic [7:0] f(input int a);
    logic [0:0] t;
    t[0] = 1'b1;
    f = t;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000001",
        &[],
    );
}

/// `[7:0]` covered in a loop.
/// iverilog `V P=11111111`
/// sv2v `V P=11111111`
/// verilator `V P=11111111`
/// vita `V P=11111111`
#[test]
fn unmarked_literal_cover() {
    check(
        r#"module top;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    for (int i = 0; i < 8; i++) t[i] = 1'b1;
    f = t;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=11111111",
        &[],
    );
}

/// A name in a bound, a whole write: the write clears, whatever the numbering.
/// iverilog `V P=00001010`
/// sv2v `V P=00001010`
/// verilator `V P=00001010`
/// vita `V P=00001010`
#[test]
fn unmarked_named_bound_whole_write() {
    check(
        r#"module top;
  localparam L = 2;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t;
    t = 4'hA; f = t;
  endfunction
  if (1) begin : g
    localparam L = 0;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001010",
        &[],
    );
}

/// Over-mark: select-built, a name in the LSB, home `L = 0`: the select writes land in the local's own numbering, value right.
/// iverilog `V P=00001111`
/// sv2v `V P=00001111`
/// verilator `V P=00001111`
/// vita `V P=00001111`
#[test]
fn marks_over_named_lsb() {
    check(
        r#"module top;
  localparam L = 0;
  function automatic logic [7:0] f(input int a);
    logic [L+3:L] t;
    t[0] = 1'b1; t[1] = 1'b1; t[2] = 1'b1; t[3] = 1'b1; f = t;
  endfunction
  if (1) begin : g
    localparam L = 2;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// Over-mark: the same, a name in the MSB.
/// iverilog `V P=00001111`
/// sv2v `V P=00001111`
/// verilator `V P=00001111`
/// vita `V P=00001111`
#[test]
fn marks_over_named_width() {
    check(
        r#"module top;
  localparam W = 4;
  function automatic logic [7:0] f(input int a);
    logic [W-1:0] t;
    t[0 +: 4] = 4'hF; f = t;
  endfunction
  if (1) begin : g
    localparam W = 8;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// Non-claim, owner Q-RTNBODYSCOPE: a whole write at the call's width: `logic [W-1:0] t; t = 8'hA5;`, home `W = 8`, call `W = 4`: no seed is read in vita's run, and no never-assigned bit in IEEE's.
/// iverilog `V P=10100101`
/// sv2v `V P=10100101`
/// verilator `V P=10100101`
/// vita `V P=00000101`
#[test]
fn unmarked_nonclaim_whole_write_at_the_call_width() {
    check(
        r#"module top;
  localparam W = 8;
  function automatic logic [7:0] f(input int a);
    logic [W-1:0] t;
    t = 8'hA5; f = t;
  endfunction
  if (1) begin : g
    localparam W = 4;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00000101",
        &[],
    );
}

/// Non-claim, owner Q-RTNBODYSCOPE: the body's own read of `W` folds at the call (home 4, call 8): vita's word is fully written, so nothing fabricated is read.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=11111111`
#[test]
fn unmarked_nonclaim_body_read_at_the_call() {
    check(
        r#"module top;
  localparam W = 4;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    for (int i = 0; i < W; i++) t[i] = 1'b1;
    f = t;
  endfunction
  if (1) begin : g
    localparam W = 8;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=11111111",
        &[],
    );
}

/// Non-claim, owner Q-RTNBODYSCOPE: a type parameter's kind folds at the call: `bit` there, no seed.  iverilog cannot parse a `localparam type` here.
/// iverilog `refuses: sct_tp_gif.sv:8: syntax error`
/// sv2v `V P=0000xxxx`
/// verilator `V P=0000xxxx`
/// vita `V P=00000000`
#[test]
fn unmarked_nonclaim_type_parameter_kind() {
    check(
        r#"module top;
  parameter type T = logic [3:0];
  function automatic logic [7:0] f(input int a);
    T t;
    f = t;
  endfunction
  if (1) begin : g
    localparam type T = bit [3:0];
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00000000",
        &[],
    );
}

/// Non-claim, owner Q-PKGBLKLEAK: the flat env: the inner `logic [3:0] t` takes the outer's slot and width, and the outer's four bit writes clear the inner's record.
/// iverilog `V P=11111111`
/// sv2v `V P=11111111`
/// verilator `V P=11111111`
/// vita `V P=00001111`
#[test]
fn unmarked_nonclaim_flat_env() {
    check(
        r#"package q;
  function automatic logic [7:0] g(input int a);
    logic [7:0] t = 8'hff;
    begin logic [3:0] t; end
    t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1;
    return t;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::g(1);
  initial begin $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[],
    );
}

/// Non-claim, owner §2 "A formal shadowing a parameter is selected as the parameter", which this widens to a select of a LOCAL: the select arm (`const_fn.rs:1124`) reaches `const_select.rs:541`, whose `lookup_scoped` has no `envw` guard (the Ident arm has it, `const_fn.rs:942`), so `t[1:0]` reads the same-named constant's word `4'hA` and no env read happens: the set is true and empty.  This pin holds vita's wrong value `V P=00000010`, on PRE too; a fix moves output.  The renamed local (`u`, h52_local_ctl) is E3009 on PRE and here: no run, so it is no twin.
/// iverilog `V P=000000x1`
/// sv2v `V P=000000x1`
/// verilator `V P=000000x1`
/// vita `V P=00000010`
#[test]
fn unmarked_nonclaim_part_select_of_a_local_reads_the_same_named_parameter() {
    check(
        r#"module top;
localparam logic [3:0] t = 4'hA;
  function automatic logic [7:0] f(input int a);
logic [3:0] t; t[0]=1'b1; return t[1:0];
  endfunction
  localparam logic [7:0] P = f(1);

  initial begin $display("V P=%b", P); #1 $finish; end
endmodule
"#,
        0,
        "V P=00000010",
        &[],
    );
}

/// Non-claim, owner §2 "A formal shadowing a parameter is selected as the parameter", as `unmarked_nonclaim_part_select_of_a_local_reads_the_same_named_parameter`: `t[1]` of the local answers the constant's bit 1, so the comparison takes the arm no oracle takes.  This pin holds vita's wrong value `V P=00000001`, on PRE too.
/// iverilog `V P=00000010`
/// sv2v `V P=00000010`
/// verilator `V P=00000010`
/// vita `V P=00000001`
#[test]
fn unmarked_nonclaim_compare_of_a_local_select_takes_the_wrong_arm() {
    check(
        r#"module top;
localparam logic [3:0] t = 4'hA;
  function automatic logic [7:0] f(input int a);
logic [3:0] t; t[0]=1'b1; if (t[1] == 1'b1) return 8'd1; return 8'd2;
  endfunction
  localparam logic [7:0] P = f(1);

  initial begin $display("V P=%b", P); #1 $finish; end
endmodule
"#,
        0,
        "V P=00000001",
        &[],
    );
}

/// Non-claim, owner §2 "A formal shadowing a parameter is selected as the parameter", with no seed anywhere: the local is whole-written, then `t[1]` answers the constant's bit 1.  The set is empty and right to be; the wrong value is not a seed read.  This pin holds vita's wrong value `V P=00000001`, on PRE too.
/// iverilog `V P=00000000`
/// sv2v `V P=00000000`
/// verilator `V P=00000000`
/// vita `V P=00000001`
#[test]
fn unmarked_nonclaim_select_of_a_written_local_reads_the_same_named_parameter() {
    check(
        r#"module top;
localparam logic [3:0] t = 4'hA;
  function automatic logic [7:0] f(input int a);
logic [3:0] t; t = 4'h5; return t[1];
  endfunction
  localparam logic [7:0] P = f(1);

  initial begin $display("V P=%b", P); #1 $finish; end
endmodule
"#,
        0,
        "V P=00000001",
        &[],
    );
}

/// Non-claim, owner §2 "A block-local named like its function": a package routine's local named as its function and narrower than the return.  Z2 widens `envw` to the return's 8 and the seed skips a name the env holds, so the four bit writes clear the local's 4-bit record and the upper return bits were never recorded.  Invalid text (IEEE 1800-2017 §13.4.1, the implicit variable is in the same scope), package routines only: the module twin is E3009.  This pin holds vita's value `V P=00001111`, on PRE too.  `marks_narrow_local_renamed_from_its_function` is the twin.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=00001111`
/// verilator `refuses: h2_narrow_read.sv:3:22: Non-ANSI I/O declaration of signal conflicts with type declaration: 'f'`
/// vita `V P=00001111`
#[test]
fn unmarked_nonclaim_narrow_local_named_like_its_function() {
    check(
        r#"package p;

  function automatic logic [7:0] f(input int a);
logic [3:0] f; f[0]=1'b1; f[1]=1'b1; f[2]=1'b1; f[3]=1'b1; return f;
  endfunction
endpackage
module top;

  localparam logic [7:0] P = p::f(1);
  initial begin $display("V P=%b", P); #1 $finish; end
endmodule
"#,
        0,
        "V P=00001111",
        &[],
    );
}

/// h2_ren_narrow, the twin of `unmarked_nonclaim_narrow_local_named_like_its_function`: the local takes another name and the return variable's read at the fall-off marks.
/// iverilog `refuses: h2_ren_narrow.sv:9: error: Unable to evaluate parameter P value: p.f(32'sd1)`
/// sv2v `refuses: s2o.v:12: error: Unable to evaluate parameter P value: top.p_f(32'sd1)`
/// verilator `V P=xxxxxxxx`
/// vita `V P=00000000`
#[test]
fn marks_narrow_local_renamed_from_its_function() {
    check(
        r#"package p;

  function automatic logic [7:0] f(input int a);
logic [3:0] t; t[0]=1'b1; t[1]=1'b1; t[2]=1'b1; t[3]=1'b1;
  endfunction
endpackage
module top;

  localparam logic [7:0] P = p::f(1);
  initial begin $display("V P=%b", P); #1 $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("f", "f")],
    );
}

/// Non-claim, owner §2 "A block-local named like its function": a body-level 2-state local named as its package function takes the return variable's slot with no record, so the fall-off read is empty.  Invalid text (IEEE 1800-2017 §13.4.1), no oracle value.  This pin holds vita's value `V P=00000000`, on PRE too.  `marks_int_local_renamed_from_its_function` is the twin.
/// iverilog `refuses: b_int.sv:7: error: Unable to evaluate parameter P value: <select=q.g(32'sd1)[(0)+:8] expr_type=logic>`
/// sv2v `refuses: s2out.v:7: error: Unable to evaluate parameter P value: <select=top.q_g(32'sd1)[(0)+:8] expr_type=logic>`
/// verilator `refuses: b_int.sv:2:22: Non-ANSI I/O declaration of signal conflicts with type declaration: 'g'`
/// vita `V P=00000000`
#[test]
fn unmarked_nonclaim_int_local_named_like_its_function() {
    check(
        r#"package q;
  function automatic logic [3:0] g(input int a);
    int g;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::g(1);
  initial begin $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[],
    );
}

/// b_int_tw, the twin of `unmarked_nonclaim_int_local_named_like_its_function`: the local takes another name and the return variable's read at the fall-off marks.
/// iverilog `refuses: b_int_tw.sv:7: error: Unable to evaluate parameter P value: <select=q.g(32'sd1)[(0)+:8] expr_type=logic>`
/// sv2v `refuses: s2out.v:7: error: Unable to evaluate parameter P value: <select=top.q_g(32'sd1)[(0)+:8] expr_type=logic>`
/// verilator `V P=0000xxxx`
/// vita `V P=00000000`
#[test]
fn marks_int_local_renamed_from_its_function() {
    check(
        r#"package q;
  function automatic logic [3:0] g(input int a);
    int g_tw;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = q::g(1);
  initial begin $display("V P=%b", P); $finish; end
endmodule
"#,
        0,
        "V P=00000000",
        &[("g", "g")],
    );
}

/// Non-claim, owner Q-RTNBODYSCOPE: the body's own read of `W` folds at the call (home 4, call 8), so the `if` arm writes `t[4 +: 4]` in vita's run: vita's word is fully written, nothing fabricated is read.  bd_if_gif, the `[msb:lsb]` spelling, is E3009; this `+:` spelling gives a value.  This pin holds vita's wrong value `V P=10101111`, on PRE too.  `marks_indexed_write_arm_at_the_home_width` is the twin.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=10101111`
#[test]
fn unmarked_nonclaim_indexed_write_arm_read_at_the_call() {
    check(
        r#"module top;
  localparam W = 4;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    t[0 +: 4] = 4'hF;
    if (W == 8) t[4 +: 4] = 4'hA;
    f = t;
  endfunction
  if (1) begin : g
    localparam W = 8;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=10101111",
        &[],
    );
}

/// tw_bd_if2_gif, the twin of `unmarked_nonclaim_indexed_write_arm_read_at_the_call`: the generate block's constant takes another name, the call folds `W` as 4, the arm stays shut and `f = t` reads the unwritten bits.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_indexed_write_arm_at_the_home_width() {
    check(
        r#"module top;
  localparam W = 4;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    t[0 +: 4] = 4'hF;
    if (W == 8) t[4 +: 4] = 4'hA;
    f = t;
  endfunction
  if (1) begin : g
    localparam W_tw = 8;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}

/// Non-claim, owner Q-RTNBODYSCOPE: the body's own read of `W` folds at the call (home 4, call 8), so `t[0 +: W]` writes every bit in vita's run: nothing fabricated is read.  bd_sel_gif, the `[W-1:0]` spelling, is E3009; this `+:` spelling gives a value.  This pin holds vita's wrong value `V P=11111111`, on PRE too.  `marks_indexed_write_width_at_the_home_width` is the twin.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=11111111`
#[test]
fn unmarked_nonclaim_indexed_write_width_read_at_the_call() {
    check(
        r#"module top;
  localparam W = 4;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    t[0 +: W] = 8'hFF;
    f = t;
  endfunction
  if (1) begin : g
    localparam W = 8;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=11111111",
        &[],
    );
}

/// tw_bd_sel2_gif, the twin of `unmarked_nonclaim_indexed_write_width_read_at_the_call`: the generate block's constant takes another name, the call folds `W` as 4 and `f = t` reads the unwritten bits.
/// iverilog `V P=xxxx1111`
/// sv2v `V P=xxxx1111`
/// verilator `V P=xxxx1111`
/// vita `V P=00001111`
#[test]
fn marks_indexed_write_width_at_the_home_width() {
    check(
        r#"module top;
  localparam W = 4;
  function automatic logic [7:0] f(input int a);
    logic [7:0] t;
    t[0 +: W] = 8'hFF;
    f = t;
  endfunction
  if (1) begin : g
    localparam W_tw = 8;
    localparam logic [7:0] P = f(2);
    initial begin #1 $display("V P=%b", P); $finish; end
  end
endmodule
"#,
        0,
        "V P=00001111",
        &[("f", "t")],
    );
}
