//! Filelist (`-f` / `-F`) expansion — doc-14 §3.1, v1 subset.
//!
//! `.f` files are tokenized like argv and spliced IN PLACE at the reference
//! point (depth-first pre-order; the command line is frame 0), so every flag
//! legal on the command line is legal inside a `.f`. The expansion happens at
//! the argv level in `run()`, BEFORE per-applet flag parsing — staged applets
//! accept filelists for free.
//!
//! v1 scope: source paths + every existing CLI flag + nested `-f`/`-F`,
//! comments (`//`, `/* */`, leading `#`), `\`-continuation, `$VAR`/`${VAR}`/
//! `$(VAR)` env expansion (undefined = loud `E-FLIST-UNDEF-ENV`), glob
//! rejection (`E-FLIST-GLOB`), cycle (`E-FLIST-CYCLE`, lexical ∪ physical
//! identity) and depth (`E-FLIST-DEPTH`, cap 256) guards, and the
//! `W-FLIST-MIXED-BASE` lint (`-f` inside a `-F` frame re-anchors to CWD).
//! `+define+N=V+M` rides verbatim (macro text, never base-resolved) and
//! `+incdir+a+b` segments resolve against the frame base — both feed the
//! typed `PreOpts` surface via `parse_io_args` (`-D`/`-I` equivalents).
//!
//! Path policy (doc-14 "canonicalization 잠금"): identity for the cycle guard
//! uses pure LEXICAL `.`/`..` normalization — `fs::canonicalize` (symlink
//! resolution) is used only for the PHYSICAL identity key, which is
//! diagnostic-only and never reaches hashes or manifests.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use diag::{Diagnostic, LogEvent, LogSink, MsgCode, Severity};

/// Depth backstop (doc-14: cycle guard is primary; this catches pathology).
const MAX_DEPTH: u32 = 256;

/// Flags whose NEXT token is a value, not a source path — the value must not
/// be base-resolved (e.g. `--timeout 200`: "200" is not a path).
fn takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "-o" | "--out"
            | "--threads"
            | "-j"
            | "--timeout"
            | "-D"
            | "--define"
            | "-I"
            | "--incdir"
            | "-l"
            | "--log"
            | "--verbosity"
    )
}

/// True if `tok` smells like a glob — banned in filelists (readdir order is
/// platform-unstable, breaking deterministic flat order).
fn is_glob(tok: &str) -> bool {
    tok.contains('*') || tok.contains('?') || tok.contains('[')
}

/// Pure lexical `.`/`..` normalization (no fs access, no symlink resolution).
fn lexical_normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Physical identity key for the cycle guard (diagnostic-only — never hashed).
/// Unix: dev+inode; elsewhere: the OS-canonicalized path string.
fn phys_id(p: &Path) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(p)
            .ok()
            .map(|m| format!("{}:{}", m.dev(), m.ino()))
    }
    #[cfg(not(unix))]
    {
        std::fs::canonicalize(p)
            .ok()
            .map(|c| c.to_string_lossy().into_owned())
    }
}

/// Expand `$VAR` / `${VAR}` / `$(VAR)` in one token. An undefined variable is
/// a hard error (doc-14: silent empty-string substitution is banned).
fn expand_env(tok: &str) -> Result<String, String> {
    let mut out = String::with_capacity(tok.len());
    let b = tok.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'$' {
            out.push(b[i] as char);
            i += 1;
            continue;
        }
        // `$` introducer: ${NAME}, $(NAME), or $NAME (ident chars).
        let (name, next) = if i + 1 < b.len() && (b[i + 1] == b'{' || b[i + 1] == b'(') {
            let close = if b[i + 1] == b'{' { b'}' } else { b')' };
            let start = i + 2;
            let Some(endrel) = b[start..].iter().position(|&c| c == close) else {
                return Err(format!("unterminated environment reference in '{tok}'"));
            };
            (&tok[start..start + endrel], start + endrel + 1)
        } else {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                j += 1;
            }
            if j == start {
                // a lone `$` (e.g. an escaped-ident source path): keep verbatim.
                out.push('$');
                i += 1;
                continue;
            }
            (&tok[start..j], j)
        };
        match std::env::var(name) {
            Ok(v) => out.push_str(&v),
            Err(_) => return Err(format!("undefined environment variable '${name}'")),
        }
        i = next;
    }
    Ok(out)
}

/// Strip comments from a `.f` body: `/* */` blocks (non-nesting), then per
/// line `//`-tails and leading-`#` lines; join `\`-continuations.
fn strip_comments(body: &str) -> String {
    // block comments first (they may span lines).
    let mut no_block = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(open) = rest.find("/*") {
        no_block.push_str(&rest[..open]);
        match rest[open + 2..].find("*/") {
            Some(close) => rest = &rest[open + 2 + close + 2..],
            None => {
                rest = "";
            }
        }
        no_block.push(' '); // a comment is a separator, never a token joiner
    }
    no_block.push_str(rest);

    let mut out = String::with_capacity(no_block.len());
    for line in no_block.lines() {
        let line = match line.find("//") {
            Some(p) => &line[..p],
            None => line,
        };
        if line.trim_start().starts_with('#') {
            out.push('\n');
            continue;
        }
        if let Some(stripped) = line.trim_end().strip_suffix('\\') {
            out.push_str(stripped);
            out.push(' '); // continuation: join with the next line
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

struct Expander<'a> {
    sink: &'a dyn LogSink,
    cwd: PathBuf,
    /// Active lexical-canonical paths (cycle guard key 1).
    stack: Vec<PathBuf>,
    /// Active physical identities (cycle guard key 2 — catches symlink loops).
    phys: Vec<String>,
}

impl Expander<'_> {
    fn err(&self, code: MsgCode, msg: String) {
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Error,
            code,
            message: msg,
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }

    fn warn(&self, code: MsgCode, msg: String) {
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code,
            message: msg,
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }

    /// Resolve `tok` against `base` when relative; absolute stays as-is.
    fn resolve(&self, tok: &str, base: &Path) -> String {
        let p = Path::new(tok);
        if p.is_absolute() {
            tok.to_string()
        } else {
            lexical_normalize(&base.join(p))
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Expand one `.f` file into `out`. `base` anchors ITS relative paths
    /// (`-f` frames: invocation CWD; `-F` frames: the file's own directory —
    /// the base is a property of HOW the frame was entered, not inherited).
    /// `in_big_f` marks that any enclosing frame was `-F` (for the lint).
    fn expand_file(&mut self, path: &str, in_big_f: bool, out: &mut Vec<String>) -> Result<(), ()> {
        let canon = lexical_normalize(&if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.cwd.join(path)
        });
        let pid = phys_id(&canon);
        if self.stack.contains(&canon) || pid.as_ref().is_some_and(|p| self.phys.contains(p)) {
            let chain: Vec<String> = self
                .stack
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .chain([canon.to_string_lossy().into_owned()])
                .collect();
            self.err(
                MsgCode::FlistCycle,
                format!("filelist cycle: {}", chain.join(" -> ")),
            );
            return Err(());
        }
        if self.stack.len() as u32 >= MAX_DEPTH {
            self.err(
                MsgCode::FlistDepth,
                format!("filelist nesting exceeded {MAX_DEPTH} levels at '{path}'"),
            );
            return Err(());
        }
        let body = match std::fs::read_to_string(&canon) {
            Ok(b) => b,
            Err(e) => {
                self.err(
                    MsgCode::FlistNotFound,
                    format!("cannot read filelist '{path}': {e}"),
                );
                return Err(());
            }
        };
        self.stack.push(canon.clone());
        if let Some(p) = pid.clone() {
            self.phys.push(p);
        }
        let frame_dir = canon
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.cwd.clone());
        let r = self.expand_tokens(&strip_comments(&body), &frame_dir, in_big_f, out);
        self.stack.pop();
        if pid.is_some() {
            self.phys.pop();
        }
        r
    }

    /// Process one frame's token stream. `frame_dir` is the `.f`'s directory;
    /// `-f` targets/`-F` targets and bare source tokens resolve per spec.
    fn expand_tokens(
        &mut self,
        text: &str,
        frame_dir: &Path,
        in_big_f: bool,
        out: &mut Vec<String>,
    ) -> Result<(), ()> {
        // NOTE: inside a `-f` frame relative paths anchor to the invocation
        // CWD; inside a `-F` frame they anchor to the file's own directory.
        // (Owned clone: the recursive `expand_file` below needs `&mut self`.)
        let base: PathBuf = if in_big_f {
            frame_dir.to_path_buf()
        } else {
            self.cwd.clone()
        };
        let base: &Path = &base;
        let toks: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0;
        while i < toks.len() {
            let tok = match expand_env(toks[i]) {
                Ok(t) => t,
                Err(msg) => {
                    self.err(MsgCode::FlistUndefEnv, msg);
                    return Err(());
                }
            };
            match tok.as_str() {
                "-f" | "-F" => {
                    let Some(raw) = toks.get(i + 1) else {
                        self.err(
                            MsgCode::FlistNotFound,
                            format!("'{tok}' needs a filelist argument"),
                        );
                        return Err(());
                    };
                    let target = match expand_env(raw) {
                        Ok(t) => t,
                        Err(msg) => {
                            self.err(MsgCode::FlistUndefEnv, msg);
                            return Err(());
                        }
                    };
                    let nested_big = tok == "-F";
                    if in_big_f && !nested_big {
                        // `-f` inside a relocatable `-F` tree re-anchors to the
                        // invocation CWD — almost always a vendor bug.
                        self.warn(
                            MsgCode::FlistMixedBase,
                            format!(
                                "'-f {target}' inside a -F frame resolves against the \
                                 invocation CWD, not the filelist directory"
                            ),
                        );
                    }
                    // The nested target itself resolves against THIS frame's base.
                    let resolved = self.resolve(&target, base);
                    self.expand_file(&resolved, in_big_f || nested_big, out)?;
                    i += 2;
                }
                flag if takes_value(flag) => {
                    out.push(tok.clone());
                    if let Some(v) = toks.get(i + 1) {
                        match expand_env(v) {
                            Ok(t) => out.push(t),
                            Err(msg) => {
                                self.err(MsgCode::FlistUndefEnv, msg);
                                return Err(());
                            }
                        }
                    }
                    i += 2;
                }
                // `+define+N=V+M` rides verbatim — its segments are macro text,
                // NEVER paths (base resolution would corrupt them).
                t if t.starts_with("+define+") => {
                    out.push(tok.clone());
                    i += 1;
                }
                // `+incdir+a+b`: each '+'-joined segment IS a path — resolve
                // against this frame's base (a -F vendor tree stays relocatable).
                t if t.starts_with("+incdir+") => {
                    let mut joined = String::from("+incdir");
                    for seg in t["+incdir+".len()..].split('+').filter(|x| !x.is_empty()) {
                        if is_glob(seg) {
                            self.err(
                                MsgCode::FlistGlob,
                                format!("wildcard '{seg}' not allowed in a filelist"),
                            );
                            return Err(());
                        }
                        joined.push('+');
                        joined.push_str(&self.resolve(seg, base));
                    }
                    out.push(joined);
                    i += 1;
                }
                flag if flag.starts_with('-') && flag.len() > 1 => {
                    out.push(tok.clone());
                    i += 1;
                }
                src => {
                    if is_glob(src) {
                        self.err(
                            MsgCode::FlistGlob,
                            format!("wildcard '{src}' not allowed in a filelist"),
                        );
                        return Err(());
                    }
                    out.push(self.resolve(src, base));
                    i += 1;
                }
            }
        }
        Ok(())
    }
}

/// Expand every `-f`/`-F` in `args` in place (depth-first pre-order). Returns
/// the flat argv, or `Err(exit_code)` after the error diagnostic was emitted
/// (filelist failures are usage errors — doc-13 class 3).
pub(crate) fn expand_argv(args: &[String], sink: &dyn LogSink) -> Result<Vec<String>, i32> {
    if !args.iter().any(|a| a == "-f" || a == "-F") {
        return Ok(args.to_vec()); // fast path: nothing to expand
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut ex = Expander {
        sink,
        cwd,
        stack: Vec::new(),
        phys: Vec::new(),
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-f" | "-F" => {
                let Some(target) = args.get(i + 1) else {
                    ex.err(
                        MsgCode::FlistNotFound,
                        format!("'{}' needs a filelist argument", args[i]),
                    );
                    return Err(crate::EXIT_CLI_ERROR);
                };
                let target = match expand_env(target) {
                    Ok(t) => t,
                    Err(msg) => {
                        ex.err(MsgCode::FlistUndefEnv, msg);
                        return Err(crate::EXIT_CLI_ERROR);
                    }
                };
                // Command-line `-f`/`-F` targets resolve against the CWD.
                let resolved = ex.resolve(&target, &ex.cwd.clone());
                if ex
                    .expand_file(&resolved, args[i] == "-F", &mut out)
                    .is_err()
                {
                    return Err(crate::EXIT_CLI_ERROR);
                }
                i += 2;
            }
            _ => {
                out.push(args[i].clone());
                i += 1;
            }
        }
    }
    // doc-15 E8003 family: the SAME canonical source appearing twice in the
    // expansion dedups to its FIRST occurrence (a silent duplicate would
    // otherwise double-compile the unit → a confusing downstream E-DUP-UNIT).
    // v6 ⑤ CONFLICT arm: the tracked sticky context (RULE S) is the inherited
    // `` `timescale `` — if the dropped occurrence would have inherited a
    // DIFFERENT directive than the kept one, the dedup would silently change
    // meaning → hard error presenting both contexts. The context walk (which
    // reads file text) only runs when a duplicate actually exists.
    // Flags and their values are exempt (only source positionals dedup).
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut deduped = Vec::with_capacity(out.len());
    let mut skip_value = false;
    let mut has_dup = false;
    for tok in &out {
        if skip_value {
            skip_value = false;
            deduped.push(tok.clone());
            continue;
        }
        if tok.starts_with('-') || tok.starts_with('+') {
            skip_value = takes_value(tok);
            deduped.push(tok.clone());
            continue;
        }
        let resolved = lexical_normalize(&ex.cwd.join(tok));
        let key = phys_id(&resolved).unwrap_or_else(|| resolved.to_string_lossy().into_owned());
        if seen.insert(key) {
            deduped.push(tok.clone());
        } else {
            has_dup = true;
        }
    }
    if has_dup && !check_dup_contexts(&ex, &out) {
        return Err(crate::EXIT_CLI_ERROR);
    }
    Ok(deduped)
}

/// v6 ⑤ (E8003 CONFLICT arm): walk the PRE-dedup token stream in order,
/// tracking the sticky `` `timescale `` state over KEPT sources (the dedup'd
/// compilation that actually runs). The kept first occurrence records its
/// inherited context; a dropped duplicate compares its would-be inherited
/// context against it — different → `E-FLIST-DUP-CTX-CONFLICT`. Returns
/// false on conflict.
fn check_dup_contexts(ex: &Expander<'_>, toks: &[String]) -> bool {
    let mut first_ctx: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    let mut ts_state: Option<String> = None;
    let mut skip_value = false;
    let mut ok = true;
    for tok in toks {
        if skip_value {
            skip_value = false;
            continue;
        }
        if tok.starts_with('-') || tok.starts_with('+') {
            skip_value = takes_value(tok);
            continue;
        }
        let resolved = lexical_normalize(&ex.cwd.join(tok));
        let key = phys_id(&resolved).unwrap_or_else(|| resolved.to_string_lossy().into_owned());
        match first_ctx.get(&key) {
            None => {
                first_ctx.insert(key, (tok.clone(), ts_state.clone()));
                // a KEPT file's own directives govern what follows it.
                if let Ok(text) = std::fs::read_to_string(&resolved) {
                    if let Some(ts) = last_timescale_directive(&text) {
                        ts_state = Some(ts);
                    }
                }
            }
            Some((first_tok, first_inherited)) => {
                if *first_inherited != ts_state {
                    let show = |c: &Option<String>| {
                        c.clone().unwrap_or_else(|| "(base 1ns/1ns)".to_string())
                    };
                    ex.err(
                        MsgCode::FlistDupCtxConflict,
                        format!(
                            "{tok} included twice under differing sticky context: first ({first_tok}) inherits `{}`, duplicate inherits `{}`",
                            show(first_inherited),
                            show(&ts_state)
                        ),
                    );
                    ok = false;
                }
                // the duplicate is DROPPED — its directives never run.
            }
        }
    }
    ok
}

/// Last `` `timescale `` directive in `src`, comment- and string-aware (a
/// light scan — only the E8003 duplicate gate reads it, and only when a
/// duplicate exists). `None` ⇒ the file sets no timescale.
fn last_timescale_directive(src: &str) -> Option<String> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut last = None;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'`' if src[i..].starts_with("`timescale") => {
                let end = src[i..].find('\n').map(|e| i + e).unwrap_or(src.len());
                last = Some(src[i..end].trim().to_string());
                i = end;
            }
            _ => i += 1,
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_normalize_handles_dots() {
        assert_eq!(
            lexical_normalize(Path::new("/a/b/../c/./d.sv")),
            PathBuf::from("/a/c/d.sv")
        );
    }

    #[test]
    fn env_expansion_forms() {
        std::env::set_var("VITA_FL_T", "X");
        assert_eq!(expand_env("$VITA_FL_T/a").unwrap(), "X/a");
        assert_eq!(expand_env("${VITA_FL_T}b").unwrap(), "Xb");
        assert_eq!(expand_env("$(VITA_FL_T)c").unwrap(), "Xc");
        assert!(expand_env("$VITA_FL_NOPE_123").is_err());
        // a lone `$` stays verbatim (escaped-identifier paths).
        assert_eq!(expand_env("a$").unwrap(), "a$");
    }

    #[test]
    fn comments_and_continuation() {
        let s =
            strip_comments("a.sv // tail\n# whole line\nb.sv \\\n  c.sv\n/* block\nspan */d.sv\n");
        let toks: Vec<&str> = s.split_whitespace().collect();
        assert_eq!(toks, ["a.sv", "b.sv", "c.sv", "d.sv"]);
    }
}
