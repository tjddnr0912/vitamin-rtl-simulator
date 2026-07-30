//! definite-assignment / ident-read AST queries — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

mod expr_effect;
mod loops;
mod reads;
mod writes;

pub(crate) use expr_effect::*;
pub(crate) use loops::*;
pub(crate) use reads::*;
pub(crate) use writes::*;

/// BL4 (round-19): the callee-signature resolver da.rs is threaded with. Given a
/// callee PATH, its positional ARGS, and the local `name`, returns `true` iff `name`
/// is at a PURE OUTPUT actual position of that call (a whole-var copy-out, IEEE
/// §13.5.2) and appears at NO read position of the SAME call (no input actual, no
/// select index, and — critically — no INOUT actual, whose copy-in READS `name`).
/// Built in `block_local.rs` from the Elaborator's `func_table` / `task_table` port
/// directions. Returns `false` for anything it cannot resolve (hierarchical callee,
/// named args, unknown name) → the DA walk then treats the reference conservatively
/// as a read (correct-or-loud). da.rs itself has no view of port directions, hence
/// the closure.
pub(crate) type OutActualWrites<'a> = &'a dyn Fn(&ast::HierPath, &[ast::Expr], &str) -> CallEffect;

/// R18-X1: "can enabling this task let simulation time advance?" — resolved against
/// the Elaborator's `task_table`/`func_table` in `block_local::gate`, because `da`
/// itself has no view of callee bodies (the same reason [`OutActualWrites`] is a
/// closure).
///
/// Answers `true` for anything it cannot resolve. This predicate is only ever read
/// on the REJECT side (a shared flattened net plus a suspend ⇒ loud), so an
/// unresolvable callee must be assumed to suspend.
pub(crate) type CallSuspends<'a> = &'a dyn Fn(&ast::HierPath) -> bool;

/// R20 §3.3: "does this LOOP statement provably execute its body at least once?"
///
/// A loop cannot normally carry its body's assignment out, because it may run zero
/// times — so `for (int j = 0; j < 3; j++) fill(cur);` left `cur` unwritten for the
/// walk and the read after the loop was a read-before-write. The trip count is a
/// CONSTANT question, and answering it needs the Elaborator's parameter scope
/// (`const_eval_in_scope`), which `da` has no view of — the same reason
/// [`OutActualWrites`] and [`CallSuspends`] are closures.
///
/// POLARITY: this is read on the ACCEPT side (a `true` lets the walk claim the local
/// assigned), so every unknown must answer `false`. Passing a closure that always
/// answers `false` is mechanically the pre-R20 walk.
pub(crate) type LoopRunsOnce<'a> = &'a dyn Fn(&ast::Stmt) -> bool;

/// R20: the resolvers and flags the definite-assignment walk carries unchanged from top to
/// bottom, bundled so adding one does not re-thread every recursive call (and does not push
/// the walk past clippy's argument limit, which is the same warning about the same thing).
#[derive(Clone, Copy)]
pub(crate) struct DaCtx<'a> {
    /// What a CALL does to the local — see [`OutActualWrites`].
    pub(crate) out_writes: OutActualWrites<'a>,
    /// Can enabling a task let time advance? — see [`CallSuspends`].
    pub(crate) suspends: CallSuspends<'a>,
    /// Does a loop provably run its body once? — see [`LoopRunsOnce`].
    pub(crate) loop_once: LoopRunsOnce<'a>,
    /// Is this block the ONLY writer of the flattened net? `false` when a same-named
    /// block-local in another block shares it, which is what makes a suspend dangerous.
    pub(crate) sole: bool,
}

/// R16 §3.2: what a CALL does to the local `name`.
///
/// The pre-R16 resolver answered one bit — "is this a pure output-actual write?" —
/// and everything else collapsed to "conservatively a read", which made a statement
/// call like `show(0);` abort the definite-assignment walk. Three verdicts separate
/// the two distinct reasons a call can be harmless from the reason it is not.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallEffect {
    /// Provably touches `name` at no position — no actual references it AND the
    /// callee's body (transitively) cannot reach the flattened net. The walk steps
    /// over it with the assigned-state unchanged.
    Inert,
    /// Definitely WRITES `name` whole, through a pure output actual with no
    /// same-call read (IEEE §13.5.2 copy-out).
    Writes,
    /// R17 §3.2: provably does NOT write `name`, but does READ it — every actual
    /// mentioning `name` sits at an `input` formal (copy-in only), the callee
    /// resolves, and its body cannot reach the flattened net. Distinct from
    /// `Unknown` because "cannot write" is the fact the never-written argument needs:
    /// a local deliberately left empty and passed by value stays at its default, so
    /// the flatten and per-entry storage agree. For the walk itself this is still a
    /// read, and behaves exactly like `Unknown`.
    Reads,
    /// Anything else: an input / inout / select reference, a callee that cannot be
    /// resolved, or a body that may reach `name`. Conservatively a read.
    Unknown,
}

/// BL4, generalized in R19 §3.1: is the WHOLE expression `e` guaranteed to WRITE
/// `name` through an output actual, with no read of `name` first?
///
/// BL4's version unwrapped only `!`/`(…)` and matched a bare `Call`, which is the
/// STATEMENT-shaped subset. [`expr_da`] answers the same question for an expression
/// of any shape the lowering can emit a copy-out from — the rhs of an assignment,
/// an operand of `==`, the left operand of `&&` — which is where a call that
/// RETURNS A VALUE actually appears.
fn expr_out_writes(e: &ast::Expr, name: &str, out_writes: OutActualWrites) -> bool {
    expr_da(e, name, out_writes) == ExprDa::Writes
}

/// R19 §3.1: does `e` READ `name`? The read side of [`expr_da`], and the replacement
/// for the bare `!expr_no_ref(e, name)` the walk used to ask.
///
/// The difference is only ever in vita's favour and only ever about CALLS: an
/// expression whose sole mention of `name` is an output actual (`nxt(5, r)`) does
/// not read it, and one where that call sits in a conditionally-evaluated operand
/// (`c && nxt(5, r)`) does not read it either — it merely may or may not write. For
/// every other node `expr_da` falls back to `expr_no_ref` verbatim.
fn expr_reads(e: &ast::Expr, name: &str, out_writes: OutActualWrites) -> bool {
    expr_da(e, name, out_writes) == ExprDa::Reads
}

/// R16 §3.1: what control does AFTER a statement, for definite-assignment purposes.
///
/// The pre-R16 walker carried only the assigned BOOL, which silently equated "runs
/// on to the next statement" with "jumps away". That made a `break`/`continue`
/// placed BEFORE the first write read as a live path reaching the later read, and
/// 49 of the 84 diagnostics in the round-16 report were that one conflation: every
/// path that actually reaches the read has already executed the write.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DaOut {
    /// Control reaches the NEXT statement, with this assigned-state.
    Falls(bool),
    /// Control does NOT reach the next statement — a `break`/`continue` (which the
    /// parser desugars to a `disable` of a synthetic enclosing-loop label) or a
    /// `return`. A join ignores such a path entirely (IEEE 1800 §11.5 / the standard
    /// definite-assignment rule): nothing downstream of it executes on this path.
    Jumps,
}

impl DaOut {
    /// The assigned-state to enter the next statement with. A `Jumps` path never
    /// reaches it, so it contributes the identity of the `&&` merge below.
    fn state(self) -> bool {
        match self {
            DaOut::Falls(a) => a,
            DaOut::Jumps => true,
        }
    }

    /// Merge two arms of an `if` / `case` at their join point. A non-falling arm
    /// drops out (it never reaches the join); if BOTH jump, the join is unreachable
    /// and the whole construct jumps.
    fn merge(self, other: DaOut) -> DaOut {
        match (self, other) {
            (DaOut::Jumps, DaOut::Jumps) => DaOut::Jumps,
            _ => DaOut::Falls(self.state() && other.state()),
        }
    }
}

/// R17 §3.3 / §4.2: WHERE and WHY the definite-assignment walk stopped.
///
/// The round-17 report could not reduce 21 of its 34 diagnostics because E3009's
/// lifetime message names only two possible causes — "an initializer" and "a read
/// before its first write" — while the actual cause for most sites is a third one
/// the message never mentions: the walk reached a construct it does not model and
/// answered "unsafe" for the whole block. The declaration is the only location
/// printed, so the construct that ended the scan can be in another statement, another
/// function, or another FILE (§3.1b), and nothing at the printed location is wrong.
///
/// `da_stmt` now carries this out of the walk so the caller can attach a `note:` at
/// the construct's own span. It is diagnostic-only: no accept/reject decision reads
/// it.
#[derive(Clone, Copy)]
pub(crate) struct DaGiveUp {
    /// The span of the construct that stopped the walk — a statement, or the
    /// expression within it that referenced the local.
    pub(crate) span: ast::Span,
    /// A short phrase completing "…stopped here: {what}".
    pub(crate) what: &'static str,
}

impl DaGiveUp {
    fn at(span: ast::Span, what: &'static str) -> Self {
        DaGiveUp { span, what }
    }
}

/// The walk's result: the control-flow/assigned state after a statement, or the
/// give-up that ended it. `Result` rather than `Option` so every `?` propagates the
/// FIRST (innermost, earliest) give-up — the one the user has to look at.
type DaResult = Result<DaOut, DaGiveUp>;

/// The give-up phrase for "the flattened net is shared and time can advance here".
/// Module-level because BOTH the per-statement walk and the top-level loop raise it —
/// the top-level one is R18-X1's fix and the reason it must not be a `da_stmt` local.
const SHARED_SUSPEND: &str = "time can advance here, and the flattened net is shared \
     with a same-named block-local in another block — that block can write it before \
     this one reads it again";

/// R16 §3.1: is this `disable` target the parser's synthetic label for a `break` /
/// `continue`? Those labels are `$break$<lo>` / `$continue$<lo>` (see
/// `hdl-parser::stmt_ctl`), and the leading `$` makes them unwritable as a user
/// block name — so this cannot mistake a real `disable` for a loop jump.
///
/// The distinction is load-bearing and must NOT be widened to `disable` at large: a
/// `disable` naming a block that is not an ancestor of this statement kills that
/// other block and lets THIS one run on, so treating it as `Jumps` would drop a live
/// path from the join and silently accept a genuine read-before-write. A loop jump,
/// by contrast, always targets a lexical ancestor, so control provably leaves every
/// statement between here and that loop.
fn is_loop_jump(target: &ast::HierPath) -> bool {
    target.segments.len() == 1
        && matches!(target.segments.first(), Some(s)
            if s.name.starts_with("$break$") || s.name.starts_with("$continue$"))
}

/// R17: does `st` (or anything nested in it) let SIMULATION TIME ADVANCE?
///
/// Only used for the shared-flattened-net rule (see [`da_stmt`]); a fresh net has no
/// other writer, so suspending is irrelevant to it. RECURSIVE, and checked before the
/// `stmt_no_ref` fast path, because a suspending statement that never mentions the
/// local (`begin #1 y = 3; end`) hands the scheduler over just the same.
///
/// Covers the forms the pre-R17 walk rejected by falling to its catch-all — delay /
/// event / wait control, in prefix or wrapper position — AND a user task CALL whose
/// body reaches one of them, via `call_suspends`.
///
/// R18-X1: the call arm closes a SILENT-WRONG that predates R17 (measured identical
/// at `c8ad2b4` and `46b9816`). Wrapping the suspend in a one-line helper —
/// `task automatic tick(); @(posedge clk); endtask` — hid it from a rule that read
/// only syntax, so a block that wrote its local, called the helper, and read the
/// local back observed a sibling block's write to the shared net instead: vita
/// printed `A v=99` where iverilog prints `A v=1`, at exit 0. R17's own comment here
/// argued the call case could be left alone because the pre-R17 walk accepted a
/// provably-inert callee; that reasoning was about the REFERENCE question, and
/// suspending is a different one — an inert callee still hands the scheduler over.
///
/// POLARITY: this feeds a REJECT decision (`!sole && advances_time` ⇒ loud), so every
/// unknown must answer `true`. `call_suspends` returns `true` for an unresolvable
/// callee, a hierarchical path, and an exhausted depth budget.
fn stmt_advances_time(st: &ast::Stmt, call_suspends: CallSuspends<'_>) -> bool {
    use ast::Stmt::*;
    let sub = |s: &ast::Stmt| stmt_advances_time(s, call_suspends);
    let opt = |s: &Option<Box<ast::Stmt>>| {
        s.as_deref()
            .is_some_and(|x| stmt_advances_time(x, call_suspends))
    };
    match st {
        DelayCtrl { .. } | EventCtrl { .. } | Wait { .. } | WaitFork { .. } => true,
        Blocking { delay, event, .. } | NonBlocking { delay, event, .. } => {
            delay.is_some() || event.is_some()
        }
        // IEEE 1800 §13.4.4 forbids a FUNCTION from consuming time, so only a task
        // enable can suspend — and a task enable is always this statement form.
        UserTaskCall { name, .. } => call_suspends(name),
        Block { stmts, .. } | Fork { stmts, .. } => stmts.iter().any(sub),
        If { then_s, else_s, .. } => sub(then_s) || opt(else_s),
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => sub(body),
        }),
        For {
            init, step, body, ..
        } => sub(init) || sub(step) || sub(body),
        While { body, .. } | Repeat { body, .. } | Forever { body, .. } => sub(body),
        DeferredAssert { then_s, else_s, .. } => sub(then_s) || sub(else_s),
        ConcurrentAssert { pass, fail, .. } => opt(pass) || opt(fail),
        _ => false,
    }
}

/// Walk a callee body for the suspend question — the time-side twin of
/// [`reads::stmt_no_ref_deep`]. Public so the resolver in `block_local::gate` can
/// recurse through it with its own depth budget.
pub(crate) fn body_advances_time(body: &ast::Stmt, call_suspends: CallSuspends<'_>) -> bool {
    stmt_advances_time(body, call_suspends)
}

/// GAP-D definite-assignment (round-4, guard-aware). Returns the assigned-state
/// AFTER `st` for the automatic local `name`, or `None` if `st` (or a
/// sub-statement) READS `name` on a path where it is not yet definitely written
/// THIS execution — the only case where the v1 static flattening (persist the
/// last value) diverges from `automatic` (fresh each block entry). Every form
/// not explicitly proven safe collapses to `None` (loud), never a silent accept:
/// a statement provably free of any `name` reference (`stmt_no_ref`) is a no-op;
/// a clean whole-var write establishes assignment; control flow recurses and
/// merges (an `if`/`case` assigns only if EVERY arm does; a loop cannot newly
/// guarantee assignment, it may run zero times); anything else that touches
/// `name` is conservatively unsafe. BL4: `out_writes` recognizes a call whose
/// OUTPUT actual is `name` as a definite ASSIGNMENT when the call is
/// unconditionally evaluated (a bare call statement, or the whole cond / scrutinee
/// of an `if`/`while`/`case`, incl. a `!`/paren wrapper).
pub(crate) fn da_stmt(st: &ast::Stmt, assigned: bool, name: &str, ctx: &DaCtx) -> DaResult {
    use ast::Stmt as S;
    // R17: does a statement that lets TIME ADVANCE break the flatten here? Only when
    // the net is SHARED with a same-named block-local in another block (`sole` false):
    // suspending hands the scheduler to that other block, which writes the one net, so
    // this block's later read observes a value its own `automatic` storage never would.
    // With a fresh net there is no other writer and suspending changes nothing.
    //
    // This is what the pre-R17 walk enforced by ACCIDENT — every timing form fell to
    // the catch-all — and the accident was load-bearing: modelling those forms without
    // this rule turned an `initial begin int k; #1; k=2; … end` sharing a generate-scope
    // `k` from loud into a silent `2` where iverilog prints `1`.
    //
    // R18-X1 CORRECTION: R17 also wrote here that "a timing statement reached AFTER the
    // local is definitely assigned was accepted before and still is", and treated that
    // as safe. It is not. Being written does not make the net OURS — the other block
    // writes the same one net, and the suspend is exactly the moment it gets to. See
    // the entry loop, which used to return early on `assigned` and so never reached
    // this guard at all.
    if !ctx.sole && stmt_advances_time(st, ctx.suspends) {
        return Err(DaGiveUp::at(st.span(), SHARED_SUSPEND));
    }
    // R17 §3.3: the give-up for "this expression reads `name` and nothing has written
    // it yet on this path" — reported at the EXPRESSION's span, which is the token the
    // user must look at (the statement span would point at the whole `if`/`while`).
    let read =
        |e: &ast::Expr| DaGiveUp::at(e.span, "it is read here before any write on this path");
    // R16 §3.1: a `disable` — whether the parser's `break`/`continue` desugaring or a
    // user one — names a BLOCK, never a variable, so it can neither read nor write
    // `name`. Checked ahead of the `stmt_no_ref` fast path because that path answers
    // only the reference question and would flatten the control-flow answer to
    // `Falls`. A loop jump is the precise `Jumps`; anything else conservatively falls
    // through (sound whether or not it actually transfers control — see
    // [`is_loop_jump`]).
    if let S::Disable { target, .. } = st {
        return Ok(if is_loop_jump(target) {
            DaOut::Jumps
        } else {
            DaOut::Falls(assigned)
        });
    }
    // A statement with no reference to `name` at all can neither read nor assign
    // it — the assigned-state passes through unchanged.
    if stmt_no_ref(st, name) {
        return Ok(DaOut::Falls(assigned));
    }
    match st {
        S::Blocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        } => {
            // R17 §3.3: a TIMING-CONTROLLED blocking assign — `#1 name = e;`,
            // `@(posedge clk) name = e;`, `name = #1 e;`, `name = @(ev) e;` — is still a
            // blocking assign. The process does not continue until the write has
            // happened, so by the time any later statement runs, `name` is written
            // exactly as in the un-delayed form. Only the PREFIX expressions are extra,
            // and they are reads evaluated before the write.
            //
            // These fell to the catch-all and rejected the block, which made the very
            // common testbench idiom "first write is clock- or delay-aligned" loud for
            // no reason. The un-timed form was already handled; the arms differed only
            // in that the timed one was never written.
            if !assigned {
                if let Some(d) = delay {
                    if let Some(e) = d
                        .values
                        .iter()
                        .find(|e| expr_reads(e, name, ctx.out_writes))
                    {
                        return Err(read(e));
                    }
                }
                if let Some(ev) = event {
                    if !intra_event_no_ref(ev, name) {
                        return Err(DaGiveUp::at(
                            st.span(),
                            "the event control on this assignment reads it before any \
                             write on this path",
                        ));
                    }
                }
            }
            // RHS is evaluated first: reading `name` here while unassigned is unsafe.
            if !assigned && expr_reads(rhs, name, ctx.out_writes) {
                return Err(read(rhs));
            }
            // R19 §3.1: …and by the same token, an output-actual call in the rhs has
            // ALREADY written `name` by the time the assignment lands — `go = nxt(5, r);`
            // establishes `r`, which is the single most common shape in the report (33 of
            // 34 diagnostics). This is not a new claim about calls, it is BL4's claim
            // applied where the call actually is: BL4 recognized the write only in
            // statement position, so it saw `fill(5, r);` and missed `go = nxt(5, r);`.
            //
            // Claimed only when the LVALUE cannot itself reference `name` — a `name[i] =`
            // base or an `other[name] =` index is a read whose order against the rhs is
            // not fixed, and a whole-var `name = …` lvalue is handled right below anyway.
            let assigned = assigned
                || (lvalue_no_ref(lhs, name) && expr_out_writes(rhs, name, ctx.out_writes));
            // A clean WHOLE-var write (`name = …`) makes it definitely assigned.
            if let ast::Lvalue::Ident(p) = lhs {
                if p.segments.len() == 1 && p.segments[0].name == name {
                    return Ok(DaOut::Falls(true));
                }
            }
            // Otherwise the lvalue references `name` only through a select base
            // (`name[i] = …`, a read-modify-write of the unwritten bits) or an
            // index (`other[name] = …`, a read of `name`). Either is unsafe while
            // unassigned; when already assigned it is a safe read that does not
            // downgrade the state. (A fully ref-free lvalue+rhs took the
            // `stmt_no_ref` fast path, so reaching here means one side references
            // `name`.)
            if !assigned {
                return Err(DaGiveUp::at(
                    st.span(),
                    "this assignment writes only PART of it (a select), so the rest is \
                     still unwritten",
                ));
            }
            Ok(DaOut::Falls(assigned))
        }
        S::If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            // BL4: a whole-cond output-actual call (`if (!setmode(3, m)) …`) is
            // evaluated unconditionally and WRITES `name` before either branch — not a
            // read-before-write. `after` is then true entering both branches AND after
            // the `if` (the cond ran on every path).
            let after = assigned || expr_out_writes(cond, name, ctx.out_writes);
            if !after && expr_reads(cond, name, ctx.out_writes) {
                return Err(read(cond));
            }
            // R19 §3.1: a branch is entered only for a particular VALUE of the cond, and
            // that value can imply an operand ran which `after` cannot claim: `a && f(r)`
            // is true only when BOTH ran, `a || f(r)` false only when both ran. So the
            // taken branch may know `name` is written where the join does not.
            let in_then = after || expr_writes_when(cond, name, ctx.out_writes, true);
            let in_else = after || expr_writes_when(cond, name, ctx.out_writes, false);
            let a_then = da_stmt(then_s, in_then, name, ctx)?;
            // R16 §3.1: an absent `else` is an empty arm that FALLS THROUGH with the
            // entry state — it is not a jump, so it always participates in the merge.
            let a_else = match else_s {
                Some(e) => da_stmt(e, in_else, name, ctx)?,
                // The join state is `after`, NOT `in_else`: an absent `else` reaches the
                // join, and what the join may assume is what EVERY path establishes.
                None => DaOut::Falls(after),
            };
            Ok(a_then.merge(a_else))
        }
        S::Block { stmts, decls, .. } => {
            // A nested block-local decl whose initializer reads `name` observes
            // the entry value too (mirror the sibling-init gate at the top level).
            for dd in decls {
                for nn in &dd.names {
                    if let Some(e) = &nn.init {
                        if !assigned && expr_reads(e, name, ctx.out_writes) {
                            return Err(read(e));
                        }
                    }
                }
            }
            let mut a = assigned;
            for s in stmts {
                match da_stmt(s, a, name, ctx)? {
                    DaOut::Falls(next) => a = next,
                    // R16 §3.1: the rest of this block is UNREACHABLE — a jump has
                    // already left it. Stop rather than keep walking statements that
                    // cannot execute (walking them is what turned a `continue`-first
                    // loop body loud).
                    DaOut::Jumps => return Ok(DaOut::Jumps),
                }
            }
            Ok(DaOut::Falls(a))
        }
        S::Fork { .. } => {
            // Fork branches run CONCURRENTLY — a write in one branch does not
            // provably precede a read in another (racy order), so sequential
            // threading (as for `Block`) would be unsound. A fork is safe only if
            // `name` is ALREADY definitely assigned BEFORE it (every branch read
            // then sees a current-execution value regardless of interleaving); a
            // write inside the fork cannot establish assignment. Reaching here
            // means the fork DOES reference `name` (else `stmt_no_ref` fast-pathed
            // it), so an unassigned fork is conservatively loud.
            if assigned {
                Ok(DaOut::Falls(true))
            } else {
                Err(DaGiveUp::at(
                    st.span(),
                    "a `fork` references it, and concurrent arms give no order in which \
                     a write precedes a read",
                ))
            }
        }
        S::Case {
            scrutinee, items, ..
        } => {
            // BL4: the scrutinee is evaluated unconditionally, so a whole-scrutinee
            // output-actual call WRITES `name` before the arms and after the case.
            let after = assigned || expr_out_writes(scrutinee, name, ctx.out_writes);
            if !after && expr_reads(scrutinee, name, ctx.out_writes) {
                return Err(read(scrutinee));
            }
            // R16 §3.1: arms merge with the same rule as `if` — an arm that jumps
            // (`case (x) 2: continue; …`) never reaches the join and drops out.
            let mut all: Option<DaOut> = None;
            let mut has_default = false;
            for it in items {
                let body = match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        if !after {
                            if let Some(l) =
                                labels.iter().find(|l| expr_reads(l, name, ctx.out_writes))
                            {
                                return Err(read(l));
                            }
                        }
                        body
                    }
                    ast::CaseItem::Default { body, .. } => {
                        has_default = true;
                        body
                    }
                };
                let arm = da_stmt(body, after, name, ctx)?;
                all = Some(match all {
                    Some(acc) => acc.merge(arm),
                    None => arm,
                });
            }
            // Definitely assigned after the case if the scrutinee wrote `name`, or if
            // EVERY arm assigns AND a default exists (else a scrutinee value can match
            // no arm → skip all). Without a default the no-match path falls through
            // with the entry state, so the case as a whole always falls through then.
            match all {
                Some(m) if has_default && !after => Ok(m),
                _ => Ok(DaOut::Falls(after)),
            }
        }
        S::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            let a0 = da_stmt(init, assigned, name, ctx)?.state();
            let a0 = a0 || expr_out_writes(cond, name, ctx.out_writes);
            if !a0 && expr_reads(cond, name, ctx.out_writes) {
                return Err(read(cond));
            }
            // The FIRST iteration enters with `a0` (the binding read-before-write
            // case; the loop may also run zero times). Body / step are checked
            // against `a0` — conservative, since a later iteration only ever has
            // MORE assigned than the first.
            // R19 §3.1: the body and the step run only when the cond was TRUE (see the
            // `If` arm) — the `.rsp`-walker idiom `for (…; n < lim && next(fd, r); …)`.
            let in_body = a0 || expr_writes_when(cond, name, ctx.out_writes, true);
            let body_out = da_stmt(body, in_body, name, ctx)?;
            da_stmt(step, in_body, name, ctx)?;
            // A loop always FALLS THROUGH for this analysis — a `break` inside its body
            // targets THIS loop, so it lands right here rather than skipping what follows.
            //
            // R20 §3.3: it CAN newly guarantee assignment, when the trip count proves the
            // body ran and nothing could jump past the write — `for (int j = 0; j < 3;
            // j++) fill(cur);` writes `cur` before the `cur.size()` after the loop.
            Ok(DaOut::Falls(
                a0 || loop_body_assigns(st, body, body_out, ctx.loop_once),
            ))
        }
        S::While { cond, body, .. } => {
            // BL4: a `while` cond is evaluated at least once (unconditionally) — a
            // whole-cond output-actual call there WRITES `name` before the body and
            // after the loop.
            let after = assigned || expr_out_writes(cond, name, ctx.out_writes);
            if !after && expr_reads(cond, name, ctx.out_writes) {
                return Err(read(cond));
            }
            // R19 §3.1: the BODY runs only when the cond is TRUE, and `a && next(fd, r)`
            // is true only when the call ran. This is the report's real site — the
            // table-driven `.rsp` walker, the standard shape for CAVP/Monte vectors:
            //   `while (n < sweep_limit && rsp_next(fd, r) == 1) begin … r … end`
            // The loop EXIT keeps `after`, because a FALSE cond may have short-circuited
            // the call away.
            let in_body = after || expr_writes_when(cond, name, ctx.out_writes, true);
            let body_out = da_stmt(body, in_body, name, ctx)?;
            Ok(DaOut::Falls(
                after || loop_body_assigns(st, body, body_out, ctx.loop_once),
            ))
        }
        S::Repeat { count, body, .. } => {
            // The count is evaluated exactly once, before the first iteration.
            let assigned = assigned || expr_out_writes(count, name, ctx.out_writes);
            if !assigned && expr_reads(count, name, ctx.out_writes) {
                return Err(read(count));
            }
            let body_out = da_stmt(body, assigned, name, ctx)?;
            Ok(DaOut::Falls(
                assigned || loop_body_assigns(st, body, body_out, ctx.loop_once),
            ))
        }
        S::Forever { body, .. } => {
            let body_out = da_stmt(body, assigned, name, ctx)?;
            // Conservatively falls through even though a `forever` without a `break`
            // never does — over-stating reachability only ever costs precision here.
            Ok(DaOut::Falls(
                assigned || loop_body_assigns(st, body, body_out, ctx.loop_once),
            ))
        }
        S::Return { value, .. } => {
            if let Some(v) = value {
                if !assigned && expr_reads(v, name, ctx.out_writes) {
                    return Err(read(v));
                }
            }
            // R16 §3.1: a `return` leaves the subroutine — nothing after it in this
            // block executes, so it drops out of any enclosing join.
            Ok(DaOut::Jumps)
        }
        // BL4: a bare call STATEMENT `f(…, name, …);` (a task or void-function call —
        // both parse to `UserTaskCall`) is evaluated unconditionally. When `name` is at
        // a PURE OUTPUT actual position (copy-out only, no copy-in) and at NO read
        // position of the SAME call (`out_writes`), the call definitely ASSIGNS `name`.
        // The guard is deliberate: a UserTaskCall NOT matching `out_writes` (an input /
        // inout / index reference to `name`, or an unresolvable callee) falls through to
        // the `_ => None` below — BYTE-IDENTICAL to the prior behavior for every
        // non-output-actual call (this arm only ever turns a genuine output-write from
        // loud into supported, never the reverse).
        S::UserTaskCall { name: cn, args, .. } => match (ctx.out_writes)(cn, args, name) {
            CallEffect::Writes => Ok(DaOut::Falls(true)),
            // R16 §3.2: a call proven to touch `name` at no position leaves the
            // assigned-state exactly as it was, and the walk continues past it. This
            // is what makes `show(0); a = 1; … a …` legal instead of a diagnostic
            // pointing at `a`.
            CallEffect::Inert => Ok(DaOut::Falls(assigned)),
            CallEffect::Reads | CallEffect::Unknown if assigned => Ok(DaOut::Falls(true)),
            // R17 §3.2: a proven-read-only call is still a READ of an unwritten local
            // here. It is reported as one (not as an unresolvable call) because that is
            // what it is, and because the never-written rule below can forgive it.
            CallEffect::Reads => Err(DaGiveUp::at(
                st.span(),
                "this call reads it (an `input` actual) before any write on this path",
            )),
            CallEffect::Unknown => Err(DaGiveUp::at(
                st.span(),
                "this call could not be proven to leave it alone (an unresolved callee, \
                 a hierarchical or named-argument call, or a body that may reach the \
                 flattened net)",
            )),
        },
        // R17 §3.3: a NON-blocking assign `name <= e;`. Its write lands in the NBA
        // region, AFTER every later blocking statement in this process, so it does NOT
        // make `name` definitely assigned for a read that follows in the same time step
        // — that read would still observe the entry value, and on the flatten that is
        // the leftover. But it is also not a READ of `name` when the rhs is ref-free,
        // so the honest answer is "state unchanged", not "give up": a later real write
        // still establishes assignment, and any read before it is judged exactly as it
        // was. (Reaching here means the lvalue is rooted at `name` or the rhs
        // references it — the ref-free case took the `stmt_no_ref` fast path.)
        S::NonBlocking { rhs, .. } => {
            // A non-blocking write lands in the NBA region, so on a SHARED net another
            // block scheduled in between sees it — the same hazard as suspending, and
            // the same thing the pre-R17 catch-all rejected here.
            if !ctx.sole {
                return Err(DaGiveUp::at(st.span(), SHARED_SUSPEND));
            }
            if !assigned && expr_reads(rhs, name, ctx.out_writes) {
                return Err(read(rhs));
            }
            // R19 §3.1: the rhs of a NON-blocking assign is evaluated in the ACTIVE
            // region, right here — only the target UPDATE is deferred to the NBA region.
            // So an output-actual call in it has written `name` by the next statement,
            // exactly as in the blocking form.
            Ok(DaOut::Falls(
                assigned || expr_out_writes(rhs, name, ctx.out_writes),
            ))
        }
        // R17 §3.3: a timing / event / wait WRAPPER around a statement. The prefix is a
        // read; the body then runs with the same state, and control falls through to the
        // next statement with whatever the body established. `Wait`'s body may be absent
        // (`wait (c);`).
        // R19 §3.1: these two use `expr_reads` for the same reason every other arm does
        // — a mention of `name` at an output actual is not a read. No WRITE is claimed
        // from them: a delay / wait expression is not a position the lowering emits a
        // copy-out from, so a call there is loud anyway and claiming the write would be
        // an accept gate running ahead of what can execute.
        S::DelayCtrl { delay, body, .. } => {
            if !assigned {
                if let Some(e) = delay
                    .values
                    .iter()
                    .find(|e| expr_reads(e, name, ctx.out_writes))
                {
                    return Err(read(e));
                }
            }
            match body {
                Some(b) => da_stmt(b, assigned, name, ctx),
                None => Ok(DaOut::Falls(assigned)),
            }
        }
        S::EventCtrl { ctrl, body, .. } => {
            if !assigned && !sensitivity_no_ref(ctrl, name) {
                return Err(DaGiveUp::at(
                    st.span(),
                    "this event control reads it before any write on this path",
                ));
            }
            match body {
                Some(b) => da_stmt(b, assigned, name, ctx),
                None => Ok(DaOut::Falls(assigned)),
            }
        }
        S::Wait { cond, body, .. } => {
            if !assigned && expr_reads(cond, name, ctx.out_writes) {
                return Err(read(cond));
            }
            match body {
                Some(b) => da_stmt(b, assigned, name, ctx),
                None => Ok(DaOut::Falls(assigned)),
            }
        }
        // R17 §3.3: once `name` is DEFINITELY ASSIGNED on this path, no later
        // statement can make the flatten diverge — the per-entry reset it would have
        // observed has already been overwritten this execution, so every later read
        // sees a current-execution value and every later write keeps it that way.
        // There is no operation that un-assigns it. This is the same rule the
        // top-level scan already applied between statements (`if assigned { return
        // true }`) and the `Fork` arm applied locally; NOT applying it inside a
        // nested construct is what made a write followed by an unmodelled statement
        // — `x = 5; #1 x = x + 1;` inside a `while` body — loud, and it is why the
        // reporter measured that a dummy write BEFORE the loop clears the diagnostic
        // while the same write INSIDE the loop does not (their in-situ facts 2 and 3).
        // `Falls(true)` is never weaker than `Jumps` at a join, so over-stating
        // reachability for a statement that might transfer control costs nothing.
        _ if assigned => Ok(DaOut::Falls(true)),
        // Any other statement that references `name` (timing, `assign`/`force`,
        // event control, disable, non-blocking, a task call with a referencing
        // arg, SVA, …) is not vetted for read-before-write → conservatively unsafe.
        _ => Err(DaGiveUp::at(
            st.span(),
            "this statement form is not modelled by the walk, and it references the \
             local before any write on this path",
        )),
    }
}

/// GAP-D soundness (see `hoist_block_local_nets`). v1 flattens a PROCEDURAL
/// block-local to ONE static module net (no per-block-entry frame). An
/// `automatic` block-local is byte-identical to that flattening iff it is
/// DEFINITELY ASSIGNED before every read on every path THIS execution (its
/// per-entry reset is then unobservable). Scans the block's statements: the
/// moment `name` is definitely assigned at the top level, every later read is
/// safe (accept); a read reached while it may still be unwritten is a loud
/// reject (`da_stmt` returns `None`). A never-read local is trivially equivalent.
/// The guard-aware `da_stmt` accepts a write that dominates the read inside a
/// shared conditional / loop (`if (c) begin x = …; … x … end`, the round-4
/// prefix-builder shape) which the earlier top-level-only scan rejected. Still
/// conservative: iverilog cannot oracle block-local `automatic`, so any un-vetted
/// form stays loud rather than silently given static semantics.
/// R16 §3.3: the constant index of a straight-line whole-ELEMENT write
/// `name[<literal>] = rhs`, when `rhs` cannot reference `name`. `None` for anything
/// else — a computed index, a bit-select inside the element, a delayed or
/// event-controlled assign, or an rhs that may read the array.
/// R18 §3.2: the constant BIT SPAN a straight-line select-write covers —
/// `name[<lit>:<lit>] = rhs` (inclusive, in either bound order) or
/// `name[<lit>] = rhs` — when `rhs` cannot reference `name`. `None` for anything
/// else, with the same conservatism as [`const_elem_write`].
///
/// Why bits and not members: a struct is PARSER-desugared (`s.c` becomes a constant
/// part-select into one flat vector, see `hdl-parser::typedefs`), so by the time the
/// definite-assignment walk runs there are no members left to count — only bits. That
/// makes the rule both simpler and more general than member counting: a single-member
/// struct is the case the reporter hit (`rm.c = 5;` writes ALL of `rm`, yet the walk
/// called it partial), and a two-member struct written field by field falls out for
/// free. It also covers a hand-written `x[31:16] = a; x[15:0] = b;`.
fn const_bit_span_write(st: &ast::Stmt, name: &str) -> Option<(u32, u32)> {
    let ast::Stmt::Blocking {
        lhs,
        delay: None,
        event: None,
        rhs,
        ..
    } = st
    else {
        return None;
    };
    if !expr_no_ref(rhs, name) {
        return None;
    }
    let base_is_name = |b: &ast::Lvalue| {
        matches!(b, ast::Lvalue::Ident(p)
            if p.segments.len() == 1 && p.segments[0].name == name)
    };
    match lhs {
        ast::Lvalue::PartSelect { base, msb, lsb, .. } if base_is_name(base) => {
            let (a, b) = (const_index(msb)?, const_index(lsb)?);
            let (a, b) = (u32::try_from(a).ok()?, u32::try_from(b).ok()?);
            Some((a.min(b), a.max(b)))
        }
        ast::Lvalue::BitSelect { base, index, .. } if base_is_name(base) => {
            let i = u32::try_from(const_index(index)?).ok()?;
            Some((i, i))
        }
        _ => None,
    }
}

/// A plain decimal literal index, unwrapping parens. Anything else (a parameter, a
/// folded expression) needs the elaborator's scope, and answering "unknown" here only
/// costs precision. Shared by [`const_elem_write`] and [`const_bit_span_write`].
fn const_index(e: &ast::Expr) -> Option<i64> {
    let mut e = e;
    while let ast::ExprKind::Paren { inner } = &e.kind {
        e = inner;
    }
    match &e.kind {
        ast::ExprKind::IntLit {
            kind: ast::IntLitKind::Decimal,
            raw,
        } => raw.replace('_', "").parse::<i64>().ok(),
        _ => None,
    }
}

fn const_elem_write(st: &ast::Stmt, name: &str) -> Option<i64> {
    let ast::Stmt::Blocking {
        lhs,
        delay: None,
        event: None,
        rhs,
        ..
    } = st
    else {
        return None;
    };
    if !expr_no_ref(rhs, name) {
        return None;
    }
    let ast::Lvalue::BitSelect { base, index, .. } = lhs else {
        return None;
    };
    let ast::Lvalue::Ident(p) = &**base else {
        return None;
    };
    if p.segments.len() != 1 || p.segments[0].name != name {
        return None;
    }
    const_index(index)
}

pub(crate) fn automatic_local_definitely_assigned(
    stmts: &[ast::Stmt],
    name: &str,
    ctx: &DaCtx,
    elem_bounds: Option<(i64, i64)>,
    bit_width: Option<u32>,
) -> Result<(), DaGiveUp> {
    // R17 §3.2: a local that is NEVER WRITTEN anywhere in the block is byte-identical
    // to per-entry storage without any path reasoning at all. `automatic` gives a
    // fresh default each entry; the flattened static net is default-initialized once
    // and — with no write reaching it — still holds that same default at every entry.
    // The two are equal because neither ever changes.
    //
    // This is the whole of the report's §3.2: `automatic byte exp[];` deliberately
    // left empty and passed as an INPUT actual is not a "read before its first write",
    // it is a variable with no first write to be before. IEEE 1800 §7.5 makes an
    // unassigned dynamic array size 0, and passing it by value is legal.
    //
    // The writer-detector is the conservative one used elsewhere (`_`-free over every
    // statement form; any doubt answers "may be written"), given the SAME call
    // resolver the walk uses so that a pure `input` actual is not miscounted as a
    // possible copy-out. Checked FIRST: a local with no writer needs no path
    // reasoning, and no give-up below can be anything but spurious for it.
    //
    // SCOPE of the claim, and why `ctx.sole` exists: the walk sees only THIS
    // block's statements, so "never written" is a statement about this block alone.
    // That is the whole truth for a local with a fresh net, but NOT for the
    // same-name coalesce gate, where a block-local in ANOTHER block shares the
    // flattened net and its write is the leftover being read here — the exact
    // silent-wrong that gate exists to stop. Its call site passes `false`; the
    // fresh-net gates pass `true`. (Writes from outside the declaring block are a
    // third case, rejected independently by `check_block_local_scope_leaks`.)
    if ctx.sole && stmt_never_writes_ident(stmts, name, Some(ctx.out_writes)) {
        return Ok(());
    }
    let mut assigned = false;
    // R16 §3.3: indices of `name` written by straight-line constant-index element
    // writes so far. A fixed unpacked array has no whole-value write until every
    // element has one, so `automatic int a[4]; a[0]=…; a[1]=…; a[2]=…; a[3]=…;` was
    // rejected as read-before-write even though nothing was read at all — which, with
    // the decl-init case, left the type essentially unusable.
    //
    // Coverage is only ever claimed where it is PROVEN: literal indices, at the top
    // level of the block (so unconditional), with an rhs that cannot read the array.
    // A computed or `foreach` index is not counted, and the local stays loud — the
    // alternative, resetting the array at each block entry, was measured WRONG
    // (iverilog keeps the leftover across loop re-entries; storage is per activation).
    let mut covered: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    // R18 §3.2: bit indices written by straight-line constant select writes. Bounded
    // by `MAX_COVERED_BITS` at the caller, so this set can never be pathological.
    let mut covered_bits: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    for st in stmts {
        if assigned {
            // A FRESH net has no other writer, so once this block has written it every
            // later read observes a current-execution value — safe regardless of the
            // statement form, and the walk is done.
            if ctx.sole {
                return Ok(());
            }
            // R18-X1: a SHARED net is not done. Being written does not make the net
            // ours; the same one net is written by a same-named block-local in another
            // block, and a suspend is exactly when that block gets to run. Returning
            // here is what let the silent-wrong through — the guard inside `da_stmt`
            // was never reached once the local was assigned, so `v = 1; tick(); read v`
            // printed the OTHER block's 99 where iverilog prints 1, at exit 0.
            //
            // Keep walking instead: a statement that cannot advance time preserves the
            // claim, one that can ends it.
            if stmt_advances_time(st, ctx.suspends) {
                return Err(DaGiveUp::at(st.span(), SHARED_SUSPEND));
            }
            continue;
        }
        if let Some((lo, hi)) = elem_bounds {
            if let Some(ix) = const_elem_write(st, name) {
                if (lo..=hi).contains(&ix) {
                    covered.insert(ix);
                    if i64::try_from(covered.len()).is_ok_and(|n| n == hi - lo + 1) {
                        assigned = true;
                    }
                    continue;
                }
            }
        }
        // R18 §3.2: the same coverage argument one level down, in BITS. A struct
        // member write is a constant part-select after the parser's desugar, so
        // `rm.c = 5;` on a single-member struct writes every bit of `rm` — yet the
        // walk called it "only PART" and treated the local as still unwritten. Field
        // by field (`rm.a = …; rm.b = …;`) reaches full coverage the same way.
        if let Some(w) = bit_width {
            if let Some((lo, hi)) = const_bit_span_write(st, name) {
                if hi < w {
                    for b in lo..=hi {
                        covered_bits.insert(b);
                    }
                    if covered_bits.len() as u32 == w {
                        assigned = true;
                    }
                    continue;
                }
            }
        }
        match da_stmt(st, false, name, ctx) {
            Ok(DaOut::Falls(a)) => assigned = a,
            // R16 §3.1: control left the block here (a top-level `break`/`continue`/
            // `return`), so every remaining statement is unreachable and cannot
            // contain a read.
            Ok(DaOut::Jumps) => return Ok(()),
            Err(g) => return Err(g),
        }
    }
    Ok(())
}
