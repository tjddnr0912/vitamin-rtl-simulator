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
    // START CLEAN. `n` restarts at 0 in every test PROCESS (nextest runs one per
    // test) and the OS recycles PIDs, so two runs can land on the same directory —
    // and nothing here removes it. Measured: `compile_error_writes_no_obs`, whose
    // whole assertion is "no run.json exists", failed once in a full-suite run and
    // passed in isolation, because a PREVIOUS process had left one there.
    let _ = std::fs::remove_dir_all(&d);
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
    assert_eq!(field(&manifest, "format_version"), "26");
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

// ── OBS-2: trace.jsonl (R-L3) — `--probe` net change-stream ─────────────────
// Teeth: 3-WAY — the trace value timeline == the same net's VCD/`$monitor`
// timeline (both derive from the SAME per-change event). Probe typo = loud.

/// Parse `trace.jsonl` into `(t, new_binary)` pairs for a given path.
fn trace_pairs(json: &str, path: &str) -> Vec<(u64, String)> {
    let needle = format!("\"path\":\"{path}\"");
    json.lines()
        .filter(|l| l.contains(&needle))
        .filter_map(|l| {
            let t = l.split("\"t\":").nth(1)?.split(',').next()?.parse().ok()?;
            let nv = l.split("\"new\":\"").nth(1)?.split('"').next()?.to_string();
            Some((t, nv))
        })
        .collect()
}

#[test]
fn trace_jsonl_change_stream() {
    // A 4-bit counter: t0 x→0, then 0→1→2→5. Change-only, MSB..LSB binary.
    let src = "module m;\n\
         logic [3:0] cnt;\n\
         initial begin cnt=0; #1 cnt=1; #1 cnt=2; #1 cnt=5; #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &["--probe", "m.cnt"]);
    assert_eq!(code, 0, "{out}");
    let pairs = trace_pairs(&read(&obs.join("trace.jsonl")), "m.cnt");
    assert_eq!(
        pairs,
        vec![
            (0, "0000".into()),
            (1, "0001".into()),
            (2, "0010".into()),
            (3, "0101".into()),
        ]
    );
}

#[test]
fn trace_jsonl_3way_matches_monitor() {
    // The trace (binary→decimal) timeline == `$monitor` (decimal) timeline.
    let src = "module m;\n\
         logic [3:0] cnt;\n\
         initial begin $monitor(\"MON %0t %0d\", $time, cnt);\n\
           cnt=0; #1 cnt=3; #1 cnt=10; #1 cnt=15; #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &["--probe", "m.cnt"]);
    assert_eq!(code, 0, "{out}");
    // $monitor lines → (t, dec)
    let mon: Vec<(u64, u32)> = out
        .lines()
        .filter_map(|l| l.strip_prefix("MON "))
        .filter_map(|r| {
            let mut it = r.split_whitespace();
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
        })
        .collect();
    // trace (t, binary→dec)
    let tr: Vec<(u64, u32)> = trace_pairs(&read(&obs.join("trace.jsonl")), "m.cnt")
        .iter()
        .map(|(t, b)| (*t, u32::from_str_radix(b, 2).unwrap()))
        .collect();
    assert_eq!(tr, mon, "trace timeline must equal $monitor timeline");
}

#[test]
fn trace_probe_typo_is_loud() {
    // An unresolved --probe path is a loud error (exit != 0), never a silent skip.
    let src = "module m; logic x; initial begin x=0; #1 $finish; end endmodule\n";
    let (_o, code, obs) = run(src, &["--probe", "m.nonexistent"]);
    assert_ne!(code, 0, "probe typo must be loud");
    assert!(!obs.join("trace.jsonl").exists());
}

#[test]
fn trace_probe_without_obs_dir_is_loud() {
    // `--probe` needs `--obs-dir` (the trace.jsonl target).
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_obs_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module m; logic x; initial begin x=0; #1 $finish; end endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .args(["--probe", "m.x"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_ne!(
        out.status.code(),
        Some(0),
        "--probe without --obs-dir is loud"
    );
}

#[test]
fn trace_determinism() {
    let src = "module m;\n\
         logic [3:0] cnt;\n\
         initial begin cnt=0; #1 cnt=7; #1 cnt=8; #1 $finish; end\n\
         endmodule\n";
    let (_o1, c1, obs1) = run(src, &["--probe", "m.cnt"]);
    let (_o2, c2, obs2) = run(src, &["--probe", "m.cnt"]);
    assert_eq!(c1, 0);
    assert_eq!(c2, 0);
    assert_eq!(
        read(&obs1.join("trace.jsonl")),
        read(&obs2.join("trace.jsonl")),
        "trace.jsonl byte-identical across runs"
    );
}

#[test]
fn no_probe_no_trace_jsonl() {
    let src = "module m; logic x; initial begin x=0; #1 $finish; end endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(obs.join("run.json").exists());
    assert!(
        !obs.join("trace.jsonl").exists(),
        "no --probe ⇒ no trace.jsonl"
    );
}

#[test]
fn trace_probe_unpacked_array_is_loud() {
    // An unpacked-array probe target would under-report (element 0 only) — v1 loud-
    // rejects it (per-element probing is a follow-on), never a silent partial trace.
    let src = "module m;\n\
         reg [7:0] mem [0:3];\n\
         initial begin mem[0]=1; mem[1]=2; #1 $finish; end\n\
         endmodule\n";
    let (_o, code, obs) = run(src, &["--probe", "m.mem"]);
    assert_ne!(code, 0, "unpacked-array probe must be loud");
    assert!(!obs.join("trace.jsonl").exists());
}

#[test]
fn trace_probe_packed_multidim_full_width() {
    // A packed multi-dim net (array_len==1) traces its FULL width, not element 0.
    let src = "module m;\n\
         reg [1:0][7:0] p;\n\
         initial begin p=16'hABCD; #1 p=16'h1234; #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &["--probe", "m.p"]);
    assert_eq!(code, 0, "{out}");
    let tj = read(&obs.join("trace.jsonl"));
    assert!(
        tj.contains("\"new\":\"1010101111001101\""),
        "ABCD full 16-bit\n{tj}"
    );
    assert!(
        tj.contains("\"new\":\"0001001000110100\""),
        "1234 full 16-bit\n{tj}"
    );
}

#[test]
fn trace_probe_real_is_loud() {
    // A `real`/`realtime` net has array_len==1 but f64 storage — the whole-net
    // formatter would emit the raw IEEE-754 bit pattern (≠ VCD `r1.5` / `$monitor`).
    // So loud-reject it (real probing is a follow-on), never a silent bit-pattern.
    let src = "module m;\n\
         real r;\n\
         initial begin r=1.5; #1 r=2.5; #1 $finish; end\n\
         endmodule\n";
    let (_o, code, obs) = run(src, &["--probe", "m.r"]);
    assert_ne!(code, 0, "real probe must be loud");
    assert!(!obs.join("trace.jsonl").exists());
}

// ── OBS-3: stage.jsonl (R-S3) — $vita_stage vendor stage-trace task ──────────
// Teeth: 3-WAY (stage vals == parallel `$display %0d`) + no-op without +STAGE_TRACE.

#[test]
fn stage_jsonl_capture() {
    let src = "module m;\n\
         logic [3:0] st;\n\
         initial begin st=1; $vita_stage(\"init\", st);\n\
           #1 st=5; $vita_stage(\"run\", st, 42); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &["+STAGE_TRACE"]);
    assert_eq!(code, 0, "{out}");
    let sj = read(&obs.join("stage.jsonl"));
    assert!(
        sj.contains("\"label\":\"init\",\"idx\":0,\"vals\":[\"1\"]"),
        "{sj}"
    );
    assert!(
        sj.contains("\"label\":\"run\",\"idx\":1,\"vals\":[\"5\",\"42\"]"),
        "{sj}"
    );
    assert!(sj.contains("\"t\":1,"), "time recorded\n{sj}");
}

#[test]
fn stage_3way_matches_display() {
    // stage vals == parallel `$display("%0d",…)` (incl. signed + x).
    let src = "module m;\n\
         logic [7:0] a; logic signed [7:0] b; logic [3:0] x;\n\
         initial begin a=200; b=-5; x=4'bxx01;\n\
           $display(\"D %0d %0d %0d\", a, b, x);\n\
           $vita_stage(\"s\", a, b, x); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &["+STAGE_TRACE"]);
    assert_eq!(code, 0, "{out}");
    // parse $display "D <a> <b> <x>"
    let disp: Vec<String> = out
        .lines()
        .find_map(|l| l.strip_prefix("D "))
        .map(|r| r.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    let sj = read(&obs.join("stage.jsonl"));
    let vals: Vec<String> = sj
        .split("\"vals\":[")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .unwrap_or("")
        .split(',')
        .map(|t| t.trim_matches('"').to_string())
        .collect();
    assert_eq!(vals, disp, "stage vals must equal $display %0d\n{sj}");
}

/// Slice #6 ABSOLUTE ANCHOR — the stage rail on TIER-3.
///
/// The `stage` design row said the G2 rails "ride the interpreter's change
/// hooks", and for `--probe` that is true. `$vita_stage` is not a change hook at
/// all: elaborate lowers it to a no-op `Display` plus a StmtId, and the rail's
/// state lives in `SimState`, which both kernels borrow. What was store-bound
/// was `run_vita_stage`'s two argument reads (a bare `sched.eval`).
///
/// Every part of this design is the discriminator: the LABEL comes from a net
/// (a literal label is a constant and would agree either way), so does each
/// VALUE, and both move between the two calls. An unthreaded read records the
/// ENGINE's untouched slots — a `stage.jsonl` that is silently wrong rather
/// than absent, which is the failure mode A7's `coverage.json` had.
#[test]
fn stage_jsonl_on_tier_3() {
    let src = "module top;\n\
         reg [7:0] a = 8'd0; reg [15:0] w = 16'd0; reg [63:0] lbl = \"phaseA\";\n\
         initial begin a = 8'd5; w = 16'd300;\n\
           $vita_stage(\"start\", a, w);\n\
           #1 a = 8'd9; w = 16'hBEEF;\n\
           $vita_stage(lbl, a, w);\n\
           #1 $vita_stage(\"end\"); $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &["+STAGE_TRACE", "--backend", "native"]);
    assert_eq!(code, 0, "{out}");
    // ANTI-VACUITY: a refused design falls back to the VM, and this test would
    // then be a second copy of `stage_jsonl_capture`.
    let rj = read(&obs.join("run.json"));
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}"
    );
    assert_eq!(
        read(&obs.join("stage.jsonl")),
        "{\"v\":1,\"t\":0,\"kind\":\"stage\",\"label\":\"start\",\"idx\":0,\"vals\":[\"5\",\"300\"]}\n\
         {\"v\":1,\"t\":1,\"kind\":\"stage\",\"label\":\"phaseA\",\"idx\":1,\"vals\":[\"9\",\"48879\"]}\n\
         {\"v\":1,\"t\":2,\"kind\":\"stage\",\"label\":\"end\",\"idx\":2,\"vals\":[]}\n",
        "tier-3 stage.jsonl"
    );
}

#[test]
fn stage_no_plusarg_is_noop() {
    // Without +STAGE_TRACE: no stage.jsonl AND no stdout leak ($vita_stage suppressed).
    let src = "module m;\n\
         logic [3:0] st;\n\
         initial begin st=7; $vita_stage(\"x\", st); #1 $finish; end\n\
         endmodule\n";
    let (out, code, obs) = run(src, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        !obs.join("stage.jsonl").exists(),
        "no +STAGE_TRACE ⇒ no stage.jsonl"
    );
    assert!(
        !out.contains('x'),
        "no $vita_stage text leaks to stdout\n{out}"
    );
}

#[test]
fn stage_determinism() {
    let src = "module m;\n\
         logic [3:0] st;\n\
         initial begin st=1; $vita_stage(\"a\", st); #1 st=8; $vita_stage(\"b\", st); #1 $finish; end\n\
         endmodule\n";
    let (_o1, c1, o1) = run(src, &["+STAGE_TRACE"]);
    let (_o2, c2, o2) = run(src, &["+STAGE_TRACE"]);
    assert_eq!(c1, 0);
    assert_eq!(c2, 0);
    assert_eq!(read(&o1.join("stage.jsonl")), read(&o2.join("stage.jsonl")));
}

#[test]
fn stage_zero_arg_is_loud() {
    let src = "module m; initial begin $vita_stage(); #1 $finish; end endmodule\n";
    let (_o, code, _obs) = run(src, &["+STAGE_TRACE"]);
    assert_ne!(code, 0, "$vita_stage() with no label must be loud");
}

#[test]
fn stage_plusarg_value_form_enables() {
    // `+STAGE_TRACE=1` (the conventional `+name=value` form) enables capture, but a
    // prefix neighbour `+STAGE_TRACEX` does NOT (exact/`=`-prefixed match only).
    let src = "module m; logic [3:0] s;\n\
         initial begin s=1; $vita_stage(\"a\", s); #1 $finish; end endmodule\n";
    let (_o, c1, obs1) = run(src, &["+STAGE_TRACE=1"]);
    assert_eq!(c1, 0);
    assert!(
        obs1.join("stage.jsonl").exists(),
        "+STAGE_TRACE=1 enables capture"
    );
    let (_o2, c2, obs2) = run(src, &["+STAGE_TRACEX"]);
    assert_eq!(c2, 0);
    assert!(
        !obs2.join("stage.jsonl").exists(),
        "+STAGE_TRACEX must NOT enable"
    );
}

// ── T0 + S0 (doc-21 §7.3): the tier instruments in run.json ────────────────────

/// T0: the `codegen` object pins the ②층 VM's claim on the design AND why the
/// rest was refused — before this the only observation was a `--backend interp`
/// vs `bytecode` A/B timing run, which is how a design with exactly 0% VM
/// contribution (`bench/keccak` 호출형, round-26) went unnoticed. This design
/// reproduces that shape in miniature: full of work, `frame_bodies > 0`, and
/// the caller process refused for `user_call_in_expr` — so the report must say
/// so, with exact counts (a wrong log is a silent-wrong, doc-19 §3).
#[test]
fn run_json_codegen_pins_the_vm_claim_and_reasons() {
    let (_, code, obs) = run(
        "module top;\n\
           function automatic int add1(input int x); return x + 1; endfunction\n\
           reg clk = 0; int v = 0;\n\
           always #1 clk = ~clk;\n\
           always @(posedge clk) v = add1(v);\n\
           initial begin #10 $display(\"v=%0d\", v); $finish; end\n\
         endmodule\n",
        &[],
    );
    assert_eq!(code, 0);
    let manifest = read(&obs.join("run.json"));
    assert_eq!(
        field(&manifest, "codegen"),
        "{\"able\": 1, \"total\": 4, \"frame_bodies\": 1, \
         \"reject_reasons\": {\"delay\": 2, \"user_call_in_expr\": 1}}",
        "full manifest:\n{manifest}"
    );
    // S0: framed user calls are CORE (rev-4: S3 absorbs T1/T2), and #delay is
    // the v1 target's normal TB shape — so this design is within v1's SCOPE.
    // It is buildable too since S3a: `add1`'s body names only its own frame
    // slots, which is the subset the tier-3 store can serve by delegating to the
    // engine's frame executor. (This read `buildable: false, refused:
    // "frame-local storage: S3 (subroutine frames)"` before that slice. The
    // scope-vs-storage split is pinned on a still-refused shape in
    // `backend_flag::the_native_verdict_reports_scope_and_storage_separately`.)
    assert_eq!(
        field(&manifest, "native"),
        "{\"eligible\": true, \"buildable\": true, \"refused\": null, \"reject_reasons\": {}}",
        "full manifest:\n{manifest}"
    );
    // The effective executor is recorded next to the census (soundness F1):
    // `codegen` is a STATIC capability claim, and without this field an
    // `--backend interp` run's `able` rows read as "the VM ran this".
    assert_eq!(field(&manifest, "backend"), "\"vm\"", "{manifest}");
}

/// The `codegen` object is a static property of the DESIGN — selecting the
/// interpreter must not change one byte of it (it is a capability census, not
/// an execution log). What DOES change is the `backend` field, which is what
/// keeps the census from being misread on an interp-forced run.
#[test]
fn run_json_codegen_is_backend_invariant_and_backend_is_recorded() {
    let (_, c1, obs1) = run(PASS_SV, &[]);
    let (_, c2, obs2) = run(PASS_SV, &["--backend", "interp"]);
    let (_, c3, obs3) = run(PASS_SV, &["--backend", "native"]);
    assert_eq!((c1, c2, c3), (0, 0, 0));
    let m1 = read(&obs1.join("run.json"));
    let m2 = read(&obs2.join("run.json"));
    let m3 = read(&obs3.join("run.json"));
    assert_eq!(field(&m1, "backend"), "\"vm\"");
    assert_eq!(field(&m2, "backend"), "\"interp\"");
    // ③층 (S1d-4c-2c): requested AND honored. This assertion used to read
    // `"vm"`, and the change is the slice: `PASS_SV` has no continuous assign,
    // no in-body waiter and no refused system task, so all three gate layers
    // pass and the tier-3 run loop executes it. Updated rather than deleted —
    // the old value encoded "there is no native executor yet", which is exactly
    // what stopped being true. The fall-back PAIR is still asserted, on a design
    // that is genuinely refused, in `run_json_reports_native_fallback` below.
    assert_eq!(field(&m3, "backend"), "\"native\"", "{m3}");
    assert_eq!(field(&m3, "backend_requested"), "\"native\"", "{m3}");
    // …and running it natively must not change one byte of what it printed.
    assert_eq!(
        std::fs::read_to_string(obs1.join("results.jsonl")).unwrap_or_default(),
        std::fs::read_to_string(obs3.join("results.jsonl")).unwrap_or_default(),
        "the native run's ledger differs from the VM's"
    );
    assert_eq!(field(&m1, "backend_requested"), "\"vm\"");
    assert_eq!(field(&m3, "native"), field(&m1, "native"), "verdict moved");
    assert_eq!(field(&m3, "codegen"), field(&m1, "codegen"), "census moved");
    assert_eq!(
        field(&m1, "codegen"),
        field(&m2, "codegen"),
        "the census must not depend on the selected executor\n{m1}\n{m2}"
    );
    assert_eq!(field(&m1, "native"), field(&m2, "native"));
}

/// The FALL-BACK half of the pair above, on a design that is genuinely refused.
///
/// `--backend native` may not be silently ignored, and it may not be silently
/// honored either: run.json has to carry both what was asked and what ran. A
/// `$monitor` is the refusal used here because it is one the run gate adds on
/// top of design eligibility (the kernel's systask refusal, S1d-4b).
/// Note `native.eligible` stays TRUE, which is the whole point of the layering:
/// the design is within v1's scope, today's executor just cannot run it.
///
/// The design here has changed SIX times as the executor grew: a plain `assign`
/// until S1d-4d-1, `assign #2` until S1d-4d-3 wired the inertial wheel, a
/// multi-driven net until S1d-4d-4 wired the group resolution, a `$monitor`
/// until A5-b gave tier-3 a postponed region, `$writememh` until slice #8
/// threaded its three reads. It is now `$dumpall`.
///
/// ⚠️ That churn IS the test working. Its claim is about the SHAPE of the
/// report — an executor-layer refusal must publish `eligible: true` beside
/// `backend: "vm"` — so every time the executor grows, the design has to be
/// re-picked or the test starts asserting that shape about a design that runs.
///
/// ⚠️⚠️ And the well is nearly dry: slice #8's census measured that
/// `$writemem*` were the last refused system tasks ANY design in the suite
/// reached, so `$dumpall`/`$dumpon` is the whole remaining population of this
/// executor row and it has to be spelled by hand.
#[test]
fn run_json_reports_native_fallback_on_a_refused_design() {
    // ⚠️⚠️ SEVENTH shape, and the well the note above predicted has now run dry:
    // A5-dumpall wired `$dumpall`/`$dumpon`, so `systask_refusal` is EMPTY and
    // that executor row can no longer refuse anything at all.
    //
    // What is left on the executor layer is the `wait fork` row — and it is the
    // one shape that reaches it while staying ELIGIBLE, because a bare
    // `wait fork;` populates no `fork_modes` entry and so is invisible to the S0
    // `fork` row. That is stated in `native::run::executor_rows` itself, and it
    // is why this test still has a subject.
    const WAITFORK_SV: &str = "module top; reg [7:0] n = 8'd0;\n\
         initial begin n = 8'd1; wait fork; $display(\"n=%0d\", n); #1 $finish; end endmodule\n";
    let (_, code, obs) = run(WAITFORK_SV, &["--backend", "native"]);
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    assert_eq!(field(&m, "backend_requested"), "\"native\"", "{m}");
    assert_eq!(field(&m, "backend"), "\"vm\"", "{m}");
    assert!(
        field(&m, "native").contains("\"eligible\": true"),
        "a bare `wait fork` design is within v1's SCOPE — the refusal is the \
         executor's, not the gate's:\n{m}"
    );
}

/// S0: the `native` object pins the ③층 design-level verdict. TWO distinct
/// reject families in one design, whose SOURCES differ — a sidecar table entry
/// (`fork`, from `fork_modes`) and a statement scan (`disable_fork`) — which is
/// the claim worth pinning; the units are per-family counts, so each is exactly
/// 1, and `refused` reports the first in the map's byte order.
///
/// ⚠️ THIRD shape. It was fork + queue + string until V1 slice 2 admitted every
/// heap kind, then fork + `real` until A6 admitted that.
///
/// ⚠️⚠️ A6 also retired the SOURCE this pin used to contrast with: the design
/// gate's net-KIND loop now has no rejecting arm at all — every `NetKind` is
/// core or admitted — so "a net-table kind" is no longer a family any design can
/// produce. The remaining families are all sidecar- or statement-sourced, and
/// this pair is picked to still span two of those sources rather than two
/// spellings of one.
#[test]
fn run_json_native_pins_the_reject_families() {
    let (_, code, obs) = run(
        "module top;\n\
           integer n;\n\
           initial begin\n\
             fork begin n = 1; end begin n = 2; end join\n\
             disable fork;\n\
             $display(\"%0d\", n);\n\
             $finish;\n\
           end\n\
         endmodule\n",
        &[],
    );
    assert_eq!(code, 0);
    let manifest = read(&obs.join("run.json"));
    assert_eq!(
        field(&manifest, "native"),
        "{\"eligible\": false, \"buildable\": true, \"refused\": \"disable_fork\", \
         \"reject_reasons\": {\"disable_fork\": 1, \"fork\": 1}}",
        "full manifest:\n{manifest}"
    );
}

/// A8-probe ABSOLUTE ANCHOR — the G2 probe rail on TIER-3.
///
/// The design row that refused `--probe` said something true and kept being
/// true: `emit_probe_change` is called from the engine's `note_change`, i.e. the
/// rail rides a hook tier-3 does not have. (Slice #6 measured that this is
/// exactly what separated it from `stage`, which shared the row's comment and
/// rode no hook at all.) What made it cheap anyway is that everything ELSE is on
/// `SimState` — `probed`, `probe_prev`, `trace_lines`, `net_names` — so only the
/// VALUE was store-bound, and tier-3 already had a store-point capture for VCD.
///
/// Every line here is a discriminator:
///   * `v` is written 1 → 2 → 1 inside ONE time slot, so a capture that re-read
///     the value at drain time would emit `1,1,1` instead of `1,2,1`. This is
///     the whole reason the emitter cannot live at sweep time, and the VCD
///     queue's argument verbatim.
///   * `same` is rewritten with the value it already holds, which must emit
///     NOTHING — the dedup lives in shared `probe_prev`, so this pins that the
///     native path reaches it rather than keeping a second one.
///   * both nets emit a t=0 record for their declaration initialiser, and `v`
///     emits again at t=1, so the `"t"` stamp is exercised across a time move.
///
/// ⚠️ ANTI-VACUITY: run.json must say the run was native. An unwired probe rail
/// does not crash — it writes the t0 lines and nothing after, at exit 0, which
/// is a G2 artifact that is present and wrong.
///
/// ⚠️ No iverilog oracle: `trace.jsonl` is vita's own G2 format. The pin is
/// absolute (the exact lines), which is what the format contract deserves.
#[test]
fn trace_probe_on_tier_3_captures_at_the_store_point() {
    let src = "module top;\n\
                 reg [7:0] v = 0;\n\
                 reg [7:0] same = 8'd3;\n\
                 initial begin\n\
                   v = 8'd1;\n\
                   v = 8'd2;\n\
                   v = 8'd1;\n\
                   same = 8'd3;\n\
                   #1 v = 8'd9;\n\
                   #1 $finish;\n\
                 end\n\
               endmodule\n";
    let (_o, code, obs) = run(
        src,
        &[
            "--backend",
            "native",
            "--probe",
            "top.v",
            "--probe",
            "top.same",
        ],
    );
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    assert_eq!(field(&m, "backend"), "\"native\"", "{m}");
    let trace = read(&obs.join("trace.jsonl"));
    assert_eq!(
        trace,
        "{\"v\":1,\"t\":0,\"kind\":\"chg\",\"path\":\"top.v\",\"old\":\"xxxxxxxx\",\"new\":\"00000000\"}\n\
         {\"v\":1,\"t\":0,\"kind\":\"chg\",\"path\":\"top.same\",\"old\":\"xxxxxxxx\",\"new\":\"00000011\"}\n\
         {\"v\":1,\"t\":0,\"kind\":\"chg\",\"path\":\"top.v\",\"old\":\"00000000\",\"new\":\"00000001\"}\n\
         {\"v\":1,\"t\":0,\"kind\":\"chg\",\"path\":\"top.v\",\"old\":\"00000001\",\"new\":\"00000010\"}\n\
         {\"v\":1,\"t\":0,\"kind\":\"chg\",\"path\":\"top.v\",\"old\":\"00000010\",\"new\":\"00000001\"}\n\
         {\"v\":1,\"t\":1,\"kind\":\"chg\",\"path\":\"top.v\",\"old\":\"00000001\",\"new\":\"00001001\"}\n",
        "the glitch must survive as three records and the same-value write as none"
    );
}
