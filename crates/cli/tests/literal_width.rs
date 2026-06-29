//! Unsized integer-literal width (P0-10). IEEE 1364 §3.5.1: an unsized literal
//! is "at least 32 bits", grown to hold its value. Pre-P0-10 every unsized
//! literal was packed to a FIXED 32 bits, so values >= 2^31 silently broke
//! (2^31 -> -2^31, 2^32 -> 0). The literal width also feeds self-determined
//! expression context, so a too-narrow literal corrupted comparisons/shifts.
//!
//! Every expected value/width is pinned LIVE against iverilog 13.0 ($bits).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` and report (stdout, stderr, exit-code, wall-clock). Used by
/// the DoS-guard tests: a pathological literal must finish (loud or quiet) WITHOUT
/// the O(n^2) decimal conversion or a width-sized giant allocation blowing the
/// time/space budget. The bound is generous (seconds) — pre-fix the 40000-digit
/// case took ~27s and the over-cap cases were minutes / ~1 GiB.
fn run_timed(src: &str) -> (String, String, Option<i32>, Duration) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_litw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let t0 = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let elapsed = t0.elapsed();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
        elapsed,
    )
}

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_litw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    // Drop the trailing "simulation ended (...)" status line vita prints.
    let raw = String::from_utf8_lossy(&out.stdout);
    let mut s = String::new();
    for l in raw.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn run_err(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_litw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn plain_decimal_value_and_bits_match_iverilog() {
    // boundaries: 2^31-1 (32b), 2^31 (33b), 2^32-1 (33b), 2^32 (34b), 2^33 (35b).
    let out = run("module t; initial begin\n\
         $display(\"%0d %0d\", 2147483647, $bits(2147483647));\n\
         $display(\"%0d %0d\", 2147483648, $bits(2147483648));\n\
         $display(\"%0d %0d\", 4294967295, $bits(4294967295));\n\
         $display(\"%0d %0d\", 4294967296, $bits(4294967296));\n\
         $display(\"%0d %0d\", 8589934592, $bits(8589934592));\n\
         $display(\"%0d %0d\", 42, $bits(42));\n\
         $finish; end endmodule\n");
    assert_eq!(
        out,
        "2147483647 32\n2147483648 33\n4294967295 33\n4294967296 34\n8589934592 35\n42 32\n"
    );
}

#[test]
fn unsized_literal_width_feeds_expression_context() {
    // With truncation these were wrong: 2^32>1 was 0, 2^32>>>1 was 0.
    let out = run("module t; initial begin\n\
         $display(\"%0d\", (4294967296 > 1));\n\
         $display(\"%0d\", 4294967296 >>> 1);\n\
         $finish; end endmodule\n");
    assert_eq!(out, "1\n2147483648\n");
}

#[test]
fn based_decimal_signed_vs_unsigned_width() {
    // 'd (unsigned): 2^31 fits in 32, 2^32 needs 33.  'sd (signed): 2^31 needs 33.
    let out = run("module t; initial begin\n\
         $display(\"%0d %0d\", 'd2147483648, $bits('d2147483648));\n\
         $display(\"%0d %0d\", 'd4294967296, $bits('d4294967296));\n\
         $display(\"%0d %0d\", 'sd2147483648, $bits('sd2147483648));\n\
         $finish; end endmodule\n");
    assert_eq!(out, "2147483648 32\n4294967296 33\n2147483648 33\n");
}

#[test]
fn based_hex_uses_digit_span() {
    // h/b/o width = digit span: 8 hex digits = 32, 9 = 36 (NOT the value MSB 33).
    let out = run("module t; initial begin\n\
         $display(\"%0d %0d\", 'hFFFFFFFF, $bits('hFFFFFFFF));\n\
         $display(\"%0d %0d\", 'h1FFFFFFFF, $bits('h1FFFFFFFF));\n\
         $display(\"%0d %0d\", 'sh1FFFFFFFF, $bits('sh1FFFFFFFF));\n\
         $finish; end endmodule\n");
    assert_eq!(out, "4294967295 32\n8589934591 36\n8589934591 36\n");
}

#[test]
fn oversized_literal_width_is_capped_loud() {
    // P0-10 robustness: now that unsized widths are real, cap them like declared
    // nets (MAX_NET_WIDTH = 1<<20). A literal wider than the cap is rejected loud
    // (E3009), never interned as a giant const. A wide-but-under-cap literal works.
    let (err, code) = run_err(
        "module t; initial begin\n\
         $display(\"%0d\", 1100000'h1);\n\
         #1 $finish; end endmodule\n",
    );
    assert_eq!(
        code,
        Some(1),
        "must reject over-cap literal; stderr:\n{err}"
    );
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");

    // 1024-bit literal (well under cap) elaborates fine.
    let out =
        run("module t; initial begin $display(\"%0d\", $bits(1024'h1)); $finish; end endmodule\n");
    assert_eq!(out, "1024\n");
}

// Generous wall-clock ceiling for the DoS-guard tests. After the fix every case
// below is milliseconds; pre-fix they were 27 s / minutes / a ~1 GiB allocation.
const DOS_BUDGET: Duration = Duration::from_secs(8);

#[test]
fn wide_decimal_value_roundtrips() {
    // A 97-bit (2-word) decimal: parse -> store -> %0d must echo it exactly. This
    // locks the multi-word magnitude conversion end-to-end (0xDEADBEEF*2^64 + 1).
    let out = run("module t; initial begin\n\
         $display(\"%0d\", 68915718005535514953299001345);\n\
         $finish; end endmodule\n");
    assert_eq!(out, "68915718005535514953299001345\n");
}

#[test]
fn huge_decimal_completes_fast() {
    // 40000-digit literal (well under the 2^20-bit width cap): the schoolbook
    // bit-at-a-time division was O(n^2) (~27 s). With Horner base-10^9 it is ms.
    // Round-trips through %0d, so we also assert the value survives intact.
    let digits = "1234567890".repeat(4000); // 40000 digits, no leading zero
    let src =
        format!("module t; initial begin $display(\"%0d\", {digits}); $finish; end endmodule\n");
    let (out, err, code, elapsed) = run_timed(&src);
    assert_eq!(code, Some(0), "should elaborate+run; stderr:\n{err}");
    // run_timed keeps vita's trailing "simulation ended" status line; the value is
    // the FIRST output line.
    assert_eq!(
        out.lines().next(),
        Some(digits.as_str()),
        "value must round-trip"
    );
    assert!(elapsed < DOS_BUDGET, "decimal DoS: took {elapsed:?}");
}

#[test]
fn over_digit_cap_decimal_rejected_fast() {
    // > MAX_DECIMAL_DIGITS: a value with this many digits cannot fit the 2^20-bit
    // width cap, so it is rejected in O(n) (a digit scan) instead of running the
    // O(n^2) base conversion. Pre-fix this was minutes of CPU.
    let digits = "9".repeat(400_000);
    let src =
        format!("module t; initial begin $display(\"%0d\", {digits}); #1 $finish; end endmodule\n");
    let (_o, err, code, elapsed) = run_timed(&src);
    assert_eq!(
        code,
        Some(1),
        "must reject loud; stderr head:\n{}",
        &err[..err.len().min(200)]
    );
    assert!(err.contains("VITA-E3009"), "E3009 expected");
    assert!(elapsed < DOS_BUDGET, "digit-cap guard: took {elapsed:?}");
}

#[test]
fn huge_sized_width_rejected_without_oom() {
    // `4294967295'h1`: tiny source, but the explicit width would size a ~1 GiB
    // BitPacked before the width-cap reject. The allocation is now clamped, so it
    // is rejected loud and fast. (Adversarial-review-found sibling of the decimal
    // O(n^2): same "tiny input, huge work" shape, in the sized-width pack path.)
    let (_o, err, code, elapsed) = run_timed(
        "module t; initial begin $display(\"%0d\", 4294967295'h1); #1 $finish; end endmodule\n",
    );
    assert_eq!(code, Some(1), "must reject loud; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
    assert!(elapsed < DOS_BUDGET, "sized-width pack: took {elapsed:?}");
}

#[test]
fn small_literals_unchanged() {
    // Regression: ordinary literals stay 32-bit (pre-P0-10 byte-identical).
    let out = run("module t; initial begin\n\
         $display(\"%0d %0d %0d\", $bits(0), $bits(42), $bits(2147483647));\n\
         $display(\"%0d %0d\", $bits('hFF), $bits('hFFFFFFFF));\n\
         $finish; end endmodule\n");
    assert_eq!(out, "32 32 32\n32 32\n");
}
