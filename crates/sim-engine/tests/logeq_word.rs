//! LOGEQ-WORD (ROADMAP §5.2, 2026-06-23): `==`/`!=`/`===`/`!==` were evaluated
//! with a per-bit `get_vu(i)` loop while every sibling 4-state op is
//! word-parallel. These tests pin the IEEE-correct behavior across the cases a
//! naive word-parallel rewrite could break:
//! - a difference that lives ONLY in a high word (word-0-only bug → wrong eq),
//! - an x/z anywhere making `==`/`!=` return X but `===`/`!==` stay known,
//! - mixed-sign width unification (zero-extend unless BOTH signed).
//!
//! They must stay green before AND after the word-parallel rewrite.

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

fn run(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let (_res, out) = simulate_capture(&ir.expect("ir"), SimOpts::default());
    out
}

/// The distinguishing bit lives in word 3 (bit 192); word 0 is identical (0).
/// A word-0-only compare would wrongly report equal.
#[test]
fn logeq_difference_in_high_word() {
    let out = run(r#"
module t;
  reg [255:0] a, b;
  initial begin
    a = 256'h1_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000; // 2^192
    b = 256'd0;
    $display("eq=%b ne=%b ceq=%b cne=%b", a == b, a != b, a === b, a !== b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "eq=0 ne=1 ceq=0 cne=1");
}

/// Equal across multiple words (differs nowhere) — must report equal.
#[test]
fn logeq_equal_multiword() {
    let out = run(r#"
module t;
  reg [199:0] a, b;
  initial begin
    a = 200'hDEAD_BEEF_0000_0000_0000_0000_1234_5678_9ABC_DEF0_1111;
    b = 200'hDEAD_BEEF_0000_0000_0000_0000_1234_5678_9ABC_DEF0_1111;
    $display("eq=%b ne=%b ceq=%b cne=%b", a == b, a != b, a === b, a !== b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "eq=1 ne=0 ceq=1 cne=0");
}

/// An x in a high bit makes `==`/`!=` X, but `===`/`!==` compare exact (the x
/// position differs from the all-known RHS, so `===` is 0 and `!==` is 1).
#[test]
fn logeq_x_high_bit_poisons_eq_not_caseeq() {
    let out = run(r#"
module t;
  reg [99:0] a, b;
  initial begin
    a = 100'h0;
    a[80] = 1'bx;
    b = 100'h0;
    $display("eq=%b ne=%b ceq=%b cne=%b", a == b, a != b, a === b, a !== b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "eq=x ne=x ceq=0 cne=1");
}

/// `===` between two values whose only difference is an x vs a known 0 at a
/// high bit is exact-mismatch (0); identical x patterns match (1).
#[test]
fn caseeq_exact_x_pattern_matches() {
    let out = run(r#"
module t;
  reg [99:0] a, b;
  initial begin
    a = 100'h0; a[80] = 1'bx;
    b = 100'h0; b[80] = 1'bx;
    $display("ceq=%b cne=%b", a === b, a !== b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "ceq=1 cne=0");
}

/// Mixed-sign width unification: a lone signed operand must zero-extend (not
/// sign-extend) when the context is unsigned. `4'sb1111 == 8'hFF` → 0
/// (4'sb1111 zero-extends to 8'b0000_1111 = 0x0F ≠ 0xFF).
#[test]
fn logeq_mixed_sign_zero_extends() {
    let out = run(r#"
module t;
  reg signed [3:0] s;
  reg [7:0] u;
  initial begin
    s = 4'sb1111;
    u = 8'hFF;
    $display("eq=%b ceq=%b", s == u, s === u);
  end
endmodule
"#);
    assert_eq!(out.trim(), "eq=0 ceq=0");
}
