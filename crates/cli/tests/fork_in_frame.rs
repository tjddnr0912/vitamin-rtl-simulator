//! Stage 1 (Case A): `fork <self-contained arms> join[_any|_none]` inside a
//! suspendable task now runs. The arms are separate task calls / blocks that do
//! NOT reference the enclosing task's automatic locals, so the existing owned-
//! window model isolates them (the single-threaded scheduler + stash/restore).
//! ORACLE: iverilog 13.0 runs fork…join inside a task.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fif_{}_{n}", std::process::id()));
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

// ── the report's C1 repro: fork of two separate suspendable tasks, join ──
#[test]
fn report_repro_fork_of_tasks_join() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic b; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); b(); join $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // run's @posedge at t=5; a,b each wait 2 posedges (t=15,25) → both done t=25.
    assert!(!is_loud(&o) && o.contains("PASS @25"), "report repro:\n{o}");
}

// ── a single-arm fork (regression for the Fork-children rebase fix) ──
#[test]
fn fork_single_arm_join() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); join $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("PASS @25"), "single arm:\n{o}");
}

// ── join waits for the SLOWEST arm (a=3 posedges → t35, b=2 → t25) ──
#[test]
fn fork_join_waits_for_max() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (3) @(posedge clk); endtask\n\
        task automatic b; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); b(); join $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // ORACLE iverilog: PASS @35 (join = last arm).
    assert!(!is_loud(&o) && o.contains("PASS @35"), "join max:\n{o}");
}

// ── join_any fires at the FASTEST arm (b=2 → t25; a=3 surplus drains) ──
#[test]
fn fork_join_any_fires_at_min() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (3) @(posedge clk); endtask\n\
        task automatic b; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); b(); join_any $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // ORACLE iverilog: PASS @25 (join_any = first arm).
    assert!(!is_loud(&o) && o.contains("PASS @25"), "join_any min:\n{o}");
}

// ── inline-block arms (no task call) that only touch module nets run ──
#[test]
fn fork_inline_block_arms() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          @(posedge clk);\n\
          fork begin @(posedge clk); @(posedge clk); end join\n\
          $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // ORACLE iverilog: PASS @25.
    assert!(!is_loud(&o) && o.contains("PASS @25"), "inline block:\n{o}");
}

// ── join_none: parent continues immediately; children run in background.
//    ORACLE: iverilog 13.0 crashes on this exact source — `Assertion failed:
//    (child->wt_context==0 || thr->wt_context!=child->wt_context), function
//    of_JOIN_DETACH, file vthread.cc, line 3793` (a known iverilog join_none /
//    background-detach bug, not an SV-legality issue — the source is plain
//    `fork … join_none` inside a task, which IEEE §9.3.2 permits). The expected
//    value below is by IEEE timing reasoning, not a differential match. ──
#[test]
fn fork_join_none_of_tasks() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat (2) @(posedge clk); endtask\n\
        task automatic run;\n\
          @(posedge clk); fork a(); a(); join_none $display(\"forked @%0t\", $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // run's @(posedge clk) fires at t=5; join_none does not block, so the very
    // next statement ($display) runs immediately in the same time step, t=5. The
    // two forked `a()` children (each repeat(2) @(posedge clk) → done at t=25)
    // keep running in the background after the parent moves on.
    assert!(!is_loud(&o) && o.contains("forked @5"), "join_none:\n{o}");
}

// ── two forks in sequence in the same task — the second fork's window/child
//    bookkeeping must not leak state from the first. ORACLE iverilog 13.0 MATCH
//    (DONE @15). ──
#[test]
fn two_sequential_forks() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; @(posedge clk); endtask\n\
        task automatic run;\n\
          fork a(); a(); join fork a(); a(); join $display(\"DONE @%0t\", $time);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    // 1st fork at t=0: both `a()` wait 1 posedge → t=5; join resumes t=5.
    // 2nd fork at t=5: both `a()` wait 1 posedge → next posedge t=15; join resumes t=15.
    assert!(!is_loud(&o) && o.contains("DONE @15"), "two forks:\n{o}");
}

// ── correct-or-loud: an arm passing a parent frame-local as a task arg is Case B
//    (needs the shared window) → stays LOUD in Stage 1 (never silent-wrong) ──
#[test]
fn fork_arg_is_parent_local_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic use_it(input int v); @(posedge clk); $display(\"v=%0d\", v); endtask\n\
        task automatic run; int x = 7; @(posedge clk);\n\
          fork use_it(x); join $display(\"done\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "arm reading a parent frame-local must stay loud:\n{o}"
    );
}

// ── correct-or-loud: a NESTED fork inside an arm stays LOUD ──
#[test]
fn fork_nested_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; @(posedge clk); endtask\n\
        task automatic run; @(posedge clk);\n\
          fork begin fork a(); join end join $display(\"done\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "nested fork must stay loud:\n{o}");
}

// ── correct-or-loud (review Finding 1, position #1): an arm whose `#(d)` DELAY
//    AMOUNT reads a parent frame-local is Case B (the amount is evaluated on the
//    arm's empty owned window). Before the classifier fix this misclassified Case A
//    → a frame_eval panic ("index out of bounds len 0"); it must be a clean E3009. ──
#[test]
fn fork_delay_amount_is_parent_local_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int d = 3;\n\
          @(posedge clk);\n\
          fork #(d) $display(\"armhi @%0t\", $time); join\n\
          $display(\"PASS @%0t\", $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "arm delay amount reading a parent frame-local must stay loud (no panic):\n{o}"
    );
}

// ── correct-or-loud (review Finding 1, position #2): an arm writing a module array
//    element `mem[d]` where the INDEX `d` is a parent frame-local is Case B (the index
//    is evaluated on the empty owned window). Before the fix this panicked; it must be
//    a clean E3009. ──
#[test]
fn fork_lvalue_index_is_parent_local_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        logic [7:0] mem [0:3];\n\
        task automatic run;\n\
          int d = 1;\n\
          @(posedge clk);\n\
          fork mem[d] = 8'hAA; join\n\
          $display(\"PASS mem1=%02h\", mem[1]);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "arm lvalue index reading a parent frame-local must stay loud (no panic):\n{o}"
    );
}

// ── correct-or-loud (review Finding 1, static variant): the SAME Case-B arm in a
//    STATIC (recursive → framed) task. Before the fix this did NOT panic — it SILENTLY
//    RAN off the static slab (window=None), a silent-wrong. It must go loud (E3009). ──
#[test]
fn fork_static_task_case_b_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        logic [7:0] mem [0:3];\n\
        task run(input int n);\n\
          if (n > 1) run(n - 1);\n\
          @(posedge clk);\n\
          fork mem[n] = 8'hAA; join\n\
        endtask\n\
        initial begin run(2); #100 $display(\"mem1=%02h mem2=%02h\", mem[1], mem[2]); $finish; end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "static-task Case-B fork arm must go loud, not silently run off the static slab:\n{o}"
    );
}

// ── correct-or-loud: an arm doing a whole-net blocking WRITE to a parent
//    frame-local (`x = 42`) is Case B — distinct from the existing index-write
//    (`fork_lvalue_index_is_parent_local_stays_loud`) and arg-read
//    (`fork_arg_is_parent_local_stays_loud`) guards, which hit a chunk's `word`/
//    in-bind expr; this hits a plain `lhs.chunks` net match in `classify_one_arm`'s
//    `BlockingAssign` arm. Stays LOUD in Stage 1 (Case B is a Stage-2 follow-on). ──
#[test]
fn case_b_arm_writes_parent_local_stays_loud_stage1() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          fork begin @(posedge clk); x = 42; end @(posedge clk); join\n\
          $display(\"x=%0d\", x);\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "case B whole-scalar write must stay loud in Stage 1:\n{o}"
    );
}

// ── correct-or-loud: `wait fork` inside a frame body — a fork-family construct
//    with no in-frame implicit-child-barrier support — stays LOUD (all stages,
//    design §8). Before the `frame_task_has_unsafe_construct` fix (this slice) this
//    slipped past BOTH elaborate guards (`wait_cond_reads_frame_local` explicitly
//    returns `false` for `WaitCause::Fork`, and `frame_body_is_leaf_nonsuspending`
//    treats every `Wait` cause uniformly) and reached the engine's runtime
//    in-frame `WaitCause::Fork` arm, which calls `mark_fatal()` — a backstop that
//    reuses the delta-limit diagnostic (VITA-F4016 "did not converge"), a
//    misleading RUNTIME fatal instead of a clean compile-time E3009. ──
#[test]
fn wait_fork_in_frame_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; @(posedge clk); endtask\n\
        task automatic run;\n\
          fork a(); join_none wait fork; $display(\"X\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "wait fork in frame must stay loud:\n{o}");
}

// ── correct-or-loud: `disable fork` inside a frame body stays LOUD (all stages).
//    Already caught by `frame_task_has_unsafe_construct`'s disable-fork arm before
//    this slice; locked in here as a fork-in-frame regression guard alongside its
//    sibling `wait fork` case above. ──
#[test]
fn disable_fork_in_frame_stays_loud() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic a; repeat(9) @(posedge clk); endtask\n\
        task automatic run;\n\
          fork a(); join_none disable fork; $display(\"X\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(is_loud(&o), "disable fork in frame must stay loud:\n{o}");
}
