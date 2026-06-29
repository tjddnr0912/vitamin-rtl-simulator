//! P2-4: `--help`/`--version` first-impression UX. Before this fix
//! `vita --help` tried to READ a file named `--help` (exit 3, confusing error).
use std::process::Command;

fn vita(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .output()
        .expect("run vita")
}

#[test]
fn help_prints_usage_and_exits_zero() {
    let out = vita(&["--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "stdout:\n{stdout}");
    assert!(stdout.contains("Usage"), "got:\n{stdout}");
    assert!(stdout.contains("vita"), "got:\n{stdout}");
}

#[test]
fn short_help_flag_works() {
    let out = vita(&["-h"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage"));
}

#[test]
fn version_prints_and_exits_zero() {
    let out = vita(&["--version"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "got:\n{stdout}");
}

#[test]
fn staged_applet_help_via_subcommand() {
    let out = vita(&["vrun", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "got:\n{stdout}");
    assert!(stdout.contains("Usage"), "got:\n{stdout}");
    assert!(stdout.contains("vrun"), "got:\n{stdout}");
}

// ── P4-T1: `--threads N` / `-j N` / VITA_THREADS ────────────────────────────

fn write_tmp(name: &str, body: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("vita_ux_{}_{name}", std::process::id()));
    std::fs::write(&p, body).unwrap();
    p
}

const DUMP_TB: &str = r#"
module tb;
  reg clk; reg [15:0] n; integer k;
  always @(posedge clk) n <= n + 16'd1;
  initial begin
    $dumpfile("d.vcd"); $dumpvars;
    clk = 0; n = 0;
    for (k = 0; k < 200; k = k + 1) begin #1 clk = 1; #1 clk = 0; end
    $finish;
  end
endmodule
"#;

/// `--threads 1` vs `--threads 4`: accepted on the one-shot applet, and the
/// VCD artifact is byte-identical (the P4 contract).
#[test]
fn threads_flag_byte_identical_vcd() {
    let src = write_tmp("thr.sv", DUMP_TB);
    let v1 = std::env::temp_dir().join(format!("vita_ux_{}_t1.vcd", std::process::id()));
    let v4 = std::env::temp_dir().join(format!("vita_ux_{}_t4.vcd", std::process::id()));
    let o1 = vita(&[
        src.to_str().unwrap(),
        "-o",
        v1.to_str().unwrap(),
        "--threads",
        "1",
    ]);
    let o4 = vita(&[src.to_str().unwrap(), "-o", v4.to_str().unwrap(), "-j", "4"]);
    assert_eq!(
        o1.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&o1.stderr)
    );
    assert_eq!(
        o4.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&o4.stderr)
    );
    let b1 = std::fs::read(&v1).expect("threads=1 VCD");
    let b4 = std::fs::read(&v4).expect("threads=4 VCD");
    assert!(b1.len() > 500, "real VCD expected");
    assert_eq!(b1, b4, "VCD must be byte-identical across thread counts");
    for p in [&src, &v1, &v4] {
        let _ = std::fs::remove_file(p);
    }
}

/// An unparsable `--threads` value is a CLI usage error (exit 3).
#[test]
fn threads_invalid_value_exits_three() {
    let src = write_tmp("thrbad.sv", "module t; endmodule");
    let out = vita(&[src.to_str().unwrap(), "--threads", "abc"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(3));
}

/// `VITA_THREADS` env applies when no flag is given (flag wins otherwise).
#[test]
fn threads_env_accepted() {
    let src = write_tmp("threnv.sv", DUMP_TB);
    let v = std::env::temp_dir().join(format!("vita_ux_{}_env.vcd", std::process::id()));
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .args([src.to_str().unwrap(), "-o", v.to_str().unwrap()])
        .env("VITA_THREADS", "4")
        .output()
        .expect("run vita");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(v.exists(), "VCD written under VITA_THREADS=4");
    for p in [&src, &v] {
        let _ = std::fs::remove_file(p);
    }
}

// ── P2-9: `--timeout <ticks>` CI killswitch ─────────────────────────────────

/// A design that never `$finish`es is bounded by `--timeout` (clean exit 0).
#[test]
fn timeout_bounds_endless_design() {
    let src = write_tmp(
        "tmo.sv",
        "module t; reg clk; initial clk = 0; always #1 clk = ~clk; endmodule",
    );
    let out = vita(&[src.to_str().unwrap(), "--timeout", "200"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn timeout_invalid_value_exits_three() {
    let src = write_tmp("tmobad.sv", "module t; endmodule");
    let out = vita(&[src.to_str().unwrap(), "--timeout", "soon"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(3));
}

// ── P2-12: defensive clamp of the VCD `$timescale` unit renderer ─────────────

#[test]
fn timescale_unit_string_clamps_to_vcd_range() {
    // VCD admits only 1|10|100 × s..fs (exp ∈ [-15, +2]). Out-of-range
    // exponents must saturate, never misrender (old fallback: -16 → "100s").
    assert_eq!(cli::timescale_unit_string(-16), "1fs");
    assert_eq!(cli::timescale_unit_string(3), "100s");
    // Boundaries and representative in-range values keep exact rendering.
    assert_eq!(cli::timescale_unit_string(-15), "1fs");
    assert_eq!(cli::timescale_unit_string(2), "100s");
    assert_eq!(cli::timescale_unit_string(-9), "1ns");
    assert_eq!(cli::timescale_unit_string(-10), "100ps");
}

#[test]
fn out_clobbering_input_dot_spelling_rejected() {
    // P2-12 regression: `-o` naming an input through a different spelling
    // (./x.sv vs x.sv) is caught by canonicalization — exit 3, input intact.
    let src = write_tmp("clob.sv", "module m; endmodule\n");
    let dotted = format!(
        "{}/./{}",
        src.parent().unwrap().display(),
        src.file_name().unwrap().to_string_lossy()
    );
    let out = vita(&["vcmp", &dotted, "-o", src.to_str().unwrap()]);
    let body = std::fs::read_to_string(&src).unwrap();
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(body, "module m; endmodule\n", "input must be untouched");
}

// ── Phase-1.x D: -Wno-<CODE> / -Werror[=<CODE>] gates + exit class 2 ─────────

/// A design that always emits W1017 (no `timescale) and finishes cleanly.
fn warning_design(name: &str) -> std::path::PathBuf {
    write_tmp(name, "module t; initial $finish; endmodule\n")
}

#[test]
fn wno_suppresses_a_warning() {
    let src = warning_design("wno.sv");
    let base = vita(&[src.to_str().unwrap()]);
    let gated = vita(&[src.to_str().unwrap(), "-Wno-W-PP-TIMESCALE-DEFAULT"]);
    let _ = std::fs::remove_file(&src);
    assert!(
        String::from_utf8_lossy(&base.stderr).contains("VITA-W1017"),
        "baseline must warn"
    );
    assert_eq!(gated.status.code(), Some(0));
    assert!(
        !String::from_utf8_lossy(&gated.stderr).contains("VITA-W1017"),
        "suppressed warning must not print"
    );
}

#[test]
fn werror_promotes_warnings_and_fails() {
    let src = warning_design("werr.sv");
    let out = vita(&[src.to_str().unwrap(), "-Werror"]);
    let _ = std::fs::remove_file(&src);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "promoted warning must fail");
    // Rendered as an ERROR but keeps its original (stable) code number.
    assert!(
        err.contains("error[VITA-W1017]"),
        "promotion renders error with the original code, got:\n{err}"
    );
}

#[test]
fn werror_targeted_hits_only_that_code() {
    let src = warning_design("werrt.sv");
    let hit = vita(&[src.to_str().unwrap(), "-Werror=W-PP-TIMESCALE-DEFAULT"]);
    let miss = vita(&[src.to_str().unwrap(), "-Werror=W-PP-MACRO-REDEFINED"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(hit.status.code(), Some(1));
    assert_eq!(
        miss.status.code(),
        Some(0),
        "unrelated promotion must not fire"
    );
}

#[test]
fn gate_flag_with_unknown_mnemonic_is_cli_error() {
    let src = warning_design("wbad.sv");
    let out = vita(&[src.to_str().unwrap(), "-Wno-NOT-A-REAL-CODE"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("VITA-E0001"));
}

#[test]
fn errors_are_never_suppressible() {
    // -Wno- of an Error code is accepted (valid mnemonic) but has no effect:
    // the always-logged spine keeps every Error/Fatal.
    let src = write_tmp(
        "wnoerr.sv",
        "module t; initial x = 1; endmodule\n", // undeclared name → E3010
    );
    let out = vita(&[src.to_str().unwrap(), "-Wno-E-ELAB-UNRESOLVED-NAME"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("VITA-E3010"),
        "errors must still print under -Wno-"
    );
}

#[test]
fn promoted_runtime_warning_fails_run() {
    // doc-13: `-Werror=W-RUN-USER-WARNING` turns RTL $warning into a CI failure
    // without editing the RTL.
    let src = write_tmp(
        "werrrun.sv",
        "`timescale 1ns/1ns\nmodule t; initial begin $warning(\"w\"); $finish; end endmodule\n",
    );
    let ok = vita(&[src.to_str().unwrap()]);
    let promoted = vita(&[src.to_str().unwrap(), "-Werror=W-RUN-USER-WARNING"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(ok.status.code(), Some(0));
    assert_eq!(promoted.status.code(), Some(1));
}

#[test]
fn artifact_gate_failure_exits_class_two() {
    // doc-13 exit table: stale/artifact gates are class 2 — CI re-runs
    // vcmp/velab instead of debugging RTL. Garbage header → magic mismatch.
    let bad = write_tmp("garbage.velab", "this is not a velab artifact\n");
    let out = vita(&["vrun", bad.to_str().unwrap()]);
    let _ = std::fs::remove_file(&bad);
    assert_eq!(out.status.code(), Some(2), "artifact gate must exit 2");
    assert!(String::from_utf8_lossy(&out.stderr).contains("VITA-E9001"));
}

// ── Phase-1.x E: filelist -f/-F expansion (doc-14 §3.1, v1 subset) ───────────

fn tmpdir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vita_flist_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn filelist_expands_sources_and_flags() {
    // A .f can hold sources AND any flag legal on the command line.
    let d = tmpdir("basic");
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ns\nmodule t; initial $finish; endmodule\n",
    )
    .unwrap();
    std::fs::write(
        d.join("build.f"),
        format!(
            "// comment line\n# hash comment\n{} \\\n  --timeout 500\n",
            d.join("t.sv").display()
        ),
    )
    .unwrap();
    let out = vita(&["-f", d.join("build.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn filelist_big_f_resolves_relative_to_file_dir() {
    // -F: relative paths inside resolve against the .f's OWN directory, so the
    // tree is relocatable; a nested -f inside a -F frame warns (mixed base).
    let d = tmpdir("bigf");
    std::fs::create_dir_all(d.join("ip")).unwrap();
    std::fs::write(
        d.join("ip/t.sv"),
        "`timescale 1ns/1ns\nmodule t; initial $finish; endmodule\n",
    )
    .unwrap();
    std::fs::write(d.join("ip/vendor.f"), "t.sv\n").unwrap();
    let out = vita(&["-F", d.join("ip/vendor.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn filelist_mixed_base_warns() {
    let d = tmpdir("mixed");
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ns\nmodule t; initial $finish; endmodule\n",
    )
    .unwrap();
    std::fs::write(d.join("inner.f"), format!("{}\n", d.join("t.sv").display())).unwrap();
    std::fs::write(
        d.join("outer.f"),
        format!("-f {}\n", d.join("inner.f").display()),
    )
    .unwrap();
    let out = vita(&["-F", d.join("outer.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("VITA-W8008"),
        "-f inside a -F frame must warn W-FLIST-MIXED-BASE"
    );
}

#[test]
fn filelist_cycle_is_loud() {
    let d = tmpdir("cycle");
    std::fs::write(d.join("a.f"), format!("-f {}\n", d.join("b.f").display())).unwrap();
    std::fs::write(d.join("b.f"), format!("-f {}\n", d.join("a.f").display())).unwrap();
    let out = vita(&["-f", d.join("a.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("VITA-E8001"));
}

#[test]
fn filelist_env_expansion_and_undefined_are_handled() {
    let d = tmpdir("env");
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ns\nmodule t; initial $finish; endmodule\n",
    )
    .unwrap();
    std::fs::write(d.join("ok.f"), "${VITA_FLIST_DIR}/t.sv\n").unwrap();
    std::fs::write(d.join("bad.f"), "$VITA_FLIST_UNDEFINED_VAR/t.sv\n").unwrap();
    let ok = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["-f", d.join("ok.f").to_str().unwrap()])
        .env("VITA_FLIST_DIR", &d)
        .output()
        .expect("run vita");
    let bad = vita(&["-f", d.join("bad.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        ok.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert_eq!(bad.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("VITA-E8006"));
}

#[test]
fn filelist_glob_and_missing_are_loud() {
    let d = tmpdir("globmiss");
    std::fs::write(d.join("g.f"), "*.sv\n").unwrap();
    let glob = vita(&["-f", d.join("g.f").to_str().unwrap()]);
    let miss = vita(&["-f", d.join("nope.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(glob.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&glob.stderr).contains("VITA-E8004"));
    assert_eq!(miss.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&miss.stderr).contains("VITA-E8005"));
}

#[test]
fn filelist_works_for_staged_vcmp() {
    // The expansion is argv-level, so staged applets accept .f too.
    let d = tmpdir("staged");
    std::fs::write(d.join("t.sv"), "module t; endmodule\n").unwrap();
    std::fs::write(d.join("c.f"), format!("{}\n", d.join("t.sv").display())).unwrap();
    let vu = d.join("out.vu");
    let out = vita(&[
        "vcmp",
        "-f",
        d.join("c.f").to_str().unwrap(),
        "-o",
        vu.to_str().unwrap(),
    ]);
    let made = vu.exists();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(made, ".vu must be written from a filelist-driven vcmp");
}

// ── Phase-1.x F: `vita explain <CODE>` ───────────────────────────────────────

#[test]
fn explain_prints_entry_for_mnemonic() {
    let out = vita(&["explain", "E-ELAB-MULTIDRIVER"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "got:\n{stdout}");
    assert!(stdout.contains("VITA-E3001"), "got:\n{stdout}");
    assert!(
        stdout.contains("E-ELAB-MULTIDRIVER"),
        "entry body expected, got:\n{stdout}"
    );
}

#[test]
fn explain_accepts_grep_number_form() {
    let out = vita(&["explain", "VITA-W1017"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout.contains("W-PP-TIMESCALE-DEFAULT"), "got:\n{stdout}");
}

#[test]
fn explain_unknown_code_exits_three() {
    let out = vita(&["explain", "E-NOT-A-CODE"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("VITA-E0001"));
}

// ── filelist typed buckets: -D/-I, +define+/+incdir+ (doc-14 §3.1) ──────────

#[test]
fn define_flag_reaches_preprocessor() {
    let src = write_tmp(
        "defflag.sv",
        "`timescale 1ns/1ns\nmodule t; reg [`W-1:0] q; initial begin q = {`W{1'b1}}; $display(\"%0d\", q); $finish; end endmodule\n",
    );
    let with = vita(&[src.to_str().unwrap(), "-D", "W=4"]);
    let without = vita(&[src.to_str().unwrap()]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(
        with.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&with.stderr)
    );
    assert!(String::from_utf8_lossy(&with.stdout).contains("15"));
    assert_eq!(without.status.code(), Some(1), "undefined `W must fail");
}

#[test]
fn plus_define_token_with_multiple_names() {
    let src = write_tmp(
        "plusdef.sv",
        "`timescale 1ns/1ns\nmodule t; initial begin $display(\"%0d %0d\", `A, `B); $finish; end endmodule\n",
    );
    let out = vita(&[src.to_str().unwrap(), "+define+A=3+B=9"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("3 9"));
}

#[test]
fn incdir_resolves_include() {
    let d = tmpdir("incdir");
    std::fs::create_dir_all(d.join("inc")).unwrap();
    std::fs::write(d.join("inc/w.svh"), "`define W 6\n").unwrap();
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ns\n`include \"w.svh\"\nmodule t; initial begin $display(\"%0d\", `W); $finish; end endmodule\n",
    )
    .unwrap();
    let flag = vita(&[
        d.join("t.sv").to_str().unwrap(),
        "-I",
        d.join("inc").to_str().unwrap(),
    ]);
    let plus = vita(&[
        d.join("t.sv").to_str().unwrap(),
        &format!("+incdir+{}", d.join("inc").display()),
    ]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        flag.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&flag.stderr)
    );
    assert_eq!(plus.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&flag.stdout).contains("6"));
}

#[test]
fn filelist_carries_defines_and_relative_incdir() {
    // +define+ rides a .f verbatim; +incdir+ RELATIVE paths in a -F frame
    // resolve against the .f's own directory (relocatable vendor tree).
    let d = tmpdir("fbucket");
    std::fs::create_dir_all(d.join("ip/inc")).unwrap();
    std::fs::write(d.join("ip/inc/p.svh"), "`define P 7\n").unwrap();
    std::fs::write(
        d.join("ip/t.sv"),
        "`timescale 1ns/1ns\n`include \"p.svh\"\nmodule t; initial begin $display(\"%0d %0d\", `P, `Q); $finish; end endmodule\n",
    )
    .unwrap();
    std::fs::write(d.join("ip/vendor.f"), "+incdir+inc\n+define+Q=2\nt.sv\n").unwrap();
    let out = vita(&["-F", d.join("ip/vendor.f").to_str().unwrap()]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("7 2"));
}

#[test]
fn wrong_stage_rejects_preprocess_buckets() {
    // velab/vrun have no preprocess pass — a +define+ (from argv or a .f)
    // must be a loud E-FLIST-WRONG-STAGE, never silently ignored.
    let bad = write_tmp("ws.velab", "module x; endmodule\n");
    let out = vita(&["velab", bad.to_str().unwrap(), "+define+W=8"]);
    let out2 = vita(&["vrun", bad.to_str().unwrap(), "-D", "W=8"]);
    let _ = std::fs::remove_file(&bad);
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("VITA-E8007"));
    assert_eq!(out2.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out2.stderr).contains("VITA-E8007"));
}

#[test]
fn single_value_knob_override_warns_last_wins() {
    let src = warning_design("ovr.sv");
    let out = vita(&[src.to_str().unwrap(), "--timeout", "5", "--timeout", "500"]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("VITA-W8009"),
        "override must warn (always-logged), got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ── Phase-1.x: vita-log stage 2 — `--log` tee / `-q`/`-v` / counts epilogue ──

/// A design that prints RTL output AND trips W1017 (no `timescale).
fn talky_design(name: &str) -> std::path::PathBuf {
    write_tmp(
        name,
        "module t; initial begin $display(\"hello-rtl\"); $finish; end endmodule\n",
    )
}

fn tmp_log(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("vita_ux_{}_{name}", std::process::id()))
}

#[test]
fn log_tee_captures_display_diags_and_progress() {
    let src = talky_design("tee.sv");
    let log = tmp_log("tee.log");
    let out = vita(&[src.to_str().unwrap(), "--log", log.to_str().unwrap()]);
    let _ = std::fs::remove_file(&src);
    let transcript = std::fs::read_to_string(&log).expect("log file must exist");
    let _ = std::fs::remove_file(&log);
    assert_eq!(out.status.code(), Some(0));
    // terminal keeps the $display transcript…
    assert!(String::from_utf8_lossy(&out.stdout).contains("hello-rtl"));
    // …and the log holds the FULL interleaved transcript: RTL output,
    // diagnostics (same rendering as stderr), and progress lines.
    assert!(transcript.contains("hello-rtl"), "log:\n{transcript}");
    assert!(transcript.contains("VITA-W1017"), "log:\n{transcript}");
    assert!(
        transcript.contains("simulation ended"),
        "log:\n{transcript}"
    );
}

#[test]
fn quiet_hides_terminal_rtl_output_but_not_log_or_diags() {
    let src = talky_design("quiet.sv");
    let log = tmp_log("quiet.log");
    let out = vita(&[src.to_str().unwrap(), "-q", "--log", log.to_str().unwrap()]);
    let _ = std::fs::remove_file(&src);
    let transcript = std::fs::read_to_string(&log).expect("log file must exist");
    let _ = std::fs::remove_file(&log);
    assert_eq!(out.status.code(), Some(0));
    // -q silences the TERMINAL copy of $display/progress…
    assert!(!String::from_utf8_lossy(&out.stdout).contains("hello-rtl"));
    // …but neither the diagnostics spine nor the log copy.
    assert!(String::from_utf8_lossy(&out.stderr).contains("VITA-W1017"));
    assert!(transcript.contains("hello-rtl"), "log:\n{transcript}");
}

#[test]
fn verbose_echoes_effective_inputs() {
    let src = talky_design("verb.sv");
    let out = vita(&[
        src.to_str().unwrap(),
        "-v",
        "-D",
        "FOO=1",
        "-I",
        "somewhere",
    ]);
    let _ = std::fs::remove_file(&src);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "got:\n{stdout}");
    assert!(stdout.contains("defines: FOO=1"), "got:\n{stdout}");
    assert!(stdout.contains("incdirs: somewhere"), "got:\n{stdout}");
}

#[test]
fn log_append_accumulates_overwrite_resets() {
    let src = talky_design("app.sv");
    let log = tmp_log("app.log");
    let s = src.to_str().unwrap().to_owned();
    let l = log.to_str().unwrap().to_owned();
    vita(&[&s, "--log", &l]);
    vita(&[&s, "--log", &l, "--log-append"]);
    let appended = std::fs::read_to_string(&log).unwrap();
    vita(&[&s, "--log", &l]); // no append ⇒ overwrite
    let fresh = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&log);
    assert_eq!(
        appended.matches("hello-rtl").count(),
        2,
        "append must accumulate:\n{appended}"
    );
    assert_eq!(
        fresh.matches("hello-rtl").count(),
        1,
        "default must overwrite:\n{fresh}"
    );
}

#[test]
fn counts_epilogue_always_printed() {
    let src = talky_design("epi.sv");
    let plain = vita(&[src.to_str().unwrap()]);
    let promoted = vita(&[src.to_str().unwrap(), "-Werror"]);
    let _ = std::fs::remove_file(&src);
    // W1017 fires once: plain run counts it as a warning…
    assert!(
        String::from_utf8_lossy(&plain.stderr).contains("errors=0 warnings=1 notes=0"),
        "got:\n{}",
        String::from_utf8_lossy(&plain.stderr)
    );
    // …and -Werror promotion moves it into the error bucket (post-gate counts).
    assert!(
        String::from_utf8_lossy(&promoted.stderr).contains("errors=1 warnings=0 notes=0"),
        "got:\n{}",
        String::from_utf8_lossy(&promoted.stderr)
    );
}

#[test]
fn log_open_failure_is_cli_error() {
    let src = talky_design("badlog.sv");
    let out = vita(&[
        src.to_str().unwrap(),
        "--log",
        "/definitely/not/a/dir/x.log",
    ]);
    let _ = std::fs::remove_file(&src);
    assert_eq!(out.status.code(), Some(3), "unopenable log = usage error");
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot open log"));
}
