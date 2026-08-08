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
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Like `run`, but hands back stderr and the exit code too — the refusal tests need
/// to see that the design DID fail, not merely that stdout is empty.
fn run_status(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_scws_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let r = (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    );
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn run(src: &str) -> String {
    run_status(src)
        .0
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

/// §4.5.317: a REAL value is refused at every operand position the SIZE-CONTEXT
/// LOWERING builds. It used to depend on which arm of the lowering the operand
/// happened to take — `/`, `%` and `**` on a bare real were loud (they are in
/// `expr_is_real`'s `Binary` arm, so the cast arm's own check saw them) while `*`,
/// `+`, `-` and unary `-` were silently wrong: `4'(r*2)` with `r = 7.5` printed
/// `0000` where the value is 15 = `1111`. Guarding only the LEAF fixed those four
/// and left three more families quiet, which is why the refusal is a funnel:
///
/// * the shift AMOUNT (`4'(u << r)`) is lowered with a plain `lower_expr`, and the
///   very same `u << r` OUTSIDE a cast was already loud;
/// * the `/`|`%` NARROWING branch (`8'(u / r)`, §4.5.316) lowers its operands with
///   `lower_ctx_or_plain`, which never reaches the leaf guard — it printed
///   `00110011` (the f64 read as bits) at exit 0;
/// * `$signed`/`$unsigned` are transparent to the value's domain, so
///   `4'($signed(r)*2)` printed `0000` while the uncast `$signed(r)*2` printed the
///   real-domain 15 — two answers to one expression inside one design.
///
/// ⚠️ NOT a claim of completeness. When `ast_ctx_signed` cannot resolve a leaf (a
/// real param / literal / function return / `$realtime` / `$sqrt`) the cast never
/// enters `lower_size_ctx` at all, and if the operator is also outside
/// `expr_is_real`'s `Binary` arm the cast stays silent: `4'(RP ^ '0)` with
/// `parameter real RP` runs at exit 0. Measured as 84 of 288 matrix cells, PRE ==
/// POST, PRE-EXISTING → ROADMAP §2 (needs the tree-wide AST self-width pass).
///
/// iverilog rejects every row below (`Cast base expression must be a vector type`,
/// or `<<(<) operator may not have REAL operands`), and it rejects a bare `4'(r)`
/// too, so this is a silent wrong answer becoming the oracle's own refusal.
#[test]
fn a_real_is_refused_at_every_operand_the_size_context_lowering_builds() {
    for expr in [
        // `lower_size_leaf` — these four discriminate against PRE (`0000`/`0001`/`1111`)
        "4'(r * 2)",
        "4'(r + 1)",
        "4'(r - 1)",
        "4'(-r)",
        // …at a width ABOVE the byte, so a guard narrowed to small N cannot pass
        "32'(r * 2)",
        "16'(r + 1)",
        // the `/`|`%` NARROWING branch, rhs (`lower_ctx_or_plain`). `8'(u % r)` is
        // the only prover of that site: `Mod` is the one operator missing from
        // `expr_is_real`'s `Binary` arm, so the cast arm cannot catch it instead.
        "8'(u / r)",
        "8'(u % r)",
        // …and its lhs, via a parent whose own arm never resizes the operand.
        "8'((r / 2) & 8'hFF)",
        "8'((r / 2) << 1)",
        // the shift AMOUNT. `<<`/`<<<`/`**` take the `Pow|Shl|AShl` site; `>>`/`>>>`
        // at w == n take the WIDENING site; the 32-bit `s` forces w > n, which is
        // the only row that reaches the narrowing branch's shift amount.
        "4'(u << r)",
        "4'(u <<< r)",
        "8'(u ** r)",
        "8'(u >> r)",
        "8'(u >>> r)",
        "8'(s >> r)",
        // `$signed`/`$unsigned` are transparent to the domain — in ANY argument
        // slot, since vita still accepts the (illegal) two-argument spelling.
        "4'($signed(r) * 2)",
        "4'($unsigned(r) + 1)",
        "8'(u / $signed(r))",
        // …in ANY argument slot. vita still accepts the illegal two-argument
        // spelling (pre-existing — iverilog: "takes exactly one(1) argument"), and
        // the IR node keeps BOTH args, so a predicate keyed on `args[0]` refuses
        // `$signed(r, u)` and lets `$signed(u, r)` through at exit 0. Both rows.
        "4'($signed(r, u) * 2)",
        "4'($signed(u, r) * 2)",
        // …and two rows that were ALREADY loud with this same message before the
        // slice (`Div`/`Pow` are in `expr_is_real`'s `Binary` arm). They pin the
        // pre-existing half; they discriminate nothing and kill no mutation.
        "8'(r / r2)",
        "4'(r ** 2)",
    ] {
        let (o, e, c) = run_status(&format!(
            "module t; real r, r2; logic [7:0] u; logic [31:0] s;\n\
             initial begin r=7.5; r2=2.0; u=8'd9; s=32'd9;\n\
             $display(\"%b\", {expr}); #1 $finish; end endmodule"
        ));
        assert!(
            matches!(c, Some(n) if n != 0),
            "{expr}: exit {c:?}, printed {o}"
        );
        // Pin the SIZE-CAST message, not just "real operand": the pre-existing
        // generic real diagnostics say that too, so the loose spelling let a
        // mutated message through.
        assert!(
            e.contains("size cast is not defined on a real operand"),
            "{expr}: got {e}"
        );
    }
    // …and the shapes that are LEGAL keep working: a comparison is one bit, an
    // explicit conversion is an integer, and a ternary CONDITION may legally be real
    // (it is tested for nonzero, never resized). `8'(a*b)` is the §4.5.212 carry case.
    let (o, e, c) = run_status(
        "module t; real r; logic [3:0] a, b; logic signed [7:0] s;\n\
         initial begin r=7.5; a=4'd15; b=4'd3; s=-8'sd8;\n\
         $display(\"%b %b %b %b %b %b\", 4'(r > 1.0), 4'($rtoi(r)), 4'(int'(r)),\n\
                  8'(a*b), 4'(r ? a : b), 8'(s >>> 1));\n\
         $finish; end endmodule",
    );
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(
        o.lines().next().unwrap().trim(),
        "0001 0111 1000 00101101 1111 11111100"
    );
    // The real ternary condition in BOTH directions, including the `-0.0`
    // discriminator that separates "tested for nonzero" from "tested for nonzero
    // BITS" — all three iverilog-pinned.
    assert_eq!(
        run("module t; real r; logic [3:0] a, b;\n\
             initial begin a=4'd15; b=4'd5;\n\
             r=-0.0; $display(\"%b\", 4'(r ? a : b));\n\
             r= 0.0; $display(\"%b\", 4'(r ? a : b));\n\
             r=1e-300; $display(\"%b\", 4'(r ? a : b)); #1 $finish; end endmodule"),
        "0101\n0101\n1111"
    );
}

/// The refusal is reported once per CAST, not once per real LEAF. iverilog reports
/// one error for `8'((r*2)+(r+1)+(r-1)+(-r))`; reporting four burns the
/// `MAX_ELAB_ERRORS` cap four times as fast, and with enough leaves in one cast an
/// unrelated LATER diagnostic is pushed out and lost (measured: 250 leaves + an
/// undeclared net → the `E3010` vanishes behind the cap). Every OTHER cast still
/// gets its own report — including one NESTED inside the reporting cast, which is
/// the direction the save/restore in `lower_size_ctx_entry` exists for. A sibling
/// pair cannot test that: the entry resets the flag on the way IN, so siblings are
/// immune whether or not the restore on the way OUT happens.
#[test]
fn the_real_size_cast_refusal_is_reported_once_per_cast() {
    let count = |src: &str| {
        let (_, e, _) = run_status(src);
        e.lines()
            .filter(|l| l.contains("size cast is not defined on a real operand"))
            .count()
    };
    assert_eq!(
        count(
            "module t; real r, r2; initial begin r=7.5; r2=2.0;\n\
             $display(\"%b\", 4'(r + r2 + r + r2 + r)); #1 $finish; end endmodule"
        ),
        1,
        "five real leaves in ONE cast must report once"
    );
    assert_eq!(
        count(
            "module t; real r; logic [3:0] a; initial begin r=7.5; a=4'd3;\n\
             $display(\"%b %b\", 4'(r * 2), 4'(a << r)); #1 $finish; end endmodule"
        ),
        2,
        "two separate casts each report"
    );
    // NESTED: the inner cast reports, then the OUTER one must regain its own. The
    // concat around the inner cast is load-bearing — `ast_ctx_signed` answers `None`
    // for a bare `Cast` in a sign-determining slot, so `8'(4'(r2*2) + r)` never
    // enters the size-context lowering at all. iverilog also reports both.
    assert_eq!(
        count(
            "module t; real r, r2; initial begin r=7.5; r2=3.5;\n\
             $display(\"%b\", 8'({4'(r2*2)} + r)); #1 $finish; end endmodule"
        ),
        2,
        "a nested cast must not consume the outer cast's report"
    );
}

/// §4.5.318: an unsized fill is a leaf whose VALUE depends on the width it is sized
/// to, so `lower_size_leaf`'s bare `lower_expr` built it at ONE bit and then extended
/// that — `4'(a+'1)` with `a=4'hD` printed `1110` (it added 1) where both oracles
/// print `1100` (it must add 15). The 40/64-bit rows are not decoration: below 32
/// bits `select_low` hides a wrong fill width, so a mutation that sizes the fill to a
/// constant 32 survives every narrower row. iverilog: `00 1100 00001100 12 12`.
#[test]
fn an_unsized_fill_leaf_is_sized_to_the_cast_width() {
    let o = run("module t; logic [3:0] a;\n\
        initial begin a = 4'hD;\n\
        $display(\"%b %b %b %0d %0d\", 2'(a+'1), 4'(a+'1), 8'(a+'1),\n\
                 64'(a+'1), 40'(a+'1)); #1 $finish; end endmodule");
    assert_eq!(o, "00 1100 00001100 12 12");
}

/// §4.5.318: §11.8.1 coerces EVERY operand in the cast region to the expression's
/// signedness, not just the outermost one. The §4.5.316 NARROW branch (`/ % >> >>>`)
/// lowered its operand self-determined and then stamped the sign on the RESULT, so a
/// signed sub-expression under an unsigned outer operator sign-extended and nothing
/// undid it: `4'(a + ((i13 | s4) >> 2))` printed 12 where iverilog and verilator both
/// print 0. Only the recursion reaches an inner operand. A 1,620-cell sweep over this
/// shape counted 207 fixed and 0 regressed.
#[test]
fn a_signed_subexpression_under_an_unsigned_cast_is_zero_extended() {
    let o = run(
        "module t; logic [3:0] a; logic signed [3:0] s4; integer i13;\n\
        initial begin a = 4'hD; s4 = -4'sd3; i13 = 13;\n\
        $display(\"%0d %0d %0d\", 4'(a + ((i13 | s4) >> 2)),\n\
                 4'(a + ((i13 | s4) / 3)), 4'(a & ((i13 | s4) % 5)));\n\
        #1 $finish; end endmodule",
    );
    assert_eq!(o, "0 1 1");
}
