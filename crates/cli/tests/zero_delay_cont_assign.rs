//! A zero-delay continuous-assign / gate update is an Inactive-region event of its time step
//! (ROADMAP §2 "Delays / events": the `#0` delivery bullet, the runtime delay that evaluates to
//! zero, and the zero-rise sidecar trade — one root, `Scheduler::delayed_ca`).
//!
//! The mechanism: a delayed write due at `now` used to land only on the ADVANCE path, after the
//! Postponed region, when the loop re-entered the same tick — so a `#0` cascade in the process
//! that saw the rhs change never read it (`h1 r=0` for iverilog `r=7` from `h0` and verilator
//! from `h1`), `$strobe` printed the old value, and a runtime `#(dz)` with `dz = 0`, a
//! `#(0, F)` rise, `wire #0` and `buf #0` all lagged the same way. Both run loops now deliver
//! the writes due at `now` at the `#0` promotion (`sched/run_loop.rs`, `native/run.rs`): before
//! the promoted processes run, so a process resumed by its own `#0` reads the landed value;
//! what the landing wakes and the promoted `#0` resumes are one Active batch — the batch's
//! wakes first, in declaration order, then the promoted `#0` resumes in the order the `#0`s
//! were executed (IEEE 1800 §4.4.2.3 moves both kinds of Inactive event to Active together); a write
//! the landing schedules for `now` waits for the next promotion (the next Inactive round).
//!
//! Time 0: before any process is armed a delayed write whose effective delay is zero lands
//! INSIDE the settle — decided once the fixpoint has converged, by `schedule_delayed_cas`, and
//! settled again — like an undelayed driver: `assign #0 w = 1'b1;` reads 1 at `h0`, wakes
//! `always @(w)` once and posedges nothing (it is settle-constant and a copy like its undelayed
//! twin, `t0_edge`, `alias::copy_nets_landed`). In the settle that runs BEFORE the declaration
//! initializers every zero-delay decision is deferred (`Scheduler::pre_init`): the runtime
//! lane's delay and a zero-delay driver's rhs are phantoms there (`int dv = 5;` still holds its
//! default; `wire b = (s === 2'bxx)` of a `reg [1:0] s = 2'b01` is still 1), and the
//! initializer re-settle decides from the real values. Elaborate: a SCOPE-folded zero rise (`#(ZP)`) is `Some(0)` now — a
//! `#0` assign — so `#(ZP, 9)` carries its fall sidecar (both oracles fall at +9; it fell
//! immediately); on a RESOLVED net it keeps the pre-slice no-delay shape (`ca_zero_scope`).
//!
//! Oracles: iverilog 13 delivers a `#0` update IMMEDIATELY (before the writer's next statement
//! and before any process it woke: `h0 r=1`), verilator 5.052 two zero-delay hops after the
//! writer (`h2`) and one after a process the change woke. The two agree from the writer's second
//! hop and from a woken process's first, which is where every mid-run cell below is pinned; the
//! writer's own `h0`/`h1` reads are recorded as a split. Both oracles: no posedge, one level line
//! and `h0 w=1` for `assign #0 w = 1'b1;` at time 0; `h0 z=0` for `assign #(9, 0) z = a;` with
//! `a = 0`; the strobe and monitor lines. Every cell runs on all three backends.
//!
//! Recorded, not changed: the resolved-net delayed lane (`#(ZP)` on a two-driver net keeps its
//! pre-slice no-delay, the literal `#0` there is E3001); iverilog's `Y 5 y=0` on a cancelled
//! pulse; verilator's second delayed assign in a chain never following (`assign #0 b = a;`
//! behind `assign #0 a = u;` stays 0 in verilator, iverilog and vita read 1 by `h2`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_zdca_{}_{n}.sv", std::process::id()));
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
fn a_zero_delay_update_is_read_by_the_next_zero_delay_hop() {
    // The §2 cell: `always @(u)` woken by `u = 7` at 5. iverilog `h0 r=7` (immediate), verilator
    // `h0 r=0` and `h1 r=7`; both `h1`..`h3` = 7. Was `r=0` on every line.
    check(&[
        (
            "module t; reg [7:0] u = 0; wire [7:0] r; assign #0 r = u;\n\
             always @(u) begin $display(\"h0 r=%0d\", r); #0 $display(\"h1 r=%0d\", r);\n\
             #0 $display(\"h2 r=%0d\", r); #0 $display(\"h3 r=%0d\", r); end\n\
             initial begin #5 u = 7; #5 $finish; end endmodule\n",
            "h0 r=0\nh1 r=7\nh2 r=7\nh3 r=7\n",
        ),
        // The blocking read after three hops (both oracles `s=7`; was 0).
        (
            "module t; reg [7:0] u = 0; reg [7:0] s = 0; wire [7:0] r; assign #0 r = u;\n\
             always @(u) begin #0; #0; #0; s = r; end\n\
             initial begin #5 u = 7; #1 $display(\"s=%0d\", s); $finish; end endmodule\n",
            "s=7\n",
        ),
        // The writer's own hops: `h0` and `h1` are the split (iverilog 1 1, verilator 0 0),
        // `h2` both 1. vita lands at the first promotion.
        (
            "module t; reg a = 0; wire y; assign #0 y = a;\n\
             initial begin #5 a = 1; $display(\"h0 %0d\", y); #0 $display(\"h1 %0d\", y);\n\
             #0 $display(\"h2 %0d\", y); #1 $finish; end endmodule\n",
            "h0 0\nh1 1\nh2 1\n",
        ),
        // `$strobe` and `$monitor` are the Postponed region — both oracles read the landed value
        // (`S 5 r=1`, `M 5 r=1` once); vita printed `S 5 r=0` and two monitor lines at time 0
        // (`M 0 r=x`, `M 0 r=0`).
        (
            "module t; reg u = 0; wire r; assign #0 r = u;\n\
             initial $monitor(\"M %0t r=%0d\", $time, r);\n\
             initial begin #5 u = 7; $strobe(\"S %0t r=%0d\", $time, r); #2 $finish; end endmodule\n",
            "M 0 r=0\nS 5 r=1\nM 5 r=1\n",
        ),
    ]);
}

#[test]
fn every_zero_delay_spelling_lands_at_the_promotion() {
    check(&[
        // A runtime delay evaluating to zero (§2: iverilog `1 1 1`, verilator `0 0 1`; was
        // `0 0 0`), and one that becomes zero mid-run (both oracles `11 h2 y=0`; was 1).
        (
            "module t; int dz = 0; reg a = 0; wire y; assign #(dz) y = a;\n\
             initial begin #5 a = 1; $display(\"%0d\", y); #0 $display(\"%0d\", y);\n\
             #0 $display(\"%0d\", y); #1 $finish; end endmodule\n",
            "0\n1\n1\n",
        ),
        (
            "module t; int d = 5; reg a = 0; wire y; assign #(d) y = a;\n\
             initial begin #5 a = 1; #0 $display(\"%0t h1 y=%0d\", $time, y);\n\
             #5 $display(\"%0t y=%0d\", $time, y); d = 0; #1 a = 0;\n\
             #0 $display(\"%0t h1 y=%0d\", $time, y); #0 $display(\"%0t h2 y=%0d\", $time, y);\n\
             d = 3; #1 a = 1; #0 $display(\"%0t h1 y=%0d\", $time, y);\n\
             #3 $display(\"%0t y=%0d\", $time, y); #1 $finish; end endmodule\n",
            "5 h1 y=0\n10 y=1\n11 h1 y=0\n11 h2 y=0\n12 h1 y=0\n15 y=1\n",
        ),
        // The zero side of a rise/fall pair, literal and parameter: the rise lands at the first
        // hop (both oracles by the second), the fall at +9 (both oracles). `#(ZP, 9)` fell
        // immediately (`10 f0=0`) — the zero-rise trade, retired.
        (
            "module t; parameter ZP = 0; reg a = 0; wire y, z; assign #(0, 9) y = a; assign #(ZP, 9) z = a;\n\
             initial begin #5 a = 1; #0 $display(\"%0t r=%0d %0d\", $time, y, z);\n\
             #5 a = 0; #0 $display(\"%0t f0=%0d %0d\", $time, y, z); #8 $display(\"%0t f8=%0d %0d\", $time, y, z);\n\
             #1 $display(\"%0t f9=%0d %0d\", $time, y, z); #1 $finish; end endmodule\n",
            "5 r=1 1\n10 f0=1 1\n18 f8=1 1\n19 f9=0 0\n",
        ),
        // `#(ZP)` is the literal `#0` (it was no delay: `h0 y=1`).
        (
            "module t; parameter ZP = 0; reg a = 0; wire y, z; assign #(ZP) y = a; assign #0 z = a;\n\
             initial begin #5 a = 1; $display(\"h0 y=%0d z=%0d\", y, z); #0 $display(\"h1 y=%0d z=%0d\", y, z);\n\
             #0 $display(\"h2 y=%0d z=%0d\", y, z); #1 $finish; end endmodule\n",
            "h0 y=0 z=0\nh1 y=1 z=1\nh2 y=1 z=1\n",
        ),
        // The net-declaration delay and the gate (both oracles `h2 r=1 g=1`).
        (
            "module t; reg u = 0; wire #0 r = u; wire g; buf #0 (g, u);\n\
             initial begin #5 u = 1; #0 $display(\"h1 r=%0d g=%0d\", r, g); #0 $display(\"h2 r=%0d g=%0d\", r, g);\n\
             #1 $finish; end endmodule\n",
            "h1 r=1 g=1\nh2 r=1 g=1\n",
        ),
        // A part-select target (both oracles `h2 w=1110`).
        (
            "module t; reg [3:0] u = 0; wire [3:0] w; assign #0 w[1:0] = u[3:2]; assign w[3:2] = 2'b11;\n\
             initial begin #5 u = 4'b1000; $display(\"h0 w=%b\", w); #0 $display(\"h1 w=%b\", w);\n\
             #0 $display(\"h2 w=%b\", w); #1 $finish; end endmodule\n",
            "h0 w=1100\nh1 w=1110\nh2 w=1110\n",
        ),
    ]);
}

#[test]
fn what_the_landing_wakes_runs_before_the_promotion() {
    check(&[
        // The woken level waiter and the writer's `#0` continuation are one Active batch: the
        // landing's wakes first, in declaration order, then the promoted `#0` resumes in the
        // order their `#0` executed — so the waiter prints before `B` whichever is declared
        // first (iverilog's order; verilator prints `B r=0` first — its second hop, the `#0`
        // visibility split). No `R 0 r=0`:
        // `r` is a `#0` copy of an initialised `u` (iverilog; verilator runs every level waiter
        // once at time 0).
        (
            "module t; reg u = 0; wire r; assign #0 r = u;\n\
             always @(r) $display(\"R %0t r=%0d\", $time, r);\n\
             initial begin #5 u = 7; $display(\"A\"); #0 $display(\"B r=%0d\", r); #0 $display(\"C\");\n\
             #1 $finish; end endmodule\n",
            "A\nR 5 r=1\nB r=1\nC\n",
        ),
        (
            "module t; reg u = 0; wire r; assign #0 r = u;\n\
             initial begin #5 u = 7; $display(\"A\"); #0 $display(\"B r=%0d\", r); #0 $display(\"C\");\n\
             #1 $finish; end\n\
             always @(r) $display(\"R %0t r=%0d\", $time, r);\n\
             endmodule\n",
            "A\nR 5 r=1\nB r=1\nC\n",
        ),
        // A posedge waiter and an NBA sampled before the landing (both oracles `h2 s=0 r=1`).
        (
            "module t; reg u = 0; reg s = 0; wire r; assign #0 r = u;\n\
             always @(posedge r) $display(\"P %0t\", $time);\n\
             initial begin #5 u = 1; s <= r; #0 $display(\"h1 s=%0d r=%0d\", s, r);\n\
             #0 $display(\"h2 s=%0d r=%0d\", s, r); #1 $finish; end endmodule\n",
            "P 5\nh1 s=0 r=1\nh2 s=0 r=1\n",
        ),
        // `@*` and `always_comb` readers (both oracles `h2 q=1 q2=1`).
        (
            "module t; reg u = 0; wire r; assign #0 r = u; reg q; reg q2;\n\
             always @* q = r; always_comb q2 = r;\n\
             initial begin #5 u = 1; #0 $display(\"h1 q=%0d q2=%0d\", q, q2);\n\
             #0 $display(\"h2 q=%0d q2=%0d\", q, q2); #1 $finish; end endmodule\n",
            "h1 q=1 q2=1\nh2 q=1 q2=1\n",
        ),
        // A chain of two `#0` assigns: the second lands one promotion after the first (the
        // LRM's next Inactive round; iverilog reads both at once, verilator never follows).
        (
            "module t; reg u = 0; wire a, b, c; assign #0 a = u; assign #0 b = a; assign c = b;\n\
             initial begin #5 u = 1; #0 $display(\"h1 a=%0d b=%0d c=%0d\", a, b, c);\n\
             #0 $display(\"h2 a=%0d b=%0d c=%0d\", a, b, c); #1 $finish; end endmodule\n",
            "h1 a=1 b=0 c=0\nh2 a=1 b=1 c=1\n",
        ),
        // Scheduled from the NBA region, it still wakes `@(r)` (all three tools `R 5 r=1`).
        (
            "module t; reg u = 0; wire r; assign #0 r = u;\n\
             initial begin #5 u <= 1; @(r) $display(\"R %0t r=%0d\", $time, r); #1 $finish; end endmodule\n",
            "R 5 r=1\n",
        ),
        // A `#0` clock counts every edge (all three tools `cnt=3`).
        (
            "module t; reg clk = 0; wire clkd; assign #0 clkd = clk; int cnt = 0;\n\
             always @(posedge clkd) cnt++;\n\
             initial begin forever #5 clk = ~clk; end\n\
             initial begin #27 $display(\"cnt=%0d\", cnt); $finish; end endmodule\n",
            "cnt=3\n",
        ),
        // A `$finish` in the landing's step: the landing and its wake still run (iverilog; the
        // finish takes effect at the step's stable point, §4.5.531).
        (
            "module t; reg u = 0; wire r; assign #0 r = u;\n\
             always @(r) $display(\"R %0t r=%0d\", $time, r);\n\
             initial begin #5 u = 7; $finish; end endmodule\n",
            "R 5 r=1\n",
        ),
    ]);
}

#[test]
fn a_zero_delay_driver_lands_in_the_time_zero_settle() {
    check(&[
        // A literal driver: `h0 w=1`, one level line, no posedge (both oracles; was `h0 w=x`
        // and `P 0 w=1`). Literal `#0`, `#(ZP)`, and the zero rise of `#(0, 9)` / `#(ZP, 9)`.
        (
            "module t; parameter ZP = 0; wire w, v, y, z;\n\
             assign #0 w = 1'b1; assign #(ZP) v = 1'b1; assign #(0, 9) y = 1'b1; assign #(ZP, 9) z = 1'b1;\n\
             always @(w) $display(\"LW %0t w=%b\", $time, w);\n\
             always @(posedge w) $display(\"PW %0t\", $time);\n\
             always @(v) $display(\"LV %0t v=%b\", $time, v);\n\
             always @(posedge v) $display(\"PV %0t\", $time);\n\
             always @(y) $display(\"LY %0t y=%b\", $time, y);\n\
             always @(posedge y) $display(\"PY %0t\", $time);\n\
             always @(z) $display(\"LZ %0t z=%b\", $time, z);\n\
             always @(posedge z) $display(\"PZ %0t\", $time);\n\
             initial begin $display(\"h0 %b %b %b %b\", w, v, y, z); #0 $display(\"h1 %b %b %b %b\", w, v, y, z);\n\
             #3 $finish; end endmodule\n",
            "h0 1 1 1 1\nLW 0 w=1\nLV 0 v=1\nLY 0 y=1\nLZ 0 z=1\nh1 1 1 1 1\n",
        ),
        // A `#1` driver keeps its x phase and its posedge at 1 (iverilog); the `#0` twin beside it
        // posedges nothing (both oracles).
        (
            "module t; wire w1; assign #1 w1 = 1'b1; wire w0; assign #0 w0 = 1'b1;\n\
             always @(posedge w1) $display(\"P1 %0t\", $time);\n\
             always @(w1) $display(\"L1 %0t w1=%b\", $time, w1);\n\
             always @(posedge w0) $display(\"P0 %0t\", $time);\n\
             initial #3 $finish; endmodule\n",
            "P1 1\nL1 1 w1=1\n",
        ),
        // A driver reading a variable keeps the settle's edge, as its undelayed twin does (both
        // oracles `P 0 w=01`).
        (
            "module t; reg r = 0; wire [1:0] w; assign #0 w = {r, 1'b1};\n\
             always @(w) $display(\"L %0t w=%b\", $time, w);\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             initial #3 $finish; endmodule\n",
            "L 0 w=01\nP 0 w=01\n",
        ),
        // A copy of an initialised variable: `h0 w=0`, no edge, no level line — the undelayed
        // twin's answer (it printed `h0 w=x`, `L 0 w=0`, `N 0 w=0`; both oracles are silent on
        // the edge, iverilog on the level line too).
        (
            "module t; reg r = 0; wire w; assign #0 w = r; wire u; assign u = r;\n\
             always @(w) $display(\"L %0t w=%b\", $time, w);\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             always @(negedge w) $display(\"N %0t w=%b\", $time, w);\n\
             always @(u) $display(\"LU %0t u=%b\", $time, u);\n\
             always @(negedge u) $display(\"NU %0t u=%b\", $time, u);\n\
             initial begin $display(\"h0 w=%b u=%b\", w, u); #0 $display(\"h1 w=%b u=%b\", w, u); #1 $finish; end endmodule\n",
            "h0 w=0 u=0\nh1 w=0 u=0\n",
        ),
        // The zero FALL of `#(9, 0)` lands the settle's 0 (both oracles `h0 z=0`; was `x` until
        // the first hop); the zero RISE of `#(0, 9)` on a 0 rhs keeps the x phase to 9
        // (iverilog; verilator is 2-state).
        (
            "module t; reg a = 0; wire y, z; assign #(0, 9) y = a; assign #(9, 0) z = a;\n\
             initial begin $display(\"h0 y=%b z=%b\", y, z); #0 $display(\"h1 y=%b z=%b\", y, z);\n\
             #8 $display(\"8 y=%b z=%b\", y, z); #1 $display(\"9 y=%b z=%b\", y, z); #1 $finish; end endmodule\n",
            "h0 y=x z=0\nh1 y=x z=0\n8 y=x z=0\n9 y=0 z=0\n",
        ),
        // An unwritten variable behind `#0`: x, no level line (iverilog; the x-drop, §4.5.533).
        (
            "module t; reg r; wire w; assign #0 w = r;\n\
             always @(w) $display(\"L %0t w=%b\", $time, w);\n\
             initial begin $display(\"h0 w=%b\", w); #0 $display(\"h1 w=%b\", w); #1 $finish; end endmodule\n",
            "h0 w=x\nh1 w=x\n",
        ),
    ]);
}

#[test]
fn the_runtime_lane_is_decided_after_the_initializers() {
    check(&[
        // `int dz = 0; assign #(dz) y = a;` with `reg a = 1`: `h0 y=1` (iverilog; verilator's
        // second hop), no posedge (iverilog; it is a copy of `a`). `int dv = 5; assign #(dv) z`
        // keeps its x phase to 5 and posedges there (iverilog `PZ 5`, both oracles `5 z=1`).
        // Both oracles print the `#5` resume's `5 y=1 z=1` before `PZ 5`: the landing's wake
        // runs behind the delay resume already due at 5 (verilator's time-0 lines differ on
        // the value axis above).
        (
            "module t; int dz = 0; int dv = 5; reg a = 1; wire y, z; assign #(dz) y = a; assign #(dv) z = a;\n\
             always @(posedge y) $display(\"PY %0t\", $time);\n\
             always @(posedge z) $display(\"PZ %0t\", $time);\n\
             initial begin $display(\"h0 y=%b z=%b\", y, z); #0 $display(\"h1 y=%b z=%b\", y, z);\n\
             #0 $display(\"h2 y=%b z=%b\", y, z); #5 $display(\"5 y=%b z=%b\", y, z); #1 $finish; end endmodule\n",
            "h0 y=1 z=x\nh1 y=1 z=x\nh2 y=1 z=x\n5 y=1 z=1\nPZ 5\n",
        ),
        // A constant rhs under a variable delay: the delay is read AFTER `int dv = 5;` has run
        // (iverilog `z=x` to 5, then 1; verilator folds the constant to 1 from time 0 — a split;
        // vita landed the phantom-zero write at time 0, after the hops: `4 z=1`).
        (
            "module t; int dv = 5; wire z; assign #(dv) z = 1'b1; reg a = 1; wire y; assign #(dv) y = a;\n\
             initial begin $display(\"h0 z=%b y=%b\", z, y); #0 $display(\"h1 z=%b y=%b\", z, y);\n\
             #4 $display(\"4 z=%b y=%b\", z, y); #1 $display(\"5 z=%b y=%b\", z, y); #1 $finish; end endmodule\n",
            "h0 z=x y=x\nh1 z=x y=x\n4 z=x y=x\n5 z=1 y=1\n",
        ),
    ]);
}

#[test]
fn a_zero_delay_oscillator_is_loud() {
    // `assign #0 a = s ? ~a : 1'b0;` once `s` is 1: every landing schedules the next, in the
    // same time step. It ran until the watchdog (so does iverilog); it is the delta limit now.
    let src = "module t; reg s = 0; wire a; assign #0 a = s ? ~a : 1'b0;\n\
               initial begin #5 s = 1; #5 $finish; end endmodule\n";
    for b in [None, Some("interp"), Some("vm"), Some("native")] {
        let (out, ok) = vita_on(src, b);
        assert!(!ok, "backend {b:?}: expected a loud stop, got:\n{out}");
        assert!(
            out.contains("F-RUN-NO-CONVERGE") && out.contains("at time 5"),
            "backend {b:?}: got:\n{out}"
        );
    }
}

#[test]
fn a_resolved_net_keeps_the_pre_slice_shapes() {
    // Recorded residue (ROADMAP §2: the delayed lane cannot drive a resolved net). `#(ZP)` on a
    // two-driver net is demoted to no delay as it was (`h0 m=0`, then the resolution's x);
    // `#(ZP, 9)` there falls immediately as it did; the literal `#0` is E3001 as it was.
    check(&[
        (
            "module t; parameter ZP = 0; reg a = 0, b = 0; wire m; assign #(ZP) m = a; assign m = b;\n\
             initial begin #5 a = 1; $display(\"h0 m=%b\", m); #0 $display(\"h1 m=%b\", m);\n\
             #0 $display(\"h2 m=%b\", m); #1 $finish; end endmodule\n",
            "h0 m=0\nh1 m=x\nh2 m=x\n",
        ),
        (
            "module t; parameter ZP = 0; reg a = 0, b = 0; wire m; assign #(ZP, 9) m = a; assign m = b;\n\
             initial begin #5 a = 1; #1 $display(\"6 m=%b\", m); a = 0; #1 $display(\"7 m=%b\", m);\n\
             #10 $display(\"17 m=%b\", m); $finish; end endmodule\n",
            "6 m=x\n7 m=0\n17 m=0\n",
        ),
    ]);
    let (out, ok) = vita_on(
        "module t; reg a = 0, b = 0; wire m; assign #0 m = a; assign m = b;\n\
         initial begin #5 a = 1; #1 $finish; end endmodule\n",
        None,
    );
    assert!(!ok && out.contains("E-ELAB-MULTIDRIVER"), "got:\n{out}");
}
