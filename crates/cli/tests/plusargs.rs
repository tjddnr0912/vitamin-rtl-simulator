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

/// WIDE conversions — every value here is iverilog 13's, measured. The
/// previous conversion went through `u64::from_str_radix(..).unwrap_or(0)`,
/// so all four of these silently wrote ZERO with `got=1`: a `%h` past 16
/// digits, a `%b` past 64 digits, a `%d` past `u64::MAX`, and (the sibling
/// axis) a negative `%d` into a wider-than-64 destination, which came out
/// zero- instead of sign-extended.
#[test]
fn wide_conversions_do_not_truncate_to_a_word() {
    let (out, err, code) = run_with(
        "module top;\n\
         reg [95:0] w; reg [69:0] b; reg [95:0] d; reg [95:0] neg;\n\
         reg [31:0] ok;\n\
         initial begin\n\
           w = 0; b = 0; d = 0; neg = 96'hAA;\n\
           ok = $value$plusargs(\"W=%h\", w);   $display(\"w: ok=%0d w=%h\", ok, w);\n\
           ok = $value$plusargs(\"B=%b\", b);   $display(\"b: ok=%0d b=%b\", ok, b);\n\
           ok = $value$plusargs(\"D=%d\", d);   $display(\"d: ok=%0d d=%0d\", ok, d);\n\
           ok = $value$plusargs(\"NEG=%d\", neg); $display(\"neg: ok=%0d neg=%h\", ok, neg);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[
            "+W=123456789abcdef012345678",
            "+B=1010111010101110101011101010111010101110101011101010111010101110101011",
            "+D=79228162514264337593543950335",
            "+NEG=-5",
        ],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("w: ok=1 w=123456789abcdef012345678"),
        "got:\n{out}"
    );
    assert!(
        out.contains(
            "b: ok=1 b=1010111010101110101011101010111010101110101011101010111010101110101011"
        ),
        "got:\n{out}"
    );
    assert!(
        out.contains("d: ok=1 d=79228162514264337593543950335"),
        "got:\n{out}"
    );
    assert!(
        out.contains("neg: ok=1 neg=fffffffffffffffffffffffb"),
        "got:\n{out}"
    );
}

/// Truncation keeps the LOW bits for every radix (iverilog-measured:
/// `+D=4294967297` into 32 bits reads back 1, not saturation's ffffffff).
#[test]
fn wide_parse_into_a_narrow_destination_truncates() {
    let (out, err, code) = run_with(
        "module top;\n\
         reg [31:0] d32, ok;\n\
         initial begin\n\
           d32 = 0;\n\
           ok = $value$plusargs(\"D=%d\", d32); $display(\"d32: ok=%0d d32=%0d\", ok, d32);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &["+D=4294967297"],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("d32: ok=1 d32=1"), "got:\n{out}");
}
