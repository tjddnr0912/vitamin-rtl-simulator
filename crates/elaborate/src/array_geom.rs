//! array geometry / flatten_word — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Which geometry an index is being lowered for.
///
/// `flatten_word`/`flatten_word_eids` serve BOTH — the note at their loop says
/// so — and the two want different answers for an index whose value does not
/// fit thirty-two bits: an unpacked array WORD index is read as a 32-bit
/// integer by iverilog 13 AND verilator 5.050 alike (§4.5.310), while the
/// trailing element index of a packed array keeps its true value in iverilog
/// and is therefore dropped. Deriving this from `ascending.is_empty()` would
/// work today and is exactly the kind of proxy that false-rejects later, so the
/// callers say which one they mean.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexDomain {
    /// Every index in the list is an unpacked array WORD index.
    ArrayWord,
    /// Every index in the list is a packed offset.
    PackedElem,
    /// The first `n` are word axes and the rest are packed offsets.
    ///
    /// A subroutine-local or formal array's extent list is one entry per
    /// unpacked dim PLUS a trailing entry for the ELEMENT'S OWN BIT AXIS
    /// (`array_formal_ext_dims`), so a per-NET domain gets the bit-select wrong:
    /// `lm[0][b]` with a 64-bit `b` took the word truncation and wrote bit 2
    /// where the module-level twin `gm[0][b]` dropped, in one design, at exit 0.
    /// The axis is a property of the POSITION, not of the net.
    WordsThenElem(usize),
}

impl IndexDomain {
    /// The domain of index position `k`.
    fn at(self, k: usize) -> IndexDomain {
        match self {
            IndexDomain::WordsThenElem(n) if k < n => IndexDomain::ArrayWord,
            IndexDomain::WordsThenElem(_) => IndexDomain::PackedElem,
            other => other,
        }
    }
}

impl Elaborator<'_> {
    /// Per-dim bounds-guard conjunct for `idx ∈ [lo, lo+size-1]`, shared by both
    /// `flatten_word` twins so the rule exists once.
    ///
    /// For `lo >= 0` this is the long-standing form — `idx >= lo && idx <= hi`
    /// with UNSIGNED 32-bit constants. ⚠️ It is no longer on the RAW index and
    /// no longer byte-identical: §4.5.308 seals the index at its own width
    /// before this sees it (a `Concat`), because widening it into these 32-bit
    /// bounds re-evaluated a context-determined operator inside it. 14 of 144
    /// corpus designs changed IR bytes as a result; none changed output.
    ///
    /// For `lo < 0` that form would be wrong twice over: an unsigned `hi`/`lo` constant
    /// mis-reads the bound, and making the constants signed instead would make the
    /// comparison depend on whether the user's index expression happens to be signed
    /// (`int i` vs `logic [3:0] i` would disagree). So a negative-`lo` dim is guarded on
    /// the NORMALIZED coordinate — `coord <u size` — which is exact for either
    /// signedness because an out-of-range index wraps the 32-bit subtraction to a huge
    /// unsigned value. Only dims that were broken before take this path.
    fn dim_guard(&mut self, i_eid: u32, coord_eid: u32, lo: i64, size: u32) -> u32 {
        if lo < 0 {
            let size_c = self.const_u32_expr(size, 32);
            return self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Lt,
                lhs: coord_eid,
                rhs: size_c,
            });
        }
        let lo_c = self.const_u32_expr(lo as u32, 32);
        let hi_c = self.const_u32_expr((lo as u32).saturating_add(size - 1), 32);
        let ge = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Ge,
            lhs: i_eid,
            rhs: lo_c,
        });
        let le = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Le,
            lhs: i_eid,
            rhs: hi_c,
        });
        self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::LogAnd,
            lhs: ge,
            rhs: le,
        })
    }

    /// Per-dim 0-based coordinate for a source index, shared by both `flatten_word`
    /// twins. `lo == 0` emits NOTHING (the byte-identical common case), `lo > 0` emits
    /// `idx - lo`, `lo < 0` emits `idx + (-lo)` — an `Add` rather than a `Sub` by a
    /// wrapped unsigned constant, so the arithmetic is right by construction instead of
    /// by relying on 32-bit wrap. `ascending` (packed little-endian `[lo:hi]`) maps to
    /// `hi - idx` and never carries a negative `lo` (packed bounds clamp elsewhere).
    fn dim_coord(&mut self, i_eid: u32, lo: i64, size: u32, ascending: bool) -> u32 {
        if ascending {
            // `ascending` is a PACKED-dim convention and `packed_extents` clamps a packed
            // bound that folds negative, so `lo >= 0` on every reachable path. Assert
            // rather than clamp silently: a future caller handing this an unpacked
            // negative-`lo` dim should fail, not get a quietly wrong coordinate.
            debug_assert!(lo >= 0, "ascending dim with a negative low bound: {lo}");
            let hi_c = self.const_u32_expr((lo.max(0) as u32).saturating_add(size - 1), 32);
            return self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: hi_c,
                rhs: i_eid,
            });
        }
        match lo.cmp(&0) {
            std::cmp::Ordering::Equal => i_eid,
            std::cmp::Ordering::Greater => {
                let lo_c = self.const_u32_expr(lo as u32, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Sub,
                    lhs: i_eid,
                    rhs: lo_c,
                })
            }
            std::cmp::Ordering::Less => {
                let add_c = self.const_u32_expr(lo.unsigned_abs().min(u32::MAX as u64) as u32, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: i_eid,
                    rhs: add_c,
                })
            }
        }
    }

    /// PRE-LOWERED-eid twin of [`Self::flatten_word`]: identical row-major offset
    /// arithmetic and per-dim bounds guards (`d ≥ 2` → `idx∈[lo,hi]` conjunction, OOB
    /// sentinel `0x8000_0000`), but the per-index `i_eid` comes from `idx_eids` (lowered
    /// at the read site) instead of `lower_expr`. Kept SEPARATE from `flatten_word` so
    /// the local lowering path's push order — and thus every existing design's golden
    /// IR — is byte-for-byte untouched; this runs ONLY in the hierarchical fixup, on
    /// fresh exprs appended after all designs are lowered.
    pub(crate) fn flatten_word_eids(
        &mut self,
        extents: &[(i64, u32)],
        idx_eids: &[u32],
        ascending: &[bool],
        domain: IndexDomain,
    ) -> u32 {
        let d = extents.len();
        let mut strides = vec![1u64; d];
        for k in (0..d.saturating_sub(1)).rev() {
            strides[k] = strides[k + 1].saturating_mul(extents[k + 1].1 as u64);
        }
        let guard_dims = d >= 2;
        let mut valid: Option<u32> = None;
        let mut acc: Option<u32> = None;
        for (k, &i_eid) in idx_eids.iter().enumerate() {
            let (lo, size) = extents[k];
            // SEAL — see `seal_index_unsigned`. This funnel serves the
            // HIERARCHICAL sites (`u.mem[~i]`); the AST twin below serves the
            // local ones. A patch that reached only the twin left the two
            // spellings of one geometry disagreeing about `mem[~i]` — local
            // right, hierarchical wrong — which the soundness review measured.
            let i_eid = self.seal_index_unsigned(i_eid, domain.at(k));
            let asc = ascending.get(k).copied().unwrap_or(false);
            // A negative-`lo` dim guards on the coordinate, so it must be built first;
            // for every other dim the guard comes first, exactly as before (push order
            // is the golden IR).
            let coord_first = lo < 0;
            let mut coord = coord_first.then(|| self.dim_coord(i_eid, lo, size, asc));
            if guard_dims {
                let both = self.dim_guard(i_eid, coord.unwrap_or(i_eid), lo, size);
                valid = Some(match valid {
                    None => both,
                    Some(v) => self.push_expr(ir::Expr::Binary {
                        op: ir::BinOp::LogAnd,
                        lhs: v,
                        rhs: both,
                    }),
                });
            }
            let coord = match coord.take() {
                Some(c) => c,
                None => self.dim_coord(i_eid, lo, size, asc),
            };
            let term = if strides[k] == 1 {
                coord
            } else {
                let s = self.const_u32_expr(strides[k].min(u32::MAX as u64) as u32, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Mul,
                    lhs: coord,
                    rhs: s,
                })
            };
            acc = Some(match acc {
                None => term,
                Some(a) => self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: a,
                    rhs: term,
                }),
            });
        }
        let flat = acc.unwrap_or_else(|| self.const_u32_expr(0, 32));
        match valid {
            None => flat,
            Some(cond) => {
                let oob = self.const_u32_expr(0x8000_0000, 32);
                self.push_expr(ir::Expr::Ternary {
                    cond,
                    then_e: flat,
                    else_e: oob,
                })
            }
        }
    }

    /// N6: (min, max) index bounds of a SINGLE FIXED unpacked dim (`[N]` → (0, N-1);
    /// `[hi:lo]` / `[lo:hi]` → (min, max)), or None for a dynamic/queue/assoc/non-const
    /// dim (a dynamic string array is a deeper follow-on → loud).
    pub(crate) fn fixed_dim_bounds(&mut self, dim: &ast::Dim) -> Option<(i64, i64)> {
        match dim {
            ast::Dim::Size(e) => {
                let n = self.const_eval_in_scope(e)?;
                (n >= 1).then_some((0, n - 1))
            }
            ast::Dim::Range(r) => {
                let m = self.const_eval_in_scope(&r.msb)?;
                let l = self.const_eval_in_scope(&r.lsb)?;
                Some((m.min(l), m.max(l)))
            }
            _ => None,
        }
    }

    /// True if `base` names an unpacked-array net (bare or `pkg::`-scoped) — the
    /// select on it is an array-element read. A scalar/packed net is not one.
    pub(crate) fn base_is_array_net(&self, base: &ast::Expr) -> bool {
        match &base.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => self
                .lookup_net_scoped(&p.segments[0].name)
                .is_some_and(|n| self.net_is_static_array(n)),
            ast::ExprKind::PkgScoped { pkg, name } => self
                .pkg_vars
                .get(&pkg.name)
                .and_then(|m| m.get(&name.name))
                .is_some_and(|&n| self.net_is_static_array(n)),
            _ => false,
        }
    }

    /// Per-dimension `(lo, size)` extents of an unpacked-array declaration (source
    /// order). `lo` is the lower (minimum) range endpoint — the value subtracted
    /// from a source index to get a 0-based word slot (`mem[4:7]` → lo 4). `size`
    /// is the param/genvar-aware folded length (`abs_diff+1`, widened to `u64` then
    /// clamped to `u32` to avoid the [`Self::range_to_dims`] panic), floored at 1 so
    /// a degenerate `[0:0]` dim contributes one word. The product of the sizes is
    /// the flat `array_len`.
    ///
    /// `lo` is `i64`, NOT `u32`: a declared low bound may be NEGATIVE (`int a[-1:1]`,
    /// IEEE §7.4.2). Clamping it to 0 used to shrink the dim (`[-1:1]` became lo 0
    /// size 2), so the array lost a word and `foreach` yielded two iterations instead
    /// of three — silently, at exit 0. The size below is computed from the UNCLAMPED
    /// bounds for the same reason.
    pub(crate) fn array_dim_extents(&mut self, dims: &[ast::Dim]) -> Vec<(i64, u32)> {
        let mut out = Vec::with_capacity(dims.len());
        for d in dims {
            out.push(match d {
                ast::Dim::Range(r) => {
                    let msb_v = self.const_eval_in_scope(&r.msb);
                    let lsb_v = self.const_eval_in_scope(&r.lsb);
                    // P0-NCW: net/hierarchical-referenced (non-constant) unpacked
                    // bound is loud, NOT a silent length-1 dim.
                    self.check_const_range_bound(&r.msb, msb_v);
                    self.check_const_range_bound(&r.lsb, lsb_v);
                    let msb = msb_v.unwrap_or(0);
                    let lsb = lsb_v.unwrap_or(0);
                    let size = ((msb.abs_diff(lsb)) + 1).min(u32::MAX as u64) as u32;
                    (msb.min(lsb), size.max(1))
                }
                ast::Dim::Size(e) => {
                    let v = self.const_eval_in_scope(e);
                    self.check_const_range_bound(e, v);
                    (
                        0,
                        v.map_or(1, |v| {
                            u32::try_from(v).unwrap_or(if v < 0 { 1 } else { u32::MAX })
                        })
                        .max(1),
                    )
                }
                // v5 ⑥: dyn dims never reach the static-extent path
                // (`elaborate_netvar_decl` routes them to handle nets first) —
                // neutral extent, defensive only.
                ast::Dim::Dyn | ast::Dim::Queue(_) | ast::Dim::Assoc(_) => (0, 1),
            });
        }
        out
    }

    /// The DECLARED low bound of `range` when it folds NEGATIVE (`logic [3:-2]` → -2),
    /// else `None`. Recorded per net by the declaration site so bit/part selects can
    /// normalize against the real bound; see [`Self::range_to_dims_opt`].
    pub(crate) fn declared_neg_lsb(&mut self, range: Option<&ast::Range>) -> Option<i64> {
        let r = range?;
        let m = self.const_eval_in_scope(&r.msb)?;
        let l = self.const_eval_in_scope(&r.lsb)?;
        // ONLY the `[hi:neg]` shape. `msb < 0` with `lsb >= 0` is the degenerate
        // `[W-1:0]`-with-W==0 parameter underflow, which keeps its graceful width-1.
        (l < 0 && m >= l).then_some(l)
    }

    pub(crate) fn range_to_dims(
        &mut self,
        kind: ast::NetVarKind,
        range: Option<&ast::Range>,
        signed: bool,
    ) -> (u32, u32, u32, bool) {
        self.range_to_dims_opt(kind, range, signed, false)
    }

    /// [`Self::range_to_dims`] with an OPT-IN for a negative declared low bound.
    ///
    /// `allow_neg_lsb` is `false` at every call site that has not wired up the
    /// companion `declared_neg_lsb` side-map record, and a literal `false` short-circuits
    /// to the long-standing warn-and-clamp — so those sites are byte-identical and stay
    /// LOUD rather than silently receiving a wide net whose selects nobody normalizes.
    /// That is the whole reason this is a parameter instead of a behaviour change: the
    /// width and the select normalization have to be turned on together, and the sites
    /// that create nets are not all in one place.
    pub(crate) fn range_to_dims_opt(
        &mut self,
        kind: ast::NetVarKind,
        range: Option<&ast::Range>,
        signed: bool,
        allow_neg_lsb: bool,
    ) -> (u32, u32, u32, bool) {
        if matches!(kind, ast::NetVarKind::Integer) {
            // `integer` defaults signed; honor an explicit `unsigned` (the parser
            // resolves the default + qualifier into `signed`).
            return (32, 31, 0, signed);
        }
        // 2-state integer atom types (fixed width, IEEE §6.11.1): signed by default,
        // but `int unsigned` / `byte unsigned` / … must stay UNSIGNED. The parser
        // resolves the kind default + signing qualifier into `signed`, so honor it
        // here rather than hardcoding signed=true (which silently mis-signed every
        // `… unsigned` atom in comparisons / `%0d`).
        match kind {
            ast::NetVarKind::Byte => return (8, 7, 0, signed),
            ast::NetVarKind::Shortint => return (16, 15, 0, signed),
            ast::NetVarKind::Int => return (32, 31, 0, signed),
            ast::NetVarKind::Longint => return (64, 63, 0, signed),
            _ => {}
        }
        // `real`/`realtime` are dimensionless 64-bit signed (no [msb:lsb] range).
        if matches!(kind, ast::NetVarKind::Real | ast::NetVarKind::Realtime) {
            return (64, 63, 0, true);
        }
        // `time` is a dimensionless 64-bit UNSIGNED 4-state variable (IEEE §6.11).
        if matches!(kind, ast::NetVarKind::Time) {
            return (64, 63, 0, false);
        }
        // a named event is dimensionless; its counter desugar is 64-bit unsigned.
        if matches!(kind, ast::NetVarKind::Event) {
            return (64, 63, 0, false);
        }
        // N7: a class handle is a dimensionless 32-bit unsigned object-id.
        if matches!(kind, ast::NetVarKind::ClassHandle) {
            return (32, 31, 0, false);
        }
        match range {
            None => (1, 0, 0, signed),
            Some(r) => {
                // v3: fold through the param-aware evaluator so `[W-1:0]` resolves
                // `W` to the bound parameter value in the current instance scope.
                let msb_v = self.const_eval_in_scope(&r.msb);
                let lsb_v = self.const_eval_in_scope(&r.lsb);
                // P0-NCW: a net/hierarchical-referenced (non-constant) bound is loud,
                // NOT a silent width-1.
                self.check_const_range_bound(&r.msb, msb_v);
                self.check_const_range_bound(&r.lsb, lsb_v);
                // A bound that folds NEGATIVE clamps to width 1 + warn (NOT a fatal
                // MAX_NET_WIDTH explosion). Two shapes fold negative:
                //  - only msb < 0, lsb ≥ 0 → the degenerate `[W-1:0]`-with-W==0
                //    PARAMETER underflow (v3_12 keeps this graceful, non-fatal).
                //  - lsb < 0 → a negative LOW bound (`[3:-2]`, `[-1:-8]`, or a
                //    param low bound that folded negative like `[7:W-3]` W==1).
                //    iverilog sizes it |msb-lsb|+1; vita cannot yet (the u32 dbase/
                //    offset and frozen `NetVar.msb/lsb` cannot hold a negative base).
                //    Loud→supported gap (ROADMAP §3) — the packed-struct-member path
                //    already does whole-correct + sub-select-loud (struct_field_
                //    select.rs); the plain-net path should mirror it.
                // The old message misdiagnosed the literal case as "parameterized";
                // keep the diagnostic honest for both shapes.
                if lsb_v.is_some_and(|v| v < 0) {
                    // OPT-IN (see `allow_neg_lsb`): size the net `|msb-lsb|+1` as IEEE
                    // §7.4.2 / iverilog do and store the range NORMALIZED to `[w-1:0]`
                    // (`NetVar.msb`/`lsb` are frozen `u32` and cannot hold -2). The real
                    // low bound rides `net_decl_neg_lsb`, which `norm_offset_for_net`
                    // consults so `x[3]` on a `[3:-2]` addresses internal bit 5.
                    if allow_neg_lsb {
                        if let (Some(m), Some(l)) = (msb_v, lsb_v) {
                            if m >= l {
                                let w = ((m - l) as u64 + 1).min(MAX_NET_WIDTH) as u32;
                                return (w, w.saturating_sub(1), 0, signed);
                            }
                        }
                    }
                    self.warn(
                        "negative packed-range low bound (e.g. `[3:-2]`) is not yet supported; net clamped to width 1",
                    );
                    return (1, 0, 0, signed);
                }
                if msb_v.is_some_and(|v| v < 0) {
                    self.warn(
                        "parameterized range underflowed (param value 0?); net clamped to width 1",
                    );
                    return (1, 0, 0, signed);
                }
                let clamp =
                    |v: Option<i64>| v.map_or(0u32, |v| u32::try_from(v).unwrap_or(u32::MAX));
                let msb = clamp(msb_v);
                let lsb = clamp(lsb_v);
                let width64 = (msb.abs_diff(lsb) as u64) + 1;
                if width64 > MAX_NET_WIDTH {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "declared net width {width64} exceeds the v1 cap ({MAX_NET_WIDTH})"
                        ),
                    );
                    return (1, 0, 0, signed);
                }
                (width64 as u32, msb, lsb, signed)
            }
        }
    }

    /// Resolve a part-select base expression to its ROOT single-segment net,
    /// peeling `(…)` and array `name[i][j]…` indexing — so an array element
    /// (`m[2]`) resolves to the array net `m` (whose packed range is the element
    /// shape). A computed/concat/field/hierarchical base yields `None`.
    pub(crate) fn base_root_net(&self, base: &ast::Expr) -> Option<u32> {
        let mut cur = base;
        loop {
            match &cur.kind {
                ast::ExprKind::Paren { inner } => cur = inner,
                ast::ExprKind::BitSelect { base, .. } => cur = base,
                ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                    return self.lookup_net_scoped(&path.segments[0].name);
                }
                // Interface-member alias (`bi.data`) — a known dotted symbol; its
                // declared range drives the ascending check (a hierarchical ref
                // resolves to `None` here and stays on the descending-default path).
                ast::ExprKind::Ident(path) => return self.iface_member_net(path),
                // Explicit `pkg::vec[m:l]` / `pkg::arr[i][m:l]` — the root is the
                // package net (its declared range drives the ascending check).
                ast::ExprKind::PkgScoped { .. } => return self.pkg_scoped_var_net(cur),
                _ => return None,
            }
        }
    }

    /// Resolve a WHOLE select expression (`name[i]` / `name[m:l]` / `name[b+:w]`,
    /// nested + parenthesized) to its root single-segment net. Unlike
    /// [`Self::base_root_net`] this also peels the select operators themselves, so
    /// it is used on a task ARGUMENT actual (the whole `tmp[3:0]`, not its base).
    pub(crate) fn actual_root_net(&self, a: &ast::Expr) -> Option<u32> {
        let mut cur = a;
        loop {
            match &cur.kind {
                ast::ExprKind::Paren { inner } => cur = inner,
                ast::ExprKind::BitSelect { base, .. }
                | ast::ExprKind::PartSelect { base, .. }
                | ast::ExprKind::IndexedPart { base, .. } => cur = base,
                ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                    return self.lookup_net_scoped(&path.segments[0].name);
                }
                _ => return None,
            }
        }
    }

    /// Per-dim `(lo, size)` extents of array `net` (source order). Arrays with
    /// non-0-based or multi-dim addressing record their shape in `array_dims`; a
    /// plain 0-based 1-D array falls back to a single `(0, array_len)` extent.
    pub(crate) fn net_dim_extents(&self, net: u32) -> Vec<(i64, u32)> {
        self.array_dims
            .get(&net)
            .cloned()
            .unwrap_or_else(|| vec![(0, self.nets[net as usize].array_len.max(1))])
    }

    /// Row-major flat word ExprId for the first `extents.len()` indices. Each index
    /// is normalized to a 0-based slot by subtracting its dim's `lo` (`mem[4]` on a
    /// `[4:7]` dim → `i-4`); strides are the suffix products of the dim sizes. A
    /// `lo` of 0 emits NO `Sub` and a stride of 1 emits NO `Mul`, so a plain 0-based
    /// 1-D `mem[i]` still lowers to exactly `lower_expr(i)` — the golden IR for the
    /// common case is byte-for-byte preserved.
    ///
    /// `ascending[k]` (per packed dim; empty/short ⇒ all `false` = descending, the
    /// unpacked-array + descending-packed byte-identical path) flips dim `k` to
    /// `coord = hi - idx` (N3.3, little-endian `[lo:hi]`).
    pub(crate) fn flatten_word(
        &mut self,
        extents: &[(i64, u32)],
        word_idxs: &[&ast::Expr],
        ascending: &[bool],
        domain: IndexDomain,
    ) -> u32 {
        let d = extents.len();
        // strides[k] = product(size[k+1..]) as u64 (saturating into u32 at use).
        let mut strides = vec![1u64; d];
        for k in (0..d.saturating_sub(1)).rev() {
            strides[k] = strides[k + 1].saturating_mul(extents[k + 1].1 as u64);
        }
        // Phase-1.x ③: per-dim bounds guards, MULTI-dim only. A 1-D access is
        // already exact (lo-normalization wraps an under-index to a huge word
        // that the engine's flat check rejects) and stays byte-identically
        // guard-free; with d ≥ 2 an inner-dim violation lands INSIDE the flat
        // space — a silent alias into the neighbouring row (iverilog 13.0
        // shares that alias, so the X behavior is hand-pinned to IEEE §7.4.6).
        let guard_dims = d >= 2;
        let mut valid: Option<u32> = None;
        let mut acc: Option<u32> = None;
        for (k, idx) in word_idxs.iter().enumerate() {
            let (lo, size) = extents[k];
            // r19/S2: the array WORD index and the multi-dim-packed trailing
            // element index both funnel here. Gating only the flat-vector
            // bit-select left `p[0][R]` reading the wrong nibble at exit 0.
            let i_eid = self.lower_index_expr(idx);
            // SEAL — the AST twin of the eid-taking `flatten_word_eids` above;
            // see the note there. Both funnels must do it or the two spellings
            // of one geometry disagree about `mem[~i]`.
            let i_eid = self.seal_index_unsigned(i_eid, domain.at(k));
            let asc = ascending.get(k).copied().unwrap_or(false);
            // `idx >= lo && idx <= hi` on the RAW index. A negative or wrapped index
            // always fails one side in either signedness reading (bounds < 2^24 «
            // 2^31); an X index X-poisons the conjunction and the final ternary merge
            // below. A NEGATIVE-`lo` dim guards on the coordinate instead, so its
            // coordinate must be built first (see `dim_guard`); every other dim keeps
            // the original guard-then-coordinate push order = the golden IR.
            let coord_first = lo < 0;
            let mut coord = coord_first.then(|| self.dim_coord(i_eid, lo, size, asc));
            if guard_dims {
                let both = self.dim_guard(i_eid, coord.unwrap_or(i_eid), lo, size);
                valid = Some(match valid {
                    None => both,
                    Some(v) => self.push_expr(ir::Expr::Binary {
                        op: ir::BinOp::LogAnd,
                        lhs: v,
                        rhs: both,
                    }),
                });
            }
            // normalized 0-based coordinate. Descending (or unpacked): `idx - lo`
            // (lo==0 ⇒ raw index, no Sub; lo<0 ⇒ `idx + |lo|`). Ascending `[lo:hi]`:
            // `hi - idx` where hi = lo+size-1 is the lsb endpoint — internal bit 0
            // (mirrors `norm_offset_for_net`).
            let coord = match coord.take() {
                Some(c) => c,
                None => self.dim_coord(i_eid, lo, size, asc),
            };
            let term = if strides[k] == 1 {
                coord
            } else {
                let s = self.const_u32_expr(strides[k].min(u32::MAX as u64) as u32, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Mul,
                    lhs: coord,
                    rhs: s,
                })
            };
            acc = Some(match acc {
                None => term,
                Some(a) => self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: a,
                    rhs: term,
                }),
            });
        }
        let flat = acc.unwrap_or_else(|| self.const_u32_expr(0, 32));
        match valid {
            None => flat,
            Some(cond) => {
                // OOB sentinel 0x8000_0000: far above MAX_ARRAY_LEN (2^24) and
                // ADDITION-STABLE — a downstream `base + residual` (the array-
                // assignment expansion) cannot wrap it back into range. An X
                // condition ternary-merges flat-vs-sentinel into an unknown-
                // contaminated word, which the engine already treats as
                // invalid (read X / write no-op).
                let oob = self.const_u32_expr(0x8000_0000, 32);
                self.push_expr(ir::Expr::Ternary {
                    cond,
                    then_e: flat,
                    else_e: oob,
                })
            }
        }
    }

    /// Fixed-size unpacked array stored in the flat word store (dyn/queue/assoc
    /// handles have `array_len == 0`). `array_len > 1` alone would exempt
    /// 1-element arrays (`reg x [0:0]`, adversarial find #5), so declared
    /// array-ness is tracked explicitly in `unpacked_array_nets`.
    pub(crate) fn net_is_static_array(&self, net: u32) -> bool {
        self.unpacked_array_nets.contains(&net)
            || self
                .nets
                .get(net as usize)
                .is_some_and(|nv| nv.array_len > 1)
    }

    /// Record the SYS-INTRO dimension descriptor for net `id` from its AST shape:
    /// UNPACKED dims (declaration order) THEN packed dims (`range` then extra
    /// `packed` ranges), each as `(left, right)` const-folded endpoints. An
    /// integral ATOM type with no explicit packed range carries an IMPLICIT packed
    /// dim — `integer`⇒`[31:0]`, `time`⇒`[63:0]` (matching iverilog) — while
    /// dimensionless `real`/`realtime`/`event` and a 1-bit scalar get an EMPTY
    /// descriptor (0 dims). Stored for EVERY name so the fold is unambiguous and a
    /// scalar (`reg s`, empty) is distinct from `reg [0:0] x` (one dim).
    pub(crate) fn compute_dim_desc(
        &mut self,
        kind: ast::NetVarKind,
        range: Option<&ast::Range>,
        packed: &[ast::Range],
        unpacked: &[ast::Dim],
    ) -> (Vec<(i64, i64)>, usize) {
        let mut dims: Vec<(i64, i64)> = Vec::new();
        for u in unpacked {
            match u {
                ast::Dim::Range(r) => {
                    if let (Some(m), Some(l)) = (
                        self.const_eval_in_scope(&r.msb),
                        self.const_eval_in_scope(&r.lsb),
                    ) {
                        dims.push((m, l));
                    }
                }
                // `[N]` shorthand = `[0:N-1]` (IEEE 1800 §7.4.2).
                ast::Dim::Size(e) => {
                    if let Some(n) = self.const_eval_in_scope(e) {
                        dims.push((0, (n - 1).max(0)));
                    }
                }
                ast::Dim::Dyn | ast::Dim::Queue(_) | ast::Dim::Assoc(_) => {}
            }
        }
        let unpacked_n = dims.len();
        for r in range.into_iter().chain(packed.iter()) {
            if let (Some(m), Some(l)) = (
                self.const_eval_in_scope(&r.msb),
                self.const_eval_in_scope(&r.lsb),
            ) {
                dims.push((m, l));
            }
        }
        // Implicit packed dim for an integral atom with no explicit packed range.
        if range.is_none() && packed.is_empty() {
            match kind {
                ast::NetVarKind::Integer | ast::NetVarKind::Int => dims.push((31, 0)),
                ast::NetVarKind::Time | ast::NetVarKind::Longint => dims.push((63, 0)),
                ast::NetVarKind::Byte => dims.push((7, 0)),
                ast::NetVarKind::Shortint => dims.push((15, 0)),
                // `bit` with no packed range is a 1-bit scalar (like reg): 0-dim.
                _ => {} // real/realtime/event/bit-scalar/string: dimensionless here
            }
        }
        (dims, unpacked_n)
    }

    /// Store the [`Self::compute_dim_desc`] entry for `id` AND register its `intro_kind`
    /// (the module-scope decl path — the `intro_kind` insert drives $typename + 2-state
    /// coercion). The frame-local packed path inserts the `dim_desc` via
    /// `compute_dim_desc` WITHOUT this side effect, since it gates `intro_kind` on
    /// 2-state kinds itself (a 4-state `logic` packed local must not be coerced).
    pub(crate) fn record_dim_desc(
        &mut self,
        id: u32,
        kind: ast::NetVarKind,
        range: Option<&ast::Range>,
        packed: &[ast::Range],
        unpacked: &[ast::Dim],
    ) {
        let dd = self.compute_dim_desc(kind, range, packed, unpacked);
        self.dim_desc.insert(id, dd);
        self.intro_kind.insert(id, kind);
    }

    /// The SYS-INTRO dimension descriptor for `net`: the decl-time `dim_desc`
    /// entry when present (authoritative — distinguishes a scalar from `[0:0]`),
    /// otherwise derived from `NetVar` (ports/params): a vector contributes one
    /// packed `(msb,lsb)` dim, `array_len>1` prepends an unpacked `(0,len-1)`,
    /// and a width-1 unranged net is a 0-dim scalar.
    pub(crate) fn net_dims_desc(&self, net: u32) -> Option<(Vec<(i64, i64)>, usize)> {
        if let Some(d) = self.dim_desc.get(&net) {
            return Some(d.clone());
        }
        let nv = self.nets.get(net as usize)?;
        let mut dims: Vec<(i64, i64)> = Vec::new();
        let unpacked = if nv.array_len > 1 {
            dims.push((0, nv.array_len as i64 - 1));
            1
        } else {
            0
        };
        if nv.width > 1 || nv.msb != nv.lsb {
            dims.push((nv.msb as i64, nv.lsb as i64));
        }
        Some((dims, unpacked))
    }
}
