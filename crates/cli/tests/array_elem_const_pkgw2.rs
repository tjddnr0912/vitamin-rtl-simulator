//! §3 ⑤ ⓔ census, a package array under `import p::*`, element types int/st/pmd of the 7
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
    // pkgw_int_bound: verilator
    digest(
        "pkgw_int_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // pkgw_int_bs: verilator
    digest(
        "pkgw_int_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgw_int_cat: verilator
    digest(
        "pkgw_int_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // pkgw_int_genif: verilator
    digest(
        "pkgw_int_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
generate if (A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // pkgw_int_ip: verilator
    digest(
        "pkgw_int_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // pkgw_int_ovr: verilator
    digest(
        "pkgw_int_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
import p::*;
c #(.R(A[1][3:0])) u();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // pkgw_int_ps: verilator
    digest(
        "pkgw_int_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // pkgw_int_repl: verilator
    digest(
        "pkgw_int_repl",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam logic [11:0] L = {A[1][3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // pkgw_int_rt: verilator
    digest(
        "pkgw_int_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // pkgw_int_size: verilator
    digest(
        "pkgw_int_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // pkgw_int_unt: verilator
    digest(
        "pkgw_int_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // pkgw_int_wunt: verilator
    digest(
        "pkgw_int_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter int A[2] = '{165, 60};
endpackage
module tb;
import p::*;
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "0000003c 32",
    );
}

#[test]
fn elem_pmd() {
    // pkgw_pmd_bound: verilator
    loud(
        "pkgw_pmd_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
logic [A[1][1]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_bs: verilator
    loud(
        "pkgw_pmd_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam logic L = A[1][0][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_cat: verilator
    loud(
        "pkgw_pmd_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam L = {A[1][1], A[1][1]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_genif: verilator
    loud(
        "pkgw_pmd_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
generate if (A[1][1] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_ip: verilator
    loud(
        "pkgw_pmd_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam logic [1:0] L = A[1][1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_ovr: verilator
    loud(
        "pkgw_pmd_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
import p::*;
c #(.R(A[1][1])) u();
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_ps: verilator
    loud(
        "pkgw_pmd_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam logic [3:0] L = A[1][1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_rt: verilator
    loud(
        "pkgw_pmd_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
initial $display("DIGEST=%h %h %h %0d", A[1][1], A[1][0][0], A[1][1][3-:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_size: verilator
    loud(
        "pkgw_pmd_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_unt: verilator
    loud(
        "pkgw_pmd_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam L = A[1][1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // pkgw_pmd_wunt: verilator
    loud(
        "pkgw_pmd_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
import p::*;
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
}

#[test]
fn elem_st() {
    // pkgw_st_bound: verilator
    digest(
        "pkgw_st_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
logic [A[1].a:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "7",
    );
    // pkgw_st_bs: verilator
    digest(
        "pkgw_st_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam logic L = A[1].b[0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgw_st_cat: verilator
    digest(
        "pkgw_st_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam L = {A[1].a, A[1].a};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "66 8",
    );
    // pkgw_st_genif: verilator
    digest(
        "pkgw_st_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
generate if (A[1].a > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // pkgw_st_ip: verilator
    digest(
        "pkgw_st_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam logic [1:0] L = A[1].a[3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
    // pkgw_st_ovr: verilator
    digest(
        "pkgw_st_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
import p::*;
c #(.R(A[1].a)) u();
initial begin #1 $finish; end
endmodule
"#,
        "6",
    );
    // pkgw_st_ps: verilator
    digest(
        "pkgw_st_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam logic [3:0] L = A[1].a;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "6 4",
    );
    // pkgw_st_repl: verilator
    digest(
        "pkgw_st_repl",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam logic [11:0] L = {A[1].a[3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "00a",
    );
    // pkgw_st_rt: verilator
    digest(
        "pkgw_st_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
initial $display("DIGEST=%h %h %h %0d", A[1].a, A[1].b[0], A[1].a[3-:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "6 0 1 2",
    );
    // pkgw_st_size: verilator
    digest(
        "pkgw_st_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // pkgw_st_unt: verilator
    digest(
        "pkgw_st_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam L = A[1].a;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "6 4",
    );
    // pkgw_st_wunt: verilator
    digest(
        "pkgw_st_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
endpackage
module tb;
import p::*;
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "1a 6",
    );
}
