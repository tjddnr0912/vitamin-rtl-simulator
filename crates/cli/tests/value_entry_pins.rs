//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the VAL-2 lane
//! cells that show a step of the declaration chain (ROADMAP §5.2 VAL-2: `value/chains.rs`'s
//! `param_decl_fold` in the four binders -- module body, package, generate scope, header after its
//! override channels) and the live-scope REG ask (`value/lanes.rs`'s `fold_self_wide`). Every
//! expected text was measured on the PRE binary (257c4990, `vita-post` md5 d5ac3b05 ;
//! W/val2/build/gen/pre_out) with `t.sv` as the argument.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_value_entry_{}_{n}", std::process::id()));
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

/// C1 module body: the five steps per binder (f1 s1-or-s3 control, f3 s3 at the declared width, n2 control, w2 s2 (s3 alone reads fe), p3 s3, r4 s4, s5 / c5 s2 controls, wd the wide bind after all five decline) ; mutant B1 (s2 after s3) kills on w2, B3 (s4 dropped) on r4.
#[test]
fn b_body() {
    check(
        "b_body",
        r##"module top;
  localparam int K = 15;
  localparam logic [127:0] A = 128'hF0;
  localparam F1 = '1;
  localparam logic [39:0] F3 = '1;
  localparam logic [3:0] N2 = K + 1;
  localparam logic [7:0] W2 = 8'hFF - (-1'sb1) + 65'd0;
  localparam int P3 = 3 * 4 + 1;
  localparam int R4 = 2.5 * 2;
  localparam logic [3:0] S5 = A[7:4];
  localparam bit C5 = (A == 128'hF0);
  localparam logic [127:0] WD = {64'h1, 64'h2};
  initial begin #1 $display("A f1=%0d bf1=%0d f3=%h n2=%0d w2=%h p3=%0d r4=%0d s5=%0d c5=%0d wd=%h", F1, $bits(F1), F3, N2, W2, P3, R4, S5, C5, WD); $finish; end
endmodule
"##,
        r##"A f1=1 bf1=1 f3=ffffffffff n2=0 w2=00 p3=13 r4=5 s5=15 c5=1 wd=00000000000000010000000000000002
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// C2 package: the same ten values.
#[test]
fn b_pkg() {
    check(
        "b_pkg",
        r##"package p;
  localparam int K = 15;
  localparam logic [127:0] A = 128'hF0;
  localparam F1 = '1;
  localparam logic [39:0] F3 = '1;
  localparam logic [3:0] N2 = K + 1;
  localparam logic [7:0] W2 = 8'hFF - (-1'sb1) + 65'd0;
  localparam int P3 = 3 * 4 + 1;
  localparam int R4 = 2.5 * 2;
  localparam logic [3:0] S5 = A[7:4];
  localparam bit C5 = (A == 128'hF0);
  localparam logic [127:0] WD = {64'h1, 64'h2};
endpackage
module top;
  initial begin #1 $display("A f1=%0d bf1=%0d f3=%h n2=%0d w2=%h p3=%0d r4=%0d s5=%0d c5=%0d wd=%h", p::F1, $bits(p::F1), p::F3, p::N2, p::W2, p::P3, p::R4, p::S5, p::C5, p::WD); $finish; end
endmodule
"##,
        r##"A f1=1 bf1=1 f3=ffffffffff n2=0 w2=00 p3=13 r4=5 s5=15 c5=1 wd=00000000000000010000000000000002
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// C3 generate scope: the same ten values.
#[test]
fn b_gen() {
    check(
        "b_gen",
        r##"module top;
  if (1) begin : g
  localparam int K = 15;
  localparam logic [127:0] A = 128'hF0;
  localparam F1 = '1;
  localparam logic [39:0] F3 = '1;
  localparam logic [3:0] N2 = K + 1;
  localparam logic [7:0] W2 = 8'hFF - (-1'sb1) + 65'd0;
  localparam int P3 = 3 * 4 + 1;
  localparam int R4 = 2.5 * 2;
  localparam logic [3:0] S5 = A[7:4];
  localparam bit C5 = (A == 128'hF0);
  localparam logic [127:0] WD = {64'h1, 64'h2};
  end
  initial begin #1 $display("A f1=%0d bf1=%0d f3=%h n2=%0d w2=%h p3=%0d r4=%0d s5=%0d c5=%0d wd=%h", g.F1, $bits(g.F1), g.F3, g.N2, g.W2, g.P3, g.R4, g.S5, g.C5, g.WD); $finish; end
endmodule
"##,
        r##"A f1=1 bf1=1 f3=ffffffffff n2=0 w2=00 p3=13 r4=5 s5=15 c5=1 wd=00000000000000010000000000000002
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// C4 header, default binds: the same ten values.
#[test]
fn b_hdr() {
    check(
        "b_hdr",
        r##"module m #(
  parameter int K = 15,
  parameter logic [127:0] A = 128'hF0,
  parameter F1 = '1,
  parameter logic [39:0] F3 = '1,
  parameter logic [3:0] N2 = K + 1,
  parameter logic [7:0] W2 = 8'hFF - (-1'sb1) + 65'd0,
  parameter int P3 = 3 * 4 + 1,
  parameter int R4 = 2.5 * 2,
  parameter logic [3:0] S5 = A[7:4],
  parameter bit C5 = (A == 128'hF0),
  parameter logic [127:0] WD = {64'h1, 64'h2}) ();
  initial begin #1 $display("A f1=%0d bf1=%0d f3=%h n2=%0d w2=%h p3=%0d r4=%0d s5=%0d c5=%0d wd=%h", F1, $bits(F1), F3, N2, W2, P3, R4, S5, C5, WD); $finish; end
endmodule
module top; m u(); endmodule
"##,
        r##"A f1=1 bf1=1 f3=ffffffffff n2=0 w2=00 p3=13 r4=5 s5=15 c5=1 wd=00000000000000010000000000000002
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// C4 header under overrides: each override channel answers before the chain (p3=7, s5=9, the wide pair), n2 / r4 from the chain ; mutant B9 (the chain before the channels) kills on p3.
#[test]
fn b_hdr_ovr() {
    check(
        "b_hdr_ovr",
        r##"module m #(
  parameter int K = 15, parameter logic [127:0] A = 128'hF0,
  parameter F1 = '1, parameter logic [39:0] F3 = '1, parameter logic [3:0] N2 = K + 1,
  parameter int P3 = 3 * 4 + 1, parameter int R4 = 2.5 * 2, parameter logic [3:0] S5 = A[7:4],
  parameter logic [127:0] WD = {64'h1, 64'h2}) ();
  initial begin #1 $display("A f1=%0d bf1=%0d f3=%h n2=%0d p3=%0d r4=%0d s5=%0d wd=%h", F1, $bits(F1), F3, N2, P3, R4, S5, WD); $finish; end
endmodule
module top; m #(.F1('1), .F3('1), .N2(4'd15 + 1), .P3(7), .S5(4'd9), .WD({64'h3, 64'h4})) u(); endmodule
"##,
        r##"A f1=1 bf1=1 f3=ffffffffff n2=0 p3=7 r4=5 s5=9 wd=00000000000000030000000000000004
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// an interface header + body parameter (bind_one_param = C4).
#[test]
fn b_iface() {
    check(
        "b_iface",
        r##"interface ifc #(parameter F1 = '1, parameter logic [3:0] N2 = 15 + 1, parameter int R4 = 2.5 * 2) ();
  localparam int K = 15; localparam logic [3:0] N2b = K + 1; localparam int R4b = 2.5 * 2;
  logic [N2b + R4b:0] w;
endinterface
module top; ifc i(); initial begin #1 $display("A bw=%0d f1=%0d r4=%0d", $bits(i.w), i.F1, i.R4); $finish; end endmodule
"##,
        r##"A bw=6 f1=1 r4=5
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// C4 fill gate, default binds: `parameter F1 = '1` binds 1 through step 1.
#[test]
fn c4_gate_dflt() {
    check(
        "c4_gate_dflt",
        r##"module m #(parameter F1 = '1) ();
  logic [F1*1100000:0] v;
endmodule
module top; m u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 1100001 exceeds the v1 cap (1048576) [in top.u]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// C4 fill gate, a non-constant override: step 1 is not asked (fill = default_binds = false), the recovery value reads 4294967296 ; mutant B5 (C4 passes true) kills here.
#[test]
fn c4_gate_ovr() {
    check(
        "c4_gate_ovr",
        r##"module m #(parameter F1 = '1) ();
  logic [F1*1100000:0] v;
endmodule
module top; logic n; m #(.F1(n)) u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:8: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: override of parameter `F1` is not a constant; default kept [in top]
t.sv:4:34: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `F1` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in top.u]
t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 4294967296 exceeds the v1 cap (1048576) [in top.u]
errors=2 warnings=2 notes=0
"##,
        1,
    );
}

/// the package site's own decline text (no cause, no span: PRE's, F2).
#[test]
fn d_pkg_div0() {
    check(
        "d_pkg_div0",
        r##"package p;
  localparam int D = 7 / 0;
endpackage
module top;
  initial begin #1 $display("A d=%0d", p::D); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
error[VITA-E3009] E-ELAB-UNSUPPORTED: package parameter `D` value is not a foldable constant [in p]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// the generate site's own decline text.
#[test]
fn d_gen_div0() {
    check(
        "d_gen_div0",
        r##"module top;
  if (1) begin : g
  localparam int D = 7 / 0;
  end
  initial begin #1 $display("A d=%0d", g.D); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: generate-scope parameter `D` value is not a constant: the `/` operation divides by zero, which is `x` and not a constant [in top.g]
t.sv:5:20: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared hierarchical name `g.D` (no such cross-instance net or parameter)
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// C1: steps 2 and 5 both ask the declared-width read ; one E3009 line.
#[test]
fn c1_two_declared() {
    check(
        "c1_two_declared",
        r##"module top;
  localparam int K = 15;
  localparam logic [3:0] D = K + 4'b1x00;
  initial begin #1 $display("A d=%b", D); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `D` value is not a constant: 4'b1x00 has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// step 3 isolated (premise C3): a constant-function call only IN folds.
#[test]
fn s3_call() {
    check(
        "s3_call",
        r##"package q; function automatic int h(input int a); return a; endfunction endpackage
module top;
  localparam int P = q::h(5);
  localparam P2 = q::h(5);
  initial begin #1 $display("A p=%0d p2=%0d", P, P2); $finish; end
endmodule
"##,
        r##"A p=5 p2=5
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 5 answers first in the module body (premise C1): mutant B4 (s5 dropped) kills here.
#[test]
fn s5f_body() {
    check(
        "s5f_body",
        r##"module top;
  localparam logic [31:0] S = $countones(4'b1100);
  localparam logic O = $onehot(4'b0100);
  localparam logic U = $isunknown(4'b1x00);
  localparam logic [31:0] A = $countones(4'b1100) + 1'b0;
  initial begin #1 $display("A s=%0d o=%0d u=%0d a=%0d bs=%0d", S, O, U, A, $bits(S)); $finish; end
endmodule
"##,
        r##"A s=2 o=1 u=1 a=2 bs=32
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 5 answers first in a package.
#[test]
fn s5f_pkg() {
    check(
        "s5f_pkg",
        r##"package p;
  localparam logic [31:0] S = $countones(4'b1100);
  localparam logic U = $isunknown(4'b1x00);
  localparam logic [31:0] A = $countones(4'b1100) + 1'b0;
endpackage
module top;
  initial begin #1 $display("A s=%0d u=%0d a=%0d", p::S, p::U, p::A); $finish; end
endmodule
"##,
        r##"A s=2 u=1 a=2
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 5 answers first in a generate scope.
#[test]
fn s5f_gen() {
    check(
        "s5f_gen",
        r##"module top;
  if (1) begin : g
    localparam logic [31:0] S = $countones(4'b1100);
    localparam logic U = $isunknown(4'b1x00);
    localparam logic [31:0] A = $countones(4'b1100) + 1'b0;
  end
  initial begin #1 $display("A s=%0d u=%0d a=%0d", g.S, g.U, g.A); $finish; end
endmodule
"##,
        r##"A s=2 u=1 a=2
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 5 answers first in a header (default binds).
#[test]
fn s5f_hdr() {
    check(
        "s5f_hdr",
        r##"module m #(parameter logic [31:0] S = $countones(4'b1100), parameter logic U = $isunknown(4'b1x00),
           parameter logic [31:0] A = $countones(4'b1100) + 1'b0) ();
  initial begin #1 $display("A s=%0d u=%0d a=%0d", S, U, A); $finish; end
endmodule
module top; m u(); endmodule
"##,
        r##"A s=2 u=1 a=2
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 5 answers first in an interface header / body.
#[test]
fn s5f_iface() {
    check(
        "s5f_iface",
        r##"interface ifc;
  localparam logic [31:0] S = $countones(4'b1100);
  localparam logic U = $isunknown(4'b1x00);
endinterface
module top;
  ifc i();
  initial begin #1 $display("A s=%0d u=%0d", i.S, i.U); $finish; end
endmodule
"##,
        r##"A s=2 u=1
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// control: the untyped twin of s5f_body (dm None: step 5 declines too).
#[test]
fn s5f_body_ut() {
    check(
        "s5f_body_ut",
        r##"module top;
  localparam S = $countones(4'b1100);
  initial begin #1 $display("A s=%0d", S); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:18: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `S` value is not a foldable constant expression [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// control: the untyped twin of s5f_pkg.
#[test]
fn s5f_pkg_ut() {
    check(
        "s5f_pkg_ut",
        r##"package p;
  localparam S = $countones(4'b1100);
endpackage
module top;
  initial begin #1 $display("A s=%0d", p::S); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
error[VITA-E3009] E-ELAB-UNSUPPORTED: package parameter `S` value is not a foldable constant [in p]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// control: the untyped twin of s5f_gen.
#[test]
fn s5f_gen_ut() {
    check(
        "s5f_gen_ut",
        r##"module top;
  if (1) begin : g
    localparam S = $countones(4'b1100);
  end
  initial begin #1 $display("A s=%0d", g.S); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:20: error[VITA-E3009] E-ELAB-UNSUPPORTED: generate-scope parameter `S` value is not a foldable constant expression [in top.g]
t.sv:5:20: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared hierarchical name `g.S` (no such cross-instance net or parameter)
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// control: the untyped twin of s5f_hdr.
#[test]
fn s5f_hdr_ut() {
    check(
        "s5f_hdr_ut",
        r##"module m #(parameter S = $countones(4'b1100)) ();
  initial begin #1 $display("A s=%0d", S); $finish; end
endmodule
module top; m u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `S` value is not a foldable constant expression [in top.u]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// step 1 isolated in the module body (premise C2): mutants B2 (s1 dropped) and B6a (C1 passes false) kill here (without s1, s3 reads 1).
#[test]
fn s1e_body() {
    check(
        "s1e_body",
        r##"package q; function automatic int h(input int a); return a; endfunction endpackage
module top;
  localparam U = ('1 >> 1) | (q::h(0) != 0) | (128'd0 == 128'd1);
  initial begin #1 $display("A u=%0d bu=%0d", U, $bits(U)); $finish; end
endmodule
"##,
        r##"A u=0 bu=1
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 1 isolated in a package: mutant B6b (C2 passes false).
#[test]
fn s1e_pkg() {
    check(
        "s1e_pkg",
        r##"package q; function automatic int h(input int a); return a; endfunction endpackage
package p;
  localparam U = ('1 >> 1) | (q::h(0) != 0) | (128'd0 == 128'd1);
endpackage
module top;
  initial begin #1 $display("A u=%0d", p::U); $finish; end
endmodule
"##,
        r##"A u=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 1 isolated in a generate scope: mutant B6c (C3 passes false).
#[test]
fn s1e_gen() {
    check(
        "s1e_gen",
        r##"package q; function automatic int h(input int a); return a; endfunction endpackage
module top;
  if (1) begin : g
    localparam U = ('1 >> 1) | (q::h(0) != 0) | (128'd0 == 128'd1);
  end
  initial begin #1 $display("A u=%0d", g.U); $finish; end
endmodule
"##,
        r##"A u=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 1 isolated in a header, default binds: mutant B5b (C4 passes false).
#[test]
fn s1e_hdr() {
    check(
        "s1e_hdr",
        r##"package q; function automatic int h(input int a); return a; endfunction endpackage
module m #(parameter U = ('1 >> 1) | (q::h(0) != 0) | (128'd0 == 128'd1)) ();
  initial begin #1 $display("A u=%0d bu=%0d", U, $bits(U)); $finish; end
endmodule
module top; m u(); endmodule
"##,
        r##"A u=0 bu=1
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// step 5 reads `dm`, not `meta` (premise C4): meta = (32, _), dm = None ; mutant B8 (s5 at meta) would answer 2.
#[test]
fn b8_body_ut() {
    check(
        "b8_body_ut",
        r##"module top;
  localparam A = $countones(4'b1100) + 1'b0;
  initial begin #1 $display("A a=%0d", A); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:18: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a foldable constant expression [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// the same in a header.
#[test]
fn b8_hdr_ut() {
    check(
        "b8_hdr_ut",
        r##"module m #(parameter A = $countones(4'b1100) + 1'b0) ();
  initial begin #1 $display("A a=%0d", A); $finish; end
endmodule
module top; m u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a foldable constant expression [in top.u]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// C4 override lane (D7): a typed default answers at step 5 under a non-constant override (S = 2 -> width 2200001).
#[test]
fn c4_ovr_s5() {
    check(
        "c4_ovr_s5",
        r##"module m #(parameter logic [31:0] S = $countones(4'b1100)) ();
  logic [S*1100000:0] v;
endmodule
module top; logic n; m #(.S(n)) u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:8: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: override of parameter `S` is not a constant; default kept [in top]
t.sv:4:33: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `S` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in top.u]
t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 2200001 exceeds the v1 cap (1048576) [in top.u]
errors=2 warnings=2 notes=0
"##,
        1,
    );
}

/// control: the untyped twin (step 5 declines: E3009 on S).
#[test]
fn c4_ovr_s5_ut() {
    check(
        "c4_ovr_s5_ut",
        r##"module m #(parameter S = $countones(4'b1100)) ();
  logic [S*1100000:0] v;
endmodule
module top; logic n; m #(.S(n)) u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:8: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: override of parameter `S` is not a constant; default kept [in top]
t.sv:4:33: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `S` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in top.u]
t.sv:1:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `S` value is not a foldable constant expression [in top.u]
errors=2 warnings=2 notes=0
"##,
        1,
    );
}

/// C4 override lane (D7): step 4 answers (R = 5 -> width 5500001).
#[test]
fn c4_ovr_s4() {
    check(
        "c4_ovr_s4",
        r##"module m #(parameter int R = 2.5 * 2) ();
  logic [R*1100000:0] v;
endmodule
module top; logic n; m #(.R(n)) u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:8: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: override of parameter `R` is not a constant; default kept [in top]
t.sv:4:33: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `R` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in top.u]
t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 5500001 exceeds the v1 cap (1048576) [in top.u]
errors=2 warnings=2 notes=0
"##,
        1,
    );
}

/// C4 override lane (D7): step 2 answers (W = 0 -> width 1100001 ; step 3 alone would read fe).
#[test]
fn c4_ovr_s2() {
    check(
        "c4_ovr_s2",
        r##"module m #(parameter logic [7:0] W = 8'hFF - (-1'sb1) + 65'd0) ();
  logic [W+1100000:0] v;
endmodule
module top; logic n; m #(.W(n)) u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:8: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: override of parameter `W` is not a constant; default kept [in top]
t.sv:4:33: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `W` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in top.u]
t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 1100001 exceeds the v1 cap (1048576) [in top.u]
errors=2 warnings=2 notes=0
"##,
        1,
    );
}

/// REG in the live scope (fold_self_wide): truth x2, reduction, compare, `$clog2` narrow and wide, `$bits`, the real twin, a range, a generate condition ; mutants L1 / L3 kill here.
#[test]
fn l_live() {
    check(
        "l_live",
        r##"module top;
  localparam logic [127:0] A = 128'hF0;
  localparam logic [199:0] B = 200'h1 << 190;
  localparam int T1 = !4'b10x1;                 // selfdet_truth (a known 1)
  localparam int T2 = 4'b10x1 ? 5 : 6;
  localparam int R1 = ^A;                       // selfdet_bits_i64 (IN reduction)
  localparam int R2 = (A[7:4] == 4'hF);
  localparam int C1 = $clog2(A);                // selfdet_bits_unsigned
  localparam int C2 = $clog2(B);                // selfdet_clog2_wide
  localparam int W1 = $bits(A + B);             // wide_selfdet_width
  localparam real Q1 = A;                       // const_selfdet_real
  logic [(^A) + 3:0] v;
  if (A[4]) begin : g localparam int L = 9; end
  initial begin #1 $display("A t1=%0d t2=%0d r1=%0d r2=%0d c1=%0d c2=%0d w1=%0d q1=%0f bv=%0d l=%0d", T1, T2, R1, R2, C1, C2, W1, Q1, $bits(v), g.L); $finish; end
endmodule
"##,
        r##"A t1=0 t2=5 r1=0 r2=1 c1=8 c2=190 w1=200 q1=240.000000 bv=4 l=9
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// the generate condition has no wide truth (PRE's E3010).
#[test]
fn l_live_gen() {
    check(
        "l_live_gen",
        r##"module top;
  localparam logic [127:0] A = 128'h3;
  if (4'b110x) begin : g localparam int L = 1; end else begin : g localparam int L = 2; end
  if (A) begin : h localparam int L = 3; end
  initial begin #1 $display("A hl=%0d", h.L); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:7: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: generate-if condition is not a constant: 4'b110x has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}
