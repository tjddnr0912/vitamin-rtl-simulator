//! coverage sampling synthesis — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// A decimal integer literal expr from an `i64` (N5 coverage bin bounds). A negative
/// value is `-` (unary minus) over the positive magnitude (a bare `-N` literal raw is
/// not a single token the literal parser accepts).
pub(crate) fn cov_int_lit(v: i64, sp: ast::Span) -> ast::Expr {
    if v < 0 {
        return sva_unary(ast::UnOp::Minus, cov_int_lit_pos(-(v as i128), sp), sp);
    }
    cov_int_lit_pos(v as i128, sp)
}

pub(crate) fn cov_int_lit_pos(v: i128, sp: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::IntLit {
            kind: ast::IntLitKind::Decimal,
            raw: v.to_string(),
        },
        span: sp,
    }
}

/// The auto-bin sample statement `bitmap = bitmap | (64'd1 << (expr & 63))` as AST
/// (used only on the guarded `coverpoint x iff(g)` auto path — same semantics as the
/// IR-direct legacy path, but lowerable inside an `if (g)`).
pub(crate) fn cov_auto_sample_stmt(
    bitmap: &str,
    expr: &ast::Expr,
    mask: u32,
    sp: ast::Span,
) -> ast::Stmt {
    let bm = || sva_ident_expr(bitmap, sp);
    let masked = sva_binary(
        ast::BinOp::BitAnd,
        expr.clone(),
        cov_int_lit(mask as i64, sp),
        sp,
    );
    let one64 = ast::Expr {
        kind: ast::ExprKind::IntLit {
            kind: ast::IntLitKind::Sized,
            raw: "64'd1".to_string(),
        },
        span: sp,
    };
    let shifted = sva_binary(ast::BinOp::Shl, one64, masked, sp);
    let newv = sva_binary(ast::BinOp::BitOr, bm(), shifted, sp);
    ast::Stmt::Blocking {
        lhs: ast::Lvalue::Ident(ast::HierPath {
            segments: vec![ast::Ident {
                name: bitmap.to_string(),
                span: sp,
            }],
            span: sp,
        }),
        delay: None,
        event: None,
        rhs: newv,
        span: sp,
    }
}

/// `bitmap[bit] = 1'b1;` — set one coverpoint bin's covered-bit (N5).
pub(crate) fn cov_set_bit_stmt(bitmap: &str, bit: u32, sp: ast::Span) -> ast::Stmt {
    ast::Stmt::Blocking {
        lhs: ast::Lvalue::BitSelect {
            base: Box::new(ast::Lvalue::Ident(ast::HierPath {
                segments: vec![ast::Ident {
                    name: bitmap.to_string(),
                    span: sp,
                }],
                span: sp,
            })),
            index: Box::new(cov_int_lit(bit as i64, sp)),
            span: sp,
        },
        delay: None,
        event: None,
        rhs: sva_one(sp),
        span: sp,
    }
}

/// `option.at_least > 1` then-block (slice D): `begin if (ctr < N) ctr = ctr + 1;
/// if (ctr >= N) bitmap[bit] = 1'b1; end`. Blocking assigns so the second `if` sees
/// the incremented counter — the covered-bit is set exactly when the N-th hit lands.
pub(crate) fn cov_counter_then(
    bitmap: &str,
    bit: u32,
    counter: &str,
    at_least: u32,
    sp: ast::Span,
) -> ast::Stmt {
    let ctr = sva_ident_expr(counter, sp);
    let ctr_path = ast::Lvalue::Ident(ast::HierPath {
        segments: vec![ast::Ident {
            name: counter.to_string(),
            span: sp,
        }],
        span: sp,
    });
    let n = || cov_int_lit(at_least as i64, sp);
    let inc = ast::Stmt::If {
        cond: sva_binary(ast::BinOp::Lt, ctr.clone(), n(), sp),
        then_s: Box::new(ast::Stmt::Blocking {
            lhs: ctr_path,
            delay: None,
            event: None,
            rhs: sva_binary(ast::BinOp::Add, ctr.clone(), cov_int_lit(1, sp), sp),
            span: sp,
        }),
        else_s: None,
        span: sp,
    };
    let setb = ast::Stmt::If {
        cond: sva_binary(ast::BinOp::Ge, ctr, n(), sp),
        then_s: Box::new(cov_set_bit_stmt(bitmap, bit, sp)),
        else_s: None,
        span: sp,
    };
    ast::Stmt::Block {
        label: None,
        decls: Vec::new(),
        stmts: vec![inc, setb],
        span: sp,
    }
}

/// Membership predicate for one bin: `OR` over its ranges of `(v == k)` (single
/// value, `lo==hi`) or `(v >= lo) && (v <= hi)`. Empty ranges ⇒ `1'b0` (never match).
pub(crate) fn cov_bin_match(expr: &ast::Expr, ranges: &[(i64, i64)], sp: ast::Span) -> ast::Expr {
    let mut acc: Option<ast::Expr> = None;
    for &(lo, hi) in ranges {
        let term = if lo == hi {
            sva_binary(ast::BinOp::Eq, expr.clone(), cov_int_lit(lo, sp), sp)
        } else {
            sva_binary(
                ast::BinOp::LogAnd,
                sva_binary(ast::BinOp::Ge, expr.clone(), cov_int_lit(lo, sp), sp),
                sva_binary(ast::BinOp::Le, expr.clone(), cov_int_lit(hi, sp), sp),
                sp,
            )
        };
        acc = Some(match acc {
            None => term,
            Some(a) => sva_binary(ast::BinOp::LogOr, a, term, sp),
        });
    }
    acc.unwrap_or_else(|| sva_zero(sp))
}

/// `OR` of the membership predicates of every bin of `kind` (the `any_illegal`
/// runtime `$error` gate). `None` when no bin of that kind exists.
pub(crate) fn cov_match_any(
    bins: &[ResolvedBin],
    expr: &ast::Expr,
    kind: ast::BinKind,
    sp: ast::Span,
) -> Option<ast::Expr> {
    let mut acc: Option<ast::Expr> = None;
    for rb in bins.iter().filter(|b| b.kind == kind) {
        let m = cov_bin_match(expr, &rb.ranges, sp);
        acc = Some(match acc {
            None => m,
            Some(a) => sva_binary(ast::BinOp::LogOr, a, m, sp),
        });
    }
    acc
}

impl Elaborator<'_> {
    /// Const-fold a `dist` constraint into `(field_id, [(lo,hi,total_weight)])`. The
    /// value must be a rand field; each item's bounds + weight are constants. A `:=`
    /// (per-value) range carries total weight `w·count`; a `:/` (spread) range carries
    /// total `w`. Non-positive / empty items are dropped.
    pub(crate) fn fold_dist(
        &mut self,
        value: &ast::Expr,
        items: &[ast::DistItem],
        map: &std::collections::HashMap<String, u32>,
    ) -> Option<DistField> {
        let fname = rand_field_ident(value)?;
        let Some(&idx) = map.get(&fname) else {
            self.error(
                MsgCode::ElabUnsupported,
                "a `dist` value must be a rand field of this class (B2)",
            );
            return None;
        };
        let mut entries: Vec<(i64, i64, i64)> = Vec::new();
        for it in items {
            let Some(lo) = self.const_eval_in_scope(&it.lo) else {
                self.error(MsgCode::ElabUnsupported, "a `dist` bound must be constant");
                return None;
            };
            let hi = match &it.hi {
                Some(h) => self.const_eval_in_scope(h)?,
                None => lo,
            };
            let Some(w) = self.const_eval_in_scope(&it.weight) else {
                self.error(MsgCode::ElabUnsupported, "a `dist` weight must be constant");
                return None;
            };
            if hi < lo || w <= 0 {
                continue;
            }
            let count = hi - lo + 1;
            let total_w = if it.per_range {
                w
            } else {
                w.saturating_mul(count)
            };
            entries.push((lo, hi, total_w));
        }
        if entries.is_empty() {
            return None;
        }
        Some((idx, entries))
    }

    /// Materialize each collected `cover property` (SVA-REST) into a clocked match
    /// COUNTER plus an end-of-sim `final` `$display` of the hit count. Pure IR-0:
    /// `always @(clk) if (match && !dis) cnt <= cnt + 1;` + `final $display("…%0d…",
    /// cnt);`. The match signal reuses the same sequence machinery as an assertion
    /// antecedent (`synth_seq_match`); a single clocking event is required.
    pub(crate) fn materialize_cover(&mut self) {
        let pending = std::mem::take(&mut self.pending_cover);
        for cov in pending {
            let sp = cov.span;
            let single_clock = matches!(&cov.clock, ast::Sensitivity::List(evs) if evs.len() == 1);
            if !single_clock {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a cover property must have a single clocking event \
                     (multi-clock cover is unsupported in this subset)",
                );
                continue;
            }
            // Reject a re-clocked / multi-clock sequence (handoff machinery is for
            // assertions only) — keep cover to a single-clock match.
            if seq_has_clocked(&cov.seq) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a re-clocked (`@(c2)`) sequence inside `cover property` is \
                     unsupported in this subset",
                );
                continue;
            }
            let mut regs = SvaRegs::default();
            let mut pipeline_nbas: Vec<ast::Stmt> = Vec::new();
            let matched = self.synth_seq_match(&cov.seq, &mut regs, &mut pipeline_nbas, sp);
            let dis = cov
                .disable_iff
                .as_ref()
                // §16.13.5: an X/Z disable condition is NOT definitely-true → it does
                // NOT disable (X-strict), so a real violation is not silently masked.
                .map(|e| sva_match(self.rewrite_sampled(e, &mut regs), sp));
            // hit condition (1-bit), gated by `!dis`.
            let mut hit = sva_unary(ast::UnOp::RedOr, matched, sp);
            if let Some(d) = &dis {
                hit = sva_binary(
                    ast::BinOp::LogAnd,
                    sva_unary(ast::UnOp::LogNot, d.clone(), sp),
                    hit,
                    sp,
                );
            }
            // 32-bit 0-init hit counter (`fresh_cover_counter`).
            let cnt = self.fresh_cover_counter(32);
            let cnt_e = sva_ident_expr(&cnt, sp);
            // `if (hit) cnt <= cnt + 1;`
            let incr = ast::Stmt::If {
                cond: hit,
                then_s: Box::new(sva_nba_1bit(
                    &cnt,
                    sva_binary(ast::BinOp::Add, cnt_e.clone(), sva_one(sp), sp),
                    sp,
                )),
                else_s: None,
                span: sp,
            };
            let mut stmts = vec![incr];
            stmts.extend(regs.nbas);
            // `disable iff`: clear the antecedent pipeline obligations on the dis clock.
            if let Some(d) = &dis {
                for s in pipeline_nbas {
                    stmts.push(gate_nba_with_disable(s, d, sp));
                }
            } else {
                stmts.extend(pipeline_nbas);
            }
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
                sensitivity: Some(cov.clock.clone()),
                body: Box::new(body),
                span: sp,
            };
            let proc = self.lower_proc_block(&pb);
            self.push_process(proc);
            // End-of-sim coverage report: `final $display("Cover ...: %0d hits", cnt);`.
            let report = ast::Stmt::SysTaskCall {
                name: ast::Ident {
                    name: "$display".to_string(),
                    span: sp,
                },
                args: vec![
                    ast::Expr {
                        kind: ast::ExprKind::StrLit {
                            raw: "\"Cover property hits: %0d\"".to_string(),
                        },
                        span: sp,
                    },
                    cnt_e,
                ],
                span: sp,
            };
            let fpb = ast::ProceduralBlock {
                kind: ast::ProcKind::Final,
                sensitivity: None,
                body: Box::new(report),
                span: sp,
            };
            let fproc = self.lower_proc_block(&fpb);
            self.push_process(fproc);
        }
    }
}
