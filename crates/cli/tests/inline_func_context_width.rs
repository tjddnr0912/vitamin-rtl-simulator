//! A STATIC (non-`automatic`) simple-body function is normally INLINED (fold-by-
//! substitution). The inline path lowered each assignment RHS at its SELF width and
//! then resized to the target, so a WIDENING context-sensitive op — `f = s + u` into
//! a wider `f` (carry lost), `f = c << 4` (high bits shifted out), a `*`/`-` overflow
//! — was silently wrong (the frame/module net-assignment path evaluates at the §11.6
//! context width correctly). The fix detects such a body (`body_needs_context_width`)
//! and routes the function to the correct frame path via `build_frame_set`. A
//! NON-widening body (`f = a + b` same width, `f = a` widen-with-no-op, a concat)
//! stays inline (byte-identical). Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ctxw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn signed_plus_unsigned_wider_return() {
    // `s + u` (u unsigned ⇒ §11.8.1 unsigned) into a 16-bit return: 0xFF + 0x01 =
    // 0x100 = 256, NOT the self-width 8-bit `-1 + 1 = 0`. iverilog: 256.
    let (out, code) = run("module m;\n\
         function [15:0] f(input signed [7:0] s, input [7:0] u); f = s + u; endfunction\n\
         initial begin $display(\"%0d\", f(-8'sd1, 8'd1)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("256"), "signed+unsigned widening, got:\n{out}");
}

#[test]
fn left_shift_wider_return() {
    // `c << 4` into 16 bits: 0xAB << 4 = 0x0AB0, NOT the self-width 8-bit 0xB0.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] c); f = c << 4; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0ab0"), "left shift widening, got:\n{out}");
}

#[test]
fn signed_add_overflow_wider_return() {
    // `c + 8'sd1` with c = 8'h7F (+127) into signed 16-bit: 128, NOT the self-width
    // 8-bit overflow -128. iverilog: 128 / 0x0080.
    let (out, code) = run("module m;\n\
         function signed [15:0] f(input signed [7:0] c); f = c + 8'sd1; endfunction\n\
         initial begin $display(\"%0d %h\", f(8'h7F), f(8'h7F)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("128") && out.contains("0080"),
        "signed overflow, got:\n{out}"
    );
}

#[test]
fn multiply_wider_return() {
    // `a * b` = 200*200 = 40000 (needs 16 bits), NOT the self-width 8-bit 0x40=64.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a, input [7:0] b); f = a * b; endfunction\n\
         initial begin $display(\"%0d\", f(8'd200, 8'd200)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("40000"), "multiply widening, got:\n{out}");
}

#[test]
fn widening_via_body_local() {
    // A widening assignment to a body-LOCAL intermediate (`t = a * b`), then returned.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a, input [7:0] b);\n\
         logic [15:0] t; t = a * b; f = t; endfunction\n\
         initial begin $display(\"%0d\", f(8'd200, 8'd200)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("40000"),
        "local-intermediate widening, got:\n{out}"
    );
}

#[test]
fn subtract_borrow_wider_return() {
    // `a - b` = 1 - 3 = -2; unsigned 8-bit self-width = 0xFE = 254, but into a 16-bit
    // unsigned result it is 0xFFFE = 65534. iverilog: 65534.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a, input [7:0] b); f = a - b; endfunction\n\
         initial begin $display(\"%0d\", f(8'd1, 8'd3)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("65534"), "subtract borrow, got:\n{out}");
}

// ---- unfoldable / param-width targets: fail CLOSED (route conservatively) ----

#[test]
fn param_width_target_add() {
    // A `[W-1:0]` (param-width) return can't be decimal-folded, so the detector can't
    // prove it non-widening ⇒ must route (fail closed). W=16: 0xFF + 0x01 = 256, not
    // the self-width 8-bit 0. iverilog: 256.
    let (out, code) = run("module m #(parameter W=16);\n\
         function [W-1:0] f(input [7:0] a, input [7:0] b); f = a + b; endfunction\n\
         initial begin $display(\"%0d\", f(8'hFF, 8'h01)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("256"), "param-width target add, got:\n{out}");
}

#[test]
fn localparam_width_target_mul() {
    // Same, via a `localparam` return width and a multiply: 200*200 = 40000.
    let (out, code) = run("module m;\n\
         localparam W=16;\n\
         function [W-1:0] f(input [7:0] a, input [7:0] b); f = a * b; endfunction\n\
         initial begin $display(\"%0d\", f(8'd200, 8'd200)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("40000"), "localparam-width mul, got:\n{out}");
}

#[test]
fn time_return_mul() {
    // A `time` return is a 64-bit integral target: 100000*100000 = 10^10 needs 64
    // bits, not the 32-bit self-width product. iverilog: 10000000000.
    let (out, code) = run("module m;\n\
         function time f(input [31:0] a, input [31:0] b); f = a * b; endfunction\n\
         initial begin $display(\"%0d\", f(100000, 100000)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("10000000000"), "time-return mul, got:\n{out}");
}

#[test]
fn pow_literal_exponent_wider_return() {
    // `x ** 2` self-width is L(x) = 8 (§11.6.1), NOT max(8, 32-bit literal `2`). Into a
    // 16-bit return: 200**2 = 40000, not the self-width 8-bit 0x40 = 64. iverilog: 40000.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] x); f = x ** 2; endfunction\n\
         initial begin $display(\"%0d\", f(8'd200)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("40000"), "pow literal exponent, got:\n{out}");
}

#[test]
fn real_return_not_routed_as_bitwidth() {
    // A `real` return is NOT a bit-width context — real multiply must stay correct
    // whether inlined or (harmlessly) framed. 2.5 * 2.5 = 6.25.
    let (out, code) = run("module m;\n\
         function real f(input real a, input real b); f = a * b; endfunction\n\
         initial begin $display(\"%g\", f(2.5, 2.5)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("6.25"), "real return, got:\n{out}");
}

// ---- a narrow widening op nested under a FULL-WIDTH sibling (context width is set by
//      the sibling, not the whole-RHS self width — the gate must use CW per §11.6) ----

#[test]
fn widening_under_wide_bitwise_sibling() {
    // `(s+u) & 16'hFFFF`: the 16-bit mask lifts the context width to 16, so `s+u` must
    // evaluate at 16 bits = 256, NOT the 8-bit self-width 0 masked to 0. iverilog: 256.
    let (out, code) = run("module m;\n\
         function [15:0] f(input signed [7:0] s, input [7:0] u); f = (s + u) & 16'hFFFF; endfunction\n\
         initial begin $display(\"%0d\", f(-8'sd1, 8'd1)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("256"),
        "widening under wide sibling, got:\n{out}"
    );
}

#[test]
fn widening_under_wide_ternary_default() {
    // `c ? (s+u) : 16'h0`: the 16-bit else-branch lifts the context width to 16.
    let (out, code) = run("module m;\n\
         function [15:0] f(input signed [7:0] s, input [7:0] u, input c); f = c ? (s + u) : 16'h0; endfunction\n\
         initial begin $display(\"%0d\", f(-8'sd1, 8'd1, 1'b1)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("256"),
        "widening under wide ternary, got:\n{out}"
    );
}

#[test]
fn signed_shift_under_wide_sibling() {
    // A signed `>>` nested under a 16-bit OR: `(a >> 2) | 16'h0` with a = 8'hFF (-1) =
    // 0x003F at the 16-bit context, NOT 0xFFFF. iverilog: 003f.
    let (out, code) = run("module m;\n\
         function signed [15:0] f(input signed [7:0] a); f = (a >> 2) | 16'h0; endfunction\n\
         initial begin $display(\"%h\", f(8'hFF)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("003f"),
        "signed shift under wide sibling, got:\n{out}"
    );
}

#[test]
fn unsigned_xnor_into_wider() {
    // `a ~^ b` (XNOR) flips the high extension bits to 1 at the wider context (like
    // unary `~`): 0xAB ~^ 0xCD = 0xFF99 at 16 bits, NOT the 8-bit 0x99 → 0x0099.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a, input [7:0] b); f = a ~^ b; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB, 8'hCD)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("ff99"),
        "unsigned xnor into wider, got:\n{out}"
    );
}

// ---- non-widening bodies stay inline (correct; must not regress) ----

#[test]
fn same_width_widen_under_same_width_sibling_unchanged() {
    // `(a+b) & 8'hFF` into an 8-bit return: context width == 8 == the add's self width,
    // so it is NOT routed (no churn) and inline is already correct. 200+100 = 44 (mod).
    let (out, code) = run("module m;\n\
         function [7:0] f(input [7:0] a, input [7:0] b); f = (a + b) & 8'hFF; endfunction\n\
         initial begin $display(\"%0d\", f(8'd200, 8'd100)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("44"), "same-width masked add, got:\n{out}");
}

#[test]
fn plain_xor_into_wider_unchanged() {
    // `a ^ b` (XOR, not XNOR) is genuinely context-insensitive: high bits stay 0 either
    // way. 0xAB ^ 0xCD = 0x66 → 0x0066. Must stay inline / correct.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a, input [7:0] b); f = a ^ b; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB, 8'hCD)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0066"), "plain xor into wider, got:\n{out}");
}

#[test]
fn same_width_add_unchanged() {
    // `a + b` into an 8-bit return (self == context) — inline is already correct.
    let (out, code) = run("module m;\n\
         function [7:0] f(input [7:0] a, input [7:0] b); f = a + b; endfunction\n\
         initial begin $display(\"%0d\", f(8'd100, 8'd100)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("200"), "{out}");
}

// ---- routing a widening function to the frame path must keep an implicit
//      `always_comb`/`@(*)` sensitivity list correct ----

#[test]
fn comb_call_arg_sensitivity() {
    // `always_comb y = f(a,b)` with a widening (framed) `f`: the block MUST re-fire when
    // an arg changes — the frame call's arg expressions belong in the comb sensitivity
    // list. Only `b` toggles after `a` is stable; y must become 200*200 = 40000.
    let (out, code) = run("module m; logic [7:0] a, b; logic [15:0] y;\n\
         function [15:0] f(input [7:0] a, input [7:0] b); f = a * b; endfunction\n\
         always_comb y = f(a, b);\n\
         initial begin a = 8'd200; b = 0; #1 b = 8'd200; #2 $display(\"%0d\", y); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("40000"), "comb arg sensitivity, got:\n{out}");
}

#[test]
fn comb_body_global_reader_stays_inline() {
    // A widening body that READS a module-level signal `g` (not an arg) must stay INLINE
    // — else `g` would vanish from the comb sensitivity list and `y` would go stale when
    // only `g` changes. Here a=3 is stable and g toggles 4→10; y must track to 3*10 = 30.
    let (out, code) = run("module m; logic [7:0] a, g; logic [15:0] y;\n\
         function [15:0] f(input [7:0] a); f = a * g; endfunction\n\
         always_comb y = f(a);\n\
         initial begin a = 8'd3; g = 8'd4; #1 g = 8'd10; #2 $display(\"%0d\", y); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("30"),
        "global-reader comb sensitivity, got:\n{out}"
    );
}

#[test]
fn plain_widen_no_op_unchanged() {
    // `f = a` widened with NO op — the inline resize zero-extends correctly.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a); f = a; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("00ab"), "{out}");
}

#[test]
fn concat_self_determined_unchanged() {
    // A concat is self-determined width — no context widening, stays inline.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a, input [7:0] b); f = {a, b}; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB, 8'hCD)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("abcd"), "{out}");
}

#[test]
fn unsigned_right_shift_into_wider_stays_correct() {
    // `a >> 1` into a wider result routes conservatively (a SIGNED `>>` would drag the
    // sign extension into view — see `signed_right_shift_into_wider`). For an UNSIGNED
    // operand the value is unchanged either way: 0xAB >> 1 = 0x55. Churn-safe.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a); f = a >> 1; endfunction\n\
         initial begin $display(\"%h\", f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0055"), "{out}");
}

#[test]
fn signed_right_shift_into_wider() {
    // A SIGNED `>>` into a wider result: `a` is context-extended to 16 bits (0xFF80)
    // BEFORE the logical shift, so `a >> 1` = 0x7FC0, NOT the self-width 8-bit 0x40
    // zero-extended to 0x0040. iverilog: 7fc0.
    let (out, code) = run("module m;\n\
         function signed [15:0] f(input signed [7:0] a); f = a >> 1; endfunction\n\
         initial begin $display(\"%h\", f(8'h80)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("7fc0"),
        "signed right shift widening, got:\n{out}"
    );
}

#[test]
fn signed_divide_overflow_into_wider() {
    // Signed `a / b` at INT_MIN / -1: self-width 8-bit -128/-1 overflows to -128, but
    // at the 16-bit context it is +128 (0x0080). iverilog: 128.
    let (out, code) = run("module m;\n\
         function signed [15:0] f(input signed [7:0] a, input signed [7:0] b); f = a / b; endfunction\n\
         initial begin $display(\"%0d\", f(-8'sd128, -8'sd1)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("128"), "signed divide overflow, got:\n{out}");
}
