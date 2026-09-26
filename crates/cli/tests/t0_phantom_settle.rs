//! A driver that reads a declaration-initialised variable no longer makes events out of
//! the PHANTOM value it settled on before the initializer landed (ROADMAP §2
//! Delays/events, "the time-0 settle evaluates a driver that reads a variable BEFORE that
//! variable's declaration initializer … lands"). The mechanism and its measurement:
//! `sim_engine::t0_edge` (the phantom-hop paragraph).
//!
//! The time-0 settle runs before the declaration initializers, so `reg r = 1; wire w =
//! (r !== 1'b1);` settled `w` to 1 (`x !== 1`) and the first delta's settle brought it to
//! 0 — the write funnel folded both hops into the edge mask, and `always @(posedge w)`
//! printed `P 0`, `always_ff @(posedge w) c <= c + 1` counted one, `always @(posedge w or
//! negedge q)` ran once at 0, a child's `always @(posedge i)` on that port, a copy `wire c
//! = w;`, `logic w; assign w = …`, `$isunknown(r)`, `int r = 1; (r !== 1)` and a chain
//! `wire v = w ? 1'b0 : 1'b1;` all did the same, where both oracles (iverilog 13.0
//! `-g2012`, verilator 5.052 `--binary --timing`) print only the level line `W 0 w=0`.
//! IEEE 1800 §6.21 puts the initializer before any process starts, so both kernels
//! (`arm_processes`, `arm_t0`) now RE-SETTLE the drivers the initializers' writes marked,
//! after the initializer bodies, and assign each dirty edge-target net's mask from the bit
//! it held before the first settle to the bit it holds after the re-settle. Membership is
//! untouched (the level waiter still runs once), the copy-net repair and suppression, the
//! settle-constant edge clear and the x-drop run after it as before.
//!
//! The settle's record is then DELIVERED at the start of the run (`take_t0_wakes`, both
//! kernels), before the first Active batch, and the processes it wakes are held and queued
//! after that batch has run and its own writes have propagated, ahead of what those
//! writes woke. Both oracles print the `initial` bodies first, then the settle's wakes,
//! then the batch-write wakes: `initial $display("I")` before `always @(w) $display("W")`;
//! `always @(w) x = 5;` before an `always @(s)` that `initial s = 1;` woke (it reads
//! `x=5`); and an in-body wait armed in the first batch does not see the settle's edge
//! (`initial begin @(negedge w); … end` prints nothing). PRE printed `W` first and `x=0`
//! exactly on the phantom designs and on a constant driver alike, and let the in-body
//! wait fall through on `reg r = 0; wire w = r | 1'b1;`.
//!
//! Every expected text below is printed by both oracles, except the `N 0` lines: vita's
//! rule for the settle of a variable-reading driver is the funnel's transition rule
//! (`z → 1` posedge, `z → 0` negedge, IEEE §9.4.2), on which the oracles split — iverilog
//! fires `N 0 w=10` on `reg r = 1; wire [1:0] w = {r, 1'b0};` and nothing on `(r !==
//! 1'b1)` (functor-decided), verilator holds no `z` (2-state) — and which §4.5.534 left
//! as it was (ROADMAP §2 Oracle splits). Those lines are pinned as vita's value, marked.
//!
//! Not pinned, measured and left as they are (PRE = POST unless said):
//! - A LEVEL wait armed in an Active batch after a write made earlier in that batch no longer
//!   sees it (§4.5.537, `same_batch_wait.rs`); an EDGE wait still does — `initial #5 r = 1;`
//!   before `initial begin #5 @(posedge r); $display("late"); end` prints `late 5`, both
//!   oracles nothing — recorded in ROADMAP §2 with its prerequisite (the same-time resume
//!   order).
//! - An `always_comb` reading a settled net runs ONCE at time 0 (it arms its sensitivity
//!   after that run, which follows the delivery): verilator once, iverilog twice, on a
//!   constant driver and on `r + 1` of an initialised `r` alike (PRE: twice on the
//!   constant, once on the initialised). `obs_procs.rs` derives its `always_comb` count
//!   from this.
//! - `reg r; wire w = (r === 1'bx); initial r = 0;` — the FIRST-BATCH half: the phantom is
//!   real until the `initial` runs, and iverilog runs the `initial` before the functor's
//!   first propagation (§2 start-order table). PRE = POST: `P 0`, `N 0`, `W 0 w=0`.
//! - `reg clk = 0; assign nc = ~clk; always @(posedge nc)` prints `Pnc 0` (`z → 1`); both
//!   oracles are silent beside an `always #5` toggler, verilator fires on `wire w = ~r;`
//!   beside an `always @(w)`. Oracle split, PRE = POST.
//! - `wire a = 1'b1; reg r = a;` reads `r=1` (verilator `1`, iverilog `z`), and `reg r0 =
//!   1; wire a = (r0 !== 1'b1); reg r1 = a;` reads `r1=1` (iverilog `1`, verilator `0`):
//!   an initializer reading a net is a §4.7 race, PRE = POST.
//! - A `bit`-typed net is not a continuous-assign destination (E3018), an unpacked element
//!   level wait and a `tri1` net are E3009 — the `bit`/array/`tri1` cells are unexercised.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_t0p_{}_{n}.sv", std::process::id()));
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

const THREE_WAITERS: &str = "always @(posedge w) $display(\"P %0t\", $time);\n\
always @(negedge w) $display(\"N %0t\", $time);\n\
always @(w) $display(\"W %0t w=%b\", $time, w);";

/// One declaration line, the three waiters on `w`, a `#3 $finish` watchdog.
fn cell(decl: &str) -> String {
    format!("module top; {decl}\n{THREE_WAITERS}\ninitial #3 $finish;\nendmodule\n")
}

/// The row's design: `w` settles to 1 on the default `x`, to 0 once `r = 1` has landed.
const ROW: &str = "reg r = 1; wire w = (r !== 1'b1);";

#[test]
fn the_phantom_value_of_an_initialised_variables_driver_makes_no_posedge() {
    // `N 0` is vita's `z → 0` (marked in the module doc); `P 0` was the phantom's.
    check(&[
        (&cell(ROW), "N 0\nW 0 w=0\n"),
        (
            &cell("reg r = 1; logic w; assign w = (r !== 1'b1);"),
            "N 0\nW 0 w=0\n",
        ),
        (
            &cell("reg [3:0] r = 4'd5; wire w = $isunknown(r);"),
            "N 0\nW 0 w=0\n",
        ),
        (&cell("int r = 1; wire w = (r !== 1);"), "N 0\nW 0 w=0\n"),
        // The other direction: settled to 0 on the default, 1 once `r = 0` landed —
        // the phantom's `N 0` is gone, `P 0` is the `z → 1` both oracles fire for a
        // definite settle (verilator prints it here, iverilog's compare functor does not).
        (&cell("reg r = 0; wire w = (r === 1'b0);"), "P 0\nW 0 w=1\n"),
    ]);
}

#[test]
fn a_chain_a_copy_and_a_port_take_the_settled_value_only() {
    check(&[
        (
            "module top; reg r = 1; wire a = (r !== 1'b1); wire b = ~a;\n\
             always @(posedge b) $display(\"Pb %0t\", $time);\n\
             always @(negedge b) $display(\"Nb %0t\", $time);\n\
             always @(b) $display(\"Wb %0t b=%b\", $time, b);\n\
             always @(posedge a) $display(\"Pa %0t\", $time);\n\
             always @(negedge a) $display(\"Na %0t\", $time);\n\
             always @(a) $display(\"Wa %0t a=%b\", $time, a);\n\
             initial #3 $finish;\nendmodule\n",
            "Pb 0\nWb 0 b=1\nNa 0\nWa 0 a=0\n",
        ),
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); wire v = w ? 1'b0 : 1'b1;\n\
             always @(posedge v) $display(\"Pv %0t\", $time);\n\
             always @(negedge v) $display(\"Nv %0t\", $time);\n\
             always @(v) $display(\"Wv %0t v=%b\", $time, v);\n\
             initial #3 $finish;\nendmodule\n",
            "Pv 0\nWv 0 v=1\n",
        ),
        (
            "module top; reg r = 1; wire c = (r !== 1'b1); wire w = c;\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             always @(negedge w) $display(\"N %0t\", $time);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\n\
             initial #3 $finish;\nendmodule\n",
            "N 0\nW 0 w=0\n",
        ),
        (
            "module c(input i);\n\
             always @(posedge i) $display(\"cP %0t\", $time);\n\
             always @(negedge i) $display(\"cN %0t\", $time);\n\
             always @(i) $display(\"cW %0t i=%b\", $time, i);\nendmodule\n\
             module top; reg r = 1; wire w = (r !== 1'b1); c u(.i(w));\n\
             initial #3 $finish;\nendmodule\n",
            "cN 0\ncW 0 i=0\n",
        ),
    ]);
}

#[test]
fn a_counter_a_mixed_edge_list_and_a_stored_level_read_are_unchanged_by_the_phantom() {
    check(&[
        // `always_ff @(posedge w)` no longer counts the phantom; the negedge one counts
        // vita's `z → 0` (both oracles `c=0 d=0`; `d=1` is the marked split).
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); int c = 0; int d = 0;\n\
             always_ff @(posedge w) c <= c + 1; always_ff @(negedge w) d <= d + 1;\n\
             initial #1 $display(\"c=%0d d=%0d\", c, d);\ninitial #3 $finish;\nendmodule\n",
            "c=0 d=1\n",
        ),
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); reg q = 1;\n\
             always @(posedge w or negedge q) $display(\"E %0t w=%b q=%b\", $time, w, q);\n\
             initial begin #2 q = 0; end\ninitial #3 $finish;\nendmodule\n",
            "E 2 w=0 q=0\n",
        ),
        (
            "module top; reg q = 9; reg [3:0] cnt = 0; reg r = 1; wire w = (r !== 1'b1);\n\
             always @(w) begin q = w; cnt = cnt + 1; end\n\
             initial #1 $display(\"q=%b cnt=%0d\", q, cnt);\ninitial #3 $finish;\nendmodule\n",
            "q=0 cnt=1\n",
        ),
        (
            "module top; reg r = 1; wire w = (r !== 1'b1);\n\
             initial $display(\"t0 w=%b\", w); initial #0 $display(\"t0z w=%b\", w);\n\
             initial #3 $finish;\nendmodule\n",
            "t0 w=0\nt0z w=0\n",
        ),
    ]);
}

#[test]
fn the_first_batch_runs_before_the_settles_wakes_on_a_phantom_design_too() {
    check(&[
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); always @(w) $display(\"W\"); \
             initial $display(\"I\"); initial #3 $finish;\nendmodule\n",
            "I\nW\n",
        ),
        (
            "module top; reg r = 0; wire [1:0] w = {r, 1'b1}; always @(w) $display(\"W\"); \
             initial $display(\"I\"); initial #3 $finish;\nendmodule\n",
            "I\nW\n",
        ),
        (
            "module top; reg r = 0; wire [1:0] w = {r, 1'b1}; always @(posedge w) $display(\"P\"); \
             initial $display(\"I\"); initial #3 $finish;\nendmodule\n",
            "I\nP\n",
        ),
    ]);
}

#[test]
fn a_definite_settle_a_delayed_driver_and_an_initializer_chain_are_unchanged() {
    check(&[
        // §4.5.534's two-oracle cells: a variable-reading driver whose bit 0 settles
        // definite keeps its edge either way.
        (
            "module top; reg r = 0; wire [1:0] w = {r, 1'b1};\n\
             always @(posedge w) $display(\"P %0t w=%b\", $time, w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\ninitial #3 $finish;\nendmodule\n",
            "P 0 w=01\nW 0 w=01\n",
        ),
        (
            &cell("reg r = 1; wire [1:0] w = {r, 1'b0};"),
            "N 0\nW 0 w=10\n",
        ),
        // A delayed driver's first write lands at `#d` with the settled RHS (iverilog).
        (
            &cell("reg r = 1; wire w; assign #2 w = (r !== 1'b1);"),
            "N 2\nW 2 w=0\n",
        ),
        // `r + 1` settles on no definite bit and takes its value after the initializer:
        // level once, and the `z → 1` posedge (verilator's text).
        (
            &cell("reg [3:0] r = 2; wire [3:0] w = r + 1;"),
            "P 0\nW 0 w=0011\n",
        ),
        // An initializer chain through a computed net: `r1` reads the value `a` held
        // when its initializer ran (a §4.7 race, unchanged); `w` makes no phantom event.
        (
            "module top; reg r0 = 1; wire a = (r0 !== 1'b1); reg r1 = a; wire w = (r1 !== 1'b1);\n\
             initial #1 $display(\"r1=%b w=%b\", r1, w);\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\ninitial #3 $finish;\nendmodule\n",
            "W 0 w=0\nr1=1 w=0\n",
        ),
    ]);
}

#[test]
fn an_oscillator_that_opens_after_the_initializer_is_the_same_delta_limit_fatal() {
    // With `r = x` the loop settles on `x`; once `r = 1` has landed it oscillates. The
    // re-settle inside arming reports it exactly as the first delta did.
    let (s, ok) = vita_on(
        "module top; reg r = 1; wire a, b; assign a = r ? ~b : 1'b0; assign b = (a === 1'b1);\n\
         always @(a) $display(\"A %0t\", $time); initial #3 $finish;\nendmodule\n",
        None,
    );
    assert!(!ok, "expected a non-zero exit, got:\n{s}");
    assert!(s.contains("F-RUN-NO-CONVERGE"), "{s}");
    assert!(!s.contains("A 0"), "{s}");
}

#[test]
fn the_settles_wakes_run_after_the_first_batch_and_before_the_batch_write_wakes() {
    check(&[
        // Both oracles: `always @(w) x = 5` (settle-woken) runs before `always @(s)`
        // (woken by `initial s = 1`), which therefore reads `x=5`; declaration order
        // would run `always @(s)` first. PRE printed `x=5` here only through the
        // phantom's first-delta change, and `x=0` on a constant `wire w = 1'b0;`.
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); reg s = 0; integer x = 0; \
             initial s = 1;\n\
             always @(s) $display(\"SW %0t x=%0d\", $time, x); always @(w) x = 5;\n\
             initial #3 $finish;\nendmodule\n",
            "SW 0 x=5\n",
        ),
        (
            "module top; wire w = 1'b0; reg s = 0; integer x = 0; initial s = 1;\n\
             always @(s) $display(\"SW %0t x=%0d\", $time, x); always @(w) x = 5;\n\
             initial #3 $finish;\nendmodule\n",
            "SW 0 x=5\n",
        ),
        (
            "module top; reg [3:0] r = 3; wire [3:0] w = r + 1; reg s = 0;\n\
             initial begin s = 1; $display(\"I1 %0t w=%b\", $time, w); end\n\
             always @(s) $display(\"SW %0t s=%b w=%b\", $time, s, w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\ninitial #3 $finish;\nendmodule\n",
            "I1 0 w=0100\nW 0 w=0100\nSW 0 s=1 w=0100\n",
        ),
        (
            "module top; wire [1:0] w = 2'b01; reg s = 0;\n\
             initial begin s = 1; $display(\"I1 %0t w=%b\", $time, w); end\n\
             always @(s) $display(\"SW %0t s=%b w=%b\", $time, s, w);\n\
             always @(w) $display(\"W %0t w=%b\", $time, w);\ninitial #3 $finish;\nendmodule\n",
            "I1 0 w=01\nW 0 w=01\nSW 0 s=1 w=01\n",
        ),
        // …and before an `always @(u)` that the batch's write reaches one settle later,
        // through `wire u = ~s;` (both oracles `x=5`; PRE read `x` on a constant `w`).
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); reg s; wire u = ~s; integer x;\n\
             always @(u) $display(\"U %0t u=%b x=%0d\", $time, u, x); always @(w) x = 5;\n\
             initial begin s = 0; $display(\"I %0t\", $time); end\ninitial #3 $finish;\nendmodule\n",
            "I 0\nU 0 u=1 x=5\n",
        ),
        (
            "module top; wire w = 1'b0; reg s; wire u = ~s; integer x;\n\
             always @(u) $display(\"U %0t u=%b x=%0d\", $time, u, x); always @(w) x = 5;\n\
             initial begin s = 0; $display(\"I %0t\", $time); end\ninitial #3 $finish;\nendmodule\n",
            "I 0\nU 0 u=1 x=5\n",
        ),
        // `always @(w) q = ~q;` (settle-woken) runs after the `initial` that reads `q`.
        (
            "module top; reg r = 1; wire w = (r !== 1'b1); reg q = 0;\n\
             always @(w) begin q = ~q; $display(\"Q %0t q=%b\", $time, q); end\n\
             initial $display(\"I %0t q=%b\", $time, q);\ninitial #3 $finish;\nendmodule\n",
            "I 0 q=0\nQ 0 q=1\n",
        ),
    ]);
}

#[test]
fn a_wait_armed_in_the_first_batch_does_not_see_the_settles_edge() {
    check(&[
        (
            "module top; reg r = 1; wire w = (r !== 1'b1);\n\
             initial begin @(negedge w); $display(\"gotN %0t\", $time); end\n\
             initial begin @(posedge w); $display(\"gotP %0t\", $time); end\n\
             initial #3 $finish;\nendmodule\n",
            "",
        ),
        (
            "module top; reg r = 0; wire w = r | 1'b1;\n\
             initial begin @(posedge w); $display(\"got %0t\", $time); end\n\
             initial #3 $finish;\nendmodule\n",
            "",
        ),
        // …while the static waiter on the same net fires, and `wait (w)` resumes on
        // the value.
        (
            "module top; reg r = 0; wire w = r | 1'b1;\n\
             always @(posedge w) $display(\"P %0t\", $time);\n\
             initial begin wait (w); $display(\"gotWT %0t\", $time); end\n\
             initial #3 $finish;\nendmodule\n",
            "gotWT 0\nP 0\n",
        ),
    ]);
}

#[test]
fn an_always_comb_runs_once_at_time_zero_and_a_heap_size_driver_takes_the_initialised_size() {
    check(&[
        (
            "module top; logic [7:0] c = 0; wire [7:0] d; logic [7:0] e; assign d = c + 8'd1;\n\
             always_comb begin e = d ^ 8'hA5; $display(\"C %0t d=%0d e=%0d\", $time, d, e); end\n\
             initial #3 $finish;\nendmodule\n",
            "C 0 d=1 e=164\n",
        ),
        (
            "module top; wire [7:0] d = 8'd1; logic [7:0] e;\n\
             always_comb begin e = d ^ 8'hA5; $display(\"C %0t d=%0d e=%0d\", $time, d, e); end\n\
             initial #3 $finish;\nendmodule\n",
            "C 0 d=1 e=164\n",
        ),
        // A heap initializer leaves no net dirt; the re-settle is keyed on the
        // initializer LIST so `q.size()` still takes the initialised size before any
        // process runs (verilator's text; iverilog aborts on the design).
        (
            "module top; int q[] = new[3]; wire [31:0] n = q.size();\n\
             always @(posedge n[0]) $display(\"P %0t n=%0d\", $time, n);\n\
             always @(negedge n[0]) $display(\"N %0t n=%0d\", $time, n);\n\
             always @(n) $display(\"W %0t n=%0d\", $time, n);\n\
             initial $display(\"I %0t n=%0d\", $time, n);\ninitial #3 $finish;\nendmodule\n",
            "I 0 n=3\nP 0 n=3\nW 0 n=3\n",
        ),
        // The FIRST-BATCH half keeps its lines (§2 start-order table): the phantom is
        // real until the `initial` runs, and the settle-woken level waiter runs once,
        // after the batch's write has re-settled `w`, ahead of the negedge that
        // re-settle made.
        (
            &cell("reg r; wire w = (r === 1'bx); initial r = 0;"),
            "P 0\nW 0 w=0\nN 0\n",
        ),
    ]);
}

#[test]
fn every_backend_drops_the_same_phantom() {
    let designs = [
        cell(ROW),
        cell("reg r = 0; wire w = (r === 1'b0);"),
        "module top; reg r = 1; wire w = (r !== 1'b1); int c = 0; int d = 0;\n\
         always_ff @(posedge w) c <= c + 1; always_ff @(negedge w) d <= d + 1;\n\
         initial #1 $display(\"c=%0d d=%0d\", c, d);\ninitial #3 $finish;\nendmodule\n"
            .to_string(),
        "module top; reg r = 1; wire w = (r !== 1'b1); always @(w) $display(\"W\"); \
         initial $display(\"I\"); initial #3 $finish;\nendmodule\n"
            .to_string(),
        "module top; wire w = 1'b0; reg s = 0; integer x = 0; initial s = 1;\n\
         always @(s) $display(\"SW %0t x=%0d\", $time, x); always @(w) x = 5;\n\
         initial #3 $finish;\nendmodule\n"
            .to_string(),
    ];
    let wants = [
        "N 0\nW 0 w=0\n",
        "P 0\nW 0 w=1\n",
        "c=0 d=1\n",
        "I\nW\n",
        "SW 0 x=5\n",
    ];
    for (src, want) in designs.iter().zip(wants) {
        for be in ["interp", "vm", "native"] {
            let (s, ok) = vita_on(src, Some(be));
            assert!(ok, "{be}: expected exit 0, got:\n{s}");
            assert_eq!(s, want, "{be}: design:\n{src}");
        }
    }
}
