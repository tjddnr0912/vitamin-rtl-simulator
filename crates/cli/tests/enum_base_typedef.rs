//! An enum whose BASE type is a user-defined type name — `typedef logic[1:0]
//! b_t; typedef enum b_t {R,G,B} e_t;` — was a parse error ("expected '{' for
//! enum body"): the enum-base parser accepted only a built-in keyword or a bare
//! `[range]`, not a typedef name. iverilog 13.0 supports it.
//!
//! Fixed by a typedef-name arm in the enum-base parser: a SIMPLE UNSIGNED vector
//! typedef (logic/bit/reg [N]) resolves to its range as the enum base. A SIGNED
//! base (the built-in `enum logic signed[N]` path also drops signedness — a
//! separate pre-existing limit), an atom (int/byte), or a struct/class/multi-dim
//! typedef base cannot be represented by the enum's `Option<Range>` base model —
//! honest-loud. Pure parser, AST/`.vu`/format unchanged, IR-0. Pinned to iverilog
//! 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ebt_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

#[test]
fn vector_typedef_base() {
    let (o, ok) = run(
        "module top; typedef logic [1:0] b_t; typedef enum b_t {R,G,B} e_t;\n\
         e_t x; initial begin x=G; $display(\"%0d %h\", x, x); #1 $finish; end endmodule",
    );
    assert!(ok && o == "1 1", "got:\n{o}");
}

#[test]
fn wide_vector_typedef_base() {
    let (o, ok) = run(
        "module top; typedef logic [7:0] b_t; typedef enum b_t {R=8'hAA, G=8'hBB} e_t;\n\
         e_t x; initial begin x=G; $display(\"%h\", x); #1 $finish; end endmodule",
    );
    assert!(ok && o == "bb", "got:\n{o}");
}

#[test]
fn bit_typedef_base() {
    let (o, ok) = run(
        "module top; typedef bit [2:0] b_t; typedef enum b_t {R,G,B,C} e_t;\n\
         e_t x; initial begin x=C; $display(\"%0d\", x); #1 $finish; end endmodule",
    );
    assert!(ok && o == "3", "got:\n{o}");
}

#[test]
fn enum_method_on_typedef_based_enum() {
    // The enum still supports its value methods (`.next`) when based on a typedef.
    let (o, ok) = run(
        "module top; typedef logic [1:0] b_t; typedef enum b_t {R,G,B} e_t;\n\
         e_t x; initial begin x=R; $display(\"%0d\", x.next); #1 $finish; end endmodule",
    );
    assert!(ok && o == "1", "got:\n{o}");
}

#[test]
fn builtin_base_byte_identical() {
    // The built-in-keyword base path is unaffected.
    let (o, ok) = run("module top; typedef enum logic [1:0] {R,G,B} e_t;\n\
         e_t x; initial begin x=G; $display(\"%0d %h\", x, x); #1 $finish; end endmodule");
    assert!(ok && o == "1 1", "got:\n{o}");
}

#[test]
fn signed_typedef_base_is_loud() {
    // A signed typedef base would need signedness the enum base model drops (the
    // built-in `enum logic signed[N]` path is itself pre-existing-limited) —
    // honest-loud rather than silently unsigned.
    let (_o, ok) = run(
        "module top; typedef logic signed [3:0] b_t; typedef enum b_t {R=-1} e_t;\n\
         e_t x; initial begin x=R; $display(\"%0d\", x); #1 $finish; end endmodule",
    );
    assert!(!ok, "a signed typedef enum base must be loud");
}

#[test]
fn atom_typedef_base_is_loud() {
    // An atom (int) typedef base is signed and has no explicit range — honest-loud.
    let (_o, ok) = run(
        "module top; typedef int b_t; typedef enum b_t {R,G,B} e_t;\n\
         e_t x; initial begin x=B; $display(\"%0d\", x); #1 $finish; end endmodule",
    );
    assert!(!ok, "an atom typedef enum base must be loud");
}
