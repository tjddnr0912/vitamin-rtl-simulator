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

#[test]
fn ident_alias_inherits_signedness() {
    // `C = D` inherits D's signedness (both signed decimals) — so `s < C` is
    // signed like `s < D`, not the pre-fix unsigned residual.
    let (out, code) = run(
        "module t;\n  localparam D = 7; localparam C = D;\n  logic signed [3:0] s;\n\
         initial begin s = -1; $display(\"%0d %0d\", s < D, s < C); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 1"), "ident alias signed:\n{out}");
}

#[test]
fn expression_param_is_signed() {
    // `E = 3 + 4` (both signed) is signed; several operator forms.
    let (out, code) = run(
        "module t;\n  localparam A = 5, B = 3;\n  localparam SUM=A+B, PRD=A*B, SHL=A<<1, \
         TERN=(A>B)?A:B;\n  logic signed [3:0] s;\n\
         initial begin s = -1; $display(\"%0d %0d %0d %0d\", s<SUM, s<PRD, s<SHL, s<TERN); \
         $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 1 1 1"), "expression params signed:\n{out}");
}

#[test]
fn unsigned_expression_param_stays_unsigned() {
    // §11.8.1: an unsigned operand makes the whole expression (and the param)
    // unsigned — must not be flipped to signed by the fix.
    let (out, code) = run(
        "module t;\n  localparam UH = 8'hFF;\n  localparam UE = UH + 8'h01;\n\
         localparam MIX = UH + 5;\n  logic signed [3:0] s;\n\
         initial begin s = -1; $display(\"%0d %0d\", s < UE, s < MIX); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    // s (as unsigned in the collective compare) < unsigned RHS.
    assert!(
        out.contains("1 1"),
        "unsigned expression stays unsigned:\n{out}"
    );
}

#[test]
fn narrow_alias_inherits_width() {
    // Aliasing a narrow typed param keeps its width (ident-inherit returns the
    // source param's full meta, not a value-sized 32-bit).
    let (out, code) = run(&disp(
        "localparam [3:0] N = 4'ha; localparam C = N;",
        "%0d %0d",
        "$bits(N), $bits(C)",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("4 4"), "narrow alias width:\n{out}");
}

#[test]
fn nested_alias_chain_signed() {
    // `C = A` (negative), `E = C + 1` — signedness propagates through the chain.
    let (out, code) = run(
        "module t;\n  localparam A = -2; localparam C = A; localparam E = C + 1;\n\
         initial begin $display(\"%0d %0d %0d\", A, E, E < 0); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("-2 -1 1"), "nested alias chain:\n{out}");
}

#[test]
fn time_param_alias_does_not_inherit_sign() {
    // A `time` param aliasing a SIGNED param must keep its declared 64-bit
    // UNSIGNED type — the value-determined ident/expression paths are gated to
    // `Implicit` params, so `time C = D` does NOT inherit D's signedness. iverilog
    // compares unsigned here (x → 65535 < 5 = 0); a sign-inheriting bug gave 1.
    let (out, code) = run(
        "module t;\n  localparam signed [7:0] D = 5;\n  localparam time C = D;\n\
         logic signed [15:0] x;\n\
         initial begin x = -1; $display(\"%0d\", x < C); #100 $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('0'), "time alias must stay unsigned:\n{out}");
}

#[test]
fn pkg_scoped_alias_inherits_signedness() {
    // `C = p::X` inherits the package constant's signedness (X a signed decimal),
    // both as a bare alias and inside an expression — was unsigned pre-fix.
    let (out, code) = run("package p; localparam X = 7; endpackage\n\
         module t;\n  localparam C = p::X; localparam E = p::X + 1;\n\
         logic signed [3:0] s;\n\
         initial begin s = -1; $display(\"%0d %0d\", s < C, s < E); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 1"), "pkg-scoped alias signed:\n{out}");
}

#[test]
fn pkg_scoped_narrow_alias_inherits_width() {
    // A bare `p::N` alias of a narrow package param keeps its 4-bit width.
    let (out, code) = run("package p; localparam [3:0] N = 4'ha; endpackage\n\
         module t; localparam C = p::N;\n\
         initial begin $display(\"%0d\", $bits(C)); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('4'), "pkg-scoped narrow alias width:\n{out}");
}

#[test]
fn intra_package_alias_inherits_signedness() {
    // A package-INTERNAL alias/expression (`B = A`, `E = A + 1` inside the pkg)
    // resolves the sibling param's type — param_meta is made live during the
    // package fold. Read scoped from a module. Was unsigned pre-fix.
    let (out, code) = run(
        "package p; localparam A = 7; localparam B = A; localparam E = A + 1; endpackage\n\
         module t; logic signed [3:0] s;\n\
         initial begin s = -1; $display(\"%0d %0d\", s < p::B, s < p::E); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 1"), "intra-package alias signed:\n{out}");
}

#[test]
fn intra_package_narrow_alias_width() {
    // An intra-package alias of a narrow/signed sibling keeps its width.
    let (out, code) = run(
        "package p; localparam signed [7:0] A = 8'sd5; localparam B = A;\n\
         localparam [3:0] N = 4'ha; localparam M = N; endpackage\n\
         module t; initial begin $display(\"%0d %0d\", $bits(p::B), $bits(p::M)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("8 4"),
        "intra-package narrow alias width:\n{out}"
    );
}

#[test]
fn package_param_meta_does_not_pollute_module() {
    // The package fold makes its params' meta live only transiently; a module
    // param of the SAME NAME must keep its own type (restore, no pollution).
    let (out, code) = run("package p; localparam signed [7:0] X = -1; endpackage\n\
         module t; localparam X = 8'hFF; logic signed [15:0] s;\n\
         initial begin s = -1; $display(\"%0d %0d\", $bits(X), s < X); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    // Module X is its own unsigned 8-bit (not p's signed): $bits=8, compare unsigned.
    assert!(
        out.contains("8 0"),
        "no package→module param_meta pollution:\n{out}"
    );
}
