//! G2 observability rail writers — OBS-1a run manifest (`run.json`, R-L0) +
//! result ledger (`results.jsonl`, R-L1), OBS-1b functional coverage
//! (`coverage.json`, R-L5), OBS-2 probe trace (`trace.jsonl`), and OBS-3 stage
//! ledger (`stage.jsonl`), all written to `--obs-dir`.
//!
//! This is an out-of-band, machine-readable companion to the human-facing
//! stdout/VCD — "what was the run's config + source identity + pass/fail
//! verdict" for an LLM/CI harness. SPEC = `docs/preview/19-ai-agent-observability.md`
//! §2/§4. Everything here is derived from the SAME `SimResult` + diagnostic
//! counts that drive the exit code and `$display`, so the JSON can never
//! disagree with them (doc-19 §3: a wrong log is a silent-wrong). All fields
//! are DETERMINISTIC except the four isolated timing fields (`utc_unix_s`,
//! `wall_s`, `elab_s`, `sim_s`), which a determinism golden excludes.
//!
//! Not here (follow-ons): `sva.jsonl` (OBS-2 잔여), staged/vrun obs, a
//! compile-fail manifest, a `--seed` flag, and a per-testcase `results`
//! ledger (v2, `$vita_test_begin/end`) are later slices (ROADMAP §6).

use std::io::Write;

/// obs rail schema version (doc-19 §3; bump only on a record-envelope change,
/// never for an additive field on an existing kind).
const SCHEMA_VER: u32 = 1;

/// The end-of-run facts the manifest + ledger serialize. Gathered once, from
/// the engine result + CLI opts (single source — see module docs).
pub struct ObsRun<'a> {
    /// Display name of the design (first source path).
    pub source_name: &'a str,
    /// blake3 hex of the fused source TEXT only. NOT a full input identity:
    /// `-D`/`+define+` macros and `-I` include contents are not folded in yet
    /// (a run with different `-Dfoo` yields the same hash) — recording those is
    /// an OBS-1b follow-on. It IS a stable digest of the concatenated sources.
    pub source_blake3: String,
    /// Runtime plusargs, command-line order (leading `+` already stripped).
    pub plusargs: &'a [String],
    /// The frozen artifact `format_version` this build emits.
    pub format_version: u32,
    /// `"finish"|"stop"|"quiescent"|"delta_limit"|"error"`.
    pub finish_reason: &'static str,
    /// `"ok"|"had_errors"|"fatal"`.
    pub exit_class: &'static str,
    /// The doc-13 process exit code actually returned.
    pub exit_code: i32,
    /// Final simulation time (ticks).
    pub sim_time: u64,
    // Three PRECISE buckets. NOTE the convention: `errors` EXCLUDES fatals
    // (fatals is its own field), whereas the doc-13 stderr epilogue's `errors=`
    // token = errors + fatals. They differ exactly when `fatals > 0` — a harness
    // must not naively equate `counts.errors` with the epilogue number.
    pub errors: u32,
    pub warnings: u32,
    pub fatals: u32,
    /// `"PASS"|"FAIL"` (PASS iff `exit_code == 0`).
    pub status: &'static str,
    /// What `--backend` ASKED for (same vocabulary). Present because it can
    /// differ from `backend`: `native` falls back while no native executor
    /// exists, and `native.refused` cannot reveal that — it describes the
    /// DESIGN and is `null` precisely when nothing refuses it. Comparing the
    /// two fields is the only way a manifest reader sees an unhonored request.
    pub backend_requested: &'static str,
    /// The EFFECTIVE process-body executor of this run, in the `--backend` flag's
    /// vocabulary (`"vm"`|`"interp"`|`"native"`). Recorded so `codegen` below cannot be
    /// misread: that object is a static capability census, and on an
    /// `--backend interp` run `able == total` with 0% of the runtime on the VM
    /// is normal — this field is what says so (soundness-review F1).
    pub backend: &'static str,
    /// T0 (doc-21 §7.3): the engine's VM-coverage report — computed by the same
    /// walk the VM's compile gate runs (single source), serialized as the
    /// `codegen` object. Deterministic (a static property of the design).
    pub codegen: &'a sim_engine::CodegenReport,
    /// S0 (doc-21 §7.3): the ③층 design-level eligibility verdict, serialized
    /// as the `native` object. Deterministic; static per (design, run options)
    /// — a `--probe`/stage-instrumented run is ineligible by design (§4.3).
    pub native: &'a sim_engine::native::NativeEligibility,
    /// R14 (ROADMAP §3 ⑭): the per-body execution profile, or `None` without
    /// `--obs-procs`. The `evals` half is DETERMINISTIC and belongs in the
    /// golden; the `nanos` half only exists under `--obs-procs-time` and is
    /// isolated exactly like the four wall-clock fields below.
    pub procs: Option<ObsProcs<'a>>,
    /// Isolated non-deterministic field (excluded from the determinism golden).
    pub utc_unix_s: u64,
    /// Isolated non-deterministic field (excluded from the determinism golden).
    pub wall_s: f64,
    /// Wall-clock spent BEFORE `simulate` — preprocess, lex, parse, elaborate.
    /// Isolated non-deterministic field (excluded from the determinism golden).
    ///
    /// ⚠️ `wall_s` alone attributes nothing. A reader comparing `--backend`
    /// values is comparing runs whose front end is identical, so a front-end
    /// dominated design shows every backend within noise and reads as "the
    /// backend does not help" — which is the conclusion an external report drew
    /// from 16 s over 593 cycles. These two fields say which half to look at.
    pub elab_s: f64,
    /// Wall-clock inside `simulate` — the only part `--backend` can move.
    /// Isolated non-deterministic field (excluded from the determinism golden).
    pub sim_s: f64,
}

/// Append `s` as a JSON string literal (quotes + minimal RFC-8259 escaping).
fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Append a JSON array of strings (fixed order = input order).
fn json_str_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        json_str(out, it);
    }
    out.push(']');
}

/// R14 (ROADMAP §3 ⑭) — everything `run.json`'s `processes` object needs, in one
/// borrow: the engine's accumulators plus the elaborate-time identity tables
/// that turn an index into a line of RTL.
///
/// Two SEPARATE tables and not one, because the two domains are indexed
/// independently in the IR (`SimIr.processes` / `SimIr.cont_assigns`). The
/// serialized row carries its `domain` explicitly rather than letting the reader
/// infer it from `kind` — `kind` is a source-construct vocabulary that may grow,
/// and a consumer keying off it would break the day it does.
pub struct ObsProcs<'a> {
    pub profile: &'a sim_engine::ProcProfile,
    /// Per-ProcId identity, parallel to `profile.evals`.
    pub proc_idents: &'a [sim_engine::ProcIdent],
    /// Per-cont-assign identity, parallel to `profile.ca_evals`.
    pub ca_idents: &'a [sim_engine::ProcIdent],
}

/// One serialized profile row, already resolved from the two parallel tables.
///
/// `evals` is the SORT KEY and it is deterministic, so the row ORDER is
/// deterministic too — including on a `--obs-procs-time` run, where `nanos` is
/// wall clock. That is the whole reason timing is not the sort key: a profile
/// whose row order changed between two runs of the same design could not be
/// byte-diffed, which is what the rest of this rail exists to allow.
struct ProcRow<'a> {
    domain: &'static str,
    index: usize,
    ident: &'a sim_engine::ProcIdent,
    evals: u64,
    nanos: u64,
}

impl ObsProcs<'_> {
    /// Flatten both domains into one list, sorted most-expensive-first.
    ///
    /// ⚠️ An identity table SHORTER than its accumulator does not drop the row —
    /// it falls back to a default ident (empty kind/scope). A dropped row would
    /// be a profile that silently omits the process the user is hunting, which
    /// is the one failure this feature cannot afford; an unlabelled row is at
    /// least a visible "here is cost I cannot name".
    fn rows(&self) -> Vec<ProcRow<'_>> {
        static UNKNOWN: std::sync::OnceLock<sim_engine::ProcIdent> = std::sync::OnceLock::new();
        let unknown = UNKNOWN.get_or_init(sim_engine::ProcIdent::default);
        let p = self.profile;
        let mut out: Vec<ProcRow<'_>> = Vec::with_capacity(p.evals.len() + p.ca_evals.len());
        for (i, &n) in p.evals.iter().enumerate() {
            out.push(ProcRow {
                domain: "process",
                index: i,
                ident: self.proc_idents.get(i).unwrap_or(unknown),
                evals: n,
                nanos: p.nanos.get(i).copied().unwrap_or(0),
            });
        }
        for (i, &n) in p.ca_evals.iter().enumerate() {
            out.push(ProcRow {
                domain: "assign",
                index: i,
                ident: self.ca_idents.get(i).unwrap_or(unknown),
                evals: n,
                nanos: p.ca_nanos.get(i).copied().unwrap_or(0),
            });
        }
        // Descending by cost, then a TOTAL tiebreak on (domain, index) so two
        // rows with equal counts cannot swap between runs.
        out.sort_by(|a, b| {
            b.evals
                .cmp(&a.evals)
                .then_with(|| a.domain.cmp(b.domain))
                .then_with(|| a.index.cmp(&b.index))
        });
        out
    }
}

/// Shape the isolated `wall_s` value cleanly (its VALUE is never compared).
fn fmt_wall(w: f64) -> String {
    if w.is_finite() {
        format!("{w:.6}")
    } else {
        "0.0".to_string()
    }
}

impl ObsRun<'_> {
    /// The `run.json` L0 manifest: one field per line for human diffability,
    /// key order fixed so two runs of the same input are byte-identical (bar
    /// the two isolated wall-clock fields).
    fn manifest_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema_ver\": ");
        s.push_str(&SCHEMA_VER.to_string());
        s.push_str(",\n  \"tool\": \"vita\",\n  \"version\": ");
        json_str(&mut s, env!("CARGO_PKG_VERSION"));
        s.push_str(",\n  \"format_version\": ");
        s.push_str(&self.format_version.to_string());
        // No `--seed` flag yet: vita runs are deterministic by construction, so
        // `null` is the honest value (a real seed lands with the flag — R-L0).
        s.push_str(",\n  \"seed\": null,\n  \"plusargs\": ");
        json_str_array(&mut s, self.plusargs);
        s.push_str(",\n  \"source\": {\"name\": ");
        json_str(&mut s, self.source_name);
        s.push_str(", \"blake3\": ");
        json_str(&mut s, &self.source_blake3);
        s.push_str("},\n  \"finish_reason\": ");
        json_str(&mut s, self.finish_reason);
        s.push_str(",\n  \"exit_class\": ");
        json_str(&mut s, self.exit_class);
        s.push_str(",\n  \"exit_code\": ");
        s.push_str(&self.exit_code.to_string());
        s.push_str(",\n  \"sim_time\": ");
        s.push_str(&self.sim_time.to_string());
        s.push_str(",\n  \"counts\": {\"errors\": ");
        s.push_str(&self.errors.to_string());
        s.push_str(", \"warnings\": ");
        s.push_str(&self.warnings.to_string());
        s.push_str(", \"fatals\": ");
        s.push_str(&self.fatals.to_string());
        s.push_str("},\n  \"status\": ");
        json_str(&mut s, self.status);
        // What ran, and what was asked for (see the field docs — the pair is
        // what makes an unhonored `--backend native` visible, and `backend`
        // guards `codegen` against the "able==total means the VM ran it"
        // misreading).
        s.push_str(",\n  \"backend\": ");
        json_str(&mut s, self.backend);
        s.push_str(",\n  \"backend_requested\": ");
        json_str(&mut s, self.backend_requested);
        // T0: the ②층(bytecode VM) claim on this design + why the rest was
        // refused. `able == total` with `frame_bodies > 0` and a `frame_call`/
        // `user_call_in_expr` row is the round-26 shape: full process coverage,
        // 0% of the runtime on the VM. BTreeMap iteration ⇒ key order is stable.
        s.push_str(",\n  \"codegen\": {\"able\": ");
        s.push_str(&self.codegen.coverage.codegen_able.to_string());
        s.push_str(", \"total\": ");
        s.push_str(&self.codegen.coverage.total.to_string());
        s.push_str(", \"frame_bodies\": ");
        s.push_str(&self.codegen.frame_bodies.to_string());
        s.push_str(", \"reject_reasons\": {");
        for (i, (k, n)) in self.codegen.reject_reasons.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            json_str(&mut s, k);
            s.push_str(": ");
            s.push_str(&n.to_string());
        }
        s.push_str("}}");
        // S0: the ③층 design-level verdict (doc-21 §7.3) — same stable-order
        // BTreeMap serialization as `codegen`.
        s.push_str(",\n  \"native\": {\"eligible\": ");
        s.push_str(if self.native.eligible {
            "true"
        } else {
            "false"
        });
        // The STORAGE-level half next to the scope-level one: they answer
        // different questions and their counts differ (a subroutine design is
        // eligible and not buildable), so folding them into one flag would let
        // an upper bound read as a capability. `refused` is the RUNTIME gate's
        // answer — `null` means nothing refuses this design.
        s.push_str(", \"buildable\": ");
        s.push_str(if self.native.buildable {
            "true"
        } else {
            "false"
        });
        s.push_str(", \"refused\": ");
        match self.native.refused {
            Some(r) => json_str(&mut s, r),
            None => s.push_str("null"),
        }
        s.push_str(", \"reject_reasons\": {");
        for (i, (k, n)) in self.native.reject_reasons.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            json_str(&mut s, k);
            s.push_str(": ");
            s.push_str(&n.to_string());
        }
        s.push_str("}}");
        // R14: the DYNAMIC counterpart of `codegen`. That object says which
        // bodies the VM *could* compile; this one says which bodies actually
        // ran and how often — the question "which `always_comb` eats the cost"
        // asks, and the one a static census cannot answer. `null` (not an empty
        // object) without `--obs-procs`, so a consumer can tell "not measured"
        // from "measured, nothing ran".
        s.push_str(",\n  \"processes\": ");
        match &self.procs {
            None => s.push_str("null"),
            Some(pp) => {
                let rows = pp.rows();
                let timed = pp.profile.timed;
                s.push_str("{\"timed\": ");
                s.push_str(if timed { "true" } else { "false" });
                s.push_str(", \"counts\": {\"processes\": ");
                s.push_str(&pp.profile.evals.len().to_string());
                s.push_str(", \"assigns\": ");
                s.push_str(&pp.profile.ca_evals.len().to_string());
                s.push_str(", \"total_evals\": ");
                let total: u64 = rows.iter().map(|r| r.evals).sum();
                s.push_str(&total.to_string());
                s.push_str("},\n  \"items\": [");
                for (i, r) in rows.iter().enumerate() {
                    s.push_str(if i > 0 { ",\n    {" } else { "\n    {" });
                    s.push_str("\"domain\": ");
                    json_str(&mut s, r.domain);
                    s.push_str(", \"index\": ");
                    s.push_str(&r.index.to_string());
                    s.push_str(", \"kind\": ");
                    json_str(&mut s, r.ident.kind);
                    s.push_str(", \"scope\": ");
                    json_str(&mut s, &r.ident.scope);
                    s.push_str(", \"file\": ");
                    json_str(&mut s, &r.ident.file);
                    s.push_str(", \"line\": ");
                    s.push_str(&r.ident.line.to_string());
                    s.push_str(", \"col\": ");
                    s.push_str(&r.ident.col.to_string());
                    s.push_str(", \"evals\": ");
                    s.push_str(&r.evals.to_string());
                    // The wall-clock half is EMITTED ONLY when it was measured.
                    // A `"time_s": 0.0` on an untimed run reads as "this body
                    // costs nothing", which is a different claim from "nobody
                    // asked" — and this rail is read by agents.
                    if timed {
                        s.push_str(", \"time_s\": ");
                        s.push_str(&fmt_wall(r.nanos as f64 / 1e9));
                    }
                    s.push('}');
                }
                s.push_str("\n  ]}");
            }
        }
        // ── isolated wall-clock (excluded from the determinism golden) ──
        s.push_str(",\n  \"utc_unix_s\": ");
        s.push_str(&self.utc_unix_s.to_string());
        s.push_str(",\n  \"wall_s\": ");
        s.push_str(&fmt_wall(self.wall_s));
        s.push_str(",\n  \"elab_s\": ");
        s.push_str(&fmt_wall(self.elab_s));
        s.push_str(",\n  \"sim_s\": ");
        s.push_str(&fmt_wall(self.sim_s));
        s.push_str("\n}\n");
        s
    }

    /// The `results.jsonl` L1 ledger line (v1 = one line per run; record
    /// envelope `{"v","t","kind",…}` per doc-19 §3). FULLY deterministic — no
    /// wall-clock field, so the whole file byte-diffs clean across runs.
    fn ledger_line(&self) -> String {
        let mut s = String::new();
        s.push_str("{\"v\":1,\"t\":");
        s.push_str(&self.sim_time.to_string());
        s.push_str(",\"kind\":\"result\",\"status\":");
        json_str(&mut s, self.status);
        s.push_str(",\"finish_reason\":");
        json_str(&mut s, self.finish_reason);
        s.push_str(",\"exit_code\":");
        s.push_str(&self.exit_code.to_string());
        s.push_str(",\"sim_time\":");
        s.push_str(&self.sim_time.to_string());
        s.push_str(",\"errors\":");
        s.push_str(&self.errors.to_string());
        s.push_str(",\"warnings\":");
        s.push_str(&self.warnings.to_string());
        s.push_str(",\"fatals\":");
        s.push_str(&self.fatals.to_string());
        s.push('}');
        s.push('\n');
        s
    }
}

/// Map `FinishReason` to its stable rail string.
pub fn finish_reason_str(r: sim_engine::FinishReason) -> &'static str {
    use sim_engine::FinishReason::*;
    match r {
        Finish => "finish",
        Stop => "stop",
        Quiescent => "quiescent",
        DeltaLimit => "delta_limit",
        Error => "error",
    }
}

/// Write `run.json` + `results.jsonl` into `dir` (created if absent). Returns
/// an `io::Error` on any filesystem failure so the caller can surface it loud
/// (a silently-missing obs file would mislead the harness).
pub fn write_run_dir(dir: &str, run: &ObsRun) -> std::io::Result<()> {
    let d = std::path::Path::new(dir);
    std::fs::create_dir_all(d)?;
    std::fs::File::create(d.join("run.json"))?.write_all(run.manifest_json().as_bytes())?;
    std::fs::File::create(d.join("results.jsonl"))?.write_all(run.ledger_line().as_bytes())?;
    Ok(())
}

/// Shape a coverage percent to 6 decimals — matches `$display("%f", …)`, and the
/// underlying f64 is computed IDENTICALLY to `get_coverage` (see the engine's
/// end-of-run summary), so `coverage.json` never disagrees with the RTL's own
/// `c.get_coverage()`. Fixed format ⇒ deterministic (2-run byte-identical).
fn fmt_pct(p: f64) -> String {
    if p.is_finite() {
        format!("{p:.6}")
    } else {
        "0.000000".to_string()
    }
}

/// The `coverage.json` (R-L5) payload: N5 functional coverage — per covergroup
/// instance, its overall percent + a per-item (coverpoint/cross) breakdown. Fixed
/// key + iteration order (the manifest is built in deterministic elaboration order)
/// ⇒ byte-identical across runs of the same input.
pub fn coverage_json(cov: &sim_engine::CoverageSummary) -> String {
    let mut s = String::new();
    s.push_str("{\n  \"schema_ver\": ");
    s.push_str(&SCHEMA_VER.to_string());
    s.push_str(",\n  \"kind\": \"coverage\",\n  \"groups\": [");
    for (gi, g) in cov.groups.iter().enumerate() {
        s.push_str(if gi > 0 { ",\n    {" } else { "\n    {" });
        s.push_str("\"instance\": ");
        json_str(&mut s, &g.instance);
        s.push_str(", \"coverage_pct\": ");
        s.push_str(&fmt_pct(g.coverage_pct));
        s.push_str(", \"coverpoints\": [");
        for (ii, it) in g.items.iter().enumerate() {
            s.push_str(if ii > 0 { ",\n      {" } else { "\n      {" });
            s.push_str("\"name\": ");
            json_str(&mut s, &it.name);
            s.push_str(", \"kind\": ");
            json_str(&mut s, if it.is_cross { "cross" } else { "coverpoint" });
            s.push_str(", \"num_bins\": ");
            s.push_str(&it.num_bins.to_string());
            s.push_str(", \"covered_bins\": ");
            s.push_str(&it.covered_bins.to_string());
            s.push_str(", \"coverage_pct\": ");
            s.push_str(&fmt_pct(it.coverage_pct));
            s.push('}');
        }
        s.push_str("]}");
    }
    s.push_str("\n  ]\n}\n");
    s
}

/// Write `coverage.json` into `dir` (R-L5, OBS-1b). Loud on any filesystem error
/// (a silently-missing coverage file would mislead the harness). Only called when
/// the run produced a coverage summary (the design had ≥1 covergroup instance).
pub fn write_coverage_dir(dir: &str, cov: &sim_engine::CoverageSummary) -> std::io::Result<()> {
    let d = std::path::Path::new(dir);
    std::fs::create_dir_all(d)?;
    std::fs::File::create(d.join("coverage.json"))?.write_all(coverage_json(cov).as_bytes())?;
    Ok(())
}

/// Write `trace.jsonl` into `dir` (R-L3, OBS-2). Each element of `lines` is a
/// complete `{v,t,kind:"chg",…}` record (already serialized by the engine, in time
/// order); this joins them one-per-line. Loud on any filesystem error.
pub fn write_trace_dir(dir: &str, lines: &[String]) -> std::io::Result<()> {
    write_jsonl(dir, "trace.jsonl", lines)
}

/// Write `stage.jsonl` into `dir` (R-S3, OBS-3): one `{v,t,kind:"stage",…}` record
/// per `$vita_stage` call, in emission order. Loud on any filesystem error.
pub fn write_stage_dir(dir: &str, lines: &[String]) -> std::io::Result<()> {
    write_jsonl(dir, "stage.jsonl", lines)
}

/// Join engine-serialized JSONL records one-per-line and write them to `dir/name`.
fn write_jsonl(dir: &str, name: &str, lines: &[String]) -> std::io::Result<()> {
    let d = std::path::Path::new(dir);
    std::fs::create_dir_all(d)?;
    let mut body = String::with_capacity(lines.iter().map(|l| l.len() + 1).sum());
    for l in lines {
        body.push_str(l);
        body.push('\n');
    }
    std::fs::File::create(d.join(name))?.write_all(body.as_bytes())?;
    Ok(())
}
