//! The scope censuses of §5.2 Q-LEXSCOPE (unit LS1, parts 1 and 2), as home tests: twelve
//! syntax censuses of `crates/elaborate/src/**/*.rs`, each computed by `syn` (full) and
//! compared with a pinned table. The tables live in ONE data file beside this one,
//! `scope_home.tables` (moved there in part 2, when this file passed ~4500 lines with them
//! inline), a section per table ; each table's note there names its rule and its source.
//! A census row is keyed by (file, fn): `fn` is the
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
//! Part 2 (a second visitor, `Pass2`, reads a macro as g3's census did: a `matches!` with
//! its guard, else the arguments as an expression list, else the first expression):
//!
//! - HM the mints of scope-qualified text (386 = 375 by syntax + 11 derived-fn rows, HAND):
//!   a mint fn call, a `format!` / `write!` glue literal, `.join(".")`, a separator push,
//!   a `cur_prefix.clone()` ; a mint inside a `matches!` guard counts (strings.rs). TAG
//!   (hand) = key / scope / segment / display / spelling.
//! - HS the key scans (32 = 30 by syntax + 2 HAND): an iteration over a `String`-keyed
//!   map / set field of `Elaborator` (the store set, HS_STORES, is itself computed and
//!   pinned) ; KIND and ENTRY (the disposition, g4 entries.tsv) are hand columns.
//! - HH the current-prefix reads (59): a probe of a store whose key is an `fq` mint, role
//!   whole / composite. It PINS today's sites ; it does not close the door: a read through
//!   a key the syntax does not see as minted from `cur_prefix` is not counted.
//! - HSEG the segment-text tests (99): a text test whose literal holds `.` `$` `[` `::`, any
//!   `strip_prefix`, a non-literal prefix / suffix test, `is_gen_scope_segment` ; TAG and
//!   UNIT (walk, LS2, LS4, DK1, DK3, DK4, RS1, `-`) are hand columns.
//! - H5 the name-typed fields (215) with g1's CLASS / CODE and a STORE column (hand).
//! - HD the display-chain fields' accesses (`block_scope`, `block_scope_root`,
//!   `cur_class_method`): read / write / init.
//! - HK the producers' scope kinds: per fn the `cur_prefix` writes (H3's 25 sites), the
//!   `with_scope` calls and the `elaborate_instance` calls, KIND a hand column.
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
//! at main 87e48d99 ; every census number equals its source's there. Part 2's: g1
//! fields.tsv, g3 mints.tsv / scans.tsv / k4_reads.tsv / fields_keyed.tsv /
//! name_keyed_fields.txt, g4 REPORT.txt (OWED-a, -d, -k, -r) and c/entries.tsv, plan6
//! DELTA6.txt F1 F2 F5 ; where one differs from the code the code is pinned (LS1 part 2's
//! REPORT lists the rows: HS's store set, HS's two HAND scans).
//!
//! Part 2 does not see: a mint, scan or probe written through a macro other than the
//! three forms above, through a receiver or a local not spelled as the rule says, or in
//! a helper the key travels through ; a HAND row is checked only for its fn (HM, HS) or its
//! fn plus the call (HS).
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
    for row in table(&tables::get("H2")) {
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
    if counts != tables::get("H2_COUNTS") {
        msgs.push(format!(
            "H2 COUNTS table [{}] now [{counts}] — set H2_COUNTS with the row change that moved it",
            tables::get("H2_COUNTS")
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
    for row in table(&tables::get("H3")) {
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
    if counts != tables::get("H3_COUNTS") {
        msgs.push(format!(
            "H3 COUNTS table [{}] now [{counts}]",
            tables::get("H3_COUNTS")
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
    for row in table(&tables::get("H6")) {
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
    if counts != tables::get("H6_COUNTS") {
        msgs.push(format!(
            "H6 COUNTS table [{}] now [{counts}]",
            tables::get("H6_COUNTS")
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
    for row in table(&tables::get("H6C")) {
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
    if counts != tables::get("H6C_COUNTS") {
        msgs.push(format!(
            "H6c COUNTS table [{}] now [{counts}]",
            tables::get("H6C_COUNTS")
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
    for row in table(&tables::get("H7")) {
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
    for row in table(&tables::get("H7_READERS")) {
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
    if counts != tables::get("H7_COUNTS") {
        msgs.push(format!(
            "H7 COUNTS table [{}] now [{counts}]",
            tables::get("H7_COUNTS")
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
// Part 2: a second pass (census_g3's reading of macros) for HM HS HH HSEG HD HK, and the
// struct reach for H5 and HS's store set
// =====================================================================================

/// HM: the mint fns (g3 census_g3.rs MINTS / link.py CALLS).
const MINT_CALLS: &[&str] = &[
    "fq",
    "child_prefix",
    "walk_scopes_key",
    "walk_scopes_key_shadowed",
    "walk_scopes_key_from",
    "walk_scopes_key_inner",
    "walk_out_from",
    "scoped_key_at_or_above",
    "walk_rtn_scopes",
    "hier_resolve",
    "hier_key_within",
    "singleton_scope_key",
    "indexed_gen_singleton",
    "decl_probe_key",
    "gen_rtn_key",
    "walk_scopes",
];
/// HM: a glue literal that starts so is a temp name (link.py TEMP).
const MINT_TEMP: &[&str] = &[
    "$ia_",
    "$dyn_tmp",
    "$repeat_cnt",
    "$sinit",
    "$sfmt_tmp",
    "$popsink",
    "__outpack",
    "__houtpack",
    "{}$sae",
    "{}$sad",
    "$unp$",
    "$ia_ret",
];
/// HM: the fns whose every `push_str` is a mint piece (link.py).
const MINT_PUSH_STR_FNS: &[&str] = &["display_of", "walk_out_from", "gen_rtn_key"];
/// HS: an iteration over a store (census_g3 SCAN).
const SCAN_METHODS: &[&str] = &[
    "iter",
    "keys",
    "values",
    "range",
    "retain",
    "iter_mut",
    "into_iter",
    "drain",
    "values_mut",
];
/// HS / HD: a receiver that is the `Elaborator` (scans.py's window).
const RECV2: &[&str] = &["self", "s", "me", "this"];
/// HS: the map / set types a store set is keyed by.
const KEYED_COLL: &[&str] = &[
    "BTreeMap", "HashMap", "BTreeSet", "HashSet", "IndexMap", "IndexSet",
];
/// HH: a probe of a store (k4.py READ less the compiler-only ops).
const HH_READ: &[&str] = &["get", "contains_key", "contains", "range", "get_mut"];
/// HH: the walks that make a read a composite step (k4.py WALK).
const HH_WALK: &[&str] = &[
    "walk_scopes_key",
    "walk_scopes_key_shadowed",
    "walk_scopes_key_from",
    "scoped_key_at_or_above",
    "walk_scopes",
    "walk_rtn_scopes",
    "hier_resolve",
];
/// HSEG: the text-test methods.
const SEG_METHODS: &[&str] = &[
    "split",
    "rsplit",
    "split_once",
    "rsplit_once",
    "splitn",
    "rsplitn",
    "split_terminator",
    "find",
    "rfind",
    "starts_with",
    "ends_with",
    "strip_prefix",
    "strip_suffix",
    "contains",
    "trim_start_matches",
    "trim_end_matches",
];
/// HSEG: the methods whose non-literal argument is a text-ancestry test or cut.
const SEG_DYN: &[&str] = &["starts_with", "strip_prefix", "ends_with", "strip_suffix"];
/// HD: the display-chain fields (DELTA6 F1).
const DISPLAY_FIELDS: &[&str] = &["block_scope", "block_scope_root", "cur_class_method"];
/// HD: a method that mutates its receiver (census_fields.rs MUT).
const HD_MUT: &[&str] = &[
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
    "first_mut",
    "swap",
    "drain",
    "pop",
    "resize",
    "extend_from_slice",
    "splice",
    "swap_remove",
    "split_off",
    "sort",
    "sort_by",
    "sort_by_key",
    "sort_unstable",
    "dedup",
    "reverse",
    "fill",
    "clone_from",
    "as_mut",
    "entry",
    "or_insert",
    "or_default",
    "or_insert_with",
    "values_mut",
    "take",
    "replace",
    "set",
    "borrow_mut",
    "push_str",
    "push_back",
    "push_front",
    "pop_back",
    "pop_front",
    "retain_mut",
    "get_or_insert_with",
    "insert",
];
/// HK: the scope-entry callees whose callers know the kind.
const HK_CALLEES: &[&str] = &["with_scope", "elaborate_instance"];
/// H5: a type ident that makes a field name-typed (census_fields.rs NAME_TYPES).
const NAME_TYPES: &[&str] = &["String", "str", "Symbol", "Ident"];

/// What the second pass records per fn.
#[derive(Default)]
struct Fn2 {
    file: String,
    qual: String,
    simple: String,
    line: usize,
    /// HM: (mint kind, name)
    mints: Vec<(String, String)>,
    /// HS: (field, method) of an iteration over `<recv>.<field>`
    scans: Vec<(String, String)>,
    /// HH: the `let` bindings holding an `fq` mint (the latest binding of a name)
    fq_locals: BTreeMap<String, String>,
    /// HH: (store, op, mint) — mint = `let:<name>@<n>` or `call<n>`, n the fq call's ordinal
    reads: Vec<(String, String, String)>,
    fq_calls: usize,
    /// every called name (method or path)
    calls: BTreeSet<String>,
    /// HSEG: (method, argument)
    segs: Vec<(String, String)>,
    /// HD: (field, access)
    disp: Vec<(String, String)>,
    /// HK: the scope-entry calls
    entries: Vec<String>,
}

struct Pass2<'a> {
    file: String,
    stack: Vec<usize>,
    out: &'a mut Vec<Fn2>,
    write_nodes: BTreeSet<usize>,
}

/// Through `&`, `( )`, `.borrow()` / `.borrow_mut()`: the `(base, field)` of a field access.
fn field_of2(e: &Expr) -> Option<(&Expr, String)> {
    match e {
        Expr::Field(f) => match &f.member {
            syn::Member::Named(i) => Some((&f.base, i.to_string())),
            syn::Member::Unnamed(_) => None,
        },
        Expr::Reference(r) => field_of2(&r.expr),
        Expr::Paren(p) => field_of2(&p.expr),
        Expr::MethodCall(m) if m.method == "borrow" || m.method == "borrow_mut" => {
            field_of2(&m.receiver)
        }
        _ => None,
    }
}

fn is_recv2(e: &Expr) -> bool {
    matches!(single_path(e), Some(n) if RECV2.contains(&n.as_str()))
}

/// Through `&`, `( )` and a value-keeping method: the expression a key argument is.
fn strip_key(e: &Expr) -> &Expr {
    match e {
        Expr::Reference(r) => strip_key(&r.expr),
        Expr::Paren(p) => strip_key(&p.expr),
        Expr::MethodCall(m)
            if matches!(
                m.method.to_string().as_str(),
                "clone" | "as_str" | "as_ref" | "to_string" | "to_owned" | "as_deref"
            ) =>
        {
            strip_key(&m.receiver)
        }
        _ => e,
    }
}

/// `<x>.fq(..)` or `fq(..)`.
fn is_fq_call(e: &Expr) -> bool {
    match strip_key(e) {
        Expr::MethodCall(m) => m.method == "fq",
        Expr::Call(c) => {
            matches!(&*c.func, Expr::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "fq"))
        }
        _ => false,
    }
}

/// A `"…"` literal of glue: letters, digits, `_ [ ] $ .` and `{…}` holes (link.py GLUE).
fn is_glue(lit: &str) -> bool {
    let b = lit.as_bytes();
    if b.len() < 2 || b[0] != b'"' || b[b.len() - 1] != b'"' {
        return false;
    }
    let inner = &b[1..b.len() - 1];
    let mut i = 0;
    while i < inner.len() {
        let c = inner[i];
        if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'[' | b']' | b'$' | b'.') {
            i += 1;
        } else if c == b'{' {
            let mut j = i + 1;
            while j < inner.len() && inner[j] != b'}' && inner[j] != b'{' {
                j += 1;
            }
            if j >= inner.len() || inner[j] != b'}' {
                return false;
            }
            i = j + 1;
        } else {
            return false;
        }
    }
    true
}

/// HM: the kind of a `format!` / `write!` glue literal, if it is a mint.
fn glue_kind(lit: &str) -> Option<&'static str> {
    if !is_glue(lit) || !(lit.contains('.') || lit.contains('$')) {
        return None;
    }
    let inner = &lit[1..lit.len() - 1];
    let scoped = ["$blk$", "$func$", "$pkg$", "$class$", "$itask$"]
        .iter()
        .any(|p| inner.contains(p))
        || inner.contains('.');
    if MINT_TEMP.iter().any(|p| inner.starts_with(p)) || !scoped {
        Some("fmt-tempname")
    } else {
        Some("fmt")
    }
}

/// HSEG: a literal argument's source text and value.
fn lit_arg(e: &Expr) -> Option<(String, String)> {
    match e {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Str(s) => Some((s.token().to_string(), s.value())),
            syn::Lit::Char(c) => Some((c.token().to_string(), c.value().to_string())),
            _ => None,
        },
        Expr::Reference(r) => lit_arg(&r.expr),
        Expr::Paren(p) => lit_arg(&p.expr),
        _ => None,
    }
}

impl Pass2<'_> {
    fn cur(&mut self) -> Option<&mut Fn2> {
        let i = *self.stack.last()?;
        Some(&mut self.out[i])
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
        self.out.push(Fn2 {
            file: self.file.clone(),
            qual,
            simple,
            ..Default::default()
        });
        self.stack.push(self.out.len() - 1);
        true
    }

    fn mint(&mut self, kind: &str, name: &str) {
        if let Some(c) = self.cur() {
            c.mints.push((kind.to_string(), name.to_string()));
        }
    }

    fn called(&mut self, name: &str) {
        if let Some(c) = self.cur() {
            c.calls.insert(name.to_string());
            if HK_CALLEES.contains(&name) {
                c.entries.push(name.to_string());
            }
        }
        if MINT_CALLS.contains(&name) {
            self.mint("call", name);
        }
        if name == "is_gen_scope_segment" {
            if let Some(c) = self.cur() {
                c.segs.push((name.to_string(), "-".to_string()));
            }
        }
    }

    /// HH: a store probe whose key is an `fq` mint, directly or through a `let`.
    fn key_read(&mut self, store: &str, op: &str, arg: &Expr) {
        let k = strip_key(arg);
        let mint = if is_fq_call(k) {
            let Some(c) = self.cur() else { return };
            // the fq call itself is counted when the argument is visited
            format!("call{}", c.fq_calls + 1)
        } else if let Some(n) = single_path(k) {
            match self.cur().and_then(|c| c.fq_locals.get(&n)) {
                Some(id) => id.clone(),
                None => return,
            }
        } else {
            return;
        };
        if let Some(c) = self.cur() {
            c.reads.push((store.to_string(), op.to_string(), mint));
        }
    }

    fn mark_place(&mut self, e: &Expr) {
        match e {
            Expr::Field(f) => {
                self.write_nodes.insert(f as *const _ as usize);
                self.mark_place(&f.base);
            }
            Expr::Index(i) => self.mark_place(&i.expr),
            Expr::Paren(p) => self.mark_place(&p.expr),
            Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => self.mark_place(&u.expr),
            Expr::MethodCall(m)
                if [
                    "borrow_mut",
                    "get_mut",
                    "last_mut",
                    "first_mut",
                    "iter_mut",
                    "entry",
                    "as_mut",
                    "values_mut",
                ]
                .contains(&m.method.to_string().as_str()) =>
            {
                self.mark_place(&m.receiver)
            }
            _ => {}
        }
    }

    fn visit_macro_args(&mut self, mac: &syn::Macro) {
        if mac.path.is_ident("matches") {
            let parser = |input: ParseStream| -> syn::Result<(Expr, Option<Expr>)> {
                let e: Expr = input.parse()?;
                input.parse::<Token![,]>()?;
                let _ = Pat::parse_multi_with_leading_vert(input)?;
                let g = if input.peek(Token![if]) {
                    input.parse::<Token![if]>()?;
                    Some(input.parse::<Expr>()?)
                } else {
                    None
                };
                let _ = input.parse::<Option<Token![,]>>()?;
                Ok((e, g))
            };
            if let Ok((e, g)) = parser.parse2(mac.tokens.clone()) {
                self.visit_expr(&e);
                if let Some(g) = g {
                    self.visit_expr(&g);
                }
                return;
            }
        }
        if let Ok(list) = Punctuated::<Expr, Token![,]>::parse_terminated.parse2(mac.tokens.clone())
        {
            for e in &list {
                self.visit_expr(e);
            }
            return;
        }
        // census_g3: the first expression only
        let first = |input: ParseStream| -> syn::Result<Expr> {
            let e: Expr = input.parse()?;
            let _: proc_macro2::TokenStream = input.parse()?;
            Ok(e)
        };
        if let Ok(e) = first.parse2(mac.tokens.clone()) {
            self.visit_expr(&e);
        }
    }
}

impl<'ast> Visit<'ast> for Pass2<'_> {
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
        if !has_cfg_test(&m.attrs) {
            visit::visit_item_mod(self, m);
        }
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let Some(init) = &l.init {
            let mut names = Vec::new();
            pat_names(&l.pat, &mut names);
            let single = matches!(&l.pat, Pat::Ident(_) | Pat::Type(_)) && names.len() == 1;
            if let Some(c) = self.cur() {
                if single && init.diverge.is_none() && is_fq_call(&init.expr) {
                    // the initializer's fq call is the next one counted
                    let id = format!("let:{}@{}", names[0], c.fq_calls + 1);
                    c.fq_locals.insert(names[0].clone(), id);
                } else {
                    // a rebinding of the name holds no fq key
                    for n in &names {
                        c.fq_locals.remove(n);
                    }
                }
            }
        }
        visit::visit_local(self, l);
    }
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        let meth = m.method.to_string();
        if HD_MUT.contains(&meth.as_str()) {
            self.mark_place(&m.receiver);
        }
        // HM: `<recv>.cur_prefix.clone()` (the receiver the field itself)
        if meth == "clone"
            && matches!(&*m.receiver, Expr::Field(f) if matches!(&f.member, syn::Member::Named(i) if i == "cur_prefix"))
        {
            self.mint("curp", "cur_prefix.clone()");
        }
        self.visit_expr(&m.receiver);
        self.called(&meth);
        if meth == "fq" {
            if let Some(c) = self.cur() {
                c.fq_calls += 1;
            }
        }
        // HM: join(".") and the accumulator pushes
        if meth == "join"
            && m.args
                .first()
                .and_then(lit_arg)
                .is_some_and(|(t, _)| t == "\".\"")
        {
            self.mint("join", "join");
        }
        if meth == "push" || meth == "push_str" {
            let dot = meth == "push"
                && m.args
                    .first()
                    .and_then(lit_arg)
                    .is_some_and(|(t, _)| t == "'.'" || t == "'$'");
            let simple = self.cur().map(|c| c.simple.clone()).unwrap_or_default();
            if dot || (meth == "push_str" && MINT_PUSH_STR_FNS.contains(&simple.as_str())) {
                self.mint("push", &meth);
            }
        }
        // HS
        if SCAN_METHODS.contains(&meth.as_str()) {
            if let Some((base, f)) = field_of2(&m.receiver) {
                if is_recv2(base) {
                    if let Some(c) = self.cur() {
                        c.scans.push((f, meth.clone()));
                    }
                }
            }
        }
        // HH
        if HH_READ.contains(&meth.as_str()) {
            // a field of any receiver, or a fn-local table
            let store = field_of2(&m.receiver).map(|(_, f)| f).or_else(|| {
                single_path(strip_key(&m.receiver)).filter(|n| !RECV2.contains(&n.as_str()))
            });
            if let (Some(store), Some(a)) = (store, m.args.first()) {
                self.key_read(&store, &meth, a);
            }
        }
        // HSEG
        if SEG_METHODS.contains(&meth.as_str()) {
            if let Some(a) = m.args.first() {
                let row = match lit_arg(a) {
                    Some((t, v)) => {
                        (v.contains(['.', '$', '[']) || v.contains("::") || meth == "strip_prefix")
                            .then_some(t)
                    }
                    None => SEG_DYN
                        .contains(&meth.as_str())
                        .then(|| format!("<{}>", red(a))),
                };
                if let (Some(t), Some(c)) = (row, self.cur()) {
                    c.segs.push((meth.clone(), t));
                }
            }
        }
        for a in &m.args {
            self.visit_expr(a);
        }
    }
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let Expr::Path(p) = &*c.func {
            if let Some(last) = p.path.segments.last() {
                let n = last.ident.to_string();
                self.called(&n);
                if n == "fq" {
                    if let Some(r) = self.cur() {
                        r.fq_calls += 1;
                    }
                }
            }
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_expr_index(&mut self, i: &'ast syn::ExprIndex) {
        if let Some((_, store)) = field_of2(&i.expr) {
            self.key_read(&store, "index", &i.index);
        }
        visit::visit_expr_index(self, i);
    }
    fn visit_expr_for_loop(&mut self, fl: &'ast syn::ExprForLoop) {
        if let Some((base, f)) = field_of2(&fl.expr) {
            if is_recv2(base) {
                if let Some(c) = self.cur() {
                    c.scans.push((f, "for".to_string()));
                }
            }
        }
        visit::visit_expr_for_loop(self, fl);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let n = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if n == "format" || n == "write" {
            let lit = mac.tokens.clone().into_iter().find_map(|t| match t {
                proc_macro2::TokenTree::Literal(x) => {
                    let s = x.to_string();
                    (s.starts_with('"') || s.starts_with("r\"") || s.starts_with("r#")).then_some(s)
                }
                _ => None,
            });
            if let Some(kind) = lit.as_deref().and_then(glue_kind) {
                self.mint(kind, &n);
            }
        }
        self.visit_macro_args(mac);
    }
    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        self.mark_place(&a.left);
        visit::visit_expr_assign(self, a);
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        if matches!(
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
        ) {
            self.mark_place(&b.left);
        }
        visit::visit_expr_binary(self, b);
    }
    fn visit_expr_reference(&mut self, r: &'ast syn::ExprReference) {
        if r.mutability.is_some() {
            self.mark_place(&r.expr);
        }
        visit::visit_expr_reference(self, r);
    }
    fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
        if let syn::Member::Named(i) = &f.member {
            let n = i.to_string();
            if DISPLAY_FIELDS.contains(&n.as_str()) {
                let acc = if self.write_nodes.contains(&(f as *const _ as usize)) {
                    "write"
                } else {
                    "read"
                };
                if let Some(c) = self.cur() {
                    c.disp.push((n, acc.to_string()));
                }
            }
        }
        visit::visit_expr_field(self, f);
    }
    fn visit_expr_struct(&mut self, s: &'ast syn::ExprStruct) {
        for fv in &s.fields {
            if let syn::Member::Named(i) = &fv.member {
                let n = i.to_string();
                if DISPLAY_FIELDS.contains(&n.as_str()) {
                    if let Some(c) = self.cur() {
                        c.disp.push((n, "init".to_string()));
                    }
                }
            }
        }
        visit::visit_expr_struct(self, s);
    }
}

/// A struct the crate declares: its file and fields (name, type).
struct StructInfo {
    file: String,
    fields: Vec<(String, syn::Type)>,
}

/// census_fields.rs's struct collector: every struct outside a `#[cfg(test)]` mod, and the
/// type aliases.
struct StructV<'a> {
    file: String,
    out: &'a mut BTreeMap<String, Vec<StructInfo>>,
    aliases: &'a mut BTreeMap<String, syn::Type>,
}

impl<'ast> Visit<'ast> for StructV<'_> {
    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        if !has_cfg_test(&m.attrs) {
            visit::visit_item_mod(self, m);
        }
    }
    fn visit_item_type(&mut self, t: &'ast syn::ItemType) {
        self.aliases.insert(t.ident.to_string(), (*t.ty).clone());
    }
    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        if has_cfg_test(&s.attrs) {
            return;
        }
        let fields = s
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let n = f
                    .ident
                    .as_ref()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| i.to_string());
                (n, f.ty.clone())
            })
            .collect();
        self.out
            .entry(s.ident.to_string())
            .or_default()
            .push(StructInfo {
                file: self.file.clone(),
                fields,
            });
    }
}

/// census_fields.rs's type text.
fn type_str(t: &syn::Type) -> String {
    match t {
        syn::Type::Path(p) => p
            .path
            .segments
            .iter()
            .map(|seg| {
                let mut s = seg.ident.to_string();
                if let syn::PathArguments::AngleBracketed(a) = &seg.arguments {
                    let args: Vec<String> = a
                        .args
                        .iter()
                        .filter_map(|g| match g {
                            syn::GenericArgument::Type(t) => Some(type_str(t)),
                            syn::GenericArgument::Lifetime(l) => Some(format!("'{}", l.ident)),
                            _ => None,
                        })
                        .collect();
                    write!(s, "<{}>", args.join(", ")).expect("write");
                }
                s
            })
            .collect::<Vec<_>>()
            .join("::"),
        syn::Type::Reference(r) => format!(
            "&{}{}",
            if r.mutability.is_some() { "mut " } else { "" },
            type_str(&r.elem)
        ),
        syn::Type::Slice(r) => format!("[{}]", type_str(&r.elem)),
        syn::Type::Array(r) => format!("[{}; N]", type_str(&r.elem)),
        syn::Type::Tuple(tt) => format!(
            "({})",
            tt.elems.iter().map(type_str).collect::<Vec<_>>().join(", ")
        ),
        syn::Type::Paren(r) => type_str(&r.elem),
        _ => "?".into(),
    }
}

/// The second pass over the crate, and the struct reach.
struct Census2 {
    fns: Vec<Fn2>,
    structs: BTreeMap<String, Vec<StructInfo>>,
    aliases: BTreeMap<String, syn::Type>,
}

fn run_census2() -> Census2 {
    let srcs = load_src();
    let mut fns = Vec::new();
    let mut structs = BTreeMap::new();
    let mut aliases = BTreeMap::new();
    for s in &srcs {
        let mut recs = Vec::new();
        Pass2 {
            file: s.rel.clone(),
            stack: Vec::new(),
            out: &mut recs,
            write_nodes: BTreeSet::new(),
        }
        .visit_file(&s.file);
        StructV {
            file: s.rel.clone(),
            out: &mut structs,
            aliases: &mut aliases,
        }
        .visit_file(&s.file);
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
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for r in &mut recs {
            let n = seen.entry(r.qual.clone()).or_default();
            *n += 1;
            if *n > 1 {
                r.qual = format!("{}#{}", r.qual, n);
            }
        }
        fns.extend(recs);
    }
    Census2 {
        fns,
        structs,
        aliases,
    }
}

impl Census2 {
    /// A type's idents with the crate's type aliases expanded (census_fields.rs).
    fn expand(&self, ty: &syn::Type) -> Vec<String> {
        let mut ids = type_idents(ty);
        let mut k = 0;
        while k < ids.len() && ids.len() < 400 {
            if let Some(t) = self.aliases.get(&ids[k]) {
                ids.extend(type_idents(t));
            }
            k += 1;
        }
        ids
    }

    /// The structs reached from `Elaborator` and `ConstFrame` through field types, in
    /// census_fields.rs's order.
    fn reached(&self) -> Vec<String> {
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        let mut queue = vec!["ConstFrame".to_string(), "Elaborator".to_string()];
        while let Some(sn) = queue.pop() {
            if !seen.insert(sn.clone()) {
                continue;
            }
            order.push(sn.clone());
            for si in self.structs.get(&sn).map(Vec::as_slice).unwrap_or(&[]) {
                for (_, ty) in &si.fields {
                    for id in self.expand(ty) {
                        if self.structs.contains_key(&id) && !seen.contains(&id) {
                            queue.push(id);
                        }
                    }
                }
            }
        }
        order
    }

    /// HS's store set: the `Elaborator` fields whose type is a map / set keyed by `String`.
    fn keyed_stores(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for si in self
            .structs
            .get("Elaborator")
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            for (n, ty) in &si.fields {
                if string_keyed(ty) {
                    out.insert(n.clone());
                }
            }
        }
        out
    }

    fn has_fn(&self, file: &str, simple: &str) -> bool {
        self.fns
            .iter()
            .any(|r| r.file == file && r.simple == simple)
    }
}

/// A map / set type (through `Option`, `RefCell`, `Rc`, `Box`, `Bound`) whose first type
/// argument is `String`.
fn string_keyed(ty: &syn::Type) -> bool {
    let syn::Type::Path(p) = ty else { return false };
    let Some(last) = p.path.segments.last() else {
        return false;
    };
    let syn::PathArguments::AngleBracketed(a) = &last.arguments else {
        return false;
    };
    let first = a.args.iter().find_map(|g| match g {
        syn::GenericArgument::Type(t) => Some(t),
        _ => None,
    });
    let Some(first) = first else { return false };
    let id = last.ident.to_string();
    if KEYED_COLL.contains(&id.as_str()) {
        return matches!(first, syn::Type::Path(fp) if fp.path.is_ident("String"));
    }
    matches!(id.as_str(), "Option" | "RefCell" | "Rc" | "Box" | "Bound") && string_keyed(first)
}

/// A pinned hand column `a=2+b=1` (or one word for every site of the row): its words, each
/// counted, checked against the row's count.
fn hand_words(col: &str, n: usize) -> Result<Vec<(String, usize)>, String> {
    if !col.contains('=') {
        return Ok(vec![(col.to_string(), n)]);
    }
    let mut out = Vec::new();
    for part in col.split('+') {
        let Some((w, k)) = part.split_once('=') else {
            return Err(format!("`{part}` has no `=<count>`"));
        };
        let k: usize = k.parse().map_err(|_| format!("`{part}`: count"))?;
        out.push((w.to_string(), k));
    }
    let sum: usize = out.iter().map(|(_, k)| k).sum();
    if sum != n {
        return Err(format!("the words count {sum}, the row {n}"));
    }
    Ok(out)
}

fn dbg_counts(m: &BTreeMap<String, usize>) -> String {
    format!("{m:?}")
}

// =====================================================================================
// HM the mints of scope-qualified text
// =====================================================================================

fn hm_rows(c: &Census2) -> Vec<(String, String, usize)> {
    let mut agg: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for r in &c.fns {
        for (kind, name) in &r.mints {
            let e = agg
                .entry(format!("{}\t{}\t{kind}\t{name}", r.file, r.qual))
                .or_insert((0, r.line));
            e.0 += 1;
        }
    }
    agg.into_iter()
        .map(|(k, (n, line))| (k, n.to_string(), line))
        .collect()
}

#[test]
fn hm_the_mints_of_scope_text_are_pinned() {
    // the pass sees a mint inside a `matches!` guard (strings.rs:139, the row census_g3
    // missed), a glue literal, a `join(".")`, a `cur_prefix.clone()` and a temp name
    let probe = syn::parse_file(
        "impl E { fn m(&self, b: &X) -> bool { let a = format!(\"{}.{}\", self.cur_prefix, 1); \
         let t = format!(\"$ia_tmp${}\", 2); let p = self.cur_prefix.clone(); let j = v.join(\".\"); \
         matches!(&b.kind, K::I(p) if self.walk_scopes_key_shadowed(&p.n, |k| self.s.contains_key(k)).is_some()) } }",
    )
    .expect("probe parses");
    let mut recs = Vec::new();
    Pass2 {
        file: "probe".into(),
        stack: Vec::new(),
        out: &mut recs,
        write_nodes: BTreeSet::new(),
    }
    .visit_file(&probe);
    assert_eq!(
        recs[0].mints,
        [
            ("fmt".to_string(), "format".to_string()),
            ("fmt-tempname".into(), "format".into()),
            ("curp".into(), "cur_prefix.clone()".into()),
            ("join".into(), "join".into()),
            ("call".into(), "walk_scopes_key_shadowed".into()),
        ],
        "HM's pass no longer reads a glue literal, a temp name, a clone, a join or a `matches!` guard"
    );

    let c = run_census2();
    let rows = hm_rows(&c);
    let mut got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, v, _)| (k.clone(), v.clone()))
        .collect();
    let dumped: Vec<String> = rows
        .iter()
        .map(|(k, v, l)| format!("{k}\t{l}\t{v}"))
        .collect();
    dump("HM", "file\tfn\tkind\tname\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut msgs = Vec::new();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_tag: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for row in table(&tables::get("HM")) {
        assert_eq!(
            row.len(),
            6,
            "HM table row has {} columns, not 6: {row:?}",
            row.len()
        );
        let k = row[..4].join("\t");
        let n: usize = row[4].parse().expect("count");
        if row[2] == "derived-fn" {
            // HAND: a fn returning text its body mints (G3's compile probe named it ; no
            // syntax sees "returns key / prefix text")
            if !c.has_fn(&row[0], &row[3]) {
                msgs.push(format!(
                    "HM HAND ROW GONE {} — the derived mint fn is not in the file: name the unit, remove the row",
                    k.replace('\t', " · ")
                ));
            }
            got.insert(k.clone(), n.to_string());
        }
        match hand_words(&row[5], n) {
            Ok(ws) => {
                for (w, k) in ws {
                    *by_tag.entry(w).or_default() += k;
                }
            }
            Err(e) => msgs.push(format!(
                "HM TAG {} [{}]: {e}",
                k.replace('\t', " · "),
                row[5]
            )),
        }
        *by_kind.entry(row[2].clone()).or_default() += n;
        total += n;
        want.insert(k, n.to_string());
    }
    msgs.extend(diff_rows(
        "HM",
        &got,
        &want,
        "a new mint of scope-qualified text (or a mint moved fn): name the unit that adds it (Q-DECLKEY DK1 takes \
         every mint into lexscope), read what the text is and tag it (key / scope / segment / display / spelling), \
         add the row",
        "a mint is gone: name the unit, remove the row",
    ));
    let counts = format!(
        "mints {total} kinds {} tags {}",
        dbg_counts(&by_kind),
        dbg_counts(&by_tag)
    );
    if counts != tables::get("HM_COUNTS") {
        msgs.push(format!(
            "HM COUNTS table [{}] now [{counts}]",
            tables::get("HM_COUNTS")
        ));
    }
    finish("HM", &msgs);
}

// =====================================================================================
// HS the key scans
// =====================================================================================

#[test]
fn hs_the_key_scans_are_pinned() {
    let c = run_census2();
    // the store set: Elaborator fields keyed by `String`, pinned by name
    let stores = c.keyed_stores();
    let pinned: BTreeSet<String> = tables::get("HS_STORES")
        .split_whitespace()
        .map(str::to_string)
        .collect();
    dump(
        "HS_STORES",
        "store",
        &stores.iter().cloned().collect::<Vec<_>>(),
    );
    let mut msgs = Vec::new();
    for s in stores.difference(&pinned) {
        msgs.push(format!(
            "HS NEW STORE {s} — an Elaborator field keyed by String: name the unit, add it to HS_STORES (its scans \
             join the census)"
        ));
    }
    for s in pinned.difference(&stores) {
        msgs.push(format!(
            "HS GONE STORE {s} — no longer an Elaborator field keyed by String: name the unit (a retype to a key \
             record is LS3 / DK1's), remove it from HS_STORES"
        ));
    }
    let mut agg: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for r in &c.fns {
        for (f, m) in &r.scans {
            if stores.contains(f) {
                let e = agg
                    .entry(format!("{}\t{}\t{f}\t{m}", r.file, r.qual))
                    .or_insert((0, r.line));
                e.0 += 1;
            }
        }
    }
    let mut got: BTreeMap<String, String> = agg
        .iter()
        .map(|(k, (n, _))| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = agg
        .iter()
        .map(|(k, (n, l))| format!("{k}\t{l}\t{n}"))
        .collect();
    dump("HS", "file\tfn\tstore\tmethod\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for row in table(&tables::get("HS")) {
        assert_eq!(
            row.len(),
            7,
            "HS table row has {} columns, not 7: {row:?}",
            row.len()
        );
        let k = row[..4].join("\t");
        let n: usize = row[4].parse().expect("count");
        if row[5].starts_with("HAND:") {
            // a scan the store set does not hold (a Vec field, a parameter): the fn must
            // still call the method on that receiver
            let found = c.fns.iter().any(|r| {
                r.file == row[0]
                    && r.qual == row[1]
                    && (r.scans.iter().any(|(f, m)| f == &row[2] && m == &row[3])
                        || r.calls.contains(&row[3]))
            });
            if !found {
                msgs.push(format!(
                    "HS HAND ROW GONE {} — re-read the fn: the scan moved or left, fix or remove the row",
                    k.replace('\t', " · ")
                ));
            }
            got.insert(k.clone(), n.to_string());
        }
        match hand_words(row[5].trim_start_matches("HAND:"), n) {
            Ok(ws) => {
                for (w, k) in ws {
                    *by_kind.entry(w).or_default() += k;
                }
            }
            Err(e) => msgs.push(format!(
                "HS KIND {} [{}]: {e}",
                k.replace('\t', " · "),
                row[5]
            )),
        }
        total += n;
        want.insert(k, n.to_string());
    }
    msgs.extend(diff_rows(
        "HS",
        &got,
        &want,
        "a new iteration over a String-keyed store: name the unit that adds it, read whether it reads the key text \
         (TEXT-SCAN-ANSWERS / PRINT/DUMP / KEY-BLIND) and its lexscope entry (g4/c/entries.tsv's SCAN rows), add the row",
        "a key scan is gone: name the unit, remove the row",
    ));
    let counts = format!(
        "stores {} scans {total} kinds {}",
        pinned.len(),
        dbg_counts(&by_kind)
    );
    if counts != tables::get("HS_COUNTS") {
        msgs.push(format!(
            "HS COUNTS table [{}] now [{counts}]",
            tables::get("HS_COUNTS")
        ));
    }
    finish("HS", &msgs);
}

// =====================================================================================
// HH the current-prefix reads
// =====================================================================================

#[test]
fn hh_the_current_prefix_reads_are_pinned() {
    // the reader: an fq key as the argument, through a `let`, and an index
    let probe = syn::parse_file(
        "impl E { fn r(&self, n: &str) { let key = self.fq(n); let _ = self.symbols.get(&key); \
         let _ = self.params.contains_key(&self.fq(n).clone()); let _ = &self.wide[&key]; \
         let _ = self.walk_scopes_key(n, |k| self.symbols.contains_key(k)); } }",
    )
    .expect("probe parses");
    let mut recs = Vec::new();
    Pass2 {
        file: "probe".into(),
        stack: Vec::new(),
        out: &mut recs,
        write_nodes: BTreeSet::new(),
    }
    .visit_file(&probe);
    assert_eq!(
        recs[0].reads,
        [
            (
                "symbols".to_string(),
                "get".to_string(),
                "let:key@1".to_string()
            ),
            ("params".into(), "contains_key".into(), "call2".into()),
            ("wide".into(), "index".into(), "let:key@1".into()),
        ],
        "HH's reader no longer links an fq key through a `let`, a direct argument or an index"
    );

    let c = run_census2();
    let mut agg: BTreeMap<String, (BTreeMap<String, usize>, usize)> = BTreeMap::new();
    for r in &c.fns {
        let walks = HH_WALK.iter().any(|w| r.calls.contains(*w));
        for (store, op, mint) in &r.reads {
            let others = r.reads.iter().any(|(s, _, m)| m == mint && s != store);
            let role = if walks || others {
                "composite"
            } else {
                "whole"
            };
            let e = agg
                .entry(format!("{}\t{}\t{store}\t{role}", r.file, r.qual))
                .or_insert((BTreeMap::new(), r.line));
            *e.0.entry(op.clone()).or_default() += 1;
        }
    }
    let fmt_ops = |m: &BTreeMap<String, usize>| {
        m.iter()
            .map(|(o, n)| format!("{o}={n}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let got: BTreeMap<String, String> = agg
        .iter()
        .map(|(k, (ops, _))| (k.clone(), fmt_ops(ops)))
        .collect();
    let dumped: Vec<String> = agg
        .iter()
        .map(|(k, (ops, l))| format!("{k}\t{l}\t{}", fmt_ops(ops)))
        .collect();
    dump("HH", "file\tfn\tstore\trole\tline\tops", &dumped);
    let mut want = BTreeMap::new();
    let mut by_role: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_store: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(&tables::get("HH")) {
        assert_eq!(
            row.len(),
            5,
            "HH table row has {} columns, not 5: {row:?}",
            row.len()
        );
        let n: usize = row[4]
            .split(',')
            .map(|p| p.split_once('=').map_or(0, |(_, k)| k.parse().unwrap_or(0)))
            .sum();
        *by_role.entry(row[3].clone()).or_default() += n;
        *by_store.entry(row[2].clone()).or_default() += n;
        want.insert(row[..4].join("\t"), row[4].clone());
    }
    let mut msgs = diff_rows(
        "HH",
        &got,
        &want,
        "a new read of a store by a current-prefix key (`fq`, no walk): name the unit that adds it — this table PINS \
         today's sites and does not close the door (premise2 B3: a key entry called with the current scope is an \
         fq this rule does not see) — and add the row",
        "a current-prefix read is gone: name the unit, remove the row",
    );
    let total: usize = by_role.values().sum();
    let counts = format!(
        "reads {total} roles {} stores {}",
        dbg_counts(&by_role),
        dbg_counts(&by_store)
    );
    if counts != tables::get("HH_COUNTS") {
        msgs.push(format!(
            "HH COUNTS table [{}] now [{counts}]",
            tables::get("HH_COUNTS")
        ));
    }
    finish("HH", &msgs);
}

// =====================================================================================
// HSEG the segment-text tests
// =====================================================================================

#[test]
fn hseg_the_segment_text_tests_are_pinned() {
    let c = run_census2();
    let mut agg: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for r in &c.fns {
        for (m, a) in &r.segs {
            let e = agg
                .entry(format!("{}\t{}\t{m}\t{a}", r.file, r.qual))
                .or_insert((0, r.line));
            e.0 += 1;
        }
    }
    let got: BTreeMap<String, String> = agg
        .iter()
        .map(|(k, (n, _))| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = agg
        .iter()
        .map(|(k, (n, l))| format!("{k}\t{l}\t{n}"))
        .collect();
    dump("HSEG", "file\tfn\tmethod\targ\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut msgs = Vec::new();
    let mut by_tag: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_unit: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;
    for row in table(&tables::get("HSEG")) {
        assert_eq!(
            row.len(),
            7,
            "HSEG table row has {} columns, not 7: {row:?}",
            row.len()
        );
        let k = row[..4].join("\t");
        let n: usize = row[4].parse().expect("count");
        for (col, map) in [(&row[5], &mut by_tag), (&row[6], &mut by_unit)] {
            match hand_words(col, n) {
                Ok(ws) => {
                    for (w, k) in ws {
                        *map.entry(w).or_default() += k;
                    }
                }
                Err(e) => msgs.push(format!("HSEG {} [{col}]: {e}", k.replace('\t', " · "))),
            }
        }
        total += n;
        want.insert(k, n.to_string());
    }
    msgs.extend(diff_rows(
        "HSEG",
        &got,
        &want,
        "a new test of segment text (`.` / `$` / `[` / `::` in a cut or a test, a non-literal prefix test, \
         is_gen_scope_segment): name the unit that adds it, read whether it tests a scope path, a key, a spelling or \
         display text, give it the unit that owns it (walk / LS4 / DK3 / DK4 / RS1 / none), add the row",
        "a segment-text test is gone: name the unit, remove the row",
    ));
    let counts = format!(
        "tests {total} tags {} units {}",
        dbg_counts(&by_tag),
        dbg_counts(&by_unit)
    );
    if counts != tables::get("HSEG_COUNTS") {
        msgs.push(format!(
            "HSEG COUNTS table [{}] now [{counts}]",
            tables::get("HSEG_COUNTS")
        ));
    }
    finish("HSEG", &msgs);
}

// =====================================================================================
// H5 the name-typed fields
// =====================================================================================

#[test]
fn h5_the_name_typed_fields_are_classified() {
    let c = run_census2();
    let mut got: BTreeMap<String, String> = BTreeMap::new();
    let mut dumped = Vec::new();
    for sn in c.reached() {
        for si in c.structs.get(&sn).map(Vec::as_slice).unwrap_or(&[]) {
            for (f, ty) in &si.fields {
                let ids = c.expand(ty);
                if ids.iter().any(|i| NAME_TYPES.contains(&i.as_str())) {
                    let k = format!("{sn}\t{}\t{f}", si.file);
                    let t = type_str(ty);
                    dumped.push(format!("{k}\t{t}"));
                    got.insert(k, t);
                }
            }
        }
    }
    dump("H5", "struct\tfile\tfield\ttype", &dumped);
    let mut want = BTreeMap::new();
    let mut msgs = Vec::new();
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_store: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(&tables::get("H5")) {
        assert_eq!(
            row.len(),
            7,
            "H5 table row has {} columns, not 7: {row:?}",
            row.len()
        );
        let k = row[..3].join("\t");
        if row[4].is_empty() || row[4] == "-" || row[5].is_empty() || row[5] == "-" {
            msgs.push(format!(
                "H5 UNCLASSIFIED {} — give it a class (G1: BINDING / NOT) and its code by reading",
                k.replace('\t', " · ")
            ));
        }
        *by_class
            .entry(format!("{}:{}", row[4], row[5]))
            .or_default() += 1;
        *by_store.entry(row[6].clone()).or_default() += 1;
        want.insert(k, row[3].clone());
    }
    msgs.extend(diff_rows(
        "H5",
        &got,
        &want,
        "a new name-typed field (a String / str / Symbol / Ident in its type, on a struct the Elaborator reaches): \
         name the unit that adds it, classify it (BINDING code V R T S, or NOT code D CTX I INPUT COPY DEF G CHK X) \
         and give it its store class (g3 fields_keyed.tsv / G4: KEYED-*, ROUTINE, SCOPE, DISPLAY, OUTPUT, KEY, -), \
         add the row — no unclassified name-typed field",
        "a name-typed field is gone (or no longer name-typed: a retype to a key / scope record names its unit): \
         remove the row",
    ));
    let counts = format!(
        "fields {} classes {} stores {}",
        want.len(),
        dbg_counts(&by_class),
        dbg_counts(&by_store)
    );
    if counts != tables::get("H5_COUNTS") {
        msgs.push(format!(
            "H5 COUNTS table [{}] now [{counts}]",
            tables::get("H5_COUNTS")
        ));
    }
    finish("H5", &msgs);
}

// =====================================================================================
// HD the display-chain fields' writers and readers
// =====================================================================================

#[test]
fn hd_the_display_chain_accesses_are_pinned() {
    let c = run_census2();
    let mut agg: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for r in &c.fns {
        for (f, a) in &r.disp {
            let e = agg
                .entry(format!("{}\t{}\t{f}\t{a}", r.file, r.qual))
                .or_insert((0, r.line));
            e.0 += 1;
        }
    }
    let got: BTreeMap<String, String> = agg
        .iter()
        .map(|(k, (n, _))| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = agg
        .iter()
        .map(|(k, (n, l))| format!("{k}\t{l}\t{n}"))
        .collect();
    dump("HD", "file\tfn\tfield\taccess\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(&tables::get("HD")) {
        assert_eq!(
            row.len(),
            5,
            "HD table row has {} columns, not 5: {row:?}",
            row.len()
        );
        let n: usize = row[4].parse().expect("count");
        *by.entry(format!("{}:{}", row[2], row[3])).or_default() += n;
        want.insert(row[..4].join("\t"), n.to_string());
    }
    let mut msgs = diff_rows(
        "HD",
        &got,
        &want,
        "a new access of a display-chain field (block_scope, block_scope_root, cur_class_method): name the unit — \
         LS2-d makes the three private to lexscope ; a reader that feeds a store key or a compare is PM20's \
         falsifier — add the row",
        "a display-chain access is gone: name the unit, remove the row",
    );
    let counts = dbg_counts(&by);
    if counts != tables::get("HD_COUNTS") {
        msgs.push(format!(
            "HD COUNTS table [{}] now [{counts}]",
            tables::get("HD_COUNTS")
        ));
    }
    finish("HD", &msgs);
}

// =====================================================================================
// HK the producers' scope kinds
// =====================================================================================

#[test]
fn hk_the_scope_entry_kinds_are_pinned() {
    let c = run_census();
    let c2 = run_census2();
    let mut got: BTreeMap<String, String> = BTreeMap::new();
    let mut dumped = Vec::new();
    // the 25 writes of cur_prefix (H3's sites, per fn)
    for (k, v, l) in h3_rows(&c) {
        let sites = v.split('\t').nth(1).unwrap_or("0").to_string();
        dumped.push(format!("{k}\tcur_prefix\t{l}\t{sites}"));
        got.insert(format!("{k}\tcur_prefix"), sites);
    }
    // the callers of with_scope / elaborate_instance
    let mut agg: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for r in &c2.fns {
        for e in &r.entries {
            let x = agg
                .entry(format!("{}\t{}\t{e}", r.file, r.qual))
                .or_insert((0, r.line));
            x.0 += 1;
        }
    }
    for (k, (n, l)) in &agg {
        dumped.push(format!("{k}\t{l}\t{n}"));
        got.insert(k.clone(), n.to_string());
    }
    dump("HK", "file\tfn\twhat\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut msgs = Vec::new();
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(&tables::get("HK")) {
        assert_eq!(
            row.len(),
            5,
            "HK table row has {} columns, not 5: {row:?}",
            row.len()
        );
        let k = row[..3].join("\t");
        let n: usize = row[3].parse().expect("count");
        match hand_words(&row[4], n) {
            Ok(ws) => {
                for (w, k) in ws {
                    *by.entry(format!("{}:{w}", row[2])).or_default() += k;
                }
            }
            Err(e) => msgs.push(format!(
                "HK KIND {} [{}]: {e}",
                k.replace('\t', " · "),
                row[4]
            )),
        }
        want.insert(k, n.to_string());
    }
    msgs.extend(diff_rows(
        "HK",
        &got,
        &want,
        "a new scope entry (a write of cur_prefix, a with_scope or elaborate_instance call): name the unit, read the \
         kind its PRODUCER knows (G4 OWED-k ; DELTA6 F2's 13 kinds ; LS2 passes it to `enter`), add the row",
        "a scope entry is gone: name the unit, remove the row",
    ));
    let counts = dbg_counts(&by);
    if counts != tables::get("HK_COUNTS") {
        msgs.push(format!(
            "HK COUNTS table [{}] now [{counts}]",
            tables::get("HK_COUNTS")
        ));
    }
    finish("HK", &msgs);
}

// =====================================================================================
// The pinned tables
// =====================================================================================

mod tables {
    //! The tables live in ONE data file beside this test, `scope_home.tables` (moved out of
    //! this file in LS1 part 2, when it passed ~4500 lines): a section per table.
    const DATA: &str = include_str!("scope_home.tables");

    /// The section `name`, its `#` note lines left out.
    pub fn get(name: &str) -> String {
        let open = format!("== {name}");
        let mut lines = DATA.lines().skip_while(|l| l.trim_end() != open);
        assert!(
            lines.next().is_some(),
            "scope_home.tables has no section `{name}`"
        );
        lines
            .take_while(|l| !l.starts_with("== "))
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }
}
