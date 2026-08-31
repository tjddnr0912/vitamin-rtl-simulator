//! A continuous driver that MOVES bits invents no transition at time zero.
//!
//! `assign n = m;` between two whole nets of the same width is a second name for
//! `m`, not a computation — and so is `assign n[1] = a; assign n[0] = b;`, which
//! is what a port connection, a bus bit and half the wiring of a fabric design
//! lower to. Such a net has no state of its own, so there is no instant at which
//! it holds a value its source never held.
//!
//! vita drove those nets from the t0 structural settle, which runs BEFORE the
//! declaration initializers. The settle therefore read the source at its declared
//! default and the run loop's first delta moved the net AGAIN once
//! `reg m = 1'b0;` had landed — a real change, which woke every level-sensitive
//! block reading it. ROADMAP §2-N; fixed by re-running the moves after static
//! initialization and giving each such net its sources' event status
//! (`sim_engine::alias`).
//!
//! ⚠️ The other half of the rule is that a CONSTANT driver really does move its
//! net off `z` at time zero and must keep waking its readers — `run.rs` records
//! the measurement (49 of 270 generated cont-assign designs diverged when that
//! dirt was dropped wholesale). Every test below that asserts "no event" has a
//! sibling on the same line asserting an event, so neither can be fixed by
//! breaking the other.
//!
//! Values pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    run_on(src, None)
}

/// The rule is written TWICE — `native::run::arm_t0` and
/// `sched::scan_arm::arm_processes_after_seed` — over two different dirty-set
/// representations. Nothing else in this file would notice if one copy drifted,
/// so the headline cell is asked of both.
fn run_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_copynet_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(f.to_str().unwrap());
    if let Some(b) = backend {
        cmd.arg("--backend").arg(b);
    }
    let out = cmd.current_dir(&d).output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

/// ⚠️⚠️ THE SUPPRESSION IS ONE-DIRECTIONAL, and an adversarial review found that
/// the hard way. The first version handed a copy net its sources' event status
/// verbatim (`dirty[n] := OR over its sources`), which is the same rule only if
/// `n` and its source share a storage default — and they do not: a driven `wire`
/// starts `z`, a `logic`/`reg` starts `x`. Here the source genuinely moves and the
/// destination provably never does (`p` is `x` from time zero to `$finish`), and
/// the OR arm woke the child anyway. iverilog prints the parent line only, and so
/// did vita before the slice, so it was a correct→wrong step.
#[test]
fn a_source_that_moves_does_not_wake_a_copy_that_did_not() {
    let src = "module sub (input logic p);\n\
           always @(p) $display(\"sub woke p=%b\", p);\n\
         endmodule\n\
         module m;\n\
           wire [1:0] mm; assign mm = {1'b1, 1'bx};\n\
           sub u (.p(mm[0]));\n\
           always @(mm) $display(\"top woke mm=%b\", mm);\n\
           initial begin #2 $finish; end\n\
         endmodule\n";
    for backend in [None, Some("vm")] {
        let (o, ok) = run_on(src, backend);
        assert!(ok, "vita failed on {backend:?}:\n{o}");
        assert!(o.contains("top woke mm=1x"), "{backend:?}:\n{o}");
        assert!(
            !o.contains("sub woke"),
            "the child's port never moved off x ({backend:?}):\n{o}"
        );
    }
}

/// A module whose `always @(p)` counts how many times its input net moved.
const COUNTER: &str = "module sink (input wire p, output wire [7:0] cnt);\n\
     reg [7:0] c = 8'd0; always @(p) c = c + 8'd1; assign cnt = c;\n\
   endmodule\n";

/// THE §2-N REPRO, both rows on one line. `pr` is 0 from its declaration, so the
/// port net it drives never transitions and the child's `always @*` never runs —
/// `o1` keeps `x`. `pw` genuinely goes `z`→0 at time zero, so the same block in
/// the same child DOES run and `o2` is 0.
#[test]
fn a_port_bound_variable_makes_no_edge_but_a_constant_driver_does() {
    let (o, ok) = run("module sub (input wire p, output wire o);\n\
           reg r; always @* r = p; assign o = r;\n\
         endmodule\n\
         module m;\n\
           reg  pr = 1'b0;\n\
           wire pw; assign pw = 1'b0;\n\
           wire o1, o2;\n\
           sub u1 (.p(pr), .o(o1));\n\
           sub u2 (.p(pw), .o(o2));\n\
           initial begin #1 $display(\"o1=%b o2=%b\", o1, o2); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("o1=x o2=0"), "got:\n{o}");
    // …and the OTHER backend runs the other copy of the rule.
    let (o2, ok2) = run_on(
        "module sub (input wire p, output wire o);\n\
           reg r; always @* r = p; assign o = r;\n\
         endmodule\n\
         module m;\n\
           reg  pr = 1'b0;\n\
           wire pw; assign pw = 1'b0;\n\
           wire o1, o2;\n\
           sub u1 (.p(pr), .o(o1));\n\
           sub u2 (.p(pw), .o(o2));\n\
           initial begin #1 $display(\"o1=%b o2=%b\", o1, o2); $finish; end\n\
         endmodule\n",
        Some("vm"),
    );
    assert!(ok2, "vita --backend vm failed:\n{o2}");
    assert!(o2.contains("o1=x o2=0"), "vm got:\n{o2}");
}

/// The same axis without a hierarchy: `assign w = <variable>` is a rename,
/// `assign w = <constant>` is a driver that moves the net off `z`.
#[test]
fn a_renamed_variable_raises_no_event_but_a_constant_does() {
    let (o, ok) = run(&format!(
        "{COUNTER}\
         module m;\n\
           reg pr = 1'b0; wire wr; assign wr = pr;\n\
           wire wc; assign wc = 1'b0;\n\
           wire [7:0] cr, cc;\n\
           sink k0 (.p(wr), .cnt(cr));\n\
           sink k1 (.p(wc), .cnt(cc));\n\
           initial begin #1 $display(\"cr=%0d cc=%0d\", cr, cc); $finish; end\n\
         endmodule\n"
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("cr=0 cc=1"), "got:\n{o}");
}

/// ⚠️⚠️ THE SUPPRESSION IS TRANSITIVE, and round 2 of the review is what settled
/// that. An intermediate copy can stay put for a reason that has nothing to do
/// with its source — its own storage default already equals the copied value —
/// and it must still FORWARD that its source moved. Here `vv` really does move
/// (`zz`→`1z`), `s` is a `wire` whose `z` default already matches `vv[0]`, and `d`
/// is a `logic` whose `x` default does not. iverilog fires on `d`; asking only
/// `s`'s dirt suppressed it, which was a correct→wrong step against HEAD.
///
/// The `x` twin is the control: identical shape, `2'b1x` instead of `2'b1z`, and
/// there iverilog is silent — so a rule that just forwards everything is wrong
/// too. Both spellings on one test so neither can be fixed by breaking the other.
#[test]
fn a_move_reaches_past_an_intermediate_copy_that_did_not_move() {
    let design = |fill: &str| {
        format!(
            "module m;\n  \
               wire [1:0] vv; assign vv = 2'b1{fill};\n  \
               wire  s; assign s = vv[0];\n  \
               logic d; assign d = s;\n  \
               reg [7:0] c = 8'd0; always @(d) c = c + 8'd1;\n  \
               initial begin #1 $display(\"c=%0d d=%b s=%b\", c, d, s); $finish; end\n\
             endmodule\n"
        )
    };
    let (oz, ok) = run(&design("z"));
    assert!(ok, "vita failed:\n{oz}");
    assert!(oz.contains("c=1 d=z s=z"), "z twin:\n{oz}");
    let (ox, ok) = run(&design("x"));
    assert!(ok, "vita failed:\n{ox}");
    assert!(ox.contains("c=0 d=x s=x"), "x twin:\n{ox}");
}

/// A chain of renames carries the answer along: the source's silence reaches the
/// end of the chain, and a constant's event reaches the end of the other one.
/// This is what makes the repair's DEPENDENCY ORDER observable — repairing `w2`
/// before `w1` would leave `w1`'s stale value for `w2` to copy.
#[test]
fn a_chain_of_renames_carries_both_answers() {
    let (o, ok) = run(&format!(
        "{COUNTER}\
         module m;\n\
           reg pr = 1'b0; wire a1; assign a1 = pr; wire a2; assign a2 = a1;\n\
           wire b1; assign b1 = 1'b0; wire b2; assign b2 = b1;\n\
           wire [7:0] ca, cb;\n\
           sink k0 (.p(a2), .cnt(ca));\n\
           sink k1 (.p(b2), .cnt(cb));\n\
           initial begin #1 $display(\"ca=%0d cb=%0d\", ca, cb); $finish; end\n\
         endmodule\n"
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("ca=0 cb=1"), "got:\n{o}");
}

/// A BUS assembled one bit at a time — every vector port in a fabric design.
/// Two quiet variables leave it quiet; two constant-driven sources, which really
/// do move off `z` at time zero, wake it. iverilog agrees on both.
///
/// ⚠️ The MIXED bus (one quiet source, one constant) is deliberately not here:
/// vita's dirty channel is per NET and iverilog's collapse is per BIT, so a
/// constant on bit 1 wakes a reader of bit 0 in vita and not in iverilog. That
/// granularity is pre-existing and independent of this rule — ROADMAP §2-N.
#[test]
fn a_bus_built_from_constant_slices_takes_the_or_of_its_sources() {
    let (o, ok) = run(&format!(
        "{COUNTER}\
         module m;\n  \
           reg r0 = 1'b0; reg r1 = 1'b0;\n  \
           wire c0; assign c0 = 1'b0;\n  \
           wire c1; assign c1 = 1'b0;\n  \
           wire [1:0] quiet; assign quiet[0] = r0; assign quiet[1] = r1;\n  \
           wire [1:0] loud;  assign loud[0]  = c0; assign loud[1]  = c1;\n  \
           wire [7:0] cq, cl;\n  \
           sink k0 (.p(quiet[0]), .cnt(cq));\n  \
           sink k1 (.p(loud[0]),  .cnt(cl));\n  \
           initial begin #1 $display(\"cq=%0d cl=%0d q=%b l=%b\", cq, cl, quiet, loud); $finish; end\n\
         endmodule\n"
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("cq=0 cl=1 q=00 l=00"), "got:\n{o}");
}

/// A constant-offset slice of a variable is still a move. A RUNTIME offset is
/// not — it selects different bits at different times — so it keeps its event.
#[test]
fn a_constant_slice_is_a_move_and_a_runtime_index_is_not() {
    let (o, ok) = run(&format!(
        "{COUNTER}\
         module m;\n\
           reg [1:0] q = 2'b00; reg [1:0] s = 2'b00;\n\
           wire kf; assign kf = q[0];\n\
           wire kv; assign kv = q[s];\n\
           wire [7:0] cf, cv;\n\
           sink k0 (.p(kf), .cnt(cf));\n\
           sink k1 (.p(kv), .cnt(cv));\n\
           initial begin #1 $display(\"cf=%0d cv=%0d\", cf, cv); $finish; end\n\
         endmodule\n"
    ));
    assert!(ok, "vita failed:\n{o}");
    // ⚠️ `cv=1` is a DIVERGENCE from iverilog, which reports 0 — pinned as vita's
    // answer, not as the oracle's. iverilog decides this in its elaborator (it
    // collapses `q[0]` and also `q & 1'b1`, but NOT `q | 1'b0`, two spellings of
    // the same identity), so its boundary is an artifact rather than a rule.
    // vita's rule is uniform: a driver that computes has an initial state.
    assert!(o.contains("cf=0 cv=1"), "got:\n{o}");
}

/// A driver that COMPUTES is not a move, even when the computation is an
/// identity and the value is the same. iverilog fires here too.
#[test]
fn a_computed_driver_keeps_its_time_zero_event() {
    let (o, ok) = run(&format!(
        "{COUNTER}\
         module m;\n\
           reg pr = 1'b0;\n\
           wire w; assign w = pr | 1'b0;\n\
           wire [7:0] cw;\n\
           sink k0 (.p(w), .cnt(cw));\n\
           initial begin #1 $display(\"cw=%0d w=%b\", cw, w); $finish; end\n\
         endmodule\n"
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("cw=1 w=0"), "got:\n{o}");
}

/// A WIDTH CHANGE is padding, which is computed — so the net keeps its event.
/// iverilog agrees.
#[test]
fn a_widening_driver_is_not_a_move() {
    let (o, ok) = run("module m;\n\
           reg [1:0] pr = 2'b00;\n\
           wire [3:0] pw; assign pw = pr;\n\
           reg [7:0] c = 8'd0; always @(pw) c = c + 8'd1;\n\
           initial begin #1 $display(\"c=%0d pw=%b\", c, pw); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("c=1 pw=0000"), "got:\n{o}");
}

/// The source need not be INITIALIZED for the rule to hold — an uninitialised
/// `reg` never transitions either, so its rename must not manufacture one. The
/// value that reaches the reader is `x`, not the net's `z` default.
#[test]
fn an_uninitialised_source_still_makes_no_edge() {
    let (o, ok) = run("module m;\n\
           reg pr;\n\
           wire pw; assign pw = pr;\n\
           reg [7:0] c = 8'd0; always @(pw) c = c + 8'd1;\n\
           initial begin #1 $display(\"c=%0d pw=%b\", c, pw); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("c=0 pw=x"), "got:\n{o}");
}

/// The repair must land the VALUE, not just suppress the event: a net whose
/// event is dropped still has to hold what its source holds by the time anything
/// reads it. A wide, non-zero initializer, so a stale `z`/`x` cannot pass.
#[test]
fn the_repaired_value_is_the_sources_initialised_value() {
    let (o, ok) = run("module sub (input wire [7:0] p, output wire [7:0] o);\n\
           assign o = p;\n\
         endmodule\n\
         module m;\n\
           reg [7:0] pr = 8'hA5;\n\
           wire [7:0] o1;\n\
           sub u1 (.p(pr), .o(o1));\n\
           initial begin #1 $display(\"o1=%h u1p=%h\", o1, u1.p); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("o1=a5 u1p=a5"), "got:\n{o}");
}

/// A source written by an `initial` block DOES transition — the initializer
/// phase is before time zero, an `initial` block is not. iverilog fires too.
#[test]
fn an_initial_block_write_still_reaches_a_renamed_net() {
    let (o, ok) = run(&format!(
        "{COUNTER}\
         module m;\n\
           reg pr; initial pr = 1'b0;\n\
           wire pw; assign pw = pr;\n\
           wire [7:0] cw;\n\
           sink k0 (.p(pw), .cnt(cw));\n\
           initial begin #1 $display(\"cw=%0d pw=%b\", cw, pw); $finish; end\n\
         endmodule\n"
    ));
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("cw=1 pw=0"), "got:\n{o}");
}
