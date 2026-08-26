//! R2 (round-35 external report): **a flag absent from `--help` does not exist.**
//!
//! This class of bug has now cost two external reports. The first spent six days
//! not finding `--obs-dir`, which prompted the OBSERVABILITY block in
//! `pipeline.rs::print_help` and the comment above it. That block then did not
//! keep up: `--obs-procs` / `--obs-procs-time` (R14) and `--probe-file` (OBS-2)
//! shipped working, and documented in `docs/manual/004_cli-reference.md`, but
//! never reached the help. A round-35 reporter therefore saw `"processes": null`
//! in `run.json`, concluded per-process observability was UNIMPLEMENTED, and
//! only found the flags by digging through the manual.
//!
//! A test that greps for the two literal strings just added would not have
//! caught either occurrence, so this one ENUMERATES instead: it parses the flag
//! literals out of the arg parser's own match arms and asserts each one appears
//! in `vita --help`. A new flag arm is covered the moment it is written.
//!
//! ## What is enumerated, and the residue
//!
//! Covered automatically: every match arm in `stage_args.rs` (`parse_io_args` —
//! the single argv parser all four applets share) and in `filelist.rs` whose
//! pattern is nothing but string literals, e.g. `"-o" | "--out" => …`. That is
//! every exactly-spelled flag.
//!
//! NOT enumerable from arm patterns, because they are guard arms rather than
//! literals (`s if s.starts_with("-D")`): the attached-value and multi-value
//! spellings `-D<N>`, `-I<dir>`, `-G<N>=<V>`, `+define+`, `+incdir+`, bare
//! `+plusarg`, and the diagnostic gate's `-Wno-<CODE>` / `-Werror[=<CODE>]`.
//! Those are pinned by hand in [`pattern_arm_prefixes`] — the pin is over the
//! prefixes the PARSER tests, so adding a new guard arm fails this test and
//! makes the author decide what `--help` should say about it.
//!
//! Also outside the parser and therefore hand-listed: `-h`/`--help` and
//! `-V`/`--version`, which `pipeline.rs` answers before `parse_io_args` runs.

use std::collections::BTreeSet;
use std::process::Command;

/// The two files that decide whether an argv token is a known flag.
/// `stage_args.rs` is the parser proper; `filelist.rs` consumes `-f`/`-F`
/// ahead of it.
const STAGE_ARGS: &str = include_str!("../src/stage_args.rs");
const FILELIST: &str = include_str!("../src/filelist.rs");

/// Flags spelled as literal match-arm patterns, e.g. `"-q" | "--quiet" => {`.
///
/// An arm qualifies only when EVERY `|` alternative is a bare string literal,
/// which is what keeps `Some("interp") | Some("vm") => …` (a `--backend` VALUE,
/// not a flag) and `Some(Ok(())) => …` out. Tokens not starting with `-` are
/// dropped: those are subcommand names and filelist tokens, not flags.
fn literal_arm_flags(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in src.lines() {
        let Some((head, _)) = line.trim().split_once("=>") else {
            continue;
        };
        let head = head.trim();
        if head.is_empty() {
            continue;
        }
        let mut lits = Vec::new();
        for alt in head.split('|') {
            let a = alt.trim();
            let inner = a
                .strip_prefix('"')
                .and_then(|r| r.strip_suffix('"'))
                .filter(|i| !i.contains('"'));
            match inner {
                Some(i) => lits.push(i.to_string()),
                // Not a bare literal ⇒ not a flag arm; abandon the whole line.
                None => {
                    lits.clear();
                    break;
                }
            }
        }
        out.extend(lits.into_iter().filter(|s| s.starts_with('-')));
    }
    out
}

/// The guard-arm prefixes `parse_io_args` matches with `starts_with`, pinned by
/// hand because a guard has no literal pattern to enumerate. Keep in sync with
/// `stage_args.rs`; [`pattern_arms_are_still_the_pinned_set`] enforces that.
fn pattern_arm_prefixes() -> BTreeSet<String> {
    ["+define+", "+incdir+", "+", "-D", "-I", "-G", "-"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Every `starts_with("…")`/`starts_with('…')` guard arm actually present in the
/// parser, as the prefix each one tests.
fn guard_arm_prefixes(src: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in src.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("s if s.starts_with(") else {
            continue;
        };
        let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            panic!("guard arm with a non-literal prefix, extend this test: {t}");
        };
        let body = &rest[q.len_utf8()..];
        let end = body
            .find(q)
            .unwrap_or_else(|| panic!("unterminated literal in guard arm: {t}"));
        found.insert(body[..end].to_string());
    }
    found
}

fn help_text() -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--help")
        .output()
        .expect("run vita --help");
    assert_eq!(out.status.code(), Some(0));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// All flags this test holds `--help` responsible for.
fn accepted_flags() -> BTreeSet<String> {
    let mut flags = literal_arm_flags(STAGE_ARGS);
    flags.extend(literal_arm_flags(FILELIST));
    // Answered in `pipeline.rs` before `parse_io_args` ever sees argv.
    flags.extend(
        ["-h", "--help", "-V", "--version"]
            .iter()
            .map(|s| s.to_string()),
    );
    flags
}

/// `hay` mentions `flag` as a whole token. A plain `contains` would let
/// `--obs-procs` be "documented" by the words "--obs-procs-time", and `--work`
/// by "--workdir" — i.e. it would call the bug this file exists for a pass.
fn mentions_flag(hay: &str, flag: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(rel) = hay[from..].find(flag) {
        let start = from + rel;
        let end = start + flag.len();
        let before_ok = start == 0 || {
            let c = bytes[start - 1] as char;
            !(c.is_ascii_alphanumeric() || c == '-' || c == '+' || c == '_')
        };
        let after_ok = end == bytes.len() || {
            let c = bytes[end] as char;
            !(c.is_ascii_alphanumeric() || c == '-' || c == '_')
        };
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// The enumeration itself must not be able to go quiet. If a refactor changes
/// the arm shape (a `match` replaced by an `if` chain, say), `literal_arm_flags`
/// would return an empty set and every other assertion in this file would pass
/// vacuously — the exact failure mode of a green test that measures nothing.
#[test]
fn the_flag_enumeration_is_not_vacuous() {
    let flags = literal_arm_flags(STAGE_ARGS);
    assert!(
        flags.len() >= 30,
        "parsed only {} flag literals out of stage_args.rs — the extractor has \
         gone stale against the parser's shape, so every assertion built on it \
         is vacuous: {flags:?}",
        flags.len()
    );
    // Spot anchors across the parser's whole span: cheap insurance that the
    // extractor did not stop early.
    for f in [
        "-o",
        "--backend",
        "--obs-dir",
        "--obs-procs",
        "--dump-filelist",
    ] {
        assert!(flags.contains(f), "extractor missed {f}: {flags:?}");
    }
    assert!(literal_arm_flags(FILELIST).contains("-f"));
    // And the token matcher must be able to say NO, or the main test is a
    // tautology over any non-empty help text.
    assert!(mentions_flag("  --obs-procs   add", "--obs-procs"));
    assert!(!mentions_flag("only --obs-procs-time here", "--obs-procs"));
    assert!(!mentions_flag("only --workdir here", "--work"));
}

/// The extracted literals are really flags the binary ACCEPTS — the source parse
/// and the shipped parser cannot drift apart silently.
#[test]
fn every_extracted_flag_is_accepted_by_the_binary() {
    let mut flags = literal_arm_flags(STAGE_ARGS);
    flags.extend(literal_arm_flags(FILELIST));
    for f in &flags {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(f)
            .output()
            .expect("run vita");
        let err = String::from_utf8_lossy(&out.stderr);
        // The run fails (no sources, missing flag value, …) — only the
        // "unknown flag" verdict would mean the extractor invented a flag.
        assert!(
            !err.contains(&format!("unknown flag '{f}'")),
            "{f} was extracted as a flag but the binary rejects it as unknown:\n{err}"
        );
    }
}

/// THE regression test: every flag the parser accepts appears in `--help`.
#[test]
fn help_documents_every_flag_the_parser_accepts() {
    let help = help_text();
    let flags = accepted_flags();
    let missing: Vec<&String> = flags.iter().filter(|f| !mentions_flag(&help, f)).collect();
    assert!(
        missing.is_empty(),
        "these flags are ACCEPTED by the CLI but absent from `vita --help`: \
         {missing:?}\n\nA flag absent from --help does not exist — that is how \
         two external reports lost days each. Add them to `print_help` in \
         crates/cli/src/pipeline.rs.\n\n--- help ---\n{help}"
    );
}

/// The staged applets share one `Options` block with `vita`, and they share the
/// parser too, so the same obligation holds for each of them. (The per-applet
/// difference is which flags are then REJECTED as out-of-stage, e.g.
/// `reject_obs_dir` — a rejection whose message names a flag the help never
/// mentioned would be the same dead end.)
#[test]
fn staged_applet_help_documents_the_same_flags() {
    let flags = accepted_flags();
    for applet in ["vcmp", "velab", "vrun"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args([applet, "--help"])
            .output()
            .expect("run vita <applet> --help");
        assert_eq!(out.status.code(), Some(0));
        let help = String::from_utf8_lossy(&out.stdout);
        let missing: Vec<&String> = flags.iter().filter(|f| !mentions_flag(&help, f)).collect();
        assert!(
            missing.is_empty(),
            "`vita {applet} --help` is missing accepted flags: {missing:?}"
        );
    }
}

/// The guard arms have no literal to enumerate, so pin the prefix set: adding a
/// new `s if s.starts_with("…")` arm fails here, which is the prompt to decide
/// what `--help` says about the new spelling.
#[test]
fn pattern_arms_are_still_the_pinned_set() {
    assert_eq!(
        guard_arm_prefixes(STAGE_ARGS),
        pattern_arm_prefixes(),
        "the set of `starts_with` guard arms in stage_args.rs changed. These \
         spellings cannot be enumerated from a pattern, so they are pinned here \
         AND must be described in `vita --help`."
    );
    // The prefixed spellings are described in the Options block by their
    // documented form, not as bare prefixes; assert that wording is still there.
    // (`-D`/`-I`/`-G` are covered as whole tokens by the main test above.)
    let help = help_text();
    for phrase in ["+define+", "+incdir+", "-Wno-", "-Werror"] {
        assert!(
            help.contains(phrase),
            "`{phrase}` is accepted by the parser but unmentioned in --help"
        );
    }
}
