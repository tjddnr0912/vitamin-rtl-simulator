//! `$finish` / `$stop` inside a function or task body. ROADMAP §3 ⑧.
//!
//! One statement in a library function refused 2,155 lines of `verilog-ethernet`:
//! `lfsr_mask` guards its `$finish` with `if (LFSR_CONFIG == "FIBONACCI") … else`, a
//! parameter every instantiation in the tree satisfies, so the statement is provably
//! never executed — and `classify_frame_body` is a STATIC walk, which sees it anyway.
//!
//! ⭐⭐ **The shape is admitted; the semantics are refused at RUNTIME.** §4.5.372 built
//! the real promotion and reverted it after four review rounds, because a frame body
//! that stops half-way owes its caller a return value and the three tools each pick a
//! different one — iverilog does not assign at all, verilator runs the body to the end,
//! vita commits whatever the return slot holds. Choosing any of them trades this loud
//! for a silent wrong answer. Ending the run when one is REACHED never has to answer
//! the question: there is no caller left to hand a value to. That is the cheap
//! alternative the queue line recorded, and it is what ships.
//!
//! ⚠️ So a design that reaches one moves loud → loud (a refusal at elaborate becomes a
//! fatal at simulation), and a design that does not moves loud → CORRECT. Only the
//! second is a gain, and it is the whole demand.
//!
//! ⚠️ Three routings had to learn this at once — `elaborate/frames_classify.rs`, the
//! `&self` executors in `state/frame_eval.rs` and `state/task_frames.rs`, and the
//! tier-3 admission walk in `native/frames.rs`. §4.5.372's round-2 review found what
//! teaching only one does: the untaught executor's `_ => {}` DROPS the statement, and
//! the untaught native walk sends the design to the VM with a message saying the
//! executor drops it, which would then be false.
//!
//! Values pinned to iverilog 13.0 and verilator 5.050.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fisb_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.v");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// `lfsr_mask`, reduced: the `$finish` is in the arm the parameter never selects, and
/// the design runs. Both oracles print `m=7`; the PRE binary printed nothing and exited
/// 1 with `E3009 … outside the frame-call subset`.
#[test]
fn a_finish_in_a_branch_the_parameters_never_take_no_longer_refuses_the_design() {
    let (o, ok) = run("module top;\n  \
           parameter CFG = \"A\";\n  \
           function [7:0] f(input [7:0] x);\n    \
             begin if (CFG == \"A\") f = x; else begin f = 8'd0; $finish; end end\n  \
           endfunction\n  \
           wire [7:0] m = f(8'd7);\n  \
           initial begin #1 $display(\"OUT m=%0d\", m); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("OUT m=7"), "both oracles print 7:\n{o}");
}

/// …and the twin that is REACHED. Loud, located, and it names why it is not performed.
/// iverilog finishes cleanly here (exit 0) and verilator refuses the source outright, so
/// there is no consensus to adopt — see the module docstring.
#[test]
fn a_reached_finish_is_a_located_fatal_not_a_finish() {
    let (o, ok) = run("module top;\n  \
           parameter CFG = \"B\";\n  \
           function [7:0] f(input [7:0] x);\n    \
             begin if (CFG == \"A\") f = x; else begin f = 8'd0; $finish; end end\n  \
           endfunction\n  \
           wire [7:0] m = f(8'd7);\n  \
           initial begin #1 $display(\"OUT m=%0d\", m); $finish; end\n\
         endmodule\n");
    assert!(
        !ok,
        "a reached `$finish` in a subroutine body must not exit 0:\n{o}"
    );
    assert!(
        o.contains("F-RUN-FATAL") && o.contains("`$finish` was reached inside a subroutine body"),
        "the fatal must name the construct and the reason:\n{o}"
    );
    assert!(
        o.contains("t.v:4:") && o.contains("[in top.f]"),
        "…and WHERE: the motivating one sits in a library function instantiated eighty \
         times, so a location-less fatal would be unactionable:\n{o}"
    );
}

/// `$stop` takes the same route — one arm, both spellings, because the reason they
/// cannot be performed is the same reason.
#[test]
fn a_reached_stop_is_the_same_fatal() {
    let (o, ok) = run("module top;\n  \
           function [7:0] f(input [7:0] x); begin f = 8'd0; $stop; end endfunction\n  \
           wire [7:0] m = f(8'd7);\n  \
           initial begin #1 $display(\"OUT m=%0d\", m); $finish; end\n\
         endmodule\n");
    assert!(!ok, "{o}");
    assert!(
        o.contains("`$stop` was reached inside a subroutine body"),
        "{o}"
    );
}

/// ⚠️ The cell that says this did NOT re-route the task path. A `$finish` in a task
/// called from a process still runs on the statement executor and still ends the run
/// CLEANLY — admitting the shape at elaborate must not turn a working `$finish` into a
/// fatal, which would be correct → loud. Instrumenting the new `task_frames` arm across
/// the whole suite and every `$finish`-in-a-task spelling showed it never fires; this
/// pins the outcome rather than the instrumentation.
#[test]
fn a_finish_in_a_task_still_finishes_cleanly() {
    let (o, ok) = run("module top;\n  \
           task automatic t(input [7:0] x); begin $display(\"OUT in-task %0d\", x); $finish; end \
           endtask\n  \
           initial begin #3 t(8'd9); $display(\"OUT after\"); end\n\
         endmodule\n");
    assert!(ok, "a task `$finish` must stay a clean finish:\n{o}");
    assert!(o.contains("OUT in-task 9"), "{o}");
    assert!(
        !o.contains("OUT after"),
        "the run must end at the `$finish`:\n{o}"
    );
}

/// ⚠️ RESIDUE, pinned so it is not mistaken for this slice's doing: the body keeps
/// executing after the latch, so the statement AFTER a reached `$finish` still prints.
/// That is `$fatal`'s behaviour in a frame body too — the boundary is the enclosing
/// STATEMENT, not the system task — and the twin below proves the two agree. iverilog
/// prints only `before` for both.
#[test]
fn the_statement_after_a_reached_finish_still_prints_exactly_as_after_a_fatal() {
    let src = |end_task: &str| {
        format!(
            "module top;\n  reg [7:0] r;\n  \
               function [7:0] f(input [7:0] x);\n    \
                 begin $display(\"OUT before\"); {end_task} $display(\"OUT after\"); f = x; end\n  \
               endfunction\n  \
               initial begin #3 r = f(8'd9); $display(\"OUT r=%0d\", r); end\n\
             endmodule\n"
        )
    };
    let (fin, fin_ok) = run(&src("$finish;"));
    let (fat, fat_ok) = run(&src("$fatal(1, \"boom\");"));
    assert!(!fin_ok && !fat_ok, "both end the run with an error");
    for (name, o) in [("$finish", &fin), ("$fatal", &fat)] {
        assert!(o.contains("OUT before"), "{name}: {o}");
        assert!(
            o.contains("OUT after"),
            "{name}: KNOWN residue — iverilog stops at the system task:\n{o}"
        );
        assert!(
            !o.contains("OUT r="),
            "{name}: the caller's statement must not complete:\n{o}"
        );
    }
}
