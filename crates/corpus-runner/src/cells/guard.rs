//! The re-pin guard: `cells pin` compares what it is about to write with the manifest
//! already there.
//!
//! The gate fails on any move from the manifest, so the one way to make it pass again
//! without fixing vita is to re-pin. A re-pin under a broken binary would quietly
//! rewrite a right cell as known-wrong, or move a crashing or reordering cell out of
//! the manifest. So every move is printed, and a move in the regression direction is
//! written only when the cell is named on the command line (`--accept`,
//! `--accept-file`):
//!
//! | Move | Direction |
//! |---|---|
//! | runs → known-wrong, runs → refused, refused → known-wrong | regression: must be named |
//! | known-wrong → another known-wrong, refused → another refusal | regression: must be named |
//! | a pinned cell leaving for a vita-side reason (crash, non-determinism, order-only), or with no reason | regression: must be named |
//! | known-wrong → runs, refused → runs, known-wrong → refused | improvement: printed |
//! | the oracles' answer or an exclusion reason changing; a cell entering | oracle side: printed |

use std::collections::{BTreeMap, BTreeSet};

use super::admit::{Excluded, Reason};
use super::exec::one_line;
use super::manifest::Cell;
use super::Pinned;
use crate::Expect;

/// Which way a move goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    /// Must be named to be written.
    Regression,
    Improvement,
    /// The oracle side changed, or a cell entered the manifest.
    OracleSide,
}

/// One cell's move between the existing manifest and the new pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Move {
    pub path: String,
    pub direction: Direction,
    pub from: String,
    pub to: String,
}

/// A cell's standing in one pin.
#[derive(Debug, Clone, PartialEq, Eq)]
enum State<'a> {
    Pinned(&'a Cell),
    Excluded(&'a Excluded),
    Absent,
}

fn vita_side(r: Reason) -> bool {
    matches!(
        r,
        Reason::VitaCrashAtPin | Reason::VitaNondeterministicAtPin | Reason::OrderOnly
    )
}

fn describe(s: &State) -> String {
    match s {
        State::Pinned(c) => match &c.expect {
            Expect::Runs { .. } => "runs".into(),
            Expect::KnownWrong { vita, .. } => format!("known-wrong [{}]", one_line(vita)),
            Expect::Refused { diag } => format!("refused [{}]", one_line(diag)),
            Expect::Split { .. } => "split".into(),
        },
        State::Excluded(x) => format!("excluded: {}", x.reason.label()),
        State::Absent => "absent".into(),
    }
}

/// Every move from (`old_cells`, `old_excluded`) to `new`, sorted by direction, then
/// path.
pub fn compare(old_cells: &[Cell], old_excluded: &[Excluded], new: &Pinned) -> Vec<Move> {
    let mut old: BTreeMap<&str, State> = BTreeMap::new();
    for x in old_excluded {
        old.insert(&x.path, State::Excluded(x));
    }
    for c in old_cells {
        old.insert(&c.path, State::Pinned(c));
    }
    let mut now: BTreeMap<&str, State> = BTreeMap::new();
    for x in &new.excluded {
        now.insert(&x.path, State::Excluded(x));
    }
    for c in &new.cells {
        now.insert(&c.path, State::Pinned(c));
    }
    let paths: BTreeSet<&str> = old.keys().chain(now.keys()).copied().collect();
    let mut moves = Vec::new();
    for p in paths {
        let a = old.get(p).cloned().unwrap_or(State::Absent);
        let b = now.get(p).cloned().unwrap_or(State::Absent);
        if let Some(direction) = direction(&a, &b) {
            moves.push(Move {
                path: p.to_string(),
                direction,
                from: describe(&a),
                to: describe(&b),
            });
        }
    }
    moves.sort_by(|x, y| (x.direction, &x.path).cmp(&(y.direction, &y.path)));
    moves
}

/// The direction of one cell's move; `None` when nothing moved.
fn direction(a: &State, b: &State) -> Option<Direction> {
    use Direction::*;
    match (a, b) {
        (State::Pinned(x), State::Pinned(y)) => {
            let vita = match (&x.expect, &y.expect) {
                (Expect::Runs { exit: e }, Expect::Runs { exit: f }) if e == f => None,
                (
                    Expect::KnownWrong { exit: e, vita: v },
                    Expect::KnownWrong { exit: f, vita: w },
                ) if e == f && v == w => None,
                (Expect::Refused { diag: p }, Expect::Refused { diag: q }) if p == q => None,
                (Expect::KnownWrong { .. } | Expect::Refused { .. }, Expect::Runs { .. })
                | (Expect::KnownWrong { .. }, Expect::Refused { .. }) => Some(Improvement),
                _ => Some(Regression),
            };
            match vita {
                Some(d) => Some(d),
                None if x.oracle != y.oracle || x.oracles != y.oracles || x.name != y.name => {
                    Some(OracleSide)
                }
                None => None,
            }
        }
        (State::Pinned(_), State::Excluded(x)) if vita_side(x.reason) => Some(Regression),
        (State::Pinned(_), State::Excluded(_)) => Some(OracleSide),
        (State::Pinned(_), State::Absent) => Some(Regression),
        (State::Excluded(_) | State::Absent, State::Pinned(_)) => Some(OracleSide),
        (State::Excluded(x), State::Excluded(y)) if x.reason != y.reason => Some(OracleSide),
        _ => None,
    }
}

/// The regression-direction moves that are not named in `accept`.
pub fn blocked<'a>(moves: &'a [Move], accept: &BTreeSet<String>) -> Vec<&'a Move> {
    moves
        .iter()
        .filter(|m| m.direction == Direction::Regression && !accept.contains(&m.path))
        .collect()
}

/// Read an `--accept-file`: one cell path per line; blank lines and `#` comments skip.
pub fn parse_accept_file(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Print every move, grouped by direction, and the names that matched no
/// regression-direction move.
pub fn print_moves(moves: &[Move], accept: &BTreeSet<String>) {
    let groups = [
        (
            Direction::Regression,
            "regression direction (written only when named)",
        ),
        (Direction::Improvement, "improvements"),
        (Direction::OracleSide, "oracle side changed, or entered"),
    ];
    println!("pin: {} moves against the existing manifest", moves.len());
    for (d, title) in groups {
        let ms: Vec<&Move> = moves.iter().filter(|m| m.direction == d).collect();
        println!("  {title}: {}", ms.len());
        for m in ms {
            let named = match (d, accept.contains(&m.path)) {
                (Direction::Regression, true) => "  [named]",
                (Direction::Regression, false) => "  [NOT NAMED]",
                _ => "",
            };
            println!("    {}  {} → {}{named}", m.path, m.from, m.to);
        }
    }
    let regression: BTreeSet<&str> = moves
        .iter()
        .filter(|m| m.direction == Direction::Regression)
        .map(|m| m.path.as_str())
        .collect();
    for a in accept {
        if !regression.contains(a.as_str()) {
            println!("  named, but not a regression-direction move: {a}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(path: &str, oracle: &str, expect: Expect<String>) -> Cell {
        Cell {
            path: path.into(),
            name: path.rsplit("__").next().unwrap().into(),
            oracles: vec!["iverilog".into(), "verilator".into()],
            oracle: oracle.into(),
            expect,
        }
    }
    const RUNS: Expect<String> = Expect::Runs { exit: 0 };
    fn kw(v: &str) -> Expect<String> {
        Expect::KnownWrong {
            exit: 0,
            vita: v.into(),
        }
    }
    fn refused(p: &str) -> Expect<String> {
        Expect::Refused { diag: p.into() }
    }
    fn excluded(path: &str, reason: Reason) -> Excluded {
        Excluded {
            path: path.into(),
            reason,
            detail: String::new(),
        }
    }
    const D11: &str = "AE/s588__docs__d11.sv";
    const E05: &str = "AE/s588__s0e__e05_ret_enum_bit4.sv";
    const E07: &str = "AE/s588__s0e__e07_ret_enum_nobase.sv";

    fn names(ns: &[&str]) -> BTreeSet<String> {
        ns.iter().map(|n| n.to_string()).collect()
    }

    /// The lens's scenario: a binary that prints d11's lines in another order, panics
    /// on e05 and prints `n=1` on e07. Re-pinned under it, the three right cells would
    /// leave the manifest or turn known-wrong and the gate would pass again. The guard
    /// blocks all three until each is named.
    #[test]
    fn a_regression_direction_repin_needs_naming() {
        let old = vec![
            cell(D11, "n=2\ndone", RUNS),
            cell(E05, "n=2\ndone", RUNS),
            cell(E07, "n=2\ndone", RUNS),
        ];
        let new = Pinned {
            cells: vec![cell(E07, "n=2\ndone", kw("n=1\ndone"))],
            excluded: vec![
                excluded(D11, Reason::OrderOnly),
                excluded(E05, Reason::VitaCrashAtPin),
            ],
        };
        let moves = compare(&old, &[], &new);
        assert_eq!(moves.len(), 3, "{moves:?}");
        assert!(moves.iter().all(|m| m.direction == Direction::Regression));
        assert_eq!(blocked(&moves, &BTreeSet::new()).len(), 3);
        assert_eq!(blocked(&moves, &names(&[D11, E05])).len(), 1);
        assert!(blocked(&moves, &names(&[D11, E05, E07])).is_empty());
    }

    /// Every regression-direction kind, one cell each.
    #[test]
    fn each_regression_direction_move_is_blocked() {
        let old = vec![
            cell("A/runs_kw.sv", "a", RUNS),
            cell("A/runs_ref.sv", "a", RUNS),
            cell("A/kw_kw.sv", "a", kw("b")),
            cell(
                "A/ref_ref.sv",
                "a",
                refused("1:1: error[VITA-E1] X: y\ncodes VITA-E1\nat 1:1"),
            ),
            cell("A/ref_kw.sv", "a", refused("p")),
            cell("A/nondet.sv", "a", RUNS),
            cell("A/gone.sv", "a", RUNS),
        ];
        let new = Pinned {
            cells: vec![
                cell("A/runs_kw.sv", "a", kw("b")),
                cell("A/runs_ref.sv", "a", refused("p")),
                cell("A/kw_kw.sv", "a", kw("c")),
                cell(
                    "A/ref_ref.sv",
                    "a",
                    refused("2:1: error[VITA-E1] X: y\ncodes VITA-E1\nat 2:1"),
                ),
                cell("A/ref_kw.sv", "a", kw("b")),
            ],
            excluded: vec![excluded("A/nondet.sv", Reason::VitaNondeterministicAtPin)],
        };
        let moves = compare(&old, &[], &new);
        let blocked: Vec<&str> = blocked(&moves, &BTreeSet::new())
            .iter()
            .map(|m| m.path.as_str())
            .collect();
        assert_eq!(
            blocked,
            [
                "A/gone.sv",
                "A/kw_kw.sv",
                "A/nondet.sv",
                "A/ref_kw.sv",
                "A/ref_ref.sv",
                "A/runs_kw.sv",
                "A/runs_ref.sv"
            ]
        );
    }

    /// A fix re-pinned: printed, never blocked.
    #[test]
    fn a_pure_improvement_repin_passes() {
        let old = vec![
            cell("A/kw_runs.sv", "a", kw("b")),
            cell("A/ref_runs.sv", "a", refused("p")),
            cell("A/kw_ref.sv", "a", kw("b")),
        ];
        let new = Pinned {
            cells: vec![
                cell("A/kw_runs.sv", "a", RUNS),
                cell("A/ref_runs.sv", "a", RUNS),
                cell("A/kw_ref.sv", "a", refused("p")),
            ],
            excluded: vec![],
        };
        let moves = compare(&old, &[], &new);
        assert_eq!(moves.len(), 3);
        assert!(moves.iter().all(|m| m.direction == Direction::Improvement));
        assert!(blocked(&moves, &BTreeSet::new()).is_empty());
    }

    /// With no manifest yet every cell enters: nothing to block.
    #[test]
    fn a_first_pin_needs_no_existing_manifest() {
        let new = Pinned {
            cells: vec![cell(D11, "n=2\ndone", RUNS), cell(E07, "n=2", kw("n=1"))],
            excluded: vec![excluded(E05, Reason::OracleRejects)],
        };
        let moves = compare(&[], &[], &new);
        assert!(moves.iter().all(|m| m.direction == Direction::OracleSide));
        assert!(blocked(&moves, &BTreeSet::new()).is_empty());
    }

    /// The oracles' answer changing, or a cell leaving for an oracle-side reason, is
    /// printed apart and not blocked; nothing at all is not a move.
    #[test]
    fn oracle_side_changes_are_printed_not_blocked() {
        let old = vec![
            cell("A/o.sv", "a", RUNS),
            cell("A/c5.sv", "a", refused("p")),
            cell("A/same.sv", "a", RUNS),
        ];
        let new = Pinned {
            cells: vec![cell("A/o.sv", "a2", RUNS), cell("A/same.sv", "a", RUNS)],
            excluded: vec![excluded("A/c5.sv", Reason::ForwardRefWithoutIverilog)],
        };
        let moves = compare(&old, &[], &new);
        assert_eq!(moves.len(), 2);
        assert!(moves.iter().all(|m| m.direction == Direction::OracleSide));
    }

    #[test]
    fn an_accept_file_is_one_path_per_line() {
        let got = parse_accept_file("# moves of round 3\nAE/a.sv\n\n  AE/b.sv  # why\n");
        assert_eq!(got, names(&["AE/a.sv", "AE/b.sv"]));
    }
}
