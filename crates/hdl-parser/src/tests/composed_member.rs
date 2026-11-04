//! `elem_t [hi:0] m` and the declaration forms that travel with it
//! (`structs/member_array.rs`): the node each select leaves as, against the hand form
//! the guard was measured on, and every shape that stays refused.
//!
//! The hand text in `same` is what `crates/cli/tests/composed_member.rs` runs through
//! the real lowering; a pin here holds the natural text to it, spans aside. The refused
//! shapes are the two oracles' business only where a value would exist: here each one
//! names the fact that keeps it out.
use super::*;

const PKG: &str = "package p;\n\
  typedef struct packed { logic [1:0] q; logic e; } e_t;\n\
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;\n\
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;\n\
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;\n\
  typedef struct packed { logic q; } b_t;\n\
  typedef struct packed { b_t [7:0] n; } w_t;\n\
  typedef e_t [3:0] vec_t;\n\
  parameter vec_t D = '0;\n\
  endpackage\n";

fn top(body: &str) -> String {
    format!(
        "{PKG}module top(input p::r_t pi, input p::vec_t pv, output logic o);\n\
         import p::*;\nr_t s; s_t t; w_t u; int k; byte b; logic [3:0] d; logic x;\n\
         {body}\nendmodule\n"
    )
}

fn errors(src: &str) -> Vec<String> {
    p(src)
        .1
        .iter()
        .map(|e| format!("expected {}, found {:?}", e.expected, e.found))
        .collect()
}

/// `Span { lo: 1, hi: 2 }` erased from a `Debug` print.
fn no_spans(s: &str) -> String {
    const OPEN: &str = "Span {";
    let (mut out, mut rest) = (String::new(), s);
    while let Some(i) = rest.find(OPEN) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        rest = &tail[tail.find('}').expect("a span closes") + 1..];
    }
    out + rest
}

fn module_ast(body: &str) -> String {
    let src = top(body);
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "`{body}`: {errs:?}");
    let su = su.unwrap();
    no_spans(&format!("{:?}", su.items.last().unwrap()))
}

/// The natural text and the hand form are one tree. A `for` stands at one offset on
/// both sides, so its renamed variable is one name.
#[track_caller]
fn same(natural: &str, hand: &str) {
    assert_eq!(module_ast(natural), module_ast(hand), "{natural}");
}

#[track_caller]
fn refused(body: &str, fragment: &str) {
    let e = errors(&top(body));
    assert!(
        e.iter().any(|m| m.contains(fragment)),
        "`{body}` wants `{fragment}`: {e:?}"
    );
}

#[track_caller]
fn accepted(body: &str) {
    let e = errors(&top(body));
    assert!(e.is_empty(), "`{body}`: {e:?}");
}

#[track_caller]
fn decl_refused(decl: &str, fragment: &str) {
    let src = format!("package q;\n{decl}\nendpackage\nmodule top; endmodule\n");
    let e = errors(&src);
    assert!(
        e.iter().any(|m| m.contains(fragment)),
        "`{decl}` wants `{fragment}`: {e:?}"
    );
}

const N1: &str = "packed array of a packed struct as a struct member";
const N2: &str = "a member type with no packed-array member";
const N3: &str = "no packed-array struct member";
const N4: &str = "on a packed-array struct member";
const N5: &str = "this packed dimension on a named type";
const NOT_ID: &str = "expected identifier, found Some(LBracket)";

// ---------------------------------------------------------------- the two node forms

#[test]
fn literal_read_is_the_flat_part_select() {
    same("assign x = s.m[2].e;", "assign x = s[9:9];");
    same("assign d[1:0] = s.m[3].q;", "assign d[1:0] = s[14:13];");
    same("assign d[2:0] = s.m[0];", "assign d[2:0] = s[5:3];");
}

#[test]
fn literal_write_is_the_flat_part_select() {
    same("assign s.m[2].q = d[1:0];", "assign s[11:10] = d[1:0];");
    same("initial s.m[1].e = x;", "initial s[6:6] = x;");
    same(
        "always_ff @(posedge x) s.m[1].e <= x;",
        "always_ff @(posedge x) s[6:6] <= x;",
    );
    same(
        "initial {s.m[1].e, o} = 2'b0;",
        "initial {s[6:6], o} = 2'b0;",
    );
}

#[test]
fn literal_reaches_a_nested_leaf() {
    same("assign d[1:0] = t.v[1].n.h;", "assign d[1:0] = t[14:13];");
    same("assign x = t.v[0].k;", "assign x = t[8:8];");
    same("assign d[2:0] = t.v[1].n;", "assign d[2:0] = t[15:13];");
}

#[test]
fn each_index_form_reads_the_measured_tree() {
    for (index, hand) in [
        ("i", "(i)"),
        ("2*i", "(2*i)"),
        ("2*i+1", "(2*i+1)"),
        ("31-i", "(31-i)"),
        ("i+8", "(i+8)"),
    ] {
        same(
            &format!("always_comb for (int i = 0; i < 4; i++) d[1:0] = s.m[{index}].q;"),
            &format!(
                "always_comb for (int i = 0; i < 4; i++) \
                 d[1:0] = s[({hand} >= 0 && {hand} < 4) ? {hand}*3+4 : 2097152 +: 2];"
            ),
        );
    }
}

#[test]
fn each_index_form_writes_the_measured_tree() {
    for (index, hand) in [
        ("i", "(i)"),
        ("2*i", "(2*i)"),
        ("2*i+1", "(2*i+1)"),
        ("31-i", "(31-i)"),
        ("i+8", "(i+8)"),
    ] {
        same(
            &format!("always_comb for (int i = 0; i < 4; i++) s.m[{index}].e = x;"),
            &format!(
                "always_comb for (int i = 0; i < 4; i++) \
                 s[({hand} >= 0 && {hand} < 4) ? {hand}*3+3 : 2097152 +: 1] = x;"
            ),
        );
    }
}

#[test]
fn run_time_whole_element_and_depth_two() {
    same(
        "always_comb for (int unsigned i = 0; i < 4; i++) d[2:0] = s.m[i];",
        "always_comb for (int unsigned i = 0; i < 4; i++) \
         d[2:0] = s[((i) >= 0 && (i) < 4) ? (i)*3+3 : 2097152 +: 3];",
    );
    same(
        "always_comb for (int i = 0; i < 2; i++) d[1:0] = t.v[i].n.h;",
        "always_comb for (int i = 0; i < 2; i++) \
         d[1:0] = t[((i) >= 0 && (i) < 2) ? (i)*4+9 : 2097152 +: 2];",
    );
}

#[test]
fn genvar_in_a_continuous_assign() {
    same(
        "for (genvar g = 0; g < 4; g++) assign d[g] = s.m[g].e;",
        "for (genvar g = 0; g < 4; g++) assign d[g] = s[((g) >= 0 && (g) < 4) ? (g)*3+3 : 2097152 +: 1];",
    );
    same(
        "for (genvar g = 0; g < 4; g++) assign d[g] = pi.m[3-g].e;",
        "for (genvar g = 0; g < 4; g++) assign d[g] = pi[((3-g) >= 0 && (3-g) < 4) ? (3-g)*3+3 : 2097152 +: 1];",
    );
    same(
        "for (genvar g = 0; g < 4; g++) assign s.m[g].e = d[g];",
        "for (genvar g = 0; g < 4; g++) assign s[((g) >= 0 && (g) < 4) ? (g)*3+3 : 2097152 +: 1] = d[g];",
    );
    same(
        "for (genvar g = 0; g < 2; g++) assign x = s.m[g].e, s.m[g].e = d[g];",
        "for (genvar g = 0; g < 2; g++) assign x = s[((g) >= 0 && (g) < 4) ? (g)*3+3 : 2097152 +: 1], s[((g) >= 0 && (g) < 4) ? (g)*3+3 : 2097152 +: 1] = d[g];",
    );
}

#[test]
fn stride_one_element_is_an_element_not_a_bit_arm() {
    same(
        "always_comb for (int i = 0; i < 8; i++) d[0] = u.n[i].q;",
        "always_comb for (int i = 0; i < 8; i++) \
         d[0] = u[((i) >= 0 && (i) < 8) ? (i)*1+0 : 2097152 +: 1];",
    );
    same(
        "always_comb for (int i = 0; i < 8; i++) d[0] = u.n[i];",
        "always_comb for (int i = 0; i < 8; i++) \
         d[0] = u[((i) >= 0 && (i) < 8) ? (i)*1+0 : 2097152 +: 1];",
    );
}

// ------------------------------------------------------------ member declaration, D1

#[test]
fn outside_a_named_packed_package_struct_the_parser_says_what_it_said() {
    let e = "typedef struct packed { logic a; } e_t;\n";
    decl_refused(
        &format!("{e}typedef union packed {{ e_t [1:0] m; logic [1:0] z; }} u_t;"),
        NOT_ID,
    );
    decl_refused(
        &format!("{e}typedef struct packed {{ struct packed {{ e_t [1:0] m; }} n; }} r_t;"),
        NOT_ID,
    );
    decl_refused(
        &format!("{e}typedef struct {{ e_t [1:0] m; }} r_t;"),
        NOT_ID,
    );
    let e = errors(
        "module top; typedef struct packed { logic a; } e_t;\n\
         typedef struct packed { e_t [1:0] m; } r_t; endmodule\n",
    );
    assert!(e.iter().any(|m| m.contains(NOT_ID)), "{e:?}");
    let e = errors(
        "typedef struct packed { logic a; } e_t;\n\
         typedef struct packed { e_t [1:0] m; } r_t; module top; endmodule\n",
    );
    assert!(e.iter().any(|m| m.contains(NOT_ID)), "{e:?}");
}

#[test]
fn a_member_of_another_form_is_named() {
    let e = "typedef struct packed { logic a; logic b; } e_t;\n";
    let holder = |m: &str| format!("{e}typedef struct packed {{ {m}; }} r_t;");
    for m in [
        "e_t [1:0][1:0] m", // two dimensions
        "e_t [4:1] m",      // non-zero LSB
        "e_t [0:3] m",      // ascending
    ] {
        decl_refused(&holder(m), N1);
    }
    // A bound that does not fold is a refusal, not a per-instance layout.
    decl_refused(&holder("e_t [$bits(zz)-1:0] m"), N1);
    // A bound past `int`, and past the net limit, by the arithmetic of neither type.
    decl_refused(&holder("e_t [2147483648:0] m"), N1);
    decl_refused(&holder("e_t [600000:0] m"), N1);
    decl_refused(&holder("e_t [4294967295:0] m"), N1);
    // Bases.
    decl_refused(
        "typedef logic [3:0] v_t; typedef struct packed { v_t [3:0] m; } r_t;",
        N1,
    );
    decl_refused(
        "typedef enum logic { A, B } e_t; typedef struct packed { e_t [3:0] m; } r_t;",
        N1,
    );
    decl_refused(
        "typedef union packed { logic [1:0] a; logic [1:0] b; } u_t;\n\
         typedef struct packed { u_t [3:0] m; } r_t;",
        N1,
    );
    decl_refused(
        "typedef struct packed signed { logic [1:0] a; } e_t;\n\
         typedef struct packed { e_t [3:0] m; } r_t;",
        N1,
    );
    // Leaves: a signed leaf reads as a value, a 2-state leaf reads 0 past the guard.
    decl_refused(
        "typedef struct packed { logic signed [1:0] a; } e_t;\n\
         typedef struct packed { e_t [3:0] m; } r_t;",
        N1,
    );
    decl_refused(
        "typedef struct packed { bit a; logic b; } e_t;\n\
         typedef struct packed { e_t [3:0] m; } r_t;",
        N1,
    );
    decl_refused(
        "typedef struct packed { bit a; } i_t; typedef struct packed { i_t i; logic b; } e_t;\n\
         typedef struct packed { e_t [3:0] m; } r_t;",
        N1,
    );
    decl_refused(
        "typedef union packed { logic [1:0] a; logic [1:0] b; } u_t;\n\
         typedef struct packed { u_t i; logic b; } e_t;\n\
         typedef struct packed { e_t [3:0] m; } r_t;",
        N1,
    );
}

/// The element's own members, against the census of what the pages declare (row 41
/// PLAN2): each leaf by the form its declaration wrote, never by the absence of a flag.
/// One shape per term, each refused by the member rule; an alias of the element by the
/// alias rule.
#[test]
fn an_element_of_another_form_is_named() {
    let holder = |e: &str| format!("{e}\ntypedef struct packed {{ e_t [3:0] m; }} r_t;");
    let alias = |e: &str| format!("{e}\ntypedef e_t [3:0] a_t;");
    for e in [
        // A leaf of another keyword, ascending, off zero, of two dimensions.
        "typedef struct packed { reg [1:0] a; } e_t;",
        "typedef struct packed { logic [0:1] a; } e_t;",
        "typedef struct packed { logic [2:1] a; } e_t;",
        "typedef struct packed { logic [1:0][1:0] a; } e_t;",
        "typedef struct packed { logic [1:0][0:0] a; logic e; } e_t;", // k09
        // A signed leaf, a leaf with `unsigned` written, a 2-state leaf, a union.
        "typedef struct packed { logic signed [1:0] a; } e_t;",
        "typedef struct packed { logic unsigned [1:0] a; } e_t;",
        "typedef struct packed { bit [1:0] a; logic e; } e_t;",
        "typedef union packed { logic [1:0] a; logic [1:0] b; } e_t;",
        // An anonymous union one level down: it records no member form.
        "typedef struct packed { union packed { logic [1:0] a; logic [1:0] b; } u; logic e; } e_t;",
        // An enum leaf of a 4-state and of a 2-state base: the kind is `Logic` for both.
        "typedef enum logic [1:0] { A, B } k_t; typedef struct packed { k_t a; } e_t;",
        "typedef enum bit [1:0] { A, B } k_t; typedef struct packed { k_t a; logic e; } e_t;",
        // A type parameter, of a `logic` vector, a `reg` vector and an enum of a 2-state
        // base (lens r2 F1, f01_tp_cast).
        "localparam type T = logic [1:0]; typedef struct packed { T a; logic e; } e_t;",
        "localparam type T = reg [1:0]; typedef struct packed { T a; logic e; } e_t;",
        "typedef enum bit [1:0] { A = 2'(1), B } be_e; localparam type T = be_e;\n\
         typedef struct packed { T k; logic e; } e_t;",
        // A vector type name of another form: another kind, a sign keyword, a scalar, a
        // chained name, a bound not a decimal literal, a page vector two levels down.
        "typedef reg [1:0] v_t; typedef struct packed { v_t a; } e_t;",
        "typedef logic unsigned [1:0] v_t; typedef struct packed { v_t a; } e_t;",
        "typedef logic v_t; typedef struct packed { v_t a; logic e; } e_t;",
        "typedef logic [1:0] v_t; typedef v_t w_t; typedef struct packed { w_t a; } e_t;",
        "parameter int W = 2; typedef logic [W-1:0] v_t; typedef struct packed { v_t a; } e_t;",
        "parameter int Z = 0; typedef logic [1:Z] v_t; typedef struct packed { v_t a; } e_t;",
        "typedef logic [2'd1:0] v_t; typedef struct packed { v_t a; } e_t;",
        "typedef logic [2-1:0] v_t; typedef struct packed { v_t a; } e_t;",
        "typedef struct packed { logic a; } x_t; typedef logic [$bits(x_t):0] v_t;\n\
         typedef struct packed { v_t a; } e_t;",
        "typedef logic [1:0] v_t; typedef struct packed { v_t a; } i_t;\n\
         typedef struct packed { i_t i; logic q; } e_t;",
        // An escaped page-vector name.
        "typedef logic [1:0] \\v$t ; typedef struct packed { \\v$t a; logic e; } e_t;",
        // A signed struct, named or anonymous; one with `unsigned` written, named or
        // anonymous, nested or the element itself.
        "typedef struct packed signed { logic [3:0] a; } g_t;\n\
         typedef struct packed { g_t s; logic q; } e_t;",
        "typedef struct packed { struct packed signed { logic [3:0] a; } s; logic q; } e_t;",
        "typedef struct packed unsigned { logic [3:0] a; } g_t;\n\
         typedef struct packed { g_t s; logic q; } e_t;",
        "typedef struct packed { struct packed unsigned { logic [3:0] a; } s; logic q; } e_t;",
        "typedef struct packed unsigned { logic [3:0] a; logic q; } e_t;",
        // A struct two levels down, every leaf plain, named or anonymous.
        "typedef struct packed { logic a; } i_t; typedef struct packed { i_t i; } n_t;\n\
         typedef struct packed { n_t n; logic q; } e_t;",
        "typedef struct packed { struct packed { struct packed { logic a; } i; } n; } e_t;",
        // A nested struct of a chained name.
        "typedef struct packed { logic a; } s0_t; typedef s0_t s_t;\n\
         typedef struct packed { s_t n; logic q; } e_t;",
        // An element of a chained name.
        "typedef struct packed { logic [1:0] a; logic b; } e0_t; typedef e0_t e_t;",
        // An enum leaf one level down.
        "typedef enum bit [1:0] { A, B } k_t; typedef struct packed { k_t k; } i_t;\n\
         typedef struct packed { i_t i; logic q; } e_t;",
        // A struct or union one level down with no member (IEEE 1800 Syntax 7-1 wants
        // one; lens r3 N-E h2_empty_nested): an empty member list is no fact.
        "typedef struct packed { struct packed { } n; logic [3:0] a; logic [1:0] b; } e_t;",
        "typedef struct packed { union packed { } u; logic [1:0] x; logic y; } e_t;",
    ] {
        decl_refused(&holder(e), N1);
        decl_refused(&alias(e), N5);
    }
    // Declared outside the holder's package: imported, scoped, `$unit`.
    let ext = "package x;\n\
               typedef logic [1:0] v_t; typedef struct packed { logic a; } s_t;\n\
               typedef struct packed { logic [1:0] a; logic b; } e_t;\n\
               typedef struct packed { logic [1:0] a; logic b; } e0_t; typedef e0_t c_t;\n\
               endpackage\n";
    let unit = "typedef logic [1:0] v_t; typedef struct packed { logic a; } s_t;\n\
                typedef struct packed { logic [1:0] a; logic b; } e_t;\n";
    let refused_in = |pre: &str, decl: &str, fragment: &str| {
        let src = format!("{pre}package q;\n{decl}\nendpackage\nmodule top; endmodule\n");
        let e = errors(&src);
        assert!(
            e.iter().any(|m| m.contains(fragment)),
            "`{decl}` wants `{fragment}`: {e:?}"
        );
    };
    for (pre, e) in [
        (ext, "import x::*; typedef struct packed { v_t a; } e_t;"),
        (ext, "import x::v_t; typedef struct packed { v_t a; } e_t;"),
        (ext, "typedef struct packed { x::v_t a; } e_t;"),
        (unit, "typedef struct packed { v_t a; logic b; } e_t;"),
        (
            ext,
            "import x::*; typedef struct packed { s_t n; logic b; } e_t;",
        ),
        (
            ext,
            "import x::s_t; typedef struct packed { s_t n; logic b; } e_t;",
        ),
        (ext, "typedef struct packed { x::s_t n; logic b; } e_t;"),
        (unit, "typedef struct packed { s_t n; logic b; } e_t;"),
    ] {
        refused_in(pre, &holder(e), N1);
        refused_in(pre, &alias(e), N5);
    }
    for (pre, h) in [
        (
            ext,
            "import x::*; typedef struct packed { e_t [3:0] m; } r_t;",
        ),
        (
            ext,
            "import x::e_t; typedef struct packed { e_t [3:0] m; } r_t;",
        ),
        (unit, "typedef struct packed { e_t [3:0] m; } r_t;"),
        (ext, "typedef struct packed { x::c_t [3:0] m; } r_t;"),
        (
            "",
            "typedef struct packed { logic a; } \\e$t ;\n\
             typedef struct packed { \\e$t [3:0] m; } r_t;",
        ),
    ] {
        refused_in(pre, h, N1);
    }
    // A struct holding an element array of width one: its member is no leaf.
    let h = "typedef struct packed { logic q; } b_t; typedef struct packed { b_t [1:0] n; } e_t;";
    decl_refused(&alias(h), N5);
    // In a subroutine body nothing is a site: the holder, and an alias of a package
    // element, say what they said (`a simple chained typedef alias`).
    decl_refused(
        "function automatic int f();\n\
         typedef struct packed { logic a; } e_t; typedef struct packed { e_t [3:0] m; } r_t;\n\
         return 0; endfunction",
        NOT_ID,
    );
    decl_refused(
        "typedef struct packed { logic a; } e_t;\n\
         function automatic int f(); typedef struct packed { e_t [3:0] m; } r_t; return 0;\n\
         endfunction",
        NOT_ID,
    );
    decl_refused(
        "typedef struct packed { logic a; } e_t;\n\
         function automatic int f(); typedef e_t [3:0] a_t; return 0; endfunction",
        "a simple chained typedef alias",
    );
    // A `begin` block of an `initial` or `always_comb` the parser takes in a package
    // (no package item; sv2v, verilator and iverilog stop at the keyword): outside a
    // subroutine, so only the body flag keeps the alias rule away (row 41 r2b, U38).
    for blk in ["initial", "always_comb"] {
        decl_refused(
            &format!(
                "typedef struct packed {{ logic a; }} e_t;\n\
                 {blk} begin typedef e_t [3:0] a_t; end"
            ),
            "a simple chained typedef alias",
        );
    }
}

/// A typedef of another kind over a packed struct's name leaves the struct's packed slot
/// behind (§5.2 row 41 P1, PLAN2 probes qs2-qs10): the element, the leaf and the alias
/// base are read by the declaration the typedef slot names, never by that slot.
#[test]
fn a_name_another_declaration_left_is_refused() {
    let unit = "typedef struct packed { logic [1:0] a; logic b; } e_t;\n\
                typedef struct packed { logic a; } v_t;\n";
    for (decl, fragment) in [
        // qs6: the element a package vector over a `$unit` struct.
        (
            "typedef logic [5:0] e_t; typedef struct packed { logic pre; e_t [1:0] m; } h_t;",
            N1,
        ),
        // qs7: a page vector over a `$unit` struct, as a leaf.
        (
            "typedef logic [1:0] v_t; typedef struct packed { v_t x; logic y; } e_t;\n\
             typedef struct packed { logic pre; e_t [1:0] m; } h_t;",
            N1,
        ),
        // qs10: the alias base.
        ("typedef logic [5:0] e_t; typedef e_t [1:0] a_t;", N5),
    ] {
        let src = format!("{unit}package p;\n{decl}\nendpackage\nmodule top; endmodule\n");
        let e = errors(&src);
        assert!(e.iter().any(|m| m.contains(fragment)), "`{decl}`: {e:?}");
    }
    // qs2, qs3, qs5: in a function body the holder is refused before the slot is read.
    for decl in [
        "typedef logic [W-1:0] v_t; typedef struct packed { v_t x; logic y; } e_t;",
        "typedef logic [1:0] v_t; typedef struct packed { v_t x; logic y; } e_t;",
        "typedef logic [5:0] e_t;",
    ] {
        let src = format!(
            "package p; parameter int W = 2;\n\
             typedef struct packed {{ logic a; }} v_t;\n\
             typedef struct packed {{ logic [1:0] a; logic b; }} e_t;\n\
             function automatic int f(); {decl}\n\
             typedef struct packed {{ logic pre; e_t [1:0] m; }} r_t; return 0; endfunction\n\
             endpackage\nmodule top; endmodule\n"
        );
        let e = errors(&src);
        assert!(e.iter().any(|m| m.contains(NOT_ID)), "`{decl}`: {e:?}");
    }
}

/// The forms the census holds are admitted: a `logic` leaf of a parameter's width or of
/// `[0:0]`, a page vector type name (`[D:0]`, D = 0 too), two declarators of one type,
/// an anonymous and a named struct of them one level down; the element named `p::e_t`
/// from another package; the racl alias (`top_racl_pkg.sv:38 :41 :55`, mR04).
#[test]
fn an_element_of_the_pages_form_is_admitted() {
    for e in [
        "parameter int W = 4; typedef struct packed { logic [W-1:0] a; logic b; } e_t;",
        "typedef logic [1:0] v_t; typedef struct packed { v_t a; logic b, c; } e_t;",
        "typedef logic [0:0] v_t; typedef struct packed { v_t a; logic b; } e_t;",
        "typedef struct packed { logic [0:0] a; logic e; } e_t;",
        "typedef struct packed { struct packed { logic q; logic [9:0] v; } en; logic w; } e_t;",
        "typedef struct packed { logic q; logic qe; } i_t;\n\
         typedef struct packed { i_t i; logic [3:0] w; } e_t;",
    ] {
        for d in [
            format!("{e}\ntypedef struct packed {{ e_t [3:0] m; }} r_t;"),
            format!("{e}\ntypedef e_t [3:0] a_t;"),
        ] {
            let src = format!("package q;\n{d}\nendpackage\nmodule top; endmodule\n");
            let e = errors(&src);
            assert!(e.is_empty(), "`{d}`: {e:?}");
        }
    }
    for src in [
        // q03: the holder in q, the element `p::e_t`.
        "package p; typedef logic [1:0] v_t; typedef struct packed { v_t a; logic b; } e_t;\n\
         endpackage\npackage q; typedef struct packed { logic pre; p::e_t [1:0] m; } r_t;\n\
         endpackage\nmodule top; endmodule\n",
        // a body-local typedef before the element: the body ends, the package's sites stand.
        "package q; function automatic int f(); typedef logic [1:0] t_t; return 0; endfunction\n\
         typedef logic [1:0] v_t; typedef struct packed { v_t a; logic b; } e_t;\n\
         typedef struct packed { logic pre; e_t [1:0] m; } r_t; endpackage\nmodule top; endmodule\n",
        // the racl shape, mR04.
        "package top_racl_pkg;\n\
         parameter int unsigned NrRaclPolicies = 3;\n\
         typedef logic [1:0] racl_role_vec_t;\n\
         typedef struct packed { racl_role_vec_t read_perm; racl_role_vec_t write_perm; }\n\
         racl_policy_t;\n\
         typedef racl_policy_t [NrRaclPolicies-1:0] racl_policy_vec_t;\n\
         endpackage\nmodule top; endmodule\n",
    ] {
        let e = errors(src);
        assert!(e.is_empty(), "`{src}`: {e:?}");
    }
}

/// The two refusal texts §5.2 row 41 adds for the element and the alias, whole.
const MEMBER: &str = "a packed array of a packed struct as a struct member: `elem_t [hi:0] m`, \
     hi >= 0 a constant, in a named packed struct declared at the top of this package whose \
     every member has a constant width, within the net width limit; `elem_t` a packed struct \
     with no sign keyword, named by its own declaration (not a typedef alias of it), bare for \
     one declared at the top of this package or as `pkg::name` for one declared at the top of \
     `pkg`, no name escaped, whose members are each `logic`, `logic [N:0]` with N >= 0, the \
     bare name of a `typedef logic [D:0]` declared at the top of the element's package (D a \
     decimal literal) that no packed struct or union in `$unit` or a wildcard-imported package \
     also names, or a `struct packed` with no sign keyword of one or more such `logic` and \
     `logic [N:0]` members, anonymous or named bare by its own declaration at the top of the \
     element's package (any other `elem_t [hi:lo] m` is unsupported in v1)";
const FORM: &str = "a package-level `typedef elem_t [hi:0] name;`, hi >= 0 a constant, of an \
     `elem_t` a packed-array struct member admits; an ANSI port `pkg::name p` of such an alias \
     with no dimension or default after its name; or a function return, or a function or task \
     ANSI formal written `input`, `v_t [l:r]` in this package, `l` and `r` integer literals or \
     `+`, `-`, `*` of them, `v_t` the bare name of a `typedef logic [D:0]` declared at the top \
     of this package (D a decimal literal) that no packed struct or union in `$unit` or a \
     wildcard-imported package also names (this packed dimension on a named type is \
     unsupported in v1)";

#[test]
fn the_refusal_texts_are_whole() {
    let e = "typedef struct packed { reg [1:0] a; } e_t;";
    decl_refused(
        &format!("{e}\ntypedef struct packed {{ e_t [3:0] m; }} r_t;"),
        &format!("expected {MEMBER}, found "),
    );
    decl_refused(
        &format!("{e}\ntypedef e_t [3:0] a_t;"),
        &format!("expected {FORM}, found "),
    );
}

/// Each text is true where it is printed (lens r3 C1, C2, diff T-A): every shape here is
/// one the text leaves out, by the clause named beside it.
#[test]
fn each_refusal_text_leaves_out_what_it_refuses() {
    let whole = |src: &str, text: &str| {
        let e = errors(src);
        assert!(
            e.iter()
                .any(|m| m.starts_with(&format!("expected {text}, found "))),
            "`{src}` wants the whole text: {e:?}"
        );
    };
    let pkg = |body: &str| format!("package p;\n{body}\nendpackage\nmodule top; endmodule\n");
    let h = "typedef struct packed { logic pre; e_t [1:0] m; } h_t;";
    let ve = "typedef logic [1:0] v_t; typedef struct packed { v_t x; logic y; } e_t;";
    for src in [
        // `$unit` / a wildcard-imported package also names the page vector (t01,
        // e_struct2_h, e_union2_h, r2c pw1).
        format!(
            "typedef struct packed {{ logic a; }} v_t;\n{}",
            pkg(&format!("{ve}\n{h}"))
        ),
        format!(
            "typedef union packed {{ bit [1:0] q; bit [1:0] r; }} v_t;\n{}",
            pkg(&format!("{ve}\n{h}"))
        ),
        format!(
            "package q; typedef struct packed {{ logic a; }} v_t; endpackage\n{}",
            pkg(&format!("import q::*;\n{ve}\n{h}"))
        ),
        // A typedef alias of the element (t03).
        pkg(&format!(
            "typedef struct packed {{ logic [1:0] x; logic y; }} e0_t; typedef e0_t e_t;\n{h}"
        )),
        // A leaf `logic [N:0]` with N < 0 (t04).
        pkg(&format!(
            "typedef struct packed {{ logic [-1:0] x; logic y; }} e_t;\n{h}"
        )),
        // A vector element (t4_vec_member).
        pkg("typedef logic [7:0] w_t;\n\
             typedef struct packed { logic [1:0] pre; w_t [3:0] m; logic post; } h_t;"),
    ] {
        whole(&src, MEMBER);
    }
    let e = "typedef struct packed { logic [1:0] x; logic y; } e_t;";
    let v = "typedef logic [1:0] v_t;";
    for src in [
        // An alias of a vector (t1_vec_alias, a01).
        pkg("typedef logic [7:0] w_t; typedef w_t [3:0] a_t;"),
        // A struct return and a struct formal (v12, v13).
        pkg(&format!(
            "{e}\nfunction automatic e_t [1:0] f(input logic [5:0] x); return x; endfunction"
        )),
        pkg(&format!(
            "{e}\nfunction automatic logic [5:0] f(input e_t [1:0] x); return x; endfunction"
        )),
        // An outer bound of a name, an implicit and an `output` direction (v14 v15 v16).
        pkg(&format!(
            "{v} localparam int N = 2;\n\
             function automatic v_t [N-1:0] f(input logic [3:0] x); return x; endfunction"
        )),
        pkg(&format!(
            "{v}\nfunction automatic logic [3:0] f(v_t [1:0] x); return x; endfunction"
        )),
        pkg(&format!(
            "{v}\nfunction automatic int f(output v_t [1:0] x); x = 0; return 0; endfunction"
        )),
        // A base `$unit` or a wildcard-imported package also names (r2c pw8 pw12).
        format!(
            "typedef struct packed {{ logic a; }} v_t;\n{}",
            pkg(&format!(
                "{v}\nfunction automatic v_t [1:0] f(input logic [3:0] x); return x; endfunction"
            ))
        ),
        format!(
            "package q; typedef union packed {{ logic [1:0] a; logic [1:0] b; }} v_t; endpackage\n{}",
            pkg(&format!(
                "import q::*; {v}\n\
                 function automatic v_t [1:0] f(input logic [3:0] x); return x; endfunction"
            ))
        ),
        // An alias port with a dimension or a default after its name (a02 a03).
        format!(
            "{}module m(input p::ev_t x [2]); endmodule\n",
            pkg(&format!("{e} typedef e_t [1:0] ev_t;"))
        ),
        format!(
            "{}module m(input p::ev_t x = '0); endmodule\n",
            pkg(&format!("{e} typedef e_t [1:0] ev_t;"))
        ),
    ] {
        whole(&src, FORM);
    }
}

/// A member after a holder is read as it is in the keyword twin: the holder's element
/// marks no later member of its struct, nor of the next struct.
#[test]
fn a_member_after_a_holder_is_the_twins() {
    let ast = |m: &str| {
        let src = format!(
            "package q;\n\
             typedef struct packed {{ logic [1:0] q; logic e; }} e_t;\n\
             typedef struct packed {{ {m}; logic [3:0] z; }} r_t;\n\
             typedef struct packed {{ logic [3:0] y; }} t_t;\n\
             endpackage\n\
             module top; import q::*; r_t s; t_t t; logic x; logic [1:0] d;\n\
             assign x = s.z[1]; assign d = s.z[2:1]; assign x = t.y[2];\n\
             endmodule\n"
        );
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "`{m}`: {errs:?}");
        let module = no_spans(&format!("{:?}", su.unwrap().items.last().unwrap()));
        // The three assigns; `s` itself differs by design (a pattern onto a holder's
        // struct stays loud, so its declaration records no pattern members).
        let at = module.find("ContAssign(").expect("an assign");
        module[at..].to_string()
    };
    assert_eq!(ast("e_t [1:0] m"), ast("logic [1:0][2:0] m"));
}

#[test]
fn a_struct_holding_one_nests_nowhere() {
    let h = "typedef struct packed { logic a; } e_t;\n\
             typedef struct packed { e_t [3:0] m; } h_t;\n";
    decl_refused(&format!("{h}typedef struct packed {{ h_t h; }} r_t;"), N2);
    decl_refused(
        &format!("{h}typedef struct packed {{ h_t [1:0] h; }} r_t;"),
        N2,
    );
    decl_refused(
        &format!("{h}typedef union packed {{ h_t a; logic [3:0] b; }} u_t;"),
        N2,
    );
    decl_refused(&format!("{h}typedef struct {{ h_t h; }} r_t;"), N2);
}

#[test]
fn a_struct_with_a_symbolic_member_holds_none() {
    decl_refused(
        "parameter int unknown = zz; typedef struct packed { logic a; } e_t;\n\
         typedef struct packed { e_t [3:0] m; logic [unknown:0] n; } r_t;",
        N1,
    );
}

#[test]
fn two_declarators_share_the_dimension() {
    accepted("assign x = 1'b0;");
    let src = "package q; typedef struct packed { logic a; } e_t;\n\
               typedef struct packed { e_t [1:0] m, n; } r_t; endpackage\n\
               module top; import q::*; r_t s; logic x, y;\n\
               assign x = s.m[1].a; assign y = s.n[1].a; endmodule\n";
    let e = errors(src);
    assert!(e.is_empty(), "{e:?}");
}

// ---------------------------------------------------------------- the select, loud

#[test]
fn a_literal_outside_the_dimension_or_not_a_plain_decimal() {
    for i in ["4", "2147483648", "9999999999999999999999"] {
        refused(&format!("assign x = s.m[{i}].e;"), N4);
        refused(&format!("assign s.m[{i}].e = x;"), N4);
    }
    for i in ["2'd1", "1'b1", "'d1", "4'hf", "-1", "2+3", "1*2"] {
        refused(&format!("assign x = s.m[{i}].e;"), N4);
    }
}

#[test]
fn a_select_past_the_element() {
    refused("assign d[1:0] = s.m[1:0];", N4);
    refused("assign d[1:0] = s.m[1+:2];", N4);
    refused("assign d[1:0] = s.m[3-:2];", N4);
    refused("assign x = s.m[1][0];", N4);
    refused("assign s.m[1][0] = x;", N4);
    refused("assign x = s.m[1].q[0];", N4);
    refused("assign s.m[1].q[0] = x;", N4);
    refused("assign x = s.m[1].e.z;", N4);
}

#[test]
fn an_unknown_chain_member() {
    refused("assign x = s.m[1].nope;", "a member of the nested struct");
    refused("assign x = t.v[1].n.nope;", "a member of the nested struct");
}

#[test]
fn an_index_past_the_grammar() {
    for e in [
        "i+i",
        "i*2",
        "2*i-1",
        "1+i",
        "3*i+i",
        "(i)",
        "i-1",
        "-i",
        "~i",
        "i%4",
        "i<<1",
        "i[1:0]",
        "2'd1*i",
        "1'b1+i",
        "3000000000-i",
        "i+3000000000",
        "3000000000*i",
        "f(i)",
        "i++",
    ] {
        refused(
            &format!("always_comb for (int i = 0; i < 4; i++) x = s.m[{e}].e;"),
            N4,
        );
    }
}

#[test]
fn a_name_no_loop_bound_is_no_index() {
    refused("always_comb x = s.m[k].e;", N4);
    refused("always_comb for (int i = 0; i < 4; i++) x = s.m[k].e;", N4);
    refused("for (genvar g = 0; g < 4; g++) assign x = s.m[k].e;", N4);
}

#[test]
fn only_a_plain_int_loop_is_an_index() {
    for t in [
        "byte",
        "shortint",
        "longint",
        "integer",
        "logic [3:0]",
        "logic",
        "bit [31:0]",
    ] {
        refused(
            &format!("always_comb for ({t} i = 0; i < 4; i++) x = s.m[i].e;"),
            N4,
        );
    }
    refused(
        "typedef int i_t; always_comb for (i_t i = 0; i < 4; i++) x = s.m[i].e;",
        N4,
    );
    refused("always_comb foreach (d[i]) x = s.m[i].e;", N4);
    refused(
        "logic [3:0][1:0] q; always_comb foreach (q[i, j]) x = s.m[j].e;",
        N4,
    );
}

#[test]
fn the_innermost_binding_decides() {
    refused(
        "always_comb for (int i = 0; i < 4; i++) for (byte i = 0; i < 4; i++) x = s.m[i].e;",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) begin byte i; x = s.m[i].e; end",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) begin automatic byte i = 0; x = s.m[i].e; end",
        N4,
    );
    refused(
        "typedef byte b_t; always_comb for (int i = 0; i < 4; i++) begin b_t i; x = s.m[i].e; end",
        N4,
    );
    refused(
        "for (genvar g = 0; g < 4; g++) begin localparam byte g = 1; assign x = s.m[g].e; end",
        N4,
    );
}

/// Element offsets are bounded by the net limit: past it the sentinel lies inside.
#[test]
fn a_struct_past_the_net_limit_has_no_element_select() {
    let e = errors(
        "package q; typedef struct packed { logic a; } e_t;\n\
         typedef struct packed { logic [2097151:0] b; e_t [3:0] m; } big_t; endpackage\n\
         module top; q::big_t s; logic x; assign x = s.m[1].a; endmodule\n",
    );
    assert!(e.iter().any(|m| m.contains(N4)), "{e:?}");
}

/// A module's bases are its own: a non-ANSI port takes no fact from the module before.
#[test]
fn a_base_is_a_fact_of_one_module() {
    let e = errors(&format!(
        "{PKG}module a; p::r_t s; endmodule\n\
         module top(s, o); input p::r_t s; output logic [3:0] o;\n\
         for (genvar g = 0; g < 4; g++) assign o[g] = s.m[g].e;\nendmodule\n"
    ));
    assert!(e.iter().any(|m| m.contains(N4)), "{e:?}");
}

/// A header of its own reads its own name in its condition and step: not the outer
/// `int` of the same name.
#[test]
fn a_nested_header_binds_from_its_condition() {
    refused(
        "always_comb for (int i = 0; i < 4; i++) \
         for (byte i = 0; s.m[i].q != 2'd3 && i < 4; i = i + 1) o = 1'b1;",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) \
         for (integer i = 0; i < 4; i = i + int'(s.m[i].e)) o = 1'b1;",
        N4,
    );
    refused(
        "always_comb for (int i = 0; s.m[i].e != 1'b1; i = i + 1) o = 1'b1;\n\
         always_comb for (byte i = 0; s.m[i].e != 1'b1; i = i + 1) o = 1'b1;",
        N4,
    );
}

#[test]
fn a_loop_name_ends_with_the_loop() {
    refused(
        "always_comb begin for (int i = 0; i < 4; i++) x = 0; x = s.m[i].e; end",
        N4,
    );
    refused(
        "for (genvar g = 0; g < 4; g++) assign d[g] = 1'b0;\n assign x = s.m[g].e;",
        N4,
    );
}

#[test]
fn outside_the_two_processes() {
    let f = "for (int i = 0; i < 4; i++) x = s.m[i].e;";
    refused(&format!("initial {f}"), N4);
    refused(&format!("always_ff @(posedge o) {f}"), N4);
    refused(&format!("always @* {f}"), N4);
    refused(&format!("always_latch {f}"), N4);
    refused(&format!("final {f}"), N4);
    refused(&format!("function automatic void z(); {f} endfunction"), N4);
    refused(&format!("task automatic z(); {f} endtask"), N4);
    refused(
        "for (genvar g = 0; g < 4; g++) always_comb x = s.m[g].e;",
        N4,
    );
    refused("for (genvar g = 0; g < 4; g++) wire w = s.m[g].e;", N4);
    refused("for (genvar g = 0; g < 4; g++) logic w = s.m[g].e;", N4);
}

#[test]
fn a_clause_that_hides_the_loop() {
    refused(
        "int a[3]; int r; always_comb for (int i = 0; i < 4; i++) \
         r = a.sum(i) with (int'(s.m[i].e));",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) assert (s.m[i].e) else $display(\"x\");",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) assume (s.m[i].e);",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) assert #0 (s.m[i].e);",
        N4,
    );
    refused(
        "always_comb for (int i = 0; i < 4; i++) assert final (s.m[i].e);",
        N4,
    );
}

#[test]
fn a_base_not_at_the_top_of_the_module() {
    refused(
        "always_comb begin r_t l; for (int i = 0; i < 4; i++) x = l.m[i].e; end",
        N4,
    );
    refused(
        "if (1) begin r_t l; always_comb for (int i = 0; i < 4; i++) x = l.m[i].e; end",
        N4,
    );
    refused(
        "for (genvar g = 0; g < 2; g++) begin r_t l; assign x = l.m[g].e; end",
        N4,
    );
    // A name a block declares over the variable is no struct there.
    let e = errors(&top(
        "always_comb begin logic s; for (int i = 0; i < 4; i++) x = s.m[i].e; end",
    ));
    assert!(!e.is_empty());
    // An array of the struct.
    refused("r_t a [2]; assign x = a.m[1].e;", N4);
    refused("r_t [1:0] a; assign x = a.m[1].e;", N4);
}

/// A block-local struct of the module variable's own name hides it: the name is no
/// base inside the block, read or written.
#[test]
fn a_block_local_shadow_of_the_variable_is_no_base() {
    refused(
        "always_comb begin r_t s; for (int i = 0; i < 4; i++) x = s.m[i].e; end",
        N4,
    );
    refused(
        "always_comb begin r_t s; for (int i = 0; i < 4; i++) s.m[i].e = x; end",
        N4,
    );
}

#[test]
fn a_port_is_read_by_a_genvar_only() {
    refused("always_comb for (int i = 0; i < 4; i++) x = pi.m[i].e;", N4);
    accepted("assign x = pi.m[1].e;");
}

// ---------------------------------------------------------------- the write site

#[test]
fn a_write_beyond_a_plain_blocking_assign() {
    let l = "always_comb for (int i = 0; i < 4; i++)";
    refused(&format!("{l} {{s.m[i].e, o}} = 2'b0;"), N4);
    refused(&format!("{l} {{o, s.m[i].e}} = 2'b0;"), N4);
    refused(&format!("{l} s.m[i].e <= x;"), N4);
    refused(&format!("{l} s.m[i].q |= d[1:0];"), N4);
    refused(&format!("{l} s.m[i].q += 2'd1;"), N4);
    refused(&format!("{l} s.m[i].e++;"), N4);
    refused(&format!("{l} ++s.m[i].e;"), N4);
    refused(&format!("{l} s.m[i].e = #1 x;"), N4);
    refused(&format!("{l} s.m[i].e = @(posedge o) x;"), N4);
    refused(&format!("{l} s.m[i].e = repeat (2) @(posedge o) x;"), N4);
    refused(&format!("{l} force s.m[i].e = x;"), N4);
    refused(&format!("{l} release s.m[i].e;"), N4);
    refused(&format!("{l} assign s.m[i].e = x;"), N4);
    refused(&format!("{l} deassign s.m[i].e;"), N4);
    refused(&format!("{l} for (s.m[i].e = 0; x; ) o = 1'b0;"), N4);
}

/// The site belongs to one statement: a plain `=` before a force, a procedural assign
/// or a for-init in the same body leaves none of them a site.
#[test]
fn a_site_is_taken_by_its_own_statement() {
    let l = "always_comb for (int i = 0; i < 4; i++) begin o = 1'b0;";
    refused(&format!("{l} force s.m[i].e = x; end"), N4);
    refused(&format!("{l} assign s.m[i].e = x; end"), N4);
    refused(&format!("{l} for (s.m[i].e = 0; x; ) o = 1'b0; end"), N4);
}

#[test]
fn a_continuous_write_with_a_delay_or_into_a_concat() {
    let l = "for (genvar g = 0; g < 4; g++)";
    refused(&format!("{l} assign #1 s.m[g].e = d[g];"), N4);
    refused(&format!("{l} assign {{s.m[g].e, o}} = 2'b0;"), N4);
}

#[test]
fn a_port_is_no_run_time_target() {
    let e = errors(&format!(
        "{PKG}module top(output p::r_t po);\n\
         for (genvar g = 0; g < 4; g++) assign po.m[g].e = 1'b0;\nendmodule\n"
    ));
    assert!(e.iter().any(|m| m.contains(N4)), "{e:?}");
}

/// A declaration in a one-item generate branch names no base, not even by shadowing.
#[test]
fn a_one_item_generate_branch_produces_no_base() {
    let e = errors(&format!(
        "{PKG}module top(output p::r_t po);\nif (1) p::r_t po;\n\
         for (genvar g = 0; g < 4; g++) assign po.m[g].e = 1'b0;\nendmodule\n"
    ));
    assert!(e.iter().any(|m| m.contains(N4)), "{e:?}");
}

#[test]
fn a_whole_element_write_stays_on_the_twin_path() {
    let l = "always_comb for (int i = 0; i < 4; i++)";
    let e = errors(&top(&format!("{l} s.m[i] = d[2:0];")));
    assert!(!e.is_empty() && !e.iter().any(|m| m.contains(N4)), "{e:?}");
    // A literal one is refused by the same text as the keyword twin's.
    let lit = errors(&top("initial s.m[1] = d[2:0];"));
    assert_eq!(e[0], lit[0], "the keyword twin has one text for both");
    // An element of width 1: the twin's bit arm. A literal past the end has no effect.
    same("initial u.n[7] = 1'b1;", "initial u[7] = 1'b1;");
}

// ---------------------------------------------------------------- patterns

#[test]
fn a_pattern_onto_a_holder() {
    refused("r_t l = '{a: 1'b0, m: '0, z: 3'd0};", N3);
    refused("r_t l = '{default: 1'b0};", N3);
    refused("assign s = '{a: 1'b0, m: '0, z: 3'd0};", N3);
    refused("initial s = '{default: 1'b0};", N3);
    refused("initial s = r_t'{a: 1'b0, m: '0, z: 3'd0};", N3);
}

// ---------------------------------------------------------------- alias, D3 D4 D5

#[test]
fn an_alias_is_no_type_of_a_variable() {
    let v = errors(&format!("{PKG}module top; p::vec_t v; endmodule\n"));
    assert!(!v.is_empty());
    let v = errors(&format!(
        "{PKG}module top; import p::*; vec_t v; endmodule\n"
    ));
    assert!(!v.is_empty());
    let m = "package q; import p::*; typedef struct packed { vec_t m; } r_t; endpackage\n";
    assert!(!errors(&format!("{PKG}{m}module top; endmodule\n")).is_empty());
}

#[test]
fn an_alias_outside_a_package_says_what_it_said() {
    let t = "a simple chained typedef alias";
    let e = errors(
        "module top; typedef struct packed { logic a; } e_t; typedef e_t [1:0] a_t; endmodule\n",
    );
    assert!(e.iter().any(|m| m.contains(t)), "{e:?}");
    let e = errors(
        "typedef struct packed { logic a; } e_t; typedef e_t [1:0] a_t; module top; endmodule\n",
    );
    assert!(e.iter().any(|m| m.contains(t)), "{e:?}");
    let e = errors(
        "package q; typedef struct packed { logic a; } e_t;\n\
         function automatic int f(); typedef e_t [1:0] a_t; return 0; endfunction endpackage\n\
         module top; endmodule\n",
    );
    assert!(e.iter().any(|m| m.contains(t)), "{e:?}");
}

#[test]
fn an_alias_of_another_form_is_named() {
    let e = "typedef struct packed { logic a; } e_t;\n";
    decl_refused(&format!("{e}typedef e_t [4:1] a_t;"), N5);
    decl_refused(&format!("{e}typedef e_t [0:3] a_t;"), N5);
    decl_refused(&format!("{e}typedef e_t [1:0][1:0] a_t;"), N5);
    decl_refused(&format!("{e}typedef e_t [1:0] a_t [2];"), N5);
    decl_refused("typedef logic [3:0] v_t; typedef v_t [1:0] a_t;", N5);
}

#[test]
fn the_bare_alias_key_leaves_with_the_package() {
    let e = errors(&format!("{PKG}module top(input vec_t a); endmodule\n"));
    assert!(!e.is_empty());
    let e = errors(&format!(
        "{PKG}module top(input vec_t a); import p::*; endmodule\n"
    ));
    assert!(!e.is_empty());
    let e = errors(&format!(
        "{PKG}package q; parameter vec_t E = '0; endpackage\nmodule top; endmodule\n"
    ));
    assert!(!e.is_empty());
}

#[test]
fn an_alias_port_is_its_twin() {
    let twin = |ty: &str| {
        let src = format!(
            "{PKG}module top(input {ty} a, b, output logic o); assign o = a[1].e ^ b[0].q[1]; endmodule\n"
        );
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "{errs:?}");
        no_spans(&format!("{:?}", su.unwrap().items.last().unwrap()))
    };
    let (natural, spelled) = (twin("p::vec_t"), twin("p::e_t [3:0]"));
    // That flag is only read for an unpacked port, which an alias port refuses.
    let flag = |s: String| s.replace("integral_typedef: true", "integral_typedef: false");
    assert_eq!(flag(natural), flag(spelled));
}

#[test]
fn an_alias_port_has_no_tail() {
    let e = |h: &str| errors(&format!("{PKG}module top({h}); endmodule\n"));
    assert!(e("input p::vec_t a = '0").iter().any(|m| m.contains(N5)));
    assert!(e("input p::vec_t a, b = '0").iter().any(|m| m.contains(N5)));
    assert!(e("input p::vec_t a [2]").iter().any(|m| m.contains(N5)));
    assert!(e("input p::vec_t a, b [2]").iter().any(|m| m.contains(N5)));
}

#[test]
fn a_non_ansi_port_takes_no_alias() {
    let e = errors(&format!(
        "{PKG}module top(a); input p::vec_t a; endmodule\n"
    ));
    // The name is not read as a type here at all: the port stops at the scope, as a
    // name of no type does, and never reaches the twin's packed-dimension text.
    let none = errors(&format!(
        "{PKG}module top(a); input p::no_such_t a; endmodule\n"
    ));
    assert!(!e.is_empty());
    assert_eq!(e.first(), none.first(), "{e:?}");
    assert!(
        !e.iter()
            .any(|m| m.contains("packed dimensions after a typedef name")),
        "{e:?}"
    );
}

#[test]
fn an_alias_parameter_is_its_twin() {
    let twin = |ty: &str| {
        let src = format!(
            "package q; typedef struct packed {{ logic [1:0] a; }} e_t;\n\
             typedef e_t [3:0] v_t; parameter {ty} D = '0; endpackage\nmodule top; endmodule\n"
        );
        let (su, errs) = p(&src);
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let TopItem::Package(k) = &su.items[0] else {
            panic!()
        };
        no_spans(&format!("{:?}", k.body.last().unwrap()))
    };
    assert_eq!(twin("v_t"), twin("logic [3:0][1:0]"));
}

// ---------------------------------------------------------------- return, formal

fn sha(decl: &str) -> String {
    format!("package q; typedef logic [3:0] w_t;\n{decl}\nendpackage\nmodule top; endmodule\n")
}

#[test]
fn a_return_and_a_formal_are_their_twins() {
    let f = |ret: &str, arg: &str| {
        let (su, errs) = p(&sha(&format!(
            "function automatic {ret} f(input {arg} h);\n return h; endfunction"
        )));
        assert!(errs.is_empty(), "{errs:?}");
        let su = su.unwrap();
        let TopItem::Package(k) = &su.items[0] else {
            panic!()
        };
        no_spans(&format!("{:?}", k.body.last().unwrap()))
    };
    assert_eq!(
        f("w_t [1:0]", "w_t [2:0]"),
        f("logic [1:0][3:0]", "logic [2:0][3:0]")
    );
}

#[test]
fn a_return_outside_a_package_says_what_it_said() {
    let e = errors(
        "module top; typedef logic [3:0] w_t;\n\
         function automatic w_t [1:0] f(); return '0; endfunction endmodule\n",
    );
    assert!(!e.is_empty() && !e.iter().any(|m| m.contains(N5)), "{e:?}");
}

#[test]
fn a_formal_outside_a_package_says_what_it_said() {
    for src in [
        "module top; typedef logic [3:0] w_t;\n\
         function automatic int f(input w_t [1:0] h); return 0; endfunction endmodule\n",
        "typedef logic [3:0] w_t;\n\
         function automatic int f(input w_t [1:0] h); return 0; endfunction\n\
         module top; endmodule\n",
    ] {
        let e = errors(src);
        assert!(e.iter().any(|m| m.contains(NOT_ID)), "{e:?}");
    }
}

#[test]
fn a_return_or_formal_of_another_form_is_named() {
    for base in [
        "typedef logic signed [3:0] t_t;",
        "typedef bit [3:0] t_t;",
        "typedef reg [3:0] t_t;",
        "typedef enum logic [1:0] { A } t_t;",
        "typedef struct packed { logic a; } t_t;",
        "typedef logic [1:0][3:0] t_t;",
        "parameter int W = 4; typedef logic [W-1:0] t_t;",
        "typedef int t_t;",
    ] {
        let e = errors(&sha(&format!(
            "{base} function automatic t_t [1:0] f(); return '0; endfunction"
        )));
        assert!(e.iter().any(|m| m.contains(N5)), "{base}: {e:?}");
        let e = errors(&sha(&format!(
            "{base} function automatic int f(input t_t [1:0] h); return 0; endfunction"
        )));
        assert!(e.iter().any(|m| m.contains(N5)), "{base}: {e:?}");
    }
    for ret in ["w_t [1:0][1:0]", "w_t [P:0]"] {
        let e = errors(&sha(&format!(
            "parameter int P = 1; function automatic {ret} f(); return '0; endfunction"
        )));
        assert!(e.iter().any(|m| m.contains(N5)), "{ret}: {e:?}");
    }
    for dir in [
        "output w_t [1:0]",
        "inout w_t [1:0]",
        "w_t [1:0]",
        "ref w_t [1:0]",
    ] {
        let e = errors(&sha(&format!(
            "function automatic int f({dir} h); return 0; endfunction"
        )));
        assert!(e.iter().any(|m| m.contains(N5)), "{dir}: {e:?}");
    }
}

/// A return or `input` formal base beyond the page vector (§5.2 row 41 PLAN2 V1'): the
/// bare, unescaped name of a `typedef logic [D:0]` of this package only.
#[test]
fn a_base_of_another_form_is_named() {
    for (pre, base, name) in [
        ("", "typedef logic [1:0][1:0] t_t;", "t_t"),
        ("", "typedef logic [2:1] t_t;", "t_t"),
        ("", "typedef logic [1:0] t_t [2];", "t_t"),
        ("", "typedef bit [7:0] t_t;", "t_t"),
        ("", "typedef logic unsigned [3:0] t_t;", "t_t"),
        ("", "typedef logic [3:0] \\t$t ;", "\\t$t "),
        (
            "package x; typedef logic [3:0] t_t; endpackage\n",
            "import x::*;",
            "t_t",
        ),
        (
            "package x; typedef logic [3:0] t_t; endpackage\n",
            "",
            "x::t_t",
        ),
    ] {
        for decl in [
            format!("{base} function automatic {name} [1:0] f(); return '0; endfunction"),
            format!("{base} function automatic int f(input {name} [1:0] h); return 0; endfunction"),
        ] {
            let e = errors(&format!("{pre}{}", sha(&decl)));
            assert!(e.iter().any(|m| m.contains(N5)), "{decl}: {e:?}");
        }
    }
}

#[test]
fn a_non_ansi_formal_stays_as_it_was() {
    let e = errors(&sha(
        "function automatic int f; input w_t [1:0] h; f = 0; endfunction",
    ));
    assert!(e.iter().any(|m| m.contains(NOT_ID)), "{e:?}");
}

// ---------------------------------------------------------------- loud, inherited

#[test]
fn a_holder_as_an_array_element_stays_outside() {
    assert!(!errors(&top("r_t a [2]; assign x = a[1].m[0].e;")).is_empty());
    assert!(!errors(&top("r_t [1:0] a; assign x = a[1].m[0].e;")).is_empty());
}

/// A whole element of the member, through an element of a struct array: the array
/// path has no element stride, so the member is unknown there, literal or run-time.
#[test]
fn an_element_of_a_struct_array_takes_no_member_select() {
    const UNKNOWN: &str = "unknown field in a struct-array element member access";
    refused("r_t a [2]; initial d[2:0] = a[1].m[2];", UNKNOWN);
    refused("r_t a [2]; initial d[2:0] = a[1].m[k];", UNKNOWN);
}

#[test]
fn a_keyword_member_is_untouched() {
    let src = "package q; typedef struct packed { logic [3:0][1:0] m; } r_t; endpackage\n\
               module top; import q::*; r_t s; logic [1:0] x; logic [3:0] i;\n\
               assign x = s.m[i]; endmodule\n";
    let e = errors(src);
    assert!(e.is_empty(), "{e:?}");
}
