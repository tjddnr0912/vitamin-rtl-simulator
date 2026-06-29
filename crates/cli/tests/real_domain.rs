//! CLI smoke test (§5 test #15): pipe a real testbench through the `vita`
//! oneshot binary and assert the printed real. End-to-end coverage of the
//! real/realtime domain through the production CLI entry point.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp_sv() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_real_{}_{n}.v", std::process::id()))
}

/// Write a temp `.sv`, run `vita <file>` (oneshot), capture stdout.
fn run_vita_oneshot(src: &str) -> String {
    let path = tmp_sv();
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// 15. cli smoke — a real testbench computes the harmonic sum H4 and prints it.
// NOTE (deviation from the spec literal): `for` loops are an UNRELATED MVP gap
// in this codebase (elaborate emits VITA-W3008 "for loop skipped"), so the H4
// accumulation is written as the equivalent unrolled real assignments. The
// assertion (H4 = 1 + 1/2 + 1/3 + 1/4 = 2.083333) is identical.
#[test]
fn cli_smoke_real_testbench() {
    let src = r#"
module tb; real acc; integer k;
initial begin
  acc = 0.0;
  k = 1; acc = acc + (1.0 / k);
  k = 2; acc = acc + (1.0 / k);
  k = 3; acc = acc + (1.0 / k);
  k = 4; acc = acc + (1.0 / k);
  $display("H4=%f", acc);
end
endmodule
"#;
    let out = run_vita_oneshot(src);
    // 1 + 0.5 + 0.333333... + 0.25 = 2.083333...
    assert!(
        out.lines().any(|l| l == "H4=2.083333"),
        "expected 'H4=2.083333' in vita output, got:\n{out}"
    );
}
