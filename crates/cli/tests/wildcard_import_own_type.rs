//! A wildcard import never rebinds a type name the importing scope binds itself
//! (§2 Scoping, ROADMAP §5.2 row 1 — the prerequisite of §3.a ⑤ⓚ, corpus `ibex`).
//!
//! ## What was wrong
//!
//! Type names are resolved in the parser, through flat maps keyed by the bare name
//! (`typedefs`, `struct_layouts`, `enum_defs`, `union_type_names`,
//! `unpacked_struct_layouts`). `import p::*` copied each `p::X` key to bare `X`, map by
//! map: over an existing entry when `cu_type_overridable` said the name was a unit-scope
//! type the importing module had not redeclared as a non-type (§4.5.434: the module's
//! import is the nearer scope, IEEE §26.3), else only where that map had no entry. The
//! predicate never counted the importing scope's OWN type names, so
//!
//! ```text
//! typedef struct packed { logic [3:0] a; logic [3:0] b; } st;        // $unit
//! package p; typedef struct packed { logic [3:0] a; logic [7:0] b; } st; endpackage
//! module t;
//!   typedef struct packed { logic [5:0] a; logic [1:0] b; } st;
//!   import p::*;
//!   st s2;   // s2 = '{a: 6'h1, b: 2'h2};  $display("%h bits=%0d", s2, $bits(s2));
//! ```
//!
//! printed `102 bits=12` where both oracles print `06 bits=8`, and the same happened at
//! the unit scope itself (a unit `typedef st` then a unit `import p::*`), to an explicit
//! import (`import q::st; import p::*;`) in a module, a header or an interface, to a
//! package's own typedef (so its `q2::st` twin carried p's type out of the package), to
//! a program's and to a type parameter. Where no unit-scope `st` existed, the per-map
//! `or_insert` still let a package type of ANOTHER kind land beside the scope's own: a
//! local struct `st` beside p's union `st` was marked a union (its keyed pattern
//! refused), and an unpacked-struct twin on either side read the wrong layout. And the
//! copy offered every `p::X` key, including the layout the parser keeps for a type p
//! merely IMPORTED (its own variables' replay): with a unit `typedef st`, `import p::*`
//! wrote base's layout under the unit's `st`.
//!
//! ## The fix
//!
//! `scope_type_names` (hdl-parser) holds the bare type names the current scope binds
//! itself — every `typedef`, type parameter and explicit import — starting empty in every
//! container and generate block and handed back by `ScopeSnapshot`. `wildcard_type_bind`
//! decides once per name for every map: `Skip` for a name in that set, `Replace` for a
//! unit-scope type (§4.5.434, unchanged), else `IfAbsent`; the union flag follows the
//! decision. A wildcard offers only the types p exports: a `typedefs` or unpacked-struct
//! twin.
//!
//! ## Oracles
//!
//! Every value here was measured on iverilog 13.0 (`-g2012`, `vvp -n`), sv2v 0.0.13 →
//! iverilog 13.0, and verilator 5.050 (`--binary --timing`). All three agree on every
//! cell with these exceptions: iverilog rejects a keyed pattern (syntax error) and an
//! unpacked struct ("sorry"), sv2v rejects the enum `num()` method, a class and a
//! `program` block, so those cells rest on the remaining two (each test says which).
//! The census (457 cells, PRE 6e56cef against the shipped build) moved 105 cells wrong →
//! right and 4 loud → right, and none right → wrong, right → loud or loud → wrong.
//!
//! ## Not in this slice
//!
//! Three review rounds measured what widening the REPLACEMENT would take — a container's
//! wildcard over a unit-scope import, a generate block's repeated import over the
//! module's own name, the replacement clearing every map, a class as a binder — and each
//! step met a pre-existing root: variables and nested members keep their type by bare
//! NAME (`var_struct`, `var_enum`, a layout's nested keys), so rebinding a name in a
//! nearer scope retargets them; and a package's struct / enum twins are decided by
//! diffing against the pre-body tables, so a package typedef identical to an outer one
//! gets none. Those rows are ROADMAP §2 Scoping with REMAINING_WORK §D prerequisites; the
//! cells here that meet them are controls held at their PRE value. docs/PROBE_CATALOG.md
//! holds the unpacked cross-kind shadow class (a declaration of one kind does not clear
//! another kind's unpacked binding — `an_unpacked_cross_kind_twin_meets_the_typedef_arm`
//! pins the equality it leaves), two wildcards of one scope (an oracle split), the
//! value side of a repeated generate-block import, and a module value named like a unit
//! typedef, which `$bits` and a cast still read as the type
//! (`an_explicit_import_of_a_value_keeps_the_wildcard_type_out` pins that equality too).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wiot_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

/// The design's one line tagged `tag` (a clean run, exit 0).
fn line(src: &str, tag: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected a clean run\n{src}\n{out}");
    let want = format!("{tag} ");
    out.lines()
        .find(|l| l.starts_with(&want))
        .unwrap_or_else(|| panic!("no `{tag}` line\n{src}\n{out}"))
        .to_string()
}

fn is_loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{src}\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

/// The five kinds a type name can be declared as. Each cell declares `st` at up to
/// four places with four widths, so the line names which declaration won.
const KINDS: [&str; 5] = ["struct", "alias", "enum", "union", "ustruct"];

/// Which declaration of `st` a cell's `st s2;` must reach.
#[derive(Clone, Copy)]
enum Won {
    /// The importing scope's own, 8 bits.
    Local,
    /// The compilation unit's, 10 bits.
    Unit,
    /// `p::st`, 12 bits (the wildcard import's).
    P,
    /// `q::st`, 9 bits (the explicit import's).
    Q,
}

/// `st` of `kind`, `w` bits wide; enum labels are prefixed with `tag`.
fn ty(kind: &str, w: u32, tag: &str) -> String {
    let a = match w {
        8 => 6,
        10 => 5,
        12 => 4,
        9 => 7,
        _ => unreachable!(),
    };
    let b = w - a;
    match kind {
        "struct" => format!(
            "struct packed {{ logic [{}:0] a; logic [{}:0] b; }}",
            a - 1,
            b - 1
        ),
        "alias" => format!("logic [{}:0]", w - 1),
        "enum" => {
            let n = match w {
                8 => 2,
                10 => 4,
                12 => 3,
                _ => 5,
            };
            let labels: Vec<String> = (0..n).map(|i| format!("{tag}{i}")).collect();
            format!("enum logic [{}:0] {{ {} }}", w - 1, labels.join(", "))
        }
        "union" => format!(
            "union packed {{ logic [{0}:0] a; logic [{0}:0] b; }}",
            w - 1
        ),
        "ustruct" => format!("struct {{ logic [{}:0] a; logic [{}:0] b; }}", a - 1, b - 1),
        _ => unreachable!(),
    }
}

/// The readout of `v` (declared `st`): its width, plus a member width and a value or a
/// label count — enough to tell the four declarations apart for every kind.
fn show(kind: &str, tag: &str, v: &str) -> String {
    match kind {
        "struct" | "ustruct" => format!(
            "    {v}.a = '0; {v}.b = '1;\n    $display(\"{tag} %0d %0d %0d\", $bits({v}), $bits({v}.a), $bits({v}.b));\n"
        ),
        "union" => format!(
            "    {v} = '1;\n    $display(\"{tag} %0d %0d %h\", $bits({v}), $bits({v}.a), {v});\n"
        ),
        "enum" => format!("    $display(\"{tag} %0d %0d\", $bits({v}), {v}.num());\n"),
        _ => format!("    {v} = '1;\n    $display(\"{tag} %0d %h\", $bits({v}), {v});\n"),
    }
}

/// The oracles' line for `kind` when `won` is the declaration reached.
fn oracle(kind: &str, won: Won, tag: &str) -> String {
    let v = match (kind, won) {
        ("struct" | "ustruct", Won::Local) => "8 6 2",
        ("struct" | "ustruct", Won::Unit) => "10 5 5",
        ("struct" | "ustruct", Won::P) => "12 4 8",
        ("struct" | "ustruct", Won::Q) => "9 7 2",
        ("alias", Won::Local) => "8 ff",
        ("alias", Won::Unit) => "10 3ff",
        ("alias", Won::P) => "12 fff",
        ("alias", Won::Q) => "9 1ff",
        ("enum", Won::Local) => "8 2",
        ("enum", Won::Unit) => "10 4",
        ("enum", Won::P) => "12 3",
        ("enum", Won::Q) => "9 5",
        ("union", Won::Local) => "8 8 ff",
        ("union", Won::Unit) => "10 10 3ff",
        ("union", Won::P) => "12 12 fff",
        ("union", Won::Q) => "9 9 1ff",
        _ => unreachable!(),
    };
    format!("{tag} {v}")
}

fn pkgs(kind: &str) -> String {
    format!(
        "package p;\n  typedef {} st;\nendpackage\npackage q;\n  typedef {} st;\nendpackage\n",
        ty(kind, 12, "P"),
        ty(kind, 9, "Q")
    )
}

fn unit_td(kind: &str) -> String {
    format!("typedef {} st;\n", ty(kind, 10, "U"))
}

fn local_td(kind: &str) -> String {
    format!("  typedef {} st;\n", ty(kind, 8, "L"))
}

/// `module t<hdr>; <body> st s2; initial … endmodule`.
fn module(kind: &str, tag: &str, hdr: &str, body: &str) -> String {
    format!(
        "module t{hdr};\n{body}  st s2;\n  initial begin\n{}  end\nendmodule\n",
        show(kind, tag, "s2")
    )
}

fn check(kind: &str, tag: &str, src: &str, won: Won) {
    assert_eq!(line(src, tag), oracle(kind, won, tag), "{src}");
}

#[test]
fn a_module_typedef_is_not_rebound_by_its_wildcard_import() {
    // The row's cell: the module's own `st`, a unit `st` beside it (PRE: p's `12 4 8`,
    // `12 fff`, `12 3`, `12 12 fff`, `12 4 8`). Its twin with no unit `st` was already
    // right and stays right. enum: iverilog + verilator; ustruct: sv2v → iverilog +
    // verilator; the rest all three.
    for k in KINDS {
        let body = format!("{}  import p::*;\n", local_td(k));
        let with_unit = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "M_own_U", "", &body)
        );
        check(k, "M_own_U", &with_unit, Won::Local);
        let no_unit = format!("{}{}", pkgs(k), module(k, "M_own_noU", "", &body));
        check(k, "M_own_noU", &no_unit, Won::Local);
        // A chained alias of the scope's own aggregate is the scope's own too.
        if matches!(k, "struct" | "enum" | "union") {
            let ca = format!(
                "  typedef {} base_t;\n  typedef base_t st;\n  import p::*;\n",
                ty(k, 8, "L")
            );
            let src = format!(
                "{}{}{}",
                pkgs(k),
                unit_td(k),
                module(k, "CA_own_U", "", &ca)
            );
            check(k, "CA_own_U", &src, Won::Local);
        }
    }
}

#[test]
fn a_unit_typedef_is_not_rebound_by_a_unit_wildcard_import() {
    // PRE read p's `st` in every module after `typedef … st; import p::*;` at the unit
    // scope — also in a module declared after another module's own import.
    for k in KINDS {
        let src = format!(
            "{}{}import p::*;\n{}",
            pkgs(k),
            unit_td(k),
            module(k, "U_own", "", "")
        );
        check(k, "U_own", &src, Won::Unit);
        let comma = format!(
            "{}{}import p::*, q::*;\n{}",
            pkgs(k),
            unit_td(k),
            module(k, "U_typedef_comma", "", "")
        );
        check(k, "U_typedef_comma", &comma, Won::Unit);
        let two = format!(
            "{}{}import p::*;\nmodule a;\n  import q::*;\nendmodule\n{}",
            pkgs(k),
            unit_td(k),
            module(k, "Two_modules_Uimp", "", "")
        );
        check(k, "Two_modules_Uimp", &two, Won::Unit);
        // The unit's wildcard written BEFORE its typedef was already right.
        let first = format!(
            "{}import p::*;\n{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "U_imp_first", "", "")
        );
        check(k, "U_imp_first", &first, Won::Unit);
    }
}

#[test]
fn an_explicit_import_is_not_rebound_by_a_wildcard_import() {
    // IEEE §26.3: an explicitly imported name is bound in the scope. PRE read p's `st`
    // after `import q::st; import p::*;` whenever the unit declared an `st` — in a
    // module body, a header, a comma list and an interface body.
    for k in KINDS {
        let body = "  import q::st;\n  import p::*;\n";
        let m = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "M_explicit_U", "", body)
        );
        check(k, "M_explicit_U", &m, Won::Q);
        let h = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "H_explicit_U", " import q::st; import p::*;", "")
        );
        check(k, "H_explicit_U", &h, Won::Q);
        let c = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "M_comma_U", "", "  import q::st, p::*;\n")
        );
        check(k, "M_comma_U", &c, Won::Q);
        let i = format!(
            "{}{}interface i;\n  import q::st;\n  import p::*;\n  st s2;\n  initial begin\n{}  end\nendinterface\nmodule t;\n  i u();\nendmodule\n",
            pkgs(k),
            unit_td(k),
            show(k, "If_explicit_U", "s2")
        );
        check(k, "If_explicit_U", &i, Won::Q);
        // With no unit `st` the explicit import already won; at the unit scope too.
        let no_unit = format!("{}{}", pkgs(k), module(k, "M_explicit_noU", "", body));
        check(k, "M_explicit_noU", &no_unit, Won::Q);
        let unit = format!(
            "{}import q::st;\nimport p::*;\n{}",
            pkgs(k),
            module(k, "U_explicit", "", "")
        );
        check(k, "U_explicit", &unit, Won::Q);
    }
}

#[test]
fn a_package_and_a_program_keep_their_own_type() {
    // A package's own `st` beside `import p::*` (and a unit `st`) is what its `q2::st`
    // twin carries out: PRE handed every user of `q2::st` p's type. A program body is a
    // container like a module (sv2v rejects `program`; iverilog + verilator agree).
    for k in KINDS {
        let src = format!(
            "{}{}package q2;\n{}  import p::*;\nendpackage\nmodule t;\n  q2::st s2;\n  initial begin\n{}  end\nendmodule\n",
            pkgs(k),
            unit_td(k),
            local_td(k),
            show(k, "Pkg_twin_U", "s2")
        );
        check(k, "Pkg_twin_U", &src, Won::Local);
        let prg = format!(
            "{}{}program t;\n{}  import p::*;\n  st s2;\n  initial begin\n{}  end\nendprogram\n",
            pkgs(k),
            unit_td(k),
            local_td(k),
            show(k, "Prg_own_U", "s2")
        );
        check(k, "Prg_own_U", &prg, Won::Local);
    }
    // Inside the package: its constant and a function over its own `st` (the unpacked
    // twin's `$bits(st)` constant is refused for an unrelated reason, so it is left out).
    for k in ["struct", "alias", "enum", "union"] {
        let memb = matches!(k, "struct" | "union");
        let func = if memb {
            "  function automatic int fa();\n    st x;\n    return $bits(x.a);\n  endfunction\n"
        } else {
            ""
        };
        let (fmt, args, want) = if memb {
            (
                "%0d %0d",
                "q2::BW, q2::fa()",
                if k == "struct" { "8 6" } else { "8 8" },
            )
        } else {
            ("%0d", "q2::BW", "8")
        };
        let src = format!(
            "{}{}package q2;\n{}  import p::*;\n  localparam int BW = $bits(st);\n{func}endpackage\nmodule t;\n  initial $display(\"Pkg_own_U {fmt}\", {args});\nendmodule\n",
            pkgs(k),
            unit_td(k),
            local_td(k)
        );
        assert_eq!(
            line(&src, "Pkg_own_U"),
            format!("Pkg_own_U {want}"),
            "{src}"
        );
    }
}

#[test]
fn a_type_of_another_kind_gains_nothing_from_the_import() {
    // The scope's own `st` of one kind and p's `st` of another: no map of any kind
    // learns p's. PRE, with a unit `st` of p's kind, read p's type in every pair; with
    // no unit `st` it was right unless an unpacked struct sat on either side (p's
    // unpacked record answered a local alias / struct / union declaration, p's struct
    // or union layout answered a local unpacked struct's members, and a local enum's
    // `num()` was refused). Oracles: sv2v → iverilog + verilator on every unpacked
    // pair, verilator alone on the enum-over-unpacked pair (sv2v rejects `num()`), all
    // three elsewhere.
    for lk in KINDS {
        for pk in KINDS {
            if lk == pk {
                continue;
            }
            let p = format!(
                "package p;\n  typedef {} st;\nendpackage\n",
                ty(pk, 12, "P")
            );
            let body = format!("{}  import p::*;\n", local_td(lk));
            let tag = format!("X_{lk}_over_{pk}_noU");
            check(
                lk,
                &tag,
                &format!("{p}{}", module(lk, &tag, "", &body)),
                Won::Local,
            );
            // With a unit `st` of p's kind — the five pairs with an unpacked struct on
            // one side meet the typedef arm (see the last test).
            if lk != "ustruct" && pk != "ustruct" {
                let u = format!("typedef {} st;\n", ty(pk, 10, "U"));
                let tag = format!("X_{lk}_over_{pk}_U");
                check(
                    lk,
                    &tag,
                    &format!("{p}{u}{}", module(lk, &tag, "", &body)),
                    Won::Local,
                );
            }
            // The unit scope's own `st` of one kind, the unit's wildcard of another.
            let u = format!("typedef {} st;\nimport p::*;\n", ty(lk, 10, "U"));
            let tag = format!("XU_{lk}_over_{pk}");
            check(
                lk,
                &tag,
                &format!("{p}{u}{}", module(lk, &tag, "", "")),
                Won::Unit,
            );
        }
    }
}

#[test]
fn a_type_parameter_keeps_its_type() {
    // A type parameter is a type name of its scope. PRE: the unit's own `parameter type
    // st` read p's unpacked record after a unit `import p::*` (`12 fff`), as did a
    // module header's with no unit `st` (sv2v → iverilog + verilator: `10 3ff`, `8 ff`).
    // The other kinds were already right (all three oracles).
    for pk in KINDS {
        let p = format!(
            "package p;\n  typedef {} st;\nendpackage\n",
            ty(pk, 12, "P")
        );
        let u = format!(
            "{p}parameter type st = logic [9:0];\nimport p::*;\n{}",
            module("alias", "UTP", "", "")
        );
        assert_eq!(line(&u, "UTP"), "UTP 10 3ff", "{u}");
        let m = format!(
            "{p}module t #(parameter type st = logic [7:0]);\n  import p::*;\n  st s2;\n  initial begin\n{}  end\nendmodule\n",
            show("alias", "MTP", "s2")
        );
        assert_eq!(line(&m, "MTP"), "MTP 8 ff", "{m}");
    }
}

#[test]
fn the_keyed_pattern_twins_read_the_scopes_own_struct() {
    // The report's own readout: a keyed whole pattern into `st s2`. PRE `102 bits=12`
    // (p's layout) where sv2v → iverilog and verilator print `06 bits=8` (iverilog
    // alone rejects a keyed pattern). The union cells: a local struct beside p's UNION
    // `st` was marked a union, and PRE refused its pattern (E3009).
    let p =
        "package p;\n  typedef struct packed { logic [3:0] a; logic [7:0] b; } st;\nendpackage\n";
    let local = "  typedef struct packed { logic [5:0] a; logic [1:0] b; } st;\n";
    let unit10 = "typedef struct packed { logic [4:0] a; logic [4:0] b; } st;\n";
    let pat = |tag: &str| {
        format!(
            "  st s2;\n  initial begin\n    s2 = '{{a: 6'h1, b: 2'h2}};\n    $display(\"{tag} %h bits=%0d\", s2, $bits(s2));\n  end\n"
        )
    };
    let cells = [
        (
            "KP_M_own_U",
            format!("{p}{unit10}module t;\n{local}  import p::*;\n{}endmodule\n", pat("KP_M_own_U")),
        ),
        (
            "KP_U_own",
            format!(
                "{p}typedef struct packed {{ logic [5:0] a; logic [1:0] b; }} st;\nimport p::*;\nmodule t;\n{}endmodule\n",
                pat("KP_U_own")
            ),
        ),
        (
            "KP_M_explicit_U",
            // q's `st` has the local layout, so the explicit import's line is `06`.
            format!(
                "{p}package q;\n{local}endpackage\n{unit10}module t;\n  import q::st;\n  import p::*;\n{}endmodule\n",
                pat("KP_M_explicit_U")
            ),
        ),
        (
            "KP_Pkg_twin_U",
            format!(
                "{p}{unit10}package q2;\n{local}  import p::*;\nendpackage\nmodule t;\n  q2::st s2;\n  initial begin\n    s2 = '{{a: 6'h1, b: 2'h2}};\n    $display(\"KP_Pkg_twin_U %h bits=%0d\", s2, $bits(s2));\n  end\nendmodule\n"
            ),
        ),
    ];
    for (tag, src) in &cells {
        assert_eq!(line(src, tag), format!("{tag} 06 bits=8"), "{src}");
    }
    let pu =
        "package p;\n  typedef union packed { logic [11:0] a; logic [11:0] b; } st;\nendpackage\n";
    for (tag, unit) in [
        ("KP_X_struct_over_union_noU", ""),
        (
            "KP_X_struct_over_union_U",
            "typedef union packed { logic [9:0] a; logic [9:0] b; } st;\n",
        ),
    ] {
        let src = format!(
            "{pu}{unit}module t;\n{local}  import p::*;\n{}endmodule\n",
            pat(tag)
        );
        assert_eq!(line(&src, tag), format!("{tag} 06 bits=8"), "{src}");
    }
}

#[test]
fn a_unit_import_answers_where_the_module_binds_nothing_nearer() {
    // Controls, right before and after (all three oracles, sv2v / iverilog per kind as
    // above): a unit wildcard's `st` answers a module that imports nothing of its own
    // or the same package again, and a module's own `st` beside its `import p::*`
    // keeps the module's. (A container's wildcard over a unit IMPORT is ROADMAP §2
    // Scoping — it waits on the name-keyed variable bindings, REMAINING_WORK §D.)
    for k in KINDS {
        let only = format!(
            "{}import q::*;\n{}",
            pkgs(k),
            module(k, "U_wild_only", "", "")
        );
        check(k, "U_wild_only", &only, Won::Q);
        let same = format!(
            "{}import q::st;\n{}",
            pkgs(k),
            module(k, "U_explicit_M_wild_same", "", "  import q::*;\n")
        );
        check(k, "U_explicit_M_wild_same", &same, Won::Q);
        let own = format!(
            "{}import q::*;\n{}",
            pkgs(k),
            module(
                k,
                "U_wild_M_own",
                "",
                &format!("{}  import p::*;\n", local_td(k))
            )
        );
        check(k, "U_wild_M_own", &own, Won::Local);
    }
}

#[test]
fn the_unit_rebinding_and_the_declaration_order_controls_are_unchanged() {
    // §4.5.434 itself (a unit typedef, a module or header `import p::*`), a module
    // typedef written after its wildcard, and a header wildcard under a body typedef —
    // right before and after (all three oracles, sv2v / iverilog per kind as above).
    for k in KINDS {
        let mu = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "MU_import", "", "  import p::*;\n")
        );
        check(k, "MU_import", &mu, Won::P);
        let hu = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "H_import_U", " import p::*;", "")
        );
        check(k, "H_import_U", &hu, Won::P);
        let first = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(
                k,
                "M_imp_first_U",
                "",
                &format!("  import p::*;\n{}", local_td(k))
            )
        );
        check(k, "M_imp_first_U", &first, Won::Local);
        let ho = format!(
            "{}{}{}",
            pkgs(k),
            unit_td(k),
            module(k, "H_own_U", " import p::*;", &local_td(k))
        );
        check(k, "H_own_U", &ho, Won::Local);
        // Two unit-scope wildcards offering `st` make it ambiguous (IEEE §26.3):
        // iverilog and sv2v refuse, verilator reads the first package's. An oracle
        // split, not chased; held at vita's answer (verilator's): a unit-scope wildcard
        // never replaces a binding.
        let two = format!(
            "{}import q::*;\nimport p::*;\n{}",
            pkgs(k),
            module(k, "U_two_wild", "", "")
        );
        check(k, "U_two_wild", &two, Won::Q);
    }
}

#[test]
fn a_generate_block_keeps_its_own_type_under_a_repeated_import() {
    // Elaborate ignores a generate block's `import p::*` when the module has the same
    // one (`generate.rs`, redundant), so the parser's binding inside the block is what
    // runs. PRE, whenever the unit declared an `st`: the block's own `st` beside the
    // repeated import read p's (R1), and so did a block that imports nothing under the
    // module's own `st` (R9); all three oracles read the block's / the module's. Each
    // block's own type names start empty, so the unit-twin replacement of an ENCLOSING
    // name still fires there, as before (R2 / R6 / R5 with a unit `st`: p's, all three
    // oracles). With no unit `st` the block's import leaves the module's own `st` (R2)
    // or explicit `q::st` (R6) in place where the oracles read p's — ROADMAP §2 Scoping,
    // waiting on the name-keyed variable bindings (REMAINING_WORK §D). enum: iverilog
    // and verilator; the unpacked pair is E2002 in the block, unchanged.
    let blk = |k: &str, tag: &str, inner: &str| {
        format!(
            "  if (1) begin : g\n{inner}    st s2;\n    initial begin\n{}    end\n  end\n",
            show(k, tag, "s2")
        )
    };
    let modu = |body: String| format!("module t;\n{body}endmodule\n");
    for k in ["struct", "alias", "enum", "union"] {
        let own = local_td(k).replace("  typedef", "    typedef");
        for (unit, u) in [(unit_td(k), "U"), (String::new(), "noU")] {
            let r1 = format!(
                "{}{unit}{}",
                pkgs(k),
                modu(format!(
                    "  import p::*;\n{}",
                    blk(k, &format!("R1_{u}"), &format!("{own}    import p::*;\n"))
                ))
            );
            check(k, &format!("R1_{u}"), &r1, Won::Local);
            let r9 = format!(
                "{}{unit}{}",
                pkgs(k),
                modu(format!(
                    "{}  import p::*;\n{}",
                    local_td(k),
                    blk(k, &format!("R9_{u}"), "")
                ))
            );
            check(k, &format!("R9_{u}"), &r9, Won::Local);
            let r2 = format!(
                "{}{unit}{}",
                pkgs(k),
                modu(format!(
                    "{}  import p::*;\n{}",
                    local_td(k),
                    blk(k, &format!("R2_{u}"), "    import p::*;\n")
                ))
            );
            if u == "U" {
                check(k, &format!("R2_{u}"), &r2, Won::P);
            }
            let r6 = format!(
                "{}{unit}{}",
                pkgs(k),
                modu(format!(
                    "  import q::st;\n  import p::*;\n{}",
                    blk(k, &format!("R6_{u}"), "    import p::*;\n")
                ))
            );
            if u == "U" {
                check(k, &format!("R6_{u}"), &r6, Won::P);
            }
            let r5 = format!(
                "{}{unit}{}",
                pkgs(k),
                modu(format!(
                    "  import p::*;\n{}",
                    blk(k, &format!("R5_{u}"), "    import p::*;\n")
                ))
            );
            check(k, &format!("R5_{u}"), &r5, Won::P);
        }
        // Two wildcards of the block offering `st`, both repeated from the module:
        // ambiguous (§26.3) — iverilog and sv2v refuse, verilator reads the first
        // package's. An oracle split, not chased; with no unit `st` held at vita's answer
        // (verilator's).
        let two = format!(
            "{}{}",
            pkgs(k),
            modu(format!(
                "  import q::*;\n  import p::*;\n{}",
                blk(k, "RB_two_wild", "    import q::*;\n    import p::*;\n")
            ))
        );
        check(k, "RB_two_wild", &two, Won::Q);
    }
}

#[test]
fn an_import_in_a_generate_block_stays_refused() {
    // Loud before and after: E3009 names the construct. The oracles' lines, all three
    // (iverilog segfaults on the enum pair, sv2v rejects `num()`): the block's own `st`
    // beside its `import p::*` `8 6 2`, an explicit `import q::st` there `9 7 2`, and a
    // block import under the MODULE's `st` `12 4 8` (the nearer scope).
    let k = "struct";
    for (tag, blk) in [
        (
            "G_own_U",
            format!(
                "{}    import p::*;\n",
                local_td(k).replace("  typedef", "    typedef")
            ),
        ),
        (
            "G_explicit_U",
            "    import q::st;\n    import p::*;\n".to_string(),
        ),
    ] {
        let src = format!(
            "{}{}module t;\n  if (1) begin : g\n{blk}    st s2;\n    initial begin\n{}    end\n  end\nendmodule\n",
            pkgs(k),
            unit_td(k),
            show(k, tag, "s2")
        );
        is_loud(
            &src,
            "an import inside a generate block is not applied in v1",
        );
    }
    let nested = format!(
        "{}module t;\n{}  if (1) begin : g\n    import p::*;\n    st s2;\n    initial begin\n{}    end\n  end\nendmodule\n",
        pkgs(k),
        local_td(k),
        show(k, "G_nested_noU", "s2")
    );
    is_loud(
        &nested,
        "an import inside a generate block is not applied in v1",
    );
}

#[test]
fn an_unpacked_cross_kind_twin_meets_the_typedef_arm() {
    // Out of this slice, pinned so the next one moves it deliberately: a module
    // typedef of one kind beside a UNIT unpacked struct `st` of another reads the unit's
    // record with no import at all (`T_alias_over_unit_ustruct 10 3ff`; sv2v → iverilog
    // and verilator `8 ff`), and the typedef arm owns that (docs/PROBE_CATALOG.md). With
    // `import p::*` added, the import is now a no-op for the module's own `st`, so the
    // design prints exactly its import-free twin's line — PRE printed p's record
    // (`12 fff`) instead. The equality is the pin; neither side is the oracles'.
    let p = format!(
        "package p;\n  typedef {} st;\nendpackage\n",
        ty("ustruct", 12, "P")
    );
    let u = format!("typedef {} st;\n", ty("ustruct", 10, "U"));
    let bare = format!("{u}{}", module("alias", "T", "", &local_td("alias")));
    let imp = format!(
        "{p}{u}{}",
        module(
            "alias",
            "T",
            "",
            &format!("{}  import p::*;\n", local_td("alias"))
        )
    );
    assert_eq!(line(&imp, "T"), line(&bare, "T"));
    assert_eq!(
        line(&bare, "T"),
        "T 10 3ff",
        "the typedef arm's defect moved: re-measure"
    );
}

/// The class the class-control cells declare.
const CLS: &str = "  class st;\n    int v = 7;\n  endclass\n";

#[test]
fn a_class_named_like_a_package_type_keeps_its_binding() {
    // Controls (review round 1): a module's `class st` beside `import p::*` with a unit
    // wildcard offering `st` — the class answers (`h = new`, iverilog and verilator
    // `7`; sv2v has no classes), in a module body with the import after or before the
    // class, the same package again, no unit import, a header and a program. A first
    // cut that made a module's wildcard replace a unit IMPORT's `st` turned every one
    // of them E3009.
    let pq = "package p;\n  typedef logic [11:0] st;\nendpackage\npackage q;\n  typedef logic [8:0] st;\nendpackage\n";
    let use_h = "  st h;\n  initial begin\n    h = new;\n    $display(\"CLS %0d\", h.v);\n  end\n";
    for (unit, body) in [
        ("import q::*;\n", format!("{CLS}  import p::*;\n")),
        ("import q::*;\n", format!("  import p::*;\n{CLS}")),
        ("import q::*;\n", format!("{CLS}  import q::*;\n")),
        ("", format!("{CLS}  import p::*;\n")),
    ] {
        let src = format!("{pq}{unit}module t;\n{body}{use_h}endmodule\n");
        assert_eq!(line(&src, "CLS"), "CLS 7", "{src}");
    }
    let hdr = format!("{pq}import q::*;\nmodule t import p::*; ;\n{CLS}{use_h}endmodule\n");
    assert_eq!(line(&hdr, "CLS"), "CLS 7", "{hdr}");
    let prg = format!("{pq}import q::*;\nprogram t;\n{CLS}  import p::*;\n{use_h}endprogram\n");
    assert_eq!(line(&prg, "CLS"), "CLS 7", "{prg}");
}

#[test]
fn a_type_the_package_imported_is_not_offered() {
    // IEEE §26.3: a package does not re-export what it imports. p imports base's `st`
    // for its own variable, so the parser keeps a `p::st` layout for that variable's
    // replay — not a type p exports. PRE's §4.5.434 replacement wrote base's layout under
    // a unit typedef's `st` (`12 2 2 003`; all three oracles `12 4 8 0ff`); the unit
    // import's twin was already right and is the control (a first cut that replaced unit
    // imports turned it `12 2 2 003` too).
    let pre = "package base;\n  typedef struct packed { logic [1:0] a; logic [1:0] b; } st;\nendpackage\npackage p;\n  import base::*;\n  st pv;\nendpackage\n";
    let rd = "module t;\n  import p::*;\n  st s2;\n  initial begin\n    s2 = '0; s2.b = '1;\n    $display(\"PS %0d %0d %0d %h\", $bits(s2), $bits(s2.a), $bits(s2.b), s2);\n  end\nendmodule\n";
    let st12 = "typedef struct packed { logic [3:0] a; logic [7:0] b; } st;\n";
    let r = format!("{pre}package r;\n  {st12}endpackage\nimport r::*;\n{rd}");
    assert_eq!(line(&r, "PS"), "PS 12 4 8 0ff", "{r}");
    let td = format!("{pre}{st12}{rd}");
    assert_eq!(line(&td, "PS"), "PS 12 4 8 0ff", "{td}");
}

#[test]
fn a_union_flag_follows_the_types_the_package_exports() {
    // The union flag of a type p only IMPORTED (its layout kept for p's own variable)
    // marked the module's own struct, or r's struct, a union and refused the keyed
    // pattern on PRE; sv2v → iverilog and verilator print `06 bits=8`.
    let base = |n: &str| {
        format!("package base;\n  typedef union packed {{ logic [7:0] a; logic [7:0] b; }} {n};\nendpackage\npackage p;\n  import base::*;\n  {n} pv;\nendpackage\n")
    };
    let pat = |n: &str, tag: &str| {
        format!("  {n} s2;\n  initial begin\n    s2 = '{{a: 6'h1, b: 2'h2}};\n    $display(\"{tag} %h bits=%0d\", s2, $bits(s2));\n  end\nendmodule\n")
    };
    let own = format!(
        "{}module t;\n  typedef struct packed {{ logic [5:0] a; logic [1:0] b; }} st;\n  import p::*;\n{}",
        base("st"),
        pat("st", "PU")
    );
    assert_eq!(line(&own, "PU"), "PU 06 bits=8", "{own}");
    let r = format!(
        "{}package r;\n  typedef struct packed {{ logic [5:0] a; logic [1:0] b; }} ut;\nendpackage\nmodule t;\n  import r::*;\n  import p::*;\n{}",
        base("ut"),
        pat("ut", "PUN")
    );
    assert_eq!(line(&r, "PUN"), "PUN 06 bits=8", "{r}");
}

#[test]
fn an_explicit_import_of_a_value_keeps_the_wildcard_type_out() {
    // Review round 3. `import p::W;` binds the VALUE `W` (a package `parameter int`), so
    // a later `import q::*;` must not bring q's type `W` in (IEEE §26.3). PRE copied q's
    // type in beside it: `$bits(W)` read q's 16 bits and `W'(x)` cast to them (`K=6
    // bits=16 cast=ffff`); all three oracles print `K=6 bits=32 cast=1f`.
    let body = "package p;\n  parameter int W = 5;\nendpackage\npackage q;\n  typedef logic [15:0] W;\nendpackage\nmodule m;\n  import p::W;\n  import q::*;\n  localparam int K = W + 1;\n  logic [31:0] x;\n  initial begin\n    x = 32'hFFFF_FFFF;\n    $display(\"VI K=%0d bits=%0d cast=%h\", K, $bits(W), W'(x));\n  end\nendmodule\n";
    assert_eq!(line(body, "VI"), "VI K=6 bits=32 cast=1f", "{body}");
    // Out of this slice (docs/PROBE_CATALOG.md): a module value named like a UNIT
    // typedef — an explicitly imported parameter, or a `localparam` with no import at
    // all — does not shadow the unit's type in `$bits(W)` and `W'(x)` (`bits=8
    // cast=ff`; sv2v → iverilog and verilator `bits=32 cast=1f`). With the unit
    // typedef added, the wildcard is now a no-op and the design prints its
    // import-free twin's line; PRE printed q's (`bits=16 cast=ffff`). The equality is
    // the pin; neither side is the oracles'.
    let unit = format!("typedef logic [7:0] W;\n{body}");
    let bare = unit.replace("  import q::*;\n", "");
    assert_eq!(line(&unit, "VI"), line(&bare, "VI"));
    assert_eq!(
        line(&bare, "VI"),
        "VI K=6 bits=8 cast=ff",
        "the value-over-unit-type defect moved: re-measure"
    );
}

#[test]
fn a_package_variable_keeps_its_packages_type_beside_the_modules_own() {
    // Review round 3. p's variable `pv` of p's own `st`, a module's own `st` and its
    // `import p::*`, and a unit `st`: PRE's §4.5.434 replacement pointed the module's
    // `st v` at p's struct and refused `v.c` (E3010); all three oracles print the line
    // below — the module's `v` its own layout, p's `pv` p's.
    let src = "typedef struct packed { logic [7:0] x; } st;\npackage p;\n  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;\n  st pv = 8'h5A;\nendpackage\nmodule m;\n  typedef struct packed { logic [1:0] c; logic [5:0] d; } st;\n  import p::*;\n  st v;\n  initial begin\n    v = 8'hC3;\n    $display(\"PV pv=%h pa=%h pb=%h vc=%h vd=%h bits=%0d\", pv, pv.a, pv.b, v.c, v.d, $bits(v));\n  end\nendmodule\n";
    assert_eq!(
        line(src, "PV"),
        "PV pv=5a pa=5 pb=a vc=3 vd=03 bits=8",
        "{src}"
    );
}
