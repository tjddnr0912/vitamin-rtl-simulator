//! SEQ-DEPTH (2026-06-22 hostile-input hardening): `expand_sequence` recurses on
//! nested `##`/`[*]`/`and`/`or` sub-sequences with NO depth cap (only the
//! prop-level `and`/`or` reduction and the per-attempt alternative count were
//! capped). A pathological deeply-nested sequence (`a ##1 a ##1 … a` parses to a
//! left-deep `Delay` AST — the `##` parser loop is iterative, so the PARSER
//! survives, but elaborate's recursive `expand_sequence` does not) overflows the
//! elaborate stack and SIGABRTs with no diagnostic. The cap must turn that into a
//! loud `ElabUnsupported` (exit 1), mirroring the prop-level and/or depth cap.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_seqdepth_{}_{n}", std::process::id()));
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

fn deep_seq_src(depth: usize) -> String {
    // The deep sequence is the implication ANTECEDENT (`seq |-> b`), so it flows
    // through `expand_sequence`. A bare sequence property is rejected at parse
    // time (E2002) before elaborate, so it would not exercise the recursion.
    let seq = "a ##1 ".repeat(depth) + "a";
    format!(
        "module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) {seq} |-> b);\n\
         initial #20 $finish;\n\
         endmodule\n"
    )
}

/// A deeply-nested `##1` sequence must elaborate-reject loud (exit 1), never
/// crash the process with a stack overflow (signal → `code` is `None`).
#[test]
fn deep_sequence_nesting_is_loud_not_crash() {
    let (_out, err, code) = run(&deep_seq_src(100_000));
    assert_eq!(
        code,
        Some(1),
        "deep sequence must be a loud reject, not a crash. stderr:\n{err}"
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("depth cap") || low.contains("too deep") || low.contains("nesting"),
        "must carry a depth diagnostic; stderr:\n{err}"
    );
}

/// A realistic shallow sequence still elaborates and runs (no signal, and the
/// cap never false-trips).
#[test]
fn shallow_sequence_still_runs() {
    let (_out, err, code) = run(&deep_seq_src(20));
    assert!(
        code.is_some(),
        "shallow sequence must not crash. stderr:\n{err}"
    );
    assert!(
        !err.to_lowercase().contains("depth cap"),
        "cap must not false-trip on a 20-deep sequence; stderr:\n{err}"
    );
}
