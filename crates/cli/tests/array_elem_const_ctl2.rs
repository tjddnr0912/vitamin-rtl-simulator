//! §3 ⑤ ⓔ census, the SCALAR-parameter control twin (`localparam T A1 = v1;` — pre-existing paths, iverilog is a second oracle), element types int/st/pmd of the 7
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
    // ctl_int_bound: both oracles
    digest(
        "ctl_int_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
logic [A1[3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // ctl_int_bs: both oracles
    digest(
        "ctl_int_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam logic L = A1[0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // ctl_int_cat: both oracles
    digest(
        "ctl_int_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam L = {A1[3:0], A1[3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // ctl_int_genif: both oracles
    digest(
        "ctl_int_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
generate if (A1[3:0] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // ctl_int_ip: both oracles
    digest(
        "ctl_int_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam logic [1:0] L = A1[3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // ctl_int_ovr: both oracles
    digest(
        "ctl_int_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
c #(.R(A1[3:0])) u();
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // ctl_int_ps: both oracles
    digest(
        "ctl_int_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam logic [3:0] L = A1[3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // ctl_int_repl: both oracles
    digest(
        "ctl_int_repl",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam logic [11:0] L = {A1[3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // ctl_int_rt: both oracles
    loud(
        "ctl_int_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
initial $display("DIGEST=%h %h %h %0d", A1[3:0], A1[0], A1[3-:2], $size(A1));
initial begin #1 $finish; end
endmodule
"#,
        "unsupported system function in expression",
    );
    // ctl_int_size: both oracles
    digest(
        "ctl_int_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam int N = $size(A1);
localparam int H = $high(A1);
localparam int R = $right(A1);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "32 31 0",
    );
    // ctl_int_unt: both oracles
    digest(
        "ctl_int_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam L = A1[3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // ctl_int_wunt: both oracles
    digest(
        "ctl_int_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A1 = 60;
localparam L = A1;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "0000003c 32",
    );
}

#[test]
fn elem_pmd() {
    // ctl_pmd_bound: verilator
    digest(
        "ctl_pmd_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
logic [A1[1]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "4",
    );
    // ctl_pmd_bs: verilator
    digest(
        "ctl_pmd_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam logic L = A1[0][0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // ctl_pmd_cat: verilator
    digest(
        "ctl_pmd_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam L = {A1[1], A1[1]};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "33 8",
    );
    // ctl_pmd_genif: verilator
    digest(
        "ctl_pmd_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
generate if (A1[1] > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // ctl_pmd_ip: verilator
    digest(
        "ctl_pmd_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam logic [1:0] L = A1[1][3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // ctl_pmd_ovr: verilator
    digest(
        "ctl_pmd_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
c #(.R(A1[1])) u();
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // ctl_pmd_ps: verilator
    digest(
        "ctl_pmd_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam logic [3:0] L = A1[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // ctl_pmd_rt: verilator
    digest(
        "ctl_pmd_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
initial $display("DIGEST=%h %h %h %0d", A1[1], A1[0][0], A1[1][3-:2], $size(A1));
initial begin #1 $finish; end
endmodule
"#,
        "3 0 0 2",
    );
    // ctl_pmd_size: verilator
    digest(
        "ctl_pmd_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam int N = $size(A1);
localparam int H = $high(A1);
localparam int R = $right(A1);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "2 1 0",
    );
    // ctl_pmd_unt: verilator
    digest(
        "ctl_pmd_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam L = A1[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3 4",
    );
    // ctl_pmd_wunt: verilator
    digest(
        "ctl_pmd_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A1 = 8'h3C;
localparam L = A1;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "3c 8",
    );
}

#[test]
fn elem_st() {
    // ctl_st_bound: verilator
    digest(
        "ctl_st_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
logic [A1.a:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
initial begin #1 $finish; end
endmodule
"#,
        "7",
    );
    // ctl_st_bs: verilator
    digest(
        "ctl_st_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam logic L = A1.b[0];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // ctl_st_cat: verilator
    digest(
        "ctl_st_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam L = {A1.a, A1.a};
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "66 8",
    );
    // ctl_st_genif: verilator
    digest(
        "ctl_st_genif",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
generate if (A1.a > 2) begin : g initial $display("DIGEST=gt"); end else begin : h initial $display("DIGEST=le"); end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "gt",
    );
    // ctl_st_ip: verilator
    digest(
        "ctl_st_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam logic [1:0] L = A1.a[3-:2];
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
    // ctl_st_ovr: verilator
    digest(
        "ctl_st_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
c #(.R(A1.a)) u();
initial begin #1 $finish; end
endmodule
"#,
        "6",
    );
    // ctl_st_ps: verilator
    digest(
        "ctl_st_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam logic [3:0] L = A1.a;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "6 4",
    );
    // ctl_st_repl: verilator
    digest(
        "ctl_st_repl",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam logic [11:0] L = {A1.a[3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
initial begin #1 $finish; end
endmodule
"#,
        "00a",
    );
    // ctl_st_rt: verilator
    loud(
        "ctl_st_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
initial $display("DIGEST=%h %h %h %0d", A1.a, A1.b[0], A1.a[3-:2], $size(A1));
initial begin #1 $finish; end
endmodule
"#,
        "unsupported system function in expression",
    );
    // ctl_st_size: both oracles
    digest(
        "ctl_st_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam int N = $size(A1);
localparam int H = $high(A1);
localparam int R = $right(A1);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
initial begin #1 $finish; end
endmodule
"#,
        "6 5 0",
    );
    // ctl_st_unt: verilator
    digest(
        "ctl_st_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam L = A1.a;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "6 4",
    );
    // ctl_st_wunt: both oracles
    digest(
        "ctl_st_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A1 = '{4'h6, 2'd2};
localparam L = A1;
initial $display("DIGEST=%h %0d", L, $bits(L));
initial begin #1 $finish; end
endmodule
"#,
        "1a 6",
    );
}
