//! `wait fork;` (IEEE §9.6.1) — block the current process until all of its
//! outstanding forked children complete. iverilog 13.0 live pins (probes
//! captured 2026-06-14). The implicit barrier is over the CUMULATIVE set of
//! this process's immediate children (across every `fork ... join_none` /
//! surplus `join_any` child), which is why it cannot reuse a single fork
//! barrier.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wf_{}_{n}", std::process::id()));
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

#[test]
fn wait_fork_blocks_until_children_complete() {
    // iverilog-pinned: join_none spawns two children (t=1, t=5); wait fork
    // blocks until BOTH finish → afterwait at t=5.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork\n\
             #5 $display(\"c5 t=%0d\", $time);\n\
             #1 $display(\"c1 t=%0d\", $time);\n\
           join_none\n\
           wait fork;\n\
           $display(\"afterwait t=%0d\", $time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("c1 t=1"), "got:\n{out}");
    assert!(out.contains("c5 t=5"), "got:\n{out}");
    assert!(out.contains("afterwait t=5"), "got:\n{out}");
    // ordering: afterwait must come AFTER c5 (it waited for the slow child).
    let p_c5 = out.find("c5 t=5").unwrap();
    let p_aw = out.find("afterwait t=5").unwrap();
    assert!(p_c5 < p_aw, "afterwait must follow c5:\n{out}");
}

#[test]
fn wait_fork_zero_children_falls_through() {
    // iverilog-pinned: no preceding fork → wait fork unblocks immediately at t=0.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           wait fork;\n\
           $display(\"immediate t=%0d\", $time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("immediate t=0"), "got:\n{out}");
}

#[test]
fn wait_fork_waits_cumulative_children() {
    // iverilog-pinned: TWO separate join_none forks before one wait fork — the
    // barrier covers the cumulative child set, so done waits for the LATER (t=4)
    // child, not just the most recent fork.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork #2 $display(\"a t=%0d\",$time); join_none\n\
           fork #4 $display(\"b t=%0d\",$time); join_none\n\
           wait fork;\n\
           $display(\"done t=%0d\",$time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a t=2"), "got:\n{out}");
    assert!(out.contains("b t=4"), "got:\n{out}");
    assert!(out.contains("done t=4"), "got:\n{out}");
    let p_b = out.find("b t=4").unwrap();
    let p_done = out.find("done t=4").unwrap();
    assert!(p_b < p_done, "done must follow b:\n{out}");
}

#[test]
fn wait_fork_after_disable_fork_no_hang() {
    // iverilog-pinned (deadlock guard): disable fork kills the t=5 victim at
    // t=2; the following wait fork has no live children → falls through at t=2.
    // MUST NOT hang.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork\n\
             #5 $display(\"victim t=%0d\",$time);\n\
             #1 $display(\"quick t=%0d\",$time);\n\
           join_none\n\
           #2 disable fork;\n\
           wait fork;\n\
           $display(\"nohang t=%0d\",$time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("quick t=1"), "got:\n{out}");
    assert!(!out.contains("victim"), "victim must be killed:\n{out}");
    assert!(out.contains("nohang t=2"), "got:\n{out}");
}

#[test]
fn wait_fork_waits_for_join_any_surplus() {
    // iverilog-pinned: join_any resumes at the first child (t=1) leaving the
    // t=3 surplus child running; wait fork then blocks for that surplus →
    // alldone at t=3.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork\n\
             #1 $display(\"f1 t=%0d\",$time);\n\
             #3 $display(\"f3 t=%0d\",$time);\n\
           join_any\n\
           $display(\"any t=%0d\",$time);\n\
           wait fork;\n\
           $display(\"alldone t=%0d\",$time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("f1 t=1"), "got:\n{out}");
    assert!(out.contains("any t=1"), "got:\n{out}");
    assert!(out.contains("f3 t=3"), "got:\n{out}");
    assert!(out.contains("alldone t=3"), "got:\n{out}");
    let p_f3 = out.find("f3 t=3").unwrap();
    let p_all = out.find("alldone t=3").unwrap();
    assert!(p_f3 < p_all, "alldone must follow f3:\n{out}");
}

// ── edge-sensitive always must not re-fire while suspended mid-body ───────────
// Adversarial-review regression (2026-06-14): a permanent `net_to_edge`
// sensitivity entry was re-triggering an `always @(posedge clk)` block from its
// entry while it was still parked inside its body (on `#delay`/`wait`/`@`/`wait
// fork`), spuriously re-running it on every subsequent edge — a general engine
// bug (silent-wrong + non-termination), newly exposed by wait fork. The `busy`
// guard suppresses the static re-fire until the body completes and re-arms.
//
// Both cases place the in-body resume at t=97 (NOT on a posedge — clk period 10,
// posedges at t=5,15,…) so the differential is CLEAN. A wait-fork resume that
// coincides with a clock posedge (e.g. child `#100` from t=5 → t=105) is a
// same-tick tie between the resume and the edge wake — the documented
// tool-divergent area (CLAUDE.md: 동시-틱 tie는 도구-발산 영역); not asserted here.
//
// Same class, separately pinned (review finding 1, hand-IEEE): a `#N $finish`
// racing a fork child's own `#N` wake at the SAME tick is order-divergent by
// design — vita runs the Active-region $finish first (ending the run and
// dropping the child's print), vvp applies the child wake first. IEEE leaves
// same-tick ordering among independent processes unspecified; not asserted.

#[test]
fn clocked_always_mid_body_delay_no_respawn() {
    // iverilog-pinned: an in-body `#92` from the first posedge (t=5) finishes at
    // t=97 — the block must NOT re-fire while parked (no `tick` before t=97).
    let (out, err, code) = run("module top;\n\
         reg clk=0; integer n=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) begin\n\
           n=n+1;\n\
           if(n==1) #92 $display(\"delaydone n=%0d t=%0d\",n,$time);\n\
           else $display(\"tick n=%0d t=%0d\",n,$time);\n\
         end\n\
         initial #120 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("delaydone n=1 t=97"), "got:\n{out}");
    assert!(out.contains("tick n=2 t=105"), "got:\n{out}");
    assert!(out.contains("tick n=3 t=115"), "got:\n{out}");
    // the bug printed tick at t=15,25,… while parked — must be absent.
    assert!(
        !out.contains("t=15"),
        "spurious re-fire while parked:\n{out}"
    );
    assert!(
        !out.contains("t=25"),
        "spurious re-fire while parked:\n{out}"
    );
}

#[test]
fn wait_fork_in_clocked_always_no_respawn() {
    // iverilog-pinned: parked on `wait fork` from t=5 until the child (#92)
    // completes at t=97 — no spurious tick while parked, then normal ticks.
    let (out, err, code) = run("module top;\n\
         reg clk=0; integer n=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) begin\n\
           n=n+1;\n\
           if(n==1) begin\n\
             fork #92 $display(\"child t=%0d\",$time); join_none\n\
             wait fork;\n\
             $display(\"afterwait n=%0d t=%0d\",n,$time);\n\
           end else $display(\"tick n=%0d t=%0d\",n,$time);\n\
         end\n\
         initial #120 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("child t=97"), "got:\n{out}");
    assert!(out.contains("afterwait n=1 t=97"), "got:\n{out}");
    assert!(out.contains("tick n=2 t=105"), "got:\n{out}");
    assert!(
        !out.contains("t=15"),
        "spurious re-fire while parked:\n{out}"
    );
    assert!(
        !out.contains("t=25"),
        "spurious re-fire while parked:\n{out}"
    );
}
