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

// ───────────────────────── Phase 4 guard: fork in a task stays loud ──────────
#[test]
fn fork_task_still_loud() {
    // GUARD: a `fork` inside a task body is a Phase-4 follow-on — it must stay LOUD
    // (E3009), never silently mis-run on the suspendable-frame path.
    let src = "module t; logic c=0; always #5 c=~c;\n\
        task automatic par(); fork @(posedge c); @(posedge c); join endtask\n\
        initial begin par(); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(!ok, "fork task must stay loud, got:\n{out}");
    assert!(
        out.contains("fork"),
        "expected the fork reject, got:\n{out}"
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

// ───────────────── Phase 3: timing (@/#/wait) suspends the caller (vs iverilog) ─────
#[test]
fn v3_at_wait_suspends() {
    // Two `@(posedge clk)` waits in a task suspend the calling process; with a 5ns clock
    // the second edge lands at t=15. (iverilog: o=15.)
    let src = "module t; logic c=0; always #5 c=~c;\n\
        task automatic w2(); @(posedge c); @(posedge c); endtask\n\
        initial begin w2(); $display(\"o=%0t\", $time); #1 $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "@-wait task must run, got:\n{out}");
    assert!(out.contains("o=15"), "got:\n{out}");
}

#[test]
fn v3_delay_suspends() {
    // A `#10` delay in a task advances time to 10. (iverilog: o=10.)
    let src = "module t; task automatic dly(); #10; endtask\n\
        initial begin dly(); $display(\"o=%0t\", $time); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "#delay task must run, got:\n{out}");
    assert!(out.contains("o=10"), "got:\n{out}");
}

#[test]
fn v3_sequential_drivers() {
    // The KAT-driver shape: several `task drive(v); @(posedge clk); bus<=v;` calls in one
    // initial. Second drive wins ⇒ bus=22. (iverilog: o=22.)
    let src = "module t; logic c=0; logic [7:0] bus; always #5 c=~c;\n\
        task automatic drive(input logic [7:0] v); @(posedge c); bus <= v; endtask\n\
        initial begin drive(8'h11); @(posedge c); drive(8'h22); @(posedge c);\n\
          $display(\"o=%0h\", bus); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "sequential driver task must run, got:\n{out}");
    assert!(out.contains("o=22"), "got:\n{out}");
}

// ─── Phase 4: recursion + nested suspendable task calls (call-stack depth > 1) ───
#[test]
fn v4_recursion_with_timing() {
    // A task that recursively calls itself, waiting each level — the call-stack grows one
    // FrameRec per level and all windows stash/restore together across a suspend.
    // countdown(3): 3 waits on a 5ns clock ⇒ $time=25. (iverilog: o=25.)
    let src = "module t; logic c=0; always #5 c=~c;\n\
        task automatic countdown(input int n); if (n>0) begin @(posedge c); countdown(n-1); end endtask\n\
        initial begin countdown(3); $display(\"o=%0t\", $time); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "recursion-with-timing must run, got:\n{out}");
    assert!(out.contains("o=25"), "got:\n{out}");
}

#[test]
fn v4_nested_suspendable_call() {
    // A suspendable task `hi` that calls another suspendable task `lo` (which waits). The
    // nested frame is pushed onto the same call-stack; both windows stash across the wait.
    // hi(0x33) ⇒ bus=33. (iverilog: o=33.)
    let src = "module t; logic c=0; logic [7:0] bus; always #5 c=~c;\n\
        task automatic lo(input logic [7:0] v); @(posedge c); bus <= v; endtask\n\
        task automatic hi(input logic [7:0] v); lo(v); @(posedge c); endtask\n\
        initial begin hi(8'h33); $display(\"o=%0h\", bus); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "nested suspendable call must run, got:\n{out}");
    assert!(out.contains("o=33"), "got:\n{out}");
}
