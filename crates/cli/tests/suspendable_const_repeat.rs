//! r18 (C1, part 1): a CONSTANT-count `repeat(N) @(edge)` inside a suspendable task is now
//! supported. It was loud-rejected by an over-conservative `ast_has_repeat_with_timing`
//! that flagged ANY `repeat` with timing — but `lower_repeat` straight-UNROLLS a const-small
//! count (`repeat(2) @(posedge clk)` → `@(posedge clk); @(posedge clk)`), which carries no
//! runtime counter and is safe across suspends. Only a NON-const / large count desugars to
//! the shared `$repeat_cnt$` net (the real cross-activation hazard) and stays loud.
//!
//! (The `fork...join` half of the report's C1 repro is a separate, deeper scheduler item —
//! spawning concurrent children inside a frame needs a shared frame-window model — and stays
//! loud, correct-or-loud.)
//!
//! ORACLE: iverilog 13.0 runs const-repeat suspendable tasks, so these are iverilog-verified.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_scr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

// ── a const `repeat(2) @(posedge clk)` in a suspendable task, called directly ──
#[test]
fn const_repeat_direct() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        int done = 0;\n\
        task automatic wait2; repeat (2) @(posedge clk); done = 1; endtask\n\
        initial begin wait2(); $display(\"done=%0d @%0t\", done, $time); $finish; end\n\
        endmodule\n");
    // First posedge at t=5, second at t=15 → done=1 @15.
    assert!(
        !is_loud(&o) && o.contains("done=1 @15"),
        "const repeat direct:\n{o}"
    );
}

// ── a const `repeat` suspendable task called NESTED from another suspendable task ──
#[test]
fn const_repeat_nested_call() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); a(); $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // run's @posedge at t=5, a's two @posedge at t=15,25 → PASS @25.
    assert!(
        !is_loud(&o) && o.contains("PASS @25"),
        "const repeat nested:\n{o}"
    );
}

// ── `repeat(N)` with N a literal larger than 1 in a loop-ish body ──
#[test]
fn const_repeat_three() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        int cnt = 0;\n\
        task automatic tick3; repeat (3) begin @(posedge clk); cnt++; end endtask\n\
        initial begin tick3(); $display(\"cnt=%0d @%0t\", cnt, $time); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("cnt=3 @25"),
        "const repeat three:\n{o}"
    );
}

// ── correct-or-loud: a NON-const (runtime) `repeat(n) @(edge)` in a suspendable
//    task stays loud (the shared `$repeat_cnt$` net corrupts across activations) ──
#[test]
fn runtime_repeat_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic waitn (input int n); repeat (n) @(posedge clk); endtask\n\
        initial begin waitn(2); $display(\"done\"); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "runtime repeat count must stay loud:\n{o}");
}

// ── Stage-1 fork-in-frame: `fork <self-contained arms> join` inside a suspendable
//    task now RUNS (see crates/cli/tests/fork_in_frame.rs for the full coverage) ──
#[test]
fn fork_of_tasks_join_runs() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic b; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); b(); join $display(\"PASS\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "fork of tasks in a frame task now runs:\n{o}"
    );
}
