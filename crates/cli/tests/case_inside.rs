//! `case (e) inside` (IEEE 1800-2017 §12.5.4) — the accepted shapes. Each item is
//! compared with the set-membership `inside` operator (§11.4.13): a constant item's
//! x/z/? bits are don't-cares, a range is `lo <= e && e <= hi`, the first item that
//! answers `1'b1` takes its arm, an x answer is no match, and the case expression is
//! evaluated exactly once, before the items. `unique` / `unique0` / `priority` behave
//! as for a plain `case`. Before this slice every design here was three cascading
//! E2002 parse errors (or an E3010 for `inside[lo:hi]` read as a part-select).
//!
//! Accepted only where the reference tools' sizing rules agree (the refused shapes
//! are pinned in `case_inside_refused.rs`): sv2v per pair (§11.4.13), §12.5's
//! collective rule, and verilator's (collective width, each item extended by its own
//! sign).
//!
//! Oracles, recorded per test: sv2v 0.0.13 → iverilog 13.0 (4-state; it spells the
//! construct as an `if` chain that re-reads the case expression per item), verilator
//! 5.052 (2-state cells only — it reads x/z as 0; and its case-inside compares a signed
//! range that spans zero unsigned, a self-contradiction that disqualifies it there),
//! iverilog 13.0 itself rejects `case … inside`. Census: §4.5.582's grounding
//! (c01–c40, m1–m9, f1, a1, v2, the 312-row sign/width matrix) and plan probes q1–q3.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    /// The `$display` lines, without the simulator's trailer.
    fn lines(&self) -> Vec<&str> {
        self.out
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .collect()
    }
}

fn run(src: &str, args: &[&str]) -> Run {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_case_inside_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

/// Run `src` and assert a clean exit with exactly `want` as the `$display` lines.
fn expect(src: &str, want: &[&str]) -> Run {
    let r = run(src, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), want, "stdout:\n{}\nstderr:\n{}", r.out, r.err);
    r
}

const V01: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:7] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0000, 4'b0011, 4'b0100, 4'b0001};
  initial begin
    for (int i = 0; i < 8; i++) begin
      v = vals[i]; m = 9;
      case (v) inside
        4'b1?00: m = 1;
        [4'd1:4'd3]: m = 2;
        default: m = 0;
      endcase
      $display("v=%b m=%0d", v, m);
    end
    $finish;
  end
endmodule
"##;
const V02: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:9] = '{4'b1000, 4'b1001, 4'b1100, 4'b1101, 4'b0001, 4'b0111, 4'b0011, 4'b0000, 4'b1010, 4'b1011};
  initial begin
    for (int i = 0; i < 10; i++) begin
      v = vals[i]; m = 9;
      case (v) inside 4'b1x0z: m = 1; 4'b0zz1: m = 2; 4'b??11: m = 3; default: m = 0; endcase
      $display("v=%b m=%0d", v, m);
    end
    $finish;
  end
endmodule
"##;
const V03: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:8] = '{4'b1x00, 4'bx100, 4'b011x, 4'b100z, 4'bxxxx, 4'b1z00, 4'b1001, 4'bzzzz, 4'b10x1};
  initial begin
    for (int i = 0; i < 9; i++) begin
      v = vals[i]; m = 9;
      case (v) inside 4'b1?00: m = 1; 4'b0110: m = 2; [4'd8:4'd9]: m = 3; default: m = 0; endcase
      $display("v=%b m=%0d", v, m);
    end
    $finish;
  end
endmodule
"##;
const V04: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [3:0] v; int m;
  logic signed [3:0] vals [0:7] = '{-4'sd2, -4'sd1, 4'sd0, 4'sd1, 4'sd2, 4'sd3, -4'sd8, 4'sd7};
  initial begin
    for (int i = 0; i < 8; i++) begin
      v = vals[i]; m = 9;
      case (v) inside [-4'sd2:4'sd1]: m = 1; [4'sd3:4'sd5]: m = 2; default: m = 0; endcase
      $display("v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
"##;
const V05: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; logic [7:0] w; int m;
  task automatic a(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 8'b0000_1?00: m = 1; 8'h1F: m = 2; [8'd5:8'd6]: m = 3; 8'b1???_??11: m = 4; default: m = 0; endcase
    $display("a v=%b m=%0d", v, m);
  endtask
  task automatic b(input logic [7:0] x);
    w = x; m = 9;
    case (w) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("b w=%h m=%0d", w, m);
  endtask
  initial begin
    a(4'b1100); a(4'b1000); a(4'b1111); a(4'd5); a(4'd6); a(4'b0011);
    b(8'h0C); b(8'h1C); b(8'h08); b(8'h01); b(8'h11); b(8'hF8);
    $finish;
  end
endmodule
"##;
const V06: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  task automatic rev(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside [4'd3:4'd1]: m = 1; default: m = 0; endcase
    $display("rev v=%0d m=%0d", v, m);
  endtask
  task automatic multi(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 4'd1, 4'd3, [4'd8:4'd9]: m = 1; 4'd2, 4'b11??: m = 2; default: m = 0; endcase
    $display("multi v=%0d m=%0d", v, m);
  endtask
  task automatic overlap(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside [4'd0:4'd7]: m = 1; 4'd5: m = 2; 4'b01??: m = 3; 4'b1???: m = 4; default: m = 0; endcase
    $display("overlap v=%0d m=%0d", v, m);
  endtask
  task automatic nodef(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase
    $display("nodef v=%b m=%0d", v, m);
  endtask
  initial begin
    rev(4'd1); rev(4'd2); rev(4'd3);
    multi(4'd1); multi(4'd3); multi(4'd8); multi(4'd9); multi(4'd2); multi(4'd12); multi(4'd15); multi(4'd4);
    overlap(4'd5); overlap(4'd6); overlap(4'd8); overlap(4'd9);
    nodef(4'd1); nodef(4'd2); nodef(4'd4); nodef(4'bxx00);
    $finish;
  end
endmodule
"##;
const V07: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  localparam logic [3:0] P = 4'b1000;
  logic [71:0] w72; logic [3:0] v; logic [35:0] w36; int m;
  initial begin
    m = 9; case (4'b1100) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("const m=%0d", m);
    m = 9; case (P) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("param m=%0d", m);
    w72 = 72'h80_0000_0000_0000_0050; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h80_0000_0000_0000_0051; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h2; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h0; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    v = 4'bxxxx; m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    v = 4'bzzzz; m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    v = 4'd3;    m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    w36 = 36'h1; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    w36 = 36'h3; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    w36 = 36'h2; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    $finish;
  end
endmodule
"##;
const V07E: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:2] = '{4'd1, 4'd2, 4'd3};
  initial begin
    for (int i = 0; i < 3; i++) begin
      v = vals[i]; m = 9;
      case (v) inside 4'd1: ; 4'd2: m = 2; default: m = 0; endcase
      $display("empty v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
"##;
const V08: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [63:0] v; int m;
  task automatic t1(input logic [63:0] x);
    v = x;
    case (v) inside [0:3]: m = 1; 32'sd7: m = 2; default: m = 0; endcase
    $display("v=%h m=%0d", v, m);
  endtask
  initial begin
    t1(64'd2); t1(64'd7); t1(64'd5); t1(64'hFFFFFFFF_FFFFFFFF); t1(64'h1_00000002);
    $finish;
  end
endmodule
"##;
const V09: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v; logic signed [7:0] s; int m;
  task automatic tu(input logic [7:0] x);
    v = x;
    case (v) inside (-4'sd8 + 8'sd0): m = 1; default: m = 0; endcase
    $display("u v=%h m=%0d", v, m);
  endtask
  task automatic ts(input logic signed [7:0] x);
    s = x;
    case (s) inside (-4'sd8 + 8'sd0): m = 1; default: m = 0; endcase
    $display("s s=%h m=%0d", s, m);
  endtask
  initial begin
    tu(8'hF8); tu(8'h08); ts(8'shF8); ts(8'sh08);
    $finish;
  end
endmodule
"##;
const V10: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v8; bit [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("sum m=%0d", m);
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("sumrange m=%0d", m);
    v8 = 8'hF7;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("notF7 m=%0d", m);
    v8 = 8'h07;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("not07 m=%0d", m);
    $finish;
  end
endmodule
"##;
const V10X: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v8; logic [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("sumrange m=%0d", m);
    $finish;
  end
endmodule
"##;
const V11: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  typedef enum logic [2:0] {R_F, I_F, S_F, B_F, U_F, J_F} fmt_t;
  string s; fmt_t f; int m;
  initial begin
    s = "ab";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    s = "zz";
    case (s) inside "aa", "ab": m = 1; default: m = 0; endcase $display("s=%s m=%0d", s, m);
    for (int i = 0; i < 6; i++) begin
      f = fmt_t'(i);
      case (f) inside R_F: m = 1; S_F, B_F: m = 2; [U_F:J_F]: m = 3; default: m = 0; endcase
      $display("f=%0d m=%0d", f, m);
    end
    $finish;
  end
endmodule
"##;
const V12: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m; bit [3:0] k2; int ki;
  initial begin
    k2 = 4'd3; ki = 5;
    for (int i = 0; i < 9; i++) begin
      v = i[3:0];
      case (v) inside k2: m = 1; ki: m = 2; default: m = 0; endcase
      $display("var v=%0d m=%0d", v, m);
    end
    for (int i = 6; i < 11; i++) begin
      v = i[3:0];
      case (v) inside [k2 + 4'd5 : 4'd9]: m = 3; default: m = 0; endcase
      $display("rng v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
"##;
const X01: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [15:0] v; int m;
  initial begin
    v = 16'h6162; case (v) inside "ab": m = 1; default: m = 0; endcase $display("m=%0d", m);
    v = 16'h6163; case (v) inside "ab": m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const E01: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int cnt = 0; int m;
  function automatic logic [3:0] f(input int n); cnt++; $display("f(%0d) call %0d", n, cnt); return n[3:0]; endfunction
  initial begin
    case (f(3)) inside 4'd1: m = 1; [4'd5:4'd6]: m = 2; 4'b1?00: m = 3; 4'd3: m = 4; default: m = 0; endcase
    $display("A m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (f(9)) inside 4'd1, 4'd2: m = 1; default: m = 0; endcase
    $display("B m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (f(7)) inside default: m = 5; endcase
    $display("C m=%0d cnt=%0d", m, cnt);
    $finish;
  end
endmodule
"##;
const E02: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  function automatic int fn(input int n);
    case (g(n)) inside 4'd1: fn = 1; [4'd5:4'd6]: fn = 2; 4'b1?00: fn = 3; 4'd3: fn = 4; default: fn = 0; endcase
  endfunction
  task automatic tk(input int n, output int o);
    case (g(n)) inside 4'd1: o = 1; [4'd5:4'd6]: o = 2; 4'b1?00: o = 3; 4'd3: o = 4; default: o = 0; endcase
  endtask
  initial begin
    case (g(3)) inside 4'd1: m = 1; [4'd5:4'd6]: m = 2; 4'b1?00: m = 3; 4'd3: m = 4; default: m = 0; endcase
    $display("mod m=%0d", m);
    case (g(7)) inside default: m = 5; endcase
    $display("defonly m=%0d", m);
    m = fn(3); $display("fn m=%0d", m);
    tk(3, m); $display("tk m=%0d", m);
    $finish;
  end
endmodule
"##;
const C01: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic clk = 0; logic [3:0] v; int mc, mf, mt; logic [7:0] r;
  class C;
    function int meth(logic [3:0] x);
      case (x) inside 4'b1?00: return 1; [4'd1:4'd3]: return 2; default: return 0; endcase
    endfunction
  endclass
  function automatic int fn(input logic [3:0] x);
    case (x) inside 4'b1?00: fn = 1; [4'd1:4'd3]: fn = 2; default: fn = 0; endcase
  endfunction
  task automatic tk(input logic [3:0] x, output int o);
    case (x) inside 4'b1?00: o = 1; [4'd1:4'd3]: o = 2; default: o = 0; endcase
  endtask
  always_comb begin
    case (v) inside 4'b1?00: mc = 1; [4'd1:4'd3]: mc = 2; default: mc = 0; endcase
  end
  always_ff @(posedge clk) begin
    case (v) inside 4'b1?00: r <= 8'hAB; [4'd1:4'd3]: r <= 4'h5; default: r <= 16'hFFEE; endcase
  end
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    C c; c = new;
    for (int i = 0; i < 5; i++) begin
      v = vals[i]; #1 clk = 1; #1 clk = 0;
      tk(v, mt); mf = fn(v);
      case (v) inside
        [4'd0:4'd7]: case (v) inside 4'b0?10: $display("nest A"); default: $display("nest B"); endcase
        default: $display("nest C");
      endcase
      $display("v=%b comb=%0d ff=%h fn=%0d task=%0d meth=%0d", v, mc, r, mf, mt, c.meth(v));
    end
    $finish;
  end
endmodule
"##;
const C02: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] x; logic [31:0] y;
  function automatic int fn(input logic [3:0] a);
    case (a) inside 4'b1?00: fn = 1; [4'd1:4'd3]: fn = 2; default: fn = 0; endcase
  endfunction
  assign y = fn(x);
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    for (int i = 0; i < 5; i++) begin x = vals[i]; #1 $display("x=%b y=%0d", x, y); end
    $finish;
  end
endmodule
"##;
const C03: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v, lo, hi; int m;
  always_comb begin
    case (v) inside [lo:hi]: m = 1; 4'b1?00: m = 2; default: m = 0; endcase
  end
  initial begin
    v = 4'd5; lo = 4'd1; hi = 4'd3;
    #1 $display("t1 m=%0d", m);
    hi = 4'd6;
    #1 $display("t2 m=%0d", m);
    lo = 4'd6;
    #1 $display("t3 m=%0d", m);
    v = 4'd12;
    #1 $display("t4 m=%0d", m);
    $finish;
  end
endmodule
"##;
const UNIQUE_A: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9; unique case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    v = 4'd5; m = 9; unique case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;
const UNIQUE_B: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd5; m = 9; unique case (v) inside [4'd0:4'd7]: m = 1; 4'd5: m = 2; endcase $display("v=%b m=%0d", v, m);
    v = 4'd1; m = 9; unique case (v) inside [4'd0:4'd7]: m = 1; 4'd5: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;
const UNIQUE_C: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9; unique0 case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    v = 4'd5; m = 9; unique0 case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;
const UNIQUE_D: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9; priority case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    v = 4'd5; m = 9; priority case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;
const UNIQUE_E: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd5; m = 9; priority case (v) inside [4'd0:4'd7]: m = 1; 4'd5: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;
const UNIQUE_F: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; default: m = 0; endcase $display("v=%b m=%0d", v, m);
    v = 4'd12; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; default: m = 0; endcase $display("v=%b m=%0d", v, m);
    v = 4'd3; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; default: m = 0; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;
const UNIQUE_G: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'bx001; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; endcase $display("v=%b m=%0d", v, m);
    v = 4'b1x00; m = 9; unique case (v) inside 4'd1: m = 1; 4'b1?00: m = 2; endcase $display("v=%b m=%0d", v, m);
    $finish;
  end
endmodule
"##;

/// c01, the queue row's own repro: a wildcard value and a range under a default.
/// sv2v → iverilog and verilator print exactly these lines.
#[test]
fn v01_wildcard_value_and_range() {
    expect(
        V01,
        &[
            "v=1000 m=1",
            "v=0010 m=2",
            "v=0110 m=0",
            "v=1100 m=1",
            "v=0000 m=0",
            "v=0011 m=2",
            "v=0100 m=0",
            "v=0001 m=2",
        ],
    );
}

/// c02: x, z and ? digits of an item are don't-cares at any position. sv2v → iverilog
/// and verilator print exactly these lines.
#[test]
fn v02_xz_digits_in_items_are_dont_cares() {
    expect(
        V02,
        &[
            "v=1000 m=1",
            "v=1001 m=1",
            "v=1100 m=1",
            "v=1101 m=1",
            "v=0001 m=2",
            "v=0111 m=2",
            "v=0011 m=2",
            "v=0000 m=0",
            "v=1010 m=0",
            "v=1011 m=3",
        ],
    );
}

/// c03: an x/z bit of the CASE EXPRESSION is a don't-care only where the item has a
/// wildcard; against a 0/1 item bit the comparison is x, which is no match (§12.5.4,
/// §11.4.6). sv2v → iverilog prints exactly these lines (= hand-IEEE); verilator is
/// 2-state, and refuses the `z` in the value table (`Unsupported tristate construct`).
#[test]
fn v03_xz_case_expression() {
    expect(
        V03,
        &[
            "v=1x00 m=1",
            "v=x100 m=0",
            "v=011x m=0",
            "v=100z m=0",
            "v=xxxx m=0",
            "v=1z00 m=1",
            "v=1001 m=3",
            "v=zzzz m=0",
            "v=10x1 m=0",
        ],
    );
}

/// c05: an all-signed set compares signed. sv2v → iverilog prints exactly these lines
/// (= hand-IEEE). verilator prints `m=0` for -2..1: its case-inside compares a signed
/// range that spans zero unsigned — here at 4 bits, and an int-wide `[-2:0]` too (review
/// d06c: `m=0` where its own `if (k inside {[-2:0]})` prints 1) — so it contradicts
/// itself and is disqualified on such ranges.
#[test]
fn v04_all_signed_ranges() {
    expect(
        V04,
        &[
            "v=-2 m=1", "v=-1 m=1", "v=0 m=1", "v=1 m=1", "v=2 m=0", "v=3 m=2", "v=-8 m=0",
            "v=7 m=0",
        ],
    );
}

/// c06a / c06b: unsigned leaves of different widths — every rule zero-extends to the
/// pair or case width alike. sv2v → iverilog and verilator print exactly these lines.
#[test]
fn v05_mixed_width_unsigned_leaves() {
    expect(
        V05,
        &[
            "a v=1100 m=1",
            "a v=1000 m=1",
            "a v=1111 m=0",
            "a v=0101 m=3",
            "a v=0110 m=3",
            "a v=0011 m=0",
            "b w=0c m=1",
            "b w=1c m=0",
            "b w=08 m=1",
            "b w=01 m=2",
            "b w=11 m=0",
            "b w=f8 m=0",
        ],
    );
}

/// c07 reversed range (empty), c10 several elements per item, c11 overlapping items
/// (the first wins), c12 no default and no match (nothing runs). sv2v → iverilog prints
/// exactly these lines; verilator agrees except `nodef v=0000 m=9` (2-state `xx00`).
#[test]
fn v06_reversed_multi_overlap_nodefault() {
    expect(
        V06,
        &[
            "rev v=1 m=0",
            "rev v=2 m=0",
            "rev v=3 m=0",
            "multi v=1 m=1",
            "multi v=3 m=1",
            "multi v=8 m=1",
            "multi v=9 m=1",
            "multi v=2 m=2",
            "multi v=12 m=2",
            "multi v=15 m=2",
            "multi v=4 m=0",
            "overlap v=5 m=1",
            "overlap v=6 m=1",
            "overlap v=8 m=4",
            "overlap v=9 m=4",
            "nodef v=0001 m=1",
            "nodef v=0010 m=2",
            "nodef v=0100 m=9",
            "nodef v=xx00 m=9",
        ],
    );
}

/// c20a/c20b a constant case expression, c32 a 72-bit one, c37 an all-`?` item, c40
/// an unsized `'bx1` item against a 36-bit case expression (its x MSB extends as x).
/// sv2v → iverilog prints exactly these lines; verilator agrees on every `m` and prints
/// the `allq` operands as `0000` (2-state).
#[test]
fn v07_constant_wide_allq_unsized() {
    expect(
        V07,
        &[
            "const m=1",
            "param m=1",
            "w72=800000000000000050 m=1",
            "w72=800000000000000051 m=0",
            "w72=000000000000000002 m=2",
            "w72=000000000000000000 m=0",
            "allq v=xxxx m=1",
            "allq v=zzzz m=1",
            "allq v=0011 m=1",
            "bx1 w36=000000001 m=1",
            "bx1 w36=000000003 m=1",
            "bx1 w36=000000002 m=0",
        ],
    );
}

/// c38: an empty statement as an arm still claims its match. sv2v → iverilog and
/// verilator print exactly these lines.
#[test]
fn v07e_empty_arm() {
    expect(V07E, &["empty v=1 m=9", "empty v=2 m=2", "empty v=3 m=0"]);
}

/// q1 A: a 64-bit unsigned case expression against 32-bit SIGNED constants whose MSB
/// is 0 (`[0:3]`, `32'sd7`): sign- and zero-extension agree, so every rule does. sv2v →
/// iverilog and verilator print exactly these lines.
#[test]
fn v08_wide_unsigned_against_nonnegative_signed_constants() {
    expect(
        V08,
        &[
            "v=0000000000000002 m=1",
            "v=0000000000000007 m=2",
            "v=0000000000000005 m=0",
            "v=ffffffffffffffff m=0",
            "v=0000000100000002 m=0",
        ],
    );
}

/// q2: a signed operator item at the case width is evaluated in its pair's sign — an
/// unsigned case expression makes `-4'sd8` zero-extend (`-8'h08` = `f8`), a signed one
/// sign-extends (`-8'hf8` = `08`). sv2v → iverilog and verilator print exactly these.
#[test]
fn v09_signed_operator_item_at_the_case_width() {
    expect(
        V09,
        &["u v=f8 m=1", "u v=08 m=0", "s s=f8 m=0", "s s=08 m=1"],
    );
}

/// m1 (cells 2–4): an operator item is evaluated at its pair's width — `a4 + b4`
/// against an 8-bit case expression keeps its carry, `~a4` sets its upper bits. sv2v →
/// iverilog and verilator print exactly these lines. The operands are `bit`: a 4-state
/// operator VALUE item is refused (`case_inside_refused.rs`), its range twin is not.
#[test]
fn v10_operator_items_take_the_pair_width() {
    expect(V10, &["sum m=1", "sumrange m=1", "notF7 m=1", "not07 m=0"]);
    // a range bound over 4-state operands: `<=`/`>=` need no wildcard. Both oracles: 1.
    expect(V10X, &["sumrange m=1"]);
}

/// m7: enum labels (two in one item, and a label range), and a `string` case
/// expression against string values (the `StrCmp` compare a plain `case (s)` uses).
/// verilator prints exactly these lines; sv2v → iverilog aborts on the string case
/// (`draw_eval_vec4` assertion), also on a design holding only the two string cells.
#[test]
fn v11_enum_labels_and_string_values() {
    expect(
        V11,
        &[
            "s=ab m=1", "s=zz m=0", "f=0 m=1", "f=1 m=0", "f=2 m=2", "f=3 m=2", "f=4 m=3",
            "f=5 m=3",
        ],
    );
}

/// v2, split where every rule agrees: 2-state variable items (`bit [3:0]`, `int`), and
/// a range whose bound is an operator at the case width. sv2v → iverilog and verilator
/// print exactly these lines.
#[test]
fn v12_two_state_variable_items_and_operator_bound() {
    expect(
        V12,
        &[
            "var v=0 m=0",
            "var v=1 m=0",
            "var v=2 m=0",
            "var v=3 m=1",
            "var v=4 m=0",
            "var v=5 m=2",
            "var v=6 m=0",
            "var v=7 m=0",
            "var v=8 m=0",
            "rng v=6 m=0",
            "rng v=7 m=0",
            "rng v=8 m=3",
            "rng v=9 m=3",
            "rng v=10 m=0",
        ],
    );
}

/// A string literal item under a packed case expression is its packed value. sv2v →
/// iverilog and verilator print exactly these lines.
#[test]
fn v13_string_literal_item_under_packed_case_expression() {
    expect(X01, &["m=1", "m=0"]);
}

/// m2: the case expression runs exactly once — with four items and a late match, with
/// a miss, and with only a default. sv2v → iverilog and verilator print exactly these.
#[test]
fn e01_case_expression_evaluated_once() {
    expect(
        E01,
        &[
            "f(3) call 1",
            "A m=4 cnt=1",
            "f(9) call 1",
            "B m=0 cnt=1",
            "f(7) call 1",
            "C m=5 cnt=1",
        ],
    );
}

/// m2b + f1: the same for a pure function, at module scope (including the
/// default-only statement, whose case expression still runs) and inside a function and
/// a task body. sv2v → iverilog and verilator print exactly these lines.
#[test]
fn e02_evaluated_once_in_module_function_and_task() {
    expect(
        E02,
        &[
            "g(3)",
            "mod m=4",
            "g(7)",
            "defonly m=5",
            "g(3)",
            "fn m=4",
            "g(3)",
            "tk m=4",
        ],
    );
}

/// m4: `always_comb`, `always_ff` (arms of three widths into an 8-bit reg), a
/// function, a task, a class method (its case expression is a formal: repeatable, so
/// re-reading it is safe where the method body has no capture slot) and a nested case
/// inside. verilator prints exactly these lines (sv2v cannot parse the class).
#[test]
fn c01_procedural_contexts() {
    expect(
        C01,
        &[
            "nest C",
            "v=1000 comb=1 ff=ab fn=1 task=1 meth=1",
            "nest A",
            "v=0010 comb=2 ff=05 fn=2 task=2 meth=2",
            "nest A",
            "v=0110 comb=0 ff=ee fn=0 task=0 meth=0",
            "nest C",
            "v=1100 comb=1 ff=ab fn=1 task=1 meth=1",
            "nest B",
            "v=0011 comb=2 ff=05 fn=2 task=2 meth=2",
        ],
    );
}

/// A function holding a case inside, called from a continuous assign. sv2v → iverilog
/// and verilator print exactly these lines.
#[test]
fn c02_function_from_continuous_assign() {
    expect(
        C02,
        &[
            "x=1000 y=1",
            "x=0010 y=2",
            "x=0110 y=0",
            "x=1100 y=1",
            "x=0011 y=2",
        ],
    );
}

/// a1: an `always_comb` is sensitive to a range's bound variables. sv2v → iverilog and
/// verilator print exactly these lines.
#[test]
fn c03_always_comb_wakes_on_range_bounds() {
    expect(C03, &["t1 m=0", "t2 m=1", "t3 m=0", "t4 m=2"]);
}

fn w4031(r: &Run) -> usize {
    r.err.matches("VITA-W4031").count()
}

/// c13a–g: `unique` / `unique0` / `priority`. The values are sv2v → iverilog's; the
/// no-match report is W4031, exactly as for a plain `case` (verilator reports `unique
/// case, but none matched` / `priority case, but non-match found` and stops). An
/// OVERLAP is not reported (c13b): verilator reports `unique case, but multiple matches
/// found`; the overlap check is a documented cut for plain `case` too.
#[test]
fn u01_unique_unique0_priority() {
    let r = expect(UNIQUE_A, &["v=0001 m=1", "v=0101 m=9"]);
    assert_eq!(w4031(&r), 1, "stderr:\n{}", r.err);
    let r = expect(UNIQUE_B, &["v=0101 m=1", "v=0001 m=1"]);
    assert_eq!(w4031(&r), 0, "stderr:\n{}", r.err);
    // unique0: no report (verilator: none either)
    let r = expect(UNIQUE_C, &["v=0001 m=1", "v=0101 m=9"]);
    assert_eq!(w4031(&r), 0, "stderr:\n{}", r.err);
    let r = expect(UNIQUE_D, &["v=0001 m=1", "v=0101 m=9"]);
    assert_eq!(w4031(&r), 1, "stderr:\n{}", r.err);
    let r = expect(UNIQUE_E, &["v=0101 m=1"]);
    assert_eq!(w4031(&r), 0, "stderr:\n{}", r.err);
    let r = expect(UNIQUE_F, &["v=0001 m=1", "v=1100 m=2", "v=0011 m=0"]);
    assert_eq!(w4031(&r), 0, "stderr:\n{}", r.err);
    // an x case-expression bit against the items' 0/1 bits matches nothing (verilator
    // is 2-state: `v=0001 m=1`, `v=1000 m=2`)
    let r = expect(UNIQUE_G, &["v=x001 m=9", "v=1x00 m=2"]);
    assert_eq!(w4031(&r), 1, "stderr:\n{}", r.err);
}

/// The three executors print the same bytes for the capture, the wildcard compare and
/// the evaluate-once cells.
#[test]
fn b01_backends_agree() {
    for src in [V01, V03, E01] {
        let base = run(src, &[]);
        assert_eq!(base.code, 0, "stderr:\n{}", base.err);
        for be in ["interp", "vm", "native"] {
            let r = run(src, &["--backend", be]);
            assert_eq!(r.code, 0, "{be} stderr:\n{}", r.err);
            assert_eq!(r.out, base.out, "{be} stdout differs");
        }
    }
}

fn tmp(ext: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "vita_case_inside_st_{}_{n}.{ext}",
        std::process::id()
    ))
}

fn s(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

/// S1: the `.vu` round trip keeps the `Inside` kind and its labels.
#[test]
fn s1_vu_round_trip_keeps_inside() {
    let src = tmp("sv");
    std::fs::write(&src, V01).unwrap();
    let vu = tmp("vu");
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    let bytes = std::fs::read(&vu).unwrap();
    let (_h, body) = vita_artifact::read_vu(&bytes).expect("read_vu");
    let decoded: hdl_ast::SourceUnit = postcard::from_bytes(body).expect("decode SourceUnit");
    let reference =
        cli::frontend_to_unit(&s(&src), &cli::StderrSink::new()).expect("reference frontend parse");
    assert_eq!(decoded, reference);
    let text = format!("{decoded:?}");
    assert!(
        text.contains("kind: Inside"),
        "no Inside case in the decoded unit"
    );
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&vu);
}

/// S2: vcmp → velab → vrun prints what the one-shot run prints, for a plain and a
/// `unique` case inside.
#[test]
fn s2_staged_chain_matches_oneshot() {
    for design in [V01, UNIQUE_A] {
        let src = tmp("sv");
        std::fs::write(&src, design).unwrap();
        let vu = tmp("vu");
        let velab = tmp("velab");
        let ref_unit = cli::frontend_to_unit(&s(&src), &cli::StderrSink::new()).unwrap();
        let ref_ir = elaborate::elaborate(&ref_unit, &cli::StderrSink::new()).unwrap();
        let (_, ref_out) = sim_engine::simulate_capture(&ref_ir, sim_engine::SimOpts::default());
        assert_eq!(
            cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
            cli::EXIT_OK
        );
        assert_eq!(
            cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
            cli::EXIT_OK
        );
        assert_eq!(
            cli::run_vrun(&s(&velab), &cli::VitaOpts::default()),
            cli::EXIT_OK
        );
        let bytes = std::fs::read(&velab).unwrap();
        let (_h, body) = vita_artifact::read_velab(&bytes).unwrap();
        let (staged_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).unwrap();
        let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).unwrap();
        let (_, staged_out) = sim_engine::simulate_capture(
            &staged_ir,
            sim_engine::SimOpts {
                fork_modes: modes,
                ..Default::default()
            },
        );
        assert_eq!(staged_out, ref_out);
        assert!(ref_out.contains("m=1"), "{ref_out}");
        for p in [&src, &vu, &velab] {
            let _ = std::fs::remove_file(p);
        }
    }
}
