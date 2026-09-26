//! A `$finish` ends the run at the END of its time step.
//!
//! Before this slice the `Step::Finish` arm of both kernels returned with the
//! Active queue unrun: the processes already woken in the step (a later `initial`
//! at the same tick, an `always` woken by a blocking write the finishing body just
//! made, a settle at time 0), the pending `#0` and NBA updates and everything they
//! woke were dropped. Measured on the pre-change binary: `initial begin #1 u = 7;
//! $finish; end` beside `always @(u) m++;` printed `m=0` in `final` where both
//! oracles print `m=1`; a clocked `cnt <= cnt + 1` missed the finish-coincident
//! edge (`cnt=2` for the oracles' `cnt=3`).
//!
//! Now the arm latches `finish_pending` and the loop's stable point ends the run:
//! every region of the step drains, the postponed region runs, time never
//! advances. `$stop` and `$fatal` keep their immediate arms.
//!
//! Every expected line is live iverilog 13 output; verilator 5.052 agrees except
//! where noted per test (it runs an `always @(v)` once more at time 0 on the
//! declaration initializer, so its counters read one higher, and it drops a woken
//! process whose body holds a timing control).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` and return (exit code, display lines) with diagnostics-free noise dropped.
fn run_with(src: &str, args: &[&str]) -> (Option<i32>, Vec<String>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_finish_drain_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let lines = text
        .lines()
        .filter(|l| {
            !(l.starts_with("warning[")
                || l.starts_with("note[")
                || l.starts_with("simulation ended")
                || l.starts_with("errors="))
        })
        .map(str::to_owned)
        .collect();
    (out.status.code(), lines)
}

/// All three backends must agree; the pinned lines are the oracle's.
fn check(src: &str, want: &[&str]) {
    for be in ["interp", "vm", "native"] {
        let (rc, got) = run_with(src, &["--backend", be]);
        assert_eq!(
            rc,
            Some(0),
            "backend {be}: full output:\n{}",
            got.join("\n")
        );
        assert_eq!(got, want, "backend {be}: full output:\n{}", got.join("\n"));
    }
}

#[test]
fn time0_always_woken_by_the_settle_runs_after_finish() {
    // ROADMAP cell: `I` / `A` in both oracles; the old arm printed `I` alone.
    check(
        "module top;
           wire w; assign w = 1'b1;
           always @(w) $display(\"A\");
           initial begin $display(\"I\"); $finish; end
         endmodule",
        &["I", "A"],
    );
}

#[test]
fn always_woken_by_the_finishing_body_runs() {
    check(
        "module top;
           int u = 0, m = 0;
           always @(u) m++;
           initial begin #1 u = 7; $display(\"U=%0d\", u); $finish; end
           final $display(\"m=%0d\", m);
         endmodule",
        &["U=7", "m=1"],
    );
}

#[test]
fn cascade_from_a_woken_process_runs() {
    // verilator: `V-wake` / `W-wake v=8` / `m=1 v=8` at time 1 (plus its time-0 run).
    // iverilog halts every other thread at its first system-task call once a
    // `$finish` is pending, so it prints `V-wake` and then `m=0 v=0` — whether
    // `v = u + 1` runs there depends on an unrelated `$display` before it (c19
    // below, the same cascade without a display, runs to `m=1` in iverilog).
    check(
        "module top;
           int u = 0, v = 0, m = 0;
           always @(u) begin $display(\"V-wake\"); v = u + 1; end
           always @(v) begin $display(\"W-wake v=%0d\", v); m++; end
           initial begin #1 u = 7; $finish; end
           final $display(\"m=%0d v=%0d\", m, v);
         endmodule",
        &["V-wake", "W-wake v=8", "m=1 v=8"],
    );
}

#[test]
fn pending_nba_applies_and_wakes() {
    check(
        "module top;
           int q = 0;
           always @(q) $display(\"Q=%0d\", q);
           initial begin #1 q <= 5; $display(\"before\"); $finish; end
           final $display(\"final q=%0d\", q);
         endmodule",
        &["before", "Q=5", "final q=5"],
    );
}

#[test]
fn nba_cascade_without_a_display_runs_in_both_oracles() {
    check(
        "module top;
           int q = 0, r = 0, m = 0;
           always @(q) begin r = q + 1; end
           always @(r) m++;
           initial begin #1 q <= 5; $finish; end
           final $display(\"q=%0d r=%0d m=%0d\", q, r, m);
         endmodule",
        &["q=5 r=6 m=1"],
    );
}

#[test]
fn zero_delay_fork_child_scheduled_before_finish_runs() {
    // iverilog `S` / `Z0` / `F`; verilator drops the `#0` child (`S` / `F`) — its
    // timing-control resumptions are not run once `gotFinish` is set. The `#0`
    // region is part of the time step, so vita drains it.
    check(
        "module top;
           initial begin #1 fork #0 $display(\"Z0\"); join_none $display(\"S\"); $finish; end
           final $display(\"F\");
         endmodule",
        &["S", "Z0", "F"],
    );
}

#[test]
fn later_initial_at_the_same_tick_runs_in_either_order() {
    check(
        "module top;
           initial #1 $finish;
           initial #1 $display(\"S\");
           final $display(\"F\");
         endmodule",
        &["S", "F"],
    );
    check(
        "module top;
           initial #1 $display(\"S\");
           initial #1 $finish;
           final $display(\"F\");
         endmodule",
        &["S", "F"],
    );
    // Time 0, both orders.
    check(
        "module top;
           initial $finish;
           initial $display(\"S\");
           final $display(\"F\");
         endmodule",
        &["S", "F"],
    );
    check(
        "module top;
           initial begin $display(\"A\"); $finish; end
           initial begin $display(\"B\"); #1 $display(\"C\"); end
           final $display(\"F\");
         endmodule",
        &["A", "B", "F"],
    );
}

#[test]
fn finish_through_a_task_enable_drains_too() {
    check(
        "module top;
           int u = 0, m = 0;
           task automatic t; $display(\"T\"); $finish; endtask
           always @(u) m++;
           initial begin #1 u = 3; t; end
           final $display(\"m=%0d\", m);
         endmodule",
        &["T", "m=1"],
    );
}

#[test]
fn clocked_counter_samples_the_finish_coincident_edge() {
    check(
        "module top;
           reg clk = 0; always #5 clk = ~clk;
           int cnt = 0;
           always @(posedge clk) cnt <= cnt + 1;
           always @(posedge clk) if (cnt == 2) $finish;
           final $display(\"cnt=%0d\", cnt);
         endmodule",
        &["cnt=3"],
    );
    check(
        "module top;
           reg clk = 0; always #5 clk = ~clk;
           int cnt = 0;
           always @(posedge clk) if (cnt == 2) begin $display(\"FIN cnt=%0d\", cnt); $finish; end
           always @(posedge clk) begin $display(\"INC cnt=%0d\", cnt); cnt <= cnt + 1; end
           final $display(\"cnt=%0d\", cnt);
         endmodule",
        &["INC cnt=0", "INC cnt=1", "FIN cnt=2", "INC cnt=2", "cnt=3"],
    );
    check(
        "module top;
           reg clk = 0; always #5 clk = ~clk;
           int a = 0, b = 0;
           always @(posedge clk) begin a <= a + 1; b <= a; end
           initial begin #15 $finish; end
           final $display(\"a=%0d b=%0d\", a, b);
         endmodule",
        &["a=2 b=1"],
    );
}

#[test]
fn strobe_and_monitor_from_a_woken_process_flush() {
    // verilator `D u=4` / `S u=4`; iverilog halts the thread after `D u=4` (its
    // first system call) so its `$strobe` is never queued.
    check(
        "module top;
           int u = 0;
           always @(u) begin $display(\"D u=%0d\", u); $strobe(\"S u=%0d\", u); end
           initial begin #1 u = 4; $finish; end
         endmodule",
        &["D u=4", "S u=4"],
    );
    check(
        "module top;
           int u = 0, m = 0;
           always @(u) begin m = u * 3; $monitor(\"MON m=%0d\", m); end
           initial begin #1 u = 2; $finish; end
         endmodule",
        &["MON m=6"],
    );
}

#[test]
fn woken_process_suspending_on_a_delay_does_not_run_past_the_step() {
    // iverilog `W1` / `F`; verilator prints `F` alone (drops the timing process).
    check(
        "module top;
           int u = 0;
           always @(u) begin $display(\"W1\"); #1 $display(\"W2\"); end
           initial begin #1 u = 4; $finish; end
           final $display(\"F\");
         endmodule",
        &["W1", "F"],
    );
}

#[test]
fn always_comb_and_cont_assign_chain_reach_their_readers() {
    check(
        "module top;
           int u = 0, y;
           always_comb begin y = u * 2; end
           initial begin #1 u = 4; $finish; end
           final $display(\"y=%0d\", y);
         endmodule",
        &["y=8"],
    );
    check(
        "module top;
           int u = 0; wire [31:0] y = u + 1;
           always @(y) $display(\"Y=%0d\", y);
           initial begin #1 u = 4; $finish; end
           final $display(\"y=%0d\", y);
         endmodule",
        &["Y=1", "Y=5", "y=5"],
    );
}

#[test]
fn finish_in_a_fork_arm_and_an_event_woken_initial() {
    check(
        "module top;
           int u = 0, m = 0;
           always @(u) m++;
           initial begin #1 fork begin u = 1; $finish; end $display(\"ARM2\"); join end
           final $display(\"m=%0d\", m);
         endmodule",
        &["ARM2", "m=1"],
    );
    check(
        "module top;
           event e; int m = 0;
           initial begin @(e); m = 1; $display(\"E\"); end
           initial begin #1 -> e; $finish; end
           final $display(\"m=%0d\", m);
         endmodule",
        &["E", "m=1"],
    );
}

#[test]
fn a_second_finish_in_the_drain_is_absorbed_and_the_statement_after_finish_does_not_run() {
    // iverilog `A m=1` / `m=1`; verilator additionally prints `A-after` (it runs
    // the statement after a reached `$finish`; vita and iverilog stop the body).
    check(
        "module top;
           int u = 0, m = 0;
           always @(u) begin m++; $display(\"A m=%0d\", m); $finish; $display(\"A-after\"); end
           initial begin #1 u = 7; $finish; $display(\"I-after\"); end
           final $display(\"m=%0d\", m);
         endmodule",
        &["A m=1", "m=1"],
    );
    check(
        "module top;
           int u = 0, m = 0;
           always @(u) m++;
           initial begin #1 $finish; u = 9; end
           final $display(\"m=%0d u=%0d\", m, u);
         endmodule",
        &["m=0 u=0"],
    );
}

#[test]
fn process_order_around_the_finishing_one() {
    check(
        "module top;
           int u = 0;
           initial begin #1 u = 1; $display(\"P1 done\"); $finish; end
           initial begin #1 $display(\"P2 at 1\"); end
           always @(u) $display(\"W\");
           final $display(\"F\");
         endmodule",
        &["P1 done", "P2 at 1", "W", "F"],
    );
}

#[test]
fn nba_at_time0_and_zero_delay_continuation_of_another_initial() {
    check(
        "module top;
           int q = 0;
           always @(q) $display(\"Q=%0d\", q);
           initial begin q <= 1; $finish; end
           final $display(\"final q=%0d\", q);
         endmodule",
        &["Q=1", "final q=1"],
    );
    // iverilog `after0` / `F`; verilator `F` (drops the `#0` resumption).
    check(
        "module top;
           int u = 0;
           initial begin #1 u = 1; #0 $display(\"after0\"); end
           initial #1 $finish;
           final $display(\"F\");
         endmodule",
        &["after0", "F"],
    );
}

#[test]
fn fatal_reached_in_the_drain_ends_the_run_as_an_error() {
    // iverilog: FATAL line, `F`, rc 1. verilator aborts with rc 1.
    for be in ["interp", "vm", "native"] {
        let (rc, got) = run_with(
            "module top;
               int u = 0;
               always @(u) $fatal(1, \"late fatal u=%0d\", u);
               initial begin #1 u = 1; $finish; end
               final $display(\"F\");
             endmodule",
            &["--backend", be],
        );
        assert_eq!(rc, Some(1), "backend {be}: {}", got.join("\n"));
        assert!(
            got.iter()
                .any(|l| l.contains("F-RUN-FATAL: late fatal u=1")),
            "backend {be}: {}",
            got.join("\n")
        );
        assert!(
            got.iter().any(|l| l == "F"),
            "backend {be}: {}",
            got.join("\n")
        );
    }
}

#[test]
fn fatal_itself_keeps_the_immediate_arm() {
    // `$fatal` is an error: the process woken by `u = 7` does not run (rc 1, `m=0`).
    // iverilog runs it (`A`, `m=1`); this is the pre-existing vita choice that no
    // output follows a `$fatal`, unchanged by this slice.
    for be in ["interp", "vm", "native"] {
        let (rc, got) = run_with(
            "module top;
               int u = 0, m = 0;
               always @(u) begin m++; $display(\"A\"); end
               initial begin #1 u = 7; $fatal(1, \"boom\"); end
               final $display(\"m=%0d\", m);
             endmodule",
            &["--backend", be],
        );
        assert_eq!(rc, Some(1), "backend {be}: {}", got.join("\n"));
        assert!(
            got.iter().any(|l| l == "m=0"),
            "backend {be}: {}",
            got.join("\n")
        );
        assert!(
            !got.iter().any(|l| l == "A"),
            "backend {be}: {}",
            got.join("\n")
        );
    }
}

#[test]
fn a_process_that_reached_finish_is_not_re_entered_in_the_same_step() {
    // review round 1 F1: iverilog ends the thread at `$finish` (`n=1 m=0`); the
    // first POST re-ran the body on the NBA edge of `b` (`n=2`). verilator runs
    // the statements after `$finish` and re-enters too (`n=2 m=2`) — the
    // statement-after-`$finish` split, recorded.
    check(
        "module top;
           reg a = 0, b = 0;
           integer n = 0, m = 0;
           always @(posedge a or posedge b) begin n = n + 1; $finish; m = m + 1; end
           initial begin #1 a = 1; b <= 1; end
           final $display(\"final n=%0d m=%0d\", n, m);
           initial #10 $finish;
         endmodule",
        &["final n=1 m=0"],
    );
    check(
        "module top;
           reg clk = 0, rst = 0;
           integer n = 0, q = 0;
           always @(posedge clk or posedge rst) begin
             n = n + 1;
             if (rst) q <= 0; else q <= q + 1;
             if (q == 2) $finish;
           end
           always @(posedge clk) if (q == 2) rst <= 1;
           always #5 clk = ~clk;
           final $display(\"final n=%0d q=%0d rst=%0d t=%0t\", n, q, rst, $time);
         endmodule",
        &["final n=3 q=3 rst=1 t=25"],
    );
}

#[test]
fn a_zero_delay_cont_assign_due_in_the_finish_step_is_delivered() {
    // review round 1 differential F1(b): a `#0` cont-assign / gate update due in
    // the finishing step is delivered before the finish takes effect (it is an
    // Inactive-region event of the step, `zero_delay_cont_assign.rs`; it used to
    // land on the advance path that re-entered the tick), where the first drain
    // skipped it (`r=0`); iverilog `r=7` / `r=1` / `r=8`. verilator is not an
    // oracle here (it drops `#0` work after `$finish`).
    check(
        "module top;
           bit [7:0] u; int m;
           wire [7:0] r; assign #0 r = u;
           initial #5 begin u = 7; $finish; end
           final $display(\"F u=%0d r=%0d m=%0d\", u, r, m);
         endmodule",
        &["F u=7 r=7 m=0"],
    );
    check(
        "module top;
           bit [7:0] u; int m;
           wire r; buf #0 g(r, u[0]);
           initial #5 begin u = 7; $finish; end
           final $display(\"F u=%0d r=%0d m=%0d\", u, r, m);
         endmodule",
        &["F u=7 r=1 m=0"],
    );
    check(
        "module top;
           bit [7:0] u; int m;
           wire [7:0] r; wire [7:0] r0; assign #0 r0 = u; assign r = r0 + 1;
           initial #5 begin u = 7; $finish; end
           final $display(\"F u=%0d r=%0d m=%0d\", u, r, m);
         endmodule",
        &["F u=7 r=8 m=0"],
    );
}
