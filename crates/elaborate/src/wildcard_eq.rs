//! Wildcard equality — §11.4.6 `==?` / `!=?` and §11.4.13's `inside` element
//! comparison. One builder ([`Elaborator::wildcard_cmp_ids`]) serves both, and
//! [`Elaborator::inside_value_cmp`] is the element rule a `case … inside` item
//! (§12.5.4) is to reuse: it works on operands that are ALREADY LOWERED, so a
//! caller decides nothing by lowering an operand twice.

use super::*;

impl Elaborator<'_> {
    /// §11.4.6 `==?`/`!=?` wildcard equality: the RHS PATTERN's x/z bits are
    /// don't-care; every other bit compares like plain `==`/`!=` (an LHS x/z in
    /// a compared position propagates x — UNLIKE `CasexEq`, which wildcards
    /// EITHER side, so mapping there would be a silent-wrong). Lowered as
    /// `(lhs & mask) ==/!= cleaned` with mask/cleaned computed from the CONSTANT
    /// pattern at elaborate time ([`Self::wildcard_cmp_ids`]). The compare
    /// inherits vita's oracle-pinned `Eq`/`Ne` 4-state semantics. A NON-constant
    /// pattern would need a runtime known-bit mask (no frozen-IR primitive exposes
    /// the unk plane) → honest-loud; iverilog supports it, recorded as a follow-on.
    pub(crate) fn lower_wildcard_eq(&mut self, lhs: &ast::Expr, rhs: &ast::Expr, ne: bool) -> u32 {
        let lhs_id = self.lower_expr(lhs);
        // A fill pattern (`'1`/`'x`/…) sizes to the LHS width, like a case
        // label (§11.6 — mirrors `lower_case_label`).
        let rhs_id = if expr_contains_fill(rhs) {
            let w = self.ir_bits_of(lhs_id).unwrap_or(32);
            self.lower_expr_ctx(rhs, w)
        } else {
            self.lower_expr(rhs)
        };
        if self.expr_is_real(lhs_id) || self.expr_is_real(rhs_id) {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?) is not defined on a real operand",
            );
            return self.placeholder_expr();
        }
        let cv = match self.exprs.get(rhs_id as usize) {
            Some(ir::Expr::Const { val }) => self.consts.get(*val as usize).cloned(),
            _ => None,
        };
        // Numeric AND string-literal patterns are fine (a string has packed
        // known bytes and no x/z, so its mask is all-ones — iverilog accepts
        // `"ab" ==? "ab"`). Real is guarded above; a COMPOUND const expression
        // (`{2'b1?,2'b1?}`, `(P|1)`) lowers to a non-Const node and stays loud
        // (a const-fold walker is a recorded follow-on — honest, iverilog folds).
        let Some(cv) = cv.filter(|c| c.repr != ir::ConstRepr::Real) else {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?) needs a constant right-hand pattern \
                 (a runtime pattern's x/z mask has no IR primitive; a compound \
                 const expression is not folded yet — use a literal or parameter)",
            );
            return self.placeholder_expr();
        };
        self.wildcard_cmp_ids(lhs_id, rhs, &cv, ne)
    }

    /// ONE value element of an `inside` set (IEEE 1800-2017 §11.4.13) with both
    /// operands already lowered — each exactly once, by the calls the `Eq` arm of
    /// the same lowering walk makes. `None` means the comparison IS `==` and the
    /// caller pushes the `Eq` node it pushes for `==` (byte-identical IR); `Some`
    /// is the `==?` compare, or a placeholder after a loud refusal.
    ///
    /// §11.4.13 compares an integral element with `==?` and a non-integral one
    /// (`real`, `shortreal`) with `==`; the `||` fold over the elements the parser
    /// builds already yields its result rule (any 1 → 1, else any x → x, else 0).
    /// `==?` and `==` differ only where the ELEMENT has an x/z bit, so:
    ///
    /// | lowered element | comparison |
    /// |---|---|
    /// | either operand real | `==` |
    /// | provably no x/z bit (`expr_may_be_unknown` false) | `==` |
    /// | one `Const` with x/z bits | `==?`, built by [`Self::wildcard_cmp_ids`] |
    /// | an x/z literal built into a larger expression (`{p, 2'b?0}`, `~4'b0?11`, `$unsigned(4'b1?00)`) | loud: the wildcard bits are not one constant pattern here |
    /// | anything else — a variable, a call | `==` (residue below) |
    ///
    /// ⚠️ RESIDUE, recorded not fixed: an element whose x/z bits arrive at RUN time
    /// (a 4-state variable holding `4'b1x00`, a function returning one) is compared
    /// with `==`, so it reads `x` where §11.4.13 matches. iverilog runs it (the
    /// runtime `==?`); vita has no IR primitive for a runtime mask, the same reason
    /// `lower_wildcard_eq` refuses a non-constant pattern. A variable element with no
    /// x/z bit is right either way, which is why this stays `==` rather than loud.
    pub(crate) fn inside_value_cmp(
        &mut self,
        el: &ast::Expr,
        lhs_id: u32,
        el_id: u32,
    ) -> Option<u32> {
        if self.expr_is_real(lhs_id) || self.expr_is_real(el_id) {
            return None;
        }
        if !self.expr_may_be_unknown(el_id) {
            return None;
        }
        if let Some(ir::Expr::Const { val }) = self.exprs.get(el_id as usize) {
            let cv = self.consts.get(*val as usize).cloned()?;
            let has_xz = cv.bits.unk.iter().any(|&u| u != 0);
            if cv.repr == ir::ConstRepr::Numeric && has_xz {
                return Some(self.wildcard_cmp_ids(lhs_id, el, &cv, false));
            }
            return None;
        }
        if self.xz_literal_reaches_value(el_id) {
            self.error(
                MsgCode::ElabUnsupported,
                "an `inside` element that builds an x/z/? literal into a larger \
                 expression is not supported: IEEE 1800-2017 §11.4.13 compares the \
                 element with `==?`, and its don't-care bits must be one constant \
                 pattern here — write the element as a single literal",
            );
            return Some(self.placeholder_expr());
        }
        None
    }

    /// Does an x/z/? bit of a LITERAL inside the already-lowered `e` reach `e`'s
    /// value bits? An exhaustive walk (a new IR kind is a compile error here) that
    /// descends through every node whose result bits come from its operands' bits
    /// and stops at the ones whose value is not their arguments' bits: a call, an
    /// array item, a system function other than the `$signed`/`$unsigned`
    /// re-interpretation (`TwoState` maps x/z to 0). A `Select` descends into its
    /// index too, so the walk errs toward `true`, which its one caller turns into a
    /// refusal rather than a value.
    fn xz_literal_reaches_value(&self, e: u32) -> bool {
        let Some(x) = self.exprs.get(e as usize) else {
            return true;
        };
        match x {
            ir::Expr::Const { val } => self
                .consts
                .get(*val as usize)
                .is_none_or(|c| c.bits.unk.iter().any(|&u| u != 0)),
            ir::Expr::Signal { .. } | ir::Expr::Call { .. } | ir::Expr::ArrayItem { .. } => false,
            ir::Expr::Select {
                base,
                offset,
                width,
                ..
            } => {
                self.xz_literal_reaches_value(*base)
                    || self.xz_literal_reaches_value(*offset)
                    || self.xz_literal_reaches_value(*width)
            }
            ir::Expr::Concat { parts } => parts.iter().any(|&p| self.xz_literal_reaches_value(p)),
            ir::Expr::Replicate { count, value } => {
                self.xz_literal_reaches_value(*count) || self.xz_literal_reaches_value(*value)
            }
            ir::Expr::Unary { operand, .. } => self.xz_literal_reaches_value(*operand),
            ir::Expr::Binary { lhs, rhs, .. } => {
                self.xz_literal_reaches_value(*lhs) || self.xz_literal_reaches_value(*rhs)
            }
            ir::Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.xz_literal_reaches_value(*cond)
                    || self.xz_literal_reaches_value(*then_e)
                    || self.xz_literal_reaches_value(*else_e)
            }
            ir::Expr::SysFunc { which, args } => {
                matches!(which, ir::SysFuncId::Signed | ir::SysFuncId::Unsigned)
                    && args.iter().any(|&a| self.xz_literal_reaches_value(a))
            }
        }
    }

    /// The `(lhs & mask) ==/!= cleaned` compare for the constant pattern `cv`
    /// (lowered from `pat`) against the already-lowered `lhs_id`.
    ///
    /// The comparison runs at `w = max(lhs width, pattern width)`. Within the
    /// pattern's own width, mask = its known bits and cleaned = its value with the
    /// wildcard bits cleared. An operand narrower than `w` is extended first, by
    /// §11.4.5's rule for equality (which §11.4.6 applies to `==?`): sign extension
    /// when BOTH operands are signed, zero extension otherwise. So the pattern's
    /// extension bits [pattern width, w) are:
    ///
    /// | case | extension bits |
    /// |---|---|
    /// | signed comparison, pattern MSB known | the MSB's value, compared |
    /// | signed comparison, pattern MSB x/z | x/z — don't-care (`4'sb?100` against a signed byte) |
    /// | unsized literal whose leftmost digit is x/z (§5.7.1: it pads to the width of the expression) | don't-care (`'bx1` against 36 bits) |
    /// | otherwise | 0, compared (`3'b1?0` against 4 bits) |
    ///
    /// and a signed LEFT operand narrower than the pattern is sign-extended by
    /// giving the mask AND the cleaned pattern the signed type: the `Eq` and its
    /// `BitAnd` are one §11.8.2 region, signed only when every operand is, and the
    /// engine extends the left operand by the region's sign. Every other comparison
    /// is unsigned with unsigned constants, as it always was. Measured against iverilog 13.0's own
    /// `==?` and sv2v 0.0.13 → iverilog on every cell, verilator 5.052 on the
    /// 2-state ones (it reads an x/z sign bit as 0, so it is not an oracle for that
    /// extension).
    ///
    /// An unsizable left operand is loud: falling back to the pattern width would
    /// build a too-narrow mask whose zero-extension ANDs the lhs's high bits away —
    /// every high bit would silently "match". A signed pattern of another width
    /// against a left operand whose signedness is not known yet (an unresolved
    /// hierarchical placeholder) is loud for the same reason: the extension rule
    /// depends on it.
    pub(crate) fn wildcard_cmp_ids(
        &mut self,
        lhs_id: u32,
        pat: &ast::Expr,
        cv: &ir::ConstVal,
        ne: bool,
    ) -> u32 {
        let Some(aw) = self.ir_bits_of(lhs_id) else {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?, or an `inside` element with x/z bits) \
                 on a left operand of unsizable width is unsupported (the pattern \
                 mask must cover it)",
            );
            return self.placeholder_expr();
        };
        let pw = cv.width.max(1);
        let w = aw.max(pw);
        // Only an extension can make the left operand's sign matter, and only a
        // signed pattern can make the comparison signed (§11.8.1).
        let both_signed = if aw != pw && cv.signed {
            match self.canonical_self_width(lhs_id) {
                Some(s) => s.signed,
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "wildcard equality (==?/!=?, or an `inside` element with x/z \
                         bits) with a signed pattern of another width needs the left \
                         operand's signedness, which is not known at this point",
                    );
                    return self.placeholder_expr();
                }
            }
        } else {
            false
        };
        let nwords = (w as usize).div_ceil(64);
        let mut mask = vec![0u64; nwords];
        let mut clean = vec![0u64; nwords];
        for wi in 0..nwords {
            let cvv = cv.bits.val.get(wi).copied().unwrap_or(0);
            let cvu = cv.bits.unk.get(wi).copied().unwrap_or(0);
            mask[wi] = !cvu; // pattern x/z ⇒ 0 (don't-care); known/extension ⇒ 1
            clean[wi] = cvv & !cvu; // wildcard positions cleared to 0
        }
        if pw < w {
            let bit = |v: &[u64], i: u32| {
                v.get(i as usize / 64)
                    .is_some_and(|x| (x >> (i % 64)) & 1 == 1)
            };
            let msb_xz = bit(&cv.bits.unk, pw - 1);
            let fill = if both_signed {
                (!msb_xz).then(|| bit(&cv.bits.val, pw - 1))
            } else if msb_xz && is_unsized_literal(pat) {
                None
            } else {
                Some(false)
            };
            // `Some(b)`: compared against `b`; `None`: don't-care. `Some(false)` is
            // what the loop above already wrote, so the IR of every comparison that
            // predates the sign and unsized rules is unchanged.
            if fill != Some(false) {
                for i in pw..w {
                    let (word, sh) = ((i / 64) as usize, i % 64);
                    match fill {
                        Some(b) => {
                            mask[word] |= 1 << sh;
                            if b {
                                clean[word] |= 1 << sh;
                            } else {
                                clean[word] &= !(1 << sh);
                            }
                        }
                        None => {
                            mask[word] &= !(1 << sh);
                            clean[word] &= !(1 << sh);
                        }
                    }
                }
            }
        }
        let top = w % 64;
        if top != 0 {
            let m = (1u64 << top) - 1;
            mask[nwords - 1] &= m;
            clean[nwords - 1] &= m;
        }
        let push_const = |el: &mut Self, bits: Vec<u64>, signed: bool| -> u32 {
            let cid = el.intern_const(ir::ConstVal {
                width: w,
                signed,
                repr: ir::ConstRepr::Numeric,
                bits: ir::BitPacked {
                    val: bits,
                    unk: vec![0u64; nwords],
                },
            });
            el.push_expr(ir::Expr::Const { val: cid })
        };
        // Both constants signed: the compare is ONE §11.8.2 region, signed only if
        // every operand is, so a signed mask alone would leave `Eq` unsigned and
        // zero-extend the left operand before the `BitAnd` saw it.
        let sext_lhs = both_signed && aw < w;
        let m_id = push_const(self, mask, sext_lhs);
        let c_id = push_const(self, clean, sext_lhs);
        let anded = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::BitAnd,
            lhs: lhs_id,
            rhs: m_id,
        });
        self.push_expr(ir::Expr::Binary {
            op: if ne { ir::BinOp::Ne } else { ir::BinOp::Eq },
            lhs: anded,
            rhs: c_id,
        })
    }
}

/// Is `e` (through parentheses) an UNSIZED based literal (`'bx1`, `'h?`)? §5.7.1
/// pads such a literal whose leftmost digit is x/z with that x/z to the width of the
/// expression containing it, not with zeros.
fn is_unsized_literal(e: &ast::Expr) -> bool {
    match &e.kind {
        ast::ExprKind::Paren { inner } => is_unsized_literal(inner),
        ast::ExprKind::IntLit { kind, .. } => matches!(kind, ast::IntLitKind::UnsizedBased),
        _ => false,
    }
}
