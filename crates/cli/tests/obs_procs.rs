//! R14 (ROADMAP §3 ⑭) — `--obs-procs` / `--obs-procs-time`: the per-body
//! execution profile in `run.json`'s `processes` object.
//!
//! SPEC = `docs/preview/19-ai-agent-observability.md` §4.6.
//!
//! WHY THE COUNTS ARE ASSERTED EXACTLY. A profile that is merely "present and
//! plausible" is exactly the failure mode this feature exists to prevent: a
//! user reads it, believes the wrong `always_comb` is hot, and optimises the
//! wrong thing. So every count below is derived BY HAND from the design in the
//! test, and the derivation is written next to it. If a scheduler change moves
//! one of these numbers, the right response is to redo the derivation — not to
//! relax the assertion.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` with extra `args`; returns (stdout, exit_code, obs_dir).
///
/// The per-test unique directory + `remove_dir_all` is `obs.rs`'s recipe and it
/// is load-bearing for the same reason: `n` restarts at 0 in every test PROCESS
/// and the OS recycles PIDs, so two runs can otherwise land on one directory.
fn run(src: &str, args: &[&str]) -> (String, i32, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_obsprocs_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let obs = d.join("obsout");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg(obs.to_str().unwrap())
        .args(args)
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
        obs,
    )
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

/// One serialized `items[]` row, parsed out of the manifest by hand.
///
/// A hand parser and not a JSON crate because the CLI has no JSON dependency
/// and adding one to assert on a file the CLI itself hand-serializes would let
/// a formatting bug hide behind a lenient reader. This reads the exact bytes.
#[derive(Debug, PartialEq, Eq)]
struct Row {
    domain: String,
    kind: String,
    scope: String,
    line: u32,
    evals: u64,
}

fn str_field(row: &str, key: &str) -> String {
    let pat = format!("\"{key}\": \"");
    let i = row
        .find(&pat)
        .unwrap_or_else(|| panic!("no {key} in {row}"))
        + pat.len();
    let rest = &row[i..];
    rest[..rest.find('"').expect("unterminated string")].to_string()
}

fn num_field(row: &str, key: &str) -> u64 {
    let pat = format!("\"{key}\": ");
    let i = row
        .find(&pat)
        .unwrap_or_else(|| panic!("no {key} in {row}"))
        + pat.len();
    let rest = &row[i..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().expect("numeric field")
}

/// Every `items[]` row of the manifest, in file order (= the emitted sort).
fn rows(json: &str) -> Vec<Row> {
    json.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("{\"domain\""))
        .map(|l| Row {
            domain: str_field(l, "domain"),
            kind: str_field(l, "kind"),
            scope: str_field(l, "scope"),
            line: num_field(l, "line") as u32,
            evals: num_field(l, "evals"),
        })
        .collect()
}

/// THE hand-checkable design. Every count below is derived from it in the
/// assertions; keep the line numbers stable (they are asserted).
///
///  1 module tb;
///  2   logic clk = 0;
///  3   logic [7:0] c = 0;
///  4   wire  [7:0] d;
///  5   logic [7:0] e;
///  6   assign d = c + 8'd1;
///  7   always #5 clk = ~clk;
///  8   always_ff @(posedge clk) c <= c + 8'd1;
///  9   always_comb e = d ^ 8'hA5;
/// 10   initial begin
/// 11     #100;
/// 12     $display("c=%0d d=%0d e=%0d", c, d, e);
/// 13     $finish;
/// 14   end
/// 15 endmodule
const CLOCKED: &str = "module tb;\n\
     \x20 logic clk = 0;\n\
     \x20 logic [7:0] c = 0;\n\
     \x20 wire  [7:0] d;\n\
     \x20 logic [7:0] e;\n\
     \x20 assign d = c + 8'd1;\n\
     \x20 always #5 clk = ~clk;\n\
     \x20 always_ff @(posedge clk) c <= c + 8'd1;\n\
     \x20 always_comb e = d ^ 8'hA5;\n\
     \x20 initial begin\n\
     \x20   #100;\n\
     \x20   $display(\"c=%0d d=%0d e=%0d\", c, d, e);\n\
     \x20   $finish;\n\
     \x20 end\n\
     endmodule\n";

/// SHAPE + EXACT COUNTS, each derived by hand from `CLOCKED`.
#[test]
fn obs_procs_counts_are_hand_checkable() {
    let (stdout, code, obs) = run(CLOCKED, &["--obs-procs"]);
    assert_eq!(code, 0, "run failed: {stdout}");
    // ANTI-VACUITY: the design really did run 10 clocked increments. Without
    // this the count assertions below could all be satisfied by a design that
    // elaborated and did nothing.
    assert!(
        stdout.contains("c=10 d=11 e=174"),
        "design did not run as derived: {stdout}"
    );
    let m = read(&obs.join("run.json"));

    // ── shape ──
    assert!(
        m.contains("\"processes\": {\"timed\": false, "),
        "processes object missing or timed on a count-only run:\n{m}"
    );
    // 5 processes: the two decl initializers are ONE synthesized flush, plus
    // `always #5`, `always_ff`, `always_comb`, `initial`. 1 continuous assign.
    assert!(
        m.contains("\"counts\": {\"processes\": 5, \"assigns\": 1, \"total_evals\": 57}"),
        "domain sizes / total wrong:\n{m}"
    );
    // COUNT-ONLY runs carry NO `time_s`: a 0.0 would read as "this body is
    // free", which is a different claim from "nobody measured".
    assert!(!m.contains("time_s"), "untimed run emitted time_s:\n{m}");

    let rows = rows(&m);
    assert_eq!(rows.len(), 6, "one row per process + per assign: {rows:?}");

    // ── the counts, derived ──
    //
    // `always #5 clk = ~clk` (line 7): entered once at t=0, then resumed at
    // t=5,10,…,100 = 20 resumes ⇒ 21 activations. It is the hottest body, so it
    // must also be row 0 (sort = evals descending).
    assert_eq!(
        rows[0],
        Row {
            domain: "process".into(),
            kind: "always".into(),
            scope: "tb".into(),
            line: 7,
            evals: 21,
        },
        "clock generator: {rows:?}"
    );
    // `assign d = c + 1` (line 6): the t0 settle runs TWICE (once before
    // arming, once at the top of the run loop) and then once per `c` change =
    // 10 posedges ⇒ 12.
    assert_eq!(
        rows[1],
        Row {
            domain: "assign".into(),
            kind: "assign".into(),
            scope: "tb".into(),
            line: 6,
            evals: 12,
        },
        "continuous assign: {rows:?}"
    );
    // `always_comb e = d ^ 8'hA5` (line 9): once at t=0 plus once per `d`
    // change (10) ⇒ 11.
    assert_eq!(
        rows[2],
        Row {
            domain: "process".into(),
            kind: "always_comb".into(),
            scope: "tb".into(),
            line: 9,
            evals: 11,
        },
        "always_comb: {rows:?}"
    );
    // `always_ff @(posedge clk)` (line 8): clk rises at t=5,15,…,95 ⇒ 10.
    assert_eq!(
        rows[3],
        Row {
            domain: "process".into(),
            kind: "always_ff".into(),
            scope: "tb".into(),
            line: 8,
            evals: 10,
        },
        "always_ff: {rows:?}"
    );
    // The user `initial` (line 10) runs, parks on `#100`, resumes ⇒ 2.
    assert_eq!(
        rows[4],
        Row {
            domain: "process".into(),
            kind: "initial".into(),
            scope: "tb".into(),
            line: 10,
            evals: 2,
        },
        "user initial: {rows:?}"
    );
    // The declaration-initializer flush: ONE synthesized zero-time body for
    // both `clk = 0` and `c = 0`. `var_init`, NOT `initial` — there is no
    // `initial` keyword at line 2 and saying so would send a reader hunting.
    assert_eq!(
        rows[5],
        Row {
            domain: "process".into(),
            kind: "var_init".into(),
            scope: "tb".into(),
            line: 2,
            evals: 1,
        },
        "decl-init flush: {rows:?}"
    );
    // 21 + 12 + 11 + 10 + 2 + 1 == the published total.
    assert_eq!(rows.iter().map(|r| r.evals).sum::<u64>(), 57);
}

/// R-F1: the COUNTS are deterministic, so two runs of the same input produce a
/// byte-identical `processes` object. This is what lets a CI job diff two
/// `run.json` files directly — and it is the reason wall-clock timing is a
/// SECOND flag rather than implied by the first.
#[test]
fn obs_procs_is_byte_identical_across_runs() {
    // TWICE FROM ONE DIRECTORY. ⚠️ Two runs from two temp dirs are NOT the same
    // input: `file` carries the source path as given (see §4.6 — a 19-file
    // design needs the directory to be actionable, so this field deliberately
    // does not take `source.name`'s basename). The claim under test is R-F1 —
    // same input, byte-identical file — and the path is part of the input.
    let d = std::env::temp_dir().join(format!("vita_obsprocs_det_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, CLOCKED).unwrap();
    let once = |out: &str| {
        let o = d.join(out);
        let st = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(f.to_str().unwrap())
            .arg("--obs-dir")
            .arg(o.to_str().unwrap())
            .arg("--obs-procs")
            .current_dir(&d)
            .status()
            .expect("run vita");
        assert!(st.success());
        let s = read(&o.join("run.json"));
        let i = s.find("\"processes\":").expect("processes object");
        let j = s.find("\"utc_unix_s\"").expect("wall-clock block");
        s[i..j].to_string()
    };
    assert_eq!(
        once("a"),
        once("b"),
        "the profile must be byte-identical across runs"
    );
}

/// Both `--backend` executors bump the counters at their own dispatch seam and
/// their own settle fixpoint. If one of the four sites is ever forgotten the
/// profile becomes a function of the backend, which would make it useless for
/// comparing runs — and the failure is SILENT (a smaller number still looks
/// like a number).
#[test]
fn obs_procs_is_backend_invariant() {
    let (_, _, n) = run(CLOCKED, &["--obs-procs", "--backend", "native"]);
    let (_, _, v) = run(CLOCKED, &["--obs-procs", "--backend", "vm"]);
    let (_, _, i) = run(CLOCKED, &["--obs-procs", "--backend", "interp"]);
    let a = rows(&read(&n.join("run.json")));
    assert_eq!(a.len(), 6, "native produced no profile");
    assert_eq!(
        a,
        rows(&read(&v.join("run.json"))),
        "vm differs from native"
    );
    assert_eq!(
        a,
        rows(&read(&i.join("run.json"))),
        "interp differs from native"
    );
}

/// `--obs-procs-time` adds `time_s` and flips `timed`. The VALUE is wall clock
/// and cannot be asserted; the SHAPE can, and so can the invariant that turning
/// timing on does not change a single count (it must not perturb the schedule,
/// only the clock reads around it).
#[test]
fn obs_procs_time_adds_time_s_without_moving_the_counts() {
    let (_, code, obs) = run(CLOCKED, &["--obs-procs-time"]);
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    assert!(
        m.contains("\"processes\": {\"timed\": true, "),
        "timed flag not set:\n{m}"
    );
    assert!(m.contains("\"time_s\": "), "no time_s on a timed run:\n{m}");
    // Timing IMPLIES counting: a `nanos` with no `evals` beside it would be a
    // cost with no way to normalise it.
    let (_, _, plain) = run(CLOCKED, &["--obs-procs"]);
    assert_eq!(
        rows(&m),
        rows(&read(&plain.join("run.json"))),
        "timing changed the counts"
    );
}

/// A module instantiated twice is TWO rows with the same `file:line:col` and
/// different `scope` — which is the whole point of carrying the instance path.
/// Folding them into one row would answer "which line" and refuse to answer
/// "which instance", and an unbalanced design is exactly when you need the
/// second answer.
#[test]
fn each_instance_gets_its_own_row() {
    // `u0` is clocked every cycle; `u1` only when `en` is high, which the
    // testbench holds low. Same line, deliberately different counts.
    let src = "module m(input logic clk, input logic en, output logic [7:0] q);\n\
         \x20 always_ff @(posedge clk) if (en) q <= q + 8'd1;\n\
         endmodule\n\
         module tb;\n\
         \x20 logic clk = 0, hi = 1, lo = 0;\n\
         \x20 logic [7:0] a, b;\n\
         \x20 m u0(.clk(clk), .en(hi), .q(a));\n\
         \x20 m u1(.clk(clk), .en(lo), .q(b));\n\
         \x20 always #5 clk = ~clk;\n\
         \x20 initial begin #100; $display(\"done\"); $finish; end\n\
         endmodule\n";
    let (stdout, code, obs) = run(src, &["--obs-procs"]);
    assert_eq!(code, 0, "{stdout}");
    let rows = rows(&read(&obs.join("run.json")));
    let ff: Vec<&Row> = rows.iter().filter(|r| r.kind == "always_ff").collect();
    assert_eq!(
        ff.len(),
        2,
        "one row per INSTANCE of the always_ff: {rows:?}"
    );
    let mut scopes: Vec<&str> = ff.iter().map(|r| r.scope.as_str()).collect();
    scopes.sort_unstable();
    assert_eq!(scopes, vec!["tb.u0", "tb.u1"], "instance paths: {rows:?}");
    // Both are the SAME source line (the one `always_ff` in `m`) …
    assert!(ff.iter().all(|r| r.line == 2), "both name m's line 2");
    // … and both fired on all 10 posedges (the sensitivity is the edge; `en`
    // gates the body, not the activation). The instance identity is what the
    // row buys here; the counts happen to match, and asserting that they DO is
    // what keeps this test honest about what an "evaluation" means.
    assert!(
        ff.iter().all(|r| r.evals == 10),
        "10 posedges each: {rows:?}"
    );
}

/// The profile is published through `run.json` and nowhere else, so
/// `--obs-procs` without `--obs-dir` would instrument the whole run and discard
/// every number at exit 0. Loud, exactly as `--probe` is.
#[test]
fn obs_procs_without_obs_dir_is_loud() {
    let d = std::env::temp_dir().join(format!("vita_obsprocs_nodir_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, CLOCKED).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .arg("--obs-procs")
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert_ne!(out.status.code(), Some(0), "must not exit 0");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--obs-procs") && err.contains("--obs-dir"),
        "the message must name both flags: {err}"
    );
}

/// Without the flag the object is `null` — NOT an empty object and not absent.
/// A consumer must be able to tell "not measured" from "measured, nothing ran",
/// and this rail is read by agents that cannot ask.
#[test]
fn without_the_flag_processes_is_null() {
    let (_, code, obs) = run(CLOCKED, &[]);
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    assert!(
        m.contains("\"processes\": null"),
        "expected an explicit null:\n{m}"
    );
}

/// Staged applets do not write `run.json`, so accepting the flag there would
/// profile a run and throw the numbers away.
#[test]
fn staged_applets_reject_obs_procs() {
    let d = std::env::temp_dir().join(format!("vita_obsprocs_staged_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, CLOCKED).unwrap();
    for stage in ["vcmp", "velab", "vrun"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(stage)
            .arg(f.to_str().unwrap())
            .arg("--obs-procs")
            .current_dir(&d)
            .output()
            .expect("run applet");
        assert_ne!(out.status.code(), Some(0), "{stage} accepted --obs-procs");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("--obs-procs"),
            "{stage} must name the flag it rejected: {err}"
        );
    }
}
