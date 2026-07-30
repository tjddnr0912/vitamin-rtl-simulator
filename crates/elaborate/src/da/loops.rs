//! R20 §3.3: when a LOOP may carry its body's assignment state past itself.
//!
//! Split from `da/mod.rs` to keep it under the 1000-line module cap.

use super::*;

/// R20 §3.3: may loop `st` carry its BODY's assignment state past the loop?
///
/// Normally it may not: the body might run zero times, so the pre-R20 arms all answered
/// `Falls(a0)` and a write that only happens inside the loop stayed invisible. Three
/// conditions make the claim sound, and all three are needed:
///
///   1. `loop_once(st)` — the body provably runs AT LEAST ONCE. A constant question,
///      answered by the Elaborator (see [`LoopRunsOnce`]); every unknown answers `false`.
///   2. the body FALLS THROUGH with `name` assigned. Tested as `Falls(true)`, NOT
///      `.state()`: a `Jumps` path collapses to `true` there as the identity of the join
///      merge, which would read as "assigned" for a body that never assigned anything.
///   3. no control transfer can leave the body ([`stmt_cannot_escape`]). This is the
///      condition that is easy to miss. A `break` — even a conditional one — lands
///      exactly HERE, past the loop, on a path where the write may not have run; and the
///      walk cannot report that path, because `If` merges a jumping arm by DROPPING it
///      (`merge(Jumps, Falls(x))` is `Falls(x)`), so the break is invisible in `body_out`.
///      A `continue` is the same hazard one iteration later. Hence a syntactic check.
pub(crate) fn loop_body_assigns(
    st: &ast::Stmt,
    body: &ast::Stmt,
    body_out: DaOut,
    loop_once: LoopRunsOnce,
) -> bool {
    matches!(body_out, DaOut::Falls(true)) && stmt_cannot_escape(body) && loop_once(st)
}

/// R20 §3.3: is `st` free of every control transfer that could leave it?
///
/// `break` / `continue` (the parser's synthetic `disable`), a `disable` of any other
/// block, and `return` all bypass whatever follows them, so a loop body containing one
/// cannot promise its writes ran (see [`loop_body_assigns`]). `$finish` / `$fatal` are
/// NOT transfers to worry about: they end the simulation, so the statement after the loop
/// is never reached and any claim about it is vacuous.
///
/// Conservative and `_`-free-exhaustive, because this feeds an ACCEPT gate: a future
/// statement form must be a compile error here, not a silently permitted escape. A
/// `break` belonging to a loop NESTED inside `st` is swallowed by that loop and does not
/// really escape, but it is rejected here anyway — that costs precision (the local stays
/// loud), never soundness.
pub(crate) fn stmt_cannot_escape(st: &ast::Stmt) -> bool {
    use ast::Stmt::*;
    let sub = |s: &ast::Stmt| stmt_cannot_escape(s);
    let opt = |s: &Option<Box<ast::Stmt>>| s.as_deref().is_none_or(stmt_cannot_escape);
    match st {
        Disable { .. } | Return { .. } => false,
        Block { stmts, .. } | Fork { stmts, .. } => stmts.iter().all(sub),
        If { then_s, else_s, .. } => sub(then_s) && opt(else_s),
        Case { items, .. } => items.iter().all(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => sub(body),
        }),
        For {
            init, step, body, ..
        } => sub(init) && sub(step) && sub(body),
        While { body, .. } | Repeat { body, .. } | Forever { body, .. } => sub(body),
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => opt(body),
        DeferredAssert { then_s, else_s, .. } => sub(then_s) && sub(else_s),
        ConcurrentAssert { pass, fail, .. } => opt(pass) && opt(fail),
        // No nested statement and no transfer of control.
        Blocking { .. }
        | NonBlocking { .. }
        | Assign { .. }
        | Force { .. }
        | Deassign { .. }
        | Release { .. }
        | UserTaskCall { .. }
        | SysTaskCall { .. }
        | RandomizeWith { .. }
        | EventTrigger { .. }
        | WaitFork { .. }
        | CoverProperty { .. }
        | Null(_)
        | Error(_) => true,
    }
}
