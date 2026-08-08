//! §4.5.212: a size cast `N'(expr)` over a CONTEXT-DETERMINED operation now runs the
//! operation at N bits (was: computed at the operands' self-width, then zero/sign-extended
//! — silently dropping the carry). `8'(a*b)` with a=15,b=3 gave 13; iverilog gives 45.
//!
//! ORACLE: iverilog 13.0 IS a differential oracle for size-cast width/sign (unlike the
//! array-formal slices), so every case here is iverilog-verified.
//!
//! Mechanism: `lower_size_ctx` recurses the context-determined operator structure
//! (arith/bitwise both operands; shift/`**` base only; unary `+`/`-`/`~`; ternary branches),
//! widening each self-determined LEAF to N before the operation. The extension sign is the
//! operand's OVERALL sign (`ast_ctx_signed` — signed iff every leaf is signed; §11.8.1 makes
//! a mixed expression unsigned, zero-extending all leaves, verified against iverilog). A leaf
//! whose sign can't be resolved here (a param/call) keeps the old fill-only path (no
//! regression). Self-determined operands (a bare leaf, select, concat, comparison) are
//! byte-identical to before.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_scw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── unsigned context-determined operations (iverilog-verified) ───────────────

#[test]
fn unsigned_multiply() {
    // a=15, b=3 → 45 (was 13 = 45 mod 16).
    let o = run("module t; logic [3:0] a=4'hF,b=4'h3; initial begin $display(\"%0d\", 8'(a*b)); $finish; end endmodule");
    assert_eq!(o, "45");
}

#[test]
fn unsigned_add_carry() {
    // 15+1 in 5-bit context → 10000 (16), not 00000.
    let o = run("module t; logic [3:0] a=4'hF; initial begin $display(\"%b\", 5'(a+4'h1)); $finish; end endmodule");
    assert_eq!(o, "10000");
}

#[test]
fn unsigned_shift_left() {
    // 15<<1 in 6-bit → 011110 (30), not 001110 (14).
    let o = run("module t; logic [3:0] a=4'hF; initial begin $display(\"%b\", 6'(a<<1)); $finish; end endmodule");
    assert_eq!(o, "011110");
}

#[test]
fn unsigned_subtract_borrow() {
    // 1-2 in 5-bit → 11111 (31), not 01111 (15).
    let o = run("module t; logic [3:0] a=4'h1; initial begin $display(\"%b\", 5'(a-4'h2)); $finish; end endmodule");
    assert_eq!(o, "11111");
}

#[test]
fn unsigned_multiply_16() {
    // 255*255 in 16-bit → fe01, not 0001.
    let o = run("module t; logic [7:0] a=8'hFF,b=8'hFF; initial begin $display(\"%h\", 16'(a*b)); $finish; end endmodule");
    assert_eq!(o, "fe01");
}

#[test]
fn unsigned_shift_variable_amount() {
    // 15<<2 in 7-bit → 0111100 (60); the shift amount is self-determined.
    let o = run("module t; logic [3:0] a=4'hF,n=4'h2; initial begin $display(\"%b\", 7'(a<<n)); $finish; end endmodule");
    assert_eq!(o, "0111100");
}

#[test]
fn unsigned_nested_arith() {
    // 15*2+3 in 8-bit → 33, not 1.
    let o = run("module t; logic [3:0] a=4'hF,b=4'h2,c=4'h3; initial begin $display(\"%0d\", 8'(a*b+c)); $finish; end endmodule");
    assert_eq!(o, "33");
}

#[test]
fn unsigned_divmod() {
    let o = run("module t; logic [7:0] a=200,b=7; initial begin $display(\"%0d %0d\", 16'(a/b), 16'(a%b)); $finish; end endmodule");
    assert_eq!(o, "28 4");
}

#[test]
fn unsigned_power() {
    // 3**3 in 16-bit → 27.
    let o = run("module t; logic [3:0] a=4'h3; initial begin $display(\"%0d\", 16'(a**3)); $finish; end endmodule");
    assert_eq!(o, "27");
}

// ── signed context-determined operations ─────────────────────────────────────

#[test]
fn signed_multiply() {
    // signed -3 * 5 → -15 (was 1). All leaves signed ⇒ sign-extend.
    let o = run("module t; logic signed [3:0] a=-3,b=5; initial begin $display(\"%0d\", 8'(a*b)); $finish; end endmodule");
    assert_eq!(o, "-15");
}

#[test]
fn signed_arith_shift_left() {
    // signed -8 <<< 1 → -16 (was 0).
    let o = run("module t; logic signed [3:0] a=-8; initial begin $display(\"%0d\", 8'(a<<<1)); $finish; end endmodule");
    assert_eq!(o, "-16");
}

#[test]
fn signed_division() {
    // signed division must use signed semantics (the widened operands re-stamp $signed).
    let o = run("module t; logic signed [7:0] a=-100,b=7; initial begin $display(\"%0d %0d\", 16'(a/b), 16'(a%b)); $finish; end endmodule");
    assert_eq!(o, "-14 -2");
}

#[test]
fn signed_arith_shift_right() {
    // -64 >>> 2 → -16 (arithmetic, sign preserved through the widening).
    let o = run("module t; logic signed [7:0] a=-64; initial begin $display(\"%0d\", 16'(a>>>2)); $finish; end endmodule");
    assert_eq!(o, "-16");
}

#[test]
fn signed_unary_negate() {
    let o = run("module t; logic signed [3:0] a=3; initial begin $display(\"%0d\", 8'(-a)); $finish; end endmodule");
    assert_eq!(o, "-3");
}

// ── mixed-sign: §11.8.1 makes the whole expression unsigned (zero-extend all) ─

#[test]
fn mixed_sign_nested_is_unsigned() {
    // (signed -1 * signed 1) + unsigned 0 → the UNSIGNED c makes the whole expr unsigned,
    // so the signed pair is ZERO-extended → 15, not 255. (iverilog-verified.)
    let o = run("module t; logic signed [3:0] a=-1,b=1; logic [3:0] c=0; initial begin $display(\"%0d\", 8'(a*b+c)); $finish; end endmodule");
    assert_eq!(o, "15");
}

#[test]
fn mixed_sign_multiply_is_unsigned() {
    // signed -3 * unsigned 5 → unsigned ⇒ a zero-extended to 13, 13*5=65.
    let o = run("module t; logic signed [3:0] a=-3; logic [3:0] b=5; initial begin $display(\"%0d\", 8'(a*b)); $finish; end endmodule");
    assert_eq!(o, "65");
}

// ── ternary / unary / nested cast ────────────────────────────────────────────

#[test]
fn ternary_branches_context_determined() {
    let o = run("module t; logic [3:0] a=4'hF,b=4'h3; logic s=1; initial begin $display(\"%0d\", 8'(s?a*b:a+b)); $finish; end endmodule");
    assert_eq!(o, "45");
}

#[test]
fn nested_size_cast() {
    // 8'(a*b)=45, then 16'(45*2)=90.
    let o = run("module t; logic [3:0] a=4'hF,b=4'h3; initial begin $display(\"%0d\", 16'(8'(a*b)*2)); $finish; end endmodule");
    assert_eq!(o, "90");
}

// ── correctness of neighbouring cases (regression guards) ────────────────────

#[test]
fn narrowing_truncates() {
    // 4'(255+255) → the low 4 bits of 510 = e.
    let o = run("module t; logic [7:0] a=8'hFF,b=8'hFF; initial begin $display(\"%h\", 4'(a+b)); $finish; end endmodule");
    assert_eq!(o, "e");
}

#[test]
fn leaf_operand_unchanged() {
    // A bare leaf operand is self-determined (already correct) — byte-identical to before.
    let o = run("module t; logic [3:0] a=4'h5; initial begin $display(\"%h %h\", 8'(a), 4'(8'hAB)); $finish; end endmodule");
    assert_eq!(o, "05 b");
}

#[test]
fn comparison_inside_is_one_bit() {
    // a>0 is a 1-bit unsigned result; the cast zero-extends it.
    let o = run("module t; logic [3:0] a=4'hF; initial begin $display(\"%0d\", 8'(a>4'h0)); $finish; end endmodule");
    assert_eq!(o, "1");
}

#[test]
fn fill_literal_still_grows() {
    // `8'('1)` — a fill operand is self-determined; the pre-existing fill path fills to 8.
    let o = run("module t; initial begin $display(\"%b\", 8'('1)); $finish; end endmodule");
    assert_eq!(o, "11111111");
}

#[test]
fn signed_add_no_overflow_unchanged() {
    // A signed add that doesn't overflow its self-width was already correct — still is.
    let o = run("module t; logic signed [3:0] a=-4,b=2; initial begin $display(\"%0d\", 8'(a+b)); $finish; end endmodule");
    assert_eq!(o, "-2");
}

// ── the OTHER direction: a narrowing context (§11.8.1 is max(self, N)) ────────
//
// §4.5.212 passed the cast width N straight down as the operand context. That is
// right when N WIDENS — the carry survives, which is what these tests above buy —
// and nobody measured that the same code also NARROWS. For `+ - * & | ^ << **`
// the difference is invisible: their low N bits are decided by the operands' low N
// bits, so "evaluate at N" and "evaluate wide, then truncate" agree. For `/ % >>
// >>>` they are different computations, because a quotient's low bits are decided
// by bits ABOVE them.
//
// ⚠️ THE FIRST ATTEMPT AT THIS FIX WAS REVERTED. Sending those four down a
// self-determined path fixes narrowing and breaks widening and sign propagation —
// measured 886 cells fixed against 856 broken, a trade between silent wrongs. The
// rule is max(self, N) in BOTH directions, so every test below has a narrowing
// cell AND a widening cell; a test with only one is what let the trade through.
//
// Oracles: iverilog 13.0 and verilator 5.050 agree on every expected value here.

/// NARROW. `integer k` is 32 bits, so a 2- or 3-bit cast narrows: the operation
/// must still run at 32. vita gave `xx` (a 2-bit context turned the divisor `4`
/// into `0`), `111`, `00`, `01`.
#[test]
fn a_narrowing_cast_does_not_shrink_a_divide_or_shift() {
    let o = run("module t; integer k; initial begin k=7;\n\
        $display(\"%b %b %b %b\", 2'(k%4), 3'(k%4), 2'(k/2), 2'(k>>1)); $finish; end endmodule");
    assert_eq!(o, "11 011 11 11");
}

/// WIDEN — the half the reverted attempt broke. The context's bits are part of the
/// answer: `b` is `-4'sd8`, so `8'(b>>1)` sign-extends to 8 first and shifts
/// logically (`01111100`), and `8'(b/c)` with `c = -4'sd1` overflows at 4 bits but
/// not at 8. The self-determined path gave `00000100` and `11111000`.
#[test]
fn a_widening_cast_still_evaluates_the_operation_at_the_cast_width() {
    let o = run(
        "module t; logic signed [3:0] b, c; initial begin b=-4'sd8; c=-4'sd1;\n\
        $display(\"%b %b\", 8'(b>>1), 8'(b/c)); $finish; end endmodule",
    );
    assert_eq!(o, "01111100 00001000");
}

/// The context's SIGNEDNESS propagates in both directions — an unsigned sibling
/// makes the nested division unsigned. The reverted attempt lowered the node with
/// its own operand signs and re-stamped afterwards, giving `255` and `1110`.
#[test]
fn the_cast_context_signedness_reaches_a_nested_divide() {
    let o = run("module t; logic signed [7:0] a, b; logic [7:0] u;\n\
        logic [3:0] p; logic signed [3:0] q, r;\n\
        initial begin a=-8'sd7; b=8'sd4; u=8'd0; p=4'd1; q=-4'sd7; r=4'sd4;\n\
        $display(\"%b %b\", 8'((a/b)+u), 4'(p+(q%r))); $finish; end endmodule");
    assert_eq!(o, "00111110 0010");
}

/// Nesting reaches the rule from every enclosing shape, so the fix lives inside the
/// recursion rather than at its entry. vita gave `01`, `xx`, `00`.
#[test]
fn a_divide_nested_under_add_mul_or_a_ternary_arm_is_still_self_determined() {
    let o = run("module t; integer k; initial begin k=7;\n\
        $display(\"%b %b %b\", 2'((k/2)+1), 2'((k%4)*1), 2'(k ? k/2 : 0)); $finish; end endmodule");
    assert_eq!(o, "00 11 11");
}

/// A division by zero is `x` across ALL N bits — the widened form must not produce
/// the `x` at the self width and then zero-extend it. vita gave `0xxxxxxxx`.
/// (iverilog only: verilator's `--binary` is two-state and prints zeros for x.)
#[test]
fn a_widened_divide_by_zero_is_unknown_across_the_whole_cast_width() {
    let o = run(
        "module t; logic [7:0] a, b; initial begin a=8'd0; b=8'd0;\n\
        $display(\"%b\", 9'(a/b)); $finish; end endmodule",
    );
    assert_eq!(o, "xxxxxxxxx");
}

/// §11.8.1 propagates the SIGNEDNESS even when the context does not widen, and for
/// `>>>` that changes the operation: an unsigned left operand shifts LOGICALLY
/// (§11.4.10). An unsigned sibling anywhere in the cast operand makes the whole
/// expression unsigned, so the arithmetic fill must not reach the low N bits.
///
/// This is the cell the first `max(self, N)` implementation missed — it kept the
/// node's own width correctly and dropped its sign, so `4'(u8 + (s8 >>> 9))` printed
/// `0010` where both oracles say `0011`. `/ % >>` never showed it because the plain
/// path already carries the context's unsignedness to them.
///
/// iverilog 13.0 and verilator 5.050 agree on all six.
#[test]
fn an_unsigned_context_makes_a_narrowed_arithmetic_shift_logical() {
    let o = run("module t;\n\
        logic signed [7:0] s8; logic [7:0] u8; logic signed [3:0] s4;\n\
        logic [2:0] u3; logic [15:0] u16;\n\
        initial begin s8=-8'sd9; u8=8'd3; s4=-4'sd5; u3=3'd2; u16=16'd7;\n\
        $display(\"%b %b %b %b %b %b\",\n\
          4'(u8 + (s8 >>> 9)), 5'(u16 & (s8 >>> 3)), 4'(u8 * (s8 >>> 1)),\n\
          4'({1'b0,u3} + (s8 >>> 4)), 4'(u8 + (s8 >>> u3)), 4'(s4 + (s8 >>> 9)));\n\
        $finish; end endmodule");
    // The last cell keeps a SIGNED context (both operands signed) — it must stay
    // arithmetic, which is what makes the other five mean something.
    assert_eq!(o, "0011 00110 0001 0001 0000 1010");
}

/// An unsized fill inside a narrowed cast operand sizes to the EXPRESSION's width
/// (§11.6), which in the narrowing branch is `max(self, N)` — never `N`. Passing `N`
/// there built `'1` as two ones instead of thirty-two, so `2'(k / '1)` printed `2`
/// where both oracles print `0`. iverilog + verilator: `0 3 0`.
#[test]
fn a_fill_inside_a_narrowed_cast_sizes_to_the_expression_width() {
    let o = run("module t; integer k; logic [7:0] a;\n\
        initial begin k=7; a=8'd200;\n\
        $display(\"%0d %0d %0d\", 2'(k / '1), 2'(k % '1), 4'(a / '1)); $finish; end endmodule");
    assert_eq!(o, "0 3 0");
}
