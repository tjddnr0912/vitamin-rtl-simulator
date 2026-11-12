//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the VAL-1 lane
//! cells that show a lane of the three merged arms (ROADMAP §5.2 VAL-1: `value/arms.rs`'s
//! `selfdet_truth_arm` in the Unlimited / Env / Ctx lanes, `unary_i64` under each core, and the
//! signing-over-size-cast body of both cast twins). Every expected text was measured on the
//! PRE binary (257c4990, `vita-post` md5 d5ac3b05 ; W/val1/build/gen/pre_out) with `t.sv` as
//! the argument. Not here: the w = 63 cells (k_sg_s63, k_sg_u63, k_sp_s63 ; a dev build
//! overflows there in PRE's text, PARSE-1 L14) -- the lane holds them.
//! The batch 3 boundary's pins follow `u4_bitnot_div`: one input per surviving battery mutant
//! (W/batch3/battery: a01 a02 a08 a09 a11 a12) and the five lane cells that were the only
//! killer of theirs (a10 a16 a27 k06 k07), each measured on POST (30adf073, md5 66617835) and
//! PRE (md5 d5ac3b05), which print the same, and on a dev build of 30adf073 (the test profile).
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
/// PRE's value, which the oracles contradict: iverilog 13 + vvp `A a=15`, verilator 5.052
/// `A a=15`. Owner: the unmasked value of a call in an interpreter formal is §5.2 Q-RANGEMASK
/// (`const_decl_wsign`), docs/roadmap/2/new-AC.md ; that fix turns this pin red on purpose.
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
/// The pinned `bw=1` is PRE's, which the oracles contradict: iverilog 13 + vvp
/// `A a=-9223372036854775808 bw=-1` (+ `warning: Vector size is greater than 1073741824.`),
/// verilator 5.052 `%Error: t.sv:1:100: Value too wide for 32-bits expected in this context
/// 64'h8000000000000002`. Owner: the bound range rule for a bound past `integer`,
/// docs/roadmap/5.2/Q-PARAM2STATE.md:35 (Q-VALRULES (b) R6.x) ; that fix turns this pin red.
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

/// Battery a01 (the Ctx guard reads only `env`): a frame local whose initializer does not fold
/// (`logic [3:0] p = 4'b00x0`) is bound in `envw` and not in `env` (const_fn.rs `bind_const_decl`), so
/// the guard still sees it and the function declines; a guard over `env` alone misses it, the wide
/// truth reads the module `p` (`4'b1111`) and folds 5.
/// Loud: iverilog 13 -g2012 + vvp `A a=0`, sv2v `A a=x`, verilator 5.054 --binary `A a=6`.
#[test]
fn l_guard_envw_only() {
    check(
        "l_guard_envw_only",
        r##"module t; localparam logic [3:0] p = 4'b1111; function automatic int f(input int a); logic [3:0] p = 4'b00x0; f = ({p, 4'b000x} ? 5 : 6) + a; endfunction
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

/// Battery a02 (the Ctx guard reads only `envw`): a package function's body sees its package's
/// constants through `env` alone (const_fn.rs seeds `pkg_consts` into `env`, not `envw`), so the guard
/// declines `{p, 4'b000x}` in `k::f`; a guard over `envw` alone misses `p` and the wide truth folds 5.
/// Loud where the oracles fold: iverilog 13 -g2012 + vvp `A a=5`, sv2v `A a=5`, verilator 5.054 --binary `A a=5` -- the Ctx local-name
/// guard class (batch 3 differential lens G13, filed by the batch 3 landing docs).
#[test]
fn l_guard_pkg_const() {
    check(
        "l_guard_pkg_const",
        r##"package k; localparam logic [3:0] p = 4'b1111; function automatic int f(input int a); f = ({p, 4'b000x} ? 5 : 6) + a; endfunction endpackage
module t; import k::*; localparam longint A = k::f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:47: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `k.f(…)` has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Battery a08 (`u32::try_from` -> `as u32` in `const_size_cast`): a named size `N = 2**32 + 8`
/// is past u32, so `N'(300)` declines; `as u32` reads an 8-bit cast and folds 44.
/// Loud: iverilog 13 -g2012 + vvp `A a=44`, verilator 5.054 --binary `%Error: t.sv:1:75: Value too wide for 32-bits expected in this context
/// 64'h100000008`.
#[test]
fn k_size_past_u32() {
    check(
        "k_size_past_u32",
        r##"module t; localparam longint N = 64'd4294967304; localparam longint A = N'(300); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:73: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: the cast has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Battery a09 (`1..=64` -> `1..=63` in `const_size_cast`): `64'(e)` in a constant function body
/// folds through ENV's cast arm, which has no later domain, so a 64-bit size declining turns the
/// call loud.
/// iverilog 13 -g2012 + vvp `A a=4`, verilator 5.054 --binary `A a=4`.
#[test]
fn k_size_s64_env() {
    check(
        "k_size_s64_env",
        r##"module t; function automatic longint f(input int a); f = 64'(a - 1) + 64'(5); endfunction localparam longint A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=4
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Battery a11 (the 64-bit negative decline without `!sg`): `64'(a - 5)` has a SIGNED operand, so
/// its negative 64-bit value is the value and folds -5; declining every negative at 64 turns it loud.
/// iverilog 13 -g2012 + vvp `A a=-5`, verilator 5.054 --binary `A a=-5`.
#[test]
fn k_size_neg_s64_env() {
    check(
        "k_size_neg_s64_env",
        r##"module t; function automatic longint f(input int a); f = 64'(a - 5); endfunction localparam longint A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=-5
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Battery a12 (the unsigned 64-bit negative decline dropped): `N'(64'hFFFF_FFFF_FFFF_FFFF)` with
/// `N = 64` is unsigned with its top bit set, which the i64 domain cannot hold, so it declines
/// (`const_size_cast`'s doc: the 64-bit class ROADMAP §2 records for a bare literal); without the
/// decline the bits escape as -1.
/// Loud where the oracles print the 64-bit pattern: iverilog 13 -g2012 + vvp `A a=-1`, verilator 5.054 --binary `A a=-1`.
#[test]
fn k_size_u64_top_env() {
    check(
        "k_size_u64_top_env",
        r##"module t; localparam N = 64; function automatic longint f(input int a); f = N'(64'hFFFF_FFFF_FFFF_FFFF) + a; endfunction localparam longint A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:145: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: `f(…)` has no constant-fold arm [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Promoted lane cell val1_lane/k_size_edges, the only killer of battery a10 (`1..=64` -> `1..=65`):
/// `65'(1)` in a range bound declines (a carry past bit 63 would be lost), so the bound is loud;
/// admitting 65 folds it.
/// Loud: iverilog 13 -g2012 + vvp `A a=-1 b=0 c=-1 bw=3 bw2=3`, sv2v `A a=-1 b=0 c=3 bw=3 bw2=3`, verilator 5.054 --binary
/// `A a=-1 b=0 c=-1 bw=3 bw2=3`.
#[test]
fn k_size_edges() {
    check(
        "k_size_edges",
        r##"module t; localparam longint A = 64'(-1); localparam longint B = 8'(4'sd15 + 4'sd1); localparam longint C = (4'd9+4'd8)'(3); logic [(64'(64'hFFFF_FFFF_FFFF_FFFF) > 0) + 1:0] w; logic [65'(1) + 1:0] w2;
initial begin $display("A a=%0d b=%0d c=%0d bw=%0d bw2=%0d", A, B, C, $bits(w), $bits(w2)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:185: error[VITA-E3009] E-ELAB-UNSUPPORTED: a cast that is not a constant here is not allowed in a constant range bound [in t]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Promoted lane cell val1_lane/k_prim, the only killer of battery a16 (the Prim arm without
/// `coerce_int_width`): `byte'(300)` is 44 and `byte'(259)` 3.
/// iverilog 13 -g2012 + vvp `A a=16 b=44 c=3 d=-1 bw=4`, verilator 5.054 --binary the same.
#[test]
fn k_prim() {
    check(
        "k_prim",
        r##"module t; localparam longint A = int'(4'd15 + 4'd1); localparam longint B = byte'(300); localparam longint C = int'(2.5); localparam longint D = shortint'(-1); logic [byte'(259):0] w;
initial begin $display("A a=%0d b=%0d c=%0d d=%0d bw=%0d", A, B, C, D, $bits(w)); $finish; end endmodule
"##,
        r##"A a=16 b=44 c=3 d=-1 bw=4
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Promoted lane cell val1_lane/k_named, the only killer of battery a27 (the signing arm without the
/// `Named` inner cast): `signed'(RPS'(15))` over a parameter-named size folds -1.
/// iverilog 13 -g2012 + vvp `A a=-1 b=-1 c=43`, verilator 5.054 --binary the same.
#[test]
fn k_named() {
    check(
        "k_named",
        r##"module t; localparam RPS = 4; localparam longint A = signed'(RPS'(15)); localparam longint B = RPS'(31); function automatic longint f(input int a); f = RPS'(a + 31) + 8'(a + 300); endfunction localparam longint C = f(0);
initial begin $display("A a=%0d b=%0d c=%0d", A, B, C); $finish; end endmodule
"##,
        r##"A a=-1 b=-1 c=43
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Promoted lane cell val1_lane/l_shadow2_lognot, the only killer of battery k06 (fold_ctx's `!`
/// in the Unlimited lane): `!` over a frame local that shadows a module constant declines (the local
/// guard); without the guard the wide truth reads the module `p`.
/// Loud: iverilog 13 -g2012 + vvp `A a=0`, sv2v `A a=x`, verilator 5.054 --binary `A a=x`.
#[test]
fn l_shadow2_lognot() {
    check(
        "l_shadow2_lognot",
        r##"module t; localparam logic [3:0] p = 4'b1111; function automatic int f(input int a); logic [3:0] p; p = 4'b0000; f = (!{p, 4'b000x}) + a; endfunction
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

/// Promoted lane cell val1_lane/b_tern_wide, the only killer of battery k07 (fold_ctx's `?:` in the
/// Env lane): a >64-bit x-free condition with no frame local takes the wide truth in the Ctx lane and
/// folds 5; the Env lane has no wide fallback and declines.
/// iverilog 13 -g2012 + vvp `A a=5`, verilator 5.054 --binary `A a=5`.
#[test]
fn b_tern_wide() {
    check(
        "b_tern_wide",
        r##"module t; function automatic int f(input int a);  f = (128'h1_0000_0000_0000_0000_0000 ? 5 : 6) + a; endfunction
localparam longint A = f(0); initial begin $display("A a=%0d", A); $finish; end endmodule
"##,
        r##"A a=5
simulation ended (Finish) at time 0
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}
