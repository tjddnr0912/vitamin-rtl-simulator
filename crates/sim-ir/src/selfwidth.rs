//! Self-determined width and signedness of an expression (IEEE 1364-2005
//! §5.4.1 / §5.5) — the CANONICAL rule, and the only spelling of it.
//!
//! It lives in `sim-ir` rather than in the engine because two crates need the
//! same answer and a second spelling is how they drift. `sim-engine`'s
//! `WidthTable` drives this over the whole arena and memoizes; `elaborate` asks
//! it about ONE expression while lowering, to decide whether an index it is
//! about to normalize is signed (§4.5.309). The previous arrangement had
//! elaborate carrying a hand-written CONSERVATIVE subset of the sign half, and
//! three of its arms were measurably wrong — `**` (the base's sign alone), a
//! class field (whose sign is a sidecar, not its handle net's), and a real
//! literal — each a silent-wrong until review found it.
//!
//! Pure analysis over the frozen IR: no `#[derive(SchemaHash)]` type is
//! defined or moved here, so the artifact hashes are untouched (same class as
//! `analysis.rs`).
//!
//! The one thing this cannot know by itself is a user function's declared
//! return type, which lives in a sidecar on either side — so it is a
//! parameter: `call_ret(func_id) -> Option<(width, signed)>`.

use crate as sim_ir;
use sim_ir::{BinOp, ConstRepr, ConstVal, Expr, NetVar, SelKind, SimIr, SysFuncId, UnOp};

/// The three arenas the rule reads. A struct rather than `&SimIr` because
/// `elaborate` asks this WHILE BUILDING the IR and has no `SimIr` yet — and a
/// second spelling for that caller is exactly what this module exists to
/// prevent.
#[derive(Clone, Copy)]
pub struct ExprCtx<'a> {
    pub exprs: &'a [Expr],
    pub consts: &'a [ConstVal],
    pub nets: &'a [NetVar],
}

impl<'a> ExprCtx<'a> {
    /// The whole-IR view, for callers that do have one.
    pub fn of(ir: &'a SimIr) -> Self {
        ExprCtx {
            exprs: &ir.exprs,
            consts: &ir.consts,
            nets: &ir.nets,
        }
    }
}

/// `width` is the bottom-up self-width; `signed` is the self-signedness
/// (the both-signed rule already folded in for context-determined operators).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfWidth {
    pub width: u32,
    pub signed: bool,
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

fn clamp_w(w: u32) -> u32 {
    w.min(WIDTH_MAX)
}

pub const WIDTH_MAX: u32 = 1 << 24;

/// Resolve an expr to a compile-time u32 if it is a literal const (the v1
/// elaborate guarantee for part-select width and replicate count). Returns
/// None for non-const trees (e.g. a runtime offset, which never feeds width).
/// NOTE: this is a SHALLOW fold over exactly the trees elaborate synthesizes:
///   - direct `Const` (replicate count, PartIdxUp/Down width);
///   - `Add(lhs, rhs)` — the OUTER node of `[msb:lsb]` width `Add(Sub(msb,lsb),1)`;
///   - `Sub(lhs, rhs)` — the INNER `msb - lsb` node.
///
/// `Mul`/other shapes → None (caller falls back to width 1; eval clamps at run).
pub fn const_u32_of_expr_ctx(ir: ExprCtx, eid: u32) -> Option<u32> {
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
            let a = const_u32_of_expr_ctx(ir, *lhs)?;
            let b = const_u32_of_expr_ctx(ir, *rhs)?;
            Some(clamp_w(a.saturating_add(b)))
        }
        // INNER node of the `[msb:lsb]` width tree: `msb - lsb`.
        Expr::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            let a = const_u32_of_expr_ctx(ir, *lhs)?;
            let b = const_u32_of_expr_ctx(ir, *rhs)?;
            Some(a.saturating_sub(b))
        }
        // `$clog2(k)` of a SINGLE constant argument (a literal, or a param folded to
        // a Const) is a legal constant replication count / part-select width (IEEE
        // §11.4.12.2). Without this arm `$clog2` survives lowering as a `SysFunc`
        // node → `None` → a silent 0-width replication (`{$clog2(8){2'b10}}` printed
        // `00000000` instead of `00101010`). For a SINGLE operand the result depends
        // ONLY on the argument's value — width and signedness are irrelevant (there
        // is no arithmetic, hence no wraparound), so this is bit-exact regardless of
        // vita's self-width inference. An ARITHMETIC argument (`$clog2(N+1)`) is
        // deliberately NOT folded: SV width-limited constant arithmetic, compounded
        // by the pre-existing derived-localparam value-inferred width (elaborate's
        // `param_decl_width` → ≥ 32 for an expression initializer), makes it unsafe
        // to reproduce bit-exactly here — elaborate folds such a count to a `Const`
        // or refuses it (`edge_gate.rs`, §4.5.601), NEVER a wrong non-zero one.
        Expr::SysFunc {
            which: SysFuncId::Clog2,
            args,
        } if args.len() == 1 => {
            let Expr::Const { val } = &ir.exprs[args[0] as usize] else {
                return None;
            };
            let c = &ir.consts[*val as usize];
            // Fold ONLY a genuine numeric integer Const. A REAL literal's `bits.val[0]`
            // is the raw IEEE-754 f64 bit pattern (`$clog2(8.0)` would misread
            // `0x4020…0000` as a huge int; iverilog rounds the real first), and a
            // string literal's bits are byte data — neither is an integer clog2
            // argument, so decline (→ silent 0-width, base==fix).
            if !matches!(c.repr, ConstRepr::Numeric) {
                return None;
            }
            // Reject X/Z, a value beyond one u64 lane, or a SIGNED-NEGATIVE value
            // (`$clog2` of a negative widens to a ≥ 32-bit integer — unmodeled here).
            if c.bits.unk.iter().any(|&u| u != 0) || c.bits.val.iter().skip(1).any(|&v| v != 0) {
                return None;
            }
            let n = c.bits.val.first().copied().unwrap_or(0);
            if c.signed && c.width >= 1 && c.width <= 64 && (n >> (c.width - 1)) & 1 == 1 {
                return None;
            }
            // clog2(n) = 0 for n ≤ 1, else the top set-bit position of n-1, plus one
            // (byte-identical to the engine's runtime `SysFuncId::Clog2`). The result
            // (≤ 64) is well within WIDTH_MAX, so no clamp is needed.
            Some(if n <= 1 {
                0
            } else {
                64 - (n - 1).leading_zeros()
            })
        }
        _ => None,
    }
}

#[inline]
fn add_w(a: u32, b: u32) -> u32 {
    clamp_w(a.saturating_add(b))
}

#[inline]
fn mul_w(a: u32, b: u32) -> u32 {
    clamp_w(a.saturating_mul(b))
}

/// One expression's self-width and self-signedness, given every CHILD's
/// already computed in `sw` (the arena is post-order, §1).
pub fn self_width_of(
    ir: ExprCtx,
    call_ret: &dyn Fn(u32) -> Option<(u32, bool)>,
    sw: &[SelfWidth],
    i: u32,
) -> SelfWidth {
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
            // `.get`, not `[..]`: `elaborate` asks this rule WHILE BUILDING the
            // IR, and a deferred hierarchical select is a placeholder `Signal`
            // carrying `POISON_NET` (`u32::MAX`) until it is resolved. Indexing
            // panicked on it — measured, and the whole hierarchical
            // container-read suite caught it. The engine cannot reach the arm
            // (it runs post-resolve), so the fallback changes no answer it
            // could ever have produced; it only turns a panic into the same
            // 1-bit unsigned default a `Call` with no sidecar takes.
            //
            // `word` selects an ARRAY element; element width == NetVar.width.
            match ir.nets.get(*net as usize) {
                Some(nv) => SelfWidth {
                    width: clamp_w(nv.width.max(1)),
                    signed: nv.signed,
                },
                None => SelfWidth {
                    width: 1,
                    signed: false,
                },
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
                _ => const_u32_of_expr_ctx(ir, *width).unwrap_or(1),
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
            let n = const_u32_of_expr_ctx(ir, *count).unwrap_or(0);
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
            // v34: the real→int assignment conversion delivers a 128-bit signed
            // integer; the consumer resizes it.
            SysFuncId::RealToInt => SelfWidth {
                width: 128,
                signed: true,
            },
            // v34: the 2-state store keeps the operand's width AND sign.
            SysFuncId::TwoState => args
                .first()
                .map(|&a| child(sw, i, a))
                .unwrap_or(SelfWidth {
                    width: 1,
                    signed: false,
                }),
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
            | SysFuncId::StrToLower
            // v33: `string'(e)` is the same family — its width is 8×len at eval.
            | SysFuncId::StrCast => SelfWidth {
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
        Expr::Call { func, .. } => call_ret(*func)
            .map(|(w, s)| SelfWidth {
                width: clamp_w(w.max(1)),
                signed: s,
            })
            .unwrap_or(SelfWidth {
                width: 1,
                signed: false,
            }),
    }
}

/// `const_u32_of_expr_ctx` for callers holding a whole `SimIr`.
pub fn const_u32_of_expr(ir: &SimIr, eid: u32) -> Option<u32> {
    const_u32_of_expr_ctx(ExprCtx::of(ir), eid)
}

/// Which constant edge [`edge_value`] decides. The eids are the LOWERED bounds: the
/// two bounds of a `[m:l]` select (in the declared direction of what it selects), the
/// width of a `[b +: w]` / `[b -: w]` select, the count of a replication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeSpec {
    /// `[msb:lsb]`: the width is `|msb - lsb| + 1` in the declared direction.
    Part { msb: u32, lsb: u32, desc: bool },
    /// `[b +: w]` / `[b -: w]`.
    Indexed { w: u32 },
    /// `{n{…}}`.
    Count { n: u32 },
}

/// Why an edge has no decided value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeFault {
    /// The tree holds a node outside [`const_u32_of_expr_ctx`]'s arms (a read, a call,
    /// `*`, a cast…): the engine cannot read it, so it is not decided here either.
    Unreduced,
    /// The value is negative: a negative count or width, or `[m:l]` bounds out of order
    /// for the declared direction.
    Negative,
    /// A leaf has x or z bits.
    Unknown,
}

/// The region a bound forms (IEEE 1800 §11.6.1, §11.8.1-2): the width and signedness
/// [`self_width_of`] gives its root, over exactly the arms of
/// [`const_u32_of_expr_ctx`] — a `Numeric` constant without x/z, `+`, `-`, and `$clog2`
/// of one constant with the engine's own decline set.
fn edge_region(ir: ExprCtx, eid: u32) -> Result<(u32, bool), EdgeFault> {
    match &ir.exprs[eid as usize] {
        Expr::Const { val } => {
            let c = &ir.consts[*val as usize];
            if !matches!(c.repr, ConstRepr::Numeric) {
                return Err(EdgeFault::Unreduced);
            }
            if c.bits.unk.iter().any(|&u| u != 0) {
                return Err(EdgeFault::Unknown);
            }
            Ok((clamp_w(c.width.max(1)), c.signed))
        }
        Expr::Binary {
            op: BinOp::Add | BinOp::Sub,
            lhs,
            rhs,
        } => {
            let (a, sa) = edge_region(ir, *lhs)?;
            let (b, sb) = edge_region(ir, *rhs)?;
            // arithmetic: max(L, R), signed iff BOTH signed (the `self_width_of` arm)
            Ok((a.max(b), sa && sb))
        }
        Expr::SysFunc {
            which: SysFuncId::Clog2,
            args,
        } if args.len() == 1 => {
            const_u32_of_expr_ctx(ir, eid).ok_or(EdgeFault::Unreduced)?;
            // integer return: the `self_width_of` `$clog2` arm
            Ok((32, true))
        }
        _ => Err(EdgeFault::Unreduced),
    }
}

/// Bit `b` of a word vector.
fn word_bit(v: &[u64], b: u32) -> bool {
    v.get((b / 64) as usize)
        .is_some_and(|w| (w >> (b % 64)) & 1 == 1)
}

/// A leaf's `lw` bits extended to the region's `w` bits: by the leaf's sign only when the
/// region is signed (§11.8.2: an operand is sign-extended only if the propagated type is
/// signed).
fn edge_extend(bits: &[u64], lw: u32, w: u32, signed: bool) -> Vec<u64> {
    let keep = lw.min(w);
    let mut v = sim_ir::mw::mw_mask(bits.to_vec(), keep.max(1));
    if keep == 0 {
        v.iter_mut().for_each(|x| *x = 0);
    }
    v.resize(w.div_ceil(64).max(1) as usize, 0);
    if signed && keep >= 1 && word_bit(bits, keep - 1) {
        for b in keep..w {
            v[(b / 64) as usize] |= 1u64 << (b % 64);
        }
    }
    sim_ir::mw::mw_mask(v, w)
}

/// The region's value bits (`w` wide), every operation wrapping at `w`.
fn edge_eval(ir: ExprCtx, eid: u32, w: u32, signed: bool) -> Vec<u64> {
    match &ir.exprs[eid as usize] {
        Expr::Const { val } => {
            let c = &ir.consts[*val as usize];
            edge_extend(&c.bits.val, clamp_w(c.width.max(1)), w, signed)
        }
        Expr::Binary { op, lhs, rhs } => {
            let a = edge_eval(ir, *lhs, w, signed);
            let b = edge_eval(ir, *rhs, w, signed);
            let b = if matches!(op, BinOp::Sub) {
                sim_ir::mw::mw_neg(&b)
            } else {
                b
            };
            sim_ir::mw::mw_mask(sim_ir::mw::mw_add(&a, &b), w)
        }
        // `$clog2`: the engine arm's own answer, a non-negative 32-bit integer.
        _ => {
            let v = const_u32_of_expr_ctx(ir, eid).unwrap_or(0);
            edge_extend(&[u64::from(v)], 32, w, signed)
        }
    }
}

/// The value of one bound: its region evaluated at the region's width and read at the
/// region's signedness. Saturates at `i128::MAX` / `i128::MIN` past 127 bits of magnitude
/// (every consumer of such a value refuses or clamps it).
pub fn edge_bound_value(ir: ExprCtx, eid: u32) -> Result<i128, EdgeFault> {
    let (w, signed) = edge_region(ir, eid)?;
    let v = edge_eval(ir, eid, w, signed);
    let negative = signed && word_bit(&v, w - 1);
    let mag = if negative {
        sim_ir::mw::mw_mask(sim_ir::mw::mw_neg(&v), w)
    } else {
        v
    };
    let wide = mag.iter().skip(2).any(|&x| x != 0) || mag.get(1).is_some_and(|&x| x >> 63 != 0);
    let small = if wide {
        i128::MAX
    } else {
        let lo = u128::from(mag.first().copied().unwrap_or(0));
        let hi = u128::from(mag.get(1).copied().unwrap_or(0));
        i128::try_from((hi << 64) | lo).unwrap_or(i128::MAX)
    };
    Ok(if negative { -small } else { small })
}

/// The decided value of a constant edge (§4.5.601): each bound one self-determined region
/// (its own width and signedness, every operation wrapping there), a `[m:l]` width
/// `hi - lo + 1` in the declared direction. Saturates at `u64::MAX`.
pub fn edge_value(ir: ExprCtx, spec: EdgeSpec) -> Result<u64, EdgeFault> {
    match spec {
        EdgeSpec::Part { msb, lsb, desc } => {
            let m = edge_bound_value(ir, msb)?;
            let l = edge_bound_value(ir, lsb)?;
            part_width(m, l, desc)
        }
        EdgeSpec::Indexed { w: e } | EdgeSpec::Count { n: e } => {
            let v = edge_bound_value(ir, e)?;
            if v < 0 {
                return Err(EdgeFault::Negative);
            }
            Ok(u64::try_from(v).unwrap_or(u64::MAX))
        }
    }
}

/// `|m - l| + 1` in the declared direction; bounds out of order are [`EdgeFault::Negative`].
pub fn part_width(m: i128, l: i128, desc: bool) -> Result<u64, EdgeFault> {
    let (hi, lo) = if desc { (m, l) } else { (l, m) };
    if hi < lo {
        return Err(EdgeFault::Negative);
    }
    let w = hi.saturating_sub(lo).saturating_add(1);
    Ok(u64::try_from(w).unwrap_or(u64::MAX))
}

/// Every constant EDGE the engine folds with [`const_u32_of_expr_ctx`]: each part or
/// indexed select's width and each replication's count in `exprs`, and each part
/// chunk's width in the targets of `stmts` and `cont_assigns` and in `call_outs`, the
/// caller lvalues a task call copies its outputs to (the `out_binds` of the task-call
/// side tables, which live outside the statement arena). A part chunk with no width
/// edge is reported as `None` (its readers write the whole net).
pub fn for_each_constant_edge<'a>(
    exprs: &[Expr],
    stmts: &'a [sim_ir::Stmt],
    cont_assigns: &'a [sim_ir::ContAssign],
    call_outs: impl IntoIterator<Item = &'a sim_ir::Lvalue>,
    mut f: impl FnMut(Option<u32>),
) {
    for e in exprs {
        match e {
            Expr::Select { width, kind, .. } if *kind != SelKind::Bit => f(Some(*width)),
            Expr::Replicate { count, .. } => f(Some(*count)),
            _ => {}
        }
    }
    let stmt_targets = stmts.iter().filter_map(|s| match s {
        sim_ir::Stmt::BlockingAssign { lhs, .. }
        | sim_ir::Stmt::NonblockingAssign { lhs, .. }
        | sim_ir::Stmt::Force { lhs, .. }
        | sim_ir::Stmt::Release { lhs } => Some(lhs),
        sim_ir::Stmt::SysTask { .. } | sim_ir::Stmt::Disable { .. } => None,
    });
    let targets = stmt_targets
        .chain(cont_assigns.iter().map(|c| &c.lhs))
        .chain(call_outs);
    for lv in targets {
        for c in &lv.chunks {
            if c.kind != SelKind::Bit {
                f(c.width);
            }
        }
    }
}

/// [`for_each_constant_edge`] with each edge handed out to be repointed: every holder the
/// engine reads (a select's width, a replication's count, a part chunk's width in a
/// statement target, a continuous-assign target and a task call's output targets).
pub fn for_each_constant_edge_mut<'a>(
    exprs: &mut [Expr],
    stmts: &'a mut [sim_ir::Stmt],
    cont_assigns: &'a mut [sim_ir::ContAssign],
    call_outs: impl IntoIterator<Item = &'a mut sim_ir::Lvalue>,
    mut f: impl FnMut(&mut u32),
) {
    for e in exprs.iter_mut() {
        match e {
            Expr::Select { width, kind, .. } if *kind != SelKind::Bit => f(width),
            Expr::Replicate { count, .. } => f(count),
            _ => {}
        }
    }
    let stmt_targets = stmts.iter_mut().filter_map(|s| match s {
        sim_ir::Stmt::BlockingAssign { lhs, .. }
        | sim_ir::Stmt::NonblockingAssign { lhs, .. }
        | sim_ir::Stmt::Force { lhs, .. }
        | sim_ir::Stmt::Release { lhs } => Some(lhs),
        sim_ir::Stmt::SysTask { .. } | sim_ir::Stmt::Disable { .. } => None,
    });
    let targets = stmt_targets
        .chain(cont_assigns.iter_mut().map(|c| &mut c.lhs))
        .chain(call_outs);
    for lv in targets {
        for c in &mut lv.chunks {
            if c.kind != SelKind::Bit {
                if let Some(w) = &mut c.width {
                    f(w);
                }
            }
        }
    }
}

#[cfg(test)]
mod edge_value_tests;
