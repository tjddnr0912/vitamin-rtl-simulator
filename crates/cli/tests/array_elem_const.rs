//! §3 ⑤ ⓔ: constant-context consumers of an array-parameter ELEMENT — a select or
//! struct member of `A[i]` in a localparam / range bound / generate-if / child
//! override / replication count / ternary / concatenation / `$bits`, the `$size`
//! family over an array parameter (module, package, scoped, header, generate loop
//! bound) and over a parameter with a declared range, `pkg::X` inside an untyped
//! constant concatenation, and the width an untyped parameter takes from a select
//! initializer (`localparam L = A[1][3:0]` is 4 bits, `localparam L = A[1]` the
//! element's type — was 32 at exit 0, the scalar spelling too).
//!
//! Oracle: verilator 5.050 (`--binary --timing`); iverilog 13.0 rejects every
//! unpacked array parameter ("sorry: unpacked array parameters are not supported
//! yet") and is a second oracle only where the comment says "both oracles".

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_array_el_{}_{n}", std::process::id()));
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
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Every `DIGEST=` line, sorted, joined by `|` (the census harness format; sorted
/// because two instances print in scheduling order).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let mut v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    v.sort_unstable();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{name}: expected a loud reject:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

#[test]
fn probe_a01() {
    // a01_ps_elem: verilator
    digest(
        "a01_ps_elem",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [4:0] L = A[1][4:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "1c 5",
    );
}

#[test]
fn probe_a02() {
    // a02_bs_elem: verilator
    digest(
        "a02_bs_elem",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam L = A[0][7];
initial $display("DIGEST=%0d", L);
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
}

#[test]
fn probe_a03() {
    // a03_ip_elem: verilator
    digest(
        "a03_ip_elem",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[1][7-:4];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
}

#[test]
fn probe_a04() {
    // a04_member_elem: verilator
    digest(
        "a04_member_elem",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
localparam int X = S[1].b;
localparam logic [3:0] Y = S[0].a;
initial $display("DIGEST=%0d %h", X, Y);
initial begin #1 $finish; end
endmodule
"#,
        "2 9",
    );
}

#[test]
fn probe_a05() {
    // a05_size: verilator
    digest(
        "a05_size",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int N = $size(A);
logic [N-1:0] v = '1;
initial $display("DIGEST=%0d %0d", N, $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "2 2",
    );
}

#[test]
fn probe_a06() {
    // a06_size_range: verilator
    digest(
        "a06_size_range",
        r#"module tb;
localparam int I[0:2] = '{10, 20, 30};
localparam int N = $size(I);
initial $display("DIGEST=%0d", N);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
}

#[test]
fn probe_a07() {
    // a07_genif_member: verilator
    digest(
        "a07_genif_member",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (S[1].b == 2) begin : g
  initial $display("DIGEST=yes");
end else begin : h
  initial $display("DIGEST=no");
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "yes",
    );
}

#[test]
fn probe_a08() {
    // a08_scoped_elem: verilator
    digest(
        "a08_scoped_elem",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
localparam logic [7:0] L = p::A[1];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3c",
    );
}

#[test]
fn probe_a09() {
    // a09_scoped_elem_ps: verilator
    digest(
        "a09_scoped_elem_ps",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
localparam logic [3:0] L = p::A[1][3:0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
}

#[test]
fn probe_a10() {
    // a10_scoped_member: verilator — PRE-EXISTING (§2 🆕 L ⓗ): a scoped member access `p::S[1].b` — the struct desugar keys on the bare first segment
    loud(
        "a10_scoped_member",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
localparam int L = p::S[1].b;
initial $display("DIGEST=%0d", L);
initial begin #1 $finish; end
endmodule
"#,
        "expected ';', found '.'",
    );
}

#[test]
fn probe_a11() {
    // a11_import_member: verilator
    digest(
        "a11_import_member",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
import p::*;
localparam int L = S[0].b;
localparam logic [3:0] M = S[1].a;
initial $display("DIGEST=%0d %h", L, M);
initial begin #1 $finish; end
endmodule
"#,
        "1 6",
    );
}

#[test]
fn probe_a12() {
    // a12_concat_scoped: verilator
    digest(
        "a12_concat_scoped",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
localparam logic [8:0] L = {1'b1, p::X};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "15a 9",
    );
}

#[test]
fn probe_a13() {
    // a13_concat_import: verilator
    digest(
        "a13_concat_import",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
import p::*;
localparam logic [8:0] L = {1'b1, X};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "15a 9",
    );
}

#[test]
fn probe_a14() {
    // a14_size_pkg: verilator
    digest(
        "a14_size_pkg",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
  parameter int N2 = $size(A);
endpackage
module tb;
import p::*;
localparam int N = p::N2;
initial $display("DIGEST=%0d", N);
initial begin #1 $finish; end
endmodule
"#,
        "2",
    );
}

#[test]
fn probe_a15() {
    // a15_override_ip: verilator
    digest(
        "a15_override_ip",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
genvar i;
generate for (i = 0; i < 2; i++) begin : g
  c #(.R(A[i][7-:4])) u();
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "3|a",
    );
}

#[test]
fn probe_a16() {
    // a16_override_elem: verilator
    digest(
        "a16_override_elem",
        r#"module c #(parameter logic [7:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
genvar i;
generate for (i = 0; i < 2; i++) begin : g
  c #(.R(A[i])) u();
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "3c|a5",
    );
}

#[test]
fn probe_a17() {
    // a17_bound: verilator
    digest(
        "a17_bound",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'd5, 8'd3};
logic [A[1][3:0]-1:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
}

#[test]
fn probe_a18() {
    // a18_genif_bit: verilator
    digest(
        "a18_genif_bit",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
generate if (A[0][0]) begin : g initial $display("DIGEST=one"); end
else begin : h initial $display("DIGEST=zero"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "one",
    );
}

#[test]
fn probe_a19() {
    // a19_asc_elem: verilator
    digest(
        "a19_asc_elem",
        r#"module tb;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[0][0:3];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "a",
    );
}

#[test]
fn probe_a20() {
    // a20_lsb4_elem: verilator
    digest(
        "a20_lsb4_elem",
        r#"module tb;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[0][11:8];
localparam logic [3:0] M = A[1][7:4];
initial $display("DIGEST=%h %h", L, M);
initial begin #1 $finish; end
endmodule
"#,
        "a c",
    );
}

#[test]
fn probe_a21() {
    // a21_signed_elem: verilator
    digest(
        "a21_signed_elem",
        r#"module tb;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam int L = A[0][7:4];
localparam int M = A[0];
initial $display("DIGEST=%0d %0d", L, M);
initial begin #1 $finish; end
endmodule
"#,
        "15 -5",
    );
}

#[test]
fn probe_a22() {
    // a22_hdr_elem_ps: verilator
    digest(
        "a22_hdr_elem_ps",
        r#"module c #(parameter logic [7:0] A[2] = '{8'hA5, 8'h3C}) ();
  localparam logic [3:0] L = A[1][3:0];
  initial $display("DIGEST=%h", L);
endmodule
module tb;
c u1();
c #(.A('{8'h12, 8'h34})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "4|c",
    );
}

#[test]
fn probe_a24() {
    // a24_size_packed: split (vita = verilator)
    digest(
        "a24_size_packed",
        r#"module tb;
localparam logic [11:4] W = 8'h5A;
localparam int N = $size(W);
localparam int H = $high(W);
localparam int L = $low(W);
initial $display("DIGEST=%0d %0d %0d", N, H, L);
initial begin #1 $finish; end
endmodule
"#,
        "8 11 4",
    );
}

#[test]
fn probe_a25() {
    // a25_size_dim2: verilator
    digest(
        "a25_size_dim2",
        r#"module tb;
localparam logic [7:0] A[3] = '{8'd1, 8'd2, 8'd3};
localparam int N = $size(A, 2);
localparam int M = $size(A, 1);
initial $display("DIGEST=%0d %0d", N, M);
initial begin #1 $finish; end
endmodule
"#,
        "8 3",
    );
}

#[test]
fn probe_a26() {
    // a26_repl_count: verilator
    digest(
        "a26_repl_count",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'd2, 8'd3};
localparam logic [11:0] R = {A[1][3:0]{4'hA}};
initial $display("DIGEST=%h", R);
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
}

#[test]
fn probe_a27() {
    // a27_concat_elem: verilator
    digest(
        "a27_concat_elem",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [15:0] R = {A[0], A[1]};
localparam logic [11:0] Q = {A[0][3:0], A[1]};
initial $display("DIGEST=%h %h", R, Q);
initial begin #1 $finish; end
endmodule
"#,
        "a53c 53c",
    );
}

#[test]
fn probe_a28() {
    // a28_pmd_elem: verilator — a multi-packed ELEMENT `logic [1:0][3:0] A[2]` — `A[1][0]` names a packed nibble, declined on purpose (verilator c 3; residue)
    loud(
        "a28_pmd_elem",
        r#"module tb;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[1][0];
localparam logic [3:0] M = A[1][1];
initial $display("DIGEST=%h %h", L, M);
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
}

#[test]
fn probe_a29() {
    // a29_runtime: verilator
    digest(
        "a29_runtime",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
initial $display("DIGEST=%h %0d %0d %h", A[1][4:0], S[1].b, $size(A), p_dummy());
function automatic int p_dummy(); return 7; endfunction
initial begin #1 $finish; end
endmodule
"#,
        "1c 2 2 00000007",
    );
}

#[test]
fn probe_a30() {
    // a30_oor: verilator — a select reaching outside the element (`A[1][9:6]` on 8 bits) is `x` (§11.5.1) — declined; verilator (2-state) prints 0
    loud(
        "a30_oor",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[1][9:6];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the part-select [9:6] is outsid",
    );
}

#[test]
fn probe_a32() {
    // a32_size_genloop: verilator
    digest(
        "a32_size_genloop",
        r#"module tb;
localparam logic [7:0] A[3] = '{8'd1, 8'd2, 8'd3};
genvar i;
generate for (i = 0; i < $size(A); i++) begin : g
  initial $display("DIGEST=%0d", A[i]);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "1|2|3",
    );
}

#[test]
fn probe_a33() {
    // a33_shadow: verilator
    digest(
        "a33_shadow",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : g
  localparam logic [7:0] A = 8'h77;
  localparam logic [3:0] L = A[3:0];
  initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "7",
    );
}

#[test]
fn probe_a34() {
    // a34_concat_scoped_elem: verilator
    digest(
        "a34_concat_scoped_elem",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
localparam logic [15:0] R = {p::A[0], p::A[1]};
initial $display("DIGEST=%h", R);
initial begin #1 $finish; end
endmodule
"#,
        "a53c",
    );
}

#[test]
fn probe_a35() {
    // a35_size_scoped: verilator
    digest(
        "a35_size_scoped",
        r#"package p;
  typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
  parameter int I[0:2] = '{10, 20, 30};
  parameter logic [7:0] X = 8'h5A;
endpackage
module tb;
localparam int N = $size(p::A);
localparam int M = $size(p::I);
initial $display("DIGEST=%0d %0d", N, M);
initial begin #1 $finish; end
endmodule
"#,
        "2 3",
    );
}

#[test]
fn probe_a36() {
    // a36_ternary_elem: verilator
    digest(
        "a36_ternary_elem",
        r#"module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int L = A[1][7] ? A[0][3:0] : A[1][3:0];
initial $display("DIGEST=%0d", L);
initial begin #1 $finish; end
endmodule
"#,
        "12",
    );
}

#[test]
fn probe_a37() {
    // a37_member_typed_int: verilator
    digest(
        "a37_member_typed_int",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t S[2] = '{'{a: 4'h9, b: 2'd1}, '{a: 4'h6, b: 2'd2}};
localparam int X = S[1].a + S[0].b;
initial $display("DIGEST=%0d", X);
initial begin #1 $finish; end
endmodule
"#,
        "7",
    );
}

#[test]
fn probe_a38() {
    // a38_bits_elem: verilator
    digest(
        "a38_bits_elem",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam s_t S[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
localparam int B1 = $bits(A[1]);
localparam int B2 = $bits(A[1][4:0]);
localparam int B3 = $bits(S[1].a);
localparam int B4 = $bits(A);
initial $display("DIGEST=%0d %0d %0d %0d", B1, B2, B3, B4);
initial begin #1 $finish; end
endmodule
"#,
        "8 5 4 16",
    );
}

#[test]
fn probe_a39() {
    // a39_concat_untyped: both oracles
    digest(
        "a39_concat_untyped",
        r#"package p; parameter logic [7:0] X = 8'h5A; endpackage
module tb;
localparam L = {1'b1, p::X};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "15a 9",
    );
}

#[test]
fn probe_a40() {
    // a40_pkg_size_in_pkg: verilator
    digest(
        "a40_pkg_size_in_pkg",
        r#"package p;
  parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
  parameter int N2 = $size(A);
  parameter logic [3:0] E = A[1][3:0];
endpackage
module tb;
import p::*;
initial $display("DIGEST=%0d %h", N2, E);
initial begin #1 $finish; end
endmodule
"#,
        "2 c",
    );
}

/// Adversarial-review pins (§4.5.416). B1: a generate-scope SCALAR named like an outer
/// constant array shadows it for every consumer (the wide domain and the `$size`
/// family included — and the value arm's GAP-G root, which read the outer element:
/// `L` was 20). F1: an UNTYPED, unranged child parameter overridden by a SELECT of an
/// element stays loud (its meta would come from the default literal, §2 row 25 — the
/// scalar spelling `.P(W[3:0])` is that pre-existing silent 32). Oracle: verilator.
#[test]
fn review_pins() {
    digest(
        "b1_generate_scalar_shadows_outer_array",
        r#"module tb;
  localparam int ROT [0:3] = '{10,20,30,40};
  generate if (1) begin : g
    localparam int ROT = 99;
    localparam L = ROT[1];
    localparam M = {ROT[0], ROT[1]};
    localparam N = ROT[3:0];
    initial $display("DIGEST=%0d/%0d %0h/%0d %0d/%0d", L,$bits(L), M,$bits(M), N,$bits(N));
  end endgenerate
  initial begin #1 $finish; end
endmodule
"#,
        "1/1 3/2 3/4",
    );
    digest(
        "b1_generate_vector_shadows_outer_array",
        r#"module tb;
  localparam int ROT [0:3] = '{10,20,30,40};
  generate if (1) begin : g
    localparam logic [7:0] ROT = 8'hC3;
    localparam S = $size(ROT);
    localparam P = ROT[7:4];
    localparam W = {ROT[1], 1'b0};
    initial $display("DIGEST=%0d %0h/%0d %0h/%0d", S, P,$bits(P), W,$bits(W));
  end endgenerate
  initial begin #1 $finish; end
endmodule
"#,
        "8 c/4 2/2",
    );
    loud(
        "f1_untyped_override_by_element_select",
        r#"module c #(parameter P = 0) (); initial $display("DIGEST=bits=%0d cat=%h", $bits(P), {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
c #(.P(A[0][3:0])) u1();
initial begin #1 $finish; end
endmodule
"#,
        "is a select of an array-parameter element",
    );
    // the TYPED target takes the select (control: verilator `bits=4 cat=55`)
    digest(
        "f1_typed_override_by_element_select",
        r#"module c #(parameter logic [3:0] P = 0) (); initial $display("DIGEST=bits=%0d cat=%h", $bits(P), {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
c #(.P(A[0][3:0])) u1();
initial begin #1 $finish; end
endmodule
"#,
        "bits=4 cat=55",
    );
}
