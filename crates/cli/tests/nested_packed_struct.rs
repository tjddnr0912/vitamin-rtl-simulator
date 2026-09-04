//! §3 ⑤ ⓓ: NESTED packed structs — a `typedef struct packed` member whose type is
//! another packed struct/union typedef (`perms_t perms;`, ibex_cheriot_pkg's
//! `cap_t` / `decoded_cap_t` / `bound_result_t`). The parser lays the member out
//! FLAT at the nested type's total width and records the nested type key on the
//! field, so a chain `s.perms.EX` (any depth) resolves to the leaf's geometry at
//! the summed offset — read, write, sub-select, `$bits`, ports, array elements,
//! function locals, generate blocks, a `'{…}` value that recurses per member, a
//! `default:` fill, a struct-typed cast `cap_t'(e)` (size + signing, folded in a
//! constant), and a package parameter of such a type.
//!
//! Every expected value is the census oracle line (iverilog 13.0 `-g2012` and
//! verilator 5.050 agree unless the comment says which one ran; iverilog rejects
//! every `'{…}` pattern with a nested struct and a method call through a chain).
//! Cells whose comment names a PRE-EXISTING answer pin the observed value.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_nest_{}_{n}", std::process::id()));
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
    // ctl_bits_scoped_type: both oracles
    loud(
        "ctl_bits_scoped_type",
        r#"package p;
  typedef struct packed { logic [1:0] cor; logic [3:0] perms; logic valid; } cap_t;
endpackage
module tb;
  p::cap_t c;
  initial begin
    c = '0; c.perms = 4'b1010;
    $display("DIGEST=%h %0d", c, $bits(p::cap_t));
  end

  initial #5 $finish;
endmodule
"#,
        "`p::cap_t` does not name a package constant or variable (v7 supports p",
    );
    // ctl_hier_read: both oracles
    loud(
        "ctl_hier_read",
        r#"module tb;
  c u();
  initial begin
    #1 $display("DIGEST=%h %b", u.c, u.c.q);
  end
endmodule
module c;
  typedef struct packed { logic [1:0] cor; logic [3:0] q; logic valid; } cap_t;
  cap_t c;
  initial begin c = '0; c.q = 4'b0011; end

  initial #5 $finish;
endmodule
"#,
        "undeclared hierarchical name `u.c.q` (no such cross-instance net or pa",
    );
    // ctl_two_state: split — PRE-EXISTING (§2): an x/z WRITE into a 2-state member of a 4-state struct is kept (iverilog coerces to 0, §7.2.1); pinned as observed, not as correct
    digest(
        "ctl_two_state",
        r#"module tb;
  typedef struct packed { logic a; bit [3:0] q; bit s; } out_t;
  out_t o;
  initial begin
    $display("DIGEST=%b", o);
    o.q = 4'bx1z0;
    $display("DIGEST=%b", o);
    o = 6'bx1z0x1;
    $display("DIGEST=%b %b", o, o.q);
  end

  initial #5 $finish;
endmodule
"#,
        "xxxxxx|xx1z0x|x1z0x1 1z0x",
    );
}

#[test]
fn ctl_read() {
    // ctl_read_byte: both oracles
    digest(
        "ctl_read_byte",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; byte q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = -3; o.U0 = 1;
    $display("DIGEST=%h %0d %0d", o, o.q, $bits(o));
  end

  initial #5 $finish;
endmodule
"#,
        "07f4 -3 13",
    );
    // ctl_read_enum: both oracles
    digest(
        "ctl_read_enum",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; e_t q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = C; o.U0 = 1;
    $display("DIGEST=%h %0d %0d", o, o.q, $bits(o));
  end

  initial #5 $finish;
endmodule
"#,
        "18 2 7",
    );
    // ctl_read_vec: both oracles
    digest(
        "ctl_read_vec",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; logic [3:0] q; logic SE; logic valid; } out_t;
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
    // ctl_sub_vec: both oracles
    digest(
        "ctl_sub_vec",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic [1:0] cor; logic U0; logic [3:0] q; logic SE; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.q = 4'b1011;
    $display("DIGEST=%b %b %b", o.q[2], o.q[3:1], o);
    o.q[3] = 1'b0; o.q[1:0] = 2'b10;
    $display("DIGEST=%b %b", o.q, o);
  end

  initial #5 $finish;
endmodule
"#,
        "0 101 000101100|0010 000001000",
    );
}

#[test]
fn ns_misc() {
    // ns_always_comb: both oracles
    digest(
        "ns_always_comb",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c, d;
  always_comb begin
    d = c;
    d.perms.q = c.perms.q ^ 2'b11;
    d.perms.U0 = c.valid;
  end
  initial begin
    c = '0; c.perms.q = 2'b01; c.valid = 1;
    #1 $display("DIGEST=%h %h", c, d);
  end

  initial #5 $finish;
endmodule
"#,
        "03 15",
    );
    // ns_array_elem: verilator
    digest(
        "ns_array_elem",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t arr[2];
  initial begin
    arr[0] = '0; arr[1] = '0;
    arr[1].perms.q = 2'b11; arr[0].cor = 2'b10; arr[0].perms = '{U0: 1, SE: 0, q: 2'b01};
    $display("DIGEST=%h %h %b %b %h", arr[0], arr[1], arr[1].perms.q, arr[0].perms.U0, arr[0].perms);
    arr[1] = '{cor: 2'b01, perms: '{default: '1}, valid: 0};
    $display("DIGEST=%h %b", arr[1], arr[1].perms.q[0]);
  end

  initial #5 $finish;
endmodule
"#,
        "52 06 11 1 9|3e 1",
    );
    // ns_block_local_typedef: both oracles
    digest(
        "ns_block_local_typedef",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin : b
    typedef struct packed { logic z; perms_t p; } loc_t;
    loc_t l;
    l = '0; l.p.q = 2'b10; c = '0; c.perms.q = 2'b01;
    $display("DIGEST=%b %b %b", l, l.p.q, c.perms.q);
  end

  initial #5 $finish;
endmodule
"#,
        "00010 10 01",
    );
    // ns_compare_concat: verilator
    digest(
        "ns_compare_concat",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c, d;
  initial begin
    c = '{cor: 2'b10, perms: '{U0: 1, SE: 0, q: 2'b11}, valid: 1};
    d = c; d.perms.SE = 1;
    $display("DIGEST=%b %b %h %b", c == d, c.perms == d.perms, {c.perms.q, d.perms.q}, c.perms != 4'b1011);
  end

  initial #5 $finish;
endmodule
"#,
        "0 0 f 0",
    );
    // ns_cross_pkg: verilator
    digest(
        "ns_cross_pkg",
        r#"package q;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
endpackage
package p;
  import q::*;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  typedef struct packed { logic [1:0] cor; q::perms_t perms; logic valid; } cap2_t;
endpackage
module tb;
  import p::*;
  cap_t c; cap2_t d;
  initial begin
    c = '0; c.perms.q = 2'b11; d = '0; d.perms.SE = 1;
    $display("DIGEST=%h %b %h %b", c, c.perms.q, d, d.perms.SE);
  end

  initial #5 $finish;
endmodule
"#,
        "06 11 08 1",
    );
    // ns_enum_method: verilator
    loud(
        "ns_enum_method",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { e_t e; logic [2:0] q; } in_t;
  typedef struct packed { in_t i; logic m; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.e = C; o.i.q = 3'b101;
    $display("DIGEST=%h %h %0d %s", o, o.i.e, o.i.e, o.i.e.name());
  end

  initial #5 $finish;
endmodule
"#,
        "unsupported hierarchical function call `o.i.e.name` (the callee must b",
    );
    // ns_explicit_import: both oracles
    digest(
        "ns_explicit_import",
        r#"package p;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
endpackage
module tb;
  import p::cap_t;
  cap_t c;
  initial begin
    c = '0; c.perms.q = 2'b11;
    $display("DIGEST=%h %b", c, c.perms.q);
  end

  initial #5 $finish;
endmodule
"#,
        "06 11",
    );
    // ns_func: verilator
    digest(
        "ns_func",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  function automatic logic [1:0] getq(input cap_t x);
    return x.perms.q;
  endfunction
  function automatic cap_t mk(input logic [1:0] q);
    cap_t r;
    r = '{cor: 2'b11, perms: '{U0: 0, SE: 1, q: q}, valid: 1};
    r.perms.U0 = 1;
    return r;
  endfunction
  cap_t c;
  initial begin
    c = mk(2'b10);
    $display("DIGEST=%h %b", c, getq(c));
  end

  initial #5 $finish;
endmodule
"#,
        "7d 10",
    );
    // ns_generate: both oracles
    digest(
        "ns_generate",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  if (1) begin : g
    cap_t c;
    initial begin
      c = '0; c.perms.q = 2'b11;
      $display("DIGEST=%h %b", c, c.perms.q);
    end
  end

  initial #5 $finish;
endmodule
"#,
        "06 11",
    );
    // ns_hier_read: both oracles
    loud(
        "ns_hier_read",
        r#"package tb_pkg;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
endpackage
module tb;
  import tb_pkg::*;
  c u();
  initial begin
    #1 $display("DIGEST=%h %b", u.c, u.c.perms.q);
  end
endmodule
module c;
  import tb_pkg::*;
  cap_t c;
  initial begin c = '0; c.perms.q = 2'b11; end

  initial #5 $finish;
endmodule
"#,
        "undeclared hierarchical name `u.c.perms.q` (no such cross-instance net",
    );
    // ns_md_member: verilator
    loud(
        "ns_md_member",
        r#"module tb;
  typedef struct packed { logic a; logic [1:0] q; } in_t;
  typedef struct packed { in_t i [1:0]; logic m; } out_t;
  out_t o;
  initial begin
    o = '0;
    $display("DIGEST=%b", o);
  end

  initial #5 $finish;
endmodule
"#,
        "expected ';', found '['",
    );
    // ns_port: both oracles
    digest(
        "ns_port",
        r#"package p;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
endpackage
module c import p::*; (input cap_t ci, output cap_t co);
  assign co.perms.q = ci.perms.q + 1;
  assign co.cor = ci.cor;
  assign co.perms.U0 = ~ci.perms.U0;
  assign co.perms.SE = ci.perms.SE;
  assign co.valid = 1;
endmodule
module tb;
  import p::*;
  cap_t a, b;
  c u(.ci(a), .co(b));
  initial begin
    a = '0; a.perms.q = 2'b01; a.cor = 2'b11;
    #1 $display("DIGEST=%h %h %b", a, b, b.perms.q);
  end

  initial #5 $finish;
endmodule
"#,
        "62 75 10",
    );
    // ns_same_name_collision: both oracles
    digest(
        "ns_same_name_collision",
        r#"package pa;
  typedef struct packed { logic [3:0] a; } in_t;
  typedef struct packed { in_t i; logic v; } out_t;
endpackage
package pb;
  typedef struct packed { logic [7:0] a; } in_t;
  typedef struct packed { in_t i; logic v; } out_t;
endpackage
module tb;
  pa::out_t x; pb::out_t y;
  initial begin
    x = '0; y = '0; x.i.a = '1; y.i.a = 8'h81;
    $display("DIGEST=%h %h %0d %0d", x, y, $bits(x.i.a), $bits(y.i.a));
  end

  initial #5 $finish;
endmodule
"#,
        "1e 102 4 8",
    );
    // ns_scoped_type: verilator
    digest(
        "ns_scoped_type",
        r#"package p;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
endpackage
module tb;
  p::cap_t c;
  initial begin
    c = '0; c.perms.q = 2'b11; c.perms = '{U0: 1, SE: 0, q: 2'b10};
    $display("DIGEST=%h %b", c, c.perms.q);
  end

  initial #5 $finish;
endmodule
"#,
        "14 10",
    );
    // ns_struct_array_member: both oracles
    loud(
        "ns_struct_array_member",
        r#"module tb;
  typedef struct packed { logic a; logic [1:0] q; } in_t;
  typedef struct packed { in_t [1:0] i; logic m; } out_t;
  out_t o;
  initial begin
    o = '0; o.i[1].q = 2'b11;
    $display("DIGEST=%b %b", o, o.i[1].q);
  end

  initial #5 $finish;
endmodule
"#,
        "expected identifier, found '['",
    );
    // ns_struct_in_union: both oracles
    digest(
        "ns_struct_in_union",
        r#"module tb;
  typedef struct packed { logic a; logic [2:0] q; } s_t;
  typedef union packed { s_t s; logic [3:0] w; } u_t;
  u_t u;
  initial begin
    u = '0; u.w = 4'b1101;
    $display("DIGEST=%b %b %b %0d", u, u.s.a, u.s.q, $bits(u));
    u.s.q = 3'b010;
    $display("DIGEST=%b", u.w);
  end

  initial #5 $finish;
endmodule
"#,
        "1101 1 101 4|1010",
    );
    // ns_three: both oracles
    digest(
        "ns_three",
        r#"module tb;
  typedef struct packed { byte b; logic [1:0] y; } in_t;
  typedef struct packed { in_t i; logic m; } mid_t;
  typedef struct packed { mid_t md; logic [2:0] t; } out_t;
  out_t o;
  initial begin
    o = '0; o.md.i.b = -5; o.md.m = 1; o.t = 3'b011; o.md.i.y = 2'b10;
    $display("DIGEST=%0d %0d %0d %h %0d %h %b %b", $bits(o), $bits(o.md), $bits(o.md.i), o, o.md.i.b, o.md, o.md.i.y, o.md.i.b[7]);
    o.md.i.b[3:0] = 4'hf;
    $display("DIGEST=%h %0d", o, o.md.i.b);
  end

  initial #5 $finish;
endmodule
"#,
        "14 11 10 3eeb -5 7dd 10 1|3feb -1",
    );
    // ns_two_state: verilator — verilator is 2-state (prints 0 for every x/z); a 4-state outer struct defaults to x (§7.2.1), a 2-state member of it coerces a PATTERN value (line 2 `p`, line 3 `q`) but a direct x/z WRITE into it is kept (line 2 `o.i.q`) — the pre-existing flat class of ctl_two_state
    digest(
        "ns_two_state",
        r#"module tb;
  typedef struct packed { bit [3:0] q; bit s; } in_t;
  typedef struct packed { logic a; in_t i; } out_t;
  typedef struct packed { bit a; in_t i; } out2_t;
  out_t o; out2_t p;
  initial begin
    $display("DIGEST=%b %b", o, p);
    o.i.q = 4'bx1z0; p.i = '{q: 4'b1x01, s: 1'bz};
    $display("DIGEST=%b %b", o, p);
    o = '{a: 1'bx, i: '{q: 4'bxxxx, s: 1'b1}};
    $display("DIGEST=%b", o);
  end

  initial #5 $finish;
endmodule
"#,
        "xxxxxx 000000|xx1z0x 010010|x00001",
    );
    // ns_union_in_struct: both oracles
    digest(
        "ns_union_in_struct",
        r#"module tb;
  typedef union packed { logic [3:0] w; logic [3:0] v; } u_t;
  typedef struct packed { logic a; u_t u; logic b; } s_t;
  s_t s;
  initial begin
    s = '0; s.u.w = 4'b1010; s.a = 1;
    $display("DIGEST=%b %b %b %0d", s, s.u.v, s.u, $bits(s));
  end

  initial #5 $finish;
endmodule
"#,
        "110100 1010 1010 6",
    );
    // ns_unpacked_record: verilator
    loud(
        "ns_unpacked_record",
        r#"module tb;
  typedef struct packed { logic a; logic [1:0] q; } in_t;
  typedef struct { in_t i; int n; } rec_t;
  rec_t r;
  initial begin
    r.n = 5; r.i = '0; r.i.q = 2'b11;
    $display("DIGEST=%0d %b %b", r.n, r.i, r.i.q);
  end

  initial #5 $finish;
endmodule
"#,
        "expected a non-struct member type in an unpacked struct (a packed-stru",
    );
    // ns_whole_write: split — iverilog reads/writes a nested member part-select as the whole member (disqualified by its own neighbouring answers); verilator = LRM
    digest(
        "ns_whole_write",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [3:0] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i = 6'b101101;
    $display("DIGEST=%b %b %b %b", o, o.i, o.i.q, o.i[2:1]);
    o.i[3:0] = 4'b0110; o.i[5] = 1'b0;
    $display("DIGEST=%b %b", o, o.i.U0);
  end

  initial #5 $finish;
endmodule
"#,
        "001011010 101101 0110 10|000001100 0",
    );
    // ns_whole_write_idx: verilator
    loud(
        "ns_whole_write_idx",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [3:0] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i[1+:2] = 2'b11;
    $display("DIGEST=%b", o.i);
  end

  initial #5 $finish;
endmodule
"#,
        "expected a constant `[a:b]` range or bit-select on a packed-struct mem",
    );
}

#[test]
fn ns_read() {
    // ns_read_byte: both oracles
    digest(
        "ns_read_byte",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; byte q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = -3; o.i.U0 = 1;
    $display("DIGEST=%h %0d %h %0d %0d", o, o.i.q, o.i, $bits(o), $bits(o.i));
  end

  initial #5 $finish;
endmodule
"#,
        "07f4 -3 3fa 13 10",
    );
    // ns_read_enum: both oracles
    digest(
        "ns_read_enum",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; e_t q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = C; o.i.U0 = 1;
    $display("DIGEST=%h %0d %h %0d %0d", o, o.i.q, o.i, $bits(o), $bits(o.i));
  end

  initial #5 $finish;
endmodule
"#,
        "18 2 c 7 4",
    );
    // ns_read_vec: both oracles
    digest(
        "ns_read_vec",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [3:0] q; logic SE; } in_t;
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
    // ns_sub_vec: both oracles
    digest(
        "ns_sub_vec",
        r#"module tb;
  typedef enum logic [1:0] { A=0, B=1, C=2 } e_t;
  typedef struct packed { logic U0; logic [3:0] q; logic SE; } in_t;
  typedef struct packed { logic [1:0] cor; in_t i; logic valid; } out_t;
  out_t o;
  initial begin
    o = '0; o.i.q = 4'b1011;
    $display("DIGEST=%b %b %b", o.i.q[2], o.i.q[3:1], o);
    o.i.q[3] = 1'b0; o.i.q[1:0] = 2'b10;
    $display("DIGEST=%b %b", o.i.q, o);
  end

  initial #5 $finish;
endmodule
"#,
        "0 101 000101100|0010 000001000",
    );
}
