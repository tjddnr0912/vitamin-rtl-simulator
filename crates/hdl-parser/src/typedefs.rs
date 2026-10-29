//! typedefs — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// A user typedef name usable as a NUMERIC cast `T'(e)` (B2) → its
    /// `(width, signed)`. `Some` only for a simple 4-state vector or
    /// enum-with-logic-base typedef whose range folds to a literal; `None`
    /// (→ honest-loud at the `Named` cast arm) for a struct / union / class /
    /// multi-dim-packed / atom (no range, e.g. base-less enum / `int`) / 2-state
    /// (`bit`) typedef — those need per-field or 2-state-coercion semantics the
    /// size+sign desugar cannot reproduce.
    pub(crate) fn simple_typedef_cast(&self, name: &str) -> Option<(i64, bool)> {
        let info = self.typedefs.get(name)?;
        if info.class_name.is_some()
            || !info.packed.is_empty()
            // §3 ⑤: there is no cast to an UNPACKED array, and the size+sign
            // desugar would silently cast to the ELEMENT.
            || !info.unpacked.is_empty()
            || !matches!(info.kind, NetVarKind::Logic | NetVarKind::Reg)
        {
            return None;
        }
        // §3 ⑤ ⓓ (IEEE §6.24.1): a cast to a 4-state packed STRUCT/UNION type is
        // the cast to its packed width with the struct's whole-value signedness —
        // `cap_t'(decoded_cap)` truncates the wider record to the struct's bits
        // (both oracles). A 2-state struct stays loud with the 2-state vector
        // typedef (the size cast would not coerce X/Z).
        if self.struct_layouts.contains_key(name) {
            let w = self.bits_of_type_name(name)?;
            return Some((i64::from(w), info.signed));
        }
        let range = info.range.as_ref()?;
        // §3 ⑤ ⓔ ladder: the bounds fold through the same parse-time constant table a
        // struct member's width does (`try_const_index`, §4.5.414), so a vector
        // typedef whose range names a package constant — `typedef logic [CBOUND_W-1:0]
        // cbound_t;` and its `cbound_t'(addr >> exp)` in ibex_cheriot_pkg — casts
        // instead of declining (a literal range takes the same route it did).
        // §2 🆕 L (aa): a range bound is self-determined (§11.6.1) — `[C+D:0]` over
        // two 4-bit constants is ONE bit in both oracles; the i64 fold cast at 17.
        let msb = self.const_bound(&range.msb)?;
        let lsb = self.const_bound(&range.lsb)?;
        // Direction-agnostic width (overflow-safe `abs_diff`, matching
        // `member_width`); the range direction does not affect the cast VALUE.
        Some((msb.abs_diff(lsb) as i64 + 1, info.signed))
    }

    /// If the current token names a known typedef, return its resolved underlying
    /// type (peek only — the caller commits the decl). `None` ⇒ not a type name.
    /// The full `"pkg::t"` key at the cursor when it names a scope-qualified type
    /// whose package registered it (see the post-package pass in `parse_module_like`
    /// that duplicates each package typedef under a `"pkg::name"` key). `None` for a
    /// bare name, a scoped VALUE ref (`pkg::CONST` — `CONST` is not a type), or an
    /// unknown scope. Keying the FULL `pkg::t` (not the bare final segment) is what
    /// makes a scoped type resolve to the RIGHT package under a cross-package
    /// same-name collision (`pa::t` 8-bit vs `pb::t` 16-bit) — the flat bare-keyed
    /// registry only holds the last-registered `t`, so a bare lookup would be
    /// silent-wrong.
    pub(crate) fn scoped_type_key(&self) -> Option<String> {
        if self.is_ident()
            && self.peek_at(1) == Some(TokenKind::ColonColon)
            && matches!(
                self.peek_at(2),
                Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
            )
        {
            let key = format!("{}::{}", self.cur_text(), self.text_at(2));
            // Round-9: an unpacked struct type is NOT in `typedefs` (it has no flat
            // TypeInfo), so accept an `unpacked_struct_layouts` key too — otherwise
            // a scoped `pkg::rec_t` decl / `pkg::rec_t'()` would not resolve. Every
            // packed-path caller re-checks `typedefs` after this, so a scoped
            // unpacked key still routes only to the unpacked decl path.
            if self.typedefs.contains_key(&key) || self.unpacked_struct_layouts.contains_key(&key) {
                return Some(key);
            }
        }
        None
    }

    /// The map key a type-name consumer should use for the parser's type sub-maps
    /// (`struct_layouts`/`enum_defs`/`union_type_names`/`var_struct`/`var_enum`):
    /// the full `"pkg::t"` for a scoped type (matching the twin key registered for
    /// the package), else the bare name. This keeps a scoped struct/enum type's
    /// layout / label / union bindings resolving to the right package's definition.
    pub(crate) fn type_name_key(&self) -> String {
        self.scoped_type_key()
            .unwrap_or_else(|| self.cur_text().to_string())
    }

    pub(crate) fn peek_typedef_name(&self) -> Option<TypeInfo> {
        if let Some(key) = self.scoped_type_key() {
            // Scoped `pkg::t`: resolve the package-scoped twin key (collision-safe).
            return self.typedefs.get(&key).cloned();
        }
        if self.is_ident() {
            return self.typedefs.get(self.cur_text()).cloned();
        }
        None
    }

    /// Inside a procedural block's decl region, a `<typedef_name> <ident>` opener is
    /// unambiguously a local declaration — no statement begins that way (`e = x` is
    /// invalid since `e` is a type, `e::A`/`e'(…)` have a non-ident second token).
    /// Returns the type only for that shape, so a typedef name used otherwise falls
    /// through to the statement region. (The module-item parser needs no such guard:
    /// no statements appear there.) A scoped type `pkg::t name` (qualifier length
    /// `q`) requires the NAME after the final segment (`peek_at(q+1)`), so a scoped
    /// value ref / cast (`pkg::t'(…)`) still falls through to the statement region.
    pub(crate) fn peek_block_typedef_decl(&self) -> Option<TypeInfo> {
        let info = self.peek_typedef_name()?;
        let q = self.scope_qualifier_len();
        if matches!(
            self.peek_at(q + 1),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            Some(info)
        } else {
            None
        }
    }

    /// A procedural body-local typedef DEFINITION (`typedef logic [3:0] t;` inside
    /// a begin/function/task body — IEEE §6.18). The cursor is at the `typedef`
    /// keyword. Registers the name in `self.typedefs` (and `struct_layouts` for a
    /// struct/union) so subsequent local decls resolve it; an alias/struct/union
    /// typedef emits no runtime decl and elaborate needs nothing from its node, so
    /// it is discarded (returns `None`). An ENUM typedef additionally needs
    /// elaborate-side label-constant registration (the AST node carries the labels)
    /// — when `allow_enum` (a function/task body, which has a `body_enums` slot to
    /// carry it) the enum node is RETURNED so the caller can thread it to elaborate;
    /// otherwise (a bare `begin/end` block, no carrier) it stays honest-loud (define
    /// it at module scope). The caller snapshots/restores the registries for scope.
    pub(crate) fn parse_body_typedef_def(&mut self, allow_enum: bool) -> Option<TypedefDecl> {
        let is_enum = matches!(
            self.peek_at(1),
            Some(TokenKind::Word(WordKind::Keyword(Kw::Enum)))
        );
        if is_enum && !allow_enum {
            self.error(
                "an alias / struct / union typedef in a begin/function/task body (a body-local enum typedef is unsupported in v1 — define it at module scope)",
            );
            // GAP-B: emit the loud reject, then PARSE the enum typedef NORMALLY
            // (registering the type NAME + consuming all its tokens) so the rest
            // of the body — which references the type (`e v; v = e'(x);`) — parses
            // without cascading into spurious follow-on errors. The design still
            // fails (the loud error aborts before elaborate). "emit + consume".
            let _ = self.parse_typedef();
            return None;
        }
        // Registration into the parser scratch maps is the functional effect (so a
        // later `t v;`/`t'(x)` resolves). An alias/struct/union needs nothing from
        // elaborate → return None; a body enum (allow_enum) RETURNS its node so the
        // caller carries the labels to elaborate (round-5 Gap B).
        match self.parse_typedef() {
            Some(ModuleItem::Typedef(td)) if is_enum => Some(td),
            _ => None,
        }
    }

    /// `typedef …;` of any kind, in any scope. The declared name joins the scope's own
    /// type names (`scope_type_names`), so a LATER wildcard import in the same scope
    /// never rebinds it (IEEE §26.3).
    pub(crate) fn parse_typedef(&mut self) -> Option<ModuleItem> {
        let it = self.parse_typedef_inner();
        if let Some(ModuleItem::Typedef(td)) = &it {
            self.scope_type_names.insert(td.name.name.clone());
            self.note_type_rebound(&td.name.name);
        }
        it
    }

    /// `typedef enum [base] { L0, L1 = expr, … } name;` (Phase-2). Registers
    /// `name` in `self.typedefs` (so a later `name var;` parses) and returns the
    /// AST node so elaborate can register the labels as integer constants.
    fn parse_typedef_inner(&mut self) -> Option<ModuleItem> {
        let start = self.cur_span();
        self.bump(); // `typedef`
        if self.at_kw(Kw::Struct) {
            return self.parse_typedef_struct(start);
        }
        if self.at_kw(Kw::Union) {
            return self.parse_typedef_union(start);
        }
        if !self.at_kw(Kw::Enum) {
            // `typedef logic [7:0] byte_t;` — plain alias to a net/var type.
            if self.net_var_kind().is_some() {
                return self.parse_typedef_alias(start);
            }
            // `typedef base_t alias_t;` — a chained alias of an EXISTING typedef.
            if let Some(info) = self.peek_typedef_name() {
                return self.parse_typedef_chained_alias(start, info);
            }
            // unpacked struct / union forms are out of v1 scope.
            self.error("`enum`, `struct packed`, or a type after `typedef`");
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        self.bump(); // `enum`
                     // Optional packed base: `enum logic [1:0] {…}` or `enum [1:0] {…}`.
        let mut base_signed: Option<bool> = None;
        // §4.5.154: for a rangeless ATOM (`byte`/`shortint`/`longint`) or bare vector
        // (`logic`/`bit`/`reg`) enum base, `atom_kind` records the REAL base kind so the
        // enum's `TypeInfo` preserves it (correct width via the atom machinery + 2-state-
        // ness + keeps `T'(e)` casts / the enum-base guard on their kind/range-gated
        // paths), while `base` below carries a SYNTHESIZED width range consumed ONLY by
        // the AST label-width path (`enum_base_width`). `None` = the base is not one of
        // those synthesized kinds (explicit-range vector, `int`/`integer`, base-less).
        let mut atom_kind: Option<NetVarKind> = None;
        // §3.b cont-array-typedef-elem: a 2-state VECTOR base (`enum bit [7:0]`, or a
        // `bit [N]` typedef base) is recorded as 4-state `logic` below, so the enum's
        // `TypeInfo::layout_exact` is false, as it is for a base bound not written as a
        // literal. An atom or bare-kind base keeps its kind and has no written bound.
        let mut base_exact = true;
        let base = if let Some(nvk) = self.net_var_kind() {
            self.bump(); // base kind keyword (logic/reg/integer/…)
                         // §4.5.153: capture an explicit `signed`/`unsigned` on the built-in enum
                         // base so the enum's WHOLE value signedness (`%0d`/compare/sign-extend) is
                         // honored. Mirrors the struct/union §4.5.152 fix — same `TypeInfo.signed`
                         // funnel. `Some(true)`=`signed`, `Some(false)`=`unsigned`, `None`=absent →
                         // each arm below applies its own default (vector base=unsigned, atom/
                         // base-less=signed-`int`), so a qualifier-less enum is byte-identical.
            base_signed = self.opt_signed();
            match self.opt_range() {
                // Explicit packed range (`enum logic [3:0] …`): vector base (default unsigned).
                // §4.5.156 (§3 全 site): a NON-vector base (`enum byte [3:0]`) may not carry a
                // packed range — reject (a vector base logic/reg/bit is fine).
                Some(r) => {
                    self.reject_packed_dims_on_nonvector(nvk, true);
                    base_exact =
                        nvk != NetVarKind::Bit && self.bounds_source_literal(Some(&r), &[]);
                    Some(r)
                }
                // No explicit range → an ATOM (`byte`/`shortint`/`longint`) or a bare vector kind
                // (`logic`/`bit`/`reg`). The prior model collapsed ALL of these onto the 32-bit-
                // `int` `None` arm below, so `$bits`/`%b`/concat width were wrong for BOTH the enum
                // variable and its labels (`enum byte`=32 not 8, `enum logic`=32 not 1). Record the
                // real kind (`atom_kind`, drives the variable's `TypeInfo`) and synthesize the
                // kind's true width as the AST `base` (drives label width), seeding the per-kind
                // default sign (atoms signed, bare vectors unsigned). `int`/`integer`/`time` →
                // `_ => None` (unchanged 32-bit signed int).
                None => {
                    let synth = match nvk {
                        // 2-state signed atoms.
                        NetVarKind::Byte => Some((7u32, true)),
                        NetVarKind::Shortint => Some((15, true)),
                        NetVarKind::Int => Some((31, true)),
                        NetVarKind::Longint => Some((63, true)),
                        // `integer` = 4-state 32-bit signed (Verilog legacy); `time` = 4-state
                        // 64-bit UNSIGNED. Preserving their real kind keeps `integer` 4-state
                        // (X-init) and fixes `time` width (was 32). §4.5.154.
                        NetVarKind::Integer => Some((31, true)),
                        NetVarKind::Time => Some((63, false)),
                        // Bare vector kinds (no range) = 1-bit unsigned; `bit` is 2-state.
                        NetVarKind::Logic | NetVarKind::Reg | NetVarKind::Bit => Some((0, false)),
                        _ => None,
                    };
                    match synth {
                        Some((hi, def_signed)) => {
                            atom_kind = Some(nvk);
                            base_signed = Some(base_signed.unwrap_or(def_signed));
                            Some(Self::dec_range(hi))
                        }
                        None => None,
                    }
                }
            }
        } else if let Some(info) = self.peek_typedef_name() {
            // §3 ⑤ⓕ: an enum base carries no shape slot (`EnumDecl` stores an
            // `Option<Range>` and the parse-time `base_signed`), so a type-parameter
            // base keeps `T`'s STRICT guard.
            self.note_uncarried_shape_use(&info);
            // `enum b_t {…}` — the base type is an existing typedef name. Support a
            // SIMPLE UNSIGNED vector typedef (`logic`/`bit`/`reg` `[N]`); the enum
            // then stores as that vector (its range). A SIGNED *typedef* base (the
            // built-in `enum logic signed [N]` path IS honored via `base_signed`
            // above — §4.5.153; a signed typedef-name base stays honest-loud), an
            // atom (`int`/`byte` — signed, no explicit range), or a struct / class /
            // multi-dim-packed typedef cannot be represented by the enum's
            // `Option<Range>` base model — honest-loud.
            let nm = self.type_name_key();
            if self.struct_layouts.contains_key(&nm)
                || info.class_name.is_some()
                || info.signed
                || !info.packed.is_empty()
                // §3 ⑤: both oracles REFUSE an unpacked array as an enum base
                // (§6.19 wants an integer atom or vector type).
                || !info.unpacked.is_empty()
                || info.range.is_none()
            {
                self.error(
                    "a simple unsigned vector typedef (logic/bit/reg [N]) as an enum base (a signed / atom / struct / class / multi-dim typedef base is unsupported in v1)",
                );
                self.synchronize();
                return Some(ModuleItem::Error(start.to(self.prev_span())));
            }
            self.eat_scope_qualifier();
            self.bump(); // the typedef-name token
            base_exact = !Self::member_kind_two_state(info.kind) && info.layout_exact;
            info.range
        } else {
            let r = self.opt_range();
            base_exact = self.bounds_source_literal(r.as_ref(), &[]);
            r
        };
        self.expect(TokenKind::LBrace, "'{' for enum body");
        let mut labels = Vec::new();
        if self.peek() != Some(TokenKind::RBrace) {
            loop {
                let name = self.ident()?;
                let value = if self.eat(TokenKind::Eq) {
                    Some(self.expr(0))
                } else {
                    None
                };
                // §6.19: a label is a declaration of the scope holding the typedef, so
                // from here on it hides an outer constant, multi-dimensional packed
                // parameter or struct variable of its name in every parse-time table
                // (§3 ⑤ⓗ). Without it, a generate block's label `A` beside a module
                // `localparam A = 2` folded `C = A + 4` to 6 for `.first()` and
                // rewrote `P[1]` through the module's `[1:0][3:0] P` layout.
                self.unbind_struct_enum_name(&name.name);
                labels.push(EnumLabel { name, value });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RBrace, "'}' to close enum body");
        let tname = self.ident()?;
        self.expect(TokenKind::Semi, "';'");
        // Enum storage is `int` (32-bit signed) unless a packed base range was
        // given, in which case a `logic` vector of that range.
        let info = if let Some(ak) = atom_kind {
            // §4.5.154: a synthesized ATOM (`byte`/`shortint`/`longint`) or bare vector
            // (`logic`/`bit`/`reg`) base — preserve the REAL kind (range None) so the enum
            // variable is sized + state-typed exactly like a plain `byte`/`logic`/… decl.
            // `base_signed` was resolved to `Some(..)` in the synth arm above. The AST
            // `base` (synthesized range) drives the separate label-width path.
            TypeInfo {
                kind: ak,
                signed: base_signed.unwrap_or(false),
                range: None,
                packed: Vec::new(),
                class_name: None,
                unpacked: Vec::new(),
                shape_param: None,
                enum_type: true,
                layout_exact: base_exact,
                pattern_members: None,
                layout_reg: None,
                bind: 0,
            }
        } else {
            match &base {
                // Vector base (`enum logic [N] …`): defaults UNSIGNED, honors explicit `signed`.
                Some(r) => TypeInfo {
                    kind: NetVarKind::Logic,
                    signed: base_signed.unwrap_or(false),
                    range: Some(r.clone()),
                    packed: Vec::new(),
                    class_name: None,
                    unpacked: Vec::new(),
                    shape_param: None,
                    enum_type: true,
                    layout_exact: base_exact,
                    pattern_members: None,
                    layout_reg: None,
                    bind: 0,
                },
                // Base-less `enum {…}` (and any illegal non-integral base that slipped through):
                // the default enum base is `int` = 32-bit signed 2-state (§4.5.154 — was the
                // 4-state `Integer`, so an uninitialized base-less enum read X instead of 0).
                // Every real integral base kind is now carried by `atom_kind` above; `int`/
                // `integer`/`time` route through it, so only the base-less/illegal case lands
                // here. Defaults SIGNED, honors explicit `unsigned` — §4.5.153.
                None => TypeInfo {
                    kind: NetVarKind::Int,
                    signed: base_signed.unwrap_or(true),
                    range: None,
                    packed: Vec::new(),
                    class_name: None,
                    unpacked: Vec::new(),
                    shape_param: None,
                    enum_type: true,
                    layout_exact: base_exact,
                    pattern_members: None,
                    layout_reg: None,
                    bind: 0,
                },
            }
        };
        // §4.5.158: the enum's DECLARED sign (the `TypeInfo.signed` §4.5.153/154 resolves)
        // rides into the AST `Enum` node so a label reference is lowered with the enum's
        // sign, not the value-inferred one — a positive label of a signed enum stays signed.
        let enum_signed = info.signed;
        self.typedefs.insert(tname.name.clone(), info);
        self.stamp_type_bind(&tname.name);
        // Const-foldable enum base WIDTH in bits, for the label-range check below.
        // `None` = skip the check (fail-open, never over-rejects) when the base range
        // is not a literal (`enum logic [N-1:0]`) or is >64 bits wide. Base-less
        // `enum {…}` = `int` (32-bit signed). Widths 1..=64 are checked with i128
        // bounds (overflow-safe): a SIGNED 64-bit base (`longint`) admits any i64, but
        // an UNSIGNED 64-bit base (`time`, `bit [63:0]`) still rejects a negative label
        // (not representable in `[0, 2^64-1]`) — so width 64 is checked, not skipped.
        let base_w: Option<u32> = match &base {
            None => Some(32),
            Some(r) => match (Self::const_lit(&r.msb), Self::const_lit(&r.lsb)) {
                (Some(msb), Some(lsb)) => match msb.checked_sub(lsb) {
                    Some(d) if d.unsigned_abs() < 64 => Some(d.unsigned_abs() as u32 + 1),
                    _ => None,
                },
                _ => None,
            },
        };
        // SV §6.19.5 enum-method support: fold each label's value (running counter,
        // reset by an explicit literal-foldable `= expr`). Record the ordered
        // (label, value) list ONLY if EVERY value folds (`const_lit`) — an enum with
        // a non-literal label value is omitted, so `x.method()` on it stays loud.
        {
            let mut folded: Vec<(String, i64)> = Vec::with_capacity(labels.len());
            let mut counter: i64 = 0;
            let mut foldable = true;
            for lab in &labels {
                let v = match &lab.value {
                    None => counter,
                    // The enum-label fold OPTS IN to based literals (`A = 4'h3`);
                    // without it the whole enum stayed out of `enum_defs` and
                    // every enum method on it went loud with a misleading
                    // "hierarchical function call" message.
                    // §0 T2: …and to a module-scope `localparam` (`A = L`, `A = L+1`).
                    // ⚠️ NOT to a `parameter`, and that is the whole safety argument:
                    // measured, an instance override CHANGES the label values —
                    // `m #(.K(9))` on `enum { A = K, B = K+1 }` makes iverilog print
                    // `10`/`first=9`, not `4`/`3` — so folding one at PARSE time, before
                    // any override is known, would be silently wrong. `try_const_index`
                    // is exactly the right predicate because `const_locals` already
                    // carries that guarantee ("A `parameter` is overridable → never
                    // recorded") and it is the same fold a constant generate index uses,
                    // so a `localparam` cannot mean one thing in `g[L]` and another here.
                    // A `parameter` label keeps declining, which leaves the whole enum
                    // out of `enum_defs` and its methods loud — correct-or-loud, and the
                    // real fix for that half is moving the enum-method desugar out of the
                    // parser, which is architectural (recorded, not attempted here).
                    Some(e) => match Self::const_lit_enum(e, enum_signed)
                        .or_else(|| self.try_const_index(e))
                    {
                        Some(v) => v,
                        None => {
                            foldable = false;
                            break;
                        }
                    },
                };
                // §6.19: a label value outside the enum base type's representable range
                // is an error (IEEE — iverilog rejects "too large"/"negative"/overflow).
                // vita previously TRUNCATED it silently (`{X=16}` in `[3:0]` read 0). Loud
                // it — only for const-foldable values against a known base width, so the
                // check never fires on a legitimately-typed label (correct-or-loud).
                // At width 64 the distinction is PROVENANCE, not magnitude. An
                // explicitly WRITTEN negative in an unsigned 64-bit base is an error
                // (iverilog: "has a negative value"), but an AUTO-INCREMENTED label
                // that wraps past `64'sh7FFF_FFFF_FFFF_FFFF` is a perfectly good
                // `logic [63:0]` pattern that iverilog accepts — and it only became
                // visible here once sized labels started folding. So the check runs
                // for every explicit value, and for auto-increment only where the
                // width can actually overflow.
                let explicit = lab.value.is_some();
                if let Some(w) = base_w.filter(|w| *w < 64 || explicit) {
                    let vi = v as i128;
                    let (lo, hi): (i128, i128) = if enum_signed {
                        (-(1i128 << (w - 1)), (1i128 << (w - 1)) - 1)
                    } else {
                        (0, (1i128 << w) - 1)
                    };
                    if vi < lo || vi > hi {
                        let sp = lab.value.as_ref().map(|e| e.span).unwrap_or(lab.name.span);
                        self.error_at(sp, "an enum label value that fits the enum base type");
                    }
                }
                folded.push((lab.name.name.clone(), v));
                counter = v.wrapping_add(1);
            }
            if foldable && !folded.is_empty() {
                // §5.2 row 67b: the enum's declaration key (`decl_key.rs`).
                let reg = self.next_type_reg();
                self.register_decl(reg, DeclType::Enum(folded.clone()));
                self.enum_defs.insert(
                    tname.name.clone(),
                    EnumDef {
                        labels: folded,
                        reg,
                    },
                );
            }
        }
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Enum {
                base,
                signed: enum_signed,
                labels,
            },
            span: start.to(self.prev_span()),
        }))
    }

    /// `typedef <kind> [signed] [range] [packed] name;` — a plain type alias.
    /// `start` is the span of the leading `typedef` keyword (already consumed).
    pub(crate) fn parse_typedef_alias(&mut self, start: Span) -> Option<ModuleItem> {
        let kind = self.net_var_kind().unwrap();
        self.bump(); // kind keyword
        let signed = self.signed_eff(Some(kind));
        let range = self.opt_range();
        let packed = self.opt_packed_dims();
        self.reject_packed_dims_on_nonvector(kind, range.is_some() || !packed.is_empty());
        let layout_exact = self.bounds_source_literal(range.as_ref(), &packed);
        let tname = self.ident()?;
        // §3 ⑤: UNPACKED dims after the NAME — `typedef logic [7:0] a_t [0:3];`.
        // Both oracles accept it and vita's own machinery already runs the shape
        // (`logic [7:0] y [0:3];` is correct today); what was missing is a carrier
        // from here to the declaration, so the `[` raised E2002 at the typedef.
        // Parsed with `parse_decl_name_list`'s own loop, because a declarator's
        // dims and a typedef's are the same grammar and must not drift.
        let mut unpacked = Vec::new();
        while self.at_dim_start() {
            match self.parse_dim() {
                Some(d) => unpacked.push(d),
                None => break,
            }
        }
        self.expect(TokenKind::Semi, "';'");
        self.typedefs.insert(
            tname.name.clone(),
            TypeInfo {
                kind,
                signed,
                range: range.clone(),
                packed: packed.clone(),
                class_name: None,
                unpacked,
                shape_param: None,
                enum_type: false,
                layout_exact,
                pattern_members: None,
                layout_reg: None,
                bind: 0,
            },
        );
        self.stamp_type_bind(&tname.name);
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Alias {
                kind,
                signed,
                range,
                packed,
            },
            span: start.to(self.prev_span()),
        }))
    }

    /// `typedef base_t alias_t;` — a chained alias of an EXISTING typedef
    /// (IEEE §6.18). The new name inherits the base's full registration — type
    /// info plus any struct/union layout or enum-method binding — so a later
    /// `alias_t v;` / `v.field` / `v.next` resolves exactly as `base_t v;` would.
    /// `start` is the span of the leading `typedef` (already consumed); the cursor
    /// is on the base typedef-name token, whose resolved `info` is passed in.
    /// Adding packed or unpacked dimensions to an aliased type
    /// (`typedef base_t [3:0] a_t;` / `typedef base_t a_t [4];`) needs type
    /// composition not in v1 — honest-loud.
    pub(crate) fn parse_typedef_chained_alias(
        &mut self,
        start: Span,
        info: TypeInfo,
    ) -> Option<ModuleItem> {
        let base_name = self.type_name_key(); // "pkg::t" (scoped) / bare — keys the sub-maps
        self.eat_scope_qualifier(); // skip an optional `pkg::` before the base type name
        self.bump(); // base typedef name
                     // honest-loud: PACKED dims before the new name (`typedef base_t [3:0] a_t;`).
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(
                "a simple chained typedef alias (adding packed dimensions to an aliased type is unsupported in v1)",
            );
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        let tname = self.ident()?;
        // honest-loud: UNPACKED dims after the new name (`typedef base_t a_t [4];`).
        if self.peek() == Some(TokenKind::LBracket) {
            self.error(
                "a simple chained typedef alias (an unpacked-array typedef of an aliased type is unsupported in v1)",
            );
            self.synchronize();
            return Some(ModuleItem::Error(start.to(self.prev_span())));
        }
        self.expect(TokenKind::Semi, "';'");
        let alias = tname.name.clone();
        // Mirror the base's registration under the new name. Each side-map is SET
        // when the base has that property and CLEARED otherwise, so a cross-module
        // same-name stale entry (the union-desync hazard) cannot leak through.
        // §5.2 row 36: the alias is a registration of its own, and it names a packed
        // struct only when that is what its base names where the alias stands — not
        // when an unpacked record declared later shadows a same-named struct, whose
        // entry here it would otherwise copy (`typedef_is_freshest`).
        let mut alias_info = info.clone();
        if !self.typedef_is_freshest(&base_name) {
            alias_info.layout_reg = None;
        }
        self.typedefs.insert(alias.clone(), alias_info);
        self.stamp_type_bind(&alias);
        match self.struct_layouts.get(&base_name).cloned() {
            Some(layout) => {
                self.struct_layouts.insert(alias.clone(), layout);
            }
            None => {
                self.struct_layouts.remove(&alias);
            }
        }
        match self.enum_defs.get(&base_name).cloned() {
            Some(defs) => {
                self.enum_defs.insert(alias.clone(), defs);
            }
            None => {
                self.enum_defs.remove(&alias);
            }
        }
        if self.union_type_names.contains(&base_name) {
            self.union_type_names.insert(alias.clone());
        } else {
            self.union_type_names.remove(&alias);
        }
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Alias {
                kind: info.kind,
                signed: info.signed,
                range: info.range,
                packed: info.packed,
            },
            span: start.to(self.prev_span()),
        }))
    }

    pub(crate) fn parse_typedef_struct(&mut self, start: Span) -> Option<ModuleItem> {
        self.parse_struct_type(start, None)
    }

    /// The type name after a struct/union body: the typedef's `name ;`, or for an
    /// anonymous member type the minted key (nothing to consume).
    pub(crate) fn anon_or_typedef_name(
        &mut self,
        start: Span,
        anon: Option<String>,
    ) -> Option<Ident> {
        match anon {
            Some(name) => Some(Ident {
                name,
                span: start.to(self.prev_span()),
            }),
            None => {
                let tname = self.ident()?;
                self.expect(TokenKind::Semi, "';'");
                Some(tname)
            }
        }
    }

    /// The body of `parse_typedef_struct`, shared with an ANONYMOUS packed struct
    /// member type (`parse_anon_member_type`): `anon` is then the minted key the
    /// layout is registered under, and no name or `;` follows the closing `}`.
    pub(crate) fn parse_struct_type(
        &mut self,
        start: Span,
        anon: Option<String>,
    ) -> Option<ModuleItem> {
        self.bump(); // `struct`
        let packed = self.eat_kw(Kw::Packed);
        // `struct packed signed` (§7.2.1): the qualifier sets the WHOLE-struct value
        // signedness (used when the struct is read as one value — display / compare /
        // arithmetic / arithmetic-shift / sign-extend-on-assign). The member LAYOUT is
        // unaffected (offsets ignore sign) and each member keeps its OWN signedness, so
        // `s.field` reads stay member-typed. Absent the keyword ⇒ unsigned (byte-identical
        // to the pre-existing behaviour for every non-`signed` struct).
        let struct_signed = if packed {
            self.opt_signed().unwrap_or(false)
        } else {
            false
        };
        let (members, nested_keys, shape_keys, each_exact) =
            self.parse_struct_member_list(packed)?;
        let layout_exact = Self::members_layout_exact(&members, each_exact);
        let tname = self.anon_or_typedef_name(start, anon)?;
        if !packed {
            // §3 ⑤ ⓓ: a packed-struct member inside an UNPACKED record would need
            // the record's per-member net to carry a layout — not in v1, loud.
            if let Some(i) = nested_keys.iter().position(Option::is_some) {
                if let Some(m) = members.get(i) {
                    self.error_at(
                        m.span,
                        "a non-struct member type in an unpacked struct (a packed-struct member of an unpacked struct is unsupported in v1)",
                    );
                }
            }
            // Round-9: an UNPACKED struct (record). Members keep their OWN types (a
            // `string`/`int` member can't share a flat packed vector), so a scalar
            // variable desugars to N independent member nets `k$field` (there is no
            // aggregate storage in v1). Record the member layout and emit a
            // `TypedefKind::Struct` node (elaborate treats a struct typedef as a
            // no-op) so the package `endpackage` twin loop registers `pkg::T`. An
            // unpacked-array VARIABLE of this type, a decl-init / `'{…}` pattern, a
            // whole-struct copy/compare, and a tf-port all stay LOUD — v1 supports
            // the SCALAR record only.
            // §5.2 row 67b: the record's number is also its declaration key.
            let bind = self.next_type_reg();
            self.register_decl(bind, DeclType::Record(members.clone()));
            self.unpacked_struct_layouts.insert(
                tname.name.clone(),
                RecordDef {
                    members: members.clone(),
                    reg: bind,
                },
            );
            self.unpacked_bind.insert(tname.name.clone(), bind);
            return Some(ModuleItem::Typedef(TypedefDecl {
                name: tname,
                kind: TypedefKind::Struct { members },
                span: start.to(self.prev_span()),
            }));
        }
        // Compute each member width. A named integer-atom kind (`int`/`byte`/…)
        // carries a fixed width from its TYPE; a vector kind (`bit`/`logic`) sizes
        // from a constant-literal range (`None` ⇒ 1).
        // §3 ⑤ ⓒ: when a member's width does not fold, the WHOLE struct takes the
        // symbolic layout (`register_sym_struct`) — or reports the member.
        if members
            .iter()
            .any(|m| !matches!(self.member_flat_dims(m.kind, &m.range, &m.packed_dims), Some((f, _)) if f > 0))
        {
            return self.register_sym_struct(
                start,
                members,
                nested_keys,
                shape_keys,
                tname,
                struct_signed,
            );
        }
        let mut widths = Vec::with_capacity(members.len()); // (flat_width, elem_stride)
        for m in &members {
            match self.member_flat_dims(m.kind, &m.range, &m.packed_dims) {
                Some((flat, stride)) if flat > 0 => widths.push((flat, stride)),
                _ => {
                    self.error_at(
                        m.span,
                        "struct member width must be a named integer type or a \
                         constant-literal range in v1",
                    );
                    widths.push((1, 1));
                }
            }
        }
        let total: u32 = widths.iter().map(|(f, _)| *f).sum();
        // Lay out MSB-first: first member occupies the high bits.
        let mut off = total;
        let mut fields = Vec::with_capacity(members.len());
        for ((m, (w, stride)), nested) in members.iter().zip(&widths).zip(&nested_keys) {
            off -= *w;
            fields.push((
                m.name.name.clone(),
                off,
                *w,
                self.member_ascending(&m.range),
                m.signed,
                Self::member_kind_two_state(m.kind),
                self.member_dbase(&m.range),
                *stride,
                nested.clone(),
            ));
        }
        // §3.a ⑤ⓚ: the members a declaration of this type records, when the layout is
        // exact and flat (no packed-struct member).
        let pattern_members =
            (layout_exact && nested_keys.iter().all(Option::is_none)).then(|| {
                fields
                    .iter()
                    .map(|f| hdl_ast::PatternMember {
                        name: f.0.clone(),
                        width: f.2,
                        two_state: f.5,
                    })
                    .collect::<Vec<_>>()
            });
        let reg = self.next_type_reg();
        let layout = StructLayout { fields, reg };
        self.register_decl(
            reg,
            DeclType::Packed {
                layout: layout.clone(),
                union: false,
            },
        );
        self.struct_layouts.insert(tname.name.clone(), layout);
        // If a union with the same name was defined in an earlier module, retract it
        // from union_type_names so this struct definition wins (consistent with
        // struct_layouts last-writer-wins semantics; otherwise a later same-named
        // struct var would be wrongly excluded from '{…} pattern desugar).
        self.union_type_names.remove(&tname.name);
        // §7.2.1: an all-2-state struct is itself 2-state — back it with a 2-state
        // `bit` vector so it defaults to 0, not X (matches iverilog). Any 4-state
        // member (`logic`/`integer`/`time`/net) makes the whole struct 4-state.
        let struct_kind =
            if !members.is_empty() && members.iter().all(|m| Self::member_kind_two_state(m.kind)) {
                NetVarKind::Bit
            } else {
                NetVarKind::Logic
            };
        self.typedefs.insert(
            tname.name.clone(),
            TypeInfo {
                kind: struct_kind,
                signed: struct_signed,
                range: Some(Self::dec_range(total.saturating_sub(1))),
                packed: Vec::new(),
                class_name: None,
                unpacked: Vec::new(),
                shape_param: None,
                enum_type: false,
                layout_exact,
                pattern_members,
                layout_reg: Some(reg),
                bind: 0,
            },
        );
        self.stamp_type_bind(&tname.name);
        Some(ModuleItem::Typedef(TypedefDecl {
            name: tname,
            kind: TypedefKind::Struct { members },
            span: start.to(self.prev_span()),
        }))
    }
}

impl Parser<'_, '_> {
    /// `pkg::type_name <name>` in a TYPE position where `pkg::type_name` resolves to
    /// nothing — almost always a source file that was not passed to the tool.
    ///
    /// ⭐ Without this the shape simply did not parse, and the cascade was the whole
    /// cost: one missing `rtl/sha2_pkg.sv` produced ELEVEN `E2002`s, the first at the
    /// `::` ("expected ')' closing tf-port list"), the rest at perfectly correct
    /// declarations further down that only failed because the port list never closed.
    /// The package's NAME — the single fact that identifies the missing file —
    /// appeared in none of them.
    ///
    /// The position is not ambiguous: a tf-port item is `[direction] data_type
    /// identifier`, so `IDENT :: IDENT IDENT` there can only be a scope-qualified
    /// type. Recognising it costs one error that says which package, and consuming the
    /// three tokens lets the rest of the declaration parse — which is what turns
    /// eleven errors into one.
    ///
    /// Returns `true` when it fired (the caller then has a type-less port to name).
    pub(crate) fn reject_unknown_scoped_type(&mut self) -> bool {
        if self.scoped_type_key().is_some() || !self.is_ident() {
            return false;
        }
        if self.peek_at(1) != Some(TokenKind::ColonColon) {
            return false;
        }
        if !matches!(
            self.peek_at(2),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            return false;
        }
        // The 4th token must be the declared NAME — that is what makes this a type
        // position rather than a `pkg::CONST` value reference.
        if !matches!(
            self.peek_at(3),
            Some(TokenKind::Word(WordKind::Ident)) | Some(TokenKind::EscapedIdent)
        ) {
            return false;
        }
        let (pkg, ty) = (self.cur_text().to_string(), self.text_at(2).to_string());
        // One message for both causes, because from here they are indistinguishable
        // and both point at the same fix: `{pkg}` was never declared, or it was and
        // has no `{ty}`. Either way the file that would supply it is not in this run.
        self.error_owned(format!(
            "a type this compilation declares — `{pkg}::{ty}` names no typedef in any \
             package called `{pkg}` (is a source file missing from the file list?)"
        ));
        self.bump(); // pkg
        self.bump(); // ::
        self.bump(); // type name
        true
    }
}
