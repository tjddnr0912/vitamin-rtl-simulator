//! Running a workload and timing it honestly.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::Workload;

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
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    secs: f64,
    timed_out: bool,
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
fn run_bounded(cmd: &mut Command, budget: Duration) -> Run {
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

            let want = expected_exit(j.tool, j.workload);

            let outcome = if r.timed_out {
                Outcome::Timeout
            } else {
                match (r.code, digest_line(&r.stdout)) {
                    (Some(c), Some(d)) if c == want && d == j.workload.digest => {
                        if !acc[i].digests.contains(&d) {
                            acc[i].digests.push(d);
                        }
                        Outcome::Match
                    }
                    (Some(c), Some(d)) if c == want => {
                        if !acc[i].digests.contains(&d) {
                            acc[i].digests.push(d.clone());
                        }
                        Outcome::Mismatch { got: d }
                    }
                    // Right answer, wrong exit code. Not a mismatch — the simulation
                    // was correct — but not a clean pass either, and saying so is the
                    // whole reason the expected code is pinned.
                    (Some(c), Some(d)) if d == j.workload.digest => Outcome::Crashed {
                        code: c,
                        tail: format!("digest correct but exit {c}, expected {want}"),
                    },
                    // The pinned refusal is matched against the WHOLE of stderr, not
                    // against whichever diagnostic happens to come first: vita emits
                    // 24 warnings before the pinned error on `verilog-ethernet`, and
                    // grading on emission order would call a reordering a drift.
                    (_, _) => match refusal(&r.stderr, j.workload) {
                        Some(diag) => Outcome::Refused { diag },
                        None => Outcome::Crashed {
                            code: r.code.unwrap_or(-1),
                            tail: tail_of(&r.stdout, &r.stderr),
                        },
                    },
                }
            };
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

    for m in &mut acc {
        m.median_secs = median(m.secs.clone());
    }
    acc
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
/// Per-workload, not universally 0: `aes` prints the right digest and still exits 1.
/// iverilog is always expected to exit 0. And a workload pinned as REFUSED is
/// expected to exit **0** if it ever starts running — that is the promotion, and it
/// has to be observable rather than checked against the code the refusal itself
/// returns. `Expect::Runs { exit }` is what makes the wrong pin unrepresentable;
/// this is where that shape is read.
pub(crate) fn expected_exit(tool: Tool, w: &Workload) -> i32 {
    match (tool, &w.expect) {
        (Tool::Vita, crate::Expect::Runs { exit }) => *exit,
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

/// The verdict once an [`Outcome`] is read against what the manifest expected.
///
/// The distinction matters because a refusal is not a failure — it is the
/// correct-or-loud ladder working — but a refusal *for a different reason than the
/// one pinned*, or a refusal on a design that used to run, is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grade {
    /// Ran and matched the oracle's digest, as expected.
    Ok,
    /// Refused with the pinned diagnostic. A known gap, and part of the count the
    /// corpus exists to shrink.
    KnownGap,
    /// Refused where the manifest says it runs, or ran differently than pinned.
    Regression(String),
    /// Ran where the manifest says it refuses: a slice landed. Not a failure — but
    /// the manifest row is now wrong and has to be moved to `Expect::Runs`.
    Promoted,
    /// Refused for a reason other than the pinned one. The gap moved; somebody has
    /// to look, because the pinned diagnostic no longer describes the design.
    Drifted { got: String },
    /// Not on this machine.
    Absent,
    /// The oracle itself no longer reproduces the pinned digest. Nothing about vita
    /// is being asserted here — either the pin is stale or this machine's simulator
    /// differs from the one that produced it, and both invalidate the row until
    /// someone looks.
    OracleDrifted { got: String },
}

impl Grade {
    pub fn label(&self) -> &'static str {
        match self {
            Grade::Ok => "ok",
            Grade::KnownGap => "known-gap",
            Grade::Regression(_) => "REGRESSION",
            Grade::Promoted => "PROMOTED",
            Grade::Drifted { .. } => "DRIFTED",
            Grade::Absent => "absent",
            Grade::OracleDrifted { .. } => "ORACLE-DRIFT",
        }
    }

    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Grade::Regression(_) | Grade::Drifted { .. } | Grade::OracleDrifted { .. }
        )
    }
}

/// Read an outcome against the manifest's expectation.
///
/// `Workload::expect` is a statement about **vita**, so an oracle run is graded on a
/// different axis entirely: it either reproduces the pinned digest or it does not.
/// Grading iverilog against `Expect::Refused` would report every known gap as a
/// promotion, which is exactly backwards.
pub fn grade(w: &Workload, tool: Tool, outcome: &Outcome) -> Grade {
    use crate::Expect;
    if tool != Tool::Vita {
        return match outcome {
            Outcome::Absent => Grade::Absent,
            Outcome::Match => Grade::Ok,
            Outcome::Mismatch { got } => Grade::OracleDrifted { got: got.clone() },
            Outcome::Refused { diag } => Grade::OracleDrifted { got: diag.clone() },
            Outcome::Crashed { code, tail } => Grade::OracleDrifted {
                got: format!("exit {code}: {tail}"),
            },
            Outcome::Timeout => Grade::OracleDrifted {
                got: "timed out".into(),
            },
        };
    }
    match (&w.expect, outcome) {
        (_, Outcome::Absent) => Grade::Absent,

        (Expect::Runs { .. }, Outcome::Match) => Grade::Ok,
        (Expect::Runs { .. }, Outcome::Mismatch { got }) => {
            Grade::Regression(format!("digest changed: {got}"))
        }
        (Expect::Runs { .. }, Outcome::Refused { diag }) => {
            Grade::Regression(format!("newly refused: {diag}"))
        }
        (Expect::Runs { .. }, Outcome::Crashed { code, tail }) => {
            Grade::Regression(format!("exit {code}: {tail}"))
        }
        (Expect::Runs { .. }, Outcome::Timeout) => Grade::Regression("timed out".into()),

        (Expect::Refused { .. }, Outcome::Match) => Grade::Promoted,
        (Expect::Refused { diag }, Outcome::Refused { diag: got }) => {
            if got.contains(diag) {
                Grade::KnownGap
            } else {
                Grade::Drifted { got: got.clone() }
            }
        }
        // A design pinned as refused that now produces a WRONG answer is the worst
        // move on the ladder — loud to silent-wrong — and is graded as such.
        (Expect::Refused { .. }, Outcome::Mismatch { got }) => {
            Grade::Regression(format!("was loud, now silently wrong: {got}"))
        }
        (Expect::Refused { .. }, Outcome::Crashed { code, tail }) => {
            Grade::Regression(format!("was loud, now crashes: exit {code}: {tail}"))
        }
        (Expect::Refused { .. }, Outcome::Timeout) => {
            Grade::Regression("was loud, now hangs".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn a_match_where_the_manifest_says_runs_is_ok() {
        assert_eq!(grade(&wl(RUNS), Tool::Vita, &Outcome::Match), Grade::Ok);
    }

    #[test]
    fn a_refusal_where_the_manifest_says_runs_is_a_regression() {
        let g = grade(
            &wl(RUNS),
            Tool::Vita,
            &Outcome::Refused {
                diag: "E3009 nope".into(),
            },
        );
        assert!(matches!(g, Grade::Regression(_)), "got {g:?}");
    }

    #[test]
    fn the_pinned_refusal_is_a_known_gap_not_a_failure() {
        let g = grade(
            &wl(REFUSED),
            Tool::Vita,
            &Outcome::Refused {
                diag: "x: error[VITA-E3009] y".into(),
            },
        );
        assert_eq!(g, Grade::KnownGap);
        assert!(!g.is_failure());
    }

    #[test]
    fn refusing_for_a_different_reason_is_drift_and_fails() {
        let g = grade(
            &wl(REFUSED),
            Tool::Vita,
            &Outcome::Refused {
                diag: "error[VITA-E2002]".into(),
            },
        );
        assert!(matches!(g, Grade::Drifted { .. }), "got {g:?}");
        assert!(g.is_failure());
    }

    #[test]
    fn running_a_refused_workload_is_a_promotion_not_a_failure() {
        let g = grade(&wl(REFUSED), Tool::Vita, &Outcome::Match);
        assert_eq!(g, Grade::Promoted);
        assert!(!g.is_failure());
    }

    /// The one move the ladder forbids: a design that was honestly loud now answers,
    /// and answers wrongly. It must never be graded as a promotion.
    #[test]
    fn loud_becoming_silently_wrong_is_a_regression() {
        let g = grade(
            &wl(REFUSED),
            Tool::Vita,
            &Outcome::Mismatch {
                got: "DIGEST=bad".into(),
            },
        );
        assert!(matches!(g, Grade::Regression(_)), "got {g:?}");
        assert!(g.is_failure());
    }

    /// The oracle is graded on whether it still reproduces the pin, never against
    /// `Expect` — which describes vita. Grading it the other way reported every
    /// known gap as a promotion.
    #[test]
    fn the_oracle_is_not_graded_against_the_vita_expectation() {
        assert_eq!(
            grade(&wl(REFUSED), Tool::Iverilog, &Outcome::Match),
            Grade::Ok
        );
        let g = grade(
            &wl(RUNS),
            Tool::Iverilog,
            &Outcome::Mismatch { got: "x".into() },
        );
        assert!(matches!(g, Grade::OracleDrifted { .. }), "got {g:?}");
        assert!(g.is_failure());
    }

    #[test]
    fn an_absent_workload_never_fails_the_gate() {
        for e in [RUNS, REFUSED] {
            let g = grade(&wl(e), Tool::Vita, &Outcome::Absent);
            assert_eq!(g, Grade::Absent);
            assert!(!g.is_failure());
        }
    }

    /// The event the corpus exists to detect. An earlier version pinned the exit
    /// code as a free field beside `expect`, and the three refused rows carried the
    /// code vita returns WHILE REFUSING (1). The day a gap closed, vita would exit 0,
    /// fail the equality test, and grade "was loud, now crashes" — red, and false.
    /// `Expect::Runs { exit }` makes that unrepresentable; this pins the behaviour.
    #[test]
    fn a_closed_gap_is_a_promotion_and_the_refusals_exit_code_is_not_consulted() {
        let w = wl(REFUSED);
        assert!(matches!(w.expect, Expect::Refused { .. }));
        // A promoted workload exits 0 — nothing in the manifest can demand otherwise.
        assert_eq!(grade(&w, Tool::Vita, &Outcome::Match), Grade::Promoted);
        assert!(!grade(&w, Tool::Vita, &Outcome::Match).is_failure());
    }

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
