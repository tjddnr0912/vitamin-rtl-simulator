//! Assignment patterns into an unpacked array: the whole array (or a partial
//! index, `m[i] = '{…}`) and, since §4.5.620 (ROADMAP §5.2 row 28), a constant
//! SLICE of one dimension (`v[1:0] <= '{a, b}`, `m[1][2:1] = '{…}`,
//! `v[1+:2] = '{…}`). Moved out of `arrays.rs` with the slice lane.

use super::*;

/// A constant slice of one unpacked dimension, resolved and range-checked:
/// the window `[lo, lo + size)` of dimension `dim` (= the number of leading
/// indices). `lo` is the slice's LOWEST index, whatever its direction.
pub(crate) struct ArraySlice {
    pub(crate) lo: i64,
    pub(crate) size: u32,
}

impl Elaborator<'_> {
    pub(crate) fn lower_array_assign_pattern(
        &mut self,
        b: &mut ProcessBuilder,
        t_net: u32,
        t_lead: &[&ast::Expr],
        elems: &[ast::Expr],
        delay: Option<&ast::Delay>,
        nonblocking: bool,
    ) -> bool {
        self.lower_array_pattern_window(b, t_net, t_lead, None, elems, delay, nonblocking)
    }

    /// The element-wise expansion behind both the whole-residual pattern and the
    /// slice pattern. `slice` narrows the FIRST residual dimension to its window;
    /// every other dimension is taken whole. Element `k` of the flattened pattern
    /// goes to position `k` of the window in declared left-to-right order, so the
    /// leftmost pattern element lands on the slice's left index (IEEE 1800-2017
    /// §10.10, §7.6: a slice keeps the array's direction, which the caller checks).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn lower_array_pattern_window(
        &mut self,
        b: &mut ProcessBuilder,
        t_net: u32,
        t_lead: &[&ast::Expr],
        slice: Option<ArraySlice>,
        elems: &[ast::Expr],
        delay: Option<&ast::Delay>,
        nonblocking: bool,
    ) -> bool {
        let t_dims = self.net_dim_extents(t_net);
        let mut t_res: Vec<(i64, u32)> = t_dims[t_lead.len()..].to_vec();
        // Word offset of the window's lowest index inside the residual sub-array:
        // `(lo - dim_lo) * stride`, the stride being the product of the inner sizes.
        let mut win_off: u64 = 0;
        if let Some(s) = &slice {
            let stride: u64 = t_res[1..].iter().map(|&(_, n)| n as u64).product();
            win_off = (s.lo - t_res[0].0) as u64 * stride;
            t_res[0] = (s.lo, s.size);
        }
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
        let Some(leaves) = self.flatten_assign_pattern(elems, &t_res) else {
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
        let t_offs = Self::residual_word_offsets(&t_res, &t_desc);
        let t_base = (!t_lead.is_empty())
            .then(|| self.flatten_word(&t_dims, t_lead, &[], IndexDomain::ArrayWord));
        let delay_id = if nonblocking {
            delay.map(|d| self.lower_delay(d).0)
        } else {
            None
        };
        let elem_width = self.nets[t_net as usize].width;
        // A BLOCKING slice write is element-wise, so an element or a leading index
        // that reads anything could see an element already written — the target
        // itself under any of its spellings (a cast, `top.x`, a `let`, `p::x`), or a
        // net a continuous assign drives from it, which vita updates inside the
        // process. Naming those spellings missed one per review round (§4.5.620
        // rounds 1–3), so the rule is the fail-closed one the constant-index
        // recognizer uses: every element and the leading index must lower to a
        // compile-time constant. Nonblocking reads every right-hand side and index
        // before any write, so it needs no rule.
        let pre: Option<Vec<u32>> = if slice.is_some() && !nonblocking {
            // §11.6: each element is in the array element's width context (below).
            let ids: Vec<u32> = leaves
                .iter()
                .map(|e| self.lower_ctx_or_plain(e, elem_width))
                .collect();
            if ids
                .iter()
                .chain(t_base.iter())
                .any(|&id| !self.index_is_compile_time_constant(id))
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a blocking assignment pattern into an unpacked-array slice is \
                     supported when every element and every leading index is a constant \
                     (v1: the element-wise write could let a later element read an \
                     earlier one's new value); use `<=`, or assign the elements one by one",
                );
                return true;
            }
            Some(ids)
        } else {
            None
        };
        let mut kind_checked = false;
        for (k, &t_off) in t_offs.iter().enumerate() {
            // §11.6: each element is in the array element's width context, so a fill
            // literal (`'1`/`'x`/`'z`) grows to the element width (a bare `lower_expr`
            // would size it to 1 bit, then the engine zero-extends it = silent-wrong).
            // `lower_ctx_or_plain` is byte-identical for a non-fill element.
            let rhs_id = match &pre {
                Some(ids) => ids[k],
                None => self.lower_ctx_or_plain(leaves[k], elem_width),
            };
            let off = (t_off as u64 + win_off).min(u32::MAX as u64) as u32;
            let t_word = self.word_expr_at(t_base, off);
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

    /// `v[a:b] = …` / `v[a+:w] <= …` where `v[…]` (after any leading indices) is a
    /// static unpacked array: a SLICE of its next dimension. Returns `false` when
    /// `lhs` is not such a slice (the caller's generic path takes it, unchanged).
    /// Otherwise the statement is consumed: a positional or `'{default: v}` pattern
    /// is expanded element-wise; any other right-hand side is loud, because the
    /// generic path would write the bits of element 0 (`v[1:0] = 2'b11` stored
    /// `v[0] = 3` at exit 0 — iverilog and verilator refuse it).
    pub(crate) fn array_slice_assign(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
        nonblocking: bool,
    ) -> bool {
        let base = match lhs {
            ast::Lvalue::PartSelect { base, .. } | ast::Lvalue::IndexedPart { base, .. } => base,
            _ => return false,
        };
        // An inline task / function formal named like a module array: the view below
        // resolves the ARRAY (round 2: `task t(input logic [7:0] v)` beside a module
        // `v [3:0]` wrote the module's `v[1:0]` from `v[1:0] = '{1'b1, 1'b0}` at exit
        // 0), so the generic path keeps the refusal it had.
        if lval_root_path(lhs).is_some_and(|p| self.bound_by_inline_formal(p)) {
            return false;
        }
        let Some((t_net, t_lead)) = self.lval_array_view(base) else {
            return false;
        };
        // The same bypass rules `array_assign_special` re-runs for the whole array.
        if let Some(path) = lval_root_path(lhs) {
            let path = path.clone();
            self.check_modport_write(&path);
        }
        self.deny_readonly_write(t_net, "assign to");
        let Some(slice) = self.const_array_slice(lhs, t_net, t_lead.len()) else {
            return true; // error emitted
        };
        match &rhs.kind {
            ast::ExprKind::AssignPattern(elems) => self.lower_array_pattern_window(
                b,
                t_net,
                &t_lead,
                Some(slice),
                elems,
                delay,
                nonblocking,
            ),
            ast::ExprKind::AssignPatternKeyed(keyed) => {
                let t_dims = self.net_dim_extents(t_net);
                let mut t_res: Vec<(i64, u32)> = t_dims[t_lead.len()..].to_vec();
                t_res[0] = (slice.lo, slice.size);
                let Some(elems) = self.expand_array_default_pattern(keyed, &t_res) else {
                    return true; // error already emitted
                };
                self.lower_array_pattern_window(
                    b,
                    t_net,
                    &t_lead,
                    Some(slice),
                    &elems,
                    delay,
                    nonblocking,
                )
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "assigning to an unpacked-array slice is supported from an assignment \
                     pattern `'{…}` or `'{default: v}` only (a slice copy or a packed value \
                     is not)",
                );
                true
            }
        }
    }

    /// A slice bound's value when it lowers to ONE constant — a literal, a
    /// parameter, a genvar. An arithmetic bound (`A+1`, `W*2-1`) lowers to an
    /// expression, not a constant, and is refused by name (v1).
    fn const_bound(&mut self, e: &ast::Expr) -> Option<i64> {
        let id = self.lower_index_expr(e);
        self.const_index_value(id)
    }

    /// The window a slice lvalue selects in dimension `dim` of `net`, or `None`
    /// after a loud error. Both bounds (or the `+:` / `-:` base and width) must be
    /// constants, the window must lie inside the dimension, and a range slice must
    /// run in the dimension's declared direction: for `[1:0]` of `logic v[0:1]`
    /// verilator and sv2v assign in declaration order where the slice's left index
    /// says otherwise, and for an ascending slice of a descending array they
    /// disagree, so neither reading is settled.
    fn const_array_slice(&mut self, lhs: &ast::Lvalue, net: u32, dim: usize) -> Option<ArraySlice> {
        let (dlo, dsize) = self.net_dim_extents(net)[dim];
        let desc = self
            .array_dim_desc
            .get(&net)
            .and_then(|v| v.get(dim).copied())
            .unwrap_or(false);
        let (lo, hi) = match lhs {
            ast::Lvalue::PartSelect { msb, lsb, .. } => {
                let (Some(m), Some(l)) = (self.const_bound(msb), self.const_bound(lsb)) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an unpacked-array slice whose bound is not a single constant (a \
                         literal, a parameter or a genvar; v1: an arithmetic bound is not \
                         folded)",
                    );
                    return None;
                };
                if m != l && (m > l) != desc {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an unpacked-array slice whose direction is opposite to the array's \
                         declared range",
                    );
                    return None;
                }
                (m.min(l), m.max(l))
            }
            ast::Lvalue::IndexedPart {
                offset, width, dir, ..
            } => {
                let (Some(o), Some(w)) = (self.const_bound(offset), self.const_bound(width)) else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an unpacked-array slice `[base +: width]` / `[base -: width]` whose \
                         base or width is not a single constant (a literal, a parameter or a \
                         genvar; v1: an arithmetic bound is not folded)",
                    );
                    return None;
                };
                if w < 1 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an unpacked-array slice of width zero or less",
                    );
                    return None;
                }
                match dir {
                    ast::PartDir::PlusColon => (o, o.saturating_add(w - 1)),
                    ast::PartDir::MinusColon => (o.saturating_sub(w - 1), o),
                }
            }
            _ => return None,
        };
        let dhi = dlo + dsize as i64 - 1;
        if lo < dlo || hi > dhi {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "an unpacked-array slice [{lo}..{hi}] outside the dimension's range \
                     [{dlo}..{dhi}]"
                ),
            );
            return None;
        }
        Some(ArraySlice {
            lo,
            size: (hi - lo + 1) as u32,
        })
    }
}
