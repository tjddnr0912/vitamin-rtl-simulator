//! `corpus-runner` — list, fetch and measure the workload corpus.
//!
//! ```text
//! cargo run -p corpus-runner -- list
//! cargo run -p corpus-runner -- fetch [--run]
//! cargo run -p corpus-runner -- run [--filter <substr>] [--reps N] [--compare]
//! cargo run -p corpus-runner -- cells run [--vita PATH] [--filter <substr>] [--jobs N] [--cells DIR]
//! cargo run -p corpus-runner -- cells pin --vita PATH [--label TEXT] [--jobs N] [--cells DIR] [--rows R,R]
//!                                          [--accept CELL,CELL] [--accept-file FILE]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use corpus_runner::{
    clear_stale, clone_state, coverage, grade, measure, plan_fetch, resolve_bench_root, CloneState,
    Expect, Grade, Origin, Outcome, Tool, CORPUS,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("list");

    let Some(root) = resolve_bench_root() else {
        eprintln!(
            "corpus-runner: could not locate the repository root (no bench/ above this crate)"
        );
        return ExitCode::from(3);
    };

    match cmd {
        "list" => {
            list();
            ExitCode::SUCCESS
        }
        "fetch" => {
            let run = args.iter().any(|a| a == "--run");
            fetch(&root, run)
        }
        "run" => {
            if args.iter().any(|a| a == "--filter") && flag_value(&args, "--filter").is_none() {
                eprintln!("corpus-runner: --filter expects a value");
                return ExitCode::from(3);
            }
            let filter = flag_value(&args, "--filter");
            let reps: usize = match flag_value(&args, "--reps") {
                None => 3,
                Some(v) => match v.parse() {
                    Ok(n) => n,
                    // Silently falling back to the default would report a number
                    // measured differently from the one that was asked for.
                    Err(_) => {
                        eprintln!("corpus-runner: --reps expects a count, got {v:?}");
                        return ExitCode::from(3);
                    }
                },
            };
            let compare = args.iter().any(|a| a == "--compare");
            run_corpus(&root, filter.as_deref(), reps, compare)
        }
        "cells" => cells(&root, &args[1..]),
        "-h" | "--help" | "help" => {
            eprintln!("{}", USAGE);
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("corpus-runner: unknown command {other:?}\n\n{USAGE}");
            ExitCode::from(3)
        }
    }
}

const USAGE: &str = "\
corpus-runner — the vitamin workload corpus

    list                       what the corpus contains, and what is on this machine
    fetch [--run]              show (or perform) the clones the corpus needs
    run [--filter S] [--reps N] [--compare]
                               run each present workload and check its pinned digest
                               --reps N = N TIMED samples (N+1 rounds; the first is
                               discarded as cache warm-up). Default 3.
                               --compare also times iverilog on the same workloads.

    cells run [--vita PATH] [--filter S] [--jobs N] [--cells DIR]
                               grade every oracle cell in crates/testdata/cells/MANIFEST.txt
                               against PATH (default target/release/vita, else debug),
                               two runs per cell; any move from the manifest fails
    cells pin --vita PATH [--label TEXT] [--jobs N] [--cells DIR] [--rows R,R]
                               regenerate MANIFEST.txt and MANIFEST.excluded.tsv: admit
                               the seed rows' cells whose oracles agree, classify each
                               under PATH. TEXT names the binary in the header.
                               --cells DIR reads and writes another cells directory.
                               Every move against the existing manifest is printed; a
                               move in the regression direction is written only when
                               its cell is named with --accept CELL,CELL or
                               --accept-file FILE (one path per line); otherwise
                               nothing is written and the exit is 1

exit: 0 = every present workload matched  ·  1 = a mismatch or crash
      2 = nothing present (run `fetch` first)  ·  3 = usage";

/// `cells run` / `cells pin`.
fn cells(root: &std::path::Path, args: &[String]) -> ExitCode {
    use corpus_runner::cells;
    let sub = args.first().map(String::as_str).unwrap_or("");
    for f in [
        "--vita",
        "--filter",
        "--jobs",
        "--label",
        "--cells",
        "--rows",
        "--accept",
        "--accept-file",
    ] {
        if args.iter().any(|a| a == f) && flag_value(args, f).is_none() {
            eprintln!("corpus-runner: {f} expects a value");
            return ExitCode::from(3);
        }
    }
    let jobs = match flag_value(args, "--jobs") {
        None => cells::default_jobs(),
        Some(v) => match v.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                eprintln!("corpus-runner: --jobs expects a positive count, got {v:?}");
                return ExitCode::from(3);
            }
        },
    };
    let dir = flag_value(args, "--cells").map_or_else(|| cells::cells_dir(root), PathBuf::from);
    // Absolute: each cell runs with its own working directory, where a relative
    // `./target/…` would name nothing and every row would grade as a crash.
    let vita = flag_value(args, "--vita").map(|v| {
        let p = PathBuf::from(v);
        std::fs::canonicalize(&p).unwrap_or(p)
    });
    match sub {
        "run" => {
            let vita = vita.unwrap_or_else(|| vita_binary(root));
            if !vita.is_file() {
                eprintln!("corpus-runner: no vita binary at {}", vita.display());
                return ExitCode::from(3);
            }
            let all = match cells::load_manifest(&dir) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("corpus-runner: {e}");
                    return ExitCode::from(2);
                }
            };
            let filter = flag_value(args, "--filter");
            let selected: Vec<&cells::Cell> = all
                .iter()
                .filter(|c| filter.as_deref().is_none_or(|f| c.path.contains(f)))
                .collect();
            if selected.is_empty() {
                eprintln!("corpus-runner: no cell matched");
                return ExitCode::from(2);
            }
            // The cells are committed, not fetched: a missing source is a broken
            // checkout, never an `absent` row.
            if let Some(c) = selected.iter().find(|c| !dir.join(&c.path).is_file()) {
                eprintln!(
                    "corpus-runner: the manifest names {} and it is not here",
                    c.path
                );
                return ExitCode::from(2);
            }
            let t0 = std::time::Instant::now();
            let graded = cells::run_manifest(&dir, &selected, &vita, jobs, cells::BUDGET);
            let failures = cells::print_table(&selected, &graded);
            println!(
                "wall {:.1}s · {} · {jobs} jobs",
                t0.elapsed().as_secs_f64(),
                vita.display()
            );
            if failures > 0 {
                eprintln!("\ncorpus-runner: {failures} failing");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        "pin" => {
            let Some(vita) = vita.filter(|v| v.is_file()) else {
                eprintln!("corpus-runner: cells pin needs --vita PATH, an existing binary");
                return ExitCode::from(3);
            };
            let label = flag_value(args, "--label").unwrap_or_else(|| "unlabelled".into());
            let rows_arg = flag_value(args, "--rows");
            let rows: Vec<&str> = match &rows_arg {
                Some(r) => r.split(',').filter(|r| !r.is_empty()).collect(),
                None => cells::SEED_ROWS.to_vec(),
            };
            let pinned = match cells::pin(&dir, &rows, &vita, jobs) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("corpus-runner: {e}");
                    return ExitCode::from(1);
                }
            };
            // The guard: compare with the manifest already there before writing.
            let mut accept: std::collections::BTreeSet<String> = flag_value(args, "--accept")
                .map(|a| {
                    a.split(',')
                        .filter(|c| !c.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            if let Some(f) = flag_value(args, "--accept-file") {
                match std::fs::read_to_string(&f) {
                    Ok(t) => accept.extend(cells::guard::parse_accept_file(&t)),
                    Err(e) => {
                        eprintln!("corpus-runner: --accept-file {f}: {e}");
                        return ExitCode::from(3);
                    }
                }
            }
            if dir.join(cells::MANIFEST_FILE).exists() {
                let old =
                    cells::load_manifest(&dir).and_then(|c| Ok((c, cells::load_excluded(&dir)?)));
                let (old_cells, old_excluded) = match old {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!("corpus-runner: the existing manifest does not read ({e}); nothing written");
                        return ExitCode::from(1);
                    }
                };
                let moves = cells::guard::compare(&old_cells, &old_excluded, &pinned);
                cells::guard::print_moves(&moves, &accept);
                let blocked = cells::guard::blocked(&moves, &accept);
                if !blocked.is_empty() {
                    eprintln!(
                        "corpus-runner: {} regression-direction moves are not named; nothing written. \
                         Name each with --accept CELL,CELL or --accept-file FILE.",
                        blocked.len()
                    );
                    return ExitCode::from(1);
                }
            } else {
                println!("pin: no existing manifest; writing the first one");
            }
            if let Err(e) = cells::write_pinned(&dir, &label, &rows, &pinned) {
                eprintln!("corpus-runner: {e}");
                return ExitCode::from(1);
            }
            for line in cells::header(&label, &rows, &pinned).iter().skip(3) {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("corpus-runner: cells expects `run` or `pin`, got {other:?}\n\n{USAGE}");
            ExitCode::from(3)
        }
    }
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).cloned()
}

fn list() {
    println!(
        "{:<18} {:<7} {:<9} {:<12} {:<9} note",
        "workload", "shape", "origin", "licence", "vita"
    );
    for w in CORPUS {
        let (origin, lic) = match w.origin {
            Origin::FirstParty => ("in-repo", "ours"),
            Origin::Upstream { license, .. } => ("upstream", license),
        };
        let state = match w.expect {
            Expect::Runs { .. } => "runs",
            Expect::Refused { .. } => "refused",
            Expect::Split { .. } => "split",
            Expect::KnownWrong { .. } => "known-wrong",
        };
        println!(
            "{:<18} {:<7} {:<9} {lic:<12} {state:<9} {}",
            w.name,
            w.shape.label(),
            origin,
            w.note
        );
    }
    if CORPUS.is_empty() {
        println!("(empty)");
        return;
    }
    let (runs, total) = coverage();
    println!("\ncoverage: {runs}/{total} run under vita");
    for w in CORPUS {
        match w.expect {
            Expect::Refused { diag } => println!("  {:<18} refused: {diag}", w.name),
            Expect::Split { why, .. } => println!("  {:<18} ruled split: {why}", w.name),
            Expect::KnownWrong { vita, .. } => println!("  {:<18} known-wrong: {vita}", w.name),
            Expect::Runs { .. } => {}
        }
    }
}

fn fetch(root: &std::path::Path, execute: bool) -> ExitCode {
    let steps = plan_fetch(root);
    if steps.is_empty() {
        println!("nothing to fetch — the corpus is entirely first-party");
        return ExitCode::SUCCESS;
    }
    // A stale clone that may not be removed (local changes, an unreadable status) fails
    // its own row; the other rows are still fetched, and the exit is 1 at the end.
    let mut refused = 0usize;
    for s in &steps {
        if let CloneState::Stale(why) = &s.state {
            println!(
                "# {} — {} is not the pinned checkout ({why}); `fetch --run` removes it, unless \
                 it holds local changes, and clones again",
                s.name, s.dest
            );
            if execute {
                let w = CORPUS
                    .iter()
                    .find(|w| w.name == s.name)
                    .expect("every planned row comes from the manifest");
                match clear_stale(root, w, s.sha) {
                    Ok(true) => println!("#   removed {}", s.dest),
                    // Measured again inside `clear_stale`: no longer stale. A clone that
                    // is now the pinned one is left; one that vanished is cloned below.
                    Ok(false) => {
                        if clone_state(root, w, s.sha) == CloneState::Present {
                            println!("#   {} is no longer stale; left as it is", s.dest);
                            continue;
                        }
                    }
                    Err(e) => {
                        eprintln!("corpus-runner: {}: {e}", s.name);
                        refused += 1;
                        continue;
                    }
                }
            }
        }
        if s.state == CloneState::Present {
            println!("# {} — already present at {}", s.name, s.dest);
            // The prepare step still runs: it regenerates artifacts that are
            // deliberately not committed, and it is idempotent.
            if let (true, Some(prep)) = (execute, &s.prepare) {
                let st = std::process::Command::new("sh")
                    .arg(prep)
                    .current_dir(root)
                    .status();
                if !matches!(st, Ok(x) if x.success()) {
                    eprintln!("corpus-runner: {prep} failed");
                    return ExitCode::from(1);
                }
                println!("#   re-ran {prep}");
            }
            continue;
        }
        println!("# {} ({})", s.name, s.license);
        println!("{}\n", s.script());
        if execute {
            for line in s.script().lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let status = std::process::Command::new(parts[0])
                    .args(&parts[1..])
                    .current_dir(root)
                    .status();
                match status {
                    Ok(st) if st.success() => {}
                    Ok(st) => {
                        eprintln!("corpus-runner: `{line}` exited {st}");
                        return ExitCode::from(1);
                    }
                    Err(e) => {
                        eprintln!("corpus-runner: `{line}` failed: {e}");
                        return ExitCode::from(1);
                    }
                }
            }
        }
    }
    if !execute {
        println!("# nothing was run — re-invoke with `fetch --run` to perform these clones");
    }
    if refused > 0 {
        eprintln!(
            "corpus-runner: {refused} stale clone(s) left in place; nothing was removed from them"
        );
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn vita_binary(root: &std::path::Path) -> PathBuf {
    // Prefer the release binary: measuring a debug build once cost this project a
    // whole review round on a fabricated +88% regression.
    let rel = root.join("target/release/vita");
    if rel.is_file() {
        rel
    } else {
        root.join("target/debug/vita")
    }
}

fn run_corpus(
    root: &std::path::Path,
    filter: Option<&str>,
    reps: usize,
    compare: bool,
) -> ExitCode {
    let vita = vita_binary(root);
    if !vita.is_file() {
        eprintln!(
            "corpus-runner: no vita binary at {} — run `cargo build --release` first",
            vita.display()
        );
        return ExitCode::from(3);
    }
    if vita.ends_with("target/debug/vita") {
        eprintln!("corpus-runner: WARNING measuring a debug binary; timings are not comparable");
    }

    let selected: Vec<&'static corpus_runner::Workload> = CORPUS
        .iter()
        .filter(|w| filter.is_none_or(|f| w.name.contains(f)))
        .collect();

    if selected.is_empty() {
        eprintln!("corpus-runner: no workload matched");
        return ExitCode::from(2);
    }

    let mut jobs = Vec::new();
    for w in &selected {
        if let Some(j) = corpus_runner::job_for(root, w, Tool::Vita, &vita) {
            jobs.push(j);
        }
        if compare {
            match corpus_runner::prepare_iverilog(root, w) {
                Ok(()) => {
                    if let Some(j) = corpus_runner::job_for(root, w, Tool::Iverilog, &vita) {
                        jobs.push(j);
                    }
                }
                // Not a failure of the corpus: the comparison is a convenience, and a
                // machine without iverilog still gates on the pinned digest.
                Err(e) => eprintln!("corpus-runner: no iverilog comparison for {}: {e}", w.name),
            }
        }
    }

    let results = measure(&jobs, reps, Duration::from_secs(600));

    println!(
        "{:<18} {:<10} {:<11} {:>9}  detail",
        "workload", "tool", "grade", "median"
    );
    let mut failures = 0usize;
    let mut present = 0usize;
    let mut promoted = Vec::new();
    for m in &results {
        let w = CORPUS
            .iter()
            .find(|w| w.name == m.workload)
            .expect("job came from CORPUS");
        let g = grade(w, m.tool, &m.outcome);
        if m.outcome != Outcome::Absent {
            present += 1;
        }
        if g.is_failure() {
            failures += 1;
        }
        if g == Grade::Promoted && m.tool == Tool::Vita {
            promoted.push(w.name);
        }
        let med = m
            .median_secs
            .map(|s| format!("{s:.3}s"))
            .unwrap_or_else(|| "-".into());
        // Checked before the grade: two distinct digests from one tool is a bigger
        // fact than whichever of them the last round happened to produce, and
        // guarding this behind `Grade::Ok` made it unreachable — a mismatch retires
        // the job, so a flapping tool was reported as a deterministically wrong one.
        let detail = if m.is_nondeterministic() {
            format!("*** NON-DETERMINISTIC *** {}", m.digests.join("  |  "))
        } else {
            match (&g, &m.outcome) {
                (Grade::Regression(why), _) => why.clone(),
                (Grade::Drifted { got }, _) => match w.expect {
                    Expect::KnownWrong { .. } => format!("the pinned wrong answer moved: {got}"),
                    _ => format!("expected a different refusal; got {got}"),
                },
                (Grade::Promoted, _) => "now runs — move its manifest row to Expect::Runs".into(),
                (Grade::RuledSplit, _) => match w.expect {
                    Expect::Split { why, .. } => format!("ruled split — {why}"),
                    _ => String::new(),
                },
                (Grade::KnownGap, Outcome::Refused { diag }) => diag.clone(),
                (Grade::OracleDrifted { got }, _) => {
                    format!("the ORACLE no longer reproduces the pin: {got}")
                }
                (Grade::Absent, _) => "fetch first".into(),
                _ => String::new(),
            }
        };
        println!(
            "{:<18} {:<10} {:<11} {med:>9}  {detail}",
            m.workload,
            m.tool.label(),
            g.label()
        );
    }

    // The front-end / executor split, one line per vita row that produced it.
    // Printed BELOW the table rather than as a column: the table is fixed-width
    // and read by column offset, and a new column would move every consumer's
    // parse. A regression in either phase is invisible in the median wall time,
    // which is what the external report had to reconstruct by hand.
    let phase_rows: Vec<_> = results
        .iter()
        .filter(|m| m.tool == Tool::Vita)
        .filter_map(|m| m.phases.map(|p| (m.workload, p)))
        .collect();
    if !phase_rows.is_empty() {
        println!("\nphase split (vita, one probe run each — not the timed rounds)");
        for (name, p) in phase_rows {
            let total = p.elab_s + p.sim_s;
            let pct = if total > 0.0 {
                100.0 * p.elab_s / total
            } else {
                0.0
            };
            println!(
                "{name:<18} elab {:.3}s  sim {:.3}s  ({pct:.0}% front end)",
                p.elab_s, p.sim_s
            );
        }
    }

    // The vita/iverilog ratio is the number the performance track is actually
    // steering by, so compute it here rather than leaving it to be eyeballed.
    if compare {
        println!();
        for w in &selected {
            let f = |t: Tool| {
                results
                    .iter()
                    .find(|m| m.workload == w.name && m.tool == t)
                    .and_then(|m| m.median_secs)
            };
            if let (Some(v), Some(i)) = (f(Tool::Vita), f(Tool::Iverilog)) {
                let ratio = i / v;
                let verdict = if ratio >= 1.0 { "faster" } else { "SLOWER" };
                println!(
                    "{:<18} vita {v:.3}s  iverilog {i:.3}s  = {ratio:.2}x {verdict}",
                    w.name
                );
            }
        }
    }

    for name in &promoted {
        println!("\ncorpus-runner: {name} now runs under vita — update its manifest row.");
    }
    let (runs, total) = coverage();
    println!("\ncoverage: {runs}/{total} of the corpus runs under vita");

    if present == 0 {
        eprintln!("\ncorpus-runner: no workload is present on this machine — run `corpus-runner fetch --run`");
        return ExitCode::from(2);
    }
    if failures > 0 {
        eprintln!("\ncorpus-runner: {failures} failing");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
