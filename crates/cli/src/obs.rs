//! OBS-1a (G2 observability rail): the run manifest (`run.json`, R-L0) and the
//! single-run result ledger (`results.jsonl`, R-L1), written to `--obs-dir`.
//!
//! This is an out-of-band, machine-readable companion to the human-facing
//! stdout/VCD — "what was the run's config + source identity + pass/fail
//! verdict" for an LLM/CI harness. SPEC = `docs/preview/19-ai-agent-observability.md`
//! §2/§4. Everything here is derived from the SAME `SimResult` + diagnostic
//! counts that drive the exit code and `$display`, so the JSON can never
//! disagree with them (doc-19 §3: a wrong log is a silent-wrong). All fields
//! are DETERMINISTIC except the two isolated wall-clock fields (`utc_unix_s`,
//! `wall_s`), which a determinism golden excludes.
//!
//! Not here (follow-ons): `coverage.json` (R-L5) is OBS-1b — functional
//! coverage is synthesized into IR nets with no engine-level aggregate to
//! flush, so a faithful export needs its own slice, not a fabricated count.
//! A per-source-file breakdown, a `--seed` flag, and a per-testcase `results`
//! ledger (v2, `$vita_test_begin/end`) are later slices too.

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
    /// Isolated non-deterministic field (excluded from the determinism golden).
    pub utc_unix_s: u64,
    /// Isolated non-deterministic field (excluded from the determinism golden).
    pub wall_s: f64,
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
        // ── isolated wall-clock (excluded from the determinism golden) ──
        s.push_str(",\n  \"utc_unix_s\": ");
        s.push_str(&self.utc_unix_s.to_string());
        s.push_str(",\n  \"wall_s\": ");
        s.push_str(&fmt_wall(self.wall_s));
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
    let d = std::path::Path::new(dir);
    std::fs::create_dir_all(d)?;
    let mut body = String::with_capacity(lines.iter().map(|l| l.len() + 1).sum());
    for l in lines {
        body.push_str(l);
        body.push('\n');
    }
    std::fs::File::create(d.join("trace.jsonl"))?.write_all(body.as_bytes())?;
    Ok(())
}
