//! P3-1: fork-arena slot recycling under churn. A `fork…join_none` in a loop
//! used to grow the activity/barrier arenas O(iterations) (~800 MB over 10M
//! cycles); slots are now recycled once a child reports / a barrier drains.
//! These tests pin that the recycling preserves semantics under heavy churn
//! (ids are internal — byte-identity over the corpus is enforced by P5).

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, FinishReason, SimOpts};

#[derive(Default)]
struct OutSink(RefCell<String>);
impl LogSink for OutSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::RtlOutput(t) = e {
            self.0.borrow_mut().push_str(&t.text);
        }
    }
}

fn run(src: &str) -> (FinishReason, String) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = OutSink::default();
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su.expect("source unit"), &sink, &BTreeMap::new(), -9);
    let ir = ir.expect("elaborate");
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        radixes: sc.radixes,
        proc_scopes: sc.proc_scopes,
        ..SimOpts::default()
    };
    let res = simulate(&ir, &sink, opts);
    (res.finish_reason, sink.0.into_inner())
}

/// 5000 join_none forks in a loop: every child runs exactly once (slot reuse
/// must not lose or duplicate work).
#[test]
fn join_none_churn_runs_every_child_once() {
    let (reason, out) = run(r#"
module t;
  integer i;
  reg [31:0] acc;
  initial begin
    acc = 0;
    for (i = 0; i < 5000; i = i + 1) begin
      fork
        acc = acc + 1;
      join_none
      #1;
    end
    #1 $display("acc=%0d", acc);
    $finish;
  end
endmodule
"#);
    assert_eq!(reason, FinishReason::Finish);
    assert!(out.contains("acc=5000"), "got {out:?}");
}

/// Churning `fork…join` (blocking) with TWO children per iteration — barrier
/// slots recycle between iterations; both children always complete first.
#[test]
fn blocking_join_churn_recycles_barriers() {
    let (reason, out) = run(r#"
module t;
  integer i;
  reg [31:0] a, b;
  initial begin
    a = 0; b = 0;
    for (i = 0; i < 2000; i = i + 1) begin
      fork
        a = a + 1;
        b = b + 2;
      join
    end
    $display("a=%0d b=%0d", a, b);
    $finish;
  end
endmodule
"#);
    assert_eq!(reason, FinishReason::Finish);
    assert!(out.contains("a=2000 b=4000"), "got {out:?}");
}
