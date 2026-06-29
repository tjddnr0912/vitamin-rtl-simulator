//! SVA-QUAD test hardening (2026-06-23, ROADMAP §5.2): a characterization net for
//! the `##[m:n]` window matcher, pinning the EXACT match semantics the planned
//! quad→linear refactor (collapse the per-delay fan-out pipelines into one windowed
//! accumulator) must preserve. iverilog 13 supports NO concurrent assertions, so
//! every expectation here is HAND-IEEE (no differential oracle) — these were
//! independently cross-checked before being frozen.
//!
//! Existing `sva_property.rs` covers `##[1:2]` endpoints, `[*m:n]`, `##[m:$]`,
//! throughout/within, goto/nonconsec. This file targets the window cases a naive
//! collapse is most likely to get wrong:
//!   - m=0 overlap (`##[0:n]`): the delay-0 alternative matches at the SAME clock.
//!   - interior delays of a 3-wide window (`##[1:3]` matching strictly inside).
//!   - the upper bound (delay n+1 must NOT match).
//!   - `throughout` AND-ed across whichever window alternative matches.
//!   - a nested sequence as the left operand of a range (Cartesian expansion).
//!   - MULTIPLE completions inside one window (each imposes its own obligation).
//!
//! Clock: `always #5 clk=~clk` → posedges at t=5,15,25,35,45,55,…; a value set at
//! `#10k` is sampled at the next posedge.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaqh_{}_{n}", std::process::id()));
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

// ── m=0 overlap: the delay-0 alternative of `##[0:n]` matches at the SAME clock ──

#[test]
fn window_m0_delay0_overlap_fires() {
    // a ##[0:2] b |-> c: at t15 a=1 AND b=1 → the delay-0 alternative completes at
    // t15; |-> obliges c at t15; c=0 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[0:2] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=1; c=0;\n\
           #10 a=0; b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "##[0:2] delay-0 overlap (a&&b at one clock)",
    );
}

#[test]
fn window_m0_delay0_overlap_holds() {
    // Same delay-0 match, but c=1 at the match clock → obligation met → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[0:2] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=1; c=1;\n\
           #10 a=0; b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "##[0:2] delay-0 overlap with c high");
}

// ── interior + boundary of a 3-wide window `##[1:3]` ──

#[test]
fn window_interior_delay2_of_1_3_fires() {
    // a ##[1:3] b |-> c: a@t15, b first high at t35 (delay 2 = strictly interior of
    // [1:3]); c=0 → the interior alternative completes at t35 → fire. Pins that the
    // collapse keeps the MIDDLE delay, not only the endpoints.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:3] b |-> c);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0;\n\
           #10 b=1;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "##[1:3] interior delay-2");
}

#[test]
fn window_upper_bound_excludes_delay4() {
    // a ##[1:3] b |-> c: a@t15, b first high at t55 (delay 4 > 3) → NO alternative
    // matches → no obligation → clean. Pins the window's upper edge.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:3] b |-> c);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=0;\n\
           #30 b=1;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "##[1:3] delay-4 outside the window");
}

// ── throughout AND-ed across whichever window alternative matches ──

#[test]
fn window_throughout_over_range_fires() {
    // g throughout a ##[1:2] c |-> d: a@t15, c@t35 (delay-2 alternative); g held at
    // t15,t25,t35 → throughout satisfied → match completes t35, d=0 → fire. (Paren-
    // free: a parenthesized sub-sequence is honest-loud, pinned separately below.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, c=0, d=0, g=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) g throughout a ##[1:2] c |-> d);\n\
         initial begin\n\
           g=1;\n\
           #10 a=1;\n\
           #10 a=0; c=0;\n\
           #10 c=1;\n\
           #10 c=0; g=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "throughout held across the delay-2 window",
    );
}

#[test]
fn window_throughout_over_range_guard_drop_kills() {
    // Same property, but g DROPS at t25 (mid-window) → the throughout fails on the
    // delay-2 alternative → no match → no obligation → clean. Pins that the guard is
    // AND-ed at EVERY stage of the collapsed window, not just the endpoints.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, c=0, d=0, g=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) g throughout a ##[1:2] c |-> d);\n\
         initial begin\n\
           g=1;\n\
           #10 a=1;\n\
           #10 a=0; g=0;\n\
           #10 c=1;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "throughout guard dropped mid-window");
}

// ── a range in the MIDDLE of a flat sequence (not at the start) ──

#[test]
fn window_range_mid_sequence_fires() {
    // a ##1 b ##[1:2] c |-> d: a@t15, b@t25 (##1), then c at delay [1:2] from t25 →
    // c@t45 (delay 2) → match completes t45, d=0 → fire. Pins a window whose start is
    // itself the tail of a prior fixed-delay hop (the flat form of a nested seq).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##[1:2] c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1;\n\
           #10 b=0; c=0;\n\
           #10 c=1;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b ##[1:2] c, delay-2 alternative");
}

// ── slice A.2: a parenthesized sub-sequence now PARSES and elaborates via the
//    existing single-clock pipeline machinery. `##` concat is left-associative, so
//    `(a ##1 b) ##[1:2] c` is semantically identical to the unparenthesized form —
//    pinned here so the grouping stays transparent (not silently mismatched). ──

#[test]
fn parenthesized_subsequence_matches_unparenthesized() {
    // Identical input vector; the parenthesized and unparenthesized antecedents must
    // produce byte-identical verdicts (parens are transparent for `##` concat).
    let stim = "initial begin #10 a=1; #10 a=0; b=1; #10 b=0; c=1; #10 c=0; #20 $finish; end\n";
    let (op, ep, cp) = run(&format!(
        "module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) (a ##1 b) ##[1:2] c |-> d);\n\
         {stim}\
         endmodule\n"
    ));
    let (ou, eu, cu) = run(&format!(
        "module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##[1:2] c |-> d);\n\
         {stim}\
         endmodule\n"
    ));
    assert!(
        !format!("{ep}{op}").contains("VITA-E2002"),
        "parenthesized sub-sequence must parse, not error:\n{ep}{op}"
    );
    assert_eq!(
        (op, ep, cp),
        (ou, eu, cu),
        "parens are transparent for `##` concat → identical verdict"
    );
}

#[test]
fn grouped_subsequence_repetition_matches_hand_expansion() {
    // Slice A.2 enabled `(seq)[*n]` (grouped consecutive repetition) as a parser
    // side-effect. Pin it correct-or-loud: `(a ##1 b)[*2]` ≡ `a ##1 b ##1 a ##1 b`
    // (the bounded-Consec expand applied to a SEQUENCE operand, not just a boolean)
    // → must be byte-identical, never silently mismatched.
    let stim = "initial begin\n\
           @(posedge clk) a=1; @(posedge clk) a=0; b=1;\n\
           @(posedge clk) b=0; a=1; @(posedge clk) a=0; b=1;\n\
           @(posedge clk) b=0; c=0; #100 $finish;\n\
         end\n";
    let (og, eg, cg) = run(&format!(
        "module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) (a ##1 b)[*2] |-> c);\n\
         {stim}\
         endmodule\n"
    ));
    let (oe, ee, ce) = run(&format!(
        "module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b ##1 a ##1 b |-> c);\n\
         {stim}\
         endmodule\n"
    ));
    assert!(
        !format!("{eg}{og}").contains("VITA-E2002"),
        "grouped repetition must parse, not error:\n{eg}{og}"
    );
    assert_eq!(
        (og, eg, cg),
        (oe, ee, ce),
        "(a ##1 b)[*2] must equal its hand-expansion a ##1 b ##1 a ##1 b"
    );
}

// ── multiple completions inside one window each impose an obligation ──

#[test]
fn window_multiple_completions_each_obligated() {
    // a ##[1:3] b |-> c: a@t15; b high at t25 (delay 1) AND t45 (delay 3). The
    // delay-1 completion obliges c@t25 (c=1, met); the delay-3 completion obliges
    // c@t45 (c=0, VIOLATED) → fire. Pins that the collapse keeps EVERY completion in
    // the window live, not just the first.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:3] b |-> c);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1; c=1;\n\
           #10 b=0; c=0;\n\
           #10 b=1;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "delay-3 completion obligation after a satisfied delay-1",
    );
}

// ── cases surfaced by the adversarial fan-out-vs-collapse divergence hunt
//    (the no-oracle substitute). All hand-IEEE verdicts were triple-verified. ──

#[test]
fn window_m0_lead_range_then_trailing_hop_fires() {
    // a ##[0:1] b ##1 c |-> d: the delay-0 alternative (a&&b at t15) then ##1 c at
    // t25; d=0 at t25 → fire. Pins an m=0 leading range whose match must propagate
    // into a FURTHER fixed hop (the collapse's "live range" must survive the hop).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[0:1] b ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=1;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "##[0:1] delay-0 lead then ##1 hop");
}

#[test]
fn window_m0_throughout_kills_delay0_thread() {
    // g throughout a ##[0:2] c |-> d: the only candidate is the delay-0 alternative
    // (a&&c at t15), but g=0 at t15 → throughout kills it → no match → clean. Pins
    // that the guard is AND-ed at the m=0 (overlap) cycle, not just the lookback
    // stages — the divergence axis a naive collapse most easily drops.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, c=0, d=0, g=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) g throughout a ##[0:2] c |-> d);\n\
         initial begin\n\
           #10 a=1; c=1; g=0; d=0;\n\
           #10 a=0; c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "throughout kills the m=0 (delay-0) thread",
    );
}

#[test]
fn window_unbounded_zero_lower_same_clock_fires() {
    // a ##[0:$] b |-> c: a&&b at the SAME clock (t15) is the d=0 completion of the
    // unbounded range; c=0 → fire. The unbounded `AtLeast(0)` armed-latch lowering
    // previously read one clock late and DROPPED this same-clock completion (a
    // silent-wrong vs IEEE surfaced by the SVA-QUAD hardening divergence hunt);
    // fixed by OR-ing the this-clock activation into the m=0 post-hop signal.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[0:$] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=1; c=0;\n\
           #10 a=0; b=0; c=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "##[0:$] same-clock (d=0) completion");
}

#[test]
fn window_unbounded_zero_lower_later_clock_still_fires() {
    // Regression guard for the m=0 fix: a ##[0:$] b with b ONE clock after a (d=1)
    // must still fire — the latch path for d>=1 is unchanged by the d=0 OR.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[0:$] b |-> c);\n\
         initial begin\n\
           #10 a=1; b=0; c=1;\n\
           #10 a=0; b=1; c=0;\n\
           #10 b=0; c=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "##[0:$] later-clock (d=1) completion");
}
