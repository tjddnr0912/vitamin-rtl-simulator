//! A `$signed(e)` / `$unsigned(e)` leaf, a signing cast and a SIZE / primitive
//! cast inside a §11.6.1 region are widened to the region like any other leaf.
//!
//! The region's extension sign is decided once by `ctx_signed_impl` (the size
//! cast's classifier, which the inline-body context of §4.5.491 reuses). Its
//! `_ => None` tail covered every `SysCall` and every `Cast`, and `None` stands
//! the WHOLE region down to its pre-slice lowering — so the leaf was right and
//! everything around it folded at 8 bits: `function [31:0] f; f = $unsigned(s8)
//! * q8;` with `s8 = -9`, `q8 = -32` printed `00000020` where both oracles print
//! `0000d820`, and the size-cast consumer of the same walk printed `0020` for
//! `16'($signed(u8) * q8)` where both print `0120`. The stamps now answer their
//! own sign (§11.8.1), a signing cast the same, a size cast its operand's
//! (§6.24.1) and a primitive cast the type's; a typedef / `parameter type` cast
//! still declines (it can name `real`).
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! value below was measured in BOTH. PRE values are from a release binary built
//! at the parent commit. A user CALL as a leaf is NOT in this slice (ROADMAP §2:
//! the sign walk declines a call for its own reason) and its cell is pinned at
//! the PRE value so the boundary is visible.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_issl_{}_{n}", std::process::id()));
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

/// ① THE HEADLINE and its leaf census: both stamps, both signs of region, a
/// nested stamp, a parenthesised one, a signed return, 16/32/64-bit returns —
/// beside the leaves that were already widened (a part-select, a concat, a
/// plain name) and the ones that fit (a signed product that needs no widening).
#[test]
fn a_sign_stamp_leaf_is_widened_to_the_region() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  function [31:0] f_uns;    f_uns    = $unsigned(s8) * q8;            endfunction
  function [31:0] f_sgn;    f_sgn    = $signed(u8) * q8;              endfunction
  function [31:0] f_sgn2;   f_sgn2   = $signed(u8) * $signed(b8);     endfunction
  function [31:0] f_uns2;   f_uns2   = $unsigned(s8) * $unsigned(q8); endfunction
  function [31:0] f_psel;   f_psel   = u8[7:0] * b8;                  endfunction
  function [31:0] f_bsel;   f_bsel   = {u8} * b8;                     endfunction
  function [31:0] f_plain;  f_plain  = u8 * b8;                       endfunction
  function [31:0] f_sgnp;   f_sgnp   = $signed(u8) + q8;              endfunction
  function [31:0] f_unsp;   f_unsp   = $unsigned(s8) + $unsigned(q8); endfunction
  function [31:0] f_sgnsh;  f_sgnsh  = $signed(u8) << 4;              endfunction
  function [31:0] f_unsneg; f_unsneg = -$unsigned(s8);                endfunction
  function [31:0] f_sgnnest; f_sgnnest = $signed({u8}) * q8;          endfunction
  function [31:0] f_sgnpar; f_sgnpar = ($signed(u8)) * q8;            endfunction
  function signed [31:0] f_sret; f_sret = $signed(u8) * q8;           endfunction
  function [63:0] f_w64;    f_w64    = $unsigned(s8) * q8;            endfunction
  function [15:0] f_w16;    f_w16    = $unsigned(s8) * q8;            endfunction
  function [31:0] f_mixed;  f_mixed  = $signed(u8) * b8;              endfunction
  function [31:0] f_mixed2; f_mixed2 = $signed(u8) * $unsigned(q8);   endfunction
  initial begin
    $display("%h %h %h %h %h %h %h", f_uns(), f_sgn(), f_sgn2(), f_uns2(), f_psel(), f_bsel(), f_plain());
    $display("%h %h %h %h %h %h %h", f_sgnp(), f_unsp(), f_sgnsh(), f_unsneg(), f_sgnnest(), f_sgnpar(), f_sret());
    $display("%h %h %h %h", f_w64(), f_w16(), f_mixed(), f_mixed2());
    #1 $finish;
  end
endmodule
"#);
    // PRE: `00000020 00000020 00000009 00000020 0000f609 0000f609 0000f609`,
    //      `ffffffd7 000000d7 00000070 00000009 00000020 00000020 00000020`,
    //      `0000000000000020 0020 00000009 00000020`.
    assert_eq!(
        o,
        "0000d820 00000120 00000009 0000d820 0000f609 0000f609 0000f609\n\
         ffffffd7 000001d7 ffffff70 ffffff09 00000120 00000120 00000120\n\
         000000000000d820 d820 0000f609 0000d820"
    );
}

/// ② The CAST leaves: a signing cast, a size cast NARROWER than, EQUAL to and
/// WIDER than the operand (the wider one was already right — it widened the
/// region itself), and a primitive cast (`byte'` / `int'` fit in 8 / 32 bits, so
/// their products were right by width; they are the controls).
#[test]
fn a_cast_leaf_is_widened_to_the_region() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic signed [3:0] s4 = -3;
  function [31:0] f1; f1 = signed'(u8) * q8;   endfunction
  function [31:0] f2; f2 = unsigned'(s8) * q8; endfunction
  function [31:0] f3; f3 = int'(s4) * q8;      endfunction
  function [31:0] f4; f4 = byte'(s4) * q8;     endfunction
  function [31:0] f5; f5 = 8'(s8) * q8;        endfunction
  function [31:0] f6; f6 = 4'(s8) * q8;        endfunction
  function [31:0] f7; f7 = 16'(s8) * q8;       endfunction
  function [31:0] f8; f8 = 8'(u8) * b8;        endfunction
  initial begin
    $display("%h %h %h %h %h %h %h %h", f1(), f2(), f3(), f4(), f5(), f6(), f7(), f8());
    #1 $finish;
  end
endmodule
"#);
    // PRE: `00000020 00000020 00000060 00000060 00000020 00000020 00000120 00000009`.
    assert_eq!(
        o,
        "00000120 0000d820 00000060 00000060 00000120 ffffff20 00000120 0000f609"
    );
}

/// ③ The stamp's OPERAND is its own §11.6.1 region (self-determined): a
/// part-select, a sum that overflows at 8 bits, a 2-bit literal. And the
/// positions where the region was already open (a 32-bit sibling in a ternary
/// or a sum) are unchanged.
#[test]
fn the_stamps_operand_is_self_determined_and_open_regions_are_unchanged() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  function [31:0] f9;  f9  = $signed(u8[3:0]) * q8;                        endfunction
  function [31:0] f10; f10 = $signed(u8 + b8) * q8;                        endfunction
  function [31:0] f11; f11 = $signed(2'b11) * q8;                          endfunction
  function [31:0] f12; f12 = $unsigned(-1) * 2;                            endfunction
  function [31:0] f13; f13 = $signed(u8) ** 2;                             endfunction
  function [31:0] f14; f14 = ($signed(u8) > 0) ? $signed(u8) * q8 : 1;     endfunction
  function [31:0] f15; f15 = 1 ? $signed(u8) * q8 : 1;                     endfunction
  function [31:0] f16; f16 = $signed(u8) * q8 + 32'd1;                     endfunction
  initial begin
    $display("%h %h %h %h %h %h %h %h", f9(), f10(), f11(), f12(), f13(), f14(), f15(), f16());
    #1 $finish;
  end
endmodule
"#);
    // PRE: `00000020 00000040` for the first two; the rest unchanged.
    assert_eq!(
        o,
        "ffffff20 00000140 00000020 fffffffe 00000051 00000001 00000120 0000d821"
    );
}

/// ④ The SIZE-CAST consumer of the same walk moves the same way: the stamps and
/// casts under `16'(…)`, beside the cells that fit.
#[test]
fn the_size_cast_consumer_widens_the_same_leaves() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic signed [3:0] s4 = -3;
  logic [15:0] c1, c2, c3, c4, c5, c6, c7, c8, c9, c10;
  initial begin
    c1 = 16'($signed(u8) * q8);
    c2 = 16'($unsigned(s8) * q8);
    c3 = 16'($signed(u8) + q8);
    c4 = 16'($unsigned(s8) >> 1);
    c5 = 16'($signed(u8));
    c6 = 16'(8'(u8) * b8);
    c7 = 16'(signed'(u8) * q8);
    c8 = 16'(unsigned'(s8) * q8);
    c9 = 16'(int'(s4) * q8);
    c10 = 16'(byte'(s4) * q8);
    $display("%h %h %h %h %h %h %h %h %h %h", c1, c2, c3, c4, c5, c6, c7, c8, c9, c10);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `0020 0020 ffd7 007b fff7 0009 0020 0020 0060 0060`.
    assert_eq!(o, "0120 d820 ffd7 007b fff7 f609 0120 d820 0060 0060");
}

/// ⑤ BOUNDARIES, byte-identical to PRE. A user CALL leaf still stands the region
/// down (`00000009` for both oracles' `0000f609` — ROADMAP §2, its own slice),
/// and a stamp over an OPAQUE leaf (a hierarchical read) keeps the pre-slice
/// lowering on both consumers (`xx20` / `20` for the oracles' `0120` /
/// `00000120` — PRE-identical, including the placeholder's unknown print width;
/// the placeholder has no width to widen by, the same stand-down
/// `size_ctx_route` documents).
#[test]
fn a_call_leaf_and_an_opaque_leaf_still_stand_the_region_down() {
    let o = run(
        r#"module sub; logic [7:0] x = 8'hF7; logic signed [7:0] q = -32; endmodule
module t;
  sub u();
  logic signed [7:0] q8 = -32;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic [15:0] c1;
  function [7:0] id8(input [7:0] v); id8 = v; endfunction
  function [31:0] f_call; f_call = id8(u8) * b8; endfunction
  function [31:0] fo; fo = $signed(u.x) * q8; endfunction
  initial begin
    c1 = 16'($signed(u.x) * q8);
    $display("%h %h %h", f_call(), c1, fo());
    #1 $finish;
  end
endmodule
"#,
    );
    assert_eq!(o, "00000009 xx20 20");
}
