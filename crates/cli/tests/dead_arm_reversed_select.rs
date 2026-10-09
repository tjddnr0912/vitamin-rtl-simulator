//! §5.2 row 17 (C-R02): a reversed constant part-select (`tid[W-1:W-CL]` with `CL = $clog2(1) = 0`)
//! in the arm of a procedural `if` whose constant condition never selects that arm runs instead of
//! E3009, as its generate twin does; a reversed select the run can reach stays E3009 (IEEE 1800
//! §11.5.1: the first expression shall address a more significant bit than the second). Each cell is
//! the grounding cell, byte for byte, with the oracles' answer beside it: iverilog 13.0, sv2v 0.0.13 ->
//! iverilog 13.0 and verilator 5.052 (`--binary`). PRE is vita at 73c918a6 (md5 a4bbc405), which
//! refuses every cell here except the four in-order controls. The `undecided_condition_stays_refused`
//! cells are row 17 round 1's review cells (both lenses).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// stdout without the `simulation ended` line, success, stderr
fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_dar_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.sv");
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&dir);
    let so = String::from_utf8_lossy(&out.stdout);
    let s: Vec<&str> = so
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .collect();
    (
        s.join("\n"),
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

enum Want {
    /// Runs and prints exactly this.
    Prints(&'static str),
    /// Refused, an error line holding this text.
    Refused(&'static str),
    /// Refused with exactly one error, holding this text.
    RefusedOnce(&'static str),
    /// Refused with exactly two errors, holding these texts in this order.
    RefusedTwice(&'static str, &'static str),
}

fn check(cells: &[(&str, &str, Want)]) {
    let mut bad = Vec::new();
    for (name, src, want) in cells {
        let (out, ok, err) = run_raw(src);
        let errors: Vec<&str> = err.lines().filter(|l| l.contains("error[VITA-E")).collect();
        let pass = match want {
            Want::Prints(w) => ok && out.trim_end() == *w && errors.is_empty(),
            Want::Refused(n) => !ok && errors.iter().any(|l| l.contains(n)),
            Want::RefusedOnce(n) => !ok && errors.len() == 1 && errors[0].contains(n),
            Want::RefusedTwice(a, b) => {
                !ok && errors.len() == 2 && errors[0].contains(a) && errors[1].contains(b)
            }
        };
        if !pass {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// A reversed `[m:l]` select in the arm a constant condition never selects: the design runs, and prints what the three oracles print (PRE refused each with E3009).
#[test]
fn dead_arm_runs() {
    check(&[
        // g01_dead_then_w: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g01_dead_then_w",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) begin
      tid[W-1:W-CL] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g02_dead_else_w: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g02_dead_else_w",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N == 1) tid[0] = g[0];
    else tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // g03_dead_then_r: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g03_dead_then_r",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid = 8'h5a;
  reg [7:0] x;
  reg g = 0;
  always @(g) begin
    x = 8'h33;
    if (N > 1) x = tid[W-1:W-CL];
  end
  initial begin #1 g = 1; #1 $display("A x=%h", x); #10 $finish; end
endmodule
"#,
            Want::Prints("A x=33"),
        ),
        // g16_dead_asc: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (a descending select on an ascending net)
        (
            "g16_dead_asc",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [0:W-1] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) tid[W-CL:W-1] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g06_dead_display: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (the dead arm's $display never prints)
        (
            "g06_dead_display",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid = 8'h5a;
  reg g = 0;
  always @(g) begin
    if (N > 1) begin
      $display("A dead=%h", tid[W-1:W-CL]);
      tid[W-1:W-CL] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // c26_dead_display_reversed_arg: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c26_dead_display_reversed_arg",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid = 8'h5a;
  initial begin
    if (N > 1) $display("A dead=%h", tid[7:8]);
    else $display("A live=%h", tid[7:6]);
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A live=1"),
        ),
        // g10_dead_nested_case: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (pcie_us_axi_dma_wr's shape)
        (
            "g10_dead_nested_case",
            r#"module top;
  parameter DW = 64;
  reg [DW-1:0] d;
  reg [255:0] s = {8{32'hdeadbeef}};
  reg [1:0] st = 0;
  always @* begin
    d = {DW{1'b0}};
    case (st)
      2'd0: d[7:0] = 8'h11;
      2'd1: begin
        if (DW >= 256) begin
          d[DW-1:128] = s[DW-1:128];
        end else begin
          d[7:0] = 8'h22;
        end
      end
      default: d = {DW{1'b1}};
    endcase
  end
  initial begin #1 st = 1; #1 $display("A d=%h", d); #1 st = 2; #1 $display("A d=%h", d); #10 $finish; end
endmodule
"#,
            Want::Prints("A d=0000000000000022\nA d=ffffffffffffffff"),
        ),
        // g23_else_if_chain: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g23_else_if_chain",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N == 0) tid = 0;
    else if (N == 1) tid[1] = g[0];
    else tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // c25_dead_nested_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c25_dead_nested_dead",
            r#"module top;
  parameter N = 1;
  parameter M = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) begin
      tid[7:8] = g;
      if (M > 1) tid[6:7] = g; else tid[5:6] = g;
    end else begin
      if (M == 1) tid[0] = g; else tid[3:4] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // g19_dead_initial: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g19_dead_initial",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  initial begin
    tid = 8'h5a;
    if (N > 1) tid[W-1:W-CL] = 1'b1;
    $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g25_dead_nba: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g25_dead_nba",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid = 8'h5a;
  reg clk = 0;
  reg [CL:0] g = 1;
  always @(posedge clk) begin
    tid[0] <= ~tid[0];
    if (N > 1) tid[W-1:W-CL] <= g;
  end
  initial begin #1 clk = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c23_always_comb_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c23_always_comb_dead",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  logic [7:0] tid;
  logic [7:0] g = 0;
  always_comb begin
    tid = 8'h5a ^ (g & 8'h0);
    if (N > 1) tid[7:8-CL] = g[0];
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // c24_always_ff_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c24_always_ff_dead",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  logic [7:0] tid = 8'h5a;
  logic clk = 0;
  always_ff @(posedge clk) begin
    tid[0] <= ~tid[0];
    if (N > 1) tid[7:8-CL] <= 1'b1;
  end
  initial begin #1 clk = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // g34_dead_concat_lval: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g34_dead_concat_lval",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [3:0] x;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a; x = 4'h3;
    if (N > 1) {tid[W-1:W-CL], x} = {g, 4'hf};
  end
  initial begin #1 g = 1; #1 $display("A tid=%h x=%h", tid, x); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a x=3"),
        ),
        // g37_dead_in_for: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g37_dead_in_for",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid [0:3];
  reg [CL:0] g = 0;
  integer i;
  always @(g) begin
    for (i = 0; i < 4; i = i + 1) begin
      tid[i] = 8'h5a + i;
      if (N > 1) tid[i][W-1:W-CL] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A t0=%h t3=%h", tid[0], tid[3]); #10 $finish; end
endmodule
"#,
            Want::Prints("A t0=5a t3=5d"),
        ),
        // c27_dead_while: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c27_dead_while",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid = 8'h5a;
  integer k;
  initial begin
    k = 0;
    while (k < 3) begin
      if (N > 1) tid[7:8] = 0;
      k = k + 1;
    end
    $display("A tid=%h k=%0d", tid, k);
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A tid=5a k=3"),
        ),
        // g41_dead_fork: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g41_dead_fork",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [7:0] tid;
  initial begin
    tid = 8'h5a;
    if (N > 1) fork
      tid[7:8-CL] = 2'b11;
      #1 tid[0] = 0;
    join
    #2 $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g40_dead_task_call_outarg: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g40_dead_task_call_outarg",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [7:0] tid;
  task t(output [1:0] o); o = 2'b11; endtask
  initial begin
    tid = 8'h5a;
    if (N > 1) t(tid[7:8-CL]);
    #1 $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g27_dead_unique_if: sv2v 0.0.13 -> iverilog and verilator 5.052 (iverilog 13.0: syntax error)
        (
            "g27_dead_unique_if",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    unique if (N > 1) tid[W-1:W-CL] = g;
    else tid[0] = g[0];
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
    ]);
}

/// The arm is decided where the select is lowered: per instance, per generate iteration, inside a subroutine body, an interface, a package and a class.
#[test]
fn dead_arm_runs_per_scope() {
    check(&[
        // e01_multi_inst: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "e01_multi_inst",
            r#"module m #(parameter N = 1) (input [7:0] g, output reg [7:0] tid);
  localparam CL = $clog2(N);
  localparam W = 8;
  always @* begin
    tid = 8'h5a ^ (g & 8'h00);
    if (N > 1) tid[W-1:W-CL] = g;
  end
endmodule
module top;
  reg [7:0] g = 0;
  wire [7:0] t1, t2, t4;
  m #(.N(1)) u1 (.g(g), .tid(t1));
  m #(.N(2)) u2 (.g(g), .tid(t2));
  m #(.N(4)) u4 (.g(g), .tid(t4));
  initial begin #1 g = 8'hff; #1 $display("A t1=%h t2=%h t4=%h", t1, t2, t4); #10 $finish; end
endmodule
"#,
            Want::Prints("A t1=5a t2=da t4=da"),
        ),
        // e02_axis_switch_shape: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (axis_switch at S_COUNT=1 and 2)
        (
            "e02_axis_switch_shape",
            r#"module sw #(parameter S_COUNT = 1, parameter M_COUNT = 2, parameter UPDATE_TID = 1,
            parameter S_ID_WIDTH = 8)
  (input [S_COUNT*S_ID_WIDTH-1:0] s_tid, input [1:0] gsel, output [M_COUNT*16-1:0] m_tid);
  localparam CL_S_COUNT = $clog2(S_COUNT);
  localparam M_ID_WIDTH = S_ID_WIDTH + CL_S_COUNT;
  genvar n;
  generate for (n = 0; n < M_COUNT; n = n + 1) begin : m_ifaces
    reg [M_ID_WIDTH-1:0] tid_mux;
    wire [CL_S_COUNT > 0 ? CL_S_COUNT-1 : 0:0] grant_encoded = gsel;
    always @* begin
      tid_mux = s_tid[0 +: S_ID_WIDTH];
      if (UPDATE_TID && S_COUNT > 1) begin
        tid_mux[M_ID_WIDTH-1:M_ID_WIDTH-CL_S_COUNT] = grant_encoded;
      end
    end
    assign m_tid[n*16 +: 16] = tid_mux;
  end endgenerate
endmodule
module top;
  reg [7:0] s = 0;
  reg [15:0] s2 = 0;
  reg [1:0] gs = 0;
  wire [31:0] m1, m2;
  sw #(.S_COUNT(1)) u1 (.s_tid(s), .gsel(gs), .m_tid(m1));
  sw #(.S_COUNT(2)) u2 (.s_tid(s2), .gsel(gs), .m_tid(m2));
  initial begin #1 s = 8'h3c; s2 = 16'h3c5a; gs = 2'b11; #1 $display("A m1=%h m2=%h", m1, m2); #10 $finish; end
endmodule
"#,
            Want::Prints("A m1=003c003c m2=015a015a"),
        ),
        // g21_genvar_cond: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g21_genvar_cond",
            r#"module top;
  reg [7:0] tid [0:1];
  reg g = 0;
  for (genvar i = 0; i < 2; i = i + 1) begin : gg
    always @(g) begin
      tid[i] = 8'h5a;
      if (i > 0) tid[i][7:8-i] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A t0=%h t1=%h", tid[0], tid[1]); #10 $finish; end
endmodule
"#,
            Want::Prints("A t0=5a t1=da"),
        ),
        // c19_gen_local_shadow: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (the generate block's own localparam decides)
        (
            "c19_gen_local_shadow",
            r#"module top;
  localparam X = 1;
  reg [7:0] tid;
  reg g = 0;
  if (1) begin : gb
    localparam X = 2;
    always @(g) begin
      tid = 8'h5a;
      if (X > 1) tid[1] = g;
      else tid[7:8] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // c20_gen_local_shadow_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c20_gen_local_shadow_dead",
            r#"module top;
  localparam X = 2;
  reg [7:0] tid;
  reg g = 0;
  if (1) begin : gb
    localparam X = 1;
    always @(g) begin
      tid = 8'h5a;
      if (X > 1) tid[7:8] = g;
      else tid[1] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // c21_defparam: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c21_defparam",
            r#"module m;
  parameter N = 2;
  localparam CL = $clog2(N);
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) tid[7:8-CL] = g;
  end
endmodule
module top;
  m u();
  defparam u.N = 1;
  initial begin #1 u.g = 1; #1 $display("A tid=%h", u.tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g11_dead_task: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g11_dead_task",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  task t(input [CL:0] v);
    begin
      tid = 8'h5a;
      if (N > 1) tid[W-1:W-CL] = v;
    end
  endtask
  initial begin t(1); #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g12_dead_func: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g12_dead_func",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  function [W-1:0] f(input [W-1:0] a, input [CL:0] v);
    begin
      f = a;
      if (N > 1) f[W-1:W-CL] = v;
    end
  endfunction
  always @(g) tid = f(8'h5a, g);
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // s04_task_ref_param: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "s04_task_ref_param",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid;
  task t;
    begin
      tid = 8'h5a;
      if (N > 1) tid[7:8] = 1'b1;
    end
  endtask
  initial begin t; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g45_auto_func_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g45_auto_func_dead",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [7:0] tid;
  reg g = 0;
  function automatic [7:0] f(input [7:0] a, input v);
    reg [7:0] r;
    begin
      r = a;
      if (N > 1) r[7:8-CL] = v;
      f = r;
    end
  endfunction
  always @(g) tid = f(8'h5a, g);
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // k02_const_fn_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (a constant function's body)
        (
            "k02_const_fn_dead",
            r#"module top;
  parameter N = 1;
  function integer cf(input integer x);
    reg [7:0] r;
    begin
      r = 8'h5a;
      if (N > 1) r[7:8] = 1'b1;
      cf = r + x;
    end
  endfunction
  localparam L = cf(1);
  initial begin $display("A L=%0d", L); #10 $finish; end
endmodule
"#,
            Want::Prints("A L=91"),
        ),
        // g42_dead_iface_proc: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g42_dead_iface_proc",
            r#"interface ifc #(parameter N = 1);
  localparam CL = $clog2(N);
  logic [7:0] tid;
  logic g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) tid[7:8-CL] = g;
  end
endinterface
module top;
  ifc #(.N(1)) i0();
  initial begin #1 i0.g = 1; #1 $display("A tid=%h", i0.tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // g43_dead_pkg_func: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g43_dead_pkg_func",
            r#"package p;
  parameter N = 1;
  localparam CL = $clog2(N);
  function automatic logic [7:0] f(input logic [7:0] a, input logic v);
    logic [7:0] r;
    r = a;
    if (N > 1) r[7:8-CL] = v;
    return r;
  endfunction
endpackage
module top;
  reg [7:0] tid;
  reg g = 0;
  always @(g) tid = p::f(8'h5a, g);
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // k01_class_param_method: verilator 5.052 (iverilog 13.0 and sv2v 0.0.13 do not parse a parameterized class)
        (
            "k01_class_param_method",
            r#"class C #(parameter N = 1);
  function logic [7:0] f(input logic [7:0] a, input logic v);
    logic [7:0] r;
    r = a;
    if (N > 1) r[7:8] = v;
    else r[0] = v;
    return r;
  endfunction
endclass
module top;
  reg [7:0] tid;
  C #(1) c;
  initial begin
    c = new;
    tid = c.f(8'h5a, 1'b1);
    $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
    ]);
}

/// The condition decides only when both readings fold it: known constants under `!`, `&&`, `||`, relational and equality operators, compared signed only when both operands are signed (IEEE 1800 §11.8.2).
#[test]
fn condition_folds() {
    check(&[
        // c01_signed_gt0: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`PS > 0`, PS = 4'sb1111: signed, false)
        (
            "c01_signed_gt0",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (PS > 0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c05_mixed_lit: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`4'sb1111 < 8'd0`: unsigned, false)
        (
            "c05_mixed_lit",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (4'sb1111 < 8'd0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c07_not: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c07_not",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (!(N == 1)) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c08_or: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c08_or",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1 || 1'b0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c09_caseeq: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c09_caseeq",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N === 2) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c10_ne: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c10_ne",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N != 1) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c11_ge: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c11_ge",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N >= 2) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c12_le: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c12_le",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N <= 0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c28_le_boundary: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`N <= 1` with N = 1)
        (
            "c28_le_boundary",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N <= 1) tid[0] = g;
    else tid[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c29_ge_boundary: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`N >= 1`)
        (
            "c29_ge_boundary",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N >= 1) tid[0] = g;
    else tid[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c30_lt_boundary: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`N < 1`)
        (
            "c30_lt_boundary",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N < 1) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c31_gt_boundary: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`N > 1`)
        (
            "c31_gt_boundary",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c16_and_true_false: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c16_and_true_false",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (1 && N > 1) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // c17_land_param: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "c17_land_param",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N && 0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // g30_bits_cond: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052
        (
            "g30_bits_cond",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(g) > 1) tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5a"),
        ),
        // x01_param_trunc_else_dead: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`parameter [3:0] P = -1` is 15: `P > 0` is true)
        (
            "x01_param_trunc_else_dead",
            r#"module top;
  parameter [3:0] P = -1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (P > 0) tid[0] = g;
    else tid[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // x03_param_wide_value: iverilog 13.0, sv2v 0.0.13 -> iverilog and verilator 5.052 (`parameter [3:0] P = 5'h11` is 1)
        (
            "x03_param_wide_value",
            r#"module top;
  parameter [3:0] P = 5'h11;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (P == 1) tid[0] = g;
    else tid[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=5b"),
        ),
        // x02_param_trunc_then_live: iverilog and sv2v -> iverilog refuse (`P > 0` is true: the arm runs); verilator `tid=59`
        (
            "x02_param_trunc_then_live",
            r#"module top;
  parameter [3:0] P = -1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (P > 0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // c02_signed_lt0: iverilog and sv2v -> iverilog refuse (`PS < 0` is true: the arm runs); verilator `tid=59`
        (
            "c02_signed_lt0",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (PS < 0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // c14_wide_neg: iverilog and sv2v -> iverilog refuse (`PS == 4'b1111` is unsigned, true); verilator `tid=59`
        (
            "c14_wide_neg",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (PS == 4'b1111) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // c06_signed_lits: iverilog and sv2v -> iverilog refuse (`4'sb1111 < 8'sd0` is signed, true); verilator `tid=59`
        (
            "c06_signed_lits",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (4'sb1111 < 8'sd0) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
    ]);
}

/// A reversed select the run can reach stays E3009, as in Icarus Verilog.
#[test]
fn reachable_stays_refused() {
    check(&[
        // g04_reach_w: iverilog and sv2v -> iverilog: `part select tid[7:8] is reversed`; verilator `tid=59`
        (
            "g04_reach_w",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g05_reach_if_var: iverilog and sv2v -> iverilog refuse; verilator `tid=59`
        (
            "g05_reach_if_var",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (g) tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g28_live_true_then: iverilog and sv2v -> iverilog refuse; verilator `tid=59`
        (
            "g28_live_true_then",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N == 1) tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g13_dead_ternary: iverilog and sv2v -> iverilog: `part select tid[7:8] is out of order`; verilator `x=33`
        (
            "g13_dead_ternary",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid = 8'h5a;
  reg [7:0] x;
  reg g = 0;
  always @(g) x = (N > 1) ? tid[W-1:W-CL] : 8'h33;
  initial begin #1 g = 1; #1 $display("A x=%h", x); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g18_case_const: iverilog and sv2v -> iverilog refuse; verilator `tid=5b`
        (
            "g18_case_const",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    case (N)
      1: tid[0] = g[0];
      default: tid[W-1:W-CL] = g;
    endcase
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g14_shadow_blocklocal: iverilog and sv2v -> iverilog refuse (the block-local `N` is 2); verilator `tid=59`
        (
            "g14_shadow_blocklocal",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin : b
    integer N;
    N = 2;
    tid = 8'h5a;
    if (N > 1) tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g31c_func_formal_reversed: iverilog and sv2v -> iverilog refuse (the formal `N` is 2); verilator `tid=59`
        (
            "g31c_func_formal_reversed",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg g = 0;
  function [W-1:0] f(input integer N, input v);
    begin
      f = 8'h5a;
      if (N > 1) f[W-1:W-CL] = v;
    end
  endfunction
  always @(g) tid = f(2, g);
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g35_task_formal_reversed: iverilog and sv2v -> iverilog refuse; verilator `tid=59`
        (
            "g35_task_formal_reversed",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  task t(input integer N, input v);
    begin
      tid = 8'h5a;
      if (N > 1) tid[W-1:W-CL] = v;
    end
  endtask
  initial begin t(2, 1'b1); #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s01_task_formal_const_actual: iverilog and sv2v -> iverilog refuse (the formal `K` is a variable, though every call passes 1); verilator `tid=5a`
        (
            "s01_task_formal_const_actual",
            r#"module top;
  reg [7:0] tid;
  task t(input integer K);
    begin
      tid = 8'h5a;
      if (K > 1) tid[7:8] = 1'b1;
    end
  endtask
  initial begin t(1); #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s02_task_formal_const_actual_else: iverilog and sv2v -> iverilog refuse; verilator `tid=5b`
        (
            "s02_task_formal_const_actual_else",
            r#"module top;
  reg [7:0] tid;
  task t(input integer K);
    begin
      tid = 8'h5a;
      if (K == 1) tid[0] = 1'b1;
      else tid[7:8] = 1'b1;
    end
  endtask
  initial begin t(1); #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // k03_class_shadow_param: verilator `tid=59` (the class's `N` is 2: the arm runs)
        (
            "k03_class_shadow_param",
            r#"class C #(parameter N = 2);
  function logic [7:0] f(input logic [7:0] a, input logic v);
    logic [7:0] r;
    r = a;
    if (N > 1) r[7:8] = v;
    else r[0] = v;
    return r;
  endfunction
endclass
module top;
  parameter N = 1;
  reg [7:0] tid;
  C #(2) c;
  initial begin
    c = new;
    tid = c.f(8'h5a, 1'b1);
    $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g33_dead_plus_reach: iverilog and sv2v -> iverilog refuse the reachable `u[7:8]` only; verilator `tid=5a u=11`
        (
            "g33_dead_plus_reach",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid, u;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) tid[W-1:W-CL] = g;
    u = 8'h11;
    u[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h u=%h", tid, u); #10 $finish; end
endmodule
"#,
            Want::RefusedOnce("t.sv:11:5"),
        ),
    ]);
}

/// Everything else in a dead arm is checked as before, and a select the arm does not hold is refused.
#[test]
fn other_refusals_stay() {
    check(&[
        // g07_dead_zero_indexed: verilator: `Width of bit extract must be positive`; iverilog and sv2v -> iverilog `tid=5a` (row 65 keeps it refused, IEEE 1800 §11.5.1)
        (
            "g07_dead_zero_indexed",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [7:0] tid;
  reg [7:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) begin
      tid[0 +: CL] = g;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("indexed part-select is zero"),
        ),
        // g08_dead_undeclared: verilator: `Can't find definition of variable`; iverilog and sv2v -> iverilog `tid=5a`
        (
            "g08_dead_undeclared",
            r#"module top;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1) begin
      tid = nosuch_name;
    end
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("E3010"),
        ),
        // g17b_dead_mdpacked: all three `p=5511223344`: another refusal site, not opened
        (
            "g17b_dead_mdpacked",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [4:0][7:0] p;
  reg g = 0;
  always @(g) begin
    p = 40'h5511223344;
    if (N > 1) p[3:4-CL] = 8'hff;
  end
  initial begin #1 g = 1; #1 $display("A p=%h", p); #10 $finish; end
endmodule
"#,
            Want::Refused("multi-dim packed array is out of order"),
        ),
        // g15d_two_inst_live_read: all three `t1=5a t2=da t4=da`: a reversed select by a negative bound (`g[CL-1:0]`) is the edge decision's refusal, not opened
        (
            "g15d_two_inst_live_read",
            r#"module m #(parameter N = 1) (input [7:0] g, output reg [7:0] tid);
  localparam CL = $clog2(N);
  localparam W = 8;
  always @* begin
    tid = 8'h5a ^ (g & 8'h00);
    if (N > 1) tid[W-1:W-CL] = g[CL-1:0];
  end
endmodule
module top;
  reg [7:0] g = 0;
  wire [7:0] t1, t2, t4;
  m #(.N(1)) u1 (.g(g), .tid(t1));
  m #(.N(2)) u2 (.g(g), .tid(t2));
  m #(.N(4)) u4 (.g(g), .tid(t4));
  initial begin #1 g = 8'hff; #1 $display("A t1=%h t2=%h t4=%h", t1, t2, t4); #10 $finish; end
endmodule
"#,
            Want::Refused("the width of this part-select is negative"),
        ),
        // g38_dead_static_blocklocal_init: all three `r=11`: a static initializer is not held by the arm
        (
            "g38_dead_static_blocklocal_init",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [7:0] tid = 8'h5a;
  reg [7:0] r;
  initial begin
    r = 8'h11;
    if (N > 1) begin : b
      reg [1:0] s = tid[7:8-CL];
      r = s;
    end
    #1 $display("A r=%h", r);
    #10 $finish;
  end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g39_dead_hier_part: all three `tid=5a`: a hierarchical select is lowered after the arm
        (
            "g39_dead_hier_part",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [CL:0] g = 0;
  sub u();
  always @(g) begin
    u.tid = 8'h5a;
    if (N > 1) u.tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", u.tid); #10 $finish; end
endmodule
module sub;
  reg [7:0] tid;
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g36_dead_wait_event: all three `tid=5a`: an event expression is not held by the arm's statements
        (
            "g36_dead_wait_event",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  reg [7:0] tid = 8'h5a;
  reg g = 0;
  initial begin
    if (N > 1) begin
      @(tid[7:8-CL]);
      tid = 0;
    end
    #1 $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g09_dead_and_var: all three `tid=5a`: `N > 1 && g` reads a variable
        (
            "g09_dead_and_var",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1 && g) tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g30b_clog2_cond: all three `tid=5a`: the lowered `$clog2` is not folded
        (
            "g30b_clog2_cond",
            r#"module top;
  parameter N = 1;
  localparam CL = $clog2(N);
  localparam W = 8;
  reg [W-1:0] tid;
  reg [CL:0] g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($clog2(N) > 0) tid[W-1:W-CL] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // c13_bare_param: all three `tid=5b`: arithmetic is not folded
        (
            "c13_bare_param",
            r#"module top;
  parameter signed [3:0] PS = -1;
  parameter [3:0] PU = 4'hf;
  parameter N = 1;
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N - 1) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // g20b_xcond_lit: all three `tid=5a`: an x condition is not folded
        (
            "g20b_xcond_lit",
            r#"module top;
  reg [7:0] tid;
  initial begin
    tid = 8'h5a;
    if (1'bx) tid[7:8] = 1'b1;
    $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
    ]);
}

/// A condition over a placeholder is not folded: `$bits(<instance>.<name>)` lowers to a 32 the deferred pass rewrites after the arm is lowered (row 17 round 1, F1), and a real or wider-than-64-bit leaf has no fold. The arm the run takes keeps its refusal, as on PRE.
#[test]
fn undecided_condition_stays_refused() {
    check(&[
        // h01_bits_hier_else: iverilog: `part select tid[7:8] is reversed`; verilator `tid=59`
        (
            "h01_bits_hier_else",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.X) > 8) tid[0] = g;
    else tid[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h02_bits_hier_then: iverilog: `part select tid[7:8] is reversed`; verilator `tid=59 y=aa`
        (
            "h02_bits_hier_then",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg [7:0] y = 8'h00;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.X) < 16) begin
      y = 8'hAA;
      tid[7:8] = g;
    end
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h y=%h", tid, y); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h04_bits_hier_inrange_write: iverilog: `part select tid[3:4] is reversed`; verilator `tid=7a`
        (
            "h04_bits_hier_inrange_write",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.X) == 4) tid[3:4] = {g, g};
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h05_bits_hier_inrange_read: iverilog: `part select tid[3:4] is out of order`; verilator `r=01`
        (
            "h05_bits_hier_inrange_read",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid = 8'h5a;
  reg [1:0] r;
  reg g = 0;
  always @(g) begin
    r = 2'b00;
    if ($bits(u.X) != 32) r = tid[3:4];
  end
  initial begin #1 g = 1; #1 $display("A r=%b", r); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h06_bits_hier_land: iverilog: `part select tid[3:4] is reversed`; verilator `tid=7a`
        (
            "h06_bits_hier_land",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  parameter N = 2;
  sub u();
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if (N > 1 && $bits(u.X) < 8) tid[3:4] = {g, g};
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h10_bits_hier_param: iverilog: `part select tid[3:4] is reversed`; verilator `tid=7a`
        (
            "h10_bits_hier_param",
            r#"module sub;
  parameter [3:0] P = 4'd5;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.P) == 4) tid[3:4] = {g, g};
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h11_bits_hier_gen_inst: iverilog: `part select tid[3:4] is reversed`; verilator `tid=7a`
        (
            "h11_bits_hier_gen_inst",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  for (genvar i = 0; i < 2; i++) begin : gen
    sub u();
  end
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(gen[0].u.X) < 8) tid[3:4] = {g, g};
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h18_bits_hier_upward: iverilog: `part select tid[3:4] is reversed`; verilator `t=7a`
        (
            "h18_bits_hier_upward",
            r#"module child (input g, output reg [7:0] tid);
  always @(g) begin
    tid = 8'h5a;
    if ($bits(top.X) == 4) tid[3:4] = {g, g};
    else tid[0] = g;
  end
endmodule
module top;
  logic [3:0] X;
  reg g = 0;
  wire [7:0] t;
  child c (.g(g), .tid(t));
  initial begin #1 g = 1; #1 $display("A t=%h", t); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s01_bits_hier_then: iverilog: `part select t[7:8] is reversed`; verilator `t=59 b=8`
        (
            "s01_bits_hier_then",
            r#"module sub;
  reg [7:0] tid;
endmodule
module top;
  sub u();
  reg [7:0] t;
  reg g = 0;
  always @(g) begin
    t = 8'h5a;
    if ($bits(u.tid) != 32) t[7:8] = g;
    else t[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A t=%h b=%0d", t, $bits(u.tid)); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s03_bits_hier_else: iverilog: `part select t[7:8] is reversed`; verilator `t=59 b=8`
        (
            "s03_bits_hier_else",
            r#"module sub;
  reg [7:0] tid;
endmodule
module top;
  sub u();
  reg [7:0] t;
  reg g = 0;
  always @(g) begin
    t = 8'h5a;
    if ($bits(u.tid) > 16) t[0] = g;
    else t[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A t=%h b=%0d", t, $bits(u.tid)); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s04_bits_hier_display: iverilog: `part select t[7:8] is out of order`; verilator `ran-dead=1`
        (
            "s04_bits_hier_display",
            r#"module sub;
  reg [7:0] tid = 8'h5a;
endmodule
module top;
  sub u();
  reg [7:0] t = 8'ha5;
  initial begin
    #1;
    if ($bits(u.tid) == 32) $display("A else-dead-fold live=%h", t[7:6]);
    else $display("A ran-dead=%h", t[7:8]);
    #10 $finish;
  end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s05_bits_hier_param: iverilog: `part select t[7:8] is out of order`; verilator `ran=1`
        (
            "s05_bits_hier_param",
            r#"module sub #(parameter [7:0] P = 8'h3);
endmodule
module top;
  sub u();
  reg [7:0] t = 8'ha5;
  initial begin
    #1;
    if ($bits(u.P) == 32) $display("A live=%h", t[7:6]);
    else $display("A ran=%h", t[7:8]);
    #10 $finish;
  end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s06_bits_hier_in_func: iverilog: `part select f[7:8] is reversed`; verilator `t=5b`
        (
            "s06_bits_hier_in_func",
            r#"module sub;
  reg [15:0] tid;
endmodule
module top;
  sub u();
  reg [7:0] t;
  reg g = 0;
  function [7:0] f(input [7:0] a, input v);
    begin
      f = a;
      if ($bits(u.tid) < 32) f[7:8] = {v, v};
    end
  endfunction
  always @(g) t = f(8'h5a, g);
  initial begin #1 g = 1; #1 $display("A t=%h", t); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s07_bits_hier_and: iverilog: `part select t[7:8] is reversed`; verilator `t=59`
        (
            "s07_bits_hier_and",
            r#"module sub;
  reg [7:0] tid;
endmodule
module top;
  parameter N = 2;
  sub u();
  reg [7:0] t;
  reg g = 0;
  always @(g) begin
    t = 8'h5a;
    if (N > 1 && !($bits(u.tid) == 8)) t[0] = g;
    else t[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A t=%h", t); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // s10_bits_hier_genblk: iverilog: `part select t[7:8] is out of order`; verilator `ran=1`
        (
            "s10_bits_hier_genblk",
            r#"module sub;
  reg [7:0] tid;
endmodule
module top;
  sub u();
  reg [7:0] t = 8'ha5;
  if (1) begin : gb
    initial begin
      #1;
      if ($bits(u.tid) == 32) $display("A live=%h", t[7:6]);
      else $display("A ran=%h", t[7:8]);
    end
  end
  initial #10 $finish;
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h03_bits_hier_truly_dead: iverilog and verilator `tid=5b`: dead by the real width, but the condition is undecided while the arm is lowered, so it stays refused
        (
            "h03_bits_hier_truly_dead",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.X) > 8) tid[7:8] = g;
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // w01b_real_param_then_rev: iverilog: `part select t[7:8] is reversed` (`R < 1` with `parameter real R = 0.5` is true); verilator `t=59`
        (
            "w01b_real_param_then_rev",
            r#"module top;
  parameter real R = 0.5;
  reg [7:0] t;
  reg g = 0;
  always @(g) begin
    t = 8'h5a;
    if (R < 1) t[7:8] = g;
    else t[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A t=%h", t); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // x10_err_placeholder_cond: PRE prints both errors: the refused `h + 1` leaves a placeholder constant, which is not folded
        (
            "x10_err_placeholder_cond",
            r#"class C;
endclass
module top;
  C h;
  reg [7:0] tid;
  initial begin
    tid = 8'h5a;
    if (h + 1 > 2) tid[7:8] = 1'b1;
    $display("A tid=%h", tid);
    #10 $finish;
  end
endmodule
"#,
            Want::RefusedTwice(
                "a class handle / `null` is only a legal operand",
                "ascend but the net is descending",
            ),
        ),
        // w02b_wide_param_else_rev: iverilog: `part select t[7:8] is reversed` (an 80-bit `P == 0` is false); verilator `t=59`
        (
            "w02b_wide_param_else_rev",
            r#"module top;
  parameter [79:0] P = 80'h1_0000_0000_0000_0000;
  reg [7:0] t;
  reg g = 0;
  always @(g) begin
    t = 8'h5a;
    if (P == 0) t[0] = g;
    else t[7:8] = g;
  end
  initial begin #1 g = 1; #1 $display("A t=%h", t); #10 $finish; end
endmodule
"#,
            Want::Refused("ascend but the net is descending"),
        ),
        // h01c_bits_hier_else_inorder: iverilog 13.0 and verilator 5.052 (control: the run takes the arm a placeholder fold would call dead)
        (
            "h01c_bits_hier_else_inorder",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.X) > 8) tid[0] = g;
    else tid[7:6] = {g, g};
  end
  initial begin #1 g = 1; #1 $display("A tid=%h", tid); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=da"),
        ),
        // h02c_bits_hier_then_inorder: iverilog 13.0 and verilator 5.052 (control)
        (
            "h02c_bits_hier_then_inorder",
            r#"module sub;
  logic [3:0] X;
endmodule
module top;
  sub u();
  reg [7:0] tid;
  reg [7:0] y = 8'h00;
  reg g = 0;
  always @(g) begin
    tid = 8'h5a;
    if ($bits(u.X) < 16) begin
      y = 8'hAA;
      tid[7:6] = {g, g};
    end
    else tid[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A tid=%h y=%h", tid, y); #10 $finish; end
endmodule
"#,
            Want::Prints("A tid=da y=aa"),
        ),
        // s02_bits_hier_ctrl: iverilog 13.0 and verilator 5.052 (control)
        (
            "s02_bits_hier_ctrl",
            r#"module sub;
  reg [7:0] tid;
endmodule
module top;
  sub u();
  reg [7:0] t;
  reg g = 0;
  always @(g) begin
    t = 8'h5a;
    if ($bits(u.tid) != 32) t[1] = g;
    else t[0] = g;
  end
  initial begin #1 g = 1; #1 $display("A t=%h b=%0d", t, $bits(u.tid)); #10 $finish; end
endmodule
"#,
            Want::Prints("A t=5a b=8"),
        ),
        // s12_bits_hier_ctrl_display: iverilog 13.0 and verilator 5.052 (control)
        (
            "s12_bits_hier_ctrl_display",
            r#"module sub;
  reg [7:0] tid;
endmodule
module top;
  sub u();
  initial begin
    #1;
    if ($bits(u.tid) == 32) $display("A then b=%0d", $bits(u.tid));
    else $display("A else b=%0d", $bits(u.tid));
    #10 $finish;
  end
endmodule
"#,
            Want::Prints("A else b=8"),
        ),
    ]);
}
