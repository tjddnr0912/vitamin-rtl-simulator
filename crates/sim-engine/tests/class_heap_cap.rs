//! CLASS-HEAP-CAP (2026-06-22 hardening audit): the N7 class heap is never
//! garbage-collected, so an unbounded `new()` in a loop grows it without limit
//! (≈160 MiB per 1M objects). A `SimOpts::max_class_objs` budget must turn a
//! runaway into a loud `F-RUN-CLASS-LIMIT` fatal (graceful `$finish`, exit
//! class Fatal) instead of an OOM — mirroring `max_deltas` → `F-RUN-NO-CONVERGE`.

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, ExitClass, SimOpts, SimResult};

#[derive(Default)]
struct RunSink {
    diags: RefCell<Vec<String>>,
}

impl LogSink for RunSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.diags.borrow_mut().push(format!(
                "{}[{}]: {}",
                d.severity.token(),
                d.code.code_num(),
                d.message
            ));
        }
    }
}

/// Elaborate + simulate `src` with the full N7 class sidecars threaded and a
/// custom class-object budget.
fn run_capped(src: &str, cap: u64) -> (SimResult, Vec<String>) {
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
        two_state_nets: sc.two_state_nets,
        class_handle_nets: sc.class_handle_nets,
        class_new_sites: sc.class_new_sites,
        class_layouts: sc.class_layouts,
        class_field_inits: sc.class_field_inits,
        class_vtable: sc.class_vtable,
        class_calls: sc.class_calls,
        class_field_widths: sc.class_field_widths,
        max_class_objs: cap,
        ..SimOpts::default()
    };
    let result = simulate(&ir, &sink, opts);
    (result, sink.diags.into_inner())
}

const PER_ITER_NEW: &str = r#"
class C; int x; function new(); x = 1; endfunction endclass
module t; C c; integer i;
  initial begin
    for (i = 0; i < 12; i = i + 1) c = new;
    $finish;
  end
endmodule
"#;

/// Allocating past the budget fails loud (F-RUN-CLASS-LIMIT, Fatal), not OOM.
#[test]
fn class_heap_budget_exceeded_is_fatal() {
    let (res, diags) = run_capped(PER_ITER_NEW, 5);
    assert_eq!(
        res.exit_class,
        ExitClass::Fatal,
        "exceeding the class budget must be Fatal; diags={diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("fatal[VITA-F4024]") && d.contains("budget")),
        "must emit F-RUN-CLASS-LIMIT; got {diags:?}"
    );
}

/// A design that stays within the budget runs to a clean finish — the cap
/// never perturbs legitimate class use.
#[test]
fn class_heap_within_budget_is_clean() {
    let (res, diags) = run_capped(PER_ITER_NEW, 1000);
    assert_eq!(
        res.exit_class,
        ExitClass::Ok,
        "within budget must finish cleanly; diags={diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.contains("F4024")),
        "no class-limit diag within budget; got {diags:?}"
    );
}
