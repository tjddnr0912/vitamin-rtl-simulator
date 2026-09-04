//! hdl-preprocess — Verilog-2005 MVP preprocessor.
//!
//! Runs before the lexer: raw source -> expanded text + SourceMap -> lex -> parse.
//! Pure text-to-text transform plus a byte-offset provenance map. std-only + `diag`.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use diag::{MsgCode, Severity, SourceLoc};

// ---- split parts (mechanical refactor) ----
mod directives;
mod lexutil;
mod scan;
pub use lexutil::*;
#[cfg(test)]
mod tests;

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
    /// `` `default_nettype `` regions as (expanded offset, is_none), in source order.
    /// Same shape and same resolution rule as `timescales`: the LAST region whose
    /// offset is <= a module's start governs that module.
    pub nettype_none: Vec<(usize, bool)>,
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

/// Per-module timescale resolution result (S2). Plain types so `elaborate` need
/// not depend on this crate: the glue passes `unit_exp` + `global_prec_exp` in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedTimescales {
    /// module NAME → its delay-unit exponent (`unit_exp`). The per-module delay
    /// multiplier is `10^(unit_exp − global_prec_exp)`.
    pub unit_exp: std::collections::BTreeMap<String, i8>,
    /// module NAME → its OWN precision exponent (`prec_exp`). IEEE two-stage
    /// delay conversion: a `#delay` first rounds to the declaring module's own
    /// precision (`round(d × 10^(unit−prec))`), THEN scales by
    /// `10^(prec − global_prec_exp)` to global ticks — one global-grain rounding
    /// kept sub-precision digits the module declared away (adversarial review,
    /// doc-08 §delay 2단계).
    pub prec_exp: std::collections::BTreeMap<String, i8>,
    /// Modules with NO governing `` `timescale `` — empty when every module has one,
    /// or when none does. IEEE 1800-2017 §3.14.2.2 makes the MIXED case an error:
    /// if any module in the design has a timescale, all must. vita ran such a design
    /// at `errors=0 warnings=0`, and the reporter shipped it to sign-off where xrun
    /// refused to elaborate it (`*F,CUMSTS`); Verilator says `Error-TIMESCALEMOD`.
    pub ungoverned: Vec<String>,
    /// design-wide FINEST precision exponent = the global tick base.
    pub global_prec_exp: i8,
    /// true if ANY module fell back to the `1ns/1ns` base (→ W-PP-TIMESCALE-DEFAULT).
    pub default_used: bool,
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
    /// Per-file [`line_starts_of`] index, built on first `resolve` and reused.
    ///
    /// A CACHE, not data: it is derived from `files[i].text` and is deliberately
    /// not part of the `.vu` wire form. It exists because V33-8 made `resolve`
    /// a per-statement cost at elaborate time — see `byte_to_line_col_indexed`
    /// for the measurement. Private so nothing can construct a map whose index
    /// disagrees with its text; `from_parts` is the outside-the-crate builder.
    line_starts: std::cell::OnceCell<Vec<Vec<u32>>>,
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
    /// Build a map from its wire parts (the `.vu` v28 tail). The line index is
    /// derived, so it is never carried and never passed in.
    pub fn from_parts(files: Vec<SourceFileEntry>, segments: Vec<Segment>) -> Self {
        SourceMap {
            files,
            segments,
            line_starts: std::cell::OnceCell::new(),
        }
    }

    /// `(line, col)` of `orig_byte` in file `fi`, through the memoized index.
    fn line_col_in(&self, fi: usize, orig_byte: u32) -> (u32, u32) {
        let idx = self
            .line_starts
            .get_or_init(|| self.files.iter().map(|f| line_starts_of(&f.text)).collect());
        match (self.files.get(fi), idx.get(fi)) {
            (Some(f), Some(starts)) => {
                byte_to_line_col_indexed(&f.text, starts, orig_byte as usize)
            }
            // Unreachable given `idx` is built from `files`, but a resolver that
            // panics turns a diagnostic into a crash; degrade to the walk.
            (Some(f), None) => byte_to_line_col(&f.text, orig_byte as usize),
            _ => (1, 1),
        }
    }

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
        let (line, col) = self.line_col_in(seg.file.0 as usize, orig_byte);
        let file = &self.files[seg.file.0 as usize];
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
// Internal state
// ─────────────────────────────────────────────────────────────────────────────

/// A stored macro definition.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Macro {
    /// `None` => object-like. `Some(params)` => function-like with these params
    /// (empty vec is a zero-arg function-like macro, callable only as `NAME()`).
    params: Option<Vec<String>>,
    /// §22.5.1 default text per formal (`None` = no default), same length as
    /// `params`; empty for an object-like macro. Bound at USE time exactly like an
    /// actual — pre-expanded without the macro active — so a default naming a macro
    /// defined after this one still resolves (both oracles).
    defaults: Vec<Option<String>>,
    /// Replacement text, continuation-joined (the newline of every `\`+NL is kept
    /// so a body's directives stay one per line), body-trimmed (leading ws after NAME
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
    /// While scanning EXPANSION text (`scan_text`): the use site every emit collapses
    /// to. A directive met inside an expansion (`\`ifdef` in a macro body) must map
    /// its newline to this site too — `consume_logical_line`'s byte offsets index the
    /// expansion STRING, not the site file, and a verbatim emit with one of them would
    /// point provenance at an arbitrary byte of that file.
    cur_site: Option<(FileId, u32)>,
    /// The byte just past the outermost macro USE being expanded — `\`__LINE__` inside
    /// a body is the line where the use's argument list CLOSES (both oracles: a use
    /// spanning lines 4–5 reports 5), not where its backtick sits.
    line_anchor: Option<(FileId, u32)>,

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
    nettype_none: Vec<(usize, bool)>,
}

/// A captured logical directive line (continuation-joined).
struct CapturedLine {
    /// The joined line text (each continuation's `\` removed, its NL kept; terminating NL excluded).
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
            .is_none_or(|f| f.active && f.parent_emitting)
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
                    defaults: Vec::new(),
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
            cur_site: None,
            line_anchor: None,
            saw_directive: false,
            budget_blown: false,
            timescales: Vec::new(),
            nettype_none: Vec::new(),
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

    /// Multi-source variant of [`run`](Self::run): scan files `0..n` in order into the
    /// shared output/segments/macros, flushing each file's trailing directive newline
    /// before the next begins (so it maps to the right file). The `\`ifdef stack and
    /// macros persist across files (shared compilation unit); unterminated conditionals
    /// are reported once at the end, identically to `run`.
    fn run_sources(&mut self, n: usize) {
        for i in 0..n {
            self.scan_file(FileId(i as u32));
            self.flush_pending_nl();
        }
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
        let map = SourceMap::from_parts(self.files, self.segments);
        PpResult {
            text: self.out,
            map,
            diags: self.diags,
            timescales: self.timescales,
            nettype_none: self.nettype_none,
        }
    }
}
