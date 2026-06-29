//! Slice #6 — DEEPER homogeneous non-overlap property-reference chains.
//!
//! A 2-deep nested skew (`a |=> (b |=> c)` ≡ `(a ##1 b) |=> c`) was already handled
//! (slices N2b / SVA-R2). This slice makes the nested non-overlap chain RECURSE,
//! accumulating EXACTLY ONE `##1` per `|=>` (IEEE 1800-2017 §16.12 textual
//! substitution):
//!
//!   `a |=> b |=> c |=> d` ≡ `(a ##1 b ##1 c) |=> d`   — d due at a-clock + 3
//!   `a |=> b |=> c |=> d |=> e` ≡ `(a ##1 b ##1 c ##1 d) |=> e`  — +4
//!   `a |-> b |=> c |=> d` ≡ `((a && b) ##1 c) |=> d`  — a&&b fuse (skew 0), then +2
//!
//! Each link's antecedent prepends one `##1` into the top-level `|=>` antecedent
//! SEQUENCE; the chain terminates at a single top-level `|=>` + the existing pend-reg.
//!
//! iverilog 13.0 REJECTS all concurrent assertions (NULL oracle) → every expectation
//! below is HAND CYCLE-TRACED against §16.12 and pinned with an OFF-BY-ONE
//! DISCRIMINATOR: the consequent holds at every clock EXCEPT the exact fire clock, so
//! a single-cycle shift in either direction would flip fire↔hold. (Clock: `always #5
//! clk=~clk` ⇒ posedges at t=5,15,25,35,45,55,…)
//!
//! KEPT LOUD (correct-or-loud; reuse the existing rejects): a cross-clock inner link,
//! a self-/mutually-recursive property (`p: a|=>p` — must reject via the cycle guard,
//! NOT hang), a mixed `|=>…|->` chain deeper than the two handled outer shapes, an
//! inner `disable iff` / formals, a sequence inner side.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svachain_{}_{n}", std::process::id()));
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

fn fire_count(out: &str, err: &str) -> usize {
    format!("{err}{out}")
        .to_lowercase()
        .matches("assertion")
        .count()
}

// ════════════════════════ 3-deep: a|=>b|=>c|=>d  (+3) ════════════════════════
// HAND TRACE (≡ (a ##1 b ##1 c) |=> d): a@k · b@(k+1) · c@(k+2) → antecedent match
// completes at k+2 · top `|=>` ⇒ d-obligation due at k+3.

#[test]
fn three_deep_steady_holds() {
    // a=b=c=d=1 every clock → antecedent matches every clock, d=1 three clocks
    // later → clean. (a@5,b@15,c@25,d-due@35 is the first full obligation; #46 finish
    // covers posedges @5,15,25,35,45.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "3-deep with d held +3 must hold → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E") && !err.contains("unsupported"),
        "the 3-deep chain must be SYNTHESIZED, not loud-rejected:\n{err}\n{out}"
    );
}

#[test]
fn three_deep_constant_violation_fires() {
    // a=b=c=1, d=0 always → antecedent matches every clock, d=0 three clocks later →
    // fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=0; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "3-deep with d=0 must FIRE (assertion violation), not loud reject. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        fire_count(&out, &err) >= 1 && !err.contains("unsupported"),
        "must be an assertion violation, not a loud reject:\n{err}\n{out}"
    );
}

#[test]
fn three_deep_fires_exactly_at_plus_three_not_plus_two_or_four() {
    // THE OFF-BY-ONE DISCRIMINATOR. `a` pulses high ONLY at the first posedge (t=5);
    // b,c held 1; d held 1 EVERYWHERE except d=0 across posedge t=35.
    //   single match: a@5 ##1 b@15 ##1 c@25 → completes @25 (k2) → d due @35 (k3).
    //   d=0 only @35 → fires EXACTLY once. A wrong +2 reading checks d@25 (=1, no fire);
    //   a wrong +4 reading checks d@45 (=1, no fire). So a single fire PROVES +3.
    // Timeline: #6 a=0 (t=6) · #28 d=0 (t=34) · #2 d=1 (t=36) · #10 finish (t=46).
    // d=0 window [34,36) covers posedge @35 only.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #6 a=0; #28 d=0; #2 d=1; #10 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the d-obligation must be checked THREE clocks after a (d=0 @t35) → fire. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fire_count(&out, &err),
        1,
        "exactly ONE fire (the single a@5 attempt's +3 obligation @35):\n{err}\n{out}"
    );
}

#[test]
fn three_deep_d_held_at_plus_three_holds() {
    // The CONVERSE of the discriminator: same single a-pulse, but d held 1 at +3 (=t35)
    // — instead d=0 only at t=25 (the WRONG +2 clock) and t=45 (the WRONG +4 clock).
    // The genuine +3 obligation sees d=1 → NO fire. Proves the assertion does NOT read d
    // at +2 or +4 (either would fire here). d=0 windows [24,26) @25 and [44,46) @45.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #6 a=0; #18 d=0; #2 d=1; #18 d=0; #2 d=1; #10 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "d held at the genuine +3 clock (only the wrong +2/+4 clocks are low) → clean. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn three_deep_middle_link_gates() {
    // c=0 always → the antecedent `a ##1 b ##1 c` never completes → no obligation →
    // clean regardless of d. Proves the MIDDLE link c genuinely participates (the
    // desugar is NOT `a ##1 b |=> d`).
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0, d=0; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "middle link c=0 → vacuous → clean. {err}");
}

// ══════════════ overlap-outer + 2 nonoverlap: a|->b|=>c|=>d  (+2) ══════════════
// HAND TRACE (≡ ((a && b) ##1 c) |=> d): overlap `|->` FUSES a&&b at the SAME clock
// (skew 0) · c@(k+1) → match completes k+1 · top `|=>` ⇒ d due at k+2.

#[test]
fn overlap_outer_two_nonoverlap_fires_at_plus_two() {
    // OFF-BY-ONE DISCRIMINATOR for the overlap-outer fold. a pulses high only @5; b,c
    // held; d held 1 except d=0 across posedge t=25.
    //   (a&&b)@5 ##1 c@15 → completes @15 (k1) → d due @25 (k2). d=0 only @25 → fire.
    //   a wrong +1 reads d@15 (=1); a wrong +3 reads d@35 (=1) → either would NOT fire.
    // Timeline: #6 a=0 · #18 d=0 (t=24) · #2 d=1 (t=26) · #20 finish. window [24,26) @25.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |-> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #6 a=0; #18 d=0; #2 d=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "overlap outer + 2 nonoverlap must fire at +2 (d=0 @t25). stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fire_count(&out, &err),
        1,
        "exactly ONE fire at the +2 obligation @25:\n{err}\n{out}"
    );
}

#[test]
fn overlap_outer_d_held_at_plus_two_holds() {
    // CONVERSE: d held 1 at the genuine +2 clock (t=25); d=0 only at the WRONG +1 (t=15)
    // and +3 (t=35). The genuine obligation sees d=1 → NO fire. Proves +2 exactly.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |-> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #6 a=0; #8 d=0; #2 d=1; #18 d=0; #2 d=1; #10 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "d held at the genuine +2 clock (only wrong +1/+3 low) → clean. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn overlap_outer_two_nonoverlap_steady_holds() {
    // a=b=c=d=1 → (a&&b) obligation every clock, d=1 two clocks later → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |-> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "overlap outer chain held → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

// ════════════════════════ 4-deep: a|=>b|=>c|=>d|=>e  (+4) ════════════════════════
// HAND TRACE (≡ (a ##1 b ##1 c ##1 d) |=> e): a@k·b@(k+1)·c@(k+2)·d@(k+3) → completes
// k+3 · top `|=>` ⇒ e due at k+4.

#[test]
fn four_deep_fires_exactly_at_plus_four() {
    // OFF-BY-ONE DISCRIMINATOR. a pulses only @5; b,c,d held; e held 1 except e=0
    // across posedge t=45 (= a@5 + 4 clocks).
    //   a@5 ##1 b@15 ##1 c@25 ##1 d@35 → completes @35 (k3) → e due @45 (k4).
    //   e=0 only @45 → fire. A wrong +3 reads e@35 (=1); +5 reads e@55 (=1).
    // Timeline: #6 a=0 · #38 e=0 (t=44) · #2 e=1 (t=46) · #10 finish (t=56). [44,46) @45.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1, e=1; always #5 clk=~clk;\n\
         property s; @(posedge clk) d |=> e; endproperty\n\
         property r; @(posedge clk) c |=> s; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #6 a=0; #38 e=0; #2 e=1; #10 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "4-deep e-obligation must fire FOUR clocks after a (e=0 @t45). stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fire_count(&out, &err),
        1,
        "exactly ONE fire at the +4 obligation @45:\n{err}\n{out}"
    );
}

#[test]
fn four_deep_steady_holds() {
    // a..d=1, e=1 always → e held +4 → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=1, e=1; always #5 clk=~clk;\n\
         property s; @(posedge clk) d |=> e; endproperty\n\
         property r; @(posedge clk) c |=> s; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #56 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "4-deep held +4 → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(!err.contains("VITA-E"), "must synthesize:\n{err}");
}

// ════════════════════ 2-deep REGRESSION (must stay green, +2) ════════════════════

#[test]
fn two_deep_still_fires_at_plus_two() {
    // Regression guard: the pre-existing 2-deep chain `a|=>b|=>c` ≡ `(a##1 b)|=>c` must
    // stay byte-behaviorally identical (fires at +2). a pulses only @5; b held; c held
    // 1 except c=0 across posedge t=25. a@5 ##1 b@15 completes @15 → c due @25 (+2).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #6 a=0; #18 c=0; #2 c=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "2-deep regression must fire at +2 (c=0 @t25). stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fire_count(&out, &err),
        1,
        "exactly ONE fire at +2:\n{err}\n{out}"
    );
}

// ═══════════════════════════════ KEPT LOUD (RED) ═══════════════════════════════

#[test]
fn flat_self_recursive_property_is_loud_not_hang() {
    // `p: @(clk) a |=> p` — a FLAT (non-tree) self-reference: p's consequent is the
    // property p itself. The chain peel must hit the `sva_inline_stack` cycle guard and
    // LOUD-REJECT (recursive property illegal), NOT hang or silently mis-synthesize.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1; always #5 clk=~clk;\n\
         property p; @(posedge clk) a |=> p; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a recursive property must be loud-rejected, not silently accepted:\n{err}\n{out}"
    );
    assert!(
        err.contains("VITA-E") && err.to_lowercase().contains("recursive"),
        "expected the loud `recursive property` diagnostic:\n{err}\n{out}"
    );
}

#[test]
fn mutually_recursive_chain_is_loud_not_hang() {
    // `p: a|=>q`, `q: b|=>p` — a 2-cycle MUTUAL recursion. The chain peel of p recurses
    // into q, then back to p (on the inline stack) → cycle guard fires → loud, not hang.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> p; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "mutual recursion must be loud-rejected, not hang:\n{err}\n{out}"
    );
    assert!(
        err.contains("VITA-E") && err.to_lowercase().contains("recursive"),
        "expected the loud `recursive property` diagnostic:\n{err}\n{out}"
    );
}

#[test]
fn mixed_nonoverlap_then_overlap_chain_is_loud() {
    // `a |=> b |=> c |-> d` — a NON-overlap outer/middle but an OVERLAP `|->` deepest
    // link. Mixing overlap/non-overlap beyond the two handled outer shapes is out of
    // subset → loud (the deepest link is `|->`, not the homogeneous `|=>` chain).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1, d=0; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |-> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a mixed |=>…|-> chain must be loud-rejected:\n{err}\n{out}"
    );
    assert!(err.contains("VITA-E"), "{err}\n{out}");
}

#[test]
fn cross_clock_inner_chain_is_loud() {
    // `r` is clocked on a DIFFERENT edge (clk2) than the outer p/q (clk). A cross-clock
    // inner link has no portable single-pipeline meaning → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, clk2=0, a=1, b=1, c=1, d=0;\n\
         always #5 clk=~clk; always #7 clk2=~clk2;\n\
         property r; @(posedge clk2) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "a cross-clock inner link must be loud-rejected:\n{err}\n{out}"
    );
    assert!(err.contains("VITA-E"), "{err}\n{out}");
}

#[test]
fn inner_disable_iff_in_chain_is_loud() {
    // A deeper link carrying its own `disable iff` is beyond this slice → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, rst=0, a=1, b=1, c=1, d=0; always #5 clk=~clk;\n\
         property r; @(posedge clk) disable iff (rst) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "an inner link with disable iff must be loud-rejected:\n{err}\n{out}"
    );
    assert!(err.contains("VITA-E"), "{err}\n{out}");
}

#[test]
fn inner_property_operator_consequent_is_loud() {
    // CRITICAL (found by the #6 adversarial review): a named property whose body is
    // a property-LEVEL operator (`always`/`not`/`s_eventually`/`nexttime`) is stored
    // with inert placeholder flat fields; the overlap flattener used to fold the
    // placeholder to `1'b1` and SILENTLY DROP the obligation. The same form is loud
    // standalone (`c |=> always d`), so the nested form must be loud too.
    for inner in ["always c", "not c", "s_eventually c", "nexttime c"] {
        let (out, err, code) = run(&format!(
            "module top;\n\
             logic clk=0,a,b,c; always #5 clk=~clk;\n\
             property q1; @(posedge clk) b |=> {inner}; endproperty\n\
             initial assert property (@(posedge clk) a |=> q1);\n\
             initial #80 $finish;\n\
             endmodule\n"
        ));
        assert_ne!(
            code,
            Some(0),
            "nested inner `{inner}` must be loud, not silently dropped:\n{err}\n{out}"
        );
        assert!(err.contains("VITA-E"), "inner `{inner}`:\n{err}\n{out}");
    }
}

// ═══════════════════════════════ determinism ═══════════════════════════════

#[test]
fn three_deep_runs_deterministically() {
    let src = "module top;\n\
         reg clk=0, a=1, b=1, c=1, d=0; always #5 clk=~clk;\n\
         property r; @(posedge clk) c |=> d; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
