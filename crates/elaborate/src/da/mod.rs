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
pub(crate) type OutActualWrites<'a> = &'a dyn Fn(&ast::HierPath, &[ast::Expr], &str) -> bool;

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
        K::Call { name: cn, args } => out_writes(cn, args, name),
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
) -> Option<bool> {
    use ast::Stmt as S;
    // A statement with no reference to `name` at all can neither read nor assign
    // it — the assigned-state passes through unchanged.
    if stmt_no_ref(st, name) {
        return Some(assigned);
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
                    return Some(true);
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
            Some(assigned)
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
            let a_else = match else_s {
                Some(e) => da_stmt(e, after, name, out_writes)?,
                None => after,
            };
            Some(a_then && a_else)
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
                a = da_stmt(s, a, name, out_writes)?;
            }
            Some(a)
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
                Some(true)
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
            let mut all = true;
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
                all = da_stmt(body, after, name, out_writes)? && all;
            }
            // Definitely assigned after the case if the scrutinee wrote `name`, or if
            // EVERY arm assigns AND a default exists (else a scrutinee value can match
            // no arm → skip all).
            Some(after || (has_default && all))
        }
        S::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            let a0 = da_stmt(init, assigned, name, out_writes)?;
            if !a0 && !expr_no_ref(cond, name) {
                return None;
            }
            // The FIRST iteration enters with `a0` (the binding read-before-write
            // case; the loop may also run zero times). Body / step are checked
            // against `a0` — conservative, since a later iteration only ever has
            // MORE assigned than the first.
            da_stmt(body, a0, name, out_writes)?;
            da_stmt(step, a0, name, out_writes)?;
            Some(a0) // a loop cannot newly guarantee assignment (may run 0 times)
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
            Some(after)
        }
        S::Repeat { count, body, .. } => {
            if !assigned && !expr_no_ref(count, name) {
                return None;
            }
            da_stmt(body, assigned, name, out_writes)?;
            Some(assigned)
        }
        S::Forever { body, .. } => {
            da_stmt(body, assigned, name, out_writes)?;
            Some(assigned)
        }
        S::Return { value, .. } => {
            if let Some(v) = value {
                if !assigned && !expr_no_ref(v, name) {
                    return None;
                }
            }
            Some(assigned)
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
        S::UserTaskCall { name: cn, args, .. } if out_writes(cn, args, name) => Some(true),
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
            Some(a) => assigned = a,
            None => return false,
        }
    }
    true
}
