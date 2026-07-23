//! struct declarations — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Parse a packed struct/union MEMBER's type into `(kind, signed, range)`. A
    /// built-in keyword resolves directly (§7.2.1); a SIMPLE user-defined type name
    /// (a vector / enum / atom typedef) resolves to its `TypeInfo`. A nested
    /// struct/union, a class handle, or a multi-dim packed typedef member needs
    /// nested-layout machinery not in v1 — honest-loud. Returns `None` (with the
    /// error already emitted) on a non-type token or an unsupported member type; the
    /// caller breaks out of the member loop.
    pub(crate) fn parse_struct_member_type(
        &mut self,
    ) -> Option<(NetVarKind, bool, Option<Range>, Vec<Range>)> {
        if let Some(kind) = self.net_var_kind() {
            self.bump(); // kind keyword
            let signed = self.signed_eff(Some(kind));
            let range = self.opt_range();
            self.reject_packed_dims_on_nonvector(kind, range.is_some());
            // Multi-dimensional PACKED member (`logic [1:0][3:0] m`): collect the INNER
            // packed dims after the first range. Each is a constant `[a:b]` range; the
            // member's flat width folds them in and a first-level `m[i]` selects one
            // ∏(inner)-bit element (see `parse_struct_field_sel`). Empty for the common
            // single-dim member, so that path is byte-identical.
            let mut packed_dims = Vec::new();
            while self.peek() == Some(TokenKind::LBracket) {
                match self.opt_range() {
                    Some(d) => packed_dims.push(d),
                    None => break,
                }
            }
            return Some((kind, signed, range, packed_dims));
        }
        if let Some(info) = self.peek_typedef_name() {
            let nm = self.type_name_key();
            if self.struct_layouts.contains_key(&nm)
                || info.class_name.is_some()
                || !info.packed.is_empty()
            {
                self.error(
                    "a simple (non-struct) type for a struct/union member (a nested struct / class / multi-dim packed member is unsupported in v1)",
                );
                return None;
            }
            self.eat_scope_qualifier();
            self.bump(); // the typedef-name token
            return Some((info.kind, info.signed, info.range, Vec::new()));
        }
        self.error("a net/var type in a struct/union member");
        None
    }

    /// `typedef struct packed { <type> f1, f2; … } name;` (Phase-2). Members are
    /// laid out MSB-first into one flat `logic [W-1:0]` vector; the layout is
    /// recorded so `name var;` resolves and `var.field` desugars to a part-select.
    /// `start` is the span of the leading `typedef` keyword (already consumed).
    /// Parse `{ <type> f1, f2; … }` — the shared member list of a packed OR
    /// unpacked `typedef struct`. Cursor at `{`; consumes through the closing `}`.
    pub(crate) fn parse_struct_member_list(&mut self) -> Option<Vec<StructMember>> {
        self.expect(TokenKind::LBrace, "'{' for struct body");
        let mut members = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let m_start = self.cur_span();
            let Some((kind, signed, range, packed_dims)) = self.parse_struct_member_type() else {
                break;
            };
            loop {
                let Some(name) = self.ident() else { break };
                members.push(StructMember {
                    name,
                    kind,
                    signed,
                    range: range.clone(),
                    packed_dims: packed_dims.clone(),
                    span: m_start.to(self.prev_span()),
                });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Semi, "';'");
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close struct body");
        Some(members)
    }

    /// Width of a struct member from its range. `None` ⇒ scalar (1). Only
    /// constant-literal bounds fold (`[7:0]`, `[8-1:0]`); param widths return `None`.
    pub(crate) fn member_width(&self, range: &Option<Range>) -> Option<u32> {
        match range {
            None => Some(1),
            Some(r) => {
                let msb = Self::const_lit(&r.msb)?;
                let lsb = Self::const_lit(&r.lsb)?;
                Some(msb.abs_diff(lsb) as u32 + 1)
            }
        }
    }

    /// Is this member kind a 2-state type (`bit`/`byte`/`shortint`/`int`/`longint`)?
    /// IEEE §7.2.1: a packed struct is 2-state iff EVERY member is 2-state — then
    /// it defaults to 0, not X. `integer`/`time`/`logic`/`reg`/nets are 4-state.
    /// Mirrors `elaborate::net_kind_is_two_state` exactly (the engine's default-fill
    /// reads the same predicate via the `two_state_nets` sidecar).
    pub(crate) fn member_kind_two_state(kind: NetVarKind) -> bool {
        matches!(
            kind,
            NetVarKind::Bit
                | NetVarKind::Byte
                | NetVarKind::Shortint
                | NetVarKind::Int
                | NetVarKind::Longint
        )
    }

    /// Fixed bit-width of a named integer-atom type used as a struct/union member,
    /// or `None` for a vector-capable kind (`bit`/`logic`/`reg`/nets) whose width
    /// is given by the range instead (`member_width`). The atom types carry an
    /// implicit width that no `[msb:lsb]` range follows, so a member declared with
    /// a bare named type (`int a;`) must size from the type — NOT default to 1.
    /// Mirrors the SVA-local-var atom table (`parse_sva_local_decl`) and §6.11.
    pub(crate) fn atom_member_width(kind: NetVarKind) -> Option<u32> {
        match kind {
            NetVarKind::Byte => Some(8),
            NetVarKind::Shortint => Some(16),
            NetVarKind::Int | NetVarKind::Integer => Some(32),
            NetVarKind::Longint | NetVarKind::Time => Some(64),
            _ => None,
        }
    }

    /// Width of a struct/union member from its declared kind AND range. A named
    /// integer-atom kind (`int`/`byte`/…) carries a fixed width (the range, if any,
    /// is not a packed dimension on an atom in this subset); a vector-capable kind
    /// (`bit`/`logic`/`reg`) sizes from the range (`None` ⇒ 1). Returns the width
    /// or `None` when a vector range is present but non-constant.
    pub(crate) fn member_width_kind(&self, kind: NetVarKind, range: &Option<Range>) -> Option<u32> {
        if let Some(w) = Self::atom_member_width(kind) {
            return Some(w);
        }
        self.member_width(range)
    }

    /// The FLAT bit width and first-level element STRIDE of a (possibly
    /// multi-dimensional) packed struct/union member. `flat = base_width ×
    /// ∏(packed_dims widths)`; `elem_stride = ∏(packed_dims widths)` — the width of
    /// ONE `m[i]` element. For a single-dim member `packed_dims` is empty, so
    /// `elem_stride == 1` and `flat == base_width` (byte-identical to before). None
    /// (→ loud) if the base kind/range or any inner dim has no constant width.
    pub(crate) fn member_flat_dims(
        &self,
        kind: NetVarKind,
        range: &Option<Range>,
        packed_dims: &[Range],
    ) -> Option<(u32, u32)> {
        let base = self.member_width_kind(kind, range)?;
        let mut stride: u32 = 1;
        for d in packed_dims {
            let w = self.member_width(&Some(d.clone()))?;
            if w == 0 {
                return None;
            }
            stride = stride.checked_mul(w)?;
        }
        let flat = base.checked_mul(stride)?;
        Some((flat, stride))
    }

    /// N3: is a struct member kind a genuine bit-vector (packable into a flat record
    /// value)? Excludes `string`/`real`/`realtime`/`event`/class-handle/virtual-
    /// interface — those have no fixed bit width, so `member_width_kind` would give
    /// them a bogus 1-bit default; a record containing one is NOT packable (→ loud).
    pub(crate) fn member_kind_is_integral(kind: NetVarKind) -> bool {
        matches!(
            kind,
            NetVarKind::Reg
                | NetVarKind::Logic
                | NetVarKind::Integer
                | NetVarKind::Time
                | NetVarKind::Bit
                | NetVarKind::Byte
                | NetVarKind::Shortint
                | NetVarKind::Int
                | NetVarKind::Longint
        )
    }

    /// N3 Phase 3: a record member kind that a SoA record array can carry — an
    /// integral (→ int/logic dyn array), a `string` (→ string dyn array), or a
    /// `real`/`realtime` (→ real dyn array). A nested struct / event / class-handle
    /// member has no per-field dyn-array representation → the record array stays loud.
    pub(crate) fn member_kind_soa_ok(kind: NetVarKind) -> bool {
        Self::member_kind_is_integral(kind)
            || matches!(
                kind,
                NetVarKind::String | NetVarKind::Real | NetVarKind::Realtime
            )
    }

    /// Round-9: the collision-free member-net name for `var.field` of a scalar
    /// unpacked struct. Prefixed with `$` — a SV SIMPLE identifier cannot BEGIN
    /// with `$` (only `[a-zA-Z_]…`), so this never collides with a user variable
    /// (mirrors the `$blk$`/`$func$` internal-name convention). The desugar
    /// refuses a `$` in the VARIABLE name (see `parse_unpacked_struct_decl`), so
    /// the first `$` after the `$unp$` prefix unambiguously delimits var from
    /// field → the (var, field) → name map is injective (distinct accesses never
    /// alias). A `$` in a FIELD name is fine (it lands entirely after that first
    /// separator).
    pub(crate) fn unpacked_member_net(var: &str, field: &str) -> String {
        format!("$unp${var}${field}")
    }

    /// R5: expand one unpacked-struct tf-port `<dir> rec_t r` into its N member
    /// formals `$unp$r$field`, each carrying the member's OWN kind/signed/range and
    /// the port's direction. A heterogeneous record (e.g. `string` + `int`) cannot
    /// ride a single flat vector the way a PACKED struct tf-port does, so — exactly
    /// like `parse_unpacked_struct_decl` does for a local `rec_t r;` — it desugars to
    /// one scalar member net per field. Registers `var_unpacked_struct[r]` so the
    /// body's `r.field` resolves to `$unp$r$field` (that same map also drives the
    /// call-site actual expansion, `expand_struct_call_args`). Returns `[]` (after a
    /// loud diagnostic) if the port name is empty or contains `$` (which would break
    /// the `$unp$<var>$<field>` mangle's injectivity).
    pub(crate) fn unpacked_struct_member_ports(
        &mut self,
        port: &TfPort,
        tyname: &str,
    ) -> Vec<TfPort> {
        if port.name.name.is_empty() {
            return Vec::new();
        }
        if port.name.name.contains('$') {
            self.error_at(
                port.name.span,
                "an unpacked-struct tf-port name containing `$` is unsupported in v1",
            );
            return Vec::new();
        }
        let Some(members) = self.unpacked_struct_layouts.get(tyname).cloned() else {
            return Vec::new();
        };
        self.var_unpacked_struct
            .insert(port.name.name.clone(), tyname.to_string());
        members
            .iter()
            .map(|m| TfPort {
                dir: port.dir,
                net_or_var: Some(m.kind),
                signed: m.signed,
                range: m.range.clone(),
                name: Ident {
                    name: Self::unpacked_member_net(&port.name.name, &m.name.name),
                    span: port.name.span,
                },
                unpacked: Vec::new(),
                default: None,
                span: port.span,
            })
            .collect()
    }

    /// R5: at a USER function/task/method call, expand each bare-ident actual that
    /// names an unpacked-struct variable `r` into its member nets `$unp$r$field…`
    /// (positional, struct-declaration order — matching the callee's expanded member
    /// formals from `unpacked_struct_member_ports`). A whole-struct value has no flat
    /// representation, so the callee formal was expanded the same way and the arities
    /// line up; a mismatch (passing `r` to a non-struct formal) is a loud arity error
    /// at elaborate. Non-struct args pass through byte-identically, and the whole is a
    /// no-op when no struct var is in scope. NOT applied to `$system` calls (a bare
    /// `$display(r)` stays a whole-struct use = the existing loud path).
    pub(crate) fn expand_struct_call_args(&self, args: Vec<Expr>) -> Vec<Expr> {
        if self.var_unpacked_struct.is_empty() {
            return args;
        }
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            let expanded = match &a.kind {
                ExprKind::Ident(path) if path.segments.len() == 1 => self
                    .var_unpacked_struct
                    .get(&path.segments[0].name)
                    .and_then(|ty| self.unpacked_struct_layouts.get(ty))
                    .map(|members| (path.segments[0].name.clone(), a.span, members.clone())),
                _ => None,
            };
            match expanded {
                Some((var, span, members)) => {
                    for m in &members {
                        out.push(Expr {
                            kind: ExprKind::Ident(HierPath {
                                segments: vec![Ident {
                                    name: Self::unpacked_member_net(&var, &m.name.name),
                                    span,
                                }],
                                span,
                            }),
                            span,
                        });
                    }
                }
                None => out.push(a),
            }
        }
        out
    }

    /// Round-9: if `path` is `var.field` where `var` is an UNPACKED-struct
    /// variable and `field` is one of its members, return the single-segment
    /// member-net path `$unp$var$field` (the desugar target). `None` for a
    /// non-unpacked var, a non-member field, or a non-2-segment path — so packed
    /// structs and every other access fall through byte-identically.
    pub(crate) fn unpacked_field_ident(&self, path: &HierPath) -> Option<HierPath> {
        if path.segments.len() != 2 {
            return None;
        }
        let var = &path.segments[0].name;
        let tyname = self.var_unpacked_struct.get(var)?;
        let members = self.unpacked_struct_layouts.get(tyname)?;
        let field = &path.segments[1].name;
        if !members.iter().any(|m| &m.name.name == field) {
            return None;
        }
        Some(HierPath {
            segments: vec![Ident {
                name: Self::unpacked_member_net(var, field),
                span: path.span,
            }],
            span: path.span,
        })
    }

    /// Round-9: peek an UNPACKED-struct-typed declaration `[pkg::]T name…` at the
    /// cursor. Returns the (scoped-or-bare) type-name key when the leading
    /// token(s) name a KNOWN unpacked struct type AND a var-name ident follows.
    /// Non-consuming; mirrors `peek_block_typedef_decl` (which only sees `typedefs`
    /// and so never fires for an unpacked struct).
    pub(crate) fn peek_unpacked_struct_decl(&self) -> Option<String> {
        let key = self.scoped_type_key().or_else(|| {
            if self.is_ident() {
                Some(self.cur_text().to_string())
            } else {
                None
            }
        })?;
        if !self.unpacked_struct_layouts.contains_key(&key) {
            return None;
        }
        let q = self.scope_qualifier_len();
        if matches!(
            self.peek_at(q + 1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            Some(key)
        } else {
            None
        }
    }

    /// Round-9: `block_body`'s unpacked-struct decl branch — the PEEK and the
    /// parse, extracted whole into a COLD, non-inlined helper so NONE of their
    /// locals (the peek's `String` key + the member-`NetVarDecl` construction)
    /// enlarge `block_body`'s frame on the deep `parse_statement → parse_seq_block
    /// → block_body` recursion (the `MAX_STMT_DEPTH` budget is frame-sized).
    /// Returns whether a decl was consumed. Mirrors `parse_automatic_block_decl` —
    /// enlarging `block_body`'s frame overflowed the 2 MiB CI test-thread stack at
    /// the cap (`depth_guard.rs::deep_stmt_nesting_errors_cleanly`).
    #[inline(never)]
    pub(crate) fn try_block_unpacked_struct_decl(
        &mut self,
        decls: &mut Vec<NetVarDecl>,
        scope: &mut Option<Box<ScopeSnapshot>>,
    ) -> bool {
        let Some(tyname) = self.peek_unpacked_struct_decl() else {
            return false;
        };
        // Registers a var binding → triggers the block scope snapshot like any
        // struct-typed decl.
        if scope.is_none() {
            *scope = Some(Box::new(self.snapshot_scope()));
        }
        if let Some(member_decls) = self.parse_unpacked_struct_decl(tyname) {
            decls.extend(member_decls);
        }
        true
    }

    /// G6B: the module/interface/program-body twin of
    /// [`try_block_unpacked_struct_decl`] — a scalar `[pkg::]T k;` at module scope
    /// desugars to N member `NetVarDecl`s pushed as `ModuleItem::NetVar`. Cold and
    /// non-inlined so its locals don't enlarge the body-loop frame (depth_guard).
    /// Module-scope needs no block-scope snapshot: `var_unpacked_struct` is cleared
    /// per module (`parse_module_like`), so registrations don't leak across modules.
    #[inline(never)]
    pub(crate) fn try_module_unpacked_struct_decl(&mut self, body: &mut Vec<ModuleItem>) -> bool {
        let Some(tyname) = self.peek_unpacked_struct_decl() else {
            return false;
        };
        if let Some(member_decls) = self.parse_unpacked_struct_decl(tyname) {
            body.extend(member_decls.into_iter().map(ModuleItem::NetVar));
        }
        true
    }

    /// N3: the packed `StructLayout` (MSB-first) of a PACKABLE unpacked record — one
    /// whose members are ALL integral (`int`/`byte`/`logic [W:0]`/…). `None` if the
    /// type is not a known unpacked record or has a `string`/`real`/dynamic member
    /// (not packable → a record ARRAY of it stays loud, deferred to a heterogeneous
    /// heap). Mirrors the `parse_typedef_struct` packed layout loop.
    pub(crate) fn packable_record_layout(&self, tyname: &str) -> Option<StructLayout> {
        let members = self.unpacked_struct_layouts.get(tyname)?;
        let mut widths = Vec::with_capacity(members.len());
        for m in members {
            // A `string`/`real`/handle/event member is NOT a bit-vector — `member_width_
            // kind` would give it a bogus default 1-bit width (silently corrupting the
            // field), so gate on the integral KIND FIRST → non-packable → loud.
            if !Self::member_kind_is_integral(m.kind) {
                return None;
            }
            // A multi-dim packed member inside a RECORD ARRAY (`rec_t a[N]; a[i].m[j]`)
            // needs the element-stride sub-select on the array element's shared net — a
            // follow-on. Keep it correct-or-LOUD here (non-packable → scalar/loud path).
            if !m.packed_dims.is_empty() {
                return None;
            }
            match self.member_width_kind(m.kind, &m.range) {
                Some(w) if w > 0 => widths.push(w),
                _ => return None,
            }
        }
        // A MIXED 2-/4-state record (≥1 two-state member like `int`/`bit` AND ≥1
        // four-state member like `logic`) is NOT packable into one net: the single
        // packed net has ONE kind, so a `Bit` net would let a `logic` field lose X and
        // a `Logic` net would let a 2-state field hold/default X (IEEE §6.11.2 says a
        // 2-state member defaults 0 and coerces X/Z→0). The scalar record path (per-
        // member nets) and the whole-element `'{…}` desugar handle this per-field, but
        // the array's shared net and its element member-write/fresh-default paths
        // cannot — so a mixed record ARRAY is correct-or-LOUD (never a silent X), like a
        // non-packable (string/real) record. An all-2-state OR all-4-state record is
        // fine (uniform kind). (Adversarial soundness review RANK 1.)
        let any_two_state = members.iter().any(|m| Self::member_kind_two_state(m.kind));
        let any_four_state = members.iter().any(|m| !Self::member_kind_two_state(m.kind));
        if any_two_state && any_four_state {
            return None;
        }
        let total: u32 = widths.iter().sum();
        let mut off = total;
        let mut fields = Vec::with_capacity(members.len());
        for (m, w) in members.iter().zip(&widths) {
            off -= w;
            fields.push((
                m.name.name.clone(),
                off,
                *w,
                Self::member_ascending(&m.range),
                m.signed,
                Self::member_kind_two_state(m.kind),
                Self::member_dbase(&m.range),
                1u32, // elem_stride: multi-dim members were rejected above
            ));
        }
        Some(StructLayout { fields })
    }

    /// Round-9: parse a scalar UNPACKED-struct declaration `[pkg::]T k [, k2];`,
    /// desugaring each variable into its member nets (`k$field`, one `NetVarDecl`
    /// per member, each with the member's OWN type). Registers each var in
    /// `var_unpacked_struct` so `k.field` member access desugars. N3: a DYNAMIC array
    /// of a PACKABLE record (`T arr [];`) lowers instead to a single wide `logic` net
    /// (packed-struct width) registered in `record_array_vars` — `arr[i].field` is a
    /// part-select on the element. A FIXED unpacked array, a non-packable record
    /// array, and a scalar decl-init stay LOUD.
    /// §4.5.192: a packable UNPACKED-struct SCALAR body-local in a function/task
    /// (`rec_t p;`). The type lives only in `unpacked_struct_layouts` (never in
    /// `typedefs`), so `peek_block_typedef_decl` misses it and the body-decl loop
    /// would treat `rec_t` as a statement. Recognized here and lowered to a
    /// packed-vector local (`logic/bit [W-1:0] p;`) registered as a scalar struct var,
    /// so `p.field` desugars to a part-select (`struct_field_select` →
    /// `packable_record_layout`). Returns `None` when the current tokens are not a
    /// packable-record scalar decl (a non-packable record, an array, a decl-init, or
    /// a non-`<name> <ident>;` shape) — the caller then falls through (loud downstream).
    pub(crate) fn parse_body_unpacked_struct_local(&mut self) -> Option<NetVarDecl> {
        if !self.is_ident() {
            return None;
        }
        let tyname = self.cur_text().to_string();
        if !self.unpacked_struct_layouts.contains_key(&tyname) {
            return None;
        }
        // Need `<type> <ident>` and a packable layout; else leave it to the caller.
        if !matches!(self.peek_at(1), Some(TokenKind::Word(WordKind::Ident))) {
            return None;
        }
        let layout = self.packable_record_layout(&tyname)?;
        let span = self.cur_span();
        self.bump(); // the type name
        let names = self.parse_decl_name_list()?;
        self.expect(TokenKind::Semi, "';'");
        let w: u32 = layout.fields.iter().map(|f| f.2).sum();
        let all_two_state = layout.fields.iter().all(|f| f.5);
        let mut decl_names: Vec<DeclName> = Vec::new();
        for n in &names {
            // Only a SCALAR, non-init, non-`$` name (array / decl-init are follow-ons).
            if !n.unpacked.is_empty() || n.init.is_some() || n.name.name.contains('$') {
                self.error_at(
                    n.name.span,
                    "an unpacked-struct body-local supports only a scalar `rec_t p;` in v1 \
                     (an array element, a `'{…}` initializer, or a `$`-name is unsupported)",
                );
                continue;
            }
            self.var_struct.insert(n.name.name.clone(), tyname.clone());
            self.struct_scalar_vars.insert(n.name.name.clone());
            decl_names.push(DeclName {
                name: n.name.clone(),
                unpacked: Vec::new(),
                init: None,
                span: n.name.span,
            });
        }
        if decl_names.is_empty() {
            return None;
        }
        Some(NetVarDecl {
            kind: if all_two_state {
                NetVarKind::Bit
            } else {
                NetVarKind::Logic
            },
            signed: false,
            range: Some(self.synth_bit_range(w, span)),
            packed: Vec::new(),
            delay: None,
            names: decl_names,
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span,
        })
    }

    pub(crate) fn parse_unpacked_struct_decl(&mut self, tyname: String) -> Option<Vec<NetVarDecl>> {
        self.eat_scope_qualifier(); // optional `pkg::`
        self.bump(); // the type-name identifier
        let names = self.parse_decl_name_list()?;
        self.expect(TokenKind::Semi, "';'");
        let members = self.unpacked_struct_layouts.get(&tyname)?.clone();
        let mut out = Vec::new();
        for n in &names {
            if !n.unpacked.is_empty() {
                // N3: a DYNAMIC array of a PACKABLE record (`rec_t arr [];`) lowers to
                // ONE wide `logic` DynArray net (the packed-struct total width). A decl
                // init `'{ '{…}, … }` desugars each element to a field-width concat; the
                // existing dyn-array `'{…}` flush turns it into `new[N]` + writes. A
                // fixed / multi-dim / non-packable record array stays loud.
                if n.unpacked.len() == 1
                    && matches!(n.unpacked[0], Dim::Dyn)
                    && !n.name.name.contains('$')
                {
                    if let Some(layout) = self.packable_record_layout(&tyname) {
                        let w: u32 = layout.fields.iter().map(|f| f.2).sum();
                        // An all-2-state record (`bit`/`byte`/`int`/… members only) defaults
                        // its fields to 0, not X — pick `Bit` so a `new[n]`'d / undeclared
                        // element reads 0 (mirrors the packed-struct `NetVarKind::Bit` path);
                        // any 4-state member keeps `Logic` (X default). (Adversarial soundness
                        // review RANK 2.)
                        let all_two_state = layout.fields.iter().all(|f| f.5);
                        self.record_array_vars
                            .insert(n.name.name.clone(), tyname.clone());
                        let init = n
                            .init
                            .clone()
                            .map(|e| self.desugar_record_array_init(&tyname, e));
                        out.push(NetVarDecl {
                            kind: if all_two_state {
                                NetVarKind::Bit
                            } else {
                                NetVarKind::Logic
                            },
                            signed: false,
                            range: Some(self.synth_bit_range(w, n.name.span)),
                            packed: Vec::new(),
                            delay: None,
                            names: vec![DeclName {
                                name: n.name.clone(),
                                unpacked: vec![Dim::Dyn],
                                init,
                                span: n.name.span,
                            }],
                            lifetime: None,
                            class_type: None,
                            class_args: Vec::new(),
                            const_param: false,
                            span: n.name.span,
                        });
                        continue;
                    }
                    // N3 SoA (heterogeneous heap): the record is NOT uniform-packable
                    // (a MIXED 2-/4-state record — `packable_record_layout` returned
                    // None — or a string/real-member record) → lower each member to its
                    // OWN typed dyn array `$unp$arr$field` (integral → int/logic dyn, so
                    // a 2-state field defaults 0 / a 4-state field keeps X natively;
                    // `string` → string dyn; `real` → real dyn — Phase 3 heterogeneous
                    // heap). A nested-struct / event / class member has no dyn-array
                    // form → falls through to loud. `arr[i].field` / `new[]` / `'{…}` are
                    // desugared to native dyn ops at the use sites.
                    if members.iter().all(|m| Self::member_kind_soa_ok(m.kind)) {
                        let field_inits: Vec<Option<Expr>> = match &n.init {
                            Some(e) => match self.soa_field_inits(members.len(), e) {
                                Some(v) => v.into_iter().map(Some).collect(),
                                None => {
                                    self.error_at(
                                        n.name.span,
                                        "a record-array initializer must be `'{ '{…}, … }` \
                                         with one inner pattern per element, each listing \
                                         every field",
                                    );
                                    continue;
                                }
                            },
                            None => vec![None; members.len()],
                        };
                        self.record_soa_vars
                            .insert(n.name.name.clone(), tyname.clone());
                        for (m, finit) in members.iter().zip(field_inits) {
                            out.push(NetVarDecl {
                                kind: m.kind,
                                signed: m.signed,
                                range: m.range.clone(),
                                packed: Vec::new(),
                                delay: None,
                                names: vec![DeclName {
                                    name: Ident {
                                        name: Self::unpacked_member_net(&n.name.name, &m.name.name),
                                        span: n.name.span,
                                    },
                                    unpacked: vec![Dim::Dyn],
                                    init: finit,
                                    span: n.name.span,
                                }],
                                lifetime: None,
                                class_type: None,
                                class_args: Vec::new(),
                                const_param: false,
                                span: n.name.span,
                            });
                        }
                        continue;
                    }
                }
                // §4.5.191: a FIXED 1-D unpacked array of a PACKABLE record
                // (`rec_t mem[N]`) lowers to a fixed array of the packed-struct-width
                // vector `logic/bit [W-1:0] mem[N]`, registered as a struct 1-D array so
                // `mem[i].field` (§4.5.190) desugars to a part-select on the element
                // (`struct_array_field_geom` falls back to `packable_record_layout`). A
                // decl-init `'{…}`, a multi-dim array, or a non-packable record (mixed
                // 2-/4-state, string/real/nested/class member) stays loud.
                // Family A (r17): the UNBOUNDED QUEUE form (`rec_t q[$]` =
                // `Dim::Queue(None)`) routes through the SAME body — it lowers to a
                // packed-vector queue `logic/bit [W-1:0] q[$]`, byte-identical to the
                // registration a packed-struct queue gets via `parse_typed_decl`, so
                // `q.push_back`/`q.size()`/`q[i]`/`q[i].field` all work with no further
                // change. A bounded queue (`[$:N]` = `Dim::Queue(Some(_))`) stays loud.
                if n.unpacked.len() == 1
                    && matches!(
                        n.unpacked[0],
                        Dim::Range(_) | Dim::Size(_) | Dim::Queue(None)
                    )
                    && n.init.is_none()
                    && !n.name.name.contains('$')
                {
                    if let Some(layout) = self.packable_record_layout(&tyname) {
                        let w: u32 = layout.fields.iter().map(|f| f.2).sum();
                        let all_two_state = layout.fields.iter().all(|f| f.5);
                        self.var_struct.insert(n.name.name.clone(), tyname.clone());
                        self.struct_1d_array_vars.insert(n.name.name.clone());
                        out.push(NetVarDecl {
                            kind: if all_two_state {
                                NetVarKind::Bit
                            } else {
                                NetVarKind::Logic
                            },
                            signed: false,
                            range: Some(self.synth_bit_range(w, n.name.span)),
                            packed: Vec::new(),
                            delay: None,
                            names: vec![DeclName {
                                name: n.name.clone(),
                                unpacked: n.unpacked.clone(),
                                init: None,
                                span: n.name.span,
                            }],
                            lifetime: None,
                            class_type: None,
                            class_args: Vec::new(),
                            const_param: false,
                            span: n.name.span,
                        });
                        continue;
                    }
                }
                self.error_at(
                    n.name.span,
                    "an array of unpacked structs (record array) is unsupported in v1 — scalar record only",
                );
                continue;
            }
            if n.init.is_some() {
                self.error_at(
                    n.name.span,
                    "an unpacked-struct declaration initializer / '{…} pattern is unsupported in v1",
                );
                continue;
            }
            // A `$` in the variable name would break the `$unp$<var>$<field>`
            // desugar's injectivity (the first `$` after the prefix must delimit
            // var from field) → reject. `$` in a var name is unusual; loud is safe.
            if n.name.name.contains('$') {
                self.error_at(
                    n.name.span,
                    "an unpacked-struct variable name containing `$` is unsupported in v1",
                );
                continue;
            }
            // Family A (r17): a PACKABLE scalar unpacked-struct VARIABLE (`pkt_t p;`,
            // all-integral members) lowers to a WHOLE packed-vector net registered in
            // `var_struct` + `struct_scalar_vars` (byte-identical to the §4.5.192
            // tf-body local `parse_body_unpacked_struct_local`), NOT the per-member
            // `$unp$p$field` nets below. This flips MODULE + BLOCK scope together (both
            // route scalars through here), unifying every scope on whole-vector for
            // packable records so a struct var passes WHOLE to a (now whole-vector)
            // struct tf-port and works as a whole value (`q.push_back(p)`, `r = p`).
            // `p.field` still resolves — `unpacked_field_ident` misses (var left
            // `var_unpacked_struct`) then `struct_field_select` → part-select. A
            // NON-packable record (string/real/mixed) keeps the per-member path.
            if let Some(layout) = self.packable_record_layout(&tyname) {
                let w: u32 = layout.fields.iter().map(|f| f.2).sum();
                let all_two_state = layout.fields.iter().all(|f| f.5);
                self.var_struct.insert(n.name.name.clone(), tyname.clone());
                self.struct_scalar_vars.insert(n.name.name.clone());
                out.push(NetVarDecl {
                    kind: if all_two_state {
                        NetVarKind::Bit
                    } else {
                        NetVarKind::Logic
                    },
                    signed: false,
                    range: Some(self.synth_bit_range(w, n.name.span)),
                    packed: Vec::new(),
                    delay: None,
                    names: vec![DeclName {
                        name: n.name.clone(),
                        unpacked: Vec::new(),
                        init: None,
                        span: n.name.span,
                    }],
                    lifetime: None,
                    class_type: None,
                    class_args: Vec::new(),
                    const_param: false,
                    span: n.name.span,
                });
                continue;
            }
            self.var_unpacked_struct
                .insert(n.name.name.clone(), tyname.clone());
            for m in &members {
                out.push(NetVarDecl {
                    kind: m.kind,
                    signed: m.signed,
                    range: m.range.clone(),
                    packed: Vec::new(),
                    delay: None,
                    names: vec![DeclName {
                        name: Ident {
                            name: Self::unpacked_member_net(&n.name.name, &m.name.name),
                            span: n.name.span,
                        },
                        unpacked: Vec::new(),
                        init: None,
                        span: n.name.span,
                    }],
                    lifetime: None,
                    class_type: None,
                    class_args: Vec::new(),
                    const_param: false,
                    span: n.name.span,
                });
            }
        }
        Some(out)
    }

    /// Is a packed-struct member declared with an ascending range (`logic [0:N]`,
    /// so field index 0 is the MSB)? Scalar members (no range) are not ascending.
    pub(crate) fn member_ascending(range: &Option<Range>) -> bool {
        match range {
            Some(r) => match (Self::const_lit(&r.msb), Self::const_lit(&r.lsb)) {
                (Some(m), Some(l)) => m < l,
                _ => false,
            },
            None => false,
        }
    }

    /// The member's DECLARED base index — the numerically smaller of its declared
    /// `msb`/`lsb` (so `logic [15:8]` → 8, ascending `logic [4:11]` → 4, a plain
    /// `[N:0]`/`[0:N]`/atom member → 0). Subtracted from a sub-select's SOURCE index
    /// to map it into the field-relative `[0, w)` range: the field part-select `pv`
    /// normalizes the member to `[w-1:0]`, so a non-zero declared base must be
    /// removed or the sub-select overruns `pv` (silent X) / lands on the wrong bits.
    ///
    /// Signed: a NEGATIVE base (`logic [7:-4]`, min = −4) is returned verbatim so a
    /// sub-select of such a member is loud-rejected downstream — a field-relative
    /// remap with a negative base needs signed offsets across every form (read,
    /// write, `+:`/`-:`, runtime), which v1 does not do; the whole-field read/write
    /// is unaffected and stays correct. A non-negative base casts to the `u32` the
    /// select machinery uses, so every currently-valid member is byte-identical.
    pub(crate) fn member_dbase(range: &Option<Range>) -> i64 {
        match range {
            Some(r) => match (Self::const_lit(&r.msb), Self::const_lit(&r.lsb)) {
                (Some(m), Some(l)) => m.min(l),
                _ => 0,
            },
            None => 0,
        }
    }

    /// Bind a struct/union tf-port name to its layout so `name.field` desugars to a
    /// part-select in the body (mirrors `parse_typed_decl`'s struct binding). A union
    /// is excluded from the `'{…}` assignment-pattern (`struct_scalar_vars`) exactly
    /// as a regular union var is. Scoped to the enclosing function/task by the
    /// snapshot/restore in `parse_function_def`/`parse_task_def`.
    pub(crate) fn bind_tf_port_struct(&mut self, name: &str, struct_name: &str) {
        self.var_struct
            .insert(name.to_string(), struct_name.to_string());
        if !self.union_type_names.contains(struct_name) {
            self.struct_scalar_vars.insert(name.to_string());
        }
    }
}
