//! The >64-bit constant domain gets `/`, `%`, `**`, `<<<`, `$clog2`, `$bits` and
//! `$isunknown` — the four operators (plus the two size queries) that had no fold
//! arm while the RUNTIME lane computed every one of them correctly.
//!
//! ⭐ The reason they were missing is written into `const_wide.rs` itself: a wide
//! divide "would be a second spelling of the engine's arithmetic, and a subtly wrong
//! one produces a silent wrong PARAMETER". That was a statement about a RISK, and the
//! risk is avoidable rather than payable — `mw_divmod` / `mw_pow` moved DOWN from
//! `sim-engine::eval` into `sim_ir::mw`, so the constant fold and `EvalCtx::arith`
//! call the SAME function. There is no second spelling to be subtly wrong.
//!
//! ⚠️ §11.6.1 makes `**` the exception to the arm above it: the EXPONENT is
//! self-determined while the base takes the context, so the shared "bring both to a
//! common width" prep is wrong for it. `8'd200 ** 2` is `40` (8 bits) in both
//! oracles, not `40000` — pinned below.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 AND an
//! independent python3 golden (integer arithmetic masked to 128 bits). A 45-cell
//! sweep of these shapes was 3-way identical before any of it was pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wdmp_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// The two 128-bit constants every cell below is built from; `A / B` = 14 and
/// `A % B` = `0e2d…2f`, both checked against python3.
const A: &str = "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n";
const B: &str = "  localparam logic [127:0] B = 128'h0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f;\n";

fn m(decls: &str, fmt: &str, expr: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule tb;\n{decls}\n  \
         initial begin #1 $display(\"R={fmt}\", {expr}); $finish; end\nendmodule\n"
    )
}

fn folds(decls: &str, fmt: &str, expr: &str, want: &str) {
    let (out, code) = run(&m(decls, fmt, expr));
    assert_eq!(code, Some(0), "should fold:\n{out}");
    assert!(out.contains(&format!("R={want}")), "want R={want}:\n{out}");
}

fn loud(decls: &str, fmt: &str, expr: &str) {
    let (out, code) = run(&m(decls, fmt, expr));
    assert_ne!(code, Some(0), "must stay loud:\n{out}");
}

/// ⭐ The reporter's own four cells, at the values they published.
#[test]
fn the_four_missing_arms_fold_at_128_bits() {
    let d = format!(
        "{A}{B}  localparam logic [127:0] Q = A / B;\n  \
         localparam logic [127:0] R = A % B;\n  \
         localparam logic [127:0] P = A ** 2;\n  \
         localparam int           C = $clog2(A);"
    );
    folds(&d, "%h", "Q", "0000000000000000000000000000000e");
    folds(&d, "%h", "R", "0e2d2d2d2d2d2d2d2d2d2d2d2d2d2d2f");
    folds(&d, "%h", "P", "c2000000000000000000000000000001");
    folds(&d, "%0d", "C", "128");
}

/// `(A/B)*B + (A%B) == A` — the division identity, computed entirely at elaborate
/// time. A quotient that is right and a remainder that is not (or vice versa) cannot
/// satisfy it, so this one cell covers both kernels against each other.
#[test]
fn quotient_and_remainder_reconstruct_the_dividend() {
    let d = format!("{A}{B}  localparam logic [127:0] R = (A / B) * B + (A % B);");
    folds(&d, "%h", "R", "e1000000000000000000000000000001");
}

/// §11.4.3 SIGN rules, mirrored from `EvalCtx::arith`: the quotient's sign is the
/// operands' XOR, the remainder takes the DIVIDEND's, and one UNSIGNED operand makes
/// the whole operation unsigned no matter how the other is declared.
#[test]
fn signed_division_follows_the_dividends_sign() {
    let d = "  localparam logic signed [127:0] SA = -128'sd17;\n  \
             localparam logic signed [127:0] SB = 128'sd5;\n  \
             localparam logic [127:0] UB = 128'd5;\n";
    folds(
        &format!("{d}  localparam logic signed [127:0] R = SA / SB;"),
        "%0d",
        "R",
        "-3",
    );
    folds(
        &format!("{d}  localparam logic signed [127:0] R = SA % SB;"),
        "%0d",
        "R",
        "-2",
    );
    folds(
        &format!("{d}  localparam logic signed [127:0] R = SA / -SB;"),
        "%0d",
        "R",
        "3",
    );
    // ⚠️ the remainder does NOT follow the divisor: −17 % −5 is −2, not +2.
    folds(
        &format!("{d}  localparam logic signed [127:0] R = SA % -SB;"),
        "%0d",
        "R",
        "-2",
    );
    // §11.4.3: one unsigned operand ⇒ the whole operation unsigned, so the −17 is
    // read as the 128-bit pattern 2¹²⁸−17 and the quotient is `3333…332f`.
    folds(
        &format!("{d}  localparam logic [127:0] R = SA / UB;"),
        "%h",
        "R",
        "3333333333333333333333333333332f",
    );
}

/// ⚠️⚠️ §11.6.1 Table 11-21: `**`'s exponent is SELF-determined and its base takes
/// the context, so the result's width is the BASE's. Folding both to a common width
/// would size these two cells at 32 bits and answer `40000` and `2**100` truncated to
/// 32 — both oracles say `40` and the full 128-bit `2**100`.
#[test]
fn the_power_exponent_is_self_determined() {
    folds("  localparam logic [7:0] R = 8'd200 ** 2;", "%h", "R", "40");
    folds(
        "  localparam logic [127:0] R = 128'd2 ** 100;",
        "%h",
        "R",
        "00000010000000000000000000000000",
    );
    // 2**128 wraps to 0 inside the base's width — the carry has nowhere to go.
    folds(
        "  localparam logic [127:0] R = 128'd2 ** 128;",
        "%h",
        "R",
        "00000000000000000000000000000000",
    );
    // exponent 0 is 1 even for a zero base (§11.4.10 Table 11-6).
    folds(
        "  localparam logic [127:0] R = 128'd0 ** 0;",
        "%h",
        "R",
        "00000000000000000000000000000001",
    );
}

/// §11.4.10 Table 11-6, the NEGATIVE-exponent rows: 1 → 1, −1 → ±1 by exponent
/// parity, |base| > 1 → 0.
#[test]
fn a_negative_exponent_uses_the_ieee_table() {
    let s = "  localparam logic signed [127:0] R = ";
    folds(&format!("{s}128'sd1 ** -128'sd3;"), "%0d", "R", "1");
    folds(&format!("{s}(-128'sd1) ** -128'sd3;"), "%0d", "R", "-1");
    folds(&format!("{s}(-128'sd1) ** -128'sd4;"), "%0d", "R", "1");
    folds(&format!("{s}128'sd9 ** -128'sd2;"), "%0d", "R", "0");
    // a negative BASE with a positive exponent is ordinary two's-complement power
    folds(&format!("{s}(-128'sd3) ** 3;"), "%0d", "R", "-27");
    folds(&format!("{s}(-128'sd3) ** 4;"), "%0d", "R", "81");
    // ⚠️ `0 ** negative` is X in IEEE and in both oracles, so it has no constant
    // value and this domain declines rather than installing one.
    loud(
        "  localparam logic signed [127:0] R = 128'sd0 ** -128'sd1;",
        "%0d",
        "R",
    );
}

/// ⭐ `<<<` at a fixed width IS `<<` (§11.4.10 gives an arithmetic LEFT shift the
/// same zero fill), but it fell to the catch-all — so which of two spellings of one
/// operation you wrote decided whether the parameter existed.
#[test]
fn arithmetic_left_shift_folds_like_the_logical_one() {
    let d = format!("{A}  localparam logic [127:0] R = A <<< 4;");
    folds(&d, "%h", "R", "10000000000000000000000000000010");
    // the `<<` twin, unchanged, at the same value
    let e = format!("{A}  localparam logic [127:0] R = A << 4;");
    folds(&e, "%h", "R", "10000000000000000000000000000010");
}

/// ⭐ The item the reporter called most valuable: `localparam int AW = $clog2(MAX);`
/// is the standard width idiom, and it was E3009 for any `MAX` whose MAGNITUDE
/// passes 64 bits — `selfdet_bits_unsigned` reads the folded bits back as a `u64`
/// and there was nothing after it. The ceiling is a bit INDEX, so the bit domain can
/// answer it without ever forming the value.
#[test]
fn clog2_of_a_wide_constant_is_the_width_idiom() {
    let d = format!("{A}{B}");
    folds(
        &format!("{d}  localparam int R = $clog2(A);"),
        "%0d",
        "R",
        "128",
    );
    folds(
        &format!("{d}  localparam int R = $clog2(B);"),
        "%0d",
        "R",
        "124",
    );
    // an exact power of two costs no extra bit; one more set bit does
    folds(
        "  localparam int R = $clog2(128'h8000_0000_0000_0000_0000_0000_0000_0000);",
        "%0d",
        "R",
        "127",
    );
    folds(
        "  localparam int R = $clog2(128'h8000_0000_0000_0000_0000_0000_0000_0001);",
        "%0d",
        "R",
        "128",
    );
    folds("  localparam int R = $clog2(128'd1);", "%0d", "R", "0");
    folds("  localparam int R = $clog2(128'd0);", "%0d", "R", "0");
    folds(
        "  localparam int R = $clog2({128{1'b1}});",
        "%0d",
        "R",
        "128",
    );
    // over an expression, not just a name
    folds(
        &format!("{d}  localparam int R = $clog2(A / B);"),
        "%0d",
        "R",
        "4",
    );
}

/// The same `$clog2` in the positions that are NOT a parameter value: a declaration
/// bound, a generate condition, and the runtime. All four agree, which is the point —
/// one source line used to have two answers.
#[test]
fn clog2_of_a_wide_constant_sizes_a_declaration() {
    let src = format!(
        "`timescale 1ns/1ns\nmodule tb;\n{A}  \
         logic [$clog2(A)-1:0] bus;\n  \
         generate if ($clog2(A) == 128) begin : g\n    \
         initial $display(\"GEN=1\");\n  end endgenerate\n  \
         initial begin #1 $display(\"W=%0d RT=%0d\", $bits(bus), $clog2(A)); $finish; end\n\
         endmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "should elaborate:\n{out}");
    assert!(out.contains("GEN=1"), "generate condition:\n{out}");
    assert!(out.contains("W=128 RT=128"), "bound and runtime:\n{out}");
}

/// §20.6.2 `$bits` and §20.9 `$isunknown` over a >64-bit operand. vita's RUNTIME
/// `$bits` already answered 128 for `$bits(A)` while the constant fold was E3009, so
/// one source line had two answers here too.
#[test]
fn bits_and_isunknown_answer_over_a_wide_operand() {
    let d = format!("{A}{B}");
    folds(
        &format!("{d}  localparam int R = $bits(A);"),
        "%0d",
        "R",
        "128",
    );
    folds(
        &format!("{d}  localparam int R = $bits(A[63:0]);"),
        "%0d",
        "R",
        "64",
    );
    folds(
        &format!("{d}  localparam int R = $bits({{A,B}});"),
        "%0d",
        "R",
        "256",
    );
    // a comparison is ONE bit however wide its operands are (§11.6.1)
    folds(
        &format!("{d}  localparam int R = $bits(A > B);"),
        "%0d",
        "R",
        "1",
    );
    folds(
        &format!("{d}  localparam int R = $bits(A / B);"),
        "%0d",
        "R",
        "128",
    );
    folds(
        &format!("{d}  localparam int R = $isunknown(A);"),
        "%0d",
        "R",
        "0",
    );
    folds(
        "  localparam int R = $isunknown({124'h0, 4'hx});",
        "%0d",
        "R",
        "1",
    );
    // …and `$bits` as a declaration bound, the shape §4.5.380 N32-1 fixed for
    // narrow operands and could not answer for wide ones.
    let src = format!(
        "`timescale 1ns/1ns\nmodule tb;\n{A}  logic [$bits(A)-1:0] bus;\n  \
         initial begin #1 $display(\"W=%0d\", $bits(bus)); $finish; end\nendmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "should elaborate:\n{out}");
    assert!(out.contains("W=128"), "bound:\n{out}");
}

/// ⚠️ `x / 0` is X in IEEE §11.4.3, X in iverilog and X in vita's own runtime;
/// verilator's `0` is a 2-state artifact, so the oracles split and this 2-state
/// domain installs NEITHER answer. It declines, and the caller stays loud.
///
/// The diagnostic is pinned too, because the one it replaced was the item's second
/// half: it said "`A` is wider than 64 bits", blaming the WIDTH of a name this
/// elaborator now reads without difficulty, and never mentioned the zero divisor.
#[test]
fn division_by_zero_stays_loud_and_says_why() {
    let d = format!(
        "{A}  localparam logic [127:0] Z = 128'd0;\n  \
                     localparam logic [127:0] R = A / Z;"
    );
    let (out, code) = run(&m(&d, "%h", "R"));
    assert_ne!(code, Some(0), "must stay loud:\n{out}");
    assert!(
        out.contains("divides by zero"),
        "the diagnostic must name the divisor, not `A`'s width:\n{out}"
    );
    assert!(
        !out.contains("wider than 64 bits"),
        "`A`'s width is not the reason:\n{out}"
    );
}

/// An x/z bit still declines every VALUE-reading arm, and it is still the sharpest
/// thing to name — the wide-domain child skip must not promote the operator above it
/// into the message when the operand itself is the culprit.
#[test]
fn an_unknown_operand_stays_loud_and_is_named() {
    let (out, code) = run(&m(
        "  localparam logic [127:0] R = 128'hx / 128'd3;",
        "%h",
        "R",
    ));
    assert_ne!(code, Some(0), "must stay loud:\n{out}");
    assert!(out.contains("128'hx"), "name the unknown operand:\n{out}");
}

/// ⚠️ A BARE >64-bit name in a position that wants an integral value still fails for
/// its width, and must still say so. The child skip above is scoped to COMPOUND
/// expressions precisely so this message survives.
///
/// ⚠️⚠️ The position matters, and the first draft of this test picked the wrong one.
/// `localparam int R = A;` is NOT such a position — it is an assignment, and §6.20.2
/// truncates: iverilog prints `R=1` (the low bits of `…0001`) and so does vita, before
/// and after this slice. A WIDTH BOUND is a position that genuinely needs an integral
/// value, and that is what this pins.
#[test]
fn a_bare_wide_name_still_blames_its_width() {
    let src = format!(
        "`timescale 1ns/1ns\nmodule tb;\n{A}  logic [A-1:0] v;\n  \
         initial begin #1 $display(\"W=%0d\", $bits(v)); $finish; end\nendmodule\n"
    );
    let (out, code) = run(&src);
    assert_ne!(code, Some(0), "must stay loud:\n{out}");
    assert!(
        out.contains("wider than 64 bits"),
        "a bare name really does fail for its width:\n{out}"
    );

    // The control that makes the sentence above true rather than vacuous: the same
    // bare name in an ASSIGNMENT position truncates and runs, exactly as iverilog does.
    let src = format!(
        "`timescale 1ns/1ns\nmodule tb;\n{A}  localparam int R = A;\n  \
         initial begin #1 $display(\"R=%0d\", R); $finish; end\nendmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=1"), "iverilog prints R=1:\n{out}");
}

/// ⚠️ The elaborate-time work budget. Restoring division is O(w·n) word ops, so a
/// 2²⁰-bit `localparam` divide would run for tens of seconds with no `$finish` to
/// interrupt it. 65536 bits is admitted (52 ms measured, release); 131072 declines
/// and stays LOUD rather than hanging — a rung up from a silent X, which is what the
/// runtime lane's `WIDE_ARITH_CAP` produces for the same shape.
#[test]
fn the_super_linear_kernels_are_budgeted() {
    let ok = "  localparam logic [65535:0] W = {8192{8'ha7}};\n  \
              localparam logic [65535:0] D = {4096{16'h1357}};\n  \
              localparam logic [65535:0] R = W / D;";
    let (out, code) = run(&m(ok, "%0d", "$bits(R)"));
    assert_eq!(code, Some(0), "65536 bits is inside the budget:\n{out}");
    assert!(out.contains("R=65536"), "{out}");

    let too_big = "  localparam logic [131071:0] W = {16384{8'ha7}};\n  \
                   localparam logic [131071:0] D = {8192{16'h1357}};\n  \
                   localparam logic [131071:0] R = W / D;";
    let (out, code) = run(&m(too_big, "%0d", "$bits(R)"));
    assert_ne!(
        code,
        Some(0),
        "over the budget must be LOUD, not slow:\n{out}"
    );
}
