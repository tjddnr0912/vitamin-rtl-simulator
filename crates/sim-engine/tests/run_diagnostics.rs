//! P2-1/P2-3: runtime operational diagnostics (REMAINING_WORK 2026-06-10).
//!
//! Before this fix two failure classes were SILENT: a delta-limit blowup exited 1
//! with zero diagnostic lines, and a VCD open failure dropped the main artifact
//! with exit 0 and no message. Pinned contract:
//!   - delta limit → `fatal[VITA-F4016]` (`F-RUN-NO-CONVERGE`) diagnostic
//!   - `$dumpfile`/`$dumpvars` open failure → `warning[VITA-W4018]`
//!     (`W-RUN-VCD-OPEN-FAIL`), the run itself continues

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, Backend, ExitClass, FinishReason, SimOpts, SimResult};

#[derive(Default)]
struct RunSink {
    out: RefCell<String>,
    diags: RefCell<Vec<String>>,
}

impl LogSink for RunSink {
    fn emit(&self, e: LogEvent) {
        match e {
            LogEvent::RtlOutput(t) => self.out.borrow_mut().push_str(&t.text),
            LogEvent::Diagnostic(d) => self.diags.borrow_mut().push(format!(
                "{}[{}]: {}",
                d.severity.token(),
                d.code.code_num(),
                d.message
            )),
            _ => {}
        }
    }
}

fn run_with(src: &str, backend: Backend) -> (SimResult, String, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = RunSink::default();
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su.expect("source unit"), &sink, &BTreeMap::new(), -9);
    let ir = ir.expect("elaborate");
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        radixes: sc.radixes,
        backend,
        ..SimOpts::default()
    };
    let result = simulate(&ir, &sink, opts);
    (result, sink.out.into_inner(), sink.diags.into_inner())
}

fn run(src: &str) -> (SimResult, String, Vec<String>) {
    run_with(src, Backend::Interpreter)
}

/// A zero-delay event feedback loop blows the per-timestep delta budget: the run
/// must end with a `F-RUN-NO-CONVERGE` FATAL diagnostic, not a bare exit code.
/// Two processes ping-pong (`@(a) b=~b; @(b) a=~a;`) so neither ever settles —
/// a genuine CROSS-process loop. NOTE: a single-process self-write oscillator
/// (`@(a) a=~a`) is NOT infinite (IEEE §9, matched by iverilog: ticks once), so a
/// definite seed plus two procs are required. (A pure `assign a = ~a` is X-stable
/// in 4-state.)
#[test]
fn delta_limit_event_loop_emits_diag() {
    let (res, _out, diags) = run(r#"
module t;
  reg [3:0] a, b;
  initial a = 0;
  always @(a) b = a + 1;
  always @(b) a = b;
endmodule
"#);
    assert_eq!(res.finish_reason, FinishReason::DeltaLimit);
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("fatal[VITA-F4016]") && d.contains("delta limit")),
        "delta-limit must emit F-RUN-NO-CONVERGE; got {diags:?}"
    );
}

/// A suspend-free `forever` body trips the in-body activation guard — same code.
#[test]
fn delta_limit_zero_delay_loop_interp() {
    let (res, _out, diags) = run(r#"
module t;
  reg a;
  initial begin
    a = 0;
    forever a = ~a;
  end
endmodule
"#);
    assert_eq!(res.exit_class, ExitClass::Fatal);
    assert!(
        diags.iter().any(|d| d.starts_with("fatal[VITA-F4016]")),
        "in-body guard must emit F-RUN-NO-CONVERGE; got {diags:?}"
    );
}

/// VM backend: the same guard fires with the identical diagnostic.
#[test]
fn delta_limit_vm_parity() {
    let src = r#"
module t;
  reg a;
  initial begin
    a = 0;
    forever a = ~a;
  end
endmodule
"#;
    let (ri, _oi, di) = run_with(src, Backend::Interpreter);
    let (rv, _ov, dv) = run_with(src, Backend::Bytecode);
    assert_eq!(ri.exit_class, rv.exit_class, "exit class parity");
    assert_eq!(di, dv, "diag parity");
}

/// `$dumpvars` with an unopenable dump path must WARN (W-RUN-VCD-OPEN-FAIL) and
/// keep simulating — previously the VCD silently never appeared (exit 0, 0 lines).
#[test]
fn vcd_open_failure_warns_and_run_continues() {
    let (res, out, diags) = run(r#"
module t;
  initial begin
    $dumpfile("/nonexistent_vita_dir_zz9/x.vcd");
    $dumpvars;
    $display("ran");
    $finish;
  end
endmodule
"#);
    assert_eq!(res.exit_class, ExitClass::Ok, "warning must not dirty exit");
    assert!(out.contains("ran"), "run continues: {out:?}");
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("warning[VITA-W4018]") && d.contains("x.vcd")),
        "open failure must warn with the path; got {diags:?}"
    );
}
