//! IEEE 1800-2017 §9.2.2.4: "The always_ff procedure imposes the restriction that it
//! contains one and only one event control and no blocking timing controls."
//!
//! Two legal `always_ff` shapes have no edge list, and vita ran both as an edge process
//! armed on nothing: the body never ran, every read printed `x`, and the only sign was
//! `VITA-W3056` ("always_ff requires an explicit @(edge ...) list").
//!
//! - no header, the one event control in the body: `always_ff begin @(posedge clk)
//!   q <= d; end`;
//! - `always_ff @*` (and `@(*)`) with no event control in the body.
//!
//! Each now lowers through the `always` lane spelled the same way — the self-timed
//! `always`, and `always @*` — so its IR is the plain-`always` twin's, byte for byte
//! (`routed_shapes_lower_to_the_twins_ir`), and only the process label differs.
//!
//! Oracles, measured on the testbench in `TB` (clk, d, rst; five display lines):
//!
//! ```text
//!   i2_inner_only   always_ff begin @(posedge clk) q <= d; end
//!     iverilog 13.0 -g2012 + vvp        c1 q=1  c2 q=0  c3 q=1  r1 q=1  c4 q=1
//!     verilator 5.052 --binary --timing c1 q=1  c2 q=0  c3 q=1  r1 q=1  c4 q=1
//!     sv2v 0.0.13 -> iverilog 13.0      c1 q=1  c2 q=0  c3 q=1  r1 q=1  c4 q=1
//!     vita before                       warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: always_ff
//!                                       requires an explicit @(edge ...) list [in tb.u]
//!                                       c1 q=x  c2 q=x  c3 q=x  r1 q=x  c4 q=x
//!   ff_star         always_ff @* begin q <= d; end
//!     iverilog 13.0                     ff_star.sv:2 warning: Synthesis requires the
//!                                       sensitivity list of an always_ff process to only be
//!                                       edge sensitive. d is missing a pos/negedge.
//!                                       c1 q=1  c2 q=0  c3 q=1  r1 q=1  c4 q=1
//!     verilator 5.052                   %Warning-COMBDLY: ff_star.sv:3:7: Non-blocking
//!                                       assignment '<=' in combinational logic process
//!                                       c1 q=1  c2 q=0  c3 q=1  r1 q=1  c4 q=1
//!     sv2v 0.0.13 -> iverilog 13.0      c1 q=1  c2 q=0  c3 q=1  r1 q=1  c4 q=1
//!     vita before                       the same W3056 line, then x on every line
//! ```
//!
//! The plain-`always` twin of each printed `1 0 1 1 1` before the change too, on vita and
//! on all three tools.
//!
//! The other routed shapes, on the five-line testbench in `TB2` (clk, d, en; q, q2), vita
//! after = the twin = these oracle lines (verilator reads 0 for an unwritten variable,
//! sv2v leaves the undriven `q2` port z; neither is an x oracle):
//!
//! ```text
//!   always_ff begin @(posedge clk) begin q <= d; q2 <= ~d; end end
//!     iverilog / verilator / sv2v       q=1 q2=0 | q=0 q2=1 | q=1 q2=0 | q=0 q2=1 | q=1 q2=0
//!   always_ff begin q2 <= d; @(posedge clk) q <= d; end
//!     iverilog   error: the first statement of an always_ff process must be an event
//!                control statement.
//!     verilator / sv2v                  q=1 q2=1 | q=0 q2=0 | q=1 q2=1 | q=0 q2=0 | q=1 q2=1
//!   always_ff begin : nb @(posedge clk) q <= d; end
//!     iverilog   the same "first statement" error
//!     verilator  q=1 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0
//!     sv2v       q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z
//!   always_ff begin begin @(posedge clk) q <= d; q2 <= ~d; end end
//!     iverilog   the same "first statement" error
//!     verilator / sv2v                  q=1 q2=0 | q=0 q2=1 | q=1 q2=0 | q=0 q2=1 | q=1 q2=0
//!   always_ff begin : nb @(posedge clk) q <= d; if (!en) disable nb; q2 <= d; end
//!     iverilog   the same "first statement" error; sv2v cannot parse `disable`
//!     verilator  q=1 q2=1 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=1
//!     (a `disable` AFTER the event control ends the pass early; the next pass reaches
//!     the `@` again)
//!   always_ff forever @(posedge clk) q <= d;
//!     iverilog   the same "first statement" error, and "warning: A forever statement cannot
//!                be synthesized in an always_ff process."
//!     verilator  q=1 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0
//!     sv2v       q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z
//!   always_ff begin @(negedge clk) q <= d; end
//!     iverilog   q=1 q2=x | q=0 q2=x | q=1 q2=x | q=0 q2=x | q=1 q2=x
//!     verilator  q=0 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0   (misses the
//!                time-0 x->0 negedge; its plain-`always` twin prints the same)
//!     sv2v       q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z
//!   always_ff begin @(posedge clk) q <= #1 d; end
//!     iverilog   q=x q2=x | q=1 q2=x | q=0 q2=x | q=1 q2=x | q=0 q2=x
//!     verilator  q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0
//!     sv2v       q=x q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z
//!   always_ff @(*) q <= d & en;
//!     iverilog   (aborts: assert: elaborate.cc:7280: failed assertion
//!                prb->pin_count() == 1); its plain `always @(*)` twin
//!                q=1 q2=x | q=0 q2=x | q=0 q2=x | q=0 q2=x | q=1 q2=x
//!     verilator  q=1 q2=0 | q=0 q2=0 | q=0 q2=0 | q=0 q2=0 | q=1 q2=0
//!     sv2v       q=1 q2=z | q=0 q2=z | q=0 q2=z | q=0 q2=z | q=1 q2=z
//!   always_ff @* begin q = d; q2 = q & en; end
//!     iverilog   (the same abort); twin q=1 q2=1 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=1
//!     verilator / sv2v                  q=1 q2=1 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=1
//!   always_ff do @(posedge clk) q <= d; while (0);
//!     iverilog   the same "first statement" error; its twin q=1 q2=x | q=0 q2=x | q=1 q2=x |
//!                q=0 q2=x | q=1 q2=x
//!     verilator  q=1 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0
//!     sv2v       q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z
//!     (the parser lowers `do S while (c)` to `begin S; while (c) S; end` with a copy of
//!     `S`; the copy is not counted, so this is one event control, as written)
//! ```
//!
//! Time 0 (`T0_*`): a clock edge at time 0 reaches the self-timed lane exactly as it
//! reaches the plain-`always` twin. Three ways to raise `clk` at time 0 — `clk = 1;` first
//! in the testbench's `initial`, `logic clk = 1;`, `#0 clk = 1;` — each with the header-less
//! block, its twin, and the header form `always_ff @(posedge clk) q <= d;`:
//!
//! ```text
//!                         t1 t2 t3     iverilog   verilator  sv2v
//!   clk = 1;   (t0a)  hl  1  1  0      1 1 0      0 0 0      1 1 0
//!                     tw  1  1  0      1 1 0      0 0 0      1 1 0
//!                     hd  1  1  0      1 1 0      1 1 0      1 1 0
//!   logic clk = 1 (t0b) hl x x 0       x x 0      0 0 0      x x 0
//!   #0 clk = 1; (t0c) hl  1  1  0      1 1 0      1 1 0      1 1 0
//!   always_ff @* q <= d & clk;   t0a 1 0 0 (iverilog aborts; verilator 1 0 0; sv2v 1 0 0)
//!                                t0b x 0 0 (verilator 1 0 0; sv2v x 0 0)
//!                                t0c 1 0 0 (verilator 0 0 0; sv2v 1 0 0)
//! ```
//!
//! (vita columns: after the change; each equals its twin, and the twins equal iverilog.)
//!
//! ## Every other shape that breaks §9.2.2.4 is `VITA-E3061`
//!
//! The event controls are the header plus every `@(…)` statement and every
//! intra-assignment `@` on a non-blocking assignment (`q <= @(e) d`) in the body; a count
//! of 0, or of 2 and more, is an error, and so is a blocking timing control (`#`, `wait`,
//! `wait fork`, `q = #1 d`, `q = @(e) d`). A non-blocking `q <= #1 d` is legal. Two more
//! refusals keep a block from looping without suspending: an event control inside a
//! `fork` (a forked process cannot be the procedure's one event control; its
//! plain-`always` twin `always begin fork @(posedge clk) q <= d; join_none end` grows
//! without bound), and, with no header, an intra-assignment `@` as the only event
//! control (`always_ff q <= @(posedge clk) d;`).
//!
//! With no header, the one `@(…)` must also be reached on EVERY pass: in the body's
//! sequential list or a nested `begin … end` (named or not), the first statement of a
//! `do … while`, or a `forever` body, and after no loop and no `disable` (`break`,
//! `continue`). An `@` inside `if` / `else`, a `case` arm, a `for` / `foreach` /
//! `while` / `repeat` body or a `fork`, or after a loop or a `disable`, is refused. This
//! is STRICTER than the IEEE text, which counts such an `@` as the one event control; the
//! block cannot run (a pass that misses the `@` ends without suspending, and its NBAs pile
//! up: the plain-`always` twin of `begin if (en) @(posedge clk) q <= d; q2 <= d; end`
//! reaches 1.5 GB in 0.69 s), and iverilog refuses it as "the first statement of an
//! always_ff process must be an event control statement." (u1 below; verilator spins).
//! Owner decision (2026-10-06): an error, not a warning. Running it as IEEE allows is
//! ROADMAP §7 FF-MISSED-AT, gated on the engine bounding NBA growth (§5.b NBA-GROWTH) and
//! on a real design using the shape.
//!
//! Owner ruling (2026-10-06): a count of 2 and more is an ERROR, exit 1, though only
//! iverilog refuses it and verilator and sv2v run it (an exception to ER §10.4, which
//! would make a shape the second tool accepts a warning at most): "loud where IEEE says
//! error". A count of 0 is an error too.
//!
//! ```text
//!   f_twoev (palways/chk)  always_ff @(posedge clk) begin q <= d; @(posedge clk) q <= ~d; end
//!     iverilog 13.0         f_twoev.sv:4: error: an event control is not allowed in an
//!                           always_comb, always_ff or always_latch process.
//!                           f_twoev.sv:2: error: there must only be a single event control
//!                           and no blocking delays in an always_ff process.
//!                           Elaboration failed
//!     verilator 5.052 --lint-only -Wall --timing   (nothing, rc=0)
//!     under TB (i2_twoev): verilator --binary  c1 q=1 c2 q=1 c3 q=1 r1 q=1 c4 q=0
//!                          sv2v -> iverilog    c1 q=1 c2 q=1 c3 q=1 r1 q=1 c4 q=0
//!     vita before           exit 0, no diagnostic; under TB the same five lines
//!   i2_noctl               always_ff q <= d;
//!     iverilog 13.0         i2_noctl.sv:2: error: the first statement of an always_ff
//!                           process must be an event control statement.
//!     verilator 5.052       %Warning-COMBDLY: i2_noctl.sv:2:15: Non-blocking assignment '<='
//!                           in combinational logic process
//!                           c1 q=1 c2 q=0 c3 q=1 r1 q=1 c4 q=1
//!     sv2v -> iverilog      i2_noctl.sv2v.v:11: error: always process does not have any
//!                           delay. / : A runtime infinite loop will occur.
//!     vita before           W3056, x on every line
//!   i2_delay               always_ff @(posedge clk) begin #1 q <= d; end
//!     iverilog 13.0         i2_delay.sv:3: error: a blocking delay is not allowed in an
//!                           always_comb, always_ff or always_latch process.
//!                           i2_delay.sv:2: error: there must only be a single event control
//!                           and no blocking delays in an always_ff process.
//!     verilator 5.052       c1 q=0 c2 q=0 c3 q=1 r1 q=1 c4 q=1
//!     sv2v -> iverilog      c1 q=x c2 q=0 c3 q=1 r1 q=1 c4 q=1
//!     vita before           c1 q=x c2 q=1 c3 q=0 r1 q=1 c4 q=1   (the input-port hop lands
//!                           after its batch: ROADMAP §2 "A CA hop lands after the whole batch")
//!   i2_wait                always_ff @(posedge clk) begin wait (d) q <= ~q; end
//!     iverilog 13.0         i2_wait.sv:3: error: a wait statement is not allowed in an
//!                           always_comb, always_ff or always_latch process. (and the
//!                           "single event control" line)
//!     verilator 5.052       c1 q=1 c2 q=1 c3 q=1 r1 q=1 c4 q=0
//!     sv2v -> iverilog      x on every line
//!     vita before           x on every line
//!   i2_blk_intra           always_ff @(posedge clk) q = #1 d;
//!     iverilog 13.0         i2_blk_intra.sv:2: error: a blocking delay is not allowed in an
//!                           always_comb, always_ff or always_latch process. (and the
//!                           "single event control" line)
//!     verilator 5.052       c1 q=0 c2 q=1 c3 q=0 r1 q=1 c4 q=1
//!     sv2v -> iverilog      c1 q=x c2 q=1 c3 q=0 r1 q=1 c4 q=1
//!     vita before           c1 q=x c2 q=1 c3 q=0 r1 q=1 c4 q=1
//!   intra `@` on an NBA     always_ff @(posedge clk) q <= @(negedge clk) d;   (count 2)
//!     iverilog 13.0         impl_c_nbaev.sv:2: error: A non-blocking assignment cannot be
//!                           synthesized with an event control in an always_ff process.
//!     verilator 5.052       c1 q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0
//!     sv2v -> iverilog      c1 q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z
//!     vita before           c1 q=x q2=x | q=1 q2=x | q=0 q2=x | q=1 q2=x | q=0 q2=x
//!     (and under the review's free-running clock, nbaev_mytb: iverilog the same error;
//!     verilator t=10 q=0 | t=20 q=3 | t=30 q=5 | t=40 q=a | t=50 q=c | t53 q=7; sv2v
//!     t=10 q=3 | t=20 q=5 | t=30 q=a | t=40 q=c | t=50 q=7 | t53 q=7; vita before t=10
//!     q=x | t=20 q=3 | t=30 q=5 | t=40 q=a | t=50 q=c | t53 q=7)
//!   u1  always_ff begin if (en) @(posedge clk) q <= d; q2 <= d; end   (not every pass)
//!     iverilog 13.0         u1_nba_loop.sv:5: error: the first statement of an always_ff
//!                           process must be an event control statement.
//!     verilator 5.052       spins (killed by the watchdog at 20 s)
//!     vita before           W3056, x; the self-timed lane of the second S2 build grew to
//!                           1.5 GB in 0.69 s (6 GB with four NBAs), as the twin does
//!   u2  always_ff begin repeat (cnt) @(posedge clk); q2 <= d; end (cnt 0)
//!     iverilog 13.0         the "first statement" error / warning: A repeat statement
//!                           cannot be synthesized in an always_ff process.
//!     verilator 5.052       q=0 q2=0
//!     vita before           W3056, x; the second S2 build grew past 1.5 GB
//!   always_ff q <= @(posedge clk) d;                  (no statement-level event control)
//!     iverilog 13.0         error: the first statement of an always_ff process must be an
//!                           event control statement. / error: A non-blocking assignment
//!                           cannot be synthesized with an event control in an always_ff
//!                           process.
//!     verilator 5.052       %Error: Internal Error: …:2:20: ../V3Active.cpp:552: Should not
//!                           reach here when walking body without --timing
//!     sv2v -> iverilog      error: always process does not have any delay. / : A runtime
//!                           infinite loop will occur.
//!   always_ff begin fork @(posedge clk) q <= d; join_none end         (`@` in a fork)
//!     iverilog 13.0         error: the first statement of an always_ff process must be an
//!                           event control statement. / A fork/join_none statement cannot be
//!                           synthesized in an always_ff process.
//!     verilator 5.052       %Error-DIDNOTCONVERGE: Active region did not converge after
//!                           '--converge-limit' of 10000 tries
//!     sv2v                  Parse error: missing expected `join`
//!     vita before           W3056, x; the self-timed lane of the first S2 build grew to
//!                           1.5 GB in 1.44 s (`join_any`: 4.8 s), as the twin does
//!   always_ff fork @(posedge clk) q <= d; join                        (`@` in a `join`)
//!     iverilog 13.0         the "first statement" error / error: A fork/join statement
//!                           cannot be synthesized in an always_ff process.
//!     verilator 5.052       q=1 q2=0 | q=0 q2=0 | q=1 q2=0 | q=0 q2=0 | q=1 q2=0
//!     sv2v -> iverilog      q=1 q2=z | q=0 q2=z | q=1 q2=z | q=0 q2=z | q=1 q2=z
//!     vita before           W3056, x (the first S2 build routed it and printed its
//!                           twin's `1 0 1 0 1`; `join` waits, so it was bounded, but a
//!                           `fork` is refused whatever its join)
//! ```
//!
//! Legal, and unchanged:
//!
//! ```text
//!   i2_nba_intra           always_ff @(posedge clk) q <= #1 d;
//!     iverilog 13.0         c1 q=x c2 q=1 c3 q=0 r1 q=1 c4 q=1
//!     verilator 5.052       c1 q=0 c2 q=1 c3 q=0 r1 q=1 c4 q=1   (2-state: not an x oracle)
//!     sv2v -> iverilog      c1 q=x c2 q=1 c3 q=0 r1 q=1 c4 q=1
//!     vita                  c1 q=x c2 q=1 c3 q=0 r1 q=1 c4 q=1   (before and after)
//!   f_ok     always_ff @(posedge clk or negedge rst_n) if (!rst_n) q <= 1'b0; else if (en) q <= d;
//!     iverilog, verilator --lint-only -Wall: nothing
//!   f_noedge always_ff @(a or b) y <= a & b;
//!     iverilog 13.0         f_noedge.sv:2 warning: Synthesis requires the sensitivity list of
//!                           an always_ff process to only be edge sensitive. f_noedge.b is
//!                           missing a pos/negedge. (and the same for f_noedge.a)
//!     verilator --lint-only -Wall: nothing
//!   f_block  always_ff @(posedge clk) begin q1 = d; q2 = q1; end
//!     iverilog: nothing; verilator --lint-only -Wall: %Warning-BLKSEQ: f_block.sv:3:12:
//!     Blocking assignment '=' in sequential logic process (and 4:12)
//!   each: vita exit 0, no diagnostic (before and after)
//! ```
//!
//! Every test counts the EXACT set of codes the run prints (`-Wno-W1017` silences the
//! no-`timescale` warning so that nothing is left out).

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use diag::{LogEvent, LogSink};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design through one-shot `vita` with `-Wno-W1017` and return its exit code
/// and combined output.
fn run(src: &str) -> (Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ffev_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    let f = d.join("t.sv");
    std::fs::write(&f, src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), s)
}

/// Every diagnostic code the run printed (`[VITA-…]`), as a set.
fn codes(out: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for (i, _) in out.match_indices("[VITA-") {
        let rest = &out[i + 1..];
        if let Some(end) = rest.find(']') {
            set.insert(rest[..end].to_string());
        }
    }
    set
}

/// The run printed exactly these codes and no other.
fn expect_codes(out: &str, want: &[&str], what: &str) {
    let want: BTreeSet<String> = want.iter().map(|c| c.to_string()).collect();
    assert_eq!(codes(out), want, "{what}: exact code set:\n{out}");
}

/// The testbench's `$display` lines, in order.
fn values(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| l.contains(" q=") && !l.contains("VITA-"))
        .collect()
}

/// Splice `dut_body` into the module of `tb` (which names it `DUT_BODY`).
fn design(tb: &str, dut_body: &str) -> String {
    tb.replace("DUT_BODY", dut_body)
}

/// The i2 testbench of the grounding: clk, d, rst; five display lines.
const TB: &str = "module dut(input logic clk, d, rst, output logic q);
DUT_BODY
endmodule
module tb;
  logic clk, d, rst; wire q;
  dut u(.clk(clk), .d(d), .rst(rst), .q(q));
  initial begin
    clk=0; d=1; rst=0;
    #1 clk=1; #1 clk=0; $display(\"c1 q=%b\", q);
    d=0; #1 clk=1; #1 clk=0; $display(\"c2 q=%b\", q);
    d=1; #1 clk=1; #1 clk=0; $display(\"c3 q=%b\", q);
    rst=1; #1 $display(\"r1 q=%b\", q);
    rst=0; d=1; #1 clk=1; #1 clk=0; $display(\"c4 q=%b\", q);
    #10 $finish;
  end
endmodule
";

/// A second testbench with an enable and a second output.
const TB2: &str = "module dut(input logic clk, d, en, output logic q, q2);
DUT_BODY
endmodule
module tb;
  logic clk, d, en; wire q, q2;
  dut u(.clk(clk), .d(d), .en(en), .q(q), .q2(q2));
  initial begin
    clk=0; d=1; en=1;
    #1 clk=1; #1 clk=0; $display(\"c1 q=%b q2=%b\", q, q2);
    d=0; #1 clk=1; #1 clk=0; $display(\"c2 q=%b q2=%b\", q, q2);
    d=1; en=0; #1 clk=1; #1 clk=0; $display(\"c3 q=%b q2=%b\", q, q2);
    d=0; en=1; #1 clk=1; #1 clk=0; $display(\"c4 q=%b q2=%b\", q, q2);
    d=1; #1 clk=1; #1 clk=0; $display(\"c5 q=%b q2=%b\", q, q2);
    #10 $finish;
  end
endmodule
";

/// A time-0 testbench: `T0_INIT` raises `clk` at time 0 (or `T0_DECL` declares it 1).
const TB_T0: &str = "module dut(input logic clk, d, output logic q);
DUT_BODY
endmodule
module tb;
  logic clk T0_DECL; logic d = 1; wire q;
  dut u(.clk(clk), .d(d), .q(q));
  initial begin
    T0_INIT
    #1 $display(\"t1 q=%b\", q);
    clk=0; #1 $display(\"t2 q=%b\", q);
    d=0; clk=1; #1 $display(\"t3 q=%b\", q);
    #10 $finish;
  end
endmodule
";

/// The three ways the grounding raised `clk` at time 0: (label, declaration, statement).
const T0_WAYS: [(&str, &str, &str); 3] = [
    ("t0a clk = 1;", "", "clk = 1;"),
    ("t0b logic clk = 1", "= 1", ""),
    ("t0c #0 clk = 1;", "", "#0 clk = 1;"),
];

fn t0_design(way: (&str, &str, &str), dut_body: &str) -> String {
    TB_T0
        .replace("T0_DECL", way.1)
        .replace("T0_INIT", way.2)
        .replace("DUT_BODY", dut_body)
}

/// The `always_ff` body and its plain-`always` twin, column-aligned, for every shape the
/// change routes.
const ROUTED: [(&str, &str); 13] = [
    (
        "  always_ff begin @(posedge clk) q <= d; end",
        "  always    begin @(posedge clk) q <= d; end",
    ),
    (
        "  always_ff begin @(posedge clk) begin q <= d; q2 <= ~d; end end",
        "  always    begin @(posedge clk) begin q <= d; q2 <= ~d; end end",
    ),
    (
        "  always_ff begin q2 <= d; @(posedge clk) q <= d; end",
        "  always    begin q2 <= d; @(posedge clk) q <= d; end",
    ),
    (
        "  always_ff begin : nb @(posedge clk) q <= d; end",
        "  always    begin : nb @(posedge clk) q <= d; end",
    ),
    (
        "  always_ff begin begin @(posedge clk) q <= d; q2 <= ~d; end end",
        "  always    begin begin @(posedge clk) q <= d; q2 <= ~d; end end",
    ),
    (
        "  always_ff begin : nb @(posedge clk) q <= d; if (!en) disable nb; q2 <= d; end",
        "  always    begin : nb @(posedge clk) q <= d; if (!en) disable nb; q2 <= d; end",
    ),
    (
        "  always_ff forever @(posedge clk) q <= d;",
        "  always    forever @(posedge clk) q <= d;",
    ),
    (
        "  always_ff begin @(negedge clk) q <= d; end",
        "  always    begin @(negedge clk) q <= d; end",
    ),
    (
        "  always_ff begin @(posedge clk) q <= #1 d; end",
        "  always    begin @(posedge clk) q <= #1 d; end",
    ),
    (
        "  always_ff @* begin q <= d; end",
        "  always    @* begin q <= d; end",
    ),
    (
        "  always_ff @(*) q <= d & en;",
        "  always    @(*) q <= d & en;",
    ),
    (
        "  always_ff @* begin q = d; q2 = q & en; end",
        "  always    @* begin q = d; q2 = q & en; end",
    ),
    (
        "  always_ff do @(posedge clk) q <= d; while (0);",
        "  always    do @(posedge clk) q <= d; while (0);",
    ),
];

#[derive(Default)]
struct Codes(std::cell::RefCell<Vec<String>>);
impl LogSink for Codes {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0.borrow_mut().push(d.code.code_num().to_string());
        }
    }
}

/// Lex, parse and elaborate `src` in memory; the IR and every diagnostic code raised.
fn elaborate_ir(src: &str) -> (Option<sim_ir::SimIr>, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = Codes::default();
    let (ir, _) =
        elaborate::elaborate_with_timescale(&su.expect("source unit"), &sink, &BTreeMap::new(), -9);
    (ir, sink.0.into_inner())
}

/// `i2_inner_only`: the event control is the body's first statement and there is no
/// header. Before: W3056 and `x` on every line.
#[test]
fn inner_only_prints_the_twins_values() {
    let ff = design(TB, "  always_ff begin\n    @(posedge clk) q <= d;\n  end");
    let tw = design(TB, "  always    begin\n    @(posedge clk) q <= d;\n  end");
    let (rc, out) = run(&ff);
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &[], "i2_inner_only");
    let want = ["c1 q=1", "c2 q=0", "c3 q=1", "r1 q=1", "c4 q=1"];
    assert_eq!(values(&out), want, "i2_inner_only:\n{out}");
    let (rc, tw_out) = run(&tw);
    assert_eq!(rc, Some(0), "{tw_out}");
    assert_eq!(values(&tw_out), want, "plain-always twin:\n{tw_out}");
}

/// `ff_star`: `always_ff @*` is `always @*`. Before: W3056 and `x` on every line.
#[test]
fn ff_star_prints_the_twins_values() {
    let ff = design(TB, "  always_ff @* begin\n    q <= d;\n  end");
    let tw = design(TB, "  always @* begin\n    q <= d;\n  end");
    let (rc, out) = run(&ff);
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &[], "ff_star");
    let want = ["c1 q=1", "c2 q=0", "c3 q=1", "r1 q=1", "c4 q=1"];
    assert_eq!(values(&out), want, "ff_star:\n{out}");
    let (rc, tw_out) = run(&tw);
    assert_eq!(rc, Some(0), "{tw_out}");
    assert_eq!(values(&tw_out), want, "always @* twin:\n{tw_out}");
}

/// Every routed shape elaborates to the SAME `SimIr` as its plain-`always` twin, with no
/// diagnostic on either; the run prints the same lines. The process label (`always_ff`)
/// lives in a side table, not in the IR.
#[test]
fn routed_shapes_lower_to_the_twins_ir() {
    for (ff, tw) in ROUTED {
        let (ff_ir, ff_codes) = elaborate_ir(&design(TB2, ff));
        let (tw_ir, tw_codes) = elaborate_ir(&design(TB2, tw));
        assert!(ff_codes.is_empty(), "{ff}: diagnostics {ff_codes:?}");
        assert!(tw_codes.is_empty(), "{tw}: diagnostics {tw_codes:?}");
        let ff_ir = ff_ir.expect("always_ff elaborates");
        let tw_ir = tw_ir.expect("twin elaborates");
        assert!(ff_ir == tw_ir, "{ff}: IR differs from its twin");
        let (rc, ff_out) = run(&design(TB2, ff));
        let (_, tw_out) = run(&design(TB2, tw));
        assert_eq!(rc, Some(0), "{ff}:\n{ff_out}");
        expect_codes(&ff_out, &[], ff);
        assert_eq!(values(&ff_out).len(), 5, "{ff}:\n{ff_out}");
        assert_eq!(values(&ff_out), values(&tw_out), "{ff}: values vs twin");
    }
}

/// The routed shapes' values, pinned against the oracle lines in the module doc.
#[test]
fn routed_shapes_print_the_oracles_values() {
    let want: [(&str, [&str; 5]); 5] = [
        (
            "  always_ff begin q2 <= d; @(posedge clk) q <= d; end",
            [
                "c1 q=1 q2=1",
                "c2 q=0 q2=0",
                "c3 q=1 q2=1",
                "c4 q=0 q2=0",
                "c5 q=1 q2=1",
            ],
        ),
        (
            "  always_ff begin : nb @(posedge clk) q <= d; if (!en) disable nb; q2 <= d; end",
            [
                "c1 q=1 q2=1",
                "c2 q=0 q2=0",
                "c3 q=1 q2=0",
                "c4 q=0 q2=0",
                "c5 q=1 q2=1",
            ],
        ),
        (
            "  always_ff begin @(posedge clk) q <= #1 d; end",
            [
                "c1 q=x q2=x",
                "c2 q=1 q2=x",
                "c3 q=0 q2=x",
                "c4 q=1 q2=x",
                "c5 q=0 q2=x",
            ],
        ),
        (
            "  always_ff do @(posedge clk) q <= d; while (0);",
            [
                "c1 q=1 q2=x",
                "c2 q=0 q2=x",
                "c3 q=1 q2=x",
                "c4 q=0 q2=x",
                "c5 q=1 q2=x",
            ],
        ),
        (
            "  always_ff @* begin q = d; q2 = q & en; end",
            [
                "c1 q=1 q2=1",
                "c2 q=0 q2=0",
                "c3 q=1 q2=0",
                "c4 q=0 q2=0",
                "c5 q=1 q2=1",
            ],
        ),
    ];
    for (body, lines) in want {
        let (rc, out) = run(&design(TB2, body));
        assert_eq!(rc, Some(0), "{body}:\n{out}");
        expect_codes(&out, &[], body);
        assert_eq!(values(&out), lines, "{body}:\n{out}");
    }
}

/// A clock edge at time 0 reaches the header-less block as it reaches its twin; the
/// `@*` block reads a time-0 level change as `always @*` does.
#[test]
fn time_zero_edges_match_the_twin() {
    let hl = "  always_ff begin @(posedge clk) q <= d; end";
    let tw = "  always    begin @(posedge clk) q <= d; end";
    let hd = "  always_ff @(posedge clk) q <= d;";
    let st = "  always_ff @* q <= d & clk;";
    let st_tw = "  always    @* q <= d & clk;";
    let want_edge = [
        ["t1 q=1", "t2 q=1", "t3 q=0"],
        ["t1 q=x", "t2 q=x", "t3 q=0"],
        ["t1 q=1", "t2 q=1", "t3 q=0"],
    ];
    let want_star = [
        ["t1 q=1", "t2 q=0", "t3 q=0"],
        ["t1 q=x", "t2 q=0", "t3 q=0"],
        ["t1 q=1", "t2 q=0", "t3 q=0"],
    ];
    for (i, way) in T0_WAYS.into_iter().enumerate() {
        for (body, want) in [
            (hl, want_edge[i]),
            (tw, want_edge[i]),
            (hd, want_edge[i]),
            (st, want_star[i]),
            (st_tw, want_star[i]),
        ] {
            let src = t0_design(way, body);
            let (rc, out) = run(&src);
            assert_eq!(rc, Some(0), "{} / {body}:\n{out}", way.0);
            expect_codes(&out, &[], body);
            assert_eq!(values(&out), want, "{} / {body}:\n{out}", way.0);
        }
        let (a, _) = elaborate_ir(&t0_design(way, hl));
        let (b, _) = elaborate_ir(&t0_design(way, tw));
        assert!(a.expect("hl") == b.expect("tw"), "{}: IR differs", way.0);
        let (a, _) = elaborate_ir(&t0_design(way, st));
        let (b, _) = elaborate_ir(&t0_design(way, st_tw));
        assert!(
            a.expect("st") == b.expect("st_tw"),
            "{}: @* IR differs",
            way.0
        );
    }
}

/// What `VITA-E3061` says, by rule.
const NO_EVENT: &str =
    "`always_ff` has no event control; IEEE 1800-2017 §9.2.2.4 requires exactly one";
const BLOCKING: &str = "a blocking timing control; IEEE 1800-2017 §9.2.2.4 allows none";
const NO_STMT_EVENT: &str = "`always_ff` has no statement-level event control: with no header, its one event control is an intra-assignment `@` on a non-blocking assignment";
const NOT_EVERY_PASS: &str =
    "`always_ff` has no header, and its one event control is not reached on every pass";
const IN_FORK: &str = "`always_ff` has an event control inside a `fork`; a forked process cannot be the procedure's one event control";

fn count_msg(n: usize, place: &str) -> String {
    format!(
        "`always_ff` has {n} event controls ({place}); IEEE 1800-2017 §9.2.2.4 allows exactly one"
    )
}

/// The `VITA-E3061` lines the run printed.
fn e3061_lines(out: &str) -> Vec<&str> {
    out.lines().filter(|l| l.contains("[VITA-E3061]")).collect()
}

/// A refused design: exit 1, `VITA-E3061` and nothing else, one line per expected
/// message, and no simulation.
fn expect_refused(src: &str, msgs: &[&str], what: &str) -> String {
    let (rc, out) = run(src);
    assert_eq!(rc, Some(1), "{what}:\n{out}");
    expect_codes(&out, &["VITA-E3061"], what);
    let lines = e3061_lines(&out);
    assert_eq!(lines.len(), msgs.len(), "{what}: E3061 lines:\n{out}");
    for (l, m) in lines.iter().zip(msgs) {
        assert!(l.contains(m), "{what}: want {m:?} in\n{l}");
    }
    assert!(values(&out).is_empty(), "{what}: simulated:\n{out}");
    out
}

/// `f_twoev`, as the blog post writes it: two event controls, the header's and one in
/// the body. Owner ruling 2026-10-06: an error, exit 1 (see the module doc). Anchored at
/// the body's `@`, the one past the first.
#[test]
fn f_twoev_is_refused() {
    let src = "module f_twoev (input logic clk, d, output logic q);
    always_ff @(posedge clk) begin
        q <= d;
        @(posedge clk) q <= ~d;
    end
endmodule
";
    let out = expect_refused(
        src,
        &[&count_msg(2, "the header and 1 in the body")],
        "f_twoev",
    );
    assert!(
        out.contains("t.sv:4:9: error[VITA-E3061]"),
        "anchor:\n{out}"
    );
    // The same block under the i2 testbench (i2_twoev), where verilator and sv2v print
    // `1 1 1 1 0`.
    let tb = design(
        TB,
        "  always_ff @(posedge clk) begin\n    q <= d;\n    @(posedge clk) q <= ~d;\n  end",
    );
    expect_refused(
        &tb,
        &[&count_msg(2, "the header and 1 in the body")],
        "i2_twoev",
    );
}

/// `i2_noctl`: no event control at all.
#[test]
fn i2_noctl_is_refused() {
    let out = expect_refused(&design(TB, "  always_ff q <= d;"), &[NO_EVENT], "i2_noctl");
    assert!(
        out.contains("t.sv:2:3: error[VITA-E3061]"),
        "anchor:\n{out}"
    );
}

/// `i2_delay`, `i2_wait`, `i2_blk_intra`: one event control and a blocking timing
/// control, each named; and the other blocking forms.
#[test]
fn blocking_timing_is_refused() {
    for (what, body, kind) in [
        (
            "i2_delay",
            "  always_ff @(posedge clk) begin\n    #1 q <= d;\n  end",
            "contains a `#` delay, ",
        ),
        (
            "i2_wait",
            "  always_ff @(posedge clk) begin\n    wait (d) q <= ~q;\n  end",
            "contains a `wait`, ",
        ),
        (
            "i2_blk_intra",
            "  always_ff @(posedge clk) q = #1 d;",
            "contains an intra-assignment delay on a blocking assignment (`= #…`), ",
        ),
        (
            "blocking intra event",
            "  always_ff @(posedge clk) q = @(negedge clk) d;",
            "contains an intra-assignment event control on a blocking assignment (`= @(…)`), ",
        ),
        (
            "wait fork",
            "  logic t;\n  always_ff @(posedge clk) begin fork t <= d; join_none wait fork; q <= d; end",
            "contains a `wait fork`, ",
        ),
        (
            "# in a fork branch",
            "  always_ff @(posedge clk) fork #1 q <= d; join",
            "contains a `#` delay, ",
        ),
        (
            "# in a join_none branch",
            "  logic t;\n  always_ff @(posedge clk) begin fork #1 t <= d; join_none q <= d; end",
            "contains a `#` delay, ",
        ),
    ] {
        let out = expect_refused(&design(TB, body), &[BLOCKING], what);
        let line = e3061_lines(&out)[0];
        assert!(line.contains(kind), "{what}: want {kind:?} in\n{line}");
    }
}

/// Both rules broken: one diagnostic each, the count first.
#[test]
fn both_rules_report_once_each() {
    expect_refused(
        &design(
            TB,
            "  always_ff @(posedge clk) begin #1; @(negedge clk) q <= d; end",
        ),
        &[&count_msg(2, "the header and 1 in the body"), BLOCKING],
        "count 2 and a delay",
    );
    expect_refused(
        &design(TB, "  always_ff begin wait (d); q <= d; end"),
        &[NO_EVENT, BLOCKING],
        "count 0 and a wait",
    );
}

/// The shapes the change does not route (they kept the edge lane and its W3056 before
/// this rule) are refused: two event controls (an intra-assignment `@` on a non-blocking
/// assignment among them), an `@*` header beside one in the body, a blocking timing
/// control beside the one event control, no event control, an intra-assignment `@` as the
/// only one, and an `@` inside a `fork`.
#[test]
fn unrouted_shapes_are_refused() {
    let two_hl = count_msg(2, "2 in the body");
    let two_hd = count_msg(2, "the header and 1 in the body");
    for (body, msg) in [
        (
            "  always_ff begin @(posedge clk) q <= d; @(posedge clk) q <= ~d; end",
            two_hl.as_str(),
        ),
        (
            "  always_ff begin @(posedge clk) begin q <= d; @(negedge clk) q2 <= d; end end",
            two_hl.as_str(),
        ),
        (
            "  always_ff begin @(posedge clk) if (en) @(negedge clk) q <= d; end",
            two_hl.as_str(),
        ),
        (
            "  always_ff begin @(posedge clk) q <= d; #1 q2 <= d; end",
            BLOCKING,
        ),
        (
            "  always_ff begin @(posedge clk) q <= d; wait (en) q2 <= d; end",
            BLOCKING,
        ),
        ("  always_ff begin @(posedge clk) q = #1 d; end", BLOCKING),
        ("  always_ff q <= @(posedge clk) d;", NO_STMT_EVENT),
        ("  always_ff q <= d;", NO_EVENT),
        (
            "  always_ff @* begin @(posedge clk) q <= d; end",
            two_hd.as_str(),
        ),
        ("  always_ff @* q <= @(posedge clk) d;", two_hd.as_str()),
        (
            "  always_ff begin @(posedge clk) q <= d; q2 <= @(negedge clk) d; end",
            two_hl.as_str(),
        ),
        (
            "  always_ff begin fork @(posedge clk) q <= d; join_none end",
            IN_FORK,
        ),
        (
            "  always_ff fork @(posedge clk) q <= d; begin end join_any",
            IN_FORK,
        ),
        (
            "  always_ff begin fork @(posedge clk) q <= d; join end",
            IN_FORK,
        ),
    ] {
        expect_refused(&design(TB2, body), &[msg], body);
    }
}

/// Legal shapes keep their behaviour: `f_ok`, `f_noedge` and `f_block` as the blog post
/// writes them run with no diagnostic, and `i2_ok_async` prints the oracles' values.
#[test]
fn legal_shapes_are_unchanged() {
    for (what, src) in [
        (
            "f_ok",
            "module f_ok (input logic clk, rst_n, en, d, output logic q);
    always_ff @(posedge clk or negedge rst_n)
        if (!rst_n)  q <= 1'b0;
        else if (en) q <= d;
endmodule
",
        ),
        (
            "f_noedge",
            "module f_noedge (input logic a, b, output logic y);
    always_ff @(a or b) y <= a & b;
endmodule
",
        ),
        (
            "f_block",
            "module f_block (input logic clk, d, output logic q1, q2);
    always_ff @(posedge clk) begin
        q1 = d;
        q2 = q1;
    end
endmodule
",
        ),
    ] {
        let (rc, out) = run(src);
        assert_eq!(rc, Some(0), "{what}:\n{out}");
        expect_codes(&out, &[], what);
    }
    let (rc, out) = run(&design(
        TB,
        "  always_ff @(posedge clk or posedge rst)\n    if (rst) q <= 1'b0; else q <= d;",
    ));
    assert_eq!(rc, Some(0), "i2_ok_async:\n{out}");
    expect_codes(&out, &[], "i2_ok_async");
    assert_eq!(
        values(&out),
        ["c1 q=1", "c2 q=0", "c3 q=1", "r1 q=0", "c4 q=1"],
        "i2_ok_async:\n{out}"
    );
}

/// `i2_nba_intra`: a non-blocking intra-assignment delay does not block the process, so
/// it is legal; the value is iverilog's and sv2v's (verilator reads 0 for the unwritten
/// `q` at `c1`).
#[test]
fn nba_intra_delay_is_accepted() {
    let (rc, out) = run(&design(TB, "  always_ff @(posedge clk) q <= #1 d;"));
    assert_eq!(rc, Some(0), "i2_nba_intra:\n{out}");
    expect_codes(&out, &[], "i2_nba_intra");
    assert_eq!(
        values(&out),
        ["c1 q=x", "c2 q=1", "c3 q=0", "r1 q=1", "c4 q=1"],
        "i2_nba_intra:\n{out}"
    );
}

/// An intra-assignment EVENT control on a non-blocking assignment is an `event_control`
/// by the grammar (`nonblocking_assignment ::= variable_lvalue <= [delay_or_event_control]
/// expression`), so it counts: beside a header it makes two (impl_c_nbaev, nbaev_mytb:
/// iverilog refuses both, verilator runs them; the module doc has the raw text). With no
/// header it can never be the one: nothing would suspend the block.
#[test]
fn nba_intra_event_counts() {
    let two_hd = count_msg(2, "the header and 1 in the body");
    for (what, tb, body, msg) in [
        (
            "impl_c_nbaev",
            TB2,
            "  always_ff @(posedge clk) q <= @(negedge clk) d;",
            two_hd.as_str(),
        ),
        (
            "repeat intra event",
            TB2,
            "  always_ff @(posedge clk) q <= repeat (1) @(negedge clk) d;",
            two_hd.as_str(),
        ),
        (
            "i2 testbench",
            TB,
            "  always_ff @(posedge clk) q <= @(negedge clk) d;",
            two_hd.as_str(),
        ),
        (
            "no statement-level event control",
            TB2,
            "  always_ff q <= @(posedge clk) d;",
            NO_STMT_EVENT,
        ),
    ] {
        let out = expect_refused(&design(tb, body), &[msg], what);
        assert!(
            out.contains("t.sv:2:"),
            "{what}: anchor on the block's line:\n{out}"
        );
    }
}

/// An event control inside a `fork` branch — `join`, `join_any`, `join_none`, nested — is
/// refused, with a header or without; a `fork` with no timing in it is legal.
#[test]
fn event_control_inside_a_fork_is_refused() {
    for body in [
        "  always_ff begin fork @(posedge clk) q <= d; join end",
        "  always_ff fork @(posedge clk) q <= d; begin end join_any",
        "  always_ff begin fork @(posedge clk) q <= d; join_none end",
        "  always_ff begin fork begin begin @(posedge clk) q <= d; end end join end",
        "  always_ff @(posedge clk) begin fork begin @(negedge clk) q2 <= d; end join_none q <= d; end",
        "  always_ff @* begin fork @(posedge clk) q <= d; join end",
    ] {
        expect_refused(&design(TB2, body), &[IN_FORK], body);
    }
    // An intra-assignment `@` in a fork branch is refused too; vita also cannot lower it
    // there (`VITA-E3009`, before this rule as after).
    let (rc, out) = run(&design(
        TB2,
        "  always_ff @(posedge clk) begin fork q2 <= @(negedge clk) d; join_none q <= d; end",
    ));
    assert_eq!(rc, Some(1), "forked intra event:\n{out}");
    expect_codes(&out, &["VITA-E3009", "VITA-E3061"], "forked intra event");
    assert!(
        e3061_lines(&out).iter().all(|l| l.contains(IN_FORK)),
        "forked intra event:\n{out}"
    );
    // An `@` outside the fork beside one inside it: the fork's is its own violation.
    expect_refused(
        &design(
            TB2,
            "  always_ff begin @(posedge clk) q <= d; fork @(negedge clk) q2 <= d; join_none end",
        ),
        &[IN_FORK],
        "outer @ and forked @",
    );
    let (rc, out) = run(&design(
        TB2,
        "  always_ff @(posedge clk) fork q <= d; q2 <= ~d; join",
    ));
    assert_eq!(rc, Some(0), "timing-free fork:\n{out}");
    expect_codes(&out, &[], "timing-free fork");
    assert_eq!(
        values(&out),
        [
            "c1 q=1 q2=0",
            "c2 q=0 q2=1",
            "c3 q=1 q2=0",
            "c4 q=0 q2=1",
            "c5 q=1 q2=0"
        ],
        "timing-free fork:\n{out}"
    );
}

/// With no header, an `@(…)` that some pass misses is refused (stricter than IEEE, see the
/// module doc): inside `if` / `else`, a `case` arm, a loop body (`for`, `foreach`,
/// `while`, `repeat`), a nested `begin` under an `if`, an `if` under a `begin`, a
/// `forever` or `do … while` body under an `if`; and after a loop or a `disable`.
#[test]
fn event_control_missed_on_some_pass_is_refused() {
    for body in [
        "  always_ff begin if (en) @(posedge clk) q <= d; q2 <= d; end",
        "  always_ff begin if (en) @(posedge clk) q <= d; else q2 <= d; end",
        "  always_ff begin case (en) 1'b1: @(posedge clk) q <= d; default: q2 <= d; endcase end",
        "  always_ff begin for (int i = 0; i < 2; i++) @(posedge clk) q <= d; end",
        "  logic fa [2];\n  always_ff begin foreach (fa[i]) @(posedge clk) q <= d; end",
        "  always_ff begin while (en) @(posedge clk) q <= d; q2 <= d; end",
        "  always_ff begin repeat (0) @(posedge clk); q2 <= d; end",
        "  always_ff begin repeat (2) @(posedge clk); q <= d; end",
        "  always_ff begin begin if (en) @(posedge clk) q <= d; end q2 <= d; end",
        "  always_ff begin if (en) begin @(posedge clk) q <= d; end end",
        "  always_ff begin if (en) forever @(posedge clk) q <= d; end",
        "  always_ff forever begin if (en) @(posedge clk) q <= d; q2 <= d; end",
        "  always_ff do begin if (en) @(posedge clk) q <= d; q2 <= d; end while (0);",
        "  always_ff begin for (int i = 0; i < 2; i++) q2 <= d; @(posedge clk) q <= d; end",
        "  always_ff begin : nb q2 <= d; if (!en) disable nb; @(posedge clk) q <= d; end",
        "  always_ff begin do q2 <= d; while (0); @(posedge clk) q <= d; end",
        "  always_ff begin forever q2 <= d; @(posedge clk) q <= d; end",
    ] {
        let out = expect_refused(&design(TB2, body), &[NOT_EVERY_PASS], body);
        assert!(
            out.contains("stricter than IEEE 1800-2017 §9.2.2.4"),
            "{body}: the message says the rule is vita's:\n{out}"
        );
    }
}

/// `do S while (c)` is one statement in the source, though the parser copies `S`: one
/// `@` is one event control. With no header it is routed (its values are pinned with the
/// other routed shapes); under a header it makes two, not three.
#[test]
fn do_while_counts_its_event_once() {
    let (rc, out) = run(&design(
        TB2,
        "  always_ff do @(posedge clk) q <= d; while (0);",
    ));
    assert_eq!(rc, Some(0), "do-while:\n{out}");
    expect_codes(&out, &[], "do-while");
    expect_refused(
        &design(
            TB2,
            "  always_ff @(posedge clk) do @(negedge clk) q <= d; while (0);",
        ),
        &[&count_msg(2, "the header and 1 in the body")],
        "do-while under a header",
    );
}

/// Out of scope, pinned as it is (PROBE_CATALOG §4.5.597): timing inside a CALLED task is
/// not counted. A header block whose task holds a `#` runs (iverilog counts through the
/// task and refuses it: "a blocking delay is not allowed in an always_comb, always_ff or
/// always_latch process."); a header-less block whose only event control is in a task
/// has a count of 0 and is refused (iverilog refuses it too: "the first statement of an
/// always_ff process must be an event control statement."; verilator and sv2v print
/// `1 0 1 0 1`).
#[test]
fn timing_in_a_called_task_is_not_counted() {
    let tasks =
        "  task automatic t_ev; @(posedge clk); endtask\n  task automatic t_dl; #1; endtask\n";
    let (rc, out) = run(&design(
        TB2,
        &format!("{tasks}  always_ff @(posedge clk) begin t_dl; q <= d; end"),
    ));
    assert_eq!(rc, Some(0), "task delay:\n{out}");
    expect_codes(&out, &[], "task delay");
    expect_refused(
        &design(TB2, &format!("{tasks}  always_ff begin t_ev; q <= d; end")),
        &[NO_EVENT],
        "task event",
    );
}
