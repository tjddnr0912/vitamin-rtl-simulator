//! §3 ⑤ⓘ: a whole fixed-size unpacked array as the target of a continuous `assign`
//! (IEEE 1800-2017 §10.3 with §7.6 and §10.9.1). Every such `assign` was E3009 (`a whole
//! unpacked array cannot be the write target in this context`), which stopped the corpus
//! row `ibex` 20 times: its tie-offs `assign ic_tag_rdata = '{default:'b0};`, its lint
//! sinks `assign unused_csr_pmp_addr = csr_pmp_addr;`, an output port driven from a
//! register array (`assign imd_val_q_ex_o = imd_val_q;`) and a positional pattern
//! (`assign alu_imd_val_q = '{imd_val_q_i[0][31:0], imd_val_q_i[1][31:0]};`).
//!
//! Three right-hand sides are lowered now, one continuous assign per element: another
//! whole array of the same shape and element type, a positional pattern, and
//! `'{default: v}`. They share the procedural array assignment's rules (position
//! correspondence, pattern flattening, the §7.6 element-type rule), and each element is
//! sized by the scalar `assign`'s own rule. The lowering is kept only when the `assign`
//! is the array's only writer; with another writer it stays loud (the last tests).
//!
//! Oracles: verilator 5.052 (`--binary --timing`) and iverilog 13.0 (`-g2012`), and
//! sv2v 0.0.13 → iverilog where iverilog cannot parse a keyed pattern. sv2v lowers a
//! positional pattern into a self-determined concatenation (`{x, y + 8'd1, 9'h007}`) and
//! a nested `'{default: …}` into `{2 {12'd9}}`, so on those cells verilator (and iverilog
//! for the positional ones) are the oracles, and each test says which ran.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cawa_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    std::fs::write(d.join("mem.hex"), "11\n22\n").unwrap();
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

/// The design's own lines that start with `tag`, without vita's warnings and end-of-run
/// lines.
fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out
        .lines()
        .filter(|l| {
            l.starts_with(tag)
                && !l.starts_with("warning[")
                && !l.starts_with("simulation ended")
                && !l.starts_with("errors=")
        })
        .collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
    out
}

const SOLE: &str = "a whole unpacked array driven by a continuous `assign` has another writer";

// ───────────────────────────── values ─────────────────────────────

#[test]
fn the_ibex_tie_offs_sinks_and_pattern_run() {
    // `ibex_alu` (`g_no_alu_rvb`), `ibex_ex_block`, `ibex_id_stage`, `ibex_if_stage`
    // (`gen_prefetch_buffer`) and `ibex_top` (`gen_norams`), reduced: each `assign` of a
    // whole array is the array's only writer. verilator and sv2v → iverilog print these
    // three lines; iverilog cannot parse `'{default: …}`.
    let src = r#"
module alu(input logic [31:0] imd_val_q_i [2], output logic [31:0] imd_val_d_o [2],
           output logic [1:0] imd_val_we_o, output logic [31:0] sum_o);
  logic [31:0] unused_imd_val_q [2];
  assign unused_imd_val_q = imd_val_q_i;
  assign imd_val_d_o = '{default: '0};
  assign imd_val_we_o = 2'b00;
  assign sum_o = unused_imd_val_q[0] + unused_imd_val_q[1];
endmodule
module ex_block(input logic [33:0] imd_val_q_i [2], output logic [31:0] alu_d0, output logic [31:0] sum_o);
  logic [31:0] alu_imd_val_q [2];
  logic [31:0] alu_imd_val_d [2];
  logic [1:0]  alu_imd_val_we;
  assign alu_imd_val_q = '{imd_val_q_i[0][31:0], imd_val_q_i[1][31:0]};
  alu alu_i(.imd_val_q_i(alu_imd_val_q), .imd_val_d_o(alu_imd_val_d), .imd_val_we_o(alu_imd_val_we), .sum_o(sum_o));
  assign alu_d0 = alu_imd_val_d[0] | {30'b0, alu_imd_val_we};
endmodule
module id_stage(input logic clk_i, input logic rst_ni, input logic [1:0] imd_val_we_ex_i,
                input logic [33:0] imd_val_d_ex_i [2], output logic [33:0] imd_val_q_ex_o [2]);
  logic [33:0] imd_val_q [2];
  for (genvar i = 0; i < 2; i++) begin : gen_intermediate_val_reg
    always_ff @(posedge clk_i or negedge rst_ni) begin
      if (!rst_ni) imd_val_q[i] <= '0;
      else if (imd_val_we_ex_i[i]) imd_val_q[i] <= imd_val_d_ex_i[i];
    end
  end
  assign imd_val_q_ex_o = imd_val_q;
endmodule
module if_stage(input logic [21:0] ic_tag_rdata_i [2], output logic [21:0] seen_o);
  logic [21:0] unused_tag_ram_input [2];
  assign unused_tag_ram_input = ic_tag_rdata_i;
  assign seen_o = unused_tag_ram_input[0] ^ unused_tag_ram_input[1];
endmodule
module t;
  logic clk = 1'b0, rst_n = 1'b1;
  logic [1:0] we;
  logic [33:0] d [2];
  logic [33:0] q [2];
  logic [21:0] ic_tag_rdata [2];
  logic [21:0] seen;
  logic [31:0] alu_d0, sum;
  if (1) begin : gen_norams
    assign ic_tag_rdata = '{default: 'b0};
  end
  id_stage id(.clk_i(clk), .rst_ni(rst_n), .imd_val_we_ex_i(we), .imd_val_d_ex_i(d), .imd_val_q_ex_o(q));
  ex_block ex(.imd_val_q_i(q), .alu_d0(alu_d0), .sum_o(sum));
  if_stage ifs(.ic_tag_rdata_i(ic_tag_rdata), .seen_o(seen));
  always #5 clk = ~clk;
  initial begin
    we = 2'b00; d[0] = 34'h3_0000_0011; d[1] = 34'h2_0000_0022;
    #1 rst_n = 1'b0;
    #3 rst_n = 1'b1;
    #1 $display("IBX t5 q=%h %h sum=%h alu_d0=%h seen=%h", q[0], q[1], sum, alu_d0, seen);
    we = 2'b11;
    #10 $display("IBX t15 q=%h %h sum=%h alu_d0=%h seen=%h", q[0], q[1], sum, alu_d0, seen);
    we = 2'b01; d[0] = 34'h1_ffff_ffff;
    #10 $display("IBX t25 q=%h %h sum=%h alu_d0=%h seen=%h", q[0], q[1], sum, alu_d0, seen);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "IBX",
        &[
            "IBX t5 q=000000000 000000000 sum=00000000 alu_d0=00000000 seen=000000",
            "IBX t15 q=300000011 200000022 sum=00000033 alu_d0=00000000 seen=000000",
            "IBX t25 q=1ffffffff 200000022 sum=00000021 alu_d0=00000000 seen=000000",
        ],
    );
}

#[test]
fn a_copy_follows_its_source_into_a_variable_and_a_net() {
    // All three oracles.
    let src = r#"
module t;
  logic [7:0] src [4];
  logic [7:0] dst [4];
  wire  [7:0] wd  [4];
  assign dst = src;
  assign wd = src;
  initial begin
    src[0] = 8'h11; src[1] = 8'h22; src[2] = 8'h33; src[3] = 8'h44;
    #1 $display("A1 t1 %h %h %h %h | %h %h %h %h", dst[0], dst[1], dst[2], dst[3], wd[0], wd[1], wd[2], wd[3]);
    src[2] = 8'hAA;
    #1 $display("A1 t2 %h %h %h %h | %h %h %h %h", dst[0], dst[1], dst[2], dst[3], wd[0], wd[1], wd[2], wd[3]);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "A1",
        &[
            "A1 t1 11 22 33 44 | 11 22 33 44",
            "A1 t2 11 22 aa 44 | 11 22 aa 44",
        ],
    );
}

#[test]
fn elements_pair_by_position_whatever_the_ranges() {
    // §7.6: the leftmost element of the source goes to the leftmost of the target, for a
    // copy and for a positional pattern alike. All three oracles.
    let src = r#"
module t;
  logic [7:0] a [0:3];
  logic [7:0] b [3:0];
  logic [7:0] c [1:4];
  logic [7:0] d [8:5];
  logic [7:0] p [3:0];
  logic [7:0] o [5:8];
  assign a = b;
  assign c = b;
  assign d = a;
  assign p = '{8'd1, 8'd2, 8'd3, 8'd4};
  assign o = '{8'd1, 8'd2, 8'd3, 8'd4};
  initial begin
    b[3] = 8'd1; b[2] = 8'd2; b[1] = 8'd3; b[0] = 8'd4;
    #1 $display("A2 a %0d %0d %0d %0d c %0d %0d %0d %0d d %0d %0d %0d %0d", a[0], a[1], a[2], a[3], c[1], c[2], c[3], c[4], d[8], d[7], d[6], d[5]);
    $display("A2 p %0d %0d %0d %0d o %0d %0d %0d %0d", p[3], p[2], p[1], p[0], o[5], o[6], o[7], o[8]);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "A2",
        &["A2 a 1 2 3 4 c 1 2 3 4 d 1 2 3 4", "A2 p 1 2 3 4 o 1 2 3 4"],
    );
}

#[test]
fn a_multi_dimensional_array_copies_and_fills() {
    // The copy and the nested positional pattern: all three oracles. The nested
    // `'{default: …}`: verilator (sv2v lowers it to `{2 {12'd9}}` and reads `0 9 0 9`).
    let src = r#"
module t;
  logic [3:0] m [2][3];
  logic [3:0] n [1:0][0:2];
  logic [3:0] k [2][3];
  logic [5:0] f [2][2];
  logic [3:0] b [2][3];
  logic [3:0] c [2][3];
  assign m = '{'{4'd1, 4'd2, 4'd3}, '{4'd4, 4'd5, 4'd6}};
  assign n = '{'{4'd1, 4'd2, 4'd3}, '{4'd4, 4'd5, 4'd6}};
  assign k = n;
  assign f = '{default: 6'd9};
  assign c = b;
  initial begin
    for (int i = 0; i < 2; i++) for (int j = 0; j < 3; j++) b[i][j] = 4'(i*3 + j + 1);
    #1 $display("A3 m %0d %0d %0d %0d %0d %0d n %0d %0d %0d %0d %0d %0d k %0d %0d %0d %0d %0d %0d", m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], n[1][0], n[1][1], n[1][2], n[0][0], n[0][1], n[0][2], k[0][0], k[0][1], k[0][2], k[1][0], k[1][1], k[1][2]);
    $display("A3 f %0d %0d %0d %0d c %0d %0d %0d %0d %0d %0d", f[0][0], f[0][1], f[1][0], f[1][1], c[0][0], c[0][1], c[0][2], c[1][0], c[1][1], c[1][2]);
    b[1][2] = 4'd15;
    #1 $display("A3 c %0d", c[1][2]);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "A3",
        &[
            "A3 m 1 2 3 4 5 6 n 1 2 3 4 5 6 k 1 2 3 4 5 6",
            "A3 f 9 9 9 9 c 1 2 3 4 5 6",
            "A3 c 15",
        ],
    );
}

#[test]
fn each_default_element_is_sized_by_its_element_type() {
    // §10.9.1: `default`'s value is evaluated as an assignment to each element. A fill
    // grows to the element (`'1` → 7ff), a wider value truncates (a5 → 5), a signed
    // narrower one sign-extends (3'sb101 → fd), and a value that reads a signal follows
    // it. verilator and sv2v → iverilog.
    let src = r#"
module t;
  logic [10:0] z0 [2];
  logic [10:0] z1 [2];
  logic [7:0]  zx [3];
  logic [3:0]  zt [2];
  logic signed [7:0] zs [2];
  logic [7:0]  x;
  assign z0 = '{default:'b0};
  assign z1 = '{default:'1};
  assign zx = '{default: x};
  assign zt = '{default: 8'hA5};
  assign zs = '{default: 3'sb101};
  initial begin
    x = 8'h3c;
    #1 $display("B1 t1 %h %h | %h %h | %h %h %h | %h %h | %h %h", z0[0], z0[1], z1[0], z1[1], zx[0], zx[1], zx[2], zt[0], zt[1], zs[0], zs[1]);
    x = 8'hc3;
    #1 $display("B1 t2 %h %h %h", zx[0], zx[1], zx[2]);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "B1",
        &[
            "B1 t1 000 000 | 7ff 7ff | 3c 3c 3c | 5 5 | fd fd",
            "B1 t2 c3 c3 c3",
        ],
    );
}

#[test]
fn each_positional_item_is_sized_by_its_element_type() {
    // §10.9.1: every item is evaluated as an assignment to its element, so a sum keeps
    // its carry in a wider element (ff + 1 → 100) and `'1` fills nine bits. verilator
    // and iverilog; sv2v concatenates the items unsized (`{x, y + 8'd1, 9'h007}`).
    let src = r#"
module t;
  logic [7:0]  x, y;
  logic [8:0]  p [4];
  logic [31:0] q [2];
  logic [33:0] q_in [2];
  assign p = '{x, y + 8'd1, 8'd3, '1};
  assign q = '{q_in[0][31:0], q_in[1][31:0]};
  initial begin
    x = 8'h10; y = 8'hff; q_in[0] = 34'h3_1234_5678; q_in[1] = 34'h2_8765_4321;
    #1 $display("C1 t1 %h %h %h %h | %h %h", p[0], p[1], p[2], p[3], q[0], q[1]);
    x = 8'h20; y = 8'h01;
    #1 $display("C1 t2 %h %h %h %h", p[0], p[1], p[2], p[3]);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "C1",
        &[
            "C1 t1 010 100 003 1ff | 12345678 87654321",
            "C1 t2 020 002 003 1ff",
        ],
    );
}

#[test]
fn x_and_z_fill_each_element() {
    // iverilog, the 4-state oracle, prints this line. verilator refuses the `'z` items
    // (`Unsupported tristate construct`), and sv2v concatenates `'x` as one bit.
    let src = r#"
module t;
  logic [7:0] zx [2];
  logic [7:0] zz [2];
  logic [7:0] pz [3];
  logic [3:0] x4;
  assign zx = '{'x, 8'h0f};
  assign zz = '{'z, 'z};
  assign pz = '{x4, 'x, 4'bx01z};
  initial begin
    x4 = 4'b1x0z;
    #1 $display("E1 %b %b | %b %b | %b %b %b", zx[0], zx[1], zz[0], zz[1], pz[0], pz[1], pz[2]);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "E1",
        &["E1 xxxxxxxx 00001111 | zzzzzzzz zzzzzzzz | 00001x0z xxxxxxxx 0000x01z"],
    );
}

#[test]
fn an_element_wakes_its_readers_once_per_change() {
    // Two writes of the same source element in one step, and a rewrite of the value it
    // already holds. All three oracles.
    let src = r#"
module t;
  logic [7:0] b [2];
  logic [7:0] a [2];
  logic [8:0] y;
  int n1 = 0, n0 = 0;
  assign a = b;
  always_comb y = a[0] + a[1];
  always @(a[1]) n1++;
  always @(a[0]) n0++;
  initial begin
    b[0] = 8'd1; b[1] = 8'd2;
    #1 $display("E4 t1 y=%0d n0=%0d n1=%0d", y, n0, n1);
    b[1] = 8'd5; b[1] = 8'd6; b[0] = 8'd200;
    #1 $display("E4 t2 y=%0d n0=%0d n1=%0d", y, n0, n1);
    b[1] = 8'd6;
    #1 $display("E4 t3 y=%0d n0=%0d n1=%0d", y, n0, n1);
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "E4",
        &[
            "E4 t1 y=3 n0=1 n1=1",
            "E4 t2 y=206 n0=2 n1=2",
            "E4 t3 y=206 n0=2 n1=2",
        ],
    );
}

#[test]
fn a_multi_dimensional_packed_element_copies_and_fills() {
    // A 2-D packed element written in the declaration, filled with `'1` and copied.
    // verilator and sv2v → iverilog.
    let src = r#"
module t;
  logic [1:0][3:0] pa [2];
  logic [1:0][3:0] pb [2];
  assign pa = '{default: '1};
  assign pb = pa;
  initial begin
    #1 $display("S1 %h %h | %h %h", pa[0], pa[1], pb[0], pb[1]);
    $finish;
  end
endmodule
"#;
    prints(src, "S1", &["S1 ff ff | ff ff"]);
}

#[test]
fn generate_blocks_and_ports_chain_whole_array_assigns() {
    // An array local to each iteration of a generate loop, and a module whose input
    // array port feeds a sink array and whose output array port is a positional pattern,
    // instantiated twice in a chain. verilator; sv2v → iverilog agrees on everything but
    // the hierarchical read `l1.sink[…]`, which its flattening turns into bit selects.
    let src = r#"
module leaf(input logic [3:0] i [2], output logic [3:0] o [2]);
  logic [3:0] sink [2];
  assign sink = i;
  assign o = '{sink[1], sink[0]};
endmodule
module t;
  logic [3:0] src [2];
  logic [3:0] r0 [2];
  logic [3:0] r1 [2];
  for (genvar g = 0; g < 2; g++) begin : gl
    logic [3:0] loc [2];
    assign loc = '{default: 4'(g + 3)};
  end
  leaf l0(.i(src), .o(r0));
  leaf l1(.i(r0), .o(r1));
  initial begin
    src[0] = 4'd1; src[1] = 4'd2;
    #1 $display("E5 r0 %0d %0d r1 %0d %0d gl %0d %0d %0d %0d sink %0d %0d", r0[0], r0[1], r1[0], r1[1], gl[0].loc[0], gl[0].loc[1], gl[1].loc[0], gl[1].loc[1], l1.sink[0], l1.sink[1]);
    $finish;
  end
endmodule
"#;
    prints(src, "E5", &["E5 r0 2 1 r1 1 2 gl 3 3 4 4 sink 2 1"]);
}

#[test]
fn an_interface_member_array_takes_a_whole_array_assign() {
    // verilator and sv2v → iverilog (iverilog cannot parse an interface port).
    let src = r#"
interface bus;
  logic [7:0] arr [2];
  logic [7:0] src [2];
endinterface
module drv(bus b);
  assign b.arr = b.src;
endmodule
module t;
  bus ib();
  drv d(.b(ib));
  initial begin
    ib.src[0] = 8'h12; ib.src[1] = 8'h34;
    #1 $display("I1 %h %h", ib.arr[0], ib.arr[1]);
    $finish;
  end
endmodule
"#;
    prints(src, "I1", &["I1 12 34"]);
}

#[test]
fn reading_the_target_elsewhere_leaves_the_assign_its_only_writer() {
    // An input port, a function and a task input, `$countones`, `$isunknown` and
    // `$display` all read the array. All three oracles.
    let src = r#"
module chk(input logic [7:0] i [2], output logic [7:0] s);
  assign s = i[0] + i[1];
endmodule
module t;
  logic [7:0] b [2];
  logic [7:0] a [2];
  logic [7:0] s;
  assign a = b;
  function automatic logic [7:0] f(input logic [7:0] v); return v + 1; endfunction
  task automatic tk(input logic [7:0] q); $display("R1 tk %h", q); endtask
  chk c(.i(a), .s(s));
  initial begin
    b[0] = 8'h11; b[1] = 8'h22;
    #1 $display("R1 a=%h %h s=%h f=%h cnt=%0d unk=%0d", a[0], a[1], s, f(a[1]), $countones(a[0]), $isunknown(a[1]));
    tk(a[0]);
    $finish;
  end
endmodule
"#;
    prints(src, "R1", &["R1 a=11 22 s=33 f=23 cnt=2 unk=0", "R1 tk 11"]);
}

#[test]
fn a_streaming_item_is_sized_like_a_procedural_pattern_item() {
    // A stream that is an ITEM of a pattern is zero-extended into its element, as the
    // procedural pattern path does; only a stream that is the whole right-hand side of a
    // scalar `assign` is left-justified. verilator (iverilog: `sorry: Streaming
    // concatenation not supported`).
    let src = r#"
module t;
  logic [7:0] a [2];
  logic [7:0] p [2];
  logic [7:0] s;
  logic [3:0] n = 4'b0001;
  assign a = '{{<<{n}}, {>>{n}}};
  assign s = {<<{n}};
  initial begin p = '{{<<{n}}, {>>{n}}}; #1 $display("C28 %h %h | %h %h | %h", a[0], a[1], p[0], p[1], s); $finish; end
endmodule
"#;
    prints(src, "C28", &["C28 08 01 | 08 01 | 80"]);
}

#[test]
fn sformat_reading_an_element_leaves_the_assign_its_only_writer() {
    // `$sformat` writes its first argument only. verilator and iverilog.
    let src = r#"
module t;
  logic [7:0] src [2]; logic [7:0] dst [2]; assign dst = src;
  logic [15:0] s;
  initial begin src[0]=8'h1; src[1]=8'h2; #1 $sformat(s, "%h", dst[1]); $display("C57 %s %h", s, dst[0]); $finish; end
endmodule
"#;
    prints(src, "C57", &["C57 02 01"]);
}

// ───────────────────────────── loud ─────────────────────────────

/// A design whose array `a` is written by `assign a = b;` and by `other`.
fn with_other_writer(other: &str) -> String {
    format!(
        r#"
module leaf(output logic [7:0] o);
  assign o = 8'h99;
endmodule
module t;
  logic [7:0] b [2];
  logic [7:0] a [2];
  logic [7:0] c;
  wire  [7:0] w [2];
  int r;
  assign a = b;
{other}
  initial begin
    b[0] = 8'h11; b[1] = 8'h22; c = 8'h77;
    #2 $display("M a=%h %h w=%h %h", a[0], a[1], w[0], w[1]);
    $finish;
  end
endmodule
"#
    )
}

#[test]
fn another_writer_keeps_a_whole_array_assign_loud() {
    // With a second writer the element-wise lowering reaches what this IR does not model.
    // The procedural and continuous mixtures are refused by iverilog (`Cannot perform
    // procedural assignment to array word 'a['sd0]' because it is also continuously
    // assigned`, `Variable 'a' cannot have multiple drivers`) and run by verilator, and
    // the same shapes written element by element already run in vita — so the whole
    // spelling keeps its refusal rather than joining that split.
    for other in [
        "  initial #1 a[0] = 8'h55;",
        "  always_comb a[1] = c;",
        "  assign a[0] = c;",
        "  always @(b[0]) a[1] = c;",
        "  initial $readmemh(\"mem.hex\", a);",
        "  task automatic tk(output logic [7:0] q); q = 8'h66; endtask\n  initial #1 tk(a[1]);",
        "  task tk(output logic [7:0] q); q = 8'h66; endtask\n  initial #1 tk(a[1]);",
        "  function automatic int fo(output logic [7:0] q); q = 8'h67; return 1; endfunction\n  initial #1 r = fo(a[0]);",
        "  leaf u(.o(a[1]));",
        "  initial #1 a[1]++;",
        "  assign a = '{default: c};",
    ] {
        let src = with_other_writer(other);
        let out = is_loud(&src, SOLE);
        assert!(out.contains("t.sv:11:3"), "anchored at the `assign`\n{out}");
    }
}

#[test]
fn a_wire_array_with_two_drivers_stays_loud() {
    // iverilog resolves the doubly driven element to `XX` (sv2v → iverilog agrees) where
    // the last element driver would win here; verilator, a 2-state tool, prints `77`.
    let src = with_other_writer("  assign w = b;\n  assign w[0] = c;");
    is_loud(&src, SOLE);
}

#[test]
fn a_hierarchical_write_and_an_initializer_are_other_writers() {
    let hier = r#"
module t;
  logic [7:0] b [2];
  logic [7:0] a [2];
  assign a = b;
  sub s();
  initial begin
    b[0] = 8'h11; b[1] = 8'h22;
    #2 $display("H a=%h %h", a[0], a[1]);
    $finish;
  end
endmodule
module sub;
  initial #1 t.a[0] = 8'h44;
endmodule
"#;
    is_loud(hier, SOLE);
    // verilator: `%Error-CONTASSINIT … Continuous assignment to variable with initial
    // value: 'a'`.
    let init = r#"
module t;
  logic [7:0] b [2];
  logic [7:0] a [2] = '{8'h1, 8'h2};
  assign a = b;
  initial begin
    b[0] = 8'h11; b[1] = 8'h22;
    #2 $display("H a=%h %h", a[0], a[1]);
    $finish;
  end
endmodule
"#;
    is_loud(init, SOLE);
}

/// A design with one extra item `item` over a few arrays.
fn with_item(item: &str) -> String {
    format!(
        r#"
module t;
  logic [7:0] b [2];
  logic [7:0] a [2];
  logic [3:0] n4 [2];
  logic [7:0] b3 [3];
  logic [7:0] m [2][2];
  logic [7:0] row [2];
  real ra [2];
  real rb [2];
  reg [7:0] rg [2];
  logic sel;
{item}
  initial begin
    #1 $display("K a=%h %h", a[0], a[1]);
    $finish;
  end
endmodule
"#
    )
}

#[test]
fn shapes_outside_the_three_keep_their_refusal() {
    // A delay, the target as its own source, a sub-array on either side, a `?:` of
    // arrays and a `real` element keep the answer they had.
    for item in [
        "  assign #1 a = b;",
        "  assign a = a;",
        "  assign row = m[1];",
        "  assign a = sel ? b : row;",
        "  assign ra = rb;",
    ] {
        is_loud(
            &with_item(item),
            "a whole unpacked array cannot be the write target in this context",
        );
    }
    is_loud(
        &with_item("  assign m[1] = row;"),
        "partial unpacked-array slice",
    );
}

#[test]
fn a_mismatched_source_or_pattern_names_the_mismatch() {
    // The procedural array assignment's messages, now shared.
    is_loud(
        &with_item("  assign a = n4;"),
        "unpacked-array assignment requires identical element types",
    );
    is_loud(
        &with_item("  assign a = b3;"),
        "unpacked-array assignment requires the same number of dimensions",
    );
    is_loud(
        &with_item("  assign a = '{8'd1, 8'd2, 8'd3};"),
        "assignment pattern has 3 element(s) but the array dimension has 2",
    );
    is_loud(
        &with_item(
            "  function automatic logic [7:0] g(); return 8'd3; endfunction\n  assign a = '{default: g()};",
        ),
        "a call-free `'{default: v}` value",
    );
    // A `reg` array takes the scalar `assign`'s E3018.
    is_loud(
        &with_item("  assign rg = b;"),
        "continuous assign drives variable `t.rg`",
    );
}

#[test]
fn a_packed_target_default_pattern_fills_every_bit() {
    // `'{default: v}` on a PACKED target (`ibex_top`'s `icache_tag_alert`, `ibex_alu`'s
    // `imd_val_we_o`) is a different construct, lowered by `packed_pattern.rs` since §3
    // ⑤ⓛ (its tests are `packed_default_pattern.rs`); it was E3009 here. verilator and
    // sv2v → iverilog print `0000 1111 00`.
    let src = r#"
module t;
  logic [3:0] v0, v1;
  logic [1:0] w;
  assign v0 = '{default:'b0};
  assign v1 = '{default:'1};
  assign w  = '{default: '0};
  initial begin
    #1 $display("D1 %b %b %b", v0, v1, w);
    $finish;
  end
endmodule
"#;
    prints(src, "D1", &["D1 0000 1111 00"]);
}

#[test]
fn writers_no_ir_statement_carries_are_other_writers() {
    // A clocking block's output drive (verilator refuses `cb.dst <= 8'hAB` on an array;
    // iverilog has no clocking blocks), an `inout` child driving an element (iverilog and
    // sv2v → iverilog read `XX` where the parent→child approximation would print `02`),
    // and a task output bound to a hierarchical element, whose placeholder no pass
    // resolves (it panicked the engine; verilator and iverilog print `01 77`).
    let clocking = r#"
module t;
  logic clk = 0;
  logic [7:0] src [2];
  logic [7:0] dst [2];
  assign dst = src;
  clocking cb @(posedge clk);
    output dst;
  endclocking
  initial begin
    src[0] = 8'h11; src[1] = 8'h22;
    cb.dst <= 8'hAB;
    #1 clk = 1;
    #1 $display("C01 %h %h", dst[0], dst[1]);
    $finish;
  end
endmodule
"#;
    is_loud(clocking, SOLE);
    let inout = r#"
module sub(inout wire [7:0] io); assign io = 8'h33; endmodule
module t;
  logic [7:0] s [2];
  wire [7:0] a [2];
  assign a = s;
  sub u(.io(a[1]));
  initial begin s = '{1, 2}; #1 $display("W3 %h %h", a[0], a[1]); end
endmodule
"#;
    is_loud(inout, SOLE);
    let hier_out = r#"
module sub; logic [7:0] src [2]; logic [7:0] dst [2]; assign dst = src;
  initial begin src[0]=1; src[1]=2; end endmodule
module t;
  sub u();
  task automatic tk(output logic [7:0] o); #1 o = 8'h77; endtask
  initial begin #1 tk(u.dst[1]); #1 $display("C35 %h %h", u.dst[0], u.dst[1]); $finish; end
endmodule
"#;
    is_loud(hier_out, SOLE);
}

#[test]
fn a_delayed_net_and_a_two_state_source_keep_their_refusal() {
    // A net declaration delay is applied to a net-declaration assignment only (verilator
    // delays `assign w = src;` by 2 through `wire [7:0] #2 w [2]`), and a `bit` array is
    // not an equivalent element type for a `logic` one (verilator: `Assignment between
    // 2-state and 4-state types`; iverilog: `Element types are not compatible`).
    let delayed = r#"
module t;
  logic [7:0] src [2];
  wire [7:0] #2 w [2];
  assign w = src;
  initial begin src[0]=8'h11; src[1]=8'h22; #1 $display("C39 %h %h", w[0], w[1]); $finish; end
endmodule
"#;
    is_loud(
        delayed,
        "a whole unpacked array cannot be the write target in this context",
    );
    let two_state = r#"
module t;
  bit [7:0] src [2];
  logic [7:0] dst [2];
  assign dst = src;
  initial begin src[0]=8'h01; src[1]=8'h02; #1 $display("C20b %h %h", dst[0], dst[1]); $finish; end
endmodule
"#;
    is_loud(
        two_state,
        "a whole unpacked array cannot be the write target in this context",
    );
}

#[test]
fn an_element_type_from_a_typedef_keeps_its_refusal() {
    // This IR keeps neither an enum's identity nor its base's 2-state-ness
    // (`enum bit [7:0]` elements are recorded as 4-state `logic`), so an element type
    // from a typedef is lowered only for a copy, and only when the parser marks it
    // integral (`cont_assign_typedef_elem.rs`). The enum cells are the reason: an `x`
    // item into `enum bit [7:0]` elements reads `x1` where verilator and iverilog read
    // `01`; a copy from such an array into a `logic` one is refused by both oracles, and
    // one from an `enum logic [7:0]` array by iverilog. A pattern into a typedef element
    // and a 1-bit element (`enum logic` has no range to tell it apart) wait with them.
    for decl in [
        "typedef enum bit [7:0] {A = 8'h01, B = 8'h02} e_t;\n  e_t d [2];\n  logic [7:0] x1 = 8'hx1;\n  assign d = '{e_t'(x1), B};",
        "typedef enum bit [7:0] {A = 8'h11, B = 8'h22} e_t;\n  e_t s [2];\n  wire [7:0] d [2];\n  assign d = s;",
        "typedef enum logic [7:0] {C = 8'h33, D = 8'h44} e_t;\n  e_t s [2];\n  logic [7:0] d [2];\n  assign d = s;",
        "typedef logic [7:0] b_t;\n  b_t d [2];\n  assign d = '{default: 8'h5};",
        "logic d [2];\n  assign d = '{1'b1, 1'b0};",
    ] {
        let src = format!(
            "module t;\n  {decl}\n  initial begin #1 $display(\"T %h %h\", d[0], d[1]); $finish; end\nendmodule\n"
        );
        is_loud(
            &src,
            "a whole unpacked array cannot be the write target in this context",
        );
    }
}

#[test]
fn a_struct_typedef_element_copy_runs() {
    // Formerly refused with the typedefs above; a packed struct typedef is now copied
    // (`cont_assign_typedef_elem.rs`). The never-written source reads `x x` in iverilog
    // 13 (verilator, 2-state, `0 0`; sv2v → iverilog `z z`: it declares the source a
    // `wire` nothing drives).
    let src = "module t;\n  typedef struct packed { logic l; logic [1:0] a; } s_t;\n  s_t s [2];\n  s_t d [2];\n  assign d = s;\n  initial begin #1 $display(\"T %h %h\", d[0], d[1]); $finish; end\nendmodule\n";
    prints(src, "T", &["T x x"]);
}
