//! VM-ARITY-ASSERT (ROADMAP §5.3, 2026-06-23): try_compile trusts a
//! hand-written arity() table to size the native-eval fixed stacks. A debug-only
//! push-boundary assert in run() now verifies each op's ACTUAL stack movement
//! matches arity(), so a wrong entry (or a new NOp routed to the `_` catchall)
//! panics in debug instead of corrupting the stack. This test drives a broad set
//! of op kinds (narrow arith/div/shift/reduce/ternary + structural concat/
//! replicate/select + wide >64-bit arith) through native eval so the assert is
//! exercised across lanes; values are also pinned (the run must stay correct).

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

/// Narrow lane: arith chain, divide/mod, shifts, reduction, ternary, and the
/// structural trio (concat / replicate / part-select).
#[test]
fn native_narrow_op_zoo_runs_under_arity_assert() {
    let out = run(r#"
module t;
  reg [15:0] a, b;
  reg [7:0] p;
  initial begin
    a = 16'd1000; b = 16'd7; p = 8'hA5;
    $display("arith=%0d divmod=%0d:%0d", a + b*2 - 3, a / b, a % b);
    $display("shift=%0d:%0d red=%b:%b tern=%0d",
             a << 2, a >> 3, ^p, |p, (b[0] ? a : b));
    $display("concat=%h repl=%h sel=%h", {p, b[7:0]}, {4{p[1:0]}}, a[11:4]);
  end
endmodule
"#);
    assert_eq!(
        out,
        "arith=1011 divmod=142:6\n\
         shift=4000:125 red=0:1 tern=1000\n\
         concat=a507 repl=55 sel=3e\n"
    );
}

/// Wide lane (> 64 bits): arith + concat exercise the W* opcodes whose arity is
/// flag-dependent (WSelect/WConcatPair base_wide/out_wide).
#[test]
fn native_wide_op_zoo_runs_under_arity_assert() {
    let out = run(r#"
module t;
  reg [99:0] a, b;
  initial begin
    a = 100'h1_0000_0000_0000_0007;
    b = 100'd5;
    $display("wsum=%h wand=%h", a + b, a & b);
    $display("wcat=%h", {a[3:0], b[3:0]});
  end
endmodule
"#);
    // a = 2^64 + 7; a+5 = 2^64 + 12 (25-hex, '1' at nibble 16); a&5 = 5;
    // {a[3:0], b[3:0]} = {7, 5} = 0x75.
    assert_eq!(
        out,
        "wsum=000000001000000000000000c wand=0000000000000000000000005\n\
         wcat=75\n"
    );
}
