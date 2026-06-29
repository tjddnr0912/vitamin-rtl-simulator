//! Inertial delay on continuous assigns (Phase-1.x precision item ④).
//!
//! v1 shipped `assign #d` as a TRANSPORT delay (every RHS change delivered
//! after d). IEEE 1364-2005 §6.1.3 / gate-delay semantics make it INERTIAL:
//! a re-change of the RHS before a pending update delivers CANCELS that
//! pending update, so pulses narrower than the delay never appear on the LHS.
//!
//! iverilog 13.0 pins (live, 2026-06-12):
//!   - 2ns pulse through `assign #5` → filtered (LHS never toggles)
//!   - EXACTLY-5ns pulse → survives (the pending write delivers at the tick
//!     boundary BEFORE the new change re-arms) — matches our drain order
//!     (delayed writes apply at tick start, then processes run).
//!
//! (The t0 undriven surface differs — iverilog shows `x`, vita `z` — which is
//! a pre-existing init-value surface, so assertions start at the first edge.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ic_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn short_pulse_is_filtered() {
    // iverilog: y shows ONLY x→0@5; the 2ns high pulse never lands.
    let (out, err, code) = run("`timescale 1ns/1ns\n\
         module top;\n\
         reg a; wire y;\n\
         assign #5 y = a;\n\
         initial begin\n\
           $monitor(\"%0t y=%b\", $time, y);\n\
           a = 0;\n\
           #10 a = 1;\n\
           #2  a = 0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("5 y=0"), "got:\n{out}");
    assert!(
        !out.contains("y=1"),
        "a sub-delay pulse must be inertially filtered:\n{out}"
    );
}

#[test]
fn boundary_pulse_survives() {
    // Pulse width EXACTLY the delay: the pending write delivers at the tick
    // start before the reverting change re-arms (iverilog: 15 y=1, 20 y=0).
    let (out, err, code) = run("`timescale 1ns/1ns\n\
         module top;\n\
         reg a; wire y;\n\
         assign #5 y = a;\n\
         initial begin\n\
           $monitor(\"%0t y=%b\", $time, y);\n\
           a = 0;\n\
           #10 a = 1;\n\
           #5  a = 0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("15 y=1"), "got:\n{out}");
    assert!(out.contains("20 y=0"), "got:\n{out}");
}

#[test]
fn rapid_toggles_only_last_survives() {
    // a: 0 →1@10 →0@11 →1@12 — only the LAST change delivers (y=1@17),
    // with no intermediate glitches.
    let (out, err, code) = run("`timescale 1ns/1ns\n\
         module top;\n\
         reg a; wire y;\n\
         assign #5 y = a;\n\
         initial begin\n\
           $monitor(\"%0t y=%b\", $time, y);\n\
           a = 0;\n\
           #10 a = 1;\n\
           #1  a = 0;\n\
           #1  a = 1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("17 y=1"), "got:\n{out}");
    assert!(
        !out.contains("15 y="),
        "the superseded first change must not deliver at 15:\n{out}"
    );
    assert!(
        !out.contains("16 y="),
        "the superseded second change must not deliver at 16:\n{out}"
    );
}

#[test]
fn steady_value_still_delivers_after_delay() {
    // Regression: plain arming (no pulses) keeps the one-shot delivery.
    let (out, err, code) = run("`timescale 1ns/1ns\n\
         module top;\n\
         reg a; wire y;\n\
         assign #3 y = a;\n\
         initial begin\n\
           $monitor(\"%0t y=%b\", $time, y);\n\
           a = 1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("3 y=1"), "got:\n{out}");
}

#[test]
fn independent_assigns_do_not_cancel_each_other() {
    // Two delayed assigns share the wheel — generation bookkeeping must be
    // PER ASSIGN, not global.
    let (out, err, code) = run("`timescale 1ns/1ns\n\
         module top;\n\
         reg a, b; wire y, z;\n\
         assign #5 y = a;\n\
         assign #5 z = b;\n\
         initial begin\n\
           a = 0; b = 0;\n\
           #10 a = 1;\n\
           #1  b = 1;\n\
           #2  a = 0;\n\
           #20 $finish;\n\
         end\n\
         initial begin\n\
           #17 $display(\"y=%b z=%b\", y, z);\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // a's 3ns pulse is filtered (y=0 at 17); b's steady 1 delivered at 16.
    assert!(out.contains("y=0 z=1"), "got:\n{out}");
}
