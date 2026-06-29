//! Engine-side IEEE 1364-2005 context-determined width inference.
//!
//! Builds a side table `Vec<SelfWidth>` indexed by ExprId, parallel to
//! `SimIr.exprs`, computed once at `SimState::new`. The frozen sim-ir is read
//! verbatim; this table lives ENTIRELY in engine state. It encodes each expr's
//! self-determined (bottom-up) width and signedness per §5.4.1 / §5.5.
//!
//! See `docs/superpowers/plans/2026-06-04-width-inference-spec.md`.

use sim_ir::{BinOp, ConstRepr, Expr, SelKind, SimIr, SysFuncId, UnOp};

/// Self-determined sizing of one expression node (IEEE §5.4.1 / §5.5).
/// `width` is the bottom-up self-width; `signed` is the self-signedness
/// (the both-signed rule already folded in for context-determined operators).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelfWidth {
    pub width: u32,
    pub signed: bool,
}

/// The whole side table, one entry per `SimIr.exprs[i]`.
pub(crate) struct WidthTable {
    sw: Vec<SelfWidth>,
}

impl WidthTable {
    #[inline]
    pub(crate) fn get(&self, eid: u32) -> SelfWidth {
        self.sw[eid as usize]
    }
    #[inline]
    pub(crate) fn width(&self, eid: u32) -> u32 {
        self.sw[eid as usize].width
    }
    #[inline]
    pub(crate) fn signed(&self, eid: u32) -> bool {
        self.sw[eid as usize].signed
    }
    /// N7: override the self-width of class field-read `Signal`s. Such a Signal's
    /// net is the 32-bit handle, so the generic build gives it width 32; the real
    /// width is the FIELD's, computed at elaborate and carried here per-ExprId.
    /// EMPTY ⇒ no-op (byte-identical for every prior design).
    pub(crate) fn patch_class_fields(
        &mut self,
        map: &std::collections::BTreeMap<u32, (u32, bool)>,
    ) {
        for (&eid, &(w, signed)) in map {
            if let Some(slot) = self.sw.get_mut(eid as usize) {
                slot.width = clamp_w(w.max(1));
                slot.signed = signed;
            }
        }
    }
}

const WIDTH_MAX: u32 = 1 << 24;

#[inline]
// `w` is u32 so it is already >= 0; do NOT add `.max(0)` — that is a no-op that
// trips `clippy::unnecessary_min_or_max` under `-D warnings`. A floor of 1 is
// applied separately by callers that need it (`.max(1)`), not here.
fn clamp_w(w: u32) -> u32 {
    w.min(WIDTH_MAX)
}
#[inline]
fn add_w(a: u32, b: u32) -> u32 {
    clamp_w(a.saturating_add(b))
}
#[inline]
fn mul_w(a: u32, b: u32) -> u32 {
    clamp_w(a.saturating_mul(b))
}

impl WidthTable {
    /// Build the self-width table by a single forward pass over `ir.exprs`.
    /// PRECONDITION (verified §1): every child ExprId < its parent ExprId, so a
    /// forward scan reads only already-filled entries.
    pub(crate) fn build(ir: &SimIr, ft: &crate::FuncTable) -> WidthTable {
        let n = ir.exprs.len();
        let mut sw: Vec<SelfWidth> = Vec::with_capacity(n);
        for i in 0..n {
            let s = Self::self_width_of(ir, ft, &sw, i as u32);
            debug_assert_eq!(sw.len(), i, "forward pass invariant");
            sw.push(s);
        }
        WidthTable { sw }
    }
}

/// Read an already-computed child's self-width. `child` MUST be < `parent`
/// (post-order arena, §1). The guard converts any future ordering regression
/// into a deterministic panic instead of reading a default/garbage slot.
#[inline]
fn child(sw: &[SelfWidth], parent: u32, child: u32) -> SelfWidth {
    assert!(
        (child as usize) < sw.len() && child < parent,
        "width pass: child {child} not yet computed for parent {parent} \
         (arena not post-order — see width.rs §1)"
    );
    sw[child as usize]
}

impl WidthTable {
    fn self_width_of(ir: &SimIr, ft: &crate::FuncTable, sw: &[SelfWidth], i: u32) -> SelfWidth {
        match &ir.exprs[i as usize] {
            // ── leaves ──────────────────────────────────────────────────────
            Expr::Const { val } => {
                let c = &ir.consts[*val as usize];
                // A real const is {width:64, signed:true} (its real-ness is
                // established at eval time via ConstRepr::Real).
                if matches!(c.repr, ConstRepr::Real) {
                    return SelfWidth {
                        width: 64,
                        signed: true,
                    };
                }
                // A const is signed ONLY when repr==Numeric AND its signed flag set
                // (string consts never signed) — mirrors eval_const (eval.rs:67).
                let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
                SelfWidth {
                    width: clamp_w(c.width.max(1)),
                    signed,
                }
            }
            Expr::Signal { net, .. } => {
                let nv = &ir.nets[*net as usize];
                // `word` selects an ARRAY element; element width == NetVar.width.
                SelfWidth {
                    width: clamp_w(nv.width.max(1)),
                    signed: nv.signed,
                }
            }
            // ⓑ-breadth (v17): the with-clause iterator carries its element type.
            Expr::ArrayItem { width, signed, .. } => SelfWidth {
                width: clamp_w((*width).max(1)),
                signed: *signed,
            },

            // ── select: bit=1 (UNSIGNED), part=`width` operand (UNSIGNED) ─────
            // IEEE §5.4.1: bit-select and part-select results are ALWAYS unsigned.
            Expr::Select { width, kind, .. } => {
                let w = match kind {
                    SelKind::Bit => 1,
                    // `width` is an EXPR INDEX (frozen IR). v1 elaborate emits a
                    // constant width tree for PartConst/PartIdxUp/PartIdxDown; we
                    // resolve it via the const-fold helper (§3.4). If it does not
                    // const-fold, fall back to width 1 (defensive); eval_select
                    // still clamps at runtime.
                    _ => const_u32_of_expr(ir, *width).unwrap_or(1),
                };
                SelfWidth {
                    width: clamp_w(w.max(1)),
                    signed: false,
                }
            }

            // ── concat: SUM of part self-widths, ALWAYS UNSIGNED, self-determined ─
            Expr::Concat { parts } => {
                let mut total = 0u32;
                for &p in parts {
                    total = add_w(total, child(sw, i, p).width);
                }
                SelfWidth {
                    width: clamp_w(total.max(1)),
                    signed: false,
                }
            }

            // ── replicate: count * width(value), ALWAYS UNSIGNED, self-determined ─
            Expr::Replicate { count, value } => {
                let n = const_u32_of_expr(ir, *count).unwrap_or(0);
                let vw = child(sw, i, *value).width;
                // A zero replication `{0{x}}` has width 0 (IEEE §11.4.12.1 — legal
                // only inside a concatenation, where it contributes nothing). The old
                // `.max(1)` injected a spurious bit, so e.g. `{4'hA,{0{1'b1}},4'h5}`
                // sized to 9 bits and printed `45` instead of `a5`. `eval_replicate`
                // already yields width 0, so this aligns the table with the engine.
                SelfWidth {
                    width: clamp_w(mul_w(n, vw)),
                    signed: false,
                }
            }

            // ── unary ─────────────────────────────────────────────────────────
            Expr::Unary { op, operand } => {
                let o = child(sw, i, *operand);
                match op {
                    // context-determined unary: width = operand width, sign = operand
                    UnOp::Plus | UnOp::Minus | UnOp::BitNot => SelfWidth {
                        width: o.width.max(1),
                        signed: o.signed,
                    },
                    // reductions + logical-not: 1-bit, UNSIGNED, operand self-det
                    UnOp::LogNot
                    | UnOp::RedAnd
                    | UnOp::RedNand
                    | UnOp::RedOr
                    | UnOp::RedNor
                    | UnOp::RedXor
                    | UnOp::RedXnor => SelfWidth {
                        width: 1,
                        signed: false,
                    },
                }
            }

            // ── binary ────────────────────────────────────────────────────────
            Expr::Binary { op, lhs, rhs } => {
                let l = child(sw, i, *lhs);
                let r = child(sw, i, *rhs);
                match op {
                    // arithmetic + bitwise: max(L,R), signed iff BOTH signed
                    BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::Mod
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor
                    | BinOp::BitXnor => SelfWidth {
                        width: clamp_w(l.width.max(r.width).max(1)),
                        signed: l.signed && r.signed,
                    },
                    // power: width = LEFT (base) operand width (IEEE Table 11-21,
                    // like shifts), NOT max(L,R). The EXPONENT is SELF-determined —
                    // it affects neither the result width nor the sign. `**` is
                    // signed iff the BASE is signed (an unsigned exponent must not
                    // demote a signed base to unsigned). `b4 ** e8` with `b4`
                    // declared [3:0] is a 4-bit result (iverilog parity).
                    BinOp::Pow => SelfWidth {
                        width: l.width.max(1),
                        signed: l.signed, // BASE sign only
                    },
                    // comparisons / case-eq (incl. v7 casez/casex match) /
                    // logical: 1-bit, UNSIGNED
                    BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::CaseEq
                    | BinOp::CaseNe
                    | BinOp::CasezEq
                    | BinOp::CasexEq
                    | BinOp::LogAnd
                    | BinOp::LogOr => SelfWidth {
                        width: 1,
                        signed: false,
                    },
                    // shifts: width = LEFT operand width, sign follows LEFT.
                    // RHS (amount) is SELF-DETERMINED — does not affect this width.
                    BinOp::Shl | BinOp::Shr | BinOp::AShl | BinOp::AShr => SelfWidth {
                        width: l.width.max(1),
                        signed: l.signed,
                    },
                }
            }

            // ── ternary: max(then,else), signed iff BOTH branches signed ───────
            Expr::Ternary { then_e, else_e, .. } => {
                let t = child(sw, i, *then_e);
                let e = child(sw, i, *else_e);
                SelfWidth {
                    width: clamp_w(t.width.max(e.width).max(1)),
                    signed: t.signed && e.signed,
                }
            }

            // ── system functions (§6) ──────────────────────────────────────────
            Expr::SysFunc { which, args } => match which {
                // $time / $realtime: 64-bit unsigned (realtime modeled as 64-bit).
                SysFuncId::Time | SysFuncId::Realtime => SelfWidth {
                    width: 64,
                    signed: false,
                },
                // v5 dyn-storage methods: size()/num() are int (32 signed),
                // exists() is 1 bit.
                SysFuncId::DynSize | SysFuncId::AssocNum => SelfWidth {
                    width: 32,
                    signed: true,
                },
                SysFuncId::AssocExists => SelfWidth {
                    width: 1,
                    signed: false,
                },
                // v6: assoc iteration methods return the int STATUS
                // (1 found / 0 none / −1 ref-arg truncation, §7.9.4).
                SysFuncId::AssocFirst
                | SysFuncId::AssocNext
                | SysFuncId::AssocLast
                | SysFuncId::AssocPrev => SelfWidth {
                    width: 32,
                    signed: true,
                },
                // ④: pops return the ELEMENT type of their handle (args[0] is
                // its whole-net Signal) — the signedness drives the §5.5
                // assignment extension (a signed byte −1 pops as −1 into an
                // int, an unsigned 255 stays 255; iverilog live).
                // ⓑ-breadth (v15): array reductions (sum/product/and/or/xor) are
                // also element-typed — same handle-net recipe as the pops.
                SysFuncId::QPopBack
                | SysFuncId::QPopFront
                | SysFuncId::ArrSum
                | SysFuncId::ArrProduct
                | SysFuncId::ArrAnd
                | SysFuncId::ArrOr
                | SysFuncId::ArrXor => args
                    .first()
                    .and_then(|&a| match ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, .. }) => ir.nets.get(*net as usize),
                        _ => None,
                    })
                    .map(|nv| SelfWidth {
                        width: nv.width.max(1),
                        signed: nv.signed,
                    })
                    .unwrap_or(SelfWidth {
                        width: 32,
                        signed: true,
                    }),
                // $signed / $unsigned: PRESERVE operand width, flip sign attribute.
                SysFuncId::Signed => {
                    let w = args.first().map(|&a| child(sw, i, a).width).unwrap_or(1);
                    SelfWidth {
                        width: w.max(1),
                        signed: true,
                    }
                }
                SysFuncId::Unsigned => {
                    let w = args.first().map(|&a| child(sw, i, a).width).unwrap_or(1);
                    SelfWidth {
                        width: w.max(1),
                        signed: false,
                    }
                }
                // $clog2: integer return → 32-bit signed `integer` convention.
                SysFuncId::Clog2 => SelfWidth {
                    width: 32,
                    signed: true,
                },
                // $rtoi: integer return → 32-bit signed.
                SysFuncId::Rtoi => SelfWidth {
                    width: 32,
                    signed: true,
                },
                // $itor / $bitstoreal: real return → 64-bit signed (real-domain;
                // the is_real flag is established at eval time). v19: the N6
                // real-math functions (§20.8.2) are likewise real-returning.
                SysFuncId::Itor
                | SysFuncId::BitsToReal
                | SysFuncId::Ln
                | SysFuncId::Log10
                | SysFuncId::Exp
                | SysFuncId::Sqrt
                | SysFuncId::Pow
                | SysFuncId::Floor
                | SysFuncId::Ceil
                | SysFuncId::Sin
                | SysFuncId::Cos
                | SysFuncId::Tan
                | SysFuncId::Asin
                | SysFuncId::Acos
                | SysFuncId::Atan
                | SysFuncId::Atan2
                | SysFuncId::Hypot
                | SysFuncId::Sinh
                | SysFuncId::Cosh
                | SysFuncId::Tanh
                | SysFuncId::Asinh
                | SysFuncId::Acosh
                | SysFuncId::Atanh => SelfWidth {
                    width: 64,
                    signed: true,
                },
                // $realtobits: raw 64-bit vector → 64-bit unsigned.
                SysFuncId::RealToBits => SelfWidth {
                    width: 64,
                    signed: false,
                },
                // v7: int-returning funcs (32 signed — IEEE function returns).
                SysFuncId::Random
                | SysFuncId::CountOnes
                | SysFuncId::Fopen
                | SysFuncId::TestPlusargs
                | SysFuncId::ValuePlusargs
                | SysFuncId::StrLen
                | SysFuncId::StrCmp
                // v9: file-read family + $dist_* + $cast — all `int` returns.
                | SysFuncId::Fgets
                | SysFuncId::Fscanf
                | SysFuncId::Sscanf
                | SysFuncId::Fread
                | SysFuncId::Feof
                | SysFuncId::Fgetc
                | SysFuncId::Ungetc
                | SysFuncId::DistUniform
                | SysFuncId::DistNormal
                | SysFuncId::DistExponential
                | SysFuncId::DistPoisson
                | SysFuncId::DistChiSquare
                | SysFuncId::DistT
                | SysFuncId::DistErlang
                | SysFuncId::Cast
                // v18: string→int conversions — all `int` returns.
                | SysFuncId::StrAtoi
                | SysFuncId::StrAtohex
                | SysFuncId::StrAtooct
                | SysFuncId::StrAtobin => SelfWidth {
                    width: 32,
                    signed: true,
                },
                // v18: `.atoreal()` → real (64-bit, real-ness set at eval time).
                SysFuncId::StrAtoreal => SelfWidth {
                    width: 64,
                    signed: true,
                },
                // v7: unsigned-32 funcs ($urandom family per §18.13, $stime's
                // truncated 32-bit time per 1364 §17.7.2).
                SysFuncId::Urandom | SysFuncId::UrandomRange | SysFuncId::Stime => SelfWidth {
                    width: 32,
                    signed: false,
                },
                // v7: 1-bit predicates.
                SysFuncId::OneHot | SysFuncId::OneHot0 | SysFuncId::IsUnknown => SelfWidth {
                    width: 1,
                    signed: false,
                },
                // v7: string `.getc(i)` is one byte.
                SysFuncId::StrGetC => SelfWidth {
                    width: 8,
                    signed: false,
                },
                // v7: string-VALUE producers — true width is DYNAMIC (8×len at
                // eval). Static placeholder pending the string slice, which
                // gives string-domain values a resize bypass (like is_real).
                SysFuncId::Sformatf
                | SysFuncId::StrSubstr
                | SysFuncId::StrToUpper
                | SysFuncId::StrToLower => SelfWidth {
                    width: 8,
                    signed: false,
                },
            },

            // ── user function call (B1): self-width = the DECLARED return width
            // from the frame-call sidecar (`Expr::Call` has no net id of its own).
            // Empty/absent ⇒ 1-bit (byte-identical to the pre-B1 stub, and the
            // matching eval arm X-poisons a Call with no sidecar entry). The arm
            // reads `ft` (an independent table, NOT a child expr) so the
            // child<parent forward-pass invariant is untouched; `ret_width` is the
            // declared width, never body-derived → no circular dependency.
            Expr::Call { func, .. } => ft
                .get(*func as usize)
                .map(|m| SelfWidth {
                    width: clamp_w(m.ret_width.max(1)),
                    signed: m.ret_signed,
                })
                .unwrap_or(SelfWidth {
                    width: 1,
                    signed: false,
                }),
        }
    }
}

/// Resolve an expr to a compile-time u32 if it is a literal const (the v1
/// elaborate guarantee for part-select width and replicate count). Returns
/// None for non-const trees (e.g. a runtime offset, which never feeds width).
/// NOTE: this is a SHALLOW fold over exactly the trees elaborate synthesizes:
///   - direct `Const` (replicate count, PartIdxUp/Down width);
///   - `Add(lhs, rhs)` — the OUTER node of `[msb:lsb]` width `Add(Sub(msb,lsb),1)`;
///   - `Sub(lhs, rhs)` — the INNER `msb - lsb` node.
///
/// `Mul`/other shapes → None (caller falls back to width 1; eval clamps at run).
pub(crate) fn const_u32_of_expr(ir: &SimIr, eid: u32) -> Option<u32> {
    match &ir.exprs[eid as usize] {
        Expr::Const { val } => {
            let c = &ir.consts[*val as usize];
            // value-plane (`bits.val: Vec<u64>`) word0, no X/Z. Reject if any
            // unknown bit set. Anti-wrap (§10): a count/width above u32::MAX would
            // be silently truncated by `as u32`, so CLAMP to WIDTH_MAX whenever any
            // word >= 1 is nonzero OR word0 > u32::MAX, rather than wrap.
            if c.bits.unk.iter().any(|&u| u != 0) {
                return None;
            }
            let word0 = c.bits.val.first().copied().unwrap_or(0);
            let high_words_set = c.bits.val.iter().skip(1).any(|&v| v != 0);
            if high_words_set || word0 > u32::MAX as u64 {
                return Some(WIDTH_MAX);
            }
            Some((word0 as u32).min(WIDTH_MAX))
        }
        // OUTER node of the `[msb:lsb]` width tree: `(msb - lsb) + 1`.
        Expr::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            let a = const_u32_of_expr(ir, *lhs)?;
            let b = const_u32_of_expr(ir, *rhs)?;
            Some(clamp_w(a.saturating_add(b)))
        }
        // INNER node of the `[msb:lsb]` width tree: `msb - lsb`.
        Expr::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            let a = const_u32_of_expr(ir, *lhs)?;
            let b = const_u32_of_expr(ir, *rhs)?;
            Some(a.saturating_sub(b))
        }
        _ => None,
    }
}
