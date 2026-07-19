//! An UNTYPED `localparam`/`parameter` (no explicit type) must take its type
//! from its VALUE (IEEE §6.20.2). Before, a non-negative untyped DECIMAL was
//! made UNSIGNED 32-bit by the value-inferred fallback (`const_u32_expr`), so
//! `localparam A = -1, B = 2; A < B` compared UNSIGNED — vita said 0 (A treated
//! as a huge unsigned) where iverilog says 1 (both signed decimals, -1 < 2).
//! Now `param_decl_width` reads the value literal via `parse_int_literal`: a
//! plain decimal is SIGNED with a width grown to hold value+sign (32, or 33 for
//! a magnitude ≥ 2^31), a sized literal keeps its width, an unsized-based
//! literal keeps its base's sign; a leading unary `-`/`+` is peeled so a
//! negative literal sizes like its magnitude. iverilog 13.0-pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_upvs_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

/// Build a module with `decls` and a `$display(fmt, args)`.
fn disp(decls: &str, fmt: &str, args: &str) -> String {
    format!(
        "module t;\n  {decls}\n  initial begin $display(\"{fmt}\", {args}); $finish; end\nendmodule\n"
    )
}

#[test]
fn positive_decimal_param_is_signed_in_compare() {
    // The headline bug: both untyped decimals are signed → signed compare.
    let (out, code) = run(&disp("localparam A = -1, B = 2;", "%0d", "A < B"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('1'), "-1 < 2 must be signed (1):\n{out}");
}

#[test]
fn small_positive_vs_negative_param() {
    let (out, code) = run(&disp(
        "localparam SMALL = 5; localparam NEG = -1;",
        "%0d",
        "SMALL < NEG",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('0'), "5 < -1 signed = 0:\n{out}");
}

#[test]
fn large_decimal_param_width_and_sign() {
    // 3000000000 ≥ 2^31 → 33-bit SIGNED; %b is 33 bits, and it stays > a
    // negative param (signed compare), not wrapped unsigned.
    let (out, code) = run(&disp(
        "localparam BIG = 3000000000; localparam NEG = -1;",
        "%0d %0d",
        "$bits(BIG), BIG > NEG",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("33 1"), "big decimal 33-bit signed:\n{out}");
}

#[test]
fn negative_large_decimal_sizes_like_magnitude() {
    // -3000000000 sizes like its 33-bit magnitude, not the 64-bit fallback.
    let (out, code) = run(&disp(
        "localparam NB = -3000000000;",
        "%0d %0d",
        "$bits(NB), NB",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("33 -3000000000"), "neg-big 33-bit:\n{out}");
}

#[test]
fn boundary_widths() {
    // 2^31-1 → 32-bit; 2^31 and 2^32-1 → 33-bit (signed decimals grow for sign).
    let (out, code) = run(&disp(
        "localparam P31 = 2147483647, P32 = 2147483648, U = 4294967295;",
        "%0d %0d %0d",
        "$bits(P31), $bits(P32), $bits(U)",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("32 33 33"), "boundary widths:\n{out}");
}

#[test]
fn negative_power_of_two_widths() {
    // A negative EXACT power of two needs one fewer bit than its positive twin:
    // -2^31 → 32-bit (not 33), -2^32 → 33, -2^33 → 34. Sizing by the FOLDED value's
    // minimal signed width (not the magnitude literal's) is what gets this right.
    let (out, code) = run(&disp(
        "localparam N31 = -2147483648, N32 = -4294967296, N33 = -8589934592;",
        "%0d %0d %0d",
        "$bits(N31), $bits(N32), $bits(N33)",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("32 33 34"), "neg-power-of-two widths:\n{out}");
}

#[test]
fn based_literal_signedness_preserved() {
    // 'hFF unsigned (compare unsigned), 'shFF signed (positive value here).
    let (out, code) = run(&disp(
        "localparam H = 'hFF; localparam SH = 'shFF;",
        "%0d %0d",
        "H > -1, SH < 0",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0 0"), "based-literal sign:\n{out}");
}

#[test]
fn sized_literal_param_unchanged() {
    // A SIZED literal param keeps its declared width/sign (regression guard).
    let (out, code) = run(&disp(
        "localparam byte8 = 8'hAB; localparam [3:0] nib = 4'ha;",
        "%0d %h %h",
        "$bits(byte8), byte8, nib",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("8 ab a"), "sized literal unchanged:\n{out}");
}

#[test]
fn positive_decimal_param_as_width_and_count() {
    // Making a positive decimal signed must not perturb width/count/replicate use.
    let (out, code) = run("module t;\n  localparam W = 8;\n  logic [W-1:0] x;\n  \
         localparam CNT = 3;\n  \
         initial begin x = 8'hFF; $display(\"%0d %h\", x, {CNT{4'b1}}); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("255 111"), "width/replicate use:\n{out}");
}

#[test]
fn comma_list_mixed_sign_decimals() {
    // Untyped comma-list (prior slice) with mixed-sign decimal values.
    let (out, code) = run(&disp(
        "localparam A = 1, B = -2, C = 3;",
        "%0d %0d",
        "A < B, C > B",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0 1"), "comma-list mixed sign:\n{out}");
}

#[test]
fn typed_params_unchanged() {
    // int/ranged params keep declared signedness (regression guard).
    let (out, code) = run(&disp(
        "localparam int TI = -1; localparam int unsigned TU = -1;",
        "%0d %0d",
        "TI < 0, TU < 0",
    ));
    assert_eq!(code, Some(0), "{out}");
    // TI signed (−1 < 0 = 1); TU unsigned (huge, < 0 = 0).
    assert!(out.contains("1 0"), "typed params unchanged:\n{out}");
}
