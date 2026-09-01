//! unpacked arrays — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;
use crate::array_geom::IndexDomain;

impl Elaborator<'_> {
    pub(crate) fn expr_array_chain<'a>(
        &self,
        base: &'a ast::Expr,
        index: &'a ast::Expr,
    ) -> Option<(u32, Vec<&'a ast::Expr>)> {
        let mut outer_first: Vec<&ast::Expr> = Vec::new();
        let mut cur = base;
        let net = loop {
            match &cur.kind {
                ast::ExprKind::BitSelect { base: b, index: i } => {
                    outer_first.push(i);
                    cur = b;
                }
                // A 1-segment local net OR a multi-segment RESOLVABLE hierarchical
                // net (a same-module generate scope `g[0].mem`, already elaborated —
                // HIER-REST②). A cross-instance ref whose net is not yet created folds
                // to None here and takes the deferred-sel lane (`hier_sel_chain`).
                ast::ExprKind::Ident(p) => {
                    let joined = p
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    // A2b-prereq S1/S2/F2: mirror of `lval_array_chain` — a
                    // shadowed (bare) or dotted (§26.3) alias hit falls out of
                    // the fast path so the scalar/deferred lanes stay loud.
                    if p.segments.len() == 1 {
                        if self.bare_hit_is_shadowed_pkg_alias(&joined) {
                            return None;
                        }
                    } else if self.dotted_hit_is_pkg_alias(&joined) {
                        return None;
                    }
                    match self.lookup_net_scoped(&joined) {
                        // Declared array-ness, NOT `array_len > 1`: a `[0:0]`
                        // array's element access is still an ELEMENT access
                        // (adversarial find #5 — it used to bit-select word 0).
                        Some(n) if self.net_is_static_array(n) => break n,
                        _ => return None,
                    }
                }
                // Explicit `pkg::arr[i]` — resolve the package variable net and
                // apply the same declared-array-ness rule as the `Ident` arm.
                ast::ExprKind::PkgScoped { .. } => match self.pkg_scoped_var_net(cur) {
                    Some(n) if self.net_is_static_array(n) => break n,
                    _ => return None,
                },
                _ => return None,
            }
        };
        outer_first.reverse(); // base-chain → source order
        outer_first.push(index); // outermost index is the last in source order
        Some((net, outer_first))
    }

    /// Write-side twin of [`Self::expr_array_chain`] over `Lvalue` nodes.
    pub(crate) fn lval_array_chain<'a>(
        &self,
        base: &'a ast::Lvalue,
        index: &'a ast::Expr,
    ) -> Option<(u32, Vec<&'a ast::Expr>)> {
        let mut outer_first: Vec<&ast::Expr> = Vec::new();
        let mut cur = base;
        let net = loop {
            match cur {
                ast::Lvalue::BitSelect {
                    base: b, index: i, ..
                } => {
                    outer_first.push(i);
                    cur = b;
                }
                // 1-segment local OR multi-segment resolvable hierarchical array
                // (same-module generate scope — HIER-REST②). Cross-instance unresolved
                // → None → deferred-sel write lane.
                ast::Lvalue::Ident(p) => {
                    let joined = p
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    // A2b-prereq S1/S2/F2: a const/genvar-shadowed import alias
                    // (bare) or ANY dotted hit on an alias (§26.3) must not take
                    // the array-chain fast path — the scalar/deferred lanes'
                    // guards keep the reference loud.
                    if p.segments.len() == 1 {
                        if self.bare_hit_is_shadowed_pkg_alias(&joined) {
                            return None;
                        }
                    } else if self.dotted_hit_is_pkg_alias(&joined) {
                        return None;
                    }
                    match self.lookup_net_scoped(&joined) {
                        // Same declared-array-ness rule as expr_array_chain.
                        Some(n) if self.net_is_static_array(n) => break n,
                        _ => return None,
                    }
                }
                _ => return None,
            }
        };
        outer_first.reverse();
        outer_first.push(index);
        Some((net, outer_first))
    }

    /// Lower a read `net[idxs…]`: first D indices → flat word (the array element).
    /// Fewer than D indices is a partial unpacked slice (loud). The trailing
    /// index(es) select INSIDE the element:
    ///   - array-of-PACKED element (`reg [3:0][7:0] qm[0:1]`, in `packed_dims`):
    ///     the trailing indices walk the element's packed dims — `qm[i][j]` picks the
    ///     j-th packed sub-vector (a byte here), NOT a single bit (N3.2 fix). Offset
    ///     and width mirror [`Self::lower_packed_read`]; `> packed_dims.len()` trailing
    ///     indices is a bit-of-bit select (loud).
    ///   - plain element (flat vector): ONE trailing index → a single bit-select; more
    ///     than one is a bit-of-bit select (loud). Byte-identical to the pre-N3.2 path.
    ///
    /// The trailing handling is SYMMETRIC with the write path (`collect_array_write`).
    pub(crate) fn lower_array_read(&mut self, net: u32, idxs: &[&ast::Expr]) -> u32 {
        let dims = self.net_dim_extents(net);
        let d = dims.len();
        if idxs.len() < d {
            self.error(
                MsgCode::ElabUnsupported,
                "partial unpacked-array slice (v1: index every dimension)",
            );
            return self.placeholder_expr();
        }
        let word = self.flatten_word(&dims, &idxs[..d], &[], IndexDomain::ArrayWord);
        let val = self.push_expr(ir::Expr::Signal {
            net,
            word: Some(word),
        });
        let trailing = &idxs[d..];
        if trailing.is_empty() {
            return val;
        }
        // Array-of-packed: trailing indices select within the element's packed bit
        // space (`flatten_word(packed_dims, …)` offset, product-of-remaining width).
        if let Some(pdims) = self.packed_dims.get(&net).cloned() {
            if trailing.len() > pdims.len() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "bit-select then bit-select on a multi-dim array element (v1: single bit/part)",
                );
                return self.placeholder_expr();
            }
            let (ext, dirs) = Self::packed_split(&pdims);
            let offset = self.flatten_word(&ext, trailing, &dirs, IndexDomain::PackedElem);
            let elem_w: u64 = pdims[trailing.len()..]
                .iter()
                .map(|&(_, w, _)| w as u64)
                .product();
            let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
            return self.push_expr(ir::Expr::Select {
                base: val,
                offset,
                width,
                kind: ir::SelKind::PartIdxUp,
            });
        }
        // Plain (flat-vector) element: a single trailing bit-select.
        if trailing.len() == 1 {
            // P0-NZE: a trailing bit-select on an element with a non-zero / ascending
            // packed range normalizes against the element's LSB (parity with the plain
            // bit-select path, which uses `norm_offset_for_net`); a `[N:0]` element is a
            // no-op → byte-identical golden for every zero-based design.
            let raw = self.lower_index_expr(trailing[0]);
            let offset = self.norm_offset_for_net(net, raw);
            let width = self.const_u32_expr(1, 32);
            return self.push_expr(ir::Expr::Select {
                base: val,
                offset,
                width,
                kind: ir::SelKind::Bit,
            });
        }
        self.error(
            MsgCode::ElabUnsupported,
            "bit-select then bit-select on a multi-dim array element (v1: single bit/part)",
        );
        self.placeholder_expr()
    }

    /// Lower a write `net[idxs…] = …` into one `LvalChunk`: first D indices → flat
    /// word (the array element). Trailing indices select inside the element, the
    /// write-side twin of [`Self::lower_array_read`]:
    ///   - array-of-PACKED element: trailing indices walk the element's packed dims
    ///     (`qm[i][j] = …` writes the j-th packed sub-vector, a byte here — N3.2 fix),
    ///     mirroring [`Self::collect_packed_write`]; `> packed_dims.len()` trailing
    ///     indices is a bit-of-bit LHS (loud). Engine `write_chunk` lands a
    ///     `{word:Some, PartIdxUp}` chunk at `base + offset` for `width` bits.
    ///   - plain element: ONE trailing index → single bit-select (byte-identical to
    ///     pre-N3.2); `< D` (partial slice) and `> D+1` (bit-of-bit) are loud.
    pub(crate) fn collect_array_write(
        &mut self,
        net: u32,
        idxs: &[&ast::Expr],
        out: &mut Vec<ir::LvalChunk>,
    ) {
        let dims = self.net_dim_extents(net);
        let d = dims.len();
        if idxs.len() < d {
            self.error(
                MsgCode::ElabUnsupported,
                "partial unpacked-array slice (v1: index every dimension)",
            );
            out.push(ir::LvalChunk {
                net: POISON_NET,
                word: None,
                offset: None,
                width: None,
                kind: ir::SelKind::Bit,
            });
            return;
        }
        let word = self.flatten_word(&dims, &idxs[..d], &[], IndexDomain::ArrayWord);
        let trailing = &idxs[d..];
        // Array-of-packed: trailing indices → an indexed part-select on the element.
        if !trailing.is_empty() {
            if let Some(pdims) = self.packed_dims.get(&net).cloned() {
                if trailing.len() > pdims.len() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "nested bit-select on a multi-dim array lvalue (v1: single bit/part)",
                    );
                    out.push(ir::LvalChunk {
                        net: POISON_NET,
                        word: None,
                        offset: None,
                        width: None,
                        kind: ir::SelKind::Bit,
                    });
                    return;
                }
                let (ext, dirs) = Self::packed_split(&pdims);
                let offset = self.flatten_word(&ext, trailing, &dirs, IndexDomain::PackedElem);
                let elem_w: u64 = pdims[trailing.len()..]
                    .iter()
                    .map(|&(_, w, _)| w as u64)
                    .product();
                let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
                out.push(ir::LvalChunk {
                    net,
                    word: Some(word),
                    offset: Some(offset),
                    width: Some(width),
                    kind: ir::SelKind::PartIdxUp,
                });
                return;
            }
        }
        // Plain (flat-vector) element: whole word (0 trailing) or single bit (1).
        let (offset, width) = match trailing.len() {
            0 => (None, None),
            1 => {
                // P0-NZE: normalize the trailing bit against the element's LSB (a
                // `[N:0]` element is a no-op → byte-identical golden).
                let raw = self.lower_index_expr(trailing[0]);
                let off = self.norm_offset_for_net(net, raw);
                let w = self.const_u32_expr(1, 32);
                (Some(off), Some(w))
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "nested bit-select on a multi-dim array lvalue (v1: single bit/part)",
                );
                (None, None)
            }
        };
        out.push(ir::LvalChunk {
            net,
            word: Some(word),
            offset,
            width,
            kind: ir::SelKind::Bit,
        });
    }

    // ── unpacked-array ASSIGNMENT (whole array / partial slice) ─────
    //
    // IEEE 1800 §7.6: source and target need the same number of unpacked
    // dims, the same SIZE per dim, and identical element types; elements
    // correspond POSITIONALLY in declared left-to-right index order (the
    // LRM example pairs `A[10:1] = B[0:9]` as A[10]=B[0] … A[1]=B[9]).
    //
    // ⚠️ iverilog 13.0 rejects fixed-size unpacked array assignment outright,
    // so this lane is hand-pinned to the LRM (same precedent as assoc /
    // interface ports). The expansion is element-wise: one assignment per
    // element, leading (user) indices lowered ONCE and shared as a base
    // word expression, each element adding its constant residual offset.
    // Element-wise order is observationally equivalent to the LRM's
    // evaluate-then-assign because the supported slice forms make source
    // and target rows either identical or disjoint, and slice indices are
    // REJECTED if they read the target array (the one case where a write
    // mid-copy could move the index).

    /// `Some((net, leading))` when `lv` is a static unpacked-array lvalue
    /// indexed by FEWER indices than its dimension count (whole = zero).
    pub(crate) fn lval_array_view<'a>(
        &self,
        lv: &'a ast::Lvalue,
    ) -> Option<(u32, Vec<&'a ast::Expr>)> {
        match lv {
            ast::Lvalue::Ident(p) => {
                let name = match p.segments.as_slice() {
                    [seg] => {
                        if self.out_subst_lookup(&seg.name).is_some() {
                            return None; // task out-formal: vector surface
                        }
                        // A2b-prereq S1: a const-shadowed import alias must not
                        // take the array-view fast path — fall through (None) to
                        // the scalar funnel, whose `resolve_net` guard is loud.
                        if self.bare_hit_is_shadowed_pkg_alias(&seg.name) {
                            return None;
                        }
                        seg.name.clone()
                    }
                    segs => {
                        let joined = segs
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        // A2b-prereq F2: dotted paths never resolve through an
                        // import alias (§26.3) — the scalar funnel stays loud.
                        if self.dotted_hit_is_pkg_alias(&joined) {
                            return None;
                        }
                        joined
                    }
                };
                let net = self.lookup_net_scoped(&name)?;
                self.net_is_static_array(net).then(|| (net, Vec::new()))
            }
            ast::Lvalue::BitSelect { base, index, .. } => {
                let (net, idxs) = self.lval_array_chain(base, index)?;
                (idxs.len() < self.net_dim_extents(net).len()).then_some((net, idxs))
            }
            _ => None,
        }
    }

    /// The net a WHOLE-NAME expression denotes, with the same resolution
    /// priority [`Self::expr_array_view`] uses — inline-subst formals, params,
    /// genvars and shadowed package aliases all decline, because a name they own
    /// is not a net at all.
    ///
    /// Extracted so that "which net does this bare/dotted name mean" has ONE
    /// spelling: `expr_array_view` filters its answer to static arrays, the `%p`
    /// argument gate ([`Self::lower_pattern_arg`]) filters the same answer to
    /// dynamic-storage handles, and a second copy of these four shadow rules is
    /// exactly the drift that would let one of them read a package's storage.
    pub(crate) fn whole_name_net(&self, e: &ast::Expr) -> Option<u32> {
        let ast::ExprKind::Ident(p) = &e.kind else {
            return None;
        };
        let name = match p.segments.as_slice() {
            [seg] => {
                // Inline-subst formals / params shadow nets (mirrors
                // the lower_expr Ident arm's resolution priority).
                // A2b-prereq S1/S2: a const/genvar-shadowed import
                // alias also falls through — the scalar funnel's
                // guard keeps the reference loud, never a silent
                // read of the package storage.
                if self.subst_lookup(&seg.name).is_some()
                    || self.out_subst_lookup(&seg.name).is_some()
                    || self.lookup_scoped(&seg.name).is_some()
                    || self.bare_hit_is_shadowed_pkg_alias(&seg.name)
                {
                    return None;
                }
                seg.name.clone()
            }
            segs => {
                let joined = segs
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                // A2b-prereq F2: dotted paths never resolve through an
                // import alias (IEEE §26.3).
                if self.dotted_hit_is_pkg_alias(&joined) {
                    return None;
                }
                joined
            }
        };
        self.lookup_net_scoped(&name)
    }

    /// The `%p` (IEEE 1800 §21.2.1.7) ARGUMENT surface.
    ///
    /// `%p` is defined for exactly the aggregates whose whole-value reads are
    /// E3009 — an unpacked array ("a whole unpacked array has no value in this
    /// context") and a dynamic-storage handle ("a dynamic-storage handle has no
    /// whole-value surface"). Both messages answer a question about a VALUE, and
    /// they are right about it; `%p` asks a different question, so in this ONE
    /// argument position the aggregate is the operand rather than a value and the
    /// engine's `builtins::pattern` renders it.
    ///
    /// Shaped after the `$readmem*` memory argument (§4.5.376): the eid pushed
    /// here is `Signal { net, word: None }`, the same node `lower_expr` would
    /// have produced had the guard not fired, so the whole behavioural delta is
    /// "which eids skip the read guard".
    ///
    /// `None` ⇒ the caller lowers normally, which keeps every non-aggregate `%p`
    /// argument (an int, a packed struct, a real, a string, a select) byte-identical.
    ///
    /// ⚠️ A ONE-ELEMENT unpacked array (`int a[0:0]`, `array_len == 1`) declines
    /// DELIBERATELY, and the caller then reports it: elaborate knows it is an
    /// array (`unpacked_array_nets`), but `sim_ir::NetVar` records only
    /// `array_len`, which is `1` for a scalar too, and that table never reaches
    /// the engine. Admitting it would render `'{'h2a}` as the bare scalar `42` at
    /// exit 0 — the exact silent-wrong this feature exists to remove — so it stays
    /// LOUD until something carries array-ness into the IR.
    pub(crate) fn lower_pattern_arg(&mut self, e: &ast::Expr) -> Option<u32> {
        if let ast::ExprKind::Paren { inner } = &e.kind {
            return self.lower_pattern_arg(inner);
        }
        // A whole fixed-size unpacked array. A PARTIAL index (`a[i]` on a 2-D
        // array) also has an assignment-pattern form, but its flat window is not
        // a net — it is the `lead`-selected sub-array — so it stays loud.
        if let Some((net, lead)) = self.expr_array_view(e) {
            if lead.is_empty() && self.nets[net as usize].array_len > 1 {
                return Some(self.push_expr(ir::Expr::Signal { net, word: None }));
            }
            return None;
        }
        // A whole dynamic-storage handle (dyn array / queue / assoc / assoc-str).
        // A `string` is NOT here on purpose: it already has a whole-value surface,
        // and the renderer's string arm is the one that quotes it.
        if let Some(net) = self.whole_name_net(e) {
            if self.is_dyn_handle_net(net) {
                return Some(self.push_expr(ir::Expr::Signal { net, word: None }));
            }
            return None;
        }
        // A CROSS-INSTANCE name (`dut.mem`, `dut.q`). Its net does not exist yet —
        // the child's nets are created in pass 8, after this pass-7 lowering — so
        // `whole_name_net` declines and `lower_expr` emits the deferred placeholder,
        // which is ALREADY `Signal { net: POISON_NET, word: None }`: the exact node
        // the two arms above build by hand. Nothing needs building; only the read
        // guard in `resolve_deferred_hier` has to know that in THIS position an
        // aggregate is the operand (§4.5.376's shape, verbatim).
        if matches!(&e.kind, ast::ExprKind::Ident(p) if p.segments.len() > 1) {
            let eid = self.lower_expr(e);
            self.hier_pattern_args.insert(eid);
            return Some(eid);
        }
        None
    }

    /// Lower ONE value argument of a `$display`-family call, given whether its
    /// conversion is `%p`.
    ///
    /// The single entry point both print lowerings use (the general system-task
    /// path and the severity family), so "what does `%p` accept" has one answer
    /// and one diagnostic. `is_pattern == false` is literally `lower_expr`, which
    /// is what keeps every other argument byte-identical.
    pub(crate) fn lower_fmt_value_arg(&mut self, a: &ast::Expr, is_pattern: bool) -> u32 {
        if !is_pattern {
            return self.lower_expr(a);
        }
        if let Some(eid) = self.lower_pattern_arg(a) {
            return eid;
        }
        if self.pattern_arg_is_unrenderable_array(a) {
            self.error(
                MsgCode::ElabUnsupported,
                "`%p` of a ONE-ELEMENT unpacked array is unsupported: `sim_ir::NetVar` \
                 records only `array_len`, which is 1 for a scalar too, so the renderer \
                 cannot tell the two apart and would print the element without its \
                 assignment-pattern braces (index the element instead)",
            );
            return self.placeholder_expr();
        }
        self.lower_expr(a)
    }

    /// `$sformatf(fmt, args…)` argument lowering, with the `%p` aggregate surface.
    ///
    /// THREE sites build a `SysFunc::Sformatf` out of a user call — the
    /// blocking-assign special, the `sformatf_expr_ok` expression arm and a string
    /// `return` — and all three carried the identical
    /// `args.iter().map(lower_expr)` line. They share this one instead, because
    /// `$sformatf("%p", q)` being loud while `$display("%p", q)` renders would be a
    /// difference in the SPELLING of the call rather than in the question asked.
    /// `args[0]` is the format literal itself, so value-argument `k` is `args[k+1]`.
    pub(crate) fn lower_sformatf_args(&mut self, args: &[ast::Expr]) -> Vec<u32> {
        let conv: Vec<char> = match args.first().map(|a| &a.kind) {
            Some(ast::ExprKind::StrLit { raw }) => arg_conv_specs(&parse_str_literal_text(raw)),
            _ => Vec::new(),
        };
        args.iter()
            .enumerate()
            .map(|(i, a)| {
                let is_pattern =
                    matches!(i.checked_sub(1).and_then(|k| conv.get(k)), Some('p' | 'P'));
                self.lower_fmt_value_arg(a, is_pattern)
            })
            .collect()
    }

    /// True when `e` names an aggregate that `%p` OUGHT to render but the engine
    /// cannot recognise — today exactly the one-element unpacked array described
    /// on [`Self::lower_pattern_arg`]. The caller turns this into an honest
    /// refusal rather than letting the scalar path answer.
    pub(crate) fn pattern_arg_is_unrenderable_array(&self, e: &ast::Expr) -> bool {
        if let ast::ExprKind::Paren { inner } = &e.kind {
            return self.pattern_arg_is_unrenderable_array(inner);
        }
        matches!(self.expr_array_view(e), Some((net, ref lead))
            if lead.is_empty() && self.nets[net as usize].array_len <= 1)
    }

    /// Read-side twin of [`Self::lval_array_view`] over expressions.
    pub(crate) fn expr_array_view<'a>(
        &self,
        e: &'a ast::Expr,
    ) -> Option<(u32, Vec<&'a ast::Expr>)> {
        match &e.kind {
            ast::ExprKind::Ident(_) => {
                let net = self.whole_name_net(e)?;
                self.net_is_static_array(net).then(|| (net, Vec::new()))
            }
            ast::ExprKind::BitSelect { base, index } => {
                let (net, idxs) = self.expr_array_chain(base, index)?;
                (idxs.len() < self.net_dim_extents(net).len()).then_some((net, idxs))
            }
            ast::ExprKind::Paren { inner } => self.expr_array_view(inner),
            _ => None,
        }
    }

    /// Intercept `lhs = rhs` / `lhs <= [#d] rhs` when the LHS is an unpacked
    /// array (whole or partial slice). Returns `true` when the statement was
    /// consumed (expanded element-wise, or rejected loudly).
    pub(crate) fn array_assign_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
        nonblocking: bool,
    ) -> bool {
        const ARRAY_COPY_UNROLL_CAP: u64 = 4096;
        let Some((t_net, t_lead)) = self.lval_array_view(lhs) else {
            return false;
        };
        // The expansion builds chunks by hand, bypassing collect_lval_chunks
        // — re-run its modport write rule here (adversarial find #1: a
        // modport-`input` array was silently writable as `p.arr = l`).
        if let Some(path) = lval_root_path(lhs) {
            let path = path.clone();
            self.check_modport_write(&path);
        }
        // A2a: same bypass — a whole-array write (`R = '{…}` / `R = other`)
        // targeting a desugared array parameter must stay loud.
        self.deny_readonly_write(t_net, "assign to");
        // SV §10.9: a positional assignment pattern RHS (`a = '{e0,…}`) assigns each
        // element to the corresponding 1-D unpacked array slot (declaration order).
        if let ast::ExprKind::AssignPattern(elems) = &rhs.kind {
            return self.lower_array_assign_pattern(b, t_net, &t_lead, elems, delay, nonblocking);
        }
        // §10.9.1 `a = '{default: v}` — `default` fills every element not otherwise
        // given. vita resolves the ALL-default form only: with no other key there is
        // nothing to order, so the target's own dimensions are the whole answer and
        // `expand_array_default_pattern` hands the positional path one clone of `v`
        // per residual slot. Integer keys (`'{0: a, default: b}`) stay loud — see
        // that function.
        if let ast::ExprKind::AssignPatternKeyed(keyed) = &rhs.kind {
            let t_dims = self.net_dim_extents(t_net);
            let t_res: Vec<(i64, u32)> = t_dims[t_lead.len()..].to_vec();
            let Some(elems) = self.expand_array_default_pattern(keyed, &t_res) else {
                return true; // error already emitted
            };
            return self.lower_array_assign_pattern(b, t_net, &t_lead, &elems, delay, nonblocking);
        }
        let Some((s_net, s_lead)) = self.expr_array_view(rhs) else {
            self.error(
                MsgCode::ElabUnsupported,
                "assigning a non-array value to an unpacked array (copy from an \
                 identically-shaped array, or index an element)",
            );
            return true;
        };
        let t_dims = self.net_dim_extents(t_net);
        let s_dims = self.net_dim_extents(s_net);
        let t_res = &t_dims[t_lead.len()..];
        let s_res = &s_dims[s_lead.len()..];
        if t_res.len() != s_res.len()
            || t_res.iter().zip(s_res).any(|(&(_, ts), &(_, ss))| ts != ss)
        {
            self.error(
                MsgCode::ElabUnsupported,
                "unpacked-array assignment requires the same number of dimensions \
                 and the same size per dimension (IEEE 1800 §7.6)",
            );
            return true;
        }
        let (tw, tk, tsg) = {
            let nv = &self.nets[t_net as usize];
            (nv.width, nv.kind, nv.signed)
        };
        let (sw, sk, ssg) = {
            let nv = &self.nets[s_net as usize];
            (nv.width, nv.kind, nv.signed)
        };
        // §6.22.2 equivalent element types: width, realness AND signedness
        // (a raw word copy would be bit-correct either way, but accepting a
        // signed/unsigned mix would silently diverge from conformant tools).
        if tw != sw || (tk == ir::NetKind::Real) != (sk == ir::NetKind::Real) || tsg != ssg {
            self.error(
                MsgCode::ElabUnsupported,
                "unpacked-array assignment requires identical element types \
                 (IEEE 1800 §7.6)",
            );
            return true;
        }
        if !nonblocking && delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on an unpacked-array assignment \
                 (v1: plain `=`, `<=`, or `<= #d`)",
            );
            return true;
        }
        if t_lead
            .iter()
            .chain(s_lead.iter())
            .any(|i| self.expr_reads_net(i, t_net))
        {
            self.error(
                MsgCode::ElabUnsupported,
                "an array-slice index in an array assignment reads the assignment \
                 target itself (v1: the element-wise copy could move the index)",
            );
            return true;
        }
        let n: u64 = t_res.iter().map(|&(_, s)| s as u64).product();
        if n > ARRAY_COPY_UNROLL_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "unpacked-array assignment copies {n} elements \
                     (v1 cap {ARRAY_COPY_UNROLL_CAP})"
                ),
            );
            return true;
        }
        let t_desc: Vec<bool> = self
            .array_dim_desc
            .get(&t_net)
            .map(|v| v[t_lead.len()..].to_vec())
            .unwrap_or_default();
        let s_desc: Vec<bool> = self
            .array_dim_desc
            .get(&s_net)
            .map(|v| v[s_lead.len()..].to_vec())
            .unwrap_or_default();
        let t_offs = Self::residual_word_offsets(t_res, &t_desc);
        let s_offs = Self::residual_word_offsets(s_res, &s_desc);
        // Leading (user) indices lower ONCE; every element shares the base
        // ExprId (pure reads — sharing is the function-inline precedent).
        let t_base = (!t_lead.is_empty())
            .then(|| self.flatten_word(&t_dims, &t_lead, &[], IndexDomain::ArrayWord));
        let s_base = (!s_lead.is_empty())
            .then(|| self.flatten_word(&s_dims, &s_lead, &[], IndexDomain::ArrayWord));
        let delay_id = if nonblocking {
            delay.map(|d| self.lower_delay(d).0)
        } else {
            None
        };
        let mut kind_checked = false;
        for (&t_off, &s_off) in t_offs.iter().zip(&s_offs) {
            let t_word = self.word_expr_at(t_base, t_off);
            let s_word = self.word_expr_at(s_base, s_off);
            let rhs_id = self.push_expr(ir::Expr::Signal {
                net: s_net,
                word: Some(s_word),
            });
            let lv = ir::Lvalue {
                chunks: vec![ir::LvalChunk {
                    net: t_net,
                    word: Some(t_word),
                    offset: None,
                    width: None,
                    kind: ir::SelKind::Bit,
                }],
            };
            if !kind_checked {
                self.check_lvalue_kind(&lv, true); // E3018 once (same net throughout)
                kind_checked = true;
            }
            let sid = if nonblocking {
                self.push_stmt(ir::Stmt::NonblockingAssign {
                    lhs: lv,
                    rhs: rhs_id,
                    delay: delay_id,
                })
            } else {
                self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: lv,
                    rhs: rhs_id,
                })
            };
            b.push_stmt_id(sid);
        }
        true
    }

    /// SV §10.9 positional assignment pattern bound to a 1-D unpacked array
    /// (`a = '{e0,…,eN};` and, via the synthesized var-init `initial`, the decl
    /// form `int a[N] = '{…};`). Element k is assigned to the k-th array slot in
    /// DECLARATION order (which is exactly the order `residual_word_offsets`
    /// enumerates, so it is correct for ascending, descending and offset bounds).
    /// A multi-dimensional target, an element-count mismatch, or an intra-assign
    /// delay is loud (correct-or-loud). Returns `true` (the assignment is consumed).
    /// Flatten a (possibly nested) assignment pattern `'{…}` into its leaf elements
    /// in ROW-MAJOR (declaration) order, validating the shape against the residual
    /// unpacked dimensions `dims`. A 1-D residual is the base case (each element is a
    /// leaf); a multi-dimensional residual requires a nested `'{…}` per element whose
    /// shape matches the next dimension (IEEE 1800 §10.9.1). Returns `None` (with a
    /// loud error already emitted) on a count mismatch or a missing nested pattern.
    pub(crate) fn flatten_assign_pattern<'a>(
        &mut self,
        elems: &'a [ast::Expr],
        dims: &[(i64, u32)],
    ) -> Option<Vec<&'a ast::Expr>> {
        let size = dims[0].1 as usize;
        if elems.len() != size {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "assignment pattern has {} element(s) but the array dimension has {} \
                     (IEEE 1800 §10.9.1 requires an exact match)",
                    elems.len(),
                    size
                ),
            );
            return None;
        }
        if dims.len() == 1 {
            return Some(elems.iter().collect());
        }
        let mut out: Vec<&'a ast::Expr> = Vec::new();
        for e in elems {
            let ast::ExprKind::AssignPattern(sub) = &e.kind else {
                self.error(
                    MsgCode::ElabUnsupported,
                    "each element of a multi-dimensional array pattern must itself be a \
                     nested assignment pattern `'{…}`",
                );
                return None;
            };
            out.extend(self.flatten_assign_pattern(sub, &dims[1..])?);
        }
        Some(out)
    }

    /// §10.9.1 `'{default: v}` bound to a fixed-size unpacked array: expand it into
    /// the `n`-element POSITIONAL list `lower_array_assign_pattern` already lowers,
    /// so the two spellings share one lowering and one element-width rule rather
    /// than growing a second copy that could drift.
    ///
    /// ONLY the sole-`default` form is expanded. A member name is meaningless on an
    /// array, and an INTEGER key (`'{0: a, default: b}`) is left loud on purpose:
    /// iverilog 13 rejects every keyed pattern outright (measured — see the parser's
    /// `parse_assign_pattern`), so verilator would be the only tool available to
    /// settle index-vs-`default` priority and non-zero-based bounds, and one tool is
    /// not an oracle. `n` is the residual element COUNT, so a multi-dimensional
    /// target is filled correctly too (every leaf gets `v`, regardless of shape).
    ///
    /// `v` is CLONED into every slot, so a side-effecting `v` would run once per
    /// element instead of once; §10.9.1 does not pin that count and iverilog cannot
    /// be asked, so `assign_pattern_expr_has_call` keeps a call-bearing default loud.
    pub(crate) fn expand_array_default_pattern(
        &mut self,
        keyed: &[(ast::AssignPatternKey, ast::Expr)],
        dims: &[(i64, u32)],
    ) -> Option<Vec<ast::Expr>> {
        let [(ast::AssignPatternKey::Default, v)] = keyed else {
            self.error(
                MsgCode::ElabUnsupported,
                "an unpacked-array assignment pattern that is positional `'{e0,…}` or \
                 exactly `'{default: v}` (IEEE 1800 §10.9.1); a member or index key on \
                 an array target is not supported",
            );
            return None;
        };
        if Self::assign_pattern_expr_has_call(v) {
            self.error(
                MsgCode::ElabUnsupported,
                "a call-free `'{default: v}` value (it is evaluated once per element, \
                 so a call there would run once per element instead of once)",
            );
            return None;
        }
        if dims.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "`'{default: v}` on an unpacked-array target (this target has no \
                 unpacked dimension left to fill)",
            );
            return None;
        }
        // Bound the expansion before building it — `lower_array_assign_pattern`'s own
        // unroll cap fires only AFTER flattening, which would mean materialising the
        // clones first. Same number, checked earlier.
        let total: u64 = dims
            .iter()
            .fold(1u64, |a, &(_, w)| a.saturating_mul(w as u64));
        if total > 4096 {
            self.error(
                MsgCode::ElabUnsupported,
                "a `'{default: v}` pattern expanding past the v1 4096-element cap",
            );
            return None;
        }
        // A multi-dimensional target needs the same NESTED shape the positional path
        // validates (`flatten_assign_pattern` requires one `'{…}` per element of each
        // outer dimension), so build the nest rather than a flat list.
        fn nest(v: &ast::Expr, dims: &[(i64, u32)]) -> Vec<ast::Expr> {
            (0..dims[0].1)
                .map(|_| {
                    if dims.len() == 1 {
                        v.clone()
                    } else {
                        ast::Expr {
                            kind: ast::ExprKind::AssignPattern(nest(v, &dims[1..])),
                            span: v.span,
                        }
                    }
                })
                .collect()
        }
        Some(nest(v, dims))
    }

    /// Does `e` contain a call-like node? The array `'{default: v}` expansion clones
    /// `v` once per element, so a side-effecting `v` would run `n` times; §10.9.1
    /// does not pin that count and iverilog cannot be asked (it rejects the form),
    /// so a call-bearing default stays loud. Deliberately CONSERVATIVE — anything
    /// this walker does not recognise as a pure leaf/compound counts as a call.
    fn assign_pattern_expr_has_call(e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        match &e.kind {
            K::IntLit { .. }
            | K::RealLit { .. }
            | K::StrLit { .. }
            | K::Ident(_)
            | K::PkgScoped { .. }
            | K::Null
            | K::Dollar
            | K::Error => false,
            K::Unary { operand, .. } => Self::assign_pattern_expr_has_call(operand),
            K::Binary { lhs, rhs, .. } => {
                Self::assign_pattern_expr_has_call(lhs) || Self::assign_pattern_expr_has_call(rhs)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                Self::assign_pattern_expr_has_call(cond)
                    || Self::assign_pattern_expr_has_call(then_e)
                    || Self::assign_pattern_expr_has_call(else_e)
            }
            K::BitSelect { base, index } => {
                Self::assign_pattern_expr_has_call(base)
                    || Self::assign_pattern_expr_has_call(index)
            }
            K::PartSelect { base, msb, lsb } => {
                Self::assign_pattern_expr_has_call(base)
                    || Self::assign_pattern_expr_has_call(msb)
                    || Self::assign_pattern_expr_has_call(lsb)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                Self::assign_pattern_expr_has_call(base)
                    || Self::assign_pattern_expr_has_call(offset)
                    || Self::assign_pattern_expr_has_call(width)
            }
            K::Concat { parts } | K::AssignPattern(parts) => {
                parts.iter().any(Self::assign_pattern_expr_has_call)
            }
            K::Replicate { count, value } => {
                Self::assign_pattern_expr_has_call(count)
                    || value.iter().any(Self::assign_pattern_expr_has_call)
            }
            K::Paren { inner } => Self::assign_pattern_expr_has_call(inner),
            K::Cast { expr, .. } => Self::assign_pattern_expr_has_call(expr),
            K::TimeLit { num, .. } => Self::assign_pattern_expr_has_call(num),
            // Everything else (calls, method calls, `new`, randomize, dist, min:typ:max,
            // named args, nested keyed patterns) is treated as impure — fail-closed.
            _ => true,
        }
    }

    pub(crate) fn lower_array_assign_pattern(
        &mut self,
        b: &mut ProcessBuilder,
        t_net: u32,
        t_lead: &[&ast::Expr],
        elems: &[ast::Expr],
        delay: Option<&ast::Delay>,
        nonblocking: bool,
    ) -> bool {
        let t_dims = self.net_dim_extents(t_net);
        let t_res = &t_dims[t_lead.len()..];
        if !nonblocking && delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "intra-assignment delay on an assignment-pattern array assignment",
            );
            return true;
        }
        // Flatten the (possibly nested) pattern into row-major leaves matching the
        // residual unpacked dims. A 1-D residual is the base case; a multi-dim array
        // (`int a[2][3] = '{'{1,2,3},'{4,5,6}}`) requires a nested `'{…}` per element,
        // assigned in the same row-major flat order `residual_word_offsets` produces.
        let Some(leaves) = self.flatten_assign_pattern(elems, t_res) else {
            return true; // count mismatch / missing nested pattern — error emitted
        };
        // The pattern unrolls one assignment per leaf; bound it like the array-copy
        // path (a deeply-nested large pattern would otherwise emit unbounded IR).
        const ARRAY_PATTERN_UNROLL_CAP: usize = 4096;
        if leaves.len() > ARRAY_PATTERN_UNROLL_CAP {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "an assignment pattern expands to {} elements (v1 cap {})",
                    leaves.len(),
                    ARRAY_PATTERN_UNROLL_CAP
                ),
            );
            return true;
        }
        let t_desc: Vec<bool> = self
            .array_dim_desc
            .get(&t_net)
            .map(|v| v[t_lead.len()..].to_vec())
            .unwrap_or_default();
        let t_offs = Self::residual_word_offsets(t_res, &t_desc);
        let t_base = (!t_lead.is_empty())
            .then(|| self.flatten_word(&t_dims, t_lead, &[], IndexDomain::ArrayWord));
        let delay_id = if nonblocking {
            delay.map(|d| self.lower_delay(d).0)
        } else {
            None
        };
        let elem_width = self.nets[t_net as usize].width;
        let mut kind_checked = false;
        for (k, &t_off) in t_offs.iter().enumerate() {
            // §11.6: each element is in the array element's width context, so a fill
            // literal (`'1`/`'x`/`'z`) grows to the element width (a bare `lower_expr`
            // would size it to 1 bit, then the engine zero-extends it = silent-wrong).
            // `lower_ctx_or_plain` is byte-identical for a non-fill element.
            let rhs_id = self.lower_ctx_or_plain(leaves[k], elem_width);
            let t_word = self.word_expr_at(t_base, t_off);
            let lv = ir::Lvalue {
                chunks: vec![ir::LvalChunk {
                    net: t_net,
                    word: Some(t_word),
                    offset: None,
                    width: None,
                    kind: ir::SelKind::Bit,
                }],
            };
            if !kind_checked {
                self.check_lvalue_kind(&lv, true);
                kind_checked = true;
            }
            let sid = if nonblocking {
                self.push_stmt(ir::Stmt::NonblockingAssign {
                    lhs: lv,
                    rhs: rhs_id,
                    delay: delay_id,
                })
            } else {
                self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: lv,
                    rhs: rhs_id,
                })
            };
            b.push_stmt_id(sid);
        }
        true
    }
}
