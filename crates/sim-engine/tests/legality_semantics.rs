//! P1-2/3/9: legality promotions (REMAINING_WORK 2026-06-10).
//!
//! These constructs previously WARNED and then misbehaved: `force/release`,
//! procedural `assign/deassign`, `->event` were warn+no-op (values never
//! changed; an `@(ev)` waited forever), a non-constant `#delay` silently
//! degraded to `#0` (turning `forever #x` into a delta-limit blowup), and
//! (2026-06-10 §F-(F): force/release, proc assign/deassign and enclosing-block
//! `disable` are REAL now — the loud lanes below are the remaining v1 cuts) —
//! net-vs-variable assignment legality was never checked (iverilog rejects
//! both directions; doc-02 documents them as errors). All are LOUD now:
//! `E-ELAB-UNSUPPORTED` for the v1 cuts, `E-ELAB-LVALUE-KIND` (VITA-E3018,
//! promoted from doc-15 Appendix A) for the kind violations.

use diag::{LogEvent, LogSink};

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0.borrow_mut().push(format!(
                "{}[{}]: {}",
                d.severity.token(),
                d.code.code_num(),
                d.message
            ));
        }
    }
}

/// lex → parse → elaborate; returns (ir present?, diagnostics).
fn elab(src: &str) -> (bool, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    (ir.is_some(), sink.0.into_inner())
}

fn assert_rejected(src: &str, needle: &str) {
    let (ok, diags) = elab(src);
    assert!(
        !ok,
        "elaborate must FAIL (was warn+no-op); diags: {diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| d.starts_with("error[") && d.contains(needle)),
        "expected an error mentioning {needle:?}; got {diags:?}"
    );
}

/// Named events are REAL (v5 batch B, counter desugar): a declared event
/// elaborates cleanly with `->`/`@()`; an UNDECLARED trigger target stays a
/// hard error, and the event has NO value surface — reads, writes, and a
/// packed range on the declaration are all loud.
#[test]
fn named_event_accepted_and_value_surface_is_loud() {
    let (ok, diags) = elab(
        r#"
module t;
  event ev;
  initial -> ev;
  always @(ev) ;
endmodule
"#,
    );
    assert!(ok, "declared event must elaborate; diags: {diags:?}");
    assert!(
        diags.iter().all(|d| !d.starts_with("error[")),
        "no errors expected: {diags:?}"
    );

    // undeclared trigger target stays loud.
    assert_rejected("module t; initial -> nope; endmodule", "nope");
    // an event cannot be READ as a value…
    assert_rejected(
        "module t; event ev; reg x; initial x = ev; endmodule",
        "event",
    );
    // …or WRITTEN like a variable…
    assert_rejected("module t; event ev; initial ev = 1'b1; endmodule", "event");
    // …and carries no packed range.
    assert_rejected("module t; event [3:0] ev; endmodule", "event");
}

/// force/release semantics landed (format_version 4 follow-up): whole-net
/// targets elaborate cleanly. Behaviour pinned by end_to_end `force_*` +
/// `diff_force_release`; the bit-select reject below keeps the legality edge.
#[test]
fn force_release_whole_net_is_accepted() {
    let (ok, diags) = elab(
        r#"
module t;
  reg a;
  initial begin
    force a = 1'b1;
    release a;
  end
endmodule
"#,
    );
    assert!(
        ok,
        "whole-net force/release must elaborate; diags: {diags:?}"
    );
    assert!(
        diags.iter().all(|d| !d.starts_with("Error")),
        "no errors expected: {diags:?}"
    );
}

/// procedural `assign`/`deassign` (IEEE 1364 §9.3.1) is now REAL: it rides the
/// force machinery at a weaker rank (sidecar-marked StmtIds). Variable targets
/// elaborate cleanly; semantics are pinned by end_to_end + the iverilog
/// differential (const-rhs lanes).
#[test]
fn procedural_assign_is_accepted() {
    let (ok, diags) = elab(
        r#"
module t;
  reg a;
  initial begin
    assign a = 1'b1;
    deassign a;
  end
endmodule
"#,
    );
    assert!(ok, "proc assign/deassign must elaborate; diags: {diags:?}");
    assert!(
        diags.iter().all(|d| !d.starts_with("error[")),
        "no errors expected: {diags:?}"
    );
}

/// §9.3.1 legality: proc-assign targets a VARIABLE (E3018 for a wire) and only
/// a WHOLE variable (bit/part-select is loud-unsupported, like force).
#[test]
fn procedural_assign_to_wire_or_select_is_loud() {
    assert_rejected(
        "module t; wire w; initial assign w = 1'b1; endmodule",
        "VITA-E3018",
    );
    assert_rejected(
        "module t; reg [3:0] r; initial assign r[0] = 1'b1; endmodule",
        "whole",
    );
    assert_rejected(
        "module t; wire w; initial deassign w; endmodule",
        "VITA-E3018",
    );
}

/// format_version 4 flipped the P1-3 contract: a non-constant `#delay` is now
/// SUPPORTED (`Delay.amount` is an ExprId, evaluated at suspension time) — it
/// must elaborate cleanly. Behaviour is pinned by `end_to_end`
/// `runtime_delay_*` + the `diff_runtime_delay` iverilog case.
#[test]
fn nonconstant_delay_is_accepted() {
    let (ok, diags) = elab(
        r#"
module t;
  reg clk;
  integer d;
  initial begin
    d = 5;
    forever #d clk = ~clk;
  end
endmodule
"#,
    );
    assert!(ok, "runtime #delay must elaborate; diags: {diags:?}");
    assert!(
        diags.iter().all(|d| !d.starts_with("Error")),
        "no errors expected: {diags:?}"
    );
}

/// E3018: a continuous assign may not drive a variable (reg).
#[test]
fn cont_assign_to_reg_is_rejected() {
    let (ok, diags) = elab(
        r#"
module t;
  reg r;
  assign r = 1'b1;
endmodule
"#,
    );
    assert!(!ok, "must fail; diags: {diags:?}");
    assert!(
        diags.iter().any(|d| d.contains("VITA-E3018")),
        "expected E-ELAB-LVALUE-KIND; got {diags:?}"
    );
}

/// E3018: a procedural assignment may not target a net (wire).
#[test]
fn procedural_assign_to_wire_is_rejected() {
    let (ok, diags) = elab(
        r#"
module t;
  wire w;
  initial w = 1'b1;
endmodule
"#,
    );
    assert!(!ok, "must fail; diags: {diags:?}");
    assert!(
        diags.iter().any(|d| d.contains("VITA-E3018")),
        "expected E-ELAB-LVALUE-KIND; got {diags:?}"
    );
}

/// Sanity: the legal pairings stay accepted (wire ⇐ assign, reg ⇐ procedural).
#[test]
fn legal_kinds_still_accepted() {
    let (ok, diags) = elab(
        r#"
module t;
  wire w;
  reg r;
  assign w = r;
  initial begin
    r = 1'b1;
    #1 $finish;
  end
endmodule
"#,
    );
    let errs: Vec<&String> = diags.iter().filter(|d| d.starts_with("error[")).collect();
    assert!(
        ok && errs.is_empty(),
        "legal design must elaborate: {errs:?}"
    );
}

// ── P1-8: part/bit-select multi-driver accounting ───────────────────────────

/// Overlapping part-selects from two `assign`s were silently last-write-wins —
/// now E-ELAB-MULTIDRIVER (the v1 policy: no resolution, loud reject).
#[test]
fn overlapping_part_select_drivers_rejected() {
    let (ok, diags) = elab(
        r#"
module t;
  wire [7:0] y;
  reg [7:0] a, b;
  assign y[3:0] = a[3:0];
  assign y[4:2] = b[2:0];
endmodule
"#,
    );
    assert!(!ok, "overlap must fail; diags: {diags:?}");
    assert!(
        diags
            .iter()
            .any(|d| d.contains("E-ELAB-MULTIDRIVER") || d.contains("VITA-E3001")),
        "expected multidriver error; got {diags:?}"
    );
}

/// Whole-net + bit-select on the same net overlaps too.
#[test]
fn whole_net_plus_bit_select_rejected() {
    let (ok, diags) = elab(
        r#"
module t;
  wire [7:0] y;
  reg [7:0] a;
  reg c;
  assign y = a;
  assign y[0] = c;
endmodule
"#,
    );
    assert!(!ok, "overlap must fail; diags: {diags:?}");
    assert!(
        diags.iter().any(|d| d.contains("VITA-E3001")),
        "got {diags:?}"
    );
}

/// DISJOINT part-selects are a legal, common idiom — must stay accepted.
#[test]
fn disjoint_part_selects_accepted() {
    let (ok, diags) = elab(
        r#"
module t;
  wire [7:0] y;
  reg [3:0] a, b;
  assign y[3:0] = a;
  assign y[7:4] = b;
endmodule
"#,
    );
    let errs: Vec<&String> = diags.iter().filter(|d| d.starts_with("error[")).collect();
    assert!(
        ok && errs.is_empty(),
        "disjoint selects are legal: {errs:?}"
    );
}

// ── P2-6 / P2-11 operational legality ───────────────────────────────────────

/// P2-6: a multi-GB unpacked array is rejected loudly (was: OS OOM kill).
#[test]
fn huge_unpacked_array_rejected() {
    let (ok, diags) = elab(
        r#"
module t;
  reg [7:0] m [0:2147483647];
endmodule
"#,
    );
    assert!(!ok, "must fail; diags: {diags:?}");
    assert!(
        diags.iter().any(|d| d.contains("elements")),
        "expected array-cap error; got {diags:?}"
    );
}

/// P2-11: duplicate module definition is E-DUP-UNIT (was: warn, first wins).
#[test]
fn duplicate_module_is_error() {
    let (ok, diags) = elab(
        r#"
module t; endmodule
module t; endmodule
"#,
    );
    assert!(!ok, "must fail; diags: {diags:?}");
    assert!(
        diags.iter().any(|d| d.contains("VITA-E2001")),
        "expected E-DUP-UNIT; got {diags:?}"
    );
}

/// A bit/part-select is not a legal force/release target (IEEE §9.3.2) — loud.
#[test]
fn force_on_bit_select_is_rejected() {
    assert_rejected(
        r#"
module t;
  reg [3:0] q;
  initial force q[1] = 1'b1;
endmodule
"#,
        "force target",
    );
}

#[test]
fn blocking_and_nba_intra_delay_both_accepted() {
    // `a = #d rhs` carries capture-now/write-later semantics; the NBA form
    // `a <= #d rhs` is REAL since format_version 5 (value-carrying delayed
    // NBA event — increment (A)). Both must elaborate cleanly; behaviour is
    // pinned by end_to_end `nba_transport_*` + `diff_nba_transport_delay`.
    for src in [
        "module t; reg [7:0] a, b; initial a = #2 b; endmodule",
        "module t; reg [7:0] a, b; reg clk; always @(posedge clk) a <= #2 b; endmodule",
    ] {
        let (ok, diags) = elab(src);
        assert!(ok, "intra-delay must elaborate; diags: {diags:?}");
        assert!(
            diags.iter().all(|d| !d.starts_with("error[")),
            "no errors expected: {diags:?}"
        );
    }
}

/// `disable` is REAL for a lexically-enclosing named begin-block (break /
/// continue idiom — see end_to_end.rs). Everything else was warn+no-op
/// (silent-wrong: the disable did nothing) and is now LOUD: cross-process
/// targets, unknown labels, hierarchical paths, and a fork child disabling a
/// block OUTSIDE the fork (which would bypass the join barrier).
#[test]
fn disable_non_enclosing_targets_are_loud() {
    // cross-process: A is another process's block.
    assert_rejected(
        "module tb; reg x; initial begin : A x = 1; end \
         always @(x) disable A; endmodule",
        "disable",
    );
    // unknown label.
    assert_rejected("module tb; initial disable NOPE; endmodule", "disable");
    // hierarchical path.
    assert_rejected(
        "module tb; initial begin : A disable tb.A; end endmodule",
        "disable",
    );
    // fork child may not disable a block outside its own body.
    assert_rejected(
        "module tb; initial begin : B fork begin disable B; end join end endmodule",
        "disable",
    );
}
