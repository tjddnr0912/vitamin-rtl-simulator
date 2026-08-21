//! A `real` destination lends NO bit width to its right-hand side (§4.5.358).
//!
//! IEEE §6.12: a real has no bit width, so the assignment rule `width = max(lhs,
//! self(rhs))` has nothing to take from the left — the right-hand side is
//! SELF-DETERMINED and its value is then converted (§11.8.1 / IEEE 1364 §4.3). Asking
//! `lvalue_width` anyway answers the real's STORAGE size of 64, and evaluating there
//! changes the value whenever the operand is UNSIGNED and narrower:
//!
//! | design                                   | PRE           | both oracles |
//! |------------------------------------------|---------------|--------------|
//! | `byte unsigned b = 8; real r; r = -b;`   | `1.84467e+19` | `248`        |
//! | `shortint unsigned s;      r = -s;`      | `1.84467e+19` | `65528`      |
//! | `int unsigned i;           r = -i;`      | `1.84467e+19` | `4.29497e+09`|
//!
//! ⭐ THIRD OCCURRENCE OF ONE TRAP, and the first on the engine side. §4.5.353 caught
//! `ir_lvalue_width` answering 64 for a real assignment target; §4.5.354 caught
//! `ir_bits_of` answering 64 for a real operand. Both were in elaborate. This is the
//! engine asking the same question, in THREE places — the interpreter/VM scheduler, the
//! native kernel, and the native-program compiler — which is why the fix is one named
//! predicate (`width::lvalue_targets_real`) rather than three inline checks.
//!
//! ⚠️⚠️ AND THE THIRD SITE IS WHY THE `#1` VARIANTS BELOW EXIST. With the first two
//! fixed, `real r; r = -b;` was correct in a process containing a delay and wrong in
//! one without: a delay-free process compiles to a native program, which computed its
//! own context width. A test that only ever wrote `#1 $display(...)` — the habit
//! everywhere else in this suite — would have passed against a half-fixed engine.
//!
//! The expression alone was never wrong: `$display("%0d", -b)` printed 248 before this
//! slice too, which is what located the bug at the assignment boundary rather than in
//! the operator.
//!
//! ORACLES: iverilog 13.0 and verilator 5.050 agree on every value here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `body` and return the `r=` line. `delay` picks the execution path: a process
/// with a delay goes through the scheduler, one without compiles to a native program.
fn run(decl: &str, expr: &str, delay: bool) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = if delay { "#1 " } else { "" };
    let src = format!(
        "module t;\n  {decl} real q = 2.0; real r;\n\
         initial begin r = {expr}; {d}$display(\"r=%g\", r); $finish; end\nendmodule\n"
    );
    let p = std::env::temp_dir().join(format!("vita_rtlnw_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, &src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "expected success for `{expr}`:\n{all}"
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("r=").map(str::to_string))
        .unwrap_or_else(|| panic!("no r= line for `{expr}`:\n{all}"))
}

/// Both execution paths must agree, and agree with the oracles.
fn both(decl: &str, expr: &str, want: &str) {
    for delay in [true, false] {
        assert_eq!(
            run(decl, expr, delay),
            want,
            "for `{expr}` with delay={delay}"
        );
    }
}

#[test]
fn negating_an_unsigned_operand_into_a_real_keeps_the_operand_width() {
    both("byte unsigned b = 8;", "-b", "248");
    both("shortint unsigned s = 8;", "-s", "65528");
    both("int unsigned i = 8;", "-i", "4.29497e+09");
    both("bit [7:0] v = 8;", "-v", "248");
    both("bit [11:0] v = 8;", "-v", "4088");
    both("logic [31:0] v = 8;", "-v", "4.29497e+09");
    // Parenthesised is the spelling the ROADMAP entry recorded; it must not differ.
    both("byte unsigned b = 8;", "(-b)", "248");
}

#[test]
fn a_sixty_four_bit_operand_is_unchanged() {
    // The trap only bites when the operand is NARROWER than the real's storage size,
    // so these are the control: they were right before and must stay right.
    both("longint unsigned v = 8;", "-v", "1.84467e+19");
    both("bit [63:0] v = 8;", "-v", "1.84467e+19");
}

#[test]
fn a_signed_operand_is_unchanged() {
    // Sign-extension preserves the value, so widening to 64 was harmless here — which
    // is exactly why the bug hid: every signed spelling was already right.
    both("int i = 8;", "-i", "-8");
    both("shortint s = 8;", "-s", "-8");
    both("byte b = 8;", "-b", "-8");
    both("logic signed [11:0] v = 8;", "-v", "-8");
}

#[test]
fn a_real_sibling_already_supplied_the_width() {
    // With a real operand in the expression the conversion happened at the integral
    // operand's own width all along — the assignment boundary was the only broken one.
    both("byte unsigned b = 8;", "q + (-b)", "250");
    both("byte unsigned b = 8;", "(-b) * 1.0", "248");
    both("byte unsigned b = 8;", "(-b) / 2.0", "124");
}

#[test]
fn other_unsigned_expressions_at_a_real_target() {
    // Not just unary minus: any expression whose value depends on the evaluation width.
    both("byte unsigned b = 8;", "~b", "247");
    both("bit [11:0] v = 8;", "~v", "4087");
    // ⚠️ These two are NOT 164 and 3996, which is what hand-arithmetic says and what an
    // earlier draft of this test asserted. The literal `100` is an unsized decimal, so
    // it is 32 bits wide and the expression's own self-determined width is 32 — the
    // operand's 8 or 12 bits never decide anything. Unsigned, `8 - 100` at 32 bits is
    // 2**32 - 92. Both oracles say so, and vita already agreed before this slice: the
    // value here does not depend on the assignment context at all, which is what makes
    // it a useful control rather than a second copy of the test above.
    both("byte unsigned b = 8;", "b - 100", "4.29497e+09");
    both("bit [11:0] v = 8;", "v - 100", "4.29497e+09");
    // The narrow-literal spelling IS context-free in the other direction and shows the
    // operand width surviving: `8'd100` keeps the arithmetic at eight bits.
    both("byte unsigned b = 8;", "b - 8'd100", "164");
}
