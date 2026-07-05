//! OBS-1a (G2 observability): `--obs-dir` → `run.json` (L0 manifest) +
//! `results.jsonl` (L1 ledger). SPEC = docs/preview/19-ai-agent-observability.md.
//!
//! Teeth (doc-19 §3): (1) DETERMINISM — two runs of the same input produce
//! byte-identical files, except the two isolated wall-clock fields
//! (`utc_unix_s`, `wall_s`); (2) 3-WAY consistency — the manifest `status`/
//! `exit_code` match the actual process exit code (which drives `$display`/exit
//! from the SAME `SimResult`). A wrong log is a silent-wrong (LLM-misleading),
//! so these are pinned hard.
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` with extra `args`; returns (stdout, exit_code, obs_dir).
fn run(src: &str, args: &[&str]) -> (String, i32, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_obs_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let obs = d.join("obsout");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg(obs.to_str().unwrap())
        .args(args)
        .current_dir(&d);
    let out = cmd.output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
        obs,
    )
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

/// Grab the value token of a top-level `"key": <value>` line from run.json.
fn field<'a>(json: &'a str, key: &str) -> &'a str {
    for line in json.lines() {
        let t = line.trim().trim_end_matches(',');
        if let Some(rest) = t.strip_prefix(&format!("\"{key}\": ")) {
            return rest;
        }
    }
    ""
}

const PASS_SV: &str = "module top; int x = 5;\n\
     initial begin #10; $display(\"x=%0d\", x); $finish; end endmodule\n";

#[test]
fn writes_run_and_results() {
    let (_, code, obs) = run(PASS_SV, &[]);
    assert_eq!(code, 0);
    let manifest = read(&obs.join("run.json"));
    let ledger = read(&obs.join("results.jsonl"));
    assert!(!manifest.is_empty(), "run.json missing");
    assert!(!ledger.is_empty(), "results.jsonl missing");
    // L0 manifest key fields.
    assert_eq!(field(&manifest, "schema_ver"), "1");
    assert_eq!(field(&manifest, "tool"), "\"vita\"");
    assert_eq!(field(&manifest, "format_version"), "19");
    assert_eq!(field(&manifest, "seed"), "null");
    assert_eq!(field(&manifest, "finish_reason"), "\"finish\"");
    assert_eq!(field(&manifest, "exit_class"), "\"ok\"");
    assert_eq!(field(&manifest, "exit_code"), "0");
    assert_eq!(field(&manifest, "sim_time"), "10");
    assert_eq!(field(&manifest, "status"), "\"PASS\"");
    // L1 ledger = ONE envelope line, deterministic (no wall-clock).
    assert_eq!(ledger.lines().count(), 1);
    assert!(ledger.starts_with("{\"v\":1,\"t\":10,\"kind\":\"result\","));
    assert!(ledger.contains("\"status\":\"PASS\""));
    assert!(ledger.contains("\"exit_code\":0"));
    assert!(ledger.ends_with("}\n"));
}

#[test]
fn two_runs_byte_identical_bar_wallclock() {
    // The determinism golden: same input → identical files, except the two
    // isolated wall-clock fields (which the harness excludes).
    let (_, _, a) = run(PASS_SV, &["+SEED_ARG=1"]);
    let (_, _, b) = run(PASS_SV, &["+SEED_ARG=1"]);
    // results.jsonl has NO wall-clock — fully byte-identical.
    assert_eq!(
        read(&a.join("results.jsonl")),
        read(&b.join("results.jsonl")),
        "results.jsonl must be byte-identical across runs"
    );
    // run.json identical once the two isolated fields are stripped.
    let strip = |s: String| {
        s.lines()
            .filter(|l| {
                let t = l.trim();
                !t.starts_with("\"utc_unix_s\"") && !t.starts_with("\"wall_s\"")
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        strip(read(&a.join("run.json"))),
        strip(read(&b.join("run.json"))),
        "run.json must be byte-identical bar the isolated wall-clock fields"
    );
    // And the isolated fields ARE present (so the exclusion rule is meaningful).
    assert!(read(&a.join("run.json")).contains("\"utc_unix_s\":"));
    assert!(read(&a.join("run.json")).contains("\"wall_s\":"));
}

#[test]
fn fail_status_matches_process_exit() {
    // 3-way: a $fatal run exits non-zero AND the manifest/ledger say FAIL with
    // the SAME exit code — derived from the same SimResult, cannot disagree.
    let (_, code, obs) = run(
        "module top; initial begin $display(\"before\"); $fatal(1, \"boom\"); end endmodule\n",
        &[],
    );
    assert_ne!(code, 0, "$fatal must exit non-zero");
    let manifest = read(&obs.join("run.json"));
    assert_eq!(field(&manifest, "status"), "\"FAIL\"");
    assert_eq!(field(&manifest, "exit_class"), "\"fatal\"");
    assert_eq!(field(&manifest, "exit_code"), code.to_string());
    let ledger = read(&obs.join("results.jsonl"));
    assert!(ledger.contains("\"status\":\"FAIL\""));
    assert!(ledger.contains(&format!("\"exit_code\":{code}")));
    assert!(ledger.contains("\"fatals\":1"));
}

#[test]
fn plusargs_and_source_hash_recorded() {
    let (_, _, obs) = run(PASS_SV, &["+MYARG=1", "+FOO"]);
    let manifest = read(&obs.join("run.json"));
    // plusargs preserve command-line order, leading '+' stripped.
    assert_eq!(field(&manifest, "plusargs"), "[\"MYARG=1\",\"FOO\"]");
    // source blake3 = the 64-hex value inside `"blake3": "<hash>"`.
    let after = manifest
        .split("\"blake3\": \"")
        .nth(1)
        .expect("blake3 field missing");
    let hash = &after[..after.find('"').expect("unterminated blake3")];
    assert_eq!(hash.len(), 64, "blake3 must be 64 hex chars: {hash:?}");
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "blake3 must be all hex: {hash:?}"
    );
}

#[test]
fn compile_error_writes_no_obs() {
    // v1 behavior (documented): obs files are written only when the SIMULATION
    // runs. A parse/elaborate failure returns before the sim, so no obs is
    // emitted — the non-zero exit + stderr diagnostics are the failure signal.
    // (A compile-failed manifest is an OBS-1b follow-on.) Pinned so a change is
    // intentional.
    let (_, code, obs) = run("module top; initial x = 1; endmodule\n", &[]);
    assert_ne!(code, 0, "elaborate error must exit non-zero");
    assert!(
        !obs.join("run.json").exists(),
        "compile failure must not write run.json (v1)"
    );
}

#[test]
fn exit_class_matches_final_code_under_werror() {
    // Adversarial find (fixed): under `-Werror` a promoted warning flips the
    // exit code, so `exit_class` (a verdict field that maps to the exit code)
    // must NOT stay the engine's raw "ok" while exit_code=1/status=FAIL — else
    // a harness keying on exit_class reads a failed run as clean. `finish_reason`
    // stays "finish" (DESCRIPTIVE: the sim did reach $finish).
    // (t.sv has no `timescale ⇒ a W1017 warning ⇒ promoted to an error by -Werror.)
    let (_, code, obs) = run(PASS_SV, &["-Werror"]);
    assert_ne!(code, 0, "-Werror-promoted warning must fail");
    let m = read(&obs.join("run.json"));
    assert_eq!(field(&m, "status"), "\"FAIL\"");
    assert_eq!(field(&m, "exit_code"), code.to_string());
    assert_eq!(
        field(&m, "exit_class"),
        "\"had_errors\"",
        "exit_class must agree with the failing exit code:\n{m}"
    );
    // finish_reason is descriptive — the sim genuinely finished.
    assert_eq!(field(&m, "finish_reason"), "\"finish\"");
}

#[test]
fn obs_dir_rejected_on_staged_applets() {
    // Adversarial find (fixed): `--obs-dir` is honored only by one-shot `vita`.
    // On the staged applets it must LOUD-reject, not silently drop (a silent
    // no-op on `vrun` — the simulate stage — would mislead a harness).
    for applet in ["vcmp", "velab", "vrun"] {
        let d =
            std::env::temp_dir().join(format!("vita_obs_staged_{}_{applet}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("t.sv");
        std::fs::write(&f, PASS_SV).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(applet) // explicit `vita <applet>` multicall form
            .arg(f.to_str().unwrap())
            .arg("--obs-dir")
            .arg(d.join("obsout").to_str().unwrap())
            .current_dir(&d)
            .output()
            .expect("run vita");
        assert_ne!(
            out.status.code(),
            Some(0),
            "`{applet} --obs-dir` must be loud-rejected"
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("--obs-dir") && err.contains(applet),
            "`{applet}` reject message wrong:\n{err}"
        );
        assert!(
            !d.join("obsout").join("run.json").exists(),
            "`{applet}` must write no obs"
        );
    }
}

#[test]
fn empty_obs_dir_is_rejected() {
    // Adversarial find (fixed): `--obs-dir ""` (e.g. a `--obs-dir $UNSET` slip)
    // would write into the CWD — reject it loud instead.
    let d = std::env::temp_dir().join(format!("vita_obs_empty_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, PASS_SV).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg("")
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_ne!(
        out.status.code(),
        Some(0),
        "empty --obs-dir must be rejected"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("non-empty"));
    // Nothing leaked into the run dir.
    assert!(!d.join("run.json").exists());
}

#[test]
fn no_obs_flag_writes_nothing() {
    // Without --obs-dir the run is byte-identical to before (pure no-op). Run
    // directly (the `run` helper always passes --obs-dir, so bypass it).
    let d = std::env::temp_dir().join(format!("vita_obs_noflag_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, PASS_SV).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_eq!(out.status.code(), Some(0));
    // No obs artifacts anywhere in the run dir.
    assert!(!d.join("run.json").exists());
    assert!(!d.join("results.jsonl").exists());
    assert!(String::from_utf8_lossy(&out.stdout).contains("x=5"));
}

// ── OBS-1b: coverage.json (R-L5) ────────────────────────────────────────────
// Teeth: 3-WAY — the coverage.json overall percent equals `c.get_coverage()`
// printed by `$display("%f", …)` (both derive from the SAME final hit-bitmaps +
// the SAME weighted-average formula). Plus a determinism golden.

/// The group-level overall `coverage_pct` (the first one in the file — it sits on
/// the `{"instance": …}` line, before the per-item coverpoints).
fn overall_pct(json: &str) -> String {
    json.split("\"coverage_pct\": ")
        .nth(1)
        .and_then(|s| s.split(['}', ',', ']', '\n']).next())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Extract `cov=<f>` printed by the RTL's `$display("cov=%f", c.get_coverage())`.
fn rtl_cov(stdout: &str) -> String {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("cov="))
        .unwrap_or("")
        .to_string()
}

#[test]
fn coverage_json_matches_get_coverage() {
    // 3 bins, 2 hit → 66.666667. coverage.json overall == get_coverage $display.
    let src = "module m;\n\
         logic [1:0] x;\n\
         covergroup cg; cp: coverpoint x { bins lo={0}; bins mid={1,2}; bins hi={3}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample(); x=1; c.sample();\n\
           $display(\"cov=%f\", c.get_coverage()); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    let cj = read(&obs.join("coverage.json"));
    assert_eq!(rtl_cov(&out), overall_pct(&cj), "3-way mismatch\n{cj}");
    assert!(cj.contains("\"covered_bins\": 2"), "per-item detail\n{cj}");
    assert!(cj.contains("\"num_bins\": 3"), "{cj}");
}

#[test]
fn coverage_json_cross_included_in_overall() {
    // ca 100 + cb 100 + cross 50, /3 = 83.333333. The cross MUST be in the overall
    // (else it disagrees with get_coverage).
    let src = "module m;\n\
         logic a, b;\n\
         covergroup cg;\n\
           ca: coverpoint a { bins z={0}; bins o={1}; }\n\
           cb: coverpoint b { bins z={0}; bins o={1}; }\n\
           cx: cross ca, cb;\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=0; c.sample(); a=1;b=1; c.sample();\n\
           $display(\"cov=%f\", c.get_coverage()); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    let cj = read(&obs.join("coverage.json"));
    assert_eq!(
        rtl_cov(&out),
        overall_pct(&cj),
        "cross 3-way mismatch\n{cj}"
    );
    assert!(
        cj.contains("\"kind\": \"cross\""),
        "cross item present\n{cj}"
    );
}

#[test]
fn coverage_json_zero_hits() {
    // Never sampled → 0.000000, covered_bins 0.
    let src = "module m;\n\
         logic [1:0] x;\n\
         covergroup cg; cp: coverpoint x { bins lo={0}; bins hi={3}; } endgroup\n\
         cg c = new;\n\
         initial begin $display(\"cov=%f\", c.get_coverage()); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    let cj = read(&obs.join("coverage.json"));
    assert_eq!(rtl_cov(&out), overall_pct(&cj), "{cj}");
    assert_eq!(overall_pct(&cj), "0.000000", "{cj}");
    assert!(cj.contains("\"covered_bins\": 0"), "{cj}");
}

#[test]
fn coverage_json_weighted() {
    // cp1 weight 3 (100%), cp2 weight 1 (0%) → (3*100 + 1*0)/4 = 75.000000.
    let src = "module m;\n\
         logic p, q;\n\
         covergroup cg;\n\
           c1: coverpoint p { bins z={0}; option.weight=3; }\n\
           c2: coverpoint q { bins z={0}; bins o={1}; option.weight=1; }\n\
         endgroup\n\
         cg c = new;\n\
         initial begin p=0; q=0; c.sample();\n\
           $display(\"cov=%f\", c.get_coverage()); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    let cj = read(&obs.join("coverage.json"));
    assert_eq!(
        rtl_cov(&out),
        overall_pct(&cj),
        "weighted 3-way mismatch\n{cj}"
    );
}

#[test]
fn coverage_json_determinism() {
    let src = "module m;\n\
         logic [1:0] x;\n\
         covergroup cg; cp: coverpoint x { bins lo={0}; bins mid={1,2}; bins hi={3}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample(); x=3; c.sample(); #1 $finish; end\n\
         endmodule\n";
    let (_o1, c1, obs1) = run(src, &[]);
    let (_o2, c2, obs2) = run(src, &[]);
    assert_eq!(c1, 0);
    assert_eq!(c2, 0);
    assert_eq!(
        read(&obs1.join("coverage.json")),
        read(&obs2.join("coverage.json")),
        "coverage.json must be byte-identical across runs"
    );
}

#[test]
fn no_covergroup_no_coverage_json() {
    // A design with no covergroups writes run.json but NOT coverage.json.
    let src = "module m; initial begin $display(\"hi\"); #1 $finish; end endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(obs.join("run.json").exists(), "run.json still written");
    assert!(
        !obs.join("coverage.json").exists(),
        "no covergroups ⇒ no coverage.json"
    );
}
