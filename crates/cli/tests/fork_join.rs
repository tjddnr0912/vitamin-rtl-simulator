//! A4-a ABSOLUTE ANCHOR — a PROCESS-LEVEL `fork` on tier-3, iverilog-pinned.
//!
//! `fork` was the last design-gate family and the whole of what still blocked
//! tier-3: every one of the 110 designs the census still counted was in its
//! family. The row is gone for a process-level fork; what stays refused is a
//! fork INSIDE a task frame (`native::frames`'s own row) and a bare `wait fork`
//! (the executor row).
//!
//! The design is delegation: `Scheduler::exec_fork_into` and
//! `on_child_complete_into` do the barrier, the tie composition, the window
//! sharing and the `JoinMode` decision — everything that is queue-independent —
//! and the kernel supplies only the queue the children go on. A second spelling
//! of the tie order would make sibling arms nondeterministic in one backend
//! only, and an under-decrement would fire an All-barrier EARLY, which is a
//! wrong ORDER rather than a crash.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(backend: &str, src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fork_{}_{n}", std::process::id()));
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
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    let body: String = txt
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .fold(String::new(), |mut a, l| {
            a.push_str(l);
            a.push('\n');
            a
        });
    (body, rj)
}

fn run_native(src: &str) -> (String, String) {
    run_on("native", src)
}

/// `fork … join` with arms that PARK — the shape 87% of forking designs have.
///
/// The arms delay for different amounts, so the ORDER the prints come out in is
/// decided by the scheduler rather than by declaration: arm2 (`#1`) must print
/// before arm1 (`#2`), and the parent must not resume until BOTH have.
///
/// ⚠️ Measured the hard way. The first version of the walk had no
/// child-completion intercept, so an arm ran past its join into the PARENT's
/// continuation: `fork a=1; b=2; join` printed `a=1 b=0` because the first arm
/// executed the code after the join before the second arm started. Not a hang
/// and not a panic — a wrong order at exit 0.
#[test]
fn fork_join_with_parking_arms_matches_iverilog() {
    let (body, rj) = run_native(
        "module top;\n\
           integer a = 0, b = 0;\n\
           initial begin\n\
             fork\n\
               begin #2; a = 1; $display(\"arm1 t=%0t\", $time); end\n\
               begin #1; b = 2; $display(\"arm2 t=%0t\", $time); end\n\
             join\n\
             $display(\"join a=%0d b=%0d t=%0t\", a, b, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "arm2 t=1\n\
         arm1 t=2\n\
         join a=1 b=2 t=2\n"
    );
}

/// `join_any` — the parent resumes on the FIRST arm, and the surplus arm stays
/// live and runs to completion.
///
/// Both halves are discriminators: `a=0` at the resume shows the parent did not
/// wait for arm1, and `late a=1` shows arm1 was not killed by the barrier firing.
#[test]
fn fork_join_any_resumes_on_the_first_arm() {
    let (body, rj) = run_native(
        "module top;\n\
           integer a = 0, b = 0;\n\
           initial begin\n\
             fork\n\
               begin #2; a = 1; $display(\"arm1 t=%0t\", $time); end\n\
               begin #1; b = 2; $display(\"arm2 t=%0t\", $time); end\n\
             join_any\n\
             $display(\"any a=%0d b=%0d t=%0t\", a, b, $time);\n\
             #5 $display(\"late a=%0d b=%0d\", a, b);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "arm2 t=1\n\
         any a=0 b=2 t=1\n\
         arm1 t=2\n\
         late a=1 b=2\n"
    );
}

/// `join_none` — the parent does not block at all, and the arm runs later.
///
/// This is the mode that rules out the cheap implementation: running the arms
/// inline in declaration order would print `arm` BEFORE `none`, because the
/// parent is supposed to continue first and the arm is merely queued. It is why
/// the slice delegates to the scheduler instead of executing arms in place.
#[test]
fn fork_join_none_continues_before_the_arm() {
    let (body, rj) = run_native(
        "module top;\n\
           integer a = 0;\n\
           initial begin\n\
             fork\n\
               begin #2; a = 1; $display(\"arm t=%0t\", $time); end\n\
             join_none\n\
             $display(\"none a=%0d t=%0t\", a, $time);\n\
             #5 $display(\"late a=%0d t=%0t\", a, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "none a=0 t=0\n\
         arm t=2\n\
         late a=1 t=5\n"
    );
}

/// A fork arm that CALLS A PARKING TASK — the only shape that runs the
/// child-completion intercept's guard in its false direction.
///
/// The intercept only asks `k_child_join_bb` when `frames.is_empty()`, and every
/// other test here leaves that vacuously true. Here each arm is inside
/// `bump`'s frame when it suspends, so the walk resumes with a non-empty frame
/// stack and the guard is what stops the arm's `bb` (an index into the GLOBAL
/// `ir.blocks`) from being compared against a join sentinel (an index into
/// `ir.processes[t].body`). Without it a numeric collision between the two
/// spaces kills the arm mid-task.
///
/// It also stacks A4-a on A3-ii-b: the arm's frames are parked and restored per
/// ACTIVITY, so the two arms' `n`/`o` windows must not see each other — `a=20`
/// and `b=10` are that, and they would both read the same number if one window
/// leaked.
#[test]
fn a_fork_arm_may_call_a_parking_task() {
    let (body, rj) = run_native(
        "module top;\n\
           integer a = 0, b = 0;\n\
           task automatic bump(input integer n, output integer o);\n\
             begin #n; o = n * 10; end\n\
           endtask\n\
           initial begin\n\
             fork\n\
               begin bump(2, a); $display(\"arm1 a=%0d t=%0t\", a, $time); end\n\
               begin bump(1, b); $display(\"arm2 b=%0d t=%0t\", b, $time); end\n\
             join\n\
             $display(\"join a=%0d b=%0d t=%0t\", a, b, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "arm2 b=10 t=1\n\
         arm1 a=20 t=2\n\
         join a=20 b=10 t=2\n"
    );
}

/// TWO forks in sequence — the only shape that can see which key orders siblings.
///
/// ⚠️⚠️ THE BATTERY FOUND THIS AXIS, and it was blind in all three tests above.
/// Replacing the sibling sort key with the ACTIVITY ID survived them, because a
/// design that forks once allocates its children in declaration order and the
/// two keys agree by accident. They come apart on the SECOND fork: a completed
/// child's slot goes back on `free_activities`, which is a LIFO, so the second
/// fork's arms are handed ids in an order decided by the order the FIRST fork's
/// arms happened to finish in. Measured with the mutation applied: `B1 B2 B3`
/// became `B2 B1 B3`. Wrong order, exit 0, and only in designs that fork twice.
///
/// The first fork is deliberately staggered (`#1` then `#2`) so its arms retire
/// in declaration order and push the free list into the state that reverses the
/// pair. A same-delay first fork would not set it up.
///
/// ⚠️ IVERILOG IS NOT THE ORACLE FOR THIS ONE. IEEE leaves the order of
/// concurrent same-time arms unspecified, and iverilog runs the zero-delay arms
/// in REVERSE declaration order (`B3 B2 B1`, measured with iverilog 13). vita's
/// rule is declaration order; what makes it a rule rather than an accident is
/// that all THREE vita backends have to produce it, which is what the assert
/// below pins — the absolute string, and interp/vm/native agreeing on it.
#[test]
fn sibling_arms_keep_declaration_order_across_a_second_fork() {
    const SRC: &str = "module top;\n\
           initial begin\n\
             fork\n\
               begin #1; $display(\"A1 t=%0t\", $time); end\n\
               begin #2; $display(\"A2 t=%0t\", $time); end\n\
             join\n\
             fork\n\
               $display(\"B1 t=%0t\", $time);\n\
               $display(\"B2 t=%0t\", $time);\n\
               $display(\"B3 t=%0t\", $time);\n\
             join\n\
             $finish;\n\
           end\n\
         endmodule\n";
    const WANT: &str = "A1 t=1\n\
                        A2 t=2\n\
                        B1 t=2\n\
                        B2 t=2\n\
                        B3 t=2\n";

    let (native, rj) = run_native(SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT);

    // The cross-backend half. `tie` is the engine's ordering key too, so a
    // second spelling on either side shows up here as a disagreement.
    for b in ["vm", "interp"] {
        let (other, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}
