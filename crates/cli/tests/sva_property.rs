//! SVA concurrent-assertion subset (v8, Phase-3): `assert property(@(clk) a
//! |-> b)` / `|=>`. iverilog 13.0 does NOT support concurrent assertions OR the
//! sampled-value functions ($past/$rose/$fell/$stable) — it rejects them with
//! "sorry: concurrent_assertion_item not supported" / "not defined by any
//! module". So this whole subset is HAND-IEEE pinned (no differential oracle),
//! like assoc arrays / interfaces / string methods. The desugar is a synthesized
//! clocked checker: `assert property(@(clk) a |-> b)` ≡ `always @(clk) if (a &&
//! !b) $error(...)`; `|=>` delays the antecedent one clock via a pending reg.

#[path = "sva_property_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

#[test]
fn sva_overlap_holds_no_error() {
    // a |-> b holds at every posedge where a is high → no $error, clean exit.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "should pass cleanly. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.to_lowercase().contains("assertion") && !out.to_lowercase().contains("assertion"),
        "no assertion violation expected:\nstderr={err}\nout={out}"
    );
}

#[test]
fn sva_overlap_violation_fires_error() {
    // at t=25 a=1,b=0 → a |-> b is violated → $error (exit class 1).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a violation must set exit class 1. stderr:\n{err}\nout:\n{out}"
    );
    let blob = format!("{err}{out}").to_lowercase();
    assert!(
        blob.contains("assertion"),
        "a violation diagnostic was expected:\nstderr={err}\nout={out}"
    );
}

#[test]
fn sva_nonoverlap_delays_one_clock() {
    // a |=> b: antecedent at clock T requires consequent at clock T+1. a is high
    // only at t=15; b must hold at t=25. Here b is LOW at t=25 → violation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |=> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "nonoverlap violation must set exit 1. stderr:\n{err}\nout:\n{out}"
    );
    let blob = format!("{err}{out}").to_lowercase();
    assert!(
        blob.contains("assertion"),
        "violation diagnostic expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_nonoverlap_holds_no_error() {
    // a |=> b: a high at t=15, b high at t=25 (next clock) → holds, no $error.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |=> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "nonoverlap should hold. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

// ── sampled-value functions (slice S3, hand-IEEE) ────────────────────────────
// $past(x)=value 1 clock ago; $rose/$fell=LSB 0→1 / 1→0; $stable=no change.
// Synthesized as prev-registers NBA-updated each clock in the checker process.

#[test]
fn sva_rose_fires_when_consequent_low() {
    // a rises (0→1) seen at the t=15 posedge while b is still 0 → $rose(a) |-> b
    // is violated exactly once.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) $rose(a) |-> b);\n\
         initial begin\n\
           #12 a=1;\n\
           #30 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "rose with low consequent must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_rose_holds_when_consequent_high() {
    // a rises while b is high → $rose(a) |-> b holds, and a STABLE a (no rise)
    // imposes no obligation (vacuous pass) even when b is low later.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) $rose(a) |-> b);\n\
         initial begin\n\
           #12 a=1; b=1;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "rose with high consequent holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_past_tracks_previous_value() {
    // b must equal a's value one clock earlier. Wired so it HOLDS, proving $past
    // delivers the prior sampled value (not the current one).
    //
    // NOTE (§16.13.5): at the FIRST posedge `$past(a)` is X (no history), so an
    // unguarded `b == $past(a)` would be `0 == X` = X — a non-match that fires (as
    // VCS/Questa do; the prior lenient X-as-hold was a false-negative). Guard with
    // a `started` reg so the meaningful tracking check begins at the 2nd clock; the
    // first cycle is vacuous, not a spurious fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, started=0;\n\
         reg [3:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) started<=1;\n\
         initial assert property(@(posedge clk) started |-> (b == $past(a)));\n\
         initial begin\n\
           a=4'd3; b=4'd0;\n\
           #10 a=4'd7; b=4'd3;\n\
           #10 a=4'd9; b=4'd7;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "$past tracking should hold. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_past_mismatch_fires() {
    // b deliberately does NOT equal a's previous value → violation.
    let (out, err, code) = run("module top;\n\
         reg clk=0;\n\
         reg [3:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) (b == $past(a)));\n\
         initial begin\n\
           a=4'd3; b=4'd0;\n\
           #10 a=4'd7; b=4'd9;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "$past mismatch must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_stable_detects_change() {
    // $stable(a) |-> b: when a is unchanged across a clock, b must hold. Make a
    // change so the antecedent is false (vacuous) at the change, then a stable
    // window with b low → violation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) $stable(a) |-> b);\n\
         initial begin\n\
           a=1; b=1;\n\
           #20 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "stable a with low b must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

// ── adversarial-review regressions (2026-06-14) ──────────────────────────────
// NOTE on X/Z (deliberate subset choice, NOT a bug): vitamin treats an X/Z
// antecedent OR consequent as "don't-fire" (a consistent X=don't-know policy).
// Strict IEEE 1800 §16.4.2 reads an X boolean as false (so an X consequent would
// fail), but the subset has no `disable iff`/reset qualification, so strict
// X-fail would make every $past-based assertion fire spuriously on its first
// clock (when $past is X). The lenient policy is documented and intentional.

#[test]
fn sva_nonoverlap_multibit_antecedent_is_boolean() {
    // Review F1: a multi-bit antecedent is a BOOLEAN (any nonzero = true), not its
    // LSB. `a=2'b10` (nonzero) must impose the |=> obligation; b low next clock
    // → violation. (The bug stored a's LSB=0 into the 1-bit pending reg → silent
    // pass.) Fixed by sampling reduction-OR of the antecedent.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [1:0] a=0; reg b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |=> b);\n\
         initial begin a=2'b10; b=0; #30 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "multi-bit |=> antecedent must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_sampled_hierarchical_signal_is_loud() {
    // Review F3: a hierarchical signal in a sampled-value function would be keyed
    // only by its last segment, silently aliasing two distinct signals onto one
    // prev-register. It must be a LOUD error instead.
    let (_out, err, code) = run("module sub; reg [7:0] x = 8'hAA; endmodule\n\
         module top;\n\
         reg clk=0; reg [3:0] x = 4'h3; sub u();\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) (x==4'h3) |-> ($past(x) != $past(u.x)));\n\
         initial begin #30 $finish; end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "hierarchical sampled arg must not silently pass. stderr:\n{err}"
    );
    assert!(
        err.to_lowercase().contains("hierarchical"),
        "expected a loud hierarchical-signal diagnostic:\n{err}"
    );
}

#[test]
fn sva_seq_delay_holds_no_error() {
    // same sequence but d=1 exactly when it completes (t35) -> holds, no $error.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "seq-delay completion with high consequent holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_delay_gap_breaks_no_obligation() {
    // b is LOW at its slot (t25) -> the pipeline thread drops; c high later imposes
    // NO obligation (vacuous), even with d=0.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=0; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a dropped sequence thread must impose no obligation. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_repeat_violation_fires() {
    // a[*3] |-> b: a high 3 consecutive clocks (t15,t25,t35), b=0 on the 3rd -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a[*3] |-> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=1; b=0;\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "3-consecutive repetition with low consequent must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_repeat_holds_no_error() {
    // a[*3] |-> b: b high on the completion clock -> holds.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a[*3] |-> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=1; b=0;\n\
           #10 a=1; b=1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "3-consecutive repetition with high consequent holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_nonoverlap_delays_one_clock() {
    // a ##1 b |=> c: sequence a@t15,b@t25 matches at t25; |=> obliges c at t35. c low -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b |=> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "nonoverlap seq with low consequent next clock must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_overlap_two_threads_both_checked() {
    // a ##1 b |-> c with a high at t15 AND t25 -> two overlapping antecedent threads.
    // thread A ends t25 (c=1 holds), thread B ends t35 (c=0 violates) -> fires once.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=1; b=1; c=1;\n\
           #10 a=0; b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the second overlapping thread must be enforced independently. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_antecedent_never_matches_vacuous() {
    // a never high -> no antecedent thread ever completes -> d ignored -> exit 0.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##1 c |-> d);\n\
         initial begin #40 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "an antecedent that never matches is vacuously true. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_first_clock_no_spurious() {
    // a ##1 b |-> c asserted from t=0: at the first posedge the pipeline reg is X, so
    // the check is `if(X) $error` = don't-fire (no thread legitimately started pre-t0).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b |-> c);\n\
         initial begin #8 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "X-init pipeline must not fire on the first clock. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no first-clock spurious violation expected:\n{err}\n{out}"
    );
}

// ── SVA SEQUENCE RANGES (slice S5, hand-IEEE) ────────────────────────────────
// Bounded constant ranges `##[m:n]` cycle-delay and `[*m:n]` consecutive
// repetition. Desugar = OR of the (n-m+1) fixed-delay alternatives (each a
// shift-register pipeline), match = any alternative completes. No AST change
// (reuses Sequence::Delay/Repeat min/max), no sim-ir bump. Hand-IEEE (no oracle).

#[test]
fn sva_seq_delay_range_upper_bound_fires() {
    // a ##[1:2] b |-> c: a@t15, b at t35 (delay 2, in [1:2]) with c=0 -> the
    // delay-2 alternative matches and fires (b is LOW at t25 so delay-1 misses).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:2] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "delay-2 alternative of ##[1:2] must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_delay_range_lower_bound_holds() {
    // a ##[1:2] b |-> c: b@t25 (delay 1) with c=1 -> the delay-1 alternative
    // holds; b is LOW at t35 so no delay-2 obligation -> clean pass.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:2] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=1;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "delay-1 alternative holding must pass cleanly. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_repeat_range_fires() {
    // a[*2:3] |-> b: a true 2 consecutive (t15,t25) completes the [*2] alternative
    // at t25 with b=0 -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a[*2:3] |-> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a 2-consecutive run must satisfy [*2:3] and fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_repeat_range_below_min_vacuous() {
    // a[*2:3] |-> b: a true only 1 clock (run=1 < min 2) -> no alternative matches
    // -> b ignored -> exit 0.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a[*2:3] |-> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a run shorter than the min repeat must impose no obligation. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

// ── SVA UNBOUNDED DELAY (slice S6, hand-IEEE) ────────────────────────────────
// `##[m:$]` — the consequent term may arrive ANY number of clocks (>=m) after
// the prefix. Cannot expand to fixed alternatives; desugar = an `armed` latch:
// once the prefix matches it latches (never resets), and every later term clock
// (>=m after) re-completes the match. Hand-IEEE (no oracle).

#[test]
fn sva_seq_delay_unbounded_fires() {
    // a ##[1:$] b |-> c: a@t15, b@t35 (delay 2, >=1) with c=0 -> the armed latch
    // (set by a@t15) makes b@t35 a match -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:$] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "an unbounded-delay match must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_delay_unbounded_min_excludes_early() {
    // a ##[2:$] b |-> c: b at t25 is only 1 clock after a@t15 (< min 2) -> NO match,
    // c ignored -> exit 0.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[2:$] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a term closer than the min delay must not match. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_delay_unbounded_latch_persists() {
    // a ##[1:$] b |-> c: a@t15. b@t25 holds (c=1). b@t45 (still armed, delay 3)
    // with c=0 -> fires -> proves the armed latch persists across clocks.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:$] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=1;\n\
           #10 a=0; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the armed latch must persist and fire on a later term. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_delay_unbounded_no_antecedent_vacuous() {
    // a never high -> the latch never arms -> b ignored -> exit 0 (X-init latch
    // stays don't-know, if(X) doesn't fire).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:$] b |-> c);\n\
         initial begin #40 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "no prefix match means the latch never arms. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

// ── SVA THROUGHOUT (slice S7, hand-IEEE) ─────────────────────────────────────
// `cond throughout seq` — boolean `cond` must hold at EVERY clock of seq's match
// window (start through end). Desugar = AND `|cond` into the seed and every
// shift-register stage of the synthesized pipeline, so a thread dies the instant
// cond drops. IR-0 over bounded inner sequences (unbounded inner = loud).

#[test]
fn sva_seq_throughout_holds_fires() {
    // g throughout a ##2 c |-> d: g high across the whole window (t15,t25,t35),
    // a ##2 c completes at t35 with d=0 -> the throughout passes and the
    // implication fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, g=0, a=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) g throughout a ##2 c |-> d);\n\
         initial begin\n\
           #10 g=1; a=1; c=0; d=0;\n\
           #10 g=1; a=0; c=0; d=0;\n\
           #10 g=1; a=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "throughout holding across the window must let the match fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_throughout_violated_kills_match() {
    // g throughout a ##2 c |-> d: g DROPS at the gap clock (t25), so the throughout
    // is broken -> the thread dies -> no match -> d (low) imposes no obligation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, g=0, a=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) g throughout a ##2 c |-> d);\n\
         initial begin\n\
           #10 g=1; a=1; c=0; d=0;\n\
           #10 g=0; a=0; c=0; d=0;\n\
           #10 g=1; a=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a dropped throughout condition must kill the match. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

// ── SVA GOTO / NONCONSECUTIVE REPETITION (slice S8, hand-IEEE) ────────────────
// `b[->n]` goto: the n-th occurrence of b (gaps allowed), match ends ON the n-th.
// `b[=n]` nonconsec: n occurrences of b, match may extend past the n-th (until
// the next b). Desugar = existence-latch FSM (per-stage boolean regs), which is
// exact for the |-> any-completion semantics. Hand-IEEE (no oracle).

#[test]
fn sva_seq_goto_fires_on_nth_b() {
    // a ##1 b[->2] |-> c: after a@t15, the 2nd b (gaps allowed) lands at t45
    // (b@t25 is the 1st, gap@t35, b@t45 the 2nd) with c=0 -> fires at t45.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[->2] |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the 2nd b (with a gap) must complete the goto and fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_goto_not_yet_nth_no_fire() {
    // a ##1 b[->2] |-> c: only ONE b after a (t25), never a 2nd -> no goto
    // completion -> c (low) imposes no obligation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[->2] |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "one b is not enough for [->2]. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_goto_first_b_immediate() {
    // a ##1 b[->1] |-> c: the FIRST b after a (t25) completes [->1]; c=0 -> fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[->1] |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the first b must complete [->1] and fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_nonconsec_extends_past_nth() {
    // a ##1 b[=1] ##1 c |-> d: after a, 1 b (t25), then c one-or-more clocks later
    // (t45, with a non-b gap at t35) -> d=0 at t45 fires. Proves [=n] lets the
    // match float past the n-th b (a non-b clock between the b and c).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[=1] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "[=1] must let c land a non-b clock after the b. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_nonconsec_broken_by_extra_b() {
    // a ##1 b[=1] ##1 c |-> d: a 2nd b (t35) before c makes it 2 b's, not 1 ->
    // the [=1] thread dies -> c at t45 imposes no obligation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[=1] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "an extra b must break [=1]. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}
