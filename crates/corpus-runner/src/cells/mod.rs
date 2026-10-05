//! The oracle cells: a regression gate built on `crates/testdata/cells/`.
//!
//! The workload corpus gates real designs on one digest each, and the defect classes
//! the structural track works on fire zero times there. The cells are where they
//! fire: thousands of small designs the slices §4.5.580–594 wrote, each preserved with
//! the raw output of every oracle that ran it. This module turns them into a gate that
//! tells apart the three moves a change can make on a cell —
//!
//! | Pinned | Now | Grade |
//! |---|---|---|
//! | runs (right) | the oracles' answer | `ok` |
//! | runs (right) | anything else | `REGRESSION` (broken) |
//! | known-wrong | the same wrong answer | `known-wrong` |
//! | known-wrong | the oracles' answer | `PROMOTED` (fixed) |
//! | known-wrong | another wrong answer, or a refusal | `DRIFTED` (differently wrong; wrong to loud) |
//! | known-wrong | a crash, a hang, another exit | `REGRESSION` |
//! | refused | the exact pinned refusal / another refusal / the oracles' answer | `known-gap` / `DRIFTED` / `PROMOTED` |
//!
//! — through the same [`Expect`](crate::Expect) and [`Grade`](crate::Grade) as the
//! workload corpus. Unlike the workload table, every move fails here: only `ok`,
//! `known-wrong` and `known-gap` pass ([`Graded::fails`]), so a fix fails until it is
//! re-pinned and cannot be lost silently.
//!
//! Two subcommands. `cells pin` reads the preserved captures, admits the cells whose
//! oracles agree ([`admit`]), runs each twice under the binary it is given, and writes
//! `MANIFEST.txt` and `MANIFEST.excluded.tsv` — after [`guard`] has compared them with
//! the manifest already there and every regression-direction move is named. `cells
//! run` grades every manifest cell against a binary. Nothing else writes the manifest.

pub mod admit;
pub mod exec;
pub mod expect_file;
pub mod forward_ref;
pub mod guard;
pub mod manifest;
pub mod normalize;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub use admit::{admit, row_of, Admission, Candidate, Excluded, Reason};
pub use exec::{
    cell_outcome, classify, default_jobs, grade_cell, grade_outcome, par_map, run_cell, Graded,
};
pub use manifest::Cell;

/// The rows the seed was cut from: the slices §4.5.587–594 (🆕 AE, AI, V, U, W, AD).
/// Other preserved rows are not admitted yet.
pub const SEED_ROWS: &[&str] = &["AD", "AE", "AI", "U", "V", "W"];

/// The manifest and its exclusion list, beside the cells.
pub const MANIFEST_FILE: &str = "MANIFEST.txt";
pub const EXCLUDED_FILE: &str = "MANIFEST.excluded.tsv";

/// A cell's wall-clock budget per run. The slowest seed cell takes well under a
/// second; a run that reaches this hangs.
pub const BUDGET: Duration = Duration::from_secs(20);

/// The width of the `cell` column of the graded table. Fixed, because the table is
/// read by column offset; `tests/cells.rs` holds every manifest path to it.
pub const CELL_COLUMN: usize = 76;

/// Where the cells live in a checkout.
pub fn cells_dir(root: &Path) -> PathBuf {
    root.join("crates/testdata/cells")
}

/// Read and parse `MANIFEST.txt`.
pub fn load_manifest(cells: &Path) -> Result<Vec<Cell>, String> {
    let p = cells.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    manifest::parse(&text)
}

/// Read `MANIFEST.excluded.tsv`; empty when the file is not there.
pub fn load_excluded(cells: &Path) -> Result<Vec<Excluded>, String> {
    let p = cells.join(EXCLUDED_FILE);
    if !p.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    manifest::parse_excluded(&text)
}

/// The pin's result, before it is written.
pub struct Pinned {
    pub cells: Vec<Cell>,
    pub excluded: Vec<Excluded>,
}

/// Classify every admitted candidate with `runner`, which returns two runs of one
/// candidate. Separated from the process spawning so the generator's determinism is
/// testable without a vita binary.
pub fn pin_with(
    admission: Admission,
    jobs: usize,
    runner: impl Fn(usize, &Candidate) -> (crate::Run, crate::Run) + Sync,
) -> Pinned {
    let results = par_map(&admission.candidates, jobs, |slot, c| {
        let (a, b) = runner(slot, c);
        classify(c, &a, &b)
    });
    let mut cells = Vec::new();
    let mut excluded = admission.excluded;
    for r in results {
        match r {
            Ok(c) => cells.push(c),
            Err(x) => excluded.push(x),
        }
    }
    cells.sort_by(|a, b| a.path.cmp(&b.path));
    excluded.sort_by(|a, b| a.path.cmp(&b.path));
    Pinned { cells, excluded }
}

/// Pin against a real binary: two runs per candidate.
pub fn pin(cells: &Path, rows: &[&str], vita: &Path, jobs: usize) -> Result<Pinned, String> {
    let admission = admit(cells, rows)?;
    Ok(pin_with(admission, jobs, |slot, c| {
        let a = run_cell(vita, cells, &c.path, &c.name, slot, BUDGET);
        let b = run_cell(vita, cells, &c.path, &c.name, slot, BUDGET);
        (a, b)
    }))
}

/// The header both generated files carry.
pub fn header(label: &str, rows: &[&str], p: &Pinned) -> Vec<String> {
    let mut rows: Vec<&str> = rows.to_vec();
    rows.sort_unstable();
    rows.dedup();
    let kinds = ["runs", "known-wrong", "refused"];
    let count = |k: &str| p.cells.iter().filter(|c| c.kind() == k).count();
    let mut reasons: BTreeMap<Reason, usize> = BTreeMap::new();
    for x in &p.excluded {
        *reasons.entry(x.reason).or_default() += 1;
    }
    vec![
        "Oracle-cell manifest, generated by `corpus-runner cells pin`. Do not edit by hand.".into(),
        "Format and grading: README.md, \"Manifest and harness\".".into(),
        format!("classified by: {label}"),
        format!("rows: {}", rows.join(" ")),
        format!(
            "cells: {} = {}",
            p.cells.len(),
            kinds
                .iter()
                .map(|k| format!("{k} {}", count(k)))
                .collect::<Vec<_>>()
                .join(" + ")
        ),
        excluded_line(p.excluded.len(), &reasons),
    ]
}

/// `excluded: N = reason n + …`, or `excluded: 0`.
fn excluded_line(total: usize, reasons: &BTreeMap<Reason, usize>) -> String {
    let parts: Vec<String> = reasons
        .iter()
        .map(|(r, n)| format!("{} {n}", r.label()))
        .collect();
    if parts.is_empty() {
        format!("excluded: {total}")
    } else {
        format!("excluded: {total} = {}", parts.join(" + "))
    }
}

/// Write `MANIFEST.txt` and `MANIFEST.excluded.tsv`.
pub fn write_pinned(cells: &Path, label: &str, rows: &[&str], p: &Pinned) -> Result<(), String> {
    let h = header(label, rows, p);
    let write = |name: &str, text: String| {
        let path = cells.join(name);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    };
    write(MANIFEST_FILE, manifest::render(&h, &p.cells))?;
    write(EXCLUDED_FILE, manifest::render_excluded(&h, &p.excluded))
}

/// Grade `cells` against `vita`, two runs each.
pub fn run_manifest(
    cells_root: &Path,
    cells: &[&Cell],
    vita: &Path,
    jobs: usize,
    budget: Duration,
) -> Vec<Graded> {
    par_map(cells, jobs, |slot, c| {
        let a = run_cell(vita, cells_root, &c.path, &c.name, slot, budget);
        let b = run_cell(vita, cells_root, &c.path, &c.name, slot, budget);
        grade_cell(c, &a, &b)
    })
}

/// Print the graded table and its summary; returns the number of failing rows.
pub fn print_table(cells: &[&Cell], graded: &[Graded]) -> usize {
    println!("{:<CELL_COLUMN$} {:<12} detail", "cell", "grade");
    let mut failures = 0usize;
    let mut by_label: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_row: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for (c, g) in cells.iter().zip(graded) {
        let label = g.grade.label();
        if g.fails() {
            failures += 1;
        }
        *by_label.entry(label).or_default() += 1;
        *by_row.entry(c.row()).or_default().entry(label).or_default() += 1;
        println!("{:<CELL_COLUMN$} {label:<12} {}", c.path, g.detail);
    }
    let fmt = |m: &BTreeMap<&str, usize>| {
        m.iter()
            .map(|(l, n)| format!("{l} {n}"))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    println!("\ncells: {} graded · {}", graded.len(), fmt(&by_label));
    for (row, m) in &by_row {
        println!("  {row:<4} {}", fmt(m));
    }
    failures
}
