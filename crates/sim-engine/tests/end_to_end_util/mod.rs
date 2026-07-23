#![allow(dead_code)]
#![allow(unused_imports)]
// shared helpers for the split end_to_end integration tests (mechanical move)

use std::cell::RefCell;
use std::rc::Rc;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, simulate_capture, Backend, FinishReason, SimOpts};

// ── pipeline + sink helpers ────────────────────────────────────────────────

#[derive(Default)]
pub struct DiagSink(pub RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

pub fn build(src: &str) -> sim_ir::SimIr {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    ir.expect("elaborate returned None")
}

/// Full front-end incl. preprocess + per-module `timescale` resolution. Returns
/// `(ir, opts)` with `proc_multipliers` threaded so delays scale to global-precision
/// ticks and `$time`/`$realtime` divide by the calling module's multiplier.
pub fn build_timescaled(src: &str) -> (sim_ir::SimIr, SimOpts) {
    let pp = hdl_preprocess::preprocess_str(
        std::path::Path::new("/v"),
        "t.sv",
        src,
        &hdl_preprocess::PreOpts::default(),
    );
    assert!(!pp.has_errors(), "pp errors: {:?}", pp.diags);
    let (toks, le) = hdl_lexer::lex(&pp.text);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, &pp.text);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let su = su.expect("source unit");
    let modules: Vec<(&str, usize)> = su
        .items
        .iter()
        .filter_map(|it| match it {
            hdl_ast::TopItem::Module(m) => Some((m.name.name.as_str(), m.span.lo as usize)),
            _ => None,
        })
        .collect();
    let rt = hdl_preprocess::resolve_module_timescales(&modules, &pp.timescales);
    let sink = DiagSink::default();
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su, &sink, &rt.unit_exp, rt.global_prec_exp);
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        assign_ranks: sc.assign_ranks,
        radixes: sc.radixes,
        ..SimOpts::default()
    };
    (ir.expect("elaborate returned None"), opts)
}

/// Elaborate `src` WITH the per-net hierarchical name side table (for VCD naming).
pub fn build_named(src: &str) -> (sim_ir::SimIr, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let (ir, _modes, names) = elaborate::elaborate_with_sidecars(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    (ir.expect("elaborate returned None"), names)
}

/// Elaborate `src` WITH the fork-mode side table and return `(ir, opts)` where
/// `opts` carries `fork_modes`. Existing non-fork tests keep using
/// `build()`/`SimOpts::default()` unchanged. Fork tests do:
///   `let (ir, opts) = build_fork(src); simulate_capture(&ir, opts);`
pub fn build_fork(src: &str) -> (sim_ir::SimIr, SimOpts) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let (ir, fork_modes) = elaborate::elaborate_with_modes(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let ir = ir.expect("elaborate returned None");
    (
        ir,
        SimOpts {
            fork_modes,
            ..SimOpts::default()
        },
    )
}

/// A unique temp VCD path per test.
pub fn tmp_vcd(tag: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("vita_sim_{}_{}.vcd", tag, std::process::id()));
    p.to_string_lossy().into_owned()
}

pub fn opts_with_vcd(path: &str) -> SimOpts {
    SimOpts {
        vcd_path_override: Some(path.to_string()),
        ..SimOpts::default()
    }
}

/// Drop the build-version-dependent `$version … $end` block for a stable golden.
pub fn strip_version_block(vcd: &str) -> String {
    let mut out = String::new();
    let mut lines = vcd.lines();
    while let Some(l) = lines.next() {
        if l == "$version" {
            for x in lines.by_ref() {
                if x == "$end" {
                    break;
                }
            }
        } else {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

// helper to silence unused import warnings if a test path drops them
#[allow(dead_code)]
pub fn _touch() {
    let _ = Rc::new(RefCell::new(0));
}

// ════════════════════════════════════════════════════════════════════════════
// REAL / REALTIME DOMAIN (deliberate sim-ir evolution, format_version 2→3)
// Strings are blessed against the §4.1 formatter algorithms as written.
// ════════════════════════════════════════════════════════════════════════════

/// Build + simulate `src`, returning the captured $display/$write output.
pub fn run_sv(src: &str) -> String {
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    out
}

/// Build `src` through lex→parse→elaborate, returning the collected diagnostic
/// strings (severity-prefixed). Used to assert real-operand illegality gates.
pub fn elaborate_diags(src: &str) -> Vec<String> {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let _ = elaborate::elaborate(&su.expect("source unit"), &sink);
    let collected = sink.0.borrow().clone();
    drop(sink);
    collected
}
