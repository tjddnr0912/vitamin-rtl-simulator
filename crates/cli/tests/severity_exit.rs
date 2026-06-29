//! P1-1 CLI contract: runtime `$fatal` must exit 1 (doc-13 exit table row
//! "runtime `$fatal`"), `$error` exits 1 but completes the run, `$warning`/
//! `$info` stay exit 0. The staged path proves the severity side table
//! round-trips through the `.velab` trailer (vcmp → velab → vrun).
#![allow(clippy::needless_update)]
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp(ext: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_sev_{}_{n}.{ext}", std::process::id()))
}

fn s(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

const FATAL_TB: &str = r#"
module tb;
  initial begin
    $display("running");
    $fatal(1, "broken");
  end
endmodule
"#;

const WARN_TB: &str = r#"
module tb;
  initial begin
    $warning("just a warning");
    $finish;
  end
endmodule
"#;

const ERROR_TB: &str = r#"
module tb;
  initial begin
    $error("bad");
    #1 $display("kept going");
    $finish;
  end
endmodule
"#;

/// One-shot `vita` subprocess: `$fatal` → exit 1 + `fatal[VITA-F4004]` on stderr.
#[test]
fn oneshot_fatal_exits_one_with_diag() {
    let src = tmp("sv");
    std::fs::write(&src, FATAL_TB).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&src)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&src);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "exit code; stderr:\n{stderr}");
    assert!(
        stderr.contains("VITA-F4004") && stderr.contains("broken"),
        "stderr must carry the fatal diag + message:\n{stderr}"
    );
}

/// `$warning` alone stays a clean exit 0.
#[test]
fn oneshot_warning_exits_zero() {
    let src = tmp("sv");
    std::fs::write(&src, WARN_TB).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&src)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&src);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Staged flow: the severity table must survive the `.velab` trailer, so
/// `vrun` on a `$fatal` design exits 1.
#[test]
fn staged_fatal_exits_one() {
    let src = tmp("sv");
    std::fs::write(&src, FATAL_TB).unwrap();
    let vu = tmp("vu");
    let velab = tmp("velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);
    assert_eq!(
        cli::run_vrun(&s(&velab), &opts),
        cli::EXIT_USER_ERROR,
        "staged $fatal must exit 1 (severity trailer round-trip)"
    );
    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}

/// Staged flow: `$error` exits 1 (HadErrors) even though the run completes.
#[test]
fn staged_error_exits_one() {
    let src = tmp("sv");
    std::fs::write(&src, ERROR_TB).unwrap();
    let vu = tmp("vu");
    let velab = tmp("velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);
    assert_eq!(cli::run_vrun(&s(&velab), &opts), cli::EXIT_USER_ERROR);
    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}

const ASSERT_DEFAULT_TB: &str = r#"
module tb;
  initial begin
    assert (1'b0);
    #1 $display("kept going");
    $finish;
  end
endmodule
"#;

const ASSERT_PASS_TB: &str = r#"
module tb;
  initial begin
    assert (1'b1);
    $finish;
  end
endmodule
"#;

/// SV immediate assert with NO else clause: failure runs the IEEE 1800 §16.3
/// default action — `$error` ⇒ stderr diagnostic + exit 1, run completes.
#[test]
fn oneshot_assert_default_failure_exits_one() {
    let src = tmp("sv");
    std::fs::write(&src, ASSERT_DEFAULT_TB).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&src)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&src);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "exit code; stderr:\n{stderr}");
    assert!(
        stderr.contains("Assertion failed"),
        "default action must carry the assert message:\n{stderr}"
    );
    assert!(
        stdout.contains("kept going"),
        "$error continues the run:\n{stdout}"
    );
}

/// A PASSING no-action assert is silent: exit 0, no synthesized diagnostic.
#[test]
fn oneshot_assert_pass_exits_zero() {
    let src = tmp("sv");
    std::fs::write(&src, ASSERT_PASS_TB).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&src)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&src);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "stderr:\n{stderr}");
    assert!(!stderr.contains("Assertion failed"), "stderr:\n{stderr}");
}
