//! The inline function lane's straight-line body fold — split out of
//! `inline_fn.rs` (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// Fold a straight-line function body. Returns false (caller emits the error)
    /// on the first non-foldable construct. Each `local = expr;` pushes a
    /// substitution binding (SSA-by-substitution); `fname = expr;` records the
    /// return ExprId. Lowering happens with the CURRENT substitution scope active.
    pub(crate) fn fold_straight_line(
        &mut self,
        s: &ast::Stmt,
        fname: &str,
        ret_w: u32,
        ret_signed: bool,
        scope: &InlineScope<'_>,
        ret: &mut Option<u32>,
    ) -> bool {
        match s {
            ast::Stmt::Null(_) => true,
            ast::Stmt::Block { stmts, .. } => {
                // begin-end local decls need NO nets: each local lives only as a
                // substitution binding (combinational). Fold each stmt in order.
                stmts
                    .iter()
                    .all(|st| self.fold_straight_line(st, fname, ret_w, ret_signed, scope, ret))
            }
            ast::Stmt::Blocking {
                lhs, delay, rhs, ..
            } => {
                if delay.is_some() {
                    self.warn("intra-assignment delay in inlined function dropped");
                }
                // LHS must be a bare single-segment Ident (a local var or func name).
                let ast::Lvalue::Ident(p) = lhs else {
                    return false;
                };
                if p.segments.len() != 1 {
                    return false;
                }
                let target = p.segments[0].name.clone();
                // …and it must be a name this body OWNS. Anything else is a write the
                // inline fold cannot perform — see `writable`.
                if !scope.writable.contains(&target) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        // §3.b: the `automatic` advice used to say it gives "the same
                        // diagnostic from the frame path". It no longer does — a framed
                        // function whose body writes a module net is routed to the
                        // statement executor and RUNS. The inline fold still cannot do it
                        // (it has no statement to emit the write from), so the honest
                        // sentence names the spelling that works.
                        &format!(
                            "function `{fname}` assigns `{target}`, which is not one of \
                             its own formals or locals — an inlined function body has no \
                             statement to carry the write. Declare `{fname}` `automatic`, \
                             or give it a `return`: the frame path performs the write"
                        ),
                    );
                    scope.named_a_reason.set(true);
                    return false;
                }
                // §11.6.1: the rhs is lowered IN the LHS width context — the
                // return-type width for `fname = …`, the declared width for a
                // body/block local. See `inline_body_ctx.rs`: this is the width
                // the frame route gets from the engine's net write, so `fld * x`
                // in a `[31:0]` body folds at 32 rather than at 8.
                let (ctx_w, ctx_signed) = if target == fname {
                    (ret_w, ret_signed)
                } else {
                    scope.dims.get(&target).copied().unwrap_or((0, false))
                };
                let rhs_id0 =
                    self.lower_inline_assign_rhs(rhs, ctx_w, !scope.non_bv.contains(&target));
                // §10.7: apply the LHS-declared width/sign the inline SSA path
                // otherwise misses (no net write). ctx_w==0 = unknown/implicit
                // width ⇒ leave untouched (byte-identical). A real-valued rhs —
                // including a call to a real-returning FRAME function, whose
                // `Expr::Call` node carries no real flag so the helper's IR-level
                // `expr_is_real` cannot see it — must NOT be bit-resized/sign-
                // stamped; guard with the AST-aware check `lower_prim_cast` uses.
                let target_bv = !scope.non_bv.contains(&target);
                let mut rhs_id = if ctx_w > 0 && target_bv && self.expr_is_real(rhs_id0) {
                    // §10.7 / §6.12.2: a REAL-valued rhs stored into an integral
                    // target rounds half away from zero, then narrows or extends to
                    // the target's declared width and takes its sign — the frame
                    // path's net store does exactly this. Leaving it verbatim kept
                    // the real value at the rhs's own width: `f = r + x` into
                    // `[7:0]` with r=1.5, x=254 read back `0100`, both oracles
                    // `0000`. `RealToInt` names the rhs once, so a call or `$random`
                    // inside it is evaluated once. Gated on the VALUE-based
                    // `expr_is_real` for the resolver reason
                    // `coerce_real_actual_to_formal` documents; a real TARGET keeps
                    // the verbatim arm below.
                    self.real_to_int_store(rhs_id0, ctx_w, ctx_signed)
                } else if ctx_w == 0 || self.cast_operand_is_real(rhs, rhs_id0) {
                    self.verbatim_actuals.insert(rhs_id0);
                    rhs_id0
                } else {
                    let out = self.resize_inline_assign(rhs_id0, ctx_w, ctx_signed);
                    // A resize over an UNTRUSTED width (a hierarchical placeholder)
                    // is a new node whose width the mirror still
                    // cannot read — record it like a verbatim actual, or the cast
                    // classifier fabricates 32 for it (`16'(y + 0)` printed `xxxx`).
                    if self.trusted_self_width(rhs_id0).is_none() {
                        self.verbatim_actuals.insert(out);
                    }
                    out
                };
                // §6.11.1: a 2-state LOCAL drops x/z on the store. Applied after the
                // resize, so the seal and sign stamp stay underneath; `TwoState`
                // keeps its operand's width and sign, and names it once.
                if target != fname
                    && target_bv
                    && scope.two_state.contains(&target)
                    && self.expr_may_be_unknown(rhs_id)
                {
                    let squashed = self.push_expr(ir::Expr::SysFunc {
                        which: ir::SysFuncId::TwoState,
                        args: vec![rhs_id],
                    });
                    // Width and sign are the operand's, so an operand recorded as
                    // having an untrusted width passes that record on.
                    if self.verbatim_actuals.contains(&rhs_id) {
                        self.verbatim_actuals.insert(squashed);
                    }
                    rhs_id = squashed;
                }
                if target == fname {
                    *ret = Some(rhs_id); // return assignment
                } else {
                    self.subst.push((target, rhs_id)); // local: innermost-wins binding
                }
                true
            }
            // R2: an explicit `return e;` in a straight-line inline body — same as the
            // `fname = e` return assignment (size the value to the return width/sign).
            // Behavior-preserving for existing code: every Return-bodied function is
            // pre-framed by `body_needs_frame`, so no function reaching the inline fold
            // today contains a `Return` — this arm is exercised only by the new R2
            // read-only-dyn carve-out.
            ast::Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    let rhs_id0 =
                        self.lower_inline_assign_rhs(e, ret_w, !scope.non_bv.contains(fname));
                    let rhs_id =
                        if ret_w > 0 && !scope.non_bv.contains(fname) && self.expr_is_real(rhs_id0)
                        {
                            // The same real→integral store rule as the Blocking arm.
                            self.real_to_int_store(rhs_id0, ret_w, ret_signed)
                        } else if ret_w == 0 || self.cast_operand_is_real(e, rhs_id0) {
                            rhs_id0
                        } else {
                            self.resize_inline_assign(rhs_id0, ret_w, ret_signed)
                        };
                    *ret = Some(rhs_id);
                }
                true
            }
            // if/case/loop/nonblocking/task-call/etc. ⇒ not reducible to one expr.
            _ => false,
        }
    }
}
