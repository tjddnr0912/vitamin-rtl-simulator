//! split part of `cli` (mechanical move).

use super::*;

pub(crate) fn parse_io_args(args: &[String]) -> Result<IoArgs, i32> {
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
    let mut top_params: Vec<(String, String)> = Vec::new();
    let mut plusargs: Vec<String> = Vec::new();
    let mut obs_dir: Option<String> = None;
    let mut hier_tree: Option<String> = None;
    let mut inst_paths: Option<String> = None;
    let mut probes: Vec<String> = Vec::new();
    let mut probe_file: Option<String> = None;
    let mut backend: Option<sim_engine::Backend> = None;
    let mut overrides: Vec<(String, String, String)> = Vec::new();
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
                    record_override(&mut overrides, "-o", prev, v);
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
                    record_override(
                        &mut overrides,
                        "--threads",
                        &prev.to_string(),
                        &n.to_string(),
                    );
                }
                threads = Some(n.max(1));
                i += 2;
            }
            // Process-body executor. Output is byte-identical for either value
            // (locked by the P5 gate, `sim-engine/tests/backend_equiv.rs`); the
            // value only moves wall-clock, like `--threads`.
            "--backend" => {
                // ⚠️ B2': WITHOUT the `oracle` feature the two spellings do not
                // vanish — they become a LOUD rejection. Dropping them from the
                // match would make `--backend vm` fall into the `_ => None` arm
                // and report "unknown value", which is a different and worse
                // message: the value is known, it is this BUILD that does not
                // carry that executor. Silently accepting it would be worse
                // still — the user would get native and be told nothing.
                let parsed = match args.get(i + 1).map(String::as_str) {
                    #[cfg(feature = "oracle")]
                    Some("interp") | Some("interpreter") => Some(sim_engine::Backend::Interpreter),
                    #[cfg(feature = "oracle")]
                    Some("vm") | Some("bytecode") => Some(sim_engine::Backend::Bytecode),
                    #[cfg(not(feature = "oracle"))]
                    Some("interp") | Some("interpreter") | Some("vm") | Some("bytecode") => {
                        // The existing `--backend` flag code, not a new one: this
                        // is a bad VALUE for this build, which is what
                        // `CliBadFlag` already means. Inventing a code here would
                        // also break the MsgCode↔doc bijection gate, since a raw
                        // `eprintln!` never reaches the enum.
                        eprintln!(
                            "error[{}]: '--backend' takes only 'native' in this build — \
                             the oracle executors ('vm', 'interp') are compiled out. \
                             Rebuild with the `oracle` feature to select them.",
                            MsgCode::CliBadFlag.code_num()
                        );
                        return Err(EXIT_CLI_ERROR);
                    }
                    Some("native") => Some(sim_engine::Backend::Native),
                    _ => None,
                };
                let Some(b) = parsed else {
                    eprintln!(
                        "error[{}]: '--backend' takes 'vm' (default, the bytecode VM), \
                         'interp' (the reference tree-walking semantics, for bisecting), \
                         or 'native' (the tier-3 backend — it runs a design only when \
                         nothing in it is outside the tier-3 subset, and falls back to \
                         the VM otherwise; --obs-dir run.json reports which executor \
                         actually ran plus this design's native verdict and refusal) \
                         — same output whichever you pick, this only moves wall-clock",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if let Some(prev) = backend {
                    record_override(
                        &mut overrides,
                        "--backend",
                        backend_name(prev),
                        backend_name(b),
                    );
                }
                backend = Some(b);
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
                    record_override(
                        &mut overrides,
                        "--timeout",
                        &prev.to_string(),
                        &n.to_string(),
                    );
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
                    record_override(&mut overrides, "--upstream", prev, v);
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
                    record_override(&mut overrides, "--work", prev, v);
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
                    record_override(&mut overrides, "--workdir", prev, v);
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
            "-G" | "--param" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '{}' needs NAME=VALUE",
                        MsgCode::CliBadFlag.code_num(),
                        args[i]
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                match v.split_once('=') {
                    Some((n, val)) => top_params.push((n.to_string(), val.to_string())),
                    None => {
                        eprintln!(
                            "error[{}]: '{} {v}' needs NAME=VALUE",
                            MsgCode::CliBadFlag.code_num(),
                            args[i]
                        );
                        return Err(EXIT_CLI_ERROR);
                    }
                }
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
                    record_override(&mut overrides, "--log", prev, v);
                }
                log = Some(v.clone());
                i += 2;
            }
            "--log-append" => {
                log_append = true;
                i += 1;
            }
            // G2 OBS-1a: directory for the run manifest + result ledger.
            "--obs-dir" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--obs-dir' needs a directory",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                // Reject an empty value: `Path::new("").join(...)` is a relative
                // path, so `--obs-dir ""` (e.g. a `--obs-dir $UNSET` slip) would
                // silently write run.json/results.jsonl into the CWD.
                if v.is_empty() {
                    eprintln!(
                        "error[{}]: '--obs-dir' needs a non-empty directory",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                }
                if let Some(prev) = &obs_dir {
                    record_override(&mut overrides, "--obs-dir", prev, v);
                }
                obs_dir = Some(v.clone());
                i += 2;
            }
            "--hier-tree" => {
                let Some(v) = args.get(i + 1).filter(|v| !v.is_empty()) else {
                    eprintln!(
                        "error[{}]: '--hier-tree' needs a non-empty path",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                hier_tree = Some(v.clone());
                i += 2;
            }
            "--inst-paths" => {
                let Some(v) = args.get(i + 1).filter(|v| !v.is_empty()) else {
                    eprintln!(
                        "error[{}]: '--inst-paths' needs a non-empty path",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                inst_paths = Some(v.clone());
                i += 2;
            }
            "--probe" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--probe' needs a hierarchical net path",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                if v.is_empty() {
                    eprintln!(
                        "error[{}]: '--probe' needs a non-empty net path",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                }
                probes.push(v.clone());
                i += 2;
            }
            "--probe-file" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!(
                        "error[{}]: '--probe-file' needs a file path",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                };
                probe_file = Some(v.clone());
                i += 2;
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
            // ATTACHED-VALUE `-D<NAME>[=<VAL>]` / `-I<dir>`, the form every other tool
            // takes. vita accepted only the separated `-D NAME` and the `+define+`
            // plus-arg, so a command line copied from an iverilog or VCS flow died on
            // `unknown flag '-DFAST_MODE'` — a compatibility trap, not a real
            // disagreement about the flag. Placed AHEAD of the `-`-prefixed catch-all
            // and below every exact-match arm, so `-D`/`-I` alone still take the next
            // argv element and `-Wno-…` still reaches the gate parser (neither `-D` nor
            // `-I` is a prefix of any flag vita already knows).
            s if s.starts_with("-D") && s.len() > 2 => {
                defines.push(split_define(&s[2..]));
                i += 1;
            }
            s if s.starts_with("-I") && s.len() > 2 => {
                incdirs.push(s[2..].to_string());
                i += 1;
            }
            // `-GNAME=VALUE`, the attached-value spelling Verilator uses. Kept next to
            // `-D`/`-I` because it is the same shape; `--param NAME=VALUE` is the
            // separated spelling.
            s if s.starts_with("-G") && s.len() > 2 => {
                match s[2..].split_once('=') {
                    Some((n, val)) => top_params.push((n.to_string(), val.to_string())),
                    None => {
                        eprintln!(
                            "error[{}]: '-G{}' needs NAME=VALUE",
                            MsgCode::CliBadFlag.code_num(),
                            &s[2..]
                        );
                        return Err(EXIT_CLI_ERROR);
                    }
                }
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
        top_params,
        plusargs,
        obs_dir,
        hier_tree,
        inst_paths,
        probes,
        probe_file,
        backend,
        overrides,
    })
}

/// The spelling `--backend` accepts, for override records and the `-v` echo.
pub(crate) fn backend_name(b: sim_engine::Backend) -> &'static str {
    match b {
        #[cfg(feature = "oracle")]
        sim_engine::Backend::Interpreter => "interp",
        #[cfg(feature = "oracle")]
        sim_engine::Backend::Bytecode => "vm",
        sim_engine::Backend::Native => "native",
    }
}

/// `--dump-filelist` (doc-14 §3.1 debugging surface): print the EFFECTIVE
/// post-expansion inputs — sources in argv order, then defines, then incdirs
/// — and exit 0 without compiling. Deterministic (no sorting, no resolution
/// beyond what the expansion itself did), so CI can diff two trees' effective
/// inputs directly.
pub(crate) fn run_dump_filelist(io: &IoArgs) -> i32 {
    // The dry-run surfaces override warnings too — through the same gate as the
    // real pipeline (no raw eprintln bypass).
    let inner = StderrSink::with_output(io.verbosity.unwrap_or(1), None);
    let sink = vita_log::GatedSink::new(&inner, io.gate.clone());
    emit_flist_overrides(&sink, &io.overrides);
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
pub(crate) fn reject_preprocess_buckets(stage: &str, io: &IoArgs) -> Result<(), i32> {
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
pub(crate) fn reject_plusargs(stage: &str, io: &IoArgs) -> Result<(), i32> {
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

/// `--obs-dir` (G2 OBS-1a) is honored ONLY by the one-shot `vita` applet in v1.
/// Loud-reject it on the staged applets rather than silently accept-and-drop it
/// (a silent no-op on `vrun`, the simulate stage, would mislead a harness — the
/// staged obs rail is an OBS-1b follow-on). Mirrors [`reject_plusargs`].
/// OBS-3: `$vita_stage` is a one-shot `vita` observability task — the staged tools do
/// not emit `stage.jsonl`, and a staged run would print the intercepted no-op Display.
/// Loud-reject a staged design that uses `$vita_stage` (staged staging is a follow-on;
/// `--obs-dir` is already one-shot only).
pub(crate) fn reject_stage_staged(sc: &elaborate::Sidecars) -> Result<(), i32> {
    if !sc.stage_stmts.is_empty() {
        eprintln!(
            "error[{}]: `$vita_stage` is a one-shot `vita` task — `velab` does not \
             stage it (run one-shot: `vita <design> --obs-dir <D> +STAGE_TRACE`)",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

/// `--backend` selects the SIMULATE-side executor, so it means nothing to a
/// compile/elaborate applet: nothing an artifact records depends on it (the
/// backend rides `SimOpts` out-of-band and never enters the frozen `SimIr`).
/// Accepting-and-ignoring it would silently mislead — `vcmp --backend vm` would
/// look like it had produced a faster artifact.
pub(crate) fn reject_backend(stage: &str, io: &IoArgs) -> Result<(), i32> {
    if let Some(b) = io.backend {
        eprintln!(
            "error[{}]: '--backend {}' is a simulate-side argument — '{stage}' does not \
             run process bodies. Pass it to `vita` or `vrun` instead; the choice does not \
             affect the artifact '{stage}' writes",
            MsgCode::CliBadFlag.code_num(),
            backend_name(b)
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

/// Loud wrong-stage rejection for `-G`/`--param`. doc-14 RULE B puts the parameter
/// override on the ELABORATE stage: it changes the flattened `sim-ir`, so `vcmp`
/// (which has not elaborated yet — it only writes a `.vu`) and `vrun` (which reads an
/// already-elaborated artifact) cannot act on it. Both used to accept it and drop it
/// silently: on `vrun` the design then ran with the declared default at `errors=0`, and
/// on `vcmp` the flag simply vanished — either way the exact failure mode `-G` is made
/// loud about everywhere else. The message names `-G` for whichever spelling was
/// typed; `--param` is documented as its alias.
pub(crate) fn reject_top_params(stage: &str, io: &IoArgs) -> Result<(), i32> {
    if let Some((n, v)) = io.top_params.first() {
        eprintln!(
            "error[{}]: '-G {n}={v}' is an elaborate-stage argument — '{stage}' cannot \
             apply a parameter override. Pass it to `velab` (or one-shot `vita`)",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

pub(crate) fn reject_obs_dir(stage: &str, io: &IoArgs) -> Result<(), i32> {
    if let Some(dir) = &io.obs_dir {
        eprintln!(
            "error[{}]: '--obs-dir {dir}' is a one-shot `vita` argument — '{stage}' \
             does not emit the obs rail (a staged-run manifest is a follow-on)",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    if !io.probes.is_empty() || io.probe_file.is_some() {
        eprintln!(
            "error[{}]: '--probe'/'--probe-file' is a one-shot `vita` argument — \
             '{stage}' does not emit the trace rail (staged probing is a follow-on)",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

/// OBS-2: resolve `--probe`/`--probe-file` paths to net ids against the elaborated
/// `net_names` table. `--probe` requires `--obs-dir` (the `trace.jsonl` target). An
/// unresolved path is LOUD (never a silent skip — a probe typo must not vanish).
/// Returns the resolved net ids (empty when no `--probe` was given).
pub(crate) fn resolve_probes(
    probes: &[String],
    probe_file: Option<&str>,
    obs_dir: Option<&str>,
    net_names: &[String],
    nets: &[sim_ir::NetVar],
) -> Result<Vec<u32>, i32> {
    let mut paths: Vec<String> = probes.to_vec();
    if let Some(f) = probe_file {
        let content = std::fs::read_to_string(f).map_err(|e| {
            eprintln!(
                "error[{}]: cannot read --probe-file '{f}': {e}",
                MsgCode::CliBadFlag.code_num()
            );
            EXIT_CLI_ERROR
        })?;
        for line in content.lines() {
            let t = line.trim();
            if !t.is_empty() && !t.starts_with('#') {
                paths.push(t.to_string());
            }
        }
    }
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    if obs_dir.is_none() {
        eprintln!(
            "error[{}]: '--probe' requires '--obs-dir <D>' (trace.jsonl is written there)",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    let mut ids = Vec::new();
    for p in &paths {
        match net_names.iter().position(|n| n == p) {
            Some(idx) => {
                // OBS-2 v1: `fmt_probe_value` formats a plain integral bit-vector
                // (Wire/Reg/Logic/Integer, `array_len == 1`) — the only kind whose
                // whole-net value the trace captures faithfully. Loud-reject anything
                // else, because it would silently misreport:
                //   • unpacked ARRAY (`array_len > 1`) → only element 0 is formatted;
                //   • dynamic-array/queue/string HANDLE (`array_len == 0`) → heap
                //     storage, not a bit vector;
                //   • REAL/realtime (`array_len == 1`, kind `Real`) → the raw f64
                //     bit-pattern, NOT the value (≠ VCD `r1.5` / `$monitor` 1.5).
                // per-element / real / handle probing is a follow-on.
                let reason = nets.get(idx).and_then(|n| {
                    if n.array_len == 0 {
                        Some("a dynamic-array/queue/string handle")
                    } else if n.array_len > 1 {
                        Some("an unpacked array")
                    } else if matches!(n.kind, sim_ir::NetKind::Real) {
                        Some("a real/realtime net")
                    } else {
                        None
                    }
                });
                if let Some(r) = reason {
                    eprintln!(
                        "error[{}]: --probe path '{p}' is {r} — v1 can trace only a \
                         scalar/vector/packed net (real / per-element array / handle \
                         probing is a follow-on)",
                        MsgCode::CliBadFlag.code_num()
                    );
                    return Err(EXIT_CLI_ERROR);
                }
                ids.push(idx as u32);
            }
            None => {
                eprintln!(
                    "error[{}]: --probe path '{p}' does not resolve to a net \
                     (check the hierarchical name; `--dump-filelist`-style net listing is a follow-on)",
                    MsgCode::CliBadFlag.code_num()
                );
                return Err(EXIT_CLI_ERROR);
            }
        }
    }
    Ok(ids)
}

pub(crate) fn reject_worklib_flags(
    stage: &str,
    io: &IoArgs,
    allow_work: bool,
    allow_libs: bool,
    allow_tops: bool,
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
    // `--top` selects the elaboration root. `velab` (staged elaborate) and the
    // one-shot `vita` both elaborate, so both accept it; `vcmp` (compile-only)
    // and `vrun` (root already fixed in the `.vu`/`.velab`) do not.
    if !allow_tops && !io.tops.is_empty() {
        eprintln!(
            "error[{}]: --top selects an elaboration root — '{stage}' has no root selection",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}

/// Resolve `--work <name[=dir]>` / `--workdir <dir>` into (logical name, dir):
/// `--work n=d` pins both; `--work n` puts the library at `./n` unless
/// `--workdir` overrides; a bare `--workdir d` means the default name `work`.
pub(crate) fn parse_work_spec(io: &IoArgs) -> Result<Option<(String, String)>, i32> {
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

pub(crate) fn dispatch_vcmp(args: &[String], inv: Invocation) -> i32 {
    let io = match parse_io_args(args) {
        Ok(x) => x,
        Err(c) => return c,
    };
    if io.dump_filelist {
        return run_dump_filelist(&io);
    }
    if let Err(c) = reject_worklib_flags("vcmp", &io, true, false, false) {
        return c;
    }
    if let Err(c) = reject_plusargs("vcmp", &io) {
        return c;
    }
    if let Err(c) = reject_obs_dir("vcmp", &io) {
        return c;
    }
    if let Err(c) = reject_backend("vcmp", &io) {
        return c;
    }
    if let Err(c) = reject_top_params("vcmp", &io) {
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
            overrides: io.overrides.clone(),
            invocation: Some(inv),
            ..VitaOpts::default()
        },
    )
}

pub(crate) fn dispatch_velab(args: &[String], inv: Invocation) -> i32 {
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
    if let Err(c) = reject_worklib_flags("velab", &io, false, true, true) {
        return c;
    }
    if let Err(c) = reject_plusargs("velab", &io) {
        return c;
    }
    if let Err(c) = reject_obs_dir("velab", &io) {
        return c;
    }
    if let Err(c) = reject_backend("velab", &io) {
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
                // doc-14 RULE B: `-G` is a velab input. Dropping it HERE — at the
                // argv→opts boundary — is what made the flag a silent no-op even
                // though every stage below it was ready to apply it.
                top_params: io.top_params.clone(),
                overrides: io.overrides.clone(),
                invocation: Some(inv),
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
            // doc-14 RULE B — twin of the `-L` branch above.
            top_params: io.top_params,
            overrides: io.overrides.clone(),
            invocation: Some(inv),
            ..VitaOpts::default()
        },
    )
}

pub(crate) fn dispatch_vrun(args: &[String], inv: Invocation) -> i32 {
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
    if let Err(c) = reject_worklib_flags("vrun", &io, false, false, false) {
        return c;
    }
    if let Err(c) = reject_obs_dir("vrun", &io) {
        return c;
    }
    if let Err(c) = reject_top_params("vrun", &io) {
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
        backend: io.backend,
        overrides: io.overrides.clone(),
        invocation: Some(inv),
        ..VitaOpts::default()
    };
    run_vrun(&io.pos[0], &opts)
}
