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

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};

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
}

/// A re-entry token: the scope that was current before an opener. Move-only ; consumed by
/// [`Scopes::scope_restore`] ; dropping one unrestored is a debug assertion.
#[must_use]
pub(crate) struct Saved {
    id: ScopeId,
}

/// A debug witness of the current scope at one point of a fn, checked later against the
/// current scope by [`Scopes::scope_witness_check`] (wire_ports' parent / child toggles).
#[must_use]
pub(crate) struct ScopeWitness {
    id: ScopeId,
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

impl Scopes {
    /// The empty current scope (the Elaborator's initial value).
    pub(crate) fn new() -> Scopes {
        let mut s = Scopes {
            cur_prefix: String::new(),
            cur: ScopeId(0),
            names: Vec::new(),
            ids: HashMap::default(),
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

    /// Re-point the current scope to `text` inside an open pair (its `Saved` is untouched).
    pub(crate) fn scope_repoint(&mut self, text: String) {
        self.cur = self.intern(&text);
        self.cur_prefix = text;
        self.c_path();
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
