//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the BIND-2 lane cells that
//! show what a constant-function local is (ROADMAP §5.2 BIND-2a: the frame `ConstFrame`, `bound/frame.rs`,
//! its writers `bind_shape` / `bind_value` / `unbind_value` / `seed_value`, and the presence lanes L2-L8 of
//! the BIND-2 plan: a name declared and unbound, bound and undeclared, width 0 = unknown, one slot per name,
//! the empty frame, the `[N:0]` numbering, the range folding outside the frame) and, BIND-2b, the read
//! lanes no other pin tells apart from a neighbour lane (k9 `is_local_w`, k10 `shape`'s width 0). Every
//! expected text was measured on a PRE binary with `t.sv` as the argument: BIND-1b's POST (7672b198,
//! `vita` md5 ddcbd1df) for the BIND-2a pins, BIND-2a's POST (c5c131a0, `vita` md5 74dab3ae) for all 28 ;
//! the same text on 6bcd57fa (H/pre/bind2_lane). The oracle text in each doc is iverilog 13.0 and
//! verilator 5.054 verbatim (S/build_a/lane/ORACLE.txt, S/build_b/lane/oracle) ; where vita's value differs from an oracle's, the
//! pin holds vita's value as PRE prints it and names the §2 filing (structure only: no fix here). A named
//! mutant killer here is a claim until its mutant runs.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_frame_pins_{}_{n}", std::process::id()));
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

/// L2 declared, unbound (lane cell `c29_init_reads_self_redecl`) -- a block declaration's initializer reads its own name before it is bound: `envw` only, the read is loud.
/// Owner line: the plan's L2 (the leaf's declared-unbound arm) ; PRE E3009 rc 1
/// PRE E3009 rc 1 where iverilog vvp prints `A P=15` (soundness lens N-S4b).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=15` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn c29() {
    check(
        "c29",
        r##"module top;
  function automatic int f(input int k);
    int acc; acc = 0;
    for (int i = 0; i < k; i++) begin int t = t + 5; acc = acc + t; end
    return acc;
  endfunction
  localparam int P = f(3);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L2 declared, unbound (lane cell `c33_unknown_width_init`) -- an initializer of a local whose width did not fold (`w(4)-1`): the local stays unbound (W5, `unbind_value`), the read is loud.
/// Owner line: the plan's L2 ; "the loud must not be conditional on the width being known" (bind_const_decl) ; PRE E3009 rc 1
/// IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14647-14648): "Constant functions shall not themselves use constant functions in any context requiring a constant expression" (soundness lens N-S4c).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=5` / `t.sv:5: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:3:66: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 4 bits.`.
#[test]
fn c33() {
    check(
        "c33",
        r##"module top;
  function automatic int w(input int a); return a; endfunction
  function automatic int f(input int k); logic [w(4)-1:0] t = 5; return t; endfunction
  localparam int P = f(0);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L3 bound, undeclared (lane cell `c18_pkg_assign_const`) -- a package function assigns a package constant: the seeded name (W8, `seed_value`) has no `envw` entry, the blocking write folds unlimited.
/// Owner line: BIND-2 filing F1, JOINS §2 pkg-const-whole-write (pre-existing, both oracles refuse, PRE folds 13 silently) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:3: error: Could not find variable ``C'' in ``p.g''` / `1 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:3:42: Storing to parameter variable 'C' in a context that is determined only at runtime` / `%Error: Exiting due to 1 error(s)`.
#[test]
fn c18() {
    check(
        "c18",
        r##"package p;
  localparam logic [3:0] C = 3;
  function automatic int g(input int a); C = C + a; return C; endfunction
endpackage
module top;
  localparam int P = p::g(10);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=13
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L3 bound, undeclared (lane cell `g3a_formal_pkgconst`) -- a formal and a package constant of one name: the formal is bound first and the package seed keeps it (`seed_value` = `entry().or_insert`).
/// Owner line: the plan's L3 ; the mutant M5 (W8 `seed_value` -> `bind_value`) killer claim ; PRE `A P=11`
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=11` / `t.sv:7: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:3:59: Operator ADD expects 32 bits on the LHS, but LHS's VARREF 'C' generates 4 bits.` / `%Warning-WIDTHTRUNC: t.sv:6:25: Operator FUNCREF 'g' expects 4 bits on the Function Argument, but Function Argument's CONST '?32?sh1a' generates 32 or 5 bits.`.
#[test]
fn g3a() {
    check(
        "g3a",
        r##"package p;
  localparam int C = 3;
  function automatic int g(input logic [3:0] C); return C + 1; endfunction
endpackage
module top;
  localparam int P = p::g(26);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=11
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L3 bound, undeclared (lane cell `g3b_pkgconst_concat`) -- a package constant as a concat operand: an env-only name has no `envw` entry, so the placement falls to `lookup_scoped` and reads the module's `C`.
/// Owner line: BIND-2 filing F3, JOINS §2 pkg-body-const-unhooked (candidate ; pre-existing, PRE 241 vs iverilog 49) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=49` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:3:59: Operator ADD expects 32 bits on the LHS, but LHS's REPLICATE generates 8 bits.`.
#[test]
fn g3b() {
    check(
        "g3b",
        r##"package p;
  localparam logic [3:0] C = 4'd3;
  function automatic int g(input int a); return {C, 4'h1} + a; endfunction
endpackage
module top;
  localparam logic [7:0] C = 8'hFF;
  localparam int P = p::g(0);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=241
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L4 width 0 = unknown (lane cell `c01_j4r`) -- a local whose range names a body localparam that is not bound: `bind_shape(.., None)` records width 0 and `x = '1` folds at 1 bit.
/// Owner line: the plan's L4 (`bind_shape`'s None arm, the one width-0 reading) ; the mutant M1 (None -> (32, false)) killer claim (R13) ; PRE `A P=1`
/// PRE `A P=1` where iverilog vvp prints `A P=511` ; gap entry: §2 const-fn-local-unbound-width (docs/roadmap/2/const-fn-local-unbound-width.md, witness j4r ; soundness lens N-S4a).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=511` / `t.sv:6: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:3:64: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 'x' generates 9 bits.`.
#[test]
fn c01_j4r() {
    check(
        "c01_j4r",
        r##"module m #(parameter P = f()) ();
  localparam W = 9;
  function automatic integer f(); logic [W - 1 : 0] x; x = '1; return x; endfunction
  initial #1 $display("A P=%0d", P);
endmodule
module top; m u(); initial #2 $finish; endmodule
"##,
        r##"A P=1
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L4 width 0 = unknown (lane cell `c02_w0_local`) -- a local whose range holds a call: width 0, no mask on the assignment.
/// Owner line: the plan's L4 ; VAL-2 F1 (const-fn-local-unbound-width) ; PRE `A p=300`
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A p=12` / `t.sv:9: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:5:7: Operator ASSIGN expects 4 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.` / `%Warning-WIDTHEXPAND: t.sv:6:7: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 4 bits.`.
#[test]
fn c02() {
    check(
        "c02",
        r##"module top;
  function automatic int w(input int k); w = k; endfunction
  function automatic int f(input int a);
    logic [w(4)-1:0] t;
    t = a;
    f = t;
  endfunction
  localparam int P = f(300);
  initial begin #1 $display("A p=%0d", P); $finish; end
endmodule
"##,
        r##"A p=300
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L4 width 0 = unknown (lane cell `c03_w0_formal`) -- a formal whose range holds a call: W6 `bind_shape(.., None)`, width 0.
/// Owner line: the plan's L4 ; PRE `A p=300`
/// PRE `A p=300` where iverilog vvp prints `A p=12` ; gap entry: §2 const-fn-local-unbound-width (docs/roadmap/2/const-fn-local-unbound-width.md, "the formal twin w0_formal the same" ; soundness lens N-S4a). IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14647-14648): "Constant functions shall not themselves use constant functions in any context requiring a constant expression" (soundness lens N-S4c).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A p=12` / `t.sv:7: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:4:7: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 4 bits.` / `%Warning-WIDTHTRUNC: t.sv:6:22: Operator FUNCREF 'f' expects 4 bits on the Function Argument, but Function Argument's CONST '?32?sh12c' generates 32 or 9 bits.`.
#[test]
fn c03() {
    check(
        "c03",
        r##"module top;
  function automatic int w(input int k); w = k; endfunction
  function automatic int f(input logic [w(4)-1:0] a);
    f = a;
  endfunction
  localparam int P = f(300);
  initial begin #1 $display("A p=%0d", P); $finish; end
endmodule
"##,
        r##"A p=300
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L4 width 0 = unknown (lane cell `g1b_w5_init_then_assign`) -- a width-unknown local with an initializer, then assigned: unbound by W5, then bound unmasked by W12.
/// Owner line: BIND-2 filing F6, JOINS §2 const-fn-local-unbound-width (pre-existing, PRE 300 vs iverilog 12) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=12` / `t.sv:5: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:3:68: Operator ASSIGN expects 4 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.` / `%Warning-WIDTHEXPAND: t.sv:3:75: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 4 bits.`.
#[test]
fn g1b() {
    check(
        "g1b",
        r##"module top;
  function automatic int w(input int k); return k; endfunction
  function automatic int f(input int a); logic [w(4)-1:0] t = 5; t = a; f = t; endfunction
  localparam int P = f(300);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=300
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L4 width 0 = unknown (lane cell `c23_w0_select_write`) -- a select write on a width-0 local declines (`vw == 0`).
/// Owner line: the plan's L4 ; PRE E3009 rc 1, iverilog `A P=2`: a loud, not a wrong value
/// IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14647-14648): "Constant functions shall not themselves use constant functions in any context requiring a constant expression" (soundness lens N-S4c).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=2` / `t.sv:5: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:3:82: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 4 bits.`.
#[test]
fn c23() {
    check(
        "c23",
        r##"module top;
  function automatic int w(input int a); return a; endfunction
  function automatic int f(input int k); logic [w(4)-1:0] t; t = 0; t[1] = 1'b1; return t; endfunction
  localparam int P = f(0);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L5 flat (lane cell `g2b_block_local_named_fn`) -- a block local named like its function: one slot per name, the block declaration rewrites the return variable's shape.
/// Owner line: BIND-2 filing F5, JOINS §2 block-local-named-function (pre-existing, PRE 12, iverilog refuses) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:3: error: Unable to evaluate parameter P value: top.f(32'sd300)` / `1 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:2:65: Operator ASSIGN expects 4 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.`.
#[test]
fn g2b() {
    check(
        "g2b",
        r##"module top;
  function automatic int f(input int a); begin logic [3:0] f; f = a; end endfunction
  localparam int P = f(300);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=12
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L5 flat + L9 (lane cell `g5_init_reads_old`) -- a block declaration's initializer naming itself reads the last iteration's value (the shape is recorded before the initializer, the old value stays visible).
/// Owner line: BIND-2 filing F4, NEW §2 const-fn-redecl-init-reads-prior (pre-existing, PRE 33 vs iverilog 12) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=12` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn g5() {
    check(
        "g5",
        r##"module top;
  function automatic int f(input int n);
    int s; s = 0;
    for (int i = 0; i < n; i++) begin int t = t + 1; if (i == 0) t = 10; s = s + t; end
    return s;
  endfunction
  localparam int P = f(3);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=33
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L6 empty frame (lane cell `g6a_default_first_formal`) -- a FIRST formal's default runs on the empty frame inside a call and folds.
/// Owner line: the plan's L6 ; PRE `A P=3`
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=3` / `t.sv:4: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-REALCVT: t.sv:2:53: Implicit conversion of real to integer; expected integral input to CLOG2`.
#[test]
fn g6a() {
    check(
        "g6a",
        r##"module top;
  function automatic int f(input int a = $clog2(2.0 * 4)); return a; endfunction
  localparam int P = f();
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=3
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L6 empty frame (lane cell `g6b_default_second_formal`) -- a SECOND formal's default runs on a half-built frame: the `$clog2` of a real argument declines.
/// Owner line: BIND-2 filing F8, a note under L6 (a loud is not filed as wrong ; iverilog `A P=3`) ; PRE E3009 rc 1
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=3` / `t.sv:4: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-REALCVT: t.sv:2:70: Implicit conversion of real to integer; expected integral input to CLOG2`.
#[test]
fn g6b() {
    check(
        "g6b",
        r##"module top;
  function automatic int f(input int z = 0, input int a = $clog2(2.0 * 4)); return a + z; endfunction
  localparam int P = f();
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L7 numbering (lane cell `c12_offset_select`) -- a select write on a `[5:2]` local: the bit index is the declared one (Q-CONSTSELLSB's k_off_b shape).
/// Owner line: the plan's L7 ; Q-CONSTSELLSB (pre-existing: PRE 00001100, iverilog 00000011) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=00000011` / `t.sv:4: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:2:90: Operator ASSIGN expects 8 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 4 bits.`.
#[test]
fn c12() {
    check(
        "c12",
        r##"module top;
  function automatic logic [7:0] f(input int k); bit [5:2] t; t = 0; t[2] = 1; t[3] = 1; return t; endfunction
  localparam logic [7:0] P = f(0);
  initial begin #1 $display("A P=%b", P); $finish; end
endmodule
"##,
        r##"A P=00001100
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L7 numbering (lane cell `c13_md_local`) -- a select write on a packed 2-D local (Q-CONSTAGG's k_md_b shape).
/// Owner line: the plan's L7 ; Q-CONSTAGG (pre-existing: PRE 00000001) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=01100101` / `t.sv:4: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn c13() {
    check(
        "c13",
        r##"module top;
  function automatic logic [7:0] f(input int k); bit [1:0][3:0] t; t[0] = 4'd5; t[1] = 4'd6; return t; endfunction
  localparam logic [7:0] P = f(0);
  initial begin #1 $display("A P=%b", P); $finish; end
endmodule
"##,
        r##"A P=00000001
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L8 range outside the frame (lane cell `c20b_range_formal_shadow`) -- a local's range naming a formal that shadows a module localparam folds at module scope and reads the module's `n`.
/// Owner line: BIND-2 filing F2, NEW §2 const-fn-range-formal-shadow (pre-existing, both oracles refuse, PRE folds 511 silently) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=2, `t.sv:3: error: A reference to a net or variable (`n') is not allowed in a constant expression.` / `t.sv:3: error: Dimensions must be constant.` / `t.sv:3       : This MSB expression violates the rule: (n)-('sd1)` / `2 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:3:49: Expecting expression to be constant, but variable isn't const: 'n'` / `%Error: t.sv:3:50: left side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1)` / `%Warning-WIDTHEXPAND: t.sv:3:67: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 2 bits.` / `%Error: Exiting due to 2 error(s)`.
#[test]
fn c20b() {
    check(
        "c20b",
        r##"module top;
  localparam int n = 9;
  function automatic int f(input int n); logic [n-1:0] t; t = '1; return t; endfunction
  localparam int P = f(4);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=511
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// 76 lane (lane cell `c14_unwritten_ret`) -- a never-written 4-state return variable: W9 shape, W10 `seed_value(0)`, the value reads 0.
/// Owner line: row 76 (an x plane on a local's value ; PRE 0000, the 76 page: verilator xxxx) ; structure only, no fix
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:3: error: Unable to evaluate parameter P value: top.f(32'sd0)` / `1 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn c14() {
    check(
        "c14",
        r##"module top;
  function automatic logic [3:0] f(input int k); endfunction
  localparam logic [3:0] P = f(0);
  initial begin #1 $display("A P=%b", P); $finish; end
endmodule
"##,
        r##"A P=0000
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L2 declared, unbound (lane cell `k1_shape_before_init`) -- a block local whose initializer reads its own name beside a module localparam of that name: W1 `bind_shape` runs BEFORE the initializer, so the read finds the declared, unbound local and is loud (PRE E3009).
/// Owner line: R13, the named killer of M2 (W1 moved after the initializer: the read falls to the module `t` and folds `A P=8`) ; a loud where iverilog prints a value: pre-existing, structure only
/// PRE E3009 rc 1 where iverilog vvp prints `A P=1` (soundness lens N-S4b).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=1` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn k1() {
    check(
        "k1",
        r##"module top;
  localparam int t = 7;
  function automatic int f();
    int t = t + 1;
    return t;
  endfunction
  localparam int P = f();
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L2 declared, unbound (lane cell `k2_redecl_init_unfolds`) -- a loop-body declaration re-run per iteration whose initializer stops folding on the 2nd pass (`10 / (1 - 1)`): W4 `unbind_value` drops the 1st pass value, the read is loud.
/// Owner line: R13, the named killer of M3 (W4 dropped: the stale 10 is read, `A P=20`) ; pre-existing loud, structure only
/// PRE E3009 rc 1 where iverilog vvp prints `A P=10` (soundness lens N-S4b).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=10` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn k2() {
    check(
        "k2",
        r##"module top;
  function automatic int f(input int n);
    int s; s = 0;
    for (int i = 0; i < n; i++) begin int t = 10 / (1 - i); s = s + t; end
    return s;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L2 declared, unbound (lane cell `k3_unbound_local_beside_param`) -- a local initialised `4'bx` (does not fold) beside a module localparam of its name: the leaf finds the declared, unbound local and does not fall to the parameter.
/// Owner line: R13 (premise judge C3), the named killer of M9 (the leaf's declared-unbound arm dropped: the module `t` = 5 is read, `A P=5`) ; pre-existing loud, structure only
/// PRE E3009 rc 1 where iverilog vvp prints `A P=0` (soundness lens N-S4b).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=0` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:5:14: Operator ADD expects 32 bits on the LHS, but LHS's VARREF 't' generates 4 bits.`.
#[test]
fn k3() {
    check(
        "k3",
        r##"module top;
  localparam logic [3:0] t = 4'd5;
  function automatic int f(input int k);
    logic [3:0] t = 4'bx;
    return t + k;
  endfunction
  localparam int P = f(0);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L4 width 0 = unknown (lane cell `k4_redecl_width_unknown`) -- a loop-body declaration of a width-unknown local (`[w(4)-1:0]`) with an initializer: W5 `unbind_value` leaves it unbound every pass, the read is loud.
/// Owner line: R13, the named killer of M4 (W5 arm emptied: the last pass value 9 is read, `A P=9`) ; pre-existing loud, structure only
/// IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14647-14648): "Constant functions shall not themselves use constant functions in any context requiring a constant expression" (soundness lens N-S4c).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=5` / `t.sv:9: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:5:77: Operator ASSIGN expects 32 bits on the Assign RHS, but Assign RHS's VARREF 't' generates 4 bits.`.
#[test]
fn k4() {
    check(
        "k4",
        r##"module top;
  function automatic int w(input int k); return k; endfunction
  function automatic int f(input int n);
    int s; s = 0;
    for (int i = 0; i < n; i++) begin logic [w(4)-1:0] t = 5; if (i == 1) s = t; t = 9; end
    return s;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:8:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L3 bound, undeclared (lane cell `k5_formal_named_fn`) -- a formal named like its function: W7 binds the formal 1, W10 `seed_value(0)` for the return variable keeps it, the body is empty and the return reads 1.
/// Owner line: R13, the named killer of M6 (W10 as `bind_value`: the return variable overwrites the formal, `A P=00000000`)
/// PRE `A P=00000001` where both oracles refuse the text (rc 1) ; gap entry: §2 block-local-named-function (docs/roadmap/2/block-local-named-function.md, witness h2_formal ; soundness lens N-S4a).
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:2: error: Function p.f port f is not an input port.` / `t.sv:2:      : Function arguments must be input ports.` / `1 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:2:46: Duplicate declaration of signal: 'f'` / `%Error: Exiting due to 1 error(s)`.
#[test]
fn k5() {
    check(
        "k5",
        r##"package p;
  function automatic logic [7:0] f(input int f);
  endfunction
endpackage
module top;
  localparam logic [7:0] P = p::f(1);
  initial begin #1 $display("A P=%b", P); $finish; end
endmodule
"##,
        r##"A P=00000001
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L4 width 0 = unknown (lane cell `k6_assign_ctx_target`) -- a blocking assignment `t = (4'd15 + 4'd1) >> 1` to a 4-bit local: the target's declared width is the context (the sum wraps at 4 bits), `A P=0`.
/// Owner line: R13 (premise judge C3), the named killer of M10 (the target width dropped: the sum folds wide, `A P=8`)
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=0` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:5:14: Operator ADD expects 32 bits on the LHS, but LHS's VARREF 't' generates 4 bits.`.
#[test]
fn k6() {
    check(
        "k6",
        r##"module top;
  function automatic int f(input int k);
    bit [3:0] t;
    t = (4'd15 + 4'd1) >> 1;
    return t + k;
  endfunction
  localparam int P = f(0);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##"A P=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// L6 empty frame (lane cell `k7_first_init_empty_env`) -- a zero-formal function whose first local's initializer is `$bits(Q)`: it runs on a frame with no width yet, the default arm declines and the call is loud.
/// Owner line: R13 (premise judge C3), the named killer of M7 (the `&& fr.widths().is_empty()` guard dropped: `A P=32`) ; M7 is NOT equivalent ; pre-existing loud, structure only
/// PRE E3009 rc 1 where iverilog vvp prints `A P=32` (soundness lens N-S4b). IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14635-14636): "It shall be legal to call any system function that is allowed in a constant_expression (see 11.2.1). This includes $bits and the array query functions."
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=32` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn k7() {
    check(
        "k7",
        r##"module top;
  localparam int Q = 5;
  function automatic int f();
    int t = $bits(Q);
    return t;
  endfunction
  localparam int P = f();
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L2 declared, unbound (lane cell `k8_reduce_unbound_local`) -- a reduction `|t` of a declared, unbound local (initialised `4'bx`) beside a module localparam of its name: the local closure in fold.rs sees the local's shape and declines.
/// Owner line: R13, the named killer of M8 (the closure drops `envw`: the module `t` is reduced, `A P=1`) ; pre-existing loud, structure only
/// PRE E3009 rc 1 where iverilog vvp prints `A P=0` (soundness lens N-S4b).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=0` / `t.sv:8: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn k8() {
    check(
        "k8",
        r##"module top;
  localparam logic [3:0] t = 4'b0101;
  function automatic int f(input int k);
    logic [3:0] t = 4'bx;
    return (|t) + k;
  endfunction
  localparam int P = f(0);
  initial begin #1 $display("A P=%0d", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L2 declared, unbound (lane cell `k9_concat_sel_unbound_local`) -- a select of a declared, unbound local (`logic [g(8)-1:0] A = 5;`, width did not fold: W5) as a concat operand beside a module localparam `A` with a non-zero lsb: `const_placement_wide`'s select-root test `is_local_w` (envw first) sees the local and declines, so the module parameter is not read.
/// Owner line: R13, the named killer of BIND-2b's N3 (`is_local_w` without its envw half: the mutant reads the module `A`, `A P=c1`) ; PRE E3009 rc 1 (loud where iverilog folds 01: JOINS §2 const-fn-local-unbound-width, structure only)
/// IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14647-14648): "Constant functions shall not themselves use constant functions in any context requiring a constant expression" (soundness lens N-S4c).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=01` / `t.sv:9: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn k9() {
    check(
        "k9",
        r##"module top;
  localparam logic [11:4] A = 8'h3C;
  function automatic int g(input int n); g = n; endfunction
  function automatic logic [7:0] f(input int a);
    logic [g(8)-1:0] A = 5;
    f = {A[7:4], 4'h1};
  endfunction
  localparam logic [7:0] P = f(0);
  initial begin #1 $display("A P=%h", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:8:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// L4 width 0 = unknown (lane cell `k10_concat_w0_local_beside_param`) -- a local of unknown width (`logic [g(4)-1:0] t;`, bound by `t = a`) as a concat operand beside a module localparam `t`: `const_placement_wide` reads its `shape` (0, false) and declines instead of falling to the module's `t`.
/// Owner line: R13, the named killer of BIND-2b's N7 (`shape` reading width 0 as undeclared: the mutant reads the module `t`, `A P=51`) ; PRE E3009 rc 1 (loud where iverilog folds 21: JOINS §2 const-fn-local-unbound-width, structure only)
/// IEEE 1800-2017 §13.4.3 (ieee1800-2017.txt:14647-14648): "Constant functions shall not themselves use constant functions in any context requiring a constant expression" (soundness lens N-S4c).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A P=21` / `t.sv:10: $finish called at 1 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:6:7: Operator ASSIGN expects 4 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.`.
#[test]
fn k10() {
    check(
        "k10",
        r##"module top;
  localparam logic [3:0] t = 4'h5;
  function automatic int g(input int n); g = n; endfunction
  function automatic logic [7:0] f(input int a);
    logic [g(4)-1:0] t;
    t = a;
    f = {t, 4'h1};
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("A P=%h", P); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:9:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}
