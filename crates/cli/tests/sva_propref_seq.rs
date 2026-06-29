//! Slice A.3 — SVA outer-implication property-reference skew with a SEQUENCE
//! antecedent (IEEE 1800-2017 §16.12 textual substitution).
//!
//! The OUTER antecedent is a sequence (not a bare boolean); the consequent is a
//! clean inner NON-OVERLAP property `q: b |=> c`. §16.12 textual substitution:
//!   `seq |-> (b |=> c)` ≡ `(seq ##0 b) |=> c`   (overlap fuses b at seq-end)
//!   `seq |=> (b |=> c)` ≡ `(seq ##1 b) |=> c`   (non-overlap skews one clock)
//! Lowered by rewriting the antecedent to that sequence (kind NonOverlap) so the
//! existing sequence pipeline + the top-level `|=>` pend reg produce all skews —
//! pure IR-0 (sim-ir frozen, format_version 19; NO AST change).
//!
//! iverilog 13.0 rejects all concurrent assertions (NULL oracle) → hand-IEEE;
//! every expected value below is derived by clock-counting from §16.12 and
//! value-pinned. clk posedge every 10ns starting t=5 (always #5 clk=~clk, clk
//! starts 0 → first posedge at t=5, then t=15, t=25, …). Deeper HOMOGENEOUS `|=>`
//! chains are now synthesized (slice #6, see `sva_propref_chain.rs`); STILL LOUD
//! (deferred): a MIXED `|=>…|->` chain, a different/multi clock, formals, `disable
//! iff`, a recursive property.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaseq_{}_{n}", std::process::id()));
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

// ── seq |-> q (overlap outer, sequence antecedent) ─────────────────────────────

#[test]
fn seq_overlap_prop_ref_fires() {
    // `x ##1 y |-> q`, q = `b |=> c`. ≡ `((x ##1 y) ##0 b) |=> c`.
    // x=1@t5, y=1@t15 → the antecedent sequence ends at t15. b=1 there → fuses
    // (##0) at t15. The inner |=> then makes c due the NEXT clock t25; c=0@t25 →
    // FIRES at t25.
    let (out, err, code) = run("module top;\n\
         reg clk=0, x=0, y=0, b=1, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) x ##1 y |-> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #40 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "seq|->q with c=0 at t25 must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion") && !err.contains("unsupported"),
        "must be an assertion violation, not a loud reject:\n{err}\n{out}"
    );
}

#[test]
fn seq_overlap_prop_ref_discriminating() {
    // Same shape, but c=1 everywhere EXCEPT c=0 at t15 (the seq-end clock). The
    // inner |=> checks c the NEXT clock (t25), where c=1 → must NOT fire. A wrong
    // overlap (`|->`) reading would check c at the fuse clock t15 (c=0) and fire.
    // So a clean exit proves the inner |=> skew is genuine.
    let (out, err, code) = run("module top;\n\
         reg clk=0, x=0, y=0, b=1, c=1; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) x ##1 y |-> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #3 c=0; #3 c=1; #34 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "c checked at t25 (not t15) → must NOT fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

#[test]
fn seq_overlap_prop_ref_b_gates() {
    // b=0 always → `(x ##1 y) ##0 b` never completes (the ##0 fuse fails) →
    // vacuous → clean regardless of c. Proves b is FUSED into the antecedent, not
    // silently dropped.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, x=0, y=0, b=0, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) x ##1 y |-> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #40 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "b=0 → ##0 fuse fails → vacuous → clean. {err}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

// ── seq |=> q (non-overlap outer, sequence antecedent) ─────────────────────────

#[test]
fn seq_nonoverlap_prop_ref_fires() {
    // `x ##1 y |=> q`, q = `b |=> c`. ≡ `((x ##1 y) ##1 b) |=> c`.
    // x=1@t5, y=1@t15 → seq ends t15. The outer |=> skews one clock (##1) to b@t25;
    // b=1 there. The inner |=> then makes c due the NEXT clock t35; c=0 → FIRES@t35.
    let (out, err, code) = run("module top;\n\
         reg clk=0, x=0, y=0, b=1, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) x ##1 y |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #50 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "seq|=>q with c=0 at t35 must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion") && !err.contains("unsupported"),
        "must be an assertion violation, not a loud reject:\n{err}\n{out}"
    );
}

#[test]
fn seq_nonoverlap_prop_ref_discriminating() {
    // Same shape, but c=1 everywhere EXCEPT c=0 at t25 (the b clock, one too early).
    // The obligation is due at t35 (c=1 there) → must NOT fire. A wrong one-clock-
    // short reading would check c at t25 (c=0) and fire. So clean exit proves the
    // full `##1` + inner-|=> 2-clock skew.
    let (out, err, code) = run("module top;\n\
         reg clk=0, x=0, y=0, b=1, c=1; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) x ##1 y |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #13 c=0; #3 c=1; #24 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "c due at t35 (not t25) → must NOT fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

// ── LOUD preserved: cross-clock inner property ─────────────────────────────────

#[test]
fn cross_clock_inner_prop_is_loud() {
    // The inner property q is on clk2 while the outer assertion is on clk1 →
    // peel_nonoverlap_property returns None (clock mismatch) → existing loud reject.
    // Must NOT be silently synthesized.
    let (_o, err, code) = run("module top;\n\
         reg clk1=0, clk2=0, x=0, y=0, b=1, c=0;\n\
         always #5 clk1=~clk1; always #7 clk2=~clk2;\n\
         property q; @(posedge clk2) b |=> c; endproperty\n\
         property p; @(posedge clk1) x ##1 y |-> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #40 $finish; end\n\
         endmodule\n");
    assert!(
        err.contains("VITA-E"),
        "cross-clock inner prop must be loud, not silently synthesized:\n{err}"
    );
    let _ = code;
}

// ── determinism ───────────────────────────────────────────────────────────────

#[test]
fn seq_prop_ref_runs_deterministically() {
    let src = "module top;\n\
         reg clk=0, x=0, y=0, b=1, c=0; always #5 clk=~clk;\n\
         property q; @(posedge clk) b |=> c; endproperty\n\
         property p; @(posedge clk) x ##1 y |=> q; endproperty\n\
         initial assert property(p);\n\
         initial begin #1 x=1; #10 x=0; y=1; #50 $finish; end\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
