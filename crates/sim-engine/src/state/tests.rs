use super::*;

/// P2-2: a VCD flush failure at finalize must surface as W-RUN-VCD-WRITE-FAIL
/// (was: `let _ =` silently swallowed it — truncated VCD, zero diagnostics).
#[test]
fn finalize_vcd_flush_error_warns() {
    struct FailWriter;
    impl Write for FailWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len()) // accept records...
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("disk full")) // ...but fail the flush
        }
    }
    #[derive(Default)]
    struct DiagSink(std::cell::RefCell<Vec<String>>);
    impl LogSink for DiagSink {
        fn emit(&self, e: LogEvent) {
            if let LogEvent::Diagnostic(d) = e {
                self.0
                    .borrow_mut()
                    .push(format!("{}: {}", d.code.code_num(), d.message));
            }
        }
    }

    let (toks, _) = hdl_lexer::lex("module t; endmodule");
    let (su, _) = hdl_parser::parse(&toks, "module t; endmodule");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("unit"), &sink).expect("elaborate");
    let mut st = SimState::new(
        &ir,
        Box::new(std::io::sink()),
        &sink,
        "1ns".to_string(),
        "test".to_string(),
        None,
    );
    st.open_vcd(Box::new(FailWriter));
    st.finalize_vcd();
    let diags = sink.0.borrow();
    assert!(
        diags.iter().any(|d| d.starts_with("VITA-W4019")),
        "flush failure must emit W-RUN-VCD-WRITE-FAIL; got {diags:?}"
    );
}
