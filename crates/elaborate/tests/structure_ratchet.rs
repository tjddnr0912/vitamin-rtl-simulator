//! Structure ratchet: counts, from the source, five shapes that structural cleanup is
//! driving down, and fails when one RISES. When one FALLS it fails too, asking for the
//! baseline (`tests/structure_ratchet.baseline`) to be lowered in the same change, so
//! the floor follows every gain and cannot be given back silently.
//!
//! | key | counts |
//! |---|---|
//! | `a.expr_kind_matchers.<crate>` | functions that pattern-match 3 or more distinct `ExprKind` variants, per crate, outside `hdl_ast::walk` (the one sanctioned child enumeration) |
//! | `b.const_option_fns` | `pub(crate) fn` in `elaborate/src/const_*.rs` returning `Option` |
//! | `c.calls.<name>` | call sites of `const_eval_in_scope`, `const_eval_u32`, `const_self_width` in `elaborate/src` |
//! | `d.elaborator_string_keyed` | `Elaborator` fields typed `BTreeMap`/`HashMap`/`BTreeSet`/`HashSet` keyed by `String` |
//! | `e.sim_engine_elaborate_reexports` | items `sim-engine` re-exports with `pub use elaborate::…` |
//!
//! The scanner parses every file with `syn` (full syntax), so a comment or a string
//! that mentions `ExprKind::X` is not code and does not count. "Matches" means a
//! PATTERN: a `match` arm, `if let`/`let … else`/`while let`, a parameter, and the
//! pattern of `matches!`. Variants are recognised through `ExprKind::X`, an alias
//! (`use ast::ExprKind as K` → `K::X`), a glob (`use ast::ExprKind::*` → `X`) and
//! a named variant import. A macro body the scanner cannot parse that still names a
//! variant fails the test rather than being skipped. The self-tests at the bottom
//! plant positive and negative controls.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit::Visit;

const BASELINE: &str = "tests/structure_ratchet.baseline";
/// The sanctioned enumeration, excluded from (a).
const WALK_MODULE: &str = "crates/hdl-ast/src/walk.rs";
const CALLEES: [&str; 3] = ["const_eval_in_scope", "const_eval_u32", "const_self_width"];

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

fn parse(path: &Path) -> syn::File {
    let src = std::fs::read_to_string(path).expect("read source");
    syn::parse_file(&src).unwrap_or_else(|e| panic!("syn cannot parse {}: {e}", path.display()))
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

// ───────────────────────── (a) ExprKind matchers ─────────────────────────

/// The `ExprKind` variant names, read from the enum itself.
fn expr_kind_variants(root: &Path) -> BTreeSet<String> {
    let f = parse(&root.join("crates/hdl-ast/src/lib.rs"));
    for it in &f.items {
        if let syn::Item::Enum(e) = it {
            if e.ident == "ExprKind" {
                return e.variants.iter().map(|v| v.ident.to_string()).collect();
            }
        }
    }
    panic!("enum ExprKind not found in hdl-ast")
}

/// How a scope can name an `ExprKind` variant.
#[derive(Clone, Default)]
struct Names {
    /// `use …::ExprKind as K;` → `K`.
    aliases: BTreeSet<String>,
    /// `use …::ExprKind::*;`
    glob: bool,
    /// `use …::ExprKind::{A, B};` → `A`, `B`.
    imported: BTreeSet<String>,
}

/// Walk a `use` tree; record what it brings in under `names`. `prefix_last` is the
/// last segment seen so far.
fn use_tree(t: &syn::UseTree, prefix_last: Option<&str>, names: &mut Names) {
    match t {
        syn::UseTree::Path(p) => {
            let seg = p.ident.to_string();
            use_tree(&p.tree, Some(&seg), names);
        }
        syn::UseTree::Rename(r) => {
            if r.ident == "ExprKind" {
                names.aliases.insert(r.rename.to_string());
            } else {
                // A rename of something else to a name an alias used: shadowed.
                names.aliases.remove(&r.rename.to_string());
                if prefix_last == Some("ExprKind") {
                    names.imported.insert(r.rename.to_string());
                }
            }
        }
        syn::UseTree::Name(n) => {
            if prefix_last == Some("ExprKind") {
                names.imported.insert(n.ident.to_string());
            }
        }
        syn::UseTree::Glob(_) => {
            if prefix_last == Some("ExprKind") {
                names.glob = true;
            }
        }
        syn::UseTree::Group(g) => {
            for t in &g.items {
                use_tree(t, prefix_last, names);
            }
        }
    }
}

/// Every `use` item inside a function body (any depth), so a function-local alias is
/// in scope for the whole function.
#[derive(Default)]
struct LocalUses(Vec<syn::UseTree>);
impl<'ast> Visit<'ast> for LocalUses {
    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        self.0.push(u.tree.clone());
    }
    // A nested function's uses belong to it, not to the enclosing one.
    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}
    fn visit_item_impl(&mut self, _: &'ast syn::ItemImpl) {}
}

struct MatchesArgs {
    expr: syn::Expr,
    pat: syn::Pat,
    guard: Option<syn::Expr>,
}
impl Parse for MatchesArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(syn::Token![if]) {
            input.parse::<syn::Token![if]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        input.parse::<Option<syn::Token![,]>>()?;
        Ok(MatchesArgs { expr, pat, guard })
    }
}

/// One function's findings.
struct FnHits {
    name: String,
    variants: BTreeSet<String>,
}

struct MatcherScan<'v> {
    variants: &'v BTreeSet<String>,
    file_names: Names,
    /// Innermost function last; each carries the names in scope for it.
    stack: Vec<(FnHits, Names)>,
    done: Vec<FnHits>,
    /// Macro bodies that would not parse yet name a variant.
    unscanned: Vec<String>,
    impl_ty: Vec<String>,
}

impl<'v> MatcherScan<'v> {
    fn new(variants: &'v BTreeSet<String>, file: &syn::File) -> Self {
        let mut file_names = Names::default();
        // File-level (and inline-module) uses apply to the whole file.
        struct TopUses<'n>(&'n mut Names);
        impl<'ast> Visit<'ast> for TopUses<'_> {
            fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
                use_tree(&u.tree, None, self.0);
            }
            fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}
            fn visit_impl_item_fn(&mut self, _: &'ast syn::ImplItemFn) {}
            fn visit_trait_item_fn(&mut self, _: &'ast syn::TraitItemFn) {}
        }
        TopUses(&mut file_names).visit_file(file);
        MatcherScan {
            variants,
            file_names,
            stack: Vec::new(),
            done: Vec::new(),
            unscanned: Vec::new(),
            impl_ty: Vec::new(),
        }
    }

    fn enter(&mut self, name: String, body: &syn::Block) {
        let mut names = self
            .stack
            .last()
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| self.file_names.clone());
        let mut lu = LocalUses::default();
        lu.visit_block(body);
        for t in &lu.0 {
            use_tree(t, None, &mut names);
        }
        let name = match self.impl_ty.last() {
            Some(t) => format!("{t}::{name}"),
            None => name,
        };
        self.stack.push((
            FnHits {
                name,
                variants: BTreeSet::new(),
            },
            names,
        ));
    }

    fn leave(&mut self) {
        let (h, _) = self.stack.pop().expect("balanced");
        self.done.push(h);
    }

    /// The variant `path` names in the current scope, if any.
    fn variant_of_path(&self, path: &syn::Path) -> Option<String> {
        let names = self
            .stack
            .last()
            .map(|(_, n)| n)
            .unwrap_or(&self.file_names);
        let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let last = segs.last()?;
        if !self.variants.contains(last) {
            return None;
        }
        let hit = match segs.len() {
            1 => names.glob || names.imported.contains(last),
            n => {
                let prev = &segs[n - 2];
                prev == "ExprKind" || names.aliases.contains(prev)
            }
        };
        hit.then(|| last.clone())
    }

    fn record(&mut self, v: String) {
        if let Some((h, _)) = self.stack.last_mut() {
            h.variants.insert(v);
        }
    }

    /// Does a raw token stream name a variant (`ExprKind :: X`, `<alias> :: X`, or a
    /// bare `X` under a glob)? Used only for macro bodies `syn` cannot parse.
    fn tokens_name_variant(&self, ts: proc_macro2::TokenStream) -> bool {
        let names = self
            .stack
            .last()
            .map(|(_, n)| n)
            .unwrap_or(&self.file_names);
        let toks: Vec<proc_macro2::TokenTree> = ts.into_iter().collect();
        for (i, t) in toks.iter().enumerate() {
            match t {
                proc_macro2::TokenTree::Group(g) => {
                    if self.tokens_name_variant(g.stream()) {
                        return true;
                    }
                }
                proc_macro2::TokenTree::Ident(id) => {
                    let s = id.to_string();
                    if !self.variants.contains(&s) {
                        continue;
                    }
                    if names.glob || names.imported.contains(&s) {
                        return true;
                    }
                    // `<prev> :: X`
                    if i >= 3 {
                        if let (
                            proc_macro2::TokenTree::Ident(prev),
                            proc_macro2::TokenTree::Punct(c1),
                            proc_macro2::TokenTree::Punct(c2),
                        ) = (&toks[i - 3], &toks[i - 2], &toks[i - 1])
                        {
                            let p = prev.to_string();
                            if c1.as_char() == ':'
                                && c2.as_char() == ':'
                                && (p == "ExprKind" || names.aliases.contains(&p))
                            {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}

impl<'ast> Visit<'ast> for MatcherScan<'_> {
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let ty = match &*i.self_ty {
            syn::Type::Path(p) => p
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default(),
            _ => String::from("_"),
        };
        self.impl_ty.push(ty);
        syn::visit::visit_item_impl(self, i);
        self.impl_ty.pop();
    }
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.enter(f.sig.ident.to_string(), &f.block);
        syn::visit::visit_item_fn(self, f);
        self.leave();
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.enter(f.sig.ident.to_string(), &f.block);
        syn::visit::visit_impl_item_fn(self, f);
        self.leave();
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if let Some(b) = &f.default {
            self.enter(f.sig.ident.to_string(), b);
            syn::visit::visit_trait_item_fn(self, f);
            self.leave();
        }
    }
    fn visit_pat(&mut self, p: &'ast syn::Pat) {
        let path = match p {
            syn::Pat::Struct(s) => Some(&s.path),
            syn::Pat::TupleStruct(s) => Some(&s.path),
            syn::Pat::Path(s) => Some(&s.path),
            _ => None,
        };
        if let Some(v) = path.and_then(|p| self.variant_of_path(p)) {
            self.record(v);
        }
        // A bare unit variant under a glob parses as a binding.
        if let syn::Pat::Ident(id) = p {
            if id.subpat.is_none() && id.by_ref.is_none() && id.mutability.is_none() {
                let path = syn::Path::from(id.ident.clone());
                if let Some(v) = self.variant_of_path(&path) {
                    self.record(v);
                }
            }
        }
        syn::visit::visit_pat(self, p);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = m
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if name == "matches" {
            if let Ok(a) = m.parse_body::<MatchesArgs>() {
                self.visit_expr(&a.expr);
                self.visit_pat(&a.pat);
                if let Some(g) = &a.guard {
                    self.visit_expr(g);
                }
                return;
            }
        } else if let Ok(args) =
            m.parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        {
            for e in &args {
                self.visit_expr(e);
            }
            return;
        }
        if self.tokens_name_variant(m.tokens.clone()) {
            let at = self
                .stack
                .last()
                .map(|(h, _)| h.name.clone())
                .unwrap_or_default();
            self.unscanned.push(format!("{name}! in {at}"));
        }
    }
}

/// `(fn name, distinct variants matched)` for every function in `src`, plus the
/// unscannable macro bodies.
fn scan_matchers(src: &str, variants: &BTreeSet<String>) -> (Vec<(String, usize)>, Vec<String>) {
    let file = syn::parse_file(src).expect("parse");
    let mut s = MatcherScan::new(variants, &file);
    s.visit_file(&file);
    (
        s.done
            .into_iter()
            .map(|h| (h.name, h.variants.len()))
            .collect(),
        s.unscanned,
    )
}

// ───────────────────────── (b) const_* Option fns ─────────────────────────

fn returns_option(sig: &syn::Signature) -> bool {
    match &sig.output {
        syn::ReturnType::Type(_, t) => match &**t {
            syn::Type::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == "Option"),
            _ => false,
        },
        syn::ReturnType::Default => false,
    }
}

fn is_pub_crate(v: &syn::Visibility) -> bool {
    matches!(v, syn::Visibility::Restricted(r) if r.in_token.is_none() && r.path.is_ident("crate"))
}

#[derive(Default)]
struct OptionFns(Vec<String>);
impl<'ast> Visit<'ast> for OptionFns {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if is_pub_crate(&f.vis) && returns_option(&f.sig) {
            self.0.push(f.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, f);
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        if is_pub_crate(&f.vis) && returns_option(&f.sig) {
            self.0.push(f.sig.ident.to_string());
        }
        syn::visit::visit_impl_item_fn(self, f);
    }
}

fn scan_option_fns(src: &str) -> Vec<String> {
    let mut v = OptionFns::default();
    v.visit_file(&syn::parse_file(src).expect("parse"));
    v.0
}

// ───────────────────────── (c) call sites ─────────────────────────

#[derive(Default)]
struct Calls(BTreeMap<&'static str, u64>);
impl Calls {
    fn hit(&mut self, ident: &syn::Ident) {
        if let Some(n) = CALLEES.iter().find(|n| ident == *n) {
            *self.0.entry(n).or_default() += 1;
        }
    }
}
impl<'ast> Visit<'ast> for Calls {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        self.hit(&m.method);
        syn::visit::visit_expr_method_call(self, m);
    }
    /// `Self::f(…)`, `f(…)`, and `Self::f` passed as a function value.
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if let Some(s) = p.path.segments.last() {
            self.hit(&s.ident);
        }
        syn::visit::visit_expr_path(self, p);
    }
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if let Ok(args) =
            m.parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        {
            for e in &args {
                self.visit_expr(e);
            }
        } else if let Ok(a) = m.parse_body::<MatchesArgs>() {
            self.visit_expr(&a.expr);
            if let Some(g) = &a.guard {
                self.visit_expr(g);
            }
        }
    }
}

fn scan_calls(src: &str) -> BTreeMap<&'static str, u64> {
    let mut c = Calls::default();
    c.visit_file(&syn::parse_file(src).expect("parse"));
    c.0
}

// ───────────────────────── (d) String-keyed Elaborator fields ─────────────────────────

fn string_keyed(t: &syn::Type) -> bool {
    let syn::Type::Path(p) = t else {
        return false;
    };
    let Some(last) = p.path.segments.last() else {
        return false;
    };
    if !["BTreeMap", "HashMap", "BTreeSet", "HashSet"].contains(&last.ident.to_string().as_str()) {
        return false;
    }
    let syn::PathArguments::AngleBracketed(a) = &last.arguments else {
        return false;
    };
    matches!(a.args.first(), Some(syn::GenericArgument::Type(syn::Type::Path(k)))
        if k.qself.is_none() && k.path.segments.last().is_some_and(|s| s.ident == "String"))
}

fn scan_elaborator_fields(src: &str) -> Option<Vec<String>> {
    let f = syn::parse_file(src).expect("parse");
    f.items.iter().find_map(|it| match it {
        syn::Item::Struct(s) if s.ident == "Elaborator" => Some(
            s.fields
                .iter()
                .filter(|fd| string_keyed(&fd.ty))
                .map(|fd| fd.ident.as_ref().map(|i| i.to_string()).unwrap_or_default())
                .collect(),
        ),
        _ => None,
    })
}

// ───────────────────────── (e) sim-engine re-exports ─────────────────────────

fn use_leaves(t: &syn::UseTree) -> u64 {
    match t {
        syn::UseTree::Path(p) => use_leaves(&p.tree),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => 1,
        syn::UseTree::Group(g) => g.items.iter().map(use_leaves).sum(),
    }
}

#[derive(Default)]
struct Reexports(u64);
impl<'ast> Visit<'ast> for Reexports {
    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        if matches!(u.vis, syn::Visibility::Public(_)) {
            if let syn::UseTree::Path(p) = &u.tree {
                if p.ident == "elaborate" {
                    self.0 += use_leaves(&p.tree);
                }
            }
        }
    }
}

fn scan_reexports(src: &str) -> u64 {
    let mut r = Reexports::default();
    r.visit_file(&syn::parse_file(src).expect("parse"));
    r.0
}

// ───────────────────────── measure + compare ─────────────────────────

struct Measured {
    counts: BTreeMap<String, u64>,
    /// Human-readable lists behind each count, printed on failure.
    detail: BTreeMap<String, Vec<String>>,
    unscanned: Vec<String>,
}

fn measure(root: &Path) -> Measured {
    let mut counts = BTreeMap::new();
    let mut detail: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unscanned = Vec::new();
    let variants = expr_kind_variants(root);

    // (a) every crate's src/, outside the walk module.
    let mut crates: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
        .expect("crates/")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.join("src").is_dir())
        .collect();
    crates.sort();
    for c in &crates {
        let name = c.file_name().unwrap().to_string_lossy().to_string();
        let key = format!("a.expr_kind_matchers.{name}");
        let mut files = Vec::new();
        rs_files(&c.join("src"), &mut files);
        for f in files {
            let r = rel(root, &f);
            if r == WALK_MODULE {
                continue;
            }
            let src = std::fs::read_to_string(&f).expect("read");
            let (fns, un) = scan_matchers(&src, &variants);
            unscanned.extend(un.into_iter().map(|u| format!("{r}: {u}")));
            for (fname, n) in fns {
                if n >= 3 {
                    *counts.entry(key.clone()).or_insert(0) += 1;
                    detail
                        .entry(key.clone())
                        .or_default()
                        .push(format!("{r} {fname} ({n})"));
                }
            }
        }
    }

    let elab_src = root.join("crates/elaborate/src");
    // (b)
    let mut top: Vec<PathBuf> = std::fs::read_dir(&elab_src)
        .expect("elaborate/src")
        .map(|e| e.expect("entry").path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("const_"))
                && p.extension().is_some_and(|x| x == "rs")
        })
        .collect();
    top.sort();
    let mut b = 0;
    for f in top {
        let r = rel(root, &f);
        for n in scan_option_fns(&std::fs::read_to_string(&f).expect("read")) {
            b += 1;
            detail
                .entry("b.const_option_fns".into())
                .or_default()
                .push(format!("{r} {n}"));
        }
    }
    counts.insert("b.const_option_fns".into(), b);

    // (c)
    let mut files = Vec::new();
    rs_files(&elab_src, &mut files);
    let mut c: BTreeMap<&str, u64> = CALLEES.iter().map(|n| (*n, 0)).collect();
    for f in &files {
        for (n, k) in scan_calls(&std::fs::read_to_string(f).expect("read")) {
            *c.entry(n).or_default() += k;
        }
    }
    for (n, k) in c {
        counts.insert(format!("c.calls.{n}"), k);
    }

    // (d)
    let lib = std::fs::read_to_string(elab_src.join("lib.rs")).expect("read lib.rs");
    let fields = scan_elaborator_fields(&lib).expect("struct Elaborator in elaborate/src/lib.rs");
    counts.insert("d.elaborator_string_keyed".into(), fields.len() as u64);
    detail.insert("d.elaborator_string_keyed".into(), fields);

    // (e)
    let mut files = Vec::new();
    rs_files(&root.join("crates/sim-engine/src"), &mut files);
    let e: u64 = files
        .iter()
        .map(|f| scan_reexports(&std::fs::read_to_string(f).expect("read")))
        .sum();
    counts.insert("e.sim_engine_elaborate_reexports".into(), e);

    Measured {
        counts,
        detail,
        unscanned,
    }
}

fn read_baseline(path: &Path) -> BTreeMap<String, u64> {
    let text = std::fs::read_to_string(path).expect("read baseline");
    let mut m = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("baseline line {}: expected `key = value`", i + 1));
        let v: u64 = v
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("baseline line {}: not a number", i + 1));
        assert!(
            m.insert(k.trim().to_string(), v).is_none(),
            "baseline line {}: duplicate key",
            i + 1
        );
    }
    m
}

#[test]
fn structure_does_not_regress() {
    let root = workspace_root();
    let m = measure(&root);
    assert!(
        m.unscanned.is_empty(),
        "macro bodies the scanner cannot parse name an ExprKind variant — teach the scanner \
         their syntax: {:#?}",
        m.unscanned
    );
    let base = read_baseline(&Path::new(env!("CARGO_MANIFEST_DIR")).join(BASELINE));
    let keys: BTreeSet<&String> = m.counts.keys().chain(base.keys()).collect();
    let mut report = String::new();
    for k in keys {
        let now = m.counts.get(k).copied().unwrap_or(0);
        let was = base.get(k).copied().unwrap_or(0);
        match now.cmp(&was) {
            std::cmp::Ordering::Greater => {
                let _ = writeln!(report, "{k} ROSE {was} -> {now}");
                if let Some(d) = m.detail.get(k) {
                    for line in d {
                        let _ = writeln!(report, "    {line}");
                    }
                }
            }
            std::cmp::Ordering::Less => {
                let _ = writeln!(
                    report,
                    "{k} fell {was} -> {now}: lower the baseline to {now} in {BASELINE}"
                );
            }
            std::cmp::Ordering::Equal => {}
        }
    }
    assert!(report.is_empty(), "structure ratchet:\n{report}");
}

// ───────────────────────── scanner self-tests ─────────────────────────

fn variants() -> BTreeSet<String> {
    expr_kind_variants(&workspace_root())
}

fn count_of(src: &str, f: &str) -> usize {
    let (fns, un) = scan_matchers(src, &variants());
    assert!(un.is_empty(), "unexpected unscanned: {un:?}");
    fns.iter()
        .find(|(n, _)| n.ends_with(f))
        .map(|(_, n)| *n)
        .unwrap_or_else(|| panic!("fn {f} not found in {fns:?}"))
}

/// Positive: three spellings of a pattern, three variants — counted.
#[test]
fn self_test_planted_matcher_counts() {
    let src = r#"
        use hdl_ast as ast;
        fn planted(e: &ast::Expr) -> bool {
            use ast::ExprKind as K;
            if let ast::ExprKind::Paren { .. } = &e.kind { return true; }
            match &e.kind {
                K::Unary { .. } => true,
                _ => matches!(e.kind, K::Binary { .. }),
            }
        }
    "#;
    assert_eq!(count_of(src, "planted"), 3);
    // Planted in a real crate, it raises (a): one more function at or above 3.
    let (fns, _) = scan_matchers(src, &variants());
    assert_eq!(fns.iter().filter(|(_, n)| *n >= 3).count(), 1);
}

/// Positive: a glob import and a named variant import count bare names, a unit variant
/// included; a function-local alias counts; an `impl` method is a function too.
#[test]
fn self_test_glob_and_local_alias() {
    let src = r#"
        mod m {
            use hdl_ast::ExprKind::*;
            fn g(k: &hdl_ast::ExprKind) -> u8 {
                match k { Null => 0, Dollar => 1, Ident(_) => 2, _ => 3 }
            }
        }
        use hdl_ast::ExprKind::{Concat, Replicate};
        struct S;
        impl S {
            fn h(k: &hdl_ast::ExprKind) -> bool {
                use hdl_ast::ExprKind as EK;
                matches!(k, Concat { .. } | Replicate { .. } | EK::Error)
            }
        }
    "#;
    assert_eq!(count_of(src, "g"), 3);
    assert_eq!(count_of(src, "S::h"), 3);
}

/// Negative: comments, strings, constructions, and a function-local `K` that shadows the
/// file's `ExprKind as K` with another enum, do not count.
#[test]
fn self_test_comments_strings_and_constructions_do_not_count() {
    let src = r#"
        use hdl_ast::ExprKind as K;
        // ExprKind::Paren ExprKind::Unary ExprKind::Binary
        /// `ExprKind::Concat`, `ExprKind::Replicate`, `ExprKind::Call`
        fn quiet(e: &hdl_ast::Expr) -> String {
            /* ExprKind::Ternary ExprKind::Cast ExprKind::Null */
            let _built = [
                hdl_ast::ExprKind::Null,
                hdl_ast::ExprKind::Dollar,
                hdl_ast::ExprKind::Error,
            ];
            format!("ExprKind::Paren ExprKind::Unary ExprKind::Binary {:?}", e)
        }
        fn other_k(s: &hdl_ast::Stmt) -> bool {
            use hdl_ast::Stmt as K;
            matches!(s, K::Block { .. } | K::Null(_) | K::Blocking { .. })
        }
    "#;
    assert_eq!(count_of(src, "quiet"), 0);
    assert_eq!(count_of(src, "other_k"), 0);
}

/// A macro body `syn` cannot read that names a variant is reported, not skipped.
#[test]
fn self_test_unparseable_macro_is_flagged() {
    let src = r#"
        fn odd(e: &hdl_ast::Expr) {
            my_macro!(=> hdl_ast::ExprKind::Paren { .. } ;; );
        }
    "#;
    let (_, un) = scan_matchers(src, &variants());
    assert_eq!(un.len(), 1, "{un:?}");
    let quiet = r#"fn odd() { my_macro!(=> "ExprKind::Paren" ;; ); }"#;
    assert!(scan_matchers(quiet, &variants()).1.is_empty());
}

#[test]
fn self_test_other_counters() {
    let src = r#"
        impl E {
            pub(crate) fn a(&self) -> Option<u32> { None }
            pub(crate) fn b(&self) -> u32 { 0 }
            pub fn c(&self) -> Option<u32> { None }
            fn d(&self) -> Option<u32> { None }
        }
        pub(crate) fn e() -> std::option::Option<u8> {
            // self.const_eval_in_scope(x)
            let _ = "const_self_width(x)";
            let a = self.const_eval_in_scope(x);
            let b = Self::const_self_width(y, z);
            let f = xs.iter().map(Self::const_eval_u32);
            assert!(self.const_eval_u32(q).is_some(), "{}", 1);
            None
        }
    "#;
    assert_eq!(scan_option_fns(src), vec!["a".to_string(), "e".to_string()]);
    let c = scan_calls(src);
    assert_eq!(c.get("const_eval_in_scope"), Some(&1));
    assert_eq!(c.get("const_self_width"), Some(&1));
    assert_eq!(c.get("const_eval_u32"), Some(&2));

    let lib = r#"
        struct Elaborator<'s> {
            a: BTreeMap<String, u32>,
            b: std::collections::BTreeSet<String>,
            c: BTreeMap<u32, String>,
            d: Vec<BTreeMap<String, u32>>,
            e: HashMap<String, (u32, u32)>,
        }
    "#;
    assert_eq!(
        scan_elaborator_fields(lib),
        Some(vec!["a".into(), "b".into(), "e".into()])
    );

    let reexp = r#"
        pub use elaborate::{A, B, c::D};
        use elaborate::E;
        pub(crate) use elaborate::F;
        pub use other::G;
    "#;
    assert_eq!(scan_reexports(reexp), 3);
}
