//! split part of `cli` (mechanical move).

use super::*;

/// Build the preprocessor options a `VitaOpts` describes (`-I`/`-D` surface).
pub(crate) fn pre_opts_of(opts: &VitaOpts) -> hdl_preprocess::PreOpts {
    hdl_preprocess::PreOpts {
        incdirs: opts.incdirs.iter().map(std::path::PathBuf::from).collect(),
        cli_defines: opts.defines.clone(),
        ..hdl_preprocess::PreOpts::default()
    }
}

/// Split a `NAME[=VAL]` define token (empty VAL = definedness only).
pub(crate) fn split_define(tok: &str) -> (String, String) {
    match tok.split_once('=') {
        Some((n, v)) => (n.to_string(), v.to_string()),
        None => (tok.to_string(), String::new()),
    }
}

/// P4-T1 thread-count resolution: explicit flag > `VITA_THREADS` env > auto
/// (`min(available_parallelism, 8)`). Clamped to ≥1. The count never changes
/// output bytes — only wall-clock — so "auto" is safe as the default.
pub(crate) fn resolve_threads(flag: Option<u32>) -> u32 {
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

/// Open the `--log` tee writer a `VitaOpts` describes (`-` = stderr, vvp `-l -`
/// parity; default overwrite, `--log-append` accumulates). An unopenable path
/// is a loud CLI/usage error — never a silent no-log run.
pub(crate) fn open_log(opts: &VitaOpts) -> Result<Option<Box<dyn Write>>, i32> {
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

// The `-v` echo moved to `echo.rs` (doc-13): it now replays the whole resolved
// invocation, not just the define/incdir sets.

/// Map a byte offset into `src` to a 1-based `(line, col)`. Columns count
/// Unicode scalar values from the last newline (good enough for v1 caret-less
/// reporting; the real side-table bridge lives in `vita-log`).
///
/// Retained per preprocess-spec §4.3: line/col resolution now flows through
/// `hdl_preprocess::SourceMap` (which carries a byte-identical copy of this
/// function), so this is currently unreferenced. It is kept as the reference
/// the SourceMap copy must agree with byte-for-byte.
#[allow(dead_code)]
pub(crate) fn byte_to_line_col(src: &str, byte: usize) -> (u32, u32) {
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

/// A [`diag::SpanResolver`] over the preprocessor's `SourceMap`, so elaborate-time
/// diagnostics resolve to the original `file:line:col` the same way lex/parse ones do.
pub(crate) struct MapResolver<'a>(pub(crate) &'a hdl_preprocess::SourceMap);

impl diag::SpanResolver for MapResolver<'_> {
    fn resolve(&self, lo: u32, hi: u32) -> SourceLoc {
        self.0.resolve_span(lo as usize, hi as usize)
    }
}

/// Build a `SourceLoc` for the half-open expanded-byte range `[lo, hi)` by
/// resolving it through the preprocessor's `SourceMap` back to original positions.
pub(crate) fn loc_from_span(map: &hdl_preprocess::SourceMap, lo: usize, hi: usize) -> SourceLoc {
    map.resolve_span(lo, hi)
}

/// Emit a front-end (lex/parse) diagnostic with a resolved location.
pub(crate) fn emit_frontend_error(
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
    frontend_sources_to_unit_pre_with_includes(
        &[(file.to_string(), text.to_string())],
        sink,
        pre_opts,
    )
}

/// [`frontend_text_to_unit_pre`] over MULTIPLE command-line sources (G12): each file
/// keeps its own name + local line in diagnostics via the multi-file SourceMap,
/// instead of the old pre-concatenation that named `sources[0]` with a global line.
pub fn frontend_sources_to_unit_pre(
    sources: &[(String, String)],
    sink: &dyn LogSink,
    pre_opts: &hdl_preprocess::PreOpts,
) -> Option<(hdl_ast::SourceUnit, hdl_preprocess::ResolvedTimescales)> {
    frontend_sources_to_unit_pre_with_includes(sources, sink, pre_opts).map(|(u, rt, _)| (u, rt))
}

/// Multi-source twin of [`frontend_text_to_unit_pre_with_includes`]: preprocess all
/// `(name, text)` sources as ONE compilation unit (per-file SourceMap), then the
/// shared lex/parse/timescale path. `\`include` search is relative to the first
/// source's directory (matches the old single-buffer behavior).
pub fn frontend_sources_to_unit_pre_with_includes(
    sources: &[(String, String)],
    sink: &dyn LogSink,
    pre_opts: &hdl_preprocess::PreOpts,
) -> Option<FrontendUnit> {
    frontend_sources_mapped(sources, sink, pre_opts).map(|(u, _)| u)
}

/// [`frontend_sources_to_unit_pre_with_includes`] keeping the `SourceMap`, so the
/// one-shot driver can locate elaborate-time diagnostics.
pub fn frontend_sources_mapped(
    sources: &[(String, String)],
    sink: &dyn LogSink,
    pre_opts: &hdl_preprocess::PreOpts,
) -> Option<(FrontendUnit, hdl_preprocess::SourceMap)> {
    // ── preprocess ─────────────────────────────────────────────────────────
    // raw sources -> expanded text + multi-file SourceMap. The expanded text (one
    // buffer) is what the lexer and parser consume; spans they produce index the
    // expanded buffer and resolve back to the correct ORIGINAL file via `pp.map`.
    let first_name = sources.first().map(|(n, _)| n.as_str()).unwrap_or("");
    let base_dir = std::path::Path::new(first_name)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let pp = hdl_preprocess::preprocess_sources(base_dir, sources, pre_opts);
    frontend_pp_to_unit_mapped(pp, sink)
}

/// Post-preprocess front-end shared by the single-file and multi-source entry points:
/// consume a `PpResult` (expanded text + SourceMap) → lex → parse → resolve module
/// timescales, plus the `\`include` closure. Diagnostics resolve through `pp.map` to
/// the correct per-file name + local line, and the map is RETURNED so the caller can
/// build a [`MapResolver`] and locate elaborate-time diagnostics too.
pub(crate) fn frontend_pp_to_unit_mapped(
    pp: hdl_preprocess::PpResult,
    sink: &dyn LogSink,
) -> Option<(FrontendUnit, hdl_preprocess::SourceMap)> {
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
    let Some(mut unit) = unit else {
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
    // R28 §3.5: the same offset resolution answers `` `default_nettype ``. The parser
    // wrote the IEEE default (`wire`) into every `ModuleDecl`; stamp the real policy on
    // now, while the region offsets and the module spans still share one coordinate
    // space. It rides the AST from here, so the staged `.vu` carries it too.
    let nt = hdl_preprocess::resolve_module_nettype(&modules, &pp.nettype_none);
    drop(modules); // release the borrow of `unit` before moving it
    for it in &mut unit.items {
        if let hdl_ast::TopItem::Module(m) = it {
            m.nettype_none = nt.get(&m.name.name).copied().unwrap_or(false);
        }
    }
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
    Some(((unit, rt, includes), pp.map))
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

/// Render the design hierarchy as an indented tree keyed by MODULE name (`--hier-tree`),
/// top module at the root: `<instance> : <module>`, two spaces per level. Children are
/// the instances whose `parent` is this index; order = deterministic elaboration order.
pub(crate) fn render_hier_tree(insts: &[elaborate::InstanceInfo]) -> String {
    // The leaf scope segment of the full dotted path (`top.u_cpu.u_alu` → `u_alu`); the
    // top instance's path is its own name.
    fn leaf(path: &str) -> &str {
        path.rsplit('.').next().unwrap_or(path)
    }
    // Iterative pre-order walk (stack of (idx, depth)) so a deep hierarchy cannot
    // overflow the process stack. Children are pushed in REVERSE so they emit in
    // ascending elaboration order.
    fn walk(insts: &[elaborate::InstanceInfo], root: usize, out: &mut String) {
        let mut stack = vec![(root, 0usize)];
        while let Some((idx, depth)) = stack.pop() {
            let inf = &insts[idx];
            for _ in 0..depth {
                out.push_str("  ");
            }
            out.push_str(&format!("{} : {}\n", leaf(&inf.path), inf.module));
            let children: Vec<usize> = insts
                .iter()
                .enumerate()
                .filter(|(_, c)| c.parent == Some(idx as u32))
                .map(|(j, _)| j)
                .collect();
            for &j in children.iter().rev() {
                stack.push((j, depth + 1));
            }
        }
    }
    let mut out = String::new();
    for (i, inf) in insts.iter().enumerate() {
        if inf.parent.is_none() {
            walk(insts, i, &mut out);
        }
    }
    out
}

/// Render every instance's full dotted path from the top, one per line (`--inst-paths`) —
/// copy/paste-ready for scope-setting / signal-force control. VCD-`$scope`-consistent
/// (arrayed / generate segments appear verbatim). Order = elaboration order.
pub(crate) fn render_inst_paths(insts: &[elaborate::InstanceInfo]) -> String {
    let mut out = String::new();
    for inf in insts {
        out.push_str(&inf.path);
        out.push('\n');
    }
    out
}

/// Core: run the `vita` one-shot pipeline over already-read source `text`
/// (`file` is the display name used in diagnostics). Returns the process exit
/// code. This is the unit-test entry point — it never reads argv or files and
/// never calls `std::process::exit`.
pub fn run_vita_str(file: &str, text: &str, opts: &VitaOpts) -> i32 {
    run_vita_sources(&[(file.to_string(), text.to_string())], opts)
}

/// [`run_vita_str`] over MULTIPLE command-line sources (G12) — preserves per-file
/// name + line in diagnostics. `sources` = (display-name, text) in command order.
pub fn run_vita_sources(sources: &[(String, String)], opts: &VitaOpts) -> i32 {
    let log = match open_log(opts) {
        Ok(l) => l,
        Err(c) => return c,
    };
    let inner = StderrSink::with_output(opts.verbosity.unwrap_or(1), log);
    let sink = vita_log::GatedSink::new(&inner, opts.gate.clone());
    emit_flist_overrides(&sink, &opts.overrides);
    if inner.verbose() {
        let names: Vec<String> = sources.iter().map(|(n, _)| n.clone()).collect();
        echo::echo_effective_invocation(
            &sink,
            &names,
            opts.vcd_path_override.as_deref(),
            opts,
            &[],
        );
    }
    let code = run_vita_str_gated(sources, opts, &inner, &sink);
    // doc-13: the counts summary epilogue is the unsuppressible end-of-stage
    // spine — printed on EVERY pipeline run (not on --help/--version/usage).
    inner.epilogue();
    code
}

pub(crate) fn run_vita_str_gated(
    sources: &[(String, String)],
    opts: &VitaOpts,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    // G2 OBS-1a wall-clock baseline (isolated, non-deterministic — only used if
    // `--obs-dir` is set; the Instant read is cheap and never affects output).
    let obs_start = std::time::Instant::now();
    // ── preprocess → lex → parse (shared front-end) ─────────────────────────
    let Some(((unit, rt, _inc), smap)) = frontend_sources_mapped(sources, sink, &pre_opts_of(opts))
    else {
        return EXIT_USER_ERROR;
    };

    // ── elaborate ──────────────────────────────────────────────────────────
    // The elaborator emits its own diagnostics through `sink`; `None` ⇒ a hard
    // elaboration error was reported. `elaborate_with_timescale` also yields the
    // fork-join, net-name, and per-process time-multiplier side tables threaded into
    // `SimOpts`; the timescale env scales `#delay`/`$time`/`$realtime`.
    // r17 one-shot `--top`: when the user pins root(s), pass them through so the
    // elaborator overrides auto-top; empty ⇒ `None` ⇒ auto-top (unchanged).
    let root_sel: Option<&[String]> = if opts.tops.is_empty() {
        None
    } else {
        Some(&opts.tops)
    };
    // §4.5.249: the one-shot flow still holds the preprocessor's SourceMap, so give
    // elaborate-time diagnostics the same `file:line:col` lex/parse ones get. Without
    // it, N identical E3009s in one run are indistinguishable and the declaration that
    // caused them cannot be found at all.
    let resolver = MapResolver(&smap);
    let (ir, sc) = elaborate::elaborate_located(
        &unit,
        sink,
        &rt.unit_exp,
        &rt.prec_exp,
        rt.global_prec_exp,
        root_sel,
        Some(&resolver),
    );
    let Some(ir) = ir else {
        return EXIT_USER_ERROR;
    };

    // Design-structure exports (one-shot): the elaborated hierarchy → a module tree
    // (`--hier-tree`) and/or a full instance-path list (`--inst-paths`). Out-of-band; a
    // write failure is loud but does NOT change the exit code (the elaboration itself
    // succeeded). Staged `velab` support is a follow-on.
    if let Some(path) = &opts.hier_tree {
        if let Err(e) = std::fs::write(path, render_hier_tree(&sc.instances_info)) {
            eprintln!(
                "error[{}]: cannot write --hier-tree '{path}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
        }
    }
    if let Some(path) = &opts.inst_paths {
        if let Err(e) = std::fs::write(path, render_inst_paths(&sc.instances_info)) {
            eprintln!(
                "error[{}]: cannot write --inst-paths '{path}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
        }
    }

    // ── simulate ────────────────────────────────────────────────────────────
    // OBS-2: resolve --probe paths → net ids against the elaborated net_names
    // (loud on an unresolved path / --probe without --obs-dir) BEFORE net_names is
    // moved into SimOpts below.
    let probed_nets = match resolve_probes(
        &opts.probes,
        opts.probe_file.as_deref(),
        opts.obs_dir.as_deref(),
        &sc.net_names,
        &ir.nets,
    ) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let sim_opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        probed_nets,
        proc_multipliers: sc.proc_multipliers,
        proc_prec_mults: sc.proc_prec_mults,
        severities: sc.severities,
        // §21.3.2 %t/$timeformat: the call-site table + the precision exponent
        // %t scales against (one-shot path; empty/−9 ⇒ byte-identical).
        timeformat_stmts: sc.timeformat_stmts,
        stage_stmts: sc.stage_stmts,
        handle_copy_stmts: sc.handle_copy_stmts,
        queue_slice_stmts: sc.queue_slice_stmts,
        global_prec_exp: rt.global_prec_exp,
        radixes: sc.radixes,
        assign_ranks: sc.assign_ranks,
        queue_bounds: sc.queue_bounds,
        proc_scopes: sc.proc_scopes,
        // OBS-1b: the coverage manifest → end-of-run `coverage.json` (empty for
        // covergroup-free designs → no coverage payload). One-shot path only.
        coverage_manifest: sc.coverage_manifest,
        net_dims: sc.net_dims,
        net_decl_ranges: sc.net_decl_ranges,
        file_directed_stmts: sc.file_directed_stmts,
        init_procs: sc.init_procs,
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
        func_names: sc.func_names,
        task_calls_proc: sc.task_calls_proc,
        task_calls_func: sc.task_calls_func,
        // SVPART: 2-state nets coerce X/Z→0 on write (one-shot path only).
        two_state_nets: sc.two_state_nets,
        // N3 Phase 2 heterogeneous heap: real/string dyn-array element markers.
        real_elem_dyn_nets: sc.real_elem_dyn_nets,
        string_elem_dyn_nets: sc.string_elem_dyn_nets,
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
    let final_code = if code == EXIT_OK && inner.had_error_or_fatal() {
        EXIT_USER_ERROR
    } else {
        code
    };
    // G2 OBS-1a: emit the run manifest + result ledger AFTER the exit code is
    // final (so `status`/`exit_code` match exactly what the process returns).
    // Derived from the SAME `result` + diagnostic counts — no second source.
    if let Some(dir) = &opts.obs_dir {
        // Reconstruct the display name + concatenated source text for the OBS run
        // manifest (basename + blake3), byte-identical to the pre-G12 single buffer.
        let file = sources.first().map(|(n, _)| n.as_str()).unwrap_or("");
        let mut text = String::new();
        for (_, t) in sources {
            text.push_str(t);
            if !t.ends_with('\n') {
                text.push('\n');
            }
        }
        // The EFFECTIVE executor, in the `--backend` flag's vocabulary — the
        // same default resolution `sim_opts()` applied (`None` ⇒ the VM).
        let backend = match opts.backend.unwrap_or_default() {
            sim_engine::Backend::Interpreter => "interp",
            sim_engine::Backend::Bytecode => "vm",
        };
        emit_obs(
            dir,
            file,
            &text,
            &opts.plusargs,
            backend,
            &result,
            inner,
            final_code,
            obs_start,
        );
    }
    final_code
}

/// Build the OBS-1a `ObsRun` from the finished run and write `run.json` +
/// `results.jsonl` into `dir`. A filesystem failure is surfaced LOUD (a
/// silently-missing obs file would mislead the harness) but does NOT change the
/// simulation's exit code — the run itself succeeded/failed independently.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_obs(
    dir: &str,
    file: &str,
    text: &str,
    plusargs: &[String],
    backend: &'static str,
    result: &sim_engine::SimResult,
    inner: &StderrSink,
    final_code: i32,
    start: std::time::Instant,
) {
    let utc_unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // The BASENAME only (not the full path): a run from `/tmp/x/d.sv` and one
    // from `/home/ci/d.sv` of the same design must byte-diff clean (R-F1). The
    // content identity is the blake3, not the path.
    let source_name = std::path::Path::new(file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file);
    // `exit_class` MUST agree with the FINAL exit code (adversarial find): a
    // `-Werror`-promoted warning flips an engine-"ok" run to a failing
    // `final_code`, so deriving `exit_class` from the raw `SimResult` would emit
    // `"ok"` next to `exit_code:1`/`status:"FAIL"` — a self-contradicting log.
    // Derive it from `final_code` + the fatal count instead (verdict fields all
    // agree). `finish_reason` stays from the engine — it is DESCRIPTIVE (how the
    // sim stopped: it really did `$finish`), not the pass/fail verdict.
    let exit_class = if final_code == EXIT_OK {
        "ok"
    } else if inner.fatal_count() > 0 {
        "fatal"
    } else {
        "had_errors"
    };
    let run = obs::ObsRun {
        source_name,
        source_blake3: blake3::hash(text.as_bytes()).to_hex().to_string(),
        plusargs,
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        finish_reason: obs::finish_reason_str(result.finish_reason),
        exit_class,
        exit_code: final_code,
        sim_time: result.sim_time,
        errors: inner.error_count(),
        warnings: inner.warning_count(),
        fatals: inner.fatal_count(),
        status: if final_code == EXIT_OK {
            "PASS"
        } else {
            "FAIL"
        },
        backend,
        codegen: &result.codegen,
        native: &result.native,
        utc_unix_s,
        wall_s: start.elapsed().as_secs_f64(),
    };
    if let Err(e) = obs::write_run_dir(dir, &run) {
        eprintln!(
            "error[{}]: cannot write --obs-dir '{dir}': {e}",
            MsgCode::CliBadFlag.code_num()
        );
    }
    // OBS-1b: emit `coverage.json` when the design produced functional coverage
    // (≥1 covergroup instance). Absent ⇒ no covergroups (the file is simply not
    // written). A write failure is loud, like run.json (silent-missing misleads).
    if let Some(cov) = &result.coverage {
        if let Err(e) = obs::write_coverage_dir(dir, cov) {
            eprintln!(
                "error[{}]: cannot write coverage.json to '{dir}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
        }
    }
    // OBS-2: emit `trace.jsonl` when the run was probed (`--probe`). Absent ⇒ no
    // probe. A write failure is loud (a silently-missing trace misleads).
    if let Some(lines) = &result.trace {
        if let Err(e) = obs::write_trace_dir(dir, lines) {
            eprintln!(
                "error[{}]: cannot write trace.jsonl to '{dir}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
        }
    }
    // OBS-3: emit `stage.jsonl` when `+STAGE_TRACE` armed `$vita_stage` capture.
    if let Some(lines) = &result.stage {
        if let Err(e) = obs::write_stage_dir(dir, lines) {
            eprintln!(
                "error[{}]: cannot write stage.jsonl to '{dir}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
        }
    }
}

/// Map a finished `SimResult` to the doc-13 exit code. A clean `$finish`/quiescent
/// run with no error-or-fatal diagnostics is 0; anything else (`$fatal`, runtime
/// `$error`, delta-limit blowup) is a user/design error (1).
pub(crate) fn sim_exit_code(result: &sim_engine::SimResult) -> i32 {
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
pub(crate) fn lex_error_message(kind: hdl_lexer::LexErrorKind) -> &'static str {
    use hdl_lexer::LexErrorKind as K;
    match kind {
        K::UnexpectedChar => "unexpected character",
        K::UnterminatedString => "unterminated string literal",
        K::UnterminatedBlockComment => "unterminated block comment",
        K::EmptyEscapedIdent => "empty escaped identifier",
        K::LoneSigil => "stray `$` or backtick with no identifier body",
        K::UnterminatedAttribute => {
            "`(*` opens an attribute instance that is never closed by a `*)` \
             (an implicit sensitivity list is `@(*)` or `@*`)"
        }
    }
}

/// Run `vita` over one or more source files: read each file, then drive the pipeline.
/// Returns the process exit code.
///
/// File-read failures are CLI/usage errors (exit 3). Each file is registered as its
/// own SourceMap entry (G12) so multi-file diagnostics report the correct per-file
/// name + local line, instead of the FIRST file's name with a concat-global line.
pub fn run_vita(sources: &[String], opts: &VitaOpts) -> i32 {
    if sources.is_empty() {
        eprintln!(
            "error[{}]: no source files given",
            MsgCode::CliBadFlag.code_num()
        );
        return EXIT_CLI_ERROR;
    }
    let mut srcs: Vec<(String, String)> = Vec::with_capacity(sources.len());
    for path in sources {
        match std::fs::read_to_string(path) {
            Ok(s) => srcs.push((path.clone(), s)),
            Err(e) => {
                eprintln!(
                    "error[{}]: cannot read '{path}': {e}",
                    MsgCode::FlistNotFound.code_num()
                );
                return EXIT_CLI_ERROR;
            }
        }
    }
    run_vita_sources(&srcs, opts)
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

/// Restore the DEFAULT SIGPIPE disposition (Unix). The Rust runtime IGNOREs
/// SIGPIPE at startup, so a write to a pipe whose consumer has closed
/// (`vita design.sv | head`) returns EPIPE, which the `print!`/`println!`
/// machinery turns into a panic (`failed printing to stdout: Broken pipe`, exit
/// 101). Resetting to `SIG_DFL` makes the OS terminate the process on the broken
/// pipe (the conventional producer behaviour, exit 141) — quiet, not a panic.
/// Process-wide for the signal DISPOSITION (not the per-thread MASK): on Linux
/// the worker thread inherits SIG_DFL and dies on the broken-pipe write; on
/// macOS the spawned thread has SIGPIPE masked, so its writes see EPIPE (no
/// signal) — that case is handled by StderrSink's broken-pipe-safe
/// `out_write`/`err_write` (the §4.5.59 follow-on). No-op on Windows (no
/// SIGPIPE). A tiny FFI avoids pulling in `libc` for one call; `SIGPIPE` is 13
/// and `SIG_DFL` is 0 on every Unix target vita builds for (Linux/macOS).
#[cfg(unix)]
pub(crate) fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal(2)` with `SIG_DFL` only resets a signal's disposition to
    // the OS default; it allocates nothing and the ignored return is the prior
    // handler. This is the standard CLI idiom.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
pub(crate) fn restore_default_sigpipe() {}
