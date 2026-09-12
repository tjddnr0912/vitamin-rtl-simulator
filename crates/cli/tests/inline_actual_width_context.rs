//! IEEE 1800-2017 §13.5.3 / §11.6.1: a call ACTUAL is assigned to its FORMAL, so the
//! formal's declared width is the actual's context — on the INLINE lane too.
//!
//! The inline function lane lowered each actual with `lower_ctx_or_plain(a, w)`,
//! which takes the context walk only for a FILL-bearing actual; a fill-free one
//! folded at its operands' width and was then resized: `idw(u8 * b8)` into
//! `input [31:0]` printed `00000009` where both oracles print `0000f609`, and the
//! `automatic` twin (a frame formal is a net the engine sizes against) was right.
//! The §4.5.491 opt-in (`lower_inline_assign_rhs`, with its real-target and
//! real-operand guards) now runs at the actual, keyed on the formal's width and
//! bit-vector-ness; a nested call's actuals take THEIR formal's width, not the
//! enclosing region's. The same defect at a MODULE-scope call site
//! (`z = idw($signed(u8) * q8)`) is the same site, so it moves too.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! value asserted below was measured in BOTH (verilator's `$random` stream differs,
//! so the draw-count cell is iverilog's). PRE values are from a release binary built
//! at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_iawc_{}_{n}", std::process::id()));
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
    let _ = std::fs::remove_dir_all(&d);
    so
}

/// ① THE HEADLINE: a plain product, a stamped product and a 16-bit formal at a
/// module-scope call site and inside an inline body; the `automatic` twin, a fill
/// actual and a `real` formal as the controls that were already right.
#[test]
fn an_actual_takes_the_formals_width_as_its_context() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  function [31:0] idw(input [31:0] v); idw = v; endfunction
  function automatic [31:0] idwa(input [31:0] v); idwa = v; endfunction
  function [31:0] id16(input [15:0] v); id16 = v; endfunction
  function [31:0] idr(input real v); idr = v; endfunction
  function [31:0] f1; f1 = idw($signed(u8) * q8); endfunction
  function [31:0] f2; f2 = idw(u8 * b8); endfunction
  function [31:0] f3; f3 = idwa(u8 * b8); endfunction
  function [31:0] f4; f4 = id16(u8 * b8); endfunction
  function [31:0] f5; f5 = idw(u8 + '1); endfunction
  function [31:0] f6; f6 = idr(u8 * b8); endfunction
  logic [31:0] z1, z2, z3, z4, z5, z6;
  initial begin
    z1 = idw($signed(u8) * q8); z2 = idw(u8 * b8); z3 = idwa(u8 * b8); z4 = id16(u8 * b8); z5 = idw(u8 + '1); z6 = idr(u8 * b8);
    $display("Z=%h %h %h %h %h %h", z1, z2, z3, z4, z5, z6);
    $display("F=%h %h %h %h %h %h", f1(), f2(), f3(), f4(), f5(), f6());
    #1 $finish;
  end
endmodule
"#);
    // PRE: `Z=00000020 00000009 0000f609 00000009 000000f6 00000009` and the same
    // for `F=` — z3 (frame), z5 (fill) and z6 (real formal) were already right.
    assert_eq!(
        o,
        "Z=00000120 0000f609 0000f609 0000f609 000000f6 00000009\n\
         F=00000120 0000f609 0000f609 0000f609 000000f6 00000009"
    );
}

/// ② The context is the FORMAL's, per actual: two actuals of one call, a shifted
/// product, a NESTED call (the inner actual takes the inner formal's width), a
/// call in a sum, a real operand (closes the context, §11.8.1), and an inline
/// TASK's input, which copies to a formal-width local and was already right.
#[test]
fn every_actual_position_and_the_controls() {
    let o = run(r#"module t;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  real r = 2.5;
  function [31:0] idw(input [31:0] v); idw = v; endfunction
  function [31:0] two(input [31:0] v, input [31:0] w); two = v + w; endfunction
  function [31:0] f1; f1 = idw(u8 * b8 + r); endfunction
  function [31:0] f2; f2 = two(u8 * b8, u8 + b8); endfunction
  function [31:0] f3; f3 = idw((u8 * b8) >> 4); endfunction
  function [31:0] f4; f4 = idw(idw(u8 * b8)); endfunction
  function [31:0] f5; f5 = idw(u8 * b8) + idw(u8 + b8); endfunction
  task ti(input [31:0] v); $display("TI=%h", v); endtask
  initial begin
    $display("F=%h %h %h %h %h", f1(), f2(), f3(), f4(), f5());
    ti(u8 * b8);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `F=0000000c 000000ff 00000000 00000009 000000ff`, `TI=0000f609`.
    assert_eq!(
        o,
        "F=0000000c 0000f7ff 00000f60 0000f609 0000f7ff\nTI=0000f609"
    );
}

/// ③ Sign and width of the FORMAL decide the region: a signed 16-bit formal
/// (unsigned and signed regions), a 64-bit formal, a shift, a comparison, a
/// ternary, an 8-bit formal (no widening), a `string` formal, a concat and a
/// signed product — plus a HIERARCHICAL actual, which stays on the pre-slice
/// lowering (an opaque leaf, PRE-identical `09` for the oracles' `f609`).
#[test]
fn the_formals_sign_and_width_decide_the_region() {
    let o = run(r#"module sub; logic [7:0] x = 8'hF7; endmodule
module t;
  sub u();
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [63:0] w64 = 64'hFFFF_FFFF_FFFF_FFFF;
  real r = 2.5; string sv = "ab";
  function [31:0] idw(input [31:0] v); idw = v; endfunction
  function [31:0] ids(input signed [15:0] v); ids = v; endfunction
  function [63:0] id64(input [63:0] v); id64 = v; endfunction
  function [7:0] id8(input [7:0] v); id8 = v; endfunction
  function [31:0] idstr(input string v); idstr = v.len(); endfunction
  function [31:0] f1; f1 = idw($signed(u8) + '1); endfunction
  function [31:0] f2; f2 = ids(u8 * b8); endfunction
  function [31:0] f3; f3 = ids(s8 * q8); endfunction
  function [63:0] f4; f4 = id64(w64 + 1); endfunction
  function [31:0] f5; f5 = idw(u8 << 4); endfunction
  function [31:0] f6; f6 = idw(u8 > b8); endfunction
  function [31:0] f7; f7 = idw(u8 ? u8 * b8 : 0); endfunction
  function [31:0] f8; f8 = id8(u8 * b8); endfunction
  function [31:0] f9; f9 = idw(u.x * b8); endfunction
  function [31:0] f10; f10 = idstr(sv); endfunction
  function [31:0] f11; f11 = idw(u8 * b8 + r); endfunction
  function [31:0] f12; f12 = idw({u8} * b8); endfunction
  function [31:0] f13; f13 = idw(s8 * q8); endfunction
  function [31:0] f14; f14 = idw((u8 * b8) >> u8[3:0]); endfunction
  initial begin
    $display("%h %h %h %h %h %h %h", f1(), f2(), f3(), f4(), f5(), f6(), f7());
    $display("%h %h %h %h %h %h %h", f8(), f9(), f10(), f11(), f12(), f13(), f14());
    #1 $finish;
  end
endmodule
"#);
    // PRE: `000000f6 00000009 00000020 0000000000000000 00000070 00000000 0000f609`
    //      `00000009 09 00000002 0000000c 00000009 00000020 00000000`.
    assert_eq!(
        o,
        "000000f6 fffff609 00000120 0000000000000000 00000f70 00000000 0000f609\n\
         00000009 09 00000002 0000000c 0000f609 00000120 000001ec"
    );
}

/// ④ A `$random` actual is still drawn ONCE: the region's sign walk declines a
/// system function it cannot sign, so the actual keeps its pre-slice lowering and
/// the draw stream matches iverilog's (`12153524` is iverilog's first draw).
#[test]
fn a_random_actual_is_drawn_once() {
    let o = run(r#"module t;
  function [31:0] idw(input [31:0] v); idw = v; endfunction
  function [31:0] ids(input signed [31:0] v); ids = v; endfunction
  logic [31:0] z1, z2;
  initial begin
    z1 = idw($random); z2 = ids($random);
    $display("R=%h %h", z1, z2);
    #1 $finish;
  end
endmodule
"#);
    // iverilog: the first two draws of its stream.
    assert_eq!(o, "R=12153524 c0895e81");
}
