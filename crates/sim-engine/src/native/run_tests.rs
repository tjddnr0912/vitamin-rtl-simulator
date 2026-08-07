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
    ]
}

#[test]
fn s1d4c2c_native_run_matches_the_vm_on_adversarial_shapes() {
    let designs = adversarial_designs();
    assert_eq!(designs.len(), 60, "adversarial set shrank");
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
        (
            "refused system task in a LATER block",
            "a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)",
            r#"
module top;
  reg [7:0] n;
  initial begin
    n = 8'd0;
    #1 n = 8'd1;
    if (n == 8'd1) $monitor("n=%0d", n);
    #1 $finish;
  end
endmodule
"#,
        ),
        (
            "wait fork",
            "a `wait fork`, or a subroutine CALL STATEMENT (task / output formals)",
            r#"
module top;
  reg [7:0] n = 8'd0;
  initial begin wait fork; n = 8'd7; $display("after n=%0d", n); end
  initial #2 $finish;
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
