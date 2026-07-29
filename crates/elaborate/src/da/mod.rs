//! definite-assignment / ident-read AST queries — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

mod reads;
mod writes;

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

/// BL4: true when the WHOLE expression `e` — after unwrapping any unary operator
/// (`!`/`~`/`-`) and `(paren)` — is a function CALL that WRITES `name` through an
/// OUTPUT actual with no read of `name` (per `out_writes`). Such a cond / scrutinee
/// is evaluated UNCONDITIONALLY (a unary operator and a paren always evaluate their
/// operand), so the write establishes `name` before the branch. A `&&`/`||` (Binary)
/// or `?:` (Ternary) is deliberately NOT unwrapped — the call inside it may be
/// short-circuited / not taken, so it cannot be counted as a definite write.
fn cond_out_writes(e: &ast::Expr, name: &str, out_writes: OutActualWrites) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Paren { inner } => cond_out_writes(inner, name, out_writes),
        K::Unary { operand, .. } => cond_out_writes(operand, name, out_writes),
        K::Call { name: cn, args } => out_writes(cn, args, name) == CallEffect::Writes,
        _ => false,
    }
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
/// Covers exactly the forms the pre-R17 walk rejected by falling to its catch-all:
/// delay / event / wait control, in prefix or wrapper position. A user task CALL can
/// also suspend, but the pre-R17 walk accepted a provably-inert one and this rule is
/// not the place to take that back.
fn stmt_advances_time(st: &ast::Stmt) -> bool {
    use ast::Stmt::*;
    let sub = |s: &ast::Stmt| stmt_advances_time(s);
    let opt = |s: &Option<Box<ast::Stmt>>| s.as_deref().is_some_and(stmt_advances_time);
    match st {
        DelayCtrl { .. } | EventCtrl { .. } | Wait { .. } | WaitFork { .. } => true,
        Blocking { delay, event, .. } | NonBlocking { delay, event, .. } => {
            delay.is_some() || event.is_some()
        }
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
        ConcurrentAssert { pass, fail, .. } => {
            pass.as_deref().is_some_and(stmt_advances_time)
                || fail.as_deref().is_some_and(stmt_advances_time)
        }
        _ => false,
    }
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
pub(crate) fn da_stmt(
    st: &ast::Stmt,
    assigned: bool,
    name: &str,
    out_writes: OutActualWrites,
    sole: bool,
) -> DaResult {
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
    // `k` from loud into a silent `2` where iverilog prints `1`. Positional, exactly
    // like the accident it replaces: a timing statement reached AFTER the local is
    // definitely assigned was accepted before and still is.
    const SHARED_SUSPEND: &str = "time can advance here, and the flattened net is shared \
         with a same-named block-local in another block — that block can write it before \
         this one reads it again";
    if !sole && stmt_advances_time(st) {
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
                    if let Some(e) = d.values.iter().find(|e| !expr_no_ref(e, name)) {
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
            if !assigned && !expr_no_ref(rhs, name) {
                return Err(read(rhs));
            }
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
            let after = assigned || cond_out_writes(cond, name, out_writes);
            if !after && !expr_no_ref(cond, name) {
                return Err(read(cond));
            }
            let a_then = da_stmt(then_s, after, name, out_writes, sole)?;
            // R16 §3.1: an absent `else` is an empty arm that FALLS THROUGH with the
            // entry state — it is not a jump, so it always participates in the merge.
            let a_else = match else_s {
                Some(e) => da_stmt(e, after, name, out_writes, sole)?,
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
                        if !assigned && !expr_no_ref(e, name) {
                            return Err(read(e));
                        }
                    }
                }
            }
            let mut a = assigned;
            for s in stmts {
                match da_stmt(s, a, name, out_writes, sole)? {
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
            let after = assigned || cond_out_writes(scrutinee, name, out_writes);
            if !after && !expr_no_ref(scrutinee, name) {
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
                            if let Some(l) = labels.iter().find(|l| !expr_no_ref(l, name)) {
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
                let arm = da_stmt(body, after, name, out_writes, sole)?;
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
            let a0 = da_stmt(init, assigned, name, out_writes, sole)?.state();
            if !a0 && !expr_no_ref(cond, name) {
                return Err(read(cond));
            }
            // The FIRST iteration enters with `a0` (the binding read-before-write
            // case; the loop may also run zero times). Body / step are checked
            // against `a0` — conservative, since a later iteration only ever has
            // MORE assigned than the first.
            da_stmt(body, a0, name, out_writes, sole)?;
            da_stmt(step, a0, name, out_writes, sole)?;
            // A loop cannot newly guarantee assignment (may run 0 times), and it always
            // FALLS THROUGH for this analysis — a `break` inside its body targets THIS
            // loop, so it lands right here rather than skipping what follows.
            Ok(DaOut::Falls(a0))
        }
        S::While { cond, body, .. } => {
            // BL4: a `while` cond is evaluated at least once (unconditionally) — a
            // whole-cond output-actual call there WRITES `name` before the body and
            // after the loop.
            let after = assigned || cond_out_writes(cond, name, out_writes);
            if !after && !expr_no_ref(cond, name) {
                return Err(read(cond));
            }
            da_stmt(body, after, name, out_writes, sole)?;
            Ok(DaOut::Falls(after))
        }
        S::Repeat { count, body, .. } => {
            if !assigned && !expr_no_ref(count, name) {
                return Err(read(count));
            }
            da_stmt(body, assigned, name, out_writes, sole)?;
            Ok(DaOut::Falls(assigned))
        }
        S::Forever { body, .. } => {
            da_stmt(body, assigned, name, out_writes, sole)?;
            // Conservatively falls through even though a `forever` without a `break`
            // never does — over-stating reachability only ever costs precision here.
            Ok(DaOut::Falls(assigned))
        }
        S::Return { value, .. } => {
            if let Some(v) = value {
                if !assigned && !expr_no_ref(v, name) {
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
        S::UserTaskCall { name: cn, args, .. } => match out_writes(cn, args, name) {
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
            if !sole {
                return Err(DaGiveUp::at(st.span(), SHARED_SUSPEND));
            }
            if !assigned && !expr_no_ref(rhs, name) {
                return Err(read(rhs));
            }
            Ok(DaOut::Falls(assigned))
        }
        // R17 §3.3: a timing / event / wait WRAPPER around a statement. The prefix is a
        // read; the body then runs with the same state, and control falls through to the
        // next statement with whatever the body established. `Wait`'s body may be absent
        // (`wait (c);`).
        S::DelayCtrl { delay, body, .. } => {
            if !assigned {
                if let Some(e) = delay.values.iter().find(|e| !expr_no_ref(e, name)) {
                    return Err(read(e));
                }
            }
            match body {
                Some(b) => da_stmt(b, assigned, name, out_writes, sole),
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
                Some(b) => da_stmt(b, assigned, name, out_writes, sole),
                None => Ok(DaOut::Falls(assigned)),
            }
        }
        S::Wait { cond, body, .. } => {
            if !assigned && !expr_no_ref(cond, name) {
                return Err(read(cond));
            }
            match body {
                Some(b) => da_stmt(b, assigned, name, out_writes, sole),
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
    // Only a plain decimal literal. A parameter or a folded expression would need
    // the elaborator's scope, and answering "unknown" here only costs precision.
    let mut e: &ast::Expr = index;
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

pub(crate) fn automatic_local_definitely_assigned(
    stmts: &[ast::Stmt],
    name: &str,
    out_writes: OutActualWrites,
    elem_bounds: Option<(i64, i64)>,
    sole_writer: bool,
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
    // SCOPE of the claim, and why `sole_writer` exists: the walk sees only THIS
    // block's statements, so "never written" is a statement about this block alone.
    // That is the whole truth for a local with a fresh net, but NOT for the
    // same-name coalesce gate, where a block-local in ANOTHER block shares the
    // flattened net and its write is the leftover being read here — the exact
    // silent-wrong that gate exists to stop. Its call site passes `false`; the
    // fresh-net gates pass `true`. (Writes from outside the declaring block are a
    // third case, rejected independently by `check_block_local_scope_leaks`.)
    if sole_writer && stmt_never_writes_ident(stmts, name, Some(out_writes)) {
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
    for st in stmts {
        // Once definitely assigned at the top level, every later read observes a
        // current-execution value → safe regardless of the statement form.
        if assigned {
            return Ok(());
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
        match da_stmt(st, false, name, out_writes, sole_writer) {
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
