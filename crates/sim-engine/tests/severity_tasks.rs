//! P1-1: `$fatal`/`$error`/`$warning`/`$info` severity tasks (REMAINING_WORK
//! 2026-06-10).
//!
//! Before this fix the four severity tasks were "unsupported system task →
//! warn + skip": a failing `$fatal` testbench finished with exit 0 (CI silent
//! PASS). The contract pinned here (doc-13 §Severity):
//!   - `$fatal`   → `fatal[VITA-F4004]` diagnostic + immediate abort, ExitClass::Fatal
//!   - `$error`   → `error[VITA-E4003]` diagnostic, run continues, ExitClass::HadErrors
//!   - `$warning` → `warning[VITA-W4007]`, run continues, ExitClass::Ok
//!   - `$info`    → `info[VITA-I4005]`, run continues, ExitClass::Ok
//!
//! Severity text goes to the DIAGNOSTIC stream (stderr), never stdout. The
//! severity table rides out-of-band (SimOpts), so the frozen SimIr is untouched.

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, Backend, ExitClass, FinishReason, SimOpts, SimResult};

#[derive(Default)]
struct SevSink {
    out: RefCell<String>,
    diags: RefCell<Vec<String>>,
}

impl LogSink for SevSink {
    fn emit(&self, e: LogEvent) {
        match e {
            LogEvent::RtlOutput(t) => self.out.borrow_mut().push_str(&t.text),
            LogEvent::Diagnostic(d) => {
                let t = d.sim_time.map(|t| t.ticks as i64).unwrap_or(-1);
                self.diags.borrow_mut().push(format!(
                    "{}[{}]@{}: {}",
                    d.severity.token(),
                    d.code.code_num(),
                    t,
                    d.message
                ));
            }
            _ => {}
        }
    }
}

/// lex → parse → elaborate (full sidecars) → simulate with the given backend;
/// returns (result, stdout, diagnostics).
fn run_with(src: &str, backend: Backend) -> (SimResult, String, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = SevSink::default();
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

/// `$fatal` aborts the run: later statements never execute, exit class Fatal.
#[test]
fn fatal_aborts_with_fatal_class() {
    let (res, out, _diags) = run(r#"
module t;
  initial begin
    $display("before");
    $fatal;
    $display("after");
  end
endmodule
"#);
    assert_eq!(res.exit_class, ExitClass::Fatal, "exit class");
    assert_eq!(res.finish_reason, FinishReason::Error, "finish reason");
    assert!(out.contains("before"), "pre-fatal output runs: {out:?}");
    assert!(
        !out.contains("after"),
        "post-fatal stmt must not run: {out:?}"
    );
}

/// `$fatal(finish_number, fmt, args)` — the leading 0/1/2 literal is the
/// finish_number (consumed, not printed); message + sim_time render in the diag.
#[test]
fn fatal_renders_message_and_time() {
    let (res, _out, diags) = run(r#"
module t;
  reg [7:0] v;
  initial begin
    v = 5;
    #3 $fatal(1, "boom %0d", v);
  end
endmodule
"#);
    assert_eq!(res.exit_class, ExitClass::Fatal);
    assert!(
        diags.iter().any(|d| d == "fatal[VITA-F4004]@3: boom 5"),
        "expected fatal diag with message+time; got {diags:?}"
    );
}

/// `$fatal("...")` without a finish_number: first string arg is the format.
#[test]
fn fatal_without_finish_number() {
    let (_res, _out, diags) = run(r#"
module t;
  initial $fatal("plain %0d", 9);
endmodule
"#);
    assert!(
        diags
            .iter()
            .any(|d| d.contains("fatal[VITA-F4004]@0: plain 9")),
        "got {diags:?}"
    );
}

/// Bare `$fatal;` still aborts and emits the code's title as the message.
#[test]
fn fatal_bare_uses_title() {
    let (res, _out, diags) = run(r#"
module t;
  initial $fatal;
endmodule
"#);
    assert_eq!(res.exit_class, ExitClass::Fatal);
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("fatal[VITA-F4004]@0: ")
                && d.len() > "fatal[VITA-F4004]@0: ".len()),
        "bare $fatal must carry a non-empty default message; got {diags:?}"
    );
}

/// `$error` records the diagnostic and CONTINUES; exit class HadErrors.
#[test]
fn error_continues_run() {
    let (res, out, diags) = run(r#"
module t;
  initial begin
    $error("e%0d", 1);
    #1 $display("alive");
    $finish;
  end
endmodule
"#);
    assert_eq!(res.exit_class, ExitClass::HadErrors, "exit class");
    assert_eq!(res.finish_reason, FinishReason::Finish, "run completed");
    assert!(
        out.contains("alive"),
        "run must continue after $error: {out:?}"
    );
    assert!(
        diags.iter().any(|d| d == "error[VITA-E4003]@0: e1"),
        "got {diags:?}"
    );
}

/// `$warning`/`$info` print diagnostics but leave the exit class clean.
#[test]
fn warning_and_info_stay_clean() {
    let (res, _out, diags) = run(r#"
module t;
  initial begin
    $warning("w");
    $info("i");
    $finish;
  end
endmodule
"#);
    assert_eq!(
        res.exit_class,
        ExitClass::Ok,
        "warning/info must not dirty exit"
    );
    assert!(
        diags.iter().any(|d| d == "warning[VITA-W4007]@0: w"),
        "got {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d == "info[VITA-I4005]@0: i"),
        "got {diags:?}"
    );
}

/// Severity text is diagnostic-only — it must NOT leak into the stdout stream.
#[test]
fn severity_not_on_stdout() {
    let (_res, out, _diags) = run(r#"
module t;
  initial begin
    $error("secret %0d", 7);
    $finish;
  end
endmodule
"#);
    assert!(
        !out.contains("secret"),
        "severity text leaked to stdout: {out:?}"
    );
}

/// VM backend parity: severity behavior is byte-identical across backends.
#[test]
fn vm_backend_parity() {
    let src = r#"
module t;
  reg [7:0] v;
  initial begin
    v = 9;
    $error("e %0d", v);
    #2 $fatal(0, "f %0d", v);
  end
endmodule
"#;
    let (ri, oi, di) = run_with(src, Backend::Interpreter);
    let (rv, ov, dv) = run_with(src, Backend::Bytecode);
    assert_eq!(ri.exit_class, rv.exit_class, "exit class parity");
    assert_eq!(ri.finish_reason, rv.finish_reason, "finish reason parity");
    assert_eq!(oi, ov, "stdout parity");
    assert_eq!(di, dv, "diag parity");
}
