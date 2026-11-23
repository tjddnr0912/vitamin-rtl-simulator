//! The declaration-key stores (§5.2 Q-DECLKEY, sub-unit DK1-1): every Elaborator field keyed by a
//! declaration's dotted key is a [`Keyed`] map or a [`KeyedSet`] — the same `BTreeMap` /
//! `BTreeSet` as before, held privately, so the iteration order is the key text's order it
//! always was. No key type exists yet: a key is still text, and text reaches a store only
//! through the WINDOW below.
//!
//! THE WINDOW (w9, DELTA14 F1): `Deref` / `DerefMut` to the map / set each store holds, so every
//! existing use compiles as it was written (one counted door, text in and text out). It is
//! deleted at DK3-b, where the compiler proves no text use is left. `Keyed`'s `Default` is written
//! by hand (no `V: Default` bound: `Bound::new_empty` builds one over `ir::ConstVal`) ; there is no
//! `Clone`: a `.clone()` of a store reaches the std map's through the window and yields the std
//! map, which the snapshot code (`bound/undo.rs`, `frames_static_init.rs`) relies on.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};

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
