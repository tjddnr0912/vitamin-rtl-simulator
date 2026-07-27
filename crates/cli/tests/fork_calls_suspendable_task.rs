//! A TOP-LEVEL `fork` whose arms CALL a suspendable (framed) task — distinct from
//! `fork_in_frame.rs`, which forks from INSIDE a task frame.
//!
//! Regression for a silent-wrong: the in-frame child-completion intercept compared the
//! child's sole frame `bb` against the barrier's `join_bb`, but for a top-level fork
//! those live in different numbering spaces (`bb` = a global `ir.blocks` id, `join_bb` =
//! a process-local block index). Any numeric collision killed the child mid-task, so the
//! task body after the first `@`/`#` vanished at exit 0 — no diagnostic. Padding the task
//! CFG (pushing its resume `bb` past the collision) made the SAME design pass, which is
//! what identified the cause; `FrameRec::is_arm` now gates the intercept.
//!
//! ORACLE: iverilog 13.0 on every design below.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fcst_{}_{n}", std::process::id()));
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

// ── the repro: two arms, each calling a task that parks on `@` ──
// PRE printed only `enter 1`/`enter 2`/`done`; iverilog prints both `after` lines.
#[test]
fn fork_arms_call_suspendable_task_join() {
    let o = run("module t;\n\
        reg clk = 0; always #5 clk = ~clk;\n\
        task automatic tk(input int id);\n\
          int loc; loc = id;\n\
          $display(\"enter %0d\", loc);\n\
          @(posedge clk);\n\
          $display(\"after %0d\", loc);\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join $display(\"done\"); $finish; end\n\
        endmodule\n");
    for want in ["enter 1", "enter 2", "after 1", "after 2", "done"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── the arm's task calls ANOTHER task that parks: the frame stack is 2 deep at the
// suspend, so the intercept must not fire at either depth.
#[test]
fn fork_arm_task_nested_call_parks() {
    let o = run("module t;\n\
        reg clk = 0; always #5 clk = ~clk;\n\
        task automatic inner(input int id);\n\
          @(posedge clk); $display(\"inner-after %0d\", id);\n\
        endtask\n\
        task automatic tk(input int id);\n\
          $display(\"enter %0d\", id); inner(id); $display(\"after %0d\", id);\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join $display(\"done\"); $finish; end\n\
        endmodule\n");
    for want in [
        "enter 1",
        "enter 2",
        "inner-after 1",
        "inner-after 2",
        "after 1",
        "after 2",
        "done",
    ] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── copy-out of an OUTPUT formal must still happen after the resume ──
#[test]
fn fork_arm_task_output_formal_copies_out() {
    let o = run("module t;\n\
        reg clk = 0; always #5 clk = ~clk;\n\
        task automatic tk(input int id, output int o);\n\
          @(posedge clk); o = id * 10;\n\
        endtask\n\
        int r1, r2;\n\
        initial begin fork tk(1, r1); tk(2, r2); join\n\
          $display(\"r=%0d %0d\", r1, r2); $finish; end\n\
        endmodule\n");
    assert!(o.contains("r=10 20"), "copy-out:\n{o}");
}

// ── join_none + `wait fork`: PRE dropped both `after` lines ──
#[test]
fn fork_arm_task_join_none_wait_fork() {
    let o = run("module t;\n\
        reg clk = 0; always #5 clk = ~clk;\n\
        task automatic tk(input int id);\n\
          $display(\"enter %0d\", id); @(posedge clk); $display(\"after %0d\", id);\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join_none\n\
          wait fork; $display(\"none-done\"); $finish; end\n\
        endmodule\n");
    for want in ["enter 1", "enter 2", "after 1", "after 2", "none-done"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── join_any: the SURVIVING arm keeps running after the barrier fires. PRE lost it. ──
#[test]
fn fork_arm_task_join_any_survivor_completes() {
    let o = run("module t;\n\
        reg clk = 0; always #5 clk = ~clk;\n\
        task automatic short_t; $display(\"enter S\"); @(posedge clk); $display(\"after S\"); endtask\n\
        task automatic long_t;  $display(\"enter L\"); @(posedge clk); @(posedge clk); $display(\"after L\"); endtask\n\
        initial begin fork short_t(); long_t(); join_any\n\
          $display(\"any-done at %0t\", $time); #40 $finish; end\n\
        endmodule\n");
    for want in ["enter S", "enter L", "after S", "any-done at 5", "after L"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── `disable fork` still kills a parked task arm (no-regression pin) ──
#[test]
fn fork_arm_task_disable_fork_kills_parked_arm() {
    let o = run("module t;\n\
        reg clk = 0; always #5 clk = ~clk;\n\
        task automatic tk(input int id);\n\
          $display(\"enter %0d\", id); @(posedge clk); @(posedge clk);\n\
          $display(\"after %0d\", id);\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join_none\n\
          @(posedge clk); disable fork;\n\
          $display(\"killed at %0t\", $time); #40 $finish; end\n\
        endmodule\n");
    assert!(
        o.contains("enter 1") && o.contains("killed at 5"),
        "kill:\n{o}"
    );
    assert!(!o.contains("after 1"), "arm survived `disable fork`:\n{o}");
}
