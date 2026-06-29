//! Property-references-property with a NON-OVERLAP inner property (slice SVA-R2,
//! the outer-`|=>` skew that A4 left loud). A4 flattened only an OVERLAP inner
//! property `q: b |-> c` (single-tick `!b || c`). When the referenced property is
//! NON-OVERLAP — `q: @(clk) b |=> c` — the obligation spans a clock, so it cannot
//! collapse to a single-tick boolean. For the canonical `property p; @(clk) a |-> q;`
//! (outer OVERLAP, boolean outer antecedent), this rewrites the top-level assertion
//! to `(a && b) |=> c` and lets the existing top-level `|=>` pend-reg machinery
//! produce the 1-cycle skew. Pure IR-0 (sim-ir frozen, format_version 8).
//!
//! iverilog 13.0 rejects all of this (NULL oracle) → hand-IEEE. The 2-cycle skew
//! (outer `|=>` AND inner `|=>`) is now synthesized as `(a ##1 b) |=> c` (slice N2b,
//! see `sva_propref_2cycle.rs`); a DEEPER homogeneous `|=>` chain is now synthesized
//! too (slice #6, see `sva_propref_chain.rs`). STILL LOUD (deferred): a MIXED
//! `|=>…|->` chain, a different/multi clock, formals, `disable iff`, or recursion.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaprn_{}_{n}", std::process::id()));
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

// ── outer OVERLAP, inner NON-OVERLAP: `a |-> (b |=> c)` ≡ `(a && b) |=> c` ──────

#[test]
fn overlap_outer_nonoverlap_inner_holds() {
    // a=b=c=1 every clock → (a&&b) sets the obligation, c holds next clock → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "(a&&b)|=>c with c held next clock must hold → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("unsupported") && !err.contains("VITA-E"),
        "the inner |=> property must be synthesized, not loud:\n{err}"
    );
}

#[test]
fn nonoverlap_inner_checks_following_clock_not_same() {
    // KEY SKEW TEST: c is high AT the antecedent clock but low AFTER. If the inner
    // |=> were wrongly treated as overlap (`(a&&b)|->c`, same-clock c), it would
    // PASS. The |=> skew checks c on the FOLLOWING clock → it must FIRE.
    // posedge t=5: (a&&b)=1 sampled, c=1. c→0 at t=6. posedge t=15: pend=1, c=0 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial begin #6 c=0; #30 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the inner |=> must check the FOLLOWING clock (c low there) → fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn nonoverlap_inner_violation_fires() {
    // a=b=1, c=0 constant → (a&&b) sets obligation, c=0 next clock → fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "(a&&b)|=>c with c=0 next clock must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn nonoverlap_inner_antecedent_false_is_vacuous() {
    // b=0 → (a&&b)=0 → no obligation → holds regardless of c. Proves the inner
    // antecedent b actually gates (the desugar is NOT `a |=> c`).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "inner antecedent b=0 → vacuous → clean. stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn nonoverlap_outer_antecedent_false_is_vacuous() {
    // a=0 → (a&&b)=0 → no obligation → holds regardless of c. Proves the outer
    // antecedent a is folded in.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "outer antecedent a=0 → vacuous → clean. stderr:\n{err}\nout:\n{out}"
    );
}

// ── regression: outer NON-OVERLAP + inner OVERLAP already worked (A4) ───────────

#[test]
fn nonoverlap_outer_overlap_inner_still_works() {
    // `p: a |=> q` with `q: b |-> c` (OVERLAP inner) = `a |=> (!b||c)` — handled by
    // A4 (flatten q to !b||c) + the top-level |=> pend reg. Must stay working.
    // a=1,b=1,c=0 → at t=5 pend<=a=1; t=15 check (!b||c)=(0||0)=0 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |-> c; endproperty\n\
         property p_aq; @(posedge clk) a |=> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "outer |=> + inner overlap (A4 path) must still fire. stderr:\n{err}\nout:\n{out}"
    );
}

// ── LOUD (deferred) ────────────────────────────────────────────────────────────

#[test]
fn two_cycle_skew_outer_and_inner_nonoverlap_now_synthesized() {
    // SLICE N2b: outer |=> AND inner |=> = a genuine 2-cycle skew, now synthesized as
    // `(a ##1 b) |=> c` (was loud pre-N2b). a=b=1,c=0 → obligation every clock, c=0 two
    // clocks later → fires (an ASSERTION violation, NOT a VITA-E unsupported reject).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0; always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |=> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a 2-cycle (outer |=> + inner |=>) skew with c=0 must FIRE. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion") && !err.contains("unsupported"),
        "must be an assertion violation, not a loud unsupported reject:\n{err}\n{out}"
    );
}

#[test]
fn nonoverlap_inner_with_disable_iff_is_loud() {
    // inner |=> property carrying its own `disable iff` is beyond this slice → loud.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, rst=0, a=1, b=1, c=0; always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) disable iff (rst) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "inner |=> with disable iff must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn nonoverlap_propref_runs_deterministically() {
    let src = "module top;\n\
         reg clk=0, a=1, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         property q_bc; @(posedge clk) b |=> c; endproperty\n\
         property p_aq; @(posedge clk) a |-> q_bc; endproperty\n\
         initial assert property(p_aq);\n\
         initial #36 $finish;\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
