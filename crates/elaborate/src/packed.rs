//! packed selects (read) — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Internal-bit select direction for an indexed part-select `[base ± width]`.
/// On a DESCENDING net the source-index direction equals the internal-bit
/// direction (`+:` ⇒ up, `-:` ⇒ down). On an ASCENDING (`[lo:hi]`) net the source
/// index runs opposite to the internal bit (index 0 is the MSB), so `+:` moves
/// DOWN in internal bits and `-:` UP — the offset (`norm_offset_for_net`) already
/// maps the base index onto its internal bit. IEEE 1800 §11.5.1 + §7.4.3.
/// Was this index one the seal touched BEFORE §4.5.309 — a conservative,
/// hand-written §5.5.1 proof of unsignedness?
///
/// It is kept, unchanged, for exactly one job: deciding whether to seal an
/// UNSIGNED index. §4.5.309 replaced it with the canonical rule everywhere else,
/// and doing the same here moved 13 measured cells from right to wrong, because
/// what the seal does to an unsigned index under a NEGATIVE declared base is not
/// "pin its width" but "remove a 32-bit wrap" — and that wrap is the answer
/// iverilog gives (`reg [7:0] ma[-3:2]` indexed by `$stime + ii` reads `ma[-3]`
/// there). Which unsigned shapes should be sealed is therefore not a signedness
/// question at all; it is the array-word i32-reinterpretation question already
/// open in ROADMAP §2, and no width/base predicate separates the two groups —
/// measured: `reg [31:0] ix` and `$stime` are both 32-bit unsigned under the same
/// negative base, and PRE is right about the first only when sealed and right
/// about the second only when NOT sealed.
///
/// So the unsigned half is deliberately frozen at its pre-§4.5.309 decision and
/// this predicate is its definition, not an independent claim about IEEE. The
/// three arms review measured as WRONG about signedness (`**`, class fields, real
/// literals) no longer decide anything: `sealed_signed_index` asks
/// `index_self_width` first, and only an index that rule calls UNSIGNED ever
/// reaches this function.
pub fn expr_provably_unsigned(
    exprs: &[ir::Expr],
    consts: &[ir::ConstVal],
    nets: &[ir::NetVar],
    class_fields: &std::collections::BTreeMap<u32, (u32, bool)>,
    eid: u32,
) -> bool {
    match exprs.get(eid as usize) {
        // §5.4.1: a bit-select / part-select result is ALWAYS unsigned; a
        // concat or replication likewise.
        Some(ir::Expr::Select { .. })
        | Some(ir::Expr::Concat { .. })
        | Some(ir::Expr::Replicate { .. }) => true,
        // A REAL literal is canonically SIGNED (`width.rs` gives it
        // `{64, signed: true}`). The operative guard is `!c.signed`: the sole
        // real producer sets `signed: true` and the sole `StrUtf8` producer
        // sets `signed: false`, so naming `Real` in the `matches!` is
        // defence-in-depth rather than the exclusion, and admitting `StrUtf8`
        // costs no proofs. Both measured, not assumed.
        Some(ir::Expr::Const { val }) => consts.get(*val as usize).is_some_and(|c| {
            matches!(c.repr, ir::ConstRepr::Numeric | ir::ConstRepr::StrUtf8) && !c.signed
        }),
        Some(ir::Expr::Signal { net, word }) => {
            // A CLASS-FIELD read is also a `Signal{net, word: Some(field)}`,
            // but its net is the 32-bit HANDLE — the field's own signedness is
            // in `class_field_widths`, the very map the engine applies through
            // `WidthTable::patch_class_fields`. Asking that map here is what
            // makes this function the WHOLE predicate: the first fix put a
            // blanket refusal in a wrapper instead, which was sound but threw
            // away the unsigned-field case the seal had just started getting
            // right, and left the wrapper's own recursion untested.
            if let Some(&(_, signed)) = class_fields.get(&eid) {
                return !signed;
            }
            let _ = word;
            nets.get(*net as usize).is_some_and(|n| !n.signed)
        }
        Some(ir::Expr::Unary { op, operand }) => match op {
            // A reduction or logical negation yields ONE unsigned bit.
            ir::UnOp::RedAnd
            | ir::UnOp::RedOr
            | ir::UnOp::RedXor
            | ir::UnOp::RedNand
            | ir::UnOp::RedNor
            | ir::UnOp::RedXnor
            | ir::UnOp::LogNot => true,
            // `~`, unary `+`/`-` keep the operand's signedness.
            _ => expr_provably_unsigned(exprs, consts, nets, class_fields, *operand),
        },
        Some(ir::Expr::Binary { op, lhs, rhs }) => match op {
            // Relational / equality / logical: one unsigned bit.
            ir::BinOp::Lt
            | ir::BinOp::Le
            | ir::BinOp::Gt
            | ir::BinOp::Ge
            | ir::BinOp::Eq
            | ir::BinOp::Ne
            | ir::BinOp::CaseEq
            | ir::BinOp::CaseNe
            | ir::BinOp::CasezEq
            | ir::BinOp::LogAnd
            | ir::BinOp::LogOr => true,
            // Shifts keep the LEFT operand's signedness (§5.5.1: the right
            // operand is self-determined and never signs the result).
            ir::BinOp::Shl | ir::BinOp::Shr | ir::BinOp::AShl | ir::BinOp::AShr => {
                expr_provably_unsigned(exprs, consts, nets, class_fields, *lhs)
            }
            // `**` takes the BASE's sign alone — an unsigned EXPONENT must not
            // demote a signed base (`width.rs`'s rule, and iverilog's). It used
            // to fall into the either-unsigned arm below, which proved
            // `s32 ** u32` unsigned and made the seal reinterpret a negative
            // result as huge: a measured regression, caught by review.
            ir::BinOp::Pow => expr_provably_unsigned(exprs, consts, nets, class_fields, *lhs),
            // Arithmetic / bitwise: unsigned if EITHER operand is unsigned.
            _ => {
                expr_provably_unsigned(exprs, consts, nets, class_fields, *lhs)
                    || expr_provably_unsigned(exprs, consts, nets, class_fields, *rhs)
            }
        },
        // A ternary is signed only when BOTH arms are; either arm proving
        // unsigned is enough.
        Some(ir::Expr::Ternary { then_e, else_e, .. }) => {
            expr_provably_unsigned(exprs, consts, nets, class_fields, *then_e)
                || expr_provably_unsigned(exprs, consts, nets, class_fields, *else_e)
        }
        // ONLY `$unsigned`. `$clog2` looked unsigned-when-its-argument-is and
        // was written that way first; the canonical table says SIGNED (it
        // returns an `integer`), and the cross-crate property test caught it
        // on its first run. Every other system function is left unproven — the
        // cost of a `false` here is one lowering that stays exactly as it was.
        Some(ir::Expr::SysFunc { which, .. }) => matches!(which, ir::SysFuncId::Unsigned),
        _ => false,
    }
}

/// An expression node that is still a DEFERRED-HIERARCHY placeholder — its net
/// or callee is `POISON_*` until `resolve_deferred_hier_*` patches it in place
/// after every instance exists.
///
/// Two variants and no third: `POISON_NET` is only ever written into an
/// `Expr::Signal`'s `net` and `POISON_FID` only into an `Expr::Call`'s `func`
/// (`limits.rs`, and the `expr_main`/`params` sites that create them). A
/// placeholder is a leaf as far as this question goes — what matters is that
/// asking the width rule about one gets a FABRICATED answer, not a wrong one.
///
/// ⚠️ The `Call` arm is EQUIVALENT today and is here for the next change, not
/// for this one — measured by mutation. Fabricating an answer for a `Call`
/// yields "unsigned", and the unsigned half of the seal is frozen at the old
/// predicate, which never admits an `Expr::Call`; so a hierarchical call
/// declines through the fabricated path exactly as it declines through this
/// one. Delete this arm and nothing observable moves — until the unsigned half
/// is unfrozen (ROADMAP §2), at which point it is the only thing between a
/// hierarchical call and a seal built on a one-bit lie.
fn is_expr_placeholder(e: &ir::Expr) -> bool {
    matches!(
        e,
        ir::Expr::Signal {
            net: POISON_NET,
            ..
        } | ir::Expr::Call {
            func: POISON_FID,
            ..
        }
    )
}

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
    ) -> Vec<(i64, u32, bool)> {
        let mut out = Vec::new();
        for r in range.into_iter().chain(packed.iter()) {
            let msb_v = self.const_eval_in_scope(&r.msb);
            let lsb_v = self.const_eval_in_scope(&r.lsb);
            // P0-NCW: net/hierarchical-referenced (non-constant) packed bound is loud.
            self.check_const_range_bound(&r.msb, msb_v);
            self.check_const_range_bound(&r.lsb, lsb_v);
            // `lo` is `i64` and the bounds are NOT clamped: a NEGATIVE inner packed bound
            // is legal (`logic [1:0][3:-2]`, IEEE §7.4.1) and clamping it to 0 made the
            // dim 4 bits instead of 6 — the whole vector came out 8 bits wide instead of
            // 12, SILENTLY (measured against iverilog; the clamp warning came from a
            // sibling declaration, never from here). `flatten_word` carries a signed `lo`
            // now, so the coordinate map handles it.
            let msb = msb_v.unwrap_or(0);
            let lsb = lsb_v.unwrap_or(0);
            let w = ((msb.abs_diff(lsb)) + 1).min(u32::MAX as u64) as u32;
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

    /// Is this lowered expression PROVABLY unsigned by IEEE 1364-2005 §5.5.1?
    /// Thin delegate to the free function, which the cross-crate soundness
    /// test can call directly.
    /// The index's CANONICAL self-signedness — `sim_ir::selfwidth`, the same
    /// rule `sim-engine`'s `WidthTable` drives, not a conservative subset.
    ///
    /// §4.5.309 moved that rule into `sim-ir` precisely so this could ask it.
    /// The predicate that stood here was hand-written, and three of its arms
    /// were measurably wrong (`**` takes the base's sign alone; a class field's
    /// sign is a sidecar, not its handle net's; a real literal is signed) —
    /// each a silent-wrong until review found it. The answer is now the same
    /// one the engine will use to evaluate the very expression being lowered.
    /// The pre-§4.5.309 unsigned-seal decision, as a method (see the free
    /// function for why it is still here and what it is now allowed to mean).
    fn unsigned_seal_admitted(&self, eid: u32) -> bool {
        expr_provably_unsigned(
            &self.exprs,
            &self.consts,
            &self.nets,
            &self.class_field_widths,
            eid,
        )
    }

    fn index_self_width(&mut self, eid: u32) -> Option<sim_ir::selfwidth::SelfWidth> {
        // NOT YET KNOWABLE is not the same answer as UNSIGNED, and conflating
        // them is a silent-wrong. A hierarchical reference lowers to a
        // PLACEHOLDER — `Signal{net: POISON_NET}` / `Call{func: POISON_FID}` —
        // patched to the real net/callee only after every instance exists. Ask
        // the canonical rule about one of those and it reads a net that is not
        // there, falls back to 1-bit unsigned, and the seal zero-extends an
        // index that is about to become signed −1: `mg[u.k]` on
        // `reg [7:0] mg[-3:2]` went from the oracle's `aa` to `x` plus an E4002
        // (correct-support → loud-wrong, a rung DOWN the ladder). So say
        // `None` and let the seal decline, exactly as it did before §4.5.309.
        //
        // The arena is post-order, so `eid`'s subtree lies inside `0..=eid` and
        // scanning that range answers "is anything under me still a
        // placeholder?" conservatively without a second expression walker (a
        // walker that under-detects would put the silent-wrong straight back).
        let last = eid as usize;
        let mut i = self.selfw_scan as usize;
        while i <= last {
            if is_expr_placeholder(&self.exprs[i]) {
                // Stay parked here rather than advancing: this one may be
                // patched later, and then the scan resumes from the same spot.
                self.selfw_scan = i as u32;
                // Defensive, and measured to be a no-op: nothing at or above a
                // live placeholder can have been cached, because a placeholder
                // is created when its id is FRESH, so every id above it was
                // answered later — and any such answer had to walk past this
                // very scan. Removing the truncate changes no test. It states
                // the invariant rather than trusting it, and it is what makes
                // the cache below sound to keep across a resolve.
                self.selfw_cache.truncate(i);
                return None;
            }
            i += 1;
        }
        self.selfw_scan = i as u32;
        // Past that scan the whole prefix is resolved, and a resolved expr is
        // never rewritten, so these entries are final and the cache makes a
        // design with many indexed selects pay for the fill once in total
        // rather than once per select.
        let metas = &self.func_metas;
        let call_ret = move |f: u32| metas.get(f as usize).map(|m| (m.ret_width, m.ret_signed));
        let ctx = sim_ir::selfwidth::ExprCtx {
            exprs: &self.exprs,
            consts: &self.consts,
            nets: &self.nets,
        };
        let mut sw = std::mem::take(&mut self.selfw_cache);
        for i in sw.len() as u32..=eid {
            let s = if let Some(&(w, sg)) = self.class_field_widths.get(&i) {
                sim_ir::selfwidth::SelfWidth {
                    width: w.max(1),
                    signed: sg,
                }
            } else {
                sim_ir::selfwidth::self_width_of(ctx, &call_ret, &sw, i)
            };
            sw.push(s);
        }
        let out = sw[eid as usize];
        self.selfw_cache = sw;
        Some(out)
    }

    /// A CONSTANT index's integer value, interpreted with its own signedness —
    /// `None` for a non-constant, an x/z-bearing constant (which must keep
    /// landing on the out-of-range sentinel), a non-numeric literal, or one
    /// wider than 64 bits.
    ///
    /// Folding is what covers the case the seal cannot: an unsized decimal
    /// literal is SIGNED by §5.7.1, so `bus[1 +: 2]` on a `reg [33:2]` is not
    /// provably unsigned and the seal declines it — yet its value is known
    /// here, so the whole normalization collapses to one signed constant and
    /// the underflow (`1 − 2 = −1`, a partial write of the in-range bits) is
    /// expressed directly.
    fn const_index_value(&self, eid: u32) -> Option<i64> {
        let ir::Expr::Const { val } = self.exprs.get(eid as usize)? else {
            return None;
        };
        let c = self.consts.get(*val as usize)?;
        if !matches!(c.repr, ir::ConstRepr::Numeric) || c.width > 64 {
            return None;
        }
        if c.bits.unk.iter().any(|&u| u != 0) {
            return None;
        }
        let raw = c.bits.val.first().copied().unwrap_or(0);
        let w = c.width.max(1);
        let masked = if w >= 64 {
            raw
        } else {
            raw & ((1u64 << w) - 1)
        };
        Some(if c.signed && w < 64 && (masked >> (w - 1)) & 1 == 1 {
            (masked | (!0u64 << w)) as i64
        } else {
            masked as i64
        })
    }

    /// Seal an index at its own self-determined width WITHOUT changing its
    /// sign domain: `{1'b0, idx}`, unsigned, value-preserving.
    ///
    /// The unpacked (array-word) geometry needs only this half, because its
    /// arithmetic is unsigned by design and the defect there was the WIDENING
    /// alone: `ma[~r5]` on a `reg [7:0] ma [2:5]` evaluated `~r5` at 32 bits,
    /// fell outside the dimension and dropped the access, where iverilog reads
    /// and writes `ma[3]`. Gated on the same proof for the same reason — a
    /// concat erases signedness.
    ///
    /// ⚠️ Do NOT justify the unsigned seal by "the per-dim guard rejects an
    /// under-range index": `guard_dims = d >= 2` in both funnels, so a
    /// ONE-dimensional array has no guard at all and an under-range value is
    /// caught only by the word-count bound — on a value this seal is what keeps
    /// honest. Not academic: a `reg [7:0] mg [-3:2]` reached through an
    /// UNSIGNED-typed index holding `0xFFFF_FFFD` is the one cell where vita
    /// and iverilog still differ, and it differs because vita answers by VALUE
    /// where iverilog reinterprets (see `eval::offset_of_index_value`).
    pub(crate) fn seal_index_unsigned(&mut self, raw_off: u32) -> u32 {
        // A SIGNED index cannot ride a concat — it erases the sign — and this
        // path's arithmetic and bounds are unsigned throughout, so the seal is
        // for the unsigned case only. A narrow signed index on an unpacked
        // array is therefore still zero-extended rather than sign-extended;
        // that is the one residual class §4.5.309 leaves, and closing it means
        // moving this geometry into a signed domain (ROADMAP §2).
        // `None` (an unresolved hierarchical reference below this index) declines
        // too — the pre-§4.5.309 answer, and the only safe one.
        //
        // …and an unsigned index is sealed on the PRE-§4.5.309 decision, which is
        // this funnel's whole rule: see `unsigned_seal_admitted`.
        match self.index_self_width(raw_off) {
            Some(sw) if !sw.signed && self.unsigned_seal_admitted(raw_off) => {}
            _ => return raw_off,
        }
        let zero1 = self.const_u32_expr(0, 1);
        self.push_expr(ir::Expr::Concat {
            parts: vec![zero1, raw_off],
        })
    }

    /// The index expression, ready to be normalized: SEALED at its own
    /// self-determined width and lifted into a SIGNED domain, as
    /// `$signed({1'b0, idx})` — but ONLY when `idx` is provably unsigned.
    ///
    /// `None` ⇒ the caller must emit exactly what it emitted before, so a
    /// possibly-signed index keeps today's behaviour bit for bit.
    ///
    /// Why the seal, and why the sign (§4.5.308, oracle iverilog 13):
    ///
    /// - **The seal.** Without it the raw index becomes an operand of a 32-bit
    ///   `Sub`/`Add`, so a Verilog CONTEXT-DETERMINED operator inside it — `~`,
    ///   or the carry/borrow of `+`/`-` — evaluates at thirty-two bits instead
    ///   of its own. `a0[~r5]` on a `reg [0:31]` wrote nothing where iverilog
    ///   writes bit 3, and read `x` where iverilog reads 1. A concat is
    ///   self-determined, so `{1'b0, idx}` pins the width to what the user
    ///   wrote. This is "convert at the CONTEXT BOUNDARY, not the leaf",
    ///   applied to the boundary this function IS.
    /// - **The sign.** The normalization legitimately goes negative: an index
    ///   below a non-zero declared LSB (`bus[1 +: 2]` on `reg [33:2]`) is an
    ///   underflow that partial-writes the in-range bits (P0-IPU, which
    ///   `eval::offset_of_index_value` reads as a small negative in the i32
    ///   domain). Computed unsigned it WRAPPED to a huge positive and the whole
    ///   write vanished.
    /// - **Why only when provably unsigned.** A concat erases signedness — it
    ///   is the only self-determined construct Verilog has — so sealing a
    ///   SIGNED index would turn a genuine negative into a large positive and
    ///   write the wrong bit. The first version of this fix did exactly that
    ///   and the axis sweep caught it: `~k`/`k-1`/`-1` (signed `integer`
    ///   indices) got WORSE while `~r5` got better. Trading one silent-wrong
    ///   for another is the one move the accuracy ladder forbids, so the seal
    ///   is gated on a proof and possibly-signed indices are left alone —
    ///   still wrong, but no more wrong than before, and recorded in ROADMAP §2.
    fn sealed_signed_index(&mut self, raw_off: u32) -> Option<u32> {
        // `$signed(x)` preserves x's SELF width and forces the sign attribute
        // (`sim_ir::selfwidth`'s own rule for it), so it is the seal in both
        // cases — what differs is what has to be sealed first:
        //
        // - UNSIGNED index: `$signed({1'b0, idx})`. The concat pins the width;
        //   the extra zero keeps the reinterpretation a zero-extension, so a
        //   large unsigned value stays large instead of reading as negative.
        // - SIGNED index: `$signed(idx)`. The value is already the number the
        //   user wrote. Sealing it through a concat would erase the sign and
        //   turn a negative index into a large positive — the silent-wrong the
        //   first version of this shipped, and the reason the seal used to
        //   DECLINE signed indices outright. Declining was honest but left four
        //   classes wrong; asking the canonical rule closes them.
        let sw = self.index_self_width(raw_off)?;
        let signed = sw.signed;
        if !signed && !self.unsigned_seal_admitted(raw_off) {
            return None;
        }
        let inner = if signed {
            raw_off
        } else {
            let zero1 = self.const_u32_expr(0, 1);
            self.push_expr(ir::Expr::Concat {
                parts: vec![zero1, raw_off],
            })
        };
        Some(self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Signed,
            args: vec![inner],
        }))
    }

    /// `idx − k` (descending, `k` = declared LSB). Sealed+signed when the index
    /// is provably unsigned; otherwise EXACTLY the previous emission — an
    /// unsigned 32-bit `Sub`, or an `Add` by `|k|` for a negative `k`, which is
    /// what the old code wrote to dodge a wrapped unsigned constant.
    fn norm_sub_k(&mut self, raw_off: u32, k: i32) -> u32 {
        if let Some(v) = self.const_index_value(raw_off) {
            let n = v.saturating_sub(i64::from(k));
            return self.const_s32_expr(n.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
        }
        match self.sealed_signed_index(raw_off) {
            Some(idx) => {
                let k_c = self.const_s32_expr(k);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Sub,
                    lhs: idx,
                    rhs: k_c,
                })
            }
            None if k < 0 => {
                let add = self.const_u32_expr(k.unsigned_abs(), 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Add,
                    lhs: raw_off,
                    rhs: add,
                })
            }
            None => {
                let k_c = self.const_u32_expr(k as u32, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Sub,
                    lhs: raw_off,
                    rhs: k_c,
                })
            }
        }
    }

    /// `k − idx` (ascending, `k` = the larger declared endpoint, whose source
    /// index is internal bit 0). Same two-way split as `norm_sub_k`.
    fn norm_k_sub(&mut self, raw_off: u32, k: i32) -> u32 {
        if let Some(v) = self.const_index_value(raw_off) {
            let n = i64::from(k).saturating_sub(v);
            return self.const_s32_expr(n.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
        }
        match self.sealed_signed_index(raw_off) {
            Some(idx) => {
                let k_c = self.const_s32_expr(k);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Sub,
                    lhs: k_c,
                    rhs: idx,
                })
            }
            None => {
                let k_c = self.const_u32_expr(k.max(0) as u32, 32);
                self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Sub,
                    lhs: k_c,
                    rhs: raw_off,
                })
            }
        }
    }

    pub(crate) fn norm_offset_for_net(&mut self, net: u32, raw_off: u32) -> u32 {
        let Some((msb, lsb)) = self.nets.get(net as usize).map(|nv| (nv.msb, nv.lsb)) else {
            return raw_off;
        };
        // A net declared with a NEGATIVE low bound is STORED normalized as `[w-1:0]`
        // (frozen `u32` fields), so `lsb == 0` above would take the raw index verbatim and
        // read the wrong bit — `x[3]` on a `logic [3:-2]` is internal bit 5, not 3. The
        // declared bound comes back from the sparse side map; the offset is `raw + |lsb|`
        // rather than a `Sub` by a wrapped constant, so the arithmetic is right by
        // construction. Absent for every ordinary net ⇒ byte-identical below.
        if let Some(&neg) = self.net_decl_neg_lsb.get(&net) {
            // `raw + |lsb|`, i.e. `raw − lsb` with a negative lsb — the same
            // sealed signed subtraction as the other two arms.
            let k = -(neg.unsigned_abs().min(i32::MAX as u64) as i64) as i32;
            return self.norm_sub_k(raw_off, k);
        }
        if msb >= lsb {
            if lsb == 0 {
                return raw_off; // `[N:0]` — raw index is already internal
            }
            self.norm_sub_k(raw_off, lsb.min(i32::MAX as u32) as i32)
        } else {
            // ascending `[lo:hi]`: the largest source index (`lsb`) is internal bit 0.
            self.norm_k_sub(raw_off, lsb.min(i32::MAX as u32) as i32)
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
    pub(crate) fn packed_elem_resid(&self, base: &ast::Expr) -> Option<(i64, u32, bool)> {
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
        lo: i64,
        width: u32,
        asc: bool,
    ) -> u32 {
        if !asc {
            match lo.cmp(&0) {
                // byte-identical common case: a `[w-1:0]` residual dim
                std::cmp::Ordering::Equal => raw_off,
                std::cmp::Ordering::Greater => {
                    self.norm_sub_k(raw_off, lo.min(i32::MAX as i64) as i32)
                }
                // NEGATIVE declared bound (`[1:0][3:-2]`): `idx − lo` with a
                // negative `lo`, which the SEALED arm expresses directly. The
                // non-sealed arm below still emits `Add |lo|` and still leans on
                // the 32-bit wrap — the wrap is gone only where the seal
                // applies, which is why both arms exist.
                std::cmp::Ordering::Less => {
                    self.norm_sub_k(raw_off, lo.max(i32::MIN as i64) as i32)
                }
            }
        } else {
            // `hi = lo + width - 1` in the DECLARED domain; the subtraction below is on
            // 32-bit values, so a negative `hi` cannot arise for any reachable dim
            // (an ascending dim's `hi` is its larger endpoint).
            let hi = lo.saturating_add(i64::from(width).saturating_sub(1));
            self.norm_k_sub(raw_off, hi.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
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
                    return self.norm_offset_for_range(raw_off, i64::from(lo), w, asc);
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
            return self.norm_offset_for_range(raw_off, i64::from(lo), w, asc);
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
    /// True when a select's base is a net declared with a NEGATIVE low bound.
    ///
    /// Its whole value and its BIT selects are exact (`norm_offset_for_net` maps `x[-1]`
    /// onto the right internal bit), but a PART select folds its own bounds through the
    /// UNSIGNED `const_eval_u32`, where `-2` reads as `0xFFFFFFFE` and trips the
    /// direction check with a message about the wrong thing. Callers use this to say
    /// what is actually unsupported instead.
    pub(crate) fn base_has_neg_decl_lsb(&self, base: &ast::Expr) -> bool {
        self.actual_root_net(base)
            .is_some_and(|n| self.net_decl_neg_lsb.contains_key(&n))
    }

    /// The diagnostic for [`Self::base_has_neg_decl_lsb`], shared by the read and write
    /// part-select paths so they cannot drift.
    pub(crate) fn error_neg_lsb_part_select(&mut self) {
        self.error(
            MsgCode::ElabUnsupported,
            "a PART select of a net declared with a negative low bound \
             (`logic [3:-2] x; x[1:-2]`) is not yet supported — the whole value and \
             single-BIT selects (`x[-1]`) are exact, so select bits individually",
        );
    }

    /// `hi - lo + 1` for a part-select whose bounds both folded — or None when that
    /// is not a believable width, in which case the caller keeps the unfolded arena
    /// tree (its previous behavior) instead of acting on the number.
    ///
    /// Both bounds arrive as `u32` through [`const_eval_u32`], which folds a NEGATIVE
    /// literal by `wrapping_neg` — so `x[-1:0]` presents as `0xFFFF_FFFF - 0 + 1` and
    /// used to overflow (a debug panic; a wrap to a 0-width select in release). The
    /// `MAX_NET_WIDTH` ceiling rejects that class outright: a genuine part-select is
    /// never wider than a net may be, and a wrapped negative bound always lands far
    /// above it. The u64 arithmetic means the check itself cannot overflow.
    fn folded_part_width(hi: u32, lo: u32) -> Option<u32> {
        let w = (hi as u64).checked_sub(lo as u64)?.checked_add(1)?;
        (w <= MAX_NET_WIDTH).then_some(w as u32)
    }

    pub(crate) fn width_from_msb_lsb_dir(
        &mut self,
        msb_ast: &ast::Expr,
        lsb_ast: &ast::Expr,
        msb_id: u32,
        lsb_id: u32,
        ascending: bool,
    ) -> u32 {
        let folded = (self.const_bound_u32(msb_ast), self.const_bound_u32(lsb_ast));
        if let (Some(m), Some(l)) = folded {
            if ascending {
                if m > l {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "part-select bounds [msb:lsb] descend but the net is ascending [lo:hi] (out of order)",
                    );
                } else if let Some(w) = Self::folded_part_width(l, m) {
                    // width = (l - m) + 1, folded; offset handled by norm_offset.
                    return self.const_u32_expr(w, 32);
                }
            } else if m < l {
                self.error(
                    MsgCode::ElabUnsupported,
                    "part-select bounds [msb:lsb] ascend but the net is descending [hi:lo] (out of order)",
                );
            } else if self.const_of_expr_u32(msb_id).is_none()
                || self.const_of_expr_u32(lsb_id).is_none()
            {
                // The bounds ARE constant, but at least one did not LOWER to a
                // foldable `Const` tree (a cast, a constant-function call, `*`, a
                // ternary…), so the `(msb - lsb) + 1` arena tree below would not
                // fold downstream — the read then took the `unwrap_or(1)` fallback
                // (a silent 1-bit part-select) and the write clobbered the whole
                // net above `lsb`. Return the folded width directly.
                //
                // Guarded on the LOWERED form, not on the AST: when both bounds do
                // lower to `Const` (every shape that worked before, literals
                // included) this falls through to the arena tree verbatim, so the
                // golden IR of a design that already worked is byte-identical.
                if let Some(w) = Self::folded_part_width(m, l) {
                    return self.const_u32_expr(w, 32);
                }
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
        let Some(w) = self.const_bound_u32(width) else {
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
        let Some(c) = self.const_bound_u32(offset) else {
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
        let (a, b) = (self.const_bound_u32(msb)?, self.const_bound_u32(lsb)?);
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
        // Signed throughout: `olo` may be a NEGATIVE declared bound (`[1:0][3:-2]`), and
        // the caller's `lo`/`hi` are source indices in that same declared domain.
        let (lo, hi) = (i64::from(lo), i64::from(hi));
        let osize = i64::from(osize);
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
    pub(crate) fn packed_split(dims: &[(i64, u32, bool)]) -> (Vec<(i64, u32)>, Vec<bool>) {
        let ext = dims.iter().map(|&(l, s, _)| (l, s)).collect();
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
