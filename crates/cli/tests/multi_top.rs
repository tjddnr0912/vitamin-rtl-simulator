//! Regression: MULTIPLE uninstantiated top modules must ALL elaborate and
//! simulate as independent roots — IEEE 1364 / iverilog elaborate every
//! uninstantiated module as a root. The old elaborator picked a SINGLE top
//! (the last-declared uninstantiated module) and silently dropped every other,
//! so a design with two independent top modules ran only one of them — even an
//! immediate `$display` in the dropped module never appeared.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Write a temp `.sv`, run `vita <file>` (oneshot, interpreter backend), capture stdout.
fn run_vita(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_multitop_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// Two independent top modules, neither instantiating the other. BOTH `initial`
// blocks must run — the bug dropped `first` (declared earlier) entirely.
#[test]
fn both_independent_top_modules_simulate() {
    let src = r#"
module first;
  initial $display("first-ran");
endmodule
module second;
  initial $display("second-ran");
endmodule
"#;
    let out = run_vita(src);
    assert!(
        out.lines().any(|l| l == "first-ran"),
        "first top module must simulate (expected 'first-ran'); got:\n{out}"
    );
    assert!(
        out.lines().any(|l| l == "second-ran"),
        "second top module must simulate (expected 'second-ran'); got:\n{out}"
    );
}

// A third uninstantiated top alongside one that DOES instantiate a child:
// `tb` instantiates `dut` (so `dut` is not a root), while `aux` is independent.
// Roots = {tb, aux}; `dut` runs only under `tb`. Confirms the instantiated child
// is not double-elaborated and the independent extra top still runs.
#[test]
fn independent_top_coexists_with_a_hierarchy() {
    let src = r#"
module dut(output wire o);
  assign o = 1'b1;
endmodule
module tb;
  wire w;
  dut u(.o(w));
  initial $display("tb-ran");
endmodule
module aux;
  initial $display("aux-ran");
endmodule
"#;
    let out = run_vita(src);
    assert!(
        out.lines().any(|l| l == "tb-ran"),
        "tb (a root that instantiates dut) must simulate; got:\n{out}"
    );
    assert!(
        out.lines().any(|l| l == "aux-ran"),
        "aux (an independent root) must simulate; got:\n{out}"
    );
}

// ── r17: dual top model — one-shot `--top` + auto-top ambiguity is loud ──────

/// Like [`run_vita`] but with extra leading args, capturing stdout + stderr + code.
fn run_vita_full(extra: &[&str], src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dualtop_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(extra)
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

// Two roots, each with an immediate `$finish` — whichever runs first ends the
// whole sim, so without an explicit top the "other" root is masked. This is the
// exact shape the round-17 reviewer flagged as a silent wrong-top hazard.
const TWO_ROOT_FINISH: &str = r#"
module aaa; initial begin $display("RAN aaa"); $finish; end endmodule
module zzz; initial begin $display("RAN zzz"); $finish; end endmodule
"#;

// One-shot `--top <UNIT>` used to be rejected ("--top is a velab flag"); now it
// pins the elaboration root so a test user gets a DETERMINISTIC single top.
#[test]
fn oneshot_top_flag_pins_named_root() {
    let (out, _e, _c) = run_vita_full(&["--top", "zzz"], TWO_ROOT_FINISH);
    assert!(
        out.lines().any(|l| l == "RAN zzz"),
        "--top zzz must run zzz; got:\n{out}"
    );
    assert!(
        !out.lines().any(|l| l == "RAN aaa"),
        "--top zzz must NOT elaborate aaa; got:\n{out}"
    );
}

// Selection is by the flag, not alphabetical/declaration order: pinning `aaa`
// (which auto-top happens to pick anyway) vs `zzz` proves the flag is honored.
#[test]
fn oneshot_top_flag_selects_the_other_root() {
    let (out, _e, _c) = run_vita_full(&["--top", "aaa"], TWO_ROOT_FINISH);
    assert!(
        out.lines().any(|l| l == "RAN aaa") && !out.lines().any(|l| l == "RAN zzz"),
        "--top aaa must run only aaa; got:\n{out}"
    );
}

// correct-or-loud: ambiguous auto-top (2+ roots, no `--top`) must WARN (W3057),
// naming the roots — it used to silent-pick with no diagnostic (rc=0).
#[test]
fn auto_top_ambiguity_is_loud() {
    let (_o, err, _c) = run_vita_full(&[], TWO_ROOT_FINISH);
    assert!(
        err.contains("VITA-W3057"),
        "ambiguous auto-top must warn W3057; stderr:\n{err}"
    );
    assert!(
        err.contains("aaa") && err.contains("zzz"),
        "the warning must name every root candidate; stderr:\n{err}"
    );
}

// No false positive: a single-root design (the common case) must NOT warn.
#[test]
fn single_root_does_not_warn() {
    let (out, err, _c) = run_vita_full(&[], "module only; initial $display(\"solo\"); endmodule");
    assert!(out.contains("solo"), "single root must run; got:\n{out}");
    assert!(
        !err.contains("VITA-W3057"),
        "single root must not warn; stderr:\n{err}"
    );
}

// Pinning `--top` suppresses the ambiguity warning (the user was explicit).
#[test]
fn explicit_top_suppresses_ambiguity_warning() {
    let (_o, err, _c) = run_vita_full(&["--top", "zzz"], TWO_ROOT_FINISH);
    assert!(
        !err.contains("VITA-W3057"),
        "explicit --top must not warn about ambiguity; stderr:\n{err}"
    );
}

// An unknown `--top` name is loud (E3009 "not found"), never a silent fallback
// to auto-top — a silently-wrong root selection would defeat the whole point.
#[test]
fn oneshot_top_flag_unknown_is_loud() {
    let (_o, err, code) = run_vita_full(&["--top", "nope"], TWO_ROOT_FINISH);
    assert!(
        err.contains("not found in the design"),
        "unknown --top must be loud; stderr:\n{err}"
    );
    assert_ne!(
        code,
        Some(0),
        "unknown --top must exit non-zero; got {code:?}"
    );
}
