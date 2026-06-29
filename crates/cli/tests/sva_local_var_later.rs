//! SVA sequence LOCAL VARIABLE read in a LATER ANTECEDENT term (slice #3, IEEE
//! 1800-2017 §16.10). Extends the single-capture data-shift-register subset so a
//! local var captured in the antecedent (`(a, d=x) ##1 grant`) may be READ not only
//! in the consequent but also in a LATER antecedent term, e.g.
//! `(a, d=x) ##1 (b && (d==7)) ##1 c |-> e`.
//!
//! ── NO EXTERNAL ORACLE ──────────────────────────────────────────────────────
//! iverilog 13.0 rejects all concurrent assertions / local variables; verilator is
//! absent. Verification is therefore (1) HAND CYCLE/STAGE trace and (2) a
//! vita-INTERNAL DIFFERENTIAL: the same data dependency built WITHOUT a local var by
//! sampling the captured signal into an explicit RTL shift register
//! (`always @(posedge clk) xq1 <= x;`) and reading `xq1` at the later term. Driven
//! with identical stimulus the two MUST fire/hold identically — a wrong data_chain
//! stage flips the comparison.
//!
//! ── STAGE-INDEX RULE (the whole correctness argument) ───────────────────────
//! For a read at antecedent term index `j > cap_idx`, the value is `data_chain[S-1]`
//! where `S = sum of the FIXED hops from cap_idx+1 .. = j` — a COMPILE-TIME CONSTANT
//! (a ranged hop on the capture→read path is loud-rejected at flatten time, so S is
//! never ambiguous). `data_chain[k]` = the captured value delayed by (k+1) clocks.
//! A read one `##1` after the capture (S=1) reads `data_chain[0]` = the value sampled
//! at the CAPTURE clock, observed at the read clock (one clock later).
//!
//! ── CLOCK / SAMPLING CONVENTION (shared with sva_local_var.rs) ───────────────
//! `clk` toggles every 5ns from t=5, so posedges land at t=10,20,30,40,50. A stimulus
//! `#10 sig=v` sets `sig` at a posedge instant; the clocked checker samples the NEW
//! value (the initial block's blocking assigns precede the checker at that timestep).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svalvl_{}_{n}", std::process::id()));
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

fn loud(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_ne!(code, Some(0), "{ctx}: must not silently pass.\n{err}{out}");
    assert!(
        format!("{err}{out}").contains("VITA-E"),
        "{ctx}: expected a loud diagnostic.\nstderr:\n{err}\nout:\n{out}"
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  S=1 — read at antecedent term cap+1 (`data_chain[0]`)
//  `(a, d=x) ##1 (b && (d==7)) |-> e` — d read one `##1` after capture.
//  The antecedent MATCHES only when the captured x (sampled at the a-clock) == 7 at
//  the read clock; on a match the consequent `e` is checked. With `e=0` a MATCH
//  always FIRES, so the antecedent-read value directly gates the verdict — the
//  sharpest off-by-one discriminator (x changes EVERY clock).
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn later_read_s1_match_fires() {
    // t10 (a-clock): a=1, x=7 → capture d=7 (data_chain[0] <= 7 via NBA).
    // t20 (read):    b=1, x=3 → term1 = b && (d==7); d = data_chain[0] = x@t10 = 7 →
    //                7==7 TRUE → antecedent MATCHES → check e=0 → FALSE → FIRE @t20.
    // A WRONG stage (combinational x@t20=3, or x@t30) would read 3 → 3==7 FALSE → no
    // match → HOLD. So the FIRE proves the read = x@t10 (data_chain[0]).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 (b && (d==7)) |-> e);\n\
         initial begin\n\
           #10 a=1; x=7;\n\
           #10 a=0; b=1; x=3; e=0;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "later read S=1: d=x@t10=7 matches, e=0 → fire",
    );
}

#[test]
fn later_read_s1_offbyone_discriminator_holds() {
    // The MANDATORY off-by-one teeth: capture x=3 @t10, x=7 @t20.
    // Correct (S=1, data_chain[0]=x@t10=3): d==7 → 3==7 FALSE → no match → HOLD.
    // A combinational misread (x@t20=7) would MATCH → check e=0 → FIRE. The HOLD
    // proves the read is delayed exactly one clock, not combinational.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 (b && (d==7)) |-> e);\n\
         initial begin\n\
           #10 a=1; x=3;\n\
           #10 a=0; b=1; x=7; e=0;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "later read S=1 off-by-one: d=x@t10=3 ≠ 7 → no match → hold",
    );
}

#[test]
fn later_read_s1_differential_matches_shiftreg() {
    // vita-INTERNAL DIFFERENTIAL: the local-var read at term cap+1 must equal an
    // explicit one-stage shift register `xq1 <= x` read at the same term. Identical
    // stimulus → identical verdict. A wrong data_chain stage flips one of them.
    let stim = "\n         initial begin\n\
           #10 a=1; x=7;\n\
           #10 a=0; b=1; x=3; e=0;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n         endmodule\n";
    let lv = format!(
        "module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 (b && (d==7)) |-> e);{stim}"
    );
    let sr = format!(
        "module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0, xq1=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) xq1 <= x;\n\
         initial assert property(@(posedge clk) (a) ##1 (b && (xq1==7)) |-> e);{stim}"
    );
    let (lo, le, lc) = run(&lv);
    let (so, se, sc) = run(&sr);
    fires(&lo, &le, lc, "differential S=1 local-var");
    fires(&so, &se, sc, "differential S=1 shift-register");
    assert_eq!(lc, sc, "S=1 differential: exit codes must match");
}

// ════════════════════════════════════════════════════════════════════════════
//  S=2 — read at antecedent term cap+2 (`data_chain[1]`), two `##1` hops.
//  `(a, d=x) ##1 m ##1 (b && (d==7))`.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn later_read_s2_two_hop_match_fires() {
    // t10: a=1, x=7 → capture d=7.   t20: m=1, x=1.   t30: b=1, x=2.
    // S=2 → data_chain[1] = x@t10 = 7. term2 = b && (d==7) → 7==7 → MATCH → e=0 → FIRE.
    // A wrong stage (x@t20=1 or x@t30=2) would not equal 7 → no match → HOLD.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, m=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 m ##1 (b && (d==7)) |-> e);\n\
         initial begin\n\
           #10 a=1; x=7;\n\
           #10 a=0; m=1; x=1;\n\
           #10 m=0; b=1; x=2; e=0;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "later read S=2: d=x@t10=7 at t30 → match → fire",
    );
}

#[test]
fn later_read_s2_differential_matches_double_shiftreg() {
    // DIFFERENTIAL: S=2 read must equal a TWO-stage shift register `xq2<=xq1<=x`.
    let stim = "\n         initial begin\n\
           #10 a=1; x=7;\n\
           #10 a=0; m=1; x=1;\n\
           #10 m=0; b=1; x=2; e=1;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n         endmodule\n";
    let lv = format!(
        "module top;\n\
         reg clk=0, a=0, m=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 m ##1 (b && (d==7)) |-> e);{stim}"
    );
    let sr = format!(
        "module top;\n\
         reg clk=0, a=0, m=0, b=0, e=0;\n\
         reg [7:0] x=0, xq1=0, xq2=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) begin xq1 <= x; xq2 <= xq1; end\n\
         initial assert property(@(posedge clk) (a) ##1 m ##1 (b && (xq2==7)) |-> e);{stim}"
    );
    let (lo, le, lc) = run(&lv);
    let (so, se, sc) = run(&sr);
    // x@t10=7 read at t30, e=1 → match → e true → HOLD on both.
    holds(&lo, &le, lc, "differential S=2 local-var");
    holds(&so, &se, sc, "differential S=2 shift-register");
    assert_eq!(lc, sc, "S=2 differential: exit codes must match");
}

// ════════════════════════════════════════════════════════════════════════════
//  BACK-TO-BACK overlapping attempts — distinct in-flight values at distinct stages.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn back_to_back_reads_prior_capture_holds() {
    // The pipeline teeth: attempt A captures x=3 @t10 and reads at t20; attempt B
    // captures x=7 @t20 (the SAME clock A reads). A's read must see the PRIOR-clock
    // value (data_chain[0] = x@t10 = 3), NOT B's same-clock capture (x@t20 = 7).
    //   A: d=3 → 3==7 FALSE → no match → no fire.
    //   B: b=0 @t30 → term1 false → no match → no fire.
    // Correct → HOLD (exit 0). A "read this-clock's capture" bug would read 7 → A
    // matches → e=0 → FIRE. The HOLD proves stage[0] is the prior clock.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 (b && (d==7)) |-> e);\n\
         initial begin\n\
           #10 a=1; x=3;\n\
           #10 a=1; x=7; b=1; e=0;\n\
           #10 a=0;      b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "back-to-back: A reads prior x@t10=3, not B's same-clock x@t20=7",
    );
}

#[test]
fn back_to_back_two_in_flight_differential() {
    // Two attempts in flight reading DISTINCT captured values; compared against the
    // explicit shift register. A captures 7 @t10 (matches @t20 → fire), B captures 3
    // @t20 (no match @t30). The local-var pipeline must agree with `xq1<=x` exactly.
    let stim = "\n         initial begin\n\
           #10 a=1; x=7;\n\
           #10 a=1; x=3; b=1; e=0;\n\
           #10 a=0;      b=1; e=0;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n         endmodule\n";
    let lv = format!(
        "module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 (b && (d==7)) |-> e);{stim}"
    );
    let sr = format!(
        "module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0, xq1=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) xq1 <= x;\n\
         initial assert property(@(posedge clk) (a) ##1 (b && (xq1==7)) |-> e);{stim}"
    );
    let (lo, le, lc) = run(&lv);
    let (so, se, sc) = run(&sr);
    fires(
        &lo,
        &le,
        lc,
        "back-to-back differential local-var (A fires @t20)",
    );
    fires(&so, &se, sc, "back-to-back differential shift-register");
    assert_eq!(lc, sc, "back-to-back differential: exit codes must match");
}

// ════════════════════════════════════════════════════════════════════════════
//  REGRESSION — the consequent read continues to use its existing (correct) stage.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn consequent_read_still_works_regression() {
    // `(a, d=x) ##1 b |-> (rdata == d)`: capture@t10, completion@t20, consequent reads
    // the captured d (=5) at t20. rdata=7 ≠ 5 → FIRE @t20. (The consequent path is
    // unchanged by slice #3.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         reg [7:0] x=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 b |-> (rdata == d));\n\
         initial begin\n\
           #10 a=1; x=5;\n\
           #10 a=0; b=1; rdata=7;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "consequent read regression: rdata=7 ≠ captured 5",
    );
}

#[test]
fn later_antecedent_and_consequent_both_read() {
    // d read in BOTH a later antecedent term AND the consequent of one property:
    // `(a, d=x) ##1 (b && (d==7)) |-> (rdata == d)`. Capture 7 @t10; term1 matches
    // (d==7 @t20); consequent rdata==d (=7) @t20. rdata=7 → HOLD.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         reg [7:0] x=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##1 (b && (d==7)) |-> (rdata == d));\n\
         initial begin\n\
           #10 a=1; x=7;\n\
           #10 a=0; b=1; rdata=7;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "both reads: term1 matches (d==7), consequent rdata==7 → hold",
    );
}

// ════════════════════════════════════════════════════════════════════════════
//  KEEP LOUD — the correctness boundary (correct-or-loud).
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ranged_hop_capture_to_read_is_loud() {
    // A RANGED hop `##[1:3]` between the capture and the read makes the read-stage S
    // ambiguous (the sliding-OR unions multiple offsets — no single data value). This
    // is the RED thread-table convergence → must stay LOUD (E3009), never a guess.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d=x) ##[1:3] (b && (d==7)) |-> e);\n\
         initial begin #10 a=1; x=7; #50 $finish; end\n\
         endmodule\n");
    loud(
        &out,
        &err,
        code,
        "ranged ##[1:3] hop on the capture→read path",
    );
}

#[test]
fn read_at_capture_term_is_loud() {
    // Read in the SAME term as the capture (`a && (d==7), d=x`): index == cap_idx →
    // undefined data stage → LOUD.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a && (d==7), d=x) ##1 b |-> e);\n\
         initial begin #10 a=1; x=7; #50 $finish; end\n\
         endmodule\n");
    loud(
        &out,
        &err,
        code,
        "read at the capture term (index == cap_idx)",
    );
}

#[test]
fn read_before_capture_earlier_term_is_loud() {
    // Read in an EARLIER term than the capture (`(a && (d==7)) ##1 (b, d=x)`): index <
    // cap_idx → the value is not yet captured → LOUD (never a silent guess).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a && (d==7)) ##1 (b, d=x) |-> e);\n\
         initial begin #10 a=1; x=7; #50 $finish; end\n\
         endmodule\n");
    loud(
        &out,
        &err,
        code,
        "read in a term BEFORE the capture (index < cap_idx)",
    );
}

#[test]
fn multi_capture_later_read_is_loud() {
    // Two captures (`d=x, f=x`) + a later antecedent read: multiple writes converge /
    // alias → LOUD (the single-capture invariant is preserved).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; int f; (a, d=x, f=x) ##1 (b && (d==7)) |-> e);\n\
         initial begin #10 a=1; x=7; #50 $finish; end\n\
         endmodule\n");
    loud(
        &out,
        &err,
        code,
        "multi-capture with a later antecedent read",
    );
}

#[test]
fn self_referential_capture_is_loud() {
    // A self-referential capture `(a, d = d + 1)` reads d inside its own capture
    // expression (no defined value at its own term) → LOUD, even though the read is at
    // the capture term.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, e=0;\n\
         reg [7:0] x=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (a, d = d + 1) ##1 (b && (d==7)) |-> e);\n\
         initial begin #10 a=1; x=7; #50 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "self-referential capture (d = d + 1)");
}
