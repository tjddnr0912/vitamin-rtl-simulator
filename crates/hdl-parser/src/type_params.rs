//! `parameter type T = <type>` — a TYPE parameter (IEEE 1800 §6.20.3), desugared in
//! the parser with no AST change (§3 ⑤ / ROADMAP §2 🆕 L ⓥ, §4.5.437).
//!
//! The AST carries VALUE parameters only, and a module is parsed once while its
//! instances may each override `T` with a different type. What every use of `T`
//! inside the module needs from the type is its packed WIDTH and its SHAPE
//! (signedness, 2- or 4-state), so a type parameter becomes two value parameters
//! the override channel already carries — `T$w` (the width) and `T$s` (the shape:
//! bit 0 = signed, bit 1 = 2-state) — plus a parser typedef `T` = `logic [T$w-1:0]`
//! (or `bit`, with the default's signedness) that every declaration, port, cast
//! (`T'(e)` = `signing'(T$w'(e))`) and `$bits(T)` (= `T$w`) resolves through, so
//! the body lowers exactly as it would with a symbolic-width typedef and elaborate
//! folds `T$w` per instance.
//!
//! An instance override `.T(logic [15:0])` / positional `#(logic [15:0], …)` /
//! pass-through `.T(T)` desugars to the two value overrides. The width can be
//! per-instance; the SHAPE cannot (a declaration's `signed` and its 2-state kind are
//! fixed in the AST), so an override whose shape differs from the default's is
//! refused LOUDLY by a synthesized `initial if (T$s != <default>) $fatal` — never
//! a silently unsigned `T`. The integral vector subset is the delivered scope
//! (`logic`/`reg`/`bit` with one packed range, the 2-state atoms, `time`, an
//! integral vector typedef, another type parameter); a struct / enum / real /
//! string / class / multi-dimensional default or override is a parse error.
use super::*;

/// A type parameter's parse-time record.
#[derive(Clone)]
pub(crate) struct TypeParam {
    /// The width parameter's name (`T$w`).
    pub(crate) width_name: String,
    pub(crate) signed: bool,
}

/// One resolved integral type: its width EXPRESSION (a literal when it folds) and
/// shape.
pub(crate) struct TypeValue {
    pub(crate) width: Expr,
    pub(crate) signed: bool,
    pub(crate) two_state: bool,
}

impl TypeValue {
    /// `T$s`: bit 0 = signed, bit 1 = 2-state.
    pub(crate) fn shape_flags(&self) -> u32 {
        (self.signed as u32) | ((self.two_state as u32) << 1)
    }
}

impl Parser<'_, '_> {
    /// Cursor at `parameter` / `localparam` / `type`: does a TYPE parameter
    /// declaration start here (`[parameter|localparam] type NAME`)? `type` is not a
    /// reserved word in this lexer, so it is matched as the contextual identifier.
    pub(crate) fn starts_type_param(&self) -> bool {
        let i = usize::from(matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Parameter | Kw::Localparam
            )))
        ));
        matches!(self.peek_at(i), Some(TokenKind::Word(WordKind::Ident)))
            && self.text_at(i) == "type"
            && matches!(
                self.peek_at(i + 1),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
    }

    /// Parse ONE type-parameter group `[parameter|localparam] type NAME = <type>
    /// {, NAME = <type>}` (a continuation inherits `type`, §6.20.1). Returns the
    /// desugared value parameters (two per name, in order) and, for an overridable
    /// one, the shape guard process. Registers each name as a typedef and in
    /// `type_params`. The cursor is left on the token after the last type.
    pub(crate) fn parse_type_param_group(
        &mut self,
        header: bool,
    ) -> (Vec<ParamDecl>, Vec<ModuleItem>) {
        let start = self.cur_span();
        let kind = if self.eat_kw(Kw::Localparam) {
            ParamKind::Localparam
        } else {
            self.eat_kw(Kw::Parameter);
            ParamKind::Parameter
        };
        self.bump(); // `type`
        let mut decls = Vec::new();
        let mut guards = Vec::new();
        loop {
            let Some(name) = self.ident() else { break };
            if !self.expect(TokenKind::Eq, "'=' after the type parameter name") {
                break;
            }
            let Some(tv) = self.parse_type_param_value() else {
                self.error(
                    "an integral type as the type parameter's default (`logic [N:0]` / `bit` / `int` / a vector typedef / another type parameter — a struct, enum, real, string, class or multi-dimensional type is unsupported in v1)",
                );
                break;
            };
            let span = start.to(self.prev_span());
            let width_name = format!("{}$w", name.name);
            let shape_name = format!("{}$s", name.name);
            let overridable = kind == ParamKind::Parameter
                && !self.in_package
                && (header || !self.has_param_header);
            decls.push(ParamDecl {
                kind,
                signed: false,
                ty: ParamType::Implicit,
                range: None,
                name: Ident {
                    name: width_name.clone(),
                    span: name.span,
                },
                value: tv.width.clone(),
                span,
            });
            decls.push(ParamDecl {
                kind,
                signed: false,
                ty: ParamType::Implicit,
                range: None,
                name: Ident {
                    name: shape_name.clone(),
                    span: name.span,
                },
                value: Self::dec_lit(tv.shape_flags(), span),
                span,
            });
            if overridable {
                self.overridable_params.insert(width_name.clone());
                self.overridable_params.insert(shape_name.clone());
                guards.push(self.type_param_shape_guard(
                    &name.name,
                    &shape_name,
                    tv.shape_flags(),
                    span,
                ));
            }
            // The typedef every use of `T` resolves through: `[T$w-1:0]` of the
            // default's kind and signedness.
            let msb = Self::sub(
                Self::ident_expr(&width_name, span),
                Self::dec_lit(1, span),
                span,
            );
            self.typedefs.insert(
                name.name.clone(),
                TypeInfo {
                    kind: if tv.two_state {
                        NetVarKind::Bit
                    } else {
                        NetVarKind::Logic
                    },
                    signed: tv.signed,
                    range: Some(Range {
                        msb,
                        lsb: Self::dec_lit(0, span),
                        span,
                    }),
                    packed: Vec::new(),
                    class_name: None,
                    unpacked: Vec::new(),
                },
            );
            self.type_params.insert(
                name.name.clone(),
                TypeParam {
                    width_name,
                    signed: tv.signed,
                },
            );
            self.local_decl_names.insert(name.name.clone());
            // A continuation `, NAME = <type>` stays in this group; `, parameter …`
            // / `, type …` / a port list ends it (the caller eats that comma).
            if self.peek() == Some(TokenKind::Comma)
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
                )
                && self.peek_at(2) == Some(TokenKind::Eq)
                && !self.starts_type_param_at(1)
            {
                self.bump(); // ,
                continue;
            }
            break;
        }
        (decls, guards)
    }

    /// `starts_type_param` looking `n` tokens ahead.
    fn starts_type_param_at(&self, n: usize) -> bool {
        let i = n + usize::from(matches!(
            self.peek_at(n),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Parameter | Kw::Localparam
            )))
        ));
        matches!(self.peek_at(i), Some(TokenKind::Word(WordKind::Ident)))
            && self.text_at(i) == "type"
    }

    /// `initial if (T$s != <default>) $fatal(1, "…");` — the loud refusal of an
    /// override that changes the type's SHAPE (signedness / 2-state), which the
    /// module's declarations cannot follow. A process rather than a generate `if`
    /// so the user's unnamed generate blocks keep their §27.6 `genblk<N>` numbers.
    fn type_param_shape_guard(
        &self,
        tname: &str,
        shape_name: &str,
        default_flags: u32,
        span: Span,
    ) -> ModuleItem {
        let cond = Expr {
            kind: ExprKind::Binary {
                op: BinOp::Ne,
                lhs: Box::new(Self::ident_expr(shape_name, span)),
                rhs: Box::new(Self::dec_lit(default_flags, span)),
            },
            span,
        };
        let msg = format!(
            "\"type parameter `{tname}`: the override changes the type's signedness or 2-state kind, which the module's declarations of `{tname}` cannot follow (an override must keep the default type's shape; only its width may differ — v1)\""
        );
        let call = Stmt::SysTaskCall {
            name: Ident {
                name: "$fatal".to_string(),
                span,
            },
            args: vec![
                Self::dec_lit(1, span),
                Expr {
                    kind: ExprKind::StrLit { raw: msg },
                    span,
                },
            ],
            span,
        };
        ModuleItem::Proc(ProceduralBlock {
            kind: ProcKind::Initial,
            sensitivity: None,
            body: Box::new(Stmt::If {
                cond,
                then_s: Box::new(call),
                else_s: None,
                span,
            }),
            span,
        })
    }

    /// Parse a TYPE in type-parameter position (a default, or an instance
    /// override's value) and resolve it to a width expression and shape. `None`
    /// (nothing consumed) when the cursor is not on a type this desugar carries:
    /// the caller either errors (a default) or parses an ordinary expression (an
    /// override, where the token may be a value).
    pub(crate) fn parse_type_param_value(&mut self) -> Option<TypeValue> {
        let save = self.pos;
        let span = self.cur_span();
        // A data-type keyword.
        if let Some(kind) = self.net_var_kind() {
            let (atom_w, atom_signed, two_state) = match kind {
                NetVarKind::Logic | NetVarKind::Reg => (None, false, false),
                NetVarKind::Bit => (None, false, true),
                NetVarKind::Int => (Some(32), true, true),
                NetVarKind::Integer => (Some(32), true, false),
                NetVarKind::Byte => (Some(8), true, true),
                NetVarKind::Shortint => (Some(16), true, true),
                NetVarKind::Longint => (Some(64), true, true),
                NetVarKind::Time => (Some(64), false, false),
                _ => return None,
            };
            self.bump(); // the kind keyword
            let s0 = self.opt_signed();
            let (width, signed) = match atom_w {
                Some(w) => {
                    let s1 = self.opt_signed();
                    (Self::dec_lit(w, span), s0.or(s1).unwrap_or(atom_signed))
                }
                None => {
                    let range = self.opt_range();
                    if self.peek() == Some(TokenKind::LBracket) {
                        // a second packed dimension: the flat width would lose the
                        // element shape a select on `T v` needs
                        self.pos = save;
                        return None;
                    }
                    let s1 = self.opt_signed();
                    let w = match &range {
                        None => Self::dec_lit(1, span),
                        Some(r) => match self.member_width(&Some(r.clone())) {
                            Some(w) => Self::dec_lit(w, span),
                            None => match self.sym_range_width(r) {
                                Some(e) => e,
                                None => {
                                    self.pos = save;
                                    return None;
                                }
                            },
                        },
                    };
                    (w, s0.or(s1).unwrap_or(false))
                }
            };
            return Some(TypeValue {
                width,
                signed,
                two_state,
            });
        }
        // A type NAME: another type parameter of this module, or an integral vector
        // typedef (bare or `pkg::t`). A struct / enum / union / class / non-integral
        // typedef is not carried (the caller decides between an error and a value).
        if self.is_ident() {
            let key = self.type_name_key();
            if let Some(tp) = self.type_params.get(&key).cloned() {
                self.bump();
                return Some(TypeValue {
                    width: Self::ident_expr(&tp.width_name, span),
                    signed: tp.signed,
                    two_state: self
                        .typedefs
                        .get(&key)
                        .is_some_and(|i| i.kind == NetVarKind::Bit),
                });
            }
            let info = self.peek_typedef_name()?;
            if self.struct_layouts.contains_key(&key)
                || self.sym_struct_layouts.contains_key(&key)
                || self.enum_defs.contains_key(&key)
                || self.union_type_names.contains(&key)
                || info.class_name.is_some()
                || !info.packed.is_empty()
                // §3 ⑤: the `T$w`/`T$s` value-parameter desugar has no dim slot.
                || !info.unpacked.is_empty()
            {
                return None;
            }
            let two_state = match info.kind {
                NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Integer | NetVarKind::Time => {
                    false
                }
                NetVarKind::Bit
                | NetVarKind::Int
                | NetVarKind::Byte
                | NetVarKind::Shortint
                | NetVarKind::Longint => true,
                _ => return None,
            };
            let width = match Self::atom_member_width(info.kind) {
                Some(w) => Self::dec_lit(w, span),
                None => match &info.range {
                    None => Self::dec_lit(1, span),
                    Some(r) => match self.member_width(&Some(r.clone())) {
                        Some(w) => Self::dec_lit(w, span),
                        None => self.sym_range_width(r)?,
                    },
                },
            };
            self.eat_scope_qualifier();
            self.bump(); // the type name
            return Some(TypeValue {
                width,
                signed: info.signed,
                two_state,
            });
        }
        None
    }

    /// The width of a packed range whose bounds name overridable parameters, as the
    /// §7.4.1 expression `(msb >= lsb) ? msb - lsb + 1 : lsb - msb + 1` (elaborate
    /// folds it per instance); `[X-1:0]` folds to `X`. `None` when a bound is not
    /// a parse-time constant or an overridable parameter (a variable, a function).
    pub(crate) fn sym_range_width(&self, r: &Range) -> Option<Expr> {
        if self.names_an_overridable(&r.msb)? | self.names_an_overridable(&r.lsb)? {
            // fine — at least one bound is symbolic
        } else {
            return None;
        }
        let span = r.span;
        if Self::lit_u32(&r.lsb) == Some(0) {
            if let ExprKind::Binary {
                op: BinOp::Sub,
                lhs,
                rhs,
            } = &r.msb.kind
            {
                if Self::lit_u32(rhs) == Some(1) {
                    return Some((**lhs).clone());
                }
            }
        }
        let bin = |op: BinOp, l: Expr, rr: Expr| Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(rr),
            },
            span,
        };
        let up = Self::add(
            Self::sub(r.msb.clone(), r.lsb.clone(), span),
            Self::dec_lit(1, span),
            span,
        );
        let down = Self::add(
            Self::sub(r.lsb.clone(), r.msb.clone(), span),
            Self::dec_lit(1, span),
            span,
        );
        Some(Expr {
            kind: ExprKind::Ternary {
                cond: Box::new(bin(BinOp::Ge, r.msb.clone(), r.lsb.clone())),
                then_e: Box::new(up),
                else_e: Box::new(down),
            },
            span,
        })
    }

    /// `$bits(<type>)` where the type is a type parameter (`T$w`) or an integral
    /// vector typedef whose range names an overridable parameter (the symbolic
    /// width): the width EXPRESSION. `None` for every other argument — the caller's
    /// literal fold (`parse_bits_type_arg`) and the expression path answer those.
    /// The cursor is just after `(`; consumes through `)` on success.
    pub(crate) fn parse_bits_sym_type_arg(&mut self) -> Option<Expr> {
        if !self.is_ident() || self.peek_at(1) != Some(TokenKind::RParen) {
            return None;
        }
        let span = self.cur_span();
        let key = self.cur_text().to_string();
        let e = if let Some(tp) = self.type_params.get(&key) {
            Self::ident_expr(&tp.width_name, span)
        } else {
            let info = self.typedefs.get(&key)?;
            if !matches!(
                info.kind,
                NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit
            ) || !info.packed.is_empty()
                || !info.unpacked.is_empty() // §3 ⑤: no dim slot in the desugar
                || self.struct_layouts.contains_key(&key)
                || self.sym_struct_layouts.contains_key(&key)
            {
                return None;
            }
            let r = info.range.as_ref()?;
            if self.member_width(&Some(r.clone())).is_some() {
                return None; // a literal width: the numeric fold answers
            }
            self.sym_range_width(r)?
        };
        self.bump(); // type name
        self.bump(); // )
        Some(e)
    }

    /// `T'(e)` for a type parameter or a symbolic-width vector typedef: the size
    /// cast's width expression and the type's signedness (the same composition
    /// `parse_size_or_named_cast` builds for a literal-width typedef).
    pub(crate) fn type_param_cast(&self, key: &str) -> Option<(Expr, bool)> {
        let span = Span::new(0, 0);
        if let Some(tp) = self.type_params.get(key) {
            return Some((Self::ident_expr(&tp.width_name, span), tp.signed));
        }
        let info = self.typedefs.get(key)?;
        if !matches!(info.kind, NetVarKind::Logic | NetVarKind::Reg)
            || !info.packed.is_empty()
            || !info.unpacked.is_empty() // §3 ⑤: no dim slot in the desugar
            || self.struct_layouts.contains_key(key)
            || self.sym_struct_layouts.contains_key(key)
        {
            return None;
        }
        let r = info.range.as_ref()?;
        if self.member_width(&Some(r.clone())).is_some() {
            return None;
        }
        Some((self.sym_range_width(r)?, info.signed))
    }

    /// An instance's parameter override whose value is a TYPE (`.T(logic [15:0])`,
    /// a positional `logic [15:0]`, a typedef name, a pass-through `.T(T)`): push
    /// the two value overrides the type parameter desugars to. `false` (nothing
    /// consumed) when the value is not a type — the caller parses an expression.
    pub(crate) fn push_type_param_override(
        &mut self,
        out: &mut Vec<ParamConn>,
        name: Option<&Ident>,
        start: Span,
    ) -> bool {
        // A bare identifier is a type only when it names a type: a value parameter
        // of this module (`.N(W)`) must parse as the expression it is.
        if self.is_ident() {
            let key = self.type_name_key();
            let is_type = self.type_params.contains_key(&key)
                || (self.typedefs.contains_key(&key)
                    && !self.const_locals.contains_key(&key)
                    && !self.overridable_params.contains(&key));
            if !is_type {
                return false;
            }
        } else if self.net_var_kind().is_none() {
            return false;
        }
        let save = self.pos;
        let Some(tv) = self.parse_type_param_value() else {
            self.pos = save;
            self.error(
                "an integral type as the type parameter override (a struct, enum, real, string, class or multi-dimensional type is unsupported in v1)",
            );
            return false;
        };
        if !matches!(self.peek(), Some(TokenKind::RParen | TokenKind::Comma)) {
            self.pos = save;
            return false;
        }
        let span = start.to(self.prev_span());
        let flags = Self::dec_lit(tv.shape_flags(), span);
        match name {
            Some(n) => {
                out.push(ParamConn::Named {
                    name: Ident {
                        name: format!("{}$w", n.name),
                        span: n.span,
                    },
                    value: Some(tv.width),
                    span,
                });
                out.push(ParamConn::Named {
                    name: Ident {
                        name: format!("{}$s", n.name),
                        span: n.span,
                    },
                    value: Some(flags),
                    span,
                });
            }
            None => {
                out.push(ParamConn::Positional(tv.width));
                out.push(ParamConn::Positional(flags));
            }
        }
        true
    }
}
