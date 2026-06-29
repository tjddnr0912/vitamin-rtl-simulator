//! [P6] Tests for the deterministic constrained-random corpus generator
//! (`common::corpus`). Proves: (1) reproducibility — same seed → byte-identical
//! designs (the cross-OS determinism the gate relies on); (2) variation across
//! seeds; (3) EVERY generated design parses, elaborates, and runs to `$finish`
//! cleanly (validates that the templates only emit synthesizable RTL).

mod common;

use common::{build, corpus, run_on};
use sim_engine::{Backend, ExitClass, FinishReason};

/// Same seed ⇒ byte-for-byte identical corpus. This is what lets the P5 gate pin a
/// fixed corpus reproducibly on all three CI legs.
#[test]
fn corpus_is_reproducible() {
    let a = corpus(0x00C0_FFEE, 32);
    let b = corpus(0x00C0_FFEE, 32);
    assert_eq!(a.len(), 32);
    assert_eq!(a, b, "same seed must produce byte-identical designs");
}

/// Different seeds ⇒ different corpora (the generator actually varies).
#[test]
fn corpus_varies_by_seed() {
    let a = corpus(1, 24);
    let b = corpus(2, 24);
    assert_ne!(a, b, "different seeds should produce different designs");
}

/// The corpus spans every template (round-robin), so a corpus of >= TEMPLATES.len()
/// touches each corner. With 9 templates, 9 designs hit all of them; names are unique.
#[test]
fn corpus_covers_all_templates_with_unique_names() {
    let designs = corpus(42, 45);
    let mut names: Vec<&str> = designs.iter().map(|d| d.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 45, "design names must be unique");
    // Each template kind is represented (prefix check over all 9).
    let prefixes = [
        "counter_",
        "alu_",
        "shiftreg_",
        "mem_oob_",
        "nba_sample_",
        "wide_",
        "xz_index_",
        "glitch_",
        "cont_mixed_",
    ];
    for p in prefixes {
        assert!(
            designs.iter().any(|d| d.name.starts_with(p)),
            "no design for template {p}"
        );
    }
}

/// THE validation: every generated design builds (parse+elaborate, no hard errors)
/// and runs to `$finish` on the interpreter. A template that emits invalid or
/// non-terminating RTL fails here loudly with its source.
#[test]
fn every_corpus_design_builds_and_runs() {
    for d in corpus(0xABCD_1234, 48) {
        let ir = build(&d.src);
        let (res, _out) = run_on(&ir, Backend::Interpreter, &d.name);
        assert_eq!(
            res.finish_reason,
            FinishReason::Finish,
            "design {} did not reach $finish (got {:?})",
            d.name,
            res.finish_reason
        );
        // OOR/X-index designs may emit E-RUN-RANGE (recovered), but never Fatal.
        assert_ne!(
            res.exit_class,
            ExitClass::Fatal,
            "design {} ended Fatal",
            d.name
        );
    }
}
