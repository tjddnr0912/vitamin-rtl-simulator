//! split part of `native_eval` (mechanical move).

use super::*;

/// `low_mask` at 128 bits.
#[inline]
pub(crate) fn wmask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// Stack effect of one op on (narrow, wide): (npop, npush, wpop, wpush).
pub(crate) fn arity(op: &NOp) -> (u32, u32, u32, u32) {
    match op {
        NOp::Const { .. } | NOp::LoadScalar { .. } => (0, 1, 0, 0),
        NOp::LoadIndexed { .. } => (1, 1, 0, 0),
        NOp::Not { .. }
        | NOp::Neg { .. }
        | NOp::Reduce { .. }
        | NOp::LogNot { .. }
        | NOp::Repl { .. } => (1, 1, 0, 0),
        NOp::Ternary { .. } => (3, 1, 0, 0),
        // wide lane
        NOp::Promote => (1, 0, 0, 1),
        NOp::WConst { .. } | NOp::WLoadScalar { .. } => (0, 0, 0, 1),
        NOp::WLoadIndexed { .. } => (1, 0, 0, 1),
        NOp::WArith { .. } | NOp::WBitwise { .. } | NOp::WDivMod { .. } => (0, 0, 2, 1),
        NOp::WNot { .. } | NOp::WNeg { .. } => (0, 0, 1, 1),
        NOp::WCmp { .. } | NOp::WEqNe { .. } | NOp::WCaseEqNe { .. } => (0, 1, 2, 0),
        NOp::WShl { .. } | NOp::WShr { .. } => (1, 0, 1, 1),
        NOp::WTernary { cond_wide, .. } => {
            if *cond_wide {
                (0, 0, 3, 1)
            } else {
                (1, 0, 2, 1)
            }
        }
        NOp::WReduce { .. } | NOp::WLogNot { .. } => (0, 1, 1, 0),
        // v6 ④ wide structural trio
        NOp::WSelect {
            base_wide,
            out_wide,
            ..
        } => (
            1 + u32::from(!base_wide),
            u32::from(!out_wide),
            u32::from(*base_wide),
            u32::from(*out_wide),
        ),
        NOp::WConcatPair {
            acc_wide,
            part_wide,
            ..
        } => (
            u32::from(!acc_wide) + u32::from(!part_wide),
            0,
            u32::from(*acc_wide) + u32::from(*part_wide),
            1,
        ),
        NOp::WRepl { .. } => (1, 0, 0, 1),
        // remaining narrow binaries (Arith/Bitwise/Cmp/EqNe/CaseEqNe/Shl/Shr/
        // DivMod/LogBin/Select/ConcatPair)
        _ => (2, 1, 0, 0),
    }
}

/// `eval_const` minus the real lane (we bail on real). Returns the const's natural
/// `Value` (pre-resize); `None` for a real const.
pub(crate) fn const_value(ir: &SimIr, cid: u32) -> Option<Value> {
    let c = &ir.consts[cid as usize];
    if matches!(c.repr, ConstRepr::Real) {
        return None;
    }
    let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
    Some(Value::from_packed(&c.bits, c.width, signed))
}

/// Extract a `Value`'s low 128 bits as a masked u128 pair (wide registers).
pub(crate) fn wide_pair(v: &Value, w: u32) -> (u128, u128) {
    let m = wmask(w);
    let vlo = v.val.first().copied().unwrap_or(0) as u128;
    let vhi = v.val.get(1).copied().unwrap_or(0) as u128;
    let ulo = v.unk.first().copied().unwrap_or(0) as u128;
    let uhi = v.unk.get(1).copied().unwrap_or(0) as u128;
    ((vlo | (vhi << 64)) & m, (ulo | (uhi << 64)) & m)
}

/// Bridge a narrow 1-bit/natural producer into a wide context (plain
/// zero-extend — narrow registers keep bits ≥ node width 0).
pub(crate) fn promote_if(wide: bool, ops: &mut Vec<NOp>) {
    if wide {
        ops.push(NOp::Promote);
    }
}

pub(crate) fn classify_binop(op: BinOp) -> Option<BinClass> {
    Some(match op {
        BinOp::Add => BinClass::Arith(ArithKind::Add),
        BinOp::Sub => BinClass::Arith(ArithKind::Sub),
        BinOp::Mul => BinClass::Arith(ArithKind::Mul),
        BinOp::BitAnd => BinClass::Bit(BitKind::And),
        BinOp::BitOr => BinClass::Bit(BitKind::Or),
        BinOp::BitXor => BinClass::Bit(BitKind::Xor),
        BinOp::BitXnor => BinClass::Bit(BitKind::Xnor),
        _ => return None,
    })
}

/// POW-LANE: `Some(n)` iff `eid` is a `Const` exponent that is X-free and a small
/// integer `2..=POW_MAX`. A non-const, X-bearing, <2, negative, or too-large
/// exponent returns `None` (the `**` node then stays oracle-bound). Exponent 1 is
/// excluded on purpose: `a**1` is `a` natively, but the oracle routes it through
/// `arith(Pow,…)` which X-poisons ANY X bit to all-X — only a chain with an actual
/// Mul (n>=2) reproduces that, so n==1 is left to the oracle.
pub(crate) fn const_pow_exponent(ir: &SimIr, eid: u32) -> Option<u32> {
    let Expr::Const { val } = ir.exprs.get(eid as usize)? else {
        return None;
    };
    let v = const_value(ir, *val)?; // None for a real const
    if v.has_xz() {
        return None;
    }
    let n = v.to_u128()?; // a negative signed const reads huge here → rejected below
    if (2..=POW_MAX).contains(&n) {
        Some(n as u32)
    } else {
        None
    }
}

/// Try to compile `eid` evaluated in context `(ctx_width, ctx_signed)` — the SAME
/// context `eval_for_lvalue` passes (`ctx_width = max(lvalue_w, self_w(rhs))`,
/// `ctx_signed = rhs self-sign`). `None` ⇒ outside the supported subset ⇒ fall back.
pub(crate) fn try_compile(
    ir: &SimIr,
    wt: &WidthTable,
    eid: u32,
    ctx_width: u32,
    ctx_signed: bool,
) -> Option<NativeProg> {
    let self_sw = wt.get(eid);
    let root_w = self_sw.width.max(ctx_width);
    if root_w == 0 || root_w > 128 {
        return None;
    }
    let mut ops = Vec::new();
    lower(ir, wt, eid, ctx_width, ctx_signed, &mut ops)?;
    // P3-5: verify the post-order program fits BOTH fixed run-time stacks.
    let (mut nbal, mut wbal): (u32, u32) = (0, 0);
    let (mut nmax, mut wmax): (u32, u32) = (0, 0);
    for op in &ops {
        let (npop, npush, wpop, wpush) = arity(op);
        nbal = nbal.checked_sub(npop)?; // malformed program defends as a bail
        wbal = wbal.checked_sub(wpop)?;
        nbal += npush;
        wbal += wpush;
        nmax = nmax.max(nbal);
        wmax = wmax.max(wbal);
    }
    if nmax as usize > NATIVE_STACK || wmax as usize > WIDE_STACK {
        return None; // absurdly right-leaning nesting: leave it to the oracle
    }
    // At the root, `ctx_signed` IS `rhs` self-sign (eval_for_lvalue), so
    // `eff_signed = self_signed && ctx_signed == ctx_signed`, and the arith/
    // bitwise/compare arms stamp the result `signed = eff_signed` = `ctx_signed`.
    // The STRUCTURAL arms are the exception: the oracle finishes them with
    // `resize_keep_sign(w, false)` (select/concat/replicate are unsigned by
    // definition), which stamps `signed = false` REGARDLESS of context — mirror
    // that so a structural root matches the oracle for any caller-provided ctx.
    let root_signed = match ir.exprs.get(eid as usize) {
        Some(Expr::Select { .. } | Expr::Concat { .. } | Expr::Replicate { .. }) => false,
        _ => ctx_signed,
    };
    Some(NativeProg {
        ops,
        root_w,
        root_signed,
        needs_wide: wmax > 0, // VM-WIDEZERO
    })
}

/// Post-order lowering mirroring `eval_ctx`'s context propagation. Returns `None`
/// (bailing the WHOLE expression) on any unsupported node or any node width > 64.
pub(crate) fn lower(
    ir: &SimIr,
    wt: &WidthTable,
    eid: u32,
    ctx_width: u32,
    ctx_signed: bool,
    ops: &mut Vec<NOp>,
) -> Option<()> {
    let self_sw = wt.get(eid);
    let w = self_sw.width.max(ctx_width);
    let eff_signed = self_sw.signed && ctx_signed;
    if w == 0 || w > 128 {
        return None;
    }
    // C6: a node's register lives on the wide stack IFF its eval width > 64.
    let wide = w > 64;
    // P2-8: a same-schema corrupted `.velab` can carry an out-of-range ExprId;
    // bail to the interpreter (which raises its own diagnostics) over panicking.
    let expr = ir.exprs.get(eid as usize)?;
    match expr {
        Expr::Const { val } => {
            let r = const_value(ir, *val)?.resize_keep_sign(w, eff_signed);
            if wide {
                let (cv, cu) = wide_pair(&r, w);
                ops.push(NOp::WConst { val: cv, unk: cu });
            } else {
                let m = low_mask(w);
                ops.push(NOp::Const {
                    val: r.val.first().copied().unwrap_or(0) & m,
                    unk: r.unk.first().copied().unwrap_or(0) & m,
                });
            }
            Some(())
        }
        Expr::Signal { net, word } => {
            if let Some(weid) = word {
                // v5 ⑤/v6: assoc keys are SIGNED-i64 (or byte-string) domain
                // — the u32 LoadIndexed funnel cannot carry them (a negative
                // key would sentinel to X while the interpreter reads the
                // element). Stay oracle-bound (eval_ctx fallback).
                if matches!(
                    ir.nets.get(*net as usize).map(|n| n.kind),
                    Some(sim_ir::NetKind::Assoc | sim_ir::NetKind::AssocStr)
                ) {
                    return None;
                }
                // dynamic array-word read: index is SELF-determined (oracle
                // `self.eval(weid)`); a wide index stays oracle-bound.
                let iw = wt.get(*weid).width;
                if iw == 0 || iw > 64 {
                    return None;
                }
                lower(ir, wt, *weid, 0, true, ops)?;
                ops.push(if wide {
                    NOp::WLoadIndexed {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                } else {
                    NOp::LoadIndexed {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                });
            } else {
                ops.push(if wide {
                    NOp::WLoadScalar {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                } else {
                    NOp::LoadScalar {
                        net: *net,
                        w,
                        signed: eff_signed,
                    }
                });
            }
            Some(())
        }
        Expr::Unary { op, operand } => match op {
            // context-determined unary: propagate (w, eff_signed) into the operand.
            UnOp::Plus => lower(ir, wt, *operand, w, eff_signed, ops), // passthrough
            UnOp::Minus => {
                lower(ir, wt, *operand, w, eff_signed, ops)?;
                ops.push(if wide {
                    NOp::WNeg { w }
                } else {
                    NOp::Neg { w }
                });
                Some(())
            }
            UnOp::BitNot => {
                lower(ir, wt, *operand, w, eff_signed, ops)?;
                ops.push(if wide {
                    NOp::WNot { w }
                } else {
                    NOp::Not { w }
                });
                Some(())
            }
            // reductions / lognot: 1-bit result over a SELF-DETERMINED operand
            // (lower with ctx_width 0 / ctx_signed true ⇒ (self_w, self_signed),
            // exactly the oracle's `self.eval(operand)`), then zero-extend (free:
            // the register's upper bits are already 0 and parents mask).
            UnOp::LogNot => {
                let opw = wt.get(*operand).width;
                lower(ir, wt, *operand, 0, true, ops)?;
                ops.push(if opw > 64 {
                    NOp::WLogNot { opw }
                } else {
                    NOp::LogNot { opw }
                });
                promote_if(wide, ops);
                Some(())
            }
            UnOp::RedAnd
            | UnOp::RedNand
            | UnOp::RedOr
            | UnOp::RedNor
            | UnOp::RedXor
            | UnOp::RedXnor => {
                let opw = wt.get(*operand).width;
                lower(ir, wt, *operand, 0, true, ops)?;
                let (kind, neg) = match op {
                    UnOp::RedAnd => (RedK::And, false),
                    UnOp::RedNand => (RedK::And, true),
                    UnOp::RedOr => (RedK::Or, false),
                    UnOp::RedNor => (RedK::Or, true),
                    UnOp::RedXor => (RedK::Xor, false),
                    _ => (RedK::Xor, true), // RedXnor
                };
                ops.push(if opw > 64 {
                    NOp::WReduce { kind, neg, opw }
                } else {
                    NOp::Reduce { kind, neg, opw }
                });
                promote_if(wide, ops);
                Some(())
            }
        },
        Expr::Binary { op, lhs, rhs } => {
            use BinOp as B;
            match op {
                // COMPARISONS: operands mutually context-determined at
                // (max(self_w(L), self_w(R)), bothsigned); 1-bit result.
                B::Lt | B::Le | B::Gt | B::Ge | B::Eq | B::Ne | B::CaseEq | B::CaseNe => {
                    let cmp_w = wt.get(*lhs).width.max(wt.get(*rhs).width);
                    if cmp_w == 0 || cmp_w > 128 {
                        return None;
                    }
                    let cmp_wide = cmp_w > 64;
                    let pair_signed = wt.get(*lhs).signed && wt.get(*rhs).signed;
                    lower(ir, wt, *lhs, cmp_w, pair_signed, ops)?;
                    lower(ir, wt, *rhs, cmp_w, pair_signed, ops)?;
                    let cmp = |kind| {
                        if cmp_wide {
                            NOp::WCmp {
                                kind,
                                w: cmp_w,
                                signed: pair_signed,
                            }
                        } else {
                            NOp::Cmp {
                                kind,
                                w: cmp_w,
                                signed: pair_signed,
                            }
                        }
                    };
                    let eq = |ne| {
                        if cmp_wide {
                            NOp::WEqNe { ne, w: cmp_w }
                        } else {
                            NOp::EqNe { ne, w: cmp_w }
                        }
                    };
                    let ceq = |ne| {
                        if cmp_wide {
                            NOp::WCaseEqNe { ne, w: cmp_w }
                        } else {
                            NOp::CaseEqNe { ne, w: cmp_w }
                        }
                    };
                    ops.push(match op {
                        B::Lt => cmp(CmpKind::Lt),
                        B::Le => cmp(CmpKind::Le),
                        B::Gt => cmp(CmpKind::Gt),
                        B::Ge => cmp(CmpKind::Ge),
                        B::Eq => eq(false),
                        B::Ne => eq(true),
                        B::CaseEq => ceq(false),
                        _ => ceq(true), // CaseNe
                    });
                    promote_if(wide, ops);
                    Some(())
                }
                // LOGICAL: each operand self-determined, tri-valued combine
                // (wide operands stay oracle-bound — `c = a128 && b` is rare).
                B::LogAnd | B::LogOr => {
                    let lw = wt.get(*lhs).width;
                    let rw = wt.get(*rhs).width;
                    if lw > 64 || rw > 64 {
                        return None;
                    }
                    lower(ir, wt, *lhs, 0, true, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(NOp::LogBin {
                        and: matches!(op, B::LogAnd),
                        lw,
                        rw,
                    });
                    promote_if(wide, ops);
                    Some(())
                }
                // SHIFTS: LEFT context-determined; amount SELF-determined
                // (a >64-bit amount register stays oracle-bound).
                B::Shl | B::AShl => {
                    if wt.get(*rhs).width > 64 {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(if wide {
                        NOp::WShl { w }
                    } else {
                        NOp::Shl { w }
                    });
                    Some(())
                }
                B::Shr => {
                    if wt.get(*rhs).width > 64 {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(if wide {
                        NOp::WShr { w, arith: false }
                    } else {
                        NOp::Shr { w, arith: false }
                    });
                    Some(())
                }
                // `>>>`: fill governed by the LEFT operand's OWN sign (oracle
                // evaluates lhs at (w, own-sign); the final ctx re-stamp only
                // changes the sign FLAG, never the bits — registers carry bits).
                B::AShr => {
                    if wt.get(*rhs).width > 64 {
                        return None;
                    }
                    let lhs_signed = wt.get(*lhs).signed;
                    lower(ir, wt, *lhs, w, lhs_signed, ops)?;
                    lower(ir, wt, *rhs, 0, true, ops)?;
                    ops.push(if wide {
                        NOp::WShr {
                            w,
                            arith: lhs_signed,
                        }
                    } else {
                        NOp::Shr {
                            w,
                            arith: lhs_signed,
                        }
                    });
                    Some(())
                }
                // DIV/MOD: context-determined like Add (X/Z or /0 → all-X).
                // SIGNED wide div/mod X-poisons in the oracle — oracle-bound.
                B::Div | B::Mod => {
                    if wide && eff_signed {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, w, eff_signed, ops)?;
                    let kind = if matches!(op, B::Div) {
                        DivKind::Div
                    } else {
                        DivKind::Mod
                    };
                    ops.push(if wide {
                        NOp::WDivMod { kind, w }
                    } else {
                        NOp::DivMod {
                            kind,
                            w,
                            signed: eff_signed,
                        }
                    });
                    Some(())
                }
                // POWER: native only for a small const exponent n>=2, expanded to a
                // Mul chain (a*a*…). For n>=2 the X-poison matches the oracle's Pow
                // exactly (any X bit → all-X, identical to repeated Mul's arith
                // X-guard; n==1 is excluded above because it would skip the Mul).
                // Two guards keep VALUES byte-identical to the oracle:
                //  - bail when eff_signed (signed uses ipow_signed, a different path),
                //  - require w*n <= 128 so a^n < 2^128 — the oracle computes a^n in a
                //    u128 and returns 0 on u128 OVERFLOW (`checked_pow().unwrap_or(0)`)
                //    rather than wrapping, a quirk a per-step mod-2^w chain can't mimic.
                B::Pow => {
                    let n = const_pow_exponent(ir, *rhs)?;
                    if eff_signed || (w as u128) * (n as u128) > 128 {
                        return None;
                    }
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    for _ in 1..n {
                        lower(ir, wt, *lhs, w, eff_signed, ops)?;
                        ops.push(if wide {
                            NOp::WArith {
                                kind: ArithKind::Mul,
                                w,
                            }
                        } else {
                            NOp::Arith {
                                kind: ArithKind::Mul,
                                w,
                            }
                        });
                    }
                    Some(())
                }
                _ => {
                    let class = classify_binop(*op)?;
                    // SIGNED wide arith: the oracle X-poisons (its i128 sign lane
                    // gates at 64) — conservatively oracle-bound.
                    if wide && eff_signed && matches!(class, BinClass::Arith(_)) {
                        return None;
                    }
                    // ARITHMETIC + BITWISE: BOTH operands at (w, eff_signed).
                    lower(ir, wt, *lhs, w, eff_signed, ops)?;
                    lower(ir, wt, *rhs, w, eff_signed, ops)?;
                    ops.push(match (class, wide) {
                        (BinClass::Arith(kind), false) => NOp::Arith { kind, w },
                        (BinClass::Arith(kind), true) => NOp::WArith { kind, w },
                        (BinClass::Bit(kind), false) => NOp::Bitwise { kind, w },
                        (BinClass::Bit(kind), true) => NOp::WBitwise { kind, w },
                    });
                    Some(())
                }
            }
        }
        // ternary: cond self-determined truthiness; branches context-determined.
        Expr::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            let cond_w = wt.get(*cond).width;
            // a wide cond steering NARROW branches stays oracle-bound (the
            // narrow Ternary op reads a single-word cond register).
            if !wide && cond_w > 64 {
                return None;
            }
            lower(ir, wt, *cond, 0, true, ops)?;
            lower(ir, wt, *then_e, w, eff_signed, ops)?;
            lower(ir, wt, *else_e, w, eff_signed, ops)?;
            ops.push(if wide {
                NOp::WTernary {
                    w,
                    cond_wide: cond_w > 64,
                    cond_w,
                }
            } else {
                NOp::Ternary { w, cond_w }
            });
            Some(())
        }
        // ── structural ops: SELF-determined natural value, then UNSIGNED
        //    zero-extend to the node width (oracle: `eval_select`/`eval_concat`/
        //    `eval_replicate` + `resize_keep_sign(w, false)` — and unsigned
        //    `resize` is a plain zero-extend, which is FREE here because every
        //    register keeps its upper bits 0). ──
        Expr::Select {
            base,
            offset,
            width,
            kind,
        } => {
            // `width` is a const-expr edge — fold it exactly as the oracle does
            // (`unwrap_or(1)`); `Bit` forces a 1-bit select regardless.
            let folded = crate::width::const_u32_of_expr(ir, *width).unwrap_or(1);
            let sel_w = match kind {
                SelKind::Bit => 1,
                _ => folded,
            };
            let src_w = wt.get(*base).width;
            // v6 ④: the trio runs to 128 bits (two-word gather); only a wide
            // OFFSET register still bails.
            if sel_w == 0 || sel_w > 128 || src_w == 0 || src_w > 128 {
                return None;
            }
            if wt.get(*offset).width > 64 {
                return None;
            }
            lower(ir, wt, *base, 0, true, ops)?; // oracle: self.eval(base)
            lower(ir, wt, *offset, 0, true, ops)?; // oracle: self.eval(offset)
            let base_wide = src_w > 64;
            let out_wide = sel_w > 64;
            if !base_wide && !out_wide {
                ops.push(NOp::Select {
                    kind: *kind,
                    sel_w,
                    src_w,
                });
            } else {
                ops.push(NOp::WSelect {
                    kind: *kind,
                    sel_w,
                    src_w,
                    base_wide,
                    out_wide,
                });
            }
            // a narrow result feeding a wide context still bridges; a wide
            // result is already in place (zero-extend beyond sel_w is free).
            promote_if(wide && !out_wide, ops);
            Some(())
        }
        Expr::Concat { parts } => {
            // parts[0] is MSB-most; left-fold `(hi << lo_w) | lo` reproduces the
            // oracle's top-down fill. Natural width = Σ self widths ≤ node w ≤ 64.
            let (&first, rest) = parts.split_first()?;
            let mut tot = wt.get(first).width;
            if tot == 0 || tot > 128 {
                return None;
            }
            lower(ir, wt, first, 0, true, ops)?;
            for &p in rest {
                let pw = wt.get(p).width;
                if pw == 0 || pw > 128 {
                    return None;
                }
                let acc_wide = tot > 64; // where the RUNNING acc lives
                tot = tot.checked_add(pw).filter(|&t| t <= 128)?;
                lower(ir, wt, p, 0, true, ops)?;
                if tot <= 64 {
                    ops.push(NOp::ConcatPair { lo_w: pw, w: tot });
                } else {
                    ops.push(NOp::WConcatPair {
                        lo_w: pw,
                        w: tot,
                        acc_wide,
                        part_wide: pw > 64,
                    });
                }
            }
            // single-part `{x}` (or an all-narrow fold) may end narrow in a
            // wide context — bridge; a >64 fold already sits on the wide stack.
            promote_if(wide && tot <= 64, ops);
            Some(())
        }
        Expr::Replicate { count, value } => {
            // `count` is a const-expr edge (oracle folds with `unwrap_or(0)`);
            // a zero count is the degenerate width-0 case — leave it to the oracle.
            let count = crate::width::const_u32_of_expr(ir, *count).unwrap_or(0);
            if count == 0 {
                return None;
            }
            let part_w = wt.get(*value).width;
            // v6 ④: total runs to 128 (the PART stays narrow — a >64-bit part
            // with count ≥ 2 exceeds the wide lane anyway).
            if part_w == 0 || part_w > 64 {
                return None;
            }
            let total = part_w.checked_mul(count).filter(|&t| t <= 128)?;
            lower(ir, wt, *value, 0, true, ops)?;
            if total <= 64 {
                ops.push(NOp::Repl {
                    part_w,
                    count,
                    w: total,
                });
                promote_if(wide, ops);
            } else {
                ops.push(NOp::WRepl {
                    part_w,
                    count,
                    w: total,
                });
            }
            Some(())
        }
        // B1 frame-call: the VM never compiles a Call-bearing body
        // (`is_codegen_able`'s `expr_has_call` exclusion), so the native path
        // must NEVER reach a user `Expr::Call`. Assert the contract in debug;
        // bail to the oracle (kernel `eval_ctx`, which runs the real frame
        // evaluator) in release — safe either way.
        Expr::Call { .. } => {
            debug_assert!(
                false,
                "is_codegen_able must keep Expr::Call off the native/VM path"
            );
            None
        }
        // sysfunc / array-indexed signal: deferred increment.
        _ => None,
    }
}
