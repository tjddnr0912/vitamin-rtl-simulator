//! The scope censuses of §5.2 Q-LEXSCOPE (unit LS1, parts 1 to 4), as home tests: fourteen
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
//!   `walk_rtn_scopes`, 10 `hier_resolve`, 1 `decl_probe_key`. Since §5.2 Q-DECLKEY DK1-4b-2 a
//!   local closure that reads a walk's FOUND key (a `DeclKey`: `found_key_closure`) is a FOUND
//!   row of its own (entry column `<name>()`), so the stores of each found-key copy of a
//!   closure pair (E1.4: edge_gate.rs `other_kind`, held.rs `refused`) are pinned (R-H8 (g)).
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
//!   UNIT (walk, DK1, DK3, B3a, B3k, RS1, `-` ; LS2 / LS4 / DK4 re-tagged by §5.2 Q-DECLKEY
//!   DK1-0, DELTA14 D1, R-F3) are hand columns.
//! - H5 the name-typed fields (215) with g1's CLASS / CODE and a STORE column (hand).
//! - HD the display-chain fields' accesses (`block_scope`, `block_scope_root`,
//!   `cur_class_method`): read / write / init.
//! - HK the producers' scope kinds: per fn the `cur_prefix` writes (H3's 25 sites), the
//!   `with_scope` calls and the `elaborate_instance` calls, KIND a hand column.
//!
//! Part 3 (a third walk, `HfWalk`, with positions as statement paths, and a path pass,
//! `HfFlow`, over each opener's block):
//!
//! - HF the shape of each `cur_prefix` write site (H3's 25, plan8 H3 / H5): per site the
//!   form, the saved binding, the ARG tokens, the opener's block and statement, each
//!   restore's and re-point's position, whether every restore sits in the opener's block,
//!   the exits between the opener and the last restore (leaving the region or local),
//!   every path's restore count where it leaves (the end of the block, a `return`, a `?`,
//!   a `break` / `continue` out of it), and the calls in between of fns that write the
//!   prefix or of a parameter / local ; PLAN8's class and a READ column are hand columns.
//!   HF_CALLS pins the `with_scope` / `elaborate_instance` calls by their scope argument.
//!   HF does not see a panic other than the four diverging macros, nor a write through a
//!   `&mut String` bound earlier.
//!
//! Part 4 (a fourth walk, `HoutW`, that carries each value's consumer down the tree ;
//! HT-SURF, the module-surface test of the same part, is `scope_surface.rs`):
//!
//! - HOUT where display / child-prefix text goes (DELTA9 J4, DELTA10 L5): every call of the
//!   display fns, `child_prefix` and `enter_frame_scope_root`, the `ProcIdent` /
//!   `InstanceInfo` / `CovgInstMeta` literals, the 9 saves and 9 restores of the three
//!   display fields, push_func's `m_path` producers and the `reported` set's lines, by
//!   (file, fn, what, arg, SINK) — SINK the first consumer above the value. It is HC's seed
//!   and LS2-d's prediction. It does not see a value carried through a helper fn's return
//!   (a sink is one level) nor an implicit format capture (`{name}` inside a literal does not
//!   count in LOCAL's uses).
//!
//! LS2a (§5.2 Q-LEXSCOPE, the module `crate::lexscope` and its funnel): H3 / HK / HF read a
//! funnel entry call `<recv>.lex.<entry>(..)` as the write it replaced, with the entry's name as
//! its form (`scope_restore(b)` / `scope_repoint(x)` as `cur_prefix = b / x`) ; the field inside
//! `lexscope/` is `Scopes`' own and is not a site ; HM's `curp` also reads the bridge copy
//! `<recv>.lex.cur_prefix_text().to_string()`. Three pins of the funnel itself, keyed by the
//! module's lists in `scope_surface.tables`:
//!
//! - HDBG the identifier `debug_assertions` occurs 0 times in lexscope's files ; its
//!   `debug_assert*!` calls by (file, fn, macro) with a count (DELTA9 N-8, DELTA10 N-g).
//! - HBR every call of a bridge entry (SURF_LEAVE's items) outside lexscope by (file, fn,
//!   entry, SINK) with a count, SINK as HOUT's (DELTA10 w1 ; TW-BR: it may only fall).
//! - HC ARGS for every SURF_ENTER row of role to-scope, the token text of the argument at that
//!   parameter in every call of the entry, crate-wide, by (file, fn, callee, tokens) with a
//!   count (DELTA11 R4 N-8, DELTA12 S2 r4) ; the entry's name must be unique in the crate.
//!   LS3 (PLAN8 H2 (b)): and for every lexscope fn with a parameter whose type names
//!   `ScopeRef` (computed from lexscope's own signatures, at most one such parameter per fn:
//!   PLAN8 H2 (4)), the argument at that parameter, the same way.
//! - HC CARRY (LS3, PLAN8 H2 (c)) every call of `scope_carry` outside lexscope by (file, fn,
//!   SINK) with a count, SINK as HBR's ; a LOCAL sink (a carry bound by `let`) is pinned 0.
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
/// H6: a probe of a local table (`contains_at`: a Keyed / KeyedSet probed by an AtKey, the walk's
/// candidate since §5.2 Q-DECLKEY DK1-4b-1 — walk_scopes' `table.contains_at(k)` ; the `_found`
/// verbs: a store probed by the key a walk FOUND, a DeclKey since DK1-4b-2).
const PROBE_METHODS: &[&str] = &[
    "contains_key",
    "contains_at",
    "contains_found",
    "get_found",
    "index_found",
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
    // §5.2 Q-DECLKEY DK1-3: fq's two forms (the read key, the message text), read as fq is
    "at_cur",
    "show_miss",
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
    /// H6 (§5.2 Q-DECLKEY DK1-4b-2): the local closures that read a walk's FOUND key (a
    /// DeclKey) -> their probes ; each is its own FOUND row (`found_key_closure`)
    found_closures: BTreeMap<String, Vec<String>>,
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
            // inside the module the field is `Scopes`' own (LS2a): its writes are the
            // entries' bodies (HT-SURF's BODY-ALL), not sites ; the entry calls count
            if in_lexscope(&c.file) && !is_lex_entry_name(form) {
                return;
            }
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

/// The funnel's openers (LS2a, §5.2 Q-LEXSCOPE): `<recv>.lex.<entry>(..)` moves the current
/// scope and returns its `Saved` ; after LS2 a write site is the same row with the entry's
/// name as its form (PLAN8 H5 LS1 part 2). LS3 adds `scope_swap`, the re-entry of a carried
/// scope (`&ScopeRef`) ; LS3b `swap_to_mark`, the re-entry of a mark's scope.
const LEX_OPENERS: &[&str] = &[
    "scope_swap_text",
    "scope_swap_empty",
    "scope_swap_pkg",
    "scope_enter",
    "scope_hold",
    "scope_swap",
    "swap_to_mark",
];
/// The funnel's closer and its re-point (inside an open pair).
const LEX_CLOSERS: &[&str] = &["scope_restore", "scope_repoint"];

fn is_lex_entry_name(n: &str) -> bool {
    LEX_OPENERS.contains(&n) || LEX_CLOSERS.contains(&n)
}

/// A file of the module `crate::lexscope`.
fn in_lexscope(file: &str) -> bool {
    file.starts_with("lexscope/")
}

/// `<recv>.lex.<entry>(..)` for a funnel entry: the entry's name.
fn lex_entry(m: &syn::ExprMethodCall) -> Option<String> {
    let n = m.method.to_string();
    let on_lex = matches!(&*m.receiver, Expr::Field(f) if matches!(&f.member, syn::Member::Named(i) if i == "lex"));
    (on_lex && is_lex_entry_name(&n)).then_some(n)
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

/// H6 (§5.2 Q-DECLKEY DK1-4b-2): a closure reads a walk's FOUND key through its first parameter
/// when that parameter is typed `DeclKey` (by value or by reference) or, untyped, when the body
/// hands it to a `_found` entry, to `key_same`, or asks it `is_formed_at` / `is_formed_as` (a
/// closure typed anything else, e.g. `|me: &Self|` around its own walk, is not one). REVIEW-ONLY:
/// a found key read by a fn (not a closure), a closure whose parameter is a pattern, or one that
/// only hands its key on to another closure ; and (§5.2 Q-DECLKEY DK1-5a, DELTA16 R-H12 note 3)
/// two found-key closures bound to ONE name in one fn (a shadowing `let`) are one FOUND row:
/// `found_closures` is keyed by the binding name, so the later closure's probes replace the
/// earlier's and the earlier's stores are pinned by nothing.
fn found_key_closure(e: &Expr) -> bool {
    struct F<'n>(&'n str, bool);
    fn names(e: &Expr, n: &str) -> bool {
        match e {
            Expr::Reference(r) => names(&r.expr, n),
            Expr::Paren(p) => names(&p.expr, n),
            other => single_path(other).is_some_and(|p| p == n),
        }
    }
    impl<'ast> Visit<'ast> for F<'_> {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            let meth = m.method.to_string();
            if (meth.ends_with("_found") && m.args.iter().any(|a| names(a, self.0)))
                || ((meth == "is_formed_at" || meth == "is_formed_as")
                    && names(&m.receiver, self.0))
            {
                self.1 = true;
            }
            visit::visit_expr_method_call(self, m);
        }
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if single_path(&c.func).is_some_and(|n| n == "key_same")
                && c.args.iter().any(|a| names(a, self.0))
            {
                self.1 = true;
            }
            visit::visit_expr_call(self, c);
        }
    }
    let Expr::Closure(c) = e else {
        return false;
    };
    let (pat, ty) = match c.inputs.first() {
        Some(Pat::Type(pt)) => (&*pt.pat, Some(&*pt.ty)),
        Some(p) => (p, None),
        None => return false,
    };
    if let Some(mut ty) = ty {
        while let syn::Type::Reference(r) = ty {
            ty = &r.elem;
        }
        return matches!(ty, syn::Type::Path(tp)
            if tp.path.segments.last().is_some_and(|l| l.ident == "DeclKey"));
    }
    let Pat::Ident(pi) = pat else {
        return false;
    };
    let n = pi.ident.to_string();
    let mut f = F(&n, false);
    f.visit_expr(&c.body);
    f.1
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
                let found = found_key_closure(&init.expr);
                if let Some(c) = self.cur() {
                    if found {
                        c.found_closures.insert(first.clone(), probes.clone());
                    }
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
            if matches!(&f.member, syn::Member::Named(i) if i == "cur_prefix" || i == "lex") {
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
        if let Some(e) = lex_entry(m) {
            self.prefix_write(&e);
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
                o if LEX_OPENERS.contains(&o) => {
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
        // FOUND (§5.2 Q-DECLKEY DK1-4b-2, DELTA16 R-H8 (g)): a closure that reads a walk's found
        // key is no walk hit, so no SITE row holds its stores — each gets its own row, entry
        // column `<name>()`, its probes and their store set as a hit's
        for (name, probes) in &r.found_closures {
            let text = if probes.is_empty() {
                "?".to_string()
            } else {
                probes.join("|")
            };
            let mut set: BTreeSet<String> = BTreeSet::new();
            let mut todo: Vec<String> = probes.clone();
            let mut seen: BTreeSet<String> = BTreeSet::new();
            while let Some(p) = todo.pop() {
                if !seen.insert(p.clone()) {
                    continue;
                }
                if let Some(fs) = hit_fns.get(p.as_str()) {
                    todo.extend(fs.iter().cloned());
                } else if let Some(local) = p
                    .strip_suffix("()")
                    .filter(|n| n != name)
                    .and_then(|n| r.local_closures.get(n))
                {
                    todo.extend(local.iter().cloned());
                } else {
                    set.insert(p);
                }
            }
            let set = set.into_iter().collect::<Vec<_>>().join("+");
            let k = format!("{}\t{}\t{name}()\t{text}", r.file, r.qual);
            let e = agg
                .entry(k)
                .or_insert(("FOUND".to_string(), set, 0, r.line));
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
    // a local Keyed probed by the walk's AtKey candidate (DK1-4b-1)
    assert_eq!(
        probes_of(&syn::parse_str::<Expr>("|k| table.contains_at(k)").expect("probe at")),
        ["table"]
    );
    // a local Keyed probed by a walk's FOUND key, and the found-key closures (DK1-4b-2)
    assert_eq!(
        probes_of(&syn::parse_str::<Expr>("|k| table.contains_found(k)").expect("probe found")),
        ["table"]
    );
    for (src, found) in [
        ("|k: &DeclKey| self.symbols.contains_found(k)", true),
        (
            "|k: &DeclKey| self.clk_refused.iter().position(|r| k.is_formed_at(&r.alias))",
            true,
        ),
        ("|k| self.wide_param_bits.contains_found(k)", true),
        ("|i| key_same(i, &key)", true),
        ("|k: &AtKey| self.symbols.contains_at(k)", false),
        ("|k: &str| self.symbols.contains_key(k)", false),
        ("|me: &Self| me.symbols.contains_found(&k)", false),
        ("|k| self.symbols.contains_found(&other)", false),
    ] {
        assert_eq!(
            found_key_closure(&syn::parse_str::<Expr>(src).expect("found closure")),
            found,
            "{src}"
        );
    }
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
            .filter(|a| a.0 == "W" && !matches!(a.1.as_str(), "fq" | "at_cur" | "show_miss"))
            .map(|a| a.1.as_str())
            .collect();
        let crit_a = fs.len() >= 2 || (!fs.is_empty() && !ws.is_empty());
        let mut by_arg: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for a in r.asks.iter().filter(|a| a.0 == "W") {
            // fq's two forms count as the one entry fq was (criterion (c): two entries, one argument)
            let e = match a.1.as_str() {
                "at_cur" | "show_miss" => "fq",
                e => e,
            };
            by_arg.entry(a.2.as_str()).or_default().insert(e);
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
            if a.0 == "Fw"
                || matches!(a.1.as_str(), "fq" | "at_cur" | "show_miss")
                || matches!(a.2.as_str(), "" | "?" | "lit" | "call()")
            {
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
/// HS / H5 (§5.2 Q-DECLKEY DK1-1): the stores' two types of window w9 (`lexscope/key.rs`), by their
/// exact names: a `BTreeMap<String, V>` / `BTreeSet<String>` held behind `Deref` / `DerefMut`, so a
/// field of one is still keyed by `String` text (HS's store set, H5's name-typed fields) until DK3-b.
const WINDOW_TYPES: &[&str] = &["Keyed", "KeyedSet"];
/// HH: a probe of a store (k4.py READ less the compiler-only ops).
const HH_READ: &[&str] = &[
    "get",
    "contains_key",
    "contains",
    "range",
    "get_mut",
    // §5.2 Q-DECLKEY DK1-3: the AtKey entries of a store
    "get_at",
    "contains_at",
];
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
/// HD: the display-chain fields (DELTA6 F1) — the Elaborator's three until LS2-d, and their
/// names inside `lexscope::Scopes` from LS2-d on (`disp_chain`, `disp_root`, `disp_method`).
const DISPLAY_FIELDS: &[&str] = &[
    "block_scope",
    "block_scope_root",
    "cur_class_method",
    "disp_chain",
    "disp_root",
    "disp_method",
];
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

/// `<x>.fq(..)` or `fq(..)`, or (§5.2 Q-DECLKEY DK1-3) `<x>.at_cur(..)`: the current-prefix read key.
fn is_fq_call(e: &Expr) -> bool {
    match strip_key(e) {
        Expr::MethodCall(m) => m.method == "fq" || m.method == "at_cur",
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
        // HM after LS2a: the bridge's copy `<recv>.lex.cur_prefix_text().to_string()` (the
        // same mint with the entry's name)
        if meth == "to_string"
            && matches!(&*m.receiver, Expr::MethodCall(b) if b.method == "cur_prefix_text"
                && matches!(&*b.receiver, Expr::Field(f) if matches!(&f.member, syn::Member::Named(i) if i == "lex")))
        {
            self.mint("curp", "cur_prefix_text().to_string()");
        }
        self.visit_expr(&m.receiver);
        self.called(&meth);
        if meth == "fq" || meth == "at_cur" {
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
/// argument is `String`, or a type of WINDOW_TYPES.
fn string_keyed(ty: &syn::Type) -> bool {
    let syn::Type::Path(p) = ty else { return false };
    let Some(last) = p.path.segments.last() else {
        return false;
    };
    if WINDOW_TYPES.contains(&last.ident.to_string().as_str()) {
        return true;
    }
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
         let _ = self.walk_scopes_key(n, |k| self.symbols.contains_key(k)); \
         let _ = self.symbols.get_at(&self.lex.at_cur(n)); } }",
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
            ("symbols".into(), "get_at".into(), "call3".into()),
        ],
        "HH's reader no longer links an fq / at_cur key through a `let`, a direct argument or an index"
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
                if ids
                    .iter()
                    .any(|i| NAME_TYPES.contains(&i.as_str()) || WINDOW_TYPES.contains(&i.as_str()))
                {
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
// HF the shape of each `cur_prefix` write site (LS1 part 3)
// =====================================================================================

/// HF: a position — the statement index in a block (index, block length) or a control
/// construct entered (`then`, `else`, `arm<j>`, `for`, `while`, `loop`, `blk`,
/// `closure`, `letelse`). Expressions that are not control constructs add no step.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    S(usize, usize),
    C(String),
}

fn path_text(p: &[Step]) -> String {
    if p.is_empty() {
        return "body".into();
    }
    p.iter()
        .map(|s| match s {
            Step::S(i, _) => format!("s{i}"),
            Step::C(c) => c.clone(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// A position inside the block `b`: its first step counted back from the block's last
/// statement (`e0` = the last), deeper steps from their block's first.
fn rel_text(b: &[Step], p: &[Step]) -> String {
    if !p.starts_with(b) || p.len() == b.len() {
        return format!("^{}", path_text(p));
    }
    let mut v = Vec::new();
    for (k, s) in p[b.len()..].iter().enumerate() {
        v.push(match (k, s) {
            (0, Step::S(i, n)) => format!("e{}", n - 1 - i),
            (_, Step::S(i, _)) => format!("s{i}"),
            (_, Step::C(c)) => c.clone(),
        });
    }
    v.join("/")
}

/// The token text of an expression, for an ARG column (this test has no `quote`).
fn tx(e: &Expr) -> String {
    let list = |v: &Punctuated<Expr, Token![,]>| v.iter().map(tx).collect::<Vec<_>>().join(", ");
    let path = |p: &syn::Path| {
        p.segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    };
    match e {
        Expr::Path(p) => path(&p.path),
        Expr::Field(f) => match &f.member {
            syn::Member::Named(i) => format!("{}.{i}", tx(&f.base)),
            syn::Member::Unnamed(i) => format!("{}.{}", tx(&f.base), i.index),
        },
        Expr::MethodCall(m) => format!("{}.{}({})", tx(&m.receiver), m.method, list(&m.args)),
        Expr::Call(c) => format!("{}({})", tx(&c.func), list(&c.args)),
        Expr::Macro(m) => {
            match Punctuated::<Expr, Token![,]>::parse_terminated.parse2(m.mac.tokens.clone()) {
                Ok(l) => format!("{}!({})", path(&m.mac.path), list(&l)),
                Err(_) => format!("{}!({})", path(&m.mac.path), m.mac.tokens),
            }
        }
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Str(s) => format!("{:?}", s.value()),
            syn::Lit::Int(i) => i.base10_digits().to_string(),
            syn::Lit::Bool(b) => b.value.to_string(),
            syn::Lit::Char(c) => format!("{:?}", c.value()),
            _ => "lit".into(),
        },
        Expr::Reference(r) => format!(
            "&{}{}",
            if r.mutability.is_some() { "mut " } else { "" },
            tx(&r.expr)
        ),
        Expr::Paren(p) => format!("({})", tx(&p.expr)),
        Expr::Unary(u) => {
            let op = match u.op {
                syn::UnOp::Deref(_) => "*",
                syn::UnOp::Not(_) => "!",
                syn::UnOp::Neg(_) => "-",
                _ => "?",
            };
            format!("{op}{}", tx(&u.expr))
        }
        Expr::Index(i) => format!("{}[{}]", tx(&i.expr), tx(&i.index)),
        Expr::Tuple(t) => format!("({})", list(&t.elems)),
        Expr::Array(a) => format!("[{}]", list(&a.elems)),
        Expr::Cast(c) => format!("{} as _", tx(&c.expr)),
        Expr::Try(t) => format!("{}?", tx(&t.expr)),
        Expr::Closure(_) => "|..| ..".into(),
        _ => "{..}".into(),
    }
}

/// A macro's arguments as expressions (Pass2's reading: a comma list, else the first).
fn mac_exprs(mac: &syn::Macro) -> Vec<Expr> {
    if let Ok(l) = Punctuated::<Expr, Token![,]>::parse_terminated.parse2(mac.tokens.clone()) {
        return l.into_iter().collect();
    }
    let first = |input: ParseStream| -> syn::Result<Expr> {
        let e: Expr = input.parse()?;
        let _: proc_macro2::TokenStream = input.parse()?;
        Ok(e)
    };
    first.parse2(mac.tokens.clone()).into_iter().collect()
}

const DIVERGE_MACROS: &[&str] = &["panic", "unreachable", "todo", "unimplemented"];

/// `std::mem::{replace, take, swap}(&mut <recv>.cur_prefix, ..)`: (form, ARG).
fn opener_call(e: &Expr) -> Option<(&'static str, String)> {
    match e {
        Expr::Call(c) => opener_of(c),
        Expr::MethodCall(m) => lex_opener_of(m),
        _ => None,
    }
}

/// `<recv>.lex.<opener>(..)`: (the entry's name, ARG = its first argument or `-`).
fn lex_opener_of(m: &syn::ExprMethodCall) -> Option<(&'static str, String)> {
    let n = lex_entry(m)?;
    let form = *LEX_OPENERS.iter().find(|o| **o == n)?;
    Some((form, m.args.first().map(tx).unwrap_or_else(|| "-".into())))
}

fn opener_of(c: &syn::ExprCall) -> Option<(&'static str, String)> {
    let Expr::Path(p) = &*c.func else { return None };
    let form = match p.path.segments.last()?.ident.to_string().as_str() {
        "replace" => "replace",
        "take" => "take",
        "swap" => "swap",
        _ => return None,
    };
    if !c.args.first().is_some_and(is_mut_prefix_ref) {
        return None;
    }
    Some((
        form,
        c.args.iter().nth(1).map(tx).unwrap_or_else(|| "-".into()),
    ))
}

/// `<recv>.cur_prefix.clone()`.
fn is_prefix_clone(e: &Expr) -> bool {
    matches!(e, Expr::MethodCall(m) if m.method == "clone" && m.args.is_empty() && is_prefix_field(&m.receiver))
}

/// One HF event of a fn body, in source order.
enum HfEv<'a> {
    /// a `mem::*` opener ; `bind` the `let` it initializes ; `region` its block and index
    Open {
        form: &'static str,
        arg: String,
        bind: Option<String>,
        region: Option<(&'a [syn::Stmt], usize)>,
    },
    /// `let <bind> = <recv>.cur_prefix.clone()` (a save the fn may restore from)
    Clone {
        bind: String,
        region: Option<(&'a [syn::Stmt], usize)>,
    },
    /// `<recv>.cur_prefix = <rhs>` ; `ident` when the right side is one name
    Assign { rhs: String, ident: Option<String> },
    /// `cur_prefix: <arg>` in a struct literal
    Init { arg: String },
    /// any other write: a `&mut` not a `mem::*` opener, a compound assignment, a
    /// mutating method
    Other { form: &'static str },
    /// `return`, `?`, `break`, `continue` ; `target` the loop (or labeled block) a
    /// `break` / `continue` leaves, `None` for the fn (or the closure, `closure` > 0)
    Exit {
        kind: &'static str,
        target: Option<Vec<Step>>,
    },
    /// a method call on the Elaborator (`self` / `s` / `me` / `this`), or `dyn:<name>` a
    /// call of a parameter or a local
    Call { name: String },
    /// a `with_scope` / `elaborate_instance` call and its scope argument
    Entry { name: String, arg: String },
}

struct HfAt<'a> {
    pos: Vec<Step>,
    closure: usize,
    ev: HfEv<'a>,
}

/// The HF walk of one fn body (nested fns are their own rows).
struct HfWalk<'a> {
    path: Vec<Step>,
    /// (label, path of the construct's body, a loop rather than a labeled block)
    loops: Vec<(Option<String>, Vec<Step>, bool)>,
    closure: usize,
    names: BTreeSet<String>,
    block: Vec<(&'a [syn::Stmt], usize)>,
    evs: Vec<HfAt<'a>>,
    /// a fn of `crate::lexscope`: the field is `Scopes`' own, its writes are not sites
    lexmod: bool,
}

impl<'a> HfWalk<'a> {
    fn push(&mut self, ev: HfEv<'a>) {
        let field_write = matches!(
            ev,
            HfEv::Open { .. }
                | HfEv::Clone { .. }
                | HfEv::Assign { .. }
                | HfEv::Init { .. }
                | HfEv::Other { .. }
        );
        if self.lexmod && field_write {
            return;
        }
        self.push_lex(ev);
    }
    /// an event of a funnel entry call (`<recv>.lex.<entry>(..)`), counted in every file
    fn push_lex(&mut self, ev: HfEv<'a>) {
        self.evs.push(HfAt {
            pos: self.path.clone(),
            closure: self.closure,
            ev,
        });
    }
    fn region(&self) -> Option<(&'a [syn::Stmt], usize)> {
        self.block.last().copied()
    }
    fn body_block(&mut self, b: &'a syn::Block) {
        for (i, s) in b.stmts.iter().enumerate() {
            self.path.push(Step::S(i, b.stmts.len()));
            self.block.push((&b.stmts, i));
            self.visit_stmt(s);
            self.block.pop();
            self.path.pop();
        }
    }
    fn body_expr(&mut self, e: &'a Expr) {
        match e {
            Expr::Block(b) if b.label.is_none() => self.body_block(&b.block),
            _ => self.visit_expr(e),
        }
    }
    fn jump(&mut self, kind: &'static str, label: Option<&syn::Lifetime>) {
        let target = match label {
            Some(l) => {
                let n = l.ident.to_string();
                self.loops
                    .iter()
                    .rev()
                    .find(|(lb, _, _)| lb.as_deref() == Some(n.as_str()))
            }
            None => self.loops.iter().rev().find(|(_, _, is_loop)| *is_loop),
        }
        .map(|(_, p, _)| p.clone());
        self.push(HfEv::Exit { kind, target });
    }
    fn in_loop(&mut self, label: &Option<syn::Label>, name: &str, body: &'a syn::Block) {
        self.path.push(Step::C(name.into()));
        self.loops.push((
            label.as_ref().map(|l| l.name.ident.to_string()),
            self.path.clone(),
            true,
        ));
        self.body_block(body);
        self.loops.pop();
        self.path.pop();
    }
}

impl<'a> Visit<'a> for HfWalk<'a> {
    fn visit_item(&mut self, _: &'a syn::Item) {}
    fn visit_block(&mut self, b: &'a syn::Block) {
        self.path.push(Step::C("blk".into()));
        self.body_block(b);
        self.path.pop();
    }
    fn visit_local(&mut self, l: &'a syn::Local) {
        let region = self.region();
        if let Some(init) = &l.init {
            let mut binds = Vec::new();
            pat_names(&l.pat, &mut binds);
            if let Some((form, arg)) = opener_call(&init.expr) {
                let bind = match &l.pat {
                    Pat::Ident(i) => Some(i.ident.to_string()),
                    _ => None,
                };
                match &*init.expr {
                    Expr::Call(c) => {
                        self.push(HfEv::Open {
                            form,
                            arg,
                            bind,
                            region,
                        });
                        for a in c.args.iter().skip(1) {
                            self.visit_expr(a);
                        }
                    }
                    Expr::MethodCall(m) => {
                        for a in &m.args {
                            self.visit_expr(a);
                        }
                        self.push_lex(HfEv::Open {
                            form,
                            arg,
                            bind,
                            region,
                        });
                    }
                    _ => {}
                }
            } else {
                let pairs: Vec<(&Pat, &Expr)> = match (&l.pat, &*init.expr) {
                    (Pat::Tuple(p), Expr::Tuple(t)) if p.elems.len() == t.elems.len() => {
                        p.elems.iter().zip(t.elems.iter()).collect()
                    }
                    (p, e) => vec![(p, e)],
                };
                let mut lex_opened = false;
                for (p, e) in pairs {
                    if let (Pat::Ident(i), true) = (p, is_prefix_clone(e)) {
                        self.push(HfEv::Clone {
                            bind: i.ident.to_string(),
                            region,
                        });
                    } else if let (Pat::Ident(i), Some((form, arg))) = (p, opener_call(e)) {
                        if matches!(e, Expr::MethodCall(_)) {
                            self.push_lex(HfEv::Open {
                                form,
                                arg,
                                bind: Some(i.ident.to_string()),
                                region,
                            });
                            lex_opened = true;
                        }
                    }
                }
                if !lex_opened {
                    self.visit_expr(&init.expr);
                }
            }
            if let Some((_, els)) = &init.diverge {
                self.path.push(Step::C("letelse".into()));
                self.body_expr(els);
                self.path.pop();
            }
            self.names.extend(binds);
        }
    }
    fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
        if let Some((form, arg)) = opener_of(c) {
            let region = self.region();
            self.push(HfEv::Open {
                form,
                arg,
                bind: None,
                region,
            });
            for a in c.args.iter().skip(1) {
                self.visit_expr(a);
            }
            return;
        }
        if let Some(n) = single_path(&c.func) {
            if self.names.contains(&n) {
                self.push(HfEv::Call {
                    name: format!("dyn:{n}"),
                });
            }
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) {
        let meth = m.method.to_string();
        if STRING_MUT.contains(&meth.as_str()) && is_prefix_field(&m.receiver) {
            self.push(HfEv::Other { form: "method" });
        }
        self.visit_expr(&m.receiver);
        for a in &m.args {
            self.visit_expr(a);
        }
        if let Some((form, arg)) = lex_opener_of(m) {
            let region = self.region();
            self.push_lex(HfEv::Open {
                form,
                arg,
                bind: None,
                region,
            });
        } else if lex_entry(m).is_some() {
            // scope_restore(<bind>) / scope_repoint(<text>): an assignment of the field
            let x = m.args.first();
            self.push_lex(HfEv::Assign {
                rhs: x.map(tx).unwrap_or_else(|| "-".into()),
                ident: x.and_then(single_path),
            });
        }
        let at = match meth.as_str() {
            "with_scope" => Some(0),
            "elaborate_instance" => Some(1),
            _ => None,
        };
        if let Some(k) = at {
            let arg = m.args.iter().nth(k).map(tx).unwrap_or_else(|| "-".into());
            self.push(HfEv::Entry {
                name: meth.clone(),
                arg,
            });
        }
        if matches!(&*m.receiver, Expr::Path(p) if p.path.segments.len() == 1 && RECV2.contains(&p.path.segments[0].ident.to_string().as_str()))
        {
            self.push(HfEv::Call { name: meth });
        }
    }
    fn visit_expr_reference(&mut self, r: &'a syn::ExprReference) {
        if r.mutability.is_some() && is_prefix_field(&r.expr) {
            self.push(HfEv::Other { form: "mutref" });
        }
        visit::visit_expr_reference(self, r);
    }
    fn visit_expr_assign(&mut self, a: &'a syn::ExprAssign) {
        if is_prefix_field(&a.left) {
            self.visit_expr(&a.right);
            self.push(HfEv::Assign {
                rhs: tx(&a.right),
                ident: single_path(&a.right),
            });
            return;
        }
        visit::visit_expr_assign(self, a);
    }
    fn visit_expr_binary(&mut self, b: &'a syn::ExprBinary) {
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
            self.push(HfEv::Other { form: "opassign" });
        }
        visit::visit_expr_binary(self, b);
    }
    fn visit_expr_struct(&mut self, s: &'a syn::ExprStruct) {
        for f in &s.fields {
            if matches!(&f.member, syn::Member::Named(i) if i == "cur_prefix" || i == "lex") {
                self.push(HfEv::Init { arg: tx(&f.expr) });
            }
        }
        visit::visit_expr_struct(self, s);
    }
    fn visit_expr_closure(&mut self, c: &'a syn::ExprClosure) {
        let loops = std::mem::take(&mut self.loops);
        self.closure += 1;
        self.path.push(Step::C("closure".into()));
        self.body_expr(&c.body);
        self.path.pop();
        self.closure -= 1;
        self.loops = loops;
    }
    fn visit_expr_if(&mut self, i: &'a syn::ExprIf) {
        self.visit_expr(&i.cond);
        self.path.push(Step::C("then".into()));
        self.body_block(&i.then_branch);
        self.path.pop();
        if let Some((_, e)) = &i.else_branch {
            self.path.push(Step::C("else".into()));
            self.body_expr(e);
            self.path.pop();
        }
    }
    fn visit_expr_match(&mut self, m: &'a syn::ExprMatch) {
        self.visit_expr(&m.expr);
        for (j, a) in m.arms.iter().enumerate() {
            self.path.push(Step::C(format!("arm{j}")));
            if let Some((_, g)) = &a.guard {
                self.visit_expr(g);
            }
            self.body_expr(&a.body);
            self.path.pop();
        }
    }
    fn visit_expr_for_loop(&mut self, f: &'a syn::ExprForLoop) {
        self.visit_expr(&f.expr);
        self.in_loop(&f.label, "for", &f.body);
    }
    fn visit_expr_while(&mut self, w: &'a syn::ExprWhile) {
        self.visit_expr(&w.cond);
        self.in_loop(&w.label, "while", &w.body);
    }
    fn visit_expr_loop(&mut self, l: &'a syn::ExprLoop) {
        self.in_loop(&l.label, "loop", &l.body);
    }
    fn visit_expr_block(&mut self, b: &'a syn::ExprBlock) {
        self.path.push(Step::C("blk".into()));
        let labeled = b.label.is_some();
        if let Some(l) = &b.label {
            self.loops
                .push((Some(l.name.ident.to_string()), self.path.clone(), false));
        }
        self.body_block(&b.block);
        if labeled {
            self.loops.pop();
        }
        self.path.pop();
    }
    fn visit_expr_return(&mut self, r: &'a syn::ExprReturn) {
        visit::visit_expr_return(self, r);
        self.push(HfEv::Exit {
            kind: "return",
            target: None,
        });
    }
    fn visit_expr_try(&mut self, t: &'a syn::ExprTry) {
        visit::visit_expr_try(self, t);
        self.push(HfEv::Exit {
            kind: "?",
            target: None,
        });
    }
    fn visit_expr_break(&mut self, b: &'a syn::ExprBreak) {
        visit::visit_expr_break(self, b);
        self.jump("break", b.label.as_ref());
    }
    fn visit_expr_continue(&mut self, c: &'a syn::ExprContinue) {
        self.jump("continue", c.label.as_ref());
    }
    fn visit_macro(&mut self, mac: &'a syn::Macro) {
        // the parsed arguments are owned by this call: a sub-walk, whose events keep no
        // region (an opener inside a macro is a row with no paths)
        let es = mac_exprs(mac);
        let mut w = HfWalk {
            path: self.path.clone(),
            loops: self.loops.clone(),
            closure: self.closure,
            names: self.names.clone(),
            block: Vec::new(),
            evs: Vec::new(),
            lexmod: self.lexmod,
        };
        for e in &es {
            w.visit_expr(e);
        }
        for a in w.evs {
            let ev = match a.ev {
                HfEv::Open {
                    form, arg, bind, ..
                } => HfEv::Open {
                    form,
                    arg,
                    bind,
                    region: None,
                },
                HfEv::Clone { bind, .. } => HfEv::Clone { bind, region: None },
                HfEv::Assign { rhs, ident } => HfEv::Assign { rhs, ident },
                HfEv::Init { arg } => HfEv::Init { arg },
                HfEv::Other { form } => HfEv::Other { form },
                HfEv::Exit { kind, target } => HfEv::Exit { kind, target },
                HfEv::Call { name } => HfEv::Call { name },
                HfEv::Entry { name, arg } => HfEv::Entry { name, arg },
            };
            self.evs.push(HfAt {
                pos: a.pos,
                closure: a.closure,
                ev,
            });
        }
    }
}

/// HF's path state at a point after the opener: (the prefix differs from the saved
/// value, restores so far saturated at 2).
type HfSt = (bool, u8);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum HfJump {
    Ret,
    Try,
    Panic,
    Brk(Option<String>),
    Cont(Option<String>),
}

#[derive(Default)]
struct HfOut {
    fall: BTreeSet<HfSt>,
    jumps: BTreeSet<(HfJump, HfSt)>,
}

/// The paths from an opener to the end of its block: each `cur_prefix = <bind>` a
/// restore, each other assignment a re-point, unless its right side is another site's
/// saved binding (that site's own restore). A closure body is not entered (its exits
/// leave the closure) ; a write inside one is counted in `closure_writes`.
struct HfFlow<'s> {
    bind: &'s str,
    others: &'s BTreeSet<String>,
    closure_writes: usize,
}

impl HfFlow<'_> {
    fn stmts(&mut self, ss: &[syn::Stmt], ins: &BTreeSet<HfSt>) -> HfOut {
        let mut out = HfOut::default();
        let mut cur = ins.clone();
        for s in ss {
            if cur.is_empty() {
                break;
            }
            let o = self.stmt(s, &cur);
            out.jumps.extend(o.jumps);
            cur = o.fall;
        }
        out.fall = cur;
        out
    }
    fn stmt(&mut self, s: &syn::Stmt, ins: &BTreeSet<HfSt>) -> HfOut {
        match s {
            syn::Stmt::Local(l) => match &l.init {
                Some(init) => {
                    let mut o = self.expr(&init.expr, ins);
                    if let Some((_, els)) = &init.diverge {
                        let e = self.expr(els, &o.fall);
                        o.jumps.extend(e.jumps);
                    }
                    o
                }
                None => HfOut {
                    fall: ins.clone(),
                    ..Default::default()
                },
            },
            syn::Stmt::Item(_) => HfOut {
                fall: ins.clone(),
                ..Default::default()
            },
            syn::Stmt::Expr(e, _) => self.expr(e, ins),
            syn::Stmt::Macro(m) => self.mac(&m.mac, ins),
        }
    }
    fn seq<'e>(&mut self, es: impl Iterator<Item = &'e Expr>, ins: &BTreeSet<HfSt>) -> HfOut {
        let mut out = HfOut::default();
        let mut cur = ins.clone();
        for e in es {
            let o = self.expr(e, &cur);
            out.jumps.extend(o.jumps);
            cur = o.fall;
        }
        out.fall = cur;
        out
    }
    fn mac(&mut self, mac: &syn::Macro, ins: &BTreeSet<HfSt>) -> HfOut {
        let es = mac_exprs(mac);
        let mut o = self.seq(es.iter(), ins);
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if DIVERGE_MACROS.contains(&name.as_str()) {
            for st in std::mem::take(&mut o.fall) {
                o.jumps.insert((HfJump::Panic, st));
            }
        }
        o
    }
    fn block(&mut self, b: &syn::Block, ins: &BTreeSet<HfSt>) -> HfOut {
        self.stmts(&b.stmts, ins)
    }
    /// a loop: `head` the states at each iteration's start, to a fixpoint
    fn lp(
        &mut self,
        label: &Option<syn::Label>,
        cond: Option<&Expr>,
        body: &syn::Block,
        entry: &BTreeSet<HfSt>,
        may_skip: bool,
    ) -> HfOut {
        let lb = label.as_ref().map(|l| l.name.ident.to_string());
        let mine = |l: &Option<String>| l.is_none() || *l == lb;
        let mut head = entry.clone();
        let mut out = HfOut::default();
        loop {
            let enter = match cond {
                Some(c) => {
                    let o = self.expr(c, &head);
                    out.jumps.extend(o.jumps);
                    out.fall.extend(o.fall.iter().copied());
                    o.fall
                }
                None => {
                    if may_skip {
                        out.fall.extend(head.iter().copied());
                    }
                    head.clone()
                }
            };
            let o = self.block(body, &enter);
            let mut next = head.clone();
            next.extend(o.fall.iter().copied());
            for (j, st) in o.jumps {
                match &j {
                    HfJump::Cont(l) if mine(l) => {
                        next.insert(st);
                    }
                    HfJump::Brk(l) if mine(l) => {
                        out.fall.insert(st);
                    }
                    _ => {
                        out.jumps.insert((j, st));
                    }
                }
            }
            if next == head {
                break;
            }
            head = next;
        }
        out
    }
    fn expr(&mut self, e: &Expr, ins: &BTreeSet<HfSt>) -> HfOut {
        let pass = |ins: &BTreeSet<HfSt>| HfOut {
            fall: ins.clone(),
            ..Default::default()
        };
        // after LS2a `<recv>.lex.scope_restore(x)` / `scope_repoint(x)` is the assignment
        // `<recv>.cur_prefix = x` with the entry's name
        let lex_set = match e {
            Expr::MethodCall(m)
                if lex_entry(m).is_some_and(|n| LEX_CLOSERS.contains(&n.as_str())) =>
            {
                m.args.first()
            }
            _ => None,
        };
        match e {
            Expr::Assign(_) | Expr::MethodCall(_)
                if lex_set.is_some()
                    || matches!(e, Expr::Assign(a) if is_prefix_field(&a.left)) =>
            {
                let right = match (e, lex_set) {
                    (_, Some(x)) => x,
                    (Expr::Assign(a), None) => &*a.right,
                    _ => unreachable!(),
                };
                let mut o = self.expr(right, ins);
                let id = single_path(right);
                if id.as_deref() == Some(self.bind) {
                    o.fall = o
                        .fall
                        .iter()
                        .map(|&(_, n)| (false, (n + 1).min(2)))
                        .collect();
                } else if !id.is_some_and(|i| self.others.contains(&i)) {
                    o.fall = o.fall.iter().map(|&(_, n)| (true, n)).collect();
                }
                o
            }
            Expr::If(i) => {
                let c = self.expr(&i.cond, ins);
                let t = self.block(&i.then_branch, &c.fall);
                let mut out = match &i.else_branch {
                    Some((_, eb)) => self.expr(eb, &c.fall),
                    None => pass(&c.fall),
                };
                out.fall.extend(t.fall);
                out.jumps.extend(t.jumps);
                out.jumps.extend(c.jumps);
                out
            }
            Expr::Let(l) => self.expr(&l.expr, ins),
            Expr::Match(m) => {
                let s = self.expr(&m.expr, ins);
                let mut out = HfOut {
                    jumps: s.jumps.clone(),
                    ..Default::default()
                };
                for a in &m.arms {
                    let g = match &a.guard {
                        Some((_, g)) => self.expr(g, &s.fall),
                        None => pass(&s.fall),
                    };
                    let b = self.expr(&a.body, &g.fall);
                    out.jumps.extend(g.jumps);
                    out.jumps.extend(b.jumps);
                    out.fall.extend(b.fall);
                }
                out
            }
            Expr::ForLoop(f) => {
                let it = self.expr(&f.expr, ins);
                let mut o = self.lp(&f.label, None, &f.body, &it.fall, true);
                o.jumps.extend(it.jumps);
                o
            }
            Expr::While(w) => self.lp(&w.label, Some(&w.cond), &w.body, ins, false),
            Expr::Loop(l) => self.lp(&l.label, None, &l.body, ins, false),
            Expr::Block(b) => {
                let mut o = self.block(&b.block, ins);
                if let Some(l) = &b.label {
                    let n = Some(l.name.ident.to_string());
                    let (mine, rest): (BTreeSet<_>, BTreeSet<_>) = o
                        .jumps
                        .into_iter()
                        .partition(|(j, _)| matches!(j, HfJump::Brk(x) if *x == n));
                    o.fall.extend(mine.into_iter().map(|(_, st)| st));
                    o.jumps = rest;
                }
                o
            }
            Expr::Unsafe(b) => self.block(&b.block, ins),
            Expr::Const(b) => self.block(&b.block, ins),
            Expr::Closure(c) => {
                let mut w = HfWalk {
                    path: Vec::new(),
                    loops: Vec::new(),
                    closure: 0,
                    names: BTreeSet::new(),
                    block: Vec::new(),
                    evs: Vec::new(),
                    lexmod: false,
                };
                w.visit_expr(&c.body);
                self.closure_writes += w
                    .evs
                    .iter()
                    .filter(|a| {
                        matches!(
                            a.ev,
                            HfEv::Open { .. }
                                | HfEv::Assign { .. }
                                | HfEv::Other { .. }
                                | HfEv::Init { .. }
                        )
                    })
                    .count();
                pass(ins)
            }
            Expr::Return(r) => {
                let o = match &r.expr {
                    Some(x) => self.expr(x, ins),
                    None => pass(ins),
                };
                let mut out = HfOut {
                    jumps: o.jumps,
                    ..Default::default()
                };
                out.jumps
                    .extend(o.fall.into_iter().map(|st| (HfJump::Ret, st)));
                out
            }
            Expr::Try(t) => {
                let mut o = self.expr(&t.expr, ins);
                o.jumps.extend(o.fall.iter().map(|&st| (HfJump::Try, st)));
                o
            }
            Expr::Break(b) => {
                let o = match &b.expr {
                    Some(x) => self.expr(x, ins),
                    None => pass(ins),
                };
                let l = b.label.as_ref().map(|l| l.ident.to_string());
                let mut out = HfOut {
                    jumps: o.jumps,
                    ..Default::default()
                };
                out.jumps
                    .extend(o.fall.into_iter().map(|st| (HfJump::Brk(l.clone()), st)));
                out
            }
            Expr::Continue(c) => {
                let l = c.label.as_ref().map(|l| l.ident.to_string());
                HfOut {
                    jumps: ins
                        .iter()
                        .map(|&st| (HfJump::Cont(l.clone()), st))
                        .collect(),
                    ..Default::default()
                }
            }
            Expr::Macro(m) => self.mac(&m.mac, ins),
            Expr::Binary(b) if matches!(b.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
                let l = self.expr(&b.left, ins);
                let r = self.expr(&b.right, &l.fall);
                let mut out = HfOut {
                    fall: l.fall,
                    jumps: l.jumps,
                };
                out.fall.extend(r.fall);
                out.jumps.extend(r.jumps);
                out
            }
            Expr::Binary(b) => self.seq([&*b.left, &*b.right].into_iter(), ins),
            Expr::MethodCall(m) => {
                self.seq(std::iter::once(&*m.receiver).chain(m.args.iter()), ins)
            }
            Expr::Call(c) => self.seq(std::iter::once(&*c.func).chain(c.args.iter()), ins),
            Expr::Assign(a) => self.seq([&*a.left, &*a.right].into_iter(), ins),
            Expr::Unary(u) => self.expr(&u.expr, ins),
            Expr::Field(f) => self.expr(&f.base, ins),
            Expr::Index(i) => self.seq([&*i.expr, &*i.index].into_iter(), ins),
            Expr::Reference(r) => self.expr(&r.expr, ins),
            Expr::Paren(p) => self.expr(&p.expr, ins),
            Expr::Group(g) => self.expr(&g.expr, ins),
            Expr::Cast(c) => self.expr(&c.expr, ins),
            Expr::Await(a) => self.expr(&a.base, ins),
            Expr::Tuple(t) => self.seq(t.elems.iter(), ins),
            Expr::Array(a) => self.seq(a.elems.iter(), ins),
            Expr::Repeat(r) => self.expr(&r.expr, ins),
            Expr::Struct(s) => self.seq(
                s.fields.iter().map(|f| &f.expr).chain(s.rest.as_deref()),
                ins,
            ),
            Expr::Range(r) => self.seq(r.start.as_deref().into_iter().chain(r.end.as_deref()), ins),
            _ => pass(ins),
        }
    }
}

fn st_text(st: &HfSt) -> String {
    let n = match st.1 {
        2 => "2+".to_string(),
        n => n.to_string(),
    };
    format!("r{n}{}", if st.0 { "!" } else { "" })
}

/// One fn body for HF, keyed as the census keys it.
struct HfFn<'a> {
    file: String,
    qual: String,
    simple: String,
    line: usize,
    params: BTreeSet<String>,
    body: Option<&'a syn::Block>,
}

struct Pass3<'a, 'b> {
    file: String,
    stack: Vec<usize>,
    out: &'b mut Vec<HfFn<'a>>,
}

impl<'a> Pass3<'a, '_> {
    fn push(
        &mut self,
        attrs: &[syn::Attribute],
        sig: &syn::Signature,
        body: Option<&'a syn::Block>,
    ) -> bool {
        if has_cfg_test(attrs) {
            return false;
        }
        let simple = sig.ident.to_string();
        let qual = match self.stack.last() {
            Some(&o) => format!("{}::{}", self.out[o].qual, simple),
            None => simple.clone(),
        };
        let mut ps = Vec::new();
        for a in &sig.inputs {
            if let syn::FnArg::Typed(t) = a {
                pat_names(&t.pat, &mut ps);
            }
        }
        self.out.push(HfFn {
            file: self.file.clone(),
            qual,
            simple,
            line: 0,
            params: ps.into_iter().collect(),
            body,
        });
        self.stack.push(self.out.len() - 1);
        true
    }
}

impl<'a> Visit<'a> for Pass3<'a, '_> {
    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        if self.push(&f.attrs, &f.sig, Some(&f.block)) {
            visit::visit_item_fn(self, f);
            self.stack.pop();
        }
    }
    fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
        if self.push(&f.attrs, &f.sig, Some(&f.block)) {
            visit::visit_impl_item_fn(self, f);
            self.stack.pop();
        }
    }
    fn visit_trait_item_fn(&mut self, f: &'a syn::TraitItemFn) {
        if self.push(&f.attrs, &f.sig, f.default.as_ref()) {
            visit::visit_trait_item_fn(self, f);
            self.stack.pop();
        }
    }
    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        if !has_cfg_test(&i.attrs) {
            visit::visit_item_impl(self, i);
        }
    }
    fn visit_item_mod(&mut self, m: &'a syn::ItemMod) {
        if !has_cfg_test(&m.attrs) {
            visit::visit_item_mod(self, m);
        }
    }
}

fn hf_fns(srcs: &[Src]) -> Vec<HfFn<'_>> {
    let mut fns = Vec::new();
    for s in srcs {
        let mut recs = Vec::new();
        Pass3 {
            file: s.rel.clone(),
            stack: Vec::new(),
            out: &mut recs,
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
    fns
}

/// HF's rows: per site (key = file, fn, site ordinal) the computed columns, and per
/// (file, fn, callee, ARG) the count of `with_scope` / `elaborate_instance` calls.
struct HfCensus {
    sites: Vec<(String, String, usize)>,
    calls: BTreeMap<String, (usize, usize)>,
}

fn hf_census() -> HfCensus {
    let srcs = load_src();
    let fns = hf_fns(&srcs);
    let mut walks = Vec::new();
    for f in &fns {
        let Some(body) = f.body else { continue };
        let mut w = HfWalk {
            path: Vec::new(),
            loops: Vec::new(),
            closure: 0,
            names: f.params.clone(),
            block: Vec::new(),
            evs: Vec::new(),
            lexmod: in_lexscope(&f.file),
        };
        w.body_block(body);
        walks.push((f, w.evs));
    }
    // the fns that write cur_prefix (a call of one inside a region is a NEST)
    let writers: BTreeSet<&str> = walks
        .iter()
        .filter(|(_, evs)| {
            evs.iter().any(|a| {
                matches!(
                    a.ev,
                    HfEv::Open { .. }
                        | HfEv::Assign { .. }
                        | HfEv::Init { .. }
                        | HfEv::Other { .. }
                )
            })
        })
        .map(|(f, _)| f.simple.as_str())
        .collect();
    let mut sites = Vec::new();
    let mut calls: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (f, evs) in &walks {
        for a in evs {
            if let HfEv::Entry { name, arg } = &a.ev {
                let x = calls
                    .entry(format!("{}\t{}\t{name}\t{arg}", f.file, f.qual))
                    .or_insert((0, f.line));
                x.0 += 1;
            }
        }
        sites.extend(hf_sites(f, evs, &writers));
    }
    HfCensus { sites, calls }
}

/// The sites of one fn: a `mem::*` opener, a clone-save some assignment restores, a
/// struct-literal init, and one site for the writes no region covers.
fn hf_sites(
    f: &HfFn<'_>,
    evs: &[HfAt<'_>],
    writers: &BTreeSet<&str>,
) -> Vec<(String, String, usize)> {
    struct Site<'a> {
        ev: usize,
        form: &'static str,
        arg: String,
        bind: Option<String>,
        region: Option<(&'a [syn::Stmt], usize)>,
        bpath: Vec<Step>,
        at: usize,
    }
    let restored: BTreeSet<&str> = evs
        .iter()
        .filter_map(|a| match &a.ev {
            HfEv::Assign { ident: Some(i), .. } => Some(i.as_str()),
            _ => None,
        })
        .collect();
    let mut sites: Vec<Site<'_>> = Vec::new();
    for (k, a) in evs.iter().enumerate() {
        let (form, arg, bind, region) = match &a.ev {
            HfEv::Open {
                form,
                arg,
                bind,
                region,
            } => (*form, arg.clone(), bind.clone(), *region),
            HfEv::Clone { bind, region } if restored.contains(bind.as_str()) => {
                ("clone", "-".to_string(), Some(bind.clone()), *region)
            }
            HfEv::Init { arg } => ("init", arg.clone(), None, None),
            _ => continue,
        };
        let (bpath, at) = match a.pos.split_last() {
            Some((Step::S(i, _), rest)) if region.is_some() => (rest.to_vec(), *i),
            _ => (a.pos.clone(), 0),
        };
        sites.push(Site {
            ev: k,
            form,
            arg,
            bind,
            region,
            bpath,
            at,
        });
    }
    // is the event at `pos` inside the site's region (its block, after the opener)?
    let inside = |s: &Site<'_>, pos: &[Step]| -> bool {
        s.region.is_some()
            && pos.len() > s.bpath.len()
            && pos.starts_with(&s.bpath)
            && matches!(pos[s.bpath.len()], Step::S(j, _) if j > s.at)
    };
    let binds: BTreeSet<String> = sites.iter().filter_map(|s| s.bind.clone()).collect();
    let mut owner: Vec<Option<usize>> = vec![None; evs.len()];
    let mut stray = Vec::new();
    for (k, a) in evs.iter().enumerate() {
        let HfEv::Assign { ident, .. } = &a.ev else {
            if matches!(a.ev, HfEv::Other { .. }) {
                stray.push(k);
            }
            continue;
        };
        // the innermost region holding it: a restore names its binding ; a re-point
        // belongs to the innermost region
        let own = sites
            .iter()
            .enumerate()
            .filter(|(_, s)| s.ev < k && inside(s, &a.pos))
            .filter(|(_, s)| match ident {
                Some(i) if binds.contains(i) => s.bind.as_ref() == Some(i),
                _ => true,
            })
            .map(|(n, _)| n)
            .next_back();
        match own {
            Some(n) => owner[k] = Some(n),
            None => stray.push(k),
        }
    }
    let mut rows = Vec::new();
    for (n, s) in sites.iter().enumerate() {
        let mine: Vec<usize> = (0..evs.len()).filter(|&k| owner[k] == Some(n)).collect();
        let is_restore = |k: usize| matches!(&evs[k].ev, HfEv::Assign { ident: Some(i), .. } if s.bind.as_ref() == Some(i));
        let restores: Vec<usize> = mine.iter().copied().filter(|&k| is_restore(k)).collect();
        let repoints: Vec<usize> = mine.iter().copied().filter(|&k| !is_restore(k)).collect();
        let last = restores.last().copied().unwrap_or(s.ev);
        let rel = |k: usize| rel_text(&s.bpath, &evs[k].pos);
        let list = |v: Vec<String>| {
            if v.is_empty() {
                "-".to_string()
            } else {
                v.join(",")
            }
        };
        let restores_t = list(restores.iter().map(|&k| rel(k)).collect());
        let repoints_t = list(
            repoints
                .iter()
                .map(|&k| match &evs[k].ev {
                    HfEv::Assign { rhs, .. } => format!("{rhs}@{}", rel(k)),
                    _ => unreachable!(),
                })
                .collect(),
        );
        let same = if restores.is_empty() {
            "-"
        } else if restores
            .iter()
            .all(|&k| evs[k].pos.len() == s.bpath.len() + 1)
        {
            "yes"
        } else {
            "no"
        };
        // exits and calls between the opener and the last restore, inside the region
        let mut outs = Vec::new();
        let mut locals: BTreeMap<String, usize> = BTreeMap::new();
        let mut nest: BTreeMap<String, usize> = BTreeMap::new();
        for (k, a) in evs.iter().enumerate().take(last).skip(s.ev + 1) {
            if !inside(s, &a.pos) {
                continue;
            }
            match &a.ev {
                HfEv::Exit { kind, target } => {
                    let local = if a.closure > 0 {
                        Some("closure")
                    } else {
                        match target {
                            Some(t) if inside(s, t) => Some("local"),
                            _ => None,
                        }
                    };
                    match local {
                        Some(w) => *locals.entry(format!("{kind}({w})")).or_default() += 1,
                        None => outs.push(format!("{kind}@{}", rel(k))),
                    }
                }
                HfEv::Call { name }
                    if name.starts_with("dyn:") || writers.contains(name.as_str()) =>
                {
                    *nest.entry(name.clone()).or_default() += 1;
                }
                _ => {}
            }
        }
        let mut exits = outs;
        exits.extend(locals.iter().map(|(k, n)| format!("{k}x{n}")));
        let exits_t = list(exits);
        let nest_t = list(
            nest.iter()
                .map(|(k, n)| {
                    if *n == 1 {
                        k.clone()
                    } else {
                        format!("{k}x{n}")
                    }
                })
                .collect(),
        );
        let (paths_t, region_t, opener_t) = match (&s.region, &s.bind) {
            (Some((ss, i)), Some(b)) => {
                let others: BTreeSet<String> = binds.iter().filter(|x| *x != b).cloned().collect();
                let mut fl = HfFlow {
                    bind: b,
                    others: &others,
                    closure_writes: 0,
                };
                let start: HfSt = (s.form != "clone", 0);
                let o = fl.stmts(&ss[i + 1..], &BTreeSet::from([start]));
                let mut by: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
                for st in &o.fall {
                    by.entry("end").or_default().insert(st_text(st));
                }
                for (j, st) in &o.jumps {
                    let w = match j {
                        HfJump::Ret => "return",
                        HfJump::Try => "?",
                        HfJump::Panic => "panic",
                        HfJump::Brk(_) => "break",
                        HfJump::Cont(_) => "continue",
                    };
                    by.entry(w).or_default().insert(st_text(st));
                }
                let mut p = by
                    .iter()
                    .map(|(w, v)| {
                        format!("{w}={}", v.iter().cloned().collect::<Vec<_>>().join("|"))
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                if fl.closure_writes > 0 {
                    let _ = write!(p, ";closure-writes={}", fl.closure_writes);
                }
                (p, path_text(&s.bpath), format!("s{}", s.at))
            }
            (Some(_), None) => (
                "no-binding".into(),
                path_text(&s.bpath),
                format!("s{}", s.at),
            ),
            _ => ("-".into(), "-".into(), "-".into()),
        };
        rows.push((
            format!("{}\t{}\t{}", f.file, f.qual, n + 1),
            [
                s.form.to_string(),
                s.bind.clone().unwrap_or_else(|| "-".into()),
                s.arg.clone(),
                region_t,
                opener_t,
                restores_t,
                repoints_t,
                same.to_string(),
                exits_t,
                paths_t,
                nest_t,
            ]
            .join("\t"),
            f.line,
        ));
    }
    if !stray.is_empty() {
        let forms = stray
            .iter()
            .map(|&k| match &evs[k].ev {
                HfEv::Assign { rhs, .. } => format!("{rhs}@{}", path_text(&evs[k].pos)),
                HfEv::Other { form } => format!("{form}@{}", path_text(&evs[k].pos)),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>()
            .join(",");
        rows.push((
            format!("{}\t{}\tstray", f.file, f.qual),
            ["stray", "-", "-", "-", "-", "-", &forms, "-", "-", "-", "-"].join("\t"),
            f.line,
        ));
    }
    rows
}

#[test]
fn hf_the_write_sites_shapes_are_pinned() {
    let c = hf_census();
    let got: BTreeMap<String, String> = c
        .sites
        .iter()
        .map(|(k, v, _)| (k.clone(), v.clone()))
        .collect();
    let dumped: Vec<String> = c
        .sites
        .iter()
        .map(|(k, v, l)| format!("{k}\t{l}\t{v}"))
        .collect();
    dump(
        "HF",
        "file\tfn\tsite\tline\tform\tbind\targ\tblock\topener\trestores\trepoints\tsame\texits\tpaths\tnest",
        &dumped,
    );
    let calls_got: BTreeMap<String, String> = c
        .calls
        .iter()
        .map(|(k, (n, _))| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = c
        .calls
        .iter()
        .map(|(k, (n, l))| format!("{k}\t{l}\t{n}"))
        .collect();
    dump("HF_CALLS", "file\tfn\tcallee\targ\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut msgs = Vec::new();
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(&tables::get("HF")) {
        assert_eq!(
            row.len(),
            16,
            "HF table row has {} columns, not 16: {row:?}",
            row.len()
        );
        let k = row[..3].join("\t");
        let v = row[3..14].join("\t");
        let (form, same, exits, paths, nest, plan8, hand) = (
            &row[3], &row[10], &row[11], &row[12], &row[13], &row[14], &row[15],
        );
        let ctor = form == "init";
        let exit_class = if ctor {
            "-"
        } else if exits == "-" {
            "none"
        } else if exits.contains('@') {
            "out"
        } else {
            "local"
        };
        let once = if ctor {
            "-"
        } else if paths
            .split(';')
            .all(|p| p.split_once('=').is_some_and(|(_, s)| s == "r1"))
        {
            "yes"
        } else {
            "no"
        };
        // a row the syntax leaves open needs its hand reading
        let open = form == "clone"
            || nest.contains("dyn:")
            || exit_class == "out"
            || once == "no"
            || form == "stray";
        if open && hand == "-" {
            msgs.push(format!(
                "HF HAND {}: the row has a clone save, a dyn call, an exit out of the region or a path that does not \
                 restore once, and no HAND reading — read the site and write what the syntax does not decide",
                k.replace('\t', " · ")
            ));
        }
        for (col, val) in [
            ("same-block", same.as_str()),
            ("exit-between", exit_class),
            ("every-path-once", once),
            ("plan8", plan8.as_str()),
        ] {
            *by.entry(format!("{col}:{val}")).or_default() += 1;
        }
        want.insert(k, v);
    }
    msgs.extend(diff_rows(
        "HF",
        &got,
        &want,
        "a new write site of `cur_prefix` (or a site whose opener moved fn / ordinal): name the unit that adds it (LS2's \
         funnel takes every write), read what it sets the prefix to and whether every path restores it once, give it \
         its PLAN8 class and HAND reading, add the row (and H3's / HK's)",
        "a write site of `cur_prefix` is gone: name the unit, remove the row",
    ));
    let mut cwant = BTreeMap::new();
    for row in table(&tables::get("HF_CALLS")) {
        assert_eq!(
            row.len(),
            5,
            "HF_CALLS table row has {} columns, not 5: {row:?}",
            row.len()
        );
        *by.entry(format!("calls:{}", row[2])).or_default() +=
            row[4].parse::<usize>().expect("count");
        cwant.insert(row[..4].join("\t"), row[4].clone());
    }
    msgs.extend(diff_rows(
        "HF_CALLS",
        &calls_got,
        &cwant,
        "a new `with_scope` / `elaborate_instance` call, or a call's scope argument changed: name the unit, read the \
         argument's text (the scope it enters), add the row (and HK's)",
        "a `with_scope` / `elaborate_instance` call is gone or its argument changed: name the unit, remove the row",
    ));
    let counts = format!("sites {} {}", got.len(), dbg_counts(&by));
    if counts != tables::get("HF_COUNTS") {
        msgs.push(format!(
            "HF COUNTS table [{}] now [{counts}]",
            tables::get("HF_COUNTS")
        ));
    }
    finish("HF", &msgs);
}

// =====================================================================================
// The pinned tables
// =====================================================================================

// =====================================================================================
// HOUT where the display text and the child prefix go (LS1 part 4: DELTA9 J4, DELTA10 L5)
// =====================================================================================

/// The fns whose result is display / child-prefix text (DELTA9 J4 + DELTA10 L5's
/// `enter_frame_scope_root`).
///
/// LS2-d (DELTA10 L2, DELTA11 N-1 N-2): the six display fns and `enter_frame_scope_root` are
/// private to `lexscope` ; the outside calls the typed shims (`disp_here`, `disp_inst_top`,
/// `disp_chain_abs`, `ctx_here`, `disp_frame_path_of`, `disp_enter_frame_root`), LEAVE L4
/// `display_of_key_text`, the two fragment entries, the one text -> Shown entry
/// `shown_from_spelling` and the door's one text exit `scope_out` (a call or the fn as a value
/// handed to `map`) ; a `.plain()` call (no argument) is the exit's trait method, counted
/// beside them (0 outside the module: HC).
const HOUT_CALLEES: &[&str] = &[
    "display_prefix",
    "display_of",
    "scope_chain_abs",
    "frame_path",
    "frame_path_of",
    "cur_context",
    "child_prefix",
    "enter_frame_scope_root",
    "disp_here",
    "disp_inst_top",
    "disp_inst_top_text",
    "disp_chain_abs",
    "ctx_here",
    "disp_frame_path_of",
    "display_of_key_text",
    "stmt_loc_instance",
    "shown_from_spelling",
    "scope_out",
    // LS3b-2 (DELTA9 J4 (2)): the current scope as display / output text, and its message form.
    "cur_shown",
    "cur_shown_quoted_or",
];
/// LS2-d: the entries whose return is a SAVE of a display field (`SavedChain` / `SavedRoot`
/// / `SavedMethod`), with the field, and the restore entries, with theirs.
const HOUT_SAVE_ENTRIES: &[(&str, &str)] = &[
    ("disp_chain_take", "disp_chain"),
    ("disp_chain_swap_single", "disp_chain"),
    ("disp_root_swap_empty", "disp_root"),
    ("disp_enter_frame_root", "disp_root"),
    ("disp_root_swap_decl_inst", "disp_root"),
    ("disp_class_method_swap", "disp_method"),
];
const HOUT_RESTORE_ENTRIES: &[(&str, &str)] = &[
    ("disp_restore_chain", "disp_chain"),
    ("disp_restore_root", "disp_root"),
    ("disp_restore_method", "disp_method"),
];
/// The struct literals whose scope field is pinned, with that field.
const HOUT_LITS: &[(&str, &str)] = &[
    ("ProcIdent", "scope"),
    // LS3b-2: the elaborating row of `ProcIdent` (its scope a `Shown`).
    ("ProcIdentRow", "scope"),
    ("InstanceInfo", "path"),
    ("CovgInstMeta", "inst"),
];
/// A method that hands its receiver's value on unchanged: the sink is read through it.
const HOUT_THROUGH: &[&str] = &[
    "clone",
    "to_string",
    "to_owned",
    "into",
    "unwrap_or_default",
];
/// The `reported` set's type (edge_gate.rs, N-a).
const HOUT_SET: &str = "BTreeSet<(u32, u32, String)>";

/// Where a value goes: the first consumer above it (`&`, `( )`, `?`, a cast and the
/// HOUT_THROUGH methods are see-through).
#[derive(Clone)]
enum Sk {
    Local(String),
    Field(String),
    Arg(String),
    Recv(String),
    Assign(String),
    Ret,
    Mac(String, Box<Sk>),
    Bin(String),
    Stmt,
    Cond,
    Closure(Box<Sk>),
    Other(&'static str),
}

fn sk_text(s: &Sk, uses: &BTreeMap<String, usize>) -> String {
    match s {
        Sk::Local(n) => format!("LOCAL:{n}#{}", uses.get(n).copied().unwrap_or(0)),
        Sk::Field(f) => format!("FIELD:{f}"),
        Sk::Arg(c) => format!("ARG:{c}"),
        Sk::Recv(m) => format!("RECV:.{m}"),
        Sk::Assign(l) => format!("ASSIGN:{l}"),
        Sk::Ret => "RET".into(),
        Sk::Mac(m, outer) => format!("MAC:{m}>{}", sk_text(outer, uses)),
        Sk::Bin(op) => format!("BIN:{op}"),
        Sk::Stmt => "STMT".into(),
        Sk::Cond => "COND".into(),
        Sk::Closure(outer) => format!("CLOSURE>{}", sk_text(outer, uses)),
        Sk::Other(k) => format!("OTHER:{k}"),
    }
}

/// The direct child expressions of an expression (statements of its blocks included).
struct Kids<'a>(Vec<&'a Expr>);
impl<'a> Visit<'a> for Kids<'a> {
    fn visit_expr(&mut self, e: &'a Expr) {
        self.0.push(e);
    }
    fn visit_item(&mut self, _: &'a syn::Item) {}
}

/// An argument list's token text, `-` when empty.
fn hout_args<'a>(args: impl Iterator<Item = &'a Expr>) -> String {
    let v: Vec<String> = args.map(tx).collect();
    if v.is_empty() {
        "-".into()
    } else {
        v.join(", ")
    }
}

/// One fn's HOUT walk: its key, raw rows, display-field assigns, identifier counts.
type HoutFn = (
    String,
    Vec<HoutRaw>,
    Vec<(String, String)>,
    BTreeMap<String, usize>,
);

/// One HOUT row before its sink is rendered: (what, arg, sink).
type HoutRaw = (String, String, Sk);

struct HoutW {
    rows: Vec<HoutRaw>,
    /// the callees HBR / HC ARGS watch (empty for HOUT)
    watch: Vec<String>,
    /// each watched call: (callee, its argument tokens, its sink)
    watched: Vec<(String, Vec<String>, Sk)>,
    /// `<recv>.<display field> = <ident>`: (field, ident)
    assigns: Vec<(String, String)>,
    /// `let <ident> = <init>` (the init's token text)
    lets: BTreeMap<String, String>,
    m_path_idx: Option<usize>,
}

fn display_field_of(e: &Expr) -> Option<String> {
    // `<recv>.F`, or (LS2-d, the fields inside `lexscope::Scopes`) `<recv>.lex.F`
    let lex_base = |b: &Expr| {
        matches!(b, Expr::Field(l) if is_recv(&l.base)
            && matches!(&l.member, syn::Member::Named(n) if n == "lex"))
    };
    match e {
        Expr::Field(f) if is_recv(&f.base) || lex_base(&f.base) => match &f.member {
            syn::Member::Named(n) if DISPLAY_FIELDS.contains(&n.to_string().as_str()) => {
                Some(n.to_string())
            }
            _ => None,
        },
        Expr::Paren(p) => display_field_of(&p.expr),
        _ => None,
    }
}

fn mut_display_field(e: &Expr) -> Option<String> {
    match e {
        Expr::Reference(r) if r.mutability.is_some() => display_field_of(&r.expr),
        _ => None,
    }
}

impl HoutW {
    fn block(&mut self, b: &syn::Block, ctx: &Sk) {
        let n = b.stmts.len();
        for (i, st) in b.stmts.iter().enumerate() {
            match st {
                syn::Stmt::Local(l) => self.local(l),
                syn::Stmt::Expr(e, semi) => {
                    let c = if semi.is_none() && i + 1 == n {
                        ctx.clone()
                    } else {
                        Sk::Stmt
                    };
                    self.expr(e, &c);
                }
                syn::Stmt::Macro(m) => {
                    let name = m
                        .mac
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default();
                    for a in mac_exprs(&m.mac) {
                        self.expr(&a, &Sk::Mac(name.clone(), Box::new(Sk::Stmt)));
                    }
                }
                syn::Stmt::Item(_) => {}
            }
        }
    }

    fn local(&mut self, l: &syn::Local) {
        let Some(init) = &l.init else { return };
        let pat = match &l.pat {
            Pat::Type(t) => &*t.pat,
            p => p,
        };
        match (pat, &*init.expr) {
            (Pat::Ident(i), e) => {
                self.lets.insert(i.ident.to_string(), tx(e));
                self.expr(e, &Sk::Local(i.ident.to_string()));
            }
            (Pat::Tuple(pt), Expr::Tuple(et)) if pt.elems.len() == et.elems.len() => {
                for (p, e) in pt.elems.iter().zip(et.elems.iter()) {
                    match p {
                        Pat::Ident(i) => {
                            self.lets.insert(i.ident.to_string(), tx(e));
                            self.expr(e, &Sk::Local(i.ident.to_string()));
                        }
                        _ => self.expr(e, &Sk::Other("let-pattern")),
                    }
                }
            }
            (_, e) => self.expr(e, &Sk::Other("let-pattern")),
        }
        if let Some((_, d)) = &init.diverge {
            self.expr(d, &Sk::Other("let-else"));
        }
    }

    fn expr(&mut self, e: &Expr, ctx: &Sk) {
        match e {
            Expr::MethodCall(m) => {
                let name = m.method.to_string();
                let args = hout_args(m.args.iter());
                if self.watch.contains(&name) {
                    self.watched
                        .push((name.clone(), m.args.iter().map(tx).collect(), ctx.clone()));
                }
                if HOUT_CALLEES.contains(&name.as_str()) {
                    self.rows.push((name.clone(), args.clone(), ctx.clone()));
                }
                if name == "plain" && m.args.is_empty() {
                    self.rows.push(("plain".into(), "-".into(), ctx.clone()));
                }
                if let Some((_, f)) = HOUT_SAVE_ENTRIES.iter().find(|(n, _)| *n == name) {
                    self.rows
                        .push((format!("SAVE:{f}"), format!(".{name}({args})"), ctx.clone()));
                }
                if let Some((_, f)) = HOUT_RESTORE_ENTRIES.iter().find(|(n, _)| *n == name) {
                    if let [Expr::Path(p)] = m.args.iter().collect::<Vec<_>>().as_slice() {
                        if let Some(i) = p.path.get_ident() {
                            self.assigns.push((f.to_string(), i.to_string()));
                        }
                    }
                }
                if let Some(f) = display_field_of(&m.receiver) {
                    if name == "replace" || name == "take" {
                        self.rows.push((
                            format!("SAVE:{f}"),
                            format!(".{name}({args})"),
                            ctx.clone(),
                        ));
                    }
                }
                if name == "push_func" {
                    let arg = self
                        .m_path_idx
                        .and_then(|i| m.args.iter().nth(i))
                        .map(|a| {
                            let t = tx(a);
                            match self.lets.get(&t) {
                                Some(init) => format!("{t} <- {init}"),
                                None => t,
                            }
                        })
                        .unwrap_or_else(|| "?".into());
                    self.rows
                        .push(("push_func.m_path".into(), arg, ctx.clone()));
                }
                let rc = if HOUT_THROUGH.contains(&name.as_str()) {
                    ctx.clone()
                } else {
                    Sk::Recv(name.clone())
                };
                self.expr(&m.receiver, &rc);
                let ac = Sk::Arg(format!("{}.{name}", tx(&m.receiver)));
                for a in &m.args {
                    self.expr(a, &ac);
                }
            }
            Expr::Call(c) => {
                let callee = tx(&c.func);
                let last = callee.rsplit("::").next().unwrap_or("").to_string();
                let args = hout_args(c.args.iter());
                if self.watch.contains(&last) {
                    self.watched
                        .push((last.clone(), c.args.iter().map(tx).collect(), ctx.clone()));
                }
                if HOUT_CALLEES.contains(&last.as_str()) {
                    self.rows.push((last.clone(), args.clone(), ctx.clone()));
                }
                if (last == "take" || last == "replace") && callee.contains("mem") {
                    if let Some(f) = c.args.first().and_then(mut_display_field) {
                        self.rows.push((
                            format!("SAVE:{f}"),
                            format!("mem::{last}({args})"),
                            ctx.clone(),
                        ));
                    }
                }
                self.expr(&c.func, &Sk::Other("callee"));
                // a wrapper constructor hands its argument on: the sink is read through it
                let through = matches!(callee.as_str(), "Some" | "Ok" | "Err" | "Box::new");
                for a in &c.args {
                    if through {
                        self.expr(a, ctx);
                    } else {
                        self.expr(a, &Sk::Arg(callee.clone()));
                    }
                }
            }
            Expr::Struct(s) => {
                let name = s
                    .path
                    .segments
                    .last()
                    .map(|x| x.ident.to_string())
                    .unwrap_or_default();
                if let Some((_, field)) = HOUT_LITS.iter().find(|(n, _)| *n == name) {
                    let init = s
                        .fields
                        .iter()
                        .find(|fv| matches!(&fv.member, syn::Member::Named(i) if i == field))
                        .map(|fv| format!("{field}={}", tx(&fv.expr)))
                        .unwrap_or_else(|| format!("{field}=?"));
                    self.rows.push((format!("LIT:{name}"), init, ctx.clone()));
                }
                for fv in &s.fields {
                    let m = match &fv.member {
                        syn::Member::Named(i) => i.to_string(),
                        syn::Member::Unnamed(i) => i.index.to_string(),
                    };
                    self.expr(&fv.expr, &Sk::Field(format!("{name}.{m}")));
                }
                if let Some(r) = &s.rest {
                    self.expr(r, &Sk::Other("struct-base"));
                }
            }
            Expr::Macro(m) => {
                let name = m
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                for a in mac_exprs(&m.mac) {
                    self.expr(&a, &Sk::Mac(name.clone(), Box::new(ctx.clone())));
                }
            }
            Expr::Assign(a) => {
                if let (Some(f), Expr::Path(p)) = (display_field_of(&a.left), &*a.right) {
                    if p.path.segments.len() == 1 {
                        self.assigns.push((f, tx(&a.right)));
                    }
                }
                self.expr(&a.left, &Sk::Other("lhs"));
                self.expr(&a.right, &Sk::Assign(tx(&a.left)));
            }
            Expr::Reference(r) => self.expr(&r.expr, ctx),
            Expr::Paren(p) => self.expr(&p.expr, ctx),
            Expr::Group(g) => self.expr(&g.expr, ctx),
            Expr::Try(t) => self.expr(&t.expr, ctx),
            Expr::Cast(c) => self.expr(&c.expr, ctx),
            Expr::Binary(b) => {
                let op = match b.op {
                    syn::BinOp::Eq(_) => "==",
                    syn::BinOp::Ne(_) => "!=",
                    syn::BinOp::Add(_) | syn::BinOp::AddAssign(_) => "+",
                    syn::BinOp::And(_) => "&&",
                    syn::BinOp::Or(_) => "||",
                    _ => "op",
                };
                self.expr(&b.left, &Sk::Bin(op.into()));
                self.expr(&b.right, &Sk::Bin(op.into()));
            }
            Expr::Block(b) => self.block(&b.block, ctx),
            Expr::Unsafe(b) => self.block(&b.block, ctx),
            Expr::If(i) => {
                self.expr(&i.cond, &Sk::Cond);
                self.block(&i.then_branch, ctx);
                if let Some((_, el)) = &i.else_branch {
                    self.expr(el, ctx);
                }
            }
            Expr::Match(m) => {
                self.expr(&m.expr, &Sk::Cond);
                for arm in &m.arms {
                    if let Some((_, g)) = &arm.guard {
                        self.expr(g, &Sk::Cond);
                    }
                    self.expr(&arm.body, ctx);
                }
            }
            Expr::Let(l) => self.expr(&l.expr, &Sk::Cond),
            Expr::Return(r) => {
                if let Some(x) = &r.expr {
                    self.expr(x, &Sk::Ret);
                }
            }
            Expr::Closure(c) => self.expr(&c.body, &Sk::Closure(Box::new(ctx.clone()))),
            Expr::Field(f) => self.expr(
                &f.base,
                &Sk::Recv(format!(
                    "field {}",
                    match &f.member {
                        syn::Member::Named(i) => i.to_string(),
                        syn::Member::Unnamed(i) => i.index.to_string(),
                    }
                )),
            ),
            Expr::ForLoop(f) => {
                self.expr(&f.expr, &Sk::Other("for-iter"));
                self.block(&f.body, &Sk::Stmt);
            }
            Expr::While(w) => {
                self.expr(&w.cond, &Sk::Cond);
                self.block(&w.body, &Sk::Stmt);
            }
            Expr::Loop(l) => self.block(&l.body, &Sk::Stmt),
            Expr::Tuple(t) => {
                for x in &t.elems {
                    self.expr(x, &Sk::Other("tuple"));
                }
            }
            // LS2-d: a watched fn handed on as a value (`.map(scope_out)`), not in a callee
            // position (a call's callee is walked with `Other("callee")`)
            Expr::Path(p) if !matches!(ctx, Sk::Other("callee")) => {
                let last = p
                    .path
                    .segments
                    .last()
                    .map(|x| x.ident.to_string())
                    .unwrap_or_default();
                if p.path.segments.len() == 1 && last == "scope_out" {
                    self.rows.push((last, "fn-value".into(), ctx.clone()));
                }
            }
            other => {
                let mut k = Kids(Vec::new());
                visit::visit_expr(&mut k, other);
                for x in k.0 {
                    self.expr(x, &Sk::Other("expr"));
                }
            }
        }
    }
}

/// Every identifier's token count in a body (expression paths, bindings, macro tokens).
struct IdentCount(BTreeMap<String, usize>);
impl<'a> Visit<'a> for IdentCount {
    fn visit_expr_path(&mut self, p: &'a syn::ExprPath) {
        if let Some(i) = p.path.get_ident() {
            *self.0.entry(i.to_string()).or_default() += 1;
        }
        visit::visit_expr_path(self, p);
    }
    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        *self.0.entry(p.ident.to_string()).or_default() += 1;
        visit::visit_pat_ident(self, p);
    }
    fn visit_macro(&mut self, m: &'a syn::Macro) {
        fn walk(ts: proc_macro2::TokenStream, out: &mut BTreeMap<String, usize>) {
            for t in ts {
                match t {
                    proc_macro2::TokenTree::Ident(i) => *out.entry(i.to_string()).or_default() += 1,
                    proc_macro2::TokenTree::Group(g) => walk(g.stream(), out),
                    _ => {}
                }
            }
        }
        walk(m.tokens.clone(), &mut self.0);
    }
    fn visit_item(&mut self, _: &'a syn::Item) {}
}

/// HOUT's census: (file, fn, what, arg, sink) -> count, and the line of each fn.
fn hout_census() -> (BTreeMap<String, usize>, BTreeMap<String, usize>) {
    let srcs = load_src();
    let fns = hf_fns(&srcs);
    // push_func's `m_path` position among its arguments (from its definition)
    let mut m_path_idx = None;
    for s in &srcs {
        struct Def<'o>(&'o mut Option<usize>);
        impl<'a> Visit<'a> for Def<'_> {
            fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
                if f.sig.ident == "push_func" {
                    *self.0 = f
                        .sig
                        .inputs
                        .iter()
                        .filter(|a| matches!(a, syn::FnArg::Typed(_)))
                        .position(|a| match a {
                            syn::FnArg::Typed(t) => {
                                matches!(&*t.pat, Pat::Ident(i) if i.ident == "m_path")
                            }
                            _ => false,
                        });
                }
            }
        }
        Def(&mut m_path_idx).visit_file(&s.file);
    }
    assert!(
        m_path_idx.is_some(),
        "HOUT: push_func's `m_path` parameter is not found"
    );
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    let mut lines: BTreeMap<String, usize> = BTreeMap::new();
    let mut ret_savers: BTreeMap<String, String> = BTreeMap::new();
    let mut per_fn: Vec<HoutFn> = Vec::new();
    for f in &fns {
        let Some(body) = f.body else { continue };
        let mut w = HoutW {
            rows: Vec::new(),
            watch: Vec::new(),
            watched: Vec::new(),
            assigns: Vec::new(),
            lets: BTreeMap::new(),
            m_path_idx,
        };
        w.block(body, &Sk::Ret);
        let mut ic = IdentCount(BTreeMap::new());
        ic.visit_block(body);
        let k = format!("{}\t{}", f.file, f.qual);
        for (what, _, sk) in &w.rows {
            if let (Some(fl), Sk::Ret) = (what.strip_prefix("SAVE:"), sk) {
                ret_savers.insert(f.simple.clone(), fl.to_string());
            }
        }
        lines.insert(k.clone(), f.line);
        per_fn.push((k, w.rows, w.assigns, ic.0));
    }
    for (k, raw, assigns, uses) in &per_fn {
        let mut bound: BTreeMap<String, String> = BTreeMap::new();
        let mut out: Vec<(String, String, String)> = Vec::new();
        for (what, arg, sk) in raw {
            out.push((what.clone(), arg.clone(), sk_text(sk, uses)));
            let field = match what.strip_prefix("SAVE:") {
                Some(f) => Some(f.to_string()),
                None => ret_savers.get(what).map(|f| {
                    out.push((
                        format!("SAVE:{f}"),
                        format!("via {what}({arg})"),
                        sk_text(sk, uses),
                    ));
                    f.clone()
                }),
            };
            if let (Some(f), Sk::Local(n)) = (field, sk) {
                bound.insert(n.clone(), f);
            }
        }
        for (f, ident) in assigns {
            if let Some(from) = bound.get(ident) {
                let form = if from == f {
                    format!("= {ident}")
                } else {
                    format!("= {ident} (saved from {from})")
                };
                out.push((format!("RESTORE:{f}"), form, "-".into()));
            }
        }
        for (what, arg, sink) in out {
            *rows
                .entry(format!("{k}\t{what}\t{arg}\t{sink}"))
                .or_default() += 1;
        }
    }
    // the `reported` set's declarations: a `let` or a parameter of its type
    for f in &fns {
        let k = format!("{}\t{}", f.file, f.qual);
        let Some(body) = f.body else { continue };
        struct Sets(Vec<(String, String)>);
        impl<'a> Visit<'a> for Sets {
            fn visit_local(&mut self, l: &'a syn::Local) {
                if let Pat::Type(t) = &l.pat {
                    if type_str(&t.ty) == HOUT_SET {
                        let mut n = Vec::new();
                        pat_names(&t.pat, &mut n);
                        self.0.push(("let".into(), n.join(",")));
                    }
                }
                visit::visit_local(self, l);
            }
            fn visit_item(&mut self, _: &'a syn::Item) {}
        }
        let mut s = Sets(Vec::new());
        s.visit_block(body);
        for (kind, name) in s.0 {
            *rows
                .entry(format!("{k}\tSET\t{kind} {name}\t-"))
                .or_default() += 1;
        }
    }
    for s in &srcs {
        struct Params<'o> {
            file: &'o str,
            out: &'o mut BTreeMap<String, usize>,
            fns: &'o [HfFn<'o>],
        }
        impl<'a> Visit<'a> for Params<'_> {
            fn visit_signature(&mut self, sig: &'a syn::Signature) {
                for a in &sig.inputs {
                    if let syn::FnArg::Typed(t) = a {
                        let ty = type_str(&t.ty);
                        let bare = ty.trim_start_matches("&mut ").trim_start_matches('&');
                        if bare == HOUT_SET {
                            let mut n = Vec::new();
                            pat_names(&t.pat, &mut n);
                            let quals: Vec<&HfFn<'_>> = self
                                .fns
                                .iter()
                                .filter(|f| f.file == self.file && sig.ident == f.simple)
                                .collect();
                            let q = match quals.as_slice() {
                                [one] => one.qual.clone(),
                                _ => sig.ident.to_string(),
                            };
                            *self
                                .out
                                .entry(format!("{}\t{q}\tSET\tparam {}\t-", self.file, n.join(",")))
                                .or_default() += 1;
                        }
                    }
                }
            }
        }
        Params {
            file: &s.rel,
            out: &mut rows,
            fns: &fns,
        }
        .visit_file(&s.file);
    }
    (rows, lines)
}

#[test]
fn hout_the_display_outputs_are_pinned() {
    let (rows, lines) = hout_census();
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows
        .iter()
        .map(|(k, n)| {
            let fk: String = k.split('\t').take(2).collect::<Vec<_>>().join("\t");
            format!("{k}\t{}\t{n}", lines.get(&fk).copied().unwrap_or(0))
        })
        .collect();
    dump("HOUT", "file\tfn\twhat\targ\tsink\tline\tcount", &dumped);
    let mut want = BTreeMap::new();
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for row in table(&tables::get("HOUT")) {
        assert_eq!(
            row.len(),
            6,
            "HOUT table row has {} columns, not 6: {row:?}",
            row.len()
        );
        let n: usize = row[5].parse().expect("count");
        let what = row[2].split(':').next().unwrap_or("").to_string();
        *by.entry(what).or_default() += n;
        want.insert(row[..5].join("\t"), n.to_string());
    }
    let mut msgs = diff_rows(
        "HOUT",
        &got,
        &want,
        "a new place display / child-prefix text goes (a call, a literal, a save or restore of a display field, a \
         push_func `m_path`, the `reported` set): name the unit — LS2-d / LS3 retype these sinks, HC's SINK column \
         takes the row — and add it",
        "a display output is gone: name the unit that removed it, remove the row",
    );
    let counts = dbg_counts(&by);
    if counts != tables::get("HOUT_COUNTS") {
        msgs.push(format!(
            "HOUT COUNTS table [{}] now [{counts}]",
            tables::get("HOUT_COUNTS")
        ));
    }
    // HC (LS2-d ; DELTA9 J1 (2), DELTA11 N-2 (a)): the door's text exit by its sink. The LOCAL
    // rows of `scope_out` are pinned 5 — the four `let context` of driver.rs's error / note_at /
    // warn_code / warn_code_at and rtl_diag's tuple `let` — each local used once more, as a
    // Diagnostic's `context` ; `plain`, the exit's trait method, has no call outside lexscope.
    let mut hc_local = 0usize;
    let mut plain_out = 0usize;
    for (k, n) in &rows {
        let c: Vec<&str> = k.split('\t').collect();
        if c.len() == 5 && c[2] == "scope_out" && c[4].starts_with("LOCAL:") {
            hc_local += n;
        }
        if c.len() == 5 && c[2] == "plain" && !c[0].starts_with("lexscope/") {
            plain_out += n;
        }
    }
    let hc = format!("scope_out_local={hc_local} plain_outside={plain_out}");
    if hc != tables::get("HC_LOCAL") {
        msgs.push(format!(
            "HC LOCAL table [{}] now [{hc}] — a LOCAL sink of `scope_out` other than the five pinned \
             `context` locals, or a call of `plain` outside lexscope, is text leaving the door \
             unpinned: put the call in its sink's argument, or name the unit and re-pin",
            tables::get("HC_LOCAL")
        ));
    }
    finish("HOUT", &msgs);
}

// =====================================================================================
// HDBG, HBR, HC ARGS — the funnel's own pins (§5.2 Q-LEXSCOPE LS2a: DELTA9 J3 N-8, DELTA10
// N-g / w1, DELTA11 R4 N-8, DELTA12 S2 r4)
// =====================================================================================

/// The sections of `scope_surface.tables` these three are keyed by (LEAVE, ENTER): the
/// module's own lists, so an entry added there is watched here with no edit.
const SURF_DATA: &str = include_str!("scope_surface.tables");

fn surf_rows(name: &str) -> Vec<Vec<String>> {
    let open = format!("== {name}");
    let body: Vec<&str> = SURF_DATA
        .lines()
        .skip_while(|l| l.trim_end() != open)
        .skip(1)
        .take_while(|l| !l.starts_with("== "))
        .collect();
    assert!(
        !body.is_empty(),
        "scope_surface.tables has no section `{name}`"
    );
    table(&body.join("\n"))
}

/// The simple name of a surface item (`Scopes::scope_swap_text` -> `scope_swap_text`).
fn item_simple(item: &str) -> String {
    item.rsplit("::").next().unwrap_or(item).to_string()
}

/// Every call of a watched name in every fn with a body: (file, fn, callee, args, sink).
fn watched_calls(watch: &[String]) -> Vec<(String, String, String, Vec<String>, String)> {
    let srcs = load_src();
    let fns = hf_fns(&srcs);
    let mut out = Vec::new();
    for f in &fns {
        let Some(body) = f.body else { continue };
        let mut w = HoutW {
            rows: Vec::new(),
            watch: watch.to_vec(),
            watched: Vec::new(),
            assigns: Vec::new(),
            lets: BTreeMap::new(),
            m_path_idx: None,
        };
        w.block(body, &Sk::Ret);
        let mut ic = IdentCount(BTreeMap::new());
        ic.visit_block(body);
        for (callee, args, sk) in w.watched {
            out.push((
                f.file.clone(),
                f.qual.clone(),
                callee,
                args,
                sk_text(&sk, &ic.0),
            ));
        }
    }
    out
}

fn lexscope_texts() -> Vec<(String, String)> {
    let dir = src_dir().join("lexscope");
    let mut paths = Vec::new();
    if dir.is_dir() {
        rs_files(&dir, &mut paths);
    }
    paths
        .iter()
        .map(|p| {
            let rel = p.strip_prefix(src_dir()).unwrap_or(p).display().to_string();
            (rel, std::fs::read_to_string(p).expect("read lexscope file"))
        })
        .collect()
}

fn count_ident(ts: proc_macro2::TokenStream, name: &str) -> usize {
    ts.into_iter()
        .map(|t| match t {
            proc_macro2::TokenTree::Ident(i) => usize::from(i == name),
            proc_macro2::TokenTree::Group(g) => count_ident(g.stream(), name),
            _ => 0,
        })
        .sum()
}

/// HDBG: (the identifier `debug_assertions`'s token count in lexscope's files, every file of
/// the directory, tests included ; the `debug_assert*!` calls by (file, fn, macro)).
fn hdbg_census() -> (usize, BTreeMap<String, usize>) {
    let mut idents = 0;
    for (rel, text) in lexscope_texts() {
        let ts: proc_macro2::TokenStream = text
            .parse()
            .unwrap_or_else(|e| panic!("HDBG cannot lex {rel}: {e}"));
        idents += count_ident(ts, "debug_assertions");
    }
    let srcs = load_src();
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for f in hf_fns(&srcs) {
        if !in_lexscope(&f.file) {
            continue;
        }
        let Some(body) = f.body else { continue };
        struct Dbg(BTreeMap<String, usize>);
        impl<'a> Visit<'a> for Dbg {
            fn visit_macro(&mut self, m: &'a syn::Macro) {
                if let Some(l) = m.path.segments.last() {
                    let n = l.ident.to_string();
                    if n.starts_with("debug_assert") {
                        *self.0.entry(n).or_default() += 1;
                    }
                }
            }
            fn visit_item(&mut self, _: &'a syn::Item) {}
        }
        let mut d = Dbg(BTreeMap::new());
        d.visit_block(body);
        for (m, n) in d.0 {
            *rows
                .entry(format!("{}\t{}\t{m}", f.file, f.qual))
                .or_default() += n;
        }
    }
    (idents, rows)
}

#[test]
fn hdbg_the_debug_only_code_is_pinned() {
    let (idents, rows) = hdbg_census();
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HDBG", "file\tfn\tmacro\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HDBG")) {
        assert_eq!(
            row.len(),
            4,
            "HDBG table row has {} columns, not 4: {row:?}",
            row.len()
        );
        want.insert(row[..3].join("\t"), row[3].clone());
    }
    let mut msgs = Vec::new();
    if idents != 0 {
        msgs.push(format!(
            "HDBG the identifier `debug_assertions` occurs {idents} time(s) in lexscope's files: lexscope has no \
             representation or code that differs by profile but `debug_assert*!` statements (DELTA9 N-8)"
        ));
    }
    msgs.extend(diff_rows(
        "HDBG",
        &got,
        &want,
        "a new `debug_assert*!` in lexscope: read its ARGUMENT for a side effect (no test sees one), add the row",
        "a `debug_assert*!` of lexscope is gone: name the unit, remove the row",
    ));
    let counts = format!("ident=0 asserts={}", rows.values().sum::<usize>());
    if counts != tables::get("HDBG_COUNTS") {
        msgs.push(format!(
            "HDBG COUNTS table [{}] now [{counts}]",
            tables::get("HDBG_COUNTS")
        ));
    }
    finish("HDBG", &msgs);
}

#[test]
fn hbr_the_bridge_reads_are_pinned() {
    // the bridge entries = LEAVE's items (L1 the text bridge, L2 / L3 residue R, L4 the key
    // text window w6 from LS2-d) ; not L0, the BOUNDARY `scope_out` / `Outgoing::plain`, whose
    // call sites are HC's (HOUT's rows with their SINK and HC_LOCAL ; DELTA10 L1)
    let mut entries: Vec<String> = surf_rows("SURF_LEAVE")
        .iter()
        .filter(|r| r[0] != "L0")
        .map(|r| item_simple(&r[1]))
        .collect();
    entries.sort();
    entries.dedup();
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for (file, qual, callee, _, sink) in watched_calls(&entries) {
        if in_lexscope(&file) {
            continue;
        }
        *rows
            .entry(format!("{file}\t{qual}\t{callee}\t{sink}"))
            .or_default() += 1;
    }
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HBR", "file\tfn\tentry\tsink\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HBR")) {
        assert_eq!(
            row.len(),
            5,
            "HBR table row has {} columns, not 5: {row:?}",
            row.len()
        );
        want.insert(row[..4].join("\t"), row[4].clone());
    }
    let mut msgs = diff_rows(
        "HBR",
        &got,
        &want,
        "a new read through a bridge (LEAVE L1-L3): TW-BR — the bridge may only fall ; name the unit, read where \
         the text goes (SINK), add the row",
        "a bridge read is gone: name the unit, remove the row (L1 falls at LS3, 0 at DK1)",
    );
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for (k, n) in &rows {
        *by.entry(k.split('\t').nth(2).unwrap_or("").to_string())
            .or_default() += n;
    }
    let counts = dbg_counts(&by);
    if counts != tables::get("HBR_COUNTS") {
        msgs.push(format!(
            "HBR COUNTS table [{}] now [{counts}]",
            tables::get("HBR_COUNTS")
        ));
    }
    finish("HBR", &msgs);
}

#[test]
fn hc_carry_the_carried_scope_sinks_are_pinned() {
    // LS3 (PLAN8 H2 (c)): every call of `scope_carry` outside lexscope by (file, fn, SINK) with a
    // count ; a carry bound by `let` (SINK LOCAL:..) must be a row of HC_CARRY_LOCAL, and the
    // LOCAL count equals that table's (W12: it may only fall). Until §5.2 Q-DECLKEY DK1-0 the pin
    // was 0 in code ; DK1-0 names two (the parent scope taken before a child instance is entered,
    // re-entered by `scope_swap` in wire_ports: DELTA14 D8 q2, R-F2, R-F10, premise10 N10-3)
    let entries = vec!["scope_carry".to_string()];
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for (file, qual, callee, _, sink) in watched_calls(&entries) {
        if in_lexscope(&file) {
            continue;
        }
        *rows
            .entry(format!("{file}\t{qual}\t{callee}\t{sink}"))
            .or_default() += 1;
    }
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HC_CARRY", "file\tfn\tentry\tsink\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HC_CARRY")) {
        assert_eq!(
            row.len(),
            5,
            "HC_CARRY table row has {} columns, not 5: {row:?}",
            row.len()
        );
        want.insert(row[..4].join("\t"), row[4].clone());
    }
    let mut msgs = diff_rows(
        "HC_CARRY",
        &got,
        &want,
        "a new carry of the current scope: read where the value goes (SINK: a record field or an argument), name \
         the unit, add the row",
        "a carry of the current scope is gone: name the unit, remove the row",
    );
    let sites: usize = rows.values().sum();
    let local: usize = rows
        .iter()
        .filter(|(k, _)| {
            k.split('\t')
                .nth(3)
                .is_some_and(|s| s.starts_with("LOCAL:"))
        })
        .map(|(_, n)| n)
        .sum();
    // the admitted LOCAL carries (HC_CARRY_LOCAL): a LOCAL row off it fails whatever HC_CARRY says
    let mut local_want: BTreeMap<String, String> = BTreeMap::new();
    for row in table(&tables::get("HC_CARRY_LOCAL")) {
        assert_eq!(
            row.len(),
            5,
            "HC_CARRY_LOCAL table row has {} columns, not 5: {row:?}",
            row.len()
        );
        assert!(
            row[3].starts_with("LOCAL:"),
            "HC_CARRY_LOCAL row's SINK is not LOCAL:..: {row:?}"
        );
        local_want.insert(row[..4].join("\t"), row[4].clone());
    }
    let local_got: BTreeMap<String, String> = got
        .iter()
        .filter(|(k, _)| {
            k.split('\t')
                .nth(3)
                .is_some_and(|s| s.starts_with("LOCAL:"))
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (k, v) in &local_got {
        if local_want.get(k) != Some(v) {
            msgs.push(format!(
                "HC_CARRY a `scope_carry` bound by `let` off HC_CARRY_LOCAL: {} [{v}] — LOCAL carry rows are \
                 pinned to the {} rows of HC_CARRY_LOCAL (PLAN8 H2 (c), W12, DK1-0) — pass the carry as the argument \
                 or the field it is for",
                k.replace('\t', " · "),
                local_want.len()
            ));
        }
    }
    for k in local_want.keys() {
        if !local_got.contains_key(k) {
            msgs.push(format!(
                "HC_CARRY_LOCAL STALE {} — the admitted LOCAL carry is gone: remove the row (W12: the list may only fall)",
                k.replace('\t', " · ")
            ));
        }
    }
    let local_listed: usize = local_want
        .values()
        .map(|n| n.parse::<usize>().expect("HC_CARRY_LOCAL count"))
        .sum();
    if local != local_listed {
        msgs.push(format!(
            "HC_CARRY a `scope_carry` bound by `let` ({local}): LOCAL carry rows are pinned to HC_CARRY_LOCAL's \
             {local_listed} (PLAN8 H2 (c), W12, DK1-0)"
        ));
    }
    let counts = format!("sites={sites} local={local}");
    if counts != tables::get("HC_CARRY_COUNTS") {
        msgs.push(format!(
            "HC_CARRY COUNTS table [{}] now [{counts}]",
            tables::get("HC_CARRY_COUNTS")
        ));
    }
    finish("HC_CARRY", &msgs);
}

#[test]
fn hc_scopeq_the_scope_question_askers_are_pinned() {
    // LS4 (PLAN7 LS4 "HC is extended to these entries (file, fn, entry, count)" ; W12 gains the
    // scope-question entries with deleting unit B3k): W12_SCOPEQ lists the entries, each a
    // surface fn of lexscope returning bool (SURF_ROWS, itself pinned to the code by HT-SURF) ;
    // HC_SCOPEQ pins every call of one outside lexscope by (file, fn, entry) with a count
    let w12 = table(&tables::get("W12_SCOPEQ"));
    let surf = surf_rows("SURF_ROWS");
    let mut msgs = Vec::new();
    let mut entries = Vec::new();
    for row in &w12 {
        assert_eq!(
            row.len(),
            4,
            "W12_SCOPEQ table row has {} columns, not 4: {row:?}",
            row.len()
        );
        let rets: Vec<&Vec<String>> = surf
            .iter()
            .filter(|r| r.len() > 5 && r[3] == row[1] && r[4] == "RET")
            .collect();
        if rets.len() != 1 || rets[0][5] != "bool" {
            msgs.push(format!(
                "W12_SCOPEQ entry `{}` is not ONE surface fn returning bool in SURF_ROWS (RET rows: {rets:?}) — a \
                 scope question answers with a bool, no text (PLAN7 LS4)",
                row[1]
            ));
        }
        if row[3] != "B3k" {
            msgs.push(format!(
                "W12_SCOPEQ entry `{}` has deleting unit `{}`, not B3k (DELTA6 W12, PLAN7 LS4)",
                row[1], row[3]
            ));
        }
        entries.push(row[0].clone());
    }
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for (file, qual, callee, _, _) in watched_calls(&entries) {
        if in_lexscope(&file) {
            continue;
        }
        *rows.entry(format!("{file}\t{qual}\t{callee}")).or_default() += 1;
    }
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HC_SCOPEQ", "file\tfn\tentry\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HC_SCOPEQ")) {
        assert_eq!(
            row.len(),
            4,
            "HC_SCOPEQ table row has {} columns, not 4: {row:?}",
            row.len()
        );
        want.insert(row[..3].join("\t"), row[3].clone());
    }
    msgs.extend(diff_rows(
        "HC_SCOPEQ",
        &got,
        &want,
        "a new asker of a scope question (W12_SCOPEQ): name the unit, read what the bool decides, add the row \
         (the deleting unit B3k replaces the entry, every asker then takes the record's answer)",
        "an asker of a scope question is gone: name the unit, remove the row",
    ));
    let counts = format!(
        "entries={} sites={}",
        entries.len(),
        rows.values().sum::<usize>()
    );
    if counts != tables::get("HC_SCOPEQ_COUNTS") {
        msgs.push(format!(
            "HC_SCOPEQ COUNTS table [{}] now [{counts}] — W12: the scope-question entries may only fall",
            tables::get("HC_SCOPEQ_COUNTS")
        ));
    }
    finish("HC_SCOPEQ", &msgs);
}

#[test]
fn hxf_the_found_set_fills_are_pinned() {
    // §5.2 Q-DECLKEY DK1-5a (DELTA16 R-H12 note 2): `KeyedSet::extend_found` moves a found
    // key's String into a KeyedSet, and window w9 (key.rs Deref / DerefMut) still opens that set
    // to text until DK3-b — covered by w9's NOT-PROMISED line, so its callers outside lexscope
    // are pinned by (file, fn, entry) with a count: a new caller is a new found-key text in a
    // text-open set. Calls through a fn value or a macro body are not seen (watched_calls'
    // walker, as HC_SCOPEQ's).
    let entries = vec!["extend_found".to_string()];
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for (file, qual, callee, _, _) in watched_calls(&entries) {
        if in_lexscope(&file) {
            continue;
        }
        *rows.entry(format!("{file}\t{qual}\t{callee}")).or_default() += 1;
    }
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HXF", "file\tfn\tentry\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HXF")) {
        assert_eq!(
            row.len(),
            4,
            "HXF table row has {} columns, not 4: {row:?}",
            row.len()
        );
        want.insert(row[..3].join("\t"), row[3].clone());
    }
    let mut msgs = diff_rows(
        "HXF",
        &got,
        &want,
        "a new caller of KeyedSet::extend_found: a found key's text into a set window w9 still opens — name \
         the unit, read what the set answers, add the row (DK3-b deletes the window)",
        "a caller of KeyedSet::extend_found is gone: name the unit, remove the row",
    );
    let counts = format!(
        "entries={} sites={}",
        entries.len(),
        rows.values().sum::<usize>()
    );
    if counts != tables::get("HXF_COUNTS") {
        msgs.push(format!(
            "HXF COUNTS table [{}] now [{counts}]",
            tables::get("HXF_COUNTS")
        ));
    }
    finish("HXF", &msgs);
}

/// The position of `param` among the typed parameters of the lexscope fn `item`.
fn enter_param_index(item: &str, param: &str) -> usize {
    let (ty, name) = match item.rsplit_once("::") {
        Some((t, n)) => (Some(t.to_string()), n.to_string()),
        None => (None, item.to_string()),
    };
    let mut found = Vec::new();
    for (rel, text) in lexscope_texts() {
        let file = syn::parse_file(&text).unwrap_or_else(|e| panic!("parse {rel}: {e}"));
        struct Find<'o> {
            ty: Option<String>,
            name: String,
            param: String,
            cur: Option<String>,
            out: &'o mut Vec<usize>,
        }
        impl Find<'_> {
            fn sig(&mut self, sig: &syn::Signature) {
                if sig.ident != self.name.as_str() || self.cur != self.ty {
                    return;
                }
                let pos = sig
                    .inputs
                    .iter()
                    .filter(|a| matches!(a, syn::FnArg::Typed(_)))
                    .position(|a| matches!(a, syn::FnArg::Typed(t) if matches!(&*t.pat, Pat::Ident(i) if i.ident == self.param.as_str())));
                if let Some(p) = pos {
                    self.out.push(p);
                }
            }
        }
        impl<'a> Visit<'a> for Find<'_> {
            fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
                let saved = self.cur.take();
                if let syn::Type::Path(p) = &*i.self_ty {
                    self.cur = p.path.segments.last().map(|s| s.ident.to_string());
                }
                visit::visit_item_impl(self, i);
                self.cur = saved;
            }
            fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
                self.sig(&f.sig);
            }
            fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
                self.sig(&f.sig);
            }
        }
        Find {
            ty: ty.clone(),
            name: name.clone(),
            param: param.to_string(),
            cur: None,
            out: &mut found,
        }
        .visit_file(&file);
    }
    assert_eq!(
        found.len(),
        1,
        "HC ARGS: ENTER row {item} · {param} matches {} lexscope fn(s)",
        found.len()
    );
    found[0]
}

/// Whether a type names `ScopeRef` (a path segment anywhere in it).
fn names_scope_ref(ty: &syn::Type) -> bool {
    struct V(bool);
    impl<'a> Visit<'a> for V {
        fn visit_path_segment(&mut self, s: &'a syn::PathSegment) {
            if s.ident == "ScopeRef" {
                self.0 = true;
            }
            visit::visit_path_segment(self, s);
        }
    }
    let mut v = V(false);
    v.visit_type(ty);
    v.0
}

/// LS3: every fn of lexscope's non-test files with a typed parameter whose type names
/// `ScopeRef`: (simple name, the index of the first such parameter among the typed ones, how
/// many it has).
fn carried_params() -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    for (rel, text) in lexscope_texts() {
        if rel.ends_with("tests.rs") {
            continue;
        }
        let file = syn::parse_file(&text).unwrap_or_else(|e| panic!("parse {rel}: {e}"));
        struct Find<'o> {
            out: &'o mut Vec<(String, usize, usize)>,
        }
        impl Find<'_> {
            fn sig(&mut self, sig: &syn::Signature) {
                let hits: Vec<usize> = sig
                    .inputs
                    .iter()
                    .filter_map(|a| match a {
                        syn::FnArg::Typed(t) => Some(t),
                        syn::FnArg::Receiver(_) => None,
                    })
                    .enumerate()
                    .filter(|(_, t)| names_scope_ref(&t.ty))
                    .map(|(i, _)| i)
                    .collect();
                if let Some(first) = hits.first() {
                    self.out.push((sig.ident.to_string(), *first, hits.len()));
                }
            }
        }
        impl<'a> Visit<'a> for Find<'_> {
            fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
                self.sig(&f.sig);
            }
            fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
                self.sig(&f.sig);
            }
        }
        Find { out: &mut out }.visit_file(&file);
    }
    out
}

#[test]
fn hc_args_the_to_scope_arguments_are_pinned() {
    // keyed by ENTER's to-scope rows (DELTA12 S2 r4): the argument at that parameter, every call
    let srcs = load_src();
    let fns = hf_fns(&srcs);
    let mut at: BTreeMap<String, usize> = BTreeMap::new();
    let mut msgs = Vec::new();
    for r in surf_rows("SURF_ENTER")
        .iter()
        .filter(|r| r[2] == "to-scope")
    {
        let simple = item_simple(&r[0]);
        let defs = fns.iter().filter(|f| f.simple == simple).count();
        if defs != 1 {
            msgs.push(format!(
                "HC ARGS the to-scope entry `{simple}` is defined {defs} times in the crate: a call is matched by its \
                 name, so the name must be unique"
            ));
        }
        at.insert(simple, enter_param_index(&r[0], &r[1]));
    }
    for (simple, idx, n) in carried_params() {
        let defs = fns.iter().filter(|f| f.simple == simple).count();
        if defs != 1 || n != 1 {
            msgs.push(format!(
                "HC ARGS the carried-scope entry `{simple}` is defined {defs} times in the crate and has {n} \
                 `ScopeRef` parameter(s): a call is matched by its name, and no entry takes two scopes (PLAN8 H2 (4))"
            ));
        }
        if at.insert(simple.clone(), idx).is_some() {
            msgs.push(format!(
                "HC ARGS `{simple}` is both a to-scope ENTER row and a carried-scope entry"
            ));
        }
    }
    let watch: Vec<String> = at.keys().cloned().collect();
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for (file, qual, callee, args, _) in watched_calls(&watch) {
        let arg = args.get(at[&callee]).cloned().unwrap_or_else(|| "?".into());
        *rows
            .entry(format!("{file}\t{qual}\t{callee}\t{arg}"))
            .or_default() += 1;
    }
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HC_ARGS", "file\tfn\tcallee\targ\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HC_ARGS")) {
        assert_eq!(
            row.len(),
            5,
            "HC_ARGS table row has {} columns, not 5: {row:?}",
            row.len()
        );
        want.insert(row[..4].join("\t"), row[4].clone());
    }
    msgs.extend(diff_rows(
        "HC_ARGS",
        &got,
        &want,
        "a new call of a to-scope entry, or its scope argument changed: read the text it enters, name the unit, \
         add the row",
        "a call of a to-scope entry is gone or its argument changed: name the unit, remove the row",
    ));
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for (k, n) in &rows {
        *by.entry(k.split('\t').nth(2).unwrap_or("").to_string())
            .or_default() += n;
    }
    let counts = dbg_counts(&by);
    if counts != tables::get("HC_ARGS_COUNTS") {
        msgs.push(format!(
            "HC_ARGS COUNTS table [{}] now [{counts}]",
            tables::get("HC_ARGS_COUNTS")
        ));
    }
    finish("HC_ARGS", &msgs);
}

// =====================================================================================
// HOLDERS — who holds a lexscope type (§5.2 Q-LEXSCOPE LS2-d: DELTA9 J1 (3), J4)
// =====================================================================================

/// The lexscope types a holder is pinned for (SURF_VOCAB's struct names).
const HOLDER_TYPES: &[&str] = &[
    "Scopes",
    "Saved",
    "ScopeWitness",
    "Shown",
    "ShownCtx",
    "SavedChain",
    "SavedRoot",
    "SavedMethod",
    "ScopeRef",
    "ScopeKeyed",
    "ReportedAt",
];

/// Every struct field, enum variant field and fn signature position (a typed parameter, the
/// return) of crates/elaborate/src whose type names a lexscope type, by (file, item,
/// position, type) with a count ; a `#[cfg(test)]` item is not read.
fn holders_census() -> BTreeMap<String, usize> {
    struct W<'o> {
        file: &'o str,
        owner: Vec<String>,
        out: &'o mut BTreeMap<String, usize>,
    }
    impl W<'_> {
        fn ty(&mut self, item: &str, pos: &str, t: &syn::Type) {
            let ids = type_idents(t);
            if ids.iter().any(|i| HOLDER_TYPES.contains(&i.as_str())) {
                *self
                    .out
                    .entry(format!("{}\t{item}\t{pos}\t{}", self.file, type_str(t)))
                    .or_default() += 1;
            }
        }
        fn sig(&mut self, sig: &syn::Signature) {
            let item = match self.owner.last() {
                Some(o) => format!("{o}::{}", sig.ident),
                None => sig.ident.to_string(),
            };
            for a in &sig.inputs {
                if let syn::FnArg::Typed(pt) = a {
                    let mut n = Vec::new();
                    pat_names(&pt.pat, &mut n);
                    self.ty(&item, &format!("IN:{}", n.join(",")), &pt.ty);
                }
            }
            if let syn::ReturnType::Type(_, t) = &sig.output {
                self.ty(&item, "RET", t);
            }
        }
        fn fields(&mut self, item: &str, f: &syn::Fields) {
            for (i, fd) in f.iter().enumerate() {
                let n = fd
                    .ident
                    .as_ref()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| i.to_string());
                self.ty(&format!("{item}.{n}"), "FIELD", &fd.ty);
            }
        }
    }
    impl<'a> Visit<'a> for W<'_> {
        fn visit_item(&mut self, it: &'a syn::Item) {
            let attrs: &[syn::Attribute] = match it {
                syn::Item::Mod(m) => &m.attrs,
                syn::Item::Fn(f) => &f.attrs,
                syn::Item::Impl(i) => &i.attrs,
                syn::Item::Struct(s) => &s.attrs,
                syn::Item::Enum(e) => &e.attrs,
                _ => &[],
            };
            if has_cfg_test(attrs) {
                return;
            }
            visit::visit_item(self, it);
        }
        fn visit_item_struct(&mut self, s: &'a syn::ItemStruct) {
            self.fields(&s.ident.to_string(), &s.fields);
        }
        fn visit_item_enum(&mut self, e: &'a syn::ItemEnum) {
            for v in &e.variants {
                self.fields(&format!("{}::{}", e.ident, v.ident), &v.fields);
            }
        }
        fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
            let o = match &*i.self_ty {
                syn::Type::Path(p) => p
                    .path
                    .segments
                    .last()
                    .map(|x| x.ident.to_string())
                    .unwrap_or_default(),
                t => type_str(t),
            };
            self.owner.push(o);
            visit::visit_item_impl(self, i);
            self.owner.pop();
        }
        fn visit_item_trait(&mut self, t: &'a syn::ItemTrait) {
            self.owner.push(t.ident.to_string());
            visit::visit_item_trait(self, t);
            self.owner.pop();
        }
        fn visit_signature(&mut self, sig: &'a syn::Signature) {
            self.sig(sig);
        }
    }
    let srcs = load_src();
    let mut out = BTreeMap::new();
    for s in &srcs {
        W {
            file: &s.rel,
            owner: Vec::new(),
            out: &mut out,
        }
        .visit_file(&s.file);
    }
    out
}

#[test]
fn holders_the_lexscope_types_holders_are_pinned() {
    let rows = holders_census();
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HOLDERS", "file\titem\tposition\ttype\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HOLDERS")) {
        assert_eq!(
            row.len(),
            5,
            "HOLDERS table row has {} columns, not 5: {row:?}",
            row.len()
        );
        want.insert(row[..4].join("\t"), row[4].clone());
    }
    let msgs = diff_rows(
        "HOLDERS",
        &got,
        &want,
        "a new holder of a lexscope type (a field, a variant field, a parameter or a return): a new copy \
         of a scope or of display text has its row here or in HOUT / HC, no third place — name the unit, \
         add the row",
        "a holder is gone: name the unit, remove the row",
    );
    finish("HOLDERS", &msgs);
}

// =====================================================================================
// HKC the key constructions (§5.2 Q-DECLKEY DK1-4b-1, DELTA16 R-H8 (d) ; widened DK1-4b-2, R-H9 note 1)
// =====================================================================================

/// HKC: the key types whose tuple constructor is a lexscope door — NewKey's field is private
/// to `key.rs` ; AtKey's (DK1-4b-1) and DeclKey's (DK1-4b-2) are `pub(super)`: the walk forms
/// its candidates and returns its found key, so their door is the whole module.
const HKC_TYPES: &[&str] = &["AtKey", "NewKey", "DeclKey"];
/// HKC: the forms counted per (file, type) — `ctor` the constructor named (a call, a struct
/// literal, a pattern, a macro argument) ; `value` the type's path used as a VALUE (not called,
/// not a type, not a pattern: `.map(AtKey)`, `let mk = AtKey`) ; `alias` a `type` alias or a
/// `use .. as` rename naming the type ; `write` the key's field `.0` written (an assignment, a
/// compound assignment, `&mut k.0`, a String-mutating method on it, `write!` / `writeln!` into
/// it) through a binding the fn types as the key (§5.2 Q-DECLKEY DK1-4b-2, DELTA16 R-H9 note 1).
const HKC_FORMS: &[&str] = &["ctor", "value", "alias", "write"];
/// HKC: the String methods that mutate the key's text in place.
const HKC_MUT: &[&str] = &[
    "clear",
    "push",
    "push_str",
    "insert",
    "insert_str",
    "truncate",
    "pop",
    "remove",
    "retain",
    "drain",
    "extend",
    "replace_range",
    "split_off",
    "as_mut_str",
    "make_ascii_lowercase",
    "make_ascii_uppercase",
];

/// HKC: (file, type, form) -> how many times a lexscope file names a key type in that form
/// (HKC_FORMS) — every lexscope file read, its `#[cfg(test)]` file too, and the arguments of a
/// macro read as a comma list of expressions. A `write` is attributed to a type through the
/// binding's own spelled type: a fn / closure parameter `k: T` / `&T` / `&mut T`, a `let k: T`,
/// a `let k = T(..)`, or `self` inside `impl T`. REVIEW-ONLY (restated at §5.2 Q-DECLKEY DK1-5a,
/// DELTA16 R-H12 note 3): `Self(..)` inside the type's own `impl` — a construction HKC does not
/// count (key.rs has none) ; the body of a `macro_rules!` definition and a macro argument that is
/// no expression — neither is read ; a `.0` write through a
/// binding whose type is not spelled where HKC reads it (an untyped `let` of a returned key, a
/// struct field holding a key, a `&mut String` taken by a method HKC_MUT does not list) ; a
/// whole-key move (`mem::swap` of two keys moves a key, not its text). Outside lexscope the
/// field is private: no row can exist there (C_AT_FROM_TEXT / C_AT_BUILT_OUTSIDE fail to
/// compile).
fn hkc_census() -> BTreeMap<String, usize> {
    let mut rows = BTreeMap::new();
    for (rel, text) in lexscope_texts() {
        let file = syn::parse_file(&text).unwrap_or_else(|e| panic!("HKC cannot parse {rel}: {e}"));
        for ((ty, form), n) in hkc_count(&file) {
            rows.insert(format!("{rel}\t{ty}\t{form}"), n);
        }
    }
    rows
}

/// HKC's count of one file: (type, form) -> uses (HKC_FORMS).
fn hkc_count(file: &syn::File) -> BTreeMap<(String, String), usize> {
    fn key_ty(t: &syn::Type) -> Option<String> {
        match t {
            syn::Type::Reference(r) => key_ty(&r.elem),
            syn::Type::Paren(p) => key_ty(&p.elem),
            syn::Type::Path(p) => p
                .path
                .segments
                .last()
                .map(|l| l.ident.to_string())
                .filter(|n| HKC_TYPES.contains(&n.as_str())),
            _ => None,
        }
    }
    fn names_key(t: &syn::Type) -> Option<String> {
        struct N(Option<String>);
        impl<'a> Visit<'a> for N {
            fn visit_path(&mut self, p: &'a syn::Path) {
                if let Some(l) = p.segments.last() {
                    let n = l.ident.to_string();
                    if HKC_TYPES.contains(&n.as_str()) && self.0.is_none() {
                        self.0 = Some(n);
                    }
                }
                visit::visit_path(self, p);
            }
        }
        let mut n = N(None);
        n.visit_type(t);
        n.0
    }
    struct C {
        n: BTreeMap<(String, String), usize>,
        /// the bindings each enclosing fn / closure types as a key: name -> type
        typed: Vec<BTreeMap<String, String>>,
        /// the key type of the enclosing `impl`, if any
        impl_ty: Vec<Option<String>>,
    }
    impl C {
        fn add(&mut self, ty: &str, form: &str) {
            *self
                .n
                .entry((ty.to_string(), form.to_string()))
                .or_default() += 1;
        }
        fn path(&mut self, p: &syn::Path, form: &str) {
            if let Some(l) = p.segments.last() {
                let n = l.ident.to_string();
                if HKC_TYPES.contains(&n.as_str()) {
                    self.add(&n, form);
                }
            }
        }
        fn bind_pat(&mut self, p: &Pat, ty: Option<String>) {
            let (pat, ty) = match p {
                Pat::Type(pt) => (&*pt.pat, key_ty(&pt.ty).or(ty)),
                other => (other, ty),
            };
            if let (Pat::Ident(pi), Some(t)) = (pat, ty) {
                if let Some(m) = self.typed.last_mut() {
                    m.insert(pi.ident.to_string(), t);
                }
            }
        }
        /// the key type of `<base>.0` when `e` is that field of a typed binding
        fn field0(&self, e: &Expr) -> Option<String> {
            let Expr::Field(f) = e else {
                return None;
            };
            if !matches!(&f.member, syn::Member::Unnamed(i) if i.index == 0) {
                return None;
            }
            let base = single_path(&f.base)?;
            if base == "self" {
                return self.impl_ty.last().cloned().flatten();
            }
            self.typed.iter().rev().find_map(|m| m.get(&base).cloned())
        }
        fn write(&mut self, e: &Expr) {
            if let Some(t) = self.field0(e) {
                self.add(&t, "write");
            }
        }
    }
    impl<'a> Visit<'a> for C {
        fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
            self.impl_ty.push(if i.trait_.is_none() {
                key_ty(&i.self_ty)
            } else {
                None
            });
            visit::visit_item_impl(self, i);
            self.impl_ty.pop();
        }
        fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
            self.typed.push(BTreeMap::new());
            for a in &f.sig.inputs {
                if let syn::FnArg::Typed(pt) = a {
                    self.bind_pat(&Pat::Type(pt.clone()), None);
                }
            }
            visit::visit_item_fn(self, f);
            self.typed.pop();
        }
        fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
            self.typed.push(BTreeMap::new());
            for a in &f.sig.inputs {
                if let syn::FnArg::Typed(pt) = a {
                    self.bind_pat(&Pat::Type(pt.clone()), None);
                }
            }
            visit::visit_impl_item_fn(self, f);
            self.typed.pop();
        }
        fn visit_expr_closure(&mut self, c: &'a syn::ExprClosure) {
            self.typed.push(BTreeMap::new());
            for p in &c.inputs {
                self.bind_pat(p, None);
            }
            visit::visit_expr_closure(self, c);
            self.typed.pop();
        }
        fn visit_local(&mut self, l: &'a syn::Local) {
            let from_init = l.init.as_ref().and_then(|i| match &*i.expr {
                Expr::Call(c) => match &*c.func {
                    Expr::Path(p) => p
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .filter(|n| HKC_TYPES.contains(&n.as_str())),
                    _ => None,
                },
                _ => None,
            });
            self.bind_pat(&l.pat, from_init);
            visit::visit_local(self, l);
        }
        fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
            if let Expr::Path(p) = &*c.func {
                self.path(&p.path, "ctor");
                // the constructor's own path is not a value use
                for a in &c.args {
                    self.visit_expr(a);
                }
                return;
            }
            visit::visit_expr_call(self, c);
        }
        fn visit_expr_path(&mut self, p: &'a syn::ExprPath) {
            self.path(&p.path, "value");
            visit::visit_expr_path(self, p);
        }
        fn visit_expr_struct(&mut self, s: &'a syn::ExprStruct) {
            self.path(&s.path, "ctor");
            visit::visit_expr_struct(self, s);
        }
        fn visit_pat_tuple_struct(&mut self, p: &'a syn::PatTupleStruct) {
            self.path(&p.path, "ctor");
            visit::visit_pat_tuple_struct(self, p);
        }
        fn visit_pat_struct(&mut self, p: &'a syn::PatStruct) {
            self.path(&p.path, "ctor");
            visit::visit_pat_struct(self, p);
        }
        fn visit_item_type(&mut self, t: &'a syn::ItemType) {
            if let Some(n) = names_key(&t.ty) {
                self.add(&n, "alias");
            }
            visit::visit_item_type(self, t);
        }
        fn visit_use_rename(&mut self, r: &'a syn::UseRename) {
            let n = r.ident.to_string();
            if HKC_TYPES.contains(&n.as_str()) {
                self.add(&n, "alias");
            }
            visit::visit_use_rename(self, r);
        }
        fn visit_expr_assign(&mut self, a: &'a syn::ExprAssign) {
            self.write(&a.left);
            visit::visit_expr_assign(self, a);
        }
        fn visit_expr_binary(&mut self, b: &'a syn::ExprBinary) {
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
                self.write(&b.left);
            }
            visit::visit_expr_binary(self, b);
        }
        fn visit_expr_reference(&mut self, r: &'a syn::ExprReference) {
            if r.mutability.is_some() {
                self.write(&r.expr);
            }
            visit::visit_expr_reference(self, r);
        }
        fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) {
            if HKC_MUT.contains(&m.method.to_string().as_str()) {
                self.write(&m.receiver);
            }
            visit::visit_expr_method_call(self, m);
        }
        fn visit_macro(&mut self, m: &'a syn::Macro) {
            if let Ok(args) = m.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
                let wr = m
                    .path
                    .segments
                    .last()
                    .is_some_and(|l| l.ident == "write" || l.ident == "writeln");
                if wr {
                    if let Some(first) = args.first() {
                        self.write(first);
                    }
                }
                for a in &args {
                    self.visit_expr(a);
                }
            }
        }
    }
    let mut c = C {
        n: BTreeMap::new(),
        typed: vec![BTreeMap::new()],
        impl_ty: Vec::new(),
    };
    c.visit_file(file);
    c.n
}

/// HKC_COUNTS: per form, per type (every listed type and form printed, zeros too).
fn hkc_counts(rows: &BTreeMap<String, usize>) -> String {
    let mut out = Vec::new();
    for form in HKC_FORMS {
        let mut per_ty: BTreeMap<&str, usize> = HKC_TYPES.iter().map(|t| (*t, 0)).collect();
        for (k, n) in rows {
            let mut cols = k.split('\t').skip(1);
            let (ty, f) = (cols.next().expect("type"), cols.next().expect("form"));
            if f == *form {
                *per_ty.get_mut(ty).expect("a listed type") += n;
            }
        }
        let tys: Vec<String> = per_ty.iter().map(|(t, n)| format!("{t}={n}")).collect();
        out.push(format!("{form}[{}]", tys.join(" ")));
    }
    out.join(" ")
}

#[test]
fn hkc_the_key_constructions_are_pinned() {
    // the census on a fixture: a call, a struct literal, a tuple-struct pattern, a macro
    // argument ; not the definition, not a type position, not `AtKey::new()`, not a path that
    // only ends in a longer name
    let fx = syn::parse_file(
        "struct AtKey(pub(super) String); fn f(k: &AtKey) -> AtKey { let a = AtKey(String::new()); \
         let b = NewKey { 0: s }; let AtKey(t) = a; g(vec![AtKey(x)]); m::AtKeyish(1); AtKey::new(); b }",
    )
    .expect("HKC fixture");
    let shown = |n: BTreeMap<(String, String), usize>| {
        n.iter()
            .map(|((t, f), c)| format!("{t}:{f}={c}"))
            .collect::<Vec<_>>()
    };
    assert_eq!(shown(hkc_count(&fx)), ["AtKey:ctor=3", "NewKey:ctor=1"]);
    // DK1-4b-2 (R-H9 note 1): the six shapes the constructor count was blind to, and the walk's
    // own scratch writes ; an untyped `.0` write and a non-key `.0` are not counted
    let fx = syn::parse_file(
        "fn a(v: Vec<String>) -> Vec<AtKey> { v.into_iter().map(AtKey).collect() } \
         fn b(s: String) -> AtKey { let mk = AtKey; mk(s) } \
         fn c(k: &mut AtKey, s: String) { k.0 = s; } \
         type HkcK = AtKey; \
         fn d(s: String) -> AtKey { use super::AtKey as Hk; Hk(s) } \
         fn e(v: Option<String>) -> Option<DeclKey> { v.map(DeclKey) } \
         fn w(mut key: AtKey, x: &mut Shown, u: &mut Other) { key.0.clear(); key.0.push_str(n); \
         let _ = write!(key.0, \"{p}.{n}\"); x.0.clear(); u.0 = s; let mut j = AtKey(String::new()); \
         j.0 += \"x\"; std::mem::swap(&mut j.0, &mut t); } \
         impl DeclKey { fn z(&mut self) { self.0.clear(); } } \
         impl Keyed { fn y(&mut self) { self.0.clear(); } }",
    )
    .expect("HKC fixture 2");
    assert_eq!(
        shown(hkc_count(&fx)),
        [
            "AtKey:alias=2",
            "AtKey:ctor=1",
            "AtKey:value=2",
            "AtKey:write=6",
            "DeclKey:value=1",
            "DeclKey:write=1"
        ]
    );
    let rows = hkc_census();
    let got: BTreeMap<String, String> = rows
        .iter()
        .map(|(k, n)| (k.clone(), n.to_string()))
        .collect();
    let dumped: Vec<String> = rows.iter().map(|(k, n)| format!("{k}\t{n}")).collect();
    dump("HKC", "file\ttype\tform\tcount", &dumped);
    let mut want = BTreeMap::new();
    for row in table(&tables::get("HKC")) {
        assert_eq!(
            row.len(),
            4,
            "HKC table row has {} columns, not 4: {row:?}",
            row.len()
        );
        want.insert(row[..3].join("\t"), row[3].clone());
    }
    let mut msgs = diff_rows(
        "HKC",
        &got,
        &want,
        "a new construction, value use, alias or field write of a key type in lexscope (the \
         constructor door is the module, not one file): a key formed from text is P23's — name the \
         unit, read where the text comes from, add the row",
        "a key construction is gone: name the unit, remove the row",
    );
    let counts = hkc_counts(&rows);
    if counts != tables::get("HKC_COUNTS") {
        msgs.push(format!(
            "HKC COUNTS table [{}] now [{counts}]",
            tables::get("HKC_COUNTS")
        ));
    }
    finish("HKC", &msgs);
}

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
