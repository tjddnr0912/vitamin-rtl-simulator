//! A packed struct member declared `elem_t [hi:0] m`, a packed array of a packed struct,
//! and the five declaration forms that travel with it.
//!
//! The member is laid out as its keyword twin `logic [hi:0][W-1:0] m`, so every layout
//! reader sees a form it already knows, and `StructLayout::arrays` records what the
//! twin cannot say: the element's declaration. A select of an element is resolved here
//! and leaves as one of two nodes on the flat struct net:
//!
//! * a literal index, `s.m[2].q`, is the part-select `s.q` would be (`s[hi:lo]`);
//! * a run-time index is the guarded image an out-of-range index must have:
//!   `s[((e) >= 0 && (e) < N) ? (e)*W+L : OOB_DROP_BIT +: w]`. The sentinel lies past
//!   every net, so an out-of-range read is x and an out-of-range write has no effect.
//!
//! The guard reads `e` twice and compares it against 32-bit literals, so it is only
//! right where `e` is a 32-bit, 2-state value that nothing can move between the two
//! reads. That is why the run-time arm takes a closed set of index shapes
//! (`index_var`), on a name bound by a loop the parser saw (`Parser::idx_scope`), in
//! the two processes where it was measured; everything else is refused by name.
//! Neither arm folds a bound at the use: the numbers were fixed at the declaration.

use super::*;

/// A member, or a `typedef`, of this form that the rule does not open.
const REFUSE_MEMBER: &str = "a packed array of a packed struct as a struct member: `elem_t \
     [hi:0] m`, hi >= 0 a constant, in a named packed struct declared at the top of this \
     package whose every member has a constant width, within the net width limit; `elem_t` a \
     packed struct with no sign keyword, named by its own declaration (not a typedef alias of \
     it), bare for one declared at the top of this package or as `pkg::name` for one declared \
     at the top of `pkg`, no name escaped, whose members are each `logic`, `logic [N:0]` with \
     N >= 0, the bare name of a `typedef logic [D:0]` declared at the top of the element's \
     package (D a decimal literal) that no packed struct or union in `$unit` or a \
     wildcard-imported package also names, or a `struct packed` with no sign keyword of one or \
     more such `logic` and `logic [N:0]` members, anonymous or named bare by its own \
     declaration at the top of the element's package (any other `elem_t [hi:lo] m` is \
     unsupported in v1)";
/// A struct holding such a member used as a member of another type.
const REFUSE_NESTED: &str = "a member type with no packed-array member of its own (a struct \
     holding `elem_t [hi:0] m` as a member of another type is unsupported in v1)";
/// An assignment pattern onto such a struct.
const REFUSE_PATTERN: &str = "an assignment target with no packed-array struct member (an \
     assignment pattern onto a struct holding `elem_t [hi:0] m` is unsupported in v1; assign \
     its members one by one)";
/// An index, or a place for it, the rule does not open.
const REFUSE_INDEX: &str = "a literal index in range, a `for (int i ...)` index in always_comb, \
     or a genvar index in a continuous assign, on a packed-array struct member (this select of \
     `s.m[i]` is unsupported in v1)";
/// The alias, return and formal spellings outside the rule.
const REFUSE_FORM: &str = "a package-level `typedef elem_t [hi:0] name;`, hi >= 0 a constant, \
     of an `elem_t` a packed-array struct member admits; an ANSI port `pkg::name p` of such an \
     alias with no dimension or default after its name; or a function return, or a function or \
     task ANSI formal written `input`, `v_t [l:r]` in this package, `l` and `r` integer \
     literals or `+`, `-`, `*` of them, `v_t` the bare name of a `typedef logic [D:0]` declared \
     at the top of this package (D a decimal literal) that no packed struct or union in `$unit` \
     or a wildcard-imported package also names (this packed dimension on a named type is \
     unsupported in v1)";

/// The largest flat offset a member can reach (`limits::MAX_NET_WIDTH`).
const MAX_FLAT_BITS: u64 = 1 << 20;

/// An element select, resolved as far as the layout goes.
struct ElementSelect {
    base: HierPath,
    /// `(element count, element width, member offset)`.
    array: (u32, u32, u32),
    /// `(offset, width)` of the chain leaf inside the element; `None`: the whole element.
    leaf: Option<(u32, u32)>,
}

/// A decimal literal as written, and nothing that folds to one.
fn decimal_literal(e: &Expr) -> Option<i64> {
    match &e.kind {
        ExprKind::IntLit {
            kind: IntLitKind::Decimal,
            ..
        } => Parser::const_lit(e),
        _ => None,
    }
}

/// `decimal_literal`, within `int`.
fn index_literal(e: &Expr) -> bool {
    decimal_literal(e).is_some_and(|v| v <= i64::from(i32::MAX))
}

/// A one-segment name.
fn index_name(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Ident(p) if p.segments.len() == 1 => Some(&p.segments[0].name),
        _ => None,
    }
}

/// The `L*V` shape's `V`.
fn scaled_name(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Binary {
            op: BinOp::Mul,
            lhs,
            rhs,
        } if index_literal(lhs) => index_name(rhs),
        _ => None,
    }
}

/// The `V` of the index shapes the guard is measured on: `V`, `L*V`, `L*V+L`, `L-V`,
/// `V+L`, each `L` a decimal literal within `int`. Each of these has one self-determined
/// width, the 32 bits of `V` and of its literals, which is the width the guard's own
/// literals put it in, so no operand is extended and guard and offset read one value.
fn index_var(e: &Expr) -> Option<&str> {
    let ExprKind::Binary { op, lhs, rhs } = &e.kind else {
        return index_name(e);
    };
    match op {
        BinOp::Mul => scaled_name(e),
        BinOp::Sub if index_literal(lhs) => index_name(rhs),
        BinOp::Add if index_literal(rhs) => index_name(lhs).or_else(|| scaled_name(lhs)),
        _ => None,
    }
}

impl Parser<'_, '_> {
    /// How the member type at the cursor is written, read before it is parsed: its
    /// keyword, or the site of the declaration the name it spells points at. A sign
    /// keyword after a keyword or after `struct packed` is another form.
    pub(crate) fn member_form_ahead(&self) -> FormType {
        let sign_at = |n: usize| matches!(self.text_at(n), "signed" | "unsigned");
        if let Some(kind) = self.net_var_kind() {
            return if sign_at(1) {
                FormType::Other
            } else {
                FormType::Keyword(kind)
            };
        }
        if self.peek_typedef_name().is_none() {
            return if sign_at(2) {
                FormType::Other
            } else {
                FormType::Anonymous
            };
        }
        let nm = self.type_name_key();
        if self.site_here(&nm, SiteForm::PageVector) {
            FormType::PageVector
        } else if self.site_here(&nm, SiteForm::Struct) && self.one_declaration(&nm).is_some() {
            FormType::Struct
        } else {
            FormType::Other
        }
    }

    /// §5.2 row 41: record where the declaration `key` of `form`, named `name`, was
    /// written: in a container, outside a body, under an unescaped name (`decl_sites`).
    pub(crate) fn note_decl_site(&mut self, key: DeclKey, name: &str, form: SiteForm) {
        if let Some(container) = self.cur_container {
            if !self.in_body_typedef && !name.starts_with('\\') {
                let name = name.to_string();
                self.decl_sites.insert(
                    key,
                    DeclSite {
                        container,
                        name,
                        form,
                    },
                );
            }
        }
    }

    /// `nm`'s typedef slot points at a declaration of `form` written in this container
    /// under the name `nm` itself (a scoped `p::n` is never a declared name).
    fn site_here(&self, nm: &str, form: SiteForm) -> bool {
        self.td_slot(nm)
            .and_then(|t| self.decl_sites.get(&t.key))
            .is_some_and(|s| {
                s.form == form && Some(s.container) == self.cur_container && s.name == nm
            })
    }

    /// `p::n`: the twin of a declaration of `form` written as `n`
    /// (`register_pkg_typedef_twin` copies p's own typedefs only).
    fn site_scoped(&self, nm: &str, form: SiteForm) -> bool {
        let Some((_, n)) = nm.split_once("::") else {
            return false;
        };
        self.td_slot(nm)
            .and_then(|t| self.decl_sites.get(&t.key))
            .is_some_and(|s| s.form == form && s.name == n)
    }

    /// `nm`'s packed slot is the layout its typedef slot's declaration laid out: a typedef
    /// of another kind over the name leaves the packed slot behind (§5.2 row 41 P1).
    fn one_declaration(&self, nm: &str) -> Option<&StructLayout> {
        let l = self.packed_layout(nm)?;
        (self.td_slot(nm)?.layout_reg == Some(l.reg)).then_some(l)
    }

    /// The element `k` is of the form the pages declare, and nothing else, each fact one
    /// its own declaration recorded: a packed struct whose layout records a form per member
    /// (a union and a record record none), with at least one member (`struct packed { }` is
    /// not a declaration, IEEE 1800 Syntax 7-1, and an empty list is no fact), holding no
    /// `elem_t [hi:0]` member, each member
    /// unsigned, 4-state, written with one descending `[N:0]` dimension or none, and
    ///
    /// * written `logic`; or
    /// * in the element itself, the bare name of a `typedef logic [D:0]` this container
    ///   declared; or
    /// * in the element itself, not deeper, a packed struct of `logic` members,
    ///   anonymous, or named bare and declared in this container.
    ///
    /// Any other form -- a type parameter, an enum, a chained, imported, scoped, escaped or
    /// `$unit` name, `reg`, `bit`, a sign keyword, a second dimension -- is refused, and no
    /// flag is read for it.
    fn element_admitted(&self, k: DeclKey, top: bool) -> bool {
        let Some(l) = self.decl_packed(k) else {
            return false;
        };
        if !l.arrays.is_empty() || l.fields.is_empty() {
            return false;
        }
        l.fields.iter().enumerate().all(|(i, f)| {
            let Some(form) = l.forms.get(i) else {
                return false;
            };
            let plain = !f.3 && !f.4 && !f.5 && f.6 == 0 && form.dims <= 1;
            plain
                && match (form.ty, &f.8) {
                    (FormType::Keyword(NetVarKind::Logic), None) => true,
                    (FormType::PageVector, None) => top,
                    (FormType::Struct | FormType::Anonymous, Some(t)) => {
                        top && self.element_admitted(t.key, false)
                    }
                    _ => false,
                }
        })
    }

    /// `(hi, W, element)` when `nm` with the dimension `r` is the type the rule opens.
    fn composed_shape(&self, nm: &str, info: &TypeInfo, r: &Range) -> Option<(u32, u32, TyRef)> {
        if info.signed
            || info.range.is_none()
            || !info.packed.is_empty()
            || !info.unpacked.is_empty()
        {
            return None;
        }
        let l = self.one_declaration(nm)?;
        if !(self.site_here(nm, SiteForm::Struct) || self.site_scoped(nm, SiteForm::Struct)) {
            return None;
        }
        let key = DeclKey(l.reg);
        if !self.element_admitted(key, true) {
            return None;
        }
        let w: u64 = l.fields.iter().map(|f| u64::from(f.2)).sum();
        let hi = self.const_bound(&r.msb)?;
        let lo = self.const_bound(&r.lsb)?;
        if w == 0 || lo != 0 || !(0..=i64::from(i32::MAX)).contains(&hi) {
            return None;
        }
        let count = hi as u64 + 1;
        if count.checked_mul(w)? > MAX_FLAT_BITS {
            return None;
        }
        let elem = TyRef {
            name: self.stable_type_key(nm),
            key,
        };
        Some((hi as u32, w as u32, elem))
    }

    /// The member `nm [hi:0]`, cursor on `[`, as its keyword twin. The refusal consumes
    /// the dimensions and answers with the element's own type, so it stands alone.
    pub(crate) fn composed_member_type(&mut self, nm: &str, info: &TypeInfo) -> MemberType {
        let dims = self.opt_packed_dims();
        let shape = match dims.as_slice() {
            [r] => self.composed_shape(nm, info, r),
            _ => None,
        };
        let Some((hi, w, elem)) = shape else {
            if !self.is_nested_holder(nm) {
                self.error(REFUSE_MEMBER);
            }
            return (
                info.kind,
                info.signed,
                info.range.clone(),
                Vec::new(),
                None,
                None,
                false,
            );
        };
        self.pending_member_array = Some((elem, hi + 1));
        (
            NetVarKind::Logic,
            false,
            Some(Self::dec_range(hi)),
            vec![Self::dec_range(w - 1)],
            None,
            None,
            false,
        )
    }

    fn is_nested_holder(&self, nm: &str) -> bool {
        self.packed_layout(nm).is_some_and(|l| !l.arrays.is_empty())
    }

    /// No struct holding a packed-array member nests in another type: the chain walk
    /// has no element stride to add.
    pub(crate) fn refuse_nested_holder(&mut self, nm: &str) {
        if self.is_nested_holder(nm) {
            self.error(REFUSE_NESTED);
        }
    }

    /// A per-instance layout has no flat offset for an element.
    pub(crate) fn refuse_symbolic_holder(&mut self, arrays: &[MemberArray], at: Span) {
        if !arrays.is_empty() {
            self.error_at(at, REFUSE_MEMBER);
        }
    }

    pub(crate) fn refuse_holder_pattern(&mut self, at: Span) {
        self.error_at(at, REFUSE_PATTERN);
    }

    /// `typedef elem_t [hi:0] name;`, cursor on `[`. Not a typedef: a variable of the
    /// type would read the element's fields at element 0 through a mirrored slot, so
    /// the name enters no type table, only `composed_aliases`, read by an ANSI port
    /// and a package parameter.
    pub(crate) fn parse_composed_alias(
        &mut self,
        start: Span,
        base: &str,
        info: &TypeInfo,
    ) -> Option<ModuleItem> {
        let dims = self.opt_packed_dims();
        let shape = match dims.as_slice() {
            [r] => self.composed_shape(base, info, r),
            _ => None,
        };
        let Some((hi, w, elem)) = shape else {
            self.error(REFUSE_FORM);
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        };
        let tname = self.ident()?;
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(REFUSE_FORM);
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        self.expect(TokenKind::Semi, "';'");
        let (outer, elem_range) = (Self::dec_range(hi), Self::dec_range(w - 1));
        self.composed_aliases.insert(
            tname.name.clone(),
            ComposedAlias {
                outer: outer.clone(),
                elem: elem_range.clone(),
                elem_key: elem.name,
            },
        );
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Alias {
                dtype: hdl_ast::DataType::new_net(
                    NetVarKind::Logic,
                    false,
                    Some(outer),
                    vec![elem_range],
                ),
            },
            span: start.to(self.prev_span()),
        }))
    }

    /// At `endpackage`: this package's aliases take their `pkg::` key, as its types
    /// take their twins, and the bare key leaves with the package.
    pub(crate) fn rekey_composed_aliases(&mut self, pkg: &str) {
        let bare: Vec<String> = self
            .composed_aliases
            .keys()
            .filter(|k| !k.contains("::"))
            .cloned()
            .collect();
        for k in bare {
            let Some(mut a) = self.composed_aliases.remove(&k) else {
                continue;
            };
            if !a.elem_key.contains("::") {
                a.elem_key = format!("{pkg}::{}", a.elem_key);
            }
            self.composed_aliases.insert(format!("{pkg}::{k}"), a);
        }
    }

    /// An ANSI port typed `pkg::alias name`: the tuple `try_port_typedef` builds for
    /// its twin `pkg::elem [hi:0] name`.
    #[allow(clippy::type_complexity)]
    pub(crate) fn try_alias_port(
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
        if !self.is_ident() || self.peek_at(1) != Some(TokenKind::ColonColon) {
            return None;
        }
        let key = format!("{}::{}", self.cur_text(), self.text_at(2));
        let a = self.composed_aliases.get(&key)?.clone();
        if !matches!(
            self.peek_at(3),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            return None;
        }
        self.pos += 3;
        Some((
            NetVarKind::Logic,
            false,
            Some(a.elem),
            Some(a.elem_key),
            vec![a.outer],
            Vec::new(),
            Vec::new(),
        ))
    }

    /// What follows an alias port's name, itself or a bare continuation: nothing. A
    /// default would be dropped on an unconnected port, and a dimension has no twin.
    pub(crate) fn refuse_alias_port_tail(&mut self, alias: bool, unpacked: bool) {
        if alias
            && (unpacked
                || !matches!(
                    self.peek(),
                    Some(TokenKind::Comma) | Some(TokenKind::RParen)
                ))
        {
            self.error(REFUSE_FORM);
        }
    }

    /// `parameter alias P`, inside the alias's package: the type its keyword twin
    /// `logic [hi:0][W-1:0]` would give, for `typedef_param_shape`.
    pub(crate) fn peek_alias_param(&self) -> Option<TypeInfo> {
        if !self.in_package || !self.is_ident() || self.peek_typedef_name().is_some() {
            return None;
        }
        let a = self.composed_aliases.get(self.cur_text())?;
        if !matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            return None;
        }
        Some(TypeInfo {
            // The same kind as `kind` below: every composed alias the parser admits has
            // `logic` members (a `bit` or `logic signed` member is refused above).
            decl_kind: TypeKind::Kind(NetVarKind::Logic),
            kind: NetVarKind::Logic,
            signed: false,
            range: Some(a.outer.clone()),
            packed: vec![a.elem.clone()],
            class_name: None,
            unpacked: Vec::new(),
            shape_param: None,
            enum_type: false,
            layout_exact: false,
            pattern_members: None,
            layout_reg: None,
            bind: 0,
        })
    }

    /// A vector base the return and the formal open: the bare name of a `typedef logic
    /// [D:0]` this container declared (`decl_sites`), no struct or symbolic layout under
    /// the name. The site's writer checked the kind, sign, range, packed and unpacked dims.
    fn plain_vector_base(&self, nm: &str) -> bool {
        self.site_here(nm, SiteForm::PageVector)
            && self.packed_layout(nm).is_none()
            && self.sym_layout(nm).is_none()
    }

    /// In a package, cursor on `T [hi:lo]`: consume it. `Some((outer, own))` when `T`
    /// is a plain vector and one literal dimension stands before a name, else the
    /// refusal, with `None` in place of the dimensions.
    fn composed_vector_type(&mut self) -> Option<(Range, Range)> {
        let info = self.peek_typedef_name()?;
        let nm = self.type_name_key();
        self.eat_scope_qualifier();
        self.bump(); // the type name
        let dims = self.opt_packed_dims();
        let named = matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        );
        match (dims.as_slice(), &info.range) {
            ([outer], Some(own))
                if named
                    && self.plain_vector_base(&nm)
                    && self.bounds_source_literal(Some(outer), &[]) =>
            {
                Some((outer.clone(), own.clone()))
            }
            _ => {
                self.error(REFUSE_FORM);
                None
            }
        }
    }

    /// In a package, the cursor stands on a typedef name with a dimension after it.
    fn at_composed_vector(&self) -> bool {
        self.in_package
            && self.peek_typedef_name().is_some()
            && self.peek_at(self.scope_qualifier_len() + 1) == Some(TokenKind::LBracket)
    }

    /// `function T [hi:lo] f`: the `(range, ret_md)` of its keyword twin.
    pub(crate) fn try_composed_return(&mut self) -> Option<(Option<Range>, Vec<Range>)> {
        if !self.at_composed_vector() {
            return None;
        }
        Some(match self.composed_vector_type() {
            Some((outer, own)) => (Some(outer), vec![own]),
            None => (None, Vec::new()),
        })
    }

    /// `input T [hi:lo] x`, ANSI only: the `(range, md_dims)` of its keyword twin.
    pub(crate) fn try_composed_formal(
        &mut self,
        dir: PortDir,
        written: bool,
    ) -> Option<(Option<Range>, Vec<Range>)> {
        if !self.at_composed_vector() {
            return None;
        }
        let shape = self.composed_vector_type();
        if !written || dir != PortDir::Input {
            if shape.is_some() {
                self.error(REFUSE_FORM);
            }
            return Some((None, Vec::new()));
        }
        Some(match shape {
            Some((outer, own)) => (Some(outer), vec![own]),
            None => (None, Vec::new()),
        })
    }

    /// `path[` names an `elem_t [hi:0]` member.
    pub(crate) fn composed_member_ahead(&self, path: &HierPath) -> bool {
        let [var, member] = path.segments.as_slice() else {
            return false;
        };
        self.var_struct
            .get(&var.name)
            .and_then(|ty| self.decl_packed(ty.key))
            .is_some_and(|l| l.member_array(&member.name).is_some())
    }

    /// As `composed_member_ahead`, for a write: `s.m[i] = x`, a whole element, stays on
    /// the path of the keyword twin, and so does its text; what follows the matching
    /// `]` says which one this is. The cursor does not move.
    pub(crate) fn composed_chain_ahead(&self, path: &HierPath) -> bool {
        if !self.composed_member_ahead(path) {
            return false;
        }
        let mut depth = 0usize;
        let mut n = 0;
        loop {
            match self.peek_at(n) {
                Some(TokenKind::LBracket) => depth += 1,
                Some(TokenKind::RBracket) => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(
                            self.peek_at(n + 1),
                            Some(TokenKind::Dot) | Some(TokenKind::LBracket)
                        );
                    }
                }
                None => return false,
                _ => {}
            }
            n += 1;
        }
    }

    /// Past a refused select: to the end of its index, then of its chain.
    fn skip_select_tail(&mut self, mut depth: usize) {
        loop {
            match self.peek() {
                None => return,
                Some(TokenKind::LBracket) => depth += 1,
                Some(TokenKind::RBracket) if depth > 0 => depth -= 1,
                Some(TokenKind::Dot)
                    if depth == 0 && self.peek_at(1) == Some(TokenKind::Word(WordKind::Ident)) =>
                {
                    self.bump();
                }
                _ if depth == 0 => return,
                _ => {}
            }
            self.bump();
        }
    }

    /// `[e]`. Its own frame: this is the edge expression parsing recurses through.
    #[inline(never)]
    fn composed_index(&mut self) -> Option<Box<Expr>> {
        self.bump(); // '['
        let e = Box::new(self.expr(0));
        if self.peek() == Some(TokenKind::RBracket) {
            self.bump();
            return Some(e);
        }
        self.error(REFUSE_INDEX);
        self.skip_select_tail(1);
        None
    }

    /// The rest of a select, cursor past `]`: the member's numbers and the chain.
    #[inline(never)]
    fn composed_chain(&mut self, path: &HierPath) -> Option<ElementSelect> {
        let (var, member) = (&path.segments[0], &path.segments[1]);
        let ty = self.var_struct.get(&var.name)?;
        let l = self.decl_packed(ty.key)?;
        let total: u64 = l.fields.iter().map(|f| u64::from(f.2)).sum();
        let (off, flat, ..) = l.field(&member.name)?;
        let a = l.member_array(&member.name)?;
        let (count, elem) = (a.count, a.elem.clone());
        if total > MAX_FLAT_BITS || !self.struct_scalar_vars.contains(&var.name) {
            self.error(REFUSE_INDEX);
            self.skip_select_tail(0);
            return None;
        }
        let leaf = if self.peek() == Some(TokenKind::Dot) {
            self.bump(); // '.'
            let name = self.ident()?;
            let el = self.decl_packed(elem.key)?;
            let Some(g) = el.field(&name.name) else {
                self.error_at(name.span, "a member of the nested struct in a member chain");
                self.skip_select_tail(0);
                return None;
            };
            let nested = el.nested_of(&name.name).cloned();
            let ((o, w, ..), _) = self.extend_member_chain(g, nested);
            Some((o, w))
        } else {
            None
        };
        if matches!(
            self.peek(),
            Some(TokenKind::Dot) | Some(TokenKind::LBracket)
        ) {
            self.error(REFUSE_INDEX);
            self.skip_select_tail(0);
            return None;
        }
        Some(ElementSelect {
            base: HierPath {
                segments: vec![var.clone()],
                span: var.span,
            },
            array: (count, flat / count, off),
            leaf,
        })
    }

    /// An index the literal arm takes: a decimal in `[0, count)`.
    fn literal_element(e: &Expr, count: u32) -> Option<u32> {
        decimal_literal(e)
            .filter(|v| (0..i64::from(count)).contains(v))
            .map(|v| v as u32)
    }

    /// The run-time index `e` in the place the cursor stands, on variable `var`.
    fn rt_index_admitted(&self, e: &Expr, var: &str) -> bool {
        let Some(v) = index_var(e) else {
            return false;
        };
        let bound = self.idx_scope.iter().rev().find(|(n, _)| n == v);
        match (bound.map(|b| b.1), self.module_struct_vars.get(var)) {
            (Some(IdxKind::ForInt), Some(StructVarKind::Var)) => self.in_always_comb,
            (Some(IdxKind::Genvar), Some(_)) => self.in_cont_assign,
            _ => false,
        }
    }

    /// `e*W + L` under its bound check, the sentinel past it: the flat offset of an
    /// element leaf, with the parentheses of its written form.
    fn guarded_offset(e: &Expr, count: u32, stride: u32, l: u32) -> Expr {
        let span = e.span;
        let lit = |v| Self::dec_lit(v, span);
        let paren = |x: Expr| Expr {
            kind: ExprKind::Paren { inner: Box::new(x) },
            span,
        };
        let cond = mk_bin(
            BinOp::LogAnd,
            mk_bin(BinOp::Ge, paren(e.clone()), lit(0)),
            mk_bin(BinOp::Lt, paren(e.clone()), lit(count)),
        );
        let flat = mk_bin(
            BinOp::Add,
            mk_bin(BinOp::Mul, paren(e.clone()), lit(stride)),
            lit(l),
        );
        Expr {
            kind: ExprKind::Ternary {
                cond: Box::new(paren(cond)),
                then_e: Box::new(flat),
                else_e: Box::new(lit(OOB_DROP_BIT)),
            },
            span,
        }
    }

    /// `(L, w)`: the leaf's offset past the element's, and its width.
    fn leaf_geom(sel: &ElementSelect) -> (u32, u32) {
        let (_, elem_w, member) = sel.array;
        let (o, w) = sel.leaf.unwrap_or((0, elem_w));
        (member + o, w)
    }

    /// A read of `s.m[e]…`, cursor on `[`.
    #[inline(never)]
    pub(crate) fn composed_member_read(&mut self, path: &HierPath) -> Expr {
        let Some(e) = self.composed_index() else {
            return Self::error_expr(path.span);
        };
        self.composed_read_tail(path, e)
    }

    #[inline(never)]
    fn composed_read_tail(&mut self, path: &HierPath, e: Box<Expr>) -> Expr {
        let Some(sel) = self.composed_chain(path) else {
            return Self::error_expr(path.span);
        };
        let span = path.span.to(self.prev_span());
        let (count, stride, _) = sel.array;
        let (l, w) = Self::leaf_geom(&sel);
        let var = Expr {
            span,
            kind: ExprKind::Ident(sel.base),
        };
        if let Some(i) = Self::literal_element(&e, count) {
            let lo = l + i * stride;
            return Expr {
                kind: ExprKind::PartSelect {
                    base: Box::new(var),
                    msb: Box::new(Self::dec_lit(lo + w - 1, span)),
                    lsb: Box::new(Self::dec_lit(lo, span)),
                },
                span,
            };
        }
        if !self.rt_index_admitted(&e, &path.segments[0].name) {
            self.error_at(span, REFUSE_INDEX);
            return Self::error_expr(span);
        }
        Expr {
            kind: ExprKind::IndexedPart {
                base: Box::new(var),
                offset: Box::new(Self::guarded_offset(&e, count, stride, l)),
                width: Box::new(Self::dec_lit(w, span)),
                dir: PartDir::PlusColon,
            },
            span,
        }
    }

    /// A write to `s.m[e].f`, cursor on `[`. `site`: the assignment form armed it, the
    /// one-shot flag `parse_lvalue` took.
    #[inline(never)]
    pub(crate) fn composed_member_lval(&mut self, path: &HierPath, site: bool) -> Lvalue {
        let Some(e) = self.composed_index() else {
            return Lvalue::Error(path.span);
        };
        let Some(sel) = self.composed_chain(path) else {
            return Lvalue::Error(path.span);
        };
        let span = path.span.to(self.prev_span());
        let (count, stride, _) = sel.array;
        let (l, w) = Self::leaf_geom(&sel);
        if let Some(i) = Self::literal_element(&e, count) {
            let lo = l + i * stride;
            return Lvalue::PartSelect {
                base: Box::new(Lvalue::Ident(sel.base)),
                msb: Box::new(Self::dec_lit(lo + w - 1, span)),
                lsb: Box::new(Self::dec_lit(lo, span)),
                span,
            };
        }
        let var = &path.segments[0].name;
        // A write beyond a plain blocking `=` reads the target or moves the write,
        // and a port's driver stands outside this module.
        let plain = site
            && self.peek() == Some(TokenKind::Eq)
            && !matches!(self.peek_at(1), Some(TokenKind::Hash) | Some(TokenKind::At))
            && !self.at_kw_at(1, Kw::Repeat)
            && self.module_struct_vars.get(var.as_str()) == Some(&StructVarKind::Var);
        if !plain || !self.rt_index_admitted(&e, var) {
            self.error_at(span, REFUSE_INDEX);
            return Lvalue::Error(span);
        }
        Lvalue::IndexedPart {
            base: Box::new(Lvalue::Ident(sel.base)),
            offset: Box::new(Self::guarded_offset(&e, count, stride, l)),
            width: Box::new(Self::dec_lit(w, span)),
            dir: PartDir::PlusColon,
            span,
        }
    }

    fn at_kw_at(&self, n: usize, kw: Kw) -> bool {
        matches!(
            self.peek_at(n),
            Some(TokenKind::Word(WordKind::Keyword(k))) if k == kw
        )
    }

    fn error_expr(span: Span) -> Expr {
        Expr {
            kind: ExprKind::Error,
            span,
        }
    }

    /// A declared name hides the loop name it shadows, and a name that is no longer
    /// the struct variable is no base.
    pub(crate) fn mask_composed_index_name(&mut self, n: &str) {
        if let Some(e) = self.idx_scope.iter_mut().rev().find(|(name, _)| name == n) {
            e.1 = IdxKind::Other;
        }
        self.module_struct_vars.remove(n);
    }

    /// A module-item declaration of struct variables, outside any generate block.
    pub(crate) fn note_module_struct_vars(&mut self, d: Option<&NetVarDecl>) {
        let Some(d) = d else { return };
        if self.gen_depth != 0 || !d.dtype.packed_dims().is_empty() {
            return;
        }
        for n in d.names.iter().filter(|n| n.unpacked.is_empty()) {
            if self.var_struct.contains_key(&n.name.name) {
                self.module_struct_vars
                    .insert(n.name.name.clone(), StructVarKind::Var);
            }
        }
    }

    /// An ANSI struct port, scalar or not.
    pub(crate) fn note_struct_port(&mut self, name: &str, scalar: bool) {
        if scalar {
            self.module_struct_vars
                .insert(name.to_string(), StructVarKind::Port);
        } else {
            self.module_struct_vars.remove(name);
        }
    }
}
