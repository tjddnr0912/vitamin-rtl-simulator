//! ROADMAP §2 F7: a class-field read (`c.bu`) in an inline function's bind, body
//! or return, and under a §11.6.1 width context.
//!
//! A class-field read lowers to `Signal{net: <32-bit HANDLE net>, word:
//! Some(field id)}`, with the FIELD's width and sign in the `class_field_widths`
//! sidecar. The engine's width table and elaborate's canonical rule read that
//! sidecar; elaborate's mirror (`ir_bits_of` / `expr_self_signed`) answered the
//! handle's 32/unsigned instead, so `trusted_self_width` saw a disagreement and
//! every class-field bind kept its unsized tail: `i16(c.bu)` printed `xxc3` for
//! `00c3`, `fw = c.sf` into `signed [63:0]` printed a fabricated 40-bit `fd`.
//! The mirror now reads the sidecar. Because that makes the field's own 8 bits
//! TRUSTED, a context-determined operator over the field (`c.bu + 8'hFF` into 16
//! bits, `32'(c.u8 + ua)`) must be evaluated at the context width before it is
//! sealed — so the §11.6.1 context walks now resolve a class-field leaf too
//! (`class_field_leaf`) instead of treating it as opaque.
//!
//! ORACLES: iverilog 13.0 (`iverilog -g2012`) and verilator 5.052
//! (`--binary --timing`). Every pinned value is the raw output of both unless the
//! test says otherwise. PRE = the binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_ok(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ibcf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let se = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(out.status.code(), Some(0), "stdout:\n{so}\nstderr:\n{se}");
    assert!(!se.contains("error["), "no diagnostic expected:\n{se}");
    so
}

/// f7a — bind of an 8-bit field into 16/8/64-bit formals, and a 32-bit field.
/// PRE: `i16=xxc3 … i64=00000000c3` (10 hex digits: the fabricated width).
#[test]
fn a_class_field_actual_takes_the_formal_width() {
    let o = run_ok(
        r#"class C; bit [7:0] bu = 8'hC3; byte sf = -3; bit [31:0] wide = 32'hDEADBEEF; endclass
module top;
  C c;
  function [15:0] i16(input [15:0] a); i16 = a; endfunction
  function [7:0] i8(input [7:0] a); i8 = a; endfunction
  function [63:0] i64(input [63:0] a); i64 = a; endfunction
  initial begin c = new; $display("F7A i16=%h i8=%h wide=%h i64=%h", i16(c.bu), i8(c.bu), i16(c.wide), i64(c.bu)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F7A i16=00c3 i8=c3 wide=beef i64=00000000000000c3");
}

/// f7b2 — the SIGN half: a signed `byte` field into a signed formal and a signed
/// `[63:0]` return (16 hex digits), a local, an add, and `$bits`.
/// PRE: `s16=xxfd s16u=xxc3 fw=00000000fd g=xxc3`.
#[test]
fn a_signed_class_field_extends_by_its_own_sign() {
    let o = run_ok(
        r#"class C; bit [7:0] bu = 8'hC3; byte sf = -3; bit [39:0] big = 40'h1; endclass
module top;
  C c;
  function [15:0] s16(input signed [15:0] a); s16 = a; endfunction
  function signed [63:0] fw(); fw = c.sf; endfunction
  function [15:0] g(); logic [15:0] t; t = c.bu; g = t; endfunction
  function [15:0] fh(); fh = c.big + 1'b1; endfunction
  logic [63:0] o1; logic [15:0] o2;
  initial begin c = new; o1 = fh(); o2 = fh(); $display("F7B s16=%h s16u=%h fw=%h g=%h fh64=%h fh16=%h bits=%0d", s16(c.sf), s16(c.bu), fw(), g(), o1, o2, $bits(c.bu)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "F7B s16=fffd s16u=00c3 fw=fffffffffffffffd g=00c3 fh64=0000000000000002 fh16=0002 bits=8"
    );
}

/// f7c — an operator over the field, a 4-state field into a 2-state `bit`
/// formal (x drops to 0 — the class-field read cannot be repeated, so this is the
/// bind's single-mention `TwoState`), the frame twin, and the 4-state formal that
/// keeps the x (iverilog `00X7`; verilator, 2-state, prints `0007`).
/// PRE: `ar=xxc4 b16=xxX7 … lu=xxX7`.
#[test]
fn an_operator_a_two_state_formal_and_a_four_state_field() {
    let o = run_ok(
        r#"class C; bit [7:0] bu = 8'hC3; logic [7:0] lu = 8'bx0000111; endclass
module top;
  C c;
  function [15:0] i16(input [15:0] a); i16 = a; endfunction
  function [15:0] b16(input bit [15:0] a); b16 = a; endfunction
  function automatic [15:0] fr16(input [15:0] a); fr16 = a; endfunction
  initial begin c = new; $display("F7C ar=%h b16=%h frame=%h lu=%h", i16(c.bu + 8'd1), b16(c.lu), fr16(c.bu), i16(c.lu)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F7C ar=00c4 b16=0007 frame=00c3 lu=00X7");
}

/// A context-determined operator over the field is evaluated at the CONTEXT width
/// before the seal: `c.bu + 8'hFF` = 0x1c2 in 16 bits, through the bind, a
/// return, a local, and a size cast. Sealing the field's own 8 bits without the
/// context walk gives `00c2` (and `000000c2` for the cast, which PRE had right).
/// PRE: `cls=xxc2 ret=xxc2 loc=xxc2`.
#[test]
fn an_operator_over_a_field_is_evaluated_at_the_context_width() {
    let o = run_ok(
        r#"class C; bit [7:0] bu = 8'hC3; endclass
module top;
  C c; logic [7:0] n8 = 8'hC3;
  function [15:0] i16(input [15:0] a); i16 = a; endfunction
  function [15:0] g16(); g16 = c.bu + 8'hFF; endfunction
  function [15:0] h16(); logic [15:0] t; t = c.bu + 8'hFF; h16 = t; endfunction
  function [15:0] gn16(); gn16 = n8 + 8'hFF; endfunction
  logic [31:0] q;
  initial begin c = new; q = 32'(c.bu + 8'hFF);
    $display("B1 cls=%h net=%h ret=%h loc=%h retn=%h cast=%h", i16(c.bu + 8'hFF), i16(n8 + 8'hFF), g16(), h16(), gn16(), q); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "B1 cls=01c2 net=01c2 ret=01c2 loc=01c2 retn=01c2 cast=000001c2"
    );
}

/// A census of the context walk over class fields of several widths and signs —
/// multiply, signed multiply, mixed sign, `~`, unary minus, a 1-bit field, `>>>`,
/// `<<`, a ternary, 64-bit fields into a 128-bit return, `{}` (self-determined),
/// a 4-state field, `this.`/bare members in a method, and size casts.
/// PRE had 18 of the 25 values wrong (`xx…` high bytes, a 128-bit row of zeros,
/// an unextended `fb`).
#[test]
fn class_field_context_census() {
    let o = run_ok(
        r#"class C;
  bit [7:0] u8 = 8'hFD; byte s8 = -3; logic [7:0] l8 = 8'hF0; bit b1 = 1'b1;
  longint s64 = -5; bit [63:0] u64 = 64'hFFFF_FFFF_FFFF_FFFF; shortint s16 = -300;
  function [15:0] m1(); m1 = this.u8 + 8'hFF; endfunction
  function [15:0] m2(); m2 = u8 + 8'hFF; endfunction
endclass
module top;
  C c; logic [3:0] ua = 4'd3; logic signed [7:0] sp = 8'sd100; logic signed [7:0] sn = -8'sd2;
  function [15:0] f1(); f1 = c.u8 * c.u8; endfunction
  function signed [15:0] f2(); f2 = c.s8 * sn; endfunction
  function [15:0] f3(); f3 = c.s8 + ua; endfunction
  function signed [31:0] f4(); f4 = c.s8 + sn; endfunction
  function [15:0] f5(); f5 = c.u8 + 4'hF; endfunction
  function [15:0] f6(); f6 = ~c.u8; endfunction
  function [15:0] f7(); f7 = -c.s8; endfunction
  function [15:0] f8(); f8 = c.b1 + 1'b1; endfunction
  function [15:0] f9(); f9 = c.s8 >>> 1; endfunction
  function [15:0] f10(); f10 = c.u8 << 4; endfunction
  function [15:0] f11(); f11 = (c.u8 > 8'd1) ? c.s8 : sn; endfunction
  function [127:0] f12(); f12 = c.s64 * 2; endfunction
  function [127:0] f13(); f13 = c.u64 + 1; endfunction
  function [31:0] f14(); f14 = c.s16 + c.s8; endfunction
  function [15:0] f15(); f15 = {c.u8} + 8'hFF; endfunction
  function [15:0] f16(); f16 = c.l8 + 8'h20; endfunction
  logic [63:0] r;
  initial begin c = new;
    $display("K1a %h %h %h %h %h %h %h %h", f1(), f2(), f3(), f4(), f5(), f6(), f7(), f8());
    $display("K1b %h %h %h %h %h %h %h %h", f9(), f10(), f11(), f12(), f13(), f14(), f15(), f16());
    $display("K1c %h %h", c.m1(), c.m2());
    r = 32'(c.u8 + ua); $display("K1d %h", r);
    r = 32'(c.s8 + sp); $display("K1e %h", r);
    r = 16'(c.s8 * sn); $display("K1f %h", r);
    r = 16'(c.u8 * 2'd2); $display("K1g %h", r);
    r = 40'(c.s64 - 1); $display("K1h %h", r);
    r = 8'(c.u8 + 8'h10); $display("K1i %h", r);
    r = 16'(c.s8 + sn) + 1; $display("K1j %h", r);
    #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "K1a fa09 0006 0100 fffffffb 010c ff02 0003 0002\n\
         K1b fffe 0fd0 fffd fffffffffffffffffffffffffffffff6 00000000000000010000000000000000 fffffed1 01fc 0110\n\
         K1c 01fc 01fc\n\
         K1d 0000000000000100\n\
         K1e 0000000000000061\n\
         K1f 0000000000000006\n\
         K1g 00000000000001fa\n\
         K1h fffffffffffffffa\n\
         K1i 000000000000000d\n\
         K1j fffffffffffffffc"
    );
}
