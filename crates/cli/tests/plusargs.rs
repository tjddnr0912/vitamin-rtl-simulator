//! v7 plusargs: `vita design.sv +N=5 +FOO` → `$test$plusargs` (prefix probe,
//! pure eval) / `$value$plusargs` (ref-var write — statement intercept, the
//! seeded-$random family). Semantics pinned LIVE against iverilog 13.0
//! (2026-06-12, probes t7/t8): prefix match, 32-bit return, first matching
//! plusarg wins, a MISS leaves the target variable untouched.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_with(src: &str, extra: &[&str]) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .args(extra)
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn test_plusargs_prefix_match() {
    // iverilog: "N" matches +N=5 (prefix), "N=5" exact-matches, "ZZZ" misses.
    let (out, err, code) = run_with(
        "module top;\n\
         initial begin\n\
           $display(\"tN=%0d tF=%0d tNeq=%0d tX=%0d\",\n\
             $test$plusargs(\"N\"), $test$plusargs(\"FOO\"),\n\
             $test$plusargs(\"N=5\"), $test$plusargs(\"ZZZ\"));\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &["+N=5", "+FOO"],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("tN=1 tF=1 tNeq=1 tX=0"), "got:\n{out}");
}

#[test]
fn value_plusargs_decimal_hex_string_and_miss() {
    // iverilog-pinned: %d → 5, %h → 255, %s packs right-aligned, a MISS
    // returns 0 and leaves the variable UNCHANGED.
    let (out, err, code) = run_with(
        "module top;\n\
         integer n, ok;\n\
         reg [63:0] s;\n\
         initial begin\n\
           ok = $value$plusargs(\"N=%d\", n);\n\
           $display(\"ok=%0d n=%0d\", ok, n);\n\
           ok = $value$plusargs(\"H=%h\", n);\n\
           $display(\"okh=%0d nh=%0d\", ok, n);\n\
           ok = $value$plusargs(\"S=%s\", s);\n\
           $display(\"oks=%0d s=%s\", ok, s);\n\
           ok = $value$plusargs(\"MISS=%d\", n);\n\
           $display(\"okm=%0d nm=%0d\", ok, n);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &["+N=5", "+FOO", "+H=ff", "+S=hello"],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1 n=5"), "got:\n{out}");
    assert!(out.contains("okh=1 nh=255"), "got:\n{out}");
    assert!(out.contains("oks=1 s=   hello"), "got:\n{out}");
    assert!(out.contains("okm=0 nm=255"), "got:\n{out}");
}

#[test]
fn value_plusargs_first_match_wins() {
    // iverilog-pinned (t8): +D=1 +D=2 → 1.
    let (out, err, code) = run_with(
        "module top;\n\
         integer n, ok;\n\
         initial begin\n\
           ok = $value$plusargs(\"D=%d\", n);\n\
           $display(\"ok=%0d n=%0d\", ok, n);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &["+D=1", "+D=2"],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok=1 n=1"), "got:\n{out}");
}

#[test]
fn value_plusargs_outside_direct_rhs_is_loud() {
    let (out, err, code) = run_with(
        "module top;\n\
         integer n, ok;\n\
         initial begin\n\
           ok = $value$plusargs(\"D=%d\", n) + 1;\n\
           $display(\"ok=%0d\", ok);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &["+D=1"],
    );
    assert_ne!(code, Some(0), "must fail loud, got stdout:\n{out}");
    assert!(err.contains("E3009"), "stderr:\n{err}");
}

#[test]
fn binary_and_octal_conversions() {
    let (out, err, code) = run_with(
        "module top;\n\
         integer n, ok;\n\
         initial begin\n\
           ok = $value$plusargs(\"B=%b\", n);\n\
           $display(\"okb=%0d nb=%0d\", ok, n);\n\
           ok = $value$plusargs(\"O=%o\", n);\n\
           $display(\"oko=%0d no=%0d\", ok, n);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &["+B=1010", "+O=17"],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("okb=1 nb=10"), "got:\n{out}");
    assert!(out.contains("oko=1 no=15"), "got:\n{out}");
}
