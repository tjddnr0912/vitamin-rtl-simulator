//! §5.2 row 67b: declaration keys. The type name table (`type_rec.rs`: a name's
//! packed, per-instance, record and enum slots) is keyed by type NAME and scoped: a
//! routine, a block, a generate block, a class or an import that declares a name
//! again puts another declaration under it. A variable bound to a type, and a packed
//! struct member of a nested struct type, therefore store the KEY of the declaration
//! they bound — a number one declaration owns for the whole parse
//! (`Parser::next_type_reg`, never restored) — and every access reads that
//! declaration's layout from `decl_types`, which no scope restore touches. No access
//! looks the type name up again where it is used.
//!
//! Each slot holds the declaration's key (`StructLayout::reg` and
//! `SymStructLayout::reg` carry it too), so a copy — a chained alias, an import, a
//! package twin, a scope restore — carries the declaration's key with it. A binding
//! site takes the key from the slots standing under the type name where it binds, in
//! the order the member reads take them (`var_struct_ref`: packed struct or union,
//! then packable record, then per-instance struct; a record or an enum binding takes
//! the record or enum slot), so in the declaring scope every read answers as before;
//! which entry the name SHOULD denote where two kinds stand under it is a by-name
//! question this key does not change (ROADMAP §2 struct-binding-by-name).

use super::*;

/// One type declaration's identity (`Parser::next_type_reg`; never 0).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DeclKey(pub(crate) u64);

/// What a variable binding or a nested member stores for its type: the declaration's
/// key, and the name it was bound by (the stored spelling — a bare `t` or a package
/// `p::t` — which names, never resolves: the `$enum_name$<name>` stem of an enum
/// method, the nested member key a package twin re-spells).
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct TyRef {
    pub(crate) name: String,
    pub(crate) key: DeclKey,
}

/// A declaration's layout as it was declared, by kind.
pub(crate) enum DeclType {
    /// A packed struct, or a packed union (`union`), laid out at parse time.
    Packed { layout: StructLayout, union: bool },
    /// A packed struct laid out per instance (§3 ⑤ ⓒ).
    Sym(SymStructLayout),
    /// An unpacked struct (record): its members.
    Record(Vec<StructMember>),
    /// An enum whose labels fold at parse time.
    Enum(Vec<(String, i64)>),
}

impl Parser<'_, '_> {
    /// Record a fresh declaration's layout under its key. Called once per key, where
    /// the key is minted.
    pub(crate) fn register_decl(&mut self, reg: u64, ty: DeclType) {
        self.decl_types.insert(DeclKey(reg), ty);
    }

    /// The packed struct or union layout `k` declared; `None` for any other kind.
    pub(crate) fn decl_packed(&self, k: DeclKey) -> Option<&StructLayout> {
        match self.decl_types.get(&k)? {
            DeclType::Packed { layout, .. } => Some(layout),
            _ => None,
        }
    }

    /// `k` declared a packed union.
    pub(crate) fn decl_is_union(&self, k: DeclKey) -> bool {
        matches!(
            self.decl_types.get(&k),
            Some(DeclType::Packed { union: true, .. })
        )
    }

    /// The per-instance layout `k` declared.
    pub(crate) fn decl_sym(&self, k: DeclKey) -> Option<&SymStructLayout> {
        match self.decl_types.get(&k)? {
            DeclType::Sym(l) => Some(l),
            _ => None,
        }
    }

    /// The members of the unpacked record `k` declared.
    pub(crate) fn decl_record(&self, k: DeclKey) -> Option<&Vec<StructMember>> {
        match self.decl_types.get(&k)? {
            DeclType::Record(m) => Some(m),
            _ => None,
        }
    }

    /// The labels of the enum `k` declared.
    pub(crate) fn decl_labels(&self, k: DeclKey) -> Option<&Vec<(String, i64)>> {
        match self.decl_types.get(&k)? {
            DeclType::Enum(l) => Some(l),
            _ => None,
        }
    }

    /// The flat layout a member select or a `'{…}` reads through a `var_struct`
    /// binding: a packed struct or union as declared, or a packable record's packed
    /// layout (`packable_record_layout`). `None` for a per-instance struct (its own
    /// selects read `decl_sym`) and for a record that is not packable.
    pub(crate) fn decl_struct_layout(&self, k: DeclKey) -> Option<StructLayout> {
        match self.decl_types.get(&k)? {
            DeclType::Packed { layout, .. } => Some(layout.clone()),
            DeclType::Record(members) => self.packable_members_layout(members),
            _ => None,
        }
    }

    /// The binding of a packed struct or union type `name` as this scope reads it.
    pub(crate) fn packed_ref(&self, name: &str) -> Option<TyRef> {
        self.packed_layout(name).map(|l| TyRef {
            name: name.to_string(),
            key: DeclKey(l.reg),
        })
    }

    /// The binding of a per-instance struct type `name`.
    pub(crate) fn sym_ref(&self, name: &str) -> Option<TyRef> {
        self.sym_layout(name).map(|l| TyRef {
            name: name.to_string(),
            key: DeclKey(l.reg),
        })
    }

    /// The binding of an unpacked record type `name`.
    pub(crate) fn record_ref(&self, name: &str) -> Option<TyRef> {
        self.record_def(name).map(|r| TyRef {
            name: name.to_string(),
            key: DeclKey(r.reg),
        })
    }

    /// The binding of an enum type `name`.
    pub(crate) fn enum_ref(&self, name: &str) -> Option<TyRef> {
        self.enum_def(name).map(|e| TyRef {
            name: name.to_string(),
            key: DeclKey(e.reg),
        })
    }

    /// The binding a `var_struct` entry stores for type `name`: the declaration a
    /// member select or a `'{…}` through the variable read where the variable was
    /// declared — a packed struct or union, else a packable record, else a per-instance
    /// struct (`struct_field_select`, then `sym_struct_field_select`). Where the scope
    /// holds more than one entry of the name (an outer type of another kind left
    /// under it), this is the entry those reads took at the binding site; which entry
    /// the NAME should denote there is a by-name question this key does not change
    /// (ROADMAP §2 struct-binding-by-name).
    pub(crate) fn var_struct_ref(&self, name: &str) -> Option<TyRef> {
        if let Some(t) = self.packed_ref(name) {
            return Some(t);
        }
        if self.packable_record_layout(name).is_some() {
            if let Some(t) = self.record_ref(name) {
                return Some(t);
            }
        }
        self.sym_ref(name)
    }
}
