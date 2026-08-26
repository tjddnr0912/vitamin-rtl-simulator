//! `-v` effective-invocation echo (doc-13 bucket C).
//!
//! WHY THIS EXISTS. Under a Makefile or a wrapper script the arguments a human
//! reads (`+define+W=$(WIDTH)`, `-f $(RTL)/build.f`, `+SEED=$(SEED)`) and the
//! arguments the process receives are different texts, and only the second one
//! decided the run. By the time anything can be logged, the substitution is
//! gone: the shell expanded `$(…)`, the filelist expander spliced `-f` frames
//! away, and `VITA_THREADS` never appeared in argv at all. So a failing CI job
//! leaves a transcript that cannot answer "which `W` was compiled in?".
//!
//! `-v` prints the resolved answer as ordinary [`LogEvent::Progress`] lines,
//! which means the `--log` tee captures them through the SAME writer as the
//! diagnostics and `$display` output (doc-13 단일 writer): the log file is a
//! complete record of what ran, in emission order.
//!
//! Everything here is pure sink policy — never hashed into an artifact, never
//! reaching the golden IR. `-q` suppresses the terminal copy but not the log
//! copy, exactly like every other Progress event.

use super::*;

/// Value column. Widest label is `invocation:` (11) + one space.
const LABEL_W: usize = 12;

/// Soft right margin for wrapping a value list. Values wider than this on
/// their own (a long absolute path) are never split — a broken path is worse
/// than a long line.
const WRAP_AT: usize = 92;

/// Render one `label: v1 v2 …` row, wrapping the values at [`WRAP_AT`] with
/// continuation lines indented to the value column. Returns no lines when
/// `items` is empty, so an absent knob prints nothing rather than `(none)`.
fn row(label: &str, items: &[String]) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    // `saturating_sub`, not `-`: a label as long as the column would underflow.
    // A future over-long label then pushes its value one space out instead of
    // panicking — misaligned, never crashed.
    let pad = LABEL_W.saturating_sub(label.len() + 1).max(1);
    let mut cur = format!("{label}:{:width$}", "", width = pad);
    let mut empty = true;
    for it in items {
        // `empty` (not a length test) decides the break: a single item wider
        // than the margin still goes on its own line instead of being split.
        if !empty && cur.chars().count() + 1 + it.chars().count() > WRAP_AT {
            lines.push(std::mem::replace(&mut cur, " ".repeat(LABEL_W)));
            empty = true;
        }
        if !empty {
            cur.push(' ');
        }
        cur.push_str(it);
        empty = false;
    }
    lines.push(cur);
    lines
}

/// POSIX-quote one argv token so the echoed `invocation:` line can be pasted
/// back into a shell verbatim. Anything outside a conservative safe set is
/// single-quoted (with the `'\''` escape for embedded quotes).
fn shell_quote(tok: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "-_=+./:,@%^".contains(c);
    if !tok.is_empty() && tok.chars().all(safe) {
        return tok.to_string();
    }
    format!("'{}'", tok.replace('\'', r"'\''"))
}

/// Split argv into wrappable atoms, keeping every value-taking flag glued to
/// its value (`-D`, `-f`, `--top`, …). Wrapping between the two would put
/// `-D` at the end of one line and `W=32` at the start of the next, which
/// reads as a bare flag and a stray source file — the opposite of what the
/// echo is for. `-f`/`-F` are glued here too: the expander consumes them, so
/// [`filelist::takes_value`] deliberately excludes them.
fn argv_atoms(argv: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let tok = &argv[i];
        let glue = filelist::takes_value(tok) || tok == "-f" || tok == "-F";
        match argv.get(i + 1).filter(|_| glue) {
            Some(v) => {
                out.push(format!("{} {}", shell_quote(tok), shell_quote(v)));
                i += 2;
            }
            None => {
                out.push(shell_quote(tok));
                i += 1;
            }
        }
    }
    out
}

/// Where the effective thread count came from — the flag, the environment, or
/// the auto default. The provenance is the point: a `VITA_THREADS` exported
/// three Makefiles up is invisible in argv, and "why is this run using 4
/// threads" has no other answer in the transcript.
fn threads_row(opts: &VitaOpts) -> String {
    let n = resolve_threads(opts.threads);
    let src = if opts.threads.is_some() {
        "--threads"
    } else if std::env::var("VITA_THREADS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .is_some()
    {
        "VITA_THREADS"
    } else {
        "auto"
    };
    format!("{n} ({src})")
}

/// Environment variables that changed THIS run's behaviour, as `NAME=value`.
/// Only vars the code actually reads are listed, and only when set — an empty
/// row is omitted entirely, so the common case adds no noise.
fn env_row() -> Vec<String> {
    ["VITA_THREADS", "VITA_SVA_COLLAPSE"]
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| format!("{k}={v}")))
        .collect()
}

/// The `-v` effective-invocation block: what the process actually received,
/// after shell substitution, filelist expansion and environment lookup.
///
/// `sources` are the post-expansion inputs in command order (source files for
/// `vita`/`vcmp`, the `.vu`/`.velab` for the later stages); `extra` carries
/// applet-specific rows (`-L` libraries, `--work`, `--upstream`) that do not
/// live in [`VitaOpts`]. A blank line brackets the block so it reads as one
/// unit in a transcript that is otherwise a flat line stream.
pub(crate) fn echo_effective_invocation(
    sink: &dyn LogSink,
    sources: &[String],
    out: Option<&str>,
    opts: &VitaOpts,
    extra: &[(&str, Vec<String>)],
) {
    let mut lines: Vec<String> = Vec::new();
    if let Some(inv) = &opts.invocation {
        lines.extend(row("invocation", &argv_atoms(&inv.argv)));
        lines.extend(row("cwd", std::slice::from_ref(&inv.cwd)));
        lines.extend(row("filelists", &inv.filelists));
    }
    lines.extend(row("sources", sources));
    for (label, items) in extra {
        lines.extend(row(label, items));
    }
    lines.extend(row("incdirs", &opts.incdirs));
    let defines: Vec<String> = opts
        .defines
        .iter()
        .map(|(n, v)| {
            if v.is_empty() {
                n.clone()
            } else {
                format!("{n}={v}")
            }
        })
        .collect();
    lines.extend(row("defines", &defines));
    let plusargs: Vec<String> = opts.plusargs.iter().map(|p| format!("+{p}")).collect();
    lines.extend(row("plusargs", &plusargs));
    // The elaborate-stage knob, beside its compile-stage (`defines`) and run-stage
    // (`plusargs`) siblings. `-G` is the one flag whose effect is a DIFFERENT design,
    // so leaving it visible only inside the raw `invocation:` line — where a filelist
    // or an attached `-GW=8` spelling hides it — is exactly backwards for a row whose
    // job is to report effective values.
    let params: Vec<String> = opts
        .top_params
        .iter()
        .map(|(n, v)| format!("{n}={v}"))
        .collect();
    lines.extend(row("params", &params));
    lines.extend(row("tops", &opts.tops));
    if let Some(o) = out {
        lines.extend(row("output", std::slice::from_ref(&o.to_string())));
    }
    if let Some(d) = &opts.obs_dir {
        lines.extend(row("obs-dir", std::slice::from_ref(d)));
    }
    // R14: the profile is a knob that changes what `run.json` contains AND, in
    // its timed form, what the wall-clock fields beside it mean — so it belongs
    // in the block whose job is "which knobs decided this run". `timed` is
    // spelled out rather than implied by a second row, because a reader
    // comparing two transcripts needs to see WHICH of the two flags was used:
    // only the count-only one leaves `run.json` byte-reproducible.
    if let Some(cfg) = &opts.proc_profile {
        let mode = if cfg.timed {
            "counts+time (--obs-procs-time)"
        } else {
            "counts (--obs-procs)"
        };
        lines.extend(row("obs-procs", &[mode.to_string()]));
    }
    lines.extend(row("probes", &opts.probes));
    if let Some(t) = opts.time_limit {
        lines.extend(row("timeout", &[format!("{t} ticks")]));
    }
    lines.extend(row("threads", &[threads_row(opts)]));
    if let Some(l) = &opts.log {
        lines.extend(row("log", std::slice::from_ref(l)));
    }
    lines.extend(row("env", &env_row()));

    // Blank line above and below: the block is a header, not another progress
    // line, and a Makefile transcript is already dense.
    for msg in std::iter::once(String::new())
        .chain(lines)
        .chain(std::iter::once(String::new()))
    {
        sink.emit(LogEvent::Progress(diag::ProgressEvent { message: msg }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_wraps_at_the_margin_and_indents_continuations() {
        let items: Vec<String> = (0..12)
            .map(|i| format!("item{i:02}-{}", "x".repeat(8)))
            .collect();
        let lines = row("sources", &items);
        assert!(lines.len() > 1, "must wrap: {lines:#?}");
        // Every line puts its first value at the same column — that alignment
        // is what makes the block scannable.
        assert!(lines[0].starts_with("sources:"), "got {:?}", lines[0]);
        for l in &lines {
            assert_eq!(l.len() - l[LABEL_W..].len(), LABEL_W, "got {l:?}");
            assert!(!l[LABEL_W..].starts_with(' '), "value off-column: {l:?}");
        }
        for l in &lines[1..] {
            assert!(l.starts_with(&" ".repeat(LABEL_W)), "got {l:?}");
        }
        assert!(lines.iter().all(|l| l.chars().count() <= WRAP_AT));
    }

    #[test]
    fn one_over_long_value_is_never_split() {
        let long = "/".to_string() + &"d".repeat(200);
        let lines = row("cwd", std::slice::from_ref(&long));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].ends_with(&long));
    }

    #[test]
    fn an_empty_list_prints_no_row() {
        assert!(row("defines", &[]).is_empty());
    }

    #[test]
    fn a_flag_never_wraps_away_from_its_value() {
        let argv: Vec<String> = ["vita", "-D", "W=32", "-f", "b.f", "--top", "t", "x.sv"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            argv_atoms(&argv),
            ["vita", "-D W=32", "-f b.f", "--top t", "x.sv"]
        );
    }

    #[test]
    fn a_trailing_value_flag_does_not_steal_a_missing_argument() {
        let argv = vec!["vita".to_string(), "-D".to_string()];
        assert_eq!(argv_atoms(&argv), ["vita", "-D"]);
    }

    #[test]
    fn quoting_survives_a_shell_round_trip() {
        assert_eq!(shell_quote("+define+W=8"), "+define+W=8");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote(""), "''");
    }
}
