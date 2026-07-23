//! coverage bins — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// `base \ excl` over closed integer ranges, returned normalized (sorted, disjoint).
/// `excl` is first merged (overlapping/adjacent integer ranges unioned); each base
/// range then has the excluded spans carved out. Used to apply ignore/illegal
/// precedence to a regular bin's declared value set (§19.5.1).
pub(crate) fn subtract_ranges(base: &[(i64, i64)], excl: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut e: Vec<(i64, i64)> = excl.iter().copied().filter(|&(l, h)| l <= h).collect();
    e.sort_unstable();
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (l, h) in e {
        if let Some(last) = merged.last_mut() {
            if l <= last.1.saturating_add(1) {
                last.1 = last.1.max(h);
                continue;
            }
        }
        merged.push((l, h));
    }
    let mut out = Vec::new();
    for &(bl, bh) in base {
        if bl > bh {
            continue;
        }
        let mut cur = bl;
        for &(el, eh) in &merged {
            if eh < cur || el > bh {
                continue;
            }
            if el > cur {
                out.push((cur, el - 1));
            }
            cur = cur.max(eh.saturating_add(1));
            if cur > bh {
                break;
            }
        }
        if cur <= bh {
            out.push((cur, bh));
        }
    }
    out
}

impl Elaborator<'_> {
    // ── N5: functional-coverage synthesis (hand-IEEE; iverilog rejects covergroup) ──
    /// Auto-bin count for a coverpoint: `min(2^W, 64)` where W is the sampled expr's
    /// self-determined width (slice G — precise for nets, binops, selects, concat,
    /// replicate, reductions/comparisons; truly-unknown kinds keep the legacy w=6).
    pub(crate) fn coverpoint_num_bins(&self, e: &ast::Expr) -> u32 {
        let w = self.coverpoint_domain(e).0;
        if w >= 6 {
            64
        } else {
            1u32 << w
        }
    }

    /// `(width, signed)` of a coverpoint's sampled domain — used both for auto-bin
    /// counting and `$`-endpoint clamping. Slice G computes the self-determined
    /// width of an arbitrary expression (mirroring `ir_bits_of`'s rules but on the
    /// AST, pre-lowering); a kind we can't resolve falls back to the legacy
    /// `(6, false)` default so unhandled shapes keep their prior behavior.
    pub(crate) fn coverpoint_domain(&self, e: &ast::Expr) -> (u32, bool) {
        const DEFAULT: (u32, bool) = (6, false);
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.coverpoint_domain(inner),
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => self
                .lookup_net_scoped(&p.segments[0].name)
                .map(|n| (self.nets[n as usize].width, self.nets[n as usize].signed))
                .unwrap_or(DEFAULT),
            ast::ExprKind::Unary { op, operand } => match op {
                // size-preserving unary: width & sign of the operand.
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    self.coverpoint_domain(operand)
                }
                // reductions and logical-not are 1-bit unsigned.
                _ => (1, false),
            },
            ast::ExprKind::Binary { op, lhs, rhs } => match op {
                // arithmetic / bitwise: max operand width, signed iff both signed.
                ast::BinOp::Add
                | ast::BinOp::Sub
                | ast::BinOp::Mul
                | ast::BinOp::Div
                | ast::BinOp::Mod
                | ast::BinOp::Pow
                | ast::BinOp::BitAnd
                | ast::BinOp::BitOr
                | ast::BinOp::BitXor
                | ast::BinOp::BitXnor => {
                    let (lw, ls) = self.coverpoint_domain(lhs);
                    let (rw, rs) = self.coverpoint_domain(rhs);
                    (lw.max(rw), ls && rs)
                }
                // shifts take the left operand's width & sign.
                ast::BinOp::Shl | ast::BinOp::Shr | ast::BinOp::AShl | ast::BinOp::AShr => {
                    self.coverpoint_domain(lhs)
                }
                // relational / equality / logical → 1-bit unsigned.
                _ => (1, false),
            },
            ast::ExprKind::Ternary { then_e, else_e, .. } => {
                let (tw, ts) = self.coverpoint_domain(then_e);
                let (ew, es) = self.coverpoint_domain(else_e);
                (tw.max(ew), ts && es)
            }
            ast::ExprKind::BitSelect { .. } => (1, false),
            ast::ExprKind::PartSelect { msb, lsb, .. } => {
                match (self.const_eval_in_scope(msb), self.const_eval_in_scope(lsb)) {
                    (Some(m), Some(l)) => ((m - l).unsigned_abs() as u32 + 1, false),
                    _ => DEFAULT,
                }
            }
            ast::ExprKind::IndexedPart { width, .. } => match self.const_eval_in_scope(width) {
                Some(w) if w >= 1 => (w as u32, false),
                _ => DEFAULT,
            },
            ast::ExprKind::Concat { parts } => {
                let mut sum = 0u32;
                for p in parts {
                    sum = sum.saturating_add(self.coverpoint_domain(p).0);
                }
                (sum.max(1), false)
            }
            ast::ExprKind::Replicate { count, value } => {
                let Some(c) = self.const_eval_in_scope(count) else {
                    return DEFAULT;
                };
                let mut sum = 0u32;
                for p in value {
                    sum = sum.saturating_add(self.coverpoint_domain(p).0);
                }
                (sum.saturating_mul(c.max(0) as u32).max(1), false)
            }
            // A user-function-call coverpoint takes the declared return width/sign
            // (looked up in the func table) — else the default. (`ir_bits_of` can't
            // help here: it returns None for a Call.)
            ast::ExprKind::Call { name, .. } if name.segments.len() == 1 => {
                match self.func_table.get(&name.segments[0].name) {
                    Some(f) => match &f.range {
                        Some(r) => match (
                            self.const_eval_in_scope(&r.msb),
                            self.const_eval_in_scope(&r.lsb),
                        ) {
                            (Some(m), Some(l)) => ((m - l).unsigned_abs() as u32 + 1, f.signed),
                            _ => DEFAULT,
                        },
                        None => match f.ret_type {
                            ast::ParamType::Integer => (32, true),
                            ast::ParamType::Time => (64, false),
                            _ => (1, f.signed), // implicit scalar / real
                        },
                    },
                    None => DEFAULT,
                }
            }
            _ => DEFAULT,
        }
    }

    /// Fold a bin's value set (`{0,[2:4],$}`) to closed integer ranges; `None` if any
    /// endpoint is non-constant (the bin is then loud-rejected by the caller).
    pub(crate) fn resolve_bin_ranges(
        &self,
        bin: &ast::BinSpec,
        w: u32,
        signed: bool,
    ) -> Option<Vec<(i64, i64)>> {
        let mut ranges = Vec::new();
        for r in &bin.values {
            let lo = self.resolve_range_end(&r.lo, w, signed, false)?;
            let hi = self.resolve_range_end(&r.hi, w, signed, true)?;
            ranges.push((lo, hi));
        }
        Some(ranges)
    }

    /// Resolve a coverpoint's explicit `{ bin* }` body into `(ResolvedBin*, num_bins)`.
    /// Precedence (illegal > ignore > regular, §19.5.1) is realized by SUBTRACTING the
    /// `ignore ∪ illegal` value set from each regular bin's declared set: a regular bin
    /// counts only if its EFFECTIVE set is non-empty, and credits only effective
    /// values. Regular bins become counting bitmap bits (`[]` arrays expand one bit per
    /// effective value); the illegal set is kept (no bit) to fire `$error`; `default`
    /// is excluded (§19.5.1). Unsupported forms (`iff`, fixed `[N]`, non-const value,
    /// >64 bins) are loud-rejected.
    pub(crate) fn resolve_explicit_bins(
        &mut self,
        cp: &ast::Coverpoint,
    ) -> (Vec<ResolvedBin>, u32) {
        let (w, signed) = self.coverpoint_domain(&cp.expr);
        // Pass 1: gather ignore ∪ illegal (the excluded set) and the illegal set.
        // A guard on ignore/illegal is loud-rejected: the precedence subtraction is
        // STATIC (resolve-time), so a runtime-guarded exclusion can't be modeled here.
        let mut excluded: Vec<(i64, i64)> = Vec::new();
        let mut illegal: Vec<(i64, i64)> = Vec::new();
        for bin in &cp.bins {
            if matches!(bin.kind, ast::BinKind::Ignore | ast::BinKind::Illegal) {
                if bin.iff.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "`iff` guard on ignore_bins/illegal_bins (follow-on)",
                    );
                }
                match self.resolve_bin_ranges(bin, w, signed) {
                    Some(rs) => {
                        excluded.extend(rs.iter().copied());
                        if matches!(bin.kind, ast::BinKind::Illegal) {
                            illegal.extend(rs);
                        }
                    }
                    None => self.error(
                        MsgCode::ElabUnsupported,
                        "non-constant coverage bin value (follow-on)",
                    ),
                }
            }
        }
        let mut resolved: Vec<ResolvedBin> = Vec::new();
        if !illegal.is_empty() {
            // No bit — its only role is the runtime `$error` gate.
            resolved.push(ResolvedBin {
                kind: ast::BinKind::Illegal,
                ranges: illegal,
                bit: None,
                iff: None,
                counter: None,
            });
        }
        // Pass 2: regular bins, with the excluded set subtracted from each.
        let mut next_bit: u32 = 0;
        let mut capped = false;
        for bin in &cp.bins {
            if let ast::BinArray::Fixed(_) = bin.array {
                self.error(
                    MsgCode::ElabUnsupported,
                    "fixed-size bin array `[N]` (follow-on)",
                );
                continue;
            }
            if bin.is_default {
                continue; // `default` bins are EXCLUDED from coverage (§19.5.1).
            }
            if !matches!(bin.kind, ast::BinKind::Regular) {
                continue; // ignore/illegal handled in pass 1.
            }
            let Some(declared) = self.resolve_bin_ranges(bin, w, signed) else {
                self.error(
                    MsgCode::ElabUnsupported,
                    "non-constant coverage bin value (follow-on)",
                );
                continue;
            };
            let effective = subtract_ranges(&declared, &excluded);
            if effective.is_empty() {
                continue; // every declared value is ignored/illegal → does not count.
            }
            match bin.array {
                ast::BinArray::Unsized => {
                    let count: i128 = effective
                        .iter()
                        .map(|&(l, h)| h as i128 - l as i128 + 1)
                        .sum();
                    if count > 64 {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "array bin `[]` exceeds the 64-bin bitmap",
                        );
                        continue;
                    }
                    for &(l, h) in &effective {
                        let mut v = l;
                        while v <= h {
                            if next_bit >= 64 {
                                capped = true;
                                break;
                            }
                            resolved.push(ResolvedBin {
                                kind: ast::BinKind::Regular,
                                ranges: vec![(v, v)],
                                bit: Some(next_bit),
                                iff: bin.iff.clone(),
                                counter: None,
                            });
                            next_bit += 1;
                            v += 1;
                        }
                    }
                }
                _ => {
                    if next_bit >= 64 {
                        capped = true;
                        continue;
                    }
                    resolved.push(ResolvedBin {
                        kind: ast::BinKind::Regular,
                        ranges: effective,
                        bit: Some(next_bit),
                        iff: bin.iff.clone(),
                        counter: None,
                    });
                    next_bit += 1;
                }
            }
        }
        if capped {
            self.error(
                MsgCode::ElabUnsupported,
                "coverpoint has more than 64 explicit bins (64-bit bitmap cap)",
            );
        }
        let num_bins = resolved.iter().filter(|b| b.bit.is_some()).count() as u32;
        (resolved, num_bins)
    }

    /// Resolve each `cross cp_a, cp_b;` into a [`CrossTracker`]: look up the named
    /// constituent coverpoints, collect their COUNTING-bin match data (auto-bin
    /// coverpoints expand to one (i,i) bin per auto-bin), and allocate a product
    /// hit-bitmap. Loud-rejects: unknown coverpoint name, `iff`-guarded constituent,
    /// or a product exceeding the 64-bit bitmap. A constituent with 0 counting bins
    /// yields a 0-bin cross (silently dropped — nothing to cover).
    pub(crate) fn resolve_crosses(
        &mut self,
        specs: &[ast::CrossSpec],
        trackers: &[CoverpointTracker],
    ) -> Vec<CrossTracker> {
        if specs.is_empty() {
            return Vec::new();
        }
        let mut by_name: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for (i, t) in trackers.iter().enumerate() {
            if let Some(n) = &t.name {
                by_name.insert(n.as_str(), i);
            }
        }
        let mut out = Vec::new();
        for cr in specs {
            let mut pts: Vec<CrossPoint> = Vec::new();
            let mut product: u64 = 1;
            let mut ok = true;
            for pn in &cr.points {
                let Some(&ti) = by_name.get(pn.name.as_str()) else {
                    self.error(
                        MsgCode::ElabUnresolvedName,
                        &format!("cross references unknown coverpoint `{}`", pn.name),
                    );
                    ok = false;
                    break;
                };
                let t = &trackers[ti];
                if t.cp_iff.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "cross of an `iff`-guarded coverpoint (follow-on)",
                    );
                    ok = false;
                    break;
                }
                let bins: Vec<Vec<(i64, i64)>> = if t.has_explicit {
                    t.bins
                        .iter()
                        .filter(|b| b.bit.is_some())
                        .map(|b| b.ranges.clone())
                        .collect()
                } else {
                    // auto-bins: bin i matches the single value i.
                    (0..t.num_bins as i64).map(|i| vec![(i, i)]).collect()
                };
                product = product.saturating_mul(bins.len() as u64);
                pts.push((t.expr.clone(), bins));
            }
            if !ok || product == 0 {
                continue;
            }
            if product > 64 {
                self.error(
                    MsgCode::ElabUnsupported,
                    "cross product exceeds the 64-bin bitmap",
                );
                continue;
            }
            let bitmap = self.fresh_sva_reg(64, "covx");
            out.push(CrossTracker {
                bitmap,
                num_bins: product as u32,
                points: pts,
            });
        }
        out
    }
}
