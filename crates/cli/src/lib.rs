//! cli — the user-facing `vita` driver that wires the whole pipeline.
//!
//! Pipeline: read source(s) → (preprocess passthrough) → lex → parse →
//! elaborate → simulate → VCD. Diagnostics go to stderr through a concrete
//! [`StderrSink`] (the first real `diag::LogSink`); the numeric exit code follows
//! doc-13 §Exit codes:
//!
//! | code | meaning |
//! |------|---------|
//! | 0    | clean: parse+elaborate ok, simulation finished with no errors |
//! | 1    | user/design error: lex/parse errors, elaborate `None`, runtime `$fatal` |
//! | 3    | CLI/usage error: no source files, file not found, unknown applet |
//!
//! `main()` is a thin wrapper that parses argv, reads files, and calls
//! [`run_vita`]; the staged applets ([`run_vcmp`]/[`run_velab`]/[`run_vrun`])
//! serialize the front-end `SourceUnit` to a `.vu`, elaborate it to a `.velab`
//! (golden `SimIr` frame + non-golden `ForkModeTable` trailer), and simulate it,
//! with a `schema_hash` staleness gate between every stage.

use std::cell::{Cell, RefCell};
use std::io::Write;

use diag::{Diagnostic, LogEvent, LogSink, MsgCode, Severity, SourceLoc};
use sim_engine::{ExitClass, FinishReason, SimOpts};

/// Exit code for a clean run (doc-13 §Exit codes).
pub const EXIT_OK: i32 = 0;
/// Exit code for a user/design error (lex/parse/elab/runtime-fatal).
pub const EXIT_USER_ERROR: i32 = 1;
/// Exit code for a stale/artifact-gate rejection (doc-13 class 2): magic/
/// schema/format/version mismatches. Distinct from 1 so CI re-runs vcmp/velab
/// instead of debugging RTL.
pub const EXIT_STALE: i32 = 2;
/// Exit code for a CLI/usage error (no sources, file not found, unknown applet).
pub const EXIT_CLI_ERROR: i32 = 3;

mod filelist;
pub mod worklib;

/// Knobs the `vita` driver threads down into the pipeline. Kept tiny for v1 — the
/// full bucket-C flag surface (doc-13) lands with `vita-log`.
#[derive(Debug, Clone, Default)]
pub struct VitaOpts {
    /// Overrides the design's `$dumpfile` path (CLI `-o`). `None` ⇒ use `$dumpfile`.
    pub vcd_path_override: Option<String>,
    /// Worker-thread budget (P4-T1, CLI `--threads N`/`-j N`). `None` ⇒ auto:
    /// `VITA_THREADS` env if set, else `min(available_parallelism, 8)`. Output is
    /// byte-identical for every value (the P4 contract) — wall-clock only.
    pub threads: Option<u32>,
    /// Hard cap on advanced simulation time in ticks (CLI `--timeout N`, P2-9).
    /// Reaching it ends the run cleanly (Quiescent) — a CI killswitch for
    /// designs that never `$finish`. `None` ⇒ unbounded.
    pub time_limit: Option<u64>,
    /// Diagnostic suppress/promote policy (`-Wno-*` / `-Werror[=*]`, doc-13
    /// bucket C). Pure output-stream filtering — never hashed into artifacts.
    pub gate: vita_log::GatePolicy,
    /// `` `include `` search dirs (`-I <dir>` / `+incdir+a+b`), tried in order
    /// after the current file's directory.
    pub incdirs: Vec<String>,
    /// Predefined object-like macros (`-D NAME[=VAL]` / `+define+N=V+M`).
    /// Name-wise last-wins is applied by the PREPROCESSOR seed order.
    pub defines: Vec<(String, String)>,
    /// Output verbosity (`-q`=0 / default 1 / `-v`=2 / `-vv`=3). `None` ⇒ 1.
    /// Pure sink policy — never hashed into artifacts (doc-13 bucket C).
    pub verbosity: Option<u8>,
    /// `--log <file>` tee transcript path (`-` = stderr). `None` ⇒ no tee.
    pub log: Option<String>,
    /// `--log-append`: accumulate instead of the default overwrite.
    pub log_append: bool,
    /// `vrun --upstream <file.vu>` (v6 ⑤, RULE V): re-hash the live upstream
    /// artifact and refuse to run on a digest mismatch with the `.velab`'s
    /// recorded `composite_input_hash` (`E-ART-STALE-UPSTREAM`, exit class 2).
    /// `None` ⇒ no verification (the pre-worklib default).
    pub upstream: Option<String>,
    /// `vcmp --work` (P2-A): record the compiled CU into this work library —
    /// (logical name, directory). `None` ⇒ plain `-o` flow only.
    pub work: Option<(String, String)>,
    /// `--top <unit>` (P2-A): explicit elaborate roots (velab/lib mode).
    pub tops: Vec<String>,
    /// Runtime plusargs (v7, `+name[=value]`, leading '+' stripped, CLI
    /// order). Searched first-match by `$test/$value$plusargs`. Pure runtime
    /// input — never hashed into artifacts.
    pub plusargs: Vec<String>,
}

impl VitaOpts {
    fn sim_opts(&self) -> SimOpts {
        SimOpts {
            vcd_path_override: self.vcd_path_override.clone(),
            threads: resolve_threads(self.threads),
            time_limit: self.time_limit,
            plusargs: self.plusargs.clone(),
            ..SimOpts::default()
        }
    }
}

/// Build the preprocessor options a `VitaOpts` describes (`-I`/`-D` surface).
fn pre_opts_of(opts: &VitaOpts) -> hdl_preprocess::PreOpts {
    hdl_preprocess::PreOpts {
        incdirs: opts.incdirs.iter().map(std::path::PathBuf::from).collect(),
        cli_defines: opts.defines.clone(),
        ..hdl_preprocess::PreOpts::default()
    }
}

/// Split a `NAME[=VAL]` define token (empty VAL = definedness only).
fn split_define(tok: &str) -> (String, String) {
    match tok.split_once('=') {
        Some((n, v)) => (n.to_string(), v.to_string()),
        None => (tok.to_string(), String::new()),
    }
}

/// P4-T1 thread-count resolution: explicit flag > `VITA_THREADS` env > auto
/// (`min(available_parallelism, 8)`). Clamped to ≥1. The count never changes
/// output bytes — only wall-clock — so "auto" is safe as the default.
fn resolve_threads(flag: Option<u32>) -> u32 {
    flag.or_else(|| {
        std::env::var("VITA_THREADS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
    })
    .unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
            .min(8)
    })
    .max(1)
}

/// A minimal concrete `LogSink`: the first real sink in the workspace.
///
/// - `Diagnostic` → stderr as `<severity>[<CODE>]: <message>` (+ `file:line:col`
///   when a `location` is present).
/// - `Progress` / `RtlOutput` → stdout (the `$display` transcript + run summary),
///   suppressed on the TERMINAL at verbosity 0 (`-q`) — diagnostics never are.
/// - With a `--log` writer attached, EVERY event line is teed to that single
///   writer in emission order (doc-13 단일 writer tee: terminal copy and file
///   copy consume the SAME stream so they cannot drift; `-q` only affects the
///   terminal copy).
///
/// Severity counters are interior-mutable so the driver can decide the exit
/// code and print the doc-13 counts epilogue (the trait's `emit(&self)`
/// forbids `&mut`).
pub struct StderrSink {
    errors: Cell<u32>,
    fatals: Cell<u32>,
    warnings: Cell<u32>,
    notes: Cell<u32>,
    /// 0 = quiet (`-q`), 1 = default, 2 = verbose (`-v`), 3 = trace (`-vv`,
    /// currently rendering the same as 2 — reserved surface).
    verbosity: u8,
    log: Option<RefCell<Box<dyn Write>>>,
}

impl StderrSink {
    pub fn new() -> Self {
        Self::with_output(1, None)
    }

    /// Sink with an explicit verbosity and an optional `--log` tee writer.
    pub fn with_output(verbosity: u8, log: Option<Box<dyn Write>>) -> Self {
        StderrSink {
            errors: Cell::new(0),
            fatals: Cell::new(0),
            warnings: Cell::new(0),
            notes: Cell::new(0),
            verbosity,
            log: log.map(RefCell::new),
        }
    }

    /// Count of Error-severity diagnostics seen so far.
    pub fn error_count(&self) -> u32 {
        self.errors.get()
    }

    /// Count of Fatal-severity diagnostics seen so far.
    pub fn fatal_count(&self) -> u32 {
        self.fatals.get()
    }

    /// True if any Error or Fatal diagnostic was emitted.
    pub fn had_error_or_fatal(&self) -> bool {
        self.errors.get() > 0 || self.fatals.get() > 0
    }

    /// Verbose mode (`-v` and up)?
    pub fn verbose(&self) -> bool {
        self.verbosity >= 2
    }

    fn tee(&self, line: &str) {
        if let Some(w) = &self.log {
            let _ = w.borrow_mut().write_all(line.as_bytes());
        }
    }

    /// Broken-pipe-safe stdout write (§4.5.59 follow-on): `print!`/`println!`
    /// PANIC on EPIPE, and on macOS the worker thread can see EPIPE (its
    /// thread-directed SIGPIPE pends while masked) — the process still dies
    /// 141 via the pending signal, but the panic message polluted stderr
    /// first. `write_all` with the error dropped is the conventional
    /// producer behaviour: stop quietly. NOTE: this also swallows a non-pipe
    /// write failure (e.g. ENOSPC on a redirected stdout) — a truncated
    /// transcript instead of a loud panic, the usual Unix CLI convention.
    fn out_write(s: &str) {
        use std::io::Write as _;
        let _ = std::io::stdout().write_all(s.as_bytes());
    }

    /// Broken-pipe-safe stderr write (same rationale as [`Self::out_write`]).
    fn err_write(s: &str) {
        use std::io::Write as _;
        let _ = std::io::stderr().write_all(s.as_bytes());
    }

    /// doc-13 counts summary epilogue (`errors=E warnings=W notes=N`) — the
    /// unsuppressible end-of-stage spine. A `$fatal`/Fatal counts as an error
    /// here (the run definitely failed); `notes` = Info + Note.
    pub fn epilogue(&self) {
        let line = format!(
            "errors={} warnings={} notes={}",
            self.errors.get() + self.fatals.get(),
            self.warnings.get(),
            self.notes.get()
        );
        Self::err_write(&format!("{line}\n"));
        self.tee(&format!("{line}\n"));
    }

    fn render_diagnostic(&self, d: &Diagnostic) {
        match d.severity {
            Severity::Error => self.errors.set(self.errors.get() + 1),
            Severity::Fatal => self.fatals.set(self.fatals.get() + 1),
            Severity::Warning => self.warnings.set(self.warnings.get() + 1),
            _ => self.notes.set(self.notes.get() + 1),
        }
        let head = format!(
            "{}[{}]: {}",
            d.severity.token(),
            d.code.code_num(),
            d.message
        );
        let line = match &d.location {
            Some(loc) => format!("{}:{}:{}: {}", loc.file, loc.line, loc.col, head),
            None => head,
        };
        Self::err_write(&format!("{line}\n"));
        self.tee(&format!("{line}\n"));
    }
}

impl Default for StderrSink {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSink for StderrSink {
    fn emit(&self, event: LogEvent) {
        match event {
            LogEvent::Diagnostic(d) => self.render_diagnostic(&d),
            LogEvent::Progress(p) => {
                if self.verbosity >= 1 {
                    Self::out_write(&format!("{}\n", p.message));
                }
                self.tee(&format!("{}\n", p.message));
            }
            LogEvent::RtlOutput(t) => {
                if self.verbosity >= 1 {
                    Self::out_write(&t.text);
                }
                self.tee(&t.text);
            }
        }
    }
}

/// Open the `--log` tee writer a `VitaOpts` describes (`-` = stderr, vvp `-l -`
/// parity; default overwrite, `--log-append` accumulates). An unopenable path
/// is a loud CLI/usage error — never a silent no-log run.
fn open_log(opts: &VitaOpts) -> Result<Option<Box<dyn Write>>, i32> {
    let Some(path) = &opts.log else {
        return Ok(None);
    };
    if path == "-" {
        return Ok(Some(Box::new(std::io::stderr())));
    }
    let mut o = std::fs::OpenOptions::new();
    o.create(true).write(true);
    if opts.log_append {
        o.append(true);
    } else {
        o.truncate(true);
    }
    match o.open(path) {
        Ok(f) => Ok(Some(Box::new(std::io::LineWriter::new(f)))),
        Err(e) => {
            eprintln!(
                "error[{}]: cannot open log '{path}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
            Err(EXIT_CLI_ERROR)
        }
    }
}

/// `-v` effective-inputs echo (doc-13): the define/incdir sets the run will
/// actually use, as Progress events (⇒ terminal stdout + `--log` tee).
fn echo_effective_inputs(sink: &dyn LogSink, opts: &VitaOpts) {
    if !opts.defines.is_empty() {
        let s = opts
            .defines
            .iter()
            .map(|(n, v)| {
                if v.is_empty() {
                    n.clone()
                } else {
                    format!("{n}={v}")
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        sink.emit(LogEvent::Progress(diag::ProgressEvent {
            message: format!("defines: {s}"),
        }));
    }
    if !opts.incdirs.is_empty() {
        sink.emit(LogEvent::Progress(diag::ProgressEvent {
            message: format!("incdirs: {}", opts.incdirs.join(" ")),
        }));
    }
}

/// Map a byte offset into `src` to a 1-based `(line, col)`. Columns count
/// Unicode scalar values from the last newline (good enough for v1 caret-less
/// reporting; the real side-table bridge lives in `vita-log`).
///
/// Retained per preprocess-spec §4.3: line/col resolution now flows through
/// `hdl_preprocess::SourceMap` (which carries a byte-identical copy of this
/// function), so this is currently unreferenced. It is kept as the reference
/// the SourceMap copy must agree with byte-for-byte.
#[allow(dead_code)]
fn byte_to_line_col(src: &str, byte: usize) -> (u32, u32) {
    // Clamp out-of-range, then floor to a UTF-8 char boundary so the
    // `src[line_start..byte]` slice cannot split a multibyte scalar. Mirrors
    // `hdl_preprocess::byte_to_line_col` byte-for-byte.
    let mut byte = byte.min(src.len());
    while byte > 0 && !src.is_char_boundary(byte) {
        byte -= 1;
    }
    let mut line = 1u32;
    let mut line_start = 0usize;
    for (i, c) in src.char_indices() {
        if i >= byte {
            break;
        }
        if c == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = src[line_start..byte].chars().count() as u32 + 1;
    (line, col)
}

/// Build a `SourceLoc` for the half-open expanded-byte range `[lo, hi)` by
/// resolving it through the preprocessor's `SourceMap` back to original positions.
fn loc_from_span(map: &hdl_preprocess::SourceMap, lo: usize, hi: usize) -> SourceLoc {
    map.resolve_span(lo, hi)
}

/// Emit a front-end (lex/parse) diagnostic with a resolved location.
fn emit_frontend_error(
    sink: &dyn LogSink,
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

/// Read a single source file, then run the preprocess→lex→parse front-end,
/// emitting any diagnostics through `sink`. Returns `Some(unit)` on a clean
/// parse, `None` if read / preprocess / lex / parse failed OR the parse produced
/// no design units (the caller maps `None` to `EXIT_USER_ERROR`; a single-file
/// read failure also returns `None` after no emit — callers that need exit-3 on a
/// missing file read it themselves first).
///
/// The full pipeline (incl. the preprocessor) runs even for directive-free input,
/// so byte offsets / spans match the production one-shot path exactly. The staged
/// `vcmp` path and the round-trip tests parse through this same function so the
/// comparison never silently depends on a preprocessor bypass.
pub fn frontend_to_unit(file: &str, sink: &dyn LogSink) -> Option<hdl_ast::SourceUnit> {
    let text = std::fs::read_to_string(file).ok()?;
    let text = if text.ends_with('\n') {
        text
    } else {
        format!("{text}\n")
    };
    // `vcmp` serializes only the SourceUnit to `.vu`; timescale resolution happens at
    // `velab` (one-shot) time where it can ride into the SimIr-bearing `.velab`.
    frontend_text_to_unit(file, &text, sink).map(|(u, _)| u)
}

/// The preprocess→lex→parse core, factored so the one-shot driver, multi-file
/// `vcmp` (which concatenates first), and single-file [`frontend_to_unit`] all
/// share one implementation. Returns `None` (after emitting) on any front-end
/// error or an empty unit. `file` is the display name used in diagnostics; `text`
/// is the already-read source buffer.
pub fn frontend_text_to_unit(
    file: &str,
    text: &str,
    sink: &dyn LogSink,
) -> Option<(hdl_ast::SourceUnit, hdl_preprocess::ResolvedTimescales)> {
    frontend_text_to_unit_pre(file, text, sink, &hdl_preprocess::PreOpts::default())
}

/// (parsed unit, resolved timescales, include closure as (path, raw digest)).
pub type FrontendUnit = (
    hdl_ast::SourceUnit,
    hdl_preprocess::ResolvedTimescales,
    Vec<(String, [u8; 32])>,
);

/// [`frontend_text_to_unit`] with an explicit preprocessor surface (`-I`/`-D`).
pub fn frontend_text_to_unit_pre(
    file: &str,
    text: &str,
    sink: &dyn LogSink,
    pre_opts: &hdl_preprocess::PreOpts,
) -> Option<(hdl_ast::SourceUnit, hdl_preprocess::ResolvedTimescales)> {
    frontend_text_to_unit_pre_with_includes(file, text, sink, pre_opts).map(|(u, rt, _)| (u, rt))
}

/// [`frontend_text_to_unit_pre`] that ALSO returns the `\`include` closure —
/// every on-disk file the preprocessor opened, as (canonical path, raw bytes
/// digest) pairs. The worklib manifest records these so a header edit without
/// recompiling trips the RULE-V gate (the entry file itself is excluded: its
/// digest is taken per-source by the caller).
pub fn frontend_text_to_unit_pre_with_includes(
    file: &str,
    text: &str,
    sink: &dyn LogSink,
    pre_opts: &hdl_preprocess::PreOpts,
) -> Option<FrontendUnit> {
    // ── preprocess ─────────────────────────────────────────────────────────
    // raw source -> expanded text + SourceMap. The expanded text (not `text`) is
    // what the lexer and parser consume; spans they produce index the expanded
    // buffer and resolve back to original files via `pp.map`.
    let base_dir = std::path::Path::new(file)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pp = hdl_preprocess::preprocess_str(base_dir, file, text, pre_opts);
    for d in &pp.diags {
        let loc = pp.map.resolve_span(d.at, d.at);
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
        return None;
    }
    let expanded: &str = &pp.text;

    // ── lex ──────────────────────────────────────────────────────────────
    let (tokens, lex_errors) = hdl_lexer::lex(expanded);
    if !lex_errors.is_empty() {
        for e in &lex_errors {
            let (mnemonic, _) = e.kind.msg_code_hint();
            let msg = format!("lex error: {} ({mnemonic})", lex_error_message(e.kind));
            emit_frontend_error(sink, &pp.map, e.span.start, e.span.end, msg);
        }
        return None;
    }

    // ── parse ─────────────────────────────────────────────────────────────
    let (unit, parse_errors) = hdl_parser::parse(&tokens, expanded);
    if !parse_errors.is_empty() {
        for e in &parse_errors {
            let found = match e.found {
                Some(k) => format!("{k:?}"),
                None => "end of file".to_string(),
            };
            let msg = format!("expected {}, found {found}", e.expected);
            emit_frontend_error(sink, &pp.map, e.span.lo as usize, e.span.hi as usize, msg);
        }
        return None;
    }
    let Some(unit) = unit else {
        // Empty source with no errors: nothing to simulate. Treat as a usage
        // error — the user pointed the tool at a file with no design units.
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Error,
            code: MsgCode::ParseUnexpectedToken,
            message: "no design units found in source".to_string(),
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
        return None;
    };
    // Resolve each module's `timescale by file order (region offsets and module spans
    // share the expanded-text coordinate space). The result rides into elaborate.
    let modules: Vec<(&str, usize)> = unit
        .items
        .iter()
        .filter_map(|it| match it {
            hdl_ast::TopItem::Module(m) => Some((m.name.name.as_str(), m.span.lo as usize)),
            _ => None,
        })
        .collect();
    let rt = hdl_preprocess::resolve_module_timescales(&modules, &pp.timescales);
    drop(modules); // release the borrow of `unit` before moving it
                   // doc-08: a design with NO `timescale at all gets the 1ns/1ns base + one warning.
    if pp.timescales.is_empty() {
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::PpTimescaleDefault,
            message: "no `timescale in the design; assuming the 1ns/1ns base".to_string(),
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }
    let includes: Vec<(String, [u8; 32])> = pp
        .map
        .files
        .iter()
        .filter_map(|f| {
            f.canon.as_ref().map(|c| {
                (
                    c.to_string_lossy().into_owned(),
                    *blake3::hash(f.text.as_bytes()).as_bytes(),
                )
            })
        })
        .collect();
    Some((unit, rt, includes))
}

/// Render a base-10 second exponent as a `` `timescale ``-style unit string
/// (`-9` → `1ns`, `-10` → `100ps`, `-8` → `10ns`) for the VCD `$timescale` preamble.
/// VCD admits only 1|10|100 × s..fs, i.e. exp ∈ [-15, +2]; the preprocessor only
/// produces that range, but out-of-range exponents saturate to the nearest
/// representable unit rather than misrendering (old fallback: -16 → "100s").
pub fn timescale_unit_string(exp: i8) -> String {
    let exp = (exp as i32).clamp(-15, 2);
    let unit_exp = exp.div_euclid(3) * 3; // floor to a multiple of 3
    let mantissa = 10i32.pow((exp - unit_exp) as u32);
    let unit = match unit_exp {
        0 => "s",
        -3 => "ms",
        -6 => "us",
        -9 => "ns",
        -12 => "ps",
        _ => "fs", // -15 (the clamp admits nothing lower)
    };
    format!("{mantissa}{unit}")
}

/// Core: run the `vita` one-shot pipeline over already-read source `text`
/// (`file` is the display name used in diagnostics). Returns the process exit
/// code. This is the unit-test entry point — it never reads argv or files and
/// never calls `std::process::exit`.
pub fn run_vita_str(file: &str, text: &str, opts: &VitaOpts) -> i32 {
    let log = match open_log(opts) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
    let sink = vita_log::GatedSink::new(&inner, opts.gate.clone());
    if inner.verbose() {
        echo_effective_inputs(&sink, opts);
    }
    let code = run_vita_str_gated(file, text, opts, &inner, &sink);
    // doc-13: the counts summary epilogue is the unsuppressible end-of-stage
    // spine — printed on EVERY pipeline run (not on --help/--version/usage).
    inner.epilogue();
    code
}

fn run_vita_str_gated(
    file: &str,
    text: &str,
    opts: &VitaOpts,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    // ── preprocess → lex → parse (shared front-end) ─────────────────────────
    let Some((unit, rt)) = frontend_text_to_unit_pre(file, text, sink, &pre_opts_of(opts)) else {
        return EXIT_USER_ERROR;
    };

    // ── elaborate ──────────────────────────────────────────────────────────
    // The elaborator emits its own diagnostics through `sink`; `None` ⇒ a hard
    // elaboration error was reported. `elaborate_with_timescale` also yields the
    // fork-join, net-name, and per-process time-multiplier side tables threaded into
    // `SimOpts`; the timescale env scales `#delay`/`$time`/`$realtime`.
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&unit, sink, &rt.unit_exp, rt.global_prec_exp);
    let Some(ir) = ir else {
        return EXIT_USER_ERROR;
    };

    // ── simulate ────────────────────────────────────────────────────────────
    let sim_opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        // §21.3.2 %t/$timeformat: the call-site table + the precision exponent
        // %t scales against (one-shot path; empty/−9 ⇒ byte-identical).
        timeformat_stmts: sc.timeformat_stmts,
        handle_copy_stmts: sc.handle_copy_stmts,
        queue_slice_stmts: sc.queue_slice_stmts,
        global_prec_exp: rt.global_prec_exp,
        radixes: sc.radixes,
        assign_ranks: sc.assign_ranks,
        queue_bounds: sc.queue_bounds,
        proc_scopes: sc.proc_scopes,
        net_dims: sc.net_dims,
        final_procs: sc.final_procs,
        // N4 clocking: thread the preponed-sampler sidecars (one-shot path; empty
        // for designs with no clocking block → byte-identical).
        clocking_inputs: sc.clocking_inputs,
        clocking_commit: sc.clocking_commit,
        clocking_outputs: sc.clocking_outputs,
        // S1 gate/assign rise·fall·turnoff delay (one-shot path; empty unless a
        // delay has differing rise/fall/turnoff → byte-identical otherwise).
        ca_delays: sc.ca_delays,
        defer_marks: sc.defer_marks,
        defer_acts: sc.defer_acts,
        // B1/B2 frame-call: thread the func/task sidecars on the one-shot path
        // (empty for designs with no automatic/recursive func/task → byte-identical).
        func_table: sc.func_table,
        task_calls_proc: sc.task_calls_proc,
        task_calls_func: sc.task_calls_func,
        // SVPART: 2-state nets coerce X/Z→0 on write (one-shot path only).
        two_state_nets: sc.two_state_nets,
        wired_and_nets: sc.wired_and_nets,
        wired_or_nets: sc.wired_or_nets,
        // N7 class/OOP sidecars (one-shot path only).
        class_handle_nets: sc.class_handle_nets,
        class_new_sites: sc.class_new_sites,
        class_layouts: sc.class_layouts,
        class_field_inits: sc.class_field_inits,
        class_rand: sc.class_rand,
        class_constraints: sc.class_constraints,
        class_dist: sc.class_dist,
        class_randc: sc.class_randc,
        randomize_with: sc.randomize_with,
        class_vtable: sc.class_vtable,
        class_calls: sc.class_calls,
        class_field_widths: sc.class_field_widths,
        assert_fire: sc.assert_fire,
        assert_ctl: sc.assert_ctl,
        timescale_unit: timescale_unit_string(rt.global_prec_exp),
        ..opts.sim_opts()
    };
    let result = sim_engine::simulate(&ir, sink, sim_opts);
    let code = sim_exit_code(&result);
    // A `-Werror`-promoted warning is a real Error in the post-gate stream:
    // doc-13 class 1 ("승격-warning 실패") — flip an otherwise-clean exit.
    if code == EXIT_OK && inner.had_error_or_fatal() {
        return EXIT_USER_ERROR;
    }
    code
}

/// Map a finished `SimResult` to the doc-13 exit code. A clean `$finish`/quiescent
/// run with no error-or-fatal diagnostics is 0; anything else (`$fatal`, runtime
/// `$error`, delta-limit blowup) is a user/design error (1).
fn sim_exit_code(result: &sim_engine::SimResult) -> i32 {
    let clean_reason = matches!(
        result.finish_reason,
        FinishReason::Finish | FinishReason::Quiescent | FinishReason::Stop
    );
    match result.exit_class {
        ExitClass::Ok if clean_reason => EXIT_OK,
        _ => EXIT_USER_ERROR,
    }
}

/// Short human message for a lexer failure reason.
fn lex_error_message(kind: hdl_lexer::LexErrorKind) -> &'static str {
    use hdl_lexer::LexErrorKind as K;
    match kind {
        K::UnexpectedChar => "unexpected character",
        K::UnterminatedString => "unterminated string literal",
        K::UnterminatedBlockComment => "unterminated block comment",
        K::EmptyEscapedIdent => "empty escaped identifier",
        K::LoneSigil => "stray `$` or backtick with no identifier body",
    }
}

/// Run `vita` over one or more source files: read + concatenate (preprocess is a
/// passthrough), then drive the pipeline. Returns the process exit code.
///
/// File-read failures are CLI/usage errors (exit 3). With multiple files the
/// concatenated text uses the FIRST file's name in diagnostics (v1 — the §7
/// file_id→path bridge that disambiguates spans across files lands with vita-log).
pub fn run_vita(sources: &[String], opts: &VitaOpts) -> i32 {
    if sources.is_empty() {
        eprintln!(
            "error[{}]: no source files given",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    let mut text = String::new();
    for path in sources {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                text.push_str(&s);
                // separate files with a newline so a missing trailing newline in
                // one file can't fuse tokens across the boundary.
                if !s.ends_with('\n') {
                    text.push('\n');
                }
            }
            Err(e) => {
                eprintln!(
                    "error[{}]: cannot read '{path}': {e}",
                    MsgCode::FlistNotFound.code_num()
                );
                return EXIT_CLI_ERROR;
            }
        }
    }
    let display_name = sources[0].as_str();
    run_vita_str(display_name, &text, opts)
}

/// Which multicall applet was requested (by `argv[0]` basename, or `vita <sub>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applet {
    /// The one-shot driver (implemented).
    Vita,
    /// A staged-flow applet (`vcmp`/`velab`/`vrun`).
    Staged(&'static str),
}

/// Resolve the applet from `argv` (basename of `argv[0]`, then an optional
/// `vcmp`/`velab`/`vrun` subcommand for the `vita <applet>` explicit form).
/// Returns `(applet, remaining_args)` where `remaining_args` drops a consumed
/// subcommand token.
pub fn resolve_applet(argv: &[String]) -> (Applet, Vec<String>) {
    let base = argv
        .first()
        .map(std::path::Path::new)
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("vita");
    let rest = &argv[argv.len().min(1)..];
    match base {
        "vcmp" => (Applet::Staged("vcmp"), rest.to_vec()),
        "velab" => (Applet::Staged("velab"), rest.to_vec()),
        "vrun" => (Applet::Staged("vrun"), rest.to_vec()),
        _ => {
            // `vita` (or any other basename): allow an explicit `vita vcmp …` form.
            if let Some(sub) = rest.first().map(|s| s.as_str()) {
                if matches!(sub, "vcmp" | "velab" | "vrun") {
                    let staged: &'static str = match sub {
                        "vcmp" => "vcmp",
                        "velab" => "velab",
                        _ => "vrun",
                    };
                    return (Applet::Staged(staged), rest[1..].to_vec());
                }
            }
            (Applet::Vita, rest.to_vec())
        }
    }
}

/// The full multicall entry: dispatch on `argv[0]` basename / explicit subcommand,
/// then either run the one-shot pipeline or print the staged-flow stub. Returns
/// the process exit code. `main()` is a thin wrapper around this.
pub fn run(argv: &[String]) -> i32 {
    let (applet, args) = resolve_applet(argv);
    // P2-4: `--help`/`--version` anywhere in the args short-circuits (before this,
    // `vita --help` tried to READ a file named `--help`). Applet-specific usage.
    let applet_name = match applet {
        Applet::Vita => "vita",
        Applet::Staged(s) => s,
    };
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help(applet_name);
        return EXIT_OK;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{applet_name} {}", env!("CARGO_PKG_VERSION"));
        return EXIT_OK;
    }
    // Filelist expansion (doc-14 §3.1) happens at the ARGV level, before any
    // per-applet flag parsing — every applet accepts `-f`/`-F` uniformly and
    // a `.f` may carry any flag legal on the command line.
    let args = match filelist::expand_argv(&args, &StderrSink::new()) {
        Ok(a) => a,
        Err(code) => return code,
    };
    match applet {
        Applet::Vita => {
            // `vita explain <CODE>` — doc-15 catalog lookup (no pipeline).
            if args.first().map(String::as_str) == Some("explain") {
                return run_explain(&args[1..]);
            }
            // One-shot flag surface: `-o <vcd>` + `--threads N` (P4-T1), then
            // positional sources. (Before T1 the one-shot accepted NO flags —
            // `-o` was read as a source file.)
            let io = match parse_io_args(&args) {
                Ok(x) => x,
                Err(c) => return c,
            };
            if io.dump_filelist {
                return run_dump_filelist(&io);
            }
            if let Err(c) = reject_worklib_flags("vita", &io, false, false) {
                return c;
            }
            let opts = VitaOpts {
                vcd_path_override: io.out,
                threads: io.threads,
                time_limit: io.timeout,
                gate: io.gate,
                incdirs: io.incdirs,
                defines: io.defines,
                verbosity: io.verbosity,
                log: io.log,
                log_append: io.log_append,
                upstream: None, // one-shot has no staged upstream
                work: None,
                tops: Vec::new(),
                plusargs: io.plusargs,
            };
            run_vita(&io.pos, &opts)
        }
        Applet::Staged("vcmp") => dispatch_vcmp(&args),
        Applet::Staged("velab") => dispatch_velab(&args),
        Applet::Staged("vrun") => dispatch_vrun(&args),
        Applet::Staged(other) => {
            eprintln!(
                "error[{}]: unknown staged applet '{other}'",
                MsgCode::CliBadFlag.code_num()
            );
            EXIT_CLI_ERROR
        }
    }
}

/// The doc-15 catalog, embedded at compile time (cargo-only — no build.rs).
/// doc-15 is the single authority for cause/example/fix text; the bijection
/// test guarantees every `MsgCode` has a full entry in it.
const ERROR_CATALOG: &str = include_str!("../../../docs/preview/15-error-code-reference.md");

/// `vita explain <CODE>`: print the doc-15 entry for a mnemonic
/// (`E-ELAB-MULTIDRIVER`) or grep-number (`VITA-E3001`) form.
fn run_explain(args: &[String]) -> i32 {
    let Some(query) = args.first() else {
        eprintln!(
            "error[{}]: 'explain' needs a diagnostic code (mnemonic or VITA-####)",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    };
    let Some(code) = MsgCode::ALL
        .iter()
        .copied()
        .find(|c| c.mnemonic() == query || c.code_num() == query)
    else {
        eprintln!(
            "error[{}]: unknown diagnostic code '{query}'",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    };
    let header = format!("### {} ·", code.code_num());
    if let Some(start) = ERROR_CATALOG.find(&header) {
        let body = &ERROR_CATALOG[start..];
        // The entry runs to the next section header or horizontal rule.
        let next_hdr = body[4..].find("\n### ").map(|p| p + 4);
        let next_hr = body.find("\n---");
        let end = match (next_hdr, next_hr) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => body.len(),
        };
        println!("{}", body[..end].trim_end());
    } else {
        // Defensive: enum-registered but no full entry (the bijection gate
        // makes this unreachable; print the enum metadata rather than nothing).
        println!(
            "{} · `{}` ({:?})\n{}",
            code.code_num(),
            code.mnemonic(),
            code.default_severity(),
            code.title()
        );
    }
    EXIT_OK
}

/// P2-4: applet-specific usage text (doc-13 exit table: help/version are clean
/// exits). Kept truthful to the IMPLEMENTED surface (`-o` only).
fn print_help(applet: &str) {
    let body = match applet {
        "vcmp" => {
            "Usage: vcmp [-o <out.vu>] [--work <name[=dir]>] <sources>...\n\n\
             Compile sources (preprocess + lex + parse) into a `.vu` snapshot.\n\
             With --work, record the unit(s) into a work library (lib.toml +\n\
             content-addressed blob) instead of / in addition to `-o`."
        }
        "velab" => {
            "Usage: velab [-o <out.velab>] <in.vu> [--top <unit>]\n\
             \x20      velab -L <name[=dir]>... --top <unit>... [-o <out.velab>]\n\n\
             Elaborate a `.vu` snapshot into a `.velab` (golden SimIr + side tables).\n\
             Library mode (-L) resolves --top units by logical name (first -L wins)\n\
             and elaborates their instantiation closure."
        }
        "vrun" => {
            "Usage: vrun [-o <out.vcd>] <in.velab>\n\n\
             Simulate a `.velab`, writing the VCD and RTL stdout."
        }
        _ => {
            "Usage: vita [-o <out.vcd>] <sources>...\n\
             \x20      vita {vcmp|velab|vrun} [OPTIONS] ...\n\n\
             One-shot RTL simulation: preprocess -> lex -> parse -> elaborate ->\n\
             simulate -> VCD. The staged subcommands split the same pipeline.\n\
             `vita explain <CODE>` prints the doc-15 entry for a diagnostic."
        }
    };
    println!(
        "{applet} {} - vitamin RTL simulator",
        env!("CARGO_PKG_VERSION")
    );
    println!("{body}");
    println!(
        "\nOptions:\n  -o, --out <PATH>      output path override\n  \
         -f <FILE>             expand a filelist (paths relative to the CWD)\n  \
         -D, --define <N[=V]>  predefine a text macro (+define+N=V+M also accepted)\n  \
         -I, --incdir <DIR>    `include search dir (+incdir+a+b also accepted)\n  \
         -F <FILE>             expand a filelist (paths relative to the file's dir)\n  \
         --dump-filelist       print the effective post-expansion inputs and exit\n  \
         --threads, -j <N>     worker threads (output byte-identical for any N)\n  \
         --timeout <TICKS>     stop cleanly after TICKS sim time (CI killswitch)\n  \
         --upstream <FILE>     (vrun) verify the .velab's recorded upstream digest\n  \
         --work <NAME[=DIR]>   (vcmp) record units into a work library (default dir ./NAME)\n  \
         --workdir <DIR>       (vcmp) work-library directory when --work has no =dir\n  \
         -L <NAME[=DIR]>       (velab) bind a compiled library; search order = -L order\n  \
         --top <UNIT>          (velab) explicit elaborate root(s); required with -L\n  \
         -Wno-<CODE>           suppress a Warning/Info diagnostic (mnemonic, doc-15)\n  \
         -Werror[=<CODE>]      promote warnings (all, or one code) to errors\n  \
         -q, --quiet           silence terminal $display/progress (diags + --log keep all)\n  \
         -v / -vv              verbose: echo effective files/defines/incdirs (-vv reserved)\n  \
         --verbosity <0..3>    numeric form of -q/-v/-vv\n  \
         -l, --log <FILE>      tee the full transcript (RTL+diags+progress) to FILE ('-'=stderr)\n  \
         --log-append          accumulate into --log instead of overwriting\n  \
         -h, --help            print help\n  -V, --version         print version"
    );
}

/// P2-7: atomic artifact write — stage into `<out>.tmp.<pid>` then rename, so a
/// crash mid-write can never leave a partial `.vu`/`.velab` that the staleness
/// gate would misreport as a format mismatch. Same-directory rename is atomic on
/// POSIX and best-effort-replace on Windows.
fn write_artifact_atomic(out: &str, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = format!("{out}.tmp.{}", std::process::id());
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, out).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

// ───────────────────────── staged-flow applets ──────────────────────────────

/// Render an artifact-gate rejection through the sink as an Error diagnostic
/// (no source location — artifact-level), then return `EXIT_STALE` (doc-13
/// class 2: "rebuild upstream", distinct from class-1 design errors and
/// class-3 usage errors).
fn emit_artifact_error(sink: &dyn LogSink, e: &vita_artifact::ArtifactError) -> i32 {
    sink.emit(LogEvent::Diagnostic(Diagnostic {
        severity: Severity::Error,
        code: e.code,
        message: e.message.clone(),
        location: None,
        context: Vec::new(),
        sim_time: None,
    }));
    EXIT_STALE
}

/// Read a file as bytes; a read failure is a CLI/usage error (exit 3).
fn read_artifact_bytes(path: &str) -> Result<Vec<u8>, i32> {
    std::fs::read(path).map_err(|e| {
        eprintln!(
            "error[{}]: cannot read '{path}': {e}",
            MsgCode::FlistNotFound.code_num()
        );
        EXIT_CLI_ERROR
    })
}

/// Default output path: replace **only the final** extension component on the
/// input (std `Path::with_extension` semantics — never panics, replaces the last
/// `.ext` only). e.g. `default_out("a.sv","vu") -> "a.vu"`;
/// `default_out("a.b.sv","vu") -> "a.b.vu"`. Callers MUST run `out` through
/// `reject_out_clobbers_input` before writing.
fn default_out(input: &str, ext: &str) -> String {
    let p = std::path::Path::new(input);
    p.with_extension(ext).to_string_lossy().into_owned()
}

/// True iff two path strings denote the same file. Canonicalizes when BOTH paths
/// already exist (handles `./a.sv` vs `a.sv`, symlinks, `..`); otherwise falls
/// back to a raw string compare (the output usually does not exist yet). Never
/// panics.
fn same_path(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Reject when the resolved output path would overwrite any positional input.
/// Guards both the `default_out` self-clobber (`vcmp foo.vu` -> default `foo.vu`)
/// and an explicit `-o a.sv` that names an input.
fn reject_out_clobbers_input(inputs: &[String], out: &str) -> Result<(), i32> {
    if inputs.iter().any(|p| same_path(p, out)) {
        eprintln!(
            "error[{}]: output '{out}' would overwrite an input file",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

/// Build the `.vu`/`.velab` header. `global_time_precision` carries the resolved
/// design-wide precision exponent (real now that timescale is wired).
/// `composite` is the RULE-V upstream digest — blake3 of the stage's INPUT
/// (vcmp: the preprocessed source text; velab: the consumed `.vu` bytes) —
/// RECORDED since 2026-06-11 for provenance/forensics. The live re-hash gate
/// (`E-ART-STALE-UPSTREAM`) plus `consumed`/`worklib_manifest_hash` remain the
/// documented Phase-2 piece (they need a worklib for vrun to re-hash against);
/// `verify_header` already gates the primary staleness via `schema_hash` +
/// `format_version`.
fn artifact_header(
    schema_hash: [u8; 32],
    global_prec_exp: i8,
    composite: [u8; 32],
) -> vita_artifact::VelabHeader {
    vita_artifact::VelabHeader {
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        schema_hash,
        composite_input_hash: composite,
        global_time_precision: global_prec_exp as i64,
        consumed: Vec::new(),
        worklib_manifest_hash: [0u8; 32],
        uses_dump: false,
        tool_semver_major: env!("CARGO_PKG_VERSION_MAJOR")
            .parse()
            .expect("CARGO_PKG_VERSION_MAJOR is a valid u32"),
        provenance: vita_artifact::Provenance::capture(),
    }
}

/// `vcmp`: read+preprocess+lex+parse the source(s) into a `SourceUnit`, then write
/// a `.vu` artifact. `out` is the resolved output path.
/// Exit: 0 ok / 1 lex|parse|empty-unit / 3 missing-file|write-error.
pub fn run_vcmp(sources: &[String], out: Option<&str>, opts: &VitaOpts) -> i32 {
    if sources.is_empty() {
        eprintln!(
            "error[{}]: no source files given",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    let log = match open_log(opts) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
    let sink = vita_log::GatedSink::new(&inner, opts.gate.clone());
    if inner.verbose() {
        sink.emit(LogEvent::Progress(diag::ProgressEvent {
            message: format!("files: {}", sources.join(" ")),
        }));
        echo_effective_inputs(&sink, opts);
    }
    let code = run_vcmp_gated(sources, out, opts, &inner, &sink);
    inner.epilogue();
    code
}

fn run_vcmp_gated(
    sources: &[String],
    out: Option<&str>,
    opts: &VitaOpts,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    // read+concat (mirrors run_vita): read error → exit 3. Per-source raw
    // digests feed the worklib manifest (RULE-V staleness keys).
    let mut text = String::new();
    let mut src_digests: Vec<(String, [u8; 32])> = Vec::new();
    for path in sources {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                src_digests.push((path.clone(), *blake3::hash(s.as_bytes()).as_bytes()));
                text.push_str(&s);
                if !s.ends_with('\n') {
                    text.push('\n');
                }
            }
            Err(e) => {
                eprintln!(
                    "error[{}]: cannot read '{path}': {e}",
                    MsgCode::FlistNotFound.code_num()
                );
                return EXIT_CLI_ERROR;
            }
        }
    }
    let file = sources[0].as_str();

    // preprocess → lex → parse through the SAME shared front-end the one-shot uses.
    let Some((unit, rt, includes)) =
        frontend_text_to_unit_pre_with_includes(file, &text, sink, &pre_opts_of(opts))
    else {
        return EXIT_USER_ERROR;
    };

    // ── write `.vu` body = postcard(SourceUnit) ++ postcard((unit_exp map, global
    //    precision)). The resolved timescale rides after the hashed SourceUnit frame
    //    (the gate covers the type, not these bytes) so `velab` can elaborate the
    //    staged path with the same scaling as the one-shot path. ──
    // `-Werror`: a promoted warning is an Error — the stage fails and writes
    // NO artifact (matching a real compile error).
    if inner.had_error_or_fatal() {
        return EXIT_USER_ERROR;
    }
    let mut body = postcard::to_stdvec(&unit).expect("SourceUnit postcard encode infallible");
    body.extend_from_slice(
        &postcard::to_stdvec(&(rt.unit_exp, rt.global_prec_exp))
            .expect("timescale env postcard encode infallible"),
    );
    // RULE-V composite (recorded 2026-06-11): digest of this stage's INPUT —
    // the concatenated raw source plus the -D/-I surface in argv order (they
    // change preprocessing). `include`d FILE contents are not yet folded in
    // (that is the worklib `consumed[]` Phase-2 piece — documented limit).
    let composite = {
        let mut h = blake3::Hasher::new();
        h.update(text.as_bytes());
        for (n, v) in &opts.defines {
            h.update(n.as_bytes());
            h.update(b"=");
            h.update(v.as_bytes());
            h.update(b"\n");
        }
        for d in &opts.incdirs {
            h.update(d.as_bytes());
            h.update(b"\n");
        }
        *h.finalize().as_bytes()
    };
    let header = artifact_header(
        vita_schema::schema_hash::<hdl_ast::SourceUnit>(),
        rt.global_prec_exp,
        composite,
    );
    let bytes = vita_artifact::write_vu(&header, &body);
    if let Some(out) = out {
        if let Err(e) = write_artifact_atomic(out, &bytes) {
            eprintln!(
                "error[{}]: cannot write '{out}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
            return EXIT_CLI_ERROR;
        }
    }
    // `--work`: record the CU (blob + canonical manifest entry) into the
    // library. The blob bytes ARE the `.vu` bytes — the frozen artifact
    // format is reused verbatim, only the directory layout is new.
    if let Some((lib_name, dir)) = &opts.work {
        let units: Vec<(String, String)> = unit
            .items
            .iter()
            .filter_map(|it| match it {
                hdl_ast::TopItem::Module(m) => Some(("module".to_string(), m.name.name.clone())),
                hdl_ast::TopItem::Interface(m) => {
                    Some(("interface".to_string(), m.name.name.clone()))
                }
                // v7: packages are units too (importable from libraries).
                hdl_ast::TopItem::Package(m) => Some(("package".to_string(), m.name.name.clone())),
                // N7: a top-level class is a unit too (importable like a package).
                hdl_ast::TopItem::Class(c) => Some(("class".to_string(), c.name.name.clone())),
                hdl_ast::TopItem::Import(_) | hdl_ast::TopItem::Error(_) => None,
            })
            .collect();
        let cu = worklib::Cu {
            blob: String::new(), // content-addressed name assigned by add_cu
            defines: opts
                .defines
                .iter()
                .map(|(n, v)| {
                    if v.is_empty() {
                        n.clone()
                    } else {
                        format!("{n}={v}")
                    }
                })
                .collect(),
            incdirs: opts.incdirs.clone(),
            sources: src_digests,
            includes,
            units,
        };
        match worklib::add_cu(
            std::path::Path::new(dir),
            lib_name,
            &bytes,
            cu,
            &write_artifact_atomic,
        ) {
            Ok(worklib::AddOutcome::Ok) => {}
            Ok(worklib::AddOutcome::DupUnit(name)) => {
                sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Error,
                    code: MsgCode::DupUnit,
                    message: format!(
                        "design unit `{name}` is already defined in library `{lib_name}` \
                         by a different source — recompile that source, or rename"
                    ),
                    location: None,
                    context: Vec::new(),
                    sim_time: None,
                }));
                return EXIT_USER_ERROR;
            }
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError {
                        code: MsgCode::WorkManifestInvalid,
                        message: e,
                    },
                );
            }
        }
    }
    EXIT_OK
}

/// `velab`: read a `.vu`, gate the hdl-ast hash, decode the `SourceUnit`,
/// elaborate (with fork modes), then write a `.velab` = header(SimIr hash) +
/// body(`postcard(SimIr) ++ postcard(ForkModeTable)`).
/// Exit: 0 ok / 1 gate-reject|elab-fail|corrupt-body / 3 missing-file|write-error.
pub fn run_velab(vu_path: &str, out: &str, opts: &VitaOpts) -> i32 {
    let log = match open_log(opts) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
    let sink = vita_log::GatedSink::new(&inner, opts.gate.clone());
    if inner.verbose() {
        sink.emit(LogEvent::Progress(diag::ProgressEvent {
            message: format!("in: {vu_path}  out: {out}"),
        }));
    }
    let code = run_velab_gated(vu_path, out, opts, &inner, &sink);
    inner.epilogue();
    code
}

fn run_velab_gated(
    vu_path: &str,
    out: &str,
    opts: &VitaOpts,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    let bytes = match read_artifact_bytes(vu_path) {
        Ok(b) => b,
        Err(code) => return code,
    };
    // RULE-V composite (recorded 2026-06-11): the `.velab` carries the digest
    // of the exact `.vu` bytes it consumed — provenance now, the
    // E-ART-STALE-UPSTREAM re-hash gate when a worklib exists (Phase-2).
    let vu_composite = *blake3::hash(&bytes).as_bytes();

    let (unit, unit_exp, global_prec_exp) = match decode_vu_unit(&bytes, sink) {
        Ok(x) => x,
        Err(code) => return code,
    };

    // ── elaborate (with the staged timescale env; `--top` overrides roots) ──
    let roots: Option<&[String]> = if opts.tops.is_empty() {
        None
    } else {
        Some(&opts.tops)
    };
    let (ir, sc) =
        elaborate::elaborate_with_timescale_roots(&unit, sink, &unit_exp, global_prec_exp, roots);
    let Some(ir) = ir else {
        return EXIT_USER_ERROR; // elab error already emitted
    };

    // ── write `.velab` body = postcard(SimIr) ++ postcard(ForkModeTable) ++
    //    postcard(NetNameTable) ++ postcard((proc_multipliers, global_prec_exp)) ++
    //    postcard(SeverityTable). All trailers ride OUTSIDE the hashed SimIr frame
    //    (the schema gate covers the type, not these bytes), so the golden hash and
    //    staleness are unaffected; names give `vrun` hierarchical VCD, the multipliers
    //    give it `$time`/`$realtime` scaling, and the severities give it
    //    `$fatal`/`$error`/`$warning`/`$info` routing (P1-1). ──
    // `-Werror`: promoted warnings fail the stage before any artifact lands.
    if inner.had_error_or_fatal() {
        return EXIT_USER_ERROR;
    }
    write_velab_file(out, &ir, &sc, global_prec_exp, vu_composite, None)
}

/// Decode a `.vu`: header gate (magic/format/schema) + `SourceUnit` frame +
/// the tolerant timescale tail. Shared by the legacy positional path and the
/// worklib closure loader.
fn decode_vu_unit(
    bytes: &[u8],
    sink: &dyn LogSink,
) -> Result<
    (
        hdl_ast::SourceUnit,
        std::collections::BTreeMap<String, i8>,
        i8,
    ),
    i32,
> {
    let (header, body) = match vita_artifact::read_vu(bytes) {
        Ok(x) => x,
        Err(e) => return Err(emit_artifact_error(sink, &e)),
    };
    // staleness gate: this `.vu` must match the hdl-ast shape THIS velab was built against.
    let tool = vita_artifact::ToolContext::new(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    if let Err(e) = vita_artifact::verify_header(&header, &tool) {
        return Err(emit_artifact_error(sink, &e)); // E-ART-SCHEMA-MISMATCH etc.
    }
    // decode the SourceUnit frame, then the trailing timescale env (tolerant of an
    // older `.vu` with no env → the 1ns/1ns base).
    let (unit, vu_rest): (hdl_ast::SourceUnit, &[u8]) = match postcard::take_from_bytes(body) {
        Ok(x) => x,
        Err(e) => {
            return Err(emit_artifact_error(
                sink,
                &vita_artifact::ArtifactError::format(&format!("undecodable .vu body: {e}")),
            ))
        }
    };
    let (unit_exp, global_prec_exp): (std::collections::BTreeMap<String, i8>, i8) =
        if vu_rest.is_empty() {
            (std::collections::BTreeMap::new(), -9)
        } else {
            match postcard::from_bytes(vu_rest) {
                Ok(x) => x,
                Err(e) => {
                    return Err(emit_artifact_error(
                        sink,
                        &vita_artifact::ArtifactError::format(&format!(
                            "undecodable .vu timescale trailer: {e}"
                        )),
                    ))
                }
            }
        };
    Ok((unit, unit_exp, global_prec_exp))
}

/// 14th `.velab` trailer (2026-06-22 STAGED-DROP audit fix): the engine-facing
/// sidecars that were previously threaded ONLY through one-shot `vita` and
/// silently dropped on the staged `velab→vrun` path — N7 class/OOP, B-track
/// frame-call, 2-state nets, and assertion control. Bundling them in ONE named
/// struct makes the field order the single source of truth, so the encode/decode
/// coupling cannot skew (the trailer-coupling fragility the audit flagged). All
/// fields are out-of-band side tables, never the golden `SimIr` root → no
/// `format_version` bump; empty for plain RTL (≈13 length-zero bytes).
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct StagedExtraSidecars {
    func_table: sim_engine::FuncTable,
    task_calls_proc: sim_engine::TaskCallProc,
    task_calls_func: sim_engine::TaskCallFunc,
    two_state_nets: std::collections::BTreeSet<u32>,
    class_handle_nets: std::collections::BTreeSet<u32>,
    class_new_sites: std::collections::BTreeMap<u32, u32>,
    class_layouts: Vec<Vec<(u32, bool, bool)>>,
    class_field_inits: Vec<Vec<Option<sim_ir::BitPacked>>>,
    class_vtable: Vec<Vec<u32>>,
    class_calls: std::collections::BTreeMap<u32, (Option<u32>, u32)>,
    class_field_widths: std::collections::BTreeMap<u32, (u32, bool)>,
    assert_fire: std::collections::BTreeSet<u32>,
    assert_ctl: std::collections::BTreeMap<u32, u8>,
    /// N7-REST rand-field bounds (so staged velab→vrun doesn't drop randomize()).
    class_rand: Vec<Vec<sim_engine::RandBound>>,
    /// N7-REST B2 constraint predicates (staged velab→vrun must carry them too).
    class_constraints: Vec<Vec<Vec<sim_ir::COp>>>,
    class_dist: Vec<Vec<sim_engine::DistField>>,
    class_randc: Vec<Vec<sim_engine::RandcField>>,
    /// N7-REST B-CRV final: per-call inline `randomize() with {…}` constraints
    /// (staged velab→vrun must carry them or inline `with` is silently dropped).
    randomize_with: Vec<sim_engine::RandWithCall>,
    /// N4 clocking: preponed-sampler sidecars (staged velab→vrun must carry them or
    /// `cb.sig` is silently never sampled = stuck at X).
    clocking_inputs: std::collections::BTreeSet<u32>,
    clocking_commit: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// N4 clocking output pairs (staged sidecar — must survive vcmp→vrun).
    clocking_outputs: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// S1 gate/assign rise·fall·turnoff delay (staged sidecar — must survive
    /// vcmp→vrun or falling/turnoff transitions silently use the rise delay).
    /// EMPTY ⇒ no differing delays ⇒ byte-identical.
    ca_delays: std::collections::BTreeMap<u32, (u32, u32, u32)>,
    /// wand/wor wired-logic nets (staged sidecar — must survive vcmp→vrun or a
    /// multi-driven `wand`/`wor` net silently falls back to plain wire resolution
    /// = wrong value). Net IDs whose multi-driver resolution is wired-AND / -OR
    /// instead of wire. APPEND-ONLY (kept LAST so the wire stays an additive
    /// extension). EMPTY ⇒ no wand/wor nets ⇒ byte-identical.
    wired_and_nets: std::collections::BTreeSet<u32>,
    wired_or_nets: std::collections::BTreeSet<u32>,
    /// §21.3.2 `$timeformat` call-site StmtIds (staged sidecar — must survive
    /// vcmp→vrun or a staged `$timeformat` silently degrades to a bare-args
    /// `$display` print). APPEND-ONLY tail. EMPTY ⇒ byte-identical.
    timeformat_stmts: std::collections::BTreeSet<u32>,
    /// §7.10 whole-handle copy markers (staged sidecar — must survive
    /// vcmp→vrun or a staged `dst = src` silently prints an empty Display
    /// instead of copying). APPEND-ONLY tail. EMPTY ⇒ byte-identical.
    handle_copy_stmts: std::collections::BTreeMap<u32, (u32, u32)>,
    /// §7.10.1 queue-slice markers (staged sidecar — same drop hazard).
    /// APPEND-ONLY tail. EMPTY ⇒ byte-identical.
    queue_slice_stmts: std::collections::BTreeSet<u32>,
}

impl StagedExtraSidecars {
    /// Snapshot the extra sidecars from an elaboration result. Clones (one-time
    /// at artifact write, never a sim hot path) so the wire struct owns its data.
    fn from_sidecars(sc: &elaborate::Sidecars) -> Self {
        StagedExtraSidecars {
            func_table: sc.func_table.clone(),
            task_calls_proc: sc.task_calls_proc.clone(),
            task_calls_func: sc.task_calls_func.clone(),
            two_state_nets: sc.two_state_nets.clone(),
            class_handle_nets: sc.class_handle_nets.clone(),
            class_new_sites: sc.class_new_sites.clone(),
            class_layouts: sc.class_layouts.clone(),
            class_field_inits: sc.class_field_inits.clone(),
            class_vtable: sc.class_vtable.clone(),
            class_calls: sc.class_calls.clone(),
            class_field_widths: sc.class_field_widths.clone(),
            assert_fire: sc.assert_fire.clone(),
            assert_ctl: sc.assert_ctl.clone(),
            class_rand: sc.class_rand.clone(),
            class_constraints: sc.class_constraints.clone(),
            class_dist: sc.class_dist.clone(),
            class_randc: sc.class_randc.clone(),
            randomize_with: sc.randomize_with.clone(),
            clocking_inputs: sc.clocking_inputs.clone(),
            clocking_commit: sc.clocking_commit.clone(),
            clocking_outputs: sc.clocking_outputs.clone(),
            ca_delays: sc.ca_delays.clone(),
            wired_and_nets: sc.wired_and_nets.clone(),
            wired_or_nets: sc.wired_or_nets.clone(),
            timeformat_stmts: sc.timeformat_stmts.clone(),
            handle_copy_stmts: sc.handle_copy_stmts.clone(),
            queue_slice_stmts: sc.queue_slice_stmts.clone(),
        }
    }
}

/// One RULE-V fast-path stamp: (whole seconds since `UNIX_EPOCH`, sub-second
/// nanos, byte length) of a consumed file AT THE INSTANT velab verified its
/// content still hashed to the recorded digest. Decomposed `SystemTime` (no i64
/// epoch nanos) so it never overflows and stays exact across the FS round-trip.
type FileStamp = (u64, u32, u64);

/// 15th `.velab` trailer (RULEV-MTIME, 2026-06-23 ROADMAP §5 option A): per-entry
/// `(mtime, size)` fast-path stamps PARALLEL (same order, same length) to the 9th
/// `WorkConsumed` trailer's `libs`/`blobs`/`files` vecs.
///
/// velab records `Some(stamp)` for an entry ONLY after it has re-read the path and
/// CONFIRMED the bytes still hash to the recorded digest — so the stamped mtime is
/// tied to exactly that content. Capturing it at any looser point (e.g. blindly at
/// velab-write time) would reopen the very vcmp→velab staleness window that ruled
/// out the storage-free `source_mtime < velab_mtime` shortcut. `None` = "could not
/// verify, always rehash". At vrun, RULE-V stats the path: a matching `(mtime,size)`
/// trusts the recorded hash and skips the read+blake3; any mismatch (or absent
/// stamp, e.g. a legacy `.velab` with no 15th segment) falls back to the
/// authoritative rehash. Out-of-band side table → no `format_version` bump; ~3
/// length-zero bytes for explicit-path/legacy artifacts.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct WorkStamps {
    libs: Vec<Option<FileStamp>>,
    blobs: Vec<Option<FileStamp>>,
    files: Vec<Option<FileStamp>>,
}

/// Re-read `path` and, IFF its bytes hash to `want`, return its verified
/// `(mtime, size)` stamp. Any read/hash/metadata/time failure → `None` (the
/// entry will always be rehashed at vrun — never a silent skip).
fn stamp_verified(path: &std::path::Path, want: &[u8; 32]) -> Option<FileStamp> {
    let data = std::fs::read(path).ok()?;
    if blake3::hash(&data).as_bytes() != want {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    // Guard a write between the read and the stat: an inconsistent length means
    // the mtime no longer describes the bytes we hashed.
    if meta.len() != data.len() as u64 {
        return None;
    }
    let dur = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some((dur.as_secs(), dur.subsec_nanos(), meta.len()))
}

impl WorkStamps {
    /// Stamp every entry of `consumed`, verifying each path's content against its
    /// recorded digest first. One-time at velab write (never a sim hot path).
    fn from_consumed(consumed: &worklib::WorkConsumed) -> Self {
        WorkStamps {
            libs: consumed
                .libs
                .iter()
                .map(|(_n, dir, h)| stamp_verified(&std::path::Path::new(dir).join("lib.toml"), h))
                .collect(),
            blobs: consumed
                .blobs
                .iter()
                .map(|(p, h)| stamp_verified(std::path::Path::new(p), h))
                .collect(),
            files: consumed
                .files
                .iter()
                .map(|(p, h)| stamp_verified(std::path::Path::new(p), h))
                .collect(),
        }
    }
}

/// RULE-V freshness of one recorded `(path, hash)` with an optional fast-path
/// stamp. A matching live `(mtime, size)` trusts the recorded hash and skips the
/// read+blake3; anything else rehashes authoritatively.
enum Freshness {
    Fresh,
    Changed,
    Unreadable(std::io::Error),
}

fn check_fresh(path: &std::path::Path, want: &[u8; 32], stamp: Option<FileStamp>) -> Freshness {
    // Fast-path: velab proved this path's content hashed to `want` at the stamped
    // (mtime,size). If the live fingerprint still matches, trust it (the standard
    // make-style mtime assumption: content-change ⇒ fingerprint-change). A
    // sub-granularity or mtime-frozen rewrite is the documented residual hole.
    if let Some((secs, nanos, size)) = stamp {
        if let Ok(meta) = std::fs::metadata(path) {
            let live = meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok());
            if let Some(dur) = live {
                if meta.len() == size && dur.as_secs() == secs && dur.subsec_nanos() == nanos {
                    return Freshness::Fresh;
                }
            }
        }
        // stamp/stat mismatch → fall through to the authoritative rehash.
    }
    match std::fs::read(path) {
        Ok(b) if blake3::hash(&b).as_bytes() == want => Freshness::Fresh,
        Ok(_) => Freshness::Changed,
        Err(e) => Freshness::Unreadable(e),
    }
}

/// Serialize and atomically write a `.velab`: golden `SimIr` frame + the
/// append-only side-table trailers (+ the optional 9th WorkConsumed trailer —
/// legacy explicit-path builds write NOTHING extra, so their bytes are
/// unchanged by the worklib feature).
fn write_velab_file(
    out: &str,
    ir: &sim_ir::SimIr,
    sc: &elaborate::Sidecars,
    global_prec_exp: i8,
    composite: [u8; 32],
    consumed: Option<&worklib::WorkConsumed>,
) -> i32 {
    let mut velab_body = postcard::to_stdvec(ir).expect("SimIr postcard encode infallible");
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.fork_modes).expect("ForkModeTable postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.net_names).expect("NetNameTable postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&(&sc.proc_multipliers, global_prec_exp))
            .expect("timescale trailer postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.severities).expect("severity trailer postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.radixes).expect("radix trailer postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.proc_scopes).expect("scope trailer postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.assign_ranks)
            .expect("assign-rank trailer postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.queue_bounds)
            .expect("queue-bound trailer postcard encode infallible"),
    );
    // 9th segment: ALWAYS written since the net_dims trailer follows it —
    // a missing-when-legacy optional segment would make the 10th ambiguous.
    let wc_default = worklib::WorkConsumed::default();
    velab_body.extend_from_slice(
        &postcard::to_stdvec(consumed.unwrap_or(&wc_default))
            .expect("work-consumed trailer postcard encode infallible"),
    );
    // 10th segment: unpacked-array dims for per-element VCD naming (⑤).
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.net_dims).expect("net-dims trailer postcard encode infallible"),
    );
    // 11th segment: P2-E `final` ProcIds (BTreeSet — postcard-deterministic).
    // ALWAYS written (the deferred-assert trailers follow it) — same
    // disambiguation rule as the 9th segment.
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.final_procs)
            .expect("final-procs trailer postcard encode infallible"),
    );
    // 12th + 13th segments (§16.4 deferred immediate asserts): marker→region and
    // action→(marker, region). Empty by default (no deferred asserts).
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.defer_marks)
            .expect("defer-marks trailer postcard encode infallible"),
    );
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&sc.defer_acts)
            .expect("defer-acts trailer postcard encode infallible"),
    );
    // 14th segment (STAGED-DROP audit fix): class/frame-call/2-state/assert-ctl
    // sidecars, one append-only struct. Empty for plain RTL; out-of-band.
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&StagedExtraSidecars::from_sidecars(sc))
            .expect("extra-sidecars trailer postcard encode infallible"),
    );
    // 15th segment (RULEV-MTIME): per-entry (mtime,size) fast-path stamps parallel
    // to the 9th WorkConsumed trailer. velab verifies each path's content against
    // its recorded digest before stamping, so vrun can trust a matching stat and
    // skip the rehash. Empty (~3 bytes) for explicit-path .velab. Out-of-band.
    velab_body.extend_from_slice(
        &postcard::to_stdvec(&WorkStamps::from_consumed(consumed.unwrap_or(&wc_default)))
            .expect("work-stamps trailer postcard encode infallible"),
    );
    let vheader = artifact_header(
        vita_schema::schema_hash::<sim_ir::SimIr>(),
        global_prec_exp,
        composite,
    );
    let out_bytes = vita_artifact::write_velab(&vheader, &velab_body);
    if let Err(e) = write_artifact_atomic(out, &out_bytes) {
        eprintln!(
            "error[{}]: cannot write '{out}': {e}",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    EXIT_OK
}

/// `velab -L <lib> --top <unit>` (P2-A worklib): discover units by logical
/// name across the given libraries (first `-L` wins a name), load the
/// instantiation CLOSURE of the requested tops (a library's unrelated units
/// never become roots), elaborate with the explicit roots, and record the
/// consumed manifests/blobs/files into the 9th `.velab` trailer for the
/// `vrun` RULE-V auto-gate.
pub fn run_velab_lib(
    libs: &[(String, String)],
    tops: &[String],
    out: &str,
    opts: &VitaOpts,
) -> i32 {
    let log = match open_log(opts) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
    let sink = vita_log::GatedSink::new(&inner, opts.gate.clone());
    if inner.verbose() {
        sink.emit(LogEvent::Progress(diag::ProgressEvent {
            message: format!(
                "libs: {}  tops: {}  out: {out}",
                libs.iter()
                    .map(|(n, d)| format!("{n}={d}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                tops.join(" ")
            ),
        }));
    }
    let code = run_velab_lib_gated(libs, tops, out, &inner, &sink);
    inner.epilogue();
    code
}

fn run_velab_lib_gated(
    libs: &[(String, String)],
    tops: &[String],
    out: &str,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    // ── 1. load manifests (strict; E-WORK-MANIFEST on any failure) ──
    struct Lib {
        name: String,
        dir: std::path::PathBuf,
        manifest: worklib::Manifest,
        mhash: [u8; 32],
        dir_str: String,
    }
    let mut loaded_libs: Vec<Lib> = Vec::new();
    for (name, dir) in libs {
        let mpath = std::path::Path::new(dir).join("lib.toml");
        let text = match std::fs::read_to_string(&mpath) {
            Ok(t) => t,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError {
                        code: MsgCode::WorkManifestInvalid,
                        message: format!("{}: {e}", mpath.display()),
                    },
                )
            }
        };
        let mhash = *blake3::hash(text.as_bytes()).as_bytes();
        let manifest = match worklib::Manifest::parse(&text) {
            Ok(m) => m,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError {
                        code: MsgCode::WorkManifestInvalid,
                        message: format!("{}: {e}", mpath.display()),
                    },
                )
            }
        };
        if &manifest.name != name {
            return emit_artifact_error(
                sink,
                &vita_artifact::ArtifactError {
                    code: MsgCode::WorkManifestInvalid,
                    message: format!(
                        "{}: directory holds library `{}` (requested `{name}`)",
                        mpath.display(),
                        manifest.name
                    ),
                },
            );
        }
        loaded_libs.push(Lib {
            name: name.clone(),
            dir: std::path::PathBuf::from(dir),
            manifest,
            mhash,
            dir_str: dir.clone(),
        });
    }

    // ── 2. logical unit map — FIRST `-L` wins a duplicate name ──
    let mut unit_map: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    for (li, lib) in loaded_libs.iter().enumerate() {
        for (_, name, ci) in lib.manifest.unit_index() {
            unit_map.entry(name.to_string()).or_insert((li, ci));
        }
    }

    // ── 3. resolve tops, then walk the instantiation closure (BFS) ──
    let mut queue: std::collections::VecDeque<(usize, usize)> = std::collections::VecDeque::new();
    let mut seen_cu: std::collections::BTreeSet<(usize, usize)> = std::collections::BTreeSet::new();
    for t in tops {
        let Some(&key) = unit_map.get(t) else {
            sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Error,
                code: MsgCode::ElabUnsupported,
                message: format!("top unit `{t}` not found in the given libraries"),
                location: None,
                context: Vec::new(),
                sim_time: None,
            }));
            return EXIT_USER_ERROR;
        };
        if seen_cu.insert(key) {
            queue.push_back(key);
        }
    }
    struct LoadedCu {
        unit: hdl_ast::SourceUnit,
        unit_exp: std::collections::BTreeMap<String, i8>,
        prec: i8,
        blob_path: String,
        blob_hash: [u8; 32],
        lib_idx: usize,
        cu_idx: usize,
    }
    let mut loaded: Vec<LoadedCu> = Vec::new();
    let mut blob_bytes_all: Vec<u8> = Vec::new();
    while let Some((li, ci)) = queue.pop_front() {
        let lib = &loaded_libs[li];
        let blob_rel = &lib.manifest.cus[ci].blob;
        let blob_path = lib.dir.join(blob_rel);
        let bytes = match std::fs::read(&blob_path) {
            Ok(b) => b,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError {
                        code: MsgCode::WorkManifestInvalid,
                        message: format!(
                            "{}: {e} (library blob missing — re-run `vcmp --work`)",
                            blob_path.display()
                        ),
                    },
                )
            }
        };
        let blob_hash = *blake3::hash(&bytes).as_bytes();
        blob_bytes_all.extend_from_slice(&bytes);
        let (unit, unit_exp, prec) = match decode_vu_unit(&bytes, sink) {
            Ok(x) => x,
            Err(code) => return code,
        };
        // Enqueue the unit-map WINNER for every name this CU instantiates.
        // Resolution is by-name against the -L search order — never by
        // whatever definition happens to ride along in an already-loaded CU
        // (a passenger must not beat the search order). Deterministic:
        // BTreeSet walk over a BTreeMap lookup; `seen_cu` dedups.
        for name in elaborate::instantiated_names(&unit) {
            if let Some(&key) = unit_map.get(&name) {
                if seen_cu.insert(key) {
                    queue.push_back(key);
                }
            }
        }
        loaded.push(LoadedCu {
            unit,
            unit_exp,
            prec,
            blob_path: blob_path.to_string_lossy().into_owned(),
            blob_hash,
            lib_idx: li,
            cu_idx: ci,
        });
    }

    // ── 4. merge into ONE SourceUnit. A NAMED item is emitted only from the
    //       CU the unit map resolves its name to (first `-L` wins) — a
    //       shadowed passenger definition in another loaded CU is skipped
    //       regardless of load order. ──
    let mut merged = hdl_ast::SourceUnit {
        items: Vec::new(),
        span: hdl_ast::Span { lo: 0, hi: 0 },
    };
    let mut emitted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut merged_exp: std::collections::BTreeMap<String, i8> = std::collections::BTreeMap::new();
    let mut prec = i8::MAX;
    for cu in &loaded {
        prec = prec.min(cu.prec);
        for (k, v) in &cu.unit_exp {
            merged_exp.entry(k.clone()).or_insert(*v);
        }
        for it in &cu.unit.items {
            let name = match it {
                hdl_ast::TopItem::Module(m)
                | hdl_ast::TopItem::Interface(m)
                | hdl_ast::TopItem::Package(m) => Some(m.name.name.clone()),
                hdl_ast::TopItem::Class(c) => Some(c.name.name.clone()),
                hdl_ast::TopItem::Import(_) | hdl_ast::TopItem::Error(_) => None,
            };
            if let Some(n) = name {
                match unit_map.get(&n) {
                    // The search-order winner for this name is a DIFFERENT
                    // CU: this copy is shadowed.
                    Some(&key) if key != (cu.lib_idx, cu.cu_idx) => continue,
                    // Winner (or unmapped — a manifest that under-reports its
                    // units): first emission wins as a deterministic fallback.
                    _ => {
                        if !emitted.insert(n) {
                            continue;
                        }
                    }
                }
            }
            merged.items.push(it.clone());
        }
    }
    if prec == i8::MAX {
        prec = -9;
    }

    // ── 5. elaborate with the EXPLICIT roots ──
    let (ir, sc) =
        elaborate::elaborate_with_timescale_roots(&merged, sink, &merged_exp, prec, Some(tops));
    let Some(ir) = ir else {
        return EXIT_USER_ERROR;
    };
    if inner.had_error_or_fatal() {
        return EXIT_USER_ERROR;
    }

    // ── 6. record the consumed upstream for the vrun auto-gate ──
    let mut consumed = worklib::WorkConsumed::default();
    for lib in &loaded_libs {
        consumed
            .libs
            .push((lib.name.clone(), lib.dir_str.clone(), lib.mhash));
    }
    let mut files_seen: std::collections::BTreeSet<(String, [u8; 32])> =
        std::collections::BTreeSet::new();
    for cu in &loaded {
        consumed.blobs.push((cu.blob_path.clone(), cu.blob_hash));
        let mcu = &loaded_libs[cu.lib_idx].manifest.cus[cu.cu_idx];
        for (p, h) in mcu.sources.iter().chain(&mcu.includes) {
            if files_seen.insert((p.clone(), *h)) {
                consumed.files.push((p.clone(), *h));
            }
        }
    }
    let composite = *blake3::hash(&blob_bytes_all).as_bytes();
    write_velab_file(out, &ir, &sc, prec, composite, Some(&consumed))
}

/// `vrun`: read a `.velab`, gate the SimIr hash, decode SimIr+ForkModeTable,
/// simulate (threading `fork_modes` into `SimOpts`), writing the VCD. Returns the
/// doc-13 sim exit code.
/// Exit: 0 clean / 1 gate-reject|corrupt-body|runtime-fatal / 3 missing-file.
pub fn run_vrun(velab_path: &str, opts: &VitaOpts) -> i32 {
    let log = match open_log(opts) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
    let sink = vita_log::GatedSink::new(&inner, opts.gate.clone());
    if inner.verbose() {
        sink.emit(LogEvent::Progress(diag::ProgressEvent {
            message: format!("in: {velab_path}"),
        }));
    }
    let code = run_vrun_gated(velab_path, opts, &inner, &sink);
    inner.epilogue();
    code
}

fn run_vrun_gated(
    velab_path: &str,
    opts: &VitaOpts,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    let bytes = match read_artifact_bytes(velab_path) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let (header, body) = match vita_artifact::read_velab(&bytes) {
        Ok(x) => x,
        Err(e) => return emit_artifact_error(sink, &e), // bad magic → E-ART-FORMAT-MISMATCH
    };
    let tool = vita_artifact::ToolContext::current(); // SimIr-flavored
    if let Err(e) = vita_artifact::verify_header(&header, &tool) {
        return emit_artifact_error(sink, &e); // schema/version → E-ART-SCHEMA-MISMATCH / E-ART-VERSION-GATE
    }
    // v6 ⑤ (RULE V, doc-15 E9003): `--upstream <file.vu>` — re-hash the LIVE
    // upstream bytes and compare against the digest the `.velab` recorded
    // when it consumed them. Content hash only (never mtime); a mismatch
    // refuses to run rather than simulate a stale snapshot. The worklib
    // increment automates upstream DISCOVERY; the verification seam is this.
    if let Some(up) = &opts.upstream {
        let up_bytes = match read_artifact_bytes(up) {
            Ok(b) => b,
            Err(code) => return code,
        };
        let live = *blake3::hash(&up_bytes).as_bytes();
        if live != header.composite_input_hash {
            return emit_artifact_error(
                sink,
                &vita_artifact::ArtifactError {
                    code: diag::MsgCode::ArtStaleUpstream,
                    message: format!(
                        "{up}: digest changed since the .velab snapshot (rerun velab, or drop --upstream)"
                    ),
                },
            );
        }
    }

    // split the golden SimIr frame from the fork trailer.
    let (ir, rest): (sim_ir::SimIr, &[u8]) = match postcard::take_from_bytes(body) {
        Ok(x) => x,
        Err(e) => {
            return emit_artifact_error(
                sink,
                &vita_artifact::ArtifactError::format(&format!(
                    "undecodable .velab SimIr body: {e}"
                )),
            )
        }
    };
    let (fork_modes, rest2): (sim_engine::ForkModeTable, &[u8]) =
        match postcard::take_from_bytes(rest) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab fork trailer: {e}"
                    )),
                )
            }
        };
    // NetNameTable trailer (hierarchical VCD names). Tolerant of an older `.velab`
    // with no names trailer → empty ⇒ flat `n{i}` fallback (no decode error).
    let (net_names, rest3): (sim_engine::NetNameTable, &[u8]) = if rest2.is_empty() {
        (Vec::new(), rest2)
    } else {
        match postcard::take_from_bytes(rest2) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab name trailer: {e}"
                    )),
                )
            }
        }
    };
    // Timescale trailer (proc multipliers + global precision). Tolerant of an older
    // `.velab` with no trailer → 1ns/1ns base ($time unscaled, preamble 1ns).
    let ((proc_multipliers, global_prec_exp), rest4): ((Vec<u64>, i8), &[u8]) = if rest3.is_empty()
    {
        ((Vec::new(), -9), rest3)
    } else {
        match postcard::take_from_bytes(rest3) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab timescale trailer: {e}"
                    )),
                )
            }
        }
    };
    // Severity trailer ($fatal/$error/$warning/$info, P1-1). Tolerant of an older
    // `.velab` with no trailer → empty ⇒ severity tasks degrade to plain $display.
    let (severities, rest5): (sim_engine::SeverityTable, &[u8]) = if rest4.is_empty() {
        (sim_engine::SeverityTable::new(), rest4)
    } else {
        match postcard::take_from_bytes(rest4) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab severity trailer: {e}"
                    )),
                )
            }
        }
    };
    // Radix trailer (b/o/h print variants, P1-5). Tolerant → empty ⇒ decimal.
    let (radixes, rest6): (sim_engine::RadixTable, &[u8]) = if rest5.is_empty() {
        (sim_engine::RadixTable::new(), rest5)
    } else {
        match postcard::take_from_bytes(rest5) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab radix trailer: {e}"
                    )),
                )
            }
        }
    };
    // Scope trailer (`%m`, P2-11). Tolerant → empty ⇒ flat `top`.
    let (proc_scopes, rest7): (Vec<String>, &[u8]) = if rest6.is_empty() {
        (Vec::new(), rest6)
    } else {
        match postcard::take_from_bytes(rest6) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab scope trailer: {e}"
                    )),
                )
            }
        }
    };
    // Assign-rank trailer (§9.3.1 proc assign/deassign). Tolerant → empty ⇒
    // every Force/Release stmt is a real force/release (pre-rank `.velab`s
    // cannot contain proc-assign stmts, so empty is also CORRECT for them).
    let (assign_ranks, rest8): (sim_engine::AssignRankTable, &[u8]) = if rest7.is_empty() {
        (sim_engine::AssignRankTable::new(), rest7)
    } else {
        match postcard::take_from_bytes(rest7) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab assign-rank trailer: {e}"
                    )),
                )
            }
        }
    };
    // Queue-bound trailer (v6 ③). Tolerant → empty ⇒ every queue unbounded
    // (also CORRECT for pre-bound `.velab`s, which reject `[$:N]` upstream).
    let (queue_bounds, rest9): (sim_engine::QueueBoundTable, &[u8]) = if rest8.is_empty() {
        (sim_engine::QueueBoundTable::new(), rest8)
    } else {
        match postcard::take_from_bytes(rest8) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab queue-bound trailer: {e}"
                    )),
                )
            }
        }
    };
    // WorkConsumed trailer (P2-A worklib). Tolerant → empty ⇒ no work gate
    // (legacy/explicit-path `.velab`s carry no library provenance).
    let (consumed, rest10): (worklib::WorkConsumed, &[u8]) = if rest9.is_empty() {
        (worklib::WorkConsumed::default(), rest9)
    } else {
        match postcard::take_from_bytes(rest9) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab work-consumed trailer: {e}"
                    )),
                )
            }
        }
    };
    // net-dims trailer (Phase-1.x ⑤). Tolerant → empty ⇒ 1-D 0-based VCD names.
    let (net_dims, rest11): (sim_engine::NetDimsTable, &[u8]) = if rest10.is_empty() {
        (sim_engine::NetDimsTable::new(), &[])
    } else {
        match postcard::take_from_bytes(rest10) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab net-dims trailer: {e}"
                    )),
                )
            }
        }
    };
    // 11th: P2-E final ProcIds. Tolerant → empty ⇒ no final blocks (legacy).
    let (final_procs, rest12): (std::collections::BTreeSet<u32>, &[u8]) = if rest11.is_empty() {
        (std::collections::BTreeSet::new(), rest11)
    } else {
        match postcard::take_from_bytes(rest11) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab final-procs trailer: {e}"
                    )),
                )
            }
        }
    };
    // 12th + 13th (§16.4 deferred asserts). Tolerant → empty ⇒ no deferred
    // asserts (also correct for pre-deferred `.velab`s, which reject `#0`/`final`
    // upstream and never emit a marker).
    let (defer_marks, rest13): (sim_engine::DeferMarkTable, &[u8]) = if rest12.is_empty() {
        (sim_engine::DeferMarkTable::new(), rest12)
    } else {
        match postcard::take_from_bytes(rest12) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab defer-marks trailer: {e}"
                    )),
                )
            }
        }
    };
    let (defer_acts, rest14): (sim_engine::DeferActTable, &[u8]) = if rest13.is_empty() {
        (sim_engine::DeferActTable::new(), rest13)
    } else {
        match postcard::take_from_bytes(rest13) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab defer-acts trailer: {e}"
                    )),
                )
            }
        }
    };
    // 14th (STAGED-DROP audit fix): class/frame-call/2-state/assert-ctl sidecars.
    // Tolerant → all-default for legacy/pre-audit `.velab`s (segment absent), so
    // those decode exactly as before (and plain RTL never populated these). Decode
    // with `take_from_bytes` (not `from_bytes`) so the 15th segment that now
    // follows is exposed as `rest15` instead of being silently ignored.
    let (extra, rest15): (StagedExtraSidecars, &[u8]) = if rest14.is_empty() {
        (StagedExtraSidecars::default(), rest14)
    } else {
        match postcard::take_from_bytes(rest14) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab extra-sidecars trailer: {e}"
                    )),
                )
            }
        }
    };
    // 15th (RULEV-MTIME): per-entry (mtime,size) fast-path stamps. Tolerant →
    // empty for legacy/explicit-path `.velab`s ⇒ every RULE-V entry rehashes, the
    // exact pre-optimization behavior.
    let stamps: WorkStamps = if rest15.is_empty() {
        WorkStamps::default()
    } else {
        match postcard::from_bytes(rest15) {
            Ok(x) => x,
            Err(e) => {
                return emit_artifact_error(
                    sink,
                    &vita_artifact::ArtifactError::format(&format!(
                        "undecodable .velab work-stamps trailer: {e}"
                    )),
                )
            }
        }
    };
    // ── RULE V auto-gate (doc-14 vrun 재검증): re-verify the LIVE upstream the
    //    snapshot recorded — manifest bytes, CU blobs, and raw source/include
    //    files. The authoritative check is the content hash; ANY mismatch refuses
    //    to simulate a stale snapshot (E-ART-STALE-UPSTREAM, exit class 2). The
    //    15th-trailer (mtime,size) stamps (RULEV-MTIME) let an unchanged entry
    //    skip the read+blake3 — a fast-path, never a relaxation: a stamp miss
    //    rehashes, so a mismatch is still caught (modulo the documented mtime
    //    hole: a sub-granularity / mtime-frozen rewrite of identical length). ──
    {
        let stale = |message: String| vita_artifact::ArtifactError {
            code: diag::MsgCode::ArtStaleUpstream,
            message,
        };
        let at = |v: &[Option<FileStamp>], i: usize| v.get(i).copied().flatten();
        for (i, (name, dir, h)) in consumed.libs.iter().enumerate() {
            let mpath = std::path::Path::new(dir).join("lib.toml");
            match check_fresh(&mpath, h, at(&stamps.libs, i)) {
                Freshness::Fresh => {}
                Freshness::Changed => {
                    return emit_artifact_error(
                        sink,
                        &stale(format!(
                            "work library `{name}`: {} changed since the .velab snapshot (re-run velab)",
                            mpath.display()
                        )),
                    )
                }
                Freshness::Unreadable(e) => {
                    return emit_artifact_error(
                        sink,
                        &stale(format!(
                            "work library `{name}`: {}: {e} (re-run `vcmp --work` + velab)",
                            mpath.display()
                        )),
                    )
                }
            }
        }
        for (i, (path, h)) in consumed.blobs.iter().enumerate() {
            match check_fresh(std::path::Path::new(path), h, at(&stamps.blobs, i)) {
                Freshness::Fresh => {}
                Freshness::Changed => {
                    return emit_artifact_error(
                        sink,
                        &stale(format!(
                            "{path}: library blob changed since the .velab snapshot (re-run velab)"
                        )),
                    )
                }
                Freshness::Unreadable(e) => {
                    return emit_artifact_error(
                        sink,
                        &stale(format!("{path}: {e} (re-run `vcmp --work` + velab)")),
                    )
                }
            }
        }
        for (i, (path, h)) in consumed.files.iter().enumerate() {
            match check_fresh(std::path::Path::new(path), h, at(&stamps.files, i)) {
                Freshness::Fresh => {}
                Freshness::Changed => {
                    return emit_artifact_error(
                        sink,
                        &stale(format!(
                            "{path}: source changed since `vcmp --work` (re-run vcmp + velab)"
                        )),
                    )
                }
                Freshness::Unreadable(e) => {
                    return emit_artifact_error(
                        sink,
                        &stale(format!("{path}: {e} (re-run `vcmp --work` + velab)")),
                    )
                }
            }
        }
    }

    // ── simulate ──
    let sim_opts = SimOpts {
        fork_modes,
        net_names,
        proc_multipliers,
        severities,
        radixes,
        assign_ranks,
        queue_bounds,
        proc_scopes,
        net_dims,
        final_procs,
        defer_marks,
        defer_acts,
        // 14th-trailer sidecars (STAGED-DROP fix): without these the staged
        // path silently dropped N7 class/OOP, frame-call, 2-state, and
        // assertion-control behavior (class read 0/X, recursive automatic fn
        // returned X — both exit 0). Now value-identical to one-shot.
        func_table: extra.func_table,
        task_calls_proc: extra.task_calls_proc,
        task_calls_func: extra.task_calls_func,
        two_state_nets: extra.two_state_nets,
        class_handle_nets: extra.class_handle_nets,
        class_new_sites: extra.class_new_sites,
        class_layouts: extra.class_layouts,
        class_field_inits: extra.class_field_inits,
        class_rand: extra.class_rand,
        class_constraints: extra.class_constraints,
        class_dist: extra.class_dist,
        class_randc: extra.class_randc,
        randomize_with: extra.randomize_with,
        class_vtable: extra.class_vtable,
        class_calls: extra.class_calls,
        class_field_widths: extra.class_field_widths,
        assert_fire: extra.assert_fire,
        assert_ctl: extra.assert_ctl,
        clocking_inputs: extra.clocking_inputs,
        clocking_commit: extra.clocking_commit,
        clocking_outputs: extra.clocking_outputs,
        ca_delays: extra.ca_delays,
        // wand/wor wired-logic resolution kinds (STAGED-DROP parity: without
        // these a multi-driven wand/wor net silently used wire resolution on the
        // staged path = wrong value while one-shot was correct).
        wired_and_nets: extra.wired_and_nets,
        wired_or_nets: extra.wired_or_nets,
        // §21.3.2 %t/$timeformat (STAGED-DROP parity): the call-site table rides
        // the extra-sidecars trailer; the precision exponent rides the timescale
        // trailer — without them a staged `%t` mis-scales and a `$timeformat`
        // prints its args.
        timeformat_stmts: extra.timeformat_stmts,
        handle_copy_stmts: extra.handle_copy_stmts,
        queue_slice_stmts: extra.queue_slice_stmts,
        global_prec_exp,
        timescale_unit: timescale_unit_string(global_prec_exp),
        ..opts.sim_opts()
    };
    let result = sim_engine::simulate(&ir, sink, sim_opts);
    let code = sim_exit_code(&result);
    if code == EXIT_OK && inner.had_error_or_fatal() {
        return EXIT_USER_ERROR; // `-Werror`-promoted warning (doc-13 class 1)
    }
    code
}

/// Parse a flat arg list into (positional paths, `-o` value). `-o`/`--out`
/// consume the next arg. Unknown flags → `Err(EXIT_CLI_ERROR)`.
/// Parsed common applet flags.
#[derive(Default)]
struct IoArgs {
    pos: Vec<String>,
    out: Option<String>,
    threads: Option<u32>,
    timeout: Option<u64>,
    gate: vita_log::GatePolicy,
    incdirs: Vec<String>,
    defines: Vec<(String, String)>,
    verbosity: Option<u8>,
    log: Option<String>,
    log_append: bool,
    /// `--dump-filelist`: print the EFFECTIVE post-expansion inputs and exit.
    dump_filelist: bool,
    /// `--upstream <file>` (vrun, v6 ⑤): RULE-V staleness verification.
    upstream: Option<String>,
    /// `--work <name[=dir]>` (vcmp, P2-A): logical work library to record into.
    work: Option<String>,
    /// `--workdir <dir>` (vcmp, P2-A): output dir when `--work` has no `=dir`.
    workdir: Option<String>,
    /// `-L <name[=dir]>` (velab, P2-A): precompiled libraries, search order.
    libs: Vec<String>,
    /// `--top <unit>` (velab, P2-A): explicit root units (required with `-L`).
    tops: Vec<String>,
    /// Runtime plusargs (v7): every bare `+...` arg that is not a
    /// `+define+`/`+incdir+` directive, leading '+' stripped, command-line
    /// order preserved ($test/$value$plusargs search order). vita/vrun only —
    /// the compile applets reject them loud.
    plusargs: Vec<String>,
}

/// W-FLIST-OVERRIDE (always-logged): a single-value knob set twice — proceed
/// with last-wins but say so loudly (doc-14 §3.1).
fn warn_override(knob: &str, old_v: &str, new_v: &str) {
    eprintln!(
        "warning[{}]: {knob} '{old_v}' overridden by '{new_v}' (last wins)",
        MsgCode::FlistOverride.code_num()
    );
}

fn parse_io_args(args: &[String]) -> Result<IoArgs, i32> {
    let mut pos = Vec::new();
    let mut out: Option<String> = None;
    let mut threads: Option<u32> = None;
    let mut timeout: Option<u64> = None;
    let mut upstream: Option<String> = None;
    let mut gate = vita_log::GatePolicy::default();
    let mut incdirs: Vec<String> = Vec::new();
    let mut defines: Vec<(String, String)> = Vec::new();
    let mut verbosity: Option<u8> = None;
    let mut log: Option<String> = None;
    let mut log_append = false;
    let mut dump_filelist = false;
    let mut work: Option<String> = None;
    let mut workdir: Option<String> = None;
    let mut libs: Vec<String> = Vec::new();
    let mut tops: Vec<String> = Vec::new();
    let mut plusargs: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '-o' needs an argument",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = &out {
                    warn_override("-o", prev, v);
                }
                out = Some(v.clone());
                i += 2;
            }
            // P4-T1: worker-thread budget. Output is byte-identical for every N
            // (contract); the value only moves wall-clock.
            "--threads" | "-j" => {
                let parsed = args.get(i + 1).and_then(|v| v.parse::<u32>().ok());
                let Some(n) = parsed else {
                    eprintln!(
                        "error[{}]: '--threads' needs a positive integer",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = threads {
                    warn_override("--threads", &prev.to_string(), &n.to_string());
                }
                threads = Some(n.max(1));
                i += 2;
            }
            // P2-9: CI killswitch — cap advanced sim time (ticks). Reaching the
            // cap ends the run cleanly (Quiescent), bounding `always #1;` hangs.
            "--timeout" => {
                let parsed = args.get(i + 1).and_then(|v| v.parse::<u64>().ok());
                let Some(n) = parsed else {
                    eprintln!(
                        "error[{}]: '--timeout' needs a tick count",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = timeout {
                    warn_override("--timeout", &prev.to_string(), &n.to_string());
                }
                timeout = Some(n);
                i += 2;
            }
            // v6 ⑤ (RULE V): verify the .velab's recorded upstream digest
            // against the live artifact before running.
            "--upstream" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--upstream' needs a file path",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = &upstream {
                    warn_override("--upstream", prev, v);
                }
                upstream = Some(v.clone());
                i += 2;
            }
            "--work" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--work' needs a name[=dir]",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = &work {
                    warn_override("--work", prev, v);
                }
                work = Some(v.clone());
                i += 2;
            }
            "--workdir" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--workdir' needs a directory",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = &workdir {
                    warn_override("--workdir", prev, v);
                }
                workdir = Some(v.clone());
                i += 2;
            }
            "-L" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '-L' needs a name[=dir]",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                libs.push(v.clone());
                i += 2;
            }
            "--top" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--top' needs a unit name",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                tops.push(v.clone());
                i += 2;
            }
            "-D" | "--define" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '-D' needs NAME[=VAL]",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                defines.push(split_define(v));
                i += 2;
            }
            "-I" | "--incdir" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '-I' needs a directory",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                incdirs.push(v.clone());
                i += 2;
            }
            // vita-log stage 2: verbosity + transcript tee (doc-13 bucket C —
            // pure sink policy, never hashed into artifacts).
            "-q" | "--quiet" => {
                verbosity = Some(0);
                i += 1;
            }
            "-v" => {
                verbosity = Some(2);
                i += 1;
            }
            "-vv" => {
                verbosity = Some(3);
                i += 1;
            }
            "--verbosity" => {
                let parsed = args.get(i + 1).and_then(|v| v.parse::<u8>().ok());
                let Some(n) = parsed.filter(|&n| n <= 3) else {
                    eprintln!(
                        "error[{}]: '--verbosity' needs 0..=3",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                verbosity = Some(n);
                i += 2;
            }
            "-l" | "--log" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--log' needs a path ('-' = stderr)",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = &log {
                    warn_override("--log", prev, v);
                }
                log = Some(v.clone());
                i += 2;
            }
            "--log-append" => {
                log_append = true;
                i += 1;
            }
            "--dump-filelist" => {
                dump_filelist = true;
                i += 1;
            }
            s if s.starts_with("+define+") => {
                // `+define+N=V+M[=…]` — '+'-joined multi-value (doc-14 §3.1).
                for seg in s["+define+".len()..].split('+').filter(|t| !t.is_empty()) {
                    defines.push(split_define(seg));
                }
                i += 1;
            }
            s if s.starts_with("+incdir+") => {
                for seg in s["+incdir+".len()..].split('+').filter(|t| !t.is_empty()) {
                    incdirs.push(seg.to_string());
                }
                i += 1;
            }
            // v7: any other `+...` arg is a RUNTIME plusarg (vvp convention).
            s if s.starts_with('+') && s.len() > 1 => {
                plusargs.push(s[1..].to_string());
                i += 1;
            }
            s if s.starts_with('-') && s.len() > 1 => {
                // Diagnostic gate flags (`-Wno-<CODE>` / `-Werror[=<CODE>]`).
                match gate.parse_arg(s) {
                    Some(Ok(())) => {
                        i += 1;
                        continue;
                    }
                    Some(Err(msg)) => {
                        eprintln!("error[{}]: {msg}", MsgCode::CliBadFlag.code_num());
                        return Err(EXIT_CLI_ERROR);
                    }
                    None => {}
                }
                eprintln!(
                    "error[{}]: unknown flag '{s}'",
                    MsgCode::CliBadFlag.code_num()
                );
                return Err(EXIT_CLI_ERROR);
            }
            _ => {
                pos.push(args[i].clone());
                i += 1;
            }
        }
    }
    Ok(IoArgs {
        pos,
        out,
        threads,
        timeout,
        gate,
        incdirs,
        defines,
        verbosity,
        log,
        log_append,
        dump_filelist,
        upstream,
        work,
        workdir,
        libs,
        tops,
        plusargs,
    })
}

/// `--dump-filelist` (doc-14 §3.1 debugging surface): print the EFFECTIVE
/// post-expansion inputs — sources in argv order, then defines, then incdirs
/// — and exit 0 without compiling. Deterministic (no sorting, no resolution
/// beyond what the expansion itself did), so CI can diff two trees' effective
/// inputs directly.
fn run_dump_filelist(io: &IoArgs) -> i32 {
    for f in &io.pos {
        println!("source {f}");
    }
    for (n, v) in &io.defines {
        if v.is_empty() {
            println!("define {n}");
        } else {
            println!("define {n}={v}");
        }
    }
    for d in &io.incdirs {
        println!("incdir {d}");
    }
    EXIT_OK
}

/// E-FLIST-WRONG-STAGE: velab/vrun have no preprocess pass — a `+define+`/
/// `+incdir+`/`-D`/`-I` reaching them (argv or expanded from a `.f`) would be
/// silently meaningless. Reject loudly (doc-14 §3.1).
fn reject_preprocess_buckets(stage: &str, io: &IoArgs) -> Result<(), i32> {
    if io.defines.is_empty() && io.incdirs.is_empty() {
        return Ok(());
    }
    eprintln!(
        "error[{}]: +define+/+incdir+/-D/-I are compile-stage (vcmp/vita) inputs — \
         '{stage}' has no preprocess pass, so they would be silently meaningless",
        MsgCode::FlistWrongStage.code_num()
    );
    Err(EXIT_CLI_ERROR)
}

/// Loud wrong-stage rejection for the worklib flag family — `--work`/`--workdir`
/// belong to vcmp, `-L`/`--top` to velab; anywhere else they would be silently
/// meaningless (the E-FLIST-WRONG-STAGE principle applied to argv).
/// v7: runtime plusargs are vita/vrun inputs; the compile stages reject them
/// (a stray `+FOO` at vcmp is far more likely a typo'd `+define+`).
fn reject_plusargs(stage: &str, io: &IoArgs) -> Result<(), i32> {
    if !io.plusargs.is_empty() {
        eprintln!(
            "error[{}]: runtime plusargs (+{}) are vita/vrun arguments — '{stage}' \
             compiles, it does not simulate",
            MsgCode::CliBadFlag.code_num(),
            io.plusargs[0]
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

fn reject_worklib_flags(
    stage: &str,
    io: &IoArgs,
    allow_work: bool,
    allow_libs: bool,
) -> Result<(), i32> {
    if !allow_work && (io.work.is_some() || io.workdir.is_some()) {
        eprintln!(
            "error[{}]: --work/--workdir are vcmp flags — '{stage}' does not write libraries",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    if !allow_libs && !io.libs.is_empty() {
        eprintln!(
            "error[{}]: -L is a velab flag — '{stage}' does not read libraries",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    if !allow_libs && !io.tops.is_empty() {
        eprintln!(
            "error[{}]: --top is a velab flag — '{stage}' has no root selection",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

/// Resolve `--work <name[=dir]>` / `--workdir <dir>` into (logical name, dir):
/// `--work n=d` pins both; `--work n` puts the library at `./n` unless
/// `--workdir` overrides; a bare `--workdir d` means the default name `work`.
fn parse_work_spec(io: &IoArgs) -> Result<Option<(String, String)>, i32> {
    let spec = match (&io.work, &io.workdir) {
        (None, None) => return Ok(None),
        (None, Some(d)) => ("work".to_string(), d.clone()),
        (Some(w), wd) => match w.split_once('=') {
            Some((n, d)) if !n.is_empty() && !d.is_empty() => (n.to_string(), d.to_string()),
            Some(_) => {
                eprintln!(
                    "error[{}]: '--work' needs name[=dir] with both parts non-empty",
                    MsgCode::CliBadFlag.code_num()
                );
                return Err(EXIT_CLI_ERROR);
            }
            None => (w.clone(), wd.clone().unwrap_or_else(|| format!("./{w}"))),
        },
    };
    Ok(Some(spec))
}

fn dispatch_vcmp(args: &[String]) -> i32 {
    let io = match parse_io_args(args) {
        Ok(x) => x,
        Err(c) => return c,
    };
    if io.dump_filelist {
        return run_dump_filelist(&io);
    }
    if let Err(c) = reject_worklib_flags("vcmp", &io, true, false) {
        return c;
    }
    if let Err(c) = reject_plusargs("vcmp", &io) {
        return c;
    }
    if io.pos.is_empty() {
        eprintln!(
            "error[{}]: vcmp: no source files",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    let work = match parse_work_spec(&io) {
        Ok(w) => w,
        Err(c) => return c,
    };
    // `-o` stays the default flow; with `--work` the library IS the output and
    // an explicit `-o` additionally writes the plain `.vu` (same bytes).
    let out = match (&io.out, &work) {
        (Some(o), _) => Some(o.clone()),
        (None, Some(_)) => None,
        (None, None) => Some(default_out(&io.pos[0], "vu")),
    };
    if let Some(o) = &out {
        if let Err(c) = reject_out_clobbers_input(&io.pos, o) {
            return c;
        }
    }
    run_vcmp(
        &io.pos,
        out.as_deref(),
        &VitaOpts {
            gate: io.gate,
            incdirs: io.incdirs,
            defines: io.defines,
            verbosity: io.verbosity,
            log: io.log,
            log_append: io.log_append,
            work,
            ..VitaOpts::default()
        },
    )
}

fn dispatch_velab(args: &[String]) -> i32 {
    let io = match parse_io_args(args) {
        Ok(x) => x,
        Err(c) => return c,
    };
    if io.dump_filelist {
        return run_dump_filelist(&io);
    }
    if let Err(c) = reject_preprocess_buckets("velab", &io) {
        return c;
    }
    if let Err(c) = reject_worklib_flags("velab", &io, false, true) {
        return c;
    }
    if let Err(c) = reject_plusargs("velab", &io) {
        return c;
    }
    // ── library mode (`-L`): logical discovery instead of a positional .vu ──
    if !io.libs.is_empty() {
        if !io.pos.is_empty() {
            eprintln!(
                "error[{}]: velab: a positional .vu and -L libraries are mutually exclusive",
                MsgCode::CliBadFlag.code_num()
            );
            return EXIT_CLI_ERROR;
        }
        if io.tops.is_empty() {
            eprintln!(
                "error[{}]: velab: library mode needs at least one --top <unit> \
                 (a library's unrelated units must not become roots)",
                MsgCode::CliBadFlag.code_num()
            );
            return EXIT_CLI_ERROR;
        }
        let mut libs: Vec<(String, String)> = Vec::new();
        for l in &io.libs {
            match l.split_once('=') {
                Some((n, d)) if !n.is_empty() && !d.is_empty() => {
                    libs.push((n.to_string(), d.to_string()))
                }
                Some(_) => {
                    eprintln!(
                        "error[{}]: '-L' needs name[=dir] with both parts non-empty",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return EXIT_CLI_ERROR;
                }
                None => libs.push((l.clone(), format!("./{l}"))),
            }
        }
        let out = io.out.unwrap_or_else(|| format!("{}.velab", io.tops[0]));
        return run_velab_lib(
            &libs,
            &io.tops,
            &out,
            &VitaOpts {
                gate: io.gate,
                verbosity: io.verbosity,
                log: io.log,
                log_append: io.log_append,
                ..VitaOpts::default()
            },
        );
    }
    if io.pos.len() != 1 {
        eprintln!(
            "error[{}]: velab: expected exactly one .vu input",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    let out = io.out.unwrap_or_else(|| default_out(&io.pos[0], "velab"));
    if let Err(c) = reject_out_clobbers_input(&io.pos, &out) {
        return c;
    }
    run_velab(
        &io.pos[0],
        &out,
        &VitaOpts {
            gate: io.gate,
            verbosity: io.verbosity,
            log: io.log,
            log_append: io.log_append,
            tops: io.tops,
            ..VitaOpts::default()
        },
    )
}

fn dispatch_vrun(args: &[String]) -> i32 {
    let io = match parse_io_args(args) {
        Ok(x) => x,
        Err(c) => return c,
    };
    if io.dump_filelist {
        return run_dump_filelist(&io);
    }
    if let Err(c) = reject_preprocess_buckets("vrun", &io) {
        return c;
    }
    if let Err(c) = reject_worklib_flags("vrun", &io, false, false) {
        return c;
    }
    if io.pos.len() != 1 {
        eprintln!(
            "error[{}]: vrun: expected exactly one .velab input",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    // vrun accepts `-o` as a VCD path override (parity with one-shot vita -o).
    // Guard: a `-o` that names the input `.velab` would clobber the file being read.
    if let Some(ref o) = io.out {
        if let Err(c) = reject_out_clobbers_input(&io.pos, o) {
            return c;
        }
    }
    let opts = VitaOpts {
        vcd_path_override: io.out,
        threads: io.threads,
        time_limit: io.timeout,
        gate: io.gate,
        verbosity: io.verbosity,
        log: io.log,
        log_append: io.log_append,
        upstream: io.upstream,
        plusargs: io.plusargs,
        ..VitaOpts::default()
    };
    run_vrun(&io.pos[0], &opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    // TRAILER-PIN (ROADMAP §5.3, 2026-06-23): the `.velab` trailers ride OUTSIDE
    // the SchemaHash-pinned SimIr frame, so a silent shape edit to one (e.g. a new
    // field on the hand-maintained `StagedExtraSidecars`) that forgets a
    // format_version bump makes old artifacts decode wrong. Pin the postcard wire
    // shape of a populated `StagedExtraSidecars` fixture: any field add / remove /
    // reorder / type change flips the hash. (The other trailers are plain
    // sim-engine/sim-ir types under their own coverage; this is the cli-local one
    // the STAGED-DROP audit flagged as the fragile, hand-maintained trailer.)
    #[test]
    fn staged_extra_sidecars_wire_shape_is_pinned() {
        let mut s = StagedExtraSidecars::default();
        s.two_state_nets.insert(7);
        s.class_handle_nets.insert(2);
        s.class_new_sites.insert(3, 9);
        s.class_layouts = vec![vec![(1, true, false)], vec![(2, false, true)]];
        s.class_field_inits = vec![vec![
            None,
            Some(sim_ir::BitPacked {
                val: vec![5],
                unk: vec![0],
            }),
        ]];
        s.class_vtable = vec![vec![10, 11]];
        s.class_calls.insert(5, (Some(2), 11));
        s.class_field_widths.insert(8, (16, true));
        s.assert_fire.insert(6);
        s.assert_ctl.insert(4, 2);
        s.class_rand = vec![
            vec![(0, 32, true, 1, 6, true)],
            vec![(1, 64, false, 0, 0, false)],
        ];
        s.class_constraints = vec![
            vec![vec![
                sim_ir::COp::Field(0),
                sim_ir::COp::Field(1),
                sim_ir::COp::Bin(sim_ir::CBinOp::Lt),
            ]],
            vec![vec![sim_ir::COp::Const(7), sim_ir::COp::Not]],
        ];
        s.class_dist = vec![
            vec![(0, vec![(1, 1, 10), (2, 5, 20)])],
            vec![(3, vec![(0, 0, 1)])],
        ];
        s.class_randc = vec![vec![(2, 0, 15)], vec![(4, -8, 7)]];
        s.randomize_with = vec![
            (
                vec![(0, 1, 9), (1, -3, 3)],
                vec![vec![
                    sim_ir::COp::Field(0),
                    sim_ir::COp::Field(1),
                    sim_ir::COp::Bin(sim_ir::CBinOp::Lt),
                ]],
            ),
            (
                vec![],
                vec![vec![sim_ir::COp::SoftMarker, sim_ir::COp::Field(2)]],
            ),
        ];
        s.clocking_inputs = std::collections::BTreeSet::from([3u32, 7u32]);
        s.clocking_commit =
            std::collections::BTreeMap::from([(5u32, vec![(8u32, 3u32), (9u32, 7u32)])]);
        s.clocking_outputs = std::collections::BTreeMap::from([(6u32, vec![(4u32, 9u32)])]);
        s.ca_delays = std::collections::BTreeMap::from([(0u32, (1u32, 2u32, 3u32))]);
        s.wired_and_nets = std::collections::BTreeSet::from([11u32, 13u32]);
        s.wired_or_nets = std::collections::BTreeSet::from([17u32]);
        s.timeformat_stmts = std::collections::BTreeSet::from([19u32]);
        s.handle_copy_stmts = std::collections::BTreeMap::from([(23u32, (2u32, 5u32))]);
        s.queue_slice_stmts = std::collections::BTreeSet::from([29u32]);
        let bytes = postcard::to_stdvec(&s).expect("postcard encode");
        let got = blake3::hash(&bytes).to_hex().to_string();
        // REGEN_GOLDEN=1 cargo test -p cli staged_extra_sidecars_wire_shape -- --nocapture
        if std::env::var("REGEN_GOLDEN").is_ok() {
            println!("REGEN StagedExtraSidecars wire = {got}");
            return;
        }
        const EXPECTED: &str = "4407caf64753b4eb58110ba4381068f4800e95a51f98c1f1564a9c9fcfd15c96";
        assert_eq!(
            got, EXPECTED,
            "StagedExtraSidecars wire shape changed — a 15th-trailer field moved.\n\
             If intentional: bump format_version + regen with REGEN_GOLDEN=1."
        );
    }

    // The `%t` M-saturation fix widened `proc_multipliers` from Vec<u32> to
    // Vec<u64> WITHOUT a format_version bump — legal ONLY because postcard
    // varint-encodes both identically for every value < 2^32, so an old
    // artifact's u32-encoded timescale trailer decodes byte-exactly as u64.
    // This pins that load-bearing wire-compat invariant (if postcard ever
    // switched to fixed-width ints, this fails and a bump is required).
    #[test]
    fn timescale_trailer_u32_to_u64_wire_compat() {
        let old: (Vec<u32>, i8) = (vec![1, 1000, 1_000_000_000, u32::MAX], -12);
        let bytes = postcard::to_stdvec(&old).expect("encode u32 trailer");
        let (new, rest): ((Vec<u64>, i8), &[u8]) =
            postcard::take_from_bytes(&bytes).expect("decode as u64 trailer");
        assert!(rest.is_empty());
        assert_eq!(new.1, -12);
        assert_eq!(new.0, vec![1u64, 1000, 1_000_000_000, u32::MAX as u64]);
    }

    // RULEV-MTIME wire pin: the 15th `WorkStamps` trailer is also a hand-maintained
    // cli-local struct riding outside the SimIr frame. A field add / reorder / type
    // change to it (or to `FileStamp`) silently mis-decodes old artifacts unless the
    // format_version bumps. Pin a populated fixture's postcard shape the same way.
    #[test]
    fn work_stamps_wire_shape_is_pinned() {
        let s = WorkStamps {
            libs: vec![Some((1_700_000_000, 123, 4096))],
            blobs: vec![None, Some((1_700_000_001, 0, 64))],
            files: vec![Some((1_700_000_002, 999_999_999, 1))],
        };
        let bytes = postcard::to_stdvec(&s).expect("postcard encode");
        let got = blake3::hash(&bytes).to_hex().to_string();
        // REGEN_GOLDEN=1 cargo test -p cli work_stamps_wire_shape -- --nocapture
        if std::env::var("REGEN_GOLDEN").is_ok() {
            println!("REGEN WorkStamps wire = {got}");
            return;
        }
        const EXPECTED: &str = "923a29a56aa0974671e5453f2e9bab0dcd96e57662cd63aed6c858113e6efb38";
        assert_eq!(
            got, EXPECTED,
            "WorkStamps wire shape changed — a 15th-trailer field moved.\n\
             If intentional: bump format_version + regen with REGEN_GOLDEN=1."
        );
    }

    /// Run `run_vita` against an on-disk temp file holding `src`; return the exit
    /// code. The temp path is unique per call so tests stay parallel-safe.
    fn run_on_temp(src: &str, opts: &VitaOpts) -> (i32, String) {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = dir.join(format!("vita_cli_test_{pid}_{nonce}.sv"));
        std::fs::write(&path, src).unwrap();
        let code = run_vita(&[path.to_string_lossy().into_owned()], opts);
        let p = path.to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&path);
        (code, p)
    }

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    const CLEAN_TB: &str =
        "module tb; reg a; initial begin a=1; $display(\"a=%b\",a); #5 $finish; end endmodule";

    #[test]
    fn clean_testbench_exits_zero_and_prints() {
        // The capture API proves the $display text; the exit code proves the flow.
        let (toks, lex_errs) = hdl_lexer::lex(CLEAN_TB);
        assert!(lex_errs.is_empty(), "lex: {lex_errs:?}");
        let (unit, perrs) = hdl_parser::parse(&toks, CLEAN_TB);
        assert!(perrs.is_empty(), "parse: {perrs:?}");
        let sink = StderrSink::new();
        let ir = elaborate::elaborate(&unit.unwrap(), &sink).expect("elaborate");
        let (result, stdout) = sim_engine::simulate_capture(&ir, SimOpts::default());
        assert!(stdout.contains("a=1"), "stdout was: {stdout:?}");
        assert_eq!(sim_exit_code(&result), EXIT_OK);

        // And the full run_vita path returns 0.
        let (code, _) = run_on_temp(CLEAN_TB, &VitaOpts::default());
        assert_eq!(code, EXIT_OK);
    }

    #[test]
    fn parse_error_exits_one() {
        // A `$display` with a missing `)` / `;` — guaranteed parse error.
        let bad = "module m; initial $display(\"x\" ; endmodule";
        let (code, _) = run_on_temp(bad, &VitaOpts::default());
        assert_eq!(code, EXIT_USER_ERROR);
    }

    #[test]
    fn lex_error_exits_one() {
        let bad = "module m; reg a; initial a = \"unterminated; endmodule";
        let (code, _) = run_on_temp(bad, &VitaOpts::default());
        assert_eq!(code, EXIT_USER_ERROR);
    }

    #[test]
    fn no_source_files_exits_three() {
        assert_eq!(run_vita(&[], &VitaOpts::default()), EXIT_CLI_ERROR);
    }

    #[test]
    fn missing_file_exits_three() {
        let missing = "/nonexistent/path/that/does/not/exist_vita.sv".to_string();
        assert_eq!(run_vita(&[missing], &VitaOpts::default()), EXIT_CLI_ERROR);
    }

    #[test]
    fn vcmp_missing_source_via_run_exits_three() {
        // `vcmp` is now real: `vcmp <missing>.sv` routes to dispatch_vcmp, which
        // fails on the missing-file READ path → exit 3 (CLI/usage error, not a
        // stub). The path is deliberately one that cannot exist.
        let missing = "/nonexistent/path/unknown_applet_top.sv".to_string();
        let argv = vec!["/usr/local/bin/vcmp".to_string(), missing.clone()];
        assert_eq!(run(&argv), EXIT_CLI_ERROR);
        // explicit `vita vcmp …` form routes the same way.
        let argv = vec!["vita".to_string(), "vcmp".to_string(), missing];
        assert_eq!(run(&argv), EXIT_CLI_ERROR);
    }

    #[test]
    fn unknown_flag_to_staged_applet_exits_three() {
        // A genuinely-unknown flag to a staged applet is a CLI/usage error (exit 3)
        // — proves the arg parser rejects, not the stub.
        let argv = vec![
            "/usr/local/bin/vcmp".to_string(),
            "--bogus-flag".to_string(),
            "x.sv".to_string(),
        ];
        assert_eq!(run(&argv), EXIT_CLI_ERROR);
    }

    #[test]
    fn vita_basename_resolves_to_one_shot() {
        let argv = vec!["/usr/local/bin/vita".to_string(), "x.sv".to_string()];
        let (applet, rest) = resolve_applet(&argv);
        assert_eq!(applet, Applet::Vita);
        assert_eq!(rest, vec!["x.sv".to_string()]);
    }

    #[test]
    fn dumpvars_writes_vcd_with_enddefinitions() {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let vcd = dir.join(format!("vita_cli_test_{pid}_{nonce}.vcd"));
        let vcd_str = vcd.to_string_lossy().into_owned();
        let src = format!(
            "module tb; reg a; initial begin $dumpfile(\"{}\"); $dumpvars(0, tb); a=1; #5 $finish; end endmodule",
            vcd_str.replace('\\', "\\\\")
        );
        let opts = VitaOpts {
            vcd_path_override: Some(vcd_str.clone()),
            ..VitaOpts::default()
        };
        let (code, _) = run_on_temp(&src, &opts);
        assert_eq!(code, EXIT_OK);
        assert!(vcd.exists(), "VCD not written at {vcd_str}");
        let mut contents = String::new();
        std::fs::File::open(&vcd)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(
            contents.contains("$enddefinitions"),
            "VCD missing $enddefinitions:\n{contents}"
        );
        let _ = std::fs::remove_file(&vcd);
    }
}
