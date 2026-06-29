//! P1-4: in-body event controls (REMAINING_WORK 2026-06-10).
//!
//! Before this fix an in-body `@(*)` lowered to `Level { nets: [] }` — a wait
//! that could NEVER wake — and a multi-term in-body edge wait silently kept
//! only its FIRST edge term. Now: `@(*)` infers the read-set of the statement
//! it controls (the same machinery as block-header `@*`), and a multi-edge
//! in-body wait is a LOUD `E-ELAB-UNSUPPORTED` (the frozen `WaitCause::Edge`
//! carries one term; silent first-term-only changed wake semantics).

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, FinishReason, SimOpts};

#[derive(Default)]
struct Sink {
    out: RefCell<String>,
    errs: RefCell<Vec<String>>,
}
impl LogSink for Sink {
    fn emit(&self, e: LogEvent) {
        match e {
            LogEvent::RtlOutput(t) => self.out.borrow_mut().push_str(&t.text),
            LogEvent::Diagnostic(d) => self.errs.borrow_mut().push(format!(
                "{}[{}]: {}",
                d.severity.token(),
                d.code.code_num(),
                d.message
            )),
            _ => {}
        }
    }
}

fn run(src: &str) -> (Option<FinishReason>, String, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = Sink::default();
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su.expect("source unit"), &sink, &BTreeMap::new(), -9);
    let reason = ir.map(|ir| {
        let opts = SimOpts {
            fork_modes: sc.fork_modes,
            net_names: sc.net_names,
            proc_multipliers: sc.proc_multipliers,
            severities: sc.severities,
            radixes: sc.radixes,
            ..SimOpts::default()
        };
        simulate(&ir, &sink, opts).finish_reason
    });
    (reason, sink.out.into_inner(), sink.errs.into_inner())
}

/// In-body `@(*)` wakes on a change of any net READ by the controlled
/// statement (here {a, b}) — it used to wait forever.
#[test]
fn inbody_star_wakes_on_read_set_change() {
    let (reason, out, _d) = run(r#"
module t;
  reg a, b, y;
  initial begin
    a = 0; b = 0;
    #1 a = 1;
    #2 $finish;
  end
  initial begin
    @(*) y = a ^ b;
    $display("y=%b at %0t", y, $time);
  end
endmodule
"#);
    assert_eq!(reason, Some(FinishReason::Finish));
    assert!(
        out.contains("y=1 at 1"),
        "@(*) must wake on the a-change at t=1; got {out:?}"
    );
}

/// A multi-term in-body EDGE wait is a loud error (frozen IR carries one term;
/// the old behavior silently waited on the FIRST term only).
#[test]
fn inbody_multi_edge_is_rejected() {
    let (reason, _out, diags) = run(r#"
module t;
  reg a, b;
  initial begin
    @(posedge a or negedge b) $display("woke");
  end
endmodule
"#);
    assert_eq!(reason, None, "elaborate must fail; diags: {diags:?}");
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("error[") && d.contains("edge")),
        "expected a multi-edge error; got {diags:?}"
    );
}

/// Single-edge in-body waits keep working.
#[test]
fn inbody_single_edge_still_works() {
    let (reason, out, _d) = run(r#"
module t;
  reg clk;
  initial begin
    clk = 0;
    #1 clk = 1;
    #1 $finish;
  end
  initial begin
    @(posedge clk) $display("edge at %0t", $time);
  end
endmodule
"#);
    assert_eq!(reason, Some(FinishReason::Finish));
    assert!(out.contains("edge at 1"), "got {out:?}");
}
