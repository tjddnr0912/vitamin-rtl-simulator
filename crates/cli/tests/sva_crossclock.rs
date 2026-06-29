//! Slice N2a-1 — multi-clock SVA SEQUENCE antecedent: `@(c1) a ##1 @(c2) b |-> c`.
//! The `##1 @(c2)` boundary re-clocks MID-ANTECEDENT (unlike A3, where the
//! implication crosses clocks). `a` is sampled on c1 and arms a 1-bit handoff; the
//! FIRST c2 edge after consumes it and samples `b`; the antecedent having completed
//! on that c2, the OVERLAP `|->` checks `c` on the SAME c2 edge. Two synthesized
//! processes (arm @c1, consume+check @c2) joined by the handoff — pure IR-0 (sim-ir
//! frozen, format_version 8). The AST gains `Sequence::Clocked` → `.vu` re-pins once.
//!
//! iverilog 13.0 rejects all concurrent assertions (NULL oracle) → hand-IEEE; every
//! expectation is derived from §16.13/§16.16 + value-pinned. The clocks here are
//! c1 posedges @5,15,25,… and c2 posedges @8,18,28,… (c2 always 3 after c1).
//!
//! `|=>` and >2 clock domains are now handled by N2a-2 (see sva_crossclock_n2a2.rs).
//! STILL LOUD here / unsupported: a same-clock multi-term segment, `##n` (n≠1) across
//! clocks, an explicit consequent clock, `disable iff` / a custom action.
//!
//! HAND-IEEE PINS (review N2a-1): (1) a redundant SAME-clock re-clock
//! `@(clk) a ##1 @(clk) b` is folded to the single-clock pipeline (a §16.13 no-op) so
//! it counts identically to `a ##1 b`. (2) the 1-bit handoff is VERDICT-SAFE but merges
//! several c1 attempts that fall DUE on the same c2 edge into one violation report (a
//! count under-fidelity — a real failure is never missed, a pass never spuriously
//! failed; only duplicate failure reports are merged).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaxc_{}_{n}", std::process::id()));
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

/// Two clocks: c1 posedges @5,15,25,…; c2 posedges @8,18,28,… (c2 = c1 + 3).
const CLKS: &str = "reg c1=0, c2=0;\n\
     always #5 c1=~c1;\n\
     initial begin #3 forever #5 c2=~c2; end\n";

#[test]
fn crossclock_violation_fires() {
    // a=b=1, c=0 → every c1 arms, every first-c2-after completes (b=1) and checks
    // c=0 → fires (one violation per c2 consume).
    let (out, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, b=1, c=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "cross-clock seq with c=0 must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion") && !err.contains("unsupported"),
        "must be an assertion violation, not a loud reject:\n{err}\n{out}"
    );
}

#[test]
fn crossclock_holds() {
    // a=b=c=1 → antecedent completes, c held on the c2 edge → clean.
    let (out, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, b=1, c=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "c held on the c2 edge must hold → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

#[test]
fn consequent_sampled_on_c2_not_c1() {
    // SKEW PROOF (consequent clock). a=b=1, c=1 everywhere EXCEPT c=0 in [17,19] — that
    // covers the c2 edge @18 but NOT the c1 edges @15,25 (c=1 there). The check fires
    // ONLY if `c` is sampled on c2 (@18, c=0). If it were sampled on c1 it would see
    // c=1 and never fire. Exactly one violation @18.
    let (out, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, b=1, c=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial begin #17 c=0; #2 c=1; #40 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "the consequent must be sampled on c2 (@18, c=0) → fire. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        format!("{err}{out}")
            .matches("Assertion property violation")
            .count(),
        1,
        "exactly ONE violation (only the c2@18 check sees c=0):\n{err}\n{out}"
    );
}

#[test]
fn second_segment_sampled_on_c2_b_zero_is_vacuous() {
    // SKEW PROOF (sequence clock). b=0, c=0. `a` arms every c1, but at the first c2
    // after, b=0 → the antecedent never completes → NO obligation → no fire despite
    // c=0. Proves `b` is sampled on c2 and the `##1` window is exactly one c2 edge.
    let (_o, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, b=0, c=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "b=0 on c2 → antecedent never completes → clean. {err}"
    );
}

#[test]
fn first_segment_arms_on_c1_a_zero_is_vacuous() {
    // a=0 → no c1 arm → no handoff → nothing to consume on c2 → clean regardless of c.
    let (_o, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=0, b=1, c=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "a=0 → no arm → clean. {err}");
}

#[test]
fn nonoverlap_crossclock_now_synthesizes() {
    // N2a-2: `|=>` across the sequence boundary (c on the NEXT c2 — an extra handoff
    // stage) now SYNTHESIZES (was loud in N2a-1). With c=0 it must FIRE (exit 1 with a
    // genuine assertion violation), NOT be loud-rejected. (Review N2a-2 MEDIUM: this
    // test formerly asserted loud-reject and passed vacuously once `|=>` synthesized.)
    let (out, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, b=1, c=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |=> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "a `|=>` cross-clock with c=0 must fire. {err}"
    );
    assert!(
        !err.contains("unsupported"),
        "must synthesize, not loud-reject:\n{err}"
    );
    assert!(
        format!("{err}{out}").contains("Assertion property violation"),
        "must be a genuine violation, not an elaboration error:\n{err}\n{out}"
    );
}

#[test]
fn multiterm_first_segment_is_loud() {
    // `a ##1 d ##1 @(c2) b` — a same-clock multi-term first segment is deferred (the
    // multi-term lane, beyond N2a-2) → loud.
    let (_o, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, d=1, b=1, c=0;\n\
         property p; @(posedge c1) a ##1 d ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "a multi-term first segment must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn hashhash_n_crossclock_is_loud() {
    // Only `##1` is a well-defined cross-clock connector (§16.13); `##2 @(c2)` is loud.
    let (_o, err, code) = run(&format!(
        "module top;\n{CLKS}reg a=1, b=1, c=0;\n\
         property p; @(posedge c1) a ##2 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "a `##2` cross-clock connector must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn same_clock_reclock_folds_to_single_clock() {
    // REVIEW FIX (N2a-1): a redundant same-clock re-clock `@(clk) a ##1 @(clk) b` is a
    // §16.13 no-op ≡ `@(clk) a ##1 b`. It must fold to the single-clock pipeline and
    // count IDENTICALLY (was ~2x under-firing via the lossy handoff). a=b=1,c=0 over
    // posedges @5,15,25,35 (#40) → antecedent completes @15,25,35 → 3 violations,
    // matching the plain `a ##1 b` form.
    let reclocked = run("module top;\n\
         reg clk=0, a=1, b=1, c=0; always #5 clk=~clk;\n\
         property p; @(posedge clk) a ##1 @(posedge clk) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n");
    let single = run("module top;\n\
         reg clk=0, a=1, b=1, c=0; always #5 clk=~clk;\n\
         property p; @(posedge clk) a ##1 b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n");
    let count = |s: &(String, String, Option<i32>)| {
        format!("{}{}", s.0, s.1)
            .matches("Assertion property violation")
            .count()
    };
    assert_eq!(
        reclocked.2,
        Some(1),
        "re-clocked form must fire. {}",
        reclocked.1
    );
    assert_eq!(
        count(&reclocked),
        count(&single),
        "same-clock re-clock must count identically to single-clock `a ##1 b` \
         (folded, not the lossy handoff): reclocked={} single={}",
        count(&reclocked),
        count(&single)
    );
    assert_eq!(count(&single), 3, "expected 3 fires over @15,25,35");
}

#[test]
fn multiple_attempts_same_due_edge_is_verdict_safe() {
    // HAND-IEEE pin (N2a-1): multiple c1 arms that fall DUE on the same (slower) c2
    // edge collapse onto the 1-bit handoff → reported ONCE, not per-attempt. This is
    // VERDICT-SAFE: the violation is still DETECTED (exit 1). c1 posedges @5,15,25,…;
    // c2 posedges @18,38 — arms @5,15 both due @18, b=1, c=0 → one (merged) fire.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=1, b=1, c=0;\n\
         always #5 c1=~c1;\n\
         initial begin #18 forever #10 c2=~c2; end\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #30 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "merged multi-attempt violation must still be DETECTED (verdict-safe). stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected a violation:\n{err}\n{out}"
    );
}

#[test]
fn multiple_attempts_same_due_edge_holds_clean() {
    // Verdict-safety complement: the SAME multi-attempt-merge setup with c=1 must stay
    // CLEAN — the merge never spuriously fails a holding property.
    let (_o, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=1, b=1, c=1;\n\
         always #5 c1=~c1;\n\
         initial begin #18 forever #10 c2=~c2; end\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #30 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "merged attempts with c held must stay clean. {err}"
    );
}

#[test]
fn crossclock_runs_deterministically() {
    let src = format!(
        "module top;\n{CLKS}reg a=1, b=1, c=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    );
    let (o1, e1, c1) = run(&src);
    let (o2, e2, c2) = run(&src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
