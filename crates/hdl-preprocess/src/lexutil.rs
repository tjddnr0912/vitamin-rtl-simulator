//! split part of `pp` (mechanical move).

use super::*;

/// Parse a `` `timescale `` argument string (`"1ns/100ps"`) into a [`TimeScale`].
/// Each side is `{1|10|100}{s|ms|us|ns|ps|fs}`; precision must be ≤ unit. Returns
/// `Err(message)` on any malformed field or a precision coarser than the unit.
pub fn parse_timescale(arg: &str) -> Result<TimeScale, String> {
    let mut parts = arg.split('/');
    let unit_s = parts.next().unwrap_or("").trim();
    let prec_s = parts.next().map(str::trim).unwrap_or("");
    if prec_s.is_empty() || parts.next().is_some() {
        return Err(format!("expected `unit/precision`, got `{}`", arg.trim()));
    }
    let unit_exp = parse_time_literal(unit_s)?;
    let prec_exp = parse_time_literal(prec_s)?;
    if prec_exp > unit_exp {
        return Err(format!(
            "time_precision ({prec_s}) coarser than time_unit ({unit_s})"
        ));
    }
    Ok(TimeScale { unit_exp, prec_exp })
}

/// Resolve each module's governing `` `timescale `` by file order. `modules` is
/// `(name, span_lo)` in EXPANDED-text coordinates; `regions` is the ascending-offset
/// table from [`PpResult::timescales`]. A module is governed by the LAST region whose
/// offset ≤ its `span_lo`; a module before any region (or a directive-free design)
/// uses the `1ns/1ns` base. `global_prec_exp` is the min precision across all modules
/// (the tick base). Empty `modules` ⇒ base precision.
pub fn resolve_module_timescales(
    modules: &[(&str, usize)],
    regions: &[(usize, TimeScale)],
) -> ResolvedTimescales {
    let mut unit_exp = std::collections::BTreeMap::new();
    let mut prec_exp = std::collections::BTreeMap::new();
    let mut precs: Vec<i8> = Vec::new();
    let mut default_used = false;
    for &(name, lo) in modules {
        let gov = regions
            .iter()
            .rev()
            .find(|(off, _)| *off <= lo)
            .map(|(_, ts)| *ts);
        let ts = match gov {
            Some(ts) => ts,
            None => {
                default_used = true;
                TimeScale::DEFAULT
            }
        };
        unit_exp.insert(name.to_string(), ts.unit_exp);
        prec_exp.insert(name.to_string(), ts.prec_exp);
        precs.push(ts.prec_exp);
    }
    let global_prec_exp = precs
        .into_iter()
        .min()
        .unwrap_or(TimeScale::DEFAULT.prec_exp);
    ResolvedTimescales {
        unit_exp,
        prec_exp,
        global_prec_exp,
        default_used,
    }
}

/// `{1|10|100}<s|ms|us|ns|ps|fs>` → base-10 exponent of seconds.
pub(crate) fn parse_time_literal(s: &str) -> Result<i8, String> {
    let digits_end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num, unit) = s.split_at(digits_end);
    let mantissa_exp: i8 = match num {
        "1" => 0,
        "10" => 1,
        "100" => 2,
        _ => return Err(format!("time mantissa must be 1/10/100, got `{num}`")),
    };
    let unit_exp: i8 = match unit.trim() {
        "s" => 0,
        "ms" => -3,
        "us" => -6,
        "ns" => -9,
        "ps" => -12,
        "fs" => -15,
        other => return Err(format!("unknown time unit `{other}`")),
    };
    Ok(mantissa_exp + unit_exp)
}

/// 1-based (line, col) of `byte` in `src`, col counting Unicode scalars from the
/// last newline. Mirrors `cli::byte_to_line_col` exactly so numbers agree.
pub fn byte_to_line_col(src: &str, byte: usize) -> (u32, u32) {
    // Clamp out-of-range, then floor to a UTF-8 char boundary so the
    // `src[last_nl..byte]` slice below can never split a multibyte scalar
    // (a resolved orig_byte can land mid-scalar). Identity on aligned input.
    let mut byte = byte.min(src.len());
    while byte > 0 && !src.is_char_boundary(byte) {
        byte -= 1;
    }
    let mut line: u32 = 1;
    let mut last_nl: usize = 0; // byte index just past the last '\n'
    for (i, c) in src.char_indices() {
        if i >= byte {
            break;
        }
        if c == '\n' {
            line += 1;
            last_nl = i + 1;
        }
    }
    let col = src[last_nl..byte].chars().count() as u32 + 1;
    (line, col)
}

// ─────────────────────────────────────────────────────────────────────────────
// Include resolution (injected; keeps std::fs out of the core for tests)
// ─────────────────────────────────────────────────────────────────────────────

/// Reads include files. Production uses `FsIncludeReader`; tests use an in-memory
/// shim. `read` returns the file's text and its canonical absolute path (for the
/// cycle guard) given a resolved path.
pub trait IncludeReader {
    /// Resolve `request` (the quoted include path) against `current_dir` then each
    /// `incdir`, returning (resolved_display_name, canonical_path, text) for the
    /// first that exists, or `Err(())` if none exists. `current_dir` is the
    /// directory of the file CURRENTLY being processed (`files[file].dir`), per the
    /// IEEE nested-include rule — the caller never passes the global entry dir for
    /// an included file. The returned canonical_path's parent becomes the new
    /// file's own `dir` (the search root for ITS nested includes), so this must
    /// canonicalize to a real absolute path.
    ///
    /// The `Err(())` signature is fixed by the preprocess spec §1: "not found" is a
    /// boolean condition; the diagnostic + message are synthesized by the caller.
    #[allow(clippy::result_unit_err)]
    fn resolve(
        &self,
        request: &str,
        current_dir: &Path,
        incdirs: &[PathBuf],
    ) -> Result<(String, PathBuf, String), ()>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry points
// ─────────────────────────────────────────────────────────────────────────────

/// Preprocess in-memory source. `base_dir` is the directory of `name` (used as the
/// first include search root). `name` is the display name of the entry file.
/// Uses `FsIncludeReader` for includes.
pub fn preprocess_str(base_dir: &Path, name: &str, src: &str, opts: &PreOpts) -> PpResult {
    preprocess_with(base_dir, name, src, opts, &FsIncludeReader)
}

/// Like `preprocess_str` but with an injected `IncludeReader` (testable in-memory).
pub fn preprocess_with(
    base_dir: &Path,
    name: &str,
    src: &str,
    opts: &PreOpts,
    reader: &dyn IncludeReader,
) -> PpResult {
    let mut pp = Preprocessor::new(base_dir, name, src, opts, reader);
    pp.run();
    pp.finish()
}

/// Preprocess MULTIPLE command-line source files as ONE compilation unit (G12).
///
/// Each source is registered as its OWN [`SourceFileEntry`] (FileId 0..N) — exactly
/// how `\`include` registers a file — so every segment carries its own FileId and
/// diagnostics resolve to the correct per-file name + local line, instead of the old
/// pre-concatenation that named `sources[0]` with a cumulative global line. Macros /
/// `\`define`s and the `\`ifdef stack persist ACROSS files (shared compilation unit),
/// preserving the concatenation's cross-file visibility. `sources` = (display-name,
/// text) pairs in command-line order. With a single source this is byte-identical to
/// [`preprocess_str`]. Out-of-band (the SourceMap is not in the frozen `.vu`), so no
/// `format_version` / schema-hash impact.
pub fn preprocess_sources(
    base_dir: &Path,
    sources: &[(String, String)],
    opts: &PreOpts,
) -> PpResult {
    preprocess_sources_with(base_dir, sources, opts, &FsIncludeReader)
}

/// Like [`preprocess_sources`] but with an injected [`IncludeReader`] (testable).
pub fn preprocess_sources_with(
    base_dir: &Path,
    sources: &[(String, String)],
    opts: &PreOpts,
    reader: &dyn IncludeReader,
) -> PpResult {
    assert!(
        !sources.is_empty(),
        "preprocess_sources requires at least one source"
    );
    // The first source seeds FileId(0) exactly as the single-file path.
    let mut pp = Preprocessor::new(base_dir, &sources[0].0, &sources[0].1, opts, reader);
    // Register the remaining command-line files as their own SourceFileEntry
    // (FileId 1..N) — mirroring `dir_include`'s registration — so segments they emit
    // resolve back to that file. `canon: None` matches the entry-file convention (the
    // CLI takes per-source digests separately; only `\`include`d files carry a canon).
    for (name, text) in &sources[1..] {
        pp.files.push(SourceFileEntry {
            name: name.clone(),
            text: text.clone(),
            canon: None,
            dir: base_dir.to_path_buf(),
        });
    }
    // Fusion guard: ensure every NON-final file's text ends in a newline so the last
    // token of one file can't fuse with the first of the next in the expanded buffer
    // (the old concatenation inserted the same separator). The final file is scanned
    // last, so it needs none.
    let n = sources.len();
    for f in pp.files.iter_mut().take(n.saturating_sub(1)) {
        if !f.text.ends_with('\n') {
            f.text.push('\n');
        }
    }
    pp.run_sources(n);
    pp.finish()
}

// ─────────────────────────────────────────────────────────────────────────────
// Lexical helpers (ASCII-delimiter invariant, see §2.3)
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

pub(crate) fn is_ident_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// Match a simple identifier `[A-Za-z_][A-Za-z0-9_$]*` starting at byte `i`.
/// Returns `(name, end)` or `None` if no identifier-start is present at `i`.
pub(crate) fn parse_ident(src: &str, i: usize) -> Option<(&str, usize)> {
    let bytes = src.as_bytes();
    if i >= bytes.len() || !is_ident_start(bytes[i]) {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() && is_ident_continue(bytes[j]) {
        j += 1;
    }
    Some((&src[i..j], j))
}

pub(crate) fn is_directive_kw(name: &str) -> bool {
    matches!(
        name,
        "define"
            | "undef"
            | "include"
            | "ifdef"
            | "ifndef"
            | "elsif"
            | "else"
            | "endif"
            | "timescale"
            | "default_nettype"
            | "celldefine"
            | "endcelldefine"
            | "resetall"
            | "line"
            | "pragma"
            | "begin_keywords"
            | "end_keywords"
            | "unconnected_drive"
            | "nounconnected_drive"
    )
}

/// Scan a string literal starting at the opening `"` (index `i`). Returns the byte
/// index just past the literal's logical end and whether it terminated on a `"`.
/// IEEE: strings never span newlines. A `\` escapes the next char ONLY when that
/// next char is not `\n`. On reaching a `\n` (bare or right after `\`), the string
/// is UNTERMINATED and ends AT the newline (the `\n` is not consumed). Returns
/// `(end_index, terminated_ok)` where `end_index` is the byte index just past the
/// closing `"` on success, or the byte index OF the `\n` (or EOF) on failure.
pub(crate) fn scan_string(src: &str, i: usize) -> (usize, bool) {
    let bytes = src.as_bytes();
    debug_assert_eq!(bytes[i], b'"');
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => return (j + 1, true),
            b'\n' => return (j, false),
            b'\\' => {
                // Escape: a `\` followed by a non-newline consumes that next char.
                // A `\` immediately followed by `\n` does NOT continue the string.
                if j + 1 < bytes.len() && bytes[j + 1] != b'\n' {
                    j += 2;
                } else {
                    // `\` then newline (or EOF): string ends unterminated at the NL.
                    let nl = j + 1;
                    return (nl.min(bytes.len()), false);
                }
            }
            _ => j += 1,
        }
    }
    (bytes.len(), false)
}

/// Index just past a line comment `//...` (NOT including the `\n`).
pub(crate) fn scan_line_comment(src: &str, i: usize) -> usize {
    let bytes = src.as_bytes();
    let mut j = i + 2;
    while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
    }
    j
}

/// Index just past a block comment `/* ... */` (including delimiters), or EOF.
pub(crate) fn scan_block_comment(src: &str, i: usize) -> usize {
    let bytes = src.as_bytes();
    let mut j = i + 2;
    while j + 1 < bytes.len() {
        if bytes[j] == b'*' && bytes[j + 1] == b'/' {
            return j + 2;
        }
        j += 1;
    }
    bytes.len()
}

/// Split actuals starting just after `open` (the index of '('). Per §2.4.
pub(crate) fn split_args(src: &str, open: usize) -> SplitArgs {
    let bytes = src.as_bytes();
    let mut i = open + 1;
    let mut depth_paren: u32 = 0;
    let mut depth_brack: u32 = 0;
    let mut depth_brace: u32 = 0;
    let mut cur = String::new();
    let mut args: Vec<String> = Vec::new();
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let (end, _ok) = scan_string(src, i);
                cur.push_str(&src[i..end]);
                i = end;
                continue;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                let end = scan_line_comment(src, i);
                cur.push_str(&src[i..end]);
                i = end;
                continue;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                let end = scan_block_comment(src, i);
                cur.push_str(&src[i..end]);
                i = end;
                continue;
            }
            b'(' => {
                depth_paren += 1;
                cur.push('(');
            }
            b'[' => {
                depth_brack += 1;
                cur.push('[');
            }
            b'{' => {
                depth_brace += 1;
                cur.push('{');
            }
            b')' => {
                if depth_paren == 0 && depth_brack == 0 && depth_brace == 0 {
                    args.push(cur.trim().to_string());
                    return SplitArgs {
                        actuals: args,
                        close: i,
                        closed: true,
                    };
                }
                // Unsigned-depth guard (§2.4 BLOCKER): saturating_sub so a top-level
                // unmatched `)` (legal literal text inside an actual) never underflows.
                depth_paren = depth_paren.saturating_sub(1);
                cur.push(')');
            }
            b']' => {
                // Same guard: a top-level unmatched `]` is literal, not a depth event.
                depth_brack = depth_brack.saturating_sub(1);
                cur.push(']');
            }
            b'}' => {
                // Same guard: a top-level unmatched `}` is literal, not a depth event.
                depth_brace = depth_brace.saturating_sub(1);
                cur.push('}');
            }
            b',' => {
                if depth_paren == 0 && depth_brack == 0 && depth_brace == 0 {
                    args.push(cur.trim().to_string());
                    cur = String::new();
                } else {
                    cur.push(',');
                }
            }
            _ => {
                // Copy this (possibly multibyte) char verbatim. The current byte is
                // ASCII (not a delimiter handled above), but advance by char to keep
                // slices on char boundaries for any following multibyte run.
                let ch_len = utf8_len(c);
                cur.push_str(&src[i..(i + ch_len).min(bytes.len())]);
                i += ch_len;
                continue;
            }
        }
        i += 1;
    }
    // EOF before a top-level ')'.
    args.push(cur.trim().to_string());
    SplitArgs {
        actuals: args,
        close: src.len(),
        closed: false,
    }
}

/// Byte length of a UTF-8 sequence given its lead byte.
pub(crate) fn utf8_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else if lead >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Substitute parameters into a macro body. Per §2.3 step 8: a mini-lexer that
/// recognizes the same verbatim contexts (strings/comments copied through, idents
/// inside them NOT substituted). Each maximal identifier run that exactly equals a
/// parameter name is replaced with the corresponding actual (raw text).
pub(crate) fn substitute(body: &str, params: &[String], actuals: &[String]) -> String {
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let (end, _ok) = scan_string(body, i);
                out.push_str(&body[i..end]);
                i = end;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                let end = scan_line_comment(body, i);
                out.push_str(&body[i..end]);
                i = end;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                let end = scan_block_comment(body, i);
                out.push_str(&body[i..end]);
                i = end;
            }
            // Token-paste ``` `` ``` (IEEE 1800 §22.5.2): delete the two-backtick operator
            // so the surrounding (already-substituted) tokens abut. iverilog does NOT
            // trim adjacent whitespace, so `a``b` => `ab` but `a `` b` => `a  b`.
            0x60 if i + 1 < bytes.len() && bytes[i + 1] == 0x60 => {
                i += 2;
            }
            // Stringification `"…`" (IEEE 1800 §22.5.1): emit a real `"`, substitute
            // params inside, and turn the embedded `\`"` escape into `\"`. Closing `"`
            // ends the literal. Unlike a real `"…"`, params ARE substituted inside.
            0x60 if i + 1 < bytes.len() && bytes[i + 1] == b'"' => {
                out.push('"');
                i += 2;
                loop {
                    if i >= bytes.len() {
                        break; // unterminated — emit what we have, the lexer will diag
                    }
                    let b = bytes[i];
                    if b == 0x60
                        && i + 3 < bytes.len()
                        && bytes[i + 1] == b'\\'
                        && bytes[i + 2] == 0x60
                        && bytes[i + 3] == b'"'
                    {
                        out.push_str("\\\""); // `\`"  =>  \"
                        i += 4;
                    } else if b == 0x60 && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                        out.push('"'); // closing `"
                        i += 2;
                        break;
                    } else if b == 0x60 && i + 1 < bytes.len() && bytes[i + 1] == 0x60 {
                        i += 2; // token-paste operator inside a stringify: delete it (iverilog parity)
                    } else if is_ident_start(b) {
                        let (name, end) = parse_ident(body, i).unwrap();
                        if let Some(idx) = params.iter().position(|p| p == name) {
                            out.push_str(actuals.get(idx).map(|s| s.as_str()).unwrap_or(""));
                        } else {
                            out.push_str(name);
                        }
                        i = end;
                    } else {
                        let ch_len = utf8_len(b);
                        out.push_str(&body[i..(i + ch_len).min(bytes.len())]);
                        i += ch_len;
                    }
                }
            }
            _ if is_ident_start(c) => {
                let (name, end) = parse_ident(body, i).unwrap();
                if let Some(idx) = params.iter().position(|p| p == name) {
                    out.push_str(actuals.get(idx).map(|s| s.as_str()).unwrap_or(""));
                } else {
                    out.push_str(name);
                }
                i = end;
            }
            _ => {
                let ch_len = utf8_len(c);
                out.push_str(&body[i..(i + ch_len).min(bytes.len())]);
                i += ch_len;
            }
        }
    }
    out
}

/// Join physical-line continuations (`\`+LF, `\`+CRLF) in ordinary (non-verbatim)
/// text, per §0 item 3. A `\`+NL inside a `"..."` is LEFT for the string scanner
/// (strings never silently join). Comments are copied through. Used when capturing
/// a logical directive/macro line.
pub(crate) fn join_continuations(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let (end, _ok) = scan_string(s, i);
                out.push_str(&s[i..end]);
                i = end;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                let end = scan_line_comment(s, i);
                out.push_str(&s[i..end]);
                i = end;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                let end = scan_block_comment(s, i);
                out.push_str(&s[i..end]);
                i = end;
            }
            b'\\' => {
                // `\`+LF or `\`+CRLF => drop both/all, joining the lines.
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    i += 2;
                } else if i + 2 < bytes.len() && bytes[i + 1] == b'\r' && bytes[i + 2] == b'\n' {
                    i += 3;
                } else {
                    out.push('\\');
                    i += 1;
                }
            }
            _ => {
                let ch_len = utf8_len(c);
                out.push_str(&s[i..(i + ch_len).min(bytes.len())]);
                i += ch_len;
            }
        }
    }
    out
}

/// Strip a trailing line comment from a captured logical line, per §2.5: within the
/// joined logical line a `//` truncates the body at its position (comment dropped).
/// String/block-comment contexts are respected so a `//` inside a string is kept.
pub(crate) fn strip_trailing_line_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'"' => {
                let (end, _ok) = scan_string(s, i);
                i = end;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                return &s[..i];
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i = scan_block_comment(s, i);
            }
            _ => i += utf8_len(c),
        }
    }
    s
}

/// Parse a parameter list starting at the `(` of `s`. Returns `(params, rest)`
/// where `rest` is the body text after the `)`. Rejects `=` defaults and duplicate
/// names (returns `Err(message)`).
pub(crate) fn parse_param_list(s: &str) -> Result<(Vec<String>, &str), &'static str> {
    debug_assert!(s.starts_with('('));
    let bytes = s.as_bytes();
    // Find the matching ')'.
    let mut depth = 0u32;
    let mut close = None;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let Some(close) = close else {
        return Err("`define parameter list is unterminated");
    };
    let inner = &s[1..close];
    let rest = &s[close + 1..];
    let mut params: Vec<String> = Vec::new();
    let inner_trim = inner.trim();
    if !inner_trim.is_empty() {
        for part in inner.split(',') {
            let p = part.trim();
            if p.contains('=') {
                return Err("`define default argument values are not supported");
            }
            let Some((nm, end)) = parse_ident(p, 0) else {
                return Err("`define parameter name is invalid");
            };
            if !p[end..].trim().is_empty() {
                return Err("`define parameter name is invalid");
            }
            if params.iter().any(|x| x == nm) {
                return Err("`define has a duplicate parameter name");
            }
            params.push(nm.to_string());
        }
    }
    Ok((params, rest))
}

/// Parse `s` requiring exactly one double-quoted token surrounded only by
/// whitespace/comments. Returns the inner bytes (quotes stripped) or `None`.
pub(crate) fn parse_single_quoted(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading ws/comments
    i = skip_ws_comments(s, i);
    if i >= bytes.len() || bytes[i] != b'"' {
        return None;
    }
    let (end, ok) = scan_string(s, i);
    if !ok {
        return None;
    }
    let inner = s[i + 1..end - 1].to_string();
    let j = skip_ws_comments(s, end);
    if j != bytes.len() {
        return None; // trailing tokens
    }
    Some(inner)
}

pub(crate) fn skip_ws_comments(s: &str, mut i: usize) -> usize {
    let bytes = s.as_bytes();
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i = scan_line_comment(s, i);
        } else if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i = scan_block_comment(s, i);
        } else {
            break;
        }
    }
    i
}
