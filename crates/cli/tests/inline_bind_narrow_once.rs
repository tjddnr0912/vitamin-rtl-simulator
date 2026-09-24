//! The inline bind of an actual that may NOT be repeated (a class-field read, an
//! unpacked-array element, a function call, `$random`) to a SIGNED or 2-STATE
//! formal, when its width is trusted.
//!
//! Signedness and 2-state-ness are the two formal properties whose usual builders
//! name the actual more than once (a sign-fill bit, the per-bit 2-state
//! coercion), so such an actual used to keep an unsized tail: never narrowed to
//! the formal, never given its sign — `ib4(c.u8)` into `bit [3:0]` printed `c3`,
//! `ibyte(c.l8)` `c3` where both oracles sign-extend the byte to `..ffc3`, and
//! `bs32($random)` through `bit signed [15:0]` `8484d609` for iverilog's
//! `ffffd609`. A narrowing or equal-width bind, and a widening of an UNSIGNED
//! actual, are now applied with operations that name the actual once
//! (`select_low` or a constant zero fill, the formal's sign stamp, `TwoState`).
//! A SIGNED actual widened into the formal still needs a sign-fill bit — a second
//! mention — and keeps the older tail (ROADMAP §2 residue).
//!
//! ORACLES: iverilog 13.0 (`iverilog -g2012`) and verilator 5.052
//! (`--binary --timing`); every pinned value is the raw output of both unless
//! the test says otherwise. PRE = the binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_ok(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ibno_{}_{n}", std::process::id()));
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

/// Narrow 2-state formals (`bit [3:0]`, `bit signed [3:0]`, `bit [11:0]`, `int`,
/// `byte`) bound to class fields (one with an x bit), an array element, a call and
/// `$random`. `rnd` is iverilog's (the low nibble of its first draw, 0x12153524);
/// verilator's stream differs, every other value is both oracles'. PRE:
/// `u8=..c3 l8=..c3 lx=..X3 arr=..c3 rnd=..12153524` and `b12_lx int_lx byte_lx=..X3`.
#[test]
fn two_state_formals_bound_to_non_repeatable_actuals() {
    let o = run_ok(
        r#"class C; bit [7:0] u8 = 8'hC3; logic [7:0] l8 = 8'b1100_0011; logic [7:0] lx = 8'b1x00_0011; endclass
module top;
  C c; logic [79:0] o; logic [7:0] mm [0:1]; logic [7:0] lm = 8'hC3; integer seed;
  function [7:0] g(input [7:0] a); g = a; endfunction
  function [79:0] ib4(input bit [3:0] a); ib4 = a; endfunction
  function [79:0] ib4s(input bit signed [3:0] a); ib4s = a; endfunction
  function [79:0] ib12(input bit [11:0] a); ib12 = a; endfunction
  function [79:0] ii(input int a); ii = a; endfunction
  function [79:0] ibyte(input byte a); ibyte = a; endfunction
  function [79:0] qb4(input bit [3:0] a); return a; endfunction
  initial begin c = new; mm[0] = 8'hC3; seed = 1;
    o = ib4(c.u8); $write("E7 u8=%h", o); o = ib4(c.l8); $write(" l8=%h", o); o = ib4(c.lx); $write(" lx=%h", o); o = ib4(g(8'hC3)); $write(" call=%h", o);
    o = ib4(mm[0]); $write(" arr=%h", o); o = ib4(lm); $write(" mod=%h", o); o = ib4($random); $write(" rnd=%h", o); o = ib4(g(8'hC3) + 1'b0); $display(" callp=%h", o);
    o = ib4s(g(8'hC3)); $write("E8 s_call=%h", o); o = ib12(g(8'hC3)); $write(" b12_call=%h", o); o = ib12(c.lx); $write(" b12_lx=%h", o); o = ii(c.lx); $write(" int_lx=%h", o);
    o = ibyte(c.lx); $write(" byte_lx=%h", o); o = ii(g(8'bx000_0001)); $write(" int_callx=%h", o); o = qb4(c.lx); $write(" frame_lx=%h", o); o = qb4(g(8'hC3)); $display(" frame_call=%h", o);
    #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "E7 u8=00000000000000000003 l8=00000000000000000003 lx=00000000000000000003 call=00000000000000000003 arr=00000000000000000003 mod=00000000000000000003 rnd=00000000000000000004 callp=00000000000000000003\nE8 s_call=00000000000000000003 b12_call=000000000000000000c3 b12_lx=00000000000000000083 int_lx=00000000000000000083 byte_lx=ffffffffffffffffff83 int_callx=00000000000000000001 frame_lx=00000000000000000003 frame_call=00000000000000000003"
    );
}

/// `byte` / `logic signed [7:0]` formals sign-extend the bound field or element
/// into the 80-bit return; `bit [3:0]` / `logic [3:0]` narrow. The generate-scope
/// continuous assigns carry F8/F9 through a `wire` initializer. PRE:
/// `byte_*=..c3 lsb_*=..c3 b4_bm=..c3 l4_u8=..c3`, `g0w=012d g1w=012e g0w2=00X3`.
#[test]
fn signed_formals_narrower_than_the_actual() {
    let o = run_ok(
        r#"class C; bit [7:0] u8 = 8'hC3; logic [7:0] l8 = 8'hC3; endclass
module top;
  C c; logic [79:0] o; logic [7:0] mm [0:1]; bit [7:0] bm [0:1];
  function automatic real rf(input real k); rf = k; endfunction
  function [79:0] ibyte(input byte a); ibyte = a; endfunction
  function [79:0] ilb(input logic signed [7:0] a); ilb = a; endfunction
  function [79:0] ib4(input bit [3:0] a); ib4 = a; endfunction
  function [79:0] il4(input logic [3:0] a); il4 = a; endfunction
  genvar gi;
  for (gi = 0; gi < 2; gi = gi + 1) begin : gb
    function [7:0] pb(input byte x); pb = x; endfunction
    function [7:0] ff(input [7:0] x); bit [7:0] b; b = x; ff = b; endfunction
    real rr = 300.7 + gi;
    wire [15:0] w = pb(rf(rr));
    wire [15:0] w2 = ff(8'bx000_0011);
  end
  initial begin c = new; mm[0] = 8'hC3; bm[0] = 8'hC3;
    o = ibyte(c.l8); $write("E9 byte_l8=%h", o); o = ibyte(c.u8); $write(" byte_u8=%h", o); o = ibyte(mm[0]); $write(" byte_mm=%h", o); o = ibyte(bm[0]); $write(" byte_bm=%h", o);
    o = ilb(c.u8); $write(" lsb_u8=%h", o); o = ilb(mm[0]); $write(" lsb_mm=%h", o); o = ib4(bm[0]); $write(" b4_bm=%h", o); o = il4(mm[0]); $write(" l4_mm=%h", o); o = il4(c.u8); $display(" l4_u8=%h", o);
    #1 $display("E10 g0w=%h g1w=%h g0w2=%h", gb[0].w, gb[1].w, gb[0].w2);
    #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "E9 byte_l8=ffffffffffffffffffc3 byte_u8=ffffffffffffffffffc3 byte_mm=ffffffffffffffffffc3 byte_bm=ffffffffffffffffffc3 lsb_u8=ffffffffffffffffffc3 lsb_mm=ffffffffffffffffffc3 b4_bm=00000000000000000003 l4_mm=00000000000000000003 l4_u8=00000000000000000003\nE10 g0w=002d g1w=002e g0w2=0003"
    );
}

/// `logic signed [3:0]` / `[15:0]` formals bound to wider class fields narrow, then
/// sign-extend into the 80-bit return. PRE:
/// `u7=..55 s40=0000000000fffffffff7 s64=0000fffffffffffffff5 w40=..fffffffff7 w64=..f5`.
#[test]
fn signed_formal_narrower_than_a_class_field() {
    let o = run_ok(
        r#"class C; bit [6:0] u7 = 7'h55; logic signed [39:0] s40 = -9; logic signed [63:0] s64 = -11; endclass
module top;
  C c; logic [79:0] o;
  function [79:0] in4s(input logic signed [3:0] a); in4s = a; endfunction
  function [79:0] iw16s(input logic signed [15:0] a); iw16s = a; endfunction
  initial begin c = new;
    o = in4s(c.u7); $write("N1 u7=%h", o); o = in4s(c.s40); $write(" s40=%h", o); o = in4s(c.s64); $write(" s64=%h", o);
    o = iw16s(c.s40); $write(" w40=%h", o); o = iw16s(c.s64); $display(" w64=%h", o);
    #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "N1 u7=00000000000000000005 s40=00000000000000000007 s64=00000000000000000005 w40=fffffffffffffffffff7 w64=fffffffffffffffffff5"
    );
}

/// `$random` and a frame call into `bit [15:0]`, `bit signed [15:0]`, `int` and
/// `shortint` formals. `next=` is the following draw, so it shows each bind drew
/// exactly once. iverilog 13 (verilator's stream differs). PRE:
/// `C1 X3X3 0000X3X3`, `R2 8484d609`, `R4 b2c28466` (the draws already single).
#[test]
fn random_actuals_are_drawn_once_and_take_the_formal_sign() {
    let o = run_ok(
        r#"module top;
  integer k; logic [7:0] X;
  function automatic logic [15:0] uf(input logic [7:0] x); uf = {x, x}; endfunction
  function [15:0] b16(input bit [15:0] x); b16 = x; endfunction
  function [31:0] bs32(input bit signed [15:0] x); bs32 = x; endfunction
  function [31:0] i32(input int x); i32 = x; endfunction
  function [31:0] sh(input shortint x); sh = x + 1; endfunction
  initial begin X = 8'b1x0z_0011;
    $display("C1 %h %h", b16(uf(X)), bs32(uf(X)));
    $display("R1 %h", b16($random)); k = $random; $display("next=%h", k);
    $display("R2 %h", bs32($random)); k = $random; $display("next=%h", k);
    $display("R3 %h", i32($random)); k = $random; $display("next=%h", k);
    $display("R4 %h", sh($random)); k = $random; $display("next=%h", k);
    #1 $finish;
  end
endmodule
"#,
    );
    assert_eq!(
        o,
        "C1 8383 ffff8383\nR1 3524\nnext=c0895e81\nR2 ffffd609\nnext=b1f05663\nR3 06b97b0d\nnext=46df998d\nR4 ffff8466\nnext=89375212"
    );
}
