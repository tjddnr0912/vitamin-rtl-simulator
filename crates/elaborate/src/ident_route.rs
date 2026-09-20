//! The ONE decision of how a bare single-segment name in an expression binds —
//! shared by the lowering (`lower_expr`'s `Ident` arm) and by the size-cast
//! classifier / self-width walk (`expr_size_ctx.rs`), so the classifier can
//! never answer a sign or width for a binding the lowering does not take.
//!
//! It had to be one function, not a mirror: the first cut of ROADMAP §2 row 29
//! re-derived the walk in the classifier and got the ORDER wrong twice — an
//! inline function/task formal (`subst_lookup`) binds before any net or
//! constant, and the combined innermost-wins key binds an inner generate-scope
//! `localparam` before an outer module NET — so `8'(pa >>> 4)` inside an inline
//! function whose formal `pa` shadows a module parameter answered the
//! parameter's sign (`0f` for the oracles' `ff`), and `8'(P >> 4)` under
//! `generate begin:g localparam logic signed [7:0] P` took the outer 64-bit
//! net's width (`ff` for `0f`). Both routes are decided here now.

use super::*;

/// Where a bare `NAME` binds, in the lowering's order. `Other` is everything the
/// lowering decides FURTHER DOWN its arm (a `let`, a class field, a plain net via
/// `resolve_net`, an undeclared name) — the classifier answers that tail with
/// `lookup_net_scoped`, exactly as it did before this type existed.
pub(crate) enum BareIdentRoute {
    /// the iterator of an array-method `with` clause
    ArrayItem {
        width: u32,
        signed: bool,
    },
    /// an inline function/task INPUT formal or body local: the actual's ExprId
    Subst(u32),
    /// an inline task OUTPUT/INOUT formal: the caller's net
    OutSubst(u32),
    /// a `string` parameter (no bit width)
    Str(String),
    /// a parameter wider than the i64 constant domain
    Wide(ir::ConstVal),
    /// a `real` parameter (no bit width)
    Real(f64),
    /// a numeric parameter / localparam / genvar / enum label, with its declared
    /// meta — `const_param_expr_w(v, meta)` is how it materializes. `guessed`:
    /// that meta is not a fact (`param_type_guessed`) — the lowering still builds
    /// with it, a classifier must not extend by it.
    Param {
        v: i64,
        meta: Option<(u32, bool)>,
        guessed: bool,
    },
    Other,
}

impl Elaborator<'_> {
    /// Decide [`BareIdentRoute`] for `seg` read at `at`. Pure: no diagnostics,
    /// no IR. The order is the lowering's and is load-bearing at every step:
    ///
    /// 1. the `with`-clause iterator name;
    /// 2. inline substitution (`subst_lookup`, innermost wins) — no IR node,
    ///    exactly like `Paren` unwrapping;
    /// 3. an output/inout task formal (`out_subst_lookup`);
    /// 4. (3½) a package routine body's own package constant or variable
    ///    (`pkg_body_scope.rs`), for a name the routine does not declare;
    /// 5. the INNERMOST key over the COMBINED binding set (string / wide / real /
    ///    numeric params and symbols): a string, wide or real parameter is
    ///    answered only when that exact key is the parameter (an independent
    ///    `walk_scopes` over one side map would match an OUTER binding even when
    ///    an inner net / numeric param / frame-local shadows it — IEEE §6.21
    ///    innermost-wins). When the innermost key is a hoisted block-local NET
    ///    that covers the reader (or was declared at it), the net wins over a
    ///    constant on the same key (the v1 flatten publishes a module-level
    ///    block-local under the BARE name, landing on the key the constant
    ///    already occupies; a reader inside the declaring block must see the
    ///    net, one outside must still see the constant);
    /// 6. a numeric constant in this or an enclosing generate scope
    ///    (`lookup_scoped`) — resolved before `resolve_net` so a param never
    ///    errors as an undeclared net.
    pub(crate) fn bare_ident_route(&self, seg: &str, at: ast::Span) -> BareIdentRoute {
        if self.array_iter.as_deref() == Some(seg) {
            let (width, signed) = self.array_iter_elem.unwrap_or((32, true));
            return BareIdentRoute::ArrayItem { width, signed };
        }
        if let Some(eid) = self.subst_lookup(seg) {
            return BareIdentRoute::Subst(eid);
        }
        if let Some(net) = self.out_subst_lookup(seg) {
            return BareIdentRoute::OutSubst(net);
        }
        // 3½. §26.3: inside a PACKAGE ROUTINE's body, a name the routine does not
        //    declare itself binds to the package's constant or variable before the
        //    caller module's scope walk below (`pkg_body_scope.rs`). A frame formal
        //    or local is excluded by the routine's declared set (it is a net under
        //    the body's own `$func$` scope and must keep winning); an inline formal
        //    or local returned at step 2. `Other` for a variable sends the lowering
        //    to `resolve_net`, whose first step is the same hook.
        if let Some(r) = self.pkg_body_const_route(seg) {
            return r;
        }
        if self.pkg_body_var(seg).is_some() {
            return BareIdentRoute::Other;
        }
        let mut local_shadows_param = false;
        if let Some(key) = self.walk_scopes_key(seg, |k| {
            self.str_param_raw.contains_key(k)
                || self.wide_param_bits.contains_key(k)
                || self.real_param_val.contains_key(k)
                || self.params.contains_key(k)
                || self.symbols.contains_key(k)
        }) {
            if let Some(raw) = self.str_param_raw.get(&key) {
                return BareIdentRoute::Str(raw.clone());
            }
            if let Some(cv) = self.wide_param_bits.get(&key) {
                return BareIdentRoute::Wide(cv.clone());
            }
            if let Some(&v) = self.real_param_val.get(&key) {
                return BareIdentRoute::Real(v);
            }
            if self.symbols.contains_key(&key)
                && (self.block_local_declared_at(&key, at)
                    || (!self.params.contains_key(&key) && self.block_local_covers(&key, at)))
            {
                local_shadows_param = true;
            }
            // else: an inner numeric param wins — fall through to `lookup_scoped`,
            // which resolves that innermost binding.
        }
        if !local_shadows_param {
            if let Some(v) = self.lookup_scoped(seg) {
                let meta = self.walk_scopes(seg, &self.param_meta);
                let guessed = self
                    .walk_scopes_key(seg, |k| self.params.contains_key(k))
                    .is_some_and(|k| self.param_type_guessed.contains(&k));
                return BareIdentRoute::Param { v, meta, guessed };
            }
        }
        BareIdentRoute::Other
    }

    /// Does the bare single-segment `name`, read at `at`, bind to a CONSTANT?
    ///
    /// The array / packed index CHAINS (`expr_array_chain`, `lval_array_chain`,
    /// `expr_packed_chain`, `lval_packed_chain`) resolve their base with
    /// `lookup_net_scoped`, which walks `symbols` ALONE — so a generate-scope
    /// `localparam` / `parameter` / genvar shadowing an outer ARRAY does not stop
    /// the walk, and `ROTA[1]` under `generate if (1) begin : g localparam int ROTA
    /// = 99;` read the OUTER array's element (`20`) where both oracles read bit 1
    /// of the inner constant (`1`) — while the WHOLE-name read in the same
    /// `$display` was right, because the lowering routes THAT through
    /// [`Self::bare_ident_route`]. The write twins were worse: they stored into the
    /// outer array at exit 0 where both oracles reject the program.
    ///
    /// So the chains ask the same question the lowering asks, with the same
    /// function: a constant binding declines the chain, and the base then lowers
    /// as the constant it is.
    ///
    /// ⚠️ `Subst` / `OutSubst` / `ArrayItem` deliberately do NOT decline — those are
    /// the inline-formal lanes the chains legitimately serve (an array passed to an
    /// inlined function is read through the chain by design).
    ///
    /// ⚠️ Do not hand-roll this as `params.contains_key(key)`: the v1 flatten
    /// publishes a hoisted block-local NET under the module's bare name, so a
    /// module `localparam` and that net collide on ONE key, and only
    /// `bare_ident_route`'s `block_local_declared_at` / `block_local_covers`
    /// tie-break gets the reader-position rule right.
    pub(crate) fn bare_name_binds_constant(&self, name: &str, at: ast::Span) -> bool {
        matches!(
            self.bare_ident_route(name, at),
            BareIdentRoute::Param { .. }
                | BareIdentRoute::Str(_)
                | BareIdentRoute::Wide(_)
                | BareIdentRoute::Real(_)
        )
    }

    /// [`Self::lookup_net_scoped`] for a reader that must honour IEEE §6.21
    /// innermost-wins: `None` when the bare single-segment `name`, read at `at`,
    /// binds to a CONSTANT, whatever net that walk would otherwise find.
    ///
    /// ROADMAP §2 🆕 O. `lookup_net_scoped` walks `symbols` ALONE, so a
    /// generate-scope `localparam` / `parameter` / genvar / enum label shadowing an
    /// outer NET does not stop it and every reader built on it answered for the
    /// outer object while the whole-name read (which routes through
    /// [`Self::bare_ident_route`]) answered for the constant — one name, two objects,
    /// in one design. This is the ONE funnel those readers share; a site that has a
    /// constant path takes it below the decline, and a site that has none refuses
    /// through [`Self::error_const_shadows_net`].
    ///
    /// ⚠️ Not for the net RESOLVER itself (`resolve_net`, `lookup_net_scoped`): those
    /// also serve positions where a constant is a legal operand, where over-reporting
    /// would be a loud regression rather than a refusal.
    pub(crate) fn lookup_net_unshadowed(&self, name: &str, at: ast::Span) -> Option<u32> {
        if self.bare_name_binds_constant(name, at) {
            return None;
        }
        self.lookup_net_scoped(name)
    }

    /// Does the bare single-segment name at the ROOT of `lv` bind to a constant —
    /// i.e. would lowering `lv` as a write target raise the
    /// [`Self::error_const_shadows_net`] refusal? A `Concat` answers for any part.
    ///
    /// Used where an lvalue is built SPECULATIVELY, before the site knows whether the
    /// value is written at all (`inline_task`'s deferred hierarchical enable lowers
    /// every actual as a candidate copy-OUT target). `_`-free so a new `Lvalue`
    /// variant is a compile error rather than a silent `false`.
    pub(crate) fn lvalue_binds_constant(&self, lv: &ast::Lvalue) -> bool {
        match lv {
            ast::Lvalue::Ident(p) => match p.segments.as_slice() {
                [seg] => self.bare_name_binds_constant(&seg.name, p.span),
                _ => false,
            },
            ast::Lvalue::BitSelect { base, .. }
            | ast::Lvalue::PartSelect { base, .. }
            | ast::Lvalue::IndexedPart { base, .. } => self.lvalue_binds_constant(base),
            ast::Lvalue::Concat { parts, .. } => {
                parts.iter().any(|p| self.lvalue_binds_constant(p))
            }
            ast::Lvalue::Error(_) => false,
        }
    }

    /// Is `name`, read at `at`, the §2 🆕 O SHADOW shape — a bare name that binds a
    /// constant WHILE a net of the same name is in scope? The extra term is what
    /// makes [`Self::error_const_shadows_net`]'s sentence true: a constant with no
    /// same-named net is an ordinary constant, and a site that cannot use one there
    /// keeps whatever diagnostic it already had.
    pub(crate) fn bare_const_shadows_net(&self, name: &str, at: ast::Span) -> bool {
        self.bare_name_binds_constant(name, at) && self.lookup_net_scoped(name).is_some()
    }

    /// The ONE §2 🆕 O refusal. `why` completes the sentence for the site: every
    /// caller is a position that has no constant path, so the reference cannot be
    /// lowered as either object.
    pub(crate) fn error_const_shadows_net(&mut self, name: &str, why: &str) {
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "`{name}` here resolves to a constant (parameter / localparam / \
                 genvar / enum label), which shadows the net of the same name \
                 — {why} (rename one of them to disambiguate)"
            ),
        );
    }

    /// Does `e` read (by bare name, anywhere in its tree) a constant whose type is
    /// a guess? A body parameter / localparam initialized from one inherits the
    /// guess: its value was folded from the guessed binding and its own meta is
    /// inferred from that value. Kinds this walk does not descend are literals
    /// and names it cannot bind — never a guess by themselves.
    pub(crate) fn ast_reads_guessed_param(&self, e: &ast::Expr) -> bool {
        if self.param_type_guessed.is_empty() {
            return false;
        }
        if let ast::ExprKind::Ident(p) = &e.kind {
            if let [seg] = p.segments.as_slice() {
                return self
                    .walk_scopes_key(&seg.name, |k| self.params.contains_key(k))
                    .is_some_and(|k| self.param_type_guessed.contains(&k));
            }
        }
        Self::const_fold_children(e)
            .iter()
            .any(|c| self.ast_reads_guessed_param(c))
    }
}
