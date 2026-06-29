//! hdl-preprocess — Verilog-2005 MVP preprocessor.
//!
//! Runs before the lexer: raw source -> expanded text + SourceMap -> lex -> parse.
//! Pure text-to-text transform plus a byte-offset provenance map. std-only + `diag`.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use diag::{MsgCode, Severity, SourceLoc};

// ─────────────────────────────────────────────────────────────────────────────
// IDs
// ─────────────────────────────────────────────────────────────────────────────

/// Index into `SourceMap.files`. 0 is always the top-level entry file.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileId(pub u32);

// ─────────────────────────────────────────────────────────────────────────────
// Options
// ─────────────────────────────────────────────────────────────────────────────

/// Preprocessor options. Constructed by the CLI from argv (-I incdirs, -D defines).
#[derive(Clone, Debug)]
pub struct PreOpts {
    /// Include search directories, tried in order after the current-file directory.
    pub incdirs: Vec<PathBuf>,
    /// Command-line `-D NAME` / `-D NAME=text` predefined object-like macros.
    /// Empty text => empty body (definedness only).
    pub cli_defines: Vec<(String, String)>,
    /// Hard cap on macro-expansion nesting depth (recursion backstop in addition
    /// to the active-set guard). Default 256.
    pub max_macro_depth: u32,
    /// Hard cap on include nesting depth (in addition to the cycle guard). Default 64.
    pub max_include_depth: u32,
    /// Cumulative cap on TOTAL expanded output bytes (PP-FANOUT-CAP). The depth
    /// guard bounds nesting but NOT fan-out: chained doubling macros
    /// (`` `Mi = `Mi-1 `Mi-1 ``) materialize 2^N copies at depth N, so a ~30-line
    /// file can OOM (≈8 GiB at N=24) before parse. This bounds the expansion so
    /// such a file fails loud instead. Default 256 MiB — generous for any real
    /// design, and well under `u32::MAX` so the `as u32` segment offsets (which
    /// index `self.out`) can never wrap.
    pub max_output_bytes: usize,
}

impl Default for PreOpts {
    fn default() -> Self {
        PreOpts {
            incdirs: Vec::new(),
            cli_defines: Vec::new(),
            max_macro_depth: 256,
            max_include_depth: 64,
            max_output_bytes: 256 * 1024 * 1024,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostics
// ─────────────────────────────────────────────────────────────────────────────

/// A preprocessor diagnostic. `at` is a byte offset into the **expanded** output
/// (so it resolves through `SourceMap::resolve`). For errors detected while a
/// region has not yet been emitted (e.g. an unterminated `ifdef` at EOF), `at`
/// points at the offset where emission stopped (clamped to `expanded.len()`).
#[derive(Clone, Debug)]
pub struct PpDiag {
    pub code: MsgCode,
    pub severity: Severity,
    pub message: String,
    /// Byte offset into the expanded text, for SourceMap resolution.
    pub at: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Result
// ─────────────────────────────────────────────────────────────────────────────

/// Result of preprocessing. `text` is the lexer/parser input. `map` translates
/// expanded-byte offsets back to original (file, line, col, byte). `diags` carries
/// errors and warnings; the CLI decides exit codes from `severity`.
#[derive(Debug)]
pub struct PpResult {
    pub text: String,
    pub map: SourceMap,
    pub diags: Vec<PpDiag>,
    /// `` `timescale `` regions in EXPANDED-text coordinates: `(from_offset, ts)`,
    /// source order. Each entry takes effect from `from_offset` until the next
    /// entry (file-order inheritance). A module is governed by the LAST entry whose
    /// `from_offset ≤ module.span.lo`. Empty ⇒ no directive seen (caller applies the
    /// `1ns/1ns` base + `W-PP-TIMESCALE-DEFAULT`).
    pub timescales: Vec<(usize, TimeScale)>,
}

/// A `` `timescale unit/precision `` value as base-10 exponents of SECONDS, e.g.
/// `1ns` → -9, `100ps` → -10, `10ns` → -8, `1ps` → -12. The unit/precision ratio
/// (`unit_exp - prec_exp`, always ≥ 0) is the per-module delay multiplier; the
/// design-wide `min(prec_exp)` defines the global tick base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeScale {
    pub unit_exp: i8,
    pub prec_exp: i8,
}

impl TimeScale {
    /// The `1ns/1ns` no-timescale base (doc-08 lock).
    pub const DEFAULT: TimeScale = TimeScale {
        unit_exp: -9,
        prec_exp: -9,
    };
}

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

/// Per-module timescale resolution result (S2). Plain types so `elaborate` need
/// not depend on this crate: the glue passes `unit_exp` + `global_prec_exp` in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedTimescales {
    /// module NAME → its delay-unit exponent (`unit_exp`). The per-module delay
    /// multiplier is `10^(unit_exp − global_prec_exp)`.
    pub unit_exp: std::collections::BTreeMap<String, i8>,
    /// design-wide FINEST precision exponent = the global tick base.
    pub global_prec_exp: i8,
    /// true if ANY module fell back to the `1ns/1ns` base (→ W-PP-TIMESCALE-DEFAULT).
    pub default_used: bool,
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
    let mut precs: Vec<i8> = Vec::new();
    let mut default_used = false;
    for &(name, lo) in modules {
        let gov = regions
            .iter()
            .rev()
            .find(|(off, _)| *off <= lo)
            .map(|(_, ts)| *ts);
        match gov {
            Some(ts) => {
                unit_exp.insert(name.to_string(), ts.unit_exp);
                precs.push(ts.prec_exp);
            }
            None => {
                default_used = true;
                unit_exp.insert(name.to_string(), TimeScale::DEFAULT.unit_exp);
                precs.push(TimeScale::DEFAULT.prec_exp);
            }
        }
    }
    let global_prec_exp = precs
        .into_iter()
        .min()
        .unwrap_or(TimeScale::DEFAULT.prec_exp);
    ResolvedTimescales {
        unit_exp,
        global_prec_exp,
        default_used,
    }
}

/// `{1|10|100}<s|ms|us|ns|ps|fs>` → base-10 exponent of seconds.
fn parse_time_literal(s: &str) -> Result<i8, String> {
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

impl PpResult {
    /// True if any diagnostic is `Error` or `Fatal`.
    pub fn has_errors(&self) -> bool {
        self.diags
            .iter()
            .any(|d| matches!(d.severity, Severity::Error | Severity::Fatal))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Source map
// ─────────────────────────────────────────────────────────────────────────────

/// One file touched during preprocessing (top-level entry or an `include`-d file).
#[derive(Clone, Debug)]
pub struct SourceFileEntry {
    /// Display name / path used in diagnostics (`SourceLoc.file`).
    pub name: String,
    /// The file's ORIGINAL text. Line/col are computed against this, never against
    /// the expanded buffer.
    pub text: String,
    /// Canonicalized absolute path, when known, used for the include cycle guard.
    /// `None` for in-memory entry files (the top-level source in `preprocess_str`).
    pub canon: Option<PathBuf>,
    /// This file's OWN directory — the first search root for a `` `include "..." ``
    /// appearing INSIDE this file (IEEE 1364 §19.3.2: a quoted include is searched
    /// relative to the directory of the currently-processed file first, then
    /// `incdirs`). Derived at register time as the parent of `canon` (or `base_dir`
    /// for the entry file, which has no resolved path). Never the global entry dir
    /// for an included file.
    pub dir: PathBuf,
}

/// One contiguous run of the expanded buffer with a single origin.
///
/// `exp_start..exp_end` is the half-open byte range in `expanded`.
/// For a VERBATIM run (`collapsed == false`), output byte `b` came from original
/// byte `orig_start + (b - exp_start)` in `file` (1:1, lengths equal).
/// For a COLLAPSED run (`collapsed == true`: macro-expanded text, substituted body,
/// included-but-mapped boundary), every output byte in the range maps to the single
/// origin byte `orig_start` in `file` (the directive / macro-use site).
#[derive(Clone, Debug)]
pub struct Segment {
    pub exp_start: u32,
    pub exp_end: u32,
    pub file: FileId,
    pub orig_start: u32,
    pub collapsed: bool,
}

/// Provenance map from expanded-byte offsets to original positions.
///
/// `segments` is kept sorted and non-overlapping by `exp_start`, covering
/// `0..expanded.len()` with no gaps. `resolve` binary-searches it.
#[derive(Debug, Default)]
pub struct SourceMap {
    pub files: Vec<SourceFileEntry>,
    pub segments: Vec<Segment>,
}

/// What `resolve` returns: enough to build a `diag::SourceLoc`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLoc {
    pub file_name: String,
    pub line: u32,
    pub col: u32,
    pub orig_byte: u32,
}

impl SourceMap {
    /// Translate an expanded-text byte offset to an original position.
    ///
    /// Binary-searches `segments` for the segment containing `exp_byte`, computes
    /// the origin byte (1:1 for verbatim, pinned to the site for collapsed runs),
    /// then runs `byte_to_line_col` against THAT file's original text.
    ///
    /// Robust to out-of-range input: clamps to the last segment / file end so a
    /// diagnostic always resolves to *some* real position. The empty-check runs
    /// FIRST (before any indexing) and `delta` is clamped to the segment's own
    /// original width with `checked_add`, so an EOF-clamped offset resolves to the
    /// last real byte of the segment — never one past it, never an overflow.
    pub fn resolve(&self, exp_byte: usize) -> ResolvedLoc {
        // Defensive: handle the empty map before any binary_search / indexing.
        if self.segments.is_empty() {
            return ResolvedLoc {
                file_name: String::new(),
                line: 1,
                col: 1,
                orig_byte: 0,
            };
        }
        let exp = exp_byte as u32;
        // Binary search: largest segment with exp_start <= exp.
        let idx = match self.segments.binary_search_by(|s| s.exp_start.cmp(&exp)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        let seg = &self.segments[idx];
        let orig_byte = if seg.collapsed {
            seg.orig_start
        } else {
            // Verbatim: clamp delta to the segment's mapped original width so an
            // out-of-range (EOF-clamped) offset never resolves past this segment's
            // origin range. `seg_width == exp_end - exp_start` equals the mapped
            // original length by the verbatim 1:1 invariant. checked_add guards the
            // u32 sum for pathological large files (falls back to the seg start).
            let seg_width = seg.exp_end - seg.exp_start;
            let delta = exp.saturating_sub(seg.exp_start).min(seg_width);
            seg.orig_start.checked_add(delta).unwrap_or(seg.orig_start)
        };
        let file = &self.files[seg.file.0 as usize];
        let (line, col) = byte_to_line_col(&file.text, orig_byte as usize);
        ResolvedLoc {
            file_name: file.name.clone(),
            line,
            col,
            orig_byte,
        }
    }

    /// Convenience: build a `diag::SourceLoc` for an expanded span `[lo, hi)`.
    /// `line`/`col`/`file` come from `lo`; `byte_start`/`byte_end` are the resolved
    /// original bytes (clamped so `byte_end >= byte_start`).
    pub fn resolve_span(&self, lo: usize, hi: usize) -> SourceLoc {
        let a = self.resolve(lo);
        let b = self.resolve(hi.max(lo));
        let byte_end = if b.file_name == a.file_name {
            b.orig_byte.max(a.orig_byte)
        } else {
            a.orig_byte
        };
        SourceLoc {
            file: a.file_name,
            line: a.line,
            col: a.col,
            byte_start: a.orig_byte,
            byte_end,
        }
    }
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

/// Production reader backed by `std::fs`.
pub struct FsIncludeReader;

impl IncludeReader for FsIncludeReader {
    fn resolve(
        &self,
        request: &str,
        current_dir: &Path,
        incdirs: &[PathBuf],
    ) -> Result<(String, PathBuf, String), ()> {
        let try_one = |base: &Path| -> Option<(String, PathBuf, String)> {
            let cand = base.join(request);
            let text = std::fs::read_to_string(&cand).ok()?;
            let canon = std::fs::canonicalize(&cand).unwrap_or_else(|_| cand.clone());
            Some((cand.display().to_string(), canon, text))
        };
        if let Some(hit) = try_one(current_dir) {
            return Ok(hit);
        }
        for dir in incdirs {
            if let Some(hit) = try_one(dir) {
                return Ok(hit);
            }
        }
        Err(())
    }
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

// ─────────────────────────────────────────────────────────────────────────────
// Internal state
// ─────────────────────────────────────────────────────────────────────────────

/// A stored macro definition.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Macro {
    /// `None` => object-like. `Some(params)` => function-like with these params
    /// (empty vec is a zero-arg function-like macro, callable only as `NAME()`).
    params: Option<Vec<String>>,
    /// Replacement text, continuation-joined, body-trimmed (leading ws after NAME
    /// removed; trailing ws kept; final newline excluded).
    body: String,
    /// (file_id, byte) of the body's start, for definition-site provenance.
    def_file: FileId,
    def_byte: u32,
}

/// One frame of the ifdef/ifndef/.../endif stack.
#[derive(Clone, Copy, Debug)]
struct CondFrame {
    /// This frame's arm is currently emitting.
    active: bool,
    /// Some arm in this group has already been taken (true since first true arm).
    taken: bool,
    /// `else` has been seen (a second `else`/`elsif` after it is an error).
    seen_else: bool,
    /// Whether the ENCLOSING context was emitting when this group opened. A group
    /// nested inside a dead arm never emits regardless of its own condition.
    parent_emitting: bool,
    /// Expanded-buffer byte offset where this group's `` `ifdef ``/`` `ifndef ``
    /// opened, captured AFTER emitting the directive's verbatim newline trace so it
    /// resolves through the SourceMap to the ACTUAL opening directive's line —
    /// instead of all unclosed frames collapsing to EOF/`out.len()`.
    open_at: u32,
}

struct Preprocessor<'a> {
    opts: &'a PreOpts,
    reader: &'a dyn IncludeReader,

    files: Vec<SourceFileEntry>,
    segments: Vec<Segment>,
    out: String,
    diags: Vec<PpDiag>,

    macros: BTreeMap<String, Macro>,
    active: BTreeSet<String>, // recursion guard (names currently expanding)
    cond: Vec<CondFrame>,     // conditional stack
    inc_stack: Vec<PathBuf>,  // canonical paths currently open (cycle guard)
    inc_depth: u32,
    macro_depth: u32,

    /// Lazy directive-line newline. A stripped directive replaces its line with a
    /// single newline (to preserve line numbering), but a maximal run of consecutive
    /// directives collapses to ONE newline, and a continuation-joined directive line
    /// contributes none. We accumulate the pending newline's origin `(file, byte)`
    /// and the run's continuation count, then flush `max(0, 1 - cont)` newlines just
    /// before the next non-directive output (or at the next conditional / EOF).
    pending_nl: Option<(FileId, u32)>,
    pending_cont: u32,

    /// Whether any directive or macro was seen at all. If false at finish, the
    /// identity fast path is taken (single 1:1 segment).
    saw_directive: bool,

    /// PP-FANOUT-CAP: set once `out` would exceed `opts.max_output_bytes`. After
    /// it trips, every emit is a no-op and `scan_text` returns immediately, so a
    /// fan-out recursion (2^N nodes) unwinds in O(depth) — bounding CPU, not just
    /// memory — instead of grinding through the whole expansion as no-ops.
    budget_blown: bool,

    /// `` `timescale `` regions captured in EXPANDED-text order (offset, scale).
    timescales: Vec<(usize, TimeScale)>,
}

/// A captured logical directive line (continuation-joined).
struct CapturedLine {
    /// The joined line text (continuation `\`+NL removed; terminating NL excluded).
    text: String,
    /// Cursor just past the terminating newline (or EOF).
    cursor: usize,
    /// Byte index of the terminating newline in the source (or EOF index).
    nl_byte: u32,
    /// Number of continuation joins absorbed into this logical line.
    conts: u32,
}

/// Outcome of argument splitting.
struct SplitArgs {
    /// Trimmed actuals (interior ws/newlines preserved).
    actuals: Vec<String>,
    /// Byte index of the matching ')' on success, or `src.len()` on EOF-before-close.
    close: usize,
    /// `true` iff a matching top-level ')' was found before EOF.
    closed: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Lexical helpers (ASCII-delimiter invariant, see §2.3)
// ─────────────────────────────────────────────────────────────────────────────

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_ident_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// Match a simple identifier `[A-Za-z_][A-Za-z0-9_$]*` starting at byte `i`.
/// Returns `(name, end)` or `None` if no identifier-start is present at `i`.
fn parse_ident(src: &str, i: usize) -> Option<(&str, usize)> {
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

fn is_directive_kw(name: &str) -> bool {
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
fn scan_string(src: &str, i: usize) -> (usize, bool) {
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
fn scan_line_comment(src: &str, i: usize) -> usize {
    let bytes = src.as_bytes();
    let mut j = i + 2;
    while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
    }
    j
}

/// Index just past a block comment `/* ... */` (including delimiters), or EOF.
fn scan_block_comment(src: &str, i: usize) -> usize {
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
fn split_args(src: &str, open: usize) -> SplitArgs {
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
fn utf8_len(lead: u8) -> usize {
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
fn substitute(body: &str, params: &[String], actuals: &[String]) -> String {
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
fn join_continuations(s: &str) -> String {
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
fn strip_trailing_line_comment(s: &str) -> &str {
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

// ─────────────────────────────────────────────────────────────────────────────
// Emission helpers (segment append, §2.2)
// ─────────────────────────────────────────────────────────────────────────────

impl Preprocessor<'_> {
    /// PP-FANOUT-CAP cumulative-output budget. Returns false (caller emits
    /// nothing) once `out` would exceed `opts.max_output_bytes`, emitting one
    /// loud `PpRecursiveMacro` diagnostic on the trip. The cap is well under
    /// `u32::MAX`, so the `as u32` segment offsets below never wrap.
    fn output_budget_ok(&mut self, incoming: usize) -> bool {
        if self.budget_blown {
            return false;
        }
        if self.out.len().saturating_add(incoming) > self.opts.max_output_bytes {
            self.budget_blown = true;
            self.err(
                MsgCode::PpRecursiveMacro,
                format!(
                    "macro expansion exceeded the {}-byte output budget \
                     (likely exponential macro fan-out)",
                    self.opts.max_output_bytes
                ),
                self.out.len(),
            );
            return false;
        }
        true
    }

    fn emit_verbatim(&mut self, s: &str, file: FileId, orig_start: u32) {
        if s.is_empty() {
            return;
        }
        if !self.output_budget_ok(s.len()) {
            return;
        }
        let exp_start = self.out.len() as u32;
        if let Some(last) = self.segments.last_mut() {
            let prev_orig_end = last.orig_start.checked_add(last.exp_end - last.exp_start);
            if !last.collapsed
                && last.file == file
                && last.exp_end == exp_start
                && prev_orig_end == Some(orig_start)
            {
                self.out.push_str(s);
                last.exp_end = self.out.len() as u32;
                return;
            }
        }
        self.out.push_str(s);
        self.segments.push(Segment {
            exp_start,
            exp_end: self.out.len() as u32,
            file,
            orig_start,
            collapsed: false,
        });
    }

    fn emit_collapsed(&mut self, s: &str, file: FileId, site_byte: u32) {
        if s.is_empty() {
            return;
        }
        if !self.output_budget_ok(s.len()) {
            return;
        }
        let exp_start = self.out.len() as u32;
        self.out.push_str(s);
        self.segments.push(Segment {
            exp_start,
            exp_end: self.out.len() as u32,
            file,
            orig_start: site_byte,
            collapsed: true,
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostic helpers
// ─────────────────────────────────────────────────────────────────────────────

impl Preprocessor<'_> {
    fn err(&mut self, code: MsgCode, msg: impl Into<String>, at: usize) {
        let at = at.min(self.out.len());
        self.diags.push(PpDiag {
            code,
            severity: Severity::Error,
            message: msg.into(),
            at,
        });
    }

    fn warn(&mut self, code: MsgCode, msg: impl Into<String>, at: usize) {
        let at = at.min(self.out.len());
        self.diags.push(PpDiag {
            code,
            severity: Severity::Warning,
            message: msg.into(),
            at,
        });
    }

    fn emitting(&self) -> bool {
        self.cond
            .last()
            .map_or(true, |f| f.active && f.parent_emitting)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Construction / run / finish (§2.6)
// ─────────────────────────────────────────────────────────────────────────────

impl<'a> Preprocessor<'a> {
    fn new(
        base_dir: &Path,
        name: &str,
        src: &str,
        opts: &'a PreOpts,
        reader: &'a dyn IncludeReader,
    ) -> Self {
        let entry = SourceFileEntry {
            name: name.to_string(),
            text: src.to_string(),
            canon: None,
            dir: base_dir.to_path_buf(),
        };
        let mut macros: BTreeMap<String, Macro> = BTreeMap::new();
        for (nm, body) in &opts.cli_defines {
            macros.insert(
                nm.clone(),
                Macro {
                    params: None,
                    body: body.clone(),
                    def_file: FileId(0),
                    def_byte: 0,
                },
            );
        }
        Preprocessor {
            opts,
            reader,
            files: vec![entry],
            segments: Vec::new(),
            out: String::new(),
            diags: Vec::new(),
            macros,
            active: BTreeSet::new(),
            cond: Vec::new(),
            inc_stack: Vec::new(),
            inc_depth: 0,
            macro_depth: 0,
            pending_nl: None,
            pending_cont: 0,
            saw_directive: false,
            budget_blown: false,
            timescales: Vec::new(),
        }
    }

    fn run(&mut self) {
        self.scan_file(FileId(0));
        // Flush a trailing directive-line newline so the output preserves it.
        self.flush_pending_nl();
        // At EOF, every still-open CondFrame is an unterminated conditional. Point
        // each at its own recorded opening offset (open_at), not at out.len().
        let unclosed: Vec<u32> = self.cond.iter().map(|f| f.open_at).collect();
        for open_at in unclosed {
            self.err(
                MsgCode::PpBadDirective,
                "unterminated `ifdef/`ifndef (no matching `endif)",
                open_at as usize,
            );
        }
        self.cond.clear();
    }

    fn finish(self) -> PpResult {
        let map = SourceMap {
            files: self.files,
            segments: self.segments,
        };
        PpResult {
            text: self.out,
            map,
            diags: self.diags,
            timescales: self.timescales,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The scanner (§2.3)
// ─────────────────────────────────────────────────────────────────────────────

impl Preprocessor<'_> {
    /// Walk one file's original text, mapping output verbatim 1:1.
    fn scan_file(&mut self, file: FileId) {
        let src = self.files[file.0 as usize].text.clone();
        self.scan_impl(&src, file, None, 0);
    }

    /// Walk synthetic macro-expansion text; every emit collapses to `site`.
    fn scan_text(&mut self, text: &str, site: (FileId, u32), depth: u32) {
        // PP-FANOUT-CAP: once the output budget is blown, abandon the expansion
        // immediately so a 2^N fan-out recursion unwinds in O(depth), not O(2^N).
        if self.budget_blown {
            return;
        }
        if depth > self.opts.max_macro_depth {
            self.err(
                MsgCode::PpRecursiveMacro,
                "macro expansion exceeded maximum depth",
                self.out.len(),
            );
            return;
        }
        // The `file` argument identifies the include-search context. Macro bodies
        // resolve includes relative to the use-site file (site.0).
        self.scan_impl(text, site.0, Some(site), depth);
    }

    /// Shared scanner core. `site_for_collapse = None` => verbatim emits mapped
    /// 1:1 against `file`. `Some((f, b))` => every emit collapses to `(f, b)`.
    fn scan_impl(
        &mut self,
        src: &str,
        file: FileId,
        site_for_collapse: Option<(FileId, u32)>,
        depth: u32,
    ) {
        let bytes = src.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            match c {
                b'"' => {
                    let (end, ok) = scan_string(src, i);
                    if !ok && end < bytes.len() {
                        // Reached a newline (unterminated). Emit up to the newline,
                        // report, and continue scanning AT the newline.
                        self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                        self.err(
                            MsgCode::PpBadDirective,
                            "unterminated string literal",
                            self.out.len(),
                        );
                        i = end;
                        continue;
                    }
                    self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                    if !ok {
                        // Unterminated at EOF.
                        self.err(
                            MsgCode::PpBadDirective,
                            "unterminated string literal",
                            self.out.len(),
                        );
                    }
                    i = end;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    let end = scan_line_comment(src, i);
                    self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                    i = end;
                }
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                    let end = scan_block_comment(src, i);
                    self.emit_run(&src[i..end], file, i as u32, site_for_collapse);
                    i = end;
                }
                b'`' => {
                    i = self.handle_backtick(src, file, site_for_collapse, depth, i);
                }
                _ => {
                    // Ordinary run: copy up to the next interesting byte.
                    let start = i;
                    let mut j = i + 1;
                    while j < bytes.len()
                        && bytes[j] != b'"'
                        && bytes[j] != b'`'
                        && bytes[j] != b'/'
                    {
                        j += 1;
                    }
                    self.emit_run(&src[start..j], file, start as u32, site_for_collapse);
                    i = j;
                }
            }
        }
    }

    /// Emit `s` verbatim (mapped to `file`@`orig`) or collapsed to the site, and
    /// only when emitting.
    fn emit_run(
        &mut self,
        s: &str,
        file: FileId,
        orig: u32,
        site_for_collapse: Option<(FileId, u32)>,
    ) {
        if !self.emitting() {
            return;
        }
        // Before any real (non-directive) output, flush the pending directive-line
        // newline so a maximal run of directives collapses to one newline.
        self.flush_pending_nl();
        match site_for_collapse {
            None => self.emit_verbatim(s, file, orig),
            Some((sf, sb)) => self.emit_collapsed(s, sf, sb),
        }
    }

    /// Handle a backtick at `i`. Returns the new cursor.
    fn handle_backtick(
        &mut self,
        src: &str,
        file: FileId,
        site_for_collapse: Option<(FileId, u32)>,
        depth: u32,
        i: usize,
    ) -> usize {
        let backtick = i;
        let Some((name, name_end)) = parse_ident(src, i + 1) else {
            // Stray backtick: no identifier follows.
            self.saw_directive = true;
            if self.emitting() {
                self.err(MsgCode::PpBadDirective, "stray backtick", self.out.len());
            }
            return i + 1;
        };
        self.saw_directive = true;
        let name = name.to_string();

        if is_directive_kw(&name) {
            return self.handle_directive(src, file, &name, backtick, name_end);
        }

        // Macro use.
        if !self.emitting() {
            // Dead region: skip the macro use, do not parse arguments.
            return name_end;
        }
        self.handle_macro_use(
            src,
            file,
            site_for_collapse,
            depth,
            &name,
            backtick,
            name_end,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_macro_use(
        &mut self,
        src: &str,
        file: FileId,
        site_for_collapse: Option<(FileId, u32)>,
        depth: u32,
        name: &str,
        backtick: usize,
        name_end: usize,
    ) -> usize {
        // The site to which expanded text collapses. For a top-level (verbatim)
        // file the site is (file, backtick). For a re-scan the site is inherited.
        let site = site_for_collapse.unwrap_or((file, backtick as u32));
        let literal = format!("`{name}");

        if self.active.contains(name) {
            self.err(
                MsgCode::PpRecursiveMacro,
                format!("recursive expansion of macro `{name}"),
                self.out.len(),
            );
            self.emit_run(&literal, file, backtick as u32, site_for_collapse);
            return name_end;
        }

        let Some(mac) = self.macros.get(name).cloned() else {
            self.err(
                MsgCode::PpBadDirective,
                format!("undefined macro use `{name}"),
                self.out.len(),
            );
            self.emit_run(&literal, file, backtick as u32, site_for_collapse);
            return name_end;
        };

        match &mac.params {
            None => {
                // Object-like: NEVER consumes a following `(`. Route through
                // `substitute` (empty params = identity except token-paste ` `` ` and
                // stringify ` `" `, which an object-like body may also use).
                let body = substitute(&mac.body, &[], &[]);
                self.active.insert(name.to_string());
                self.macro_depth += 1;
                self.scan_text(&body, site, depth + 1);
                self.macro_depth = self.macro_depth.saturating_sub(1);
                self.active.remove(name);
                name_end
            }
            Some(params) => {
                let params = params.clone();
                // Skip whitespace/newlines after NAME looking for `(`.
                let bytes = src.as_bytes();
                let mut k = name_end;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k >= bytes.len() || bytes[k] != b'(' {
                    self.err(
                        MsgCode::PpMacroArity,
                        format!("function-like macro `{name} used without argument list"),
                        self.out.len(),
                    );
                    self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                    return name_end;
                }
                let split = split_args(src, k);
                if !split.closed {
                    self.err(
                        MsgCode::PpMacroArity,
                        format!("unterminated macro argument list for `{name}"),
                        self.out.len(),
                    );
                    self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                    return split.close;
                }
                // Empty-actual rule: a single whitespace-only actual maps to []
                // iff the macro declares zero params.
                let mut actuals = split.actuals;
                if params.is_empty() && actuals.len() == 1 && actuals[0].is_empty() {
                    actuals.clear();
                }
                if actuals.len() != params.len() {
                    self.err(
                        MsgCode::PpMacroArity,
                        format!(
                            "macro `{name} expects {} argument(s), got {}",
                            params.len(),
                            actuals.len()
                        ),
                        self.out.len(),
                    );
                    self.emit_run(&literal, file, backtick as u32, site_for_collapse);
                    return split.close + 1;
                }
                // Pre-expand each actual to completion WITHOUT `name` in active.
                let expanded_actuals: Vec<String> = actuals
                    .iter()
                    .map(|a| self.expand_text_to_string(a, site, depth + 1))
                    .collect();
                // Substitute into the body, then re-scan ONLY body-derived text with
                // `name` held active.
                let substituted = substitute(&mac.body, &params, &expanded_actuals);
                self.active.insert(name.to_string());
                self.macro_depth += 1;
                self.scan_text(&substituted, site, depth + 1);
                self.macro_depth = self.macro_depth.saturating_sub(1);
                self.active.remove(name);
                split.close + 1
            }
        }
    }

    /// Expand `text` (an argument actual or an include line) to a finished string,
    /// re-scanning macro uses to a stable result, WITHOUT polluting `self.out`.
    /// Used for pre-expanded actuals (recursion-guard scoping) and include paths.
    fn expand_text_to_string(&mut self, text: &str, site: (FileId, u32), depth: u32) -> String {
        // Swap out the live output buffers AND the pending-newline state, scan into
        // fresh ones, restore. The temp expansion must not flush the parent's pending
        // directive newline into the throwaway buffer, nor leak its own.
        let saved_out = std::mem::take(&mut self.out);
        let saved_segments = std::mem::take(&mut self.segments);
        let saved_pending = self.pending_nl.take();
        let saved_cont = std::mem::take(&mut self.pending_cont);
        // The argument is expanded in the CURRENT emitting context (we only reach
        // here when emitting), and collapses provenance to the use site like any
        // expansion text.
        self.scan_text(text, site, depth);
        let result = std::mem::take(&mut self.out);
        self.out = saved_out;
        self.segments = saved_segments;
        self.pending_nl = saved_pending;
        self.pending_cont = saved_cont;
        result
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Directive handlers (§2.5)
// ─────────────────────────────────────────────────────────────────────────────

impl Preprocessor<'_> {
    /// Dispatch a directive. Returns the new cursor (always past the directive line
    /// as consumed).
    fn handle_directive(
        &mut self,
        src: &str,
        file: FileId,
        name: &str,
        backtick: usize,
        name_end: usize,
    ) -> usize {
        match name {
            "define" => self.dir_define(src, file, backtick, name_end),
            "undef" => self.dir_undef(src, file, name_end),
            "include" => self.dir_include(src, file, backtick, name_end),
            "ifdef" | "ifndef" => self.dir_ifdef(src, file, name == "ifdef", name_end),
            "elsif" => self.dir_elsif(src, name_end),
            "else" => self.dir_else(src, name_end),
            "endif" => self.dir_endif(name_end),
            "timescale" => {
                let line = self.consume_logical_line(src, name_end);
                // The directive line is stripped from the output, so `self.out.len()`
                // is the expanded offset where this timescale takes effect for all
                // following modules (file-order inheritance).
                match parse_timescale(&line.text) {
                    Ok(ts) => self.timescales.push((self.out.len(), ts)),
                    Err(msg) => self.err(
                        MsgCode::PpBadDirective,
                        format!("malformed `timescale: {msg}"),
                        self.out.len(),
                    ),
                }
                self.note_dir_newline(file, &line);
                line.cursor
            }
            "line" => {
                let line = self.consume_logical_line(src, name_end);
                self.note_dir_newline(file, &line);
                line.cursor
            }
            // `pragma <expression…>` (IEEE 1800 §22.11): accept-ignore policy —
            // the whole logical line is consumed, nothing is emitted, no diag.
            "pragma" => {
                let line = self.consume_logical_line(src, name_end);
                self.note_dir_newline(file, &line);
                line.cursor
            }
            "default_nettype" => self.consume_one_token(src, file, name_end),
            // `begin_keywords "spec"` and `unconnected_drive pull1|pull0` carry an
            // argument on their line; consume the whole logical line (accept-ignore —
            // we keep the full keyword set / drive state). IEEE 1800 §22.14, §22.10.
            "begin_keywords" | "unconnected_drive" => self.consume_one_token(src, file, name_end),
            // `end_keywords` / `nounconnected_drive` take no argument — strip the
            // directive token only, like `celldefine`.
            "celldefine"
            | "endcelldefine"
            | "resetall"
            | "end_keywords"
            | "nounconnected_drive" => name_end,
            _ => name_end,
        }
    }

    /// Capture the rest of the logical line from `from` (continuation-joined). The
    /// terminating NEWLINE is consumed (the directive line is stripped). Returns the
    /// joined text, the cursor past the terminating newline, the byte index of that
    /// terminating newline (or EOF), and how many continuation joins were absorbed.
    fn consume_logical_line(&self, src: &str, from: usize) -> CapturedLine {
        let bytes = src.as_bytes();
        let mut i = from;
        let mut raw = String::new();
        let mut conts: u32 = 0;
        // Capture physical lines, honoring `\`+NL continuation and verbatim contexts.
        loop {
            // Find end of this physical line, respecting strings/comments. We only
            // break on a continuation `\`+NL or a bare top-level newline.
            let mut j = i;
            let mut continued = false;
            while j < bytes.len() {
                match bytes[j] {
                    b'"' => {
                        let (end, _ok) = scan_string(src, j);
                        j = end;
                    }
                    b'/' if j + 1 < bytes.len() && bytes[j + 1] == b'/' => {
                        j = scan_line_comment(src, j);
                    }
                    b'/' if j + 1 < bytes.len() && bytes[j + 1] == b'*' => {
                        j = scan_block_comment(src, j);
                    }
                    b'\\' => {
                        let nl = j + 1 < bytes.len() && bytes[j + 1] == b'\n';
                        let crlf =
                            j + 2 < bytes.len() && bytes[j + 1] == b'\r' && bytes[j + 2] == b'\n';
                        if nl || crlf {
                            continued = true;
                            break;
                        }
                        j += 1;
                    }
                    b'\n' => break,
                    _ => j += utf8_len(bytes[j]),
                }
            }
            raw.push_str(&src[i..j]);
            if continued {
                conts += 1;
                // Skip `\` + (CR)LF, continue capturing the next physical line.
                if j + 1 < bytes.len() && bytes[j + 1] == b'\n' {
                    i = j + 2;
                } else {
                    i = j + 3; // `\` CR LF
                }
                continue;
            }
            // Reached a bare newline or EOF. `j` is the terminating newline byte.
            let cursor = if j < bytes.len() { j + 1 } else { j };
            return CapturedLine {
                text: raw,
                cursor,
                nl_byte: j as u32,
                conts,
            };
        }
    }

    /// Record this directive's stripped line as a pending newline. A maximal run of
    /// consecutive directives collapses to ONE newline; each continuation join in the
    /// run removes one from the pending count (so a fully-continued line yields none).
    /// `flush_pending_nl` emits `max(0, 1 - cont)` newlines just before the next
    /// non-directive output. Only meaningful when `emitting()`.
    fn note_dir_newline(&mut self, file: FileId, line: &CapturedLine) {
        if !self.emitting() {
            return;
        }
        if self.pending_nl.is_none() {
            self.pending_nl = Some((file, line.nl_byte));
        }
        self.pending_cont += line.conts;
    }

    /// Emit the pending directive-line newline(s) verbatim (mapped to the run's first
    /// terminating newline), collapsing a consecutive directive run to one newline.
    fn flush_pending_nl(&mut self) {
        if let Some((file, nl_byte)) = self.pending_nl.take() {
            let cont = std::mem::take(&mut self.pending_cont);
            let n = 1u32.saturating_sub(cont);
            for _ in 0..n {
                self.emit_verbatim("\n", file, nl_byte);
            }
        }
    }

    fn consume_one_token(&mut self, src: &str, file: FileId, from: usize) -> usize {
        let line = self.consume_logical_line(src, from);
        self.note_dir_newline(file, &line);
        line.cursor
    }

    fn dir_define(&mut self, src: &str, file: FileId, backtick: usize, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        self.note_dir_newline(file, &captured);
        if !self.emitting() {
            return cursor;
        }
        let joined = join_continuations(&captured.text);
        // Parse NAME at the start of the joined line (after the ws that followed
        // `define`).
        let trimmed = joined.trim_start();
        let lead_ws = joined.len() - trimmed.len();
        let Some((nm, after_name)) = parse_ident(trimmed, 0) else {
            self.err(
                MsgCode::PpBadDirective,
                "`define requires a macro name",
                self.out.len(),
            );
            return cursor;
        };
        let nm = nm.to_string();
        if is_directive_kw(&nm) {
            self.err(
                MsgCode::PpBadDirective,
                "cannot define a directive keyword as a macro",
                self.out.len(),
            );
            return cursor;
        }
        let tail = &trimmed[after_name..];
        // Significant-space: function-like iff `(` IMMEDIATELY follows NAME.
        let (params, body_src): (Option<Vec<String>>, &str) = if tail.starts_with('(') {
            // Parse parameter list up to matching ')'.
            match parse_param_list(tail) {
                Ok((ps, rest)) => (Some(ps), rest),
                Err(msg) => {
                    self.err(MsgCode::PpBadDirective, msg, self.out.len());
                    return cursor;
                }
            }
        } else {
            (None, tail)
        };
        // Body: trim leading ws after NAME/param-list, drop trailing line comment.
        let body_no_comment = strip_trailing_line_comment(body_src);
        let body = body_no_comment.trim_start().to_string();
        // def_byte: start of the body in the ORIGINAL file. Approximate as the
        // backtick site for collapsed provenance; exact body offset isn't surfaced.
        let def_byte = (backtick + 1) as u32;
        let _ = lead_ws;
        let new_mac = Macro {
            params,
            body,
            def_file: file,
            def_byte,
        };
        if let Some(existing) = self.macros.get(&nm) {
            if *existing == new_mac
                || (existing.params == new_mac.params && existing.body == new_mac.body)
            {
                // Identical redefinition: silent.
            } else {
                self.warn(
                    MsgCode::PpMacroRedefined,
                    format!("macro `{nm} redefined with different text"),
                    self.out.len(),
                );
            }
        }
        self.macros.insert(nm, new_mac);
        cursor
    }

    fn dir_undef(&mut self, src: &str, file: FileId, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        self.note_dir_newline(file, &captured);
        if !self.emitting() {
            return cursor;
        }
        let trimmed = captured.text.trim();
        let Some((nm, _)) = parse_ident(trimmed, 0) else {
            self.err(
                MsgCode::PpBadDirective,
                "`undef requires a macro name",
                self.out.len(),
            );
            return cursor;
        };
        if is_directive_kw(nm) {
            self.err(
                MsgCode::PpBadDirective,
                "cannot `undef a directive keyword",
                self.out.len(),
            );
            return cursor;
        }
        if self.macros.remove(nm).is_none() {
            self.warn(
                MsgCode::PpUndefUndefined,
                format!("`undef of macro `{nm} that was never defined"),
                self.out.len(),
            );
        }
        cursor
    }

    fn dir_include(&mut self, src: &str, file: FileId, backtick: usize, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        self.note_dir_newline(file, &captured);
        if !self.emitting() {
            return cursor;
        }
        let joined = join_continuations(&captured.text);
        let no_comment = strip_trailing_line_comment(&joined);
        // Macro-expand the captured path text (fixpoint, bounded).
        let expanded = self.expand_text_to_string(no_comment, (file, backtick as u32), 1);
        // Parse: optional ws/comment, exactly one "..." token, optional ws/comment.
        let Some(request) = parse_single_quoted(&expanded) else {
            self.err(
                MsgCode::PpBadDirective,
                "`include requires a single quoted path",
                self.out.len(),
            );
            return cursor;
        };
        if self.inc_depth >= self.opts.max_include_depth {
            self.err(
                MsgCode::PpRecursiveInclude,
                "`include nesting exceeded maximum depth",
                self.out.len(),
            );
            return cursor;
        }
        let current_dir = self.files[file.0 as usize].dir.clone();
        let Ok((disp_name, canon, text)) =
            self.reader
                .resolve(&request, &current_dir, &self.opts.incdirs)
        else {
            self.err(
                MsgCode::PpIncludeNotFound,
                format!("`include \"{request}\" not found on search path"),
                self.out.len(),
            );
            return cursor;
        };
        if self.inc_stack.iter().any(|p| p == &canon) {
            self.err(
                MsgCode::PpRecursiveInclude,
                format!("cyclic `include of \"{request}\""),
                self.out.len(),
            );
            return cursor;
        }
        let dir = canon
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(current_dir);
        let new_id = FileId(self.files.len() as u32);
        self.files.push(SourceFileEntry {
            name: disp_name,
            text,
            canon: Some(canon.clone()),
            dir,
        });
        self.inc_stack.push(canon);
        self.inc_depth += 1;
        self.scan_file(new_id);
        self.inc_stack.pop();
        self.inc_depth = self.inc_depth.saturating_sub(1);
        cursor
    }

    fn dir_ifdef(&mut self, src: &str, file: FileId, is_ifdef: bool, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        let trimmed = captured.text.trim();
        let name = parse_ident(trimmed, 0).map(|(n, _)| n.to_string());
        let defined = name
            .as_deref()
            .map(|n| self.macros.contains_key(n))
            .unwrap_or(false);
        let active = defined == is_ifdef;
        let parent_emitting = self.emitting();
        // Flush any pending define-run newline, then anchor this `ifdef` with a
        // verbatim newline mapped to its own terminating-newline byte (emitted even
        // in a dead region) so `open_at` resolves to THIS directive's line, giving
        // distinct unterminated-conditional diagnostics per frame.
        self.flush_pending_nl();
        self.emit_verbatim("\n", file, captured.nl_byte);
        self.cond.push(CondFrame {
            active,
            taken: active,
            seen_else: false,
            parent_emitting,
            open_at: self.out.len().saturating_sub(1) as u32,
        });
        if name.is_none() {
            self.err(
                MsgCode::PpBadDirective,
                "`ifdef/`ifndef requires a macro name",
                self.out.len(),
            );
        }
        cursor
    }

    fn dir_elsif(&mut self, src: &str, name_end: usize) -> usize {
        let captured = self.consume_logical_line(src, name_end);
        let cursor = captured.cursor;
        let trimmed = captured.text.trim();
        let name = parse_ident(trimmed, 0).map(|(n, _)| n.to_string());
        let Some(frame) = self.cond.last_mut() else {
            self.err(
                MsgCode::PpBadDirective,
                "`elsif without matching `ifdef",
                self.out.len(),
            );
            return cursor;
        };
        if frame.seen_else {
            self.err(
                MsgCode::PpBadDirective,
                "`elsif after `else",
                self.out.len(),
            );
            return cursor;
        }
        let defined = match &name {
            Some(n) => self.macros.contains_key(n),
            None => false,
        };
        let frame = self.cond.last_mut().unwrap();
        if frame.taken {
            frame.active = false;
        } else if defined {
            frame.active = true;
            frame.taken = true;
        } else {
            frame.active = false;
        }
        cursor
    }

    fn dir_else(&mut self, src: &str, name_end: usize) -> usize {
        let cursor = self.consume_logical_line(src, name_end).cursor;
        let Some(frame) = self.cond.last_mut() else {
            self.err(
                MsgCode::PpBadDirective,
                "`else without matching `ifdef",
                self.out.len(),
            );
            return cursor;
        };
        if frame.seen_else {
            self.err(MsgCode::PpBadDirective, "duplicate `else", self.out.len());
            return cursor;
        }
        frame.seen_else = true;
        frame.active = !frame.taken;
        frame.taken = true;
        cursor
    }

    fn dir_endif(&mut self, name_end: usize) -> usize {
        if self.cond.pop().is_none() {
            self.err(
                MsgCode::PpBadDirective,
                "`endif without matching `ifdef",
                self.out.len(),
            );
        }
        name_end
    }
}

/// Parse a parameter list starting at the `(` of `s`. Returns `(params, rest)`
/// where `rest` is the body text after the `)`. Rejects `=` defaults and duplicate
/// names (returns `Err(message)`).
fn parse_param_list(s: &str) -> Result<(Vec<String>, &str), &'static str> {
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
fn parse_single_quoted(s: &str) -> Option<String> {
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

fn skip_ws_comments(s: &str, mut i: usize) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// In-memory include shim. `files` is keyed by FULL virtual path (e.g.
    /// "/virtual/sub/b.svh"). `resolve` joins `request` onto `current_dir` (then each
    /// incdir), normalizes `.`/`..`, and looks the result up — so it honors the
    /// IEEE nested-include rule and lets a test prove directory-relative resolution.
    /// canon == the normalized joined path; its parent is the new file's `dir`.
    struct MemReader {
        files: std::collections::BTreeMap<String, String>,
    }
    impl MemReader {
        /// Lexically normalize a path to a "/"-joined string (no fs access): drop
        /// "." components, pop on "..". Splits on BOTH separators — on Windows
        /// `Path::join` inserts `\`, which silently missed the '/'-keyed map
        /// (caught by the first 3-OS CI run: nested include NOT-FOUND on
        /// windows-latest only).
        fn norm(p: &Path) -> String {
            let mut parts: Vec<&str> = Vec::new();
            let lossy = p.to_string_lossy();
            for comp in lossy.split(['/', '\\']) {
                match comp {
                    "" | "." => {}
                    ".." => {
                        parts.pop();
                    }
                    other => parts.push(other),
                }
            }
            format!("/{}", parts.join("/"))
        }
        fn try_key(&self, base: &Path, request: &str) -> Option<(String, PathBuf, String)> {
            let key = Self::norm(&base.join(request));
            self.files
                .get(&key)
                .map(|t| (key.clone(), PathBuf::from(&key), t.clone()))
        }
    }
    impl IncludeReader for MemReader {
        fn resolve(
            &self,
            request: &str,
            current_dir: &Path,
            incdirs: &[PathBuf],
        ) -> Result<(String, PathBuf, String), ()> {
            if let Some(hit) = self.try_key(current_dir, request) {
                return Ok(hit);
            }
            for d in incdirs {
                if let Some(hit) = self.try_key(d, request) {
                    return Ok(hit);
                }
            }
            Err(())
        }
    }

    fn pp(src: &str) -> PpResult {
        preprocess_str(Path::new("/virtual"), "top.sv", src, &PreOpts::default())
    }

    /// PP-FANOUT-CAP (2026-06-22 audit): chained doubling macros
    /// (`` `Mi = `Mi-1 `Mi-1 ``) expand to 2^N copies at depth N, so the depth
    /// guard never trips and a ~30-line file OOMs (≈8 GiB at N=24, measured
    /// 2.1 GiB at N=22). The cumulative-output budget must turn this into a loud
    /// bounded error instead of unbounded materialization.
    #[test]
    fn macro_fanout_is_bounded_not_oom() {
        let mut src = String::from("`define M0 xx\n");
        for i in 1..=20 {
            src.push_str(&format!("`define M{i} `M{} `M{}\n", i - 1, i - 1));
        }
        // expand OUTSIDE a string literal so the substitution actually fires.
        src.push_str("module t; initial $display(\"%0d\", $bits({`M20})); endmodule\n");
        let opts = PreOpts {
            max_output_bytes: 64 * 1024, // small so the test is fast
            ..PreOpts::default()
        };
        let r = preprocess_str(Path::new("/virtual"), "top.sv", &src, &opts);
        assert!(
            r.has_errors(),
            "exponential fan-out must fail loud, not silently expand"
        );
        assert!(
            r.diags
                .iter()
                .any(|d| d.code == MsgCode::PpRecursiveMacro && d.message.contains("budget")),
            "expected the output-budget diagnostic, got {:?}",
            r.diags
        );
        // the materialized text must stay near the cap, not balloon to MBs.
        assert!(
            r.text.len() <= 64 * 1024 + 4096,
            "output must stay bounded by the budget, got {} bytes",
            r.text.len()
        );
    }

    #[test]
    fn timescale_literal_parsing() {
        assert_eq!(
            parse_timescale("1ns/100ps"),
            Ok(TimeScale {
                unit_exp: -9,
                prec_exp: -10
            })
        );
        assert_eq!(
            parse_timescale("10ns/1ns"),
            Ok(TimeScale {
                unit_exp: -8,
                prec_exp: -9
            })
        );
        assert_eq!(
            parse_timescale(" 1us / 1ps "),
            Ok(TimeScale {
                unit_exp: -6,
                prec_exp: -12
            })
        );
        // precision coarser than unit → error
        assert!(parse_timescale("1ns/1us").is_err());
        // bad mantissa / unit
        assert!(parse_timescale("5ns/1ns").is_err());
        assert!(parse_timescale("1xs/1ns").is_err());
        assert!(parse_timescale("1ns").is_err());
    }

    #[test]
    fn timescale_region_table_file_order() {
        let r = pp(
            "`timescale 1ns/100ps\nmodule a; endmodule\n`timescale 10ns/1ns\nmodule b; endmodule\n",
        );
        assert!(!r.has_errors(), "diags: {:?}", r.diags);
        assert_eq!(r.timescales.len(), 2);
        assert_eq!(
            r.timescales[0].1,
            TimeScale {
                unit_exp: -9,
                prec_exp: -10
            }
        );
        assert_eq!(
            r.timescales[1].1,
            TimeScale {
                unit_exp: -8,
                prec_exp: -9
            }
        );
        // the first region begins before `module a`, the second before `module b`.
        let a = r.text.find("module a").unwrap();
        let b = r.text.find("module b").unwrap();
        assert!(r.timescales[0].0 <= a && a < r.timescales[1].0);
        assert!(r.timescales[1].0 <= b);
    }

    #[test]
    fn timescale_malformed_is_error() {
        let r = pp("`timescale 1ns/1us\nmodule m; endmodule\n");
        assert!(r.has_errors(), "coarse precision must error");
    }

    #[test]
    fn resolve_module_timescales_file_order_and_global_min() {
        let regions = [
            (
                10usize,
                TimeScale {
                    unit_exp: -9,
                    prec_exp: -10,
                },
            ), // 1ns/100ps
            (
                50usize,
                TimeScale {
                    unit_exp: -8,
                    prec_exp: -12,
                },
            ), // 10ns/1ps
        ];
        // a@20 → region@10 ; b@60 → region@50 ; c@5 (before any) → 1ns/1ns base.
        let modules = [("a", 20usize), ("b", 60usize), ("c", 5usize)];
        let r = resolve_module_timescales(&modules, &regions);
        assert_eq!(r.unit_exp["a"], -9);
        assert_eq!(r.unit_exp["b"], -8);
        assert_eq!(r.unit_exp["c"], -9); // default
        assert_eq!(r.global_prec_exp, -12); // min(-10, -12, -9)
        assert!(r.default_used); // c fell back
    }

    #[test]
    fn resolve_no_regions_is_default() {
        let r = resolve_module_timescales(&[("m", 0)], &[]);
        assert_eq!(r.unit_exp["m"], -9);
        assert_eq!(r.global_prec_exp, -9);
        assert!(r.default_used);
    }
    /// `files` keys are paths RELATIVE to "/virtual" (joined for the shim map).
    fn pp_mem(src: &str, files: &[(&str, &str)]) -> PpResult {
        let reader = MemReader {
            files: files
                .iter()
                .map(|(k, v)| {
                    (
                        MemReader::norm(&Path::new("/virtual").join(k)),
                        v.to_string(),
                    )
                })
                .collect(),
        };
        preprocess_with(
            Path::new("/virtual"),
            "top.sv",
            src,
            &PreOpts::default(),
            &reader,
        )
    }
    fn codes(r: &PpResult) -> Vec<&'static str> {
        r.diags.iter().map(|d| d.code.mnemonic()).collect()
    }

    // 1. object-like macro
    #[test]
    fn object_macro_expands() {
        let r = pp("`define W 8\nwire [`W-1:0] x;\n");
        assert!(r.diags.is_empty());
        assert_eq!(r.text, "\nwire [8-1:0] x;\n");
    }

    // 2. function-like macro
    #[test]
    fn function_macro_expands() {
        let r = pp("`define MAX(a,b) ((a)>(b)?(a):(b))\nassign y = `MAX(p, q);\n");
        assert!(r.diags.is_empty());
        assert_eq!(r.text, "\nassign y = ((p)>(q)?(p):(q));\n");
    }

    // DIR-PP: token-paste `` in a function-like macro body (IEEE 1800 §22.5.2).
    #[test]
    fn token_paste_function_macro() {
        let r = pp("`define CAT(a,b) a``b\n`CAT(foo,bar)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\nfoobar\n");
    }

    // DIR-PP: paste a prefix onto an argument to build an identifier.
    #[test]
    fn token_paste_prefix_ident() {
        let r = pp("`define REG(n) reg_``n\n`REG(count)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\nreg_count\n");
    }

    // DIR-PP: chained paste a``b``c.
    #[test]
    fn token_paste_chained() {
        let r = pp("`define D(a,b,c) a``b``c\n`D(x,y,z)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\nxyz\n");
    }

    // DIR-PP: paste deletes the two-backtick operator but PRESERVES adjacent
    // whitespace (iverilog parity: `a `` b` => `a  b`, NOT `ab`).
    #[test]
    fn token_paste_preserves_surrounding_whitespace() {
        let r = pp("`define C(a,b) a `` b\n`C(foo,bar)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\nfoo  bar\n");
    }

    // DIR-PP: token-paste operator INSIDE a stringification is also deleted
    // (whitespace preserved): `"a `` b`" => "a  b"; tight `"a``b`" => "xy".
    #[test]
    fn token_paste_inside_stringify() {
        let r = pp("`define J(a,b) `\"a `` b`\"\n`J(x,y)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\n\"x  y\"\n");
        let r2 = pp("`define J2(a,b) `\"a``b`\"\n`J2(x,y)\n");
        assert!(r2.diags.is_empty(), "{:?}", r2.diags);
        assert_eq!(r2.text, "\n\"xy\"\n");
    }

    // DIR-PP: object-like macro paste (no params) — routed through substitute too.
    #[test]
    fn token_paste_object_macro() {
        let r = pp("`define J x``y\n`J\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\nxy\n");
    }

    // DIR-PP: stringification `"x`" with argument substitution (IEEE 1800 §22.5.1).
    #[test]
    fn stringify_basic() {
        let r = pp("`define STR(x) `\"x`\"\n`STR(hi)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\n\"hi\"\n");
    }

    // DIR-PP: stringify preserves literal text + spaces around the arg.
    #[test]
    fn stringify_spaces_and_text() {
        let r = pp("`define P(a,b) `\"a and b`\"\n`P(x,y)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\n\"x and y\"\n");
    }

    // DIR-PP: `\`" inside a stringify produces an escaped quote (=> `\"`).
    #[test]
    fn stringify_embedded_escaped_quote() {
        let r = pp("`define Q(x) `\"a `\\`\"x`\\`\" b`\"\n`Q(z)\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        // expansion: "a \"z\" b"
        assert_eq!(r.text, "\n\"a \\\"z\\\" b\"\n");
    }

    // DIR-PP: object-like stringify.
    #[test]
    fn stringify_object_macro() {
        let r = pp("`define LIT `\"text`\"\n`LIT\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert_eq!(r.text, "\n\"text\"\n");
    }

    // DIR-PP: `begin_keywords/`end_keywords are accepted and stripped (no diag).
    #[test]
    fn begin_end_keywords_accepted() {
        let r = pp("`begin_keywords \"1364-2005\"\nmodule m; endmodule\n`end_keywords\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert!(r.text.contains("module m; endmodule"));
        assert!(!r.text.contains("begin_keywords"));
        assert!(!r.text.contains("end_keywords"));
    }

    // DIR-PP: `unconnected_drive/`nounconnected_drive accepted and stripped.
    #[test]
    fn unconnected_drive_accepted() {
        let r = pp("`unconnected_drive pull1\nmodule m; endmodule\n`nounconnected_drive\n");
        assert!(r.diags.is_empty(), "{:?}", r.diags);
        assert!(r.text.contains("module m; endmodule"));
        assert!(!r.text.contains("unconnected_drive"));
    }

    // 3. multi-line continuation in a macro body
    #[test]
    fn line_continuation_joins_body() {
        let r = pp("`define LONG aaa \\\nbbb\nx = `LONG;\n");
        assert!(r.diags.is_empty());
        assert_eq!(r.text, "x = aaa bbb;\n");
    }

    // 4. nested ifdef/ifndef/elsif/else/endif
    #[test]
    fn conditional_arms() {
        let src = "\
`define A
`ifdef A
keepA
`elsif B
dropB
`else
dropE
`endif
`ifndef A
dropN
`else
keepM
`endif
";
        let r = pp(src);
        assert!(r.diags.is_empty());
        assert!(r.text.contains("keepA"));
        assert!(r.text.contains("keepM"));
        assert!(!r.text.contains("dropB"));
        assert!(!r.text.contains("dropE"));
        assert!(!r.text.contains("dropN"));
    }

    // 5. undef removes a macro (later use becomes E-PP-BAD-DIRECTIVE)
    #[test]
    fn undef_removes_macro() {
        let r = pp("`define X 1\n`undef X\nv = `X;\n");
        assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
        assert!(r.text.contains("`X")); // emitted literally after undef
    }

    // 6. arity error
    #[test]
    fn arity_mismatch_errors() {
        let r = pp("`define F(a,b) (a+b)\nz = `F(1);\n");
        assert_eq!(codes(&r), vec!["E-PP-MACRO-ARITY"]);
    }

    // 7. recursive-macro guard terminates and reports
    #[test]
    fn recursive_macro_guarded() {
        let r = pp("`define R `R\nq = `R;\n");
        assert_eq!(codes(&r), vec!["E-PP-RECURSIVE-MACRO"]);
        assert!(r.text.contains("`R")); // left literal, finite output
    }

    // 8. comma inside parens does NOT split args
    #[test]
    fn comma_in_parens_not_split() {
        let r = pp("`define G(x) [x]\ny = `G(foo(a, b));\n");
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert_eq!(r.text, "\ny = [foo(a, b)];\n");
    }

    // 9. comma inside a string does NOT split args
    #[test]
    fn comma_in_string_not_split() {
        let r = pp("`define S(x) {x}\ny = `S(\"a,b\");\n");
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert_eq!(r.text, "\ny = {\"a,b\"};\n");
    }

    // 10. a macro-looking token inside a string is NOT expanded
    #[test]
    fn backtick_in_string_not_expanded() {
        let r = pp("`define M xyz\nv = \"`M\";\n");
        assert!(r.diags.is_empty());
        assert_eq!(r.text, "\nv = \"`M\";\n"); // string preserved verbatim
    }

    // 11. include happy path (in-memory shim), defines persist after include
    #[test]
    fn include_happy_path() {
        let r = pp_mem(
            "`include \"defs.svh\"\nwire [`WIDTH-1:0] bus;\n",
            &[("defs.svh", "`define WIDTH 16\n")],
        );
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert!(r.text.contains("wire [16-1:0] bus;"));
    }

    // 12. include cycle guard
    #[test]
    fn include_cycle_guarded() {
        let r = pp_mem(
            "`include \"a.svh\"\n",
            &[
                ("a.svh", "`include \"b.svh\"\n"),
                ("b.svh", "`include \"a.svh\"\n"), // would re-open a.svh
            ],
        );
        assert!(codes(&r).contains(&"E-PP-RECURSIVE-INCLUDE"));
    }

    // 13. include not found
    #[test]
    fn include_not_found() {
        let r = pp_mem("`include \"missing.svh\"\n", &[]);
        assert_eq!(codes(&r), vec!["E-PP-INCLUDE-NOT-FOUND"]);
    }

    // 14. redefine warning (different body); identical redefine is silent
    #[test]
    fn redefine_warns_only_on_difference() {
        let r = pp("`define D 1\n`define D 1\n`define D 2\n");
        assert_eq!(codes(&r), vec!["W-PP-MACRO-REDEFINED"]); // exactly one (the 1->2)
    }

    // 15. undef of an undefined name warns
    #[test]
    fn undef_undefined_warns() {
        let r = pp("`undef NOPE\n");
        assert_eq!(codes(&r), vec!["W-PP-UNDEF-UNDEFINED"]);
    }

    // 16. unknown directive errors
    #[test]
    fn unknown_directive_errors() {
        let r = pp("`frobnicate foo\n");
        assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
    }

    // 17. SOURCE-MAP round trip
    #[test]
    fn source_map_round_trip() {
        let src = "`define P qq\n\nz = `P + bad;\n";
        let r = pp(src);
        assert!(r.diags.is_empty());
        let exp_off = r.text.find("bad").unwrap();
        let loc = r.map.resolve(exp_off);
        assert_eq!(loc.file_name, "top.sv");
        assert_eq!(
            loc.line, 3,
            "expanded offset must map back to original line 3"
        );
        let exp_qq = r.text.find("qq").unwrap();
        let loc_qq = r.map.resolve(exp_qq);
        assert_eq!(
            loc_qq.line, 3,
            "expanded macro text collapses to the use site"
        );
    }

    // 18. unterminated string reported as E-PP-BAD-DIRECTIVE
    #[test]
    fn unterminated_string_reported() {
        let r = pp("v = \"abc\nnext;\n");
        assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
    }

    // 19. SOURCE-MAP verbatim fidelity
    #[test]
    fn verbatim_region_resolves_byte_for_byte() {
        let src = "module m;\n  wire w;\nendmodule\n";
        let r = pp(src);
        assert!(r.diags.is_empty());
        assert_eq!(r.text, src); // identity fast path
        for off in [0usize, 10, 20, src.len()] {
            let loc = r.map.resolve(off);
            let (line, col) = byte_to_line_col(src, off);
            assert_eq!((loc.line, loc.col), (line, col), "verbatim off={off}");
            assert_eq!(loc.orig_byte as usize, off.min(src.len()));
        }
    }

    // 20. define significant-space rule
    #[test]
    fn define_significant_space_makes_object_like() {
        let r1 = pp("`define F (x)\ny = `F;\n");
        assert!(r1.diags.is_empty(), "got {:?}", codes(&r1));
        assert_eq!(r1.text, "\ny = (x);\n");
        let r2 = pp("`define G(x) (x+1)\ny = `G(1);\n");
        assert!(r2.diags.is_empty(), "got {:?}", codes(&r2));
        assert_eq!(r2.text, "\ny = (1+1);\n");
    }

    // 21. recursion guard scoping: `A in an arg to `A is a SIBLING use
    #[test]
    fn macro_name_in_argument_is_not_recursive() {
        let r = pp("`define B z\n`define A(x) [x]\ny = `A(`B);\n");
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert_eq!(r.text, "\ny = [z];\n");
    }

    // 22. unterminated macro argument list is ALWAYS an error
    #[test]
    fn unterminated_macro_call_errors() {
        let r = pp("`define MAX(a,b) ((a)>(b)?(a):(b))\nz = `MAX(p, q\n");
        assert_eq!(codes(&r), vec!["E-PP-MACRO-ARITY"]);
    }

    // 23. two unclosed `ifdef`s report at two DISTINCT lines
    #[test]
    fn two_unclosed_ifdefs_report_distinct_lines() {
        let r = pp("`ifdef A\n`ifdef B\nx\n");
        let errs: Vec<_> = r
            .diags
            .iter()
            .filter(|d| d.code.mnemonic() == "E-PP-BAD-DIRECTIVE")
            .map(|d| r.map.resolve(d.at).line)
            .collect();
        assert_eq!(errs.len(), 2, "two unterminated frames");
        assert_ne!(errs[0], errs[1], "distinct opening lines, not both EOF");
        assert!(errs.contains(&1) && errs.contains(&2));
    }

    // 24. include path supplied via a macro
    #[test]
    fn include_path_via_macro() {
        let r = pp_mem(
            "`define INC \"f.svh\"\n`include `INC\nwire [`W-1:0] b;\n",
            &[("f.svh", "`define W 4\n")],
        );
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert!(r.text.contains("wire [4-1:0] b;"));
    }

    // 25. NESTED include resolves relative to the INCLUDING file's own directory
    #[test]
    fn nested_include_uses_including_file_dir() {
        let r = pp_mem(
            "`include \"sub/b.svh\"\nwire [`N-1:0] z;\n",
            &[
                ("sub/b.svh", "`include \"c.svh\"\n"),
                ("sub/c.svh", "`define N 3\n"), // only in sub/, not at entry dir
            ],
        );
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert!(r.text.contains("wire [3-1:0] z;"));
    }

    // 26. P2-12 policy: `` `pragma <rest-of-line> `` is accepted and ignored
    // (IEEE 1800 §22.11) — previously misparsed as an undefined macro use.
    #[test]
    fn pragma_directive_accepted_and_ignored() {
        let r = pp_mem(
            "`pragma protect begin\nmodule m; endmodule\n`pragma translate_off // tail\n",
            &[],
        );
        assert!(!r.has_errors(), "diags: {:?}", r.diags);
        assert!(r.text.contains("module m"));
        assert!(!r.text.contains("pragma"));
    }

    // 27. byte_to_line_col / SourceMap::resolve never panic on a byte that lands
    // mid-UTF-8-scalar (a resolved orig_byte can fall inside a multibyte char).
    #[test]
    fn resolve_mid_char_no_panic() {
        let src = "// 한글 주석\n`define W 8\nwire [`W-1:0] x;\n";
        // Every byte offset (incl. mid-scalar ones in the comment) must be safe.
        for b in 0..=src.len() + 4 {
            let (line, col) = byte_to_line_col(src, b);
            assert!(line >= 1 && col >= 1);
        }
        // And through the public SourceMap on the expanded text.
        let r = pp_mem(src, &[]);
        for b in 0..=r.text.len() + 4 {
            let _ = r.map.resolve(b);
        }
    }
}
