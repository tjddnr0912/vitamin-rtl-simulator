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
use sim_engine::{Backend, ExitClass, FinishReason, SimOpts};

// ---- split parts (mechanical refactor) ----
mod frontend;
mod pipeline;
mod stage_args;
mod staged;
pub use frontend::*;
pub use pipeline::*;
pub(crate) use stage_args::*;
pub use staged::*;
#[cfg(test)]
mod tests;

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

mod echo;
mod filelist;
mod obs;
pub mod worklib;

/// The resolved command line a `-v` run echoes back (doc-13 bucket C).
///
/// A `vita` invocation driven by a Makefile or a shell script reaches the
/// process with every `$(VAR)` already substituted, so the *variable names* the
/// author reads in the Makefile and the *values* the simulator actually saw are
/// two different texts. Nothing downstream can reconstruct the second from the
/// first — env vars are gone by then, `-f` frames have been spliced away, and
/// `VITA_THREADS` never appears in argv at all. This record is the one place
/// that copy is kept, so `-v` can print it into the transcript (and therefore
/// into `--log`).
///
/// Populated ONLY by the argv driver ([`run`]). The library entry points
/// (`run_vita_str`, `run_vita_sources` from unit tests) leave it `None` — their
/// output must not depend on the harness's own command line.
#[derive(Debug, Clone, Default)]
pub struct Invocation {
    /// The process argv — argv[0] by basename, then the arguments exactly as
    /// they arrived (post-shell-expansion, pre-filelist-expansion). This is
    /// what the Makefile really ran, including the multicall subcommand token
    /// when one was used (`vita velab …`).
    pub argv: Vec<String>,
    /// Directory every relative path in `argv` resolved against.
    pub cwd: String,
    /// `-f`/`-F` filelists that were opened, in depth-first expansion order.
    pub filelists: Vec<String>,
}

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
    /// `-G NAME=VALUE` / `--param NAME=VALUE`: parameter overrides for the TOP
    /// module(s). Without this a configuration sweep needs one hand-written wrapper
    /// module per combination, and the same filelist cannot be shared with a tool
    /// that does support overrides (xrun `-defparam`, VCS `-pvalue+`, Verilator `-G`).
    pub top_params: Vec<(String, String)>,
    /// Runtime plusargs (v7, `+name[=value]`, leading '+' stripped, CLI
    /// order). Searched first-match by `$test/$value$plusargs`. Pure runtime
    /// input — never hashed into artifacts.
    pub plusargs: Vec<String>,
    /// `--obs-dir <D>` (G2 OBS-1a): write the run manifest + result ledger
    /// (`run.json` + `results.jsonl`) into `D` at end-of-run. `None` ⇒ no obs
    /// rail (byte-identical to before). Out-of-band sink — never hashed into
    /// artifacts, never enters the golden IR. One-shot `vita` only for v1.
    pub obs_dir: Option<String>,
    /// `--obs-procs` / `--obs-procs-time` (R14, ROADMAP §3 ⑭): add the
    /// `processes` object — per-body evaluation counts, and with
    /// `--obs-procs-time` cumulative wall-clock — to `run.json`. `None` ⇒ no
    /// profile at all (nothing is allocated and both dispatch seams cost one
    /// null test). Requires `--obs-dir`.
    pub proc_profile: Option<sim_engine::ProcProfileCfg>,
    /// `--hier-tree <path>` (design-structure export): after elaborate, write the module
    /// hierarchy as an indented tree (`instance : module`, top at the root) to `<path>`.
    /// `None` ⇒ not requested. Out-of-band (never hashed / never in the golden IR).
    pub hier_tree: Option<String>,
    /// `--inst-paths <path>` (design-structure export): after elaborate, write every
    /// instance's full dotted path from the top (one per line, VCD-scope-consistent) to
    /// `<path>` — copy/paste-ready for scope-setting / signal-force control.
    pub inst_paths: Option<String>,
    /// `--probe <path>` (OBS-2): hierarchical net names to trace into `trace.jsonl`
    /// (requires `--obs-dir`). Resolved to net ids after elaborate (miss ⇒ loud).
    /// EMPTY ⇒ no probing. One-shot `vita` only.
    pub probes: Vec<String>,
    /// `--probe-file <F>` (OBS-2): a file of probe paths, one per line (`#` comments
    /// and blank lines skipped), merged with `--probe`.
    pub probe_file: Option<String>,
    /// W-FLIST-OVERRIDE events recorded during arg parsing (knob, old, new) —
    /// emitted through the GATED sink at pipeline start so `-Wno-*`/`-Werror=`
    /// and the counts epilogue apply uniformly (doc-13).
    pub overrides: Vec<(String, String, String)>,
    /// The resolved command line, for the `-v` invocation echo. `None` outside
    /// the argv driver (see [`Invocation`]) — the echo then prints only the
    /// facts it can state without inventing an argv.
    pub invocation: Option<Invocation>,
    /// `--backend <interp|vm|native>`: which executor runs process bodies. `None` ⇒
    /// [`Backend::Native`], the tier-3 backend (the default since Phase B1 — it runs
    /// every design in the corpus, and the flip run measured the whole suite
    /// byte-identical under it, which is a far stronger gate than the differential
    /// corpus alone).
    ///
    /// Neither value may change a single output byte — that equivalence is what
    /// `sim-engine/tests/backend_equiv.rs` locks — so this is a wall-clock knob only,
    /// exactly like `--threads`. `interp` exists to bisect a suspected VM defect
    /// against the reference semantics in one flag.
    pub backend: Option<Backend>,
}

impl VitaOpts {
    fn sim_opts(&self) -> SimOpts {
        SimOpts {
            vcd_path_override: self.vcd_path_override.clone(),
            threads: resolve_threads(self.threads),
            time_limit: self.time_limit,
            plusargs: self.plusargs.clone(),
            backend: self.backend.unwrap_or_default(),
            proc_profile: self.proc_profile,
            ..SimOpts::default()
        }
    }
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

    /// Count of Warning-severity diagnostics seen so far (obs manifest).
    pub fn warning_count(&self) -> u32 {
        self.warnings.get()
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
        // A runtime diagnostic knows when it fired (every runtime emitter
        // stamps `sim_time` — the renderer was dropping it), and since #10 the
        // SEVERITY family also knows where: elaborate resolves each severity
        // statement's span into `severity_locs` and the emitters attach it
        // (the engine itself still works on span-free IR). Without the time
        // a design with many `unique case` sites or many indexed arrays reports
        // N identical lines that cannot be told apart; the time alone separates
        // "during reset" from "in steady state", which is the question a reader
        // actually asks. Same wording and same clock as the `simulation ended
        // (…) at time N` epilogue. Elaborate/parse diagnostics carry `None` and
        // are unchanged.
        let when = match &d.sim_time {
            Some(t) => format!(" [at time {}]", t.ticks),
            None => String::new(),
        };
        // The INSTANCE path, when the emitter knew one. `file:line:col` alone
        // does not identify an elaborate diagnostic: a module instantiated N
        // times produces N of them at ONE source line, and until this was
        // rendered the only way to tell them apart was to not have to.
        let whose = match d.context.first() {
            Some(f) => format!(" [in {}]", f.label),
            None => String::new(),
        };
        // The MNEMONIC next to the number. doc-15 is the reference a reader is
        // sent to and 42 of its 55 worked examples print this form; the product
        // printed none of them. It is also the only string that works in
        // `-Wno-`/`-Werror=` unambiguously — the number can be shared by
        // unrelated diagnostics (`VITA-W3056` is the generic simplification
        // channel), and seeing the mnemonic is how a reader learns that
        // suppressing it is wider than the line they are looking at.
        let head = format!(
            "{}[{}] {}: {}{whose}{when}",
            d.severity.token(),
            d.code.code_num(),
            d.code.mnemonic(),
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

/// (parsed unit, resolved timescales, include closure as (path, raw digest)).
pub type FrontendUnit = (
    hdl_ast::SourceUnit,
    hdl_preprocess::ResolvedTimescales,
    Vec<(String, [u8; 32])>,
);

/// Which multicall applet was requested (by `argv[0]` basename, or `vita <sub>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applet {
    /// The one-shot driver (implemented).
    Vita,
    /// A staged-flow applet (`vcmp`/`velab`/`vrun`).
    Staged(&'static str),
}

/// The doc-15 catalog, embedded at compile time (cargo-only — no build.rs).
/// doc-15 is the single authority for cause/example/fix text; the bijection
/// test guarantees every `MsgCode` has a full entry in it.
const ERROR_CATALOG: &str = include_str!("../../../docs/preview/15-error-code-reference.md");

/// `.vu` decode result: the `SourceUnit` + its timescale env — per-module
/// `unit_exp` map, per-module `prec_exp` map (v22 two-stage `#delay`), the
/// design-wide `global_prec_exp` — and the reconstructed preprocessor
/// `SourceMap` (v28 tail). NOT an `Option`: every v28 `vcmp` writes the tail,
/// so an absent one is a truncated artifact and `decode_vu_unit` is loud about
/// it — a tolerant `None` here would silently resurrect the location-less
/// staged diagnostics the tail exists to end.
type VuUnitEnv = (
    hdl_ast::SourceUnit,
    std::collections::BTreeMap<String, i8>,
    std::collections::BTreeMap<String, i8>,
    i8,
    hdl_preprocess::SourceMap,
);

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
    /// N1/round-11: FuncId → "module.function" for frame-body `%m` (staged
    /// velab→vrun must carry it or a frame `%m` silently renders the module
    /// scope only). APPEND-ONLY tail. Rides the format_version 20 bump — unlike
    /// the empty-default tails above, this is POPULATED for any frame subroutine
    /// (so NOT byte-identical), which is exactly why the wire-shape pin below
    /// forced the bump. Old `.velab` are loud-rejected at the header gate.
    #[serde(default)]
    func_names: Vec<String>,
    /// N3 Phase 2 heterogeneous heap: DynArray handle NetIds whose ELEMENTS are
    /// `real` / `string` (`real r[]` / `string s[]`). The engine flags the net
    /// `is_real` / string-element and fills `new[]` with 0.0 / "". APPEND-ONLY tail;
    /// rides the format_version 21 bump. EMPTY for any design without a real/string
    /// dyn array (or record). Old `.velab` are loud-rejected at the header gate.
    #[serde(default)]
    real_elem_dyn_nets: std::collections::BTreeSet<u32>,
    #[serde(default)]
    string_elem_dyn_nets: std::collections::BTreeSet<u32>,
    /// DECLARED packed `(msb, lsb)` for a net whose stored `NetVar.msb`/`lsb` cannot
    /// express it — a NEGATIVE low bound (`logic [3:-2] x`), stored normalized as
    /// `[w-1:0]` because those fields are frozen `u32`. Drives the VCD `$var` range only
    /// (`x [3:-2]`, matching iverilog); without it a STAGED run labels the bits
    /// `[5:0]` — the values are right, the indices in the waveform are not.
    /// APPEND-ONLY tail; rides the format_version 25 bump. EMPTY for any design without
    /// such a net ⇒ byte-identical.
    #[serde(default)]
    net_decl_ranges: sim_engine::NetDeclRangeTable,
    /// `$fmonitor`/`$fstrobe` call-site StmtIds (they share the frozen `Monitor`/`Strobe`
    /// ids, so without this a STAGED run prints them to stdout instead of the file).
    /// APPEND-ONLY tail; rides the format_version 25 bump. EMPTY ⇒ byte-identical.
    #[serde(default)]
    file_directed_stmts: std::collections::BTreeSet<u32>,
    /// The synthesized declaration-initializer ProcIds, in INITIALIZATION order
    /// (§4.5.256/257). The engine runs these before arming anything — IEEE 1800 §6.21's
    /// "before any initial or always block starts" — in an order that is NOT the order
    /// vita creates the processes in (a child instance's initializers precede its
    /// parent's, which the pass structure cannot express). Without this a STAGED run arms
    /// them as ordinary t0 processes: a parent `initial` reading `u.some_string` sees the
    /// empty default, and `reg clk = 0;` hands `always @clk` a spurious edge. APPEND-ONLY
    /// tail; rides the format_version 26 bump. EMPTY ⇒ no declaration initializers.
    #[serde(default)]
    init_procs: Vec<u32>,
    /// #10: StmtId → (file, line, col, byte range, instance) for severity
    /// statements, resolved at ELABORATE time (velab holds the source map since
    /// v28; the engine's IR is span-free, so vrun can only REPLAY this record).
    /// Without it a STAGED `$fatal`/`$error`/`$warning`/`$info` (and a
    /// `unique`/`priority` violation / deferred assert) silently prints
    /// location-less while the one-shot run prints `file:line:col [in path]`.
    /// APPEND-ONLY tail; rides the format_version 29 bump. EMPTY when no
    /// severity tasks (or no resolver) ⇒ byte-identical.
    #[serde(default)]
    severity_locs: sim_engine::SeverityLocTable,
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
            func_names: sc.func_names.clone(),
            real_elem_dyn_nets: sc.real_elem_dyn_nets.clone(),
            string_elem_dyn_nets: sc.string_elem_dyn_nets.clone(),
            net_decl_ranges: sc.net_decl_ranges.clone(),
            file_directed_stmts: sc.file_directed_stmts.clone(),
            init_procs: sc.init_procs.clone(),
            severity_locs: sc.severity_locs.clone(),
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
    top_params: Vec<(String, String)>,
    /// Runtime plusargs (v7): every bare `+...` arg that is not a
    /// `+define+`/`+incdir+` directive, leading '+' stripped, command-line
    /// order preserved ($test/$value$plusargs search order). vita/vrun only —
    /// the compile applets reject them loud.
    plusargs: Vec<String>,
    /// `--obs-dir <D>` (G2 OBS-1a): directory for the run manifest + result
    /// ledger. `None` ⇒ no obs rail. Out-of-band; one-shot `vita` only for v1.
    obs_dir: Option<String>,
    /// `--obs-procs` / `--obs-procs-time` (R14, ROADMAP §3 ⑭): per-body
    /// evaluation counts in `run.json`'s `processes` object, optionally with
    /// cumulative wall-clock. `None` ⇒ no profile (the counters are not even
    /// allocated). Requires `--obs-dir`; one-shot `vita` only, like the rest of
    /// the rail.
    proc_profile: Option<sim_engine::ProcProfileCfg>,
    /// `--hier-tree <path>` / `--inst-paths <path>`: design-structure exports (module
    /// hierarchy tree / full instance-path list). `None` ⇒ not requested.
    hier_tree: Option<String>,
    inst_paths: Option<String>,
    /// `--probe <path>` (OBS-2, repeatable): net names to trace into `trace.jsonl`.
    probes: Vec<String>,
    /// `--probe-file <F>` (OBS-2): file of probe paths, one per line.
    probe_file: Option<String>,
    /// `--backend <interp|vm|native>`: process-body executor. `None` ⇒ `native`, the
    /// default since Phase B1; `interp`/`vm` are DEBUG knobs for bisecting against a
    /// second implementation, and are absent without the `oracle` feature. Simulate-side
    /// only (`vita`/`vrun`) — it changes nothing an artifact records, so `vcmp`/`velab`
    /// reject it.
    backend: Option<Backend>,
    /// W-FLIST-OVERRIDE events recorded during arg parsing (knob, old, new) —
    /// replayed through the gated sink by [`emit_flist_overrides`].
    overrides: Vec<(String, String, String)>,
}
