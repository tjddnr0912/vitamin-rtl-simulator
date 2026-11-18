//! §5.2 Q-WREXT: the write-extent view has one home, `crates/elaborate/src/wext.rs` (ER
//! §5.1). This test pins, counted per (file, enclosing fn) over
//! `crates/elaborate/src/**/*.rs` (the item after `#[cfg(test)]` skipped), what the view
//! rests on, who reads it and what its one reader reads:
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
//! - H3 the view is built once and read through one entry: `build_view (` only in
//!   `build_write_view`, `build_write_view (` only in the end phase (`run`), `. wext`
//!   only where it is built, where it is handed to `Sidecars` (the `wext:` initialiser in
//!   `driver.rs` `new` is a field init, not this token) and once in its one reader,
//!   `mdrv_mix.rs` `check_mixed_drivers` (§5.2 Q-MDRVMIX ; a second reader fails here),
//!   and the token `write_view` in no production source of any crate but `api.rs` (the
//!   hand-off) and `tables.rs` (the field).
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
//!   wext.rs.
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
//! - H10 (§5.2 Q-MDRVMIX C1, C2) what the view's one reader reads, in `mdrv_mix.rs`: H10d
//!   every identifier of its decision fns (`H10D_FNS`: the target set, rule X's and rule
//!   Y's sides and processes, the sweep, the pairs and the entry), any receiver, is on `H10D_ALLOW` — no `at`, no name
//!   (`symbols`, `display_of`), no `nets`, no `intro_kind`, no `cont_var` / `reported` ;
//!   H10m every identifier of its message fns (`H10M_FNS`) and every name a format literal
//!   there captures (`{name}`, a `name$` in a spec) is on `H10M_ALLOW` — the view's `Write`
//!   and the pair's fields, `symbols`, `display_of`, `span_file_line_col`, `error`,
//!   `cur_span`, `cur_prefix`, the file's consts and std verbs, so no identity row
//!   (`proc_idents`, `ca_idents`, `frame_func_names`, `frame_decl_locs`), no `nets`, no
//!   `intro_kind`, no name split (`rsplit`) feeds a word of the text (the re-entry's (b)) ;
//!   and each process / continuous kind string of the closed vocabulary (`KINDS`) spelled
//!   once, in the file's consts.
//! - H10k (§5.2 Q-MDRVMIX C3) rule Y's A-kind list and its clause table are ONE const each
//!   (`A_KINDS`, `A_CLAUSES`), of the same declared length and the same number of entries,
//!   and no kind string of `KINDS` is spelled inside a fn of `mdrv_mix.rs`.
//! - H-W1 (§5.2 Q-MDRVMIX C3) `MsgCode :: ElabMultidriverStrict` (W3060) is spelled in
//!   exactly two non-comment, non-test sites of `crates/elaborate/src`, both in
//!   `multidriver.rs` `check_multidriver_processes`: Rule C (a declaration initializer on
//!   an `always_ff` / `always_latch` variable) and Rule B's `always_latch` arm when the
//!   latch, or every other writer, writes the name by `force` alone (premise judge 2 of
//!   C3, line 3) ; a third producer fails.
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
/// H10: the closed kind vocabulary of `proc_idents` (`push_process`, `lower_synth_proc`'s
/// callers) and `ca_idents` (`push_cont_assign`'s callers).
const KINDS: [&str; 13] = [
    "initial",
    "always",
    "always_ff",
    "always_comb",
    "always_latch",
    "final",
    "sva",
    "covergroup",
    "var_init",
    "assign",
    "port",
    "net_init",
    "t0_pulse",
];
/// H10d: the reader's decision fns (`mdrv_mix.rs`).
const H10D_FNS: [&str; 11] = [
    "in_t2",
    "procedural",
    "x_side",
    "in_p5",
    "y_side",
    "who",
    "meets",
    "keep",
    "sweep",
    "pairs",
    "check_mixed_drivers",
];
/// H10d: every identifier the decision fns may spell, any receiver: the view's `Write`
/// fields but `at` and `stmt` / `chunk`, its declaration fact, the flag, the file's own
/// items and std / language words. Not here, by design: `at`, `symbols`, `display_of`,
/// `nets`, `intro_kind`, `net_names`, `cont_var`, `reported`, every identity row.
#[rustfmt::skip]
const H10D_ALLOW: &[&str] = &[
    "ALWAYS_COMB",
    "A_KINDS",
    "AnsiPort",
    "BTreeMap",
    "BTreeSet",
    "Bit",
    "Bits",
    "Blocking",
    "Body",
    "CONT",
    "Cont",
    "DeclSite",
    "Direct",
    "Extent",
    "Form",
    "Generate",
    "Internal",
    "Iv",
    "Logic",
    "Module",
    "NetVarKind",
    "NonAnsiPort",
    "NonBlocking",
    "None",
    "Output",
    "P6",
    "Pair",
    "PortDir",
    "Proc",
    "Reg",
    "Rule",
    "Some",
    "TaskInline",
    "Top2",
    "VAR_INIT",
    "Vec",
    "Via",
    "Write",
    "Writer",
    "X",
    "Y",
    "_",
    "a",
    "all",
    "any",
    "as",
    "ast",
    "b",
    "c",
    "chain",
    "collect",
    "cont",
    "contains",
    "continue",
    "copied",
    "decl",
    "default",
    "dir",
    "distinct",
    "else",
    "entry",
    "enumerate",
    "extent",
    "f",
    "false",
    "find",
    "flatten",
    "for",
    "form",
    "found",
    "from",
    "func",
    "had_error",
    "hi",
    "i",
    "idx",
    "if",
    "in",
    "in_p5",
    "in_t2",
    "inc",
    "into_iter",
    "ir",
    "iter",
    "iv_of",
    "ivs",
    "k",
    "ka",
    "kb",
    "keep",
    "kept",
    "kind",
    "let",
    "lo",
    "map",
    "match",
    "matches",
    "max",
    "mdrv_emit",
    "meets",
    "min",
    "mut",
    "n",
    "net",
    "new",
    "or_default",
    "out",
    "p",
    "pairs",
    "per",
    "pid",
    "proc",
    "procedural",
    "push",
    "return",
    "row",
    "rule",
    "self",
    "side",
    "site",
    "skip",
    "sort_by_key",
    "sort_unstable_by_key",
    "sweep",
    "t0",
    "t1",
    "then_some",
    "true",
    "type_param",
    "u32",
    "u64",
    "usize",
    "via",
    "view",
    "w",
    "wext",
    "who",
    "writer",
    "writes",
    "x",
    "x_side",
    "xs",
    "y",
    "y_side",
];
/// H10m: the reader's message fns (`mdrv_mix.rs`).
const H10M_FNS: [&str; 3] = ["mdrv_emit", "mdrv_net_name", "mdrv_writer"];
/// H10m: every identifier (and format capture) the message fns may spell: the view's
/// `Write` fields (`writer`, `via`, `at`), the pair's (`rule`, `net`, `lo`, `hi`), the net's one
/// `symbols` key through `display_of`, `span_file_line_col`, `error`, `cur_span`,
/// `cur_prefix`, the file's consts and std / language words.
#[rustfmt::skip]
const H10M_ALLOW: &[&str] = &[
    "ASSIGN",
    "A_CLAUSES",
    "A_KINDS",
    "Body",
    "Cont",
    "ElabMultidriver",
    "MsgCode",
    "None",
    "PORT",
    "Proc",
    "Rule",
    "Some",
    "TaskInline",
    "VAR_INIT",
    "Via",
    "Writer",
    "X",
    "Y",
    "_",
    "a",
    "at",
    "b",
    "c",
    "clause",
    "col",
    "cur_prefix",
    "cur_span",
    "display_of",
    "error",
    "false",
    "file",
    "filter",
    "find",
    "format",
    "hi",
    "id",
    "if",
    "iter",
    "k",
    "kind",
    "let",
    "line",
    "lo",
    "map",
    "map_or",
    "match",
    "mdrv_net_name",
    "mdrv_writer",
    "mem",
    "msg",
    "mut",
    "n",
    "name",
    "net",
    "p",
    "pos",
    "prefix",
    "return",
    "rule",
    "self",
    "sp",
    "span",
    "span_file_line_col",
    "std",
    "symbols",
    "take",
    "to_string",
    "true",
    "unwrap_or_else",
    "via",
    "w",
    "wa",
    "wb",
    "what",
    "writer",
    "x",
    "zip",
];
/// H10: names neither list may hold (the self-check of the two lists).
const H10_NEVER: [&str; 13] = [
    "proc_idents",
    "ca_idents",
    "frame_func_names",
    "frame_decl_locs",
    "span_resolver",
    "nets",
    "intro_kind",
    "net_names",
    "rsplit",
    "strip_prefix",
    "scope",
    "cont_var",
    "reported",
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
                "record_geom"
                | "carry_geom"
                | "record_row_at"
                | "wext_unlocated"
                | "wext_located"
                | "wext_body_enter"
                | "wext_body_enter_keep"
                | "wext_body_exit" => out.push((format!("H7:{}", tx(i)), k.clone())),
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
        // H10 (§5.2 Q-MDRVMIX) a kind string of the closed vocabulary, as a whole literal
        if t[i].kind == Kind::Lit {
            if let Some(v) = KINDS.iter().find(|v| t[i].text == format!("\"{v}\"")) {
                out.push((format!("H10:kind:{v}"), k.clone()));
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

/// H10: the names a format literal captures (`{name}`, `{name:spec}`, a `name$` width or
/// precision in the spec) ; `{{` is no capture and a positional `{0}` names nothing.
fn captures(lit: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b: Vec<char> = lit.chars().collect();
    let ident = |s: &str| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
            && s.chars().all(|c| c.is_alphanumeric() || c == '_')
    };
    let mut i = 0;
    while i < b.len() {
        if b[i] == '{' {
            if b.get(i + 1) == Some(&'{') {
                i += 2;
                continue;
            }
            let end = (i + 1..b.len()).find(|&j| b[j] == '}').unwrap_or(b.len());
            let inner: String = b[i + 1..end].iter().collect();
            let (arg, spec) = inner.split_once(':').unwrap_or((inner.as_str(), ""));
            if ident(arg) {
                out.push(arg.to_string());
            }
            for (j, _) in spec.match_indices('$') {
                let start = spec[..j]
                    .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .map_or(0, |x| x + 1);
                if ident(&spec[start..j]) {
                    out.push(spec[start..j].to_string());
                }
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// H10: per fn of `src` named in `fns`, every identifier token (any receiver) and every
/// name a string literal captures, as (fn, name).
fn h10_idents(src: &str, fns: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for x in tokens(src) {
        if !fns.contains(&x.key.as_str()) {
            continue;
        }
        match x.kind {
            Kind::Id => out.push((x.key.clone(), x.text.clone())),
            Kind::Lit if x.text.ends_with('"') => {
                for c in captures(&x.text) {
                    out.push((x.key.clone(), c));
                }
            }
            _ => {}
        }
    }
    out
}

/// H10: the (fn, name) pairs of `src`'s `fns` that `allow` does not hold, sorted, unique.
fn h10_offenders(src: &str, fns: &[&str], allow: &[&str]) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = h10_idents(src, fns)
        .into_iter()
        .filter(|(_, n)| !allow.contains(&n.as_str()))
        .collect();
    v.sort();
    v.dedup();
    v
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
            "H7:wext_body_enter" | "H7:wext_body_enter_keep" => *n as i64,
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
        "check_mixed_drivers",
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
fn h3_built_once_read_through_one_entry() {
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

/// H10 (§5.2 Q-MDRVMIX C1): what the view's one reader reads (module head). The kind
/// spellings are census rows (`H10`, filtered to `mdrv_mix.rs`) ; the decision and the
/// message fns are checked against their allow lists, and each listed fn must exist.
#[test]
fn h10_the_mixed_driver_reader() {
    let got: Table = rows_of(&census(), "H10")
        .into_iter()
        .filter(|((_, f, _), _)| f == "mdrv_mix.rs")
        .collect();
    let want = want(H10);
    if got != want {
        let mut s = String::new();
        for ((r, f, k), n) in &got {
            s.push_str(&format!("    (\"{r}\", \"{f}\", \"{k}\", {n}, \"\"),\n"));
        }
        panic!("H10: the census moved.\n--- got ---\n{s}--- want ---\n{want:?}");
    }
    for n in H10_NEVER {
        assert!(
            !H10D_ALLOW.contains(&n) && !H10M_ALLOW.contains(&n),
            "H10: `{n}` is on an allow list"
        );
    }
    for n in ["at", "symbols", "display_of", "span_file_line_col"] {
        assert!(
            !H10D_ALLOW.contains(&n),
            "H10d: `{n}` is on the decision list"
        );
    }
    let root = workspace_root();
    let src = std::fs::read_to_string(root.join(SRC).join("mdrv_mix.rs")).expect("mdrv_mix.rs");
    let t = tokens(&src);
    for f in H10D_FNS.iter().chain(H10M_FNS.iter()) {
        assert!(
            t.windows(2).any(|w| w[0].text == "fn" && w[1].text == *f),
            "H10: `{f}` is not a fn of mdrv_mix.rs any more"
        );
    }
    let d = h10_offenders(&src, &H10D_FNS, H10D_ALLOW);
    assert!(
        d.is_empty(),
        "H10d: a decision fn reads a name off its list: {d:?}"
    );
    let m = h10_offenders(&src, &H10M_FNS, H10M_ALLOW);
    assert!(
        m.is_empty(),
        "H10m: a message fn reads a name off its list: {m:?}"
    );
}

/// H10k (§5.2 Q-MDRVMIX C3): the problems of one `mdrv_mix.rs` source against the rule —
/// `A_KINDS` and `A_CLAUSES` each declared once as `const NAME : [ & str ; N ]`, the same
/// `N`, each initializer holding `N` entries, and no `KINDS` string inside a fn (the
/// walker's key is `-` outside every fn). Empty = the rule holds.
fn h10k_problems(src: &str) -> Vec<String> {
    let t = tokens(src);
    let tx = |i: usize| t.get(i).map_or("", |x| x.text.as_str());
    let mut out = Vec::new();
    let mut lens: Vec<(String, String, usize)> = Vec::new();
    for name in ["A_KINDS", "A_CLAUSES"] {
        let at: Vec<usize> = (0..t.len())
            .filter(|&i| tx(i) == "const" && tx(i + 1) == name)
            .collect();
        if at.len() != 1 {
            out.push(format!("{name} declared {} times", at.len()));
            continue;
        }
        let i = at[0];
        // const NAME : [ & str ; N ] = [ e , e , .. ] ;
        let ty = i + 3;
        if tx(i + 2) != ":" || tx(ty) != "[" {
            out.push(format!("{name}: not an array const"));
            continue;
        }
        let ty_close = close_of(&t, ty);
        let n = tx(ty_close - 1).to_string();
        let init = ty_close + 2;
        if tx(ty_close + 1) != "=" || tx(init) != "[" {
            out.push(format!("{name}: no array initializer"));
            continue;
        }
        let close = close_of(&t, init);
        let entries = (init + 1..close).filter(|&j| tx(j) != ",").count();
        lens.push((name.to_string(), n, entries));
    }
    if let [(_, na, ea), (_, nc, ec)] = lens.as_slice() {
        if na != nc || ea != ec || na != &ea.to_string() {
            out.push(format!(
                "A_KINDS [{na}; {ea} entries] and A_CLAUSES [{nc}; {ec} entries] differ"
            ));
        }
    }
    for x in &t {
        if x.kind == Kind::Lit && x.key != "-" && KINDS.iter().any(|v| x.text == format!("\"{v}\""))
        {
            out.push(format!("kind {} spelled in fn {}", x.text, x.key));
        }
    }
    out
}

/// H10k (§5.2 Q-MDRVMIX C3): rule Y's A-kind list and its clause table (module head).
#[test]
fn h10k_one_a_kind_list_and_its_clause_table() {
    let root = workspace_root();
    let src = std::fs::read_to_string(root.join(SRC).join("mdrv_mix.rs")).expect("mdrv_mix.rs");
    let p = h10k_problems(&src);
    assert!(p.is_empty(), "H10k: {p:?}");
}

/// H-W1: the `MsgCode :: ElabMultidriverStrict` sites of `files` (non-comment, non-test:
/// the walker drops comments and `#[cfg(test)]` items), per (file, fn).
fn hw1_sites(files: &[(String, String)]) -> BTreeMap<(String, String), usize> {
    let mut out: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (rel, src) in files {
        let f = rel.trim_start_matches("crates/elaborate/src/").to_string();
        if f.starts_with("tests/") {
            continue;
        }
        let t = tokens(src);
        for i in 0..t.len().saturating_sub(2) {
            if t[i].text == "MsgCode"
                && t[i + 1].text == "::"
                && t[i + 2].text == "ElabMultidriverStrict"
            {
                *out.entry((f.clone(), t[i].key.clone())).or_default() += 1;
            }
        }
    }
    out
}

/// H-W1 (§5.2 Q-MDRVMIX C3): W3060's two producers (module head).
#[test]
fn hw1_two_w3060_producers() {
    let root = workspace_root();
    let got = hw1_sites(&files_in(&root, SRC));
    let want: BTreeMap<(String, String), usize> = [(
        (
            "multidriver.rs".to_string(),
            "check_multidriver_processes".to_string(),
        ),
        2,
    )]
    .into_iter()
    .collect();
    assert_eq!(got, want, "H-W1: W3060's producers moved");
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
    // H3 (§5.2 Q-MDRVMIX): a second reader file of the view is a row H3 does not hold
    let second = table_of(
        "mdrv_two.rs",
        "impl E { fn g(&self) -> usize { self.wext.writes.len() } }",
    );
    assert!(
        second.keys().any(|(r, _, _)| r == "H3:wext")
            && !want(H3).keys().any(|(_, f, _)| f == "mdrv_two.rs"),
        "H3 planted: a second reader"
    );
    // H10 kinds: the whole literal only ; a comment drops
    assert_eq!(
        count(
            "H10",
            "fn f() { let s = \"always_comb is a keyword\"; let t = \"always\"; }"
        ),
        1,
        "H10 a kind is the whole literal"
    );
    assert_eq!(
        count("H10", "// \"assign\"\nfn f() {}"),
        0,
        "H10 kind in a comment"
    );
    // H10 the capture parser: `{{` is none, a positional names nothing, a spec's `name$`
    assert_eq!(
        captures("\"{a} {{b}} {0} {c:>wd$} {d:.pr$} {:?}\""),
        vec!["a", "c", "wd", "d", "pr"],
        "H10 captures"
    );
    // H10d planted: a reported set through a local, a position, a name — each must fail
    let d = "impl E { fn pairs(&self) { let s = &*self; s.cont_var.reported.contains(&n); \
             let q = w.at; self.symbols.len(); } }";
    let off = h10_offenders(d, &H10D_FNS, H10D_ALLOW);
    for n in ["cont_var", "reported", "at", "symbols"] {
        assert!(
            off.iter().any(|(_, x)| x == n),
            "H10d planted `{n}`: {off:?}"
        );
    }
    // H10m planted: each former failure site's read (the re-entry's (b)) must fail
    for (body, n) in [
        ("self.proc_idents.get(0);", "proc_idents"),
        ("self.frame_func_names.get(0);", "frame_func_names"),
        ("self.ca_idents.get(0);", "ca_idents"),
        ("let v = w; v.rsplit('.');", "rsplit"),
        ("self.nets.get(0);", "nets"),
        ("self.intro_kind.get(&n);", "intro_kind"),
        ("format!(\"{port}\");", "port"),
        ("format!(\"{x:>wd$}\", x = 1);", "wd"),
        ("format!(\"{x:.pr$}\", x = 1);", "pr"),
    ] {
        let src = format!("impl E {{ fn mdrv_writer(&self) {{ {body} }} }}");
        let off = h10_offenders(&src, &H10M_FNS, H10M_ALLOW);
        assert!(
            off.iter().any(|(_, x)| x == n),
            "H10m planted `{n}`: {off:?}"
        );
    }
    // H10m negative control: the same tokens in a comment and under `#[cfg(test)]`
    let quiet =
        "impl E { fn mdrv_writer(&self) {\n // self.proc_idents \"{port}\" self.nets\n } }\n\
                 #[cfg(test)]\nmod t { fn mdrv_writer(&self) { self.proc_idents.get(0); \
                 format!(\"{port}\"); self.nets.get(0); } }";
    assert!(
        h10_offenders(quiet, &H10M_FNS, H10M_ALLOW).is_empty(),
        "H10m comment / cfg(test)"
    );
    // H10m real site spelled back (`mdrv_mix.rs` `mdrv_net_name`): on the list
    let back = "impl E { fn mdrv_net_name(&self, net: u32) -> String { self.symbols.iter()\
                .find(|(_, &id)| id == net).map(|(n, _)| self.display_of(n))\
                .unwrap_or_else(|| format!(\"#{net}\")) } }";
    assert!(
        h10_offenders(back, &H10M_FNS, H10M_ALLOW).is_empty()
            && h10_idents(back, &H10M_FNS).len() > 10,
        "H10m real site spelled back"
    );
    // H10d planted (C2, rule Y): y_side reading a position, a name or a process's identity
    // row, and `in_p5` reading the declared bounds — each must fail
    let d = "impl E { fn y_side(w: &Write) { let q = w.at; self.symbols.len(); \
             self.proc_idents.get(0); } fn in_p5(k: &str) { self.nets.get(0); } }";
    let off = h10_offenders(d, &H10D_FNS, H10D_ALLOW);
    for (f, n) in [
        ("y_side", "at"),
        ("y_side", "symbols"),
        ("y_side", "proc_idents"),
        ("in_p5", "nets"),
    ] {
        assert!(
            off.iter().any(|(g, x)| g == f && x == n),
            "H10d planted `{f}` `{n}`: {off:?}"
        );
    }
    // H10d real site spelled back (`mdrv_mix.rs` `y_side`): on the list, and H9 counts its
    // `Via::Direct`
    let back = "fn y_side(w: &Write) -> Option<(bool, bool)> { match w.writer { \
                Writer::Proc { kind, .. } if in_p5(kind) && procedural(w) => { \
                Some((kind == ALWAYS_COMB && w.via == Some(Via::Direct), true)) } _ => None } }";
    assert!(
        h10_offenders(back, &H10D_FNS, H10D_ALLOW).is_empty()
            && h10_idents(back, &H10D_FNS).len() > 10
            && count("H9", back) == 1,
        "H10d real site spelled back (y_side)"
    );
    // H10m planted (C2): rule Y's sentence capturing a scope or reading a routine name
    for (body, n) in [
        ("format!(\"variable `{name}` in {scope}\");", "scope"),
        ("self.frame_func_names.get(0);", "frame_func_names"),
    ] {
        let src = format!("impl E {{ fn mdrv_emit(&mut self) {{ {body} }} }}");
        let off = h10_offenders(&src, &H10M_FNS, H10M_ALLOW);
        assert!(
            off.iter().any(|(_, x)| x == n),
            "H10m planted (mdrv_emit) `{n}`: {off:?}"
        );
    }
    // H10k (C3): the real shape spelled back holds ; a fourth kind without a clause, a kind
    // spelled inline in y_side, a second list each fail ; a comment and cfg(test) are quiet
    let real =
        "const ALWAYS_COMB: &str = \"always_comb\"; const ALWAYS_LATCH: &str = \"always_latch\"; \
                const ALWAYS_FF: &str = \"always_ff\"; \
                const A_KINDS: [&str; 3] = [ALWAYS_COMB, ALWAYS_LATCH, ALWAYS_FF]; \
                const A_CLAUSES: [&str; 3] = [\"9.2.2.2\", \"9.2.2.3\", \"9.2.2.4\"]; \
                fn y_side(w: &Write) -> bool { A_KINDS.contains(&w.kind) }";
    assert!(
        h10k_problems(real).is_empty(),
        "H10k real shape: {:?}",
        h10k_problems(real)
    );
    for (planted, what) in [
        (
            real.replace(
                "[&str; 3] = [ALWAYS_COMB, ALWAYS_LATCH, ALWAYS_FF]",
                "[&str; 4] = [ALWAYS_COMB, ALWAYS_LATCH, ALWAYS_FF, \"always\"]",
            ),
            "a fourth kind without a clause",
        ),
        (
            real.replace("A_KINDS.contains(&w.kind)", "w.kind == \"always_ff\""),
            "a kind inline in y_side",
        ),
        (
            format!("{real} const A_KINDS: [&str; 1] = [ALWAYS_COMB];"),
            "a second A list",
        ),
    ] {
        assert!(!h10k_problems(&planted).is_empty(), "H10k planted: {what}");
    }
    let quiet = format!(
        "{real}\nfn f() {{ // \"always_ff\"\n }}\n#[cfg(test)]\nmod t {{ fn g() {{ let k = \"always_ff\"; }} }}"
    );
    assert!(h10k_problems(&quiet).is_empty(), "H10k comment / cfg(test)");
    // H-W1 (C3): a third W3060 producer fails ; a comment and cfg(test) are quiet ; the real
    // latch-arm site spelled back counts once in its fn
    let site = |body: &str| {
        vec![(
            "crates/elaborate/src/multidriver.rs".to_string(),
            format!("impl E {{ fn check_multidriver_processes(&mut self) {{ {body} }} }}"),
        )]
    };
    let one = "self.warn_code_at(MsgCode::ElabMultidriverStrict, span, &m);";
    let n_of = |fs: Vec<(String, String)>| hw1_sites(&fs).values().sum::<usize>();
    assert_eq!(n_of(site(one)), 1, "H-W1 real site spelled back");
    assert_eq!(
        n_of(site(&format!("{one} {one} {one}"))),
        3,
        "H-W1 planted: a third producer"
    );
    assert_eq!(
        n_of(vec![(
            "crates/elaborate/src/x.rs".to_string(),
            format!("// {one}\n#[cfg(test)]\nmod t {{ fn g() {{ {one} }} }}")
        )]),
        0,
        "H-W1 comment / cfg(test)"
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
        "impl E { fn g(&mut self) { let t = self.wext_body_enter(); } fn h(&mut self) { let t = s.wext_body_enter_keep(); s.wext_body_exit(t); } }",
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
        h7c3("impl E { fn inline_task_body(&mut self, b: &mut B, body: &ast::Stmt) { self.wext_located(|s| { s.lower_stmt(b, body); }); } }"),
        vec!["H7:wext_located@inline_task_body".to_string()],
        "H7 real site (inline_task.rs inline_task_body, a callee entry) spelled back"
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
        "impl E { fn lower_proc_block(&mut self) { let mut b = B::new(); let t = self.wext_body_enter(); self.lower_stmt(&mut b, &p.body); self.wext_body_exit(t); } }",
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
        "impl E { fn lower_proc_block(&mut self) { let mut b = B::new(); let t = self.wext_body_enter_keep(); self.lower_stmt(&mut b, &p.body); self.wext_body_exit(t); } }",
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
    let at = |r: &str| {
        tab.get(&(
            r.to_string(),
            "stmt_flow.rs".to_string(),
            "lower_proc_block".to_string(),
        ))
        .copied()
        .unwrap_or(0)
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

/// H3: built once, handed to `Sidecars`, read through one entry.
#[rustfmt::skip]
const H3: &[(&str, &str, &str, usize, &str)] = &[
    ("H3:build_view", "wext.rs", "build_write_view", 1, "the one build"),
    ("H3:build_write_view", "driver.rs", "run", 1, "the end phase, before check_whole_net_multidriver"),
    ("H3:wext", "api.rs", "elaborate_located_params", 1, "the hand-off to Sidecars::write_view"),
    ("H3:wext", "mdrv_mix.rs", "check_mixed_drivers", 1, "the one reader's one entry (§5.2 Q-MDRVMIX)"),
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
    ("H7:wext_body_enter", "class_lower.rs", "lower_class_method_body", 1, "the method body's floor: save, set 0 (the callee entry of a method), restore"),
    ("H7:wext_body_enter", "frames_body.rs", "lower_frame_body_inner_lower_frame_func_body", 1, "a function body's floor: save, set 0, restore"),
    ("H7:wext_body_enter", "frames_body.rs", "lower_frame_body_inner_lower_frame_task_body", 1, "a task body's floor: save, set 0, restore"),
    ("H7:wext_body_enter_keep", "stmt_flow.rs", "lower_proc_block", 1, "the process root's floor: save and restore, the mask KEPT (premise judge line 1)"),
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
    ("H7:wext_rec", "wext.rs", "wext_body_enter", 2, "a body root: open count, mask saved and set 0"),
    ("H7:wext_rec", "wext.rs", "wext_body_enter_keep", 2, "the process root: open count, mask saved and kept"),
    ("H7:wext_rec", "wext.rs", "wext_body_exit", 3, "a body root's exit: open count, mask restored, the floor"),
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
    ("H8:DeclSite::AnsiPort", "mdrv_mix.rs", "in_t2", 1, "the target set's reader (§5.2 Q-MDRVMIX T2)"),
    ("H8:DeclSite::AnsiPort", "ports.rs", "elaborate_ports", 1, "the ANSI port writer"),
    ("H8:DeclSite::Generate", "mdrv_mix.rs", "in_t2", 1, "the target set's reader (§5.2 Q-MDRVMIX T2)"),
    ("H8:DeclSite::Generate", "wext.rs", "body_site", 1, "the body writer's generate site"),
    ("H8:DeclSite::Module", "mdrv_mix.rs", "in_t2", 1, "the target set's reader (§5.2 Q-MDRVMIX T2)"),
    ("H8:DeclSite::Module", "wext.rs", "body_site", 1, "the body writer's module site"),
    ("H8:DeclSite::NonAnsiPort", "instance.rs", "elaborate_instance", 1, "the non-ANSI port writer"),
    ("H8:DeclSite::NonAnsiPort", "mdrv_mix.rs", "in_t2", 1, "the target set's reader (§5.2 Q-MDRVMIX T2)"),
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
    ("H9:Via::Direct", "mdrv_mix.rs", "procedural", 1, "rule X's procedural writer"),
    ("H9:Via::Direct", "mdrv_mix.rs", "y_side", 1, "rule Y's A side: an `always_comb`'s own text only"),
    ("H9:Via::Direct", "wext.rs", "build_view", 2, "the default of a procedural statement"),
    ("H9:Via::Direct", "wext.rs", "render", 1, "`direct`"),
    ("H9:Via::TaskInline", "inline_task.rs", "inline_task", 1, "the inline decision"),
    ("H9:Via::TaskInline", "mdrv_mix.rs", "mdrv_writer", 1, "the message's \"through a subroutine it calls\""),
    ("H9:Via::TaskInline", "mdrv_mix.rs", "procedural", 1, "rule X's procedural writer"),
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

/// H10: the kind spellings of `mdrv_mix.rs`, each once, in its consts.
#[rustfmt::skip]
const H10: &[(&str, &str, &str, usize, &str)] = &[
    ("H10:kind:always", "mdrv_mix.rs", "-", 1, "P6"),
    ("H10:kind:always_comb", "mdrv_mix.rs", "-", 1, "ALWAYS_COMB, P6's and A_KINDS' member"),
    ("H10:kind:always_ff", "mdrv_mix.rs", "-", 1, "ALWAYS_FF, P6's and A_KINDS' member (C3)"),
    ("H10:kind:always_latch", "mdrv_mix.rs", "-", 1, "ALWAYS_LATCH, P6's and A_KINDS' member (C3)"),
    ("H10:kind:assign", "mdrv_mix.rs", "-", 1, "ASSIGN, CONT's member the message names"),
    ("H10:kind:initial", "mdrv_mix.rs", "-", 1, "P6"),
    ("H10:kind:port", "mdrv_mix.rs", "-", 1, "PORT, CONT's member the message names"),
    ("H10:kind:var_init", "mdrv_mix.rs", "-", 1, "VAR_INIT, P6's member the message names"),
];
