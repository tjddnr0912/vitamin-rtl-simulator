//! split part of `native_eval` (mechanical move).

use super::*;

/// Run a compiled program against `nets`, producing the single `Value` the oracle's
/// `eval_ctx` would return for the same `(ExprId, ctx)`.
/// Read one scalar net as a `(val, unk)` word pair — the single copy of this, shared by
/// the `LoadScalar` op and by `FastShape::LoadScalar`.
///
/// LEAF FAST PATH: a plain scalar net yields its word pair without ever building a
/// `Value`. Everything else falls through to the original read-then-resize, so the
/// special cases (real, frame-local, handle, array, wide) are untouched.
#[inline]
pub(crate) fn load_scalar(nets: &dyn NetReader, net: u32, w: u32, signed: bool) -> (u64, u64) {
    if let Some(pair) = nets.read_scalar_words(net, w, signed) {
        return pair;
    }
    let v = nets.read_net(net, None).resize_keep_sign(w, signed);
    let m = low_mask(w);
    (
        v.val.first().copied().unwrap_or(0) & m,
        v.unk.first().copied().unwrap_or(0) & m,
    )
}

pub(crate) fn run(prog: &NativeProg, nets: &dyn NetReader, scratch: &mut NativeScratch) -> Value {
    // TRIVIAL-SHAPE SHORTCUT: 46.3% of executions are ONE op (measured). For those the
    // whole apparatus below — two stacks over the scratch, the loop, a push and a pop —
    // moves a value that is already in hand. `NativeProg::fast` records which, decided
    // once at compile time from the finished op vector.
    match prog.fast {
        FastShape::Const { val, unk } => {
            let mut out = Value::zeros(prog.root_w, prog.root_signed);
            out.val[0] = val;
            out.unk[0] = unk;
            return out;
        }
        FastShape::LoadScalar { net, w, signed } => {
            let (pv, pu) = load_scalar(nets, net, w, signed);
            let mut out = Value::zeros(prog.root_w, prog.root_signed);
            out.val[0] = pv;
            out.unk[0] = pu;
            return out;
        }
        FastShape::Vm => {}
    }
    // P3-5: fixed arrays + manual sp — no heap allocation per evaluation. The arrays
    // live on the CALLER (see `NativeScratch`) rather than in this frame: `try_compile`
    // caps depth at `NATIVE_STACK` = 64, but the programs that actually run average 4.2
    // ops at depth 2–3, so declaring them here zeroed and touched a kilobyte of stack per
    // four-instruction expression, 6.5 million times.
    //
    // Stale contents are harmless: `sp` starts at 0 every run and `push` writes a slot
    // before `pop` can read it — the ARITY check `try_compile` performs is what
    // guarantees that, and the debug-only VM-ARITY-ASSERT below re-verifies it per op.
    let mut sp = 0usize;
    let mut stack = FixedStack {
        buf: &mut scratch.narrow,
        sp: &mut sp,
    };
    // VM-WIDEZERO: a narrow-only program (wmax == 0) never executes a W* opcode, so hand
    // the wide stack an empty slice — it is never indexed, and the cost of carrying it is
    // now zero either way since the buffer is no longer built per call.
    let wbuf_slice: &mut [(u128, u128)] = if prog.needs_wide {
        &mut scratch.wide
    } else {
        &mut []
    };
    let mut wsp = 0usize;
    let mut wstack = FixedStack {
        buf: wbuf_slice,
        sp: &mut wsp,
    };
    for op in &prog.ops {
        // VM-ARITY-ASSERT: verify each op's actual stack movement matches arity()
        // (which try_compile trusts for the fixed-stack cap check). Debug-only, so
        // release is byte-identical with zero overhead. Catches both a wrong
        // explicit arity entry and a NEW NOp variant silently routed to the
        // `_ => (2,1,0,0)` catchall.
        #[cfg(debug_assertions)]
        let (sp_dbg, wsp_dbg) = (*stack.sp, *wstack.sp);
        match *op {
            NOp::Const { val, unk } => stack.push((val, unk)),
            NOp::LoadScalar { net, w, signed } => stack.push(load_scalar(nets, net, w, signed)),
            NOp::Arith { kind, w } => {
                let (bv, bu) = stack.pop().expect("native arith: missing rhs");
                let (av, au) = stack.pop().expect("native arith: missing lhs");
                let m = low_mask(w);
                // Oracle `arith`: ANY X/Z in EITHER operand poisons the whole result to
                // X. An X bit is `(val=0, unk=1)` (matching `Value::xs`), so all-X is
                // `(0, m)` — NOT `(m, m)`.
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else {
                    let rv = match kind {
                        ArithKind::Add => av.wrapping_add(bv),
                        ArithKind::Sub => av.wrapping_sub(bv),
                        ArithKind::Mul => av.wrapping_mul(bv),
                    };
                    (rv & m, 0)
                };
                stack.push(res);
            }
            NOp::Bitwise { kind, w } => {
                let (bv, bu) = stack.pop().expect("native bitwise: missing rhs");
                let (av, au) = stack.pop().expect("native bitwise: missing lhs");
                let m = low_mask(w);
                let (rv, ru) = match kind {
                    BitKind::And => and_w(av, au, bv, bu),
                    BitKind::Or => or_w(av, au, bv, bu),
                    BitKind::Xor => xor_w(av, au, bv, bu),
                    BitKind::Xnor => xnor_w(av, au, bv, bu),
                };
                stack.push((rv & m, ru & m));
            }
            NOp::Not { w } => {
                let (av, au) = stack.pop().expect("native not: missing operand");
                let m = low_mask(w);
                let (rv, ru) = not_w(av, au);
                stack.push((rv & m, ru & m));
            }
            NOp::Neg { w } => {
                let (av, au) = stack.pop().expect("native neg: missing operand");
                let m = low_mask(w);
                let res = if (au & m) != 0 {
                    (0, m) // oracle `negate`: any X/Z poisons to X (val=0, unk=1)
                } else {
                    ((!av).wrapping_add(1) & m, 0)
                };
                stack.push(res);
            }
            NOp::Cmp { kind, w, signed } => {
                let (bv, bu) = stack.pop().expect("native cmp: missing rhs");
                let (av, au) = stack.pop().expect("native cmp: missing lhs");
                let m = low_mask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, 1) // oracle: any X/Z → 1-bit X
                } else {
                    use std::cmp::Ordering::*;
                    let ord = if signed {
                        let sx = |x: u64| ((x << (64 - w)) as i64) >> (64 - w);
                        sx(av & m).cmp(&sx(bv & m))
                    } else {
                        (av & m).cmp(&(bv & m))
                    };
                    let b = matches!(
                        (kind, ord),
                        (CmpKind::Lt, Less)
                            | (CmpKind::Le, Less)
                            | (CmpKind::Le, Equal)
                            | (CmpKind::Gt, Greater)
                            | (CmpKind::Ge, Greater)
                            | (CmpKind::Ge, Equal)
                    );
                    (b as u64, 0)
                };
                stack.push(res);
            }
            NOp::EqNe { ne, w } => {
                let (bv, bu) = stack.pop().expect("native eq: missing rhs");
                let (av, au) = stack.pop().expect("native eq: missing lhs");
                let m = low_mask(w);
                let u = (au | bu) & m;
                // §11.4.5: a both-known differing bit decides (definite 0/1);
                // X only for an AMBIGUOUS compare (mirrors eval.rs log_eq).
                let res = if ((av ^ bv) & !u & m) != 0 {
                    (ne as u64, 0)
                } else if u != 0 {
                    (0, 1)
                } else {
                    ((!ne) as u64, 0)
                };
                stack.push(res);
            }
            NOp::CaseEqNe { ne, w } => {
                let (bv, bu) = stack.pop().expect("native caseeq: missing rhs");
                let (av, au) = stack.pop().expect("native caseeq: missing lhs");
                let m = low_mask(w);
                let eq = (av & m) == (bv & m) && (au & m) == (bu & m);
                stack.push(((eq ^ ne) as u64, 0));
            }
            NOp::Shl { w } => {
                let (rv, ru) = stack.pop().expect("native shl: missing amount");
                let (lv, lu) = stack.pop().expect("native shl: missing lhs");
                let m = low_mask(w);
                let res = if ru != 0 {
                    (0, m) // X/Z amount → all-X at w (oracle xs(l.width))
                } else if rv >= 64 {
                    (0, 0) // everything shifted out
                } else {
                    ((lv << rv) & m, (lu << rv) & m)
                };
                stack.push(res);
            }
            NOp::Shr { w, arith } => {
                let (rv, ru) = stack.pop().expect("native shr: missing amount");
                let (lv, lu) = stack.pop().expect("native shr: missing lhs");
                let m = low_mask(w);
                let res = if ru != 0 {
                    (0, m)
                } else if !arith {
                    if rv >= 64 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    }
                } else {
                    // sign fill from l's MSB pair (which may itself be X/Z).
                    let fv = (lv >> (w - 1)) & 1;
                    let fu = (lu >> (w - 1)) & 1;
                    let body = if rv >= 64 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    };
                    let fill_n = rv.min(w as u64) as u32; // top bits to fill
                    let fill_mask = if fill_n == 0 {
                        0
                    } else {
                        m & !low_mask(w - fill_n)
                    };
                    (
                        (body.0 & !fill_mask) | (if fv == 1 { fill_mask } else { 0 }),
                        (body.1 & !fill_mask) | (if fu == 1 { fill_mask } else { 0 }),
                    )
                };
                stack.push(res);
            }
            NOp::DivMod { kind, w, signed } => {
                let (bv, bu) = stack.pop().expect("native divmod: missing rhs");
                let (av, au) = stack.pop().expect("native divmod: missing lhs");
                let m = low_mask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else if signed {
                    let sx = |x: u64| ((x << (64 - w)) as i64) >> (64 - w);
                    let b = sx(bv & m);
                    if b == 0 {
                        (0, m) // divide by zero → all-X
                    } else {
                        let a = sx(av & m);
                        let r = match kind {
                            DivKind::Div => a.wrapping_div(b),
                            DivKind::Mod => a.wrapping_rem(b),
                        };
                        ((r as u64) & m, 0)
                    }
                } else {
                    let b = bv & m;
                    if b == 0 {
                        (0, m)
                    } else {
                        let a = av & m;
                        let r = match kind {
                            DivKind::Div => a / b,
                            DivKind::Mod => a % b,
                        };
                        (r & m, 0)
                    }
                };
                stack.push(res);
            }
            NOp::Ternary { w, cond_w } => {
                let (ev, eu) = stack.pop().expect("native ternary: missing else");
                let (tv, tu) = stack.pop().expect("native ternary: missing then");
                let (cv, cu) = stack.pop().expect("native ternary: missing cond");
                stack.push(op_ternary(cv, cu, tv, tu, ev, eu, w, cond_w));
            }
            NOp::Reduce { kind, neg, opw } => {
                let (av, au) = stack.pop().expect("native reduce: missing operand");
                stack.push(op_reduce(av, au, kind, neg, opw));
            }
            NOp::LogNot { opw } => {
                let (av, au) = stack.pop().expect("native lognot: missing operand");
                let m = low_mask(opw);
                let res = if (av & !au & m) != 0 {
                    (0, 0) // truthy → !a = 0
                } else if (au & m) != 0 {
                    (0, 1) // unknown → X
                } else {
                    (1, 0) // falsy → 1
                };
                stack.push(res);
            }
            NOp::LogBin { and, lw, rw } => {
                let (bv, bu) = stack.pop().expect("native logbin: missing rhs");
                let (av, au) = stack.pop().expect("native logbin: missing lhs");
                let tri = |v: u64, u: u64, w: u32| {
                    let m = low_mask(w);
                    if (v & !u & m) != 0 {
                        Some(true)
                    } else if (u & m) != 0 {
                        None
                    } else {
                        Some(false)
                    }
                };
                let (l, r) = (tri(av, au, lw), tri(bv, bu, rw));
                let res = if and {
                    match (l, r) {
                        (Some(false), _) | (_, Some(false)) => (0, 0),
                        (Some(true), Some(true)) => (1, 0),
                        _ => (0, 1),
                    }
                } else {
                    match (l, r) {
                        (Some(true), _) | (_, Some(true)) => (1, 0),
                        (Some(false), Some(false)) => (0, 0),
                        _ => (0, 1),
                    }
                };
                stack.push(res);
            }
            NOp::Select { kind, sel_w, src_w } => {
                let (off_v, off_u) = stack.pop().expect("native select: missing offset");
                let (sv, su) = stack.pop().expect("native select: missing base");
                stack.push(op_select(sv, su, off_v, off_u, kind, sel_w, src_w));
            }
            NOp::ConcatPair { lo_w, w } => {
                let (lo_v, lo_u) = stack.pop().expect("native concat: missing lo");
                let (hi_v, hi_u) = stack.pop().expect("native concat: missing hi");
                let m = low_mask(w);
                // lo_w ≤ 63 here: with ≥2 parts of ≥1 bit each and w ≤ 64.
                stack.push((((hi_v << lo_w) | lo_v) & m, ((hi_u << lo_w) | lo_u) & m));
            }
            NOp::Repl { part_w, count, w } => {
                let (pv, pu) = stack.pop().expect("native repl: missing part");
                let (mut rv, mut ru) = (0u64, 0u64);
                for c in 0..count {
                    let sh = c * part_w; // (count-1)·part_w < w ≤ 64
                    rv |= pv << sh;
                    ru |= pu << sh;
                }
                let m = low_mask(w);
                stack.push((rv & m, ru & m));
            }
            NOp::LoadIndexed { net, w, signed } => {
                let (iv, iu) = stack.pop().expect("native loadidx: missing index");
                let v = nets
                    .read_net(net, Some(word_index(iv, iu)))
                    .resize_keep_sign(w, signed);
                let m = low_mask(w);
                stack.push((
                    v.val.first().copied().unwrap_or(0) & m,
                    v.unk.first().copied().unwrap_or(0) & m,
                ));
            }

            // ── C6 wide lane ──
            NOp::Promote => {
                let (v, u) = stack.pop().expect("native promote: missing operand");
                wstack.push((v as u128, u as u128));
            }
            NOp::WConst { val, unk } => wstack.push((val, unk)),
            NOp::WLoadScalar { net, w, signed } => {
                let v = nets.read_net(net, None).resize_keep_sign(w, signed);
                wstack.push(wide_pair(&v, w));
            }
            NOp::WLoadIndexed { net, w, signed } => {
                let (iv, iu) = stack.pop().expect("native wloadidx: missing index");
                let v = nets
                    .read_net(net, Some(word_index(iv, iu)))
                    .resize_keep_sign(w, signed);
                wstack.push(wide_pair(&v, w));
            }
            NOp::WArith { kind, w } => {
                let (bv, bu) = wstack.pop().expect("native warith: missing rhs");
                let (av, au) = wstack.pop().expect("native warith: missing lhs");
                let m = wmask(w);
                // oracle `arith` unsigned lane: u128 wrapping ops, X/Z poisons.
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else {
                    let rv = match kind {
                        ArithKind::Add => av.wrapping_add(bv),
                        ArithKind::Sub => av.wrapping_sub(bv),
                        ArithKind::Mul => av.wrapping_mul(bv),
                    };
                    (rv & m, 0)
                };
                wstack.push(res);
            }
            NOp::WBitwise { kind, w } => {
                let (bv, bu) = wstack.pop().expect("native wbitwise: missing rhs");
                let (av, au) = wstack.pop().expect("native wbitwise: missing lhs");
                let m = wmask(w);
                // the same `value::*_w` plane formulas, bit-parallel at u128.
                let (rv, ru) = match kind {
                    BitKind::And => {
                        let known0 = (!au & !av) | (!bu & !bv);
                        let known1 = (!au & av) & (!bu & bv);
                        (known1, !known0 & !known1)
                    }
                    BitKind::Or => {
                        let known1 = (!au & av) | (!bu & bv);
                        let known0 = (!au & !av) & (!bu & !bv);
                        (known1, !known1 & !known0)
                    }
                    BitKind::Xor => {
                        let ru = au | bu;
                        ((av ^ bv) & !ru, ru)
                    }
                    BitKind::Xnor => {
                        let ru = au | bu;
                        (!(av ^ bv) & !ru, ru)
                    }
                };
                wstack.push((rv & m, ru & m));
            }
            NOp::WNot { w } => {
                let (av, au) = wstack.pop().expect("native wnot: missing operand");
                let m = wmask(w);
                wstack.push(((!av & !au) & m, au & m));
            }
            NOp::WNeg { w } => {
                let (av, au) = wstack.pop().expect("native wneg: missing operand");
                let m = wmask(w);
                let res = if (au & m) != 0 {
                    (0, m) // oracle `negate`: any X/Z poisons to X
                } else {
                    // (!x + 1) at u128 ≡ the oracle's per-word carry chain.
                    ((!av).wrapping_add(1) & m, 0)
                };
                wstack.push(res);
            }
            NOp::WCmp { kind, w, signed } => {
                let (bv, bu) = wstack.pop().expect("native wcmp: missing rhs");
                let (av, au) = wstack.pop().expect("native wcmp: missing lhs");
                let m = wmask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, 1) // oracle: any X/Z → 1-bit X
                } else {
                    use std::cmp::Ordering::*;
                    let ord = if signed {
                        // sign-extend from w (65..=128; w==128 shifts by 0).
                        let sx = |x: u128| ((x << (128 - w)) as i128) >> (128 - w);
                        sx(av & m).cmp(&sx(bv & m))
                    } else {
                        (av & m).cmp(&(bv & m))
                    };
                    let b = matches!(
                        (kind, ord),
                        (CmpKind::Lt, Less)
                            | (CmpKind::Le, Less)
                            | (CmpKind::Le, Equal)
                            | (CmpKind::Gt, Greater)
                            | (CmpKind::Ge, Greater)
                            | (CmpKind::Ge, Equal)
                    );
                    (b as u64, 0)
                };
                stack.push(res);
            }
            NOp::WEqNe { ne, w } => {
                let (bv, bu) = wstack.pop().expect("native weq: missing rhs");
                let (av, au) = wstack.pop().expect("native weq: missing lhs");
                let m = wmask(w);
                let u = (au | bu) & m;
                // §11.4.5 definite-mismatch rule — mirrors EqNe / log_eq.
                let res = if ((av ^ bv) & !u & m) != 0 {
                    (ne as u64, 0)
                } else if u != 0 {
                    (0, 1)
                } else {
                    ((!ne) as u64, 0)
                };
                stack.push(res);
            }
            NOp::WCaseEqNe { ne, w } => {
                let (bv, bu) = wstack.pop().expect("native wcaseeq: missing rhs");
                let (av, au) = wstack.pop().expect("native wcaseeq: missing lhs");
                let m = wmask(w);
                let eq = (av & m) == (bv & m) && (au & m) == (bu & m);
                stack.push(((eq ^ ne) as u64, 0));
            }
            NOp::WShl { w } => {
                let (rv, ru) = stack.pop().expect("native wshl: missing amount");
                let (lv, lu) = wstack.pop().expect("native wshl: missing lhs");
                let m = wmask(w);
                let res = if ru != 0 {
                    (0, m)
                } else if rv >= 128 {
                    (0, 0)
                } else {
                    ((lv << rv) & m, (lu << rv) & m)
                };
                wstack.push(res);
            }
            NOp::WShr { w, arith } => {
                let (rv, ru) = stack.pop().expect("native wshr: missing amount");
                let (lv, lu) = wstack.pop().expect("native wshr: missing lhs");
                let m = wmask(w);
                let res = if ru != 0 {
                    (0, m)
                } else if !arith {
                    if rv >= 128 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    }
                } else {
                    let fv = (lv >> (w - 1)) & 1;
                    let fu = (lu >> (w - 1)) & 1;
                    let body = if rv >= 128 {
                        (0, 0)
                    } else {
                        ((lv >> rv) & m, (lu >> rv) & m)
                    };
                    let fill_n = rv.min(w as u64) as u32;
                    let fill_mask = if fill_n == 0 {
                        0
                    } else {
                        m & !wmask(w - fill_n)
                    };
                    (
                        (body.0 & !fill_mask) | (if fv == 1 { fill_mask } else { 0 }),
                        (body.1 & !fill_mask) | (if fu == 1 { fill_mask } else { 0 }),
                    )
                };
                wstack.push(res);
            }
            NOp::WDivMod { kind, w } => {
                let (bv, bu) = wstack.pop().expect("native wdivmod: missing rhs");
                let (av, au) = wstack.pop().expect("native wdivmod: missing lhs");
                let m = wmask(w);
                let res = if (au & m) != 0 || (bu & m) != 0 {
                    (0, m)
                } else {
                    let b = bv & m;
                    if b == 0 {
                        (0, m)
                    } else {
                        let a = av & m;
                        let r = match kind {
                            DivKind::Div => a / b,
                            DivKind::Mod => a % b,
                        };
                        (r & m, 0)
                    }
                };
                wstack.push(res);
            }
            NOp::WTernary {
                w,
                cond_wide,
                cond_w,
            } => {
                let (ev, eu) = wstack.pop().expect("native wternary: missing else");
                let (tv, tu) = wstack.pop().expect("native wternary: missing then");
                // truthiness fold is OR-equivalent across words, so the wide
                // cond uses the same rule on u128.
                let (c1, cx) = if cond_wide {
                    let (cv, cu) = wstack.pop().expect("native wternary: missing cond");
                    let mc = wmask(cond_w);
                    ((cv & !cu & mc) != 0, (cu & mc) != 0)
                } else {
                    let (cv, cu) = stack.pop().expect("native wternary: missing cond");
                    let mc = low_mask(cond_w);
                    ((cv & !cu & mc) != 0, (cu & mc) != 0)
                };
                let m = wmask(w);
                let res = if c1 {
                    (tv & m, tu & m)
                } else if cx {
                    let ident = !((tv ^ ev) | (tu ^ eu)) & m;
                    ((tv & ident), (tu & ident) | (m & !ident))
                } else {
                    (ev & m, eu & m)
                };
                wstack.push(res);
            }
            NOp::WReduce { kind, neg, opw } => {
                let (av, au) = wstack.pop().expect("native wreduce: missing operand");
                let m = wmask(opw);
                let known1 = av & !au & m;
                let known0 = !au & !av & m;
                let unk = au & m;
                let (v, u): (u64, u64) = match kind {
                    RedK::And if known0 != 0 => (0, 0),
                    RedK::And if unk != 0 => (0, 1),
                    RedK::And => (1, 0),
                    RedK::Or if known1 != 0 => (1, 0),
                    RedK::Or if unk != 0 => (0, 1),
                    RedK::Or => (0, 0),
                    RedK::Xor if unk != 0 => (0, 1),
                    RedK::Xor => ((known1.count_ones() & 1) as u64, 0),
                };
                let out = if neg && u == 0 { (v ^ 1, 0) } else { (v, u) };
                stack.push(out);
            }
            // ── v6 ④ wide structural trio ──
            NOp::WSelect {
                kind,
                sel_w,
                src_w,
                base_wide,
                out_wide,
            } => {
                let (off_v, off_u) = stack.pop().expect("native wselect: missing offset");
                let (sv, su): (u128, u128) = if base_wide {
                    wstack.pop().expect("native wselect: missing base")
                } else {
                    let (v, u) = stack.pop().expect("native wselect: missing base");
                    (v as u128, u as u128)
                };
                let m = wmask(sel_w);
                let res: (u128, u128) = match (off_u == 0)
                    .then_some(off_v)
                    .and_then(|v| i64::try_from(v).ok())
                {
                    None => (0, m), // X/Z offset → sel_w X bits (oracle)
                    Some(off) => {
                        let lsb = match kind {
                            SelKind::Bit | SelKind::PartConst | SelKind::PartIdxUp => off,
                            SelKind::PartIdxDown => off - (sel_w as i64) + 1,
                        };
                        // fully in-range: one two-word shift (oracle fast path)
                        if lsb >= 0 && (lsb as u64) + sel_w as u64 <= src_w as u64 {
                            (((sv >> lsb) & m), ((su >> lsb) & m))
                        } else {
                            let (mut rv, mut ru) = (0u128, 0u128);
                            for i in 0..sel_w as i64 {
                                let si = lsb + i;
                                if si >= 0 && (si as u32) < src_w {
                                    rv |= ((sv >> si) & 1) << i;
                                    ru |= ((su >> si) & 1) << i;
                                } else {
                                    ru |= 1 << i; // out-of-range read → X
                                }
                            }
                            (rv, ru)
                        }
                    }
                };
                if out_wide {
                    wstack.push(res);
                } else {
                    stack.push((res.0 as u64, res.1 as u64));
                }
            }
            NOp::WConcatPair {
                lo_w,
                w,
                acc_wide,
                part_wide,
            } => {
                let (lo_v, lo_u): (u128, u128) = if part_wide {
                    wstack.pop().expect("native wconcat: missing lo")
                } else {
                    let (v, u) = stack.pop().expect("native wconcat: missing lo");
                    (v as u128, u as u128)
                };
                let (hi_v, hi_u): (u128, u128) = if acc_wide {
                    wstack.pop().expect("native wconcat: missing hi")
                } else {
                    let (v, u) = stack.pop().expect("native wconcat: missing hi");
                    (v as u128, u as u128)
                };
                // lo_w ≤ 127 here: tot = acc_w + lo_w ≤ 128 with acc_w ≥ 1.
                let m = wmask(w);
                wstack.push((((hi_v << lo_w) | lo_v) & m, ((hi_u << lo_w) | lo_u) & m));
            }
            NOp::WRepl { part_w, count, w } => {
                let (pv, pu) = stack.pop().expect("native wrepl: missing part");
                let (pv, pu) = (pv as u128, pu as u128);
                let (mut rv, mut ru) = (0u128, 0u128);
                for c in 0..count {
                    let sh = c * part_w; // (count-1)·part_w < w ≤ 128
                    rv |= pv << sh;
                    ru |= pu << sh;
                }
                let m = wmask(w);
                wstack.push((rv & m, ru & m));
            }
            NOp::WLogNot { opw } => {
                let (av, au) = wstack.pop().expect("native wlognot: missing operand");
                let m = wmask(opw);
                let res = if (av & !au & m) != 0 {
                    (0, 0)
                } else if (au & m) != 0 {
                    (0, 1)
                } else {
                    (1, 0)
                };
                stack.push(res);
            }
        }
        #[cfg(debug_assertions)]
        {
            let (np, npu, wp, wpu) = arity(op);
            debug_assert_eq!(
                *stack.sp as i64,
                sp_dbg as i64 - np as i64 + npu as i64,
                "VM-ARITY-ASSERT: narrow-stack drift (arity npop={np} npush={npu})"
            );
            debug_assert_eq!(
                *wstack.sp as i64,
                wsp_dbg as i64 - wp as i64 + wpu as i64,
                "VM-ARITY-ASSERT: wide-stack drift (arity wpop={wp} wpush={wpu})"
            );
        }
    }
    let mut out = Value::zeros(prog.root_w, prog.root_signed);
    if prog.root_w > 64 {
        let (fv, fu) = wstack.pop().expect("native eval produced no wide result");
        out.val[0] = fv as u64;
        out.val[1] = (fv >> 64) as u64;
        out.unk[0] = fu as u64;
        out.unk[1] = (fu >> 64) as u64;
    } else {
        let (fv, fu) = stack.pop().expect("native eval produced no result");
        out.val[0] = fv;
        out.unk[0] = fu;
    }
    out
}

/// The oracle's array-word index conversion: `to_u64` (None on X/Z) then
/// `u32::try_from`, both failures mapping to the `u32::MAX` OOR sentinel.
#[inline]
pub(crate) fn word_index(iv: u64, iu: u64) -> u32 {
    if iu != 0 {
        u32::MAX
    } else {
        u32::try_from(iv).unwrap_or(u32::MAX)
    }
}

// ── OP BODIES SHARED WITH THE JIT ────────────────────────────────────────────
//
// These are the arms whose semantics are a LOOP or a table rather than a handful of
// branchless bit operations. The body JIT calls them through `extern "C"` shims instead
// of re-expressing them in cranelift IR, so there is no third implementation of them to
// drift: the VM arm above and the compiled body run the same function.

/// `NOp::Select` — a bit/part select, including the out-of-range and X-offset rules.
pub(crate) fn op_select(
    sv: u64,
    su: u64,
    off_v: u64,
    off_u: u64,
    kind: SelKind,
    sel_w: u32,
    src_w: u32,
) -> (u64, u64) {
    let m = low_mask(sel_w);
    // Oracle: X/Z offset (or one beyond the i64 lane) ⇒ the whole select reads X at its
    // natural width (upper bits stay 0 — the unsigned resize is a zero-extend).
    match (off_u == 0)
        .then_some(off_v)
        .and_then(|v| i64::try_from(v).ok())
    {
        None => (0, m),
        Some(off) => {
            let lsb = match kind {
                SelKind::Bit | SelKind::PartConst | SelKind::PartIdxUp => off,
                SelKind::PartIdxDown => off - (sel_w as i64) + 1,
            };
            let (mut rv, mut ru) = (0u64, 0u64);
            for i in 0..sel_w as i64 {
                let si = lsb + i;
                if si >= 0 && (si as u32) < src_w {
                    rv |= ((sv >> si) & 1) << i;
                    ru |= ((su >> si) & 1) << i;
                } else {
                    ru |= 1 << i; // out-of-range read → X (val=0)
                }
            }
            (rv, ru)
        }
    }
}

/// `NOp::Reduce` — `&`/`|`/`^` over the operand's bits, with the N-forms' inversion.
pub(crate) fn op_reduce(av: u64, au: u64, kind: RedK, neg: bool, opw: u32) -> (u64, u64) {
    let m = low_mask(opw);
    let known1 = av & !au & m;
    let known0 = !au & !av & m;
    let unk = au & m;
    let (v, u): (u64, u64) = match kind {
        RedK::And if known0 != 0 => (0, 0),
        RedK::And if unk != 0 => (0, 1),
        RedK::And => (1, 0),
        RedK::Or if known1 != 0 => (1, 0),
        RedK::Or if unk != 0 => (0, 1),
        RedK::Or => (0, 0),
        RedK::Xor if unk != 0 => (0, 1),
        RedK::Xor => ((known1.count_ones() & 1) as u64, 0),
    };
    if neg && u == 0 {
        (v ^ 1, 0)
    } else {
        (v, u)
    }
}

/// `NOp::Ternary` — tri-valued condition, with `merge_x` when it is unknown.
#[allow(clippy::too_many_arguments)]
pub(crate) fn op_ternary(
    cv: u64,
    cu: u64,
    tv: u64,
    tu: u64,
    ev: u64,
    eu: u64,
    w: u32,
    cond_w: u32,
) -> (u64, u64) {
    let mc = low_mask(cond_w);
    let m = low_mask(w);
    // truthiness: any definite-1 → True; else any unknown → Unknown; else False
    // (matches oracle `Tri`).
    if (cv & !cu & mc) != 0 {
        (tv & m, tu & m)
    } else if (cu & mc) != 0 {
        // merge_x: identical (val,unk) pairs pass through, else X.
        let ident = !((tv ^ ev) | (tu ^ eu)) & m;
        (tv & ident, (tu & ident) | (m & !ident))
    } else {
        (ev & m, eu & m)
    }
}
