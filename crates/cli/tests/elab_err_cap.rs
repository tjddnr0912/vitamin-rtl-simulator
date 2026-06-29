//! ELAB-ERR-CAP (2026-06-22 hostile-input hardening): elaborate had no cap on
//! the number of error diagnostics it emits, so a single broken construct inside
//! a large (or nested) generate emits one error PER unrolled iteration — a
//! diagnostic flood (thousands of identical lines, unbounded stderr/memory). The
//! parser already caps at 50; elaborate must cap too. All `Severity::Error`
//! emission funnels through `Elaborator::error`, so one soft cap there is total.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_elaberr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// One undefined reference per unrolled generate iteration → `n` errors without
/// a cap.
fn err_flood_src(n: usize) -> String {
    format!(
        "module top;\n\
         genvar i;\n\
         generate\n\
         for (i = 0; i < {n}; i = i + 1) begin: g\n\
           wire w; assign w = nonexistent_signal_xyz;\n\
         end\n\
         endgenerate\n\
         endmodule\n"
    )
}

/// A broken design that would flood thousands of identical errors emits a
/// BOUNDED count (the soft cap), and still exits loud (1).
#[test]
fn elaborate_error_flood_is_capped() {
    let (_out, err, code) = run(&err_flood_src(4096));
    assert_eq!(code, Some(1), "broken design must be a loud reject");
    let count = err.matches("error[VITA-").count();
    assert!(
        count <= 210,
        "elaborate error emission must be capped (~200); got {count}"
    );
}

/// A handful of genuine errors all pass through (the cap never truncates a
/// realistic multi-error report).
#[test]
fn few_errors_not_capped() {
    let (_out, err, code) = run(&err_flood_src(3));
    assert_eq!(code, Some(1), "broken design must be a loud reject");
    let count = err.matches("error[VITA-").count();
    assert!(
        (1..=12).contains(&count),
        "small error sets pass through uncapped; got {count}"
    );
}
