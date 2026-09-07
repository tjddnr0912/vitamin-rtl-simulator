//! §3 ⑤ ⓔ census, a body `localparam T A[2]`, element types v8/asc/lsb4/sg of the 7
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

#[test]
fn elem_asc() {
    // body_asc_bound: verilator
    digest(
        "body_asc_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
logic [A[1][0:3]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // body_asc_bs: verilator
    digest(
        "body_asc_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // body_asc_cat: verilator
    digest(
        "body_asc_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam L = {A[1][0:3], A[1][0:3]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "33 8",
    );
    // body_asc_genif: verilator
    digest(
        "body_asc_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
generate if (A[1][0:3] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // body_asc_ip: verilator
    digest(
        "body_asc_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam logic [1:0] L = A[1][0+:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // body_asc_ovr: verilator
    digest(
        "body_asc_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
c #(.R(A[1][0:3])) u();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // body_asc_ps: verilator
    digest(
        "body_asc_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[1][0:3];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // body_asc_rt: verilator
    digest(
        "body_asc_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
initial $display("DIGEST=%h %h %h %0d", A[1][0:3], A[1][0], A[1][0+:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "3 0 0 2",
    );
    // body_asc_size: verilator
    digest(
        "body_asc_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // body_asc_unt: verilator
    digest(
        "body_asc_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam L = A[1][0:3];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // body_asc_wunt: verilator
    digest(
        "body_asc_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [0:7] A[2] = '{8'hA5, 8'h3C};
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_lsb4() {
    // body_lsb4_bound: verilator
    digest(
        "body_lsb4_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
logic [A[1][7:4]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // body_lsb4_bs: verilator
    digest(
        "body_lsb4_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam logic L = A[1][4];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // body_lsb4_cat: verilator
    digest(
        "body_lsb4_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam L = {A[1][7:4], A[1][7:4]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // body_lsb4_genif: verilator
    digest(
        "body_lsb4_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
generate if (A[1][7:4] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // body_lsb4_ip: verilator
    digest(
        "body_lsb4_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam logic [1:0] L = A[1][7-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // body_lsb4_ovr: verilator
    digest(
        "body_lsb4_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
c #(.R(A[1][7:4])) u();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // body_lsb4_ps: verilator
    digest(
        "body_lsb4_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[1][7:4];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // body_lsb4_repl: verilator
    digest(
        "body_lsb4_repl",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam logic [11:0] L = {A[1][7-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // body_lsb4_rt: verilator
    digest(
        "body_lsb4_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
initial $display("DIGEST=%h %h %h %0d", A[1][7:4], A[1][4], A[1][7-:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // body_lsb4_size: verilator
    digest(
        "body_lsb4_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // body_lsb4_unt: verilator
    digest(
        "body_lsb4_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam L = A[1][7:4];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // body_lsb4_wunt: verilator
    digest(
        "body_lsb4_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [11:4] A[2] = '{8'hA5, 8'h3C};
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_sg() {
    // body_sg_bound: verilator
    digest(
        "body_sg_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // body_sg_bs: verilator
    digest(
        "body_sg_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
    // body_sg_cat: verilator
    digest(
        "body_sg_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "33 8",
    );
    // body_sg_genif: verilator
    digest(
        "body_sg_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
generate if (A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // body_sg_ip: verilator
    digest(
        "body_sg_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // body_sg_ovr: verilator
    digest(
        "body_sg_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
c #(.R(A[1][3:0])) u();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // body_sg_ps: verilator
    digest(
        "body_sg_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // body_sg_rt: verilator
    digest(
        "body_sg_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "3 1 0 2",
    );
    // body_sg_size: verilator
    digest(
        "body_sg_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // body_sg_unt: verilator
    digest(
        "body_sg_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // body_sg_wunt: verilator
    digest(
        "body_sg_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "03 8",
    );
}

#[test]
fn elem_v8() {
    // body_v8_bound: verilator
    digest(
        "body_v8_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // body_v8_bs: verilator
    digest(
        "body_v8_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // body_v8_cat: verilator
    digest(
        "body_v8_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // body_v8_genif: verilator
    digest(
        "body_v8_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
generate if (A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // body_v8_ip: verilator
    digest(
        "body_v8_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // body_v8_ovr: verilator
    digest(
        "body_v8_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
c #(.R(A[1][3:0])) u();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // body_v8_ps: verilator
    digest(
        "body_v8_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // body_v8_repl: verilator
    digest(
        "body_v8_repl",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam logic [11:0] L = {A[1][3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // body_v8_rt: verilator
    digest(
        "body_v8_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // body_v8_size: verilator
    digest(
        "body_v8_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // body_v8_unt: verilator
    digest(
        "body_v8_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // body_v8_wunt: verilator
    digest(
        "body_v8_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}
