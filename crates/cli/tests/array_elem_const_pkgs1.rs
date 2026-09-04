//! §3 ⑤ ⓔ census, a package array read `p::A[1]` / `$size(p::A)`, element types v8/asc/lsb4/sg of the 7
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
    // pkgs_asc_bound: verilator
    digest(
        "pkgs_asc_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
logic [p::A[1][0:3]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // pkgs_asc_bs: verilator
    digest(
        "pkgs_asc_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic L = p::A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgs_asc_cat: verilator
    loud(
        "pkgs_asc_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = {p::A[1][0:3], p::A[1][0:3]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the concatenation `{…}` has no ",
    );
    // pkgs_asc_genif: verilator
    digest(
        "pkgs_asc_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
generate if (p::A[1][0:3] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // pkgs_asc_ip: verilator
    digest(
        "pkgs_asc_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [1:0] L = p::A[1][0+:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgs_asc_ovr: verilator
    digest(
        "pkgs_asc_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
c #(.R(p::A[1][0:3])) u();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // pkgs_asc_ps: verilator
    digest(
        "pkgs_asc_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [3:0] L = p::A[1][0:3];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // pkgs_asc_rt: verilator
    loud(
        "pkgs_asc_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
initial $display("DIGEST=%h %h %h %0d", p::A[1][0:3], p::A[1][0], p::A[1][0+:2], $size(p::A));
initial begin #1 $finish; end
endmodule
"#,
        "a part-select of a package array element (`pkg::arr[i][m:l]`) is not s",
    );
    // pkgs_asc_size: verilator
    digest(
        "pkgs_asc_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam int N = $size(p::A);
localparam int H = $high(p::A);
localparam int R = $right(p::A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // pkgs_asc_unt: verilator
    digest(
        "pkgs_asc_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = p::A[1][0:3];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // pkgs_asc_wunt: verilator
    digest(
        "pkgs_asc_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [0:7] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = p::A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_lsb4() {
    // pkgs_lsb4_bound: verilator
    digest(
        "pkgs_lsb4_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
logic [p::A[1][7:4]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // pkgs_lsb4_bs: verilator
    digest(
        "pkgs_lsb4_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic L = p::A[1][4];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgs_lsb4_cat: verilator
    loud(
        "pkgs_lsb4_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = {p::A[1][7:4], p::A[1][7:4]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the concatenation `{…}` has no ",
    );
    // pkgs_lsb4_genif: verilator
    digest(
        "pkgs_lsb4_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
generate if (p::A[1][7:4] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // pkgs_lsb4_ip: verilator
    digest(
        "pkgs_lsb4_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [1:0] L = p::A[1][7-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // pkgs_lsb4_ovr: verilator
    digest(
        "pkgs_lsb4_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
c #(.R(p::A[1][7:4])) u();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // pkgs_lsb4_ps: verilator
    digest(
        "pkgs_lsb4_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [3:0] L = p::A[1][7:4];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // pkgs_lsb4_repl: verilator
    loud(
        "pkgs_lsb4_repl",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [11:0] L = {p::A[1][7-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "parameter `L` value is not a constant: the replication `{n{…}}` has no",
    );
    // pkgs_lsb4_rt: verilator
    loud(
        "pkgs_lsb4_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
initial $display("DIGEST=%h %h %h %0d", p::A[1][7:4], p::A[1][4], p::A[1][7-:2], $size(p::A));
initial begin #1 $finish; end
endmodule
"#,
        "a part-select of a package array element (`pkg::arr[i][m:l]`) is not s",
    );
    // pkgs_lsb4_size: verilator
    digest(
        "pkgs_lsb4_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam int N = $size(p::A);
localparam int H = $high(p::A);
localparam int R = $right(p::A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // pkgs_lsb4_unt: verilator
    digest(
        "pkgs_lsb4_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = p::A[1][7:4];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // pkgs_lsb4_wunt: verilator
    digest(
        "pkgs_lsb4_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [11:4] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = p::A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_sg() {
    // pkgs_sg_bound: verilator
    digest(
        "pkgs_sg_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
logic [p::A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // pkgs_sg_bs: verilator
    digest(
        "pkgs_sg_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam logic L = p::A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
    // pkgs_sg_cat: verilator
    digest(
        "pkgs_sg_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam L = {p::A[1][3:0], p::A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "33 8",
    );
    // pkgs_sg_genif: verilator
    digest(
        "pkgs_sg_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
generate if (p::A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // pkgs_sg_ip: verilator
    digest(
        "pkgs_sg_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam logic [1:0] L = p::A[1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgs_sg_ovr: verilator
    digest(
        "pkgs_sg_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
c #(.R(p::A[1][3:0])) u();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // pkgs_sg_ps: verilator
    digest(
        "pkgs_sg_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam logic [3:0] L = p::A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // pkgs_sg_rt: verilator
    loud(
        "pkgs_sg_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
initial $display("DIGEST=%h %h %h %0d", p::A[1][3:0], p::A[1][0], p::A[1][3-:2], $size(p::A));
initial begin #1 $finish; end
endmodule
"#,
        "a part-select of a package array element (`pkg::arr[i][m:l]`) is not s",
    );
    // pkgs_sg_size: verilator
    digest(
        "pkgs_sg_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam int N = $size(p::A);
localparam int H = $high(p::A);
localparam int R = $right(p::A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // pkgs_sg_unt: verilator
    digest(
        "pkgs_sg_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam L = p::A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // pkgs_sg_wunt: verilator
    digest(
        "pkgs_sg_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic signed [7:0] A[2] = '{-8'sd5, 8'sd3};
endpackage
module tb;
localparam L = p::A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "03 8",
    );
}

#[test]
fn elem_v8() {
    // pkgs_v8_bound: verilator
    digest(
        "pkgs_v8_bound",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
logic [p::A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // pkgs_v8_bs: verilator
    digest(
        "pkgs_v8_bs",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic L = p::A[1][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // pkgs_v8_cat: verilator
    digest(
        "pkgs_v8_cat",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = {p::A[1][3:0], p::A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // pkgs_v8_genif: verilator
    digest(
        "pkgs_v8_genif",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
generate if (p::A[1][3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // pkgs_v8_ip: verilator
    digest(
        "pkgs_v8_ip",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [1:0] L = p::A[1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // pkgs_v8_ovr: verilator
    digest(
        "pkgs_v8_ovr",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
c #(.R(p::A[1][3:0])) u();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // pkgs_v8_ps: verilator
    digest(
        "pkgs_v8_ps",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [3:0] L = p::A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // pkgs_v8_repl: verilator
    digest(
        "pkgs_v8_repl",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam logic [11:0] L = {p::A[1][3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // pkgs_v8_rt: verilator
    loud(
        "pkgs_v8_rt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
initial $display("DIGEST=%h %h %h %0d", p::A[1][3:0], p::A[1][0], p::A[1][3-:2], $size(p::A));
initial begin #1 $finish; end
endmodule
"#,
        "a part-select of a package array element (`pkg::arr[i][m:l]`) is not s",
    );
    // pkgs_v8_size: verilator
    digest(
        "pkgs_v8_size",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam int N = $size(p::A);
localparam int H = $high(p::A);
localparam int R = $right(p::A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // pkgs_v8_unt: verilator
    digest(
        "pkgs_v8_unt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = p::A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // pkgs_v8_wunt: verilator
    digest(
        "pkgs_v8_wunt",
        r#"package p;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
parameter logic [7:0] A[2] = '{8'hA5, 8'h3C};
endpackage
module tb;
localparam L = p::A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}
