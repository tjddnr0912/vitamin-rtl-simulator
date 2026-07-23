//! definite-assignment / ident-read AST queries — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

pub(crate) fn expr_reads_ident(e: &ast::Expr, name: &str) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Ident(p) => p.segments.len() == 1 && p.segments[0].name == name,
        K::Unary { operand, .. } => expr_reads_ident(operand, name),
        K::Binary { lhs, rhs, .. } => expr_reads_ident(lhs, name) || expr_reads_ident(rhs, name),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_reads_ident(cond, name)
                || expr_reads_ident(then_e, name)
                || expr_reads_ident(else_e, name)
        }
        K::BitSelect { base, index } => {
            expr_reads_ident(base, name) || expr_reads_ident(index, name)
        }
        K::PartSelect { base, msb, lsb } => {
            expr_reads_ident(base, name)
                || expr_reads_ident(msb, name)
                || expr_reads_ident(lsb, name)
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            expr_reads_ident(base, name)
                || expr_reads_ident(offset, name)
                || expr_reads_ident(width, name)
        }
        K::Concat { parts } => parts.iter().any(|x| expr_reads_ident(x, name)),
        K::Replicate { count, value } => {
            expr_reads_ident(count, name) || value.iter().any(|x| expr_reads_ident(x, name))
        }
        K::Call { args, .. } | K::SysCall { args, .. } => {
            args.iter().any(|x| expr_reads_ident(x, name))
        }
        K::Paren { inner } => expr_reads_ident(inner, name),
        K::MinTypMax { min, typ, max } => {
            expr_reads_ident(min, name)
                || expr_reads_ident(typ, name)
                || expr_reads_ident(max, name)
        }
        // A cast wraps an operand that may read the ident (`int'(a)` / `signed'(a)`);
        // a SIZE cast's width expr can too (`a'(5)`). Missing this let a tf-port
        // default like `int b = int'(a)` bypass the formal-reference guard.
        K::Cast { target, expr } => {
            expr_reads_ident(expr, name)
                || matches!(target, ast::CastTarget::Size(s) if expr_reads_ident(s, name))
        }
        // `'{s[0], …}` assignment-pattern elements and `new[s[0]]` (dynamic-array
        // size / copy source) are rvalue reads. These leaf kinds were unhandled in
        // BOTH this walker and the scope-leak walker that shares it — a shared blind
        // spot invisible to a walker-vs-walker audit — so a coalesced-string read
        // via `'{…}` / `new[…]` was silently reading the wrong net's bits.
        K::AssignPattern(parts) => parts.iter().any(|x| expr_reads_ident(x, name)),
        K::New { size, src } => {
            expr_reads_ident(size, name) || src.as_ref().is_some_and(|s| expr_reads_ident(s, name))
        }
        _ => false,
    }
}

/// R5-B: the single root net name of an lvalue-shaped expression (a bare Ident or a
/// select on one) — used to name the variable an inout ACTUAL mutates. `None` for a
/// concat / literal / arbitrary expression.
pub(crate) fn expr_root_ident(e: &ast::Expr) -> Option<String> {
    use ast::ExprKind as K;
    match &e.kind {
        K::Ident(p) if p.segments.len() == 1 => Some(p.segments[0].name.clone()),
        K::BitSelect { base, .. } | K::PartSelect { base, .. } | K::IndexedPart { base, .. } => {
            expr_root_ident(base)
        }
        K::Paren { inner } => expr_root_ident(inner),
        _ => None,
    }
}

/// R2: the single root net name of an lvalue (a bare Ident or a select on one).
pub(crate) fn lval_root_name(lhs: &ast::Lvalue) -> Option<String> {
    match lhs {
        ast::Lvalue::Ident(p) if p.segments.len() == 1 => Some(p.segments[0].name.clone()),
        ast::Lvalue::BitSelect { base, .. }
        | ast::Lvalue::PartSelect { base, .. }
        | ast::Lvalue::IndexedPart { base, .. } => lval_root_name(base),
        _ => None,
    }
}

/// Like [`expr_reads_ident`] but arg-direction-aware at a system-func boundary, for
/// the block-local coalesce read-gate ONLY. A string-building / scanf / file-read
/// sysfunc's write DEST args are NOT reads (see [`syscall_read_args`]) — e.g. the
/// idiomatic rvalue `n = $sscanf(src, fmt, s)` WRITES `s`. The shared
/// `expr_reads_ident` cannot encode this: the scope-leak walker also uses it and
/// must keep treating a dest as a reference. Gates the sysfunc boundary (and a
/// `(paren)` / nested read-arg sysfunc), then defers to `expr_reads_ident` for
/// ordinary sub-exprs. A sysfunc nested under an arithmetic wrapper is not re-gated,
/// but vita supports these funcs only as a bare assignment RHS, so such forms are
/// already loud-rejected upstream (never a silent over-reject of a valid program).
/// `new(s[0])` (class-ctor args) is a read too, gated here (read-gate-only) rather
/// than in the shared `expr_reads_ident` — a ctor arg is always a read so this cannot
/// over-reject, and keeping it out of the shared walker avoids perturbing the
/// scope-leak walker's OOP handling.
pub(crate) fn rvalue_reads_ident(e: &ast::Expr, name: &str) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::SysCall { name: task, args } => syscall_read_args(&task.name, args)
            .iter()
            .any(|a| rvalue_reads_ident(a, name)),
        K::ClassNew { args } => args.iter().any(|a| rvalue_reads_ident(a, name)),
        K::Paren { inner } => rvalue_reads_ident(inner, name),
        _ => expr_reads_ident(e, name),
    }
}

/// Does a `#(…)` delay-control expression read `name`? The delay prefix of
/// `#(s[0]) stmt` / intra-assignment `= #(s) rhs` is a read the coalesce read-gate
/// must see; the reference walker drops it via `..`.
pub(crate) fn delay_reads_ident(d: &ast::Delay, name: &str) -> bool {
    d.values.iter().any(|e| rvalue_reads_ident(e, name))
}

/// Does an `@(…)` sensitivity list read `name`? (`@(s) stmt`; `@(*)` reads nothing.)
pub(crate) fn sensitivity_reads_ident(s: &ast::Sensitivity, name: &str) -> bool {
    match s {
        ast::Sensitivity::Star => false,
        ast::Sensitivity::List(evs) => evs.iter().any(|ev| rvalue_reads_ident(&ev.expr, name)),
    }
}

/// Does an intra-assignment event control `= @(ev) rhs` / `= repeat(n) @(ev) rhs`
/// read `name`? Both the `repeat` count and the event expressions are reads.
pub(crate) fn intra_event_reads_ident(e: &ast::IntraEvent, name: &str) -> bool {
    e.repeat
        .as_ref()
        .is_some_and(|r| rvalue_reads_ident(r, name))
        || sensitivity_reads_ident(&e.ctrl, name)
}

/// Does the lvalue (a write target) reference `name` — either as the written
/// base identifier or inside a select index? Mirrors [`expr_reads_ident`] for the
/// write side (block-local scope-leak detection).
pub(crate) fn lvalue_refs_ident(lv: &ast::Lvalue, name: &str) -> bool {
    use ast::Lvalue as L;
    match lv {
        L::Ident(p) => p.segments.len() == 1 && p.segments[0].name == name,
        L::BitSelect { base, index, .. } => {
            lvalue_refs_ident(base, name) || expr_reads_ident(index, name)
        }
        L::PartSelect { base, msb, lsb, .. } => {
            lvalue_refs_ident(base, name)
                || expr_reads_ident(msb, name)
                || expr_reads_ident(lsb, name)
        }
        L::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            lvalue_refs_ident(base, name)
                || expr_reads_ident(offset, name)
                || expr_reads_ident(width, name)
        }
        L::Concat { parts, .. } => parts.iter().any(|p| lvalue_refs_ident(p, name)),
        L::Error(_) => false,
    }
}

/// Does any `begin…end`/`fork` block nested anywhere within `s` have span
/// `target`? Used to tell a TRUE SIBLING of the declaring block (does not contain
/// it) from an ANCESTOR (does) during scope-leak detection.
pub(crate) fn stmt_contains_block_span(s: &ast::Stmt, target: ast::Span) -> bool {
    use ast::Stmt::*;
    match s {
        Block { stmts, span, .. } | Fork { stmts, span, .. } => {
            *span == target || stmts.iter().any(|st| stmt_contains_block_span(st, target))
        }
        If { then_s, else_s, .. } => {
            stmt_contains_block_span(then_s, target)
                || else_s
                    .as_ref()
                    .is_some_and(|e| stmt_contains_block_span(e, target))
        }
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_contains_block_span(body, target)
            }
        }),
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            stmt_contains_block_span(body, target)
        }
        Wait { body, .. } | DelayCtrl { body, .. } | EventCtrl { body, .. } => body
            .as_ref()
            .is_some_and(|b| stmt_contains_block_span(b, target)),
        _ => false,
    }
}

/// Is `name` referenced (read in any expression, or written as a target) anywhere
/// in statement `s` EXCEPT inside the block whose span equals `skip`? Used to
/// detect a block-local that is referenced outside its declaring block — vita's
/// flat per-function local table would silently resolve such a reference to the
/// block-local instead of the lexically-correct outer binding (IEEE §6.21 scope).
/// Conservative: an unhandled statement form simply returns `false` (under-
/// detection is safe — it only means the loud diagnostic is not raised).
pub(crate) fn stmt_refs_ident_outside(s: &ast::Stmt, skip: ast::Span, name: &str) -> bool {
    use ast::Stmt::*;
    let decl_inits = |decls: &[ast::NetVarDecl]| {
        decls.iter().any(|d| {
            d.names
                .iter()
                .any(|n| n.init.as_ref().is_some_and(|e| expr_reads_ident(e, name)))
        })
    };
    match s {
        Block {
            stmts, decls, span, ..
        }
        | Fork {
            stmts, decls, span, ..
        } => {
            if *span == skip {
                return false; // the declaring block itself — references here are in-scope
            }
            // If THIS block ALSO declares `name`, references to `name` inside it
            // bind to its own local — BUT only when it is a TRUE SIBLING of the
            // declaring block (does not contain `skip`). If it is an ANCESTOR of
            // `skip`, references here that are outside `skip` still hit the
            // coalesced flat slot and are a genuine hazard, so we must recurse
            // (the recursion returns false once it reaches `skip` itself).
            if decls
                .iter()
                .flat_map(|d| d.names.iter())
                .any(|n| n.name.name == name)
                && !stmts.iter().any(|st| stmt_contains_block_span(st, skip))
            {
                return false;
            }
            decl_inits(decls)
                || stmts
                    .iter()
                    .any(|st| stmt_refs_ident_outside(st, skip, name))
        }
        Blocking { lhs, rhs, .. } | NonBlocking { lhs, rhs, .. } => {
            lvalue_refs_ident(lhs, name) || expr_reads_ident(rhs, name)
        }
        If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            expr_reads_ident(cond, name)
                || stmt_refs_ident_outside(then_s, skip, name)
                || else_s
                    .as_ref()
                    .is_some_and(|e| stmt_refs_ident_outside(e, skip, name))
        }
        Return { value, .. } => value.as_ref().is_some_and(|e| expr_reads_ident(e, name)),
        Case {
            scrutinee, items, ..
        } => {
            expr_reads_ident(scrutinee, name)
                || items.iter().any(|it| match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        labels.iter().any(|e| expr_reads_ident(e, name))
                            || stmt_refs_ident_outside(body, skip, name)
                    }
                    ast::CaseItem::Default { body, .. } => {
                        stmt_refs_ident_outside(body, skip, name)
                    }
                })
        }
        For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            stmt_refs_ident_outside(init, skip, name)
                || expr_reads_ident(cond, name)
                || stmt_refs_ident_outside(step, skip, name)
                || stmt_refs_ident_outside(body, skip, name)
        }
        While { cond, body, .. } => {
            expr_reads_ident(cond, name) || stmt_refs_ident_outside(body, skip, name)
        }
        Repeat { count, body, .. } => {
            expr_reads_ident(count, name) || stmt_refs_ident_outside(body, skip, name)
        }
        Forever { body, .. } => stmt_refs_ident_outside(body, skip, name),
        Wait { cond, body, .. } => {
            expr_reads_ident(cond, name)
                || body
                    .as_ref()
                    .is_some_and(|b| stmt_refs_ident_outside(b, skip, name))
        }
        DelayCtrl { body, .. } | EventCtrl { body, .. } => body
            .as_ref()
            .is_some_and(|b| stmt_refs_ident_outside(b, skip, name)),
        SysTaskCall { args, .. } | UserTaskCall { args, .. } | RandomizeWith { args, .. } => {
            args.iter().any(|e| expr_reads_ident(e, name))
        }
        _ => false,
    }
}

/// True if any statement in `s` READS `name` in an rvalue position — an assignment
/// / `assign` / `force` RHS, an intra-assignment `#(s)` / `@(s)` control, a
/// block-local initializer, a condition / scrutinee / loop bound, a `return` value, an
/// lvalue-index sub-expr, a `#(s)` delay / `@(s)` event-control prefix, `new(s)` ctor
/// args, or a task/sysfunc READ arg (a string-building / scanf sysfunc's write DEST
/// arg is NOT a read, per [`syscall_read_args`]). A plain write `name = <expr>` does
/// NOT count. READ-GATES the module block-local dynamic-storage coalesce reject: a
/// WRITE-ONLY string local coalesces harmlessly (its truncated write is discarded onto
/// the sibling's scalar net), so only a READ of the wrong-kind coalesced net is
/// silent-wrong. Covers `stmt_refs_ident_outside`'s rvalue positions minus the
/// lvalue-write side, PLUS the timing-control prefix fields that reference walker drops
/// via `..` (`DelayCtrl.delay` / `EventCtrl.ctrl` / intra-assignment `event`) and
/// `new(…)` ctor args. An uncovered form returns `false` — the only remaining read
/// positions that lack coverage lack a live iverilog oracle: SVA properties / `assert
/// #0` (iverilog "sorry"), CRV `RandomizeWith.constraints` / `Dist` / `ArrayMethodWith`
/// (no oracle), and impossible forms (literals, multi-segment paths). Never an
/// over-reject of a valid write-only program.
/// CONSERVATIVE "expression `e` certainly does NOT reference `name`". Unlike the
/// shared `expr_reads_ident` — whose `_ => false` UNDER-detects (it misses a read
/// hidden in `ArrayMethodWith` / `ClassNew` / `RandomizeWith` / `Dist` /
/// `PkgScoped`) — this returns `false` for ANY form it does not fully vet, so the
/// GAP-D accept below can never be fooled by a walker blind spot. Only enumerated
/// leaf / composite forms with EVERY sub-expression provably ref-free return
/// `true`. (Sound-by-construction: unknown ⇒ "may reference".)
pub(crate) fn expr_no_ref(e: &ast::Expr, name: &str) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::Null | K::Dollar => true,
        // The FIRST segment matching `name` is a reference — a bare read (`t`), a
        // field / hierarchical read (`t.field`, a multi-seg ident for a class
        // handle), all count. Only a path whose HEAD is some other name (`other.x`)
        // is ref-free.
        K::Ident(p) => p.segments.first().is_none_or(|s| s.name != name),
        K::Unary { operand, .. } => expr_no_ref(operand, name),
        K::Binary { lhs, rhs, .. } => expr_no_ref(lhs, name) && expr_no_ref(rhs, name),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => expr_no_ref(cond, name) && expr_no_ref(then_e, name) && expr_no_ref(else_e, name),
        K::BitSelect { base, index } => expr_no_ref(base, name) && expr_no_ref(index, name),
        K::PartSelect { base, msb, lsb } => {
            expr_no_ref(base, name) && expr_no_ref(msb, name) && expr_no_ref(lsb, name)
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => expr_no_ref(base, name) && expr_no_ref(offset, name) && expr_no_ref(width, name),
        K::Concat { parts } => parts.iter().all(|x| expr_no_ref(x, name)),
        K::Replicate { count, value } => {
            expr_no_ref(count, name) && value.iter().all(|x| expr_no_ref(x, name))
        }
        // A call's NAME head can be the receiver of a method call (`t.size()`,
        // `t.method(a)`) — a read of `name`. A plain `f(args)` head is some other
        // function. So the head must not be `name`, AND every arg must be ref-free.
        K::Call { name: cn, args } => {
            cn.segments.first().is_none_or(|s| s.name != name)
                && args.iter().all(|x| expr_no_ref(x, name))
        }
        K::SysCall { args, .. } => args.iter().all(|x| expr_no_ref(x, name)),
        K::Paren { inner } => expr_no_ref(inner, name),
        K::MinTypMax { min, typ, max } => {
            expr_no_ref(min, name) && expr_no_ref(typ, name) && expr_no_ref(max, name)
        }
        K::Cast { target, expr } => {
            expr_no_ref(expr, name)
                && match target {
                    ast::CastTarget::Size(s) => expr_no_ref(s, name),
                    _ => true,
                }
        }
        K::AssignPattern(parts) => parts.iter().all(|x| expr_no_ref(x, name)),
        K::New { size, src } => {
            expr_no_ref(size, name) && src.as_ref().is_none_or(|s| expr_no_ref(s, name))
        }
        // PkgScoped / ClassNew / ArrayMethodWith / RandomizeWith / Dist / Error —
        // not vetted → conservatively "may reference".
        _ => false,
    }
}

/// CONSERVATIVE lvalue counterpart of [`expr_no_ref`] — the lvalue (write target
/// and any select index) certainly does NOT reference `name`. Uses `expr_no_ref`
/// for index sub-exprs (so a read hidden in an index is not a blind spot).
pub(crate) fn lvalue_no_ref(lv: &ast::Lvalue, name: &str) -> bool {
    use ast::Lvalue as L;
    match lv {
        // As in `expr_no_ref`: any path headed by `name` references it (a whole
        // write `name`, a field/hier write `name.f`). Only another head is ref-free.
        L::Ident(p) => p.segments.first().is_none_or(|s| s.name != name),
        L::BitSelect { base, index, .. } => lvalue_no_ref(base, name) && expr_no_ref(index, name),
        L::PartSelect { base, msb, lsb, .. } => {
            lvalue_no_ref(base, name) && expr_no_ref(msb, name) && expr_no_ref(lsb, name)
        }
        L::IndexedPart {
            base,
            offset,
            width,
            ..
        } => lvalue_no_ref(base, name) && expr_no_ref(offset, name) && expr_no_ref(width, name),
        L::Concat { parts, .. } => parts.iter().all(|p| lvalue_no_ref(p, name)),
        L::Error(_) => true,
    }
}

/// CONSERVATIVE "statement `st` certainly does NOT reference `name`" (read OR
/// write, anywhere). Returns `false` for any statement form not fully vetted —
/// only a simple blocking / non-blocking assign (NO intra-assign timing) with a
/// ref-free lvalue+rhs, a system-task call with ref-free args, or a null
/// statement is provably ref-free. Control flow / timing / `assign` / `force` /
/// method-with all conservatively block the accept.
pub(crate) fn stmt_no_ref(st: &ast::Stmt, name: &str) -> bool {
    use ast::Stmt as S;
    match st {
        S::Null(_) => true,
        S::Blocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        }
        | S::NonBlocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        } => {
            delay.is_none() && event.is_none() && lvalue_no_ref(lhs, name) && expr_no_ref(rhs, name)
        }
        S::SysTaskCall { args, .. } => args.iter().all(|a| expr_no_ref(a, name)),
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
/// `name` is conservatively unsafe.
pub(crate) fn da_stmt(st: &ast::Stmt, assigned: bool, name: &str) -> Option<bool> {
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
            if !assigned && !expr_no_ref(cond, name) {
                return None;
            }
            let a_then = da_stmt(then_s, assigned, name)?;
            let a_else = match else_s {
                Some(e) => da_stmt(e, assigned, name)?,
                None => assigned,
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
                a = da_stmt(s, a, name)?;
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
            if !assigned && !expr_no_ref(scrutinee, name) {
                return None;
            }
            let mut all = true;
            let mut has_default = false;
            for it in items {
                let body = match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        if !assigned && labels.iter().any(|l| !expr_no_ref(l, name)) {
                            return None;
                        }
                        body
                    }
                    ast::CaseItem::Default { body, .. } => {
                        has_default = true;
                        body
                    }
                };
                all = da_stmt(body, assigned, name)? && all;
            }
            // Definitely assigned after the case only if EVERY arm assigns AND a
            // default exists (else a scrutinee value can match no arm → skip all).
            Some(assigned || (has_default && all))
        }
        S::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            let a0 = da_stmt(init, assigned, name)?;
            if !a0 && !expr_no_ref(cond, name) {
                return None;
            }
            // The FIRST iteration enters with `a0` (the binding read-before-write
            // case; the loop may also run zero times). Body / step are checked
            // against `a0` — conservative, since a later iteration only ever has
            // MORE assigned than the first.
            da_stmt(body, a0, name)?;
            da_stmt(step, a0, name)?;
            Some(a0) // a loop cannot newly guarantee assignment (may run 0 times)
        }
        S::While { cond, body, .. } => {
            if !assigned && !expr_no_ref(cond, name) {
                return None;
            }
            da_stmt(body, assigned, name)?;
            Some(assigned)
        }
        S::Repeat { count, body, .. } => {
            if !assigned && !expr_no_ref(count, name) {
                return None;
            }
            da_stmt(body, assigned, name)?;
            Some(assigned)
        }
        S::Forever { body, .. } => {
            da_stmt(body, assigned, name)?;
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
pub(crate) fn automatic_local_definitely_assigned(stmts: &[ast::Stmt], name: &str) -> bool {
    let mut assigned = false;
    for st in stmts {
        // Once definitely assigned at the top level, every later read observes a
        // current-execution value → safe regardless of the statement form.
        if assigned {
            return true;
        }
        match da_stmt(st, false, name) {
            Some(a) => assigned = a,
            None => return false,
        }
    }
    true
}

pub(crate) fn stmt_reads_ident(s: &ast::Stmt, name: &str) -> bool {
    use ast::Stmt::*;
    match s {
        Block { stmts, decls, .. } | Fork { stmts, decls, .. } => {
            stmts.iter().any(|st| stmt_reads_ident(st, name))
                // A sibling/nested block-local's OWN initializer (`int x = s[0];`) is
                // a read even though it is a decl, not a statement — mirror the
                // reference walker's `decl_inits` so a decl-init read is not missed.
                || decls.iter().flat_map(|d| d.names.iter()).any(|n| {
                    n.init.as_ref().is_some_and(|e| rvalue_reads_ident(e, name))
                })
        }
        // Blocking / nonblocking assignments read their RHS and the lvalue INDEX
        // sub-exprs (`mem[s] = …`; the written base ident is not a read), AND any
        // intra-assignment `#(s)` delay / `@(s)` event control (`= #(s) rhs` /
        // `= @(s) rhs`) — a read the plain lhs/rhs walk drops via `..`.
        Blocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        }
        | NonBlocking {
            lhs,
            delay,
            event,
            rhs,
            ..
        } => {
            lvalue_index_reads_ident(lhs, name)
                || rvalue_reads_ident(rhs, name)
                || delay.as_ref().is_some_and(|d| delay_reads_ident(d, name))
                || event
                    .as_ref()
                    .is_some_and(|e| intra_event_reads_ident(e, name))
        }
        // Procedural-continuous `assign` / `force` read their RHS (+ lvalue index).
        Assign { lhs, rhs, .. } | Force { lhs, rhs, .. } => {
            lvalue_index_reads_ident(lhs, name) || rvalue_reads_ident(rhs, name)
        }
        If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            rvalue_reads_ident(cond, name)
                || stmt_reads_ident(then_s, name)
                || else_s.as_ref().is_some_and(|e| stmt_reads_ident(e, name))
        }
        Return { value, .. } => value.as_ref().is_some_and(|e| rvalue_reads_ident(e, name)),
        Case {
            scrutinee, items, ..
        } => {
            rvalue_reads_ident(scrutinee, name)
                || items.iter().any(|it| match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        labels.iter().any(|e| rvalue_reads_ident(e, name))
                            || stmt_reads_ident(body, name)
                    }
                    ast::CaseItem::Default { body, .. } => stmt_reads_ident(body, name),
                })
        }
        For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            stmt_reads_ident(init, name)
                || rvalue_reads_ident(cond, name)
                || stmt_reads_ident(step, name)
                || stmt_reads_ident(body, name)
        }
        While { cond, body, .. } => rvalue_reads_ident(cond, name) || stmt_reads_ident(body, name),
        Repeat { count, body, .. } => {
            rvalue_reads_ident(count, name) || stmt_reads_ident(body, name)
        }
        Forever { body, .. } => stmt_reads_ident(body, name),
        Wait { cond, body, .. } => {
            rvalue_reads_ident(cond, name)
                || body.as_ref().is_some_and(|b| stmt_reads_ident(b, name))
        }
        // `#(s[0]) stmt` — the delay expr is a read the plain body-walk drops via `..`.
        DelayCtrl { delay, body, .. } => {
            delay_reads_ident(delay, name)
                || body.as_ref().is_some_and(|b| stmt_reads_ident(b, name))
        }
        // `@(s) stmt` — the event/sensitivity is a read (vita has a dedicated
        // `@(string)` reject that the coalesce-to-packed-net would silently bypass).
        EventCtrl { ctrl, body, .. } => {
            sensitivity_reads_ident(ctrl, name)
                || body.as_ref().is_some_and(|b| stmt_reads_ident(b, name))
        }
        SysTaskCall {
            name: task, args, ..
        } => syscall_read_args(task.name.as_str(), args)
            .iter()
            .any(|e| rvalue_reads_ident(e, name)),
        UserTaskCall { args, .. } | RandomizeWith { args, .. } => {
            args.iter().any(|e| rvalue_reads_ident(e, name))
        }
        _ => false,
    }
}

/// True if an lvalue's INDEX sub-expressions (`mem[<idx>] = …`, `r[<msb>:<lsb>]`)
/// read `name`. The written BASE ident is NOT a read (a plain `s = …` write is not
/// a read of `s`). Mirrors `lvalue_refs_ident` minus the base-ident match.
pub(crate) fn lvalue_index_reads_ident(lv: &ast::Lvalue, name: &str) -> bool {
    use ast::Lvalue as L;
    match lv {
        L::Ident(_) => false,
        L::BitSelect { base, index, .. } => {
            lvalue_index_reads_ident(base, name) || rvalue_reads_ident(index, name)
        }
        L::PartSelect { base, msb, lsb, .. } => {
            lvalue_index_reads_ident(base, name)
                || rvalue_reads_ident(msb, name)
                || rvalue_reads_ident(lsb, name)
        }
        L::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            lvalue_index_reads_ident(base, name)
                || rvalue_reads_ident(offset, name)
                || rvalue_reads_ident(width, name)
        }
        L::Concat { parts, .. } => parts.iter().any(|p| lvalue_index_reads_ident(p, name)),
        L::Error(_) => false,
    }
}

/// v5 ⑥: VARIABLE kinds eligible as dynamic-storage ELEMENT types (the heap
/// stores 4-state `Value`s; real elements are deferred, nets are illegal).
/// The root `Ident` of an lvalue (through select bases). `None` for a concat
/// (its parts are checked individually) and parse-error recovery.
pub(crate) fn lval_root_path(lv: &ast::Lvalue) -> Option<&ast::HierPath> {
    match lv {
        ast::Lvalue::Ident(p) => Some(p),
        ast::Lvalue::BitSelect { base, .. }
        | ast::Lvalue::PartSelect { base, .. }
        | ast::Lvalue::IndexedPart { base, .. } => lval_root_path(base),
        _ => None,
    }
}
