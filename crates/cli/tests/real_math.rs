//! N6 real-math system functions (IEEE 1800 §20.8.2): `$ln`/`$log10`/`$exp`/
//! `$sqrt`/`$pow`/`$floor`/`$ceil`/`$sin`/`$cos`/`$tan`/`$asin`/`$acos`/`$atan`/
//! `$atan2`/`$hypot`/`$sinh`/`$cosh`/`$tanh`/`$asinh`/`$acosh`/`$atanh`. Computed
//! via the vendored pure-Rust libm (third_party/libm) → 3-OS byte-identical.
//!
//! Oracle: iverilog 13.0 (`-g2012`) at `%g`/`%f` DISPLAY precision. The vendored
//! libm matches iverilog's platform libm at full f64 precision for most inputs,
//! and within 1 ULP for the rest (e.g. $tan/$exp); the gap is far below display
//! precision, so the `$display` strings are identical. (A full-precision
//! `$realtobits` pin would diverge on those — see sim-engine's libm_determinism
//! test, which documents the D3 trade-off.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> std::process::Output {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rm_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    out
}

fn run(src: &str) -> String {
    let out = vita(src);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| {
        !l.starts_with("simulation ended") && !l.contains("VITA-W") && !l.trim().is_empty()
    }) {
        s.push_str(l.trim());
        s.push('\n');
    }
    s
}

/// Every real-math function at display precision, against the iverilog `%g`
/// reference (captured live).
#[test]
fn real_math_display_precision_matches_iverilog() {
    let out = run("module top;\n\
        initial begin\n\
          $display(\"ln2=%g\", $ln(2.0));\n\
          $display(\"log10=%g\", $log10(1000.0));\n\
          $display(\"exp1=%g\", $exp(1.0));\n\
          $display(\"sqrt2=%g\", $sqrt(2.0));\n\
          $display(\"pow=%g\", $pow(2.0, 10.0));\n\
          $display(\"powh=%g\", $pow(2.0, 0.5));\n\
          $display(\"floor=%g\", $floor(2.7));\n\
          $display(\"ceil=%g\", $ceil(2.1));\n\
          $display(\"sin=%g\", $sin(1.0));\n\
          $display(\"cos=%g\", $cos(1.0));\n\
          $display(\"tan=%g\", $tan(1.0));\n\
          $display(\"asin=%g\", $asin(0.5));\n\
          $display(\"acos=%g\", $acos(0.5));\n\
          $display(\"atan=%g\", $atan(1.0));\n\
          $display(\"atan2=%g\", $atan2(1.0, 1.0));\n\
          $display(\"hypot=%g\", $hypot(3.0, 4.0));\n\
          $display(\"sinh=%g\", $sinh(1.0));\n\
          $display(\"cosh=%g\", $cosh(1.0));\n\
          $display(\"tanh=%g\", $tanh(1.0));\n\
          $display(\"asinh=%g\", $asinh(1.0));\n\
          $display(\"acosh=%g\", $acosh(2.0));\n\
          $display(\"atanh=%g\", $atanh(0.5));\n\
        end\n\
      endmodule\n");
    assert_eq!(
        out,
        "ln2=0.693147\n\
         log10=3\n\
         exp1=2.71828\n\
         sqrt2=1.41421\n\
         pow=1024\n\
         powh=1.41421\n\
         floor=2\n\
         ceil=3\n\
         sin=0.841471\n\
         cos=0.540302\n\
         tan=1.55741\n\
         asin=0.523599\n\
         acos=1.0472\n\
         atan=0.785398\n\
         atan2=0.785398\n\
         hypot=5\n\
         sinh=1.1752\n\
         cosh=1.54308\n\
         tanh=0.761594\n\
         asinh=0.881374\n\
         acosh=1.31696\n\
         atanh=0.549306\n"
    );
}

/// Tighter `%0.10f` pins for two functions that match iverilog at full precision.
#[test]
fn real_math_tight_precision() {
    let out = run("module top;\n\
        initial begin\n\
          $display(\"sin=%0.10f\", $sin(1.0));\n\
          $display(\"ln2=%0.10f\", $ln(2.0));\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "sin=0.8414709848\nln2=0.6931471806\n");
}

/// An integral argument coerces to real (IEEE: real-math args are real).
#[test]
fn integral_arg_coerces_to_real() {
    let out = run("module top;\n\
        initial $display(\"%g %g\", $sqrt(4), $pow(3, 2));\n\
      endmodule\n");
    assert_eq!(out, "2 9\n");
}

/// A real variable assigned from a real-math call (exercises expr_is_real on the
/// rhs so the assignment keeps the real value, not a truncated bit pattern).
#[test]
fn real_var_assignment_keeps_real() {
    let out = run("module top;\n\
        real r;\n\
        initial begin r = $sin(1.0); $display(\"%g\", r); end\n\
      endmodule\n");
    assert_eq!(out, "0.841471\n");
}

/// Real-math composes in arithmetic and nests, matching iverilog.
#[test]
fn real_math_composes_and_nests() {
    let out = run("module top;\n\
        initial begin\n\
          $display(\"%g\", $sin(1.0)*$sin(1.0) + $cos(1.0)*$cos(1.0));\n\
          $display(\"%g\", $sqrt($pow(3.0,2.0) + $pow(4.0,2.0)));\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "1\n5\n");
}

/// Domain errors propagate IEEE NaN/±inf exactly like C/iverilog (nan / -inf).
#[test]
fn domain_errors_match_iverilog() {
    let out = run("module top;\n\
        initial begin\n\
          $display(\"sqrt_neg=%g\", $sqrt(-1.0));\n\
          $display(\"ln0=%g\", $ln(0.0));\n\
          $display(\"acos2=%g\", $acos(2.0));\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "sqrt_neg=nan\nln0=-inf\nacos2=nan\n");
}

/// IEEE-754 negative zero (from a `-0.0` literal or a real-math result like
/// `$ceil(-0.3)`/`$sin(-0.0)`) displays as a plain "0" across %f/%e/%g, matching
/// iverilog. `$atan2(-0.0, -1.0)` correctly stays −π (the sign of the −0.0 y arg
/// is meaningful and both tools agree on the direct-constant form).
#[test]
fn negative_zero_displays_as_positive() {
    let out = run("module top;\n\
        initial begin\n\
          $display(\"a=%f b=%e c=%g\", -0.0, -0.0, -0.0);\n\
          $display(\"ceil=%f sin=%f\", $ceil(-0.3), $sin(-0.0));\n\
          $display(\"atan2=%f\", $atan2(-0.0, -1.0));\n\
        end\n\
      endmodule\n");
    assert_eq!(
        out,
        "a=0.000000 b=0.000000e+00 c=0\nceil=0.000000 sin=0.000000\natan2=-3.141593\n"
    );
}

/// Non-finite reals spell `nan`/`inf`/`-inf` (C/iverilog lowercase) across ALL
/// real format specs — %f, %e, %g — not just %g. (Rust's Display gives "NaN";
/// coercing it is a $display-parity fix found by this oracle.)
#[test]
fn nonfinite_lowercase_across_all_specs() {
    let out = run("module top;\n\
        initial begin\n\
          $display(\"ff=%f|%f|%f\", $sqrt(-1.0), $exp(1000.0), -$exp(1000.0));\n\
          $display(\"ee=%e|%e\", $sqrt(-1.0), $exp(1000.0));\n\
          $display(\"gg=%g|%g\", $sqrt(-1.0), $exp(1000.0));\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "ff=nan|inf|-inf\nee=nan|inf\ngg=nan|inf\n");
}
