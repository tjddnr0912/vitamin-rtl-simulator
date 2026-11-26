//! The current scope (§5.2 Q-LEXSCOPE, unit LS2a): the dotted path of the scope being
//! lowered, owned here and nowhere else. The Elaborator holds one [`Scopes`] ; code outside
//! this module moves the current scope only through the named entries below (no entry takes a
//! scope argument), and reads no text of it (the bridge [`Scopes::cur_prefix_text`], LEAVE L1, is
//! private since DK1-5b).
//!
//! The representation is the same in every profile: the text `cur_prefix` (the legacy String,
//! written by the legacy rules moved here), the id `cur` of that text in a private interner
//! (one probe per scope entry), and `Saved` / [`ScopeWitness`] carry the id. Debug builds
//! check two things with `debug_assert*!` and nothing else: C-INTERN (the interner is a
//! bijection) and C-PATH (after every entry the id's text is `cur_prefix`).
//!
//! The display chain (unit LS2-d) lives here too: the named-block label chain, the root it is
//! spelled under and the class method being lowered are private fields of [`Scopes`], saved
//! only as the move-only [`SavedChain`] / [`SavedRoot`] / [`SavedMethod`] ; display text made
//! from a scope is a [`Shown`] / [`ShownCtx`], opaque, and leaves as plain data only through
//! [`scope_out`] (the door's one text exit, its sinks counted by the home tests).
//!
//! The questions code outside asks about a scope's kind or place (unit LS4: is it a package's,
//! a block-local one, under a function or inlined task, the frame of one of these routines,
//! named by a self-path) are named entries returning `bool` over the same text ; none returns
//! text, and their call sites are counted by the home tests (HC_SCOPEQ).
//!
//! The outward walk of a name (DK1-4a) is here too, in `walk.rs`: the generic walks
//! (`walk_scopes_key` and its forms, private to `walk.rs` since §5.2 Q-DECLKEY DK2), reading the
//! scope texts as this module's fields, and the named entries every scoped lookup calls, one per
//! hit set ; since DK1-4b the walks hand `hit` an [`AtKey`] and return a [`DeclKey`]. The hierarchical
//! resolver is here too (DK1-5a), in `hier.rs`: `hier_resolve`, two of the generate-spelling
//! helpers it asks and its scope question `is_hier_scope`.

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, DefaultHasher};

use crate::edge_gate::span_holds;

mod disp;
mod elab;
mod hier;
mod key;
#[cfg(test)]
mod tests;
mod walk;

use key::key_text;
pub(crate) use key::{
    at_same, found_text, key_in, key_same, key_show, AtKey, DeclKey, Keyed, KeyedSet, NewKey,
};

/// An interned scope text. Private to the module: no id leaves it.
#[derive(Clone)]
struct ScopeId(u32);

/// The current scope and the interner of every scope text entered so far.
pub(crate) struct Scopes {
    cur_prefix: String,
    cur: ScopeId,
    names: Vec<String>,
    /// The text -> id half of the interner. A hash map, not a `BTreeMap`: it is probed
    /// once per scope entry and never iterated, so no order of it can reach output ; the
    /// hasher has fixed keys, so a probe does the same work on every run.
    ids: HashMap<String, ScopeId, BuildHasherDefault<DefaultHasher>>,
    // §4.5.426: the named-block label chain in effect while lowering statements
    // (pushed by the `Stmt::Block`/`Stmt::Fork` arms for a user label; replaced by
    // `[task]` while an inline task body lowers; empty in a frame body), and the two
    // sidecars it records into (`push_stmt` for a SysTask, the `$sformatf` lowering).
    disp_chain: Vec<String>,
    /// §4.5.435: the DECLARING scope the chain is rooted at while a subroutine body
    /// lowers — the module instance (display spelling) for an inlined task, the
    /// frame's own `<inst>.<name>` for a frame body; `None` = the process's own
    /// scope (`display_prefix`). A recorded chain is absolute (`scope_chain_abs`).
    disp_root: Option<String>,
    /// §4.5.437: `<class>.<method>` while a class method body lowers — the
    /// diagnostic context (`record_stmt_loc`) of a statement inside it, stored with
    /// a leading `.` so the engine prefixes the calling process's scope.
    disp_method: Option<String>,
    /// §4.5.435: the path of the module INSTANCE being elaborated — the current scope
    /// without the generate-scope segments `with_scope` appends inside it. A subroutine is
    /// declared by the module, so `%m` in its body names this (IEEE §21.2.1), whatever
    /// generate scope or frame chain called it (mark [`Mark::Inst`] ; LS3b).
    mark_inst: ScopeId,
    /// R19-X1: the scope in force when `func_table` / `task_table` were collected — the
    /// scope every function / task in them is DECLARED in (mark [`Mark::TfDecl`]). Read
    /// only by `default_binding_matches_decl_scope` (IEEE 1800 §13.5.4).
    mark_tf_decl: ScopeId,
    /// The instance scope `decl_pos` describes (mark [`Mark::DeclPos`]): the use-before-
    /// declaration check fires only when a name resolved to THAT scope's binding.
    mark_decl_pos: ScopeId,
    /// The generate scopes the walk has open, innermost last: each one's scope, the source
    /// range of the construct that opened it (the `if` / `case` / `for` / block), and for a
    /// generate-for iteration the genvar key it binds — which sits at the ENCLOSING
    /// scope's key, shadowing a same-named constant there. Read only by the §11.5.1
    /// zero-width claim ([`Self::gen_binding_encloses`]), to tell whether a name binds
    /// where the select's own text does.
    gen_text: Vec<(ScopeId, hdl_ast::Span, Option<NewKey>)>,
}

/// One of the three scalar marks of [`Scopes`] (LS3b): a scope remembered beside the current
/// one, written and read only through the `mark_*` entries.
pub(crate) enum Mark {
    /// The module instance being elaborated (`inst_prefix`).
    Inst,
    /// The scope the routine tables were collected in (`tf_decl_scope`).
    TfDecl,
    /// The scope `decl_pos` describes (`decl_pos_scope`).
    DeclPos,
}

/// A store keyed by a scope (LS3b): the map is private and ordered by the scope's TEXT (a
/// `BTreeMap<String, V>`), so an order it is drained in is the text order it always was ;
/// no method returns or takes a key text but [`Self::entry_under_cur`]'s instance name and
/// the drained keys as [`Shown`].
pub(crate) struct ScopeKeyed<V>(BTreeMap<String, V>);

/// A re-entry token: the scope that was current before an opener. Move-only ; consumed by
/// [`Scopes::scope_restore`] ; dropping one unrestored is a debug assertion.
#[must_use]
pub(crate) struct Saved {
    id: ScopeId,
}

/// A carried scope (unit LS3): a scope held outside the module — a deferred record's lowering
/// scope, a parameter that names one. Clone and an opaque Debug, nothing else: it is re-entered
/// with [`Scopes::scope_swap`] / [`Scopes::scope_repoint`] ; its text does not leave the module
/// (DK1-5a: [`Scopes::scope_text`] is private, [`Scopes::key_under`] returns an [`AtKey`]).
pub(crate) struct ScopeRef(ScopeId);

/// The set of `(span, scope)` pairs [`crate::Elaborator::report_at`] has reported: opaque,
/// made empty by `Default`, read and written only by that entry.
#[derive(Default)]
pub(crate) struct ReportedAt(std::collections::BTreeSet<(u32, u32, String)>);

/// A debug witness of the current scope at one point of a fn, checked later against the
/// current scope by [`Scopes::scope_witness_check`] (wire_ports' parent / child toggles).
#[must_use]
pub(crate) struct ScopeWitness {
    id: ScopeId,
}

/// Display / output text made from a scope (a `%m` path, an `[in …]` label, a frame's
/// path): opaque. It leaves as plain text only through [`scope_out`]. `Default` is the
/// empty text (a profile row's fail-safe default, `ProcIdentRow`).
#[derive(Default)]
pub(crate) struct Shown(String);

/// The diagnostic context of "here" (the `[in …]` frames): opaque. It leaves only through
/// [`scope_out`].
pub(crate) struct ShownCtx(Vec<diag::Frame>);

/// A saved display label chain: move-only, restored by [`Scopes::disp_restore_chain`].
#[must_use]
pub(crate) struct SavedChain(Vec<String>);

/// A saved display root: move-only, restored by [`Scopes::disp_restore_root`].
#[must_use]
pub(crate) struct SavedRoot(Option<String>);

/// A saved class-method display text: move-only, restored by
/// [`Scopes::disp_restore_method`].
#[must_use]
pub(crate) struct SavedMethod(Option<String>);

/// The boundary: a value whose plain form leaves the module through [`scope_out`] (two
/// impls: `Shown` -> `String`, `ShownCtx` -> `Vec<diag::Frame>`).
pub(crate) trait Outgoing {
    type Plain;
    fn plain(self) -> Self::Plain;
}

impl Outgoing for Shown {
    type Plain = String;
    fn plain(self) -> String {
        self.0
    }
}

impl Outgoing for ShownCtx {
    type Plain = Vec<diag::Frame>;
    fn plain(self) -> Vec<diag::Frame> {
        self.0
    }
}

/// THE one way text made from a scope leaves as plain data (a sidecar, a diagnostic's
/// context, a `%m` byte string): it consumes the opaque value.
pub(crate) fn scope_out<T: Outgoing>(t: T) -> T::Plain {
    t.plain()
}

/// The one text -> `Shown` entry: a display text the caller spells itself (a class
/// method's `<class>.<method>`), made from no scope.
pub(crate) fn shown_from_spelling(s: String) -> Shown {
    Shown(s)
}

/// The shims' wrappers (private): a display text, a context, a saved root.
fn wrap_shown(s: String) -> Shown {
    Shown(s)
}

fn wrap_ctx(v: Vec<diag::Frame>) -> ShownCtx {
    ShownCtx(v)
}

fn wrap_root(r: Option<String>) -> SavedRoot {
    SavedRoot(r)
}

impl Clone for Shown {
    fn clone(&self) -> Self {
        Shown(self.0.clone())
    }
}

impl Clone for ShownCtx {
    fn clone(&self) -> Self {
        ShownCtx(self.0.clone())
    }
}

impl std::fmt::Debug for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Shown(..)")
    }
}

impl std::fmt::Debug for ShownCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShownCtx(..)")
    }
}

impl Clone for ScopeRef {
    fn clone(&self) -> Self {
        ScopeRef(self.0.clone())
    }
}

impl std::fmt::Debug for ScopeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopeRef(..)")
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        debug_assert!(
            std::thread::panicking(),
            "a lexscope `Saved` was dropped without `scope_restore`"
        );
    }
}

/// ZN: asserts at compile time that a type does NOT implement any of the listed traits: with
/// only the `()` impl the `_` below is inferred ; when the type has one of the traits a second
/// impl applies and the inference is ambiguous, so the build fails at that `zn!` line.
macro_rules! zn {
    ($t:ty: $($tr:path),+ $(,)?) => {
        const _: fn() = || {
            trait AmbiguousIfImpl<A> {
                fn some_item() {}
            }
            impl<T: ?Sized> AmbiguousIfImpl<()> for T {}
            $({
                #[allow(dead_code)]
                struct Invalid;
                impl<T: ?Sized + $tr> AmbiguousIfImpl<Invalid> for T {}
            })+
            let _ = <$t as AmbiguousIfImpl<_>>::some_item;
        };
    };
}

zn!(Scopes: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default);
zn!(Saved: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default);
zn!(ScopeWitness: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default);
zn!(SavedChain: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default);
zn!(SavedRoot: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default);
zn!(SavedMethod: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default);
zn!(Shown: PartialEq, Eq, PartialOrd, Ord, core::hash::Hash, core::fmt::Display, Copy,
    AsRef<str>, core::borrow::Borrow<str>, core::ops::Deref<Target = str>, Into<String>);
zn!(ShownCtx: PartialEq, Eq, PartialOrd, Ord, core::hash::Hash, core::fmt::Display, Copy, Default,
    IntoIterator);
zn!(ScopeRef: Copy, PartialEq, Eq, PartialOrd, Ord, core::hash::Hash, core::fmt::Display, Default,
    AsRef<str>, core::borrow::Borrow<str>, core::ops::Deref<Target = str>, Into<String>);
zn!(ReportedAt: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug);
zn!(ScopeKeyed<()>: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, Default,
    IntoIterator);
zn!(Keyed<()>: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, IntoIterator);
zn!(KeyedSet: Clone, Copy, PartialEq, PartialOrd, core::hash::Hash, core::fmt::Debug, IntoIterator);
zn!(NewKey: Copy, PartialEq, Eq, PartialOrd, Ord, core::hash::Hash, core::fmt::Display, core::fmt::Debug,
    Default, AsRef<str>, core::borrow::Borrow<str>, core::borrow::Borrow<String>, core::ops::Deref<Target = str>,
    Into<String>, From<String>);
zn!(AtKey: Clone, Copy, PartialEq, Eq, PartialOrd, Ord, core::hash::Hash, core::fmt::Display, core::fmt::Debug,
    Default, AsRef<str>, core::borrow::Borrow<str>, core::borrow::Borrow<String>, core::ops::Deref<Target = str>,
    Into<String>, From<String>);
zn!(DeclKey: Clone, Copy, PartialEq, Eq, PartialOrd, Ord, core::hash::Hash, core::fmt::Display, core::fmt::Debug,
    Default, AsRef<str>, core::borrow::Borrow<str>, core::borrow::Borrow<String>, core::ops::Deref<Target = str>,
    Into<String>, From<String>);

impl Scopes {
    /// The empty current scope (the Elaborator's initial value).
    pub(crate) fn new() -> Scopes {
        let mut s = Scopes {
            cur_prefix: String::new(),
            cur: ScopeId(0),
            names: Vec::new(),
            ids: HashMap::default(),
            disp_chain: Vec::new(),
            disp_root: None,
            disp_method: None,
            mark_inst: ScopeId(0),
            mark_tf_decl: ScopeId(0),
            mark_decl_pos: ScopeId(0),
            gen_text: Vec::new(),
        };
        // The empty text is id 0: the three marks start at the empty scope.
        s.cur = s.intern("");
        s.c_path();
        s
    }

    /// The current scope's text, read inside this module only (LEAVE L1 closed at DK1-5b: no
    /// reader is left outside).
    fn cur_prefix_text(&self) -> &str {
        &self.cur_prefix
    }

    /// The keys of `keys` formed under the current scope, each once, in text order: a package
    /// fold's saved keys (DK1-5b: `decl_keep_snapshot`'s one scope question). Sorted, deduplicated
    /// and filtered in the caller's Vec: no second collection and no `<prefix>.` text formed.
    pub(crate) fn keys_under_cur<'k>(&self, mut keys: Vec<&'k NewKey>) -> Vec<&'k NewKey> {
        let prefix = self.cur_prefix_text();
        let mine = |k: &NewKey| {
            let k = key_text(k);
            k.len() > prefix.len() && k.starts_with(prefix) && k.as_bytes()[prefix.len()] == b'.'
        };
        keys.sort_unstable_by(|a, b| key_text(a).cmp(key_text(b)));
        keys.dedup_by(|a, b| key_text(a) == key_text(b));
        debug_assert!(
            keys.iter().all(|k| mine(k)),
            "a package fold saved a key outside its own prefix {prefix}"
        );
        keys.retain(|k| mine(k));
        keys
    }

    /// Opener: make `text` the current scope ; the scope that was current is returned.
    pub(crate) fn scope_swap_text(&mut self, text: String) -> Saved {
        let id = self.intern(&text);
        let saved = Saved {
            id: std::mem::replace(&mut self.cur, id),
        };
        self.cur_prefix = text;
        self.c_path();
        saved
    }

    /// Opener: make the empty path the current scope (the `mem::take` form).
    pub(crate) fn scope_swap_empty(&mut self) -> Saved {
        self.scope_swap_text(String::new())
    }

    /// Opener: enter the package scope `$pkg$<pkg>`.
    pub(crate) fn scope_swap_pkg(&mut self, pkg: &str) -> Saved {
        self.scope_swap_text(format!("$pkg${pkg}"))
    }

    /// Opener: enter the child segment `seg` of the current scope (with_scope's rule: a
    /// segment of the empty path is the segment itself, else `<cur>.<seg>`). The child text
    /// is formed in the current text's own buffer, which the restore overwrites in place: no
    /// String per entry (the buffer grows only past its longest text so far ; a text new to
    /// the interner is copied into it).
    fn scope_enter(&mut self, seg: &str) -> Saved {
        let mut new_prefix = std::mem::take(&mut self.cur_prefix);
        if !new_prefix.is_empty() {
            new_prefix.push('.');
        }
        new_prefix.push_str(seg);
        self.scope_swap_text(new_prefix)
    }

    /// Opener that does not move: the current scope, saved for a later restore (a loop that
    /// re-points once per item and restores once after it).
    pub(crate) fn scope_hold(&self) -> Saved {
        Saved {
            id: self.cur.clone(),
        }
    }

    /// Re-point the current scope to the carried scope `at` inside an open pair (its `Saved`
    /// is untouched). No interner probe: the id is `at`'s, its text written in place.
    pub(crate) fn scope_repoint(&mut self, at: &ScopeRef) {
        self.cur = at.0.clone();
        self.cur_prefix.clone_from(&self.names[self.cur.0 as usize]);
        self.c_path();
    }

    /// The current scope as a carried value (an id copy, no text).
    pub(crate) fn scope_carry(&self) -> ScopeRef {
        ScopeRef(self.cur.clone())
    }

    /// Opener: re-enter the carried scope `at` ; the scope that was current is returned. No
    /// interner probe: the id is `at`'s, its text written in place as [`Self::scope_restore`]
    /// writes it.
    pub(crate) fn scope_swap(&mut self, at: &ScopeRef) -> Saved {
        let saved = Saved {
            id: std::mem::replace(&mut self.cur, at.0.clone()),
        };
        self.cur_prefix.clone_from(&self.names[self.cur.0 as usize]);
        self.c_path();
        saved
    }

    /// A carried scope's text, read inside this module only (LEAVE L6 closed at DK1-5a: its last reader
    /// outside, hier_resolve, moved into this module's `hier.rs`). The interner's id -> text read: no
    /// allocation, no probe.
    fn scope_text(&self, at: &ScopeRef) -> &str {
        &self.names[at.0 .0 as usize]
    }

    /// The key a read of `name` probes under the carried scope `at` (the empty scope's key is the
    /// name itself), typed (DK1-5a: LEAVE L5 closed).
    pub(crate) fn key_under(&self, at: &ScopeRef, name: &str) -> AtKey {
        let prefix = &self.names[at.0 .0 as usize];
        if prefix.is_empty() {
            AtKey(name.to_string())
        } else {
            AtKey(format!("{}.{}", prefix, name))
        }
    }

    /// Closer: the scope `saved` was taken from becomes current again.
    pub(crate) fn scope_restore(&mut self, saved: Saved) {
        let id = saved.id.clone();
        std::mem::forget(saved);
        self.cur_prefix.clone_from(&self.names[id.0 as usize]);
        self.cur = id;
        self.c_path();
    }

    /// A witness of the current scope, for a later [`Self::scope_witness_check`].
    pub(crate) fn scope_witness(&self) -> ScopeWitness {
        ScopeWitness {
            id: self.cur.clone(),
        }
    }

    /// Debug check: the current scope is the one `w` witnessed.
    pub(crate) fn scope_witness_check(&self, w: &ScopeWitness) {
        debug_assert_eq!(
            self.cur.0, w.id.0,
            "the current scope moved since its witness was taken"
        );
    }

    /// The display chain is saved and emptied (a frame / class-method body is its own `%m`
    /// scope).
    pub(crate) fn disp_chain_take(&mut self) -> SavedChain {
        SavedChain(std::mem::take(&mut self.disp_chain))
    }

    /// The display chain is saved and becomes the one label `seg` (an inlined task body).
    pub(crate) fn disp_chain_swap_single(&mut self, seg: &str) -> SavedChain {
        SavedChain(std::mem::replace(
            &mut self.disp_chain,
            vec![seg.to_string()],
        ))
    }

    /// A user block label joins the display chain.
    pub(crate) fn disp_chain_push(&mut self, label: String) {
        self.disp_chain.push(label);
    }

    /// The innermost label leaves the display chain.
    pub(crate) fn disp_chain_pop(&mut self) {
        self.disp_chain.pop();
    }

    /// Whether no block label is in the display chain.
    pub(crate) fn disp_chain_is_empty(&self) -> bool {
        self.disp_chain.is_empty()
    }

    /// Closer: the display chain `saved` was taken from is in effect again.
    pub(crate) fn disp_restore_chain(&mut self, saved: SavedChain) {
        self.disp_chain = saved.0;
    }

    /// The display root is saved and becomes the empty root (a class method body).
    pub(crate) fn disp_root_swap_empty(&mut self) -> SavedRoot {
        SavedRoot(self.disp_root.replace(String::new()))
    }

    /// Closer: the display root `saved` was taken from is in effect again.
    pub(crate) fn disp_restore_root(&mut self, saved: SavedRoot) {
        self.disp_root = saved.0;
    }

    /// The class-method display text is saved and becomes `method` (`<class>.<method>`).
    pub(crate) fn disp_class_method_swap(&mut self, method: String) -> SavedMethod {
        SavedMethod(self.disp_method.replace(method))
    }

    /// Whether no class method body is being lowered.
    pub(crate) fn class_method_is_none(&self) -> bool {
        self.disp_method.is_none()
    }

    /// Closer: the class-method display text `saved` was taken from is in effect again.
    pub(crate) fn disp_restore_method(&mut self, saved: SavedMethod) {
        self.disp_method = saved.0;
    }

    /// The child scope `inst_name` of the current scope (a child instance's path),
    /// interned: `child_prefix`'s text.
    pub(crate) fn scope_child(&mut self, inst_name: &str) -> ScopeRef {
        let text = self.child_prefix(inst_name);
        ScopeRef(self.intern(&text))
    }

    /// Child prefix = current prefix + child instance name.
    fn child_prefix(&self, inst_name: &str) -> String {
        if self.cur_prefix.is_empty() {
            inst_name.to_string()
        } else {
            format!("{}.{}", self.cur_prefix, inst_name)
        }
    }

    /// A carried scope as display / output text (its raw path, no display spelling).
    pub(crate) fn scope_shown(&self, at: &ScopeRef) -> Shown {
        Shown(self.names[at.0 .0 as usize].clone())
    }

    /// The current scope as display / output text (its raw path, no display spelling: a
    /// profile row's instance scope, `ProcIdentRow`).
    pub(crate) fn cur_shown(&self) -> Shown {
        Shown(self.cur_prefix.clone())
    }

    /// The current scope for a message: its raw path in backquotes, or `none` when the
    /// current scope is the empty path.
    pub(crate) fn cur_shown_quoted_or(&self, none: &str) -> Shown {
        if self.cur_prefix.is_empty() {
            Shown(none.to_string())
        } else {
            Shown(format!("`{}`", self.cur_prefix))
        }
    }

    /// The slot of mark `m`.
    fn mark_slot(&mut self, m: Mark) -> &mut ScopeId {
        match m {
            Mark::Inst => &mut self.mark_inst,
            Mark::TfDecl => &mut self.mark_tf_decl,
            Mark::DeclPos => &mut self.mark_decl_pos,
        }
    }

    /// The text of mark `m` (read inside the module only).
    fn mark_text(&self, m: Mark) -> &str {
        let id = match m {
            Mark::Inst => &self.mark_inst,
            Mark::TfDecl => &self.mark_tf_decl,
            Mark::DeclPos => &self.mark_decl_pos,
        };
        &self.names[id.0 as usize]
    }

    /// Mark `m` becomes the carried scope `at` ; the scope it held is returned.
    pub(crate) fn mark_swap(&mut self, m: Mark, at: &ScopeRef) -> ScopeRef {
        ScopeRef(std::mem::replace(self.mark_slot(m), at.0.clone()))
    }

    /// Mark `m` becomes the current scope ; the scope it held is returned.
    pub(crate) fn mark_swap_here(&mut self, m: Mark) -> ScopeRef {
        let here = self.cur.clone();
        ScopeRef(std::mem::replace(self.mark_slot(m), here))
    }

    /// Mark `m` becomes `at` (a restore of what a swap returned).
    pub(crate) fn mark_put(&mut self, m: Mark, at: ScopeRef) {
        *self.mark_slot(m) = at.0;
    }

    /// Mark `m` and `at` exchange their scopes.
    pub(crate) fn mark_exchange(&mut self, m: Mark, at: &mut ScopeRef) {
        std::mem::swap(self.mark_slot(m), &mut at.0);
    }

    /// Opener: re-enter the scope mark `m` holds ; the scope that was current is returned.
    /// No interner probe (the mark's id, its text written in place).
    pub(crate) fn swap_to_mark(&mut self, m: Mark) -> Saved {
        let at = ScopeRef(match m {
            Mark::Inst => self.mark_inst.clone(),
            Mark::TfDecl => self.mark_tf_decl.clone(),
            Mark::DeclPos => self.mark_decl_pos.clone(),
        });
        self.scope_swap(&at)
    }

    /// Is the current scope the module instance's own — not a generate block's or a
    /// routine's below it (and not the empty scope)?
    pub(crate) fn cur_is_inst_top(&self) -> bool {
        let inst = self.mark_text(Mark::Inst);
        !inst.is_empty() && self.cur_prefix == inst
    }

    /// Is the current scope the one the routine tables were collected in?
    pub(crate) fn cur_is_tf_decl(&self) -> bool {
        self.cur_prefix == self.mark_text(Mark::TfDecl)
    }

    /// Is the current scope the module instance's own, or under it only through
    /// SYNTHESIZED segments (`$func$…`, `$blk$…`, a class method's: each one starting with
    /// `$`)? (`hier_leaf_scope`'s guard, `expr_size_hier.rs`.)
    pub(crate) fn cur_is_inst_top_or_synth_below(&self) -> bool {
        let Some(rest) = self.cur_prefix.strip_prefix(self.mark_text(Mark::Inst)) else {
            return false;
        };
        if rest.is_empty() {
            return true;
        }
        let Some(rest) = rest.strip_prefix('.') else {
            return false;
        };
        rest.split('.').all(|s| s.starts_with('$'))
    }

    /// Is the carried scope `at` a package's (a `$pkg$…` path: the package pre-sweep)?
    /// (`hier_resolve`'s guard, this module's `hier.rs` since DK1-5a.)
    pub(crate) fn scope_is_pkg(&self, at: &ScopeRef) -> bool {
        self.names[at.0 .0 as usize].starts_with("$pkg$")
    }

    /// Is the current scope inside a function's or an inlined task's body (a `$func$…` or
    /// `$itask$…` segment anywhere in its path)? (`struct_arm_context`, `struct_arm.rs`.)
    pub(crate) fn cur_under_func_or_itask(&self) -> bool {
        self.cur_prefix
            .split('.')
            .any(|s| s.starts_with("$func$") || s.starts_with("$itask$"))
    }

    /// Is the active prefix a block-local `$blk$<lo>` scope? That is exactly the shape
    /// `flush_block_local_inits` claims, so it is also the test for "this scope's t0 writes
    /// are replayed by that flush, not by the enclosing scope's".
    pub(crate) fn in_block_local_scope(&self) -> bool {
        self.cur_prefix
            .rsplit('.')
            .next()
            .is_some_and(|seg| seg.starts_with("$blk$"))
    }

    /// Is the routine whose frame encloses the current scope (the trailing `$func$<name>` /
    /// `$itask$<name>` segment of its path) one of the call targets `fids` names?
    /// (`emit_dyn_formal_markers_nested`'s recursion refusal, `frames_call/emit.rs`.)
    pub(crate) fn cur_frame_routine_is_any(&self, fids: &[(u32, String)]) -> bool {
        let enclosing = self.cur_prefix.rsplit('.').next().and_then(|s| {
            s.strip_prefix("$func$")
                .or_else(|| s.strip_prefix("$itask$"))
        });
        if let Some(encl) = enclosing {
            if fids.iter().any(|(_, n)| n == encl) {
                return true;
            }
        }
        false
    }

    /// Does the hierarchical path `segs` name something in the CURRENT scope (a self-path
    /// like `t.o` / `t.blk.o`), rather than reaching into a child instance?
    ///
    /// Only a self-path can alias a bare block-local: v1 flattens block-locals to module
    /// nets by bare name, so `t.o` and `o` are one net, while `sub.o` is a different module's
    /// net and cannot be. (`hoist/special.rs` read_roots, `hoist/general.rs` order_walk.)
    pub(crate) fn hier_path_is_self(&self, segs: &[hdl_ast::Ident]) -> bool {
        let Some(head) = segs.first() else {
            return false;
        };
        self.cur_prefix
            .split('.')
            .map(|c| c.split('[').next().unwrap_or(c))
            .any(|c| c == head.name)
    }

    /// A generate scope opens: the current scope, the source range `text` of the construct
    /// that opened it, and the genvar key a generate-for iteration binds.
    pub(crate) fn gen_text_push(&mut self, text: hdl_ast::Span, gv_key: Option<NewKey>) {
        self.gen_text.push((self.cur.clone(), text, gv_key));
    }

    /// The innermost open generate scope closes.
    pub(crate) fn gen_text_pop(&mut self) {
        self.gen_text.pop();
    }

    /// §11.5.1 zero-width claim: does the binding `key` (the innermost binding of `name`)
    /// lie where the source position `at` does — the genvar of an open generate-for whose
    /// construct holds `at`, the module instance's own scope, or an open generate scope
    /// whose construct holds `at`?
    pub(crate) fn gen_binding_encloses(
        &self,
        key: &DeclKey,
        name: &str,
        at: hdl_ast::Span,
    ) -> bool {
        let key = found_text(key);
        let scopes = &self.gen_text;
        if let Some((_, text, _)) = scopes
            .iter()
            .rev()
            .find(|s| s.2.as_ref().map(key_text) == Some(key))
        {
            return span_holds(*text, at);
        }
        let scope = key
            .strip_suffix(name)
            .map(|s| s.strip_suffix('.').unwrap_or(s))
            .unwrap_or_default();
        if scope == self.mark_text(Mark::Inst) {
            return true;
        }
        scopes
            .iter()
            .rev()
            .find(|s| self.names[s.0 .0 as usize] == scope)
            .is_some_and(|(_, text, _)| span_holds(*text, at))
    }

    /// The id of `text`, interned on first sight (ids are never freed).
    fn intern(&mut self, text: &str) -> ScopeId {
        let id = match self.ids.get(text) {
            Some(id) => id.clone(),
            None => {
                let id = ScopeId(self.names.len() as u32);
                self.names.push(text.to_string());
                self.ids.insert(text.to_string(), id.clone());
                id
            }
        };
        debug_assert_eq!(self.names.len(), self.ids.len(), "C-INTERN: map and vector");
        debug_assert_eq!(self.names[id.0 as usize], text, "C-INTERN: text of the id");
        id
    }

    /// C-PATH: the current id's text is the current text.
    fn c_path(&self) {
        debug_assert_eq!(
            self.names[self.cur.0 as usize], self.cur_prefix,
            "C-PATH: the current id and the current text"
        );
    }
}

impl<V> ScopeKeyed<V> {
    /// The empty store.
    pub(crate) fn new() -> ScopeKeyed<V> {
        ScopeKeyed(BTreeMap::new())
    }

    /// The value stored at the carried scope `at`, taken out.
    pub(crate) fn take_at(&mut self, s: &Scopes, at: &ScopeRef) -> Option<V> {
        self.0.remove(&s.names[at.0 .0 as usize])
    }

    /// `v` is stored at the carried scope `at`.
    pub(crate) fn put_at(&mut self, s: &Scopes, at: &ScopeRef, v: V) {
        self.0.insert(s.names[at.0 .0 as usize].clone(), v);
    }

    /// The slot of the instance `name` declared in the current scope, made empty on first
    /// use. The key is `<cur>.<name>` UNCONDITIONALLY (not `scope_child`'s text, which differs
    /// when the current scope is empty): a defparam's target, printed as it was keyed when
    /// it matches no instance.
    pub(crate) fn entry_under_cur(&mut self, s: &Scopes, name: &str) -> &mut V
    where
        V: Default,
    {
        self.0
            .entry(format!("{}.{}", s.cur_prefix, name))
            .or_default()
    }

    /// Every scope still holding a value, in text order, as display text ; the store is
    /// left empty.
    pub(crate) fn drain_keys_shown(&mut self) -> Vec<Shown> {
        let keys = self.0.keys().cloned().map(Shown).collect();
        self.0.clear();
        keys
    }
}

impl<T> ScopeKeyed<Vec<T>> {
    /// `item` joins the list stored at the current scope (made empty on first use).
    pub(crate) fn push_here(&mut self, s: &Scopes, item: T) {
        self.0.entry(s.cur_prefix.clone()).or_default().push(item);
    }
}
