//! size-cast CONTEXT lowering — `N'(expr)` over a context-determined operation
//! (§4.5.212 width, §4.5.316 `max(self, N)` + sign propagation, §4.5.317 the real
//! refusal funnel). Split out of `expr_cast.rs` at the 1000-line policy; the cast
//! ARMS themselves stay there and enter here through `lower_size_ctx_entry`.

use super::*;

/// The ONE spelling of the size-cast real refusal. It lived at two sites before
/// §4.5.317 and the funnel would have made it three.
pub(crate) const REAL_SIZE_CAST_MSG: &str =
    "size cast is not defined on a real operand (use int'/longint')";

impl Elaborator<'_> {
    /// §4.5.212: is `e` a CONTEXT-DETERMINED operation whose result width the size
    /// cast `N'(e)` must drive down into (arith/bitwise/shift/`**`, unary `+`/`-`/`~`,
    /// ternary)? Such an operation computes at its operands' self-width by default, so
    /// a cast to a WIDER N would silently lose the carry (`8'(a*b)` = 13 vs 45). A bare
    /// leaf, select, concat, or comparison is SELF-determined — the cast just resizes
    /// its already-correct value, so the existing fill-only path stays byte-identical.
    pub(crate) fn is_size_ctx_operation(e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Paren { inner } => Self::is_size_ctx_operation(inner),
            ast::ExprKind::Binary { op, .. } => matches!(
                op,
                ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::Div
                    | ast::BinOp::Mod
                    | ast::BinOp::BitAnd
                    | ast::BinOp::BitOr
                    | ast::BinOp::BitXor
                    | ast::BinOp::BitXnor
                    | ast::BinOp::Shl
                    | ast::BinOp::Shr
                    | ast::BinOp::AShl
                    | ast::BinOp::AShr
                    | ast::BinOp::Pow
            ),
            ast::ExprKind::Unary { op, .. } => {
                matches!(op, ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot)
            }
            ast::ExprKind::Ternary { .. } => true,
            _ => false,
        }
    }

    /// §4.5.212: the OVERALL self-signedness of a size-cast operand's tree — signed iff
    /// EVERY context-determined leaf is signed (§11.8.1: any unsigned operand makes the
    /// whole expression unsigned, and ALL leaves are then zero-extended — verified
    /// against iverilog: `8'((signed*signed)+unsigned)` zero-extends the signed pair).
    /// A single sign therefore governs every leaf's extension. `None` ⇒ a leaf whose
    /// sign can't be resolved here (a param / call / package ref) → the caller keeps the
    /// current fill-only behavior (no regression). Mirrors `expr_self_signed`'s rules.
    pub(crate) fn ast_ctx_signed(&self, e: &ast::Expr) -> Option<bool> {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.ast_ctx_signed(inner),
            ast::ExprKind::Binary { op, lhs, rhs } => match op {
                ast::BinOp::Add
                | ast::BinOp::Sub
                | ast::BinOp::Mul
                | ast::BinOp::Div
                | ast::BinOp::Mod
                | ast::BinOp::BitAnd
                | ast::BinOp::BitOr
                | ast::BinOp::BitXor
                | ast::BinOp::BitXnor => {
                    Some(self.ast_ctx_signed(lhs)? && self.ast_ctx_signed(rhs)?)
                }
                // power / shifts: sign follows the LEFT (base) operand only.
                ast::BinOp::Pow
                | ast::BinOp::Shl
                | ast::BinOp::Shr
                | ast::BinOp::AShl
                | ast::BinOp::AShr => self.ast_ctx_signed(lhs),
                // comparison / logical / wildcard: a 1-bit UNSIGNED result.
                _ => Some(false),
            },
            ast::ExprKind::Unary { op, operand } => match op {
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    self.ast_ctx_signed(operand)
                }
                _ => Some(false), // reductions / logical-not: 1-bit unsigned
            },
            ast::ExprKind::Ternary { then_e, else_e, .. } => {
                Some(self.ast_ctx_signed(then_e)? && self.ast_ctx_signed(else_e)?)
            }
            // bit/part-select, concat, replicate are ALWAYS unsigned (§5.4.1).
            ast::ExprKind::Concat { .. }
            | ast::ExprKind::Replicate { .. }
            | ast::ExprKind::BitSelect { .. }
            | ast::ExprKind::PartSelect { .. }
            | ast::ExprKind::IndexedPart { .. } => Some(false),
            ast::ExprKind::IntLit { kind, raw } => {
                if literal::is_fill_literal(raw, *kind) {
                    return Some(false);
                }
                literal::parse_int_literal(raw, *kind).map(|c| c.signed)
            }
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                let net = self.lookup_net_scoped(&p.segments[0].name)?;
                Some(self.nets.get(net as usize)?.signed)
            }
            // params / calls / sysfuncs / package refs / patterns → indeterminate here.
            _ => None,
        }
    }

    /// §4.5.212: lower a size-cast operand in CONTEXT WIDTH `n`, recursing the
    /// context-determined operator structure so each leaf is widened to `n` BEFORE the
    /// operation (so `8'(a*b)` multiplies two 8-bit operands = 45, not a 4-bit 13). The
    /// operand-vs-shift-amount / self-determined split mirrors `lower_expr_ctx`; `ext`
    /// (the whole operand's sign, from `ast_ctx_signed`) governs every leaf's extension.
    /// PRIVATE on purpose: the `size_cast_real_reported` invariant requires every
    /// entry to go through [`Self::lower_size_ctx_entry`], which saves and restores
    /// the flag. A `pub(crate)` caller could set it with nothing to restore it, and
    /// a leaked `true` SUPPRESSES a later cast's diagnostic — loud→silent.
    fn lower_size_ctx(&mut self, e: &ast::Expr, n: u32, ext: bool) -> u32 {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.lower_size_ctx(inner, n, ext),
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let irop = map_binop(*op);
                match op {
                    ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::BitAnd
                    | ast::BinOp::BitOr
                    | ast::BinOp::BitXor
                    | ast::BinOp::BitXnor => {
                        // both operands context-determined at n.
                        let l = self.lower_size_ctx(lhs, n, ext);
                        let r = self.lower_size_ctx(rhs, n, ext);
                        self.push_expr(ir::Expr::Binary {
                            op: irop,
                            lhs: l,
                            rhs: r,
                        })
                    }
                    // `/ % >> >>>` — the four whose value depends on bits ABOVE the
                    // ones they produce. IEEE 1800 §11.8.1 evaluates a context-determined
                    // operand at **max(self, n)**: the context can only WIDEN. For the
                    // operators above that distinction is invisible (their low n bits are
                    // decided by the operands' low n bits, so narrowing is the same
                    // computation), which is why §4.5.212 could pass `n` straight down
                    // and only the widening half was ever measured. Here it is visible in
                    // both directions:
                    //
                    //   NARROW  `2'(k%4)` with `integer k` — at n=2 the divisor `4`
                    //           becomes `0` and the answer is `xx`; both oracles say `11`.
                    //   WIDEN   `8'(b>>1)` with `b = -4'sd8` — the context's sign bits are
                    //           part of the result (`01111100`); at self width it is
                    //           `00000100`. `8'(b/c)` with `c = -4'sd1` overflows at 4 bits
                    //           and not at 8. A div-by-zero's `x` must fill all n bits.
                    //
                    // So neither `n` nor the self width is right on its own. `plain`
                    // is lowered as a WIDTH PROBE — `w` is the only thing read out of
                    // it, on both paths — and then the branch builds the lowering the
                    // standard selects. The probe's nodes are dead: nothing references
                    // them, so no value is evaluated twice and no `$random` draw is
                    // duplicated (measured). ⚠️ They are not free either: the probe
                    // still emits DIAGNOSTICS, so an error inside a cast operand is
                    // reported twice, and its dead nodes are serialized into the
                    // artifact (a depth-32 nest grows the `.velab` 5.7×). Recorded in
                    // ROADMAP §2 — the standing fix is an AST self-width pass that
                    // answers `w` without lowering at all.
                    // (Widening PAST max(self, n) is wrong too: a 32-bit context makes
                    // `5'(s4>>u3)` 30 instead of 6.)
                    ast::BinOp::Div | ast::BinOp::Mod | ast::BinOp::Shr | ast::BinOp::AShr => {
                        let plain = self.lower_ctx_or_plain(e, n);
                        let w = self.ir_bits_of(plain).unwrap_or(n);
                        if n >= w {
                            // context WIDENS (or matches): operands at n, as before.
                            let l = self.lower_size_ctx(lhs, n, ext);
                            let r = if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) {
                                self.lower_size_ctx(rhs, n, ext)
                            } else {
                                let r = self.lower_expr(rhs); // shift amount is self-determined
                                self.refuse_real_size_operand(r)
                            };
                            self.push_expr(ir::Expr::Binary {
                                op: irop,
                                lhs: l,
                                rhs: r,
                            })
                        } else {
                            // context NARROWS: the node keeps its own WIDTH — but
                            // §11.8.1 still coerces its operands to the EXPRESSION's
                            // signedness, and for `>>>` that changes the operation
                            // itself: an unsigned left operand shifts LOGICALLY
                            // (§11.4.10). A self-determined lowering keeps the operand's
                            // own sign, so `4'(u8 + (s8 >>> 9))` kept the arithmetic fill
                            // and printed `0010` where both oracles say `0011`. `/ % >>`
                            // do not show it because the plain path already applies the
                            // context's unsignedness to them; `>>>` was the one left.
                            // The fill context is `w`, NOT `n`. §11.6 sizes an
                            // unsized fill to the EXPRESSION's width, and this branch
                            // runs only when `w > n`, so `n` is always too narrow here:
                            // `2'(k / '1)` built the divisor as 2 ones instead of 32 and
                            // printed 2 where both oracles print 0.
                            // The refusal is placed before `coerce_sign` for reading
                            // order only. It was load-bearing in the first cut —
                            // `$unsigned(real)` hid the real from every later check —
                            // but the `$signed`/`$unsigned` pass-through arm added to
                            // `expr_is_real` in this same slice makes
                            // `refuse(coerce_sign(x))` fire exactly when `refuse(x)`
                            // does, so swapping them is now provably a no-op (a
                            // reviewer's swap mutation passes every gate). Do not
                            // read this order as a constraint.
                            let l = self.lower_ctx_or_plain(lhs, w);
                            let l = self.refuse_real_size_operand(l);
                            let l = self.coerce_sign(l, ext);
                            let r = if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) {
                                let r = self.lower_ctx_or_plain(rhs, w);
                                let r = self.refuse_real_size_operand(r);
                                self.coerce_sign(r, ext)
                            } else {
                                let r = self.lower_expr(rhs); // shift amount is self-determined
                                self.refuse_real_size_operand(r)
                            };
                            let narrow_op = if matches!(op, ast::BinOp::AShr) && !ext {
                                ir::BinOp::Shr
                            } else {
                                irop
                            };
                            let _ = plain; // the width probe; nothing references it
                            self.push_expr(ir::Expr::Binary {
                                op: narrow_op,
                                lhs: l,
                                rhs: r,
                            })
                        }
                    }
                    ast::BinOp::Pow | ast::BinOp::Shl | ast::BinOp::AShl => {
                        // base context-determined at n; exponent / shift amount SELF-determined.
                        let l = self.lower_size_ctx(lhs, n, ext);
                        let r = self.lower_expr(rhs);
                        let r = self.refuse_real_size_operand(r);
                        self.push_expr(ir::Expr::Binary {
                            op: irop,
                            lhs: l,
                            rhs: r,
                        })
                    }
                    // comparison / logical: a 1-bit self-determined result → extend as a leaf.
                    _ => self.lower_size_leaf(e, n, ext),
                }
            }
            ast::ExprKind::Unary { op, operand } => match op {
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    let o = self.lower_size_ctx(operand, n, ext);
                    self.push_expr(ir::Expr::Unary {
                        op: map_unop(*op),
                        operand: o,
                    })
                }
                _ => self.lower_size_leaf(e, n, ext),
            },
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // The condition is self-determined AND may legally be real — it is
                // tested for nonzero, never resized. Deliberately NOT funnelled
                // through `refuse_real_size_operand`: `4'(r ? a : b)` matches
                // iverilog today, including the `-0.0` discriminator (both `0011`).
                let c = self.lower_expr(cond);
                let t = self.lower_size_ctx(then_e, n, ext);
                let f = self.lower_size_ctx(else_e, n, ext);
                self.push_expr(ir::Expr::Ternary {
                    cond: c,
                    then_e: t,
                    else_e: f,
                })
            }
            // a self-determined value (leaf / select / concat / call) → resize to n.
            _ => self.lower_size_leaf(e, n, ext),
        }
    }

    /// §4.5.317: refuse a REAL value that has reached a size-cast OPERAND position.
    /// Every operand [`Self::lower_size_ctx`] builds funnels through here. If one is
    /// allowed through, the resize / `Binary` node below reads the f64 as a bit
    /// vector, so `4'(r*2)` with `r = 7.5` printed `0000` (the true 15 is `1111`)
    /// and `8'(u/r)` printed `00110011` — silently, at exit 0.
    ///
    /// It has to be the funnel and not one site: the first cut guarded only
    /// `lower_size_leaf`, and `4'(a << r)` (shift amount, `lower_expr`) plus
    /// `8'(u / r)` (the `Div` NARROW branch, `lower_ctx_or_plain`) stayed silent —
    /// while the very same `a << r` OUTSIDE a cast was already loud. A second
    /// spelling of "operand" is exactly how the gate lost them.
    ///
    /// The cast arms' own `cast_operand_is_real` catches SOME of these already (it
    /// shares `expr_is_real`, so a `Div`/`Pow`-rooted operand and a bare `4'(r)` were
    /// loud before this) — what it cannot see is an operand whose leaves this
    /// lowering has already wrapped in `Select`/`Concat`, or one rooted at `Mod` /
    /// a shift / a bitwise op, none of which are in that predicate's `Binary` arm.
    ///
    /// ⚠️ SCOPE: this covers every operand the size-context lowering BUILDS, not
    /// every real that can reach a cast. When `ast_ctx_signed` cannot resolve a leaf
    /// (a real param, a real literal, a function return, `$realtime`, `$sqrt`) the
    /// cast arm never enters `lower_size_ctx` at all, and if the operator is also
    /// outside `expr_is_real`'s `Binary` arm the cast is still silent —
    /// `4'(RP ^ '0)` measures at exit 0 where iverilog refuses. That class is
    /// PRE-EXISTING and needs the tree-wide AST self-width/domain pass; ROADMAP §2.
    ///
    /// iverilog rejects all of these (`Cast base expression must be a vector type`),
    /// and it rejects a bare `4'(r)` too — so this is oracle parity, not a new
    /// restriction and not a capability gap. (verilator accepts them with a `REALCVT`
    /// warning and computes `4'(r*2)` = `1111`, so PRE was wrong against it too.)
    fn refuse_real_size_operand(&mut self, x: u32) -> u32 {
        if !self.expr_is_real(x) {
            return x;
        }
        if !self.size_cast_real_reported {
            self.size_cast_real_reported = true;
            self.error(MsgCode::ElabUnsupported, REAL_SIZE_CAST_MSG);
        }
        self.placeholder_expr()
    }

    /// The two cast arms' entry into [`Self::lower_size_ctx`]. Scopes the
    /// "already reported" flag to ONE cast (iverilog reports the cast, not each
    /// leaf) while leaving a NESTED cast free to report its own. No early return
    /// inside, so the restore cannot be skipped.
    pub(crate) fn lower_size_ctx_entry(&mut self, e: &ast::Expr, n: u32, ext: bool) -> u32 {
        let outer = std::mem::replace(&mut self.size_cast_real_reported, false);
        let out = self.lower_size_ctx(e, n, ext);
        self.size_cast_real_reported = outer;
        out
    }

    /// Coerce an already-lowered operand to the enclosing expression's signedness
    /// WITHOUT changing its width — §11.8.1 propagates the sign to every operand even
    /// when the context does not widen, and for `>>>` that decides whether the shift
    /// is arithmetic or logical (§11.4.10). The width-changing twin is `lower_size_leaf`.
    fn coerce_sign(&mut self, x: u32, ext: bool) -> u32 {
        let cur = self.expr_self_signed(x);
        if ext == cur {
            return x;
        }
        let which = if ext {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![x],
        })
    }

    /// §4.5.212: resize a self-determined leaf to the cast width `n`, extending with the
    /// operand's overall sign `ext` (NOT the leaf's own sign — §11.8.1 coerces every
    /// leaf to the expression's signedness). Re-stamps `$signed`/`$unsigned` so signed
    /// division / arithmetic-shift / comparison in the enclosing op use the right
    /// semantics (a `Concat`/`Select` extension is otherwise always unsigned).
    /// Coerce an already-lowered operand to the enclosing expression's signedness
    /// WITHOUT changing its width — §11.8.1 propagates the sign to every operand even
    /// when the context does not widen. The width-changing twin is `lower_size_leaf`.
    fn lower_size_leaf(&mut self, e: &ast::Expr, n: u32, ext: bool) -> u32 {
        let x = self.lower_expr(e);
        let x = self.refuse_real_size_operand(x);
        let w = self.ir_bits_of(x).unwrap_or(32);
        let resized = match n.cmp(&w) {
            std::cmp::Ordering::Equal => x,
            std::cmp::Ordering::Greater => self.extend_to(x, w, n, ext),
            std::cmp::Ordering::Less => self.select_low(x, n),
        };
        let cur_signed = self.expr_self_signed(resized);
        if ext && !cur_signed {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed,
                args: vec![resized],
            })
        } else if !ext && cur_signed {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Unsigned,
                args: vec![resized],
            })
        } else {
            resized
        }
    }
}
