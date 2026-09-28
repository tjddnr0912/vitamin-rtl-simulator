//! §3 ⑤ⓛ: `'{default: v}` whose target is a whole PACKED variable or net (IEEE 1800-2017
//! §10.9.1). It was E3009 (`a keyed assignment pattern … is supported for a packed-struct
//! target … and as '{default: v} on an unpacked array`), which stopped the corpus row
//! `ibex` five times: `ibex_top`'s no-icache tie-offs (`assign ram_cfg_icache_tag_o =
//! '{default: prim_ram_1p_pkg::RAM_1P_CFG_RSP_DEFAULT};` on a packed array of a packed
//! struct, `assign icache_tag_alert = '{default:'b0};` on a vector) and `ibex_alu`'s
//! `assign imd_val_we_o = '{default: '0};`.
//!
//! Every element of the FIRST packed dimension takes `v` as an assignment to that
//! element: a bit of a vector takes `v`'s low bit, an element of a multi-dimensional or
//! struct array takes `v` sized to the element, sign-extended when `v` is signed. The
//! default never reaches an element's own dimensions or members. The value is the
//! replication `{N{W'(v)}}`, and the pattern is lowered as that replication. It is known
//! only when the first packed dimension is written in the target's own declaration; any
//! other target keeps the refusal (the last tests).
//!
//! Oracles: verilator 5.052 (`--binary --timing`) and sv2v 0.0.13 → iverilog 13.0
//! (`-g2012`). iverilog cannot parse a keyed pattern. verilator is 2-state, so on the `x`
//! and `z` cells sv2v → iverilog is the oracle, and sv2v turns `bit` into `logic`, so on
//! a 2-state target verilator is.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pdp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    std::fs::write(d.join("in.txt"), "ABCDEFGH\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

/// The design's own lines, without vita's warnings and end-of-run lines.
fn prints(src: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out
        .lines()
        .filter(|l| {
            !l.starts_with("warning[")
                && !l.starts_with("simulation ended")
                && !l.starts_with("errors=")
        })
        .collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

const KEYED: &str =
    "a keyed assignment pattern `'{k: v, …}` is supported for a packed-struct target";

// ───────────────────────────── values ─────────────────────────────

#[test]
fn the_ibex_tie_offs_run() {
    // `ibex_top` (`gen_norams`) and `ibex_alu` (`g_no_alu_rvb`), reduced, with a two-bit
    // response so the element width shows: a package struct array port, two vectors and
    // an output port, each filled from a generate branch.
    let src = r#"
package ram_pkg;
  parameter int unsigned RspWidth = 32'd2;
  typedef struct packed {
    logic [RspWidth-1:0] rsp;
  } cfg_rsp_t;
  parameter cfg_rsp_t CFG_RSP_DEFAULT = 2'b10;
endpackage
package core_pkg;
  parameter int unsigned NUM_WAYS = 2;
endpackage
module top #(parameter bit ICache = 1'b0) (
  output ram_pkg::cfg_rsp_t [core_pkg::NUM_WAYS-1:0] cfg_tag_o,
  output logic [1:0] alerts_o
);
  import core_pkg::*;
  logic [NUM_WAYS-1:0] tag_alert;
  logic [NUM_WAYS-1:0] data_alert;
  if (ICache) begin : gen_rams
    assign cfg_tag_o = 4'b1111;
    assign tag_alert = 2'b11;
    assign data_alert = 2'b11;
  end else begin : gen_norams
    assign cfg_tag_o  = '{default: ram_pkg::CFG_RSP_DEFAULT};
    assign tag_alert  = '{default:'b0};
    assign data_alert = '{default:'b1};
  end
  assign alerts_o = {|tag_alert, |data_alert};
endmodule
module alu #(parameter bit RVB = 1'b0) (output logic [1:0] imd_val_we_o);
  if (RVB) begin : g_alu_rvb
    assign imd_val_we_o = 2'b01;
  end else begin : g_no_alu_rvb
    assign imd_val_we_o = '{default: '0};
  end
endmodule
module t;
  ram_pkg::cfg_rsp_t [1:0] cfg;
  logic [1:0] alerts, we;
  top u_top (.cfg_tag_o(cfg), .alerts_o(alerts));
  alu u_alu (.imd_val_we_o(we));
  initial begin
    #1 $display("IBX %b %b %b", cfg, alerts, we);
    $finish;
  end
endmodule
"#;
    prints(src, &["IBX 1010 01 00"]);
}

#[test]
fn a_vector_bit_takes_the_low_bit_of_v() {
    // Each bit is an assignment of `v` to one bit: `2`, `2'b10`, `4'hA` give 0 and
    // `2'b01`, `-1`, `8'hFF` give 1, on a non-zero, an ascending and a signed range too.
    let src = r#"
module t;
  logic s1; logic [2:0] s3;
  logic [3:0] c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12, c13, c14, c15, c16;
  logic [7:4] r1; logic [0:3] r2; logic signed [3:0] sg;
  wire [3:0] w1;
  assign c1 = '{default:'1};
  assign c2 = '{default:'0};
  assign c3 = '{default:1'b1};
  assign c4 = '{default:1};
  assign c5 = '{default:2};
  assign c6 = '{default:2'b10};
  assign c7 = '{default:2'b01};
  assign c8 = '{default:-1};
  assign c9 = '{default:'b0};
  assign c10 = '{default:'b1};
  assign c11 = '{default:'x};
  assign c12 = '{default:1'bz};
  assign c13 = '{default:s1};
  assign c14 = '{default:s3};
  assign c15 = '{default:4'hA};
  assign c16 = '{default:8'hFF};
  assign r1 = '{default:2'b01};
  assign r2 = '{default:1'b1};
  assign sg = '{default:1'b1};
  assign w1 = '{default:'1};
  initial begin
    s1 = 0; s3 = 3'b010;
    #1 $display("A1 %b A2 %b A3 %b A4 %b A5 %b A6 %b A7 %b A8 %b", c1, c2, c3, c4, c5, c6, c7, c8);
    $display("A9 %b A10 %b A11 %b A12 %b A13 %b A14 %b A15 %b A16 %b", c9, c10, c11, c12, c13, c14, c15, c16);
    $display("R1 %b R2 %b SG %0d W1 %b", r1, r2, sg, w1);
    s1 = 1; s3 = 3'b101;
    #1 $display("A13 %b A14 %b", c13, c14);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        &[
            "A1 1111 A2 0000 A3 1111 A4 1111 A5 0000 A6 0000 A7 1111 A8 1111",
            "A9 0000 A10 1111 A11 xxxx A12 zzzz A13 0000 A14 0000 A15 0000 A16 1111",
            "R1 1111 R2 1111 SG -1 W1 1111",
            "A13 1111 A14 1111",
        ],
    );
}

#[test]
fn a_two_state_target_reads_x_as_zero() {
    // `bit` is 2-state (§6.11.3). verilator prints `0000 0000`; sv2v turns `bit` into
    // `logic` and prints `xxxx xxxx`, so it is not the oracle here.
    let src = r#"
module t;
  bit [3:0] b1, b2;
  always_comb b1 = '{default:'x};
  always_comb b2 = '{default:1'bx};
  initial begin
    #1 $display("B %b %b", b1, b2);
    $finish;
  end
endmodule
"#;
    prints(src, &["B 0000 0000"]);
}

#[test]
fn a_first_dimension_element_takes_v_whole() {
    // The element of `logic [1:0][3:0]` is four bits and takes `v` sized to four bits;
    // the element of a struct array is the struct, which takes `v` whole (`1` on
    // `{logic a; logic [1:0] b;}` is `001`, not `a=1, b=01`). A typedef element is
    // whole too.
    let src = r#"
module t;
  typedef struct packed { logic [1:0] rsp; } rsp2_t;
  typedef struct packed { logic a; logic [1:0] b; } ab_t;
  typedef logic [3:0] nib_t;
  localparam rsp2_t RSP_DEF = 2'b10;
  localparam ab_t AB_DEF = 3'b110;
  logic s1; logic [2:0] s3;
  logic [1:0][3:0] m1, m2, m3, m4, m5, m6, m7;
  logic [2:0][1:0][1:0] m8, m9;
  rsp2_t [2:0] ra1, ra2, ra3, ra4, ra5;
  ab_t [1:0] ab1, ab2, ab3, ab4, ab5;
  nib_t [1:0] nb1, nb2;
  assign m1 = '{default:4'hA};
  assign m2 = '{default:1};
  assign m3 = '{default:4'h5};
  assign m4 = '{default:'1};
  assign m5 = '{default:2'b10};
  assign m6 = '{default:s3};
  assign m7 = '{default:8'h3C};
  assign m8 = '{default:2'b10};
  assign m9 = '{default:1};
  assign ra1 = '{default:RSP_DEF};
  assign ra2 = '{default:'0};
  assign ra3 = '{default:2'b10};
  assign ra4 = '{default:1};
  assign ra5 = '{default:s1 + s1};
  assign ab1 = '{default:1};
  assign ab2 = '{default:3'b101};
  assign ab3 = '{default:'1};
  assign ab4 = '{default:AB_DEF};
  assign ab5 = '{default:2};
  assign nb1 = '{default:4'hA};
  assign nb2 = '{default:1};
  initial begin
    s1 = 1; s3 = 3'b110;
    #1 $display("B1 %h B2 %h B3 %h B4 %h B5 %h B6 %h B7 %h B8 %b B9 %b", m1, m2, m3, m4, m5, m6, m7, m8, m9);
    $display("C1 %b C2 %b C3 %b C4 %b C5 %b", ra1, ra2, ra3, ra4, ra5);
    $display("D1 %b D2 %b D3 %b D4 %b D5 %b", ab1, ab2, ab3, ab4, ab5);
    $display("N1 %h N2 %h", nb1, nb2);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        &[
            "B1 aa B2 11 B3 55 B4 ff B5 22 B6 66 B7 cc B8 001000100010 B9 000100010001",
            "C1 101010 C2 000000 C3 101010 C4 010101 C5 101010",
            "D1 001001 D2 101101 D3 111111 D4 110110 D5 010010",
            "N1 aa N2 11",
        ],
    );
}

#[test]
fn procedural_and_declaration_forms_fill_the_same_way() {
    // `always_comb` (re-run when `s1`/`s3` change), blocking, non-blocking, a variable
    // and a net declaration initializer; a signed `v` sign-extends into its element, an
    // unsigned `1'bx` zero-extends.
    let src = r#"
module t;
  typedef struct packed { logic a; logic [1:0] b; } ab_t;
  localparam ab_t AB_DEF = 3'b110;
  logic s1; logic [2:0] s3;
  logic [3:0] p1, p2, p3;
  logic [1:0][3:0] p4, p5;
  ab_t [1:0] p6;
  logic [3:0] d1 = '{default:'1};
  logic [1:0][3:0] d2 = '{default:4'h9};
  ab_t [1:0] d3 = '{default:AB_DEF};
  wire [3:0] wd = '{default:s1};
  logic [1:0][3:0] ms1, ms2, ms3, ms4;
  logic signed [1:0][3:0] ms5;
  assign ms1 = '{default:2'sb10};
  assign ms2 = '{default:1'b1};
  assign ms3 = '{default:1'bx};
  assign ms4 = '{default:'x};
  assign ms5 = '{default:-1};
  always_comb p1 = '{default:s1};
  always_comb p4 = '{default:s3};
  initial begin
    s1 = 1; s3 = 3'b110;
    p2 = '{default:2'b01};
    p5 = '{default:4'hB};
    p6 = '{default:1};
    p3 <= '{default:'1};
    #1 $display("P1 %b P2 %b P3 %b P4 %h P5 %h P6 %b", p1, p2, p3, p4, p5, p6);
    $display("D1 %b D2 %h D3 %b WD %b", d1, d2, d3, wd);
    $display("MS1 %h MS2 %h MS3 %b MS4 %b MS5 %h", ms1, ms2, ms3, ms4, ms5);
    s1 = 0; s3 = 3'b011;
    #1 $display("P1 %b P4 %h WD %b", p1, p4, wd);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        &[
            "P1 1111 P2 1111 P3 1111 P4 66 P5 bb P6 001001",
            "D1 1111 D2 99 D3 110110 WD 1111",
            "MS1 ee MS2 11 MS3 000x000x MS4 xxxxxxxx MS5 ff",
            "P1 0000 P4 33 WD 0000",
        ],
    );
}

#[test]
fn force_procedural_assign_and_delays() {
    // `force` / `assign` (procedural continuous), an intra-assignment delay, a delayed
    // continuous assign and an `always @*`.
    let src = r#"
module t;
  logic [3:0] p, q, fd, ic; logic s1;
  assign #1 fd = '{default: s1};
  always @* ic = '{default: s1};
  initial begin
    s1 = 1;
    force p = '{default: s1};
    assign q = '{default: s1};
    #2 $display("F %b %b %b %b", p, q, fd, ic);
    release p; deassign q;
    p = '{default: 1'b0};
    q = #2 '{default: 1'b1};
    s1 = 0;
    #3 $display("F %b %b %b %b", p, q, fd, ic);
    $finish;
  end
endmodule
"#;
    prints(src, &["F 1111 1111 1111 1111", "F 0000 1111 0000 0000"]);
}

#[test]
fn other_element_and_value_spellings() {
    // A string literal (`"A"` is 8'h41, low bit 1), an enum element (`B` = 01), an
    // ascending first dimension, a negative low bound, a continuation port and an
    // interface member.
    let src = r#"
interface bus_if; logic [3:0] data; endinterface
module sub(output logic [1:0] a, b, bus_if bi);
  assign a = '{default: 1};
  assign b = '{default: 1};
  assign bi.data = '{default: 1};
endmodule
module t;
  typedef enum logic [1:0] {A, B, C} e_t;
  logic [3:0] r2; e_t [1:0] ea;
  logic [0:1][3:0] ma;
  logic [3:-2] ng;
  wire [1:0] a, b;
  bus_if bi();
  sub u(.a(a), .b(b), .bi(bi));
  assign r2 = '{default: "A"};
  assign ea = '{default: B};
  assign ma = '{default: 4'hA};
  assign ng = '{default: 1'b1};
  initial begin
    #1 $display("S %b %b %h %b %b %b %b", r2, ea, ma, ng, a, b, bi.data);
    $finish;
  end
endmodule
"#;
    prints(src, &["S 1111 0101 aa 111111 11 11 1111"]);
}

#[test]
fn constants_formals_and_casts_fill_the_same_way() {
    // A 72-bit literal and variable (low bits of each element), an untyped string-valued
    // localparam (`"AB"`: `0x42` has low nibble 2 and low bit 0), an enum label, a signed
    // and an unsigned localparam, `signed'(…)`, a select plus a constant, and an input
    // formal of an automatic task and of a void function.
    let src = r#"
module t;
  typedef enum logic [2:0] {E0, E5 = 5} e_t;
  localparam S = "AB";
  localparam signed [2:0] SP = -2;
  parameter P = 9;
  logic [71:0] big = 72'h80_0000_0000_0000_0003;
  logic [1:0][3:0] v1, v2, v3, v4, v5, v6, v8, h, x;
  logic [7:0] w1, w2;
  logic [1:0][39:0] v7;
  logic [2:0][1:0] y;
  logic [1:0][7:0] s;
  task automatic tk(input logic [1:0] a); y = '{default: a}; endtask
  function void setx(input logic [3:0] a); x = '{default: a}; endfunction
  assign v1 = '{default: 72'h80_0000_0000_0000_0002};
  assign v2 = '{default: big};
  assign v3 = '{default: S};
  assign v4 = '{default: E5};
  assign v5 = '{default: SP};
  assign v6 = '{default: P};
  assign w1 = '{default: big};
  assign w2 = '{default: S};
  assign v7 = '{default: big};
  assign v8 = '{default: signed'(2'b10)};
  initial begin
    tk(2'b10);
    setx(4'h3);
    s = 16'h1234;
    h = '{default: s[0][3:0] + 4'd1};
    #1 $display("V %h %h %h %h %h %h %h | %b %b | %h", v1, v2, v3, v4, v5, v6, v8, w1, w2, v7);
    $display("F %h %h %h", y, x, h);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        &[
            "V 22 33 22 55 ee 99 ee | 11111111 00000000 | 00000000030000000003",
            "F 2a 33 55",
        ],
    );
}

#[test]
fn wide_elements() {
    // A 128-bit vector, 100-bit and 70-bit elements, and a signed 70-bit `v`.
    let src = r#"
module t;
  logic s1;
  logic [127:0] big;
  logic [1:0][99:0] wd;
  logic [69:0] w70;
  logic [2:0][69:0] m70;
  assign big = '{default: 1'b1};
  assign wd = '{default: 100'h1_0000_0000_0000_0003};
  assign w70 = '{default: s1};
  assign m70 = '{default: 70'sh20_0000_0000_0000_0001};
  initial begin
    s1 = 1;
    #1 $display("W1 %h", big);
    $display("W2 %h", wd);
    $display("W3 %h %h", w70, m70);
    s1 = 0;
    #1 $display("W4 %h", w70);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        &[
            "W1 ffffffffffffffffffffffffffffffff",
            "W2 00000000100000000000000030000000010000000000000003",
            "W3 3fffffffffffffffff 20000000000000000180000000000000000600000000000000001",
            "W4 000000000000000000",
        ],
    );
}

// ───────────────────────────── refusals ─────────────────────────────

fn module(body: &str) -> String {
    format!("module t;\n{body}\n  initial #1 $finish;\nendmodule\n")
}

#[test]
fn a_target_whose_first_dimension_is_not_written_stays_refused() {
    // A typedef supplies the first dimension (verilator: `1111`), and targets that are
    // not a whole packed net: a part-select (`1100`), a concatenation (`11 11`), an
    // unpacked element (`1111`), `integer` / `int` (`ffffffff`), a task output formal
    // and a function local (`1111`).
    for body in [
        "  typedef logic [3:0] nib_t;\n  nib_t x;\n  assign x = '{default: 1};",
        "  logic [3:0] v;\n  assign v[3:2] = '{default: 1};",
        "  logic [1:0] a, b;\n  assign {a, b} = '{default: 1};",
        "  logic [3:0] arr [2];\n  initial arr[1] = '{default: 1};",
        "  integer i;\n  initial i = '{default: 1};",
        "  int j;\n  initial j = '{default: 1};",
        "  logic [3:0] r;\n  task automatic tk(output logic [3:0] o); o = '{default: 1}; endtask\n  initial tk(r);",
        "  function automatic logic [3:0] f(input logic s);\n    logic [3:0] r;\n    r = '{default: s};\n    return r;\n  endfunction\n  logic [3:0] y;\n  initial y = f(1'b1);",
    ] {
        is_loud(&module(body), KEYED);
    }
}

#[test]
fn a_value_the_replication_cannot_carry_stays_refused() {
    // A call, whose evaluation count §10.9.1 leaves open (verilator `1111`), a real
    // literal and a real variable (verilator and sv2v → iverilog `0000`: 1.6 rounds to 2),
    // a `string` variable, array element or queue element (verilator: internal error; sv2v
    // → iverilog: "cannot be implicitly cast"), a nested `'{default:}` (`ff`), a nested
    // positional struct pattern (`101101`) and a hierarchical reference, whose net does
    // not exist yet (a real one: both oracles `00110011`; a vector: `01010101`).
    let sub = "module sub; real rv = 2.6; logic [2:0] lv = 3'b101; endmodule\n";
    for body in [
        "  logic [3:0] v;\n  function automatic logic g(); return 1'b1; endfunction\n  initial v = '{default: g()};",
        "  logic [3:0] v;\n  assign v = '{default: 1.6};",
        "  real rv; logic [3:0] v;\n  initial begin rv = 1.6; v = '{default: rv}; end",
        "  string s = \"AB\";\n  logic [1:0][3:0] x;\n  initial x = '{default: s};",
        "  string sa [2];\n  logic [1:0][3:0] x;\n  initial begin sa[0] = \"AB\"; x = '{default: sa[0]}; end",
        "  string q [$];\n  logic [1:0][3:0] x;\n  initial begin q.push_back(\"AB\"); x = '{default: q[0]}; end",
        "  logic [1:0][3:0] m;\n  initial m = '{default: '{default: 1}};",
        "  typedef struct packed { logic a; logic [1:0] b; } ab_t;\n  ab_t [1:0] x;\n  assign x = '{default: '{1, 2'b01}};",
    ] {
        is_loud(&module(body), KEYED);
    }
    for body in [
        "  sub u();\n  logic [1:0][3:0] a;\n  initial #1 a = '{default: u.rv};",
        "  sub u();\n  logic [1:0][3:0] a;\n  initial #1 a = '{default: u.lv};",
    ] {
        is_loud(&format!("{sub}{}", module(body)), KEYED);
    }
}

#[test]
fn a_class_handle_value_stays_refused() {
    // The element assignment `e = h` is refused (IEEE §8.4), and so are `4'(h)` in
    // verilator ("Size-changing cast on non-basic data type") and iverilog ("Cast base
    // expression must be a vector type"); verilator's pattern build fails internally.
    let src = r#"
class C; int v; endclass
module t;
  logic [1:0][3:0] c;
  C h;
  initial begin
    h = new;
    c = '{default: h};
  end
endmodule
"#;
    is_loud(src, KEYED);
}

#[test]
fn a_call_a_statement_hoist_moved_stays_refused() {
    // The frame-call and system-call hoists move a call out of `v` into a temporary before
    // the statement lowers; `v` would then be evaluated once. The oracles split on the
    // count: an automatic function counting its calls gives verilator `x=6c cnt=4` and
    // sv2v → iverilog `x=55 cnt=1`; a static one and one with an output formal give `10 2`
    // against `11 1`; `$fgetc` gives `41424344` against `41414141`.
    for body in [
        "  int cnt = 0;\n  logic [3:0][1:0] x;\n  function automatic logic [1:0] f();\n    cnt = cnt + 1;\n    return cnt[1:0];\n  endfunction\n  initial x = '{default: f()};",
        "  int cnt = 0;\n  logic [1:0] y;\n  function logic g(); cnt = cnt + 1; return cnt[0]; endfunction\n  initial y = '{default: g()};",
        "  int cnt = 0, oo;\n  logic [1:0] y;\n  function automatic logic g(output int o); cnt = cnt + 1; o = cnt; return cnt[0]; endfunction\n  initial y = '{default: g(oo)};",
        "  integer fd; logic [3:0][7:0] x;\n  initial begin\n    fd = $fopen(\"in.txt\", \"r\");\n    x = '{default: $fgetc(fd)};\n  end",
    ] {
        is_loud(&module(body), KEYED);
    }
}

#[test]
fn sibling_block_locals_of_one_name_keep_the_refusal() {
    // v1 flattens two block-locals of one name and width onto one net, so the first
    // declaration's shape would fill the second's pattern: both oracles print `S1A x=11`,
    // `S1B x=ff` and, for a typedef declaration (`v8_t x`), `S1C x=ff`, where
    // `{2{4'(1'b1)}}` would give `11`.
    let src = r#"
module t;
  typedef logic [7:0] v8_t;
  initial begin
    begin
      logic [1:0][3:0] x;
      x = '0;
      x = '{default: 4'd1};
      $display("S1A x=%h", x);
    end
    begin
      logic [7:0] x;
      x = '0;
      x = '{default: 1'b1};
      $display("S1B x=%h", x);
    end
    begin
      v8_t x;
      x = '0;
      x = '{default: 1'b1};
      $display("S1C x=%h", x);
    end
  end
endmodule
"#;
    is_loud(src, KEYED);
}

#[test]
fn a_local_unpacked_struct_keeps_its_member_pattern() {
    // A block-local unpacked struct is declared as a flat vector whose range the parser
    // makes up; `'{default: v}` on it fills MEMBERS (the parser's struct pattern), not
    // bits. Both oracles print `S3 a=1 b=1`.
    let src = r#"
module t;
  typedef struct { logic [3:0] a; logic [3:0] b; } rec_t;
  initial begin
    rec_t p;
    p = '{default: 4'd1};
    $display("S3 a=%h b=%h", p.a, p.b);
  end
endmodule
"#;
    prints(src, &["S3 a=1 b=1"]);
}

#[test]
fn a_parameter_value_stays_refused() {
    // verilator and sv2v → iverilog print `1111 77`; a parameter's value is folded by the
    // constant evaluator, which has no arm for the pattern.
    is_loud(
        &module("  localparam logic [3:0] LP1 = '{default:1};\n  initial $display(\"%b\", LP1);"),
        "has no constant-fold arm",
    );
}
