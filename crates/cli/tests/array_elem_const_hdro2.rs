//! §3 ⑤ ⓔ census, a header array parameter under an instance override (the typedef imported through a header import), element types int/st/pmd of the 7
//! (`logic [7:0]` / ascending `[0:7]` / non-zero-LSB `[11:4]` / `signed [7:0]` / `int` /
//! a packed struct / multi-packed `[1:0][3:0]`) × 12 consumers (part-select /
//! bit-select / indexed / concat / `$size`+`$high`+`$right` / range bound /
//! generate-if / child override / untyped localparam / replication count / the
//! runtime twin / whole-element untyped localparam). Loud cells are declines on
//! purpose: a multi-packed element, a header struct-typed override pattern
//! (§4.5.413 residue), a scoped struct member (§2 🆕 L ⓗ), the runtime `$size` of
//! a scalar parameter (pre-existing).
//!
//! Every expected value is the census verilator 5.050 line (iverilog 13.0 agrees
//! where the comment says "both oracles"; it rejects unpacked array parameters).

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
fn elem_int() {
    // hdro_int_bound: verilator
    digest(
        "hdro_int_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // hdro_int_bs: verilator
    digest(
        "hdro_int_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // hdro_int_cat: verilator
    digest(
        "hdro_int_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // hdro_int_genif: verilator
    digest(
        "hdro_int_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
generate if (A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // hdro_int_ip: verilator
    digest(
        "hdro_int_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // hdro_int_ovr: verilator
    digest(
        "hdro_int_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
c #(.R(A[1][3:0])) u();
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // hdro_int_ps: verilator
    digest(
        "hdro_int_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // hdro_int_repl: verilator
    digest(
        "hdro_int_repl",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam logic [11:0] L = {A[1][3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // hdro_int_rt: verilator
    digest(
        "hdro_int_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // hdro_int_size: verilator
    digest(
        "hdro_int_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // hdro_int_unt: verilator
    digest(
        "hdro_int_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // hdro_int_wunt: verilator
    digest(
        "hdro_int_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter int A[2] = '{60, 165}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{165, 60})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0000003c 32",
    );
}

#[test]
fn elem_pmd() {
    // hdro_pmd_bound: verilator
    loud(
        "hdro_pmd_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
logic [A[1][1]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_bs: verilator
    loud(
        "hdro_pmd_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic L = A[1][0][0];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_cat: verilator
    loud(
        "hdro_pmd_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = {A[1][1], A[1][1]};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_genif: verilator
    loud(
        "hdro_pmd_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
generate if (A[1][1] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_ip: verilator
    loud(
        "hdro_pmd_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [1:0] L = A[1][1][3-:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_ovr: verilator
    loud(
        "hdro_pmd_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
c #(.R(A[1][1])) u();
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_ps: verilator
    loud(
        "hdro_pmd_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [3:0] L = A[1][1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_rt: verilator
    loud(
        "hdro_pmd_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
initial $display("DIGEST=%h %h %h %0d", A[1][1], A[1][0][0], A[1][1][3-:2], $size(A));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_size: verilator
    loud(
        "hdro_pmd_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_unt: verilator
    loud(
        "hdro_pmd_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1][1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // hdro_pmd_wunt: verilator
    loud(
        "hdro_pmd_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [1:0][3:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
}

#[test]
fn elem_st() {
    // hdro_st_bound: verilator
    loud(
        "hdro_st_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
logic [A[1].a:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_bs: verilator
    loud(
        "hdro_st_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam logic L = A[1].b[0];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_cat: verilator
    loud(
        "hdro_st_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam L = {A[1].a, A[1].a};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_genif: verilator
    loud(
        "hdro_st_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
generate if (A[1].a > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_ip: verilator
    loud(
        "hdro_st_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam logic [1:0] L = A[1].a[3-:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_ovr: verilator
    loud(
        "hdro_st_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
c #(.R(A[1].a)) u();
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_ps: verilator
    loud(
        "hdro_st_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam logic [3:0] L = A[1].a;
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_repl: verilator
    loud(
        "hdro_st_repl",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam logic [11:0] L = {A[1].a[3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_rt: verilator
    loud(
        "hdro_st_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
initial $display("DIGEST=%h %h %h %0d", A[1].a, A[1].b[0], A[1].a[3-:2], $size(A));
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_size: verilator
    loud(
        "hdro_st_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_unt: verilator
    loud(
        "hdro_st_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam L = A[1].a;
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
    // hdro_st_wunt: verilator
    loud(
        "hdro_st_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter s_t A[2] = '{'{4'h6, 2'd2}, '{4'h9, 2'd1}}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{'{4'h9, 2'd1}, '{4'h6, 2'd2}})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "the override of array parameter `A` is not a constant array (an assign",
    );
}
