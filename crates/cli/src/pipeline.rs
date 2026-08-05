//! split part of `cli` (mechanical move).

use super::*;

/// The SHARED process-level driver every binary shim calls (`vita` and the
/// dev-only `separate-bins` shims `vcmp`/`velab`/`vrun`, doc-03) — SIGPIPE
/// reset + a big-stack worker thread around [`run`], then `process::exit`.
///
/// The pipeline contains user-depth-controlled recursion: the recursive-descent
/// parser, and recursive AST walks (`Clone`/`Drop`/elaborate) over deeply nested
/// expressions, sequences, and SVA property trees. The default MAIN-THREAD stack
/// is only ~1 MiB on Windows (vs ~8 MiB on Linux/macOS), so a pathologically deep
/// design overflows it and aborts (SIGABRT / `STATUS_STACK_OVERFLOW`) BEFORE a
/// depth-cap diagnostic (e.g. `SVA_SEQ_ALT_CAP`) can report cleanly. Run the whole
/// driver on a worker thread with a large explicit stack so depth caps produce a
/// clean diagnostic on EVERY OS — the same approach rustc/swc use. This is the
/// sole place the stack is sized; [`run`] stays in-thread for unit tests, which
/// keeps the work single-threaded and deterministic (just on a bigger stack).
pub fn driver_main() -> ! {
    restore_default_sigpipe();

    /// 256 MiB — virtual address space (lazily committed), generous headroom over
    /// every OS default. Matches the order of magnitude swc/other Rust compilers use.
    const STACK_SIZE: usize = 256 * 1024 * 1024;

    let argv: Vec<String> = std::env::args().collect();
    let code = std::thread::Builder::new()
        .name("vita-main".to_string())
        .stack_size(STACK_SIZE)
        .spawn(move || run(&argv))
        .expect("spawn vita worker thread")
        .join()
        // A panic in the worker has already printed its message via the default
        // hook; re-raise it on this thread so the process exits with the same
        // conventional panic code (101) as before — NOT a vita exit class
        // (1 user/design error, 2 stale/artifact-gate, 3 CLI error), which would
        // mislead callers. (`spawn` failure above is `.expect()` -> also 101: the
        // 256 MiB is lazily-committed virtual memory so the reservation is cheap
        // and failure is near-impossible; falling back to in-thread would just
        // re-introduce the very ~1 MiB overflow this wrapper removes.)
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    std::process::exit(code);
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
    //
    // The PRE-expansion argv is captured first: that is the line the Makefile
    // or wrapper script actually ran, and the `-v` echo replays it next to the
    // expanded result so the two can be compared (see [`Invocation`]).
    let inv = Invocation {
        // The REAL argv, not a reconstruction from `applet_name` + `args`: the
        // multicall subcommand form (`vita velab …`) would otherwise echo back
        // as `velab …`, which is not the line anyone ran. argv[0] is shown by
        // basename — the full interpreter path adds width and no information.
        argv: argv
            .first()
            .map(std::path::Path::new)
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .into_iter()
            .chain(argv.iter().skip(1).cloned())
            .collect(),
        cwd: std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        filelists: Vec::new(),
    };
    let (args, filelists) = match filelist::expand_argv(&args, &StderrSink::new()) {
        Ok(a) => a,
        Err(code) => return code,
    };
    let inv = Invocation { filelists, ..inv };
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
            if let Err(c) = reject_worklib_flags("vita", &io, false, false, true) {
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
                // One-shot `--top <UNIT>` (r17): pin the elaboration root(s) for a
                // deterministic single top, instead of relying on auto-top. Empty =
                // auto-top (unchanged default). Passed through to elaborate below.
                tops: io.tops.clone(),
                plusargs: io.plusargs,
                obs_dir: io.obs_dir,
                hier_tree: io.hier_tree,
                inst_paths: io.inst_paths,
                probes: io.probes,
                probe_file: io.probe_file,
                backend: io.backend,
                overrides: io.overrides.clone(),
                invocation: Some(inv),
            };
            run_vita(&io.pos, &opts)
        }
        Applet::Staged("vcmp") => dispatch_vcmp(&args, inv),
        Applet::Staged("velab") => dispatch_velab(&args, inv),
        Applet::Staged("vrun") => dispatch_vrun(&args, inv),
        Applet::Staged(other) => {
            eprintln!(
                "error[{}]: unknown staged applet '{other}'",
                MsgCode::CliBadFlag.code_num()
            );
            EXIT_CLI_ERROR
        }
    }
}

/// `vita explain <CODE>`: print the doc-15 entry for a mnemonic
/// (`E-ELAB-MULTIDRIVER`) or grep-number (`VITA-E3001`) form.
pub(crate) fn run_explain(args: &[String]) -> i32 {
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
pub(crate) fn print_help(applet: &str) {
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
            "Usage: vrun [-o <out.vcd|out.fst>] <in.velab>\n\n\
             Simulate a `.velab`, writing the waveform and RTL stdout.\n\
             A `-o` (or `$dumpfile`) path ending in `.fst` writes FST; else VCD."
        }
        _ => {
            "Usage: vita [-o <out.vcd|out.fst>] <sources>...\n\
             \x20      vita {vcmp|velab|vrun} [OPTIONS] ...\n\n\
             One-shot RTL simulation: preprocess -> lex -> parse -> elaborate ->\n\
             simulate -> waveform. A `-o`/`$dumpfile` path ending in `.fst`\n\
             writes the GTKWave/Surfer FST format; any other extension writes VCD.\n\
             The staged subcommands split the same pipeline.\n\
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
         --threads, -j <N>     waveform-writer thread budget; N>=2 moves VCD writing off\n                        \
                       the sim thread. Simulation itself is single-threaded, so\n                        \
                       this does NOT speed up a run with no waveform dump.\n  \
         --backend <interp|vm|native> (vita/vrun) process-body executor. 'vm' (default) runs\n                        \
                       suspend-free bodies on the bytecode VM and interprets the rest;\n                        \
                       'interp' forces the reference semantics, for bisecting; 'native'\n                        \
                       selects the tier-3 backend, which runs a design when nothing in it\n                        \
                       is outside today's tier-3 subset -- no fork, no subroutine call,\n                        \
                       no `final`, no $monitor/VCD-task refusal -- and every\n                        \
                       continuous-assign family (zero-delay, delayed, multi-driven,\n                        \
                       wired) runs --\n                        \
                       and falls back to the VM otherwise (run.json reports 'backend' vs\n                        \
                       'backend_requested', and 'native.refused' names the row).\n                        \
                       Output is byte-identical whichever you pick -- this only moves\n                        \
                       wall-clock (measured\n                        \
                       1.4x on a real design, up to 2.8x on expression-heavy RTL, ~1.0x\n                        \
                       when the run is clock/scheduler-bound).\n  \
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
pub(crate) fn write_artifact_atomic(out: &str, bytes: &[u8]) -> std::io::Result<()> {
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
pub(crate) fn emit_artifact_error(sink: &dyn LogSink, e: &vita_artifact::ArtifactError) -> i32 {
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
pub(crate) fn read_artifact_bytes(path: &str) -> Result<Vec<u8>, i32> {
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
pub(crate) fn default_out(input: &str, ext: &str) -> String {
    let p = std::path::Path::new(input);
    p.with_extension(ext).to_string_lossy().into_owned()
}

/// True iff two path strings denote the same file. Canonicalizes when BOTH paths
/// already exist (handles `./a.sv` vs `a.sv`, symlinks, `..`); otherwise falls
/// back to a raw string compare (the output usually does not exist yet). Never
/// panics.
pub(crate) fn same_path(a: &str, b: &str) -> bool {
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
pub(crate) fn reject_out_clobbers_input(inputs: &[String], out: &str) -> Result<(), i32> {
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
/// RECORDED since 2026-06-11 for provenance/forensics. The live RULE-V re-hash
/// gate (`E-ART-STALE-UPSTREAM`) IS implemented on the worklib vrun path (it
/// re-hashes consumed lib manifests / CU blobs / raw sources vs the recorded
/// hashes — see the `consumed.libs` auto-gate below); the header's OWN
/// `consumed`/`worklib_manifest_hash` fields stay vestigial placeholders (the
/// live data rides the append-only trailer — frozen header shape, doc-14 §1).
/// `verify_header` gates the primary staleness via `schema_hash` +
/// `format_version`.
pub(crate) fn artifact_header(
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
    emit_flist_overrides(&sink, &opts.overrides);
    if inner.verbose() {
        let work: Vec<String> = opts.work.iter().map(|(n, d)| format!("{n}={d}")).collect();
        echo::echo_effective_invocation(&sink, sources, out, opts, &[("work", work)]);
    }
    let code = run_vcmp_gated(sources, out, opts, &inner, &sink);
    inner.epilogue();
    code
}

pub(crate) fn run_vcmp_gated(
    sources: &[String],
    out: Option<&str>,
    opts: &VitaOpts,
    inner: &StderrSink,
    sink: &vita_log::GatedSink,
) -> i32 {
    // read+concat (mirrors run_vita): read error → exit 3. Per-source raw
    // digests feed the worklib manifest (RULE-V staleness keys).
    let mut text = String::new();
    let mut srcs: Vec<(String, String)> = Vec::with_capacity(sources.len());
    let mut src_digests: Vec<(String, [u8; 32])> = Vec::new();
    for path in sources {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                src_digests.push((path.clone(), *blake3::hash(s.as_bytes()).as_bytes()));
                text.push_str(&s);
                if !s.ends_with('\n') {
                    text.push('\n');
                }
                srcs.push((path.clone(), s));
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

    // preprocess → lex → parse through the SAME shared front-end the one-shot uses.
    // Each file is its own SourceMap entry (G12) so diagnostics keep per-file name +
    // line; `text` (the concatenation) is retained only for the RULE-V composite
    // digest below, keeping worklib staleness keys byte-stable.
    let Some((unit, rt, includes)) =
        frontend_sources_to_unit_pre_with_includes(&srcs, sink, &pre_opts_of(opts))
    else {
        return EXIT_USER_ERROR;
    };

    // ── write `.vu` body = postcard(SourceUnit) ++ postcard((unit_exp map, global
    //    precision, prec_exp map)) [v22]. The resolved timescale rides after the hashed SourceUnit frame
    //    (the gate covers the type, not these bytes) so `velab` can elaborate the
    //    staged path with the same scaling as the one-shot path. ──
    // `-Werror`: a promoted warning is an Error — the stage fails and writes
    // NO artifact (matching a real compile error).
    if inner.had_error_or_fatal() {
        return EXIT_USER_ERROR;
    }
    let mut body = postcard::to_stdvec(&unit).expect("SourceUnit postcard encode infallible");
    body.extend_from_slice(
        &postcard::to_stdvec(&(rt.unit_exp, rt.global_prec_exp, rt.prec_exp))
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
                hdl_ast::TopItem::Import(_)
                | hdl_ast::TopItem::Bind(_)
                | hdl_ast::TopItem::Error(_) => None,
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
    emit_flist_overrides(&sink, &opts.overrides);
    if inner.verbose() {
        let src = [vu_path.to_string()];
        echo::echo_effective_invocation(&sink, &src, Some(out), opts, &[]);
    }
    let code = run_velab_gated(vu_path, out, opts, &inner, &sink);
    inner.epilogue();
    code
}

pub(crate) fn run_velab_gated(
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

    let (unit, unit_exp, prec_exp, global_prec_exp) = match decode_vu_unit(&bytes, sink) {
        Ok(x) => x,
        Err(code) => return code,
    };

    // ── elaborate (with the staged timescale env; `--top` overrides roots) ──
    let roots: Option<&[String]> = if opts.tops.is_empty() {
        None
    } else {
        Some(&opts.tops)
    };
    let (ir, sc) = elaborate::elaborate_with_timescale_prec_roots(
        &unit,
        sink,
        &unit_exp,
        &prec_exp,
        global_prec_exp,
        roots,
    );
    let Some(ir) = ir else {
        return EXIT_USER_ERROR; // elab error already emitted
    };
    if let Err(c) = reject_stage_staged(&sc) {
        return c;
    }

    // ── write `.velab` body = postcard(SimIr) ++ postcard(ForkModeTable) ++
    //    postcard(NetNameTable) ++ postcard((proc_multipliers, global_prec_exp, proc_prec_mults)) ++
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
pub(crate) fn decode_vu_unit(bytes: &[u8], sink: &dyn LogSink) -> Result<VuUnitEnv, i32> {
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
    type TsEnv = (
        std::collections::BTreeMap<String, i8>,
        i8,
        std::collections::BTreeMap<String, i8>,
    );
    let (unit_exp, global_prec_exp, prec_exp): TsEnv = if vu_rest.is_empty() {
        (
            std::collections::BTreeMap::new(),
            -9,
            std::collections::BTreeMap::new(),
        )
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
    Ok((unit, unit_exp, prec_exp, global_prec_exp))
}

/// Re-read `path` and, IFF its bytes hash to `want`, return its verified
/// `(mtime, size)` stamp. Any read/hash/metadata/time failure → `None` (the
/// entry will always be rehashed at vrun — never a silent skip).
pub(crate) fn stamp_verified(path: &std::path::Path, want: &[u8; 32]) -> Option<FileStamp> {
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

pub(crate) fn check_fresh(
    path: &std::path::Path,
    want: &[u8; 32],
    stamp: Option<FileStamp>,
) -> Freshness {
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
pub(crate) fn write_velab_file(
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
        &postcard::to_stdvec(&(&sc.proc_multipliers, global_prec_exp, &sc.proc_prec_mults))
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
    emit_flist_overrides(&sink, &opts.overrides);
    if inner.verbose() {
        let l: Vec<String> = libs.iter().map(|(n, d)| format!("{n}={d}")).collect();
        // Library mode has no positional source — the `-L` set IS the input.
        echo::echo_effective_invocation(&sink, &[], Some(out), opts, &[("libs", l)]);
    }
    let code = run_velab_lib_gated(libs, tops, out, &inner, &sink);
    inner.epilogue();
    code
}
