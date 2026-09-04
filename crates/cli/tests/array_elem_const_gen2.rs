//! §3 ⑤ ⓔ census, a body array read inside a generate block, element types int/st/pmd of the 7
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
fn elem_int() {
    // gen_int_bound: verilator
    digest(
        "gen_int_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
logic [A[1][3:0]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "13",
    );
    // gen_int_bs: verilator
    digest(
        "gen_int_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam logic L = A[1][0];
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // gen_int_cat: verilator
    digest(
        "gen_int_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam L = {A[1][3:0], A[1][3:0]};
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "cc 8",
    );
    // gen_int_ip: verilator
    digest(
        "gen_int_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam logic [1:0] L = A[1][3-:2];
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "3",
    );
    // gen_int_ovr: verilator
    digest(
        "gen_int_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
c #(.R(A[1][3:0])) u();
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "c",
    );
    // gen_int_ps: verilator
    digest(
        "gen_int_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam logic [3:0] L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // gen_int_repl: verilator
    digest(
        "gen_int_repl",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam logic [11:0] L = {A[1][3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "aaa",
    );
    // gen_int_rt: verilator
    digest(
        "gen_int_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
initial $display("DIGEST=%h %h %h %0d", A[1][3:0], A[1][0], A[1][3-:2], $size(A));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "c 0 3 2",
    );
    // gen_int_size: verilator
    digest(
        "gen_int_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // gen_int_unt: verilator
    digest(
        "gen_int_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam L = A[1][3:0];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "c 4",
    );
    // gen_int_wunt: verilator
    digest(
        "gen_int_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam int A[2] = '{165, 60};
generate if (1) begin : gb
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "0000003c 32",
    );
}

#[test]
fn elem_pmd() {
    // gen_pmd_bound: verilator
    loud(
        "gen_pmd_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
logic [A[1][1]:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_bs: verilator
    loud(
        "gen_pmd_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam logic L = A[1][0][0];
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_cat: verilator
    loud(
        "gen_pmd_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam L = {A[1][1], A[1][1]};
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_ip: verilator
    loud(
        "gen_pmd_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam logic [1:0] L = A[1][1][3-:2];
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_ovr: verilator
    loud(
        "gen_pmd_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
c #(.R(A[1][1])) u();
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_ps: verilator
    loud(
        "gen_pmd_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam logic [3:0] L = A[1][1];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_rt: verilator
    loud(
        "gen_pmd_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
initial $display("DIGEST=%h %h %h %0d", A[1][1], A[1][0][0], A[1][1][3-:2], $size(A));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_size: verilator
    loud(
        "gen_pmd_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_unt: verilator
    loud(
        "gen_pmd_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam L = A[1][1];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
    // gen_pmd_wunt: verilator
    loud(
        "gen_pmd_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam logic [1:0][3:0] A[2] = '{8'hA5, 8'h3C};
generate if (1) begin : gb
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "expected a one-dimensional packed element type on an array parameter (",
    );
}

#[test]
fn elem_st() {
    // gen_st_bound: verilator
    digest(
        "gen_st_bound",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
logic [A[1].a:0] v = '1;
initial $display("DIGEST=%0d", $bits(v));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "7",
    );
    // gen_st_bs: verilator
    digest(
        "gen_st_bs",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam logic L = A[1].b[0];
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "0",
    );
    // gen_st_cat: verilator
    digest(
        "gen_st_cat",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam L = {A[1].a, A[1].a};
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "66 8",
    );
    // gen_st_ip: verilator
    digest(
        "gen_st_ip",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam logic [1:0] L = A[1].a[3-:2];
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "1",
    );
    // gen_st_ovr: verilator
    digest(
        "gen_st_ovr",
        r#"module c #(parameter logic [3:0] R = 0) ();
  initial $display("DIGEST=%h", R);
endmodule
module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
c #(.R(A[1].a)) u();
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "6",
    );
    // gen_st_ps: verilator
    digest(
        "gen_st_ps",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam logic [3:0] L = A[1].a;
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "6 4",
    );
    // gen_st_repl: verilator
    digest(
        "gen_st_repl",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam logic [11:0] L = {A[1].a[3-:2]{4'hA}};
initial $display("DIGEST=%h", L);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "00a",
    );
    // gen_st_rt: verilator
    digest(
        "gen_st_rt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
initial $display("DIGEST=%h %h %h %0d", A[1].a, A[1].b[0], A[1].a[3-:2], $size(A));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "6 0 1 2",
    );
    // gen_st_size: verilator
    digest(
        "gen_st_size",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam int N = $size(A);
localparam int H = $high(A);
localparam int R = $right(A);
initial $display("DIGEST=%0d %0d %0d", N, H, R);
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "2 1 1",
    );
    // gen_st_unt: verilator
    digest(
        "gen_st_unt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam L = A[1].a;
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "6 4",
    );
    // gen_st_wunt: verilator
    digest(
        "gen_st_wunt",
        r#"module tb;
typedef struct packed { logic [3:0] a; logic [1:0] b; } s_t;
localparam s_t A[2] = '{'{4'h9, 2'd1}, '{4'h6, 2'd2}};
generate if (1) begin : gb
localparam L = A[1];
initial $display("DIGEST=%h %0d", L, $bits(L));
end endgenerate
initial begin #1 $finish; end
endmodule
"#,
        "1a 6",
    );
}
