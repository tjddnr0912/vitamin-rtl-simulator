//! FAITHFUL deferred immediate assertions (IEEE 1800-2017 §16.4): `assert #0`
//! (Observed deferred) and `assert final` (Reactive deferred). A deferred
//! assertion is EVALUATED when reached (the condition is sampled in the Active
//! region, like an immediate assertion), but its pass/fail action is not run
//! inline — it is enqueued and MATURED at a later scheduling region: the
//! Observed region for `#0`, the Reactive region for `final` (IEEE 1800 §4.4
//! stratified order: Active → NBA → Observed → Reactive → Postponed).
//!
//! The defining difference from an immediate assertion is FLUSH-ON-RE-REACH:
//! if the same static assertion (same process instance) is reached again within
//! one time slot — e.g. a combinational `always` re-triggered by a delta — the
//! prior pending report is CANCELLED and replaced by the latest evaluation. Only
//! the final settled verdict of the time slot matures. This filters the
//! transient glitches that fire a naive immediate assert on every delta.
//!
//! There is NO oracle: iverilog 13.0 rejects deferred assertions outright
//! ("Deferred assertions are not supported"), so every expected output below is
//! hand-IEEE with the §16.4 / §4.4 rationale pinned in the test. Where a
//! contrast is instructive, a sibling test runs the IMMEDIATE form against the
//! same body to show the divergent (glitchy) behavior deferral suppresses.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_adf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

// ── glitch filtering: the headline faithful difference ───────────────────────
// `b` follows `a` through an NBA, so during the Active region where `a` has
// just become 1, `b` is still 0 (the NBA has not applied) → a transient a!=b.
// A combinational `always @(a or b)` re-runs once with the transient (a=1,b=0)
// and again after the NBA (a=1,b=1). A DEFERRED `#0` assert captures the
// transient FAIL, then the second reach FLUSHES it with a PASS → no report.
const GLITCH_BODY: &str = "\
   reg a = 0, b = 0;\n\
   always @(a)      b <= a;\n\
   always @(a or b) {ASSERT}\n\
   initial begin #5 a = 1; #5 $finish; end\n";

#[test]
fn deferred_observed_filters_reeval_glitch() {
    let src = format!(
        "module top;\n{}endmodule\n",
        GLITCH_BODY.replace("{ASSERT}", "assert #0 (a == b) else $error(\"GLITCH\");")
    );
    let (out, err, code) = run(&src);
    assert_eq!(
        code,
        Some(0),
        "deferred #0 filters the glitch. err:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        count(&err, "GLITCH"),
        0,
        "the transient a!=b must NOT fire a deferred assert (flushed by the settled PASS). err:\n{err}"
    );
}

#[test]
fn immediate_assert_fires_on_reeval_glitch() {
    // CONTRAST: the SAME body with an immediate assert DOES fire on the
    // transient — proving the deferred form above is genuinely filtering, not
    // just never reaching a fail.
    let src = format!(
        "module top;\n{}endmodule\n",
        GLITCH_BODY.replace("{ASSERT}", "assert (a == b) else $error(\"GLITCH\");")
    );
    let (out, err, _code) = run(&src);
    assert!(
        count(&err, "GLITCH") >= 1,
        "an IMMEDIATE assert fires on the transient a!=b. err:\n{err}\nout:\n{out}"
    );
}

#[test]
fn deferred_final_filters_reeval_glitch() {
    // `final` (Reactive) filters the same glitch as `#0` (Observed) — both defer
    // the verdict to a settled region; they differ only in maturation ORDER.
    let src = format!(
        "module top;\n{}endmodule\n",
        GLITCH_BODY.replace("{ASSERT}", "assert final (a == b) else $error(\"GLITCH\");")
    );
    let (out, err, code) = run(&src);
    assert_eq!(
        code,
        Some(0),
        "deferred final filters the glitch. err:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        count(&err, "GLITCH"),
        0,
        "final defers + flushes the transient. err:\n{err}"
    );
}

// ── flush-on-re-reach counts once ────────────────────────────────────────────
#[test]
fn deferred_flush_counts_once_in_loop() {
    // The SAME static deferred assert reached 3× in one time slot (a loop with no
    // intervening time) keeps only the LAST verdict (§16.4 maturation): exactly
    // ONE report, not three. An immediate assert would fire three times.
    let (out, err, code) = run("module top;\n\
         integer i;\n\
         initial begin\n\
           for (i = 0; i < 3; i = i + 1) assert #0 (1'b0) else $error(\"BOOM\");\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a failing deferred assert sets exit 1. err:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        count(&err, "BOOM"),
        1,
        "flush-on-re-reach: 3 reaches in one slot mature ONCE. err:\n{err}"
    );
}

// ── region ordering: Observed → Reactive → Postponed ─────────────────────────
#[test]
fn region_order_observed_before_reactive_before_strobe() {
    // Textual order is strobe, final, #0 — but the SETTLED-region order is
    // Observed(#0) → Reactive(final) → Postponed($strobe). A passing deferred
    // assert's PASS action also defers, so all three print, in region order.
    let (out, err, _code) = run("module top;\n\
         initial begin\n\
           $strobe(\"STROBE\");\n\
           assert final (1'b1) $display(\"REACT\");\n\
           assert #0    (1'b1) $display(\"OBS\");\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    let obs = out.find("OBS");
    let react = out.find("REACT");
    let strobe = out.find("STROBE");
    assert!(
        obs.is_some() && react.is_some() && strobe.is_some(),
        "all three must print. out:\n{out}\nerr:\n{err}"
    );
    assert!(
        obs < react && react < strobe,
        "IEEE §4.4 order Observed<Reactive<Postponed. out:\n{out}"
    );
}

// ── per-instance maturation: distinct activities hold distinct reports ────────
#[test]
fn deferred_per_instance_via_fork() {
    // The SAME static assert (one StmtId, inside a task) reached by two
    // concurrent fork children is two DISTINCT assertion instances per §16.4 —
    // keyed by (assert_id, activity_id) — so it produces TWO reports, not one
    // (which a StmtId-only key would wrongly collapse).
    let (out, err, _code) = run("module top;\n\
         task chk; assert #0 (1'b0) else $error(\"INST\"); endtask\n\
         initial fork\n\
           chk();\n\
           chk();\n\
         join\n\
         initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(
        count(&err, "INST"),
        2,
        "two fork instances of the same assert mature independently. err:\n{err}\nout:\n{out}"
    );
}

// ── disable-fork cancels a pending deferred report ───────────────────────────
#[test]
fn disable_fork_cancels_pending_deferred() {
    // The child captures a pending FAIL at t=0 (Active) then SUSPENDS on `#10`,
    // staying ALIVE. The parent yields with `#0` (Inactive, after the child's
    // Active reach), then `disable fork` kills the still-alive child BEFORE the
    // Observed region — §16.4: a pending report in a DISABLED process is
    // cancelled. So NO report matures. (A normally-COMPLETED process's pending
    // report would instead mature — only an explicit disable flushes it.)
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               assert #0 (1'b0) else $error(\"KILLED\");\n\
               #10;\n\
             end\n\
           join_none\n\
           #0;\n\
           disable fork;\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "no report → clean exit. err:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        count(&err, "KILLED"),
        0,
        "disable fork cancels the pending deferred report of the still-alive child. err:\n{err}"
    );
}

#[test]
fn completed_process_deferred_report_still_matures() {
    // CONTRAST to disable_fork_cancels_pending_deferred: a child that COMPLETES
    // normally (no disable) keeps its pending report — it matures. This pins the
    // §16.4 boundary (disable cancels; normal completion does not).
    let (out, err, _code) = run("module top;\n\
         initial begin\n\
           fork\n\
             assert #0 (1'b0) else $error(\"MATURES\");\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        count(&err, "MATURES"),
        1,
        "a completed process's pending deferred report still matures. err:\n{err}\nout:\n{out}"
    );
}

// ── activity-id recycle must NOT erase a pending report (review D1, P0) ───────
#[test]
fn deferred_recycle_does_not_erase_failure() {
    // child0 (c=0) fails → its report pends; child0 completes and its activity
    // slot is freed. child1 (c=1) RECYCLES that slot and reaches the SAME marker
    // with a passing (empty) arm. Keyed by (marker, aid, GENERATION), child1's
    // flush must NOT erase child0's pending failure — else a real failure
    // silently vanishes. Expect exit 1 + the failure reported.
    let (out, err, code) = run("module top;\n\
         reg c;\n\
         task chk; assert #0 (c) else $error(\"ERASED\"); endtask\n\
         initial begin\n\
           c = 1'b0; fork chk(); join\n\
           c = 1'b1; fork chk(); join\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "child0's real failure must survive the recycled child1's flush. err:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        count(&err, "ERASED"),
        1,
        "exactly child0's failure matures. err:\n{err}"
    );
}

#[test]
fn deferred_recycle_keeps_all_reports() {
    // Three SEQUENTIAL fork children (each reusing the freed slot of the prior)
    // all fail at t=0 → three distinct instances → three reports (a StmtId+aid
    // key without the generation would collapse them to one).
    let (out, err, _code) = run("module top;\n\
         task chk; assert #0 (1'b0) else $error(\"R\"); endtask\n\
         initial begin\n\
           fork chk(); join\n\
           fork chk(); join\n\
           fork chk(); join\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        count(&err, "R"),
        3,
        "three recycled instances mature independently. err:\n{err}\nout:\n{out}"
    );
}

// ── deferred action args sampled at REACH, not maturation (review D2, §16.4.3) ─
#[test]
fn deferred_action_args_sampled_at_reach() {
    // `code` is 42 when the assert is reached; it becomes 200 before the Observed
    // region. IEEE §16.4.3: a deferred action's input arguments are sampled WHEN
    // THE ASSERTION IS EVALUATED — so the report must read 42, not 200.
    let (out, err, code) = run("module top;\n\
         reg [31:0] code;\n\
         initial begin\n\
           code = 32'd42;\n\
           assert #0 (1'b0) else $error(\"code=%0d\", code);\n\
           code = 32'd200;\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the deferred fail fires. err:\n{err}\nout:\n{out}"
    );
    assert!(
        count(&err, "code=42") == 1 && count(&err, "code=200") == 0,
        "args are sampled at reach (42), not re-evaluated at maturation (200). err:\n{err}"
    );
}

// ── deferred $fatal: exit class + termination drain ──────────────────────────
#[test]
fn deferred_fatal_exits_one_and_drains_strobe() {
    // A deferred `final` $fatal matures in the Reactive region → exit 1 (Fatal).
    // A $strobe queued the same slot still prints (the termination path drains
    // the current slot's Observed/Reactive then Postponed before exit).
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           $strobe(\"STROBE\");\n\
           assert final (1'b0) else $fatal(1, \"DOOM\");\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "deferred $fatal → exit 1. err:\n{err}\nout:\n{out}"
    );
    assert!(
        count(&err, "DOOM") >= 1,
        "the deferred $fatal fires. err:\n{err}"
    );
    assert!(
        count(&out, "STROBE") >= 1,
        "the same-slot $strobe still drains. out:\n{out}\nerr:\n{err}"
    );
}

// ── parser: only `#0` is the Observed deferred form ──────────────────────────
#[test]
fn assert_hash_nonzero_is_loud() {
    // `#0` is the Observed deferred form; a non-zero `#<n>` delay on an assert is
    // NOT a deferred assertion and stays a loud parse error.
    let (_o, err, code) = run("module top;\n\
         reg x = 0;\n\
         initial begin assert #1 (x); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "`assert #1` is loud. {err}");
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn assert_hash0_is_accepted() {
    // `assert #0` now PARSES (was a loud error before the deferred-assert slice).
    let (out, err, code) = run("module top;\n\
         reg x = 1;\n\
         initial begin assert #0 (x); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "`assert #0 (true)` parses + holds. err:\n{err}\nout:\n{out}"
    );
    assert!(!err.contains("VITA-E"), "must not be loud:\n{err}");
}

#[test]
fn deferred_runs_deterministically() {
    let src = "module top;\n\
         reg a = 0, b = 0;\n\
         always @(a)      b <= a;\n\
         always @(a or b) assert #0 (a == b) else $error(\"X\");\n\
         initial begin #5 a = 1; #5 $finish; end\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
