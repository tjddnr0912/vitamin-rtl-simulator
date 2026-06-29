//! P1-6: `$finish`/`$stop`/`$fatal` in the SAME timestep as a registered
//! `$strobe`/`$monitor` must drain the postponed region first (IEEE 1364-2005
//! §5.4/§17 — Icarus/VCS behavior). The old MVP returned before the flush,
//! silently dropping the output.

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, ExitClass, SimOpts};

#[derive(Default)]
struct OutSink(RefCell<String>);
impl LogSink for OutSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::RtlOutput(t) = e {
            self.0.borrow_mut().push_str(&t.text);
        }
    }
}

fn run(src: &str) -> (ExitClass, String) {
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
        ..SimOpts::default()
    };
    let res = simulate(&ir, &sink, opts);
    (res.exit_class, sink.0.into_inner())
}

/// `$monitor` established in the same step as `$finish` still does its
/// establishment print.
#[test]
fn monitor_establishment_survives_same_step_finish() {
    let (_c, out) = run(r#"
module t;
  reg [3:0] v;
  initial begin
    v = 9;
    $monitor("v=%0d", v);
    $finish;
  end
endmodule
"#);
    assert_eq!(out, "v=9\n", "monitor establishment print must flush");
}

/// `$fatal` is an implicit `$finish` — the same-step strobe flushes before the
/// fatal terminates the run.
#[test]
fn strobe_survives_same_step_fatal() {
    let (class, out) = run(r#"
module t;
  reg [3:0] v;
  initial begin
    v = 5;
    $strobe("s=%0d", v);
    $fatal(1, "boom");
  end
endmodule
"#);
    assert_eq!(class, ExitClass::Fatal);
    assert_eq!(out, "s=5\n", "strobe must flush before $fatal aborts");
}

/// `$stop` drains too.
#[test]
fn strobe_survives_same_step_stop() {
    let (_c, out) = run(r#"
module t;
  reg [3:0] v;
  initial begin
    v = 7;
    $strobe("s=%0d", v);
    $stop;
  end
endmodule
"#);
    assert_eq!(out, "s=7\n");
}
