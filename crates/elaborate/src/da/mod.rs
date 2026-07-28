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
) -> Option<DaOut> {
    use ast::Stmt as S;
    // R16 §3.1: a `disable` — whether the parser's `break`/`continue` desugaring or a
    // user one — names a BLOCK, never a variable, so it can neither read nor write
    // `name`. Checked ahead of the `stmt_no_ref` fast path because that path answers
    // only the reference question and would flatten the control-flow answer to
    // `Falls`. A loop jump is the precise `Jumps`; anything else conservatively falls
    // through (sound whether or not it actually transfers control — see
    // [`is_loop_jump`]).
    if let S::Disable { target, .. } = st {
        return Some(if is_loop_jump(target) {
            DaOut::Jumps
        } else {
            DaOut::Falls(assigned)
        });
    }
    // A statement with no reference to `name` at all can neither read nor assign
    // it — the assigned-state passes through unchanged.
    if stmt_no_ref(st, name) {
        return Some(DaOut::Falls(assigned));
    }
    match st {
        S::Blocking {
            lhs,
            delay: None,
            event: None,
            rhs,
            ..
        } => {
            // RHS is evaluated first: reading `name` here while unassigned is unsafe.
            if !assigned && !expr_no_ref(rhs, name) {
                return None;
            }
            // A clean WHOLE-var write (`name = …`) makes it definitely assigned.
            if let ast::Lvalue::Ident(p) = lhs {
                if p.segments.len() == 1 && p.segments[0].name == name {
                    return Some(DaOut::Falls(true));
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
                return None;
            }
            Some(DaOut::Falls(assigned))
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
                return None;
            }
            let a_then = da_stmt(then_s, after, name, out_writes)?;
            // R16 §3.1: an absent `else` is an empty arm that FALLS THROUGH with the
            // entry state — it is not a jump, so it always participates in the merge.
            let a_else = match else_s {
                Some(e) => da_stmt(e, after, name, out_writes)?,
                None => DaOut::Falls(after),
            };
            Some(a_then.merge(a_else))
        }
        S::Block { stmts, decls, .. } => {
            // A nested block-local decl whose initializer reads `name` observes
            // the entry value too (mirror the sibling-init gate at the top level).
            for dd in decls {
                for nn in &dd.names {
                    if let Some(e) = &nn.init {
                        if !assigned && !expr_no_ref(e, name) {
                            return None;
                        }
                    }
                }
            }
            let mut a = assigned;
            for s in stmts {
                match da_stmt(s, a, name, out_writes)? {
                    DaOut::Falls(next) => a = next,
                    // R16 §3.1: the rest of this block is UNREACHABLE — a jump has
                    // already left it. Stop rather than keep walking statements that
                    // cannot execute (walking them is what turned a `continue`-first
                    // loop body loud).
                    DaOut::Jumps => return Some(DaOut::Jumps),
                }
            }
            Some(DaOut::Falls(a))
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
                Some(DaOut::Falls(true))
            } else {
                None
            }
        }
        S::Case {
            scrutinee, items, ..
        } => {
            // BL4: the scrutinee is evaluated unconditionally, so a whole-scrutinee
            // output-actual call WRITES `name` before the arms and after the case.
            let after = assigned || cond_out_writes(scrutinee, name, out_writes);
            if !after && !expr_no_ref(scrutinee, name) {
                return None;
            }
            // R16 §3.1: arms merge with the same rule as `if` — an arm that jumps
            // (`case (x) 2: continue; …`) never reaches the join and drops out.
            let mut all: Option<DaOut> = None;
            let mut has_default = false;
            for it in items {
                let body = match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        if !after && labels.iter().any(|l| !expr_no_ref(l, name)) {
                            return None;
                        }
                        body
                    }
                    ast::CaseItem::Default { body, .. } => {
                        has_default = true;
                        body
                    }
                };
                let arm = da_stmt(body, after, name, out_writes)?;
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
                Some(m) if has_default && !after => Some(m),
                _ => Some(DaOut::Falls(after)),
            }
        }
        S::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            let a0 = da_stmt(init, assigned, name, out_writes)?.state();
            if !a0 && !expr_no_ref(cond, name) {
                return None;
            }
            // The FIRST iteration enters with `a0` (the binding read-before-write
            // case; the loop may also run zero times). Body / step are checked
            // against `a0` — conservative, since a later iteration only ever has
            // MORE assigned than the first.
            da_stmt(body, a0, name, out_writes)?;
            da_stmt(step, a0, name, out_writes)?;
            // A loop cannot newly guarantee assignment (may run 0 times), and it always
            // FALLS THROUGH for this analysis — a `break` inside its body targets THIS
            // loop, so it lands right here rather than skipping what follows.
            Some(DaOut::Falls(a0))
        }
        S::While { cond, body, .. } => {
            // BL4: a `while` cond is evaluated at least once (unconditionally) — a
            // whole-cond output-actual call there WRITES `name` before the body and
            // after the loop.
            let after = assigned || cond_out_writes(cond, name, out_writes);
            if !after && !expr_no_ref(cond, name) {
                return None;
            }
            da_stmt(body, after, name, out_writes)?;
            Some(DaOut::Falls(after))
        }
        S::Repeat { count, body, .. } => {
            if !assigned && !expr_no_ref(count, name) {
                return None;
            }
            da_stmt(body, assigned, name, out_writes)?;
            Some(DaOut::Falls(assigned))
        }
        S::Forever { body, .. } => {
            da_stmt(body, assigned, name, out_writes)?;
            // Conservatively falls through even though a `forever` without a `break`
            // never does — over-stating reachability only ever costs precision here.
            Some(DaOut::Falls(assigned))
        }
        S::Return { value, .. } => {
            if let Some(v) = value {
                if !assigned && !expr_no_ref(v, name) {
                    return None;
                }
            }
            // R16 §3.1: a `return` leaves the subroutine — nothing after it in this
            // block executes, so it drops out of any enclosing join.
            Some(DaOut::Jumps)
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
            CallEffect::Writes => Some(DaOut::Falls(true)),
            // R16 §3.2: a call proven to touch `name` at no position leaves the
            // assigned-state exactly as it was, and the walk continues past it. This
            // is what makes `show(0); a = 1; … a …` legal instead of a diagnostic
            // pointing at `a`.
            CallEffect::Inert => Some(DaOut::Falls(assigned)),
            CallEffect::Unknown => None,
        },
        // Any other statement that references `name` (timing, `assign`/`force`,
        // event control, disable, non-blocking, a task call with a referencing
        // arg, SVA, …) is not vetted for read-before-write → conservatively unsafe.
        _ => None,
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
pub(crate) fn automatic_local_definitely_assigned(
    stmts: &[ast::Stmt],
    name: &str,
    out_writes: OutActualWrites,
) -> bool {
    let mut assigned = false;
    for st in stmts {
        // Once definitely assigned at the top level, every later read observes a
        // current-execution value → safe regardless of the statement form.
        if assigned {
            return true;
        }
        match da_stmt(st, false, name, out_writes) {
            Some(DaOut::Falls(a)) => assigned = a,
            // R16 §3.1: control left the block here (a top-level `break`/`continue`/
            // `return`), so every remaining statement is unreachable and cannot
            // contain a read.
            Some(DaOut::Jumps) => return true,
            None => return false,
        }
    }
    true
}
