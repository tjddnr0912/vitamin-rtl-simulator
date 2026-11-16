//! `ConstEntry.lane` (§5.2 PARSE-1, the constant table's entry) on what W1
//! (`param_item_to_module_item`) records: one row per insert path, each read by pattern
//! (`EntryLane` has no `PartialEq`), each with the three other variants as its wrong-variant
//! control ; the lane rides the import and the package export whole ; a wildcard-import
//! merge keeps the first package's entry and its lane. No oracle: the lane is not a value
//! of the language, it is which W1 path decided the number (the cli pins
//! `const_entry_pins.rs` hold the outputs on both oracles' lines).
use super::*;

type Table = std::collections::HashMap<String, ConstEntry>;

/// The parser's two tables after a whole parse: `const_locals` (unit scope) and
/// `pkg_const_scoped` (every package's export).
fn tables(src: &str) -> (Table, Table) {
    let (toks, lex_errs) = hdl_lexer::lex(src);
    assert!(lex_errs.is_empty(), "lex errors: {lex_errs:?}");
    let mut p = Parser::new(&toks, src);
    let _ = p.parse_source_unit();
    assert!(p.errors.is_empty(), "`{src}`: {:?}", p.errors);
    (p.const_locals, p.pkg_const_scoped)
}

fn is_no_type_no_range(l: EntryLane) -> bool {
    matches!(l, EntryLane::NoTypeNoRange)
}
fn is_converted(l: EntryLane) -> bool {
    matches!(l, EntryLane::Converted)
}
fn is_fallback(l: EntryLane) -> bool {
    matches!(l, EntryLane::Fallback)
}
fn is_width_unknown(l: EntryLane) -> bool {
    matches!(l, EntryLane::WidthUnknown)
}

type Is = fn(EntryLane) -> bool;

const LANES: [(&str, Is); 4] = [
    ("NoTypeNoRange", is_no_type_no_range),
    ("Converted", is_converted),
    ("Fallback", is_fallback),
    ("WidthUnknown", is_width_unknown),
];

/// `want`'s predicate holds on `e.lane` and each of the other three does not (the
/// wrong-variant control), and the value is `(v, w, signed)`.
#[track_caller]
fn lane(e: Option<&ConstEntry>, want: &str, v: i64, w: u32, signed: bool) {
    let e = e.expect("an entry");
    for (name, is) in LANES {
        assert_eq!(is(e.lane), name == want, "{e:?}: {name}");
    }
    assert_eq!(e.val(), ConstVal { v, w, signed }, "{e:?}");
}

/// The entry `pkg_const_scoped["p::<name>"]` after `package p; <decls> endpackage`.
fn pkg(decls: &str, name: &str) -> ConstEntry {
    let src = format!("package p;\n{decls}\nendpackage\n");
    let (_, s) = tables(&src);
    *s.get(&format!("p::{name}"))
        .unwrap_or_else(|| panic!("`{src}`: no p::{name} in {s:?}"))
}

#[test]
fn the_helpers_tell_the_lanes_apart() {
    let all = [
        EntryLane::NoTypeNoRange,
        EntryLane::Converted,
        EntryLane::Fallback,
        EntryLane::WidthUnknown,
    ];
    for (i, l) in all.into_iter().enumerate() {
        for (k, (_, is)) in LANES.iter().enumerate() {
            assert_eq!(is(l), i == k, "{l:?}");
        }
    }
}

/// W1's paths (`typed`, `w`, then E1 `ctx_v` / E2 the i64 fold / E3 the based literal),
/// one row each. Not reachable, by reading `const_fold.rs`: typed with `w > 0` and E3
/// (`ctx_v` folds every based literal `based_lit_param_value` admits at `w <= 64`, and
/// `const_param_fits` refuses `w > 64`) and typed with `w == 0` and E3
/// (`based_lit_param_value` declines a range that does not fold and a type it does not
/// know) ; the `none_` rows are the probes for both.
#[test]
fn each_insert_path_writes_its_lane() {
    // (Implicit, None), E2 (the i64 fold) and E3 (a based literal: the i64 fold declines it)
    lane(
        Some(&pkg("localparam X = 5;", "X")),
        "NoTypeNoRange",
        5,
        32,
        true,
    );
    lane(
        Some(&pkg("localparam X = 8'd5;", "X")),
        "NoTypeNoRange",
        5,
        8,
        false,
    );
    // typed, w > 0, E1 (`ctx_v`) ; a range with no keyword is typed
    lane(
        Some(&pkg("localparam int X = 5;", "X")),
        "Converted",
        5,
        32,
        true,
    );
    lane(
        Some(&pkg("localparam [7:0] X = 5;", "X")),
        "Converted",
        5,
        8,
        false,
    );
    // typed, w > 0, E2: the width-aware fold declines a width-unknown operand
    let fb = "localparam logic [f(3):0] A = 3;\nlocalparam int X = A;";
    lane(Some(&pkg(fb, "X")), "Fallback", 3, 32, true);
    // typed, w == 0, E2: the range did not fold ; the type is one `param_const_shape`
    // does not know
    lane(Some(&pkg(fb, "A")), "WidthUnknown", 3, 0, false);
    lane(
        Some(&pkg("localparam real R = 5;", "R")),
        "WidthUnknown",
        5,
        0,
        false,
    );
}

#[test]
fn none_the_e3_probes_record_nothing() {
    for decls in [
        "localparam logic [f(3):0] A = 8'd3;",
        "localparam logic [7:0] X = 8'hx;",
        "localparam logic [99:0] X = 8'd5;",
    ] {
        let src = format!("package p;\n{decls}\nendpackage\n");
        let (_, s) = tables(&src);
        assert!(s.is_empty(), "`{src}`: {s:?}");
    }
}

/// NoTypeNoRange = W1's `(ParamType::Implicit, None)`: no keyword, a signing keyword
/// only, the `string` keyword, a string typedef (premise judge 2 line 11).
#[test]
fn no_type_no_range_is_the_implicit_pair() {
    lane(
        Some(&pkg("localparam signed X = 5;", "X")),
        "NoTypeNoRange",
        5,
        32,
        true,
    );
    lane(
        Some(&pkg("localparam string S = 5;", "S")),
        "NoTypeNoRange",
        5,
        32,
        true,
    );
    let td = "typedef string str_t;\nlocalparam str_t S = 5;";
    lane(Some(&pkg(td, "S")), "NoTypeNoRange", 5, 32, true);
}

#[test]
fn lane_survives_import_and_export() {
    let p = "package p;\nlocalparam int X = 5;\nlocalparam Y = 6;\nendpackage\n";
    for imp in ["import p::*;", "import p::X;\nimport p::Y;"] {
        let (l, s) = tables(&format!("{p}{imp}\n"));
        lane(s.get("p::X"), "Converted", 5, 32, true);
        lane(s.get("p::Y"), "NoTypeNoRange", 6, 32, true);
        lane(l.get("X"), "Converted", 5, 32, true);
        lane(l.get("Y"), "NoTypeNoRange", 6, 32, true);
    }
}

/// D-J1 (a): two wildcard imports of one name with equal values (`val()`) keep the first
/// package's entry, so its lane ; the other order keeps the other lane ; unequal values
/// drop the name (the ambiguity compare).
#[test]
fn lane_is_the_kept_entry() {
    let pq = "package p;\nlocalparam int N = 4;\nendpackage\npackage q;\nlocalparam N = 4;\nendpackage\n";
    let (l, s) = tables(&format!("{pq}import p::*;\nimport q::*;\n"));
    lane(l.get("N"), "Converted", 4, 32, true);
    lane(s.get("q::N"), "NoTypeNoRange", 4, 32, true);
    let (l, _) = tables(&format!("{pq}import q::*;\nimport p::*;\n"));
    lane(l.get("N"), "NoTypeNoRange", 4, 32, true);
    let diff = "package p;\nlocalparam int N = 4;\nendpackage\npackage q;\nlocalparam int N = 5;\nendpackage\nimport p::*;\nimport q::*;\n";
    let (l, _) = tables(diff);
    assert!(!l.contains_key("N"), "{l:?}");
}
