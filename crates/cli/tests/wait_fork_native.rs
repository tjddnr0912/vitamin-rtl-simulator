//! A4-c ABSOLUTE ANCHOR — `wait fork;` (IEEE §9.6.1) on TIER-3.
//!
//! Every one of the 8 designs the executor row still refused was a bare
//! `wait fork;` — measured, not assumed: the row named three shapes and a census
//! of the corpus found the other two (a `Terminator::Call` with no sidecar
//! entry, and `WaitCause::Named`) with zero population.
//!
//! The row's stated reason was right about the hazard and wrong about the
//! remedy. It said "nothing in `fire_waiters` can ever satisfy the cause, so
//! admitting it would park the process forever". True — because `wait fork` is
//! NOT a waiter. There is no net to file it under; what resumes it is the child
//! bookkeeping (`exec_wait_fork` parks it and counts live children,
//! `on_child_complete_into` counts them down), and tier-3 has had that since
//! A4-a. So the walk answers with a delegated call instead of filing a waiter
//! nothing fires, and the resume path needed no code at all: the parent is
//! pushed into the same `ready` vector the join barrier uses, which
//! `k_body_done` already drains.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_native(src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wfn_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
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

/// `join_none` detaches two children; `wait fork` collects both.
///
/// The discriminator is the pair of lines around the wait: `after fork a=0 b=0
/// t=0` shows the parent really did continue without blocking, and `after wait
/// a=1 b=2 t=4` shows it then waited for the LAST of them (t=4, not t=2). A
/// `wait fork` that resumed on the first child would print `t=2`; one that never
/// parked would print `t=0` with both values still zero. iverilog-pinned.
#[test]
fn wait_fork_collects_detached_children() {
    let (body, rj) = run_native(
        "module top;\n\
           integer a = 0, b = 0;\n\
           initial begin\n\
             fork\n\
               begin #2; a = 1; $display(\"c1 t=%0t\", $time); end\n\
               begin #4; b = 2; $display(\"c2 t=%0t\", $time); end\n\
             join_none\n\
             $display(\"after fork a=%0d b=%0d t=%0t\", a, b, $time);\n\
             wait fork;\n\
             $display(\"after wait a=%0d b=%0d t=%0t\", a, b, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "after fork a=0 b=0 t=0\n\
         c1 t=2\n\
         c2 t=4\n\
         after wait a=1 b=2 t=4\n"
    );
}

/// `wait fork` with NO live children falls through in the same time step.
///
/// This is the other half of the seam's contract and it is the half that hangs
/// if it is wrong: a `wait fork` that parked with nothing outstanding would
/// never be woken, because the thing that wakes it is a child completing. Both
/// occurrences matter — the first has never forked at all, the second is after a
/// `#3` so the process has advanced time without acquiring children.
/// iverilog-pinned.
#[test]
fn wait_fork_with_no_children_falls_through() {
    let (body, rj) = run_native(
        "module top;\n\
           initial begin\n\
             $display(\"before t=%0t\", $time);\n\
             wait fork;\n\
             $display(\"after t=%0t\", $time);\n\
             #3;\n\
             wait fork;\n\
             $display(\"again t=%0t\", $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "before t=0\n\
         after t=0\n\
         again t=3\n"
    );
}

/// `join_any` resumed the parent early; the SURPLUS arm is still outstanding and
/// `wait fork` must wait for it.
///
/// This is the shape that makes the count CUMULATIVE rather than per-barrier:
/// the surplus arm belongs to a barrier that has already fired, so a `wait fork`
/// implemented by asking "is my most recent barrier still outstanding" would
/// answer no and fall straight through, printing `waited a=1 b=0 t=1`. The
/// engine's `exec_wait_fork` instead scans the activity arena for every live
/// child whose barrier names this parent, and that is why the seam delegates
/// rather than restating it. iverilog-pinned.
#[test]
fn wait_fork_waits_for_a_join_any_surplus_arm() {
    let (body, rj) = run_native(
        "module top;\n\
           integer a = 0, b = 0;\n\
           initial begin\n\
             fork\n\
               begin #1; a = 1; $display(\"f1 t=%0t\", $time); end\n\
               begin #5; b = 2; $display(\"f2 t=%0t\", $time); end\n\
             join_any\n\
             $display(\"any a=%0d b=%0d t=%0t\", a, b, $time);\n\
             wait fork;\n\
             $display(\"waited a=%0d b=%0d t=%0t\", a, b, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "f1 t=1\n\
         any a=1 b=0 t=1\n\
         f2 t=5\n\
         waited a=1 b=2 t=5\n"
    );
}
