//! §3 ⑤ ⓓ: the SCOPE rules of a constant-width packed-struct member (the second
//! half of `struct_member_param_width.rs`): a package `parameter` through a
//! wildcard / explicit / scoped read and a header import, a body `parameter`
//! with and without an ANSI header (§6.20.1), a header parameter (overridable —
//! loud), a name shadowed by a port / a generate-local `localparam` / a local
//! `localparam` over a wildcard, two wildcards exporting one name, an explicit
//! import over a wildcard (§26.8), an ascending / non-zero-LSB member with a
//! constant bound, sub-selects, a pattern, a union, a record, a multi-dim member,
//! and the other two consumers of the same table (an enum label, a constant
//! generate-array index).
//!
//! Every expected value is the census oracle line (iverilog 13.0 `-g2012` and
//! verilator 5.050 agree unless the comment says otherwise).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stru_{}_{n}", std::process::id()));
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

/// Every `DIGEST=` line, in emission order, joined by `|` (the census harness format).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
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
fn ctl_misc() {
    // ctl_asc: both oracles
    digest(
        "ctl_asc",
        r#"module tb;
  typedef struct packed { logic [0:5] a; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = 6'b101101;
    $display("DIGEST=%0d %b %b %b %b", $bits(s), s.a, s.a[0], s.a[1:3], s);
    s.a[0] = 1'b0; s.a[4:5] = 2'b11;
    $display("DIGEST=%b", s);
  end

  initial #5 $finish;
endmodule
"#,
        "7 101101 1 011 1011010|0011110",
    );
    // ctl_nzl: both oracles
    digest(
        "ctl_nzl",
        r#"module tb;
  typedef struct packed { logic [7:4] a; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = 4'b1011;
    $display("DIGEST=%0d %b %b %b %b", $bits(s), s.a, s.a[4], s.a[7:6], s);
    s.a[5] = 1'b0; s.a[7:6] = 2'b00;
    $display("DIGEST=%b", s);
  end

  initial #5 $finish;
endmodule
"#,
        "5 1011 1 10 10110|00010",
    );
}

#[test]
fn ctl_read() {
    // ctl_read_asc: both oracles
    digest(
        "ctl_read_asc",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; logic [0:3] q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = 4'b1011; o.U0 = 1;
    $display("DIGEST=%h %b %0d", o, o.q, $bits(o));
  end

  initial #5 $finish;
endmodule
"#,
        "06c 1011 9",
    );
    // ctl_read_nzl: both oracles
    digest(
        "ctl_read_nzl",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; logic [7:4] q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = 4'b1011; o.U0 = 1;
    $display("DIGEST=%h %b %0d", o, o.q, $bits(o));
  end

  initial #5 $finish;
endmodule
"#,
        "06c 1011 9",
    );
}

#[test]
fn ctl_sub() {
    // ctl_sub: both oracles
    digest(
        "ctl_sub",
        r#"module tb;
  typedef struct packed { logic [5:0] a; logic [6:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = 6'b101101;
    $display("DIGEST=%0d %b %b %b", $bits(s), s.a[1], s.a[3:1], s);
    s.a[0] = 1; s.a[5:4] = 2'b00;
    $display("DIGEST=%b", s);
  end

  initial #5 $finish;
endmodule
"#,
        "14 0 110 10110100000000|00110100000000",
    );
    // ctl_sub_asc: both oracles
    digest(
        "ctl_sub_asc",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; logic [0:3] q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = 4'b1011;
    $display("DIGEST=%b %b %b", o.q[1], o.q[0:2], o);
    o.q[0] = 1'b0; o.q[2:3] = 2'b10;
    $display("DIGEST=%b %b", o.q, o);
  end

  initial #5 $finish;
endmodule
"#,
        "0 101 000101100|0010 000001000",
    );
    // ctl_sub_nzl: both oracles
    digest(
        "ctl_sub_nzl",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; logic [7:4] q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = 4'b1011;
    $display("DIGEST=%b %b %b", o.q[6], o.q[7:5], o);
    o.q[7] = 1'b0; o.q[5:4] = 2'b10;
    $display("DIGEST=%b %b", o.q, o);
  end

  initial #5 $finish;
endmodule
"#,
        "0 101 000101100|0010 000001000",
    );
}

#[test]
fn ns_read() {
    // ns_read_asc: both oracles
    digest(
        "ns_read_asc",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [0:3] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = 4'b1011; o.i.U0 = 1;
    $display("DIGEST=%h %b %h %0d %0d", o, o.i.q, o.i, $bits(o), $bits(o.i));
  end

  initial #5 $finish;
endmodule
"#,
        "06c 1011 36 9 6",
    );
    // ns_read_nzl: both oracles
    digest(
        "ns_read_nzl",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [7:4] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = 4'b1011; o.i.U0 = 1;
    $display("DIGEST=%h %b %h %0d %0d", o, o.i.q, o.i, $bits(o), $bits(o.i));
  end

  initial #5 $finish;
endmodule
"#,
        "06c 1011 36 9 6",
    );
}

#[test]
fn ns_sub() {
    // ns_sub_asc: both oracles
    digest(
        "ns_sub_asc",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [0:3] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = 4'b1011;
    $display("DIGEST=%b %b %b", o.i.q[1], o.i.q[0:2], o);
    o.i.q[0] = 1'b0; o.i.q[2:3] = 2'b10;
    $display("DIGEST=%b %b", o.i.q, o);
  end

  initial #5 $finish;
endmodule
"#,
        "0 101 000101100|0010 000001000",
    );
    // ns_sub_nzl: both oracles
    digest(
        "ns_sub_nzl",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [7:4] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = 4'b1011;
    $display("DIGEST=%b %b %b", o.i.q[6], o.i.q[7:5], o);
    o.i.q[7] = 1'b0; o.i.q[5:4] = 2'b10;
    $display("DIGEST=%b %b", o.i.q, o);
  end

  initial #5 $finish;
endmodule
"#,
        "0 101 000101100|0010 000001000",
    );
}

#[test]
fn pw_misc() {
    // pw_asc: both oracles
    digest(
        "pw_asc",
        r#"module tb;
  localparam int W = 6;
  typedef struct packed { logic [0:W-1] a; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = 6'b101101;
    $display("DIGEST=%0d %b %b %b %b", $bits(s), s.a, s.a[0], s.a[1:3], s);
    s.a[0] = 1'b0; s.a[4:5] = 2'b11;
    $display("DIGEST=%b", s);
  end

  initial #5 $finish;
endmodule
"#,
        "7 101101 1 011 1011010|0011110",
    );
    // pw_bodyparam_hdr: both oracles
    digest(
        "pw_bodyparam_hdr",
        r#"module c #(parameter int X = 1) ();
  parameter int W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end
endmodule
module tb; c #(.X(2)) u(); initial #5 $finish; endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_bodyparam_nohdr: both oracles (§4.5.431: a body `parameter` of a header-less
    // module is overridable, so the member is laid out per instance — was a loud
    // decline)
    digest(
        "pw_bodyparam_nohdr",
        r#"module tb;
  parameter int W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_enum_label: both oracles
    digest(
        "pw_enum_label",
        r#"module tb;
  localparam int K = 4;
  typedef enum logic [3:0] { A = K, B = K/2, C = $clog2(K)+4 } e_t;
  e_t e;
  initial begin
    e = B;
    $display("DIGEST=%0d %s %0d %0d", e, e.name(), A, C);
  end

  initial #5 $finish;
endmodule
"#,
        "2 B 4 6",
    );
    // pw_explicit_over_wild: split — IEEE §26.8: the explicit import wins over the wildcard; verilator takes the wildcard
    digest(
        "pw_explicit_over_wild",
        r#"package p; parameter int W = 6; endpackage
package q; parameter int W = 3; endpackage
module tb;
  import p::*; import q::W;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "8 3 e1 1",
    );
    // pw_gen_index: both oracles
    digest(
        "pw_gen_index",
        r#"module tb;
  localparam int K = 4;
  localparam int H = K/2;
  for (genvar i = 0; i < 3; i++) begin : g
    logic [3:0] x;
    initial x = i + 1;
  end
  initial #1 $display("DIGEST=%0d %0d %0d", g[H].x, g[$clog2(K)].x, g[K/K-1].x);

  initial #5 $finish;
endmodule
"#,
        "3 3 1",
    );
    // pw_hdr_import: both oracles
    digest(
        "pw_hdr_import",
        r#"package p; parameter int W = 6; endpackage
module c import p::*; ();
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end
endmodule
module tb; c u(); initial #5 $finish; endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_hdrparam: both oracles (§4.5.431: laid out per instance — was a loud decline)
    digest(
        "pw_hdrparam",
        r#"module c #(parameter int W = 6) ();
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end
endmodule
module tb; c #(.W(3)) u(); initial #5 $finish; endmodule
"#,
        "8 3 e1 1",
    );
    // pw_local_over_wild: both oracles
    digest(
        "pw_local_over_wild",
        r#"package p; parameter int W = 6; endpackage
module tb;
  import p::*;
  localparam int W = 3;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "8 3 e1 1",
    );
    // pw_md_member: both oracles
    digest(
        "pw_md_member",
        r#"module tb;
  localparam int W = 2;
  typedef struct packed { logic [W-1:0][3:0] m; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.m = 8'hA5;
    $display("DIGEST=%0d %h %h", $bits(s), s.m[1], s);
  end

  initial #5 $finish;
endmodule
"#,
        "9 a 14a",
    );
    // pw_neg_width: both oracles
    digest(
        "pw_neg_width",
        r#"module tb;
  localparam int W = 0;
  typedef struct packed { logic [W-1:0] a; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1;
    $display("DIGEST=%0d %b", $bits(s), s);
  end

  initial #5 $finish;
endmodule
"#,
        "3 110",
    );
    // pw_nzl: both oracles
    digest(
        "pw_nzl",
        r#"module tb;
  localparam int W = 4;
  typedef struct packed { logic [W+3:4] a; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = 4'b1011;
    $display("DIGEST=%0d %b %b %b %b", $bits(s), s.a, s.a[4], s.a[7:6], s);
    s.a[5] = 1'b0; s.a[7:6] = 2'b00;
    $display("DIGEST=%b", s);
  end

  initial #5 $finish;
endmodule
"#,
        "5 1011 1 10 10110|00010",
    );
    // pw_pattern: verilator
    digest(
        "pw_pattern",
        r#"module tb;
  localparam int W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '{a: '1, b: 7'h55, c: 0};
    $display("DIGEST=%h", s);
    s = '{default: '0};
    $display("DIGEST=%h", s);
  end

  initial #5 $finish;
endmodule
"#,
        "3faa|0000",
    );
    // pw_pkg_localparam_wild: both oracles
    digest(
        "pw_pkg_localparam_wild",
        r#"package p; localparam int W = 6; endpackage
module tb;
  import p::*;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_pkg_param_explicit: both oracles
    digest(
        "pw_pkg_param_explicit",
        r#"package p; parameter int W = 6; endpackage
module tb;
  import p::W;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_pkg_param_scoped: both oracles
    digest(
        "pw_pkg_param_scoped",
        r#"package p; parameter int W = 6; endpackage
module tb;
  typedef struct packed { logic [p::W-1:0] a; logic [p::W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_pkg_param_wild: both oracles
    digest(
        "pw_pkg_param_wild",
        r#"package p; parameter int W = 6; endpackage
module tb;
  import p::*;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_pkg_typedef_vec: both oracles
    digest(
        "pw_pkg_typedef_vec",
        r#"package p; parameter int unsigned W = 6; typedef logic [W-1:0] t_a; typedef logic [W:0] t_b; typedef struct packed { t_a a; t_b b; logic c; } s_t; endpackage
module tb;
  import p::*;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_shadow_gen_local: both oracles
    digest(
        "pw_shadow_gen_local",
        r#"module tb;
  localparam int W = 6;
  if (1) begin : g
    localparam int W = 3;
    typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
    s_t s;
    initial begin
      s = '0; s.a = '1; s.c = 1;
      $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
    end
  end
    typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } m_t;
  m_t m;
  initial begin m = '0; m.a = '1; #1 $display("DIGEST=%0d %h", $bits(m), m); end

  initial #5 $finish;
endmodule
"#,
        "8 3 e1 1|14 3f00",
    );
    // pw_shadow_port: verilator
    loud(
        "pw_shadow_port",
        r#"package p; parameter int W = 6; endpackage
module c import p::*; (input logic [1:0] W);
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end
endmodule
module tb; c u(.W(2'b11)); initial #5 $finish; endmodule
"#,
        "expected struct member width must be a named integer type or a constan",
    );
    // pw_shadow_var: both oracles
    digest(
        "pw_shadow_var",
        r#"module tb;
  localparam int W = 6;
  logic [7:0] W2;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_two_wild_diff: verilator
    loud(
        "pw_two_wild_diff",
        r#"package p; parameter int W = 6; endpackage
package q; parameter int W = 3; endpackage
module tb;
  import p::*; import q::*;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected struct member width must be a named integer type or a constan",
    );
    // pw_two_wild_same: verilator
    digest(
        "pw_two_wild_same",
        r#"package p; parameter int W = 6; endpackage
package q; parameter int W = 6; endpackage
module tb;
  import p::*; import q::*;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_union: both oracles
    digest(
        "pw_union",
        r#"module tb;
  localparam int W = 6;
  typedef union packed { logic [W-1:0] a; logic [5:0] b; } u_t;
  u_t u;
  initial begin
    u = '0; u.a = 6'b110101;
    $display("DIGEST=%0d %b %b", $bits(u), u.b, u);
  end

  initial #5 $finish;
endmodule
"#,
        "6 110101 110101",
    );
    // pw_unpacked_record: verilator
    digest(
        "pw_unpacked_record",
        r#"module tb;
  localparam int W = 6;
  typedef struct { logic [W-1:0] a; int n; } r_t;
  r_t r;
  initial begin
    r.a = '1; r.n = 3;
    $display("DIGEST=%0d %h %0d", $bits(r.a), r.a, r.n);
  end

  initial #5 $finish;
endmodule
"#,
        "6 3f 3",
    );
}

#[test]
fn pw_sub() {
    // pw_sub: both oracles
    digest(
        "pw_sub",
        r#"module tb;
  localparam int W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = 6'b101101;
    $display("DIGEST=%0d %b %b %b", $bits(s), s.a[1], s.a[3:1], s);
    s.a[0] = 1; s.a[5:4] = 2'b00;
    $display("DIGEST=%b", s);
  end

  initial #5 $finish;
endmodule
"#,
        "14 0 110 10110100000000|00110100000000",
    );
}

/// Review B (§4.5.414) — the three BLOCKING findings, fixed on the delta and pinned:
/// a signing cast folds its operand SELF-determined (§11.8.1; both oracles), the
/// widened constant table keeps every localparam PRE recorded (an unfoldable range
/// bound, `time`), and a `genvar` shadows an imported constant of its name.
#[test]
fn review_b_pins() {
    // b_sign_ctx: both oracles — `unsigned'(A + 8'h01)` under an 18-/72-bit target wraps at 8 bits
    digest(
        "b_sign_ctx",
        r#"module tb;
  localparam logic [7:0] A = 8'hFF;
  localparam logic [71:0] R = unsigned'(A + 8'h01);
  localparam logic [15:0] R2 = unsigned'(A + 8'h01);
  initial begin
    $display("DIGEST=%h %h", R, R2);
    #1 $finish;
  end
endmodule
"#,
        "000000000000000000 0000",
    );
    // b_fits_range / b_fits_time: both oracles — PRE-correct enum labels from a range-typed / time localparam
    digest(
        "b_fits_range",
        r#"module tb;
  localparam int W = 8;
  localparam logic [W-1:0] X = 5;
  localparam time T = 3;
  typedef enum logic [7:0] { A = X, B } e_t;
  typedef enum logic [7:0] { C = T, D } f_t;
  e_t a; f_t c;
  initial begin a = A; c = C; $display("DIGEST=%0d %s %0d %s", a, a.name(), c, c.name()); #1 $finish; end
endmodule
"#,
        "5 A 3 C",
    );
    // b_fits_gidx: both oracles — a range-typed localparam as a constant generate index (PRE-correct)
    digest(
        "b_fits_gidx",
        r#"module sub(output logic [31:0] o); assign o = 7; endmodule
module tb;
  localparam int W = 4;
  localparam logic [W-1:0] P = 1;
  genvar g;
  logic [31:0] r;
  generate for (g = 0; g < 3; g = g + 1) begin : gg
    sub u(.o());
  end endgenerate
  initial begin
    r = gg[P].u.o;
    $display("DIGEST=%0d", r);
    #1 $finish;
  end
endmodule
"#,
        "7",
    );
    // k03 (review A, delta re-score): a HEADER genvar shadows a same-named localparam for the
    // loop only (§27.4) — `g[i].x` after the loop folds the localparam (both oracles)
    digest(
        "k03_gen_genvar_name",
        r#"module tb;
  localparam int i = 2;
  genvar j;
  for (genvar i = 0; i < 3; i++) begin : g
    logic [3:0] x;
    initial x = i + 1;
  end
  for (j = 0; j < 3; j++) begin : h
    logic [3:0] x;
    initial x = j + 5;
  end
  initial #1 $display("DIGEST=%0d %0d %0d", g[i].x, h[i].x, h[i-1].x);
  initial #5 $finish;
endmodule
"#,
        "3 7 6",
    );
    // b_genvar_shadow2: a genvar named like an imported package constant must not fold that
    // constant as the generate index (POST answered p's 2 for every iteration); loud as PRE
    // (both oracles print the per-iteration 100/101/102 — the loop-scoped read is pre-existing loud)
    loud(
        "b_genvar_shadow2",
        r#"package p; parameter int gi = 2; endpackage
module sub #(parameter int V = 0) (output logic [31:0] o); assign o = 32'd100 + V; endmodule
module tb;
  import p::*;
  genvar gi;
  generate for (gi = 0; gi < 3; gi = gi + 1) begin : gg
    sub #(.V(gi)) u(.o());
    initial $display("DIGEST=%0d", gg[gi].u.o);
  end endgenerate
  initial begin #1 $finish; end
endmodule
"#,
        "a constant generate-array index",
    );
}
