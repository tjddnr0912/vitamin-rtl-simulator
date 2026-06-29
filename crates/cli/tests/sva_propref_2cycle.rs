//! Slice N2b — genuine 2-cycle non-overlap property-reference skew.
//! An outer NON-OVERLAP assertion whose consequent is a clean inner NON-OVERLAP
//! property: `property q; @(clk) b |=> c;  property p; @(clk) a |=> q;`. By IEEE
//! 1800 §16.12 textual substitution `a |=> (b |=> c)` ≡ `(a ##1 b) |=> c` — a
//! genuine TWO-clock obligation: a@k, b@(k+1), c due@(k+2). Lowered by rewriting the
//! antecedent to the sequence `a ##1 b` (kept NonOverlap) so the existing sequence
//! pipeline (the a→b `##1` skew) + the top-level `|=>` pend reg (the b→c skew)
//! produce both skews — pure IR-0 (sim-ir frozen, format_version 8; NO AST change).
//!
//! iverilog 13.0 rejects all concurrent assertions (NULL oracle) → hand-IEEE; every
//! expected value below is derived from §16.12 + value-pinned. A DEEPER homogeneous
//! chain (`a |=> (b |=> (d |=> e))`, inner consequent itself a property) is now
//! synthesized as `(a ##1 b ##1 d) |=> e` (slice #6, see `sva_propref_chain.rs`).
//! STILL LOUD (deferred): a MIXED `|=>…|->` chain, a different/multi clock, formals,
//! `disable iff`, or a recursive property.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sva2c_{}_{n}", std::process::id()));
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

#[test]
fn two_cycle_skew_violation_fires() {
    // a=b=1 always, c=0 always → the `a ##1 b` antecedent matches every clock and c
    // is low two clocks later → fires.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "2-cycle skew with c=0 must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion") && !err.contains("unsupported"),
        "must be an assertion violation, not a loud reject:\n{err}\n{out}"
    );
}

#[test]
fn two_cycle_skew_holds() {
    // a=b=c=1 always → obligation met two clocks later every time → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "c held two clocks later must hold → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

#[test]
fn skew_is_genuinely_two_cycles_not_one() {
    // THE KEY TEST. `a` pulses high only at the FIRST posedge (t=5), b=1 always, c=1
    // everywhere EXCEPT c=0 at t=25. The single antecedent match `a@t5 ##1 b@t15`
    // completes at t=15; the obligation is then due at t=25 (TWO clocks after a). c=0
    // there → fires exactly once. A wrong ONE-cycle reading would check c at the match
    // clock t=15 (c=1) and NOT fire. So a fire proves the genuine 2-cycle skew.
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
        "the obligation must be checked TWO clocks after a (c=0 @t25) → fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation at the 2-cycle point:\n{err}\n{out}"
    );
}

#[test]
fn outer_antecedent_false_is_vacuous() {
    // a=0 → `a ##1 b` never matches → no obligation → clean regardless of c.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=1, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "outer a=0 → vacuous → clean. {err}");
}

#[test]
fn inner_antecedent_false_is_vacuous() {
    // b=0 → `a ##1 b` never matches → no obligation → clean. Proves b genuinely gates
    // (the desugar is NOT `a |=> (1 |=> c)` = `a ##1 1 |=> c`).
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=1, b=0, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #36 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "inner b=0 → vacuous → clean. {err}");
}

#[test]
fn deeper_three_cycle_chain_now_synthesized() {
    // SLICE #6: `a |=> (b |=> (d |=> e))` (inner consequent is itself a property ref)
    // ≡ `(a ##1 b ##1 d) |=> e` (§16.12, +1 per `|=>`) is now SYNTHESIZED (was loud
    // pre-#6). a=b=d=1, e=0 always → antecedent matches every clock, e=0 three clocks
    // later → fires an ASSERTION violation (NOT a VITA-E unsupported reject). The exact
    // +3 cycle is pinned by the off-by-one discriminators in `sva_propref_chain.rs`.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, d=1, e=0; always #5 clk=~clk;\n\
         property r; @(posedge clk) d |=> e; endproperty\n\
         property q; @(posedge clk) b |=> r; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #46 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a deeper 3-cycle chain with e=0 must FIRE (now synthesized). stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion") && !err.contains("unsupported"),
        "must be an assertion violation, not a loud unsupported reject:\n{err}\n{out}"
    );
}

#[test]
fn two_cycle_skew_runs_deterministically() {
    let src = "module top;\n\
         reg clk=0, a=1, b=1, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) a |=> q; endproperty\n\
         initial assert property(p);\n\
         initial #36 $finish;\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
