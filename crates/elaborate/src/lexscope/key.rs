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
}

/// A NewKey in a message: the key's bytes, as display text (it leaves through `scope_out`).
pub(crate) fn key_show(k: &NewKey) -> Shown {
    Shown(k.0.clone())
}

/// L7 (LEAVE): a NewKey's text where it meets a text operation outside the module ; a counted
/// bridge, deleted at DK3-b.
pub(crate) fn key_text(k: &NewKey) -> &str {
    &k.0
}

/// The NewKey entries of a map store: the std verb on the key's own String (`entry_new` is
/// `entry(..).or_default()`, the one form a caller writes: no std `Entry` leaves the module).
impl<V> Keyed<V> {
    pub(crate) fn insert_new(&mut self, k: NewKey, v: V) -> Option<V> {
        self.0.insert(k.0, v)
    }
    pub(crate) fn remove_new(&mut self, k: &NewKey) -> Option<V> {
        self.0.remove(&k.0)
    }
    pub(crate) fn contains_new(&self, k: &NewKey) -> bool {
        self.0.contains_key(&k.0)
    }
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

/// The NewKey entry of a set store.
impl KeyedSet {
    pub(crate) fn insert_new(&mut self, k: NewKey) -> bool {
        self.0.insert(k.0)
    }
}
