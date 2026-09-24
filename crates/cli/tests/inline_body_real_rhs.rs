//! ROADMAP §2 F11: a REAL-valued rhs assigned to an INTEGRAL return or body-local
//! of an inline (static, straight-line) function body.
//!
//! The frame path's target is a net, and the engine's store rounds a real half
//! away from zero and narrows it to the declared width (§10.7 / §6.12.2). The
//! inline fold has no net: `fold_straight_line` left a real rhs verbatim, so the
//! value kept the rhs's own width and sign — `f = r + x` into `[7:0]` with 255.5
//! read back `0100` where both oracles give `0000`, `-r - x` gave `fffe` for
//! `00fe`, and `%h` of the return was E3009. The rhs now takes
//! `SysFuncId::RealToInt` (format_version 34) and then the target's declared
//! width and sign. A `real` TARGET is untouched.
//!
//! ORACLES: iverilog 13.0 (`iverilog -g2012`) and verilator 5.052
//! (`--binary --timing`). Every pinned value is the raw output of both unless the
//! test says otherwise. PRE = the binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_ok(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ibrr_{}_{n}", std::process::id()));
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

/// f11a — `%h` of a return whose rhs is real. PRE: E3009 (hex format on a real).
#[test]
fn a_real_return_prints_in_hex() {
    let o = run_ok(
        r#"module top;
  real r = 1.0;
  function [7:0] f(input [7:0] x); f = r + x*x; endfunction
  logic [15:0] o16; logic [7:0] o8;
  initial begin o16 = f(8'd17); o8 = f(8'd17); $display("F11A o16=%h o8=%h d=%h", o16, o8, f(8'd17)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F11A o16=0022 o8=22 d=22");
}

/// f11b — the NARROWING: 255.5 rounds to 256, which is 0 in `[7:0]`.
/// PRE: `f=0100`.
#[test]
fn a_real_return_narrows_to_the_declared_width() {
    let o = run_ok(
        r#"module top;
  real r = 1.5;
  function [7:0] f(input [7:0] x); f = r + x; endfunction
  function signed [7:0] fs(input [7:0] x); fs = -(r + x); endfunction
  function [15:0] g(input [7:0] x); g = r + x; endfunction
  logic [15:0] o1, o2, o3;
  initial begin o1 = f(8'd254); o2 = fs(8'd3); o3 = g(8'd254); $display("F11B f=%h fs=%h g=%h", o1, o2, o3); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F11B f=0000 fs=fffb g=0100");
}

/// f11e — the ROUNDING, narrowing and SIGN together: 255.6 → 256 → 0; -1.6 → -2
/// → `fe` in an UNSIGNED `[7:0]`, zero-extended. PRE: `0100 fffe`.
#[test]
fn a_real_return_takes_the_target_sign() {
    let o = run_ok(
        r#"module top;
  real r = 0.6;
  function [7:0] f(input [7:0] x); f = r + x; endfunction
  function [7:0] fn(input [7:0] x); fn = -r - x; endfunction
  logic [15:0] o1, o2;
  initial begin o1 = f(8'd255); o2 = fn(8'd1); $display("F11E round=%h neg=%h", o1, o2); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F11E round=0000 neg=00fe");
}

/// f11f — the same rule for a repeatable rhs and for one whose operand is bound
/// to a real CALL through an `int` formal. PRE: `0100 0100`.
#[test]
fn repeatable_and_call_bearing_rhs_both_narrow() {
    let o = run_ok(
        r#"module top;
  real r = 1.5;
  function automatic real rf(input int k); rf = 254.0 + k; endfunction
  function [7:0] f(input [7:0] x); f = r + x; endfunction
  function [7:0] fc(input int x); fc = r + x; endfunction
  logic [15:0] o1, o2;
  initial begin o1 = f(8'd254); o2 = fc(rf(0)); $display("F11F rep=%h call=%h", o1, o2); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F11F rep=0000 call=0000");
}

/// f11c/f11d — body-local targets, an `int` return, the frame twin and `return`.
/// These were `0022` in PRE too; they pin that the new store does not move them.
/// (`x*x` is an integral operand of a real `+`, so it is self-determined at 8
/// bits — 33 — on both oracles as on vita: `[15:0] f = r + x*x` with x=17 is
/// `0022` on iverilog 13 and verilator 5.052, not the 290 = `0122` a real-domain
/// product would give.)
#[test]
fn locals_int_return_frame_and_return_statement() {
    let o = run_ok(
        r#"module top;
  real r = 1.0;
  function [7:0] h(input [7:0] x); logic [7:0] t; t = r + x*x; h = t; endfunction
  function [7:0] h2(input [7:0] x); logic [15:0] t; t = r + x*x; h2 = t; endfunction
  function int fi(input [7:0] x); fi = r + x*x; endfunction
  function automatic [7:0] ff(input [7:0] x); ff = r + x*x; endfunction
  function [7:0] fr(input [7:0] x); return r + x*x; endfunction
  logic [15:0] o1, o2, o3, o4, o5;
  initial begin o1 = h(8'd17); o2 = h2(8'd17); o3 = fi(8'd17); o4 = ff(8'd17); o5 = fr(8'd17);
    $display("F11CD h=%h h2=%h fi=%h frame=%h ret=%h", o1, o2, o3, o4, o5); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F11CD h=0022 h2=0022 fi=0022 frame=0022 ret=0022");
}

/// Ties round AWAY from zero (2.5 + 253 = 255.5 → 256 → 0 in `[7:0]`;
/// -2.5 - 126 = -128.5 → -129 → `7f` in a signed `[7:0]`), 2^52+1 is exact in a
/// `[63:0]` return, and a `[71:0]` return sign-extends the rounded integer
/// (-2.5 - 1 = -3.5 → -4). iverilog 13 and verilator 5.052 both print the line.
/// PRE: `tie=0100 ntie=ff7f`, the other three already right.
#[test]
fn ties_exact_2_pow_52_plus_1_and_a_72_bit_target() {
    let o = run_ok(
        r#"module top;
  real r25 = 2.5, rn25 = -2.5;
  real big = 4503599627370497.0;
  function [7:0] f8(input [7:0] x); f8 = r25 + x; endfunction
  function signed [7:0] fn8(input [7:0] x); fn8 = rn25 - x; endfunction
  function [63:0] f64(input [7:0] x); f64 = big + x; endfunction
  function [71:0] f72(input [7:0] x); f72 = rn25 - x; endfunction
  function [71:0] f72p(input [7:0] x); f72p = big + x; endfunction
  logic [15:0] o1, o2; logic [63:0] o3; logic [71:0] o4, o5;
  initial begin o1 = f8(8'd253); o2 = fn8(8'd126); o3 = f64(0); o4 = f72(8'd1); o5 = f72p(0);
    $display("T1 tie=%h ntie=%h big=%h w72=%h w72p=%h", o1, o2, o3, o4, o5); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(
        o,
        "T1 tie=0000 ntie=007f big=0010000000000001 w72=fffffffffffffffffc w72p=000010000000000001"
    );
}

/// Integral targets WIDER than 64 bits: a `[99:0]` return, a `[99:0]` body local
/// and a `[99:0]` formal bound to a real call hold the exact rounded value (the
/// conversion is 128-bit), matching the module-level store in the same row; `i=8`
/// is 1e30. Both oracles print these lines. PRE: E3009 (hex format on a real); an
/// intermediate build that converted through 64 bits printed `i=0 g=0000000000000000000000000`.
#[test]
fn targets_wider_than_64_bits_hold_the_exact_value() {
    let o = run_ok(
        r#"module top;
  real R;
  function [99:0] g(input integer d); g = R * 1.0 + d; endfunction
  function [7:0] nb(input integer d); nb = R + d; endfunction
  function longint L(input integer d); L = R + d; endfunction
  function [99:0] gl(input integer d); reg [99:0] t; t = R + d; gl = t; endfunction
  function automatic real rf(input integer k); rf = R + k; endfunction
  function [99:0] pb(input [99:0] x); pb = x; endfunction
  function longint pL(input longint x); pL = x; endfunction
  reg [99:0] s100; longint sL; reg [7:0] s8;
  real V[0:9];
  integer i;
  initial begin
    V[0]=2.0**70; V[1]=-(2.0**70); V[2]=9.3e18; V[3]=-9.3e18; V[4]=2.0**63; V[5]=-(2.0**63);
    V[6]=4503599627370497.0; V[7]=-2.5; V[8]=1e30; V[9]=-0.5;
    for (i=0;i<10;i=i+1) begin
      R = V[i]; s100 = R; sL = R; s8 = R;
      $display("i=%0d g=%h gl=%h pb=%h store=%h | L=%h pL=%h storeL=%h | nb=%h store8=%h", i, g(0), gl(0), pb(rf(0)), s100, L(0), pL(rf(0)), sL, nb(0), s8);
    end
    #1 $finish;
  end
endmodule
"#,
    );
    assert_eq!(
        o,
        "i=0 g=0000000400000000000000000 gl=0000000400000000000000000 pb=0000000400000000000000000 store=0000000400000000000000000 | L=0000000000000000 pL=0000000000000000 storeL=0000000000000000 | nb=00 store8=00\ni=1 g=fffffffc00000000000000000 gl=fffffffc00000000000000000 pb=fffffffc00000000000000000 store=fffffffc00000000000000000 | L=0000000000000000 pL=0000000000000000 storeL=0000000000000000 | nb=00 store8=00\ni=2 g=00000000081103cb9fb220000 gl=00000000081103cb9fb220000 pb=00000000081103cb9fb220000 store=00000000081103cb9fb220000 | L=81103cb9fb220000 pL=81103cb9fb220000 storeL=81103cb9fb220000 | nb=00 store8=00\ni=3 g=fffffffff7eefc34604de0000 gl=fffffffff7eefc34604de0000 pb=fffffffff7eefc34604de0000 store=fffffffff7eefc34604de0000 | L=7eefc34604de0000 pL=7eefc34604de0000 storeL=7eefc34604de0000 | nb=00 store8=00\ni=4 g=0000000008000000000000000 gl=0000000008000000000000000 pb=0000000008000000000000000 store=0000000008000000000000000 | L=8000000000000000 pL=8000000000000000 storeL=8000000000000000 | nb=00 store8=00\ni=5 g=fffffffff8000000000000000 gl=fffffffff8000000000000000 pb=fffffffff8000000000000000 store=fffffffff8000000000000000 | L=8000000000000000 pL=8000000000000000 storeL=8000000000000000 | nb=00 store8=00\ni=6 g=0000000000010000000000001 gl=0000000000010000000000001 pb=0000000000010000000000001 store=0000000000010000000000001 | L=0010000000000001 pL=0010000000000001 storeL=0010000000000001 | nb=01 store8=01\ni=7 g=ffffffffffffffffffffffffd gl=ffffffffffffffffffffffffd pb=ffffffffffffffffffffffffd store=ffffffffffffffffffffffffd | L=fffffffffffffffd pL=fffffffffffffffd storeL=fffffffffffffffd | nb=fd store8=fd\ni=8 g=c9f2c9cd04675000000000000 gl=c9f2c9cd04675000000000000 pb=c9f2c9cd04675000000000000 store=c9f2c9cd04675000000000000 | L=4675000000000000 pL=4675000000000000 storeL=4675000000000000 | nb=00 store8=00\ni=9 g=fffffffffffffffffffffffff gl=fffffffffffffffffffffffff pb=fffffffffffffffffffffffff store=fffffffffffffffffffffffff | L=ffffffffffffffff pL=ffffffffffffffff storeL=ffffffffffffffff | nb=ff store8=ff"
    );
}
