//! An edge-sensitive `always` fires AT MOST ONCE per ACTIVE-region settle cluster
//! (ROADMAP §4.5.4 scheduler bug #4).
//!
//! When a process is sensitive to two edges where one is DERIVED from the other
//! via a cont-assign (a gated/buffered clock: `gclk = clk & en`), the derived edge
//! lands a delta AFTER the base edge — same cluster, different delta. vita
//! previously fired the process on BOTH (the per-delta dedup only collapses edges
//! in the SAME delta), double-counting. iverilog collapses the cont-assign ghost
//! into the base firing (which sees the settled state).
//!
//! Fix (engine only, IR-0): the per-process edge-wake marker `edge_seen` is
//! promoted from per-delta to per-CLUSTER scope — reset at every region boundary
//! (#0 Inactive promotion, NBA apply, time advance), NOT per-delta. A cont-assign
//! ghost (same cluster) collapses; a GENUINELY NEW event from a later region (an
//! independent `negedge rst` via `#0`/NBA) still re-fires the process. A design
//! with one edge per cluster per process is unaffected ⇒ byte-identical. Every
//! expectation is pinned to live iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gclk_{}_{n}", std::process::id()));
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
fn gated_clock_fires_once_per_timestep() {
    // `gclk = clk & en` (en=1) derives gclk one delta after clk. Two clk rises ⇒
    // 2 firings (iverilog), not 4 (vita's old per-edge count).
    let (out, code) = run("module top;\n\
         reg clk, en; wire gclk; integer cnt;\n\
         assign gclk = clk & en;\n\
         always @(posedge clk or posedge gclk) cnt = cnt + 1;\n\
         initial begin cnt=0; clk=0; en=1; #1; clk=1; #1; clk=0; #1; clk=1; #1; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "gated clock must fire once per timestep; got:\n{out}"
    );
}

#[test]
fn buffered_clock_fires_once_per_timestep() {
    // `gclk = clk` (a buffered copy) — same as gated: one timestep, one firing.
    let (out, code) = run("module top;\n\
         reg clk; wire gclk; integer cnt;\n\
         assign gclk = clk;\n\
         always @(posedge clk or posedge gclk) cnt = cnt + 1;\n\
         initial begin cnt=0; clk=0; #1; clk=1; #1; clk=0; #1; clk=1; #1; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "buffered clock must fire once per timestep; got:\n{out}"
    );
}

#[test]
fn single_firing_sees_settled_state() {
    // The single per-timestep firing must see the SETTLED value (gclk=1, after the
    // derived edge has propagated), like iverilog ⇒ log=1 (one firing reading 1),
    // not 11 (two firings).
    let (out, code) = run("module top;\n\
         reg clk, en; wire gclk; integer log;\n\
         assign gclk = clk & en;\n\
         always @(posedge clk or posedge gclk) log = log*10 + (gclk?1:0);\n\
         initial begin log=0; clk=0; en=1; #1; clk=1; #5; $display(\"log=%0d\", log); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("log=1"),
        "single firing must see settled state; got:\n{out}"
    );
}

#[test]
fn async_reset_derived_fires_once() {
    // `@(posedge clk or negedge rst_d)` with rst_d derived from rst_n: clk posedge
    // and rst_d negedge coincide (different deltas) ⇒ ONE firing.
    let (out, code) = run("module top;\n\
         reg clk, rst_n; wire rst_d; integer cnt;\n\
         assign rst_d = rst_n;\n\
         always @(posedge clk or negedge rst_d) cnt = cnt + 1;\n\
         initial begin cnt=0; clk=0; rst_n=1; #1; clk=1; rst_n=0; #5; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=1"),
        "async-reset derived edges must fire once; got:\n{out}"
    );
}

#[test]
fn two_level_derivation_fires_once() {
    // clk → g1 → g2 (two cont-assign hops): three edges across three deltas, one
    // timestep ⇒ ONE firing.
    let (out, code) = run("module top;\n\
         reg clk; wire g1, g2; integer cnt;\n\
         assign g1 = clk; assign g2 = g1;\n\
         always @(posedge clk or posedge g1 or posedge g2) cnt = cnt + 1;\n\
         initial begin cnt=0; clk=0; #1; clk=1; #5; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=1"),
        "two-level derived clock must fire once; got:\n{out}"
    );
}

#[test]
fn independent_inactive_edge_refires() {
    // The collapse is per ACTIVE-region CLUSTER, not per timestep: clk posedge
    // (delta 0) fires the process, then an INDEPENDENT `negedge rst` scheduled via
    // `#0` into a LATER delta of the same timestep is a NEW cluster ⇒ must re-fire
    // ⇒ cnt=2, q=0 (reset wins). A gated-clock ghost (cont-assign derived) would
    // NOT re-fire — this distinguishes the two.
    let (out, code) = run("module top;\n\
         reg clk, rst, q; integer cnt;\n\
         always @(posedge clk or negedge rst) begin cnt=cnt+1; if(!rst) q=0; else q=1; end\n\
         initial begin cnt=0; q=0; clk=0; rst=1; #5; clk=1; #0 rst=0; #2 $display(\"cnt=%0d q=%b\", cnt, q); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2 q=0"),
        "independent #0 edge must re-fire; got:\n{out}"
    );
}

#[test]
fn independent_nba_edge_refires() {
    // Same, but the independent edge arrives via the NBA region: a separate block
    // does `rst <= 0` on the clock, so `negedge rst` lands in NBA — a new cluster ⇒
    // the `@(… or negedge rst)` process re-fires ⇒ cnt=2.
    let (out, code) = run("module top;\n\
         reg clk, rst; integer cnt;\n\
         always @(posedge clk or negedge rst) cnt = cnt + 1;\n\
         always @(posedge clk) rst <= 0;\n\
         initial begin cnt=0; clk=0; rst=1; #5; clk=1; #2 $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "independent NBA edge must re-fire; got:\n{out}"
    );
}

#[test]
fn direct_clock_byte_identity() {
    // Byte-identity guard: a direct clock (no derivation) firing across SEPARATE
    // timesteps must fire each time ⇒ cnt=2 for two posedges in two timesteps.
    let (out, code) = run("module top;\n\
         reg clk; integer cnt;\n\
         always @(posedge clk) cnt = cnt + 1;\n\
         initial begin cnt=0; clk=0; #1; clk=1; #1; clk=0; #1; clk=1; #5; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "direct clock per-timestep firing must be unchanged; got:\n{out}"
    );
}

#[test]
fn gate_disabled_byte_identity() {
    // Byte-identity guard: with en=0 the gate's gclk never rises, so only the clk
    // edges fire ⇒ cnt=2 (unchanged from before the fix).
    let (out, code) = run("module top;\n\
         reg clk, en; wire gclk; integer cnt;\n\
         assign gclk = clk & en;\n\
         always @(posedge clk or posedge gclk) cnt = cnt + 1;\n\
         initial begin cnt=0; clk=0; en=0; #1; clk=1; #1; clk=0; #1; clk=1; #1; $display(\"cnt=%0d\", cnt); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("cnt=2"),
        "disabled gate must be unchanged; got:\n{out}"
    );
}
