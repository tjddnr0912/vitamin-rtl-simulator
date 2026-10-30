//! packed union typedefs — split out of `typedefs.rs` (mechanical move, §5.2 row 66-0).

use super::*;

impl Parser<'_, '_> {
    /// `typedef union packed { <type> f1; … } name;` (ⓑ-breadth, IEEE §7.3.1).
    /// A packed union OVERLAYS its members — every member shares bit 0, and the
    /// union width is the MAX member width (vs the struct's SUM). Recorded in the
    /// same `struct_layouts` map, so `u.field` desugars to a part-select exactly
    /// like a struct; a write to one member is visible through every other
    /// (different-width members read/write their own low bits). Pure parser
    /// addition (IR-0) — reuses `TypedefKind::Struct` for the AST node.
    pub(crate) fn parse_typedef_union(&mut self, start: Span) -> Option<ModuleItem> {
        self.parse_union_type(start, None)
    }

    /// The body of `parse_typedef_union`; `anon` as for `parse_struct_type`.
    pub(crate) fn parse_union_type(
        &mut self,
        start: Span,
        anon: Option<String>,
    ) -> Option<ModuleItem> {
        self.bump(); // `union`
        if !self.eat_kw(Kw::Packed) {
            self.error("`packed` after `union` (unpacked union unsupported in v1)");
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        // `union packed signed` (§7.3.1/§7.2.1): whole-union value signedness, exactly as
        // for a signed packed struct. Member overlay/layout and per-member signedness are
        // unaffected. Absent the keyword ⇒ unsigned (byte-identical for every prior union).
        let union_signed = self.opt_signed().unwrap_or(false);
        self.expect(TokenKind::LBrace, "'{' for union body");
        let mut members = Vec::new();
        let mut nested_keys = Vec::new();
        let mut each_exact = true;
        while self.peek() != Some(TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let m_start = self.cur_span();
            let Some((kind, signed, range, packed_dims, nested, _shape, exact)) =
                self.parse_struct_member_type(false)
            else {
                break;
            };
            each_exact &= exact;
            while let Some(name) = self.ident() {
                members.push(StructMember {
                    name,
                    kind,
                    signed,
                    range: range.clone(),
                    packed_dims: packed_dims.clone(),
                    span: m_start.to(self.prev_span()),
                });
                nested_keys.push(nested.clone());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Semi, "';'");
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close union body");
        let tname = self.anon_or_typedef_name(start, anon)?;
        let mut widths = Vec::with_capacity(members.len()); // (flat_width, elem_stride)
        for m in &members {
            match self.member_flat_dims(m.kind, &m.range, &m.packed_dims) {
                Some((flat, stride)) if flat > 0 => widths.push((flat, stride)),
                _ => {
                    self.error_at(
                        m.span,
                        "union member width must be a named integer type or a \
                         constant-literal range in v1",
                    );
                    widths.push((1, 1));
                }
            }
        }
        // OVERLAY: union width = MAX member width; every member starts at bit 0.
        let total: u32 = widths.iter().map(|(f, _)| *f).max().unwrap_or(1);
        let fields: Vec<StructFieldLayout> = members
            .iter()
            .zip(&widths)
            .zip(&nested_keys)
            .map(|((m, (w, stride)), nested)| {
                (
                    m.name.name.clone(),
                    0u32,
                    *w,
                    self.member_ascending(&m.range),
                    m.signed,
                    Self::member_kind_two_state(m.kind),
                    self.member_dbase(&m.range),
                    *stride,
                    nested.clone(),
                )
            })
            .collect();
        let reg = self.next_type_reg();
        let layout = StructLayout { fields, reg };
        self.register_decl(
            reg,
            DeclType::Packed {
                layout,
                union: true,
            },
        );
        self.tn_set(
            &tname.name,
            type_rec::Slot::Packed(type_rec::PackedSlot {
                key: DeclKey(reg),
                respelled: None,
            }),
        );
        // A union shares the packed slot (for `u.field` member reads) but its overlay
        // layout (all fields at offset 0, width = MAX) is NOT a packed concat — so it
        // is flagged here to EXCLUDE it from the `'{…}` pattern desugar (which would
        // wrongly concatenate the fields). Pattern on a union stays loud.
        self.tn_set(&tname.name, type_rec::Slot::Union);
        // §7.2.1: an all-2-state union is itself 2-state (defaults to 0); any
        // 4-state member makes the whole union 4-state (defaults to X).
        let union_kind =
            if !members.is_empty() && members.iter().all(|m| Self::member_kind_two_state(m.kind)) {
                NetVarKind::Bit
            } else {
                NetVarKind::Logic
            };
        let info = TypeInfo {
            kind: union_kind,
            signed: union_signed,
            range: Some(Self::dec_range(total.saturating_sub(1))),
            packed: Vec::new(),
            class_name: None,
            unpacked: Vec::new(),
            shape_param: None,
            enum_type: false,
            layout_exact: Self::members_layout_exact(&members, each_exact),
            pattern_members: None,
            layout_reg: Some(reg),
            bind: 0,
        };
        self.put_type_info(&tname.name, info);
        self.stamp_type_bind(&tname.name);
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Struct { members },
            span: start.to(self.prev_span()),
        }))
    }
}
