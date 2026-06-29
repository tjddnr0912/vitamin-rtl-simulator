//! P2-11: `%m` renders the EXECUTING process's hierarchical instance path
//! (was: always the literal `top`). The path rides the `proc_scopes` sidecar;
//! strobe/monitor restore the REGISTERING process's scope at flush.

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, SimOpts};

#[derive(Default)]
struct OutSink(RefCell<String>);
impl LogSink for OutSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::RtlOutput(t) = e {
            self.0.borrow_mut().push_str(&t.text);
        }
    }
}

fn run(src: &str) -> String {
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
    simulate(&ir, &sink, opts);
    sink.0.into_inner()
}

/// `%m` in a submodule prints the instance path; in the top, the top's name —
/// iverilog-parity content (verified live: "tb.u1" / "tb").
#[test]
fn percent_m_renders_instance_path() {
    let out = run(r#"
module sub;
  initial $display("in %m");
endmodule
module tb;
  sub u1();
  initial begin
    $display("at %m");
    #1 $finish;
  end
endmodule
"#);
    assert!(out.contains("in tb.u1"), "submodule %m: {out:?}");
    assert!(out.contains("at tb"), "top %m: {out:?}");
}

/// A `$strobe("%m")` renders the REGISTERING scope at flush even if another
/// module's process ran last in the timestep.
#[test]
fn strobe_percent_m_uses_registering_scope() {
    let out = run(r#"
module sub;
  initial $strobe("strobed in %m");
endmodule
module tb;
  sub u1();
  initial begin
    #1 $finish;
  end
endmodule
"#);
    assert!(out.contains("strobed in tb.u1"), "got {out:?}");
}
