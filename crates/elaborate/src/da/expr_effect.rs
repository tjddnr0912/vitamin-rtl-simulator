//! R19 §3.1: what evaluating an EXPRESSION does to one block-local `name`.
//!
//! Before this, the definite-assignment walk recognized a write through an `output`
//! actual in exactly two places: a bare call STATEMENT, and a whole condition that
//! was a call after unwrapping `!`/`(…)` (`cond_out_writes`). Everything else asked
//! only [`expr_no_ref`] — "does this expression mention `name`?" — and a mention was
//! always read as a READ. So the moment the call RETURNED A VALUE, the shape that
//! puts it anywhere an expression can go, the write became invisible:
//!
//! ```systemverilog
//! go = nxt(5, r);                       // rhs of an assignment
//! if (nxt(5, r) == 1) …                 // an operand of `==`
//! while (n < lim && rsp_next(fd, r) == 1) …   // an operand of `&&`
//! ```
//!
//! All three write `r` before anything can read it, and all three were rejected —
//! 33 of the round-19 report's 34 diagnostics, and the whole of its `TB=sha2`
//! regression. The trigger was never the formal's type or the loop: it was
//! "does the call return a value", because that is what moves it out of statement
//! position.
//!
//! The walk here answers the two questions the statement-level walk needs:
//! [`expr_da`] for an expression evaluated on its own, and [`expr_writes_when`] for
//! a branch entered because that expression was true (or false) — where a
//! short-circuit `&&` operand IS guaranteed to have run.
//!
//! SCOPE — deliberately the same node set the LOWERING hoists a copy-out from
//! (`hoist::hoist_inout_calls`: `Paren` / `Unary` / `Binary`), plus `Ternary` for
//! symmetry. Any other node answers exactly what the pre-R19 gate answered
//! (`expr_no_ref` ⇒ `Clean`, else `Reads`), so nothing outside that set changes.

use super::*;

/// What evaluating an expression does to `name`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ExprDa {
    /// No READ of `name` can happen while this expression is evaluated. A write MAY
    /// happen (a conditionally evaluated operand) — that is deliberately not claimed
    /// and does not need to be: a write is not a read, so nothing is unsafe, and
    /// leaving the local "still unwritten" is the conservative side.
    Clean,
    /// EVERY evaluation of this expression writes `name` whole, through an output
    /// actual whose copy-out reads nothing of it, and no read of `name` can precede
    /// that write inside the expression.
    Writes,
    /// The expression may READ `name`. Unsafe while `name` is unwritten — this is the
    /// verdict that ends the walk.
    Reads,
}

/// Two sub-expressions BOTH evaluated, in an order IEEE does not fix (every binary
/// operator except `&&`/`||`). A read on either side may therefore come first, so a
/// write is claimed only when the other side cannot read.
fn join(l: ExprDa, r: ExprDa) -> ExprDa {
    match (l, r) {
        (ExprDa::Reads, _) | (_, ExprDa::Reads) => ExprDa::Reads,
        (ExprDa::Writes, _) | (_, ExprDa::Writes) => ExprDa::Writes,
        _ => ExprDa::Clean,
    }
}

/// The effect of evaluating `e` once, on its own.
pub(crate) fn expr_da(e: &ast::Expr, name: &str, out_writes: OutActualWrites) -> ExprDa {
    use ast::ExprKind as K;
    let sub = |x: &ast::Expr| expr_da(x, name, out_writes);
    match &e.kind {
        K::Paren { inner } => sub(inner),
        // A unary operator always evaluates its operand.
        K::Unary { operand, .. } => sub(operand),
        K::Call { name: cn, args } => match out_writes(cn, args, name) {
            CallEffect::Writes => ExprDa::Writes,
            // ADVERSARIAL-REVIEW FIX. Everything that is NOT the new write claim must
            // answer what the pre-R19 walk answered for a call in EXPRESSION position,
            // which was `expr_no_ref` — not `call_effect`. The difference is not
            // cosmetic: `call_effect` says `Unknown` for any callee it cannot resolve or
            // whose body it cannot vet, so routing it here would have turned
            // `while (b < 3 && g(1)) … a …` — a condition that never mentions `a` — into
            // a read of `a`, a false-loud on code that worked. (Statement position IS
            // strict about this, and has been since R16; that asymmetry is pre-existing
            // and deliberately left alone here.)
            _ => {
                if expr_no_ref(e, name) {
                    ExprDa::Clean
                } else {
                    ExprDa::Reads
                }
            }
        },
        // `&&` / `||` evaluate the LEFT operand first and always; the right one only
        // when the left did not already decide the result (IEEE 1800 §11.4.7).
        K::Binary {
            op: op @ (ast::BinOp::LogAnd | ast::BinOp::LogOr),
            lhs,
            rhs,
        } => {
            match sub(lhs) {
                // The left write is complete before the right operand is evaluated at
                // all, so a read there is a read of a WRITTEN `name` — still `Writes`.
                // (This is the one place the fixed order buys precision; `join` cannot
                // say it.)
                ExprDa::Writes => ExprDa::Writes,
                ExprDa::Reads => ExprDa::Reads,
                ExprDa::Clean => match sub(rhs) {
                    // A write the right operand might not perform cannot be claimed —
                    // but it is not a read either, so this is `Clean`, not `Reads`.
                    ExprDa::Writes | ExprDa::Clean => ExprDa::Clean,
                    // r19 follow-on. `Clean` on the left does not mean "no write on the
                    // left" — it also covers a CONDITIONAL write, which is what a nested
                    // `&&` yields (`(n < 10 && rsp_next(n, r) == 1) && r.len >= 0`: the
                    // inner right operand writes, so the inner verdict is `Clean`). The
                    // read on the right is then reached ONLY on a path where that
                    // conditional write did happen, so it is not a read-before-write and
                    // must not end the walk.
                    //
                    // The verdict is `Clean`, NOT `Writes`. `Writes` means EVERY evaluation
                    // of this expression writes `name`, and that is false here: when the
                    // left short-circuits, the call never runs and the right is never
                    // reached — nothing is written. Claiming `Writes` made the gate treat
                    // the local as assigned on the short-circuit path too, so a sibling
                    // block's leftover value was read at exit 0 instead of being rejected
                    // (caught by the differential review; `Clean` is the exact claim: the
                    // read is safe, the write is not promised).
                    //
                    // Sound including an x-valued left operand, which also reaches the
                    // right. `expr_writes_when` can only answer `true` by descending the
                    // left's own short-circuit chain to a leaf that writes whenever it is
                    // evaluated, and reaching THIS right operand means the left chain did
                    // not short-circuit — so every operand of it, including that leaf, was
                    // evaluated. (Measured: an x left operand does leave the write done; a
                    // definitely-false one never evaluates the right at all.)
                    ExprDa::Reads => {
                        let reaches_rhs = matches!(op, ast::BinOp::LogAnd);
                        if expr_writes_when(lhs, name, out_writes, reaches_rhs) {
                            ExprDa::Clean
                        } else {
                            ExprDa::Reads
                        }
                    }
                },
            }
        }
        K::Binary { lhs, rhs, .. } => join(sub(lhs), sub(rhs)),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => match sub(cond) {
            ExprDa::Writes => ExprDa::Writes,
            ExprDa::Reads => ExprDa::Reads,
            // Exactly one arm runs, so neither arm's write is guaranteed; either
            // arm's read may happen.
            ExprDa::Clean => match (sub(then_e), sub(else_e)) {
                (ExprDa::Reads, _) | (_, ExprDa::Reads) => ExprDa::Reads,
                _ => ExprDa::Clean,
            },
        },
        // Every other node — a concat, a select, a system call, a method call, a
        // literal — is not a position the lowering can hoist a copy-out out of, so no
        // write is recognized there and the answer is the pre-R19 one, verbatim.
        _ => {
            if expr_no_ref(e, name) {
                ExprDa::Clean
            } else {
                ExprDa::Reads
            }
        }
    }
}

/// R19 §3.1: given that `e` was evaluated AND its value is `want`, is `name`
/// definitely written by then?
///
/// This is what a branch body needs, and it is strictly stronger than [`expr_da`] for
/// the report's actual site:
///
/// ```systemverilog
/// while (n < sweep_limit && rsp_next(fd, r) == 1) begin … r … end
/// ```
///
/// The loop BODY runs only when the whole condition is true, and `a && b` is true
/// only when BOTH operands were evaluated — so the call has written `r` by the time
/// the body starts, even though the loop EXIT (a false condition, where the call may
/// have been short-circuited away) carries no such guarantee. `||` is the mirror
/// image: false only when both operands ran, which is what an `else` branch gets.
///
/// SELF-GUARDING: a subtree that may READ `name` answers `false` outright, so a
/// caller cannot use this to claim a write that a read came before.
pub(crate) fn expr_writes_when(
    e: &ast::Expr,
    name: &str,
    out_writes: OutActualWrites,
    want: bool,
) -> bool {
    use ast::ExprKind as K;
    match expr_da(e, name, out_writes) {
        ExprDa::Writes => return true,
        ExprDa::Reads => return false,
        ExprDa::Clean => {}
    }
    let go = |x: &ast::Expr, w: bool| expr_writes_when(x, name, out_writes, w);
    match &e.kind {
        K::Paren { inner } => go(inner, want),
        // `!x` is true exactly when `x` is false.
        K::Unary {
            op: ast::UnOp::LogNot,
            operand,
        } => go(operand, !want),
        K::Binary {
            op: ast::BinOp::LogAnd,
            lhs,
            rhs,
        } if want => go(lhs, true) || go(rhs, true),
        K::Binary {
            op: ast::BinOp::LogOr,
            lhs,
            rhs,
        } if !want => go(lhs, false) || go(rhs, false),
        _ => false,
    }
}
