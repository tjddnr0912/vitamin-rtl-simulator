//! split part of `cli` (mechanical move).

use super::*;

pub(crate) fn run_velab_lib_gated(
    libs: &[(String, String)],
    tops: &[String],
    out: &str,
    top_params: &[(String, String)],
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
        prec_exp: std::collections::BTreeMap<String, i8>,
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
        // The v28 source-map tail is DROPPED on this path: the merge below
        // splices items from MULTIPLE CUs into one unit, and each CU's AST spans
        // index its OWN expanded buffer starting at 0 — the coordinate spaces
        // overlap, so no single resolver can tell which CU a span belongs to.
        // Resolving through the wrong CU's map would print a WRONG file:line,
        // which is worse than the unlocated diagnostic this leaves (recorded in
        // ROADMAP §3; the plain `velab a.vu` path locates since v28).
        let (unit, unit_exp, prec_exp, prec, _smap) = match decode_vu_unit(&bytes, sink) {
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
            prec_exp,
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
    let mut merged_prec: std::collections::BTreeMap<String, i8> = std::collections::BTreeMap::new();
    let mut prec = i8::MAX;
    for cu in &loaded {
        prec = prec.min(cu.prec);
        for (k, v) in &cu.unit_exp {
            merged_exp.entry(k.clone()).or_insert(*v);
        }
        for (k, v) in &cu.prec_exp {
            merged_prec.entry(k.clone()).or_insert(*v);
        }
        for it in &cu.unit.items {
            let name = match it {
                hdl_ast::TopItem::Module(m)
                | hdl_ast::TopItem::Interface(m)
                | hdl_ast::TopItem::Package(m) => Some(m.name.name.clone()),
                hdl_ast::TopItem::Class(c) => Some(c.name.name.clone()),
                hdl_ast::TopItem::Import(_)
                | hdl_ast::TopItem::Bind(_)
                | hdl_ast::TopItem::Error(_) => None,
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
    // doc-14 RULE B: `-G` is an elaborate-stage input, and the `-L` compose path is
    // still `velab`. Twin of the plain `velab` call in `pipeline.rs` — dropping it
    // silently elaborated the declared defaults at `errors=0`.
    let (ir, sc) = elaborate::elaborate_located_params(
        &merged,
        sink,
        &merged_exp,
        &merged_prec,
        prec,
        Some(tops),
        None,
        top_params,
    );
    let Some(ir) = ir else {
        return EXIT_USER_ERROR;
    };
    if let Err(c) = reject_stage_staged(&sc) {
        return c;
    }
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
    emit_flist_overrides(&sink, &opts.overrides);
    if inner.verbose() {
        let src = [velab_path.to_string()];
        let up: Vec<String> = opts.upstream.iter().cloned().collect();
        echo::echo_effective_invocation(
            &sink,
            &src,
            opts.vcd_path_override.as_deref(),
            opts,
            &[("upstream", up)],
        );
    }
    let code = run_vrun_gated(velab_path, opts, &inner, &sink);
    inner.epilogue();
    code
}

pub(crate) fn run_vrun_gated(
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
    type TsTrailer = (Vec<u64>, i8, Vec<u64>);
    let ((proc_multipliers, global_prec_exp, proc_prec_mults), rest4): (TsTrailer, &[u8]) =
        if rest3.is_empty() {
            ((Vec::new(), -9, Vec::new()), rest3)
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
        // OBS-2: --probe is one-shot `vita` only (reject_obs_dir rejects it for the
        // staged tools), so the staged path never probes.
        probed_nets: Vec::new(),
        proc_multipliers,
        proc_prec_mults,
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
        func_names: extra.func_names,
        task_calls_proc: extra.task_calls_proc,
        task_calls_func: extra.task_calls_func,
        two_state_nets: extra.two_state_nets,
        real_elem_dyn_nets: extra.real_elem_dyn_nets,
        string_elem_dyn_nets: extra.string_elem_dyn_nets,
        net_decl_ranges: extra.net_decl_ranges,
        file_directed_stmts: extra.file_directed_stmts,
        init_procs: extra.init_procs,
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
        // #10 (STAGED-DROP parity): severity file:line:col + instance — without
        // this the staged run silently prints location-less severity reports
        // while one-shot locates them.
        stmt_locs: extra.stmt_locs,
        stage_stmts: std::collections::BTreeSet::new(),
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

/// W-FLIST-OVERRIDE: a single-value knob set twice — RECORD it (last wins)
/// during arg parsing; [`emit_flist_overrides`] replays the events through the
/// GATED sink once it exists. (A raw `eprintln!` here bypassed the doc-13
/// uniform gate: `-Werror=W-FLIST-OVERRIDE` could never promote it and the
/// counts epilogue never included it — adversarial review.)
pub(crate) fn record_override(
    overrides: &mut Vec<(String, String, String)>,
    knob: &str,
    old_v: &str,
    new_v: &str,
) {
    overrides.push((knob.to_string(), old_v.to_string(), new_v.to_string()));
}

/// Replay the [`record_override`] events through the gated sink (doc-14 §3.1
/// wording + doc-15 W8009 example format).
pub(crate) fn emit_flist_overrides(sink: &dyn LogSink, overrides: &[(String, String, String)]) {
    for (knob, old_v, new_v) in overrides {
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::FlistOverride,
            message: format!("{knob} '{old_v}' overridden by '{new_v}' (last wins)"),
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }
}
