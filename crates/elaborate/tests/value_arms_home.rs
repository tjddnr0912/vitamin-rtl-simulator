//! The constant-fold arms two or more of elaborate's fold cores spelled have one home
//! (ROADMAP §5.2 VAL-1, ER §5.1): `crates/elaborate/src/value/arms.rs` -- the truth of a
//! self-determined operand (`!` and the `?:` condition), unary `+ - ~` on a folded operand,
//! and the size / prim / signing cast.
//!
//! This pin claims ONLY the exact token shapes below (A1 A2 A2b A3 A4), counted per (file,
//! enclosing fn) over `crates/elaborate/src/**/*.rs`, each hit outside the home an exception
//! named with its reason. What no shape sees (a truth written as a `let` then a compare on
//! another statement, `.is_some_and`, `matches!`, a fold reached through another name, a
//! negation written `0 - v` or `!v + 1`, a macro body, the sibling evaluators) is REVIEW-ONLY
//! in `value/arms.rs`'s head note and is not claimed here.
//!
//! The walker is `value_rules_home.rs`'s: every file read whole with `proc_macro2` (comments
//! dropped by the lexer, doc comments arrive as `#[doc]` groups), `#[..]` / `#![..]` skipped
//! whole, the item after an attribute whose tokens are EXACTLY `cfg(test)` skipped through its
//! first brace body or its `;` (so `arms.rs`'s PRE-text equality module is not counted), the
//! tree flattened with explicit delimiters, a `Joint` punct joined to the next only for
//! `=> == != ::`, a token keyed by its innermost enclosing `fn` (`-` outside any fn).
//!
//! Shapes (the same tokens W/val1/plan/census/pins.py counts):
//! - A1 `selfdet_truth (`, the token before not `fn` (a call).
//! - A2 `eval_const_env_self | const_int_selfdet ( .. ) ? ==|!= 0`.
//! - A2b the same call ( .. ) then `{ Some ( v ) =>` then one of `v cmp 0`, `Some ( v cmp 0`
//!   (the home's own form, premise C1), `Some ( ( v cmp 0`, `Val ( ( v cmp 0` (cmp `==|!=`).
//! - A3 `checked_neg (`.
//! - A4 `fn const_eval_cast | const_size_cast | selfdet_truth_arm | unary_i64 |
//!   signing_over_size_cast`, keyed by the defined name: only in the home, each once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

const HOME: &str = "crates/elaborate/src/value/arms.rs";
const SRC: &str = "crates/elaborate/src";
const A4_FNS: [&str; 5] = [
    "const_eval_cast",
    "const_size_cast",
    "selfdet_truth_arm",
    "unary_i64",
    "signing_over_size_cast",
];

/// (pin, key, hits, reason) inside the home.
#[rustfmt::skip]
const HOME_COUNTS: &[(&str, &str, usize, &str)] = &[
    ("A1", "selfdet_truth_arm", 1, "the wide-domain truth fallback, IN and CTX lanes"),
    ("A2b", "selfdet_truth_arm", 1, "the i64 truth `Some(v) => Some(v != 0)`"),
    ("A3", "unary_i64", 1, "unary minus declines at i64::MIN"),
    ("A4", "const_eval_cast", 1, "moved from const_fn.rs"),
    ("A4", "const_size_cast", 1, "moved from const_fn.rs"),
    ("A4", "selfdet_truth_arm", 1, "the truth arm"),
    ("A4", "signing_over_size_cast", 1, "the twin's one body (PARSE-1 L14)"),
    ("A4", "unary_i64", 1, "the unary kernel"),
];

/// (pin, file, enclosing fn, hits, reason) outside the home.
#[rustfmt::skip]
const EXCEPTIONS: &[(&str, &str, &str, usize, &str)] = &[
    ("A2", "crates/elaborate/src/const_fn.rs", "exec_const_stmt", 3, "the interpreter's if / while / for conditions (control flow, the Env lane's text ; not a fold arm)"),
    ("A2b", "crates/elaborate/src/const_real.rs", "const_eval_real_in_scope", 1, "E-real: the real twin's condition, its own `None` branch (a sibling evaluator)"),
    ("A3", "crates/elaborate/src/const_fn_width/fold.rs", "fold_ctx", 1, "CTX's Four arm: an x-free (absorbed) operand negates exactly ; the x plane's own arm"),
    ("A3", "crates/elaborate/src/expr_size_hier.rs", "env_fold", 1, "HIER lane: `+ -` only, over (value, width)"),
    ("A3", "crates/elaborate/src/block_local/proofs.rs", "fold_i32", 1, "O1 the i32 proof fold (a sixth fold, VAL-2 lane census)"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Id,
    Num,
    Lit,
    P,
}

#[derive(Clone, Debug)]
struct Tok {
    kind: Kind,
    text: String,
    joint: bool,
    key: String,
}

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

/// Every `crates/elaborate/src/**/*.rs`, relative to the workspace root, sorted.
fn src_files(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    rs_files(&root.join(SRC), &mut files);
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

fn is_cfg_test(attr: &TokenStream) -> bool {
    let s: String = attr
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    s == "cfg(test)"
}

fn push(out: &mut Vec<Tok>, kind: Kind, text: String, joint: bool, key: &str) {
    out.push(Tok {
        kind,
        text,
        joint,
        key: key.to_string(),
    });
}

fn walk(ts: TokenStream, cur: &str, out: &mut Vec<Tok>) {
    let tt: Vec<TokenTree> = ts.into_iter().collect();
    let mut pending_fn: Option<String> = None;
    let mut i = 0;
    while i < tt.len() {
        match &tt[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                let mut j = i + 1;
                if matches!(tt.get(j), Some(TokenTree::Punct(q)) if q.as_char() == '!') {
                    j += 1;
                }
                if let Some(TokenTree::Group(g)) = tt.get(j) {
                    if g.delimiter() == Delimiter::Bracket {
                        i = j + 1;
                        if is_cfg_test(&g.stream()) {
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
                push(out, Kind::P, "#".into(), p.spacing() == Spacing::Joint, cur);
                i += 1;
            }
            TokenTree::Ident(id) if id == "fn" => {
                push(out, Kind::Id, "fn".into(), false, cur);
                if let Some(TokenTree::Ident(name)) = tt.get(i + 1) {
                    pending_fn = Some(name.to_string());
                    push(out, Kind::Id, name.to_string(), false, cur);
                    i += 2;
                    continue;
                }
                i += 1;
            }
            TokenTree::Punct(p) if p.as_char() == ';' => {
                pending_fn = None;
                push(out, Kind::P, ";".into(), false, cur);
                i += 1;
            }
            TokenTree::Group(g) => {
                let mut key = cur.to_string();
                if g.delimiter() == Delimiter::Brace {
                    if let Some(f) = pending_fn.take() {
                        key = f;
                    }
                }
                let (o, c) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                if !o.is_empty() {
                    push(out, Kind::P, o.into(), false, &key);
                }
                walk(g.stream(), &key, out);
                if !c.is_empty() {
                    push(out, Kind::P, c.into(), false, &key);
                }
                i += 1;
            }
            TokenTree::Ident(id) => {
                push(out, Kind::Id, id.to_string(), false, cur);
                i += 1;
            }
            TokenTree::Punct(p) => {
                let j = p.spacing() == Spacing::Joint;
                push(out, Kind::P, p.as_char().to_string(), j, cur);
                i += 1;
            }
            TokenTree::Literal(l) => {
                let s = l.to_string();
                let kind = if s.starts_with(|c: char| c.is_ascii_digit()) {
                    Kind::Num
                } else {
                    Kind::Lit
                };
                push(out, kind, s, false, cur);
                i += 1;
            }
        }
    }
}

/// The pairs a `Joint` punct joins with the next punct.
const PAIRS: [&str; 4] = ["=>", "==", "!=", "::"];

fn tokens(src: &str) -> Vec<Tok> {
    let ts = TokenStream::from_str(src).unwrap_or_else(|e| panic!("tokenize: {e:?}"));
    let mut seq = Vec::new();
    walk(ts, "-", &mut seq);
    let mut res: Vec<Tok> = Vec::with_capacity(seq.len());
    let mut i = 0;
    while i < seq.len() {
        let t = &seq[i];
        if t.kind == Kind::P && t.joint && i + 1 < seq.len() && seq[i + 1].kind == Kind::P {
            let pair = format!("{}{}", t.text, seq[i + 1].text);
            if PAIRS.contains(&pair.as_str()) {
                let mut j = t.clone();
                j.text = pair;
                j.joint = seq[i + 1].joint;
                res.push(j);
                i += 2;
                continue;
            }
        }
        res.push(t.clone());
        i += 1;
    }
    res
}

/// The index after the `)` closing the `(` at `i`.
fn after_parens(t: &[Tok], i: usize) -> usize {
    let mut d = 0usize;
    for (j, x) in t.iter().enumerate().skip(i) {
        if x.kind == Kind::P && x.text == "(" {
            d += 1;
        } else if x.kind == Kind::P && x.text == ")" {
            d -= 1;
            if d == 0 {
                return j + 1;
            }
        }
    }
    t.len()
}

/// (pin, key) of every hit of one source.
fn hits(src: &str) -> Vec<(&'static str, String)> {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let is_cmp0 = |i: usize| matches!(tx(i), "==" | "!=") && tx(i + 1) == "0";
    let mut out = Vec::new();
    for i in 0..t.len() {
        let v = tx(i);
        let call = tx(i + 1) == "(" && (i == 0 || tx(i - 1) != "fn");
        if v == "selfdet_truth" && call {
            out.push(("A1", t[i].key.clone()));
        }
        if matches!(v, "eval_const_env_self" | "const_int_selfdet") && call {
            let j = after_parens(&t, i + 1);
            if tx(j) == "?" && is_cmp0(j + 1) {
                out.push(("A2", t[i].key.clone()));
            }
            let head = ["{", "Some", "(", "v", ")", "=>"];
            if (0..6).all(|k| tx(j + k) == head[k]) {
                let b = j + 6;
                let a2b = (tx(b) == "v" && is_cmp0(b + 1))
                    || (tx(b) == "Some" && tx(b + 1) == "(" && tx(b + 2) == "v" && is_cmp0(b + 3))
                    || (matches!(tx(b), "Some" | "Val")
                        && tx(b + 1) == "("
                        && tx(b + 2) == "("
                        && tx(b + 3) == "v"
                        && is_cmp0(b + 4));
                if a2b {
                    out.push(("A2b", t[i].key.clone()));
                }
            }
        }
        if v == "checked_neg" && tx(i + 1) == "(" {
            out.push(("A3", t[i].key.clone()));
        }
        if v == "fn" && A4_FNS.contains(&tx(i + 1)) {
            out.push(("A4", tx(i + 1).to_string()));
        }
    }
    out
}

#[test]
fn value_arm_shapes_live_in_the_home() {
    let root = workspace_root();
    let mut home: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut got: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for (rel, src) in src_files(&root) {
        for (pin, key) in hits(&src) {
            if rel == HOME {
                *home.entry((pin.to_string(), key)).or_insert(0) += 1;
            } else {
                *got.entry((pin.to_string(), rel.clone(), key)).or_insert(0) += 1;
            }
        }
    }
    let want_home: BTreeMap<(String, String), usize> = HOME_COUNTS
        .iter()
        .map(|(p, k, n, _)| ((p.to_string(), k.to_string()), *n))
        .collect();
    let want: BTreeMap<(String, String, String), usize> = EXCEPTIONS
        .iter()
        .map(|(p, f, k, n, _)| ((p.to_string(), f.to_string(), k.to_string()), *n))
        .collect();
    assert_eq!(
        home, want_home,
        "the home's own hits moved: spell the rule once in {HOME}"
    );
    assert_eq!(
        got, want,
        "a value-arm shape outside {HOME}: spell it in the home, or add the key with its reason"
    );
}

#[test]
fn walker_controls() {
    let count = |pin: &str, src: &str| hits(src).iter().filter(|(p, _)| *p == pin).count();
    // A1: a multi-line call counts ; a comment, a string, a definition, another name do not.
    assert_eq!(
        count("A1", "fn f() { let c = self\n .selfdet_truth(\n e,\n )?; }"),
        1
    );
    assert_eq!(
        count(
            "A1",
            "fn selfdet_truth() {} fn g() { let s = \"selfdet_truth(e)\"; // selfdet_truth(e)\n self.selfdet_truth_reading_str(e); }"
        ),
        0
    );
    // A2: the compare after `?`, on the next line too ; not `? != 1`, not a bare `?`.
    assert_eq!(
        count(
            "A2",
            "fn f() { if self.eval_const_env_self(c, env, envw, d)?\n != 0 {} Some((self.const_int_selfdet(f(a, (b)))? == 0) as i64) }"
        ),
        2
    );
    assert_eq!(
        count(
            "A2",
            "fn f() { let v = self.eval_const_env_self(e, env, envw, d)?; let w = self.eval_const_env_self(e, env, envw, d)? != 1; }"
        ),
        0
    );
    // A2b: the home's verbatim arm body (premise C1) and PRE's three forms ; not `v > 0`,
    // not `v + 1`.
    assert_eq!(
        count(
            "A2b",
            "fn selfdet_truth_arm(&self) -> Option<bool> { match self.eval_const_env_self(e, env, envw, depth) { Some(v) => Some(v != 0), None => { if lane == ArmLane::Env { return None; } self.selfdet_truth(e) } } }"
        ),
        1
    );
    assert_eq!(
        count(
            "A2b",
            "fn f() { match self.const_int_selfdet(c) { Some(v) => v != 0, None => x }; match self.eval_const_env_self(o, e, w, d) {\n Some(v) => Val((v == 0) as i64), None => No }; match self.const_int_selfdet(c) { Some(v) => Some((v == 0) as i64), None => None }; }"
        ),
        3
    );
    assert_eq!(
        count(
            "A2b",
            "fn f() { match self.eval_const_env_self(e, a, b, c) { Some(v) => Some(v > 0), None => None }; match self.const_int_selfdet(c) { Some(v) => v + 1, None => 0 }; }"
        ),
        0
    );
    // A3 / A4.
    assert_eq!(
        count(
            "A3",
            "fn f(v: i64) { let g = |x: i64| x.checked_neg(); v.checked_neg(); v.wrapping_neg(); }"
        ),
        2
    );
    assert_eq!(
        count(
            "A4",
            "fn unary_i64() {} fn g() { unary_i64(op, v); } /* fn const_size_cast() {} */"
        ),
        1
    );
    // `#[cfg(test)]` items are skipped (premise C6) ; `#[cfg(not(test))]` items are read.
    assert_eq!(
        count(
            "A3",
            "#[cfg(test)] mod tests { fn t() { v.checked_neg(); } } #[cfg(not(test))] fn k(v: i64) { v.checked_neg(); }"
        ),
        1
    );
    // Keys: the innermost enclosing fn ; a definition shape keyed by its name.
    let keys: Vec<(&str, String)> = hits("fn outer() { fn inner(v: i64) { v.checked_neg(); } }");
    assert_eq!(keys, vec![("A3", "inner".to_string())]);
}
