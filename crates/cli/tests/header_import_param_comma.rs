//! §3 ⑤ ⓕ census, the comma-list `import p::*, q::Q` position: 26 header shapes (scalar / typed / untyped /
//! range / range + default / expression / sibling / header localparam / named and
//! positional override / enum label / port width / body use / body localparam /
//! generate / child override / `$clog2` / `$bits` / conditional / concat /
//! part-select / header shadow / body shadow), each with a control twin that
//! spells the constant `p::X` and has no import (PRE-correct). Loud cells are the
//! LRM side of an oracle split (a header default naming a body localparam
//! declared later) or an illegal explicit-import collision (§26.3).
//!
//! Every expected value is the census oracle line (iverilog 13.0 `-g2012` and
//! verilator 5.050 agree unless the comment says which one ran; the `gen` cells
//! are the known iverilog `z` on a genvar bit-select, vita = verilator).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_/Users_{}_{n}", std::process::id()));
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
fn shape_bitsof() {
    // comma_bitsof: both oracles
    digest(
        "comma_bitsof",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = $bits(Dflt)) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "8",
    );
}

#[test]
fn shape_bitsof_ctrl() {
    // comma_bitsof_ctrl: both oracles
    digest(
        "comma_bitsof_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int X = $bits(p::Dflt)) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "8",
    );
}

#[test]
fn shape_body() {
    // comma_body: both oracles
    digest(
        "comma_body",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int N = W) ();  initial begin : b logic [N-1:0] v; v = '1; $display("DIGEST=%h", v); end endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "f",
    );
}

#[test]
fn shape_body_ctrl() {
    // comma_body_ctrl: both oracles
    digest(
        "comma_body_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int N = p::W) ();  initial begin : b logic [N-1:0] v; v = '1; $display("DIGEST=%h", v); end endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "f",
    );
}

#[test]
fn shape_bodylp() {
    // comma_bodylp: both oracles
    digest(
        "comma_bodylp",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int N = W) (); localparam int L = N * 3; initial $display("DIGEST=%0d", L); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "12",
    );
}

#[test]
fn shape_bodylp_ctrl() {
    // comma_bodylp_ctrl: both oracles
    digest(
        "comma_bodylp_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int N = p::W) (); localparam int L = N * 3; initial $display("DIGEST=%0d", L); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "12",
    );
}

#[test]
fn shape_child() {
    // comma_child: both oracles
    digest(
        "comma_child",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module c #(parameter int K = 1) (); initial $display("DIGEST=%0d", K); endmodule
module m import p::*, q::Q; #(parameter int N = W) (); c #(.K(N)) k();  endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4",
    );
}

#[test]
fn shape_child_ctrl() {
    // comma_child_ctrl: both oracles
    digest(
        "comma_child_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module c #(parameter int K = 1) (); initial $display("DIGEST=%0d", K); endmodule
module m #(parameter int N = p::W) (); c #(.K(N)) k();  endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4",
    );
}

#[test]
fn shape_clog2() {
    // comma_clog2: both oracles
    digest(
        "comma_clog2",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = $clog2(W * 8)) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "5",
    );
}

#[test]
fn shape_clog2_ctrl() {
    // comma_clog2_ctrl: both oracles
    digest(
        "comma_clog2_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int X = $clog2(p::W * 8)) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "5",
    );
}

#[test]
fn shape_concat() {
    // comma_concat: both oracles
    digest(
        "comma_concat",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [11:0] X = {Mask, Dflt}) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "ca5",
    );
}

#[test]
fn shape_concat_ctrl() {
    // comma_concat_ctrl: both oracles
    digest(
        "comma_concat_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [11:0] X = {p::Mask, p::Dflt}) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "ca5",
    );
}

#[test]
fn shape_cond() {
    // comma_cond: both oracles
    digest(
        "comma_cond",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = (W > 2) ? W : 0) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4",
    );
}

#[test]
fn shape_cond_ctrl() {
    // comma_cond_ctrl: both oracles
    digest(
        "comma_cond_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int X = (p::W > 2) ? p::W : 0) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4",
    );
}

#[test]
fn shape_enum() {
    // comma_enum: both oracles
    digest(
        "comma_enum",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter e_t X = E1) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "1",
    );
}

#[test]
fn shape_enumint() {
    // comma_enumint: both oracles
    digest(
        "comma_enumint",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = E1 + 1) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "2",
    );
}

#[test]
fn shape_enumint_ctrl() {
    // comma_enumint_ctrl: both oracles
    digest(
        "comma_enumint_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int X = p::E1 + 1) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "2",
    );
}

#[test]
fn shape_expr() {
    // comma_expr: both oracles
    digest(
        "comma_expr",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = W * 2 + 1) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "9",
    );
}

#[test]
fn shape_expr_ctrl() {
    // comma_expr_ctrl: both oracles
    digest(
        "comma_expr_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int X = p::W * 2 + 1) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "9",
    );
}

#[test]
fn shape_gen() {
    // comma_gen: split (vita = verilator)
    digest(
        "comma_gen",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int N = W) (); logic [N-1:0] v; genvar i; generate for (i = 0; i < N; i++) begin : g assign v[i] = i[0]; end endgenerate initial $display("DIGEST=%b", v); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "1010",
    );
}

#[test]
fn shape_gen_ctrl() {
    // comma_gen_ctrl: split (vita = verilator)
    digest(
        "comma_gen_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int N = p::W) (); logic [N-1:0] v; genvar i; generate for (i = 0; i < N; i++) begin : g assign v[i] = i[0]; end endgenerate initial $display("DIGEST=%b", v); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "1010",
    );
}

#[test]
fn shape_lp() {
    // comma_lp: both oracles
    digest(
        "comma_lp",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int A = 2, localparam int L = W * A) ();  initial $display("DIGEST=%0d", L); endmodule
module tb; m u(); m #(.A(3)) v(); initial begin #2 $finish; end endmodule
"#,
        "12|8",
    );
}

#[test]
fn shape_lp_ctrl() {
    // comma_lp_ctrl: both oracles
    digest(
        "comma_lp_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int A = 2, localparam int L = p::W * A) ();  initial $display("DIGEST=%0d", L); endmodule
module tb; m u(); m #(.A(3)) v(); initial begin #2 $finish; end endmodule
"#,
        "12|8",
    );
}

#[test]
fn shape_override() {
    // comma_override: both oracles
    digest(
        "comma_override",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [7:0] X = Dflt) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); m #(.X(8'h11)) v(); initial begin #2 $finish; end endmodule
"#,
        "11|a5",
    );
}

#[test]
fn shape_override_ctrl() {
    // comma_override_ctrl: both oracles
    digest(
        "comma_override_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [7:0] X = p::Dflt) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); m #(.X(8'h11)) v(); initial begin #2 $finish; end endmodule
"#,
        "11|a5",
    );
}

#[test]
fn shape_port() {
    // comma_port: both oracles
    digest(
        "comma_port",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int N = W) (input logic [N-1:0] a);  initial $display("DIGEST=%h", a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #2 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn shape_port_ctrl() {
    // comma_port_ctrl: both oracles
    digest(
        "comma_port_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int N = p::W) (input logic [N-1:0] a);  initial $display("DIGEST=%h", a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #2 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn shape_portdirect() {
    // comma_portdirect: both oracles
    digest(
        "comma_portdirect",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int N = 1) (input logic [W-1:0] a);  initial $display("DIGEST=%h", a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #2 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn shape_portdirect_ctrl() {
    // comma_portdirect_ctrl: both oracles
    digest(
        "comma_portdirect_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int N = 1) (input logic [p::W-1:0] a);  initial $display("DIGEST=%h", a); endmodule
module tb; logic [3:0] a = 4'h7; m u(a); initial begin #2 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn shape_posovr() {
    // comma_posovr: both oracles
    digest(
        "comma_posovr",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [7:0] X = Dflt, parameter int Y = W) ();  initial $display("DIGEST=%h %0d", X, Y); endmodule
module tb; m u(); m #(8'h22) v(); initial begin #2 $finish; end endmodule
"#,
        "22 4|a5 4",
    );
}

#[test]
fn shape_posovr_ctrl() {
    // comma_posovr_ctrl: both oracles
    digest(
        "comma_posovr_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [7:0] X = p::Dflt, parameter int Y = p::W) ();  initial $display("DIGEST=%h %0d", X, Y); endmodule
module tb; m u(); m #(8'h22) v(); initial begin #2 $finish; end endmodule
"#,
        "22 4|a5 4",
    );
}

#[test]
fn shape_qonly() {
    // comma_qonly: both oracles
    digest(
        "comma_qonly",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = Q) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "9",
    );
}

#[test]
fn shape_qonly_ctrl() {
    // comma_qonly_ctrl: both oracles
    digest(
        "comma_qonly_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int X = q::Q) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "9",
    );
}

#[test]
fn shape_range() {
    // comma_range: both oracles
    digest(
        "comma_range",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [W-1:0] X = 4'h9) ();  initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "9 4",
    );
}

#[test]
fn shape_range_ctrl() {
    // comma_range_ctrl: both oracles
    digest(
        "comma_range_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [p::W-1:0] X = 4'h9) ();  initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "9 4",
    );
}

#[test]
fn shape_rangedflt() {
    // comma_rangedflt: both oracles
    digest(
        "comma_rangedflt",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [W-1:0] X = Mask) ();  initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "c 4",
    );
}

#[test]
fn shape_rangedflt_ctrl() {
    // comma_rangedflt_ctrl: both oracles
    digest(
        "comma_rangedflt_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [p::W-1:0] X = p::Mask) ();  initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "c 4",
    );
}

#[test]
fn shape_scalar() {
    // comma_scalar: both oracles
    digest(
        "comma_scalar",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [7:0] X = Dflt) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "a5",
    );
}

#[test]
fn shape_scalar_ctrl() {
    // comma_scalar_ctrl: both oracles
    digest(
        "comma_scalar_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [7:0] X = p::Dflt) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "a5",
    );
}

#[test]
fn shape_sel() {
    // comma_sel: both oracles
    digest(
        "comma_sel",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter logic [3:0] X = Dflt[7:4]) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "a",
    );
}

#[test]
fn shape_sel_ctrl() {
    // comma_sel_ctrl: both oracles
    digest(
        "comma_sel_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter logic [3:0] X = p::Dflt[7:4]) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "a",
    );
}

#[test]
fn shape_shadowbody() {
    // comma_shadowbody: verilator
    loud(
        "comma_shadowbody",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int X = W) (); localparam int W = 9; initial $display("DIGEST=%0d %0d", X, W); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "parameter `X` value is not a constant: undefined name `W` is not a con",
    );
}

#[test]
fn shape_shadowhdr() {
    // comma_shadowhdr: both oracles
    digest(
        "comma_shadowhdr",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int W = 7, parameter int X = W) ();  initial $display("DIGEST=%0d", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "7",
    );
}

#[test]
fn shape_sibling() {
    // comma_sibling: both oracles
    digest(
        "comma_sibling",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter int A = W, parameter int B = A + 1) ();  initial $display("DIGEST=%0d %0d", A, B); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4 5",
    );
}

#[test]
fn shape_sibling_ctrl() {
    // comma_sibling_ctrl: both oracles
    digest(
        "comma_sibling_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter int A = p::W, parameter int B = A + 1) ();  initial $display("DIGEST=%0d %0d", A, B); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4 5",
    );
}

#[test]
fn shape_typed() {
    // comma_typed: both oracles
    digest(
        "comma_typed",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter perm_t X = P) ();  initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "3c",
    );
}

#[test]
fn shape_untyped() {
    // comma_untyped: both oracles
    digest(
        "comma_untyped",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m import p::*, q::Q; #(parameter X = W) ();  initial $display("DIGEST=%0d %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4 32",
    );
}

#[test]
fn shape_untyped_ctrl() {
    // comma_untyped_ctrl: both oracles
    digest(
        "comma_untyped_ctrl",
        r#"package p; typedef logic [7:0] perm_t; typedef enum logic [1:0] {E0, E1, E2} e_t;
parameter int W = 4; parameter logic [7:0] Dflt = 8'hA5; parameter logic [W-1:0] Mask = 4'hC; parameter perm_t P = 8'h3C; endpackage
package q; parameter int Q = 9; endpackage
module m #(parameter X = p::W) ();  initial $display("DIGEST=%0d %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #2 $finish; end endmodule
"#,
        "4 32",
    );
}
