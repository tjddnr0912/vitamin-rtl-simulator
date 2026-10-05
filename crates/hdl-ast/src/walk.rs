//! The children of an [`Expr`] — the one place that knows which sub-expressions each
//! [`ExprKind`] variant holds, in which order they are evaluated, and under what
//! condition.
//!
//! Every walker that descends an expression filters this enumeration rather than
//! spelling its own match over the variants. Seven walkers used to spell their own,
//! and a rule added to one walk did not reach the others. The match in [`children`]
//! has no wildcard arm, so a new variant does not compile until its children are
//! listed here, and every walker built on [`Expr::for_each_child`] then sees them.
//!
//! Functions and plain enums only: nothing here derives `SchemaHash`, so the `.vu`
//! root is untouched by this module.

use crate::{BinOp, CastTarget, Expr, ExprKind};

/// How one child is evaluated relative to its parent.
///
/// The classes are the ones the general hoister models (`elaborate`'s `hoist::Shape`).
/// Children are visited in the left-to-right order iverilog evaluates them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildPos {
    /// Evaluated unconditionally, exactly once, in visit order.
    Uncond,
    /// The left operand of `&&` / `||`: unconditional; its truth decides whether the
    /// right operand runs (IEEE 1800 §11.4.7).
    ShortCircuitLhs,
    /// The right operand of `&&` / `||`: evaluated only when the left operand does
    /// not decide the result.
    ShortCircuitRhs,
    /// `c` in `c ? t : e`: unconditional.
    TernaryCond,
    /// `t` in `c ? t : e`: evaluated unless `c` is definitely false. An x `c`
    /// evaluates both arms (§11.4.11).
    TernaryThen,
    /// `e` in `c ? t : e`: evaluated unless `c` is definitely true.
    TernaryElse,
    /// Evaluated, but not exactly once in source order: one of `min:typ:max` is
    /// chosen, a `with` clause runs per element, a `dist` and a `randomize() with`
    /// argument or constraint belong to the solver.
    NoHoist,
    /// Not evaluated at all: an operand of a system function that reports a property
    /// of its operand's type ([`syscall_does_not_evaluate`]).
    Unevaluated,
}

/// How a node evaluates its children as a whole — [`ChildPos`] for the node rather
/// than for one child, so it is defined for a node with no children too (an empty
/// `randomize() with {}` is still a solver node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    /// Every child is [`ChildPos::Uncond`]. Every leaf has this regime.
    Uncond,
    /// `&&` / `||`. `short_on` is the left operand's truth that skips the right
    /// operand: `false` for `&&`, `true` for `||`.
    ShortCircuit { short_on: bool },
    /// `c ? t : e`.
    Ternary,
    /// Every child is [`ChildPos::NoHoist`].
    NoHoist,
    /// Every child is [`ChildPos::Unevaluated`].
    Unevaluated,
}

impl Expr {
    /// Calls `f` once for each direct child sub-expression, in evaluation order, with
    /// the child's [`ChildPos`]. A caller that needs a subset filters on the position
    /// or on its own `self.kind`; it does not list children itself.
    pub fn for_each_child<'a>(&'a self, mut f: impl FnMut(ChildPos, &'a Expr)) {
        children(self, &mut f);
    }

    /// This node's [`Regime`].
    pub fn regime(&self) -> Regime {
        children(self, &mut |_, _| {})
    }
}

/// A system FUNCTION that does not EVALUATE its operand: it reports a static property
/// of the operand's type (IEEE 1800 §20.5 `$bits`, §20.6 array queries). Its arguments
/// are [`ChildPos::Unevaluated`], so nothing in them is read at run time, and moving a
/// side effect out of one would perform an effect the source never performs.
///
/// Measured: iverilog leaves the side effect unperformed for `$bits`; `$clog2` and
/// `$isunknown` DO evaluate their operand and are deliberately absent.
pub fn syscall_does_not_evaluate(name: &str) -> bool {
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

/// The single exhaustive match. Visits `e`'s children through `f` and returns the
/// node's regime. NO wildcard arm: a new `ExprKind` variant fails to compile here.
fn children<'a, F: FnMut(ChildPos, &'a Expr)>(e: &'a Expr, f: &mut F) -> Regime {
    use ChildPos as P;
    use ExprKind as K;
    match &e.kind {
        K::IntLit { .. }
        | K::RealLit { .. }
        | K::StrLit { .. }
        | K::PkgScoped { .. }
        | K::Ident(_)
        | K::Null
        | K::Dollar
        | K::Error => Regime::Uncond,
        K::Unary { operand, .. } => {
            f(P::Uncond, operand);
            Regime::Uncond
        }
        K::Binary { op, lhs, rhs } => match op {
            BinOp::LogAnd | BinOp::LogOr => {
                f(P::ShortCircuitLhs, lhs);
                f(P::ShortCircuitRhs, rhs);
                Regime::ShortCircuit {
                    short_on: matches!(op, BinOp::LogOr),
                }
            }
            _ => {
                f(P::Uncond, lhs);
                f(P::Uncond, rhs);
                Regime::Uncond
            }
        },
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            f(P::TernaryCond, cond);
            f(P::TernaryThen, then_e);
            f(P::TernaryElse, else_e);
            Regime::Ternary
        }
        K::BitSelect { base, index } => {
            f(P::Uncond, base);
            f(P::Uncond, index);
            Regime::Uncond
        }
        K::PartSelect { base, msb, lsb } => {
            f(P::Uncond, base);
            f(P::Uncond, msb);
            f(P::Uncond, lsb);
            Regime::Uncond
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            f(P::Uncond, base);
            f(P::Uncond, offset);
            f(P::Uncond, width);
            Regime::Uncond
        }
        K::Concat { parts } => {
            parts.iter().for_each(|c| f(P::Uncond, c));
            Regime::Uncond
        }
        // `{n{x,y}}`: the count, then the repeated elements.
        K::Replicate { count, value } => {
            f(P::Uncond, count);
            value.iter().for_each(|c| f(P::Uncond, c));
            Regime::Uncond
        }
        K::Call { args, .. } | K::ClassNew { args } => {
            args.iter().for_each(|c| f(P::Uncond, c));
            Regime::Uncond
        }
        K::SysCall { name, args } => {
            if syscall_does_not_evaluate(&name.name) {
                args.iter().for_each(|c| f(P::Unevaluated, c));
                Regime::Unevaluated
            } else {
                args.iter().for_each(|c| f(P::Uncond, c));
                Regime::Uncond
            }
        }
        K::MethodCall { recv, args, .. } => {
            f(P::Uncond, recv);
            args.iter().for_each(|c| f(P::Uncond, c));
            Regime::Uncond
        }
        K::Paren { inner } => {
            f(P::Uncond, inner);
            Regime::Uncond
        }
        // `N'(e)`: the width expression is a child too, ahead of the operand.
        K::Cast { target, expr } => {
            if let CastTarget::Size(n) = target {
                f(P::Uncond, n);
            }
            f(P::Uncond, expr);
            Regime::Uncond
        }
        K::AssignPattern(parts) => {
            parts.iter().for_each(|c| f(P::Uncond, c));
            Regime::Uncond
        }
        // The keys are names, not expressions; only the values are children.
        K::AssignPatternKeyed(kv) => {
            kv.iter().for_each(|(_, c)| f(P::Uncond, c));
            Regime::Uncond
        }
        K::NamedArg { value, .. } => {
            if let Some(v) = value {
                f(P::Uncond, v);
            }
            Regime::Uncond
        }
        K::New { size, src } => {
            f(P::Uncond, size);
            if let Some(s) = src {
                f(P::Uncond, s);
            }
            Regime::Uncond
        }
        K::TimeLit { num, .. } => {
            f(P::Uncond, num);
            Regime::Uncond
        }
        K::MinTypMax { min, typ, max } => {
            f(P::NoHoist, min);
            f(P::NoHoist, typ);
            f(P::NoHoist, max);
            Regime::NoHoist
        }
        // The sampled value, then each item's `lo`, optional `hi` and weight.
        K::Dist { value, items } => {
            f(P::NoHoist, value);
            for it in items {
                f(P::NoHoist, &it.lo);
                if let Some(hi) = &it.hi {
                    f(P::NoHoist, hi);
                }
                f(P::NoHoist, &it.weight);
            }
            Regime::NoHoist
        }
        K::RandomizeWith(rw) => {
            rw.args.iter().for_each(|c| f(P::NoHoist, c));
            rw.constraints.iter().for_each(|c| f(P::NoHoist, c));
            Regime::NoHoist
        }
        K::ArrayMethodWith(am) => {
            f(P::NoHoist, &am.with_expr);
            Regime::NoHoist
        }
    }
}
