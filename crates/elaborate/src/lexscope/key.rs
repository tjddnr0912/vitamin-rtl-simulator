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
/// DK1-2b: any [`SlotKey`] (the funnel's debug message on its generic key) ; `&NewKey` again at
/// DK1-2d.
pub(crate) fn key_show(k: &impl SlotKey) -> Shown {
    Shown(k.slot_text().to_string())
}

/// L7 (LEAVE): a NewKey's text where it meets a text operation outside the module ; a counted
/// bridge, deleted at DK3-b.
pub(crate) fn key_text(k: &NewKey) -> &str {
    &k.0
}

/// The NewKey entries of a map store: the std verb on the key's own String (`entry_new` is
/// `entry(..).or_default()`, the one form a caller writes: no std `Entry` leaves the module).
/// DK1-2b: `insert_new` / `remove_new` / `contains_new` / `get_new` take any [`SlotKey`] (bridge
/// w11: a NewKey, or the text a funnel caller still holds) ; NewKey again at DK1-2d.
impl<V> Keyed<V> {
    pub(crate) fn insert_new(&mut self, k: impl SlotKey, v: V) -> Option<V> {
        self.0.insert(k.slot_owned(), v)
    }
    pub(crate) fn remove_new(&mut self, k: &impl SlotKey) -> Option<V> {
        self.0.remove(k.slot_text())
    }
    pub(crate) fn contains_new(&self, k: &impl SlotKey) -> bool {
        self.0.contains_key(k.slot_text())
    }
    pub(crate) fn get_new(&self, k: &impl SlotKey) -> Option<&V> {
        self.0.get(k.slot_text())
    }
    pub(crate) fn entry_new(&mut self, k: NewKey) -> &mut V
    where
        V: Default,
    {
        self.0.entry(k.0).or_default()
    }
}

/// The NewKey entries of a set store (DK1-2b: any [`SlotKey`], as the map's ; `remove_new` is the
/// funnel's clear of `param_type_guessed`).
impl KeyedSet {
    pub(crate) fn insert_new(&mut self, k: impl SlotKey) -> bool {
        self.0.insert(k.slot_owned())
    }
    pub(crate) fn remove_new(&mut self, k: &impl SlotKey) -> bool {
        self.0.remove(k.slot_text())
    }
}

/// BRIDGE w11 (§5.2 Q-DECLKEY DK1-2b, DELTA15 C1.3, R-G8 (a) ; deleting unit DK1-2d): the key the
/// Bound write funnel (`bound/write.rs`, `bound/undo.rs`) and the `_new` entries above take for
/// these commits — a [`NewKey`], or the TEXT a funnel caller still holds (DK1-2c's mints), handed
/// to the store as the window w9 hands it today: no NewKey is made from text (P23). Sealed: its
/// half `slot_seal::SlotText` lives in a module private to this file, so no impl outside it
/// compiles ; its two text methods are called in this file only (a home row counts their call
/// tokens elsewhere at 0: R-G8 (e), NP11-1). `Clone` (the funnel's `bind_decl` /
/// `bind_param_range` clone a key where `to_string` stood), so `str` is no SlotKey ; the blanket
/// `&T` takes `&NewKey` and `&String` with no call-site edit (premise11 N11-1).
pub(crate) trait SlotKey: slot_seal::SlotText + Clone {}

mod slot_seal {
    /// The sealed half of [`super::SlotKey`]: the key's text, handed to a store.
    pub trait SlotText {
        fn slot_text(&self) -> &str;
        fn slot_owned(self) -> String;
    }
}

impl slot_seal::SlotText for NewKey {
    fn slot_text(&self) -> &str {
        &self.0
    }
    fn slot_owned(self) -> String {
        self.0
    }
}
impl SlotKey for NewKey {}

impl<T: SlotKey> slot_seal::SlotText for &T {
    fn slot_text(&self) -> &str {
        (**self).slot_text()
    }
    fn slot_owned(self) -> String {
        self.slot_text().to_string()
    }
}
impl<T: SlotKey> SlotKey for &T {}

// TEXT (dated, deleting unit DK1-2d): the key a funnel caller still holds as a String.
impl slot_seal::SlotText for String {
    fn slot_text(&self) -> &str {
        self
    }
    fn slot_owned(self) -> String {
        self
    }
}
impl SlotKey for String {}
