//! Manifest hygiene.
//!
//! The workloads themselves are not in the repository and are absent in CI, so none
//! of this runs a simulator. What it does check is everything about the manifest that
//! can rot without anyone noticing: a licence that is not actually permissive, a
//! branch name where a commit SHA should be, two rows fighting over one artifact
//! name, or a pinned digest that could never match the line a testbench prints.

use corpus_runner::{coverage, Expect, Origin, CORPUS};

/// Every marker `digest_line` scans for. Kept here so a pin using a marker the
/// scanner does not know — or a scanner that quietly drops one — fails loudly.
const DIGEST_MARKERS: &[&str] = &["DIGEST=", " acc="];

/// Only these may enter the corpus. The RTL is fetched onto a developer's machine
/// and measured; a copyleft or CERN-OHL design would put obligations on this
/// repository that it cannot meet, and `bench/*` being gitignored is not a defence.
const PERMISSIVE: &[&str] = &["MIT", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Apache-2.0"];

#[test]
fn every_upstream_workload_is_permissively_licensed() {
    for w in CORPUS {
        if let Origin::Upstream { license, repo, .. } = w.origin {
            assert!(
                PERMISSIVE.contains(&license),
                "{}: licence {license:?} ({repo}) is not on the permissive list",
                w.name
            );
        }
    }
}

/// A tag or a branch would let upstream move under a pinned digest, which would turn
/// a corpus failure into a mystery. Only a full commit SHA holds still.
#[test]
fn every_upstream_workload_is_pinned_to_a_full_commit_sha() {
    for w in CORPUS {
        if let Origin::Upstream { sha, .. } = w.origin {
            assert_eq!(sha.len(), 40, "{}: {sha:?} is not a full SHA", w.name);
            assert!(
                sha.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{}: {sha:?} is not lowercase hex",
                w.name
            );
        }
    }
}

/// `prepare_iverilog` writes `<name>.vvp` into the working directory, so two rows
/// sharing a name would have them overwrite each other — and `keccak`/`keccak-arr`
/// deliberately share a directory, which is exactly when this bites.
#[test]
fn workload_names_are_unique() {
    let mut seen: Vec<&str> = CORPUS.iter().map(|w| w.name).collect();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "duplicate workload name in the corpus");
}

/// The runner finds the digest by scanning output for these markers. A pin that
/// contains none of them can never be matched, and the workload would report a
/// crash forever without anyone understanding why.
#[test]
fn every_pinned_digest_is_one_the_scanner_can_find() {
    for w in CORPUS {
        assert!(
            DIGEST_MARKERS.iter().any(|m| w.digest.contains(m)),
            "{}: pinned digest {:?} matches no marker the runner scans for",
            w.name,
            w.digest
        );
    }
}

#[test]
fn every_workload_has_sources_and_an_oracle() {
    for w in CORPUS {
        assert!(!w.files.is_empty(), "{}: no sources", w.name);
        assert!(!w.oracle.is_empty(), "{}: no oracle recorded", w.name);
        assert!(
            !w.dir.is_empty() && !w.root.is_empty(),
            "{}: no directory",
            w.name
        );
        assert!(
            w.dir.starts_with(w.root),
            "{}: working dir {:?} is not inside its root {:?}",
            w.name,
            w.dir,
            w.root
        );
    }
}

/// A refusal pinned as an empty string would match every diagnostic, so a *changed*
/// gap would grade as the known one.
#[test]
fn every_pinned_refusal_names_a_reason() {
    for w in CORPUS {
        if let Expect::Refused { diag } = w.expect {
            assert!(
                diag.len() > 10,
                "{}: pinned refusal {diag:?} is too vague to distinguish a drift",
                w.name
            );
        }
    }
}

/// The corpus exists to contain designs vita does not run. If this ever reads
/// `n == total`, the corpus has stopped doing its job — either every gap really did
/// close (in which case add harder designs) or the refusals were quietly dropped.
#[test]
fn coverage_is_reported_over_the_whole_corpus() {
    let (runs, total) = coverage();
    assert_eq!(total, CORPUS.len());
    assert!(runs <= total);
    assert!(
        total >= 8,
        "a corpus this small cannot price anything: {total}"
    );
}

/// Shape diversity is the difference between measuring one design eight times and
/// measuring eight designs.
#[test]
fn the_corpus_covers_more_than_one_shape() {
    let mut shapes: Vec<&str> = CORPUS.iter().map(|w| w.shape.label()).collect();
    shapes.sort_unstable();
    shapes.dedup();
    assert!(
        shapes.len() >= 3,
        "only {} shape(s): {shapes:?}",
        shapes.len()
    );
}
