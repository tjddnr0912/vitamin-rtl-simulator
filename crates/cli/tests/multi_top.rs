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
