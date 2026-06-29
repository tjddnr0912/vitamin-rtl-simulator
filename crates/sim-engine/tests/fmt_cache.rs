//! FMT-CACHE (ROADMAP §5.2, 2026-06-23): `$display`/`$write` re-decoded the
//! packed format-string const (bit-unpack + UTF-8 validation) on every call.
//! A per-ConstId lazy cache decodes each format/string const once. This test
//! pins byte-identical output across repeated calls of TWO distinct format
//! strings interleaved in loops (a cid-keyed cache bug would cross-contaminate)
//! plus the trailing string-literal arg path (str_const_of_expr).

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

#[test]
fn repeated_format_strings_are_byte_identical() {
    let out = run(r#"
module t;
  integer i;
  initial begin
    for (i = 0; i < 3; i = i + 1)
      $display("A i=%0d hex=%h", i, i*16);
    for (i = 0; i < 2; i = i + 1)
      $display("B val=%0d", i + 100);
    $display("x=%0d ", 5, "tail");
  end
endmodule
"#);
    assert_eq!(
        out,
        "A i=0 hex=00000000\n\
         A i=1 hex=00000010\n\
         A i=2 hex=00000020\n\
         B val=100\n\
         B val=101\n\
         x=5 tail\n"
    );
}
