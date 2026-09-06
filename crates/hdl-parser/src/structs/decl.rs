//! unpacked-struct DECLARATION parsing — split out of `structs.rs` to hold it under
//! the 1000-line module cap. A second inherent `impl` block on `Parser`, which is
//! legal because this is an inherent impl, not a trait impl.

use super::*;

impl Parser<'_, '_> {
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
                self.member_ascending(&m.range),
                m.signed,
                Self::member_kind_two_state(m.kind),
                self.member_dbase(&m.range),
                1u32, // elem_stride: multi-dim members were rejected above
                None, // a nested struct member never reaches a record layout
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
                    // r18 (Fix A) SoA: a FIXED array / unbounded QUEUE of a NON-packable
                    // record (a PARAMETER-width member `[ADDR_W-1:0]`, or a MIXED 2-/4-state
                    // record `{int; logic}`, or a string/real member) → per-member native
                    // arrays/queues `$unp$var$field`, exactly like the dynamic-array SoA
                    // fallback above. Each `$unp$var$field` carries the member's RAW range,
                    // so a param width resolves per-instance at elaborate (a packed vector
                    // would need a frozen parse-time width — silent-wrong under override).
                    // Element access `var[i].field` → `$unp$var$field[i]` and queue methods
                    // (`push_back`/`pop_*`/`insert`/`delete`) fan out per field at the use
                    // sites (`try_soa_queue_method_stmt`, `try_soa_assign`). All-or-loud: a
                    // member with no array form (nested struct / event / class) stays loud.
                    if members.iter().all(|m| Self::member_kind_soa_ok(m.kind)) {
                        self.record_soa_vars
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
                        }
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
    pub(crate) fn member_ascending(&self, range: &Option<Range>) -> bool {
        match range {
            Some(r) => match (self.const_bound(&r.msb), self.const_bound(&r.lsb)) {
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
    /// §3 ⑤ ⓓ: both fold through `try_const_index`, the SAME evaluator
    /// `member_width` uses — a `[0:W-1]` member whose width folds must also be
    /// seen as ascending here, or its sub-selects would be mirrored silently.
    pub(crate) fn member_dbase(&self, range: &Option<Range>) -> i64 {
        match range {
            Some(r) => match (self.const_bound(&r.msb), self.const_bound(&r.lsb)) {
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
        // A union, and a symbolic-layout struct (§3 ⑤ ⓒ), take no `'{…}` desugar.
        if !self.union_type_names.contains(struct_name)
            && !self.sym_struct_layouts.contains_key(struct_name)
        {
            self.struct_scalar_vars.insert(name.to_string());
        }
    }
}
