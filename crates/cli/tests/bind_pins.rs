//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the BIND-1a lane cells
//! that show the write contract of the nine FQ parameter maps (ROADMAP §5.2 BIND-1a: `bound/`'s
//! DECL-ORDER `bind_decl`, the value funnel's clears, and the undo lanes U3, U5, U6, U8 of
//! `bound/undo.rs`). Every expected text was measured on the PRE binary (6bcd57fa, `vita-post` md5
//! 09768204 ; W/bind1/build_a/gen) with `t.sv` as the argument (`g08_dash_g_overrides`: the `-G`
//! arguments first). A named mutant killer here is a claim until its mutant runs. p19-p22 (BIND-1b
//! delta 1) pin the read lanes of `bound/read.rs` the narrow battery left unkilled (R2, R4, R5, W2),
//! measured on the same PRE binary, each mutant built and run on its source (W/bind1/build_b/delta1).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita <args> t.sv` in a temp dir of its own.
fn run(src: &str, args: &[&str]) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_bind_pins_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
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

fn check(id: &str, src: &str, args: &[&str], want_stdout: &str, want_stderr: &str, want_rc: i32) {
    let (so, se, rc) = run(src, args);
    assert_eq!(so, want_stdout, "{id}: stdout");
    assert_eq!(se, want_stderr, "{id}: stderr");
    assert_eq!(rc, Some(want_rc), "{id}: exit code");
}

/// DECL-ORDER's declaration window (`bind_decl`: the range fold runs with the hier copy and the meta written and the value not). A generate-scope declaration that names an outer one of its own name. Owner line: BIND-1 filing F-a (NEW §2, SILENT): iverilog and verilator both error on this source (circular / recursive parameter) ; vita's value is the record, not the oracle's.
#[test]
fn p18() {
    check(
        "p18",
        r##"module top;
  localparam logic [7:0] P = 8'd5;
  if (1) begin : g
    localparam logic [P:0] P = 3;
    initial $display("A %0d %0d", P, $bits(P));
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 3 4
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// The guessed test runs after the value write cleared the previous mark ; the generate scope re-folds a self-reference each phase. Owner line: BIND-1 filing F-a (NEW §2, SILENT): iverilog "Recursive parameter reference found involving X", verilator "Variable's initial value is circular: 'X'" ; PRE `A X=4 bw=3`.
#[test]
fn p02() {
    check(
        "p02",
        r##"module top;
  localparam X = 1;
  if (1) begin : g
    localparam X = X + 1;
    logic [X:0] w;
    initial $display("A X=%0d bw=%0d", X, $bits(w));
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A X=4 bw=3
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// META-WALK lane: the meta walk reads `param_meta` alone, so the value comes from the inner real twin and the type from the outer key. Owner line: §2 gen-real-param-shadow (BLOCKED), BIND-1 filing F-b: iverilog `A 1 20 20.0`, verilator errors ; PRE `A 4 4 20.000000`.
#[test]
fn p16() {
    check(
        "p16",
        r##"module top;
  localparam logic [3:0] N = 4'd7;
  if (1) begin : g
    localparam real N = 20;
    logic [N-1:0] w;
    initial $display("A %0d %0d %f", $bits(N), $bits(w), N);
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 4 4 20.000000
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// The meta walk's exact key (alias lane). Owner line: §2 gen-scope-alias-width (OPEN).
#[test]
fn p15() {
    check(
        "p15",
        r##"module top;
  localparam logic [3:0] Q = 4'd9;
  localparam R0 = Q;
  if (1) begin : g
    localparam R = Q;
    initial $display("A %b %0d", {R}, $bits(R));
  end
  initial $display("B %b %0d", {R0}, $bits(R0));
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 00000000000000000000000000001001 32
B 1001 4
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// U3 SIDE8: the instance-array prepass binds a child's header at the parent's prefix and puts the eight side maps back (`side_snapshot` / `side_restore`).
#[test]
fn p08() {
    check(
        "p08",
        r##"module c #(parameter logic [23:16] P = 8'hc3) (input a); endmodule
module top;
  localparam logic [15:8] P = 8'h5a;
  logic a;
  c u [1:0] (.a(a));
  initial $display("A %h %h %h %h", P, P[15:8], u[1].P, u[0].P[23:20]);
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 5a 5a c3 c
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// U3 SIDE8 with a non-zero LSB and a select in a port width: the parent's range survives the prepass.
#[test]
fn g07() {
    check(
        "g07",
        r##"module c #(parameter logic [23:16] P = 8'hc3, parameter logic [11:4] Q = 8'h81) (input [P[19:16]:0] a);
  initial $display("C %m %h %h %h %0d", P, P[23:20], Q[7:4], $bits(a));
endmodule
module top;
  localparam logic [15:8] P = 8'h5a;
  localparam logic [9:2] Q = 8'h3c;
  logic [3:0] a;
  c #(.Q(8'hf0)) u [1:0] (.a(a));
  initial $display("A %h %h %h %h %h %h", P, P[15:8], Q[5:2], u[1].P, u[0].P[23:20], u[1].Q[7:4]);
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 5a 5a c c3 c 0
C top.u[1] c3 c 0 4
C top.u[0] c3 c 0 4
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// HIER-IMMORTAL: hierarchical reads after the instance's unwind read the persistent maps.
#[test]
fn p13() {
    check(
        "p13",
        r##"module c #(parameter logic [99:0] K = {36'h3, 64'h7}) (); endmodule
module top;
  c u();
  initial $display("A %h", u.K);
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 0000000030000000000000007
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// HIER-IMMORTAL: a hierarchical read of a ranged / typed parameter after the unwind (hier range and meta kept).
#[test]
fn g15b() {
    check(
        "g15b",
        r##"module c #(parameter logic [7:4] P = 4'h9, parameter U = 3) ();
  localparam real RR = 2.0; localparam string SS = "cs"; localparam logic [69:0] KK = 70'h3_0000_0000_0000_0001;
endmodule
module top;
  c #(.U(8'sh80)) u(); c v();
  initial $display("A %h %0d %0d %0d %h %0d %0d", u.P, u.P[5:4], u.U, $bits(u.U), u.KK, v.U, $bits(v.U));
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 9 1 -128 8 030000000000000001 3 32
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// The value funnel clears the range on a re-bind (`bind_param_value`): an import's range does not survive a local re-bind.
#[test]
fn g13() {
    check(
        "g13",
        r##"package pk; parameter [39:8] W = 32'h1234_be00; parameter [39:8] X = 32'h0000_be00; endpackage
module top;
  import pk::*;
  typedef enum { W = 32'hDEADBEEF } e_t;
  for (genvar X = 0; X < 1; X++) begin : g
    initial $display("B %0d", X[15:8]);
  end
  initial $display("A %0d %0d", W[15:8], X[15:8]);
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"B 0
A 190 0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// T3 (header) under the five override channels: the range chain and the guessed test run inside `bind_decl` at PRE's positions.
#[test]
fn g12b() {
    check(
        "g12b",
        r##"module c #(parameter P = 5, parameter Q = P + 1, parameter logic [7:0] T = 8'd1) ();
  initial $display("A %m %0d %0d %b %0d %0d %h", P, $bits(P), {Q}, $bits(Q), $bits(T), 64'(-P));
endmodule
module top;
  c #(.P(8'shF0)) u1();
  c #(.P(16'd3), .T(-1)) u2();
  c #(.P('1)) u3();
  c #(.P(~8'h5A)) u4();
  c u5();
  defparam u5.P = 4'sd7;
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A top.u1 -16 8 11111111111111111111111111110001 32 8 0000000000000010
A top.u2 3 16 00000000000000000000000000000100 32 8 fffffffffffffffd
A top.u3 1 1 00000000000000000000000000000010 32 8 ffffffffffffffff
A top.u4 165 8 00000000000000000000000010100110 32 8 ffffffffffffff5b
A top.u5 7 4 00000000000000000000000000001000 32 8 fffffffffffffff9
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// U6 routine genvar replay: the value is dropped raw (`drop_value_raw`, RTN-RAW-REMOVE) and the meta put back.
#[test]
fn g09b() {
    check(
        "g09b",
        r##"module top;
  for (genvar i = 0; i < 3; i++) begin : g
    function automatic logic [7:0] f(input logic [7:0] x);
      f = x + i[1:0] + $bits(i);
    endfunction
    task automatic t(output logic [7:0] y); y = {i}[3:0] + i[0]; endtask
    logic [7:0] w, z;
    always_comb w = f(8'd2);
    initial begin t(z); #0 $display("A %0d %0d %0d %0d %0d", i, i[1:0], $bits(i), w, z); end
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 0 0 32 34 0
A 1 1 32 35 2
A 2 2 32 36 2
simulation ended (Finish) at time 1
"##,
        r##"t.sv:6:49: warning[VITA-W2004] W-PARSE-SELECT-BASE: a bit/part select here applies to an expression, not to a net or variable — IEEE 1800-2017 §11.5.1 allows one only on a variable reference (a name, possibly indexed or member-selected). vita accepts it; iverilog rejects every form of it and Verilator rejects a select on a parenthesised expression or on a literal. Assign the value to a variable first, then select from that
warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=2 notes=0
"##,
        0,
    );
}

/// Strings on the instance-array path (SIDE8 puts `str_param_raw` back).
#[test]
fn g05() {
    check(
        "g05",
        r##"module c #(parameter string S = "el", parameter T = "ty") (input a);
  initial $display("A %m %s %s", S, T);
endmodule
module top;
  localparam string S = "parent";
  localparam T = "pt";
  logic a;
  c #(.S("ov")) u [1:0] (.a(a));
  initial $display("B %s %s %0d", S, T, $bits(T));
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"B parent pt 16
A top.u[1] ov ty
A top.u[0] ov ty
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// apply_import_consts per instance: meta set and wide cleared / set through the primitives.
#[test]
fn g11b() {
    check(
        "g11b",
        r##"package pk; localparam logic [11:4] W = 8'hcb; localparam logic [79:0] K = 80'h1_0000_0000_0000_0007; localparam signed [3:0] N4 = -4'sd3; endpackage
module c #(parameter N = 1) (); import pk::*;
  localparam M = W[7:4] + N;
  initial $display("C %m %h %0d %h %0d %0d %0d", W, M, K, $bits(K), N4, $bits(N4));
endmodule
module top;
  c #(1) u1(); c #(2) u2();
  if (1) begin : g c #(3) u3(); end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"C top.u1 cb 12 00010000000000000007 80 -3 4
C top.u2 cb 13 00010000000000000007 80 -3 4
C top.g.u3 cb 14 00010000000000000007 80 -3 4
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// The value funnel clears the HIER range too: a generate-scope genvar `R` records (0, 32) and the later real twin's integer re-bind clears it, so `g.R[3:2]` stays loud (BIND-1a M5's claimed killer).
#[test]
fn g16c() {
    check(
        "g16c",
        r##"module top;
  if (1) begin : g
    for (genvar R = 0; R < 1; R++) begin : h end
    localparam real R = 12;
  end
  initial begin #1 $display("A %0d %0d %0d", g.R, g.R[3:2], $bits(g.R)); $finish; end
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
error[VITA-E3009] E-ELAB-UNSUPPORTED: a bit / part-select of the hierarchical parameter `g.R` needs its declared width, and this parameter is sized from its value (no range, type or sized literal)
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// U5: the genvar loop suspends an outer real constant of its name (`clear_real`) and puts it back under `if let` (R10) ; the sub instances bind the genvar, not 2.5 (M11's claimed killer).
#[test]
fn g17() {
    check(
        "g17",
        r##"module sub #(parameter P = 0); initial $display("B %m %0d", P); endmodule
module top;
  localparam real N = 2.5;
  for (genvar N = 0; N < 2; N++) begin : g sub #(.P(N)) u(); end
  initial begin #1 $display("A %f", N); $finish; end
endmodule
"##,
        &[],
        r##"B top.g[0].u 0
B top.g[1].u 1
A 2.500000
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// U8: a package function and task read their package's typed constants (pushed with their meta, put back after the body ; wide never put back).
#[test]
fn g18() {
    check(
        "g18",
        r##"package pk;
  localparam logic [3:0] K = 4'hC;
  localparam logic signed [7:0] S = -8'sd3;
  function automatic logic [15:0] f(input logic [15:0] a); return a + K + S; endfunction
  task automatic t(output logic [15:0] o); o = {K, 4'h0} + S; endtask
endpackage
module top;
  import pk::*;
  logic [15:0] y, z;
  localparam K2 = $bits(K);
  initial begin y = f(16'd1); t(z); #1 $display("A %h %h %0d %0d %0d", y, z, $bits(K), $bits(S), K2); $finish; end
endmodule
"##,
        &[],
        r##"A 010a 01bd 4 8 4
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// DECL-ORDER's declaration window at the generate scope: `P[5:4]` in `Z`'s initialiser reads the inner `P`'s range, folded with the hier copy and the meta written and the value not. BIND-1a R11: M1 (the range fold moved after the value write) built and run on this source answers E3009 "the part-select `P[…]` has no constant-fold arm" with exit code 1 (W/bind1/build_a/m1_probe) ; PRE `A 3 4 2 3`. Owner line: BIND-1 filing F-a (the window's self-reference).
#[test]
fn j09() {
    check(
        "j09",
        r##"module top;
  localparam logic [7:0] P = 8'd5;
  if (1) begin : g
    localparam logic [P:0] P = 3;
    localparam int Z = P[5:4] + 2;
    logic [Z:0] w;
    initial $display("A %0d %0d %0d %0d", P, $bits(P), Z, $bits(w));
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 3 4 2 3
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// T3 reached by `-G` top overrides (typed: `-G W=8'shF0` binds -16 at 8 bits ; string, real, wide channels).
#[test]
fn g08_dash_g_overrides() {
    check(
        "g08_dash_g_overrides",
        r##"module top #(parameter W = 4, parameter logic [7:4] L = 4'h3, parameter string S = "d", parameter real R = 1.5, parameter logic [99:0] K = 100'h1);
  localparam D = W + 1;
  initial $display("A %0d %0d %h %0d %s %f %h %0d %0d", W, $bits(W), L, L[5:4], S, R, K, D, $bits(D));
  initial #1 $finish;
endmodule
"##,
        &[
            r#"-G"#,
            r#"W=8'shF0"#,
            r#"-G"#,
            r#"L=4'hc"#,
            r#"-G"#,
            r#"S="gs""#,
            r#"-G"#,
            r#"R=3"#,
            r#"-GK=100'h5_0000_0000_0000_0000_0001"#,
        ],
        r##"A -16 8 c 0 gs 3.000000 0000500000000000000000001 -15 32
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// BIND-1b `hit_value_or_net`'s `symbols` arm (`expr_special.rs` `bits_of_view`'s shadow test): a generate-scope net shadows an outer typed localparam of its name, so `$bits(N)` names the net. Delta 1: R2 (the arm dropped) built and run on this source answers `A 4` with exit code 0 (W/bind1/build_b/delta1/probe/r2a) ; PRE `A 16`. iverilog 13.0 `A 16`, verilator 5.054 `A 16`.
#[test]
fn p19_gen_net_shadow_bits() {
    check(
        "p19_gen_net_shadow_bits",
        r##"module top;
  localparam logic [3:0] N = 4'd7;
  if (1) begin : g
    logic [15:0] N;
    initial $display("A %0d", $bits(N));
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 16
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// BIND-1b `hit_wide_value_or_net`'s `wide_param_bits` arm (`const_wide.rs` `wide_name_bits`): a generate-scope >64-bit localparam (in `wide_param_bits` alone) shadows an outer narrow one of its name, so `J = K` folds the inner `K`. Delta 1: R4 (the arm dropped) built and run on this source answers `A 0000000010000000000000005 0000000000000000000000005` with exit code 0 (W/bind1/build_b/delta1/probe/r4b ; the unshadowed `J = K + 1` of probe r4a goes to E3009 "`K` is wider than 64 bits" with exit code 1) ; PRE `A 0000000010000000000000005 0000000010000000000000005`. iverilog 13.0 and verilator 5.054 both `A 0000000010000000000000005 0000000010000000000000005`.
#[test]
fn p20_gen_wide_shadow_narrow() {
    check(
        "p20_gen_wide_shadow_narrow",
        r##"module top;
  localparam logic [7:0] K = 8'd5;
  if (1) begin : g
    localparam logic [99:0] K = {36'h1, 64'h5};
    localparam logic [99:0] J = K;
    initial $display("A %h %h", K, J);
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 0000000010000000000000005 0000000010000000000000005
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// BIND-1b `hit_real_value_or_net`'s `real_param_val` arm (`param_query.rs` `real_param_is_non_integral`, read by `const_bound.rs`'s range-bound guard): a non-integral real localparam (no `params` twin) in a range bound is named as a real parameter. Delta 1: R5 (the arm dropped) built and run on this source answers E3009 "undefined name `R` is not allowed in a constant range bound" with exit code 1 (W/bind1/build_b/delta1/probe/r5a) ; PRE the real-parameter E3009 below. iverilog 13.0 accepts it and prints `A 4` ; verilator 5.054 `%Error: t.sv:3:10: Value too wide for 32-bits expected in this context 2.5` ; vita's text is the record, not an oracle's.
#[test]
fn p21_real_bound_nonintegral() {
    check(
        "p21_real_bound_nonintegral",
        r##"module top;
  localparam real R = 2.5;
  logic [R:0] w;
  initial $display("A %0d", $bits(w));
  initial #1 $finish;
endmodule
"##,
        &[],
        "",
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:10: error[VITA-E3009] E-ELAB-UNSUPPORTED: a real parameter is not an integral constant and cannot be used in a width / range bound (assign it to an integer localparam first) [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// BIND-1b REAL-WALK `name_real`'s outward step (`const_real.rs` `const_eval_real_in_scope`): a generate-scope real localparam folds an outer real one. Delta 1: W2 (the walk stopped at the current scope) built and run on this source answers E3009 "generate-scope parameter `S` value is not a constant: `R` is a real, which has no integral constant value here" and E3010 "undeclared net/variable `top.g[0].S`" with exit code 1 (W/bind1/build_b/delta1/probe/w2b) ; PRE `A 5.000000`. iverilog 13.0 `A 5.000000`, verilator 5.054 `A 5.000000`.
#[test]
fn p22_real_outer_gen_fold() {
    check(
        "p22_real_outer_gen_fold",
        r##"module top;
  localparam real R = 2.5;
  if (1) begin : g
    localparam real S = R * 2;
    initial $display("A %f", S);
  end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##"A 5.000000
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

// ---- batch 4 boundary, pins r1: killers of the battery's survivors (W/batch4/battery/SUMMARY.txt), each
// input measured on PRE 6bcd57fa and POST f17a0ff6 (the same stdout, stderr and exit code).

/// Batch 4 boundary (pins r1): a genvar loop over `P` (block `g`), then an ascending net `logic [0:7] P = 8'hA5` and five selects of it. Kills mutant a05 (`unbind_param` keeps `param_range`): E3009 x2, rc 1.
/// Owner line: the value funnel's clears (`bound/write.rs` `unbind_param`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A a 5 5 0 1` / `t.sv:8: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-ASCRANGE: t.sv:5:9: Ascending bit range vector: left < right of bit range: [0:7]`.
#[test]
fn n01_gv_dead_range() {
    check(
        "n01",
        r##"module top;
  for (genvar P = 0; P < 2; P++) begin : g
    localparam int L = P;
  end
  logic [0:7] P = 8'hA5;
  int k = 1;
  initial #1 $display("A %h %h %h %b %h", P[0:3], P[4+:4], P[4:7], P[k], P[k+:2]);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A a 5 5 0 1
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): `top` declares `localparam int P = 2` and `localparam int W = 3`, an instance array `c u [1:0]` whose header declares `int P = 6` and `logic [99:0] W`, then `c2 #(.Q(P), .R(W)) w()`. Kills mutants b01 (`restore_params`' `Some` arm dropped: `Q 6 R 3`) and b14 (SIDE8 leaves `wide_param_bits`: `Q 2 R 5`).
/// Owner line: the undo lanes U1 / U3 (`bound/undo.rs`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `Q 2 R 3` / `t.sv:12: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn n02_prepass_putback() {
    check(
        "n02",
        r##"module c #(parameter int P = 6, parameter logic [99:0] W = 100'h1_0000_0000_0000_0000_0005) (input a);
endmodule
module c2 #(parameter int Q = 0, parameter int R = 0) ();
  initial #1 $display("Q %0d R %0d", Q, R);
endmodule
module top;
  localparam int P = 2;
  localparam int W = 3;
  logic a;
  c u [1:0] (.a(a));
  c2 #(.Q(P), .R(W)) w();
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"Q 2 R 3
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): `top` declares no `P` ; an instance array `c u [1:0]` whose header declares `int P = 6`, then `c2 #(.Q(P)) w()`. Kills mutant b02 (`restore_params`' `None` arm dropped: `Q 6` at rc 0).
/// Owner line: the undo lane U1 (`bound/undo.rs` `restore_params`) ; PRE 6bcd57fa and POST f17a0ff6 print the same (W3056 + E3009, rc 1).
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:9: error: Unable to bind parameter `P' in `top'` / `1 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Warning-IMPLICIT: t.sv:9:11: Signal definition not found, creating implicitly: 'P'` / `%Error: t.sv:9:11: Expecting expression to be constant, but variable isn't const: 'P'` / `%Error: t.sv:9:9: Can't convert defparam value to constant: Param 'Q' of 'w'`.
#[test]
fn n03_prepass_unbind() {
    check(
        "n03",
        r##"module c #(parameter int P = 6) (input a);
endmodule
module c2 #(parameter int Q = 0) ();
  initial #1 $display("Q %0d", Q);
endmodule
module top;
  logic a;
  c u [1:0] (.a(a));
  c2 #(.Q(P)) w();
  initial #2 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:6:8: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: override of parameter `Q` is not a constant; default kept [in top]
t.sv:9:15: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `Q` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in top.w]
errors=1 warnings=2 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r1): module localparams `logic [8:15] V` and `logic [15:8] U`, and in generate block `g` nets `logic [7:0] V` and `logic [0:7] U`, and two part-selects of each. Kills mutant k10 (params.rs:934 in `param_sel_range` asks `hit_value` for `hit_value_or_net`): E3009 x4, rc 1.
/// Owner line: BIND-1b's read lanes (`bound/read.rs` `hit_value_or_net`, site params.rs `param_sel_range`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A c 3 c 3` / `t.sv:9: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-ASCRANGE: t.sv:6:11: Ascending bit range vector: left < right of bit range: [0:7]`.
#[test]
fn n04_net_dir_shadow() {
    check(
        "n04",
        r##"module top;
  localparam logic [8:15] V = 8'h5a;
  localparam logic [15:8] U = 8'h5a;
  if (1) begin : g
    logic [7:0] V = 8'hc3;
    logic [0:7] U = 8'hc3;
    initial #1 $display("A %h %h %h %h", V[7:4], V[3:0], U[0:3], U[4:7]);
  end
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A c 3 c 3
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): a module net `logic [15:8] V = 8'h5a`, then a genvar loop over `V` whose block holds a function returning `a + V`. Kills mutant a27 (`drop_value_raw` a no-op): E3009, rc 1.
/// Owner line: the raw value remove of lane RTN-RAW-REMOVE (`bound/write.rs` `drop_value_raw`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `G 1` / `G 2` / `A 5a a` / `t.sv:11: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn n05_replay_genvar() {
    check(
        "n05",
        r##"module top;
  logic [15:8] V = 8'h5a;
  for (genvar V = 0; V < 2; V++) begin : g
    function automatic int f(input int a);
      return a + V;
    endfunction
    int r;
    initial begin r = f(1); $display("G %0d", r); end
  end
  initial #1 $display("A %h %h", V, V[11:8]);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"G 1
G 2
A 5a a
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): `top` declares `localparam logic [15:8] P = 8'h5a` and `localparam S = "QR"`, instance arrays of `cs` (header `P = "XY"`) and `cr` (header `real S = 4.5`), then `c2 #(.Q(P), .R(S)) w()`. Kills mutants b17 (SIDE8 leaves `str_param_raw`: `Q 22617 R 20818`) and b18 (SIDE8 leaves `real_param_val`: E3009, rc 1).
/// Owner line: the undo lane U3 (`bound/undo.rs` `side_snapshot` / `side_restore`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `Q 90 R 20818` / `t.sv:15: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:5:27: Operator VAR 'Q' expects 32 bits on the Initial value, but Initial value's CONST '8'h5a' generates 8 bits.`.
#[test]
fn n06_side_str_real() {
    check(
        "n06",
        r##"module cs #(parameter P = "XY") (input a);
endmodule
module cr #(parameter real S = 4.5) (input a);
endmodule
module c2 #(parameter int Q = 0, parameter int R = 0) ();
  initial #1 $display("Q %0d R %0d", Q, R);
endmodule
module top;
  localparam logic [15:8] P = 8'h5a;
  localparam S = "QR";
  logic a;
  cs u [1:0] (.a(a));
  cr v [1:0] (.a(a));
  c2 #(.Q(P), .R(S)) w();
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"Q 90 R 20818
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): an instance `c v` and an instance array `c u [1:0]` of `c` (header `logic [23:16] P = 8'hc3`), and the hierarchical references `v.P`, `v.P[23:20]`, `u[0].P[19:16]`. Kills mutants b22 (`side_restore` swaps `param_range` and `hier_param_range`) and b23 (`side_snapshot` swaps them): `A c3 x 3`.
/// Owner line: the undo lane U3 (`bound/undo.rs`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A c3 c 3` / `t.sv:8: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:7:59: Can't find definition of 'u'`.
#[test]
fn n07_side_range_hier() {
    check(
        "n07",
        r##"module c #(parameter logic [23:16] P = 8'hc3) (input a);
endmodule
module top;
  logic a;
  c v (.a(a));
  c u [1:0] (.a(a));
  initial #1 $display("A %h %h %h", v.P, v.P[23:20], u[0].P[19:16]);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A c3 c 3
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): module localparams `int V = 9`, `logic [99:0] U` and `logic [99:0] X`, and in generate block `g` nets `logic [7:0] V`, `logic [7:0] U`, a localparam `int X = 3` and `A = $bits(V)`, `B = $bits(U)`, `C = $bits(X)`. Kills mutants k07 (const_wide.rs:1256 in `narrow_param_bits` asks `hit_value` for `hit_value_or_net`: `D 32 8 32`), k08 (const_wide.rs:1375 in `wide_name_bits` asks `hit_wide` for `hit_wide_value_or_net`: `D 8 100 100`), r02 (`hit_wide_value_or_net` without its `params` arm: `D 8 8 100`) and r03 (without its `symbols` arm: `D 8 100 32`).
/// Owner line: BIND-1b's read lanes (`bound/read.rs`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `D 8 8 32` / `t.sv:14: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, no %Error or %Warning line (lint only: its value was not run).
#[test]
fn n08_net_shadows_bits() {
    check(
        "n08",
        r##"module top;
  localparam int V = 9;
  localparam logic [99:0] U = 100'hF_0000_0000_0000_0000_00a5;
  localparam logic [99:0] X = 100'hF_0000_0000_0000_0000_00a5;
  if (1) begin : g
    logic [7:0] V = 8'hc3;
    logic [7:0] U = 8'hc3;
    localparam int X = 3;
    localparam int A = $bits(V);
    localparam int B = $bits(U);
    localparam int C = $bits(X);
    initial #1 $display("D %0d %0d %0d", A, B, C);
  end
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"D 8 8 32
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r1): module localparams `real V = 2.5` and `real U = 2.5` ; generate block `g` declares `localparam logic [7:0] V` and `S = V[11:4]`, block `h` a net `logic [7:0] U` and `T = U[11:4]`. Kills mutants r04 (`hit_real_value_or_net` without its `params` arm: `g`'s reason becomes "the part-select `V[…]` is a real") and r05 (without its `symbols` arm: `h`'s reason becomes "`U` is a real"). Both oracles refuse this text (rc 1) ; the expected text is PRE's.
/// Owner line: BIND-1b's read lanes (`bound/read.rs`) ; PRE 6bcd57fa and POST f17a0ff6 print the same (E3009 x2, rc 1).
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:10: error: A reference to a net or variable (`U['sd11:'sd4]') is not allowed in a constant expression.` / `1 error(s) during elaboration.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Warning-SELRANGE: t.sv:6:33: Selection index out of range: 11:4 outside 7:0` / `%Warning-SELRANGE: t.sv:10:33: Selection index out of range: 11:4 outside 7:0` / `%Error: t.sv:10:32: Expecting expression to be constant, but variable isn't const: 'U'`.
#[test]
fn n09_real_shadow_reason() {
    check(
        "n09",
        r##"module top;
  localparam real V = 2.5;
  localparam real U = 2.5;
  if (1) begin : g
    localparam logic [7:0] V = 8'hc3;
    localparam logic [7:0] S = V[11:4];
  end
  if (1) begin : h
    logic [7:0] U = 8'hc3;
    localparam logic [7:0] T = U[11:4];
  end
  initial #2 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:6:32: error[VITA-E3009] E-ELAB-UNSUPPORTED: generate-scope parameter `S` value is not a constant: the part-select [11:4] is outside `V`'s range [7:0] [in top.g]
t.sv:10:32: error[VITA-E3009] E-ELAB-UNSUPPORTED: generate-scope parameter `T` value is not a constant: a reference to net/variable `U` is not a constant [in top.h]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r1): a variable `logic [3:0] n` as the width of `v[n +: n]`. Kills mutant k02 (edge_gate.rs:1440 in `variable_named_in` asks `hit_wide_value_or_net` for `hit_wide`: the reason becomes "a reference to net/variable `n` is not a constant"). Both oracles refuse this text (iverilog rc 2, verilator rc 1) ; the expected text is PRE's.
/// Owner line: BIND-1b's read lanes (`bound/read.rs`) ; PRE 6bcd57fa and POST f17a0ff6 print the same (E3009, rc 1). Found on lens-diff cell e17_cast_width_net_and_sel_net.
/// Oracles: iverilog 13.0 -g2012: compile rc=2, `t.sv:4: error: A reference to a net or variable (`n') is not allowed in a constant expression.` / `t.sv:4: error: Indexed part select width must be an integral constants greater than zero.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:4:41: Expecting expression to be constant, but variable isn't const: 'n'` / `%Error: t.sv:4:41: Width of :+ or :- bit slice range isn't a constant`.
#[test]
fn n10_var_width_reason() {
    check(
        "n10",
        r##"module top;
  logic [3:0] n = 2;
  logic [15:0] v = 16'h1234;
  initial begin $display("A %h", v[n +: n]); end
  initial #1 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:41: error[VITA-E3009] E-ELAB-UNSUPPORTED: the width of this indexed part-select does not fold to a constant: it reads `n`, a variable, where a constant expression is required [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r1): `localparam real R = 4` and `localparam int B = $bits($signed(R))`. Kills mutant k20 (select_scope.rs:382 in `signing_operand_admitted`: the lane call replaced by `real_param_val.get(R)`, `A 64` rc 0) ; bound_home W5d kills it as well (W/batch4/pins/out2/SUMMARY.txt). iverilog prints `A 1` and verilator refuses (rc 1) ; the expected text is PRE's.
/// Owner line: BIND-1b's read lanes (`bound/read.rs` `name_real`) ; PRE 6bcd57fa and POST f17a0ff6 print the same (E3009, rc 1).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A 1` / `t.sv:5: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:3:36: Expected integral input to SIGNED`.
#[test]
fn n11_bits_signed_real() {
    check(
        "n11",
        r##"module top;
  localparam real R = 4;
  localparam int B = $bits($signed(R));
  initial #1 $display("A %0d", B);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:3:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `B` value is not a constant: `$signed(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r1): `localparam real R = 2.0` and an untyped `localparam W = 100'h1_0000_0000_0000_0000_4142`, each in a generate-if condition `(<name> == "AB")` (blocks `g`, `h`). Kills mutants r06 (`hit_any_const` without its `real_param_val` arm: `g`'s reason becomes "undefined name `R` is not a constant") and r08 (without its `wide_param_bits` arm: `h`'s reason becomes "undefined name `W` is not a constant"). PRE prints two E3010 (rc 1) where both oracles run at rc 0 ; the expected text is PRE's.
/// Owner line: BIND-1b's read lanes (`bound/read.rs` `hit_any_const`) ; PRE 6bcd57fa and POST f17a0ff6 print the same (E3010 x2, rc 1).
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `t.sv:10: $finish called at 2 (1s)` (no other line) ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:7:9: Operator EQ expects 100 bits on the RHS, but RHS's CONST '16'h4142' generates 16 bits.`.
#[test]
fn n12_cond_str_reason() {
    check(
        "n12",
        r##"module top;
  localparam real R = 2.0;
  localparam W = 100'h1_0000_0000_0000_0000_4142;
  if (R == "AB") begin : g
    initial $display("Y");
  end
  if (W == "AB") begin : h
    initial $display("Z");
  end
  initial #2 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:7: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: generate-if condition is not a constant: a condition that holds a string literal reads only a parameter declared once in the module, outside every `generate` region and block, as an `int`, an `integer` or a type with a packed range; it does not read `R` [in top]
t.sv:7:7: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: generate-if condition is not a constant: a condition that holds a string literal reads only a parameter declared once in the module, outside every `generate` region and block, as an `int`, an `integer` or a type with a packed range; it does not read `W` [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r1): package `p` declares `localparam logic [7:4] L = 4'ha`, `localparam real R = 3.0` and the functions `fl` (return range `[$high(L):0]`) and `fr` (`[$rtoi(R):0]`) ; `top` declares `localparam int L = 1` and `localparam int R = 1` and prints `$bits(p::fl(1))`, `$bits(p::fr(1))`: `A 8 4`. Kills mutants b09 (the kept `real_param_val` entry not put back: `A 8 2`), b12 (the kept `param_range` entry not put back: `A 32 4`) and b13 (the kept range put into `hier_param_range`: `A 32 4`).
/// Measured (W/batch4/pins-judge3/var/RESULTS.txt, PRE = POST): p's `L` changed to `logic [9:4] L = 6'h2a` moves the output to `A 10 4` and p's `R` changed to 5.0 moves it to `A 8 6`, while top's `R` changed to 5 and top's `L` declared `logic [11:4] L = 1` leave `A 8 4`.
/// Owner line: the undo lane U4 (`bound/undo.rs` `decl_keep_restore`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A 4 4` / `t.sv:11: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:4:58: Operator ASSIGN expects 8 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.` / `%Warning-WIDTHTRUNC: t.sv:5:58: Operator ASSIGN expects 4 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.` (8 and 4, as vita).
#[test]
fn n13_pkg_window_kept() {
    check(
        "n13",
        r##"package p;
  localparam logic [7:4] L = 4'ha;
  localparam real R = 3.0;
  function automatic logic [$high(L):0] fl(input int a); return a; endfunction
  function automatic logic [$rtoi(R):0] fr(input int a); return a; endfunction
endpackage
module top;
  localparam int L = 1;
  localparam int R = 1;
  initial #1 $display("A %0d %0d", $bits(p::fl(1)), $bits(p::fr(1)));
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 8 4
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

// ---- batch 4 boundary, pins r2: killers of the survivors the pins-judge r1 found witnesses for
// (W/batch4/pins-judge/REPORT.txt), each input measured on PRE 6bcd57fa and POST f17a0ff6 (the same stdout,
// stderr and exit code) and on each mutant's release binary (W/batch4/pins/r2/o).

/// Batch 4 boundary (pins r2): package `q` declares `localparam logic [19:5] A = 15'h0035` ; package `p` imports `q::*`, then declares `typedef enum logic [11:4] {A = 'h35, B}` and `f` with the return range `[A[8:5]:0]` ; `top` declares `localparam logic [31:0] A = 32'h35` and prints `$bits(p::f(1))`, `p::A`: `A 6 53`.
/// Measured (W/batch4/pins/r3/var, PRE = POST): p's label `A = 'h3f` gives `A 16 63` and q's range `[19:4]` gives `A 11 53`, while q's value `15'h001f` and top's `A` (`32'h1E0`, `int 7`) leave `A 6 53`.
/// Kills mutants b29 (the Some arm dropped: `A 2 53`) and b34 (the range put back into `hier_param_range`: `A 2 53`) ; both then follow top's `A` (`32'h1E0`: `A 16 53`, `int 7`: `A 1 53`).
/// Owner line: the undo lane U7 (`bound/undo.rs` `put_back_range_raw`) ; PRE 6bcd57fa and POST f17a0ff6 print the same. No oracle agreement: vita 6, iverilog 1, verilator 11 (W/batch4/pins/DOCS_FINDINGS.txt b) ; the expected text is PRE's.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A 1 53` / `t.sv:12: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:7:55: Operator ASSIGN expects 11 bits on the Assign RHS, but Assign RHS's VARREF 'x' generates 32 bits.` ; verilator --binary run `A 11 53`.
#[test]
fn n14_pkg_import_label_range() {
    check(
        "n14",
        r##"package q;
  localparam logic [19:5] A = 15'h0035;
endpackage
package p;
  import q::*;
  typedef enum logic [11:4] {A = 'h35, B} e_t;
  function automatic logic [A[8:5]:0] f(input int x); return x; endfunction
endpackage
module top;
  localparam logic [31:0] A = 32'h35;
  initial #1 $display("A %0d %0d", $bits(p::f(1)), p::A);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 6 53
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r2): package `p` declares the enum label `X` twice (an `int`-based enum, then a `[11:4]`-based one) and `f` with the return range `[X[8:5]:0]` ; `top` declares `localparam logic [31:0] X = 32'h0135` and prints `$bits(p::f(1))`, `p::X`: `A 10 53`.
/// Measured (W/batch4/pins/r3/var, PRE = POST): top's `X = 32'h0035` gives `A 2 53` and `32'h01E0` gives `A 16 53`, while p's labels `'h0f5` / `8'hf0` leave the width at 10.
/// Kills mutant b24 (the None arm dropped: `A 2 53` for each of top's three `X` values ; its width follows p's second label instead, `8'hf0` gives `A 8 240`).
/// Invalid text vita accepts at rc 0: both oracles refuse the duplicate (gap entry duplicate-value-name-accepted, W/batch4/pins/DOCS_FINDINGS.txt a).
/// Gap entry: Q-RTNTEXT (§5.2).
/// Owner line: the undo lane U7 (`bound/undo.rs` `put_back_range_raw`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:3: error: 'X' has already been declared in this scope.` / `t.sv:2:      : It was declared here as an enum type or value.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:3:30: Duplicate declaration of enum value: X`.
#[test]
fn n15_pkg_dup_label_label() {
    check(
        "n15",
        r##"package p;
  typedef enum {X = 'h1b5, X_n3} e_0_t;
  typedef enum logic [11:4] {X = 8'h35, X_n2} e_1_t;
  function automatic logic [X[8:5]:0] f(input int a); return a; endfunction
endpackage
module top;
  localparam logic [31:0] X = 32'h0135;
  initial #1 $display("A %0d %0d", $bits(p::f(1)), p::X);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 10 53
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r2): package `p` declares `localparam logic [19:5] X = 15'h0136`, then the enum label `X` on a `[11:4]` base, and `f` with the return range `[X[8:5]:0]` ; `top` declares `localparam logic [31:0] X = 32'h0135` and prints `$bits(p::f(1))`, `p::X`: `A 10 53`.
/// Measured (W/batch4/pins/r3/var, PRE = POST): top's `X = 32'h0035` gives `A 2 53` and `32'h01E0` gives `A 16 53`, while p's `15'h01ff` and label `8'hf0` leave the width at 10.
/// Kills mutant b24 (the None arm dropped: `A 6 53` for each of top's three `X` values ; its width follows p's label instead, `8'hf0` gives `A 1 240`).
/// Invalid text vita accepts at rc 0: both oracles refuse the duplicate (gap entry duplicate-value-name-accepted, W/batch4/pins/DOCS_FINDINGS.txt a).
/// Gap entry: Q-RTNTEXT (§5.2).
/// Owner line: the undo lane U7 (`bound/undo.rs` `put_back_range_raw`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:3: error: 'X' has already been declared in this scope.` / `t.sv:2:      : It was declared here as a parameter.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:3:30: Duplicate declaration of enum value: X`.
#[test]
fn n16_pkg_dup_param_label() {
    check(
        "n16",
        r##"package p;
  localparam logic [19:5] X = 15'h0136;
  typedef enum logic [11:4] {X = 8'h35, X_n2} e_1_t;
  function automatic logic [X[8:5]:0] f(input int a); return a; endfunction
endpackage
module top;
  localparam logic [31:0] X = 32'h0135;
  initial #1 $display("A %0d %0d", $bits(p::f(1)), p::X);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 10 53
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r2): package `p` declares the enum label `X` on a `logic [7:0]` base, then `localparam real X = NOPE;`, then `typedef enum logic [3:0] {E = $bits(X)}` and `localparam int Y = $bits({X, 4'h0})`: PRE prints two E3009, `package parameter `X` value is not a foldable constant` and `enum label `E` = 32 does not fit its 4-bit base type`.
/// Measured (W/batch4/pins/r3/var, PRE = POST): `E` = 32 also with the label on a 6-bit base and with no label at all.
/// Kills mutant a17 (`clear_meta` a no-op: one E3009, errors=1, with this label and with the label on a 6-bit base ; with no label a17 prints PRE's two E3009 ; with `E`'s base `[2:0]` a17 prints `enum label `E` = 8 does not fit its 3-bit base type`).
/// Invalid text on an error path: a duplicate name vita accepts and an undefined `NOPE` (gap entry duplicate-value-name-accepted, W/batch4/pins/DOCS_FINDINGS.txt a).
/// Owner line: the value funnel's clears (`bound/write.rs` `clear_meta`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:3: error: 'X' has already been declared in this scope.` / `t.sv:2:      : It was declared here as an enum type or value.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:3:19: Unsupported in C: Variable has same name as ENUMITEM 'X': 'X'` / `%Error: t.sv:3:23: Can't find definition of variable: 'NOPE'`.
#[test]
fn n17_pkg_dup_meta_clear() {
    check(
        "n17",
        r##"package p;
  typedef enum logic [7:0] {X = 8'h35, XX} e_t;
  localparam real X = NOPE;
  typedef enum logic [3:0] {E = $bits(X)} t2_t;
  localparam int Y = $bits({X, 4'h0});
endpackage
module top;
  initial #1 $display("A %0d", p::Y);
  initial #2 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
error[VITA-E3009] E-ELAB-UNSUPPORTED: package parameter `X` value is not a foldable constant [in p]
error[VITA-E3009] E-ELAB-UNSUPPORTED: enum label `E` = 32 does not fit its 4-bit base type (§6.19); no reading of a 4-bit value reaches it (the widest range is [-8, 15]) [in p]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r2): module `c`'s untyped header `P = 5` is overridden `#(.P(-3))` ; its static function `f` declares the enum label `P = 8'd3` in its body. Kills mutant a02 (`bind_param_value` keeps the guessed mark: `A 00000000fffffffb 0000000000000001 0000000000000003`) ; n19's mutant b04 fails it as well.
/// Owner line: the value funnel (`bound/write.rs` `bind_param_value`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A ffffffff00000001 ffffffff00000001 0000000000000003` / `t.sv:11: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0 (four WIDTHEXPAND warnings) ; verilator --binary run `A fffffffffffffffb ffffffff00000001 0000000000000003` (as vita).
#[test]
fn n18_fn_label_guessed() {
    check(
        "n18",
        r##"module c #(parameter P = 5) ();
  function logic [63:0] f(input logic [63:0] a);
    typedef enum logic [7:0] {P = 8'd3, PX} e_t;
    f = a + 64'(~(P + 32'd1));
  endfunction
  logic [63:0] r;
  initial begin r = f(64'd0); #1 $display("A %h %h %h", r, 64'(~(P + 32'd1)), 64'(-P)); end
endmodule
module top;
  c #(.P(-3)) u();
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A fffffffffffffffb ffffffff00000001 0000000000000003
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r2): `top` declares `localparam logic [3:0] L = 4'hA` ; its static function `f`'s body declares `typedef enum logic signed [7:0] {L = -8'sd3, LX}` and returns `a + L` ; `top` and generate block `g` print `{L, L}`, `$bits(L)` and `B = $bits(L)`. Kills mutant b04 (`restore_param_meta`'s Some arm dropped: `A 000000fe 0a0a 0000000a 8 16` / `G 8 0a0a`).
/// Owner line: the undo lane U2 (`bound/undo.rs` `restore_param_meta`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `G 4 aa` / `A 0000000b aa 0000000a 4 8` / `t.sv:13: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:5:11: Operator ADD expects 32 bits on the RHS, but RHS's ENUMITEMREF 'L' generates 8 bits.` / `%Warning-WIDTHEXPAND: t.sv:8:78: Operator ADD expects 32 bits on the LHS, but LHS's VARREF 'L' generates 4 bits.` ; verilator --binary run `A 000000fe aa 0000000a 4 8` / `G 4 aa` (as vita).
#[test]
fn n19_fn_label_meta_putback() {
    check(
        "n19",
        r##"module top;
  localparam logic [3:0] L = 4'hA;
  function logic [31:0] f(input logic [31:0] a);
    typedef enum logic signed [7:0] {L = -8'sd3, LX} e_t;
    f = a + L;
  endfunction
  logic [31:0] r;
  initial begin r = f(32'd1); #1 $display("A %h %h %h %0d %0d", r, {L, L}, L + 32'sd0, $bits(L), $bits({L, L})); end
  if (1) begin : g
    localparam int B = $bits(L);
    initial #1 $display("G %0d %h", B, {L, L});
  end
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 000000fe aa 0000000a 4 8
G 4 aa
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r2): `top` declares `localparam logic [15:8] P = 8'h5a`, an instance array `c u [1:0]` whose header declares `int P = 6`, then `c2 #(.Q(P[11:8]), .R(P[15:12])) w()`: `Q 10 R 5`. Kills mutant b15 (SIDE8 leaves `param_range`: W3056 x2 + E3009 x2, rc 1).
/// Owner line: the undo lane U3 (SIDE8, `bound/undo.rs` `side_restore`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `Q 10 R 5` / `t.sv:11: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0 (two WIDTHEXPAND warnings) ; verilator --binary run `Q 10 R 5` (all three agree).
#[test]
fn n20_prepass_range_putback() {
    check(
        "n20",
        r##"module c #(parameter int P = 6) (input a);
endmodule
module c2 #(parameter int Q = 0, parameter int R = 0) ();
  initial #1 $display("Q %0d R %0d", Q, R);
endmodule
module top;
  localparam logic [15:8] P = 8'h5a;
  logic a;
  c u [1:0] (.a(a));
  c2 #(.Q(P[11:8]), .R(P[15:12])) w();
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"Q 10 R 5
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// Batch 4 boundary (pins r2): `top`'s static function `f1` declares the enum label `L` (signed 8-bit) in its body ; then an instance array `c u [1:0]` whose header declares `parameter L = cf(2)` and the port `[$bits(L)-1:0] a`, connected to `logic [63:0] x`. PRE prints one E3009 (`parameter `L` value is not a constant: `cf(…)` has no constant-fold arm`, rc 1). Kills mutant b05 (`restore_param_meta`'s None arm dropped: a second E3009 `port `a` width 8 expects a connection of width 8 or 16, got 64`, errors=2). PRE is loud where verilator runs (`C 32`) and iverilog refuses (rc 2) ; the expected text is PRE's.
/// Owner line: the undo lane U2 (`bound/undo.rs` `restore_param_meta`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=2, `t.sv:8: error: Unable to bind wire/reg/memory `L' in `top.f1'` / `t.sv:8: error: Unable to elaborate statement in function f1.` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHEXPAND: t.sv:8:12: Operator ADD expects 32 bits on the RHS, but RHS's ENUMITEMREF 'L' generates 8 bits.` ; verilator --binary run `A 000000fe` / `C 32` / `C 32`.
#[test]
fn n21_prepass_meta_unbind() {
    check(
        "n21",
        r##"module c #(parameter L = cf(2)) (input [$bits(L)-1:0] a);
  function automatic int cf(input int k); return k + 3; endfunction
  initial #1 $display("C %0d", $bits(a));
endmodule
module top;
  function logic [31:0] f1(input logic [31:0] a);
    typedef enum logic signed [7:0] {L = -8'sd3, LX} e_t;
    f1 = a + L;
  endfunction
  logic [31:0] r1;
  initial begin r1 = f1(32'd1); #1 $display("A %h", r1); end
  logic [63:0] x;
  c u [1:0] (.a(x));
  initial #2 $finish;
endmodule
"##,
        &[],
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `L` value is not a constant: `cf(…)` has no constant-fold arm [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// Batch 4 boundary (pins r2): `top` declares `localparam logic [3:0] L = 4'hA` ; its static function `f`'s body declares the enum label `L` twice (`8'h81` on an 8-bit base, then `-16'sd2` on a signed 16-bit base) and returns `a + L`: `A 0000ffff aa 0000000a 10`.
/// Measured (W/batch4/pins/r3/var, PRE = POST): the module's `L = 4'h5` moves `{L, L}`, `L + 32'sd0` and `L` to `55 00000005 5` ; the second label `-16'sd3` moves `r` to `0000fffe` ; the first label `8'h7f` leaves PRE's line unchanged.
/// Kills mutants b03 (`restore_params` forward: `A 0000ffff 11 00000001 1` ; with the first label `8'h7f`, `A 0000ffff ff 0000000f 15`) and b06 (`restore_param_meta` forward: `A 0000ffff 0a0a 0000000a 10`) ; n19's mutant b04 fails it as well.
/// Invalid text vita accepts at rc 0: both oracles refuse the duplicate label (gap entry duplicate-value-name-accepted, its routine-body cells, W/batch4/pins/DOCS_FINDINGS.txt a).
/// Owner line: the undo lanes U1 / U2 (`bound/undo.rs` `restore_params`, `restore_param_meta`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=1, `t.sv:5: error: 'L' has already been declared in this scope.` / `t.sv:4:      : It was declared here as an enum type or value.` ; verilator 5.054 --lint-only -Wno-fatal: rc=1, `%Error: t.sv:5:39: Duplicate declaration of enum value: L`.
#[test]
fn n22_fn_dup_label_order() {
    check(
        "n22",
        r##"module top;
  localparam logic [3:0] L = 4'hA;
  function logic [31:0] f(input logic [31:0] a);
    typedef enum logic [7:0] {L = 8'h81, LX} e_t;
    typedef enum logic signed [15:0] {L = -16'sd2, LY} e2_t;
    f = a + L;
  endfunction
  logic [31:0] r;
  initial begin r = f(32'd1); #1 $display("A %h %h %h %0d", r, {L, L}, L + 32'sd0, L); end
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 0000ffff aa 0000000a 10
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

// ---- batch 4 boundary, pins r4: the killer of b10 (W/batch4/pins-judge3/REPORT.txt:12-13), measured on PRE
// 6bcd57fa, POST f17a0ff6 and the mutant's release binary (W/batch4/pins/r4/n23, W/batch4/pins/r4/u).

/// Batch 4 boundary (pins r4): package `p` declares `localparam S = "AB"` and `fs` with the return range `[((S == "AB") ? 7 : 3):0]` ; `top` declares `localparam S = "CD"` (no import) and prints `$bits(p::fs(1))`: `A 8`. Kills mutant b10 (`decl_keep_restore`: the kept `str_param_raw` entry not put back, bound/undo.rs:71-72): `A 4`.
/// Measured (PRE = POST): p's `S` changed to "CD" moves the output to `A 4` (W/batch4/pins-judge3/var4/RESULTS.txt q_s2_topCD_pCD), while top's `S` changed to "AB" leaves `A 8` (W/batch4/pins-judge3/var2/RESULTS.txt q_s2_topAB) ; with the condition `(S == "CD")` PRE prints `A 4` and b10 `A 8` (W/batch4/pins/r4/u/RESULTS.txt f_str_t).
/// Owner line: the undo lane U4 (`bound/undo.rs` `decl_keep_restore`) ; PRE 6bcd57fa and POST f17a0ff6 print the same.
/// Oracles: iverilog 13.0 -g2012: compile rc=0 ; vvp `A 8` / `t.sv:8: $finish called at 2 (1s)` ; verilator 5.054 --lint-only -Wno-fatal: rc=0, `%Warning-WIDTHTRUNC: t.sv:3:71: Operator ASSIGN expects 8 bits on the Assign RHS, but Assign RHS's VARREF 'a' generates 32 bits.` ; verilator --binary run `A 8`.
#[test]
fn n23_pkg_str_window_kept() {
    check(
        "n23",
        r##"package p;
  localparam S = "AB";
  function automatic logic [((S == "AB") ? 7 : 3):0] fs(input int a); return a; endfunction
endpackage
module top;
  localparam S = "CD";
  initial #1 $display("A %0d", $bits(p::fs(1)));
  initial #2 $finish;
endmodule
"##,
        &[],
        r##"A 8
simulation ended (Finish) at time 2
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}
