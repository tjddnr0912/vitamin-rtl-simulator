//! §2 「다음 착수 순서」 #1 — the constant domain's `**`.
//!
//! Two things were wrong with one operator, and they share a root: the i64 the
//! const domain carries an exponent in is a CONTAINER, not the value's type.
//!
//! `64'd0 - 64'd8` is an UNSIGNED subtraction — 18446744073709551608, not −8 —
//! but the fold read the i64 CONTAINER's sign bit, decided the exponent was
//! negative, applied the IEEE negative-exponent table and answered 0 at exit 0.
//! Both oracles, and vita's own runtime, answer 926288481. The exponent's
//! signedness travels with its value now, so the table applies only when the
//! exponent really is negative and the huge-magnitude case is honestly loud.
//!
//! ⚠️ SCOPE — folding the huge case MODULARLY was tried and reverted. Square-and-
//! multiply mod 2^64 gives the oracles' answer at every context of 64 bits or
//! fewer, but the module-scope fold has no context width: a
//! `localparam [127:0] P = 3 ** 41` then zero-extends an already-truncated
//! 64-bit value, turning a loud reject into a silent wrong (and at 96 bits, one
//! silent wrong into a different one). Both adversarial lenses found it
//! independently. So `**` keeps the i64 domain's decline, exactly like the
//! overflowing `+`/`*` it sits beside — one class, waiting on a width-aware
//! module-scope fold, tracked in ROADMAP §2.
//!
//! ⚠️ The UNTYPED `localparam` spelling of these designs has no iverilog answer
//! at all: it HANGS, computing 3^(2^64−8) in arbitrary precision.
//!
//! Oracle: iverilog 13.0 and verilator 5.050 agree on every value here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_cped_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn folds_as(decl: &str, expr: &str, want: i64) {
    let src = format!(
        "module top;\n\
         parameter integer W = 8;\n\
         localparam {decl} L = {expr};\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n"
    );
    let (out, code, err) = run_raw(&src);
    assert_eq!(code, Some(0), "`{expr}` should fold, stderr:\n{err}");
    assert!(
        out.contains(&format!("R={want}")),
        "`{expr}` want R={want}; got:\n{out}"
    );
}

fn folds(expr: &str, want: i64) {
    folds_as("integer", expr, want);
}

fn loud_as(decl: &str, expr: &str) {
    let src = format!(
        "module top;\n\
         parameter integer W = 8;\n\
         localparam {decl} L = {expr};\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n"
    );
    let (_, code, err) = run_raw(&src);
    assert_eq!(code, Some(1), "`{expr}` should be loud:\n{err}");
    assert!(
        err.contains("is not a foldable constant expression")
            || err.contains("value is not a constant:"),
        "`{expr}` unexpected diagnostic:\n{err}"
    );
}

fn loud(expr: &str) {
    loud_as("integer", expr);
}

/// The exponent's SIGNEDNESS comes from the expression, not from the sign bit of
/// the i64 that carries it. An unsigned subtraction that "goes negative" is a
/// huge positive exponent, and the answer is the modular one.
#[test]
fn an_unsigned_exponent_is_not_a_negative_one() {
    // A huge unsigned exponent is outside the i64 domain, so it is LOUD — not the
    // 0 the negative-exponent table used to hand back at exit 0.
    loud("3 ** (64'd0 - 64'd8)");
    loud("4'sd3 ** (64'd0 - 64'd8)");
    loud_as("longint", "3 ** (64'd0 - 64'd8)");
    // A genuinely SIGNED negative exponent still takes the IEEE table.
    folds("3 ** (-8)", 0);
    folds("3 ** (-8'sd8)", 0);
    // A base of 0 or ±1 depends on the exponent's PARITY, never its magnitude, so
    // it answers for any exponent — including the ones the domain cannot carry.
    // Gating these behind the domain check turned both oracles' 1 into a decline.
    folds("1 ** (64'd0 - 64'd8)", 1);
    folds("(-1) ** (64'd0 - 64'd8)", 1);
    folds("(-1) ** (64'd0 - 64'd7)", -1);
    folds("0 ** (8'd0 - 8'd8)", 0);
    folds("1 ** (8'd0 - 8'd8)", 1);
    folds("(-1) ** (8'd0 - 8'd8)", 1);
    folds("0 ** 0", 1);
    folds("1 ** (-8)", 1);
    folds("(-1) ** (-7)", -1);
}

/// A result the i64 domain cannot hold stays LOUD — the discipline `+` and `*`
/// keep in this domain, and the reason `**` may not wrap here. The cells below
/// are all values both oracles print; the decline is honest, not correct.
///
/// ⚠️ Do not "fix" these by folding modularly without a context width. That was
/// measured: mod 2^64 is right for a ≤64-bit context and silently wrong the
/// moment the target is wider, because the coercion zero-extends what the
/// wrapping already discarded.
#[test]
fn a_result_outside_the_i64_domain_stays_loud() {
    loud("3 ** 40");
    loud("3 ** 64");
    loud("7 ** 30");
    loud_as("longint", "3 ** 40");
    loud_as("bit [95:0]", "3 ** 45");
    loud_as("bit [127:0]", "3 ** 41");
    loud_as("bit [95:0]", "3 ** (96'd0 - 96'd8)");
    // Where the exact value fits, nothing changed.
    folds("2 ** 40", 0);
    folds("3 ** 20", -808182895);
    folds("3 ** 3", 27);
    folds("3 ** W", 6561);
    folds("(-3) ** 3", -27);
    // A narrow exponent still wraps at its OWN width first (§4.5.339).
    folds("3 ** (4'd15 + 4'd1)", 1);
}

/// The constant-function interpreter is the second evaluator of the same text.
/// It must agree with the module scope — both loud here, where the runtime (the
/// third) has the value. Loud-versus-correct is a capability gap; loud-versus-0
/// was the defect.
#[test]
fn the_const_function_body_agrees_with_module_scope() {
    let (_, code, err) = run_raw(
        "module top;\n\
         function integer g();\n\
         g = 3 ** (64'd0 - 64'd8);\n\
         endfunction\n\
         localparam integer L = g();\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(1), "must be loud, not a silent 0:\n{err}");
    assert!(
        err.contains("is not a foldable constant expression")
            || err.contains("value is not a constant:"),
        "{err}"
    );
    // The RUNTIME does carry it — that is where the oracle value lives.
    let (out, code, err) = run_raw(
        "module top;\n\
         integer r;\n\
         initial begin r = 3 ** (64'd0 - 64'd8); $display(\"R=%0d\", r); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("R=926288481"), "got:\n{out}");
}

/// Consumers of a `**` are unchanged where the value already fitted.
#[test]
fn consumers_of_a_power_are_unchanged() {
    for (src, want) in [
        (
            "module top;\n  logic [(3 ** 3) : 0] v;\n\
             initial begin v = '1; $display(\"R=%0d\", $bits(v)); #1 $finish; end\nendmodule\n",
            28,
        ),
        (
            "module top;\n  logic [31:0] r;\n\
             initial begin r = {(2 ** 3){2'b01}}; $display(\"R=%0d\", r); #1 $finish; end\nendmodule\n",
            21845,
        ),
    ] {
        let (out, code, err) = run_raw(src);
        assert_eq!(code, Some(0), "consumer, stderr:\n{err}");
        assert!(out.contains(&format!("R={want}")), "want {want}; got:\n{out}");
    }
}

/// A pre-existing PANIC, found by this slice's own sweep: the parser folds every
/// binary expression through its generate-array-index helper, and that helper
/// used unchecked arithmetic — so an ordinary overflowing `localparam` took the
/// whole run down with "attempt to multiply with overflow". It declines now, so
/// the expression is merely not-a-constant-index and the elaborator reports the
/// overflow in its own voice. (Both oracles fold this to 145474192; that the
/// const domain still declines is the separate domain-wide class in §2.)
#[test]
fn an_overflowing_literal_product_is_loud_not_a_panic() {
    let (out, code, err) = run_raw(
        "module top;\n\
         localparam integer L = 3037000500 * 3037000500;\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    // ⭐ The property this cell exists for is "not a PANIC"; it used to be spelled as
    // "loud", because an i64 product of two 33-bit decimals overflows and the fold
    // declined. The wide lane multiplies inside the context width, which is what the
    // language says happens, so the answer is now a value — and it is the value both
    // oracles print (145474192). The overflow is still not a panic; that is what the
    // second assertion pins.
    assert_eq!(code, Some(0), "must not be a panic:\n{err}");
    assert!(!err.contains("panicked"), "panicked:\n{err}");
    assert!(
        out.contains("R=145474192"),
        "both oracles say 145474192:\n{out}"
    );
    // The generate-array index this helper exists for still folds.
    let (out, code, err) = run_raw(
        "module top;\n  genvar g;\n  int n = 0;\n\
         generate for (g = 0; g < 3; g = g + 1) begin : lp\n\
         initial n = n + 1;\n\
         end endgenerate\n\
         initial begin #1 $display(\"R=%0d\", n); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "generate index, stderr:\n{err}");
    assert!(out.contains("R=3"), "got:\n{out}");
}
