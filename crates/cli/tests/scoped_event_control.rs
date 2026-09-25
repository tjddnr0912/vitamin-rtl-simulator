//! An explicitly package-scoped name in an event control — `@(p::sig)` /
//! `@(posedge p::clk)` (IEEE 1800 §26). vita previously loud-rejected it (E3009
//! "event control must be a bare signal name"); iverilog 13.0 supports it.
//!
//! A package variable is ONE shared net per elaboration, so `p::sig` resolves to
//! the SAME net the imported bare `@(sig)` arms on — `@(p::sig)` is therefore
//! byte-for-byte equivalent to `@(sig)` (the cardinal property, mirrored from
//! `bare_event_ctrl.rs`). Fixed by a `PkgScoped` arm in `sens_event_net` that
//! resolves via the existing `pkg_scoped_var_net` (§4.5.102). A package CONSTANT /
//! enum label is a constant like a local one (`const_level_event_t0.rs`): beside a
//! live term it runs at time 0, alone it stays loud; an unknown symbol stays loud.
//!
//! Supported (whole-signal level + edge) cases are pinned to iverilog 13.0; the
//! scoped≡bare equivalence is an internal diff (both share the package net, so
//! they agree even where vita's t0 X→0 edge model differs from iverilog — a
//! pre-existing, scoping-independent detail). Loud cases assert a reject.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sevt_{}_{n}", std::process::id()));
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
fn scoped_level_matches_iverilog() {
    // `@(p::clk)` level event fires on every whole-net change. clk toggles at #5;
    // by #23 there are 5 transitions (incl. the t0 X→0). iverilog: cnt=5.
    let (out, code) = run(
        "package p; logic clk; endpackage\n\
         module top; import p::*;\n  int cnt = 0;\n  initial clk = 0;\n  always #5 clk = ~clk;\n  always @(p::clk) cnt = cnt + 1;\n  initial begin #23 $display(\"cnt=%0d\", cnt); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("cnt=5"), "got:\n{out}");
}

#[test]
fn scoped_posedge_matches_iverilog() {
    // `@(posedge p::clk)` — 4 rising edges by #43. iverilog: cnt=4.
    let (out, code) = run(
        "package p; logic clk; endpackage\n\
         module top; import p::*;\n  int cnt = 0;\n  initial clk = 0;\n  always #5 clk = ~clk;\n  always @(posedge p::clk) cnt = cnt + 1;\n  initial begin #43 $display(\"cnt=%0d\", cnt); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("cnt=4"), "got:\n{out}");
}

#[test]
fn scoped_in_event_list_with_bare() {
    // A scoped package name and a bare local in the same `or` list. iverilog: 9.
    let (out, code) = run(
        "package p; logic a, b; endpackage\n\
         module top; import p::*;\n  logic loc = 0; int cnt = 0;\n  initial begin a=0; b=0; end\n  always #3 a = ~a;\n  always #7 loc = ~loc;\n  always @(p::a or loc) cnt = cnt + 1;\n  initial begin #20 $display(\"cnt=%0d\", cnt); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("cnt=9"), "got:\n{out}");
}

#[test]
fn scoped_equals_bare_negedge() {
    // Cardinal property: `@(negedge p::clk)` is byte-identical to `@(negedge clk)`
    // — same shared package net. (Uses negedge, whose t0 X→0 count is a
    // pre-existing scoping-independent detail; the point is scoped == bare.)
    let base = "package p; logic clk; endpackage\n\
         module top; import p::*;\n  int cnt = 0;\n  initial clk = 0;\n  always #5 clk = ~clk;\n  always @(negedge SIG) cnt = cnt + 1;\n  initial begin #43 $display(\"cnt=%0d\", cnt); $finish; end\nendmodule\n";
    let (scoped, cs) = run(&base.replace("SIG", "p::clk"));
    let (bare, cb) = run(&base.replace("SIG", "clk"));
    assert_eq!(cs, Some(0));
    assert_eq!(cb, Some(0));
    assert_eq!(
        scoped, bare,
        "scoped @(negedge p::clk) must equal bare @(negedge clk)"
    );
}

#[test]
fn scoped_lsb_bitselect_matches_bare() {
    // `@(posedge p::bus[0])` — an LSB bit-select of a scoped package vector arms
    // on the same net's bit 0 as bare `@(posedge bus[0])` (both iverilog: 4).
    let base = "package p; logic [3:0] bus; endpackage\n\
         module top; import p::*;\n  int cnt = 0;\n  initial bus = 0;\n  always #5 bus = bus + 1;\n  always @(posedge BSEL) cnt = cnt + 1;\n  initial begin #43 $display(\"cnt=%0d\", cnt); $finish; end\nendmodule\n";
    let (scoped, cs) = run(&base.replace("BSEL", "p::bus[0]"));
    let (bare, cb) = run(&base.replace("BSEL", "bus[0]"));
    assert_eq!(cs, Some(0));
    assert_eq!(cb, Some(0));
    assert!(scoped.contains("cnt=4"), "scoped LSB got:\n{scoped}");
    assert_eq!(scoped, bare, "scoped LSB bit-select must equal bare");
}

#[test]
fn scoped_nonlsb_bitselect_is_loud_like_bare() {
    // A non-LSB bit `@(posedge p::bus[1])` needs per-bit edge tracking vita lacks
    // — loud, exactly like bare `@(posedge bus[1])`.
    let (_, cs) = run(
        "package p; logic [3:0] bus; endpackage\n\
         module top; import p::*;\n  always @(posedge p::bus[1]) ;\n  initial begin #1 $finish; end\nendmodule\n",
    );
    assert_ne!(cs, Some(0), "non-LSB scoped bit-select must be loud");
}

#[test]
fn scoped_constant_alone_in_a_header_level_list_is_loud() {
    // A package CONSTANT in a process-header level list is a constant like a local
    // one: both oracles (iverilog 13.0, verilator 5.052) run the process ONCE at time
    // 0 and print `x=1` with a `$display` at 1. vita refuses a list with no live term
    // — BACK ON vita_pre's ROUTE after a slice that ran it (vita's `$finish` can end
    // time 0 before that run); beside a live term it runs (const_level_event_t0.rs).
    let (_, code) = run("package p; localparam int K = 3; endpackage\n\
         module top; int x; always @(p::K) x = x + 1; initial begin #1 $display(\"x=%0d\", x); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "an all-constant header list must stay loud");
}

#[test]
fn scoped_unknown_in_event_is_loud() {
    // An unknown scoped symbol is loud.
    let (_, code) = run(
        "package p; logic clk; endpackage\n\
         module top; int x; always @(p::nope) x = x + 1; initial begin #1 $finish; end endmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "unknown scoped symbol in event control must be loud"
    );
}
