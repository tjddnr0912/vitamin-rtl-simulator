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
            type_names: self.type_names.clone(),
            var_struct: self.var_struct.clone(),
            var_unpacked_struct: self.var_unpacked_struct.clone(),
            record_array_vars: self.record_array_vars.clone(),
            record_soa_vars: self.record_soa_vars.clone(),
            var_enum: self.var_enum.clone(),
            struct_scalar_vars: self.struct_scalar_vars.clone(),
            struct_1d_array_vars: self.struct_1d_array_vars.clone(),
            struct_packed_array_vars: self.struct_packed_array_vars.clone(),
            packed_md_params: self.packed_md_params.clone(),
            wildcard_bound: self.wildcard_bound.clone(),
            local_decl_names: self.local_decl_names.clone(),
            scope_type_names: self.scope_type_names.clone(),
            const_locals: self.const_locals.clone(),
            class_records: self.class_records.clone(),
            var_class: self.var_class.clone(),
            cur_container: self.cur_container,
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
        // §5.2 row 67a: a class-handle binding the declaration shadows comes back at
        // the block's end (`exit_block_handle_scope`).
        let handles = (!self.var_class.is_empty() && !self.block_handle_shadows.is_empty())
            .then(|| self.var_class.clone());
        let d = self.parse_net_var(false);
        if let (Some(h), Some(d)) = (handles, &d) {
            self.note_block_handle_shadows(h, d);
        }
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

    /// `snapshot_scope` boxed in THIS frame, and `restore_scope` fed from the box
    /// in THIS frame — so the by-value `ScopeSnapshot` temporaries never land in a
    /// caller that recurses per nesting level. `block_body` sits on the
    /// `begin … begin` chain; every map added to the snapshot widened its frame
    /// (the parser's `depth_guard` test measures that edge — §4.5.431 tripped it).
    pub(crate) fn snapshot_scope_boxed(&self) -> Box<ScopeSnapshot> {
        Box::new(self.snapshot_scope())
    }

    pub(crate) fn restore_scope_boxed(&mut self, s: Box<ScopeSnapshot>) {
        self.restore_scope(*s);
    }

    pub(crate) fn restore_scope(&mut self, s: ScopeSnapshot) {
        self.type_names = s.type_names;
        self.var_struct = s.var_struct;
        self.var_unpacked_struct = s.var_unpacked_struct;
        self.record_array_vars = s.record_array_vars;
        self.record_soa_vars = s.record_soa_vars;
        self.var_enum = s.var_enum;
        self.struct_scalar_vars = s.struct_scalar_vars;
        self.struct_1d_array_vars = s.struct_1d_array_vars;
        self.struct_packed_array_vars = s.struct_packed_array_vars;
        self.packed_md_params = s.packed_md_params;
        self.wildcard_bound = s.wildcard_bound;
        self.local_decl_names = s.local_decl_names;
        self.scope_type_names = s.scope_type_names;
        self.const_locals = s.const_locals;
        self.class_records = s.class_records;
        self.var_class = s.var_class;
        self.cur_container = s.cur_container;
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
        // Every type slot reverts to the snapshot, except that a `::`-qualified name
        // keeps a slot the unit added where the snapshot has none (a package twin);
        // a symbolic layout is module-local and has no `pkg::` twin, so its slot is
        // restored whole (`unit_kept`).
        let kept = Self::unit_kept(&self.type_names, &s.type_names);
        self.restore_type_names_unit(s.type_names, &kept);
        // Var-struct bindings are unit-local and never leak → full restore.
        self.var_struct = s.var_struct;
        self.var_unpacked_struct = s.var_unpacked_struct;
        self.record_array_vars = s.record_array_vars;
        self.record_soa_vars = s.record_soa_vars;
        self.var_enum = s.var_enum;
        self.struct_scalar_vars = s.struct_scalar_vars;
        self.struct_1d_array_vars = s.struct_1d_array_vars;
        self.struct_packed_array_vars = s.struct_packed_array_vars;
        self.packed_md_params = s.packed_md_params;
        self.wildcard_bound = s.wildcard_bound;
        self.local_decl_names = s.local_decl_names;
        self.scope_type_names = s.scope_type_names;
        self.const_locals = s.const_locals;
        // §5.2 row 67a: a unit's class records are unit-local (full restore): a
        // module's own classes are no base for the next unit's.
        self.class_records = s.class_records;
        self.var_class = s.var_class;
        self.cur_container = s.cur_container;
    }

    /// A generate block is a scope (IEEE §27.2): the type names it binds itself start
    /// empty — the module's typedefs are an enclosing scope's. The caller has
    /// snapshotted the scope; the restore at the block's end gives the set back.
    pub(crate) fn enter_generate_type_scope(&mut self) {
        self.scope_type_names.clear();
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
