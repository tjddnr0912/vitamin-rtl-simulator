# hdl-preprocess — Implementation-Ready Specification

**Status:** Implementation-ready. Single source of truth for the `hdl-preprocess` crate.
**Date:** 2026-06-04
**Pipeline position:** `raw source → preprocess → (expanded text + SourceMap) → lex → parse`.
**Scope:** IEEE 1364-2005 §19 compiler directives, Verilog-2005 MVP subset. SystemVerilog-only directives are out of scope (see §13).

The behavioral semantics in this document (line continuation, string/comment verbatim preservation, argument splitting, conditional stack, include/macro cycle guards, pass-through directives, span mapping) are the contract. Sections 1–8 below give the concrete Rust the implementer must write; sections 9–10 give the CLI integration and diag-gate edits verbatim; sections 11–12 give tests and the coverage/out-of-scope tables.

---

## 0. Behavioral ground truth (normative summary)

This is the condensed IEEE semantics the crate implements. The full prose is reproduced after the code sections for reference, but this list is the checklist:

1. **Single left-to-right pass** maintaining: macro table, active-expansion set (recursion guard), include stack (cycle guard), conditional stack (ifdef/ifndef/elsif/else/endif).
2. **Verbatim contexts** suppress all directive/macro scanning: block comments `/* */` (non-nested), line comments `// …\n`, double-quoted strings `"…"`. A backtick inside any of these is literal text. **String rule (IEEE: strings do not span newlines):** within a `"..."` a `\` escapes the next char ONLY when that next char is NOT a newline; a `\` immediately followed by a newline does NOT continue the string. A bare or backslash-preceded NEWLINE terminates the string with `E-PP-BAD-DIRECTIVE` (message "unterminated string literal"; see §8.1 — `E-PP-UNTERMINATED-STRING` is NOT a promoted MsgCode), recovering AT the newline (the newline is not consumed). So `"a\<NL>b"` is an unterminated-string error at the `<NL>`, not a C-style continuation.
3. **Line continuation** `\` + LF (or `\` + CR-LF) joins physical lines into one logical line. This join is a **physical-line-layer** operation applied during macro-body / directive-line capture, and it does **NOT** apply inside string or comment verbatim contexts: while capturing a body, a `\`+NL encountered in ordinary (non-string/comment) text is removed (the two bytes dropped, lines joined); a `\`+NL inside a `"..."` is left to the string scanner, which treats it as the unterminated-string error of item 2 (string contents are never silently joined — "string contents verbatim" is preserved). **Ordering is fixed:** continuation join happens at the physical-line layer first for non-verbatim text; the string scanner then runs on the result and sees the un-joined `\`+NL inside any string.
4. **Directive vs macro:** after a backtick, the maximal simple-identifier run (`[A-Za-z_][A-Za-z0-9_$]*`) is matched. If it is a known directive keyword → directive; else → macro use. A backtick not followed by an identifier-start is `E-PP-BAD-DIRECTIVE`.
5. **`define`** stores raw replacement text; object-like if no `(` immediately follows NAME, function-like if `(` immediately follows (significant-space rule). No expansion at definition time.
6. **Macro use** substitutes the stored body (function-like: textual parameter substitution on whole-identifier matches only, never inside strings/comments of the body), then **re-scans** the inserted region.
7. **Recursion guard:** a use of NAME found inside NAME's own BODY expansion is emitted verbatim and raises `E-PP-RECURSIVE-MACRO`. The active-set entry is scoped to the macro BODY's literal tokens ONLY — argument-derived text is expanded BEFORE substitution, without NAME in the active set, so `NAME` appearing in a user-supplied actual to `NAME(...)` is a sibling use and expands normally (legal `` `A(`A) `` is not flagged).
8. **Argument splitter** splits on top-level commas only, tracking `()`/`[]`/`{}` depth and string/comment context independently.
9. **Conditionals** test definedness only. Skipped regions still track nesting but do not expand/define/undef/include and do not error on unknown directives inside the dead region.
10. **`include "path"`** (double-quoted only) searches current-file dir first, then `incdirs` in order; included text is preprocessed in place sharing the macro table; cycle guard on canonicalized paths on the include stack (`E-PP-RECURSIVE-INCLUDE`).
11. **Pass-through** directives (`timescale`, `default_nettype`, `celldefine`, `endcelldefine`, `resetall`, `line`) are consumed and produce no output / no semantic effect.
12. **SourceMap** lets any expanded-byte offset resolve back to `(file, line, col, orig_byte)` in some original file, across includes and macro expansions.

---

## 1. Public API (`crates/hdl-preprocess/src/lib.rs`)

`std`-only plus the `diag` crate. No `serde`, no `blake3`, no third-party deps. The SourceMap is **not** a serialized artifact in the MVP (it lives only in-process between preprocess and error emission), so it need not derive `SchemaHash`; it nonetheless obeys the determinism rules (integer offsets + `FileId` indices only, no floats, no `usize` in any persisted form).

```rust
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
}

impl Default for PreOpts {
    fn default() -> Self {
        PreOpts {
            incdirs: Vec::new(),
            cli_defines: Vec::new(),
            max_macro_depth: 256,
            max_include_depth: 64,
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
            return ResolvedLoc { file_name: String::new(), line: 1, col: 1, orig_byte: 0 };
        }
        let exp = exp_byte as u32;
        // Binary search: largest segment with exp_start <= exp.
        let idx = match self
            .segments
            .binary_search_by(|s| s.exp_start.cmp(&exp))
        {
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
    let byte = byte.min(src.len());
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
```

`diag::SourceLoc` is `{ file: String, line: u32, col: u32, byte_start: u32, byte_end: u32 }` (per the integration report, `diag/src/event.rs:5-11`). `Severity` and `MsgCode` are the existing `diag` enums.

---

## 2. Internal state and the expansion algorithm

### 2.1 Internal types

```rust
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
    /// opened (= `out.len()` at the moment the frame was pushed). Used to point the
    /// unterminated-conditional error at the ACTUAL opening directive (resolved via
    /// SourceMap), instead of all unclosed frames collapsing to EOF/`out.len()`.
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
    active: BTreeSet<String>,     // recursion guard (names currently expanding)
    cond: Vec<CondFrame>,         // conditional stack
    inc_stack: Vec<PathBuf>,      // canonical paths currently open (cycle guard)
    inc_depth: u32,
    macro_depth: u32,

    /// Whether any directive or macro was seen at all. If false at finish, the
    /// identity fast path is taken (single 1:1 segment).
    saw_directive: bool,
}
```

### 2.2 Emission helpers (segment append)

Output is built by exactly two primitives. **All** writes to `self.out` go through these so `segments` stays complete and sorted.

```rust
impl Preprocessor<'_> {
    /// Append a verbatim run: bytes from `file`'s original text `[orig_start..]`
    /// of length `s.len()`, mapped 1:1. Coalesces with the previous segment if it
    /// is the contiguous continuation of the same file (same FileId, verbatim,
    /// orig contiguous, exp contiguous).
    fn emit_verbatim(&mut self, s: &str, file: FileId, orig_start: u32) {
        if s.is_empty() {
            return;
        }
        let exp_start = self.out.len() as u32;
        if let Some(last) = self.segments.last_mut() {
            // Coalesce only when the previous segment is the *contiguous* verbatim
            // continuation of the SAME file in BOTH the expanded and original byte
            // spaces. `checked_add` guards the u32 sum (a non-contiguous or
            // overflowing run simply does not coalesce — soundness is preserved).
            let prev_orig_end = last
                .orig_start
                .checked_add(last.exp_end - last.exp_start);
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
            collapsed: false, // verbatim run: 1:1 mapping, NOT pinned to a site
        });
    }

    /// Append a collapsed run: `s` (already-expanded text) all attributed to the
    /// single site `(file, site_byte)`.
    fn emit_collapsed(&mut self, s: &str, file: FileId, site_byte: u32) {
        if s.is_empty() {
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
```

> **Segment-list invariant (the implementer MUST uphold this).** Every emit appends
> at `exp_start == self.out.len()`, so `segments` is sorted by `exp_start`, gap-free,
> and contiguous over `0..out.len()` *by construction* — regardless of the original
> byte ordering of verbatim runs. This is exactly what makes the §1 binary search
> sound. A verbatim segment ALWAYS has `collapsed == false`; only `emit_collapsed`
> sets `collapsed == true`. (Test 19 below asserts a pure-verbatim region resolves
> byte-for-byte, which would catch an accidental `collapsed = true`.)

### 2.3 The scanner — numbered algorithm

The scanner processes ONE file's text (`scan_file`). Includes recurse into `scan_file` on the included text. Macro re-scan recurses into `scan_text` on a synthetic buffer (origin pinned to the use site). The single source of truth for "are we emitting?" is `self.emitting()`:

```rust
fn emitting(&self) -> bool {
    self.cond.last().map_or(true, |f| f.active && f.parent_emitting)
}
```

**`scan_file(file: FileId, site_for_collapse: Option<(FileId, u32)>)`** walks `self.files[file].text` (cloned into a local `src: &str` borrow — clone the string up front to avoid borrow conflicts with `self`). It maintains a byte cursor `i` and these steps. `site_for_collapse` is `None` for normal files (output is verbatim, mapped 1:1) and `Some(site)` when the whole text is macro-expansion output that must collapse to a single site (used by the re-scan path). The `file` argument is the **current file** for the duration of this call: any `` `include `` encountered here resolves relative to `self.files[file].dir` (the current file's own directory), NOT the global entry dir — this is the IEEE 1364 §19.3.2 nested-include rule. Because `file` is an explicit parameter, the include handler reads `self.files[file].dir` directly with no extra "current_dir" state to thread.

1. **Loop** while `i < src.len()`.
2. **String literal.** If `src[i] == '"'`:
   - Scan to the closing `"` or to a `\n`. A `\` escapes the next char ONLY when that next char is not `\n`. When the scanner sees a `\` immediately followed by `\n`, it does NOT continue the string: the `\n` terminates it as unterminated (a string never spans a newline). If a `\n` is reached (bare or right after a `\`), push `E-PP-BAD-DIRECTIVE` ("unterminated string literal") at the current output offset and treat the string as closing AT the newline (do not consume the newline).
   - Emit the whole string span verbatim (or collapsed if `site_for_collapse`). Advance `i` past it. **No directive/macro scanning inside, and no line-continuation join inside.** Continue.
3. **Line comment.** If `src[i..]` starts with `//`: scan to end of line (not including `\n`). Emit verbatim/collapsed. Advance. Continue.
4. **Block comment.** If `src[i..]` starts with `/*`: scan to the first `*/` (or EOF). Emit verbatim/collapsed including the delimiters. Advance. Continue.
5. **Backtick.** If `src[i] == '`'` (U+0060):
   - Parse the identifier body after it (step into `parse_ident`). Let `name` = the matched simple identifier, `name_end` = byte after it. If no valid identifier-start follows, push `E-PP-BAD-DIRECTIVE` ("stray backtick"), emit nothing, advance `i` by 1, continue.
   - Set `self.saw_directive = true`.
   - **Dispatch on `name`:**
     - If `name` is a known **directive keyword** (see table §2.5) → call its handler, which consumes from `name_end` onward as needed and updates `i`.
     - Else `name` is a **macro use**: handled per step 6. (But: if `!self.emitting()`, a macro use in a dead region is skipped — emit nothing, advance past the backtick+name, and for function-like definitions do NOT consume a following `(...)`; in dead regions we do not parse arguments. Continue.)
   - Continue.
6. **Macro use (`name` not a directive keyword, emitting).**
   - If `name` is in `self.active`: push `E-PP-RECURSIVE-MACRO`, emit the literal `` `name `` verbatim (so output is finite), advance `i` past `name`, continue.
   - Look up `name` in `self.macros`. If absent → push `E-PP-BAD-DIRECTIVE` ("undefined macro use `name`"), emit the literal `` `name `` verbatim, advance, continue.
   - **Object-like** (`params == None`): take the body. Push `name` into `active`, recurse `scan_text(body, site=(file, backtick_byte), depth+1)` which appends collapsed output, pop `name`. Advance `i` to `name_end` only. **An object-like use NEVER consumes a following `(`** — a `` `FOO `` followed by `(a,b)` expands `FOO` and leaves `(a,b)` as literal following text (the `(` is not an argument list). Continue.
   - **Function-like** (`params == Some(p)`): from `name_end`, skip whitespace/newlines. If the next non-ws char is not `(` → push `E-PP-MACRO-ARITY` ("function-like macro `name` used without argument list"), emit literal `` `name ``, advance to `name_end`, continue. Else call `split_args` (step 7). **If `!result.closed` → push `E-PP-MACRO-ARITY` ("unterminated macro argument list"), emit literal `` `name ``, advance `i` to `result.close` (= EOF), continue — this is an error regardless of the actual count.** Otherwise check arity (`result.actuals.len()` against `p.len()`, applying the empty-actual rule of §2.4), pre-expand each actual to completion (without `name` in `active`), substitute the expanded actuals into the body (step 8), then re-scan ONLY the body-derived text with `active += name`, collapsing to the backtick site. Advance `i` past the closing `)` (`result.close + 1`).
7. **Argument splitting** (`split_args(src, open_paren_idx) -> SplitArgs { actuals, close, closed }`): see §2.4.
8. **Parameter substitution** (`substitute(body, params, actuals) -> String`): scan the body as a mini-lexer that recognizes the SAME verbatim contexts (strings/line-comments/block-comments are copied through, identifiers inside them NOT substituted). For each maximal identifier run in body text that exactly equals a parameter name, replace it with the corresponding actual (raw text). All other characters copied verbatim. Returns the substituted string. **No paste, no stringize.**

   **Recursion-guard scoping (MAJOR — the `` `A(`A) `` boundary).** The active-set entry for `name` must scope to the macro's OWN body tokens ONLY, never to argument-derived text. IEEE forbids a macro appearing inside its OWN expansion-in-progress; it does NOT forbid the macro's NAME appearing in user-supplied argument text that expands to non-recursive output. Therefore:
   - **Expand each actual to completion FIRST** (scan it as ordinary text WITHOUT `name` in `active`), producing already-expanded actuals. A `` `A `` appearing in an argument to `` `A(...) `` is a *sibling* use and expands normally here (it is not inside A's own body expansion).
   - **Then substitute** the already-expanded actuals into the body, and re-scan ONLY the body-derived literal text with `name` held in `active`. Because actuals are pre-expanded raw text, the body re-scan never re-enters argument regions under `active += name`, so a legal `` `A(`A) `` is NOT falsely flagged `E-PP-RECURSIVE-MACRO`. A genuine self-reference inside A's body (`` `define A `A ``) still trips the guard.
   - Equivalent, simpler implementation if pre-expansion of actuals is awkward: substitute raw actuals, then re-scan with `active += name`, but this is REJECTED here precisely because it mis-flags the sibling case. The spec mandates pre-expanded actuals.
9. **Otherwise** (ordinary char): find the next "interesting" byte (the next `"`, `/`, `` ` ``, or end) and emit that whole verbatim run at once for efficiency; if not emitting, drop it. Advance `i`.

> **ASCII-delimiter / char-boundary invariant (MINOR — precludes a "byte index is not a char boundary" panic).** Every delimiter the scanner searches for or slices at — `"`, `/`, `` ` ``, `\n`, `(`, `)`, `,`, `[`, `]`, `{`, `}`, `\` — is a single-byte ASCII char. Therefore a byte search via `src.as_bytes().iter().position(...)` followed by `&src[i..j]` always lands on a char boundary even when strings/comments contain non-ASCII (multibyte UTF-8) content: a multibyte continuation byte can never equal any ASCII delimiter, so `j` is always at the start of a delimiter (or `src.len()`), both char-aligned. `parse_ident` (matching `[A-Za-z0-9_$]`, all ASCII) must advance via `char_indices()` (or byte indices with the same ASCII guarantee) so a multibyte char adjacent to an identifier never splits a slice. The implementer MUST treat these delimiters as ASCII bytes and rely on this invariant; do not index at arbitrary computed offsets.

`scan_text(text, site, depth)` is identical to step-2-onward but every emit is **collapsed** to `site`, and it checks `depth <= opts.max_macro_depth` (else push `E-PP-RECURSIVE-MACRO` and stop). It is used for object-like and function-like re-scan.

### 2.4 Argument splitter (the hard case)

```rust
/// Outcome of argument splitting.
struct SplitArgs {
    /// Trimmed actuals (interior ws/newlines preserved).
    actuals: Vec<String>,
    /// Byte index of the matching ')' on success, or `src.len()` on EOF-before-close.
    close: usize,
    /// `true` iff a matching top-level ')' was found before EOF.
    closed: bool,
}

/// Split actuals starting just after `open` (the index of '('). Top-level commas
/// split; commas inside () [] {} (independent depth counters) or inside
/// string/line/block-comment context do NOT split. Each actual is trimmed of
/// leading/trailing ws; interior ws (incl. newlines) preserved. The caller maps a
/// single whitespace-only actual `[""]` to `[]` for a zero-param macro; the caller
/// decides. `closed == false` is ALWAYS an error at the call site, independent of
/// the actual count.
fn split_args(src: &str, open: usize) -> SplitArgs { /* per algorithm below */ }
```

Algorithm: start `i = open + 1`, `depth_paren = 0`, `depth_brack = 0`, `depth_brace = 0`, `cur = String::new()`, `args = vec![]`. Loop while `i < src.len()`:

- `"` → copy the whole string literal (respecting `\` escapes; stop at `"` or newline) into `cur`.
- `//` → copy to end of line into `cur`. `/*` → copy to `*/` into `cur`.
- `(` `[` `{` → increment the matching depth (`depth_paren`/`depth_brack`/`depth_brace`), push char to `cur`.
- `)` → if all three depths are 0, this is the closing paren: push the final `cur` (trimmed) to `args`, return `SplitArgs { actuals: args, close: i, closed: true }`. Else **only if `depth_paren > 0`** decrement `depth_paren`; push char.
- `]` → **only if `depth_brack > 0`** decrement `depth_brack`; push char. (A top-level unmatched `]` is an ordinary literal char copied into `cur` — never a depth event.)
- `}` → **only if `depth_brace > 0`** decrement `depth_brace`; push char. (Same: top-level unmatched `}` is literal.)
- `,` → if all three depths are 0, push `trim(cur)` to `args`, reset `cur`. Else push char.
- any other char → push to `cur`.
- EOF before a top-level `)` → push `trim(cur)` to `args` and return `SplitArgs { actuals: args, close: src.len(), closed: false }`.

> **Unsigned-depth guard (BLOCKER).** The depth counters are unsigned; NEVER write a
> bare `depth -= 1`. Every decrement is gated by an explicit `> 0` check (or
> `saturating_sub`) so a top-level unmatched `]`/`}`/`)` — which is legal literal
> text inside an actual — is copied verbatim into `cur` instead of underflowing the
> counter (debug panic / release wraparound that would mis-classify a later top-level
> `,`/`)` as nested). Only the matching opener may increment a counter, so a real
> top-level `)` always satisfies "all three depths are 0".

**Caller obligation (BLOCKER — unterminated call is always an error).** After
`split_args`, the function-like use handler MUST check `closed`. If `!closed`, emit
`E-PP-MACRO-ARITY` ("unterminated macro argument list for `` `name ``") **regardless of
the actual count**, leave the literal `` `name `` (do not expand), and advance `i` to
`close` (= `src.len()`). Only when `closed == true` does it proceed to the arity check
and substitution.

**Empty-actual rule:** `NAME(,)` yields `["", ""]` (two empty actuals). `NAME()` with
whitespace-only between parens yields `[""]` from the splitter; the caller maps `[""]`
to `[]` (zero actuals) **iff** the macro is declared with zero params; otherwise `[""]`
is one empty actual.

### 2.5 Directive keyword table and handlers

```rust
fn is_directive_kw(name: &str) -> bool {
    matches!(name,
        "define" | "undef" | "include"
        | "ifdef" | "ifndef" | "elsif" | "else" | "endif"
        | "timescale" | "default_nettype" | "celldefine" | "endcelldefine"
        | "resetall" | "line")
}
```

Handler summary (all advance the cursor `i`):

- **`define`**: read NAME. **If NAME is a directive keyword (`is_directive_kw`) → `E-PP-BAD-DIRECTIVE` ("cannot define a directive keyword as a macro"), strip the line, do not store** (symmetric with `undef`; without this guard a `` `define ifdef ... `` would store a dead macro that step 5 can never dispatch to). Then disambiguate **object vs function by whether `(` IMMEDIATELY follows NAME with NO intervening whitespace** (significant-space rule): `` `define F(x) `` is function-like; `` `define F (x) `` is OBJECT-like with body `(x) …`. Capture the logical line (continuation-joined at the physical-line layer FIRST — see §0 item 3). Parse params for function-like (reject `=` defaults and duplicate param names → `E-PP-BAD-DIRECTIVE`). Compute body: trim leading ws after NAME/param-list, drop the trailing newline. **Ordering for a `//` line comment in the body:** continuation join happens first (physical layer); THEN within the joined logical line a `//` truncates the body at its position and the comment text (including any joined continuation) is dropped — so `` `define X a //c\<NL>d `` has body `a`. Redefinition: if an existing macro is byte-identical (same params + same body) → silent; if it differs → store new + `W-PP-MACRO-REDEFINED`. Only acts when `emitting()`. Emits **no output** (the directive line is stripped). Record `def_file`/`def_byte` at the body start.
- **`undef`**: read one identifier. If the name is a directive keyword → `E-PP-BAD-DIRECTIVE` (symmetric with `define`). Else if defined → remove; if not defined → `W-PP-UNDEF-UNDEFINED`. Strips the line. Only acts when `emitting()`.
- **`include`** (only acts when `emitting()`; honors `max_include_depth`):
  1. **Capture** the rest of the logical line from `name_end` (continuation-joined; a `//` line comment ends the line and is dropped; do NOT scan into strings here other than respecting that a `"..."` is a single token).
  2. **Macro-expand** that captured text with a full expansion sub-pass (run the same scanner on it to a stable result, i.e. re-scan expansions until no further macro use remains, bounded by `max_macro_depth`; an unexpanded nested macro inside the path token must be resolved — "expand once" was ambiguous, so this is fixpoint expansion). Collect only the resulting text (no provenance is needed for the stripped directive line).
  3. **Parse the result** requiring: optional ws/comment, then exactly one double-quoted `"..."` token, then optional ws/comment, and nothing else. Anything else (zero strings, two strings, a `<...>` angle form, trailing tokens) → `E-PP-BAD-DIRECTIVE` ("`include requires a single quoted path").
  4. **Extract the inner bytes** of the string (strip the surrounding quotes only; for the MVP NO escape processing is performed on the path — a literal `\` in the path is kept as-is, documented in §10). That inner byte slice is the `request`.
  5. **Resolve** via `reader.resolve(request, current_dir, &opts.incdirs)` where `current_dir = self.files[file].dir` — the CURRENT file's own directory (the file being scanned), NOT the global entry `base_dir`. This is the IEEE nested-include rule: `a/sub/b.svh` doing `` `include "c.svh" `` looks in `a/sub/` first. Not found → `E-PP-INCLUDE-NOT-FOUND`.
  6. **Cycle guard:** if the returned canonical path is already on `inc_stack` → `E-PP-RECURSIVE-INCLUDE`, skip (do not register, do not recurse).
  7. **Register** the new `SourceFileEntry { name, text, canon: Some(canon), dir }` where `dir = canon.parent()` (the included file's OWN directory, for ITS nested includes) — fall back to `current_dir` if `canon` has no parent. Push canon onto `inc_stack`, increment `inc_depth`, call `scan_file(new_file, None)`, then pop `inc_stack` and decrement `inc_depth`.
  8. Strips the directive line; the included file's emitted text is verbatim-mapped to ITS file entry.
- **`ifdef NAME` / `ifndef NAME`**: read NAME. Push a `CondFrame { active = (defined == (kw==ifdef)), taken = active, seen_else = false, parent_emitting = self.emitting(), open_at = self.out.len() as u32 }`. Always interpreted (even in dead regions, to keep nesting balanced) — but `parent_emitting=false` makes the whole group inert. (In a dead region the `defined(NAME)` test is still computed for `active`/`taken`, but `emitting()` stays false because `parent_emitting` is false, so no output is produced regardless.)
- **`elsif NAME`**: error `E-PP-BAD-DIRECTIVE` if no open group or `seen_else`. Else: if `taken` already true → `active=false`; else if NAME defined → `active=true; taken=true`; else `active=false`.
- **`else`**: error if no open group or `seen_else`. Else `seen_else=true`; `active = !taken`; `taken=true`.
- **`endif`**: error `E-PP-BAD-DIRECTIVE` if no open group; else pop.
- **`timescale` / `line`**: consume the rest of the logical line, discard. (`line` renumbering NOT applied — out of scope.)
- **`default_nettype`**: consume one whitespace-delimited token, discard.
- **`celldefine` / `endcelldefine` / `resetall`**: take no args, drop. `resetall` does NOT clear the macro table in MVP (documented divergence).

All directive lines produce **no output**, so a directive on its own line leaves the surrounding newline handling to the verbatim runs around it (the newline before the backtick is verbatim; the directive's own trailing newline is consumed by the line-capture and thus dropped — acceptable because the lexer is whitespace-insensitive and the SourceMap still resolves following tokens to their true lines via verbatim segments).

### 2.6 `run` / `finish`

```rust
impl<'a> Preprocessor<'a> {
    fn new(base_dir: &Path, name: &str, src: &str, opts: &'a PreOpts, reader: &'a dyn IncludeReader) -> Self {
        // files[0] = entry: SourceFileEntry { name, text: src, canon: None,
        //   dir: base_dir.to_path_buf() }. The entry file has no resolved path, so
        //   its own include-search root is base_dir. Apply cli_defines as
        //   object-like macros (def_file=0, def_byte=0).
    }
    fn run(&mut self) {
        // scan_file(FileId(0), None); then at EOF, for each still-open CondFrame
        // push E-PP-BAD-DIRECTIVE ("unterminated `ifdef/`ifndef") at the frame's
        // recorded opening offset (open_at), NOT at out.len() — see §2.1 CondFrame.
    }
    fn finish(self) -> PpResult {
        // Identity fast path: if !saw_directive AND files.len()==1, segments is a
        // single verbatim [0..len) -> ensure that (emit_verbatim already coalesced
        // to one). Build SourceMap { files, segments }, return PpResult.
    }
}
```

---

## 3. Source map design (recap of the concrete decisions)

- **File table** `Vec<SourceFileEntry>`: index = `FileId`. Entry 0 is the top-level source (`dir = base_dir`, `canon = None`). Each `include` appends one entry (on cycle-skip or not-found we do NOT append; the skipped file is not emitted). `text` holds the ORIGINAL file text; `byte_to_line_col` always runs against it. `dir` holds that file's OWN directory (parent of its `canon`, or `base_dir` for the entry), used as the first search root for includes appearing inside it — never the global entry dir for an included file.
- **Segment list** `Vec<Segment>`: sorted by `exp_start`, contiguous, gap-free, covering `0..expanded.len()`. Built incrementally by `emit_verbatim` (1:1, coalescing) and `emit_collapsed` (pins all bytes to one site). Because every write goes through these two helpers and `exp_start` is always `self.out.len()` at append time, the list is sorted by construction — no sort needed.
- **resolve**: empty-map guard FIRST, then `binary_search_by(exp_start)`, take the segment at-or-before the offset; verbatim → `orig_start + delta` with `delta` clamped to the segment width and `checked_add` for the sum (so an EOF-clamped offset lands on the last real byte of the segment, never past it, never overflowing), collapsed → `orig_start`; then `byte_to_line_col` against that file. O(log n).
- **Identity fast path**: when no directive/macro is present and there is one file, `segments == [Segment{0..len, file 0, orig 0, verbatim}]`, so `resolve(b)` is exactly `byte_to_line_col(entry, b)` — byte-for-byte identical to today's behavior.
- **Provenance choice**: expanded macro text and substituted argument text collapse to the **macro-use site** (the backtick offset). This is the minimal-but-sound choice the task mandates: an error anywhere inside an expansion points at the user's instantiation. (Definition-site and argument-site provenance are recorded only as `def_byte` for future `diag::Frame` stacks; not surfaced in MVP `SourceLoc`.)

---

## 4. CLI integration (`crates/cli/src/lib.rs`)

### 4.1 Cargo dependency

Add to `crates/cli/Cargo.toml` under `[dependencies]`:

```toml
hdl-preprocess = { path = "../hdl-preprocess" }
```

### 4.2 The hook site in `run_vita_str`

**BEFORE** (lib.rs:177-179, verbatim):

```rust
    // preprocess is a passthrough in v1 — the text IS the lexer input.
    // ── lex ──────────────────────────────────────────────────────────────
    let (tokens, lex_errors) = hdl_lexer::lex(text);
```

and the subsequent parse call (lib.rs ~191, verbatim — note the binding is `unit`, not `ast`):

```rust
    let (unit, parse_errors) = hdl_parser::parse(&tokens, text);
```

**AFTER:**

```rust
    // ── preprocess ─────────────────────────────────────────────────────────
    // raw source -> expanded text + SourceMap. The expanded text (not `text`) is
    // what the lexer and parser consume; spans they produce index the expanded
    // buffer and resolve back to original files via `pp.map`.
    let base_dir = std::path::Path::new(file)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pre_opts = hdl_preprocess::PreOpts::default(); // incdirs/-D wired from opts in run_vita
    let pp = hdl_preprocess::preprocess_str(base_dir, file, text, &pre_opts);
    for d in &pp.diags {
        let loc = pp.map.resolve_span(d.at, d.at);
        // `sink` is `let sink = StderrSink::new();` (owned by value, &self emit) —
        // wrap in LogEvent::Diagnostic and include EVERY field of diag::Diagnostic
        // (severity, code, message, location, context, sim_time). This matches the
        // two existing call sites at lib.rs:159 and lib.rs:211 exactly.
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: d.severity,
            code: d.code,
            message: d.message.clone(),
            location: Some(loc),
            context: Vec::new(),
            sim_time: None,
        }));
    }
    if pp.has_errors() {
        return EXIT_USER_ERROR;
    }
    let expanded: &str = &pp.text;

    // ── lex ──────────────────────────────────────────────────────────────
    let (tokens, lex_errors) = hdl_lexer::lex(expanded);
```

and:

```rust
    let (unit, parse_errors) = hdl_parser::parse(&tokens, expanded);
```

> `Diagnostic`, `LogEvent`, `LogSink`, `Severity`, `MsgCode`, `SourceLoc` are already
> imported in `cli/src/lib.rs` (`use diag::{...}` at lib.rs:20). `diag::Diagnostic` has
> SIX fields — `{ severity, code, message, location, context, sim_time }`
> (`diag/src/event.rs:27-34`); the literal above includes all six. `emit` takes
> `&self`, so the by-value `sink` works without `&mut`. Do NOT call `sink.emit(diag::Diagnostic{...})` directly — `LogSink::emit` takes a `LogEvent`, never a bare `Diagnostic`.

### 4.3 Location resolution change

`emit_frontend_error` and `loc_from_span` change from operating over a single `(file, src)` string to resolving through the SourceMap. The expanded-text byte offsets coming out of lex/parse are passed unchanged; only their interpretation changes.

**BEFORE** (lib.rs:139-148, `loc_from_span` — actual current code):

```rust
fn loc_from_span(file: &str, src: &str, lo: usize, hi: usize) -> SourceLoc {
    let (line, col) = byte_to_line_col(src, lo);
    SourceLoc {
        file: file.to_string(),
        line,
        col,
        byte_start: lo as u32,
        byte_end: hi as u32,
    }
}
```

**AFTER:**

```rust
fn loc_from_span(map: &hdl_preprocess::SourceMap, lo: usize, hi: usize) -> SourceLoc {
    map.resolve_span(lo, hi)
}
```

**BEFORE** (lib.rs:150-167, `emit_frontend_error` — actual current signature; note `sink: &StderrSink`, not `&mut impl LogSink`):

```rust
fn emit_frontend_error(
    sink: &StderrSink,
    file: &str,
    src: &str,
    lo: usize,
    hi: usize,
    msg: String,
) {
    sink.emit(LogEvent::Diagnostic(Diagnostic {
        severity: Severity::Error,
        code: MsgCode::ParseUnexpectedToken,
        message: msg,
        location: Some(loc_from_span(file, src, lo, hi)),
        context: Vec::new(),
        sim_time: None,
    }));
}
```

**AFTER:**

```rust
fn emit_frontend_error(
    sink: &StderrSink,
    map: &hdl_preprocess::SourceMap,
    lo: usize,
    hi: usize,
    msg: String,
) {
    sink.emit(LogEvent::Diagnostic(Diagnostic {
        severity: Severity::Error,
        code: MsgCode::ParseUnexpectedToken,
        message: msg,
        location: Some(loc_from_span(map, lo, hi)),
        context: Vec::new(),
        sim_time: None,
    }));
}
```

The two call sites (lex loop lib.rs:180-184 and parse loop lib.rs:194-208) pass `&pp.map` instead of `(file, text)`. The span values (`e.span.start`/`e.span.end` for lex, `e.span.lo as usize`/`e.span.hi as usize` for parse) are unchanged. `byte_to_line_col` stays in `cli` (`hdl_preprocess` carries its own byte-identical copy used internally by `SourceMap`); both must agree so line/col numbers match.

### 4.4 No-preprocessing fast path is byte-identical

When the source contains no backtick directives and no macro uses and there are no includes, `pp.text == text` (the scanner copies every byte verbatim), `pp.diags` is empty, and `pp.map` is a single 1:1 verbatim segment over file 0. Therefore `pp.map.resolve_span(lo, hi)` returns exactly `SourceLoc { file, byte_to_line_col(text, lo), lo, hi }` — i.e. identical to today's `loc_from_span(file, text, lo, hi)`. The lexer/parser receive a string equal to `text`. The CLI behavior is unchanged byte-for-byte for non-preprocessed inputs.

### 4.5 `run_vita` (multi-file) and incdirs/defines wiring

`run_vita` (lib.rs:266-296) keeps owning top-level `sources` reading + concatenation. It constructs `PreOpts` from any `-I`/`-D` CLI options and passes them down (thread `PreOpts` into `run_vita_str`, or build it there). The first source's directory is `base_dir`. Include reading lives entirely inside `hdl-preprocess` via `FsIncludeReader`; `run_vita` does not read includes.

---

## 5. Diag additions (MsgCode bijection gate)

Per the diag-gate recipe, promote five Appendix-A reserved codes into doc-15 body + enum, bumping the bijection count by 5 (38 → 43). The crate uses exactly these five plus the two already-promoted (`PpIncludeNotFound` E1001, `PpMacroArity` E1002).

### 5.1 `crates/diag/src/code.rs` — add to the `// 1xxx PREPROCESS` block (after line 41)

```rust
PpRecursiveMacro       => ("E-PP-RECURSIVE-MACRO",     "VITA-E1004", Error,   "recursive text-macro expansion"),
PpRecursiveInclude     => ("E-PP-RECURSIVE-INCLUDE",   "VITA-E1005", Error,   "cyclic `include (file includes itself)"),
PpBadDirective         => ("E-PP-BAD-DIRECTIVE",       "VITA-E1013", Error,   "unknown compiler directive"),
PpMacroRedefined       => ("W-PP-MACRO-REDEFINED",     "VITA-W1007", Warning, "`define redefines a macro with different text"),
PpUndefUndefined       => ("W-PP-UNDEF-UNDEFINED",     "VITA-W1008", Warning, "`undef of a macro that was never defined"),
```

Numbers are permanently assigned in the appendix; reuse verbatim (renumbering forbidden). Non-contiguous numbers within the band are allowed; the mnemonic is the stable key.

### 5.2 `crates/diag/tests/bijection.rs` — bump the count (line 68)

```rust
assert_eq!(enum_codes.len(), 43, "MsgCode must have exactly 43 body variants");   // was 38
```

No other test changes — set-equality, number/severity cross-check, and uniqueness flow automatically from the doc-body and enum edits.

### 5.3 `docs/preview/15-error-code-reference.md` — body §1xxx (insert in numeric order after `### VITA-W1003`)

Each entry follows the house style: header `### VITA-Xnnnn · \`MNEMONIC\` (FullSeverity)`, a cause paragraph, a fenced example, a `해결` line. Add:

```markdown
### VITA-E1004 · `E-PP-RECURSIVE-MACRO` (Error)

텍스트 매크로가 자기 자신의 확장 도중 다시 호출되어 무한 확장에 빠졌다. 전처리기는 활성 확장 집합(active-expansion set)으로 이를 감지하고 해당 사용을 리터럴로 남긴 뒤 이 에러를 보고한다.

```verilog
`define A `A
`A    // E-PP-RECURSIVE-MACRO
```

해결: 매크로 본문에서 자기 참조를 제거하거나, 재귀 대신 충분히 펼친 형태로 정의한다.

### VITA-E1005 · `E-PP-RECURSIVE-INCLUDE` (Error)

`include 체인이 순환하여 이미 열려 있는 파일을 다시 포함하려 했다. canonical 경로 스택으로 감지하고 재포함을 건너뛴다.

```verilog
// a.svh
`include "b.svh"
// b.svh
`include "a.svh"   // E-PP-RECURSIVE-INCLUDE
```

해결: 순환 포함을 제거하거나 include guard(`ifndef/`define/`endif)를 사용한다.

### VITA-E1013 · `E-PP-BAD-DIRECTIVE` (Error)

알 수 없는 컴파일러 지시어, 미정의 매크로 사용, 떠돌이 backtick, 범위 밖 지시어 형태, 불균형/중복 조건부, 비리터럴 include 인자, 지시어 이름에 대한 `undef 등 전처리 형식 오류 전반을 포괄한다.

```verilog
`frobnicate        // E-PP-BAD-DIRECTIVE (unknown directive)
`UNDEFINED_MACRO   // E-PP-BAD-DIRECTIVE (undefined macro use)
`endif             // E-PP-BAD-DIRECTIVE (no open conditional)
```

해결: 지시어 철자를 확인하거나, 매크로를 먼저 `define 하거나, 조건부 블록의 짝을 맞춘다.

### VITA-W1007 · `W-PP-MACRO-REDEFINED` (Warning)

`define가 기존 매크로를 다른 본문/파라미터로 재정의했다. 새 정의가 적용되며 경고만 발생한다(동일 본문 재정의는 무경고).

```verilog
`define W 1
`define W 2   // W-PP-MACRO-REDEFINED
```

해결: 의도된 재정의가 아니면 매크로 이름을 분리하거나 `undef 후 재정의한다.

### VITA-W1008 · `W-PP-UNDEF-UNDEFINED` (Warning)

`undef가 현재 정의되지 않은 이름을 대상으로 했다. 동작은 무해하며 경고만 발생한다.

```verilog
`undef NEVER_DEFINED   // W-PP-UNDEF-UNDEFINED
```

해결: 대상 매크로 이름의 철자를 확인하거나, 정의 이후에만 `undef 한다.
```

### 5.4 `docs/preview/15-error-code-reference.md` — Appendix A §1xxx table (lines 555-572)

- Remove the five promoted rows (E1004, E1005, E1013, W1007, W1008) from the `### 1xxx · PREPROCESS` table.
- Change the heading count `### 1xxx · PREPROCESS (14)` → `(9)`.
- Add a `> 참고:` note above the table (E3009/E3010 precedent): `> 참고: E1004/E1005/E1013/W1007/W1008는 preprocessor MVP에서 본문 §1xxx로 승격되었다(2026-06-04).`

### 5.5 Verify

`cargo test -p diag --locked` then `cargo test --workspace --locked`. The bijection test is the gate.

---

## 6. `crates/hdl-preprocess/Cargo.toml`

The crate already exists as a workspace member with the standard header. **Keep workspace
inheritance** (do NOT hardcode `version`/`edition`/`rust-version`/`license` literals — that
diverges from every sibling crate and bumps the version). The existing header is:

```toml
[package]
name = "hdl-preprocess"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
```

Add ONLY the dependency wiring under it:

```toml
[dependencies]
diag = { path = "../diag" }
```

No dev-dependencies are needed: the §7 test suite is dep-free (it uses the in-memory
`MemReader` shim, never `std::fs`). Do not add `tempfile`; the workspace prizes a thin
dependency tree and the in-memory shim covers every include test.

`crates/cli/Cargo.toml` gains:

```toml
hdl-preprocess = { path = "../hdl-preprocess" }
```

---

## 7. Unit tests (`crates/hdl-preprocess/src/lib.rs` `#[cfg(test)]`)

**25 unit tests** (tests 1–18 original behavior; 19–25 added by the adversarial-review
hardening: verbatim byte-for-byte resolve, define significant-space, recursion-guard
argument scoping, unterminated macro call, distinct unclosed-`ifdef` lines,
include-path-via-macro, nested-include directory resolution). A test-only in-memory
include reader keeps `std::fs` out of the suite:

```rust
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
        /// "." components, pop on "..".
        fn norm(p: &Path) -> String {
            let mut parts: Vec<&str> = Vec::new();
            for comp in p.to_string_lossy().split('/') {
                match comp {
                    "" | "." => {}
                    ".." => { parts.pop(); }
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
    /// `files` keys are paths RELATIVE to "/virtual" (joined for the shim map).
    fn pp_mem(src: &str, files: &[(&str, &str)]) -> PpResult {
        let reader = MemReader {
            files: files
                .iter()
                .map(|(k, v)| (MemReader::norm(&Path::new("/virtual").join(k)), v.to_string()))
                .collect(),
        };
        preprocess_with(Path::new("/virtual"), "top.sv", src, &PreOpts::default(), &reader)
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

    // 17. SOURCE-MAP round trip: an error byte in expanded text resolves to the
    //     correct ORIGINAL line. A function-like macro shifts byte offsets so a
    //     naive (expanded) line count would be wrong; the map must recover the
    //     original line.
    #[test]
    fn source_map_round_trip() {
        // Line 1: define. Line 2: blank. Line 3: the use site. The expanded text
        // for the use is shorter/longer than the source, so map.resolve of the
        // expansion's expanded offset must report original line 3.
        let src = "`define P qq\n\nz = `P + bad;\n";
        let r = pp(src);
        assert!(r.diags.is_empty());
        // Find the expanded offset of "bad".
        let exp_off = r.text.find("bad").unwrap();
        let loc = r.map.resolve(exp_off);
        assert_eq!(loc.file_name, "top.sv");
        assert_eq!(loc.line, 3, "expanded offset must map back to original line 3");
        // And the expanded offset of the expansion 'qq' collapses to the use site
        // (line 3, the backtick), not the definition (line 1).
        let exp_qq = r.text.find("qq").unwrap();
        let loc_qq = r.map.resolve(exp_qq);
        assert_eq!(loc_qq.line, 3, "expanded macro text collapses to the use site");
    }

    // 18. unterminated string is reported (as E-PP-BAD-DIRECTIVE per §8.1) but
    //     scanning continues. E-PP-UNTERMINATED-STRING is NOT a promoted MsgCode;
    //     the unterminated-string path emits PpBadDirective with the message
    //     "unterminated string literal".
    #[test]
    fn unterminated_string_reported() {
        let r = pp("v = \"abc\nnext;\n");
        assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);
    }

    // 19. SOURCE-MAP verbatim fidelity: a pure-verbatim region (no directives) must
    //     resolve byte-for-byte (delta applied), which catches an accidental
    //     collapsed=true on a verbatim segment.
    #[test]
    fn verbatim_region_resolves_byte_for_byte() {
        let src = "module m;\n  wire w;\nendmodule\n";
        let r = pp(src);
        assert!(r.diags.is_empty());
        assert_eq!(r.text, src); // identity fast path
        // Every byte resolves to its own original position (line/col match a direct
        // byte_to_line_col), proving the segment is verbatim (delta applied), not
        // collapsed to one site.
        for off in [0usize, 10, 20, src.len()] {
            let loc = r.map.resolve(off);
            let (line, col) = byte_to_line_col(src, off);
            assert_eq!((loc.line, loc.col), (line, col), "verbatim off={off}");
            assert_eq!(loc.orig_byte as usize, off.min(src.len()));
        }
    }

    // 20. define significant-space rule: `define F (x) is OBJECT-like (body "(x)"),
    //     while `define G(x) is function-like. The most error-prone IEEE rule.
    #[test]
    fn define_significant_space_makes_object_like() {
        // Space before '(' => object-like; `F expands to its body "(x)".
        let r1 = pp("`define F (x)\ny = `F;\n");
        assert!(r1.diags.is_empty(), "got {:?}", codes(&r1));
        assert_eq!(r1.text, "\ny = (x);\n");
        // No space => function-like; `G(1) expands the param.
        let r2 = pp("`define G(x) (x+1)\ny = `G(1);\n");
        assert!(r2.diags.is_empty(), "got {:?}", codes(&r2));
        assert_eq!(r2.text, "\ny = (1+1);\n");
    }

    // 21. recursion guard scoping: `A passed as an ARGUMENT to a same-named
    //     function-like `A is a SIBLING use, expands normally, NOT flagged
    //     recursive. (Pre-expanded actuals; active-set scoped to body only.)
    #[test]
    fn macro_name_in_argument_is_not_recursive() {
        // B is object-like; A(x) wraps x. `A(`B) -> the actual `B expands to "z"
        // first, then A's body wraps it. No recursion, no diagnostic.
        let r = pp("`define B z\n`define A(x) [x]\ny = `A(`B);\n");
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert_eq!(r.text, "\ny = [z];\n");
    }

    // 22. unterminated macro argument list is ALWAYS an error, even when the actual
    //     count happens to match arity.
    #[test]
    fn unterminated_macro_call_errors() {
        let r = pp("`define MAX(a,b) ((a)>(b)?(a):(b))\nz = `MAX(p, q\n");
        assert_eq!(codes(&r), vec!["E-PP-MACRO-ARITY"]);
    }

    // 23. two unclosed `ifdef`s report at two DISTINCT lines (resolved via the map
    //     to each frame's opening directive), not both collapsed to EOF.
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

    // 24. include path supplied via a macro: `define INC "f.svh" then `include `INC.
    #[test]
    fn include_path_via_macro() {
        let r = pp_mem(
            "`define INC \"f.svh\"\n`include `INC\nwire [`W-1:0] b;\n",
            &[("f.svh", "`define W 4\n")],
        );
        assert!(r.diags.is_empty(), "got {:?}", codes(&r));
        assert!(r.text.contains("wire [4-1:0] b;"));
    }

    // 25. NESTED include resolves relative to the INCLUDING file's own directory:
    //     top.sv (dir /virtual) includes "sub/b.svh"; b.svh (dir /virtual/sub)
    //     includes "c.svh", which must resolve to /virtual/sub/c.svh — NOT
    //     /virtual/c.svh (the entry dir). A c.svh placed only in sub/ proves it.
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
}
```

> Note: tests 5, 14, 15 assert an EXACT single-element `codes` vector — the implementation must emit exactly those diagnostics and no spurious extras (e.g. test 14 must not warn on the identical `1`→`1` redefine). Test 18 asserts `E-PP-BAD-DIRECTIVE` per the single source of truth in §8.1 (`E-PP-UNTERMINATED-STRING` is intentionally NOT a promoted MsgCode; the unterminated-string path emits `MsgCode::PpBadDirective` with message "unterminated string literal"). The only remaining mentions of the name `E-PP-UNTERMINATED-STRING` (§0 item 2, §8.1, this note) exist solely to state that it is NOT a code and is folded into `E-PP-BAD-DIRECTIVE`; no test or coverage row asserts it.

### 7.1 CLI-level smoke (description)

A `.sv` file whose body uses a function-like macro, e.g.:

```verilog
`define REG(name, w) reg [w-1:0] name
module m; `REG(count, 8); endmodule
```

Driven end-to-end through `run_vita_str`: preprocess expands the macro to `module m; reg [8-1:0] count; endmodule`, the lexer tokenizes the expanded text, the parser builds the module AST, elaboration/sim run as normal. A deliberate undefined identifier inside the macro actual (e.g. `` `REG(count, undef_w) ``) produces a downstream parse/elaborate diagnostic whose `SourceLoc` resolves (via `pp.map`) to the macro-use line in the original `.sv` file, not to a phantom expanded line. Assert the emitted diagnostic's reported line equals the source line of the `` `REG(...) `` call.

---

## 8. Design decisions locked for the implementer

1. **Unterminated string** (`E-PP-UNTERMINATED-STRING`): NOT in the five-code promotion set. **Decision:** for the MVP, emit it as `E-PP-BAD-DIRECTIVE` with message text "unterminated string literal" — reusing the catch-all keeps the bijection count at +5. Test 18 asserts `E-PP-BAD-DIRECTIVE`. (If a dedicated code is later wanted, promote E1012/whatever the appendix assigns and bump the count by one more; out of scope now.) Update test 18 accordingly: `assert_eq!(codes(&r), vec!["E-PP-BAD-DIRECTIVE"]);`
2. **`E-PP-RECURSION` vs `E-PP-RECURSIVE-MACRO`**: the IEEE prose names it `E-PP-RECURSION`; the promoted MsgCode is `E-PP-RECURSIVE-MACRO` (E1004). Use `E-PP-RECURSIVE-MACRO` (the gated name) in all code/tests.
3. **`E-PP-INCLUDE-CYCLE` vs `E-PP-RECURSIVE-INCLUDE`**: prose says `E-PP-INCLUDE-CYCLE`; gated code is `E-PP-RECURSIVE-INCLUDE` (E1005). Use the gated name.
4. **Diagnostic offset `at`** is always a valid index into `expanded` (clamp to `expanded.len()`), so `resolve` never panics.
5. **Empty input / no-directive input** → identity fast path (one verbatim segment), output equals input byte-for-byte.

---

## 9. Coverage table (directive → behavior)

| Directive | Behavior | Diag on misuse |
|---|---|---|
| `` `define `` (object) | store raw body (no expansion at def time) | redefine-different → `W-PP-MACRO-REDEFINED` |
| `` `define `` (function) | store params+body; significant-space `(` detection | dup param / `=` default → `E-PP-BAD-DIRECTIVE` |
| `` `NAME `` (object use) | substitute body, re-scan | self-use in expansion → `E-PP-RECURSIVE-MACRO`; undefined → `E-PP-BAD-DIRECTIVE` |
| `` `NAME(...) `` (function use) | split args (nesting/string aware), substitute, re-scan | wrong arity / no arglist → `E-PP-MACRO-ARITY` |
| `` `undef `` | remove from table | undefined → `W-PP-UNDEF-UNDEFINED`; directive name → `E-PP-BAD-DIRECTIVE` |
| `` `ifdef `` / `` `ifndef `` | push cond frame (definedness test) | — |
| `` `elsif `` | activate iff not taken and defined | no group / after else → `E-PP-BAD-DIRECTIVE` |
| `` `else `` | activate iff not taken | no group / second else → `E-PP-BAD-DIRECTIVE` |
| `` `endif `` | pop cond frame | no group → `E-PP-BAD-DIRECTIVE`; EOF unclosed → `E-PP-BAD-DIRECTIVE` |
| `` `include "..." `` | resolve (cur dir → incdirs), preprocess in place, shared macro table | not found → `E-PP-INCLUDE-NOT-FOUND`; cycle → `E-PP-RECURSIVE-INCLUDE`; non-literal arg → `E-PP-BAD-DIRECTIVE` |
| `` `timescale `` | strip rest of line | — |
| `` `default_nettype `` | strip one token | — |
| `` `line `` | strip rest of line (no renumber) | — |
| `` `celldefine `` / `` `endcelldefine `` / `` `resetall `` | drop (resetall does NOT clear macros) | — |
| unknown `` `xxx `` | error | `E-PP-BAD-DIRECTIVE` |
| stray `` ` `` | error | `E-PP-BAD-DIRECTIVE` |

**Verbatim contexts** (no scanning inside): block comment `/* */`, line comment `//`, string `"..."`. Unterminated string → `E-PP-BAD-DIRECTIVE` (per §8.1), recover at newline.

---

## 10. Out-of-scope / deferred (explicit)

Recognized enough to reject cleanly with `E-PP-BAD-DIRECTIVE` where a directive token is involved, but NOT implemented in MVP:

1. **`` `define `` default argument values** — `` `define NAME(a=expr) ``.
2. **Token paste** `` `` `` (grave-grave) concatenation.
3. **Stringize** `` `" ... `" `` and the escaped-quote `` `\`" `` form.
4. **`` `line `` renumbering semantics** — stripped, but its effect on downstream spans is not applied.
5. **`` `ifdef ``/`` `elsif `` on a macro VALUE / constant expression** — definedness only.
6. **Angle-bracket include** `` `include <...> `` — only double-quoted is supported.
7. **SystemVerilog-only directives** (`` `__FILE__ ``, `` `__LINE__ ``, `` `pragma ``, `` `begin_keywords ``/`` `end_keywords ``, `` `unconnected_drive `` etc.) — not recognized; fall to `E-PP-BAD-DIRECTIVE`.
8. **SourceMap as a serialized artifact** — in-process only in MVP; no `SchemaHash`/`postcard` encoding. (`diag::Frame` expansion-stack context is recorded as `def_byte` but not surfaced; deferred.)
9. **`` `default_nettype `` semantic effect** — the token (`none`/`wire`/`tri`/…) is consumed and DISCARDED; the directive's effect on implicit-net inference is NOT applied. The parser continues to assume default `wire`, so under `` `default_nettype none `` the parser's `W-PARSE-IMPLICIT-NET` warning may fire spuriously for code that legitimately declared `none`. Tracking `default_nettype` state into the parser is deferred.
10. **Reserved-but-unpromoted preprocess codes.** Appendix A reserves distinct codes `E1009 = E-PP-UNDEF-MACRO-USE` (undefined macro use), `E1010`, and `E1006` for preprocess cases that the MVP intentionally does NOT promote — their cases are folded into the `E-PP-BAD-DIRECTIVE` (E1013) catch-all to keep the promotion at exactly +5 (mirroring the §8 RECURSION / INCLUDE-CYCLE naming decisions). A later maintainer may promote them and bump the bijection count; the folding is deliberate, not an oversight.

---

## 11. Reference: full IEEE behavioral prose

The complete normative behavior (processing model, scanning rules, define/use/expansion, argument parsing, undef, redefinition, conditionals, include, pass-through, unknown directive, span mapping, out-of-scope, diagnostic summary) is reproduced verbatim from the IEEE semantics research finding and is the authority where this document's code comments are terser. Implementers MUST consult §1–§14 of that finding (mirrored as §0 here) for any edge case not spelled out in the Rust above; the Rust is the realization, the prose is the contract.
