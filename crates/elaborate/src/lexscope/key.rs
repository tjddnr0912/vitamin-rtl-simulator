//! The declaration-key stores (§5.2 Q-DECLKEY, sub-unit DK1-1): every Elaborator field keyed by a
//! declaration's dotted key is a [`Keyed`] map or a [`KeyedSet`] — the same `BTreeMap` /
//! `BTreeSet` as before, held privately, so the iteration order is the key text's order it
//! always was. A key that is not yet typed is text, and text reaches a store only through the
//! WINDOW below.
//!
//! THE FIRST KEY TYPE (sub-unit DK1-2a): a [`NewKey`] is the key a declaration is stored under,
//! formed by [`Scopes::new_key`] from the current scope (`fq`'s text, the same bytes) and by
//! nothing else ; its text is private. It reaches a store through the `_new` entries of
//! [`Keyed`] / [`KeyedSet`] (each the std verb on the key's own String: the same map operation),
//! leaves as message text only through [`key_show`], and meets a text operation only through
//! [`key_text`] (L7, a counted bridge deleted at DK3-b). No key is made from text.
//!
//! THE READ KEY (sub-unit DK1-3): an [`AtKey`] is the key a READ of a name probes, formed by
//! [`Scopes::at_cur`] (the current scope: the same bytes as [`Scopes::new_key`]'s) and, since DK1-4b-1, by
//! the scope walk (`walk.rs`: the candidate each scope level hands `hit`) ; its constructor is lexscope-wide
//! (the `pub(super)` field), not this file's: the count of AtKey constructions per lexscope file is pinned
//! (scope_home HKC). No AtKey is formed outside the module. It reaches a
//! store only through the `_at` entries (a read: no `_new` entry takes it) and its text leaves the module
//! only through [`at_text`] (L9, a counted bridge) ; inside the module the walk MOVES the candidate its `hit`
//! accepted into the [`DeclKey`] it returns (DK1-4b-2: the text is moved, not read out). A message that
//! names what such a read would have probed takes [`Scopes::show_miss`]'s display text: no key is formed
//! for it.
//!
//! THE FOUND KEY (sub-unit DK1-4b-2): a [`DeclKey`] is what a walk returns — the candidate [`AtKey`] its
//! `hit` accepted, the walk's scratch text moved into it (no copy) ; made at the walk's three returns
//! (`walk.rs`) and, under `#[cfg(test)]`, by `tests.rs`'s helper ; the field is `pub(super)`, so its
//! constructor door is lexscope-wide (scope_home HKC pins each file's constructions). It reaches a store
//! only through the `_found` entries, its text leaves the module only through [`found_text`] (L11, a
//! counted bridge), and it meets another key only through [`key_same`], [`DeclKey::is_formed_as`] and
//! [`DeclKey::is_formed_at`].
//!
//! THE WINDOW (w9, DELTA14 F1): `Deref` / `DerefMut` to the map / set each store holds, so every
//! existing use compiles as it was written (one counted door, text in and text out). It is
//! deleted at DK3-b, where the compiler proves no text use is left. `Keyed`'s `Default` is written
//! by hand (no `V: Default` bound: `Bound::new_empty` builds one over `ir::ConstVal`) ; there is no
//! `Clone`: a `.clone()` of a store reaches the std map's through the window and yields the std
//! map, which the snapshot code (`bound/undo.rs`, `frames_static_init.rs`) relies on.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};

use super::Shown;

/// A store keyed by a declaration key: a private `BTreeMap<String, V>`.
pub(crate) struct Keyed<V>(BTreeMap<String, V>);

/// The set twin of [`Keyed`]: a private `BTreeSet<String>`.
#[derive(Default)]
pub(crate) struct KeyedSet(BTreeSet<String>);

impl<V> Default for Keyed<V> {
    fn default() -> Self {
        Keyed(BTreeMap::new())
    }
}

impl<V> Deref for Keyed<V> {
    type Target = BTreeMap<String, V>;
    fn deref(&self) -> &BTreeMap<String, V> {
        &self.0
    }
}

impl<V> DerefMut for Keyed<V> {
    fn deref_mut(&mut self) -> &mut BTreeMap<String, V> {
        &mut self.0
    }
}

impl Deref for KeyedSet {
    type Target = BTreeSet<String>;
    fn deref(&self) -> &BTreeSet<String> {
        &self.0
    }
}

impl DerefMut for KeyedSet {
    fn deref_mut(&mut self) -> &mut BTreeSet<String> {
        &mut self.0
    }
}

/// A declaration key formed to DECLARE a name in the current scope (DK1-2a): the current scope's
/// text, `.`, the local name — or the local name alone under the empty scope. Made only by
/// [`Scopes::new_key`] ; no `Display`, `Deref`, `Borrow`, `From` or `Into` (ZN in `mod.rs`), so a
/// NewKey cannot take the window's text path into a store.
#[derive(Clone)]
pub(crate) struct NewKey(String);

impl super::Scopes {
    /// The key `local` is declared under in the current scope: `fq`'s text, typed.
    pub(crate) fn new_key(&self, local: &str) -> NewKey {
        if self.cur_prefix.is_empty() {
            NewKey(local.to_string())
        } else {
            NewKey(format!("{}.{}", self.cur_prefix, local))
        }
    }

    /// The key a read of `local` in the current scope probes: `fq`'s text, typed (DK1-3).
    pub(crate) fn at_cur(&self, local: &str) -> AtKey {
        if self.cur_prefix.is_empty() {
            AtKey(local.to_string())
        } else {
            AtKey(format!("{}.{}", self.cur_prefix, local))
        }
    }

    /// `local` as a message names it in the current scope (the key a read of it probes, or would
    /// have probed): `fq`'s text as display text ; no key is formed (DK1-3).
    pub(crate) fn show_miss(&self, local: &str) -> Shown {
        if self.cur_prefix.is_empty() {
            Shown(local.to_string())
        } else {
            Shown(format!("{}.{}", self.cur_prefix, local))
        }
    }
}

/// A NewKey in a message: the key's bytes, as display text (it leaves through `scope_out`).
/// (DK1-2b widened it to bridge w11's keys ; DK1-2d closed the bridge.)
#[inline]
pub(crate) fn key_show(k: &NewKey) -> Shown {
    Shown(k.0.clone())
}

/// L7 (LEAVE): a NewKey's text where it meets a text operation outside the module ; a counted
/// bridge, deleted at DK3-b.
pub(crate) fn key_text(k: &NewKey) -> &str {
    &k.0
}

/// The key a READ probes (DK1-3): the current scope's text, `.`, the local name — or the local name
/// alone under the empty scope. Made by [`Scopes::at_cur`] and by the walk (`walk.rs`, DK1-4b-1: each
/// candidate it hands `hit`, one scratch re-written per scope level) ; the field is `pub(super)`, so the
/// constructor door is the whole of `lexscope`, not this file (scope_home HKC pins the constructions per
/// lexscope file). No `Clone`, `Display`, `Debug`, `Deref`, `Borrow`, `From` or `Into` (ZN in `mod.rs`),
/// so it cannot take the window's text path into a store, and no `_new` entry takes it.
pub(crate) struct AtKey(pub(super) String);

/// L9 (LEAVE): an AtKey's text where it meets a text operation outside the module (a key glued from it:
/// ports.rs, DK1-5 / DK3-a ; held.rs's compare with a walk's found key left at DK1-4b-2 for
/// [`DeclKey::is_formed_at`]) ; a counted bridge.
pub(crate) fn at_text(k: &AtKey) -> &str {
    &k.0
}

/// A read's key against a read's key (DK1-4b-1, P27: held.rs clk_refused_at's walk candidate against a refused
/// clocking item's alias): the same text.
#[inline]
pub(crate) fn at_same(a: &AtKey, b: &AtKey) -> bool {
    a.0 == b.0
}

/// The key a walk FOUND (DK1-4b-2): the candidate [`AtKey`] its `hit` accepted, the text moved, not
/// copied. Made at the walk's three returns (`walk.rs`: `scoped_key_at_or_above`,
/// `walk_scopes_key_inner`, `walk_out_from`) and, under `#[cfg(test)]`, by `tests.rs`'s helper ; the
/// field is `pub(super)`, so no file outside `lexscope` can make one (scope_home HKC pins the
/// constructions per lexscope file). No `Clone`, `Display`, `Debug`, `Deref`, `Borrow`, `From` or
/// `Into` (ZN in `mod.rs`), and no `PartialEq`: it is taken by the `_found` entries
/// (`Keyed::get_found` / `contains_found` / `index_found`, `KeyedSet::contains_found` /
/// `extend_found`), the compares ([`key_same`], [`DeclKey::is_formed_as`], [`DeclKey::is_formed_at`]),
/// [`found_text`] (L11) and the fns handed only a found key ; no `_new` or `_at` entry takes it.
pub(crate) struct DeclKey(pub(super) String);

/// L11 (LEAVE): a found key's text where it meets a text operation outside the module (a key's
/// segments read: frames_static_init.rs, gen_scope_name.rs, DK3-b ; a scope prefix glued from it:
/// ports.rs, DK3-a) ; a counted bridge.
#[inline]
pub(crate) fn found_text(k: &DeclKey) -> &str {
    &k.0
}

/// The NewKey entries of a map store: the std verb on the key's own String (`entry_new` is
/// `entry(..).or_default()`, the one form a caller writes: no std `Entry` leaves the module).
/// (DK1-2b widened the first four to bridge w11's keys ; DK1-2d closed the bridge: a NewKey only.)
impl<V> Keyed<V> {
    #[inline]
    pub(crate) fn insert_new(&mut self, k: NewKey, v: V) -> Option<V> {
        self.0.insert(k.0, v)
    }
    #[inline]
    pub(crate) fn remove_new(&mut self, k: &NewKey) -> Option<V> {
        self.0.remove(&k.0)
    }
    #[inline]
    pub(crate) fn contains_new(&self, k: &NewKey) -> bool {
        self.0.contains_key(&k.0)
    }
    #[inline]
    pub(crate) fn get_new(&self, k: &NewKey) -> Option<&V> {
        self.0.get(&k.0)
    }
    pub(crate) fn entry_new(&mut self, k: NewKey) -> &mut V
    where
        V: Default,
    {
        self.0.entry(k.0).or_default()
    }
}

/// The AtKey entries of a map store (DK1-3): the std read on the key's own String.
impl<V> Keyed<V> {
    #[inline]
    pub(crate) fn get_at(&self, k: &AtKey) -> Option<&V> {
        self.0.get(&k.0)
    }
    #[inline]
    pub(crate) fn contains_at(&self, k: &AtKey) -> bool {
        self.0.contains_key(&k.0)
    }
}

/// A found key against a found key (DK1-4b-2, P27: edge_gate.rs binding_encloses): the same text.
#[inline]
pub(crate) fn key_same(a: &DeclKey, b: &DeclKey) -> bool {
    a.0 == b.0
}

/// The compares of a found key with a formed one (DK1-4b-2, P27): the same text.
impl DeclKey {
    /// with the key a name was declared under (struct_arm.rs struct_arm_rhs: `pattern_targets`' NewKey).
    #[inline]
    pub(crate) fn is_formed_as(&self, k: &NewKey) -> bool {
        self.0 == k.0
    }
    /// with the key a read formed (held.rs clk_refused_at: a refused clocking item's alias).
    #[inline]
    pub(crate) fn is_formed_at(&self, k: &AtKey) -> bool {
        self.0 == k.0
    }
}

/// The found-key entries of a map store (DK1-4b-2): the std read on the key's own String
/// (`index_found` is the std index, its panic included).
impl<V> Keyed<V> {
    #[inline]
    pub(crate) fn get_found(&self, k: &DeclKey) -> Option<&V> {
        self.0.get(&k.0)
    }
    #[inline]
    pub(crate) fn contains_found(&self, k: &DeclKey) -> bool {
        self.0.contains_key(&k.0)
    }
    #[inline]
    pub(crate) fn index_found(&self, k: &DeclKey) -> &V {
        &self.0[&k.0]
    }
}

/// The NewKey entries of a set store (`remove_new` is the funnel's clear of `param_type_guessed`).
impl KeyedSet {
    #[inline]
    pub(crate) fn insert_new(&mut self, k: NewKey) -> bool {
        self.0.insert(k.0)
    }
    #[inline]
    pub(crate) fn remove_new(&mut self, k: &NewKey) -> bool {
        self.0.remove(&k.0)
    }
}

/// The AtKey entry of a set store (DK1-3: `implicit_nets`, `vif_handles`).
impl KeyedSet {
    #[inline]
    pub(crate) fn contains_at(&self, k: &AtKey) -> bool {
        self.0.contains(&k.0)
    }
}

/// The found-key entries of a set store (DK1-4b-2: frames_static_init.rs's sets of a frame's
/// resolved locals ; `extend_found` is the std `extend`, each key's String moved in).
impl KeyedSet {
    #[inline]
    pub(crate) fn contains_found(&self, k: &DeclKey) -> bool {
        self.0.contains(&k.0)
    }
    #[inline]
    pub(crate) fn extend_found(&mut self, ks: Vec<DeclKey>) {
        self.0.extend(ks.into_iter().map(|k| k.0));
    }
}
