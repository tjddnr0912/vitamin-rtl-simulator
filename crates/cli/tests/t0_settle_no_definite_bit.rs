//! The time-0 settle of a continuous driver whose value has NO definite bit wakes no
//! level waiter (ROADMAP §2 Delays/events, "a wire driven only by a continuous assign
//! raises a false event at t=0").
//!
//! A net starts at its declared default (`z` for a `wire`, `x` for a `logic`) and the
//! time-0 settle writes every continuous driver's value before any process is armed.
//! The settle's writes stay on the dirty list on purpose — `always @(w)` on
//! `assign w = 1'b1;` runs once at time 0 in both oracles — but the list also carried
//! the `z → x` hop of a driver whose operands were all still x: `wire w = r + 1;`
//! before any `initial` wrote `r`, `~r`, a delayed `assign #5` holding its x, a
//! multi-driver x, `logic d; assign d = 1'bz;`. vita ran `always @(w)` once at time 0
//! with `w = x` — an extra line at exit 0, or a wrong value when the body stores
//! (`always @(w) q = w;` turned a `reg q = 9` into x).
//!
//! iverilog gives that hop no event, and a settled value with a definite bit somewhere
//! always gets one (`1'b1`, `4'd5`, `r & 4'b0011` = `00xx`, `{r, 3'b101}` = `x101`,
//! `{1'bz, 1'b0}` = `z0`). So the rule is on the VALUE: a settled value with no definite
//! bit is dropped from the time-0 wake list; one with a definite bit anywhere is kept. Computed nets only — a copy net (`sim_engine::alias`)
//! has no event of its own and keeps taking its sources' status
//! (`copy_net_no_t0_transition.rs`). Both kernels apply it (`arm_processes`, `arm_t0`).
//!
//! Every expected text below is iverilog 13.0 (`-g2012`); verilator 5.052 is 2-state
//! and prints `0` where iverilog prints `x`, so it is not an oracle for the hop itself.
//! Where the two oracles print one time step in two orders the cell compares sorted
//! lines.
//!
//! Not pinned, measured and left as they are (all PRE = POST):
//! - `wire w = r ? 1'b1 : 1'b0;` with `r` unwritten: iverilog wakes `W 0 w=x` where it
//!   wakes nothing for `~r`, `^r`, `r == 2` — the same x from a different operator; and
//!   an x/z LITERAL piece wakes it where an equal value from a net does not (`2'bxz`
//!   wakes, `{r, 1'bz}` = `xz` does not; `{r, 1'bx}` wakes, `{r, r}` and `2'bxx` do not;
//!   `always @(e) q = e;` on `{r, 1'bx}` stores `xx` there, vita keeps `q`). An iverilog
//!   self-contradiction on equal values, so no oracle; vita wakes nothing.
//! - An unpacked array is one net: with `a[0] = 4'd3` and `a[1] = r + 1`, a copy
//!   `wire b1 = a[1]` wakes at time 0 with `xxxx` (iverilog: no line) — the array's dirt
//!   is kept for its definite element and the copy takes it (pre-existing, PRE = POST).
//! - `wire w = 1'b1; always @(posedge w)` at time 0: neither oracle fires, vita does
//!   (the settle's `z → 1` is read as a posedge). ROADMAP §2.
//! - a first-batch `initial $display(w)` of `r & 4'b0011` prints `xxxx` in iverilog
//!   (its network's value before the time-0 evaluation) and `00xx` in vita.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_t0x_{}_{n}.sv", std::process::id()));
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

fn expect(src: &str, want: &str) {
    assert_eq!(run(src), want, "design:\n{src}");
}

fn check(cells: &[(&str, &str)]) {
    for (src, want) in cells {
        expect(src, want);
    }
}

fn check_sorted(src: &str, want: &str) {
    let mut got: Vec<String> = run(src).lines().map(str::to_string).collect();
    let mut want: Vec<String> = want.lines().map(str::to_string).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "design:\n{src}");
}

/// `wire [3:0] w = r + 1;` watched by `always @(w)`, with `init` writing `r`.
fn plus_one(init: &str) -> String {
    format!(
        "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
         always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
         initial begin {init} #10 $finish; end\nendmodule\n"
    )
}

#[test]
fn a_computed_net_settling_on_x_wakes_no_level_waiter_at_time_zero() {
    check(&[
        (&plus_one("#0 r = 2;"), "W 0 w=3\n"),
        (&plus_one("#1 r = 2;"), "W 1 w=3\n"),
        (&plus_one("r <= 2;"), "W 0 w=3\n"),
        (&plus_one("r = 4'bx; #1 r = 2;"), "W 1 w=3\n"),
        (&plus_one(""), ""),
        // the first-batch write coalesces with the settle, as before
        (&plus_one("r = 2;"), "W 0 w=3\n"),
        (
            "module top; reg r; wire w = ~r;\n\
             initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #0 r = 0; #10 $finish; end\nendmodule\n",
            "I0 w=x\nW 0 w=1\n",
        ),
        (
            "module top; reg [3:0] r = 4'bxx01; wire [3:0] w = r + 1;\n\
             initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "I0 w=xxxx\nW 1 w=0011\n",
        ),
        // two hops, written after `#0`
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1; wire [3:0] v = w + 1;\n\
             always @(v) $display(\"V %0t v=%0d\", $time, v);\n\
             initial begin #0 r = 2; #10 $finish; end\nendmodule\n",
            "V 0 v=4\n",
        ),
        // written by an `always @(K)` (the time-0 pulse lane, after the first batch)
        (
            "module top; localparam int K = 3; reg [3:0] r; wire [3:0] w = r + 1;\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
             always @(K) r = K;\n\
             initial begin #5 $display(\"END r=%0d w=%0d\", r, w); $finish; end\nendmodule\n",
            "W 0 w=4\nEND r=3 w=4\n",
        ),
        (
            "module top; localparam int K = 2; reg [3:0] r; wire [3:0] n1 = r + 1; wire [3:0] n2 = n1 + 1;\n\
             always @(n2) $display(\"N2 at %0t n2=%0d\", $time, n2);\n\
             always @(K) r = K;\n\
             initial begin #5 $finish; end\nendmodule\n",
            "N2 at 0 n2=4\n",
        ),
        // beside an `always @(*)`, which runs at time 0 on its own
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
             always @(*) $display(\"S %0t w=%0d\", $time, w);\n\
             initial begin #0 r = 2; #10 $finish; end\nendmodule\n",
            "W 0 w=3\nS 0 w=3\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
             always @(*) $display(\"S %0t w=%0d\", $time, w);\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "W 1 w=3\nS 1 w=3\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
             always @(r) $display(\"R %0t r=%0d\", $time, r);\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "W 1 w=3\nR 1 r=2\n",
        ),
        // a gate primitive, a multi-driver x, a `wand` x
        (
            "module top; reg a, b; wire w; and g(w, a, b);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 a = 1; b = 1; #10 $finish; end\nendmodule\n",
            "W 1 w=1\n",
        ),
        (
            "module top; reg r; wire w; assign w = r; assign w = 1'b0;\n\
             initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 1; #10 $finish; end\nendmodule\n",
            "I0 w=x\n",
        ),
        (
            "module top; reg r; wand w; assign w = r; assign w = 1'b1;\n\
             initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 1; #10 $finish; end\nendmodule\n",
            "I0 w=x\nW 1 w=1\n",
        ),
        // constants with no definite bit, and a partial-z concat of an x
        (
            "module top; wire w = 1'bx;\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n  initial #10 $finish;\nendmodule\n",
            "I0 w=x\n",
        ),
        (
            "module top; wire w = 1'bz;\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n  initial #10 $finish;\nendmodule\n",
            "I0 w=z\n",
        ),
        (
            "module top; reg r; wire [1:0] w = {1'bz, r};\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 0; #10 $finish; end\nendmodule\n",
            "I0 w=zx\nW 1 w=z0\n",
        ),
        // a `logic` net's x → z is not an event either, nor is a copy of one
        (
            "module top; logic d; assign d = 1'bz;\n\
             reg [7:0] c = 8'd0; always @(d) c = c + 8'd1;\n\
             initial begin #1 $display(\"c=%0d d=%b\", c, d); $finish; end\nendmodule\n",
            "c=0 d=z\n",
        ),
        (
            "module top; wire s; assign s = 1'bz; logic d; assign d = s;\n\
             reg [7:0] c = 8'd0; always @(d) c = c + 8'd1;\n\
             reg [7:0] cs = 8'd0; always @(s) cs = cs + 8'd1;\n\
             initial begin #1 $display(\"c=%0d cs=%0d d=%b s=%b\", c, cs, d, s); $finish; end\nendmodule\n",
            "c=0 cs=0 d=z s=z\n",
        ),
        // a child module's computed output, both waiters
        (
            "module child(input [3:0] i, output [3:0] o); assign o = i + 1;\n\
             always @(o) $display(\"CO %0t o=%0d\", $time, o);\nendmodule\n\
             module top; reg [3:0] r; wire [3:0] o; child u(.i(r), .o(o));\n\
             always @(o) $display(\"TO %0t o=%0d\", $time, o);\n\
             initial begin #0 r = 1; #10 $finish; end\nendmodule\n",
            "TO 0 o=2\nCO 0 o=2\n",
        ),
    ]);
    check_sorted(
        "module top; reg [3:0] r; wire w = ^r; wire e = (r == 4'd2);\n\
         initial $display(\"I0 w=%b e=%b\", w, e);\n\
         always @(w) $display(\"W %0t w=%b\", $time, w);\n\
         always @(e) $display(\"E %0t e=%b\", $time, e);\n\
         initial begin #1 r = 4'd2; #10 $finish; end\nendmodule\n",
        "I0 w=x e=x\nE 1 e=1\nW 1 w=1\n",
    );
}

/// The wrong wake stored a value: `always @(w) q = w;` ran at time 0 with `w = x` and a
/// `reg q = 9` read x at `#0`; a counter ran once too often.
#[test]
fn the_dropped_wake_no_longer_stores_x_or_counts() {
    check(&[
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1; reg [3:0] q = 4'd9;\n\
             always @(w) q = w;\n\
             initial begin #0 $display(\"q0=%0d\", q); #1 r = 2; #1 $display(\"q=%0d\", q); #10 $finish; end\nendmodule\n",
            "q0=9\nq=3\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1; reg [3:0] q = 4'd9;\n\
             always @(w) q = w;\n\
             initial begin #0 $display(\"q0=%0d\", q); #1 r = 2; #1 $display(\"q=%0d\", q); #10 $finish; end\n\
             initial begin r = 4'bxxxx; end\nendmodule\n",
            "q0=9\nq=3\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1; integer n = 0;\n\
             always @(w) n = n + 1;\n\
             initial begin #1 r = 2; #1 $display(\"n=%0d\", n); #10 $finish; end\nendmodule\n",
            "n=1\n",
        ),
    ]);
}

/// A delayed driver holds x until its first write lands (`cont_assign_runtime_delay.rs`);
/// that x is not an event.
#[test]
fn a_delayed_drivers_initial_x_is_not_an_event() {
    check(&[
        (
            "module top; reg a; wire b; assign #5 b = a;\n\
             always @(b) $display(\"B %0t b=%b\", $time, b);\n\
             initial begin a = 0; #10 a = 1; #10 $finish; end\nendmodule\n",
            "B 5 b=0\nB 15 b=1\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w; assign #0 w = r;\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
             initial begin r = 1; #10 r = 2; #10 $finish; end\nendmodule\n",
            "W 0 w=1\nW 10 w=2\n",
        ),
        (
            "module top; reg a; wire w; buf #2 g(w, a);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin a = 0; #10 a = 1; #10 $finish; end\nendmodule\n",
            "W 2 w=0\nW 12 w=1\n",
        ),
        (
            "module top; wire b; assign #5 b = 1'b1;\n  initial $display(\"I0 b=%b\", b);\n\
             always @(b) $display(\"B %0t b=%b\", $time, b);\n  initial #10 $finish;\nendmodule\n",
            "I0 b=x\nB 5 b=1\n",
        ),
        (
            "module top; wire w; buf #2 g(w, 1'b1);\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n  initial #10 $finish;\nendmodule\n",
            "I0 w=x\nW 2 w=1\n",
        ),
        (
            "module top; reg a; wire b; assign #5 b = a;\n  initial $display(\"I0 b=%b\", b);\n\
             always @(b) $display(\"B %0t b=%b\", $time, b);\n\
             initial begin a = 0; #10 $finish; end\nendmodule\n",
            "I0 b=x\nB 5 b=0\n",
        ),
    ]);
}

/// A settled value with a definite bit ANYWHERE keeps its time-0 wake: the whole
/// constant, a partially-x `&`, a partially-z concat, a definite bit beside x bits.
#[test]
fn a_settled_value_with_a_definite_bit_still_wakes() {
    check(&[
        (
            "module top; wire w = 1'b1;\n  always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial #10 $finish;\nendmodule\n",
            "W 0 w=1\n",
        ),
        (
            "module top; wire w = 1'b0;\n  always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial #10 $finish;\nendmodule\n",
            "W 0 w=0\n",
        ),
        (
            "module top; parameter K = 4'd7; wire [3:0] w = K;\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n  initial #10 $finish;\nendmodule\n",
            "W 0 w=7\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r & 4'b0011;\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 4'b1111; #10 $finish; end\nendmodule\n",
            "W 0 w=00xx\nW 1 w=0011\n",
        ),
        (
            "module top; reg r; wire [3:0] w = {r, 3'b101};\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 0; #10 $finish; end\nendmodule\n",
            "W 0 w=x101\nW 1 w=0101\n",
        ),
        (
            "module top; reg [3:0] r; wire [7:0] w = {4'd0, r};\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin #1 r = 4'd2; #10 $finish; end\nendmodule\n",
            "W 0 w=0000xxxx\nW 1 w=00000010\n",
        ),
        (
            "module top; wire [1:0] w = {1'bz, 1'b0};\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n  initial #10 $finish;\nendmodule\n",
            "I0 w=z0\nW 0 w=z0\n",
        ),
        (
            "module top; wire [1:0] w = 2'b1x;\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n  initial #10 $finish;\nendmodule\n",
            "I0 w=1x\nW 0 w=1x\n",
        ),
        // a declaration initializer's value reaches a computed net as an event
        (
            "module top; reg [3:0] r = 1; wire [3:0] w = r + 1;\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%0d\", $time, w);\n\
             initial begin #10 r = 3; #10 $finish; end\nendmodule\n",
            "I0 w=0010\nW 0 w=2\nW 10 w=4\n",
        ),
        // a gate whose first-batch write makes it definite
        (
            "module top; reg a, b; wire w; and g(w, a, b);\n  initial $display(\"I0 w=%b\", w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin b = 0; #1 a = 1; #10 $finish; end\nendmodule\n",
            "I0 w=x\nW 0 w=0\n",
        ),
        (
            "module top; reg a, b; wire w; and g(w, a, b);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial begin a = 1; b = 1; #10 a = 0; #10 $finish; end\nendmodule\n",
            "W 0 w=1\nW 10 w=0\n",
        ),
        // a definite term in the list wakes the process, which then reads the x one
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1; wire [3:0] c = 4'd3;\n\
             always @(w or c) $display(\"WC %0t w=%0d c=%0d\", $time, w, c);\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "WC 0 w=x c=3\nWC 1 w=3 c=3\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1; wire [3:0] c = 4'd3; wire [3:0] s = w | c;\n\
             always @(s) $display(\"S %0t s=%b\", $time, s);\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "S 0 s=xx11\nS 1 s=0011\n",
        ),
        // a constant through a port
        (
            "module child(input i); always @(i) $display(\"CI %0t i=%b\", $time, i); endmodule\n\
             module top; child u(.i(1'b1)); initial #10 $finish; endmodule\n",
            "CI 0 i=1\n",
        ),
    ]);
    check_sorted(
        "module top; wire a = 1'b0; wire w = a | 1'b1;\n\
         always @(w) $display(\"W %0t w=%b\", $time, w);\n\
         always @(a) $display(\"A %0t a=%b\", $time, a);\n  initial #10 $finish;\nendmodule\n",
        "A 0 a=0\nW 0 w=1\n",
    );
}

/// Edge waiters, `wait` and an in-line `@(w)` are untouched.
#[test]
fn edges_wait_and_inline_event_controls_are_unchanged() {
    check(&[
        (
            "module top; reg r; wire w = r;\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             always @(negedge w) $display(\"N %0t\", $time);\n\
             initial begin #0 r = 1; #5 r = 0; #5 $finish; end\nendmodule\n",
            "P 0\nN 5\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
             always @(posedge w[0]) $display(\"P0 %0t\", $time);\n\
             initial begin #0 r = 2; #5 r = 1; #5 $finish; end\nendmodule\n",
            "P0 0\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
             initial begin wait(w); $display(\"WAIT %0t w=%0d\", $time, w); end\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "WAIT 1 w=3\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] w = r + 1;\n\
             initial begin @(w); $display(\"GOT %0t w=%0d\", $time, w); end\n\
             initial begin #1 r = 2; #10 $finish; end\nendmodule\n",
            "GOT 1 w=3\n",
        ),
    ]);
}

/// An unpacked array is ONE net with one word per element, and the first shape of the
/// filter read element 0 alone (`read_net(net, None)`) — a review cell with `a[0]`
/// undriven lost the whole array's time-0 wake and `always @* q = a[1];` never ran. The
/// question is asked of every element: one definite element keeps the wake; an array
/// whose every element is x drops it.
#[test]
fn an_unpacked_array_keeps_its_wake_when_any_element_is_definite() {
    check(&[
        (
            "module top; reg [3:0] r; wire [3:0] a [0:1]; reg [3:0] q = 4'd9;\n\
             assign a[1] = 4'd5;\n\
             always @* begin q = a[1]; $display(\"C %0t q=%b a0=%b\", $time, q, a[0]); end\n\
             initial begin #1 r = 0; #10 $display(\"END q=%b\", q); $finish; end\nendmodule\n",
            "C 0 q=0101 a0=zzzz\nEND q=0101\n",
        ),
        (
            "module top; reg [3:0] r; logic [3:0] m [0:1]; reg [3:0] q = 4'd9;\n\
             assign m[1] = 4'd6;\n\
             always @(*) q = m[1] + 1;\n\
             initial begin #1 $display(\"T1 q=%b\", q); r = 0; #10 $display(\"END q=%b\", q); $finish; end\nendmodule\n",
            "T1 q=0111\nEND q=0111\n",
        ),
        (
            "module top; reg [3:0] in; wire [3:0] lut [0:3]; reg [3:0] q = 4'd9;\n\
             genvar i;\n\
             generate for (i = 0; i < 4; i = i + 1) begin : g\n\
               if (i == 0) assign lut[i] = in + 1; else assign lut[i] = i * 3;\n\
             end endgenerate\n\
             always @* q = lut[2];\n\
             initial begin #2 $display(\"T2 q=%0d\", q); #10 $finish; end\nendmodule\n",
            "T2 q=6\n",
        ),
        (
            "module top; reg [3:0] r; wire [3:0] a [0:3]; reg [3:0] q = 4'd9; reg [3:0] q2;\n\
             assign a[0] = r + 1;\n\
             assign a[2] = 4'd5;\n\
             always @* q = a[2];\n\
             always_comb q2 = a[2];\n\
             initial begin #0 $display(\"Z q=%b q2=%b\", q, q2); #1 r = 0; #10 $display(\"END q=%b q2=%b\", q, q2); $finish; end\nendmodule\n",
            "Z q=0101 q2=0101\nEND q=0101 q2=0101\n",
        ),
        // every element x: no wake, `q` keeps its initializer until `r` is written
        (
            "module top; reg [3:0] r; wire [3:0] a [0:1]; reg [3:0] q = 4'd9;\n\
             assign a[0] = r + 1;\n\
             assign a[1] = r + 2;\n\
             always @* q = a[1];\n\
             initial begin #0 $display(\"Z q=%0d\", q); #1 r = 0; #1 $display(\"END q=%0d\", q); #10 $finish; end\nendmodule\n",
            "Z q=9\nEND q=2\n",
        ),
    ]);
}

/// The rule is written twice (`arm_processes` and `arm_t0`): the headline cells on
/// every backend.
#[test]
fn every_backend_drops_the_same_wake() {
    for be in ["interp", "vm", "native"] {
        for (src, want) in [
            (plus_one("#0 r = 2;"), "W 0 w=3\n"),
            (
                "module top; reg [3:0] r; wire [3:0] w = r + 1; reg [3:0] q = 4'd9;\n\
                 always @(w) q = w;\n\
                 initial begin #0 $display(\"q0=%0d\", q); #1 r = 2; #1 $display(\"q=%0d\", q); #10 $finish; end\nendmodule\n"
                    .to_string(),
                "q0=9\nq=3\n",
            ),
            (
                "module top; reg a; wire b; assign #5 b = a;\n\
                 always @(b) $display(\"B %0t b=%b\", $time, b);\n\
                 initial begin a = 0; #10 a = 1; #10 $finish; end\nendmodule\n"
                    .to_string(),
                "B 5 b=0\nB 15 b=1\n",
            ),
            (
                "module top; wire w = 1'b1;\n  always @(w) $display(\"W %0t w=%b\", $time, w);\n\
                 initial #10 $finish;\nendmodule\n"
                    .to_string(),
                "W 0 w=1\n",
            ),
        ] {
            let (got, ok) = vita_on(&src, Some(be));
            assert!(ok, "{be}: expected exit 0, got:\n{got}");
            assert_eq!(got, want, "{be}: design:\n{src}");
        }
    }
}
