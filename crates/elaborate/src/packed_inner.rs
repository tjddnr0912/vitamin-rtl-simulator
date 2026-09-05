//! §2 🆕 L ⓞ (§4.5.427): a part-select on a NON-innermost dimension of a partially
//! indexed multi-dim packed net. `logic [1:0][2:0][1:0] v; v[1][2:1]` selects two whole
//! 2-bit sub-elements of `v[1]` (`1011`, both oracles); the generic path read two
//! flat BITS of the element (`10`). The bare-net twin (`v[2:1]`, dim 0) has been
//! handled by `try_packed_part_select` since N3.4; this is the same rule one or more
//! indices in — the selected dimension is `k = idxs.len()`, the element width is the
//! product of the dims after it, and the prefix indices give the sub-array's own
//! bit offset (`flatten_word`, the one `lower_packed_read` uses). The innermost
//! dimension (`v[1][2][1:0]`) stays on the generic path, where flat bits ARE the
//! elements. Read and write twins share the range helper.

use super::*;
use crate::array_geom::IndexDomain;

impl Elaborator<'_> {
    /// `base` = `v[i0]…[i(k-1)]` on a multi-dim PACKED net (not an unpacked array of
    /// packed) with at least TWO dims left after the indices ⇒ `(net, k, prefix)`.
    pub(crate) fn packed_inner_ps_base(
        &mut self,
        base: &ast::Expr,
    ) -> Option<(u32, Option<u32>, usize, u32)> {
        let ast::ExprKind::BitSelect { base: b, index } = &base.kind else {
            return None;
        };
        // An unpacked ARRAY of multi-dim packed (`m[1][1][2:1]`): the leading indices
        // are words (`expr_array_chain`), the rest packed — the same split
        // `lower_packed_read` makes for the element read.
        if let Some((net, idxs)) = self.expr_array_chain(b, index) {
            return self.packed_inner_prefix_arr(net, &idxs);
        }
        let (net, idxs) = self.expr_packed_chain(b, index)?;
        self.packed_inner_prefix(net, &idxs)
            .map(|(net, k, prefix)| (net, None, k, prefix))
    }

    /// Write-side twin of [`Self::packed_inner_ps_base`].
    pub(crate) fn packed_inner_ps_base_lval(
        &mut self,
        base: &ast::Lvalue,
    ) -> Option<(u32, Option<u32>, usize, u32)> {
        let ast::Lvalue::BitSelect { base: b, index, .. } = base else {
            return None;
        };
        if let Some((net, idxs)) = self.lval_array_chain(b, index) {
            return self.packed_inner_prefix_arr(net, &idxs);
        }
        let (net, idxs) = self.lval_packed_chain(b, index)?;
        self.packed_inner_prefix(net, &idxs)
            .map(|(net, k, prefix)| (net, None, k, prefix))
    }

    /// The array-of-packed twin of [`Self::packed_inner_prefix`]: `idxs` = every
    /// unpacked word index, then one or more packed indices leaving ≥ 2 packed dims.
    fn packed_inner_prefix_arr(
        &mut self,
        net: u32,
        idxs: &[&ast::Expr],
    ) -> Option<(u32, Option<u32>, usize, u32)> {
        let pdims = self.packed_dims.get(&net)?.clone();
        if !self.net_is_static_array(net) || self.frame_arr_formal_meta.contains_key(&net) {
            return None;
        }
        let udims = self.net_dim_extents(net);
        let u = udims.len();
        if idxs.len() <= u {
            return None; // the element itself, or a partial slice — not this lane
        }
        let k = idxs.len() - u;
        if k + 1 >= pdims.len() {
            return None; // innermost dim: flat bits are the elements
        }
        let word = self.flatten_word(&udims, &idxs[..u], &[], IndexDomain::ArrayWord);
        let (ext, dirs) = Self::packed_split(&pdims);
        let prefix = self.flatten_word(&ext, &idxs[u..], &dirs, IndexDomain::PackedElem);
        Some((net, Some(word), k, prefix))
    }

    fn packed_inner_prefix(&mut self, net: u32, idxs: &[&ast::Expr]) -> Option<(u32, usize, u32)> {
        let dims = self.packed_dims.get(&net)?.clone();
        let k = idxs.len();
        // `k == 0` is the bare-net outer select (N3.4); `k + 1 == dims.len()` is the
        // innermost dim, whose "elements" are bits — the generic path is right there.
        if k == 0 || k + 1 >= dims.len() {
            return None;
        }
        // An unpacked array of packed, or a frame formal's array slot: the chain's
        // leading indices are WORDS, not packed offsets — not this lane.
        if self.net_is_static_array(net) || self.frame_arr_formal_meta.contains_key(&net) {
            return None;
        }
        let (ext, dirs) = Self::packed_split(&dims);
        let prefix = self.flatten_word(&ext, idxs, &dirs, IndexDomain::PackedElem);
        Some((net, k, prefix))
    }

    /// The selected index SET `[lo ..= hi]` on dim `k` under `prefix` ⇒
    /// `Ok((offset eid, width eid))`; E3009 + `Err` when it leaves the dim.
    pub(crate) fn packed_inner_range(
        &mut self,
        net: u32,
        k: usize,
        prefix: u32,
        lo: u32,
        hi: u32,
    ) -> Result<(u32, u32), ()> {
        let dims = self.packed_dims[&net].clone();
        let (dlo, dsize, ascending) = dims[k];
        let (lo, hi) = (i64::from(lo), i64::from(hi));
        let dhi = dlo + i64::from(dsize) - 1;
        if lo < dlo || hi > dhi {
            self.error(
                MsgCode::ElabUnsupported,
                "part-select range exceeds the declared bounds of the packed array",
            );
            return Err(());
        }
        let elem_w: u64 = dims[k + 1..].iter().map(|&(_, w, _)| w as u64).product();
        let count = (hi - lo + 1) as u64;
        let coord = if ascending {
            (dhi - hi) as u64
        } else {
            (lo - dlo) as u64
        };
        let rel = (coord * elem_w).min(u32::MAX as u64) as u32;
        let offset = match self.const_index_value(prefix) {
            Some(p) if p >= 0 => {
                self.const_u32_expr((p as u64 + u64::from(rel)).min(u32::MAX as u64) as u32, 32)
            }
            _ => {
                let c = self.const_u32_expr(rel, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: prefix,
                    rhs: c,
                })
            }
        };
        let width = self.const_u32_expr((count * elem_w).min(u32::MAX as u64) as u32, 32);
        Ok((offset, width))
    }

    /// `[msb:lsb]` on dim `k`: the select direction must match the dim's (a reversed
    /// select is "out of order", as on the outer dim).
    pub(crate) fn packed_inner_part_select(
        &mut self,
        net: u32,
        k: usize,
        prefix: u32,
        msb: &ast::Expr,
        lsb: &ast::Expr,
    ) -> Option<Result<(u32, u32), ()>> {
        let (a, b) = (self.const_bound_u32(msb)?, self.const_bound_u32(lsb)?);
        let ascending = self.packed_dims[&net][k].2;
        let dir_ok = if ascending { a <= b } else { a >= b };
        if !dir_ok {
            self.error(
                MsgCode::ElabUnsupported,
                "reversed part-select on a multi-dim packed array is out of order \
                 (the select direction must match the declared dimension)",
            );
            return Some(Err(()));
        }
        Some(self.packed_inner_range(net, k, prefix, a.min(b), a.max(b)))
    }

    /// `[c+:w]` / `[c-:w]` on dim `k` — constant offset and width, as on the outer dim.
    pub(crate) fn packed_inner_indexed(
        &mut self,
        net: u32,
        k: usize,
        prefix: u32,
        offset: &ast::Expr,
        width: &ast::Expr,
        dir: &ast::PartDir,
    ) -> Result<(u32, u32), ()> {
        let Some(w) = self.const_bound_u32(width).filter(|w| *w >= 1) else {
            self.error(
                MsgCode::ElabUnsupported,
                "indexed part-select width must be a constant ≥ 1",
            );
            return Err(());
        };
        let Some(c) = self.const_bound_u32(offset) else {
            self.error(
                MsgCode::ElabUnsupported,
                "variable indexed part-select on a multi-dim packed array is \
                 unsupported (iverilog 13.0 also rejects it; the bit-vs-element \
                 unit is undefined)",
            );
            return Err(());
        };
        let (lo64, hi64) = match dir {
            ast::PartDir::PlusColon => (u64::from(c), u64::from(c) + u64::from(w) - 1),
            ast::PartDir::MinusColon => {
                if u64::from(c) + 1 < u64::from(w) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "part-select range exceeds the declared bounds of the packed array",
                    );
                    return Err(());
                }
                (u64::from(c) + 1 - u64::from(w), u64::from(c))
            }
        };
        let (Ok(lo), Ok(hi)) = (u32::try_from(lo64), u32::try_from(hi64)) else {
            self.error(
                MsgCode::ElabUnsupported,
                "part-select range exceeds the declared bounds of the packed array",
            );
            return Err(());
        };
        self.packed_inner_range(net, k, prefix, lo, hi)
    }
}
