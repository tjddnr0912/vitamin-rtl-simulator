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
    assert_eq!(designs.len(), 90, "adversarial set shrank");
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
    let mut sites: Vec<(&str, usize)> = Vec::new();
    for (name, src) in files {
        for (i, line) in src.lines().enumerate() {
            // Skip doc/comment lines: the funnel's own doc names the call it
            // replaced, and a comment is not a call.
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains("arena.write_lvalue(") {
                sites.push((name, i + 1));
            }
        }
    }
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
            3,
            "`split_file_directed`'s fd (the `file_directed` row, and \
             `$monitor`/`$strobe` are in `systask_refusal` too), `$dumplimit`'s \
             size and `$fclose`'s fd (both in `systask_refusal`)",
        ),
        (
            "crv_draw.rs",
            4,
            "the class/CRV surface is the `class` row, and `writemem`'s window \
             bounds are `$writemem*`, which `systask_refusal` refuses (it reads \
             the MEMORY itself, not a formatted argument). A1-iii took this from \
             6 to 4 by threading `readmem`'s two window bounds — measured, not \
             assumed: `$readmemh(f, m, lo, hi)` with NET bounds loaded the whole \
             array on a native run",
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
///   * a §16.4 DEFERRED assertion matures in the Observed/Reactive regions via
///     `Scheduler::mature_deferred`, and tier-3's region cascade calls it
///     nowhere — that is the separate `deferred_assert` row (14 of the 2,834
///     measured fall-backs, against this slice's 760).
#[test]
fn sva_shapes_that_need_machinery_still_refuse_by_their_own_name() {
    let cases: Vec<(&str, &str, &str)> = vec![
        (
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
        ),
        (
            "deferred immediate assertion",
            "deferred_assert",
            r#"
module top;
  reg clk = 0, a = 0;
  always #5 clk = ~clk;
  always @(posedge clk) assert #0 (a) else $error("deferred");
  initial begin #10 a = 1; #10 $finish; end
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
    }
    assert_eq!(cases.len(), 2);
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
