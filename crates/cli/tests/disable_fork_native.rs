//! A4-d ABSOLUTE ANCHOR — `disable fork` (IEEE §9.6.3) on TIER-3, and the
//! dispatch-choke fix it dragged out with it.
//!
//! This is the last family in Phase A. With it wired, `native::design_eligibility`
//! has no row a design can reach and `native::kernel`'s `gate_refused!` macro has
//! no call sites — both are gone rather than left as decoration.
//!
//! The kill itself needed no machinery: `Scheduler::k_disable_fork` walks
//! `activities` and `barriers` transitively and cancels §16.4 reports in
//! `st.postponed`, none of which reads a net value, so the tier-3 method is one
//! delegated line. What was genuinely missing sat at the dispatch choke —
//! `Scheduler::set_cur_activity`, which the engine calls in `run_body` and
//! tier-3 never called at all.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(backend: &str, src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dfn_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    let merged = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let body: String = merged
        .lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.contains("VITA-W1017"))
        .filter(|l| !l.starts_with("errors="))
        .fold(String::new(), |mut a, l| {
            a.push_str(l);
            a.push('\n');
            a
        });
    (body, rj, out.status.code().unwrap_or(-1))
}

fn run_native(src: &str) -> (String, String, i32) {
    run_on("native", src)
}

/// `disable fork` terminates every active descendant; the caller lives on.
///
/// Both halves are the discriminator, and the second is the one a partial
/// implementation passes: `killed a=0 b=0` shows the children had not yet run,
/// which is true even with the kill missing, while `end a=0 b=0 c=3 t=7` shows
/// they never ran AT ALL. Measured with the dispatch-choke fix removed, this
/// design printed `k1 t=2`, `k2 t=4` and `end a=1 b=2` — the kill set was rooted
/// at activity 0 instead of the caller, so it named nobody. iverilog-pinned.
#[test]
fn disable_fork_kills_the_children_and_the_caller_continues() {
    let (body, rj, code) = run_native(
        "module top;\n\
           integer a = 0, b = 0, c = 0;\n\
           initial begin\n\
             fork\n\
               begin #2; a = 1; $display(\"k1 t=%0t\", $time); end\n\
               begin #4; b = 2; $display(\"k2 t=%0t\", $time); end\n\
             join_none\n\
             #1;\n\
             disable fork;\n\
             $display(\"killed a=%0d b=%0d t=%0t\", a, b, $time);\n\
             #6 c = 3;\n\
             $display(\"end a=%0d b=%0d c=%0d t=%0t\", a, b, c, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "killed a=0 b=0 t=1\n\
         end a=0 b=0 c=3 t=7\n"
    );
}

/// The kill is scoped to the caller's OWN descendants — a sibling process is
/// untouched.
///
/// `k_disable_fork` grows its kill set by following barriers whose parent is
/// already in it, and an over-broad walk (or one rooted at the wrong activity)
/// would take the other `initial` with it. `other z=9` and `other end` are what
/// says it did not. iverilog-pinned.
#[test]
fn disable_fork_leaves_a_sibling_process_alone() {
    let (body, rj, code) = run_native(
        "module top;\n\
           integer a = 0, z = 0;\n\
           initial begin\n\
             fork\n\
               begin #3; a = 1; $display(\"child t=%0t\", $time); end\n\
             join_none\n\
             #1 disable fork;\n\
             $display(\"after t=%0t a=%0d\", $time, a);\n\
           end\n\
           initial begin\n\
             #2 z = 9; $display(\"other z=%0d t=%0t\", z, $time);\n\
             #5 $display(\"other end a=%0d t=%0t\", a, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "after t=1 a=0\n\
         other z=9 t=2\n\
         other end a=0 t=7\n"
    );
}

/// The OTHER half of the same key, and it had no coverage anywhere in the
/// repository — engine included.
///
/// `set_cur_activity` writes `cur_aid` and `cur_gen` together, and the pair is
/// one fact: a completed fork child's activity slot goes back on
/// `free_activities`, so the NEXT fork can be handed the same `aid` with `gen`
/// bumped. Deferred reports are keyed by `(marker_sid, aid, gen)` precisely so
/// that the fresh incarnation reaching the same marker does not flush its
/// predecessor's still-pending action.
///
/// The design below is the smallest shape that recycles: two `fork … join`s in
/// ONE time slot, both arms reaching the same `assert #0`. Measured with the
/// `cur_gen` half deleted, on BOTH backends (this is shared scheduler code):
/// `errors=1`, and only `who=2` printed. The first arm's `$error` vanished
/// because the second incarnation's key matched it.
///
/// It is here rather than in the engine's own tests because A4-d is what
/// extracted `set_cur_activity` into one spelling, and this is the assertion
/// that stops the two halves from being split again.
#[test]
fn a_recycled_activity_slot_does_not_flush_its_predecessors_report() {
    const SRC: &str = "module top;\n\
           integer v = 0;\n\
           task automatic chk(input integer who);\n\
             begin\n\
               assert #0 (0) else $error(\"def who=%0d t=%0t\", who, $time);\n\
             end\n\
           endtask\n\
           initial begin\n\
             fork chk(1); join\n\
             fork chk(2); join\n\
             #1 $finish;\n\
           end\n\
         endmodule\n";
    // `[at time N]` is the runtime-diagnostic timestamp (round-29 R29-4). Here it
    // is not decoration: both reports must mature at t=0, which is what makes
    // "the older one was not flushed by the recycled slot" the only reading.
    const WANT: &str = "error[VITA-E4003] E-RUN-USER-ERROR: def who=1 t=0 [at time 0]\n\
                        error[VITA-E4003] E-RUN-USER-ERROR: def who=2 t=0 [at time 0]\n";

    let (native, rj, _) = run_native(SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT, "a recycled slot flushed the older report");
    for b in ["vm", "interp"] {
        let (other, _, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}

/// ⚠️⚠️ A PRE-EXISTING SILENT-WRONG THIS SLICE UNCOVERED AND FIXED, pinned here
/// because nothing else in the suite could see it.
///
/// §16.4 deferred reports are keyed by `(marker_sid, cur_aid, cur_gen)` so that
/// re-reaching the marker flushes only THIS activation's pending action. Tier-3
/// never set the pair — `Scheduler::set_cur_activity` is called from the
/// engine's `run_body`, and tier-3's `dispatch_body` had no equivalent — so
/// every deferred report on this backend was filed under activity 0.
///
/// The design below is the smallest shape that notices: two processes reach the
/// SAME `assert #0` statement (one task, two callers), so the two reports share
/// a `marker_sid` and are separated only by the activity. Measured with the fix
/// removed: `errors=1` and only `who=2` printed. One `$error` silently gone at
/// exit 0, which is the class this repository exists to refuse.
///
/// A4-d needed the same two lines for its own reason (`cur_aid` is the root of
/// `k_disable_fork`'s kill set), which is why the defect surfaced now rather
/// than when A8-b wired deferred assertions.
#[test]
fn deferred_reports_are_keyed_by_the_running_activity() {
    const SRC: &str = "module top;\n\
           integer p = 0, q = 0;\n\
           task automatic chk(input integer v, input integer who);\n\
             begin\n\
               assert #0 (v != 0) else $error(\"bad who=%0d v=%0d\", who, v);\n\
             end\n\
           endtask\n\
           initial begin #1; chk(p, 1); end\n\
           initial begin #1; chk(q, 2); end\n\
           initial #5 $finish;\n\
         endmodule\n";
    // `[at time N]` = the runtime-diagnostic timestamp (round-29 R29-4). Both
    // reports mature in the SAME slot (t=1), which is the whole point: two
    // processes reaching the same `assert #0` must file two reports there.
    const WANT: &str = "error[VITA-E4003] E-RUN-USER-ERROR: bad who=1 v=0 [at time 1]\n\
                        error[VITA-E4003] E-RUN-USER-ERROR: bad who=2 v=0 [at time 1]\n";

    let (native, rj, _) = run_native(SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT, "a deferred report went missing");
    // …and the other two backends agree, which is what makes the expected text a
    // property of the language rather than of this executor. (iverilog rejects
    // deferred assertions outright, so it is not the oracle for this one.)
    for b in ["vm", "interp"] {
        let (other, _, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}
