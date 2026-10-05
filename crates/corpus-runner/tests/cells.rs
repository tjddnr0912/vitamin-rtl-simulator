//! The oracle-cell manifest and its normalization, checked against the preserved
//! captures in `crates/testdata/cells/` — no simulator runs here. The graded run of
//! the manifest under this tree's vita is `crates/cli/tests/oracle_cells.rs`.

use std::path::PathBuf;

use corpus_runner::cells::admit::{classify_capture, Capture};
use corpus_runner::cells::normalize::{normalized, split_capture, vita_diagnostic, Producer};
use corpus_runner::cells::{
    self, admit, expect_file, manifest, Candidate, Excluded, Reason, SEED_ROWS,
};
use corpus_runner::{Expect, Run};

fn dir() -> PathBuf {
    cells::cells_dir(&corpus_runner::resolve_bench_root().expect("repository root"))
}

/// Every oracle block of one preserved cell, as (producer kind, classification).
fn classified_blocks(rel: &str) -> Vec<(String, Capture)> {
    let bytes = std::fs::read(dir().join(rel)).expect("preserved cell");
    let f = expect_file::parse(&bytes).expect("parses");
    f.blocks
        .iter()
        .filter_map(|b| {
            let tool = b.oracle()?;
            let text = std::str::from_utf8(&b.body).ok()?;
            Some((b.kind.clone(), classify_capture(tool, text)?))
        })
        .collect()
}

fn raw_block(rel: &str, kind_prefix: &str) -> String {
    let bytes = std::fs::read(dir().join(rel)).expect("preserved cell");
    let f = expect_file::parse(&bytes).expect("parses");
    let b = f
        .blocks
        .iter()
        .find(|b| b.kind.starts_with(kind_prefix))
        .unwrap_or_else(|| panic!("{rel}: no {kind_prefix} block"));
    String::from_utf8(b.body.clone()).expect("utf-8")
}

/// POSITIVE CONTROL. One real cell whose three oracle captures carry every family of
/// tool report at once — iverilog's compile warning, its continuation line and its
/// `$finish` notice; verilator's `%Warning` build line, `$finish` line and report
/// banner; sv2v→iverilog's warning against the translated file — and one design line.
/// All three normalize to that line.
#[test]
fn every_tools_report_on_one_real_cell_normalizes_away() {
    let rel = "AD/s589__g__c2__x_inst_hdr.expect";
    let iv = raw_block(rel, "oracle iverilog");
    assert!(iv.contains(": warning: Port 1 (p)") && iv.contains(":        : Pruning"));
    assert!(raw_block(rel, "oracle verilator").contains("%Warning-WIDTHTRUNC"));

    let blocks = classified_blocks(rel);
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    for (kind, c) in &blocks {
        assert_eq!(*c, Capture::Clean("top.u W=3 b=4 p=5".into()), "{kind}");
    }
}

/// POSITIVE CONTROL. iverilog's location-free internal notice (V, §4.5.592), and
/// sv2v's stderr that one runner appended AFTER its `rc=` marker (U, §4.5.591).
#[test]
fn location_free_notices_and_text_after_the_marker_are_not_output() {
    let s2v = raw_block("V/s592__g__newc__Z8a.expect", "oracle sv2v");
    assert!(s2v.contains("warning: verinum::as_long()"), "{s2v}");
    for (_, c) in classified_blocks("V/s592__g__newc__Z8a.expect") {
        if let Capture::Clean(a) = c {
            assert!(!a.contains("verinum"), "{a}");
        }
    }

    let s2v = raw_block("U/s591__plan__a3__n1.expect", "oracle sv2v");
    let (_, rc, before) = split_capture(&s2v).expect("a run capture");
    assert_eq!(rc, 0);
    assert!(
        s2v.len() > before.len() + "rc=0\n".len(),
        "text follows the marker"
    );
    let answers: Vec<String> = classified_blocks("U/s591__plan__a3__n1.expect")
        .into_iter()
        .filter_map(|(_, c)| match c {
            Capture::Clean(a) => Some(a),
            _ => None,
        })
        .collect();
    assert!(
        answers.len() >= 2 && answers.iter().all(|a| *a == answers[0]),
        "{answers:?}"
    );
}

/// DISSENT CONTROL. The §2 🆕 V cell Z5a (ROADMAP's open forward-reference defect):
/// sv2v and verilator print `@K=8`, iverilog refuses with `Unable to bind`. iverilog
/// decides that axis, so its refusal is a dissent and the cell is not admitted.
#[test]
fn an_oracle_that_rejects_the_design_excludes_the_cell() {
    let blocks = classified_blocks("V/s592__g__c__Z5a_procread_nosh.expect");
    assert!(blocks.iter().any(|(k, c)| k.starts_with("oracle iverilog")
        && matches!(c, Capture::Rejects(l) if l.contains("Unable to bind"))));
    let excluded = manifest::parse_excluded(
        &std::fs::read_to_string(dir().join(cells::EXCLUDED_FILE)).expect("exclusions"),
    )
    .expect("parses");
    let z5a = excluded
        .iter()
        .find(|x| x.path == "V/s592__g__c__Z5a_procread_nosh.sv")
        .expect("Z5a is listed");
    assert_eq!(z5a.reason, Reason::OracleRejects);
}

/// NEGATIVE CONTROL. A real design line that looks most like structure (`R: L=1`, a
/// `name: value` the §4.5.592 cells print) is pinned, and a changed value in a real
/// capture changes the answer.
#[test]
fn design_output_survives_normalization() {
    let manifest = std::fs::read_to_string(dir().join(cells::MANIFEST_FILE)).expect("manifest");
    assert!(
        manifest.contains("\n| R: L=1\n"),
        "the real `R: L=1` line is pinned"
    );
    let iv = raw_block("AD/s589__g__c2__x_inst_hdr.expect", "oracle iverilog");
    let mutated = iv.replace("p=5", "p=6");
    assert_ne!(
        normalized(Producer::Iverilog, split_capture(&iv).unwrap().2),
        normalized(Producer::Iverilog, split_capture(&mutated).unwrap().2)
    );
}

/// The checked-in manifest is what admission derives from the preserved captures:
/// the same cells, the same oracle sets, the same pinned oracle lines, and the same
/// oracle-side exclusions. A normalization change, or an edit to a capture, fails here
/// until `cells pin` is re-run.
#[test]
fn the_manifest_is_reproducible_from_the_captures() {
    let pinned = cells::load_manifest(&dir()).expect("manifest");
    let excluded = manifest::parse_excluded(
        &std::fs::read_to_string(dir().join(cells::EXCLUDED_FILE)).expect("exclusions"),
    )
    .expect("parses");
    let a = admit(&dir(), SEED_ROWS).expect("admission");

    let vita_side = [
        Reason::VitaCrashAtPin,
        Reason::VitaNondeterministicAtPin,
        Reason::OrderOnly,
    ];
    let from_candidates: Vec<(String, String, Vec<String>, String)> = a
        .candidates
        .iter()
        .filter(|c| {
            !excluded
                .iter()
                .any(|x| x.path == c.path && vita_side.contains(&x.reason))
        })
        .map(|c| {
            (
                c.path.clone(),
                c.name.clone(),
                c.oracles.clone(),
                c.oracle.clone(),
            )
        })
        .collect();
    let from_manifest: Vec<(String, String, Vec<String>, String)> = pinned
        .iter()
        .map(|c| {
            (
                c.path.clone(),
                c.name.clone(),
                c.oracles.clone(),
                c.oracle.clone(),
            )
        })
        .collect();
    assert_eq!(from_candidates.len(), from_manifest.len());
    assert!(
        from_candidates == from_manifest,
        "the manifest's oracle side has drifted"
    );

    let oracle_side: Vec<&Excluded> = excluded
        .iter()
        .filter(|x| !vita_side.contains(&x.reason))
        .collect();
    assert_eq!(oracle_side, a.excluded.iter().collect::<Vec<_>>());
    // Every candidate is accounted for exactly once.
    assert_eq!(
        a.candidates.len() + a.excluded.len(),
        pinned.len() + excluded.len()
    );
    // Non-vacuous: every expectation is present and every seed row contributes.
    for kind in ["runs", "known-wrong", "refused"] {
        assert!(pinned.iter().any(|c| c.kind() == kind), "no {kind} cell");
    }
    for row in SEED_ROWS {
        assert!(pinned.iter().any(|c| c.row() == *row), "no cell from {row}");
    }
    // Two simulators behind every pin: the iverilog family counts once.
    for c in &pinned {
        let families: std::collections::BTreeSet<&str> = c
            .oracles
            .iter()
            .map(|t| cells::admit::family_of(t))
            .collect();
        assert!(families.len() >= 2, "{}: {:?}", c.path, c.oracles);
    }
}

/// The vita half of every pin has the shape the gate compares: a refusal is a whole
/// vita error line with its file stripped plus the sorted codes of every error, a
/// known-wrong answer differs from the oracles' and is not merely reordered. A pin
/// degraded by hand (`refusal error`) fails here and in the graded run.
#[test]
fn the_vita_half_of_every_pin_has_its_shape() {
    for c in cells::load_manifest(&dir()).expect("manifest") {
        match &c.expect {
            Expect::Refused { diag } => {
                let mut parts = diag.split('\n');
                let (first, codes, at) = (
                    parts.next().expect("first line"),
                    parts
                        .next()
                        .and_then(|l| l.strip_prefix("codes "))
                        .expect("codes"),
                    parts
                        .next()
                        .and_then(|l| l.strip_prefix("at "))
                        .expect("at"),
                );
                assert!(parts.next().is_none(), "{}", c.path);
                assert!(
                    at == "-"
                        || at
                            .split(' ')
                            .all(|p| p.split(':').all(|n| n.parse::<u32>().is_ok())),
                    "{}: at {at:?}",
                    c.path
                );
                // `L:C: sev[…] …`, or a diagnostic vita printed with no location.
                let body = match first.split_once(": ") {
                    Some((loc, body))
                        if loc.split(':').count() == 2
                            && loc.split(':').all(|n| n.parse::<u32>().is_ok()) =>
                    {
                        body
                    }
                    _ => first,
                };
                let (sev, code) = vita_diagnostic(body).expect("a vita diagnostic");
                assert!(matches!(sev, "error" | "fatal"), "{}: {first}", c.path);
                let codes: Vec<&str> = codes.split(' ').collect();
                let mut sorted = codes.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(codes, sorted, "{}", c.path);
                assert!(codes.contains(&code), "{}: {code} not in {codes:?}", c.path);
                assert!(codes.iter().all(|k| k.starts_with("VITA-")), "{}", c.path);
            }
            Expect::KnownWrong { vita, .. } => {
                assert_ne!(*vita, c.oracle, "{}", c.path);
                let mut a: Vec<&str> = vita.split('\n').collect();
                let mut b: Vec<&str> = c.oracle.split('\n').collect();
                a.sort_unstable();
                b.sort_unstable();
                assert_ne!(a, b, "{}: an order-only pin", c.path);
            }
            Expect::Runs { exit } => assert_eq!(*exit, 0, "{}", c.path),
            Expect::Split { .. } => panic!("{}: a cell is never a ruled split", c.path),
        }
    }
}

fn fake_run(c: &Candidate, salt: usize) -> Run {
    // A deterministic stand-in for vita: right, wrong or refused by the path's hash.
    let h = c
        .path
        .bytes()
        .fold(salt, |h, b| h.wrapping_mul(31).wrapping_add(b as usize));
    let (code, stdout, stderr) = match h % 3 {
        0 => (0, format!("{}\n", c.oracle), ""),
        1 => (0, "wrong\n".to_string(), ""),
        _ => (1, String::new(), "x.sv:1:1: error[VITA-E3009] E-X: no\n"),
    };
    Run {
        code: Some(code),
        stdout,
        stderr: stderr.into(),
        secs: 0.0,
        timed_out: false,
    }
}

/// The generator is a function of the captures and the binary's answers: two runs,
/// with different worker counts and so a different completion order, write the same
/// bytes.
#[test]
fn the_generator_is_deterministic() {
    let a1 = admit(&dir(), SEED_ROWS).expect("admission");
    let a2 = admit(&dir(), &["W", "V", "U", "AI", "AE", "AD", "AE"]).expect("admission");
    assert!(a1 == a2, "admission depends on the order rows were named");
    assert!(!a1.candidates.is_empty());

    let render = |a: cells::Admission, jobs: usize| {
        let p = cells::pin_with(a, jobs, |_, c| (fake_run(c, 7), fake_run(c, 7)));
        let h = cells::header("fake", SEED_ROWS, &p);
        (
            manifest::render(&h, &p.cells),
            manifest::render_excluded(&h, &p.excluded),
        )
    };
    let one = render(a1, 1);
    let many = render(a2, 8);
    assert!(one == many, "the generated files depend on scheduling");
    // And the fake's three answers all reached the file.
    for kind in [
        "expect runs 0",
        "expect known-wrong 0",
        "expect refused\nrefusal 1:1: ",
    ] {
        assert!(one.0.contains(kind), "{kind}");
    }
}

/// What `cells pin` writes, `cells run` reads back unchanged — and the checked-in
/// file is byte-for-byte what the writer produces from it.
#[test]
fn the_manifest_round_trips() {
    let text = std::fs::read_to_string(dir().join(cells::MANIFEST_FILE)).expect("manifest");
    let header: Vec<String> = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(|l| l.trim_start_matches("# ").to_string())
        .collect();
    let parsed = manifest::parse(&text).expect("parses");
    assert_eq!(manifest::render(&header, &parsed), text);
    // A known-wrong pin keeps the empty answer distinct from no lines at all, and a
    // refusal keeps both of its parts.
    let c = |expect| cells::Cell {
        path: "AE/x.sv".into(),
        name: "x.sv".into(),
        oracles: vec!["iverilog".into(), "verilator".into()],
        oracle: "a\n\n| b".into(),
        expect,
    };
    let cs = vec![
        c(Expect::KnownWrong {
            exit: 0,
            vita: String::new(),
        }),
        cells::Cell {
            path: "AE/y.sv".into(),
            ..c(Expect::Refused {
                diag: "3:5: error[VITA-E3009] E-X: no\ncodes VITA-E3009 VITA-E3010\nat 3:5 6:1"
                    .into(),
            })
        },
    ];
    let back = manifest::parse(&manifest::render(&[], &cs)).expect("parses");
    assert_eq!(back, cs);
}

/// The graded table is read by column offset.
#[test]
fn every_manifest_path_fits_the_cell_column() {
    for c in cells::load_manifest(&dir()).expect("manifest") {
        assert!(
            c.path.chars().count() < cells::CELL_COLUMN,
            "{} is wider than the cell column",
            c.path
        );
        assert!(dir().join(&c.path).is_file(), "{} is missing", c.path);
    }
}
