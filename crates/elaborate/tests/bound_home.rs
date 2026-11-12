//! The nine FQ parameter maps are written in one home (ROADMAP §5.2 BIND-1a, ER §5.1):
//! `crates/elaborate/src/bound/`. The compiler is the first pin: the fields are
//! `Bound<T>` and the mutable accessor is `pub(in crate::bound)`, so a raw `insert` /
//! `remove` / `&mut` through a field outside the home does not compile. This test pins
//! what the compiler does NOT refuse, and the home's own shape, counted per (file,
//! enclosing fn) over `crates/elaborate/src/**/*.rs`:
//!
//! - W0 the writer table: `. w ( )` per (file, fn) inside the home, exactly the moved
//!   funnel, the primitives, the restores and the put-backs.
//! - W1 `Bound :: new_empty (` nine times, in `driver.rs` `new` only.
//! - W2 the field-PLACE forms that compile outside the home (premise C2): F1 `& mut
//!   <recv> . <field>`, F2 `<recv> . <field> =` (an assignment, not `==` / `=>`), F3 a
//!   parenthesised left-hand side naming a field (`( .. <recv> . <field> .. ) =`), F4 a
//!   struct literal or pattern `Self {` / `Elaborator {` (not `-> Self {`): 0, 0, 0 and
//!   the one constructor ; F2's two hits on another struct's `params` field
//!   (`ModuleFacts`, `W2_OTHER_STRUCT`) are listed and counted.
//! - W3 the package constant store: each of its ten maps has one write (`insert` /
//!   `remove` / `entry` / `clear` / `extend` / `retain` / `get_mut` / `append` call, or
//!   F1 / F2) in `elaborate_package` (`pkg_var_aliases` is another store, left out).
//! - W4 DECL-ORDER: `bind_decl (` called by the three tails, one per (file, fn), and its
//!   body's calls in PRE's order: `publish_hier`, `set_meta`, `range ( self )`,
//!   `bind_param_value`, `bind_param_range`, `guessed ( self )`, `mark_guessed`.
//! - W5 the read lanes (BIND-1b, `bound/read.rs`), outside the home: W5a no
//!   `walk_scopes(.., &<recv>.<map>)` over a guarded map but the value walk
//!   `lookup_scoped` (the meta and real walks are `name_meta` / `name_real`) ; W5b no
//!   closure whose body names two or more of the nine maps ; W5c no `walk_scopes_key(..)`
//!   whose arguments spell a `<recv>.<map>.contains_key(` arm (the hit is a `hit_*`
//!   rule). Each exception is a row with its reason (`W5A_EXCEPT`, `W5B_EXCEPT`,
//!   `W5C_EXCEPT`). What the token shapes do not see is REVIEW-ONLY: a closure bound
//!   to a name and passed later is seen where it is written, not where it is walked ;
//!   a closure opened after another token than `( , = { ; => :`, `move`, `return` ;
//!   a map read through a local reference or a helper fn.
//!
//! What no shape sees is REVIEW-ONLY in `bound/mod.rs`'s head note: a write through a
//! reference bound earlier, a macro body, `mem::swap` / `replace` of two fields reached
//! through locals of a destructured `Elaborator`, a receiver spelled through a call.
//!
//! The walker is `value_arms_home.rs`'s: proc_macro2 tokens (comments dropped, doc
//! comments arrive as `#[doc]` groups), `#[..]` skipped whole, the item after
//! `#[cfg(test)]` skipped, a `Joint` punct joined to the next for `=> == != ::`, a token
//! keyed by its innermost enclosing `fn` (`-` outside any fn).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

const HOME: &str = "crates/elaborate/src/bound/";
const SRC: &str = "crates/elaborate/src";
const NINE: [&str; 9] = [
    "params",
    "param_meta",
    "param_range",
    "hier_param_range",
    "hier_params",
    "str_param_raw",
    "real_param_val",
    "wide_param_bits",
    "param_type_guessed",
];
const PKG_STORE: [&str; 10] = [
    "pkg_consts",
    "pkg_types",
    "pkg_enum_labels",
    "pkg_const_meta",
    "pkg_const_range",
    "pkg_real_val",
    "pkg_str_raw",
    "pkg_wide_bits",
    "pkg_array_const_vals",
    "pkg_array_const_meta",
];
const WRITE_VERBS: [&str; 8] = [
    "insert", "remove", "entry", "clear", "extend", "retain", "get_mut", "append",
];

/// W0: (file, fn, `.w()` count, what it writes) inside the home.
#[rustfmt::skip]
const WRITERS: &[(&str, &str, usize, &str)] = &[
    ("write.rs", "bind_param_value", 4, "the value funnel: both ranges and the guessed mark cleared, the value bound"),
    ("write.rs", "unbind_param", 2, "the local range and the value"),
    ("write.rs", "bind_param_range", 4, "both ranges, set or cleared"),
    ("write.rs", "bind_wide_param", 3, "the hier range set or cleared, the wide value"),
    ("write.rs", "set_meta", 1, "primitive"),
    ("write.rs", "clear_meta", 1, "primitive"),
    ("write.rs", "set_range_raw", 1, "primitive (PKG-ORDER, no hier twin)"),
    ("write.rs", "clear_range_raw", 1, "primitive (PKG-ORDER, no hier twin)"),
    ("write.rs", "publish_hier", 1, "primitive"),
    ("write.rs", "set_real", 1, "primitive"),
    ("write.rs", "clear_real", 1, "primitive (U5's suspend)"),
    ("write.rs", "set_str", 1, "primitive"),
    ("write.rs", "set_wide_raw", 1, "primitive"),
    ("write.rs", "clear_wide_raw", 1, "primitive"),
    ("write.rs", "mark_guessed", 1, "primitive"),
    ("write.rs", "drop_value_raw", 1, "primitive (U6, RTN-RAW-REMOVE)"),
    ("undo.rs", "restore_param_meta", 2, "U2"),
    ("undo.rs", "decl_keep_restore", 6, "U4 insert-if-kept"),
    ("undo.rs", "side_restore", 8, "U3 SIDE8 whole-map put-back"),
    ("undo.rs", "put_back_meta", 2, "U5-U8"),
    ("undo.rs", "put_back_range_raw", 2, "U7"),
    ("undo.rs", "put_back_real", 2, "U7"),
    ("undo.rs", "put_back_str", 2, "U7"),
    ("undo.rs", "put_back_wide", 2, "U7"),
];

/// W1 / W2-F4: (file, fn, count) of the constructor forms.
const W1_NEW: (&str, &str, usize) = ("crates/elaborate/src/driver.rs", "new", 9);
const W2_F4: (&str, &str, usize) = ("crates/elaborate/src/driver.rs", "new", 1);

/// W2-F2 hits that are another struct's field of a guarded name, each counted: `f.params =`
/// on `ModuleFacts::params` (`Option<Vec<ParamSlot>>`, expr_size_hier.rs), not the
/// Elaborator's map. A new hit anywhere else, or a third here, fails.
#[rustfmt::skip]
const W2_OTHER_STRUCT: &[(&str, &str, usize)] = &[
    ("crates/elaborate/src/expr_size_hier.rs", "build_module_facts", 1),
    ("crates/elaborate/src/expr_size_hier.rs", "module_facts", 1),
];

/// W4: the three DECL-ORDER callers.
#[rustfmt::skip]
const W4_CALLERS: &[(&str, &str, usize, &str)] = &[
    ("crates/elaborate/src/generate.rs", "lower_gen_module_item", 1, "T2 generate scope (result dropped)"),
    ("crates/elaborate/src/instance.rs", "elaborate_instance", 1, "T1 module body"),
    ("crates/elaborate/src/params.rs", "bind_one_param", 1, "T3 header"),
];
/// W4: `bind_decl`'s body, the calls in PRE's order.
const W4_ORDER: [&str; 7] = [
    "publish_hier",
    "set_meta",
    "range",
    "bind_param_value",
    "bind_param_range",
    "guessed",
    "mark_guessed",
];

/// W5a: a `walk_scopes(.., &<recv>.<guarded map>)` outside the home, (file, fn, map,
/// count, reason). `name_meta` / `name_real` (`bound/read.rs`) are the meta and real
/// walks ; the value walk is `lookup_scoped`.
#[rustfmt::skip]
const W5A_EXCEPT: &[(&str, &str, &str, usize, &str)] = &[
    ("crates/elaborate/src/scope.rs", "lookup_scoped", "params", 1, "the value walk, already one fn"),
];
/// W5b: a closure outside the home whose body names two or more of the nine maps,
/// (file, fn, count, reason).
#[rustfmt::skip]
const W5B_EXCEPT: &[(&str, &str, usize, &str)] = &[
    ("crates/elaborate/src/decl_scope.rs", "decl_keep_snapshot", 1, "U4's `DeclKept` builder: six `get`s, not a membership test"),
    ("crates/elaborate/src/edge_gate.rs", "binding_encloses", 1, "`other_kind` (wide || real || str || symbols): not a rule's set, also asked alone of the value walk's key"),
    ("crates/elaborate/src/const_str.rs", "const_str_at", 1, "left by R7: any_const_or_net in its own order (str, real, params, wide, symbols), not the rule's"),
    ("crates/elaborate/src/const_fn_width/fold_x.rs", "names_real_binding", 1, "left by R7, UNMEASURED: real_value_or_net in its own order ; re-pointed, its host fn's code moved (tbz -> cbz on the hit) and no cost cell counts a use"),
    ("crates/elaborate/src/param_query.rs", "real_param_lowers_real", 1, "left by R7, UNMEASURED: real_value_or_net in its own order ; re-pointed, its host fn's code moved (tbz -> cbz on the hit) and no cost cell counts a use"),
];
/// W5c: a `walk_scopes_key(..)` call outside the home whose arguments spell a
/// `<recv>.<guarded map>.contains_key(` arm instead of a `hit_*` rule, (file, fn,
/// count, reason).
#[rustfmt::skip]
const W5C_EXCEPT: &[(&str, &str, usize, &str)] = &[
    ("crates/elaborate/src/const_array.rs", "const_array_ref_of_base", 1, "array_const_vals || params || symbols: not a rule's set"),
    ("crates/elaborate/src/const_str.rs", "const_str_at", 1, "left by R7: any_const_or_net in its own order (str, real, params, wide, symbols)"),
    ("crates/elaborate/src/edge_gate.rs", "binding_encloses", 1, "left by R7: params || other_kind(k), any_const_or_net in another order"),
    ("crates/elaborate/src/hier_defer/write.rs", "every_name_resolves", 1, "left by R7: symbols || params, value_or_net in the other order"),
    ("crates/elaborate/src/const_fn_width/fold_x.rs", "names_real_binding", 1, "left by R7, UNMEASURED: real_value_or_net in its own order ; re-pointed, its host fn's code moved (tbz -> cbz on the hit) and no cost cell counts a use"),
    ("crates/elaborate/src/param_query.rs", "real_param_lowers_real", 1, "left by R7, UNMEASURED: real_value_or_net in its own order ; re-pointed, its host fn's code moved (tbz -> cbz on the hit) and no cost cell counts a use"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Id,
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
                push(out, Kind::Lit, l.to_string(), false, cur);
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

/// The index of the `(` that opens the group whose `)` is at `close`.
fn open_of(t: &[Tok], close: usize) -> Option<usize> {
    let mut d = 0usize;
    for j in (0..=close).rev() {
        if t[j].kind != Kind::P {
            continue;
        }
        match t[j].text.as_str() {
            ")" => d += 1,
            "(" => {
                d -= 1;
                if d == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
    }
    None
}

/// A field access `. <name>` that is not a method call (the next token is not `(`).
fn field_at(t: &[Tok], i: usize, names: &[&str]) -> bool {
    i >= 1
        && t[i - 1].text == "."
        && t[i].kind == Kind::Id
        && names.contains(&t[i].text.as_str())
        && t.get(i + 1).is_none_or(|x| x.text != "(")
}

/// (rule, key) hits of one source. Rules: W0 `.w()`, W1 `Bound::new_empty(`, W2F1-F4,
/// W3 (`W3:<map>` for a package-store write), W4c a `bind_decl (` call, W4o the
/// `bind_decl` body's order (one hit per call in `W4_ORDER`, keyed `W4o:<i>:<name>`).
fn hits(src: &str) -> Vec<(String, String)> {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let mut out: Vec<(String, String)> = Vec::new();
    for i in 0..t.len() {
        let v = tx(i);
        let key = t[i].key.clone();
        // W0: `. w ( )`
        if v == "w" && i >= 1 && tx(i - 1) == "." && tx(i + 1) == "(" && tx(i + 2) == ")" {
            out.push(("W0".into(), key.clone()));
        }
        // W1: `Bound :: new_empty (`
        if v == "Bound" && tx(i + 1) == "::" && tx(i + 2) == "new_empty" && tx(i + 3) == "(" {
            out.push(("W1".into(), key.clone()));
        }
        // W2 F1: `& mut <recv> . <field>` (the field not called)
        if v == "&" && tx(i + 1) == "mut" && tx(i + 3) == "." && field_at(&t, i + 4, &NINE) {
            out.push(("W2F1".into(), key.clone()));
        }
        // W2 F2: `<recv> . <field> =`
        if field_at(&t, i, &NINE) && tx(i + 1) == "=" {
            out.push(("W2F2".into(), key.clone()));
        }
        // W2 F3: `( .. . <field> .. ) =`
        if v == ")" && tx(i + 1) == "=" {
            if let Some(o) = open_of(&t, i) {
                if (o + 1..i).any(|j| field_at(&t, j, &NINE)) {
                    out.push(("W2F3".into(), key.clone()));
                }
            }
        }
        // W2 F4: `Self {` / `Elaborator {` (not a return type `-> Self {`)
        if matches!(v, "Self" | "Elaborator") && tx(i + 1) == "{" && (i == 0 || tx(i - 1) != ">") {
            out.push(("W2F4".into(), key.clone()));
        }
        // W3: a package-store write: `. <map> . <verb> (`, `& mut <recv> . <map>`,
        // `<recv> . <map> =`
        if field_at(&t, i, &PKG_STORE) {
            let m = t[i].text.clone();
            let call = tx(i + 1) == "." && WRITE_VERBS.contains(&tx(i + 2)) && tx(i + 3) == "(";
            let amut = i >= 4 && tx(i - 4) == "&" && tx(i - 3) == "mut";
            let assign = tx(i + 1) == "=";
            if call || amut || assign {
                out.push((format!("W3:{m}"), key.clone()));
            }
        }
        // W4c: a `bind_decl (` call (not its definition)
        if v == "bind_decl" && tx(i + 1) == "(" && (i == 0 || tx(i - 1) != "fn") {
            out.push(("W4c".into(), key.clone()));
        }
    }
    // W4o: inside `fn bind_decl`'s body, the calls of W4_ORDER's names, in source order.
    let mut n = 0;
    for (i, tok) in t.iter().enumerate() {
        if tok.key == "bind_decl"
            && W4_ORDER.contains(&tx(i))
            && tx(i + 1) == "("
            && (i == 0 || tx(i - 1) != "fn")
            && (!matches!(tx(i), "range" | "guessed") || tx(i + 2) == "self")
        {
            out.push(("W4o".into(), format!("{n}:{}", tx(i))));
            n += 1;
        }
    }
    out
}

/// The index of the token that closes the group opened at `open` (`(`, `{` or `[`).
fn close_of(t: &[Tok], open: usize) -> usize {
    let mut d = 0usize;
    for (j, x) in t.iter().enumerate().skip(open) {
        if x.kind != Kind::P {
            continue;
        }
        match x.text.as_str() {
            "(" | "{" | "[" => d += 1,
            ")" | "}" | "]" => {
                d -= 1;
                if d == 0 {
                    return j;
                }
            }
            _ => {}
        }
    }
    t.len()
}

/// The body span `[lo, hi)` of the closure whose parameter list opens with the `|` at
/// `i`, or `None` when that `|` is an operator. A closure opens after `( , = { ; => :`,
/// `move` or `return` ; its parameters end at the next `|` (`||` = none) ; a `-> T`
/// return type runs to the `{` ; a braced body is its group, any other body runs to
/// the `,` / `;` at its depth or the bracket that closes the enclosing group.
fn closure_body(t: &[Tok], i: usize) -> Option<(usize, usize)> {
    if t[i].kind != Kind::P || t[i].text != "|" {
        return None;
    }
    let prev = if i == 0 { "(" } else { t[i - 1].text.as_str() };
    if !matches!(
        prev,
        "(" | "," | "=" | "{" | ";" | "=>" | ":" | "move" | "return"
    ) {
        return None;
    }
    let mut end = i + 1;
    if !(t[i].joint && t.get(i + 1).is_some_and(|x| x.text == "|")) {
        while end < t.len() && t[end].text != "|" {
            end += 1;
        }
    }
    let mut b = end + 1;
    if t.get(b).is_some_and(|x| x.text == "-") && t.get(b + 1).is_some_and(|x| x.text == ">") {
        while b < t.len() && t[b].text != "{" {
            b += 1;
        }
    }
    if t.get(b).is_some_and(|x| x.text == "{") {
        return Some((b, close_of(t, b)));
    }
    let mut d = 0i32;
    let mut j = b;
    while j < t.len() {
        if t[j].kind == Kind::P {
            match t[j].text.as_str() {
                "(" | "{" | "[" => d += 1,
                ")" | "}" | "]" => {
                    d -= 1;
                    if d < 0 {
                        break;
                    }
                }
                "," | ";" if d == 0 => break,
                _ => {}
            }
        }
        j += 1;
    }
    Some((b, j))
}

/// W5 (rule, key) hits of one source: `W5a:<map>` a `walk_scopes(.., &<recv>.<map>)` ;
/// `W5b` a closure naming two or more of the nine maps ; `W5c` a `walk_scopes_key(..)`
/// whose arguments spell `<recv>.<map>.contains_key(`.
fn hits_w5(src: &str) -> Vec<(String, String)> {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let mut out: Vec<(String, String)> = Vec::new();
    for i in 0..t.len() {
        let key = t[i].key.clone();
        if tx(i) == "."
            && tx(i + 2) == "("
            && matches!(tx(i + 1), "walk_scopes" | "walk_scopes_key")
        {
            let close = close_of(&t, i + 2);
            if tx(i + 1) == "walk_scopes" {
                let mut d = 0i32;
                for j in i + 2..close {
                    match tx(j) {
                        "(" | "{" | "[" => d += 1,
                        ")" | "}" | "]" => d -= 1,
                        "," if d == 1 => {
                            if tx(j + 1) == "&"
                                && tx(j + 3) == "."
                                && field_at(&t, j + 4, &NINE)
                                && j + 5 == close
                            {
                                out.push((format!("W5a:{}", tx(j + 4)), key.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            } else if (i + 3..close)
                .any(|j| field_at(&t, j, &NINE) && tx(j + 1) == "." && tx(j + 2) == "contains_key")
            {
                out.push(("W5c".into(), key.clone()));
            }
        }
        if let Some((lo, hi)) = closure_body(&t, i) {
            let mut named: Vec<&str> = (lo..hi.min(t.len()))
                .filter(|&j| field_at(&t, j, &NINE))
                .map(|j| t[j].text.as_str())
                .collect();
            named.sort_unstable();
            named.dedup();
            if named.len() >= 2 {
                out.push(("W5b".into(), key.clone()));
            }
        }
    }
    out
}

fn in_home(rel: &str) -> bool {
    rel.starts_with(HOME)
}

#[test]
fn the_read_lanes_are_named() {
    let root = workspace_root();
    let mut got: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for (rel, src) in src_files(&root) {
        if in_home(&rel) {
            continue;
        }
        for (rule, key) in hits_w5(&src) {
            *got.entry((rule, rel.clone(), key)).or_insert(0) += 1;
        }
    }
    let mut want: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for (f, k, m, n, _) in W5A_EXCEPT {
        want.insert((format!("W5a:{m}"), f.to_string(), k.to_string()), *n);
    }
    for (f, k, n, _) in W5B_EXCEPT {
        want.insert(("W5b".into(), f.to_string(), k.to_string()), *n);
    }
    for (f, k, n, _) in W5C_EXCEPT {
        want.insert(("W5c".into(), f.to_string(), k.to_string()), *n);
    }
    assert_eq!(
        got, want,
        "W5 outside {HOME}: a meta / real walk, a multi-map closure or a scope-walk hit spelled at its site instead of `bound/read.rs` (a new exception is a row with its reason)"
    );
}

#[test]
fn the_nine_maps_are_written_in_the_home() {
    let root = workspace_root();
    let mut w0: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut outside: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    let mut w4o: Vec<String> = Vec::new();
    let mut seen_home = false;
    for (rel, src) in src_files(&root) {
        let home = in_home(&rel);
        seen_home |= home;
        for (rule, key) in hits(&src) {
            if rule == "W0" && home {
                let file = rel.trim_start_matches(HOME).to_string();
                *w0.entry((file, key)).or_insert(0) += 1;
            } else if rule == "W4o" {
                if rel == format!("{HOME}write.rs") {
                    w4o.push(key);
                }
            } else if home && rule != "W1" {
                // inside the home only W0 / W4o are claimed
            } else {
                *outside.entry((rule, rel.clone(), key)).or_insert(0) += 1;
            }
        }
    }
    assert!(seen_home, "no file under {HOME}");
    let want_w0: BTreeMap<(String, String), usize> = WRITERS
        .iter()
        .map(|(f, k, n, _)| ((f.to_string(), k.to_string()), *n))
        .collect();
    assert_eq!(
        w0, want_w0,
        "W0: the home's writer table moved (a new writer is a row with its reason)"
    );
    let mut want: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    want.insert(("W1".into(), W1_NEW.0.into(), W1_NEW.1.into()), W1_NEW.2);
    want.insert(("W2F4".into(), W2_F4.0.into(), W2_F4.1.into()), W2_F4.2);
    for (f, k, n) in W2_OTHER_STRUCT {
        want.insert(("W2F2".into(), f.to_string(), k.to_string()), *n);
    }
    for m in PKG_STORE {
        want.insert(
            (
                format!("W3:{m}"),
                "crates/elaborate/src/package.rs".into(),
                "elaborate_package".into(),
            ),
            1,
        );
    }
    for (f, k, n, _) in W4_CALLERS {
        want.insert(("W4c".into(), f.to_string(), k.to_string()), *n);
    }
    assert_eq!(
        outside, want,
        "W1-W4 outside {HOME}: a guarded field written by place, a second constructor, a package-store writer, or a DECL-ORDER caller moved"
    );
    let want_o: Vec<String> = W4_ORDER
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{i}:{s}"))
        .collect();
    assert_eq!(
        w4o, want_o,
        "W4: bind_decl's body left PRE's order (hier, meta, range fold, value, range, guessed test, mark)"
    );
}

fn count(rule: &str, src: &str) -> usize {
    hits(src).iter().filter(|(r, _)| r == rule).count()
}

#[test]
fn walker_controls() {
    // W0: `.w()` counted, `.w(x)`, `w()` without a receiver, a comment and a string not.
    assert_eq!(
        count(
            "W0",
            "fn f() { self.params.w().insert(k, v); self.x\n .w()\n .remove(k); }"
        ),
        2
    );
    assert_eq!(
        count(
            "W0",
            "fn f() { self.params.w(1); w(); let s = \".w()\"; // self.params.w()\n }"
        ),
        0
    );
    // W1: the call counted, a path in a type or a comment not.
    assert_eq!(
        count(
            "W1",
            "fn new() -> E { E { a: Bound::new_empty(), b: Bound :: new_empty () } }"
        ),
        2
    );
    assert_eq!(
        count(
            "W1",
            "fn f(x: Bound<T>) { /* Bound::new_empty() */ Bound::new(); }"
        ),
        0
    );
    // W2 F1: `&mut <recv>.<field>` on each receiver spelling ; not a method call, not
    // another field, not `&self.params`.
    assert_eq!(
        count(
            "W2F1",
            "fn f() { std::mem::swap(&mut self.params, &mut e.hier_params); g(& mut me . param_meta); }"
        ),
        3
    );
    assert_eq!(
        count(
            "W2F1",
            "fn f() { g(&mut self.params_x); g(&self.params); g(&mut self.params()); }"
        ),
        0
    );
    // W2 F2: an assignment ; not a compare, a match arm, a method, another field.
    assert_eq!(
        count(
            "W2F2",
            "fn f() { self.params = Bound::new_empty(); s.param_range\n = x; }"
        ),
        2
    );
    assert_eq!(
        count(
            "W2F2",
            "fn f() { if self.params == x {} let a = self.params_x = 1; match q { self.params => 0 }; self.params(1); }"
        ),
        0
    );
    // W2 F3: the SIDE8 tuple form and a nested one ; not a call followed by `==`, not a
    // tuple of other fields.
    assert_eq!(
        count(
            "W2F3",
            "fn f() { (self.wide_param_bits, self.param_range) = side; (a, (b, self.params)) = z; }"
        ),
        2
    );
    assert_eq!(
        count(
            "W2F3",
            "fn f() { (self.cur_prefix, self.symbols) = side; if g(self.params) == x {} let (a, b) = (self.params.len(), 1); }"
        ),
        0
    );
    // W2 F4: a struct literal / pattern ; not a return type, not a generic impl head.
    assert_eq!(
        count(
            "W2F4",
            "fn new() -> Self { Self { a: 1 } } fn g(e: E) { let Elaborator { params, .. } = e; }"
        ),
        2
    );
    assert_eq!(
        count(
            "W2F4",
            "fn new() -> Self { make() } impl<'s> Elaborator<'s> { fn h() {} }"
        ),
        0
    );
    // W3: a write verb, `&mut`, an assignment on a store map ; not a read, not
    // `pkg_var_aliases`.
    assert_eq!(
        count(
            "W3:pkg_consts",
            "fn f() { self.pkg_consts.insert(p, c); self.pkg_consts.entry(p); g(&mut self.pkg_consts); self.pkg_consts = m; }"
        ),
        4
    );
    assert_eq!(
        count(
            "W3:pkg_consts",
            "fn f() { self.pkg_consts.get(p); self.pkg_var_aliases.insert(k, v); self.pkg_consts.contains_key(p); }"
        ),
        0
    );
    // W4c: a call (multi-line too), not the definition.
    assert_eq!(
        count(
            "W4c",
            "fn bind_decl(&mut self) {} fn g() { self\n .bind_decl(\n &key, v, meta, r, g); }"
        ),
        1
    );
    // W4o: the order is read from the body ; a swapped body reads swapped.
    let order = |src: &str| -> Vec<String> {
        hits(src)
            .into_iter()
            .filter(|(r, _)| r == "W4o")
            .map(|(_, k)| k)
            .collect()
    };
    let pre = "fn bind_decl(&mut self) { self.publish_hier(k, v); if let Some(m) = meta { self.set_meta(k, m); } let r = range(self); let prev = self.bind_param_value(k, v); self.bind_param_range(k, r); if guessed(self) { self.mark_guessed(k); } prev }";
    assert_eq!(
        order(pre),
        vec![
            "0:publish_hier",
            "1:set_meta",
            "2:range",
            "3:bind_param_value",
            "4:bind_param_range",
            "5:guessed",
            "6:mark_guessed"
        ]
    );
    let m1 = "fn bind_decl(&mut self) { self.publish_hier(k, v); if let Some(m) = meta { self.set_meta(k, m); } let prev = self.bind_param_value(k, v); let r = range(self); self.bind_param_range(k, r); if guessed(self) { self.mark_guessed(k); } prev }";
    assert_ne!(order(m1), order(pre));
    // `#[cfg(test)]` items are skipped.
    assert_eq!(
        count(
            "W2F2",
            "#[cfg(test)]\nmod t { fn f() { self.params = x; } }\nfn g() {}"
        ),
        0
    );
}

fn count_w5(rule: &str, src: &str) -> usize {
    hits_w5(src).iter().filter(|(r, _)| r == rule).count()
}

#[test]
fn walker_controls_w5() {
    // W5a: the meta / real walk on each receiver, multi-line ; not the lane, not
    // another table, not a map that is the walk's first argument.
    assert_eq!(
        count_w5(
            "W5a:param_meta",
            "fn f() { self.walk_scopes(&p.segments[0].name, &self.param_meta); me\n .walk_scopes(n,\n &me.param_meta); }"
        ),
        2
    );
    assert_eq!(
        count_w5(
            "W5a:real_param_val",
            "fn f() { self.walk_scopes(n, &self.real_param_val).is_some(); }"
        ),
        1
    );
    assert_eq!(
        hits_w5("fn f() { self.name_meta(n); self.walk_scopes(n, &self.symbols); self.walk_scopes(&self.params, t); self.walk_scopes(n, &self.param_meta.x); }").len(),
        0
    );
    // W5b: a closure (typed, `move`, a return type, a braced body, a struct literal)
    // naming two maps ; not one map, not a logical `||` between two maps, not a rule.
    assert_eq!(
        count_w5(
            "W5b",
            "fn f() { let o = |k: &str| self.params.contains_key(k) || self.wide_param_bits.contains_key(k); g(move |me: &Self| -> bool { me.params.len() > me.param_meta.len() }); h(|k| D { a: self.str_param_raw.get(k), b: self.real_param_val.get(k) }); }"
        ),
        3
    );
    assert_eq!(
        count_w5(
            "W5b",
            "fn f() { let o = |k: &str| self.params.contains_key(k) || self.symbols.contains_key(k); if self.params.contains_key(k) || self.wide_param_bits.contains_key(k) {} g(|k| self.hit_any_const(k)); x.or_else(|| self.name_meta(n)); }"
        ),
        0
    );
    // a closure that ends at its argument's `,` does not take the next argument's maps
    assert_eq!(
        count_w5(
            "W5b",
            "fn f() { g(|k| self.params.contains_key(k), self.wide_param_bits.len()); }"
        ),
        0
    );
    // W5c: an arm spelled in the walk's argument (one map is enough) ; not a rule, not
    // `symbols` alone, not a `contains_key` outside the call.
    assert_eq!(
        count_w5(
            "W5c",
            "fn f() { self.walk_scopes_key(n, |k| self.params.contains_key(k)); self\n .walk_scopes_key(&seg.name, |k| { self.symbols.contains_key(k) || self.params.contains_key(k) }); }"
        ),
        2
    );
    assert_eq!(
        count_w5(
            "W5c",
            "fn f() { self.walk_scopes_key(n, |k| self.hit_value(k)).is_some_and(|k| self.params.contains_key(&k)); self.walk_scopes_key(n, |k| self.symbols.contains_key(k)); }"
        ),
        0
    );
}
