//! S1d-4c-2c gate — **the whole run, both backends, byte for byte.**
//!
//! Every earlier tier-3 gate compared a PART: a read, a write, a changed set, a
//! wake decision, one statement, one body. This one compares the thing the user
//! sees. `simulate` is driven twice over the same `SimIr` — once on the bytecode
//! VM, once with `Backend::Native` — and the two must agree on stdout BYTES, on
//! the finish reason, and on the final simulation time.
//!
//! That is the original S1 gate ("corpus 적격분 stdout+VCD 바이트 동일") minus
//! VCD, which S1d-4d owns because `$dumpvars` is still a refused system task.
//!
//! ## The check that makes it non-vacuous
//!
//! `Backend::Native` FALLS BACK to the VM whenever any of the three gate layers
//! refuses. So "the two agree" is trivially true for a refused design — they are
//! the same run. Every assertion here is therefore paired with
//! `assert_eq!(res.backend, Backend::Native)`: the comparison counts only when
//! the native executor actually ran it. The admitted COUNT is pinned exactly,
//! for the same reason every earlier gate pins its comparison count — a
//! predicate that quietly narrows would otherwise read as a passing gate.

use std::cell::RefCell;

use crate::{simulate, simulate_capture, Backend, FinishReason, SimOpts};

use super::test_common as common;
use super::tests::build_with_opts;
use common::corpus;

/// A sink that keeps stdout and diagnostics in ONE interleaved list.
///
/// `simulate_capture`'s `CaptureSink` keeps only `LogEvent::RtlOutput`, so a
/// gate built on it is structurally blind to every diagnostic — which is where
/// this backend's one real defect lived. And `SimResult.exit_class` does NOT
/// close that hole: it reads `st.had_error`, which only `$error`-family
/// severities set. `warn_run_range` emits an `Error` diagnostic WITHOUT setting
/// it (the CLI's own sink is what counts diagnostics into the process exit
/// code), so an `exit_class` compare passes on exactly the defect it looks like
/// it covers. Measured — the first version of this gate asserted `exit_class`
/// and the OOB-NBA design still read `Ok` on both backends.
///
/// Interleaved rather than two lists, because ORDER is a real axis here and the
/// second review round found a divergence on it: the arena reported at the
/// STATEMENT boundary while the engine reports at the ACCESS, so an out-of-range
/// read inside `$error("%0d", mem[i])` came out after its own `$error` line
/// instead of before it. Two separate lists would have compared equal. The fix
/// put a drain inside the format engine (which holds the reader and the sink at
/// once); this comparison is what keeps it there.
#[derive(Default)]
struct MergedSink {
    events: RefCell<Vec<String>>,
}

impl diag::LogSink for MergedSink {
    fn emit(&self, e: diag::LogEvent) {
        let row = match e {
            diag::LogEvent::RtlOutput(t) => format!("out|{}", t.text),
            diag::LogEvent::Diagnostic(d) => {
                format!("diag|{:?}|{}|{}", d.severity, d.code.code_num(), d.message)
            }
            // Progress events carry wall-clock-ish bookkeeping, not simulation
            // output; including them would make the gate flaky rather than strict.
            diag::LogEvent::Progress(_) => return,
        };
        self.events.borrow_mut().push(row);
    }
}

/// Run one design on both backends and assert they agree.
///
/// Returns `Err(reason)` when the third gate layer refuses — the caller counts
/// those rather than letting them pass silently as agreements.
fn agree(src: &str, name: &str) -> Result<(), &'static str> {
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts)?;

    let vm = SimOpts {
        backend: Backend::Bytecode,
        ..opts.clone()
    };
    let nat = SimOpts {
        backend: Backend::Native,
        ..opts
    };
    let sink_vm = MergedSink::default();
    let sink_nat = MergedSink::default();
    let r_vm = simulate(&ir, &sink_vm, vm);
    let r_nat = simulate(&ir, &sink_nat, nat);
    let out_vm = sink_vm.events.into_inner();
    let out_nat = sink_nat.events.into_inner();

    // ANTI-VACUITY, first: a fall-back would make every other assertion compare
    // the VM against itself.
    assert_eq!(
        r_nat.backend,
        Backend::Native,
        "{name}: `runnable` said yes but `simulate` fell back to {:?} (refused: {:?}) — \
         the two gates disagree, and every comparison below would have been the VM \
         against itself",
        r_nat.backend,
        r_nat.native.refused
    );
    assert_eq!(
        out_vm, out_nat,
        "{name}: the interleaved stdout+diagnostic stream differs"
    );
    assert_eq!(
        r_vm.finish_reason, r_nat.finish_reason,
        "{name}: finish reason differs"
    );
    assert_eq!(r_vm.sim_time, r_nat.sim_time, "{name}: end time differs");
    assert_eq!(
        r_vm.exit_class, r_nat.exit_class,
        "{name}: exit class differs"
    );
    Ok(())
}

/// The P6 corpus, with its `$dumpfile`/`$dumpvars` lines removed.
///
/// Stripping them is not papering over a difference: the two backends are
/// compared on the SAME `SimIr` either way, so an unstripped corpus would simply
/// be REFUSED wholesale (44 of the 72 designs carry a dump task, which
/// `k_dispatch_systask` will not render from the arena). Removing the two lines
/// converts 44 refusals into 44 comparisons, and the VCD path they exercise is
/// S1d-4d's gate rather than this one. Both numbers are asserted below.
fn corpus_no_vcd() -> Vec<(String, String)> {
    corpus(0x5EED_F00D, 72)
        .into_iter()
        .map(|d| {
            let src = d
                .src
                .lines()
                .filter(|l| !l.contains("$dumpfile") && !l.contains("$dumpvars"))
                .collect::<Vec<_>>()
                .join("\n");
            (d.name, src)
        })
        .collect()
}

/// The gate: the corpus's runnable subset, end to end, on both backends.
#[test]
fn s1d4c2c_native_run_matches_the_vm_over_corpus() {
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in corpus_no_vcd() {
        match agree(&src, &name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    // EXACT, not a floor. The refusal breakdown is asserted too, so a row that
    // starts firing on designs it never used to — the way a widened predicate
    // silently shrinks a gate — moves a number here instead of passing.
    assert_eq!(
        (ran, refused.len()),
        (65, 1),
        "corpus coverage moved — re-pin deliberately. ran={ran} refused={refused:?}"
    );
    assert_eq!(
        refused.get("continuous assigns (S1d-4d settles them)"),
        Some(&7),
        "the only expected refusal is the cont-assign row: {refused:?}"
    );
}

/// The corpus AS GENERATED — i.e. with the VCD tasks — must be REFUSED, not run
/// wrongly. The stripping above is only legitimate if the unstripped form is
/// loud, and this is the assertion that keeps it so.
#[test]
fn s1d4c2c_vcd_designs_are_refused_not_run() {
    let mut with_dump = 0usize;
    for d in corpus(0x5EED_F00D, 72) {
        if !d.src.contains("$dumpvars") {
            continue;
        }
        with_dump += 1;
        let (ir, opts) = build_with_opts(&d.src);
        let verdict = crate::native::run::runnable(&ir, &opts);
        // The ROW is asserted only where it is the only candidate. A design that
        // also has continuous assigns is refused by that row first (the checks
        // are ordered), and pinning the message there would be pinning the ORDER
        // of two independent refusals rather than the refusal itself.
        if ir.cont_assigns.is_empty() {
            assert_eq!(
                verdict,
                Err("a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)"),
                "{}: a $dumpvars design must be refused by the dispatch row",
                d.name
            );
        } else {
            assert!(verdict.is_err(), "{}: must be refused", d.name);
        }
        // …and the refusal must reach `simulate`, not merely exist: a request for
        // the native backend has to come back as a bytecode run.
        let (r, _) = simulate_capture(
            &ir,
            SimOpts {
                backend: Backend::Native,
                vcd_path_override: Some(
                    std::env::temp_dir()
                        .join("vita_s1d4c2c_refused.vcd")
                        .to_string_lossy()
                        .into_owned(),
                ),
                ..opts
            },
        );
        assert_eq!(r.backend, Backend::Bytecode, "{}: no fall-back", d.name);
    }
    assert_eq!(with_dump, 44, "corpus VCD population moved");
}

/// One design per shape the corpus does not contain, each named for the rule it
/// is the only cover of. Measured absences, not guesses: the corpus's 138
/// suspending terminators are ALL `Delay`/`Active`, it has no transport NBA, no
/// `#0`, no self-write sensitivity, no oscillator and no quiescent end.
fn adversarial_designs() -> Vec<(&'static str, String)> {
    vec![
        // `#0` — the INACTIVE region. The whole `inactive` queue and its
        // promotion step are unreached by the corpus.
        // A `#d` whose amount evaluates to ZERO ticks is a `#0` too — the second
        // half of the Inactive predicate, and the half a `matches!(region, …)`
        // alone would drop.
        // TRANSPORT NBA (`<= #3`) — files into `delayed_nba`, which the time
        // advance must fold into its minimum or the update vanishes.
        (
            "transport_nba_only_pending_work",
            r#"
module top;
  reg [7:0] q;
  initial begin q = 8'd0; q <= #3 8'd42; end
  initial begin #9 $display("q=%0d t=%0t", q, $time); $finish; end
endmodule
"#
            .to_string(),
        ),
        // A transport NBA as the ONLY pending work at its tick: if the time
        // advance ignored `delayed_nba` the run would end quiescent at t=0 with
        // q still 0, and nothing else would notice.
        (
            "transport_nba_drives_a_wake",
            r#"
module top;
  reg [7:0] q, seen;
  initial begin q = 8'd0; seen = 8'd0; q <= #4 8'd5; end
  always @(q) seen = seen + 8'd1;
  initial begin #10 $display("q=%0d seen=%0d", q, seen); $finish; end
endmodule
"#
            .to_string(),
        ),
        // SELF-WRITE suppression: a process sensitive to a net it blocking-writes
        // must not re-fire on its own write. Without the `blocking_writer` tag
        // this is an infinite delta loop, not a wrong value.
        // DECLARATION INITIALIZER: `reg clk = 0;` must NOT hand `always @clk` an
        // x→0 edge (IEEE 1800 §6.21) — the un-dirty step in `arm_t0`.
        (
            "decl_init_gives_no_edge",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] hits;
  initial begin hits = 8'd0; #1 clk = 1'b1; #1 $display("hits=%0d", hits); $finish; end
  always @(clk) hits = hits + 8'd1;
endmodule
"#
            .to_string(),
        ),
        // QUIESCENT end — no `$finish` anywhere. The `None` arm of the time
        // advance, which every corpus design (all 72 carry a `$finish`) skips.
        (
            "quiescent_no_finish",
            r#"
module top;
  reg [7:0] a;
  initial begin a = 8'd3; #2 a = 8'd4; $display("a=%0d", a); end
endmodule
"#
            .to_string(),
        ),
        // NBA ordering across a clock edge — the `seq` sort is only observable
        // once a delta loop exists to interleave two queues.
        (
            "nba_swap_across_edge",
            r#"
module top;
  reg clk;
  reg [7:0] x, y;
  initial begin clk = 1'b0; x = 8'd1; y = 8'd2; #1 clk = 1'b1; #1 $display("x=%0d y=%0d", x, y); $finish; end
  always @(posedge clk) begin x <= y; y <= x; end
endmodule
"#
            .to_string(),
        ),
        // A `$finish` reached mid-batch: the processes after it in the SAME
        // Active batch must not run.
        (
            "finish_stops_the_batch",
            r#"
module top;
  reg [7:0] a;
  initial begin a = 8'd0; $display("first"); $finish; end
  initial begin $display("second"); end
endmodule
"#
            .to_string(),
        ),
        // A combinational oscillator: both backends must report the SAME
        // delta-limit termination at the same point.
        (
            "delta_oscillator",
            r#"
module top;
  reg a, b;
  initial begin a = 1'b0; b = 1'b0; end
  always @(a) b = ~a;
  always @(b) a = ~b;
  initial begin #100 $finish; end
endmodule
"#
            .to_string(),
        ),

        // ZERO-TICK vs `#0`: both must land in INACTIVE, and the only way to
        // see that is an ORDER flip. p0's `#0` and p1's `#(z)` (z == 0) resume
        // at the same tick; if the zero-tick one went to Active it would drain
        // BEFORE the Inactive batch and print first. An earlier version of this
        // design had both processes print in proc order either way, so dropping
        // the `|| ticks == 0` half survived it — measured.
        (
            "zero_ticks_is_inactive_not_active",
            r#"
module top;
  reg [7:0] a, b;
  reg [3:0] z;
  initial begin : p0 a = 8'd0; z = 4'd0; #1 a = 8'd1; #0 $display("p0 zero"); end
  initial begin : p1 b = 8'd0; #1 b = 8'd1; #(z) $display("p1 zero"); end
  initial begin #4 $finish; end
endmodule
"#
            .to_string(),
        ),
        // `#0` must be INACTIVE, which drains BEFORE the NBA region. With the
        // resume filed on the wheel (or into Active) instead, the NBA applies
        // first and the `#0` process reads the updated value.
        (
            "zero_delay_reads_pre_nba_value",
            r#"
module top;
  reg [7:0] a, b;
  initial begin : p0 a = 8'd1; b = 8'd1; #0 $display("zero b=%0d", b); end
  initial begin : p1 b <= 8'd9; end
  initial begin #2 $display("late b=%0d", b); $finish; end
endmodule
"#
            .to_string(),
        ),
        // …and the other half of the region distinction: an INACTIVE resume must
        // wait for Active to empty. The `#0` process is woken alongside a
        // level-sensitive process; Inactive means the level process prints first.
        (
            "inactive_waits_for_active",
            r#"
module top;
  reg [7:0] a, hits;
  initial begin : p0 hits = 8'd0; a = 8'd1; #0 $display("zero hits=%0d", hits); end
  always @(a) begin hits = hits + 8'd1; $display("wake a hits=%0d", hits); end
  initial begin #2 $finish; end
endmodule
"#
            .to_string(),
        ),
        // BUSY: an `always @(posedge clk)` that suspends mid-body must not be
        // re-entered by a posedge arriving while it is parked — and must be
        // re-armed once it completes. Two posedges inside the body's `#3` window
        // and one after it, so BOTH halves of the flag are observable (setting it
        // and clearing it). The corpus contains no suspending edge-sensitive
        // body at all, so both mutations survived without this.
        (
            "busy_suppresses_reentry_and_clears",
            r#"
module top;
  reg clk;
  reg [7:0] n;
  initial begin
    clk = 1'b0; n = 8'd0;
    #1 clk = 1'b1; #1 clk = 1'b0; #1 clk = 1'b1; #1 clk = 1'b0;
    #10 clk = 1'b1; #1 clk = 1'b0;
    #5 $display("n=%0d", n); $finish;
  end
  always @(posedge clk) begin n = n + 8'd1; #3 n = n + 8'd16; end
endmodule
"#
            .to_string(),
        ),
        // SELF-WRITE suppression: the process must be sensitive to a net IT
        // blocking-writes, or the rule is not exercised at all. The previous
        // design incremented a net that was NOT in its sensitivity list, so
        // dropping the `blocking_writer` tag entirely survived it.
        (
            "self_write_does_not_retrigger",
            r#"
module top;
  reg [7:0] n, cnt;
  initial begin n = 8'd0; cnt = 8'd0; #1 n = 8'd4; #1 $display("cnt=%0d n=%0d", cnt, n); $finish; end
  always @(n) begin cnt = cnt + 8'd1; n = n | 8'd1; end
endmodule
"#
            .to_string(),
        ),
        // GATED-CLOCK, cluster 1 of 3 — TIME ADVANCE resets the per-timestep
        // edge-dedup marks. Deliberately NBA-free and `#0`-free: a counter that
        // does `q <= …` resets the marks at the NBA region every cycle, which is
        // why 65 corpus designs did not catch the missing reset here.
        (
            "edge_dedup_resets_at_time_advance",
            r#"
module top;
  reg clk;
  reg [7:0] n;
  initial begin clk = 1'b0; n = 8'd0; #1 clk = 1'b1; #1 clk = 1'b0; #1 clk = 1'b1; #1 $display("n=%0d", n); $finish; end
  always @(posedge clk) n = n + 8'd1;
endmodule
"#
            .to_string(),
        ),
        // …cluster 2: the NBA region is a NEW cluster, so an edge an NBA update
        // produces must re-fire a process already woken this timestep.
        (
            "edge_dedup_resets_at_nba",
            r#"
module top;
  reg clk, rst;
  reg [7:0] n;
  initial begin clk = 1'b0; rst = 1'b1; n = 8'd0; #1 clk = 1'b1; #5 $display("n=%0d", n); $finish; end
  always @(posedge clk) rst <= 1'b0;
  always @(posedge clk or negedge rst) n = n + 8'd1;
endmodule
"#
            .to_string(),
        ),
        // …cluster 3: so is the `#0` (Inactive) promotion.
        (
            "edge_dedup_resets_at_inactive",
            r#"
module top;
  reg clk, rst;
  reg [7:0] n;
  initial begin clk = 1'b0; rst = 1'b1; n = 8'd0; #1 clk = 1'b1; end
  initial begin #1 #0 rst = 1'b0; end
  always @(posedge clk or negedge rst) n = n + 8'd1;
  initial begin #5 $display("n=%0d", n); $finish; end
endmodule
"#
            .to_string(),
        ),
        // NEGATIVE REAL delay: `delay_ticks_of` returns `u64::MAX` for it, which
        // must mean "never fires". A wrapping add would file the resume at
        // `now - 1` — a wheel key BELOW `now`, which the time advance would then
        // pick, running the simulation BACKWARDS.
        (
            "negative_real_delay_never_fires",
            r#"
module top;
  reg [7:0] a;
  initial begin a = 8'd0; #5 $display("at %0t", $time); #(-1.0) $display("never a=%0d", a); end
  initial begin #9 $display("done %0t", $time); $finish; end
endmodule
"#
            .to_string(),
        ),
        // An out-of-range read INSIDE a diagnostic's own arguments. Both
        // diagnostics go to the same stream, so their relative ORDER is
        // observable without merging two fds — the engine emits E4002 at the
        // read, i.e. BEFORE the `$error` line. Round 2 measured this backwards
        // (E4003 then E4002) and it is what put the drain inside the format
        // engine: that function holds the reader AND the sink, so it can report
        // after the argument reads and before the caller emits.
        (
            "oob_inside_a_severity_argument",
            r#"
module top;
  reg [7:0] mem [0:1];
  integer i;
  initial begin i = 9; $error("e v=%0d", mem[i]); $display("after"); #1 $finish; end
endmodule
"#
            .to_string(),
        ),
        // …and inside a plain `$display`'s arguments: the diagnostic must come
        // out BEFORE the line it was read for, not after it.
        (
            "oob_inside_a_display_argument",
            r#"
module top;
  reg [7:0] mem [0:1];
  integer i;
  initial begin i = 9; $display("before"); $display("a=%0d", mem[i]); $display("after"); #1 $finish; end
endmodule
"#
            .to_string(),
        ),
        // A DECLARATION INITIALIZER that reads out of range, in a design with no
        // processes at all — `arm_t0` runs it before anything is armed and the
        // run then goes straight to quiescence, so the report has to survive a
        // path with no format-engine call and no later body.
        //
        // ⚠️ It does NOT cover `arm_t0`'s own `drain_range_diags` — measured:
        // removing that line keeps this green, because `run_body` drains at
        // every statement boundary and an initializer body is straight-line.
        // Saying so beats letting the design's name imply a cover it does not
        // provide.
        (
            "oob_in_a_declaration_initializer",
            r#"
module top;
  reg [7:0] mem [0:1];
  reg [7:0] x = mem[9];
endmodule
"#
            .to_string(),
        ),
        // OUT-OF-RANGE array access — the silent-wrong this slice's own review
        // found, and the only cover for it. The write pointer walks past the
        // memory in ordinary clocked RTL, with no literal OOB anywhere in the
        // source. `warn_run_range` is `Severity::Error`, so the VM run is
        // `ExitClass::HadErrors` and exit 1; before the arena reported it, the
        // native run was a clean PASS with byte-identical stdout. stdout cannot
        // see this — `agree`'s `exit_class` compare is what does.
        (
            "oob_array_access_reports_and_sets_exit_class",
            r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] wp, rd;
  reg clk;
  initial begin clk = 1'b0; wp = 8'd0; rd = 8'd0; end
  always @(posedge clk) begin mem[wp] = wp; rd = mem[wp]; wp = wp + 8'd1; end
  initial begin
    #1 clk = 1'b1; #1 clk = 1'b0; #1 clk = 1'b1; #1 clk = 1'b0;
    #1 clk = 1'b1; #1 clk = 1'b0; #1 clk = 1'b1; #1 clk = 1'b0;
    #1 clk = 1'b1; #1 clk = 1'b0; #1 clk = 1'b1; #1 clk = 1'b0;
    $display("wp=%0d rd=%0d", wp, rd);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // …and the same on the NBA path, which the per-body drain does not
        // cover (an NBA write happens outside any body).
        (
            "oob_array_nba_reports",
            r#"
module top;
  reg [7:0] mem [0:1];
  reg [7:0] i;
  initial begin i = 8'd0; mem[0] = 8'd0; i = 8'd7; mem[i] <= 8'd3; end
  initial begin #2 $display("mem0=%0d", mem[0]); $finish; end
endmodule
"#
            .to_string(),
        ),
        // A transport NBA that is the ONLY pending work at the deciding moment:
        // no `#` anywhere else, so the WHEEL IS EMPTY and the time advance would
        // report quiescence if it did not fold `delayed_nba` into its minimum.
        // The two designs above it both carry an `initial #N`, so the wheel is
        // non-empty for them and the named hazard was uncovered (review find).
        (
            "transport_nba_is_the_sole_pending_work",
            r#"
module top;
  reg [7:0] q, seen;
  initial begin q = 8'd0; seen = 8'd0; q <= #3 8'd42; end
  always @(q) begin seen = seen + 8'd1; $display("t=%0t q=%0d seen=%0d", $time, q, seen); $finish; end
endmodule
"#
            .to_string(),
        ),
    ]
}

#[test]
fn s1d4c2c_native_run_matches_the_vm_on_adversarial_shapes() {
    let designs = adversarial_designs();
    assert_eq!(designs.len(), 22, "adversarial set shrank");
    for (name, src) in designs {
        agree(&src, name).unwrap_or_else(|r| panic!("{name}: must be runnable, refused: {r}"));
    }
}

/// Every REFUSAL ROW must be reached by a design, and the design must be one the
/// walk would MIS-handle if the row were removed.
///
/// Written because the row-removal mutations survived: deleting the
/// `body_is_walkable` check left the whole suite green, since no design in it
/// carries an in-body waiter at all. A gate row with no design behind it is a
/// comment. Each case below panics (`unreachable!` in the walk) or produces a
/// wrong run if its row is deleted.
#[test]
fn s1d4c2c_each_refusal_row_has_a_design() {
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "in-body edge waiter",
            "an in-body `wait`/`@(…)` waiter, `fork` or subroutine call",
            r#"
module top;
  reg clk;
  reg [7:0] n;
  initial begin clk = 1'b0; n = 8'd0; #1 clk = 1'b1; #2 $finish; end
  always begin @(posedge clk); n = n + 8'd1; end
endmodule
"#,
        ),
        (
            "in-body wait(expr)",
            "an in-body `wait`/`@(…)` waiter, `fork` or subroutine call",
            r#"
module top;
  reg go;
  reg [7:0] n;
  initial begin go = 1'b0; n = 8'd0; #1 go = 1'b1; #2 $finish; end
  initial begin wait (go); n = 8'd5; end
endmodule
"#,
        ),
        (
            "continuous assign",
            "continuous assigns (S1d-4d settles them)",
            r#"
module top;
  wire [7:0] w;
  reg [7:0] r;
  assign w = r + 8'd1;
  initial begin r = 8'd1; #1 $display("w=%0d", w); $finish; end
endmodule
"#,
        ),
        (
            "final block",
            "`final` blocks (the post-loop drain is not restated)",
            r#"
module top;
  reg [7:0] n;
  initial begin n = 8'd3; #1 $finish; end
  final $display("final n=%0d", n);
endmodule
"#,
        ),
        (
            "refused system task",
            "a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)",
            r#"
module top;
  reg [7:0] n;
  initial begin n = 8'd0; $monitor("n=%0d", n); #1 n = 8'd1; #1 $finish; end
endmodule
"#,
        ),
    ];
    for (what, row, src) in &cases {
        let (ir, opts) = build_with_opts(src);
        assert_eq!(
            crate::native::run::runnable(&ir, &opts),
            Err(*row),
            "{what}: wrong refusal row"
        );
        // …and the design must be one the earlier layers ACCEPT, or the row is
        // not what is doing the work (a `class` design would be refused by S0
        // and prove nothing about this layer).
        let e = crate::native::design_eligibility(&ir, &opts);
        assert!(
            e.eligible && e.buildable,
            "{what}: refused by an EARLIER layer ({:?}), so this row is untested",
            e.refused
        );
    }
    assert_eq!(cases.len(), 5, "refusal-row coverage moved");
    // ⚠️ The `fork` and subroutine-CALL halves of the waiter row have no case,
    // and cannot: a `Terminator::Call` needs a non-empty `func_table`, which
    // `NetArena::build` refuses first (measured — the case was written and the
    // verdict came back "frame-local storage: S3"), and `fork_modes` non-empty
    // is an S0 reject. Both halves of that row are defence in depth against a
    // lost `.velab` trailer, not reachable paths.
}

/// An out-of-range NBA whose timestep has NOTHING after it. The per-statement
/// drain in the walk cannot report it — there is no next statement — so the
/// diagnostic (and with it `ExitClass::HadErrors`) exists only if the NBA apply
/// drains too. Measured: deleting that drain survived every other design here,
/// because they all happen to execute another statement afterwards.
#[test]
fn s1d4c2c_oob_nba_at_quiescence_still_reports() {
    let src = r#"
module top;
  reg [7:0] mem [0:1];
  reg [7:0] i;
  initial begin i = 8'd0; mem[0] = 8'd0; i = 8'd7; mem[i] <= 8'd3; end
endmodule
"#;
    agree(src, "oob_nba_at_quiescence").expect("runnable");
    // ANTI-VACUITY: the native run must actually EMIT the range diagnostic and
    // end quiescent — otherwise `agree` above compared two clean runs.
    // Asserted on the diagnostic itself, NOT on `exit_class`: `warn_run_range`
    // does not set `st.had_error` (see `MergedSink`'s doc), so an `exit_class`
    // assertion here reads `Ok` and proves nothing.
    let (ir, opts) = build_with_opts(src);
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    assert_eq!(r.backend, Backend::Native);
    assert_eq!(r.finish_reason, FinishReason::Quiescent);
    let events = sink.events.into_inner();
    assert!(
        events
            .iter()
            .any(|e| e.starts_with("diag|Error|VITA-E4002")),
        "the OOB NBA must report E4002 — nothing else in this design drains \
         `pending_range`: {events:?}"
    );
}

/// An out-of-range read in a TERMINATOR expression, with nothing after it.
///
/// The walk drains at every STATEMENT boundary, but a `Branch` condition (and a
/// `#(mem[i])` delay amount) is evaluated after the last statement of its block.
/// If the process then returns and the run goes quiescent, the only thing that
/// can still report it is the drain the run loop does after `run_body`. Measured:
/// deleting that drain survived every other design here.
#[test]
fn s1d4c2c_oob_in_a_terminator_condition_still_reports() {
    let src = r#"
module top;
  reg [7:0] mem [0:1];
  reg [7:0] i;
  initial begin
    mem[0] = 8'd0;
    i = 8'd9;
    if (mem[i] == 8'd0) mem[0] = 8'd1;
  end
endmodule
"#;
    agree(src, "oob_in_terminator").expect("runnable");
    let (ir, opts) = build_with_opts(src);
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    assert_eq!(r.backend, Backend::Native);
    let events = sink.events.into_inner();
    assert!(
        events
            .iter()
            .any(|e| e.starts_with("diag|Error|VITA-E4002")),
        "the condition's OOB read must report — no statement follows it: {events:?}"
    );
}

/// The DELTA budget is not the BODY budget, and the two names are one letter
/// apart (`k_delta_budget` vs `k_max_deltas`, whose name means the opposite of
/// what it returns — round-25's conflation, written down).
///
/// Swapping them survived the whole gate, and the reason is not subtle: the
/// defaults are `max_deltas = 1_000_000` and `max_body_steps = 100_000_000`, so
/// an oscillator delta-limits either way and only takes 100x longer to say so.
/// Distinguishing them needs budgets that are SMALL and DIFFERENT, with the
/// design's real delta count between them — then one budget completes the run
/// and the other cuts it off.
#[test]
fn s1d4c2c_delta_budget_is_not_the_body_budget() {
    // ~6 deltas of real work: `a` flips 3 times, each flip waking the other
    // process. Between `max_body_steps` (4) and `max_deltas` (64).
    let src = r#"
module top;
  reg [7:0] a, b;
  initial begin a = 8'd0; b = 8'd0; end
  always @(a) begin if (a < 8'd3) b = a + 8'd1; end
  always @(b) begin if (b < 8'd3) a = b; end
  initial begin #1 $display("a=%0d b=%0d", a, b); $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    let mk = |backend| SimOpts {
        backend,
        max_deltas: 64,
        max_body_steps: 4,
        ..opts.clone()
    };
    let (r_vm, out_vm) = simulate_capture(&ir, mk(Backend::Bytecode));
    let (r_nat, out_nat) = simulate_capture(&ir, mk(Backend::Native));
    assert_eq!(r_nat.backend, Backend::Native, "fell back");
    assert_eq!(out_vm, out_nat, "stdout differs");
    assert_eq!(r_vm.finish_reason, r_nat.finish_reason);
    assert_eq!(r_vm.exit_class, r_nat.exit_class);
    // ANTI-VACUITY: the run must actually FINISH. If it delta-limited, both
    // backends would agree on the failure and the budgets would be untested.
    assert_eq!(
        r_nat.finish_reason,
        FinishReason::Finish,
        "the design must complete under the delta budget, or reading the WRONG \
         budget would be indistinguishable: {out_nat}"
    );
}

/// TIME LIMIT: `SimOpts::time_limit` stops the advance, and the two backends
/// must stop at the same tick with the same reason. Separate from `agree`
/// because it is the one comparison that needs a non-default option.
#[test]
fn s1d4c2c_time_limit_agrees() {
    let src = r#"
module top;
  reg [7:0] n;
  initial n = 8'd0;
  always begin #1 n = n + 8'd1; $display("t=%0t n=%0d", $time, n); end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    let mk = |b| SimOpts {
        backend: b,
        time_limit: Some(5),
        ..opts.clone()
    };
    let (r_vm, out_vm) = simulate_capture(&ir, mk(Backend::Bytecode));
    let (r_nat, out_nat) = simulate_capture(&ir, mk(Backend::Native));
    assert_eq!(r_nat.backend, Backend::Native, "fell back");
    assert_eq!(out_vm, out_nat);
    assert_eq!(r_vm.finish_reason, r_nat.finish_reason);
    assert_eq!(r_nat.finish_reason, FinishReason::Quiescent);
    assert_eq!(r_vm.sim_time, r_nat.sim_time);
}
