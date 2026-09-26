//! The time-0 settle of a SETTLE-CONSTANT net is not an EDGE (ROADMAP §2 Delays/events,
//! "an edge waiter on a net whose time-0 settle lands on a definite value fires at
//! time 0"). The rule and its measurement: `sim_engine::t0_edge`.
//!
//! A net starts at its declared default (`z` for a `wire`, `x` for a `logic`) and the
//! time-0 settle writes every continuous driver's value before any process is armed.
//! That record stays on the dirty list so a LEVEL waiter runs once at time 0 —
//! `always @(w)` on `wire w = 1'b1;` prints `W 0 w=1` in both oracles — but the write
//! funnel had also folded the `z → 1` hop into the net's intra-slot edge mask, and the
//! first delta's edge scan read it as a posedge: `always @(posedge w)` printed `P 0`,
//! `always_ff @(posedge w) q <= ~q;` flipped a `logic q = 0` once, `initial @(posedge w)`
//! fell through at time 0, and a `wire v = 1'b0` gave `always @(negedge v)` an `N 0`.
//!
//! Neither iverilog 13.0 (`-g2012`) nor verilator 5.052 (`--binary --timing`) gives
//! the settle an edge on a net with NO variable behind it: a literal, a multi-bit
//! literal, a multi-driver constant, a copy of a constant wire (one hop or three), a
//! concat of constant wires, a hierarchical port copy, `{1'b0, k}` of a constant `k`,
//! `logic d; assign d = 1'b1;`, `$clog2(2)`, `4'(1)` — while both give the level waiter on the same net its
//! time-0 run. And both DO fire the edge when the driver reads a variable and bit 0
//! settles definite: `reg r = 0; wire [1:0] w = {r, 1'b1};` prints `P 0 w=01` in both,
//! as do `bit b; {b, 1'b1}`, `b ? 2'b01 : 2'b11`, `{r, k}` of a constant `k`, and the
//! same shape through a port. So both kernels (`arm_processes`, `arm_t0`) zero the edge
//! mask of the settle-constant nets only — a net whose every continuous driver reads
//! literals and other settle-constant nets, by fixpoint — and keep every mask and every
//! dirty membership otherwise. A value written to a net LATER in time 0 is a fresh
//! change and still edges (`initial r = 1;` on `wire w = r;` prints `P 0 w=1`).
//!
//! Every expected text below is printed by both oracles, except where marked
//! iverilog-only (an x in the value: verilator is 2-state).
//!
//! Not pinned, measured and left as they are:
//! - iverilog decides the time-0 edge by the driver's functor, not by the value: an
//!   `and g(w, 1'b1, 1'b1)` posedges at 0, `wire n = ~w` of a constant `w` negedges,
//!   `reg a = 0; wire w = a | 1'b1` posedges — where the literal `1'b1`, a copy of it and
//!   `wire w = ~r` with `reg r = 0` do not. verilator is silent on the first three (it
//!   folds them) and fires on the last. vita: the first two are settle-constant (silent,
//!   verilator's text); the other two read a variable (they fire, iverilog's / verilator's
//!   text). A 2-state unwritten variable through an operator (`bit b; wire w = ~b`,
//!   `int i; (i == 0)`) is silent in both oracles and fires in vita — but verilator fires
//!   on `{b, 1'b1}` of the same `b` and iverilog on `{1'b0, b} | 2'b01`, so neither is an
//!   oracle for "reads an unwritten 2-state variable". PRE = POST on all of these.
//! - A driver that reads a variable BEFORE its initializer or first-batch write lands:
//!   `reg r = 1; wire w = (r !== 1'b1);` settles to 1 and is recomputed to 0 in the first
//!   delta, so vita prints `P 0` and `N 0` where both oracles print only `W 0 w=0`.
//!   Pre-existing (PRE = POST), ROADMAP §2.
//! - `assign #0 w = 1'b1;` posedged at 0 (its write landed as a fresh change after the
//!   settle); both oracles print only the level line. Closed: a zero-delay driver lands its
//!   time-0 value inside the settle and is settle-constant like its undelayed twin
//!   (`zero_delay_cont_assign.rs`).
//! - `initial begin r = 0; r = 1; r = 0; end` on `wire w = r;`: iverilog propagates each
//!   blocking write and posedges at 0, verilator and vita see only the batch's end
//!   value. PRE = POST.
//! - `always @(posedge w[1])` is E3009 (non-LSB edge select), unchanged.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_t0e_{}_{n}.sv", std::process::id()));
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
            && !l.contains("W-ELAB-MULTIDRIVER-STRICT")
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

/// One net with one constant driver, one waiter, a `#3 $finish` watchdog.
fn cell(decl: &str, waiter: &str) -> String {
    format!("module top; {decl}\n{waiter}\ninitial #3 $finish;\nendmodule\n")
}

#[test]
fn a_constant_drivers_settle_is_not_an_edge_at_time_zero() {
    check(&[
        (
            &cell(
                "wire w = 1'b1;",
                "always @(posedge w) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "wire v = 1'b0;",
                "always @(negedge v) $display(\"N %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "wire w = 1'b1;",
                "always @(posedge w, negedge w) $display(\"E %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "wire [3:0] w = 4'd5;",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);",
            ),
            "",
        ),
        (
            &cell(
                "wire [3:0] w = 4'd4;",
                "always @(negedge w) $display(\"N %0t w=%b\", $time, w);",
            ),
            "",
        ),
        (
            &cell(
                "wire w; assign w = 1'b1; assign w = 1'b1;",
                "always @(posedge w) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "wire w = 1'b1; wire c = w;",
                "always @(posedge c) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "logic d; assign d = 1'b1;",
                "always @(posedge d) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "logic d; assign d = 1'b0;",
                "always @(negedge d) $display(\"N %0t\", $time);",
            ),
            "",
        ),
        (
            "module sub(input a, output b); assign b = a; endmodule\n\
             module top; wire x = 1'b1; wire y; sub s(x, y);\n\
             always @(posedge y) $display(\"P %0t\", $time);\n\
             always @(y) $display(\"W %0t y=%b\", $time, y);\n\
             initial #3 $finish;\nendmodule\n",
            "W 0 y=1\n",
        ),
        // a constant through one copy, three copies, a concat of copies, two drivers
        (
            &cell(
                "wire k = 1'b1; wire [1:0] w = {1'b0, k};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
                 always @(w) $display(\"W %0t w=%b\", $time, w);",
            ),
            "W 0 w=01\n",
        ),
        (
            &cell(
                "wire k = 1'b1; wire c1 = k; wire c2 = c1; wire c3 = c2;",
                "always @(posedge c3) $display(\"P %0t\", $time);\n\
                 always @(c3) $display(\"W %0t c3=%b\", $time, c3);",
            ),
            "W 0 c3=1\n",
        ),
        (
            &cell(
                "wire k = 1'b1; wire [1:0] w = {k, k};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
                 always @(w) $display(\"W %0t w=%b\", $time, w);",
            ),
            "W 0 w=11\n",
        ),
        // a pure system function or cast of literals is still a constant driver
        (
            &cell(
                "wire [1:0] w = $clog2(2); wire [3:0] v = 4'(1); wire s = $signed(1'b1);",
                "always @(posedge w) $display(\"P %0t\", $time);\n\
                 always @(posedge v) $display(\"Q %0t\", $time);\n\
                 always @(posedge s) $display(\"R %0t\", $time);\n\
                 always @(w or v or s) $display(\"W %0t %b %b %b\", $time, w, v, s);",
            ),
            "W 0 01 0001 1\n",
        ),
        (
            &cell(
                "wire a = 1'b1; wire b; assign b = a; assign b = a;",
                "always @(posedge b) $display(\"P %0t\", $time);\n\
                 always @(b) $display(\"W %0t b=%b\", $time, b);",
            ),
            "W 0 b=1\n",
        ),
    ]);
}

/// The other half of the rule: a driver that reads a variable keeps the settle's edge.
#[test]
fn a_driver_that_reads_a_variable_keeps_its_edge() {
    check(&[
        (
            &cell(
                "reg r = 0; wire [1:0] w = {r, 1'b1};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
                 always @(w) $display(\"W %0t w=%b\", $time, w);",
            ),
            "P 0 w=01\nW 0 w=01\n",
        ),
        (
            &cell(
                "bit b; wire [1:0] w = {b, 1'b1};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
                 always @(w) $display(\"W %0t w=%b\", $time, w);",
            ),
            "P 0 w=01\nW 0 w=01\n",
        ),
        (
            &cell(
                "bit b; wire [1:0] w = b ? 2'b01 : 2'b11;",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);",
            ),
            "P 0 w=11\n",
        ),
        // iverilog-only text (an x in the value)
        (
            &cell(
                "reg r; wire [1:0] w = {r, 1'b1};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);",
            ),
            "P 0 w=x1\n",
        ),
        // a constant read beside a variable is not a constant driver
        (
            &cell(
                "reg r = 0; wire k = 1'b1; wire [1:0] w = {r, k};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);",
            ),
            "P 0 w=01\n",
        ),
        (
            "module sub(input [1:0] i);\n\
             always @(posedge i) $display(\"P %0t i=%b\", $time, i);\n\
             always @(i) $display(\"W %0t i=%b\", $time, i);\nendmodule\n\
             module top; bit b; sub s({b, 1'b1}); initial #3 $finish; endmodule\n",
            "P 0 i=01\nW 0 i=01\n",
        ),
    ]);
}

#[test]
fn the_level_waiter_on_the_same_settle_still_runs_once() {
    check(&[
        (
            &cell(
                "wire w = 1'b1;",
                "always @(w) $display(\"W %0t w=%b\", $time, w);",
            ),
            "W 0 w=1\n",
        ),
        (
            &cell(
                "wire v = 1'b0;",
                "always @(v) $display(\"W %0t v=%b\", $time, v);",
            ),
            "W 0 v=0\n",
        ),
        (
            &cell(
                "wire w = 1'b1;",
                "always @(posedge w) $display(\"P %0t\", $time);\n\
                 always @(w) $display(\"W %0t\", $time);",
            ),
            "W 0\n",
        ),
        (
            &cell(
                "wire w = 1'b1; wire n = ~w;",
                "always @(negedge n) $display(\"N %0t n=%b\", $time, n);\n\
                 always @(n) $display(\"W %0t n=%b\", $time, n);",
            ),
            "W 0 n=0\n",
        ),
    ]);
}

#[test]
fn an_inline_edge_control_armed_at_time_zero_waits() {
    check(&[
        (
            &cell(
                "wire w = 1'b1;",
                "initial begin @(posedge w) $display(\"I %0t\", $time); end",
            ),
            "",
        ),
        (
            &cell(
                "wire w = 1'b1;",
                "initial begin #1; @(posedge w) $display(\"I %0t\", $time); end",
            ),
            "",
        ),
        // `#0` after the settle: the edge does not reappear in the inactive batch
        (
            "module top; wire w = 1'b1;\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             initial begin #0; $display(\"Z\"); #3 $finish; end\nendmodule\n",
            "Z\n",
        ),
    ]);
}

#[test]
fn the_dropped_edge_no_longer_stores() {
    check(&[
        (
            "module top; logic q = 0; wire w = 1'b1;\n\
             always_ff @(posedge w) q <= ~q;\n\
             initial #3 $display(\"q=%b\", q);\ninitial #4 $finish;\nendmodule\n",
            "q=0\n",
        ),
        (
            "module top; wire w = 1'b1; reg q = 0;\n\
             always @(posedge w) q <= 1;\n\
             initial #3 $display(\"q=%b\", q);\ninitial #4 $finish;\nendmodule\n",
            "q=0\n",
        ),
        (
            "module top; reg clk = 0; wire w = 1'b1;\n\
             always @(posedge w or posedge clk) $display(\"P %0t clk=%b\", $time, clk);\n\
             initial begin #1 clk = 1; #1 clk = 0; #1 clk = 1; #1 $finish; end\nendmodule\n",
            "P 1 clk=1\nP 3 clk=1\n",
        ),
    ]);
}

#[test]
fn a_change_after_the_settle_still_edges() {
    check(&[
        // a first-batch write is a time-0 change
        (
            "module top; reg r; wire w = r;\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             initial begin r = 1; #3 $finish; end\nendmodule\n",
            "P 0 w=1\n",
        ),
        (
            "module top; reg r; wire w = r;\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             initial begin #5 r = 1; #3 $finish; end\nendmodule\n",
            "P 5 w=1\n",
        ),
        (
            "module top; wire w; assign #2 w = 1'b1;\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             initial #5 $finish;\nendmodule\n",
            "P 2\n",
        ),
        (
            "module top; wire w = 1'b1;\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             initial begin #1; force w = 0; #1 release w; #3 $finish; end\nendmodule\n",
            "P 2\n",
        ),
        (
            "module top; reg r; wire w = r;\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 0; #1 r = 1; #3 $finish; end\nendmodule\n",
            "W 1 w=0\nP 2 w=1\nW 2 w=1\n",
        ),
    ]);
}

#[test]
fn the_opposite_edge_and_an_initialised_reg_are_unchanged() {
    check(&[
        (
            &cell(
                "wire w = 1'b1;",
                "always @(negedge w) $display(\"N %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "wire v = 1'b0;",
                "always @(posedge v) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "reg clk = 1;",
                "always @(posedge clk) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "reg r = 1; wire w = r;",
                "always @(posedge w) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            &cell(
                "reg [7:0] r8 = 8'h13; wire [3:0] w = r8[3:0];",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);",
            ),
            "",
        ),
    ]);
}

#[test]
fn every_backend_drops_the_same_edge() {
    let cells = [
        (
            cell(
                "wire w = 1'b1;",
                "always @(posedge w) $display(\"P %0t\", $time);",
            ),
            "",
        ),
        (
            "module top; logic q = 0; wire w = 1'b1;\n\
             always_ff @(posedge w) q <= ~q;\n\
             initial #3 $display(\"q=%b\", q);\ninitial #4 $finish;\nendmodule\n"
                .to_string(),
            "q=0\n",
        ),
        (
            cell(
                "wire w = 1'b1;",
                "always @(posedge w) $display(\"P %0t\", $time);\n\
                 always @(w) $display(\"W %0t\", $time);",
            ),
            "W 0\n",
        ),
        (
            "module top; reg r; wire w = r;\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             initial begin r = 1; #3 $finish; end\nendmodule\n"
                .to_string(),
            "P 0 w=1\n",
        ),
        (
            cell(
                "reg r = 0; wire [1:0] w = {r, 1'b1};",
                "always @(posedge w) $display(\"P %0t w=%b\", $time, w);",
            ),
            "P 0 w=01\n",
        ),
    ];
    for (src, want) in &cells {
        for be in ["interp", "vm", "native"] {
            let (got, ok) = vita_on(src, Some(be));
            assert!(ok, "{be}: expected exit 0, got:\n{got}");
            assert_eq!(got, *want, "backend {be}, design:\n{src}");
        }
    }
}
