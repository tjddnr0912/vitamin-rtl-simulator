//! SV §6.19: an enum label value must be representable in the enum base type.
//! A value outside the range (`{X=16}` in a 4-bit base, a negative label in an
//! unsigned base, or an auto-incremented value that overflows) is an ERROR —
//! iverilog rejects it; vita previously TRUNCATED it silently (`{X=16}` in
//! `[3:0]` read 0, a genuine silent-wrong on invalid input). The parser now
//! emits a loud E2002 for const-foldable labels against a const-foldable base
//! width, and stays fail-open (never over-rejects) when either is unknown.
//! iverilog 13.0-pinned oracle.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src`; return (stdout, exit-code).
fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_elr_{}_{n}", std::process::id()));
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

#[test]
fn signed_base_overflow_is_loud() {
    // 8 does not fit a signed 4-bit base (range -8..7). iverilog: "value too large".
    let (out, code) = run(
        "module t;\n  typedef enum logic signed [3:0] {A=8} e_t;\n  e_t e;\n\
         initial begin e=A; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "signed overflow must be loud:\n{out}");
}

#[test]
fn unsigned_base_negative_is_loud() {
    // A negative label in an UNSIGNED base. iverilog: "has a negative value".
    // vita previously truncated -1 to 15 silently.
    let (out, code) = run(
        "module t;\n  typedef enum logic [3:0] {A=-1} e_t;\n  e_t e;\n\
         initial begin e=A; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "unsigned negative must be loud:\n{out}");
}

#[test]
fn unsigned_base_too_large_is_loud() {
    // 16 does not fit a 4-bit unsigned base (max 15). vita previously read 0.
    let (out, code) = run(
        "module t;\n  typedef enum logic [3:0] {X=16, Y=17} e_t;\n  e_t e;\n\
         initial begin e=X; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "too-large label must be loud:\n{out}");
}

#[test]
fn autoincrement_overflow_is_loud() {
    // A,B,C,D auto-assign 0..3 (fine for [1:0]); E overflows to 4. iverilog:
    // "inferred value overflowed". vita previously wrapped E to 0 silently.
    let (out, code) = run(
        "module t;\n  typedef enum logic [1:0] {A, B, C, D, E} e_t;\n  e_t e;\n\
         initial begin e=E; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "auto-increment overflow must be loud:\n{out}"
    );
}

#[test]
fn in_range_values_pass() {
    // Unsigned max (15 in [3:0]) and signed extremes (7, -8 in signed [3:0]) are
    // all valid — must NOT be rejected, and must read back exactly.
    let (out, code) = run("module t;\n\
         typedef enum logic [3:0] {U=15} u_t;\n\
         typedef enum logic signed [3:0] {A=7, B=-8} s_t;\n\
         u_t u; s_t a;\n\
         initial begin u=U; a=B; $display(\"%0d %0d\", u, a); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("15 -8"), "in-range values:\n{out}");
}

#[test]
fn atom_and_default_bases_pass() {
    // `byte` extremes (127, -128), a large `int`-default label (200), and a full
    // 8-bit unsigned (`bit [7:0]` = 255) are all in range — must pass.
    let (out, code) = run(
        "module t;\n\
         typedef enum byte {A=127, B=-128} a_t;\n\
         typedef enum {C=200} c_t;\n\
         typedef enum bit [7:0] {D=255} d_t;\n\
         a_t a; c_t c; d_t d;\n\
         initial begin a=B; c=C; d=D; $display(\"%0d %0d %0d\", a, c, d); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("-128 200 255"), "atom/default bases:\n{out}");
}

#[test]
fn param_width_base_is_failopen() {
    // The base width `[W-1:0]` is not const-foldable at parse time, so the range
    // check is skipped (fail-open) — even a label that would overflow a concrete
    // width must NOT be rejected here (honest: never over-reject on unknown width).
    let (out, code) = run(
        "module t #(parameter W=4);\n  typedef enum logic [W-1:0] {A=15, B=99} e_t;\n  e_t e;\n\
         initial begin e=A; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "param-width base must be fail-open:\n{out}");
}

#[test]
fn sized_literal_label_range_is_checked() {
    // A sized/based literal label used NOT to const-fold, so the range check was
    // skipped and an out-of-range value silently truncated. The enum-method slice
    // folds these labels, so the check now fires — and iverilog rejects the same
    // source ("has a value that is too large: 8'd255"), so this is silent-truncate
    // turning into a loud reject, not an over-rejection.
    let (out, code) = run(
        "module t;\n  typedef enum logic [3:0] {A=4'h5, B=8'hFF} e_t;\n  e_t e;\n\
         initial begin e=A; $display(\"%0d\", e); $finish; end\nendmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "an out-of-range sized label must be loud:\n{out}"
    );

    // An IN-range sized label is accepted and behaves like any other label.
    let (out, code) = run(
        "module t;\n  typedef enum logic [3:0] {A=4'h5, B=4'hF} e_t;\n  e_t e;\n\
         initial begin e=B; $display(\"R=%0d %s\", e, e.name()); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "an in-range sized label must work:\n{out}");
    assert!(out.contains("R=15 B"), "in-range sized label; got:\n{out}");
}

#[test]
fn unsigned_64bit_base_negative_is_loud() {
    // An UNSIGNED 64-bit base (`time`, `bit [63:0]`) rejects a negative label —
    // it is not representable in `[0, 2^64-1]`. iverilog: "has a negative value".
    // (Width-64 is checked, not skipped — only SIGNED 64-bit `longint` admits any
    // i64.) Regression guard for the soundness-review FINDING 1 fail-open gap.
    for base in ["time", "bit [63:0]", "logic [63:0]"] {
        let src = format!(
            "module t;\n  typedef enum {base} {{A=-1}} e_t;\n  e_t e;\n\
             initial begin e=A; $display(\"%0d\", e); $finish; end\nendmodule\n"
        );
        let (out, code) = run(&src);
        assert_ne!(
            code,
            Some(0),
            "unsigned-64 negative must be loud ({base}):\n{out}"
        );
    }
}

#[test]
fn signed_longint_admits_any_i64() {
    // A SIGNED 64-bit base admits every i64, so width-64 must NOT over-reject a
    // large or negative `longint` label (the flip side of FINDING 1's fix). Uses
    // ±i64::MAX (i64::MIN's `-(2^63)` literal is separately non-foldable → a
    // pre-existing E3009 unrelated to the range check).
    let (out, code) = run("module t;\n\
         typedef enum longint {A=-9223372036854775807, B=9223372036854775807} l_t;\n\
         l_t x;\n\
         initial begin x=B; $display(\"%0d\", x); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "longint extremes must pass:\n{out}");
    assert!(out.contains("9223372036854775807"), "longint max:\n{out}");
}
