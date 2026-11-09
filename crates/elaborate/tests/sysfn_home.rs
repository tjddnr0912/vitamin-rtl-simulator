//! The system-function name facts have one home (ROADMAP §5.2 SYS-1a, ER §5.1):
//! `crates/hdl-ast/src/sysfn.rs` (the facts a `$name` alone decides) and
//! `crates/sim-ir/src/names.rs` (the id <-> name map, `sysfunc_name` / `sysfunc_id`).
//!
//! U = every string literal of name shape (`$` then `[a-z0-9_$]`) in `sysfn.rs` and inside
//! the bodies of `fn sysfunc_name` / `fn sysfunc_id` in `names.rs` (so `systask_name`'s task
//! names stay out). Every `src` file of every crate is read with `proc_macro2` (comments
//! dropped; doc comments arrive as `#[doc]` / `#![doc]` groups and every attribute group is
//! skipped; the item after a `#[cfg(test)]` is skipped), each literal token's value is read
//! through `syn::Lit` (escapes, raw strings), and a literal whose WHOLE value is in U is a
//! hit: `"$fopen"` is, `"$fopen takes ..."` is not. The two homes are skipped whole.
//!
//! Every hit outside the homes is an EXCEPTION keyed by (file, enclosing fn), its count
//! exact, with its reason (a per-file count would miss a swap inside one file). A new
//! literal fails here: spell the fact in the home, or add the key with its reason.
//! `columns_keep_pre` pins each column of the home against its PRE set (a contract pin,
//! not a second rule: a value row that changes a column edits both).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

const HOME_FACTS: &str = "crates/hdl-ast/src/sysfn.rs";
const HOME_IDS: &str = "crates/sim-ir/src/names.rs";
/// The `names.rs` functions whose literals are U (the id <-> name map).
const ID_FNS: [&str; 2] = ["sysfunc_name", "sysfunc_id"];
/// |U|: the union of the columns, `sysfunc_id`'s domain and `sysfunc_name`'s `$` names.
const U_SIZE: usize = 68;

/// (file, enclosing fn, hits, reason). `-` = outside any fn.
#[rustfmt::skip]
const EXCEPTIONS: &[(&str, &str, usize, &str)] = &[
    // VAL-1: a fold core or a G1 VAL arm computes the value (the name selects the arm).
    ("crates/elaborate/src/const_fn.rs", "const_eval_in_scope", 3, "VAL-1 fold core arms ($clog2 $rtoi $bits)"),
    ("crates/elaborate/src/const_fn.rs", "eval_const_env", 1, "VAL-1 fold core arm"),
    ("crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 1, "VAL-1 fold core arm"),
    ("crates/elaborate/src/const_wide.rs", "fold_region", 10, "VAL-1 fold core arms"),
    ("crates/elaborate/src/expr_size_hier.rs", "env_fold", 1, "VAL-1 fold core arm"),
    ("crates/elaborate/src/const_array.rs", "const_dim_query", 10, "VAL-1 G1 arm: the query-shape sets and per-query values (H1)"),
    // H1: the lowering's §20.7 introspection fold (no home row yet).
    ("crates/elaborate/src/expr_special.rs", "try_introspect_fold", 20, "H1 the lowering's introspection fold, a lane of const_dim_query"),
    // TYPE-1: the width / sign / kind walkers (G1 TYPE).
    ("crates/elaborate/src/expr_size_ctx.rs", "ctx_signed_impl", 2, "TYPE-1 sign rule arms"),
    ("crates/elaborate/src/expr_size_ctx.rs", "size_ctx_self_width", 2, "TYPE-1 self-width rule arms"),
    ("crates/elaborate/src/expr_special.rs", "bits_of_selfdet", 2, "TYPE-1 self-width rule arms"),
    ("crates/elaborate/src/strings.rs", "expr_is_string_ast", 1, "TYPE-1 kind rule arm"),
    // range_rules: the §20.7 values (part 4 / 5, owner 2026-11-06).
    ("crates/hdl-ast/src/range_rules.rs", "dim_query", 6, "range_rules §20.7 value arms"),
    ("crates/hdl-ast/src/range_rules.rs", "md_query", 6, "range_rules §20.7 value arms"),
    // PARSE-1: the parse-time fold and its symbolic gate.
    ("crates/hdl-parser/src/expr_primary.rs", "cw_eval", 1, "PARSE-1 parse-time fold"),
    ("crates/hdl-parser/src/expr_primary.rs", "cw_shape", 1, "PARSE-1 parse-time fold"),
    ("crates/hdl-parser/src/expr_primary.rs", "try_const_index", 1, "PARSE-1 parse-time fold"),
    ("crates/hdl-parser/src/expr_primary.rs", "expr_primary", 1, "PARSE-1 parse-time fold"),
    ("crates/hdl-parser/src/packed_md.rs", "rewrite_packed_md_dim_query", 4, "PARSE-1 the parser's md value arms"),
    ("crates/hdl-parser/src/sym_struct.rs", "names_an_overridable", 1, "PARSE-1 symbolic gate"),
    // producer: builds a SysCall node.
    ("crates/hdl-parser/src/struct_sel.rs", "parse_record_array_member", 1, "producer (builds the `$signed` node)"),
    // SYS-1b: the special-form facts.
    ("crates/elaborate/src/hoist/special.rs", "is_hoistable_rhs_only_sysfunc", 8, "SYS-1b rhs-only effectful set"),
    ("crates/elaborate/src/hoist/special.rs", "observes_fd_state", 1, "SYS-1b fd observer"),
    ("crates/elaborate/src/hoist/special.rs", "write_arg_range", 5, "SYS-1b written-argument range"),
    ("crates/elaborate/src/systask.rs", "syscall_writes_arg", 14, "SYS-1b written argument"),
    ("crates/elaborate/src/systask.rs", "syscall_read_args", 6, "SYS-1b read arguments"),
    ("crates/elaborate/src/stmt_main.rs", "lower_stmt_inner", 10, "SYS-1b statement route set (9) + 1 dispatch key"),
    ("crates/elaborate/src/expr_main.rs", "lower_expr_ungated", 20, "SYS-1b rhs-only refusal sets (6 + 8) + 6 dispatch keys"),
    // dispatch keys: a guard selecting the routine written for that one name.
    ("crates/elaborate/src/stmt_flow.rs", "lower_branch_cond", 1, "dispatch key"),
    ("crates/elaborate/src/frames_body.rs", "lower_return_rhs", 1, "dispatch key"),
    ("crates/elaborate/src/hoist/mod.rs", "hoist_sformatf_arg", 1, "dispatch key"),
    ("crates/elaborate/src/hoist/sformatf.rs", "hoist_sformatf_children", 1, "dispatch key"),
    ("crates/elaborate/src/expr_cast.rs", "cast_special", 1, "dispatch key"),
    ("crates/elaborate/src/crv.rs", "random_seeded_special", 1, "dispatch key"),
    ("crates/elaborate/src/sys_special.rs", "fgets_special", 1, "dispatch key"),
    ("crates/elaborate/src/sys_special.rs", "fopen_special", 1, "dispatch key"),
    ("crates/elaborate/src/sys_special.rs", "fread_special", 1, "dispatch key"),
    ("crates/elaborate/src/sys_special.rs", "sformatf_special", 1, "dispatch key"),
    ("crates/elaborate/src/sys_special.rs", "value_plusargs_special", 1, "dispatch key"),
    // signature tables (name -> id + arity): SYS-1b decides.
    ("crates/elaborate/src/crv.rs", "dist_uniform_special", 7, "signature table (SYS-1b)"),
    ("crates/elaborate/src/sys_special.rs", "file_read_int_special", 3, "signature table (SYS-1b)"),
    ("crates/elaborate/src/sys_special.rs", "scanf_special", 2, "signature table (SYS-1b)"),
    // task form.
    ("crates/elaborate/src/systask.rs", "map_systask", 1, "task form (`$cast` the task)"),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every `crates/<crate>/src/**/*.rs`, relative to the workspace root, sorted.
fn src_files(root: &Path) -> Vec<(String, String)> {
    let mut crates: Vec<_> = std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.join("src").is_dir())
        .collect();
    crates.sort();
    let mut files = Vec::new();
    for c in crates {
        rs_files(&c.join("src"), &mut files);
    }
    files
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&p).expect("read source");
            (rel, src)
        })
        .collect()
}

fn is_name_shape(t: &str) -> bool {
    t.len() > 1
        && t.starts_with('$')
        && t[1..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'$')
}

/// `(enclosing fn, value)` of every string literal in `src`, attributes and the item
/// after a `#[cfg(test)]` skipped. `-` = outside any fn.
fn literals(src: &str) -> Vec<(String, String)> {
    let ts = TokenStream::from_str(src).unwrap_or_else(|e| panic!("tokenize: {e:?}"));
    let mut out = Vec::new();
    walk(ts, "-", &mut out);
    out
}

fn is_cfg_test(attr: &TokenStream) -> bool {
    let s: String = attr
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    s == "cfg(test)"
}

fn walk(ts: TokenStream, cur: &str, out: &mut Vec<(String, String)>) {
    let tt: Vec<TokenTree> = ts.into_iter().collect();
    let mut pending_fn: Option<String> = None;
    let mut i = 0;
    while i < tt.len() {
        match &tt[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                // `#[..]` or `#![..]`: skipped whole (doc comments arrive here).
                let mut j = i + 1;
                if matches!(tt.get(j), Some(TokenTree::Punct(q)) if q.as_char() == '!') {
                    j += 1;
                }
                if let Some(TokenTree::Group(g)) = tt.get(j) {
                    if g.delimiter() == Delimiter::Bracket {
                        i = j + 1;
                        if is_cfg_test(&g.stream()) {
                            // Skip the item: through its first brace body or its `;`.
                            while i < tt.len() {
                                match &tt[i] {
                                    TokenTree::Group(b) if b.delimiter() == Delimiter::Brace => {
                                        i += 1;
                                        break;
                                    }
                                    TokenTree::Punct(q) if q.as_char() == ';' => {
                                        i += 1;
                                        break;
                                    }
                                    _ => i += 1,
                                }
                            }
                        }
                        continue;
                    }
                }
                i += 1;
            }
            TokenTree::Ident(id) if id == "fn" => {
                if let Some(TokenTree::Ident(name)) = tt.get(i + 1) {
                    pending_fn = Some(name.to_string());
                    i += 2;
                    continue;
                }
                i += 1;
            }
            TokenTree::Punct(p) if p.as_char() == ';' => {
                pending_fn = None;
                i += 1;
            }
            TokenTree::Group(g) => {
                if g.delimiter() == Delimiter::Brace {
                    if let Some(f) = pending_fn.take() {
                        walk(g.stream(), &f, out);
                        i += 1;
                        continue;
                    }
                }
                walk(g.stream(), cur, out);
                i += 1;
            }
            TokenTree::Literal(l) => {
                if let syn::Lit::Str(s) = syn::Lit::new(l.clone()) {
                    out.push((cur.to_string(), s.value()));
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// U, read from the two homes.
fn home_names(files: &[(String, String)]) -> BTreeSet<String> {
    let mut u = BTreeSet::new();
    for (rel, src) in files {
        let take_fn = |f: &str| match rel.as_str() {
            HOME_FACTS => true,
            HOME_IDS => ID_FNS.contains(&f),
            _ => false,
        };
        if rel != HOME_FACTS && rel != HOME_IDS {
            continue;
        }
        for (f, v) in literals(src) {
            if take_fn(&f) && is_name_shape(&v) {
                u.insert(v);
            }
        }
    }
    u
}

/// (file, fn) -> hits, outside the homes.
fn hits(files: &[(String, String)], u: &BTreeSet<String>) -> BTreeMap<(String, String), usize> {
    let mut m = BTreeMap::new();
    for (rel, src) in files {
        if rel == HOME_FACTS || rel == HOME_IDS {
            continue;
        }
        for (f, v) in literals(src) {
            if u.contains(&v) {
                *m.entry((rel.clone(), f)).or_insert(0) += 1;
            }
        }
    }
    m
}

#[test]
fn name_literals_live_in_the_homes() {
    let root = workspace_root();
    let files = src_files(&root);
    let u = home_names(&files);
    assert_eq!(u.len(), U_SIZE, "|U| (a column edit edits U_SIZE): {u:?}");
    let got = hits(&files, &u);
    let want: BTreeMap<(String, String), usize> = EXCEPTIONS
        .iter()
        .map(|(f, n, c, _)| ((f.to_string(), n.to_string()), *c))
        .collect();
    assert_eq!(
        want.len(),
        EXCEPTIONS.len(),
        "an EXCEPTIONS key listed twice"
    );
    if got != want {
        let mut msg = String::from("measured (file, fn, hits) != EXCEPTIONS:\n");
        for k in got.keys().chain(want.keys()).collect::<BTreeSet<_>>() {
            let (g, w) = (got.get(k).copied(), want.get(k).copied());
            if g != w {
                msg += &format!("  {} | {} : measured {g:?} listed {w:?}\n", k.0, k.1);
            }
        }
        panic!("{msg}");
    }
}

#[test]
fn scanner_matches_its_controls() {
    let u: BTreeSet<String> = ["$clog2", "$bits", "$fdisplay_not_in_u"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let count = |src: &str| {
        literals(src)
            .into_iter()
            .filter(|(_, v)| u.contains(v))
            .count()
    };
    // Positive controls.
    assert_eq!(
        count(r#"fn f(n: &str) -> bool { matches!(n, "$clog2" | "$bits") }"#),
        2
    );
    assert_eq!(count(r#"fn f(n: &str) -> bool { n == "$clog2" }"#), 1);
    assert_eq!(count(r##"fn f(n: &str) -> bool { n == r#"$clog2"# }"##), 1);
    assert_eq!(
        literals(r#"fn outer() { fn inner() { let _ = "$bits"; } }"#),
        vec![("inner".to_string(), "$bits".to_string())]
    );
    assert_eq!(
        literals(r#"impl X { fn m(&self) -> &str { "$clog2" } }"#),
        vec![("m".to_string(), "$clog2".to_string())]
    );
    assert_eq!(
        literals(r#"const N: &str = "$clog2";"#),
        vec![("-".to_string(), "$clog2".to_string())]
    );
    // Negative controls: each 0.
    for neg in [
        r#"fn f() -> &'static str { "$clog2 needs one argument" }"#,
        r#"fn f() { // "$clog2"
        }"#,
        r#"/// "$clog2"
        fn f() {}"#,
        r#"fn f() { /* "$clog2" */ }"#,
        r#"fn f() { let _ = '"'; let _ = "x"; }"#,
        r#"fn f() -> &'static str { "\"$clog2\"" }"#,
        r#"fn f() -> &'static str { "$clog2x" }"#,
        r#"fn f() -> &'static str { "$fdisplay" }"#,
        r#"#[cfg(test)] mod t { fn f() -> &'static str { "$clog2" } }"#,
        r#"#[cfg(test)] fn t() -> &'static str { "$clog2" }"#,
        r#"#[doc = "$clog2"] fn f() {}"#,
        r#"#![doc = "$clog2"]"#,
        r#"fn f() -> &'static [u8] { b"$clog2" }"#,
    ] {
        assert_eq!(count(neg), 0, "negative control counted: {neg}");
    }
    // A `#[cfg(test)]` item ends at its body: the item after it is read again.
    assert_eq!(
        count(r#"#[cfg(test)] mod t { fn a() { let _ = "$bits"; } } fn b() { let _ = "$bits"; }"#),
        1
    );
    // Name shape.
    for (t, ok) in [
        ("$clog2", true),
        ("$value$plusargs", true),
        ("$dist_chi_square", true),
        ("$readmem target", false),
        ("$Clog2", false),
        ("$", false),
        (".size()", false),
    ] {
        assert_eq!(is_name_shape(t), ok, "{t}");
    }
}

/// Each column's accepted subset of U plus the negatives equals its PRE set (PRE
/// file:line at 4a8f086b), and the lane relations between columns hold.
#[test]
fn columns_keep_pre() {
    use hdl_ast::sysfn;
    let root = workspace_root();
    let files = src_files(&root);
    let mut probe = home_names(&files);
    for n in ["$fdisplay", "$foo", "$Clog2", "$clog", ""] {
        probe.insert(n.to_string());
    }
    let set = |f: fn(&str) -> bool| -> BTreeSet<&str> {
        probe.iter().map(|s| s.as_str()).filter(|s| f(s)).collect()
    };
    let pre =
        |names: &[&'static str]| -> BTreeSet<&'static str> { names.iter().copied().collect() };

    // elaborate/src/const_fn_width.rs:64 sys_fn_is_integer
    let folds_integer = pre(&["$clog2", "$bits", "$rtoi"]);
    // elaborate/src/const_array.rs:266 is_dim_query_name = hdl-parser/src/packed_md.rs:705
    let array_query = pre(&[
        "$size",
        "$left",
        "$right",
        "$low",
        "$high",
        "$increment",
        "$dimensions",
        "$unpacked_dimensions",
    ]);
    // elaborate/src/edge_gate.rs:633 is_other_sys_call's complement
    let width_neutral = pre(&["$clog2", "$signed", "$unsigned"]);
    // elaborate/src/param_query.rs:122 real_conversion_is_folded
    let real_conversion = pre(&["$clog2", "$rtoi"]);
    // elaborate/src/expr_size_hier_exact.rs:44 expr_exact
    let exact_i64 = pre(&["$clog2"]);
    // elaborate/src/hoist/sformatf.rs:119-140 expr_is_inert
    let inert = pre(&[
        "$sformatf",
        "$time",
        "$stime",
        "$realtime",
        "$bits",
        "$clog2",
        "$signed",
        "$unsigned",
        "$size",
        "$left",
        "$right",
        "$high",
        "$low",
        "$dimensions",
        "$increment",
        "$typename",
        "$countones",
        "$onehot",
        "$onehot0",
        "$isunknown",
    ]);
    // hdl-ast/src/walk.rs:85 syscall_does_not_evaluate
    let does_not_evaluate = pre(&[
        "$bits",
        "$size",
        "$high",
        "$low",
        "$left",
        "$right",
        "$increment",
        "$dimensions",
        "$unpacked_dimensions",
        "$typename",
    ]);
    // elaborate/src/ast_query.rs:553 count_reads_runtime_net
    let type_query = pre(&[
        "$bits",
        "$size",
        "$high",
        "$low",
        "$left",
        "$right",
        "$increment",
        "$dimensions",
        "$unpacked_dimensions",
        "$isunbounded",
        "$typename",
    ]);
    // elaborate/src/ast_query.rs:580 count_reads_runtime_net's runtime subset
    let runtime_size = pre(&["$size", "$high", "$right", "$bits"]);

    assert_eq!(set(sysfn::folds_integer), folds_integer);
    assert_eq!(set(sysfn::is_array_query_function), array_query);
    assert_eq!(set(sysfn::width_neutral), width_neutral);
    assert_eq!(set(sysfn::real_conversion), real_conversion);
    assert_eq!(set(sysfn::exact_i64), exact_i64);
    assert_eq!(set(sysfn::inert), inert);
    assert_eq!(set(sysfn::does_not_evaluate), does_not_evaluate);
    assert_eq!(set(sysfn::type_query), type_query);
    assert_eq!(set(sysfn::type_query_runtime_size), runtime_size);

    // elaborate/src/systask.rs:301 map_sysfunc's domain (40), now sim-ir sysfunc_id.
    let map_domain: BTreeSet<&str> = probe
        .iter()
        .map(|s| s.as_str())
        .filter(|s| sim_ir::sysfunc_id(s).is_some())
        .collect();
    assert_eq!(map_domain.len(), 40, "{map_domain:?}");
    for n in [
        "$bits",
        "$fgets",
        "$cast",
        "$sformatf",
        "$dist_uniform",
        "$size",
    ] {
        assert!(sim_ir::sysfunc_id(n).is_none(), "{n} is not mapped in PRE");
    }

    // Lane relations (distinct questions, never merged).
    let diff = |a: &BTreeSet<&'static str>, b: &BTreeSet<&'static str>| -> BTreeSet<&'static str> {
        a.difference(b).copied().collect()
    };
    assert_eq!(
        diff(&type_query, &does_not_evaluate),
        pre(&["$isunbounded"])
    );
    assert!(does_not_evaluate.is_subset(&type_query));
    assert_eq!(diff(&folds_integer, &real_conversion), pre(&["$bits"]));
    assert!(runtime_size.is_subset(&type_query));
    assert!(array_query.is_subset(&type_query));
    assert!(array_query.is_subset(&does_not_evaluate));
    assert_eq!(
        diff(&type_query, &inert),
        pre(&["$unpacked_dimensions", "$isunbounded"])
    );
}
