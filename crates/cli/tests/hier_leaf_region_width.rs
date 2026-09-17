//! A HIERARCHICAL net or call inside a §11.6.1 width region is sized and signed
//! through the instance's module declaration.
//!
//! A cross-instance read (`u.hs`) and a cross-instance call (`u.hf(x)`) lower to a
//! PLACEHOLDER — the child instance is elaborated after the referring body, so no
//! net and no `FuncId` exists yet. `has_opaque_leaf` therefore reported every such
//! leaf as unresolvable, which left the whole region on the pre-slice classifier
//! and handed `lower_size_leaf` a node whose `ir_bits_of` is `None`; the
//! fabricated 32 then built `select_low(x, n)` over what is at run time an 8-bit
//! net, so bits 8..n-1 read as `x`: `16'(u.hs * q8)` printed `0000xx20` where both
//! oracles print `00000120`, and the case twin `case (16'(u.hs * sq8))
//! 16'h0120:` missed its arm. The declaration answers both questions, so a leaf
//! whose instance path names plain body instances and whose net/function
//! declaration folds to a literal width now carries its declared sign and width
//! into the region, and `lower_size_leaf` sizes the placeholder from that instead
//! of from 32.
//!
//! What still declines, keeping its pre-slice value (④): a range that is not a
//! literal (`[W-1:0]`, with or without an instantiation override), an instance
//! reached through a generate scope, a reference made from inside a generate body,
//! an upward reference, and an instance array.
//!
//! ORACLES: iverilog 13.0 (`-g2012`) and verilator 5.052 (`--binary --timing`);
//! every value below was measured in both, and they agree on every cell. PRE
//! values are from a release binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hierreg_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// ① THE HEADLINE: a hierarchical NET (`u.hs`) and a hierarchical CALL
/// (`u.hf(s8)`) as a region leaf, mixed and all-signed, over `*` and `+`, at a
/// cast wider and narrower than 16, through a two-level instance path
/// (`m.u2.hs`), under a `$signed` stamp, as a part-select and as an unpacked
/// ARRAY ELEMENT — and the two consumers that reuse the same walk: an inline
/// function body and a `case` selector.
#[test]
fn a_hierarchical_leaf_is_sized_through_the_instance_module() {
    let o = run(r#"module sub;
  logic signed [7:0] hs = 8'sd16; logic [7:0] hu = 8'd16;
  logic signed [7:0] arr [0:1] = '{16, 3};
  function automatic logic signed [7:0] hf(input logic signed [7:0] a); return a; endfunction
endmodule
module mid; sub u2(); endmodule
module t;
  sub u(); mid m();
  logic signed [7:0] s8 = 8'sd16; logic [7:0] q8 = 8'd18; logic signed [7:0] sq8 = 8'sd18;
  logic [31:0] o1,o2,o3,o4,o5,o6,o7,o8,o9,o10,o11,o12,o13;
  function automatic logic [15:0] fb(input logic signed [7:0] a); return 16'(u.hs * a); endfunction
  function logic [15:0] fc(input logic signed [7:0] a); fc = 16'(u.hs * a); endfunction
  initial begin
    o1 = 16'(u.hs * q8);
    o2 = 16'(u.hf(s8) * sq8);
    o3 = 16'(u.hf(s8) * q8);
    o4 = 16'(m.u2.hs * sq8);
    o5 = 16'($signed(u.hu) * sq8);
    o6 = 16'(u.hs[3:0] * sq8);
    o7 = 16'(u.arr[0] * sq8);
    o8 = fb(sq8);
    o9 = fc(sq8);
    o10 = 16'(u.hs + sq8);
    o11 = 12'(u.hs + sq8);
    o12 = 16'(u.hs * sq8 + u.hf(s8));
    o13 = 16'(u.hs * s8);
    $display("A=%h %h %h %h %h %h %h", o1,o2,o3,o4,o5,o6,o7);
    $display("B=%h %h %h %h %h %h", o8,o9,o10,o11,o12,o13);
    case (16'(u.hs * sq8)) 16'h0120: $display("C=1"); 16'h0020: $display("C=2"); default: $display("C=0"); endcase
    $finish;
  end
endmodule
"#);
    // PRE: `A=0000xx20 0000xx20 0000xx20 0000xx20 xxxxxx20 0000xxxx 0000xxxx`
    //      `B=0000xx20 0000xx20 0000xx22 00000x22 0000xx30 0000xx00`, `C=0`.
    assert_eq!(
        o,
        "A=00000120 00000120 00000120 00000120 00000120 00000000 00000120\n\
         B=00000120 00000120 00000022 00000022 00000130 00000100\n\
         C=1"
    );
}

/// ② A hierarchical PORT — the declaration lives in the child's ANSI header, not
/// its body, and an `input` and an `output` port read the same way. Beside them
/// the INSTANCE ARRAY element `ua[1].hs`, which declines and keeps its PRE value
/// (an array instance is not a plain body instance); it needs a child with ANSI
/// ports, because `sub ua [0:1] ()` on a portless child is refused outright.
#[test]
fn a_hierarchical_port_carries_its_header_declaration() {
    let o = run(
        r#"module sub(input logic signed [7:0] pin, output logic signed [7:0] pout);
  assign pout = pin;
  logic signed [7:0] hs = 8'sd16;
endmodule
module t;
  logic signed [7:0] s8 = 8'sd16; logic [7:0] q8 = 8'd18; logic signed [7:0] sq8 = 8'sd18;
  logic signed [7:0] w;
  sub u(.pin(s8), .pout(w));
  sub ua [0:1] (.pin(s8), .pout());
  logic [31:0] o1, o2, o3;
  initial begin
    #1;
    o1 = 16'(u.pin * q8);
    o2 = 16'(u.pout * q8);
    o3 = 16'(ua[1].hs * sq8);
    $display("P=%h %h %h", o1, o2, o3);
    $finish;
  end
endmodule
"#,
    );
    // PRE: `P=0000xx20 0000xx20 0000xx20`; both oracles: `00000120` for all three,
    // so the instance-array cell is a decline, not a fix.
    assert_eq!(o, "P=00000120 00000120 0000xx20");
}

/// ③ CONTROLS the fix must not move. `4'(u.hs * q8)` narrows (the region is 8
/// bits wide, the cast keeps the low 4); `32'(u.hf(s8) * q8)` was already right
/// because 32 is exactly the width the old fabricated default assumed;
/// `16'(s8 * sq8)` holds no hierarchical leaf at all; and the plain-assignment
/// spellings of the same reads — the row's originally filed cells — were correct
/// before this slice and stay so, `case` included.
#[test]
fn the_narrowing_n32_and_plain_assignment_controls_do_not_move() {
    let o = run(r#"module sub;
  logic signed [7:0] hs = 8'sd16;
  function automatic logic signed [7:0] hf(input logic signed [7:0] a); return a; endfunction
endmodule
module t;
  sub u();
  logic signed [7:0] s8 = 8'sd16; logic [7:0] q8 = 8'd18; logic signed [7:0] sq8 = 8'sd18;
  function automatic logic signed [7:0] fas8(input logic signed [7:0] a); return a; endfunction
  logic [31:0] o1,o2,o3,o4,o5,o6,o7;
  initial begin
    o1 = 4'(u.hs * q8);
    o2 = 32'(u.hf(s8) * q8);
    o3 = 16'(s8 * sq8);
    o4 = u.hf(s8) * q8;
    o5 = fas8(u.hs) * q8;
    o6 = u.hs * q8;
    o7 = u.hs * sq8;
    $display("K=%h %h %h %h %h %h %h", o1,o2,o3,o4,o5,o6,o7);
    case (u.hs * q8) 32'h120: $display("C=1"); 32'h20: $display("C=2"); default: $display("C=0"); endcase
    $finish;
  end
endmodule
"#);
    // PRE: `K=00000000 00000120 00000120 00000120 00000120 00000120 00000120`, `C=1`
    // — every cell here is already the oracle value before the slice.
    assert_eq!(
        o,
        "K=00000000 00000120 00000120 00000120 00000120 00000120 00000120\nC=1"
    );
}

/// ④ THE DECLINES. Each of these keeps its PRE value; the oracles print
/// `00000120` for all five.
///
///  * `u.hw` / `up.hw` — the net's range is `[W-1:0]`. The width of a parameter
///    in ANOTHER module's scope is not resolvable from this module's declarations
///    (`ast_kind_range_width` folds decimal literals only), and an instantiation
///    override does not change that, so both the default-`W` and the
///    `#(.W(12))` spelling decline. The overridden one prints one fewer `x`
///    because its run-time net is 12 bits, not 8.
///  * `g.u.hs` — the instance lives in a generate block; a generate-scoped
///    instance is not a body instance of the module.
///  * the read inside `generate ... begin : h`, where a generate-local scope could
///    shadow the module-level `u` this walk would resolve.
///  * `t.s8` from inside `sup` — an upward reference, which this walk only ever
///    looks downward for.
///
/// (The instance-ARRAY decline is in ②, where the child has the ANSI ports an
/// instance array requires.)
#[test]
fn a_leaf_this_walk_cannot_reach_keeps_its_pre_slice_value() {
    let o = run(r#"module sub #(parameter W = 8);
  logic signed [W-1:0] hw = 16; logic signed [7:0] hs = 8'sd16;
endmodule
module sup;
  logic [31:0] up1;
  initial begin #1; up1 = 16'(t.s8 * t.sq8); $display("U=%h", up1); end
endmodule
module t;
  sub u(); sub #(.W(12)) up(); sup su();
  logic signed [7:0] s8 = 8'sd16; logic signed [7:0] sq8 = 8'sd18;
  logic [31:0] o1,o2,o3,o4;
  generate if (1) begin : g sub gu(); end endgenerate
  generate if (1) begin : h
    initial begin #1; o4 = 16'(u.hs * sq8); end
  end endgenerate
  initial begin
    #2;
    o1 = 16'(u.hw * sq8);
    o2 = 16'(up.hw * sq8);
    o3 = 16'(g.gu.hs * sq8);
    $display("D=%h %h %h %h", o1, o2, o3, o4);
    $finish;
  end
endmodule
"#);
    // PRE (identical to POST — that is the claim): `U=0000xx20`,
    // `D=0000xx20 0000x120 0000xx20 0000xx20`. Both oracles print `00000120`
    // for every one of them.
    assert_eq!(o, "U=0000xx20\nD=0000xx20 0000x120 0000xx20 0000xx20");
}
