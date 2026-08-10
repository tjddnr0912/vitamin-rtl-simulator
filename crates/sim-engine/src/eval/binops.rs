//! split part of `eval` (mechanical move).

use super::*;

/// The three sign questions one arithmetic operation answers.
///
/// §11.8.1 gives a context-determined region ONE collective sign, and every
/// operator obeys it — except `**`, whose exponent IEEE Table 11-21 makes
/// self-determined and therefore independent of the base. So the single
/// `l.signed && r.signed` boolean was standing in for three separate answers.
/// For every operator but `**` all three are that same boolean, which is why
/// splitting them is mechanically identity everywhere else.
#[derive(Clone, Copy)]
pub(crate) struct ArithSigns {
    /// How to read the LEFT operand.
    lhs: bool,
    /// How to read the RIGHT one.
    rhs: bool,
    /// The RESULT's sign (§11.4.10: `**` is signed iff the BASE is).
    res: bool,
}

/// One operand of the narrow arithmetic lane as an `i128`, read under `sgn`.
///
/// `sgn` is the sign the OPERAND is to be read with, not the region's: a signed
/// value sign-extends from `w`, an unsigned one keeps its non-negative value and
/// is never reinterpreted through bit `w-1`. Callers reach this lane only when
/// `w <= 64`, so the unsigned arm always fits.
///
/// Identical to the old inline `resize_keep_sign(w, true).to_i128_signed()`
/// whenever `sgn` is true — which is every operand of every operator except a
/// `**` whose two sides disagree.
#[inline]
fn read_i128(v: &Value, w: u32, sgn: bool) -> i128 {
    if sgn {
        v.clone()
            .resize_keep_sign(w, true)
            .to_i128_signed()
            .unwrap()
    } else {
        v.clone().resize_keep_sign(w, false).to_u128().unwrap() as i128
    }
}

/// ONE word of the ternary's unknown-condition merge: bits where the two
/// branches agree (in BOTH planes) pass through, bits where they differ become
/// X. A free function so the tier-3 W evaluator asks the same question instead
/// of restating a rule whose whole content is which of the two planes carries
/// the disagreement.
///
/// The X bits are set everywhere the planes differ, INCLUDING above the caller's
/// width — a `w`-bit caller masks. `merge_x` below does that with `top_mask`;
/// the W evaluator does it with the op's own mask.
#[inline]
pub(crate) fn merge_x_word(tv: u64, tu: u64, ev: u64, eu: u64) -> (u64, u64) {
    let eq = !((tv ^ ev) | (tu ^ eu));
    (tv & eq, (tu & eq) | !eq)
}

/// Where a select's bits START and how many there are — the ONE spelling of
/// IEEE §11.5.1's three forms, shared by the generic `eval_select` and the
/// tier-3 W compiler's admission.
///
/// `off` is the offset ALREADY read as an i64 (`to_u64` then `i64::try_from`,
/// which is why an unknown or oversized offset never reaches here) and `width`
/// is the FOLDED constant bit count, not the const-expr edge. The result may be
/// negative or run past the source: deciding what to do about that belongs to
/// the caller (the generic path X-fills; the W compiler declines).
///
/// It is a free function rather than three lines inlined twice because the
/// difference between the forms is exactly one `- width + 1`, and a second copy
/// that dropped it would select the wrong four bits and never say so.
pub(crate) fn select_lsb_width(kind: SelKind, off: i64, width: u32) -> (i64, u32) {
    match kind {
        SelKind::Bit => (off, 1u32),
        SelKind::PartConst | SelKind::PartIdxUp => (off, width),
        SelKind::PartIdxDown => (off - (width as i64) + 1, width),
    }
}

impl<N: NetReader + ?Sized> EvalCtx<'_, N> {
    pub(crate) fn arith(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        if l.is_real || r.is_real {
            // IEEE 1364 §4.3: if either operand is real, the other promotes to real.
            // An X/Z integer entering a mixed real op decays to 0.0 (documented MVP
            // policy), never panics, never X-propagates.
            let a = l.to_f64().unwrap_or(0.0);
            let b = r.to_f64().unwrap_or(0.0);
            let res = match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b, // f64: x/0 → ±inf, 0/0 → NaN; NOT X
                // `%` and `**` on a real are permanent illegalities gated at
                // elaborate (§6.2). Defensive NaN poison instead of unreachable!()
                // so a gate regression can never crash the simulator.
                BinOp::Mod => f64::NAN,
                BinOp::Pow => f64::NAN,
                _ => f64::NAN,
            };
            return Value::from_f64(res);
        }
        let w = l.width.max(r.width).max(1);
        // §11.8.1 gives a context-determined region ONE collective sign, and every
        // operator here obeys it — except `**`. IEEE Table 11-21 makes the exponent
        // SELF-determined, so its sign is independent of the base's, and one boolean
        // cannot answer the three questions it was standing in for: how to read the
        // LEFT operand, how to read the RIGHT one, and what the RESULT's sign is.
        // For every other op the three coincide, so splitting them is mechanically
        // identity (`is_pow` is `false` and all three collapse to `both_signed`).
        let is_pow = matches!(op, BinOp::Pow);
        let both_signed = l.signed && r.signed;
        let sg = if is_pow {
            ArithSigns {
                lhs: l.signed,
                rhs: r.signed,
                res: l.signed,
            }
        } else {
            ArithSigns {
                lhs: both_signed,
                rhs: both_signed,
                res: both_signed,
            }
        };
        // Which KERNEL runs. The signed one owns Table 11-6's negative-exponent
        // rows, which the unsigned square-multiply cannot express — so `**` needs
        // it when the base is signed (for the +/-1 rows) or the exponent actually
        // IS negative. A non-negative exponent reads the same either way (two's
        // complement multiplication is sign-agnostic mod 2^w), so an unsigned base
        // with a non-negative signed exponent stays on the lane it always used.
        let exp_is_neg = is_pow && r.signed && r.width > 0 && r.get_vu(r.width - 1).0 == 1;
        let use_signed = if is_pow {
            l.signed || exp_is_neg
        } else {
            both_signed
        };
        if l.has_xz() || r.has_xz() {
            return Value::xs(w, sg.res);
        }
        // Arithmetic lane: 128 bits. SIGNED stays a 64-bit lane — sign
        // reconstruction (`to_i128_signed`) gates on width≤64, and a >64-bit signed
        // value would read mis-signed; poison to X (an honest "unsupported" beats a
        // silently wrong number). UNSIGNED now spans the full 128-bit u128 lane (so
        // a `[127:0]` add/mul carries past bit 63 correctly); only width>128 — beyond
        // the lane — poisons, mirroring the signed guard rather than truncating.
        // Phase-1.x ⑥: beyond the native lanes (signed >64 / unsigned >128)
        // arithmetic computes EXACTLY on the word grid (iverilog-differential)
        // — these used to X-poison as an honest "unsupported".
        if (use_signed && w > 64) || (!use_signed && w > 128) {
            return self.arith_wide(op, l, r, w, sg);
        }
        let res: u128 = if use_signed {
            let a = read_i128(l, w, sg.lhs);
            let b = read_i128(r, w, sg.rhs);
            match op {
                BinOp::Add => a.wrapping_add(b) as u128,
                BinOp::Sub => a.wrapping_sub(b) as u128,
                BinOp::Mul => a.wrapping_mul(b) as u128,
                BinOp::Div => {
                    if b == 0 {
                        return Value::xs(w, true);
                    }
                    a.wrapping_div(b) as u128
                }
                BinOp::Mod => {
                    if b == 0 {
                        return Value::xs(w, true);
                    }
                    a.wrapping_rem(b) as u128
                }
                BinOp::Pow => ipow_signed(a, b),
                _ => unreachable!(),
            }
        } else {
            let a = l.to_u128().unwrap();
            let b = r.to_u128().unwrap();
            match op {
                BinOp::Add => a.wrapping_add(b),
                BinOp::Sub => a.wrapping_sub(b),
                BinOp::Mul => a.wrapping_mul(b),
                BinOp::Div => {
                    if b == 0 {
                        return Value::xs(w, false);
                    }
                    a / b
                }
                BinOp::Mod => {
                    if b == 0 {
                        return Value::xs(w, false);
                    }
                    a % b
                }
                // Square-and-multiply mod 2^128 (then masked to `w`) — a^n WRAPS
                // mod 2^w like iverilog, instead of the old `checked_pow(..)
                // .unwrap_or(0)` that returned 0 on u128 overflow (`64'hF..F ** 3`
                // is all-ones, not 0). For w*n <= 128 the value never wraps, so
                // this is byte-identical to the old result there.
                BinOp::Pow => {
                    let mut acc: u128 = 1;
                    let mut base = a;
                    let mut e = b;
                    while e > 0 {
                        if e & 1 == 1 {
                            acc = acc.wrapping_mul(base);
                        }
                        e >>= 1;
                        if e > 0 {
                            base = base.wrapping_mul(base);
                        }
                    }
                    acc
                }
                _ => unreachable!(),
            }
        };
        // Store the low 128 bits across word 0 (and word 1 for w>64); `mask_top`
        // clears bits above `w`.
        let mut out = Value::zeros(w, sg.res);
        out.val[0] = res as u64;
        if nwords(w) > 1 {
            if out.val.len() < 2 {
                out.val.resize(2, 0);
            }
            out.val[1] = (res >> 64) as u64;
        }
        out.mask_top();
        out
    }

    /// Multi-word arithmetic (Phase-1.x ⑥) for widths beyond the native
    /// lanes. Operands are X-free (gated by the caller); each extends to the
    /// w-bit grid under ITS OWN sign (`sg.lhs`/`sg.rhs`, which the caller sets
    /// equal to the collective sign for every operator but `**`) and every
    /// op computes mod 2^w in two's complement — school multiplication,
    /// short (one-word divisor) or restoring long division, square-multiply
    /// power. Division signs per IEEE: quotient truncates toward zero, the
    /// remainder takes the DIVIDEND's sign.
    pub(crate) fn arith_wide(
        &self,
        op: BinOp,
        l: &Value,
        r: &Value,
        w: u32,
        sg: ArithSigns,
    ) -> Value {
        // WIDE-ARITH-CAP: the super-linear kernels would stall for tens of seconds
        // once a replication concat pushes an operand past the cap. Poison to X
        // above it (the div-by-zero degrade precedent); Add/Sub are O(n) and stay
        // exact at any width. The matching loud warning is emitted once in
        // `simulate` (W-RUN-WIDE-ARITH) so the degradation is never silent.
        if w > WIDE_ARITH_CAP && matches!(op, BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow) {
            return Value::xs(w, sg.res);
        }
        let n = nwords(w).max(1);
        let le = l.clone().resize_keep_sign(w, sg.lhs);
        let re = r.clone().resize_keep_sign(w, sg.rhs);
        let a: Vec<u64> = (0..n)
            .map(|k| le.val.get(k).copied().unwrap_or(0))
            .collect();
        let b: Vec<u64> = (0..n)
            .map(|k| re.val.get(k).copied().unwrap_or(0))
            .collect();
        let sa = sg.lhs && le.get_vu(w - 1).0 == 1;
        let sb = sg.rhs && re.get_vu(w - 1).0 == 1;
        let words = match op {
            BinOp::Add => mw_mask(mw_add(&a, &b), w),
            BinOp::Sub => mw_mask(mw_add(&a, &mw_neg(&b)), w),
            BinOp::Mul => mw_mask(mw_mul(&a, &b, n), w),
            BinOp::Div | BinOp::Mod => {
                if mw_is_zero(&b) {
                    return Value::xs(w, sg.res);
                }
                let ma = if sa {
                    mw_mask(mw_neg(&a), w)
                } else {
                    a.clone()
                };
                let mb = if sb {
                    mw_mask(mw_neg(&b), w)
                } else {
                    b.clone()
                };
                let (q, rem) = mw_divmod(&ma, &mb);
                if op == BinOp::Div {
                    let neg = sa != sb;
                    mw_mask(if neg { mw_neg(&q) } else { q }, w)
                } else {
                    mw_mask(if sa { mw_neg(&rem) } else { rem }, w)
                }
            }
            BinOp::Pow => {
                if sb {
                    // negative exponent (IEEE 1364 table): 1 → 1; -1 → ±1 by
                    // exponent parity; 0 → X; |base| > 1 → 0.
                    let one = mw_one(n);
                    let minus_one = mw_mask(mw_neg(&one), w);
                    if a == one {
                        one
                    } else if sg.lhs && a == minus_one {
                        if b[0] & 1 == 0 {
                            one
                        } else {
                            minus_one
                        }
                    } else if mw_is_zero(&a) {
                        return Value::xs(w, sg.res);
                    } else {
                        vec![0; n]
                    }
                } else {
                    mw_pow(&a, &b, w)
                }
            }
            _ => unreachable!(),
        };
        let mut out = Value::zeros(w, sg.res);
        for (k, &wd) in words.iter().enumerate().take(n) {
            out.val[k] = wd;
        }
        out.mask_top();
        out
    }

    pub(crate) fn relational(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        relational(op, l, r)
    }

    pub(crate) fn log_eq(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        log_eq(op, l, r)
    }
}

/// The ordered comparisons, as a FREE function so the tier-3 width-specialized
/// evaluator calls the same spelling the generic evaluator does (§4.5.302's
/// rule read forwards: specialize the EVALUATION, never the semantics). It
/// never used `self`; the method above delegates, so every prior call site is
/// byte-identical.
pub(crate) fn relational(op: BinOp, l: &Value, r: &Value) -> Value {
    if l.is_real || r.is_real {
        let a = l.to_f64().unwrap_or(0.0);
        let b = r.to_f64().unwrap_or(0.0);
        let bit = match (op, a.partial_cmp(&b)) {
            // partial_cmp is None on NaN → all ordered comparisons false (IEEE).
            (_, None) => false,
            (BinOp::Lt, Some(o)) => o == std::cmp::Ordering::Less,
            (BinOp::Le, Some(o)) => o != std::cmp::Ordering::Greater,
            (BinOp::Gt, Some(o)) => o == std::cmp::Ordering::Greater,
            (BinOp::Ge, Some(o)) => o != std::cmp::Ordering::Less,
            _ => unreachable!("relational only handles Lt/Le/Gt/Ge"),
        };
        return Value::logic(bit);
    }
    if l.has_xz() || r.has_xz() {
        return Value::x1();
    }
    // Exact word-wise compare at ANY width (no 64/128-bit lane): extend both
    // operands to the common width (§4.5: sign-extend only when BOTH signed),
    // then compare. For equal-width same-sign two's-complement values the
    // plain lexicographic word order IS the numeric order; differing sign
    // bits decide directly. Fixes the silent low-word truncation (P0-1).
    use std::cmp::Ordering::*;
    let w = l.width.max(r.width).max(1);
    let both_signed = l.signed && r.signed;
    let le = l.clone().resize_keep_sign(w, both_signed);
    let re = r.clone().resize_keep_sign(w, both_signed);
    let cmp_words = |a: &Value, b: &Value| {
        let n = a.val.len().max(b.val.len());
        for k in (0..n).rev() {
            let av = a.val.get(k).copied().unwrap_or(0);
            let bv = b.val.get(k).copied().unwrap_or(0);
            match av.cmp(&bv) {
                Equal => continue,
                o => return o,
            }
        }
        Equal
    };
    let ord = if both_signed {
        match (le.get_vu(w - 1).0, re.get_vu(w - 1).0) {
            (1, 0) => Less,
            (0, 1) => Greater,
            _ => cmp_words(&le, &re),
        }
    } else {
        cmp_words(&le, &re)
    };
    let b = matches!(
        (op, ord),
        (BinOp::Lt, Less)
            | (BinOp::Le, Less)
            | (BinOp::Le, Equal)
            | (BinOp::Gt, Greater)
            | (BinOp::Ge, Greater)
            | (BinOp::Ge, Equal)
    );
    Value::logic(b)
}

/// `==` / `!=`: a bit pair that is BOTH known and differing decides the
/// comparison (definite inequality → `==`=0 / `!=`=1) even when OTHER bits
/// are x/z; only an AMBIGUOUS compare (some x/z, no definite mismatch) is X
/// (IEEE §11.4.5 "if the relation is ambiguous" — iverilog-pinned:
/// `4'b1x00 == 4'b0000` is 0, not x).
///
/// Width unification follows IEEE 1364-2001 §4.5: the comparison is signed
/// ONLY when BOTH operands are signed; if either is unsigned both operands
/// zero-extend. Using `resize` (which honors each operand's *own* sign) would
/// sign-extend a lone signed operand in an unsigned context and report a false
/// match (e.g. `4'sb1111 == 8'hFF` → wrong `1`). `resize_keep_sign` clears the
/// sign when the context is unsigned, so we zero-extend correctly.
pub(crate) fn log_eq(op: BinOp, l: &Value, r: &Value) -> Value {
    {
        if l.is_real || r.is_real {
            let a = l.to_f64().unwrap_or(0.0);
            let b = r.to_f64().unwrap_or(0.0);
            // VALUE comparison: +0.0 == -0.0 is true; NaN != NaN.
            let eq = a == b;
            return Value::logic(if op == BinOp::Eq { eq } else { !eq });
        }
        let w = l.width.max(r.width);
        let ctx_signed = l.signed && r.signed;
        let le = l.clone().resize_keep_sign(w, ctx_signed);
        let re = r.clone().resize_keep_sign(w, ctx_signed);
        // Word-parallel (was a per-bit `get_vu` loop): any x/z on either side
        // (`unk`) poisons the result to X; otherwise compare the val planes.
        // `resize_keep_sign` canonicalizes both operands (planes masked past
        // `width`), so a word-wise scan is bit-exact for the live width.
        let mut unk = 0u64;
        let mut definite = 0u64;
        for k in 0..nwords(w) {
            let lu = le.unk.get(k).copied().unwrap_or(0);
            let ru = re.unk.get(k).copied().unwrap_or(0);
            let lv = le.val.get(k).copied().unwrap_or(0);
            let rv = re.val.get(k).copied().unwrap_or(0);
            let u = lu | ru;
            unk |= u;
            // both-known differing bit — the val plane of an x/z bit never
            // counts (z encodes val=1, so `& !u` is required, not cosmetic).
            definite |= (lv ^ rv) & !u;
        }
        if definite != 0 {
            return Value::logic(op == BinOp::Ne); // definite inequality
        }
        if unk != 0 {
            return Value::x1(); // ambiguous: no definite mismatch, some x/z
        }
        Value::logic(op == BinOp::Eq)
    }
}

impl<N: NetReader + ?Sized> EvalCtx<'_, N> {
    /// `===` / `!==`: exact 4-state per-bit compare, never X. Width unification
    /// uses the same context-signedness rule as `==` (zero-extend unless BOTH
    /// signed) so a mixed-sign `===` matches IEEE numeric extension.
    pub(crate) fn case_eq(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        case_eq(op, l, r)
    }

    /// v7 casez/casex per-label match (IEEE 1364 §9.5.1, live-pinned against
    /// iverilog). A bit is don't-care iff EITHER side is z (`CasezEq`) or
    /// x-or-z (`CasexEq`); every remaining position compares 4-state EXACT
    /// (val AND unk planes equal — so an explicit x in a casez label matches
    /// only an x). Word-parallel; the result is always known 0/1.
    /// Encoding reminder: x = (val 0, unk 1), z = (val 1, unk 1).
    pub(crate) fn casez_eq(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        let n = nwords(l.width.max(r.width)).max(1);
        for k in 0..n {
            let lv = l.val.get(k).copied().unwrap_or(0);
            let lu = l.unk.get(k).copied().unwrap_or(0);
            let rv = r.val.get(k).copied().unwrap_or(0);
            let ru = r.unk.get(k).copied().unwrap_or(0);
            let dc = if op == BinOp::CasezEq {
                (lu & lv) | (ru & rv) // z on either side
            } else {
                lu | ru // x OR z on either side
            };
            // 4-state exact mismatch on a non-don't-care position. mask_top
            // keeps both planes zero past `width`, so no spurious top bits.
            if !dc & ((lv ^ rv) | (lu ^ ru)) != 0 {
                return Value::logic(false);
            }
        }
        Value::logic(true)
    }

    pub(crate) fn log_and(&self, l: &Value, r: &Value) -> Value {
        log_and(l, r)
    }

    pub(crate) fn log_or(&self, l: &Value, r: &Value) -> Value {
        log_or(l, r)
    }

    pub(crate) fn shift_left(&self, l: &Value, r: &Value) -> Value {
        if r.has_xz() {
            return Value::xs(l.width, l.signed);
        }
        // An amount that doesn't fit u64 is astronomically larger than any net
        // width — saturate so everything shifts out (was: silent low-word use).
        let amt = r.to_u64().unwrap_or(u64::MAX);
        // v1 has no context-determined width (elaborate defers expr sizing), so a
        // self-determined `<<` would truncate to `l.width` and drop bits that a
        // wider assignment context would keep (`4'b0001 << 5` → 0 instead of
        // `8'h20`). We GROW the result to `l.width + amt` so no bit is ever lost;
        // the enclosing `write_lvalue`/operator then truncates to the real LHS
        // width. This is lossless and matches any context at least that wide;
        // narrower contexts truncate identically either way. Cap the growth so a
        // pathological shift amount can't allocate unboundedly.
        let grow = (l.width as u64).saturating_add(amt).min(4096) as u32;
        let w = grow.max(l.width).max(1);
        l.shl_grow(amt, w) // word-parallel (vacated low bits = 0)
    }

    pub(crate) fn shift_right(&self, l: &Value, r: &Value, arith: bool) -> Value {
        if r.has_xz() {
            return Value::xs(l.width, l.signed);
        }
        // Over-u64 amount ⇒ saturate (shift everything out / full sign fill).
        let amt = r.to_u64().unwrap_or(u64::MAX);
        let w = l.width;
        let (fv, fu) = if arith && w > 0 {
            l.get_vu(w - 1)
        } else {
            (0, 0)
        };
        l.shr_fill(amt, w, fv, fu) // word-parallel (top fill = sign for arith, else 0)
    }

    // ── Ternary ────────────────────────────────────────────────────────────

    /// Merge two equal-width branches bit-by-bit: agreeing bits pass through,
    /// differing bits become X. Both `t`/`e` are already `w`-wide from
    /// `eval_ctx`, so no inner resize is needed (verbatim former eval_ternary
    /// unknown-branch body).
    pub(crate) fn merge_x(&self, t: &Value, e: &Value, w: u32, signed: bool) -> Value {
        // WORD-PARALLEL X-merge (§A word化, 2026-06-11 — was bit-serial):
        // a result bit keeps the operand bit where BOTH planes agree and
        // X-poisons where they differ. Bits beyond an operand's width read
        // as (0,0), exactly like the old `get_vu` path (mask_top invariant).
        let mut out = Value::zeros(w, signed);
        let n = crate::value::nwords(w).max(1);
        for k in 0..n {
            let tv = t.val.get(k).copied().unwrap_or(0);
            let tu = t.unk.get(k).copied().unwrap_or(0);
            let ev = e.val.get(k).copied().unwrap_or(0);
            let eu = e.unk.get(k).copied().unwrap_or(0);
            let (v, u) = merge_x_word(tv, tu, ev, eu);
            out.val[k] = v;
            out.unk[k] = u;
        }
        let m = crate::value::top_mask(w);
        out.val[n - 1] &= m;
        out.unk[n - 1] &= m;
        out
    }

    // ── Concat / Replicate ─────────────────────────────────────────────────

    pub(crate) fn eval_concat(&self, parts: &[u32]) -> Value {
        let vals: Vec<Value> = parts.iter().map(|&p| self.eval(p)).collect();
        let total: u32 = vals.iter().map(|v| v.width).sum();
        let mut out = Value::zeros(total.max(1), false);
        out.width = total;
        // parts[0] is MSB-most; fill from the top down — word-parallel copy
        // (§A word化, 2026-06-11; was a per-bit set_vu loop).
        let mut pos = total;
        for v in &vals {
            pos -= v.width;
            copy_bits(&mut out, pos, v, 0, v.width);
        }
        out.mask_top();
        out
    }

    pub(crate) fn eval_replicate(&self, count: u32, value: u32) -> Value {
        // `count` is an ExprId (frozen IR: Replicate.count is a const-expr edge),
        // NOT a literal — fold it to the repeat count, symmetric with the
        // self-width table (width.rs) and eval_select's width fold.
        let count = crate::width::const_u32_of_expr(self.ir, count).unwrap_or(0);
        let v = self.eval(value);
        let total = v.width.saturating_mul(count);
        let mut out = Value::zeros(total.max(1), false);
        out.width = total;
        // word-parallel per repetition (§A word化, 2026-06-11).
        for c in 0..count {
            copy_bits(&mut out, c * v.width, &v, 0, v.width);
        }
        out.mask_top();
        out
    }

    // ── Select ─────────────────────────────────────────────────────────────

    pub(crate) fn eval_select(&self, base: u32, offset: u32, width: u32, kind: SelKind) -> Value {
        // `width` is an ExprId (frozen IR: `Select.width` is a const-expr edge,
        // e.g. `Add(Sub(msb,lsb),1)`), NOT a literal bit count — fold it to its
        // value. `offset` stays an evaluated expr (it is the runtime index for
        // indexed `[base +: w]`/`[base -: w]` selects).
        let width = crate::width::const_u32_of_expr(self.ir, width).unwrap_or(1);
        let src = self.eval(base);
        let off_val = self.eval(offset);
        let off = match off_val.to_u64().and_then(|o| i64::try_from(o).ok()) {
            Some(o) => o,
            // X/Z offset or one beyond the i64 lane: the select is out of range.
            None => return Value::xs(width.max(1), false),
        };
        let (lsb, w) = select_lsb_width(kind, off, width);
        let mut out = Value::zeros(w.max(1), false);
        out.width = w;
        // Fully in-range select: ONE word-parallel copy (§A word化,
        // 2026-06-11 — the dominant case). Any out-of-range overlap keeps
        // the per-bit path (mixed copied/X-filled bits).
        if lsb >= 0 && (lsb as u64) + (w as u64) <= src.width as u64 {
            copy_bits(&mut out, 0, &src, lsb as u32, w);
            out.mask_top();
            return out;
        }
        for i in 0..w as i64 {
            let src_idx = lsb + i;
            if src_idx >= 0 && (src_idx as u32) < src.width {
                let (v, u) = src.get_vu(src_idx as u32);
                out.set_vu(i as u32, v, u);
            } else {
                out.set_vu(i as u32, 0, 1); // out-of-range read → X
            }
        }
        out.mask_top();
        out
    }
}

/// `===`/`!==`, `&&`, `||` as FREE functions so the tier-3 width-specialized evaluator
/// calls the same spelling the generic evaluator does (§4.5.302's rule: specialize the
/// EVALUATION, never the semantics). None used `self`; the methods above delegate, so
/// every prior call site is byte-identical.
pub(crate) fn case_eq(op: BinOp, l: &Value, r: &Value) -> Value {
    if l.is_real || r.is_real {
        // MVP: === on real == VALUE equality. A real is 2-state, so === and ==
        // coincide; +0.0 === -0.0 is TRUE (value equal), NaN !== NaN.
        let a = l.to_f64().unwrap_or(0.0);
        let b = r.to_f64().unwrap_or(0.0);
        let eq = a == b;
        return Value::logic(if op == BinOp::CaseEq { eq } else { !eq });
    }
    let w = l.width.max(r.width);
    let ctx_signed = l.signed && r.signed;
    let le = l.clone().resize_keep_sign(w, ctx_signed);
    let re = r.clone().resize_keep_sign(w, ctx_signed);
    // Word-parallel exact 4-state compare (both planes), canonical after
    // `resize_keep_sign`; was a per-bit `get_vu` loop.
    let mut neq = 0u64;
    for k in 0..nwords(w) {
        let lv = le.val.get(k).copied().unwrap_or(0);
        let rv = re.val.get(k).copied().unwrap_or(0);
        let lu = le.unk.get(k).copied().unwrap_or(0);
        let ru = re.unk.get(k).copied().unwrap_or(0);
        neq |= (lv ^ rv) | (lu ^ ru);
    }
    let eq = neq == 0;
    Value::logic(if op == BinOp::CaseEq { eq } else { !eq })
}

pub(crate) fn log_and(l: &Value, r: &Value) -> Value {
    match (
        crate::eval::sysfunc::truthiness(l),
        crate::eval::sysfunc::truthiness(r),
    ) {
        (Tri::False, _) | (_, Tri::False) => Value::zeros(1, false),
        (Tri::True, Tri::True) => Value::one1(),
        _ => Value::x1(),
    }
}

pub(crate) fn log_or(l: &Value, r: &Value) -> Value {
    match (
        crate::eval::sysfunc::truthiness(l),
        crate::eval::sysfunc::truthiness(r),
    ) {
        (Tri::True, _) | (_, Tri::True) => Value::one1(),
        (Tri::False, Tri::False) => Value::zeros(1, false),
        _ => Value::x1(),
    }
}
