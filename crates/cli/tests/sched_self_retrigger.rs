//! Blocking self-write must NOT re-trigger its own process (ROADMAP §4.5.4
//! scheduler bug #3).
//!
//! A process suspended on `@(sig)` does not respond to a value change it made
//! itself via a BLOCKING write before re-arming (IEEE §9 active/inactive region
//! model: the write happens while the process is executing, not waiting, so by
//! the time it re-arms the change is already past). vita previously re-fired the
//! process on its own blocking self-write → `always @(a) a=~a` looped forever and
//! a one-shot `if(a) a=0` double-counted.
//!
//! Fix (engine only, IR-0): the write funnel tags each value change with its
//! author (`blocking_writer`, set only around `run_body`); propagate_changes
//! suppresses firing a process on a net IT itself blocking-wrote. An NBA
//! self-write still re-fires (it lands in the NBA region, AFTER re-arm), and a
//! DIFFERENT process always fires (the skip is per-process). Non-self-feedback
//! designs never hit the skip ⇒ byte-identical. Every expectation is pinned to
//! live iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_selfretrig_{}_{n}", std::process::id()));
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
fn one_shot_blocking_self_write_no_retrigger() {
    // a:x→0 (t0 fire) then 0→1 (t1 fire, body sets a=0). The blocking a=0 must
    // NOT re-fire the block ⇒ cnt=2 (iverilog), not 3.
    let (out, code) = run("module top;\n\
         reg a; integer cnt;\n\
         initial begin cnt=0; a=0; #1; a=1; #1; $display(\"cnt=%0d a=%0d\", cnt, a); $finish; end\n\
         always @(a) begin cnt=cnt+1; if(a) a=0; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2 a=0"),
        "blocking self-write must not re-fire; got:\n{out}"
    );
}

#[test]
fn blocking_oscillator_ticks_once() {
    // `always @(a) a=~a` is the classic self-feedback. iverilog ticks it ONCE
    // (cnt=1, a settles to 1) — it does NOT oscillate. vita previously looped to
    // the cnt<100 guard.
    let (out, code) = run("module top;\n\
         reg a; integer cnt;\n\
         initial begin cnt=0; a=0; #1; a=1; #1; $display(\"cnt=%0d a=%0d\", cnt, a); $finish; end\n\
         always @(a) begin cnt=cnt+1; if(cnt<100) a=~a; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=1 a=1"),
        "blocking oscillator must tick once; got:\n{out}"
    );
}

#[test]
fn nba_self_write_does_retrigger() {
    // Contrast: an NBA self-write a<=0 lands in the NBA region AFTER the block
    // re-arms, so it IS a fresh event and DOES re-fire ⇒ cnt=3 (iverilog). The
    // fix must leave this untouched (NBA writes author as `None`).
    let (out, code) = run("module top;\n\
         reg a; integer cnt;\n\
         initial begin cnt=0; a=0; #1; a=1; #1; $display(\"cnt=%0d a=%0d\", cnt, a); $finish; end\n\
         always @(a) begin cnt=cnt+1; if(a) a<=0; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=3 a=0"),
        "NBA self-write must re-fire; got:\n{out}"
    );
}

#[test]
fn posedge_blocking_self_write_no_retrigger() {
    // clk:0→1 (posedge fire), body sets clk=0 (1→0 = negedge, not a posedge, and
    // also a self-write). cnt=1 (iverilog).
    let (out, code) = run("module top;\n\
         reg clk; integer cnt;\n\
         initial begin cnt=0; clk=0; #1; clk=1; #1; $display(\"cnt=%0d clk=%0d\", cnt, clk); $finish; end\n\
         always @(posedge clk) begin cnt=cnt+1; if(cnt<5) clk=0; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=1 clk=0"),
        "posedge blocking self-write must not loop; got:\n{out}"
    );
}

#[test]
fn self_feedback_settle_no_retrigger() {
    // `always @(a or s) if(s) a=1` — at s=1 the block fires once and sets a=1;
    // the self-write must not re-fire ⇒ cnt=2 (t0 + the s=1 fire).
    let (out, code) = run("module top;\n\
         reg a, s; integer cnt;\n\
         initial begin cnt=0; a=0; s=0; #1; s=1; #1; $display(\"cnt=%0d a=%0d\", cnt, a); $finish; end\n\
         always @(a or s) begin cnt=cnt+1; if(s) a=1; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2 a=1"),
        "self-feedback settle must not re-fire; got:\n{out}"
    );
}

#[test]
fn cross_process_write_still_fires() {
    // The skip is PER-PROCESS: process A blocking-writes `a` (sensitive to b);
    // process B is `@(a)`. B must fire on A's write ⇒ cntB=2 (t0 + b=1 propagated
    // through a). Only the AUTHOR is suppressed, never another observer.
    let (out, code) = run("module top;\n\
         reg a, b; integer cntB;\n\
         initial begin cntB=0; a=0; b=0; #1; b=1; #1; $display(\"cntB=%0d a=%0d\", cntB, a); $finish; end\n\
         always @(b) a = b;\n\
         always @(a) cntB = cntB + 1;\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cntB=2 a=1"),
        "a cross-process observer must still fire; got:\n{out}"
    );
}

#[test]
fn self_write_to_other_net_unaffected() {
    // The block writes a DIFFERENT net (b) than its sensitivity (a) — no
    // self-feedback, so firing is unchanged ⇒ cnt=2 (t0 + a=1).
    let (out, code) = run("module top;\n\
         reg a, b; integer cnt;\n\
         initial begin cnt=0; a=0; b=0; #1; a=1; #1; $display(\"cnt=%0d\", cnt); $finish; end\n\
         always @(a) begin cnt=cnt+1; b=~b; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "write to a non-sensitivity net is unaffected; got:\n{out}"
    );
}
