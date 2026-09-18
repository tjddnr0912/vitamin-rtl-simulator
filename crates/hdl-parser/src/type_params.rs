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
//! (`logic`/`reg`/`bit` with one or more packed ranges, the 2-state atoms, `time`,
//! an integral vector typedef, another type parameter); a struct / enum / real /
//! string / class default or override is a parse error. A type with two or more
//! packed dimensions rides one more carrier per dimension — see
//! `type_param_packed.rs`.
use super::*;

/// A type parameter's parse-time record.
#[derive(Clone)]
pub(crate) struct TypeParam {
    /// The width parameter's name (`T$w`).
    pub(crate) width_name: String,
    pub(crate) signed: bool,
    /// §3 ⑤ⓕ: the UNPACKED dimensions of `parameter type T = a_t` when `a_t` is an
    /// unpacked-array typedef. Empty for every other type parameter, which is what
    /// keeps the consumers that cannot compose a dim (`T'(e)`, an instance
    /// OVERRIDE) declining positively rather than answering the element's width.
    pub(crate) unpacked: Vec<Dim>,
    /// §3.a ⑤: the CARRIED packed dimensions of a multi-dimensional type parameter
    /// (`parameter type T = logic [1:0][3:0]`), in source order — the `T$p<i>a` /
    /// `T$p<i>b` names for an overridable one, the literal ranges otherwise. Empty
    /// for every one-dimensional / atom type parameter, so an override of one
    /// pushes exactly the connections it always pushed.
    pub(crate) packed: Vec<Range>,
}

/// One resolved integral type: its width EXPRESSION (a literal when it folds) and
/// shape.
pub(crate) struct TypeValue {
    pub(crate) width: Expr,
    pub(crate) signed: bool,
    pub(crate) two_state: bool,
    /// The resolved type's UNPACKED dimensions — non-empty only where the caller
    /// asked for them (`parse_type_param_value(true)`).
    pub(crate) unpacked: Vec<Dim>,
    /// §3 ⑤ⓕ: the SHAPE as an EXPRESSION rather than the literal `shape_flags`,
    /// set only when this type resolved to another shape-carrying type parameter
    /// (`parameter type U = T`, or `typedef T t2; parameter type U = t2;`). The
    /// literal would freeze `T`'s DEFAULT shape into `U$s`, so a `#(.T(…))`
    /// override that changes the shape would reach `U v;` as the default's — the
    /// exact silent-wrong the carrier exists to prevent. `None` for every concrete
    /// type, where `shape_flags()` is the answer.
    pub(crate) shape_expr: Option<Expr>,
    /// §3.a ⑤: ALL packed dimensions IN SOURCE ORDER when the type has two or more
    /// (`logic [1:0][3:0]` ⇒ `[[1:0],[3:0]]`); EMPTY for every one-dimensional or
    /// atom type. [`Self::width`] stays the TOTAL packed width either way, so every
    /// width consumer (`T$w`, `$bits(T)`, `T'(e)`) reads the one value it always
    /// read and only the per-dimension SHAPE travels here.
    pub(crate) packed: Vec<Range>,
    /// §3.b: the single DECLARED packed range of a ONE-dimensional type
    /// (`logic [8:1]` ⇒ `[8:1]`, a vector typedef's own range, an alias's aliased
    /// range). `None` for an atom, a range-less kind and every type with two or
    /// more packed dimensions — those ride [`Self::packed`], which stays the only
    /// carrier of a dimension LIST.
    ///
    /// It exists because `[T$w-1:0]` is a WIDTH, not a range: it answers `$bits`
    /// but loses the declared LSB (`logic [8:1] v` read `v[1]` as bit 0 and `$low`
    /// as 0 where both oracles read bit 1 and 1) and it defeats every parse-time
    /// fold that needs a literal extent (a packed-struct member, a package twin's
    /// `$bits(pkg::T)`). A NON-overridable type parameter of a CONCRETE type has
    /// nothing to carry, so its typedef is registered with THIS range instead —
    /// exactly the dims the `typedef` spelling of the same type would register.
    pub(crate) range: Option<Range>,
}

impl TypeValue {
    /// `T$s`: bit 0 = signed, bit 1 = 2-state, bits 2..17 = the number of UNPACKED
    /// dimensions, bits 18.. = the number of EXTRA packed dimensions (one less than
    /// the packed list's length, so a one-dimensional type contributes 0).
    ///
    /// The dim COUNT is what refuses an override whose ARITY differs from the
    /// default's (§3 ⑤ⓕ) — a dim-losing `#(.T(logic [15:0]))` on an unpacked
    /// default, and its mirror, a dim-carrying override of a scalar default. The
    /// module's declarations of `T` are stamped with the default's dim LIST at
    /// parse time, so its length is fixed for the life of the module and an
    /// override that changes it cannot be followed; the shape guard the group
    /// synthesizes turns the mismatch into a `$fatal`. The EXTENTS are a different
    /// question — they ride [`Parser::type_param_dim_params`] and DO follow an
    /// override, so they are deliberately absent here.
    ///
    /// Byte-identical for every design that predates the extents carrier: a count
    /// of 0 and 1 are the `0` and `4` the boolean spelled, and a ≥2-dim default is
    /// the only value that moves (every override of one was refused at the parse).
    ///
    /// The PACKED count joins it on the same terms and in a field of its own: a
    /// type with one packed range contributes `0`, which is every design that
    /// parsed before §3.a ⑤ (a multi-dimensional default was a parse error), so no
    /// existing `T$s` value moves. The two counts are separate fields rather than a
    /// sum so [`Parser::narrow_shape_guards`]' right shift keeps BOTH in the
    /// compare and a mismatch on either axis is still the `$fatal` it was.
    pub(crate) fn shape_flags(&self) -> u32 {
        (self.signed as u32)
            | ((self.two_state as u32) << 1)
            | ((self.unpacked.len().min(0xFFFF) as u32) << 2)
            | ((self.packed.len().saturating_sub(1).min(0x3FFF) as u32) << 18)
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
            let Some(tv) = self.parse_type_param_value(true) else {
                self.error(
                    "an integral type as the type parameter's default (`logic [N:0]` / `logic [N:0][M:0]` / `bit` / `int` / a vector typedef / another type parameter — a struct, enum, real, string or class type is unsupported in v1)",
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
                value: tv
                    .shape_expr
                    .clone()
                    .unwrap_or_else(|| Self::dec_lit(tv.shape_flags(), span)),
                span,
            });
            if overridable {
                self.overridable_params.insert(width_name.clone());
                self.overridable_params.insert(shape_name.clone());
                // The guard is emitted HERE (its position in the body fixes the
                // order of its `$fatal` against the user's own `initial` blocks),
                // with the STRICT compare. Whether the sign / 2-state bits stay in
                // that compare depends on the uses of `T`, which are parsed after
                // this group, so `narrow_shape_guards` rewrites the condition and
                // the message at module END for every `T` whose uses all reached a
                // container that carries `T$s`.
                guards.push(
                    self.type_param_shape_guard(
                        &name.name,
                        &shape_name,
                        tv.shape_expr
                            .clone()
                            .unwrap_or_else(|| Self::dec_lit(tv.shape_flags(), span)),
                        span,
                    ),
                );
                self.shape_carriers.insert(shape_name.clone());
            }
            // §3 ⑤ⓕ: an OVERRIDABLE type parameter's unpacked EXTENTS ride two more
            // synthesized value parameters per dim, so `#(.T(b_t))` carries its own.
            // `None` (a dynamic/queue/assoc dim, which no consumer accepts anyway)
            // and a non-overridable parameter both keep the literal dims.
            let carried = match self.type_param_dim_params(&name.name, &tv.unpacked, kind, span) {
                Some((dim_decls, dims)) if overridable => {
                    for d in &dim_decls {
                        self.overridable_params.insert(d.name.name.clone());
                    }
                    decls.extend(dim_decls);
                    dims
                }
                _ => tv.unpacked.clone(),
            };
            // §3.a ⑤: and the PACKED extents of a multi-dimensional one, declared
            // AFTER the `T$d…` pairs so the positional override channel pushes them
            // in the same order the group declares them (see `type_param_packed.rs`).
            let carried_packed =
                self.declare_packed_carriers(&name.name, &tv, kind, overridable, span, &mut decls);
            // The typedef every use of `T` resolves through: `[T$w-1:0]` of the
            // default's kind and signedness — or, for a multi-dimensional type, the
            // carried dimension list itself.
            //
            // §3.b: unless the parameter is NON-overridable and its default is a
            // CONCRETE type (`localparam type`, every package type parameter, a body
            // `parameter type` under a module that has a header) — then it has
            // nothing to carry and registers the DECLARED dims, exactly as the
            // `typedef` spelling of the same type does. An ALIAS of another type
            // parameter (`shape_expr` is `Some`) keeps the symbolic path: it must
            // follow the overridable parameter it names.
            let literal = !overridable && tv.shape_expr.is_none();
            let (td_range, td_packed) =
                match self.type_param_literal_range(&tv, literal, &carried_packed, span) {
                    Some(r) => (Some(r), Vec::new()),
                    None => Self::type_param_typedef_dims(&carried_packed, &width_name, span),
                };
            self.typedefs.insert(
                name.name.clone(),
                TypeInfo {
                    kind: if tv.two_state {
                        NetVarKind::Bit
                    } else {
                        NetVarKind::Logic
                    },
                    signed: tv.signed,
                    range: td_range,
                    packed: td_packed,
                    class_name: None,
                    // §3 ⑤ⓕ: the dims of an unpacked-array default ride the typedef,
                    // which is what every DECLARATION binder already reads
                    // (`decls.rs` stamps them onto each declarator, and the tf-port
                    // formal reads the same map) — so `T v;` is the explicit
                    // `logic [T$w-1:0] v [0:2]` it would have been written as. For an
                    // OVERRIDABLE one the extents are the synthesized names, so the
                    // same stamp follows an override.
                    unpacked: carried.clone(),
                    // §3 ⑤ⓕ: a NON-overridable alias type parameter (`localparam type
                    // U = T`, or a body `parameter type U = T` in a module that has a
                    // header) still carries a shape: its `U$s` VALUE is the expression
                    // `T$s`, an ordinary parameter elaborate folds per instance. Only a
                    // non-overridable parameter of a CONCRETE type (`shape_expr` None)
                    // has nothing to carry.
                    shape_param: (overridable || tv.shape_expr.is_some())
                        .then(|| shape_name.clone()),
                },
            );
            // §3 ⑤ⓕ: `U$s = T$s` — record the alias so an UNCARRIED use of `U` marks
            // `T`'s guard, not a name no guard reads. Resolved transitively at insert,
            // so a chain `V = U = T` lands on `T$s` in one step.
            if let Some(root) = tv
                .shape_expr
                .as_ref()
                .and_then(Self::shape_ident_name)
                .map(|n| self.shape_alias_root(&n))
            {
                self.shape_alias.insert(shape_name.clone(), root);
            }
            self.type_params.insert(
                name.name.clone(),
                TypeParam {
                    width_name,
                    signed: tv.signed,
                    unpacked: carried,
                    packed: carried_packed,
                },
            );
            self.local_decl_names.insert(name.name.clone());
            // §3.b: the positive record the `endpackage` twin pass reads. Only a
            // PACKAGE body writes it (a module's type parameters have no scoped
            // twin), and every container clears it, so the set names exactly the
            // type parameters of the body being parsed.
            if self.in_package {
                self.pkg_type_param_names.insert(name.name.clone());
            }
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

    /// §3 ⑤ⓕ: the per-dim value parameters an OVERRIDABLE type parameter's unpacked
    /// EXTENTS ride, and the dim list that names them.
    ///
    /// Two parameters per dim — `T$d<i>a` / `T$d<i>b`, the dim's DECLARED endpoints
    /// — and the registered typedef gets `[T$d<i>a:T$d<i>b]` in their place, which
    /// every declaration binder already reads. So an override's own extents reach
    /// `T v;`, `$size(v,1)`, `$low`/`$high` and the element addresses by the same
    /// route the element WIDTH reaches them through `T$w`.
    ///
    /// `[N]` normalizes to `[0:N-1]` first — measured identical in all three tools
    /// for `$bits` / `$size` / `$low` / `$high` and the element values — so the
    /// channel is exactly two values per dim whatever spelling either side used,
    /// and a POSITIONAL override cannot misalign on the form. The ARITY is not
    /// carried (the declarators are stamped once, at parse); `shape_flags` records
    /// it so a mismatch is the `$fatal` it was.
    ///
    /// ⚠️ Only for an OVERRIDABLE type parameter. Making a `localparam type` /
    /// package one's dims symbolic would move `$bits(T)` onto `sym_range_width`,
    /// which answers only when a bound NAMES an overridable parameter — a decline
    /// where the literal dims fold today.
    fn type_param_dim_params(
        &self,
        tname: &str,
        dims: &[Dim],
        kind: ParamKind,
        span: Span,
    ) -> Option<(Vec<ParamDecl>, Vec<Dim>)> {
        if dims.is_empty() {
            return None;
        }
        let mut decls = Vec::new();
        let mut out = Vec::new();
        for (i, d) in dims.iter().enumerate() {
            let (a, b) = Self::dim_endpoints(d, span)?;
            let na = format!("{tname}$d{i}a");
            let nb = format!("{tname}$d{i}b");
            for (n, v) in [(&na, a), (&nb, b)] {
                decls.push(ParamDecl {
                    kind,
                    signed: false,
                    ty: ParamType::Implicit,
                    range: None,
                    name: Ident {
                        name: n.clone(),
                        span,
                    },
                    value: v,
                    span,
                });
            }
            out.push(Dim::Range(Range {
                msb: Self::ident_expr(&na, span),
                lsb: Self::ident_expr(&nb, span),
                span,
            }));
        }
        Some((decls, out))
    }

    /// One unpacked dim's two DECLARED endpoints, `[N]` read as its `[0:N-1]`
    /// (IEEE §7.4.2). `None` for a dynamic / queue / associative dim — none of
    /// which any type-parameter consumer accepts.
    fn dim_endpoints(d: &Dim, span: Span) -> Option<(Expr, Expr)> {
        match d {
            Dim::Range(r) => Some((r.msb.clone(), r.lsb.clone())),
            Dim::Size(e) => Some((
                Self::dec_lit(0, span),
                Self::sub(e.clone(), Self::dec_lit(1, span), span),
            )),
            Dim::Dyn | Dim::Queue(_) | Dim::Assoc(_) => None,
        }
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

    /// Parse a TYPE in type-parameter position (a default, or an instance
    /// override's value) and resolve it to a width expression and shape. `None`
    /// (nothing consumed) when the cursor is not on a type this desugar carries:
    /// the caller either errors (a default) or parses an ordinary expression (an
    /// override, where the token may be a value).
    /// `allow_unpacked` is the OPT-IN for an unpacked-array typedef (§3 ⑤ⓕ): the
    /// DEFAULT position carries its dims (they reach `T v;` through the registered
    /// typedef), an instance OVERRIDE does not — `T$w`/`T$s` have no dim slot, so
    /// an override that parsed would keep the DEFAULT's dims and answer `$bits` 48
    /// where both oracles measure 64. A literal `false` there short-circuits every
    /// line this parameter guards, which is what makes the override byte-identical.
    pub(crate) fn parse_type_param_value(&mut self, allow_unpacked: bool) -> Option<TypeValue> {
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
            let mut packed = Vec::new();
            let mut decl_range: Option<Range> = None;
            let (width, signed) = match atom_w {
                Some(w) => {
                    let s1 = self.opt_signed();
                    (Self::dec_lit(w, span), s0.or(s1).unwrap_or(atom_signed))
                }
                None => {
                    let range = self.opt_range();
                    // §3.a ⑤: every FURTHER packed dimension. `T$w` stays the total
                    // width (the product) and the per-dimension shape rides the
                    // `T$p<i>a/b` carrier, so a select on `T v` reads the same
                    // dimensions the explicit spelling would have given it.
                    let extra = self.opt_packed_dims();
                    if range.is_none() && !extra.is_empty() {
                        // a range-less kind cannot carry further dimensions
                        self.pos = save;
                        return None;
                    }
                    packed = Self::packed_dim_list(range.as_ref(), &extra);
                    // §3.b: only a ONE-dimensional type reports a declared range;
                    // `packed` is the carrier the moment there are two or more.
                    if packed.is_empty() {
                        decl_range = range.clone();
                    }
                    let s1 = self.opt_signed();
                    let w = if !packed.is_empty() {
                        match self.packed_dims_width(&packed, span) {
                            Some(e) => e,
                            None => {
                                self.pos = save;
                                return None;
                            }
                        }
                    } else {
                        match &range {
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
                        }
                    };
                    (w, s0.or(s1).unwrap_or(false))
                }
            };
            return Some(TypeValue {
                width,
                signed,
                two_state,
                unpacked: Vec::new(),
                shape_expr: None,
                packed,
                range: decl_range,
            });
        }
        // A type NAME: another type parameter of this module, or an integral vector
        // typedef (bare or `pkg::t`). A struct / enum / union / class / non-integral
        // typedef is not carried (the caller decides between an error and a value).
        if self.is_ident() {
            let key = self.type_name_key();
            if let Some(tp) = self.type_params.get(&key).cloned() {
                if !tp.unpacked.is_empty() && !allow_unpacked {
                    return None;
                }
                self.bump();
                let shape_expr = self
                    .typedefs
                    .get(&key)
                    .and_then(|i| i.shape_param.clone())
                    .map(|n| Self::ident_expr(&n, span));
                // §3.b: an alias inherits the aliased parameter's DECLARED range —
                // the typedef registered for it holds the same dims this one gets.
                let decl_range = if tp.packed.is_empty() {
                    self.typedefs.get(&key).and_then(|i| i.range.clone())
                } else {
                    None
                };
                return Some(TypeValue {
                    width: Self::ident_expr(&tp.width_name, span),
                    signed: tp.signed,
                    two_state: self
                        .typedefs
                        .get(&key)
                        .is_some_and(|i| i.kind == NetVarKind::Bit),
                    unpacked: tp.unpacked.clone(),
                    shape_expr,
                    // §3.a ⑤: an ALIAS (`parameter type U = T`, a pass-through
                    // `.T(T)`) inherits the CARRIED dimension names, so `U` follows
                    // an override of `T` instead of freezing its default extents.
                    packed: tp.packed.clone(),
                    range: decl_range,
                });
            }
            let info = self.peek_typedef_name()?;
            if self.struct_layouts.contains_key(&key)
                || self.sym_struct_layouts.contains_key(&key)
                || self.enum_defs.contains_key(&key)
                || self.union_type_names.contains(&key)
                || info.class_name.is_some()
                // §3 ⑤ⓕ: the `T$w`/`T$s` value-parameter desugar has no dim slot, so
                // the dims travel beside it — through the typedef this group
                // registers for `T`, which is where `T v;` reads them. Only the
                // caller that HAS that carrier opts in.
                || (!info.unpacked.is_empty() && !allow_unpacked)
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
            // §3.a ⑤: a MULTI-DIMENSIONAL vector typedef (`typedef logic [1:0][3:0]
            // a_t;`). `TypeInfo` splits the dimensions the way a declaration does —
            // `range` is the outermost and `packed` every one after it — so the
            // source-order list is `range` ahead of `packed`. An ATOM typedef
            // cannot reach here with dimensions (`typedef_dims_layout` refuses
            // packed dimensions on a range-less typedef at its declaration), and a
            // declined `range` here would silently drop the outer dimension.
            let packed = Self::packed_dim_list(info.range.as_ref(), &info.packed);
            if !info.packed.is_empty() && packed.is_empty() {
                return None;
            }
            let width = if !packed.is_empty() {
                self.packed_dims_width(&packed, span)?
            } else {
                match Self::atom_member_width(info.kind) {
                    Some(w) => Self::dec_lit(w, span),
                    None => match &info.range {
                        None => Self::dec_lit(1, span),
                        Some(r) => match self.member_width(&Some(r.clone())) {
                            Some(w) => Self::dec_lit(w, span),
                            None => self.sym_range_width(r)?,
                        },
                    },
                }
            };
            self.eat_scope_qualifier();
            self.bump(); // the type name
            let decl_range = if packed.is_empty() {
                info.range.clone()
            } else {
                None
            };
            return Some(TypeValue {
                width,
                signed: info.signed,
                two_state,
                unpacked: info.unpacked.clone(),
                shape_expr: info.shape_param.as_ref().map(|n| Self::ident_expr(n, span)),
                packed,
                range: decl_range,
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
    /// vector typedef with at least one SYMBOLIC extent â a range bound or an
    /// unpacked dimension that names an overridable parameter: the width
    /// EXPRESSION. `None` for every other argument â the caller's literal fold
    /// (`parse_bits_type_arg`) and the expression path answer those. The cursor is
    /// just after `(`; consumes through `)` on success.
    ///
    /// ⚠️ It must be an EXPRESSION and not a number even though the parser could
    /// fold the declaration's own default: a header `parameter` is overridable per
    /// instance, so a parse-time value would bake the PRE-override width. Measured:
    /// `module m #(parameter N=4); typedef logic [7:0] a_t [0:N-1];` under
    /// `m #(.N(8))` is 64 in both oracles, and the packed twin `logic [N-1:0]`
    /// already answered 8 through this same desugar.
    pub(crate) fn parse_bits_sym_type_arg(&mut self) -> Option<Expr> {
        if !self.is_ident() || self.peek_at(1) != Some(TokenKind::RParen) {
            return None;
        }
        let span = self.cur_span();
        let key = self.cur_text().to_string();
        // §3 ⑤ⓕ: `T$w` is the ELEMENT width, so a dim-carrying type parameter takes
        // the typedef route instead — `sym_typedef_bits` multiplies the element by
        // every packed and unpacked dim, which is the 24 both oracles answer for
        // `typedef logic [7:0] a_t [0:2]`.
        let e = match self.type_params.get(&key) {
            // `T$w` is the ELEMENT width, so a dim-carrying type parameter multiplies
            // it by every unpacked dim of the RESOLVED DEFAULT type — the 24 both
            // oracles answer for `typedef logic [7:0] a_t [0:2]`, 32 for a 16-bit
            // element and 48 for a 2-D one. A dim-free `T` composes no factor, so it
            // keeps the bare `T$w` this arm always returned.
            //
            // ⚠️ Built from `tp` HERE rather than routed through `sym_typedef_bits`,
            // even though that builder owns the same product for a typedef NAME: it
            // opens by standing itself down on `local_decl_names`, and the type
            // parameter's own registration inserts `T` into that set (see the
            // `local_decl_names.insert` in `parse_type_param_group`). Routing `T`
            // through it therefore declines every time. The stand-down must keep
            // firing for a genuine TYPEDEF key — a same-named variable shadows the
            // type and verilator reads the variable's width — so the ROUTE changes
            // and the guard does not. Going through `tp` also avoids
            // `sym_range_width`'s `names_an_overridable` requirement, which is what
            // makes the non-overridable `localparam type T = a_t` spelling fold too.
            Some(tp) => {
                let elem = Self::ident_expr(&tp.width_name, span);
                let mut any_sym = false;
                self.sym_unpacked_dims_mul(&tp.unpacked, elem, span, &mut any_sym)?
            }
            None => self.sym_typedef_bits(&key, span)?,
        };
        self.bump(); // type name
        self.bump(); // )
        Some(e)
    }

    /// The width EXPRESSION of an integral vector typedef, as `element × every
    /// packed dim × every unpacked dim`. Each factor is the literal count where the
    /// parse-time table folds it and the symbolic form where it does not, so a
    /// LITERAL element width beside a symbolic dimension (`logic [7:0] a_t [0:N-1]`)
    /// composes â the earlier shape declined it because the element range folded and
    /// the dimension had no slot in the desugar.
    ///
    /// `None` unless at least one factor is symbolic: with every factor literal the
    /// caller's numeric fold (`bits_of_type_name`) is the answer, and returning an
    /// expression here would move cells it already owns.
    ///
    /// ⚠️ `Dyn` / `Queue` / `Assoc` still decline and there is no oracle to move
    /// toward â iverilog rejects `$bits` of a `[]` / `[$]` typedef and verilator
    /// reports an internal fault on both.
    fn sym_typedef_bits(&self, key: &str, span: Span) -> Option<Expr> {
        // A VARIABLE of the same name shadows the type, and the parser has no scope
        // to see that with — so it must not claim the name at all when the module
        // body also DECLARES it. Measured: `typedef logic [7:0] a_t [0:N-1];` beside
        // a block-local `logic [11:0] a_t` (or a formal of that name) is 12 in
        // verilator and was 12 here through the expression path; answering the
        // type's width would be a correct → silent-wrong trade. iverilog rejects the
        // shape outright, so verilator is the oracle and vita's own PRE answer
        // agreed with it.
        if self.local_decl_names.contains(key) {
            return None;
        }
        let info = self.typedefs.get(key)?;
        if !matches!(
            info.kind,
            NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit
        ) || self.struct_layouts.contains_key(key)
            || self.sym_struct_layouts.contains_key(key)
        {
            return None;
        }
        let mut any_sym = false;
        // The ELEMENT: the declared range, or the kind's own width when it folds
        // (a range-free `typedef logic a_t [0:N-1]` is a 1-bit element).
        let mut acc = match self.member_width_kind(info.kind, &info.range) {
            Some(w) => Self::dec_lit(w, span),
            None => {
                any_sym = true;
                self.sym_range_width(info.range.as_ref()?)?
            }
        };
        // One factor per dimension. `Range` counts its extent; an unpacked `[N]` is
        // `[0:N-1]`, so the size expression IS the count.
        let factor = |me: &Self, r: &Range, any_sym: &mut bool| -> Option<Expr> {
            match me.member_width(&Some(r.clone())) {
                Some(w) => Some(Self::dec_lit(w, span)),
                None => {
                    *any_sym = true;
                    me.sym_range_width(r)
                }
            }
        };
        for d in &info.packed {
            let f = factor(self, d, &mut any_sym)?;
            acc = Self::mul(acc, f, span);
        }
        let acc = self.sym_unpacked_dims_mul(&info.unpacked, acc, span, &mut any_sym)?;
        any_sym.then_some(acc)
    }

    /// Multiply `acc` by one factor per UNPACKED dimension: the literal extent where
    /// the parse-time table folds it, the symbolic form where it does not.
    ///
    /// Shared by the two routes that answer `$bits` of a dim-carrying NAME — the
    /// typedef one ([`Self::sym_typedef_bits`]) and the type-PARAMETER one
    /// ([`Self::parse_bits_sym_type_arg`]) — so the product is one rule and not two
    /// that can drift apart. `Dyn` / `Queue` / `Assoc` decline for both, and there is
    /// no oracle to move toward: iverilog rejects `$bits` of a `[]` / `[$]` typedef
    /// and verilator reports an internal fault on both.
    ///
    /// `any_sym` is SET, never cleared: the typedef route reads it to decide whether
    /// its caller's numeric fold already owns the answer, and the type-parameter
    /// route ignores it because its element (`T$w`) is symbolic by construction.
    fn sym_unpacked_dims_mul(
        &self,
        dims: &[Dim],
        mut acc: Expr,
        span: Span,
        any_sym: &mut bool,
    ) -> Option<Expr> {
        for d in dims {
            let f = match d {
                Dim::Range(r) => match self.member_width(&Some(r.clone())) {
                    Some(w) => Self::dec_lit(w, span),
                    None => {
                        *any_sym = true;
                        self.sym_range_width(r)?
                    }
                },
                // An unpacked `[N]` is `[0:N-1]`, so the size expression IS the count.
                Dim::Size(e) => match Self::lit_u32(e) {
                    Some(n) => Self::dec_lit(n, span),
                    None => {
                        // Only an OVERRIDABLE-parameter expression composes; any
                        // other unfoldable leaf (a variable, a declined constant)
                        // must stay loud rather than reach elaborate as a width.
                        self.names_an_overridable(e)?;
                        *any_sym = true;
                        e.clone()
                    }
                },
                Dim::Dyn | Dim::Queue(_) | Dim::Assoc(_) => return None,
            };
            acc = Self::mul(acc, f, span);
        }
        Some(acc)
    }

    /// `T'(e)` for a type parameter or a symbolic-width vector typedef: the size
    /// cast's width expression and the type's signedness (the same composition
    /// `parse_size_or_named_cast` builds for a literal-width typedef).
    pub(crate) fn type_param_cast(&self, key: &str) -> Option<(Expr, bool)> {
        let span = Span::new(0, 0);
        if let Some(tp) = self.type_params.get(key) {
            // §3 ⑤ⓕ: a cast to an UNPACKED type is NO-ORACLE (iverilog aborts on an
            // internal assertion, verilator refuses it) and `T$w` is the ELEMENT
            // width, so answering here would cast to a third of the type. Loud.
            if !tp.unpacked.is_empty() {
                return None;
            }
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
        // §3 ⑤ⓕ: an unpacked-array override IS carried — its element width rides
        // `T$w` and its extents the `T$d…` pair per dim that the default
        // synthesized. A dim-COUNT mismatch is still refused, by the shape guard
        // rather than here: the module's declarators were stamped with the
        // default's dim list at parse time, so a differing arity cannot be
        // followed and `shape_flags` turns it into the `$fatal` it was.
        let Some(tv) = self.parse_type_param_value(true) else {
            self.pos = save;
            self.error(
                "an integral type as the type parameter override (a struct, enum, real, string or class type is unsupported in v1)",
            );
            return false;
        };
        if !matches!(self.peek(), Some(TokenKind::RParen | TokenKind::Comma)) {
            self.pos = save;
            return false;
        }
        let span = start.to(self.prev_span());
        // A dim this channel cannot spell (dynamic / queue / associative) keeps the
        // pre-slice refusal rather than reaching the module with its extents lost.
        let mut dims = Vec::new();
        for d in &tv.unpacked {
            let Some(ab) = Self::dim_endpoints(d, span) else {
                self.pos = save;
                self.error(
                    "an integral type as the type parameter override (a struct, enum, real, string or class type is unsupported in v1)",
                );
                return false;
            };
            dims.push(ab);
        }
        // §3 ⑤ⓕ: a PASS-THROUGH override `n #(.T(T))` hands the OUTER instance's own
        // `T$s` on, exactly as the `parameter type U = T` default does. A literal here
        // would freeze the outer type parameter's DEFAULT shape into the inner module —
        // measured silent-wrong (`n m1=255 neg=0` where verilator says `-1 1`).
        let flags = tv
            .shape_expr
            .clone()
            .unwrap_or_else(|| Self::dec_lit(tv.shape_flags(), span));
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
                for (i, (a, b)) in dims.into_iter().enumerate() {
                    for (suf, v) in [("a", a), ("b", b)] {
                        out.push(ParamConn::Named {
                            name: Ident {
                                name: format!("{}$d{i}{suf}", n.name),
                                span: n.span,
                            },
                            value: Some(v),
                            span,
                        });
                    }
                }
                // §3.a ⑤: the PACKED extents, after the unpacked ones — the order
                // the group declares them in.
                Self::push_packed_override_conns(out, Some(n), &tv.packed, span);
            }
            None => {
                out.push(ParamConn::Positional(tv.width));
                out.push(ParamConn::Positional(flags));
                // Same order the group declares them in, so the slots line up
                // whenever the arity matches — and when it does not, the shape
                // guard fires on `T$s`, which is bound at its own fixed slot.
                for (a, b) in dims {
                    out.push(ParamConn::Positional(a));
                    out.push(ParamConn::Positional(b));
                }
                Self::push_packed_override_conns(out, None, &tv.packed, span);
            }
        }
        true
    }
}
