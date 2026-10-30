//! per-instance (symbolic) packed struct producer and its helpers — split out of `typedefs.rs` (mechanical move, §5.2 row 66-0).

use super::*;

impl Parser<'_, '_> {
    /// §3 ⑤ ⓒ: register a packed struct whose layout is SYMBOLIC (see
    /// `SymStructLayout`). A member that folds keeps its folded width as a literal;
    /// one that does not must be a vector kind with a single `[E:0]` range (the
    /// width is then `E + 1`, descending, zero-based); anything else — a non-zero or
    /// non-literal LSB, an ascending range, a multi-dim member, a nested struct
    /// member — reports the member, as the numeric path did. The typedef's own
    /// range is `[total-1:0]`, so a declaration, a port and a packed array of the
    /// type size per instance through the ordinary typedef path.
    pub(crate) fn register_sym_struct(
        &mut self,
        start: Span,
        members: Vec<StructMember>,
        nested_keys: Vec<Option<TyRef>>,
        shape_keys: Vec<Option<Ident>>,
        tname: Ident,
        struct_signed: bool,
    ) -> Option<ModuleItem> {
        let mut widths: Vec<Expr> = Vec::with_capacity(members.len());
        for (m, nested) in members.iter().zip(&nested_keys) {
            let w = match self.member_flat_dims(m.kind, &m.range, &m.packed_dims) {
                Some((flat, _)) if flat > 0 => Some(Self::dec_lit(flat, m.span)),
                _ => self.sym_member_width(m, nested.is_some()),
            };
            match w {
                Some(w) => widths.push(w),
                None => {
                    self.error_at(
                        m.span,
                        "struct member width must be a named integer type or a \
                         constant-literal range in v1 (a `[E:0]` range over a header \
                         parameter is laid out per instance)",
                    );
                    widths.push(Self::dec_lit(1, m.span));
                }
            }
        }
        // MSB-first: the first member occupies the high bits; each offset is the
        // sum of the widths BELOW it.
        let mut fields = Vec::with_capacity(members.len());
        let mut below: Vec<Expr> = Vec::new();
        for ((m, w), sp) in members.iter().zip(&widths).zip(&shape_keys).rev() {
            let off = Self::sum_of(&below, m.span);
            // §3 ⑤ⓕ: `m.signed` is the DEFAULT's sign; when the member names an
            // overridable `parameter type T` the read emits `SigningParam(T$s)` and
            // folds the instance's sign instead.
            fields.push((m.name.name.clone(), off, w.clone(), m.signed, sp.clone()));
            below.push(w.clone());
        }
        fields.reverse();
        let total = Self::sum_of(&widths, start);
        let struct_kind =
            if !members.is_empty() && members.iter().all(|m| Self::member_kind_two_state(m.kind)) {
                NetVarKind::Bit
            } else {
                NetVarKind::Logic
            };
        let reg = self.next_type_reg();
        let layout = SymStructLayout {
            fields,
            total: total.clone(),
            reg,
        };
        self.register_decl(reg, DeclType::Sym(layout));
        self.tn_set(&tname.name, type_rec::Slot::Sym(DeclKey(reg)));
        self.tn_clear(&tname.name, type_rec::Kind::Union);
        let msb = Self::sym_sub_one(total, start);
        let info = TypeInfo {
            kind: struct_kind,
            signed: struct_signed,
            range: Some(Range {
                msb,
                lsb: Self::dec_lit(0, start),
                span: start,
            }),
            packed: Vec::new(),
            class_name: None,
            unpacked: Vec::new(),
            shape_param: None,
            enum_type: false,
            // Laid out per instance from a bound this parse could not fold.
            layout_exact: false,
            pattern_members: None,
            layout_reg: None,
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

    /// The symbolic width of one non-folding member: a vector kind, no packed
    /// dims, not a nested struct, a range whose LSB folds to 0 and whose MSB is
    /// an arithmetic expression over literals, foldable constants and AT LEAST
    /// ONE overridable parameter (`overridable_params`) ⇒ `msb + 1`. An MSB the
    /// table declined for any other reason keeps the numeric path's refusal.
    pub(crate) fn sym_member_width(&self, m: &StructMember, nested: bool) -> Option<Expr> {
        if nested
            || !m.packed_dims.is_empty()
            || !matches!(
                m.kind,
                NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit
            )
        {
            return None;
        }
        let r = m.range.as_ref()?;
        if self.const_bound(&r.lsb) != Some(0) || !self.names_an_overridable(&r.msb)? {
            return None;
        }
        // IEEE §7.4.1: the width is |msb − lsb| + 1 — `[W-1:0]` with `W = 0` is
        // `[-1:0]`, TWO bits (review B B1: `msb + 1` alone gave 0 and the struct
        // lost a member — both oracles `bits=5` for the first draft's 3). The
        // sign of the comparison follows `E`'s own type, as the oracles evaluate
        // the range's direction.
        let sp = r.msb.span;
        let e = r.msb.clone();
        let neg = mk_bin(BinOp::Lt, e.clone(), Self::dec_lit(0, sp));
        let up = mk_bin(BinOp::Add, e.clone(), Self::dec_lit(1, sp));
        let down = mk_bin(BinOp::Sub, Self::dec_lit(1, sp), e);
        Some(Expr {
            kind: ExprKind::Ternary {
                cond: Box::new(neg),
                then_e: Box::new(down),
                else_e: Box::new(up),
            },
            span: sp,
        })
    }

    /// `Some(true)` when `e` is built from literals, foldable constants, `+ - * /
    /// %`, unary `-`/`+` and `$clog2`, and names at least one overridable
    /// parameter; `Some(false)` when it folds outright; `None` for any other leaf
    /// (a declined constant, a variable, a call) or shape.
    pub(crate) fn names_an_overridable(&self, e: &Expr) -> Option<bool> {
        match &e.kind {
            ExprKind::IntLit { .. } => Some(false),
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                if self.overridable_params.contains(&p.segments[0].name) {
                    Some(true)
                } else {
                    self.const_bound(e).map(|_| false)
                }
            }
            ExprKind::PkgScoped { .. } => self.const_bound(e).map(|_| false),
            ExprKind::Unary {
                op: UnOp::Minus | UnOp::Plus,
                operand,
            } => self.names_an_overridable(operand),
            ExprKind::Binary {
                op:
                    BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::Mod
                    | BinOp::Shl
                    | BinOp::Shr,
                lhs,
                rhs,
            } => {
                let l = self.names_an_overridable(lhs)?;
                let r = self.names_an_overridable(rhs)?;
                Some(l || r)
            }
            ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                self.names_an_overridable(&args[0])
            }
            // A comparison and a ternary — the LRM width form this very layout
            // emits for a nested symbolic member (`(E < 0) ? 1 - E : E + 1`), and a
            // user's `[(W > 4 ? W : 4)-1:0]`.
            ExprKind::Binary {
                op:
                    BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::InsideEq,
                lhs,
                rhs,
            } => {
                let l = self.names_an_overridable(lhs)?;
                let r = self.names_an_overridable(rhs)?;
                Some(l || r)
            }
            ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                let c = self.names_an_overridable(cond)?;
                let t = self.names_an_overridable(then_e)?;
                let e = self.names_an_overridable(else_e)?;
                Some(c || t || e)
            }
            _ => None,
        }
    }

    /// `a + b + …` over `parts` — the literals folded into one leading literal
    /// (`packed_md`'s `add`), the empty sum `0`.
    pub(crate) fn sum_of(parts: &[Expr], span: Span) -> Expr {
        let mut lit: u32 = 0;
        let mut syms: Vec<&Expr> = Vec::new();
        for p in parts {
            match Self::lit_u32(p) {
                Some(v) => lit = lit.saturating_add(v),
                None => syms.push(p),
            }
        }
        let mut acc = Self::dec_lit(lit, span);
        for s in syms {
            acc = Self::add(acc, s.clone(), span);
        }
        acc
    }

    /// `e - 1`, folded when `e` is a literal or ends in `+ <literal>` (so a member
    /// width `E + 1` gives the bound `E` back).
    pub(crate) fn sym_sub_one(e: Expr, span: Span) -> Expr {
        if let ExprKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = &e.kind
        {
            if let Some(k) = Self::lit_u32(rhs) {
                if k >= 1 {
                    return Self::add((**lhs).clone(), Self::dec_lit(k - 1, span), span);
                }
            }
        }
        Self::sub(e, Self::dec_lit(1, span), span)
    }

    /// §3 ⑤ ⓒ: `T'(e)` for a 4-state symbolic-layout struct — the size cast to its
    /// packed width EXPRESSION (elaborate folds it per instance) with the struct's
    /// whole-value signedness. A 2-state one stays loud, like the numeric twin.
    pub(crate) fn sym_typedef_cast(&self, name: &str) -> Option<(Expr, bool)> {
        let info = self.td(name)?;
        if !matches!(info.kind, NetVarKind::Logic | NetVarKind::Reg) {
            return None;
        }
        let l = self.sym_layout(name)?;
        Some((l.total.clone(), info.signed))
    }
}
