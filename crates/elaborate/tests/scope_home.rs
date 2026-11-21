//! The scope censuses of §5.2 Q-LEXSCOPE (unit LS1, part 1), as home tests: five syntax
//! censuses of `crates/elaborate/src/**/*.rs`, each computed by `syn` (full) and compared
//! with a table pinned in this file. A census row is keyed by (file, fn): `fn` is the
//! enclosing fn's name, nested fns joined by `::`, a method by its own name (no impl
//! type), a second fn of the same key in one file suffixed `#2`. Items under
//! `#[cfg(test)]` and out-of-line `#[cfg(test)] mod` files are not read.
//!
//! - H2 the statement walkers (63 = 62 by syntax + `select_scope.rs` `blocks`, a HAND row
//!   whose pinned calls are checked): every fn whose own patterns name `ast::Stmt::Block` /
//!   `Fork`, or that calls `for_each_nested_stmt`, or that descends a carrier through a
//!   walker it calls, and the descent itself; per row its class, the `ast::Stmt` variants
//!   its patterns name (the arms), its catch-all arms (`_`, a plain binding, a guarded
//!   one), whether it calls `for_each_nested_stmt`, the statement carriers falling to its
//!   catch-all, and three hand columns: the catch-all KIND (g2b catchalls.tsv, 37), the
//!   reader MODEL (g2b readers.tsv, 40) and the reader's ROLE (g3 askers.tsv's class for
//!   the 29 askers, DELTA D1's kind for the 9 LEXICAL readers, BINDER for 2).
//! - H3 the writes of `Elaborator::cur_prefix` (25 sites): per (file, fn) the write forms
//!   (`mem::replace` / `mem::take` / `mem::swap` of `&mut <recv>.cur_prefix`, another
//!   `&mut`, an assignment, a mutating method, a struct-literal field) and the sites they
//!   make: an opener (`replace` / `take` / `swap`), a struct literal, and one site per fn
//!   for the assignments no opener of an enclosing block covers ; the kind per site is a
//!   hand column (u0 T2.txt:281-315).
//! - H6 the scope-walk call sites: every call of the walk family (`walk_scopes_key`,
//!   `_shadowed`, `_from`, `scoped_key_at_or_above`, `walk_scopes`, `walk_rtn_scopes`,
//!   `hier_resolve`, `decl_probe_key`, and the family's own `walk_scopes_key_inner` /
//!   `walk_out_from`), by (file, fn, entry, hit text) with a count ; the hit text is what
//!   the hit argument probes in source order (a field of `self` / `me`, a `hit_*` method,
//!   a local closure `name()`, a local table) and the hit SET expands each `hit_*` method
//!   and local closure to the fields it probes. Pinned: 51 sites outside the family, 1
//!   `walk_rtn_scopes`, 10 `hier_resolve`, 1 `decl_probe_key`.
//! - H6c the composites (103): g2b's census_g2b rule — per fn, in source order, every
//!   consult of a walk / binder entry, a stack accessor or a K1 class (ii) / (iii) field ;
//!   a row when the fn names two field / stack sources, or one and a walk entry (A), or two
//!   walk entries on one reduced first argument (C) ; per row its tier (D: two sources on
//!   one reduced argument, writes and `fq` excluded) and the hand class (g2b
//!   composites.tsv).
//! - H7 the fn-local name collections: every `let` whose declared collection type, or the
//!   collection its initializer constructs, names `String` or `str`, per (file, fn) with
//!   the bound names ; every `let` with no declared type whose initializer calls a crate fn
//!   whose declared return type is such a collection ; and the 26 readers' collections
//!   (g2b REPORT:70-75), each found by syntax where the list says it is.
//!
//! A difference between a census and its table fails with the row and what to do: a NEW
//! row names the unit that owns it (the row of ROADMAP §5.2 that adds it) and enters this
//! table in the same commit ; a GONE row leaves the table in the commit that removes it.
//! A hand column (KIND, MODEL, ROLE, the H3 kinds, the H6c class) is a pin, not a census:
//! a row whose hand column is unknown gets one by reading, never by default.
//!
//! What the syntax does not see is REVIEW-ONLY: a walker reached through a closure
//! variable or a fn pointer, a macro body that does not parse as an expression list (H2
//! fails on one that names `Stmt`), a composite split over a helper, a collection built
//! by `.collect()` into an inferred type or inside a closure, a `cur_prefix` write
//! through a `&mut String` bound earlier, a hit probing a field through a receiver not
//! spelled `self` / `me` / `this`. An UNTYPED row's callee is matched by its simple
//! name over every crate fn, so a same-named fn of another return type also counts.
//!
//! The tables' sources (the oracle of each pin, W/scopebind): g1 walkers.tsv, g2b
//! catchalls.tsv / readers.tsv / composites.tsv / census.locals.tsv / REPORT.txt, g3
//! askers.tsv / REPORT.txt K5, plan3 walk_sites.txt, u0 t2/T2.txt, plan4 DELTA.txt D1,
//! at main 87e48d99 ; every census number equals its source's there.
//!
//! Environment (a negative control's door, unset in the gate): `SCOPE_HOME_SRC` reads
//! another copy of the crate's `src` ; `SCOPE_HOME_DUMP=<dir>` writes each computed
//! census to `<dir>/<H>.tsv` before comparing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Expr, Pat, Token, UseTree};

// =====================================================================================
// The source
// =====================================================================================

struct Src {
    rel: String,
    text: String,
    file: syn::File,
}

fn src_dir() -> PathBuf {
    match std::env::var("SCOPE_HOME_SRC") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
    }
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && match &a.meta {
                syn::Meta::List(l) => {
                    let s = l.tokens.to_string();
                    s.contains("test") && !s.contains("not")
                }
                _ => false,
            }
    })
}

/// The crate's source files, `#[cfg(test)] mod x;` files left out ; every file must parse.
fn load_src() -> Vec<Src> {
    let dir = src_dir();
    let mut paths = Vec::new();
    rs_files(&dir, &mut paths);
    let mut parsed = Vec::new();
    for p in &paths {
        let text =
            std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
        let file = syn::parse_file(&text)
            .unwrap_or_else(|e| panic!("syn cannot parse {}: {e}", p.display()));
        parsed.push((p.clone(), text, file));
    }
    let mut excluded: Vec<PathBuf> = Vec::new();
    for (p, _, file) in &parsed {
        let stem = p.file_stem().expect("stem").to_string_lossy().to_string();
        let parent = p.parent().expect("parent").to_path_buf();
        let base = if stem == "lib" || stem == "mod" || stem == "main" {
            parent
        } else {
            parent.join(&stem)
        };
        for it in &file.items {
            if let syn::Item::Mod(m) = it {
                if m.content.is_none() && has_cfg_test(&m.attrs) {
                    let n = m.ident.to_string();
                    excluded.push(base.join(format!("{n}.rs")));
                    excluded.push(base.join(&n));
                }
            }
        }
    }
    parsed
        .into_iter()
        .filter(|(p, _, _)| !excluded.iter().any(|x| p.starts_with(x)))
        .map(|(p, text, file)| Src {
            rel: p
                .strip_prefix(&dir)
                .expect("under src")
                .to_string_lossy()
                .replace('\\', "/"),
            text,
            file,
        })
        .collect()
}

/// Source with comments, string and char literals blanked (newlines kept), for finding
/// a fn's `fn <name>` line (this `syn` build has no span locations).
fn blank(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let mut o = b.to_vec();
    let n = b.len();
    let fill = |o: &mut Vec<u8>, from: usize, to: usize| {
        for c in o.iter_mut().take(to.min(n)).skip(from) {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    let mut i = 0;
    while i < n {
        let c = b[i];
        if c == b'/' && i + 1 < n && b[i + 1] == b'/' {
            let s = i;
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            fill(&mut o, s, i);
        } else if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
            let s = i;
            let mut depth = 0;
            while i < n {
                if b[i] == b'/' && i + 1 < n && b[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if b[i] == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            fill(&mut o, s, i);
        } else if c == b'r'
            && i + 1 < n
            && (b[i + 1] == b'"' || b[i + 1] == b'#')
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
        {
            let s = i;
            let mut j = i + 1;
            let mut hashes = 0;
            while j < n && b[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < n && b[j] == b'"' {
                j += 1;
                while j < n {
                    if b[j] == b'"' {
                        let mut k = 0;
                        while k < hashes && j + 1 + k < n && b[j + 1 + k] == b'#' {
                            k += 1;
                        }
                        if k == hashes {
                            j += 1 + hashes;
                            break;
                        }
                    }
                    j += 1;
                }
                fill(&mut o, s, j);
                i = j;
            } else {
                i += 1;
            }
        } else if c == b'"' {
            let s = i;
            i += 1;
            while i < n && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            fill(&mut o, s, i);
        } else if c == b'\'' {
            if i + 2 < n && b[i + 1] == b'\\' {
                let s = i;
                i += 2;
                while i < n && b[i] != b'\'' {
                    i += 1;
                }
                i += 1;
                fill(&mut o, s, i);
            } else if i + 2 < n && b[i + 2] == b'\'' {
                fill(&mut o, i, i + 3);
                i += 3;
            } else {
                let rest = &src[i + 1..];
                let mut it = rest.char_indices();
                if let (Some((_, ch)), Some((k, '\''))) = (it.next(), it.next()) {
                    if ch != '\'' {
                        fill(&mut o, i, i + 1 + k + 1);
                        i += 1 + k + 1;
                        continue;
                    }
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    o
}

fn find_fn(blanked: &[u8], from: usize, name: &str) -> Option<usize> {
    let n = blanked.len();
    let mut i = from;
    while i + 2 < n {
        if &blanked[i..i + 2] == b"fn"
            && (i == 0 || !(blanked[i - 1].is_ascii_alphanumeric() || blanked[i - 1] == b'_'))
            && blanked[i + 2].is_ascii_whitespace()
        {
            let mut j = i + 2;
            while j < n && blanked[j].is_ascii_whitespace() {
                j += 1;
            }
            let k0 = j;
            while j < n && (blanked[j].is_ascii_alphanumeric() || blanked[j] == b'_') {
                j += 1;
            }
            if &blanked[k0..j] == name.as_bytes() {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

// =====================================================================================
// One pass over the source: per fn, what each census reads
// =====================================================================================

/// The 29 `ast::Stmt` variants.
const VARIANTS: &[&str] = &[
    "Blocking",
    "NonBlocking",
    "If",
    "Return",
    "Case",
    "For",
    "While",
    "Repeat",
    "Forever",
    "Block",
    "Fork",
    "SysTaskCall",
    "UserTaskCall",
    "RandomizeWith",
    "DelayCtrl",
    "EventCtrl",
    "EventTrigger",
    "Wait",
    "Disable",
    "WaitFork",
    "ConcurrentAssert",
    "Assign",
    "Deassign",
    "Force",
    "Release",
    "DeferredAssert",
    "CoverProperty",
    "Null",
    "Error",
];
/// The variants that hold a nested statement (the carriers).
const CARRIERS: &[&str] = &[
    "If",
    "Case",
    "For",
    "While",
    "Repeat",
    "Forever",
    "Block",
    "Fork",
    "DelayCtrl",
    "EventCtrl",
    "Wait",
    "DeferredAssert",
    "ConcurrentAssert",
];

/// H6: the walk family's entries ; the hit argument's place counted from the end.
const WALK_ENTRIES: &[(&str, usize)] = &[
    ("walk_scopes_key", 1),
    ("walk_scopes_key_shadowed", 1),
    ("walk_scopes_key_from", 1),
    ("scoped_key_at_or_above", 1),
    ("walk_scopes", 1),
    ("walk_rtn_scopes", 1),
    ("hier_resolve", 1),
    ("decl_probe_key", 1),
    ("walk_scopes_key_inner", 2),
    ("walk_out_from", 3),
];
/// H6: the five entries whose call sites are THE walk census (51).
const WALK_SITE_ENTRIES: &[&str] = &[
    "walk_scopes_key",
    "walk_scopes_key_shadowed",
    "walk_scopes_key_from",
    "scoped_key_at_or_above",
    "walk_scopes",
];
/// H6: the fns that ARE the walk (a call inside one is the walk's own body).
const WALK_FAMILY: &[&str] = &[
    "walk_scopes",
    "walk_scopes_key",
    "walk_scopes_key_shadowed",
    "walk_scopes_key_inner",
    "walk_scopes_key_from",
    "scoped_key_at_or_above",
    "walk_out_from",
];
/// H6: a receiver that is the `Elaborator`.
const RECV: &[&str] = &["self", "me", "this"];
/// H6: a probe of a local table.
const PROBE_METHODS: &[&str] = &[
    "contains_key",
    "contains",
    "get",
    "position",
    "get_key_value",
];

/// H6c (census_g2b): the walk / binder entries, the stack accessors, the K1 (ii) / (iii)
/// fields (last segment ; `name names nets funcs insts` left out as too generic).
const COMP_WALK: &[&str] = &[
    "walk_scopes_key",
    "walk_scopes_key_shadowed",
    "walk_scopes_key_from",
    "scoped_key_at_or_above",
    "walk_scopes",
    "lookup_scoped",
    "lookup_net_scoped",
    "lookup_net_unshadowed",
    "name_meta",
    "name_real",
    "bare_ident_route",
    "bare_name_binds_constant",
    "resolve_net",
    "pkg_body_var",
    "pkg_body_const_route",
    "bare_hit_is_shadowed_pkg_alias",
    "dotted_hit_is_pkg_alias",
    "block_local_declared_at",
    "block_local_covers",
    "walk_rtn_scopes",
    "fq",
    "lval_write_net",
    "lookup_dotted_net",
];
const COMP_STACK: &[&str] = &[
    "subst_lookup",
    "out_subst_lookup",
    "dyn_subst_lookup",
    "formal_is_string",
];
const COMP_FIELDS: &[&str] = &[
    "bits_prescan",
    "local_decl_names",
    "scoped_block_locals",
    "coalesced_block_locals",
    "per_entry_block_locals",
    "const_call_fn",
    "frame_idx",
    "task_frame_idx",
    "task_arg_locals",
    "array_iter",
    "module_facts",
    "per_entry_in_scope",
    "hier_called_task_names",
    "md_return_fns",
    "cond_census",
    "func_table",
    "const_func_table",
    "const_fn_pkg",
    "decl_wins",
    "task_table",
    "rtn_pkg",
    "rtn_decl_scope",
    "rtn_decl_genvars",
    "gen_text_scopes",
    "decl_pos",
    "decl_block_locals",
    "inout_func_names",
    "body_write_func_names",
    "dyn_formal_func_names",
    "seq_table",
    "prop_table",
    "let_table",
    "class_table",
    "clocking_events",
    "subst",
    "out_subst",
    "dyn_subst",
    "formal_str",
    "disable_stack",
    "cover_types",
    "declared",
    "safe",
    "carrier_stems",
    "real_nets",
    "body_write_funcs",
    "per_name",
    "vector_ports",
    "env",
    "envw",
];
const COMP_MUT: &[&str] = &[
    "push",
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
    "drain",
    "pop",
    "entry",
    "take",
    "replace",
    "push_str",
    "swap_remove",
    "split_off",
];

/// H7: the collection types.
const COLL: &[&str] = &[
    "HashSet", "BTreeSet", "HashMap", "BTreeMap", "Vec", "VecDeque", "IndexMap", "IndexSet",
];

/// H3: a method that mutates a `String` in place.
const STRING_MUT: &[&str] = &[
    "push",
    "push_str",
    "insert",
    "insert_str",
    "truncate",
    "clear",
    "pop",
    "drain",
    "replace_range",
    "retain",
    "extend",
    "clone_from",
    "as_mut_str",
    "remove",
    "make_ascii_lowercase",
    "make_ascii_uppercase",
];

#[derive(Default, Clone)]
struct Uses {
    names: BTreeMap<String, Vec<String>>,
    globs: Vec<Vec<String>>,
}

fn flatten_use(prefix: &mut Vec<String>, t: &UseTree, out: &mut Uses) {
    match t {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            flatten_use(prefix, &p.tree, out);
            prefix.pop();
        }
        UseTree::Name(n) => {
            let mut full = prefix.clone();
            full.push(n.ident.to_string());
            out.names.insert(n.ident.to_string(), full);
        }
        UseTree::Rename(r) => {
            let mut full = prefix.clone();
            full.push(r.ident.to_string());
            out.names.insert(r.rename.to_string(), full);
        }
        UseTree::Glob(_) => out.globs.push(prefix.clone()),
        UseTree::Group(g) => {
            for it in &g.items {
                flatten_use(prefix, it, out);
            }
        }
    }
}

fn block_uses(stmts: &[syn::Stmt]) -> Uses {
    let mut u = Uses::default();
    for s in stmts {
        if let syn::Stmt::Item(syn::Item::Use(iu)) = s {
            flatten_use(&mut Vec::new(), &iu.tree, &mut u);
        }
    }
    u
}

fn item_uses(items: &[syn::Item]) -> Uses {
    let mut u = Uses::default();
    for it in items {
        if let syn::Item::Use(iu) = it {
            flatten_use(&mut Vec::new(), &iu.tree, &mut u);
        }
    }
    u
}

fn is_ast_seg(s: &str) -> bool {
    s == "ast" || s == "hdl_ast"
}

fn is_ir_seg(s: &str) -> bool {
    s == "ir" || s == "sim_ir"
}

fn full_is_ast_stmt(full: &[String]) -> bool {
    full.last().is_some_and(|s| s == "Stmt") && full.iter().any(|s| is_ast_seg(s))
}

/// What the pass records per fn.
#[derive(Default)]
struct FnRec {
    file: String,
    qual: String,
    simple: String,
    line: usize,
    // H2
    variants: BTreeSet<String>,
    wildcard: usize,
    wildcard_guarded: usize,
    binding_arm: usize,
    self_call: bool,
    fens: bool,
    macro_unparsed_stmt: usize,
    calls: BTreeSet<String>,
    // H3: (form, block id the write sits in, the enclosing block ids)
    prefix_writes: Vec<(String, usize, Vec<usize>)>,
    // H6: (entry, probes as spelled)
    walk_calls: Vec<(String, Vec<String>)>,
    /// H6: local closures `let name = |..| ..` -> their probes
    local_closures: BTreeMap<String, Vec<String>>,
    /// H6: the probes of the fn's own body (read for `hit_*` fns)
    self_probes: Vec<String>,
    // H6c: (kind W / S / F / Fw, entry, reduced first argument)
    asks: Vec<(String, String, String)>,
    // H7: (binding, type idents) of a String collection `let` ; untyped lets' callee
    str_locals: Vec<(String, String)>,
    untyped_calls: Vec<(String, String)>,
    let_names: Vec<String>,
    /// H7: the fn's declared return type names a collection and `String` / `str`
    returns_str_coll: bool,
}

struct Pass<'a> {
    file: String,
    uses: Vec<Uses>,
    stack: Vec<usize>,
    out: &'a mut Vec<FnRec>,
    blocks: Vec<usize>,
    next_block: usize,
    /// H7: struct fields per (struct name) seen in this file
    struct_fields: &'a mut Vec<(String, String, String)>,
}

impl Pass<'_> {
    fn cur(&mut self) -> Option<&mut FnRec> {
        let i = *self.stack.last()?;
        Some(&mut self.out[i])
    }

    fn lookup_name(&self, n: &str) -> Option<&Vec<String>> {
        self.uses.iter().rev().find_map(|u| u.names.get(n))
    }

    fn glob_ast_stmt(&self) -> bool {
        self.uses
            .iter()
            .any(|u| u.globs.iter().any(|g| full_is_ast_stmt(g)))
    }

    /// The `ast::Stmt` variant a path names, if any (census_walkers.rs's resolution).
    fn stmt_variant(&self, path: &syn::Path) -> Option<String> {
        let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let last = segs.last()?.clone();
        if !VARIANTS.contains(&last.as_str()) {
            return None;
        }
        if segs.len() == 1 {
            if let Some(full) = self.lookup_name(&last) {
                let n = full.len();
                return (n >= 2 && full_is_ast_stmt(&full[..n - 1])).then_some(last);
            }
            return self.glob_ast_stmt().then_some(last);
        }
        let ty = &segs[segs.len() - 2];
        if segs.len() >= 3 && ty == "Stmt" {
            let pre = &segs[segs.len() - 3];
            if is_ast_seg(pre) {
                return Some(last);
            }
            if is_ir_seg(pre) {
                return None;
            }
        }
        if ty == "Stmt" && segs.len() == 2 {
            return match self.lookup_name("Stmt") {
                Some(full) if full_is_ast_stmt(full) => Some(last),
                _ => None,
            };
        }
        if segs.len() == 2 {
            if let Some(full) = self.lookup_name(ty) {
                if full_is_ast_stmt(full) {
                    return Some(last);
                }
            }
            return None;
        }
        if ty == "Stmt" && segs[..segs.len() - 2].iter().any(|s| is_ast_seg(s)) {
            return Some(last);
        }
        None
    }

    fn scan_pat(&mut self, p: &Pat) -> bool {
        match p {
            Pat::Struct(ps) => {
                let mut hit = false;
                if let Some(v) = self.stmt_variant(&ps.path) {
                    hit = true;
                    if let Some(c) = self.cur() {
                        c.variants.insert(v);
                    }
                }
                for fp in &ps.fields {
                    hit |= self.scan_pat(&fp.pat);
                }
                hit
            }
            Pat::TupleStruct(pt) => {
                let mut hit = false;
                if let Some(v) = self.stmt_variant(&pt.path) {
                    hit = true;
                    if let Some(c) = self.cur() {
                        c.variants.insert(v);
                    }
                }
                for e in &pt.elems {
                    hit |= self.scan_pat(e);
                }
                hit
            }
            Pat::Path(pp) => match self.stmt_variant(&pp.path) {
                Some(v) => {
                    if let Some(c) = self.cur() {
                        c.variants.insert(v);
                    }
                    true
                }
                None => false,
            },
            Pat::Or(po) => po.cases.iter().fold(false, |h, c| self.scan_pat(c) | h),
            Pat::Reference(r) => self.scan_pat(&r.pat),
            Pat::Paren(r) => self.scan_pat(&r.pat),
            Pat::Type(t) => self.scan_pat(&t.pat),
            Pat::Ident(i) => i.subpat.as_ref().is_some_and(|(_, s)| self.scan_pat(s)),
            Pat::Tuple(t) => t.elems.iter().fold(false, |h, e| self.scan_pat(e) | h),
            Pat::Slice(t) => t.elems.iter().fold(false, |h, e| self.scan_pat(e) | h),
            _ => false,
        }
    }

    fn push_fn(&mut self, attrs: &[syn::Attribute], sig: &syn::Signature) -> bool {
        if has_cfg_test(attrs) {
            return false;
        }
        let simple = sig.ident.to_string();
        let qual = match self.stack.last() {
            Some(&o) => format!("{}::{}", self.out[o].qual, simple),
            None => simple.clone(),
        };
        let returns_str_coll = match &sig.output {
            syn::ReturnType::Type(_, t) => {
                let ids = type_idents(t);
                ids.iter().any(|i| COLL.contains(&i.as_str()))
                    && ids.iter().any(|i| i == "String" || i == "str")
            }
            syn::ReturnType::Default => false,
        };
        self.out.push(FnRec {
            file: self.file.clone(),
            qual,
            simple,
            returns_str_coll,
            ..Default::default()
        });
        self.stack.push(self.out.len() - 1);
        true
    }

    fn ask(&mut self, kind: &str, entry: String, arg: String) {
        if let Some(c) = self.cur() {
            c.asks.push((kind.to_string(), entry, arg));
        }
    }

    fn prefix_write(&mut self, form: &str) {
        let inner = self.blocks.last().copied().unwrap_or(0);
        let all = self.blocks.clone();
        if let Some(c) = self.cur() {
            c.prefix_writes.push((form.to_string(), inner, all));
        }
    }

    fn walk_call(&mut self, name: &str, args: &[&Expr]) {
        let Some(&(_, from_end)) = WALK_ENTRIES.iter().find(|(n, _)| *n == name) else {
            return;
        };
        let probes = if args.len() >= from_end {
            probes_of(args[args.len() - from_end])
        } else {
            Vec::new()
        };
        if let Some(c) = self.cur() {
            c.walk_calls.push((name.to_string(), probes));
        }
    }
}

/// `true` when `e` is `<recv>.cur_prefix`.
fn is_prefix_field(e: &Expr) -> bool {
    match e {
        Expr::Field(f) => matches!(&f.member, syn::Member::Named(i) if i == "cur_prefix"),
        Expr::Paren(p) => is_prefix_field(&p.expr),
        _ => false,
    }
}

/// `&mut <recv>.cur_prefix`.
fn is_mut_prefix_ref(e: &Expr) -> bool {
    match e {
        Expr::Reference(r) => r.mutability.is_some() && is_prefix_field(&r.expr),
        Expr::Paren(p) => is_mut_prefix_ref(&p.expr),
        _ => false,
    }
}

fn type_idents(t: &syn::Type) -> Vec<String> {
    let mut out = Vec::new();
    type_idents_into(t, &mut out);
    out
}

fn type_idents_into(t: &syn::Type, out: &mut Vec<String>) {
    match t {
        syn::Type::Path(p) => {
            if let Some(q) = &p.qself {
                type_idents_into(&q.ty, out);
            }
            path_idents(&p.path, out);
        }
        syn::Type::Reference(r) => type_idents_into(&r.elem, out),
        syn::Type::Slice(r) => type_idents_into(&r.elem, out),
        syn::Type::Array(r) => type_idents_into(&r.elem, out),
        syn::Type::Paren(r) => type_idents_into(&r.elem, out),
        syn::Type::Group(r) => type_idents_into(&r.elem, out),
        syn::Type::Tuple(tt) => {
            for e in &tt.elems {
                type_idents_into(e, out);
            }
        }
        syn::Type::ImplTrait(it) => {
            for b in &it.bounds {
                if let syn::TypeParamBound::Trait(tb) = b {
                    path_idents(&tb.path, out);
                }
            }
        }
        _ => {}
    }
}

fn path_idents(p: &syn::Path, out: &mut Vec<String>) {
    for seg in &p.segments {
        out.push(seg.ident.to_string());
        if let syn::PathArguments::AngleBracketed(a) = &seg.arguments {
            for ga in &a.args {
                if let syn::GenericArgument::Type(t) = ga {
                    type_idents_into(t, out);
                }
            }
        }
    }
}

/// H7: the type idents of the collection an initializer constructs: `X::new()` /
/// `X::<T>::new()` / `X::with_capacity()` / `X::default()` / `X::from(..)` (X a
/// collection), `.collect::<T>()`, through `.clone()`.
fn init_coll_idents(e: &Expr) -> Option<Vec<String>> {
    match e {
        Expr::Call(c) => {
            let Expr::Path(p) = &*c.func else { return None };
            let segs = &p.path.segments;
            if segs.len() >= 2 && COLL.contains(&segs[segs.len() - 2].ident.to_string().as_str()) {
                let mut ids = Vec::new();
                path_idents(&p.path, &mut ids);
                return Some(ids);
            }
            None
        }
        Expr::MethodCall(m) if m.method == "collect" => {
            let mut ids = vec!["collect".to_string()];
            if let Some(tf) = &m.turbofish {
                for a in &tf.args {
                    if let syn::GenericArgument::Type(t) = a {
                        type_idents_into(t, &mut ids);
                    }
                }
            }
            Some(ids)
        }
        Expr::MethodCall(m) if m.method == "clone" => init_coll_idents(&m.receiver),
        Expr::Paren(p) => init_coll_idents(&p.expr),
        _ => None,
    }
}

/// H7: the callee a `let` initializer calls at its head (through `?`, `&`, parens and
/// `.clone()` / `.unwrap()` / `.unwrap_or_default()` / `.into_iter().collect()`-free
/// wrappers): a path call's last segment or a method's name.
fn init_callee(e: &Expr) -> Option<String> {
    match e {
        Expr::Call(c) => match &*c.func {
            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        },
        Expr::MethodCall(m)
            if matches!(
                m.method.to_string().as_str(),
                "clone" | "unwrap" | "unwrap_or_default" | "to_owned"
            ) =>
        {
            init_callee(&m.receiver)
        }
        Expr::MethodCall(m) => Some(m.method.to_string()),
        Expr::Try(t) => init_callee(&t.expr),
        Expr::Reference(r) => init_callee(&r.expr),
        Expr::Paren(p) => init_callee(&p.expr),
        _ => None,
    }
}

fn pat_names(p: &Pat, out: &mut Vec<String>) {
    match p {
        Pat::Ident(i) => out.push(i.ident.to_string()),
        Pat::Type(t) => pat_names(&t.pat, out),
        Pat::Reference(r) => pat_names(&r.pat, out),
        Pat::Paren(r) => pat_names(&r.pat, out),
        Pat::Tuple(t) => t.elems.iter().for_each(|e| pat_names(e, out)),
        Pat::TupleStruct(t) => t.elems.iter().for_each(|e| pat_names(e, out)),
        Pat::Struct(s) => s.fields.iter().for_each(|f| pat_names(&f.pat, out)),
        Pat::Slice(s) => s.elems.iter().for_each(|e| pat_names(e, out)),
        _ => {}
    }
}

/// H6c (census_g2b): the reduced form of a first argument.
fn red(e: &Expr) -> String {
    match e {
        Expr::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        Expr::Field(f) => {
            let m = match &f.member {
                syn::Member::Named(i) => i.to_string(),
                syn::Member::Unnamed(i) => i.index.to_string(),
            };
            let b = red(&f.base);
            if b == "self" || b.is_empty() {
                m
            } else {
                format!("{b}.{m}")
            }
        }
        Expr::Index(i) => format!("{}[..]", red(&i.expr)),
        Expr::Reference(r) => red(&r.expr),
        Expr::Paren(p) => red(&p.expr),
        Expr::Unary(u) => red(&u.expr),
        Expr::MethodCall(m)
            if matches!(
                m.method.to_string().as_str(),
                "as_str" | "clone" | "to_string" | "as_ref" | "as_deref" | "to_owned" | "unwrap"
            ) =>
        {
            red(&m.receiver)
        }
        Expr::MethodCall(m) => format!("{}.{}()", red(&m.receiver), m.method),
        Expr::Lit(_) => "lit".into(),
        Expr::Call(_) => "call()".into(),
        Expr::Macro(m) => format!(
            "{}!",
            m.mac
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default()
        ),
        _ => "?".into(),
    }
}

fn comp_field(e: &Expr) -> Option<String> {
    if let Expr::Field(f) = e {
        if let syn::Member::Named(i) = &f.member {
            let n = i.to_string();
            if COMP_FIELDS.contains(&n.as_str()) {
                return Some(n);
            }
        }
    }
    None
}

fn is_recv(e: &Expr) -> bool {
    matches!(e, Expr::Path(p) if p.path.segments.len() == 1 && RECV.contains(&p.path.segments[0].ident.to_string().as_str()))
}

fn single_path(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) if p.path.segments.len() == 1 && p.qself.is_none() => {
            Some(p.path.segments[0].ident.to_string())
        }
        _ => None,
    }
}

/// H6: what a hit argument probes, in source order (duplicates dropped): a field of the
/// `Elaborator` receiver, a `hit_*` method of it, a local fn / closure called `name()`, a
/// local table probed (`table.contains_key(k)`), or a bare name passed as the hit.
fn probes_of(e: &Expr) -> Vec<String> {
    struct P(Vec<String>);
    impl P {
        fn add(&mut self, s: String) {
            if !self.0.contains(&s) {
                self.0.push(s);
            }
        }
    }
    impl<'ast> Visit<'ast> for P {
        fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
            if is_recv(&f.base) {
                if let syn::Member::Named(i) = &f.member {
                    self.add(i.to_string());
                    return;
                }
            }
            visit::visit_expr_field(self, f);
        }
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            let meth = m.method.to_string();
            if is_recv(&m.receiver) && meth.starts_with("hit_") {
                self.add(meth);
            } else if let Some(local) = single_path(&m.receiver) {
                if !RECV.contains(&local.as_str()) && PROBE_METHODS.contains(&meth.as_str()) {
                    self.add(local);
                }
            }
            visit::visit_expr_method_call(self, m);
        }
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if let Some(n) = single_path(&c.func) {
                self.add(format!("{n}()"));
            }
            visit::visit_expr_call(self, c);
        }
    }
    let mut p = P(Vec::new());
    match e {
        Expr::Closure(c) => p.visit_expr(&c.body),
        Expr::Reference(r) => match &*r.expr {
            Expr::Field(_) => p.visit_expr(&r.expr),
            other => {
                if let Some(n) = single_path(other) {
                    p.add(n);
                } else {
                    p.visit_expr(other);
                }
            }
        },
        other => {
            if let Some(n) = single_path(other) {
                p.add(n);
            } else {
                p.visit_expr(other);
            }
        }
    }
    p.0
}

impl<'ast> Visit<'ast> for Pass<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if self.push_fn(&f.attrs, &f.sig) {
            visit::visit_item_fn(self, f);
            self.stack.pop();
        }
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        if self.push_fn(&f.attrs, &f.sig) {
            visit::visit_impl_item_fn(self, f);
            self.stack.pop();
        }
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if self.push_fn(&f.attrs, &f.sig) {
            visit::visit_trait_item_fn(self, f);
            self.stack.pop();
        }
    }
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if !has_cfg_test(&i.attrs) {
            visit::visit_item_impl(self, i);
        }
    }
    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        if has_cfg_test(&m.attrs) {
            return;
        }
        if let Some((_, items)) = &m.content {
            self.uses.push(item_uses(items));
            visit::visit_item_mod(self, m);
            self.uses.pop();
        }
    }
    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        if has_cfg_test(&s.attrs) {
            return;
        }
        let owner = self
            .stack
            .last()
            .map(|&i| self.out[i].qual.clone())
            .unwrap_or_else(|| "-".into());
        if let syn::Fields::Named(nf) = &s.fields {
            for f in &nf.named {
                if let Some(id) = &f.ident {
                    self.struct_fields
                        .push((owner.clone(), s.ident.to_string(), id.to_string()));
                }
            }
        }
        visit::visit_item_struct(self, s);
    }
    fn visit_block(&mut self, b: &'ast syn::Block) {
        self.next_block += 1;
        self.blocks.push(self.next_block);
        self.uses.push(block_uses(&b.stmts));
        visit::visit_block(self, b);
        self.uses.pop();
        self.blocks.pop();
    }
    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        let mut stmt_match = false;
        for arm in &m.arms {
            stmt_match |= self.scan_pat(&arm.pat);
        }
        if stmt_match {
            let (mut w, mut wg, mut b) = (0, 0, 0);
            for arm in &m.arms {
                match catch_all(&arm.pat) {
                    Some(true) if arm.guard.is_none() => w += 1,
                    Some(false) if arm.guard.is_none() => b += 1,
                    Some(_) => wg += 1,
                    None => {}
                }
            }
            if let Some(c) = self.cur() {
                c.wildcard += w;
                c.wildcard_guarded += wg;
                c.binding_arm += b;
            }
        }
        visit::visit_expr_match(self, m);
    }
    fn visit_expr_let(&mut self, l: &'ast syn::ExprLet) {
        self.scan_pat(&l.pat);
        visit::visit_expr_let(self, l);
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        self.scan_pat(&l.pat);
        let mut names = Vec::new();
        pat_names(&l.pat, &mut names);
        let first = names.first().cloned().unwrap_or_else(|| "<pat>".into());
        // H6: a local closure, for expanding a hit that calls it
        if let Some(init) = &l.init {
            if matches!(&*init.expr, Expr::Closure(_)) {
                let probes = probes_of(&init.expr);
                if let Some(c) = self.cur() {
                    c.local_closures.insert(first.clone(), probes);
                }
            }
        }
        // H7
        let declared = if let Pat::Type(pt) = &l.pat {
            Some(type_idents(&pt.ty))
        } else {
            None
        };
        let ids = match &declared {
            Some(ids) if ids.iter().any(|i| COLL.contains(&i.as_str())) => Some(ids.clone()),
            Some(_) => None,
            None => l.init.as_ref().and_then(|i| init_coll_idents(&i.expr)),
        };
        let str_coll = ids
            .as_ref()
            .is_some_and(|ids| ids.iter().any(|i| i == "String" || i == "str"));
        let untyped_callee = if declared.is_none() && ids.is_none() {
            l.init.as_ref().and_then(|i| init_callee(&i.expr))
        } else {
            None
        };
        if let Some(c) = self.cur() {
            c.let_names.extend(names.iter().cloned());
            if str_coll {
                c.str_locals
                    .push((first.clone(), ids.unwrap_or_default().join(" ")));
            } else if let Some(callee) = untyped_callee {
                c.untyped_calls.push((first.clone(), callee));
            }
        }
        visit::visit_local(self, l);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("matches") {
            let parser = |input: ParseStream| -> syn::Result<(Expr, Pat, Option<Expr>)> {
                let e: Expr = input.parse()?;
                input.parse::<Token![,]>()?;
                let p = Pat::parse_multi_with_leading_vert(input)?;
                let g = if input.peek(Token![if]) {
                    input.parse::<Token![if]>()?;
                    Some(input.parse::<Expr>()?)
                } else {
                    None
                };
                let _ = input.parse::<Option<Token![,]>>()?;
                Ok((e, p, g))
            };
            if let Ok((e, p, g)) = parser.parse2(mac.tokens.clone()) {
                self.scan_pat(&p);
                self.visit_expr(&e);
                if let Some(g) = g {
                    self.visit_expr(&g);
                }
                return;
            }
        }
        match Punctuated::<Expr, Token![,]>::parse_terminated.parse2(mac.tokens.clone()) {
            Ok(list) => {
                for e in &list {
                    self.visit_expr(e);
                }
            }
            Err(_) => {
                if mac.tokens.to_string().contains("Stmt") {
                    if let Some(c) = self.cur() {
                        c.macro_unparsed_stmt += 1;
                    }
                }
            }
        }
    }
    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        if is_prefix_field(&a.left) {
            self.prefix_write("assign");
        }
        visit::visit_expr_assign(self, a);
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        let compound = matches!(
            b.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        );
        if compound && is_prefix_field(&b.left) {
            self.prefix_write("opassign");
        }
        visit::visit_expr_binary(self, b);
    }
    fn visit_expr_reference(&mut self, r: &'ast syn::ExprReference) {
        if r.mutability.is_some() && is_prefix_field(&r.expr) {
            self.prefix_write("mutref");
        }
        visit::visit_expr_reference(self, r);
    }
    fn visit_expr_struct(&mut self, s: &'ast syn::ExprStruct) {
        for f in &s.fields {
            if matches!(&f.member, syn::Member::Named(i) if i == "cur_prefix") {
                self.prefix_write("init");
            }
        }
        visit::visit_expr_struct(self, s);
    }
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        let mut opener = None;
        if let Expr::Path(p) = &*c.func {
            if let Some(last) = p.path.segments.last() {
                let n = last.ident.to_string();
                if let Some(r) = self.cur() {
                    if n == r.simple {
                        r.self_call = true;
                    }
                    r.calls.insert(n.clone());
                }
                if n == "for_each_nested_stmt" {
                    if let Some(r) = self.cur() {
                        r.fens = true;
                    }
                }
                let args: Vec<&Expr> = c.args.iter().collect();
                self.walk_call(&n, &args);
                if matches!(n.as_str(), "replace" | "take" | "swap") && p.path.segments.len() >= 2 {
                    let mem = p.path.segments[p.path.segments.len() - 2].ident == "mem";
                    if mem && c.args.iter().any(is_mut_prefix_ref) {
                        opener = Some(n);
                    }
                }
            }
        }
        if let Some(form) = opener {
            // the `&mut <recv>.cur_prefix` argument is this opener, not a second write
            self.prefix_write(&form);
            self.visit_expr(&c.func);
            for a in &c.args {
                if is_mut_prefix_ref(a) {
                    if let Expr::Reference(r) = a {
                        self.visit_expr(&r.expr);
                    }
                } else {
                    self.visit_expr(a);
                }
            }
            return;
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        let meth = m.method.to_string();
        if let Some(r) = self.cur() {
            if meth == r.simple {
                r.self_call = true;
            }
            r.calls.insert(meth.clone());
            if meth == "for_each_nested_stmt" {
                r.fens = true;
            }
        }
        // H3
        if is_prefix_field(&m.receiver) && STRING_MUT.contains(&meth.as_str()) {
            self.prefix_write(&format!("method:{meth}"));
        }
        // H6
        let args: Vec<&Expr> = m.args.iter().collect();
        self.walk_call(&meth, &args);
        // H6c (census_g2b's order: the receiver, the consult, then the arguments)
        let arg0 = m.args.first().map(red).unwrap_or_default();
        if let Some(fname) = comp_field(&m.receiver) {
            if let Expr::Field(f) = &*m.receiver {
                self.visit_expr(&f.base);
            }
            let kind = if COMP_MUT.contains(&meth.as_str()) {
                "Fw"
            } else {
                "F"
            };
            self.ask(kind, format!("{fname}.{meth}"), arg0);
        } else {
            self.visit_expr(&m.receiver);
            if COMP_WALK.contains(&meth.as_str()) {
                self.ask("W", meth.clone(), arg0);
            } else if COMP_STACK.contains(&meth.as_str()) {
                self.ask("S", meth.clone(), arg0);
            }
        }
        for a in &m.args {
            self.visit_expr(a);
        }
    }
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path
            .segments
            .last()
            .is_some_and(|l| l.ident == "for_each_nested_stmt")
        {
            if let Some(r) = self.cur() {
                r.fens = true;
            }
        }
        visit::visit_expr_path(self, p);
    }
    fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
        if is_recv(&f.base) {
            if let syn::Member::Named(i) = &f.member {
                let n = i.to_string();
                if let Some(r) = self.cur() {
                    if !r.self_probes.contains(&n) {
                        r.self_probes.push(n);
                    }
                }
            }
        }
        if let Some(n) = comp_field(&Expr::Field(f.clone())) {
            self.ask("F", n, String::new());
            self.visit_expr(&f.base);
            return;
        }
        visit::visit_expr_field(self, f);
    }
}

/// `Some(true)` = `_`, `Some(false)` = a plain lower-case binding, `None` = neither.
fn catch_all(p: &Pat) -> Option<bool> {
    match p {
        Pat::Wild(_) => Some(true),
        Pat::Ident(i) if i.subpat.is_none() => {
            let s = i.ident.to_string();
            if s.chars().next().is_some_and(char::is_uppercase) {
                None
            } else {
                Some(false)
            }
        }
        Pat::Reference(r) => catch_all(&r.pat),
        Pat::Paren(r) => catch_all(&r.pat),
        _ => None,
    }
}

/// The census of the whole crate, one pass.
struct Census {
    fns: Vec<FnRec>,
    struct_fields: Vec<(String, String, String, String)>,
    files: usize,
}

fn run_census() -> Census {
    let srcs = load_src();
    let mut fns = Vec::new();
    let mut struct_fields = Vec::new();
    for s in &srcs {
        let mut recs = Vec::new();
        let mut sf = Vec::new();
        {
            let mut v = Pass {
                file: s.rel.clone(),
                uses: vec![item_uses(&s.file.items)],
                stack: Vec::new(),
                out: &mut recs,
                blocks: Vec::new(),
                next_block: 0,
                struct_fields: &mut sf,
            };
            v.visit_file(&s.file);
        }
        // pre-order = the textual order of the `fn` keywords
        let blanked = blank(&s.text);
        let mut cursor = 0usize;
        for r in &mut recs {
            if let Some(p) = find_fn(&blanked, cursor, &r.simple) {
                r.line = s.text.as_bytes()[..p]
                    .iter()
                    .filter(|&&c| c == b'\n')
                    .count()
                    + 1;
                cursor = p + 2;
            }
        }
        // a second fn of one key in a file: `#2`, `#3` …
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for r in &mut recs {
            let n = seen.entry(r.qual.clone()).or_default();
            *n += 1;
            if *n > 1 {
                r.qual = format!("{}#{}", r.qual, n);
            }
        }
        fns.extend(recs);
        struct_fields.extend(sf.into_iter().map(|(o, st, f)| (s.rel.clone(), o, st, f)));
    }
    Census {
        fns,
        struct_fields,
        files: srcs.len(),
    }
}

// =====================================================================================
// Tables
// =====================================================================================

/// The rows of a pinned table: tab-separated, `#` lines and blank lines skipped, the first
/// non-comment line a header.
fn table(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .skip(1)
        .map(|l| l.split('\t').map(|c| c.trim().to_string()).collect())
        .collect()
}

fn dump(name: &str, header: &str, rows: &[String]) {
    if let Ok(dir) = std::env::var("SCOPE_HOME_DUMP") {
        if dir.is_empty() {
            return;
        }
        let mut s = String::from(header);
        s.push('\n');
        for r in rows {
            s.push_str(r);
            s.push('\n');
        }
        std::fs::create_dir_all(&dir).expect("dump dir");
        std::fs::write(Path::new(&dir).join(format!("{name}.tsv")), s).expect("dump");
    }
}

/// Compares the census (key -> value) with the table (key -> value) ; the messages name
/// each row that differs.
fn diff_rows(
    test: &str,
    got: &BTreeMap<String, String>,
    want: &BTreeMap<String, String>,
    new_row: &str,
    gone_row: &str,
) -> Vec<String> {
    let show = |s: &str| s.replace('\t', " · ");
    let mut msgs = Vec::new();
    for (k, v) in got {
        match want.get(k) {
            None => msgs.push(format!("{test} NEW ROW  {}  [{}] — {new_row}", show(k), show(v))),
            Some(w) if w != v => msgs.push(format!(
                "{test} CHANGED  {}  table [{}]  code [{}] — the row's arms / forms / hit / tier moved: \
                 re-read the site, name the unit that moved it, and set the row in the same commit",
                show(k),
                show(w),
                show(v)
            )),
            _ => {}
        }
    }
    for (k, w) in want {
        if !got.contains_key(k) {
            msgs.push(format!(
                "{test} GONE ROW {}  [{}] — {gone_row}",
                show(k),
                show(w)
            ));
        }
    }
    msgs
}

fn finish(test: &str, msgs: &[String]) {
    assert!(
        msgs.is_empty(),
        "{test}: {} difference(s) between the census and its table:\n{}",
        msgs.len(),
        msgs.join("\n")
    );
}

fn key(file: &str, f: &str) -> String {
    format!("{file}\t{f}")
}

// =====================================================================================
// H2 the statement walkers
// =====================================================================================

/// The census's walker rows: (key, class, computed columns).
fn h2_rows(c: &Census) -> Vec<(String, String, String, usize)> {
    let walker_names: BTreeSet<&str> = c
        .fns
        .iter()
        .filter(|r| r.fens || r.variants.contains("Block") || r.variants.contains("Fork"))
        .map(|r| r.simple.as_str())
        .collect();
    let mut rows = Vec::new();
    for r in &c.fns {
        let has_pat = !r.variants.is_empty();
        let bf = r.variants.contains("Block") || r.variants.contains("Fork");
        let containers: Vec<&str> = CARRIERS
            .iter()
            .copied()
            .filter(|v| r.variants.contains(*v))
            .collect();
        let via: Vec<&str> = r
            .calls
            .iter()
            .map(String::as_str)
            .filter(|n| *n != r.simple && walker_names.contains(n))
            .collect();
        let class = if r.simple == "for_each_nested_stmt" && r.variants.len() == VARIANTS.len() {
            "THE-DESCENT".to_string()
        } else if r.fens {
            "WALKER-FENS".to_string()
        } else if bf {
            "WALKER-OWN".to_string()
        } else if has_pat
            && containers
                .iter()
                .any(|v| *v != "DeferredAssert" && *v != "ConcurrentAssert")
            && (r.self_call || !via.is_empty())
        {
            format!("WALKER-VIA-HELPER({})", via.join(","))
        } else {
            continue;
        };
        let arms = if r.variants.len() == VARIANTS.len() {
            "ALL".to_string()
        } else {
            r.variants.iter().cloned().collect::<Vec<_>>().join(",")
        };
        let wild = if r.wildcard + r.binding_arm + r.wildcard_guarded == 0 {
            "n".to_string()
        } else {
            format!(
                "_{} bind{} guarded{}",
                r.wildcard, r.binding_arm, r.wildcard_guarded
            )
        };
        let to_wild = if r.wildcard + r.binding_arm > 0 {
            let fall: Vec<&str> = CARRIERS
                .iter()
                .copied()
                .filter(|v| !r.variants.contains(*v))
                .collect();
            if fall.is_empty() {
                "none".to_string()
            } else {
                fall.join(",")
            }
        } else {
            "-".to_string()
        };
        let fens = if r.fens { "y" } else { "n" };
        rows.push((
            key(&r.file, &r.qual),
            class.clone(),
            format!("{class}\t{arms}\t{wild}\t{fens}\t{to_wild}"),
            r.line,
        ));
    }
    rows
}

#[test]
fn h2_the_statement_walkers_are_pinned() {
    let c = run_census();
    // the pass sees what census_walkers.rs saw: a pattern under `matches!`, a glob import
    let probe = syn::parse_file(
        "use hdl_ast as ast; fn w(s: &ast::Stmt) { use ast::Stmt::*; match s { Block { .. } => {} _ => {} } \
         if matches!(s, ast::Stmt::Fork { .. } | ast::Stmt::Wait { .. }) {} }",
    )
    .expect("probe parses");
    let mut recs = Vec::new();
    let mut sf = Vec::new();
    Pass {
        file: "probe".into(),
        uses: vec![item_uses(&probe.items)],
        stack: Vec::new(),
        out: &mut recs,
        blocks: Vec::new(),
        next_block: 0,
        struct_fields: &mut sf,
    }
    .visit_file(&probe);
    assert_eq!(recs.len(), 1);
    assert_eq!(
        recs[0].variants.iter().cloned().collect::<Vec<_>>(),
        ["Block", "Fork", "Wait"],
        "H2's pass no longer resolves a glob or a `matches!` pattern"
    );
    assert_eq!(recs[0].wildcard, 1);

    // a macro that names `Stmt` and does not parse as an expression list hides its
    // patterns from the census (G1 blind spot 1: 0 such macros)
    let unread: Vec<String> = c
        .fns
        .iter()
        .filter(|r| r.macro_unparsed_stmt > 0)
        .map(|r| key(&r.file, &r.qual).replace('\t', " · "))
        .collect();
    assert!(
        unread.is_empty(),
        "H2: a macro naming `Stmt` that the census cannot parse, in {unread:?} — read it by hand: a walker \
         there is a row this table lacks (enter it HAND with its reason)"
    );
    let rows = h2_rows(&c);
    let mut got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, _, v, _)| (k.clone(), v.clone()))
        .collect();
    let dumped: Vec<String> = rows
        .iter()
        .map(|(k, _, v, l)| format!("{k}\t{l}\t{v}"))
        .collect();
    dump(
        "H2",
        "file\tfn\tline\tclass\tarms\tcatch_all\tcalls_FENS\tcarriers_to_catch_all",
        &dumped,
    );

    let mut want: BTreeMap<String, String> = BTreeMap::new();
    let mut hand: Vec<(String, String, String, String)> = Vec::new();
    let mut msgs = Vec::new();
    for row in table(tables::H2) {
        assert_eq!(
            row.len(),
            10,
            "H2 table row has {} columns, not 10: {row:?}",
            row.len()
        );
        let k = key(&row[0], &row[1]);
        if row[2] == "HAND" {
            // a row the syntax cannot see as a walker: it must exist and call what the row says
            let calls: Vec<&str> = row[3].split(',').collect();
            match c.fns.iter().find(|r| key(&r.file, &r.qual) == k) {
                None => msgs.push(format!(
                    "H2 HAND ROW GONE {k} — the fn is not in the source: remove the row"
                )),
                Some(r) => {
                    for n in &calls {
                        if !r.calls.contains(*n) {
                            msgs.push(format!(
                                "H2 HAND ROW {k} no longer calls `{n}` — re-read it: still a walker (keep, fix the \
                                 row) or not (remove the row)"
                            ));
                        }
                    }
                }
            }
            if got.contains_key(&k) {
                msgs.push(format!(
                    "H2 HAND ROW {k} is now a walker by syntax — drop HAND and pin the computed row (arms, catch-all)"
                ));
            }
            got.insert(k.clone(), row[2..7].join("\t"));
        }
        want.insert(k.clone(), row[2..7].join("\t"));
        hand.push((k, row[7].clone(), row[8].clone(), row[9].clone()));
    }
    msgs.extend(diff_rows(
        "H2",
        &got,
        &want,
        "a new statement walker: name the unit that adds it (ROADMAP §5.2), read its catch-all (KIND), whether it \
         reads a declared name (MODEL) and how it asks (ROLE), and add the row",
        "the walker is gone or no longer a walker: name the unit, remove the row",
    ));
    // the hand columns' own pins: a KIND only on a walker with a catch-all, counts as read
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut models: BTreeMap<&str, usize> = BTreeMap::new();
    let mut roles: BTreeMap<&str, usize> = BTreeMap::new();
    for (k, kind, model, role) in &hand {
        if kind != "-" {
            *kinds.entry(kind).or_default() += 1;
            let w = want.get(k).map(String::as_str).unwrap_or("");
            if !w.starts_with("WALKER-OWN\t") || w.split('\t').nth(2) == Some("n") {
                msgs.push(format!(
                    "H2 KIND {kind} on {k}, which is not a WALKER-OWN row with a catch-all arm"
                ));
            }
        }
        if model != "-" {
            *models.entry(model).or_default() += 1;
        }
        if role != "-" {
            *roles.entry(role).or_default() += 1;
            if model == "-" {
                msgs.push(format!("H2 ROLE {role} on {k}, which has no reader MODEL"));
            }
        }
    }
    let classes: BTreeMap<String, usize> = want.values().fold(BTreeMap::new(), |mut m, v| {
        *m.entry(v.split(['\t', '(']).next().unwrap_or("").to_string())
            .or_default() += 1;
        m
    });
    let counts = format!("{classes:?} kinds {kinds:?} models {models:?} roles {roles:?}");
    if counts != tables::H2_COUNTS {
        msgs.push(format!(
            "H2 COUNTS table [{}] now [{counts}] — set H2_COUNTS with the row change that moved it",
            tables::H2_COUNTS
        ));
    }
    finish("H2", &msgs);
}

// =====================================================================================
// H3 the writes of `cur_prefix`
// =====================================================================================

fn h3_rows(c: &Census) -> Vec<(String, String, usize)> {
    let mut rows = Vec::new();
    for r in &c.fns {
        if r.prefix_writes.is_empty() {
            continue;
        }
        let mut forms: BTreeMap<&str, usize> = BTreeMap::new();
        for (f, _, _) in &r.prefix_writes {
            *forms.entry(f.as_str()).or_default() += 1;
        }
        // sites: an opener, a struct literal, and one per fn for the rest no opener covers
        let mut openers: Vec<usize> = Vec::new();
        let mut sites = 0;
        let mut uncovered = false;
        for (f, inner, all) in &r.prefix_writes {
            match f.as_str() {
                "replace" | "take" | "swap" => {
                    openers.push(*inner);
                    sites += 1;
                }
                "init" => sites += 1,
                _ => {
                    if !openers.iter().any(|o| all.contains(o)) {
                        uncovered = true;
                    }
                }
            }
        }
        if uncovered {
            sites += 1;
        }
        let forms = forms
            .iter()
            .map(|(f, n)| format!("{f}={n}"))
            .collect::<Vec<_>>()
            .join(",");
        rows.push((key(&r.file, &r.qual), format!("{forms}\t{sites}"), r.line));
    }
    rows
}

#[test]
fn h3_the_writes_of_cur_prefix_are_pinned() {
    let c = run_census();
    let rows = h3_rows(&c);
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, v, _)| (k.clone(), v.clone()))
        .collect();
    let dumped: Vec<String> = rows
        .iter()
        .map(|(k, v, l)| format!("{k}\t{l}\t{v}"))
        .collect();
    dump("H3", "file\tfn\tline\tforms\tsites", &dumped);
    let mut want = BTreeMap::new();
    let mut msgs = Vec::new();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for row in table(tables::H3) {
        assert_eq!(
            row.len(),
            5,
            "H3 table row has {} columns, not 5: {row:?}",
            row.len()
        );
        let k = key(&row[0], &row[1]);
        let sites: usize = row[3].parse().expect("sites");
        let kinds: Vec<&str> = row[4].split('+').collect();
        if kinds.len() != sites {
            msgs.push(format!(
                "H3 {k}: {sites} site(s) but {} kind(s) [{}]",
                kinds.len(),
                row[4]
            ));
        }
        for kd in kinds {
            *by_kind.entry(kd.to_string()).or_default() += 1;
        }
        total += sites;
        want.insert(k, format!("{}\t{}", row[2], row[3]));
    }
    msgs.extend(diff_rows(
        "H3",
        &got,
        &want,
        "a new write of `cur_prefix`: name the unit that adds it (LS2's funnel takes every write), read what it sets \
         the prefix to and whether every path restores it (u0 T2.txt PR7's method), give it its kind, add the row",
        "a write of `cur_prefix` is gone: name the unit, remove the row",
    ));
    let counts = format!("sites {total} {by_kind:?}");
    if counts != tables::H3_COUNTS {
        msgs.push(format!(
            "H3 COUNTS table [{}] now [{counts}]",
            tables::H3_COUNTS
        ));
    }
    finish("H3", &msgs);
}

// =====================================================================================
// H6 the scope-walk call sites
// =====================================================================================

fn h6_rows(c: &Census) -> Vec<(String, String, String, usize)> {
    let hit_fns: BTreeMap<&str, &Vec<String>> = c
        .fns
        .iter()
        .filter(|r| r.simple.starts_with("hit_"))
        .map(|r| (r.simple.as_str(), &r.self_probes))
        .collect();
    let mut agg: BTreeMap<String, (String, String, usize, usize)> = BTreeMap::new();
    for r in &c.fns {
        for (entry, probes) in &r.walk_calls {
            let text = if probes.is_empty() {
                "?".to_string()
            } else {
                probes.join("|")
            };
            // the set: each `hit_*` and local closure expanded to what it probes
            let mut set: BTreeSet<String> = BTreeSet::new();
            let mut todo: Vec<String> = probes.clone();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            while let Some(p) = todo.pop() {
                if !seen.insert(p.clone()) {
                    continue;
                }
                if let Some(fs) = hit_fns.get(p.as_str()) {
                    todo.extend(fs.iter().cloned());
                } else if let Some(local) =
                    p.strip_suffix("()").and_then(|n| r.local_closures.get(n))
                {
                    todo.extend(local.iter().cloned());
                } else {
                    set.insert(p);
                }
            }
            let set = set.into_iter().collect::<Vec<_>>().join("+");
            let class = if WALK_FAMILY.contains(&r.simple.as_str()) && entry != "decl_probe_key" {
                "INTERNAL"
            } else if WALK_SITE_ENTRIES.contains(&entry.as_str()) {
                "SITE"
            } else {
                match entry.as_str() {
                    "walk_rtn_scopes" => "RTN",
                    "hier_resolve" => "HIER",
                    "decl_probe_key" => "PROBE",
                    _ => "INTERNAL",
                }
            };
            let k = format!("{}\t{}\t{entry}\t{text}", r.file, r.qual);
            let e = agg.entry(k).or_insert((class.to_string(), set, 0, r.line));
            e.2 += 1;
        }
    }
    agg.into_iter()
        .map(|(k, (class, set, n, line))| {
            let v = format!("{class}\t{set}\t{n}");
            (k, class, v, line)
        })
        .collect()
}

#[test]
fn h6_the_walk_call_sites_are_pinned() {
    // the hit reader: a field, a `hit_*`, a local closure, a local table, a passed name
    let e: Expr = syn::parse_str(
        "|k| { self.symbols.contains_key(k)\n || me.hit_wide(k) || other(k) || table.contains_key(k) || refused(k).is_some() }",
    )
    .expect("probe expr");
    assert_eq!(
        probes_of(&e),
        ["symbols", "hit_wide", "other()", "table", "refused()"]
    );
    assert_eq!(
        probes_of(&syn::parse_str::<Expr>("&self.params").expect("ref")),
        ["params"]
    );
    assert_eq!(
        probes_of(&syn::parse_str::<Expr>("&hit").expect("ref")),
        ["hit"]
    );

    let c = run_census();
    let rows = h6_rows(&c);
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, _, v, _)| (k.clone(), v.clone()))
        .collect();
    let dumped: Vec<String> = rows
        .iter()
        .map(|(k, _, v, l)| format!("{k}\t{l}\t{v}"))
        .collect();
    dump(
        "H6",
        "file\tfn\tentry\thit_text\tline\tclass\thit_set\tcalls",
        &dumped,
    );
    let mut want = BTreeMap::new();
    for row in table(tables::H6) {
        assert_eq!(
            row.len(),
            7,
            "H6 table row has {} columns, not 7: {row:?}",
            row.len()
        );
        want.insert(row[..4].join("\t"), row[4..].join("\t"));
    }
    let mut msgs = diff_rows(
        "H6",
        &got,
        &want,
        "a new scope-walk call (or a new hit): name the unit that adds it, read the hit's maps (a new map set is a new \
         `Hit` value for Q-DECLKEY DK2), add the row",
        "a scope-walk call is gone: name the unit, remove the row",
    );
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    let mut texts: BTreeSet<String> = BTreeSet::new();
    let mut sets: BTreeSet<String> = BTreeSet::new();
    for (k, v) in &want {
        let cols: Vec<&str> = v.split('\t').collect();
        let n: usize = cols[2].parse().expect("count");
        *by_class.entry(cols[0].to_string()).or_default() += n;
        if cols[0] == "SITE" {
            texts.insert(k.split('\t').nth(3).unwrap_or("").to_string());
            sets.insert(cols[1].to_string());
        }
    }
    let counts = format!(
        "{by_class:?} site_texts {} site_sets {}",
        texts.len(),
        sets.len()
    );
    if counts != tables::H6_COUNTS {
        msgs.push(format!(
            "H6 COUNTS table [{}] now [{counts}]",
            tables::H6_COUNTS
        ));
    }
    finish("H6", &msgs);
}

// =====================================================================================
// H6c the composites
// =====================================================================================

fn h6c_rows(c: &Census) -> Vec<(String, String, usize)> {
    let mut rows = Vec::new();
    for r in &c.fns {
        let fs: BTreeSet<String> = r
            .asks
            .iter()
            .filter(|a| a.0 == "F" || a.0 == "S")
            .map(|a| match a.0.as_str() {
                "S" => match a.1.as_str() {
                    "subst_lookup" => "subst".to_string(),
                    "out_subst_lookup" => "out_subst".to_string(),
                    "dyn_subst_lookup" => "dyn_subst".to_string(),
                    _ => "formal_str".to_string(),
                },
                _ => a.1.split('.').next().unwrap_or("").to_string(),
            })
            .collect();
        let ws: BTreeSet<&str> = r
            .asks
            .iter()
            .filter(|a| a.0 == "W" && a.1 != "fq")
            .map(|a| a.1.as_str())
            .collect();
        let crit_a = fs.len() >= 2 || (!fs.is_empty() && !ws.is_empty());
        let mut by_arg: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for a in r.asks.iter().filter(|a| a.0 == "W") {
            by_arg.entry(a.2.as_str()).or_default().insert(a.1.as_str());
        }
        let crit_c = by_arg
            .iter()
            .any(|(arg, es)| es.len() >= 2 && !arg.is_empty() && *arg != "?" && *arg != "lit");
        if !(crit_a || crit_c) {
            continue;
        }
        // tier D (g2b tierD.py): two sources consulted with one reduced argument, writes and fq excluded
        let mut by: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        for a in &r.asks {
            if a.0 == "Fw" || a.1 == "fq" || matches!(a.2.as_str(), "" | "?" | "lit" | "call()") {
                continue;
            }
            let src = match a.0.as_str() {
                "F" => a.1.split('.').next().unwrap_or("").to_string(),
                _ => match a.1.as_str() {
                    "subst_lookup" => "subst".to_string(),
                    "out_subst_lookup" => "out_subst".to_string(),
                    "dyn_subst_lookup" => "dyn_subst".to_string(),
                    e => e.to_string(),
                },
            };
            by.entry(a.2.as_str()).or_default().insert(src);
        }
        let tier = if by.values().any(|s| s.len() >= 2) {
            "D"
        } else {
            "A/C"
        };
        rows.push((key(&r.file, &r.qual), tier.to_string(), r.line));
    }
    rows
}

#[test]
fn h6c_the_composites_are_pinned() {
    let c = run_census();
    let rows = h6c_rows(&c);
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, v, _)| (k.clone(), v.clone()))
        .collect();
    let dumped: Vec<String> = rows
        .iter()
        .map(|(k, v, l)| format!("{k}\t{l}\t{v}"))
        .collect();
    dump("H6c", "file\tfn\tline\ttier", &dumped);
    let mut want = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(tables::H6C) {
        assert_eq!(
            row.len(),
            4,
            "H6c table row has {} columns, not 4: {row:?}",
            row.len()
        );
        want.insert(key(&row[0], &row[1]), row[2].clone());
        *classes.entry(row[3].clone()).or_default() += 1;
    }
    let mut msgs = diff_rows(
        "H6c",
        &got,
        &want,
        "a new composite (a fn consulting two name answerers in its own order): name the unit that adds it, read its \
         order against R5 and give it its class (g2b composites.tsv's codes), add the row — residue R-COMP may only fall",
        "a composite is gone: name the unit, remove the row",
    );
    let counts = format!("rows {} {classes:?}", want.len());
    if counts != tables::H6C_COUNTS {
        msgs.push(format!(
            "H6c COUNTS table [{}] now [{counts}]",
            tables::H6C_COUNTS
        ));
    }
    finish("H6c", &msgs);
}

// =====================================================================================
// H7 fn-local name collections
// =====================================================================================

#[test]
fn h7_the_fn_local_name_collections_are_pinned() {
    let c = run_census();
    // (a) the typed lets
    let mut got: BTreeMap<String, String> = BTreeMap::new();
    let mut dumped = Vec::new();
    // (b) the untyped lets whose initializer calls a fn returning a String collection
    let str_fns: BTreeSet<&str> = c
        .fns
        .iter()
        .filter(|r| r.returns_str_coll)
        .map(|r| r.simple.as_str())
        .collect();
    let mut got_b: BTreeMap<String, String> = BTreeMap::new();
    for r in &c.fns {
        if !r.str_locals.is_empty() {
            let names: Vec<&str> = r.str_locals.iter().map(|(n, _)| n.as_str()).collect();
            got.insert(
                key(&r.file, &r.qual),
                format!("{}\t{}", names.len(), names.join(",")),
            );
            for (n, t) in &r.str_locals {
                dumped.push(format!("{}\t{}\t{}\tLET\t{n}\t{t}", r.file, r.qual, r.line));
            }
        }
        for (n, callee) in &r.untyped_calls {
            if str_fns.contains(callee.as_str()) {
                let e = got_b.entry(key(&r.file, &r.qual)).or_default();
                if !e.is_empty() {
                    e.push(',');
                }
                write!(e, "{n}={callee}").expect("write");
                dumped.push(format!(
                    "{}\t{}\t{}\tUNTYPED\t{n}\t{callee}",
                    r.file, r.qual, r.line
                ));
            }
        }
    }
    dump(
        "H7",
        "file\tfn\tline\tform\tbinding\ttype_or_callee",
        &dumped,
    );
    let mut msgs = Vec::new();
    let mut want = BTreeMap::new();
    let mut want_b = BTreeMap::new();
    let mut lets = 0usize;
    for row in table(tables::H7) {
        assert_eq!(
            row.len(),
            4,
            "H7 table row has {} columns, not 4: {row:?}",
            row.len()
        );
        let k = key(&row[0], &row[1]);
        match row[2].as_str() {
            "UNTYPED" => {
                want_b.insert(k, row[3].clone());
            }
            n => {
                lets += n.parse::<usize>().expect("count");
                want.insert(k, format!("{n}\t{}", row[3]));
            }
        }
    }
    let new_row = "a new fn-local name collection: name the unit that adds it (R-LOCAL may only fall), add the row";
    let gone_row = "a fn-local name collection is gone: name the unit, remove the row";
    msgs.extend(diff_rows("H7", &got, &want, new_row, gone_row));
    msgs.extend(diff_rows("H7-untyped", &got_b, &want_b, new_row, gone_row));
    // (c) the 26 readers' collections, each where the list says
    let mut n26 = 0usize;
    for row in table(tables::H7_READERS) {
        assert_eq!(
            row.len(),
            4,
            "H7 readers row has {} columns, not 4: {row:?}",
            row.len()
        );
        n26 += 1;
        let (file, owner, form, name) = (&row[0], &row[1], &row[2], &row[3]);
        let found = if let Some(st) = form.strip_prefix("field:") {
            c.struct_fields
                .iter()
                .any(|(f, o, s, fd)| f == file && o == owner && s == st && fd == name)
        } else {
            c.fns.iter().any(|r| {
                &r.file == file && &r.qual == owner && r.let_names.iter().any(|n| n == name)
            })
        };
        if !found {
            msgs.push(format!(
                "H7 READER COLLECTION GONE {file} {owner} {form} `{name}` — the reader's name collection moved or left: \
                 name the unit, re-read the reader (g2b REPORT:70-75), fix the row"
            ));
        }
    }
    let counts = format!(
        "lets {lets} fns {} untyped {} readers {n26}",
        want.len(),
        want_b.len()
    );
    if counts != tables::H7_COUNTS {
        msgs.push(format!(
            "H7 COUNTS table [{}] now [{counts}]",
            tables::H7_COUNTS
        ));
    }
    assert!(
        c.files > 100,
        "H7 read {} files: the source root is wrong",
        c.files
    );
    finish("H7", &msgs);
}

// =====================================================================================
// The pinned tables
// =====================================================================================

mod tables {
    /// H2: 62 rows by syntax + `blocks` (HAND: no `ast::Stmt` pattern of its own ; it
    /// reaches every nested block through `gather_nested_block_locals` and `actions`, the
    /// calls the row pins ; T2 counted it). KIND g2b catchalls.tsv, MODEL g2b readers.tsv,
    /// ROLE g3 askers.tsv (29) / DELTA D1 (9 LEXICAL) / BINDER (2).
    pub const H2: &str = r#"file	fn	class	arms	catch_all	calls_FENS	carriers_to_catch_all	KIND	MODEL	ROLE
ast_query.rs	collect_callee_stmt	WALKER-OWN	Block,Blocking,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,NonBlocking,Repeat,Return,SysTaskCall,UserTaskCall,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	COLLECTOR-SKIP	Flat	COLLECTOR
block_local/hoist.rs	hoist_block_local_nets	WALKER-FENS	Block,Fork	_1 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	-	LEXICAL	LEX-PRODUCER
block_local/mod.rs	collect_block_local_decls_outside_timing	WALKER-FENS	Block,Fork	n	y	-	-	Flat	COLLECTOR
block_local/mod.rs	collect_block_local_decls_spanned	WALKER-FENS	Block,Fork	_1 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	-	LEXICAL	LEX-COLLECTOR
block_local/mod.rs	for_each_nested_stmt	THE-DESCENT	ALL	n	n	-	-	-	-
block_local/mod.rs	gather_nested_block_locals	WALKER-FENS	Block,Fork	n	y	-	-	LEXICAL	LEX-COLLECTOR
block_local/mod.rs	stmts_defer_read::walk	WALKER-FENS	SysTaskCall	n	y	-	-	Flat	ONE-DECL
block_local/mod.rs	stmts_may_suspend::walk	WALKER-FENS	Blocking,DelayCtrl,EventCtrl,NonBlocking,UserTaskCall,Wait,WaitFork	_1 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,Block,Fork,DeferredAssert,ConcurrentAssert	-	-	-
block_local/proofs.rs	inout_copy_in_is_dead	WALKER-OWN	Block	_1 bind1 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	DESCEND	Flat	ONE-DECL
block_local/proofs.rs	stmt_redeclares	WALKER-OWN	ALL	n	n	-	-	Flat	ONE-DECL
block_local/timed_task.rs	timing_nested_decls::walk	WALKER-FENS	Block,Fork	n	y	-	-	-	-
block_local_class.rs	compute_per_entry_block_locals::walk	WALKER-FENS	Block,Fork	_1 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	-	Flat	COLLECTOR+MEMBERSHIP
block_local_class.rs	compute_timed_blocks::walk	WALKER-FENS	Block,Fork,WaitFork	_2 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	-	Flat	COLLECTOR+MEMBERSHIP
classes.rs	body_calls_super_new	WALKER-OWN	Block,UserTaskCall	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
const_bound.rs	const_stmt_width_safe	WALKER-OWN	Block,Blocking,For,If,Null,Repeat,Return,While	_1 bind0 guarded0	n	Case,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	-	-
const_fn.rs	exec_const_stmt	WALKER-OWN	Block,Blocking,For,If,Null,Repeat,Return,While	_1 bind0 guarded0	n	Case,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	BINDER
const_level_header.rs	body_suspend_blocker	WALKER-OWN	ALL	n	n	-	-	-	-
da/loops.rs	stmt_cannot_escape	WALKER-OWN	ALL	n	n	-	-	-	-
da/mod.rs	da_stmt	WALKER-OWN	Block,Blocking,Case,DelayCtrl,Disable,EventCtrl,For,Forever,Fork,If,NonBlocking,Repeat,Return,UserTaskCall,Wait,While	_1 bind0 guarded1	n	DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	ONE-DECL
da/mod.rs	stmt_advances_time	WALKER-OWN	Block,Blocking,Case,ConcurrentAssert,DeferredAssert,DelayCtrl,EventCtrl,For,Forever,Fork,If,NonBlocking,Repeat,UserTaskCall,Wait,WaitFork,While	_1 bind0 guarded0	n	none	LEAF-ONLY	-	-
da/reads.rs	stmt_contains_block_span	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
da/reads.rs	stmt_no_ref_deep	WALKER-OWN	Block,Blocking,Case,DelayCtrl,Disable,EventCtrl,For,Forever,Fork,If,NonBlocking,Null,Repeat,Return,SysTaskCall,UserTaskCall,Wait,WaitFork,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	ONE-DECL
da/reads.rs	stmt_reads_ident	WALKER-OWN	Assign,Block,Blocking,Case,DelayCtrl,EventCtrl,For,Force,Forever,Fork,If,NonBlocking,RandomizeWith,Repeat,Return,SysTaskCall,UserTaskCall,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	Flat	ONE-DECL
da/reads.rs	stmt_refs_ident_outside	WALKER-OWN	Block,Blocking,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,NonBlocking,RandomizeWith,Repeat,Return,SysTaskCall,UserTaskCall,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	LEXICAL	LEX-COMPARER
da/writes.rs	stmt_may_write_or_observe	WALKER-OWN	ALL	n	n	-	-	Flat	ONE-DECL
decl_collide.rs	collect_item	WALKER-OWN	Block,Fork	n	n	-	-	Flat	COLLECTOR
dynarr.rs	body_readonly_dyn_ok	WALKER-OWN	Block,Blocking,Null,Return	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	MEMBERSHIP
expr_ctx.rs	assign_needs_context_width	WALKER-OWN	Block,Blocking	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	GATED	FlatLastWins	MEMBERSHIP
ff_timing.rs	ff_walk	WALKER-OWN	ALL	n	n	-	-	-	-
frames_classify.rs	ast_has_repeat_with_timing	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
frames_classify.rs	ast_stmt_has_timing	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,WaitFork,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
frames_classify.rs	body_has_return	WALKER-OWN	Block,Case,For,Forever,Fork,If,Repeat,Return,While	_1 bind0 guarded0	n	DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
frames_classify.rs	body_needs_frame	WALKER-OWN	Block,Blocking,Null	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	-	-
frames_classify.rs	stmt_has_timing	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,WaitFork,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
frames_classify.rs	stmt_reads_only_locals	WALKER-OWN	Block,Blocking	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	MEMBERSHIP
frames_classify.rs	stmt_selects_name	WALKER-OWN	Block,Blocking	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	GATED	Flat	MEMBERSHIP
frames_classify.rs	walk_straight_line	WALKER-OWN	Block,Blocking,Null	_0 bind1 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	GATED	-	-
frames_classify_write.rs	stmt_enables_task	WALKER-OWN	ALL	n	n	-	-	-	-
frames_classify_write.rs	stmt_writes_outside_name	WALKER-OWN	ALL	n	n	-	-	Flat	MEMBERSHIP
frames_concat.rs	collect_concat_assign_spans	WALKER-OWN	Block,Blocking,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	COLLECTOR-SKIP	-	-
frames_reserve.rs	collect_case_spans	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	COLLECTOR-SKIP	-	-
frames_reserve.rs	collect_runtime_repeat_spans	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	COLLECTOR-SKIP	-	-
frames_reserve.rs	gather_auto_block_locals	WALKER-FENS	Block	n	y	-	-	LEXICAL	LEX-COLLECTOR
hier.rs	collect_hier_task_stmt	WALKER-OWN	Block,Case,DeferredAssert,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,UserTaskCall,Wait,While	_1 bind0 guarded0	n	ConcurrentAssert	COLLECTOR-SKIP	-	-
hier.rs	stmt_disables_label	WALKER-OWN	Block,Case,DelayCtrl,Disable,EventCtrl,For,Forever,Fork,If,Repeat,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	Flat	ONE-DECL
hoist/mod.rs	stmt_has_observable_effect	WALKER-OWN	Block,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,SysTaskCall,UserTaskCall,Wait,While	_1 bind0 guarded0	n	DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	-	-
inline_fold.rs	fold_straight_line	WALKER-OWN	Block,Blocking,Null,Return	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	BINDER
inline_ret_select.rs	body_writes_ret_select	WALKER-OWN	Block,Blocking	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	GATED	Flat	ONE-DECL
inline_ret_select.rs	stmt	WALKER-OWN	Block,Blocking,Null	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	LEXICAL	LEX-COMPARER
multidriver.rs	declares_local_named	WALKER-FENS	Block,ConcurrentAssert,DeferredAssert,Fork	_2 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,DelayCtrl,EventCtrl,Wait	-	Flat	ONE-DECL
multidriver.rs	stmt_writes_whole_ident	WALKER-OWN	ALL	n	n	-	-	Flat	ONE-DECL
multidriver.rs	top_level_call_writes_whole	WALKER-OWN	Block,Blocking,UserTaskCall	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	INTENDED	Flat	ONE-DECL
package.rs	pkg_stmt_pure_orig	WALKER-OWN	Block,Blocking,Return	_1 bind0 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-DECLINE	Flat	MEMBERSHIP
package.rs	pkg_stmt_pure_with	WALKER-VIA-HELPER(pkg_stmt_pure_orig)	Case,For,If,Repeat,UserTaskCall,While	_1 bind0 guarded0	n	Forever,Block,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	-	Flat	MEMBERSHIP
pkg_scoped_frames.rs	decl_scopes	WALKER-FENS	Block,Fork	n	y	-	-	LEXICAL	LEX-COLLECTOR
pkg_scoped_frames.rs	return_var_read_before_write	WALKER-OWN	Block	_0 bind1 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	DESCEND	Flat	ONE-DECL
pkg_scoped_frames.rs	static_local_read_before_write	WALKER-OWN	Block	_0 bind1 guarded0	n	If,Case,For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	DESCEND	LEXICAL	LEX-COMPARER
ports.rs	collect_vif_bindings	WALKER-OWN	Block,Blocking,Case,DelayCtrl,EventCtrl,For,Forever,Fork,If,Repeat,While	_1 bind0 guarded0	n	Wait,DeferredAssert,ConcurrentAssert	COLLECTOR-SKIP	Flat	COLLECTOR+ONE-DECL
proc_builder.rs	stmt_disables_name	WALKER-OWN	Block,Case,Disable,For,Forever,Fork,If,Repeat,While	_1 bind0 guarded0	n	DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	PREDICATE-PERMISSIVE	Flat	ONE-DECL
select_scope.rs	actions	WALKER-FENS	ConcurrentAssert,DeferredAssert	_1 bind0 guarded0	y	If,Case,For,While,Repeat,Forever,Block,Fork,DelayCtrl,EventCtrl,Wait	-	Flat	COLLECTOR
select_scope.rs	blocks	HAND	gather_nested_block_locals,actions	-	-	-	-	Flat	COLLECTOR
stmt_main.rs	lower_stmt_inner	WALKER-OWN	ALL	n	n	-	-	LEXICAL	LEX-PRODUCER
sva_decl.rs	rewrite_sampled_stmt	WALKER-OWN	Block,Blocking,Case,If,NonBlocking,SysTaskCall,UserTaskCall	_0 bind1 guarded0	n	For,While,Repeat,Forever,Fork,DelayCtrl,EventCtrl,Wait,DeferredAssert,ConcurrentAssert	COLLECTOR-SKIP	-	-
"#;
    pub const H2_COUNTS: &str = "{\"HAND\": 1, \"THE-DESCENT\": 1, \"WALKER-FENS\": 13, \"WALKER-OWN\": 47, \"WALKER-VIA-HELPER\": 1} kinds {\"COLLECTOR-SKIP\": 7, \"DESCEND\": 3, \"GATED\": 4, \"INTENDED\": 1, \"LEAF-ONLY\": 1, \"PREDICATE-DECLINE\": 10, \"PREDICATE-PERMISSIVE\": 11} models {\"Flat\": 30, \"FlatLastWins\": 1, \"LEXICAL\": 9} roles {\"BINDER\": 2, \"COLLECTOR\": 5, \"COLLECTOR+MEMBERSHIP\": 2, \"COLLECTOR+ONE-DECL\": 1, \"LEX-COLLECTOR\": 4, \"LEX-COMPARER\": 3, \"LEX-PRODUCER\": 2, \"MEMBERSHIP\": 7, \"ONE-DECL\": 14}";
    /// H3: the 25 sites of u0 T2.txt:281-315 (kinds in source order of the sites ;
    /// DEFINITION = driver.rs run's per-definition census pass, T2 "definition context" ;
    /// CONSTRUCTOR = the initial value).
    pub const H3: &str = r#"file	fn	forms	sites	kinds
cont_array.rs	check_whole_array_sole_writer	assign=2,take=1	1	DIAGNOSTIC
cont_var.rs	check_var_cont_assign_sole_writer	assign=3,take=1	1	DIAGNOSTIC
cover_synth.rs	materialize_cover	assign=1,replace=1	1	RE-ENTRY
dead_arm.rs	settle_dead_arm	assign=1,replace=1	1	DIAGNOSTIC
driver.rs	new	init=1	1	CONSTRUCTOR
driver.rs	run	assign=2,replace=2	2	DEFINITION+SCOPE-ENTRY
edge_gate.rs	report_at	assign=1,replace=1	1	DIAGNOSTIC
edge_gate.rs	edge_backstop	assign=1,take=1	1	DIAGNOSTIC
frames_reserve.rs	with_rtn_decl_scope	assign=1,replace=1	1	SCOPE-ENTRY
generate.rs	with_scope	assign=1,replace=1	1	SCOPE-ENTRY
held.rs	report_clk_refused	assign=1,replace=1	1	DIAGNOSTIC
hier_defer/write.rs	resolve_pending_fill_widths	assign=1,replace=1	1	RE-ENTRY
iface_inst.rs	elaborate_iface_instances	assign=2,replace=2	2	SCOPE-ENTRY+SCOPE-ENTRY
instance.rs	elaborate_instance	assign=1,replace=1	1	SCOPE-ENTRY
package.rs	elaborate_package	assign=1,replace=1	1	SCOPE-ENTRY
ports.rs	bind_iface_port	assign=1,replace=1	1	LOOKUP
ports.rs	wire_array_port	assign=3,replace=1	1	PARENT-SIDE
ports.rs	wire_ports	assign=6,replace=1	2	PARENT-SIDE+PARENT-SIDE
readmem_assoc.rs	check_readmem_assoc_waits	assign=2	1	DIAGNOSTIC
scope.rs	default_binding_matches_decl_scope	assign=1,replace=1	1	LOOKUP
sva_check.rs	materialize_sva_checkers	assign=1,replace=1	1	RE-ENTRY
var_init.rs	flush_block_local_inits	assign=1,replace=1	1	RE-ENTRY
"#;
    pub const H3_COUNTS: &str = "sites 25 {\"CONSTRUCTOR\": 1, \"DEFINITION\": 1, \"DIAGNOSTIC\": 7, \"LOOKUP\": 2, \"PARENT-SIDE\": 3, \"RE-ENTRY\": 4, \"SCOPE-ENTRY\": 7}";
    /// H6: SITE = plan3/walk_sites.txt's 51 (= g2a m_hits_calls.tsv less scope.rs:35 :77
    /// :145) ; HIER / RTN / PROBE = the three the 51 lack (g3 REPORT K5 N1) ; INTERNAL =
    /// the walk's own body.
    pub const H6: &str = r#"file	fn	entry	hit_text	class	hit_set	calls
array_geom.rs	shape_bits	walk_scopes_key	hit_value	SITE	params	1
block_local/hoist.rs	hoist_one_block_local	walk_scopes_key	pkg_var_aliases	SITE	pkg_var_aliases	1
bound/read.rs	name_meta	walk_scopes	param_meta	SITE	param_meta	1
bound/read.rs	name_real	walk_scopes	real_param_val	SITE	real_param_val	1
const_array.rs	const_array_ref_of_base	walk_scopes_key	array_const_vals|params|symbols	SITE	array_const_vals+params+symbols	1
const_array.rs	const_array_ref_of_base	walk_scopes_key	pkg_var_aliases	SITE	pkg_var_aliases	1
const_bound.rs	unfoldable_reason_in	walk_scopes_key	hit_any_const	SITE	params+real_param_val+str_param_raw+wide_param_bits	1
const_decl_width.rs	narrow_param_decl_width	walk_scopes_key	hit_value_or_net	SITE	params+symbols	1
const_eval.rs	array_param_init_pattern	walk_scopes_key	symbols	SITE	symbols	1
const_eval.rs	wide_param_name_in	walk_scopes_key	hit_wide	SITE	wide_param_bits	1
const_fn_width/fold_x.rs	names_real_binding	walk_scopes_key	real_param_val|params|symbols	SITE	params+real_param_val+symbols	1
const_str.rs	const_str_at	walk_scopes_key	str_param_raw|real_param_val|params|wide_param_bits|symbols	SITE	params+real_param_val+str_param_raw+symbols+wide_param_bits	1
const_wide.rs	narrow_param_bits	walk_scopes_key	hit_value_or_net	SITE	params+symbols	1
const_wide.rs	wide_name_bits	walk_scopes_key	hit_wide_value_or_net	SITE	params+symbols+wide_param_bits	1
decl_collide.rs	modport_call_refused	walk_scopes_key	iface_insts	SITE	iface_insts	1
dynarr.rs	dyn_handle	walk_scopes_key_shadowed	fixed_string_dyn_key	SITE	fixed_string_dyn_key	1
edge_gate.rs	binding_encloses	walk_scopes_key	hit_value	SITE	params	1
edge_gate.rs	binding_encloses	walk_scopes_key	params|other_kind()	SITE	params+real_param_val+str_param_raw+symbols+wide_param_bits	1
edge_gate.rs	variable_named_in	walk_scopes_key	hit_wide	SITE	wide_param_bits	1
edge_gate.rs	wide_lsb_range_of	walk_scopes_key	hit_wide_value_or_net	SITE	params+symbols+wide_param_bits	1
edge_gate.rs	wide_param_named_in	walk_scopes_key	hit_wide	SITE	wide_param_bits	1
expr_special.rs	bits_of_view	walk_scopes_key	hit_value_or_net	SITE	params+symbols	1
frames_static_init.rs	admitted_local_keys	walk_scopes_key	symbols	SITE	symbols	1
frames_static_init.rs	declined_local_keys	walk_scopes_key	symbols	SITE	symbols	1
frames_static_init.rs	emit_frame_static_prologue	walk_scopes_key	symbols	SITE	symbols	1
frames_static_init.rs	expr_reads_admitted	walk_scopes_key	symbols	SITE	symbols	1
frames_static_init.rs	frame_init_name_is_local_or_const	walk_scopes_key	symbols	SITE	symbols	1
gen_scope_name.rs	gen_spelling_hint	scoped_key_at_or_above	gen_loop_labels	SITE	gen_loop_labels	1
gen_scope_name.rs	gen_spelling_hint	scoped_key_at_or_above	gen_singleton_labels	SITE	gen_singleton_labels	1
held.rs	clk_refused_at	walk_scopes_key_from	symbols|refused()	SITE	clk_refused+symbols	1
hier.rs	hier_lookup	hier_resolve	symbols	HIER	symbols	1
hier.rs	hier_lookup_param	hier_resolve	hier_params	HIER	hier_params	1
hier.rs	hier_lookup_param_meta	hier_resolve	param_meta	HIER	param_meta	1
hier.rs	hier_lookup_wide_param	hier_resolve	wide_param_bits	HIER	wide_param_bits	1
hier_defer/bits.rs	build_hier_param_select	hier_resolve	hier_param_range	HIER	hier_param_range	1
hier_defer/func_call.rs	resolve_deferred_hier_func_call_one	hier_resolve	hier_funcs	HIER	hier_funcs	1
hier_defer/read.rs	resolve_deferred_hier_call	hier_resolve	hier_funcs	HIER	hier_funcs	1
hier_defer/read.rs	resolve_deferred_hier_call	scoped_key_at_or_above	enum_label_types	SITE	enum_label_types	1
hier_defer/read.rs	resolve_one_deferred_hier_sel	hier_resolve	fixed_string_dyn_key	HIER	fixed_string_dyn_key	1
hier_defer/task_call.rs	resolve_deferred_hier_task_call	hier_resolve	hier_tasks	HIER	hier_tasks	1
hier_defer/write.rs	every_name_resolves	walk_scopes_key	symbols|params	SITE	params+symbols	1
hier_defer/write.rs	resolve_deferred_hier_sel_write	hier_resolve	fixed_string_dyn_key	HIER	fixed_string_dyn_key	1
ident_route.rs	ast_reads_guessed_param	walk_scopes_key	hit_value	SITE	params	1
ident_route.rs	bare_ident_route	walk_scopes_key	hit_any_const_or_net	SITE	params+real_param_val+str_param_raw+symbols+wide_param_bits	1
ident_route.rs	bare_ident_route	walk_scopes_key	hit_value	SITE	params	1
lvalue.rs	collect_lval_chunks	walk_scopes_key_shadowed	string_array_elems	SITE	string_array_elems	1
package.rs	bare_hit_is_shadowed_pkg_alias	walk_scopes_key	genvar_decls	SITE	genvar_decls	1
package.rs	bare_hit_is_shadowed_pkg_alias	walk_scopes_key	symbols	SITE	symbols	1
package.rs	dotted_hit_is_pkg_alias	walk_scopes_key	symbols	SITE	symbols	1
package.rs	resolve_rtn_key	walk_rtn_scopes	func_table|task_table	RTN	func_table+task_table	1
param_query.rs	real_param_is_non_integral	walk_scopes_key	hit_real_value_or_net	SITE	params+real_param_val+symbols	1
param_query.rs	real_param_lowers_real	walk_scopes_key	real_param_val|params|symbols	SITE	params+real_param_val+symbols	1
params.rs	param_sel_range	walk_scopes_key	hit_value_or_net	SITE	params+symbols	1
ports.rs	bind_iface_port	walk_scopes_key	iface_insts	SITE	iface_insts	1
ports.rs	check_modport_write	walk_scopes_key	symbols	SITE	symbols	1
ports.rs	wire_ports	walk_scopes_key	symbols	SITE	symbols	1
scope.rs	check_decl_precedes_use	walk_scopes_key	symbols	SITE	symbols	1
scope.rs	lookup_net_scoped	walk_scopes_key	symbols	SITE	symbols	1
scope.rs	lookup_scoped	walk_scopes	params	SITE	params	1
scope.rs	walk_scopes	walk_scopes_key	table	INTERNAL	table	1
scope.rs	walk_scopes_key	walk_scopes_key_inner	hit	INTERNAL	hit	1
scope.rs	walk_scopes_key_from	walk_out_from	hit	INTERNAL	hit	1
scope.rs	walk_scopes_key_inner	decl_probe_key	hit	PROBE	hit	1
scope.rs	walk_scopes_key_inner	walk_out_from	hit	INTERNAL	hit	1
scope.rs	walk_scopes_key_shadowed	walk_scopes_key_inner	hit	INTERNAL	hit	1
strings.rs	is_string_array_base	walk_scopes_key_shadowed	string_array_elems	SITE	string_array_elems	1
strings.rs	string_array_elem_net	walk_scopes_key_shadowed	string_array_elems	SITE	string_array_elems	1
struct_arm.rs	struct_arm_rhs	walk_scopes_key	symbols	SITE	symbols	1
"#;
    pub const H6_COUNTS: &str = "{\"HIER\": 10, \"INTERNAL\": 5, \"PROBE\": 1, \"RTN\": 1, \"SITE\": 51} site_texts 25 site_sets 20";
    /// H6c: g2b composites.tsv (103 ; class codes in g2b/tools/mk_composites.py).
    pub const H6C: &str = r#"file	fn	tier	class
arrays.rs	expr_array_chain	D	SAME
arrays.rs	lval_array_chain	D	SAME
arrays.rs	lval_array_view	D	WBIND
arrays.rs	whole_name_net	D	OWN
ast_query.rs	count_reads_runtime_net	D	OWN
block_local/gate.rs	deny_static_init_reading_per_entry	A/C	PLACE
block_local/hoist.rs	hoist_one_block_local	D	PLACE
block_local/hoist.rs	hoist_block_local_nets	D	PLACE
bound/frame.rs	is_empty	A/C	COPY
bound/frame.rs	is_local	D	FRAME
bound/frame.rs	is_local_w	D	FRAME
class_lower.rs	resolve_class_member	D	OWN
class_lower.rs	lower_class_method_body	A/C	COPY
class_lower.rs	try_class_method_call	A/C	XKIND
classes.rs	ast_handle_kind	A/C	XKIND
cond_names.rs	cond_names_ok	A/C	OWN
const_array.rs	const_array_ref_of_base	D	OWN
const_bound.rs	ast_mentions_substituted_name	D	OWN
const_bound.rs	unfoldable_reason_in	A/C	OWN
const_decl_width.rs	narrow_param_decl_width	D	OWN
const_eval.rs	nonconst_bound_reason	D	OWN
const_fn.rs	const_placement_wide	D	FRAME
const_fn.rs	const_fn_def	D	RNS
const_level_header.rs	task_enable_blocker	A/C	RNS
const_real.rs	const_eval_real_in_scope	D	OWN
const_select.rs	const_select_base	D	OWN
const_wide.rs	narrow_param_bits	D	OWN
cover.rs	register_cover_instance	A/C	PLACE
decl_scope.rs	decl_body_win	A/C	COPY
driver.rs	run	A/C	COPY
dynarr.rs	dyn_handle	D	OWN
dynarr.rs	dyn_array_actual_net	D	OWN
edge_gate.rs	binding_encloses	A/C	OWN
edge_gate.rs	variable_named_in	D	OWN
edge_gate.rs	wide_lsb_param_in	D	OWN
events.rs	sens_event_net	D	OWN
expr_main.rs	lower_expr_ungated	D	SAME
expr_size_ctx.rs	ctx_signed_impl	D	OWN
expr_size_ctx.rs	select_chain	D	SAME
expr_size_ctx.rs	size_ctx_self_width	D	OWN
expr_size_hier.rs	facts_have_body_write_funcs	A/C	COPY
expr_special.rs	bits_of_view	D	OWN
frames_body.rs	seed_subroutine_routes	A/C	COPY
frames_body.rs	lower_frame_funcs	D	RNS
frames_body.rs	lower_frame_body_inner_lower_frame_func_body	A/C	COPY
frames_call/emit.rs	emit_frame_dyn_formal_markers	D	RNS
frames_classify.rs	build_task_frame_set	A/C	RNS
frames_reserve.rs	with_rtn_decl_scope	D	RNS
frames_static_init.rs	frame_init_name_is_local_or_const	D	OWN
hoist/mod.rs	inout_call_target	D	RNS
hoist/mod.rs	dyn_formal_call_target	D	RNS
ident_route.rs	bare_ident_route	D	R5
ident_route.rs	lookup_net_unshadowed	D	SAME
ident_route.rs	bare_const_shadows_net	D	SAME
iface_inst.rs	elaborate_iface_instances	D	RNS
iface_rtn_scope.rs	tables	A/C	COPY
iface_rtn_scope.rs	take_routine_scope	A/C	COPY
iface_rtn_scope.rs	live_rtn_tables	A/C	COPY
iface_rtn_scope.rs	adopt_static_scoped_frames	A/C	COPY
iface_rtn_scope.rs	restore_routine_scope	A/C	COPY
inline_body_ctx.rs	rhs_has_real_domain	D	SAME
inline_fn.rs	inline_function	D	RNS
inline_fn.rs	reduce_function_body	A/C	COPY
inline_task.rs	inline_task	D	RNS
inline_task.rs	inline_static_task	A/C	OWN
instance.rs	elaborate_instance	A/C	COPY
instance_array.rs	elaborate_instance_array	A/C	OWN
lvalue.rs	collect_lval_chunks	D	WBIND
md_return.rs	md_return_callee	D	XKIND
net_util.rs	net_is_undeclared	D	OWN
package.rs	bare_hit_is_shadowed_pkg_alias	D	OWN
package.rs	resolve_rtn_key	D	RNS
package.rs	apply_import_routines	D	RNS
packed.rs	expr_packed_chain	D	SAME
packed_lval.rs	bound_by_inline_formal	D	OWN
packed_lval.rs	lval_packed_chain	D	SAME
packed_pattern.rs	integral_operand	D	SAME
packed_pattern.rs	integral_select_base	D	SAME
params.rs	param_sel_range	D	OWN
pkg_body_scope.rs	imported_routine_bodies	D	RNS
pkg_scoped_frames.rs	frame_scoped_pkg_routines	D	RNS
pkg_scoped_frames.rs	scoped_reachable_callees	D	RNS
ports.rs	wire_ports	A/C	OWN
scope.rs	default_binding_matches_decl_scope	D	OWN
scope.rs	lookup_dotted_net	D	OWN
scope.rs	check_decl_precedes_use	D	OWN
scope.rs	resolve_net	D	R5TAIL
scope.rs	lookup_net_scoped	D	R5TAIL
scope.rs	resolve_intro_net	D	SAME
select_scope.rs	select_base_is_packed	D	OWN
select_scope.rs	signing_operand_admitted	D	OWN
static_array_method.rs	static_array_recv	D	OWN
stmt_main.rs	lower_stmt_inner	D	RNS
strings.rs	expr_is_string_ast	D	OWN
struct_arm.rs	struct_arm_rhs	D	OWN
struct_arm.rs	struct_arm_context	A/C	COPY
sva_clocking.rs	lower_clocking_blocks	A/C	PLACE
sva_decl.rs	is_property_name	D	XKIND
sva_prop.rs	flatten_prop_consequent	D	XKIND
sva_prop.rs	chain_consequent_property	D	XKIND
sva_prop.rs	flatten_overlap_property	D	XKIND
sva_prop.rs	prop_bool_operand	D	RNS
sva_prop.rs	sva_prev_for	D	SAME
"#;
    pub const H6C_COUNTS: &str = "rows 103 {\"COPY\": 15, \"FRAME\": 3, \"OWN\": 37, \"PLACE\": 5, \"R5\": 1, \"R5TAIL\": 2, \"RNS\": 18, \"SAME\": 13, \"WBIND\": 2, \"XKIND\": 7}";
    /// H7: (a) per fn, the typed `let`s of a String / str collection (170 in 89 fns = g2b
    /// census.locals.tsv's rows with a `String` / `str` type token) ; (b) UNTYPED:
    /// `binding=callee` for a `let` with no type whose initializer calls a crate fn
    /// returning such a collection.
    pub const H7: &str = r#"file	fn	lets	bindings
block_local/gate.rs	deny_static_init_reading_per_entry	1	hits
block_local/hoist.rs	hoist_block_local_nets	2	seen,pushed
block_local/mod.rs	gather_decl_positions	1	pos
block_local/mod.rs	gather_nested_block_locals	1	names
block_local/timed_task.rs	refuse_timing_block_local_hazard	1	names
block_local_class.rs	block_local_scope_prefix	1	segs
block_local_class.rs	classify_scoped_block_locals	2	cand,out
block_local_class.rs	compute_coalesced_block_locals	2	per_name,blocks
block_local_class.rs	compute_scoped_block_locals	UNTYPED	map=classify_scoped_block_locals
block_local_class.rs	compute_timed_blocks	1	path
block_local_class.rs	gather_scoped_one	1	before
block_local_feed.rs	feed_scoped_block_locals	UNTYPED	shadow_names=names_with_pkg_var_aliases,joint=classify_scoped_block_locals
classes.rs	assign_vtables	1	slots
classes.rs	assign_vtables	UNTYPED	chain=class_chain_rootfirst
classes.rs	register_classes	2	own,chain
cont_array.rs	check_whole_array_sole_writer	1	refused
cover_bins.rs	resolve_crosses	1	by_name
crv.rs	class_constraints_for	4	rand_names,randc_names,map,plain_rand
crv.rs	class_constraints_for	UNTYPED	dist_names=dist_field_names
crv.rs	class_dist_for	2	rand_names,map
crv.rs	class_rand_for	2	rand_names,bounds
crv.rs	class_randc_for	1	randc_names
crv.rs	fold_inline_with	5	rand_names,randc_names,map,bounds,plain_rand
crv.rs	fold_inline_with	UNTYPED	dist_names=dist_field_names
decl_collide.rs	check_decl_name_collisions	1	declared
decl_collide.rs	check_decl_name_collisions	UNTYPED	stems=carrier_stems
decl_collide.rs	report_decl_collisions	1	by_name
decl_scope.rs	decl_keep_snapshot	1	keys
driver.rs	run	5	seen,bind_checkers,names,cli_used,unmatched
edge_gate.rs	decide_late_edges	1	reported
expr_size_hier.rs	build_module_facts	1	out
expr_size_hier.rs	child_env	1	bound
expr_size_hier.rs	module_facts	1	seen
expr_size_hier_exact.rs	env_exact	1	bound
expr_size_hier_exact.rs	hier_leaf_width_exact	1	exact
frames_body.rs	lower_frame_body_inner_lower_frame_func_body	UNTYPED	declared=rtn_declared_names,saved_labels=push_body_enum_labels
frames_body.rs	lower_frame_body_inner_lower_frame_task_body	UNTYPED	declared=rtn_declared_names,saved_labels=push_body_enum_labels
frames_body.rs	lower_frame_funcs	UNTYPED	frame_set=build_frame_set,task_set=build_task_frame_set
frames_body.rs	seed_subroutine_routes	2	fnames,tnames
frames_call/emit.rs	emit_frame_task_call	1	out_array_unpacks
frames_classify.rs	body_duplicates_a_non_repeatable_local	1	pending
frames_classify.rs	body_reads_only_locals	1	locals
frames_classify.rs	body_selects_mdpacked_local	1	names
frames_classify.rs	build_frame_set	3	set,func_widths,edges
frames_classify.rs	build_task_frame_set	1	edges
frames_classify_write.rs	ast_func_body_writes_outside	UNTYPED	own=rtn_declared_names
frames_static_init.rs	block_wrap_prefix	1	segs
frames_static_init.rs	emit_frame_static_prologue	3	cands,formals,admitted_keys
gen_enum.rs	gen_carried_typedefs	UNTYPED	names=scope_names
gen_enum.rs	scope_names	1	names
gen_scope_name.rs	key_spells_indexed_singleton	1	segs
generate.rs	display_of	1	out
held.rs	clk_refused_path	1	names
held.rs	deny_refused_clocking_input_arg	1	joined
hier.rs	hier_resolve	1	segs
hier.rs	lval_hier_sel_chain	UNTYPED	path=lval_hier_chain
hier_defer/func_call.rs	emit_deferred_hier_func_call	1	path
hier_defer/write.rs	resolve_deferred_hier_sel_write	2	patch,force_release_paths
hier_leaf_shape.rs	record_hier_call_shape	1	names
hier_leaf_shape.rs	record_hier_net_shape	1	names
hier_leaf_shape.rs	record_hier_sel_shape	1	names
hoist/general.rs	hoist_inout_general_seq	1	snaps
hoist/general.rs	hoist_inout_general_seq	UNTYPED	roots=order_plan
iface_inst.rs	elaborate_iface_instances	6	saved_params,local_const_funcs,wc_const_fn,explicit_const_fn,wc_rtn,explicit_rtn
iface_inst.rs	elaborate_iface_instances	UNTYPED	local_names=gather_local_decl_names,sp=bind_params,dpos=gather_decl_positions,dbl=gather_block_local_names,shadow_names=names_with_pkg_var_aliases,scoped_blocks=compute_scoped_block_locals,per_entry_blocks=compute_per_entry_block_locals,coalesced=compute_coalesced_block_locals
iface_rtn_scope.rs	move_static_scoped	UNTYPED	keys=static_scoped_keys
inline_fn.rs	body_needs_context_width	2	widths,unknown_bv
inline_fn.rs	inline_pkg_function	1	pkg_rtn_names
inline_fn.rs	inline_pkg_function	UNTYPED	pkg_const_names=pkg_scope_names,refused=inject_pkg_callees
inline_fn.rs	inline_resolved_func_in_pkg	1	dyn_binds
inline_fn.rs	inline_resolved_func_in_pkg	UNTYPED	declared=rtn_declared_names
inline_fn.rs	reduce_function_body	6	decl_consts,seen,local_dims,two_state,writable,non_bv
inline_fn.rs	reduce_function_body	UNTYPED	saved_labels=push_body_enum_labels
inline_ret_select.rs	body_binding_hazard	2	owner,formals
inline_ret_select.rs	expr_bound	1	names
inline_task.rs	inline_static_task	1	dyn_binds
inline_task.rs	inline_static_task	UNTYPED	saved_labels=push_body_enum_labels,declared=rtn_declared_names
inline_task_locals.rs	hoist_inline_task_locals	1	hoisted_here
instance.rs	elaborate_instance	7	conflict_names,local_const_funcs,wc_const_fn,explicit_const_fn,saved_params,wc_rtn,explicit_rtn
instance.rs	elaborate_instance	UNTYPED	names=gather_local_decl_names,dpos=gather_decl_positions,dbl=gather_block_local_names,shadow_names=names_with_pkg_var_aliases,scoped_blocks=compute_scoped_block_locals,per_entry_blocks=compute_per_entry_block_locals,coalesced=compute_coalesced_block_locals,shadow_names=names_with_pkg_var_aliases,map=compute_scoped_block_locals
instance_array.rs	elaborate_instance_array	1	port_widths
instance_array.rs	elaborate_instance_array	UNTYPED	saved=bind_params
multidriver.rs	check_multidriver_processes	2	rule_a_fires,via_call_writers
multidriver.rs	multidriver_vars	2	body_decls,declared
net_util.rs	declare_implicit_nets	2	iface_insts,want
net_util.rs	push_body_enum_labels	1	block_local_names
package.rs	apply_import_consts	3	pkg_labels,wides,vars
package.rs	apply_import_routines	1	wildcard_roots
package.rs	apply_import_routines	UNTYPED	<pat>=inject_pkg_callees,<pat>=inject_pkg_callees
package.rs	elaborate_package	22	saved,saved_meta,saved_real,saved_str,saved_wide,consts,real_vals,str_vals,wide_vals,param_names,enum_labels,const_meta,const_range,saved_range,vars,array_vals,array_meta,funcs,tasks,types,pkg_local_names,pkg_conflict_names
package.rs	inject_pkg_callees	4	rtn_names,refused,want,seen
package.rs	inject_pkg_callees	UNTYPED	const_names=pkg_scope_names
package.rs	pkg_func_self_contained	2	names,read_names
package.rs	push_pkg_consts_scoped	UNTYPED	skip=rtn_declared_names
package.rs	push_pkg_consts_skipping	1	wides
package.rs	push_pkg_consts_task	UNTYPED	skip=rtn_declared_names
package.rs	task_self_contained_impl	2	names,read_names
param_dup.rs	check_duplicate_param_decls_in	2	first,reports
param_dup.rs	check_duplicate_param_decls_in	UNTYPED	stems=carrier_stems
params.rs	resolve_param_overrides	1	dim_arity_reported
pkg_body_scope.rs	imported_routine_bodies	1	keys
pkg_body_scope.rs	rtn_declared_names	1	names
pkg_scoped_frames.rs	frame_scoped_pkg_routines	2	late,cands
pkg_scoped_frames.rs	frame_scoped_pkg_routines	UNTYPED	frame_set=build_frame_set,reachable=scoped_reachable_callees
pkg_scoped_frames.rs	scoped_reachable_callees	3	want,seen,keys
ports.rs	bind_iface_port	2	mp_dirs,aliases
ports.rs	elaborate_virtual_iface	1	bound
ports.rs	elaborate_virtual_iface	UNTYPED	members=iface_member_names
ports.rs	wire_ports	1	wildcard_conns
ports.rs	wire_ports	UNTYPED	ports=port_list_dirs
proc_builder.rs	reaches	2	stack,seen
stmt_main.rs	lower_stmt_inner	UNTYPED	map=sva_formal_map
sva_check.rs	synth_local_var_assert	2	captures,data_chain
sva_clocking.rs	synth_crossclock	1	handoffs
sva_fsm.rs	consec_run_fsm	1	regs
sva_fsm.rs	goto_fsm	1	regs
toplevel.rs	pick_roots	1	added
toplevel.rs	pick_roots	UNTYPED	instantiated=collect_instantiated
var_init.rs	assert_block_local_inits_drained	1	keys
var_init.rs	flush_block_local_inits	3	keys,all,runs
wext.rs	check_conn_spans	UNTYPED	v=conn_span_violations
"#;
    /// H7: the 26 readers' name collections of g2b REPORT:70-75 (5 inside the 40 readers,
    /// 21 in a direct caller) ; a field row's owner is the fn holding the struct (`-` =
    /// module level).
    pub const H7_READERS: &str = r#"file	fn_or_owner	form	binding
block_local/mod.rs	gather_nested_block_locals	let	names
block_local/hoist.rs	hoist_block_local_nets	let	seen
block_local/hoist.rs	hoist_block_local_nets	let	pushed
block_local_class.rs	compute_timed_blocks::walk	let	names
select_scope.rs	blocks	let	out
inline_fn.rs	body_needs_context_width	let	widths
inline_fn.rs	body_needs_context_width	let	unknown_bv
frames_classify.rs	body_reads_only_locals	let	locals
frames_classify.rs	body_selects_mdpacked_local	let	names
frames_classify_write.rs	ast_func_body_writes_outside	let	own
inline_ret_select.rs	body_binding_hazard	let	owner
inline_ret_select.rs	body_binding_hazard	let	formals
inline_ret_select.rs	-	field:BindWalk	assigned
package.rs	pkg_func_self_contained	let	names
package.rs	pkg_func_self_contained	let	read_names
package.rs	task_self_contained_impl	let	names
package.rs	task_self_contained_impl	let	read_names
inline_fn.rs	reduce_function_body	let	decl_consts
inline_fn.rs	reduce_function_body	let	seen
inline_fn.rs	reduce_function_body	let	local_dims
inline_fn.rs	reduce_function_body	let	two_state
inline_fn.rs	reduce_function_body	let	writable
inline_fn.rs	reduce_function_body	let	non_bv
block_local_class.rs	compute_timed_blocks	let	path
block_local_class.rs	compute_timed_blocks	field:Acc	sharers
block_local_class.rs	compute_timed_blocks	field:Acc	scoped
"#;
    pub const H7_COUNTS: &str = "lets 170 fns 89 untyped 32 readers 26";
}
