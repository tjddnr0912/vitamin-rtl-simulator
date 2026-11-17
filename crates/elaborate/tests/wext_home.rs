//! §5.2 Q-WREXT: the write-extent view has one home, `crates/elaborate/src/wext.rs` (ER
//! §5.1). This test pins, counted per (file, enclosing fn) over
//! `crates/elaborate/src/**/*.rs` (the item after `#[cfg(test)]` skipped), what the view
//! rests on and what must stay true while nothing reads it:
//!
//! - H1 the append points of the arrays the view reads: `. <arr> . push (` for
//!   `cont_assigns` / `ca_idents` (`push_cont_assign` only), `processes` / `proc_idents`
//!   (`push_process` only), `stmts` (`push_stmt` for the arena, `push_stmt_id` for a
//!   block's list), `func_blocks` and `funcs` (the frame builders) — premise judge 1 K1's
//!   "one funnel each" (W/mdrv/premise/REPORT.txt:33-36).
//! - H1b their in-place writers: `& mut <recv> . <arr>`, `. <arr> . <verb> (` for a mutating
//!   verb other than `push`, and `. <arr> [ .. ] .. =` — each a listed site that runs
//!   before `build_write_view` in the end phase (the deferred hierarchical patches, the late
//!   edges, the read-set recompute, the deferred hierarchical call). A new one fails: its
//!   author says whether it runs before the view or the view goes stale.
//! - H2 `const_chunk_written_bits (` is called by `check_whole_net_multidriver` and by
//!   `chunk_extent` only: the view's `Bits` is the kernel's answer, not a copy.
//! - H3 the view is built once and read by nothing: `build_view (` only in
//!   `build_write_view`, `build_write_view (` only in the end phase (`run`), `. wext`
//!   only where it is built and handed to `Sidecars` (the `wext:` initialiser in
//!   `driver.rs` `new` is a field init, not this token), and the token
//!   `write_view` in no production source of any crate but `api.rs` (the hand-off) and
//!   `tables.rs` (the field).
//! - H4 who reads the process / continuous-row identities (`proc_idents`, `ca_idents`):
//!   the producers, the hand-off, `wext.rs`, and the two readers that compare a kind today
//!   (`cont_array.rs` `other_writers`, `hier_defer/func_call.rs`). PLAN B9's H4 asked for 0
//!   compares outside wext.rs and cont_array.rs ; the tree has a third
//!   (`hier_defer/func_call.rs`, an `always_comb` caller test of the deferred hierarchical
//!   call), listed here as it is.
//! - H5 who decides a writer from a statement: the IR storing-statement PATTERNS (not
//!   `ast :: Stmt ::`) (`Stmt :: BlockingAssign | NonblockingAssign | Force | Release { .. }` where a match arm
//!   `=>`, an alternative `|`, an `if let` / `let .. else` `=` or a `matches !` reads them)
//!   and `stmt_lvalue (` calls, per (file, fn) — `other_writers` is the listed bypass that
//!   decides writers without the view (its re-point is Q-WREXT's filed line).
//! - H2:at `chunk_written_bits_at (` (the kernel's width and select step at an offset the
//!   caller supplies, Q-WREXT C2) is called by `chunk_extent` only: a geometry extent is
//!   the kernel's step on the recorded constant, not a second extent rule.
//! - H7 (C2) the geometry constant: `record_geom (` in the two `flatten_word` twins,
//!   `carry_geom (` at the two lowerings that add a constant to a flat offset, and the raw
//!   store `. wext_rec` inside wext.rs's writers and `build_write_view` only.
//! - H8 (C2) the declaration fact: `record_decl_fact (` in its three writer fns, the
//!   `DeclScope :: <V>` each `elaborate_netvar_decl` / `elaborate_ports` caller passes and
//!   the `DeclSite :: <V>` each writer names, per (file, fn) ; and (R8) no `intro_kind` /
//!   `net_names` token in a writer's host fn or in wext.rs but the listed two.
//! - H9 (C2) the `via` intervals: `record_via (` at the inline decision and around the
//!   deferred action, and each `Via :: <V>` spelling.
//! - H6 the write-shape producers: `LvalChunk {` constructions (43 at b9cb67a1, 8 of them
//!   inside `vec ! [ .. ]`, which this token walker reads: premise judge 1 K1's count) and
//!   `ContAssign {` constructions (8), per (file, fn). A new producer fails: its author
//!   checks the extent rule (`chunk_extent`) still says what it writes.
//!
//! REVIEW-ONLY for C2: that a writer's argument is the RIGHT value (a site passed as
//! another variant of the same enum moves a count only where the spelling moves ; the
//! cli pins `wext_view.rs` w12-w14 and the lane's view record hold the values), that the
//! `via` interval brackets exactly the call's statements (the pins), and that
//! `GeomAcc` matches `flatten_word`'s arithmetic (sim-engine's
//! `geometry_constant_is_what_the_stores_write`).
//!
//! REVIEW-ONLY (no token shape here sees it): an array reached through a local bound to
//! it before (`let s = &mut self.stmts;` IS seen ; `let e = self; e.stmts.push(..)` is seen
//! as H1 ; a reference handed into a helper fn and written there is seen only by the
//! helper's own spelling), a write through `mem::swap` / `mem::replace` with no `& mut
//! <recv> . <arr>` token run, a macro that builds the receiver, a pattern spelled through
//! a type alias of `Stmt`, a construction through a helper (`whole_net_lvalue`,
//! `poison_chunk` are counted where their own `LvalChunk {` is). `exprs`, `consts` and
//! `nets` (an offset tree's leaves, a net's width) are read by the kernel at the end
//! phase and not pinned here: a later edit of an offset tree would make the view stale
//! without moving a count. The ORDER is review-only too: that each H1b site runs before
//! `build_write_view` (and that no append runs after it) is the listed reason, read from
//! the call chain in `driver.rs` `run`, not a token this walker counts.
//!
//! The walker is `bound_home.rs`'s (proc_macro2 tokens, comments dropped, `#[..]` skipped,
//! the item after `#[cfg(test)]` skipped, `=> == != ::` joined, a token keyed by its
//! innermost enclosing `fn`, `-` outside any fn).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

const SRC: &str = "crates/elaborate/src";
/// H1 / H1b: the arrays the view reads (and the identity tables kept beside two of them).
const ARR: [&str; 7] = [
    "cont_assigns",
    "ca_idents",
    "processes",
    "proc_idents",
    "stmts",
    "func_blocks",
    "funcs",
];
/// H1b / H11: the mutating verbs (`push` is H1's) — the grounding's `mutwalk2.py` VERBS
/// list (premise judge 1 line 6 of §5.2 Q-WEXTPOS: mutwalk.py's list plus `split_at_mut`
/// and `chunks_mut`; PLAN §6 MUT: `splice`, `extend_from_slice` and the `split_*_mut` /
/// `chunks*_mut` family), `push` left out.
const MUT: &[&str] = &[
    "extend",
    "insert",
    "append",
    "truncate",
    "clear",
    "retain",
    "remove",
    "get_mut",
    "iter_mut",
    "last_mut",
    "first_mut",
    "swap",
    "drain",
    "pop",
    "resize",
    "resize_with",
    "extend_from_slice",
    "splice",
    "swap_remove",
    "split_off",
    "sort",
    "sort_by",
    "sort_by_key",
    "sort_unstable",
    "sort_unstable_by",
    "sort_unstable_by_key",
    "dedup",
    "dedup_by",
    "dedup_by_key",
    "reverse",
    "rotate_left",
    "rotate_right",
    "fill",
    "fill_with",
    "copy_from_slice",
    "clone_from",
    "as_mut",
    "as_mut_slice",
    "entry",
    "or_insert",
    "or_default",
    "set_len",
    "values_mut",
    "get_or_insert_with",
    "take",
    "replace",
    "split_at_mut",
    "chunks_mut",
    "chunks_exact_mut",
    "split_first_mut",
    "split_last_mut",
    "retain_mut",
    "push_back",
    "push_front",
    "pop_back",
    "pop_front",
];
/// H11c: the verbs that add an entry to a producer list.
const ADD: &[&str] = &[
    "push",
    "insert",
    "extend",
    "append",
    "splice",
    "entry",
    "push_back",
    "extend_from_slice",
];
const STORE: [&str; 4] = ["BlockingAssign", "NonblockingAssign", "Force", "Release"];

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

/// Every `.rs` under `<root>/<dir>`, relative to the workspace root, sorted.
fn files_in(root: &Path, dir: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    rs_files(&root.join(dir), &mut files);
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

/// The token index of the `}` / `)` / `]` closing the group opened at `open`.
fn close_of(t: &[Tok], open: usize) -> usize {
    let (o, c) = match t[open].text.as_str() {
        "{" => ("{", "}"),
        "(" => ("(", ")"),
        _ => ("[", "]"),
    };
    let mut d = 0usize;
    for (j, x) in t.iter().enumerate().skip(open) {
        if x.kind != Kind::P {
            continue;
        }
        if x.text == o {
            d += 1;
        } else if x.text == c {
            d -= 1;
            if d == 0 {
                return j;
            }
        }
    }
    t.len() - 1
}

/// After the `]` at `close`: an `=` (an assignment, not `==` / `=>` / `!=`) before the end of
/// the statement (`;`, or a `{` / `}` / `,` at this depth).
fn assigned_after(t: &[Tok], close: usize) -> bool {
    let mut j = close + 1;
    while j < t.len() {
        let x = &t[j];
        if x.kind == Kind::P {
            match x.text.as_str() {
                "(" | "[" | "{" => {
                    if x.text == "{" {
                        return false;
                    }
                    j = close_of(t, j) + 1;
                    continue;
                }
                ";" | "}" | ")" | "]" | "," => return false,
                "=" => {
                    let prev = &t[j - 1];
                    let cmp = prev.kind == Kind::P
                        && prev.joint
                        && ["<", ">", "!", "=", "+", "-", "*", "/", "|", "&", "^", "%"]
                            .contains(&prev.text.as_str());
                    let next_joined =
                        x.joint && t.get(j + 1).is_some_and(|n| n.text == "=" || n.text == ">");
                    if !cmp && !next_joined {
                        return true;
                    }
                }
                _ => {}
            }
        }
        j += 1;
    }
    false
}

/// A storing-statement path `Stmt :: <V> {` at `i` (the `{`) is a PATTERN when its closing
/// `}` (and any `)` right after it) is followed by `=>`, `|`, `=` or `if` (a guard), or when
/// it sits inside `matches ! (`.
fn is_pattern(t: &[Tok], open: usize) -> bool {
    let mut close = close_of(t, open);
    // `Some(ir :: Stmt :: X { .. })` : the pattern's own closing parens first
    while t.get(close + 1).is_some_and(|n| n.text == ")") {
        close += 1;
    }
    if let Some(n) = t.get(close + 1) {
        if n.kind == Kind::P && (n.text == "=>" || n.text == "|" || (n.text == "=" && !n.joint)) {
            return true;
        }
        if n.kind == Kind::Id && n.text == "if" {
            return true;
        }
    }
    // inside `matches ! ( .. )`
    let mut d = 0i32;
    let mut j = open;
    while j > 0 {
        j -= 1;
        let x = &t[j];
        if x.kind != Kind::P {
            continue;
        }
        match x.text.as_str() {
            ")" | "]" | "}" => d += 1,
            "(" | "[" | "{" => {
                if d == 0 {
                    return x.text == "("
                        && j >= 2
                        && t[j - 1].text == "!"
                        && t[j - 2].text == "matches";
                }
                d -= 1;
            }
            _ => {}
        }
    }
    false
}

/// (rule, fn key) hits of one source.
fn hits(src: &str) -> Vec<(String, String)> {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let mut out: Vec<(String, String)> = Vec::new();
    for i in 0..t.len() {
        let k = t[i].key.clone();
        let id = t[i].kind == Kind::Id;
        // H1 / H1b `. <arr> . <verb> (`
        if tx(i) == "." && ARR.contains(&tx(i + 1)) && tx(i + 2) == "." && tx(i + 4) == "(" {
            let verb = tx(i + 3);
            if verb == "push" {
                out.push((format!("H1:{}", tx(i + 1)), k.clone()));
            } else if MUT.contains(&verb) {
                out.push((format!("H1b:{}:{verb}", tx(i + 1)), k.clone()));
            }
        }
        // H1b `& mut <path> . <arr>` (not a method call on it)
        if tx(i) == "&" && tx(i + 1) == "mut" {
            let mut j = i + 2;
            let mut last: Option<usize> = None;
            while j < t.len() {
                let x = &t[j];
                if x.kind == Kind::Id {
                    last = Some(j);
                    j += 1;
                } else if x.text == "." || x.text == "::" {
                    j += 1;
                } else {
                    break;
                }
            }
            if let Some(l) = last {
                if l >= 1 && t[l - 1].text == "." && ARR.contains(&tx(l)) && tx(l + 1) != "(" {
                    out.push((format!("H1b:{}:&mut", tx(l)), k.clone()));
                }
                // H11 `& mut <path> . exprs`
                if l >= 1 && t[l - 1].text == "." && tx(l) == "exprs" && tx(l + 1) != "(" {
                    let recv = if l >= 2 && tx(l - 2) == "rs" {
                        "rs.exprs"
                    } else {
                        "exprs"
                    };
                    out.push((format!("H11:{recv}:&mut"), k.clone()));
                }
            }
        }
        // H1b `. <arr> [ .. ] .. =`
        if tx(i) == "." && ARR.contains(&tx(i + 1)) && tx(i + 2) == "[" {
            let close = close_of(&t, i + 2);
            if assigned_after(&t, close) {
                out.push((format!("H1b:{}:index=", tx(i + 1)), k.clone()));
            }
        }
        // H1b `. <arr> =` (the whole field assigned; `==` / `=>` are one token here)
        if tx(i) == "." && ARR.contains(&tx(i + 1)) && tx(i + 2) == "=" {
            out.push((format!("H1b:{}:=", tx(i + 1)), k.clone()));
        }
        // H11 the `exprs` arena's writers: `. exprs . <verb> (` (`push` too), `. exprs [ .. ]
        // .. =`, `. exprs =` ; the read set's map `rs . exprs` keyed apart
        if tx(i) == "." && tx(i + 1) == "exprs" {
            let recv = if i >= 1 && tx(i - 1) == "rs" {
                "rs.exprs"
            } else {
                "exprs"
            };
            if tx(i + 2) == "." && tx(i + 4) == "(" {
                let verb = tx(i + 3);
                if verb == "push" || MUT.contains(&verb) {
                    out.push((format!("H11:{recv}:{verb}"), k.clone()));
                }
            }
            if tx(i + 2) == "[" && assigned_after(&t, close_of(&t, i + 2)) {
                out.push((format!("H11:{recv}:index="), k.clone()));
            }
            if tx(i + 2) == "=" {
                out.push((format!("H11:{recv}:="), k.clone()));
            }
        }
        // H11b the stand-down's setters and the registered set's insert
        if tx(i) == "."
            && tx(i + 1) == "undecided_placeholders"
            && tx(i + 2) == "."
            && tx(i + 3) == "insert"
            && tx(i + 4) == "("
        {
            out.push(("H11b:undecided_placeholders.insert".into(), k.clone()));
        }
        // H11c the producer lists: `. <recv> . <add verb> (` for a receiver `deferred_<x>`,
        // `pending_<x>`, `late_<x>`, `provisional_<x>` or `<x>_placeholders`
        if tx(i) == "."
            && t.get(i + 1).is_some_and(|x| x.kind == Kind::Id)
            && tx(i + 2) == "."
            && ADD.contains(&tx(i + 3))
            && tx(i + 4) == "("
        {
            let r = tx(i + 1);
            if ["deferred_", "pending_", "late_", "provisional_"]
                .iter()
                .any(|p| r.starts_with(p))
                || r.ends_with("_placeholders")
            {
                out.push((format!("H11c:{r}:{}", tx(i + 3)), k.clone()));
            }
        }
        if id && tx(i + 1) == "(" && (i == 0 || tx(i - 1) != "fn") {
            match tx(i) {
                "const_chunk_written_bits" => out.push(("H2".into(), k.clone())),
                "build_view" => out.push(("H3:build_view".into(), k.clone())),
                "build_write_view" => out.push(("H3:build_write_view".into(), k.clone())),
                "stmt_lvalue" => out.push(("H5:stmt_lvalue".into(), k.clone())),
                "chunk_written_bits_at" => out.push(("H2:at".into(), k.clone())),
                "record_geom" | "carry_geom" => out.push((format!("H7:{}", tx(i)), k.clone())),
                "record_decl_fact" => out.push(("H8:record_decl_fact".into(), k.clone())),
                "record_via" => out.push(("H9:record_via".into(), k.clone())),
                "note_provisional" | "register_provisional_const" | "note_patch" => {
                    out.push((format!("H11b:{}", tx(i)), k.clone()))
                }
                "patch_expr" | "patch_expr_with" => {
                    out.push((format!("H11:call:{}", tx(i)), k.clone()))
                }
                _ => {}
            }
        }
        if tx(i) == "." && tx(i + 1) == "wext" && tx(i + 2) != "(" {
            out.push(("H3:wext".into(), k.clone()));
        }
        // H7 the C2 records' raw store: `. wext_rec` (a field read or write, not a call)
        if tx(i) == "." && tx(i + 1) == "wext_rec" && tx(i + 2) != "(" {
            out.push(("H7:wext_rec".into(), k.clone()));
        }
        // H8 / H9 the site and via values: `DeclScope :: <V>`, `DeclSite :: <V>`, `Via :: <V>`
        if id
            && matches!(tx(i), "DeclScope" | "DeclSite" | "Via")
            && tx(i + 1) == "::"
            && t.get(i + 2).is_some_and(|x| x.kind == Kind::Id)
        {
            let r = if tx(i) == "Via" { "H9" } else { "H8" };
            out.push((format!("{r}:{}::{}", tx(i), tx(i + 2)), k.clone()));
        }
        // H8 the declaration fact reads no introduction kind and no net name
        if id && (tx(i) == "intro_kind" || tx(i) == "net_names") {
            out.push((format!("R8:{}", tx(i)), k.clone()));
        }
        if id && tx(i) == "write_view" {
            out.push(("H3:write_view".into(), k.clone()));
        }
        if id && (tx(i) == "proc_idents" || tx(i) == "ca_idents") {
            out.push((format!("H4:{}", tx(i)), k.clone()));
        }
        // H5 storing-statement patterns
        if id
            && STORE.contains(&tx(i))
            && i >= 2
            && tx(i - 1) == "::"
            && tx(i - 2) == "Stmt"
            && !(i >= 4 && tx(i - 3) == "::" && tx(i - 4) == "ast")
            && tx(i + 1) == "{"
            && is_pattern(&t, i + 1)
        {
            out.push((format!("H5:{}", tx(i)), k.clone()));
        }
        // H6 constructions: not a return type (`-> ir :: LvalChunk {`), not the type's own
        // `struct` item
        if id && (tx(i) == "LvalChunk" || tx(i) == "ContAssign") && tx(i + 1) == "{" {
            let mut j = i;
            while j >= 2 && tx(j - 1) == "::" && t[j - 2].kind == Kind::Id {
                j -= 2;
            }
            let ret = j >= 2 && tx(j - 1) == ">" && tx(j - 2) == "-";
            if !ret && (j == 0 || tx(j - 1) != "struct") {
                out.push((format!("H6:{}", tx(i)), k.clone()));
            }
        }
    }
    out
}

type Table = BTreeMap<(String, String, String), usize>;

/// Every rule hit of the elaborate sources, keyed (rule, file under `crates/elaborate/src`, fn).
fn census() -> Table {
    let root = workspace_root();
    let mut tab = Table::new();
    for (rel, src) in files_in(&root, SRC) {
        let f = rel.trim_start_matches("crates/elaborate/src/").to_string();
        // `src/tests/` is the crate's `#[cfg(test)] mod tests` (lib.rs), test code only.
        if f.starts_with("tests/") {
            continue;
        }
        for (r, k) in hits(&src) {
            *tab.entry((r, f.clone(), k)).or_default() += 1;
        }
    }
    tab
}

fn rows_of(tab: &Table, prefix: &str) -> Table {
    tab.iter()
        .filter(|((r, _, _), _)| r == prefix || r.starts_with(&format!("{prefix}:")))
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

fn want(rows: &[(&str, &str, &str, usize, &str)]) -> Table {
    rows.iter()
        .map(|(r, f, k, n, _)| ((r.to_string(), f.to_string(), k.to_string()), *n))
        .collect()
}

fn check(prefix: &str, rows: &[(&str, &str, &str, usize, &str)]) {
    let got = rows_of(&census(), prefix);
    let want = want(rows);
    if got != want {
        let mut s = String::new();
        for ((r, f, k), n) in &got {
            s.push_str(&format!("    (\"{r}\", \"{f}\", \"{k}\", {n}, \"\"),\n"));
        }
        panic!("{prefix}: the census moved.\n--- got ---\n{s}--- want ---\n{want:?}");
    }
}

/// The count of `rule` (exact, or `rule:<anything>`) in one synthetic source.
fn count(rule: &str, src: &str) -> usize {
    hits(src)
        .iter()
        .filter(|(r, _)| r == rule || r.starts_with(&format!("{rule}:")))
        .count()
}

#[test]
fn h1_the_append_points() {
    check("H1", H1);
}

#[test]
fn h1b_the_in_place_writers() {
    check("H1b", H1B);
}

#[test]
fn h7_the_geometry_constant_home() {
    check("H7", H7);
}

/// H11 (§5.2 Q-WEXTPOS C1): every in-place write of an `exprs` slot is the patch funnel's.
/// Also (premise judge round 1, line 3: A4's census widened to api.rs's three passes
/// after `run`): those passes and the four checks after `build_write_view` hold no H11 or
/// H1b token of their own — what this walker can see of "nothing patches after the view";
/// the order itself is the funnel's `debug_assert!` (a debug run only).
#[test]
fn h11_the_exprs_slot_writers() {
    check("H11", H11);
    let tab = census();
    let after = [
        "check_whole_net_multidriver",
        "check_whole_array_sole_writer",
        "check_var_cont_assign_sole_writer",
        "check_readmem_assoc_waits",
        "assert_block_local_inits_drained",
        "finish_assert_item_gate",
        "edge_backstop",
    ];
    let hit: Vec<_> = tab
        .keys()
        .filter(|(r, _, k)| {
            (r.starts_with("H11:") || r.starts_with("H1b:")) && after.contains(&k.as_str())
        })
        .collect();
    assert!(hit.is_empty(), "a writer after the view: {hit:?}");
    for f in after {
        assert!(
            census_fn_exists(f),
            "`{f}` is not a fn of elaborate/src any more: re-read A4's list"
        );
    }
}

/// H11b (§5.2 Q-WEXTPOS C1): the stand-down's two setters each have their one caller,
/// and the registered set has one insert, inside the registration.
#[test]
fn h11b_the_stand_down_setters() {
    check("H11b", H11B);
}

/// H11c (§5.2 Q-WEXTPOS C1, premise judge 1 line 5 of the row): every producer list's
/// add site, per (file, fn), with two hand-written columns — whether the node it records
/// is an `exprs` `Const` the end phase replaces, and whether it is registered. A new site
/// fails here until its author says both; a `Const` one must call
/// `register_provisional_const` in the same fn or say why not.
#[test]
fn h11c_the_producer_lists() {
    let rows: Vec<(&str, &str, &str, usize, &str)> = H11C
        .iter()
        .map(|&(r, f, k, n, _, _)| (r, f, k, n, ""))
        .collect();
    check("H11c", &rows);
    let tab = census();
    for &(r, f, k, _, konst, reg) in H11C {
        assert!(
            matches!(konst, "y" | "n" | "no slot"),
            "{r} {f} {k}: Const column `{konst}`"
        );
        assert!(!reg.is_empty(), "{r} {f} {k}: registered column empty");
        if konst == "y" && reg == "y" {
            let registers = tab.keys().any(|(rr, ff, kk)| {
                (rr == "H11b:register_provisional_const" || rr == "H11b:note_provisional")
                    && ff == f
                    && kk == k
            });
            assert!(registers, "{r} {f} {k}: says registered, calls no setter");
        }
        if konst == "y" && reg != "y" {
            assert!(
                reg.starts_with("no: "),
                "{r} {f} {k}: a Const not registered says why"
            );
        }
    }
}

/// Whether some fn of elaborate/src is named `name` (the census's own walk).
fn census_fn_exists(name: &str) -> bool {
    let root = workspace_root();
    files_in(&root, SRC).iter().any(|(_, src)| {
        let t = tokens(src);
        t.windows(2).any(|w| w[0].text == "fn" && w[1].text == name)
    })
}

#[test]
fn h8_the_declaration_fact_home() {
    check("H8", H8);
    // what the fact's host fns and wext.rs spell of `intro_kind` / `net_names`
    let tab = census();
    let hosts: Vec<(String, String)> = tab
        .keys()
        .filter(|(r, _, _)| r == "H8:record_decl_fact")
        .map(|(_, f, k)| (f.clone(), k.clone()))
        .collect();
    let got: Table = rows_of(&tab, "R8")
        .into_iter()
        .filter(|((_, f, k), _)| f == "wext.rs" || hosts.contains(&(f.clone(), k.clone())))
        .collect();
    let want = want(H8R);
    if got != want {
        let mut s = String::new();
        for ((r, f, k), n) in &got {
            s.push_str(&format!("    (\"{r}\", \"{f}\", \"{k}\", {n}, \"\"),\n"));
        }
        panic!("R8: the census moved.\n--- got ---\n{s}--- want ---\n{want:?}");
    }
}

#[test]
fn h9_the_via_interval_home() {
    check("H9", H9);
}

#[test]
fn h2_the_kernel_callers() {
    check("H2", H2);
}

#[test]
fn h3_built_once_read_by_nothing() {
    check("H3", H3);
    // `write_view` in the production source of every other crate: none.
    let root = workspace_root();
    let mut other: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).expect("crates") {
        let p = entry.expect("entry").path();
        let name = p.file_name().expect("name").to_string_lossy().to_string();
        if name == "elaborate" || !p.join("src").is_dir() {
            continue;
        }
        let files = files_in(&root, &format!("crates/{name}/src"));
        let test_only = cfg_test_module_files(&files);
        for (rel, src) in &files {
            if test_only.contains(rel) {
                continue;
            }
            for (r, k) in hits(src) {
                if r == "H3:write_view" {
                    other.push(format!("{rel} {k}"));
                }
            }
        }
    }
    assert!(
        other.is_empty(),
        "`write_view` read outside elaborate: {other:?}"
    );
}

/// The files a `#[cfg(test)] mod <name>;` declaration (other attributes may sit between,
/// none may carry `#[path]`) brings in — test code, as the walker skips an inline
/// `#[cfg(test)]` item: `<dir>/<name>.rs` beside a `mod.rs` / `lib.rs` / `main.rs`, else
/// `<file stem>/<name>.rs`.
fn cfg_test_module_files(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (rel, src) in files {
        let mut rest = src.as_str();
        while let Some(at) = rest.find("#[cfg(test)]") {
            rest = &rest[at + "#[cfg(test)]".len()..];
            let mut tail = rest.trim_start();
            let mut has_path = false;
            while tail.starts_with("#[") {
                let end = tail.find(']').map_or(tail.len(), |e| e + 1);
                has_path |= tail[..end].starts_with("#[path");
                tail = tail[end..].trim_start();
            }
            let Some(m) = tail.strip_prefix("mod ") else {
                continue;
            };
            let Some(semi) = m.find(';') else { continue };
            let name = m[..semi].trim();
            if has_path || name.contains(|c: char| !(c.is_alphanumeric() || c == '_')) {
                continue;
            }
            let (dir, file) = rel.rsplit_once('/').expect("a path under crates/");
            let base = if matches!(file, "mod.rs" | "lib.rs" | "main.rs") {
                dir.to_string()
            } else {
                format!("{dir}/{}", file.trim_end_matches(".rs"))
            };
            out.push(format!("{base}/{name}.rs"));
        }
    }
    out
}

#[test]
fn h4_the_identity_readers() {
    check("H4", H4);
}

#[test]
fn h5_who_decides_a_writer_from_a_statement() {
    check("H5", H5);
}

#[test]
fn h6_the_write_shape_producers() {
    check("H6", H6);
    let n: usize = H6
        .iter()
        .filter(|r| r.0 == "H6:LvalChunk")
        .map(|r| r.3)
        .sum();
    assert_eq!(n, 43, "premise judge 1 K1: 43 `LvalChunk` constructions");
    let c: usize = H6
        .iter()
        .filter(|r| r.0 == "H6:ContAssign")
        .map(|r| r.3)
        .sum();
    assert_eq!(c, 8, "premise judge 1 K1: 8 `ContAssign` constructions");
}

/// Each rule fires on its planted shape, on a real site spelled back into another fn, and
/// not on a comment, a `#[cfg(test)]` item or the neighbouring shape it must not count.
#[test]
fn walker_controls() {
    // the cross-crate `write_view` scan skips a `#[cfg(test)] mod x;` file, not a plain `mod x;`
    let fs = vec![
        (
            "crates/c/src/a/mod.rs".to_string(),
            "#[cfg(test)]\nmod t;\nmod p;\n#[cfg(test)]\n#[path = \"../q.rs\"]\nmod q;".to_string(),
        ),
        (
            "crates/c/src/b.rs".to_string(),
            "#[cfg(test)]\n#[allow(x)]\nmod u;".to_string(),
        ),
    ];
    assert_eq!(
        cfg_test_module_files(&fs),
        vec![
            "crates/c/src/a/t.rs".to_string(),
            "crates/c/src/b/u.rs".to_string()
        ],
        "cfg(test) module files"
    );
    // H1
    let h1 = "impl E { fn f(&mut self) { self.stmts.push(s); self.cont_assigns.push(c); x.processes.push(p); } }";
    assert_eq!(count("H1", h1), 3, "H1 planted");
    assert_eq!(
        count("H1", "// self.stmts.push(s);\nfn f() {}"),
        0,
        "H1 comment"
    );
    assert_eq!(
        count(
            "H1",
            "#[cfg(test)]\nfn t(&mut self) { self.stmts.push(s); }"
        ),
        0,
        "H1 cfg(test)"
    );
    let back = hits("impl E { fn g(&mut self, s: ir::Stmt) { self.stmts.push(s); } }");
    assert_eq!(
        back,
        vec![("H1:stmts".to_string(), "g".to_string())],
        "H1 real site (stmt_flow.rs push_stmt) spelled back in g"
    );
    // H1b
    assert_eq!(
        count("H1b", "fn f(&mut self) { let a = &mut self.stmts; }"),
        1,
        "H1b &mut"
    );
    assert_eq!(
        count(
            "H1b",
            "fn f(&mut self) { self.cont_assigns.retain(|_| true); }"
        ),
        1,
        "H1b verb"
    );
    assert_eq!(
        count("H1b", "fn f(&mut self) { self.stmts[i] = s; }"),
        1,
        "H1b index="
    );
    assert_eq!(
        count(
            "H1b",
            "fn f(&mut self) { self.stmts[i].lhs.chunks[0].net = n; }"
        ),
        1,
        "H1b index= deep"
    );
    assert_eq!(
        count("H1b", "fn f(&self) { let a = self.stmts[i].clone(); }"),
        0,
        "H1b read"
    );
    assert_eq!(
        count("H1b", "fn f(&self) { if self.stmts[i] == s {} }"),
        0,
        "H1b compare"
    );
    assert_eq!(
        count("H1b", "fn f(&self) { let n = &mut self.stmts.len(); }"),
        0,
        "H1b &mut of a call"
    );
    let back = hits("impl E { fn g(&mut self) { for s in &mut self.stmts { s.x(); } } }");
    assert_eq!(
        back,
        vec![("H1b:stmts:&mut".to_string(), "g".to_string())],
        "H1b real site (hier_defer/write.rs) spelled back"
    );
    // H2
    assert_eq!(
        count("H2", "fn f() { ir::const_chunk_written_bits(ctx, c, w); }"),
        1,
        "H2 planted"
    );
    assert_eq!(
        count("H2", "pub fn const_chunk_written_bits(c: u32) {}"),
        0,
        "H2 definition"
    );
    // H3
    assert_eq!(
        count("H3", "fn f(&mut self) { self.build_write_view(); }"),
        1,
        "H3 build_write_view"
    );
    assert_eq!(
        count("H3", "fn f(&mut self) { let v = build_view(&s); }"),
        1,
        "H3 build_view"
    );
    assert_eq!(
        count("H3", "fn f(&self) { let w = &self.wext; }"),
        1,
        "H3 .wext"
    );
    assert_eq!(
        count("H3", "fn f(sc: &S) { sc.write_view.writes.len(); }"),
        1,
        "H3 write_view read"
    );
    assert_eq!(
        count("H3", "fn f(&self) { self.wext_x(); }"),
        0,
        "H3 another name"
    );
    // H4
    assert_eq!(
        count(
            "H4",
            "fn f(&self) { if self.proc_idents[p].kind == \"always_comb\" {} }"
        ),
        1,
        "H4 planted"
    );
    // H5
    assert_eq!(
        count(
            "H5",
            "fn f(s: &ir::Stmt) { match s { ir::Stmt::BlockingAssign { lhs, .. } => {} _ => {} } }"
        ),
        1,
        "H5 match arm"
    );
    assert_eq!(
        count(
            "H5",
            "fn f(s: &ir::Stmt) { if let ir::Stmt::Force { lhs, .. } = s {} }"
        ),
        1,
        "H5 if let"
    );
    assert_eq!(
        count(
            "H5",
            "fn f(s: &ir::Stmt) -> bool { matches!(s, ir::Stmt::Release { .. }) }"
        ),
        1,
        "H5 matches!"
    );
    assert_eq!(count("H5", "fn f(s: Option<&ir::Stmt>) { match s { Some(ir::Stmt::NonblockingAssign { lhs, .. }) => {} _ => {} } }"), 1, "H5 Some(..) =>");
    assert_eq!(
        count(
            "H5",
            "fn f(&mut self) { self.push_stmt(ir::Stmt::BlockingAssign { lhs, rhs }); }"
        ),
        0,
        "H5 construction"
    );
    assert_eq!(
        count(
            "H5",
            "fn f(s: &ast::Stmt) { match s { ast::Stmt::Force { rhs, .. } => {} _ => {} } }"
        ),
        0,
        "H5 an AST statement"
    );
    assert_eq!(
        count("H5", "fn f(s: &ir::Stmt) { let l = stmt_lvalue(s); }"),
        1,
        "H5 stmt_lvalue call"
    );
    // H6
    let h6 = "fn f() { let v = vec![ir::LvalChunk { net, word: None, offset: None, width: None, kind }]; }";
    assert_eq!(count("H6", h6), 1, "H6 inside vec![..]");
    assert_eq!(
        count("H6", "fn f(n: u32) -> ir::LvalChunk { whole(n) }"),
        0,
        "H6 return type"
    );
    assert_eq!(
        count("H6", "pub struct LvalChunk { pub net: u32 }"),
        0,
        "H6 the struct item"
    );
    assert_eq!(
        count(
            "H6",
            "fn f() { let c = ir::ContAssign { lhs, rhs, delay: None }; }"
        ),
        1,
        "H6 ContAssign"
    );
    // H2:at
    assert_eq!(
        count(
            "H2",
            "fn f() { let e = ir::chunk_written_bits_at(ctx, c, off, w); }"
        ),
        1,
        "H2:at planted"
    );
    // H7
    let h7 = "fn f(&mut self) { self.record_geom(off, &geo); self.carry_geom(o, b, 3); self.wext_rec.geom.insert(1, 2); }";
    assert_eq!(count("H7", h7), 3, "H7 planted");
    assert_eq!(
        count(
            "H7",
            "fn record_geom(&mut self) {} fn g(&self) { self.wext_rec(); }"
        ),
        0,
        "H7 definition / a call"
    );
    let back = hits("impl E { fn g(&mut self) { self.record_geom(off, &geo); off } }");
    assert_eq!(
        back,
        vec![("H7:record_geom".to_string(), "g".to_string())],
        "H7 real site (array_geom.rs flatten_word) spelled back"
    );
    // H8
    let h8 = "fn f(&mut self) { self.elaborate_netvar_decl(d, a, wext::DeclScope::Module); \
              self.record_decl_fact(n, Some(DeclSite::NonAnsiPort), k, dr, false); }";
    assert_eq!(count("H8", h8), 3, "H8 planted: scope, site, writer");
    assert_eq!(
        count(
            "R8",
            "fn f(&self) { self.intro_kind.get(&n); let _ = &sc.net_names; }"
        ),
        2,
        "R8 planted reads"
    );
    assert_eq!(
        count("H8", "// self.record_decl_fact(n, s, k, d, t);\nfn f() {}"),
        0,
        "H8 comment"
    );
    let back = hits(
        "impl E { fn g(&mut self) { self.elaborate_netvar_decl(d, i, false, true, wext::DeclScope::BlockLocal); } }",
    );
    assert_eq!(
        back,
        vec![("H8:DeclScope::BlockLocal".to_string(), "g".to_string())],
        "H8 real site (block_local/hoist.rs) spelled back"
    );
    // H9
    let h9 = "fn f(&mut self) { let v0 = self.stmts.len(); self.record_via(wext::Via::TaskInline, v0); }";
    assert_eq!(count("H9", h9), 2, "H9 planted: writer, value");
    assert_eq!(
        count(
            "H9",
            "#[cfg(test)]\nfn t(&mut self) { self.record_via(Via::DeferAction, 0); }"
        ),
        0,
        "H9 cfg(test)"
    );
    // MUT (Q-WEXTPOS C1): the widened verbs and the whole-field assignment, on an H1b array
    assert_eq!(
        count(
            "H1b",
            "fn f(&mut self) { self.stmts.splice(0..0, v); self.funcs.split_at_mut(2); self.stmts = Vec::new(); }"
        ),
        3,
        "MUT planted: splice, split_at_mut, `. <arr> =`"
    );
    assert_eq!(
        count(
            "H1b",
            "fn f(&self) -> bool { self.stmts == v || self.stmts.len() > 0 }"
        ),
        0,
        "MUT: a compare and a read"
    );
    let back = hits(
        "impl E { fn g(&mut self) { if let Some(b) = self.func_blocks.get_mut(rb) { b.stmts.splice(0..0, u); } } }",
    );
    assert_eq!(
        back,
        vec![
            ("H1b:func_blocks:get_mut".to_string(), "g".to_string()),
            ("H1b:stmts:splice".to_string(), "g".to_string())
        ],
        "MUT real site (hier_defer/func_call.rs install_deferred_hier_call) spelled back"
    );
    // H11: PLAN §6's planted raw slot write in `resolve_deferred_hier`, each writer shape,
    // the read set's map apart, and what is not a write
    assert_eq!(
        hits("impl E { fn resolve_deferred_hier(&mut self) { self.exprs[d.eid as usize] = m; } }"),
        vec![(
            "H11:exprs:index=".to_string(),
            "resolve_deferred_hier".to_string()
        )],
        "H11 planted: a raw slot write beside the funnel"
    );
    let h11 = "fn f(&mut self) { self.exprs.split_at_mut(3); self.exprs = Vec::new(); \
               let a = std::mem::take(&mut self.exprs); self.exprs.iter_mut(); }";
    assert_eq!(
        count("H11", h11),
        4,
        "H11 planted: split_at_mut, `=`, &mut, iter_mut"
    );
    assert_eq!(
        hits("fn f(&mut self) { self.rs.exprs.insert(i, m); }"),
        vec![("H11:rs.exprs:insert".to_string(), "f".to_string())],
        "H11 the read set's map keyed apart"
    );
    assert_eq!(
        count(
            "H11",
            "fn f(&self) { if self.exprs[e] == x {} let a = &self.exprs[e]; self.exprs.len(); }\n\
             // self.exprs[e] = x;"
        ),
        0,
        "H11 a compare, a read, a length, a comment"
    );
    let back = hits(
        "impl E { fn g(&mut self, eid: u32, e: ir::Expr) { let slot = &mut self.exprs[eid as usize]; \
         self.wext_rec.note_patch(slot); *slot = e; } }",
    );
    assert_eq!(
        back,
        vec![
            ("H11:exprs:&mut".to_string(), "g".to_string()),
            ("H7:wext_rec".to_string(), "g".to_string()),
            ("H11b:note_patch".to_string(), "g".to_string())
        ],
        "H11 real site (wext.rs patch_expr) spelled back"
    );
    // H11b: a second registration caller, the set's insert spelled raw again, a definition
    let h11b = "impl E { fn lower_bits_fold(&mut self) { self.undecided_placeholders.insert(eid); } \
                fn h(&mut self) { self.register_provisional_const(e); self.wext_rec.note_provisional(); } }";
    let got: Vec<(String, String)> = hits(h11b)
        .into_iter()
        .filter(|(r, _)| r.starts_with("H11b:"))
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "H11b:undecided_placeholders.insert".to_string(),
                "lower_bits_fold".to_string()
            ),
            (
                "H11b:register_provisional_const".to_string(),
                "h".to_string()
            ),
            ("H11b:note_provisional".to_string(), "h".to_string())
        ],
        "H11b planted"
    );
    assert_eq!(
        count(
            "H11b",
            "fn note_patch(&mut self, old: &ir::Expr) {} fn register_provisional_const(&mut self) {}"
        ),
        0,
        "H11b definitions"
    );
    // H11c: PLAN §6's planted new producer, the receiver shapes, and what is not an add
    assert_eq!(
        hits("impl E { fn lower_x(&mut self) { self.deferred_new.push(x); } }"),
        vec![("H11c:deferred_new:push".to_string(), "lower_x".to_string())],
        "H11c planted: an unlisted producer"
    );
    assert_eq!(
        count(
            "H11c",
            "fn f(&mut self) { self.foo_placeholders.insert(e); self.pending_q.entry(k); self.late_v.extend(w); }"
        ),
        3,
        "H11c receiver shapes"
    );
    assert_eq!(
        count(
            "H11c",
            "fn f(&mut self) { self.deferred_hier.len(); self.pending_y.drain(..); self.deferredx.push(x); }"
        ),
        0,
        "H11c a read, a removal, another name"
    );
    let back = hits(
        "impl E { fn g(&mut self) { let eid = self.int_result_expr(32); self.register_provisional_const(eid); \
         self.deferred_hier_bits.push(d); } }",
    );
    assert_eq!(
        back,
        vec![
            (
                "H11b:register_provisional_const".to_string(),
                "g".to_string()
            ),
            ("H11c:deferred_hier_bits:push".to_string(), "g".to_string())
        ],
        "H11c real site (expr_special.rs lower_bits_fold) spelled back"
    );
}

/// H1: (rule, file, fn, count, why) — the append points.
#[rustfmt::skip]
const H1: &[(&str, &str, &str, usize, &str)] = &[
    ("H1:ca_idents", "proc_ident.rs", "push_cont_assign", 1, "the continuous append point (lockstep with ca_idents)"),
    ("H1:cont_assigns", "proc_ident.rs", "push_cont_assign", 1, "the continuous append point (lockstep with ca_idents)"),
    ("H1:func_blocks", "class_lower.rs", "lower_class_method_body", 1, "a class method frame body (lowered in run, before the end phase)"),
    ("H1:func_blocks", "frames_body.rs", "lower_frame_body_inner_lower_frame_func_body", 1, "a function frame body"),
    ("H1:func_blocks", "frames_body.rs", "lower_frame_body_inner_lower_frame_task_body", 1, "a task frame body"),
    ("H1:funcs", "frames_reserve.rs", "push_func", 1, "the frame append point"),
    ("H1:proc_idents", "stmt_flow.rs", "push_process", 1, "the process append point (lockstep with proc_idents)"),
    ("H1:processes", "stmt_flow.rs", "push_process", 1, "the process append point (lockstep with proc_idents)"),
    ("H1:stmts", "proc_builder.rs", "push_stmt_id", 1, "a block's statement list (a body under construction)"),
    ("H1:stmts", "stmt_flow.rs", "push_stmt", 1, "the statement arena append point"),
];

/// H1b: the in-place writers, each before the view or not an IR array.
#[rustfmt::skip]
const H1B: &[(&str, &str, &str, usize, &str)] = &[
    ("H1b:ca_idents:&mut", "api.rs", "elaborate_located_params", 1, "after the view: moves the identity table into Sidecars (mem::take), no edit"),
    ("H1b:cont_assigns:&mut", "edge_gate.rs", "materialize", 1, "decide_late_edges, before the view: re-points a decided edge to a fresh Const"),
    ("H1b:cont_assigns:&mut", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 1, "before the view: patches a deferred hierarchical element write chunk"),
    ("H1b:cont_assigns:&mut", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, "before the view: patches a deferred hierarchical write chunk"),
    ("H1b:cont_assigns:&mut", "hier_defer/write.rs", "resolve_pending_fill_widths", 1, "before the view: a fill width once the chunk is decided"),
    ("H1b:cont_assigns:index=", "ca_delay_rt.rs", "demote_runtime_delay_on_resolved_nets", 1, "right before the view: a row's delay lane, not its chunks"),
    ("H1b:func_blocks:get_mut", "hier_defer/func_call.rs", "install_deferred_hier_call", 2, "resolve_deferred_hier_call, before the view: a call terminator"),
    ("H1b:funcs:index=", "class_lower.rs", "lower_class_method_body", 1, "run, before the end phase: a frame's entry / sizes"),
    ("H1b:funcs:index=", "frames_body.rs", "lower_frame_body_inner_lower_frame_func_body", 1, "lowering: a frame's entry / sizes"),
    ("H1b:funcs:index=", "frames_body.rs", "lower_frame_body_inner_lower_frame_task_body", 1, "lowering: a frame's entry / sizes"),
    ("H1b:funcs:insert", "expr_size_hier.rs", "module_facts", 1, "another struct's `funcs` map (ModuleFacts), not the IR"),
    ("H1b:proc_idents:&mut", "api.rs", "elaborate_located_params", 1, "after the view: moves the identity table into Sidecars (mem::take), no edit"),
    ("H1b:processes:get_mut", "hier_defer/func_call.rs", "install_deferred_hier_call", 2, "resolve_deferred_hier_call, before the view: a call terminator"),
    ("H1b:processes:index=", "hier.rs", "recompute_comb_sensitivity_after_hier", 1, "before the view: a process's sensitivity"),
    ("H1b:stmts:&mut", "edge_gate.rs", "materialize", 1, "decide_late_edges, before the view: re-points a decided edge to a fresh Const"),
    ("H1b:stmts:&mut", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 1, "before the view: patches a deferred hierarchical element write chunk"),
    ("H1b:stmts:&mut", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, "before the view: patches a deferred hierarchical write chunk"),
    ("H1b:stmts:&mut", "hier_defer/write.rs", "resolve_pending_fill_widths", 1, "before the view: a fill width once the chunk is decided"),
    ("H1b:stmts:insert", "stmt_flow.rs", "push_stmt", 1, "`self.rs.stmts`, the read-set provenance map, not the arena"),
    ("H1b:stmts:retain", "read_set.rs", "recompute_in_body_star_sites", 1, "before the view (R-SETTLE): a block's statement list"),
    ("H1b:stmts:splice", "hier_defer/func_call.rs", "install_deferred_hier_call", 2, "resolve_deferred_hier_task_call, before the view: the copy-out unpacks put at the return block's head (seen since MUT took `splice`, Q-WEXTPOS C1)"),
];

/// H2: the kernel callers.
#[rustfmt::skip]
const H2: &[(&str, &str, &str, usize, &str)] = &[
    ("H2", "netdecl.rs", "check_whole_net_multidriver", 1, "the continuous-row checker (E3001)"),
    ("H2", "wext.rs", "chunk_extent", 1, "the view's Bits"),
    ("H2:at", "wext.rs", "chunk_extent", 1, "the geometry constant's extent: the kernel's width and select step at a recorded offset"),
];

/// H3: built once, handed to `Sidecars`, read by nothing.
#[rustfmt::skip]
const H3: &[(&str, &str, &str, usize, &str)] = &[
    ("H3:build_view", "wext.rs", "build_write_view", 1, "the one build"),
    ("H3:build_write_view", "driver.rs", "run", 1, "the end phase, before check_whole_net_multidriver"),
    ("H3:wext", "api.rs", "elaborate_located_params", 1, "the hand-off to Sidecars::write_view"),
    ("H3:wext", "wext.rs", "build_write_view", 1, "the one build"),
    ("H3:write_view", "api.rs", "elaborate_located_params", 1, "the hand-off to Sidecars::write_view"),
    ("H3:write_view", "tables.rs", "-", 1, "the Sidecars field"),
];

/// H4: the identity tables' readers.
#[rustfmt::skip]
const H4: &[(&str, &str, &str, usize, &str)] = &[
    ("H4:ca_idents", "api.rs", "elaborate_located_params", 2, "the hand-off to Sidecars"),
    ("H4:ca_idents", "cont_array.rs", "other_writers", 1, "reads a row kind (bypass: decides writers without the view, re-point filed)"),
    ("H4:ca_idents", "driver.rs", "new", 1, "initialised empty"),
    ("H4:ca_idents", "lib.rs", "-", 1, "the Elaborator field"),
    ("H4:ca_idents", "proc_ident.rs", "push_cont_assign", 2, "the producer"),
    ("H4:ca_idents", "tables.rs", "-", 1, "the Sidecars field"),
    ("H4:ca_idents", "wext.rs", "-", 1, "WextSource's field"),
    ("H4:ca_idents", "wext.rs", "build_view", 1, "the view's writer kind"),
    ("H4:ca_idents", "wext.rs", "build_write_view", 2, "handed to build_view"),
    ("H4:ca_idents", "wext.rs", "render", 1, "the dump's location"),
    ("H4:proc_idents", "api.rs", "elaborate_located_params", 2, "the hand-off to Sidecars"),
    ("H4:proc_idents", "driver.rs", "new", 1, "initialised empty"),
    ("H4:proc_idents", "hier_defer/func_call.rs", "resolve_deferred_hier_func_call_one", 1, "compares a caller kind to \"always_comb\" (deferred hierarchical call)"),
    ("H4:proc_idents", "lib.rs", "-", 1, "the Elaborator field"),
    ("H4:proc_idents", "stmt_flow.rs", "push_process", 2, "the producer"),
    ("H4:proc_idents", "tables.rs", "-", 1, "the Sidecars field"),
    ("H4:proc_idents", "wext.rs", "-", 1, "WextSource's field"),
    ("H4:proc_idents", "wext.rs", "build_view", 1, "the view's writer kind"),
    ("H4:proc_idents", "wext.rs", "build_write_view", 2, "handed to build_view"),
    ("H4:proc_idents", "wext.rs", "render", 1, "the dump's location"),
];

/// H5: who reads a storing statement's lvalue (patterns and `stmt_lvalue` calls).
#[rustfmt::skip]
const H5: &[(&str, &str, &str, usize, &str)] = &[
    ("H5:BlockingAssign", "ast_query.rs", "collect_block_reads", 1, "a block's reads (an lvalue's index expressions), no writer decided"),
    ("H5:BlockingAssign", "cont_array.rs", "other_writers", 1, "the listed bypass: decides a net's other writers without the view (re-point filed, Q-WREXT)"),
    ("H5:BlockingAssign", "dead_arm.rs", "edges_held_by", 1, "the part-select widths a target holds, no writer decided"),
    ("H5:BlockingAssign", "frames_classify.rs", "classify_frame_body", 1, "which nets a frame body writes (frame routing)"),
    ("H5:BlockingAssign", "frames_classify.rs", "frame_task_has_unsafe_construct", 1, "a frame task's write classes (frame routing)"),
    ("H5:BlockingAssign", "frames_classify_fork.rs", "classify_one_arm", 1, "a fork arm's writes (owned window)"),
    ("H5:BlockingAssign", "frames_classify_write.rs", "frame_outside_writes_are_user_nets", 1, "a frame's outside writes are user nets (routing veto)"),
    ("H5:BlockingAssign", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 1, "patches a chunk, decides no writer"),
    ("H5:BlockingAssign", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, "patches a chunk, decides no writer"),
    ("H5:BlockingAssign", "hier_defer/write.rs", "resolve_pending_fill_widths", 1, "patches a fill width, decides no writer"),
    ("H5:BlockingAssign", "read_set.rs", "build_ref_write_map", 1, "the read set's write collector (ref formals)"),
    ("H5:BlockingAssign", "read_set.rs", "mark_compound_lvalue_copy", 1, "the read set's compound-assignment copy"),
    ("H5:BlockingAssign", "read_set.rs", "rs_block", 1, "the read set's block walk"),
    ("H5:BlockingAssign", "stmt_flow.rs", "stmt_lvalue", 1, "the shared accessor (its four arms)"),
    ("H5:BlockingAssign", "wext.rs", "store_of", 1, "the view's four forms"),
    ("H5:Force", "ast_query.rs", "collect_block_reads", 1, "a block's reads (an lvalue's index expressions), no writer decided"),
    ("H5:Force", "cont_array.rs", "other_writers", 1, "the listed bypass: decides a net's other writers without the view (re-point filed, Q-WREXT)"),
    ("H5:Force", "dead_arm.rs", "edges_held_by", 1, "the part-select widths a target holds, no writer decided"),
    ("H5:Force", "frames_classify.rs", "classify_frame_body", 1, "which nets a frame body writes (frame routing)"),
    ("H5:Force", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 2, "patches a chunk, decides no writer"),
    ("H5:Force", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, "patches a chunk, decides no writer"),
    ("H5:Force", "hier_defer/write.rs", "resolve_pending_fill_widths", 1, "patches a fill width, decides no writer"),
    ("H5:Force", "read_set.rs", "rs_block", 1, "the read set's block walk"),
    ("H5:Force", "stmt_flow.rs", "stmt_lvalue", 1, "the shared accessor (its four arms)"),
    ("H5:Force", "wext.rs", "store_of", 1, "the view's four forms"),
    ("H5:NonblockingAssign", "ast_query.rs", "collect_block_reads", 1, "a block's reads (an lvalue's index expressions), no writer decided"),
    ("H5:NonblockingAssign", "cont_array.rs", "other_writers", 1, "the listed bypass: decides a net's other writers without the view (re-point filed, Q-WREXT)"),
    ("H5:NonblockingAssign", "dead_arm.rs", "edges_held_by", 1, "the part-select widths a target holds, no writer decided"),
    ("H5:NonblockingAssign", "frames_classify.rs", "classify_frame_body", 1, "which nets a frame body writes (frame routing)"),
    ("H5:NonblockingAssign", "frames_classify.rs", "frame_task_has_unsafe_construct", 1, "a frame task's write classes (frame routing)"),
    ("H5:NonblockingAssign", "frames_classify_fork.rs", "classify_one_arm", 1, "a fork arm's writes (owned window)"),
    ("H5:NonblockingAssign", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 1, "patches a chunk, decides no writer"),
    ("H5:NonblockingAssign", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, "patches a chunk, decides no writer"),
    ("H5:NonblockingAssign", "hier_defer/write.rs", "resolve_pending_fill_widths", 1, "patches a fill width, decides no writer"),
    ("H5:NonblockingAssign", "read_set.rs", "build_ref_write_map", 1, "the read set's write collector (ref formals)"),
    ("H5:NonblockingAssign", "read_set.rs", "rs_block", 1, "the read set's block walk"),
    ("H5:NonblockingAssign", "stmt_flow.rs", "stmt_lvalue", 1, "the shared accessor (its four arms)"),
    ("H5:NonblockingAssign", "wext.rs", "store_of", 1, "the view's four forms"),
    ("H5:Release", "ast_query.rs", "collect_block_reads", 1, "a block's reads (an lvalue's index expressions), no writer decided"),
    ("H5:Release", "cont_array.rs", "other_writers", 1, "the listed bypass: decides a net's other writers without the view (re-point filed, Q-WREXT)"),
    ("H5:Release", "dead_arm.rs", "edges_held_by", 1, "the part-select widths a target holds, no writer decided"),
    ("H5:Release", "frames_classify.rs", "classify_frame_body", 1, "which nets a frame body writes (frame routing)"),
    ("H5:Release", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 2, "patches a chunk, decides no writer"),
    ("H5:Release", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, "patches a chunk, decides no writer"),
    ("H5:Release", "read_set.rs", "rs_block", 1, "the read set's block walk"),
    ("H5:Release", "stmt_flow.rs", "stmt_lvalue", 1, "the shared accessor (its four arms)"),
    ("H5:Release", "wext.rs", "store_of", 1, "the view's four forms"),
    ("H5:stmt_lvalue", "stmt_flow.rs", "push_stmt", 1, "an array-word write wants a location"),
];

/// H6: the write-shape producers (constructions; `vec ! [..]` bodies included).
#[rustfmt::skip]
const H6: &[(&str, &str, &str, usize, &str)] = &[
    ("H6:ContAssign", "const_level_header.rs", "t0_pulse_net", 1, ""),
    ("H6:ContAssign", "cont_array.rs", "cont_assign_whole_array", 1, ""),
    ("H6:ContAssign", "events.rs", "level_select_net", 1, ""),
    ("H6:ContAssign", "netdecl.rs", "elaborate_cont_assign", 1, ""),
    ("H6:ContAssign", "ports.rs", "wire_array_port", 1, ""),
    ("H6:ContAssign", "ports.rs", "wire_ports", 2, ""),
    ("H6:ContAssign", "var_init.rs", "elaborate_net_init_drivers", 1, ""),
    ("H6:LvalChunk", "arrays.rs", "array_assign_special", 1, ""),
    ("H6:LvalChunk", "arrays.rs", "collect_array_write", 4, ""),
    ("H6:LvalChunk", "arrays/pattern.rs", "lower_array_pattern_window", 1, ""),
    ("H6:LvalChunk", "cont_array.rs", "cont_assign_whole_array", 1, ""),
    ("H6:LvalChunk", "dynarr_method.rs", "lower_dyn_method_stmt", 1, ""),
    ("H6:LvalChunk", "events.rs", "capture_lvalue_indices_now", 1, ""),
    ("H6:LvalChunk", "hier.rs", "build_hier_sel_write_chunk", 5, ""),
    ("H6:LvalChunk", "hier_defer/task_call.rs", "build_hier_array_unpacks", 1, ""),
    ("H6:LvalChunk", "hier_defer/write.rs", "resolve_deferred_hier_sel_write", 1, ""),
    ("H6:LvalChunk", "hier_defer/write.rs", "resolve_deferred_hier_write", 1, ""),
    ("H6:LvalChunk", "limits.rs", "poison_chunk", 1, ""),
    ("H6:LvalChunk", "lvalue.rs", "collect_lval_chunks", 12, ""),
    ("H6:LvalChunk", "lvalue.rs", "is_whole_single_net", 1, ""),
    ("H6:LvalChunk", "lvalue.rs", "poison_chunk", 1, ""),
    ("H6:LvalChunk", "lvalue.rs", "whole_net_chunk", 1, ""),
    ("H6:LvalChunk", "packed_lval.rs", "collect_packed_write", 3, ""),
    ("H6:LvalChunk", "packed_lval.rs", "try_nested_packed_part_lval", 1, ""),
    ("H6:LvalChunk", "packed_lval.rs", "try_packed_indexed_part_lval", 2, ""),
    ("H6:LvalChunk", "packed_lval.rs", "try_packed_part_select_lval", 2, ""),
    ("H6:LvalChunk", "ports.rs", "wire_array_port", 1, ""),
    ("H6:LvalChunk", "static_array_method.rs", "lower_static_array_order", 1, ""),
];

/// H7: the geometry constant's writers and the C2 records' raw store.
#[rustfmt::skip]
const H7: &[(&str, &str, &str, usize, &str)] = &[
    ("H7:carry_geom", "packed_inner.rs", "packed_inner_range", 1, "`prefix + rel`: an element range of an inner dimension"),
    ("H7:carry_geom", "packed_lval.rs", "try_nested_packed_part_lval", 1, "`base_off + l`: a part-select under an element (a struct member)"),
    ("H7:record_geom", "array_geom.rs", "flatten_word", 1, "the flat-offset tree's constant"),
    ("H7:record_geom", "array_geom.rs", "flatten_word_eids", 1, "its eid twin's constant"),
    ("H7:wext_rec", "wext.rs", "build_write_view", 2, "taken once into the view ; then marked built (the funnel's debug order check)"),
    ("H7:wext_rec", "wext.rs", "carry_geom", 2, "the geometry writer (read base, insert)"),
    ("H7:wext_rec", "wext.rs", "patch_expr", 2, "the patch funnel: its debug order check and the stand-down's setter"),
    ("H7:wext_rec", "wext.rs", "patch_expr_with", 2, "the patch funnel: its debug order check and the stand-down's setter"),
    ("H7:wext_rec", "wext.rs", "record_decl_fact", 1, "the declaration-fact writer"),
    ("H7:wext_rec", "wext.rs", "record_geom", 1, "the geometry writer"),
    ("H7:wext_rec", "wext.rs", "record_via", 1, "the via writer"),
    ("H7:wext_rec", "wext.rs", "register_provisional_const", 1, "the stand-down's registration"),
];

/// H8: the declaration fact's writer, the sites and scopes passed to it, and what it reads.
#[rustfmt::skip]
const H8: &[(&str, &str, &str, usize, &str)] = &[
    ("H8:DeclScope::BlockLocal", "block_local/hoist.rs", "hoist_one_block_local", 2, "a block local: no fact"),
    ("H8:DeclScope::BlockLocal", "wext.rs", "body_site", 1, "-> None"),
    ("H8:DeclScope::Generate", "generate.rs", "lower_gen_module_item", 1, "a generate-scope declaration"),
    ("H8:DeclScope::Generate", "wext.rs", "body_site", 1, "-> Generate"),
    ("H8:DeclScope::Iface", "iface_inst.rs", "elaborate_iface_instances", 2, "an interface member and an interface port: no fact"),
    ("H8:DeclScope::Iface", "wext.rs", "body_site", 1, "-> None"),
    ("H8:DeclScope::Module", "instance.rs", "elaborate_instance", 2, "a module body declaration and a module's ANSI ports"),
    ("H8:DeclScope::Module", "ports.rs", "elaborate_ports", 1, "a module's ANSI port -> AnsiPort"),
    ("H8:DeclScope::Module", "wext.rs", "body_site", 1, "-> Module"),
    ("H8:DeclScope::Package", "package.rs", "elaborate_pkg_netvar", 1, "a package member: no fact"),
    ("H8:DeclScope::Package", "wext.rs", "body_site", 1, "-> None"),
    ("H8:DeclSite::AnsiPort", "ports.rs", "elaborate_ports", 1, "the ANSI port writer"),
    ("H8:DeclSite::Generate", "wext.rs", "body_site", 1, "the body writer's generate site"),
    ("H8:DeclSite::Module", "wext.rs", "body_site", 1, "the body writer's module site"),
    ("H8:DeclSite::NonAnsiPort", "instance.rs", "elaborate_instance", 1, "the non-ANSI port writer"),
    ("H8:record_decl_fact", "instance.rs", "elaborate_instance", 1, "writer 3: a non-ANSI port net made from its port declaration"),
    ("H8:record_decl_fact", "netdecl.rs", "elaborate_netvar_decl_inner", 1, "writer 1: a body declaration (site from DeclScope)"),
    ("H8:record_decl_fact", "ports.rs", "elaborate_ports", 1, "writer 2: an ANSI port"),
];

/// H8 (reads): `intro_kind` / `net_names` in the fact's host fns and in wext.rs.
#[rustfmt::skip]
const H8R: &[(&str, &str, &str, usize, &str)] = &[
    ("R8:intro_kind", "instance.rs", "elaborate_instance", 1, "a WRITE (`intro_kind.insert`) after the fact, for two_state_nets: not read by it"),
    ("R8:net_names", "wext.rs", "render", 1, "the dump's net names: not the fact"),
];

/// H9: the `via` intervals' writer and the values passed to it.
#[rustfmt::skip]
const H9: &[(&str, &str, &str, usize, &str)] = &[
    ("H9:Via::DeferAction", "stmt_main.rs", "lower_stmt_inner", 1, "the deferred assertion's action arms"),
    ("H9:Via::DeferAction", "wext.rs", "build_view", 1, "laid over task intervals"),
    ("H9:Via::DeferAction", "wext.rs", "render", 1, "`defer`"),
    ("H9:Via::Direct", "wext.rs", "build_view", 2, "the default of a procedural statement"),
    ("H9:Via::Direct", "wext.rs", "render", 1, "`direct`"),
    ("H9:Via::TaskInline", "inline_task.rs", "inline_task", 1, "the inline decision"),
    ("H9:Via::TaskInline", "wext.rs", "build_view", 1, "laid first"),
    ("H9:Via::TaskInline", "wext.rs", "render", 1, "`task`"),
    ("H9:record_via", "inline_task.rs", "inline_task", 1, "around inline_static_task"),
    ("H9:record_via", "stmt_main.rs", "lower_stmt_inner", 1, "around the action arms"),
];

/// H11: the `exprs` arena's writers — the patch funnel and its 11 callers (the end
/// phase's resolvers, ground Q2-B's 11 slot writers), the append funnel, the one listed
/// hand-off, and the read set's provenance map `rs.exprs` (another receiver).
#[rustfmt::skip]
const H11: &[(&str, &str, &str, usize, &str)] = &[
    ("H11:call:patch_expr", "hier_defer/read.rs", "resolve_deferred_hier", 1, "a readmem assoc marker over a Signal placeholder"),
    ("H11:call:patch_expr", "hier_defer/read.rs", "resolve_one_deferred_hier_sel", 4, "a built select copied over a Signal placeholder"),
    ("H11:call:patch_expr_with", "hier_defer/bits.rs", "resolve_deferred_hier_bits", 1, "the registered $bits Const 32 -> the width (the one Const overwrite)"),
    ("H11:call:patch_expr_with", "hier_defer/read.rs", "resolve_deferred_hier", 2, "a wide param Const over a Signal placeholder ; the placeholder's net id"),
    ("H11:call:patch_expr_with", "hier_defer/read.rs", "resolve_deferred_hier_call", 1, "the placeholder Call's func id"),
    ("H11:call:patch_expr_with", "param_query.rs", "patch_expr_param_const", 1, "a param Const over a Signal placeholder"),
    ("H11:call:patch_expr_with", "param_query.rs", "patch_expr_param_const_w", 1, "a typed param Const over a Signal placeholder"),
    ("H11:exprs:&mut", "edge_gate.rs", "materialize", 1, "the listed hand-off: for_each_constant_edge_mut repoints edge fields, no root slot"),
    ("H11:exprs:&mut", "wext.rs", "patch_expr", 1, "the patch funnel (value form)"),
    ("H11:exprs:get_mut", "wext.rs", "patch_expr_with", 1, "the patch funnel (edit form)"),
    ("H11:exprs:push", "expr_ctx.rs", "push_expr", 1, "the append funnel"),
    ("H11:rs.exprs:entry", "read_set.rs", "mark_compound_lvalue_copy", 1, "the read set's provenance map, not the arena"),
    ("H11:rs.exprs:insert", "expr_ctx.rs", "push_expr", 1, "the read set's provenance map, not the arena"),
];

/// H11b: the stand-down's setters and the registered set's insert.
#[rustfmt::skip]
const H11B: &[(&str, &str, &str, usize, &str)] = &[
    ("H11b:note_patch", "wext.rs", "patch_expr", 1, "the funnel, value form"),
    ("H11b:note_patch", "wext.rs", "patch_expr_with", 1, "the funnel, edit form"),
    ("H11b:note_provisional", "wext.rs", "register_provisional_const", 1, "the registration"),
    ("H11b:register_provisional_const", "expr_special.rs", "lower_bits_fold", 1, "$bits of a hierarchical name: the one value-guess Const"),
    ("H11b:undecided_placeholders.insert", "wext.rs", "register_provisional_const", 1, "the registered set, inside the registration"),
];

/// H11c: (rule, file, fn, count, the node is an `exprs` `Const` the end phase replaces:
/// y / n / no slot, registered: y / "no: <why>" / "-" when not a `Const`). Sources: the
/// grounding's Q2-A and NM-5 tables (W/wextpos/ground, ground2), each site re-read.
#[rustfmt::skip]
const H11C: &[(&str, &str, &str, usize, &str, &str)] = &[
    ("H11c:deferred_assert_items:push", "assert_item_gate.rs", "note_deferred_assert_item", 1, "no slot", "-"),
    ("H11c:deferred_captures:push", "held.rs", "record_deferred_capture", 1, "no slot", "-"),
    ("H11c:deferred_hier:push", "expr_main.rs", "lower_expr_ungated", 1, "n", "-"),
    ("H11c:deferred_hier_bits:push", "expr_special.rs", "lower_bits_fold", 1, "y", "y"),
    ("H11c:deferred_hier_calls:push", "inline_fn.rs", "inline_function", 1, "n", "-"),
    ("H11c:deferred_hier_sel:push", "expr_main.rs", "lower_expr_ungated", 3, "n", "-"),
    ("H11c:deferred_hier_sel_write:push", "hier_defer/write.rs", "defer_hier_sel_write", 1, "no slot", "-"),
    ("H11c:deferred_hier_task_calls:push", "frames_body.rs", "lower_frame_body_inner_lower_frame_task_body", 1, "no slot", "-"),
    ("H11c:deferred_hier_task_calls:push", "hier_defer/func_call.rs", "emit_deferred_hier_func_call", 1, "no slot", "-"),
    ("H11c:deferred_hier_task_calls:push", "inline_task.rs", "inline_task", 1, "no slot", "-"),
    ("H11c:deferred_hier_write:push", "hier_defer/write.rs", "defer_hier_write", 1, "no slot", "-"),
    ("H11c:error_placeholders:insert", "expr_ctx.rs", "placeholder_expr", 1, "y", "no: a Const 0 after an error only (had_error latched: no fact, the reader's guard)"),
    ("H11c:error_placeholders:insert", "packed.rs", "lower_index_expr_inner", 2, "y", "no: a Const 0 index after an error only (had_error latched: no fact, the reader's guard)"),
    ("H11c:pending_block_local_inits:entry", "var_init.rs", "push_block_local_init", 1, "no slot", "-"),
    ("H11c:pending_block_local_inits:insert", "var_init.rs", "flush_block_local_inits", 1, "no slot", "-"),
    ("H11c:pending_calls:push", "read_set.rs", "note_call_site_mark", 1, "no slot", "-"),
    ("H11c:pending_cover:push", "stmt_main.rs", "lower_stmt_inner", 1, "no slot", "-"),
    ("H11c:pending_fill_width:push", "expr_ctx.rs", "resize_rhs_for_lvalue", 1, "y", "no: an rhs, repointed by resolve_pending_fill_widths, never patched in its slot ; no lvalue chunk reads it"),
    ("H11c:pending_fork_modes:push", "instance.rs", "record_fork_mode", 1, "no slot", "-"),
    ("H11c:pending_hier_task_calls:push", "hier_defer/func_call.rs", "emit_deferred_hier_func_call", 1, "no slot", "-"),
    ("H11c:pending_hier_task_calls:push", "inline_task.rs", "inline_task", 1, "no slot", "-"),
    ("H11c:pending_scoped_presize:entry", "string_array_route.rs", "route_fixed_string_array", 1, "no slot", "-"),
    ("H11c:pending_scoped_presize:insert", "var_init.rs", "drain_scoped_presize", 1, "no slot", "-"),
    ("H11c:pending_sva:push", "stmt_main.rs", "lower_stmt_inner", 4, "no slot", "-"),
    ("H11c:pending_task_calls:push", "frames_call/emit.rs", "emit_frame_func_out_call", 1, "no slot", "-"),
    ("H11c:pending_task_calls:push", "frames_call/emit.rs", "emit_frame_task_call", 1, "no slot", "-"),
    ("H11c:pending_var_inits:extend", "var_init.rs", "collect_var_init_drivers", 1, "no slot", "-"),
    ("H11c:pending_var_inits:extend", "var_init.rs", "flush_block_local_inits", 1, "no slot", "-"),
    ("H11c:pending_var_inits:push", "var_init.rs", "collect_var_init_drivers", 1, "no slot", "-"),
    ("H11c:pending_var_inits:splice", "var_init.rs", "drain_scoped_presize", 1, "no slot", "-"),
    ("H11c:undecided_placeholders:insert", "wext.rs", "register_provisional_const", 1, "y", "y"),
];
