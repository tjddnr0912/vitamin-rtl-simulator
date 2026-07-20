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

// ─── Phase 2: leaf non-suspending tasks routed through run_process (vs iverilog) ───
#[test]
fn v4_display_task_runs() {
    // V4B: a $systask ($display) in an automatic task — leaf, non-suspending — now runs
    // via the suspendable-frame path. say(7) prints o=7. (iverilog: o=7.)
    let src = "module t; task automatic say(input int n); $display(\"o=%0d\", n); endtask\n\
        initial begin say(7); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "$display task must run, got:\n{out}");
    assert!(out.contains("o=7"), "got:\n{out}");
}

#[test]
fn v4_nba_task_runs() {
    // A non-blocking assign to a module net from a task (no timing) — routed; the NBA
    // settles after #1. put(8'hAB) then d==ab. (iverilog: o=ab.)
    let src = "module t; logic [7:0] d;\n\
        task automatic put(input logic [7:0] v); d <= v; endtask\n\
        initial begin put(8'hAB); #1 $display(\"o=%0h\", d); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "NBA task must run, got:\n{out}");
    assert!(out.contains("o=ab"), "got:\n{out}");
}

#[test]
fn v4_output_arg_copied_back() {
    // An OUTPUT formal written in the routed task must be copied back to the caller — the
    // frame-local write lane (write_lvalue) routes it to the frame slot, not the flat
    // store. dbl(9) ⇒ o=18. (Regression guard for the o=0 silent-wrong caught in review.)
    let src = "module t;\n\
        task automatic dbl(input int a, output int r); r = a * 2; $display(\"in=%0d\", a); endtask\n\
        initial begin int y; dbl(9, y); $display(\"o=%0d\", y); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "output-arg task must run, got:\n{out}");
    assert!(
        out.contains("o=18"),
        "output arg must be copied back, got:\n{out}"
    );
}

#[test]
fn v4_body_local_write_read() {
    // A body-local variable written then read inside the routed task (frame-slot RW).
    // acc(3,4): t1=7, r=14 ⇒ o=14. (iverilog: o=14.)
    let src = "module t;\n\
        task automatic acc(input int a, input int b, output int r); int t1; t1 = a + b; r = t1 * 2; endtask\n\
        initial begin int y; acc(3, 4, y); $display(\"o=%0d\", y); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "body-local task must run, got:\n{out}");
    assert!(out.contains("o=14"), "got:\n{out}");
}

#[test]
fn v3_timing_task_still_loud_phase2() {
    // GUARD: a task that SUSPENDS (`@(posedge)`) is not yet supported (per-activity-window
    // phase) — it must stay LOUD (E3009), never silently mis-run on the shared frame_stack.
    let src = "module t; logic c=0; always #5 c=~c;\n\
        task automatic w(); @(posedge c); endtask\n\
        initial begin w(); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(!ok, "timing task must stay loud in Phase 2, got:\n{out}");
    assert!(out.contains("frame-call subset"), "got:\n{out}");
}
