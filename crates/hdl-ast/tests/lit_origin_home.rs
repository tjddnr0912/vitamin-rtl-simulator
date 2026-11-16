//! `ExprKind::IntLit.origin: LitOrigin` (ROADMAP §5.2 PARSE-1, the provenance piece) says
//! which code constructed a literal, and this test is what makes that true by PATH: the
//! tag is decided by, and checked on, the file and fn that construct the node, never by
//! reading how its number was decided. Counted over every `crates/*/src/**/*.rs` and
//! `crates/*/tests/**/*.rs`, per (file, enclosing fn):
//!
//! - H0 the definition (`hdl-ast/src/lib.rs`): `enum LitOrigin { Source, Parser, Elab }`
//!   (unit variants, that order), its one attribute `#[derive(Debug, Clone, Copy, Eq,
//!   Serialize, Deserialize, SchemaHash)]`, one `impl PartialEq for LitOrigin` whose body is
//!   `fn eq(&self, _other: &Self) -> bool { true }`, no other `impl .. for LitOrigin`
//!   anywhere (no hand `Default`, `Hash`, `PartialOrd`, `Ord`), and `IntLit`'s fields
//!   exactly `kind: IntLitKind, raw: String, origin: LitOrigin` (origin last, no attribute).
//! - H1 `Source`: one production construction, in `hdl-parser/src/expr_primary.rs`
//!   `lit_int`, whose `raw` is the local `let raw = self.cur_text().to_string();` (shorthand
//!   field) ; `lit_int` is called three times, all from `expr_primary`.
//! - H2 the path rule: every production `LitOrigin :: <V>` per (file, fn, variant) is the
//!   table `ORIGIN_TABLE`, and each row's variant is the path's: `crates/hdl-parser/src` ->
//!   `Parser` (but H1's one `Source`), `crates/hdl-ast/src/range_rules.rs` `dec_lit` ->
//!   `Parser`, `crates/elaborate/src` -> `Elab` ; any other file fails. Totals 1 / 18 / 18.
//!   The funnel (premise judge 3 L5): the part-5 builders of `range_rules.rs` (every `pub
//!   fn` after its `5. the parser's respell builders` header, read from the file) are
//!   reached by PATH only from `crates/hdl-parser/src` (`FUNNEL_TABLE`, per (file, name)) ;
//!   elsewhere in production 0 `range_rules :: <name>` paths, 0 use-trees naming one, 0
//!   `range_rules :: *`, 0 `range_rules as` ; inside `range_rules.rs` no fn before the
//!   header calls one. Keyed on paths and use-trees, never on bare names (elaborate's
//!   closures are spelled `sub` / `add`).
//! - H3 the positive set: outside the `enum LitOrigin` item, its `impl PartialEq` block and
//!   `IntLit`'s field declaration, every `LitOrigin` token is (i) an import `use
//!   hdl_ast::LitOrigin;` / `use hdl_ast::{.., LitOrigin, ..}` (inside `crates/hdl-ast/src`
//!   `use crate::{..}`), no `as`, no `pub` ; or (ii) `LitOrigin :: Source|Parser|Elab`
//!   directly after `origin :` inside an `IntLit {` group, the variant followed by `,` or
//!   `}` — in production that group is a construction ; (iii) in tests the same inside an
//!   `IntLit {` pattern (a pin reads a variant only as `matches!(<kind>, ExprKind::IntLit {
//!   origin: LitOrigin::X, .. })`). No leading path: elaborate takes one `use
//!   hdl_ast::LitOrigin;` in its `lib.rs`, every module sees it through `use super::*`. So
//!   no alias, field, parameter, closure parameter, return type, const, static, method,
//!   `impl`, glob, `==` / `assert_eq!` / `discriminant` operand of the type exists.
//! - H4 no read, no carry: inside every `IntLit {` group a field `origin` is followed by `:
//!   LitOrigin :: <V>` or `: _` (shorthand `origin`, a binding, fails) ; a production
//!   pattern never names a variant (a pattern is a group holding `..` or a `_` value, inside
//!   `matches!`, or followed — past `)` / `]` — by `=>`, `|`, `if` or `=`) ; `. origin` only
//!   on corpus-runner's `Workload.origin` (`DOT_ORIGIN_TABLE`).
//! - H5 no `IntLit {` construction outside `hdl-ast`, `hdl-parser` and `elaborate` ; no
//!   `IntLit` / `LitOrigin` token inside a `macro_rules!` body.
//!
//! Each rule has a planted negative control and a real site spelled back
//! (`controls_*`).
//!
//! The walker: each FILE is one token stream (multi-line ; `//` and `/* */` comments, doc
//! comments included, and the insides of string / char literals dropped) with brace
//! groups tracked ; a token is keyed by its innermost enclosing `fn` (`-` outside any) ;
//! test code = the item after `#[cfg(test)]` or `#[test]`, a file under a `#[cfg(test)] mod
//! <name>;` declaration, and `crates/*/tests/**`. Only files whose text holds `LitOrigin`,
//! `IntLit`, `origin`, `lit_int`, `range_rules` or `macro_rules` are tokenized: every rule
//! keys on a token that holds one of those strings.
//!
//! REVIEW-ONLY (no token shows it): derive / proc-macro expansion (`Deserialize` rebuilds
//! what was written, `Clone` copies it, `Debug` prints it) ; `#[cfg]` other than `test` ;
//! a closed `IntLit` pattern nested in a closed outer pattern with no `..` / `_` / `=>`
//! after it, which H4 takes for a construction (H2's per-(file, fn) count still moves) ;
//! two constructions swapped inside one (file, fn) ; and, by definition not claimed, WHERE
//! a `Source` literal sits: a clone carries it to a place where the user wrote a name
//! (`LitOrigin`'s doc).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const VARIANTS: [&str; 3] = ["Source", "Parser", "Elab"];
const DEF_FILE: &str = "crates/hdl-ast/src/lib.rs";
const RANGE_RULES: &str = "crates/hdl-ast/src/range_rules.rs";
const PART5_HEADER: &str = "// ─────────────────────────── 5. the parser's respell builders";
const PREFILTER: [&str; 6] = [
    "LitOrigin",
    "IntLit",
    "origin",
    "lit_int",
    "range_rules",
    "macro_rules",
];

/// H2: every production `LitOrigin :: <V>`, per (file, fn, variant).
const ORIGIN_TABLE: &[(&str, &str, &str, usize)] = &[
    (
        "crates/elaborate/src/block_local/proofs.rs",
        "subst_ident_zero",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/const_eval.rs",
        "array_param_init_pattern",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/cover_synth.rs",
        "cov_auto_sample_stmt",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/cover_synth.rs",
        "cov_int_lit_pos",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/dynarr.rs",
        "dyn_decl_init_stmts",
        "Elab",
        1,
    ),
    ("crates/elaborate/src/events.rs", "lower_delay", "Elab", 2),
    (
        "crates/elaborate/src/frames_body.rs",
        "emit_frame_local_inits",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/frames_call/emit.rs",
        "emit_frame_task_call",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/instance_array.rs",
        "elaborate_instance_array",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/packed_pattern.rs",
        "packed_default_rhs",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/stmt_main.rs",
        "lower_stmt_inner",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/string_array_route.rs",
        "indexed_lvalue",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/string_array_route.rs",
        "route_fixed_string_array",
        "Elab",
        1,
    ),
    (
        "crates/elaborate/src/struct_arm.rs",
        "pattern_concat",
        "Elab",
        1,
    ),
    ("crates/elaborate/src/sva_ast.rs", "sva_bit0", "Elab", 1),
    ("crates/elaborate/src/sva_ast.rs", "sva_one", "Elab", 1),
    ("crates/elaborate/src/sva_ast.rs", "sva_zero", "Elab", 1),
    ("crates/hdl-ast/src/range_rules.rs", "dec_lit", "Parser", 1),
    (
        "crates/hdl-parser/src/assertions.rs",
        "synth_bit_range",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/assign_pattern.rs",
        "fill_at_width",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/casts.rs",
        "parse_size_or_named_cast",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/classes.rs",
        "parse_dist",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/expr_primary.rs",
        "i64_lit",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/expr_primary.rs",
        "lit_int",
        "Source",
        1,
    ),
    (
        "crates/hdl-parser/src/gates.rs",
        "gate_desugar",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/generate.rs",
        "gen_step_assign",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/stmt.rs",
        "parse_pre_incdec",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/stmt.rs",
        "try_compound_assign",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/stmt_ctl.rs",
        "parse_inside",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/struct_sel.rs",
        "parse_record_array_member",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/sva_seq.rs",
        "sva_true_lit",
        "Parser",
        1,
    ),
    (
        "crates/hdl-parser/src/udp.rs",
        "parse_udp_decl",
        "Parser",
        2,
    ),
    (
        "crates/hdl-parser/src/udp_table.rs",
        "build_comb_udp",
        "Parser",
        3,
    ),
];

/// H2 funnel: the `range_rules :: <part-5 name>` paths in production, per (file, name).
const FUNNEL_TABLE: &[(&str, &str, usize)] = &[
    ("crates/hdl-parser/src/const_fold.rs", "lit_u32", 1),
    ("crates/hdl-parser/src/expr_primary.rs", "dec_lit", 1),
    ("crates/hdl-parser/src/packed_md.rs", "add", 1),
    ("crates/hdl-parser/src/packed_md.rs", "md_by_dir", 1),
    ("crates/hdl-parser/src/packed_md.rs", "md_elem_off", 1),
    ("crates/hdl-parser/src/packed_md.rs", "md_flat_range", 1),
    ("crates/hdl-parser/src/packed_md.rs", "md_query", 1),
    ("crates/hdl-parser/src/packed_md.rs", "md_stride", 1),
    ("crates/hdl-parser/src/packed_md.rs", "mul", 1),
    ("crates/hdl-parser/src/packed_md.rs", "sub", 1),
    (
        "crates/hdl-parser/src/sym_struct.rs",
        "sym_member_default_width",
        1,
    ),
    ("crates/hdl-parser/src/sym_struct.rs", "sym_member_width", 1),
    ("crates/hdl-parser/src/sym_struct.rs", "sym_sub_one", 1),
    ("crates/hdl-parser/src/sym_struct.rs", "sym_sum_of", 1),
    (
        "crates/hdl-parser/src/type_params.rs",
        "sym_typedef_width",
        1,
    ),
];

/// H4: `. origin` per file — all corpus-runner's `Workload.origin`, another struct.
const DOT_ORIGIN_TABLE: &[(&str, usize)] = &[
    ("crates/corpus-runner/src/corpus.rs", 1),
    ("crates/corpus-runner/src/fetch.rs", 2),
    ("crates/corpus-runner/src/main.rs", 1),
    ("crates/corpus-runner/tests/manifest.rs", 5),
];

// ───────────────────────────────────── the walker ─────────────────────────────────────

#[derive(Clone, Debug)]
struct Tok {
    text: String,
    /// Byte offset in the file.
    off: usize,
    /// Innermost enclosing fn (`-` outside any).
    func: String,
    test: bool,
    /// Index of the innermost enclosing group's opening token.
    parent: Option<usize>,
}

#[derive(Clone, Debug)]
struct FileA {
    rel: String,
    src: String,
    test_file: bool,
    toks: Vec<Tok>,
    /// Opening-token index -> closing-token index.
    close: BTreeMap<usize, usize>,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

/// After an opening `"` at `i`, the index past the closing `"`.
fn skip_str(b: &[u8], mut i: usize) -> usize {
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// The tokens of one file: identifiers / numbers, single punctuation and the pairs
/// `:: => == != .. -> <= >= && ||` ; a string or char literal is one token `""` / `''`.
fn lex(src: &str) -> Vec<(String, usize)> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            let mut d = 0usize;
            while i < b.len() {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    d += 1;
                    i += 2;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    d -= 1;
                    i += 2;
                    if d == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if c == b'"' {
            out.push(("\"\"".to_string(), i));
            i = skip_str(b, i + 1);
        } else if c == b'\'' {
            if b.get(i + 1) == Some(&b'\\') {
                let mut j = i + 3;
                while j < b.len() && b[j] != b'\'' {
                    j += 1;
                }
                out.push(("''".to_string(), i));
                i = j + 1;
            } else {
                // one char (possibly multi-byte) then `'` is a char literal ; else a lifetime
                let mut j = i + 1;
                if j < b.len() {
                    j += 1;
                    while j < b.len() && (b[j] & 0xC0) == 0x80 {
                        j += 1;
                    }
                }
                if b.get(j) == Some(&b'\'') {
                    out.push(("''".to_string(), i));
                    i = j + 1;
                } else {
                    let mut k = i + 1;
                    while k < b.len() && is_word(b[k]) {
                        k += 1;
                    }
                    out.push((src[i..k].to_string(), i));
                    i = k;
                }
            }
        } else if is_word(c) {
            let mut j = i;
            while j < b.len() && is_word(b[j]) {
                j += 1;
            }
            let w = &src[i..j];
            if matches!(w, "r" | "br" | "cr") && matches!(b.get(j), Some(b'"') | Some(b'#')) {
                let mut h = 0;
                let mut k = j;
                while b.get(k) == Some(&b'#') {
                    h += 1;
                    k += 1;
                }
                if b.get(k) == Some(&b'"') {
                    let mut e = k + 1;
                    loop {
                        if e >= b.len() {
                            break;
                        }
                        if b[e] == b'"' && (0..h).all(|x| b.get(e + 1 + x) == Some(&b'#')) {
                            e += 1 + h;
                            break;
                        }
                        e += 1;
                    }
                    out.push(("\"\"".to_string(), i));
                    i = e;
                    continue;
                }
                // `r#ident`: a raw identifier
                let mut e = k;
                while e < b.len() && is_word(b[e]) {
                    e += 1;
                }
                out.push((src[k..e].to_string(), i));
                i = e;
                continue;
            }
            if matches!(w, "b" | "c") && b.get(j) == Some(&b'"') {
                out.push(("\"\"".to_string(), i));
                i = skip_str(b, j + 1);
                continue;
            }
            out.push((w.to_string(), i));
            i = j;
        } else {
            let two = if i + 1 < b.len() { &src[i..i + 2] } else { "" };
            if ["::", "=>", "==", "!=", "..", "->", "<=", ">=", "&&", "||"].contains(&two) {
                out.push((two.to_string(), i));
                i += 2;
            } else {
                let mut j = i + 1;
                while j < b.len() && (b[j] & 0xC0) == 0x80 {
                    j += 1;
                }
                out.push((src[i..j].to_string(), i));
                i = j;
            }
        }
    }
    out
}

fn is_ident(s: &str) -> bool {
    s.bytes()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
}

/// `# [ cfg ( test ) ]` or `# [ test ]` starting at `i`: the index past `]`.
fn test_attr(t: &[(String, usize)], i: usize) -> Option<usize> {
    let w = |k: usize| t.get(i + k).map(|x| x.0.as_str());
    if w(0) != Some("#") || w(1) != Some("[") {
        return None;
    }
    if w(2) == Some("test") && w(3) == Some("]") {
        return Some(i + 4);
    }
    if w(2) == Some("cfg")
        && w(3) == Some("(")
        && w(4) == Some("test")
        && w(5) == Some(")")
        && w(6) == Some("]")
    {
        return Some(i + 7);
    }
    None
}

/// The `#[cfg(test)] mod <name>;` declarations of a file: the module path prefixes they make
/// test code (`<dir>/<name>.rs`, `<dir>/<name>/`).
fn test_mods(rel: &str, t: &[(String, usize)]) -> Vec<String> {
    let mut out = Vec::new();
    let (dir, stem) = match rel.rsplit_once('/') {
        Some((d, f)) => (d, f.trim_end_matches(".rs")),
        None => ("", rel.trim_end_matches(".rs")),
    };
    let base = if matches!(stem, "lib" | "main" | "mod") {
        dir.to_string()
    } else {
        format!("{dir}/{stem}")
    };
    let mut i = 0;
    while i < t.len() {
        if let Some(mut j) = test_attr(t, i) {
            if t.get(j).is_some_and(|x| x.0 == "pub") {
                j += 1;
                if t.get(j).is_some_and(|x| x.0 == "(") {
                    while j < t.len() && t[j].0 != ")" {
                        j += 1;
                    }
                    j += 1;
                }
            }
            if t.get(j).is_some_and(|x| x.0 == "mod") && t.get(j + 2).is_some_and(|x| x.0 == ";") {
                let n = &t[j + 1].0;
                out.push(format!("{base}/{n}.rs"));
                out.push(format!("{base}/{n}/"));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn analyze(rel: &str, src: &str, test_file: bool) -> FileA {
    let lx = lex(src);
    let mut toks: Vec<Tok> = Vec::with_capacity(lx.len());
    let mut close = BTreeMap::new();
    // (open index, func, test)
    let mut stack: Vec<(usize, String, bool)> = Vec::new();
    let mut pending_fn: Option<(String, usize)> = None;
    let mut pending_test: Option<usize> = None;
    for (i, (text, off)) in lx.iter().enumerate() {
        let depth = stack.len();
        let (func, test) = stack
            .last()
            .map(|(_, f, t)| (f.clone(), *t))
            .unwrap_or_else(|| ("-".to_string(), test_file));
        let parent = stack.last().map(|x| x.0);
        if test_attr(&lx, i).is_some() {
            pending_test = Some(depth);
        }
        if text == "fn" && lx.get(i + 1).is_some_and(|x| is_ident(&x.0)) {
            pending_fn = Some((lx[i + 1].0.clone(), depth));
        }
        match text.as_str() {
            "{" | "(" | "[" => {
                let mut f = func.clone();
                let mut tt = test;
                if text == "{" {
                    if let Some((name, d)) = &pending_fn {
                        if *d == depth {
                            f = name.clone();
                            pending_fn = None;
                        }
                    }
                    if pending_test == Some(depth) {
                        tt = true;
                        pending_test = None;
                    }
                }
                toks.push(Tok {
                    text: text.clone(),
                    off: *off,
                    func,
                    test,
                    parent,
                });
                stack.push((i, f, tt));
                continue;
            }
            "}" | ")" | "]" => {
                if let Some((o, _, _)) = stack.pop() {
                    close.insert(o, i);
                }
                let (func, test) = stack
                    .last()
                    .map(|(_, f, t)| (f.clone(), *t))
                    .unwrap_or_else(|| ("-".to_string(), test_file));
                toks.push(Tok {
                    text: text.clone(),
                    off: *off,
                    func,
                    test,
                    parent: stack.last().map(|x| x.0),
                });
                continue;
            }
            ";" => {
                if pending_fn.as_ref().is_some_and(|(_, d)| *d == depth) {
                    pending_fn = None;
                }
                if pending_test == Some(depth) {
                    pending_test = None;
                }
            }
            _ => {}
        }
        toks.push(Tok {
            text: text.clone(),
            off: *off,
            func,
            test,
            parent,
        });
    }
    FileA {
        rel: rel.to_string(),
        src: src.to_string(),
        test_file,
        toks,
        close,
    }
}

/// Every prefiltered file of `crates/*/src` and `crates/*/tests`, analysed once.
fn tree() -> &'static Vec<FileA> {
    static T: OnceLock<Vec<FileA>> = OnceLock::new();
    T.get_or_init(|| {
        let root = workspace_root();
        let mut paths = Vec::new();
        let mut crates: Vec<_> = std::fs::read_dir(root.join("crates"))
            .expect("crates/")
            .flatten()
            .map(|e| e.path())
            .collect();
        crates.sort();
        for c in crates {
            rs_files(&c.join("src"), &mut paths);
            rs_files(&c.join("tests"), &mut paths);
        }
        let mut raw = Vec::new();
        for p in paths {
            let rel = p
                .strip_prefix(&root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&p).expect("readable source");
            raw.push((rel, src));
        }
        build(raw)
    })
}

/// Analyse a file set (the tree, or a control's copy): test files by path and by `#[cfg(test)]
/// mod` declarations, then each prefiltered file.
fn build(raw: Vec<(String, String)>) -> Vec<FileA> {
    let mut prefixes = Vec::new();
    for (rel, src) in &raw {
        if src.contains("cfg(test)") {
            prefixes.extend(test_mods(rel, &lex(src)));
        }
    }
    raw.into_iter()
        .filter(|(_, src)| PREFILTER.iter().any(|p| src.contains(p)))
        .map(|(rel, src)| {
            let in_tests_dir = rel.split('/').nth(2) == Some("tests");
            let test_file = in_tests_dir
                || prefixes
                    .iter()
                    .any(|p| rel == *p || (p.ends_with('/') && rel.starts_with(p.as_str())));
            analyze(&rel, &src, test_file)
        })
        .collect()
}

/// The tree with `rel`'s text replaced by `edit(text)` (a real site spelled back), or with a
/// planted file added when `rel` is not in the tree.
fn with_file(rel: &str, src: Option<&str>, edit: impl Fn(&str) -> String) -> Vec<FileA> {
    let mut out: Vec<FileA> = Vec::new();
    let mut found = false;
    for f in tree() {
        if f.rel == rel {
            found = true;
            out.push(analyze(rel, &edit(&f.src), f.test_file));
        } else {
            out.push(f.clone());
        }
    }
    if !found {
        let s = src.expect("a planted file has a text");
        let test_file = rel.split('/').nth(2) == Some("tests");
        out.push(analyze(rel, &edit(s), test_file));
    }
    out
}

/// Replace exactly one occurrence of `from` (a control that matches nothing is a broken control).
fn once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "control anchor `{from}` must occur once"
    );
    text.replacen(from, to, 1)
}

fn w(f: &FileA, i: usize) -> &str {
    f.toks.get(i).map(|t| t.text.as_str()).unwrap_or("")
}

fn production(f: &FileA, i: usize) -> bool {
    !f.toks[i].test
}

/// The innermost enclosing group of token `i` is an `IntLit {` group: its opening index.
fn intlit_group(f: &FileA, i: usize) -> Option<usize> {
    let o = f.toks[i].parent?;
    (w(f, o) == "{" && o > 0 && w(f, o - 1) == "IntLit").then_some(o)
}

/// Inside a `matches!( .. )` call, at any depth.
fn in_matches(f: &FileA, i: usize) -> bool {
    let mut p = f.toks[i].parent;
    while let Some(o) = p {
        if w(f, o) == "(" && o >= 2 && w(f, o - 1) == "!" && w(f, o - 2) == "matches" {
            return true;
        }
        p = f.toks[o].parent;
    }
    false
}

/// An `IntLit {` group opened at `o` is a pattern (H4's classes) rather than a construction.
fn is_pattern(f: &FileA, o: usize) -> bool {
    let c = f.close[&o];
    let direct = |k: usize| f.toks[k].parent == Some(o);
    if (o + 1..c).any(|k| direct(k) && (w(f, k) == ".." || (w(f, k) == "_" && w(f, k - 1) == ":")))
    {
        return true;
    }
    if in_matches(f, o) {
        return true;
    }
    let mut k = c + 1;
    while matches!(w(f, k), ")" | "]") {
        k += 1;
    }
    matches!(w(f, k), "=>" | "|" | "if" | "=")
}

/// Inside the item that starts at the first token whose text sequence is `head` (e.g. `enum
/// LitOrigin`), through its closing brace: (start, end) token indices.
fn item_span(f: &FileA, head: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for i in 0..f.toks.len() {
        if head.iter().enumerate().all(|(k, h)| w(f, i + k) == *h) {
            let mut j = i + head.len();
            while j < f.toks.len() && w(f, j) != "{" && w(f, j) != ";" {
                j += 1;
            }
            if w(f, j) == "{" {
                out.push((i, f.close[&j]));
            }
        }
    }
    out
}

// ───────────────────────────────────── the rules ─────────────────────────────────────

fn h2_rows(files: &[FileA]) -> BTreeMap<(String, String, String), usize> {
    let mut rows = BTreeMap::new();
    for f in files {
        for i in 0..f.toks.len() {
            if w(f, i) == "LitOrigin" && w(f, i + 1) == "::" && production(f, i) {
                *rows
                    .entry((
                        f.rel.clone(),
                        f.toks[i].func.clone(),
                        w(f, i + 2).to_string(),
                    ))
                    .or_insert(0) += 1;
            }
        }
    }
    rows
}

/// The variant a production construction at (file, fn) must name, by path.
fn path_variant(rel: &str, func: &str) -> Option<&'static str> {
    if rel == "crates/hdl-parser/src/expr_primary.rs" && func == "lit_int" {
        Some("Source")
    } else if rel.starts_with("crates/hdl-parser/src/") || (rel == RANGE_RULES && func == "dec_lit")
    {
        Some("Parser")
    } else if rel.starts_with("crates/elaborate/src/") {
        Some("Elab")
    } else {
        None
    }
}

fn h2(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    let rows = h2_rows(files);
    let want: BTreeMap<(String, String, String), usize> = ORIGIN_TABLE
        .iter()
        .map(|(f, n, v, c)| ((f.to_string(), n.to_string(), v.to_string()), *c))
        .collect();
    if rows != want {
        bad.push(format!("H2 table moved: measured {rows:#?}"));
    }
    for (rel, func, v) in rows.keys() {
        if path_variant(rel, func) != Some(v.as_str()) {
            bad.push(format!(
                "H2 path rule: {rel} {func} names {v}, the path says {:?}",
                path_variant(rel, func)
            ));
        }
    }
    let mut tot = BTreeMap::new();
    for ((_, _, v), c) in &rows {
        *tot.entry(v.as_str()).or_insert(0usize) += c;
    }
    let tot: Vec<usize> = VARIANTS
        .iter()
        .map(|v| tot.get(v).copied().unwrap_or(0))
        .collect();
    if tot != [1, 18, 18] {
        bad.push(format!(
            "H2 totals Source / Parser / Elab = {tot:?}, want [1, 18, 18]"
        ));
    }
    bad
}

/// The part-5 builder names, read from `range_rules.rs`: every `pub fn` after the header.
fn part5(files: &[FileA]) -> (BTreeSet<String>, usize) {
    let f = files
        .iter()
        .find(|f| f.rel == RANGE_RULES)
        .expect("range_rules.rs");
    let at = f.src.find(PART5_HEADER).expect("the part-5 header");
    let mut names = BTreeSet::new();
    for i in 0..f.toks.len() {
        if f.toks[i].off > at && w(f, i) == "pub" && w(f, i + 1) == "fn" {
            names.insert(w(f, i + 2).to_string());
        }
    }
    (names, at)
}

fn funnel(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    let (names, at) = part5(files);
    if names.len() != 16 {
        bad.push(format!(
            "H2 funnel: {} part-5 builders, want 16: {names:?}",
            names.len()
        ));
    }
    let mut rows: BTreeMap<(String, String), usize> = BTreeMap::new();
    for f in files {
        for i in 0..f.toks.len() {
            if !production(f, i) {
                continue;
            }
            let part5_here = f.rel == RANGE_RULES && f.toks[i].off > at;
            if w(f, i) == "range_rules" {
                let next = w(f, i + 1);
                let name = w(f, i + 2);
                if next == "as" || (next == "::" && name == "*") {
                    bad.push(format!(
                        "H2 funnel: `range_rules {next} {name}` in {} {}",
                        f.rel, f.toks[i].func
                    ));
                }
                if next == "::" && names.contains(name) && !part5_here {
                    if f.rel.starts_with("crates/hdl-parser/src/") {
                        *rows.entry((f.rel.clone(), name.to_string())).or_insert(0) += 1;
                    } else {
                        bad.push(format!(
                            "H2 funnel: `range_rules :: {name}` in {} {}",
                            f.rel, f.toks[i].func
                        ));
                    }
                }
                // a use-tree `range_rules :: { .. <name> .. }`
                if next == "::" && name == "{" {
                    let c = f.close[&(i + 2)];
                    for k in i + 3..c {
                        if names.contains(w(f, k)) && !f.rel.starts_with("crates/hdl-parser/src/") {
                            bad.push(format!(
                                "H2 funnel: use-tree names `{}` in {}",
                                w(f, k),
                                f.rel
                            ));
                        }
                    }
                }
            }
            // a parts-1-4 fn calling a part-5 builder by its bare name
            if f.rel == RANGE_RULES
                && f.toks[i].off < at
                && names.contains(w(f, i))
                && w(f, i + 1) == "("
                && i > 0
                && !matches!(w(f, i - 1), "fn" | "." | "::")
            {
                bad.push(format!(
                    "H2 funnel: {} calls part-5 `{}` before the header",
                    f.toks[i].func,
                    w(f, i)
                ));
            }
        }
    }
    let want: BTreeMap<(String, String), usize> = FUNNEL_TABLE
        .iter()
        .map(|(f, n, c)| ((f.to_string(), n.to_string()), *c))
        .collect();
    if rows != want {
        bad.push(format!("H2 funnel table moved: measured {rows:#?}"));
    }
    bad
}

fn h0(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    let mut enums = 0;
    let mut impls = Vec::new();
    for f in files {
        for i in 0..f.toks.len() {
            if w(f, i) == "enum" && w(f, i + 1) == "LitOrigin" {
                enums += 1;
            }
            if w(f, i) == "for" && w(f, i + 1) == "LitOrigin" {
                // walk back to the `impl`
                let mut k = i;
                while k > 0 && w(f, k) != "impl" {
                    k -= 1;
                }
                let tr: Vec<&str> = (k + 1..i).map(|x| w(f, x)).collect();
                impls.push((f.rel.clone(), tr.join(" "), k));
            }
        }
    }
    if enums != 1 {
        bad.push(format!("H0 `enum LitOrigin` {enums} times"));
    }
    let def = files
        .iter()
        .find(|f| f.rel == DEF_FILE)
        .expect("hdl-ast lib.rs");
    let spans = item_span(def, &["pub", "enum", "LitOrigin"]);
    if spans.len() != 1 {
        bad.push("H0 `pub enum LitOrigin` not found once in hdl-ast lib.rs".into());
        return bad;
    }
    let (s, e) = spans[0];
    let body: Vec<&str> = (s + 3..=e).map(|k| w(def, k)).collect();
    if body != ["{", "Source", ",", "Parser", ",", "Elab", ",", "}"] {
        bad.push(format!("H0 enum body {body:?}"));
    }
    // the one attribute right before `pub enum LitOrigin`
    let mut k = s;
    let mut attrs = Vec::new();
    while k >= 1 && w(def, k - 1) == "]" {
        let mut d = 0;
        let mut j = k - 1;
        loop {
            match w(def, j) {
                "]" => d += 1,
                "[" => {
                    d -= 1;
                    if d == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j -= 1;
        }
        let a: Vec<&str> = (j - 1..k).map(|x| w(def, x)).collect();
        attrs.push(a.join(" "));
        k = j - 1;
    }
    if attrs
        != ["# [ derive ( Debug , Clone , Copy , Eq , Serialize , Deserialize , SchemaHash ) ]"]
    {
        bad.push(format!("H0 attributes {attrs:?}"));
    }
    if impls.len() != 1 || impls[0].0 != DEF_FILE || impls[0].1 != "PartialEq" {
        bad.push(format!("H0 impls for LitOrigin {impls:?}"));
    } else {
        let k = impls[0].2;
        let o = k + 4;
        let body: Vec<&str> = (o..=def.close[&o]).map(|x| w(def, x)).collect();
        if body.join(" ") != "{ fn eq ( & self , _other : & Self ) -> bool { true } }" {
            bad.push(format!("H0 PartialEq body {body:?}"));
        }
    }
    // IntLit's declaration inside `enum ExprKind`
    let ek = item_span(def, &["pub", "enum", "ExprKind"]);
    let fields = ek.first().and_then(|&(s, e)| {
        (s..e)
            .find(|&i| w(def, i) == "IntLit" && w(def, i + 1) == "{")
            .map(|i| {
                let o = i + 1;
                (o..=def.close[&o])
                    .map(|x| w(def, x))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
    });
    if fields.as_deref() != Some("{ kind : IntLitKind , raw : String , origin : LitOrigin , }") {
        bad.push(format!("H0 IntLit fields {fields:?}"));
    }
    bad
}

/// H3's exempt tokens: the enum item, the PartialEq impl, IntLit's field declaration.
fn exempt(f: &FileA, i: usize) -> bool {
    if f.rel != DEF_FILE {
        return false;
    }
    let inside = |head: &[&str]| item_span(f, head).iter().any(|&(s, e)| s <= i && i <= e);
    if inside(&["pub", "enum", "LitOrigin"]) || inside(&["impl", "PartialEq", "for", "LitOrigin"]) {
        return true;
    }
    inside(&["pub", "enum", "ExprKind"])
        && intlit_group(f, i).is_some()
        && w(f, i - 1) == ":"
        && w(f, i - 2) == "origin"
        && matches!(w(f, i + 1), "," | "}")
}

/// A `use` item holding `LitOrigin` at `i`, of the admitted shape.
fn use_form(f: &FileA, i: usize) -> bool {
    let mut k = i;
    while k > 0 && !matches!(w(f, k), "use" | ";" | "{" | "}")
        || (k > 0 && w(f, k) == "{" && w(f, k - 1) == "::")
        || (k > 0 && w(f, k) == "}" && k != i)
    {
        k -= 1;
        if w(f, k) == "use" {
            break;
        }
    }
    if w(f, k) != "use" {
        return false;
    }
    if k > 0 && (w(f, k - 1) == "pub" || w(f, k - 1) == ")") {
        return false;
    }
    let root = w(f, k + 1);
    let root_ok =
        root == "hdl_ast" || (root == "crate" && f.rel.starts_with("crates/hdl-ast/src/"));
    root_ok && matches!(w(f, i + 1), ";" | "," | "}") && matches!(w(f, i - 1), "::" | "," | "{")
}

fn h3(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    for f in files {
        for i in 0..f.toks.len() {
            if w(f, i) != "LitOrigin" || exempt(f, i) || use_form(f, i) {
                continue;
            }
            let origin_form = i >= 2
                && w(f, i - 1) == ":"
                && w(f, i - 2) == "origin"
                && w(f, i + 1) == "::"
                && VARIANTS.contains(&w(f, i + 2))
                && matches!(w(f, i + 3), "," | "}")
                && intlit_group(f, i).is_some_and(|o| !production(f, i) || !is_pattern(f, o));
            if !origin_form {
                bad.push(format!(
                    "H3 `LitOrigin` outside the positive set: {} {} `{} {} LitOrigin {} {}`",
                    f.rel,
                    f.toks[i].func,
                    w(f, i.saturating_sub(2)),
                    w(f, i.saturating_sub(1)),
                    w(f, i + 1),
                    w(f, i + 2)
                ));
            }
        }
    }
    bad
}

fn h4(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    let mut dots: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        for i in 0..f.toks.len() {
            if w(f, i) == "origin" && i > 0 && w(f, i - 1) == "." {
                *dots.entry(f.rel.clone()).or_insert(0) += 1;
            }
            if w(f, i) != "origin" || exempt_field(f, i) {
                continue;
            }
            let Some(o) = intlit_group(f, i) else {
                continue;
            };
            if !matches!(w(f, i - 1), "{" | ",") {
                continue;
            }
            let ok_tag = w(f, i + 1) == ":"
                && w(f, i + 2) == "LitOrigin"
                && w(f, i + 3) == "::"
                && VARIANTS.contains(&w(f, i + 4));
            let ok_wild = w(f, i + 1) == ":" && w(f, i + 2) == "_";
            if !(ok_tag || ok_wild) {
                bad.push(format!(
                    "H4 `origin` in an IntLit group is neither a tag nor `_`: {} {}",
                    f.rel, f.toks[i].func
                ));
            } else if ok_tag && production(f, i) && is_pattern(f, o) {
                bad.push(format!(
                    "H4 a production pattern names a variant: {} {}",
                    f.rel, f.toks[i].func
                ));
            }
        }
    }
    let want: BTreeMap<String, usize> = DOT_ORIGIN_TABLE
        .iter()
        .map(|(f, c)| (f.to_string(), *c))
        .collect();
    if dots != want {
        bad.push(format!("H4 `. origin` table moved: measured {dots:#?}"));
    }
    bad
}

/// IntLit's field declaration in `enum ExprKind` (H0's).
fn exempt_field(f: &FileA, i: usize) -> bool {
    f.rel == DEF_FILE
        && w(f, i + 1) == ":"
        && w(f, i + 2) == "LitOrigin"
        && matches!(w(f, i + 3), "," | "}")
}

fn h5(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    for f in files {
        let home = ["crates/hdl-ast/", "crates/hdl-parser/", "crates/elaborate/"]
            .iter()
            .any(|p| f.rel.starts_with(p));
        for i in 0..f.toks.len() {
            if !home && w(f, i) == "IntLit" && w(f, i + 1) == "{" && !is_pattern(f, i + 1) {
                bad.push(format!(
                    "H5 an IntLit construction in {} {}",
                    f.rel, f.toks[i].func
                ));
            }
            if w(f, i) == "macro_rules" && w(f, i + 1) == "!" {
                let mut o = i + 2;
                while o < f.toks.len() && !matches!(w(f, o), "{" | "(" | "[") {
                    o += 1;
                }
                if let Some(&c) = f.close.get(&o) {
                    if (o..c).any(|k| matches!(w(f, k), "IntLit" | "LitOrigin")) {
                        bad.push(format!(
                            "H5 IntLit / LitOrigin in a macro_rules! body: {}",
                            f.rel
                        ));
                    }
                }
            }
        }
    }
    bad
}

fn h1(files: &[FileA]) -> Vec<String> {
    let mut bad = Vec::new();
    let mut src_rows = Vec::new();
    let mut calls = BTreeMap::new();
    let mut defs = 0;
    for f in files {
        for i in 0..f.toks.len() {
            if w(f, i) == "LitOrigin"
                && w(f, i + 1) == "::"
                && w(f, i + 2) == "Source"
                && production(f, i)
            {
                src_rows.push((f.rel.clone(), f.toks[i].func.clone()));
            }
            if w(f, i) == "lit_int" && production(f, i) {
                if w(f, i - 1) == "fn" {
                    defs += 1;
                } else {
                    *calls
                        .entry(format!("{} {} {}", f.rel, f.toks[i].func, w(f, i - 1)))
                        .or_insert(0usize) += 1;
                }
            }
        }
    }
    if src_rows
        != [(
            "crates/hdl-parser/src/expr_primary.rs".to_string(),
            "lit_int".to_string(),
        )]
    {
        bad.push(format!(
            "H1 production `LitOrigin::Source` sites {src_rows:?}"
        ));
    }
    let want: BTreeMap<String, usize> = [(
        "crates/hdl-parser/src/expr_primary.rs expr_primary .".to_string(),
        3,
    )]
    .into();
    if defs != 1 || calls != want {
        bad.push(format!("H1 lit_int: {defs} definitions, calls {calls:?}"));
    }
    // lit_int's raw: the local bound from the token's text, given to the construction
    let f = files
        .iter()
        .find(|f| f.rel == "crates/hdl-parser/src/expr_primary.rs")
        .expect("expr_primary.rs");
    let body: Vec<usize> = (0..f.toks.len())
        .filter(|&i| f.toks[i].func == "lit_int")
        .collect();
    let text: Vec<&str> = body.iter().map(|&i| w(f, i)).collect();
    let bind = [
        "let",
        "raw",
        "=",
        "self",
        ".",
        "cur_text",
        "(",
        ")",
        ".",
        "to_string",
        "(",
        ")",
        ";",
    ];
    let binds = text.windows(bind.len()).filter(|x| *x == bind).count();
    let raws = text.iter().filter(|x| **x == "raw").count();
    let short = body
        .iter()
        .filter(|&&i| {
            w(f, i) == "raw" && intlit_group(f, i).is_some() && matches!(w(f, i + 1), "," | "}")
        })
        .count();
    if binds != 1 || raws != 2 || short != 1 {
        bad.push(format!(
            "H1 lit_int's raw: {binds} bindings, {raws} `raw` tokens, {short} shorthand fields"
        ));
    }
    bad
}

fn all_rules(files: &[FileA]) -> Vec<String> {
    let mut v = h0(files);
    v.extend(h1(files));
    v.extend(h2(files));
    v.extend(funnel(files));
    v.extend(h3(files));
    v.extend(h4(files));
    v.extend(h5(files));
    v
}

// ───────────────────────────────────── the tests ─────────────────────────────────────

#[test]
fn h0_the_definition() {
    let v = h0(tree());
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn h1_source_is_lit_int_only() {
    let v = h1(tree());
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn h2_each_tag_is_its_path() {
    let mut v = h2(tree());
    v.extend(funnel(tree()));
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn h3_every_lit_origin_token_is_in_the_positive_set() {
    let v = h3(tree());
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn h4_no_read_no_carry() {
    let v = h4(tree());
    assert!(v.is_empty(), "{v:#?}");
}

#[test]
fn h5_no_construction_outside_the_three_crates() {
    let v = h5(tree());
    assert!(v.is_empty(), "{v:#?}");
}

/// A control: the rule `rule` fails on `files` with a message holding `frag`.
#[track_caller]
fn fails(rule: fn(&[FileA]) -> Vec<String>, files: &[FileA], frag: &str) {
    let v = rule(files);
    assert!(
        v.iter().any(|m| m.contains(frag)),
        "control not seen (want `{frag}`): {v:#?}"
    );
}

const PLANT_E: &str = "crates/elaborate/src/zz_planted.rs";
const PLANT_P: &str = "crates/hdl-parser/src/zz_planted.rs";

fn plant(rel: &str, text: &str) -> Vec<FileA> {
    with_file(rel, Some(text), |s| s.to_string())
}

#[test]
fn controls_h0() {
    let def = |from: &str, to: &str| with_file(DEF_FILE, None, |s| once(s, from, to));
    // `#[default]` on a variant (m8)
    fails(
        h0,
        &def(
            "pub enum LitOrigin {\n    Source,",
            "pub enum LitOrigin {\n    #[default]\n    Source,",
        ),
        "H0 enum body",
    );
    // a derived Hash, a derived PartialEq with the hand impl gone (m9)
    fails(
        h0,
        &def(
            "Copy, Eq, Serialize, Deserialize, SchemaHash)]\npub enum LitOrigin",
            "Copy, Eq, Hash, Serialize, Deserialize, SchemaHash)]\npub enum LitOrigin",
        ),
        "H0 attributes",
    );
    fails(
        h0,
        &def(
            "impl PartialEq for LitOrigin {\n    fn eq(&self, _other: &Self) -> bool {\n        true\n    }\n}",
            "impl LitOrigin {}",
        ),
        "H0 impls",
    );
    // the compare made real
    fails(h0, &def("fn eq(&self, _other: &Self) -> bool {\n        true", "fn eq(&self, other: &Self) -> bool {\n        std::mem::discriminant(self) == std::mem::discriminant(other)"), "H0 PartialEq body");
    // a hand `Default` elsewhere ; a serde attribute on the field (m11) ; origin not last (m10)
    fails(
        h0,
        &plant(
            "crates/hdl-ast/src/zz_planted.rs",
            "impl Default for LitOrigin { fn default() -> Self { todo!() } }",
        ),
        "H0 impls",
    );
    fails(
        h0,
        &def(
            "        origin: LitOrigin,\n    },\n    RealLit",
            "        #[serde(skip)]\n        origin: LitOrigin,\n    },\n    RealLit",
        ),
        "H0 IntLit fields",
    );
    fails(
        h0,
        &def(
            "        kind: IntLitKind,\n        raw: String,\n",
            "        raw: String,\n        kind: IntLitKind,\n",
        ),
        "H0 IntLit fields",
    );
}

#[test]
fn controls_h1() {
    let ep = "crates/hdl-parser/src/expr_primary.rs";
    // m1 lit_int writes Parser (a real site spelled back)
    fails(
        h1,
        &with_file(ep, None, |s| {
            once(
                s,
                "origin: LitOrigin::Source,",
                "origin: LitOrigin::Parser,",
            )
        }),
        "H1 production",
    );
    // m6 a new Source construction
    fails(h1, &plant(PLANT_P, "fn f() -> ExprKind { ExprKind::IntLit { kind: IntLitKind::Decimal, raw: String::new(), origin: LitOrigin::Source } }"), "H1 production");
    // lit_int's raw from another expression
    fails(
        h1,
        &with_file(ep, None, |s| {
            once(s, "                kind,\n                raw,\n                origin: LitOrigin::Source,", "                kind,\n                raw: raw.trim().to_string(),\n                origin: LitOrigin::Source,")
        }),
        "H1 lit_int's raw",
    );
    // a fourth call
    fails(
        h1,
        &plant(
            PLANT_P,
            "impl Parser { fn g(&mut self) { self.lit_int(IntLitKind::Sized); } }",
        ),
        "H1 lit_int",
    );
}

#[test]
fn controls_h2() {
    // m2: Parser::dec_lit builds its own IntLit with Source (H1 and H2 both see it)
    let ep = "crates/hdl-parser/src/expr_primary.rs";
    let m2 = with_file(ep, None, |s| {
        once(
            s,
            "        hdl_ast::range_rules::dec_lit(v, span)",
            "        Expr { kind: ExprKind::IntLit { kind: IntLitKind::Decimal, raw: v.to_string(), origin: LitOrigin::Source }, span }",
        )
    });
    fails(h2, &m2, "H2 path rule");
    fails(h1, &m2, "H1 production");
    // m3 i64_lit writes Elab ; m4 an elaborate site writes Parser ; m5 hdl-ast dec_lit writes Elab
    fails(
        h2,
        &with_file(ep, None, |s| {
            once(
                s,
                "raw: v.unsigned_abs().to_string(),\n                origin: LitOrigin::Parser,",
                "raw: v.unsigned_abs().to_string(),\n                origin: LitOrigin::Elab,",
            )
        }),
        "H2 path rule",
    );
    fails(
        h2,
        &with_file("crates/elaborate/src/dynarr.rs", None, |s| {
            once(s, "origin: LitOrigin::Elab,", "origin: LitOrigin::Parser,")
        }),
        "H2 path rule",
    );
    fails(
        h2,
        &with_file(RANGE_RULES, None, |s| {
            once(s, "origin: LitOrigin::Parser,", "origin: LitOrigin::Elab,")
        }),
        "H2 path rule",
    );
    // m14 restated: a Parser construction planted in elaborate, an Elab one in hdl-parser
    // (each with `origin:` on its own line: the walker is file-level)
    let lit = |v: &str| {
        format!("fn f() -> ExprKind {{\n    ExprKind::IntLit {{\n        kind: IntLitKind::Decimal,\n        raw: String::new(),\n        origin:\n            LitOrigin::{v},\n    }}\n}}\n")
    };
    fails(h2, &plant(PLANT_E, &lit("Parser")), "H2 path rule");
    fails(h2, &plant(PLANT_P, &lit("Elab")), "H2 path rule");
    // one more Elab construction at its path: the table moves
    fails(h2, &plant(PLANT_E, &lit("Elab")), "H2 table moved");
    // outside the three paths
    fails(
        h2,
        &plant("crates/sim-engine/src/zz_planted.rs", &lit("Elab")),
        "H2 path rule",
    );
}

#[test]
fn controls_funnel() {
    let lib = "crates/elaborate/src/lib.rs";
    // a path call ; a name added to elaborate's range_rules use-tree ; a glob ; a module alias
    fails(
        funnel,
        &plant(
            PLANT_E,
            "fn f() { let _ = hdl_ast::range_rules::sub(l, a, b, sp); }",
        ),
        "H2 funnel: `range_rules :: sub`",
    );
    fails(
        funnel,
        &with_file(lib, None, |s| {
            once(
                s,
                "range_rules::{DeclRange, RangeFacts, RangeUnfolded}",
                "range_rules::{DeclRange, RangeFacts, RangeUnfolded, md_query}",
            )
        }),
        "H2 funnel: use-tree",
    );
    fails(
        funnel,
        &plant(PLANT_E, "use hdl_ast::range_rules::*;"),
        "H2 funnel: `range_rules :: *`",
    );
    fails(
        funnel,
        &plant(PLANT_E, "use hdl_ast::range_rules as rr;"),
        "H2 funnel: `range_rules as",
    );
    // a parts-1-4 wrapper around a part-5 builder
    fails(
        funnel,
        &with_file(RANGE_RULES, None, |s| {
            once(s, "pub fn span_folded(", "pub fn wrap(l: LitFold, a: Expr, b: Expr, s: Span) -> Expr {\n    sub(l, a, b, s)\n}\npub fn span_folded(")
        }),
        "before the header",
    );
    // one more parser caller moves the table
    fails(
        funnel,
        &plant(
            PLANT_P,
            "fn f() { let _ = hdl_ast::range_rules::add(l, a, b, sp); }",
        ),
        "H2 funnel table moved",
    );
}

#[test]
fn controls_h3() {
    // A1's six: an `as` alias, `LitOrigin::f(`, a struct field, a fn parameter, a closure
    // parameter, a `const`
    for (src, frag) in [
        ("use hdl_ast::LitOrigin as O;", "positive set"),
        ("fn f() { let _ = LitOrigin::f(); }", "positive set"),
        ("struct W { o: LitOrigin }", "positive set"),
        ("fn f(o: LitOrigin) {}", "positive set"),
        ("fn f() { let g = |o: LitOrigin| o; }", "positive set"),
        ("const C: LitOrigin = LitOrigin::Source;", "positive set"),
        // premise judge 3 L6: a non-IntLit group, an `if`, an alias inside a use-group
        ("fn f() -> W { W { origin: LitOrigin::Elab } }", "positive set"),
        ("fn f() -> ExprKind { ExprKind::IntLit { kind: IntLitKind::Decimal, raw: String::new(), origin: if c { LitOrigin::Source } else { LitOrigin::Parser } } }", "positive set"),
        ("use hdl_ast::{LitOrigin as O};", "positive set"),
        // a return type, a pub import, a glob of the variants, a compare
        ("fn f() -> LitOrigin { todo!() }", "positive set"),
        ("pub use hdl_ast::LitOrigin;", "positive set"),
        ("use hdl_ast::LitOrigin::*;", "positive set"),
        ("fn f(e: &ExprKind) -> bool { matches!(e, ExprKind::IntLit { origin: o, .. } if o == &LitOrigin::Parser) }", "positive set"),
    ] {
        fails(h3, &plant(PLANT_E, src), frag);
    }
    // premise judge 3 L2: a production arm reading a variant
    fails(h3, &plant(PLANT_P, "fn f(e: &Expr) -> bool { match &e.kind { ExprKind::IntLit { origin: LitOrigin::Parser, .. } => true, _ => false } }"), "positive set");
    // tests: the L3 pin form passes, an `assert_eq!` operand does not
    let t = "crates/hdl-ast/tests/zz_planted.rs";
    assert!(h3(&plant(t, "fn f(e: &ExprKind) -> bool { matches!(e, ExprKind::IntLit { origin: LitOrigin::Parser, .. }) }")).is_empty());
    fails(
        h3,
        &plant(
            t,
            "fn f() { assert_eq!(LitOrigin::Source, LitOrigin::Parser); }",
        ),
        "positive set",
    );
    fails(
        h3,
        &plant(
            t,
            "fn f() { let _ = std::mem::discriminant(&LitOrigin::Parser); }",
        ),
        "positive set",
    );
    // a real site spelled back: elaborate's import made an alias
    fails(
        h3,
        &with_file("crates/elaborate/src/lib.rs", None, |s| {
            once(s, "use hdl_ast::LitOrigin;", "use hdl_ast::LitOrigin as O;")
        }),
        "positive set",
    );
}

#[test]
fn controls_h4() {
    // m7: a closed production pattern binds origin
    fails(
        h4,
        &with_file("crates/elaborate/src/expr_main.rs", None, |s| {
            let at = s.find("origin: _,").expect("a closed pattern");
            format!("{}origin,{}", &s[..at], &s[at + "origin: _,".len()..])
        }),
        "neither a tag nor `_`",
    );
    // a production pattern naming a variant, closed and followed by `=>`
    fails(h4, &plant(PLANT_P, "fn f(e: &ExprKind) -> u8 { match e { ExprKind::IntLit { kind, raw, origin: LitOrigin::Source } => 1, _ => 0 } }"), "a production pattern names a variant");
    // a read through a field
    fails(
        h4,
        &plant(PLANT_E, "fn f(x: &X) -> bool { x.origin.is_some() }"),
        "`. origin` table moved",
    );
}

#[test]
fn controls_h5() {
    fails(h5, &plant("crates/cli/src/zz_planted.rs", "fn f() -> ExprKind { ExprKind::IntLit { kind: IntLitKind::Decimal, raw: String::new(), origin: LitOrigin::Elab } }"), "H5 an IntLit construction");
    fails(h5, &plant(PLANT_E, "macro_rules! lit { ($v:expr) => { ast::ExprKind::IntLit { kind: ast::IntLitKind::Decimal, raw: $v, origin: LitOrigin::Elab } }; }"), "macro_rules");
    // a real macro spelled back with an `IntLit` arm
    let code = "crates/diag/src/code.rs";
    let arm = "macro_rules! msgcodes {\n    (@lit) => { IntLit };\n";
    let m = with_file(code, None, |s| once(s, "macro_rules! msgcodes {\n", arm));
    fails(h5, &m, "macro_rules");
}

/// Controls that must NOT fire: a comment, a string, a reordered `use`.
#[test]
fn controls_neutral() {
    assert!(
        all_rules(&with_file("crates/elaborate/src/lib.rs", None, |s| once(
            s,
            "use hdl_ast::LitOrigin;",
            "use hdl_ast::LitOrigin; // LitOrigin::Parser origin: LitOrigin::Source"
        )))
        .is_empty()
    );
    assert!(all_rules(&plant(
        PLANT_E,
        "const S: &str = \"origin: LitOrigin::Parser IntLit { origin }\";"
    ))
    .is_empty());
    assert!(all_rules(&with_file(RANGE_RULES, None, |s| once(
        s,
        "IntLitKind, LitOrigin, Range",
        "IntLitKind, Range, LitOrigin"
    )))
    .is_empty());
}
