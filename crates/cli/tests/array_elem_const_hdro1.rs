//! §3 ⑤ ⓔ census, a header array parameter under an instance override (the typedef imported through a header import), element types v8/asc/lsb4/sg of the 7
//! (`logic [7:0]` / ascending `[0:7]` / non-zero-LSB `[11:4]` / `signed [7:0]` / `int` /
//! a packed struct / multi-packed `[1:0][3:0]`) × 12 consumers (part-select /
//! bit-select / indexed / concat / `$size`+`$high`+`$right` / range bound /
//! generate-if / child override / untyped localparam / replication count / the
//! runtime twin / whole-element untyped localparam). Loud cells are declines on
//! purpose: a multi-packed element, an ascending or non-zero-LSB element in the
//! wide (concat / replication) domain, a header struct-typed override pattern
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
fn elem_asc() {
    // hdro_asc_bound: verilator
    digest(
        "hdro_asc_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
logic [A[1][0:3]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // hdro_asc_bs: verilator
    digest(
        "hdro_asc_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // hdro_asc_cat: verilator
    loud(
        "hdro_asc_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = {A[1][0:3], A[1][0:3]};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the concatenation `{…}` has no ",
    );
    // hdro_asc_genif: verilator
    digest(
        "hdro_asc_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
generate if (A[1][0:3] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // hdro_asc_ip: verilator
    digest(
        "hdro_asc_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [1:0] L = A[1][0+:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // hdro_asc_ovr: verilator
    digest(
        "hdro_asc_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
c #(.R(A[1][0:3])) u();
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // hdro_asc_ps: verilator
    digest(
        "hdro_asc_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [3:0] L = A[1][0:3];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // hdro_asc_rt: verilator
    digest(
        "hdro_asc_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
initial $display("DIGEST=%h %h %h %0d", A[1][0:3], A[1][0], A[1][0+:2], $size(A));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3 0 0 2",
    );
    // hdro_asc_size: verilator
    digest(
        "hdro_asc_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
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
        "2 1 1",
    );
    // hdro_asc_unt: verilator
    digest(
        "hdro_asc_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1][0:3];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // hdro_asc_wunt: verilator
    digest(
        "hdro_asc_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [0:7] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_lsb4() {
    // hdro_lsb4_bound: verilator
    digest(
        "hdro_lsb4_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
logic [A[1][7:4]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // hdro_lsb4_bs: verilator
    digest(
        "hdro_lsb4_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic L = A[1][4];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // hdro_lsb4_cat: verilator
    loud(
        "hdro_lsb4_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = {A[1][7:4], A[1][7:4]};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the concatenation `{…}` has no ",
    );
    // hdro_lsb4_genif: verilator
    digest(
        "hdro_lsb4_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
generate if (A[1][7:4] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // hdro_lsb4_ip: verilator
    digest(
        "hdro_lsb4_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [1:0] L = A[1][7-:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // hdro_lsb4_ovr: verilator
    digest(
        "hdro_lsb4_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
c #(.R(A[1][7:4])) u();
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // hdro_lsb4_ps: verilator
    digest(
        "hdro_lsb4_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [3:0] L = A[1][7:4];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // hdro_lsb4_repl: verilator
    loud(
        "hdro_lsb4_repl",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [11:0] L = {A[1][7-:2]{4'hA}};
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the replication `{n{…}}` has no",
    );
    // hdro_lsb4_rt: verilator
    digest(
        "hdro_lsb4_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
initial $display("DIGEST=%h %h %h %0d", A[1][7:4], A[1][4], A[1][7-:2], $size(A));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // hdro_lsb4_size: verilator
    digest(
        "hdro_lsb4_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
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
        "2 1 1",
    );
    // hdro_lsb4_unt: verilator
    digest(
        "hdro_lsb4_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1][7:4];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // hdro_lsb4_wunt: verilator
    digest(
        "hdro_lsb4_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [11:4] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_sg() {
    // hdro_sg_bound: verilator
    digest(
        "hdro_sg_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // hdro_sg_bs: verilator
    digest(
        "hdro_sg_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
    // hdro_sg_cat: verilator
    digest(
        "hdro_sg_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "33 8",
    );
    // hdro_sg_genif: verilator
    digest(
        "hdro_sg_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
generate if (A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // hdro_sg_ip: verilator
    digest(
        "hdro_sg_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // hdro_sg_ovr: verilator
    digest(
        "hdro_sg_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
c #(.R(A[1][3:0])) u();
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // hdro_sg_ps: verilator
    digest(
        "hdro_sg_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // hdro_sg_rt: verilator
    digest(
        "hdro_sg_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3 1 0 2",
    );
    // hdro_sg_size: verilator
    digest(
        "hdro_sg_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // hdro_sg_unt: verilator
    digest(
        "hdro_sg_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // hdro_sg_wunt: verilator
    digest(
        "hdro_sg_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic signed [7:0] A[2] = '{8'sd3, -8'sd5}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{-8'sd5, 8'sd3})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "03 8",
    );
}

#[test]
fn elem_v8() {
    // hdro_v8_bound: verilator
    digest(
        "hdro_v8_bound",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // hdro_v8_bs: verilator
    digest(
        "hdro_v8_bs",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // hdro_v8_cat: verilator
    digest(
        "hdro_v8_cat",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // hdro_v8_genif: verilator
    digest(
        "hdro_v8_genif",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
generate if (A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // hdro_v8_ip: verilator
    digest(
        "hdro_v8_ip",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // hdro_v8_ovr: verilator
    digest(
        "hdro_v8_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
c #(.R(A[1][3:0])) u();
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // hdro_v8_ps: verilator
    digest(
        "hdro_v8_ps",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // hdro_v8_repl: verilator
    digest(
        "hdro_v8_repl",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam logic [11:0] L = {A[1][3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // hdro_v8_rt: verilator
    digest(
        "hdro_v8_rt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // hdro_v8_size: verilator
    digest(
        "hdro_v8_size",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
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
        "2 1 1",
    );
    // hdro_v8_unt: verilator
    digest(
        "hdro_v8_unt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // hdro_v8_wunt: verilator
    digest(
        "hdro_v8_wunt",
        r#"package tp;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
endpackage
module c2 import tp::*; #(parameter logic [7:0] A[2] = '{8'h3C, 8'hA5}) ();
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
endmodule
module tb;
c2 #(.A('{8'hA5, 8'h3C})) u2();
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}
