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
//! Header-less blocks with two event controls (an intra-assignment `@` on a non-blocking
//! assignment counts), with only an intra-assignment one, with an event control inside a
//! `fork`, with its one `@` where some pass can miss it (in an `if`, a `case` or a loop
//! body, or after a loop or a `disable`), or with a blocking timing control, do NOT take
//! the self-timed lane: they keep the edge lane and its W3056
//! (`unrouted_shapes_keep_the_edge_lane`). A missed `@` as the self-timed lane ends a pass
//! without suspending: the plain-`always` twin of `begin if (en) @(posedge clk) q <= d; q2
//! <= d; end` reaches 1.5 GB in 0.69 s. A forked `@` as the
//! self-timed lane does not suspend the process: the plain-`always` twin
//! `always begin fork @(posedge clk) q <= d; join_none end` grows without bound (1.5 GB in
//! 1.44 s; `join_any` 4.8 s; verilator %Error-DIDNOTCONVERGE).
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

/// A header-less block with two event controls (an intra-assignment `@` counts), with
/// only an intra-assignment one, with one inside a `fork`, or with its one `@` where some
/// pass can miss it, an `@*` block with one in its body, or any of them with a blocking
/// timing control is NOT routed: it keeps the edge lane and its W3056, every line `x`.
#[test]
fn unrouted_shapes_keep_the_edge_lane() {
    for body in [
        "  always_ff begin @(posedge clk) q <= d; @(posedge clk) q <= ~d; end",
        "  always_ff begin @(posedge clk) begin q <= d; @(negedge clk) q2 <= d; end end",
        "  always_ff begin @(posedge clk) if (en) @(negedge clk) q <= d; end",
        "  always_ff begin @(posedge clk) q <= d; #1 q2 <= d; end",
        "  always_ff begin @(posedge clk) q <= d; wait (en) q2 <= d; end",
        "  always_ff begin @(posedge clk) q = #1 d; end",
        "  always_ff q <= @(posedge clk) d;",
        "  always_ff q <= d;",
        "  always_ff @* begin @(posedge clk) q <= d; end",
        "  always_ff @* q <= @(posedge clk) d;",
        "  always_ff begin @(posedge clk) q <= d; q2 <= @(negedge clk) d; end",
        "  always_ff begin fork @(posedge clk) q <= d; join_none end",
        "  always_ff fork @(posedge clk) q <= d; begin end join_any",
        "  always_ff begin fork @(posedge clk) q <= d; join end",
        "  always_ff begin @(posedge clk) q <= d; fork @(negedge clk) q2 <= d; join_none end",
        "  always_ff begin if (en) @(posedge clk) q <= d; q2 <= d; end",
        "  always_ff begin case (en) 1'b1: @(posedge clk) q <= d; default: q2 <= d; endcase end",
        "  always_ff begin while (en) @(posedge clk) q <= d; q2 <= d; end",
        "  always_ff begin repeat (0) @(posedge clk); q2 <= d; end",
        "  always_ff begin repeat (2) @(posedge clk); q <= d; end",
        "  always_ff begin if (en) begin @(posedge clk) q <= d; end end",
        "  always_ff forever begin if (en) @(posedge clk) q <= d; q2 <= d; end",
        "  always_ff begin : nb q2 <= d; if (!en) disable nb; @(posedge clk) q <= d; end",
        "  always_ff begin forever q2 <= d; @(posedge clk) q <= d; end",
    ] {
        let (rc, out) = run(&design(TB2, body));
        assert_eq!(rc, Some(0), "{body}:\n{out}");
        expect_codes(&out, &["VITA-W3056"], body);
        assert!(
            values(&out).iter().all(|l| l.contains("q=x q2=x")),
            "{body}:\n{out}"
        );
    }
}
