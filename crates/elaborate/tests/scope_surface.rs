//! HT-SURF, the module-surface home test of §5.2 Q-LEXSCOPE (unit LS1 part 4, DELTA10 L1):
//! the door of `crate::lexscope` judged by its items' TYPES, computed by `syn` on every
//! build of the tests. It lives beside `scope_home.rs` (a second file: that one passed
//! 5000 lines) with its pins in `scope_surface.tables` and its fixture modules in
//! `scope_surface.fixtures`.
//!
//! I-SURF. Every item of `crate::lexscope` that code outside the module can name has, in each
//! OUT position, a type built only from the vocabulary (`SURF_VOCAB`: the module's opaque
//! types), `bool`, `()`, a type parameter bounded by nothing but Sized / ?Sized / Clone /
//! Default / a lifetime, and Option / Vec / Box / tuple / reference / slice / array of those —
//! except the boundary `scope_out` and the rows of the counted list LEAVE. OUT positions: a
//! return type ; the type under `&mut` in a parameter ; every argument type of a callback
//! parameter (`impl Fn*`, `dyn Fn*`, `fn(..)`, a type parameter bounded by `Fn*`) ; a visible
//! field's, variant payload's, const's, alias's and associated type's type ; a visible
//! trait's items the same ; a parameter naming an interior-mutable type is judged whole.
//! I-IN. Every text-typed IN parameter of the surface is a row of ENTER with its role
//! (LS1 part 7 replaces it by IN-TOTAL, below).
//! I-OUT. Every lexscope fn that takes the Elaborator (a receiver in `impl Elaborator`, or
//! `Self` in a callback) is on the list E, its body pinned by OUT-F (`self.<field>` whose
//! field type is not a lexscope type ; a WINDOW type's store counts as not one), OUT-C (calls of fns not defined in lexscope), OUT-M
//! (macro calls) by (fn, name, count) ; FLOW (`in`, `payload`, `text:<sink>`) is a hand column.
//!
//! The rules DELTA11 R2 adds (LS1 part 5), each a counted list whose missing row FAILS:
//! CARD — an OUT position whose type holds a cardinality constructor at any depth (Option,
//! Vec, a slice, an array, two or more `bool` ; an alias of the file set is followed) is not
//! PASS by the vocabulary: its verdict is its row of CARD (item, position, type, MEANS: hand,
//! `caller` | `hit:<question>` | `list:<what>`), else FAIL CARD ; a LEAVE row exempts it.
//! CB — every callback a surface item's parameter holds (`impl Fn*`, `dyn Fn*`, `fn(..)`, a
//! type parameter bounded by `Fn*`, at any depth of the parameter's type) is a row of CB
//! (item, parameter, Fn | FnMut | FnOnce | fn, argument types, CALLS: hand, `once` (FnOnce
//! only) | `per:<what>` | `probe:<what>`), else FAIL CB ; its argument types stay OUT.
//! MACRO-BODY — every `macro_rules!` the file set defines, or the rest of the crate defines and
//! the file set calls, is a row of MACROS with the sha256 of its token tree ; a definition off
//! the list or a changed hash FAILS, a row with no such definition is STALE.
//! BODY-ALL — every fn of the file set with a body (free, nested in a body, inherent, trait
//! impl, trait default ; tier K and tier E alike) is a row of BODY (file, fn, sha256 of its
//! body tokens) ; a new, changed or gone body FAILS. (BODY left E.)
//! ALIAS-SELF+ — in a tier-E body every pattern is read (a `let`, a `match` arm, `if let`, a
//! closure parameter): a struct or tuple-struct pattern named `Self` / `Elaborator` fails, and
//! so does a `match` or a `let` pattern whose scrutinee is `self`.
//! ELAB-PARAM, extended here (DELTA11 NB11): a fn of the file set whose parameter names the
//! Elaborator fails whatever its visibility and wherever it sits (a private method of a
//! lexscope type, a trait impl's or a trait's method included).
//!
//! The rule LS1 part 6 adds (DELTA11 R1's bar on an occurring shape: `edge_gate.rs:1598`
//! `struct Collect(RefCell<Vec<String>>)`, `:1609` `fn errors(sink: &Collect)`):
//! INTERIOR — in every parameter of a surface item and every field (or variant payload) of a
//! vocabulary type, each type name the type spells is followed, by syntax, through the
//! fields of every definition the crate's `src` gives that name (struct, enum variant, union,
//! type alias ; any file, items in bodies and `#[cfg]` modules alike), a `use .. as` rename to
//! the original's name, `Self` to the impl's type, transitively with a visited set ; the
//! position FAILS when a reached type spells a mutable carrier (Cell, RefCell, OnceCell,
//! LazyCell, UnsafeCell, Mutex, RwLock, OnceLock, LazyLock, Atomic*, mpsc Sender / SyncSender /
//! Receiver, by a path's last name, or a `*mut`), the detail being the shortest path of names
//! followed ; a row of INTERIOR (item, position, that path, why) exempts it, a row whose path
//! moved fails, an unused row is STALE. The receiver (`&self` / `&mut self`, not a typed
//! parameter) is exempt by position: in `impl Elaborator` it is the one tier-E receiver, whose
//! fields OUT-F pins ; elsewhere its type is a vocabulary type, judged by its fields. Every
//! other parameter is judged, a second Elaborator (`other: &Self`) and the `&mut Self` a
//! callback is lent included. A name the crate does not define is not followed (its type
//! arguments are still read): it stays what the OUT judge makes it.
//!
//! The rules LS1 part 7 adds (DELTA12 S1 S2 N-1 N-6):
//! ADDR — crate-wide (every file of `src`, tests under `src`, macro arguments and `use` items
//! included ; the module present or not), every token site that turns a reference into an
//! address: s1 a raw-pointer type `*const` / `*mut` anywhere ; s2 `&raw const` / `&raw mut` ;
//! s3 a string literal holding a `p` format spec by std::fmt's grammar ; n1 the path segment
//! `ptr` (`ptr ::`, `:: ptr`, any `use` of it) ; n2 / n3 the names of ADDR_NAMES (n2 an ident
//! anywhere, n3 `. name (` or `:: name`), read for the toolchain ADDR_TOOLCHAIN pins (the
//! crate test fails when rust-toolchain.toml moves). Each (file, fn, form) is a row of ADDR
//! with its count and OPERAND (the token text of each converted expression), else FAIL ADDR ;
//! a row matching nothing is STALE. CALLS: each fn holding a pinned row has its name's call
//! tokens crate-wide pinned (its `fn <name>` excluded). ADDR-FN: no fn holding an address
//! site (pinned or not) holds a token of a surface item one of whose OUT positions is REF
//! (`&` in its type ; the computed rows' last column).
//! UNWIND — the tokens catch_unwind, set_hook, take_hook, resume_unwind, panic_any in the
//! crate's `src`, pinned by ADDR's census pass (empty ; a site off it FAILS).
//! IN-TOTAL (replaces I-IN) — every name an IN position of a surface item holds gets a
//! verdict, first match: a type parameter (its bounds) ; TEXT_TYPES -> TEXT ; NUMBER_TYPES ->
//! NUMBER ; `bool` ; a vocabulary type ; `Self` (its impl's type) ; a name the index defines
//! -> FOLLOWED, breadth-first through its fields to the shortest TEXT path, else NUMBER ; a
//! `use .. as` rename (its original) ; IN_CONTAINERS (its arguments judged) ; anything else
//! FAILS IN-UNKNOWN. The index is INTERIOR's over the crate's `src` and the `src` of every path
//! dependency its Cargo.toml files name, transitively (a `workspace = true` entry through the
//! workspace manifest ; a registry crate is not read) ; a written first segment resolves as
//! `crate` / `self` / `super`, a crate alias (`use hdl_ast as ast;` in lib.rs) or a crate's
//! name ; any other path by its last name in every crate (D-1: a false hit is loud). A TEXT or
//! NUMBER position is a row of ENTER (item, parameter, role, holds: the kind and the path),
//! else FAIL ENTER ; a moved `holds` FAILS ENTER ; a role off to-scope / to-display / lookup /
//! carried / index FAILS ROLE, `index` on a text position too ; `carried` FAILS WHOLE when the
//! body reads the parameter (the base of a field, a method call or an index ; a `match` /
//! `if let` / `while let` / `for` scrutinee ; in a macro's expression list) or has no body.
//! CARD per item (N-1) — the `bool`s of an item's OUT positions (RET, every `&mut` target,
//! every callback argument) are summed ; two or more make every position holding one a CARD
//! position.
//! What these do not see: a std fn off ADDR_NAMES that reveals or compares an address (M4's
//! read of std 1.85: `UnsafeCell::get`, `AtomicPtr` fed by a coercion, `Waker::data`) ; an
//! external crate doing it (serde, not read) ; a reference passed two or more calls deep into
//! a pinned address fn (CALLS is one level) ; a pin edited to let a site through ; a format
//! spec assembled by `concat!` ; a foreign type's text reached through a method (an interned
//! symbol: NP-17) ; a field behind a macro type or an associated type (IN-UNKNOWN, loud) ; a
//! `carried` parameter read through an alias (`let x = &p; x.f`: WHOLE reads the identifier).
//!
//! The rule §5.2 Q-DECLKEY DK1-1 adds (R-F10, premise10 N10-9): WINDOW — a trait impl in the file
//! set off ALLOWED_IMPLS passes only as a row of WINDOW naming its type and its trait, the trait one
//! of WINDOW_TRAITS (`Deref`, `DerefMut`), with the unit that deletes it and the date it came ; a
//! row off WINDOW_TRAITS fails WINDOW-TRAIT, one with no unit WINDOW-UNIT, one no impl matches is
//! STALE. Its rows are the stores' text door w9 (`Keyed`, `KeyedSet`, deleted at DK3-b) ; the
//! list does not widen ALLOWED_IMPLS (a `Deref` on any other type still fails IMPL-TRAIT, NP10-1).
//!
//! The rule §5.2 Q-DECLKEY DK1-2b adds (R-G6 (e), R-G8 (a)): BRIDGE — a trait impl in the file set
//! whose trait is one of the file set's OWN listed traits (defined there and on SURF_TRAITS) passes
//! only as a row of BRIDGE naming its Self type as written (`NewKey`, `String`, `&T`) and its trait,
//! with its CLASS, the unit that deletes it and the date it came ; the class is computed and must
//! match: TEXT (a Self type of TEXT_TYPES), REF (a reference to a type parameter: the blanket
//! impl), KEY (a type the file set defines) — so a text impl for another type fails IMPL-TRAIT (no
//! row) or BRIDGE-CLASS (a row that calls it KEY). Only a BRIDGE row admits a Self type the file
//! set does not define (IMPL-FOREIGN-SELF otherwise). A row with no unit fails BRIDGE-UNIT, one
//! whose trait is off SURF_TRAITS or not defined in the file set BRIDGE-TRAIT, one no impl matches
//! is STALE. Its rows are the Bound write funnel's bridge w11 (`SlotKey` / `slot_seal::SlotText`,
//! lexscope/key.rs, deleted at DK1-2d). A supertrait passes BOUND as a bound does in IN-TOTAL: a
//! pass bound, or a trait the file set defines and SURF_TRAITS lists (SlotKey's sealed half).
//!
//! What the engine reads: the module's file set = `lexscope/mod.rs` (or `lexscope.rs`) and
//! every file a `mod x;` inside it reaches ; every item there, in `impl` blocks, inline `mod`
//! blocks, `#[cfg(..)]` twins alike (a cfg'd item counts: fail-closed), and the items
//! written inside fn bodies and `const _` blocks (an `impl` there is judged as any other) ;
//! and the rest of the crate for three things: `mod lexscope;` in `lib.rs`, the
//! `Elaborator` struct's field types (OUT-F), and every impl naming a lexscope type.
//! Visibility: an inherent method by its own marker (a `pub(crate) fn` in a private child
//! module's impl is crate-wide) ; any other item by its marker and its module depth
//! (`pub(super)` in `mod.rs` = crate-wide ; in a child = inside), path privacy NOT applied
//! (a `pub(crate)` struct in a private child counts: the stricter reading).
//!
//! The rules the engine applies where DELTA10 L1 left the reading open (the stricter one
//! each time ; LS1 part 4's REPORT lists them): `T: Copy` is a bound outside the set ;
//! `#[derive(Debug)]` fails (Debug only as a manual impl whose body names no `self`) and so
//! do `derive(Copy)`, `cfg_attr` and every attribute off the list ; a `&mut` callback fails
//! as an opaque OUT type ; an inherent impl of a lexscope type outside the module fails ; a
//! fn taking the Elaborator outside `impl Elaborator` fails ; every Elaborator-taking fn is
//! on E whatever its visibility ; OUT-C counts every method call but `self.<a lexscope
//! Elaborator method>` ; a macro in a tier-E body that does not parse as an expression list
//! and names `self` fails ; a vocabulary name must be DEFINED in the file set to pass ; an
//! allow-list row (LEAVE, ENTER, ENUMS, SHIMS, OUTGOING, E) that matches nothing fails.
//!
//! What syntax does not see (REVIEW-ONLY): a method call on a receiver other than `self`
//! whose callee is an outside fn sharing a lexscope fn's name is counted, but a path call
//! `T::f` is matched by name only ; what a body does with a value (FLOW is a hand column,
//! frozen by BODY) ; the body of a macro invocation (`zn!`: the callers are pinned, not
//! what it expands to: MACRO-BODY pins its tokens, not its expansion) ; a macro call inside
//! another macro's tokens ; a macro defined in another crate (no definition to hash) ; a
//! proc-macro attribute's output (refused, not read) ; the identity of a name through a glob
//! `use` of another module (a vocabulary name must be defined in the file set, a
//! non-vocabulary name fails as unknown in an OUT position) ; INTERIOR's limits: a name is
//! matched by its last segment, so a crate type sharing a foreign type's name is followed and
//! a carrier inside a type of another crate (hdl-ast, std beyond the listed names) is not ; a
//! carrier spelled through a macro type or a trait's associated type ; a `&mut` field inside a
//! crate type passed by value (not a carrier) ; `Self` in a trait's own signature ; what
//! a body that passes its BODY pin does at run time (the hash is loud, not a refusal).
//!
//! Three tests: the engine on fixtures (one clean module that passes with its whole surface
//! pinned, NS1-NS32, the NB controls, NI1-NI10, NA1-NA7, NU1 and NT1-NT9 each failing with the pinned (item, position, rule)
//! set) ; the real crate, where `SURF_PRESENT` pinned `absent` until LS2a flipped it to
//! `present` and pinned the computed surface (the code computes the whole surface the day
//! the module appears, with no edit here) ; SURF-PLAN, the 30 fns DELTA10 L2 moves, judged by I-SURF on
//! today's signatures and pinned (no list consulted: ENTER-NEEDED names every TEXT or NUMBER
//! parameter IN-TOTAL finds).
//!
//! Environment (negative-control doors, unset in the gate): `SCOPE_HOME_SRC` reads another
//! copy of the crate's `src` ; `SCOPE_SURFACE_FIXTURES` reads another fixture file ;
//! `SCOPE_SURFACE_DUMP=<dir>` writes each computed table to `<dir>/<name>.tsv` (and, from the
//! crate test, INTERIOR's census of the crate's types and of every fn parameter, IN-TOTAL's
//! index by crate (IN_INDEX) and the foreign names in crate-visible fns' parameters
//! (IN_FOREIGN): measurements, not pins). A fixture's `@@ table:<NAME>` hunk edits a pin for
//! that fixture (in CLEAN, for all).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Expr, FnArg, Item, Token, Type, TypeParamBound};

// =====================================================================================
// The rules' word lists (the rule, not a pin)
// =====================================================================================

const MODULE: &str = "lexscope";
const ELAB: &str = "Elaborator";
const BOUNDARY_TRAIT: &str = "Outgoing";
/// Text types (by a path's last segment): an OUT position holding one fails TEXT.
const TEXT_TYPES: &[&str] = &[
    "String",
    "str",
    "Cow",
    "char",
    "Chars",
    "Bytes",
    "Arguments",
    "Formatter",
    "Path",
    "PathBuf",
    "OsStr",
    "OsString",
    "CStr",
    "CString",
];
/// Number types: an id, a hash, a length or a depth is a comparable stand-in.
const NUMBER_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64",
];
const NUMBER_PREFIXES: &[&str] = &["NonZero", "Wrapping", "Saturating", "Atomic"];
const WRAPPERS: &[&str] = &["Option", "Vec", "Box"];
const PASS_BOUNDS: &[&str] = &["Sized", "Clone", "Default"];
const FN_TRAITS: &[&str] = &["Fn", "FnMut", "FnOnce"];
/// Interior mutability: a parameter naming one is an OUT position, judged whole.
const CELL_TYPES: &[&str] = &[
    "Cell",
    "RefCell",
    "UnsafeCell",
    "Mutex",
    "RwLock",
    "OnceCell",
    "OnceLock",
    "LazyCell",
    "LazyLock",
];
const ALLOWED_ATTRS: &[&str] = &[
    "derive", "allow", "must_use", "inline", "doc", "cfg", "test",
];
const ALLOWED_DERIVES: &[&str] = &["Clone", "Default"];
const ALLOWED_IMPLS: &[&str] = &["Clone", "Default", "Debug", "Drop", BOUNDARY_TRAIT];
/// WINDOW (§5.2 Q-DECLKEY DK1-1, R-F10 / premise10 N10-9): the traits off ALLOWED_IMPLS a row of
/// WINDOW may admit, for the ONE type that row names ; not a wider allow list (NP10-1).
const WINDOW_TRAITS: &[&str] = &["Deref", "DerefMut"];
/// BRIDGE's classes (§5.2 Q-DECLKEY DK1-2b): a key type of the file set, the blanket `&T`, a text type.
const BRIDGE_CLASSES: &[&str] = &["KEY", "REF", "TEXT"];
const STD_ROOTS: &[&str] = &["std", "core", "alloc"];
const FLOWS: &[&str] = &["in", "payload"];
/// CARD's named constructors (a slice, an array and two or more `bool` are the others).
const CARD_CTORS: &[&str] = &["Option", "Vec"];
/// CARD's MEANS values: `caller`, or one of these prefixes with a non-empty rest.
const MEANS_PREFIXES: &[&str] = &["hit:", "list:"];
/// CB's CALLS values: `once` (an FnOnce), or one of these prefixes with a non-empty rest.
const CALLS_PREFIXES: &[&str] = &["per:", "probe:"];
/// IN-TOTAL's containers (DELTA12 S2 r2): an IN position passes through one of these to its
/// type arguments ; a reference, a slice, an array and a tuple are passed through by syntax.
const IN_CONTAINERS: &[&str] = &[
    "Option", "Vec", "Box", "Rc", "Arc", "BTreeMap", "BTreeSet", "HashMap", "HashSet",
];
/// ENTER's roles: DELTA10's three and DELTA12's `carried` (WHOLE checked) and `index`.
const ROLES: &[&str] = &["to-scope", "to-display", "lookup", "carried", "index"];
/// UNWIND (DELTA12 N-6): the std names that catch, re-raise or hook a panic's payload.
const UNWIND_NAMES: &[&str] = &[
    "catch_unwind",
    "set_hook",
    "take_hook",
    "resume_unwind",
    "panic_any",
];
/// The keywords a token-level operand stops at (ADDR's OPERAND column).
const OPERAND_STOP: &[&str] = &[
    "as", "let", "return", "in", "if", "else", "while", "match", "for", "loop", "break",
    "continue", "move", "ref", "mut", "unsafe", "static", "const", "fn", "where", "impl", "type",
    "use", "mod", "pub", "struct", "enum", "trait", "dyn",
];

// =====================================================================================
// The source set: relative path -> text
// =====================================================================================

type Vfs = BTreeMap<String, String>;

fn src_dir() -> PathBuf {
    match std::env::var("SCOPE_HOME_SRC") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
    }
}

fn load_dir(dir: &Path) -> Vfs {
    fn walk(root: &Path, dir: &Path, out: &mut Vfs) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(root, &p, out);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let rel = p
                    .strip_prefix(root)
                    .expect("under src")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
                out.insert(rel, text);
            }
        }
    }
    let mut out = Vfs::new();
    walk(dir, dir, &mut out);
    out
}

// =====================================================================================
// The crates the type index reads: the crate's own `src` and its path dependencies
// =====================================================================================

/// A crate the type index reads: its name as code writes it (`-` -> `_`) and its `src`.
struct Krate {
    name: String,
    src: Vfs,
}

/// Where manifests and dependency sources are read: the disk (the real crate: its manifest
/// directory) or a fixture's map, whose keys are relative to the fixture crate's `src` (its
/// manifest is `../Cargo.toml`, a dependency's files sit under `../../<dep>/`).
enum Tree<'a> {
    Disk(String),
    Map(&'a Vfs),
}

/// `base` joined with `rel`, `.` and `..` folded (an absolute `rel` replaces `base`).
fn norm_join(base: &str, rel: &str) -> String {
    let (abs, start) = if rel.starts_with('/') {
        (true, "")
    } else {
        (base.starts_with('/'), base)
    };
    let mut parts: Vec<&str> = Vec::new();
    for p in start.split('/').chain(rel.split('/')) {
        match p {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|l| *l != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            x => parts.push(x),
        }
    }
    let s = parts.join("/");
    if abs {
        format!("/{s}")
    } else {
        s
    }
}

impl Tree<'_> {
    fn crate_dir(&self) -> String {
        match self {
            Tree::Disk(d) => d.clone(),
            Tree::Map(_) => "..".into(),
        }
    }
    fn read(&self, p: &str) -> Option<String> {
        match self {
            Tree::Disk(_) => std::fs::read_to_string(p).ok(),
            Tree::Map(m) => m.get(p).cloned(),
        }
    }
    /// The `.rs` files under `<dir>/src`, keyed relative to it.
    fn src_of(&self, dir: &str) -> Vfs {
        let src = norm_join(dir, "src");
        match self {
            Tree::Disk(_) => {
                let p = Path::new(&src);
                if p.is_dir() {
                    load_dir(p)
                } else {
                    Vfs::new()
                }
            }
            Tree::Map(m) => {
                let pre = format!("{src}/");
                m.iter()
                    .filter_map(|(k, v)| {
                        k.strip_prefix(&pre)
                            .filter(|r| r.ends_with(".rs"))
                            .map(|r| (r.to_string(), v.clone()))
                    })
                    .collect()
            }
        }
    }
}

/// A Cargo.toml read for what the index needs: `[package] name`, and each `[dependencies]`
/// entry as (key, `path`, `workspace = true`) ; `[workspace.dependencies]` as (key, `path`).
#[derive(Default)]
struct Manifest {
    name: String,
    deps: Vec<(String, Option<String>, bool)>,
    ws_deps: BTreeMap<String, Option<String>>,
    is_workspace: bool,
}

/// `key = "v"` inside an inline table `{ .. }` (or a bare `"v"` for key `""`).
fn inline_get(v: &str, key: &str) -> Option<String> {
    let unq = |s: &str| s.trim().trim_matches('"').to_string();
    let inner = v.trim().strip_prefix('{')?.strip_suffix('}')?;
    for part in inner.split(',') {
        let Some((k, x)) = part.split_once('=') else {
            continue;
        };
        if k.trim() == key {
            return Some(unq(x));
        }
    }
    None
}

fn manifest(text: &str) -> Manifest {
    let mut m = Manifest::default();
    let mut sec = String::new();
    for raw in text.lines() {
        let l = raw.split('#').next().unwrap_or("").trim();
        if l.starts_with('[') {
            sec = l.trim_matches(|c| c == '[' || c == ']').trim().to_string();
            if sec == "workspace" {
                m.is_workspace = true;
            }
            if let Some(k) = sec.strip_prefix("dependencies.") {
                m.deps.push((k.to_string(), None, false));
            }
            continue;
        }
        let Some((k, v)) = l.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        let unq = || v.trim_matches('"').to_string();
        match sec.as_str() {
            "package" if k == "name" => m.name = unq(),
            "dependencies" => {
                if let Some((dep, field)) = k.split_once('.') {
                    // `dep.path = ".."` / `dep.workspace = true`
                    let dep = dep.trim().to_string();
                    if !m.deps.iter().any(|d| d.0 == dep) {
                        m.deps.push((dep.clone(), None, false));
                    }
                    let d = m.deps.iter_mut().find(|d| d.0 == dep).expect("a dep");
                    match field.trim() {
                        "path" => d.1 = Some(unq()),
                        "workspace" => d.2 = v == "true",
                        _ => {}
                    }
                } else {
                    let ws = inline_get(v, "workspace").is_some_and(|x| x == "true");
                    m.deps.push((k.to_string(), inline_get(v, "path"), ws));
                }
            }
            "workspace.dependencies" => {
                m.ws_deps.insert(k.to_string(), inline_get(v, "path"));
            }
            s => {
                if let Some(dep) = s.strip_prefix("dependencies.") {
                    if let Some(d) = m.deps.iter_mut().find(|d| d.0 == dep) {
                        match k {
                            "path" => d.1 = Some(unq()),
                            "workspace" => d.2 = v == "true",
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    m
}

/// The crate's name and its path dependencies, transitively (DELTA12 S2 r1: read from the
/// Cargo.toml files, not listed): a `path =` entry, or a `workspace = true` entry whose
/// `[workspace.dependencies]` row has a `path` ; a registry crate (serde) is not read.
fn dep_crates(tree: &Tree<'_>) -> (String, Vec<Krate>) {
    let own_dir = tree.crate_dir();
    let own_path = norm_join(&own_dir, "Cargo.toml");
    let own = manifest(
        &tree
            .read(&own_path)
            .unwrap_or_else(|| panic!("no manifest {own_path}")),
    );
    let ws_path = |dir: &str, key: &str| -> Option<String> {
        let mut up = String::new();
        for _ in 0..12 {
            up.push_str("../");
            let anc = norm_join(dir, &up);
            if let Some(t) = tree.read(&norm_join(&anc, "Cargo.toml")) {
                let m = manifest(&t);
                if m.is_workspace {
                    return m
                        .ws_deps
                        .get(key)
                        .cloned()
                        .flatten()
                        .map(|p| norm_join(&anc, &p));
                }
            }
        }
        None
    };
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = vec![(own_dir, own.deps.clone())];
    while let Some((dir, deps)) = queue.pop() {
        for (key, path, ws) in deps {
            let d = match (path, ws) {
                (Some(p), _) => Some(norm_join(&dir, &p)),
                (None, true) => ws_path(&dir, &key),
                _ => None,
            };
            let Some(d) = d else { continue };
            if !seen.insert(d.clone()) {
                continue;
            }
            let mp = norm_join(&d, "Cargo.toml");
            let m = manifest(&tree.read(&mp).unwrap_or_else(|| panic!("no manifest {mp}")));
            out.push(Krate {
                name: key.replace('-', "_"),
                src: tree.src_of(&d),
            });
            queue.push((d, m.deps));
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    (own.name.replace('-', "_"), out)
}

/// The real crate's tree: its manifest directory (the deps are this crate's whatever
/// `SCOPE_HOME_SRC` points the `src` at).
fn crate_tree() -> Tree<'static> {
    Tree::Disk(env!("CARGO_MANIFEST_DIR").to_string())
}

// =====================================================================================
// Normalised type text
// =====================================================================================

fn nt(t: &Type) -> String {
    match t {
        Type::Array(a) => format!("[{}; _]", nt(&a.elem)),
        Type::BareFn(f) => {
            let ins: Vec<String> = f.inputs.iter().map(|a| nt(&a.ty)).collect();
            format!("fn({}){}", ins.join(", "), nret(&f.output))
        }
        Type::Group(g) => nt(&g.elem),
        Type::ImplTrait(i) => format!("impl {}", nbounds(i.bounds.iter())),
        Type::Infer(_) => "_".into(),
        Type::Macro(m) => format!("{}!(..)", npath(&m.mac.path)),
        Type::Never(_) => "!".into(),
        Type::Paren(p) => format!("({})", nt(&p.elem)),
        Type::Path(p) => match &p.qself {
            Some(q) => format!("<{} as ..>::{}", nt(&q.ty), npath(&p.path)),
            None => npath(&p.path),
        },
        Type::Ptr(p) => format!(
            "*{} {}",
            if p.mutability.is_some() {
                "mut"
            } else {
                "const"
            },
            nt(&p.elem)
        ),
        Type::Reference(r) => format!(
            "&{}{}",
            if r.mutability.is_some() { "mut " } else { "" },
            nt(&r.elem)
        ),
        Type::Slice(s) => format!("[{}]", nt(&s.elem)),
        Type::TraitObject(o) => format!("dyn {}", nbounds(o.bounds.iter())),
        Type::Tuple(t) => {
            let v: Vec<String> = t.elems.iter().map(nt).collect();
            if v.len() == 1 {
                format!("({},)", v[0])
            } else {
                format!("({})", v.join(", "))
            }
        }
        _ => "?".into(),
    }
}

fn nret(r: &syn::ReturnType) -> String {
    match r {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, t) => format!(" -> {}", nt(t)),
    }
}

fn npath(p: &syn::Path) -> String {
    p.segments
        .iter()
        .map(|s| {
            let args = match &s.arguments {
                syn::PathArguments::None => String::new(),
                syn::PathArguments::AngleBracketed(a) => {
                    let v: Vec<String> = a
                        .args
                        .iter()
                        .map(|g| match g {
                            syn::GenericArgument::Lifetime(_) => "'_".to_string(),
                            syn::GenericArgument::Type(t) => nt(t),
                            syn::GenericArgument::AssocType(a) => {
                                format!("{} = {}", a.ident, nt(&a.ty))
                            }
                            syn::GenericArgument::Constraint(c) => {
                                format!("{}: {}", c.ident, nbounds(c.bounds.iter()))
                            }
                            _ => "_".to_string(),
                        })
                        .collect();
                    format!("<{}>", v.join(", "))
                }
                syn::PathArguments::Parenthesized(p) => {
                    let v: Vec<String> = p.inputs.iter().map(nt).collect();
                    format!("({}){}", v.join(", "), nret(&p.output))
                }
            };
            format!("{}{args}", s.ident)
        })
        .collect::<Vec<_>>()
        .join("::")
}

fn nbound(b: &TypeParamBound) -> String {
    match b {
        TypeParamBound::Trait(t) => format!(
            "{}{}",
            if matches!(t.modifier, syn::TraitBoundModifier::Maybe(_)) {
                "?"
            } else {
                ""
            },
            npath(&t.path)
        ),
        TypeParamBound::Lifetime(_) => "'_".into(),
        _ => "?".into(),
    }
}

fn nbounds<'a>(b: impl Iterator<Item = &'a TypeParamBound>) -> String {
    b.map(nbound).collect::<Vec<_>>().join(" + ")
}

fn seg_names(p: &syn::Path) -> Vec<String> {
    p.segments.iter().map(|s| s.ident.to_string()).collect()
}

fn last_name(p: &syn::Path) -> String {
    p.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

fn type_last(t: &Type) -> Option<String> {
    match t {
        Type::Path(p) => Some(last_name(&p.path)),
        Type::Reference(r) => type_last(&r.elem),
        Type::Paren(p) => type_last(&p.elem),
        Type::Group(g) => type_last(&g.elem),
        _ => None,
    }
}

fn is_text_name(n: &str) -> bool {
    TEXT_TYPES.contains(&n)
}

fn is_number_name(n: &str) -> bool {
    NUMBER_TYPES.contains(&n) || NUMBER_PREFIXES.iter().any(|p| n.starts_with(p))
}

/// Every identifier a type names (paths' segments, recursively).
fn type_names(t: &Type, out: &mut BTreeSet<String>) {
    struct V<'o>(&'o mut BTreeSet<String>);
    impl<'ast> Visit<'ast> for V<'_> {
        fn visit_path_segment(&mut self, s: &'ast syn::PathSegment) {
            self.0.insert(s.ident.to_string());
            visit::visit_path_segment(self, s);
        }
    }
    V(out).visit_type(t);
}

// =====================================================================================
// The configuration: the vocabulary (a rule) and the per-module pins
// =====================================================================================

struct Cfg {
    /// vocabulary name -> kind (`struct` | `enum`)
    vocab: BTreeMap<String, String>,
    traits: BTreeSet<String>,
    /// MACROS: name -> sha256 of its definition's token tree
    macros: BTreeMap<String, String>,
    /// id, item, position, type, closes
    leave: Vec<Vec<String>>,
    /// CARD: item, position, type, means
    card: Vec<Vec<String>>,
    /// CB: item, param, kind, args, calls
    cb: Vec<Vec<String>>,
    /// BODY: file, fn, sha256
    body: Vec<Vec<String>>,
    /// INTERIOR's exemptions: item, position, reaches, why
    interior: Vec<Vec<String>>,
    /// item, param, role, holds (the kind and the path IN-TOTAL computes)
    enter: Vec<Vec<String>>,
    /// ADDR: file, fn, form, count, operand
    addr: Vec<Vec<String>>,
    /// ADDR's CALLS: fn, call tokens
    addr_calls: Vec<Vec<String>>,
    /// UNWIND: file, fn, name, count
    unwind: Vec<Vec<String>>,
    /// ADDR_NAMES: form (n1 | n2 | n3) -> names
    addr_names: BTreeMap<String, BTreeSet<String>>,
    enums: BTreeSet<String>,
    /// shim, private fn, wrapper
    shims: Vec<Vec<String>>,
    /// type -> Plain
    outgoing: BTreeMap<String, String>,
    /// item, kind, name, count, flow
    e: Vec<Vec<String>>,
    /// WINDOW: type, trait, deleting unit, since (a dated, named admission of one trait impl)
    window: Vec<Vec<String>>,
    /// BRIDGE: type (as written), trait, class (KEY | REF | TEXT), deleting unit, since (DK1-2b)
    bridge: Vec<Vec<String>>,
}

fn rows_of(text: &str, cols: usize, what: &str) -> Vec<Vec<String>> {
    let rows = table(text);
    for r in &rows {
        assert_eq!(
            r.len(),
            cols,
            "{what}: a row of {} columns, not {cols}: {r:?}",
            r.len()
        );
    }
    rows
}

fn cfg(prefix: &str) -> Cfg {
    cfg_with(prefix, &BTreeMap::new())
}

/// The pins of one prefix, a table of `over` replacing the file's (a fixture's `@@ table:`
/// hunks: a negative control on a pin without a rebuild).
fn cfg_with(prefix: &str, over: &BTreeMap<String, String>) -> Cfg {
    let get =
        |name: &str| -> String { over.get(name).cloned().unwrap_or_else(|| tables::get(name)) };
    let vocab = rows_of(&get("SURF_VOCAB"), 2, "SURF_VOCAB")
        .into_iter()
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    let one = |name: &str| -> BTreeSet<String> {
        rows_of(&get(name), 1, name)
            .into_iter()
            .map(|r| r[0].clone())
            .collect()
    };
    let mut addr_names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in rows_of(&get("ADDR_NAMES"), 2, "ADDR_NAMES") {
        assert!(
            ["n1", "n2", "n3"].contains(&r[0].as_str()),
            "ADDR_NAMES: form {} is not n1 / n2 / n3",
            r[0]
        );
        addr_names
            .entry(r[0].clone())
            .or_default()
            .insert(r[1].clone());
    }
    Cfg {
        vocab,
        traits: one("SURF_TRAITS"),
        macros: rows_of(&get(&format!("{prefix}_MACROS")), 2, "MACROS")
            .into_iter()
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect(),
        leave: rows_of(&get(&format!("{prefix}_LEAVE")), 5, "LEAVE"),
        card: rows_of(&get(&format!("{prefix}_CARD")), 4, "CARD"),
        cb: rows_of(&get(&format!("{prefix}_CB")), 5, "CB"),
        body: rows_of(&get(&format!("{prefix}_BODY")), 3, "BODY"),
        interior: rows_of(&get(&format!("{prefix}_INTERIOR")), 4, "INTERIOR"),
        enter: rows_of(&get(&format!("{prefix}_ENTER")), 4, "ENTER"),
        addr: rows_of(&get(&format!("{prefix}_ADDR")), 5, "ADDR"),
        addr_calls: rows_of(&get(&format!("{prefix}_ADDR_CALLS")), 2, "ADDR_CALLS"),
        unwind: rows_of(&get(&format!("{prefix}_UNWIND")), 4, "UNWIND"),
        addr_names,
        enums: one(&format!("{prefix}_ENUMS")),
        shims: rows_of(&get(&format!("{prefix}_SHIMS")), 3, "SHIMS"),
        outgoing: rows_of(&get(&format!("{prefix}_OUTGOING")), 2, "OUTGOING")
            .into_iter()
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect(),
        e: rows_of(&get(&format!("{prefix}_E")), 5, "E"),
        window: rows_of(&get(&format!("{prefix}_WINDOW")), 4, "WINDOW"),
        bridge: rows_of(&get(&format!("{prefix}_BRIDGE")), 5, "BRIDGE"),
    }
}

// =====================================================================================
// The file set and the items
// =====================================================================================

/// One item where it stands: its file, its module depth below `lexscope` (0 = the root
/// module), the inline `mod` names between the file and it, and whether it sits in a body.
struct At<'a> {
    file: &'a str,
    depth: usize,
    modpath: Vec<String>,
    item: &'a Item,
    in_body: bool,
}

/// Items written inside bodies (fn, method, const, static): recorded, not descended.
struct BodyItems<'a>(Vec<&'a Item>);
impl<'a> Visit<'a> for BodyItems<'a> {
    fn visit_item(&mut self, i: &'a Item) {
        self.0.push(i);
    }
}

fn flatten<'a>(
    file: &'a str,
    items: &'a [Item],
    depth: usize,
    modpath: &[String],
    in_body: bool,
    out: &mut Vec<At<'a>>,
) {
    for it in items {
        out.push(At {
            file,
            depth,
            modpath: modpath.to_vec(),
            item: it,
            in_body,
        });
        if let Item::Mod(m) = it {
            if let Some((_, inner)) = &m.content {
                let mut mp = modpath.to_vec();
                mp.push(m.ident.to_string());
                flatten(file, inner, depth + 1, &mp, in_body, out);
            }
            continue;
        }
        let mut bi = BodyItems(Vec::new());
        visit::visit_item(&mut bi, it);
        for b in bi.0 {
            flatten(file, std::slice::from_ref(b), depth, modpath, true, out);
        }
    }
}

struct FileRec {
    rel: String,
    depth: usize,
    file: syn::File,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Fail {
    item: String,
    pos: String,
    rule: String,
    detail: String,
    file: String,
}

impl Fail {
    fn line(&self) -> String {
        format!(
            "HT-SURF FAIL  {} · {} · {} · {} · {}",
            self.file, self.item, self.pos, self.rule, self.detail
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Row {
    file: String,
    tier: String,
    kind: String,
    item: String,
    pos: String,
    ty: String,
    verdict: String,
    /// `REF` when an OUT position's type holds a reference at any depth (ADDR-FN reads it ;
    /// a column, not a verdict), else `-`
    refs: String,
}

impl Row {
    fn line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.file, self.tier, self.kind, self.item, self.pos, self.ty, self.verdict, self.refs
        )
    }
}

/// What the file set defines (pass 1).
#[derive(Default)]
struct Defs {
    structs: BTreeMap<String, usize>,
    enums: BTreeMap<String, usize>,
    traits: BTreeSet<String>,
    aliases: BTreeMap<String, (Type, bool)>,
    free_fns: BTreeSet<String>,
    /// impl self type (last segment) -> its methods
    type_fns: BTreeMap<String, BTreeSet<String>>,
    macros: BTreeSet<String>,
    mods: BTreeSet<String>,
    /// `impl Outgoing for X`: X -> Plain (normalised)
    outgoing: Vec<(String, Option<Type>)>,
    /// the Elaborator's field types (from outside the file set) ; None = not found
    elab_fields: Option<BTreeMap<String, Type>>,
    /// fns not visible outside (by simple name)
    private_fns: BTreeSet<String>,
}

impl Defs {
    fn defined_type(&self, n: &str) -> bool {
        self.structs.contains_key(n) || self.enums.contains_key(n)
    }
}

fn vis_text(v: &syn::Visibility) -> String {
    match v {
        syn::Visibility::Public(_) => "pub".into(),
        syn::Visibility::Restricted(r) => format!(
            "pub({}{})",
            if r.in_token.is_some() { "in " } else { "" },
            seg_names(&r.path).join("::")
        ),
        syn::Visibility::Inherited => "-".into(),
    }
}

/// Wider than the module: nameable by code outside `crate::lexscope`.
fn vis_wider(v: &syn::Visibility, depth: usize) -> bool {
    match v {
        syn::Visibility::Public(_) => true,
        syn::Visibility::Inherited => false,
        syn::Visibility::Restricted(r) => {
            let s = seg_names(&r.path);
            match s.first().map(String::as_str) {
                Some("self") => false,
                Some("crate") => !(s.len() >= 2 && s[1] == MODULE),
                Some("super") => {
                    if s.iter().all(|x| x == "super") {
                        s.len() > depth
                    } else {
                        true
                    }
                }
                _ => true,
            }
        }
    }
}

fn attr_name(a: &syn::Attribute) -> String {
    last_name(a.path())
}

/// Paths into the module from inside it: `self::`, `super::`, `crate::lexscope::`, or a child
/// module's name.
fn internal_prefix(segs: &[String], defs: &Defs) -> bool {
    match segs.first().map(String::as_str) {
        Some("self") | Some("super") => true,
        Some("crate") => segs.get(1).is_some_and(|s| s == MODULE),
        Some(MODULE) => true,
        Some(s) => defs.mods.contains(s),
        None => false,
    }
}

fn is_self_expr(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p.qself.is_none() && p.path.is_ident("self"),
        Expr::Paren(p) => is_self_expr(&p.expr),
        Expr::Group(g) => is_self_expr(&g.expr),
        _ => false,
    }
}

/// `self`, `&self`, `&mut self`, `*self`, `&mut *self`.
fn is_self_alias(e: &Expr) -> bool {
    match e {
        Expr::Reference(r) => is_self_alias(&r.expr),
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => is_self_alias(&u.expr),
        Expr::Paren(p) => is_self_alias(&p.expr),
        _ => is_self_expr(e),
    }
}

fn tokens_name(ts: &TokenStream, name: &str) -> bool {
    ts.clone().into_iter().any(|t| match t {
        TokenTree::Ident(i) => i == name,
        TokenTree::Group(g) => tokens_name(&g.stream(), name),
        _ => false,
    })
}

// =====================================================================================
// The judge of one position
// =====================================================================================

type Gen = BTreeMap<String, Vec<TypeParamBound>>;

fn gen_of(g: &syn::Generics, out: &mut Gen) {
    for p in g.type_params() {
        out.entry(p.ident.to_string())
            .or_default()
            .extend(p.bounds.iter().cloned());
    }
    if let Some(w) = &g.where_clause {
        for pr in &w.predicates {
            if let syn::WherePredicate::Type(pt) = pr {
                if let Type::Path(tp) = &pt.bounded_ty {
                    if tp.qself.is_none() && tp.path.segments.len() == 1 {
                        out.entry(last_name(&tp.path))
                            .or_default()
                            .extend(pt.bounds.iter().cloned());
                    }
                }
            }
        }
    }
}

type Reasons = Vec<(String, String)>;

#[derive(Default)]
struct InRes {
    /// IN-TOTAL: (`TEXT` | `NUMBER`, detail) by name or by content
    enter: Vec<(String, String)>,
    /// IN-TOTAL: the names the position holds that no list and no index resolves
    unknown: Vec<String>,
    /// (position, type) judged as OUT: a `&mut` target, an interior-mutable parameter
    outs: Vec<(String, Type)>,
    /// (position, type): a callback's argument
    cbs: Vec<(String, Type)>,
    fails: Reasons,
}

struct J<'a> {
    defs: &'a Defs,
    cfg: &'a Cfg,
    tys: &'a TyIdx,
    self_ty: Option<String>,
    gen: Gen,
    in_trait: Option<String>,
}

impl J<'_> {
    fn pass_bound(b: &TypeParamBound) -> bool {
        match b {
            TypeParamBound::Lifetime(_) => true,
            TypeParamBound::Trait(t) => {
                let s = seg_names(&t.path);
                let ok_path = s.len() == 1 || STD_ROOTS.contains(&s[0].as_str());
                let no_args = t
                    .path
                    .segments
                    .iter()
                    .all(|x| matches!(x.arguments, syn::PathArguments::None));
                ok_path && no_args && PASS_BOUNDS.contains(&s[s.len() - 1].as_str())
            }
            _ => false,
        }
    }

    fn has_bound(&self, param: &str, trait_name: &str) -> bool {
        self.gen.get(param).is_some_and(|bs| {
            bs.iter()
                .any(|b| matches!(b, TypeParamBound::Trait(t) if last_name(&t.path) == trait_name))
        })
    }

    /// Judges `t` in an OUT position ; each (rule, detail) is a reason it fails.
    fn out(&self, t: &Type, r: &mut Reasons, depth: usize) {
        if depth > 16 {
            r.push(("UNKNOWN".into(), format!("alias recursion at {}", nt(t))));
            return;
        }
        match t {
            Type::Reference(x) => self.out(&x.elem, r, depth),
            Type::Paren(x) => self.out(&x.elem, r, depth),
            Type::Group(x) => self.out(&x.elem, r, depth),
            Type::Tuple(x) => {
                for e in &x.elems {
                    self.out(e, r, depth);
                }
            }
            Type::Slice(x) => self.out_seq(&x.elem, r, depth),
            Type::Array(x) => self.out_seq(&x.elem, r, depth),
            Type::Never(_) => {}
            Type::Ptr(_) => r.push(("PTR".into(), nt(t))),
            Type::BareFn(_) | Type::ImplTrait(_) | Type::TraitObject(_) => {
                r.push(("OPAQUE".into(), nt(t)))
            }
            Type::Path(p) => self.out_path(t, p, r, depth),
            _ => r.push(("UNKNOWN".into(), nt(t))),
        }
    }

    fn out_seq(&self, elem: &Type, r: &mut Reasons, depth: usize) {
        if type_last(elem).as_deref() == Some("u8") {
            r.push(("TEXT".into(), format!("bytes [{}]", nt(elem))));
        } else {
            self.out(elem, r, depth);
        }
    }

    fn out_args(&self, seg: &syn::PathSegment, r: &mut Reasons, depth: usize) {
        match &seg.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(a) => {
                for g in &a.args {
                    match g {
                        syn::GenericArgument::Lifetime(_) => {}
                        syn::GenericArgument::Type(t) => {
                            if seg.ident == "Vec" && type_last(t).as_deref() == Some("u8") {
                                r.push(("TEXT".into(), format!("bytes {}", nt(t))));
                            } else {
                                self.out(t, r, depth);
                            }
                        }
                        other => r.push((
                            "UNKNOWN".into(),
                            format!("generic argument {}", gen_arg_text(other)),
                        )),
                    }
                }
            }
            syn::PathArguments::Parenthesized(_) => {
                r.push(("OPAQUE".into(), seg.ident.to_string()));
            }
        }
    }

    fn out_path(&self, t: &Type, p: &syn::TypePath, r: &mut Reasons, depth: usize) {
        if p.qself.is_some() {
            r.push(("UNKNOWN".into(), nt(t)));
            return;
        }
        let segs = seg_names(&p.path);
        let n = segs.len();
        let last = &segs[n - 1];
        let last_seg = p.path.segments.last().expect("a segment");
        // an associated type of a type parameter or of `Self` in the boundary trait
        if n == 2 {
            let base = &segs[0];
            let through_boundary = (self.gen.contains_key(base)
                && self.has_bound(base, BOUNDARY_TRAIT))
                || (base == "Self" && self.in_trait.as_deref() == Some(BOUNDARY_TRAIT));
            if through_boundary || self.gen.contains_key(base) || base == "Self" {
                if through_boundary && last == "Plain" && !self.defs.outgoing.is_empty() {
                    for (ty, plain) in &self.defs.outgoing {
                        match plain {
                            Some(pt) => {
                                let mut sub = Vec::new();
                                self.out(pt, &mut sub, depth + 1);
                                for (rule, d) in sub {
                                    r.push((
                                        rule,
                                        format!("{d} (via {base}::Plain = {} of {ty})", nt(pt)),
                                    ));
                                }
                            }
                            None => r.push(("UNKNOWN".into(), format!("{ty}'s Plain not given"))),
                        }
                    }
                } else {
                    r.push(("UNKNOWN".into(), format!("associated type {}", nt(t))));
                }
                return;
            }
        }
        let prefix = &segs[..n - 1];
        if n == 1 {
            if last == "Self" {
                match (&self.self_ty, &self.in_trait) {
                    (Some(s), _) if s == ELAB => {
                        r.push(("UNKNOWN".into(), "Self = Elaborator".into()))
                    }
                    (Some(s), _) => {
                        if !(self.cfg.vocab.contains_key(s) && self.defs.defined_type(s)) {
                            r.push(("UNKNOWN".into(), format!("Self = {s}")));
                        }
                    }
                    _ => r.push(("UNKNOWN".into(), "Self of a trait".into())),
                }
                return;
            }
            if let Some(bounds) = self.gen.get(last) {
                let bad: Vec<String> = bounds
                    .iter()
                    .filter(|b| !Self::pass_bound(b))
                    .map(nbound)
                    .collect();
                if !bad.is_empty() {
                    r.push(("BOUND".into(), format!("{last}: {}", bad.join(" + "))));
                }
                return;
            }
            if last == "bool" {
                return;
            }
            if let Some((ty, has_gen)) = self.defs.aliases.get(last) {
                if *has_gen || !matches!(last_seg.arguments, syn::PathArguments::None) {
                    r.push(("ALIAS-GENERIC".into(), last.clone()));
                    return;
                }
                let mut sub = Vec::new();
                self.out(ty, &mut sub, depth + 1);
                for (rule, d) in sub {
                    r.push((rule, format!("{d} (via alias {last})")));
                }
                return;
            }
        } else if STD_ROOTS.contains(&prefix[0].as_str()) {
            // `std::string::String`, `std::vec::Vec<..>` judged by the last name below
        } else if internal_prefix(prefix, self.defs) {
            if self.cfg.vocab.contains_key(last) && self.defs.defined_type(last) {
                self.out_args(last_seg, r, depth);
            } else {
                r.push(("UNKNOWN".into(), nt(t)));
                self.unknown_args(p, r, depth);
            }
            return;
        } else {
            r.push((
                "UNKNOWN".into(),
                format!("named type of another module {}", nt(t)),
            ));
            self.unknown_args(p, r, depth);
            return;
        }
        if is_text_name(last) {
            r.push(("TEXT".into(), nt(t)));
        } else if is_number_name(last) {
            r.push(("NUMBER".into(), nt(t)));
        } else if WRAPPERS.contains(&last.as_str())
            || (n == 1 && self.cfg.vocab.contains_key(last) && self.defs.defined_type(last))
        {
            self.out_args(last_seg, r, depth);
        } else {
            r.push(("UNKNOWN".into(), nt(t)));
            self.unknown_args(p, r, depth);
        }
    }

    /// An unknown named type is judged through its type arguments too (nested types,
    /// recursively): `BTreeSet<(u32, u32, String)>` fails UNKNOWN, NUMBER and TEXT.
    fn unknown_args(&self, p: &syn::TypePath, r: &mut Reasons, depth: usize) {
        for s in &p.path.segments {
            if let syn::PathArguments::AngleBracketed(a) = &s.arguments {
                for g in &a.args {
                    if let syn::GenericArgument::Type(x) = g {
                        self.out(x, r, depth);
                    }
                }
            }
        }
    }

    /// An IN parameter: its text (ENTER), its OUT parts and its callbacks.
    fn in_param(&self, pname: &str, t: &Type) -> InRes {
        let mut res = InRes::default();
        let mut names = BTreeSet::new();
        type_names(t, &mut names);
        if names.iter().any(|n| CELL_TYPES.contains(&n.as_str())) {
            res.outs.push((format!("ARG:{pname}"), t.clone()));
            return res;
        }
        self.in_walk(pname, t, &mut res);
        res
    }

    fn in_walk(&self, pname: &str, t: &Type, res: &mut InRes) {
        match t {
            Type::Reference(r) => {
                if r.mutability.is_some() {
                    res.outs.push((format!("ARG:{pname}"), (*r.elem).clone()));
                } else {
                    self.in_walk(pname, &r.elem, res);
                }
            }
            Type::Paren(x) => self.in_walk(pname, &x.elem, res),
            Type::Group(x) => self.in_walk(pname, &x.elem, res),
            Type::Tuple(x) => {
                for e in &x.elems {
                    self.in_walk(pname, e, res);
                }
            }
            Type::Slice(x) => self.in_walk(pname, &x.elem, res),
            Type::Array(x) => self.in_walk(pname, &x.elem, res),
            Type::Ptr(_) => res.fails.push(("PTR".into(), nt(t))),
            Type::BareFn(f) => {
                for (i, a) in f.inputs.iter().enumerate() {
                    res.cbs.push((format!("CB:{pname}.{i}"), a.ty.clone()));
                }
                if let syn::ReturnType::Type(_, o) = &f.output {
                    self.in_walk(pname, o, res);
                }
            }
            Type::ImplTrait(i) => self.in_bounds(pname, i.bounds.iter(), res),
            Type::TraitObject(o) => self.in_bounds(pname, o.bounds.iter(), res),
            Type::Never(_) => {}
            Type::Path(p) => {
                if let Some(q) = &p.qself {
                    self.in_walk(pname, &q.ty, res);
                    res.unknown.push(format!("an associated type {}", nt(t)));
                    return;
                }
                let segs = seg_names(&p.path);
                if segs.len() == 1 {
                    if let Some(bs) = self.gen.get(&segs[0]) {
                        let bs = bs.clone();
                        self.in_bounds(pname, bs.iter(), res);
                        return;
                    }
                }
                self.in_name(&segs, t, res);
                for s in &p.path.segments {
                    if let syn::PathArguments::AngleBracketed(a) = &s.arguments {
                        for g in &a.args {
                            if let syn::GenericArgument::Type(x) = g {
                                self.in_walk(pname, x, res);
                            }
                        }
                    }
                }
            }
            _ => res.fails.push(("UNKNOWN".into(), nt(t))),
        }
    }

    /// IN-TOTAL (DELTA12 S2 r2) on one written name of an IN position, first match: a type
    /// parameter (the caller's) ; a TEXT name ; a NUMBER name ; `bool` ; a vocabulary type of
    /// the file set ; `Self` (its impl's type) ; a name the index defines, FOLLOWED through its
    /// fields to text, else to a number ; a `use .. as` rename (its original) ; a container
    /// of IN_CONTAINERS (its arguments, walked by the caller) ; anything else IN-UNKNOWN.
    fn in_name(&self, segs: &[String], t: &Type, res: &mut InRes) {
        let vocab = |n: &str| self.cfg.vocab.contains_key(n) && self.defs.defined_type(n);
        if segs.len() == 1 && segs[0] == "Self" {
            match &self.self_ty {
                Some(s) if vocab(s) => {}
                Some(s) => {
                    let ds = self
                        .tys
                        .resolve(&["crate".into(), s.clone()], &self.tys.own);
                    if ds.is_empty() {
                        res.unknown.push(format!("Self = {s}"));
                    } else {
                        let f = self.tys.follow(&ds, &format!("Self = {s}"), &vocab);
                        f.into_res(res);
                    }
                }
                None => res.unknown.push("Self of a trait".into()),
            }
            return;
        }
        match self.tys.classify(segs, &self.tys.own, &vocab) {
            NameIs::Text => res.enter.push(("TEXT".into(), nt(t))),
            NameIs::Number => res.enter.push(("NUMBER".into(), nt(t))),
            NameIs::Nothing | NameIs::Container => {}
            NameIs::Follow(ds) => self.tys.follow(&ds, "", &vocab).into_res(res),
            NameIs::Unknown => res.unknown.push(nt(t)),
        }
    }

    fn in_bounds<'b>(
        &self,
        pname: &str,
        bounds: impl Iterator<Item = &'b TypeParamBound>,
        res: &mut InRes,
    ) {
        for b in bounds {
            match b {
                TypeParamBound::Lifetime(_) => {}
                TypeParamBound::Trait(tb) => {
                    let last = tb.path.segments.last().expect("a segment");
                    let name = last.ident.to_string();
                    if FN_TRAITS.contains(&name.as_str()) {
                        if let syn::PathArguments::Parenthesized(pa) = &last.arguments {
                            for (i, a) in pa.inputs.iter().enumerate() {
                                res.cbs.push((format!("CB:{pname}.{i}"), a.clone()));
                            }
                            if let syn::ReturnType::Type(_, o) = &pa.output {
                                self.in_walk(pname, o, res);
                            }
                        } else {
                            res.fails.push(("IN-TRAIT".into(), nbound(b)));
                        }
                    } else if !(Self::pass_bound(b)
                        || (self.defs.traits.contains(&name) && self.cfg.traits.contains(&name)))
                    {
                        res.fails.push(("IN-TRAIT".into(), nbound(b)));
                    }
                }
                _ => res.fails.push(("IN-TRAIT".into(), nbound(b))),
            }
        }
    }

    /// CB: every callback a parameter's type holds, at any depth: (kind, argument types).
    fn callbacks(&self, t: &Type, out: &mut Vec<(String, Vec<Type>)>, depth: usize) {
        if depth > 16 {
            return;
        }
        match t {
            Type::Reference(x) => self.callbacks(&x.elem, out, depth),
            Type::Paren(x) => self.callbacks(&x.elem, out, depth),
            Type::Group(x) => self.callbacks(&x.elem, out, depth),
            Type::Ptr(x) => self.callbacks(&x.elem, out, depth),
            Type::Slice(x) => self.callbacks(&x.elem, out, depth),
            Type::Array(x) => self.callbacks(&x.elem, out, depth),
            Type::Tuple(x) => {
                for e in &x.elems {
                    self.callbacks(e, out, depth);
                }
            }
            Type::BareFn(f) => {
                out.push(("fn".into(), f.inputs.iter().map(|a| a.ty.clone()).collect()))
            }
            Type::ImplTrait(i) => Self::cb_bounds(i.bounds.iter(), out),
            Type::TraitObject(o) => Self::cb_bounds(o.bounds.iter(), out),
            Type::Path(p) => {
                if let Some(q) = &p.qself {
                    self.callbacks(&q.ty, out, depth + 1);
                }
                if p.path.segments.len() == 1 {
                    if let Some(bs) = self.gen.get(&last_name(&p.path)) {
                        Self::cb_bounds(bs.iter(), out);
                    }
                }
                for s in &p.path.segments {
                    if let syn::PathArguments::AngleBracketed(a) = &s.arguments {
                        for g in &a.args {
                            if let syn::GenericArgument::Type(x) = g {
                                self.callbacks(x, out, depth + 1);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn cb_bounds<'b>(
        bounds: impl Iterator<Item = &'b TypeParamBound>,
        out: &mut Vec<(String, Vec<Type>)>,
    ) {
        for b in bounds {
            let TypeParamBound::Trait(tb) = b else {
                continue;
            };
            let last = tb.path.segments.last().expect("a segment");
            let name = last.ident.to_string();
            if FN_TRAITS.contains(&name.as_str()) {
                let args = match &last.arguments {
                    syn::PathArguments::Parenthesized(pa) => pa.inputs.iter().cloned().collect(),
                    _ => Vec::new(),
                };
                out.push((name, args));
            }
        }
    }
}

fn gen_arg_text(g: &syn::GenericArgument) -> String {
    match g {
        syn::GenericArgument::AssocType(a) => format!("{} = {}", a.ident, nt(&a.ty)),
        syn::GenericArgument::Constraint(c) => format!("{}: {}", c.ident, nbounds(c.bounds.iter())),
        syn::GenericArgument::Const(_) => "a const".into(),
        syn::GenericArgument::AssocConst(a) => format!("{} = a const", a.ident),
        _ => "?".into(),
    }
}

/// CARD's verdict text for one position: its constructors, and its `n` bools when the ITEM
/// holds two or more over its OUT positions (`total`, DELTA12 N-1: a cardinality is what an
/// item hands back in one call).
fn card_text(mut found: BTreeSet<String>, n: usize, total: usize, t: &Type) -> Option<String> {
    if total >= 2 && n >= 1 {
        found.insert(if n == total {
            format!("{n} bool")
        } else {
            format!("{n} of {total} bool over the item")
        });
    }
    if found.is_empty() {
        None
    } else {
        Some(format!(
            "{} in {}",
            found.into_iter().collect::<Vec<_>>().join(", "),
            nt(t)
        ))
    }
}

/// WHOLE (DELTA12 S2 r3): where a body uses parameter `p` other than whole — the base of a
/// field access, a method call or an index ; the scrutinee of a `match`, an `if let` /
/// `while let` or a `for` (macro arguments read as an expression list, else by token).
struct WholeUse<'p> {
    p: &'p str,
    hits: Vec<String>,
}

fn base_ident(e: &Expr) -> Option<String> {
    match e {
        Expr::Paren(x) => base_ident(&x.expr),
        Expr::Group(x) => base_ident(&x.expr),
        Expr::Reference(x) => base_ident(&x.expr),
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => base_ident(&u.expr),
        Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
            Some(p.path.segments[0].ident.to_string())
        }
        _ => None,
    }
}

impl WholeUse<'_> {
    fn is_p(&self, e: &Expr) -> bool {
        base_ident(e).as_deref() == Some(self.p)
    }
}

impl<'ast> Visit<'ast> for WholeUse<'_> {
    fn visit_expr_field(&mut self, e: &'ast syn::ExprField) {
        if self.is_p(&e.base) {
            let m = match &e.member {
                syn::Member::Named(n) => n.to_string(),
                syn::Member::Unnamed(i) => i.index.to_string(),
            };
            self.hits.push(format!("{}.{m}", self.p));
        }
        visit::visit_expr_field(self, e);
    }
    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        if self.is_p(&e.receiver) {
            self.hits.push(format!("{}.{}()", self.p, e.method));
        }
        visit::visit_expr_method_call(self, e);
    }
    fn visit_expr_index(&mut self, e: &'ast syn::ExprIndex) {
        if self.is_p(&e.expr) {
            self.hits.push(format!("{}[..]", self.p));
        }
        visit::visit_expr_index(self, e);
    }
    fn visit_expr_match(&mut self, e: &'ast syn::ExprMatch) {
        if self.is_p(&e.expr) {
            self.hits.push(format!("match {}", self.p));
        }
        visit::visit_expr_match(self, e);
    }
    fn visit_expr_let(&mut self, e: &'ast syn::ExprLet) {
        if self.is_p(&e.expr) {
            self.hits.push(format!("let .. = {}", self.p));
        }
        visit::visit_expr_let(self, e);
    }
    fn visit_expr_for_loop(&mut self, e: &'ast syn::ExprForLoop) {
        if self.is_p(&e.expr) {
            self.hits.push(format!("for .. in {}", self.p));
        }
        visit::visit_expr_for_loop(self, e);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        match Punctuated::<Expr, Token![,]>::parse_terminated.parse2(m.tokens.clone()) {
            Ok(list) => {
                for e in &list {
                    self.visit_expr(e);
                }
            }
            Err(_) => {
                if tokens_name(&m.tokens, self.p) {
                    self.hits
                        .push(format!("macro {}! names {}", last_name(&m.path), self.p));
                }
            }
        }
    }
}

/// A callback argument naming the Elaborator through `Self` (marks tier E, not judged).
fn is_self_ty(t: &Type) -> bool {
    match t {
        Type::Reference(r) => is_self_ty(&r.elem),
        Type::Paren(p) => is_self_ty(&p.elem),
        Type::Path(p) => p.qself.is_none() && p.path.is_ident("Self"),
        _ => false,
    }
}

// =====================================================================================
// INTERIOR: a type name of the crate followed through its fields to a mutable carrier
// =====================================================================================

/// INTERIOR's carriers, by a path's last segment (`Atomic*` by prefix, `*mut` by syntax).
const CARRIERS: &[&str] = &[
    "Cell",
    "RefCell",
    "OnceCell",
    "LazyCell",
    "UnsafeCell",
    "Mutex",
    "RwLock",
    "OnceLock",
    "LazyLock",
    "Sender",
    "SyncSender",
    "Receiver",
];

fn is_carrier(n: &str) -> bool {
    CARRIERS.contains(&n) || (n.starts_with("Atomic") && n.len() > "Atomic".len())
}

/// One definition of a type name in the crate's source or a path dependency's: its fields as
/// (hop, type) ; a foreign definition's hops carry its crate (`hdl_ast::Ident.name`).
struct TyDef {
    krate: String,
    file: String,
    kind: &'static str,
    generics: BTreeSet<String>,
    fields: Vec<(String, Type)>,
}

/// Every struct / enum / union / type alias the crate's source defines (every file of
/// `src`, the file set included, items in bodies, inline and `#[cfg]` modules alike) and
/// every one the `src` of its path dependencies defines (DELTA12 S2 r1, read from the
/// Cargo.toml files), by name ; every `use .. as <name>` rename, by the new name ; and the
/// crate aliases (`use hdl_ast as ast;`) each crate's `lib.rs` (and the file set) declares.
#[derive(Default)]
struct TyIdx {
    /// the crate under test (`elaborate`, a fixture's `clean`)
    own: String,
    /// every crate read: the own crate and its path dependencies
    krates: BTreeSet<String>,
    defs: BTreeMap<String, Vec<TyDef>>,
    /// new name -> (the original's last name, the use text)
    renames: BTreeMap<String, Vec<(String, String)>>,
    /// new name -> (the crate the `use` stands in, the original's full path)
    rename_paths: BTreeMap<String, Vec<(String, Vec<String>)>>,
    /// crate -> alias -> the aliased crate
    aliases: BTreeMap<String, BTreeMap<String, String>>,
}

/// What IN-TOTAL makes of one written name (DELTA12 S2 r2).
enum NameIs {
    Text,
    Number,
    Nothing,
    Container,
    /// the definitions the index gives the name: (name, index into `defs[name]`)
    Follow(Vec<(String, usize)>),
    Unknown,
}

/// IN-TOTAL's walk from a set of definitions through their fields.
#[derive(Default)]
struct Followed {
    text: Option<String>,
    number: Option<String>,
    unknown: Vec<String>,
}

impl Followed {
    fn into_res(self, res: &mut InRes) {
        if let Some(t) = self.text {
            res.enter.push(("TEXT".into(), t));
        } else if let Some(n) = self.number {
            res.enter.push(("NUMBER".into(), n));
        }
        res.unknown.extend(self.unknown);
    }
}

impl TyIdx {
    fn build(vfs: &Vfs, own: &str, deps: &[Krate]) -> TyIdx {
        let mut ix = TyIdx {
            own: own.to_string(),
            ..TyIdx::default()
        };
        ix.krates.insert(own.to_string());
        for d in deps {
            ix.krates.insert(d.name.clone());
        }
        let crates: Vec<(&str, &Vfs)> = std::iter::once((own, vfs))
            .chain(deps.iter().map(|d| (d.name.as_str(), &d.src)))
            .collect();
        for (krate, files) in crates {
            ix.index_crate(krate, krate == own, files);
        }
        ix
    }

    fn index_crate(&mut self, krate: &str, is_own: bool, vfs: &Vfs) {
        let ix = self;
        for (rel, text) in vfs {
            let Ok(f) = syn::parse_file(text) else {
                continue; // a parse failure is reported by the engine's own read
            };
            let mut ats = Vec::new();
            flatten(rel, &f.items, 0, &[], false, &mut ats);
            let declares_aliases = rel == "lib.rs"
                || (is_own && rel.starts_with(&format!("{MODULE}/")))
                || (is_own && rel == &format!("{MODULE}.rs"));
            let owner_of = |n: &str| -> String {
                if is_own {
                    n.to_string()
                } else {
                    format!("{krate}::{n}")
                }
            };
            for at in &ats {
                let gens = |g: &syn::Generics| -> BTreeSet<String> {
                    g.type_params().map(|p| p.ident.to_string()).collect()
                };
                let fields_of = |owner: &str, fs: &syn::Fields| -> Vec<(String, Type)> {
                    fs.iter()
                        .enumerate()
                        .map(|(i, fd)| {
                            let l = fd
                                .ident
                                .as_ref()
                                .map(|x| x.to_string())
                                .unwrap_or_else(|| i.to_string());
                            (format!("{owner}.{l}"), fd.ty.clone())
                        })
                        .collect()
                };
                let (name, def) = match at.item {
                    Item::Struct(s) => {
                        let n = s.ident.to_string();
                        let fields = fields_of(&owner_of(&n), &s.fields);
                        (n, ("struct", gens(&s.generics), fields))
                    }
                    Item::Enum(e) => {
                        let n = e.ident.to_string();
                        let mut fields = Vec::new();
                        for v in &e.variants {
                            fields.extend(fields_of(
                                &format!("{}::{}", owner_of(&n), v.ident),
                                &v.fields,
                            ));
                        }
                        (n, ("enum", gens(&e.generics), fields))
                    }
                    Item::Union(u) => {
                        let n = u.ident.to_string();
                        let fields = u
                            .fields
                            .named
                            .iter()
                            .map(|fd| {
                                let l =
                                    fd.ident.as_ref().map(|x| x.to_string()).unwrap_or_default();
                                (format!("{}.{l}", owner_of(&n)), fd.ty.clone())
                            })
                            .collect();
                        (n, ("union", gens(&u.generics), fields))
                    }
                    Item::Type(t) => {
                        let n = t.ident.to_string();
                        let fields = vec![(format!("type {}", owner_of(&n)), (*t.ty).clone())];
                        (n, ("alias", gens(&t.generics), fields))
                    }
                    Item::Use(u) => {
                        let mut v = Vec::new();
                        use_renames(&mut Vec::new(), &u.tree, &mut v);
                        let mut full = Vec::new();
                        use_rename_paths(&mut Vec::new(), &u.tree, &mut full);
                        for (new, orig, text) in v {
                            ix.renames.entry(new).or_default().push((orig, text));
                        }
                        for (new, path) in full {
                            if declares_aliases && path.len() == 1 {
                                ix.aliases
                                    .entry(krate.to_string())
                                    .or_default()
                                    .insert(new.clone(), path[0].clone());
                            }
                            ix.rename_paths
                                .entry(new)
                                .or_default()
                                .push((krate.to_string(), path));
                        }
                        continue;
                    }
                    _ => continue,
                };
                ix.defs.entry(name).or_default().push(TyDef {
                    krate: krate.to_string(),
                    file: if is_own {
                        rel.clone()
                    } else {
                        format!("{krate}:{rel}")
                    },
                    kind: def.0,
                    generics: def.1,
                    fields: def.2,
                });
            }
        }
    }

    /// The definitions a written path names (DELTA12 S2 r1): by its first segment when that
    /// is `crate` / `self` / `super` (the crate `ctx`), a crate alias `ctx` declares, or a
    /// crate's own name ; any other path, and a bare name, through every definition of its
    /// last name in the index (INTERIOR's D-1: a false hit is loud).
    fn resolve(&self, segs: &[String], ctx: &str) -> Vec<(String, usize)> {
        let Some(last) = segs.last() else {
            return Vec::new();
        };
        let Some(ds) = self.defs.get(last) else {
            return Vec::new();
        };
        let filter: Option<String> = if segs.len() == 1 {
            None
        } else {
            let first = segs[0].as_str();
            if matches!(first, "crate" | "self" | "super") {
                Some(ctx.to_string())
            } else if let Some(k) = self.aliases.get(ctx).and_then(|m| m.get(first)) {
                Some(k.clone())
            } else if self.krates.contains(first) {
                Some(first.to_string())
            } else {
                None
            }
        };
        ds.iter()
            .enumerate()
            .filter(|(_, d)| filter.as_ref().is_none_or(|k| &d.krate == k))
            .map(|(i, _)| (last.clone(), i))
            .collect()
    }

    /// IN-TOTAL's verdict on one written name (no type parameter, no `Self`), in the crate
    /// `ctx`: TEXT, NUMBER, `bool`, a vocabulary name, the index, a rename, a container.
    fn classify(&self, segs: &[String], ctx: &str, vocab: &dyn Fn(&str) -> bool) -> NameIs {
        let Some(last) = segs.last() else {
            return NameIs::Unknown;
        };
        if is_text_name(last) {
            return NameIs::Text;
        }
        if is_number_name(last) {
            return NameIs::Number;
        }
        if segs.len() == 1 && last == "bool" {
            return NameIs::Nothing;
        }
        if vocab(last) {
            return NameIs::Nothing;
        }
        let ds = self.resolve(segs, ctx);
        if !ds.is_empty() {
            return NameIs::Follow(ds);
        }
        if segs.len() == 1 {
            if let Some(rs) = self.rename_paths.get(last) {
                // a rename: its original, read in the crate whose `use` renamed it
                let mut follow = Vec::new();
                let mut best: Option<NameIs> = None;
                for (k, orig) in rs {
                    match self.classify(orig, k, vocab) {
                        NameIs::Follow(ds) => follow.extend(ds),
                        NameIs::Text => return NameIs::Text,
                        other => {
                            if best.is_none() || matches!(other, NameIs::Number) {
                                best = Some(other);
                            }
                        }
                    }
                }
                if !follow.is_empty() {
                    return NameIs::Follow(follow);
                }
                if let Some(b) = best {
                    return b;
                }
            }
        }
        if IN_CONTAINERS.contains(&last.as_str()) {
            return NameIs::Container;
        }
        NameIs::Unknown
    }

    /// IN-TOTAL's FOLLOWED (DELTA12 S2 r2): a breadth-first walk from `start` through the
    /// fields of every definition reached (a visited set), each written name judged by
    /// `classify` in its definition's crate ; the shortest path to a TEXT name, else to a
    /// NUMBER name, and every name on the way no list and no index resolves (a hop the index
    /// cannot open: a raw pointer, a `dyn` / `impl` / `fn` type, a macro or associated type).
    fn follow(
        &self,
        start: &[(String, usize)],
        head: &str,
        vocab: &dyn Fn(&str) -> bool,
    ) -> Followed {
        let mut out = Followed::default();
        let mut seen: BTreeSet<(String, usize)> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<((String, usize), Vec<String>)> =
            std::collections::VecDeque::new();
        let head: Vec<String> = if head.is_empty() {
            Vec::new()
        } else {
            vec![head.to_string()]
        };
        for s in start {
            if seen.insert(s.clone()) {
                queue.push_back((s.clone(), head.clone()));
            }
        }
        while let Some(((name, i), path)) = queue.pop_front() {
            let d = &self.defs[&name][i];
            for (hop, ft) in &d.fields {
                let mut p = path.clone();
                p.push(hop.clone());
                self.hop(
                    ft,
                    &p,
                    &(name.clone(), i),
                    vocab,
                    &mut out,
                    &mut seen,
                    &mut queue,
                );
            }
        }
        out.unknown.sort();
        out.unknown.dedup();
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn hop(
        &self,
        t: &Type,
        path: &[String],
        me: &(String, usize),
        vocab: &dyn Fn(&str) -> bool,
        out: &mut Followed,
        seen: &mut BTreeSet<(String, usize)>,
        queue: &mut std::collections::VecDeque<((String, usize), Vec<String>)>,
    ) {
        let at = |what: String| -> String {
            let mut p = path.to_vec();
            p.push(what);
            p.join(" -> ")
        };
        let d = &self.defs[&me.0][me.1];
        match t {
            Type::Reference(x) => self.hop(&x.elem, path, me, vocab, out, seen, queue),
            Type::Paren(x) => self.hop(&x.elem, path, me, vocab, out, seen, queue),
            Type::Group(x) => self.hop(&x.elem, path, me, vocab, out, seen, queue),
            Type::Slice(x) => self.hop(&x.elem, path, me, vocab, out, seen, queue),
            Type::Array(x) => self.hop(&x.elem, path, me, vocab, out, seen, queue),
            Type::Tuple(x) => {
                for e in &x.elems {
                    self.hop(e, path, me, vocab, out, seen, queue);
                }
            }
            Type::Never(_) => {}
            Type::Path(p) if p.qself.is_none() => {
                let segs = seg_names(&p.path);
                let last = segs.last().cloned().unwrap_or_default();
                let enqueue =
                    |ds: Vec<(String, usize)>,
                     seen: &mut BTreeSet<(String, usize)>,
                     queue: &mut std::collections::VecDeque<_>| {
                        for x in ds {
                            if seen.insert(x.clone()) {
                                queue.push_back((x, path.to_vec()));
                            }
                        }
                    };
                if segs.len() == 1 && d.generics.contains(&last) {
                    // the definition's own type parameter: its argument is the use site's
                } else if segs.len() == 1 && last == "Self" {
                    enqueue(vec![me.clone()], seen, queue);
                } else {
                    match self.classify(&segs, &d.krate, vocab) {
                        NameIs::Text => {
                            if out.text.is_none() {
                                out.text = Some(at(nt(t)));
                            }
                        }
                        NameIs::Number => {
                            if out.number.is_none() {
                                out.number = Some(at(nt(t)));
                            }
                        }
                        NameIs::Nothing | NameIs::Container => {}
                        NameIs::Follow(ds) => enqueue(ds, seen, queue),
                        NameIs::Unknown => out.unknown.push(at(nt(t))),
                    }
                }
                for s in &p.path.segments {
                    if let syn::PathArguments::AngleBracketed(a) = &s.arguments {
                        for g in &a.args {
                            if let syn::GenericArgument::Type(x) = g {
                                self.hop(x, path, me, vocab, out, seen, queue);
                            }
                        }
                    }
                }
            }
            _ => out
                .unknown
                .push(at(format!("{} (a hop the index cannot open)", nt(t)))),
        }
    }

    /// INTERIOR: the shortest path of names from `t` to a mutable carrier, or None. A name
    /// is followed when the crate defines it (every definition of that name), when it is a
    /// `use .. as` rename (to the original's name), or `Self` (to `self_ty`) ; a name in
    /// `skip` (the type parameters in scope) and a name the crate does not define are not
    /// followed (their type arguments are still read: they are spelled by `t`).
    fn interior(&self, t: &Type, self_ty: Option<&str>, skip: &BTreeSet<String>) -> Option<String> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<(String, Vec<String>)> =
            std::collections::VecDeque::new();
        if let Some(found) = self.step(t, &[], self_ty, skip, &mut seen, &mut queue) {
            return Some(found);
        }
        while let Some((name, path)) = queue.pop_front() {
            for d in self.defs.get(&name).into_iter().flatten() {
                for (hop, ft) in &d.fields {
                    let mut p = path.clone();
                    p.push(hop.clone());
                    if let Some(found) =
                        self.step(ft, &p, Some(&name), &d.generics, &mut seen, &mut queue)
                    {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    /// Reads the names `t` spells: a carrier ends the walk ; a followable name is queued.
    fn step(
        &self,
        t: &Type,
        path: &[String],
        self_ty: Option<&str>,
        skip: &BTreeSet<String>,
        seen: &mut BTreeSet<String>,
        queue: &mut std::collections::VecDeque<(String, Vec<String>)>,
    ) -> Option<String> {
        let mut spelled = Vec::new();
        spelled_names(t, &mut spelled);
        for n in spelled {
            let mut p = path.to_vec();
            if is_carrier(&n) || n == "*mut" {
                p.push(n);
                return Some(p.join(" -> "));
            }
            let mut target = n.clone();
            if n == "Self" {
                match self_ty {
                    Some(s) => {
                        p.push(format!("Self = {s}"));
                        target = s.to_string();
                    }
                    None => continue,
                }
            } else if skip.contains(&n) {
                continue;
            } else if let Some(rs) = self.renames.get(&n) {
                // a rename to a carrier ends the walk ; to any other name, that name is followed
                for (orig, text) in rs {
                    if is_carrier(orig) {
                        p.push(format!("use {text}"));
                        p.push(orig.clone());
                        return Some(p.join(" -> "));
                    }
                }
                // the name itself when the crate also defines it (both readings followed)
                if self.defs.contains_key(&n) && seen.insert(n.clone()) {
                    queue.push_back((n.clone(), p.clone()));
                }
                for (orig, text) in rs {
                    let mut q = p.clone();
                    q.push(format!("use {text}"));
                    if self.defs.contains_key(orig) && seen.insert(orig.clone()) {
                        queue.push_back((orig.clone(), q));
                    }
                }
                continue;
            }
            if self.defs.contains_key(&target) && seen.insert(target.clone()) {
                queue.push_back((target, p));
            }
        }
        None
    }
}

/// INTERIOR's census of the crate (dumped, not pinned): one row per definition, with the
/// path to a carrier it reaches, whether a field of its own spells one, and whether the
/// Elaborator's fields reach the name.
fn interior_census(ix: &TyIdx) -> Vec<String> {
    let mut from_elab: BTreeSet<String> = BTreeSet::new();
    let mut queue = vec![ELAB.to_string()];
    while let Some(n) = queue.pop() {
        if !from_elab.insert(n.clone()) {
            continue;
        }
        for d in ix.defs.get(&n).into_iter().flatten() {
            for (_, ft) in &d.fields {
                let mut sp = Vec::new();
                spelled_names(ft, &mut sp);
                for m in sp {
                    let targets: Vec<String> = match ix.renames.get(&m) {
                        Some(rs) => rs.iter().map(|(o, _)| o.clone()).collect(),
                        None => vec![m.clone()],
                    };
                    for t in targets {
                        if ix.defs.contains_key(&t) && !d.generics.contains(&t) {
                            queue.push(t);
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (name, ds) in &ix.defs {
        for d in ds {
            let ty: Type = syn::parse_str(name).expect("a type name");
            let Some(path) = ix.interior(&ty, None, &BTreeSet::new()) else {
                continue;
            };
            let direct = d.fields.iter().any(|(_, ft)| {
                let mut sp = Vec::new();
                spelled_names(ft, &mut sp);
                sp.iter().any(|n| is_carrier(n) || n == "*mut")
            });
            out.push(format!(
                "{}\t{}\t{name}\t{}\t{}\t{path}",
                d.file,
                d.kind,
                if direct { "holds" } else { "reaches" },
                if from_elab.contains(name) {
                    "elab"
                } else {
                    "-"
                },
            ));
        }
    }
    out
}

/// INTERIOR applied to every typed parameter of every fn of the crate (free fns and impl /
/// trait methods ; a measurement of where the shape occurs, dumped, not pinned).
fn interior_params(vfs: &Vfs, ix: &TyIdx) -> Vec<String> {
    let mut out = Vec::new();
    for (rel, text) in vfs {
        let Ok(f) = syn::parse_file(text) else {
            continue;
        };
        let mut ats = Vec::new();
        flatten(rel, &f.items, 0, &[], false, &mut ats);
        let mut one = |item: String, sig: &syn::Signature, self_ty: Option<String>, g: &Gen| {
            let mut gen = g.clone();
            gen_of(&sig.generics, &mut gen);
            let skip: BTreeSet<String> = gen.keys().cloned().collect();
            for (i, a) in sig.inputs.iter().enumerate() {
                let FnArg::Typed(pt) = a else { continue };
                let pname = match &*pt.pat {
                    syn::Pat::Ident(pi) => pi.ident.to_string(),
                    _ => format!("_{i}"),
                };
                if let Some(p) = ix.interior(&pt.ty, self_ty.as_deref(), &skip) {
                    out.push(format!("{rel}\t{item}\t{pname}\t{}\t{p}", nt(&pt.ty)));
                }
            }
        };
        for at in &ats {
            match at.item {
                Item::Fn(x) => one(x.sig.ident.to_string(), &x.sig, None, &Gen::new()),
                Item::Impl(im) => {
                    let ty = impl_self_name(&im.self_ty);
                    let mut g = Gen::new();
                    gen_of(&im.generics, &mut g);
                    for ii in &im.items {
                        if let syn::ImplItem::Fn(x) = ii {
                            one(
                                format!("{ty}::{}", x.sig.ident),
                                &x.sig,
                                Some(ty.clone()),
                                &g,
                            );
                        }
                    }
                }
                Item::Trait(t) => {
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(x) = ti {
                            one(
                                format!("{}::{}", t.ident, x.sig.ident),
                                &x.sig,
                                None,
                                &Gen::new(),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// IN-TOTAL's index, measured (dumped, not pinned): one row per definition of every crate
/// read, with what FOLLOWED reaches from it alone and how many names on the way are unknown.
fn in_index_census(ix: &TyIdx) -> Vec<String> {
    let mut out = Vec::new();
    for (name, ds) in &ix.defs {
        for (i, d) in ds.iter().enumerate() {
            let f = ix.follow(&[(name.clone(), i)], "", &|_: &str| false);
            let holds = match (&f.text, &f.number) {
                (Some(t), _) => format!("TEXT {t}"),
                (None, Some(n)) => format!("NUMBER {n}"),
                _ => "-".into(),
            };
            out.push(format!(
                "{}\t{name}\t{}\t{}\t{holds}\t{}",
                d.krate,
                d.kind,
                d.file,
                f.unknown.len()
            ));
        }
    }
    out
}

/// IN-TOTAL's foreign names, measured (dumped, not pinned): every written type name in a
/// typed parameter of a crate-visible fn (a free fn or an inherent method marked `pub` /
/// `pub(..)`) that the index resolves into a path dependency, by (crate, name), with what
/// FOLLOWED reaches from that crate's definitions, its uses and its first use.
fn in_foreign_census(vfs: &Vfs, ix: &TyIdx) -> Vec<String> {
    struct P(Vec<Vec<String>>);
    impl<'ast> Visit<'ast> for P {
        fn visit_type_path(&mut self, p: &'ast syn::TypePath) {
            if p.qself.is_none() {
                self.0.push(seg_names(&p.path));
            }
            visit::visit_type_path(self, p);
        }
    }
    /// (uses, first use, the definitions reached)
    type Uses = (usize, String, Vec<(String, usize)>);
    let mut got: BTreeMap<(String, String), Uses> = BTreeMap::new();
    for (rel, text) in vfs {
        let Ok(f) = syn::parse_file(text) else {
            continue;
        };
        let mut ats = Vec::new();
        flatten(rel, &f.items, 0, &[], false, &mut ats);
        let mut sigs: Vec<(String, &syn::Signature)> = Vec::new();
        for at in &ats {
            match at.item {
                Item::Fn(x) if !matches!(x.vis, syn::Visibility::Inherited) => {
                    sigs.push((x.sig.ident.to_string(), &x.sig))
                }
                Item::Impl(im) if im.trait_.is_none() => {
                    for ii in &im.items {
                        if let syn::ImplItem::Fn(x) = ii {
                            if !matches!(x.vis, syn::Visibility::Inherited) {
                                sigs.push((x.sig.ident.to_string(), &x.sig));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for (fname, sig) in sigs {
            for a in &sig.inputs {
                let FnArg::Typed(pt) = a else { continue };
                let mut v = P(Vec::new());
                v.visit_type(&pt.ty);
                for segs in v.0 {
                    let NameIs::Follow(ds) = ix.classify(&segs, &ix.own, &|_: &str| false) else {
                        continue;
                    };
                    for (n, i) in ds {
                        let k = ix.defs[&n][i].krate.clone();
                        if k == ix.own {
                            continue;
                        }
                        let e = got
                            .entry((k, n.clone()))
                            .or_insert_with(|| (0, format!("{rel} {fname}"), Vec::new()));
                        e.0 += 1;
                        if !e.2.contains(&(n.clone(), i)) {
                            e.2.push((n.clone(), i));
                        }
                    }
                }
            }
        }
    }
    got.into_iter()
        .map(|((k, n), (uses, first, ds))| {
            let f = ix.follow(&ds, "", &|_: &str| false);
            let holds = match (&f.text, &f.number) {
                (Some(t), _) => format!("TEXT {t}"),
                (None, Some(x)) => format!("NUMBER {x}"),
                _ => "-".into(),
            };
            format!("{k}\t{n}\t{holds}\t{uses}\t{first}")
        })
        .collect()
}

/// The names a type spells, in syntax order: every type path's last segment (generic
/// arguments, callback arguments and bounds' arguments included) and `*mut` for a mutable
/// raw pointer.
fn spelled_names(t: &Type, out: &mut Vec<String>) {
    struct V<'o>(&'o mut Vec<String>);
    impl<'ast> Visit<'ast> for V<'_> {
        fn visit_type_path(&mut self, p: &'ast syn::TypePath) {
            self.0.push(last_name(&p.path));
            visit::visit_type_path(self, p);
        }
        fn visit_type_ptr(&mut self, p: &'ast syn::TypePtr) {
            if p.mutability.is_some() {
                self.0.push("*mut".into());
            }
            visit::visit_type_ptr(self, p);
        }
    }
    V(out).visit_type(t);
}

/// The `use .. as <new>` leaves of a use tree: (new, the original's full path).
fn use_rename_paths(
    prefix: &mut Vec<String>,
    t: &syn::UseTree,
    out: &mut Vec<(String, Vec<String>)>,
) {
    match t {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            use_rename_paths(prefix, &p.tree, out);
            prefix.pop();
        }
        syn::UseTree::Rename(r) => {
            let mut v = prefix.clone();
            v.push(r.ident.to_string());
            out.push((r.rename.to_string(), v));
        }
        syn::UseTree::Group(g) => {
            for x in &g.items {
                use_rename_paths(prefix, x, out);
            }
        }
        syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
    }
}

/// The `use .. as <new>` leaves of a use tree: (new, the original's last name, text).
fn use_renames(
    prefix: &mut Vec<String>,
    t: &syn::UseTree,
    out: &mut Vec<(String, String, String)>,
) {
    match t {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            use_renames(prefix, &p.tree, out);
            prefix.pop();
        }
        syn::UseTree::Rename(r) => {
            let mut v = prefix.clone();
            v.push(r.ident.to_string());
            out.push((
                r.rename.to_string(),
                r.ident.to_string(),
                format!("{} as {}", v.join("::"), r.rename),
            ));
        }
        syn::UseTree::Group(g) => {
            for x in &g.items {
                use_renames(prefix, x, out);
            }
        }
        syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
    }
}

// =====================================================================================
// ADDR and UNWIND: a crate-wide census of the tokens (DELTA12 S1, N-6)
// =====================================================================================

/// ADDR's and UNWIND's census of the crate's tokens (every file of `src`, tests under `src`,
/// macro arguments and `use` items included ; comments are not tokens).
#[derive(Default)]
struct AddrCensus {
    /// (file, fn, form) -> the OPERAND of each site, in source order
    sites: BTreeMap<(String, String, String), Vec<String>>,
    /// (file, fn, name) -> count
    unwind: BTreeMap<(String, String, String), usize>,
    /// (file, fn) -> the identifiers its tokens hold (its signature and body)
    idents: BTreeMap<(String, String), BTreeSet<String>>,
    /// identifier -> its tokens crate-wide, the name after `fn` excluded (CALLS)
    calls: BTreeMap<String, usize>,
    /// files that do not tokenize
    bad: Vec<(String, String)>,
}

fn is_punct(t: Option<&TokenTree>, c: char) -> bool {
    matches!(t, Some(TokenTree::Punct(p)) if p.as_char() == c)
}

/// `::` ending at `i - 1` (`::` is `:` Joint then `:`).
fn colons_before(toks: &[TokenTree], i: usize) -> bool {
    i >= 2 && is_punct(toks.get(i - 2), ':') && is_punct(toks.get(i - 1), ':')
}

fn colons_after(toks: &[TokenTree], i: usize) -> bool {
    is_punct(toks.get(i + 1), ':') && is_punct(toks.get(i + 2), ':')
}

fn tok_text(toks: &[TokenTree]) -> String {
    let s = toks.iter().cloned().collect::<TokenStream>().to_string();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_paren(t: Option<&TokenTree>) -> bool {
    matches!(t, Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
}

/// The expression ending at `end` (exclusive), read backwards: a postfix chain (names,
/// literals, groups, `.`, `::`, `?`, a macro's `!`), then the unary `*` `&` `&mut` `!` `-`
/// in front of it that no operand precedes ; `-` when empty.
fn back_expr(toks: &[TokenTree], end: usize) -> String {
    let stop = |t: &TokenTree| matches!(t, TokenTree::Ident(i) if OPERAND_STOP.contains(&i.to_string().as_str()));
    let operand_end = |t: Option<&TokenTree>| match t {
        Some(TokenTree::Ident(_)) => !t.is_some_and(stop),
        Some(TokenTree::Literal(_)) | Some(TokenTree::Group(_)) => true,
        Some(TokenTree::Punct(p)) => p.as_char() == '?',
        None => false,
    };
    let mut k = end;
    while k > 0 {
        let t = &toks[k - 1];
        let ok = match t {
            TokenTree::Group(_) | TokenTree::Literal(_) => true,
            TokenTree::Ident(_) => !stop(t),
            TokenTree::Punct(p) => {
                matches!(p.as_char(), '.' | ':' | '?')
                    || (p.as_char() == '!' && matches!(toks.get(k), Some(TokenTree::Group(_))))
            }
        };
        if !ok {
            break;
        }
        k -= 1;
    }
    while k > 0 {
        let t = &toks[k - 1];
        let is_mut = matches!(t, TokenTree::Ident(i) if i == "mut")
            && k >= 2
            && is_punct(toks.get(k - 2), '&');
        let unary =
            matches!(t, TokenTree::Punct(p) if matches!(p.as_char(), '*' | '&' | '!' | '-'));
        if !(unary || is_mut) {
            break;
        }
        let b = if is_mut { k - 2 } else { k - 1 };
        if b > 0 && operand_end(toks.get(b - 1)) {
            break; // a binary operator
        }
        k = b;
    }
    if k == end {
        "-".into()
    } else {
        tok_text(&toks[k..end])
    }
}

/// The tokens from `from` to the next `,` / `;` (exclusive) ; `-` when empty.
fn fwd_until_sep(toks: &[TokenTree], from: usize) -> String {
    let mut j = from;
    while j < toks.len() && !is_punct(toks.get(j), ',') && !is_punct(toks.get(j), ';') {
        j += 1;
    }
    if j == from {
        "-".into()
    } else {
        tok_text(&toks[from..j])
    }
}

/// A string literal holding a `p` format spec (`{:p}`, `{x:p}`, `{:#p}`, `{0:>10p}`) by
/// std::fmt's grammar: `{` [argument] `:` [[fill]align][sign][`#`][`0`][width][`.`precision]
/// `p` `}` (a brace pair that is no format spec, `{word:Some, X}` in a doc line, is not a site).
fn has_p_spec(lit: &str) -> bool {
    if !(lit.starts_with('"') || lit.starts_with('r')) {
        return false;
    }
    let cs: Vec<char> = lit.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '{' {
            if cs.get(i + 1) == Some(&'{') {
                i += 2;
                continue;
            }
            let Some(close) = cs[i..].iter().position(|c| *c == '}') else {
                break;
            };
            let inner: String = cs[i + 1..i + close].iter().collect();
            if let Some((arg, spec)) = inner.split_once(':') {
                if is_fmt_arg(arg) && is_p_format_spec(spec) {
                    return true;
                }
            }
            i += close + 1;
        } else {
            i += 1;
        }
    }
    false
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f == '_' || f.is_alphabetic())
        && c.all(|x| x == '_' || x.is_alphanumeric())
}

/// A format argument: empty, a position, or an identifier.
fn is_fmt_arg(a: &str) -> bool {
    a.is_empty() || a.chars().all(|c| c.is_ascii_digit()) || is_word(a)
}

/// A count of width / precision: digits, or an argument with `$`.
fn is_fmt_count(c: &str) -> bool {
    !c.is_empty()
        && (c.chars().all(|x| x.is_ascii_digit())
            || c.strip_suffix('$')
                .is_some_and(|a| !a.is_empty() && is_fmt_arg(a)))
}

/// std::fmt's format_spec with type `p`.
fn is_p_format_spec(spec: &str) -> bool {
    let Some(mut r) = spec.strip_suffix('p') else {
        return false;
    };
    let cs: Vec<char> = r.chars().collect();
    let align = |c: char| matches!(c, '<' | '^' | '>');
    if cs.len() >= 2 && align(cs[1]) {
        r = &r[cs[0].len_utf8() + 1..];
    } else if cs.first().is_some_and(|c| align(*c)) {
        r = &r[1..];
    }
    r = r.strip_prefix(['+', '-']).unwrap_or(r);
    r = r.strip_prefix('#').unwrap_or(r);
    r = r.strip_prefix('0').unwrap_or(r);
    let (width, prec) = match r.split_once('.') {
        Some((w, p)) => (w, Some(p)),
        None => (r, None),
    };
    (width.is_empty() || is_fmt_count(width)) && prec.is_none_or(|p| p == "*" || is_fmt_count(p))
}

struct AddrWalk<'a> {
    file: &'a str,
    names: &'a BTreeMap<String, BTreeSet<String>>,
    out: &'a mut AddrCensus,
}

impl AddrWalk<'_> {
    fn has(&self, form: &str, n: &str) -> bool {
        self.names.get(form).is_some_and(|s| s.contains(n))
    }

    fn site(&mut self, f: &str, form: String, operand: String) {
        self.out
            .sites
            .entry((self.file.to_string(), f.to_string(), form))
            .or_default()
            .push(operand);
    }

    /// One token list (a file, a group): a `fn <name>` gives its signature and body that
    /// fn's context (nested fns joined by `::`, a method by its own name) ; a `use` item is
    /// read whole (every `ptr` in it is an n1 site, the use text its operand).
    fn walk(&mut self, toks: &[TokenTree], ctx: &[String], in_use: Option<&str>) {
        let f = if ctx.is_empty() {
            "-".to_string()
        } else {
            ctx.join("::")
        };
        let mut i = 0;
        while i < toks.len() {
            match &toks[i] {
                TokenTree::Group(g) => {
                    let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                    self.walk(&inner, ctx, in_use);
                }
                TokenTree::Ident(id) => {
                    let s = id.to_string();
                    if s == "fn" {
                        if let Some(TokenTree::Ident(n)) = toks.get(i + 1) {
                            let mut j = i + 2;
                            while j < toks.len() {
                                match &toks[j] {
                                    TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                                        break
                                    }
                                    TokenTree::Punct(p) if p.as_char() == ';' => break,
                                    _ => j += 1,
                                }
                            }
                            let end = j.min(toks.len() - 1);
                            let mut c = ctx.to_vec();
                            c.push(n.to_string());
                            self.walk(&toks[i + 2..=end], &c, in_use);
                            i = end + 1;
                            continue;
                        }
                    }
                    if s == "use" && in_use.is_none() {
                        let mut j = i + 1;
                        while j < toks.len() && !is_punct(toks.get(j), ';') {
                            j += 1;
                        }
                        let text = tok_text(&toks[i..j.min(toks.len())]);
                        self.ident(&f, &s);
                        self.walk(&toks[i + 1..j.min(toks.len())], ctx, Some(&text));
                        i = j + 1;
                        continue;
                    }
                    self.ident(&f, &s);
                    self.ident_forms(toks, i, &f, &s, in_use);
                }
                TokenTree::Punct(p) => {
                    let c = p.as_char();
                    let kw = |k: usize| match toks.get(k) {
                        Some(TokenTree::Ident(x)) => x.to_string(),
                        _ => String::new(),
                    };
                    // s1: a raw-pointer type, `*const` / `*mut`
                    if c == '*' && matches!(kw(i + 1).as_str(), "const" | "mut") {
                        let form = format!("s1:*{}", kw(i + 1));
                        let op = if kw(i.wrapping_sub(1)) == "as" && i >= 1 {
                            back_expr(toks, i - 1)
                        } else {
                            // a `let x: *const T = e` : the expression after `=`
                            let mut j = i + 2;
                            let mut op = "-".to_string();
                            while j < toks.len() {
                                match &toks[j] {
                                    TokenTree::Punct(q)
                                        if q.as_char() == ';' || q.as_char() == ',' =>
                                    {
                                        break
                                    }
                                    TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                                        break
                                    }
                                    TokenTree::Punct(q)
                                        if q.as_char() == '='
                                            && q.spacing() == proc_macro2::Spacing::Alone
                                            && !matches!(toks.get(j - 1), Some(TokenTree::Punct(r)) if r.spacing() == proc_macro2::Spacing::Joint) =>
                                    {
                                        op = fwd_until_sep(toks, j + 1);
                                        break;
                                    }
                                    _ => j += 1,
                                }
                            }
                            op
                        };
                        self.site(&f, form, op);
                    }
                    // s2: the raw-borrow operator `&raw const` / `&raw mut`
                    if c == '&'
                        && kw(i + 1) == "raw"
                        && matches!(kw(i + 2).as_str(), "const" | "mut")
                    {
                        let op = fwd_until_sep(toks, i + 3);
                        self.site(&f, format!("s2:&raw {}", kw(i + 2)), op);
                    }
                }
                TokenTree::Literal(l) => {
                    if has_p_spec(&l.to_string()) {
                        let mut j = i + 1;
                        if is_punct(toks.get(j), ',') {
                            j += 1;
                        }
                        let op = if j < toks.len() {
                            tok_text(&toks[j..])
                        } else {
                            "-".into()
                        };
                        self.site(&f, "s3:{:p}".into(), op);
                    }
                }
            }
            i += 1;
        }
    }

    fn ident(&mut self, f: &str, s: &str) {
        *self.out.calls.entry(s.to_string()).or_default() += 1;
        self.out
            .idents
            .entry((self.file.to_string(), f.to_string()))
            .or_default()
            .insert(s.to_string());
        if UNWIND_NAMES.contains(&s) {
            *self
                .out
                .unwind
                .entry((self.file.to_string(), f.to_string(), s.to_string()))
                .or_default() += 1;
        }
    }

    fn ident_forms(
        &mut self,
        toks: &[TokenTree],
        i: usize,
        f: &str,
        s: &str,
        in_use: Option<&str>,
    ) {
        let group_text = |k: usize| -> Option<String> {
            match toks.get(k) {
                Some(TokenTree::Group(g)) => {
                    Some(tok_text(std::slice::from_ref(&TokenTree::Group(g.clone()))))
                }
                _ => None,
            }
        };
        // n1: the path segment `ptr` (`ptr ::`, `:: ptr`, or anywhere in a `use`)
        if self.has("n1", s)
            && (in_use.is_some() || colons_before(toks, i) || colons_after(toks, i))
        {
            let op = if let Some(u) = in_use {
                u.to_string()
            } else if colons_after(toks, i)
                && matches!(toks.get(i + 3), Some(TokenTree::Ident(_)))
                && is_paren(toks.get(i + 4))
            {
                group_text(i + 4).unwrap_or_else(|| "-".into())
            } else {
                "-".into()
            };
            self.site(f, format!("n1:{s}"), op);
        }
        // n2: a name anywhere (a macro's or a call's arguments are its operand)
        if self.has("n2", s) {
            let op = if is_punct(toks.get(i + 1), '!') {
                group_text(i + 2)
            } else if is_paren(toks.get(i + 1)) {
                group_text(i + 1)
            } else {
                None
            }
            .or_else(|| in_use.map(str::to_string))
            .unwrap_or_else(|| "-".into());
            self.site(f, format!("n2:{s}"), op);
        }
        // n3: a method `. name (` (the receiver is its operand) or a path `:: name`
        if self.has("n3", s) {
            let dot = i >= 1
                && is_punct(toks.get(i - 1), '.')
                && !(i >= 2 && is_punct(toks.get(i - 2), '.'));
            if dot && is_paren(toks.get(i + 1)) {
                let op = back_expr(toks, i - 1);
                self.site(f, format!("n3:{s}"), op);
            } else if colons_before(toks, i) || in_use.is_some() {
                let op = if is_paren(toks.get(i + 1)) {
                    group_text(i + 1).unwrap_or_else(|| "-".into())
                } else {
                    in_use.map(str::to_string).unwrap_or_else(|| "-".into())
                };
                self.site(f, format!("n3:{s}"), op);
            }
        }
    }
}

fn addr_census(vfs: &Vfs, names: &BTreeMap<String, BTreeSet<String>>) -> AddrCensus {
    let mut out = AddrCensus::default();
    for (rel, text) in vfs {
        let ts: TokenStream = match text.parse() {
            Ok(t) => t,
            Err(e) => {
                out.bad.push((rel.clone(), e.to_string()));
                continue;
            }
        };
        let toks: Vec<TokenTree> = ts.into_iter().collect();
        let mut w = AddrWalk {
            file: rel,
            names,
            out: &mut out,
        };
        w.walk(&toks, &[], None);
    }
    out
}

/// ADDR, its CALLS and UNWIND against the pins (crate-wide ; SURF-PLAN does not run it).
fn addr_check(eng: &mut Eng<'_>, c: &AddrCensus) {
    for (rel, e) in &c.bad {
        eng.fail(rel, rel, "FILE", "ADDR", format!("does not tokenize: {e}"));
    }
    let want: BTreeMap<(String, String, String), (String, String)> = eng
        .cfg
        .addr
        .iter()
        .map(|r| {
            (
                (r[0].clone(), r[1].clone(), r[2].clone()),
                (r[3].clone(), r[4].clone()),
            )
        })
        .collect();
    let mut listed_fns: BTreeSet<String> = BTreeSet::new();
    for ((file, f, form), ops) in &c.sites {
        let (count, op) = (ops.len().to_string(), ops.join(" | "));
        eng.s
            .addr_rows
            .push(format!("{file}\t{f}\t{form}\t{count}\t{op}"));
        match want.get(&(file.clone(), f.clone(), form.clone())) {
            None => eng.fail(
                file,
                f,
                form,
                "ADDR",
                format!("an address site off ADDR: {count} · {op}"),
            ),
            Some((wc, wo)) => {
                if f != "-" {
                    listed_fns.insert(f.rsplit("::").next().unwrap_or(f).to_string());
                }
                if *wc != count || *wo != op {
                    eng.fail(
                        file,
                        f,
                        form,
                        "ADDR",
                        format!("pinned {wc} · {wo} now {count} · {op}"),
                    );
                }
            }
        }
    }
    for (file, f, form) in want.keys() {
        if !c
            .sites
            .contains_key(&(file.clone(), f.clone(), form.clone()))
        {
            eng.fail(file, f, form, "STALE", "an ADDR row matches no site");
        }
    }
    // CALLS (HX's FAN-IN form, one level): each pinned address fn's call tokens
    let calls_want: BTreeMap<String, String> = eng
        .cfg
        .addr_calls
        .iter()
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    for f in &listed_fns {
        let got = c.calls.get(f).copied().unwrap_or(0).to_string();
        eng.s.addr_calls.push(format!("{f}\t{got}"));
        match calls_want.get(f) {
            None => eng.fail(
                "-",
                f,
                "CALLS",
                "ADDR",
                format!("an address fn with no CALLS row: {got} call tokens"),
            ),
            Some(w) if *w != got => {
                eng.fail("-", f, "CALLS", "ADDR", format!("pinned {w} now {got}"))
            }
            _ => {}
        }
    }
    for f in calls_want.keys() {
        if !listed_fns.contains(f) {
            eng.fail(
                "-",
                f,
                "CALLS",
                "STALE",
                "a CALLS row names no pinned address fn",
            );
        }
    }
    // UNWIND: pinned empty on the crate
    let uwant: BTreeMap<(String, String, String), String> = eng
        .cfg
        .unwind
        .iter()
        .map(|r| ((r[0].clone(), r[1].clone(), r[2].clone()), r[3].clone()))
        .collect();
    for ((file, f, n), k) in &c.unwind {
        eng.s.unwind_rows.push(format!("{file}\t{f}\t{n}\t{k}"));
        match uwant.get(&(file.clone(), f.clone(), n.clone())) {
            None => eng.fail(file, f, n, "UNWIND", format!("{k} token(s) off UNWIND")),
            Some(w) if *w != k.to_string() => {
                eng.fail(file, f, n, "UNWIND", format!("pinned {w} now {k}"))
            }
            _ => {}
        }
    }
    for (file, f, n) in uwant.keys() {
        if !c.unwind.contains_key(&(file.clone(), f.clone(), n.clone())) {
            eng.fail(file, f, n, "STALE", "an UNWIND row matches no token");
        }
    }
}

/// ADDR-FN (DELTA12 S1): no fn holding an address site (listed or not: the stricter
/// reading) holds a token of a surface item one of whose OUT positions is REF.
fn addr_fn_check(eng: &mut Eng<'_>, c: &AddrCensus) {
    let refs: BTreeSet<String> = eng
        .s
        .rows
        .iter()
        .filter(|r| r.refs == "REF")
        .map(|r| {
            let n = r.item.rsplit("::").next().unwrap_or(&r.item);
            n.rsplit('.').next().unwrap_or(n).to_string()
        })
        .collect();
    let fns: BTreeSet<(String, String)> = c
        .sites
        .keys()
        .filter(|(_, f, _)| f != "-")
        .map(|(file, f, _)| (file.clone(), f.clone()))
        .collect();
    for (file, f) in fns {
        let Some(ids) = c.idents.get(&(file.clone(), f.clone())) else {
            continue;
        };
        for r in &refs {
            if ids.contains(r) {
                eng.fail(
                    &file,
                    &f,
                    &format!("CALL:{r}"),
                    "ADDR-FN",
                    format!("an address fn names {r}, a surface item with a REF position"),
                );
            }
        }
    }
}

// =====================================================================================
// Tier E: OUT-F / OUT-C / OUT-M / BODY
// =====================================================================================

struct EVis<'a> {
    defs: &'a Defs,
    cfg_vocab: &'a BTreeMap<String, String>,
    /// WINDOW's rows: a field whose type names one of their types is read through the window
    cfg_window: &'a [Vec<String>],
    f: BTreeMap<String, usize>,
    c: BTreeMap<String, usize>,
    m: BTreeMap<String, usize>,
    alias: Vec<String>,
}

impl EVis<'_> {
    fn lex_field(&self, n: &str) -> bool {
        let Some(fields) = &self.defs.elab_fields else {
            return false;
        };
        let Some(t) = fields.get(n) else {
            return false;
        };
        let mut names = BTreeSet::new();
        type_names(t, &mut names);
        // a store of the window (WINDOW) is no lexscope field: its map is read through Deref
        if names
            .iter()
            .any(|x| self.cfg_window.iter().any(|w| &w[0] == x))
        {
            return false;
        }
        names.iter().all(|x| {
            x == MODULE
                || x == "crate"
                || WRAPPERS.contains(&x.as_str())
                || x == "bool"
                || (self.cfg_vocab.contains_key(x) && self.defs.defined_type(x))
        })
    }
}

impl<'ast> Visit<'ast> for EVis<'_> {
    fn visit_expr_field(&mut self, e: &'ast syn::ExprField) {
        if is_self_expr(&e.base) {
            if let syn::Member::Named(n) = &e.member {
                let n = n.to_string();
                if !self.lex_field(&n) {
                    *self.f.entry(n).or_default() += 1;
                }
            }
        }
        visit::visit_expr_field(self, e);
    }

    fn visit_expr_method_call(&mut self, e: &'ast syn::ExprMethodCall) {
        let m = e.method.to_string();
        let ours = is_self_expr(&e.receiver)
            && self.defs.type_fns.get(ELAB).is_some_and(|s| s.contains(&m));
        if !ours {
            *self.c.entry(format!(".{m}")).or_default() += 1;
        }
        visit::visit_expr_method_call(self, e);
    }

    fn visit_expr_call(&mut self, e: &'ast syn::ExprCall) {
        match &*e.func {
            Expr::Path(p) if p.qself.is_none() => {
                let segs = seg_names(&p.path);
                let last = segs.last().cloned().unwrap_or_default();
                let n = segs.len();
                let ty = if segs[0] == "Self" {
                    ELAB
                } else {
                    segs[0].as_str()
                };
                let ours = (n == 1 && self.defs.free_fns.contains(&last))
                    || (n == 2
                        && self
                            .defs
                            .type_fns
                            .get(ty)
                            .is_some_and(|s| s.contains(&last)))
                    || (n > 1
                        && internal_prefix(&segs[..n - 1], self.defs)
                        && self.defs.free_fns.contains(&last));
                if !ours {
                    *self.c.entry(segs.join("::")).or_default() += 1;
                }
            }
            _ => *self.c.entry("(expr)".into()).or_default() += 1,
        }
        visit::visit_expr_call(self, e);
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = last_name(&m.path);
        *self.m.entry(name.clone()).or_default() += 1;
        match Punctuated::<Expr, Token![,]>::parse_terminated.parse2(m.tokens.clone()) {
            Ok(list) => {
                for e in &list {
                    self.visit_expr(e);
                }
            }
            Err(_) => {
                if tokens_name(&m.tokens, "self") {
                    self.alias
                        .push(format!("macro {name}! does not parse and names self"));
                }
            }
        }
    }

    fn visit_local(&mut self, l: &'ast syn::Local) {
        if let Some(init) = &l.init {
            if is_self_alias(&init.expr) {
                self.alias.push("a local bound to self".into());
            }
        }
        visit::visit_local(self, l);
    }

    // ALIAS-SELF+: every pattern of the body, not only a `let`'s (a `match` arm, `if let`,
    // `while let`, a closure's or a nested fn's parameter).
    fn visit_pat_struct(&mut self, s: &'ast syn::PatStruct) {
        let n = last_name(&s.path);
        if n == "Self" || n == ELAB {
            self.alias.push("a `Self { .. }` pattern".into());
        }
        visit::visit_pat_struct(self, s);
    }

    fn visit_pat_tuple_struct(&mut self, s: &'ast syn::PatTupleStruct) {
        let n = last_name(&s.path);
        if n == "Self" || n == ELAB {
            self.alias.push("a `Self(..)` pattern".into());
        }
        visit::visit_pat_tuple_struct(self, s);
    }

    fn visit_expr_match(&mut self, e: &'ast syn::ExprMatch) {
        if is_self_alias(&e.expr) {
            self.alias.push("a match on self".into());
        }
        visit::visit_expr_match(self, e);
    }

    fn visit_expr_let(&mut self, e: &'ast syn::ExprLet) {
        if is_self_alias(&e.expr) {
            self.alias.push("a `let` pattern on self".into());
        }
        visit::visit_expr_let(self, e);
    }
}

/// BODY-ALL's token side: (simple name, body token text) of every `fn <name> .. { body }` in a
/// token stream, in pre-order (a fn before the fns nested in it), skipping a macro call's
/// group (`m!(..)`) and a `macro_rules! m { .. }` definition, which `syn` does not parse as
/// items either. `bodies_check` pairs this list with `FnNames`' and fails BODY-MAP when the
/// two simple-name sequences differ.
fn token_fn_bodies(ts: TokenStream, out: &mut Vec<(String, String)>) {
    let toks: Vec<TokenTree> = ts.into_iter().collect();
    let bang = |t: Option<&TokenTree>| matches!(t, Some(TokenTree::Punct(p)) if p.as_char() == '!');
    let mut i = 0;
    while i < toks.len() {
        if let TokenTree::Ident(id) = &toks[i] {
            if bang(toks.get(i + 1)) {
                if matches!(toks.get(i + 2), Some(TokenTree::Group(_))) {
                    i += 3;
                    continue;
                }
                if id == "macro_rules"
                    && matches!(toks.get(i + 2), Some(TokenTree::Ident(_)))
                    && matches!(toks.get(i + 3), Some(TokenTree::Group(_)))
                {
                    i += 4;
                    continue;
                }
            }
        }
        match &toks[i] {
            TokenTree::Group(g) => token_fn_bodies(g.stream(), out),
            TokenTree::Ident(id) if id == "fn" => {
                if let Some(TokenTree::Ident(n)) = toks.get(i + 1) {
                    for t in &toks[i + 2..] {
                        match t {
                            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                                out.push((n.to_string(), g.stream().to_string()));
                                break;
                            }
                            TokenTree::Punct(p) if p.as_char() == ';' => break,
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// BODY-ALL's `syn` side: (simple name, item name) of every fn with a body, in pre-order. The
/// item name is the engine's: `f` (free, or nested in a body), `Ty::f` (inherent), `Tr for
/// Ty::f` (trait impl), `Tr::f` (a trait's default body).
struct FnNames {
    ctx: Option<String>,
    out: Vec<(String, String)>,
}

impl FnNames {
    fn push(&mut self, ident: &syn::Ident) {
        let s = ident.to_string();
        let item = match &self.ctx {
            Some(c) => format!("{c}::{s}"),
            None => s.clone(),
        };
        self.out.push((s, item));
    }
}

impl<'ast> Visit<'ast> for FnNames {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.push(&f.sig.ident);
        let saved = self.ctx.take();
        visit::visit_item_fn(self, f);
        self.ctx = saved;
    }
    fn visit_item_impl(&mut self, im: &'ast syn::ItemImpl) {
        let ty = impl_self_name(&im.self_ty);
        let name = match &im.trait_ {
            Some((_, p, _)) => format!("{} for {ty}", last_name(p)),
            None => ty,
        };
        let saved = self.ctx.replace(name);
        visit::visit_item_impl(self, im);
        self.ctx = saved;
    }
    fn visit_item_trait(&mut self, t: &'ast syn::ItemTrait) {
        let saved = self.ctx.replace(t.ident.to_string());
        visit::visit_item_trait(self, t);
        self.ctx = saved;
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.push(&f.sig.ident);
        let saved = self.ctx.take();
        visit::visit_impl_item_fn(self, f);
        self.ctx = saved;
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if f.default.is_some() {
            self.push(&f.sig.ident);
        }
        let saved = self.ctx.take();
        visit::visit_trait_item_fn(self, f);
        self.ctx = saved;
    }
}

/// The names of the macros a file calls (any position ; `macro_rules` itself excluded).
struct MacroCalls(BTreeSet<String>);
impl<'ast> Visit<'ast> for MacroCalls {
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let n = last_name(&m.path);
        if n != "macro_rules" {
            self.0.insert(n);
        }
    }
}

/// SHA-256 (FIPS 180-4) of `data`, lower-case hex: BODY's hash (no crypto crate in the
/// workspace's dev-dependencies ; the fixture test checks two known vectors first).
fn sha256(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (wi, c) in w.iter_mut().zip(chunk.chunks(4)) {
            *wi = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for (k, wi) in K.iter().zip(w.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(*k)
                .wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut hex = String::with_capacity(64);
    for x in h {
        for b in x.to_be_bytes() {
            hex.push(char::from_digit(u32::from(b >> 4), 16).expect("hex"));
            hex.push(char::from_digit(u32::from(b & 15), 16).expect("hex"));
        }
    }
    hex
}

// =====================================================================================
// The engine
// =====================================================================================

#[derive(Default)]
struct Surface {
    present: bool,
    files: Vec<String>,
    rows: Vec<Row>,
    fails: BTreeSet<Fail>,
    /// item, kind, name, count
    e_rows: Vec<(String, String, String, String)>,
    out_types: BTreeSet<String>,
    /// BODY-ALL: file, fn, sha256
    bodies: Vec<(String, String, String)>,
    /// MACRO-BODY: name, sha256
    macro_hashes: Vec<(String, String)>,
    /// ADDR's computed rows (file, fn, form, count, operand), its CALLS (fn, count) and
    /// UNWIND's (file, fn, name, count): crate-wide, module or not
    addr_rows: Vec<String>,
    addr_calls: Vec<String>,
    unwind_rows: Vec<String>,
}

/// A fn of the file set, for the shim check.
struct FnInfo<'a> {
    wider: bool,
    sig: &'a syn::Signature,
    block: &'a syn::Block,
}

struct Eng<'a> {
    cfg: &'a Cfg,
    /// INTERIOR's index of the crate's type names
    tys: &'a TyIdx,
    defs: Defs,
    s: Surface,
    leave_used: BTreeSet<usize>,
    enter_used: BTreeSet<usize>,
    card_used: BTreeSet<usize>,
    cb_used: BTreeSet<usize>,
    interior_used: BTreeSet<usize>,
    enums_seen: BTreeSet<String>,
    window_used: BTreeSet<usize>,
    bridge_used: BTreeSet<usize>,
    plan: bool,
}

impl Eng<'_> {
    fn fresh<'c>(cfg: &'c Cfg, tys: &'c TyIdx, plan: bool) -> Eng<'c> {
        Eng {
            cfg,
            tys,
            defs: Defs::default(),
            s: Surface::default(),
            leave_used: BTreeSet::new(),
            enter_used: BTreeSet::new(),
            card_used: BTreeSet::new(),
            cb_used: BTreeSet::new(),
            interior_used: BTreeSet::new(),
            enums_seen: BTreeSet::new(),
            window_used: BTreeSet::new(),
            bridge_used: BTreeSet::new(),
            plan,
        }
    }

    /// CARD: the cardinality constructors `t` holds at any depth (an alias of the file set
    /// followed), or None ; for an item of one OUT position (a field, a variant payload, a
    /// const, an alias): its bools are the item's.
    fn card_of(&self, t: &Type) -> Option<String> {
        let (found, bools) = self.card_parts(t);
        card_text(found, bools, bools, t)
    }

    /// CARD's parts of one position: the named constructors and the number of `bool`s.
    fn card_parts(&self, t: &Type) -> (BTreeSet<String>, usize) {
        let mut found = BTreeSet::new();
        let mut bools = 0usize;
        self.card_walk(t, &mut found, &mut bools, 0);
        (found, bools)
    }

    fn card_walk(&self, t: &Type, found: &mut BTreeSet<String>, bools: &mut usize, depth: usize) {
        if depth > 16 {
            found.insert("alias recursion".into());
            return;
        }
        match t {
            Type::Reference(x) => self.card_walk(&x.elem, found, bools, depth),
            Type::Paren(x) => self.card_walk(&x.elem, found, bools, depth),
            Type::Group(x) => self.card_walk(&x.elem, found, bools, depth),
            Type::Ptr(x) => self.card_walk(&x.elem, found, bools, depth),
            Type::Tuple(x) => {
                for e in &x.elems {
                    self.card_walk(e, found, bools, depth);
                }
            }
            Type::Slice(x) => {
                found.insert("slice".into());
                self.card_walk(&x.elem, found, bools, depth);
            }
            Type::Array(x) => {
                found.insert("array".into());
                self.card_walk(&x.elem, found, bools, depth);
            }
            Type::Path(p) => {
                if let Some(q) = &p.qself {
                    self.card_walk(&q.ty, found, bools, depth + 1);
                }
                let last = last_name(&p.path);
                let single = p.path.segments.len() == 1;
                if single && last == "bool" {
                    *bools += 1;
                }
                if CARD_CTORS.contains(&last.as_str()) {
                    found.insert(last.clone());
                }
                if single {
                    if let Some((ty, _)) = self.defs.aliases.get(&last) {
                        self.card_walk(ty, found, bools, depth + 1);
                    }
                }
                for s in &p.path.segments {
                    if let syn::PathArguments::AngleBracketed(a) = &s.arguments {
                        for g in &a.args {
                            match g {
                                syn::GenericArgument::Type(x) => {
                                    self.card_walk(x, found, bools, depth + 1)
                                }
                                syn::GenericArgument::AssocType(a) => {
                                    self.card_walk(&a.ty, found, bools, depth + 1)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn fail(&mut self, file: &str, item: &str, pos: &str, rule: &str, detail: impl Into<String>) {
        self.s.fails.insert(Fail {
            file: file.into(),
            item: item.into(),
            pos: pos.into(),
            rule: rule.into(),
            detail: detail.into(),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn position(
        &mut self,
        file: &str,
        tier: &str,
        kind: &str,
        item: &str,
        pos: &str,
        ty: String,
        mut reasons: Reasons,
        card: Option<String>,
    ) {
        // CARD: a cardinality constructor makes the position a row of CARD (or a LEAVE row)
        // even where the vocabulary passes it ; SURF-PLAN (plan) consults no list.
        let leave_i = if self.plan || (reasons.is_empty() && card.is_none()) {
            None
        } else {
            self.cfg
                .leave
                .iter()
                .position(|l| l[1] == item && l[2] == pos && l[3] == ty)
        };
        let card_i = if self.plan || card.is_none() {
            None
        } else {
            self.cfg
                .card
                .iter()
                .position(|c| c[0] == item && c[1] == pos && c[2] == ty)
        };
        let verdict = if reasons.is_empty() && card.is_none() {
            "PASS".to_string()
        } else if let Some(i) = leave_i {
            self.leave_used.insert(i);
            format!("LEAVE:{}", self.cfg.leave[i][0])
        } else {
            match (card_i, &card) {
                (Some(i), _) => {
                    self.card_used.insert(i);
                }
                (None, Some(c)) => reasons.push(("CARD".into(), c.clone())),
                (None, None) => {}
            }
            if reasons.is_empty() {
                let i = card_i.expect("a CARD row");
                format!("CARD:{}", self.cfg.card[i][3])
            } else {
                let rules: BTreeSet<&str> = reasons.iter().map(|(r, _)| r.as_str()).collect();
                for (rule, d) in &reasons {
                    self.fail(file, item, pos, rule, d.clone());
                }
                format!("FAIL({})", rules.into_iter().collect::<Vec<_>>().join(","))
            }
        };
        if ty != "-" {
            self.s.out_types.insert(ty.clone());
        }
        // REF (DELTA12 S1 ADDR-FN): an OUT type that holds a reference at any depth (`&` in
        // its normalised text: a callback's `&` argument included, the stricter reading)
        let refs = if ty.contains('&') { "REF" } else { "-" };
        self.s.rows.push(Row {
            file: file.into(),
            tier: tier.into(),
            kind: kind.into(),
            item: item.into(),
            pos: pos.into(),
            ty,
            verdict,
            refs: refs.into(),
        });
    }

    fn judge(&self, self_ty: Option<String>, gen: Gen, in_trait: Option<String>) -> J<'_> {
        J {
            defs: &self.defs,
            cfg: self.cfg,
            tys: self.tys,
            self_ty,
            gen,
            in_trait,
        }
    }

    fn out_reasons(
        &self,
        j_self: Option<String>,
        gen: &Gen,
        in_trait: Option<String>,
        t: &Type,
    ) -> Reasons {
        let j = self.judge(j_self, gen.clone(), in_trait);
        let mut r = Vec::new();
        j.out(t, &mut r, 0);
        r
    }

    /// The rows of one fn signature (visible) ; `block` is its body (WHOLE reads it).
    #[allow(clippy::too_many_arguments)]
    fn fn_rows(
        &mut self,
        file: &str,
        item: &str,
        sig: &syn::Signature,
        block: Option<&syn::Block>,
        self_ty: Option<String>,
        mut gen: Gen,
        in_trait: Option<String>,
        tier: &str,
    ) {
        gen_of(&sig.generics, &mut gen);
        for g in sig.generics.type_params() {
            let n = g.ident.to_string();
            if self.reserved(&n) {
                self.fail(
                    file,
                    item,
                    "GENERIC",
                    "VOCAB-SHADOW",
                    format!("type parameter {n}"),
                );
            }
        }
        // the item's OUT positions, judged after the walk so CARD counts bools per ITEM
        let mut outs: Vec<(String, Type)> = Vec::new();
        let ret_ty = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, t) => Some((**t).clone()),
        };
        for (i, a) in sig.inputs.iter().enumerate() {
            let FnArg::Typed(pt) = a else { continue };
            let pname = match &*pt.pat {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                _ => format!("_{i}"),
            };
            let mut names = BTreeSet::new();
            type_names(&pt.ty, &mut names);
            if names.contains(ELAB) {
                self.fail(file, item, &format!("IN:{pname}"), "ELAB-PARAM", nt(&pt.ty));
            }
            // INTERIOR: every parameter but the receiver (`&self` / `&mut self` is a
            // receiver, not a typed parameter: the one tier-E receiver is exempt by name)
            let skip: BTreeSet<String> = gen.keys().cloned().collect();
            self.interior_pos(
                file,
                tier,
                "FN",
                item,
                &format!("IN:{pname}"),
                &pt.ty,
                self_ty.as_deref(),
                &skip,
            );
            let (res, cbs) = {
                let j = self.judge(self_ty.clone(), gen.clone(), in_trait.clone());
                let mut cbs = Vec::new();
                j.callbacks(&pt.ty, &mut cbs, 0);
                (j.in_param(&pname, &pt.ty), cbs)
            };
            outs.extend(res.outs.iter().cloned());
            for (pos, t) in &res.cbs {
                if is_self_ty(t) && self_ty.as_deref() == Some(ELAB) {
                    continue;
                }
                outs.push((pos.clone(), t.clone()));
            }
            for (k, (kind, args)) in cbs.iter().enumerate() {
                let param = if cbs.len() == 1 {
                    pname.clone()
                } else {
                    format!("{pname}#{}", k + 1)
                };
                let args_txt = if args.is_empty() {
                    "-".to_string()
                } else {
                    args.iter().map(nt).collect::<Vec<_>>().join(", ")
                };
                self.cb_row(file, tier, item, &param, &nt(&pt.ty), kind, &args_txt);
            }
            for (rule, d) in &res.fails {
                self.fail(file, item, &format!("IN:{pname}"), rule, d.clone());
            }
            self.enter_row(file, tier, item, &pname, &pt.ty, &res, block);
        }
        // CARD per item (DELTA12 N-1): the bools of every OUT position summed
        let parts: Vec<(BTreeSet<String>, usize)> = ret_ty
            .iter()
            .chain(outs.iter().map(|(_, t)| t))
            .map(|t| self.card_parts(t))
            .collect();
        let total: usize = parts.iter().map(|(_, b)| *b).sum();
        let mut parts = parts.into_iter();
        match &ret_ty {
            None => {
                self.position(file, tier, "FN", item, "RET", "()".into(), Vec::new(), None);
            }
            Some(t) => {
                let (found, n) = parts.next().expect("RET's parts");
                let r = self.out_reasons(self_ty.clone(), &gen, in_trait.clone(), t);
                let c = card_text(found, n, total, t);
                self.position(file, tier, "FN", item, "RET", nt(t), r, c);
            }
        }
        for ((pos, t), (found, n)) in outs.iter().zip(parts) {
            let r = self.out_reasons(self_ty.clone(), &gen, in_trait.clone(), t);
            let c = card_text(found, n, total, t);
            self.position(file, tier, "FN", item, pos, nt(t), r, c);
        }
    }

    /// IN-TOTAL's row of one parameter (DELTA12 S2 r2, r3): a name no list and no index
    /// resolves FAILS IN-UNKNOWN ; a position holding text or a number, by name or by
    /// content, is a row of ENTER (item, parameter, role, holds) else FAIL ENTER ; a moved
    /// `holds` FAILS ENTER ; `index` on a text position FAILS ROLE ; `carried` is checked by
    /// WHOLE on the body. SURF-PLAN (plan) consults no list: the row reads ENTER-NEEDED.
    #[allow(clippy::too_many_arguments)]
    fn enter_row(
        &mut self,
        file: &str,
        tier: &str,
        item: &str,
        pname: &str,
        ty: &Type,
        res: &InRes,
        block: Option<&syn::Block>,
    ) {
        let pos = format!("IN:{pname}");
        if !res.unknown.is_empty() {
            let mut u = res.unknown.clone();
            u.sort();
            u.dedup();
            self.fail(file, item, &pos, "IN-UNKNOWN", u.join("; "));
        }
        let kind = if res.enter.iter().any(|(k, _)| k == "TEXT") {
            "TEXT"
        } else if !res.enter.is_empty() {
            "NUMBER"
        } else {
            return;
        };
        let mut ds: Vec<String> = Vec::new();
        for (k, d) in &res.enter {
            if k == kind && !ds.contains(d) {
                ds.push(d.clone());
            }
        }
        let holds = format!("{kind} {}", ds.join(", "));
        let verdict = if self.plan {
            format!("ENTER-NEEDED:{kind}")
        } else if let Some(k) = self
            .cfg
            .enter
            .iter()
            .position(|e| e[0] == item && e[1] == pname)
        {
            self.enter_used.insert(k);
            let row = self.cfg.enter[k].clone();
            let role = row[2].clone();
            if row[3] != holds {
                self.fail(
                    file,
                    item,
                    &pos,
                    "ENTER",
                    format!("pinned {} now {holds}", row[3]),
                );
            }
            if role == "index" && kind == "TEXT" {
                self.fail(file, item, &pos, "ROLE", "`index` on a text position");
            }
            if role == "carried" {
                match block {
                    None => self.fail(file, item, &pos, "WHOLE", "`carried` with no body to read"),
                    Some(b) => {
                        let mut w = WholeUse {
                            p: pname,
                            hits: Vec::new(),
                        };
                        w.visit_block(b);
                        if !w.hits.is_empty() {
                            self.fail(
                                file,
                                item,
                                &pos,
                                "WHOLE",
                                format!("`carried` read in the body: {}", w.hits.join(", ")),
                            );
                        }
                    }
                }
            }
            format!("ENTER:{role}")
        } else {
            self.fail(file, item, &pos, "ENTER", format!("off ENTER: {holds}"));
            "FAIL(ENTER)".to_string()
        };
        self.s.rows.push(Row {
            file: file.into(),
            tier: tier.into(),
            kind: "FN".into(),
            item: item.into(),
            pos,
            ty: nt(ty),
            verdict,
            refs: "-".into(),
        });
    }

    /// CB: one callback of a surface item's parameter, against its row of CB.
    #[allow(clippy::too_many_arguments)]
    fn cb_row(
        &mut self,
        file: &str,
        tier: &str,
        item: &str,
        param: &str,
        ty: &str,
        kind: &str,
        args: &str,
    ) {
        let pos = format!("CB:{param}");
        let found = if self.plan {
            None
        } else {
            self.cfg
                .cb
                .iter()
                .position(|r| r[0] == item && r[1] == param)
        };
        let verdict = match found {
            Some(i) => {
                self.cb_used.insert(i);
                let r = self.cfg.cb[i].clone();
                if r[2] == kind && r[3] == args {
                    format!("CB:{}", r[4])
                } else {
                    self.fail(
                        file,
                        item,
                        &pos,
                        "CB",
                        format!("pinned {}({}) now {kind}({args})", r[2], r[3]),
                    );
                    "FAIL(CB)".to_string()
                }
            }
            None => {
                self.fail(
                    file,
                    item,
                    &pos,
                    "CB",
                    if self.plan {
                        format!("a callback {kind}({args})")
                    } else {
                        format!("a callback {kind}({args}) off CB")
                    },
                );
                "FAIL(CB)".to_string()
            }
        };
        self.s.rows.push(Row {
            refs: "-".into(),
            file: file.into(),
            tier: tier.into(),
            kind: "FN".into(),
            item: item.into(),
            pos,
            ty: ty.into(),
            verdict,
        });
    }

    /// INTERIOR on one position (a parameter of a surface item, a field of a vocabulary type):
    /// a path of names to a mutable carrier fails it, unless a row of INTERIOR names the
    /// item, the position and the same path (SURF-PLAN consults no list).
    #[allow(clippy::too_many_arguments)]
    fn interior_pos(
        &mut self,
        file: &str,
        tier: &str,
        kind: &str,
        item: &str,
        pos: &str,
        t: &Type,
        self_ty: Option<&str>,
        skip: &BTreeSet<String>,
    ) {
        let Some(path) = self.tys.interior(t, self_ty, skip) else {
            return;
        };
        let found = if self.plan {
            None
        } else {
            self.cfg
                .interior
                .iter()
                .position(|r| r[0] == item && r[1] == pos)
        };
        let verdict = match found {
            Some(i) if self.cfg.interior[i][2] == path => {
                self.interior_used.insert(i);
                format!("INTERIOR:{}", self.cfg.interior[i][3])
            }
            Some(i) => {
                self.interior_used.insert(i);
                let w = self.cfg.interior[i][2].clone();
                self.fail(
                    file,
                    item,
                    pos,
                    "INTERIOR",
                    format!("pinned {w} now {path}"),
                );
                "FAIL(INTERIOR)".to_string()
            }
            None => {
                self.fail(file, item, pos, "INTERIOR", path);
                "FAIL(INTERIOR)".to_string()
            }
        };
        self.s.rows.push(Row {
            refs: "-".into(),
            file: file.into(),
            tier: tier.into(),
            kind: kind.into(),
            item: item.into(),
            pos: pos.into(),
            ty: nt(t),
            verdict,
        });
    }

    /// ELAB-PARAM for a fn the surface rows do not read (private, in a body, a private method
    /// of any impl, a trait impl's method, a private trait's method).
    fn elab_param_sig(&mut self, file: &str, item: &str, sig: &syn::Signature, what: &str) {
        let mut names = BTreeSet::new();
        for a in &sig.inputs {
            if let FnArg::Typed(pt) = a {
                type_names(&pt.ty, &mut names);
            }
        }
        if names.contains(ELAB) {
            self.fail(file, item, "SIG", "ELAB-PARAM", what);
        }
    }

    fn reserved(&self, n: &str) -> bool {
        self.cfg.vocab.contains_key(n)
            || WRAPPERS.contains(&n)
            || PASS_BOUNDS.contains(&n)
            || n == "bool"
            || is_text_name(n)
            || is_number_name(n)
    }

    fn check_attrs(&mut self, file: &str, item: &str, attrs: &[syn::Attribute]) {
        for a in attrs {
            let n = attr_name(a);
            if n == "path" {
                self.fail(file, item, "ATTR", "PATH-ATTR", "#[path]");
            } else if n == "derive" {
                let ds = a
                    .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                    .map(|p| p.iter().map(last_name).collect::<Vec<_>>())
                    .unwrap_or_else(|_| vec!["?".into()]);
                for d in ds {
                    if !ALLOWED_DERIVES.contains(&d.as_str()) {
                        self.fail(file, item, "ATTR", "DERIVE", format!("derive({d})"));
                    }
                }
            } else if n == "macro_export" {
                self.fail(file, item, "ATTR", "MACRO-EXPORT", "#[macro_export]");
            } else if !ALLOWED_ATTRS.contains(&n.as_str()) {
                self.fail(file, item, "ATTR", "ATTR", format!("#[{n}]"));
            }
        }
    }
}

fn impl_self_name(t: &Type) -> String {
    type_last(t).unwrap_or_else(|| nt(t))
}

/// BRIDGE's computed class of an impl's Self type: REF (a reference to one of the impl's type
/// parameters), TEXT (a path whose last name is a TEXT_TYPES name, or a reference to one), KEY (a
/// type the file set defines), else OTHER (no row may name it).
fn bridge_class(t: &Type, gen: &Gen, defs: &Defs) -> String {
    if let Type::Reference(r) = t {
        if let Type::Path(p) = &*r.elem {
            if p.qself.is_none()
                && p.path.segments.len() == 1
                && gen.contains_key(&last_name(&p.path))
            {
                return "REF".into();
            }
        }
    }
    let n = type_last(t).unwrap_or_default();
    if is_text_name(&n) {
        "TEXT".into()
    } else if defs.defined_type(&n) {
        "KEY".into()
    } else {
        "OTHER".into()
    }
}

fn item_ident(it: &Item) -> Option<String> {
    Some(match it {
        Item::Fn(f) => f.sig.ident.to_string(),
        Item::Struct(s) => s.ident.to_string(),
        Item::Enum(e) => e.ident.to_string(),
        Item::Union(u) => u.ident.to_string(),
        Item::Trait(t) => t.ident.to_string(),
        Item::Type(t) => t.ident.to_string(),
        Item::Const(c) => c.ident.to_string(),
        Item::Static(s) => s.ident.to_string(),
        Item::Mod(m) => m.ident.to_string(),
        _ => return None,
    })
}

/// Finds the module's root file and builds its file set (following `mod x;`).
fn file_set(vfs: &Vfs, eng: &mut Eng<'_>) -> Option<Vec<FileRec>> {
    let mod_rs = format!("{MODULE}/mod.rs");
    let flat_rs = format!("{MODULE}.rs");
    let root = match (vfs.contains_key(&mod_rs), vfs.contains_key(&flat_rs)) {
        (false, false) => return None,
        (true, true) => {
            eng.fail(
                &flat_rs,
                MODULE,
                "MOD",
                "MOD-AMBIGUOUS",
                "both lexscope/mod.rs and lexscope.rs",
            );
            mod_rs
        }
        (true, false) => mod_rs,
        (false, true) => flat_rs,
    };
    let mut out = Vec::new();
    let mut queue: Vec<(String, String, usize)> = vec![(root, MODULE.to_string(), 0)];
    let mut seen = BTreeSet::new();
    while let Some((rel, dir, depth)) = queue.pop() {
        if !seen.insert(rel.clone()) {
            continue;
        }
        let text = &vfs[&rel];
        let file = match syn::parse_file(text) {
            Ok(f) => f,
            Err(e) => {
                eng.fail(&rel, &rel, "FILE", "PARSE", e.to_string());
                continue;
            }
        };
        let mut ats = Vec::new();
        flatten(&rel, &file.items, depth, &[], false, &mut ats);
        for at in &ats {
            let Item::Mod(m) = at.item else { continue };
            eng.defs.mods.insert(m.ident.to_string());
            if m.content.is_some() {
                continue;
            }
            let name = m.ident.to_string();
            if at.in_body {
                eng.fail(&rel, &name, "MOD", "BODY-MOD", "a `mod x;` inside a body");
                continue;
            }
            if m.attrs.iter().any(|a| attr_name(a) == "path") {
                eng.fail(
                    &rel,
                    &name,
                    "MOD",
                    "PATH-ATTR",
                    "#[path] on a mod: not followed",
                );
                continue;
            }
            let mut d = dir.clone();
            for p in &at.modpath {
                d = format!("{d}/{p}");
            }
            let a = format!("{d}/{name}.rs");
            let b = format!("{d}/{name}/mod.rs");
            let child = match (vfs.contains_key(&a), vfs.contains_key(&b)) {
                (true, true) => {
                    eng.fail(&rel, &name, "MOD", "MOD-AMBIGUOUS", format!("{a} and {b}"));
                    a
                }
                (true, false) => a,
                (false, true) => b,
                (false, false) => {
                    eng.fail(
                        &rel,
                        &name,
                        "MOD",
                        "MOD-MISSING",
                        format!("neither {a} nor {b}"),
                    );
                    continue;
                }
            };
            queue.push((child, format!("{d}/{name}"), at.depth + 1));
        }
        out.push(FileRec { rel, depth, file });
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Some(out)
}

fn surface(vfs: &Vfs, cfg: &Cfg, own: &str, deps: &[Krate]) -> Surface {
    let tys = TyIdx::build(vfs, own, deps);
    let mut eng = Eng::fresh(cfg, &tys, false);
    // ADDR and UNWIND read the whole crate, module or not
    let census = addr_census(vfs, &cfg.addr_names);
    addr_check(&mut eng, &census);
    let files = file_set(vfs, &mut eng);
    let in_set: BTreeSet<String> = files.iter().flatten().map(|f| f.rel.clone()).collect();
    // the rest of the crate: the declaration, the Elaborator's fields, an inline root
    let mut others: Vec<(String, syn::File)> = Vec::new();
    for (rel, text) in vfs {
        if in_set.contains(rel) || rel.starts_with(&format!("{MODULE}/")) {
            continue;
        }
        match syn::parse_file(text) {
            Ok(f) => others.push((rel.clone(), f)),
            Err(e) => eng.fail(rel, rel, "FILE", "PARSE", e.to_string()),
        }
    }
    let mut declared = false;
    for (rel, f) in &others {
        let mut ats = Vec::new();
        flatten(rel, &f.items, 0, &[], false, &mut ats);
        for at in &ats {
            match at.item {
                Item::Mod(m) if m.ident == MODULE => {
                    if m.content.is_some() {
                        eng.fail(
                            rel,
                            MODULE,
                            "MOD",
                            "INLINE-ROOT",
                            "an inline `mod lexscope { .. }`",
                        );
                        eng.s.present = true;
                    } else if rel == "lib.rs" && at.modpath.is_empty() && !at.in_body {
                        declared = true;
                        if m.attrs.iter().any(|a| attr_name(a) == "path") {
                            eng.fail(
                                rel,
                                MODULE,
                                "MOD",
                                "PATH-ATTR",
                                "#[path] on `mod lexscope;`",
                            );
                        }
                    } else {
                        eng.fail(
                            rel,
                            MODULE,
                            "MOD",
                            "MOD-ELSEWHERE",
                            "a `mod lexscope;` not at the crate root",
                        );
                    }
                }
                Item::Struct(s) if s.ident == ELAB && eng.defs.elab_fields.is_none() => {
                    let mut m = BTreeMap::new();
                    for (i, f) in s.fields.iter().enumerate() {
                        let n = f
                            .ident
                            .as_ref()
                            .map(|x| x.to_string())
                            .unwrap_or_else(|| i.to_string());
                        m.insert(n, f.ty.clone());
                    }
                    eng.defs.elab_fields = Some(m);
                }
                _ => {}
            }
        }
    }
    let Some(files) = files else {
        if declared {
            eng.fail(
                "lib.rs",
                MODULE,
                "MOD",
                "MOD-MISSING",
                "`mod lexscope;` with no file",
            );
        }
        stale_checks(&mut eng);
        return eng.s;
    };
    eng.s.present = true;
    eng.s.files = files.iter().map(|f| f.rel.clone()).collect();
    if !declared {
        eng.fail(
            &files[0].rel,
            MODULE,
            "MOD",
            "MOD-UNDECLARED",
            "no `mod lexscope;` in lib.rs",
        );
    }
    let mut ats: Vec<At<'_>> = Vec::new();
    for f in &files {
        flatten(&f.rel, &f.file.items, f.depth, &[], false, &mut ats);
    }
    pass1(&mut eng, &ats);
    let mut fn_index: BTreeMap<String, FnInfo<'_>> = BTreeMap::new();
    for at in &ats {
        pass2(&mut eng, at, &mut fn_index);
    }
    // macros anywhere (expression / statement position)
    for f in &files {
        let mut g = GVis {
            eng: &mut eng,
            file: f.rel.clone(),
        };
        g.visit_file(&f.file);
    }
    // the crate's other impls naming a lexscope type or trait
    for (rel, f) in &others {
        let mut oats = Vec::new();
        flatten(rel, &f.items, 0, &[], false, &mut oats);
        for at in &oats {
            let Item::Impl(im) = at.item else { continue };
            let ty = impl_self_name(&im.self_ty);
            let tr = im.trait_.as_ref().map(|(_, p, _)| last_name(p));
            let names_ours = eng.defs.defined_type(&ty)
                || tr.as_ref().is_some_and(|t| eng.defs.traits.contains(t));
            if !names_ours {
                continue;
            }
            match &tr {
                None => eng.fail(
                    rel,
                    &ty,
                    "IMPL",
                    "INHERENT-OUTSIDE",
                    format!("impl {ty} outside lexscope"),
                ),
                Some(t) if t == BOUNDARY_TRAIT => eng.fail(
                    rel,
                    &format!("{t} for {ty}"),
                    "IMPL",
                    "OUTGOING",
                    "an Outgoing impl outside lexscope",
                ),
                Some(t) if !ALLOWED_IMPLS.contains(&t.as_str()) => eng.fail(
                    rel,
                    &format!("{t} for {ty}"),
                    "IMPL",
                    "IMPL-TRAIT",
                    "a trait impl off the list, outside lexscope",
                ),
                _ => {}
            }
        }
    }
    shims_check(&mut eng, &fn_index);
    outgoing_check(&mut eng, &files[0].rel);
    e_compare(&mut eng);
    macros_check(&mut eng, &files, &ats, &others);
    bodies_check(&mut eng, &files, vfs);
    addr_fn_check(&mut eng, &census);
    stale_checks(&mut eng);
    eng.s.rows.sort();
    eng.s
}

/// MACRO-BODY: the hash of every `macro_rules!` the file set defines, and of every one the
/// rest of the crate defines that the file set calls, against MACROS (name -> sha256).
fn macros_check(
    eng: &mut Eng<'_>,
    files: &[FileRec],
    ats: &[At<'_>],
    others: &[(String, syn::File)],
) {
    let def_of = |it: &Item| -> Option<(String, String)> {
        match it {
            Item::Macro(m) if m.mac.path.is_ident("macro_rules") => m
                .ident
                .as_ref()
                .map(|i| (i.to_string(), m.mac.tokens.to_string())),
            _ => None,
        }
    };
    let mut defs: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for at in ats {
        if let Some((n, toks)) = def_of(at.item) {
            defs.entry(n)
                .or_insert_with(|| (at.file.to_string(), Vec::new()))
                .1
                .push(toks);
        }
    }
    let mut calls = MacroCalls(BTreeSet::new());
    for f in files {
        calls.visit_file(&f.file);
    }
    let mut outside: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for (rel, f) in others {
        let mut oats = Vec::new();
        flatten(rel, &f.items, 0, &[], false, &mut oats);
        for at in &oats {
            if let Some((n, toks)) = def_of(at.item) {
                if calls.0.contains(&n) && !defs.contains_key(&n) {
                    outside
                        .entry(n)
                        .or_insert_with(|| (rel.clone(), Vec::new()))
                        .1
                        .push(toks);
                }
            }
        }
    }
    defs.extend(outside);
    for (name, (file, toks)) in &defs {
        let h = sha256(toks.join("\n").as_bytes());
        eng.s.macro_hashes.push((name.clone(), h.clone()));
        match eng.cfg.macros.get(name) {
            None => eng.fail(
                file,
                name,
                "MACRO-DEF",
                "MACRO-BODY",
                format!("a macro_rules! definition off MACROS = {h}"),
            ),
            Some(w) if *w != h => eng.fail(
                file,
                name,
                "MACRO-DEF",
                "MACRO-BODY",
                format!("pinned {w} now {h}"),
            ),
            _ => {}
        }
    }
    let want: Vec<String> = eng.cfg.macros.keys().cloned().collect();
    for name in want {
        if !defs.contains_key(&name) {
            eng.fail(
                "-",
                &name,
                "MACRO-DEF",
                "STALE",
                "a MACROS row names no definition the file set defines or calls",
            );
        }
    }
}

/// BODY-ALL: the sha256 of every fn body of the file set against BODY (file, fn, sha256).
fn bodies_check(eng: &mut Eng<'_>, files: &[FileRec], vfs: &Vfs) {
    let mut got: BTreeMap<(String, String), String> = BTreeMap::new();
    for f in files {
        let mut names = FnNames {
            ctx: None,
            out: Vec::new(),
        };
        names.visit_file(&f.file);
        let ts: TokenStream = match vfs[&f.rel].parse() {
            Ok(t) => t,
            Err(e) => {
                eng.fail(&f.rel, &f.rel, "BODY", "BODY-MAP", format!("tokenize: {e}"));
                continue;
            }
        };
        let mut bodies = Vec::new();
        token_fn_bodies(ts, &mut bodies);
        let a: Vec<&str> = names.out.iter().map(|(s, _)| s.as_str()).collect();
        let b: Vec<&str> = bodies.iter().map(|(s, _)| s.as_str()).collect();
        if a != b {
            let at = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
            eng.fail(
                &f.rel,
                &f.rel,
                "BODY",
                "BODY-MAP",
                format!(
                    "syn reads {} fn bodies, the tokens {} ; first difference at #{at}: {:?} / {:?}",
                    a.len(),
                    b.len(),
                    a.get(at),
                    b.get(at)
                ),
            );
            continue;
        }
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for ((_, item), (_, body)) in names.out.iter().zip(&bodies) {
            let k = seen.entry(item.clone()).or_default();
            *k += 1;
            let item = if *k == 1 {
                item.clone()
            } else {
                format!("{item}#{k}")
            };
            got.insert((f.rel.clone(), item), sha256(body.as_bytes()));
        }
    }
    let want: BTreeMap<(String, String), String> = eng
        .cfg
        .body
        .iter()
        .map(|r| ((r[0].clone(), r[1].clone()), r[2].clone()))
        .collect();
    for ((file, item), h) in &got {
        eng.s.bodies.push((file.clone(), item.clone(), h.clone()));
        match want.get(&(file.clone(), item.clone())) {
            None => eng.fail(
                file,
                item,
                "BODY",
                "BODY",
                format!("a new row {file} · {item} · BODY = {h}"),
            ),
            Some(w) if w != h => {
                eng.fail(file, item, "BODY", "BODY", format!("pinned {w} now {h}"))
            }
            _ => {}
        }
    }
    for ((file, item), w) in &want {
        if !got.contains_key(&(file.clone(), item.clone())) {
            eng.fail(
                file,
                item,
                "BODY",
                "BODY",
                format!("a pinned row gone {file} · {item} · BODY = {w}"),
            );
        }
    }
}

/// Pass 1: what the file set defines.
fn pass1(eng: &mut Eng<'_>, ats: &[At<'_>]) {
    for at in ats {
        match at.item {
            Item::Fn(f) => {
                eng.defs.free_fns.insert(f.sig.ident.to_string());
                if at.in_body || !vis_wider(&f.vis, at.depth) {
                    eng.defs.private_fns.insert(f.sig.ident.to_string());
                }
            }
            Item::Struct(s) => *eng.defs.structs.entry(s.ident.to_string()).or_default() += 1,
            Item::Enum(e) => *eng.defs.enums.entry(e.ident.to_string()).or_default() += 1,
            Item::Trait(t) => {
                eng.defs.traits.insert(t.ident.to_string());
            }
            Item::Type(t) => {
                eng.defs.aliases.insert(
                    t.ident.to_string(),
                    ((*t.ty).clone(), !t.generics.params.is_empty()),
                );
            }
            Item::Macro(m) if m.mac.path.is_ident("macro_rules") => {
                if let Some(i) = &m.ident {
                    eng.defs.macros.insert(i.to_string());
                }
            }
            Item::Impl(im) => {
                let ty = impl_self_name(&im.self_ty);
                if let Some((_, p, _)) = &im.trait_ {
                    if last_name(p) == BOUNDARY_TRAIT {
                        let plain = im.items.iter().find_map(|ii| match ii {
                            syn::ImplItem::Type(t) if t.ident == "Plain" => Some(t.ty.clone()),
                            _ => None,
                        });
                        eng.defs.outgoing.push((ty.clone(), plain));
                    }
                    continue;
                }
                for ii in &im.items {
                    if let syn::ImplItem::Fn(f) = ii {
                        eng.defs
                            .type_fns
                            .entry(ty.clone())
                            .or_default()
                            .insert(f.sig.ident.to_string());
                        if !vis_wider(&f.vis, at.depth) {
                            eng.defs.private_fns.insert(f.sig.ident.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Pass 2: the rows and the rules, item by item.
fn pass2<'a>(eng: &mut Eng<'_>, at: &At<'a>, fn_index: &mut BTreeMap<String, FnInfo<'a>>) {
    let file = at.file;
    let name = match at.item {
        Item::Macro(m) => m
            .ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| format!("{}!", last_name(&m.mac.path))),
        other => item_ident(other).unwrap_or_default(),
    };
    // a definition of a reserved name that is not the vocabulary's own
    if let Some(n) = item_ident(at.item) {
        let vocab_kind = eng.cfg.vocab.get(&n).cloned();
        let kind = match at.item {
            Item::Struct(_) => Some("struct"),
            Item::Enum(_) => Some("enum"),
            Item::Trait(_) | Item::Type(_) | Item::Union(_) => Some("other"),
            _ => None,
        };
        if let Some(k) = kind {
            if eng.reserved(&n) && vocab_kind.as_deref() != Some(k) {
                eng.fail(file, &n, "DEF", "VOCAB-SHADOW", format!("a {k} named {n}"));
            }
            let count = eng.defs.structs.get(&n).copied().unwrap_or(0)
                + eng.defs.enums.get(&n).copied().unwrap_or(0);
            if vocab_kind.is_some() && count > 1 {
                eng.fail(
                    file,
                    &n,
                    "DEF",
                    "VOCAB-SHADOW",
                    format!("{n} defined {count} times"),
                );
            }
        }
    }
    match at.item {
        Item::Fn(f) => {
            eng.check_attrs(file, &name, &f.attrs);
            let wider = !at.in_body && vis_wider(&f.vis, at.depth);
            fn_index.insert(
                name.clone(),
                FnInfo {
                    wider,
                    sig: &f.sig,
                    block: &f.block,
                },
            );
            if wider {
                eng.fn_rows(
                    file,
                    &name,
                    &f.sig,
                    Some(&f.block),
                    None,
                    Gen::new(),
                    None,
                    "K",
                );
            } else {
                let mut names = BTreeSet::new();
                for a in &f.sig.inputs {
                    if let FnArg::Typed(pt) = a {
                        type_names(&pt.ty, &mut names);
                    }
                }
                if names.contains(ELAB) {
                    eng.fail(
                        file,
                        &name,
                        "SIG",
                        "ELAB-PARAM",
                        "a free fn taking the Elaborator",
                    );
                }
            }
        }
        Item::Impl(im) => impl_item(eng, at, im, fn_index),
        Item::Struct(s) => {
            eng.check_attrs(file, &name, &s.attrs);
            let wider = !at.in_body && vis_wider(&s.vis, at.depth);
            if wider {
                let listed = eng.cfg.vocab.get(&name).is_some_and(|k| k == "struct");
                eng.s.rows.push(Row {
                    refs: "-".into(),
                    file: file.into(),
                    tier: "-".into(),
                    kind: "TYPE".into(),
                    item: name.clone(),
                    pos: "-".into(),
                    ty: "-".into(),
                    verdict: if listed {
                        "PASS".into()
                    } else {
                        "FAIL(TYPE-UNLISTED)".into()
                    },
                });
                if !listed {
                    eng.fail(
                        file,
                        &name,
                        "-",
                        "TYPE-UNLISTED",
                        "a crate-visible struct off SURF_VOCAB",
                    );
                }
            }
            let mut gen = Gen::new();
            gen_of(&s.generics, &mut gen);
            for (i, fd) in s.fields.iter().enumerate() {
                let fname = fd
                    .ident
                    .as_ref()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| i.to_string());
                let item = format!("{name}.{fname}");
                eng.check_attrs(file, &item, &fd.attrs);
                if eng.cfg.vocab.contains_key(&name) {
                    // INTERIOR on every field of a vocabulary type, visible or not
                    let skip: BTreeSet<String> = gen.keys().cloned().collect();
                    eng.interior_pos(
                        file,
                        "-",
                        "FIELD",
                        &item,
                        "FIELD",
                        &fd.ty,
                        Some(&name),
                        &skip,
                    );
                }
                if wider && vis_wider(&fd.vis, at.depth) {
                    let mut r = eng.out_reasons(Some(name.clone()), &gen, None, &fd.ty);
                    r.insert(
                        0,
                        (
                            "VIS-FIELD".into(),
                            format!("a visible field {}", vis_text(&fd.vis)),
                        ),
                    );
                    let c = eng.card_of(&fd.ty);
                    eng.position(file, "-", "FIELD", &item, "FIELD", nt(&fd.ty), r, c);
                }
            }
        }
        Item::Enum(e) => {
            eng.check_attrs(file, &name, &e.attrs);
            let wider = !at.in_body && vis_wider(&e.vis, at.depth);
            if wider {
                eng.enums_seen.insert(name.clone());
                let listed = eng.cfg.enums.contains(&name)
                    && eng.cfg.vocab.get(&name).is_some_and(|k| k == "enum");
                eng.s.rows.push(Row {
                    refs: "-".into(),
                    file: file.into(),
                    tier: "-".into(),
                    kind: "ENUM".into(),
                    item: name.clone(),
                    pos: "-".into(),
                    ty: "-".into(),
                    verdict: if listed {
                        "PASS".into()
                    } else {
                        "FAIL(ENUM)".into()
                    },
                });
                if !listed {
                    eng.fail(file, &name, "-", "ENUM", "a crate-visible enum off ENUMS");
                }
            }
            for v in &e.variants {
                let vname = format!("{name}::{}", v.ident);
                eng.check_attrs(file, &vname, &v.attrs);
                if eng.cfg.vocab.contains_key(&name) {
                    // INTERIOR on every payload of a vocabulary enum
                    let mut skip = BTreeSet::new();
                    for g in e.generics.type_params() {
                        skip.insert(g.ident.to_string());
                    }
                    for (i, fd) in v.fields.iter().enumerate() {
                        eng.interior_pos(
                            file,
                            "-",
                            "VARIANT",
                            &vname,
                            &format!("VARIANT:{i}"),
                            &fd.ty,
                            Some(&name),
                            &skip,
                        );
                    }
                }
                if !wider {
                    continue;
                }
                for (i, fd) in v.fields.iter().enumerate() {
                    let mut r = eng.out_reasons(None, &Gen::new(), None, &fd.ty);
                    r.insert(
                        0,
                        (
                            "ENUM-PAYLOAD".into(),
                            "a visible variant with a payload".into(),
                        ),
                    );
                    let c = eng.card_of(&fd.ty);
                    eng.position(
                        file,
                        "-",
                        "VARIANT",
                        &vname,
                        &format!("VARIANT:{i}"),
                        nt(&fd.ty),
                        r,
                        c,
                    );
                }
            }
        }
        Item::Union(u) => {
            eng.check_attrs(file, &name, &u.attrs);
            eng.fail(file, &name, "-", "UNION", "a union");
        }
        Item::Const(c) => {
            eng.check_attrs(file, &name, &c.attrs);
            if !at.in_body && name != "_" && vis_wider(&c.vis, at.depth) {
                let r = eng.out_reasons(None, &Gen::new(), None, &c.ty);
                let k = eng.card_of(&c.ty);
                eng.position(file, "-", "CONST", &name, "CONST", nt(&c.ty), r, k);
            }
        }
        Item::Static(s) => {
            eng.check_attrs(file, &name, &s.attrs);
            eng.fail(file, &name, "-", "STATIC", "a static (pinned 0)");
        }
        Item::Type(t) => {
            eng.check_attrs(file, &name, &t.attrs);
            if !at.in_body && vis_wider(&t.vis, at.depth) {
                let r = eng.out_reasons(None, &Gen::new(), None, &t.ty);
                let k = eng.card_of(&t.ty);
                eng.position(file, "-", "ALIAS", &name, "ALIAS", nt(&t.ty), r, k);
            }
        }
        Item::Trait(t) => trait_item(eng, at, t),
        Item::Use(u) => use_item(eng, at, u),
        Item::Mod(m) => eng.check_attrs(file, &name, &m.attrs),
        Item::Macro(m) => {
            eng.check_attrs(file, &name, &m.attrs);
            let mname = last_name(&m.mac.path);
            if mname == "macro_rules" {
                // a definition: not exported (the attr check above) and pinned by MACRO-BODY
            } else if mname == "include" {
                eng.fail(
                    file,
                    "include!",
                    "MACRO",
                    "INCLUDE",
                    "include! joins a file",
                );
            } else if mname == "thread_local" {
                eng.fail(
                    file,
                    "thread_local!",
                    "MACRO",
                    "STATIC",
                    "a thread_local! (pinned 0)",
                );
            } else if !eng.cfg.macros.contains_key(&mname) {
                eng.fail(
                    file,
                    &format!("{mname}!"),
                    "MACRO",
                    "MACRO",
                    "an item-position macro off MACROS",
                );
            }
        }
        other => {
            let what = match other {
                Item::ExternCrate(_) => "an extern crate",
                Item::ForeignMod(_) => "an extern block",
                Item::TraitAlias(_) => "a trait alias",
                _ => "an item syn does not parse",
            };
            eng.fail(file, what, "-", "FORBIDDEN-ITEM", what);
        }
    }
}

fn impl_item<'a>(
    eng: &mut Eng<'_>,
    at: &At<'a>,
    im: &'a syn::ItemImpl,
    fn_index: &mut BTreeMap<String, FnInfo<'a>>,
) {
    let file = at.file;
    let ty = impl_self_name(&im.self_ty);
    let mut gen = Gen::new();
    gen_of(&im.generics, &mut gen);
    if let Some((neg, path, _)) = &im.trait_ {
        let tr = last_name(path);
        let item = format!("{tr} for {ty}");
        eng.check_attrs(file, &item, &im.attrs);
        let mut why: Vec<(String, String)> = Vec::new();
        if neg.is_some() {
            why.push(("IMPL-TRAIT".into(), "a negative impl".into()));
        }
        // BRIDGE (DK1-2b): an impl of one of the file set's own listed traits, admitted for the
        // (Self as written, trait) a row names, its class computed and compared
        let mut bridged = false;
        if !ALLOWED_IMPLS.contains(&tr.as_str()) {
            // WINDOW: a row naming this type and this trait (a trait of WINDOW_TRAITS) admits it
            let row = eng
                .cfg
                .window
                .iter()
                .position(|r| r[0] == ty && r[1] == tr && WINDOW_TRAITS.contains(&tr.as_str()));
            let own = eng.defs.traits.contains(&tr) && eng.cfg.traits.contains(&tr);
            let written = nt(&im.self_ty);
            let brow = eng
                .cfg
                .bridge
                .iter()
                .position(|r| own && r[0] == written && r[1] == tr);
            match (row, brow) {
                (Some(i), _) if neg.is_none() => {
                    eng.window_used.insert(i);
                }
                (None, Some(i)) if neg.is_none() => {
                    eng.bridge_used.insert(i);
                    bridged = true;
                    let class = bridge_class(&im.self_ty, &gen, &eng.defs);
                    if eng.cfg.bridge[i][2] != class {
                        why.push((
                            "BRIDGE-CLASS".into(),
                            format!(
                                "a BRIDGE row calls impl {tr} for {written} {} ; it is {class}",
                                eng.cfg.bridge[i][2]
                            ),
                        ));
                    }
                }
                _ => why.push((
                    "IMPL-TRAIT".into(),
                    format!("impl {} for {}", npath(path), nt(&im.self_ty)),
                )),
            }
        }
        if !bridged && !eng.defs.defined_type(&ty) {
            why.push((
                "IMPL-FOREIGN-SELF".into(),
                format!("a trait impl for {}", nt(&im.self_ty)),
            ));
        }
        if tr == "Debug" {
            let mut sv = SelfMention(false);
            for ii in &im.items {
                sv.visit_impl_item(ii);
            }
            if sv.0 {
                why.push(("DEBUG-OPAQUE".into(), "a Debug body that names self".into()));
            }
        }
        for ii in &im.items {
            match ii {
                syn::ImplItem::Fn(f) => {
                    let fname = format!("{item}::{}", f.sig.ident);
                    eng.check_attrs(file, &fname, &f.attrs);
                    eng.elab_param_sig(
                        file,
                        &fname,
                        &f.sig,
                        "a trait impl's method taking the Elaborator",
                    );
                }
                syn::ImplItem::Macro(m) => eng.fail(
                    file,
                    &item,
                    "MACRO",
                    "MACRO",
                    format!("{}! in an impl", last_name(&m.mac.path)),
                ),
                _ => {}
            }
        }
        eng.s.rows.push(Row {
            refs: "-".into(),
            file: file.into(),
            tier: "-".into(),
            kind: "IMPL".into(),
            item: item.clone(),
            pos: "-".into(),
            ty: "-".into(),
            verdict: if why.is_empty() {
                "PASS".into()
            } else {
                let rules: BTreeSet<&str> = why.iter().map(|(r, _)| r.as_str()).collect();
                format!("FAIL({})", rules.into_iter().collect::<Vec<_>>().join(","))
            },
        });
        for (rule, d) in why {
            eng.fail(file, &item, "IMPL", &rule, d);
        }
        return;
    }
    eng.check_attrs(file, &format!("impl {ty}"), &im.attrs);
    let elab = ty == ELAB;
    let ours = eng.defs.defined_type(&ty);
    if !elab && !ours {
        eng.fail(
            file,
            &format!("impl {ty}"),
            "IMPL",
            "INHERENT-FOREIGN",
            format!("an inherent impl of {}", nt(&im.self_ty)),
        );
    }
    for ii in &im.items {
        match ii {
            syn::ImplItem::Fn(f) => {
                let item = format!("{ty}::{}", f.sig.ident);
                eng.check_attrs(file, &item, &f.attrs);
                let has_recv = f.sig.receiver().is_some();
                let self_cb = f.sig.inputs.iter().any(|a| match a {
                    FnArg::Typed(pt) => {
                        let mut n = BTreeSet::new();
                        type_names(&pt.ty, &mut n);
                        n.contains("Self")
                    }
                    _ => false,
                });
                let tier = if elab && (has_recv || self_cb) {
                    "E"
                } else {
                    "K"
                };
                let wider = vis_wider(&f.vis, at.depth);
                fn_index.insert(
                    item.clone(),
                    FnInfo {
                        wider,
                        sig: &f.sig,
                        block: &f.block,
                    },
                );
                if wider {
                    eng.fn_rows(
                        file,
                        &item,
                        &f.sig,
                        Some(&f.block),
                        Some(ty.clone()),
                        gen.clone(),
                        None,
                        tier,
                    );
                } else {
                    // ELAB-PARAM extended (DELTA11 NB11): a private method of any impl
                    eng.elab_param_sig(
                        file,
                        &item,
                        &f.sig,
                        "a private method taking the Elaborator",
                    );
                }
                if tier == "E" {
                    e_fn(eng, file, &item, &f.block);
                }
            }
            syn::ImplItem::Const(c) => {
                let item = format!("{ty}::{}", c.ident);
                eng.check_attrs(file, &item, &c.attrs);
                if vis_wider(&c.vis, at.depth) {
                    let r = eng.out_reasons(Some(ty.clone()), &gen, None, &c.ty);
                    let k = eng.card_of(&c.ty);
                    eng.position(file, "-", "CONST", &item, "CONST", nt(&c.ty), r, k);
                }
            }
            syn::ImplItem::Type(t) => eng.fail(
                file,
                &format!("{ty}::{}", t.ident),
                "ASSOC",
                "FORBIDDEN-ITEM",
                "an inherent associated type",
            ),
            syn::ImplItem::Macro(m) => eng.fail(
                file,
                &format!("impl {ty}"),
                "MACRO",
                "MACRO",
                format!("{}! in an impl", last_name(&m.mac.path)),
            ),
            _ => eng.fail(
                file,
                &format!("impl {ty}"),
                "-",
                "FORBIDDEN-ITEM",
                "an impl item syn does not parse",
            ),
        }
    }
}

struct SelfMention(bool);
impl<'ast> Visit<'ast> for SelfMention {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.is_ident("self") {
            self.0 = true;
        }
        visit::visit_expr_path(self, p);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if tokens_name(&m.tokens, "self") {
            self.0 = true;
        }
    }
}

fn trait_item(eng: &mut Eng<'_>, at: &At<'_>, t: &syn::ItemTrait) {
    let file = at.file;
    let name = t.ident.to_string();
    eng.check_attrs(file, &name, &t.attrs);
    let wider = !at.in_body && vis_wider(&t.vis, at.depth);
    for ti in &t.items {
        if let syn::TraitItem::Macro(m) = ti {
            eng.fail(
                file,
                &name,
                "MACRO",
                "MACRO",
                format!("{}! in a trait", last_name(&m.mac.path)),
            );
        }
    }
    if !wider {
        for ti in &t.items {
            if let syn::TraitItem::Fn(f) = ti {
                let item = format!("{name}::{}", f.sig.ident);
                eng.elab_param_sig(
                    file,
                    &item,
                    &f.sig,
                    "a private trait's method taking the Elaborator",
                );
            }
        }
        return;
    }
    let listed = eng.cfg.traits.contains(&name);
    eng.s.rows.push(Row {
        refs: "-".into(),
        file: file.into(),
        tier: "-".into(),
        kind: "TRAIT".into(),
        item: name.clone(),
        pos: "-".into(),
        ty: "-".into(),
        verdict: if listed {
            "PASS".into()
        } else {
            "FAIL(TRAIT-UNLISTED)".into()
        },
    });
    if !listed {
        eng.fail(
            file,
            &name,
            "-",
            "TRAIT-UNLISTED",
            "a crate-visible trait off SURF_TRAITS",
        );
    }
    if !t.generics.params.is_empty() {
        eng.fail(file, &name, "GENERIC", "BOUND", "a generic trait");
    }
    for b in &t.supertraits {
        // DK1-2b: a supertrait the file set defines and SURF_TRAITS lists passes, as in IN-TOTAL's
        // in_bounds (SlotKey's sealed half `slot_seal::SlotText`)
        let own = match b {
            TypeParamBound::Trait(tb) => {
                let n = last_name(&tb.path);
                eng.defs.traits.contains(&n) && eng.cfg.traits.contains(&n)
            }
            _ => false,
        };
        if !(J::pass_bound(b) || own) {
            eng.fail(
                file,
                &name,
                "SUPER",
                "BOUND",
                format!("supertrait {}", nbound(b)),
            );
        }
    }
    let mut gen = Gen::new();
    gen_of(&t.generics, &mut gen);
    for ti in &t.items {
        match ti {
            syn::TraitItem::Fn(f) => {
                let item = format!("{name}::{}", f.sig.ident);
                eng.check_attrs(file, &item, &f.attrs);
                eng.fn_rows(
                    file,
                    &item,
                    &f.sig,
                    f.default.as_ref(),
                    None,
                    gen.clone(),
                    Some(name.clone()),
                    "K",
                );
            }
            syn::TraitItem::Const(c) => {
                let item = format!("{name}::{}", c.ident);
                eng.check_attrs(file, &item, &c.attrs);
                let r = eng.out_reasons(None, &gen, Some(name.clone()), &c.ty);
                let k = eng.card_of(&c.ty);
                eng.position(file, "-", "CONST", &item, "CONST", nt(&c.ty), r, k);
            }
            syn::TraitItem::Type(ty) => {
                let item = format!("{name}::{}", ty.ident);
                eng.check_attrs(file, &item, &ty.attrs);
                let mut r: Reasons = ty
                    .bounds
                    .iter()
                    .filter(|b| !J::pass_bound(b))
                    .map(|b| ("BOUND".to_string(), nbound(b)))
                    .collect();
                let (shown, card) = match &ty.default {
                    Some((_, d)) => {
                        r.extend(eng.out_reasons(None, &gen, Some(name.clone()), d));
                        (nt(d), eng.card_of(d))
                    }
                    None => ("-".into(), None),
                };
                eng.position(
                    file,
                    "-",
                    "ASSOC",
                    &item,
                    &format!("ASSOC:{}", ty.ident),
                    shown,
                    r,
                    card,
                );
            }
            _ => {}
        }
    }
}

fn use_item(eng: &mut Eng<'_>, at: &At<'_>, u: &syn::ItemUse) {
    let file = at.file;
    let wider = !at.in_body && vis_wider(&u.vis, at.depth);
    let mut leaves: Vec<(Vec<String>, Option<String>, bool)> = Vec::new();
    fn walk(
        prefix: &mut Vec<String>,
        t: &syn::UseTree,
        out: &mut Vec<(Vec<String>, Option<String>, bool)>,
    ) {
        match t {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                walk(prefix, &p.tree, out);
                prefix.pop();
            }
            syn::UseTree::Name(n) => {
                let mut v = prefix.clone();
                v.push(n.ident.to_string());
                out.push((v, None, false));
            }
            syn::UseTree::Rename(r) => {
                let mut v = prefix.clone();
                v.push(r.ident.to_string());
                out.push((v, Some(r.rename.to_string()), false));
            }
            syn::UseTree::Glob(_) => out.push((prefix.clone(), None, true)),
            syn::UseTree::Group(g) => {
                for x in &g.items {
                    walk(prefix, x, out);
                }
            }
        }
    }
    walk(&mut Vec::new(), &u.tree, &mut leaves);
    for (path, rename, glob) in leaves {
        let text = format!(
            "{}{}{}",
            path.join("::"),
            if glob { "::*" } else { "" },
            rename
                .as_ref()
                .map(|r| format!(" as {r}"))
                .unwrap_or_default()
        );
        let leaf = path.last().cloned().unwrap_or_default();
        let internal = internal_prefix(&path, &eng.defs);
        let mut why: Reasons = Vec::new();
        if let Some(r) = &rename {
            if eng.reserved(r) || r == "Self" {
                why.push((
                    "USE-SHADOW".into(),
                    format!("renamed to a reserved name {r}"),
                ));
            }
        } else if !glob && !internal && eng.cfg.vocab.contains_key(&leaf) {
            why.push((
                "USE-SHADOW".into(),
                format!("a vocabulary name from outside: {leaf}"),
            ));
        }
        if wider {
            if glob {
                why.push(("GLOB-USE".into(), "a visible glob re-export".into()));
            } else {
                let defined = eng.defs.defined_type(&leaf)
                    || eng.defs.traits.contains(&leaf)
                    || eng.defs.free_fns.contains(&leaf)
                    || eng.defs.aliases.contains_key(&leaf)
                    || eng.defs.mods.contains(&leaf);
                if !internal || !defined {
                    why.push((
                        "USE-FOREIGN".into(),
                        "a re-export of an item outside the file set".into(),
                    ));
                }
            }
            eng.s.rows.push(Row {
                refs: "-".into(),
                file: file.into(),
                tier: "-".into(),
                kind: "USE".into(),
                item: text.clone(),
                pos: "-".into(),
                ty: "-".into(),
                verdict: if why.is_empty() {
                    "PASS".into()
                } else {
                    let rules: BTreeSet<&str> = why.iter().map(|(r, _)| r.as_str()).collect();
                    format!("FAIL({})", rules.into_iter().collect::<Vec<_>>().join(","))
                },
            });
        }
        for (rule, d) in why {
            eng.fail(file, &text, "USE", &rule, d);
        }
    }
}

/// Macros in expression / statement position, and attributes on statements and parameters.
struct GVis<'e, 'c> {
    eng: &'e mut Eng<'c>,
    file: String,
}

impl<'ast> Visit<'ast> for GVis<'_, '_> {
    fn visit_item_macro(&mut self, _: &'ast syn::ItemMacro) {}
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = last_name(&m.path);
        let file = self.file.clone();
        if name == "include" {
            self.eng.fail(
                &file,
                "include!",
                "MACRO",
                "INCLUDE",
                "include! joins a file",
            );
        } else if name == "thread_local" {
            self.eng.fail(
                &file,
                "thread_local!",
                "MACRO",
                "STATIC",
                "a thread_local! (pinned 0)",
            );
        } else if self.eng.defs.macros.contains(&name) && !self.eng.cfg.macros.contains_key(&name) {
            self.eng.fail(
                &file,
                &format!("{name}!"),
                "MACRO",
                "MACRO",
                "a module macro off MACROS",
            );
        }
    }
    fn visit_type_macro(&mut self, m: &'ast syn::TypeMacro) {
        let file = self.file.clone();
        self.eng.fail(
            &file,
            &format!("{}!", last_name(&m.mac.path)),
            "MACRO",
            "MACRO",
            "a macro in type position",
        );
    }
    fn visit_local(&mut self, l: &'ast syn::Local) {
        self.check(&l.attrs);
        visit::visit_local(self, l);
    }
    fn visit_pat_type(&mut self, p: &'ast syn::PatType) {
        self.check(&p.attrs);
        visit::visit_pat_type(self, p);
    }
}

impl GVis<'_, '_> {
    fn check(&mut self, attrs: &[syn::Attribute]) {
        let file = self.file.clone();
        self.eng.check_attrs(&file, "(body)", attrs);
    }
}

fn e_fn(eng: &mut Eng<'_>, file: &str, item: &str, block: &syn::Block) {
    let mut v = EVis {
        defs: &eng.defs,
        cfg_vocab: &eng.cfg.vocab,
        cfg_window: &eng.cfg.window,
        f: BTreeMap::new(),
        c: BTreeMap::new(),
        m: BTreeMap::new(),
        alias: Vec::new(),
    };
    v.visit_block(block);
    let (f, c, m, alias) = (v.f, v.c, v.m, v.alias);
    for a in alias {
        eng.fail(file, item, "BODY", "ALIAS-SELF", a);
    }
    eng.s
        .e_rows
        .push((item.into(), "FN".into(), "-".into(), "-".into()));
    // BODY is BODY-ALL's (`bodies_check`), for every fn of the file set
    for (kind, map) in [("OUT-F", &f), ("OUT-C", &c), ("OUT-M", &m)] {
        for (name, k) in map {
            eng.s
                .e_rows
                .push((item.into(), kind.into(), name.clone(), k.to_string()));
        }
    }
}

fn shims_check(eng: &mut Eng<'_>, fns: &BTreeMap<String, FnInfo<'_>>) {
    let rows = eng.cfg.shims.clone();
    for row in rows {
        let (shim, private, wrapper) = (&row[0], &row[1], &row[2]);
        let file = "-";
        let Some(info) = fns.get(shim) else {
            eng.fail(
                file,
                shim,
                "SHIM",
                "SHIM",
                "the shim is not in the file set",
            );
            continue;
        };
        if !info.wider {
            eng.fail(file, shim, "SHIM", "SHIM", "the shim is not crate-visible");
        }
        let params: Vec<String> = info
            .sig
            .inputs
            .iter()
            .filter_map(|a| match a {
                FnArg::Typed(pt) => match &*pt.pat {
                    syn::Pat::Ident(i) => Some(i.ident.to_string()),
                    _ => Some("_".into()),
                },
                FnArg::Receiver(_) => None,
            })
            .collect();
        let got = shim_shape(info.block, &params);
        match got {
            Err(why) => eng.fail(file, shim, "SHIM", "SHIM", why),
            Ok((p, w)) => {
                let p_item = if fns.contains_key(&p) {
                    p.clone()
                } else {
                    fns.keys()
                        .find(|k| k.rsplit("::").next() == Some(p.as_str()))
                        .cloned()
                        .unwrap_or(p.clone())
                };
                if &p_item != private || &w != wrapper {
                    eng.fail(
                        file,
                        shim,
                        "SHIM",
                        "SHIM",
                        format!("pinned ({private}, {wrapper}) now ({p_item}, {w})"),
                    );
                }
                match fns.get(&p_item) {
                    Some(pi) if !pi.wider => {}
                    _ => eng.fail(
                        file,
                        shim,
                        "SHIM",
                        "SHIM",
                        format!("{p_item} is not a private lexscope fn"),
                    ),
                }
                if !eng.defs.private_fns.contains(&w) {
                    eng.fail(
                        file,
                        shim,
                        "SHIM",
                        "SHIM",
                        format!("wrapper {w} is not a private lexscope fn"),
                    );
                }
            }
        }
    }
}

/// The shim shape: one expression, `W(C)` or `C.W()`, where C calls one fn with the shim's
/// own parameters in order. Returns (C's fn, W).
fn shim_shape(b: &syn::Block, params: &[String]) -> Result<(String, String), String> {
    if b.stmts.len() != 1 {
        return Err(format!(
            "a body of {} statements, not one expression",
            b.stmts.len()
        ));
    }
    let syn::Stmt::Expr(e, None) = &b.stmts[0] else {
        return Err("the one statement is not a tail expression".into());
    };
    let (w, inner) = match e {
        Expr::Call(c) if c.args.len() == 1 => match &*c.func {
            Expr::Path(p) => (last_name(&p.path), &c.args[0]),
            _ => return Err("the wrapper is not a path".into()),
        },
        Expr::MethodCall(m) if m.args.is_empty() => (m.method.to_string(), &*m.receiver),
        _ => return Err("not `W(C)` nor `C.W()`".into()),
    };
    let (callee, args) = match inner {
        Expr::MethodCall(m) if is_self_expr(&m.receiver) => (m.method.to_string(), &m.args),
        Expr::Call(c) => match &*c.func {
            Expr::Path(p) => (last_name(&p.path), &c.args),
            _ => return Err("the inner call is not a path".into()),
        },
        _ => return Err("the wrapped expression is not one call".into()),
    };
    let got: Vec<String> = args
        .iter()
        .map(|a| match a {
            Expr::Path(p) if p.path.segments.len() == 1 => last_name(&p.path),
            _ => "<expr>".into(),
        })
        .collect();
    if got != params {
        return Err(format!(
            "arguments {got:?}, not the shim's parameters {params:?} in order"
        ));
    }
    Ok((callee, w))
}

fn outgoing_check(eng: &mut Eng<'_>, file: &str) {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let got = eng.defs.outgoing.clone();
    for (ty, plain) in &got {
        *seen.entry(ty.clone()).or_default() += 1;
        let p = plain.as_ref().map(nt).unwrap_or_else(|| "-".into());
        let item = format!("{BOUNDARY_TRAIT} for {ty}");
        match eng.cfg.outgoing.get(ty) {
            None => eng.fail(
                file,
                &item,
                "IMPL",
                "OUTGOING",
                format!("an Outgoing impl off the pinned two (Plain = {p})"),
            ),
            Some(w) if *w != p => eng.fail(
                file,
                &item,
                "IMPL",
                "OUTGOING",
                format!("Plain pinned {w} now {p}"),
            ),
            _ => {}
        }
    }
    for (ty, n) in &seen {
        if *n > 1 {
            eng.fail(
                file,
                &format!("{BOUNDARY_TRAIT} for {ty}"),
                "IMPL",
                "OUTGOING",
                format!("{n} impls"),
            );
        }
    }
    let want: Vec<String> = eng.cfg.outgoing.keys().cloned().collect();
    for ty in want {
        if !seen.contains_key(&ty) {
            eng.fail(
                file,
                &format!("{BOUNDARY_TRAIT} for {ty}"),
                "IMPL",
                "STALE",
                "a pinned Outgoing impl is gone",
            );
        }
    }
}

/// E: the computed (item, kind, name, count) against the pinned rows ; FLOW checked.
fn e_compare(eng: &mut Eng<'_>) {
    let key = |item: &str, kind: &str, name: &str| -> String {
        if kind == "FN" {
            format!("{item}\t{kind}")
        } else {
            format!("{item}\t{kind}\t{name}")
        }
    };
    let mut got: BTreeMap<String, (String, String, String)> = BTreeMap::new();
    for (item, kind, name, n) in &eng.s.e_rows {
        got.insert(
            key(item, kind, name),
            (item.clone(), kind.clone(), n.clone()),
        );
    }
    let mut want: BTreeMap<String, (String, String, String, String)> = BTreeMap::new();
    for r in &eng.cfg.e {
        want.insert(
            key(&r[0], &r[1], &r[2]),
            (r[0].clone(), r[1].clone(), r[3].clone(), r[4].clone()),
        );
    }
    let mut fails = Vec::new();
    for (k, (item, kind, v)) in &got {
        let rule = if kind == "FN" {
            "E-LIST"
        } else {
            kind.as_str()
        };
        match want.get(k) {
            None => fails.push((
                item.clone(),
                kind.clone(),
                rule.to_string(),
                format!("a new row {} = {v}", k.replace('\t', " · ")),
            )),
            Some((_, _, w, _)) if w != v => fails.push((
                item.clone(),
                kind.clone(),
                rule.to_string(),
                format!("pinned {w} now {v}"),
            )),
            _ => {}
        }
    }
    for (k, (item, kind, w, flow)) in &want {
        let rule = if kind == "FN" {
            "E-LIST"
        } else {
            kind.as_str()
        };
        if !got.contains_key(k) {
            fails.push((
                item.clone(),
                kind.clone(),
                rule.to_string(),
                format!("a pinned row gone {} = {w}", k.replace('\t', " · ")),
            ));
        }
        let flow_ok = if kind == "FN" {
            flow == "-"
        } else {
            FLOWS.contains(&flow.as_str())
                || flow.strip_prefix("text:").is_some_and(|s| !s.is_empty())
        };
        if !flow_ok {
            fails.push((
                item.clone(),
                kind.clone(),
                "FLOW".into(),
                format!("FLOW `{flow}`"),
            ));
        }
    }
    for (item, kind, rule, d) in fails {
        eng.fail("-", &item, &kind, &rule, d);
    }
}

fn stale_checks(eng: &mut Eng<'_>) {
    let leave = eng.cfg.leave.clone();
    for (i, l) in leave.iter().enumerate() {
        if !eng.leave_used.contains(&i) {
            eng.fail(
                "-",
                &l[1],
                &l[2],
                "STALE",
                format!("LEAVE {} matches no failing position", l[0]),
            );
        }
    }
    let enter = eng.cfg.enter.clone();
    for (i, e) in enter.iter().enumerate() {
        if !eng.enter_used.contains(&i) {
            eng.fail(
                "-",
                &e[0],
                &format!("IN:{}", e[1]),
                "STALE",
                "an ENTER row matches no text or number parameter",
            );
        }
        if !ROLES.contains(&e[2].as_str()) {
            eng.fail(
                "-",
                &e[0],
                &format!("IN:{}", e[1]),
                "ROLE",
                format!("role `{}` off {ROLES:?}", e[2]),
            );
        }
    }
    let enums = eng.cfg.enums.clone();
    for e in enums {
        if !eng.enums_seen.contains(&e) {
            eng.fail("-", &e, "-", "STALE", "an ENUMS row names no visible enum");
        }
    }
    let window = eng.cfg.window.clone();
    for (i, w) in window.iter().enumerate() {
        let item = format!("{} for {}", w[1], w[0]);
        if !WINDOW_TRAITS.contains(&w[1].as_str()) {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "WINDOW-TRAIT",
                format!("a WINDOW row's trait off {WINDOW_TRAITS:?}"),
            );
        }
        if w[2].is_empty() || w[2] == "-" {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "WINDOW-UNIT",
                "a WINDOW row names no deleting unit",
            );
        }
        if !eng.window_used.contains(&i) {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "STALE",
                "a WINDOW row matches no impl of the file set",
            );
        }
    }
    let bridge = eng.cfg.bridge.clone();
    for (i, b) in bridge.iter().enumerate() {
        let item = format!("{} for {}", b[1], b[0]);
        if !(eng.defs.traits.contains(&b[1]) && eng.cfg.traits.contains(&b[1])) {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "BRIDGE-TRAIT",
                "a BRIDGE row's trait is not a trait the file set defines and SURF_TRAITS lists",
            );
        }
        if !BRIDGE_CLASSES.contains(&b[2].as_str()) {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "BRIDGE-CLASS",
                format!("a BRIDGE row's class off {BRIDGE_CLASSES:?}"),
            );
        }
        if b[3].is_empty() || b[3] == "-" {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "BRIDGE-UNIT",
                "a BRIDGE row names no deleting unit",
            );
        }
        if !eng.bridge_used.contains(&i) {
            eng.fail(
                "-",
                &item,
                "IMPL",
                "STALE",
                "a BRIDGE row matches no impl of the file set",
            );
        }
    }
    let card = eng.cfg.card.clone();
    for (i, c) in card.iter().enumerate() {
        if !eng.card_used.contains(&i) {
            eng.fail(
                "-",
                &c[0],
                &c[1],
                "STALE",
                format!("a CARD row ({}) matches no position", c[2]),
            );
        }
        let m = c[3].as_str();
        let ok = m == "caller"
            || MEANS_PREFIXES
                .iter()
                .any(|p| m.strip_prefix(p).is_some_and(|r| !r.is_empty()));
        if !ok {
            eng.fail("-", &c[0], &c[1], "MEANS", format!("MEANS `{m}`"));
        }
    }
    let cb = eng.cfg.cb.clone();
    for (i, c) in cb.iter().enumerate() {
        let pos = format!("CB:{}", c[1]);
        if !eng.cb_used.contains(&i) {
            eng.fail(
                "-",
                &c[0],
                &pos,
                "STALE",
                "a CB row matches no callback parameter",
            );
        }
        let calls = c[4].as_str();
        let ok = (calls == "once" && c[2] == "FnOnce")
            || CALLS_PREFIXES
                .iter()
                .any(|p| calls.strip_prefix(p).is_some_and(|r| !r.is_empty()));
        if !ok {
            eng.fail(
                "-",
                &c[0],
                &pos,
                "CALLS",
                format!("CALLS `{calls}` ({})", c[2]),
            );
        }
    }
    let interior = eng.cfg.interior.clone();
    for (i, r) in interior.iter().enumerate() {
        if !eng.interior_used.contains(&i) {
            eng.fail(
                "-",
                &r[0],
                &r[1],
                "STALE",
                format!("an INTERIOR row ({}) matches no position", r[2]),
            );
        }
        if r[3].is_empty() || r[3] == "-" {
            eng.fail("-", &r[0], &r[1], "WHY", "an INTERIOR row with no reason");
        }
    }
    if !eng.s.present {
        let n = eng.cfg.shims.len()
            + eng.cfg.outgoing.len()
            + eng.cfg.e.len()
            + eng.cfg.body.len()
            + eng.cfg.macros.len();
        if n > 0 {
            eng.fail(
                "-",
                MODULE,
                "-",
                "STALE",
                format!("{n} SHIMS / OUTGOING / E / BODY / MACROS rows with no module"),
            );
        }
    }
}

/// A fixture tree: its `src` (keys without `../`) and its outer files (`../Cargo.toml`, a
/// dependency's `../../<dep>/..`), the `table:` hunk keys left out.
fn split_tree(vfs: &Vfs) -> (Vfs, Vfs) {
    let mut src = Vfs::new();
    let mut outer = Vfs::new();
    for (k, v) in vfs {
        if k.starts_with("../") {
            outer.insert(k.clone(), v.clone());
        } else if !k.starts_with("table:") {
            src.insert(k.clone(), v.clone());
        }
    }
    (src, outer)
}

/// The engine on a fixture tree.
fn run(vfs: &Vfs, cfg: &Cfg) -> Surface {
    let (src, outer) = split_tree(vfs);
    let (own, deps) = dep_crates(&Tree::Map(&outer));
    let mut s = surface(&src, cfg, &own, &deps);
    s.e_rows.sort();
    s
}

/// The engine on the real crate (its `src`, or `SCOPE_HOME_SRC`'s copy, with the path
/// dependencies its Cargo.toml names).
fn run_crate(vfs: &Vfs, cfg: &Cfg) -> Surface {
    let (own, deps) = dep_crates(&crate_tree());
    let mut s = surface(vfs, cfg, &own, &deps);
    s.e_rows.sort();
    s
}

/// A fixture's `table:<NAME>` hunks applied to the pinned table: each `- ` line removes one
/// row (it must match exactly one), each `+ ` line appends one.
fn table_overrides(vfs: &Vfs) -> BTreeMap<String, String> {
    let mut over = BTreeMap::new();
    for (k, v) in vfs {
        let Some(name) = k.strip_prefix("table:") else {
            continue;
        };
        let mut lines: Vec<String> = tables::get(name).lines().map(str::to_string).collect();
        for l in v.lines() {
            if let Some(x) = l.strip_prefix("- ") {
                let n = lines.iter().filter(|r| r.as_str() == x).count();
                assert_eq!(n, 1, "table hunk {name}: `- {x}` must match one row");
                lines.retain(|r| r.as_str() != x);
            } else if let Some(x) = l.strip_prefix("+ ") {
                lines.push(x.to_string());
            }
        }
        over.insert(name.to_string(), lines.join("\n"));
    }
    over
}

// =====================================================================================
// Tables, fixtures, dump
// =====================================================================================

fn table(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .skip(1)
        .map(|l| l.split('\t').map(|c| c.trim().to_string()).collect())
        .collect()
}

fn dump(name: &str, header: &str, rows: &[String]) {
    if let Ok(dir) = std::env::var("SCOPE_SURFACE_DUMP") {
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

/// The fixture file: `== NAME` sections ; CLEAN holds `--- <path>` files ; every other
/// section is CLEAN with planted hunks: `@@ <path>` then `- ` lines (replaced, one exact
/// match) and `+ ` lines (the replacement, or appended to the file when no `- ` line).
fn fixtures() -> BTreeMap<String, Vfs> {
    let text = match std::env::var("SCOPE_SURFACE_FIXTURES") {
        Ok(p) if !p.is_empty() => {
            std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
        }
        _ => include_str!("scope_surface.fixtures").to_string(),
    };
    let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
    for l in text.lines() {
        if let Some(n) = l.strip_prefix("== ") {
            sections.push((n.trim().to_string(), Vec::new()));
        } else if let Some(s) = sections.last_mut() {
            s.1.push(l);
        }
    }
    let mut clean = Vfs::new();
    let (_, clean_lines) = sections
        .iter()
        .find(|(n, _)| n == "CLEAN")
        .expect("a CLEAN fixture");
    let mut cur: Option<String> = None;
    for l in clean_lines {
        if let Some(p) = l.strip_prefix("--- ") {
            cur = Some(p.trim().to_string());
            clean.insert(p.trim().to_string(), String::new());
        } else if let Some(p) = l.strip_prefix("@@ table:") {
            // a table hunk in CLEAN applies to every fixture (a negative control's door)
            cur = Some(format!("table:{}", p.trim()));
            clean.entry(format!("table:{}", p.trim())).or_default();
        } else if let Some(c) = &cur {
            let t = clean.get_mut(c).expect("file");
            if c.starts_with("table:") && !(l.starts_with("- ") || l.starts_with("+ ")) {
                continue;
            }
            t.push_str(l);
            t.push('\n');
        }
    }
    let mut out = BTreeMap::new();
    out.insert("CLEAN".to_string(), clean.clone());
    for (name, lines) in &sections {
        if name == "CLEAN" {
            continue;
        }
        let mut vfs = clean.clone();
        let mut hunks: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
        for l in lines {
            if let Some(p) = l.strip_prefix("@@ ") {
                hunks.push((p.trim().to_string(), Vec::new(), Vec::new()));
            } else if let Some(h) = hunks.last_mut() {
                if let Some(x) = l
                    .strip_prefix("- ")
                    .or(if *l == "-" { Some("") } else { None })
                {
                    h.1.push(x.to_string());
                } else if let Some(x) =
                    l.strip_prefix("+ ")
                        .or(if *l == "+" { Some("") } else { None })
                {
                    h.2.push(x.to_string());
                }
            }
        }
        for (path, minus, plus) in hunks {
            let t = vfs.entry(path.clone()).or_default();
            if path.starts_with("table:") {
                for m in &minus {
                    t.push_str(&format!("- {m}\n"));
                }
                for p in &plus {
                    t.push_str(&format!("+ {p}\n"));
                }
                continue;
            }
            if minus.is_empty() {
                for p in &plus {
                    t.push_str(p);
                    t.push('\n');
                }
            } else {
                let old = minus.join("\n");
                assert_eq!(
                    t.matches(&old).count(),
                    1,
                    "fixture {name}: the `-` lines must match {path} exactly once: {old:?}"
                );
                *t = t.replacen(&old, &plus.join("\n"), 1);
            }
        }
        out.insert(name.clone(), vfs);
    }
    out
}

fn fail_key(f: &Fail) -> String {
    format!("{}\t{}\t{}", f.item, f.pos, f.rule)
}

// =====================================================================================
// The tests
// =====================================================================================

#[test]
fn ht_surf_engine_on_fixtures() {
    // the BODY kernel first: two known SHA-256 vectors
    assert_eq!(
        sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let cfg = cfg("FIX");
    let fx = fixtures();
    let mut msgs = Vec::new();
    // a fixture's `@@ table:` hunks replace pins for that fixture only
    let cfg_of = |vfs: &Vfs| -> Option<Cfg> {
        let over = table_overrides(vfs);
        if over.is_empty() {
            None
        } else {
            Some(cfg_with("FIX", &over))
        }
    };

    // the positive control: the clean module passes, its surface as pinned
    let clean_cfg = cfg_of(&fx["CLEAN"]);
    let clean = run(&fx["CLEAN"], clean_cfg.as_ref().unwrap_or(&cfg));
    assert!(clean.present, "the CLEAN fixture has no lexscope module");
    for f in &clean.fails {
        msgs.push(format!("CLEAN (must pass) {}", f.line()));
    }
    let rows: Vec<String> = clean.rows.iter().map(Row::line).collect();
    dump(
        "FIX_ROWS",
        "file\ttier\tkind\titem\tposition\ttype\tverdict\trefs",
        &rows,
    );
    let want_rows: Vec<String> = table(&tables::get("FIX_ROWS"))
        .iter()
        .map(|r| r.join("\t"))
        .collect();
    diff_lists("FIX_ROWS", &rows, &want_rows, &mut msgs);
    let types: Vec<String> = clean.out_types.iter().cloned().collect();
    dump("FIX_OUT_TYPES", "type", &types);
    let want_types: Vec<String> = table(&tables::get("FIX_OUT_TYPES"))
        .iter()
        .map(|r| r[0].clone())
        .collect();
    diff_lists("FIX_OUT_TYPES", &types, &want_types, &mut msgs);
    let erows: Vec<String> = clean
        .e_rows
        .iter()
        .map(|(a, b, c, d)| format!("{a}\t{b}\t{c}\t{d}"))
        .collect();
    dump("FIX_E", "item\tkind\tname\tcount", &erows);
    let bodies: Vec<String> = clean
        .bodies
        .iter()
        .map(|(a, b, c)| format!("{a}\t{b}\t{c}"))
        .collect();
    dump("FIX_BODY", "file\tfn\tsha256", &bodies);
    let mac: Vec<String> = clean
        .macro_hashes
        .iter()
        .map(|(a, b)| format!("{a}\t{b}"))
        .collect();
    dump("FIX_MACROS", "name\tsha256", &mac);
    dump(
        "FIX_ADDR",
        "file\tfn\tform\tcount\toperand",
        &clean.addr_rows,
    );
    dump("FIX_ADDR_CALLS", "fn\tcalls", &clean.addr_calls);

    // the negative controls: each fixture fails with exactly its pinned (item, position, rule)
    let mut expect: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in rows_of(&tables::get("FIX_EXPECT"), 4, "FIX_EXPECT") {
        expect
            .entry(r[0].clone())
            .or_default()
            .insert(format!("{}\t{}\t{}", r[1], r[2], r[3]));
    }
    let mut dumped = Vec::new();
    let mut ns_failing = BTreeSet::new();
    let mut nb_failing = BTreeSet::new();
    let mut ni_failing = BTreeSet::new();
    let mut na_failing = BTreeSet::new();
    let mut nu_failing = BTreeSet::new();
    let mut nt_failing = BTreeSet::new();
    for (name, vfs) in &fx {
        if name == "CLEAN" {
            continue;
        }
        let cfg_n = cfg_of(vfs);
        let s = run(vfs, cfg_n.as_ref().unwrap_or(&cfg));
        let got: BTreeSet<String> = s.fails.iter().map(fail_key).collect();
        for f in &s.fails {
            dumped.push(format!(
                "{name}\t{}\t{}\t{}\t{}",
                f.item, f.pos, f.rule, f.detail
            ));
        }
        if got.is_empty() {
            msgs.push(format!(
                "{name}: the planted fixture PASSES — a negative control that no longer fails"
            ));
            continue;
        }
        let want = expect.get(name).cloned().unwrap_or_default();
        if got != want {
            let extra: Vec<&String> = got.difference(&want).collect();
            let missing: Vec<&String> = want.difference(&got).collect();
            msgs.push(format!(
                "{name}: fails with another set — not pinned {extra:?} ; pinned, not seen {missing:?}"
            ));
            for f in &s.fails {
                msgs.push(format!("    {}", f.line()));
            }
        } else {
            for (pre, set) in [
                ("NS", &mut ns_failing),
                ("NB", &mut nb_failing),
                ("NI", &mut ni_failing),
                ("NA", &mut na_failing),
                ("NU", &mut nu_failing),
                ("NT", &mut nt_failing),
            ] {
                if let Some(x) = name.strip_prefix(pre) {
                    let n: String = x.chars().take_while(char::is_ascii_digit).collect();
                    set.insert(n.parse::<u32>().expect("a prefix and a number"));
                }
            }
        }
    }
    dump(
        "FIX_FAILS",
        "fixture\titem\tposition\trule\tdetail",
        &dumped,
    );
    for n in expect.keys() {
        if !fx.contains_key(n) {
            msgs.push(format!("FIX_EXPECT names {n}, which is not a fixture"));
        }
    }
    // NS1-NS22 (DELTA10 L1) + NS23-NS31 (DELTA11 R2) + NS32 (DELTA12 N-1) ; NB1-NB8 (LS1
    // part 4) + NB9-NB13 ; NI1-NI10 (LS1 part 6, INTERIOR) ; NA1-NA7 (ADDR), NU1 (UNWIND),
    // NT1-NT9 (IN-TOTAL) (LS1 part 7, DELTA12 S1 S2 S4)
    for (pre, set, last) in [
        ("NS", &ns_failing, 32),
        ("NB", &nb_failing, 13),
        ("NI", &ni_failing, 10),
        ("NA", &na_failing, 7),
        ("NU", &nu_failing, 1),
        ("NT", &nt_failing, 9),
    ] {
        let missing: Vec<u32> = (1..=last).filter(|n| !set.contains(n)).collect();
        if !missing.is_empty() {
            msgs.push(format!(
                "negative controls {pre}{missing:?} do not fail as pinned"
            ));
        }
    }
    assert!(
        msgs.is_empty(),
        "HT-SURF on fixtures: {} difference(s):\n{}",
        msgs.len(),
        msgs.join("\n")
    );
}

fn diff_lists(what: &str, got: &[String], want: &[String], msgs: &mut Vec<String>) {
    let g: BTreeSet<&String> = got.iter().collect();
    let w: BTreeSet<&String> = want.iter().collect();
    for x in g.difference(&w) {
        msgs.push(format!("{what} NEW ROW  {} — a surface item or type the table lacks: judge it, pin it with its unit", x.replace('\t', " · ")));
    }
    for x in w.difference(&g) {
        msgs.push(format!(
            "{what} GONE ROW {} — remove it with the unit that removed it",
            x.replace('\t', " · ")
        ));
    }
}

#[test]
fn ht_surf_the_lexscope_surface_is_pinned() {
    let cfg = cfg("SURF");
    let vfs = load_dir(&src_dir());
    let s = run_crate(&vfs, &cfg);
    let mut msgs = Vec::new();
    let (own, deps) = dep_crates(&crate_tree());
    let tys = TyIdx::build(&vfs, &own, &deps);
    // INTERIOR's census of the crate's types (a measurement for the report, not a pin)
    dump(
        "INTERIOR_CENSUS",
        "file\tkind\tname\tholds_or_reaches\telab\tpath",
        &interior_census(&tys),
    );
    dump(
        "INTERIOR_PARAMS",
        "file\tfn\tparam\ttype\tpath",
        &interior_params(&vfs, &tys),
    );
    // IN-TOTAL's census: the index by crate, and the foreign names in parameter positions
    // of the crate-visible fns (measurements, not pins)
    dump(
        "IN_INDEX",
        "crate\tname\tkind\tfile\tholds\tunknown",
        &in_index_census(&tys),
    );
    dump(
        "IN_FOREIGN",
        "crate\tname\tholds\tuses\tfirst_use",
        &in_foreign_census(&vfs, &tys),
    );
    // ADDR / UNWIND: the computed rows (the pins are SURF_ADDR, SURF_ADDR_CALLS, SURF_UNWIND)
    dump("SURF_ADDR", "file\tfn\tform\tcount\toperand", &s.addr_rows);
    dump("SURF_ADDR_CALLS", "fn\tcalls", &s.addr_calls);
    dump("SURF_UNWIND", "file\tfn\tname\tcount", &s.unwind_rows);
    // ADDR_NAMES was read for one toolchain: a bump re-reads it (its own unit)
    let tc_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rust-toolchain.toml");
    let channel = std::fs::read_to_string(&tc_path)
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.trim().strip_prefix("channel").map(|x| {
                    x.trim_start_matches([' ', '='])
                        .trim()
                        .trim_matches('"')
                        .to_string()
                })
            })
        })
        .unwrap_or_else(|| "-".into());
    if channel != tables::get("ADDR_TOOLCHAIN") {
        msgs.push(format!(
            "ADDR_TOOLCHAIN pinned `{}`, rust-toolchain.toml says `{channel}` — re-read ADDR_NAMES against that toolchain's std",
            tables::get("ADDR_TOOLCHAIN")
        ));
    }
    let state = if s.present { "present" } else { "absent" };
    let pinned = tables::get("SURF_PRESENT");
    if state != pinned {
        msgs.push(format!(
            "SURF_PRESENT pinned `{pinned}`, the module is {state} (files {:?}) — the unit that adds \
             `crate::lexscope` (LS2a) sets `present` and pins the computed surface below",
            s.files
        ));
    }
    for f in &s.fails {
        msgs.push(f.line());
    }
    let rows: Vec<String> = s.rows.iter().map(Row::line).collect();
    dump(
        "SURF_ROWS",
        "file\ttier\tkind\titem\tposition\ttype\tverdict\trefs",
        &rows,
    );
    let want_rows: Vec<String> = table(&tables::get("SURF_ROWS"))
        .iter()
        .map(|r| r.join("\t"))
        .collect();
    diff_lists("SURF_ROWS", &rows, &want_rows, &mut msgs);
    let types: Vec<String> = s.out_types.iter().cloned().collect();
    dump("SURF_OUT_TYPES", "type", &types);
    let want_types: Vec<String> = table(&tables::get("SURF_OUT_TYPES"))
        .iter()
        .map(|r| r[0].clone())
        .collect();
    diff_lists("SURF_OUT_TYPES", &types, &want_types, &mut msgs);
    let erows: Vec<String> = s
        .e_rows
        .iter()
        .map(|(a, b, c, d)| format!("{a}\t{b}\t{c}\t{d}"))
        .collect();
    dump("SURF_E", "item\tkind\tname\tcount", &erows);
    let counts = format!(
        "present={state} files={} rows={} out_types={} e_fns={} text_flows={}",
        s.files.len(),
        s.rows.len(),
        s.out_types.len(),
        s.e_rows.iter().filter(|r| r.1 == "FN").count(),
        cfg.e.iter().filter(|r| r[4].starts_with("text:")).count()
    );
    if counts != tables::get("SURF_COUNTS") {
        msgs.push(format!(
            "SURF_COUNTS table [{}] now [{counts}] — set it with the unit that moved it",
            tables::get("SURF_COUNTS")
        ));
    }
    if !s.present {
        // the explicit absence assertion of LS1 part 4 (flipped by LS2a through SURF_PRESENT)
        assert!(
            s.rows.is_empty()
                && s.out_types.is_empty()
                && s.e_rows.is_empty()
                && s.bodies.is_empty()
                && s.macro_hashes.is_empty()
        );
    }
    assert!(
        msgs.is_empty(),
        "HT-SURF on the crate: {} difference(s):\n{}",
        msgs.len(),
        msgs.join("\n")
    );
}

/// One of DELTA10 L2's fns, judged by I-SURF on its signature on this tree.
struct PlanOut {
    vis: String,
    tier: String,
    sig: String,
    rows: Vec<Row>,
    fails: Vec<Fail>,
}

fn plan_row(vfs: &Vfs, cfg: &Cfg, tys: &TyIdx, file: &str, fname: &str) -> Result<PlanOut, String> {
    let text = vfs.get(file).ok_or_else(|| format!("no file {file}"))?;
    let f = syn::parse_file(text).map_err(|e| format!("{file}: {e}"))?;
    let mut ats = Vec::new();
    flatten(file, &f.items, 0, &[], false, &mut ats);
    let mut found: Vec<(Option<String>, Gen, &syn::Signature, String)> = Vec::new();
    for at in &ats {
        match at.item {
            Item::Fn(x) if x.sig.ident == fname => {
                found.push((None, Gen::new(), &x.sig, vis_text(&x.vis)))
            }
            Item::Impl(im) if im.trait_.is_none() => {
                for ii in &im.items {
                    if let syn::ImplItem::Fn(x) = ii {
                        if x.sig.ident == fname {
                            let mut g = Gen::new();
                            gen_of(&im.generics, &mut g);
                            found.push((
                                Some(impl_self_name(&im.self_ty)),
                                g,
                                &x.sig,
                                vis_text(&x.vis),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if found.len() != 1 {
        return Err(format!("{} fns named {fname} in {file}", found.len()));
    }
    let (self_ty, gen, sig, vis) = found.pop().expect("one");
    let mut eng = Eng::fresh(cfg, tys, true);
    let tier = if self_ty.as_deref() == Some(ELAB) && sig.receiver().is_some() {
        "E"
    } else {
        "K"
    };
    eng.fn_rows(file, fname, sig, None, self_ty, gen, None, tier);
    let params: Vec<String> = sig
        .inputs
        .iter()
        .map(|a| match a {
            FnArg::Receiver(r) => format!(
                "{}self",
                match (&r.reference, &r.mutability) {
                    (Some(_), Some(_)) => "&mut ",
                    (Some(_), None) => "&",
                    _ => "",
                }
            ),
            FnArg::Typed(pt) => {
                let n = match &*pt.pat {
                    syn::Pat::Ident(i) => i.ident.to_string(),
                    _ => "_".into(),
                };
                format!("{n}: {}", nt(&pt.ty))
            }
        })
        .collect();
    let sig_text = format!("({}){}", params.join(", "), nret(&sig.output));
    Ok(PlanOut {
        vis,
        tier: tier.to_string(),
        sig: sig_text,
        rows: eng.s.rows,
        fails: eng.s.fails.into_iter().collect(),
    })
}

#[test]
fn ht_surf_plan_today_signatures_are_pinned() {
    let cfg = cfg("SURF");
    let vfs = load_dir(&src_dir());
    let (own, deps) = dep_crates(&crate_tree());
    let tys = TyIdx::build(&vfs, &own, &deps);
    let mut msgs = Vec::new();
    let mut got_rows = Vec::new();
    let (mut n_pass, mut ret_fail, mut text_ret, mut cb_text, mut mut_text) = (0, 0, 0, 0, 0);
    let (mut n_card, mut n_cb, mut n_interior) = (0, 0, 0);
    let (mut n_enter, mut n_enter_num, mut n_unknown) = (0, 0, 0);
    let want = rows_of(&tables::get("SURF_PLAN"), 9, "SURF_PLAN");
    let mut want_map = BTreeMap::new();
    for w in &want {
        want_map.insert(format!("{}\t{}", w[1], w[2]), w[3..].join("\t"));
    }
    for w in &want {
        let (unit, fname, file) = (&w[0], &w[1], &w[2]);
        match plan_row(&vfs, &cfg, &tys, file, fname) {
            Err(e) => msgs.push(format!(
                "SURF_PLAN {fname} ({file}): {e} — the fn moved or was renamed: re-read L2's row"
            )),
            Ok(PlanOut {
                vis,
                tier,
                sig,
                rows,
                fails,
            }) => {
                let fails_txt: Vec<String> = fails
                    .iter()
                    .map(|f| format!("{}={}({})", f.pos, f.rule, f.detail))
                    .collect();
                let ins: Vec<String> = rows
                    .iter()
                    .filter(|r| r.verdict.starts_with("ENTER-NEEDED"))
                    .map(|r| r.pos.trim_start_matches("IN:").to_string())
                    .collect();
                n_enter += ins.len();
                n_enter_num += rows
                    .iter()
                    .filter(|r| r.verdict == "ENTER-NEEDED:NUMBER")
                    .count();
                n_unknown += usize::from(fails.iter().any(|f| f.rule == "IN-UNKNOWN"));
                let verdict = if fails.is_empty() { "PASS" } else { "FAIL" };
                if fails.is_empty() {
                    n_pass += 1;
                }
                ret_fail += usize::from(fails.iter().any(|f| f.pos == "RET"));
                text_ret += usize::from(fails.iter().any(|f| f.pos == "RET" && f.rule == "TEXT"));
                cb_text += usize::from(
                    fails
                        .iter()
                        .any(|f| f.pos.starts_with("CB:") && f.rule == "TEXT"),
                );
                mut_text += usize::from(
                    fails
                        .iter()
                        .any(|f| f.pos.starts_with("ARG:") && f.rule == "TEXT"),
                );
                n_card += usize::from(fails.iter().any(|f| f.rule == "CARD"));
                n_cb += usize::from(fails.iter().any(|f| f.rule == "CB"));
                n_interior += usize::from(fails.iter().any(|f| f.rule == "INTERIOR"));
                let v = format!(
                    "{vis}\t{tier}\t{sig}\t{verdict}\t{}\t{}",
                    if fails_txt.is_empty() {
                        "-".into()
                    } else {
                        fails_txt.join("; ")
                    },
                    if ins.is_empty() {
                        "-".into()
                    } else {
                        ins.join(",")
                    }
                );
                got_rows.push(format!("{unit}\t{fname}\t{file}\t{v}"));
                let k = format!("{fname}\t{file}");
                if want_map.get(&k) != Some(&v) {
                    msgs.push(format!(
                        "SURF_PLAN CHANGED {fname} ({file})\n    table [{}]\n    code  [{}] — a signature L2 moves changed: re-judge its RT row",
                        want_map.get(&k).map(|s| s.replace('\t', " · ")).unwrap_or_default(),
                        v.replace('\t', " · ")
                    ));
                }
            }
        }
    }
    dump(
        "SURF_PLAN",
        "unit\tfn\tfile\tvis\ttier\tsignature\tverdict\tfails\tin_enter",
        &got_rows,
    );
    let counts = format!(
        "fns={} pass={n_pass} fail={} ret_fail={ret_fail} ret_text={text_ret} callback_text={cb_text} mut_text={mut_text} card={n_card} cb={n_cb} interior={n_interior} enter={n_enter} enter_number={n_enter_num} in_unknown={n_unknown}",
        want.len(),
        want.len() - n_pass
    );
    if counts != tables::get("SURF_PLAN_COUNTS") {
        msgs.push(format!(
            "SURF_PLAN_COUNTS table [{}] now [{counts}]",
            tables::get("SURF_PLAN_COUNTS")
        ));
    }
    assert!(
        msgs.is_empty(),
        "SURF-PLAN: {} difference(s):\n{}",
        msgs.len(),
        msgs.join("\n")
    );
}

mod tables {
    //! The pins of `scope_surface.rs`, one section per table (`== NAME`), `#` lines notes.
    pub fn get(name: &str) -> String {
        let data = include_str!("scope_surface.tables");
        let open = format!("== {name}");
        let mut lines = data.lines().skip_while(|l| l.trim_end() != open);
        assert!(
            lines.next().is_some(),
            "scope_surface.tables has no section `{name}`"
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
