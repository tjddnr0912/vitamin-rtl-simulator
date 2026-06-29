//! A chained typedef alias — `typedef base_t alias_t;`, where `base_t` is itself
//! an existing typedef (IEEE §6.18) — was a parse error: `parse_typedef` accepted
//! only `enum` / `struct` / `union` / a built-in type keyword after `typedef`, not
//! an existing typedef NAME. iverilog 13.0 supports it for every base kind.
//!
//! Fixed by a `parse_typedef_chained_alias` arm that resolves the base typedef's
//! `TypeInfo` (via peek_typedef_name) and mirrors its full registration —
//! type info plus any struct/union layout or enum-method binding — under the new
//! name, so a later `alias_t v;` / `v.field` / `v.next` resolves exactly as
//! `base_t v;` would. Adding packed or unpacked dimensions to an aliased type
//! (`typedef base_t [3:0] a_t;` / `typedef base_t a_t [4];`) needs type
//! composition not in v1 — honest-loud. Pure parser, AST/`.vu`/format unchanged,
//! IR-0. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_chtd_{}_{n}", std::process::id()));
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
fn vector_alias() {
    let (o, ok) = run("module top; typedef logic [7:0] b_t; typedef b_t a_t;\n\
         a_t x; initial begin x=8'hAB; $display(\"%h\", x); #1 $finish; end endmodule");
    assert!(ok && o == "ab", "got:\n{o}");
}

#[test]
fn signed_alias_reads_back_negative() {
    let (o, ok) = run(
        "module top; typedef logic signed [7:0] b_t; typedef b_t a_t;\n\
         a_t x; initial begin x=-3; $display(\"%0d\", x); #1 $finish; end endmodule",
    );
    assert!(ok && o == "-3", "got:\n{o}");
}

#[test]
fn enum_alias_value_and_method() {
    // The alias inherits the enum-method binding: `x.next` from R is G == 1.
    let (o, ok) = run(
        "module top; typedef enum logic [1:0] {R,G,B} b_t; typedef b_t a_t;\n\
         a_t x; initial begin x=R; $display(\"%0d %0d\", x, x.next); #1 $finish; end endmodule",
    );
    assert!(ok && o == "0 1", "got:\n{o}");
}

#[test]
fn struct_alias_field_access() {
    // The alias inherits the struct layout, so `x.a`/`x.b` desugar to part-selects.
    let (o, ok) = run(
        "module top; typedef struct packed {logic [3:0] a, b;} b_t; typedef b_t a_t;\n\
         a_t x; initial begin x.a=4'hC; x.b=4'hD; $display(\"%h %h %h\", x, x.a, x.b); #1 $finish; end endmodule",
    );
    assert!(ok && o == "cd c d", "got:\n{o}");
}

#[test]
fn union_alias_overlay() {
    // The alias inherits union-overlay semantics (a and b share bit 0).
    let (o, ok) = run(
        "module top; typedef union packed {logic [7:0] a; logic [7:0] b;} b_t; typedef b_t a_t;\n\
         a_t x; initial begin x.a=8'hCD; $display(\"%h\", x.b); #1 $finish; end endmodule",
    );
    assert!(ok && o == "cd", "got:\n{o}");
}

#[test]
fn int_alias() {
    let (o, ok) = run("module top; typedef int b_t; typedef b_t a_t;\n\
         a_t x; initial begin x=42; $display(\"%0d\", x); #1 $finish; end endmodule");
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn three_level_chain() {
    let (o, ok) = run(
        "module top; typedef logic [7:0] b_t; typedef b_t m_t; typedef m_t a_t;\n\
         a_t x; initial begin x=8'hAB; $display(\"%h\", x); #1 $finish; end endmodule",
    );
    assert!(ok && o == "ab", "got:\n{o}");
}

#[test]
fn alias_byte_identical_to_direct() {
    // The chained alias produces the same output as using the base type directly.
    let via_alias = run("module top; typedef logic [7:0] b_t; typedef b_t a_t;\n\
         a_t x; initial begin x=8'h5A; $display(\"%h\", x); #1 $finish; end endmodule");
    let via_direct = run("module top; typedef logic [7:0] b_t;\n\
         b_t x; initial begin x=8'h5A; $display(\"%h\", x); #1 $finish; end endmodule");
    assert!(via_alias.1 && via_direct.1, "both must run");
    assert_eq!(via_alias.0, via_direct.0, "alias must match direct");
    assert_eq!(via_alias.0, "5a");
}

#[test]
fn packed_dims_alias_is_loud() {
    // Adding packed dims to an aliased type needs type composition not in v1.
    let (_o, ok) = run(
        "module top; typedef logic [7:0] b_t; typedef b_t [3:0] a_t;\n\
         a_t x; initial begin #1 $finish; end endmodule",
    );
    assert!(!ok, "a dims-adding chained alias must be loud");
}

#[test]
fn unpacked_array_alias_is_loud() {
    let (_o, ok) = run(
        "module top; typedef logic [7:0] b_t; typedef b_t a_t [4];\n\
         a_t x; initial begin #1 $finish; end endmodule",
    );
    assert!(!ok, "an unpacked-array chained alias must be loud");
}
