//! The system-function name facts have one home (ROADMAP §5.2 SYS-1a, ER §5.1):
//! `crates/hdl-ast/src/sysfn.rs` (the facts a `$name` alone decides) and
//! `crates/sim-ir/src/names.rs` (the id <-> name map, `sysfunc_name` / `sysfunc_id`).
//!
//! U = every string literal of name shape (`$` then `[a-z0-9_$]`) in `sysfn.rs` and inside
//! the bodies of `fn sysfunc_name` / `fn sysfunc_id` in `names.rs` (so `systask_name`'s task
//! names stay out; the five string-output tasks `$sformat` `$swrite` `$swriteb` `$swriteh`
//! `$swriteo` enter U through `sysfn.rs`'s argument-direction columns, SYS-1b, the task and
//! the function form of one name asking one fact). Every `src` file of every crate is read with `proc_macro2` (comments
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
/// |U|: the union of the columns, `sysfunc_id`'s domain and `sysfunc_name`'s `$` names
/// (SYS-1a 68; SYS-1b +5 string-output task names +6 SVA sampled-value names).
const U_SIZE: usize = 79;

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
    // dispatch keys: a guard selecting the routine written for that one name.
    ("crates/elaborate/src/stmt_main.rs", "lower_stmt_inner", 11, "1 dispatch key (`$sformatf`) + task form: the display family's format index and its list (5 + 5)"),
    ("crates/elaborate/src/expr_main.rs", "lower_expr_ungated", 6, "6 dispatch keys (seeded `$random`, `$fopen`, `$value$plusargs`: each guard selects its own refusal text)"),
    ("crates/elaborate/src/sva_decl.rs", "rewrite_sampled", 6, "dispatch keys: each sampled name's rewrite arm"),
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
    // a special form's dispatch key + the id it mints (sim-ir names.rs NOT_FROM_SYSFUNC_ID
    // records each by site) + its own arity text: one unit, SYS-1b's ruling (U4).
    ("crates/elaborate/src/crv.rs", "dist_uniform_special", 7, "dispatch key + minted id (names.rs NOT_FROM_SYSFUNC_ID)"),
    ("crates/elaborate/src/sys_special.rs", "file_read_int_special", 3, "dispatch key + minted id (names.rs NOT_FROM_SYSFUNC_ID)"),
    ("crates/elaborate/src/sys_special.rs", "scanf_special", 2, "dispatch key + minted id (names.rs NOT_FROM_SYSFUNC_ID)"),
    // task form.
    ("crates/elaborate/src/systask.rs", "map_systask", 6, "task form: `$cast` + the 5 string-output tasks"),
    ("crates/elaborate/src/systask.rs", "radix_of_systask", 3, "task form: radix"),
    ("crates/elaborate/src/systask.rs", "lower_systask", 1, "task form: the `$sformat into` / `$swrite into` label"),
];

/// One row of `columns_keep_pre`'s argument table: (name, `write_arg_range` as (start, end)
/// for argc 0..=5, `syscall_writes_arg`'s positions among 0..=5, `syscall_read_args` as
/// (start, len) for argc 0..=5).
type ArgRow = (
    &'static str,
    [(usize, usize); 6],
    &'static [usize],
    [(usize, usize); 6],
);

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
    // Near misses of the SYS-1b columns (outside each set; most are in U already).
    for n in [
        "$feof",
        "$fclose",
        "$ftell",
        "$fwrite",
        "$sformatf",
        "$urandom",
        "$swritex",
        "$Fgetc",
        "$dist",
        "$dist_",
        "$castx",
        "$sampled_x",
        "$isunknown",
        "$countones",
        "$fopenx",
    ] {
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

    // ---- SYS-1b: the special-form columns (PRE file:line at 3605bbf6) ----
    // elaborate/src/hoist/special.rs:49 is_hoistable_rhs_only_sysfunc
    let c1 = pre(&[
        "$fgetc",
        "$ungetc",
        "$fgets",
        "$fread",
        "$fscanf",
        "$sscanf",
        "$value$plusargs",
        "$fopen",
    ]);
    // elaborate/src/hoist/special.rs:128 observes_fd_state
    let c3 = pre(&["$feof"]);
    // elaborate/src/stmt_main.rs:462-485 (lower_stmt_inner, inline)
    let c6 = pre(&[
        "$sscanf",
        "$fscanf",
        "$fgets",
        "$fread",
        "$value$plusargs",
        "$fgetc",
        "$ungetc",
        "$random",
        "$dist_uniform",
    ]);
    // elaborate/src/expr_main.rs:1255-1258 (lower_expr_ungated, inline)
    let c7 = pre(&[
        "$fgetc", "$ungetc", "$fgets", "$fread", "$fscanf", "$sscanf",
    ]);
    // elaborate/src/expr_main.rs:1300-1310 (lower_expr_ungated, inline)
    let c8 = pre(&[
        "$dist_uniform",
        "$dist_normal",
        "$dist_exponential",
        "$dist_poisson",
        "$dist_chi_square",
        "$dist_t",
        "$dist_erlang",
        "$cast",
    ]);
    // elaborate/src/sva_ast.rs:184 is_sva_sampled_fn
    let c9 = pre(&["$past", "$rose", "$fell", "$stable", "$changed", "$sampled"]);
    // elaborate/src/sva_prop.rs:907 sva_prev_for's `matches!(func, "$rose" | "$fell")`
    let c10 = pre(&["$rose", "$fell"]);
    // (a) member lists over U plus the near misses.
    assert_eq!(set(sysfn::is_hoistable_rhs_only_sysfunc), c1);
    assert_eq!(set(sysfn::observes_fd_state), c3);
    assert_eq!(set(sysfn::bare_call_routed), c6);
    assert_eq!(set(sysfn::refused_nested_stream_read), c7);
    assert_eq!(set(sysfn::refused_nested_seed_or_dest), c8);
    assert_eq!(set(sysfn::is_sva_sampled_fn), c9);
    assert_eq!(set(sysfn::sva_lsb_edge), c10);

    // (b) the argument table, PRE's values literal: for argc 0..=5, write_arg_range
    // (hoist/special.rs:77) as (start, end), syscall_writes_arg (systask.rs:24) as the
    // written positions among 0..=5, syscall_read_args (systask.rs:36) as (start, len) of
    // the returned sub-slice ((0, 0) when empty).
    #[rustfmt::skip]
    let table: &[ArgRow] = &[
        ("$sformat", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$swrite", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$swriteb", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$swriteh", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$swriteo", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$fgets", [(0, 0), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$fread", [(0, 0), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$cast", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 0), (1, 1), (1, 2), (1, 3), (1, 4)]),
        ("$sscanf", [(0, 0), (1, 1), (2, 2), (2, 3), (2, 4), (2, 5)], &[2, 3, 4, 5], [(0, 0), (0, 1), (0, 2), (0, 2), (0, 2), (0, 2)]),
        ("$fscanf", [(0, 0), (1, 1), (2, 2), (2, 3), (2, 4), (2, 5)], &[2, 3, 4, 5], [(0, 0), (0, 1), (0, 2), (0, 2), (0, 2), (0, 2)]),
        ("$value$plusargs", [(0, 0), (1, 1), (1, 2), (1, 2), (1, 2), (1, 2)], &[1], [(0, 0), (0, 1), (0, 1), (0, 1), (0, 1), (0, 1)]),
        ("$random", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_uniform", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_normal", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_exponential", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_poisson", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_chi_square", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_t", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$dist_erlang", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[0], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$fgetc", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$display", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$urandom", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
        ("$signed", [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)], &[], [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]),
    ];
    // The arguments `a0` .. `a4`, each a distinct string literal, so a returned sub-slice
    // reads back as the positions it holds.
    let args: Vec<hdl_ast::Expr> = (0..5)
        .map(|i| hdl_ast::Expr {
            kind: hdl_ast::ExprKind::StrLit {
                raw: format!("a{i}"),
            },
            span: hdl_ast::Span::new(0, 0),
        })
        .collect();
    let pos_of = |e: &hdl_ast::Expr| -> usize {
        match &e.kind {
            hdl_ast::ExprKind::StrLit { raw } => raw[1..].parse().expect("a<i>"),
            _ => unreachable!(),
        }
    };
    let read_slice = |name: &str, argc: usize| -> Vec<usize> {
        sysfn::syscall_read_args(name, &args[..argc])
            .iter()
            .map(pos_of)
            .collect()
    };
    let written = |name: &str, argc: usize| -> BTreeSet<usize> {
        (0..argc)
            .filter(|&i| sysfn::syscall_writes_arg(name, i))
            .collect()
    };
    for (name, c2_rows, c4_pos, c5_rows) in table {
        for argc in 0..=5 {
            let r = sysfn::write_arg_range(name, argc);
            assert_eq!(
                (r.start, r.end),
                c2_rows[argc],
                "write_arg_range({name}, {argc})"
            );
            let got = read_slice(name, argc);
            let (st, len) = c5_rows[argc];
            let want: Vec<usize> = (st..st + len).collect();
            assert_eq!(got, want, "syscall_read_args({name}, argc {argc})");
        }
        let pos: Vec<usize> = (0..=5)
            .filter(|&i| sysfn::syscall_writes_arg(name, i))
            .collect();
        assert_eq!(&pos[..], *c4_pos, "syscall_writes_arg({name}, 0..=5)");
        // `i >= 2` is unbounded (PRE): a far position is written by the scanf pair only.
        assert_eq!(
            sysfn::syscall_writes_arg(name, 1000),
            matches!(*name, "$sscanf" | "$fscanf"),
            "syscall_writes_arg({name}, 1000)"
        );
    }
    // Every name outside the table: no written range, no written position, every arg read.
    for n in probe.iter().map(|s| s.as_str()) {
        if table.iter().any(|(t, ..)| *t == n) {
            continue;
        }
        for argc in 0..=5 {
            assert!(sysfn::write_arg_range(n, argc).is_empty(), "{n}");
            assert!(written(n, argc).is_empty(), "{n}");
            assert_eq!(read_slice(n, argc), (0..argc).collect::<Vec<_>>(), "{n}");
        }
    }

    // (c) the lane relations (PRE differences kept, never merged).
    // L1: write_arg_range = syscall_writes_arg restricted to C1 and `i < argc`.
    for n in &c1 {
        for argc in 0..=5 {
            let r: BTreeSet<usize> = sysfn::write_arg_range(n, argc).collect();
            assert_eq!(r, written(n, argc), "L1 {n} argc {argc}");
        }
    }
    // L2: on the 11 destination-taking names the read slice and the written positions are
    // disjoint and cover every argument, except `$value$plusargs`'s past the second
    // (neither); the seed of `$random` / `$dist_*` is read AND written.
    let dest_names = pre(&[
        "$sformat",
        "$swrite",
        "$swriteb",
        "$swriteh",
        "$swriteo",
        "$fgets",
        "$fread",
        "$cast",
        "$sscanf",
        "$fscanf",
        "$value$plusargs",
    ]);
    for n in &dest_names {
        for argc in 0..=5 {
            let rd: BTreeSet<usize> = read_slice(n, argc).into_iter().collect();
            let wr = written(n, argc);
            assert!(rd.is_disjoint(&wr), "L2 {n} argc {argc}");
            let all: BTreeSet<usize> = rd.union(&wr).copied().collect();
            let cover = if *n == "$value$plusargs" {
                argc.min(2)
            } else {
                argc
            };
            assert_eq!(all, (0..cover).collect(), "L2 {n} argc {argc}");
        }
    }
    for n in diff(&c8, &pre(&["$cast"])).iter().chain(["$random"].iter()) {
        for argc in 1..=5 {
            let rd: BTreeSet<usize> = read_slice(n, argc).into_iter().collect();
            assert_eq!(
                rd.intersection(&written(n, argc))
                    .copied()
                    .collect::<Vec<_>>(),
                vec![0],
                "L2 seed {n} argc {argc}"
            );
        }
    }
    // L3: the refusal set and the bare route set against the hoisted set.
    assert_eq!(diff(&c1, &pre(&["$value$plusargs", "$fopen"])), c7);
    let mut c6_from_c1 = diff(&c1, &pre(&["$fopen"]));
    c6_from_c1.extend(["$random", "$dist_uniform"]);
    assert_eq!(c6_from_c1, c6);
    // L4: the LSB-edge pair is within the sampled-value list.
    assert!(c10.is_subset(&c9));
    assert_eq!(
        diff(&c9, &c10),
        pre(&["$past", "$stable", "$changed", "$sampled"])
    );

    // (d) refused_nested_seed_or_dest minus `$cast` = the `$dist_*` names crv.rs
    // `dist_uniform_special` matches (PRE's "MUST stay in sync" comment, as a test).
    let crv = files
        .iter()
        .find(|(rel, _)| rel == "crates/elaborate/src/crv.rs")
        .expect("crv.rs");
    let dist_names: BTreeSet<String> = literals(&crv.1)
        .into_iter()
        .filter(|(f, v)| f == "dist_uniform_special" && is_name_shape(v))
        .map(|(_, v)| v)
        .collect();
    let c8_dist: BTreeSet<String> = diff(&c8, &pre(&["$cast"]))
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(dist_names.len(), 7, "{dist_names:?}");
    assert_eq!(dist_names, c8_dist);
}
