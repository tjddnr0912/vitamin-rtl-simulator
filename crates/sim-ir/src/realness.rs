//! Static REAL-ness of one IR expression — the shared rule.
//!
//! Two crates need the same answer and a second spelling is how they drift:
//! `elaborate` uses it for the §6.2 illegality gates, the §4.1a format check and
//! the cast routing, while the ENGINE needs it to know that a binary is a REAL
//! operation before it evaluates either operand (IEEE §11.8.1 makes the
//! integral-to-real conversion boundary self-determined, so a real sibling's
//! 64-bit self width must not become the integral side's context).
//!
//! The `child` callback is what lets both callers share one rule: `elaborate`
//! recurses (its arena is still growing while it lowers), while the engine
//! decides the whole arena once, bottom-up, and reads its own memo table.

use crate::{ConstRepr, ConstVal, Expr, NetKind, NetVar, SysFuncId};
use std::collections::BTreeSet;

/// The N6 real-math system functions (IEEE §20.8.2) — all return `real`. Their
/// declared arity: `$pow`/`$atan2`/`$hypot` take 2 args, the rest take 1.
///
/// It lives here rather than in `elaborate` because it is a property of
/// `SysFuncId`, and `SysFuncId` lives here.
pub fn real_math_arity(which: SysFuncId) -> Option<usize> {
    use SysFuncId::*;
    match which {
        Pow | Atan2 | Hypot => Some(2),
        Ln | Log10 | Exp | Sqrt | Floor | Ceil | Sin | Cos | Tan | Asin | Acos | Atan | Sinh
        | Cosh | Tanh | Asinh | Acosh | Atanh => Some(1),
        _ => None,
    }
}

/// Everything the rule needs that is not an `ExprId`.
pub struct RealnessCtx<'a> {
    pub exprs: &'a [Expr],
    pub consts: &'a [ConstVal],
    pub nets: &'a [NetVar],
    /// DynArray handle NetIds whose ELEMENTS are `real` (`real d[]`). The net's
    /// own kind is `DynArray`, so the kind test cannot see them; this is the set
    /// that makes the engine store those elements as f64 in the first place, so
    /// it cannot claim a net the engine does not hold as real.
    pub real_elem_dyn_nets: &'a BTreeSet<u32>,
    /// Does this FuncId return a `real`? Conservative: an unknown callee is not
    /// claimed.
    pub func_ret_is_real: &'a dyn Fn(u32) -> bool,
}

/// Is `eid` real-typed? `child` answers for a sub-expression the caller has
/// already decided (or will decide by recursing).
pub fn expr_is_real_node(cx: &RealnessCtx, child: &dyn Fn(u32) -> bool, eid: u32) -> bool {
    match cx.exprs.get(eid as usize) {
        Some(Expr::Const { val }) => cx
            .consts
            .get(*val as usize)
            .is_some_and(|c| matches!(c.repr, ConstRepr::Real)),
        Some(Expr::Signal { net, .. }) => {
            cx.nets
                .get(*net as usize)
                .is_some_and(|n| matches!(n.kind, NetKind::Real))
                || cx.real_elem_dyn_nets.contains(net)
        }
        Some(Expr::Unary { op, operand }) => {
            matches!(op, crate::UnOp::Plus | crate::UnOp::Minus) && child(*operand)
        }
        // `**` is real-propagating (§11.4.4).
        // ⚠️ …and that entry is UNREACHABLE by construction, proven by a surviving
        // mutant: `elaborate` desugars a `**` into `SysFunc::Pow` under exactly
        // `expr_is_real(lhs) || expr_is_real(rhs)` — the same condition this arm
        // would fire on — so a `Binary { op: Pow }` with a real operand never
        // exists in the IR. It is kept because the LIST states the language rule,
        // not the current lowering: a future lowering that stops desugaring would
        // otherwise silently drop `**` out of the real domain.
        Some(Expr::Binary { op, lhs, rhs }) => {
            matches!(
                op,
                crate::BinOp::Add
                    | crate::BinOp::Sub
                    | crate::BinOp::Mul
                    | crate::BinOp::Div
                    | crate::BinOp::Pow
            ) && (child(*lhs) || child(*rhs))
        }
        Some(Expr::Ternary { then_e, else_e, .. }) => child(*then_e) || child(*else_e),
        // §4.5.317: `$signed`/`$unsigned` are TRANSPARENT to the value's domain.
        // ANY real argument counts, not just the first — vita currently accepts a
        // two-argument `$signed(r, a)` (ROADMAP §2), and keying on `args[0]` made
        // the answer depend on which slot the real landed in.
        Some(Expr::SysFunc {
            which: SysFuncId::Signed | SysFuncId::Unsigned,
            args,
        }) => args.iter().any(|a| child(*a)),
        Some(Expr::SysFunc { which, args }) => {
            matches!(
                which,
                SysFuncId::Realtime
                    | SysFuncId::Itor
                    | SysFuncId::BitsToReal
                    // `.atoreal()` returns `Value::from_f64`; without this arm
                    // `v[s.atoreal()]` read the f64 BIT PATTERN as an index —
                    // reachable with no `real` keyword in sight.
                    | SysFuncId::StrAtoreal
            ) || real_math_arity(*which).is_some()
                // `.sum()`/`.product()` fold with `arith`, which stays in the real
                // domain when the elements are real.
                || (matches!(which, SysFuncId::ArrSum | SysFuncId::ArrProduct)
                    && args.first().is_some_and(|a| child(*a)))
        }
        // A real-returning function. That claim is load-bearing for elaborate's
        // "must be integral" gates (a missing arm there is a silent-wrong at ~40
        // sites at once), and it is what a STATIC function needs — those inline,
        // so their result arrives through the `Signal` arm above and is answered
        // correctly.
        //
        // ⚠️ MEASURED INERT for an `automatic` (framed) callee used DIRECTLY as an
        // operand: `fa(1) + (-s)` still widens the integral sibling, and
        // `{fa(1), 1'b0}` is silently accepted where every sibling shape is loud.
        // Both are PRE-EXISTING (unchanged by the slice that introduced the engine
        // consumer) and both are recorded in ROADMAP §2 — the resolver is not the
        // hole; a framed call does not reach this arm with a resolvable FuncId.
        // Binding it through a temporary (`t = fa(1); t + (-s)`) is correct today.
        Some(Expr::Call { func, .. }) => (cx.func_ret_is_real)(*func),
        _ => false,
    }
}
