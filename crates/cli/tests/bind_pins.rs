//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the BIND-1a lane cells
//! that show the write contract of the nine FQ parameter maps (ROADMAP §5.2 BIND-1a: `bound/`'s
//! DECL-ORDER `bind_decl`, the value funnel's clears, and the undo lanes U3, U5, U6, U8 of
//! `bound/undo.rs`). Every expected text was measured on the PRE binary (6bcd57fa, `vita-post` md5
//! 09768204 ; W/bind1/build_a/gen) with `t.sv` as the argument (`g08_dash_g_overrides`: the `-G`
//! arguments first). A named mutant killer here is a claim until its mutant runs.
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
