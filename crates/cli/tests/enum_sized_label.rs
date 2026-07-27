//! `enum bit [3:0] { A = 4'h3 }` made EVERY enum method loud — `.name()`,
//! `.first()`, `.next()`, `.num()` — where iverilog prints `A`/3/5/3. The parser's
//! `const_lit` folded only unsized DECIMAL literals, so one sized label set
//! `foldable = false` and the enum was never inserted into `enum_defs`. Without
//! that entry the parser never synthesizes the `.name()` case function and the
//! call falls through to the hierarchical-call path — which is also why the old
//! diagnostic misleadingly said "unsupported hierarchical function call".
//!
//! The values are needed at PARSE time (the parser builds `.name()`'s table), but
//! `parse_int_literal` lives in `elaborate`, which depends on this crate — a
//! cycle. So `const_lit_based` is a SECOND place that turns literal text into a
//! value, which is a real hazard: if the two ever disagree, the `.name()` table
//! and the elaborated constant point at different labels and only the NAME is
//! wrong — silently.
//!
//! These tests are the mitigation. Every case prints `name/value` TOGETHER, so a
//! divergence between the parser's fold and elaborate's cannot hide: the pair
//! would stop matching the source. That internal differential needs no oracle,
//! though every value below is also pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_esl_{}_{n}", std::process::id()));
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

/// THE divergence detector: the label NAME and the label VALUE are produced by two
/// different subsystems (the parser's `.name()` table vs elaborate's constant), so
/// printing them together pins that both agree with the source.
#[test]
fn every_based_literal_form_pairs_its_name_with_its_value() {
    let (out, c) = run("module m;\n\
           typedef enum bit [7:0] { A = 8'h0A, B = 8'sd5, C = 'b1011, D = 8'o17, \
                                    E = 8'd200 } e_t;\n\
           e_t x;\n\
           initial begin\n\
             x = A; $display(\"p=%s/%0d\", x.name(), x);\n\
             x = B; $display(\"p=%s/%0d\", x.name(), x);\n\
             x = C; $display(\"p=%s/%0d\", x.name(), x);\n\
             x = D; $display(\"p=%s/%0d\", x.name(), x);\n\
             x = E; $display(\"p=%s/%0d\", x.name(), x);\n\
             #1 $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    // Every label has a DISTINCT value on purpose: iverilog rejects duplicates,
    // so a test with two labels sharing one would be pinning invalid source.
    for want in ["p=A/10", "p=B/5", "p=C/11", "p=D/15", "p=E/200"] {
        assert!(out.contains(want), "expected `{want}`; got:\n{out}");
    }
}

/// The SIGN of a label comes from the ENUM BASE, not from the literal's `s`
/// marker (§6.19 — a label is a value OF the base type). Folding the marker's
/// sign instead rejected `enum integer { A = 32'hDEADBEEF }` as out of range and
/// gave `enum bit [7:0] { A = 8'shFF }` a `.name()` table keyed on −1 while the
/// constant was 255, so the name came back EMPTY. Both axes are pinned here:
/// unsigned literal in a signed base, and signed literal in an unsigned base.
#[test]
fn the_enum_base_decides_the_sign_not_the_literal_marker() {
    let (out, c) = run("module t;\n\
           typedef enum int      { A = 32'hFFFFFFFF } a_t;\n\
           typedef enum integer  { B = 32'hDEADBEEF } b_t;\n\
           typedef enum byte     { C = 8'hFF } c_t;\n\
           typedef enum shortint { D = 16'hFFFF } d_t;\n\
           typedef enum          { E = 32'hFFFFFFFF } e_t;\n\
           typedef enum logic signed [3:0] { F = 4'hF } f_t;\n\
           typedef enum bit [7:0] { G = 8'shFF } g_t;\n\
           typedef enum bit [3:0] { H = 4'sd12 } h_t;\n\
           typedef enum bit [3:0] { I = -4'sd1 } i_t;\n\
           a_t a; b_t b; c_t c; d_t d; e_t e; f_t f; g_t g; h_t h; i_t i;\n\
           initial begin a=A; b=B; c=C; d=D; e=E; f=F; g=G; h=H; i=I;\n\
             $display(\"S=%0d %0d %0d %0d %0d %0d %0d %0d %0d\", a,b,c,d,e,f,g,h,i);\n\
             #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "these are all valid designs; got:\n{out}");
    assert!(
        out.contains("S=-1 -559038737 -1 -1 -1 -1 255 12 15"),
        "base-typed label values; got:\n{out}"
    );
}

/// A signed literal in a NON-const-foldable base range is where the range check
/// cannot mask a wrong fold, so the name/value pairing is the only detector.
#[test]
fn a_param_width_base_still_pairs_name_with_value() {
    let (out, c) = run("module t;\n  parameter W = 8;\n\
           typedef enum logic [W-1:0] { A = 8'shFF, B = 8'h1 } e_t;\n  e_t x, y;\n\
           initial begin x = A; y = x.first();\n\
             $display(\"P=%0d/%0d/<%s>\", x, y, x.name()); #1 $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("P=255/255/<A>"),
        "param-width base; got:\n{out}"
    );
}

/// The methods that were loud purely because the enum never reached `enum_defs`.
#[test]
fn enum_methods_work_on_a_sized_label_enum() {
    let (out, c) = run("module m;\n\
           typedef enum bit [3:0] { A = 4'h3, B = 4'h5, C = 4'h9 } e_t;\n\
           e_t x, z;\n\
           initial begin\n\
             x = A;\n\
             $display(\"N=%s\", x.name());\n\
             z = x.next();  $display(\"NX=%0d\", z);\n\
             z = x.first(); $display(\"FS=%0d\", z);\n\
             z = x.last();  $display(\"LS=%0d\", z);\n\
             $display(\"NM=%0d\", x.num());\n\
             #1 $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    for want in ["N=A", "NX=5", "FS=3", "LS=9", "NM=3"] {
        assert!(out.contains(want), "expected `{want}`; got:\n{out}");
    }
}

/// Forms the parser's fold declines, so the enum keeps its previous behavior
/// rather than being handed a value this cannot reproduce exactly.
#[test]
fn unreproducible_label_forms_decline_rather_than_guess() {
    // An x/z digit has no integer value; a width above 64 leaves the i64 domain;
    // a fill literal is context-sized. Each leaves the enum unfolded, so the enum
    // METHOD stays loud — the pre-existing behavior, never a guessed value.
    // `'h1FFFFFFFF` is the important one: it exceeds the 32-bit default of an
    // UNSIZED based literal, and the two literal parsers size it differently
    // (elaborate grows the width, this one would mask). Declining truncation is
    // what keeps them provably in agreement on everything they DO accept.
    for label in [
        "4'bx1",
        "65'h1_0000_0000_0000_0000",
        "'1",
        "'h1FFFFFFFF",
        "4'hFF",
    ] {
        let (out, c) = run(&format!(
            "module m;\n  typedef enum bit [3:0] {{ A = {label} }} e_t;\n  e_t x;\n\
               initial begin x = A; $display(\"%s\", x.name()); #1 $finish; end\nendmodule\n"
        ));
        assert_ne!(
            c,
            Some(0),
            "label `{label}` must not fold silently; got:\n{out}"
        );
    }
}

/// The range check follows PROVENANCE at width 64: an explicitly written negative
/// in an unsigned base is an error, while an auto-incremented wrap past
/// `64'sh7FFF…` is a legal `logic [63:0]` pattern that iverilog accepts. Folding
/// sized labels is what made the second case reachable at all.
#[test]
fn width_64_range_check_follows_provenance() {
    let (out, c) = run("package p;\n\
           typedef enum logic [63:0] { A = 64'sh7FFF_FFFF_FFFF_FFFF, B } e_t;\n\
         endpackage\n\
         module top;\n  import p::*;\n  e_t e;\n\
           initial begin e = B; $display(\"W=%h\", e); $finish; end\nendmodule\n");
    assert_eq!(c, Some(0), "an auto-increment wrap is legal; got:\n{out}");
    assert!(
        out.contains("W=8000000000000000"),
        "wrapped value; got:\n{out}"
    );

    // An explicit negative in an unsigned 64-bit base stays loud.
    let (out, c) = run(
        "module t;\n  typedef enum bit [63:0] { A = -1 } e_t;\n  e_t e;\n\
           initial begin e = A; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_ne!(c, Some(0), "explicit negative must be loud; got:\n{out}");
}
