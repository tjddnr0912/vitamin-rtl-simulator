//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the VAL-1 lane
//! cells that show a lane of the three merged arms (ROADMAP §5.2 VAL-1: `value/arms.rs`'s
//! `selfdet_truth_arm` in the Unlimited / Env / Ctx lanes, `unary_i64` under each core, and the
//! signing-over-size-cast body of both cast twins). Every expected text was measured on the
//! PRE binary (257c4990, `vita-post` md5 d5ac3b05 ; W/val1/build/gen/pre_out) with `t.sv` as
//! the argument. Not here: the w = 63 cells (k_sg_s63, k_sg_u63, k_sp_s63 ; a dev build
//! overflows there in PRE's text, PARSE-1 L14) -- the lane holds them.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_value_arms_{}_{n}", std::process::id()));
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

/// M-TRUTH Env lane: no wide fallback (a typed body with an unknown-shape target reaches eval_const_env ; mutant T1 kills here). Pair of c_lognot_x.
#[test]
fn e_lognot_x() {
    check(
        "e_lognot_x",
        r##"module t; function automatic int w1(input int k); w1 = k; endfunction
function automatic int f(input [w1(3):0] a); f = a; endfunction
localparam int A = f(!4'b100x); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:20: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f(…)` has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// M-TRUTH Ctx lane: the wide truth of `4'b101x` is 1, so `!` is 0 (pair of e_lognot_x).
#[test]
fn c_lognot_x() {
    check(
        "c_lognot_x",
        r##"module t; function automatic int f(input [3:0] a); f = a; endfunction
localparam int A = f(!4'b100x); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=0
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-TRUTH Env lane: a >64-bit condition declines (no wide fallback).
#[test]
fn e_tern_wide() {
    check(
        "e_tern_wide",
        r##"module t; function automatic int w1(input int k); w1 = k; endfunction
function automatic longint f(input [w1(3):0] a); f = a; endfunction
localparam longint A = f(128'h1_0000_0000_0000_0000_0000 ? 5 : 6); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:24: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f(…)` has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// M-TRUTH: `!4'b100x` at module scope is 0. Not T3's killer: both consumers take the Ctx lane
/// first (the typed localparam's width-aware `fold_assign` in params.rs `param_init_x`, the
/// range bound's `fold_self` in `range_bound_fold_ev`), and the binder chain ends in the wide
/// domain (`param_i64_at_declared`) ; m_override is T3's killer.
#[test]
fn m_lognot_x() {
    check(
        "m_lognot_x",
        r##"module t; localparam longint A = !4'b100x; logic [(!4'b100x) + 2:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
"##,
        r##"A a=0 bw=3
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-TRUTH Unlimited lane: an ambiguous condition (`4'b000x`) declines.
#[test]
fn m_tern_amb() {
    check(
        "m_tern_amb",
        r##"module t; localparam longint A = 4'b000x ? 5 : 6; logic [(4'b000x ? 5 : 6) + 2:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:34: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: 4'b000x has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// M-TRUTH Ctx lane: a condition naming a frame local that shadows a module constant declines (the local guard ; mutant T2 kills here).
#[test]
fn l_shadow2_tern() {
    check(
        "l_shadow2_tern",
        r##"module t; localparam logic [3:0] p = 4'b1111; function automatic int f(input int a); logic [3:0] p; p = 4'b0000; f = ({p, 4'b000x} ? 5 : 6) + a; endfunction
localparam longint A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:24: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f(…)` has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// M-TRUTH Unlimited lane: the same text at module scope takes the wide truth (no local guard).
#[test]
fn m_shadow2_tern() {
    check(
        "m_shadow2_tern",
        r##"module t; localparam logic [3:0] p = 4'b1111; localparam longint A = {p, 4'b000x} ? 5 : 6; logic [({p, 4'b000x} ? 5 : 6)+2:0] w; initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
"##,
        r##"A a=5 bw=8
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-TRUTH Unlimited lane with no later domain: an instance override folds in the parent
/// through `const_eval_in_scope` alone (instance.rs's override fold), and a `?:`-topped
/// override has no wide channel after it, so `Q`'s `4'b100x` condition is answered only by
/// the Unlimited lane's wide truth (mutant T3 kills here: W3056 + E3009, exit 1). `P`'s bare
/// `!` survives T3 through the override's self-determined wide channel.
#[test]
fn m_override() {
    check(
        "m_override",
        r##"module c #(parameter int P = 1, parameter int Q = 1) (); initial begin $display("A p=%0d q=%0d", P, Q); end endmodule
module t; c #(.P(!4'b100x), .Q(4'b100x ? -(4'sd3) : ~4'd0)) u(); initial #1 $finish; endmodule
"##,
        r##"A p=0 q=-3
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-TRUTH Env lane reached with a non-empty frame (a frame local in the operand).
#[test]
fn g_lognot_local() {
    check(
        "g_lognot_local",
        r##"module t; function automatic int w1(input int k); w1 = k; endfunction
function automatic longint f(input [w1(3):0] a); f = a; endfunction
function automatic longint g(input int z); logic [3:0] l4; l4 = 4'b1010; g = f(!l4) + z; endfunction
localparam longint A = g(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=0
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-UNARY: not CTX (no mask: -1) ; with an empty frame IN answers the same (premise PR15 / C4).
#[test]
fn e_bitnot() {
    check(
        "e_bitnot",
        r##"module t; function automatic int w1(input int k); w1 = k; endfunction
function automatic int f(input [w1(3):0] a); f = a; endfunction
localparam int A = f(~4'd0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=-1
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-UNARY: target-mask routing witness (fold_assign masks at the formal's 4 bits: 15 ; not the Val-arm mask, premise C4).
#[test]
fn c_bitnot() {
    check(
        "c_bitnot",
        r##"module t; function automatic int f(input [3:0] a); f = a; endfunction
localparam int A = f(~4'd0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=15
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-UNARY: `-` at the 64-bit minimum declines in CTX (checked_neg ; mutant U1 kills here).
#[test]
fn b_minus_min() {
    check(
        "b_minus_min",
        r##"module t; function automatic int f(input int a);  f = (-(64'sh8000_0000_0000_0000)) + a; endfunction
localparam longint A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:24: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f(…)` has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// M-UNARY: IN declines at the 64-bit minimum and the binder's wide lane answers.
#[test]
fn m_minus_min() {
    check(
        "m_minus_min",
        r##"module t; localparam longint A = -(64'sh8000_0000_0000_0000); logic [(-(64'sh8000_0000_0000_0000)) + 2:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
"##,
        r##"A a=-9223372036854775808 bw=1
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-CAST: `signed'(4'(15))` re-reads the 4 bits signed (-1).
#[test]
fn k_sg_s4() {
    check(
        "k_sg_s4",
        r##"module t; localparam longint A = signed'(4'(15)); logic [(signed'(4'(15))) + 2:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
"##,
        r##"A a=-1 bw=2
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-CAST: `unsigned'(4'(..))` keeps 15 (mutant C1 kills here).
#[test]
fn k_sg_u4() {
    check(
        "k_sg_u4",
        r##"module t; localparam longint A = unsigned'(4'(-1)); logic [(unsigned'(4'(-1))) + 2:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
"##,
        r##"A a=15 bw=18
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-CAST: the SigningParam twin (`T'(e)` over an overridable `parameter type`), the one body.
#[test]
fn k_sp_s4() {
    check(
        "k_sp_s4",
        r##"module c #(parameter type T = logic [3:0]) (); localparam longint A = T'(15); logic [(T'(15)) + 20:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
module t; c #(.T(logic signed [3:0])) u(); endmodule
"##,
        r##"A a=-1 bw=20
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// M-CAST: the SigningParam twin at w = 64 declines (mutant C2 kills here).
#[test]
fn k_sp_s64() {
    check(
        "k_sp_s64",
        r##"module c #(parameter type T = logic [3:0]) (); localparam longint A = T'(-1); logic [(T'(-1)) + 20:0] w;
initial begin $display("A a=%0d bw=%0d", A, $bits(w)); $finish; end endmodule
module t; c #(.T(logic signed [63:0])) u(); endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:71: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: the cast has no constant-fold arm [in t.u]
t.sv:1:86: error[VITA-E3009] E-ELAB-UNSUPPORTED: a cast that is not a constant here is not allowed in a constant range bound [in t.u]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// M-UNARY: CTX's Val-arm mask before a division (`(~a) / 4'd2` = 127 ; mutant U4 kills here, premise C4 / D4).
#[test]
fn u4_bitnot_div() {
    check(
        "u4_bitnot_div",
        r##"module t; function automatic int f(input int k); logic [3:0] a; bit [7:0] t; a = 4'd0; t = (~a) / 4'd2; f = t + k; endfunction
localparam int A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=127
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}
