//! packed selects (read) — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Internal-bit select direction for an indexed part-select `[base ± width]`.
/// On a DESCENDING net the source-index direction equals the internal-bit
/// direction (`+:` ⇒ up, `-:` ⇒ down). On an ASCENDING (`[lo:hi]`) net the source
/// index runs opposite to the internal bit (index 0 is the MSB), so `+:` moves
/// DOWN in internal bits and `-:` UP — the offset (`norm_offset_for_net`) already
/// maps the base index onto its internal bit. IEEE 1800 §11.5.1 + §7.4.3.
pub(crate) fn indexed_sel_kind(dir: &ast::PartDir, ascending: bool) -> ir::SelKind {
    match (dir, ascending) {
        (ast::PartDir::PlusColon, false) | (ast::PartDir::MinusColon, true) => {
            ir::SelKind::PartIdxUp
        }
        (ast::PartDir::MinusColon, false) | (ast::PartDir::PlusColon, true) => {
            ir::SelKind::PartIdxDown
        }
    }
}

impl Elaborator<'_> {
    /// Per-PACKED-dim `(lo, width, ascending)` extents of `[range][packed…]`
    /// (outer→inner). The product of the widths is the flat vector width; `lo` is the
    /// dim's lower bound (subtracted to 0-base a descending source index). `ascending`
    /// is true for a little-endian `[lo:hi]` dim (msb<lsb), where the index maps to
    /// `coord = hi - i` instead (N3.3). Empty for a scalar/plain vector.
    pub(crate) fn packed_extents(
        &mut self,
        range: Option<&ast::Range>,
        packed: &[ast::Range],
    ) -> Vec<(u32, u32, bool)> {
        let mut out = Vec::new();
        for r in range.into_iter().chain(packed.iter()) {
            // Negative folded bounds (underflow artifact) clamp to 0 — width math
            // stays small instead of the old u32-wrap explosion.
            let msb_v = self.const_eval_in_scope(&r.msb);
            let lsb_v = self.const_eval_in_scope(&r.lsb);
            // P0-NCW: net/hierarchical-referenced (non-constant) packed bound is loud.
            self.check_const_range_bound(&r.msb, msb_v);
            self.check_const_range_bound(&r.lsb, lsb_v);
            let msb = clamp_bound_u32(msb_v);
            let lsb = clamp_bound_u32(lsb_v);
            let w = (((msb.abs_diff(lsb) as u64) + 1).min(u32::MAX as u64)) as u32;
            out.push((msb.min(lsb), w.max(1), msb < lsb));
        }
        out
    }

    /// `Select(e, 0, n)` — the unsigned low `n` value-bits (truncate primitive).
    pub(crate) fn select_low(&mut self, e: u32, n: u32) -> u32 {
        let off = self.const_u32_expr(0, 32);
        let wid = self.const_u32_expr(n, 32);
        self.push_expr(ir::Expr::Select {
            base: e,
            offset: off,
            width: wid,
            kind: ir::SelKind::PartConst,
        })
    }

    /// Widen `e` (self-width `w`) to `n` bits (n > w), PRESERVING 4-state X/Z.
    /// Sign-extend (fill = the operand's MSB) iff `signed_op`, else zero-extend.
    /// Built from `Concat[Replicate(n-w, fill_bit), e]` so the operand bits and the
    /// fill survive verbatim — a bitwise `e | 0` would both zero-extend a signed
    /// operand AND collapse Z→X (`z | 0 = x`), the two extend-path silent-wrongs.
    pub(crate) fn extend_to(&mut self, e: u32, w: u32, n: u32, signed_op: bool) -> u32 {
        let fill_bit = if signed_op {
            let off = self.const_u32_expr(w.saturating_sub(1), 32);
            let wid = self.const_u32_expr(1, 32);
            self.push_expr(ir::Expr::Select {
                base: e,
                offset: off,
                width: wid,
                kind: ir::SelKind::Bit,
            })
        } else {
            self.const_u32_expr(0, 1)
        };
        let count = self.const_u32_expr(n - w, 32);
        let fill = self.push_expr(ir::Expr::Replicate {
            count,
            value: fill_bit,
        });
        // Concat is MSB-first: the high fill, then the operand's low bits.
        self.push_expr(ir::Expr::Concat {
            parts: vec![fill, e],
        })
    }

    /// Normalize a select offset (a SOURCE bit index) into an internal-bit position
    /// for a net declared `[msb:lsb]`: descending (`msb≥lsb`) → `idx − lsb`; ascending
    /// (`msb<lsb`) → `lsb − idx`. A plain `[N:0]` net (lsb 0, descending) returns the
    /// raw offset unchanged so the long-standing golden IR is byte-for-byte preserved.
    /// A POISON/out-of-range net id (error recovery) is a no-op.
    /// r19: lower a select INDEX / OFFSET / bound expression, rejecting a REAL value.
    /// IEEE §11.5.1 requires an integral index; a real one has no bit position, and
    /// the engine folded it to 0 — `v[R]` with a real `R` silently read the wrong bit,
    /// a real part-select bound produced a multi-megabit X, and a real lvalue index
    /// silently DROPPED the write. One wrapper for every index site so the rule cannot
    /// drift between rvalue and lvalue paths. (Real PARAMETERS made this reachable;
    /// a real literal index `v[1.5]` was always reachable and is covered too.)
    pub(crate) fn lower_index_expr(&mut self, e: &ast::Expr) -> u32 {
        // r19/S1: a real-returning FUNCTION. `expr_is_real` works on the lowered IR
        // and cannot see this: the inline path folds the body to an expression whose
        // nodes carry no real marker, and `func_return_dims` computes the Real kind
        // only to discard it, so the return net is not `NetKind::Real` on either
        // path. The DECLARATION does know, and this wrapper still holds the AST —
        // so ask the declaration. `v[7:f()]` silently dropped the write before this.
        // r19/S1: a real-returning FUNCTION. `expr_is_real` works on the lowered
        // IR and cannot see this — the inline path folds the body to an expression
        // with no real marker, and `func_return_dims` computes the Real kind only to
        // discard it, so the return net is not `NetKind::Real` on either path. The
        // DECLARATION knows, and this wrapper still holds the AST.
        //
        // Delegated, NOT re-implemented: the first version was a near-copy that
        // dropped this predicate's operator guards, so `v[fr(2) > 2.0]` — where `>`
        // consumes a real and yields an INTEGRAL result — was false-loud at eight
        // gated sites. `ast_has_real_call` restricts propagation to `+ - * /` and
        // the ternary arms, matching the IR-side twin `expr_is_real`.
        if self.ast_has_real_call(e) {
            let id = self.lower_expr(e);
            self.error(
                MsgCode::ElabUnsupported,
                "a select index / bound / size must be integral, not real (IEEE §11.5.1) \
                 — this reads a real-returning function",
            );
            let _ = id;
            return self.const_u32_expr(0, 32);
        }
        let id = self.lower_expr(e);
        if self.expr_is_real(id) {
            // r19: a real param whose initializer folded EXACTLY to an integer is
            // registered in BOTH `real_param_val` and `params`, but `lower_expr`
            // prefers the real map — so the twin arrives here as a real `Const` and
            // is indistinguishable from `1.5`. Converting it HERE is correct and
            // converting it at the leaf was not: this is the context boundary that
            // requires an integral operand, so `R/2` still divides in the real
            // domain while `new[R]` / `v[7:R]` get the integer they need (IEEE
            // §11.8.1 evaluates in the real domain and converts once, at the
            // boundary). EXACT only — a fractional value has no integral meaning
            // and stays loud, which is why the no-twin case is unaffected.
            if let Some(v) = self.const_real_exact_u32(id) {
                return self.const_u32_expr(v, 32);
            }
            self.error(
                MsgCode::ElabUnsupported,
                "a select index / bound / size must be integral, not real (IEEE §11.5.1)",
            );
            return self.const_u32_expr(0, 32);
        }
        id
    }

    /// r19: the exact non-negative integer behind a real `Const`, or `None` when the
    /// value is fractional, negative, or out of range. Deliberately exact: rounding
    /// here would silently accept `v[2.7]`, and this helper's whole purpose is to let
    /// an integer-valued real through WITHOUT admitting an approximation.
    pub(crate) fn const_real_exact_u32(&self, eid: u32) -> Option<u32> {
        let ir::Expr::Const { val } = self.exprs.get(eid as usize)? else {
            return None;
        };
        let c = self.consts.get(*val as usize)?;
        if !matches!(c.repr, ir::ConstRepr::Real) {
            return None;
        }
        let x = f64::from_bits(*c.bits.val.first()?);
        (x.fract() == 0.0 && x >= 0.0 && x <= u32::MAX as f64).then_some(x as u32)
    }

    pub(crate) fn norm_offset_for_net(&mut self, net: u32, raw_off: u32) -> u32 {
        let Some((msb, lsb)) = self.nets.get(net as usize).map(|nv| (nv.msb, nv.lsb)) else {
            return raw_off;
        };
        if msb >= lsb {
            if lsb == 0 {
                return raw_off; // `[N:0]` — raw index is already internal
            }
            let lsb_c = self.const_u32_expr(lsb, 32);
            self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: raw_off,
                rhs: lsb_c,
            })
        } else {
            // ascending `[lo:hi]`: the largest source index (`lsb`) is internal bit 0.
            let lsb_c = self.const_u32_expr(lsb, 32);
            self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: lsb_c,
                rhs: raw_off,
            })
        }
    }

    /// If `base` is a multi-dim packed ELEMENT (`pm[i]…`, where `pm` is a packed
    /// net — local, interface-member, or package) whose residual after the peeled
    /// index/indices is EXACTLY ONE un-indexed packed dim (a residual VECTOR),
    /// return that residual dim's `(lo, width, ascending)`. A sub-select of such an
    /// element (`pm[i][m:l]`, `pm[i][b+:w]`) normalizes its offset against the
    /// residual dim's LSB — the packed twin of the array-element / struct-member
    /// `dbase`; the element extract (`lower_expr(pm[i])`) is already the `[w-1:0]`
    /// value. `None` for a bare net, a fully-indexed (bit) access, or a residual of
    /// >1 dim (a deeper follow-on) — those stay on their existing paths.
    pub(crate) fn packed_elem_resid(&self, base: &ast::Expr) -> Option<(u32, u32, bool)> {
        let mut n_idx = 0usize;
        let mut cur = base;
        loop {
            match &cur.kind {
                ast::ExprKind::Paren { inner } => cur = inner,
                ast::ExprKind::BitSelect { base: b, .. } => {
                    n_idx += 1;
                    cur = b;
                }
                _ => break,
            }
        }
        let net = match &cur.kind {
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                self.lookup_net_scoped(&path.segments[0].name)?
            }
            ast::ExprKind::Ident(path) => self.iface_member_net(path)?,
            ast::ExprKind::PkgScoped { .. } => self.pkg_scoped_var_net(cur)?,
            _ => return None,
        };
        let dims = self.packed_dims.get(&net)?;
        // Confine to a BARE packed net (NOT an unpacked array). An array-of-packed's
        // UNPACKED indices would be miscounted as packed by `n_idx`, so a PARTIAL
        // packed-index sub-select (`tm[0][0][m:l]` on `reg [a][b][c] tm [0:1]`) would
        // normalize against the innermost dim while >1 packed dim actually remains →
        // silent-wrong. `net_is_static_array` is true only for an unpacked array (a
        // bare packed net is not), so this excludes every array-of-packed. The genuine
        // array-of-packed residual-vector case stays on the pre-existing raw path (a
        // separate follow-on), unchanged.
        if self.net_is_static_array(net) {
            return None;
        }
        // Residual = exactly ONE un-indexed dim (a vector): the peeled indices leave
        // `dims.len() - 1` dims. `n_idx == 0` is a bare packed net (its own path).
        if n_idx == 0 || n_idx != dims.len() - 1 {
            return None;
        }
        Some(dims[n_idx])
    }

    /// Offset normalization against an EXPLICIT `(lo, width, ascending)` range — the
    /// range-explicit twin of [`Self::norm_offset_for_net`] (used for a packed
    /// element's RESIDUAL dim, whose range is not a whole net's). Descending: a
    /// `lo == 0` range is a no-op (raw), else `raw − lo`. Ascending `[lo:hi]`: the
    /// largest source index `hi = lo + width − 1` is internal bit 0, so `hi − raw`.
    pub(crate) fn norm_offset_for_range(
        &mut self,
        raw_off: u32,
        lo: u32,
        width: u32,
        asc: bool,
    ) -> u32 {
        if !asc {
            if lo == 0 {
                return raw_off;
            }
            let lo_c = self.const_u32_expr(lo, 32);
            self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: raw_off,
                rhs: lo_c,
            })
        } else {
            let hi_c = self.const_u32_expr(lo + width - 1, 32);
            self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: hi_c,
                rhs: raw_off,
            })
        }
    }

    pub(crate) fn norm_offset_if_net(&mut self, base: &ast::Expr, raw_off: u32) -> u32 {
        if let ast::ExprKind::Ident(path) = &base.kind {
            if path.segments.len() == 1 {
                if let Some(net) = self.lookup_net_scoped(&path.segments[0].name) {
                    return self.norm_offset_for_net(net, raw_off);
                }
                // A NON-zero-LSB param/localparam (`localparam [15:8] P; P[15:12]`) —
                // the base folds to a Const (not a net), so normalize by the param's
                // DECLARED range (recorded in `param_range`, resolved by the SAME
                // `walk_scopes` as the value). A zero-LSB param has no entry → raw
                // (byte-identical).
                if let Some((lo, w, asc)) = self.param_sel_range(base) {
                    return self.norm_offset_for_range(raw_off, lo, w, asc);
                }
            } else if let Some(net) = self.iface_member_net(path) {
                // Interface-member alias (`bi.data`, ≥2-seg) — a KNOWN dotted symbol
                // resolved at port-binding; normalize by its declared range like a
                // single-seg net so a non-zero-LSB member (`logic [15:8] data`)
                // selects the right internal bits (bit + part + indexed read). A
                // hierarchical ref defers via `hier_chain` (never reaches here); a
                // class-field access is not in `lookup_net_scoped` → stays raw.
                return self.norm_offset_for_net(net, raw_off);
            }
        }
        // Explicit DIRECT `pkg::vec[…]` — normalize by the package net's declared
        // range, exactly as the bare `Ident` arm does (a non-zero-LSB `[15:8]` or
        // ascending `[lo:hi]` package vector selects the right internal bit). Only
        // a direct PkgScoped base: a package ARRAY-ELEMENT sub-select
        // (`pkg::mem[i][m:l]`) is loud-guarded upstream, so it never reaches here.
        if let Some(net) = self.pkg_scoped_var_net(base) {
            return self.norm_offset_for_net(net, raw_off);
        }
        // Array-element part/indexed-select `mem[i][m:l]` — peel the element
        // `BitSelect`(s) to the root net and normalize by the ELEMENT's declared
        // range, the descending twin of `norm_offset_ascending`'s `base_root_net`
        // peel (which already handles ascending elements). Without it a non-zero-LSB
        // element (`logic [15:8] mem[0:1]; mem[0][11:8]`) read raw internal bits →
        // silent `x`. Confined to a genuine STATIC-ARRAY element of a SINGLE-DIM
        // vector: `net_is_static_array` excludes an illegal bit-of-bit on a plain
        // vector (`vec[i][j]`, which iverilog rejects — keeps it byte-identical to
        // the raw path); `!packed_dims` excludes a multi-dim packed element, whose
        // residual range after the packed-dim index differs from the whole-net
        // range (deep — left raw, pre-existing silent, tracked separately). A
        // zero-LSB element normalizes to the raw offset anyway, so the ONLY behavior
        // change is a non-zero-LSB single-dim array element (the fix target). A
        // hierarchical `dut.mem[i][m:l]` root yields `None` here (separate deferred
        // path) and stays raw — another pre-existing residual, out of scope.
        if matches!(&base.kind, ast::ExprKind::BitSelect { .. }) {
            // Multi-dim packed ELEMENT sub-select (`pm[i][m:l]`) — normalize by the
            // residual (inner) dim's LSB. The element extract `lower_expr(pm[i])` is
            // the `[w-1:0]` value, so this is the packed twin of the array-element
            // `dbase`. Previously the `!packed_dims` guard below excluded it → raw
            // offset → silent `x` for a non-zero-LSB inner dim (§4.5.103 residual).
            if let Some((lo, w, asc)) = self.packed_elem_resid(base) {
                return self.norm_offset_for_range(raw_off, lo, w, asc);
            }
            if let Some(net) = self.base_root_net(base) {
                if self.net_is_static_array(net) && !self.packed_dims.contains_key(&net) {
                    return self.norm_offset_for_net(net, raw_off);
                }
            }
        }
        raw_off
    }

    /// Is the net (or array-element packed shape) named by `base` declared
    /// ASCENDING (`[lo:hi]`, `msb < lsb`)? A base that does not resolve to a net is
    /// `false` (treated as the classic descending `[N:0]`).
    pub(crate) fn base_net_ascending(&self, base: &ast::Expr) -> bool {
        // A packed element's RESIDUAL (inner) dim drives the direction/width, not
        // the whole net's outer dim (`pm[i][m:l]` on `logic [1:0][15:8]` is a
        // descending inner select regardless of the outer dim's direction).
        if let Some((_lo, _w, asc)) = self.packed_elem_resid(base) {
            return asc;
        }
        // A non-zero-LSB param's declared direction drives the select (an ascending
        // `parameter [8:15] P` part-select maps like an ascending net).
        if let Some((_lo, _w, asc)) = self.param_sel_range(base) {
            return asc;
        }
        self.base_root_net(base)
            .map(|net| self.net_ascending(net))
            .unwrap_or(false)
    }

    /// Offset normalization for a part-select base on an ASCENDING net: peel the
    /// base to its root net and map the source index onto an internal-bit position
    /// (`norm_offset_for_net`). Only used when `base_net_ascending(base)` is true,
    /// so `base_root_net` is guaranteed `Some`.
    pub(crate) fn norm_offset_ascending(&mut self, base: &ast::Expr, raw_off: u32) -> u32 {
        // Ascending packed element — normalize against the residual dim's range
        // (the descending twin runs in `norm_offset_if_net`).
        if let Some((lo, w, asc)) = self.packed_elem_resid(base) {
            return self.norm_offset_for_range(raw_off, lo, w, asc);
        }
        // Ascending non-zero-LSB param — normalize against its declared range.
        if let Some((lo, w, asc)) = self.param_sel_range(base) {
            return self.norm_offset_for_range(raw_off, lo, w, asc);
        }
        match self.base_root_net(base) {
            Some(net) => self.norm_offset_for_net(net, raw_off),
            None => raw_off,
        }
    }

    /// Is net `net` declared ascending (`[lo:hi]`)? Out-of-range id ⇒ `false`.
    pub(crate) fn net_ascending(&self, net: u32) -> bool {
        self.nets
            .get(net as usize)
            .map(|nv| nv.msb < nv.lsb)
            .unwrap_or(false)
    }

    /// Descending-default wrapper for [`Self::width_from_msb_lsb_dir`] — used where
    /// the net direction is not yet known (deferred hierarchical part-select write).
    pub(crate) fn width_from_msb_lsb_checked(
        &mut self,
        msb_ast: &ast::Expr,
        lsb_ast: &ast::Expr,
        msb_id: u32,
        lsb_id: u32,
    ) -> u32 {
        self.width_from_msb_lsb_dir(msb_ast, lsb_ast, msb_id, lsb_id, false)
    }

    /// Part-select width, direction-aware.
    ///
    /// DESCENDING net (`ascending == false`): the legal select is `[msb:lsb]` with
    /// `msb ≥ lsb`; width = `(msb - lsb) + 1` as an UNFOLDED arena tree (no
    /// const-fold in v1 — the golden IR shape). `msb_const < lsb_const` is a
    /// direction mismatch ("out of order") → `ElabUnsupported` (the inert width
    /// tree is still synthesized to keep the arena valid).
    ///
    /// ASCENDING net (`ascending == true`, `[lo:hi]`): the legal select is
    /// `[msb:lsb]` with `msb ≤ lsb`; width = `(lsb - msb) + 1` folded to a `Const`
    /// (the unsigned `msb_id - lsb_id` arena Sub would underflow). `msb_const >
    /// lsb_const` is a direction mismatch → `ElabUnsupported`. The offset machinery
    /// (`norm_offset_for_net`) already maps the larger source index onto internal
    /// bit 0, so only the width differs.
    pub(crate) fn width_from_msb_lsb_dir(
        &mut self,
        msb_ast: &ast::Expr,
        lsb_ast: &ast::Expr,
        msb_id: u32,
        lsb_id: u32,
        ascending: bool,
    ) -> u32 {
        let folded = (const_eval_u32(msb_ast), const_eval_u32(lsb_ast));
        if let (Some(m), Some(l)) = folded {
            if ascending {
                if m > l {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "part-select bounds [msb:lsb] descend but the net is ascending [lo:hi] (out of order)",
                    );
                } else {
                    // width = (l - m) + 1, folded; offset handled by norm_offset.
                    return self.const_u32_expr(l - m + 1, 32);
                }
            } else if m < l {
                self.error(
                    MsgCode::ElabUnsupported,
                    "part-select bounds [msb:lsb] ascend but the net is descending [hi:lo] (out of order)",
                );
            }
        }
        let diff = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Sub,
            lhs: msb_id,
            rhs: lsb_id,
        });
        let one = self.const_u32_expr(1, 32);
        self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: diff,
            rhs: one,
        })
    }

    /// Like [`Self::expr_array_chain`] but for a multi-dim PACKED net (a flat vector
    /// recorded in `packed_dims`): `m[i]…[k]` selects a bit-SLICE, not a word.
    pub(crate) fn expr_packed_chain<'a>(
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
                // 1-segment local OR multi-segment resolvable hierarchical packed net
                // (same-module generate scope `g[0].pm` — HIER-REST②). Cross-instance
                // unresolved → None → deferred-sel lane.
                ast::ExprKind::Ident(p) => {
                    let joined = p
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    match self.lookup_net_scoped(&joined) {
                        Some(n) if self.packed_dims.contains_key(&n) => break n,
                        _ => return None,
                    }
                }
                // Explicit `pkg::pm[i]` — multi-dim packed bit-slice off a
                // package variable net (twin of the `expr_array_chain` arm).
                ast::ExprKind::PkgScoped { .. } => match self.pkg_scoped_var_net(cur) {
                    Some(n) if self.packed_dims.contains_key(&n) => break n,
                    _ => return None,
                },
                _ => return None,
            }
        };
        outer_first.reverse();
        outer_first.push(index);
        Some((net, outer_first))
    }

    /// Lower a read `m[i0]…[ik]` on a packed multi-dim net to a bit-slice. The first
    /// `k` indices give the bit OFFSET (`(i-lo)*stride`, stride = product of inner
    /// dim widths — reusing [`Self::flatten_word`]); the result WIDTH is the product
    /// of the un-indexed inner dims. Lowered to an indexed part-select.
    pub(crate) fn lower_packed_read(&mut self, net: u32, idxs: &[&ast::Expr]) -> u32 {
        let dims = self.packed_dims[&net].clone();
        if idxs.len() > dims.len() {
            self.error(
                MsgCode::ElabUnsupported,
                "too many indices for packed array (more than its dimensions)",
            );
            return self.placeholder_expr();
        }
        // §4.5.199: an md-packed UNPACKED-array frame slot (`frame_arr_formal_meta`) has
        // NO whole-value surface, so a PARTIAL index (`m[i]` on a 2-D `int m[2][2]`, fewer
        // indices than unpacked dims) must not silently return a multi-element sub-array
        // slice — index every dimension down to the scalar element (a trailing bit/part-
        // select is still fine: `idxs.len()+1 == dims.len()` for the element, one MORE for
        // a bit-select). A genuine multi-dim PACKED net (`reg [3:0][7:0] x; x[i]`) is NOT
        // in `frame_arr_formal_meta`, so its legal partial sub-element select is untouched.
        // Never fires for a 1-D md-packed array (`dims.len()==2` ⇒ needs `idxs.len()==0`,
        // impossible on a select), so the 1-D golden IR is byte-identical.
        if idxs.len() + 1 < dims.len() && self.frame_arr_formal_meta.contains_key(&net) {
            self.error(
                MsgCode::ElabUnsupported,
                "a partial slice of an unpacked array (index every dimension down to a \
                 scalar element; a whole sub-array has no value in this context)",
            );
            return self.placeholder_expr();
        }
        let (ext, dirs) = Self::packed_split(&dims);
        let offset = self.flatten_word(&ext, idxs, &dirs);
        let elem_w: u64 = dims[idxs.len()..]
            .iter()
            .map(|&(_, w, _)| w as u64)
            .product();
        let base = self.push_expr(ir::Expr::Signal { net, word: None });
        let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
        let sel = self.push_expr(ir::Expr::Select {
            base,
            offset,
            width,
            kind: ir::SelKind::PartIdxUp,
        });
        // G3: a signed-element unpacked-array FORMAL (`byte b[0:3]`) is md-packed with a
        // whole-`signed:false` slot; a WHOLE-element read (`b[i]`) is a part-select,
        // unsigned per §11.5.1 — re-stamp `$signed` so a negative element reads negative
        // (else -1 → 255, silent-wrong). Gated on `frame_arr_formal_meta` (a regular
        // multi-dim packed net element stays unsigned) AND a whole-element read
        // (`idxs.len()+1 == dims.len()`; a sub-bit `b[i][k]` stays unsigned per §11.5.1).
        if idxs.len() + 1 == dims.len()
            && self
                .frame_arr_formal_meta
                .get(&net)
                .is_some_and(|af| af.elem_signed)
        {
            return self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed,
                args: vec![sel],
            });
        }
        sel
    }

    /// Resolve the base of a part-select / indexed part-select to a multi-dim
    /// PACKED net plus an optional element-word selector:
    ///   - a bare `Ident` ⇒ `(net, None)` (the whole net is the packed word);
    ///   - `arr[idx]` where `arr` is a 1-D array of multi-dim packed ⇒
    ///     `(net, Some(elem_word))` (N3.4 follow-on `qm[i][3:2]` — a part-select
    ///     WITHIN an array element), the element word flattened over the array's
    ///     unpacked dims exactly as [`Self::lower_array_read`] does.
    ///
    /// Yields `None` for a plain vector, a non-array/non-packed base, or a deeper
    /// nesting (multi-D unpacked array, or partial/surplus indices) — the caller
    /// then falls through to the generic path (a plain vector is correct flat-bits).
    pub(crate) fn packed_ps_base(&mut self, base: &ast::Expr) -> Option<(u32, Option<u32>)> {
        match &base.kind {
            ast::ExprKind::Ident(path) => self.bare_packed_net(path).map(|n| (n, None)),
            // Explicit `pkg::pm[m:l]`/`[b+:w]` — the outer part-select of a
            // multi-dim packed package var (twin of the `bare_packed_net` arm; a
            // scalar/vector or single-dim pkg net yields None → generic flat-bits,
            // which is correct after `norm_offset_if_net` sees the PkgScoped base).
            ast::ExprKind::PkgScoped { .. } => self
                .pkg_scoped_var_net(base)
                .filter(|n| self.packed_dims.get(n).is_some_and(|d| d.len() >= 2))
                .map(|n| (n, None)),
            ast::ExprKind::BitSelect { base: b, index } => {
                let (net, idxs) = self.expr_array_chain(b, index)?;
                // element must itself be multi-dim packed, and EVERY unpacked dim
                // indexed (full element select — partial/surplus ⇒ None ⇒ generic).
                if self.packed_dims.get(&net).is_none_or(|d| d.len() < 2) {
                    return None;
                }
                let dims = self.net_dim_extents(net);
                if idxs.len() != dims.len() {
                    return None;
                }
                let word = self.flatten_word(&dims, &idxs, &[]);
                Some((net, Some(word)))
            }
            _ => None,
        }
    }

    /// Shared `(m, l)` resolution for a constant indexed part-select on multi-dim
    /// packed `net`: `+:` ⇒ `[c+w-1 : c]`, `-:` ⇒ `[c : c-w+1]`. A non-const offset
    /// ⇒ E3009 (`Err`, iverilog aborts); width is the const element count. Underflow
    /// of `-:` and over-range are out-of-bounds (`Err` via [`Self::packed_outer_range`]).
    pub(crate) fn packed_indexed_range(
        &mut self,
        net: u32,
        offset: &ast::Expr,
        width: &ast::Expr,
        dir: &ast::PartDir,
    ) -> Result<(u32, u32), ()> {
        let Some(w) = const_eval_u32(width) else {
            self.error(
                MsgCode::ElabUnsupported,
                "indexed part-select width must be constant",
            );
            return Err(());
        };
        if w == 0 {
            self.error(
                MsgCode::ElabUnsupported,
                "indexed part-select width must be ≥ 1",
            );
            return Err(());
        }
        let Some(c) = const_eval_u32(offset) else {
            self.error(
                MsgCode::ElabUnsupported,
                "variable indexed part-select on a multi-dim packed array is \
                 unsupported (iverilog 13.0 also rejects it; the bit-vs-element \
                 unit is undefined)",
            );
            return Err(());
        };
        // selected element index SET, direction-agnostic (`[c+:w]` = {c..c+w-1},
        // `[c-:w]` = {c-w+1..c}); `packed_outer_range` maps it to flat bits per the
        // net's own dimension direction (ascending or descending). All index math
        // in u64: a huge/negative-folded `c` (`x[-1 +: 2]` → 0xFFFF_FFFF via
        // `const_eval_u32`'s wrapping_neg) used to overflow `c + w` in u32 — a
        // debug panic instead of the clean loud reject below (iverilog also
        // rejects; adversarial review).
        let (lo64, hi64) = match dir {
            ast::PartDir::PlusColon => (c as u64, c as u64 + w as u64 - 1),
            ast::PartDir::MinusColon => {
                if (c as u64) + 1 < w as u64 {
                    // c-w+1 < 0 — below the lowest element index.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "part-select range exceeds the declared bounds of the packed array",
                    );
                    return Err(());
                }
                // low index = c-w+1; add BEFORE subtract so the legal low-end case
                // (c-w+1 == 0, e.g. `x[0-:1]`/`x[1-:2]`) does not underflow — the
                // guard above already ensures `c + 1 >= w` (review M1).
                (c as u64 + 1 - w as u64, c as u64)
            }
        };
        let (Ok(lo), Ok(hi)) = (u32::try_from(lo64), u32::try_from(hi64)) else {
            self.error(
                MsgCode::ElabUnsupported,
                "part-select range exceeds the declared bounds of the packed array",
            );
            return Err(());
        };
        self.packed_outer_range(net, lo, hi)
    }

    /// A single-segment path that resolves to a multi-dim (≥2) PACKED net.
    pub(crate) fn bare_packed_net(&self, path: &ast::HierPath) -> Option<u32> {
        if path.segments.len() != 1 {
            return None;
        }
        let net = self.lookup_net_scoped(&path.segments[0].name)?;
        let dims = self.packed_dims.get(&net)?;
        (dims.len() >= 2).then_some(net)
    }

    /// N3.4 shared resolution for an outer-dim part-select on multi-dim packed
    /// `net`. The outer `Option` distinguishes NOT-APPLICABLE (a non-const select
    /// ⇒ `None` ⇒ caller falls through to the generic path) from APPLICABLE; the
    /// inner `Result` is `Ok((base-element flat bit-offset eid, count×elem_w width
    /// eid))` for an in-range select, or `Err(())` for an OUT-OF-RANGE or a
    /// direction-MISMATCHED ("out of order") select — in which case E3009 has
    /// already been emitted (iverilog rejects both at compile, so vita does too
    /// rather than silently reading/writing past or against the net). Handles BOTH
    /// directions: a descending net takes `[msb:lsb]` (a≥b), an ascending net takes
    /// `[lo:hi]` (a≤b). Mirrors [`Self::lower_packed_read`] with one outer index but
    /// a multi-element width.
    pub(crate) fn packed_outer_part_select(
        &mut self,
        net: u32,
        msb: &ast::Expr,
        lsb: &ast::Expr,
    ) -> Option<Result<(u32, u32), ()>> {
        let (a, b) = (const_eval_u32(msb)?, const_eval_u32(lsb)?);
        // The part-select direction must match the net's outer-dim direction — a
        // descending net (`[3:0]`) takes a descending select (`x[3:2]`, a≥b), an
        // ascending net (`[0:3]`) takes an ascending select (`x[0:1]`, a≤b). A
        // reversed select is "out of order" (iverilog rejects it at compile). Either
        // way the selected element index SET is `[min(a,b) ..= max(a,b)]`.
        let ascending = self.packed_dims[&net][0].2;
        let dir_ok = if ascending { a <= b } else { a >= b };
        if !dir_ok {
            self.error(
                MsgCode::ElabUnsupported,
                "reversed part-select on a multi-dim packed array is out of order \
                 (the select direction must match the declared dimension)",
            );
            return Some(Err(()));
        }
        Some(self.packed_outer_range(net, a.min(b), a.max(b)))
    }

    /// Core of [`Self::packed_outer_part_select`]: an outer-dim element range
    /// `[lo ..= hi]` (`lo ≤ hi`, the selected index SET, both already resolved to
    /// constants) on multi-dim packed `net`, also reached by a constant indexed
    /// part-select (`x[c+:w]` ⇒ {c..c+w-1}). Returns `Ok((base-element flat bit-offset
    /// eid, count×elem_w width eid))` for an in-range select, or `Err(())` after
    /// emitting E3009 for an out-of-range select. The base coord is the const form of
    /// [`Self::flatten_word`] for the range's lowest-addressed element — `lo-olo`
    /// (descending) or `ohi-hi` (ascending) — so each direction lands byte-identically
    /// to its single-element read.
    pub(crate) fn packed_outer_range(
        &mut self,
        net: u32,
        lo: u32,
        hi: u32,
    ) -> Result<(u32, u32), ()> {
        let dims = self.packed_dims[&net].clone();
        let (olo, osize, ascending) = dims[0];
        // the FULL `[lo ..= hi]` span (the selected index SET, `lo ≤ hi`) must lie
        // inside the outer dim (dims[0]). iverilog rejects an over-bounds part-select
        // at compile time (a variable indexed part-select aborts in 13.0, which
        // `try_packed_indexed_part` handles separately).
        let ohi = olo + osize - 1;
        if lo < olo || hi > ohi {
            self.error(
                MsgCode::ElabUnsupported,
                "part-select range exceeds the declared bounds of the packed array",
            );
            return Err(());
        }
        // outer dim is dims[0]; an element is the product of the inner dims, which
        // is exactly the outer dim's stride in `flatten_word`.
        let elem_w: u64 = dims[1..].iter().map(|&(_, w, _)| w as u64).product();
        let count = (hi - lo + 1) as u64;
        // flat-bit coord of the range's lowest-addressed element (its base), exactly
        // as `flatten_word`/`flatten_word_eids` map one outer index:
        //   descending net: idx → coord (idx − olo)  ⇒ lowest = lo
        //   ascending  net: idx → coord (ohi − idx)  ⇒ lowest = ohi − hi
        let coord = if ascending {
            (ohi - hi) as u64
        } else {
            (lo - olo) as u64
        };
        let offset = self.const_u32_expr((coord * elem_w).min(u32::MAX as u64) as u32, 32);
        let width = self.const_u32_expr((count * elem_w).min(u32::MAX as u64) as u32, 32);
        Ok((offset, width))
    }

    /// Split a packed-dim table `(lo, size, ascending)` into the `(lo, size)` extents
    /// `flatten_word` consumes plus the per-dim `ascending` flags (N3.3). Lets the
    /// packed read/write paths share `flatten_word` with the unpacked path.
    pub(crate) fn packed_split(dims: &[(u32, u32, bool)]) -> (Vec<(i64, u32)>, Vec<bool>) {
        let ext = dims.iter().map(|&(l, s, _)| (i64::from(l), s)).collect();
        let dirs = dims.iter().map(|&(_, _, a)| a).collect();
        (ext, dirs)
    }

    /// Per-position word offsets of a residual sub-array in DECLARED
    /// left-to-right order: position 0 is the leftmost element of every dim.
    /// `dims` are the residual `(lo, size)` extents (trailing dims of the
    /// full array, so suffix-product strides within the residual equal the
    /// full array's strides); `desc[k]` flips dim `k`'s traversal.
    pub(crate) fn residual_word_offsets(dims: &[(i64, u32)], desc: &[bool]) -> Vec<u32> {
        let n: u64 = dims.iter().map(|&(_, s)| s as u64).product();
        let mut strides = vec![1u64; dims.len()];
        for k in (0..dims.len().saturating_sub(1)).rev() {
            strides[k] = strides[k + 1].saturating_mul(dims[k + 1].1 as u64);
        }
        (0..n)
            .map(|p| {
                let mut rem = p;
                let mut off = 0u64;
                for k in (0..dims.len()).rev() {
                    let size = dims[k].1 as u64;
                    let digit = rem % size;
                    rem /= size;
                    let slot = if desc.get(k).copied().unwrap_or(false) {
                        size - 1 - digit
                    } else {
                        digit
                    };
                    off += slot * strides[k];
                }
                off.min(u32::MAX as u64) as u32
            })
            .collect()
    }

    /// Word ExprId for `base + off` (no Add node when either side is trivial).
    pub(crate) fn word_expr_at(&mut self, base: Option<u32>, off: u32) -> u32 {
        match base {
            None => self.const_u32_expr(off, 32),
            Some(b) if off == 0 => b,
            Some(b) => {
                let c = self.const_u32_expr(off, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: b,
                    rhs: c,
                })
            }
        }
    }

    /// `clk[k]` maps EXACTLY to arming on the underlying net (bit-0 edge) iff `k`
    /// is a compile-time constant equal to the net's LSB endpoint. `nv.lsb` is the
    /// source index that lands on packed bit 0 in BOTH range directions (descending
    /// `[hi:lo]` → `lo`; ascending `[lo:hi]` stored as `msb<lsb` → the larger bound
    /// `lsb`). Returns the net id when supported, else `None` (→ caller rejects loud).
    pub(crate) fn lsb_bitselect_net(&self, base: &ast::Expr, index: &ast::Expr) -> Option<u32> {
        let net = match &base.kind {
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                self.lookup_net_scoped(&path.segments[0].name)?
            }
            // `@(posedge p::vec[lsb])` — a scoped package vector's LSB bit-select,
            // the same net (and LSB rule) the imported bare `@(posedge vec[lsb])`
            // arms on. A package constant / unknown yields None → caller rejects.
            ast::ExprKind::PkgScoped { .. } => self.pkg_scoped_var_net(base)?,
            _ => return None, // computed / hierarchical / concat / multi-seg base
        };
        // Reject array elements (multi-bit words), multi-dim packed selects, and
        // dyn-storage/string handles — none is a scalar net whose bit 0 we can arm.
        if self.net_is_static_array(net)
            || self.packed_dims.contains_key(&net)
            || self.is_dyn_handle_net(net)
            || self.is_string_net(net)
        {
            return None;
        }
        let k = self.const_eval_in_scope(index)?;
        let lsb = self.nets.get(net as usize)?.lsb as i64;
        (k == lsb).then_some(net)
    }

    /// Width-edge fold for `ir_bits_of`: a direct `Const`, or the shallow
    /// `Add(Sub(msb,lsb),1)` tree elaborate synthesizes for `[msb:lsb]` —
    /// the same two shapes the engine's width-table fold accepts.
    pub(crate) fn width_edge_u32(&self, eid: u32) -> Option<u32> {
        if let Some(c) = self.const_of_expr_u32(eid) {
            return Some(c);
        }
        match self.exprs.get(eid as usize)? {
            ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs,
                rhs,
            } => {
                let a = self.width_edge_u32(*lhs)?;
                let b = self.width_edge_u32(*rhs)?;
                Some(a.saturating_add(b))
            }
            ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs,
                rhs,
            } => {
                let a = self.width_edge_u32(*lhs)?;
                let b = self.width_edge_u32(*rhs)?;
                Some(a.saturating_sub(b))
            }
            _ => None,
        }
    }
}
