//! Normalization of simulator output, one rule set per tool.
//!
//! An oracle capture under `crates/testdata/cells/` is one tool's stdout and stderr
//! merged, with the slice runner's `rc=N` marker appended; a live vita run is read
//! from its stdout alone. [`normalize`] keeps what the design printed and drops the
//! tool's own report about the run. Each rule applies only to the tool that prints that
//! shape, so a design line that looks like another tool's report is kept:
//!
//! | Producer | Dropped |
//! |---|---|
//! | iverilog family (iverilog; sv2v, then iverilog) | `F:N: $finish called at T (U)`; `F:N: warning: …` compile warnings and their `F:N:    : …` continuation lines; `warning: verinum::…` |
//! | verilator | `%Warning-CODE: F:L:C: …` build warnings; `- F:N: Verilog $finish`; the `- S i m u l a t i o n   R e p o r t` banner; `- Verilator: …` |
//! | vita (stdout) | the last line, when it is `simulation ended (R) at time T` |
//!
//! then trims trailing blank lines (one runner wrote a newline before its marker).
//! The end time is dropped with the line that carries it, and is not compared: the
//! tools count it in different units (iverilog in the global precision, sv2v's
//! translation in `1s` because it drops `` `timescale ``, verilator in the caller's
//! time unit, rounded), so equal counts are not equal times.

/// Which tool printed a capture: the rules are scoped to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Producer {
    /// iverilog + vvp, and sv2v followed by iverilog + vvp — one simulator.
    Iverilog,
    Verilator,
    /// vita's stdout.
    Vita,
}

/// The design's own output lines, in order.
pub fn normalize(p: Producer, text: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') || text.is_empty() {
        lines.pop();
    }
    if p == Producer::Vita && lines.last().is_some_and(|l| is_vita_end_anchor(l)) {
        lines.pop();
    }
    let mut kept: Vec<&str> = lines
        .into_iter()
        .filter(|l| match p {
            Producer::Iverilog => !(is_vvp_finish(l) || is_iverilog_warning(l)),
            Producer::Verilator => !(l.starts_with("- Verilator: ") || is_verilator_report(l)),
            Producer::Vita => true,
        })
        .collect();
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }
    kept
}

/// [`normalize`], joined with `\n`: the form a cell pins and compares. Unambiguous,
/// because no kept line contains a newline and trailing blank lines are gone.
pub fn normalized(p: Producer, text: &str) -> String {
    normalize(p, text).join("\n")
}

/// vvp's `F:N: $finish called at T (U)`.
fn is_vvp_finish(line: &str) -> bool {
    let Some((_, what)) = split_file_line(line) else {
        return false;
    };
    let Some(rest) = what.strip_prefix("$finish called at ") else {
        return false;
    };
    let Some((t, unit)) = rest.split_once(' ') else {
        return false;
    };
    !t.is_empty()
        && t.bytes().all(|b| b.is_ascii_digit())
        && unit.starts_with('(')
        && unit.ends_with(')')
}

/// iverilog's `F:N: warning: …`, its `F:N:    : …` continuation, `warning: verinum::…`.
fn is_iverilog_warning(line: &str) -> bool {
    if line.starts_with("warning: verinum::") {
        return true;
    }
    match split_file_line(line) {
        Some((_, what)) => {
            what.starts_with("warning: ")
                || (what.starts_with(' ') && what.trim_start().starts_with(": "))
        }
        None => false,
    }
}

/// verilator's report lines other than `- Verilator: …`, and its build warnings.
fn is_verilator_report(line: &str) -> bool {
    if line.starts_with("- S i m u l a t i o n   R e p o r t: ") {
        return true;
    }
    if let Some(rest) = line.strip_prefix("- ") {
        if let Some((_, what)) = split_file_line(rest) {
            return what == "Verilog $finish";
        }
    }
    // `%Warning-WIDTHEXPAND: x.sv:5:13: Operator …` — a code, then a location.
    if let Some(rest) = line.strip_prefix("%Warning-") {
        let Some((code, loc)) = rest.split_once(": ") else {
            return false;
        };
        let code_ok = !code.is_empty()
            && code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
        return code_ok && strip_location(loc).is_some();
    }
    false
}

/// `F:N: rest` with `F` free of whitespace and `N` all digits; returns `(F, rest)`.
/// `rest` is everything after the first `": "` — which for a continuation line is the
/// run of spaces before its second colon.
fn split_file_line(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let (file, after) = line.split_at(colon);
    if file.is_empty() || file.chars().any(char::is_whitespace) {
        return None;
    }
    let after = &after[1..];
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = after[digits..].strip_prefix(':')?;
    // `F:N: what` (vvp) or `F:N:      : what` (a continuation) — both start with a
    // space; `F:N:M: …` (a column) does not, and is not this shape.
    if !rest.starts_with(' ') {
        return None;
    }
    Some((file, &rest[1..]))
}

/// vita's `simulation ended (Finish) at time 40`.
fn is_vita_end_anchor(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("simulation ended (") else {
        return false;
    };
    let Some((reason, t)) = rest.split_once(") at time ") else {
        return false;
    };
    !reason.is_empty()
        && reason.bytes().all(|b| b.is_ascii_alphabetic())
        && !t.is_empty()
        && t.bytes().all(|b| b.is_ascii_digit())
}

/// One vita diagnostic line, `[F:L:C: ]sev[VITA-XNNNN] MNEMONIC: message`; returns
/// `(sev, "VITA-XNNNN")`.
pub fn vita_diagnostic(line: &str) -> Option<(&str, &str)> {
    let body = strip_location(line).map_or(line, |(_, rest)| rest);
    let open = body.find("[VITA-")?;
    let sev = &body[..open];
    if !matches!(sev, "note" | "info" | "warning" | "error" | "fatal") {
        return None;
    }
    let close = open + body[open..].find(']')?;
    let code = &body[open + 1..close];
    if code.len() <= "VITA-".len() || code.chars().any(char::is_whitespace) {
        return None;
    }
    // ` MNEMONIC: ` follows the code.
    let after = body[close + 1..].strip_prefix(' ')?;
    let colon = after.find(": ")?;
    if colon == 0 || after[..colon].chars().any(char::is_whitespace) {
        return None;
    }
    Some((sev, code))
}

/// Split `F:L:C: rest` into (`L:C: rest` with the file stripped, `rest`).
fn strip_location(line: &str) -> Option<(&str, &str)> {
    let end = line.find(": ")?;
    let mut it = line[..end].rsplitn(3, ':');
    let (col, ln, file) = (it.next()?, it.next()?, it.next()?);
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if digits(col) && digits(ln) && !file.is_empty() && !file.chars().any(char::is_whitespace) {
        Some((&line[file.len() + 1..], &line[end + 2..]))
    } else {
        None
    }
}

/// What a refused cell pins, from vita's stderr, as three lines compared by equality:
///
/// - the FIRST error or fatal diagnostic exactly, with only its file name stripped
///   (line, column, code, mnemonic and message kept);
/// - `codes` and the sorted set of every error and fatal code the run emitted;
/// - `at` and the sorted set of every error's `line:col` (`-` when none is located).
///
/// So a refusal that moves line, wording or arm — including a later error that moves
/// while the first stays — or that gains or loses an error code, is a drift. `None`
/// when vita printed no error.
pub fn refusal_pin(stderr: &str) -> Option<String> {
    let errors: Vec<(&str, &str)> = stderr
        .split('\n')
        .filter_map(|l| {
            vita_diagnostic(l)
                .filter(|(sev, _)| matches!(*sev, "error" | "fatal"))
                .map(|(_, code)| (l.trim_end(), code))
        })
        .collect();
    let (first, _) = errors.first()?;
    let first = strip_location(first).map_or(*first, |(l, _)| l);
    let mut codes: Vec<&str> = errors.iter().map(|(_, c)| *c).collect();
    codes.sort_unstable();
    codes.dedup();
    let mut at: Vec<(u32, u32)> = errors
        .iter()
        .filter_map(|(l, _)| {
            let (loc, _) = strip_location(l)?;
            let (ln, rest) = loc.split_once(':')?;
            let col = rest.split(':').next()?;
            Some((ln.parse().ok()?, col.parse().ok()?))
        })
        .collect();
    at.sort_unstable();
    at.dedup();
    let at = if at.is_empty() {
        "-".to_string()
    } else {
        at.iter()
            .map(|(l, c)| format!("{l}:{c}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    Some(format!("{first}\ncodes {}\nat {at}", codes.join(" ")))
}

/// Split one preserved capture at the slice runner's exit marker: the LAST line that
/// is exactly `rc=N`, or `build_rc=N` (a verilator build that failed). Returns the
/// marker's kind, `N` and the text before that line; what follows it (one runner
/// appended sv2v's stderr there) is not the simulation's output. `None` when the
/// capture has no marker — a build log or a result table — so it is not a run.
pub fn split_capture(text: &str) -> Option<(Marker, i32, &str)> {
    let mut start = 0usize;
    let mut found = None;
    for line in text.split('\n') {
        let (kind, n) = match line.strip_prefix("build_rc=") {
            Some(n) => (Marker::Build, n),
            None => match line.strip_prefix("rc=") {
                Some(n) => (Marker::Run, n),
                None => {
                    start += line.len() + 1;
                    continue;
                }
            },
        };
        let digits = n.strip_prefix('-').unwrap_or(n);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(rc) = n.parse::<i32>() {
                found = Some((kind, rc, start));
            }
        }
        start += line.len() + 1;
    }
    found.map(|(k, rc, at)| (k, rc, &text[..at]))
}

/// Which marker ended a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// `rc=N`: the run (or the compile before it) exited `N`.
    Run,
    /// `build_rc=N`: verilator's build failed; nothing ran.
    Build,
}

/// Two answers equal once runs of whitespace are collapsed and blank lines dropped:
/// what "disagree only on format" means for an oracle pair (`harness-format`).
pub fn same_but_for_whitespace(a: &str, b: &str) -> bool {
    let squash = |s: &str| -> Vec<String> {
        s.split('\n')
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    };
    squash(a) == squash(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runner_marker_is_the_last_rc_line_and_what_follows_it_is_cut() {
        assert_eq!(split_capture("a\nrc=0\n"), Some((Marker::Run, 0, "a\n")));
        assert_eq!(
            split_capture("a\nrc=0\nsv2v: warning\n"),
            Some((Marker::Run, 0, "a\n")),
            "sv2v's stderr after the marker is not the run"
        );
        assert_eq!(
            split_capture("rc=7\nb\nrc=-9\n"),
            Some((Marker::Run, -9, "rc=7\nb\n"))
        );
        assert_eq!(
            split_capture("%Error: x\nbuild_rc=1\n"),
            Some((Marker::Build, 1, "%Error: x\n"))
        );
        assert_eq!(split_capture("no marker\n"), None);
        assert_eq!(split_capture("rc=\nrc=x1\n"), None);
    }

    #[test]
    fn trailing_blank_lines_go_interior_ones_stay() {
        assert_eq!(normalize(Producer::Vita, "a\n\nb\n\n  \n"), ["a", "", "b"]);
        assert!(normalize(Producer::Iverilog, "").is_empty());
        assert_eq!(
            normalized(Producer::Vita, "a\r\nb\n"),
            "a\r\nb",
            "a CR is the design's"
        );
    }

    /// The run's end, each tool's way, is dropped — and only the shape that tool
    /// prints, so the design may print the same words.
    #[test]
    fn the_end_report_is_dropped_not_compared() {
        let iv = "n=2\nd11.sv:11: $finish called at 7 (1s)\n";
        let vl = "n=2\n- d11.sv:11: Verilog $finish\n\
                  - S i m u l a t i o n   R e p o r t: Verilator 5.052 2026-09-05\n\
                  - Verilator: $finish at 7ps; walltime 0.000 s; speed 27.027 ns/s\n\
                  - Verilator: cpu 0.000 s on 1 threads; allocated 2 MB\n";
        let vita = "n=2\nsimulation ended (Finish) at time 7000\n";
        for (p, t) in [
            (Producer::Iverilog, iv),
            (Producer::Verilator, vl),
            (Producer::Vita, vita),
        ] {
            assert_eq!(normalized(p, t), "n=2", "{p:?}");
        }
        assert_eq!(
            normalized(
                Producer::Vita,
                "n=2\nsimulation ended (Quiescent) at time 7\n"
            ),
            "n=2"
        );
        // Only vita's LAST line is its anchor; the design may print the words earlier.
        assert_eq!(
            normalized(Producer::Vita, "simulation ended (Finish) at time 1\nn=2\n"),
            "simulation ended (Finish) at time 1\nn=2"
        );
    }

    /// Each rule is the producer's own: a design line shaped like ANOTHER tool's report
    /// is the design's.
    #[test]
    fn a_rule_applies_only_to_its_own_tool() {
        let vita_like = "simulation ended (Finish) at time 3";
        let vvp_like = "x.sv:3: $finish called at 3 (1s)";
        let vl_like = "%Warning-WIDTH: x.sv:3:4: Operator";
        let iv_warn = "x.sv:3: warning: implicit";
        assert_eq!(
            normalized(Producer::Verilator, &format!("{vvp_like}\n{iv_warn}\n")),
            format!("{vvp_like}\n{iv_warn}")
        );
        assert_eq!(
            normalized(Producer::Iverilog, &format!("{vl_like}\n{vita_like}\n")),
            format!("{vl_like}\n{vita_like}")
        );
        assert_eq!(
            normalized(
                Producer::Vita,
                &format!("{vl_like}\n{iv_warn}\n{vvp_like}\nk\n")
            ),
            format!("{vl_like}\n{iv_warn}\n{vvp_like}\nk")
        );
        // verilator's warning rule needs a code and a location.
        assert_eq!(
            normalized(Producer::Verilator, "%Warning-USER: x=5\n"),
            "%Warning-USER: x=5"
        );
        assert_eq!(
            normalized(Producer::Verilator, &format!("{vl_like}\nk\n")),
            "k"
        );
    }

    #[test]
    fn a_refusal_pins_the_first_error_line_and_every_error_code() {
        let err = "x.sv:1:1: warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no\n\
                   x.sv:3:5: error[VITA-E3009] E-ELAB-X: why: because [in top]\n\
                   error[VITA-E2002] E-OTHER: later\n\
                   x.sv:9:1: error[VITA-E3009] E-ELAB-X: again\n\
                   errors=3 warnings=1 notes=0\n";
        assert_eq!(
            refusal_pin(err).as_deref(),
            Some(
                "3:5: error[VITA-E3009] E-ELAB-X: why: because [in top]\n\
                 codes VITA-E2002 VITA-E3009\nat 3:5 9:1"
            )
        );
        assert_eq!(
            refusal_pin("fatal[VITA-F4004] F-RUN-FATAL: stop\n").as_deref(),
            Some("fatal[VITA-F4004] F-RUN-FATAL: stop\ncodes VITA-F4004\nat -")
        );
        // §2 🆕 V's Z3e under mutant A2 (sign extension off): the first error and the
        // code set stay, a later error's position moves from 3:60 to 4:52.
        let head = "Z3e.sv:3:9: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: generate-if condition is not a constant: undefined name `K` is not a constant [in top.gb]\n\
                    Z3e.sv:1:8: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.gb[0].g[0].w` [in top.gb.g]\n\
                    Z3e.sv:3:60: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.gb[0].g[0].w` [in top.gb.g]\n";
        let a2 = head.replace("3:60:", "4:52:");
        assert_ne!(refusal_pin(head), refusal_pin(&a2));
        assert_eq!(refusal_pin("warning[VITA-W1] W-X: only a warning\n"), None);
        assert_eq!(refusal_pin("error: not vita's\n"), None);
    }

    #[test]
    fn whitespace_only_differences_are_format() {
        assert!(same_but_for_whitespace("a  = 1\n\nb", "a = 1\nb"));
        assert!(!same_but_for_whitespace("a=1", "a=2"));
        assert!(!same_but_for_whitespace("a=1\nb=2", "b=2\na=1"));
    }
}
