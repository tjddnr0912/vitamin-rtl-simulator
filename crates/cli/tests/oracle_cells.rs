//! The oracle cells, graded against this tree's `vita` on every gate.
//!
//! `crates/testdata/cells/MANIFEST.txt` pins, for each admitted cell, the answer two or
//! more independent simulators agree on (iverilog 13.0 `-g2012` — alone or behind sv2v
//! 0.0.13, one simulator — and verilator 5.052, captured by the slices §4.5.587–594 and
//! preserved beside the cell) and what vita did when the manifest was generated: printed
//! that answer (`runs`), printed a pinned wrong one (`known-wrong`), or refused with a
//! pinned first error line and error-code set (`refused`). This runs every cell twice
//! and fails on any move from the manifest: `REGRESSION` (a right cell went wrong, a
//! cell crashed or hung, two runs disagreed), `DRIFTED` (a wrong answer or refusal
//! moved, or a wrong answer became a refusal) and `PROMOTED` (a wrong or refused cell
//! now prints the oracles' answer — a fix, which fails until it is re-pinned with
//! `cargo run -p corpus-runner -- cells pin`, so that reverting it later is caught).
//!
//! The same table, with a release binary: `cargo run -p corpus-runner -- cells run`.

use std::path::Path;

use corpus_runner::cells;

#[test]
fn every_oracle_cell_grades_as_pinned() {
    let root = corpus_runner::resolve_bench_root().expect("repository root");
    let dir = cells::cells_dir(&root);
    let all = cells::load_manifest(&dir).expect("MANIFEST.txt parses");
    let selected: Vec<&cells::Cell> = all.iter().collect();
    let graded = cells::run_manifest(
        &dir,
        &selected,
        Path::new(env!("CARGO_BIN_EXE_vita")),
        cells::default_jobs(),
        cells::BUDGET,
    );

    let mut failing = Vec::new();
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    for (c, g) in selected.iter().zip(&graded) {
        *counts.entry(g.grade.label()).or_default() += 1;
        if g.fails() {
            failing.push(format!("{} {} {}", c.path, g.grade.label(), g.detail));
        }
    }
    eprintln!("oracle cells: {counts:?}");
    assert!(
        failing.is_empty(),
        "{} of {} oracle cells fail:\n{}",
        failing.len(),
        graded.len(),
        failing.join("\n")
    );
    // Not vacuous: every cell was graded, and cells that run printed the oracles'
    // answer through this harness (a harness that lost the output could not).
    assert_eq!(graded.len(), all.len());
    assert!(
        counts.get("ok").copied().unwrap_or(0) > 0,
        "no `ok` row: {counts:?}"
    );
}
