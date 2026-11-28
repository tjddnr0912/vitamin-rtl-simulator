//! The declaration-key stores (§5.2 Q-DECLKEY, sub-unit DK1-1): every Elaborator field keyed by a
//! declaration's dotted key is a [`Keyed`] map or a [`KeyedSet`] — the same `BTreeMap` /
//! `BTreeSet` as before, held privately, so the iteration order is the key text's order it
//! always was. No text reaches a store from outside this module: every entry takes a key type
//! or no key at all (the WINDOW below is deleted).
//!
//! THE FIRST KEY TYPE (sub-unit DK1-2a): a [`NewKey`] is the key a declaration is stored under,
//! formed by [`Scopes::new_key`] from the current scope (`fq`'s text, the same bytes) and by
//! nothing else ; its text is private. It reaches a store through the `_new` entries of
//! [`Keyed`] / [`KeyedSet`] (each the std verb on the key's own String: the same map operation),
//! leaves as message text only through [`key_show`], and meets a text operation only inside this
//! module, through [`key_text`] (`pub(super)` since DK1-5b, which closed L7: no reader outside is
//! left). No key is made from text.
//!
//! THE READ KEY (sub-unit DK1-3): an [`AtKey`] is the key a READ of a name probes, formed by
//! [`Scopes::at_cur`] (the current scope: the same bytes as [`Scopes::new_key`]'s), since DK1-4b-1 by
//! the scope walk (`walk.rs`: the candidate each scope level hands `hit`) and since DK1-5a by
//! [`Scopes::key_under`] (a carried scope) and [`key_in`] (a member under a read key) ; its constructor is lexscope-wide
//! (the `pub(super)` field), not this file's: the count of AtKey constructions per lexscope file is pinned
//! (scope_home HKC). No AtKey is formed outside the module. It reaches a
//! store only through the `_at` entries (a read: no `_new` entry takes it) and its text does not leave the
//! module (L9 `at_text`, the counted bridge, is deleted since §5.2 Q-DECLKEY DK3-a: its last reader, the
//! alias loop of an interface port, moved in as `alias_iface_port`) ; inside the module the walk MOVES the candidate its `hit`
//! accepted into the [`DeclKey`] it returns (DK1-4b-2: the text is moved, not read out). A message that
//! names what such a read would have probed takes [`Scopes::show_miss`]'s display text: no key is formed
//! for it.
//!
//! THE FOUND KEY (sub-unit DK1-4b-2): a [`DeclKey`] is what a walk returns — the candidate [`AtKey`] its
//! `hit` accepted, the walk's scratch text moved into it (no copy) ; made at the walk's three returns
//! (`walk.rs`) and, under `#[cfg(test)]`, by `tests.rs`'s helper ; the field is `pub(super)`, so its
//! constructor door is lexscope-wide (scope_home HKC pins each file's constructions). It reaches a store
//! only through the `_found` entries, its text does not leave the module ([`found_text`], L11, is
//! `pub(super)` since §5.2 Q-DECLKEY DK3-b ; the one segment test outside is [`DeclKey::has_func_segment`]),
//! and outside lexscope it meets another key only through [`key_same`], [`DeclKey::is_formed_as`] and
//! [`DeclKey::is_formed_at`].
//!
//! Each of the three key types is `#[must_use]` (§5.2 Q-DECLKEY DK2): a key formed and dropped as a
//! statement is refused by rustc's `unused_must_use` (an error under the gate's `-D warnings`) ; the
//! attribute does not reach a key inside an `Option` or a `Vec` (a walk's `Option<DeclKey>`).
//!
//! THE WINDOW (w9, DELTA14 F1) — `Deref` / `DerefMut` to the map / set each store holds, one counted
//! door, text in and text out — is DELETED (§5.2 Q-DECLKEY DK3-b): the compiler proves no text use of a
//! store is left outside this module. A store is reached through the typed entries below, the
//! whole-store ones that read no key (`has_no_key`, `same_key_count`, `clear_keys`, `snapshot`) and, inside
//! `lexscope` only, the `pub(super)` text entries (`contains_text`, `get_text`, `iter_text`,
//! `keys_text`, `range_text`, `insert_text`). `Keyed`'s `Default` is written
//! by hand (no `V: Default` bound: `Bound::new_empty` builds one over `ir::ConstVal`). Neither store type
//! is `Clone` (ZN in `mod.rs` ; §5.2 Q-DECLKEY DK3-b): a whole-store copy is the named entry
//! `snapshot`, a store, not the std map — the instance-array prepass's snapshot (`bound/undo.rs` `SideSnap`,
//! eight stores) and the non-hoistable set's fixpoint (`frames_static_init.rs`) take it ; no key text is read.

use std::collections::{BTreeMap, BTreeSet};

use super::Shown;

/// A store keyed by a declaration key: a private `BTreeMap<String, V>`.
pub(crate) struct Keyed<V>(BTreeMap<String, V>);

/// The set twin of [`Keyed`]: a private `BTreeSet<String>`.
#[derive(Default)]
pub(crate) struct KeyedSet(BTreeSet<String>);

impl<V: Clone> Keyed<V> {
    /// The whole store copied, no key read: the instance-array prepass's snapshot (`bound/undo.rs`
    /// `side_snapshot`) and the non-hoistable fixpoint (`frames_static_init.rs`). There is no `Clone`
    /// (ZN in `mod.rs`), so a `.clone()` of a store does not compile anywhere.
    #[inline(never)]
    pub(crate) fn snapshot(&self) -> Self {
        Keyed(self.0.clone())
    }
}

impl KeyedSet {
    /// The whole set copied, no key read: see [`Keyed::snapshot`].
    #[inline(never)]
    pub(crate) fn snapshot(&self) -> Self {
        KeyedSet(self.0.clone())
    }
}

impl<V> Default for Keyed<V> {
    fn default() -> Self {
        Keyed(BTreeMap::new())
    }
}

/// A declaration key formed to DECLARE a name in the current scope (DK1-2a): the current scope's
/// text, `.`, the local name — or the local name alone under the empty scope. Made only by
/// [`Scopes::new_key`] ; no `Display`, `Deref`, `Borrow`, `From` or `Into` (ZN in `mod.rs`), so a
/// NewKey's text reaches a store only through the `_new` entries (no window is left since DK3-b).
#[must_use]
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

    /// D12: the key routine `k`'s declared name is filed under in the current scope (`new_key` of the key's text).
    pub(crate) fn new_key_of_rtn(&self, k: &super::RtnKey) -> NewKey {
        self.new_key(k)
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

/// A NewKey's text where it meets a text operation inside this module (`Scopes::keys_under_cur`
/// orders and filters a package fold's saved keys by it). It was L7, the counted bridge out of the
/// module ; DK1-5b closed it (its last readers, `elaborate_package`'s six key lists, hand the
/// `&NewKey`s in), so it is `pub(super)`: no file outside `lexscope` can name it.
pub(super) fn key_text(k: &NewKey) -> &str {
    &k.0
}

/// The key a READ probes (DK1-3): the current scope's text, `.`, the local name — or the local name
/// alone under the empty scope. Made by [`Scopes::at_cur`], by the walk (`walk.rs`, DK1-4b-1: each
/// candidate it hands `hit`, one scratch re-written per scope level) and, since DK1-5a, by
/// [`Scopes::key_under`] (under a carried scope, `mod.rs`) and [`key_in`] (a member under a read key) ;
/// the field is `pub(super)`, so the
/// constructor door is the whole of `lexscope`, not this file (scope_home HKC pins the constructions per
/// lexscope file). No `Clone`, `Display`, `Debug`, `Deref`, `Borrow`, `From` or `Into` (ZN in `mod.rs`),
/// so its text reaches a store only through the `_at` entries (no window since DK3-b) ; no `_new` entry takes it.
#[must_use]
pub(crate) struct AtKey(pub(super) String);

/// The key of `member` inside the read key `k` (`<k>.<member>`), typed (DK1-5a: ports.rs
/// elaborate_virtual_iface's member glue of a bound instance ; L9's two rows there closed).
/// It always forms `<k>.<member>`: there is no empty-key guard (an empty `k` would give
/// `.<member>`, not `<member>` as `at_cur` gives under the empty scope). Its one caller,
/// `elaborate_virtual_iface`, hands the read key of a bound instance's path, never empty ;
/// a second caller is not pinned by a call table yet (NOT-PROMISED until one names it).
pub(crate) fn key_in(k: &AtKey, member: &str) -> AtKey {
    AtKey(format!("{}.{}", k.0, member))
}

/// A read's key against a read's key (DK1-4b-1, P27: the walk's candidate against a refused clocking item's
/// alias, held.rs `hit_clk_refused` since §5.2 Q-DECLKEY DK2): the same text.
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
/// [`found_text`] (inside lexscope) and the fns handed only a found key ; no `_new` or `_at` entry takes it.
#[must_use]
pub(crate) struct DeclKey(pub(super) String);

/// A found key's text where it meets a text operation inside this module (`alias_iface_port`'s scope
/// prefix glue, `key_spells_indexed_singleton`'s segments, `walk_hits_decl_pos`, `gen_binding_encloses`).
/// It was L11, the counted bridge out of the module ; §5.2 Q-DECLKEY DK3-b closed it (its last readers
/// outside, frames_static_init.rs's segment test and gen_scope_name.rs's `key_spells_indexed_singleton`,
/// became [`DeclKey::has_func_segment`] and an entry of `hier.rs`), so it is `pub(super)`.
#[inline]
pub(super) fn found_text(k: &DeclKey) -> &str {
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

/// The whole-store entries that read, form and drop no key (§5.2 Q-DECLKEY DK3-b, q8): the std
/// `is_empty` / `len` / `clear` on the private map / set, under names of their own (no std token: the
/// token census of the stores' entries stays exact).
impl<V> Keyed<V> {
    #[inline]
    pub(crate) fn has_no_key(&self) -> bool {
        self.0.is_empty()
    }
}

/// The whole-store entries of a set store (q8, as [`Keyed::has_no_key`]): `has_no_key` / `same_key_count` /
/// `clear_keys` (frames_static_init.rs's non-hoistable fixpoint, `modport_readonly`, `param_type_guessed`).
impl KeyedSet {
    #[inline]
    pub(crate) fn has_no_key(&self) -> bool {
        self.0.is_empty()
    }
    /// True when this set holds as many keys as `other` (the fixpoint's growth test: `extend_found`
    /// only adds, so an equal count is no new key) ; a bool, as I-SURF wants of an OUT position.
    #[inline]
    pub(crate) fn same_key_count(&self, other: &KeyedSet) -> bool {
        self.0.len() == other.0.len()
    }
    #[inline]
    pub(crate) fn clear_keys(&mut self) {
        self.0.clear()
    }
}

/// The text entries of a map store, for this module's own text probes (§5.2 Q-DECLKEY DK3-b: the
/// window's uses inside `lexscope` — `hier.rs`, `elab.rs`, `disp.rs`): `pub(super)`, so no file
/// outside the module can name one ; each is the std verb on the private map.
impl<V> Keyed<V> {
    #[inline]
    pub(super) fn contains_text(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }
    #[inline]
    pub(super) fn iter_text(&self) -> std::collections::btree_map::Iter<'_, String, V> {
        self.0.iter()
    }
    #[inline]
    pub(super) fn keys_text(&self) -> std::collections::btree_map::Keys<'_, String, V> {
        self.0.keys()
    }
    #[inline]
    pub(super) fn range_text(
        &self,
        from: std::ops::RangeFrom<String>,
    ) -> std::collections::btree_map::Range<'_, String, V> {
        self.0.range(from)
    }
    #[inline]
    pub(super) fn insert_text(&mut self, k: String, v: V) -> Option<V> {
        self.0.insert(k, v)
    }
}

/// The text entries of a set store (as [`Keyed::contains_text`]: this module's own probes, `pub(super)`).
impl KeyedSet {
    #[inline]
    pub(super) fn contains_text(&self, k: &str) -> bool {
        self.0.contains(k)
    }
    #[inline]
    pub(super) fn insert_text(&mut self, k: String) -> bool {
        self.0.insert(k)
    }
}

/// `hier_resolve`'s leaf read (§5.2 Q-DECLKEY DK3-b: its `table` is a store, `&Keyed<V>`): the std
/// `get` on the private map, `pub(super)` as the text entries above.
impl<V> Keyed<V> {
    #[inline]
    pub(super) fn get_text(&self, k: &str) -> Option<&V> {
        self.0.get(k)
    }
}

/// A found key's segment test (§5.2 Q-DECLKEY DK3-b: frames_static_init.rs's frame-local test, L11's
/// row there closed): is one segment of the key a function frame's (`$func$…`)?
impl DeclKey {
    #[inline]
    pub(crate) fn has_func_segment(&self) -> bool {
        self.0.split('.').any(|seg| seg.starts_with("$func$"))
    }
}
