//! The case selector and every case item are ONE §12.5 region: each is sized to
//! the common maximum width and evaluated there with the collective sign.
//!
//! `lower_case` lowered the selector and every item SELF-determined and left the
//! sizing to the engine's per-pair `CaseEq`, which widens the 8-bit RESULT of an
//! operator, not its operands: `case (a8 * b8) 32'h0000fe01: …` compared the 8-bit
//! product `01` and took the `32'h00000001` item where both oracles take `fe01`;
//! `case (s8 * q8) 32'sd288:` (all signed) compared `20`; `case (a8 << 4)
//! 32'hff0:` compared `f0`; `case (-n4) 32'hfffffff1:` compared `1`. An operator
//! selector or item is now re-lowered in the common width through the size-cast /
//! inline-body context walk (`inline_ctx_ext`), with the extension forced to the
//! collective sign (`case (s8 + q8) 9'h1d7:` zero-extends: signed operands, an
//! unsigned item). A leaf, a concat, a real or string selector, a fill and an
//! opaque leaf keep their lowering.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! value below was measured in both (iverilog aborts on the string / real
//! selector file, so those two cells are verilator + the LRM). PRE values are from
//! a release binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_casereg_{}_{n}", std::process::id()));
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

/// ① THE HEADLINE: a call product, a plain product, a stamped product, the signed
/// twins, a size-cast selector (already right), and the same on `casez` / `casex`.
#[test]
fn an_operator_selector_is_evaluated_at_the_common_width() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  logic [31:0] o1,o2,o3,o4,o5,o6,p1,p2,p4;
  function automatic signed [7:0] fas8(input signed [7:0] v); fas8 = v; endfunction
  function automatic [7:0] fa8(input [7:0] v); fa8 = v; endfunction
  initial begin
    case (fa8(a8) * b8) 32'h0000fe01: o1 = 1; 32'h00000001: o1 = 2; default: o1 = 3; endcase
    case (a8 * b8) 32'h0000fe01: o2 = 1; 32'h00000001: o2 = 2; default: o2 = 3; endcase
    case ($unsigned(a8) * b8) 32'h0000fe01: o3 = 1; 32'h00000001: o3 = 2; default: o3 = 3; endcase
    case (fas8(s8) * q8) 32'h00000120: o4 = 1; 32'h00000020: o4 = 2; default: o4 = 3; endcase
    case (s8 * q8) 32'h00000120: o5 = 1; 32'h00000020: o5 = 2; 32'hffffff20: o5 = 4; default: o5 = 3; endcase
    case (16'(a8 * b8)) 16'hfe01: o6 = 1; 16'h0001: o6 = 2; default: o6 = 3; endcase
    casez (fa8(a8) * b8) 32'h0000fe01: p1 = 1; 32'h00000001: p1 = 2; default: p1 = 3; endcase
    casex (a8 * b8) 32'h0000fe01: p2 = 1; 32'h00000001: p2 = 2; default: p2 = 3; endcase
    case (a8 + b8) 9'h1fe: p4 = 1; 8'hfe: p4 = 2; default: p4 = 3; endcase
    $display("O=%0d %0d %0d %0d %0d %0d P=%0d %0d %0d", o1,o2,o3,o4,o5,o6,p1,p2,p4);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `O=2 2 2 2 2 1 P=2 2 2`.
    assert_eq!(o, "O=1 1 1 3 3 1 P=1 1 1");
}

/// ② The collective sign and the other operator shapes: an all-signed set, items
/// narrower than the selector, an operator ITEM, a shift, a concat (self-determined),
/// a ternary (already right), a signed selector under an unsigned item, unary minus,
/// an arithmetic shift, and a case inside an `always_comb` and a function body.
#[test]
fn the_collective_sign_and_the_operator_shapes() {
    let o = run(r#"module t;
  logic signed [7:0] s8 = -9, q8 = -32; logic [7:0] a8 = 8'hFF, b8 = 8'hFF; logic [3:0] n4 = 4'hF;
  logic [31:0] o1,o2,o3,o4,o5,o6,o7,o8,t1,t2,t3,r1,r2;
  function automatic signed [7:0] fas8(input signed [7:0] v); fas8 = v; endfunction
  function automatic [31:0] f(input [7:0] x);
    case (x * b8) 32'h0000fe01: f = 1; 32'h00000001: f = 2; default: f = 3; endcase
  endfunction
  always_comb begin
    case (a8 * b8) 32'h0000fe01: r1 = 1; 32'h00000001: r1 = 2; default: r1 = 3; endcase
  end
  initial begin
    case (s8 * q8) 32'sd288: o1 = 1; 32'sd32: o1 = 2; default: o1 = 3; endcase
    case (a8 * b8) 8'h01: o2 = 1; 16'hfe01: o2 = 2; default: o2 = 3; endcase
    case (a8) (a8 + b8): o3 = 1; 8'hfe: o3 = 2; default: o3 = 3; endcase
    case (a8 << 4) 32'hff0: o4 = 1; 8'hf0: o4 = 2; default: o4 = 3; endcase
    case ({a8, b8}) 16'hffff: o5 = 1; default: o5 = 3; endcase
    case (n4 ? a8 * b8 : 32'd0) 32'h0000fe01: o6 = 1; 32'h00000001: o6 = 2; default: o6 = 3; endcase
    case (s8 + q8) 32'hffffffd7: o7 = 1; 9'h1d7: o7 = 2; 8'hd7: o7 = 4; default: o7 = 3; endcase
    case (-n4) 32'hfffffff1: o8 = 1; 4'h1: o8 = 2; default: o8 = 3; endcase
    case (a8) 32'h0000_00ff: t1 = 1; (a8 * b8): t1 = 2; default: t1 = 3; endcase
    case (s8 * q8) 16'sd288: t2 = 1; default: t2 = 3; endcase
    case (s8 <<< 4) 32'shffff_ff70: t3 = 1; 12'hf70: t3 = 2; default: t3 = 3; endcase
    #1 r2 = f(a8);
    $display("Q=%0d %0d %0d %0d %0d %0d %0d %0d T=%0d %0d %0d R=%0d %0d", o1,o2,o3,o4,o5,o6,o7,o8,t1,t2,t3,r1,r2);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `Q=2 1 3 2 1 1 4 2 T=1 1 2 R=2 2`.
    assert_eq!(o, "Q=1 2 3 1 1 1 2 1 T=1 1 2 R=1 1");
}

/// ③ Controls that keep their lowering: a string and a real selector, an all-8-bit
/// set, a bitwise operator, a parameter operator, an x operand, a 4-bit product and
/// a power (verilator; iverilog aborts on the string / real cells).
#[test]
fn the_controls_keep_their_lowering() {
    let o = run(r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF; logic [3:0] n4 = 4'hF;
  string str = "ab"; real r = 2.5; logic [31:0] o1,o2,o3,o4,o5,o6,o7,o8,o9;
  logic [7:0] ux = 8'hFx;
  parameter P = 8'hFF;
  initial begin
    case (str) "ab": o1 = 1; default: o1 = 3; endcase
    case (r) 2.5: o2 = 1; default: o2 = 3; endcase
    case (a8 * b8) 8'hfe, 8'h01: o3 = 1; default: o3 = 3; endcase
    case (a8 & b8) 32'hff: o4 = 1; default: o4 = 3; endcase
    case (P * 2) 32'h1fe: o5 = 1; 8'hfe: o5 = 2; default: o5 = 3; endcase
    case (a8 * b8) 32'h0000fe01: o6 = 1; default: o6 = 3; endcase
    case (ux * b8) 32'h0000fe01: o7 = 1; default: o7 = 3; endcase
    case (n4 * n4) 8'he1: o8 = 1; 4'h1: o8 = 2; default: o8 = 3; endcase
    case (a8 ** 2) 32'hfe01: o9 = 1; default: o9 = 3; endcase
    $display("S=%0d %0d %0d %0d %0d %0d %0d %0d %0d", o1,o2,o3,o4,o5,o6,o7,o8,o9);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "S=1 1 1 1 1 1 3 1 1");
}
