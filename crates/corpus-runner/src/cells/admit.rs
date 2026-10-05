//! Admission: which preserved cells have an oracle answer worth pinning.
//!
//! Mechanical, from the preserved captures alone — the slice's own `verdict:` word is
//! never read. Votes are counted per independent SIMULATOR, not per capture: iverilog
//! and sv2v-then-iverilog share iverilog's compiler back half and vvp, so they are one
//! simulator (`_slices/s588/files/g/run4.sh:9`); verilator and xcelium are one each.
//!
//! A cell is admitted when at least two simulators ran it to `rc=0` with equal
//! normalized output, and no capture dissents. A capture dissents when its tool ran and
//! REJECTED the design (an error diagnostic and a non-zero exit, verilator's failed
//! build included), or printed a different answer; a capture that failed in a way that
//! is not a verdict on the design — `sorry:`-only or `%Error-UNSUPPORTED`-only, an
//! internal error or crash, a hang — counts as absent. A failure this module cannot
//! classify excludes the cell. No majority: per ENGINEERING_RULES §7.3 an oracle split
//! is settled by the ladder, never by a vote.

use std::collections::BTreeMap;
use std::path::Path;

use super::expect_file;
use super::normalize::{normalized, same_but_for_whitespace, split_capture, Marker, Producer};

/// Why a preserved cell is not in the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// The `.expect` file does not parse.
    Unparseable,
    /// The cell needs files kept in its `.aux/` directory (an include, a second unit).
    NeedsAux,
    /// No `.sv`/`.v` beside the `.expect`.
    NoSource,
    /// An oracle ran the design and rejected it: a dissent.
    OracleRejects,
    /// An oracle exited non-zero in a way none of the rules classifies.
    OracleFailedUnclassified,
    /// One oracle's own runs of the cell disagree (two answers, or an answer and a
    /// failure).
    OracleSelfSplit,
    /// iverilog and sv2v-then-iverilog both ran cleanly and disagree.
    IverilogFamilySplit,
    /// Only iverilog and sv2v-then-iverilog ran cleanly: one simulator.
    SameFamilyOnly,
    /// Fewer than two simulators ran the cell cleanly.
    FewerThanTwoSimulators,
    /// Two simulators disagree, but only in whitespace or blank lines.
    HarnessFormat,
    /// iverilog and sv2v agree and verilator alone disagrees.
    OracleSplitVerilatorOnly,
    /// Two simulators disagree.
    OracleSplit,
    /// iverilog did not answer, and the source reads a `parameter` / `localparam`
    /// above its declaration: the forward-reference axis ROADMAP gives to iverilog,
    /// on which sv2v and verilator are disqualified ([`super::forward_ref`]).
    ForwardRefWithoutIverilog,
    /// The oracles agree that the design prints nothing: nothing to compare.
    EmptyOracleOutput,
    /// The classifying vita binary printed the oracles' lines in another order: an
    /// order the oracles may agree on by coincidence (ENGINEERING_RULES §7.3).
    OrderOnly,
    /// The classifying vita binary exited with neither 0 nor a refusal, or hung.
    VitaCrashAtPin,
    /// The classifying vita binary answered differently on two runs.
    VitaNondeterministicAtPin,
}

impl Reason {
    pub const ALL: [Reason; 17] = [
        Reason::Unparseable,
        Reason::NeedsAux,
        Reason::NoSource,
        Reason::OracleRejects,
        Reason::OracleFailedUnclassified,
        Reason::OracleSelfSplit,
        Reason::IverilogFamilySplit,
        Reason::SameFamilyOnly,
        Reason::FewerThanTwoSimulators,
        Reason::HarnessFormat,
        Reason::OracleSplitVerilatorOnly,
        Reason::OracleSplit,
        Reason::ForwardRefWithoutIverilog,
        Reason::EmptyOracleOutput,
        Reason::OrderOnly,
        Reason::VitaCrashAtPin,
        Reason::VitaNondeterministicAtPin,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Reason::Unparseable => "unparseable-expect",
            Reason::NeedsAux => "needs-aux",
            Reason::NoSource => "no-source",
            Reason::OracleRejects => "oracle-rejects",
            Reason::OracleFailedUnclassified => "oracle-failed-unclassified",
            Reason::OracleSelfSplit => "oracle-self-split",
            Reason::IverilogFamilySplit => "iverilog-family-split",
            Reason::SameFamilyOnly => "same-family-only",
            Reason::FewerThanTwoSimulators => "fewer-than-2-simulators",
            Reason::HarnessFormat => "harness-format",
            Reason::OracleSplitVerilatorOnly => "oracle-split-verilator-only",
            Reason::OracleSplit => "oracle-split",
            Reason::ForwardRefWithoutIverilog => "forward-ref-without-iverilog",
            Reason::EmptyOracleOutput => "empty-oracle-output",
            Reason::OrderOnly => "order-only",
            Reason::VitaCrashAtPin => "vita-crash-at-pin",
            Reason::VitaNondeterministicAtPin => "vita-nondeterministic-at-pin",
        }
    }

    pub fn from_label(s: &str) -> Option<Reason> {
        Reason::ALL.into_iter().find(|r| r.label() == s)
    }
}

/// What one preserved oracle capture says about the design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    /// Exited 0: its normalized answer.
    Clean(String),
    /// Ran and rejected the design: the first rejecting line.
    Rejects(String),
    /// Not a verdict on the design: `sorry:`-only / `%Error-UNSUPPORTED`-only.
    Unsupported,
    /// Not a verdict: an internal error, an assertion, a signal.
    Crashed,
    /// Not a verdict: killed by the runner's alarm or timeout.
    Hung,
    /// Exited non-zero and no rule above recognises why.
    Unclassified(String),
}

/// Classify one capture of `tool` (`iverilog`, `sv2v`, `verilator`, `xcelium`).
/// `None` when the capture has no runner marker, so it is not a run (a build log, a
/// table). A capture with any rejecting line is a rejection even when a crash follows:
/// when two readings are possible, the one that excludes the cell wins.
pub fn classify_capture(tool: &str, text: &str) -> Option<Capture> {
    let (marker, rc, before) = split_capture(text)?;
    let family = family_of(tool);
    if marker == Marker::Run && rc == 0 {
        let p = if family == "verilator" {
            Producer::Verilator
        } else {
            Producer::Iverilog
        };
        return Some(Capture::Clean(normalized(p, before)));
    }
    // The whole capture, the text after the marker included (sv2v's own stderr).
    let lines: Vec<&str> = text.split('\n').collect();
    let rejects = |l: &&&str| match family {
        "verilator" => is_verilator_rejection(l),
        _ => is_iverilog_family_rejection(l),
    };
    if let Some(l) = lines.iter().find(rejects) {
        return Some(Capture::Rejects(l.trim().to_string()));
    }
    let hung = rc == 142
        || rc == -9
        || lines
            .iter()
            .any(|l| *l == "TIMEOUT" || l.contains("Alarm clock"));
    if hung {
        return Some(Capture::Hung);
    }
    let crashed = matches!(rc, 133..=139)
        || lines.iter().any(|l| {
            l.contains("failed assertion")
                || l.starts_with("Assertion failed")
                || l.contains("Abort trap")
                || l.contains("Segmentation fault")
                || l.starts_with("%Error: Internal Error")
                || l.starts_with("%Error: Verilator internal fault")
                || l.contains("internal error")
        });
    if crashed {
        return Some(Capture::Crashed);
    }
    let unsupported = lines.iter().any(|l| match family {
        "verilator" => l.starts_with("%Error-UNSUPPORTED: "),
        _ => split_location(l).is_some_and(|w| w.starts_with("sorry: ")),
    });
    if unsupported {
        return Some(Capture::Unsupported);
    }
    let shown = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .map_or(String::new(), |l| l.trim().to_string());
    Some(Capture::Unclassified(format!("rc={rc}: {shown}")))
}

/// The independent simulator a tool belongs to.
pub fn family_of(tool: &str) -> &str {
    match tool {
        "iverilog" | "sv2v" => "iverilog",
        t => t,
    }
}

/// A line by which iverilog, vvp or sv2v rejects the design: a located `error:`, a
/// syntax error, sv2v's parse errors and messages, and the summaries that follow them.
fn is_iverilog_family_rejection(l: &str) -> bool {
    if l.starts_with("sv2v: ")
        || l == "I give up."
        || l == "Elaboration failed"
        || l.ends_with(" error(s) during elaboration.")
        || l.contains(": Program not runnable, ")
        || l.starts_with("ERROR: ")
        || l.starts_with("FATAL: ")
    {
        return true;
    }
    match split_location(l) {
        Some(w) => {
            let lower = w.to_ascii_lowercase();
            lower.starts_with("error: ")
                || lower.starts_with("syntax error")
                || lower.starts_with("syntax in ")
                || lower.starts_with("errors in ")
                || w.starts_with("Parse error: ")
        }
        None => false,
    }
}

/// A line by which verilator rejects the design: any `%Error` other than its
/// unsupported, internal-fault and summary lines; a run-time `[T] %Error`.
fn is_verilator_rejection(l: &str) -> bool {
    let at = l.find("%Error");
    let Some(at) = at else { return false };
    let head = &l[..at];
    if !(head.is_empty() || (head.starts_with('[') && head.trim_end().ends_with(']'))) {
        return false;
    }
    let rest = &l[at..];
    !(rest.starts_with("%Error-UNSUPPORTED: ")
        || rest.starts_with("%Error: Internal Error")
        || rest.starts_with("%Error: Verilator internal fault")
        || rest.starts_with("%Error: Command Failed ")
        || rest.starts_with("%Error: Cannot continue")
        || rest.starts_with("%Error: Exiting due to "))
}

/// What follows a `F:N: ` or `F:N:M: ` location, for iverilog's and sv2v's lines.
fn split_location(l: &str) -> Option<&str> {
    let (head, rest) = l.split_once(": ")?;
    let mut parts = head.split(':');
    let file = parts.next()?;
    let nums: Vec<&str> = parts.collect();
    let ok = !file.is_empty()
        && !file.chars().any(char::is_whitespace)
        && (1..=2).contains(&nums.len())
        && nums
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    ok.then_some(rest)
}

/// A cell whose oracles agree: it enters the manifest once vita has classified it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// `<row>/<cell>.sv` (or `.v`), relative to the cells directory.
    pub path: String,
    /// The file name the slice ran it under (the basename of `origin:`), so a run
    /// sees the same name the captures did.
    pub name: String,
    /// The oracles that ran it cleanly, sorted.
    pub oracles: Vec<String>,
    /// Their common normalized answer.
    pub oracle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Excluded {
    pub path: String,
    pub reason: Reason,
    pub detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Admission {
    pub candidates: Vec<Candidate>,
    pub excluded: Vec<Excluded>,
}

/// The row of a manifest path: `AE/s588__x.sv` → `AE`.
pub fn row_of(path: &str) -> &str {
    path.split('/').next().unwrap_or(path)
}

/// Read every `.expect` under `<cells>/<row>/` for each row, in file-name order.
pub fn admit(cells: &Path, rows: &[&str]) -> Result<Admission, String> {
    let mut out = Admission::default();
    let mut rows: Vec<&str> = rows.to_vec();
    rows.sort_unstable();
    rows.dedup();
    for row in rows {
        let dir = cells.join(row);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".expect"))
            .collect();
        names.sort_unstable();
        for n in names {
            let stem = &n[..n.len() - ".expect".len()];
            match admit_one(&dir, row, stem) {
                Ok(c) => out.candidates.push(c),
                Err(x) => out.excluded.push(x),
            }
        }
    }
    Ok(out)
}

/// One tool's standing on a cell, from all of its captures.
enum Standing {
    Clean(String),
    Rejects(String),
    Unclassified(String),
    SelfSplit(String),
    /// Ran only in ways that are not verdicts, or never ran.
    Absent(&'static str),
}

fn standing(caps: &[Capture]) -> Standing {
    if let Some(Capture::Rejects(l)) = caps.iter().find(|c| matches!(c, Capture::Rejects(_))) {
        return Standing::Rejects(l.clone());
    }
    if let Some(Capture::Unclassified(l)) =
        caps.iter().find(|c| matches!(c, Capture::Unclassified(_)))
    {
        return Standing::Unclassified(l.clone());
    }
    let mut answers: Vec<&String> = caps
        .iter()
        .filter_map(|c| match c {
            Capture::Clean(a) => Some(a),
            _ => None,
        })
        .collect();
    answers.sort_unstable();
    answers.dedup();
    let failures = caps
        .iter()
        .filter(|c| !matches!(c, Capture::Clean(_)))
        .count();
    match (answers.len(), failures) {
        (0, _) => Standing::Absent(if caps.iter().any(|c| *c == Capture::Unsupported) {
            "unsupported"
        } else if caps.iter().any(|c| *c == Capture::Crashed) {
            "crashed"
        } else if caps.iter().any(|c| *c == Capture::Hung) {
            "hung"
        } else {
            "no run"
        }),
        (1, 0) => Standing::Clean(answers[0].clone()),
        (n, f) => Standing::SelfSplit(format!("{n} answers, {f} failed runs")),
    }
}

fn admit_one(dir: &Path, row: &str, stem: &str) -> Result<Candidate, Excluded> {
    let src = ["sv", "v"]
        .iter()
        .map(|ext| format!("{stem}.{ext}"))
        .find(|f| dir.join(f).is_file());
    let path = format!("{row}/{}", src.as_deref().unwrap_or(stem));
    let excluded = |reason: Reason, detail: String| Excluded {
        path: path.clone(),
        reason,
        detail,
    };
    let bytes = std::fs::read(dir.join(format!("{stem}.expect")))
        .map_err(|e| excluded(Reason::Unparseable, e.to_string()))?;
    let file = expect_file::parse(&bytes).map_err(|e| excluded(Reason::Unparseable, e))?;
    if dir.join(format!("{stem}.aux")).exists() {
        return Err(excluded(Reason::NeedsAux, String::new()));
    }
    if src.is_none() {
        return Err(excluded(Reason::NoSource, String::new()));
    }
    let name = file
        .header("origin")
        .and_then(|o| o.rsplit('/').next())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| excluded(Reason::Unparseable, "no `origin:` header".into()))?
        .to_string();

    // Every run capture, per tool. A capture that is not UTF-8 cannot be read, so it
    // is a failure nobody classified.
    let mut caps: BTreeMap<&str, Vec<Capture>> = BTreeMap::new();
    for b in &file.blocks {
        let Some(tool) = b.oracle() else { continue };
        let c = match std::str::from_utf8(&b.body) {
            Ok(text) => classify_capture(tool, text),
            Err(_) => Some(Capture::Unclassified("not utf-8".into())),
        };
        if let Some(c) = c {
            caps.entry(tool).or_default().push(c);
        }
    }

    let mut clean: BTreeMap<&str, String> = BTreeMap::new();
    let mut absent: Vec<String> = Vec::new();
    let mut first_unclassified: Option<String> = None;
    for (tool, cs) in &caps {
        match standing(cs) {
            Standing::Rejects(l) => {
                return Err(excluded(Reason::OracleRejects, format!("{tool}: {l}")))
            }
            Standing::Unclassified(l) => {
                first_unclassified.get_or_insert(format!("{tool}: {l}"));
            }
            Standing::SelfSplit(d) => {
                return Err(excluded(Reason::OracleSelfSplit, format!("{tool}: {d}")))
            }
            Standing::Clean(a) => {
                clean.insert(tool, a);
            }
            Standing::Absent(why) => absent.push(format!("{tool} {why}")),
        }
    }
    if let Some(d) = first_unclassified {
        return Err(excluded(Reason::OracleFailedUnclassified, d));
    }
    for tool in ["iverilog", "sv2v", "verilator"] {
        if !caps.contains_key(tool) {
            absent.push(format!("{tool} no run"));
        }
    }
    absent.sort();

    // One vote per simulator; the iverilog family must agree with itself first.
    if let (Some(a), Some(b)) = (clean.get("iverilog"), clean.get("sv2v")) {
        if a != b {
            return Err(excluded(Reason::IverilogFamilySplit, String::new()));
        }
    }
    let mut votes: BTreeMap<&str, &String> = BTreeMap::new();
    for (tool, a) in &clean {
        votes.insert(family_of(tool), a);
    }
    if votes.len() < 2 {
        let reason = if clean.contains_key("iverilog") && clean.contains_key("sv2v") {
            Reason::SameFamilyOnly
        } else {
            Reason::FewerThanTwoSimulators
        };
        return Err(excluded(reason, absent.join("; ")));
    }
    let mut answers: Vec<&String> = votes.values().copied().collect();
    answers.sort_unstable();
    answers.dedup();
    let tools = clean.keys().copied().collect::<Vec<_>>().join(",");
    if answers.len() > 1 {
        if answers
            .iter()
            .all(|a| same_but_for_whitespace(a, answers[0]))
        {
            return Err(excluded(Reason::HarnessFormat, tools));
        }
        let verilator_only = votes.len() == 2
            && clean.contains_key("iverilog")
            && clean.contains_key("sv2v")
            && clean.contains_key("verilator");
        let reason = if verilator_only {
            Reason::OracleSplitVerilatorOnly
        } else {
            Reason::OracleSplit
        };
        return Err(excluded(reason, tools));
    }
    let answer = answers[0].clone();
    if answer.is_empty() {
        return Err(excluded(Reason::EmptyOracleOutput, String::new()));
    }
    if !clean.contains_key("iverilog") {
        let text = std::fs::read(dir.join(src.as_deref().unwrap_or(stem)))
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .map_err(|e| excluded(Reason::Unparseable, e.to_string()))?;
        if let Some((name, used, declared)) = super::forward_ref::forward_parameter_reference(&text)
        {
            return Err(excluded(
                Reason::ForwardRefWithoutIverilog,
                format!("`{name}` read at line {used}, declared at line {declared}"),
            ));
        }
    }
    Ok(Candidate {
        path,
        name,
        oracles: clean.keys().map(|t| t.to_string()).collect(),
        oracle: answer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(tool: &str, text: &str) -> Option<Capture> {
        classify_capture(tool, text)
    }

    /// Each class, on the shapes the preserved captures carry.
    #[test]
    fn a_capture_is_clean_a_rejection_or_absent() {
        assert_eq!(
            class("iverilog", "n=2\nx.sv:3: $finish called at 1 (1s)\nrc=0\n"),
            Some(Capture::Clean("n=2".into()))
        );
        // iverilog refuses a forward reference: a verdict, so a dissent.
        let unbound = "x.sv:3: error: Unable to bind parameter `K' in `top.gb'\n\
                       x.sv:6:      : A symbol with that name was declared here.\nrc=1\n";
        assert!(
            matches!(class("iverilog", unbound), Some(Capture::Rejects(l)) if l.contains("Unable to bind"))
        );
        assert!(matches!(
            class("iverilog", "x.sv:4: syntax error\nI give up.\nrc=2\n"),
            Some(Capture::Rejects(_))
        ));
        // sv2v's own messages follow the marker.
        assert!(matches!(
            class(
                "sv2v",
                "rc=1\nx.sv:8:8: Parse error: unexpected token '#'\n"
            ),
            Some(Capture::Rejects(_))
        ));
        assert!(matches!(
            class(
                "sv2v",
                "rc=1\nsv2v: import of pb::P conflicts\nCallStack (from HasCallStack):\n"
            ),
            Some(Capture::Rejects(_))
        ));
        // verilator's failed build is a run that rejected the design.
        let vl = "%Error: x.sv:5:23: Use of x/? constant in generate case statement\nbuild_rc=1\n";
        assert!(matches!(class("verilator", vl), Some(Capture::Rejects(_))));
        assert!(matches!(
            class(
                "verilator",
                "[0] %Error: x.sv:3: Assertion failed in top\nrc=1\n"
            ),
            Some(Capture::Rejects(_))
        ));
        // Not verdicts.
        assert_eq!(
            class(
                "iverilog",
                "x.sv:3: sorry: unpacked array parameters are not supported yet.\nrc=2\n"
            ),
            Some(Capture::Unsupported)
        );
        assert_eq!(class("verilator", "%Error-UNSUPPORTED: x.sv:4:5: Unsupported: Can't unroll\n%Error: Exiting due to 1 error(s)\nbuild_rc=1\n"), Some(Capture::Unsupported));
        assert_eq!(class("verilator", "%Error: Internal Error: x.sv:5:7: ../V3Number.cpp:1063: toUInt with 4-state\nbuild_rc=1\n"), Some(Capture::Crashed));
        assert_eq!(
            class(
                "iverilog",
                "x.sv:6: assert: eval_tree.cc:217: failed assertion rval.len() == wid\nrc=134\n"
            ),
            Some(Capture::Crashed)
        );
        assert_eq!(
            class(
                "sv2v",
                "...: line 9:  9057 Alarm clock: 14  perl -e \"alarm 30\"\nrc=142\n"
            ),
            Some(Capture::Hung)
        );
        // A rejection outranks the crash that follows it.
        let both = "%Error: x.sv:5:27: Replication value of < 0 or X/Z not legal\n\
                    %Error: Verilator internal fault, sorry.\nbuild_rc=139\n";
        assert!(matches!(
            class("verilator", both),
            Some(Capture::Rejects(_))
        ));
        // A failure no rule reads excludes the cell; a capture without a marker is no run.
        assert!(matches!(
            class("iverilog", "something odd\nrc=3\n"),
            Some(Capture::Unclassified(_))
        ));
        assert_eq!(class("verilator", "%Warning-X: x.sv:1:1: y\n"), None);
    }

    /// Votes are per simulator: iverilog and sv2v are one, and a member that rejects
    /// excludes the cell however many others agree.
    #[test]
    fn standing_counts_dissent() {
        let c = |a: &str| Capture::Clean(a.into());
        assert!(matches!(
            standing(&[c("a"), Capture::Hung]),
            Standing::SelfSplit(_)
        ));
        assert!(matches!(standing(&[c("a"), c("a")]), Standing::Clean(_)));
        assert!(matches!(
            standing(&[c("a"), Capture::Rejects("e".into())]),
            Standing::Rejects(_)
        ));
        assert!(matches!(
            standing(&[Capture::Unsupported]),
            Standing::Absent("unsupported")
        ));
        assert_eq!(family_of("sv2v"), family_of("iverilog"));
        assert_ne!(family_of("verilator"), family_of("iverilog"));
    }
}
