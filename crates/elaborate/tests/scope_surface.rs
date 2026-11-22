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
//! I-IN. Every text-typed IN parameter of the surface is a row of ENTER with its role.
//! I-OUT. Every lexscope fn that takes the Elaborator (a receiver in `impl Elaborator`, or
//! `Self` in a callback) is on the list E, its body pinned by OUT-F (`self.<field>` whose
//! field type is not a lexscope type), OUT-C (calls of fns not defined in lexscope), OUT-M
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
//! non-vocabulary name fails as unknown in an OUT position) ; an IN parameter of a type
//! defined outside the file set, or renamed by a `use .. as` / an alias, that holds interior
//! mutability (the interior-mutable test reads the names the parameter's type spells) ; what
//! a body that passes its BODY pin does at run time (the hash is loud, not a refusal).
//!
//! Three tests: the engine on fixtures (one clean module that passes with its whole surface
//! pinned, NS1-NS31 and the NB controls each failing with the pinned (item, position, rule)
//! set) ; the real crate, where `SURF_PRESENT` pins `absent` and the computed surface must
//! be empty until LS2a flips it (the code computes the whole surface the day the module
//! appears, with no edit here) ; SURF-PLAN, the 30 fns DELTA10 L2 moves, judged by I-SURF on
//! today's signatures and pinned.
//!
//! Environment (negative-control doors, unset in the gate): `SCOPE_HOME_SRC` reads another
//! copy of the crate's `src` ; `SCOPE_SURFACE_FIXTURES` reads another fixture file ;
//! `SCOPE_SURFACE_DUMP=<dir>` writes each computed table to `<dir>/<name>.tsv`.

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
const STD_ROOTS: &[&str] = &["std", "core", "alloc"];
const FLOWS: &[&str] = &["in", "payload"];
/// CARD's named constructors (a slice, an array and two or more `bool` are the others).
const CARD_CTORS: &[&str] = &["Option", "Vec"];
/// CARD's MEANS values: `caller`, or one of these prefixes with a non-empty rest.
const MEANS_PREFIXES: &[&str] = &["hit:", "list:"];
/// CB's CALLS values: `once` (an FnOnce), or one of these prefixes with a non-empty rest.
const CALLS_PREFIXES: &[&str] = &["per:", "probe:"];

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
    /// item, param, role
    enter: Vec<Vec<String>>,
    enums: BTreeSet<String>,
    /// shim, private fn, wrapper
    shims: Vec<Vec<String>>,
    /// type -> Plain
    outgoing: BTreeMap<String, String>,
    /// item, kind, name, count, flow
    e: Vec<Vec<String>>,
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
    let vocab = rows_of(&tables::get("SURF_VOCAB"), 2, "SURF_VOCAB")
        .into_iter()
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    let one = |name: &str| -> BTreeSet<String> {
        rows_of(&tables::get(name), 1, name)
            .into_iter()
            .map(|r| r[0].clone())
            .collect()
    };
    Cfg {
        vocab,
        traits: one("SURF_TRAITS"),
        macros: rows_of(&tables::get(&format!("{prefix}_MACROS")), 2, "MACROS")
            .into_iter()
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect(),
        leave: rows_of(&tables::get(&format!("{prefix}_LEAVE")), 5, "LEAVE"),
        card: rows_of(&tables::get(&format!("{prefix}_CARD")), 4, "CARD"),
        cb: rows_of(&tables::get(&format!("{prefix}_CB")), 5, "CB"),
        body: rows_of(&tables::get(&format!("{prefix}_BODY")), 3, "BODY"),
        enter: rows_of(&tables::get(&format!("{prefix}_ENTER")), 3, "ENTER"),
        enums: one(&format!("{prefix}_ENUMS")),
        shims: rows_of(&tables::get(&format!("{prefix}_SHIMS")), 3, "SHIMS"),
        outgoing: rows_of(&tables::get(&format!("{prefix}_OUTGOING")), 2, "OUTGOING")
            .into_iter()
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect(),
        e: rows_of(&tables::get(&format!("{prefix}_E")), 5, "E"),
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
}

impl Row {
    fn line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.file, self.tier, self.kind, self.item, self.pos, self.ty, self.verdict
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
    text: Vec<String>,
    /// (position, type) judged as OUT: a `&mut` target, an interior-mutable parameter
    outs: Vec<(String, Type)>,
    /// (position, type): a callback's argument
    cbs: Vec<(String, Type)>,
    fails: Reasons,
}

struct J<'a> {
    defs: &'a Defs,
    cfg: &'a Cfg,
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
                }
                let segs = seg_names(&p.path);
                if segs.len() == 1 {
                    if let Some(bs) = self.gen.get(&segs[0]) {
                        let bs = bs.clone();
                        self.in_bounds(pname, bs.iter(), res);
                        return;
                    }
                }
                let last = segs.last().cloned().unwrap_or_default();
                if is_text_name(&last) {
                    res.text.push(nt(t));
                }
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
// Tier E: OUT-F / OUT-C / OUT-M / BODY
// =====================================================================================

struct EVis<'a> {
    defs: &'a Defs,
    cfg_vocab: &'a BTreeMap<String, String>,
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
}

/// A fn of the file set, for the shim check.
struct FnInfo<'a> {
    wider: bool,
    sig: &'a syn::Signature,
    block: &'a syn::Block,
}

struct Eng<'a> {
    cfg: &'a Cfg,
    defs: Defs,
    s: Surface,
    leave_used: BTreeSet<usize>,
    enter_used: BTreeSet<usize>,
    card_used: BTreeSet<usize>,
    cb_used: BTreeSet<usize>,
    enums_seen: BTreeSet<String>,
    plan: bool,
}

impl Eng<'_> {
    fn fresh(cfg: &Cfg, plan: bool) -> Eng<'_> {
        Eng {
            cfg,
            defs: Defs::default(),
            s: Surface::default(),
            leave_used: BTreeSet::new(),
            enter_used: BTreeSet::new(),
            card_used: BTreeSet::new(),
            cb_used: BTreeSet::new(),
            enums_seen: BTreeSet::new(),
            plan,
        }
    }

    /// CARD: the cardinality constructors `t` holds at any depth (an alias of the file set
    /// followed), or None.
    fn card_of(&self, t: &Type) -> Option<String> {
        let mut found = BTreeSet::new();
        let mut bools = 0usize;
        self.card_walk(t, &mut found, &mut bools, 0);
        if bools >= 2 {
            found.insert(format!("{bools} bool"));
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
        self.s.rows.push(Row {
            file: file.into(),
            tier: tier.into(),
            kind: kind.into(),
            item: item.into(),
            pos: pos.into(),
            ty,
            verdict,
        });
    }

    fn judge(&self, self_ty: Option<String>, gen: Gen, in_trait: Option<String>) -> J<'_> {
        J {
            defs: &self.defs,
            cfg: self.cfg,
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

    /// The rows of one fn signature (visible) ; returns whether it is tier E.
    #[allow(clippy::too_many_arguments)]
    fn fn_rows(
        &mut self,
        file: &str,
        item: &str,
        sig: &syn::Signature,
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
        let (ret_ty, ret_r, ret_card) = match &sig.output {
            syn::ReturnType::Default => ("()".to_string(), Vec::new(), None),
            syn::ReturnType::Type(_, t) => (
                nt(t),
                self.out_reasons(self_ty.clone(), &gen, in_trait.clone(), t),
                self.card_of(t),
            ),
        };
        self.position(file, tier, "FN", item, "RET", ret_ty, ret_r, ret_card);
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
            let (res, cbs) = {
                let j = self.judge(self_ty.clone(), gen.clone(), in_trait.clone());
                let mut cbs = Vec::new();
                j.callbacks(&pt.ty, &mut cbs, 0);
                (j.in_param(&pname, &pt.ty), cbs)
            };
            for (pos, t) in &res.outs {
                let r = self.out_reasons(self_ty.clone(), &gen, in_trait.clone(), t);
                let c = self.card_of(t);
                self.position(file, tier, "FN", item, pos, nt(t), r, c);
            }
            for (pos, t) in &res.cbs {
                if is_self_ty(t) && self_ty.as_deref() == Some(ELAB) {
                    continue;
                }
                let r = self.out_reasons(self_ty.clone(), &gen, in_trait.clone(), t);
                let c = self.card_of(t);
                self.position(file, tier, "FN", item, pos, nt(t), r, c);
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
            if !res.text.is_empty() {
                let pos = format!("IN:{pname}");
                let verdict = if self.plan {
                    "ENTER-NEEDED".to_string()
                } else if let Some(k) = self
                    .cfg
                    .enter
                    .iter()
                    .position(|e| e[0] == item && e[1] == pname)
                {
                    self.enter_used.insert(k);
                    format!("ENTER:{}", self.cfg.enter[k][2])
                } else {
                    self.fail(
                        file,
                        item,
                        &pos,
                        "ENTER",
                        format!("text in: {}", res.text.join(", ")),
                    );
                    "FAIL(ENTER)".to_string()
                };
                self.s.rows.push(Row {
                    file: file.into(),
                    tier: tier.into(),
                    kind: "FN".into(),
                    item: item.into(),
                    pos,
                    ty: nt(&pt.ty),
                    verdict,
                });
            }
        }
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
            file: file.into(),
            tier: tier.into(),
            kind: "FN".into(),
            item: item.into(),
            pos,
            ty: ty.into(),
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

fn surface(vfs: &Vfs, cfg: &Cfg) -> Surface {
    let mut eng = Eng::fresh(cfg, false);
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
                eng.fn_rows(file, &name, &f.sig, None, Gen::new(), None, "K");
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
        if !ALLOWED_IMPLS.contains(&tr.as_str()) {
            why.push((
                "IMPL-TRAIT".into(),
                format!("impl {} for {}", npath(path), nt(&im.self_ty)),
            ));
        }
        if !eng.defs.defined_type(&ty) {
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
        if !J::pass_bound(b) {
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
                "an ENTER row matches no text parameter",
            );
        }
    }
    let enums = eng.cfg.enums.clone();
    for e in enums {
        if !eng.enums_seen.contains(&e) {
            eng.fail("-", &e, "-", "STALE", "an ENUMS row names no visible enum");
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

fn run(vfs: &Vfs, cfg: &Cfg) -> Surface {
    let mut s = surface(vfs, cfg);
    s.e_rows.sort();
    s
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
        } else if let Some(c) = &cur {
            let t = clean.get_mut(c).expect("file");
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

    // the positive control: the clean module passes, its surface as pinned
    let clean = run(&fx["CLEAN"], &cfg);
    assert!(clean.present, "the CLEAN fixture has no lexscope module");
    for f in &clean.fails {
        msgs.push(format!("CLEAN (must pass) {}", f.line()));
    }
    let rows: Vec<String> = clean.rows.iter().map(Row::line).collect();
    dump(
        "FIX_ROWS",
        "file\ttier\tkind\titem\tposition\ttype\tverdict",
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
    for (name, vfs) in &fx {
        if name == "CLEAN" {
            continue;
        }
        let s = run(vfs, &cfg);
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
            for (pre, set) in [("NS", &mut ns_failing), ("NB", &mut nb_failing)] {
                if let Some(x) = name.strip_prefix(pre) {
                    let n: String = x.chars().take_while(char::is_ascii_digit).collect();
                    set.insert(n.parse::<u32>().expect("NS<n> / NB<n>"));
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
    // NS1-NS22 (DELTA10 L1) + NS23-NS31 (DELTA11 R2) ; NB1-NB8 (LS1 part 4) + NB9-NB13
    for (pre, set, last) in [("NS", &ns_failing, 31), ("NB", &nb_failing, 13)] {
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
    let s = run(&vfs, &cfg);
    let mut msgs = Vec::new();
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
        "file\ttier\tkind\titem\tposition\ttype\tverdict",
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

fn plan_row(vfs: &Vfs, cfg: &Cfg, file: &str, fname: &str) -> Result<PlanOut, String> {
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
    let mut eng = Eng::fresh(cfg, true);
    let tier = if self_ty.as_deref() == Some(ELAB) && sig.receiver().is_some() {
        "E"
    } else {
        "K"
    };
    eng.fn_rows(file, fname, sig, self_ty, gen, None, tier);
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
    let mut msgs = Vec::new();
    let mut got_rows = Vec::new();
    let (mut n_pass, mut ret_fail, mut text_ret, mut cb_text, mut mut_text) = (0, 0, 0, 0, 0);
    let (mut n_card, mut n_cb) = (0, 0);
    let want = rows_of(&tables::get("SURF_PLAN"), 9, "SURF_PLAN");
    let mut want_map = BTreeMap::new();
    for w in &want {
        want_map.insert(format!("{}\t{}", w[1], w[2]), w[3..].join("\t"));
    }
    for w in &want {
        let (unit, fname, file) = (&w[0], &w[1], &w[2]);
        match plan_row(&vfs, &cfg, file, fname) {
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
                    .filter(|r| r.verdict == "ENTER-NEEDED")
                    .map(|r| r.pos.trim_start_matches("IN:").to_string())
                    .collect();
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
        "unit\tfn\tfile\tvis\ttier\tsignature\tverdict\tfails\tin_text",
        &got_rows,
    );
    let counts = format!(
        "fns={} pass={n_pass} fail={} ret_fail={ret_fail} ret_text={text_ret} callback_text={cb_text} mut_text={mut_text} card={n_card} cb={n_cb}",
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
