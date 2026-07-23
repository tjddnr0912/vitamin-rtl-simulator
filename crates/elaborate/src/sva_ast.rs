//! SVA AST builders — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Build a `name <= rhs;` NBA to an already-named synthesized reg.
pub(crate) fn sva_nb(name: &str, rhs: ast::Expr, sp: ast::Span) -> ast::Stmt {
    ast::Stmt::NonBlocking {
        lhs: ast::Lvalue::Ident(ast::HierPath {
            segments: vec![ast::Ident {
                name: name.to_string(),
                span: sp,
            }],
            span: sp,
        }),
        delay: None,
        event: None,
        rhs,
        span: sp,
    }
}

/// The 1-bit constant `1'b1` — the activation for a leading goto/nonconsec term
/// (a counting thread starts every clock).
pub(crate) fn sva_one(sp: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::IntLit {
            kind: ast::IntLitKind::Sized,
            raw: "1'b1".to_string(),
        },
        span: sp,
    }
}

/// IEEE 1800 §16.13.5 boolean match: a boolean expression in a sequence/property
/// matches ONLY if it evaluates to true (1); X/Z (and 0) are a NON-match. The
/// plain reduction `|e` leaves X as X, which downstream `if(X)`/`!X` then treat
/// leniently (no-fire) — a false-negative. Case-comparing the reduced truthiness
/// against `1'b1` yields a hard 0 for X/Z (and 0) and 1 only for a definitely-
/// nonzero value, so X/Z becomes a real non-match. Used at every CONSEQUENT
/// boolean site (the antecedent vacates naturally: a fire requires `LogAnd(ante,
/// !match)` and `LogAnd(X, _)` never takes the fire branch).
pub(crate) fn sva_match(e: ast::Expr, sp: ast::Span) -> ast::Expr {
    sva_binary(
        ast::BinOp::CaseEq,
        sva_unary(ast::UnOp::RedOr, e, sp),
        sva_one(sp),
        sp,
    )
}

/// The 1-bit constant `1'b0` — a never-matching antecedent (e.g. a `within`
/// whose seq1 is longer than every seq2 window).
pub(crate) fn sva_zero(sp: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::IntLit {
            kind: ast::IntLitKind::Sized,
            raw: "1'b0".to_string(),
        },
        span: sp,
    }
}

/// A 1-bit non-blocking assignment `<name> <= <rhs>;` (an SVA pend/liveness reg
/// update appended to a synthesized clocked checker body).
pub(crate) fn sva_nba_1bit(name: &str, rhs: ast::Expr, sp: ast::Span) -> ast::Stmt {
    ast::Stmt::NonBlocking {
        lhs: ast::Lvalue::Ident(ast::HierPath {
            segments: vec![ast::Ident {
                name: name.to_string(),
                span: sp,
            }],
            span: sp,
        }),
        delay: None,
        event: None,
        rhs,
        span: sp,
    }
}

/// The default SVA violation reporter `$error("Assertion property violation")`
/// (the no-action-block fail handler — routes to the diagnostic stream + exit
/// class 1, mirroring an immediate-assert severity).
pub(crate) fn sva_error_stmt(sp: ast::Span) -> ast::Stmt {
    ast::Stmt::SysTaskCall {
        name: ast::Ident {
            name: "$error".to_string(),
            span: sp,
        },
        args: vec![ast::Expr {
            kind: ast::ExprKind::StrLit {
                raw: "\"Assertion property violation\"".to_string(),
            },
            span: sp,
        }],
        span: sp,
    }
}

/// A 1-bit NBA `<name> <= 1'b1` / `1'b0` — an SVA handoff arm / discharge.
pub(crate) fn sva_nb_set(name: &str, one: bool, sp: ast::Span) -> ast::Stmt {
    ast::Stmt::NonBlocking {
        lhs: ast::Lvalue::Ident(ast::HierPath {
            segments: vec![ast::Ident {
                name: name.to_string(),
                span: sp,
            }],
            span: sp,
        }),
        delay: None,
        event: None,
        rhs: if one { sva_one(sp) } else { sva_zero(sp) },
        span: sp,
    }
}

/// Wrap an obligation NBA's RHS in `dis ? 1'b0 : rhs` so a `disable iff (dis)`
/// reset clears in-flight pipeline/pending state on the clock it is asserted
/// (slice S12). Only NonBlocking stmts occur in the obligation list; any other
/// stmt (none expected) passes through unchanged.
pub(crate) fn gate_nba_with_disable(stmt: ast::Stmt, dis: &ast::Expr, sp: ast::Span) -> ast::Stmt {
    match stmt {
        ast::Stmt::NonBlocking {
            lhs,
            delay,
            event,
            rhs,
            span,
        } => ast::Stmt::NonBlocking {
            lhs,
            delay,
            event,
            rhs: ast::Expr {
                kind: ast::ExprKind::Ternary {
                    cond: Box::new(dis.clone()),
                    then_e: Box::new(sva_zero(sp)),
                    else_e: Box::new(rhs),
                },
                span: sp,
            },
            span,
        },
        other => other,
    }
}

/// AND two optional (1-bit) `throughout` guards. The common (top-level
/// throughout) case has at most one guard, so no BinOp is built.
pub(crate) fn and_opt(
    a: Option<ast::Expr>,
    b: Option<ast::Expr>,
    sp: ast::Span,
) -> Option<ast::Expr> {
    match (a, b) {
        (None, x) | (x, None) => x,
        (Some(x), Some(y)) => Some(sva_binary(ast::BinOp::BitAnd, x, y, sp)),
    }
}

/// True iff `e` reads a bare single-segment identifier named `name` anywhere (slice
/// N2c — detect a local-variable READ). Conservative: only a single-segment `Ident`
/// counts (a hierarchical `u.name` is a different signal). Recurses structurally.
/// The body statement of a `case` item (`Match`/`Default`) — for AST timing walks.
pub(crate) fn case_item_body(it: &ast::CaseItem) -> &ast::Stmt {
    match it {
        ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => body,
    }
}

/// Build the positional formal→actual substitution map for a parameterized SVA
/// instance (slice A1). The caller has already arity-checked.
pub(crate) fn sva_formal_map(
    formals: &[ast::Ident],
    actuals: &[ast::Expr],
) -> BTreeMap<String, ast::Expr> {
    formals
        .iter()
        .map(|f| f.name.clone())
        .zip(actuals.iter().cloned())
        .collect()
}

/// Is `name` (incl. the leading `$`) an SVA sampled-value function we desugar?
pub(crate) fn is_sva_sampled_fn(name: &str) -> bool {
    matches!(
        name,
        "$past" | "$rose" | "$fell" | "$stable" | "$changed" | "$sampled"
    )
}

pub(crate) fn sva_ident_expr(name: &str, sp: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::Ident(ast::HierPath {
            segments: vec![ast::Ident {
                name: name.to_string(),
                span: sp,
            }],
            span: sp,
        }),
        span: sp,
    }
}

pub(crate) fn sva_binary(
    op: ast::BinOp,
    lhs: ast::Expr,
    rhs: ast::Expr,
    sp: ast::Span,
) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span: sp,
    }
}

pub(crate) fn sva_unary(op: ast::UnOp, operand: ast::Expr, sp: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
        span: sp,
    }
}

/// The `(edge, signal-name)` of a single bare-identifier clocking event
/// (`@(posedge clk)`), or `None` for a multi-event / non-ident / `@(*)` clock. Used
/// to compare two clocks span-insensitively (the `Sensitivity` derive compares spans,
/// so two textually-identical `@(posedge clk)` at different locations are `!=`).
pub(crate) fn sva_clock_signal(s: &ast::Sensitivity) -> Option<(ast::Edge, String)> {
    match s {
        ast::Sensitivity::List(evs) if evs.len() == 1 => match &evs[0].expr.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                Some((evs[0].edge, p.segments[0].name.clone()))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Wrap statements as a single synthesized clocked-checker body — the lone statement
/// directly, or a `Block` of several (slice A3 two-process synthesis).
pub(crate) fn sva_block_or_single(mut stmts: Vec<ast::Stmt>, sp: ast::Span) -> ast::Stmt {
    if stmts.len() == 1 {
        stmts.pop().unwrap()
    } else {
        ast::Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts,
            span: sp,
        }
    }
}

/// `e[0]` — the LSB, for `$rose`/`$fell` (IEEE 1800 §16.9.3 sample the LSB).
pub(crate) fn sva_bit0(e: ast::Expr, sp: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::BitSelect {
            base: Box::new(e),
            index: Box::new(ast::Expr {
                kind: ast::ExprKind::IntLit {
                    kind: ast::IntLitKind::Decimal,
                    raw: "0".to_string(),
                },
                span: sp,
            }),
        },
        span: sp,
    }
}
