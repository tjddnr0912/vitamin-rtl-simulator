//! Blocking/NBA glitch wake-collapse (ROADMAP §4.5.4 scheduler bugs #1+#2).
//!
//! A net written A→B→A within ONE slot (blocking OR non-blocking) ends with
//! `cur == prev`. The scheduler's dirty sweep previously filtered candidates by
//! `cur != prev`, silently DROPPING such a round-trip — so `always @(a)`,
//! `always @(posedge a)`, an in-body `@(posedge x)`, and an in-body `@(a)` all
//! failed to fire on a glitch. IEEE §9 fires the observer ONCE per slot (it
//! collapses the round-trip to a single evaluation, not one-per-transition).
//!
//! Fix (engine only, IR-0):
//!   * `propagate_changes` uses `dirty` MEMBERSHIP as the changed set (a net is
//!     dirty iff `note_change` saw a real transition) instead of the endpoint
//!     `cur != prev` test — recovers any-change `@(a)`/`@(v)` glitches.
//!   * the write funnel accumulates a per-net intra-slot bit0 edge mask
//!     (`slot_edge`); the edge-wake passes fire from that mask — recovers
//!     `@(posedge/negedge …)` glitches. For a net written once per slot the mask
//!     equals the endpoint `edge_fires`, so non-glitch designs are byte-identical.
//!
//! Every expectation is pinned to live iverilog 13.0. (The narrow-vs-wide
//! posedge x/z corner — iverilog treats 0→x as a posedge, vita does not — is a
//! SEPARATE pre-existing edge-definition issue, documented in ROADMAP §4.5.4;
//! these cases use clean 0/1 edges or any-change so they are orthogonal to it.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_glitch_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn any_change_blocking_glitch_scalar() {
    // a: x→0 (t0 fire) then 0→1→0 glitch (one more fire) ⇒ 2 (iverilog).
    let (out, code) = run("module top;\n\
         reg a; integer f;\n\
         initial begin f=0; a=0; #1; a=1; a=0; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(a) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=2"),
        "glitch must fire @(a) once; got:\n{out}"
    );
}

#[test]
fn any_change_blocking_glitch_multibit() {
    // v: x→5 (t0) then 5→3→5 glitch ⇒ 2. Bit0 stays 1 throughout, so this is a
    // pure any-change (vector) test — proves the dirty-membership path, not bit0.
    let (out, code) = run("module top;\n\
         reg [3:0] v; integer f;\n\
         initial begin f=0; v=4'd5; #1; v=4'd3; v=4'd5; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(v) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=2"),
        "vector glitch must fire @(v); got:\n{out}"
    );
}

#[test]
fn any_change_nba_glitch() {
    // NBA a<=1; a<=0; applies both in the NBA region (0→1→0) ⇒ @(a) fires once
    // for the glitch (total 2 with the t0 x→0). Same write funnel as blocking.
    let (out, code) = run("module top;\n\
         reg a; integer f;\n\
         initial begin f=0; a=0; #1; a<=1; a<=0; #1; $display(\"f=%0d a=%0d\", f, a); $finish; end\n\
         always @(a) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=2 a=0"),
        "NBA glitch must fire @(a); got:\n{out}"
    );
}

#[test]
fn any_change_glitch_through_x() {
    // a: 0→x→0 — any-change @(a) fires (a took transitions) even though it
    // returns to 0 AND never reached 1. Total 2 (t0 x→0 + the glitch). The
    // dirty-membership path is value-agnostic, so x in the middle is fine.
    let (out, code) = run("module top;\n\
         reg a; integer f;\n\
         initial begin f=0; a=0; #1; a=1'bx; a=0; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(a) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("f=2"), "x-glitch must fire @(a); got:\n{out}");
}

#[test]
fn posedge_glitch_fires_once() {
    // a: 0→1→0 inside one slot; b never rises. `@(posedge a or posedge b)` must
    // see the 0→1 and fire ONCE ⇒ 1 (iverilog). Endpoint 0→0 would miss it.
    let (out, code) = run("module top;\n\
         reg a, b; integer f;\n\
         initial begin f=0; a=0; b=0; #1; a=1; a=0; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(posedge a or posedge b) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=1"),
        "posedge glitch must fire once; got:\n{out}"
    );
}

#[test]
fn negedge_glitch_fires_once() {
    // a: 1→0→1 inside one slot ⇒ negedge sees the 1→0 and fires once ⇒ 1.
    let (out, code) = run("module top;\n\
         reg a; integer f;\n\
         initial begin f=0; a=1; #1; a=0; a=1; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(negedge a) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=1"),
        "negedge glitch must fire once; got:\n{out}"
    );
}

#[test]
fn mixed_edge_level_list_level_term_glitch() {
    // `@(posedge clk or rst)`: rst is a bare (AnyEdge) level term. After f is
    // re-zeroed at t1, rst glitches 0→1→0 ⇒ the AnyEdge term fires once ⇒ 1.
    let (out, code) = run("module top;\n\
         reg clk, rst; integer f;\n\
         initial begin f=0; clk=0; rst=0; #1; f=0; rst=1; rst=0; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(posedge clk or rst) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=1"),
        "mixed-list level glitch must fire once; got:\n{out}"
    );
}

#[test]
fn in_body_posedge_wait_through_glitch() {
    // A second process waits at `@(posedge x)`. x glitches 0→1→0 ⇒ the wait must
    // resume (done=1). Procedural edge waits live in the process body, so the
    // `is_edge_target` scan must cover process-local blocks (not just funcs).
    let (out, code) = run("module top;\n\
         reg x; reg done;\n\
         initial begin x=0; done=0; #1; x=1; x=0; #2; $display(\"done=%0d\", done); $finish; end\n\
         initial begin @(posedge x); done = 1; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("done=1"),
        "in-body @(posedge x) must wake on glitch; got:\n{out}"
    );
}

// NOTE: an in-body `@(a)` (arm-based level) waiting through a glitch that
// RETURNS to the arm value is NOT fixed by this slice — firing arm=Some waiters
// on same-slot dirtiness would spuriously re-run `@(*)` in its own arming slot
// (see sched.rs `propagate_changes` (b)). That narrow corner is documented in
// ROADMAP §4.5.4; all top-level sensitivities and in-body EDGE waits are fixed.

#[test]
fn non_glitch_posedge_byte_identity() {
    // Byte-identity guard: time-separated clean edges (no glitch) must be
    // unaffected by the fix. c: 0,1,0,1 over 4 steps ⇒ 2 posedges (unchanged).
    let (out, code) = run("module top;\n\
         reg c; integer f;\n\
         initial begin f=0; c=0; #1; c=1; #1; c=0; #1; c=1; #1; $display(\"f=%0d\", f); $finish; end\n\
         always @(posedge c) f = f + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("f=2"),
        "non-glitch posedge count must be unchanged; got:\n{out}"
    );
}

#[test]
fn clocking_holding_net_edge_still_fires() {
    // REGRESSION GUARD (no iverilog oracle — it rejects clocking blocks; pinned to
    // vita's pre-fix behavior). A clocking HOLDING net (`cb.d`) can itself be an
    // edge target: `@(posedge cb.d)`. The holding net is written by
    // `commit_clocking_sample`, NOT the normal `write_chunk` funnel — so that path
    // must ALSO feed `slot_edge` (note_change resets it; the matching
    // accumulate_edge must follow), else the posedge is silently dropped and the
    // wait hangs. A #20 watchdog distinguishes "fired" from "hung".
    let (out, code) = run("module t;\n\
         logic clk = 0, d = 0;\n\
         always #5 clk = ~clk;\n\
         clocking cb @(posedge clk); input d; endclocking\n\
         initial begin d = 1; #8; @(posedge cb.d); $display(\"FIRED\"); $finish; end\n\
         initial begin #20; $display(\"HUNG\"); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("FIRED") && !out.contains("HUNG"),
        "@(posedge cb.d) must wake (commit path must feed slot_edge); got:\n{out}"
    );
}
