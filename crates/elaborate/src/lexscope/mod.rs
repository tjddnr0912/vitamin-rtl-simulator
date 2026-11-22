//! The current scope (§5.2 Q-LEXSCOPE, unit LS2a): the dotted path of the scope being
//! lowered, owned here and nowhere else. The Elaborator holds one [`Scopes`] ; code outside
//! this module moves the current scope only through the named entries below (no entry takes a
//! scope argument), and reads its text only through the bridge [`Scopes::cur_prefix_text`]
//! (LEAVE L1, a counted bridge until LS3 / DK1).
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

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};

mod disp;
mod elab;
#[cfg(test)]
mod tests;

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
}

/// A re-entry token: the scope that was current before an opener. Move-only ; consumed by
/// [`Scopes::scope_restore`] ; dropping one unrestored is a debug assertion.
#[must_use]
pub(crate) struct Saved {
    id: ScopeId,
}

/// A carried scope (unit LS3): a scope held outside the module — a deferred record's lowering
/// scope, a parameter that names one. Clone and an opaque Debug, nothing else: it is re-entered
/// with [`Scopes::scope_swap`] / [`Scopes::scope_repoint`], and its text leaves only through
/// [`Scopes::scope_text`] (LEAVE L6, closing at DK1) and [`Scopes::key_under`] (LEAVE L5).
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
/// path): opaque. It leaves as plain text only through [`scope_out`].
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
        };
        s.cur = s.intern("");
        s.c_path();
        s
    }

    /// The bridge (LEAVE L1): the current scope's text, for the readers that still read text
    /// (counted by HBR ; falls at LS3, 0 at DK1).
    pub(crate) fn cur_prefix_text(&self) -> &str {
        &self.cur_prefix
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

    /// LEAVE L6 (closes DK1): a carried scope's text, read by the three key-forming bodies that
    /// stay outside the module until DK1 (hier_resolve, scoped_key_at_or_above,
    /// walk_scopes_key_from). The interner's id -> text read: no allocation, no probe.
    pub(crate) fn scope_text(&self, at: &ScopeRef) -> &str {
        &self.names[at.0 .0 as usize]
    }

    /// LEAVE L5 (closes DK1): the key `name` forms under the carried scope `at` (the empty
    /// scope's key is the name itself).
    pub(crate) fn key_under(&self, at: &ScopeRef, name: &str) -> String {
        let prefix = &self.names[at.0 .0 as usize];
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", prefix, name)
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
