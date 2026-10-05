//! Running a workload and timing it honestly.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::{Expect, Workload};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Vita,
    Iverilog,
    Verilator,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Vita => "vita",
            Tool::Iverilog => "iverilog",
            Tool::Verilator => "verilator",
        }
    }
}

/// What one run of one workload under one tool did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Printed the pinned digest.
    Match,
    /// Ran to completion and printed a *different* digest. This is the finding the
    /// corpus exists to produce: the design is real, the oracle answered, and we
    /// disagree with it.
    Mismatch { got: String },
    /// Declined the design and said why. Honest, and a promotion candidate.
    Refused { diag: String },
    /// Exited non-zero without a diagnostic, or panicked.
    Crashed { code: i32, tail: String },
    /// Exceeded the wall-clock budget and was killed.
    Timeout,
    /// The RTL is not on this machine. Run `corpus-runner fetch` first.
    Absent,
}

impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Match => "match",
            Outcome::Mismatch { .. } => "MISMATCH",
            Outcome::Refused { .. } => "refused",
            Outcome::Crashed { .. } => "CRASH",
            Outcome::Timeout => "TIMEOUT",
            Outcome::Absent => "absent",
        }
    }

    /// Only a mismatch or a crash is a *failure*. A refusal is the correct-or-loud
    /// ladder working as designed, and `absent` means the machine has not fetched
    /// the corpus — neither should turn a gate red.
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Mismatch { .. } | Outcome::Crashed { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub workload: &'static str,
    pub tool: Tool,
    pub outcome: Outcome,
    /// Median wall time of the timed runs, in seconds. `None` if nothing completed.
    pub median_secs: Option<f64>,
    /// Every timed run, in order, so a reader can see the spread rather than trust
    /// a single number.
    pub secs: Vec<f64>,
    /// Distinct digest lines observed. More than one means the tool is not
    /// deterministic on this design, which outranks any timing it produced.
    pub digests: Vec<String>,
    /// vita only: `(elab_s, sim_s)` from a SEPARATE probe run — see
    /// [`probe_phases`]. `None` for iverilog, for a workload that did not match,
    /// and on any machine where the probe could not be read.
    pub phases: Option<Phases>,
}

/// The front-end / executor split of one vita run, from `run.json`.
///
/// WHY THIS EXISTS. An external report measured a 39% simulation SPEEDUP and a
/// 22% elaboration SLOWDOWN across the same 51 commits and could see both only
/// because it read `run.json` by hand; this harness reported one median wall
/// time, in which the two cancel. A regression that a gate cannot see is a
/// regression that ships — so the split is a column here, not an anecdote.
///
/// ⚠️ ONE SAMPLE, and a separate run from the timed rounds. Folding `--obs-dir`
/// into the timed command would have added the file writes to every vita wall
/// time and made this harness's headline number incomparable with the history in
/// `docs/study/03-workload-corpus.md`. `elab_s`/`sim_s` are internal timers, so
/// the probe measures the same phases the timed rounds ran; what it does not give
/// is a spread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phases {
    pub elab_s: f64,
    pub sim_s: f64,
}

impl Measurement {
    pub fn is_nondeterministic(&self) -> bool {
        self.digests.len() > 1
    }
}

/// Walk up from this crate to the repository root (the directory holding `bench/`).
pub fn resolve_bench_root() -> Option<PathBuf> {
    let mut d: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    loop {
        if d.join("bench").is_dir() && d.join("Cargo.lock").is_file() {
            return Some(d);
        }
        if !d.pop() {
            return None;
        }
    }
}

/// The digest a run printed, scanned from STDOUT only.
///
/// `DIGEST=` is the corpus contract; ` acc=` is `bench/keccak`, whose testbench
/// predates it and stays readable as it is. Scanned in reverse so a workload may
/// print progress before its digest — the contract is that the digest is last.
fn digest_line(out: &str) -> Option<String> {
    out.lines()
        .rev()
        .find(|l| l.contains("DIGEST=") || l.contains(" acc="))
        .map(|l| l.trim().to_string())
}

/// The refusal to report for a run that produced no usable digest.
///
/// The PINNED diagnostic wins if it appears anywhere in the output, because that is
/// what the grade is about: `verilog-ethernet` emits 24 warnings before its pinned
/// error, and `verilog-axi` emits 54 errors of which the pinned one is merely the
/// first today. Grading on whichever diagnostic came first makes the verdict a
/// function of emission order, so a harmless reordering would read as a drift.
///
/// Only when the pin is absent does this fall back to the first diagnostic line —
/// and then it is reporting a drift, so showing what actually came out is the point.
fn refusal(err: &str, w: &Workload) -> Option<String> {
    if let crate::Expect::Refused { diag } = w.expect {
        if let Some(line) = err.lines().find(|l| l.contains(diag)) {
            return Some(line.trim().to_string());
        }
    }
    err.lines()
        .find(|l| l.contains("error[") || l.contains("error:"))
        .map(|l| l.trim().to_string())
}

/// The last few lines of a run that produced neither a digest nor a diagnostic.
fn tail_of(out: &str, err: &str) -> String {
    let mut lines: Vec<&str> = err.lines().rev().take(3).collect();
    if lines.is_empty() {
        lines = out.lines().rev().take(3).collect();
    }
    lines.join(" / ")
}

/// What one bounded run produced. stdout and stderr are kept apart on purpose: the
/// digest belongs to stdout and diagnostics to stderr, and concatenating them lets a
/// future stderr line containing `DIGEST=` outrank the real one.
pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub secs: f64,
    pub timed_out: bool,
}

/// Run one command with a hard wall-clock budget. macOS ships no `timeout(1)`, and a
/// testbench whose `$finish` never fires would otherwise wedge the harness forever.
///
/// Both pipes are drained by their own threads for the whole life of the child. The
/// obvious shape — poll `try_wait`, then `wait_with_output` — deadlocks as soon as a
/// workload outsprints the pipe buffer: the child blocks in `write`, so it never
/// exits, so the parent never reads, so it never exits either, and the run is
/// eventually killed at the budget and reported as a HANG. That is a slander, not a
/// diagnosis. It is not hypothetical here: `verilog-axi` already writes 19,238 bytes
/// of diagnostics from a 2x2 crossbar, and macOS starts a pipe at 16 KiB.
pub(crate) fn run_bounded(cmd: &mut Command, budget: Duration) -> Run {
    let t0 = Instant::now();
    let mut child = match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => {
            return Run {
                code: None,
                stdout: String::new(),
                stderr: format!("spawn failed: {e}"),
                secs: 0.0,
                timed_out: false,
            };
        }
    };

    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });

    let (code, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (status.code(), false),
            Ok(None) => {
                if t0.elapsed() > budget {
                    let _ = child.kill();
                    let _ = child.wait();
                    break (None, true);
                }
                // 2 ms, not 20: the poll interval is a floor on how late exit is
                // observed, and the reported times are quoted to the millisecond.
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => {
                return Run {
                    code: None,
                    stdout: String::new(),
                    stderr: format!("wait failed: {e}"),
                    secs: t0.elapsed().as_secs_f64(),
                    timed_out: false,
                };
            }
        }
    };
    let secs = t0.elapsed().as_secs_f64();

    Run {
        code,
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
        secs,
        timed_out,
    }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

/// One (workload, tool) pair to measure.
pub struct Job {
    pub workload: &'static Workload,
    pub tool: Tool,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

impl Job {
    /// The first source this machine does not have, if any.
    pub fn missing_source(&self) -> Option<PathBuf> {
        if !self.cwd.is_dir() {
            return Some(self.cwd.clone());
        }
        self.workload
            .files
            .iter()
            .chain(self.workload.data.iter())
            .map(|f| self.cwd.join(f))
            .find(|p| !p.is_file())
    }
}

/// Measure every job, **round-robin**, discarding the first round.
///
/// `reps` is the number of TIMED samples, so `reps + 1` rounds actually run. Saying
/// it the other way round — `reps` rounds, one discarded — reads as three samples and
/// delivers two, which is a mean wearing the word "median".
///
/// The interleaving is not a style preference. Timing all of A and then all of B on
/// this project once produced a fake +12.5% purely from cache and thermal state; the
/// first round is discarded for the same reason. Passing the whole set at once is
/// what makes the round-robin the default shape — though it does not make sequential
/// measurement impossible, since a caller can still invoke this once per job.
pub fn measure(jobs: &[Job], reps: usize, budget: Duration) -> Vec<Measurement> {
    let mut acc: Vec<Measurement> = jobs
        .iter()
        .map(|j| Measurement {
            workload: j.workload.name,
            tool: j.tool,
            outcome: Outcome::Absent,
            median_secs: None,
            secs: Vec::new(),
            digests: Vec::new(),
            phases: None,
        })
        .collect();

    // A job that has already failed is not re-run: repeating it cannot change the
    // verdict, and it would delay every job still being timed behind it.
    let mut retired = vec![false; jobs.len()];

    for round in 0..=reps.max(1) {
        for (i, j) in jobs.iter().enumerate() {
            if retired[i] {
                continue;
            }
            // Presence is a property of the SOURCES, not of the directory.
            // `fetch` creates `bench/<root>/src`, which makes `bench/<root>` exist —
            // so a directory check reports "present" on a machine that has the
            // upstream RTL and none of the harness, and every row then grades as a
            // vita regression for a missing file. That is the worst kind of red: it
            // is loud, it is confident, and it names the wrong culprit.
            if j.missing_source().is_some() {
                acc[i].outcome = Outcome::Absent;
                retired[i] = true;
                continue;
            }
            let mut cmd = Command::new(&j.program);
            cmd.args(&j.args).current_dir(&j.cwd);
            let r = run_bounded(&mut cmd, budget);

            let outcome = outcome_of(j.tool, j.workload, &r);
            if let Outcome::Match | Outcome::Mismatch { .. } = outcome {
                if let Some(d) = digest_line(&r.stdout) {
                    if !acc[i].digests.contains(&d) {
                        acc[i].digests.push(d);
                    }
                }
            }
            let secs = r.secs;

            // A run that did not produce the expected digest is not worth timing, and
            // timing it would put a meaningless number next to a real failure.
            let timed = matches!(outcome, Outcome::Match);
            acc[i].outcome = outcome;
            if timed && round > 0 {
                acc[i].secs.push(secs);
            }
            retired[i] = !timed;
        }
    }

    for (i, m) in acc.iter_mut().enumerate() {
        m.median_secs = median(m.secs.clone());
        // Only a vita row that actually matched: the probe re-runs the design, and
        // re-running a workload that refused or hung buys nothing but the wait.
        if jobs[i].tool == Tool::Vita && m.outcome == Outcome::Match {
            m.phases = probe_phases(&jobs[i], budget);
        }
    }
    acc
}

/// What one finished run means, read against the manifest. Pure, so the grading edges
/// are testable without spawning a simulator.
fn outcome_of(tool: Tool, w: &Workload, r: &Run) -> Outcome {
    if r.timed_out {
        return Outcome::Timeout;
    }
    let want = expected_exit(tool, w);
    match (r.code, digest_line(&r.stdout)) {
        (Some(c), Some(d)) if c == want && d == w.digest => Outcome::Match,
        (Some(c), Some(d)) if c == want => Outcome::Mismatch { got: d },
        // Right answer, wrong exit code. Not a mismatch — the simulation was correct —
        // but not a clean pass either, and saying so is the whole reason the expected
        // code is pinned.
        (Some(c), Some(d)) if d == w.digest => Outcome::Crashed {
            code: c,
            tail: format!("digest correct but exit {c}, expected {want}"),
        },
        // The pinned refusal is matched against the WHOLE of stderr, not against
        // whichever diagnostic happens to come first: vita emits 24 warnings before the
        // pinned error on `verilog-ethernet`, and grading on emission order would call a
        // reordering a drift.
        //
        // vita declines a design with exit 1. Any other code — a panic's 101, a signal,
        // the stale-artifact 2 — is a crash even when the pinned diagnostic was printed
        // before it; reading the text alone graded such a crash `known-gap`.
        (code, _) => match refusal(&r.stderr, w) {
            Some(diag) if tool != Tool::Vita || code == Some(1) => Outcome::Refused { diag },
            _ => Outcome::Crashed {
                code: code.unwrap_or(-1),
                tail: tail_of(&r.stdout, &r.stderr),
            },
        },
    }
}

/// Run one vita job once more with `--obs-dir` and read the phase split back.
///
/// `None` on any failure — a missing `run.json`, an unparseable one, a run that
/// behaved differently under the flag. A reporting side table must never turn a
/// green corpus red, so every path here degrades to "no number".
fn probe_phases(job: &Job, budget: Duration) -> Option<Phases> {
    let dir = std::env::temp_dir().join(format!(
        "vita_corpus_obs_{}_{}",
        std::process::id(),
        job.workload.name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cmd = Command::new(&job.program);
    cmd.args(&job.args)
        .arg("--obs-dir")
        .arg(&dir)
        .current_dir(&job.cwd);
    let _ = run_bounded(&mut cmd, budget);
    let text = std::fs::read_to_string(dir.join("run.json")).ok();
    let _ = std::fs::remove_dir_all(&dir);
    let text = text?;
    Some(Phases {
        elab_s: json_f64(&text, "elab_s")?,
        sim_s: json_f64(&text, "sim_s")?,
    })
}

/// Read one top-level `"key": <number>` out of `run.json`.
///
/// A hand parser and not a JSON dependency: this crate has none, the writer is in
/// this workspace (`cli::obs`), and both numbers are emitted as bare decimals by
/// `fmt_wall`. Anything else — a missing key, a non-numeric value — is `None`.
fn json_f64(text: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{key}\":");
    let rest = &text[text.find(&needle)? + needle.len()..];
    let num: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == 'e' || *c == '+')
        .collect();
    num.parse().ok()
}

/// Build the argument list a tool needs for a workload.
pub fn job_for(root: &Path, w: &'static Workload, tool: Tool, vita: &Path) -> Option<Job> {
    let cwd = root.join("bench").join(w.dir);
    let mut args: Vec<String> = Vec::new();
    let program = match tool {
        Tool::Vita => {
            args.extend(w.vita_args.iter().map(|s| (*s).to_string()));
            args.extend(w.files.iter().map(|s| (*s).to_string()));
            args.extend(w.plusargs.iter().map(|s| (*s).to_string()));
            vita.to_path_buf()
        }
        // iverilog is compiled ahead of the measurement by the caller; here we only
        // run the produced image, so its compile time never lands in the comparison.
        Tool::Iverilog => {
            args.push(vvp_path(w));
            args.extend(w.plusargs.iter().map(|s| (*s).to_string()));
            PathBuf::from("vvp")
        }
        Tool::Verilator => return None,
    };
    Some(Job {
        workload: w,
        tool,
        program,
        args,
        cwd,
    })
}

/// The exit code a successful run of this (tool, workload) must produce.
///
/// Per-workload, not universally 0 (every row is 0 today; `aes` was 1 until §4.5.576).
/// iverilog is always expected to exit 0. And a workload pinned as REFUSED is
/// expected to exit **0** if it ever starts running — that is the promotion, and it
/// has to be observable rather than checked against the code the refusal itself
/// returns. `Expect::Runs { exit }` is what makes the wrong pin unrepresentable;
/// this is where that shape is read.
pub(crate) fn expected_exit(tool: Tool, w: &Workload) -> i32 {
    expected_exit_of(tool, &w.expect)
}

/// [`expected_exit`] for an oracle cell, which has no [`Workload`]. Only `Runs` and
/// `KnownWrong` pin a run, so only they carry a code.
pub(crate) fn expected_exit_of<S>(tool: Tool, expect: &Expect<S>) -> i32 {
    match (tool, expect) {
        (Tool::Vita, Expect::Runs { exit }) => *exit,
        (Tool::Vita, Expect::KnownWrong { exit, .. }) => *exit,
        _ => 0,
    }
}

/// Where to put the compiled `.vvp`, relative to the workload's working directory.
///
/// Not simply `<name>.vvp`: `darkriscv` runs from inside its own pinned git checkout,
/// so writing there dirties the clone and can make a later `checkout --detach`
/// conflict. This walks back out to the workload root instead.
fn vvp_path(w: &Workload) -> String {
    let depth = w.dir.strip_prefix(w.root).map_or(0, |r| {
        r.split('/').filter(|c| !c.is_empty() && *c != ".").count()
    });
    format!("{}{}.vvp", "../".repeat(depth), w.name)
}

/// Compile a workload for `iverilog` **before** the measurement starts.
///
/// vita is a one-shot tool: it elaborates and simulates in the same invocation, so a
/// naive comparison would charge vita for elaboration and iverilog for nothing.
/// Building the `.vvp` up front puts both tools on the same footing — what gets timed
/// is simulation on each side.
pub fn prepare_iverilog(root: &Path, w: &'static Workload) -> Result<(), String> {
    let cwd = root.join("bench").join(w.dir);
    if !cwd.is_dir() {
        return Err(format!("{} is not fetched", w.name));
    }
    let out = vvp_path(w);
    let status = Command::new("iverilog")
        .arg("-g2012")
        .args(w.iverilog_args)
        .arg("-o")
        .arg(&out)
        .args(w.files)
        .current_dir(&cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("iverilog not runnable: {e}"))?;
    if status.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&status.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grade::{grade, Grade};
    use crate::{Expect, Origin, Shape};

    fn wl(expect: Expect) -> Workload {
        Workload {
            name: "t",
            origin: Origin::FirstParty,
            shape: Shape::Cpu,
            root: "t",
            dir: "t",
            files: &["t.v"],
            data: &[],
            plusargs: &[],
            vita_args: &[],
            iverilog_args: &[],
            digest: "DIGEST=abc",
            expect,
            oracle: "iverilog 13.0",
            note: "",
        }
    }

    const RUNS: Expect = Expect::Runs { exit: 0 };
    const REFUSED: Expect = Expect::Refused { diag: "E3009" };

    /// The pinned refusal is searched for across the whole of stderr. vita emits 24
    /// warnings before the pinned error on `verilog-ethernet`; taking the first
    /// diagnostic line made the grade a function of emission order.
    /// The other half of the promotion bug: even with `grade()` right, a refused row
    /// whose expected exit came from the refusal (1) would never reach `Match`.
    #[test]
    fn a_refused_workload_is_expected_to_exit_zero_once_it_runs() {
        assert_eq!(expected_exit(Tool::Vita, &wl(REFUSED)), 0);
        assert_eq!(expected_exit(Tool::Vita, &wl(Expect::Runs { exit: 0 })), 0);
        assert_eq!(expected_exit(Tool::Vita, &wl(Expect::Runs { exit: 1 })), 1);
        // The oracle's expectation is never the workload's.
        assert_eq!(
            expected_exit(Tool::Iverilog, &wl(Expect::Runs { exit: 1 })),
            0
        );
    }

    fn finished(code: Option<i32>, stdout: &str, stderr: &str) -> Run {
        Run {
            code,
            stdout: stdout.into(),
            stderr: stderr.into(),
            secs: 0.0,
            timed_out: false,
        }
    }

    /// vita printed the pinned refusal and then panicked. The text alone says "known
    /// gap"; the exit code says crash, and a crash on a refused row is a regression.
    #[test]
    fn a_panic_after_the_pinned_refusal_is_a_crash_not_a_refusal() {
        let w = wl(REFUSED);
        let err = "x.v:1:1: error[VITA-E3009] nope\nthread 'main' panicked at src/x.rs:1:1\n";
        let o = outcome_of(Tool::Vita, &w, &finished(Some(101), "", err));
        assert!(matches!(o, Outcome::Crashed { code: 101, .. }), "got {o:?}");
        assert!(grade(&w, Tool::Vita, &o).is_failure());
        // Killed by a signal: no code at all.
        let o = outcome_of(Tool::Vita, &w, &finished(None, "", err));
        assert!(matches!(o, Outcome::Crashed { code: -1, .. }), "got {o:?}");
        // The same text with vita's refusal code is the known gap.
        let o = outcome_of(Tool::Vita, &w, &finished(Some(1), "", err));
        assert_eq!(grade(&w, Tool::Vita, &o), Grade::KnownGap);
    }

    /// The digest decides before the refusal text does, and a timeout before either.
    #[test]
    fn a_run_is_read_digest_first() {
        let w = wl(RUNS);
        let o = outcome_of(Tool::Vita, &w, &finished(Some(0), "DIGEST=abc\n", ""));
        assert_eq!(o, Outcome::Match);
        let o = outcome_of(Tool::Vita, &w, &finished(Some(0), "DIGEST=abd\n", ""));
        assert_eq!(
            o,
            Outcome::Mismatch {
                got: "DIGEST=abd".into()
            }
        );
        let o = outcome_of(Tool::Vita, &w, &finished(Some(1), "DIGEST=abc\n", ""));
        assert!(matches!(o, Outcome::Crashed { code: 1, .. }), "got {o:?}");
        let mut r = finished(None, "", "");
        r.timed_out = true;
        assert_eq!(outcome_of(Tool::Vita, &w, &r), Outcome::Timeout);
    }

    #[test]
    fn the_pinned_refusal_is_found_behind_earlier_diagnostics() {
        let w = wl(Expect::Refused { diag: "S_THREADS" });
        let err = "warning[VITA-W3056]: a\nwarning[VITA-W3056]: b\n                   x.v:1:1: error[VITA-E3009]: parameter `S_THREADS` value is not foldable\n";
        let got = refusal(err, &w).expect("the pin is in there");
        assert!(got.contains("S_THREADS"), "got {got:?}");
    }

    /// With no pin to look for, the first real diagnostic is the honest thing to
    /// show — that path is only reached when reporting a drift.
    #[test]
    fn without_a_pin_the_first_diagnostic_is_reported() {
        let w = wl(RUNS);
        let got = refusal(
            "warning: w\nerror[VITA-E1]: first\nerror[VITA-E2]: second\n",
            &w,
        );
        assert_eq!(got.as_deref(), Some("error[VITA-E1]: first"));
    }

    /// `reps` counts TIMED samples. Reporting a median of two while the flag said
    /// three is how a mean ends up wearing the word "median".
    #[test]
    fn reps_counts_timed_samples_not_rounds() {
        // 0..=reps.max(1) rounds run, round 0 is discarded.
        for (reps, want) in [(0usize, 1usize), (1, 1), (3, 3), (5, 5)] {
            let rounds = (0..=reps.max(1)).count();
            assert_eq!(rounds - 1, want, "reps={reps}");
        }
    }

    #[test]
    fn the_vvp_never_lands_inside_a_pinned_upstream_checkout() {
        let mut w = wl(RUNS);
        w.name = "darkriscv";
        w.root = "darkriscv";
        w.dir = "darkriscv/src/sim";
        assert_eq!(vvp_path(&w), "../../darkriscv.vvp");
        w.dir = "darkriscv";
        assert_eq!(vvp_path(&w), "darkriscv.vvp");
    }

    #[test]
    fn the_digest_line_is_the_last_one_not_the_first() {
        // A testbench may print progress; the contract is that the digest is final.
        let out = "DIGEST=early\nsome noise\nDIGEST=late\n";
        assert_eq!(digest_line(out).as_deref(), Some("DIGEST=late"));
    }

    #[test]
    fn median_of_an_even_count_averages_the_middle_pair() {
        assert_eq!(median(vec![4.0, 1.0, 3.0, 2.0]), Some(2.5));
        assert_eq!(median(vec![3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(vec![]), None);
    }
}
