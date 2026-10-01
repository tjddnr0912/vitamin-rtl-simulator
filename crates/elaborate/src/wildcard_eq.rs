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
        self.wildcard_cmp_ids(lhs_id, rhs_id, &cv, ne)
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
    pub(crate) fn inside_value_cmp(&mut self, lhs_id: u32, el_id: u32) -> Option<u32> {
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
                return Some(self.wildcard_cmp_ids(lhs_id, el_id, &cv, false));
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

    /// §11.4.6 `==?`/`!=?` and §11.4.13's `inside` element against an x/z LITERAL
    /// pattern, in the i64 domain: the left operand is evaluated at the comparison's
    /// common width `w = max(L(lhs), L(pattern))` with the pair's sign (§11.6.1 Table
    /// 11-21, §11.8.1 — so a carry, a shift or a `~` runs at `w`, which reading it at
    /// its own width did not: `(4'd15 + 4'd1) ==? 5'b1?000` was 0, both oracles 1),
    /// the pattern is extended by [`crate::wildcard_eq::pattern_ext_fill`] — the rule
    /// the run-time builder uses — and [`crate::const_wide::wildcard_match`] decides.
    ///
    /// `None` hands the node on, never a guessed answer: a fill pattern, a pattern
    /// with no x/z bit (that is `==`, answered by the generic arm), a width past 64 or
    /// an unknown one, and a left operand this walk cannot evaluate (an x/z literal
    /// inside it) all go to the wide domain (`selfdet_bits_i64`), which answers or
    /// declines — loud at the consumer.
    pub(crate) fn const_wildcard_i64(
        &self,
        op: ast::BinOp,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        if !matches!(
            op,
            ast::BinOp::WildEq | ast::BinOp::WildNe | ast::BinOp::InsideEq
        ) {
            return None;
        }
        let mut pat = rhs;
        while let ast::ExprKind::Paren { inner } = &pat.kind {
            pat = inner;
        }
        let ast::ExprKind::IntLit { kind, raw } = &pat.kind else {
            return None;
        };
        if literal::is_fill_literal(raw, *kind) {
            return None;
        }
        let cv = parse_int_literal(raw, *kind)?;
        if !cv.bits.unk.iter().any(|&u| u != 0) {
            return None;
        }
        let pw = cv.width.max(1);
        let w = self.const_self_width(lhs, envw)?.max(pw);
        if w > 64 {
            return None;
        }
        let sg = self.const_signed_env(lhs, envw) && cv.signed;
        let a = self.eval_const_env_at(lhs, env, envw, depth, w, sg)?;
        let lbits = ir::BitPacked {
            val: vec![a as u64],
            unk: vec![0],
        };
        let fill = if pw < w {
            let bit = |v: &[u64]| v.first().is_some_and(|x| (x >> (pw - 1)) & 1 == 1);
            crate::wildcard_eq::pattern_ext_fill(
                bit(&cv.bits.unk),
                bit(&cv.bits.val),
                sg,
                matches!(kind, ast::IntLitKind::UnsizedBased),
            )
        } else {
            Some(false)
        };
        let m = crate::const_wide::wildcard_match(&lbits, &cv.bits, pw, w, fill)?;
        Some(i64::from(m != matches!(op, ast::BinOp::WildNe)))
    }

    /// The `(lhs & mask) ==/!= cleaned` compare for the constant pattern `cv` (the
    /// `Const` node `pat_id`) against the already-lowered `lhs_id`.
    ///
    /// The comparison runs at `w = max(lhs width, pattern width)`. Within the
    /// pattern's own width, mask = its known bits and cleaned = its value with the
    /// wildcard bits cleared. An operand narrower than `w` is extended first, by
    /// §11.4.5's rule for equality (which §11.4.6 applies to `==?`): sign extension
    /// when BOTH operands are signed, zero extension otherwise. The pattern's
    /// extension bits [pattern width, w) follow [`pattern_ext_fill`], the one rule the
    /// constant domains read too.
    ///
    /// A signed LEFT operand narrower than the pattern is sign-extended by giving the
    /// mask AND the cleaned pattern the signed type: the `Eq` and its `BitAnd` are
    /// one §11.8.2 region, signed only when every operand is, and the engine extends
    /// the left operand by the region's sign. Every other comparison is unsigned with
    /// unsigned constants, as it always was. Measured against iverilog 13.0's own
    /// `==?` and sv2v 0.0.13 → iverilog on every cell, verilator 5.052 on the 2-state
    /// ones (it reads an x/z sign bit as 0, so it is not an oracle for that extension).
    ///
    /// A left operand with NO width yet (an absolute hierarchical reference, which is
    /// a placeholder until every instance exists) takes [`Self::wildcard_cmp_or_form`]
    /// instead, which needs neither its width nor its sign. A signed pattern of
    /// another width against a left operand whose width is known but whose sign is not
    /// is loud: the AND form's extension depends on it.
    pub(crate) fn wildcard_cmp_ids(
        &mut self,
        lhs_id: u32,
        pat_id: u32,
        cv: &ir::ConstVal,
        ne: bool,
    ) -> u32 {
        let unsized_xz = self.unsized_xz_lits.contains(&pat_id);
        let Some(aw) = self.ir_bits_of(lhs_id) else {
            return self.wildcard_cmp_or_form(lhs_id, cv, unsized_xz, ne);
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
            let msb_xz = word_bit(&cv.bits.unk, pw - 1);
            let fill = pattern_ext_fill(
                msb_xz,
                word_bit(&cv.bits.val, pw - 1),
                both_signed,
                unsized_xz,
            );
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
        // Both constants signed: the compare is ONE §11.8.2 region, signed only if
        // every operand is, so a signed mask alone would leave `Eq` unsigned and
        // zero-extend the left operand before the `BitAnd` saw it.
        let sext_lhs = both_signed && aw < w;
        let m_id = self.push_known_const(w, sext_lhs, mask);
        let c_id = self.push_known_const(w, sext_lhs, clean);
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

    /// The wildcard compare for a left operand whose width is not known at lowering:
    /// `(lhs | W) ==/!= (P | W)`, `W` = the pattern's wildcard bits and `P | W` = the
    /// pattern with them set, both at the PATTERN's width and with the PATTERN's
    /// sign. The engine sizes and signs this as one §11.8.2 region when the operand
    /// is finally known, so nothing about it is guessed here:
    ///
    /// - a wildcard bit: `x | 1` = `1` = `1`, whatever the left bit holds;
    /// - a compared bit: `l | 0` = `l` against the pattern bit, which is `==`'s 4-state
    ///   rule (a definite mismatch decides 0, an x/z left bit gives x);
    /// - a wider left operand: `W` and `P | W` extend by the region's sign — zero when
    ///   either side is unsigned (the left operand's high bits are compared against
    ///   0), the pattern's own MSB when both are signed (a don't-care MSB has `W`'s
    ///   MSB set, so its extension stays don't-care);
    /// - a narrower left operand extends by the same region sign (§11.4.5).
    ///
    /// Hand-spelled on PRE (00c3d76d) with absolute paths, 25 cells — unsigned and
    /// signed, narrower and wider on both sides, x/z left bits under a wildcard and
    /// under a compared bit, `!=`: every one printed iverilog 13.0's own `==?` text.
    ///
    /// The one shape it cannot carry is an unsized literal whose leftmost digit is x/z:
    /// §5.7.1 pads it to the width of the expression, which `W` zero-extended past its
    /// 32 bits would compare. Without the left width there is no way to know whether
    /// that happens, so it is loud.
    fn wildcard_cmp_or_form(
        &mut self,
        lhs_id: u32,
        cv: &ir::ConstVal,
        unsized_xz: bool,
        ne: bool,
    ) -> u32 {
        if unsized_xz {
            self.error(
                MsgCode::ElabUnsupported,
                "wildcard equality (==?/!=?, or an `inside` element with x/z bits) with an \
                 unsized pattern whose leftmost digit is x/z, on a left operand whose width \
                 is not known at this point (a hierarchical reference), is unsupported: \
                 the pattern pads to the expression's width, which is not known yet",
            );
            return self.placeholder_expr();
        }
        let pw = cv.width.max(1);
        let nwords = (pw as usize).div_ceil(64);
        let mut wild = vec![0u64; nwords];
        let mut set = vec![0u64; nwords];
        for wi in 0..nwords {
            let cvv = cv.bits.val.get(wi).copied().unwrap_or(0);
            let cvu = cv.bits.unk.get(wi).copied().unwrap_or(0);
            wild[wi] = cvu;
            set[wi] = cvv | cvu;
        }
        let top = pw % 64;
        if top != 0 {
            let m = (1u64 << top) - 1;
            wild[nwords - 1] &= m;
            set[nwords - 1] &= m;
        }
        let w_id = self.push_known_const(pw, cv.signed, wild);
        let p_id = self.push_known_const(pw, cv.signed, set);
        let ored = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::BitOr,
            lhs: lhs_id,
            rhs: w_id,
        });
        self.push_expr(ir::Expr::Binary {
            op: if ne { ir::BinOp::Ne } else { ir::BinOp::Eq },
            lhs: ored,
            rhs: p_id,
        })
    }

    /// A fully known numeric constant node of `width` bits.
    fn push_known_const(&mut self, width: u32, signed: bool, val: Vec<u64>) -> u32 {
        let nwords = val.len();
        let cid = self.intern_const(ir::ConstVal {
            width,
            signed,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val,
                unk: vec![0u64; nwords],
            },
        });
        self.push_expr(ir::Expr::Const { val: cid })
    }
}

/// THE extension rule of a wildcard pattern narrower than its comparison — read by
/// the run-time builder ([`Elaborator::wildcard_cmp_ids`]) and by both constant
/// domains (`const_fn_width`'s i64 walk and `const_wide::fold_region`), so a
/// constant and a run-time spelling of one comparison cannot extend differently.
///
/// `Some(b)`: each extension bit is compared against `b`; `None`: each is a
/// don't-care.
///
/// | case | extension bits |
/// |---|---|
/// | signed comparison (§11.4.5: both operands signed), pattern MSB known | the MSB's value, compared |
/// | signed comparison, pattern MSB x/z | x/z — don't-care (`4'sb?100` against a signed byte) |
/// | unsized literal whose leftmost digit is x/z (§5.7.1: it pads to the width of the expression) | don't-care (`'bx1` against 36 bits) |
/// | otherwise | 0, compared (`3'b1?0` against 4 bits, `4'sb1?00` against an unsigned byte) |
pub(crate) fn pattern_ext_fill(
    msb_xz: bool,
    msb_val: bool,
    both_signed: bool,
    unsized_xz: bool,
) -> Option<bool> {
    if both_signed {
        (!msb_xz).then_some(msb_val)
    } else if msb_xz && unsized_xz {
        None
    } else {
        Some(false)
    }
}

/// Does `e` hold a wildcard comparison (`==?`, `!=?`, an `inside` element) with an
/// x/z literal on either side? The constant positions that swallow a declined fold —
/// a range bound, an array dimension, a generate-`case` item — ask it to refuse
/// loudly instead: such a node declines only when its value is x (an x/z left bit
/// under a compared pattern bit) or its x/z pattern is not one literal, and a
/// silent default there is a different design.
pub(crate) fn holds_xz_wildcard(e: &ast::Expr) -> bool {
    crate::param_query::ast_any(e, &|x| {
        matches!(
            &x.kind,
            ast::ExprKind::Binary {
                op: ast::BinOp::WildEq | ast::BinOp::WildNe | ast::BinOp::InsideEq,
                lhs,
                rhs,
            } if crate::param_query::ast_holds_unknown_literal(lhs)
                || crate::param_query::ast_holds_unknown_literal(rhs)
        )
    })
}

/// Bit `i` of a little-endian word vector (`false` past its end).
fn word_bit(v: &[u64], i: u32) -> bool {
    v.get(i as usize / 64)
        .is_some_and(|x| (x >> (i % 64)) & 1 == 1)
}
