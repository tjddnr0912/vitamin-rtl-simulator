//! Round-14 V3/V4 — suspendable tasks (`@`/`#`/wait/NBA/$systask in a task body).
//! Built phase-by-phase per docs/superpowers/plans/2026-07-20-suspendable-tasks-v3v4.md.
//! iverilog 13.0 goldens (Phase 0): p_at=15, p_delay=10, p_nba=a5, p_sys=7, p_seq=22,
//! p_recur=25, p_fork=5.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (combined stdout+stderr trimmed, success) for a one-shot vita run of `src`.
/// Combined so a `contains` check works for both value output (stdout, e.g. `o=15`)
/// and diagnostics (stderr, e.g. the E3009 "frame-call subset" message).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_susp_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (combined.trim().to_owned(), out.status.success())
}

// ───────────────────────── Phase 1: classifier (behaviour unchanged) ─────────
#[test]
fn nonsubset_task_still_loud_phase1() {
    // REGRESSION PIN: a task with `@(posedge)` must STILL be loud (E3009) until the
    // engine suspendable-frame path lands (Phase 2/3). The classifier refactor
    // (validate_frame_body → classify_frame_body) must not change this behaviour.
    let src = "module t; logic c=0; always #5 c=~c;\n\
        task automatic w(); @(posedge c); endtask\n\
        initial begin w(); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(
        !ok,
        "non-subset task must stay loud in Phase 1, got:\n{out}"
    );
    assert!(
        out.contains("frame-call subset"),
        "expected the frame-call-subset E3009, got:\n{out}"
    );
}

#[test]
fn subset_task_unchanged_phase1() {
    // A pure-subset task (blocking assign to its own local) must still run fine.
    let src = "module t(output int o);\n\
        task automatic s(input int n, output int r); int x; x = n + 1; r = x; endtask\n\
        initial begin int r; s(6, r); o = r; $display(\"o=%0d\", o); end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "subset task must still run, got:\n{out}");
    assert!(out.contains("o=7"), "subset task result, got:\n{out}");
}
