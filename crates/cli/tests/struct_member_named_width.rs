//! Packed-struct members declared with a NAMED integer type (`int`/`byte`/
//! `shortint`/`longint`/`integer`/`time`) must size from the TYPE, not default
//! to width 1. Before the fix, `typedef struct packed { int a; int b; } p;`
//! laid `a`/`b` out as 1 bit each, so `o.a = 7; $display("%0d", o.a)` printed `1`
//! (vita) vs `7` (iverilog) — a broad silent-wrong. The fix derives the member
//! width from the kind (`int`/`integer`=32, `byte`=8, `shortint`=16,
//! `longint`/`time`=64), records the member's effective signedness so a signed
//! whole-field read sign-extends (`int a; a=-5` ⇒ `-5`), and backs an all-2-state
//! struct with a 2-state vector so it defaults to 0 (§7.2.1), not X.
//!
//! Every expected value below is the iverilog 13.0 output for the same source.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_smnw_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| {
        !l.starts_with("simulation ended") && !l.contains("VITA-W1017") && !l.trim().is_empty()
    }) {
        s.push_str(l.trim());
        s.push('\n');
    }
    s
}

#[test]
fn int_members_width_not_one() {
    // The headline silent-wrong: `int a/b` are 32 bits, so they hold 7/9 — not the
    // low 1 bit (which printed `1 1` before the width fix).
    let out = run("module top;\n\
        typedef struct packed { int a; int b; } p;\n\
        p o;\n\
        initial begin o.a=7; o.b=9; $display(\"%0d %0d\", o.a, o.b); end\n\
      endmodule\n");
    assert_eq!(out, "7 9\n");
}

#[test]
fn all_atom_types_width() {
    let out = run("module top;\n\
        typedef struct packed { byte a; shortint b; int c; longint d; integer e; } p;\n\
        p o;\n\
        initial begin o.a=1; o.b=2; o.c=3; o.d=4; o.e=5;\n\
          $display(\"%0d %0d %0d %0d %0d\", o.a, o.b, o.c, o.d, o.e); end\n\
      endmodule\n");
    assert_eq!(out, "1 2 3 4 5\n");
}

#[test]
fn mixed_named_and_explicit_range() {
    // int(32) + logic[15:0](16) + byte(8): total 56 bits; whole-struct %h packs them.
    let out = run("module top;\n\
        typedef struct packed { int a; logic [15:0] b; byte c; } p;\n\
        p o;\n\
        initial begin o.a=1234; o.b=5678; o.c=99;\n\
          $display(\"%0d %0d %0d\", o.a, o.b, o.c); $display(\"%h\", o); end\n\
      endmodule\n");
    assert_eq!(out, "1234 5678 99\n000004d2162e63\n");
}

#[test]
fn signed_named_members_read_back_negative() {
    // A whole-field read of a signed member sign-extends (it is a TYPED member ref,
    // not a raw part-select). int/byte/shortint/longint/integer default signed.
    let out = run("module top;\n\
        typedef struct packed { int a; byte b; shortint c; longint d; integer e; } p;\n\
        p o;\n\
        initial begin o.a=-5; o.b=-2; o.c=-3; o.d=-7; o.e=-9;\n\
          $display(\"%0d %0d %0d %0d %0d\", o.a, o.b, o.c, o.d, o.e); end\n\
      endmodule\n");
    assert_eq!(out, "-5 -2 -3 -7 -9\n");
}

#[test]
fn explicit_signing_qualifiers() {
    // `int signed`/`int unsigned` and `logic signed [31:0]`/`logic [31:0]` read back
    // with the declared sign — the signed ones negative, the unsigned ones wide.
    let out = run("module top;\n\
        typedef struct packed { int signed sa; int unsigned ua;\n\
                                logic signed [31:0] vs; logic [31:0] vu; } p;\n\
        p o;\n\
        initial begin o.sa=-5; o.ua=-6; o.vs=-7; o.vu=-8;\n\
          $display(\"%0d %0d %0d %0d\", o.sa, o.ua, o.vs, o.vu); end\n\
      endmodule\n");
    assert_eq!(out, "-5 4294967290 -7 4294967288\n");
}

#[test]
fn subselect_of_signed_member_is_unsigned() {
    // §5.4.1: a part-select of a signed member is UNSIGNED (only the whole-field
    // read sign-extends). `o.a` = -5 but `o.a[15:0]` = 65531 and `o.a[3]` = 1.
    let out = run("module top;\n\
        typedef struct packed { int a; byte b; } p;\n\
        p o;\n\
        initial begin o.a=-5;\n\
          $display(\"%0d %0d %0d\", o.a, o.a[15:0], o.a[3]); end\n\
      endmodule\n");
    assert_eq!(out, "-5 65531 1\n");
}

#[test]
fn bits_and_hex_of_mixed_struct() {
    // $bits sums the type-derived widths (8+16+32+64+32+1+1+4 = 158).
    let out = run("module top;\n\
        typedef struct packed { byte a; shortint b; int c; longint d; integer e;\n\
                                bit f; logic g; bit [3:0] h; } p;\n\
        p o;\n\
        initial begin o.a=1; o.b=2; o.c=3; o.d=4; o.e=5; o.f=1; o.g=0; o.h=4'hA;\n\
          $display(\"size=%0d\", $bits(o)); $display(\"%h\", o); end\n\
      endmodule\n");
    assert_eq!(out, "size=158\n00400080000000c000000000000001000000016a\n");
}

#[test]
fn all_two_state_struct_defaults_to_zero() {
    // §7.2.1: every member 2-state ⇒ the struct is 2-state ⇒ defaults to 0, not X.
    let out = run("module top;\n\
        typedef struct packed { int a; byte b; } p;\n\
        p o;\n\
        initial $display(\"a=%0d b=%0d whole=%h\", o.a, o.b, o);\n\
      endmodule\n");
    assert_eq!(out, "a=0 b=0 whole=0000000000\n");
}

#[test]
fn any_four_state_member_makes_struct_four_state() {
    // A `logic`/`integer`/`time` member makes the WHOLE struct 4-state ⇒ even the
    // 2-state members read X until written. Matches iverilog.
    let out = run("module top;\n\
        typedef struct packed { int a; logic [7:0] b; } pm;\n\
        typedef struct packed { integer a; byte b; } pi;\n\
        pm om; pi oi;\n\
        initial begin\n\
          $display(\"mixed: a=%0d b=%0d\", om.a, om.b);\n\
          $display(\"integer: a=%0d b=%0d\", oi.a, oi.b);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "mixed: a=x b=x\ninteger: a=x b=x\n");
}

#[test]
fn write_read_and_signed_arithmetic() {
    let out = run("module top;\n\
        typedef struct packed { int a; byte b; shortint c; } p;\n\
        p o;\n\
        initial begin\n\
          o.a = 100; $display(\"a=%0d\", o.a);\n\
          o.b = -3;  $display(\"b=%0d\", o.b);\n\
          o.a = o.a + 1; $display(\"a2=%0d\", o.a);\n\
          $display(\"sum=%0d\", o.a + o.b);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "a=100\nb=-3\na2=101\nsum=98\n");
}

#[test]
fn byte_sign_boundary() {
    // 8'h80 read through a signed `byte` member is -128; 8'h7F is 127.
    let out = run("module top;\n\
        typedef struct packed { byte a; byte b; } p;\n\
        p o;\n\
        initial begin o.a=8'h7F; $display(\"%0d\", o.a); o.a=8'h80; $display(\"%0d\", o.a); end\n\
      endmodule\n");
    assert_eq!(out, "127\n-128\n");
}

#[test]
fn all_two_state_union_defaults_to_zero() {
    // A same-width all-2-state packed union is 2-state ⇒ defaults to 0.
    let out = run("module top;\n\
        typedef union packed { int a; int b; } u;\n\
        u o;\n\
        initial $display(\"a=%0d\", o.a);\n\
      endmodule\n");
    assert_eq!(out, "a=0\n");
}
