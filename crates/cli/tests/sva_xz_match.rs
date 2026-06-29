//! SVA boolean X/Z = NON-match (IEEE 1800 §16.13.5). A boolean expression in a
//! sequence/property matches ONLY if it evaluates to true (1); X/Z (and 0) are a
//! non-match. Concretely, for `a |-> b` with a true and b == X, the consequent
//! does not hold → the implication FAILS → the assertion must FIRE. The prior
//! code reduced the consequent with a plain `|b` (leaving X as X) so that
//! `a && !b` evaluated to X and `if(X)` took the no-fire branch — a false
//! NEGATIVE (a real failure silently missed). iverilog 13 supports no concurrent
//! assertions, so every expectation here is HAND-IEEE (cross-checked).
//!
//! Clock: `always #5 clk=~clk` → posedges at t=5,15,25,35,…; a value set at `#10`
//! is sampled at the t=15 posedge.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaxz_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn fires(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(1),
        "{ctx}: expected a violation (exit 1).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: a violation diagnostic was expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

fn holds(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(0),
        "{ctx}: expected a clean pass (exit 0).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: no violation expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

// ── overlap `|->`, boolean consequent == X → FIRE ──
#[test]
fn consequent_x_overlap_fires() {
    // b is never assigned → stays X. At t15 a=1 (sampled), consequent b=X is a
    // non-match → the implication fails → fire.
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg b;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) a |-> b);\n\
         endmodule\n");
    fires(&o, &e, c, "a |-> b with b==X");
}

// ── non-overlap `|=>`, boolean consequent == X → FIRE ──
#[test]
fn consequent_x_nonoverlap_fires() {
    // a=1 at t15 → obligation at t25; b=X at t25 → non-match → fire.
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg b;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #30 $finish; end\n\
         assert property (@(posedge clk) a |=> b);\n\
         endmodule\n");
    fires(&o, &e, c, "a |=> b with b==X");
}

// ── consequent EXPRESSION unknown (compare against X) → FIRE ──
#[test]
fn consequent_expr_x_fires() {
    // d is X → (d == 4'h5) is X (unknown) → consequent non-match → fire.
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg [3:0] d;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) a |-> (d == 4'h5));\n\
         endmodule\n");
    fires(&o, &e, c, "a |-> (d==5) with d==X");
}

// ── sequence consequent boolean == X → FIRE ──
#[test]
fn seq_consequent_x_fires() {
    // a=1 at t15 → obligation `##1 b` at t25; b=X → non-match → fire.
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg b;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #30 $finish; end\n\
         assert property (@(posedge clk) a |-> ##1 b);\n\
         endmodule\n");
    fires(&o, &e, c, "a |-> ##1 b with b==X");
}

// ── REGRESSION guard: antecedent X is a non-match → vacuous → NO fire ──
#[test]
fn antecedent_x_vacuous_holds() {
    // a is X always → the antecedent never matches → vacuously true → no fire.
    let (o, e, c) = run("module top;\n\
         reg clk=0, b=0;\n\
         reg a;\n\
         always #5 clk=~clk;\n\
         initial begin #30 $finish; end\n\
         assert property (@(posedge clk) a |-> b);\n\
         endmodule\n");
    holds(&o, &e, c, "X |-> b is vacuous");
}

// ── REGRESSION guard: a multi-bit DEFINED nonzero consequent still HOLDS ──
#[test]
fn multibit_nonzero_consequent_holds() {
    // d = 4'b0100 (nonzero, defined) is truthy → consequent matches → no fire.
    // (Guards that the X-strict wrap keeps definitely-nonzero values a match,
    // not just `=== 1`.)
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg [3:0] d=4'b0100;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) a |-> d);\n\
         endmodule\n");
    holds(&o, &e, c, "a |-> 4'b0100 holds");
}

// ── SANITY: a DEFINED false consequent still fires (normal violation path) ──
#[test]
fn consequent_defined_false_fires() {
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) a |-> b);\n\
         endmodule\n");
    fires(&o, &e, c, "a |-> b with b==0");
}

// ── SANITY: a DEFINED true consequent holds ──
#[test]
fn consequent_defined_true_holds() {
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0, b=1;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) a |-> b);\n\
         endmodule\n");
    holds(&o, &e, c, "a |-> b with b==1");
}

// ── property-level `always (a |-> b)` consequent X → FIRE (prop-expr path) ──
#[test]
fn prop_always_consequent_x_fires() {
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg b;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) always (a |-> b));\n\
         endmodule\n");
    fires(&o, &e, c, "always(a|->b) with b==X");
}

// ── property-level `a implies b` consequent X → FIRE (prop-expr or/not path) ──
#[test]
fn prop_implies_consequent_x_fires() {
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0;\n\
         reg b;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) a implies b);\n\
         endmodule\n");
    fires(&o, &e, c, "a implies b with b==X");
}

// ── disable iff(X) must NOT mask a real violation (§16.13.5: X ≠ definitely-true) ──
#[test]
fn disable_iff_x_does_not_mask_violation() {
    // a |-> b is violated at t15 (a=1, b=0). disable iff(d) with d==X: X is not
    // definitely true → the attempt is NOT disabled → the violation must FIRE.
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         reg d;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) disable iff (d) a |-> b);\n\
         endmodule\n");
    fires(
        &o,
        &e,
        c,
        "disable iff(X) must not mask the a|->b violation",
    );
}

// ── REGRESSION guard: disable iff(1'b1) genuinely disables (no fire) ──
#[test]
fn disable_iff_true_masks_violation() {
    let (o, e, c) = run("module top;\n\
         reg clk=0, a=0, b=0, d=1;\n\
         always #5 clk=~clk;\n\
         initial begin #10 a=1; #20 $finish; end\n\
         assert property (@(posedge clk) disable iff (d) a |-> b);\n\
         endmodule\n");
    holds(&o, &e, c, "disable iff(1) disables the attempt");
}

// ── REGRESSION guard: a bare property `b` that is DEFINED-true holds ──
#[test]
fn prop_bare_true_holds() {
    let (o, e, c) = run("module top;\n\
         reg clk=0, b=1;\n\
         always #5 clk=~clk;\n\
         initial begin #20 $finish; end\n\
         assert property (@(posedge clk) always (b));\n\
         endmodule\n");
    holds(&o, &e, c, "always(b) with b==1");
}
