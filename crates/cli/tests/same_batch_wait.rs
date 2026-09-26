//! A LEVEL wait sees only the changes made AFTER it armed (ROADMAP §2 "Delays / events", "a
//! wait ARMED in an Active batch sees the writes made EARLIER in that batch" — the level half).
//! The mechanism: `SimState::stamp_change` / `DirtyChannel::stamp_change` (a CHANGE SEQUENCE
//! stamped on every value change, one cell shared by the two stores; a heap change takes its
//! number when it is made, `note_dyn_change`) and the `arm_seq` a level waiter records when it
//! arms (`Scheduler::suspend_on`, `arm_sensitivity`; `NativeKernel::k_suspend_on`,
//! `WakeTable::rearm_level`).
//!
//! `propagate_changes` / `WakeTable::wake` ran after a WHOLE Active batch and matched a static
//! level waiter against the batch's dirt, so one that ran in the batch and re-armed fired again
//! on a write an earlier process of the same batch had made (`always @(a) s = 0;` beside
//! `always @(s) …`, `#1 a = 1; s = 1;` — two lines; both oracles one). An in-body `@(sig)` /
//! `@*` compared the net with an arm-time VALUE snapshot: blind to a glitch back to that value
//! (`a = 1; a = 0;` after the arm — iverilog wakes it) and to heap content (`q.push_back(1)`
//! after an `@* n = q.size();` never woke it — iverilog `W 1 n=1`). Now a level waiter, static
//! or in-body, fires on a watched net whose last change was stamped after its arm. A static
//! waiter armed at time 0 carries 0, so the time-0 settle's changes reach it (§4.5.535
//! delivery unchanged); the initializers' nets are reset to 0 by the rollback (an initializer
//! is no event, IEEE §6.21).
//!
//! NOT changed — the EDGE half, recorded with its prerequisite (ROADMAP §2): an in-body
//! `@(posedge x)` still fires from the slot's accumulated mask, so an edge made earlier in the
//! same batch wakes it. `initial #5 r = 1;` declared before `initial begin #5 @(posedge r);
//! $display("late"); end` prints `late 5` and `reg clk; initial clk = 1;` before `initial begin
//! @(posedge clk); … end` prints `saw 0` (both oracles nothing). The after-the-arm rule was
//! built and reviewed for it and REVERTED: both oracles resume the processes due at one time
//! in the order their delays were SCHEDULED, vita in declaration order, and under the rule
//! the common testbench (a clock generator declared first, stimulus resuming from `#15` and
//! arming `@(posedge clk)` in the edge's own batch) shifted by a whole cycle — `R 25 rst=0`
//! where both oracles and PRE print `R 15 rst=0`. The edge half waits for the same-time
//! resume order. Every such cell below is pinned at its PRE value and marked.
//!
//! Every expected text below is printed by both oracles, except where marked:
//! - The reverse declaration order (`@(posedge clk)` armed BEFORE `initial clk = 1;` runs) is
//!   a §4.7 race: iverilog prints `saw 0`, verilator nothing; vita prints it (PRE = POST).
//! - A glitch `a = 1; a = 0;` after an in-body `@(a)` armed in the same batch: iverilog wakes it
//!   at 1 (`IB 1 a=0 m=1`), verilator holds no glitch (2-state) and wakes it at 2. vita now wakes
//!   it at 1 (PRE: at 2 — the arm-time VALUE compare, ROADMAP §4.5.4's recorded miss).
//! - A write made in the batch AFTER a static waiter ran and re-armed IS its event in vita
//!   (`always @(b) …` declared before `always @(a) b = 0;`, both woken by one write of `a` and
//!   `b`: two lines, the second reading `b=0`). iverilog runs `@(a)` first and prints one line
//!   with `b=0`; verilator one line with `b=1`. Order-dependent (§4.7), PRE = POST.
//! - The edge cells marked EDGE-HALF: vita's PRE value, both oracles nothing / a later time.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_sbw_{}_{n}.sv", std::process::id()));
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

fn run(src: &str) -> String {
    let (s, ok) = vita_on(src, None);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn check(cells: &[(&str, &str)]) {
    for (src, want) in cells {
        assert_eq!(run(src), *want, "design:\n{src}");
    }
}

const LATE: &str = "module t; reg r = 0;
initial #5 r = 1;
initial begin #5 @(posedge r); $display(\"late %0t\", $time); end
initial #9 $finish; endmodule
";

const SAW: &str = "module t; reg clk; initial clk = 1;
initial begin @(posedge clk); $display(\"saw %0t\", $time); end
initial #3 $finish; endmodule
";

/// The row's third shape: `@(s)` is woken by the batch's write of `s`, runs after `@(a)`
/// has already written `s = 0` in the same batch, re-arms, and must not fire on that write.
const REARM: &str = "module t; reg a = 0, s = 0; int n = 0;
always @(a) s = 0;
always @(s) begin n++; $display(\"S %0t s=%b n=%0d\", $time, s, n); end
initial begin #1 a = 1; s = 1; #1 $finish; end endmodule
";

#[test]
fn a_level_waiter_armed_after_a_write_in_the_same_batch_does_not_see_it() {
    // PRE: `S 1 s=0 n=1\nS 1 s=0 n=2` on the re-armed static waiter. EDGE-HALF: `late 5` and
    // `saw 0` are PRE = POST (both oracles nothing).
    check(&[
        (LATE, "late 5\n"),
        (SAW, "saw 0\n"),
        (REARM, "S 1 s=0 n=1\n"),
        // The same at time 0 and through nonblocking writes of `a` and `s`.
        (
            "module t; reg a = 0, s = 0; int n = 0;
always @(a) s = 0;
always @(s) begin n++; $display(\"S %0t s=%b n=%0d\", $time, s, n); end
initial begin a = 1; s = 1; end
initial #2 $finish; endmodule
",
            "S 0 s=0 n=1\n",
        ),
        (
            "module t; reg a = 0, s = 0; int n = 0;
always @(a) s = 0;
always @(s) begin n++; $display(\"S %0t s=%b n=%0d\", $time, s, n); end
initial begin #1 a <= 1; s <= 1; #1 $finish; end endmodule
",
            "S 1 s=0 n=1\n",
        ),
        // An in-body level wait armed after the write (PRE = POST: the arm-time snapshot).
        (
            "module t; reg s = 0;
initial #1 s = 1;
initial begin #1 @(s); $display(\"L %0t s=%b\", $time, s); end
initial #3 $finish; endmodule
",
            "",
        ),
    ]);
}

#[test]
fn an_edge_wait_keeps_the_slots_accumulated_mask() {
    // EDGE-HALF. Four processes in one batch: `clk = 1`, arm `@(posedge clk)`, `clk = 0`, arm
    // `@(negedge clk)`: both oracles nothing; vita `P 1` and `N 1` from the slot's mask (PRE =
    // POST).
    check(&[
        (
            "module t; reg clk = 0;
initial begin #1 clk = 1; end
initial begin #1 @(posedge clk); $display(\"P %0t\", $time); end
initial begin #1 clk = 0; end
initial begin #1 @(negedge clk); $display(\"N %0t\", $time); end
initial #4 $finish; endmodule
",
            "P 1\nN 1\n",
        ),
        // A `#0` write after the arm is a new batch: the negedge fires.
        (
            "module t; reg clk = 0;
initial begin #1 clk = 1; @(posedge clk); $display(\"P %0t\", $time); end
initial begin #1 #0 clk = 0; end
initial begin #1 @(negedge clk); $display(\"N %0t\", $time); end
initial #4 $finish; endmodule
",
            "N 1\n",
        ),
        // A write by a LATER process of the batch, and an NBA landing after the arm, fire.
        (
            "module t; reg r = 0, s = 0;
initial begin #1 @(posedge r); $display(\"P %0t\", $time); end
initial begin #1 @(s); $display(\"L %0t s=%b\", $time, s); end
initial #1 begin r = 1; s = 1; end
initial #3 $finish; endmodule
",
            "P 1\nL 1 s=1\n",
        ),
        (
            "module t; reg r = 0;
initial begin #1 r <= 1; end
initial begin #1 @(posedge r); $display(\"NBA %0t\", $time); end
initial begin #2 r = 0; #0 r = 1; end
initial begin #2 @(posedge r); $display(\"Z %0t\", $time); end
initial #4 $finish; endmodule
",
            "NBA 1\nZ 2\n",
        ),
        // A wait inside a task and a forked wait armed after the write: EDGE-HALF, `T 5`
        // (both oracles nothing; the forked wait's `F` is not printed in any tool).
        (
            "module t; reg r = 0;
task automatic w; @(posedge r); $display(\"T %0t\", $time); endtask
initial #5 r = 1;
initial begin #5 w(); end
initial begin #5 fork @(posedge r); join $display(\"F %0t\", $time); end
initial #9 $finish; endmodule
",
            "T 5\n",
        ),
        // The waiter's own write before its arm (the self-retrigger guard, PRE = POST).
        (
            "module t; reg r = 0; reg s = 0;
initial begin #1 r = 1; @(posedge r); $display(\"SELF %0t\", $time); end
initial begin #1 s = 1; @(s); $display(\"SELFL %0t\", $time); end
initial #4 $finish; endmodule
",
            "",
        ),
    ]);
}

#[test]
fn a_glitch_after_the_arm_still_wakes_the_waiter() {
    // `a = 1; a = 0;` in one statement list after the in-body `@(a)` armed: iverilog wakes
    // it at 1 (marked in the module doc; verilator at 2). The static `always @(a)` on the
    // same glitch fires once at 1 in all three tools, and again at 2.
    check(&[(
        "module t; reg a = 0; int n = 0, m = 0;
always @(a) begin n++; $display(\"A %0t a=%b n=%0d\", $time, a, n); end
initial begin #1 @(a); m++; $display(\"IB %0t a=%b m=%0d\", $time, a, m); end
initial begin #1 a = 1; a = 0; #1 a = 1; #1 $finish; end endmodule
",
        "A 1 a=0 n=1\nIB 1 a=0 m=1\nA 2 a=1 n=2\n",
    )]);
    // A static edge process on a same-batch pulse and on its own re-pulse: once (PRE = POST).
    check(&[
        (
            "module t; reg clk = 0; int n = 0;
always @(posedge clk) begin n++; $display(\"P %0t n=%0d\", $time, n); end
initial begin #1 clk = 1; clk = 0; clk = 1; #1 $finish; end endmodule
",
            "P 1 n=1\n",
        ),
        (
            "module t; reg clk = 0; int n = 0;
always @(posedge clk) begin n++; $display(\"P %0t n=%0d\", $time, n); clk = 0; clk = 1; end
initial begin #1 clk = 1; #2 $finish; end endmodule
",
            "P 1 n=1\n",
        ),
    ]);
}

#[test]
fn the_time_zero_rules_and_the_races_are_unchanged() {
    // The settle's changes reach a static waiter armed at 0; an initializer is no event; an
    // `always @*` re-arms after its own run (PRE = POST on every line).
    check(&[
        (
            "module t; reg r = 1; wire w = ~r; int n = 0;
always @(w) begin n++; $display(\"W %0t w=%b n=%0d\", $time, w, n); end
initial begin r = 0; end
initial #2 $finish; endmodule
",
            "W 0 w=1 n=1\n",
        ),
        (
            "module t; wire w = 1'b0; reg s = 0; int n = 0;
always @(w) s = 1;
always @(s) begin n++; $display(\"S %0t s=%b n=%0d\", $time, s, n); end
initial s = 0;
initial #2 $finish; endmodule
",
            "S 0 s=1 n=1\n",
        ),
        (
            "module t; reg a = 0, b = 0; reg y; int n;
always @* begin y = a ^ b; n++; end
always @(a) b = ~b;
initial begin #1 a = 1; #1 $display(\"Y %0t y=%b n=%0d\", $time, y, n); #1 $finish; end endmodule
",
            "Y 2 y=0 n=2\n",
        ),
        // The races, marked in the module doc: vita's values, PRE = POST.
        (
            "module t; reg clk;
initial begin @(posedge clk); $display(\"saw %0t\", $time); end
initial clk = 1;
initial #3 $finish; endmodule
",
            "saw 0\n",
        ),
        (
            "module t; reg a = 0, b = 0; int n = 0;
always @(b) begin n++; $display(\"B %0t b=%b n=%0d\", $time, b, n); end
initial begin #1 a = 1; b = 1; end
always @(a) b = 0;
initial #4 $finish; endmodule
",
            "B 1 b=1 n=1\nB 1 b=0 n=2\n",
        ),
    ]);
}

#[test]
fn every_backend_arms_the_same_way() {
    for src in [LATE, SAW, REARM] {
        let mut outs = Vec::new();
        for be in ["interp", "vm", "native"] {
            let (s, ok) = vita_on(src, Some(be));
            assert!(ok, "{be}: expected exit 0, got:\n{s}");
            outs.push(s);
        }
        assert_eq!(outs[0], outs[1], "interp vs vm on:\n{src}");
        assert_eq!(outs[0], outs[2], "interp vs native on:\n{src}");
    }
}

#[test]
fn a_heap_change_is_stamped_when_it_is_made_not_when_the_batch_drains() {
    // A queue / string / dynamic-array change is staged by `note_dyn_change` and drained at
    // the sweep; the sequence is taken at the staging, so an `@*` reading the heap net that
    // arms AFTER the push in the same batch does not see it (iverilog; round-1 review cell:
    // `W 1 n=1`), one that arms BEFORE does (PRE printed nothing on it), a change three
    // time units later reaches a wait that saw nothing at 1 (PRE nothing), and a static
    // `always_comb` that ran after the push made earlier in its batch does not re-run.
    check(&[
        (
            "module t; int q[$]; int n;
initial begin #1 q.push_back(1); end
initial begin #1 @* n = q.size(); $display(\"W %0t n=%0d\", $time, n); end
initial #10 $finish;
endmodule
",
            "",
        ),
        (
            "module t; int q[$]; int n;
initial begin #1 @* n = q.size(); $display(\"W %0t n=%0d\", $time, n); end
initial begin #1 q.push_back(1); end
initial #10 $finish;
endmodule
",
            "W 1 n=1\n",
        ),
        (
            "module t; int q[$]; int n; string s; int m;
initial q.push_back(1);
initial begin @* n = q.size(); $display(\"Q %0t n=%0d\", $time, n); end
initial begin #1 s = \"ab\"; end
initial begin #1 @* m = s.len(); $display(\"S %0t m=%0d\", $time, m); end
initial #10 $finish;
endmodule
",
            "",
        ),
        (
            "module t; int d[] = new[2]; int n;
initial begin #1 d[0] = 5; end
initial begin #1 @* n = d[0]; $display(\"D %0t n=%0d\", $time, n); end
initial begin #3 d[0] = 7; end
initial #10 $finish;
endmodule
",
            "D 3 n=7\n",
        ),
        (
            "module t; string s; int m; int q[$]; int n;
initial begin #1 @* m = s.len(); $display(\"S %0t m=%0d\", $time, m); end
initial begin #1 s = \"ab\"; end
initial begin #1 q.push_back(1); end
initial begin #1 @* n = q.size(); $display(\"Q %0t n=%0d\", $time, n); end
initial begin #3 q.push_back(2); end
initial #10 $finish;
endmodule
",
            "S 1 m=2\nQ 3 n=2\n",
        ),
        (
            "module t; int q[$]; reg b = 0; int n; int k;
always @(b) q.push_back(7);
always_comb begin n = q.size() + b; k = k + 1; $display(\"C %0t n=%0d\", $time, n); end
initial #1 b = 1;
initial #10 $finish;
endmodule
",
            "C 0 n=0\nC 1 n=2\n",
        ),
    ]);
}

#[test]
fn a_clocking_event_wait_armed_in_the_edges_batch_still_catches_it() {
    // verilator: `CB 1 d=0`, `CB 5 n=1 d=0 …` — the slot's mask (PRE = POST); an after-the-arm
    // rule printed `CB 3 d=1` / `CB 15 n=1 d=1 …` in round 1 of the review (IEEE §14.13
    // delivers the clocking event in the Observed region of the edge's step).
    check(&[
        (
            "module t; reg clk = 0; reg d = 0;
clocking cb @(posedge clk); input d; endclocking
initial begin #1 clk = 1; end
initial begin #1 @(cb); $display(\"CB %0t d=%b\", $time, cb.d); end
initial begin #1 d = 1; #1 clk = 0; #1 clk = 1; #1 clk = 0; #1 clk = 1; #2 $finish; end endmodule
",
            "CB 1 d=0\n",
        ),
        (
            "module t; reg clk = 0; reg d = 0; int n = 0;
clocking cb @(posedge clk); input d; endclocking
always #5 clk = ~clk;
initial begin #5; repeat (3) begin @(cb); n++; $display(\"CB %0t n=%0d d=%b\", $time, n, cb.d); end #1 $finish; end
initial begin #12 d = 1; #60 $finish; end endmodule
",
            "CB 5 n=1 d=0\nCB 15 n=2 d=1\nCB 25 n=3 d=1\n",
        ),
        // The same position without a clocking block: EDGE-HALF, `PE 1 d=1` (both oracles
        // `PE 3 d=1`).
        (
            "module t; reg clk = 0; reg d = 0;
initial begin #1 clk = 1; end
initial begin #1 @(posedge clk); $display(\"PE %0t d=%b\", $time, d); end
initial begin #1 d = 1; #1 clk = 0; #1 clk = 1; #1 clk = 0; #1 clk = 1; #2 $finish; end endmodule
",
            "PE 1 d=1\n",
        ),
    ]);
}

#[test]
fn the_edge_half_stays_at_its_pre_value_on_the_review_cells() {
    // EDGE-HALF, PRE = POST: a write to another bit after the arm (`P 1 w0=0`; both oracles
    // nothing), two bit-0 transitions after the arm (`P 1 c=2`; both oracles `P 5 c=3`), and
    // the 4-state `1'bx; 0` after a pre-arm posedge (`P 1 clk=0`; iverilog nothing).
    check(&[
        (
            "module t; reg [127:0] w = 0;
initial begin #1 w = 1; end
initial begin #1 @(posedge w); $display(\"P %0t w0=%b\", $time, w[0]); end
initial begin #1 w[100] = 1; w[0] = 0; end
initial #10 $finish;
endmodule
",
            "P 1 w0=0\n",
        ),
        (
            "module t; reg [1:0] c = 0;
initial begin #1 c = 1; end
initial begin #1 @(posedge c); $display(\"P %0t c=%0d\", $time, c); end
initial begin #1 c = 3; c = 2; #4 c = 3; #1 $finish; end endmodule
",
            "P 1 c=2\n",
        ),
        (
            "module t; reg clk = 0;
initial begin #1 clk = 1; end
initial begin #1 @(posedge clk) $display(\"P %0t clk=%b\", $time, clk); end
initial begin #1 clk = 1'bx; clk = 0; end
initial #10 $finish;
endmodule
",
            "P 1 clk=0\n",
        ),
    ]);
}

#[test]
fn the_common_testbench_shape_and_the_same_time_resume_order_are_unchanged() {
    // Round-2 review B2: a clock generator declared FIRST and stimulus resuming from `#15` in
    // the same step, arming `@(posedge clk)` in the edge's own batch. Both oracles resume the
    // stimulus first (its delay was scheduled first) and the wait catches the edge; vita
    // resumes the clock first and the slot's mask catches it — `R 15 rst=0 | N 20 | R2 25 |
    // n=5` in all three tools (an after-the-arm edge rule printed `R 25 … n=4`). The mechanism
    // cell pins vita's declaration order (both oracles `B 10` before `A 10`; a §4.7 order the
    // recorded prerequisite names), and the negedge twin its PRE value (`N 10 clk=0`, both
    // oracles the same).
    check(&[
        (
            "module t; reg clk; reg rst; int n = 0;
initial begin clk = 0; forever #5 clk = ~clk; end
initial begin rst = 1; #15; @(posedge clk); rst = 0; $display(\"R %0t rst=%b\", $time, rst); @(posedge clk); $display(\"R2 %0t\", $time); end
initial begin #20 @(negedge clk); $display(\"N %0t\", $time); end
always @(posedge clk) if (!rst) n <= n + 1;
initial begin #60 $display(\"n=%0d\", n); $finish; end endmodule
",
            "R 15 rst=0\nN 20\nR2 25\nn=5\n",
        ),
        (
            "module t;
initial begin #5; #5 $display(\"A %0t\", $time); end
initial begin #10 $display(\"B %0t\", $time); end
initial #30 $finish; endmodule
",
            "A 10\nB 10\n",
        ),
        (
            "module t; reg clk = 0;
initial begin #5 clk = 1; #5 clk = 0; #5 clk = 1; end
initial begin #10 @(negedge clk); $display(\"N %0t clk=%b\", $time, clk); end
initial #30 $finish; endmodule
",
            "N 10 clk=0\n",
        ),
    ]);
}
