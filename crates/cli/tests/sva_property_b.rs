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

// ── SVA WITHIN (slice S9, hand-IEEE) ─────────────────────────────────────────
// `seq1 within seq2` — seq1 must match entirely INSIDE a match of seq2 (seq1's
// start >= seq2's start, seq1's end <= seq2's end); the within match ends at
// seq2's end. Desugar (bounded both) = match_2 & OR_{i=0}^{L-k1} reg^i(match_1)
// (seq1 completed within seq2's L-clock window). Hand-IEEE (no oracle).

#[test]
fn sva_seq_within_holds_fires() {
    // a within (b ##2 c) |-> d: window b@t15 .. c@t35; a@t25 is inside -> the
    // within match completes at t35 with d=0 -> fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a within b ##2 c |-> d);\n\
         initial begin\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a within the b..c window must complete and fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_within_a_outside_window_no_match() {
    // a within (b ##2 c) |-> d: a never occurs inside the b@t15..c@t35 window ->
    // no within match -> d (low) imposes no obligation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a within b ##2 c |-> d);\n\
         initial begin\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a outside the seq2 window must not match within. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

// ── module-level `assert property` (slice S10, hand-IEEE) ─────────────────────
// A concurrent assertion may appear as a module item (not only inside an
// initial/always). The parser wraps it in a synthetic `initial` so it flows
// through the same `pending_sva` collection; the checker is materialized at
// module level regardless, so this is a pure parser-placement change (no AST
// shape change). iverilog rejects it (concurrent assertions unsupported).

#[test]
fn sva_module_level_violation_fires() {
    // `assert property(...)` at MODULE level (no enclosing initial) must still
    // synthesize the clocked checker. At t=25 a=1,b=0 -> violation -> exit 1.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) a |-> b);\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "module-level assert property must check. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "a violation diagnostic was expected:\nstderr={err}\nout={out}"
    );
}

#[test]
fn sva_module_level_holds_no_error() {
    // module-level assert that always holds -> clean exit 0, no diagnostic.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) a |-> b);\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "module-level assert that holds should pass. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_module_level_immediate_assert_is_loud() {
    // A bare immediate `assert (expr)` is procedural-only; at module level it is
    // a loud parse error (only `assert property` is a module item).
    let (out, err, code) = run("module top;\n\
         reg a=1;\n\
         assert (a);\n\
         initial #10 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "immediate assert at module level must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

// ── unbounded consecutive repeat `b[*m:$]` (slice S13, hand-IEEE) ─────────────
// `b[*m:$]` = b true for >= m consecutive clocks. Cannot fan out (unbounded), so
// it lowers to a gated run-latch: a chain of 1-bit regs c_1..c_m where
// c_1 = act & |b, c_k = reg(c_{k-1}) & |b, and the top c_m self-latches
// ((reg(c_{m-1})|reg(c_m)) & |b) to saturate at ">= m". match = c_m. Boolean
// operand only (S8 goto/nonconsec precedent); `[*0:$]` (empty match) deferred.
// iverilog rejects concurrent assertions, so hand-IEEE pinned.

#[test]
fn sva_seq_consec_unbounded_fires() {
    // b[*2:$] |-> c : at t=25 b has been high 2 consecutive posedges (t15,t25)
    // -> run>=2 -> obligation c; c=0 -> violation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*2:$] |-> c);\n\
         initial begin\n\
           #10 b=1; c=1;\n\
           #10 b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "run>=2 with c low must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_consec_unbounded_holds_no_error() {
    // b[*2:$] |-> c with c high whenever run>=2 -> holds, no $error.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*2:$] |-> c);\n\
         initial begin\n\
           #10 b=1; c=1;\n\
           #10 b=1; c=1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "run>=2 with c high should hold. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_consec_unbounded_gap_resets_run() {
    // b[*2:$] |-> c : a 0 in the middle breaks the consecutive run, so it never
    // reaches 2 -> no obligation ever -> vacuous even though c stays low.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*2:$] |-> c);\n\
         initial begin\n\
           #10 b=1; c=0;\n\
           #10 b=0; c=0;\n\
           #10 b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a gap must reset the consecutive run. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_seq_consec_unbounded_saturates_past_min() {
    // b[*2:$] |-> c : the obligation persists for run lengths 3,4,... (the >= m
    // self-latch). c high at run=2 (t25) but low at run=3 (t35) -> fires at t35.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*2:$] |-> c);\n\
         initial begin\n\
           #10 b=1; c=1;\n\
           #10 b=1; c=1;\n\
           #10 b=1; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "obligation must persist past the minimum run. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_consec_unbounded_mid_sequence_fires() {
    // a ##1 b[*2:$] ##1 c |-> d : a@t15, then b>=2 consec (t25,t35), then c@t45,
    // d=0 -> fires. Exercises [*m:$] as a MID-sequence term (not just terminal).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) a ##1 b[*2:$] ##1 c |-> d);\n\
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
        Some(1),
        "mid-sequence [*m:$] then c must complete and fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_seq_consec_unbounded_empty_is_loud() {
    // `b[*0:$]` as a LEADING / standalone antecedent has an empty (zero-rep) SEED
    // alternative — the empty match's -1 start offset is unrepresentable here →
    // honest-loud at elaborate (2026-06-25; in SUFFIX/MIDDLE it is supported, see
    // cli `sva_empty_match.rs`).
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*0:$] |-> c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "leading b[*0:$] must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_seq_consec_unbounded_nonbool_operand_is_loud() {
    // A non-boolean operand (a chained repeat) for `[*m:$]` is deferred -> loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*2][*2:$] |-> c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "non-boolean [*m:$] operand must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

// ── repetition-count cap (slice S13 review fix) ──────────────────────────────
// Every SVA repetition count synthesizes O(count) helper regs (or fans out one
// alternative per count), so an absurd literal would hang elaboration. Each form
// — unbounded `[*m:$]`, goto/nonconsec `[->n]`/`[=n]`, and bounded `[*n]`/`[*m:n]`
// (whose per-count term-length the post-expansion alternative cap misses) — is
// capped at SVA_SEQ_ALT_CAP (256): a count above it is a loud E3009, not a hang.

#[test]
fn sva_seq_consec_unbounded_over_cap_is_loud() {
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*300:$] |-> c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "[*m:$] over the count cap must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_seq_goto_over_cap_is_loud() {
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[->300] |-> c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "[->n] over the count cap must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_seq_consec_bounded_over_cap_is_loud() {
    // `b[*300]` (exact) is a single alternative of 300 terms — the
    // post-expansion alternative cap misses it; the per-count cap catches it.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         assert property(@(posedge clk) b[*300] |-> c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "[*n] over the count cap must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

// ── assertion action block (slice S11, hand-IEEE) ────────────────────────────
// `assert property(...) [pass_stmt] [else fail_stmt]` (IEEE 1800 §16.14.1). The
// fail statement (after `else`) replaces the default `$error` on a violation;
// the pass statement runs on a NON-VACUOUS success (antecedent matched AND
// consequent held — a hand-IEEE choice: pass does not fire on vacuous success).
// A bare `assert property(...);` keeps the default $error (byte-identical to
// before). AST flip (pass/fail fields); sim-ir unchanged.

#[test]
fn sva_action_custom_fail_replaces_default() {
    // else <fail>: the custom fail action runs on violation (and the default
    // "Assertion property violation" text is NOT used).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b) else $error(\"CUSTOMFAILXYZ\");\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    let blob = format!("{err}{out}");
    assert_eq!(
        code,
        Some(1),
        "violation -> exit 1. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        blob.contains("CUSTOMFAILXYZ"),
        "custom fail text expected:\n{blob}"
    );
    assert!(
        !blob.contains("Assertion property violation"),
        "default text must be replaced:\n{blob}"
    );
}

#[test]
fn sva_action_fail_fatal_exits() {
    // else $fatal: a custom fatal fail action fires and exits nonzero.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b) else $fatal(1, \"BOOMFATAL\");\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "fatal fail must exit nonzero. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").contains("BOOMFATAL"),
        "fatal message expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_action_pass_runs_on_success() {
    // pass_stmt runs when the property holds non-vacuously (a&&b at a posedge).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b) $display(\"PROPPASSXYZ\");\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "holds -> exit 0. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").contains("PROPPASSXYZ"),
        "pass action expected on success:\n{err}\n{out}"
    );
}

#[test]
fn sva_action_pass_not_on_vacuous() {
    // pass_stmt must NOT run on vacuous success (antecedent never true).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b) $display(\"VACPASSXYZ\");\n\
         initial begin\n\
           #10 a=0; b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "vacuous -> exit 0. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").contains("VACPASSXYZ"),
        "pass action must not fire on vacuous success:\n{err}\n{out}"
    );
}

#[test]
fn sva_action_pass_and_fail() {
    // `pass; else fail;` — pass on the holding clock, fail on the violating one.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b) $display(\"OKPASSXYZ\"); else $error(\"BADFAILXYZ\");\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    let blob = format!("{err}{out}");
    assert_eq!(code, Some(1), "violation -> exit 1. {blob}");
    assert!(
        blob.contains("OKPASSXYZ"),
        "pass action on hold expected:\n{blob}"
    );
    assert!(
        blob.contains("BADFAILXYZ"),
        "custom fail action on violation expected:\n{blob}"
    );
}

#[test]
fn sva_action_default_fail_unchanged() {
    // No action block -> default $error("Assertion property violation").
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "violation -> exit 1. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").contains("Assertion property violation"),
        "default fail text expected:\n{err}\n{out}"
    );
}

// ── disable iff (slice S12, hand-IEEE) ───────────────────────────────────────
// `assert property(@(clk) disable iff (rst) seq |-> cons)` (IEEE 1800 §16.12.7):
// when the (clock-sampled) reset is true, the attempt is aborted — no violation,
// no pass — and in-flight pipeline/pending state is cleared so no obligation
// survives the reset. Desugar: fire condition gated with `!|rst`; every
// obligation NBA (pipeline + pend) becomes `rst ? 1'b0 : rhs`. Absent, the
// checker is byte-identical to before. AST flip; sim-ir unchanged. iverilog
// rejects concurrent assertions, so hand-IEEE pinned.

#[test]
fn sva_disable_iff_suppresses_violation() {
    // rst high during the (only) violation window -> aborted -> no fire, exit 0.
    let (out, err, code) = run("module top;\n\
         reg clk=0, rst=1, a=1, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) disable iff (rst) a |-> b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "disable iff must suppress the violation. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_disable_iff_inactive_fires() {
    // Same trace, rst low: the disable is inert -> the violation fires normally.
    let (out, err, code) = run("module top;\n\
         reg clk=0, rst=0, a=1, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) disable iff (rst) a |-> b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "inactive disable must not block firing. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_disable_iff_kills_inflight_sequence() {
    // a ##2 c |-> d : a@t15 starts an attempt that would mature at t35. rst@t25
    // (mid-sequence) clears the pipeline -> the attempt is killed -> no match at
    // t35 even though c=1,d=0 -> exit 0.
    let (out, err, code) = run("module top;\n\
         reg clk=0, rst=0, a=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) disable iff (rst) a ##2 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; rst=1;\n\
           #10 c=1; d=0; rst=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "disable iff must kill the in-flight sequence attempt. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_disable_iff_inflight_control_no_disable_fires() {
    // Control: the SAME sequence trace WITHOUT disable iff DOES fire (proving the
    // suppression above is the disable, not a vacuous trace).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##2 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0;\n\
           #10 c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the un-disabled sequence must fire. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_disable_iff_missing_iff_is_loud() {
    // `disable (rst)` without `iff` is a loud parse error.
    let (out, err, code) = run("module top;\n\
         reg clk=0, rst=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) disable (rst) a |-> b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "`disable` without `iff` must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_consequent_sequence_first_term_fails() {
    // a |-> b ##1 c : a high but b low at t15 -> consequent cannot start -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b ##1 c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "first consequent term fails. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_consequent_sequence_second_term_fails() {
    // a |-> b ##1 c : b holds at t15 but c low at t25 -> fire at t25.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b ##1 c);\n\
         initial begin\n\
           #10 a=1; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "second consequent term fails. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_consequent_repeat_fails() {
    // a |-> b[*2] : b must hold for 2 consecutive clocks; it drops at t25 -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b[*2]);\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #10 a=0; b=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "consequent repeat must hold 2 clocks. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_consequent_nonoverlap_sequence_fails() {
    // a |=> b ##1 c : consequent starts the clock AFTER a. a@t15 -> b due t25,
    // c due t35; c low at t35 -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |=> b ##1 c);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1;\n\
           #10 b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "nonoverlap consequent sequence fails. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

#[test]
fn sva_consequent_sequence_vacuous_no_fire() {
    // a never true -> no obligation -> no fire even though b/c never match.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b ##1 c);\n\
         initial begin\n\
           #10 a=0; b=0; c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "vacuous antecedent -> no obligation. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

#[test]
fn sva_consequent_range_is_loud() {
    // A ranged consequent (multiple alternatives) is deferred -> loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b ##[1:2] c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "ranged consequent must be loud. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn sva_multiclock_consequent_at_now_accepted() {
    // Slice A3: a consequent `@(c2)` on a `|=>` is now the CANONICAL multi-clock
    // pattern (two-process handoff), no longer loud. With a=0 the antecedent never
    // holds → handoff stays 0 → clean. (Behavioral coverage lives in sva_multiclock.rs.)
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2) b);\n\
         initial #30 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "the canonical multi-clock pattern is accepted; a=0 → no fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("multi-clock"),
        "no longer a multi-clock rejection:\n{err}\n{out}"
    );
}

#[test]
fn sva_multiclock_midseq_at_now_synthesized() {
    // SLICE N2a-1: a second `@` mid-sequence (after `##1`) is a cross-clock SEQUENCE
    // antecedent `@(c1) a ##1 @(c2) b |-> d`, now synthesized (was loud). Here a=0 →
    // no c1 arm → vacuous → clean (exit 0), and NOT a "multi-clock" reject. The full
    // value-pinned behavior lives in `sva_crossclock.rs`.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0, d=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a ##1 @(posedge c2) b |-> d);\n\
         initial #30 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "cross-clock seq with a=0 is vacuous → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("unsupported")
            && !format!("{err}{out}").to_lowercase().contains("multi-clock"),
        "must be synthesized, not a loud multi-clock reject:\n{err}\n{out}"
    );
}

#[test]
fn sva_multiclock_normal_or_always_unaffected() {
    // The OR-clock fix is scoped to concurrent-assertion clocks ONLY: a normal
    // `always @(posedge c1 or posedge c2)` process (clock-gen / async-reset
    // idiom) must still work.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0;\n\
         reg [7:0] hits=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         always @(posedge c1 or posedge c2) hits = hits + 1;\n\
         initial begin #40 $display(\"HITS=%0d\", hits); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "normal OR-edge always must work. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").contains("HITS="),
        "the always block must run:\n{err}\n{out}"
    );
}

#[test]
fn sva_consequent_sequence_multibit_antecedent() {
    // Regression (S14 review HIGH): a multi-bit antecedent that is truthy but has
    // bit0=0 (2'b10) must still carry the sequence-consequent obligation. The
    // due-advance must booleanize the seed (RedOr), not BitAnd a multi-bit value
    // (2'b10 & 2'b01 = 0 would silently drop the obligation). a=2'b10 & b@t15,
    // c=0@t25 -> b##1c is obligated and fails at t25.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         reg [1:0] a=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b ##1 c);\n\
         initial begin\n\
           #10 a=2'b10; b=1; c=0;\n\
           #10 a=0; b=0; c=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "multi-bit truthy antecedent must carry the obligation. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

// ── SVA SEQUENCES (slice S4, hand-IEEE) ──────────────────────────────────────
// Bounded compile-time-constant `##n` cycle-delay + `[*n]` consecutive repetition
// in the ANTECEDENT, for both |-> and |=>. iverilog 13.0 rejects ALL concurrent
// assertions (even bare |->) at COMPILE, so these are hand-IEEE pinned (no oracle).
// Desugar = a shift-register pipeline of 1-bit pending regs synthesized into the
// clocked checker. `a ##1 b ##1 c |-> d` becomes
//   always @(posedge clk) begin
//     if ((s2 & |c) & !d) $error;   // CHECK reads prior-clock pipeline state first
//     s1 <= |a; s2 <= s1 & |b;      // NBA shift; stage0 re-seeds every clock (overlap)
//   end
// Clock posedges at t=5,15,25,35,...; stimulus driven at t=10,20,30,... so each value
// is stable when sampled at the following posedge. s1/s2 are X-init so the first clocks
// produce `if(X)` = don't-fire (Verilog X-condition is false — the lenient-X policy).

#[test]
fn sva_seq_delay_violation_fires() {
    // a ##1 b ##1 c |-> d: a@t15, b@t25, c@t35 -> sequence ends t35 with d=0 -> fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "seq-delay completion with low consequent must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}\n{out}"
    );
}

// ── sequence consequent (slice S14, hand-IEEE) ───────────────────────────────
// The consequent may be a SEQUENCE (`a |-> b ##1 c`), not only a boolean. When
// the antecedent matches, the consequent sequence must match starting at that
// clock (|->) or the next (|=>); the property fails at the first consequent term
// that does not hold. Desugar (obligation chain, pure IR-0): due_0 = ante match;
// for each term, viol = due && !term, due_next = delay_hop(due && term);
// violation = OR of the viols. A boolean consequent keeps the byte-identical
// path. Bounded single-alternative boolean consequents only (ranges/goto/
// unbounded -> loud). AST flip (consequent: Expr -> Sequence); sim-ir unchanged.

#[test]
fn sva_consequent_sequence_holds() {
    // a |-> b ##1 c : a&&b at t15, c at t25 -> consequent matches -> no fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b ##1 c);\n\
         initial begin\n\
           #10 a=1; b=1; c=0;\n\
           #10 a=0; b=0; c=1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "consequent sequence holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\n{err}\n{out}"
    );
}

// ── multi-clock: deferred, but loud (slice S15) ──────────────────────────────
// Multi-clock concurrent assertions (a property sampling different signals on
// different clocks) are deferred. The single-`always` checker model does not
// extend, and iverilog gives no oracle. This slice keeps multi-clock LOUD and
// closes a silent-accept hole: an OR-of-clocks property clock
// `@(posedge c1 or posedge c2)` was previously built into one (semantically
// wrong) `always @(c1 or c2)` checker; it is now a loud reject. A second `@`
// inside a property body gets a dedicated diagnostic. The future two-process
// design is documented in docs/ROADMAP.md.

#[test]
fn sva_multiclock_or_clock_is_loud() {
    // `@(posedge c1 or posedge c2)` as a property clock is an OR-of-clocks event
    // — not a legal single SVA clock. Must be a loud elaborate reject (a single
    // clocking-event diagnostic), NOT a built checker (was silently accepted).
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1 or posedge c2) a |=> b);\n\
         initial #30 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "OR-clock property must be loud. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}")
            .to_lowercase()
            .contains("single clocking event"),
        "expected a single-clocking-event diagnostic:\n{err}\n{out}"
    );
}
