//! SVA-QUAD differential gate (2026-06-26, ROADMAP §5.2): the safety mechanism for
//! the `##[m:n]` window collapse. The EXISTING fan-out lowering (one `Fixed(d)`
//! alternative per delay `d` in `[m:n]`, each re-materializing the full prefix
//! pipeline = O(N^2) regs) is the BYTE-IDENTICAL REFERENCE ORACLE. The collapse
//! (one shared O(n-m) sliding-OR window, `SeqHop::Range`) is enabled by the
//! `VITA_SVA_COLLAPSE` env. This test runs the SAME `.sv` twice — once WITHOUT the
//! env (the fan-out ground truth) and once WITH it (the collapse candidate) — and
//! asserts the ASSERTION VERDICT STREAM is byte-identical: same exit code, same
//! ordered set of violation diagnostics, same full stdout. Fan-out is ground truth
//! BY CONSTRUCTION; ANY divergence means the collapse is wrong (the cardinal sin),
//! so a divergence FAILS this test and the collapse must stay off (deferred).
//!
//! iverilog 13 supports NO concurrent assertions, so there is no external oracle —
//! the fan-out IS the oracle, which is exactly why this differential is the gate.
//!
//! The VCD's internal `__sva_*` net SET differs between the two lowerings (fewer
//! regs under collapse); that is checker plumbing, NOT observable design behavior,
//! and never appears in a verdict/diagnostic. We compare the stdout verdict stream
//! + exit, which the collapse MUST preserve exactly.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `vita src.sv` with the collapse env either UNSET (fan-out) or SET (collapse).
fn run_one(src: &str, collapse: bool) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaqd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(f.to_str().unwrap()).current_dir(&d);
    // Always start from a clean slate so an ambient VITA_SVA_COLLAPSE in the
    // tester's shell cannot poison the fan-out reference run.
    cmd.env_remove("VITA_SVA_COLLAPSE");
    if collapse {
        cmd.env("VITA_SVA_COLLAPSE", "1");
    }
    let out = cmd.output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// The VERDICT STREAM of a run: exit code + the ordered list of assertion-relevant
/// lines from stderr+stdout. Internal `__sva_*` reg names never appear in a
/// diagnostic (they are NBA targets, not reported), and we additionally drop any
/// line mentioning `__sva` defensively, so a future change that surfaced a reg name
/// could not mask a real verdict divergence.
fn verdict(out: &str, err: &str, code: Option<i32>) -> (Option<i32>, Vec<String>) {
    let mut lines: Vec<String> = Vec::new();
    for l in err.lines().chain(out.lines()) {
        if l.contains("__sva") {
            continue;
        }
        let low = l.to_lowercase();
        // Keep every line that carries a verdict signal: assertion fires, the
        // simulation-ended summary, and the errors/warnings tally.
        if low.contains("assertion")
            || low.contains("simulation ended")
            || low.contains("errors=")
            || low.contains("violation")
        {
            lines.push(l.trim_end().to_string());
        }
    }
    (code, lines)
}

/// Assert the fan-out (oracle) and collapse runs produce an IDENTICAL verdict
/// stream for `src`. This is the differential gate's core assertion.
fn assert_verdict_equiv(src: &str, ctx: &str) {
    let (o0, e0, c0) = run_one(src, false); // fan-out reference (ground truth)
    let (o1, e1, c1) = run_one(src, true); // collapse candidate
    let v0 = verdict(&o0, &e0, c0);
    let v1 = verdict(&o1, &e1, c1);
    assert_eq!(
        v0, v1,
        "SVA-QUAD DIVERGENCE [{ctx}]: collapse verdict != fan-out verdict.\n\
         === source ===\n{src}\n\
         === fan-out (oracle) ===\nexit={c0:?}\nstdout:\n{o0}\nstderr:\n{e0}\n\
         === collapse (candidate) ===\nexit={c1:?}\nstdout:\n{o1}\nstderr:\n{e1}"
    );
    // Also require the FULL stdout to match (the assertion fail text + the
    // simulation-ended line are design-observable; only the VCD net set may differ,
    // and the VCD is a separate file, never stdout).
    assert_eq!(
        o0, o1,
        "SVA-QUAD STDOUT DIVERGENCE [{ctx}]: collapse stdout != fan-out stdout.\n\
         fan-out:\n{o0}\ncollapse:\n{o1}"
    );
}

/// Build a `top` module: a free-running clock, an `assert property(@(posedge clk) P)`,
/// and an `initial` that drives `inits` at t0 then applies `steps` (one posedge per
/// `#10` gap, values sampled at the next posedge), then `$finish`.
fn design(prop: &str, decls: &str, inits: &str, steps: &[&str]) -> String {
    let mut body = String::new();
    body.push_str("module top;\n");
    body.push_str(&format!("  reg clk=0; {decls}\n"));
    body.push_str("  always #5 clk=~clk;\n");
    body.push_str(&format!(
        "  initial assert property(@(posedge clk) {prop});\n"
    ));
    body.push_str("  initial begin\n");
    body.push_str(&format!("    {inits}\n"));
    for s in steps {
        body.push_str(&format!("    #10 {s}\n"));
    }
    body.push_str("    #20 $finish;\n");
    body.push_str("  end\n");
    body.push_str("endmodule\n");
    body
}

// ─────────────────────────── directed shapes ───────────────────────────────────

#[test]
fn diff_m0_bounded_overlap_fire_and_hold() {
    // ##[0:2] — the delay-0 alternative matches in a's own clock (hazard #1). Both
    // a fire (c=0) and a hold (c=1) at the overlap clock.
    let p = "a ##[0:2] b |-> c";
    assert_verdict_equiv(
        &design(p, "reg a=0,b=0,c=0;", "#10 a=1; b=1; c=0;", &["a=0; b=0;"]),
        "m0 overlap fire",
    );
    assert_verdict_equiv(
        &design(p, "reg a=0,b=0,c=0;", "#10 a=1; b=1; c=1;", &["a=0; b=0;"]),
        "m0 overlap hold",
    );
}

#[test]
fn diff_m_ge1_bounded_interior_and_boundary() {
    // ##[1:3] interior (delay-2 fires) and beyond-upper (delay-4 holds).
    let p = "a ##[1:3] b |-> c";
    assert_verdict_equiv(
        &design(p, "reg a=0,b=0,c=0;", "#10 a=1;", &["a=0;", "b=1;", "b=0;"]),
        "m>=1 interior delay-2",
    );
    assert_verdict_equiv(
        &design(
            p,
            "reg a=0,b=0,c=0;",
            "#10 a=1;",
            &["a=0; b=0;", "", "b=1;", "b=0;"],
        ),
        "m>=1 delay-4 beyond upper",
    );
}

#[test]
fn diff_single_window_multiple_completions() {
    // Two completions in one window (delay-1 met, delay-3 violated) — every
    // completion must stay live (hazard: the window union, not the first match).
    assert_verdict_equiv(
        &design(
            "a ##[1:3] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1;",
            &["a=0; b=1; c=1;", "b=0; c=0;", "b=1;", "b=0;"],
        ),
        "multiple completions each obligated",
    );
}

#[test]
fn diff_m_ge2_window_and_multibit_operand() {
    // m>=2 lower bound (the first loop does >=2 shifts before seeding `win`) AND a
    // MULTI-BIT operand `a` (the `|`-reduction inside the AND must survive the
    // window). Pins both axes the m=0/m=1 cases don't reach.
    assert_verdict_equiv(
        &design(
            "a ##[2:5] b |-> c",
            "reg [1:0] a=0; reg b=0,c=0;",
            "#10 a=2'b10;",
            &["a=0;", "b=0;", "b=1;", "b=0; c=0;", "b=1;"],
        ),
        "m>=2 window with multi-bit operand",
    );
}

#[test]
fn diff_xz_operand_window() {
    // X/Z operands in the window (§16.13.5 X-strict). The OR-vs-delay commutation
    // (`reg(x|y)==reg(x)|reg(y)`) must hold in 4-state, so an X feeding the window
    // union / shift chain yields the SAME verdict under both lowerings.
    assert_verdict_equiv(
        &design(
            "a ##[0:3] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1'bx; b=1; c=0;",
            &["a=0; b=0;", "a=1; b=1'bx;"],
        ),
        "X/Z operands in a window",
    );
}

#[test]
fn diff_window_and_throughout_guard() {
    // throughout AND-ed across the window: held (fires) and dropped-mid (kills).
    let p = "g throughout a ##[1:2] c |-> d";
    assert_verdict_equiv(
        &design(
            p,
            "reg a=0,c=0,d=0,g=0;",
            "g=1;",
            &["a=1;", "a=0; c=0;", "c=1;", "c=0; g=0;"],
        ),
        "throughout held across window",
    );
    assert_verdict_equiv(
        &design(
            p,
            "reg a=0,c=0,d=0,g=0;",
            "g=1;",
            &["a=1;", "a=0; g=0;", "c=1;", "c=0;"],
        ),
        "throughout dropped mid-window (hazard #2)",
    );
    // m=0 throughout: the guard must AND at the OVERLAP cycle too (hazard #1+#2).
    assert_verdict_equiv(
        &design(
            "g throughout a ##[0:2] c |-> d",
            "reg a=0,c=0,d=0,g=0;",
            "#10 a=1; c=1; g=0; d=0;",
            &["a=0; c=0;"],
        ),
        "throughout kills m=0 delay-0 thread",
    );
}

#[test]
fn diff_window_as_trailing_hop() {
    // The range is the LAST hop before the consequent's first term (hazard #3):
    // the rhs term samples the window union. m=0 lead range then a FURTHER ##1 hop.
    assert_verdict_equiv(
        &design(
            "a ##[0:1] b ##1 c |-> d",
            "reg a=0,b=0,c=0,d=0;",
            "#10 a=1; b=1;",
            &["a=0; b=0; c=1; d=0;", "c=0;"],
        ),
        "m=0 lead range then ##1 trailing hop",
    );
    // A range in the MIDDLE of a flat sequence (start is the tail of a prior hop).
    assert_verdict_equiv(
        &design(
            "a ##1 b ##[1:2] c |-> d",
            "reg a=0,b=0,c=0,d=0;",
            "#10 a=1;",
            &["a=0; b=1;", "b=0; c=0;", "c=1;", "c=0;"],
        ),
        "range mid-sequence after a ##1",
    );
}

#[test]
fn diff_window_followed_by_window() {
    // Two ranges in series (`##[1:2] … ##[1:2]`): the collapse must compose two
    // sliding windows. Several stimulus vectors to exercise the cross-product of
    // completion clocks.
    let p = "a ##[1:2] b ##[1:2] c |-> d";
    for (i, steps) in [
        vec!["a=0; b=1;", "b=0; c=1; d=1;", "c=0; d=0;", ""],
        vec!["a=0;", "b=1;", "b=0; c=1; d=0;", "c=0;"],
        vec!["a=0; b=1; c=1; d=0;", "b=0; c=0;", "", ""],
        vec!["a=0;", "b=1; c=0;", "b=0; c=1; d=0;", "c=0;"],
    ]
    .into_iter()
    .enumerate()
    {
        assert_verdict_equiv(
            &design(p, "reg a=0,b=0,c=0,d=0;", "#10 a=1;", &steps),
            &format!("window-followed-by-window vec{i}"),
        );
    }
}

#[test]
fn diff_multi_term_cross_product() {
    // A nested sequence as the LEFT operand of a range — Cartesian expansion in the
    // fan-out (lhs alts × delays). The collapse keeps lhs alts but folds the delays.
    let p = "(a ##1 b) ##[0:2] c |-> d";
    for (i, steps) in [
        vec!["a=0; b=1;", "b=0; c=1; d=0;", "c=0;", ""],
        vec!["a=0; b=1; c=1; d=1;", "b=0; c=0;", "", ""],
        vec!["a=0; b=1;", "b=0;", "c=1; d=0;", "c=0;"],
    ]
    .into_iter()
    .enumerate()
    {
        assert_verdict_equiv(
            &design(p, "reg a=0,b=0,c=0,d=0;", "#10 a=1;", &steps),
            &format!("multi-term cross-product vec{i}"),
        );
    }
}

#[test]
fn diff_consequent_window_via_or_property() {
    // A window in a NON-overlap implication `|=>` (the antecedent match is +1-clock
    // delayed before seeding) — exercises the range under a different seed path.
    assert_verdict_equiv(
        &design(
            "a |=> b ##[0:2] c",
            "reg a=0,b=0,c=0;",
            "#10 a=1;",
            &["a=0; b=1; c=0;", "b=0; c=0;", "c=0;"],
        ),
        "|=> then window consequent (loud-or-equiv)",
    );
}

// ── the 13 sva_window_hardening.rs shapes, re-run through the differential ───────

#[test]
fn diff_hardening_corpus_shapes() {
    // Each entry mirrors a frozen hardening shape (prop, decls, inits, steps). The
    // hardening file pins the ABSOLUTE verdict (hand-IEEE); here we pin that the
    // collapse REPRODUCES the fan-out verdict for the same vectors.
    type Shape = (&'static str, &'static str, &'static str, Vec<&'static str>);
    let shapes: Vec<Shape> = vec![
        (
            "a ##[0:2] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1; b=1; c=0;",
            vec!["a=0; b=0;"],
        ),
        (
            "a ##[0:2] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1; b=1; c=1;",
            vec!["a=0; b=0;"],
        ),
        (
            "a ##[1:3] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1;",
            vec!["a=0;", "b=1;", "b=0;"],
        ),
        (
            "a ##[1:3] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1;",
            vec!["a=0; b=0;", "", "b=1;", "b=0;"],
        ),
        (
            "g throughout a ##[1:2] c |-> d",
            "reg a=0,c=0,d=0,g=0;",
            "g=1;",
            vec!["a=1;", "a=0; c=0;", "c=1;", "c=0; g=0;"],
        ),
        (
            "g throughout a ##[1:2] c |-> d",
            "reg a=0,c=0,d=0,g=0;",
            "g=1;",
            vec!["a=1;", "a=0; g=0;", "c=1;", "c=0;"],
        ),
        (
            "a ##1 b ##[1:2] c |-> d",
            "reg a=0,b=0,c=0,d=0;",
            "#10 a=1;",
            vec!["a=0; b=1;", "b=0; c=0;", "c=1;", "c=0;"],
        ),
        (
            "a ##[1:3] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1;",
            vec!["a=0; b=1; c=1;", "b=0; c=0;", "b=1;", "b=0;"],
        ),
        (
            "a ##[0:1] b ##1 c |-> d",
            "reg a=0,b=0,c=0,d=0;",
            "#10 a=1; b=1;",
            vec!["a=0; b=0; c=1; d=0;", "c=0;"],
        ),
        (
            "g throughout a ##[0:2] c |-> d",
            "reg a=0,c=0,d=0,g=0;",
            "#10 a=1; c=1; g=0; d=0;",
            vec!["a=0; c=0;"],
        ),
        // ##[0:$] and ##[m:$] stay on the AtLeast latch (NOT collapsed), so the
        // differential trivially holds — but including them guards that the
        // collapse selector never accidentally reroutes the unbounded path.
        (
            "a ##[0:$] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1; b=1; c=0;",
            vec!["a=0; b=0; c=1;"],
        ),
        (
            "a ##[0:$] b |-> c",
            "reg a=0,b=0,c=0;",
            "#10 a=1; b=0; c=1;",
            vec!["a=0; b=1; c=0;", "b=0; c=1;"],
        ),
        (
            "(a ##1 b) ##[1:2] c |-> d",
            "reg a=0,b=0,c=0,d=0;",
            "#10 a=1;",
            vec!["a=0; b=1;", "b=0; c=1; d=0;", "c=0;"],
        ),
    ];
    for (i, (p, decls, inits, steps)) in shapes.iter().enumerate() {
        assert_verdict_equiv(
            &design(p, decls, inits, steps),
            &format!("hardening shape {i}"),
        );
    }
}

// ─────────────────────────── randomized fuzz ────────────────────────────────────

/// A small deterministic LCG (NOT Math.random) so the fuzz vectors are fixed and
/// reproducible across runs/OSes. Seeded by the trace index.
fn lcg(state: &mut u64) -> u64 {
    // Numerical Recipes constants.
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Build a randomized stimulus: `nsteps` posedge steps, each assigning a/b/c/d/g a
/// pseudo-random 0/1. Returns the `steps` lines.
fn fuzz_steps(seed: u64, nsteps: usize, sigs: &[&str]) -> Vec<String> {
    let mut st = seed ^ 0x9e3779b97f4a7c15;
    let mut out = Vec::with_capacity(nsteps);
    for _ in 0..nsteps {
        let r = lcg(&mut st);
        let mut line = String::new();
        for (bit, s) in sigs.iter().enumerate() {
            let v = (r >> (bit * 7)) & 1;
            line.push_str(&format!("{s}={v}; "));
        }
        out.push(line);
    }
    out
}

#[test]
fn diff_fuzz_m0_window() {
    // ##[0:2] over many randomized a/b/c traces (the m=0 same-clock axis).
    let sigs = ["a", "b", "c"];
    for seed in 0..40u64 {
        let steps = fuzz_steps(seed, 6, &sigs);
        let steps_ref: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
        assert_verdict_equiv(
            &design("a ##[0:2] b |-> c", "reg a=0,b=0,c=0;", "", &steps_ref),
            &format!("fuzz m0 seed{seed}"),
        );
    }
}

#[test]
fn diff_fuzz_mge1_window() {
    // ##[1:3] over many randomized traces (interior + boundary delays).
    let sigs = ["a", "b", "c"];
    for seed in 100..140u64 {
        let steps = fuzz_steps(seed, 7, &sigs);
        let steps_ref: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
        assert_verdict_equiv(
            &design("a ##[1:3] b |-> c", "reg a=0,b=0,c=0;", "", &steps_ref),
            &format!("fuzz m>=1 seed{seed}"),
        );
    }
}

#[test]
fn diff_fuzz_throughout_window() {
    // g throughout a ##[0:3] c — the guard-death axis under random traces.
    let sigs = ["a", "c", "d", "g"];
    for seed in 200..240u64 {
        let steps = fuzz_steps(seed, 7, &sigs);
        let steps_ref: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
        assert_verdict_equiv(
            &design(
                "g throughout a ##[0:3] c |-> d",
                "reg a=0,c=0,d=0,g=0;",
                "",
                &steps_ref,
            ),
            &format!("fuzz throughout seed{seed}"),
        );
    }
}

#[test]
fn diff_fuzz_window_followed_by_window() {
    // Two ranges in series under random traces (the hardest compositional case).
    let sigs = ["a", "b", "c", "d"];
    for seed in 300..340u64 {
        let steps = fuzz_steps(seed, 8, &sigs);
        let steps_ref: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
        assert_verdict_equiv(
            &design(
                "a ##[1:2] b ##[1:2] c |-> d",
                "reg a=0,b=0,c=0,d=0;",
                "",
                &steps_ref,
            ),
            &format!("fuzz win-then-win seed{seed}"),
        );
    }
}

#[test]
fn diff_fuzz_multi_term_left_operand() {
    // (a ##1 b) ##[0:2] c — nested left operand (Cartesian in fan-out) under random.
    let sigs = ["a", "b", "c", "d"];
    for seed in 400..440u64 {
        let steps = fuzz_steps(seed, 8, &sigs);
        let steps_ref: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
        assert_verdict_equiv(
            &design(
                "(a ##1 b) ##[0:2] c |-> d",
                "reg a=0,b=0,c=0,d=0;",
                "",
                &steps_ref,
            ),
            &format!("fuzz multiterm seed{seed}"),
        );
    }
}

#[test]
fn diff_window_over_alt_cap_both_loud() {
    // CAP-BOUNDARY: a window whose size n-m+1 exceeds SVA_SEQ_ALT_CAP (256) must be
    // loud-rejected by BOTH impls — the fan-out path caps the post-expansion
    // alternative count, and the collapse path applies the SAME cap to the window
    // depth (so they agree at the boundary AND the sliding-OR reg allocation is
    // bounded, no OOM on `##[1:5_000_000]`). Without the cap on the collapse side,
    // collapse would silently run a window fan-out rejects (loud-vs-correct
    // divergence) — this is the case the original differential corpus missed.
    assert_verdict_equiv(
        &design(
            "a ##[1:300] b |-> c",
            "reg a=0,b=0,c=0;",
            "",
            &["a=1;", "a=0;"],
        ),
        "##[1:300] window exceeds the 256 alternative cap (both loud)",
    );
}
