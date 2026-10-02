//! `case (e) inside` (IEEE 1800-2017 §12.5.4) — the refused shapes. Each is ONE
//! `VITA-E3009` naming the construct and the reason, with no value printed, where the
//! statement used to be three cascading E2002 parse errors: nothing descends the
//! accuracy ladder. A shape is refused when the reference tools' sizing rules split on
//! it (§12.5 sizes a case collectively, §11.4.13 per pair, verilator extends each item
//! by its own sign), when no tool runs it, or when the code it would route into is
//! wrong today (each such row has its own queue entry).
//!
//! Oracles, recorded per test: sv2v 0.0.13 → iverilog 13.0 and verilator 5.052 (their
//! outputs quoted beside each refusal, to show the split or the missing oracle);
//! iverilog 13.0 rejects `case … inside` itself. Census: §4.5.582's grounding and plan.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    fn lines(&self) -> Vec<&str> {
        self.out
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .collect()
    }
    fn errors(&self) -> Vec<&str> {
        self.err.lines().filter(|l| l.contains("error[")).collect()
    }
}

fn run(src: &str) -> Run {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_case_inside_r_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
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

const PREFIX: &str = "`case … inside` is not supported here";

/// Exactly one error, the case-inside refusal, naming `reason`; exit 1, no value.
fn refused(src: &str, reason: &str) -> Run {
    let r = run(src);
    assert_eq!(r.code, 1, "stderr:\n{}", r.err);
    let errs = r.errors();
    assert_eq!(errs.len(), 1, "stderr:\n{}", r.err);
    assert!(errs[0].contains("error[VITA-E3009]"), "{}", errs[0]);
    assert!(errs[0].contains(PREFIX), "{}", errs[0]);
    assert!(errs[0].contains(reason), "{}", errs[0]);
    assert!(r.lines().is_empty(), "stdout:\n{}", r.out);
    r
}

const SIGN: &str = "mixed signedness";
const WIDTH: &str = "operator narrower than the widest operand";
const RUNTIME_XZ: &str = "known only at run time";

const L01: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [3:0] v; int m;
  initial begin
    v = -4'sd1; case (v) inside -1: m = 1; 8'h00: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L02: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] a, b; int m;
  initial begin
    a = 8'h80; b = 8'h80; case (a + b) inside 8'h00: m = 1; 16'h0100: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L03: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [3:0] v; int m;
  initial begin
    v = -4'sd1; case (v) inside [-2:-1]: m = 1; [4'd0:4'd1]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L04: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd15; case (v) inside 1, 3: m = 1; [4:7]: m = 2; '1: m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L05: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [63:0] v; int m;
  initial begin
    v = 64'h00000000_FFFFFFFF; case (v) inside 32'shFFFFFFFF: m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L06: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m; logic [3:0] w = 4'b1x00;
  initial begin
    v = 4'b1000; case (v) inside w: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L07: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'b1000; case (v) inside {2'b1?, 2'b00}: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L07B: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v8; logic [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10; case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L08: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  real r; int m;
  initial begin
    r = 2.5; case (r) inside 1.0: m = 1; [2.0:3.0]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L09: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  string s; int m;
  initial begin
    s = "am"; case (s) inside ["aa":"az"]: m = 1; "b": m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L10: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int m;
  function automatic string sf(input int n); $display("sf(%0d)", n); return "b"; endfunction
  initial begin
    case (sf(1)) inside "b": m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L11: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  initial begin
    v = 4'd5; case (v) inside g(1), g(2): m = 1; [g(3):g(6)]: m = 2; g(5): m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L11R: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  initial begin
    v = 4'd5; case (v) inside [g(3):g(6)]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L12: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd15; case (v) inside [4'd12:$]: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const D01: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd5; case (v) inside [4'd1:$]: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L13: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int m;
  function int f(input int x); return x; endfunction
  initial begin
    case (f(-1)) inside [-4:4]: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L14: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  class C;
    function logic [3:0] g(int n); $display("g(%0d)", n); return n[3:0]; endfunction
    function int meth(int n);
      case (g(n)) inside 4'd1: return 1; [4'd5:4'd6]: return 2; 4'd3: return 4; default: return 0; endcase
    endfunction
  endclass
  initial begin
    C c; c = new;
    $display("meth m=%0d", c.meth(3));
    $finish;
  end
endmodule
"##;
const L15: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  class C; int x; endclass
  C h; int m;
  initial begin
    h = null;
    case (h) inside null: m = 1; default: m = 0; endcase
    $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L16A: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [3:0] v; int m;
  initial begin
    v = -4'sd1; case (v) inside [4'd1:4'd3]: m = 1; [-1:1]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L16B: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [7:0] v; int m;
  initial begin
    v = 8'shFC; case (v) inside 4'sb1?00: m = 1; 4'b1?11: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const X02: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] a, b; int m;
  initial begin
    a = 8'h80; b = 8'h80; case ((1:a + b:2)) inside 9'h100: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const D02V: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int q[$]; int x; int m;
  initial begin
    q = '{1, 5, 9};
    x = 9; case (x) inside q[$]: m = 1; default: m = 0; endcase $display("x=%0d m=%0d", x, m);
    $finish;
  end
endmodule
"##;
const D02: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int q[$]; int x; int m;
  initial begin
    q = '{1, 5, 9};
    x = 9; case (x) inside [q[$]:q[$]]: m = 1; default: m = 0; endcase $display("x=%0d m=%0d", x, m);
    x = 5; case (x) inside [q[$]:q[$]]: m = 1; default: m = 0; endcase $display("x=%0d m=%0d", x, m);
    $finish;
  end
endmodule
"##;
const P01Z: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; casez (v) inside 4'd1: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const P01X: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; casex (v) inside 4'd1: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const K01G: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  localparam P = 2;
  generate case (P) inside 1: begin : g1 initial $display("one"); end [2:3]: begin : g2 initial $display("two"); end endcase endgenerate
  initial #1 $finish;
endmodule
"##;
const K01C: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  function automatic int f(input logic [3:0] x);
    case (x) inside [4'd1:4'd3]: f = 1; 4'b1?00: f = 2; default: f = 0; endcase
  endfunction
  localparam int P1 = f(4'd2);
  initial begin $display("P1=%0d", P1); $finish; end
endmodule
"##;
const K01A: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m; logic [3:0] arr [0:1] = '{4'd5, 4'd7};
  initial begin
    v = 4'd5; case (v) inside arr: m = 1; 4'd2: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
"##;
const L17P: &str = r##"`timescale 1ns/1ns
package p;
  function automatic int f(input logic [3:0] x);
    case (x) inside 4'b1?00: f = 1; [4'd1:4'd3]: f = 2; default: f = 0; endcase
  endfunction
endpackage
module t;
  initial #1000 $finish; // watchdog
  initial begin
    $display("f=%0d", p::f(4'b1000));
    $finish;
  end
endmodule
"##;
const L17I: &str = r##"`timescale 1ns/1ns
package p;
  function automatic int f(input logic [3:0] x);
    case (x) inside 4'b1?00: f = 1; [4'd1:4'd3]: f = 2; default: f = 0; endcase
  endfunction
endpackage
module t;
  initial #1000 $finish; // watchdog
  import p::*;
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    for (int i = 0; i < 5; i++) $display("x=%b f=%0d", vals[i], f(vals[i]));
    $finish;
  end
endmodule
"##;
const F1A: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x; reg [3:0] inside; reg [3:0] m1, m2, m3;
  initial begin
    inside = 4'b0110;
    x = 4'd3; case (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    x = 4'd7; case (x) inside + 1: m2 = 1; default: m2 = 0; endcase
    x = 4'd6; case (x) inside & 4'hF: m3 = 1; default: m3 = 0; endcase
    $display("A m1=%0d B m2=%0d C m3=%0d", m1, m2, m3);
    $finish;
  end
endmodule
"##;
const F1B: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, inside; reg [3:0] m1, m2;
  initial begin
    inside = 4'd5; x = 4'd5;
    case (x) inside: m1 = 1; default: m1 = 0; endcase
    case (x) inside, 4'd9: m2 = 1; default: m2 = 0; endcase
    $display("D m1=%0d E m2=%0d", m1, m2);
    $finish;
  end
endmodule
"##;
const F1C: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, inside; reg [3:0] m1, m2;
  initial begin
    inside = 4'b0110; x = 4'd3;
    casez (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    casex (x) inside[2:1]: m2 = 1; default: m2 = 0; endcase
    $display("F m1=%0d m2=%0d", m1, m2);
    $finish;
  end
endmodule
"##;
const F1D: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, m2;
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  initial begin
    x = 4'd4;
    case (x) inside(3): m2 = 1; default: m2 = 0; endcase
    $display("G m2=%0d", m2);
    $finish;
  end
endmodule
"##;
const F1E: &str = r##"`begin_keywords "1364-2005"
`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x; reg [3:0] inside; reg [3:0] m1;
  initial begin
    inside = 4'b0110; x = 4'd3;
    case (x) inside[2:1]: m1 = 1; default: m1 = 0; endcase
    $display("A m1=%0d", m1);
    $finish;
  end
endmodule
`end_keywords
"##;
const F1F: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, inside; reg [3:0] m1;
  initial begin
    inside = 4'd5; x = 4'd1;
    case (x) inside == 4'd5: m1 = 1; default: m1 = 0; endcase
    $display("H m1=%0d", m1);
    $finish;
  end
endmodule
"##;
const F2A: &str = r##"`timescale 1ns/1ns
module child(output reg [3:0] m);
  reg [3:0] x;
  initial begin x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase end
endmodule
module t;
  initial #1000 $finish; // watchdog
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  wire [3:0] r;
  child u(r);
  initial begin #1 $display("up m=%0d", r); $finish; end
endmodule
"##;
const F2B: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] x; int m;
  initial begin
    x = 7;
    for (int inside = 6; inside < 7; inside++) begin case (x) inside + 1: m = 1; default: m = 0; endcase end
    $display("forvar m=%0d", m);
    $finish;
  end
endmodule
"##;
const ESC1: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x; reg [3:0] \inside ; int m1;
  initial begin
    \inside = 4'b0110; x = 4'd3;
    case (x) \inside [2:1]: m1 = 1; default: m1 = 0; endcase
    $display("esc m1=%0d", m1);
    $finish;
  end
endmodule
"##;
const ESC2: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, y; reg [3:0] \inside ; int m1, m2;
  initial begin
    \inside = 4'b0110; x = 4'd3; y = 4'd2;
    case (x) \inside [2:1]: m1 = 1; default: m1 = 0; endcase
    case (y) inside [4'd1:4'd3]: m2 = 1; default: m2 = 0; endcase
    $display("esc m1=%0d m2=%0d", m1, m2);
    $finish;
  end
endmodule
"##;
const CTL: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m, k;
  logic [3:0] vals [0:3] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100};
  initial begin
    for (int i = 0; i < 4; i++) begin
      v = vals[i];
      case (v) inside 4'b1?00: m = 1; [4'd1:4'd3]: m = 2; default: m = 0; endcase
      k = v inside {4'b1?00, [4'd1:4'd3]};
      $display("v=%b m=%0d k=%0d", v, m, k);
    end
    $finish;
  end
endmodule
"##;
const XA: &str = r##"package p;
  parameter int inside = 5;
endpackage
"##;
const XB: &str = r##"`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  import p::*;
  logic [3:0] x; int m;
  initial begin
    x = 4'd5;
    case (x) inside [4'd4:4'd6]: m = 1; default: m = 0; endcase
    $display("x m=%0d", m);
    $finish;
  end
endmodule
"##;

/// c19a: a signed case expression against `-1` and the unsigned `8'h00`. sv2v
/// (per pair) prints `m=1`, verilator (collectively unsigned) prints `m=0`. The error
/// points at the unsigned item.
#[test]
fn l01_mixed_signs_under_a_signed_case_expression() {
    let r = refused(L01, SIGN);
    assert!(r.err.contains("t.sv:6:44:"), "{}", r.err);
}

/// c19b: `a + b` (8 bits) is the case expression and `16'h0100` an item. sv2v
/// evaluates the sum at 16 bits (`m=1`), verilator at 8 (`m=2`).
#[test]
fn l02_operator_case_expression_narrower_than_an_item() {
    refused(L02, WIDTH);
}

/// c19c: a signed case expression against `[-2:-1]` and `[4'd0:4'd1]`. sv2v `m=1`,
/// verilator `m=0`.
#[test]
fn l03_mixed_sign_ranges() {
    refused(L03, SIGN);
}

/// c39: a fill item `'1`. verilator sizes it collectively (`m=0`), and sv2v writes it
/// as `1'sb1` here but as `{4{1'sb1}}` in its own `if` twin (`m=0` vs `m=3`) — no
/// oracle.
#[test]
fn l04_fill_item() {
    refused(L04, "fill literal");
}

/// q1 B: `32'shFFFFFFFF` against a 64-bit unsigned case expression. sv2v zero-extends
/// (`m=3`), verilator extends the item by its own sign (`m=0`).
#[test]
fn l05_narrow_signed_item_with_msb_set_under_unsigned_case_expression() {
    refused(L05, SIGN);
}

/// c30 / c31 / a 4-state operator: an item whose x/z bits exist only at run time would
/// be compared with `==` where §11.4.13 needs `==?` (c30: vita's `inside` operator
/// gives 0 where sv2v gives 1). sv2v and verilator print `m=1` for all three.
#[test]
fn l06_runtime_xz_value_items() {
    refused(L06, RUNTIME_XZ);
    refused(L07, RUNTIME_XZ);
    refused(L07B, RUNTIME_XZ);
}

/// m7: a `real` case expression. verilator fails internally (`V3Number.cpp:1910`),
/// sv2v → iverilog rejects (`^ operator may not have REAL operands`) — no oracle.
#[test]
fn l08_real_case_expression() {
    refused(L08, "`real` operand");
}

/// s1 / s2: a `string` range (verilator fails internally: `Unknown node type reached
/// emitter: INSIDERANGE`; sv2v → iverilog aborts), and a string-returning call as the
/// case expression (verilator `m=3`; a plain `case (sf(1))` prints 0 today — queued).
#[test]
fn l09_string_range_and_string_call() {
    refused(L09, "`string` operand");
    refused(L10, "`string` operand");
}

/// m3: calls in items. No tool orders them: sv2v → iverilog runs `g(1)` seven times,
/// verilator evaluates every item twice; both print `m=2`. A call in a value item is
/// also an item whose x/z bits exist only at run time; in a range bound it is refused
/// for the call alone (sv2v → iverilog: `g(3) g(6) m=2`, verilator runs each twice).
#[test]
fn l11_calls_in_items() {
    refused(L11, "a call");
    refused(L11R, "a call in an item");
}

/// c08a / D01: a `$` bound. sv2v cannot parse it; verilator reads it unsigned where
/// §11.4.13 reads the type's bound. The error points at the `$`.
#[test]
fn l12_dollar_bound() {
    let r = refused(L12, "`$` bound");
    assert!(r.err.contains("t.sv:6:39:"), "{}", r.err);
    refused(D01, "`$` bound");
}

/// p5: a call returning a signed type as the case expression. The capture would take
/// it unsigned (`case (f(-1)) 4'sb1111:` prints 0 today where iverilog and verilator
/// print 1 — queued). Here sv2v prints `m=1`, verilator `m=0`.
#[test]
fn l13_call_returning_signed_as_case_expression() {
    let r = refused(L13, "call returning a signed type");
    assert!(r.err.contains("t.sv:7:11:"), "{}", r.err);
}

/// f2: a class-method body reserves no capture slot, so a call as its case expression
/// would run once per tested item (a plain `case (g(n))` there runs `g` four times;
/// iverilog and verilator once — queued). verilator prints `g(3)` once and `meth m=4`;
/// sv2v cannot parse the class.
#[test]
fn l14_uncaptured_call_in_a_class_method() {
    refused(L14, "evaluated exactly once");
}

/// A class handle / `null` case expression. verilator `m=1`; sv2v cannot parse it.
#[test]
fn l15_handle_and_null() {
    refused(L15, "class handle or `null`");
}

/// c05b / c06c: a signed case expression with unsigned items. c05b: sv2v `m=2`,
/// verilator `m=0`. c06c happens to agree (`m=1` both) but sits on the same split.
#[test]
fn l16_signed_case_expression_with_unsigned_items() {
    refused(L16A, SIGN);
    refused(L16B, SIGN);
}

/// A `(min:typ:max)` operator case expression is the operator it chooses: narrower
/// than the item, it is refused like `a + b` (sv2v → iverilog and verilator `m=1`).
#[test]
fn l18_min_typ_max_operator_case_expression() {
    refused(X02, WIDTH);
}

/// A queue element value item cannot be proven free of x/z bits; its range twin is
/// accepted and `q[$]` there is the queue's last element, not the case expression.
/// verilator prints exactly the D02 lines (sv2v cannot parse the queue).
#[test]
fn d02_queue_last_element() {
    refused(D02V, RUNTIME_XZ);
    let r = run(D02);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), ["x=9 m=1", "x=5 m=0"]);
}

/// `casez` / `casex` have no `inside` form (A.6.7) and never start one: the word is
/// read as a label name, exactly as before the construct existed, so with nothing
/// declared `inside` this stays the parse errors it always was (sv2v: `cannot use
/// inside with casez`; verilator: `Illegal to have inside on a casex/casez`).
#[test]
fn p01_casez_casex_inside_stays_a_parse_error() {
    for src in [P01Z, P01X] {
        let r = run(src);
        assert_eq!(r.code, 1, "stderr:\n{}", r.err);
        let errs = r.errors();
        assert!(!errs.is_empty(), "stderr:\n{}", r.err);
        assert!(
            errs.iter().all(|l| l.contains("error[VITA-E2002]")),
            "{}",
            r.err
        );
        assert!(r.lines().is_empty(), "stdout:\n{}", r.out);
    }
}

const NAME_USE: &str =
    "`case … inside` is not supported in a design that also uses `inside` as a name";

/// Every error is the design-wide decline (one per `case … inside` statement) naming the
/// first name use at `first`; exit 1, no value.
fn declined(src: &str, n: usize, first: &str) -> Run {
    let r = run(src);
    declined_run(&r, n, first);
    r
}

fn declined_run(r: &Run, n: usize, first: &str) {
    assert_eq!(r.code, 1, "stderr:\n{}", r.err);
    let errs = r.errors();
    assert_eq!(errs.len(), n, "stderr:\n{}", r.err);
    for e in &errs {
        assert!(
            e.contains("error[VITA-E3009]") && e.contains(NAME_USE),
            "{e}"
        );
        // the file is named as the run named it (absolute for a one-file run here)
        let at = e.split("(first at ").nth(1).unwrap_or("");
        assert!(
            at.starts_with(first) || at.contains(&format!("/{first})")),
            "{e}"
        );
    }
    assert!(r.lines().is_empty(), "stdout:\n{}", r.out);
}

/// F1 / F2 (review rounds 1–2): IEEE 1364 does not reserve `inside`, so in a design that
/// also uses it as a NAME, `case (x) inside[2:1]:`, `inside + 1:`, `inside & 4'hF:` and
/// `inside(3):` are also plain `case` labels reading that name. iverilog 13.0 `-g2005`
/// prints `A m1=1 B m2=1 C m3=1` (F1A), `G m2=1` (F1D) and `up m=1` (F2A: the function is
/// found by the upward name search from a child instance); under `begin_keywords
/// "1364-2005"` iverilog, sv2v → iverilog and verilator 5.052 print `A m1=1` (F1E). Which
/// object the name binds is a scope question (upward search, loop variables, imports), so
/// every `case … inside` in such a design is refused, at its `case` keyword, naming the
/// first use. F2B (a `for` loop variable `inside`) has no oracle: sv2v and verilator
/// reject the declaration, and iverilog `-g2005` the `int` loop variable; PRE printed
/// `forvar m=1` (the plain label).
#[test]
fn f1_name_inside_in_the_design_declines_every_case_inside() {
    let r = declined(F1A, 3, "t.sv:4");
    assert!(r.err.contains("t.sv:7:15:"), "{}", r.err);
    declined(F1D, 1, "t.sv:5");
    declined(F1E, 1, "t.sv:5");
    declined(F2A, 1, "t.sv:8");
    declined(F2B, 1, "t.sv:7");
}

/// An escaped identifier `\inside` is the name `inside` (IEEE 1800-2017 §5.6.1) and never
/// the keyword: `case (x) \inside [2:1]:` is a plain case reading it (ESC1: PRE, sv2v →
/// iverilog and verilator print `esc m1=1`), and it counts as a name use, so a
/// `case … inside` beside it is declined (ESC2: sv2v → iverilog and verilator print
/// `esc m1=1 m2=1`; loud, as PRE was).
#[test]
fn escaped_inside_is_a_name() {
    let r = run(ESC1);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), ["esc m1=1"]);
    declined(ESC2, 1, "t.sv:4");
}

/// The record is per parsed unit and the decline design-wide: `inside` declared in one
/// file (a package parameter), the `case … inside` in another that imports it — refused
/// one-shot and through vcmp → velab (sv2v and verilator reject the package's
/// declaration). The control: the same two files with the case inside only, and a design
/// using the `inside` operator beside a `case … inside` with no name `inside`, run.
#[test]
fn name_use_in_another_file_declines_one_shot_and_staged() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_case_inside_x_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("a.sv"), XA).unwrap();
    std::fs::write(d.join("b.sv"), XB).unwrap();
    let vita = |args: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(args)
            .current_dir(&d)
            .output()
            .expect("run vita");
        Run {
            out: String::from_utf8_lossy(&o.stdout).into_owned(),
            err: String::from_utf8_lossy(&o.stderr).into_owned(),
            code: o.status.code().unwrap_or(-1),
        }
    };
    declined_run(&vita(&["a.sv", "b.sv"]), 1, "a.sv:2");
    let c = vita(&["vcmp", "-o", "t.vu", "a.sv", "b.sv"]);
    assert_eq!(c.code, 0, "stderr:\n{}", c.err);
    declined_run(&vita(&["velab", "-o", "t.velab", "t.vu"]), 1, "a.sv:2");
    let _ = std::fs::remove_dir_all(&d);
    let r = run(CTL);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    // sv2v → iverilog and verilator 5.052 print exactly these lines
    assert_eq!(
        r.lines(),
        [
            "v=1000 m=1 k=1",
            "v=0010 m=2 k=1",
            "v=0110 m=0 k=0",
            "v=1100 m=1 k=1"
        ]
    );
}

/// F1: `inside:` and `inside, 4'd9:` are whole labels and `casez` / `casex` never take
/// the word, so these run exactly as before (no `case … inside` here to decline). iverilog 13.0 `-g2005` (and PRE) print
/// exactly these lines.
#[test]
fn f1_inside_as_a_label_name_still_runs() {
    let r = run(F1B);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), ["D m1=1 E m2=1"]);
    let r = run(F1C);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), ["F m1=1 m2=1"]);
}

/// F1, the narrow parser rule's cost: after `case (…)` the word followed by an operator
/// that cannot begin an element (`inside == 4'd5:`) is still taken as `inside`, and the
/// element does not parse. Loud where PRE and iverilog 13.0 `-g2005` print `H m1=1`.
#[test]
fn f1_binary_operator_after_the_word_is_a_parse_error() {
    let r = run(F1F);
    assert_eq!(r.code, 1, "stderr:\n{}", r.err);
    assert!(
        r.errors().iter().all(|l| l.contains("error[VITA-E2002]")),
        "{}",
        r.err
    );
    assert!(r.lines().is_empty(), "stdout:\n{}", r.out);
}

/// Kept loud, unchanged: a generate-region `case … inside` (illegal by A.4.1.2; sv2v
/// and verilator reject it) stays the generate-case parse errors; a case inside in a
/// function called for a constant stays the const-fold refusal (a plain case there is
/// refused too; sv2v folds it to 1, verilator refuses); an unpacked-array item stays
/// the whole-array refusal (sv2v drops the array, verilator fails to compile).
#[test]
fn k01_kept_loud() {
    let r = run(K01G);
    assert_eq!(r.code, 1);
    assert!(
        r.errors().iter().all(|l| l.contains("VITA-E2002")),
        "{}",
        r.err
    );
    assert!(r.err.contains("generate-case item"), "{}", r.err);
    let r = run(K01C);
    assert_eq!(r.code, 1);
    assert_eq!(r.errors().len(), 1, "{}", r.err);
    assert!(r.err.contains("has no constant-fold arm"), "{}", r.err);
    let r = run(K01A);
    assert_eq!(r.code, 1);
    assert_eq!(r.errors().len(), 1, "{}", r.err);
    assert!(r.err.contains("whole unpacked array"), "{}", r.err);
}

/// A package function holding a case inside: called as `p::f()` it meets the existing
/// package-scoped-call refusal (its body walker reads the inert `$` placeholder as a
/// foreign name), and imported it runs. sv2v → iverilog and verilator print `f=1` for
/// the first and exactly the second's lines.
#[test]
fn l17_package_function() {
    let r = run(L17P);
    assert_eq!(r.code, 1);
    assert_eq!(r.errors().len(), 1, "{}", r.err);
    assert!(
        r.err.contains("package-scoped call `p::f(...)`"),
        "{}",
        r.err
    );
    let r = run(L17I);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "x=1000 f=1",
            "x=0010 f=2",
            "x=0110 f=0",
            "x=1100 f=1",
            "x=0011 f=2",
        ]
    );
}
