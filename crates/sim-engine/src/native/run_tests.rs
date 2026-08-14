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
pub(super) fn agree(src: &str, name: &str) -> Result<(), &'static str> {
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts)?;

    // Per-design VCD targets. A design with no `$dumpvars` writes neither file,
    // which the comparison below treats as agreement — the `$dump*` designs are
    // what make it bite, and since S1d-4d-2 the corpus supplies 44 of them.
    // A per-CALL counter, not just the pid: `cargo test` runs test functions on
    // parallel threads, and two of them walk the same corpus — so a tag built
    // from the design name alone collides, one test removing the file the other
    // is about to read. That reads as "the VM wrote no VCD", which is exactly
    // how it presented.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir();
    let tag = format!(
        "vita_s1d4d2_{}_{}_{}",
        name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>(),
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let vcd_vm = dir.join(format!("{tag}_vm.vcd"));
    let vcd_nat = dir.join(format!("{tag}_nat.vcd"));
    let _ = std::fs::remove_file(&vcd_vm);
    let _ = std::fs::remove_file(&vcd_nat);

    let vm = SimOpts {
        backend: Backend::Bytecode,
        vcd_path_override: Some(vcd_vm.to_string_lossy().into_owned()),
        ..opts.clone()
    };
    let nat = SimOpts {
        backend: Backend::Native,
        vcd_path_override: Some(vcd_nat.to_string_lossy().into_owned()),
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
    // THE WAVEFORM. This is the half of the original S1 gate that could not be
    // asserted until S1d-4d-2 — `$dumpvars` was a refused system task, so the
    // corpus had to be stripped of its dump calls to be compared at all.
    let b_vm = std::fs::read(&vcd_vm).ok();
    let b_nat = std::fs::read(&vcd_nat).ok();
    // ANTI-VACUITY: `None == None` passes both assertions below, so a design
    // that dumps must be shown to have PRODUCED a file. Measured — making
    // `dumpvars_with` a total no-op on both backends left the whole suite green
    // before this line, which turned "37 waveforms match" into 37 matches of
    // nothing against nothing.
    if src.contains("$dumpvars") {
        assert!(
            b_nat.is_some() && b_vm.is_some(),
            "{name}: the design dumps but a run produced no VCD"
        );
    }
    assert_eq!(
        b_vm.as_ref().map(|b| b.len()),
        b_nat.as_ref().map(|b| b.len()),
        "{name}: VCD length differs (or one side wrote no file)"
    );
    assert_eq!(b_vm, b_nat, "{name}: VCD bytes differ");
    let _ = std::fs::remove_file(&vcd_vm);
    let _ = std::fs::remove_file(&vcd_nat);
    Ok(())
}

/// The P6 corpus, AS GENERATED — dump calls and all.
///
/// It used to be stripped of `$dumpfile`/`$dumpvars`, because those were refused
/// system tasks and 44 of the 72 designs carry them: without stripping, 44
/// comparisons were 44 refusals. S1d-4d-2 wired both, so the corpus is now
/// compared in the form it is actually generated in — and `agree` compares the
/// VCD bytes those calls produce.
fn corpus_designs() -> Vec<(String, String)> {
    corpus(0x5EED_F00D, 72)
        .into_iter()
        .map(|d| (d.name, d.src))
        .collect()
}

/// The gate: the corpus's runnable subset, end to end, on both backends.
#[test]
fn s1d4c2c_native_run_matches_the_vm_over_corpus() {
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in corpus_designs() {
        match agree(&src, &name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    // EXACT, not a floor. The refusal breakdown is asserted too, so a row that
    // starts firing on designs it never used to — the way a widened predicate
    // silently shrinks a gate — moves a number here instead of passing.
    // THE WHOLE CORPUS. It was 30 designs when the body walk landed, 65 when the
    // zero-delay settle did, and 72 — every one — since the delayed
    // cont-assign wheel. Exact, not a floor: a row that starts firing on designs
    // it never used to moves a number here instead of passing.
    assert_eq!(
        (ran, refused.len()),
        (72, 0),
        "corpus coverage moved — re-pin deliberately. ran={ran} refused={refused:?}"
    );
}

/// The corpus's `$dumpvars` designs run NATIVELY and their waveforms match.
///
/// This test used to assert the opposite — that a dump design is refused — and
/// the assertion was correct until S1d-4d-2 wired `$dumpfile`/`$dumpvars`. It is
/// kept, inverted, because the population it counts (44 of 72) is the measure of
/// how much of the corpus the VCD half of the gate covers.
#[test]
fn s1d4d2_vcd_designs_run_and_their_waveforms_match() {
    let mut with_dump = 0usize;
    let mut ran = 0usize;
    for (name, src) in corpus_designs() {
        if !src.contains("$dumpvars") {
            continue;
        }
        with_dump += 1;
        // `agree` compares stdout, diagnostics, finish reason, time, exit class
        // AND the VCD bytes; a refusal is counted rather than silently passed.
        if agree(&src, &name).is_ok() {
            ran += 1;
        }
    }
    assert_eq!(
        (with_dump, ran),
        (44, 44),
        "corpus VCD population moved — re-pin deliberately (the 7 delayed-CA \
         hold-outs joined when S1d-4d-3 landed)"
    );
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
        // IN-BODY WAITERS (S1d-4c-2d) — three causes, and the corpus contains
        // none of them (measured: its 138 suspending terminators are all
        // `Delay`). Each design makes the waiter fire more than once, so a
        // model that armed but never re-armed would show a smaller count.
        (
            "in_body_edge_wait",
            r#"
module top;
  reg clk;
  reg [7:0] n;
  initial begin clk = 1'b0; n = 8'd0; #1 clk = 1'b1; #1 clk = 1'b0; #1 clk = 1'b1; #2 $display("n=%0d", n); $finish; end
  always begin @(posedge clk); n = n + 8'd1; end
endmodule
"#
            .to_string(),
        ),
        // `@(sig)` fires on a CHANGE from the arm-time value, so the middle
        // write (same value) must not wake it — an implementation keyed on
        // "was this net dirty" would count three instead of two.
        (
            "in_body_level_wait_ignores_same_value",
            r#"
module top;
  reg [7:0] a;
  reg [7:0] n;
  initial begin a = 8'd0; n = 8'd0; #1 a = 8'd5; #1 a = 8'd5; #1 a = 8'd7; #2 $display("n=%0d", n); $finish; end
  always begin @(a); n = n + 8'd1; end
endmodule
"#
            .to_string(),
        ),
        // `wait(e)` with `e` FALSE parks and resumes on the transition …
        (
            "in_body_wait_expr_blocks_then_resumes",
            r#"
module top;
  reg go;
  reg [7:0] n;
  initial begin go = 1'b0; n = 8'd0; #2 go = 1'b1; #2 $display("n=%0d", n); $finish; end
  initial begin wait (go); n = 8'd9; end
endmodule
"#
            .to_string(),
        ),
        // … and with `e` ALREADY TRUE it does not park at all — the
        // short-circuit in the walk. Without it this design hangs rather than
        // printing, which the time limit would turn into a different answer.
        (
            "in_body_wait_expr_already_true_falls_through",
            r#"
module top;
  reg go;
  reg [7:0] n;
  initial begin go = 1'b1; n = 8'd0; end
  initial begin #1 wait (go); n = 8'd4; $display("n=%0d", n); $finish; end
endmodule
"#
            .to_string(),
        ),
        // A `@(sig)` body that writes the watched net after waking, then loops
        // back to the wait. The re-arm snapshots the value the body just wrote,
        // so the wait does not re-fire on it.
        //
        // ⚠️ NOT a cover for the `Level` author guard, despite what an earlier
        // name and comment here claimed: by the time the body re-arms, its own
        // write is already IN the snapshot, so `cur != arm` is false with or
        // without the guard. That guard is unreachable on this arm (see
        // `fire_waiters`); the `Edge` one is covered separately and really is
        // reachable.
        (
            "in_body_level_wait_rearms_after_its_own_write",
            r#"
module top;
  reg [7:0] a, n;
  initial begin a = 8'd0; n = 8'd0; #1 a = 8'd1; #3 $display("n=%0d a=%0d", n, a); $finish; end
  always begin @(a); n = n + 8'd1; a = a + 8'd16; end
endmodule
"#
            .to_string(),
        ),
        // An in-body EDGE waiter and a STATIC `always @(posedge clk)` woken by
        // the same edge: the two live in different tables, and the order they
        // reach the Active queue is process order, not table order.
        (
            "static_and_in_body_edge_share_a_delta",
            r#"
module top;
  reg clk;
  reg [7:0] s, w;
  initial begin clk = 1'b0; s = 8'd0; w = 8'd0; #1 clk = 1'b1; #2 $display("s=%0d w=%0d", s, w); $finish; end
  always @(posedge clk) begin s = s + 8'd1; $display("static"); end
  always begin @(posedge clk); w = w + 8'd1; $display("inbody"); end
endmodule
"#
            .to_string(),
        ),
        // SELF-EDGE: a body that CREATES an edge and then waits for that same
        // edge must not be woken by its own write. The mask is intra-slot, so
        // without the author guard the waiter reads the edge it just caused and
        // resumes immediately — a free-running loop instead of a parked process.
        //
        // This is the ONLY reachable cover for that guard on the `Edge` arm: a
        // process cannot write while suspended, so the write has to happen in
        // the same slot BEFORE the wait arms.
        (
            "in_body_edge_wait_ignores_its_own_edge",
            r#"
module top;
  reg clk;
  reg [7:0] n;
  initial begin clk = 1'b0; n = 8'd0; end
  always begin
    clk = ~clk;
    @(posedge clk);
    n = n + 8'd1;
  end
  initial begin #5 $display("n=%0d clk=%0b", n, clk); $finish; end
endmodule
"#
            .to_string(),
        ),
        // ORDER: an in-body waiter whose process id is BELOW a statically
        // sensitive one. The existing pair has the in-body process second, so a
        // plain `push` and a sorted insert produce the same queue; this one
        // separates them.
        (
            "in_body_waiter_sorts_before_a_static_process",
            r#"
module top;
  reg clk;
  reg [7:0] w, s;
  initial begin w = 8'd0; s = 8'd0; end
  always begin @(posedge clk); w = w + 8'd1; $display("inbody"); end
  always @(posedge clk) begin s = s + 8'd1; $display("static"); end
  initial begin clk = 1'b0; #1 clk = 1'b1; #2 $display("w=%0d s=%0d", w, s); $finish; end
endmodule
"#
            .to_string(),
        ),
        // The arm snapshot must cover the UNK plane, not just the value plane:
        // `a` starts X and is written 0, which moves only unk.
        (
            "in_body_level_wait_sees_an_unk_only_change",
            r#"
module top;
  reg [7:0] a;
  reg [7:0] n;
  initial begin n = 8'd0; #1 a = 8'h00; #2 $display("n=%0d", n); $finish; end
  always begin @(a); n = n + 8'd1; end
endmodule
"#
            .to_string(),
        ),
        // …and every ELEMENT, not just element 0: `@(mem)` with a write to
        // `mem[3]`. A snapshot of element 0 alone would never see it.
        (
            "in_body_level_wait_watches_a_whole_array",
            r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] n;
  integer i;
  initial begin n = 8'd0; for (i = 0; i < 4; i = i + 1) mem[i] = 8'd0; #1 mem[3] = 8'd9; #2 $display("n=%0d", n); $finish; end
  always begin @(mem); n = n + 8'd1; end
endmodule
"#
            .to_string(),
        ),
        // …and every WATCHED NET of a multi-net `@(a or b)`: only the SECOND
        // one moves, so a test of `nets[0]` alone never fires.
        (
            "in_body_level_wait_checks_every_watched_net",
            r#"
module top;
  reg [7:0] a, b, n;
  initial begin a = 8'd0; b = 8'd0; n = 8'd0; #1 b = 8'd3; #2 $display("n=%0d", n); $finish; end
  always begin @(a or b); n = n + 8'd1; end
endmodule
"#
            .to_string(),
        ),
        // An out-of-range read from a WAIT PREDICATE. `fire_waiters` evaluates
        // it, which makes that pass a third producer of deferred range reports —
        // and it sits after both of the drains the previous slice added, so
        // without a drain at the end of `propagate` the diagnostic (and the
        // exit class it sets) is lost entirely. Measured: exit 0 vs exit 1.
        (
            "oob_read_in_a_wait_predicate_reports",
            r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] idx;
  initial begin idx = 8'd0; #1 idx = 8'd9; end
  initial begin wait (mem[idx] == 8'hAA); $display("never"); end
endmodule
"#
            .to_string(),
        ),
        // WIDE dumped net (>64 bits). The corpus's wide template
        // (`gen_wide_arith`) carries no `$dumpvars`, so the widest net the gate
        // ever dumped was 32 bits — and a `packed_of`/`bits_of` that kept only
        // the first word survived it.
        (
            "vcd_wide_net_records_every_word",
            r#"
module top;
  reg [199:0] w;
  initial begin
    $dumpfile("w.vcd"); $dumpvars(0, top);
    w = 200'd0;
    #1 w = {8'hA5, 96'hDEADBEEF_CAFEBABE_12345678, 96'h0F0F0F0F_F0F0F0F0_5A5A5A5A};
    #1 w[199:128] = 72'h7F_FEDCBA98_76543210;
    #1 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A MID-RUN x/z write on a dumped net. t0 X is covered (every net starts
        // X), but nothing in the gate turned a KNOWN net back into X later —
        // which is what a `packed_of` that zeroed the unk plane needed.
        (
            "vcd_mid_run_x_and_z_are_recorded",
            r#"
module top;
  reg [7:0] a;
  initial begin
    $dumpfile("xz.vcd"); $dumpvars(0, top);
    a = 8'h3C;
    #1 a = 8'bxxxx_0101;
    #1 a = 8'bzz11_zz00;
    #1 a = 8'h00;
    #1 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // TWO writes to one net between two drains. The glitch template writes
        // three times but in three STATEMENTS, and the walk drains at every
        // statement boundary — so it cannot tell capture-at-the-store-point from
        // re-read-at-drain, which is the whole reason the buffer holds values.
        // Two NBAs to the same net land in ONE `apply_nba` with one drain after.
        (
            "vcd_two_writes_between_drains_keep_both_values",
            r#"
module top;
  reg clk;
  reg [7:0] x;
  initial begin
    $dumpfile("t.vcd"); $dumpvars(0, top);
    clk = 1'b0; x = 8'd0;
    #1 clk = 1'b1;
    #2 $finish;
  end
  always @(posedge clk) begin
    x <= 8'd11;
    x <= 8'd22;
  end
endmodule
"#
            .to_string(),
        ),
        // `$dumpoff` — no coverage anywhere, and the `dumping` guard is what
        // stops records (and bare `#N` time stamps) after it.
        (
            "vcd_dumpoff_stops_the_records",
            r#"
module top;
  reg [7:0] a;
  initial begin
    $dumpfile("o.vcd"); $dumpvars(0, top);
    a = 8'd1;
    #1 a = 8'd2;
    #1 $dumpoff;
    #1 a = 8'd3;
    #1 a = 8'd4;
    #1 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // DELAYED CONT-ASSIGN with a DYNAMIC LHS index (S1d-4d-3). The corpus's
        // delayed assigns all have whole-net or constant destinations, so the
        // one place the LHS offsets are resolved had no cover — and it read the
        // ENGINE's store while the RHS beside it read the arena. On a native run
        // that store never moves, so the index came back X, became the
        // out-of-range sentinel and the write was DROPPED. Identical exit code,
        // identical everything, one bit missing.
        (
            "delayed_cont_assign_with_a_dynamic_bit_index",
            r#"
module top;
  wire [7:0] y;
  reg [2:0] i;
  reg v;
  assign #1 y[i] = v;
  initial begin
    i = 3'd3; v = 1'b1;
    #3 i = 3'd6;
    #3 $display("y=%b", y);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // …and the indexed part-select spelling of the same destination, which
        // resolves a WIDTH as well as an offset.
        (
            "delayed_cont_assign_with_an_indexed_part_select",
            r#"
module top;
  wire [15:0] y;
  reg [3:0] i;
  reg [3:0] v;
  assign #1 y[i*4 +: 4] = v;
  initial begin
    i = 4'd1; v = 4'hF;
    #3 i = 4'd2; v = 4'hA;
    #3 $display("y=%h", y);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A design whose ONLY pending work is a delayed cont-assign: the time
        // advance has to fold `delayed_ca` into its minimum or the run is called
        // quiescent and the write is dropped.
        (
            "delayed_cont_assign_is_the_sole_pending_work",
            r#"
module top;
  wire [7:0] o;
  reg [7:0] a;
  reg [7:0] seen;
  assign #5 o = a + 8'd1;
  initial begin a = 8'd7; seen = 8'd0; end
  always @(o) begin seen = seen + 8'd1; $display("t=%0t o=%0d seen=%0d", $time, o, seen); end
endmodule
"#
            .to_string(),
        ),
        // VCD (S1d-4d-2). The corpus's 44 dump designs are all SCALAR and all
        // call `$dumpvars` before their first write, so two things the emitter
        // must get right have no cover there.
        //
        // An ARRAY under `$dumpvars`, TWO covers in one design. (a) One VCD id
        // per ELEMENT, so a record built from word 0 names the wrong variable —
        // `mem[2]` is written on its own at t=1. (b) The elements are filled
        // BEFORE `$dumpvars`, so the t0 snapshot's ARRAY branch has to read the
        // arena; taking it from the engine store dumps x for every element and
        // every later record still looks right.
        (
            "vcd_array_records_name_the_written_element",
            r#"
module top;
  reg [7:0] mem [0:3];
  integer i;
  initial begin
    for (i = 0; i < 4; i = i + 1) mem[i] = i + 8'd1;
    $dumpfile("m.vcd"); $dumpvars(0, top);
    #1 mem[2] = 8'h55;
    #1 mem[0] = 8'hAA;
    #1 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // `$dumpvars` called AFTER the first writes: the t0 snapshot has to read
        // the ARENA. Taking it from the engine store would dump x for every net
        // — and every LATER record would still be right, so only the header's
        // initial values differ.
        (
            "vcd_initial_snapshot_reads_the_live_store",
            r#"
module top;
  reg [7:0] a, b;
  initial begin
    a = 8'h5A; b = 8'hA5;
    $dumpfile("l.vcd"); $dumpvars(0, top);
    #1 a = 8'h11;
    #1 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // CONTINUOUS ASSIGNS (S1d-4d-1). The corpus contributed ZERO coverage
        // when these were written — its cont-assign designs all pair a plain
        // assign with a delayed one, and delayed was still refused — and a
        // `panic!` at the top of `settle_cont_assigns` left the whole workspace
        // green before these existed. S1d-4d-3 admitted the delayed form, so
        // those 7 designs now run; what they buy is still only ONE shape
        // (whole-net lhs, bare-signal rhs, written once at t0), which is why
        // every mutation below needed its own design anyway. Each is named for
        // the mutation it kills.
        //
        // The t0 settle, and its changed set surviving `arm_t0`. `w` is
        // established once, at t0, and never moves again: if the settle does not
        // run, or `arm_t0` drops its dirt, `always @(w)` never fires and the run
        // is silently short one line at exit 0.
        (
            "t0_settle_change_reaches_a_level_process",
            r#"
module top;
  wire w;
  assign w = 1'b1;
  always @(w) $display("saw w=%b at %0t", w, $time);
  initial begin #10 $display("done"); $finish; end
endmodule
"#
            .to_string(),
        ),
        // A PORT-BOUND CLOCK: the settle drives a child's `clk` through a
        // continuous assign, and the edge has to reach the child's static
        // sensitivity — which only happens if a settle that moved a net runs
        // change propagation.
        (
            "settle_drives_a_port_bound_clock",
            r#"
module child(input c, output reg [7:0] n);
  initial n = 8'd0;
  always @(posedge c) n = n + 8'd1;
endmodule
module top;
  reg drv;
  wire gated;
  wire [7:0] cnt;
  assign gated = drv;
  child u(.c(gated), .n(cnt));
  initial begin drv = 1'b0; #1 drv = 1'b1; #1 drv = 1'b0; #1 drv = 1'b1; #2 $display("cnt=%0d", cnt); $finish; end
endmodule
"#
            .to_string(),
        ),
        // An IMPURE rhs (`$random`) is one `levelize::ca_deps` refuses to
        // certify, so it lives in `ca_always` and must be visited every pass
        // whatever the dirty worklist says. Dropping `ca_always` freezes it —
        // and the rng stream makes the visit COUNT observable, not just the
        // final value.
        (
            "impure_cont_assign_is_visited_every_pass",
            r#"
module top;
  reg [7:0] a;
  wire [31:0] r;
  reg [7:0] seen;
  assign r = $random;
  initial begin a = 8'd0; seen = 8'd0; #1 a = 8'd1; #1 a = 8'd2; #1 $display("seen=%0d r=%0d", seen, r); $finish; end
  always @(r) seen = seen + 8'd1;
endmodule
"#
            .to_string(),
        ),
        // A cont-assign RHS that reads OUT OF RANGE with a MOVING index. Every
        // re-evaluation emits another `E4002`, so the visit set is observable
        // in the diagnostic stream — this is what proved that dropping the
        // worklist and visiting all assigns each pass is NOT byte-identical.
        (
            "cont_assign_oob_read_counts_visits",
            r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] idx;
  wire [7:0] o;
  assign o = mem[idx];
  initial begin
    mem[0] = 8'd1; idx = 8'd0;
    #1 idx = 8'd9;
    #1 idx = 8'd10;
    #1 $display("o=%0d", o);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A settle-produced change racing an Active-queue write: the settle runs
        // at the TOP of the delta, before bodies drain, so `b` is already the
        // settled value when the process reads it.
        (
            "settle_runs_before_the_active_batch",
            r#"
module top;
  reg [7:0] a;
  wire [7:0] b;
  reg [7:0] saw;
  assign b = a + 8'd1;
  initial begin a = 8'd0; saw = 8'd0; end
  always @(a) saw = b;
  initial begin #1 a = 8'd5; #1 $display("saw=%0d b=%0d", saw, b); $finish; end
endmodule
"#
            .to_string(),
        ),
        // The t0 settle must run BEFORE the initializers, because a declaration
        // initializer can READ a continuously-assigned net. Without it `x`
        // samples X instead of 5 — and the in-loop settle is too late, it runs
        // after `arm_t0`.
        (
            "declaration_initializer_reads_a_settled_net",
            r#"
module top;
  wire [7:0] w;
  assign w = 8'd5;
  reg [7:0] x = w;
  initial begin #1 $display("x=%0d", x); $finish; end
endmodule
"#
            .to_string(),
        ),
        // An out-of-range read from a cont-assign RHS in a design with NO
        // process at all: the settle's own drain is the only thing that can
        // report it, since no body runs and no formatter is ever called.
        (
            "cont_assign_oob_with_no_process_at_all",
            r#"
module top;
  reg [7:0] mem [0:1];
  wire [7:0] o;
  assign o = mem[9];
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
        // MULTI-DRIVER (S1d-4d-4). The corpus has ZERO multi-driven or wired
        // nets (measured), so every group-resolution behaviour needs its own
        // design. The fold itself is shared code guarded by the oracle anchors
        // below; these exercise the BACKEND halves — the per-driver skip, the
        // group write, and everything downstream of it.
        //
        // Drivers move at DIFFERENT times, so a resolution that runs only at t0
        // (or only when a group member is dirty vs. every pass) diverges. The
        // tri-state enable idiom is the common real-RTL shape.
        (
            "md_tristate_drivers_move_over_time",
            r#"
module top;
  wire y;
  reg en_a, en_b, va, vb;
  assign y = en_a ? va : 1'bz;
  assign y = en_b ? vb : 1'bz;
  always @(y) $display("t=%0t y=%b", $time, y);
  initial begin
    en_a = 0; en_b = 0; va = 1; vb = 0;
    #2 en_a = 1;
    #2 en_a = 0; en_b = 1;
    #2 en_b = 0;
    #2 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The resolved net FEEDS another cont-assign — the group resolution
        // must be part of the same fixpoint, or `n` lags a delta behind `m`.
        (
            "md_net_feeds_a_downstream_assign",
            r#"
module top;
  wire [1:0] m;
  wire [1:0] n;
  reg [1:0] a, b;
  assign m = a;
  assign m = b;
  assign n = m ^ 2'b11;
  initial begin
    a = 2'b1z; b = 2'bz0;
    #1 $display("m=%b n=%b", m, n);
    a = 2'b0z;
    #1 $display("m=%b n=%b", m, n);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A group driver whose RHS reads OUT OF RANGE: the drivers are
        // re-evaluated EVERY settle pass (the engine never worklists the group
        // loop), so the E4002 COUNT is a direct probe of pass-for-pass
        // equality — a worklisted or once-per-timestep resolution changes the
        // diagnostic stream even where values agree.
        (
            "md_driver_rhs_reads_out_of_range",
            r#"
module top;
  wire [7:0] y;
  reg [7:0] mem [0:1];
  reg [7:0] i, b;
  assign y = mem[i];
  assign y = b;
  initial begin
    mem[0] = 8'hzz; i = 8'd7; b = 8'hzz;
    #1 $display("y=%h", y);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // An edge on a RESOLVED net must reach an in-body waiter: the group
        // write lands through the same funnel (dirty + edge accumulation), or
        // the `@(posedge y)` never fires even though `y`'s value is right.
        (
            "md_net_wakes_an_in_body_waiter",
            r#"
module top;
  wire y;
  reg a, b;
  assign y = a;
  assign y = b;
  initial begin a = 1'bz; b = 1'b0; #2 a = 1'b1; b = 1'bz; #5 $finish; end
  initial begin
    @(posedge y) $display("edge at t=%0t", $time);
  end
endmodule
"#
            .to_string(),
        ),
        // MULTI-WORD (96-bit): the all-Z identity and `mask_top` must hold per
        // word, not just in word 0.
        (
            "md_wide_net_resolves_per_word",
            r#"
module top;
  wire [95:0] y;
  reg [95:0] a, b;
  assign y = a;
  assign y = b;
  initial begin
    a = {32'hFFFF0000, 32'hz, 32'h12345678};
    b = {32'hFFFFzzzz, 32'h0, 32'h12345678};
    #1 $display("y=%h", y);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // THREE drivers of one net plus a `wand` and `wor` under `$dumpvars`:
        // the resolved value must land in the VCD from the store point like any
        // other write (byte-compared by `agree`).
        (
            "md_and_wired_nets_under_dumpvars",
            r#"
module top;
  wire [1:0] y;
  wand w;
  wor o;
  reg [1:0] a, b, c;
  reg p, q;
  assign y = a;
  assign y = b;
  assign y = c;
  assign w = p;
  assign w = q;
  assign o = p;
  assign o = q;
  initial begin
    $dumpfile("md.vcd"); $dumpvars(0, top);
    a = 2'b1z; b = 2'bz0; c = 2'b1z; p = 1'b1; q = 1'b0;
    #1 $display("y=%b w=%b o=%b", y, w, o);
    p = 1'b0;
    #1 $display("y=%b w=%b o=%b", y, w, o);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // S2 CONST-INDEX ADMISSION BOUNDARY. A constant array index is
        // admitted only when it is in bounds and 2-state; both halves of that
        // guard were unfalsifiable until this design (the differential review
        // measured that an off-by-one in the bound, and dropping the x-plane
        // check, survived the whole suite). Admitting either reads the NEXT
        // net's storage — a wrong value — and drops the E4002 the generic path
        // emits, so the merged stream comparison is what has the teeth.
        (
            "s2_const_index_out_of_bounds_and_xz_decline",
            r#"
module top;
  reg [3:0] mem [0:3];
  reg [3:0] y, z, w;
  initial begin
    mem[0] = 4'd1; mem[1] = 4'd2; mem[2] = 4'd3; mem[3] = 4'd4;
    y = mem[3];
    z = mem[4];
    w = mem[1'bx];
    $display("y=%b z=%b w=%b", y, z, w);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // S2's k>=w SHIFT ARM, both hazards the soundness review measured.
        // (a) A dynamic OOB index UNDER a k>=w shift: the first spelling
        // admitted the tree without visiting the lhs, so the E4002 the generic
        // path emits (and the exit class it sets) vanished — loud to silent.
        (
            "s2_kgew_shift_keeps_the_oob_diagnostic",
            r#"
module top;
  reg [3:0] mem [0:3];
  reg [3:0] i, y;
  initial begin
    mem[0] = 4'd1; i = 4'd5;
    y = mem[i] >> 4;
    $display("y=%b", y);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // (b) An impure operand under a k>=w shift: the generic path draws
        // from the RNG for the shifted-out operand (operands are evaluated),
        // so skipping it shifted the whole subsequent `$urandom` stream — a
        // value divergence at exit 0. Declining the tree (SysFunc never
        // admits, and the lhs is now always visited) keeps one stream.
        (
            "s2_kgew_shift_keeps_the_rng_draw",
            r#"
module top;
  reg [31:0] y, z;
  initial begin
    y = $urandom >> 32;
    z = $urandom;
    $display("y=%h z=%h", y, z);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // TWO IMPURE DRIVERS in one group: the fold is commutative, so the
        // driver evaluation ORDER is invisible to every value-only design —
        // reversing the group loop's iteration survived the whole suite
        // (soundness-review mutation). `$random` is admitted (pure-eval side
        // of the RNG, no seed write-back), and with two draws in one group the
        // order the drivers are evaluated IS the order the draws come out: the
        // backends agree only if both walk ascending ci.
        (
            "md_two_impure_drivers_pin_eval_order",
            r#"
module top;
  wire [3:0] y;
  assign y = $random;
  assign y = ~$random;
  initial begin #1 $display("t=%0t y=%b", $time, y); $finish; end
endmodule
"#
            .to_string(),
        ),
        // S2 one-bit ops — the EAGER-EVALUATION property their admission rests
        // on. `&&`/`||` are compiled to a `WProg` that evaluates BOTH operands,
        // which is only correct because the generic evaluator does not
        // short-circuit either. The observable is not a value: an admitted
        // subtree can COUNT an out-of-range element read, so a short-circuit
        // that skipped the rhs would drop an E4002 and flip the exit class from
        // 1 to 0 — loud to SILENT — while every value in the design stayed
        // right. (Measured: a constant-lhs short-circuit in the compile arm
        // survives the whole workspace suite without this design.)
        (
            "s2_logical_ops_evaluate_both_operands",
            r#"
module top;
  reg [7:0] m [0:3];
  integer oob;
  reg c1, c2, c3;
  initial begin
    m[0] = 8'd0; m[1] = 8'd1; m[2] = 8'd2; m[3] = 8'd3;
    oob = 9;
    c1 = m[0] && m[oob];   // lhs definitely FALSE  — rhs must still be read
    c2 = m[1] || m[oob];   // lhs definitely TRUE   — rhs must still be read
    c3 = 1'b0 && m[oob];   // constant-false lhs    — rhs must still be read
    $display("c=%b%b%b", c1, c2, c3);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // S2 one-bit ops at widths the width-4 battery cannot reach: a 1-bit
        // operand that is a NET (not a comparison result), 63/64-bit operands,
        // and a `&&` whose two operands differ in width in BOTH directions.
        (
            "s2_one_bit_ops_at_width_boundaries",
            r#"
module top;
  reg [63:0] w64;
  reg [62:0] w63;
  reg        b1;
  reg        r1, r2, r3, r4, r5, r6;
  initial begin
    w64 = 64'hDEAD_BEEF_0000_0000; w63 = 63'h1; b1 = 1'b0;
    r1 = b1 && w64;        // lw=1  < rw=64
    r2 = w64 && b1;        // lw=64 > rw=1
    r3 = w63 && w64;       // 63 vs 64
    r4 = |w64;
    r5 = &w63;
    r6 = (w64 === w64);
    $display("r=%b%b%b%b%b%b", r1, r2, r3, r4, r5, r6);
    w64 = 64'bx; b1 = 1'bx;
    $display("x=%b%b%b", b1 && w64, |w64, ^w64);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 2a: dynamic arrays (ROADMAP §5.1-c) ───────────────────
        // A DynArray net gets a DEAD slot in the arena and its elements live in
        // `SimState::dyn_heap`; `agree` proves both halves, since it asserts the
        // design is `runnable` before comparing — a row that silently started
        // falling back would fail loudly rather than compare the VM to itself.
        (
            "dyn_array_alloc_write_read_size",
            r#"
module top;
  int q[];
  integer i;
  initial begin
    q = new[4];
    for (i = 0; i < 4; i = i + 1) q[i] = i * 3;
    for (i = 0; i < 4; i = i + 1) $display("q[%0d]=%0d", i, q[i]);
    $display("size=%0d", q.size());
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The OUT-OF-RANGE read is the row that discriminates the two stores:
        // the arena counts a bad index and the heap warns once, so a design that
        // reached the dead slot would differ in the diagnostic stream and the
        // exit class even where the printed value agreed on `x`.
        (
            "dyn_array_out_of_range_and_empty",
            r#"
module top;
  int q[];
  int e[];
  initial begin
    q = new[2];
    q[0] = 5; q[1] = 6;
    $display("oob=%0d empty=%0d esize=%0d", q[9], e[0], e.size());
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A RUNTIME index on both sides of the assignment, so the element index
        // travels through `Offsets` rather than being folded — the path
        // `write_routed` hands to `dyn_write` as `(off, word)`.
        (
            "dyn_array_runtime_index_copy",
            r#"
module top;
  int a[];
  int b[];
  integer i;
  initial begin
    a = new[3]; b = new[3];
    for (i = 0; i < 3; i = i + 1) a[i] = (i + 1) * 7;
    for (i = 0; i < 3; i = i + 1) b[2 - i] = a[i];
    $display("%0d %0d %0d", b[0], b[1], b[2]);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 2b: strings (ROADMAP §5.1-c) ──────────────────────────
        // A `string` net is the heap kind whose SHAPES differ most from a dyn
        // array: a whole-handle read materializes 8xlen with `is_str`, a
        // whole-handle assign strips leading NULs (§6.16), and a byte select is
        // an element read. All three in one design.
        (
            "string_assign_concat_select_len",
            r#"
module top;
  string s, t;
  integer i;
  initial begin
    s = "hello";
    t = {s, " world"};
    $display("s=%s t=%s len=%0d", s, t, t.len());
    $display("b0=%0d b4=%0d", s[0], s[4]);
    for (i = 0; i < 5; i = i + 1) $write("%c", s[i]);
    $write("\n");
    $display("eq=%0d sub=%s", s == "hello", t.substr(6, 10));
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The EMPTY string and the never-assigned handle: the arena skips a heap
        // net's t0 slot init entirely (its declared init is the packed literal,
        // not a slot value), so "what is a string before anything writes it" is
        // answered by the heap alone. `len()` of each is the discriminator.
        (
            "string_empty_and_unassigned",
            r#"
module top;
  string a;
  string b;
  initial begin
    b = "";
    $display("alen=%0d blen=%0d", a.len(), b.len());
    $display("a=[%s] b=[%s]", a, b);
    a = "x";
    $display("after alen=%0d a=%s", a.len(), a);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A string ALONGSIDE flat nets in the same design, so the router has to
        // send each write to a different store from one funnel — the shape a
        // per-net routing bug produces correct output for only half of.
        (
            "string_beside_flat_nets",
            r#"
module top;
  string s;
  reg [7:0] r;
  integer n;
  initial begin
    s = "ab"; r = 8'hA5; n = 42;
    $display("%s %0h %0d", s, r, n);
    s = "cd"; r = 8'h5A; n = 7;
    $display("%s %0h %0d", s, r, n);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 2c: queues (ROADMAP §5.1-c) ───────────────────────────
        // The row that MEASURED the slice's real defect. Every argument here is
        // a NET, not a literal: `push_back(a)`, `push_front(a+b)`, and the
        // net-valued INDEX of `insert`/`delete`. Those reached the store through
        // `Scheduler::eval`/`eval_ctx_top`, which are hard-wired to the ENGINE's
        // nets — the store tier-3 never writes — so before the fix this printed
        // `q=x 99 X X` against the VM's `q=49 99 42 7`. The literal `32'd99`
        // beside them stayed right, which is exactly what makes the shape worth
        // pinning: a design of literals is blind to it.
        (
            "queue_net_args",
            r#"
module top;
  int q[$];
  reg [7:0] a, b, i;
  initial begin
    a = 8'd42; b = 8'd7; i = 8'd1;
    q.push_back(a); q.push_back(b); q.push_front(a + b);
    q.insert(i, 32'd99);
    $display("q=%0d %0d %0d %0d size=%0d", q[0], q[1], q[2], q[3], q.size());
    q.delete(i);
    $display("del=%0d %0d %0d size=%0d", q[0], q[1], q[2], q.size());
    q.delete();
    $display("all=%0d", q.size());
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The `queue_ops` row's two tables, which slice 2c opened alongside the
        // storage kind: `queue_slice_stmts` (`r = q[a:b]`, with NET bounds — the
        // same wrong-store read as above, in a helper the reader had to be
        // threaded into) and `queue_bounds` (`int bq[$:2]`, whose post-op drop
        // and W4020 come from `SimState::enforce_queue_bound`). Reversed and
        // out-of-range bounds are here because their answer is the EMPTY queue,
        // which is also what a wrong-store read degrades to — so the in-range
        // row is what tells the two apart.
        (
            "queue_slice_and_bound",
            r#"
module top;
  int q[$];
  int r[$];
  int bq[$:2];
  reg [7:0] a, b;
  initial begin
    a = 8'd1; b = 8'd2;
    q.push_back(10); q.push_back(20); q.push_back(30); q.push_back(40);
    r = q[a:b];  $display("s1=%0d %0d %0d", r.size(), r[0], r[1]);
    r = q[b:a];  $display("s2=%0d", r.size());
    r = q[0:99]; $display("s3=%0d %0d", r.size(), r[3]);
    bq.push_back(1); bq.push_back(2); bq.push_back(3); bq.push_back(4);
    $display("bnd=%0d %0d", bq.size(), bq[2]);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── A1-i: queue POP (the first `stmt_effect` carve-out after
        // `$value$plusargs`) ───────────────────────────────────────────────────
        // What this row can and cannot see. `k_queue_pop` is DELEGATED to the
        // engine's own impl, so a native-vs-VM comparison is structurally blind
        // to the pop's SEMANTICS (both backends call the same function —
        // ROADMAP §5.1-e). The hand-IEEE values live in the absolute anchor
        // `queue_pop_has_its_ieee_values`; what THIS row tests is the half the
        // anchor cannot isolate — that the pop sits correctly among native
        // statements: its destination rides `apply_effect`'s `k_write_lvalue`
        // (so it must land in the ARENA, not `SimState`), the queue it drains is
        // the same object the surrounding net-argument pushes filled, and the
        // reads that follow see the drained queue.
        //
        // Every push argument is a NET for the slice-2c reason: a literal-only
        // design cannot tell a wrong-store read from a right one.
        (
            "queue_pop_among_native_statements",
            r#"
module top;
  int q[$];
  int r;
  reg [7:0] a, b;
  initial begin
    a = 8'd42; b = 8'd7;
    q.push_back(a); q.push_back(b); q.push_front(a + b);
    r = q.pop_front();
    $display("pf=%0d size=%0d head=%0d", r, q.size(), q[0]);
    q.push_back(r);
    r = q.pop_back();
    $display("pb=%0d size=%0d", r, q.size());
    while (q.size() > 0) r = q.pop_front();
    $display("drained=%0d size=%0d", r, q.size());
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── A1-ii: the REF-ARG writers ──────────────────────────────────────
        // `$random(seed)`, `$dist_*(seed, …)`, `ok = $cast(dst, src)` and the
        // assoc iteration step all write a net that is NOT the enclosing
        // assignment's destination, from inside the call. The write already went
        // out through `Kernel::k_write_lvalue`; what was engine-only was the
        // READS — the seed variable, the dist parameters, the current key — and
        // they are what this row exercises, by FEEDING each result back in.
        //
        // Feeding back matters: a seed read from the wrong store returns X, the
        // Annex-N zero-substitution turns that into 0, and a single draw off
        // seed 0 still looks like a plausible random number. Only the SECOND
        // draw, and the `$display` of the seed itself, tell the two apart.
        (
            "stmt_effect_ref_arg_writers",
            r#"
module top;
  integer seed, r1, r2, d1;
  int aa[int];
  int k, st;
  byte dst;
  int src;
  initial begin
    seed = 32'd7;
    r1 = $random(seed);  $display("A r1=%0d seed=%0d", r1, seed);
    r2 = $random(seed);  $display("B r2=%0d seed=%0d", r2, seed);
    d1 = $dist_uniform(seed, 10, 20); $display("C d=%0d seed=%0d", d1, seed);
    src = -3; st = $cast(dst, src); $display("D cast=%0d dst=%0d", st, dst);
    aa[7] = 70; aa[11] = 110; aa[2] = 20;
    st = aa.first(k); $display("E st=%0d k=%0d", st, k);
    st = aa.next(k);  $display("F st=%0d k=%0d", st, k);
    st = aa.next(k);  $display("G st=%0d k=%0d", st, k);
    st = aa.next(k);  $display("H st=%0d k=%0d", st, k);
    st = aa.last(k);  $display("I st=%0d k=%0d", st, k);
    st = aa.prev(k);  $display("J st=%0d k=%0d", st, k);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── A1-iii: the SysTask destination writers ─────────────────────────
        // `$sformat` and the `$cast` TASK form write a net from inside their own
        // dispatch. `$readmem*` is in the ANCHOR instead, because it needs a real
        // file on disk and this row runs in-process.
        //
        // Both halves are net-fed on purpose. The write side is what
        // `TaskWrites::Collect` routes; the READ side is what the first cut of
        // this slice got wrong — `cast_task` still called
        // `sched.eval_for_lvalue`, so `sc = 8'd200; $cast(dc, sc);` printed
        // `dc=x` on a native run while the VM printed 200. A literal source
        // operand cannot see that.
        (
            "systask_dest_writers",
            r#"
module top;
  reg [63:0] p;
  string s;
  reg [7:0] dc;
  reg [7:0] sc;
  reg [7:0] n;
  initial begin
    n = 8'd42;
    $sformat(p, "n=%0d h=%h", n, 8'hAB);  $display("A p=%0s", p);
    s = "";
    $sformat(s, "x=%0d", n);              $display("B s=%0s", s);
    sc = 8'd200; $cast(dc, sc);           $display("C dc=%0d", dc);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── A1-iv-a: `$sscanf` ──────────────────────────────────────────────
        // The source is a `string` NET, not a literal, which is the half a
        // literal-only design cannot see: before A1-iv-a `k_sscanf` read it with
        // `Scheduler::eval`. Four destinations of three shapes (two ints, a
        // packed hex, a string) because `scan_write_dst` is what routes them.
        (
            "sscanf_from_a_string_net",
            r#"
module top;
  string s;
  int a, b, n;
  reg [31:0] h;
  string w;
  initial begin
    s = "12 -34 ff hello";
    n = $sscanf(s, "%d %d %h %s", a, b, h, w);
    $display("A n=%0d a=%0d b=%0d h=%0h w=%0s", n, a, b, h, w);
    s = "nope";
    n = $sscanf(s, "%d", a);
    $display("B n=%0d a=%0d", n, a);
    s = "";
    n = $sscanf(s, "%d", a);
    $display("C n=%0d", n);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── A1-iv-b: the fd family's argument reads ─────────────────────────
        // Every fd here is a NET, which is the half that was store-dependent:
        // `$feof`/`$fgetc`/`$ungetc` read it with `Scheduler::eval` before A1-iv-b,
        // so a native run saw X, took the `has_xz` early-out, and returned the
        // failure code WITHOUT the bad-fd warning the VM emits. No file is
        // opened on purpose — this row is in-process, and the real-file half
        // lives in `fd_family_has_its_iverilog_values`.
        (
            "fd_family_net_arguments",
            r#"
module top;
  integer bad, r, c, u;
  string path, mode;
  integer fd;
  initial begin
    bad = 32'd12345;
    r = $feof(bad);    $display("A eof=%0d", r);
    c = $fgetc(bad);   $display("B getc=%0d", c);
    u = $ungetc(c, bad); $display("C ung=%0d", u);
    path = "/definitely/not/here.txt"; mode = "r";
    fd = $fopen(path, mode);
    $display("D open=%0d", fd);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // All THREE admitted heap kinds plus flat nets in one design, so the
        // funnel has to send four destinations to three different stores from a
        // single call site. A per-kind routing bug produces correct output for
        // part of this and only part.
        //
        // `q.push_back(d[0])` is the row's second job and the only one in the
        // corpus: a task argument that reads a HEAP net. That is what makes the
        // `HeapRouted` wrapper inside `eval_ctx_with_reader` observable — the
        // reader tier-3 hands `dispatch` is the ARENA, which does not own `d`,
        // so without the wrapper this read reaches `assert_owns` instead of the
        // heap. Every other argument here is a flat net, which only exercises
        // the threading, not the routing.
        (
            "queue_beside_string_and_flat",
            r#"
module top;
  int q[$];
  string s;
  reg [7:0] r;
  int d[];
  reg [7:0] n;
  initial begin
    n = 8'd2; s = "hi"; r = 8'hA5;
    q.push_back(n); d = new[n];
    d[0] = 77; d[1] = 88;
    $display("%0d %s %0h %0d", q[0], s, r, d.size());
    q.push_back(r); s = "bye"; r = 8'h5A;
    q.push_back(d[0]);
    $display("%0d %s %0h %0d %0d", q[1], s, r, d.size(), q[2]);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The WAVEFORM axis for the heap kinds, which `agree` only compares on a
        // design that dumps. The grounding note for slice 2 listed "how do the
        // VCD/dirty/edge channels treat these nets" as an open question; the
        // answer is that a heap net is outside all three on BOTH backends (no
        // net dirty channel, by the dyn precedent), and the flat nets beside it
        // still produce their value changes. This row is what makes that an
        // assertion rather than a claim — a routing bug that mirrored a heap
        // write into the arena would show up here as an extra VCD record.
        (
            "queue_and_string_with_dumpvars",
            r#"
module top;
  int q[$];
  reg [7:0] r;
  reg clk;
  string s;
  initial begin
    $dumpfile("out.vcd"); $dumpvars(0, top);
    clk = 0; r = 8'h11; s = "a";
    #1 q.push_back(r); r = 8'h22; s = "bb"; clk = 1;
    #1 q.push_back(r); r = 8'h33; clk = 0;
    #1 $display("%0d %0d %0h %s", q.size(), q[0], r, s);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ⚠️ Slices 2a and 2b shipped WITHOUT this row and were wrong for it.
        // `new[n]`, `s.putc(i,c)` and `s.itoa(v)` read their arguments through
        // the same un-threaded evaluator; with net-valued arguments 2a printed
        // `size=0` for `new[3]` and 2b printed `s=0` for `itoa(200)`. Both
        // slices' own rows used literals, so both suites were green. The lesson
        // is not about queues: when a slice admits a task, its ARGUMENTS are a
        // second store read and need their own row.
        (
            "heap_task_args_are_nets",
            r#"
module top;
  int d[];
  string s, t;
  reg [7:0] n, v, i, c;
  initial begin
    n = 8'd3; v = 8'd200; i = 8'd1; c = 8'd90;
    d = new[n];
    s = "abc"; s.putc(i, c);
    t.itoa(v);
    $display("%0d %s %s", d.size(), s, t);
    t.hextoa(v); $display("%s", t);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 3b: heap ELEMENT refinements (ROADMAP §5.1-d) ─────────
        // Slice 2a admitted the dyn-array CONTAINER and deliberately left both
        // element refinements refused: a `string s[]` element is a byte string
        // and a `real r[]` element is an f64, neither of which is the bit-vector
        // element the container row was measured on. Measured since: both lanes
        // live entirely in `SimState`'s heap methods (`coerce_dyn_elem`,
        // `alloc_dyn_array`, `dyn_read`/`dyn_write`), which slice 2 already routes
        // every heap access to — the refinements were as conservative as the
        // container had been.
        //
        // `new[]`'s per-kind DEFAULT is the sharpest part: IEEE §7.5.2 says "" for
        // a string element and 0.0 for a real one, and `alloc_dyn_array` picks
        // that from the same two flags. An element never written is what tells a
        // correct default from a plausible one.
        (
            "string_and_real_dyn_elements",
            r#"
module top;
  string sd[];
  real rd[];
  int d[];
  reg [7:0] i;
  initial begin
    i = 8'd1;
    sd = new[3]; rd = new[3]; d = new[2];
    sd[0] = "ab"; sd[i] = "cde";
    rd[0] = 1.5; rd[i] = -2.25;
    d[0] = 7;
    $display("[%s][%s][%s] len=%0d", sd[0], sd[i], sd[2], sd[0].len());
    $display("%0f %0f %0f", rd[0], rd[i], rd[2]);
    $display("%0d %0d %0d %0d", d[0], sd.size(), rd.size(), sd[0][0]);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The QUEUE twin of the same refinement — `string q[$]` is the shape whose
        // element write once read back EMPTY (the push did its own `.resize(w)`
        // while the dyn-array write took the byte-string branch), which is why
        // `coerce_dyn_elem` exists at all. A row of dyn-array-only string elements
        // would not reach that push.
        (
            "string_queue_elements",
            r#"
module top;
  string q[$];
  reg [7:0] i;
  initial begin
    i = 8'd1;
    q.push_back("hi"); q.push_back("yo"); q.push_front("za");
    $display("[%s][%s][%s] %0d", q[0], q[i], q[2], q.size());
    q[i] = "mid";
    $display("[%s] %0d", q[i], q[i].len());
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 3a: a CALL in a system-task argument (ROADMAP §5.1-c) ──
        // `native::frames` refused this with a reason that was true when it was
        // written: `k_dispatch_systask` holds `&mut Scheduler`, so it hands
        // `dispatch` the ARENA alone and cannot also lend `&SimState` to a
        // composite — and the arena's `eval_call` is a loud panic for exactly
        // that. What changed is that slice 2 built a composite one level down
        // for an unrelated reason (`HeapRouted`, to route heap nets), and it
        // holds both stores. The row is gone; these rows are what hold the line.
        //
        // Net-valued and nested arguments, and a `$sformatf` — the FUNCTION form
        // whose render is the same seam — because the failure mode was a panic
        // in one specific reader, not a wrong value in a class of them.
        (
            "call_in_system_task_argument",
            r#"
module top;
  function automatic int sq(input int x);
    return x * x;
  endfunction
  function automatic int inc(input int x);
    return x + 1;
  endfunction
  string s;
  reg [7:0] a;
  initial begin
    a = 8'd5;
    $display("%0d %0d", sq(a), sq(3));
    $display("%0d", inc(inc(a)));
    $write("%0d\n", inc(a));
    s = $sformatf("v=%0d", sq(a));
    $display("%s", s);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The ORDERING neighbour: an out-of-range element read beside a call in
        // the same argument list. The arena counts that access and defers the
        // report; the kernel drains it at seams, and a call is one of them. If
        // the two ever crossed, the diagnostic would move relative to the line
        // it belongs to — which `agree` compares as one interleaved stream.
        (
            "call_beside_an_out_of_range_read",
            r#"
module top;
  function automatic int id(input int x);
    return x;
  endfunction
  reg [7:0] mem [0:3];
  reg [7:0] i;
  initial begin
    i = 8'd9;
    mem[0] = 8'd1;
    $display("%0d %0d", id(3), mem[i]);
    $display("%0d %0d", mem[i], id(4));
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The FORMAL half of the same seam — and the row whose OWN claim the
        // mutation battery refuted, which is why it is written out here.
        //
        // The claim was: `formal_width`/`formal_is_string` decide how each ACTUAL
        // is coerced before the frame sees it (§4.5.325), the arena answers both
        // with the trait DEFAULT (a plausible value, not a loud one), so a row of
        // int-formal calls is blind to a router that forwards them wrongly.
        //
        // ⚠️ MEASURED FALSE on this path. Routing both to the arena leaves narrow,
        // widening-signed and string formals byte-identical — and so does a
        // HOSTILE `formal_width` that answers `Some((1, false))` for every formal.
        // `eval_core`'s coercion is a PRE-sizing; `run_frame_call` then binds each
        // actual into the frame slot from its own metadata, and that binding is
        // what decides the value. The reader's answer is overwritten.
        //
        // The forwarding stays `st` anyway, for the reason `resolve_virtual_call`
        // already carries: it is the same answer the kernel gives one level up,
        // and a one-line correct answer beats a silent wrong one the day this
        // path becomes load-bearing. The row stays because these shapes DO
        // exercise `eval_call` through the new seam — just not the formal half.
        (
            "call_formals_in_a_system_task_argument",
            r#"
module top;
  function automatic [3:0] narrow(input [3:0] x);
    return x + 4'd1;
  endfunction
  function automatic int strlen_of(input string s);
    return s.len();
  endfunction
  reg [7:0] a;
  string t;
  initial begin
    a = 8'hFE; t = "hello";
    $display("%0d %0d", narrow(a), strlen_of(t));
    $display("%0d", strlen_of("ab"));
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 2d: associative arrays (ROADMAP §5.1-c) ───────────────
        // The kind whose WRITE does not fit the shared `dyn_write`: a key is an
        // i64 that cannot ride the `(offset, word)` u32 pairs, so it travels out
        // of band and `write_routed` needs its own arm. Before that arm existed
        // this design stored nothing and read back `x x 0 0` against the VM's
        // `7 11 2 1` — the key silently became 0 and `dyn_write` refused the
        // shape. Net-valued and NEGATIVE keys are here because the key domain is
        // signed i64 and the pairs are u32: a route that truncated would put
        // `-3` and `4294967293` in the same bucket.
        (
            "assoc_int_keys_and_delete",
            r#"
module top;
  int aa[int];
  reg [7:0] k;
  reg signed [7:0] n;
  initial begin
    k = 8'd5; n = -8'sd3;
    aa[3] = 7; aa[9] = 11; aa[k] = 42; aa[n] = 99;
    $display("%0d %0d %0d %0d", aa[3], aa[9], aa[k], aa[n]);
    $display("size=%0d ex3=%0d ex77=%0d miss=%0d", aa.num(), aa.exists(3), aa.exists(77), aa[77]);
    // A 32-bit signed LITERAL naming the same entry an 8-bit signed NET wrote.
    // The two agree only because a key is evaluated at >= 64 bits before it
    // becomes an i64.
    //
    // ⚠️ The first version of this comment claimed the line MAKES the key-width
    // rule observable here. The mutation battery refuted that: dropping the
    // `.max(64)` leaves this row green and is killed by the engine's own
    // `dyn_storage_b::assoc_{first_next,last_prev}_*` tests. The reason is the
    // one this file keeps rediscovering — the rule lives in ONE shared function,
    // so both backends move together and a native-vs-VM differential is blind to
    // it by construction. The line stays because it says what the design means;
    // the teeth for that rule are, correctly, elsewhere.
    $display("neg=%0d", aa[-3]);
    aa.delete(k);
    $display("after size=%0d exk=%0d", aa.num(), aa.exists(5));
    aa.delete();
    $display("all size=%0d", aa.num());
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The string-keyed twin, with a PACKED key rather than a `string` one.
        //
        // ⚠️ That choice IS the row. `sa.delete(k)` with `string k` agreed with
        // the VM before the reader was threaded — but by luck: a `string` is
        // itself a heap net, so the engine's own store happened to hold it. A
        // `reg [15:0] k = "hi"` reads the FLAT store, which a native run never
        // writes, and that is the case that discriminates.
        (
            "assoc_string_keys_packed",
            r#"
module top;
  int sa[string];
  reg [15:0] k;
  initial begin
    k = "hi";
    sa[k] = 4; sa["yo"] = 9;
    $display("%0d %0d %0d %0d", sa[k], sa["yo"], sa.num(), sa.exists("no"));
    sa.delete(k);
    $display("after %0d %0d", sa.num(), sa.exists("yo"));
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // All FOUR heap kinds and a flat net in one design — the widest the
        // funnel gets. One call site, four destinations, three stores, and the
        // assoc lane splitting off before the other two.
        (
            "all_four_heap_kinds_together",
            r#"
module top;
  int aa[int];
  int sa[string];
  int q[$];
  int d[];
  string s;
  reg [7:0] r;
  initial begin
    r = 8'd9;
    aa[r] = 1; sa["k"] = 2; q.push_back(r); d = new[2]; d[0] = 3; s = "z";
    $display("%0d %0d %0d %0d %s %0h", aa[r], sa["k"], q[0], d[0], s, r);
    r = 8'd4;
    aa[r] = 5; q.push_back(aa[9]); s = "zz";
    $display("%0d %0d %0d %s %0h", aa[4], q[1], q.size(), s, r);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── V1 slice 1: SVA (§4.5.337) ─────────────────────────────────────
        // These are here rather than in a file of their own because
        // `adversarial_designs` is the set `agree` proves BOTH halves on: it
        // calls `runnable()` first, so a row that silently started falling back
        // fails LOUDLY instead of comparing the VM against itself. That matters
        // more for this slice than for any before it — the change is the REMOVAL
        // of a gate row, and a removal's only failure mode is a design that now
        // runs where it did not.
        //
        // OVERLAPPING implication: the desugar is `always @(clk) if (a && !b)
        // $error(…)`, and the `$error` sid is in `assert_fire`. The row fires at
        // t=25, so exit class and the diagnostic stream both discriminate.
        (
            "sva_overlapping_implication_fires",
            r#"
module top;
  reg clk = 0, a = 0, b = 0;
  always #5 clk = ~clk;
  initial assert property(@(posedge clk) a |-> b);
  initial begin
    #10 a = 1; b = 1;
    #10 a = 1; b = 0;
    #10 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // NON-OVERLAPPING (`|=>`) — the antecedent is delayed one clock through a
        // synthesized pending reg, so this exercises a WRITE the overlapping form
        // does not have (and the reg is 0-init, not X-init: `fresh_sva_reg0`).
        (
            "sva_non_overlapping_implication",
            r#"
module top;
  reg clk = 0, a = 0, b = 0;
  always #5 clk = ~clk;
  initial assert property(@(posedge clk) a |=> b);
  initial begin
    #10 a = 1;
    #10 a = 0; b = 0;
    #10 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ASSERTION CONTROL is the half the implication rows cannot reach: a
        // `$assertoff` site is a no-op `Display` whose sid is in `assert_ctl`, and
        // dispatching it MUTATES `st.assert_disabled`, which then suppresses an
        // `assert_fire` sid. Both windows are exercised — a violation while
        // disabled (silent) and one after `$asserton` (fires) — so a backend that
        // dropped either half changes the exit class.
        (
            "sva_assert_control_off_then_on",
            r#"
module top;
  reg clk = 0, a = 0, b = 0;
  always #5 clk = ~clk;
  initial assert property(@(posedge clk) a |-> b);
  initial begin
    #7  $assertoff;   // t=7   assertions off
    #3  a = 1; b = 0; // t=10  -> posedge 15 VIOLATES, suppressed
    #6  a = 0;        // t=16  clear the window BEFORE re-enabling, or the
                      //       still-high `a` violates again at posedge 25
    #4  $asserton;    // t=20  -> posedge 25: a=0, implication holds
    #10 a = 1; b = 0; // t=30  -> posedge 35 VIOLATES, fires
    #10 $finish;      // t=40
  end
endmodule
"#
            .to_string(),
        ),
        // SAMPLED-VALUE functions. `$past` needs its own history reg; `$rose`/
        // `$fell`/`$stable` compare against it. They are legal ONLY inside a
        // property (a bare `$past(d)` in an `always` is E3009), which is why the
        // first probe of this slice measured nothing until it was moved inside.
        (
            "sva_sampled_value_functions",
            r#"
module top;
  reg clk = 0, a = 0, b = 0;
  always #5 clk = ~clk;
  initial assert property(@(posedge clk) $past(a) |-> b);
  initial assert property(@(posedge clk) $rose(a) |-> b);
  initial assert property(@(posedge clk) $fell(a) |-> !b);
  initial assert property(@(posedge clk) $stable(b) |-> 1'b1);
  initial assert property(@(posedge clk) $sampled(a) |-> b);
  initial begin
    #10 a = 1; b = 1;
    #10 a = 0; b = 0;
    #10 $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A property over a COUNTER, so the checker reads a net the design is
        // still driving through NBA — the shape where a backend that sampled at
        // the wrong point in the region cascade would diverge without any
        // assertion machinery being wrong.
        (
            "sva_property_over_nba_counter",
            r#"
module top;
  reg clk = 0;
  reg [3:0] c = 0;
  always #5 clk = ~clk;
  always @(posedge clk) c <= c + 1;
  initial assert property(@(posedge clk) (c > 4'd2) |-> ($past(c) < c));
  initial begin #100 $finish; end
endmodule
"#
            .to_string(),
        ),
    ]
}

#[test]
fn s1d4c2c_native_run_matches_the_vm_on_adversarial_shapes() {
    let designs = adversarial_designs();
    assert_eq!(designs.len(), 91, "adversarial set shrank");
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
    // The multi-driven and `wand`/`wor` rows lived here until S1d-4d-4 wired
    // the group resolution into the settle — those shapes run now, and their
    // designs moved to the adversarial set and the oracle anchors.
    let cases: Vec<(&str, &str, &str)> = vec![
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
        // ⚠️ These two used `$monitor` until A5-b, which WIRED it — so the row
        // they pin had to be re-spelled with a task that is still refused
        // (`$writemem*`, which reads the MEMORY itself rather than a formatted
        // argument). A refusal-row test whose shape stops being refused turns
        // green by measuring nothing.
        (
            "refused system task",
            "a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)",
            r#"
module top;
  reg [7:0] m [0:1];
  initial begin m[0] = 8'd0; m[1] = 8'd1; $writememh("out.hex", m); #1 $finish; end
endmodule
"#,
        ),
        (
            "refused system task in a LATER block",
            "a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)",
            r#"
module top;
  reg [7:0] n;
  reg [7:0] m [0:1];
  initial begin
    n = 8'd0;
    m[0] = 8'd0; m[1] = 8'd1;
    #1 n = 8'd1;
    if (n == 8'd1) $writememb("out2.hex", m);
    #1 $finish;
  end
endmodule
"#,
        ),
        (
            "wait fork",
            "a `wait fork`, a `fork`, or a call statement whose callee suspends: S3b",
            r#"
module top;
  reg [7:0] n = 8'd0;
  initial begin wait fork; n = 8'd7; $display("after n=%0d", n); end
  initial #2 $finish;
endmodule
"#,
        ),
    ];
    // ⚠️⚠️ **The call-statement half of the `wait fork` row has NO DESIGN, and
    // that is a measurement rather than an omission.** After A3-ii-a a call
    // statement is refused only when its callee PARKS, and a parking callee is
    // always a TASK: elaborate refuses a timing control inside a FUNCTION outright
    // (E3009, "functions cannot have delay statements" — iverilog says the same),
    // and a function cannot enable a task. A parking task is refused a layer
    // earlier, by `frames_admitted`'s own row, so nothing reaches this one through
    // the call arm.
    //
    // The row keeps the clause anyway, and `call_site_runnable` keeps answering
    // `false`: the two gate layers are asked INDEPENDENTLY by the census and by
    // `runnable`, and a layer that stopped refusing a shape because some other
    // layer happens to get there first is how a widening loses it. What is
    // recorded here is that today the clause is defensive — pinned by
    // `a_parking_callee_is_refused_by_the_storage_layer` rather than by a row in
    // this table.
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
    // 4 -> 5 (A3-i: the call-statement half became its own population) -> 4 again
    // (A3-ii-a: that population is now unreachable — see the note above).
    assert_eq!(cases.len(), 4, "refusal-row coverage moved");
    // The LAST case exists for a property the others do not have: its refused
    // task is behind a `#1` and an `if`, so it lives in neither the entry block
    // nor block 0. `body_dispatch_ok` scanning only the first block would admit
    // it — and a design admitted with a refused task in it does not produce a
    // wrong answer, it panics mid-run inside `k_dispatch_systask`.
    //
    // ⚠️ The `wait fork` case is here because the claim that replaced it was
    // WRONG. S1d-4c-2d deleted the two in-body-waiter cases (correctly — those
    // shapes run now) and asserted the remaining members of that row were each
    // refused by an earlier layer. Measured false: a bare `wait fork;` lowers to
    // `WaitCause::Fork` and populates NO `fork_modes` entry, so the design is
    // `eligible: true, buildable: true` and this row is the only thing keeping
    // it out — of a walk that would park it forever. `Call` is genuinely
    // earlier-refused (`func_table` → the arena), and `WaitCause::Named` is
    // never constructed at all.
    // ⚠️ The `fork` and subroutine-CALL halves of the waiter row have no case,
    // and cannot: a `Terminator::Call` needs a non-empty `func_table`, which
    // `NetArena::build` refuses first (measured — the case was written and the
    // verdict came back "frame-local storage: S3"), and `fork_modes` non-empty
    // is an S0 reject. Both halves of that row are defence in depth against a
    // lost `.velab` trailer, not reachable paths.
}

/// The composite reader must answer EVERY `NetReader` method (V1 slice 2).
///
/// `NetReader` has one required method and twenty with defaults, and every one
/// of those defaults returns a value that LOOKS like an answer — a `None` the
/// caller turns into X, a `false` meaning "not an associative array", an `xs`.
/// A backend that inherits one is not loud about it; it is quietly wrong the
/// moment a gate row lets a design reach that capability.
///
/// Measured, not feared: opening the `dyn_array` row for slice 2a made
/// `q.size()` read `x` on the line after `q = new[4]`, because `dyn_size`'s
/// default is `None` and tier-3 had never overridden it (ROADMAP §5.1-c).
///
/// So the rule is totality, and it is checked structurally because the failure
/// is an ABSENT method — there is no call site to put a runtime assertion on,
/// and the trait compiles happily without it.
#[test]
fn the_composite_reader_overrides_every_netreader_method() {
    let trait_src = include_str!("../eval/eval_core.rs");
    let body = trait_src
        .split_once("pub trait NetReader {")
        .expect("trait moved")
        .1;
    let body = &body[..body.find("\n}").expect("trait end")];
    let declared: std::collections::BTreeSet<&str> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix("fn "))
        .filter_map(|l| l.split('(').next())
        .collect();

    let kernel_src = include_str!("kernel.rs");
    let imp = kernel_src
        .split_once("impl crate::eval::NetReader for NativeKernel")
        .expect("impl moved")
        .1;
    let imp = &imp[..imp.find("\n}").expect("impl end")];
    let overridden: std::collections::BTreeSet<&str> = imp
        .lines()
        .filter_map(|l| l.strip_prefix("    fn "))
        .filter_map(|l| l.split('(').next())
        .collect();

    assert!(
        declared.len() >= 20,
        "the trait scan found only {} methods — it stopped matching the source",
        declared.len()
    );
    let missing: Vec<&&str> = declared.difference(&overridden).collect();
    assert!(
        missing.is_empty(),
        "tier-3 would inherit `NetReader`'s default for {missing:?}. \
         Each default returns a plausible value rather than failing, so the \
         result is a silently wrong answer as soon as a gate row admits a \
         design that reaches it. Delegate to `self.sched.st` (the state that \
         owns the capability) or answer from the arena — on purpose."
    );
}

/// The write funnel must STAY a funnel (V1 slice 2, ROADMAP §5.1-c).
///
/// `NativeKernel::write_routed` exists so that "which store owns this net" is
/// asked in ONE place; slice 2 makes that question have two answers (the flat
/// arena, and `SimState::dyn_heap` for a heap-kind net). A new
/// `arena.write_lvalue` call added anywhere else would be a second spelling of
/// the routing decision, and the failure mode is not a compile error — it is a
/// heap net silently written into a dead flat slot while every read takes the
/// other path, i.e. a write that vanishes.
///
/// A SOURCE scan, deliberately: no runtime test can see a call site that the
/// design under test never reaches, and this file's own history is a list of
/// gates that were green because nothing exercised the row.
#[test]
fn every_tier3_store_goes_through_the_one_write_funnel() {
    let files = [
        ("kernel.rs", include_str!("kernel.rs")),
        ("run.rs", include_str!("run.rs")),
        ("body.rs", include_str!("body.rs")),
        ("frames.rs", include_str!("frames.rs")),
    ];
    // ⚠️ **PER-LINE MATCHING HID A SITE FOR THREE SLICES.** This scan used to
    // ask whether one LINE contains `arena.write_lvalue(`, and rustfmt had
    // split `k.arena` from `.write_lvalue(` at the one call site that was not
    // the funnel — so the pin read "exactly one" while there were two. Slice #2
    // re-joined the line by adding an argument, which is the only reason it
    // surfaced. The scan now strips comments and collapses whitespace before
    // matching, so a formatter cannot decide whether this test has teeth.
    let mut sites: Vec<(&str, usize)> = Vec::new();
    for (name, src) in files {
        let lines: Vec<&str> = src.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            // Skip doc/comment lines: the funnel's own doc names the call it
            // replaced, and a comment is not a call.
            if line.trim_start().starts_with("//") {
                continue;
            }
            // Join with the following non-comment lines so a receiver split
            // from its method by the formatter still matches.
            let mut joined = String::new();
            for l in lines.iter().skip(i).take(3) {
                if l.trim_start().starts_with("//") {
                    continue;
                }
                joined.push_str(l.trim());
            }
            if joined.contains("arena.write_lvalue(") && !line.contains("fn write_lvalue") {
                sites.push((name, i + 1));
            }
        }
    }
    // A joined match can report the same call from up to three starting lines;
    // keep only the first of each run.
    sites.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 + 1);
    assert_eq!(
        sites.len(),
        1,
        "exactly one store site is allowed, and it is the body of \
         `NativeKernel::write_routed`. Found: {sites:?}"
    );
    assert_eq!(sites[0].0, "kernel.rs");
}

/// The ABSOLUTE anchor for V1 slice 3b — what a heap ELEMENT of each refined
/// kind actually IS, not merely that two backends agree about it.
///
/// ⚠️ Written because the differential cannot hold this line, and that was
/// measured rather than reasoned. `build_with_opts` did not install
/// `string_elem_dyn_nets`, and the corpus row for string elements then printed
/// `[ ][\u{1}][ ] len=0` on BOTH backends — perfect agreement about a design
/// neither was executing as written. (The `real` half diverged, which is what
/// surfaced the omission at all; a string-only slice would have shipped green.)
///
/// So the sidecars are installed now, and this pins the VALUES so that removing
/// them again fails here instead of going quiet.
///
/// ⭐ Confirmed by the battery rather than argued: of five mutations, the one
/// that drops `string_elem_dyn_nets` from the harness and the one that makes the
/// SHARED `coerce_dyn_elem` ignore its string branch are killed by THIS test and
/// by nothing else. Both are invisible to a native-vs-VM differential for the
/// same reason — they move both backends together.
#[test]
fn heap_element_refinements_have_their_ieee_defaults_and_values() {
    let src = r#"
module top;
  string sd[];
  real rd[];
  initial begin
    sd = new[3]; rd = new[3];
    sd[0] = "ab";
    rd[0] = 1.5;
    $display("[%s][%s] len=%0d", sd[0], sd[2], sd[0].len());
    $display("%0f %0f", rd[0], rd[2]);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            // A written element is its own text; an UNWRITTEN one is the IEEE
            // §7.5.2 default for its element type — "" for a string, 0.0 for a
            // real — which is what `alloc_dyn_array` picks from these two flags.
            "out|[ab][] len=2\n".to_string(),
            "out|1.500000 0.000000\n".to_string(),
        ],
        "heap element refinement values"
    );
}

/// A1-i ABSOLUTE ANCHOR — the hand-IEEE values of `q.pop_front()`/`pop_back()`.
///
/// ⚠️ This test exists because the differential CANNOT defend this slice.
/// `NativeKernel::k_queue_pop` delegates to `Scheduler::k_queue_pop`, so a
/// native-vs-VM comparison runs the same function twice and would agree on any
/// answer, right or wrong (ROADMAP §5.1-e — the differential goes blind exactly
/// where tier-3 delegates). The values below are the contract, not a recording:
///
/// * **A/B/C** — IEEE 1800 §7.10.2.2/§7.10.2.3: `pop_front` takes the FIRST
///   element, `pop_back` the LAST, and each shortens the queue by one. Swapping
///   the two ids gives `30/10/20` here, which is why the queue is asymmetric.
/// * **D/E** — a pop on an EMPTY queue yields X. `int` is 2-state (§6.8: it
///   cannot hold X), so the X result coerces to 0 on the way into `r`. iverilog
///   13 prints `x` for D/E — it does not enforce 2-state on `int` — so this line
///   is hand-IEEE, deliberately NOT iverilog-pinned.
/// * **only ONE `diag|` between D and E** — the empty-pop warning is warn-ONCE
///   per net (`dyn_warn_once_at`), vita's established anti-spam policy; iverilog
///   warns per call. Pinned so a future "fix" that makes it per-call is a
///   failure and not a silent change.
/// * **F** — the returned value is sized to the DESTINATION and sign-extended by
///   the ELEMENT's signedness (`resize_keep_sign(lw.max(sw.width), sw.signed)`),
///   so a signed −3 popped into a `byte` stays −3 rather than becoming 253.
/// * **G** — the same empty pop into a 4-state destination keeps its X, which is
///   what proves D/E is the 2-state coercion and not a lost X. iverilog agrees
///   on F and G.
/// * **H/I** — F cannot see the SIGN half: a 16-bit −3 into a `byte` truncates to
///   −3 whether the widening extended or zero-filled. So the element is narrower
///   than the destination here, where signed −3 must reach `int` as −3 and the
///   unsigned twin as 13. Both iverilog-agreeing; without this pair, flipping
///   `sw.signed` to a constant is an equivalent mutation.
#[test]
fn queue_pop_has_its_ieee_values() {
    let src = r#"
module top;
  int q[$];
  logic signed [15:0] w[$];
  logic signed [3:0] sn[$];
  logic [3:0] un[$];
  int r;
  byte nb;
  logic signed [15:0] rw;
  initial begin
    q.push_back(10); q.push_back(20); q.push_back(30);
    r = q.pop_front(); $display("A pf=%0d size=%0d", r, q.size());
    r = q.pop_back();  $display("B pb=%0d size=%0d", r, q.size());
    r = q.pop_front(); $display("C pf=%0d size=%0d", r, q.size());
    r = q.pop_front(); $display("D empty2state=%0d", r);
    r = q.pop_back();  $display("E empty_again=%0d", r);
    w.push_back(-3);
    nb = w.pop_front(); $display("F narrow=%0d", nb);
    rw = w.pop_back();  $display("G empty4state=%0h", rw);
    sn.push_back(-3); un.push_back(4'd13);
    r = sn.pop_front(); $display("H sext=%0d", r);
    r = un.pop_front(); $display("I zext=%0d", r);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A pf=10 size=2\n".to_string(),
            "out|B pb=30 size=1\n".to_string(),
            "out|C pf=20 size=0\n".to_string(),
            "diag|Warning|VITA-W4020|pop on an empty queue (X)".to_string(),
            "out|D empty2state=0\n".to_string(),
            // no second 4020: warn-ONCE per net.
            "out|E empty_again=0\n".to_string(),
            "out|F narrow=-3\n".to_string(),
            "diag|Warning|VITA-W4020|pop on an empty queue (X)".to_string(),
            "out|G empty4state=xxxx\n".to_string(),
            "out|H sext=-3\n".to_string(),
            "out|I zext=13\n".to_string(),
        ],
        "queue pop IEEE values"
    );
}

/// A1-ii ABSOLUTE ANCHOR — the ref-arg writers, half of it iverilog-pinned.
///
/// The differential is blind here for the A1-i reason (both backends now run the
/// SAME `exec::stmt_effect` body), so these are contract values:
///
/// * **A/B/C** — pinned to LIVE iverilog 13.0. `$random(seed)` is the IEEE
///   1364-2005 Annex-N LCG and iverilog is its reference implementation, so the
///   draws AND the seed write-back are cross-checked, not recorded. Two draws,
///   because one draw off a wrongly-read (X → 0) seed still looks plausible; it
///   is the SECOND draw and the printed seed that separate them.
/// * **D** — `$cast` has no oracle (iverilog 13 rejects it): hand-IEEE §6.24.2,
///   an integral assignment always succeeds, so the status is 1 and the −3 is
///   truncated to the `byte` destination by the ordinary write funnel.
/// * **E..J** — iverilog cannot parse `int aa[int]`, so hand-IEEE §7.9.4: the
///   iteration visits keys in ASCENDING order regardless of insertion order
///   (2, 7, 11 for pushes 7, 11, 2), `next` past the last returns status 0 and
///   LEAVES THE KEY UNCHANGED, and `last`/`prev` walk back down.
///
/// ⚠️ `$dist_normal` is deliberately absent: vita and iverilog differ by one
/// (53 vs 54) on the same seed, a pre-existing rounding difference in
/// `rng::dist_normal` that both vita backends share. Pinning it here would put a
/// known divergence inside an anchor, which is how an anchor stops being one.
#[test]
fn stmt_effect_ref_arg_writers_have_their_values() {
    let src = r#"
module top;
  integer seed, r1, r2, d1;
  int aa[int];
  int k, st;
  byte dst;
  int src;
  initial begin
    seed = 32'd7;
    r1 = $random(seed);  $display("A r1=%0d seed=%0d", r1, seed);
    r2 = $random(seed);  $display("B r2=%0d seed=%0d", r2, seed);
    d1 = $dist_uniform(seed, 10, 20); $display("C d=%0d seed=%0d", d1, seed);
    src = -3; st = $cast(dst, src); $display("D cast=%0d dst=%0d", st, dst);
    aa[7] = 70; aa[11] = 110; aa[2] = 20;
    st = aa.first(k); $display("E st=%0d k=%0d", st, k);
    st = aa.next(k);  $display("F st=%0d k=%0d", st, k);
    st = aa.next(k);  $display("G st=%0d k=%0d", st, k);
    st = aa.next(k);  $display("H st=%0d k=%0d", st, k);
    st = aa.last(k);  $display("I st=%0d k=%0d", st, k);
    st = aa.prev(k);  $display("J st=%0d k=%0d", st, k);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A r1=-2146999808 seed=483484\n".to_string(),
            "out|B r2=1181502348 seed=-965981971\n".to_string(),
            "out|C d=17 seed=-1386778934\n".to_string(),
            "out|D cast=1 dst=-3\n".to_string(),
            "out|E st=1 k=2\n".to_string(),
            "out|F st=1 k=7\n".to_string(),
            "out|G st=1 k=11\n".to_string(),
            // past the last key: status 0 and the key is LEFT WHERE IT WAS.
            "out|H st=0 k=11\n".to_string(),
            "out|I st=1 k=11\n".to_string(),
            "out|J st=1 k=7\n".to_string(),
        ],
        "stmt_effect ref-arg writer values"
    );
}

/// A1-iii ABSOLUTE ANCHOR — the SysTask destination writers, iverilog-pinned.
///
/// `$readmem*` lives here rather than in the corpus row because it needs a real
/// file, and the file is what makes the WINDOW BOUNDS observable: `lo`/`hi` are
/// NETS, and before A1-iii `readmem` read them with `sched.eval` — the engine's
/// store — so a native run saw X, `to_u64()` gave `None`, and the load silently
/// covered the WHOLE array instead of `[3:5]`. Values cross-checked against live
/// iverilog 13.0, which supports `$readmemh` with bounds.
///
/// The hex file has NO `@addr` directive on purpose: with one, the directive
/// picks the addresses and the bounds stop discriminating — which is exactly how
/// the first probe of this slice passed while the bug was still there.
#[test]
fn readmem_window_bounds_come_from_this_store() {
    let dir = std::env::temp_dir().join(format!("vita-a1iii-{}-{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let hex = dir.join("m.hex");
    std::fs::write(&hex, "1A\n2B\n3C\n").expect("hex file");
    let src = format!(
        r#"
module top;
  reg [15:0] m [0:7];
  integer i, lo, hi;
  initial begin
    for (i = 0; i < 8; i = i + 1) m[i] = 16'hFFFF;
    lo = 3; hi = 5;
    $readmemh("{}", m, lo, hi);
    $display("W %h %h %h %h %h %h %h %h", m[0],m[1],m[2],m[3],m[4],m[5],m[6],m[7]);
    $finish;
  end
endmodule
"#,
        hex.display()
    );
    let (ir, opts) = build_with_opts(&src);
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            // Exactly [3:5] loaded, everything else keeps its pre-load value.
            // Untreaded bounds gave `ffff ffff 001a 002b 003c ffff ffff ffff`.
            "out|W ffff ffff ffff 001a 002b 003c ffff ffff\n".to_string(),
        ],
        "readmem window"
    );
}

/// A1-iv-a ABSOLUTE ANCHOR — `$sscanf`, pinned to live iverilog 13.0.
///
/// iverilog implements `$sscanf`, so unlike most of this file's anchors these
/// are cross-checked values rather than hand-IEEE ones. Three returns that a
/// wrong-store read cannot all reproduce:
///
/// * **A** — four conversions off a string NET; every destination written.
/// * **B** — no match: the return is **0** and `a` KEEPS its previous value
///   (12), which is what separates "matched nothing" from "wrote a zero".
/// * **C** — an EMPTY source: the return is **−1** (EOF), not 0. Reading the
///   source from the wrong store yields an empty string too, so B and C
///   together are what tell a routed read from an unrouted one — B would become
///   −1 as well.
#[test]
fn sscanf_has_its_iverilog_values() {
    let src = r#"
module top;
  string s;
  int a, b, n;
  reg [31:0] h;
  string w;
  initial begin
    s = "12 -34 ff hello";
    n = $sscanf(s, "%d %d %h %s", a, b, h, w);
    $display("A n=%0d a=%0d b=%0d h=%0h w=%0s", n, a, b, h, w);
    s = "nope";
    n = $sscanf(s, "%d", a);
    $display("B n=%0d a=%0d", n, a);
    s = "";
    n = $sscanf(s, "%d", a);
    $display("C n=%0d", n);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A n=4 a=12 b=-34 h=ff w=hello\n".to_string(),
            "out|B n=0 a=12\n".to_string(),
            "out|C n=-1\n".to_string(),
        ],
        "sscanf values"
    );
}

/// A1-iv-b ABSOLUTE ANCHOR — the fd family over a real file, iverilog-pinned.
///
/// Every line below was cross-checked against live iverilog 13.0. The shapes
/// that matter:
///
/// * **A** — `$fopen` with the path AND mode as string NETS, the argument form
///   that reads the store (a literal would not).
/// * **B** — `$fgets` into a `string` dest returns the line length INCLUDING the
///   retained newline (9 for `"alpha 11\n"`).
/// * **C/D** — one byte read, then pushed back: the `$fscanf` at **E** must then
///   see it again, which is what proves the pushback landed in the shared file
///   table rather than in a copy.
/// * **G/H** — EOF is LATCHED by a read that hits it, not predicted: `$feof` is
///   still 0 after the last full line, and only the following short read sets it.
/// * **I** — a bad descriptor gives −1 from both, plus ONE W4022. Reading the fd
///   from the wrong store yields X, whose `has_xz` early-out returns −1 too — but
///   silently, so the warning is the discriminator, not the value.
///
/// ⚠️ `$fclose` was deliberately absent when this was written — it was still in
/// `systask_refusal`, so adding it would have sent the whole design back to the
/// VM and made the anchor vacuous. A5-a wired it; the shape it unblocked (a file
/// opened, read AND closed) is `a_file_testbench_closes_its_file`.
#[test]
fn fd_family_has_its_iverilog_values() {
    let dir = std::env::temp_dir().join(format!("vita-a1ivb-{}-{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let txt = dir.join("f.txt");
    std::fs::write(&txt, "alpha 11\nbeta 22\ngamma\n").expect("input file");
    let out = dir.join("w.txt");
    let src = format!(
        r#"
module top;
  integer fd, c, n, st, wfd;
  reg [8*16:1] line;
  string s;
  string path, mode;
  int a;
  initial begin
    path = "{}"; mode = "r";
    fd = $fopen(path, mode);
    $display("A open=%0d", fd != 0);
    n = $fgets(s, fd);       $display("B n=%0d s=%0s", n, s);
    c = $fgetc(fd);          $display("C c=%0d", c);
    st = $ungetc(c, fd);     $display("D ung=%0d", st);
    n = $fscanf(fd, "%s %d", line, a);
    $display("E n=%0d a=%0d", n, a);
    n = $fgets(s, fd);       $display("F n=%0d", n);
    $display("G eof=%0d", $feof(fd));
    n = $fgets(s, fd);       $display("H n=%0d eof=%0d", n, $feof(fd));
    st = $feof(32'd12345); c = $fgetc(32'd12345);
    $display("I badeof=%0d badgetc=%0d", st, c);
    wfd = $fopen("{}", "w");
    $display("J wopen=%0d", wfd != 0);
    st = $ungetc(65, wfd);   $display("K ung_wo=%0d", st);
    st = $feof(wfd);         $display("L eof_wo=%0d", st);
    st = $feof(32'd999);     $display("M eof_bad2=%0d", st);
    $finish;
  end
endmodule
"#,
        txt.display(),
        out.display()
    );
    let (ir, opts) = build_with_opts(&src);
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A open=1\n".to_string(),
            "out|B n=9 s=alpha 11\n\n".to_string(),
            "out|C c=98\n".to_string(),
            "out|D ung=0\n".to_string(),
            "out|E n=2 a=22\n".to_string(),
            "out|F n=1\n".to_string(),
            "out|G eof=0\n".to_string(),
            "out|H n=6 eof=0\n".to_string(),
            "diag|Warning|VITA-W4022|file operation on invalid/closed descriptor 0x00003039 ignored"
                .to_string(),
            "out|I badeof=-1 badgetc=-1\n".to_string(),
            // J..M were added because two mutations SURVIVED the lines above,
            // and both survivals were this test's blind spots rather than
            // equivalences:
            //   * `$feof`'s bad-fd warning could be deleted, because `$fgetc` on
            //     the SAME bad fd at I warns too and `bad_fd_warn` is once-per-fd
            //     — so M uses a descriptor nothing else touches.
            //   * `$ungetc`'s read-capability check could be deleted, because
            //     every pushback above targets a readable fd — so K pushes onto
            //     a WRITE-ONLY one, where the answer is −1 and, pointedly, NO
            //     warning (iverilog: a write stream is not pushable, silently).
            "out|J wopen=1\n".to_string(),
            "out|K ung_wo=-1\n".to_string(),
            "out|L eof_wo=0\n".to_string(),
            "diag|Warning|VITA-W4022|file operation on invalid/closed descriptor 0x000003e7 ignored"
                .to_string(),
            "out|M eof_bad2=-1\n".to_string(),
        ],
        "fd family values"
    );
}

/// A5-a ABSOLUTE ANCHOR — a file testbench that CLOSES its file.
///
/// This is the shape wiring `$fclose` bought: open, read, close, and keep
/// running natively. Before A5-a the `$fclose` alone sent the whole design to
/// the VM even though every one of its file calls was already wired, which is
/// why the A1-iv-b anchor had to leave it out.
///
/// The fd is a NET at every call, which is the half that was store-dependent:
/// `$fclose` read it with a bare `sched.eval`.
///
/// ⚠️ ONE known divergence, deliberately visible here: iverilog warns on the
/// SECOND `$fclose` of the same descriptor and vita does not, because
/// `bad_fd_warn` is once-per-fd (vita's anti-spam policy, and `$fgetc` already
/// spent the latch on this fd at B). Both vita backends agree, so it is
/// pre-existing rather than this slice's — but it is stated so a future reader
/// does not read the single W4022 as iverilog parity.
#[test]
fn a_file_testbench_closes_its_file() {
    let dir = std::env::temp_dir().join(format!("vita-a5a-{}-{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let txt = dir.join("f.txt");
    std::fs::write(&txt, "alpha 11\nbeta 22\n").expect("input file");
    let src = format!(
        r#"
module top;
  integer fd, n, c;
  string s, path;
  initial begin
    path = "{}";
    fd = $fopen(path, "r");
    n = $fgets(s, fd);   $display("A n=%0d", n);
    $fclose(fd);
    c = $fgetc(fd);      $display("B after_close=%0d", c);
    $fclose(fd);
    $display("C done");
    $finish;
  end
endmodule
"#,
        txt.display()
    );
    let (ir, opts) = build_with_opts(&src);
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    let ev = sink.events.into_inner();
    assert_eq!(
        ev,
        vec![
            "out|A n=9\n".to_string(),
            // the close TOOK: the descriptor is gone, so the read fails loudly.
            "diag|Warning|VITA-W4022|file operation on invalid/closed descriptor 0x80000003 ignored"
                .to_string(),
            "out|B after_close=-1\n".to_string(),
            "out|C done\n".to_string(),
        ],
        "file testbench with $fclose"
    );
}

/// A1-iv-c ABSOLUTE ANCHOR — `$fread`, iverilog-pinned.
///
/// `$fread` is the only family member that reads its own DESTINATION: each
/// element is merged with its prior value, so the untouched slots are what prove
/// the read went to the right store. Every `ffff` below is a prior value that
/// survived, and every `4142`-style pair is one that did not.
///
/// * **A** — a single reg: two bytes, MSB-slot filled.
/// * **B** — a memory with NET-valued `start`/`count` (1 and 2): elements 1..2
///   are written, **m[0] and m[3] keep `ffff`**. Untreaded operands would read X,
///   the x/z-to-0 coercion would make start = 0, and the fill would land in the
///   wrong elements.
/// * **C** — no `start`/`count`: fills from the base until the data runs out, so
///   the last element keeps its prior value and the return counts BYTES (6), not
///   elements.
/// * **D** — at EOF: 0, and nothing is touched.
#[test]
fn fread_has_its_iverilog_values() {
    let dir = std::env::temp_dir().join(format!("vita-a1ivc-{}-{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let bin = dir.join("bin.dat");
    std::fs::write(&bin, "ABCDEFGHIJKL").expect("input file");
    let six = dir.join("six.dat");
    std::fs::write(&six, "ABCDEF").expect("partial-read file");
    let src = format!(
        r#"
module top;
  integer fd, n;
  reg [15:0] r16;
  reg [15:0] m [0:3];
  reg [31:0] w32 [0:1];
  integer i, st, ct, fd2;
  initial begin
    for (i = 0; i < 4; i = i + 1) m[i] = 16'hFFFF;
    r16 = 16'hFFFF;
    fd = $fopen("{}", "r");
    n = $fread(r16, fd);  $display("A n=%0d r=%h", n, r16);
    st = 1; ct = 2;
    n = $fread(m, fd, st, ct);
    $display("B n=%0d m=%h %h %h %h", n, m[0], m[1], m[2], m[3]);
    n = $fread(m, fd);
    $display("C n=%0d m=%h %h %h %h", n, m[0], m[1], m[2], m[3]);
    n = $fread(m, fd);
    $display("D n=%0d", n);
    for (i = 0; i < 2; i = i + 1) w32[i] = 32'hDEADBEEF;
    fd2 = $fopen("{}", "r");
    n = $fread(w32, fd2);
    $display("P n=%0d m0=%h m1=%h", n, w32[0], w32[1]);
    $finish;
  end
endmodule
"#,
        bin.display(),
        six.display()
    );
    let (ir, opts) = build_with_opts(&src);
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A n=2 r=4142\n".to_string(),
            "out|B n=4 m=ffff 4344 4546 ffff\n".to_string(),
            "out|C n=6 m=4748 494a 4b4c ffff\n".to_string(),
            "out|D n=0\n".to_string(),
            // P was added because a mutation SURVIVED A..D: reading each
            // element's PRIOR value from the engine's store instead of this
            // kernel's changed nothing, because every element above is filled
            // COMPLETELY and the prior is overwritten. The merge is only
            // observable on a PARTIAL read — six bytes into two 4-byte
            // elements — where the second element keeps its low half (`beef`).
            "out|P n=6 m0=41424344 m1=4546beef\n".to_string(),
        ],
        "fread values"
    );
}

/// A3-i ABSOLUTE ANCHOR — a subroutine CALL STATEMENT, natively.
///
/// The shape A3-i bought: a task enable and a function-with-output-formal call
/// running on tier-3 instead of sending the whole design to the VM. Every line
/// below is one of the two halves that was store-dependent, chosen so that the
/// engine's store answering instead of tier-3's changes the printed value:
///
/// * **A** — net-valued actuals into a subset TASK, copied out to ARRAY ELEMENTS.
///   `200+57` wraps to 1 in 8 bits and `200-57` is 143. A copy-in that read the
///   engine's nets would evaluate both actuals as X (tier-3 never writes there),
///   so both would print `x`.
/// * **B** — a FUNCTION with an output formal: one call statement copies out the
///   formal AND the return slot. `twice = 200*2 = 400` in a 32-bit `integer`, and
///   the return is `201`.
/// * **C** — a NARROWING formal (`input [3:0]` taking an 8-bit actual) whose
///   destination is indexed by a NET. `200 = 8'hC8`, low nibble `8`. A copy-in
///   sized self-determined rather than to the formal would print `200`; a
///   destination resolved against the wrong store would miss `mem[2]`.
/// * **D** — a SIGNED formal taking a negative actual: `-(-5) = 5`. Unsigned
///   copy-in prints `251`.
/// * **E** — the same task again, with the RESULTS of A as its actuals, proving
///   the copy-out landed where the next copy-in reads: `1+143 = 144` and
///   `1-143 = -142` → `114` in 8 bits.
/// * **F** — BOTH output formals aliased onto ONE destination. The copy-outs run
///   in `out_binds` order and the last wins, so `q = 143`. This is the only line
///   that can see the ORDER at all — every destination above is distinct, so a
///   reversed copy-out loop prints the same thing for A through E.
/// * **G** — the destinations ARE the input actuals. IEEE §13.4.1 evaluates the
///   copy-in before the body runs, so `a` and `b` still read 200 and 57 inside
///   and the results land afterwards: `a = 1`, `b = 143`. A copy-in deferred to
///   after the call would read the already-overwritten nets.
/// * **H, I** — a formal WIDER than the actual expression, whose value overflows
///   the actual's own self-width. §13.4.3 makes the formal a variable of its
///   declared type, so the actual is an assignment context of the formal's width:
///   `200 + 57` is 257 in a 16-bit formal, not 1. And signed: `-3 - 126` is −129
///   in a signed 16-bit formal, not 127.
///
///   ⚠️ **These two exist because the first battery could not kill the copy-in
///   width and sign.** Every other line above is invariant to them, because
///   `run_task`'s `bind_formal` RE-BINDS each actual to the formal's declared type
///   on frame entry — so the context passed to `k_eval_ctx` only matters when
///   evaluating at the narrower width would have already destroyed the value. That
///   is exactly this shape, and nothing else in the anchor had it.
///
/// ⭐ **Absolute, not differential, and this slice is exactly why** (ROADMAP
/// §5.1-e): A3-i delegates the callee body to `SimState::run_task_call` — the
/// same function the engine runs — so a native-vs-VM comparison is blind to
/// everything inside it. A/C/D/E are iverilog-pinned; **B is hand-IEEE**, because
/// iverilog rejects an output formal on a FUNCTION (`port twice is not an input
/// port`) while IEEE §13.4.1 allows it and vita supports it.
#[test]
fn a_subset_task_call_has_its_iverilog_values() {
    let src = r#"
module top;
  reg [7:0] a, b, q;
  reg [7:0] mem [0:3];
  integer   idx;
  integer   r, o1;
  reg signed [7:0] sv;
  integer   sout;
  reg [15:0] r16;
  reg signed [7:0] sm;
  integer   sw;

  task automatic addsub(input [7:0] x, input [7:0] y,
                        output [7:0] s, output [7:0] d);
    begin s = x + y; d = x - y; end
  endtask

  function automatic integer scale(input integer v, output integer twice);
    begin twice = v * 2; scale = v + 1; end
  endfunction

  task automatic lownib(input [3:0] n, output [7:0] z);
    begin z = {4'h0, n}; end
  endtask

  task automatic negate(input signed [7:0] s, output integer o);
    begin o = -s; end
  endtask

  // A formal WIDER than the actual expression's own self-width.
  task automatic wide(input [15:0] x, output [15:0] o); begin o = x; end endtask
  task automatic widesig(input signed [15:0] x, output integer o); begin o = x; end endtask

  initial begin
    a = 8'd200; b = 8'd57; idx = 2; sv = -8'sd5;
    addsub(a, b, mem[0], mem[1]);
    $display("A mem0=%0d mem1=%0d", mem[0], mem[1]);
    r = scale(a, o1);
    $display("B r=%0d o1=%0d", r, o1);
    lownib(a, mem[idx]);
    $display("C mem2=%0d", mem[2]);
    negate(sv, sout);
    $display("D sout=%0d", sout);
    addsub(mem[0], mem[1], mem[2], mem[3]);
    $display("E mem2=%0d mem3=%0d", mem[2], mem[3]);
    addsub(a, b, q, q);
    $display("F q=%0d", q);
    addsub(a, b, a, b);
    $display("G a=%0d b=%0d", a, b);
    a = 8'd200; b = 8'd57;
    wide(a + b, r16);
    $display("H r16=%0d", r16);
    sm = -8'sd3;
    widesig(sm - 8'sd126, sw);
    $display("I sw=%0d", sw);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A mem0=1 mem1=143\n".to_string(),
            "out|B r=201 o1=400\n".to_string(),
            "out|C mem2=8\n".to_string(),
            "out|D sout=5\n".to_string(),
            "out|E mem2=144 mem3=114\n".to_string(),
            "out|F q=143\n".to_string(),
            "out|G a=1 b=143\n".to_string(),
            "out|H r16=257\n".to_string(),
            "out|I sw=-129\n".to_string(),
        ],
        "subset call-statement values"
    );
}

/// A3-i DIFFERENTIAL — call-statement shapes, native vs the VM.
///
/// The second lens beside `a_subset_task_call_has_its_iverilog_values`. It is
/// deliberately the WEAKER of the two here (ROADMAP §5.1-e): the callee body runs
/// in `SimState::run_task_call` on both backends, so this pass is blind to
/// anything inside it and can only see the copy-in, the copy-out and the site's
/// control flow. Those are exactly the three pieces A3-i wrote, which is what
/// makes it worth running.
///
/// Each design isolates one axis of the copy-in/copy-out that a wrong store
/// would answer differently.
#[test]
fn a3i_call_statement_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // A call inside a LOOP: the frame is fresh per activation and the actual
        // changes every iteration, so a copy-in reading a stale store prints a
        // constant.
        (
            "call in a loop",
            r#"
module top;
  integer i, acc, dbl;
  task automatic twice(input integer v, output integer o); begin o = v + v; end endtask
  initial begin
    acc = 0;
    for (i = 1; i <= 5; i = i + 1) begin twice(i, dbl); acc = acc + dbl; end
    $display("acc=%0d dbl=%0d", acc, dbl);
    $finish;
  end
endmodule
"#,
        ),
        // A call whose actual is the DESTINATION of the previous call — the
        // copy-out must be visible to the next copy-in through the same store.
        (
            "chained call",
            r#"
module top;
  reg [15:0] a, t1, t2;
  task automatic shl(input [15:0] v, output [15:0] o); begin o = v << 1; end endtask
  initial begin
    a = 16'h00A5;
    shl(a, t1); shl(t1, t2);
    $display("t1=%h t2=%h", t1, t2);
    $finish;
  end
endmodule
"#,
        ),
        // A call under a BRANCH, so the walk's `ret_bb` has to be the block the
        // rest of the body continues from rather than the next index.
        (
            "call under a branch",
            r#"
module top;
  reg [7:0] sel, out;
  task automatic pick(input [7:0] v, output [7:0] o); begin o = v ^ 8'hFF; end endtask
  initial begin
    sel = 8'd3; out = 8'd0;
    if (sel > 8'd1) pick(sel, out); else out = 8'd9;
    $display("out=%0d", out);
    $finish;
  end
endmodule
"#,
        ),
        // A call in a CLOCKED process, so it runs once per edge and the copy-out
        // feeds the dirty channel that wakes the next process.
        (
            "call in an always block",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] n = 8'd0, m;
  task automatic inc(input [7:0] v, output [7:0] o); begin o = v + 8'd1; end endtask
  always #1 clk = ~clk;
  always @(posedge clk) begin inc(n, m); n <= m; end
  initial begin #9 $display("n=%0d m=%0d", n, m); $finish; end
endmodule
"#,
        ),
        // NO output formal at all — the call is pure input, so the copy-out list
        // is empty and the only observable is that the frame ran and the walk
        // continued. (`$display` inside would make it suspendable, so the proof
        // is that the design still RUNS and the following statement executes.)
        (
            "input-only task",
            r#"
module top;
  integer sink;
  task automatic ignore(input integer v); begin sink = v; end endtask
  initial begin sink = 0; ignore(7); $display("sink=%0d", sink); $finish; end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    // ⚠️ The LAST design was refused when this test was written and RUNS now:
    // `ignore` writes the module net `sink`, which made it "suspendable", and
    // A3-ii-a drives exactly that shape. The assertion moved with the gate rather
    // than being deleted — asserting the split is what stops this test from going
    // quiet if the gate ever widens or narrows underneath it again.
    assert_eq!(ran, 5, "A3-i differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "A3-i differential: refusal breakdown moved"
    );
}

/// A2-i DIFFERENTIAL — plain-OOP shapes, native vs the VM.
///
/// ⚠️ Weaker than the anchor above BY CONSTRUCTION and kept for what it does
/// see: `class_heap`, `class_layouts` and the warn latch are all `SimState`
/// objects both backends borrow, so everything below the HANDLE is shared and
/// this comparison is blind to it (ROADMAP §5.1-e). What it can distinguish is
/// exactly what this slice routes — where the handle id is read from and where
/// a field write lands — plus the interaction of a class net with the dirty
/// channel, which the anchor's single `initial` block cannot reach.
#[test]
fn a2i_class_field_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // A field read in a CLOCKED process. The handle net is written at t0 and
        // the field every edge, so a store-blind read shows up as a frozen value
        // rather than as X.
        (
            "field in an always block",
            r#"
module top;
  class C; int v; endclass
  C c;
  reg clk = 1'b0;
  reg [7:0] seen = 8'd0;
  always #1 clk = ~clk;
  always @(posedge clk) begin c.v = c.v + 1; seen <= c.v; end
  initial begin c = new(); #9 $display("v=%0d seen=%0d", c.v, seen); $finish; end
endmodule
"#,
        ),
        // A handle net inside an ARRAY-INDEX expression: the field value picks
        // the element, so a wrong handle picks a wrong element rather than X.
        (
            "field as an array index",
            r#"
module top;
  class C; int i; endclass
  C c;
  reg [7:0] mem [0:3];
  reg [7:0] got;
  integer k;
  initial begin
    for (k = 0; k < 4; k = k + 1) mem[k] = 8'd10 + k;
    c = new(); c.i = 2;
    got = mem[c.i];
    $display("got=%0d", got);
    $finish;
  end
endmodule
"#,
        ),
        // TWO objects alive at once, so an implementation that kept one field
        // set per handle NET (rather than per OBJECT) crosses them.
        (
            "two live objects",
            r#"
module top;
  class C; int v; endclass
  C a, b;
  initial begin
    a = new(); b = new();
    a.v = 3; b.v = 9;
    $display("a=%0d b=%0d sum=%0d", a.v, b.v, a.v + b.v);
    $finish;
  end
endmodule
"#,
        ),
        // A field written from inside a METHOD — the delegated `&self` frame
        // executor, whose `this` is a frame-local net holding the handle.
        (
            "field written by a method",
            r#"
module top;
  class C;
    int v;
    function int add(input int d); v = v + d; return v; endfunction
  endclass
  C c; int r;
  initial begin c = new(); c.v = 4; r = c.add(6); $display("r=%0d v=%0d", r, c.v); $finish; end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 4, "A2-i differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "A2-i differential: refusal breakdown moved"
    );
}

/// A3-ii-a ABSOLUTE ANCHOR — a DRIVEN task frame, iverilog-pinned.
///
/// A3-i delegated a subset call to the engine's `&self` executor. This is the
/// other half: a task the engine would drive from a `FrameRec` — it prints, it
/// writes a module net — but which reaches no `Delay`/`Wait`/`Fork`, so the tier-3
/// walk runs its CFG itself and the whole activation nests inside one `run_body`.
///
/// Every line is a piece of that walk, chosen so a frame-blind path prints
/// something else:
///
/// * **A** — a `$display` INSIDE the frame reading its own formals, and a write to
///   a MODULE net from inside the frame. The two go to different stores, and the
///   first three drafts of this slice got exactly one of them right at a time.
/// * **B** — a NESTED driven frame, whose caller has a frame-local (`t = 99`) that
///   the callee must not alias: `outer` sees 21, not 99, and not the caller's.
/// * **C** — RECURSION, four deep, with a frame-local accumulator per activation.
///   `10` is `1+2+3+4`; `hits=4` counts the activations that recursed.
/// * **D** — control flow inside the frame: a `for` and an `if`. `18` is
///   `1+2+4+5+6` (3 skipped), so a branch taken the wrong way is visible.
/// * **E** — the copy-out destination is an ARRAY ELEMENT and the actual is a net.
///
/// ⭐ **Absolute, not differential** (ROADMAP §5.1-e). The frame window, the dyn
/// heap and the diagnostic sink are all `SimState` objects both backends share, so
/// a native-vs-VM comparison cannot see most of what this slice wrote. It is
/// iverilog-pinned end to end — every value below is `vvp`'s.
#[test]
fn a_driven_task_frame_has_its_iverilog_values() {
    let src = r#"
module top;
  reg [7:0] a, b, c;
  integer g, hits;
  reg [7:0] mem [0:3];

  task automatic show(input [7:0] x, output [7:0] y);
    begin y = x + 8'd1; $display("  show x=%0d y=%0d", x, y); g = g + x; end
  endtask

  task automatic outer(input [7:0] x, output [7:0] y);
    reg [7:0] t;
    begin t = 8'd99; show(x, t); $display("  outer t=%0d", t); y = t + 8'd10; end
  endtask

  task automatic down(input integer n, output integer acc);
    integer inner;
    begin
      if (n <= 0) acc = 0;
      else begin down(n - 1, inner); acc = inner + n; hits = hits + 1; end
    end
  endtask

  task automatic sum_to(input integer n, output integer s);
    integer i;
    begin
      s = 0;
      for (i = 1; i <= n; i = i + 1) if (i != 3) s = s + i;
    end
  endtask

  integer r1, r2;
  initial begin
    g = 0; hits = 0;
    show(8'd5, b);            $display("A b=%0d g=%0d", b, g);
    outer(8'd20, c);          $display("B c=%0d g=%0d", c, g);
    down(4, r1);              $display("C r1=%0d hits=%0d", r1, hits);
    sum_to(6, r2);            $display("D r2=%0d", r2);
    a = 8'd7; show(a, mem[2]); $display("E mem2=%0d g=%0d", mem[2], g);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|  show x=5 y=6\n".to_string(),
            "out|A b=6 g=5\n".to_string(),
            "out|  show x=20 y=21\n".to_string(),
            "out|  outer t=21\n".to_string(),
            "out|B c=31 g=25\n".to_string(),
            "out|C r1=10 hits=4\n".to_string(),
            "out|D r2=18\n".to_string(),
            "out|  show x=7 y=8\n".to_string(),
            "out|E mem2=8 g=32\n".to_string(),
        ],
        "driven task-frame values"
    );
}

/// A2-i ABSOLUTE ANCHOR — plain OOP, **iverilog-pinned**.
///
/// ⭐ And iverilog really is the oracle here, which was not the expectation:
/// this repo has treated the class surface as no-oracle territory since N7.
/// `iverilog 13` compiles and runs SystemVerilog classes, so every value below
/// is `vvp`'s rather than hand-IEEE — a stronger anchor than the slice planned
/// for. (Its ONE gap is measured and excluded: a null-handle dereference
/// SEGFAULTS `ivl` at compile time, so `nul.x` is pinned in its own test
/// against the LRM instead. Putting a known divergence in an anchor's expected
/// output is what stops it being an anchor — §4.5.302.)
///
/// Each line is a routing site this slice had to open, chosen so a store-blind
/// path prints something else:
///
/// * **A/B** — a field WRITE then READ on a freshly `new`ed object. The three
///   ways to get this wrong are all distinct: the write landing in the handle
///   net's own slot (the `write_routed` lane), the read taking the arena's
///   array-word path (the `read_net` lane), and the HANDLE being read from the
///   engine's store, which a native run leaves at t0 `null` (`handle_id_with`).
/// * **C** — the field in ARITHMETIC and into an ARRAY ELEMENT. The first is
///   what `wprog` must decline (it resolves a `Signal` to a slot at compile
///   time and cannot route); the second proves the ordinary funnel still works
///   beside the class lane.
/// * **D** — the field's own WIDTH and SIGN, not the handle's 32/unsigned:
///   `8'sd200` into a `byte` is `-56`, `4'd15` into a `bit [3:0]` is `15`.
/// * **E/H** — REFERENCE semantics. Two handles name one object (`q.x = 11`
///   moves `p.x`), and a second `new` leaves the old handle on the old object.
///   A design that stored fields in the handle's slot passes every line above
///   and fails these two.
/// * **F** — a METHOD, reading and writing `this` fields, INCLUDING one called
///   from inside a `$display` argument — the path that reaches the formatter's
///   `HeapRouted` wrapper rather than the kernel's own reader.
/// * **I** — the field driven in a LOOP, so its value moves well after t0.
/// * **J/K** — ⚠️ **added because the first battery left two mutations alive**,
///   and both were test-design gaps rather than equivalences (§5.1-e again).
///   **J** writes a 4-state value into a 2-STATE field beside a 4-state one, so
///   the §6.11.3 coercion is the only difference (`4'bx1z0` → `0100` in `bit
///   [3:0]`, unchanged in `logic [3:0]`); nothing above it ever put an X in a
///   field. **K** writes a 64-bit value into an `int` and a `byte`, so the
///   RESIZE to the field's own width and sign is observable — every line above
///   assigns a value the width table has already sized to the field, which makes
///   the resize a no-op there.
#[test]
fn a_plain_oop_design_has_its_iverilog_values() {
    let src = r#"
module top;
  class Pt;
    int      x;
    byte     b;
    bit [3:0] n;
    logic [3:0] l;
    function int bump(input int d); x = x + d; return x; endfunction
    function int get(); return x; endfunction
  endclass

  Pt p, q;
  int  r, s;
  reg [7:0] arr [0:3];
  reg [3:0] xs;
  reg [63:0] big;
  integer i;

  initial begin
    p = new();
    $display("A x=%0d b=%0d n=%0d", p.x, p.b, p.n);
    p.x = 7; p.b = -3; p.n = 4'hD;
    $display("B x=%0d b=%0d n=%0d", p.x, p.b, p.n);
    r = p.x + 1;
    arr[2] = p.x + 2;
    $display("C r=%0d arr2=%0d", r, arr[2]);
    p.b = 8'sd200;
    p.n = 4'd15;
    $display("D b=%0d n=%0d", p.b, p.n);
    q = p;
    q.x = 11;
    $display("E px=%0d qx=%0d", p.x, q.x);
    s = p.bump(5);
    $display("F s=%0d px=%0d get=%0d", s, p.x, p.get());
    p = new();
    p.x = 1;
    $display("H px=%0d qx=%0d", p.x, q.x);
    for (i = 0; i < 4; i = i + 1) q.x = q.x + i;
    $display("I qx=%0d", q.x);
    xs = 4'bx1z0;
    p.n = xs; p.l = xs;
    $display("J n=%b l=%b", p.n, p.l);
    big = 64'h1234_5678_9ABC_DEF0;
    p.x = big; p.b = big;
    $display("K x=%0d xh=%h b=%0d", p.x, p.x, p.b);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "must run natively or this anchor proves nothing (refused: {:?})",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A x=0 b=0 n=0\n".to_string(),
            "out|B x=7 b=-3 n=13\n".to_string(),
            "out|C r=8 arr2=9\n".to_string(),
            "out|D b=-56 n=15\n".to_string(),
            "out|E px=11 qx=11\n".to_string(),
            "out|F s=16 px=16 get=16\n".to_string(),
            "out|H px=1 qx=16\n".to_string(),
            "out|I qx=22\n".to_string(),
            "out|J n=0100 l=x1z0\n".to_string(),
            "out|K x=-1698898192 xh=9abcdef0 b=-16\n".to_string(),
        ],
        "plain-OOP values (iverilog 13 pinned)"
    );
}

/// A8-b ABSOLUTE ANCHOR — DEFERRED assertions (§16.4), hand-IEEE.
///
/// ⭐ Another conservative row. §16.4.3 renders a deferred action's text at
/// REACH, so what is enqueued is a `String` and `mature_deferred` reads no net;
/// the one store-bound line is the render inside `try_defer`, which
/// `dispatch_with` has threaded since S1d-4b. What tier-3 lacked was the two
/// REGIONS — Observed and Reactive, in that order, after the timestep's buckets
/// empty and before the postponed drain — plus the termination drain.
///
/// ⚠️ Hand-IEEE: `iverilog 13` refuses deferred assertions outright ("sorry:
/// Deferred assertions are not supported").
///
/// What the values pin:
///
/// * the report carries the values it saw at REACH (`tag=20` beside `q=2`),
///   not the ones the net holds when the region matures — §16.4.3's whole
///   point, and the half that goes wrong if the message were rendered late.
/// * the FLUSH-ON-RE-REACH: the assertion is re-reached at every posedge and
///   passes until `q` reaches 2, so exactly ONE report survives rather than one
///   per edge.
/// * an `assert final` (Reactive) beside an `assert #0` (Observed) in the same
///   block, and — ⚠️ **this is the half the first battery caught** — the
///   Reactive one must actually FAIL. With `q < 4'd3` it never did, so a
///   mutation that matures Observed and drops Reactive survived the whole
///   suite; `q < 4'd1` makes it report at two different edges, and the ORDER
///   (`R q=1` before `O q=2` before `R q=2`) is what shows both queues drain in
///   their own regions rather than one draining twice.
/// * the `$display` at `#7` lands AFTER the matured report — Active before
///   Observed.
#[test]
fn deferred_assertions_mature_in_their_regions_on_tier_3() {
    let src = r#"
module t;
  reg clk = 1'b0;
  reg [3:0] q = 4'd0;
  reg [7:0] tag = 8'd0;
  always #1 clk = ~clk;
  always @(posedge clk) begin q <= q + 4'd1; tag <= tag + 8'd10; end
  always @(posedge clk) begin
    assert #0 (q < 4'd2) else $error("O q=%0d tag=%0d", q, tag);
    assert final (q < 4'd1) else $error("R q=%0d tag=%0d", q, tag);
  end
  initial begin #7 $display("done q=%0d", q); $finish; end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "diag|Error|VITA-E4003|R q=1 tag=10".to_string(),
            "diag|Error|VITA-E4003|O q=2 tag=20".to_string(),
            "diag|Error|VITA-E4003|R q=2 tag=20".to_string(),
            "out|done q=3\n".to_string(),
        ],
        "deferred assertions (hand-IEEE §16.4; iverilog refuses them)"
    );
}

/// A8-b DIFFERENTIAL — deferred shapes, native against the VM.
#[test]
fn a8b_deferred_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // A deferred action in the SAME SLOT as the `$finish`, which reaches the
        // termination drain rather than the region cascade.
        (
            "deferred report in the finish slot",
            r#"
module top;
  reg [7:0] a = 8'd9;
  initial begin
    #1;
    assert #0 (a < 8'd5) else $error("F a=%0d", a);
    $display("D a=%0d", a);
    $finish;
  end
endmodule
"#,
        ),
        // A deferred report that MATURES INTO A TERMINATION (`$fatal`), so the
        // region's own `Some(step)` path runs rather than the body's.
        (
            "deferred fatal",
            r#"
module top;
  reg clk = 1'b0;
  reg [3:0] q = 4'd0;
  always #1 clk = ~clk;
  always @(posedge clk) q <= q + 4'd1;
  always @(posedge clk) assert #0 (q < 4'd2) else $fatal(1, "FT q=%0d", q);
  initial #20 $finish;
endmodule
"#,
        ),
        // A plain `$display` deferred action (no severity) — the other arm of
        // `mature_deferred`, which writes stdout rather than a diagnostic.
        (
            "deferred display action",
            r#"
module top;
  reg clk = 1'b0;
  reg [3:0] q = 4'd0;
  always #1 clk = ~clk;
  always @(posedge clk) q <= q + 4'd1;
  always @(posedge clk) assert #0 (q < 4'd2) else $display("P q=%0d", q);
  initial #7 $finish;
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 3, "A8-b differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "A8-b differential: refusal breakdown moved"
    );
}

/// A3-iv ABSOLUTE ANCHOR — a frame task with a HIERARCHICAL enable,
/// iverilog-pinned.
///
/// ⚠️⚠️ The row this replaces refused every such design for a reason about the
/// WRONG PHASE. `has_hier_call` says a deferred hierarchical enable's
/// `Call.target` is a placeholder until the finish-phase resolve, so
/// `frame_suspends` cannot see through it — true inside ELABORATE, where
/// `force_suspend` exists precisely because its `compute_suspendable_tasks`
/// runs before the patch. This gate runs in `simulate`, after it. Instrumented
/// over every design that reached the row: **zero** unresolvable targets.
///
/// Each line needs the resolution to be real and per-instance:
///
/// * **A** — TWO instances of the same module, each accumulating its own total
///   through the same task. A resolution that collapsed them prints one number
///   twice; `6` and `12` come apart only if `u1.add` and `u2.add` reach
///   different windows.
/// * **B** — a hier enable with an OUTPUT formal, so the copy-out crosses the
///   instance boundary in the other direction, from two different instances.
/// * **C** — the callee's own module net is CHANGED between two identical
///   calls (`u1.base = 7`), which is what makes this non-vacuous: a run reading
///   a stale store returns `101` again, and `101` is a value the design really
///   had a moment earlier.
#[test]
fn a_hierarchical_enable_from_a_frame_task_has_its_iverilog_values() {
    let src = r#"
module sub;
  integer acc;
  integer base;
  task automatic add(input integer x); acc = acc + x; endtask
  task automatic get(input integer x, output integer y); y = base + x; endtask
  initial begin acc = 0; base = 100; end
endmodule
module top;
  sub u1(); sub u2();
  integer a, b;
  task automatic drive(input integer n);
    integer i;
    begin for (i = 0; i < n; i = i + 1) begin u1.add(i); u2.add(2 * i); end end
  endtask
  task automatic two(output integer p, output integer q);
    begin u1.get(1, p); u2.get(2, q); end
  endtask
  initial begin
    #1 drive(4);
    $display("A acc1=%0d acc2=%0d", u1.acc, u2.acc);
    two(a, b);
    $display("B a=%0d b=%0d", a, b);
    u1.base = 7;
    two(a, b);
    $display("C a=%0d b=%0d", a, b);
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A acc1=6 acc2=12\n".to_string(),
            "out|B a=101 b=102\n".to_string(),
            "out|C a=8 b=102\n".to_string(),
        ],
        "hierarchical enable from a frame task (iverilog 13 pinned)"
    );
}

/// A3-iv's REJECT neighbour, and the whole argument for deleting the row: a hier
/// callee that PARKS is still refused — by `frame_suspends`, which sees through
/// the resolved target and says so in its own words.
///
/// This is what makes the deletion a narrowing rather than a hole. The `None`
/// arm of that walk still fails CLOSED if a target ever does arrive
/// unresolved; what changed is that today none do.
#[test]
fn a_hierarchical_enable_whose_callee_parks_is_still_refused() {
    use sim_engine::native::arena::NetArena;
    let src = r#"
module sub;
  integer acc;
  task automatic addd(input integer x); begin #1 acc = acc + x; end endtask
  initial acc = 0;
endmodule
module top;
  sub u1();
  task automatic drive(input integer n);
    integer i;
    begin for (i = 0; i < n; i = i + 1) u1.addd(i); end
  endtask
  initial begin drive(3); $display("acc=%0d", u1.acc); end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        Some("a task frame that SUSPENDS (delay, wait or fork inside the body): S3b"),
        "a PARKING hier callee must still be refused, and by the row that owns \
         the question rather than by a stand-in"
    );
}

/// A3-iii ABSOLUTE ANCHOR — a DELEGATED function body that READS module nets,
/// iverilog-pinned.
///
/// S3a's precondition was "an admitted subroutine body names NO net outside its
/// own frame window", and its argument was exact for the executor it had: a
/// plain function reached through `Expr::Call` runs in `SimState`'s own `&self`
/// frame executor, which reads module nets from the store a native run never
/// writes — so such a body would read the t0 value at exit 0.
///
/// A3-ii-a already showed the DRIVEN half of this is fine (a task the walk runs
/// itself reads through the kernel). This is the DELEGATED half, and the fix is
/// to hand that executor the caller's store: `HeapRouted` then splits it, a
/// frame slot coming back from the activation window and a module net from the
/// arena.
///
/// Each line is a different read position, because they reach different sites:
///
/// * **`add_g`** — a module net in an ARITHMETIC rhs (`frame_rhs_value`).
/// * **`pick`** — a module MEMORY element, so the read carries an index.
/// * **`branchy`** — a module net in the BRANCH CONDITION, which is a separate
///   site (`truthy`) and the one a rhs-only threading would miss.
/// * **`A` vs `B`** — the module net CHANGES between the two calls. This is what
///   makes the anchor non-vacuous: a body reading the engine's untouched store
///   returns the same answer both times, and `g = 5` at t0 is a value the design
///   really has, so the first line alone would look right.
/// * **`C`** — the call is a `$display` ARGUMENT, which reaches the executor
///   through the formatter's `HeapRouted` rather than through the kernel.
#[test]
fn a_delegated_body_reads_module_nets_through_the_callers_store() {
    let src = r#"
module t;
  reg [7:0] g;
  reg [7:0] mem [0:3];
  reg [7:0] r1, r2, r3;
  integer i;
  function automatic [7:0] add_g(input [7:0] x);
    add_g = x + g;
  endfunction
  function automatic [7:0] pick(input [1:0] k);
    pick = mem[k];
  endfunction
  function automatic [7:0] branchy(input [7:0] x);
    if (g > 8'd10) branchy = x + 8'd1; else branchy = x + 8'd100;
  endfunction
  initial begin
    g = 8'd5;
    for (i = 0; i < 4; i = i + 1) mem[i] = 8'd10 + i[7:0];
    r1 = add_g(8'd7);
    r2 = pick(2'd2);
    r3 = branchy(8'd1);
    $display("A r1=%0d r2=%0d r3=%0d", r1, r2, r3);
    g = 8'd50;
    r1 = add_g(8'd7);
    r3 = branchy(8'd1);
    $display("B r1=%0d r3=%0d", r1, r3);
    $display("C in-arg=%0d", add_g(8'd0));
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A r1=12 r2=12 r3=101\n".to_string(),
            "out|B r1=57 r3=2\n".to_string(),
            "out|C in-arg=50\n".to_string(),
        ],
        "delegated body reading module nets (iverilog 13 pinned)"
    );
}

/// A3-iii's SECOND executor — a function with an OUTPUT FORMAL, reached through
/// `Terminator::Call` rather than `Expr::Call`, iverilog-unpinnable and so
/// hand-traced.
///
/// ⚠️⚠️ **This test exists because narrowing the gate exposed a pre-existing
/// silent-wrong, and only the FLIP RUN found it.** S3a's row refused any
/// subroutine body naming an out-of-window net, which covered BOTH delegated
/// executors at once: `run_frame_call` (an expression call) and `run_task` (the
/// A3-i subset path). Threading the first and lifting the row left the second
/// reading the engine's untouched store, so `getnext` returned 0 on its first
/// call, the `while` never entered its body, and the design printed its final
/// line and nothing else — exit 0, no diagnostic.
///
/// The whole suite was green on that design, because its own test runs on the
/// default backend. Flipping the default is what surfaced it; that is the third
/// time this repository has measured that (V1 slice 2d, A2-i, here).
///
/// ORACLE: `iverilog 13` rejects a function with an output port outright, so
/// the values are hand-traced from `src = '{10,20,30,0}` — three iterations,
/// then `src[3] == 0` ends the loop.
#[test]
fn a_subset_task_call_reads_module_nets_through_the_callers_store() {
    let src = r#"
module t;
  int src[4] = '{10,20,30,0};
  int sel;
  function automatic int getnext (input int fd, output int val);
    int loc[4];
    begin
      // A frame-local ARRAY at a MODULE-NET index, and a BRANCH on a module
      // net. Both were added because the first battery left them alive, and
      // both then found REAL divergences rather than just killing a mutation:
      // the write-side index resolved against the engine's store in TWO more
      // places (`frame_or_class_write`, and the element read-modify-write
      // inside `frame_write_lvalue`), so `loc[sel] = src[fd]` landed in
      // `loc[0]` and read back 0 — `v=0 v=0 v=0` at exit 0.
      loc[sel] = src[fd];
      if (src[fd] != 0) val = loc[sel]; else val = -1;
      getnext = (src[fd] != 0);
    end
  endfunction
  initial begin
    int i = 0, v;
    sel = 2;
    while (getnext(i, v) == 1) begin $display("v=%0d", v); i++; end
    $display("PASS v=%0d", v);
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|v=10\n".to_string(),
            "out|v=20\n".to_string(),
            "out|v=30\n".to_string(),
            "out|PASS v=-1\n".to_string(),
        ],
        "a subset task call reading a module array (hand-IEEE; iverilog rejects output ports)"
    );
}

/// A3-iii's REJECT neighbour: a body that WRITES a module net is still refused,
/// and by the STORAGE gate in its own words.
///
/// ⚠️ This is what stops the narrowing from being a silent-wrong. Threading a
/// READER lifts the read half; the write half cannot be lifted the same way,
/// because every destination in that executor goes through
/// `SimState::frame_write_lvalue`, which is `&self` on the engine's state and
/// has no way to reach the caller's arena. Admitting one would put the write in
/// a dead store at exit 0.
///
/// Measured before narrowing: 22 of the 26 designs the old row blocked only
/// read out of window; 4 also write.
///
/// ⚠️ Finding a design that reaches this row took a probe, and the answer is
/// worth writing down: a plain `g = g + 1` on a module net is refused a phase
/// EARLIER (elaborate E3009 — "an assignment to a net outside the function …
/// is outside the frame-call subset"), so the row is unreachable that way. What
/// does reach it is a CLASS FIELD write, `c.v = …`, whose lvalue chunk names the
/// module-scope HANDLE net. Building the reject case from the obvious source
/// shape would have produced a test that passes because elaborate refused it.
#[test]
fn a_delegated_body_that_writes_a_module_net_is_refused() {
    use sim_engine::native::arena::NetArena;
    let src = "module t;\n\
         class C; int v; endclass\n\
         C c;\n\
         function automatic int bump(input int x);\n\
           begin c.v = c.v + x; bump = c.v; end\n\
         endfunction\n\
         int r;\n\
         initial begin c = new(); c.v = 1; r = bump(2); $display(\"%0d\", r); end\n\
       endmodule\n";
    let (ir, opts) = build_with_opts(src);
    assert_eq!(
        NetArena::buildable(&ir, &opts).err(),
        Some("a subroutine that WRITES a net outside its own frame: S3b"),
        "an out-of-window WRITE from a delegated body must stay refused"
    );
    // …and the READ-only neighbour must build, or the row is refusing the wrong
    // thing. The same design with the field write removed.
    let src2 = "module t;\n\
         class C; int v; endclass\n\
         C c;\n\
         function automatic int peek(input int x);\n\
           peek = c.v + x;\n\
         endfunction\n\
         int r;\n\
         initial begin c = new(); c.v = 1; r = peek(2); $display(\"%0d\", r); end\n\
       endmodule\n";
    let (ir2, opts2) = build_with_opts(src2);
    assert_eq!(
        NetArena::buildable(&ir2, &opts2).err(),
        None,
        "the read-only neighbour must build"
    );
}

/// A2-ii ABSOLUTE ANCHOR — CRV (`randomize()`), hand-IEEE + a PROPERTY.
///
/// ⭐ The whole CRV surface's store dependence was ONE line — measured rather
/// than assumed: `every_untreaded_store_read_in_builtins_sits_behind_a_reject_row`
/// counts four untreaded reads in `crv_draw.rs` and the other three belong to
/// `$writemem*`. That one line was `class_randomize_run`'s RECEIVER, read
/// through `eval_ctx_top` — the engine's nets — so on a native run the handle
/// came back `0`, `randomize()` took the null arm, returned 0, and touched no
/// field. Everything below the handle (`class_heap`, the four per-class tables,
/// the inline-`with` overrides, the RNG) is `SimState` and needed nothing.
///
/// ⚠️ Hand-IEEE: `iverilog 13` rejects constraint declarations outright
/// ("sorry: Constraint declarations not supported"), so the values below are
/// §18's, and the assertion is deliberately a PROPERTY rather than a literal
/// draw sequence — pinning `a = 14` would pin this repo's LCG rather than the
/// language. What is pinned:
///
/// * **`ok=1` every time** — with a satisfiable constraint set the solver must
///   succeed. A store-blind receiver read returns 0 here, which is what makes
///   this line the primary discriminator.
/// * **`inA`/`inB`** — every draw lands inside its declared `inside {[lo:hi]}`
///   range. Two SEPARATE ranges, so a solve that ignored one constraint and
///   satisfied the other still shows.
/// * **`randc` is a PERMUTATION** — the four draws of a 2-bit `randc` field must
///   visit 0..3 once each before repeating (§18.6). A plain uniform draw passes
///   `ok` and `inA`/`inB` and fails this.
/// * **the status lands in the destination NET** — `r = p.randomize()` writes
///   through the funnel-outside sink, which is the second half of the slice.
#[test]
fn randomize_draws_within_its_constraints_on_tier_3() {
    let src = r#"
module t;
  class P;
    rand int unsigned a;
    rand int unsigned b;
    randc bit [1:0]  c;
    constraint ca { a inside {[10:19]}; }
    constraint cb { b inside {[100:109]}; }
  endclass
  P p;
  int r;
  integer i;
  initial begin
    p = new();
    for (i = 0; i < 4; i = i + 1) begin
      r = p.randomize();
      $display("R%0d ok=%0d c=%0d inA=%0d inB=%0d",
               i, r, p.c, (p.a>=10)&&(p.a<=19), (p.b>=100)&&(p.b<=109));
    end
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    let ev = sink.events.into_inner();
    assert_eq!(ev.len(), 4, "four draws: {ev:?}");
    let mut seen_c: Vec<u32> = Vec::new();
    for (i, line) in ev.iter().enumerate() {
        let f: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(f[0], format!("out|R{i}"), "line {i}: {line}");
        assert_eq!(f[1], "ok=1", "draw {i} must succeed: {line}");
        assert_eq!(f[3], "inA=1", "draw {i} outside constraint ca: {line}");
        assert_eq!(f[4], "inB=1", "draw {i} outside constraint cb: {line}");
        seen_c.push(f[2].trim_start_matches("c=").parse().unwrap());
    }
    // §18.6: a `randc` field visits every value of its range once per cycle.
    let mut sorted = seen_c.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        vec![0, 1, 2, 3],
        "a 2-bit randc must be a permutation over four draws, got {seen_c:?}"
    );

    // ⚠️ …and the OTHER verdict, which nothing above can hold. §18.11 says a
    // failed randomize returns 0, and every design so far succeeds — so a
    // mutation that hardcodes `success = true` passes all of it. The
    // differential cannot see it either: `class_randomize_run` is shared, so
    // both backends report the same wrong 1 and agree (§5.1-e, again).
    //
    // Made infeasible at RUNTIME rather than statically: a contradictory
    // `constraint { v > 2'd3; }` is refused by elaborate (E3009, "empty
    // solution set") and never reaches a backend. An inline `with` whose domain
    // does not INTERSECT the class range fails inside the solve.
    let src2 = r#"
module t;
  class C; rand int unsigned v; constraint cv { v inside {[0:9]}; } endclass
  C c; int r;
  initial begin
    c = new();
    r = c.randomize() with { v inside {[50:59]}; };
    $display("U ok=%0d", r);
  end
endmodule
"#;
    let (ir2, opts2) = build_with_opts(src2);
    let sink2 = MergedSink::default();
    let r2 = simulate(
        &ir2,
        &sink2,
        SimOpts {
            backend: Backend::Native,
            ..opts2
        },
    );
    assert_eq!(
        r2.backend,
        Backend::Native,
        "refused: {:?}",
        r2.native.refused
    );
    assert_eq!(
        sink2.events.into_inner(),
        vec!["out|U ok=0\n".to_string()],
        "an infeasible randomize must report 0 (IEEE 1800 §18.11)"
    );
}

/// A2-ii DIFFERENTIAL — CRV shapes, native against the VM.
///
/// Weaker than the anchor by construction (the RNG and the solver both live on
/// `SimState`), and kept for the one thing it can see: whether the receiver and
/// the status destination are read and written through the same store. It also
/// carries the shapes the anchor does not — an inline `randomize() with`, a
/// `dist` weighted field, a failed solve, and a null receiver.
#[test]
fn a2ii_crv_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        (
            "randomize with inline constraints",
            r#"
module top;
  class C; rand int unsigned v; constraint cv { v inside {[0:99]}; } endclass
  C c; int r; integer i;
  initial begin
    c = new();
    for (i = 0; i < 3; i = i + 1) begin
      r = c.randomize() with { v inside {[40:49]}; };
      $display("W%0d ok=%0d in=%0d", i, r, (c.v>=40)&&(c.v<=49));
    end
  end
endmodule
"#,
        ),
        (
            "dist weighted field",
            r#"
module top;
  class C; rand bit [1:0] v; constraint cd { v dist { 0 := 1, 3 := 9 }; } endclass
  C c; int r; integer i, hi;
  initial begin
    c = new(); hi = 0;
    for (i = 0; i < 8; i = i + 1) begin r = c.randomize(); if (c.v == 2'd3) hi = hi + 1; end
    $display("D hi_ge_1=%0d ok=%0d", (hi >= 1), r);
  end
endmodule
"#,
        ),
        // ⚠️ The infeasible case has to be made infeasible AT RUNTIME: a
        // statically contradictory `constraint cx { v > 2'd3; }` is refused by
        // elaborate (E3009, "empty solution set") and never reaches a backend.
        // An inline `with` whose domain does not INTERSECT the class range is
        // the shape that fails inside `class_randomize_run` — and `ok=0` is
        // exactly what a store-blind receiver read also produces, which is why
        // it is here as a differential rather than in the anchor.
        (
            "infeasible inline-with fails at runtime",
            r#"
module top;
  class C; rand int unsigned v; constraint cv { v inside {[0:9]}; } endclass
  C c; int r;
  initial begin
    c = new();
    r = c.randomize() with { v inside {[50:59]}; };
    $display("U ok=%0d", r);
  end
endmodule
"#,
        ),
        (
            "randomize on a null handle",
            r#"
module top;
  class C; rand int v; endclass
  C c; int r;
  initial begin r = c.randomize(); $display("N ok=%0d", r); end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 4, "A2-ii differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "A2-ii differential: refusal breakdown moved"
    );
}

/// A5-b ABSOLUTE ANCHOR — the POSTPONED region, iverilog-pinned.
///
/// The last region tier-3 did not have. `$monitor` and `$strobe` were refused
/// TOGETHER because they fail together: `dispatch` only REGISTERS them (it
/// captures ExprIds and touches no net), and the render — plus, for `$monitor`,
/// the change compare that decides whether to render at all — happened in
/// `flush_postponed`, reading the engine's store. On a native run that store
/// never moves, so a `$monitor` printed its establishment line and then went
/// silent for the rest of the simulation: no diagnostic, no crash, just missing
/// output.
///
/// Every line is a distinct part of the region:
///
/// * **the `MON t=0` line** — ESTABLISHMENT, which prints unconditionally.
/// * **`MON t=1`, `t=3`, `t=5`** — change-triggered reprints. These are the
///   ones a store-blind compare loses: the values are identical every time in
///   the engine's untouched store, so `changed` is false forever.
/// * **`STB1` at t=3** — `$strobe` renders at the SETTLED point, so it shows
///   `q=2 s=4` (the NBA of that slot has applied) rather than the `q=1 s=2` a
///   `$display` on the same line would have shown.
/// * **the gap from t=5 to t=9** — `$monitoroff` at t=7 suppresses the
///   change-reprints, and `$monitoron` re-enables them.
/// * **`DISP q=6` between `MON t=11` and `MON t=13`** — the ORDER. `$display`
///   prints in the Active region and the monitor in Postponed, so a region that
///   fired at the wrong point in the loop interleaves differently.
///
/// ⚠️ The design deliberately ends `#2 $finish` in a slot of its own. A
/// `$finish` in the SAME slot as a `$strobe` has a pre-existing iverilog
/// divergence (vvp applies that slot's NBA before the postponed drain, vita does
/// not) — identical on BOTH vita backends, so it is not this slice's, and
/// putting it in an anchor's expected output is what stops the anchor being one
/// (§4.5.302).
#[test]
fn the_postponed_region_has_its_iverilog_values() {
    let src = r#"
module t;
  reg clk = 1'b0;
  reg [3:0] q = 4'd0;
  reg [7:0] s = 8'd0;
  always #1 clk = ~clk;
  always @(posedge clk) begin q <= q + 4'd1; s <= s + 8'd2; end
  initial begin
    $monitor("MON t=%0t q=%0d s=%0d", $time, q, s);
    #3 $strobe("STB1 q=%0d s=%0d", q, s);
    #4 $monitoroff;
    #2 $monitoron;
    #3 $display("DISP q=%0d", q);
    #2 $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|MON t=0 q=0 s=0\n".to_string(),
            "out|MON t=1 q=1 s=2\n".to_string(),
            "out|STB1 q=2 s=4\n".to_string(),
            "out|MON t=3 q=2 s=4\n".to_string(),
            "out|MON t=5 q=3 s=6\n".to_string(),
            "out|MON t=9 q=5 s=10\n".to_string(),
            "out|MON t=11 q=6 s=12\n".to_string(),
            "out|DISP q=6\n".to_string(),
            "out|MON t=13 q=7 s=14\n".to_string(),
        ],
        "postponed region (iverilog 13 pinned)"
    );
}

/// A5-b DIFFERENTIAL — postponed shapes, native against the VM.
///
/// The anchor pins the ordinary case; these are the ones it does not carry.
#[test]
fn a5b_postponed_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // A `$monitor` whose args are a MEMORY element and a wide net — the
        // change compare walks bit planes, so a store-blind read is X-vs-X and
        // never differs.
        (
            "monitor on a memory element",
            r#"
module top;
  reg clk = 1'b0;
  reg [15:0] mem [0:3];
  reg [1:0] i = 2'd0;
  always #1 clk = ~clk;
  always @(posedge clk) begin mem[i] <= {14'd0, i}; i <= i + 2'd1; end
  initial begin
    mem[0]=16'd0; mem[1]=16'd0; mem[2]=16'd0; mem[3]=16'd0;
    $monitor("m0=%0d m1=%0d i=%0d", mem[0], mem[1], i);
    #9 $finish;
  end
endmodule
"#,
        ),
        // A `$strobe` registered MORE THAN ONCE in the same slot: the FIFO must
        // drain in call order.
        (
            "two strobes in one slot",
            r#"
module top;
  reg [7:0] a = 8'd1, b = 8'd2;
  initial begin
    #1;
    $strobe("S1 a=%0d", a);
    $strobe("S2 b=%0d", b);
    a = 8'd9;
    #1 $finish;
  end
endmodule
"#,
        ),
        // A monitor whose ONLY changing argument is `$time` — IEEE §17.1.3 says
        // a direct `$time` does not participate in change detection, so this
        // must print ONCE.
        (
            "monitor on time alone",
            r#"
module top;
  reg [3:0] q = 4'd5;
  initial begin
    $monitor("t=%0t q=%0d", $time, q);
    #1; #1; #1;
    $finish;
  end
endmodule
"#,
        ),
        // ⚠️ A `$strobe` in the SAME SLOT as the `$finish`, which the anchor
        // deliberately cannot carry (that shape has a pre-existing iverilog
        // divergence about whether the slot's NBA lands first). Both vita
        // backends agree about it, so a DIFFERENTIAL can hold the line the
        // anchor cannot — and it is the only thing that reaches the drain at
        // the terminating arm rather than at the stable point. Measured: a
        // mutation deleting that drain survives every other design here.
        (
            "strobe in the finish slot",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] a = 8'd1;
  always #1 clk = ~clk;
  always @(posedge clk) a <= a + 8'd1;
  initial begin
    #3 $strobe("SF a=%0d", a);
    $display("D a=%0d", a);
    $finish;
  end
endmodule
"#,
        ),
        // A `$monitor` re-established mid-run REPLACES the previous one and
        // prints a fresh establishment line.
        (
            "monitor replaced mid-run",
            r#"
module top;
  reg clk = 1'b0;
  reg [3:0] q = 4'd0;
  always #1 clk = ~clk;
  always @(posedge clk) q <= q + 4'd1;
  initial begin
    $monitor("A q=%0d", q);
    #4 $monitor("B q=%0d", q);
    #4 $finish;
  end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 5, "A5-b differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "A5-b differential: refusal breakdown moved"
    );
}

/// A7 ABSOLUTE ANCHOR — functional COVERAGE, hand-IEEE.
///
/// ⭐ **Another conservative row, and the same discovery V1 slice 1 made about
/// SVA**: a covergroup is not a runtime mechanism. Elaborate DESUGARS
/// `cg.sample()` into ordinary bit-set assignments on a 64-bit bitmap net
/// (`1 << (v & 63)`, or the explicit-bin equivalent) and `get_coverage()` into
/// ordinary arithmetic over it, so the tier-3 walk has been executing
/// covergroups correctly since it could execute a body at all.
///
/// ⚠️⚠️ **What it could not do was REPORT them, and that is what this test is
/// really for.** The end-of-run summary read `st.nets[it.bitmap_net].cur` — the
/// engine's flat store — which a native run never writes, so lifting the gate
/// row without `simulate`'s `cover_bits` harvest would have published
/// **0.00% at exit 0**: a silent-wrong in a G2 deliverable (`coverage.json`),
/// not a crash.
///
/// Hand-IEEE because `iverilog 13` rejects `covergroup` outright (the header of
/// `cli/tests/coverage_n5.rs` says so). Every number below is derived:
/// six samples with `x = 0..5` hit the `lo` ({0:3}) and `mid` ({4:7}) bins but
/// not `hi` ⇒ 2/3; `y = i[1:0]` cycles 0,1,2,3,0,1 ⇒ 4/4; the cross has
/// 3×4 = 12 bins and six distinct (x-bin, y) pairs ⇒ 6/12. The instance average
/// is unweighted over the three items: (200/3 + 100 + 50)/3 = 72.222222.
#[test]
fn functional_coverage_is_reported_from_the_store_that_ran() {
    let src = r#"
module t;
  reg [3:0] x;
  reg [1:0] y;
  covergroup cg;
    cp_x: coverpoint x { bins lo = {[0:3]}; bins mid = {[4:7]}; bins hi = {[8:15]}; }
    cp_y: coverpoint y;
    cr:   cross cp_x, cp_y;
  endgroup
  cg c = new;
  integer i;
  initial begin
    for (i = 0; i < 6; i = i + 1) begin x = i[3:0]; y = i[1:0]; c.sample(); end
    $display("done x=%0d y=%0d", x, y);
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    let cov = r
        .coverage
        .expect("a covergroup design must report coverage");
    assert_eq!(cov.groups.len(), 1, "one instance");
    let g = &cov.groups[0];
    assert_eq!(g.instance, "t.c");
    let items: Vec<(&str, bool, u32, u32)> = g
        .items
        .iter()
        .map(|i| (i.name.as_str(), i.is_cross, i.num_bins, i.covered_bins))
        .collect();
    assert_eq!(
        items,
        vec![
            ("cp_x", false, 3, 2),
            ("cp_y", false, 4, 4),
            ("cross_0", true, 12, 6),
        ],
        "per-item bins (hand-IEEE; iverilog rejects covergroup)"
    );
    // The weighted average, to the same six decimals `coverage.json` publishes.
    assert!(
        (g.coverage_pct - 72.222222).abs() < 1e-5,
        "instance average: {}",
        g.coverage_pct
    );
    // ⚠️ The line that makes the whole test non-vacuous: 0.0 is exactly what an
    // unharvested native run publishes, and it is a legal value.
    assert!(g.coverage_pct > 0.0, "a native run must not report 0%");
}

/// A7 DIFFERENTIAL — the coverage SUMMARY, native against the VM.
///
/// The anchor above pins the numbers; this pins that the two backends agree
/// about them, over shapes the anchor does not carry: a covergroup that is
/// never sampled (every bin 0, and the average must still be 0.0 rather than
/// NaN), one sampled under a clocked process rather than in an `initial`, and
/// two INSTANCES of the same covergroup — which is the shape the flat
/// `cover_bits` index would get wrong if it were reset per instance.
#[test]
fn a7_coverage_summary_matches_the_vm() {
    for (name, src) in [
        (
            "never sampled",
            r#"
module t;
  reg [1:0] x;
  covergroup cg; cp: coverpoint x; endgroup
  cg c = new;
  initial begin x = 2'd1; $display("n"); end
endmodule
"#,
        ),
        (
            "sampled on a clock edge",
            r#"
module t;
  reg clk = 1'b0;
  reg [1:0] x = 2'd0;
  covergroup cg; cp: coverpoint x; endgroup
  cg c = new;
  always #1 clk = ~clk;
  always @(posedge clk) begin x <= x + 2'd1; c.sample(); end
  initial begin #9 $display("x=%0d", x); $finish; end
endmodule
"#,
        ),
        // TWO instances, so the flat `cover_bits` index has to keep walking
        // across the instance boundary. A per-instance index would report the
        // first group's bins under the second's name — and, because both are
        // 4-bin coverpoints here, would still produce plausible numbers.
        // (Two covergroups rather than one parameterized one: `covergroup
        // cg(input …)` is a pre-existing elaborate gap, E3010.)
        (
            "two instances",
            r#"
module t;
  reg [1:0] x;
  reg [2:0] y;
  covergroup cga; cpx: coverpoint x; endgroup
  covergroup cgb; cpy: coverpoint y; endgroup
  cga ca = new;
  cgb cb = new;
  initial begin
    x = 2'd0; y = 3'd7; ca.sample(); cb.sample();
    x = 2'd1;           ca.sample();
    x = 2'd2;           ca.sample();
    $display("done");
  end
endmodule
"#,
        ),
    ] {
        let (ir, opts) = build_with_opts(src);
        let vm = simulate(
            &ir,
            &MergedSink::default(),
            SimOpts {
                backend: Backend::Bytecode,
                ..opts.clone()
            },
        );
        let nat = simulate(
            &ir,
            &MergedSink::default(),
            SimOpts {
                backend: Backend::Native,
                ..opts
            },
        );
        assert_eq!(
            nat.backend,
            Backend::Native,
            "{name}: refused {:?}",
            nat.native.refused
        );
        let fmt = |r: &sim_engine::SimResult| {
            r.coverage.as_ref().map(|c| {
                c.groups
                    .iter()
                    .map(|g| {
                        (
                            g.instance.clone(),
                            format!("{:.6}", g.coverage_pct),
                            g.items
                                .iter()
                                .map(|i| (i.name.clone(), i.num_bins, i.covered_bins))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(fmt(&vm), fmt(&nat), "{name}: coverage summary diverged");
        assert!(fmt(&nat).is_some(), "{name}: no summary was produced");
    }
}

/// A8-a ABSOLUTE ANCHOR — a whole-handle COPY (IEEE §7.10), iverilog-pinned.
///
/// ⭐ **Zero kernel code, like V1 slice 1 and A1-i.** The `handle_copy` design
/// row refused 81 designs for a feature whose entire implementation is a
/// deep-clone inside `SimState::dyn_heap` — one object both kernels borrow,
/// keyed by NET ID — with the two net ids arriving from a sidecar rather than
/// from an evaluation. Nothing reads a net value, so there was nothing to route.
///
/// The lines are chosen so a SHALLOW copy (an alias rather than a clone) prints
/// something else at every one:
///
/// * **A** — dynamic array. `d1[1] = 99` after the copy must not show through.
/// * **B** — queue, and the SIZE as well as the elements: `q1.push_back(4)`
///   after the copy leaves `q2` at 3.
/// * **D** — a `string[]`, whose elements are themselves heap objects, so a
///   clone that copied the handle list without cloning the elements passes A
///   and fails here.
/// * **E** — copying OVER an existing object (`d2 = new[0]` first), which is
///   the arm that overwrites rather than fills an empty slot.
///
/// ⚠️ The ASSOC case is deliberately absent: `iverilog 13` cannot parse `int
/// a1[int]` at all ("Type names are not valid expressions here"), so it is
/// covered by the differential below instead of by this anchor. Splitting on
/// what the oracle can answer, rather than weakening the anchor to fit, is the
/// §4.5.302 rule.
#[test]
fn a_whole_handle_copy_has_its_iverilog_values() {
    let src = r#"
module top;
  int  d1[], d2[];
  int  q1[$], q2[$];
  string s1[], s2[];
  initial begin
    d1 = new[3]; d1[0]=7; d1[1]=8; d1[2]=9;
    d2 = d1;
    d1[1] = 99;
    $display("A d2=%0d,%0d,%0d size=%0d", d2[0], d2[1], d2[2], d2.size());
    q1.push_back(1); q1.push_back(2); q1.push_back(3);
    q2 = q1;
    q1.push_back(4);
    $display("B q2size=%0d q2=%0d,%0d,%0d q1size=%0d", q2.size(), q2[0], q2[1], q2[2], q1.size());
    s1 = new[2]; s1[0] = "ab"; s1[1] = "cd";
    s2 = s1; s1[0] = "zz";
    $display("D s2=%s,%s s1_0=%s", s2[0], s2[1], s1[0]);
    d2 = new[0]; d2 = d1;
    $display("E d2_1=%0d size=%0d", d2[1], d2.size());
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A d2=7,8,9 size=3\n".to_string(),
            "out|B q2size=3 q2=1,2,3 q1size=4\n".to_string(),
            "out|D s2=ab,cd s1_0=zz\n".to_string(),
            "out|E d2_1=99 size=3\n".to_string(),
        ],
        "whole-handle copy (iverilog 13 pinned)"
    );
}

/// A8-a: the ASSOC half, which the anchor above cannot carry because
/// `iverilog 13` will not parse an associative array.
///
/// Hand-IEEE §7.10: an assoc-to-assoc assignment copies the whole map by value,
/// so a later write to either side is invisible to the other. Bounded queues are
/// here too — `enforce_queue_bound` runs after the clone, and a copy INTO a
/// `[$:1]` destination truncates with a warning.
#[test]
fn a_handle_copy_of_an_assoc_and_a_bounded_queue() {
    let src = r#"
module top;
  int a1[int], a2[int];
  int q1[$], bq[$:1];
  initial begin
    a1[5] = 50; a1[9] = 90;
    a2 = a1;
    a1[5] = 500;
    $display("C a2_5=%0d a2_9=%0d a1_5=%0d n=%0d", a2[5], a2[9], a1[5], a2.num());
    q1.push_back(1); q1.push_back(2); q1.push_back(3);
    bq = q1;
    $display("F bqsize=%0d bq=%0d,%0d", bq.size(), bq[0], bq[1]);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    let ev = sink.events.into_inner();
    assert_eq!(
        ev.iter()
            .filter(|e| e.starts_with("out|"))
            .collect::<Vec<_>>(),
        vec![
            "out|C a2_5=50 a2_9=90 a1_5=500 n=2\n",
            "out|F bqsize=2 bq=1,2\n",
        ],
        "assoc + bounded-queue copy (hand-IEEE §7.10/§7.10.2)"
    );
    // …and the truncation is LOUD, which is the half a value comparison misses.
    assert!(
        ev.iter()
            .any(|e| e.contains("bounded queue exceeded its bound")),
        "the bound truncation must warn: {ev:?}"
    );
}

/// A2-i: VIRTUAL dispatch runs on tier-3, and this test is why the gate has no
/// row for it.
///
/// ⚠️⚠️ **Hand-IEEE on purpose — iverilog 13 gets this WRONG.** For a base
/// handle pointing at a derived object it calls the BASE method, where IEEE 1800
/// §8.20 requires the dynamic type's override. All three vita backends agree
/// with the LRM, so `vvp` is not the oracle here; the values below are §8.20's.
/// (One of the few places this repo is ahead of its own oracle rather than
/// behind it — recorded so a future reader does not "fix" vita to match.)
///
/// The design is built to fail under a STATIC dispatch: three levels with an
/// override at each, plus an INHERITED method, plus the same handle re-pointed
/// at two different objects. `a` is the base object, `b` the derived, `c` the
/// grand-derived; `d` takes `E`'s override of `twice` and `e` the `B` version
/// `D` inherits — so a run that resolved everything statically prints
/// `11 12 13 10 10`.
#[test]
fn a_virtual_call_dispatches_dynamically_on_tier_3() {
    let src = r#"
module top;
  class B;
    int v;
    virtual function int who(); return 10 + v; endfunction
    virtual function int twice(input int d); return d * 2; endfunction
  endclass
  class D extends B;
    virtual function int who(); return 20 + v; endfunction
  endclass
  class E extends D;
    virtual function int who(); return 30 + v; endfunction
    virtual function int twice(input int d); return d * 3; endfunction
  endclass
  B h; D dd; E ee; int a, b2, c, d2, e2;
  initial begin
    h = new(); h.v = 1; a = h.who();
    dd = new(); dd.v = 2; h = dd; b2 = h.who();
    ee = new(); ee.v = 3; h = ee; c = h.who();
    d2 = h.twice(5);
    h = dd; e2 = h.twice(5);
    $display("a=%0d b=%0d c=%0d d=%0d e=%0d", a, b2, c, d2, e2);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec!["out|a=11 b=22 c=33 d=15 e=10\n".to_string()],
        "virtual dispatch (IEEE 1800 §8.20; iverilog 13 disagrees)"
    );
}

/// The one shape iverilog cannot be asked about: dereferencing a NULL handle.
///
/// ⚠️ `ivl` SEGFAULTS at compile time on `$display("%0d", nul.x)` (measured,
/// iverilog 13.0), so this is hand-IEEE: §8.4 says an uninitialised class
/// variable is `null`, and vita's policy for a null dereference is warn-once +
/// X rather than a fatal. Pinned separately so the anchor above stays free of a
/// known divergence.
///
/// It is also the one line that proves the handle READ routes and not just the
/// field read: a never-assigned handle is `0` in BOTH stores, so it can only
/// distinguish the warn path — which is exactly why it is not in the anchor.
#[test]
fn a_null_handle_dereference_is_x_and_warns() {
    let src = r#"
module top;
  class Pt; int x; endclass
  Pt nul;
  initial begin
    $display("N=%0d", nul.x);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    // The whole stream, in order: the warning is emitted at the READ, i.e.
    // before the line that read it renders. `%0d` of an all-X value is `X`.
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "diag|Warning|VITA-W4020|null/X class handle dereference (read X)".to_string(),
            "out|N=X\n".to_string(),
        ],
        "null-handle dereference"
    );
}

/// The READ twin of the funnel pin: every store read a system task makes must go
/// through the THREADED reader, or be behind a row that refuses the design.
///
/// ⚠️ This is the pin V1 slice 2 needed and did not have. `builtins::dispatch`
/// takes the alternate store as a parameter, but only the arms that render
/// through the formatter use it — the others call `Scheduler::eval` /
/// `eval_ctx_top` / `assoc_key_of`, which build an `EvalCtx` over `SimState`'s
/// own nets. That is the store a native run NEVER writes. While every heap kind
/// was refused those arms were unreachable; slices 2a/2b/2c made four of them
/// reachable and each one silently read X:
///
/// * `d = new[n]`            → `size=0`   (VM: 3)
/// * `s.itoa(v)`             → `s=0`      (VM: 200)
/// * `q.push_back(a)`        → `q[0]=x`   (VM: 42)
/// * `r = q[a:b]`            → the empty queue
///
/// All four now go through `eval_task_arg` / `eval_task_arg_ctx`, whose `None`
/// arm is literally the call they made before — so the engine is unchanged by
/// construction (§4.5.314's opt-in rule).
///
/// This test pins what is LEFT. Each remaining site is listed with the row that
/// makes it unreachable; opening any of those rows must fail here first, which
/// is the whole point — the next slice cannot repeat slice 2's mistake by
/// accident. The count is per file rather than per line so that moving code
/// inside a file does not churn it.
#[test]
fn every_untreaded_store_read_in_builtins_sits_behind_a_reject_row() {
    // (file, expected raw-read sites, why each is unreachable)
    let files: [(&str, usize, &str); 4] = [
        (
            "dispatch.rs",
            1,
            "`split_file_directed`'s fd — the `file_directed` row. ⚠️ The other \
             half of this reason (`and $monitor/$strobe are in systask_refusal \
             too`) EXPIRED with A5-b, which wired both; the site is now held by \
             the `file_directed` row alone. §4.5.338 again, inside a test",
        ),
        (
            "crv_draw.rs",
            3,
            "⚠️ THREE now, not four — A2-ii threaded `class_randomize_run`'s \
             receiver, which was the CRV surface's whole store dependence. The \
             three that remain are `$writemem*`'s: its two window bounds and its \
             per-element memory read, all behind the `systask_refusal` row \
             (`$writemem*` reads the MEMORY itself, not a formatted argument). \
             A1-iii took this from 6 to 4 by threading `readmem`'s two window \
             bounds — measured, not assumed: `$readmemh(f, m, lo, hi)` with NET \
             bounds loaded the whole array on a native run. ⚠️ This entry has now \
             been re-worded TWICE for the same reason (§4.5.338): each time a \
             row it names is split or lifted, the sentence stops being true \
             while the NUMBER still passes",
        ),
        ("render.rs", 2, "`$vita_stage` is the `stage` row"),
        (
            "queues_io.rs",
            2,
            "the `None` arms of `eval_task_arg` and `eval_task_arg_ctx` — the \
             seams themselves, which is where a raw read is SUPPOSED to be",
        ),
    ];
    // ⚠️ `sched.assoc_` on purpose, not `sched.assoc_key_of(`. The narrow
    // spelling missed `assoc_str_key_of`, the string-keyed twin sitting twenty
    // lines above it — the pin was one arm short of the surface it claims to
    // cover, and the design that would have found it is exactly the one slice 2d
    // is about to admit. A pattern that names one member of a family is a
    // whitelist pretending to be a scan.
    let raw = [
        "sched.eval(",
        "sched.eval_ctx_top(",
        "sched.assoc_",
        "sched.st.read_net(",
    ];
    let srcs = [
        ("dispatch.rs", include_str!("../builtins/dispatch.rs")),
        ("crv_draw.rs", include_str!("../builtins/crv_draw.rs")),
        ("render.rs", include_str!("../builtins/render.rs")),
        ("queues_io.rs", include_str!("../builtins/queues_io.rs")),
    ];
    for (name, want, why) in files {
        let src = srcs.iter().find(|(n, _)| *n == name).unwrap().1;
        let got = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .filter(|l| raw.iter().any(|r| l.contains(r)))
            .count();
        assert_eq!(
            got, want,
            "{name}: raw store reads moved. Every one of them must stay behind \
             a reject row — today: {why}. If a row was OPENED, thread the read \
             through `eval_task_arg`/`eval_task_arg_ctx` instead of updating \
             this number."
        );
    }
}

/// The ABSOLUTE anchor for V1 slice 1 — what the assertion-control design MEANS,
/// not merely that two backends agree about it.
///
/// ⚠️ Written because the mutation battery proved the corpus row alone is not
/// enough. Dropping `assert_ctl` (or `assert_fire`) from `build_with_opts` left
/// `sva_assert_control_off_then_on` GREEN: without the tables, `$assertoff`
/// degrades into a plain `Display` and the suppression set is empty, so BOTH
/// violations fire — identically on both backends. A native-vs-VM differential
/// is structurally blind to that, because it is not a backend difference.
///
/// The same battery showed where the real protection for the SHARED dispatch
/// lives: mutating the `assert_fire` early-return or the `assert_ctl` flip left
/// every engine gate green and was killed only by `cli::sva_rest`'s absolute
/// pins. Value-agreement between two backends cannot see a rule both of them
/// read from one place — this test is the engine-side answer to that.
///
/// The design violates `a |-> b` TWICE: once at t=10 while assertions are off,
/// once at t=25 after `$asserton`. Exactly ONE report is the whole claim.
#[test]
fn sva_assert_control_actually_suppresses_exactly_one_violation() {
    let (ir, opts) = build_with_opts(
        r#"
module top;
  reg clk = 0, a = 0, b = 0;
  always #5 clk = ~clk;
  initial assert property(@(posedge clk) a |-> b);
  initial begin
    #7  $assertoff;   // t=7   assertions off
    #3  a = 1; b = 0; // t=10  -> posedge 15 VIOLATES, suppressed
    #6  a = 0;        // t=16  clear the window BEFORE re-enabling, or the
                      //       still-high `a` violates again at posedge 25
    #4  $asserton;    // t=20  -> posedge 25: a=0, implication holds
    #10 a = 1; b = 0; // t=30  -> posedge 35 VIOLATES, fires
    #10 $finish;      // t=40
  end
endmodule
"#,
    );
    crate::native::run::runnable(&ir, &opts).expect("must run natively, or this proves nothing");
    let sink = MergedSink::default();
    let r = simulate(
        &ir,
        &sink,
        SimOpts {
            backend: Backend::Native,
            ..opts
        },
    );
    assert_eq!(
        r.backend,
        Backend::Native,
        "fell back: {:?}",
        r.native.refused
    );
    let rows = sink.events.into_inner();
    let fires = rows.iter().filter(|r| r.contains("Assertion")).count();
    assert_eq!(
        fires, 1,
        "exactly one violation must survive: the t=10 one is suppressed by \
         `$assertoff`, the t=25 one fires after `$asserton`.\n{rows:#?}"
    );
    // …and the control statements themselves must be SWALLOWED, not printed: a
    // `$assertoff` whose sid is missing from `assert_ctl` renders as an ordinary
    // (empty) `Display` line, which is the other half of the same omission.
    assert_eq!(
        rows.iter().filter(|r| r.starts_with("out|")).count(),
        0,
        "assertion-control statements must not reach stdout\n{rows:#?}"
    );
    assert_eq!(r.exit_class, crate::ExitClass::HadErrors);
}

/// V1 slice 1's OTHER half: opening the `sva` row must not have opened the SVA
/// shapes that genuinely need machinery tier-3 does not have.
///
/// Removing a gate row is the one change whose failure mode is a design that now
/// RUNS, so the slice owes a pin on each neighbour that must still refuse — and
/// on the REASON, because "refused" by the wrong row is how a maintainer later
/// deletes the row that was actually load-bearing.
///
/// Both of these are real machinery gaps, not conservatism:
///   * `cover property` (and SVA liveness) synthesize an end-of-sim obligation
///     check registered in `final_procs`; tier-3's run loop has no post-loop
///     drain, so `executor_rows` refuses them as `final` blocks.
///   * ⚠️ a §16.4 DEFERRED assertion USED to be the second case here, and A8-b
///     removed it: tier-3's cascade now calls `mature_deferred` at the Observed
///     and Reactive positions, so that row is gone. What is left is the one
///     above — which is the point of this test, since the claim is that each
///     shape needing machinery refuses BY ITS OWN NAME rather than by SVA's.
#[test]
fn sva_shapes_that_need_machinery_still_refuse_by_their_own_name() {
    let cases: Vec<(&str, &str, &str)> = vec![(
        "cover property",
        "`final` blocks (the post-loop drain is not restated)",
        r#"
module top;
  reg clk = 0, a = 0, b = 0;
  always #5 clk = ~clk;
  initial cover property(@(posedge clk) a ##1 b);
  initial begin #10 a = 1; #10 b = 1; #20 $finish; end
endmodule
"#,
    )];
    for (what, row, src) in &cases {
        let (ir, opts) = build_with_opts(src);
        assert_eq!(
            crate::native::run::runnable(&ir, &opts),
            Err(*row),
            "{what}: wrong refusal row"
        );
    }
    // ONE now, not two — A8-b wired deferred assertions, so that case moved out
    // of this test entirely. The count is asserted exactly rather than as `> 0`
    // so that admitting or refusing another SVA shape moves a number a human has
    // to re-justify, which is what just happened.
    assert_eq!(cases.len(), 1);
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

/// `$dumpfile` with a NON-CONSTANT argument — the shape that made the
/// un-refusal wrong.
///
/// `arg_string` does not return early for a non-`Const` argument (a comment
/// once claimed it did): it renders the VALUE, and on a native run that read
/// the engine's untouched store, so the waveform landed in a file named `x`
/// instead of `42`. stdout, VCD content and exit code were all identical —
/// only the FILENAME differed, which is why `agree`'s byte compare cannot see
/// it. Compared through `SimResult::vcd_path` rather than by listing a
/// directory: the path is the observable, and reading it needs no `chdir`
/// (which is process-global and would race the parallel tests).
#[test]
fn s1d4d2_dumpfile_with_a_non_const_argument_picks_the_same_path() {
    let src = r#"
module top;
  integer nm;
  reg [7:0] a;
  initial begin
    nm = 42;
    $dumpfile(nm);
    $dumpvars(0, top);
    a = 8'd0; #1 a = 8'h11; #1 $finish;
  end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    let dir = std::env::temp_dir();
    let mut paths = Vec::new();
    for (backend, tag) in [(Backend::Bytecode, "vm"), (Backend::Native, "nat")] {
        // NOT `vcd_path_override` — that would answer the question for the task
        // and test nothing. The override is used only to keep the file out of
        // the repo, by making the design's own argument resolve under a temp
        // dir… which it cannot, so instead the run is allowed to write wherever
        // it decides and the decision itself is what is compared.
        let sink = MergedSink::default();
        let r = simulate(
            &ir,
            &sink,
            SimOpts {
                backend,
                ..opts.clone()
            },
        );
        if backend == Backend::Native {
            assert_eq!(r.backend, Backend::Native, "fell back");
        }
        paths.push((tag, r.vcd_path.clone()));
        if let Some(p) = r.vcd_path {
            let _ = std::fs::remove_file(&p);
            let _ = std::fs::remove_file(dir.join(&p));
        }
    }
    assert_eq!(
        paths[0].1, paths[1].1,
        "the two backends resolved `$dumpfile(nm)` to different paths: {paths:?}"
    );
    // ANTI-VACUITY: both must have opened SOMETHING, and it must be the value
    // of `nm` — if the argument silently rendered empty on both sides the
    // comparison above would pass on two wrongs.
    assert_eq!(
        paths[0].1.as_deref(),
        Some("42"),
        "the non-const argument must render as its VALUE: {paths:?}"
    );
}

/// The INERTIAL pulse filter, pinned to VALUES rather than to the other backend.
///
/// ⚠️ This is an ABSOLUTE assertion on purpose. The generation check that
/// implements the filter lives in `Scheduler::take_due_delayed_ca`, which BOTH
/// backends now call — so deleting it moves both sides of a VM-vs-native
/// differential equally and that comparison cannot see it (measured: the
/// mutation survives the whole differential gate). Sharing removes drift and
/// removes the differential's sensitivity with it; the anchor has to come from
/// outside.
///
/// The numbers below are iverilog 13's, measured: a pulse NARROWER than `d`
/// never reaches the LHS (`narrow` stays x through the window), and one of
/// EXACTLY `d` survives (`exact` shows 9 at t=4) — because a pending write
/// applies at the tick start, before the RHS changes again.
#[test]
fn s1d4d3_inertial_pulse_filter_matches_the_oracle() {
    let src = r#"
module top;
  wire [7:0] narrow, exact;
  reg [7:0] a;
  assign #4 narrow = a;
  assign #2 exact = a;
  initial begin
    a = 8'd0;
    #1 a = 8'd9;
    #2 a = 8'd0;
    #1 $display("t=%0t narrow=%0d exact=%0d", $time, narrow, exact);
    #2 $display("t=%0t narrow=%0d exact=%0d", $time, narrow, exact);
    #2 $display("t=%0t narrow=%0d exact=%0d", $time, narrow, exact);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &[
        "out|t=4 narrow=x exact=9
",
        "out|t=6 narrow=x exact=0
",
        "out|t=8 narrow=0 exact=0
",
    ];
    both_backends_print(src, WANT, "inertial filter");
}

/// The per-transition RISE/FALL delay selection, likewise pinned to VALUES.
///
/// `transition_delay` lives in the shared half too, so the same argument as
/// above applies: a differential between the two backends cannot see a change
/// there. Measured against iverilog 13 — rise 2 (`o` is 1 at t=13, not t=11),
/// fall 7 (`o` is still 1 at t=20 and 0 at t=23).
#[test]
fn s1d4d3_rise_fall_delays_match_the_oracle() {
    let src = r#"
module top;
  wire o;
  reg a;
  assign #(2,7) o = a;
  initial begin
    a = 1'b0;
    #10 a = 1'b1;
    #1 $display("t=%0t o=%b", $time, o);
    #2 $display("t=%0t o=%b", $time, o);
    #1 a = 1'b0;
    #6 $display("t=%0t o=%b", $time, o);
    #3 $display("t=%0t o=%b", $time, o);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &[
        "out|t=11 o=0\n",
        "out|t=13 o=1\n",
        "out|t=20 o=1\n",
        "out|t=23 o=0\n",
    ];
    both_backends_print(src, WANT, "rise/fall selection");
}

/// The rise/fall BASELINE is the last value this assign actually DROVE, not the
/// last RHS it saw — and the two only differ on an inertial SUPERSEDE.
///
/// This is the third anchor in the shared half, and the adversarial soundness
/// lens found it by mutation: swapping `last_ca_drv` for `last_ca` at
/// `scan_arm.rs` survived the corpus, the 50 adversarial designs, and both
/// anchors above, because every one of them changes the RHS at most once per
/// pending write. The distinguishing shape needs a second RHS change BEFORE the
/// first delayed write lands, with rise and fall far enough apart to name which
/// baseline was used.
///
/// `a` goes 00 → 11 (t=20, schedules t=22 on rise 2) → 10 (t=21, superseding).
/// The superseded write never landed, so the net still outputs 00 and the new
/// transition 00 → 10 is a RISE: t=23. Reading `last_ca` instead makes the
/// baseline the cancelled 11, turning bit 0 into a FALL: t=30. So a probe at
/// t=25 separates them, and iverilog 13 says the value there is **10**.
///
/// The probes are timed `$display`s and NOT an `always @(y)` monitor, on
/// purpose: an edge monitor here also reports vita's spurious t=0 event on the
/// initial-X window (ROADMAP §2, pre-existing, both backends), and an anchor
/// whose expected output encodes a known divergence from the oracle stops being
/// an anchor. Sample the values the mutation moves; do not bless the events.
#[test]
fn s1d4d3_supersede_measures_from_what_was_driven() {
    let src = r#"
module top;
  reg [1:0] a;
  wire [1:0] y;
  assign #(2,9) y = a;
  initial begin
    a = 2'b00;
    #20 a = 2'b11;
    #1  a = 2'b10;
    #1  $display("t=%0t y=%b", $time, y);
    #3  $display("t=%0t y=%b", $time, y);
    #10 $display("t=%0t y=%b", $time, y);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &["out|t=22 y=00\n", "out|t=25 y=10\n", "out|t=35 y=10\n"];
    both_backends_print(src, WANT, "supersede baseline");
}

/// The tier-3 arm builds its own evaluation CONTEXT, and the context is two
/// numbers: the width and the sign. Both were unpinned.
///
/// The engine's arm goes through `eval_cont_assign`; the native arm re-spells
/// the same rule (`max(lhs, self(rhs))`, rhs's own sign) at the seam. A corpus
/// delayed assign is `assign #2 dly = a;` — same width both sides, unsigned —
/// so dropping `lw.max(..)` and forcing `signed = false` BOTH survived every
/// gate. Truncation needs an rhs whose self-determined width is narrower than
/// the lvalue (4-bit + 4-bit carrying into 8), and sign needs a negative value
/// crossing a widening boundary. Values are iverilog 13's.
#[test]
fn s1d4d3_delayed_rhs_context_is_width_and_sign() {
    let src = r#"
module top;
  reg [3:0] a, b;
  reg signed [3:0] c;
  wire [7:0] y;
  wire signed [7:0] z;
  assign #2 y = a + b;
  assign #2 z = c;
  initial begin
    a = 4'hF; b = 4'hF; c = -3;
    #5 $display("t=%0t y=%h z=%b (%0d)", $time, y, z, z);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &["out|t=5 y=1e z=11111101 (-3)\n"];
    both_backends_print(src, WANT, "delayed rhs context");
}

/// The wire-resolution TABLES, pinned to iverilog 13's values (S1d-4d-4).
///
/// `resolve_md_group` and the three `resolve_*_into` folds are SHARED between
/// the two settle loops — the extraction that keeps the backends from
/// disagreeing also blinds the VM-vs-native differential to the whole fold
/// (§4.5.302's rule: name the anchor that guards what you share). One design
/// per kind, each chosen so every distinguishing row of its table appears:
/// WIRE needs z-yields + equal-keeps + conflict→x, WAND and WOR need the row
/// where they differ from plain AND/OR (x vs the dominating value) and the row
/// where 0/1 dominates x.
#[test]
fn s1d4d4_wire_resolution_matches_the_oracle() {
    let src = r#"
module top;
  wire [3:0] y;
  reg [3:0] a, b;
  assign y = a;
  assign y = b;
  initial begin
    a = 4'bz01z; b = 4'b10zz;
    #1 $display("t=%0t y=%b", $time, y);
    a = 4'b0xz1; b = 4'b0z1x;
    #1 $display("t=%0t y=%b", $time, y);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &["out|t=1 y=101z\n", "out|t=2 y=0x1x\n"];
    both_backends_print(src, WANT, "wire resolution");
}

#[test]
fn s1d4d4_wand_resolution_matches_the_oracle() {
    let src = r#"
module top;
  wand [3:0] y;
  reg [3:0] a, b;
  assign y = a;
  assign y = b;
  initial begin
    a = 4'b1100; b = 4'b1010;
    #1 $display("t=%0t y=%b", $time, y);
    a = 4'bz1x0; b = 4'b11xz;
    #1 $display("t=%0t y=%b", $time, y);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &["out|t=1 y=1000\n", "out|t=2 y=11x0\n"];
    both_backends_print(src, WANT, "wand resolution");
}

#[test]
fn s1d4d4_wor_resolution_matches_the_oracle() {
    let src = r#"
module top;
  wor [3:0] y;
  reg [3:0] a, b;
  assign y = a;
  assign y = b;
  initial begin
    a = 4'b1100; b = 4'b1010;
    #1 $display("t=%0t y=%b", $time, y);
    a = 4'bz1x0; b = 4'b00xz;
    #1 $display("t=%0t y=%b", $time, y);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &["out|t=1 y=1110\n", "out|t=2 y=01x0\n"];
    both_backends_print(src, WANT, "wor resolution");
}

/// THREE drivers through the shared fold: associativity over the group, and
/// the accumulator identity being all-Z rather than all-0 (an all-0 identity
/// makes bit 1 come out 0 instead of z here). iverilog 13: `10`.
#[test]
fn s1d4d4_three_driver_fold_matches_the_oracle() {
    let src = r#"
module top;
  wire [1:0] y;
  assign y = 2'b1z;
  assign y = 2'bz0;
  assign y = 2'b1z;
  initial begin #1 $display("t=%0t y=%b", $time, y); $finish; end
endmodule
"#;
    const WANT: &[&str] = &["out|t=1 y=10\n"];
    both_backends_print(src, WANT, "three-driver fold");
}

/// The md loop's POSITION in the settle pass — after the per-driver writes,
/// inside the same fixpoint — pinned through the DELTA BUDGET.
///
/// Moving the group loop before the per-driver loop survived the whole suite
/// (soundness-review mutation): every value eventually converges either way,
/// so no stream moves. What does move is HOW MANY passes convergence takes
/// when a plain cont-assign FEEDS a group driver — the mutant needs an extra
/// pass to see the plain assign's write, and at a budget of exactly 2 the
/// backends part (one delta-limits, the other finishes). Sweeping the budget
/// crosses both regimes, with anti-vacuity asserting each was seen.
#[test]
fn s1d4d4_group_loop_position_pinned_by_delta_budget() {
    let src = r#"
module top;
  wire c;
  wire y;
  reg r;
  reg [7:0] n;
  assign c = r;
  assign y = c;
  assign y = 1'bz;
  always @(y) begin n = n + 8'd1; $display("t=%0t y=%b n=%0d", $time, y, n); end
  always #1 r = ~r;
  initial begin r = 1'b0; n = 8'd0; #6 $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    let mut regimes = std::collections::BTreeSet::new();
    for budget in 1..=24u64 {
        let mut streams = Vec::new();
        for backend in [Backend::Bytecode, Backend::Native] {
            let sink = MergedSink::default();
            let r = simulate(
                &ir,
                &sink,
                SimOpts {
                    backend,
                    max_deltas: budget,
                    ..opts.clone()
                },
            );
            if backend == Backend::Native {
                assert_eq!(r.backend, Backend::Native, "budget={budget}: fell back");
            }
            streams.push((sink.events.into_inner(), r.finish_reason, r.exit_class));
        }
        assert_eq!(
            streams[0], streams[1],
            "budget={budget}: stream/finish/exit differ between backends"
        );
        regimes.insert(format!("{:?}", streams[0].1));
    }
    assert!(
        regimes.len() >= 2,
        "the sweep never crossed a regime boundary — every budget gave {regimes:?},          so the position pin is vacuous"
    );
}

/// `$value$plusargs` ON THE NATIVE PATH, pinned to iverilog 13's values.
///
/// The soundness lens measured that no test anywhere ran `$value$plusargs`
/// under the native backend: dropping the native write entirely (status still
/// returned) and hardcoding the status to 1 both survived every suite — the
/// shared conversion was anchored through the Scheduler consumer only, which
/// is exactly the §4.5.302 violation. One design, five axes, every expected
/// value measured against iverilog: hit, miss (var untouched AND status 0),
/// negative `%h` into 32 bits (`-` applies to every radix and negates within
/// the DESTINATION width — the `dest_w.max(..)` mutation's only observable),
/// negative `%b`, and a 24-digit `%h` into 96 bits (the wide fix, on this
/// consumer).
#[test]
fn s1d5_value_plusargs_native_matches_the_oracle() {
    let src = r#"
module top;
  reg [31:0] n, ok, h, b;
  reg [95:0] w;
  initial begin
    n = 32'd0; h = 32'hAA; b = 32'hBB; w = 96'hCC;
    ok = $value$plusargs("N=%d", n);    $display("t=%0t hit ok=%0d n=%0d", $time, ok, n);
    ok = $value$plusargs("MISS=%d", n); $display("t=%0t miss ok=%0d n=%0d", $time, ok, n);
    ok = $value$plusargs("H=%h", h);    $display("t=%0t negh ok=%0d h=%h", $time, ok, h);
    ok = $value$plusargs("B=%b", b);    $display("t=%0t negb ok=%0d b=%h", $time, ok, b);
    ok = $value$plusargs("W=%h", w);    $display("t=%0t wide ok=%0d w=%h", $time, ok, w);
    ok = $value$plusargs("XZ=%h", h);   $display("t=%0t xz ok=%0d h=%h", $time, ok, h);
    ok = $value$plusargs("BAD=%d", n);  $display("t=%0t bad ok=%0d n=%h", $time, ok, n);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &[
        "out|t=0 hit ok=1 n=42
",
        "out|t=0 miss ok=0 n=42
",
        "out|t=0 negh ok=1 h=fffffffb
",
        "out|t=0 negb ok=1 b=ffffffff
",
        "out|t=0 wide ok=1 w=123456789abcdef012345678
",
        "out|t=0 xz ok=1 h=00001x2z
",
        "diag|Warning|VITA-W4028|invalid decimal value \"5x9\" in a matched plusarg; variable written all-X",
        "out|t=0 bad ok=1 n=xxxxxxxx
",
    ];
    both_backends_print_with_plusargs(
        src,
        &[
            "N=42",
            "H=-5",
            "B=-1",
            "W=123456789abcdef012345678",
            "XZ=1x2z",
            "BAD=5x9",
        ],
        WANT,
        "value_plusargs native",
    );
}

/// The X/Z INDEX rule, pinned to iverilog 13's values.
///
/// `offset_of_index_value` is shared by both backends (S2 slice 3 extracted it
/// so the specialized resolver could not restate it), which is exactly the
/// shape a VM-vs-native differential cannot see: deleting its `has_xz` drop
/// moves both sides together. Measured against iverilog: an x or z index makes
/// a WRITE vanish entirely and a READ all-x, while a NEGATIVE index still
/// partial-writes the in-range bits (`bus[-3 +: 4]` sets bit 0).
#[test]
fn s2_xz_index_is_dropped_matching_the_oracle() {
    let src = r#"
module top;
  reg [7:0] mem [0:3];
  reg [15:0] bus;
  reg [7:0] i;
  reg [63:0] big;
  integer k;
  initial begin
    mem[0]=8'd10; mem[1]=8'd11; mem[2]=8'd12; mem[3]=8'd13;
    bus = 16'h1234;
    i = 8'bxxxxxxxx;
    mem[i] = 8'd99;
    $display("t=%0t A %0d %0d %0d %0d", $time, mem[0], mem[1], mem[2], mem[3]);
    $display("t=%0t B %b", $time, mem[i]);
    bus[i +: 4] = 4'hF;
    $display("t=%0t C %h", $time, bus);
    i = 8'bzzzzzzzz;
    mem[i] = 8'd88;
    $display("t=%0t D %0d %b", $time, mem[3], mem[i]);
    k = -3;
    bus[k +: 4] = 4'hF;
    $display("t=%0t E %h", $time, bus);
    big = 64'h1_0000_0000;
    mem[big] = 8'd99;
    $display("t=%0t F %0d %b", $time, mem[0], mem[big]);
    $finish;
  end
endmodule
"#;
    // Every row is iverilog 13's, row F included.
    //
    // F used to be a HAND-IEEE pin the other way, on the argument that §5.2.1
    // has no truncation step so vita was "ahead of the oracle" in dropping a
    // beyond-32-bit index. §4.5.310 measured that cell against a SECOND oracle
    // and the argument did not survive: verilator 5.050 writes `mem[0] = 99`
    // too, so vita was not ahead, it was alone. An array-word index is now
    // read as a 32-bit integer (its low 32 bits, with any x/z above them still
    // poisoning it — rows B and D).
    //
    // The row keeps its teeth on the domain check for the same reason it had
    // them before: accepting any i128 turns F's write into the WRONG element,
    // since the truncated index is 0 and an untruncated one is not.
    const WANT: &[&str] = &[
        "out|t=0 A 10 11 12 13\n",
        "out|t=0 B xxxxxxxx\n",
        "out|t=0 C 1234\n",
        "out|t=0 D 13 xxxxxxxx\n",
        "out|t=0 E 1235\n",
        "out|t=0 F 99 01100011\n",
    ];
    both_backends_print(src, WANT, "x/z index rule");
}

/// Run `src` on both backends and assert the printed lines EXACTLY match
/// `want` — an absolute anchor, not a differential.
fn both_backends_print(src: &str, want: &[&str], what: &str) {
    both_backends_print_with_plusargs(src, &[], want, what)
}

/// `both_backends_print` with plusargs installed. A separate seam because the
/// harness parses SOURCE only — plusargs arrive from the CLI in production
/// (`simulate` copies `opts.plusargs` into the state), so any test of
/// `$value$plusargs` must set them here or it measures the MISS path only
/// (the recurring sidecar trap, this time confirmed LATENT by review before
/// any test fell into it).
fn both_backends_print_with_plusargs(src: &str, plusargs: &[&str], want: &[&str], what: &str) {
    let (ir, mut opts) = build_with_opts(src);
    opts.plusargs = plusargs.iter().map(|s| s.to_string()).collect();
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    for backend in [Backend::Bytecode, Backend::Native] {
        let sink = MergedSink::default();
        let r = simulate(
            &ir,
            &sink,
            SimOpts {
                backend,
                ..opts.clone()
            },
        );
        if backend == Backend::Native {
            assert_eq!(r.backend, Backend::Native, "fell back");
        }
        let lines: Vec<String> = sink
            .events
            .into_inner()
            .into_iter()
            // W4028 is let through as well: the plusargs anchor pins that the
            // WARNING is emitted on both backends at the same statement
            // position — a native arm that dropped the warn call would
            // otherwise be invisible to a value-only anchor. `MergedSink`
            // renders diagnostics as `diag|Severity|code|message`.
            .filter(|e| e.starts_with("out|t=") || e.contains("VITA-W4028"))
            .collect();
        assert_eq!(lines, want, "{backend:?}: {what} moved");
    }
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

/// The delta budget's BOUNDARY, not just which field it reads.
///
/// `s1d4c2c_delta_budget_is_not_the_body_budget` pins that the loop reads
/// `max_deltas` rather than `max_body_steps`, but its design finishes far below
/// the limit, so shifting the comparison (`> n` → `>= n + 5`) survives it. An
/// oscillator that PRINTS once per delta turns the budget into an observable
/// line count: both backends must cut off after the same number of deltas, and
/// must report the same termination.
#[test]
fn s1d4c2c_delta_limit_fires_at_the_same_delta() {
    let src = r#"
module top;
  reg a, b;
  initial begin a = 1'b0; b = 1'b0; end
  always @(a) begin b = ~a; $display("d"); end
  always @(b) begin a = b; end
  initial begin #100 $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    let mk = |backend| SimOpts {
        backend,
        max_deltas: 12,
        ..opts.clone()
    };
    let sink_vm = MergedSink::default();
    let sink_nat = MergedSink::default();
    let r_vm = simulate(&ir, &sink_vm, mk(Backend::Bytecode));
    let r_nat = simulate(&ir, &sink_nat, mk(Backend::Native));
    assert_eq!(r_nat.backend, Backend::Native, "fell back");
    let ev_vm = sink_vm.events.into_inner();
    let ev_nat = sink_nat.events.into_inner();
    assert_eq!(
        ev_vm, ev_nat,
        "the two backends cut the oscillator at a different delta"
    );
    assert_eq!(r_vm.finish_reason, r_nat.finish_reason);
    // ANTI-VACUITY: it must actually HIT the limit, and the line count must be
    // small enough that a shifted boundary changes it.
    assert_eq!(
        r_nat.finish_reason,
        FinishReason::DeltaLimit,
        "the design must hit the delta limit: {ev_nat:?}"
    );
    let lines = ev_nat.iter().filter(|e| e.starts_with("out|d")).count();
    assert!(
        (1..=13).contains(&lines),
        "expected the budget to bound the printed deltas, got {lines}"
    );
}

/// An ALREADY-TRUE `wait` in an unbounded loop terminates the same way on both
/// backends — the in-body step guard, reached through the fall-through.
///
/// Its own test because it needs a small `max_body_steps`: with the default
/// (100_000_000) the design spends four seconds counting to the limit before it
/// says anything.
///
/// ⚠️ This does NOT pin the guard CHARGE on the fall-through itself. Measured:
/// deleting `guard += 1` there leaves the gate green, because the loop's `Goto`
/// charges the bottom guard on every iteration too, and `F4027` carries no
/// count — so the charge only halves the iteration budget, which nothing
/// observes. The charge is kept for fidelity with `run_process`, not because a
/// design distinguishes it.
#[test]
fn s1d4c2d_already_true_wait_in_a_loop_hits_the_step_guard_alike() {
    let src = r#"
module top;
  reg x;
  initial begin x = 1'b1; end
  initial forever wait (x);
  initial begin #5 $finish; end
endmodule
"#;
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    let mk = |backend| SimOpts {
        backend,
        max_body_steps: 500,
        ..opts.clone()
    };
    let sink_vm = MergedSink::default();
    let sink_nat = MergedSink::default();
    let r_vm = simulate(&ir, &sink_vm, mk(Backend::Bytecode));
    let r_nat = simulate(&ir, &sink_nat, mk(Backend::Native));
    assert_eq!(r_nat.backend, Backend::Native, "fell back");
    let ev = sink_nat.events.into_inner();
    assert_eq!(sink_vm.events.into_inner(), ev, "the two backends differ");
    assert_eq!(r_vm.finish_reason, r_nat.finish_reason);
    // ANTI-VACUITY: it must be the STEP guard that ended it, not `$finish`.
    assert!(
        ev.iter().any(|e| e.contains("F4027")),
        "the design must hit the in-body step guard: {ev:?}"
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

/// S2 slice 5 — a concatenation's parts are evaluated in SOURCE order, and the
/// out-of-range accesses they COUNT come out in that order too.
///
/// The bits alone cannot show this: `{mem[xi], mem[oi]}` reads all-X either way,
/// so a compiler that emitted its parts LSB-first would produce an identical
/// value and reverse two diagnostics. The discriminator is that the two reports
/// have DIFFERENT CODES — an unknown index is `W4029`, a known out-of-range one
/// is `E4002` — which is the same distinction that forced the arena's deferred
/// record to be an ordered `Vec` instead of two counters.
#[test]
fn s2_concat_parts_report_out_of_range_in_source_order() {
    let src = r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] xi, oi;
  reg [15:0] r;
  initial begin
    mem[0]=8'd10; mem[1]=8'd11; mem[2]=8'd12; mem[3]=8'd13;
    xi = 8'bxxxxxxxx;
    oi = 8'd9;
    r = {mem[xi], mem[oi]};
    $display("A %b", r);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &[
        "diag|Warning|VITA-W4029|array word index is unknown (x/z); read X / write ignored",
        "diag|Error|VITA-E4002|array word index (out of range; read X / write ignored)",
        "out|A xxxxxxxxxxxxxxxx\n",
    ];
    both_backends_stream(src, WANT, "concat part order");
}

/// S2 slice 5 — a replication evaluates its operand ONCE.
///
/// `eval_replicate` evaluates `value` once and copies the bits `count` times, so
/// `{2{mem[oi]}}` must report ONE out-of-range access. A compiler that got the
/// bits right by compiling the operand `count` times would report twice; the
/// value is identical in both, which is why this row counts diagnostics and not
/// bits.
#[test]
fn s2_replicate_evaluates_its_operand_once() {
    let src = r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] oi;
  reg [15:0] r;
  initial begin
    mem[0]=8'd10; mem[1]=8'd11; mem[2]=8'd12; mem[3]=8'd13;
    oi = 8'd9;
    r = {2{mem[oi]}};
    $display("B %b", r);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &[
        "diag|Error|VITA-E4002|array word index (out of range; read X / write ignored)",
        "out|B xxxxxxxxxxxxxxxx\n",
    ];
    both_backends_stream(src, WANT, "replicate single evaluation");
}

/// `both_backends_print` with NOTHING filtered out.
///
/// ⚠️ `both_backends_print` keeps only `out|t=` lines (plus the one W4028
/// anchor), so it is structurally blind to diagnostics — which is exactly the
/// axis the two rows above are about. Reusing it would have made them vacuous,
/// and it did: both first passed against an empty stream.
fn both_backends_stream(src: &str, want: &[&str], what: &str) {
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts).expect("runnable");
    for backend in [Backend::Bytecode, Backend::Native] {
        let sink = MergedSink::default();
        let r = simulate(
            &ir,
            &sink,
            SimOpts {
                backend,
                ..opts.clone()
            },
        );
        if backend == Backend::Native {
            assert_eq!(r.backend, Backend::Native, "fell back");
        }
        let lines: Vec<String> = sink.events.into_inner();
        assert_eq!(lines, want, "{backend:?}: {what} moved");
    }
}

/// S2 slice 6 — the select windows the W compiler REFUSES, pinned to iverilog 13.
///
/// Slice 6 admits only a constant offset whose window lies wholly inside the
/// base, because the generic path's other two outcomes (a per-bit X-filled
/// overhang, and all-X for an unknown or oversized offset) have no counterpart
/// in a shift and a mask. Every row here is one of those outcomes, so the whole
/// row set exercises the DECLINE — and the battery's admitted rows cannot,
/// because an admitted row is by construction one that stays in range.
///
/// The values are iverilog's, not a vm/native differential: a mistake shared by
/// both backends is exactly what a differential cannot see, and the `-:` rule in
/// particular now lives in ONE function that both the generic evaluator and the
/// W compiler call.
#[test]
fn s2_out_of_range_selects_decline_and_match_the_oracle() {
    let src = r#"
module top;
  reg [3:0] a; integer i; reg [3:0] xi;
  initial begin
    a = 4'b1011; i = 1; xi = 4'bxx01;
    $display("A %b", a[4:1]);
    $display("B %b", a[9:6]);
    $display("C %b", a[i +: 2]);
    i = 3;
    $display("D %b", a[i +: 2]);
    $display("E %b", a[xi]);
    $display("F %b", a[3 -: 2]);
    $display("G %b", a[1 +: 2]);
    $finish;
  end
endmodule
"#;
    const WANT: &[&str] = &[
        "out|A x101\n", // overhang by one bit → per-bit X fill
        "out|B xxxx\n", // wholly outside
        "out|C 01\n",   // runtime offset, in range
        "out|D x1\n",   // runtime offset, overhanging
        "out|E x\n",    // x/z offset
        "out|F 10\n",   // `-:` at a constant offset — ADMITTED, and the one row
        "out|G 01\n",   // `+:` at a constant offset — ADMITTED
    ];
    both_backends_stream(src, WANT, "out-of-range select");
}

/// S2 slice 7 — a ternary's UNTAKEN branch must not report.
///
/// The W compiler has no control flow, so it evaluates both branches and then
/// selects; `eval_ctx` runs only the taken one. Every admitted op is pure with
/// exactly one exception — `LoadIdx` COUNTS an out-of-range element read — so
/// admission asks the compiled ops whether either branch can report and declines
/// if so. The values here are identical either way; what this row measures is
/// that no diagnostic appears, which is why it compares the whole stream.
///
/// ⚠️ Writing this row found the same defect in the tier-2 VM, which had the
/// same eager design and no guard: `--backend vm` (the DEFAULT) emitted an
/// `E4002` for the untaken branch and exited 1 where the interpreter, this
/// backend and iverilog all exited 0. Fixed in `native_eval::compile`'s
/// `Ternary` arm in the same slice, which is why this row runs on both backends
/// rather than pinning the native one alone.
#[test]
fn s2_ternary_untaken_branch_does_not_report() {
    let src = r#"
module top;
  reg [7:0] mem [0:3]; reg [7:0] oi, r; reg c;
  initial begin
    mem[0]=8'd10; mem[1]=8'd11; mem[2]=8'd12; mem[3]=8'd13;
    oi = 8'd9; c = 1'b1;
    r = c ? 8'hAA : mem[oi];
    $display("A %h", r);
    c = 1'b0;
    r = c ? mem[oi] : 8'hBB;
    $display("B %h", r);
    c = 1'bx;
    r = c ? 8'hAA : 8'hAA;
    $display("C %h", r);
    r = c ? 8'hA0 : 8'hB0;
    $display("D %h", r);
    $finish;
  end
endmodule
"#;
    // iverilog 13's, and NO diagnostic on any row. `C` is the one that separates
    // the unknown-condition MERGE from an unconditional X: both branches are
    // `8'hAA`, so every bit agrees and the result is `aa`, not `xx`.
    const WANT: &[&str] = &["out|A aa\n", "out|B bb\n", "out|C aa\n", "out|D X0\n"];
    both_backends_stream(src, WANT, "ternary laziness");
}

// ─────────────────────────────────────────────────────────────────────────────
// S3 slice 1 — THE COMPILED BODY.
//
// Tier-3 gained a second executor: `backend::vm_exec` over a `CompiledBody`,
// chosen per template by `NativeKernel::compiled_for`. It is not a second
// SEMANTICS — `vm_exec` calls the same `Kernel` methods `compute_effect`/
// `apply_effect` call, in the same order — but "not a second semantics" is a
// claim, and the two are different code. So the gate is the same shape every
// earlier tier-3 slice used: run the SAME design twice on the SAME backend with
// only the executor switched, and compare everything the user can see.
//
// The switch has to survive to run time (`USE_COMPILED`) because a funnel that
// delegates cannot be its own oracle, and because what is compared here is two
// whole RUNS, not two calls.
// ─────────────────────────────────────────────────────────────────────────────

/// Run `src` natively twice — compiled bodies on, then off — and assert the two
/// executors agree on everything observable.
///
/// Returns `Err` when the gate refuses the design, so the caller can count
/// refusals instead of silently passing them.
fn compiled_agrees_with_walk(src: &str, name: &str) -> Result<u64, &'static str> {
    use crate::native::kernel::{COMPILED_ACTIVATIONS, USE_COMPILED};
    let (ir, opts) = build_with_opts(src);
    crate::native::run::runnable(&ir, &opts)?;

    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir();
    let tag = format!(
        "vita_s3_{}_{}_{}",
        name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>(),
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let vcd_c = dir.join(format!("{tag}_c.vcd"));
    let vcd_w = dir.join(format!("{tag}_w.vcd"));
    let _ = std::fs::remove_file(&vcd_c);
    let _ = std::fs::remove_file(&vcd_w);

    let run = |use_compiled: bool, vcd: &std::path::Path| {
        USE_COMPILED.with(|c| c.set(use_compiled));
        COMPILED_ACTIVATIONS.with(|c| c.set(0));
        let sink = MergedSink::default();
        let res = simulate(
            &ir,
            &sink,
            SimOpts {
                backend: Backend::Native,
                vcd_path_override: Some(vcd.to_string_lossy().into_owned()),
                ..opts.clone()
            },
        );
        USE_COMPILED.with(|c| c.set(true));
        let acts = COMPILED_ACTIVATIONS.with(|c| c.get());
        (res, sink.events.into_inner(), acts)
    };
    let (r_c, out_c, acts_c) = run(true, &vcd_c);
    let (r_w, out_w, acts_w) = run(false, &vcd_w);

    // ANTI-VACUITY, in three parts, all of them measured failures of an earlier
    // draft of this gate:
    //  1. `Backend::Native` FALLS BACK to the VM when a gate refuses, and then
    //     both runs are the same run.
    //  2. `USE_COMPILED = false` must actually reach the walk, or the "walk" run
    //     is a second compiled run.
    //  3. The compiled run must actually have COMPILED something, or both runs
    //     are the walk. (This is the one that bites: `compiled_for` returning
    //     `None` for every template — a narrowed `is_codegen_able`, a poisoned
    //     cache — leaves every assertion below trivially true.)
    assert_eq!(
        r_c.backend,
        Backend::Native,
        "{name}: fell back to {:?} (refused {:?})",
        r_c.backend,
        r_c.native.refused
    );
    assert_eq!(acts_w, 0, "{name}: the walk run executed a compiled body");

    assert_eq!(
        out_c, out_w,
        "{name}: the interleaved stdout+diagnostic stream differs between the \
         compiled executor and the walk"
    );
    assert_eq!(
        r_c.finish_reason, r_w.finish_reason,
        "{name}: finish reason differs"
    );
    assert_eq!(r_c.sim_time, r_w.sim_time, "{name}: end time differs");
    assert_eq!(r_c.exit_class, r_w.exit_class, "{name}: exit class differs");
    let b_c = std::fs::read(&vcd_c).ok();
    let b_w = std::fs::read(&vcd_w).ok();
    if src.contains("$dumpvars") {
        assert!(
            b_c.is_some() && b_w.is_some(),
            "{name}: the design dumps but a run produced no VCD"
        );
    }
    assert_eq!(b_c, b_w, "{name}: VCD bytes differ");
    let _ = std::fs::remove_file(&vcd_c);
    let _ = std::fs::remove_file(&vcd_w);
    Ok(acts_c)
}

/// THE S3-1 GATE: every corpus design, compiled executor vs walk.
#[test]
fn s3_compiled_body_matches_the_walk_over_corpus() {
    let mut ran = 0usize;
    let mut compiled = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in corpus_designs() {
        match compiled_agrees_with_walk(&src, &name) {
            Ok(acts) => {
                ran += 1;
                if acts > 0 {
                    compiled += 1;
                }
            }
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    // EXACT, matching the sibling corpus gate — and `compiled` is pinned APART
    // from `ran` because the two are very different numbers.
    //
    // ⚠️ **42 of the 72 comparisons are vacuous, and the count says so rather
    // than hiding it.** A corpus design is generated as `initial begin … #d … end`
    // more often than not, and a body with a `Delay` is not codegen-able, so on
    // 42 designs NOTHING compiles and this test compares the walk with itself.
    // That is why `s3_compiled_body_matches_the_walk_on_discriminating_designs`
    // exists and why its rows assert `acts > 0` individually: the corpus proves
    // the executor does not BREAK the designs it does not compile, and the
    // dedicated designs are what actually exercise the compiled arms.
    assert_eq!(
        (ran, refused.len(), compiled),
        (72, 0, 30),
        "corpus coverage moved — re-pin deliberately. ran={ran} compiled={compiled} \
         refused={refused:?}"
    );
}

/// The corpus's `$dumpvars` designs, compiled, waveform for waveform.
#[test]
fn s3_compiled_body_waveforms_match_the_walk() {
    let mut with_dump = 0usize;
    let mut ran = 0usize;
    for (name, src) in corpus_designs() {
        if !src.contains("$dumpvars") {
            continue;
        }
        with_dump += 1;
        if compiled_agrees_with_walk(&src, &name).is_ok() {
            ran += 1;
        }
    }
    assert_eq!((with_dump, ran), (44, 44), "corpus VCD population moved");
}

/// The shapes the corpus does not separate, each named for the rule it covers.
#[test]
fn s3_compiled_body_matches_the_walk_on_discriminating_designs() {
    for (name, src) in super::run_tests::s3_discriminating_designs() {
        let acts = compiled_agrees_with_walk(&src, name)
            .unwrap_or_else(|r| panic!("{name}: the gate refused a design built for it: {r}"));
        assert!(
            acts > 0,
            "{name}: nothing compiled — this row compared the walk with itself"
        );
    }
}

/// Designs that separate a rule the corpus leaves untested.
pub(super) fn s3_discriminating_designs() -> Vec<(&'static str, String)> {
    vec![
        // ── THE STATEMENT BOUNDARY ──
        //
        // `vm_exec` had no notion of one until this slice, and for tier-3
        // `k_drain_diags` is NOT a no-op: the arena RECORDS an out-of-range
        // access and reports it at the boundary. Without `Op::ends_statement`
        // every E4002 in a compiled body would come out AFTER the body instead
        // of interleaved with its `$display` lines — same bytes on stdout, same
        // exit code, different ORDER in the merged stream. That is what this row
        // reads.
        (
            "range_diag_interleaves_with_display",
            r#"
module top;
  reg [7:0] mem [0:3];
  reg [7:0] r; integer i;
  initial begin
    mem[0]=8'd10; mem[1]=8'd11; mem[2]=8'd12; mem[3]=8'd13;
    $display("before");
    i = 9; r = mem[i];
    $display("mid r=%0d", r);
    i = 12; r = mem[i];
    $display("after r=%0d", r);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // The same rule with the out-of-range access in the LVALUE, so the
        // diagnostic is produced by the write half rather than the read half.
        (
            "range_diag_on_lvalue_interleaves",
            r#"
module top;
  reg [7:0] mem [0:3];
  integer i;
  initial begin
    $display("before");
    i = 7; mem[i] = 8'd1;
    $display("mid");
    i = 2; mem[i] = 8'd2;
    $display("after mem2=%0d", mem[2]);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── THE FUSED OPS ──
        //
        // `EvalWriteScalar`/`EvalNbaScalar` are emitted only when the
        // destination is a compile-time-proven plain scalar. A design with BOTH
        // kinds of destination in one body separates the fused arm from the
        // general one; without the array half, a `plain_scalar_dest` that
        // wrongly said `Some` for everything would still pass.
        (
            "fused_and_general_destinations_in_one_body",
            r#"
module top;
  reg [7:0] s, t;      // plain scalars -> fused
  reg [7:0] mem [0:3]; // array         -> general
  reg [7:0] w;
  reg clk;
  initial clk = 1'b0;
  always #5 clk = ~clk;
  always @(posedge clk) begin
    s <= s + 8'd1;          // EvalNbaScalar
    mem[s[1:0]] <= s;       // ScheduleNba (general)
    t = s ^ 8'h5a;          // EvalWriteScalar
    w[3:0] = t[7:4];        // WriteLval (part-select -> general)
  end
  initial begin
    s = 8'd0; t = 8'd0; w = 8'd0;
    mem[0]=0; mem[1]=0; mem[2]=0; mem[3]=0;
    #100;
    $display("s=%0d t=%0d w=%h m=%0d %0d %0d %0d", s, t, w, mem[0], mem[1], mem[2], mem[3]);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── THE FUSED ASSIGNMENT'S CONTEXT RULE ──
        //
        // `k_eval_write_scalar`/`k_eval_nba_scalar` take the plane shortcut, and
        // the one rule they restate is the assignment context width,
        // `max(lhs, self(rhs))`, with the lhs half read from the arena slot
        // instead of walked. Every shape where the two halves DIFFER is here:
        // lhs wider than a signed rhs (sign extension), lhs wider than an
        // unsigned rhs (zero fill), lhs narrower (truncation), and equal.
        //
        // A mutation that drops the `max` is value-equivalent (the fallback and
        // the shortcut reach the same extension by different routes), so this
        // row is coverage of the shape rather than a killer — and saying which
        // it is beats implying the other.
        (
            "fused_assignment_context_widths",
            r#"
module top;
  reg signed [7:0]  sb;
  reg        [7:0]  ub;
  reg signed [15:0] sw16;
  reg        [15:0] uw16;
  reg        [3:0]  n4;
  reg        [15:0] q;
  // STRAIGHT-LINE, so this block is codegen-able and takes the fused path. A
  // `#1` anywhere in it would put a `Delay` in the body and send the WHOLE
  // process back to the walk — which is how the first draft of this row ended
  // up comparing the walk with itself (caught by the per-row `acts > 0`).
  initial begin
    sb = -8'sd3; ub = 8'hF1;
    sw16 = sb;          // narrower SIGNED rhs -> sign extend
    uw16 = sb;          // same rhs, unsigned destination
    n4   = ub;          // wider rhs -> truncate
    q    = ub;          // narrower UNSIGNED rhs -> zero fill
    $display("%0d %h %h %h", sw16, uw16, n4, q);
    q <= sb;            // the NBA twin of the same rule
  end
  initial begin
    #1 $display("q=%h", q);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── THE PER-PROCESS PROLOGUE (`k_enter_body`) ──
        //
        // The walk calls it inside itself; `vm_exec` leaves it to the caller, so
        // `dispatch_body` has to. It installs THREE per-process facts —
        // `cur_time_mult` and `cur_prec_mult` (the `timescale` `$time` is scaled
        // by) and `cur_scope` (`%m`) — and none of them is visible unless two
        // processes with DIFFERENT ones interleave. Omitting the call then
        // leaves whatever the previous process installed, so every line reads as
        // if it came from that other module.
        //
        // Separated by `%m` rather than by `timescale`: this harness parses the
        // token stream directly and never runs the preprocessor, so a
        // `` `timescale `` directive is a parse error here. `%m` needs no
        // directive — two INSTANCES of one module already have two scopes — and
        // it is carried by the same `enter_body` call, so the row covers the
        // prologue even though it separates only one of its three facts.
        // Measured: deleting `k_enter_body` from `dispatch_body` survived the
        // whole suite until this row existed.
        (
            "enter_body_installs_the_scope_per_process",
            r#"
module leaf(input wire clk);
  reg [7:0] n;
  initial n = 8'd0;
  always @(posedge clk) begin
    n = n + 8'd1;
    $display("%m n=%0d", n);
  end
endmodule

module top;
  reg clk;
  leaf a(clk);
  leaf b(clk);
  initial begin
    clk = 1'b0;
    #1 clk = 1'b1;
    #1 clk = 1'b0;
    #1 clk = 1'b1;
    #1 $display("%m done");
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // ── `k_call_fatal` AT THE STATEMENT BOUNDARY ──
        //
        // `is_codegen_able` excludes a user call from every expression position
        // it scans — and its own comment says "any expr position that can REACH
        // a frame Call excludes the body" — but the walk it performs covers
        // `rhs`, `delay`, `fmt` and `args`, NOT an lvalue's INDEX expression. So
        // `mem[idx(i)] = …` compiles, `idx` runs in a frame, and a `$fatal`
        // inside it latches `call_fatal` during `ResolveOff`.
        //
        // Two rules meet here and the row separates both: the fatal must STOP
        // the body (or `$display("survived")` prints), and it must stop it AFTER
        // the write (or the `E4002` that out-of-range write owes is lost). That
        // is why the boundary is the end of the STATEMENT and not the end of
        // every op.
        (
            "fatal_in_an_lvalue_index_call_stops_the_body",
            r#"
module top;
  reg [7:0] mem [0:3];
  integer i;
  function integer idx(input integer x);
    begin
      if (x > 100) $fatal(1, "idx too big");
      idx = x;
    end
  endfunction
  initial begin
    i = 200;
    mem[idx(i)] = 8'd7;
    $display("survived i=%0d", i);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A REAL VALUE into a plain-scalar destination — the arm
        // `write_lvalue`'s real→int coercion owns, reached through the FUSED op.
        //
        // Spelled without a real NET on purpose, and that is not a stylistic
        // choice: a `real` declaration makes the whole design tier-3-INELIGIBLE
        // (an S0 row), so the only way this arm is reachable at all is the one
        // `write.rs` names — "`x = $itor(n)/2.0` needs no real NET anywhere".
        // A first draft used `real x;` and the gate refused the design, which is
        // the cheapest possible demonstration that the two facts are linked.
        (
            "real_value_into_a_fused_scalar_destination",
            r#"
module top;
  reg [15:0] r; integer n;
  initial begin
    n = 15; r = $itor(n) / 2.0;   // 7.5 -> rounds
    $display("r=%0d", r);
    n = -5; r = $itor(n) / 2.0;   // -2.5 -> rounds
    $display("r=%0d", r);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // A branch whose condition is evaluated by `k_truthy` on both executors,
        // plus a `Goto` back-edge, so the compiled control flow is exercised
        // rather than a straight line.
        (
            "loop_and_branch_control_flow",
            r#"
module top;
  reg [7:0] acc; integer i;
  initial begin
    acc = 8'd0;
    for (i = 0; i < 10; i = i + 1) begin
      if (i[0]) acc = acc + i[7:0];
      else      acc = acc - i[7:0];
    end
    $display("acc=%0d", acc);
    $finish;
  end
endmodule
"#
            .to_string(),
        ),
        // `$finish` from inside a compiled body — `Op::SysTask`'s `Ctl::Finish`
        // arm, which returns out of `vm_exec` mid-block.
        (
            "finish_mid_block_from_a_compiled_body",
            r#"
module top;
  reg [7:0] a;
  initial begin
    a = 8'd1;
    $display("one a=%0d", a);
    $finish;
    a = 8'd2;
    $display("two a=%0d", a);
  end
endmodule
"#
            .to_string(),
        ),
    ]
}

/// Slice #1 ABSOLUTE ANCHOR — a clocking block on tier-3, hand-IEEE.
///
/// ⚠️ **Neither oracle pins this one, and for two different reasons.** iverilog
/// 13 does not parse `clocking` at all (`syntax error` on the block header), so
/// the family is invisible to it. verilator 5.050 DOES support clocking blocks
/// and disagrees with vita on purpose: it samples in the Observed region, so an
/// Active-region `always @(posedge clk)` reads the PREVIOUS edge's sample
/// (`cb.d` = 0,1,2 for `d` = 1,2,3), where vita commits at edge DETECTION and
/// reads THIS edge's (`cb.d` = 1,2,3). That is the engine's documented hand-IEEE
/// simplification (`clocking_commit_plan`), predates this slice, and is shared by
/// all three vita backends — pinning verilator's numbers here would pin a model
/// vita does not implement. So the expectation is vita's own model, stated.
///
/// Every line is a different part of the machinery:
///
/// * `P t=5 … cb.d=9` — the PREPONED snapshot is taken through the store that
///   RAN. An unthreaded snapshot reads the engine's `d` slot, which a native run
///   never writes, so it would report the declaration initializer (7) forever.
/// * `drv=a5` / `drv=5a` — the OUTPUT phase: `cb.drv = …` writes the holding net
///   and the commit drives the real net at the next clocking edge. Its read of
///   the holding net is the second threaded read, and its write is the sample
///   write; both must land in the arena.
/// * `LVL` — a LEVEL-sensitive process on a holding net, and the ONLY line that
///   can see the shared `store_sample_words` change verdict. The t=25 commit
///   re-samples the same value (11), so a verdict stuck at "changed" prints a
///   third `LVL`. A native/VM differential is blind to that by construction
///   (§5.1-e: the function is shared), which is why this line is here and not
///   in the differential.
/// * `HOLD-EDGE` — a holding net that is ITSELF an edge target
///   (`always @(posedge cg.g)`). This is the `accumulate_edge` obligation: with
///   only `note_change` the mask is RESET and the posedge is silently lost.
/// * two clocking blocks on OPPOSITE edges — the handler diversion is per-proc,
///   and `cg` has inputs only while `cb` has both, so the `inputs.is_none() &&
///   outputs.is_none()` split is exercised in both directions.
/// * `d` moving between the edge and the display — `cb.d` must be the PREPONED
///   value, not the live one, which is what makes the buffer observable at all.
#[test]
fn a_clocking_design_runs_on_tier_3() {
    let src = r#"
module t;
  logic clk = 1'b0;
  logic [7:0] d = 8'd7;
  logic [7:0] drv;
  logic       g = 1'b0;
  clocking cb @(posedge clk);
    input  d;
    output drv;
  endclocking
  clocking cg @(negedge clk);
    input  g;
  endclocking
  always #5 clk = ~clk;
  always @(posedge cg.g) $display("HOLD-EDGE t=%0t", $time);
  always @(cb.d) $display("LVL t=%0t cb.d=%0d", $time, cb.d);
  initial begin
    #1 d = 8'd9; g = 1'b1;
    #6 d = 8'd11;
    cb.drv = 8'hA5;
    #10 cb.drv = 8'h5A;
    #12 $display("END drv=%0h", drv);
    $finish;
  end
  always @(posedge clk) $display("P t=%0t d=%0d cb.d=%0d drv=%0h", $time, d, cb.d, drv);
  always @(negedge clk) $display("N t=%0t g=%0b cg.g=%0b", $time, g, cg.g);
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|P t=5 d=9 cb.d=9 drv=xx\n".to_string(),
            "out|LVL t=5 cb.d=9\n".to_string(),
            "out|N t=10 g=1 cg.g=1\n".to_string(),
            "out|HOLD-EDGE t=10\n".to_string(),
            "out|P t=15 d=11 cb.d=11 drv=a5\n".to_string(),
            "out|LVL t=15 cb.d=11\n".to_string(),
            "out|N t=20 g=1 cg.g=1\n".to_string(),
            "out|P t=25 d=11 cb.d=11 drv=5a\n".to_string(),
            "out|END drv=5a\n".to_string(),
        ],
        "clocking on tier-3 (hand-IEEE: vita commits at edge detection)"
    );
}

/// Slice #1 DIFFERENTIAL — clocking shapes, native against the VM.
#[test]
fn clocking_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // A clocking edge in the t=0 slot itself. The seed snapshot taken
        // before the loop is the only thing that makes this sample non-X, and
        // `a` is moved in the SAME slot so the printed 3 can only be the
        // preponed value.
        //
        // ⚠️ The first version of this row wrote `reg clk = 1'b1;` and no
        // driver, so the posedge NEVER HAPPENED and nothing was ever committed —
        // the row passed while the seed did nothing. The battery found it: the
        // mutation that deletes the t0 seed survived. A clocking row that does
        // not produce an edge measures nothing.
        (
            "clocking input sampled at a t0 edge",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] a = 8'd3;
  clocking cb @(posedge clk);
    input a;
  endclocking
  initial begin
    clk = 1'b1;
    a = 8'd200;
    #1 $display("A cb.a=%0d a=%0d", cb.a, a);
    $finish;
  end
endmodule
"#,
        ),
        // An input whose SOURCE moves inside the very slot the edge lands in:
        // the commit must use the preponed value, not the settled one.
        (
            "source moves within the sampling slot",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] a = 8'd1;
  clocking cb @(posedge clk);
    input a;
  endclocking
  always #5 clk = ~clk;
  always @(posedge clk) begin a = a + 8'd1; $display("S a=%0d cb.a=%0d", a, cb.a); end
  initial #26 $finish;
endmodule
"#,
        ),
        // An OUTPUT clockvar only — the handler has `clocking_outputs` and no
        // `clocking_commit`, so the plan's input half is empty.
        (
            "output clockvar only",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] o;
  clocking cb @(posedge clk);
    output o;
  endclocking
  always #5 clk = ~clk;
  initial begin
    cb.o = 8'h11;
    #12 cb.o = 8'h22;
    #12 $display("O o=%0h", o);
    $finish;
  end
endmodule
"#,
        ),
        // A holding net that drives a CONTINUOUS ASSIGN, so the commit's
        // `note_change` has to reach the cont-assign worklist too.
        (
            "holding net feeds a continuous assign",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] a = 8'd4;
  wire [7:0] y;
  clocking cb @(posedge clk);
    input a;
  endclocking
  assign y = cb.a + 8'd1;
  always #5 clk = ~clk;
  initial begin #1 a = 8'd6; #10 $display("Y y=%0d", y); #10 $display("Y y=%0d", y); $finish; end
endmodule
"#,
        ),
        // A clocking edge whose slot ALSO wakes an ordinary process on the same
        // net — the diversion must not consume that process's wake.
        (
            "handler shares its clock with an always block",
            r#"
module top;
  reg clk = 1'b0;
  reg [7:0] a = 8'd2;
  reg [7:0] c = 8'd0;
  clocking cb @(posedge clk);
    input a;
  endclocking
  always #5 clk = ~clk;
  always @(posedge clk) c <= c + 8'd1;
  always @(posedge clk) $display("C c=%0d cb.a=%0d", c, cb.a);
  initial #26 $finish;
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 5, "slice #1 differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "slice #1 differential: refusal breakdown moved"
    );
}

/// Slice #2 ABSOLUTE ANCHOR — force/release on tier-3, **iverilog-pinned**.
///
/// This half of the family HAS an oracle and every value below is `vvp`'s.
/// Each line is a different piece of the machinery:
///
/// * **B** — the pin itself lands, on a WIRE (`w`, driven by a continuous
///   assign) and on a VARIABLE (`v`). Both go through this store's funnel.
/// * **B → y=ff** — the pin PROPAGATES: `assign y = w ^ 8'h0F` re-settles off
///   the forced value, so the force write has to enter the dirty channel like
///   any other. A pin written into the wrong store leaves `y` at `c`.
/// * **C** — a normal driver is SUPPRESSED while forced, on both target kinds:
///   `a = 5` moves `w`'s continuous assign and `v = 77` is an ordinary
///   procedural write, and NEITHER lands. That is the `forced` gate inside the
///   arena's `write_chunk`, threaded from `SimState` rather than mirrored.
/// * **D** — `release` on the WIRE snaps back **in this timestep** (`w=7`,
///   recomputed from `a=5`) while `release` on the VARIABLE keeps the forced
///   value (`v=5a`, §9.3.1 — a variable has no driver to snap back to).
///   ⚠️⚠️ The wire half is a PRE-EXISTING silent-wrong this slice fixed: before
///   it, all three vita backends reported `w=240` here because clearing the
///   flag moves no net and nothing re-dirtied the driving assign. iverilog and
///   verilator both say 7.
/// * **E** — after release the variable takes ordinary writes again.
#[test]
fn force_and_release_have_their_iverilog_values_on_tier_3() {
    let src = r#"
module top;
  reg  [7:0] a = 8'd1;
  reg  [7:0] b = 8'd2;
  wire [7:0] w;
  reg  [7:0] v = 8'd9;
  wire [7:0] y;
  assign w = a + b;
  assign y = w ^ 8'h0F;
  initial begin
    #1 $display("A w=%0d v=%0h y=%0h", w, v, y);
    force w = 8'hF0;
    force v = 8'h5A;
    #1 $display("B w=%0d v=%0h y=%0h", w, v, y);
    a = 8'd5;
    v = 8'd77;
    #1 $display("C w=%0d v=%0h y=%0h", w, v, y);
    release w;
    release v;
    #1 $display("D w=%0d v=%0h y=%0h", w, v, y);
    v = 8'd33;
    #1 $display("E w=%0d v=%0h y=%0h", w, v, y);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A w=3 v=9 y=c\n".to_string(),
            "out|B w=240 v=5a y=ff\n".to_string(),
            "out|C w=240 v=5a y=ff\n".to_string(),
            "out|D w=7 v=5a y=8\n".to_string(),
            "out|E w=7 v=21 y=8\n".to_string(),
        ],
        "force/release (iverilog-pinned)"
    );
}

/// Slice #2 ABSOLUTE ANCHOR — the CONTINUOUS half, **hand-IEEE**.
///
/// ⚠️ **iverilog is not an oracle for this half and says so**: it prints
/// *"sorry: procedural continuous assignments are not yet fully supported. The
/// RHS of this assignment will only be evaluated once"* and then reports `101`
/// where §9.3.2 requires `105`. vita re-evaluates, so these are vita's own
/// values with the LRM as the authority — one of the few places the repo is
/// ahead of its oracle (`a_virtual_call_dispatches_dynamically_on_tier_3` is
/// the other).
///
/// * **A/B** — a `force` with an EXPRESSION RHS re-evaluates when its input
///   moves (§9.3.2). This is the fixpoint in `propagate`, and it is the only
///   line that shows it: a wired-but-never-re-evaluated force prints `101`
///   forever, which is exactly iverilog's answer and therefore a plausible one.
/// * **C** — a real `force` DISPLACES an active procedural `assign`, which is
///   parked latent rather than dropped.
/// * **G/H** — and the MIRROR direction, which the first version of this test
///   did not have: an `assign` issued while a force is already live is parked
///   IMMEDIATELY and writes nothing (`u` stays `aa`, not `65`), then takes
///   control at `release` (`105`). ⚠️ The mutation that made `force_prologue`
///   return "write anyway" SURVIVED until this pair existed, and a differential
///   cannot see it — `force_prologue` is shared code (ROADMAP §5.1-e).
/// * **D** — `release` hands control BACK to the parked assign, re-evaluated at
///   that moment (`a` is 5 by then, so `105` rather than the parked `101`).
/// * **E** — `deassign` drops it; the variable HOLDS (§9.3.1).
/// * **F** — and then takes ordinary writes.
#[test]
fn a_procedural_assign_and_an_expression_force_re_evaluate_on_tier_3() {
    let src = r#"
module top;
  reg [7:0] v = 8'd9;
  reg [7:0] a = 8'd1;
  reg [7:0] u = 8'd9;
  initial begin
    #1 assign v = a + 8'd100;
    #1 $display("A v=%0d", v);
    a = 8'd5;
    #1 $display("B v=%0d", v);
    force v = 8'hAA;
    #1 $display("C v=%0d", v);
    release v;
    #1 $display("D v=%0d", v);
    deassign v;
    #1 $display("E v=%0d", v);
    v = 8'd3;
    #1 $display("F v=%0d", v);
    force u = 8'hAA;
    #1 assign u = a + 8'd100;
    #1 $display("G u=%0h", u);
    release u;
    #1 $display("H u=%0d", u);
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A v=101\n".to_string(),
            "out|B v=105\n".to_string(),
            "out|C v=170\n".to_string(),
            "out|D v=105\n".to_string(),
            "out|E v=105\n".to_string(),
            "out|F v=3\n".to_string(),
            "out|G u=aa\n".to_string(),
            "out|H u=105\n".to_string(),
        ],
        "procedural assign/deassign + expression force (hand-IEEE §9.3.1/§9.3.2; \
         iverilog evaluates the RHS once and reports 101)"
    );
}

/// Slice #2 DIFFERENTIAL — force/release shapes, native against the VM.
#[test]
fn force_release_shapes_match_the_vm() {
    let designs: Vec<(&str, &str)> = vec![
        // A force whose RHS is VOLATILE (`$time`): it has no net inputs, so it
        // rides `force_always_reeval` rather than the net→forces sidecar — the
        // one selection path a net-sensitivity skip would silently freeze.
        (
            "volatile force rhs",
            r#"
module top;
  reg [7:0] v = 8'd0;
  reg clk = 1'b0;
  always #1 clk = ~clk;
  initial begin
    force v = $time;
    #1 $display("V %0d", v);
    #2 $display("V %0d", v);
    #2 $display("V %0d", v);
    $finish;
  end
endmodule
"#,
        ),
        // A force CHAIN: one force's target feeds another force's RHS, so the
        // fixpoint has to converge WITHIN one propagate rather than across
        // deltas.
        (
            "force feeds force",
            r#"
module top;
  reg [7:0] s = 8'd1;
  reg [7:0] m = 8'd0;
  reg [7:0] t = 8'd0;
  initial begin
    force m = s + 8'd1;
    force t = m + 8'd1;
    #1 $display("F m=%0d t=%0d", m, t);
    s = 8'd10;
    #1 $display("F m=%0d t=%0d", m, t);
    $finish;
  end
endmodule
"#,
        ),
        // A forced net that is an EDGE TARGET: the pin must feed `slot_edge`
        // through the normal funnel, so `always @(posedge f)` still fires.
        (
            "force on an edge target",
            r#"
module top;
  reg f = 1'b0;
  reg [7:0] n = 8'd0;
  always @(posedge f) n = n + 8'd1;
  initial begin
    #1 force f = 1'b1;
    #1 force f = 1'b0;
    #1 force f = 1'b1;
    #1 $display("E n=%0d", n);
    $finish;
  end
endmodule
"#,
        ),
        // A re-force while already forced: the pin write must go THROUGH the
        // flag it maintains (`force_lift`/`force_pin`), or the second force is
        // suppressed by the first.
        (
            "re-force while forced",
            r#"
module top;
  reg [7:0] v = 8'd0;
  initial begin
    force v = 8'd11;
    #1 force v = 8'd22;
    #1 $display("R v=%0d", v);
    release v;
    #1 $display("R v=%0d", v);
    $finish;
  end
endmodule
"#,
        ),
        // Normal drivers SUPPRESSED, observed in the SAME delta as the writes.
        //
        // ⚠️⚠️ This row exists because the mutations that delete the arena's
        // force gate SURVIVED without it, and the reason is worth keeping: with
        // continuous re-evaluation live, a leaked write is REPAIRED by the next
        // re-pin, so any observation after a `#` delay sees the right value
        // anyway. What cannot be repaired is what happened in between — the
        // `$display` before the delay, and the posedge on `f` that an
        // edge-sensitive process already counted. Both funnels are covered:
        // `v` is a one-word destination (the `write_chunk_word` entry) and
        // `wide` is 96 bits (the general `write_chunk`).
        (
            "forced nets suppress their normal drivers",
            r#"
module top;
  reg [7:0]  v = 8'd0;
  reg [95:0] wide = 96'd0;
  reg        f = 1'b0;
  reg [7:0]  n = 8'd0;
  always @(posedge f) n = n + 8'd1;
  initial begin
    force v = 8'h5A;
    force wide = 96'h1;
    force f = 1'b0;
    #1;
    v = 8'd77;
    wide = {96{1'b1}};
    f = 1'b1;
    $display("S v=%0h wide=%0h n=%0d", v, wide, n);
    #1 $display("T v=%0h wide=%0h n=%0d", v, wide, n);
    $finish;
  end
endmodule
"#,
        ),
        // `release` on a driven WIRE, which must snap back in this timestep.
        //
        // ⚠️ The mutation that reverts the ENGINE half of that fix survived
        // until this row existed: every other force row here releases a
        // variable, which has no driver to snap back to.
        (
            "release on a driven wire snaps back",
            r#"
module top;
  reg [7:0] a = 8'd1;
  wire [7:0] w;
  assign w = a + 8'd2;
  initial begin
    #1 force w = 8'hE0;
    #1 $display("W w=%0d", w);
    release w;
    #1 $display("W w=%0d", w);
    a = 8'd5;
    #1 $display("W w=%0d", w);
    $finish;
  end
endmodule
"#,
        ),
        // A forced net inside a CONCAT lvalue: only the forced chunk is
        // dropped, which is why the gate is per-chunk rather than per-lvalue.
        (
            "concat lvalue with one forced chunk",
            r#"
module top;
  reg [3:0] hi = 4'd0;
  reg [3:0] lo = 4'd0;
  initial begin
    force hi = 4'hA;
    #1 {hi, lo} = 8'h5C;
    #1 $display("K hi=%0h lo=%0h", hi, lo);
    $finish;
  end
endmodule
"#,
        ),
    ];
    let mut ran = 0usize;
    let mut refused: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for (name, src) in &designs {
        match agree(src, name) {
            Ok(()) => ran += 1,
            Err(r) => *refused.entry(r).or_default() += 1,
        }
    }
    assert_eq!(ran, 7, "slice #2 differential: runnable count moved");
    assert_eq!(
        refused,
        Default::default(),
        "slice #2 differential: refusal breakdown moved"
    );
}

/// Slice #3 ABSOLUTE ANCHOR — a frame-local `new[]` and a `disable`,
/// **iverilog-pinned**.
///
/// The row this opens said "a subroutine statement the frame executor drops",
/// and the census says what actually reached it: **13 designs are `new[]`, 2 are
/// `disable`, nothing else**. Both are arms `run_frame_call` already executes,
/// so the row was conservative — with one thing genuinely wrong underneath, and
/// **line B is the only line that shows it**.
///
/// * **A/B** — `d = new[n]` sizes from a MODULE net. `frame_dyn_new` read that
///   size through `mk_eval_ctx` (the ENGINE's nets), which a native run never
///   writes. `A` cannot see it (`n`'s declaration initializer is in the engine's
///   slot too, so both stores happen to say 5); `B` re-runs after `n = 8` and the
///   unthreaded read still says 5 → `508` where the design says `821`. Exactly V1
///   slice 2c's `d = new[n]` defect, reached through the `&self` executor
///   instead of the module process.
/// * **C** — `disable blk` inside a function body. Elaborate lowers it as a
///   marker plus a sibling `Goto`, so the executor's default arm IS its correct
///   execution: `g(0)` leaves `d[0]` at its `new[]` default (0 → `30`) while
///   `g(1)` reaches the assignment (`37`).
#[test]
fn a_frame_local_new_and_disable_have_their_iverilog_values() {
    let src = r#"
module top;
  int n = 5;
  int m = 3;
  function automatic int f(input int k);
    int d[];
    int i;
    d = new[n];
    for (i = 0; i < d.size(); i = i + 1) d[i] = i * k;
    f = d.size() * 100 + d[d.size()-1];
  endfunction
  function automatic int g(input int k);
    int d[];
    d = new[m];
    begin : blk
      if (k == 0) disable blk;
      d[0] = 7;
    end
    g = d.size() * 10 + d[0];
  endfunction
  initial begin
    $display("A %0d", f(2));
    n = 8;
    $display("B %0d", f(3));
    $display("C %0d %0d", g(1), g(0));
    $finish;
  end
endmodule
"#;
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
    assert_eq!(
        r.backend,
        Backend::Native,
        "refused: {:?}",
        r.native.refused
    );
    assert_eq!(
        sink.events.into_inner(),
        vec![
            "out|A 508\n".to_string(),
            "out|B 821\n".to_string(),
            "out|C 37 30\n".to_string(),
        ],
        "frame-local new[] sized from a module net + disable (iverilog-pinned)"
    );
}
