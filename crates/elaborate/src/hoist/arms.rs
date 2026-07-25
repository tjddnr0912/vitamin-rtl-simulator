//! ternary / short-circuit ARM lowering — split out of `hoist.rs` to hold it under
//! the 1000-line module cap. A separate inherent `impl` block on `Elaborator`, which
//! is legal precisely because this is an inherent impl and not a trait impl.

use super::*;

impl Elaborator<'_> {
    /// §4.5.216 (round-19 follow-on): is `e` cleanly TRANSFORMABLE as an isolated
    /// arm/operand of a short-circuit rhs split — i.e. any inout/output-formal call it
    /// carries is in a position `hoist_inout_calls` hoists once `e` is the whole
    /// expression of its own block (`!has_unhoistable_inout_call`) AND hoisting it is
    /// eval-order-safe (`hoist_is_safe`)? An `e` with no inout-call is vacuously
    /// transformable. A DEEPER-nested (`u || f(out r)`) or eval-order-unsafe call makes
    /// it false → the caller declines the split → the whole rhs stays loud
    /// (correct-or-loud), never a partial transform.
    pub(crate) fn arm_transformable(&self, e: &ast::Expr) -> bool {
        !self.has_unhoistable_inout_call(e) && self.hoist_is_safe(e)
    }

    /// §4.5.217: `(signed, width)` of a net IFF it is a plain bit-vector coercion
    /// context (Wire/Reg/Logic/Integer — this also covers `int`/`byte`/`bit`/`time`,
    /// all of which map onto those NetKinds); `None` for a string / real / dynamic-
    /// handle net (not a bit-width context ⇒ the coercion gate stays loud).
    fn bitvec_net_ws(&self, net: u32) -> Option<(bool, u32)> {
        let nv = self.nets.get(net as usize)?;
        matches!(
            nv.kind,
            ir::NetKind::Wire | ir::NetKind::Reg | ir::NetKind::Logic | ir::NetKind::Integer
        )
        .then_some((nv.signed, nv.width))
    }

    /// §4.5.217 (round-19 follow-on): the (effective signedness, self-determined width)
    /// of a `?:` arm, for the definite-arm coercion-safety gate. Resolves a single-segment
    /// Ident to its SCOPED net and a single-segment CALL to its function's declared return
    /// type — unlike `ast_ctx_signed`, which is indeterminate on a call, yet a call arm is
    /// the whole reason the split runs. Mirrors the IEEE §11.6.1 / §11.8.1 self-width and
    /// signedness rules of `ast_expr_self_width` + `ast_ctx_signed` in one walk. `None`
    /// (unknown ident/call, string/real/handle net, package/method ref, unfoldable select,
    /// `real` return) ⇒ the arm is treated as NOT coercion-safe (loud), never a guess.
    pub(crate) fn arm_coercion_info(&self, e: &ast::Expr) -> Option<(bool, u32)> {
        use ast::ExprKind::*;
        match &e.kind {
            Paren { inner } => self.arm_coercion_info(inner),
            Ident(p) => {
                let [seg] = p.segments.as_slice() else {
                    return None;
                };
                let net = self.lookup_net_scoped(&seg.name)?;
                self.bitvec_net_ws(net)
            }
            // A single-segment call → its function's declared return (a 2-segment
            // `h.method()` handle-method is unknown here → None). A `real`/`realtime`
            // return width is None ⇒ loud (not a bit-vector coercion context).
            Call { name, .. } => {
                let [seg] = name.segments.as_slice() else {
                    return None;
                };
                let f = self.func_table.get(&seg.name)?;
                Some((f.signed, ast_func_return_width(f)?))
            }
            IntLit { kind, raw } => {
                // A fill literal (`'0`/`'1`) is unsigned and context-filled — it never
                // widens the context (self-width 0), so it can only make the gate loud
                // via a sign mismatch (which is correct: `signed ? '0` is §11.8.1 unsigned).
                if literal::is_fill_literal(raw, *kind) {
                    return Some((false, 0));
                }
                let cv = literal::parse_int_literal(raw, *kind)?;
                Some((cv.signed, cv.width))
            }
            BitSelect { .. } => Some((false, 1)),
            PartSelect { msb, lsb, .. } => {
                let m = ast_decimal_lit_i64(msb)?;
                let l = ast_decimal_lit_i64(lsb)?;
                Some((false, u32::try_from(m.abs_diff(l) + 1).ok()?))
            }
            IndexedPart { width, .. } => {
                Some((false, u32::try_from(ast_decimal_lit_i64(width)?).ok()?))
            }
            Binary { op, lhs, rhs } => {
                use ast::BinOp::*;
                match op {
                    // comparison / logical / wildcard: a 1-bit unsigned result.
                    Lt | Le | Gt | Ge | Eq | Ne | CaseEq | CaseNe | WildEq | WildNe | LogAnd
                    | LogOr => Some((false, 1)),
                    // §11.6.1: a shift / power self width & sign follow the LEFT operand.
                    Shl | Shr | AShl | AShr | Pow => self.arm_coercion_info(lhs),
                    // arithmetic / bitwise: signed iff BOTH operands signed (§11.8.1),
                    // width = max of the two operands.
                    _ => {
                        let (ls, lw) = self.arm_coercion_info(lhs)?;
                        let (rs, rw) = self.arm_coercion_info(rhs)?;
                        Some((ls && rs, lw.max(rw)))
                    }
                }
            }
            Unary { op, operand } => match op {
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    self.arm_coercion_info(operand)
                }
                // reductions / logical-not: 1-bit unsigned.
                _ => Some((false, 1)),
            },
            Ternary { then_e, else_e, .. } => {
                let (ts, tw) = self.arm_coercion_info(then_e)?;
                let (es, ew) = self.arm_coercion_info(else_e)?;
                Some((ts && es, tw.max(ew)))
            }
            // bit/part-select, concat, replicate are ALWAYS unsigned (§5.4.1).
            Concat { parts } => {
                let mut sum: u32 = 0;
                for p in parts {
                    sum = sum.checked_add(self.arm_coercion_info(p)?.1)?;
                }
                Some((false, sum))
            }
            Replicate { count, value } => {
                let c = u32::try_from(ast_decimal_lit_i64(count)?).ok()?;
                let mut sum: u32 = 0;
                for v in value {
                    sum = sum.checked_add(self.arm_coercion_info(v)?.1)?;
                }
                Some((false, c.checked_mul(sum)?))
            }
            _ => None,
        }
    }

    /// §4.5.217: width of a `?:` transform's assignment TARGET for the coercion gate.
    /// Only a plain whole-net Ident (every current transform site, and the common shape)
    /// resolves; a part-select / concat / hierarchical / non-bit-vector target ⇒ `None`
    /// ⇒ the arms are treated as not coercion-safe (loud) rather than risk an over-wide
    /// estimate that would hide a divergence.
    pub(crate) fn ternary_lhs_width(&self, lv: &ast::Lvalue) -> Option<u32> {
        let ast::Lvalue::Ident(p) = lv else {
            return None;
        };
        let [seg] = p.segments.as_slice() else {
            return None;
        };
        let net = self.lookup_net_scoped(&seg.name)?;
        Some(self.bitvec_net_ws(net)?.1)
    }

    /// §4.5.217: are BOTH definite arms of `x = c ? then_e : else_e` COERCION-SAFE to
    /// lower in ISOLATION (`x = then_e` / `x = else_e`), i.e. byte-identical to the
    /// unified bare ternary (IEEE §11.4.11 / §11.8.1)? True iff (1) both arms have the
    /// SAME effective signedness — else §11.8.1 flips the surviving arm between sign- and
    /// zero-extend (a silent low-bit change) — AND (2) `lhs` is at least as wide as BOTH
    /// arms' self width, so the unified context width equals `lhs`'s width and every
    /// widening op (§11.6.1 shift, add carry) sees the SAME width isolated as it does
    /// unified. Any unknown sign/width (either arm or the lhs) ⇒ false (loud), never a
    /// guess. When false the caller declines the split → generic lowering → `emit_frame_call`
    /// → E3009 (correct-or-loud), closing the §4.5.216 definite-arm sign/width silent-wrong.
    pub(crate) fn ternary_arms_coercion_safe(
        &self,
        lhs: &ast::Lvalue,
        then_e: &ast::Expr,
        else_e: &ast::Expr,
    ) -> bool {
        let (Some((ts, tw)), Some((es, ew)), Some(lw)) = (
            self.arm_coercion_info(then_e),
            self.arm_coercion_info(else_e),
            self.ternary_lhs_width(lhs),
        ) else {
            return false;
        };
        ts == es && lw >= tw.max(ew)
    }

    /// §4.5.216: intercept a blocking-assign whose WHOLE rhs is a conditionally-evaluated
    /// output/inout-formal call that `hoist_inout_calls` cannot hoist (it must not be made
    /// unconditional), and lower it as explicit control flow that assigns `lhs` on EVERY
    /// path — so the call's copy-out fires ONLY on the path that reaches it. Two forms:
    ///
    ///   1. a `?:` arm:  `x = c ? f(out r) : g`  (a call in a ternary arm), and
    ///   2. a top-level short-circuit `&&`/`||`:  `x = A && f(out r)` / `x = A || f(out r)`.
    ///
    /// Returns true if it fired (the Blocking arm then returns early). Fires ONLY when the
    /// rhs is exactly one of those two shapes AND every arm/operand is cleanly
    /// `arm_transformable`; a BURIED call (`y = (A && f()) + 1`), a call in a DEEPER
    /// operand, or an eval-order-unsafe arm returns false → the generic path lowers the rhs
    /// with the call in place → loud at `emit_frame_call` (correct-or-loud). Gated on
    /// `delay.is_none()` (an intra-assignment delay is left to the generic path) and on
    /// `inout_func_names` being non-empty (byte-identical for designs with no such function).
    pub(crate) fn shortcircuit_rhs_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        if delay.is_some() || self.inout_func_names.is_empty() {
            return false;
        }
        match &rhs.kind {
            // `x = c ? T : E` with an inout/output-formal call in a CONDITIONALLY-evaluated
            // arm (`then_e` / `else_e`). A call only in `cond` (unconditional) is NOT matched
            // here — it stays loud (a separate follow-on), like an if/loop cond that carries a
            // deeper call.
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } if (self.expr_has_inout_call(then_e) || self.expr_has_inout_call(else_e))
                && self.arm_transformable(cond)
                && self.arm_transformable(then_e)
                && self.arm_transformable(else_e)
                // §4.5.217: the definite-arm transform lowers each taken arm in ISOLATION
                // (`x = T` / `x = E`); that is byte-identical to the unified bare ternary
                // ONLY when the arms are coercion-safe (same effective sign; lhs ≥ both
                // self-widths). Otherwise §11.8.1 sign-flip / §11.6.1 shift-width divergence
                // silently changes the value → decline the split → generic lowering → loud.
                && self.ternary_arms_coercion_safe(lhs, then_e, else_e) =>
            {
                self.lower_ternary_rhs(b, lhs, cond, then_e, else_e);
                true
            }
            // `x = A && B` / `x = A || B` with an inout/output-formal call in the SHORT-CIRCUIT
            // operand `B`. (A call in `A` alone is unconditionally evaluated and already hoisted
            // by `hoist_stmt_top`, so it never reaches here.)
            ast::ExprKind::Binary {
                op,
                lhs: a,
                rhs: bexpr,
            } if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr)
                && self.expr_has_inout_call(bexpr)
                && self.arm_transformable(a)
                && self.arm_transformable(bexpr) =>
            {
                self.lower_shortcircuit_rhs(b, lhs, *op, a, bexpr);
                true
            }
            _ => false,
        }
    }

    /// §4.5.216: lower `x = A && B` / `x = A || B` (the short-circuit RHS `B` carrying an
    /// output/inout-formal call) as an explicit branch chain that assigns `lhs` on every
    /// path. `A` is evaluated ONCE at `head` (any eval-order-safe unconditional call in it is
    /// hoisted there) and its tri-valued truth is CAPTURED in a fresh 1-bit net so `B`'s
    /// copy-out (in `eval_b`) can never perturb the value combined with it. The whole-
    /// expression result is byte-identical to a bare `A && B` / `A || B` because it is
    /// assembled with the SAME logical op the engine uses (`log_and`/`log_or`, tri-valued),
    /// including the 4-state corners:
    ///
    /// ```text
    ///   &&:  head:   ta = bool(A);  branch (ta !== 0) -> eval_b, sc(=0)   (A definitely-false ⇒ 0)
    ///   ||:  head:   ta = bool(A);  branch  ta        -> sc(=1),  eval_b  (A definitely-true  ⇒ 1)
    ///        eval_b: b_id = B (its copy-out fires here);  x = (ta <op> b_id)
    ///        sc:     x = (&& ? 1'b0 : 1'b1)   (B never evaluated ⇒ its call never fires)
    /// ```
    ///
    /// For `&&` the branch is `ta !== 0` (case-inequality) so an x-valued `A` still evaluates
    /// `B` — matching `log_and(x, B)`, which needs `B`; `sc` is reached only for a DEFINITELY
    /// false `A`, where `A && B == 0` regardless of `B`. For `||` a plain truth-branch on `ta`
    /// sends a definitely-true `A` to `sc` (`== 1`) and {false, x} to `eval_b`, matching
    /// `log_or`. The short-circuit path's literal is exact because a definitely-false `&&`
    /// operand / definitely-true `||` operand fully determines the 4-state result.
    pub(crate) fn lower_shortcircuit_rhs(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        op: ast::BinOp,
        a: &ast::Expr,
        bexpr: &ast::Expr,
    ) {
        let is_and = matches!(op, ast::BinOp::LogAnd);
        // head: A → 1-bit tri-valued bool(A), captured in a fresh net (immune to B's copy-out).
        let a_id = self.lower_loop_cond_operand(b, a);
        let boola = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::LogOr,
            lhs: a_id,
            rhs: a_id,
        });
        let ta_net = self.fresh_ia_tmp(1);
        let cap = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(ta_net),
            rhs: boola,
        });
        b.push_stmt_id(cap);

        let eval_b = b.new_block();
        let sc_bb = b.new_block();
        let merge = b.new_block();

        let ta1 = self.push_expr(ir::Expr::Signal {
            net: ta_net,
            word: None,
        });
        if is_and {
            // A definitely-false (bool(A) === 0) ⇒ short-circuit to 0; else (true OR x) eval B.
            let zero = self.const_u32_expr(0, 1);
            let ne0 = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::CaseNe,
                lhs: ta1,
                rhs: zero,
            });
            b.end_block_with(ir::Terminator::Branch {
                cond: ne0,
                then_bb: eval_b.raw(),
                else_bb: sc_bb.raw(),
            });
        } else {
            // A definitely-true ⇒ short-circuit to 1; else (false OR x) eval B.
            b.end_block_with(ir::Terminator::Branch {
                cond: ta1,
                then_bb: sc_bb.raw(),
                else_bb: eval_b.raw(),
            });
        }

        // eval_b: A did not short-circuit. Evaluate B (its copy-out `Terminator::Call` fires
        // here) and combine with the CAPTURED bool(A) via the engine's own logical op, so the
        // 4-state result equals a bare `A <op> B`.
        b.start_block(eval_b);
        let b_id = self.lower_loop_cond_operand(b, bexpr);
        let ta2 = self.push_expr(ir::Expr::Signal {
            net: ta_net,
            word: None,
        });
        let combined = self.push_expr(ir::Expr::Binary {
            op: if is_and {
                ir::BinOp::LogAnd
            } else {
                ir::BinOp::LogOr
            },
            lhs: ta2,
            rhs: b_id,
        });
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lv,
            rhs: combined,
        });
        b.push_stmt_id(sid);
        b.goto(merge);

        // sc_bb: A short-circuited. Result fully determined (0 for `&&`, 1 for `||`); B — and
        // its copy-out — is never evaluated.
        b.start_block(sc_bb);
        let lit = self.const_u32_expr(u32::from(!is_and), 1);
        let lv2 = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv2, true);
        let sid2 = self.push_stmt(ir::Stmt::BlockingAssign { lhs: lv2, rhs: lit });
        b.push_stmt_id(sid2);
        b.goto(merge);

        b.start_block(merge);
    }

    /// §4.5.216: lower `x = c ? T : E` where a CONDITIONALLY-evaluated arm carries an
    /// output/inout-formal call, as explicit control flow that assigns `lhs` on every path.
    /// The condition is evaluated ONCE at `head` and its tri-valued truth CAPTURED in a fresh
    /// 1-bit net (immune to the arms' copy-outs). The three ways `c` can resolve mirror the
    /// engine's own ternary (`eval_core` `Expr::Ternary`): definite-true ⇒ take `T` only,
    /// definite-false ⇒ take `E` only, and x ⇒ IEEE §11.4.11 bit-merge (evaluate BOTH arms —
    /// both copy-outs fire, exactly as a bare `c ? T : E` evaluates both when `c` is x — and
    /// combine with a plain `Ternary` so the engine's `merge_x` runs):
    ///
    /// ```text
    ///   head:      cc = bool(c);  branch cc -> t_take, not_true
    ///   t_take:    x = T   (T's copy-out fires)
    ///   not_true:  branch (cc === 0) -> e_take, x_merge
    ///   e_take:    x = E   (E's copy-out fires)
    ///   x_merge:   x = (cc ? T : E)   (both arms evaluated → both copy-outs fire → merge_x)
    /// ```
    ///
    /// For the definite arms, `x = T` / `x = E` coerce each arm directly to `lhs`'s width (as
    /// a normal blocking assign, via `assign_arm`) — byte-identical to a bare ternary ONLY
    /// when `lhs` is at least as wide as both arms AND the arms share effective signedness.
    /// §4.5.217 makes `shortcircuit_rhs_special` GATE on exactly that (`ternary_arms_coercion_safe`):
    /// a sign-mismatch (§11.8.1) or a narrow-lhs width divergence (§11.6.1) declines the split →
    /// generic lowering → loud, so a taken definite arm can never differ from the unified value.
    /// A BURIED / deeper-nested / eval-order-unsafe call is likewise filtered out by
    /// `shortcircuit_rhs_special`'s `arm_transformable` gate before we get here.
    pub(crate) fn lower_ternary_rhs(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        cond: &ast::Expr,
        then_e: &ast::Expr,
        else_e: &ast::Expr,
    ) {
        // head: evaluate & CAPTURE bool(cond) — only its truth selects the arm(s).
        let c_id = self.lower_loop_cond_operand(b, cond);
        let boolc = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::LogOr,
            lhs: c_id,
            rhs: c_id,
        });
        let cc_net = self.fresh_ia_tmp(1);
        let cap = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(cc_net),
            rhs: boolc,
        });
        b.push_stmt_id(cap);

        let t_take = b.new_block();
        let not_true = b.new_block();
        let e_take = b.new_block();
        let x_merge = b.new_block();
        let merge = b.new_block();

        // c definite-true ⇒ THEN only.
        let cc1 = self.push_expr(ir::Expr::Signal {
            net: cc_net,
            word: None,
        });
        b.end_block_with(ir::Terminator::Branch {
            cond: cc1,
            then_bb: t_take.raw(),
            else_bb: not_true.raw(),
        });

        b.start_block(t_take);
        self.assign_arm(b, lhs, then_e);
        b.goto(merge);

        // not_true: c is false OR x. Distinguish definite-false (ELSE only) from x (bit-merge).
        b.start_block(not_true);
        let cc2 = self.push_expr(ir::Expr::Signal {
            net: cc_net,
            word: None,
        });
        let zero = self.const_u32_expr(0, 1);
        let is_zero = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::CaseEq,
            lhs: cc2,
            rhs: zero,
        });
        b.end_block_with(ir::Terminator::Branch {
            cond: is_zero,
            then_bb: e_take.raw(),
            else_bb: x_merge.raw(),
        });

        b.start_block(e_take);
        self.assign_arm(b, lhs, else_e);
        b.goto(merge);

        // x_merge: c is x ⇒ evaluate BOTH arms (both copy-outs fire) and let the engine's
        // ternary `merge_x` combine them (`cc` is x here, so `Ternary` merges bit-by-bit).
        b.start_block(x_merge);
        let t_val = self.lower_loop_cond_operand(b, then_e);
        let e_val = self.lower_loop_cond_operand(b, else_e);
        let cc3 = self.push_expr(ir::Expr::Signal {
            net: cc_net,
            word: None,
        });
        let tern = self.push_expr(ir::Expr::Ternary {
            cond: cc3,
            then_e: t_val,
            else_e: e_val,
        });
        let lvx = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lvx, true);
        let sidx = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lvx,
            rhs: tern,
        });
        b.push_stmt_id(sidx);
        b.goto(merge);

        b.start_block(merge);
    }

    /// §4.5.216: lower one ternary arm `e` (hoisting an eval-order-safe inout/output-formal
    /// call in it to a copy-out `Terminator::Call` at the CURRENT block, so the copy-out
    /// fires on this path only) and assign it to `lhs` as a normal blocking assign — reusing
    /// `resize_fill_rhs` so a context-fill literal (`'0`/`'1`) arm grows to the lvalue width
    /// exactly like the generic Blocking path.
    pub(crate) fn assign_arm(&mut self, b: &mut ProcessBuilder, lhs: &ast::Lvalue, e: &ast::Expr) {
        let rhs_id = self.lower_loop_cond_operand(b, e);
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        let rhs_id = self.resize_fill_rhs(e, rhs_id, &lv);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: lv,
            rhs: rhs_id,
        });
        b.push_stmt_id(sid);
    }
}
