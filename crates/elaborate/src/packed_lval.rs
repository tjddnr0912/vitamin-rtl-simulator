//! packed selects (write) — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;
use crate::array_geom::IndexDomain;

impl Elaborator<'_> {
    /// N3.4: a `[msb:lsb]` part-select on a BARE multi-dim PACKED net (e.g.
    /// `reg [3:0][7:0] x; x[3:2]`) addresses the OUTER packed dimension — it
    /// selects whole `elem_w`-bit sub-elements, NOT flat bits. Without this the
    /// generic flat-bit path silently reads `(msb-lsb+1)` raw bits of element 0
    /// (`x[3:2]` => `3`, not `aabb`). Returns `Some(Select)` for the handled
    /// case (a bare net in `packed_dims`, ≥2 dims, const bounds — either a
    /// descending `[msb:lsb]` select on a descending net or an ascending
    /// `[lo:hi]` select on an ascending net, both whole-element ranges); `None`
    /// only for a plain vector or a non-const select. A direction-MISMATCHED
    /// ("out of order") select is loud (`Some(Err)`), matching iverilog.
    pub(crate) fn try_packed_part_select(
        &mut self,
        base: &ast::Expr,
        msb: &ast::Expr,
        lsb: &ast::Expr,
    ) -> Option<u32> {
        let (net, word) = self.packed_ps_base(base)?;
        match self.packed_outer_part_select(net, msb, lsb)? {
            Ok((offset, width)) => {
                let base_id = self.push_expr(ir::Expr::Signal { net, word });
                Some(self.push_expr(ir::Expr::Select {
                    base: base_id,
                    offset,
                    width,
                    kind: ir::SelKind::PartIdxUp,
                }))
            }
            // out-of-range: E3009 already emitted — a loud placeholder (NOT a
            // fall-through to the generic flat-bit path, which would silently
            // read past the net).
            Err(()) => Some(self.placeholder_expr()),
        }
    }

    /// N3.4 follow-on: a constant indexed part-select (`x[c+:w]` / `x[c-:w]`) on a
    /// multi-dim packed net (bare or an array-of-packed element) addresses whole
    /// OUTER elements just like `[msb:lsb]` — `x[2+:2]` ≡ `x[3:2]` (iverilog folds
    /// the const form to a range). A VARIABLE offset (`x[i+:2]`) is loud: iverilog
    /// 13.0 aborts on it, so the bit-vs-element unit is undefined and vita rejects
    /// rather than guess. `None` ⇒ non-packed base ⇒ the generic path (a plain
    /// vector indexed part-select is correct flat-bits).
    pub(crate) fn try_packed_indexed_part(
        &mut self,
        base: &ast::Expr,
        offset: &ast::Expr,
        width: &ast::Expr,
        dir: &ast::PartDir,
    ) -> Option<u32> {
        let (net, word) = self.packed_ps_base(base)?;
        match self.packed_indexed_range(net, offset, width, dir) {
            Ok((off, w)) => {
                let base_id = self.push_expr(ir::Expr::Signal { net, word });
                Some(self.push_expr(ir::Expr::Select {
                    base: base_id,
                    offset: off,
                    width: w,
                    kind: ir::SelKind::PartIdxUp,
                }))
            }
            Err(()) => Some(self.placeholder_expr()),
        }
    }

    /// Write-side twin of [`Self::packed_ps_base`] over `Lvalue` nodes.
    pub(crate) fn packed_ps_base_lval(&mut self, base: &ast::Lvalue) -> Option<(u32, Option<u32>)> {
        match base {
            ast::Lvalue::Ident(path) => self.bare_packed_net(path).map(|n| (n, None)),
            ast::Lvalue::BitSelect { base: b, index, .. } => {
                let (net, idxs) = self.lval_array_chain(b, index)?;
                if self.packed_dims.get(&net).is_none_or(|d| d.len() < 2) {
                    return None;
                }
                let dims = self.net_dim_extents(net);
                if idxs.len() != dims.len() {
                    return None;
                }
                let word = self.flatten_word(&dims, &idxs, &[], IndexDomain::ArrayWord);
                Some((net, Some(word)))
            }
            _ => None,
        }
    }

    /// Write-side twin of [`Self::try_packed_part_select`]: a `[msb:lsb] = …`
    /// part-select on a bare multi-dim PACKED net → one whole-element-range
    /// `LvalChunk` (offset/width in the outer-dim element scale, `PartIdxUp`),
    /// instead of the flat-bit chunk that silently writes `(msb-lsb+1)` bits of
    /// element 0. `None` ⇒ fall through to the generic lvalue path.
    pub(crate) fn try_packed_part_select_lval(
        &mut self,
        base: &ast::Lvalue,
        msb: &ast::Expr,
        lsb: &ast::Expr,
    ) -> Option<ir::LvalChunk> {
        let (net, word) = self.packed_ps_base_lval(base)?;
        match self.packed_outer_part_select(net, msb, lsb)? {
            Ok((offset, width)) => Some(ir::LvalChunk {
                net,
                word,
                offset: Some(offset),
                width: Some(width),
                kind: ir::SelKind::PartIdxUp,
            }),
            // out-of-range: E3009 already emitted — a loud POISON placeholder
            // chunk (so the generic path does not silently write past the net).
            Err(()) => Some(Self::poison_chunk()),
        }
    }

    /// Write-side twin of [`Self::try_packed_indexed_part`]: a constant indexed
    /// part-select LHS (`x[c+:w] = …`, `qm[i][c+:w] = …`) on multi-dim packed →
    /// one whole-element-range chunk; a variable offset is loud (POISON chunk).
    pub(crate) fn try_packed_indexed_part_lval(
        &mut self,
        base: &ast::Lvalue,
        offset: &ast::Expr,
        width: &ast::Expr,
        dir: &ast::PartDir,
    ) -> Option<ir::LvalChunk> {
        let (net, word) = self.packed_ps_base_lval(base)?;
        match self.packed_indexed_range(net, offset, width, dir) {
            Ok((off, w)) => Some(ir::LvalChunk {
                net,
                word,
                offset: Some(off),
                width: Some(w),
                kind: ir::SelKind::PartIdxUp,
            }),
            Err(()) => Some(Self::poison_chunk()),
        }
    }

    /// Write-side twin of [`Self::expr_packed_chain`] (multi-dim PACKED net).
    pub(crate) fn lval_packed_chain<'a>(
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
                // 1-segment local OR multi-segment resolvable hierarchical packed net
                // (same-module generate scope — HIER-REST②). Cross-instance unresolved
                // → None → deferred-sel write lane.
                ast::Lvalue::Ident(p) => {
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
                _ => return None,
            }
        };
        outer_first.reverse();
        outer_first.push(index);
        Some((net, outer_first))
    }

    /// Write `m[i0]…[ik] = …` on a packed multi-dim net into one bit-slice LvalChunk
    /// (indexed part-select), mirroring [`Self::lower_packed_read`].
    pub(crate) fn collect_packed_write(
        &mut self,
        net: u32,
        idxs: &[&ast::Expr],
        out: &mut Vec<ir::LvalChunk>,
    ) {
        let dims = self.packed_dims[&net].clone();
        if idxs.len() > dims.len() {
            self.error(
                MsgCode::ElabUnsupported,
                "too many indices for packed array (more than its dimensions)",
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
        // §4.5.199: a PARTIAL write to an md-packed unpacked-array frame slot (`m[i] = …`
        // on a 2-D `int m[2][2]`) must not silently overwrite a multi-element sub-array —
        // index down to a scalar element (mirror of `lower_packed_read`; byte-identical
        // for a 1-D md-packed slot). Correct-or-loud.
        if idxs.len() + 1 < dims.len() && self.frame_arr_formal_meta.contains_key(&net) {
            self.error(
                MsgCode::ElabUnsupported,
                "a partial slice of an unpacked array (index every dimension down to a \
                 scalar element; a whole sub-array cannot be assigned here)",
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
        // The DOMAIN is the source geometry, not the storage. A subroutine-local
        // or formal UNPACKED array is registered in `packed_dims` (its slot is
        // md-packed), so its word index arrives here — and labelling it
        // `PackedElem` because of where it lives skipped the array-word reading
        // for it: `lm[bg]` inside a function read the wrong element where the
        // module-level twin `gm[bg]` read the right one, and it did so at exit 0
        // where the pre-§4.5.310 build had been loud. `frame_arr_formal_meta` is
        // the same key this function already consults a few lines up.
        let domain = if self.frame_arr_formal_meta.contains_key(&net) {
            IndexDomain::ArrayWord
        } else {
            IndexDomain::PackedElem
        };
        let (ext, dirs) = Self::packed_split(&dims);
        let offset = self.flatten_word(&ext, idxs, &dirs, domain);
        let elem_w: u64 = dims[idxs.len()..]
            .iter()
            .map(|&(_, w, _)| w as u64)
            .product();
        let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
        out.push(ir::LvalChunk {
            net,
            word: None,
            offset: Some(offset),
            width: Some(width),
            kind: ir::SelKind::PartIdxUp,
        });
    }

    /// A `[msb:lsb]` part-select LHS whose base is a NESTED select that lands on the
    /// LEAF packed dimension: `x[j][m:l] = …` (bare `[1:0][7:0]` net) or
    /// `arr[i][j][m:l] = …` (unpacked-array-of-packed). The read side composes an
    /// element-select + packed-flatten; the write side's generic `lval_part_base`
    /// flattens only the unpacked dims and rejected this (E3009). Mirror the read:
    /// the unpacked indices (if any) pick the element `word`, the packed indices pick
    /// the leaf's base bit (`flatten_word` over the packed extents), and `[msb:lsb]`
    /// selects within the leaf. Returns `Some(chunk)` for the handled leaf shape,
    /// `Some(poison)` after a loud out-of-range, and `None` (fall through to the
    /// generic loud path) for any shape it does not provably handle — fail-closed:
    /// only a DESCENDING, zero-LSB leaf with a CONSTANT in-range `[msb:lsb]`, exactly
    /// one leaf dim left after the indices (so `[msb:lsb]` is bit-addressed, not an
    /// outer-element range — that is `try_packed_part_select_lval`).
    pub(crate) fn try_nested_packed_part_lval(
        &mut self,
        base: &ast::Lvalue,
        msb: &ast::Expr,
        lsb: &ast::Expr,
    ) -> Option<ir::LvalChunk> {
        let ast::Lvalue::BitSelect { base: b, index, .. } = base else {
            return None;
        };
        // Resolve the index chain: an unpacked-array-of-packed goes through the array
        // chain (idxs = unpacked… ++ packed…); a bare multi-dim packed net goes through
        // the packed chain (idxs = packed… only, no unpacked dims).
        let (net, all_idxs, ud) = if let Some((n, ix)) = self.lval_array_chain(b, index) {
            let ud = self.net_dim_extents(n).len();
            (n, ix, ud)
        } else if let Some((n, ix)) = self.lval_packed_chain(b, index) {
            (n, ix, 0usize)
        } else {
            return None;
        };
        let pdims = self.packed_dims.get(&net)?.clone();
        // Exactly one leaf packed dim must remain after the packed indices, so
        // `[msb:lsb]` is a bit-select within the leaf. Fewer packed indices ⇒ an
        // outer-element part-select (`try_packed_part_select_lval`); more ⇒ bit-of-bit.
        if pdims.len() < 2 || all_idxs.len() != ud + pdims.len() - 1 {
            return None;
        }
        // iverilog requires the PACKED indices to be constant (a variable packed index
        // "is not allowed in a constant expression"); a variable UNPACKED array index is
        // fine. A variable packed index has no oracle, so fall through to the generic
        // loud path rather than compute an unverifiable offset.
        if all_idxs[ud..]
            .iter()
            .any(|e| self.const_bound_u32(e).is_none())
        {
            return None;
        }
        // Fail-closed: the leaf must be a plain descending zero-LSB vector so `l` is the
        // bit offset within it and `[msb:lsb]` maps to `[base+l, base+m]`.
        let leaf = *pdims.last().unwrap();
        if leaf.2 || leaf.0 != 0 {
            return None;
        }
        let (Some(m), Some(l)) = (self.const_bound_u32(msb), self.const_bound_u32(lsb)) else {
            return None; // variable bounds → generic loud path
        };
        if m < l || m >= leaf.1 {
            self.error(
                MsgCode::ElabUnsupported,
                "part-select range exceeds the packed sub-element width",
            );
            return Some(Self::poison_chunk());
        }
        // Unpacked indices → element word (descending default, mirroring lval_part_base).
        let word = (ud > 0).then(|| {
            let ue = self.net_dim_extents(net);
            self.flatten_word(&ue, &all_idxs[..ud], &[], IndexDomain::ArrayWord)
        });
        // Packed indices → the leaf's base bit (bit offset, exactly as the read path's
        // flatten over the packed extents); then `+ l` for the part-select LSB.
        let (pext, pdirs) = Self::packed_split(&pdims);
        let base_off = self.flatten_word(&pext, &all_idxs[ud..], &pdirs, IndexDomain::PackedElem);
        let offset = if l == 0 {
            base_off
        } else {
            let l_c = self.const_u32_expr(l, 32);
            self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs: base_off,
                rhs: l_c,
            })
        };
        let width = self.const_u32_expr(m - l + 1, 32);
        Some(ir::LvalChunk {
            net,
            word,
            offset: Some(offset),
            width: Some(width),
            kind: ir::SelKind::PartIdxUp,
        })
    }
}
