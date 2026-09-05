//! struct declarations — split out of the original `hdl-parser` lib.rs (mechanical move).

mod decl;

use super::*;

impl Parser<'_, '_> {
    /// Parse a packed struct/union MEMBER's type into `(kind, signed, range,
    /// packed_dims, nested)`. A built-in keyword resolves directly (§7.2.1); a
    /// SIMPLE user-defined type name (a vector / enum / atom typedef) resolves to
    /// its `TypeInfo`. §3 ⑤ ⓓ: a NESTED packed struct/union typedef (`perms_t
    /// perms;`) resolves to the flat vector its `TypeInfo` already is (`[total-1:0]`,
    /// the struct's 2-/4-state kind and whole-value signedness) and returns its
    /// type key as `nested`, so the layout can chain member accesses and recurse
    /// a `'{…}` value into it. A class handle or a multi-dim packed typedef member
    /// is honest-loud. Returns `None` (with the error already emitted) on a
    /// non-type token or an unsupported member type; the caller breaks out of
    /// the member loop.
    pub(crate) fn parse_struct_member_type(&mut self) -> Option<MemberType> {
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
            return Some((kind, signed, range, packed_dims, None));
        }
        if let Some(info) = self.peek_typedef_name() {
            let nm = self.type_name_key();
            if info.class_name.is_some() || !info.packed.is_empty() {
                self.error(
                    "a simple type for a struct/union member (a class / multi-dim packed member is unsupported in v1)",
                );
                return None;
            }
            let nested = self
                .struct_layouts
                .contains_key(&nm)
                .then(|| self.stable_type_key(&nm));
            self.eat_scope_qualifier();
            self.bump(); // the typedef-name token
            return Some((info.kind, info.signed, info.range, Vec::new(), nested));
        }
        self.error("a net/var type in a struct/union member");
        None
    }

    /// §3 ⑤ ⓓ: the key a NESTED member's layout stays reachable under. A bare
    /// name is unit-scoped only until its declaring unit ends (`restore_scope_unit`
    /// drops it; an importer sees the `pkg::t` twin), so a layout that names a
    /// nested member by a bare key would fail to chain — or, worse, chain into a
    /// same-named LOCAL type of the importer. A bare name that is the wildcard /
    /// explicit-import copy of a package twin (same layout under some `pkg::nm`)
    /// is keyed by that twin; a fresh local definition keeps the bare key (a
    /// package's own is re-spelled `pkg::nm` at `endpackage`, a module's stays
    /// module-local like the variable that uses it). Candidates are sorted so
    /// two packages exporting one identical layout pick deterministically.
    pub(crate) fn stable_type_key(&self, nm: &str) -> String {
        if nm.contains("::") {
            return nm.to_string();
        }
        let Some(bare) = self.struct_layouts.get(nm) else {
            return nm.to_string();
        };
        let suffix = format!("::{nm}");
        let mut cands: Vec<&String> = self
            .struct_layouts
            .iter()
            .filter(|(k, v)| k.ends_with(&suffix) && *v == bare)
            .map(|(k, _)| k)
            .collect();
        cands.sort();
        cands
            .first()
            .map(|k| (*k).clone())
            .unwrap_or_else(|| nm.to_string())
    }

    /// `typedef struct packed { <type> f1, f2; … } name;` (Phase-2). Members are
    /// laid out MSB-first into one flat `logic [W-1:0]` vector; the layout is
    /// recorded so `name var;` resolves and `var.field` desugars to a part-select.
    /// `start` is the span of the leading `typedef` keyword (already consumed).
    /// Parse `{ <type> f1, f2; … }` — the shared member list of a packed OR
    /// unpacked `typedef struct`. Cursor at `{`; consumes through the closing `}`.
    /// Also returns, parallel to the members, each member's NESTED struct/union
    /// type key (§3 ⑤ ⓓ; `None` for a non-struct member) — `StructMember` is a
    /// frozen AST type and cannot carry it.
    pub(crate) fn parse_struct_member_list(
        &mut self,
    ) -> Option<(Vec<StructMember>, Vec<Option<String>>)> {
        self.expect(TokenKind::LBrace, "'{' for struct body");
        let mut members = Vec::new();
        let mut nested_keys = Vec::new();
        while self.peek() != Some(TokenKind::RBrace) && !self.at_eof() {
            let before = self.pos;
            let m_start = self.cur_span();
            let Some((kind, signed, range, packed_dims, nested)) = self.parse_struct_member_type()
            else {
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
                nested_keys.push(nested.clone());
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
        Some((members, nested_keys))
    }

    /// Width of a struct member from its range. `None` ⇒ scalar (1). Constant
    /// bounds fold (`[7:0]`, `[8-1:0]`, and — §3 ⑤ ⓓ — `[W-1:0]` / `[p::W:0]` /
    /// `[$clog2(N)-1:0]` over a NON-overridable constant, see `const_locals`);
    /// anything else returns `None` (→ loud).
    pub(crate) fn member_width(&self, range: &Option<Range>) -> Option<u32> {
        match range {
            None => Some(1),
            Some(r) => {
                let msb = self.const_bound(&r.msb)?;
                let lsb = self.const_bound(&r.lsb)?;
                Some(u32::try_from(msb.abs_diff(lsb)).ok()? + 1)
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
                // R6: each expanded member inherits the whole port's spelling, so a
                // `ref cfg_t c` member still reports itself as `ref`.
                dir_spelling: port.dir_spelling,
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
    ///
    /// Round-19 (Task 5 / "F-struct"): an actual that ISN'T a bare struct-var Ident
    /// falls through to [`Self::expand_soa_array_elem_arg`] — `arr[i]` naming a
    /// record SoA array (`kats[0]`) is a whole NON-packable-record ELEMENT value and
    /// expands the same way, member-by-member.
    pub(crate) fn expand_struct_call_args(&self, args: Vec<Expr>) -> Vec<Expr> {
        if self.var_unpacked_struct.is_empty() && self.record_soa_vars.is_empty() {
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
                None => match self.expand_soa_array_elem_arg(&a) {
                    Some(elems) => out.extend(elems),
                    None => out.push(a),
                },
            }
        }
        out
    }

    /// Round-19 (Task 5 / "F-struct"): if `a` is `arr[i]` (`ExprKind::BitSelect`) or
    /// `arr[i +: w]`/`arr[i -: w]` (`ExprKind::IndexedPart`) whose base is a bare
    /// single-segment Ident naming a record SoA array (`record_soa_vars` — a FIXED,
    /// QUEUE, or DYNAMIC array of a NON-packable record: a `string`/`real`/mixed-
    /// state/param-width member), expand it to its N per-member element actuals
    /// `$unp$arr$field[i]` — one per member of the record type, in the SAME
    /// declaration order [`Self::unpacked_struct_member_ports`] used to build the
    /// callee's N per-member formals (both walk `self.unpacked_struct_layouts[ty]`
    /// with a plain `.iter()`, so the two sides are aligned by construction — the
    /// SAME source list, walked the SAME way). Reuses [`Self::soa_member_field`] for
    /// the per-member net name (the identical rewrite `arr[i].field` already uses),
    /// so a would-be-unregistered field can never happen here (every name comes
    /// straight from that member's own list) — but `collect`ing through the `?` keeps
    /// this all-or-nothing regardless: one failure fails the WHOLE expansion, never a
    /// partial one that would desync formal↔actual arity.
    ///
    /// `None` for anything else — a non-SoA base, a deeper/compound base (already
    /// desugared to a plain member net elsewhere, e.g. `kats[0].mode` is NOT this
    /// shape), or an unregistered array — so the caller leaves the actual UNEXPANDED
    /// and a resulting formal/actual arity mismatch is caught (loud) downstream in
    /// `fill_default_args`.
    pub(crate) fn expand_soa_array_elem_arg(&self, a: &Expr) -> Option<Vec<Expr>> {
        let base = match &a.kind {
            ExprKind::BitSelect { base, .. } | ExprKind::IndexedPart { base, .. } => base,
            _ => return None,
        };
        let ExprKind::Ident(p) = &base.kind else {
            return None;
        };
        if p.segments.len() != 1 {
            return None;
        }
        let arr = p.segments[0].name.as_str();
        let ty = self.record_soa_vars.get(arr)?.clone();
        let members = self.unpacked_struct_layouts.get(&ty)?.clone();
        // IEEE §13.5.1: the actual (and its array index) is evaluated ONCE. This fan-out
        // CLONES the index into each of the N per-member reads, so a SIDE-EFFECTING /
        // non-deterministic index (a call — `kats[nxt()]`) would be evaluated N times →
        // duplicated side effects AND a TORN record read (members read from DIFFERENT
        // elements). The parser rewrites expressions, not statements, so it cannot hoist
        // the index to a temp — reject a non-idempotent index (→ unexpanded → loud arity,
        // correct-or-loud). A pure index (constant / ident / arithmetic / select) contains
        // no call and is idempotent (every clone reads the same element).
        let idx_pure = match &a.kind {
            ExprKind::BitSelect { index, .. } => !Self::expr_has_call(index),
            ExprKind::IndexedPart { offset, width, .. } => {
                !Self::expr_has_call(offset) && !Self::expr_has_call(width)
            }
            _ => false,
        };
        if !idx_pure {
            return None;
        }
        let span = a.span;
        members
            .iter()
            .map(|m| {
                let mnet = self.soa_member_field(arr, &m.name.name)?;
                let mbase = Self::ident_expr(&mnet, span);
                Some(match &a.kind {
                    ExprKind::BitSelect { index, .. } => Expr {
                        kind: ExprKind::BitSelect {
                            base: Box::new(mbase),
                            index: index.clone(),
                        },
                        span,
                    },
                    ExprKind::IndexedPart {
                        offset, width, dir, ..
                    } => Expr {
                        kind: ExprKind::IndexedPart {
                            base: Box::new(mbase),
                            offset: offset.clone(),
                            width: width.clone(),
                            dir: *dir,
                        },
                        span,
                    },
                    _ => unreachable!("matched in the outer fn — BitSelect or IndexedPart only"),
                })
            })
            .collect()
    }

    /// Round-19 (F-struct fix): true if `e`'s subtree contains a function / method /
    /// system / ctor / randomize / array-method CALL (or a `dist` constraint) — any
    /// potential side effect / non-determinism. Used to keep a record-array-element
    /// ACTUAL's index IDEMPOTENT before it is cloned into N per-member reads (a
    /// side-effecting index would evaluate N times → a torn record read; IEEE §13.5.1
    /// requires the actual evaluated once). A pure numeric index (constant / ident /
    /// arithmetic / select / cast) contains no call.
    ///
    /// EXHAUSTIVE (no `_` arm) — matching the discipline of `da.rs`'s
    /// `expr_call_may_write_ident`: a future call-bearing `ExprKind` addition must be a
    /// COMPILE ERROR here, not a silently-widened torn-read window (whole-branch review).
    pub(crate) fn expr_has_call(e: &Expr) -> bool {
        use ExprKind as K;
        match &e.kind {
            // Call-like / side-effecting / non-deterministic ⇒ a non-idempotent index.
            K::Call { .. }
            | K::SysCall { .. }
            | K::MethodCall { .. }
            | K::ClassNew { .. }
            | K::RandomizeWith(_)
            | K::ArrayMethodWith(_)
            | K::Dist { .. } => true,
            // Pure leaves — no call.
            K::IntLit { .. }
            | K::RealLit { .. }
            | K::StrLit { .. }
            | K::PkgScoped { .. }
            | K::Ident(_)
            | K::Null
            | K::Dollar
            | K::Error => false,
            // Compound — recurse into every sub-expression.
            K::Unary { operand, .. } => Self::expr_has_call(operand),
            K::Binary { lhs, rhs, .. } => Self::expr_has_call(lhs) || Self::expr_has_call(rhs),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                Self::expr_has_call(cond)
                    || Self::expr_has_call(then_e)
                    || Self::expr_has_call(else_e)
            }
            K::BitSelect { base, index } => Self::expr_has_call(base) || Self::expr_has_call(index),
            K::PartSelect { base, msb, lsb } => {
                Self::expr_has_call(base) || Self::expr_has_call(msb) || Self::expr_has_call(lsb)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                Self::expr_has_call(base)
                    || Self::expr_has_call(offset)
                    || Self::expr_has_call(width)
            }
            K::Concat { parts } => parts.iter().any(Self::expr_has_call),
            K::Replicate { count, value } => {
                Self::expr_has_call(count) || value.iter().any(Self::expr_has_call)
            }
            K::Paren { inner } => Self::expr_has_call(inner),
            K::MinTypMax { min, typ, max } => {
                Self::expr_has_call(min) || Self::expr_has_call(typ) || Self::expr_has_call(max)
            }
            K::New { size, src } => {
                Self::expr_has_call(size) || src.as_ref().is_some_and(|s| Self::expr_has_call(s))
            }
            K::TimeLit { num, .. } => Self::expr_has_call(num),
            K::NamedArg { value, .. } => value.as_ref().is_some_and(|v| Self::expr_has_call(v)),
            K::Cast { expr, .. } => Self::expr_has_call(expr),
            K::AssignPattern(parts) => parts.iter().any(Self::expr_has_call),
            K::AssignPatternKeyed(parts) => parts.iter().any(|(_, v)| Self::expr_has_call(v)),
        }
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

    /// r18 (E2): if `path` is `var.field.method` where `var.field` is an UNPACKED-struct
    /// MEMBER (a string/other typed net), return the 2-segment path `[$unp$var$field,
    /// method]` so a method call on a struct member (`r.name.substr(a,b)`) becomes the
    /// `net.method(args)` form elaborate already dispatches (a string method on the member
    /// net). `None` for anything else (a non-member receiver, a nested/deeper path) → the
    /// caller keeps the original path (→ a generic hierarchical Call, loud in elaborate).
    pub(crate) fn unpacked_member_method_recv(&self, path: &HierPath) -> Option<HierPath> {
        if path.segments.len() != 3 {
            return None;
        }
        let recv = HierPath {
            segments: path.segments[..2].to_vec(),
            span: path.span,
        };
        let member = self.unpacked_field_ident(&recv)?;
        Some(HierPath {
            segments: vec![member.segments[0].clone(), path.segments[2].clone()],
            span: path.span,
        })
    }
}
