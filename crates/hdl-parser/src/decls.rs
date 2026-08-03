//! net & variable declarations — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

/// The default signedness of a declared kind when no `signed`/`unsigned` is given:
/// the 2-state integer atom types (and `integer`) default SIGNED (IEEE §6.11.1);
/// nets / `reg` / `logic` / `bit` default unsigned.
pub(crate) fn atom_default_signed(kind: Option<NetVarKind>) -> bool {
    matches!(
        kind,
        Some(
            NetVarKind::Byte
                | NetVarKind::Shortint
                | NetVarKind::Int
                | NetVarKind::Longint
                | NetVarKind::Integer
        )
    )
}

impl Parser<'_, '_> {
    /// Parse an optional signing qualifier. Returns `Some(true)` for `signed`,
    /// `Some(false)` for `unsigned`, `None` for neither — so the caller can apply
    /// the TYPE's default (2-state atom types default signed, reg/wire/logic/bit
    /// default unsigned). Conflating `unsigned` with "absent" lost the distinction
    /// for atom types (`int unsigned` was treated as signed).
    pub(crate) fn opt_signed(&mut self) -> Option<bool> {
        if self.eat_kw(Kw::Unsigned) {
            return Some(false);
        }
        if self.eat_kw(Kw::Signed) {
            return Some(true);
        }
        None
    }

    pub(crate) fn net_var_kind(&self) -> Option<NetVarKind> {
        use Kw::*;
        match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(k))) => Some(match k {
                Wire => NetVarKind::Wire,
                Tri => NetVarKind::Tri,
                Wand => NetVarKind::Wand,
                Triand => NetVarKind::Triand,
                Wor => NetVarKind::Wor,
                Trior => NetVarKind::Trior,
                Tri0 => NetVarKind::Tri0,
                Tri1 => NetVarKind::Tri1,
                Supply0 => NetVarKind::Supply0,
                Supply1 => NetVarKind::Supply1,
                Trireg => NetVarKind::Trireg,
                Uwire => NetVarKind::Uwire,
                Reg => NetVarKind::Reg,
                Logic => NetVarKind::Logic,
                Integer => NetVarKind::Integer,
                Real => NetVarKind::Real,
                Realtime => NetVarKind::Realtime,
                Time => NetVarKind::Time,
                Event => NetVarKind::Event,
                String => NetVarKind::String,
                Bit => NetVarKind::Bit,
                Byte => NetVarKind::Byte,
                Shortint => NetVarKind::Shortint,
                Int => NetVarKind::Int,
                Longint => NetVarKind::Longint,
                _ => return None,
            }),
            _ => None,
        }
    }

    /// Decide ANSI vs non-ANSI by the FIRST token inside `(`: a direction keyword
    /// ⇒ ANSI. A bare identifier ⇒ non-ANSI name list. (Documented PR1 limitation,
    /// verdict H2/M1: a malformed ANSI header beginning with a bare net/var kind —
    /// e.g. illegal `module m(reg x)` — is routed to non-ANSI and errors in the body.
    /// Strict V2005 non-ANSI headers are bare-name-only, so this is correct for scope.)
    pub(crate) fn parse_port_list(&mut self) -> PortList {
        if self.peek() != Some(TokenKind::LParen) {
            return PortList::None;
        }
        self.bump(); // '('
        if self.peek() == Some(TokenKind::RParen) {
            self.bump();
            return PortList::Ansi(Vec::new());
        }
        let ansi = matches!(
            self.peek(),
            Some(TokenKind::Word(WordKind::Keyword(
                Kw::Input | Kw::Output | Kw::Inout
            )))
        ) ||
            // v5 ⑥: `module m(intf bus, …)` — an interface-typed first port:
            // Ident followed by Ident (`intf bus`) or Dot (`intf.mp bus`).
            (matches!(self.peek(), Some(TokenKind::Word(WordKind::Ident)))
                && matches!(
                    self.peek_at(1),
                    Some(TokenKind::Word(WordKind::Ident) | TokenKind::Dot)
                ));
        if ansi {
            let mut ports: Vec<AnsiPort> = Vec::new();
            loop {
                let prev = ports.last().cloned(); // for dir + type/range inheritance
                let port = self.parse_ansi_port(prev.as_ref());
                ports.push(port);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
            PortList::Ansi(ports)
        } else {
            let mut names = Vec::new();
            loop {
                if let Some(id) = self.ident() {
                    names.push(id);
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "')'");
            PortList::NonAnsi(names)
        }
    }

    /// `prev = None` ⇒ first ANSI port; a missing direction is then an ERROR
    /// (verdict M4: don't silently default the first port to Input). A comma-continued
    /// port with no direction inherits the previous port's direction; a PURE
    /// continuation (`input [7:0] a, b`) that also omits its own type/range/signed
    /// inherits those too, so both `a` and `b` are `[7:0]` (IEEE 1800 §23.2.2.1).
    pub(crate) fn parse_ansi_port(&mut self, prev: Option<&AnsiPort>) -> AnsiPort {
        let start = self.cur_span();
        // v5 ⑥: interface-typed port `intf p` / `intf.mp p` — an Ident in the
        // type position followed by Ident/Dot. No direction, no range.
        //
        // Guard: if the current Ident is a registered typedef name and the next
        // token is an Ident (port name), this is `typedef_t portname` — a typedef
        // port in a comma-continuation context (e.g. `input byte_t a, word_t c`).
        // Skip the interface heuristic so try_port_typedef() handles it below.
        // (typedef_t followed by `.` cannot be a valid typedef port, so the `.`
        // case is intentionally left to the interface heuristic.)
        let is_typedef_portname_shape = matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) && self.peek_typedef_name().is_some();
        if !is_typedef_portname_shape
            && matches!(self.peek(), Some(TokenKind::Word(WordKind::Ident)))
            && matches!(
                self.peek_at(1),
                Some(TokenKind::Word(WordKind::Ident) | TokenKind::Dot)
            )
        {
            let iface = self.ident().unwrap();
            let modport = if self.eat(TokenKind::Dot) {
                self.ident()
            } else {
                None
            };
            let name = self.ident().unwrap_or(Ident {
                name: String::new(),
                span: self.cur_span(),
            });
            let ispan = iface.span;
            return AnsiPort {
                dir: PortDir::Input, // placeholder — iface ports carry no dir
                net_or_var: None,
                signed: false,
                range: None,
                packed: Vec::new(),
                name,
                default: None,
                iface: Some(IfaceRef {
                    iface,
                    modport,
                    span: ispan,
                }),
                span: start.to(self.prev_span()),
            };
        }
        let explicit_dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                self.bump();
                Some(PortDir::Input)
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                Some(PortDir::Output)
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Inout))) => {
                self.bump();
                Some(PortDir::Inout)
            }
            _ => None,
        };
        let dir = match (explicit_dir, prev.map(|p| p.dir)) {
            (Some(d), _) => d,
            (None, Some(prev_dir)) => prev_dir, // inherit
            (None, None) => {
                self.error("port direction (first ANSI port must specify one)");
                PortDir::Input
            } // recover as Input
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        // `input mode_e m` — a typedef name as the port type (typedef-recognition
        // family). When no built-in kind keyword matched, a simple vector/enum
        // typedef resolves to its (kind, signed, range); the typedef name carries
        // the range, so the normal signed/range/packed reads are SKIPPED for it
        // (they would otherwise consume the port NAME). Built-in path unchanged.
        let typedef_ty = if net_or_var.is_none() {
            self.try_port_typedef()
        } else {
            None
        };
        // EXT2-E1: a packed-struct typedef port carries a layout name to bind
        // once the port NAME is known (below), so `c.field` desugars.
        let mut port_struct_name: Option<String> = None;
        let (mut signed, mut range, mut packed) = if let Some((k, s, r, sn)) = typedef_ty {
            net_or_var = Some(k);
            port_struct_name = sn;
            (s, r, Vec::new())
        } else {
            (
                self.signed_eff(net_or_var),
                self.opt_range(),
                self.opt_packed_dims(), // additional packed dims `[3:0][7:0]`
            )
        };
        // §4.5.156 (§3 全 site): reject an inline packed range/dim on a non-vector port
        // kind (`input byte [7:0] p`). A typedef-typed port has `net_or_var` set to the
        // typedef's resolved kind with its OWN range (an atom typedef has range=None →
        // no-op; a signed/packed atom typedef was already rejected at its decl), so no
        // valid typedef port over-rejects. A bare continuation has `net_or_var=None`.
        if let Some(k) = net_or_var {
            self.reject_packed_dims_on_nonvector(k, range.is_some() || !packed.is_empty());
        }
        // A pure continuation (no own direction/type/range/signed) inherits the
        // previous port's type — `input [7:0] a, b` ⇒ b is also `[7:0]`.
        if explicit_dir.is_none()
            && net_or_var.is_none()
            && range.is_none()
            && packed.is_empty()
            && !signed
        {
            if let Some(p) = prev {
                net_or_var = p.net_or_var;
                signed = p.signed;
                range = p.range.clone();
                packed = p.packed.clone();
            }
        }
        let name = self.ident().unwrap_or(Ident {
            name: String::new(),
            span: self.cur_span(),
        });
        // EXT2-E1: bind a packed-struct port's NAME to its layout (mirrors the
        // tf-port path) so a body `c.field` desugars to a part-select on the
        // port's flat vector. Module-scoped like a `cfg_t c;` var binding.
        if let Some(sn) = &port_struct_name {
            self.bind_tf_port_struct(&name.name, sn);
        }
        let default = if self.eat(TokenKind::Eq) {
            Some(self.expr(0))
        } else {
            None
        };
        AnsiPort {
            dir,
            net_or_var,
            signed,
            range,
            packed,
            name,
            default,
            iface: None,
            span: start.to(self.prev_span()),
        }
    }

    pub(crate) fn parse_port_decl(&mut self) -> Option<PortDecl> {
        let start = self.cur_span();
        let dir = match self.peek() {
            Some(TokenKind::Word(WordKind::Keyword(Kw::Input))) => {
                self.bump();
                PortDir::Input
            }
            Some(TokenKind::Word(WordKind::Keyword(Kw::Output))) => {
                self.bump();
                PortDir::Output
            }
            _ => {
                self.bump();
                PortDir::Inout
            }
        };
        let mut net_or_var = self.net_var_kind();
        if net_or_var.is_some() {
            self.bump();
        }
        // `input byte_t a;` — a typedef name as a non-ANSI port type (mirrors the
        // ANSI path; typedef-recognition family §4.5.36 ALL-variants).
        let typedef_ty = if net_or_var.is_none() {
            self.try_port_typedef()
        } else {
            None
        };
        let mut port_struct_name: Option<String> = None;
        let (signed, range) = if let Some((k, s, r, sn)) = typedef_ty {
            net_or_var = Some(k);
            port_struct_name = sn;
            (s, r)
        } else {
            (self.signed_eff(net_or_var), self.opt_range())
        };
        // §4.5.156 (§3 全 site): reject a packed range on a non-vector non-ANSI port.
        if let Some(k) = net_or_var {
            self.reject_packed_dims_on_nonvector(k, range.is_some());
        }
        let mut names = Vec::new();
        loop {
            if let Some(id) = self.ident() {
                // EXT2-E1: each name of a packed-struct non-ANSI port (`input
                // cfg_t a, b;`) binds to the struct layout so `a.field`/`b.field`
                // desugar (mirrors the ANSI path and `bind_tf_port_struct`).
                if let Some(sn) = &port_struct_name {
                    self.bind_tf_port_struct(&id.name, sn);
                }
                names.push(id);
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';'");
        Some(PortDecl {
            dir,
            net_or_var,
            signed,
            range,
            names,
            span: start.to(self.prev_span()),
        })
    }

    /// `allow_net_delay`: only a MODULE-ITEM / GENERATE-scope net declaration may
    /// carry an IEEE §6.1.3 net-declaration delay (it desugars to a continuous
    /// assign, which only those scopes elaborate). In a procedural block / function
    /// / task / class body a `wire #3 w = a;` is illegal — the caller passes `false`
    /// so the `#` is left to error as before (correct-or-loud: never parse a delay
    /// we would then silently drop).
    /// §4.5.156: a packed range/dimension is legal ONLY on a vector-typed decl — a
    /// NET or `logic`/`reg`/`bit` (IEEE §6.11 `integer_vector_type`). A fixed-width
    /// integer atom (`byte`/`shortint`/`int`/`longint`/`integer`/`time`) and the
    /// dimensionless types (`real`/`string`/`event`) take none. Called at every decl
    /// / type-spec site that parses `kind + opt_range()/opt_packed_dims()` (§3 全
    /// site): module/block/func-body/class-member var, typedef alias, struct/union
    /// member, module port (ANSI + non-ANSI), tf-port (ANSI + non-ANSI), typed
    /// parameter, for-typed-init, enum base, class value-param, and the `int`
    /// function-return type. (`byte`/`shortint`/`longint` in a function-return type
    /// still reject, but via the leftover-`[` parse error, not this helper.) Emits a
    /// loud reject; the caller keeps its parsed decl (elaborate is skipped, errors > 0).
    pub(crate) fn reject_packed_dims_on_nonvector(&mut self, kind: NetVarKind, has_dims: bool) {
        if has_dims
            && !(kind.is_net()
                || matches!(kind, NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit))
        {
            self.error(
                "a vector-typed decl (net or `logic`/`reg`/`bit`) for a packed range/dimension — a fixed-width integer atom (`byte`/`int`/…) / `real` / `string` / `event` takes none (IEEE §6.11)",
            );
        }
    }

    pub(crate) fn parse_net_var(&mut self, allow_net_delay: bool) -> Option<NetVarDecl> {
        let start = self.cur_span();
        let kind = self.net_var_kind().unwrap();
        self.bump();
        let signed = self.signed_eff(Some(kind));
        let range = self.opt_range();
        let packed = self.opt_packed_dims(); // additional packed dims `logic [3:0][7:0]`
                                             // §4.5.156: reject a packed range/dim on a non-vector kind (see the helper).
        self.reject_packed_dims_on_nonvector(kind, range.is_some() || !packed.is_empty());
        // IEEE §6.1.3 net-declaration delay (`wire #3 w = a;` / `wire #(2,3) w = a;`):
        // after the range, before the name list. Only a NET kind in a delay-permitting
        // scope takes one — a `#` after a variable range, or any `#` in a procedural /
        // class scope, stays a loud error (never silently accept `reg #3 r`).
        let delay = if allow_net_delay && kind.is_net() && self.peek() == Some(TokenKind::Hash) {
            self.parse_delay()
        } else {
            None
        };
        let names = self.parse_decl_name_list()?;
        self.expect(TokenKind::Semi, "';'");
        Some(NetVarDecl {
            kind,
            signed,
            range,
            packed,
            delay,
            names,
            lifetime: None,
            class_type: None,
            class_args: Vec::new(),
            const_param: false,
            span: start.to(self.prev_span()),
        })
    }

    /// Comma-separated declarator list: `a, b[3:0], c = init`. Shared by
    /// `parse_net_var` and the typedef-name decl path. Does NOT consume the `;`.
    pub(crate) fn parse_decl_name_list(&mut self) -> Option<Vec<DeclName>> {
        let mut names = Vec::new();
        loop {
            let n_start = self.cur_span();
            let name = self.ident()?;
            let mut unpacked = Vec::new();
            while self.at_dim_start() {
                match self.parse_dim() {
                    Some(d) => unpacked.push(d),
                    None => break,
                }
            }
            let init = if self.eat(TokenKind::Eq) {
                Some(self.expr(0))
            } else {
                None
            };
            names.push(DeclName {
                name,
                unpacked,
                init,
                span: n_start.to(self.prev_span()),
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        Some(names)
    }

    /// `T name1, name2 = init, …;` where the leading type-name resolved to `info`.
    pub(crate) fn parse_typed_decl(&mut self, info: TypeInfo) -> Option<NetVarDecl> {
        let start = self.cur_span();
        let tyname = self.type_name_key(); // "pkg::t" (scoped twin key) or bare "t"
        self.eat_scope_qualifier(); // skip an optional `pkg::` before the type name
        self.bump(); // the type-name identifier
                     // ⓑ-breadth (§8.25): a parameterized class handle override `C #(16) h;`.
                     // Only a class-typed name takes specialization args; for any other type a
                     // `#` here is not ours (left for the caller / a loud error downstream).
        let class_args = if info.class_name.is_some() && self.peek() == Some(TokenKind::Hash) {
            self.parse_param_override_args()
        } else {
            Vec::new()
        };
        let mut names = self.parse_decl_name_list()?;
        self.expect(TokenKind::Semi, "';'");
        // If this is a struct type, bind each declared name → type so `var.field`
        // member accesses can be desugared to part-selects. A scalar (no unpacked
        // dims) struct var is additionally eligible for the `'{…}` pattern
        // desugar, applied here to its decl-init `= '{…}` (§10.9.1).
        if self.struct_layouts.contains_key(&tyname) {
            let is_union = self.union_type_names.contains(&tyname);
            for n in &mut names {
                self.var_struct.insert(n.name.name.clone(), tyname.clone());
                // A scalar STRUCT (not union, no unpacked dims) is eligible for the
                // `'{…}` pattern desugar; a union overlay is not (kept loud).
                if n.unpacked.is_empty() && !is_union {
                    self.struct_scalar_vars.insert(n.name.name.clone());
                    if let Some(init) = n.init.take() {
                        let nm = n.name.name.clone();
                        n.init = Some(self.desugar_struct_assign_pattern(&nm, init));
                    }
                } else if n.unpacked.len() == 1 && !is_union {
                    // A 1-D array of struct (`st_t arr[N]`): `arr[i] = '{…}` is a
                    // packed-struct pattern on the element. Multi-dim arrays stay
                    // loud (the indexed lvalue would be a nested BitSelect).
                    self.struct_1d_array_vars.insert(n.name.name.clone());
                }
            }
        }
        // If this is a (literal-foldable) enum type, bind each name → enum type so
        // `var.first/last/next/prev/name/num` can desugar to its labels (§6.19.5).
        if self.enum_defs.contains_key(&tyname) {
            for n in &names {
                self.var_enum.insert(n.name.name.clone(), tyname.clone());
            }
        }
        // N7: a class-typed alias carries the class name through to elaborate.
        let class_type = info.class_name.as_ref().map(|c| Ident {
            name: c.clone(),
            span: start,
        });
        Some(NetVarDecl {
            kind: info.kind,
            signed: info.signed,
            range: info.range,
            packed: info.packed,
            delay: None,
            names,
            lifetime: None,
            class_type,
            class_args,
            const_param: false,
            span: start.to(self.prev_span()),
        })
    }

    pub(crate) fn parse_cont_assign(&mut self) -> Option<ContinuousAssign> {
        let start = self.cur_span();
        self.bump(); // assign
        let delay = if self.peek() == Some(TokenKind::Hash) {
            self.parse_delay()
        } else {
            None
        };
        let mut assigns = Vec::new();
        loop {
            let lv = self.parse_lvalue();
            self.expect(TokenKind::Eq, "'=' in assign");
            let rhs = self.expr(0);
            // §10.9.1 packed-struct `'{…}` pattern on a continuous assign to a
            // whole struct net (same desugar as a procedural assign).
            let rhs = self.maybe_struct_pattern_rhs(&lv, rhs);
            assigns.push((lv, rhs));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semi, "';'");
        Some(ContinuousAssign {
            delay,
            assigns,
            span: start.to(self.prev_span()),
            from_gate: false,
        })
    }
}
