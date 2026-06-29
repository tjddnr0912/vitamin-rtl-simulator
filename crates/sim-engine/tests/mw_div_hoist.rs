//! MW-DIV-HOIST (ROADMAP §5.2, 2026-06-23): the multi-word restoring-division
//! loop in `mw_divmod` recomputed the loop-invariant `mw_neg(&bx)` and
//! reallocated `rem` (via `mw_add`) on every dividend bit. These end-to-end
//! tests pin the exact (quotient, remainder) results of `/` and `%` on the
//! restoring path (multi-word divisor, > 64 bits) and the short path
//! (one-word divisor), so the hoist + in-place subtraction stays bit-identical.

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

/// Restoring path: divisor `b = 2^64 + 5` is multi-word (word1 = 1), so the
/// short path is bypassed. `a = 7*b + 3` → q=7, r=3.
#[test]
fn mw_div_multiword_divisor_exact_small_quotient() {
    let out = run(r#"
module t;
  reg [127:0] a, b;
  initial begin
    a = 128'h7_0000_0000_0000_0026; // 7*2^64 + 38 = 7*(2^64+5) + 3
    b = 128'h1_0000_0000_0000_0005; // 2^64 + 5
    $display("q=%0d r=%0d", a / b, a % b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "q=7 r=3");
}

/// Restoring path with a non-trivial remainder and a two-word dividend whose
/// high word matters: `b = 0xF*2^64 + 7`, `a = 13*b + 256` → q=13, r=256.
#[test]
fn mw_div_multiword_divisor_nonzero_remainder() {
    let out = run(r#"
module t;
  reg [127:0] a, b;
  initial begin
    a = 128'hC3_0000_0000_0000_015B; // 195*2^64 + 347 = 13*(15*2^64+7) + 256
    b = 128'hF_0000_0000_0000_0007;  // 15*2^64 + 7
    $display("q=%0d r=%0d", a / b, a % b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "q=13 r=256");
}

/// Short path: a one-word divisor must not regress. 1000000 / 7 → q=142857 r=1.
#[test]
fn mw_div_one_word_divisor_short_path() {
    let out = run(r#"
module t;
  reg [127:0] a, b;
  initial begin
    a = 128'd1000000;
    b = 128'd7;
    $display("q=%0d r=%0d", a / b, a % b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "q=142857 r=1");
}
