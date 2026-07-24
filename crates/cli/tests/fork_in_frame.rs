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

// ── Stage 2 (Case B / join-all): a join arm WRITES a parent automatic local; the
//    parent reads it after join. ORACLE iverilog 13.0 MATCH (`x=42 @25`). Needs the
//    interior-mutable shared-window arena (WindowSlot::Shared) so the parked parent
//    and the running arm reference one window by handle. ──
#[test]
fn case_b_join_arm_writes_parent_local() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          @(posedge clk);\n\
          fork\n\
            begin @(posedge clk); x = 42; end\n\
            begin @(posedge clk); @(posedge clk); end\n\
          join\n\
          $display(\"x=%0d @%0t\", x, $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // run @posedge t=5; arm0 writes x=42 at t=15; arm1 done t=25; join resumes t=25.
    assert!(!is_loud(&o) && o.contains("x=42 @25"), "case B join:\n{o}");
}

// ── Stage 2 (Case B / join-all): a SIBLING arm sees another arm's write to the shared
//    parent local. ORACLE iverilog 13.0 MATCH (`sib sees x=7 @6`, `run done x=7 @6`).
//    Timing-separated (posedge write, sibling reads one tick later) so the read strictly
//    follows the write in time — iverilog has a zero-delay-fork-arm scheduling quirk where
//    a bare `x = 7;` arm is not seen by a sibling in the same instant, so this form (which
//    both simulators agree on) is the robust differential shape. ──
#[test]
fn case_b_sibling_visibility() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          fork\n\
            begin @(posedge clk); x = 7; end\n\
            begin @(posedge clk); #1; if (x == 7) $display(\"sib sees x=%0d @%0t\", x, $time); end\n\
          join\n\
          $display(\"run done x=%0d @%0t\", x, $time);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // arm0 writes x=7 at t=5; arm1 reads it at t=6; join resumes t=6.
    assert!(
        !is_loud(&o) && o.contains("sib sees x=7 @6") && o.contains("run done x=7 @6"),
        "sibling visibility:\n{o}"
    );
}

// ── Stage 2 (Case B / join-all): an arm passes a parent automatic local as a task-call
//    ARG. The arg is read on the shared arena window (via `classify_one_arm`'s Call in-bind
//    path), so the nested `use_it(x)` sees x=7. ORACLE iverilog 13.0 MATCH (`v=7`, `done`).
//    (Was a Stage-1 loud placeholder; the shared window now makes it correct-support.) ──
#[test]
fn case_b_join_arm_passes_parent_local_as_arg() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic use_it(input int v); @(posedge clk); $display(\"v=%0d\", v); endtask\n\
        task automatic run; int x = 7; @(posedge clk);\n\
          fork use_it(x); join $display(\"done\");\n\
        endtask\n\
        initial begin run(); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("v=7") && o.contains("done"),
        "arm passing a parent frame-local as an arg (Case B join):\n{o}"
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

// ── Stage 2 (Case B / join-all): an arm whose `#(d)` DELAY AMOUNT reads a parent
//    automatic local. The amount is evaluated on the shared arena window (via
//    `classify_one_arm`'s Delay path), so `#(d)` waits d=3 ticks. ORACLE iverilog 13.0
//    MATCH (`armhi @8`, `PASS @8`). In Stage 1 this was loud (Case A gave the arm an
//    empty owned window → the amount read would panic); the shared window makes it correct
//    (a regression guard that the arena is restored when the Delay amount is evaluated). ──
#[test]
fn case_b_join_arm_delay_amount_is_parent_local() {
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
        !is_loud(&o) && o.contains("armhi @8") && o.contains("PASS @8"),
        "arm delay amount reading a parent frame-local (Case B join):\n{o}"
    );
}

// ── Stage 2 (Case B / join-all): an arm writes a module array element `mem[d]` where the
//    INDEX `d` is a parent automatic local. The index is read on the shared arena window
//    (via `classify_one_arm`'s chunk-word path). ORACLE iverilog 13.0 MATCH (`mem1=aa`). In
//    Stage 1 this was loud (the empty owned window would panic on the index read). ──
#[test]
fn case_b_join_arm_lvalue_index_is_parent_local() {
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
        !is_loud(&o) && o.contains("PASS mem1=aa"),
        "arm lvalue index reading a parent frame-local (Case B join):\n{o}"
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

// ── Stage 3 (was Stage-2 boundary): a Case-B fork with `join_any` is now SUPPORTED.
//    The refcounted shared window lets the surviving arm outlive the parent. Both arms
//    complete at the same posedge (t=5); the fast one fires join_any and the parent resumes
//    and reads x = 42 (arm0, lower tie, ran its write before the parent's continuation).
//    ORACLE: iverilog crashes on the surviving arm (of_JOIN_DETACH), so hand-IEEE. ──
#[test]
fn case_b_join_any_now_supported() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          fork begin @(posedge clk); x = 42; end @(posedge clk); join_any\n\
          $display(\"x=%0d\", x);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("x=42"),
        "case B join_any is now supported (refcounted shared window):\n{o}"
    );
}

// ── Stage 3 (was Stage-2 boundary): a Case-B fork with `join_none` is now SUPPORTED.
//    The parent does NOT block, so it reads x = 0 (the arm's posedge write lands later, at
//    t=5, on the still-live refcounted window — see `case_b_join_none_child_reads_after_return`
//    for the arm actually reading a parent local past the parent's Return). ──
#[test]
fn case_b_join_none_now_supported() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          int x = 0;\n\
          fork begin @(posedge clk); x = 42; end join_none\n\
          $display(\"x=%0d\", x);\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("x=0"),
        "case B join_none is now supported (parent continues; x=0 before the arm writes):\n{o}"
    );
}

// ── Stage 3 (join_none twin): a LONE join_none child reads a parent automatic local at a
//    `#20` delay — LONG AFTER the parent returned (at t=0). The shared window must survive
//    via the refcount. ORACLE: iverilog crashes on the detached child (of_JOIN_DETACH); the
//    asserted value is hand-IEEE (§9.3.2: the automatic local stays live for the child). ──
#[test]
fn case_b_join_none_child_reads_after_return() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          automatic int x = 55;\n\
          fork begin #20; if (x == 55) $display(\"jn child sees x=%0d @%0t\", x, $time); end join_none\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // run() returns at t=0 (join_none); the child reads x at t=20 on the surviving window.
    assert!(
        !is_loud(&o) && o.contains("jn child sees x=55 @20"),
        "join_none child reading a parent local after the parent returned:\n{o}"
    );
}

// ── Stage 3 adversarial (rc soundness a): TWO surplus children both reference the parent
//    local AFTER the parent returned. The window must be freed only after BOTH complete —
//    rc goes 1(alloc)+3(retain)=4 → fast arm completes (3) → parent Return (2) → surplus s1
//    at t=25 (1) → surplus s2 at t=45 (0, freed). Runs under `cargo test` with debug asserts
//    active: any premature free (rc==0 access) or double-free (rc underflow) panics. ──
#[test]
fn case_b_join_any_two_surplus_children() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          automatic int x = 9;\n\
          fork\n\
            @(posedge clk);\n\
            begin repeat (3) @(posedge clk); if (x == 9) $display(\"s1 sees x=%0d @%0t\", x, $time); end\n\
            begin repeat (5) @(posedge clk); if (x == 9) $display(\"s2 sees x=%0d @%0t\", x, $time); end\n\
          join_any\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // Both surplus arms read x=9 on the window kept alive until the LAST of them completes.
    assert!(
        !is_loud(&o) && o.contains("s1 sees x=9 @25") && o.contains("s2 sees x=9 @45"),
        "two surplus children sharing one refcounted window:\n{o}"
    );
}

// ── Stage 3 adversarial (rc soundness b): a Case-B join_none task CALLED in a loop — each
//    call allocs a DISTINCT arena window (the prior call's window is still live, so its
//    handle is NOT on the free-list → no reuse-while-live). Each child reads its OWN call's
//    automatic local, so the three distinct values (10/20/30) prove the windows never alias.
//    Debug asserts (rc>0 on access, rc==0 before free) active — no leak, no double-free. ──
#[test]
fn case_b_join_none_loop_distinct_windows() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic spawn(input int id);\n\
          automatic int x = id * 10;\n\
          fork begin repeat (2) @(posedge clk); $display(\"child%0d sees x=%0d\", id, x); end join_none\n\
        endtask\n\
        initial begin\n\
          for (int i = 1; i <= 3; i++) spawn(i);\n\
          #100 $finish;\n\
        end\n\
        endmodule\n");
    // Three concurrent live windows (h0/h1/h2), each read by its own child at t=15.
    assert!(
        !is_loud(&o)
            && o.contains("child1 sees x=10")
            && o.contains("child2 sees x=20")
            && o.contains("child3 sees x=30"),
        "join_none in a caller loop — distinct per-activation windows, no reuse-while-live:\n{o}"
    );
}

// ── Stage 3 (Case B / join_any): the SURPLUS (slow) child references a parent automatic
//    local AFTER the parent resumed+returned — the shared window must OUTLIVE the parent
//    (refcount). Needs `frame_window_rc`: the parent releases at Return, the surplus keeps
//    the window alive until it completes. ORACLE: iverilog crashes on a detached/surviving
//    fork child (`of_JOIN_DETACH` assertion), so this is hand-IEEE (§9.3.2): the arm reads
//    the same automatic `x` the parent initialized, which IEEE keeps live for the arm. ──
#[test]
fn case_b_join_any_surplus_outlives_parent() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        task automatic run;\n\
          automatic int x = 9;\n\
          fork\n\
            @(posedge clk);\n\
            begin repeat (3) @(posedge clk); if (x == 9) $display(\"surplus sees x=%0d @%0t\", x, $time); end\n\
          join_any\n\
        endtask\n\
        initial begin run(); #100 $finish; end\n\
        endmodule\n");
    // fast: posedge t=5 → join_any resumes, run RETURNS at t=5. surplus: 3 posedges
    // → t=25, still reads x=9 (window kept alive by refcount).
    assert!(
        !is_loud(&o) && o.contains("surplus sees x=9 @25"),
        "join_any surplus lifetime:\n{o}"
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
