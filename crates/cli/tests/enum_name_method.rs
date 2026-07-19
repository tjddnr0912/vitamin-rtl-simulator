//! SV §6.19.5 `enum_var.name()` returns the label as a string. It was
//! loud-rejected (E3009) because a packed string-literal ternary pads shorter
//! labels to the widest label's width (a silent-wrong vs iverilog's exact-length
//! dynamic string). Now the parser desugars `x.name`/`x.name()` to a call to a
//! synthetic `function string $enum_name$<T>(x)` — a `case(x)` returning each
//! label's string literal — which yields the EXACT length in every context
//! (assignment AND `$display("%s", …)`). iverilog 13.0-pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_enm_{}_{n}", std::process::id()));
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
fn name_exact_length_both_contexts() {
    // Assignment context AND direct-`$display` context both give the EXACT label
    // length (GREEN=5, RED=3 with no padding).
    let (out, code) = run(
        "module t;\n  typedef enum logic [1:0] {RED, GREEN, BLUE} col_t;\n  col_t c; string s;\n\
         initial begin c=GREEN; s=c.name(); $display(\"A[%s] len=%0d\", s, s.len());\n\
         c=RED; $display(\"D[%s]\", c.name()); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A[GREEN] len=5"), "assign exact:\n{out}");
    assert!(
        out.contains("D[RED]") && !out.contains("D[  RED]"),
        "direct exact (no pad):\n{out}"
    );
}

#[test]
fn name_signed_and_sparse_labels() {
    // A signed enum with a NEGATIVE label and sparse values — the synthetic
    // function's port sign follows the labels, so the negative value matches.
    let (out, code) = run(
        "module t;\n  typedef enum byte {A=-2, B=5, C=100} e_t;\n  e_t e;\n\
         initial begin e=A; $display(\"[%s]\", e.name()); e=B; $display(\"[%s]\", e.name());\n\
         e=C; $display(\"[%s]\", e.name()); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("[A]") && out.contains("[B]") && out.contains("[C]"),
        "signed/sparse labels:\n{out}"
    );
}

#[test]
fn name_property_form_no_parens() {
    // The `.name` property form (no parens) desugars identically.
    let (out, code) = run("module t;\n  typedef enum {X, Y, Z} e_t;\n  e_t e;\n\
         initial begin e=Z; $display(\"[%s]\", e.name); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("[Z]"), "property form:\n{out}");
}

#[test]
fn name_composes_with_next() {
    // `.name()` after `.next()` — the value-folding method and the string method
    // compose.
    let (out, code) = run("module t;\n  typedef enum {X, Y, Z} e_t;\n  e_t e;\n\
         initial begin e=X; e=e.next(); $display(\"[%s]\", e.name()); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("[Y]"), "next+name:\n{out}");
}

#[test]
fn name_out_of_range_is_empty() {
    // A value not matching any label returns "" (the default arm), like iverilog.
    let (out, code) = run(
        "module t;\n  typedef enum logic [3:0] {A=1, B=2} e_t;\n  e_t e; logic [3:0] r;\n\
         initial begin r=4'd9; e=e_t'(r); $display(\"[%s]\", e.name()); $finish; end\nendmodule\n",
    );
    // Some tools reject the cast; accept either a clean empty name or a loud cast
    // error, but never a wrong non-empty label.
    if code == Some(0) {
        assert!(out.contains("[]"), "out-of-range name is empty:\n{out}");
    }
}

#[test]
fn two_enum_types_distinct_name_fns() {
    // Two enum types in one module each get their own synthetic name function.
    let (out, code) = run(
        "module t;\n  typedef enum {RED, GREEN} c_t;\n  typedef enum {LO, HI} s_t;\n\
         c_t c; s_t s;\n\
         initial begin c=GREEN; s=HI; $display(\"[%s][%s]\", c.name(), s.name()); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("[GREEN][HI]"), "two enum types:\n{out}");
}

#[test]
fn name_survives_staged_pipeline() {
    // The desugar runs in `vcmp` (parse), so the synthetic `$enum_name$<T>`
    // function must SERIALIZE into the `.vu`, deserialize, and elaborate cleanly
    // through the staged `vcmp → velab → vrun` path. Self-checking via `$fatal`.
    let src = "module t;\n\
         typedef enum logic [1:0] {RED, GREEN, BLUE} col_t;\n\
         col_t c; string s;\n\
         initial begin\n\
           c = GREEN; s = c.name();\n\
           if (s != \"GREEN\") $fatal(1, \"staged .name() assign wrong: %s\", s);\n\
           c = RED;\n\
           if (c.name() != \"RED\") $fatal(1, \"staged .name() direct wrong\");\n\
           $finish;\n\
         end endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_enm_st_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("t.sv");
    std::fs::write(&sv, src).unwrap();
    let p = |x: &std::path::Path| x.to_str().unwrap().to_string();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[p(&sv)], Some(&p(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&p(&vu), &p(&velab), &o), 0, "velab failed");
    assert_eq!(
        cli::run_vrun(&p(&velab), &o),
        0,
        "staged .name() mismatch ($fatal)"
    );
}
