//! `corpus-runner` — list, fetch and measure the workload corpus.
//!
//! ```text
//! cargo run -p corpus-runner -- list
//! cargo run -p corpus-runner -- fetch [--run]
//! cargo run -p corpus-runner -- run [--filter <substr>] [--reps N] [--compare]
//! ```

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use corpus_runner::{
    coverage, grade, measure, plan_fetch, resolve_bench_root, Expect, Grade, Origin, Outcome, Tool,
    CORPUS,
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

exit: 0 = every present workload matched  ·  1 = a mismatch or crash
      2 = nothing present (run `fetch` first)  ·  3 = usage";

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
    for s in &steps {
        if s.present {
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
                (Grade::Drifted { got }, _) => format!("expected a different refusal; got {got}"),
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
