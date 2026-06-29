//! Multi-edge sensitivity dedup (foundation for YELLOW #2 clocking multi-event).
//!
//! A process sensitive to SEVERAL nets that all change in the SAME delta must be
//! woken EXACTLY ONCE (IEEE §9: `always @(posedge c1 or posedge c2)` ticks once per
//! slot even when both edges land together). vita previously pushed the process to
//! the Active queue once per firing net → ran the body multiple times (a broad
//! pre-existing silent-wrong, found while grounding the clocking multi-event slice).
//!
//! Engine fix only (sched.rs `propagate_changes` (a)): a per-delta `seen` marker
//! dedups the Active push. Every expectation below is pinned to live iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_med_{}_{n}", std::process::id()));
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
fn two_posedges_simultaneous_fire_once() {
    // c1 alone, c2 alone, c1 alone, then SIMULTANEOUS c1+c2 (one tick) ⇒ 4 (iverilog).
    let (out, code) = run("module top;\n\
         reg c1, c2; integer count;\n\
         always @(posedge c1 or posedge c2) count = count + 1;\n\
         initial begin\n\
           count = 0; c1 = 0; c2 = 0;\n\
           #1 c1 = 1;\n\
           #1 c1 = 0; c2 = 1;\n\
           #1 c1 = 1; c2 = 0;\n\
           #1 c1 = 0; c2 = 0;\n\
           #1 c1 = 1; c2 = 1;\n\
           #1 $display(\"count=%0d\", count);\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("count=4"), "got:\n{out}");
}

#[test]
fn async_reset_dff_simultaneous_edge() {
    // posedge clk + negedge rst in the SAME delta ⇒ ONE fire; rst (level 0) wins ⇒ q=0.
    let (out, code) = run("module top;\n\
         reg clk, rst; reg [3:0] q;\n\
         always @(posedge clk or negedge rst) begin\n\
           if (!rst) q <= 0; else q <= q + 1;\n\
         end\n\
         initial begin\n\
           clk=0; rst=1; q=5;\n\
           #1 clk=1;\n\
           #1 clk=0; rst=1;\n\
           #1 clk=1; rst=0;\n\
           #1 $display(\"q=%0d\", q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("q=0"), "got:\n{out}");
}

#[test]
fn level_or_list_all_change_fires_once() {
    // @(a or b or c): a t0 fire (x→0 init) + the all-change delta (ONE, not three)
    // + the a=0 change ⇒ 3 total (iverilog-pinned). With the double-fire bug the
    // all-change delta would add 2 extra ⇒ 5.
    let (out, code) = run("module top;\n\
         reg a,b,c; integer n;\n\
         always @(a or b or c) n = n + 1;\n\
         initial begin\n\
           n=0; a=0;b=0;c=0;\n\
           #1 a=1; b=1; c=1;\n\
           #1 a=0;\n\
           #1 $display(\"n=%0d\", n);\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("n=3"), "got:\n{out}");
}

#[test]
fn mixed_edge_and_level_fires_once() {
    // @(posedge x or y): a t0 fire (y x→0) + posedge x AND y-change in one delta
    // (ONE fire, not two) ⇒ 2 total (iverilog-pinned). The bug would give 3.
    let (out, code) = run("module top;\n\
         reg x, y; integer m;\n\
         always @(posedge x or y) m = m + 1;\n\
         initial begin\n\
           m=0; x=0; y=0;\n\
           #1 x=1; y=1;\n\
           #1 $display(\"m=%0d\", m);\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("m=2"), "got:\n{out}");
}

#[test]
fn single_edge_unaffected() {
    // A plain single-edge always must be untouched by the dedup (regression guard).
    let (out, code) = run("module top;\n\
         reg clk; integer k;\n\
         always @(posedge clk) k = k + 1;\n\
         initial begin\n\
           k=0; clk=0;\n\
           #1 clk=1; #1 clk=0; #1 clk=1; #1 clk=0; #1 clk=1;\n\
           #1 $display(\"k=%0d\", k);\n\
         end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("k=3"), "got:\n{out}");
}
