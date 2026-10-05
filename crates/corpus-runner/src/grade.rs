//! The grading table: what a run means once it is read against the expectation its
//! manifest row pins. Shared by the workload corpus (`run`) and the oracle cells
//! (`cells`), so a grade means the same thing in both tables.

use crate::run::{Outcome, Tool};
use crate::{Expect, Workload};

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
    /// Refused for a reason other than the pinned one, or (`Expect::KnownWrong`) printed
    /// another wrong answer. The pin no longer describes the design; somebody looks.
    Drifted { got: String },
    /// Ran, and disagreed with the oracle exactly as the manifest's `Expect::Split`
    /// says it does. Visible on every run, and NOT a failure: the row records a
    /// ruling, not a passing grade.
    RuledSplit,
    /// Printed exactly the wrong answer its `Expect::KnownWrong` pins: `KnownGap`'s
    /// counterpart for a silent-wrong, counted on every run and not a failure.
    KnownWrong,
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
            Grade::RuledSplit => "ruled-split",
            Grade::KnownWrong => "known-wrong",
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
    grade_expect(&w.expect, tool, outcome)
}

/// [`grade`] off an expectation alone: the one grading table, shared by the workload
/// corpus and the oracle cells (`cells`).
pub fn grade_expect<S: AsRef<str>>(expect: &Expect<S>, tool: Tool, outcome: &Outcome) -> Grade {
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
    match (expect, outcome) {
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
            if got.contains(diag.as_ref()) {
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

        // A ruled split. `Outcome::Match` compares against the ORACLE's digest, so a
        // match here means the two agreed again — the split closed, which is the
        // event this row is waiting for and is reported as a promotion.
        (Expect::Split { .. }, Outcome::Match) => Grade::Promoted,
        (Expect::Split { vita, .. }, Outcome::Mismatch { got }) => {
            if got == vita.as_ref() {
                Grade::RuledSplit
            } else {
                Grade::Regression(format!("digest changed: {got}"))
            }
        }
        (Expect::Split { .. }, Outcome::Refused { diag }) => {
            Grade::Regression(format!("newly refused: {diag}"))
        }
        (Expect::Split { .. }, Outcome::Crashed { code, tail }) => {
            Grade::Regression(format!("exit {code}: {tail}"))
        }
        (Expect::Split { .. }, Outcome::Timeout) => Grade::Regression("timed out".into()),
        (Expect::Refused { .. }, Outcome::Timeout) => {
            Grade::Regression("was loud, now hangs".into())
        }

        // `Match` is against the ORACLE: the fix this row waits for. Only the pinned
        // wrong answer itself is tolerated.
        (Expect::KnownWrong { .. }, Outcome::Match) => Grade::Promoted,
        (Expect::KnownWrong { vita, .. }, Outcome::Mismatch { got }) => {
            if got == vita.as_ref() {
                Grade::KnownWrong
            } else {
                Grade::Drifted { got: got.clone() }
            }
        }
        // Wrong to loud is a climb on the ladder, not a regression — but the pin no
        // longer describes the cell, so it is a drift, and a drift fails.
        (Expect::KnownWrong { .. }, Outcome::Refused { diag }) => {
            Grade::Drifted { got: diag.clone() }
        }
        (Expect::KnownWrong { .. }, Outcome::Crashed { code, tail }) => {
            Grade::Regression(format!("was known-wrong, now exit {code}: {tail}"))
        }
        (Expect::KnownWrong { .. }, Outcome::Timeout) => {
            Grade::Regression("was known-wrong, now hangs".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Origin, Shape};

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

    /// A RULED split pins BOTH answers, so it stays visible without being red and
    /// without becoming a licence to pin a wrong digest.
    ///
    /// ⚠️ Three cells, and the middle one is the point: only vita's own pinned
    /// answer is tolerated. A third digest is a `Regression` exactly as it would be
    /// on a `Runs` row, and agreeing with the ORACLE again is the event the row is
    /// waiting for — a `Promoted`, not an `Ok`, because the manifest row is then
    /// wrong and someone has to move it.
    #[test]
    fn a_ruled_split_tolerates_one_answer_and_only_that_one() {
        let w = wl(Expect::Split {
            vita: "DIGEST=beef",
            why: "ROADMAP §2-N",
        });
        let g = grade(
            &w,
            Tool::Vita,
            &Outcome::Mismatch {
                got: "DIGEST=beef".into(),
            },
        );
        assert_eq!(g, Grade::RuledSplit);
        assert!(!g.is_failure(), "a recorded ruling is not a red gate");

        let moved = grade(
            &w,
            Tool::Vita,
            &Outcome::Mismatch {
                got: "DIGEST=cafe".into(),
            },
        );
        assert!(moved.is_failure(), "vita's own answer moved: {moved:?}");

        // The split closing is the promotion this row exists to catch.
        assert_eq!(grade(&w, Tool::Vita, &Outcome::Match), Grade::Promoted);
        // …and a split row counts as one vita RUNS.
        assert!(!w.is_refused());
    }
}
