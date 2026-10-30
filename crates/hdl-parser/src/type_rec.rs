//! §5.2 row 66: the type record. A declared type's `TypeInfo` lives in a registry
//! keyed by its declaration (`Parser::type_infos`, beside row 67b's `decl_types`;
//! neither is restored by a scope), and a type NAME's bindings live in one scoped
//! table (`Parser::type_names`): per name, one slot per kind of entry the name can
//! carry at once — a typedef, a packed struct or union layout, a per-instance struct,
//! an unpacked record and its registration number, a folding enum, the union flag.
//! Each slot keeps the rule of the name-keyed map it replaced: one name can hold
//! entries of several kinds (an outer kind survives an inner declaration of another
//! kind), and every producer, import, package twin, chained alias and scope restore
//! writes, copies or drops each slot by that map's own rule (`NameSlots`).
//!
//! What belongs to the NAME rather than to the declaration rides the slot: the
//! typedef's registration number (`TdSlot::bind`), a chained alias's dropped layout
//! key (`TdSlot::layout_reg`), a package twin's re-spelled dimensions
//! (`TdSlot::dims`) and nested member names (`PackedSlot::respelled`), the record's
//! registration number (`NameSlots::record_bind`). Every reader asks one accessor per
//! kind (`td`, `packed_layout`, `sym_layout`, `record_def`, `record_bind`,
//! `enum_def`, `is_union`), which answers what the map of that kind answered.
//!
//! Older comments in the crate still name the replaced maps: read `typedefs` as the
//! `td` slot, `struct_layouts` as `packed`, `sym_struct_layouts` as `sym`,
//! `unpacked_struct_layouts` / `unpacked_bind` as `record` / `record_bind`,
//! `enum_defs` as `enm` and `union_type_names` as `union`.

use super::*;

/// One name's type bindings: one independent slot per kind (see the module header).
#[derive(Clone, Default)]
pub(crate) struct NameSlots {
    /// The typedef the name denotes (`TypeInfo` in `type_infos`).
    pub(crate) td: Option<TdSlot>,
    /// A packed struct or union layout (`DeclType::Packed`).
    pub(crate) packed: Option<PackedSlot>,
    /// A per-instance struct layout (`DeclType::Sym`).
    pub(crate) sym: Option<DeclKey>,
    /// An unpacked record (`DeclType::Record`).
    pub(crate) record: Option<DeclKey>,
    /// §5.2 row 36: when the name was last registered as an unpacked record.
    pub(crate) record_bind: Option<u64>,
    /// A folding enum's labels (`DeclType::Enum`).
    pub(crate) enm: Option<DeclKey>,
    /// The packed layout under the name is a union overlay.
    pub(crate) union: bool,
}

/// A name's typedef slot: the declaration's `TypeInfo` key plus what the name, not
/// the declaration, holds.
#[derive(Clone)]
pub(crate) struct TdSlot {
    pub(crate) key: DeclKey,
    /// `TypeInfo::bind` — the name's newest registration (`stamp_type_bind`).
    pub(crate) bind: u64,
    /// `TypeInfo::layout_reg` — `None` where a chained alias dropped it.
    pub(crate) layout_reg: Option<u64>,
    /// A package twin's dimensions re-spelled `pkg::W` (`register_pkg_typedef_twin`).
    pub(crate) dims: Option<Box<TwinDims>>,
}

/// A name's packed layout slot: the declaration's key, and a package twin's layout
/// with its nested member names re-spelled `pkg::t` (the keys stay the declaration's).
#[derive(Clone)]
pub(crate) struct PackedSlot {
    pub(crate) key: DeclKey,
    pub(crate) respelled: Option<Box<StructLayout>>,
}

/// The three dimension containers of a `TypeInfo`, as a package twin re-spells them.
#[derive(Clone)]
pub(crate) struct TwinDims {
    pub(crate) range: Option<Range>,
    pub(crate) packed: Vec<Range>,
    pub(crate) unpacked: Vec<Dim>,
}

/// One slot's value, for the copy sites.
#[derive(Clone)]
pub(crate) enum Slot {
    Td(TdSlot),
    Packed(PackedSlot),
    Sym(DeclKey),
    Record(DeclKey),
    RecordBind(u64),
    Enm(DeclKey),
    Union,
}

/// A slot's kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Td,
    Packed,
    Sym,
    Record,
    RecordBind,
    Enm,
    Union,
}

impl Kind {
    /// The kinds an import or a package twin copies (never `Sym`: a per-instance
    /// struct is module-local and has no twin).
    pub(crate) const COPIED: [Kind; 4] = [Kind::Td, Kind::Packed, Kind::Enm, Kind::Record];
}

impl Slot {
    pub(crate) fn kind(&self) -> Kind {
        match self {
            Slot::Td(_) => Kind::Td,
            Slot::Packed(_) => Kind::Packed,
            Slot::Sym(_) => Kind::Sym,
            Slot::Record(_) => Kind::Record,
            Slot::RecordBind(_) => Kind::RecordBind,
            Slot::Enm(_) => Kind::Enm,
            Slot::Union => Kind::Union,
        }
    }
}

impl NameSlots {
    pub(crate) fn is_empty(&self) -> bool {
        self.td.is_none()
            && self.packed.is_none()
            && self.sym.is_none()
            && self.record.is_none()
            && self.record_bind.is_none()
            && self.enm.is_none()
            && !self.union
    }

    /// The slot of kind `k`, if the name has one.
    pub(crate) fn get(&self, k: Kind) -> Option<Slot> {
        match k {
            Kind::Td => self.td.clone().map(Slot::Td),
            Kind::Packed => self.packed.clone().map(Slot::Packed),
            Kind::Sym => self.sym.map(Slot::Sym),
            Kind::Record => self.record.map(Slot::Record),
            Kind::RecordBind => self.record_bind.map(Slot::RecordBind),
            Kind::Enm => self.enm.map(Slot::Enm),
            Kind::Union => self.union.then_some(Slot::Union),
        }
    }

    pub(crate) fn has(&self, k: Kind) -> bool {
        match k {
            Kind::Td => self.td.is_some(),
            Kind::Packed => self.packed.is_some(),
            Kind::Sym => self.sym.is_some(),
            Kind::Record => self.record.is_some(),
            Kind::RecordBind => self.record_bind.is_some(),
            Kind::Enm => self.enm.is_some(),
            Kind::Union => self.union,
        }
    }

    fn put(&mut self, s: Slot) {
        match s {
            Slot::Td(t) => self.td = Some(t),
            Slot::Packed(p) => self.packed = Some(p),
            Slot::Sym(k) => self.sym = Some(k),
            Slot::Record(k) => self.record = Some(k),
            Slot::RecordBind(b) => self.record_bind = Some(b),
            Slot::Enm(k) => self.enm = Some(k),
            Slot::Union => self.union = true,
        }
    }

    /// Drop the slot of kind `k`.
    fn take(&mut self, k: Kind) {
        match k {
            Kind::Td => self.td = None,
            Kind::Packed => self.packed = None,
            Kind::Sym => self.sym = None,
            Kind::Record => self.record = None,
            Kind::RecordBind => self.record_bind = None,
            Kind::Enm => self.enm = None,
            Kind::Union => self.union = false,
        }
    }
}

/// An unpacked record's entry as the accessor answers it: its members and its
/// declaration key (what `RecordDef` held).
#[derive(Clone, Copy)]
pub(crate) struct RecordRef<'a> {
    pub(crate) members: &'a Vec<StructMember>,
    pub(crate) reg: u64,
}

impl std::ops::Deref for RecordRef<'_> {
    type Target = Vec<StructMember>;
    fn deref(&self) -> &Vec<StructMember> {
        self.members
    }
}

/// A folding enum's entry as the accessor answers it: its labels and its key.
#[derive(Clone, Copy)]
pub(crate) struct EnumRef<'a> {
    pub(crate) labels: &'a Vec<(String, i64)>,
    pub(crate) reg: u64,
}

impl std::ops::Deref for EnumRef<'_> {
    type Target = Vec<(String, i64)>;
    fn deref(&self) -> &Vec<(String, i64)> {
        self.labels
    }
}

// ── composition (the accessors read through these) ──
impl Parser<'_, '_> {
    /// The `TypeInfo` a typedef slot denotes: the declaration's, with the name's
    /// registration number, layout key and (a package twin's) dimensions.
    pub(crate) fn td_of(&self, t: &TdSlot) -> Option<TypeInfo> {
        let mut info = self.type_infos.get(&t.key)?.clone();
        info.bind = t.bind;
        info.layout_reg = t.layout_reg;
        if let Some(d) = &t.dims {
            info.range = d.range.clone();
            info.packed = d.packed.clone();
            info.unpacked = d.unpacked.clone();
        }
        Some(info)
    }

    /// The layout a packed slot denotes: a twin's re-spelled copy, else the
    /// declaration's.
    pub(crate) fn packed_of<'a>(&'a self, p: &'a PackedSlot) -> Option<&'a StructLayout> {
        match &p.respelled {
            Some(l) => Some(l),
            None => self.decl_packed(p.key),
        }
    }

    pub(crate) fn record_of(&self, k: DeclKey) -> Option<RecordRef<'_>> {
        self.decl_record(k)
            .map(|members| RecordRef { members, reg: k.0 })
    }

    pub(crate) fn enum_of(&self, k: DeclKey) -> Option<EnumRef<'_>> {
        self.decl_labels(k)
            .map(|labels| EnumRef { labels, reg: k.0 })
    }

    fn slots(&self, n: &str) -> Option<&NameSlots> {
        self.type_names.get(n)
    }
}

// ── accessors: one per kind; each answers what that kind's name map answered ──
impl Parser<'_, '_> {
    /// The typedef `n` names here (the former `typedefs` map's `get(n).cloned()`).
    pub(crate) fn td(&self, n: &str) -> Option<TypeInfo> {
        self.slots(n)
            .and_then(|s| s.td.as_ref())
            .and_then(|t| self.td_of(t))
    }

    /// `n` names a typedef here (the former `typedefs` map's `contains_key`).
    pub(crate) fn has_td(&self, n: &str) -> bool {
        self.slots(n).is_some_and(|s| s.td.is_some())
    }

    /// The packed struct or union layout `n` names here (the former `struct_layouts` map).
    pub(crate) fn packed_layout(&self, n: &str) -> Option<&StructLayout> {
        self.slots(n)
            .and_then(|s| s.packed.as_ref())
            .and_then(|p| self.packed_of(p))
    }

    /// The per-instance struct layout `n` names here (the former `sym_struct_layouts`).
    pub(crate) fn sym_layout(&self, n: &str) -> Option<&SymStructLayout> {
        self.slots(n)
            .and_then(|s| s.sym)
            .and_then(|k| self.decl_sym(k))
    }

    /// The unpacked record `n` names here (the former `unpacked_struct_layouts`).
    pub(crate) fn record_def(&self, n: &str) -> Option<RecordRef<'_>> {
        self.slots(n)
            .and_then(|s| s.record)
            .and_then(|k| self.record_of(k))
    }

    /// §5.2 row 36: the record registration number of `n` (the former `unpacked_bind`).
    pub(crate) fn record_bind(&self, n: &str) -> Option<u64> {
        self.slots(n).and_then(|s| s.record_bind)
    }

    /// The folding enum `n` names here (the former `enum_defs` map).
    pub(crate) fn enum_def(&self, n: &str) -> Option<EnumRef<'_>> {
        self.slots(n)
            .and_then(|s| s.enm)
            .and_then(|k| self.enum_of(k))
    }

    /// The packed layout under `n` is a union (the former `union_type_names` set).
    pub(crate) fn is_union(&self, n: &str) -> bool {
        self.slots(n).is_some_and(|s| s.union)
    }
}

// ── writes: the producers and the copy sites ──
impl Parser<'_, '_> {
    /// The slot of kind `k` under `n`, for a copy site to copy.
    pub(crate) fn tn_slot(&self, n: &str, k: Kind) -> Option<Slot> {
        self.slots(n).and_then(|s| s.get(k))
    }

    /// Set `n`'s slot of the value's kind (a map `insert`).
    pub(crate) fn tn_set(&mut self, n: &str, s: Slot) {
        self.type_names.entry(n.to_string()).or_default().put(s);
    }

    /// Set `n`'s slot of the value's kind only where it has none (`entry().or_insert`).
    pub(crate) fn tn_fill(&mut self, n: &str, s: Slot) {
        let e = self.type_names.entry(n.to_string()).or_default();
        let wrote = !e.has(s.kind());
        if wrote {
            e.put(s);
        }
    }

    /// Drop `n`'s slot of kind `k` (a map `remove`).
    pub(crate) fn tn_clear(&mut self, n: &str, k: Kind) {
        if let Some(e) = self.type_names.get_mut(n) {
            e.take(k);
            if e.is_empty() {
                self.type_names.remove(n);
            }
        }
    }

    /// Set `n`'s slot of kind `k` to `s` (a copy of another name's slot), or drop it
    /// when `s` is `None` — the SET-or-CLEAR of a chained alias.
    pub(crate) fn tn_mirror(&mut self, n: &str, k: Kind, s: Option<Slot>) {
        match s {
            Some(s) => self.tn_set(n, s),
            None => self.tn_clear(n, k),
        }
    }

    /// §5.2 row 36: `n`'s typedef registration number (the former `typedefs[n].bind = b`).
    pub(crate) fn tn_stamp_td(&mut self, n: &str, b: u64) {
        let t = self.type_names.get_mut(n).and_then(|s| s.td.as_mut());
        if let Some(t) = t {
            t.bind = b;
        }
    }

    /// A typedef-writing producer's registration: the declaration's `TypeInfo` under
    /// a fresh key (drawn here, immediately before the name's typedef slot is set),
    /// and `n`'s typedef slot pointing at it.
    pub(crate) fn put_type_info(&mut self, n: &str, info: TypeInfo) {
        let key = DeclKey(self.next_type_reg());
        let slot = TdSlot {
            key,
            bind: info.bind,
            layout_reg: info.layout_reg,
            dims: None,
        };
        let prev = self.type_infos.insert(key, info);
        debug_assert!(prev.is_none(), "a type_infos key is drawn once");
        self.tn_set(n, Slot::Td(slot));
    }

    /// S3: a wildcard import's copy of one kind's `prefix`-keyed twins (`p::st`) to
    /// their bare names (`st`), as the import's per-name decision in `binds` says,
    /// applied to this kind's slot alone (`IfAbsent` looks at this kind's slot only).
    /// A name with no decision is not a type the package exports.
    pub(crate) fn copy_wildcard_slots(
        &mut self,
        k: Kind,
        prefix: &str,
        binds: &std::collections::BTreeMap<String, WildcardBind>,
    ) {
        let mut twins: Vec<(String, Slot)> = self
            .type_names
            .iter()
            .filter_map(|(n, s)| Some((n.strip_prefix(prefix)?.to_string(), s.get(k)?)))
            .collect();
        twins.sort_by(|a, b| a.0.cmp(&b.0));
        for (b, s) in twins {
            match binds.get(&b) {
                Some(WildcardBind::Replace) => self.tn_set(&b, s),
                Some(WildcardBind::IfAbsent) => self.tn_fill(&b, s),
                Some(WildcardBind::Skip) | None => {}
            }
        }
    }

    /// S8: the slots a unit's end keeps from the current table — of a `::` name, each
    /// slot the snapshot's entry lacks (a package twin the unit added); never the
    /// per-instance struct slot (module-local, no twin). Sorted.
    pub(crate) fn unit_kept(
        cur: &std::collections::HashMap<String, NameSlots>,
        snap: &std::collections::HashMap<String, NameSlots>,
    ) -> Vec<(String, Kind)> {
        let mut kept: Vec<(String, Kind)> = Vec::new();
        for (n, s) in cur {
            if !n.contains("::") {
                continue;
            }
            for k in [
                Kind::Td,
                Kind::Packed,
                Kind::Record,
                Kind::RecordBind,
                Kind::Enm,
                Kind::Union,
            ] {
                if s.has(k) && !snap.get(n).is_some_and(|e| e.has(k)) {
                    kept.push((n.clone(), k));
                }
            }
        }
        kept.sort_by(|a, b| (&a.0, a.1 as u8).cmp(&(&b.0, b.1 as u8)));
        kept
    }

    /// S8: the table back to the unit's snapshot, plus the `kept` slots (`unit_kept`).
    pub(crate) fn restore_type_names_unit(
        &mut self,
        snap: std::collections::HashMap<String, NameSlots>,
        kept: &[(String, Kind)],
    ) {
        let cur = std::mem::replace(&mut self.type_names, snap);
        for (n, k) in kept {
            if let Some(v) = cur.get(n).and_then(|s| s.get(*k)) {
                self.type_names.entry(n.clone()).or_default().put(v);
            }
        }
    }
}
