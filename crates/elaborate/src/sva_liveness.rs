//! SVA liveness — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// True iff a property-expression tree contains a STRONG liveness operator
    /// (`s_eventually` or `s_until`) anywhere — those need an end-of-sim `final`
    /// obligation check (`synth_liveness`) rather than the per-clock safety reducer.
    pub(crate) fn prop_expr_has_liveness(pe: &ast::PropExpr) -> bool {
        match pe {
            ast::PropExpr::Seq(_) => false,
            ast::PropExpr::Impl { cons, .. } => Self::prop_expr_has_liveness(cons),
            ast::PropExpr::And(l, r) | ast::PropExpr::Or(l, r) => {
                Self::prop_expr_has_liveness(l) || Self::prop_expr_has_liveness(r)
            }
            ast::PropExpr::Not(p) | ast::PropExpr::Always(p) => Self::prop_expr_has_liveness(p),
            ast::PropExpr::Until { lhs, rhs, strong } => {
                *strong || Self::prop_expr_has_liveness(lhs) || Self::prop_expr_has_liveness(rhs)
            }
            ast::PropExpr::Eventually { .. } => true,
        }
    }

    /// Synthesize a LIVENESS property (`s_eventually` / `s_until`, SVA-REST). A
    /// liveness obligation has no per-clock safety verdict — instead a 0-init
    /// `pend` reg tracks "an attempt is still waiting for its target", maintained in
    /// a clocked `always @(clk)`, and an end-of-sim `final` block reports
    /// `if (pend) $error`. A single flag collapses all overlapping attempts (they
    /// discharge together at the first target match — exact for the canonical idioms).
    ///
    /// Recognized shapes (else loud-reject — never silently miss a liveness check):
    /// `s_eventually p` (arm EVERY clock), `req |-> s_eventually p` (arm on `req` this
    /// clock), `req |=> s_eventually p` (arm one clock later via a `pend_req` reg), and
    /// `lhs s_until rhs` (per-clock safety `!l && !r` plus liveness on `rhs`).
    ///
    /// A top-level `always` wrapper (recurrent liveness, `always s_eventually p`) is
    /// peeled (≡ `s_eventually p` under re-attempt). `req` is restricted to a boolean
    /// (a multi-term antecedent is loud). The unsupported feature combos (a consequent
    /// clock, `disable iff`, a pass action) are loud-rejected like `synth_prop_expr`.
    pub(crate) fn synth_liveness(&mut self, sva: PendingSva, sp: ast::Span) {
        let Some(pe) = sva.prop_expr.clone() else {
            return;
        };
        if sva.cons_clock.is_some() || sva.disable_iff.is_some() || sva.pass.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "a liveness property (`s_eventually` / `s_until`) combined with a \
                 consequent clock, `disable iff`, or a pass action is unsupported in \
                 this subset",
            );
            return;
        }
        let pe = peel_top_always(pe);
        let mut regs = SvaRegs::default();
        let mut nbas: Vec<ast::Stmt> = Vec::new();
        let mut check_stmts: Vec<ast::Stmt> = Vec::new();
        // The liveness `pend` reg (0-init: a never-armed attempt must not fire).
        let pend = self.fresh_sva_reg0(1, "live");
        let pend_e = sva_ident_expr(&pend, sp);
        // `arm & clear` per shape. `clear` is the 1-bit "target HELD this clock"; the
        // pend recurrence is `pend <= (pend | arm) & !clear`.
        let (arm, clear) = match &pe {
            // `s_eventually p` — arm every clock; target = held(p).
            ast::PropExpr::Eventually { prop, .. } => {
                let Some(held) = self.liveness_held(prop, &mut regs, sp) else {
                    return;
                };
                (sva_one(sp), held)
            }
            // `req |-> s_eventually p` / `req |=> s_eventually p`.
            ast::PropExpr::Impl { ante, kind, cons } => {
                let ast::PropExpr::Eventually { prop, .. } = cons.as_ref() else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a liveness implication consequent must be `s_eventually p` in \
                         this subset",
                    );
                    return;
                };
                let Some(req) = self.liveness_bool_ante(ante, &mut regs, sp) else {
                    return;
                };
                let Some(held) = self.liveness_held(prop, &mut regs, sp) else {
                    return;
                };
                let arm = match kind {
                    ast::ImplicationKind::Overlap => req,
                    ast::ImplicationKind::NonOverlap => {
                        // `pend_req <= req;` — arm the eventually one clock later.
                        let pr = self.fresh_sva_reg0(1, "livereq");
                        nbas.push(sva_nba_1bit(&pr, req, sp));
                        sva_ident_expr(&pr, sp)
                    }
                };
                (arm, held)
            }
            // `lhs s_until rhs` — per-clock safety `!l && !r`; liveness on rhs.
            ast::PropExpr::Until { lhs, rhs, .. } => {
                let Some((vl, sl)) =
                    self.prop_expr_violation(lhs, None, &mut regs, &mut nbas, 0, sp)
                else {
                    return;
                };
                let Some((vr, sr)) =
                    self.prop_expr_violation(rhs, None, &mut regs, &mut nbas, 0, sp)
                else {
                    return;
                };
                if sl != 0 || sr != 0 {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an `s_until` operand with a multi-clock (`|=>`) skew is \
                         unsupported in this subset",
                    );
                    return;
                }
                // Safety: `if (!held(l) && !held(r)) $error` = `if (viol(l) && viol(r))`.
                let safety = sva_binary(
                    ast::BinOp::LogAnd,
                    sva_unary(ast::UnOp::RedOr, vl, sp),
                    sva_unary(ast::UnOp::RedOr, vr.clone(), sp),
                    sp,
                );
                check_stmts.push(ast::Stmt::If {
                    cond: safety,
                    then_s: Box::new(sva_error_stmt(sp)),
                    else_s: None,
                    span: sp,
                });
                // Liveness target = held(rhs) = !viol(rhs); arm every clock.
                let held = sva_unary(ast::UnOp::LogNot, sva_unary(ast::UnOp::RedOr, vr, sp), sp);
                (sva_one(sp), held)
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "this liveness property shape is unsupported in this subset \
                     (supported: `s_eventually p`, `req |->/|=> s_eventually p`, \
                     `lhs s_until rhs`)",
                );
                return;
            }
        };
        // pend recurrence: `pend <= (pend | arm) & !clear` (1-bit).
        let not_clear = sva_unary(
            ast::UnOp::LogNot,
            sva_unary(ast::UnOp::RedOr, clear, sp),
            sp,
        );
        let armed = sva_binary(ast::BinOp::BitOr, pend_e.clone(), arm, sp);
        let next = sva_binary(ast::BinOp::BitAnd, armed, not_clear, sp);
        nbas.push(sva_nba_1bit(&pend, next, sp));
        // Clocked maintenance process: safety check (if any) FIRST, then NBAs.
        let mut stmts = check_stmts;
        stmts.extend(regs.nbas);
        stmts.extend(nbas);
        let body = if stmts.len() == 1 {
            stmts.pop().unwrap()
        } else {
            ast::Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts,
                span: sp,
            }
        };
        let pb = ast::ProceduralBlock {
            kind: ast::ProcKind::Always,
            sensitivity: Some(sva.clock.clone()),
            body: Box::new(body),
            span: sp,
        };
        let proc = self.lower_synth_proc(&pb, "sva");
        self.push_process(proc);
        // End-of-sim obligation: `final if (pend) $error`. A separate `final` process
        // (registered in `final_procs`) reads the module-level `pend` reg.
        let final_body = ast::Stmt::If {
            cond: pend_e,
            then_s: Box::new(ast::Stmt::SysTaskCall {
                name: ast::Ident {
                    name: "$error".to_string(),
                    span: sp,
                },
                args: vec![ast::Expr {
                    kind: ast::ExprKind::StrLit {
                        raw: "\"Liveness property not satisfied (s_eventually/s_until)\""
                            .to_string(),
                    },
                    span: sp,
                }],
                span: sp,
            }),
            else_s: None,
            span: sp,
        };
        let fpb = ast::ProceduralBlock {
            kind: ast::ProcKind::Final,
            sensitivity: None,
            body: Box::new(final_body),
            span: sp,
        };
        let fproc = self.lower_synth_proc(&fpb, "sva");
        self.push_process(fproc);
    }

    /// The 1-bit "HELD this clock" expression of a skew-0 liveness target property
    /// (`held = !viol`). Loud-rejects a multi-clock-skew target (a `|=>` target would
    /// need the obligation deferred a clock — out of this subset).
    pub(crate) fn liveness_held(
        &mut self,
        prop: &ast::PropExpr,
        regs: &mut SvaRegs,
        sp: ast::Span,
    ) -> Option<ast::Expr> {
        let mut nbas = Vec::new();
        let (viol, skew) = self.prop_expr_violation(prop, None, regs, &mut nbas, 0, sp)?;
        if skew != 0 || !nbas.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                "a multi-clock-skew liveness target (e.g. `s_eventually (a |=> b)`) is \
                 unsupported in this subset",
            );
            return None;
        }
        Some(sva_unary(
            ast::UnOp::LogNot,
            sva_unary(ast::UnOp::RedOr, viol, sp),
            sp,
        ))
    }

    /// The 1-bit boolean match of a liveness implication antecedent. Restricted to a
    /// boolean sequence (a multi-term antecedent is loud); `$past`/`$rose`/etc. are
    /// rewritten onto the shared prev-regs.
    pub(crate) fn liveness_bool_ante(
        &mut self,
        ante: &ast::Sequence,
        regs: &mut SvaRegs,
        sp: ast::Span,
    ) -> Option<ast::Expr> {
        let ast::Sequence::Boolean(e) = ante else {
            self.error(
                MsgCode::ElabUnsupported,
                "a liveness implication antecedent must be a boolean (a multi-term \
                 sequence antecedent is unsupported in this subset)",
            );
            return None;
        };
        let e = self.rewrite_sampled(e, regs);
        // §16.13.5: an X/Z antecedent is a non-match → it does not arm the
        // eventuality (X-strict, so the `pend` reg never holds X).
        Some(sva_match(e, sp))
    }
}
