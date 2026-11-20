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
//!   store `. wext_rec` inside wext.rs's writers and `build_write_view` only. §5.2
//!   Q-WEXTPOS C2: `record_row_at (` once, in `push_cont_assign` (each continuous row's
//!   own position, pushed beside the row). §5.2 Q-WEXTPOS C3: `record_stmt_at (` once, in
//!   `lower_stmt`, keyed by its first argument as spelled (`s . span ( )`, the fn's own
//!   parameter) ; the mask `wext_unlocated (`, the body roots `wext_body_enter (` /
//!   `wext_body_enter_keep (` / `wext_body_exit (` (enters and exits equal per (file, fn))
//!   and the callee entries `wext_located (`, per (file, fn) ; no `cur_span` token in
//!   wext.rs. Fix F1 (V): each body root keyed by its first argument as spelled, the span of
//!   the ITEM its body belongs to (`p . span`, `func . span`, `task . span`, the method's
//!   `item`), never the body's (premise judge L8).
//! - H14 (§5.2 Q-WEXTPOS C3) every `lower_stmt (` call, any receiver, per (file, fn), as
//!   `bare` or `masked` (inside a `wext_unlocated (` argument group), with its class
//!   written by hand: R (a parsed statement) is bare, S (one vita built) masked ; PLAN B7's
//!   27 R + 1 MIXED (R) + 19 S. And the process root `lower_proc_block` keeps the mask
//!   (`wext_body_enter_keep`), never zeroes it (premise judge round 1, line 1 and P-1).
//! - H13 (§5.2 Q-WEXTPOS C2) the `at` argument of every `push_cont_assign (` /
//!   `push_cont_assign_port (` call (its last argument), as spelled inline, per (file, fn):
//!   an `assign` row passes its own target `Some ( lv . span ( ) )`, a port row the
//!   connection's span, the kinds no reader prints `None`. A new caller, or a spelling
//!   that moves (a context's span, a span bound to a local first), fails here.
//! - H8 (C2) the declaration fact: `record_decl_fact (` in its three writer fns, the
//!   `DeclScope :: <V>` each `elaborate_netvar_decl` / `elaborate_ports` caller passes and
//!   the `DeclSite :: <V>` each writer names, per (file, fn) ; and (R8) no `intro_kind` /
//!   `net_names` token in a writer's host fn or in wext.rs but the listed two.
//! - H9 (C2) the `via` intervals: `record_via (` at the inline decision and around the
//!   deferred action, and each `Via :: <V>` spelling.
//! - H15 (§5.2 Q-WEXTPOS fix F1) every CONSTRUCTION of a node whose span a write's `at`
//!   can read — `ast :: Stmt :: <V> {` / `(` (an `ir` path left out, a `use .. :: Stmt as X`
//!   alias read per file and fn, a glob's bare variant), `PortConn {`, `ConnSite {`,
//!   `ContinuousAssign {`, `InstanceItem {`, `PortConnList :: <V> (` — not a pattern (H5's
//!   discriminator, climbing out of a tuple or a slice of patterns), keyed by its `span`
//!   field as spelled, with a hand-written class (OWN / FROM / MASKED / NOSTORE / V / NONE).
//!   A new construction, or a spelling that moves, fails: say whose text the span is.
//! - H15v (fix F1) the connection-span assertion: `check_conn_spans (` once, in
//!   `wire_ports`, and `conn_span_violations (` once, in it (P5's (i) two ports of one
//!   instance on one connection span, (ii) a connection at its instance item's span).
//! - H15p (fix F1) the same census over hdl-parser's production sources (the parser builds
//!   most statements ; its ANOTHER rows are the bodies V turns to `None`).
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
//! a type alias of `Stmt` (H15 reads a `use .. as` alias, not a `type` alias nor a grouped
//! `use p :: { .. }`), a construction through `Self { .. }` inside the type's own `impl`,
//! a construction through a helper (`whole_net_lvalue`,
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

/// The last top-level argument of the call whose `(` is at `open`, its tokens joined with
/// no space (a trailing comma ignored).
fn last_arg(t: &[Tok], open: usize) -> String {
    let close = close_of(t, open);
    let mut args: Vec<String> = vec![String::new()];
    let mut d = 0usize;
    for x in &t[open + 1..close] {
        if x.kind == Kind::P && matches!(x.text.as_str(), "(" | "[" | "{") {
            d += 1;
        } else if x.kind == Kind::P && matches!(x.text.as_str(), ")" | "]" | "}") {
            d -= 1;
        } else if d == 0 && x.text == "," {
            args.push(String::new());
            continue;
        }
        args.last_mut().expect("one arg").push_str(&x.text);
    }
    while args.len() > 1 && args.last().is_some_and(|a| a.is_empty()) {
        args.pop();
    }
    args.pop().unwrap_or_default()
}

/// The first top-level argument of the call whose `(` is at `open`, its tokens joined with
/// no space.
fn first_arg(t: &[Tok], open: usize) -> String {
    let close = close_of(t, open);
    let mut s = String::new();
    let mut d = 0usize;
    for x in &t[open + 1..close] {
        if x.kind == Kind::P && matches!(x.text.as_str(), "(" | "[" | "{") {
            d += 1;
        } else if x.kind == Kind::P && matches!(x.text.as_str(), ")" | "]" | "}") {
            d -= 1;
        } else if d == 0 && x.text == "," {
            break;
        }
        s.push_str(&x.text);
    }
    s
}

/// (rule, fn key) hits of one source.
fn hits(src: &str) -> Vec<(String, String)> {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let mut out: Vec<(String, String)> = Vec::new();
    // H14: the argument groups of every `wext_unlocated (` call (THE MASK, Q-WEXTPOS C3)
    let masks: Vec<(usize, usize)> = (0..t.len())
        .filter(|&i| tx(i) == "wext_unlocated" && tx(i + 1) == "(" && (i == 0 || tx(i - 1) != "fn"))
        .map(|i| (i + 1, close_of(&t, i + 1)))
        .collect();
    // H15 (fix F1): the `ast::Stmt` imports of this source, per fn key
    let (stmt_alias, stmt_glob) = stmt_imports(&t);
    for i in 0..t.len() {
        let k = t[i].key.clone();
        let id = t[i].kind == Kind::Id;
        // H15 / H15p (fix F1): every CONSTRUCTION of a node whose span can become a write's
        // `at`, keyed by its kind and its `span` field as spelled
        if let Some(kind) = node_ctor_at(&t, i, &stmt_alias, &stmt_glob) {
            let open = if matches!(tx(i + 1), "{" | "(") {
                i + 1
            } else {
                i + 3
            };
            out.push((format!("H15:{kind}:{}", span_field(&t, open)), k.clone()));
        }
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
                // H15v (fix F1): the connection-span assertion's site and its check
                "check_conn_spans" | "conn_span_violations" => {
                    out.push((format!("H15v:{}", tx(i)), k.clone()))
                }
                "record_geom" | "carry_geom" | "record_row_at" | "wext_unlocated"
                | "wext_located" | "wext_body_exit" => {
                    out.push((format!("H7:{}", tx(i)), k.clone()))
                }
                // H7 (fix F1, V): a body root keyed by its item argument as spelled
                "wext_body_enter" | "wext_body_enter_keep" => {
                    out.push((format!("H7:{}:{}", tx(i), first_arg(&t, i + 1)), k.clone()))
                }
                // H7 (C3): the statement recorder, keyed by its first argument as spelled
                "record_stmt_at" => out.push((
                    format!("H7:record_stmt_at:{}", first_arg(&t, i + 1)),
                    k.clone(),
                )),
                // H14 (C3): every `lower_stmt (` call, any receiver, inside the mask or not
                "lower_stmt" => {
                    let masked = masks.iter().any(|&(a, b)| a < i && i < b);
                    let m = if masked { "masked" } else { "bare" };
                    out.push((format!("H14:lower_stmt:{m}"), k.clone()))
                }
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
        // H13 the `at` argument (the last one) of a continuous append call, spelled inline
        if id
            && matches!(tx(i), "push_cont_assign" | "push_cont_assign_port")
            && tx(i + 1) == "("
            && (i == 0 || tx(i - 1) != "fn")
        {
            out.push((format!("H13:{}:{}", tx(i), last_arg(&t, i + 1)), k.clone()));
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
        // H7 (C3): the context's span, which the recorder must not read
        if id && tx(i) == "cur_span" {
            out.push(("R7:cur_span".into(), k.clone()));
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

/// H15 / H15p (fix F1): the watched node structs (a construction of one makes a span a
/// continuous row's `at` can read).
const H15_STRUCTS: [&str; 4] = ["PortConn", "ConnSite", "ContinuousAssign", "InstanceItem"];

/// `ast::Stmt`'s variants (hdl-ast `pub enum Stmt`), read from the source, with whether it
/// is a tuple variant.
fn stmt_variants() -> Vec<(String, bool)> {
    let root = workspace_root();
    let src = std::fs::read_to_string(root.join("crates/hdl-ast/src/lib.rs")).expect("hdl-ast");
    let t = tokens(&src);
    let at = (0..t.len())
        .find(|&i| t[i].text == "enum" && t[i + 1].text == "Stmt" && t[i + 2].text == "{")
        .expect("pub enum Stmt");
    let close = close_of(&t, at + 2);
    let mut v = Vec::new();
    let mut j = at + 3;
    while j < close {
        if t[j].kind == Kind::Id && (j == at + 3 || t[j - 1].text == ",") {
            v.push((t[j].text.clone(), t[j + 1].text == "("));
        }
        if matches!(t[j].text.as_str(), "(" | "{" | "[") {
            j = close_of(&t, j) + 1;
        } else {
            j += 1;
        }
    }
    v
}

/// H15: the `use <path> :: Stmt as <X> ;` aliases and the `use <path> :: Stmt :: * ;`
/// globs of one source, per fn key (`-` = the whole file), an `ir` / `sim_ir` path left
/// out. A grouped `use p :: { .. }` is not read (none today: named in the head note).
fn stmt_imports(t: &[Tok]) -> (BTreeMap<String, Vec<String>>, Vec<String>) {
    let mut alias: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut glob = Vec::new();
    for i in 0..t.len() {
        if !(t[i].kind == Kind::Id && t[i].text == "use") {
            continue;
        }
        let Some(end) = (i..t.len()).find(|&j| t[j].text == ";") else {
            continue;
        };
        let w: Vec<&str> = t[i + 1..end].iter().map(|x| x.text.as_str()).collect();
        let ir_path = w.iter().any(|s| *s == "ir" || *s == "sim_ir");
        let n = w.len();
        if n >= 3 && w[n - 3] == "Stmt" && w[n - 2] == "as" && !ir_path {
            alias
                .entry(t[i].key.clone())
                .or_default()
                .push(w[n - 1].to_string());
        }
        if n >= 3 && w[n - 3] == "Stmt" && w[n - 2] == "::" && w[n - 1] == "*" && !ir_path {
            glob.push(t[i].key.clone());
        }
    }
    (alias, glob)
}

/// H15: the kind of the node CONSTRUCTED at token `i`, when one is (a pattern, a type's own
/// item, a return type, an `ir` path or a method name is not): `Stmt` for `<path> :: Stmt
/// :: <V> {` / `(` (or an alias of `Stmt`, or a bare variant under a glob import of the same
/// fn), a struct of `H15_STRUCTS` for `<Struct> {`, `PortConnList` for `PortConnList :: <V>
/// (`. Closure heads (`|sp| Stmt :: Null ( sp )`) are not a pattern: the discriminator
/// reads what FOLLOWS the group (H5's), climbing out of a tuple or a slice.
fn node_ctor_at(
    t: &[Tok],
    i: usize,
    alias: &BTreeMap<String, Vec<String>>,
    glob: &[String],
) -> Option<String> {
    let tx = |j: usize| t.get(j).map_or("", |x| x.text.as_str());
    if t[i].kind != Kind::Id || (i >= 1 && tx(i - 1) == ".") {
        return None;
    }
    let key = &t[i].key;
    let aliased = |name: &str| {
        [key.as_str(), "-"]
            .iter()
            .any(|k| alias.get(*k).is_some_and(|v| v.iter().any(|a| a == name)))
    };
    let upper = |j: usize| {
        t.get(j).is_some_and(|x| {
            x.kind == Kind::Id && x.text.starts_with(|c: char| c.is_ascii_uppercase())
        })
    };
    let (kind, open) = if (tx(i) == "Stmt" || aliased(tx(i)))
        && tx(i + 1) == "::"
        && upper(i + 2)
        && matches!(tx(i + 3), "{" | "(")
    {
        let ir =
            tx(i) == "Stmt" && i >= 2 && tx(i - 1) == "::" && matches!(tx(i - 2), "ir" | "sim_ir");
        if ir {
            return None;
        }
        ("Stmt".to_string(), i + 3)
    } else if tx(i) == "PortConnList" && tx(i + 1) == "::" && upper(i + 2) && tx(i + 3) == "(" {
        ("PortConnList".to_string(), i + 3)
    } else if H15_STRUCTS.contains(&tx(i)) && tx(i + 1) == "{" {
        let mut j = i;
        while j >= 2 && tx(j - 1) == "::" && t[j - 2].kind == Kind::Id {
            j -= 2;
        }
        let ret = j >= 2 && tx(j - 1) == ">" && tx(j - 2) == "-";
        let item = j >= 1
            && matches!(
                tx(j - 1),
                "struct" | "enum" | "impl" | "for" | "trait" | "type"
            );
        if ret || item {
            return None;
        }
        (tx(i).to_string(), i + 1)
    } else if (glob.iter().any(|g| g == key || g == "-"))
        && (i == 0 || tx(i - 1) != "::")
        && STMT_VARIANTS.with(|v| {
            v.iter()
                .any(|(n, tuple)| n == tx(i) && (tx(i + 1) == "{" || (*tuple && tx(i + 1) == "(")))
        })
    {
        ("Stmt".to_string(), i + 1)
    } else {
        return None;
    };
    if ctor_is_pattern(t, open) {
        return None;
    }
    Some(kind)
}

thread_local! {
    static STMT_VARIANTS: Vec<(String, bool)> = stmt_variants();
}

/// H15: the group opened at `open` is a PATTERN: H5's discriminator (`=>`, `|`, `=`, `if`
/// after it, or inside `matches ! (`), or, when a `,` / `)` / `]` follows it, the same test
/// on the enclosing tuple or slice (`(A { .. }, B { .. }) =>`), never across a `{`.
fn ctor_is_pattern(t: &[Tok], open: usize) -> bool {
    if is_pattern(t, open) {
        return true;
    }
    let mut o = open;
    loop {
        let c = close_of(t, o);
        if !t
            .get(c + 1)
            .is_some_and(|n| matches!(n.text.as_str(), "," | ")" | "]"))
        {
            return false;
        }
        let Some(e) = enclosing(t, o) else {
            return false;
        };
        if t[e].text == "{" {
            return false;
        }
        if is_pattern(t, e) {
            return true;
        }
        o = e;
    }
}

/// The innermost group opener holding token `j`, if any.
fn enclosing(t: &[Tok], j: usize) -> Option<usize> {
    let mut d = 0i32;
    let mut k = j;
    while k > 0 {
        k -= 1;
        if t[k].kind != Kind::P {
            continue;
        }
        match t[k].text.as_str() {
            ")" | "]" | "}" => d += 1,
            "(" | "[" | "{" => {
                if d == 0 {
                    return Some(k);
                }
                d -= 1;
            }
            _ => {}
        }
    }
    None
}

/// H15: the `span` a construction spells — a `{ .. }` group's top-level `span : <expr>`
/// field (`span` for the shorthand, `..<rest>` when only a functional update gives it, `-`
/// when it has none), a `( .. )` group's first argument.
fn span_field(t: &[Tok], open: usize) -> String {
    if t[open].text == "(" {
        return first_arg(t, open);
    }
    let close = close_of(t, open);
    let mut fields: Vec<Vec<&str>> = vec![Vec::new()];
    let mut d = 0usize;
    for x in &t[open + 1..close] {
        if x.kind == Kind::P && matches!(x.text.as_str(), "(" | "[" | "{") {
            d += 1;
        } else if x.kind == Kind::P && matches!(x.text.as_str(), ")" | "]" | "}") {
            d -= 1;
        } else if d == 0 && x.text == "," {
            fields.push(Vec::new());
            continue;
        }
        fields.last_mut().expect("one field").push(&x.text);
    }
    let mut rest = None;
    for f in &fields {
        if f.first() == Some(&"span") {
            return if f.len() == 1 {
                "span".to_string()
            } else {
                f[2..].concat()
            };
        }
        if f.first() == Some(&".") && f.get(1) == Some(&".") {
            rest = Some(f.concat());
        }
    }
    rest.unwrap_or_else(|| "-".to_string())
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

/// H15p (fix F1): the H15 rows of the PARSER's production sources (`crates/hdl-parser/src`,
/// its `src/tests/` left out as `census` leaves elaborate's), keyed (rule, file, fn).
fn parser_census() -> Table {
    let root = workspace_root();
    let mut tab = Table::new();
    for (rel, src) in files_in(&root, "crates/hdl-parser/src") {
        let f = rel.trim_start_matches("crates/hdl-parser/src/").to_string();
        if f.starts_with("tests/") {
            continue;
        }
        for (r, k) in hits(&src) {
            if r.starts_with("H15:") {
                *tab.entry((r, f.clone(), k)).or_default() += 1;
            }
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

/// One synthetic source's hits as a census table under the file name `file`.
fn table_of(file: &str, src: &str) -> Table {
    let mut tab = Table::new();
    for (r, k) in hits(src) {
        *tab.entry((r, file.to_string(), k)).or_default() += 1;
    }
    tab
}

/// Per (file, fn): body-root enters (either form) minus exits (C3 ; 0 everywhere).
fn body_balance(tab: &Table) -> BTreeMap<(String, String), i64> {
    let mut bal: BTreeMap<(String, String), i64> = BTreeMap::new();
    for ((r, f, k), n) in tab {
        let d = match r.as_str() {
            _ if r.starts_with("H7:wext_body_enter:")
                || r.starts_with("H7:wext_body_enter_keep:") =>
            {
                *n as i64
            }
            "H7:wext_body_exit" => -(*n as i64),
            _ => continue,
        };
        *bal.entry((f.clone(), k.clone())).or_default() += d;
    }
    bal
}

/// The count of `rule` (exact, or `rule:<anything>`) in one synthetic source.
fn count(rule: &str, src: &str) -> usize {
    hits(src)
        .iter()
        .filter(|(r, _)| r == rule || r.starts_with(&format!("{rule}:")))
        .count()
}

#[test]
fn h14_every_lower_stmt_call() {
    let rows: Vec<(&str, &str, &str, usize, &str)> = H14
        .iter()
        .map(|&(r, f, k, n, _, _)| (r, f, k, n, ""))
        .collect();
    check("H14", &rows);
    for &(r, f, k, _, class, _) in H14 {
        let want = match class {
            "R" => "H14:lower_stmt:bare",
            "S" => "H14:lower_stmt:masked",
            _ => panic!("{r} {f} {k}: class `{class}`"),
        };
        assert_eq!(r, want, "{f} {k}: a class-{class} call must be {want}");
    }
    let n = |c: &str| -> usize { H14.iter().filter(|r| r.4 == c).map(|r| r.3).sum() };
    assert_eq!(
        (n("R"), n("S")),
        (28, 19),
        "PLAN B7: 27 R + 1 MIXED (R), 19 S"
    );
    let v = floor_rule(&census());
    assert!(v.is_empty(), "the process root's floor: {v:?}");
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
    // C3: every body root's enter (either form) has its exit in the same (file, fn)
    let tab = census();
    let bal = body_balance(&tab);
    assert!(bal.values().all(|&d| d == 0), "body enter / exit: {bal:?}");
    // C3: the recorder's file reads no context span
    let cs: Vec<_> = tab
        .keys()
        .filter(|(r, f, _)| r == "R7:cur_span" && f == "wext.rs")
        .collect();
    assert!(cs.is_empty(), "`cur_span` in wext.rs: {cs:?}");
}

/// H13 (§5.2 Q-WEXTPOS C2): each continuous append call's `at` argument as spelled, per
/// (file, fn) — PLAN O8's table, the positive set being the reader's two kinds.
#[test]
fn h13_the_row_positions() {
    check("H13", H13);
}

/// A census table against a (rule, file, fn, count, class, why) table: the census moved, or
/// a class out of `classes`.
fn check_classed(
    name: &str,
    got: &Table,
    rows: &[(&str, &str, &str, usize, &str, &str)],
    classes: &[&str],
) {
    let want: Table = rows
        .iter()
        .map(|&(r, f, k, n, _, _)| ((r.to_string(), f.to_string(), k.to_string()), n))
        .collect();
    if *got != want {
        let mut s = String::new();
        for ((r, f, k), n) in got {
            if want.get(&(r.clone(), f.clone(), k.clone())) != Some(n) {
                s.push_str(&format!("    got  {r} {f} {k} {n}\n"));
            }
        }
        for ((r, f, k), n) in &want {
            if got.get(&(r.clone(), f.clone(), k.clone())) != Some(n) {
                s.push_str(&format!("    want {r} {f} {k} {n}\n"));
            }
        }
        panic!("{name}: the census moved — say whose text each span is:\n{s}");
    }
    for &(r, f, k, _, c, _) in rows {
        assert!(classes.contains(&c), "{name} {r} {f} {k}: class `{c}`");
    }
}

/// Per node kind (the rule's second field), the summed count of a table, and of one class.
fn kind_counts(
    rows: &[(&str, &str, &str, usize, &str, &str)],
    class: Option<&str>,
) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for &(r, _, _, n, c, _) in rows {
        if class.is_none_or(|w| w == c) {
            *m.entry(r.split(':').nth(1).unwrap_or("").to_string())
                .or_default() += n;
        }
    }
    m
}

/// H15 (§5.2 Q-WEXTPOS fix F1): every construction of a node whose span a write's `at` can
/// read, in this crate, with its span field as spelled (premise judge L2: 105 `Stmt`).
#[test]
fn h15_the_node_constructions() {
    check_classed(
        "H15",
        &rows_of(&census(), "H15"),
        H15,
        &["OWN", "FROM", "MASKED", "NOSTORE", "V", "NONE"],
    );
    let all = kind_counts(H15, None);
    assert_eq!(
        (
            all.get("Stmt"),
            all.get("PortConn"),
            all.get("ConnSite"),
            all.len()
        ),
        (Some(&105), Some(&1), Some(&3), 3),
        "H15 totals (L2)"
    );
}

/// H15v (§5.2 Q-WEXTPOS fix F1, P5 (i) and (ii)): the connection-span check is called once,
/// in `wire_ports` after an instance's port rows are pushed, and its two checks live in
/// `conn_span_violations` (each with a `#[should_panic]` unit in wext.rs's tests).
#[test]
fn h15v_the_assert_site() {
    check("H15v", H15V);
}

/// H15p (§5.2 Q-WEXTPOS fix F1, L1): the parser's constructions — 104 `Stmt`, of which the
/// 18 ANOTHER bodies V reads `None` on.
#[test]
fn h15p_the_parser_constructions() {
    check_classed(
        "H15p",
        &parser_census(),
        H15P,
        &[
            "OWN",
            "HOLDER",
            "FROM",
            "HOLDER+FROM",
            "OWN+HOLDER",
            "NOSTORE",
            "ITEM",
            "NOSPAN",
            "ANOTHER",
        ],
    );
    let all = kind_counts(H15P, None);
    assert_eq!(all.get("Stmt"), Some(&104), "H15p: 104 `Stmt` (L1)");
    let mut another: BTreeMap<&str, usize> = BTreeMap::new();
    for &(_, f, _, n, c, _) in H15P {
        if c == "ANOTHER" {
            *another.entry(f).or_default() += n;
        }
    }
    assert_eq!(
        another.into_iter().collect::<Vec<_>>(),
        vec![
            ("enums.rs", 2),
            ("type_param_shape.rs", 2),
            ("udp.rs", 10),
            ("udp_table.rs", 4)
        ],
        "H15p: the 18 ANOTHER (L1)"
    );
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
    // H7 (C2): the row position's writer call, not its definition
    assert_eq!(
        count(
            "H7",
            "fn record_row_at(&mut self) {} fn f(&mut self) { self.record_row_at(at); }"
        ),
        1,
        "H7 record_row_at planted call, definition skipped"
    );
    assert_eq!(
        hits("impl E { fn push_cont_assign(&mut self) { self.cont_assigns.push(ca); self.record_row_at(at); } }"),
        vec![
            ("H1:cont_assigns".to_string(), "push_cont_assign".to_string()),
            ("H7:record_row_at".to_string(), "push_cont_assign".to_string())
        ],
        "H7 real site (proc_ident.rs push_cont_assign) spelled back"
    );
    // H13: PLAN §6's four planted spellings each move the table ; a definition is not a call
    let h13 = |src: &str| -> Vec<String> {
        hits(src)
            .into_iter()
            .filter(|(r, _)| r.starts_with("H13:"))
            .map(|(r, k)| format!("{r}@{k}"))
            .collect()
    };
    assert_eq!(
        h13("impl E { fn elaborate_cont_assign(&mut self) { self.push_cont_assign(ir::ContAssign { lhs, rhs: rhs_id, delay }, \"assign\", Some(ca.span), Some(ca.span)); } }"),
        vec!["H13:push_cont_assign:Some(ca.span)@elaborate_cont_assign".to_string()],
        "H13 planted: netdecl back to the statement's span"
    );
    assert_eq!(
        h13("impl E { fn wire_ports(&mut self) { self.push_cont_assign_port(ir::ContAssign { lhs, rhs, delay: None }, Some(module.name.span),); } }"),
        vec!["H13:push_cont_assign_port:Some(module.name.span)@wire_ports".to_string()],
        "H13 planted: a port site given the child module's name (trailing comma)"
    );
    assert_eq!(
        h13("impl E { fn t0_pulse_net(&mut self) { self.push_cont_assign(ca, T0_PULSE_KIND, None, Some(e.span)); } }"),
        vec!["H13:push_cont_assign:Some(e.span)@t0_pulse_net".to_string()],
        "H13 planted: a None site given a span"
    );
    assert_eq!(
        h13("impl E { fn elaborate_cont_assign(&mut self) { let at = Some(ca.span); self.push_cont_assign(ca, \"assign\", Some(ca.span), at); } }"),
        vec!["H13:push_cont_assign:at@elaborate_cont_assign".to_string()],
        "H13 planted: the span bound to a local first (J3 line 10)"
    );
    assert_eq!(
        h13("impl E { fn push_cont_assign(&mut self, ca: C, kind: &str, span: S, at: S) {} fn push_cont_assign_port(&mut self) { self.push_cont_assign(ca, \"port\", span, span); } }"),
        vec!["H13:push_cont_assign:span@push_cont_assign_port".to_string()],
        "H13 real site (proc_ident.rs push_cont_assign_port) spelled back ; the definition skipped"
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
    // H7 (Q-WEXTPOS C3): the statement recorder's argument, the mask, the floors
    let h7c3 = |src: &str| -> Vec<String> {
        hits(src)
            .into_iter()
            .filter(|(r, _)| r.starts_with("H7:") || r.starts_with("R7:"))
            .map(|(r, k)| format!("{r}@{k}"))
            .collect()
    };
    assert_eq!(
        h7c3("impl E { fn lower_stmt(&mut self, b: &mut B, s: &ast::Stmt) { let w = self.stmts.len(); self.record_stmt_at(self.cur_span.unwrap(), w); } }"),
        vec![
            "H7:record_stmt_at:self.cur_span.unwrap()@lower_stmt".to_string(),
            "R7:cur_span@lower_stmt".to_string()
        ],
        "H7 planted: the recorder given the context's span (PLAN §6) moves the row ; `cur_span` seen"
    );
    assert_eq!(
        h7c3("impl E { fn lower_stmt(&mut self, b: &mut B, s: &ast::Stmt) { let wext_start = self.stmts.len(); self.record_stmt_at(s.span(), wext_start); } \
              fn record_stmt_at(&mut self, sp: ast::Span, start: usize) {} }"),
        vec!["H7:record_stmt_at:s.span()@lower_stmt".to_string()],
        "H7 real site (stmt_main.rs lower_stmt) spelled back ; the definition skipped"
    );
    assert_eq!(
        h7c3("impl E { fn lower_synth_proc(&mut self, pb: &P) -> ir::Process { let proc = self.lower_proc_block(pb, false); proc } }"),
        Vec::<String>::new(),
        "H7 planted: lower_synth_proc unmasked (PLAN §6) loses its `wext_unlocated` row"
    );
    assert_eq!(
        h7c3("impl E { fn lower_synth_proc(&mut self, pb: &P) -> ir::Process { let proc = self.wext_unlocated(|s| s.lower_proc_block(pb, false)); proc } \
              fn wext_unlocated<R>(&mut self, f: F) -> R { f(self) } }"),
        vec!["H7:wext_unlocated@lower_synth_proc".to_string()],
        "H7 real site (proc_ident.rs lower_synth_proc) spelled back ; the definition skipped"
    );
    assert_eq!(
        count("R7", "// self.cur_span\nfn f() {}"),
        0,
        "R7 a comment"
    );
    let unbal = table_of(
        "frames_body.rs",
        "impl E { fn g(&mut self) { let t = self.wext_body_enter(f.span); } fn h(&mut self) { let t = s.wext_body_enter_keep(p.span); s.wext_body_exit(t); } }",
    );
    let bal = body_balance(&unbal);
    assert_eq!(
        bal.get(&("frames_body.rs".to_string(), "g".to_string())),
        Some(&1),
        "H7 planted: a body root entered and never left"
    );
    assert_eq!(
        bal.get(&("frames_body.rs".to_string(), "h".to_string())),
        Some(&0),
        "H7 a keep-form root with its exit balances"
    );
    assert_eq!(
        h7c3("impl E { fn lower_proc_block(&mut self, p: &P) { let t = self.wext_body_enter_keep(p.body.span()); self.wext_body_exit(t); } }"),
        vec![
            "H7:wext_body_enter_keep:p.body.span()@lower_proc_block".to_string(),
            "H7:wext_body_exit@lower_proc_block".to_string()
        ],
        "H7 planted (fix F1, m9c / L8): a body root given the BODY's span, not the item's, moves the row"
    );
    assert_eq!(
        h7c3("impl E { fn inline_task_body(&mut self, b: &mut B, body: &ast::Stmt) { self.wext_located(|s| { s.lower_stmt(b, body); }); } }"),
        vec!["H7:wext_located@inline_task_body".to_string()],
        "H7 real site (inline_task.rs inline_task_body, a callee entry) spelled back"
    );
    // H15 / H15p (fix F1): constructions, not patterns ; aliases resolved per file / fn
    let h15 = |src: &str| -> Vec<String> {
        hits(src)
            .into_iter()
            .filter(|(r, _)| r.starts_with("H15:"))
            .map(|(r, k)| format!("{r}@{k}"))
            .collect()
    };
    assert_eq!(
        h15("impl P { fn g(&mut self) { let f = |sp: Span| Stmt::Null(sp); body.push(f(s)); } }"),
        vec!["H15:Stmt:sp@g".to_string()],
        "H15p planted (L1): a construction after a closure head is counted (fails the table)"
    );
    assert_eq!(
        h15("use hdl_ast::Sequence as S;\nimpl E { fn g(&mut self) { let q = S::Repeat { span: sp, n: 1 }; } }"),
        Vec::<String>::new(),
        "H15 planted (L2): `S::Repeat {{` under `use ast::Sequence as S` is not a statement"
    );
    assert_eq!(
        h15("use hdl_ast::Stmt as S;\nimpl E { fn g(&mut self) { let q = S::Null(sp); } fn h(&mut self) { let r = Sequence::Repeat { span: sp }; } }"),
        vec!["H15:Stmt:sp@g".to_string()],
        "H15 an alias of `Stmt` counts ; another enum's variant does not"
    );
    assert_eq!(
        h15("impl E { fn wire(&mut self) { let a = ast::PortConn { name: n, expr: e, span: *span }; \
             let b = ast::PortConn { name: m, expr: f, span: item.span }; let c = ConnSite { span: Some(item.span), e }; } }"),
        vec![
            "H15:PortConn:*span@wire".to_string(),
            "H15:PortConn:item.span@wire".to_string(),
            "H15:ConnSite:Some(item.span)@wire".to_string()
        ],
        "H15 planted: a second `PortConn {{` and `ConnSite {{ span: Some(item.span) }}` are rows"
    );
    assert_eq!(
        h15("impl E { fn m(&mut self, s: &Stmt) { match s { Stmt::Null(sp) => {} Stmt::If { span, .. } | Stmt::Case { span, .. } => {} } \
             if let ast::Stmt::Block { span, .. } = s {} let ok = matches!(s, Stmt::Error(_)); \
             match (a, b) { (Stmt::Null(x), ConnSite { span, .. }) => {} } let i = ir::Stmt::Nop { span }; } \
             fn n(&self) -> ast::PortConn { todo!() } }\nstruct PortConn { span: Span }\nimpl PortConn { fn f() {} }"),
        Vec::<String>::new(),
        "H15: patterns (match arms, `|`, `if let`, `matches!`, a tuple of patterns), an `ir` path, a return type, an item are not constructions"
    );
    assert_eq!(
        h15("impl E { fn k(&mut self) { let v = vec![Stmt::Null(a), Stmt::Blocking { lhs: l, rhs: r, span }]; \
             f(Stmt::Return { value: None, span: s.span() }, x); let u = Stmt::If { span: sp, ..base }; } }"),
        vec![
            "H15:Stmt:a@k".to_string(),
            "H15:Stmt:span@k".to_string(),
            "H15:Stmt:s.span()@k".to_string(),
            "H15:Stmt:sp@k".to_string()
        ],
        "H15: constructions inside a vec and an argument list, the shorthand, a call, a functional update"
    );
    // H14 (Q-WEXTPOS C3): every `lower_stmt (` call, any receiver, masked or bare
    let h14 = |src: &str| -> Vec<String> {
        hits(src)
            .into_iter()
            .filter(|(r, _)| r.starts_with("H14:"))
            .map(|(r, k)| format!("{r}@{k}"))
            .collect()
    };
    assert_eq!(
        h14("impl E { fn hoist_rhs_only(&mut self) { self.wext_unlocated(|s| s.lower_stmt(b, &synth)); self.lower_stmt(b, &x); } }"),
        vec![
            "H14:lower_stmt:masked@hoist_rhs_only".to_string(),
            "H14:lower_stmt:bare@hoist_rhs_only".to_string()
        ],
        "H14 planted: a bare call added to a class-S fn (PLAN §6) is a `bare` row there"
    );
    assert_eq!(
        h14("impl E { fn new_lowering(&mut self) { e.lower_stmt(b, x); } }"),
        vec!["H14:lower_stmt:bare@new_lowering".to_string()],
        "H14 planted: an unlisted new call, another receiver (premise judge 3 line 10)"
    );
    assert_eq!(
        h14("impl E { fn f(&mut self) { self.wext_unlocated(|s| s.foo()); self.lower_stmt(b, x); self.lower_stmt_inner(b, x); } \
             fn lower_stmt(&mut self) {} }\n// self.lower_stmt(b, x);"),
        vec!["H14:lower_stmt:bare@f".to_string()],
        "H14: masked only inside the mask's argument group ; another name, a definition, a comment not counted"
    );
    assert_eq!(
        h14("impl E { fn synth_cover_sample_explicit(&mut self) { for s in &stmts { self.wext_unlocated(|e| e.lower_stmt(b, s)); } } }"),
        vec!["H14:lower_stmt:masked@synth_cover_sample_explicit".to_string()],
        "H14 real site (cover.rs synth_cover_sample_explicit) spelled back"
    );
    // H14 / P2: the process root zeroing the mask (PLAN's original V6 at stmt_flow.rs:471)
    let zero = table_of(
        "stmt_flow.rs",
        "impl E { fn lower_proc_block(&mut self) { let mut b = B::new(); let t = self.wext_body_enter(p.span); self.lower_stmt(&mut b, &p.body); self.wext_body_exit(t); } }",
    );
    assert_eq!(
        floor_rule(&zero),
        vec![
            "lower_proc_block zeroes the mask (wext_body_enter)".to_string(),
            "lower_proc_block has no keep-form body root".to_string()
        ],
        "H14 / P2 planted: the process root's floor zeroes the mask"
    );
    let keep = table_of(
        "stmt_flow.rs",
        "impl E { fn lower_proc_block(&mut self) { let mut b = B::new(); let t = self.wext_body_enter_keep(p.span); self.lower_stmt(&mut b, &p.body); self.wext_body_exit(t); } }",
    );
    assert!(
        floor_rule(&keep).is_empty(),
        "H14 / P2 real site (stmt_flow.rs lower_proc_block) spelled back"
    );
}

/// H14 (§5.2 Q-WEXTPOS C3): (rule, file, fn, count, class, why) — every `lower_stmt (`
/// call, any receiver, per (file, fn): class R lowers a statement the PARSER built, with the
/// span the parser gave it — its own text, or for the parser-built `$enum_name$<T>` body the
/// first `.name()` call's path (premise judge 2, F-A) — (bare), class S a statement vita
/// built whole (inside `wext_unlocated`). PLAN
/// B7's 47 calls: R 27, S 19, MIXED 1 (`stmt_main.rs`'s `iff` guard shell, R by premise
/// judge 3 line 1: its own lowering pushes no store, the parsed body inside records).
#[rustfmt::skip]
const H14: &[(&str, &str, &str, usize, &str, &str)] = &[
    ("H14:lower_stmt:bare", "case_inside.rs", "lower_case_inside_body", 2, "R", "a `case inside` arm's parsed body"),
    ("H14:lower_stmt:bare", "class_lower.rs", "lower_class_method_body", 1, "R", "a class method's parsed body (under its floor)"),
    ("H14:lower_stmt:bare", "dead_arm.rs", "lower_if_arm", 2, "R", "an `if` arm's parsed body"),
    ("H14:lower_stmt:bare", "frames_body.rs", "lower_frame_body_stmt", 2, "R", "a frame's body (parsed, or the parser's `$enum_name$` body: F-A)"),
    ("H14:lower_stmt:bare", "inline_task.rs", "inline_task_body", 2, "R", "an inlined task's parsed body (callee entry, located again)"),
    ("H14:lower_stmt:bare", "stmt_flow.rs", "lower_case", 2, "R", "a `case` arm / default's parsed body"),
    ("H14:lower_stmt:bare", "stmt_flow.rs", "lower_for", 3, "R", "a `for` loop's parsed initializer, body and step"),
    ("H14:lower_stmt:bare", "stmt_flow.rs", "lower_forever", 1, "R", "a `forever` (or self-timed `always`) parsed body"),
    ("H14:lower_stmt:bare", "stmt_flow.rs", "lower_proc_block", 1, "R", "a procedural block's parsed body (under the process root's floor)"),
    ("H14:lower_stmt:bare", "stmt_flow.rs", "lower_repeat", 2, "R", "a `repeat` / `do while` parsed body"),
    ("H14:lower_stmt:bare", "stmt_flow.rs", "lower_while", 1, "R", "a `while` parsed body"),
    ("H14:lower_stmt:bare", "stmt_main.rs", "lower_stmt_inner", 9, "R", "a block's / `if`'s / wait's / fork's parsed children ; the `iff` guard shell (MIXED, R: premise judge 3 line 1)"),
    ("H14:lower_stmt:masked", "cover.rs", "synth_cover_sample", 1, "S", "a covergroup sampler's gated bitmap store"),
    ("H14:lower_stmt:masked", "cover.rs", "synth_cover_sample_explicit", 2, "S", "a covergroup's explicit sample statements"),
    ("H14:lower_stmt:masked", "cover.rs", "synth_cross_sample", 1, "S", "a cross's bitmap store"),
    ("H14:lower_stmt:masked", "dynarr.rs", "dyn_blocking_special", 3, "S", "a dyn-array capture temp, its clear and its element stores"),
    ("H14:lower_stmt:masked", "dynarr.rs", "hoist_dyn_pattern_actuals", 1, "S", "a dyn temp's element stores"),
    ("H14:lower_stmt:masked", "events.rs", "lower_intra_event_assign", 1, "S", "an intra-assignment event's built wait"),
    ("H14:lower_stmt:masked", "events.rs", "lower_intra_event_nba", 1, "S", "an intra-assignment event's built wait (NBA)"),
    ("H14:lower_stmt:masked", "frames_body.rs", "emit_frame_local_inits", 2, "S", "a frame local's built initializer / its pre-store"),
    ("H14:lower_stmt:masked", "frames_body.rs", "emit_per_entry_block_inits", 3, "S", "a block local's built re-initializer, clear and element stores"),
    ("H14:lower_stmt:masked", "frames_call/emit.rs", "emit_frame_task_call", 1, "S", "a frame call's built unpack into a local"),
    ("H14:lower_stmt:masked", "frames_static_init.rs", "emit_static_decl_inits", 1, "S", "a static local's built initializer"),
    ("H14:lower_stmt:masked", "hoist/mod.rs", "hoist_dyn_formal_calls", 1, "S", "a hoisted call's temporary"),
    ("H14:lower_stmt:masked", "hoist/special.rs", "hoist_rhs_only", 1, "S", "a hoisted rhs's temporary"),
];

/// The process root's floor (premise judge round 1, line 1 ; P2): `lower_proc_block` is
/// reached under `lower_synth_proc`'s mask, so its body root must KEEP the mask depth
/// (`wext_body_enter_keep`), never zero it (`wext_body_enter`, PLAN's original V6 there,
/// premise P-1). The violations of that in a census table.
fn floor_rule(tab: &Table) -> Vec<String> {
    // a body root's row is keyed by its item argument (fix F1, V): summed over the spellings
    let at = |r: &str| -> usize {
        tab.iter()
            .filter(|((rr, f, k), _)| {
                rr.strip_prefix(r).is_some_and(|t| t.starts_with(':'))
                    && f == "stmt_flow.rs"
                    && k == "lower_proc_block"
            })
            .map(|(_, n)| *n)
            .sum()
    };
    let mut v = Vec::new();
    if at("H7:wext_body_enter") != 0 {
        v.push("lower_proc_block zeroes the mask (wext_body_enter)".to_string());
    }
    if at("H7:wext_body_enter_keep") != 1 {
        v.push("lower_proc_block has no keep-form body root".to_string());
    }
    v
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
    ("H7:record_row_at", "proc_ident.rs", "push_cont_assign", 1, "each continuous row's own position, pushed beside the row (Q-WEXTPOS C2)"),
    ("H7:record_stmt_at:s.span()", "stmt_main.rs", "lower_stmt", 1, "a statement's own position: the fn's own parameter's span, at its one exit (Q-WEXTPOS C3, never `cur_span`)"),
    ("H7:wext_body_enter:item", "class_lower.rs", "lower_class_method_body", 1, "the method body's floor: save, set 0 (the callee entry of a method), restore ; V's item = the method's function or task (fix F1, L8)"),
    ("H7:wext_body_enter:func.span", "frames_body.rs", "lower_frame_body_inner_lower_frame_func_body", 1, "a function body's floor: save, set 0, restore ; V's item = the FunctionDef (L8)"),
    ("H7:wext_body_enter:task.span", "frames_body.rs", "lower_frame_body_inner_lower_frame_task_body", 1, "a task body's floor: save, set 0, restore ; V's item = the TaskDef (L8)"),
    ("H7:wext_body_enter_keep:p.span", "stmt_flow.rs", "lower_proc_block", 1, "the process root's floor: save and restore, the mask KEPT (premise judge line 1) ; V's item = the ProceduralBlock (L8)"),
    ("H7:wext_body_exit", "class_lower.rs", "lower_class_method_body", 1, "its floor's exit"),
    ("H7:wext_body_exit", "frames_body.rs", "lower_frame_body_inner_lower_frame_func_body", 1, "its floor's exit"),
    ("H7:wext_body_exit", "frames_body.rs", "lower_frame_body_inner_lower_frame_task_body", 1, "its floor's exit"),
    ("H7:wext_body_exit", "stmt_flow.rs", "lower_proc_block", 1, "its floor's exit"),
    ("H7:wext_located", "frames_body.rs", "lower_frame_body_stmt", 1, "a callee entry: save, set 0, restore"),
    ("H7:wext_located", "inline_task.rs", "inline_task_body", 1, "a callee entry: save, set 0, restore (no floor: a copy-in belongs to the call)"),
    ("H7:wext_rec", "wext.rs", "build_write_view", 2, "taken once into the view ; then marked built (the funnel's debug order check)"),
    ("H7:wext_rec", "wext.rs", "carry_geom", 2, "the geometry writer (read base, insert)"),
    ("H7:wext_rec", "wext.rs", "patch_expr", 2, "the patch funnel: its debug order check and the stand-down's setter"),
    ("H7:wext_rec", "wext.rs", "patch_expr_with", 2, "the patch funnel: its debug order check and the stand-down's setter"),
    ("H7:wext_rec", "wext.rs", "record_decl_fact", 1, "the declaration-fact writer"),
    ("H7:wext_rec", "wext.rs", "record_geom", 1, "the geometry writer"),
    ("H7:wext_rec", "wext.rs", "record_row_at", 1, "the row position writer (Q-WEXTPOS C2)"),
    ("H7:wext_rec", "wext.rs", "record_via", 1, "the via writer"),
    ("H7:wext_rec", "wext.rs", "register_provisional_const", 1, "the stand-down's registration"),
    ("H7:wext_rec", "wext.rs", "record_stmt_at", 2, "the statement position writer: the mask read, the interval (Q-WEXTPOS C3)"),
    ("H7:wext_rec", "wext.rs", "wext_body_enter", 3, "a body root: open count, mask saved and set 0, V's open item saved and set (fix F1)"),
    ("H7:wext_rec", "wext.rs", "wext_body_enter_keep", 3, "the process root: open count, mask saved and kept, V's open item saved and set (fix F1)"),
    ("H7:wext_rec", "wext.rs", "wext_body_exit", 4, "a body root's exit: open count, mask restored, the floor, V's open item restored (fix F1)"),
    ("H7:wext_rec", "wext.rs", "wext_located", 2, "a callee entry: mask saved and set 0, restored"),
    ("H7:wext_rec", "wext.rs", "wext_unlocated", 2, "the mask: up, down"),
    ("H7:wext_unlocated", "cover.rs", "synth_cover_sample", 1, "S (H14)"),
    ("H7:wext_unlocated", "cover.rs", "synth_cover_sample_explicit", 2, "S (H14)"),
    ("H7:wext_unlocated", "cover.rs", "synth_cross_sample", 1, "S (H14)"),
    ("H7:wext_unlocated", "dynarr.rs", "dyn_blocking_special", 3, "S (H14)"),
    ("H7:wext_unlocated", "dynarr.rs", "hoist_dyn_pattern_actuals", 1, "S (H14)"),
    ("H7:wext_unlocated", "events.rs", "lower_intra_event_assign", 1, "S (H14)"),
    ("H7:wext_unlocated", "events.rs", "lower_intra_event_nba", 1, "S (H14)"),
    ("H7:wext_unlocated", "frames_body.rs", "emit_frame_local_inits", 2, "S (H14)"),
    ("H7:wext_unlocated", "frames_body.rs", "emit_per_entry_block_inits", 3, "S (H14)"),
    ("H7:wext_unlocated", "frames_call/emit.rs", "emit_frame_task_call", 1, "S (H14)"),
    ("H7:wext_unlocated", "frames_static_init.rs", "emit_static_decl_inits", 1, "S (H14)"),
    ("H7:wext_unlocated", "hoist/mod.rs", "hoist_dyn_formal_calls", 1, "S (H14)"),
    ("H7:wext_unlocated", "hoist/special.rs", "hoist_rhs_only", 1, "S (H14)"),
    ("H7:wext_unlocated", "proc_ident.rs", "lower_synth_proc", 1, "a synthesized body (var_init, sva and its action blocks, cover, clocking, the module-level assert wrapper)"),
];

/// H13: (rule, file, fn, count, why) — each continuous append call's `at` as spelled (PLAN O8).
#[rustfmt::skip]
const H13: &[(&str, &str, &str, usize, &str)] = &[
    ("H13:push_cont_assign:None", "const_level_header.rs", "t0_pulse_net", 1, "`t0_pulse`: a synthesized net no reader prints"),
    ("H13:push_cont_assign:None", "events.rs", "level_select_net", 1, "`event_select`: a synthesized holder no reader prints"),
    ("H13:push_cont_assign:None", "var_init.rs", "elaborate_net_init_drivers", 1, "`net_init`: a kind no reader prints (PLAN O8)"),
    ("H13:push_cont_assign:Some(lv.span())", "cont_array.rs", "cont_assign_whole_array", 1, "`assign`: the one target every element row writes (the fn's own parameter `lv`)"),
    ("H13:push_cont_assign:Some(lv.span())", "netdecl.rs", "elaborate_cont_assign", 1, "`assign`: this assignment's own target (a gate: its output terminal), not the statement"),
    ("H13:push_cont_assign:span", "proc_ident.rs", "push_cont_assign_port", 1, "`port`: the connection's span, the row identity's own (`.*` = None)"),
    ("H13:push_cont_assign_port:conn.span", "ports.rs", "wire_array_port", 1, "a per-element port row: the one connection `.p(arr)`"),
    ("H13:push_cont_assign_port:conn.span", "ports.rs", "wire_ports", 2, "an input / output port row: named from the `.`, positional the expression, `.*` None"),
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

/// H15 (§5.2 Q-WEXTPOS fix F1): (rule, file, fn, count, class, why) — every CONSTRUCTION in
/// this crate of a node whose span a write's `at` can read (`ast::Stmt :: <V>`, `PortConn {`,
/// `ConnSite {`, `ContinuousAssign {`, `InstanceItem {`, `PortConnList :: <V> (`), keyed by
/// the `span` field as spelled. Class (hand-written, R-AT's words): OWN / FROM the write's
/// own text ; MASKED a statement vita builds, lowered under THE MASK (its store takes the
/// parsed statement around it) ; NOSTORE no store of its own ; V a span V records nothing for
/// ; NONE no text. 105 `Stmt` (premise judge L2: `sva_seq.rs`'s `S::Repeat` is a `Sequence`),
/// 1 `PortConn`, 3 `ConnSite`. A row that moves: say whose text the span is.
#[rustfmt::skip]
const H15: &[(&str, &str, &str, usize, &str, &str)] = &[
    ("H15:ConnSite:None", "ports.rs", "wire_ports", 1, "NONE", "a `.*` connection: its expression is synthesized (no write text)"),
    ("H15:ConnSite:Some(c.span)", "ports.rs", "wire_ports", 1, "OWN", "a named connection: the parser's `PortConn` from its `.`"),
    ("H15:ConnSite:Some(e.span)", "ports.rs", "wire_ports", 1, "OWN", "a positional connection: its expression"),
    ("H15:PortConn:*span", "instance_array.rs", "elaborate_instance_array", 1, "FROM", "an array element's connection: the parsed connection's span, a positional one's expression (X1)"),
    ("H15:Stmt:*span", "hoist/general_stmt.rs", "hoist_stmt_general", 7, "OWN", "a hoist rewrite: the rewritten parsed statement's own span"),
    ("H15:Stmt:*span", "hoist/mod.rs", "hoist_stmt_top", 7, "OWN", "a hoist rewrite: the rewritten parsed statement's own span"),
    ("H15:Stmt:*span", "hoist/special.rs", "hoist_stmt_rhs_only", 7, "OWN", "a hoist rewrite: the rewritten parsed statement's own span"),
    ("H15:Stmt:*span", "sva_decl.rs", "rewrite_sampled_stmt", 7, "OWN", "a pattern-bound parsed statement's own span, copied"),
    ("H15:Stmt:cb.span", "sva_clocking.rs", "lower_clocking_blocks", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:decl.span", "frames_body.rs", "emit_frame_local_inits", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:decl.span", "frames_body.rs", "emit_per_entry_block_inits", 2, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:decl.span", "frames_static_init.rs", "emit_static_decl_inits", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:e.span", "dynarr.rs", "dyn_blocking_special", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:e.span", "hoist/mod.rs", "hoist_dyn_formal_calls", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:e.span", "hoist/special.rs", "hoist_rhs_only", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:gspan", "stmt_main.rs", "lower_stmt_inner", 3, "NOSTORE", "the `iff` guard's wait loop (the guard expression's span)"),
    ("H15:Stmt:p.span", "events.rs", "desugar_event_iff", 1, "V", "the always ITEM's span: V records nothing for it (PLAN J3)"),
    ("H15:Stmt:rhs.span", "dynarr.rs", "dyn_blocking_special", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover.rs", "register_cover_instance", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover.rs", "synth_cover_sample", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover.rs", "synth_cover_sample_explicit", 4, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover.rs", "synth_cross_sample", 2, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover_synth.rs", "cov_auto_sample_stmt", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover_synth.rs", "cov_counter_then", 4, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover_synth.rs", "cov_set_bit_stmt", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "cover_synth.rs", "materialize_one_cover", 3, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "frames_call/emit.rs", "emit_frame_task_call", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_ast.rs", "sva_block_or_single", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_ast.rs", "sva_error_stmt", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_ast.rs", "sva_nb", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_ast.rs", "sva_nb_set", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_ast.rs", "sva_nba_1bit", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_check.rs", "materialize_one_sva", 5, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_check.rs", "prop_expr_violation", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_check.rs", "synth_local_var_assert", 2, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_clocking.rs", "synth_crossclock", 7, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_clocking.rs", "synth_multiclock", 7, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_liveness.rs", "synth_liveness", 4, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_prop.rs", "sva_prev_for", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_prop.rs", "synth_prop_expr", 3, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_seq.rs", "seq_delay_reg", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "sva_seq.rs", "synth_seq_pipeline", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:sp", "var_init.rs", "flush_pending_var_inits", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:span", "dynarr.rs", "dyn_decl_init_stmts", 3, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:span", "events.rs", "lower_intra_event_assign", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:span", "events.rs", "lower_intra_event_nba", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:span", "frames_body.rs", "emit_frame_local_inits", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:span", "sva_ast.rs", "gate_nba_with_disable", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
    ("H15:Stmt:span", "var_init.rs", "flush_pending_var_inits", 1, "MASKED", "a statement vita builds, lowered under THE MASK (premise 2's census)"),
];

/// H15p (§5.2 Q-WEXTPOS fix F1, L1): the same census over the PARSER's production sources
/// (`crates/hdl-parser/src`): 104 `Stmt`, 2 `PortConn`, 2 `ContinuousAssign`, 1
/// `InstanceItem`, 4 `PortConnList`. Class: OWN the parsed node's own text ; HOLDER a span
/// handed in from the statement being rewritten ; FROM one construct's rows ; NOSTORE an
/// `Error` / `Null` / a store-less node ; ITEM / NOSPAN an item, a list ; ANOTHER a body the
/// parser builds whose statements carry their ITEM's span (18: `udp.rs` 10, `udp_table.rs` 4,
/// `enums.rs` 2, `type_param_shape.rs` 2) — what V turns to `None`. It lives here, not in
/// hdl-parser's tests: that crate has no `proc-macro2` dev-dependency and this walker is
/// one (the `lossy_home.rs` precedent).
#[rustfmt::skip]
const H15P: &[(&str, &str, &str, usize, &str, &str)] = &[
    ("H15:ContinuousAssign:start.to(self.prev_span())", "decls.rs", "parse_cont_assign", 1, "OWN", "the parsed node's own text"),
    ("H15:ContinuousAssign:start.to(self.prev_span())", "gates.rs", "parse_gate_primitive", 1, "OWN", "the parsed node's own text"),
    ("H15:InstanceItem:start.to(self.prev_span())", "instances.rs", "parse_instance_item", 1, "ITEM", "an instance item's own text (R-AT: never a row's `at`)"),
    ("H15:PortConn:start.to(self.prev_span())", "instances.rs", "parse_named_port_conn", 2, "OWN", "the parsed node's own text"),
    ("H15:PortConnList:Vec::new()", "instances.rs", "parse_port_conns", 2, "NOSPAN", "a connection list: no span of its own"),
    ("H15:PortConnList:conns", "instances.rs", "parse_port_conns", 2, "NOSPAN", "a connection list: no span of its own"),
    ("H15:Stmt:n_start.to(self.prev_span())", "stmt_ctl.rs", "build_for_typed_init", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:qspan.to(self.prev_span())", "assertions.rs", "parse_unique_priority", 1, "NOSTORE", "an `Error`"),
    ("H15:Stmt:s", "stmt.rs", "parse_statement", 1, "NOSTORE", "an `Error` / `Null` at the current token"),
    ("H15:Stmt:s", "stmt.rs", "parse_statement_inner", 2, "NOSTORE", "an `Error` / `Null` at the current token"),
    ("H15:Stmt:s", "stmt.rs", "stmt_error", 1, "NOSTORE", "an `Error` / `Null` at the current token"),
    ("H15:Stmt:self.cur_span()", "functask.rs", "tf_body", 1, "NOSTORE", "an empty routine body's `Null`"),
    ("H15:Stmt:sp", "assertions.rs", "parse_assert_stmt", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "assertions.rs", "parse_assert_stmt", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "assertions.rs", "parse_unique_priority", 1, "HOLDER", "a `unique` / `priority` warning: the statement's span, no store (L1: :514)"),
    ("H15:Stmt:span", "enums.rs", "build_enum_name_fn", 2, "ANOTHER", "the `$enum_name$` body: the first `.name()` call's path (F-A ; V: at=-)"),
    ("H15:Stmt:span", "item_dispatch.rs", "parse_module_item", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "soa.rs", "try_soa_assign", 7, "HOLDER", "a struct-of-arrays rewrite: the rewritten statement's span"),
    ("H15:Stmt:span", "soa.rs", "try_soa_queue_method_stmt", 2, "HOLDER+FROM", "a struct-of-arrays queue method: the rewritten statement's span (L1: :135 FROM)"),
    ("H15:Stmt:span", "stmt.rs", "assign_stmt", 2, "HOLDER", "soa's assignment builder: its caller's statement span"),
    ("H15:Stmt:span", "stmt.rs", "parse_assign_or_call", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt.rs", "parse_break_continue", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt.rs", "parse_labeled_stmt", 4, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt.rs", "parse_pre_incdec", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt.rs", "parse_void_cast_stmt", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt.rs", "try_compound_assign", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt_ctl.rs", "build_multi_foreach", 5, "HOLDER", "a multi-dimensional `foreach`'s built loop: the `foreach` (L1: :810)"),
    ("H15:Stmt:span", "stmt_ctl.rs", "parse_do_while", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt_ctl.rs", "parse_for", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:span", "stmt_ctl.rs", "parse_foreach", 4, "OWN+HOLDER", "a `foreach`'s own loop nodes ; its built index initializer (L1: :617 HOLDER)"),
    ("H15:Stmt:span", "type_param_shape.rs", "type_param_shape_guard", 2, "ANOTHER", "a type-parameter shape guard (`$fatal`, no store ; V: at=-)"),
    ("H15:Stmt:span", "udp.rs", "parse_udp_decl", 10, "ANOTHER", "a UDP body: the whole `primitive` (V: at=-)"),
    ("H15:Stmt:span", "udp_table.rs", "build_comb_udp", 4, "ANOTHER", "a combinational UDP's body: the whole `primitive` (V: at=-)"),
    ("H15:Stmt:start", "assertions.rs", "parse_assert_stmt", 1, "NOSTORE", "an assertion's `Null` action"),
    ("H15:Stmt:start", "stmt.rs", "wrap_break", 1, "HOLDER", "a loop's `break` wrapper: the loop keyword"),
    ("H15:Stmt:start", "stmt_ctl.rs", "parse_foreach", 2, "NOSTORE", "an `Error` at the `foreach`"),
    ("H15:Stmt:start", "stmt_ctl.rs", "parse_loop_body", 1, "HOLDER", "a loop body's `continue` wrapper: the loop keyword"),
    ("H15:Stmt:start", "stmt_ctl.rs", "parse_multidim_foreach", 2, "NOSTORE", "an `Error` at the `foreach` (PLAN: HOLDER)"),
    ("H15:Stmt:start.to(self.prev_span())", "assertions.rs", "parse_concurrent_assert", 3, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_deassign", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_delay_stmt", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_event_stmt", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_force", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_par_block", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_release", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_trigger_stmt", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "blocks_timing.rs", "parse_wait", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "cover.rs", "parse_cover_property", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "expr.rs", "parse_systask_call", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "functask.rs", "tf_body", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt.rs", "parse_assign_or_call", 3, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt.rs", "parse_disable", 2, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt.rs", "parse_proc_assign", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt.rs", "parse_return", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt.rs", "stmt_error_at", 1, "NOSTORE", "an `Error` over the statement's text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_case", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_case_inside", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_do_while", 1, "NOSTORE", "an `Error` over the statement's text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_for_assign", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_forever", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_if", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_repeat", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "stmt_ctl.rs", "parse_while", 1, "OWN", "the parsed node's own text"),
    ("H15:Stmt:start.to(self.prev_span())", "sva_seq.rs", "parse_seq_block", 1, "OWN", "the parsed node's own text"),
];

/// H15v: (rule, file, fn, count, why).
#[rustfmt::skip]
const H15V: &[(&str, &str, &str, usize, &str)] = &[
    ("H15v:check_conn_spans", "ports.rs", "wire_ports", 1, "one instance's located connections, after its port rows (a debug run only)"),
    ("H15v:conn_span_violations", "wext.rs", "check_conn_spans", 1, "the two checks, asserted empty"),
];
