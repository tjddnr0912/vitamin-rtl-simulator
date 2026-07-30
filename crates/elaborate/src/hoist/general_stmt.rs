//! r19 follow-on — the GENERAL hoister's STATEMENT arms, split from `hoist/mod.rs` to
//! hold both files under the 1000-line module cap.
//!
//! [`Elaborator::hoist_stmt_general`] is the LAST thing `hoist_stmt_top` tries. Every
//! shape-specific arm there runs first and keeps its exact lowering; these arms only see
//! what those declined, which is where a value-returning output-formal call can appear but
//! no hoist site existed: a `case` scrutinee, a `repeat` count, a `$display`/task argument,
//! a non-blocking rhs, a `return` value, an lvalue index, and any `if`/assign whose
//! expression carries the call too deep for the narrow walker.

use super::general_ast::assign_seq;
use super::*;

impl Elaborator<'_> {
    /// The general hoister's statement dispatch — see the module comment. `None` ⇒ nothing
    /// here is hoistable, so `lower_stmt` proceeds unchanged (and a call in a position the
    /// hoister cannot reach stays loud at `emit_frame_call`).
    pub(crate) fn hoist_stmt_general(
        &mut self,
        b: &mut ProcessBuilder,
        s: &ast::Stmt,
    ) -> Option<ast::Stmt> {
        use ast::Stmt as S;
        // A frame function/task body cannot host a copy-out `Terminator::Call`: its writes
        // must target frame-LOCAL nets, and an output actual is a module net. Emitting one
        // there made `frames_classify` report a cause the source does not contain, and in a
        // `task automatic` body it reached the engine and tripped
        // `debug_assert!(frame_local[net])` — a panic with no diagnostic (and, with the
        // assert stripped, a write into the wrong net). BOTH flags matter: a function body
        // sets `frame_fn_lowering`, a task body sets `frame_task_lowering`.
        if self.in_frame_body() {
            return None;
        }
        match s {
            // ══════════════════════════════════════════════════════════════════
            //  r19 follow-on — the GENERAL hoister (`hoist/general.rs`)
            // ══════════════════════════════════════════════════════════════════
            // Reached only when every arm above declined, so each statement form those
            // arms already own keeps its exact lowering. These arms cover the positions
            // a value-returning output-formal call can reach that the shape-specific
            // arms cannot: a call buried in a concat / another call's argument / a select
            // index, a `case` scrutinee, a `repeat` count, a `$display` argument, a task
            // call's argument, a `?:` arm inside a bigger expression, an lvalue index.
            S::If {
                cond,
                then_s,
                else_s,
                span,
            } if self.general_hoist_ok(cond) && !self.cond_needs_shortcircuit_split(cond) => {
                let cond2 = self.hoist_inout_general_top(b, cond);
                Some(S::If {
                    cond: cond2,
                    then_s: then_s.clone(),
                    else_s: else_s.clone(),
                    span: *span,
                })
            }
            // A blocking / non-blocking assign: the rhs, and the LVALUE's index
            // expressions. iverilog evaluates the RHS FIRST and the lvalue index second
            // (measured: `arr[mark(1)] = mark(2)` traces `2` then `1`), so they are
            // hoisted in that order. `shortcircuit_rhs_special` keeps ownership of a
            // whole-rhs top-level `?:` / `&&` / `||` (`sc_rhs_owned`).
            S::Blocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && !self.sc_rhs_owned(lhs, delay.as_ref(), rhs)
                && self.general_hoist_ok_seq(&assign_seq(lhs, rhs)) =>
            {
                // ONE sequence in evaluation order — the rhs first, then the lvalue's index
                // expressions (measured: `arr[mark(1)] = mark(2)` traces `2` then `1`).
                // Analysing the rhs and each index separately hid the cross-boundary
                // hazards: an index's copy-out lands BEFORE the rhs is evaluated, so
                // `arr[nxt(0, o)] = o` has to snapshot `o`, and neither piece alone sees it.
                let (rhs2, lhs2) = self.hoist_assign_seq(b, lhs, rhs);
                Some(S::Blocking {
                    lhs: lhs2,
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            S::NonBlocking {
                lhs,
                delay,
                event,
                rhs,
                span,
            } if delay.is_none()
                && event.is_none()
                && self.general_hoist_ok_seq(&assign_seq(lhs, rhs)) =>
            {
                // Same one-sequence treatment as the blocking arm above.
                let (rhs2, lhs2) = self.hoist_assign_seq(b, lhs, rhs);
                Some(S::NonBlocking {
                    lhs: lhs2,
                    delay: delay.clone(),
                    event: event.clone(),
                    rhs: rhs2,
                    span: *span,
                })
            }
            // A `case` scrutinee is evaluated exactly once, before any label compare.
            S::Case {
                kind,
                scrutinee,
                items,
                span,
            } if self.general_hoist_ok(scrutinee) => {
                let s2 = self.hoist_inout_general_top(b, scrutinee);
                Some(S::Case {
                    kind: *kind,
                    scrutinee: s2,
                    items: items.clone(),
                    span: *span,
                })
            }
            // `repeat (n)` evaluates `n` ONCE (measured: `repeat (nxt(2))` ticks three
            // times with a single copy-out), so a one-shot hoist at the statement is
            // right — unlike a `while`/`for` condition, which is per-iteration.
            S::Repeat { count, body, span } if self.general_hoist_ok(count) => {
                let c2 = self.hoist_inout_general_top(b, count);
                Some(S::Repeat {
                    count: c2,
                    body: body.clone(),
                    span: *span,
                })
            }
            // `$display`/`$write`/… arguments are each evaluated once, unconditionally.
            // §4.5.250: a `$monitor`/`$strobe`-family task RE-RENDERS its arguments later,
            // so a hoist would freeze them at the statement and fire the copy-out once,
            // early (measured: a monitor printed the same stale temp on every change).
            S::SysTaskCall { name, args, span }
                if !is_deferred_print_task(&name.name)
                    && self.general_hoist_ok_seq(&args.iter().collect::<Vec<_>>()) =>
            {
                let args2 = self.hoist_inout_general_seq(b, &args.iter().collect::<Vec<_>>());
                Some(S::SysTaskCall {
                    name: name.clone(),
                    args: args2,
                    span: *span,
                })
            }
            // A user task/function call STATEMENT whose own argument carries a call
            // (`tsk(nxt(5, o) - 1, out x)`). The statement's own output actuals are plain
            // lvalues, so they pass through the rewrite untouched.
            // The statement's OWN output/inout actuals are write DESTINATIONS, not reads:
            // rewriting one to a pre-call snapshot sent the callee's copy-out into the
            // snapshot net and left the user's variable stale — a LOST WRITE at exit 0. So
            // only the `input` actuals join the analysed/rewritten sequence; the rest pass
            // through untouched, and a call buried in one of them stays loud.
            S::UserTaskCall { name, args, span }
                if self.task_call_input_args(name, args).is_some_and(|inputs| {
                    self.general_hoist_ok_seq(&inputs)
                        // A call buried in a write destination is not hoistable from here.
                        && !args
                            .iter()
                            .enumerate()
                            .any(|(i, a)| {
                                !self.task_call_arg_is_input(name, args, i)
                                    && self.expr_has_inout_call_deep(a)
                            })
                        // An `inout` actual has a copy-IN, so it READS its actual — and it is
                        // also the write destination, so the read cannot be redirected to a
                        // snapshot. If one of the input args writes that same root, the
                        // copy-in would see the post-call value: stand down.
                        // (A pure `output` actual is not read, which is why it does not
                        // block: `tk(o, nxt(5, o))` correctly passes `o` through.)
                        && !self.task_call_inout_root_written(name, args, &inputs)
                }) =>
            {
                let inputs = self.task_call_input_args(name, args)?;
                // One sequence, so an earlier input's read is repaired against a LATER
                // input's call (`tk(nxt(5, o), o)` passes the pre-call `o`).
                let mut rewritten = self.hoist_inout_general_seq(b, &inputs).into_iter();
                let args2 = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        if self.task_call_arg_is_input(name, args, i) {
                            rewritten.next().unwrap_or_else(|| a.clone())
                        } else {
                            a.clone()
                        }
                    })
                    .collect();
                Some(S::UserTaskCall {
                    name: name.clone(),
                    args: args2,
                    span: *span,
                })
            }
            // NOTE: there is deliberately no `S::Return` arm. A `return <expr>` occurs only
            // inside a function/task body, where `in_frame_body()` has already returned
            // `None` above — an arm here would be dead code claiming a capability that does
            // not exist. The call reports its own message instead.
            _ => None,
        }
    }
}
