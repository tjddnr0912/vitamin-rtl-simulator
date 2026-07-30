//! r19 follow-on — the general hoister's AST plumbing, split from `hoist/general.rs` to
//! hold both files under the 1000-line module cap.
//!
//! Three groups, all mechanical: rebuilding an expression node around already-rewritten
//! children ([`rebuild`], driven by [`shape_children`] so it cannot disagree with `shape`),
//! flattening and rebuilding an lvalue's index expressions, and the two name predicates that
//! say which system tasks/functions the hoister must leave alone.

use super::general::{shape, Shape};
use super::*;

impl Elaborator<'_> {
    /// Hoist a blocking / non-blocking assign's rhs and lvalue indices as ONE ordered
    /// sequence, returning the rewritten `(rhs, lvalue)`.
    pub(crate) fn hoist_assign_seq(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        rhs: &ast::Expr,
    ) -> (ast::Expr, ast::Lvalue) {
        let hoisted = self.hoist_inout_general_seq(b, &assign_seq(lhs, rhs));
        let mut it = hoisted.into_iter();
        let rhs2 = it.next().unwrap_or_else(|| rhs.clone());
        let lhs2 = relvalue(lhs, &mut it);
        (rhs2, lhs2)
    }
}

/// The expressions a blocking / non-blocking assign evaluates, in the order iverilog was
/// measured to evaluate them: the rhs FIRST, then the lvalue's index and bound expressions
/// outermost-first. Feeding this one list to the sequence-wide hoist is what lets a hazard
/// spanning the rhs↔index or index↔index boundary be seen at all.
pub(crate) fn assign_seq<'a>(lhs: &'a ast::Lvalue, rhs: &'a ast::Expr) -> Vec<&'a ast::Expr> {
    let mut seq = vec![rhs];
    lvalue_index_exprs(lhs, &mut seq);
    seq
}

/// Every index / bound expression inside `lv`, outermost-first — the order `relvalue`
/// consumes the rewritten ones in, so the two must stay in step.
pub(crate) fn lvalue_index_exprs<'a>(lv: &'a ast::Lvalue, out: &mut Vec<&'a ast::Expr>) {
    match lv {
        ast::Lvalue::BitSelect { base, index, .. } => {
            lvalue_index_exprs(base, out);
            out.push(index);
        }
        ast::Lvalue::PartSelect { base, msb, lsb, .. } => {
            lvalue_index_exprs(base, out);
            out.push(msb);
            out.push(lsb);
        }
        ast::Lvalue::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            lvalue_index_exprs(base, out);
            out.push(offset);
            out.push(width);
        }
        ast::Lvalue::Concat { parts, .. } => {
            for p in parts {
                lvalue_index_exprs(p, out);
            }
        }
        ast::Lvalue::Ident(_) | ast::Lvalue::Error(_) => {}
    }
}

/// Rebuild `lv` around already-hoisted index expressions, consumed in [`lvalue_index_exprs`]
/// order. A missing one (impossible — the two walks are the same shape) keeps the original.
pub(crate) fn relvalue(lv: &ast::Lvalue, it: &mut std::vec::IntoIter<ast::Expr>) -> ast::Lvalue {
    match lv {
        ast::Lvalue::BitSelect { base, index, span } => {
            let base = Box::new(relvalue(base, it));
            ast::Lvalue::BitSelect {
                base,
                index: Box::new(it.next().unwrap_or_else(|| (**index).clone())),
                span: *span,
            }
        }
        ast::Lvalue::PartSelect {
            base,
            msb,
            lsb,
            span,
        } => {
            let base = Box::new(relvalue(base, it));
            ast::Lvalue::PartSelect {
                base,
                msb: Box::new(it.next().unwrap_or_else(|| (**msb).clone())),
                lsb: Box::new(it.next().unwrap_or_else(|| (**lsb).clone())),
                span: *span,
            }
        }
        ast::Lvalue::IndexedPart {
            base,
            offset,
            width,
            dir,
            span,
        } => {
            let base = Box::new(relvalue(base, it));
            ast::Lvalue::IndexedPart {
                base,
                offset: Box::new(it.next().unwrap_or_else(|| (**offset).clone())),
                width: Box::new(it.next().unwrap_or_else(|| (**width).clone())),
                dir: *dir,
                span: *span,
            }
        }
        ast::Lvalue::Concat { parts, span } => ast::Lvalue::Concat {
            parts: parts.iter().map(|p| relvalue(p, it)).collect(),
            span: *span,
        },
        ast::Lvalue::Ident(_) | ast::Lvalue::Error(_) => lv.clone(),
    }
}

impl Elaborator<'_> {
    /// The 4-state truth of an already-lowered expression, as `!!x` — `0`→`0`, `1`→`1`,
    /// `x`/`z`→`x`. Written as a double negation rather than `x || x` because the latter
    /// names the same expr id twice, and the engine evaluates it twice: an operand with a
    /// side effect (a `$random` draw) then happens twice.
    pub(crate) fn bool_of(&mut self, id: u32) -> u32 {
        let n = self.push_expr(ir::Expr::Unary {
            op: ir::UnOp::LogNot,
            operand: id,
        });
        self.push_expr(ir::Expr::Unary {
            op: ir::UnOp::LogNot,
            operand: n,
        })
    }
}

/// Every child of `e` that [`shape`] models, flattened in evaluation order. The two AST
/// rewriters ([`Elaborator::hoist_inout_general`]'s plain rebuild and
/// [`Elaborator::subst_pre_call_reads`]) both feed this list to [`rebuild`], so neither can
/// visit a child the other misses.
pub(crate) fn shape_children(e: &ast::Expr) -> Vec<&ast::Expr> {
    match shape(e) {
        Shape::Uncond(cs) => cs,
        Shape::ShortCircuit { lhs, rhs, .. } => vec![lhs, rhs],
        Shape::Ternary {
            cond,
            then_e,
            else_e,
        } => vec![cond, then_e, else_e],
        // Not hoist sites — the node is rebuilt unchanged, which is safe because
        // `inout_hoistable_general` already refused any call inside one.
        Shape::NoHoist(_) | Shape::Unevaluated(_) => vec![],
    }
}

/// Rebuild `e` around `children`, which must be exactly [`shape_children`]'s list for `e`
/// (same length, same order). An `Opaque` node or a leaf has no children and is cloned.
pub(crate) fn rebuild(e: &ast::Expr, children: Vec<ast::Expr>) -> ast::Expr {
    use ast::ExprKind as K;
    if children.is_empty() {
        return e.clone();
    }
    let mut k = Kids {
        it: children.into_iter(),
        span: e.span,
    };
    let kind = match &e.kind {
        K::Binary { op, .. } => K::Binary {
            op: *op,
            lhs: k.one(),
            rhs: k.one(),
        },
        K::Ternary { .. } => K::Ternary {
            cond: k.one(),
            then_e: k.one(),
            else_e: k.one(),
        },
        K::Unary { op, .. } => K::Unary {
            op: *op,
            operand: k.one(),
        },
        K::Paren { .. } => K::Paren { inner: k.one() },
        K::Concat { .. } => K::Concat { parts: k.rest() },
        K::Replicate { .. } => K::Replicate {
            count: k.one(),
            value: k.rest(),
        },
        K::Call { name, .. } => K::Call {
            name: name.clone(),
            args: k.rest(),
        },
        K::SysCall { name, .. } => K::SysCall {
            name: name.clone(),
            args: k.rest(),
        },
        K::ClassNew { .. } => K::ClassNew { args: k.rest() },
        K::MethodCall { method, .. } => K::MethodCall {
            recv: k.one(),
            method: method.clone(),
            args: k.rest(),
        },
        K::BitSelect { .. } => K::BitSelect {
            base: k.one(),
            index: k.one(),
        },
        K::PartSelect { .. } => K::PartSelect {
            base: k.one(),
            msb: k.one(),
            lsb: k.one(),
        },
        K::IndexedPart { dir, .. } => K::IndexedPart {
            base: k.one(),
            offset: k.one(),
            width: k.one(),
            dir: *dir,
        },
        K::Cast { target, .. } => match target {
            ast::CastTarget::Size(_) => K::Cast {
                target: ast::CastTarget::Size(k.one()),
                expr: k.one(),
            },
            _ => K::Cast {
                target: target.clone(),
                expr: k.one(),
            },
        },
        K::AssignPattern(_) => K::AssignPattern(k.rest()),
        K::NamedArg { formal, value } => K::NamedArg {
            formal: formal.clone(),
            value: value.as_ref().map(|_| k.one()),
        },
        K::New { src, .. } => K::New {
            size: k.one(),
            src: src.as_ref().map(|_| k.one()),
        },
        K::TimeLit { unit_exp, .. } => K::TimeLit {
            num: k.one(),
            unit_exp: *unit_exp,
        },
        // A leaf has no children, so `children.is_empty()` already returned.
        _ => return e.clone(),
    };
    ast::Expr { kind, span: e.span }
}

/// Consumes [`shape_children`]'s list in order for [`rebuild`]. `shape` produced exactly the
/// children each arm asks for, so `one` never runs dry; the `Error` fallback keeps a future
/// AST node from panicking here.
struct Kids {
    it: std::vec::IntoIter<ast::Expr>,
    span: ast::Span,
}

impl Kids {
    fn one(&mut self) -> Box<ast::Expr> {
        // `shape` produced exactly the children each `rebuild` arm asks for, so running dry
        // means the two disagree — a bug, not an input. Assert so any future AST variant that
        // breaks the pairing fails the test suite loudly, and fall back to an inert `Error`
        // node in release rather than corrupting the expression silently.
        debug_assert!(
            self.it.len() > 0,
            "rebuild consumed more children than shape produced"
        );
        Box::new(self.it.next().unwrap_or(ast::Expr {
            kind: ast::ExprKind::Error,
            span: self.span,
        }))
    }

    /// The remaining children, for the variable-arity arms (concat parts, call args).
    fn rest(&mut self) -> Vec<ast::Expr> {
        self.it.by_ref().collect()
    }
}

/// A single-segment `Ident` expression reading `name`.
pub(crate) fn ident_expr(name: String, span: ast::Span) -> ast::Expr {
    ast::Expr {
        kind: ast::ExprKind::Ident(ast::HierPath {
            segments: vec![ast::Ident { name, span }],
            span,
        }),
        span,
    }
}

/// §4.5.250: a `$monitor`/`$strobe`-family task RE-RENDERS its argument list later — a
/// monitor on every change of what it watches, a strobe at the end of the time step (IEEE
/// 1800 §21.2). Hoisting anything out of those arguments evaluates it ONCE, at the
/// statement, so the later renders show a frozen temp — and an output-formal copy-out
/// hoisted out of them would fire once, early, instead of on each render.
///
/// The single list for every hoist that walks system-task arguments (`lower_stmt`'s
/// `$sformatf` child hoist reads it too), so the two cannot drift apart.
pub(crate) fn is_deferred_print_task(name: &str) -> bool {
    matches!(
        name,
        "$monitor"
            | "$monitorb"
            | "$monitoro"
            | "$monitorh"
            | "$fmonitor"
            | "$fmonitorb"
            | "$fmonitoro"
            | "$fmonitorh"
            | "$strobe"
            | "$strobeb"
            | "$strobeo"
            | "$strobeh"
            | "$fstrobe"
            | "$fstrobeb"
            | "$fstrobeo"
            | "$fstrobeh"
    )
}

/// A system FUNCTION that does not EVALUATE its operand — it reports a static property of
/// the operand's type (IEEE 1800 §20.5 `$bits`, §20.6 array queries). Hoisting a copy-out
/// out of one of these would fire a side effect the source never performs, so `shape` reports
/// the node `Opaque` and the general hoister stands down.
///
/// Measured: iverilog leaves the side effect unperformed for `$bits`; `$clog2` and
/// `$isunknown` DO evaluate their operand and are deliberately absent.
pub(crate) fn syscall_does_not_evaluate(name: &str) -> bool {
    matches!(
        name,
        "$bits"
            | "$size"
            | "$high"
            | "$low"
            | "$left"
            | "$right"
            | "$increment"
            | "$dimensions"
            | "$unpacked_dimensions"
            | "$typename"
    )
}
