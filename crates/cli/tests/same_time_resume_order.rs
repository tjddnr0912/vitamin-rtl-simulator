//! Same-time resume order (ROADMAP §2 start-order row 7): processes due at one time run in
//! the order their resumes were SCHEDULED, not in declaration order.
//!
//! The mechanism: every queued resume carries a scheduling-order key (`Ready::seq` /
//! `NativeReady::seq`, one counter on `Scheduler`) and both run loops sort by `(seq, tie)`. A
//! `#d` / `#0` resume takes its key when it is scheduled, so a wheel bucket and the Inactive
//! queue drain FIFO; a fork arm takes its own and runs right after the body that spawned it
//! (`Scheduler::spawned` / `NativeKernel::spawned`, spliced into the batch); the time-0
//! seeds share one key, and so does every process woken between two batch takes
//! (`Scheduler::wake_seq`, a continuous-assign hop and a `#0` landing included), so inside
//! such a group declaration order (`tie`) still decides; a `#0` resume promoted in the same
//! delta sorts after that delta's wakes.
//!
//! Oracles: iverilog 13 and verilator 5.052 print every line pinned below (cells `k*` of the
//! row-7 grounding). Recorded, not changed: the order WITHIN a wake group is an oracle split
//! (iverilog resumes waiters in reverse arm order, verilator runs coroutine waiters before
//! static `always` blocks); time-0 start order across the hierarchy is a split; a zero-delay
//! continuous-assign landing mixed with promoted `#0` resumes follows verilator (the promoted
//! resumes, then the landing's wakes). Every cell runs on all three backends.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_stro_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(&path);
    if let Some(b) = backend {
        cmd.arg("--backend").arg(b);
    }
    let out = cmd.output().expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
            && !l.contains("W-RUN-BACKEND-FALLBACK")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

/// Every backend must print the same thing; the default's text is returned.
fn run(src: &str) -> String {
    let (s, ok) = vita_on(src, None);
    assert!(ok, "expected exit 0, got:\n{s}");
    for b in ["interp", "vm", "native"] {
        let (t, ok) = vita_on(src, Some(b));
        assert!(ok, "backend {b}: expected exit 0, got:\n{t}");
        assert_eq!(t, s, "backend {b} diverges from the default on:\n{src}");
    }
    s
}

fn check(cells: &[(&str, &str)]) {
    for (src, want) in cells {
        assert_eq!(run(src), *want, "design:\n{src}");
    }
}

#[test]
fn same_time_delay_resumes_run_in_scheduling_order() {
    // Both oracles resume processes due at one time from the delay wheel in the order their
    // delays were SCHEDULED (FIFO); PRE resumed them in declaration order (k1a `A B`, k1b
    // `A B C`, k1c `A B C`, k1e `10 A | 10 B`, k1g `A B C D`).
    check(&[
        // k1a
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin #5; #5 $display("T %0t A", $time); end
  initial #10 $display("T %0t B", $time);
  initial #50 $finish;
endmodule
"#,
            "T 10 B\nT 10 A\n",
        ),
        // k1b
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin #2; #8 $display("T %0t A", $time); end
  initial begin #7; #3 $display("T %0t B", $time); end
  initial #10 $display("T %0t C", $time);
  initial #50 $finish;
endmodule
"#,
            "T 10 C\nT 10 A\nT 10 B\n",
        ),
        // k1c
        (
            r#"`timescale 1ns/1ns
module t;
  task automatic w(input int d, input byte s); #d $display("T %0t %c", $time, s); endtask
  initial begin #5; w(5, "A"); end
  initial w(10, "B");
  initial begin #3; w(7, "C"); end
  initial #50 $finish;
endmodule
"#,
            "T 10 B\nT 10 C\nT 10 A\n",
        ),
        // k1e
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin #5; #5; forever begin $display("T %0t A", $time); #10; end end
  initial begin #10; forever begin $display("T %0t B", $time); #1; #9; end end
  initial #45 $finish;
endmodule
"#,
            "T 10 B\nT 10 A\nT 20 A\nT 20 B\nT 30 A\nT 30 B\nT 40 A\nT 40 B\n",
        ),
        // k1g
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin #3; #7 $display("T %0t A", $time); end
  initial begin #6; #4 $display("T %0t B", $time); end
  initial begin #1; #9 $display("T %0t C", $time); end
  initial begin #10 $display("T %0t D", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 10 D\nT 10 C\nT 10 A\nT 10 B\n",
        ),
    ]);
}

#[test]
fn fork_arms_run_right_after_the_forking_body() {
    // A fork arm is a resume event like any delay (FIFO), and the arms a body spawns run right
    // after it yields, ahead of the rest of the batch: k1d `Y S Z X` (PRE `S X Y Z`), k5a
    // `Y Z X` (PRE `X Y Z`), k5e `Q P C1` (PRE `P Q C1`). Both oracles.
    check(&[
        // k1d
        (
            r#"`timescale 1ns/1ns
module t;
  initial fork
    begin #4; #6 $display("T %0t X", $time); end
    #10 $display("T %0t Y", $time);
    begin #1; #9 $display("T %0t Z", $time); end
  join
  initial #10 $display("T %0t S", $time);
  initial #50 $finish;
endmodule
"#,
            "T 10 Y\nT 10 S\nT 10 Z\nT 10 X\n",
        ),
        // k5a
        (
            r#"`timescale 1ns/1ns
module t;
  initial fork
    begin #2; #8 $display("T %0t X", $time); end
    #10 $display("T %0t Y", $time);
    begin #1; #9 $display("T %0t Z", $time); end
  join
  initial #50 $finish;
endmodule
"#,
            "T 10 Y\nT 10 Z\nT 10 X\n",
        ),
        // k5e
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin
    #5; fork #5 $display("T %0t C1", $time); join_none
    #5 $display("T %0t P", $time);
  end
  initial #10 $display("T %0t Q", $time);
  initial #50 $finish;
endmodule
"#,
            "T 10 Q\nT 10 P\nT 10 C1\n",
        ),
    ]);
}

#[test]
fn pending_delay_resumes_keep_fifo_ahead_of_the_wakes() {
    // A wake caused at time T runs behind every delay resume already due at T, and those keep
    // their scheduling order: k2f `C A B` (PRE `A C B`), k4g `C D A B` (PRE `A C D B`), k7c
    // `S D E W A` (PRE `D S E W A`). Both oracles.
    check(&[
        // k2f
        (
            r#"`timescale 1ns/1ns
module t;
  bit clk = 0;
  initial begin #4; #6 $display("T %0t A", $time); end
  initial @(posedge clk) $display("T %0t B", $time);
  initial #10 begin clk = 1; $display("T %0t C", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 10 C\nT 10 A\nT 10 B\n",
        ),
        // k4g
        (
            r#"`timescale 1ns/1ns
module t;
  bit f = 0;
  initial begin #4; #6 $display("T %0t A", $time); end
  initial wait (f) $display("T %0t B", $time);
  initial #10 begin f = 1; $display("T %0t C", $time); end
  initial #10 $display("T %0t D", $time);
  initial #50 $finish;
endmodule
"#,
            "T 10 C\nT 10 D\nT 10 A\nT 10 B\n",
        ),
        // k7c
        (
            r#"`timescale 1ns/1ns
module t;
  bit f = 0;
  initial begin #3; @(posedge f) $display("T %0t E", $time); end
  initial begin #2; wait (f) $display("T %0t W", $time); end
  initial begin #4; #6 $display("T %0t D", $time); end
  initial #10 begin f = 1; $display("T %0t S", $time); end
  initial begin #1; @(f) $display("T %0t A", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 10 S\nT 10 D\nT 10 E\nT 10 W\nT 10 A\n",
        ),
    ]);
}

#[test]
fn zero_delay_resumes_run_in_execution_order() {
    // `#0` resumes run in the order the `#0` statements executed (the Inactive queue is FIFO):
    // k6a `B C A` (PRE `A B C`), k6b `B A` (PRE `A B`), k6e `C A W` (PRE `W A C`), k6g `B A`
    // (PRE `A B`). Both oracles.
    check(&[
        // k6a
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin #2; #3; #0 $display("T %0t A", $time); end
  initial begin #5; #0 $display("T %0t B", $time); end
  initial begin #1; #4; #0 $display("T %0t C", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 5 B\nT 5 C\nT 5 A\n",
        ),
        // k6b
        (
            r#"`timescale 1ns/1ns
module t;
  initial begin #5; #5; #0 $display("T %0t A", $time); end
  initial begin #10; #0 $display("T %0t B", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 10 B\nT 10 A\n",
        ),
        // k6e
        (
            r#"`timescale 1ns/1ns
module t;
  bit clk = 0;
  initial @(posedge clk) begin #0 $display("T %0t W", $time); end
  initial begin #4; #6; #0 $display("T %0t A", $time); end
  initial #10 begin clk = 1; #0 $display("T %0t C", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 10 C\nT 10 A\nT 10 W\n",
        ),
        // k6g
        (
            r#"`timescale 1ns/1ns
module t;
  bit f = 0;
  initial begin #5; wait (f); #0 $display("T %0t A", $time); end
  initial begin #5; f = 1; #0 $display("T %0t B", $time); end
  initial #50 $finish;
endmodule
"#,
            "T 5 B\nT 5 A\n",
        ),
    ]);
}

#[test]
fn a_join_none_child_of_the_first_batch_runs_before_the_settles_wake() {
    // ROADMAP §2 "Delays / events" (closed by §4.5.541): the child spawned by `fork … join_none`
    // in the first Active batch runs right after its parent yields, before the process the
    // time-0 settle woke. Both oracles `I | I2 | F1 | W`; vita printed `I | I2 | W | F1`.
    let s = run(r#"module t; reg r = 0; wire w = r;
always @(w) $display("W");
initial begin $display("I"); fork $display("F1"); join_none r = 1; $display("I2"); end
initial #5 $finish; endmodule
"#);
    assert_eq!(s, "I\nI2\nF1\nW\n");
}
