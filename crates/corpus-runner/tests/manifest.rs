//! Manifest hygiene.
//!
//! The workloads themselves are not in the repository and are absent from the test
//! jobs (CI's corpus jobs fetch and run them), so none of this runs a simulator. What it does check is everything about the manifest that
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

/// verilator is 2-state. It stands as a row's oracle only when iverilog cannot parse
/// the design and the digest was shown not to depend on x (contract rule 2). A row that
/// skipped the second half would grade vita against another design's answer: picorv32's
/// and serv's verilator digests are exactly that.
#[test]
fn a_verilator_oracle_row_records_its_x_invariance_check() {
    for w in CORPUS {
        if w.oracle.to_ascii_lowercase().starts_with("verilator") {
            assert!(
                w.oracle.contains("x-invariant"),
                "{}: verilator is the oracle but no x-invariance check is recorded: {:?}",
                w.name,
                w.oracle
            );
        }
    }
}

/// `fetch --run` may remove `bench/<root>/src` (a stale clone), so every upstream root
/// must name one directory of its own: exactly one plain path component — no separator,
/// no `.` or `..`, not absolute, no whitespace — and no two upstream rows may share one.
/// Two rows on one root with different pins would each delete the other's clone on every
/// `fetch --run`.
#[test]
fn every_upstream_root_is_one_plain_component_and_unique() {
    use std::path::{Component, Path};
    let mut roots: Vec<&str> = Vec::new();
    for w in CORPUS {
        if !matches!(w.origin, Origin::Upstream { .. }) {
            continue;
        }
        let mut c = Path::new(w.root).components();
        assert!(
            matches!((c.next(), c.next()), (Some(Component::Normal(_)), None))
                && !w.root.contains(['/', '\\'])
                && !w.root.contains(char::is_whitespace),
            "{}: root {:?} is not one plain path component",
            w.name,
            w.root
        );
        assert!(
            corpus_runner::root_is_plain(w.root),
            "{}: fetch's own check disagrees on {:?}",
            w.name,
            w.root
        );
        roots.push(w.root);
    }
    let n = roots.len();
    roots.sort_unstable();
    roots.dedup();
    assert_eq!(n, roots.len(), "two upstream rows share a root: {roots:?}");
    assert!(n > 0, "no upstream row");
}

/// A sparse directory is one relative path with no `..`, no leading or trailing `/` and
/// no whitespace: `fetch --run` splits each plan line on whitespace into a command's
/// arguments, so a space would turn one directory into two.
fn sparse_dir_is_plain(d: &str) -> bool {
    !d.is_empty()
        && !d.starts_with('/')
        && !d.ends_with('/')
        && !d.contains("..")
        && !d.contains(char::is_whitespace)
}

#[test]
fn a_sparse_directory_must_be_a_plain_relative_path() {
    for ok in ["hw/ip/prim/rtl", "hw", "a-b/c_d.e"] {
        assert!(sparse_dir_is_plain(ok), "{ok:?} should pass");
    }
    for bad in [
        "",
        "/hw/ip",
        "hw/ip/",
        "../hw",
        "hw/../x",
        "hw/ip prim",
        "hw\tip",
        "hw/ip\n",
        " hw",
    ] {
        assert!(!sparse_dir_is_plain(bad), "{bad:?} should be refused");
    }
}

/// A sparse row checks out only the directories it names (`fetch.rs`), so a listed
/// source, a data file or an `-I` directory of the clone that lies outside them would be
/// missing after `fetch --run`: the row would read `absent`, or fail an `include`, on
/// every machine except one that happens to hold the whole tree.
#[test]
fn every_sparse_checkout_covers_what_its_row_reads() {
    use std::path::{Component, Path, PathBuf};
    for w in CORPUS {
        let Origin::Upstream { sparse, .. } = w.origin else {
            continue;
        };
        if sparse.is_empty() {
            continue;
        }
        for d in sparse {
            assert!(
                sparse_dir_is_plain(d),
                "{}: sparse directory {d:?} must be a plain relative path",
                w.name
            );
        }
        // `bench/<dir>/<path>` with `..` resolved, then made relative to the clone.
        let in_clone = |p: &str| -> Option<String> {
            let mut out = PathBuf::new();
            for c in Path::new(w.dir).join(p).components() {
                match c {
                    Component::ParentDir => assert!(out.pop(), "{}: {p} escapes", w.name),
                    Component::CurDir => {}
                    c => out.push(c),
                }
            }
            let s = out.to_string_lossy().replace('\\', "/");
            s.strip_prefix(&format!("{}/src/", w.root))
                .map(str::to_string)
        };
        let mut read: Vec<String> = w
            .files
            .iter()
            .chain(w.data)
            .filter_map(|f| in_clone(f))
            .collect();
        for args in [w.vita_args, w.iverilog_args] {
            for (i, a) in args.iter().enumerate() {
                let dir = match a.strip_prefix("-I") {
                    Some("") => args.get(i + 1).copied(),
                    Some(d) => Some(d),
                    None => None,
                };
                if let Some(d) = dir.and_then(in_clone) {
                    read.push(format!("{d}/"));
                }
            }
        }
        assert!(
            !read.is_empty(),
            "{}: a sparse row that reads nothing from its clone",
            w.name
        );
        for r in &read {
            assert!(
                sparse.iter().any(|d| r.starts_with(&format!("{d}/"))),
                "{}: {r} is outside the sparse checkout {sparse:?}",
                w.name
            );
        }
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

/// `Expect::KnownWrong` is an oracle cell's expectation. A workload whose digest misses
/// is a regression, and `Split` — which names a ruling — is the only way to pin one;
/// pinning a bare wrong digest here would be the licence `Split`'s doc forbids.
#[test]
fn no_workload_pins_a_known_wrong_answer() {
    for w in CORPUS {
        assert!(
            !matches!(w.expect, Expect::KnownWrong { .. }),
            "{}: a workload cannot be pinned known-wrong",
            w.name
        );
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

/// Third-party RTL is never redistributed: every manifest path that is not committed —
/// the clone under `src/`, and what a `prepare.sh` regenerates (`biriscv`'s `prog.hex`)
/// — must be gitignored, so `fetch --run` cannot leave a committable file behind. CI's
/// corpus jobs assert the same thing after a real fetch (`git status --porcelain` must
/// be empty); this is the check one step earlier, without the network.
///
/// It asks git rather than re-implementing `.gitignore` matching, so it needs git and a
/// work tree, and skips with a notice without them (a tarball checkout).
#[test]
fn every_uncommitted_manifest_path_is_gitignored() {
    use std::path::{Component, Path, PathBuf};
    use std::process::Command;

    let root = corpus_runner::resolve_bench_root().expect("repository root");
    let git = |args: &[&str]| Command::new("git").arg("-C").arg(&root).args(args).output();
    match git(&["rev-parse", "--is-inside-work-tree"]) {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("SKIP every_uncommitted_manifest_path_is_gitignored: no git work tree");
            return;
        }
    }

    // `bench/<dir>/<file>`, with `..` resolved: darkriscv's working directory is inside
    // its clone and names the committed testbench as `../../tb2.v`.
    let normalise = |p: PathBuf| -> String {
        let mut out = PathBuf::new();
        for c in p.components() {
            match c {
                Component::ParentDir => assert!(out.pop(), "{} escapes the root", p.display()),
                Component::CurDir => {}
                c => out.push(c),
            }
        }
        out.to_string_lossy().replace('\\', "/")
    };
    let mut paths: Vec<String> = Vec::new();
    for w in CORPUS {
        if let Origin::Upstream { .. } = w.origin {
            // The clone root itself, whether or not a listed file sits in it.
            paths.push(format!("bench/{}/src/", w.root));
        }
        for f in w.files.iter().chain(w.data) {
            paths.push(normalise(Path::new("bench").join(w.dir).join(f)));
        }
    }
    paths.sort();
    paths.dedup();

    // `ok` lists the exit codes that still mean an answer: `check-ignore` exits 1 when
    // nothing is ignored, which the assertion below reports by name. Anything else is
    // git failing, and an empty answer from a failed git must not read as a verdict.
    let paths_from = |cmd: &str, ok: &[i32]| -> Vec<String> {
        let mut args = vec![cmd, "--"];
        args.extend(paths.iter().map(String::as_str));
        let o = git(&args).unwrap_or_else(|e| panic!("git {cmd}: {e}"));
        assert!(
            o.status.code().is_some_and(|c| ok.contains(&c)),
            "git {cmd} failed ({}): {}",
            o.status,
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8(o.stdout)
            .expect("git prints utf-8 paths")
            .lines()
            .map(str::to_string)
            .collect()
    };
    let tracked = paths_from("ls-files", &[0]);
    let ignored = paths_from("check-ignore", &[0, 1]);

    let committable: Vec<&String> = paths
        .iter()
        .filter(|p| !tracked.contains(p) && !ignored.contains(p))
        .collect();
    assert!(
        committable.is_empty(),
        "neither committed nor gitignored, so `fetch --run` would leave them committable: \
         {committable:?}"
    );
}
