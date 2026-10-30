//! typedef-typed ports, tf ports and for-loop variables — split out of `typedefs.rs` (mechanical move, §5.2 row 66-0).

use super::*;

impl Parser<'_, '_> {
    /// A typedef name as a MODULE port type (`input mode_e m`, `input byte_t a`,
    /// `input cfg_t c`) → its `(kind, signed, range, struct_name)`, mirroring
    /// `try_tf_port_typedef` for tf-ports (typedef-recognition family). REQUIRES
    /// the next token to be the port NAME (an Ident) so a bare continuation
    /// (`input byte_t a, b` — `b` is a name, not a type) is NOT misresolved as a
    /// type. EXT2-E1: a PACKED struct/union typedef IS supported as a module port
    /// — the port net is the struct's flat vector (`info.range` = total width) and
    /// the caller binds the port NAME to the layout (`var_struct`, via
    /// `bind_tf_port_struct`) so `c.field` desugars to a part-select, exactly as a
    /// module-internal `cfg_t c;` var does. A class handle or a multi-dim-packed
    /// vector typedef port stays honest-loud (the AnsiPort/PortDecl shape carries
    /// no class binding / extra packed dims). Used by both the ANSI
    /// (`parse_ansi_port`) and non-ANSI (`parse_port_decl`) port parsers.
    ///
    /// §3 ⑤: the SIXTH slot is the typedef's own UNPACKED dims
    /// (`typedef logic [7:0] a_t [0:3]; module m(input a_t i, …)`), which the ANSI
    /// caller lands on the port's `unpacked` list — the field it already fills
    /// from dims written after the name. Empty for every other typedef.
    ///
    /// §3.a ⑤: the SEVENTH slot is the typedef's own INNER PACKED dims
    /// (`typedef logic [1:0][3:0] t_t;` ⇒ `[[3:0]]`, the outermost riding the third
    /// slot). The ANSI caller appends them AFTER whatever `typedef_dims_layout`
    /// built from dims written before the port name, exactly as the declaration
    /// binder does. Empty for every other typedef, and for the shapes that stay
    /// loud (a class handle, a struct or non-vector typedef with packed dims).
    #[allow(clippy::type_complexity)]
    pub(crate) fn try_port_typedef(
        &mut self,
    ) -> Option<(
        NetVarKind,
        bool,
        Option<Range>,
        Option<String>,
        Vec<Range>,
        Vec<Dim>,
        Vec<Range>,
    )> {
        let info = self.peek_typedef_name()?;
        let q = self.scope_qualifier_len(); // 0 (bare) or 2 (`pkg::`)
                                            // `<type> <name>`, or (§4.5.425) `<type> [dims] <name>` — a packed array of
                                            // the typedef; anything else is left for the continuation/name path.
        if !matches!(
            self.peek_at(q + 1),
            Some(TokenKind::Word(WordKind::Ident))
                | Some(TokenKind::EscapedIdent)
                | Some(TokenKind::LBracket)
        ) {
            return None;
        }
        let nm = self.type_name_key();
        let is_struct = self.packed_layout(&nm).is_some() || self.sym_layout(&nm).is_some();
        // §3.a ⑤: a MULTI-DIMENSIONAL packed VECTOR typedef is now carried — the
        // seventh slot hands its inner dims to the caller, which composes them
        // exactly as the declaration binder does (`decls.rs::parse_typed_decl`), so
        // `input t_t x` is byte-identical to `input logic [1:0][3:0] x`. A class
        // handle, and a STRUCT or non-vector typedef carrying packed dims, still
        // need per-port machinery the `AnsiPort`/`PortDecl` shape has no slot for.
        let carried_packed = !info.packed.is_empty()
            && !is_struct
            && matches!(
                info.kind,
                NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit
            );
        if info.class_name.is_some() || (!info.packed.is_empty() && !carried_packed) {
            self.error(
                "a class or multi-dim-packed typedef as a module port type is unsupported in v1 (a simple vector / enum / packed-struct typedef port is supported)",
            );
        }
        self.eat_scope_qualifier();
        self.bump(); // the typedef-name token
                     // §4.5.425: packed dims AFTER the type name (`cfg_t [N-1:0] p`, `pkg::t
                     // [pkg::C-1:0] p` — the ibex_top_tracing port shape): the outer dims of a
                     // packed array whose element is the typedef; the caller folds them ahead of
                     // the typedef's own range.
        let extra = self.opt_packed_dims();
        // IEEE 1800 §7.4.1: the ELEMENT of a packed array must itself be packed, so
        // `typedef logic [3:0] u_t [0:1]; input u_t [2:0] a` is illegal — iverilog
        // says so by name ("Packed array base-type `u_t` is not packed"), verilator
        // by refusing the port connection (§7.6). Composing the dims anyway builds a
        // shape no legal source can name.
        if !extra.is_empty() && !info.unpacked.is_empty() {
            self.error(
                "no packed dimensions before the name of an unpacked-array typedef \
                     (IEEE 1800 §7.4.1: a packed array's element must be a packed \
                     type; write the dims on the element type)",
            );
        }
        let struct_name = if is_struct { Some(nm) } else { None };
        let inner_packed = if carried_packed {
            info.packed
        } else {
            Vec::new()
        };
        Some((
            info.kind,
            info.signed,
            info.range,
            struct_name,
            extra,
            info.unpacked,
            inner_packed,
        ))
    }

    /// One ANSI tf-port: `[input|output|inout] [net_or_var] [signed] [range] name`.
    /// Returns the port, the (possibly-inherited) direction, and the
    /// (possibly-inherited) type, so a following bare `, name` keeps both.
    ///
    /// §13.3/§23.2.2.3 type stickiness: a bare `, name` (no direction keyword AND
    /// no type spec) inherits the previous port's full type. A direction keyword
    /// resets the type (`input y` after `input logic [7:0] x` makes `y` the default
    /// 1-bit), and any explicit type (net/var keyword, `signed`/`unsigned`, or a
    /// range) starts a fresh type that itself propagates onward.
    /// If the cursor is on a user-defined type name usable as a tf-port type
    /// (`task t(byte_t a)` / `input byte_t a;`), consume it and return its
    /// `(kind, signed, range)`. A struct / union / class / multi-dim-packed
    /// typedef port needs per-port layout or method binding not in v1 —
    /// honest-loud (the diagnostic is emitted, but the name is still consumed so
    /// it does not cascade as the port name). Returns `None` when the cursor is
    /// not on a typedef name, so the caller keeps its built-in / inherited-type
    /// handling. Shared by the ANSI (`parse_tf_port`) and non-ANSI
    /// (`parse_tf_port_decl_into`) port parsers.
    ///
    /// §3 ⑤ⓕ: the SEVENTH slot carries the typedef's own UNPACKED dims
    /// (`typedef logic [7:0] a_t [0:3]` → `[0:3]`). `TfPort.unpacked` is the
    /// declarator's slot for exactly the same dims, so the caller seeds it with
    /// these and the resulting port is byte-identical to the explicit spelling
    /// `input logic [7:0] v [0:3]` — which elaborate already lowers
    /// (`array_formal::is_fixed_unpacked_array_formal`, `inline_task`'s
    /// force-frame). Dims on BOTH the typedef and the declarator stay loud: that
    /// is a live oracle SPLIT on dimension order, refused by the same rule the
    /// declaration binder uses (`decls.rs`).
    #[allow(clippy::type_complexity)]
    pub(crate) fn try_tf_port_typedef(
        &mut self,
    ) -> Option<(
        NetVarKind,
        bool,
        Option<Range>,
        Option<String>,
        Option<String>,
        Vec<Range>,
        Vec<Dim>,
    )> {
        // R5: an UNPACKED struct typedef IS supported as a tf-port — it expands to one
        // member formal per field (`$unp$<port>$<field>`), because a heterogeneous
        // record cannot ride a single flat vector the way a PACKED struct port does.
        // Signal the caller (via the 5th tuple slot = unpacked struct type name) to run
        // the 1→N expansion after it has parsed the port NAME; consume the type-name so
        // the port name is next (unpacked structs are absent from `typedefs`, so a
        // fall-through would otherwise leave the name unconsumed and cascade).
        let nm = self.type_name_key();
        if self.record_def(&nm).is_some() {
            self.eat_scope_qualifier();
            let span = self.cur_span();
            self.bump(); // consume the type-name token so the port name is next
                         // Family A (r17): a PACKABLE unpacked-struct tf-port rides a SINGLE flat
                         // vector — exactly like a packed struct (4th tuple slot = struct_name →
                         // `bind_tf_port_struct`, so `c.field` desugars to a part-select in the
                         // body), NOT the 1→N per-member expansion. This makes the formal's
                         // representation identical to a packable struct VARIABLE (now also
                         // whole-vector — see the scalar branch of `parse_unpacked_struct_decl`), so
                         // `f(structvar)` passes the WHOLE value (no `expand_struct_call_args`
                         // scatter) and a queue-of-record `q.push_back(p)` works end-to-end. A
                         // NON-packable record (string/real/mixed member) cannot ride one vector →
                         // 1→N per-member (5th slot), unchanged.
            if let Some(layout) = self.packable_record_layout(&nm) {
                let w: u32 = layout.fields.iter().map(|f| f.2).sum();
                let all_two_state = layout.fields.iter().all(|f| f.5);
                let kind = if all_two_state {
                    NetVarKind::Bit
                } else {
                    NetVarKind::Logic
                };
                return Some((
                    kind,
                    false,
                    Some(self.synth_bit_range(w, span)),
                    Some(nm),
                    None,
                    Vec::new(),
                    Vec::new(),
                ));
            }
            return Some((
                NetVarKind::Reg,
                false,
                None,
                None,
                Some(nm),
                Vec::new(),
                Vec::new(),
            ));
        }
        let info = self.peek_typedef_name()?;
        let nm = self.type_name_key();
        // EXT2-C: a packed struct/union typedef IS supported as a tf-port. The frame
        // var is the struct's flat vector (`info.range` = total width) and the caller
        // binds the port NAME to the layout (`var_struct`) so `c.field` desugars to a
        // part-select in the body. A class handle stays honest-loud (the TfPort shape
        // carries no class binding). A multi-dim packed vector typedef (§4.5.418)
        // returns its inner dims in the sixth slot; the caller flattens the formal and
        // binds the dims for the body rewrite, as for the inline spelling.
        let is_struct = self.packed_layout(&nm).is_some();
        // A class handle stays honest-loud (the TfPort shape carries no class
        // binding).
        if info.class_name.is_some() {
            self.error(
                "a class typedef type for a tf-port (a vector / enum / packed-struct / multi-dim packed typedef port is supported)",
            );
        }
        // §3 ⑤ⓕ: an UNPACKED-array typedef no longer declines here — its dims ride
        // the seventh slot onto `TfPort.unpacked`, the same field the explicit
        // spelling `input logic [7:0] v [0:3]` fills, so the two spellings build the
        // SAME port and elaborate's existing lowering carries it.
        //
        // A STRUCT typedef is the one shape held back: a packed struct rides `range`
        // as a flat vector and the 4th slot binds its layout, so an array OF one has
        // no element type to spell here and the dims would land on the flat vector.
        // Measured unreachable today — a struct typedef cannot CARRY unpacked dims,
        // because both spellings are refused at the declaration (the inline
        // `typedef struct packed {…} sa_t [0:1];` is a parse error, and the chained
        // `typedef s_t sa_t [0:1];` is "an unpacked-array typedef of an aliased
        // type"). It is kept fail-closed so that opening either declaration does not
        // silently bind the element here.
        if is_struct && !info.unpacked.is_empty() {
            self.error(
                "a non-array struct typedef for a tf-port (an unpacked ARRAY of a struct typedef as a formal is unsupported in v1 — write the dimensions on the formal)",
            );
        }
        self.eat_scope_qualifier();
        self.bump(); // the typedef-name token
        let struct_name = if is_struct { Some(nm) } else { None };
        let td_unpacked = info.unpacked.clone();
        Some((
            info.kind,
            info.signed,
            info.range,
            struct_name,
            None,
            info.packed,
            td_unpacked,
        ))
    }

    /// SV §12.7.1 typed for-init with a USER-DEFINED type name (`for (my_t i=0; …)`
    /// where `my_t` is a typedef/enum/struct). The resolved `TypeInfo` supplies
    /// kind/sign/range/packed — the type NAME is a single token (no inline range),
    /// so unlike the built-in path there is nothing to `opt_range`/`opt_packed`.
    /// Mirrors the built-in arm; reuses the SAME `<T> <name>` disambiguation as
    /// block-local typedef decls and function-return typedefs (§4.5.40/§4.5.41).
    pub(crate) fn parse_for_typed_init_typedef(
        &mut self,
        info: TypeInfo,
    ) -> Option<(NetVarDecl, Stmt, Ident)> {
        let start = self.cur_span(); // type-name token span (matches the built-in path)
        self.eat_scope_qualifier(); // skip an optional `pkg::` before the type name
        self.bump(); // the type-name identifier
                     // A class-handle alias cannot be a loop counter (no arithmetic on a
                     // handle, §8.4). Emit a clear loud error rather than letting a
                     // ClassHandle decl with no class type flow to elaborate (which would
                     // report the misleading "class handle without a class type" + a
                     // cascade). The type name is already consumed, so returning None
                     // lets the plain-assign fallback parse the `name = init`; elaborate's
                     // single follow-on is then the (correct) undeclared-loop-var error.
                     // §3 ⑤: an unpacked-array typedef is not a loop counter either, and
                     // `build_for_typed_init` has no dim slot — it would silently declare one
                     // element.
        if info.class_name.is_some() || !info.unpacked.is_empty() {
            // Phrased to fit the parser's "expected <X>, found <token>" template
            // (the `found` token names the offending class).
            self.error("a non-class scalar type for the for-loop variable (a class handle has no loop arithmetic; an unpacked-array typedef is not a counter)");
            return None;
        }
        self.build_for_typed_init(start, info.kind, info.signed, info.range, info.packed)
    }
}
