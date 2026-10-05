//! Running cells under a vita binary: the outcome of one run, the pin-time
//! classification, and the graded run of a whole manifest.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use super::admit::{Candidate, Excluded, Reason};
use super::manifest::Cell;
use super::normalize::{normalized, refusal_pin, Producer};
use crate::run::{expected_exit_of, run_bounded};
use crate::Run;
use crate::{grade_expect, Expect, Grade, Outcome, Tool};

/// Run one cell once: copy its source into a fresh directory under the name the slice
/// used, and run `vita <name>` there — the invocation every slice runner used. The
/// directory keeps whatever the design writes (a `$dumpfile`) out of the tree. Every
/// `VITA_*` variable is removed from the child's environment: vita reads several
/// (`VITA_THREADS`, `VITA_JIT`, …), and a grade must not depend on the caller's shell.
pub fn run_cell(
    vita: &Path,
    cells: &Path,
    path: &str,
    name: &str,
    slot: usize,
    budget: Duration,
) -> Run {
    let dir = scratch_dir(slot);
    let _ = std::fs::remove_dir_all(&dir);
    let setup = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::copy(cells.join(path), dir.join(name)).map(|_| ()));
    if let Err(e) = setup {
        let _ = std::fs::remove_dir_all(&dir);
        return Run {
            code: None,
            stdout: String::new(),
            stderr: format!("could not stage {path}: {e}"),
            secs: 0.0,
            timed_out: false,
        };
    }
    let mut cmd = Command::new(vita);
    cmd.arg(name).current_dir(&dir);
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("VITA_") {
            cmd.env_remove(k);
        }
    }
    let r = run_bounded(&mut cmd, budget);
    let _ = std::fs::remove_dir_all(&dir);
    r
}

/// One directory per worker slot, keyed by PID: PIDs recur across test processes, so
/// the directory is removed before use as well as after.
fn scratch_dir(slot: usize) -> PathBuf {
    std::env::temp_dir().join(format!("vita_cells_{}_{slot}", std::process::id()))
}

/// What one run of a cell means, read against the cell's pins. Pure.
///
/// The exit code is checked first, as in the workload corpus: a run that exits as
/// pinned is a `Match` or a `Mismatch` on its normalized stdout; any other exit is a
/// refusal when vita said why (exit 1 and an error diagnostic, read as a
/// [`refusal_pin`]) and a crash otherwise.
pub fn cell_outcome<S>(expect: &Expect<S>, oracle: &str, r: &Run) -> Outcome {
    if r.timed_out {
        return Outcome::Timeout;
    }
    let want = expected_exit_of(Tool::Vita, expect);
    match r.code {
        Some(c) if c == want => {
            let got = normalized(Producer::Vita, &r.stdout);
            if got == oracle {
                Outcome::Match
            } else {
                Outcome::Mismatch { got }
            }
        }
        code => match refusal_pin(&r.stderr) {
            Some(diag) if code == Some(1) => Outcome::Refused { diag },
            _ => Outcome::Crashed {
                code: code.unwrap_or(-1),
                tail: tail(&r.stdout, &r.stderr),
            },
        },
    }
}

fn tail(stdout: &str, stderr: &str) -> String {
    let pick = |s: &str| -> Vec<String> {
        s.lines()
            .rev()
            .filter(|l| !l.trim().is_empty())
            .take(3)
            .map(str::to_string)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    };
    let mut l = pick(stderr);
    if l.is_empty() {
        l = pick(stdout);
    }
    l.join(" / ")
}

/// How two runs of one cell compare: the same exit, the same normalized stdout and the
/// same refusal pin. The raw bytes are not compared — a diagnostic names a file.
fn same_answer(a: &Run, b: &Run) -> bool {
    a.timed_out == b.timed_out
        && a.code == b.code
        && normalized(Producer::Vita, &a.stdout) == normalized(Producer::Vita, &b.stdout)
        && refusal_pin(&a.stderr) == refusal_pin(&b.stderr)
}

/// Short, one-line form of a run's answer for a table cell.
fn answer_of(r: &Run) -> String {
    if r.timed_out {
        return "timed out".into();
    }
    let mut s = format!("exit {}: ", r.code.map_or("-".into(), |c| c.to_string()));
    s.push_str(&one_line(&normalized(Producer::Vita, &r.stdout)));
    if let Some(d) = refusal_pin(&r.stderr) {
        s.push_str(&format!(" [{}]", one_line(&d)));
    }
    s
}

/// A multi-line answer on one table line, bounded.
pub fn one_line(joined: &str) -> String {
    const MAX: usize = 160;
    let s = joined.replace('\n', " / ");
    if s.chars().count() <= MAX {
        s
    } else {
        let cut: String = s.chars().take(MAX).collect();
        format!("{cut}…")
    }
}

/// Pin time: what the classifying binary does with a candidate, from two runs.
pub fn classify(c: &Candidate, a: &Run, b: &Run) -> Result<Cell, Excluded> {
    let excluded = |reason, detail| Excluded {
        path: c.path.clone(),
        reason,
        detail,
    };
    if !same_answer(a, b) {
        return Err(excluded(
            Reason::VitaNondeterministicAtPin,
            format!("{}  |  {}", answer_of(a), answer_of(b)),
        ));
    }
    if a.timed_out {
        return Err(excluded(Reason::VitaCrashAtPin, "timed out".into()));
    }
    let expect = match a.code {
        Some(0) => {
            let got = normalized(Producer::Vita, &a.stdout);
            if got == c.oracle {
                Expect::Runs { exit: 0 }
            } else if same_lines_reordered(&got, &c.oracle) {
                // Every line right, in another order: the oracles may agree on an
                // order IEEE leaves open by coincidence (ENGINEERING_RULES §7.3).
                return Err(excluded(Reason::OrderOnly, one_line(&got)));
            } else {
                Expect::KnownWrong { exit: 0, vita: got }
            }
        }
        Some(1) => match refusal_pin(&a.stderr) {
            Some(pin) => Expect::Refused { diag: pin },
            None => {
                return Err(excluded(
                    Reason::VitaCrashAtPin,
                    format!(
                        "exit 1 without an error diagnostic: {}",
                        tail(&a.stdout, &a.stderr)
                    ),
                ))
            }
        },
        code => {
            return Err(excluded(
                Reason::VitaCrashAtPin,
                format!(
                    "exit {}: {}",
                    code.map_or("signal".into(), |c| c.to_string()),
                    tail(&a.stdout, &a.stderr)
                ),
            ))
        }
    };
    Ok(Cell {
        path: c.path.clone(),
        name: c.name.clone(),
        oracles: c.oracles.clone(),
        oracle: c.oracle.clone(),
        expect,
    })
}

/// Which parts of a refusal pin moved, as `part: pinned → now`.
fn refusal_change(pinned: &str, got: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (pinned.split('\n').collect(), got.split('\n').collect());
    let parts: Vec<String> = a
        .iter()
        .zip(&b)
        .filter(|(x, y)| x != y)
        .map(|(x, y)| format!("{x} → {y}"))
        .collect();
    if parts.is_empty() || a.len() != b.len() {
        one_line(got)
    } else {
        one_line(&parts.join("; "))
    }
}

/// The same lines, in another order.
fn same_lines_reordered(a: &str, b: &str) -> bool {
    let mut x: Vec<&str> = a.split('\n').collect();
    let mut y: Vec<&str> = b.split('\n').collect();
    x.sort_unstable();
    y.sort_unstable();
    a != b && x == y
}

/// Apply `f` to every item on `jobs` threads; results come back in input order, so
/// nothing downstream depends on scheduling.
pub fn par_map<T: Sync, R: Send>(
    items: &[T],
    jobs: usize,
    f: impl Fn(usize, &T) -> R + Sync,
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let out: Vec<Mutex<Option<R>>> = items.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for slot in 0..jobs.max(1) {
            let (next, out, f) = (&next, &out, &f);
            s.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(slot, item);
                *out[i].lock().expect("no worker panics holding a slot") = Some(r);
            });
        }
    });
    out.into_iter()
        .map(|m| {
            m.into_inner()
                .expect("no worker panics holding a slot")
                .expect("every item was run")
        })
        .collect()
}

/// The default worker count: every core.
pub fn default_jobs() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

/// One graded row of `cells run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graded {
    pub grade: Grade,
    pub detail: String,
}

impl Graded {
    /// Whether this row fails the gate. Stricter than the workload corpus: ANY move
    /// away from the manifest fails — a `PROMOTED` too — and the grade names its
    /// direction. A fix that is never re-pinned would otherwise be lost silently the
    /// day it is reverted, because the old pin still reads as `known-wrong`.
    pub fn fails(&self) -> bool {
        !matches!(self.grade, Grade::Ok | Grade::KnownWrong | Grade::KnownGap)
    }
}

/// The grade of one cell's run. The workload table, except that a refused cell's pin
/// is matched EXACTLY: the corpus pins a fragment of a message, a cell pins the whole
/// first error line and every error code ([`refusal_pin`]).
pub fn grade_outcome(expect: &Expect<String>, outcome: &Outcome) -> Grade {
    if let (Expect::Refused { diag }, Outcome::Refused { diag: got }) = (expect, outcome) {
        return if got == diag {
            Grade::KnownGap
        } else {
            Grade::Drifted { got: got.clone() }
        };
    }
    grade_expect(expect, Tool::Vita, outcome)
}

/// Grade one cell from its two runs. A cell whose runs disagree is a `Regression`
/// whatever either run says — two answers to one design outrank both of them — and its
/// detail carries the `*** NON-DETERMINISTIC ***` mark the workload table uses.
pub fn grade_cell(cell: &Cell, a: &Run, b: &Run) -> Graded {
    if !same_answer(a, b) {
        return Graded {
            grade: Grade::Regression("non-deterministic".into()),
            detail: format!(
                "*** NON-DETERMINISTIC *** {}  |  {}",
                answer_of(a),
                answer_of(b)
            ),
        };
    }
    let outcome = cell_outcome(&cell.expect, &cell.oracle, a);
    let grade = grade_outcome(&cell.expect, &outcome);
    let detail = match (&grade, &outcome, &cell.expect) {
        (Grade::Regression(why), _, _) => one_line(why),
        (Grade::Drifted { got }, Outcome::Refused { .. }, Expect::KnownWrong { .. }) => {
            format!("climb: silent-wrong → loud; re-pin: {}", one_line(got))
        }
        (Grade::Drifted { got }, _, Expect::KnownWrong { .. }) => {
            format!(
                "the pinned wrong answer moved; re-pin if intended: {}",
                one_line(got)
            )
        }
        (Grade::Drifted { got }, _, Expect::Refused { diag }) => {
            format!(
                "the refusal moved; re-pin if intended: {}",
                refusal_change(diag, got)
            )
        }
        (Grade::Drifted { got }, _, _) => one_line(got),
        (Grade::Promoted, _, _) => "now prints the oracles' answer; re-pin".into(),
        (Grade::KnownGap, Outcome::Refused { diag }, _) => one_line(diag),
        _ => String::new(),
    };
    Graded { grade, detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: &str = "a=1\nb=2";

    fn run(code: Option<i32>, stdout: &str, stderr: &str) -> Run {
        Run {
            code,
            stdout: stdout.into(),
            stderr: stderr.into(),
            secs: 0.0,
            timed_out: false,
        }
    }

    fn hung() -> Run {
        let mut r = run(None, "", "");
        r.timed_out = true;
        r
    }

    fn cell(expect: Expect<String>) -> Cell {
        Cell {
            path: "AE/x.sv".into(),
            name: "x.sv".into(),
            oracles: vec!["iverilog".into(), "verilator".into()],
            oracle: ORACLE.into(),
            expect,
        }
    }

    /// The pin of [`REFUSAL`].
    const PIN: &str = "3:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: no\ncodes VITA-E3009\nat 3:5";

    fn runs() -> Cell {
        cell(Expect::Runs { exit: 0 })
    }
    fn known_wrong() -> Cell {
        cell(Expect::KnownWrong {
            exit: 0,
            vita: "a=0\nb=2".into(),
        })
    }
    fn refused() -> Cell {
        cell(Expect::Refused { diag: PIN.into() })
    }

    /// The oracles' answer, the pinned wrong one, another wrong one, as vita prints
    /// them: with its end anchor as the last stdout line.
    const RIGHT: &str = "a=1\nb=2\nsimulation ended (Finish) at time 1\n";
    const PINNED_WRONG: &str = "a=0\nb=2\nsimulation ended (Finish) at time 1\n";
    const OTHER_WRONG: &str = "a=7\nb=2\nsimulation ended (Finish) at time 1\n";
    /// The same lines at another end time: not compared (README, blind spots).
    const LATE: &str = "a=1\nb=2\nsimulation ended (Finish) at time 2\n";
    const REFUSAL: &str =
        "x.sv:3:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: no\nerrors=1 warnings=0 notes=0\n";
    /// The same first code, another position: an arm that flipped.
    const MOVED_REFUSAL: &str =
        "x.sv:4:52: error[VITA-E3009] E-ELAB-UNSUPPORTED: no\nerrors=1 warnings=0 notes=0\n";
    /// The same first line, one more error code behind it.
    const WIDER_REFUSAL: &str = "x.sv:3:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: no\n\
                                 x.sv:6:1: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: w\n";
    const OTHER_REFUSAL: &str = "x.sv:3:5: error[VITA-E2002] E-PARSE-X: no\n";
    const PANIC: &str = "thread 'main' panicked at src/x.rs:1:1\n";

    fn graded(c: &Cell, r: Run) -> Graded {
        let again = Run {
            code: r.code,
            stdout: r.stdout.clone(),
            stderr: r.stderr.clone(),
            secs: 0.0,
            timed_out: r.timed_out,
        };
        grade_cell(c, &r, &again)
    }

    fn g(c: &Cell, r: Run) -> Grade {
        graded(c, r).grade
    }

    fn is_regression(gr: &Grade) -> bool {
        matches!(gr, Grade::Regression(_))
    }

    /// A cell that runs right: the oracles' answer is ok, anything else is red. The
    /// end time is not part of the answer (README, "Known blind spots").
    #[test]
    fn a_runs_cell_is_ok_only_on_the_oracles_answer() {
        assert_eq!(g(&runs(), run(Some(0), RIGHT, "")), Grade::Ok);
        assert_eq!(g(&runs(), run(Some(0), LATE, "")), Grade::Ok);
        assert!(!graded(&runs(), run(Some(0), RIGHT, "")).fails());
        for r in [
            run(Some(0), PINNED_WRONG, ""),
            run(Some(1), "", REFUSAL),
            run(Some(101), "", PANIC),
            run(Some(2), RIGHT, ""),
            run(None, "", ""),
            hung(),
        ] {
            let gr = graded(&runs(), r);
            assert!(is_regression(&gr.grade) && gr.fails(), "got {gr:?}");
        }
    }

    /// The moves on a known-wrong cell. Every one of them fails the cells gate; the
    /// grade names the direction.
    #[test]
    fn a_known_wrong_cell_tells_fixed_from_differently_wrong_from_broken() {
        let same = graded(&known_wrong(), run(Some(0), PINNED_WRONG, ""));
        assert_eq!(same.grade, Grade::KnownWrong);
        assert_eq!(same.grade.label(), "known-wrong");
        assert!(!same.fails(), "a pinned wrong answer is visible, not red");

        let fixed = graded(&known_wrong(), run(Some(0), RIGHT, ""));
        assert_eq!(fixed.grade, Grade::Promoted);
        assert!(
            fixed.fails(),
            "an un-pinned fix fails until it is re-pinned"
        );
        assert!(fixed.detail.contains("re-pin"), "{}", fixed.detail);

        let moved = graded(&known_wrong(), run(Some(0), OTHER_WRONG, ""));
        assert_eq!(
            moved.grade,
            Grade::Drifted {
                got: "a=7\nb=2".into()
            }
        );
        assert!(moved.fails());

        // Wrong to loud is a climb, not a regression; it still fails.
        let loud = graded(&known_wrong(), run(Some(1), "", REFUSAL));
        assert!(matches!(loud.grade, Grade::Drifted { .. }), "{loud:?}");
        assert!(
            loud.fails()
                && loud
                    .detail
                    .starts_with("climb: silent-wrong → loud; re-pin")
        );

        for r in [
            run(Some(101), PINNED_WRONG, PANIC),
            run(Some(2), PINNED_WRONG, ""),
            run(Some(1), PINNED_WRONG, ""),
            run(None, "", ""),
            hung(),
        ] {
            let gr = graded(&known_wrong(), r);
            assert!(is_regression(&gr.grade) && gr.fails(), "got {gr:?}");
        }
    }

    /// The exit a known-wrong cell is held to is its own pin: a cell pinned at exit 1
    /// (none is today) prints its wrong answer at exit 1 and is known-wrong there,
    /// while the same output at exit 0 is a different exit and red.
    #[test]
    fn a_known_wrong_cell_reads_the_exit_from_its_own_variant() {
        let at1 = cell(Expect::KnownWrong {
            exit: 1,
            vita: "a=0\nb=2".into(),
        });
        assert_eq!(g(&at1, run(Some(1), PINNED_WRONG, "")), Grade::KnownWrong);
        assert!(is_regression(&g(&at1, run(Some(0), PINNED_WRONG, ""))));
        assert_eq!(expected_exit_of(Tool::Vita, &at1.expect), 1);
        assert_eq!(expected_exit_of(Tool::Vita, &refused().expect), 0);
    }

    /// A refused cell is matched EXACTLY on its first error line and its code set.
    #[test]
    fn a_refused_cell_is_a_known_gap_only_on_its_exact_refusal() {
        let gap = graded(&refused(), run(Some(1), "", REFUSAL));
        assert_eq!(gap.grade, Grade::KnownGap);
        assert!(!gap.fails());
        for r in [MOVED_REFUSAL, WIDER_REFUSAL, OTHER_REFUSAL] {
            let moved = graded(&refused(), run(Some(1), "", r));
            assert!(
                matches!(moved.grade, Grade::Drifted { .. }) && moved.fails(),
                "{moved:?}"
            );
        }
        // A pin degraded to a fragment matches nothing: substring is never enough.
        let weak = cell(Expect::Refused {
            diag: "error".into(),
        });
        assert!(graded(&weak, run(Some(1), "", REFUSAL)).fails());

        let fixed = graded(&refused(), run(Some(0), RIGHT, ""));
        assert_eq!(fixed.grade, Grade::Promoted);
        assert!(fixed.fails());
        // Loud to silent-wrong, and a crash after the pinned text, are red.
        assert!(is_regression(&g(&refused(), run(Some(0), OTHER_WRONG, ""))));
        assert!(is_regression(&g(&refused(), run(Some(101), "", REFUSAL))));
        assert!(is_regression(&g(&refused(), hung())));
    }

    /// Two runs that answer differently are red whatever each says, and say so with
    /// the workload table's mark.
    #[test]
    fn two_different_answers_are_non_deterministic_and_red() {
        for c in [runs(), known_wrong(), refused()] {
            let gr = grade_cell(
                &c,
                &run(Some(0), RIGHT, ""),
                &run(Some(0), PINNED_WRONG, ""),
            );
            assert!(gr.fails() && is_regression(&gr.grade), "{gr:?}");
            assert!(
                gr.detail.starts_with("*** NON-DETERMINISTIC *** "),
                "{}",
                gr.detail
            );
        }
        // A diagnostic's file name is not part of the answer.
        let a = run(
            Some(1),
            "",
            "a.sv:3:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: no\n",
        );
        let b = run(
            Some(1),
            "",
            "b.sv:3:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: no\n",
        );
        assert_eq!(grade_cell(&refused(), &a, &b).grade, Grade::KnownGap);
    }

    /// The workload table is unchanged: only REGRESSION, DRIFTED and ORACLE-DRIFT
    /// fail there, while the cells gate also fails PROMOTED (and every other move).
    #[test]
    fn the_cells_gate_is_stricter_than_the_workload_table() {
        let all = [
            Grade::Ok,
            Grade::KnownGap,
            Grade::KnownWrong,
            Grade::Promoted,
            Grade::RuledSplit,
            Grade::Absent,
            Grade::Regression(String::new()),
            Grade::Drifted { got: String::new() },
            Grade::OracleDrifted { got: String::new() },
        ];
        let workload: Vec<&str> = all
            .iter()
            .filter(|g| g.is_failure())
            .map(Grade::label)
            .collect();
        assert_eq!(workload, ["REGRESSION", "DRIFTED", "ORACLE-DRIFT"]);
        let cells: Vec<&str> = all
            .iter()
            .filter(|g| {
                Graded {
                    grade: (*g).clone(),
                    detail: String::new(),
                }
                .fails()
            })
            .map(Grade::label)
            .collect();
        assert_eq!(
            cells,
            [
                "PROMOTED",
                "ruled-split",
                "absent",
                "REGRESSION",
                "DRIFTED",
                "ORACLE-DRIFT"
            ]
        );
    }

    fn cand() -> Candidate {
        Candidate {
            path: "AE/x.sv".into(),
            name: "x.sv".into(),
            oracles: vec!["iverilog".into(), "verilator".into()],
            oracle: ORACLE.into(),
        }
    }

    fn classified(a: Run, b: Run) -> Result<Expect<String>, Reason> {
        classify(&cand(), &a, &b)
            .map(|c| c.expect)
            .map_err(|x| x.reason)
    }

    fn twice(code: Option<i32>, out: &str, err: &str) -> Result<Expect<String>, Reason> {
        classified(run(code, out, err), run(code, out, err))
    }

    /// Pin time reads what the binary does, never a slice's verdict word.
    #[test]
    fn pinning_classifies_by_the_run() {
        assert_eq!(twice(Some(0), RIGHT, ""), Ok(Expect::Runs { exit: 0 }));
        assert_eq!(
            twice(Some(0), PINNED_WRONG, ""),
            Ok(Expect::KnownWrong {
                exit: 0,
                vita: "a=0\nb=2".into()
            })
        );
        assert_eq!(
            twice(Some(1), "", REFUSAL),
            Ok(Expect::Refused { diag: PIN.into() })
        );
        // The oracles' lines in another order are not pinned as wrong.
        assert_eq!(
            twice(
                Some(0),
                "b=2\na=1\nsimulation ended (Finish) at time 1\n",
                ""
            ),
            Err(Reason::OrderOnly)
        );
        assert_eq!(twice(Some(1), "", ""), Err(Reason::VitaCrashAtPin));
        assert_eq!(twice(Some(101), "", PANIC), Err(Reason::VitaCrashAtPin));
        assert_eq!(twice(Some(2), RIGHT, ""), Err(Reason::VitaCrashAtPin));
        assert_eq!(classified(hung(), hung()), Err(Reason::VitaCrashAtPin));
        assert_eq!(
            classified(run(Some(0), RIGHT, ""), run(Some(0), OTHER_WRONG, "")),
            Err(Reason::VitaNondeterministicAtPin)
        );
    }

    /// The graded vita sees none of the caller's `VITA_*` variables. `/bin/sh` stands
    /// in for vita and prints its environment.
    #[test]
    fn a_cell_runs_without_the_callers_vita_variables() {
        let dir = std::env::temp_dir().join(format!("vita_cells_env_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("R")).unwrap();
        std::fs::write(dir.join("R/env.sh"), "env\n").unwrap();
        std::env::set_var("VITA_CELLS_PROBE", "leaked");
        std::env::set_var("CELLS_PROBE_KEPT", "kept");
        let r = run_cell(
            Path::new("/bin/sh"),
            &dir,
            "R/env.sh",
            "env.sh",
            0,
            Duration::from_secs(20),
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(r.code, Some(0), "{}", r.stderr);
        assert!(
            r.stdout.contains("CELLS_PROBE_KEPT=kept"),
            "the probe ran: {}",
            r.stdout
        );
        assert!(!r.stdout.contains("VITA_CELLS_PROBE"), "{}", r.stdout);
    }

    #[test]
    fn par_map_keeps_input_order() {
        let items: Vec<usize> = (0..100).collect();
        let out = par_map(&items, 7, |_, &i| i * 2);
        assert_eq!(out, (0..100).map(|i| i * 2).collect::<Vec<_>>());
    }
}
