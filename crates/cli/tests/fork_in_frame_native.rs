//! A4-b ABSOLUTE ANCHOR — a `fork` INSIDE a suspendable task frame, on TIER-3.
//!
//! The neighbouring `fork_in_frame.rs` is §4.5.214's suite for the same feature
//! on the default backend; this file is the tier-3 half, and every test here
//! asserts `"backend": "native"` so it cannot pass by silently falling back.
//!
//! Three gate rows were closing on this shape and they had to go together,
//! which is the lesson of the slice. Two of them read the SAME predicate
//! (`frame_call::frame_forks`) — the storage row in `native::frames` and the
//! executor's `site_runnable` clause — so they always fired as a pair. The third
//! (`contains_shared_fork`) was masked behind them, because `frames_admitted`
//! returns its FIRST `Err`; it surfaced the moment the pair came out, on the
//! very first Case-B probe, and it covers fourteen of the twenty-four designs.
//!
//! Nothing about forking was built here. `Scheduler::exec_fork_into` has done
//! the in-frame case since §4.5.214 — `parent_in_frame` off the call stack,
//! `FRAME_FORK_KEY` for the mode lookup, `arm_callee` for the CFG the arm blocks
//! live in, and the Case A / Case B window split with its refcount. What tier-3
//! lacked was that its parked frames lived in a kernel-side map the scheduler
//! could not read, so a fork-in-frame would have been spawned as if it were top
//! level: arms with no window, sharing none of the parent's automatic locals.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(backend: &str, src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fifn_{}_{n}", std::process::id()));
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

/// CASE B — the arms write the enclosing automatic task's LOCALS.
///
/// This is the majority shape (14 of the 24 designs the census counted) and the
/// one the window machinery exists for: `enter_task_frame` gives a
/// `contains_shared_fork` task an interior-mutable ARENA window
/// (`WindowSlot::Shared`), each arm aliases it by handle with a retained
/// reference, and the parent's own window is stashed across the fork so the
/// concurrent arms taking turns on the shared `frame_stack` never see it.
///
/// The discriminator is that `a` and `b` are FRAME locals of `work`, not module
/// nets: if the arms did not share the parent's window they would write their
/// own copies and `r` would come back 0 rather than 38. iverilog-pinned.
///
/// ⚠️ This design is also what caught the THIRD gate row. With the pair removed
/// it still reported `buildable: false`, message "a subroutine with a shared
/// fork window" — the row A3-ii-b had left as a deliberate backstop, writing
/// "if that ever stops being true, this refuses instead of the design going
/// quiet". It did exactly that.
#[test]
fn case_b_arms_share_the_enclosing_frames_automatic_locals() {
    let (body, rj) = run_native(
        "module top;\n\
           integer r = 0;\n\
           task automatic work(input integer k);\n\
             integer a, b;\n\
             begin\n\
               a = 0; b = 0;\n\
               fork\n\
                 begin #2; a = k * 10; $display(\"armA a=%0d t=%0t\", a, $time); end\n\
                 begin #1; b = k + 5;  $display(\"armB b=%0d t=%0t\", b, $time); end\n\
               join\n\
               r = a + b;\n\
               $display(\"work k=%0d a=%0d b=%0d r=%0d t=%0t\", k, a, b, r, $time);\n\
             end\n\
           endtask\n\
           initial begin\n\
             work(3);\n\
             $display(\"done r=%0d t=%0t\", r, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "armB b=8 t=1\n\
         armA a=30 t=2\n\
         work k=3 a=30 b=8 r=38 t=2\n\
         done r=38 t=2\n"
    );
}

/// CASE A — the arms touch only MODULE nets, so no window is shared.
///
/// Kept as its own test because the two cases take DIFFERENT branches in
/// `exec_fork_into`: Case A gives each arm an empty `Owned` window and takes no
/// reference, Case B aliases a handle and retains one. Ten of the twenty-four
/// designs are this shape, and a slice that only closed Case B would pass the
/// test above while leaving them refused. iverilog-pinned.
#[test]
fn case_a_arms_touching_only_module_nets() {
    let (body, rj) = run_native(
        "module top;\n\
           integer x = 0, y = 0;\n\
           task automatic work();\n\
             begin\n\
               fork\n\
                 begin #2; x = 7; $display(\"armA t=%0t\", $time); end\n\
                 begin #1; y = 9; $display(\"armB t=%0t\", $time); end\n\
               join\n\
               $display(\"work x=%0d y=%0d t=%0t\", x, y, $time);\n\
             end\n\
           endtask\n\
           initial begin\n\
             work();\n\
             $display(\"done x=%0d y=%0d t=%0t\", x, y, $time);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "armB t=1\n\
         armA t=2\n\
         work x=7 y=9 t=2\n\
         done x=7 y=9 t=2\n"
    );
}

/// A frame-local DYNAMIC array in a task that forks, read by the arms.
///
/// ⚠️⚠️ THE BATTERY FOUND THIS AXIS. Deleting the `forked = true` mark that the
/// walk sets on the parent frame before the fork survived every other test here,
/// because none of them has a dyn-array frame local. `frame_window::park_dyn_in`
/// is what reads it: an automatic WINDOW is shared with the arms through a
/// `WindowSlot::Shared` handle, but a parked dyn array is simply GONE from the
/// net-keyed heap, so parking the parent's array at the fork leaves the arms
/// reading X. Measured with the mutation applied: `arm a0=x a1=x`, plus a
/// `W4020` — and `after a0=11` on the very next line, because the parent gets
/// its array back when it resumes. Half the output correct is what makes this
/// worth a test.
///
/// The consumer's own doc had already recorded the same shape from the engine
/// side ("a `fork begin … a[0] … end join` inside a task printed `a0=x`"), which
/// is why the discriminator was cheap to build once the battery pointed at it.
/// iverilog-pinned.
#[test]
fn arms_read_the_enclosing_frames_dynamic_array() {
    let (body, rj) = run_native(
        "module top;\n\
           task automatic work();\n\
             integer a[];\n\
             begin\n\
               a = new[3];\n\
               a[0] = 11; a[1] = 22; a[2] = 33;\n\
               fork\n\
                 begin #1; $display(\"arm a0=%0d a1=%0d\", a[0], a[1]); end\n\
                 begin #2; $display(\"arm2 a2=%0d\", a[2]); end\n\
               join\n\
               $display(\"after a0=%0d a2=%0d\", a[0], a[2]);\n\
             end\n\
           endtask\n\
           initial begin work(); $finish; end\n\
         endmodule\n",
    );
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(
        body,
        "arm a0=11 a1=22\n\
         arm2 a2=33\n\
         after a0=11 a2=33\n"
    );
}

/// `join_any` and `join_none` inside a frame, with arms that CALL a task.
///
/// Three things at once, each of which the two tests above cannot see:
///
///  * the arm calls `leaf`, so the arm's activity has TWO frames while it runs.
///    That is what `call_stack.len() == 1` in the in-frame completion intercept
///    is for — a deeper callee frame reaching a same-valued block id must not
///    mis-fire the join, and eleven of these designs have one.
///  * `join_any` leaves a surplus arm live past the parent's resume. Under a
///    shared window that arm can outlive the parent's `Return`, which is why the
///    arm's window is a retained reference and why the completion path releases
///    it rather than restoring it.
///  * `join_none` inside a frame is the mode that rules out running arms inline:
///    `none` must print BEFORE `late`.
///
/// ⚠️ IVERILOG IS NOT THE ORACLE HERE — it ABORTS on this design with an
/// internal assertion (`of_JOIN_DETACH`, vthread.cc:3793, iverilog 13), after
/// printing only the first two lines. The neighbouring `fork_in_frame.rs`
/// records the same crash for its own `join_none` case. So the expected text is
/// hand-IEEE (§9.3.2: `join_none` continues immediately; §9.3.1: `join_any`
/// blocks until one process completes) plus agreement across all three vita
/// backends, which the loop at the end asserts.
#[test]
fn join_any_and_join_none_inside_a_frame_with_calling_arms() {
    const SRC: &str = "module top;\n\
           integer p = 0, q = 0;\n\
           task automatic leaf(input integer n, output integer o);\n\
             begin #n; o = n * 100; end\n\
           endtask\n\
           task automatic work();\n\
             integer t;\n\
             begin\n\
               t = 0;\n\
               fork\n\
                 begin leaf(2, p); $display(\"A1 p=%0d t=%0t\", p, $time); end\n\
                 begin leaf(1, q); $display(\"A2 q=%0d t=%0t\", q, $time); end\n\
               join_any\n\
               t = 1;\n\
               $display(\"any p=%0d q=%0d t=%0t\", p, q, $time);\n\
               fork\n\
                 begin #4; $display(\"late t=%0t\", $time); end\n\
               join_none\n\
               $display(\"none t=%0t\", $time);\n\
               #6 $display(\"end t=%0t\", $time);\n\
             end\n\
           endtask\n\
           initial begin work(); $finish; end\n\
         endmodule\n";
    const WANT: &str = "A2 q=100 t=1\n\
                        any p=0 q=100 t=1\n\
                        none t=1\n\
                        A1 p=200 t=2\n\
                        late t=5\n\
                        end t=7\n";

    let (native, rj) = run_native(SRC);
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT);
    for b in ["vm", "interp"] {
        let (other, _) = run_on(b, SRC);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}
