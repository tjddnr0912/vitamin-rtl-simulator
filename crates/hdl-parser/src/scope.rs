//! scope snapshots — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Resolve an optional signing qualifier to the EFFECTIVE signedness using the
    /// declared kind's default (atom types `byte`/`shortint`/`int`/`longint`/
    /// `integer` default SIGNED; everything else defaults unsigned).
    pub(crate) fn signed_eff(&mut self, kind: Option<NetVarKind>) -> bool {
        self.opt_signed()
            .unwrap_or_else(|| atom_default_signed(kind))
    }
    /// Snapshot the lexically-scoped registries before a block's first body-local
    /// typedef / struct-or-enum-typed var, so they can be restored to give that
    /// block its own scope.
    pub(crate) fn snapshot_scope(&self) -> ScopeSnapshot {
        ScopeSnapshot {
            typedefs: self.typedefs.clone(),
            struct_layouts: self.struct_layouts.clone(),
            unpacked_struct_layouts: self.unpacked_struct_layouts.clone(),
            enum_defs: self.enum_defs.clone(),
            union_type_names: self.union_type_names.clone(),
            var_struct: self.var_struct.clone(),
            var_unpacked_struct: self.var_unpacked_struct.clone(),
            record_array_vars: self.record_array_vars.clone(),
            record_soa_vars: self.record_soa_vars.clone(),
            var_enum: self.var_enum.clone(),
            struct_scalar_vars: self.struct_scalar_vars.clone(),
            struct_1d_array_vars: self.struct_1d_array_vars.clone(),
            packed_md_params: self.packed_md_params.clone(),
            wildcard_bound: self.wildcard_bound.clone(),
            local_decl_names: self.local_decl_names.clone(),
        }
    }

    /// Restore the registries to a prior snapshot, dropping any block-local
    /// typedefs / struct-var bindings added since (so they do not leak out of or
    /// clobber an outer scope).
    /// A block-local PLAIN declaration (`logic [7:0] P;`) that shadows an outer
    /// multi-dim-packed PARAMETER name must shadow it for the BLOCK only:
    /// `parse_net_var` unbinds the name, and without a snapshot the outer binding
    /// never came back (measured: a module `localparam logic [3:0][4:0] P` read as
    /// its flat bit 1 after an `initial begin : b logic [7:0] P; … end` — silent).
    /// The snapshot is taken tentatively (only when some binding exists) and kept
    /// only when a declared name was actually bound — a design with no such
    /// bindings, or no shadowing decl, is byte-identical. Cold and boxed: the
    /// caller sits on the frame-budgeted `block_body` recursion.
    /// Deliberately NOT extended to `var_struct`/`var_enum`: a block-local decl
    /// flattens to a MODULE net of the same name (the v1 block-local model), so
    /// restoring a struct VARIABLE binding made a post-block `SS.g` read the local
    /// net through the package struct's layout (review B2: loud E3010 → silent `1`
    /// where both oracles read the package's `5`); a parameter is not a net, so it
    /// does not collide. That flatten collision is pre-existing (ROADMAP §2).
    #[inline(never)]
    pub(crate) fn parse_block_plain_decl(
        &mut self,
        scope: &mut Option<Box<ScopeSnapshot>>,
    ) -> Option<NetVarDecl> {
        let tentative = if scope.is_none() && !self.packed_md_params.is_empty() {
            Some(Box::new(self.snapshot_scope()))
        } else {
            None
        };
        let d = self.parse_net_var(false);
        if let (Some(t), Some(d)) = (tentative, &d) {
            let shadows = d
                .names
                .iter()
                .any(|n| t.packed_md_params.contains_key(&n.name.name));
            if shadows {
                *scope = Some(t);
            }
        }
        d
    }

    pub(crate) fn restore_scope(&mut self, s: ScopeSnapshot) {
        self.typedefs = s.typedefs;
        self.struct_layouts = s.struct_layouts;
        self.unpacked_struct_layouts = s.unpacked_struct_layouts;
        self.enum_defs = s.enum_defs;
        self.union_type_names = s.union_type_names;
        self.var_struct = s.var_struct;
        self.var_unpacked_struct = s.var_unpacked_struct;
        self.record_array_vars = s.record_array_vars;
        self.record_soa_vars = s.record_soa_vars;
        self.var_enum = s.var_enum;
        self.struct_scalar_vars = s.struct_scalar_vars;
        self.struct_1d_array_vars = s.struct_1d_array_vars;
        self.packed_md_params = s.packed_md_params;
        self.wildcard_bound = s.wildcard_bound;
        self.local_decl_names = s.local_decl_names;
    }

    /// Restore the type registries around a top-level UNIT (module / interface /
    /// program / package), dropping the unit's BARE (unqualified) type names — a
    /// unit's local `typedef t;` is unit-scoped in SV (IEEE §3.12.1), NOT visible
    /// to the next unit — while KEEPING the scoped `pkg::t` twins a package body
    /// added (so `pkg::t` and `import pkg::*` still resolve). Without this the flat
    /// unit-global maps leak a bare package type (usable without `import`, which
    /// iverilog rejects) or a module-local type into a later module — both silent-
    /// wrong. Var-struct bindings are fully restored (unit-local, never leak).
    pub(crate) fn restore_scope_unit(&mut self, s: ScopeSnapshot) {
        // typedefs / struct_layouts / enum_defs: revert to the snapshot, then
        // re-add the `::`-qualified twins the unit created (absent from the snap).
        let new_td: Vec<(String, TypeInfo)> = self
            .typedefs
            .iter()
            .filter(|(k, _)| k.contains("::") && !s.typedefs.contains_key(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.typedefs = s.typedefs;
        self.typedefs.extend(new_td);
        let new_sl: Vec<(String, StructLayout)> = self
            .struct_layouts
            .iter()
            .filter(|(k, _)| k.contains("::") && !s.struct_layouts.contains_key(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.struct_layouts = s.struct_layouts;
        self.struct_layouts.extend(new_sl);
        // Round-9: unpacked-struct layouts scope exactly like `struct_layouts` —
        // keep the `pkg::T` twins this unit added, drop its bare names.
        let new_usl: Vec<(String, Vec<StructMember>)> = self
            .unpacked_struct_layouts
            .iter()
            .filter(|(k, _)| k.contains("::") && !s.unpacked_struct_layouts.contains_key(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.unpacked_struct_layouts = s.unpacked_struct_layouts;
        self.unpacked_struct_layouts.extend(new_usl);
        let new_ed: Vec<(String, Vec<(String, i64)>)> = self
            .enum_defs
            .iter()
            .filter(|(k, _)| k.contains("::") && !s.enum_defs.contains_key(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.enum_defs = s.enum_defs;
        self.enum_defs.extend(new_ed);
        let new_un: Vec<String> = self
            .union_type_names
            .iter()
            .filter(|k| k.contains("::") && !s.union_type_names.contains(*k))
            .cloned()
            .collect();
        self.union_type_names = s.union_type_names;
        self.union_type_names.extend(new_un);
        // Var-struct bindings are unit-local and never leak → full restore.
        self.var_struct = s.var_struct;
        self.var_unpacked_struct = s.var_unpacked_struct;
        self.record_array_vars = s.record_array_vars;
        self.record_soa_vars = s.record_soa_vars;
        self.var_enum = s.var_enum;
        self.struct_scalar_vars = s.struct_scalar_vars;
        self.struct_1d_array_vars = s.struct_1d_array_vars;
        self.packed_md_params = s.packed_md_params;
        self.wildcard_bound = s.wildcard_bound;
        self.local_decl_names = s.local_decl_names;
    }

    /// Map a member SOURCE bit index `e` onto the field part-select `pv[w-1:0]`.
    /// First remove the member's declared base (`e - dbase`) so the index is
    /// field-relative (`logic [15:8] a; a[11]` → `pv[3]`); then, for a descending
    /// member `pv[e]` (identity), for an ascending member `pv[w-1-e]` (field index 0
    /// is the field MSB, which is `pv`'s high bit). `dbase == 0` (a plain
    /// `[N:0]`/`[0:N]`/atom member) emits the pre-shift `e` UNCHANGED, so the IR is
    /// byte-identical to the pre-fix path for every zero-base member. `e` may be
    /// runtime; constant `w`/`dbase` fold in elaborate.
    pub(crate) fn remap_pv_idx(w: u32, ascending: bool, dbase: u32, e: Expr) -> Expr {
        let e = if dbase == 0 {
            e
        } else {
            let sp = e.span;
            mk_bin(BinOp::Sub, e, Self::dec_lit(dbase, sp))
        };
        if ascending {
            let sp = e.span;
            mk_bin(BinOp::Sub, Self::dec_lit(w - 1, sp), e)
        } else {
            e
        }
    }
}
