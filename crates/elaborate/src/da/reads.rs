//! definite-assignment support: the READ / reference-detection AST walkers.
//!
//! Split out of `da.rs` to hold it under the 1000-line module cap. These answer
//! "does this construct REFERENCE `name`?"; the write-side twins live in
//! `da/writes.rs`, and the DA fixpoint itself stays in `da/mod.rs`.

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
        // R17-X1: a call's PATH HEAD is the RECEIVER when the path has two or more
        // segments — `s.atoi()`, `q.size()`, `a.push_back(x)` all READ `s`/`q`/`a`.
        // Checking only the args missed every one of them, and the block-local
        // scope-leak detector is built on this walker: a block-local referenced
        // outside its block ONLY through a method call was not detected, the local
        // coalesced onto the outer binding's net, and the outside read returned the
        // block's value. Measured against iverilog (`B 1234` vs `B 9999`) — a
        // silent-wrong, not a missed diagnostic. A ONE-segment head is an ordinary
        // function name (`f(x)`), never a variable read; `pkg::f()` parses as
        // `PkgScoped`, so a multi-segment head here is always a dot-path.
        K::Call { name: cn, args } => {
            (cn.segments.len() >= 2 && cn.segments[0].name == name)
                || args.iter().any(|x| expr_reads_ident(x, name))
        }
        K::SysCall { args, .. } => args.iter().any(|x| expr_reads_ident(x, name)),
        // R17 §3.1: a method chained on a call RESULT (`s.substr(2,4).atoi()`). The
        // receiver is an expression, not a path, so the arm above cannot see it.
        K::MethodCall { recv, args, .. } => {
            expr_reads_ident(recv, name) || args.iter().any(|x| expr_reads_ident(x, name))
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
/// Returns the SPAN of the first such reference (R17 §4.1), or `None`. Conservative:
/// an unhandled statement form simply returns `None` (under-detection is safe — it
/// only means the loud diagnostic is not raised).
pub(crate) fn stmt_refs_ident_outside(
    s: &ast::Stmt,
    skip: ast::Span,
    name: &str,
) -> Option<ast::Span> {
    use ast::Stmt::*;
    // R17 §4.1: the walk now yields WHERE the outside reference is, not just that one
    // exists — the diagnostic points at the declaration, and the note has to point at
    // the reference that makes it illegal. `Option<Span>` short-circuits exactly like
    // the `bool` it replaces (`||` → `.or_else`, `.any` → `.find_map`).
    let e_ref = |e: &ast::Expr| expr_reads_ident(e, name).then_some(e.span);
    let lv_ref = |lv: &ast::Lvalue, sp: ast::Span| lvalue_refs_ident(lv, name).then_some(sp);
    let decl_inits = |decls: &[ast::NetVarDecl]| {
        decls
            .iter()
            .find_map(|d| d.names.iter().find_map(|n| n.init.as_ref().and_then(e_ref)))
    };
    match s {
        Block {
            stmts, decls, span, ..
        }
        | Fork {
            stmts, decls, span, ..
        } => {
            if *span == skip {
                return None; // the declaring block itself — references here are in-scope
            }
            // If THIS block ALSO declares `name`, references to `name` inside it
            // bind to its own local — BUT only when it is a TRUE SIBLING of the
            // declaring block (does not contain it). If it is an ANCESTOR of
            // `skip`, references here that are outside `skip` still hit the
            // coalesced flat slot and are a genuine hazard, so we must recurse
            // (the recursion returns None once it reaches `skip` itself).
            if decls
                .iter()
                .flat_map(|d| d.names.iter())
                .any(|n| n.name.name == name)
                && !stmts.iter().any(|st| stmt_contains_block_span(st, skip))
            {
                return None;
            }
            decl_inits(decls).or_else(|| {
                stmts
                    .iter()
                    .find_map(|st| stmt_refs_ident_outside(st, skip, name))
            })
        }
        Blocking { lhs, rhs, span, .. } | NonBlocking { lhs, rhs, span, .. } => {
            lv_ref(lhs, *span).or_else(|| e_ref(rhs))
        }
        If {
            cond,
            then_s,
            else_s,
            ..
        } => e_ref(cond)
            .or_else(|| stmt_refs_ident_outside(then_s, skip, name))
            .or_else(|| {
                else_s
                    .as_ref()
                    .and_then(|e| stmt_refs_ident_outside(e, skip, name))
            }),
        Return { value, .. } => value.as_ref().and_then(e_ref),
        Case {
            scrutinee, items, ..
        } => e_ref(scrutinee).or_else(|| {
            items.iter().find_map(|it| match it {
                ast::CaseItem::Match { labels, body, .. } => labels
                    .iter()
                    .find_map(&e_ref)
                    .or_else(|| stmt_refs_ident_outside(body, skip, name)),
                ast::CaseItem::Default { body, .. } => stmt_refs_ident_outside(body, skip, name),
            })
        }),
        For {
            init,
            cond,
            step,
            body,
            ..
        } => stmt_refs_ident_outside(init, skip, name)
            .or_else(|| e_ref(cond))
            .or_else(|| stmt_refs_ident_outside(step, skip, name))
            .or_else(|| stmt_refs_ident_outside(body, skip, name)),
        While { cond, body, .. } => {
            e_ref(cond).or_else(|| stmt_refs_ident_outside(body, skip, name))
        }
        Repeat { count, body, .. } => {
            e_ref(count).or_else(|| stmt_refs_ident_outside(body, skip, name))
        }
        Forever { body, .. } => stmt_refs_ident_outside(body, skip, name),
        Wait { cond, body, .. } => e_ref(cond).or_else(|| {
            body.as_ref()
                .and_then(|b| stmt_refs_ident_outside(b, skip, name))
        }),
        DelayCtrl { body, .. } | EventCtrl { body, .. } => body
            .as_ref()
            .and_then(|b| stmt_refs_ident_outside(b, skip, name)),
        SysTaskCall { args, .. } | UserTaskCall { args, .. } | RandomizeWith { args, .. } => {
            args.iter().find_map(&e_ref)
        }
        _ => None,
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
    expr_no_ref_with(e, name, false)
}

/// R16 §3.2: [`expr_no_ref`] with the path rule widened to EVERY segment, for
/// reasoning about a CALLEE's body rather than the caller's own statements.
///
/// The caller-side rule ("only the head segment counts") is right where it is used:
/// a local `a` is referenced by `a`, `a.f`, `a[i]`, and a path headed by anything
/// else is a different object. But v1 flattens a block-local into the MODULE
/// namespace, so a callee can also reach it through a hierarchical self-path —
/// `task poke; t.a = 99; endtask` writes the flattened `a` (measured, not assumed).
/// Whenever the question is "can this callee touch the flattened net", the head rule
/// under-detects and the all-segments rule is the sound one.
pub(crate) fn expr_no_ref_deep(e: &ast::Expr, name: &str) -> bool {
    expr_no_ref_with(e, name, true)
}

/// The shared walker behind [`expr_no_ref`] and [`expr_no_ref_deep`]. `deep` is an
/// OPT-IN parameter: passing the literal `false` short-circuits every widened test
/// back to the head-segment rule, so `expr_no_ref` is mechanically what it was.
fn expr_no_ref_with(e: &ast::Expr, name: &str, deep: bool) -> bool {
    use ast::ExprKind as K;
    // The FIRST segment matching `name` is a reference — a bare read (`t`), a
    // field / hierarchical read (`t.field`, a multi-seg ident for a class
    // handle), all count. Only a path whose HEAD is some other name (`other.x`)
    // is ref-free. Under `deep`, a match at ANY segment counts (see above).
    let path_ok = |p: &ast::HierPath| {
        if deep {
            p.segments.iter().all(|s| s.name != name)
        } else {
            p.segments.first().is_none_or(|s| s.name != name)
        }
    };
    let sub = |x: &ast::Expr| expr_no_ref_with(x, name, deep);
    match &e.kind {
        K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::Null | K::Dollar => true,
        K::Ident(p) => path_ok(p),
        K::Unary { operand, .. } => sub(operand),
        K::Binary { lhs, rhs, .. } => sub(lhs) && sub(rhs),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => sub(cond) && sub(then_e) && sub(else_e),
        K::BitSelect { base, index } => sub(base) && sub(index),
        K::PartSelect { base, msb, lsb } => sub(base) && sub(msb) && sub(lsb),
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => sub(base) && sub(offset) && sub(width),
        K::Concat { parts } => parts.iter().all(sub),
        K::Replicate { count, value } => sub(count) && value.iter().all(sub),
        // A call's NAME head can be the receiver of a method call (`t.size()`,
        // `t.method(a)`) — a read of `name`. A plain `f(args)` head is some other
        // function. So the head must not be `name`, AND every arg must be ref-free.
        K::Call { name: cn, args } => path_ok(cn) && args.iter().all(sub),
        // R17 §3.1: a method CHAINED on a call result — `line.substr(4,5).atoi()`.
        // The receiver is an expression (the inner call), not a path, so the `Call`
        // arm above never saw it and the chain fell to `_ => false` = "may reference
        // `name`". That answer is given for EVERY name, so one chain anywhere in a
        // block rejected the chain's own assignment target and every local declared
        // after it — 12 of the 34 diagnostics in the round-17 report, plus the
        // cross-file misattribution (§3.1b) when the chain sat in a callee body.
        // The `method` Ident is a name in the RECEIVER's namespace, never a reference
        // to the caller's `name`, so only `recv` and the args are vetted.
        K::MethodCall { recv, args, .. } => sub(recv) && args.iter().all(sub),
        K::SysCall { args, .. } => args.iter().all(sub),
        K::Paren { inner } => sub(inner),
        K::MinTypMax { min, typ, max } => sub(min) && sub(typ) && sub(max),
        K::Cast { target, expr } => {
            sub(expr)
                && match target {
                    ast::CastTarget::Size(s) => sub(s),
                    _ => true,
                }
        }
        K::AssignPattern(parts) => parts.iter().all(sub),
        // A named argument's FORMAL is a name in the CALLEE's namespace — never a
        // reference to the caller's `name`. Only its value can reference anything
        // here. Leaving this unvetted made `r = add(1, .b(2));` — a clean whole-var
        // write — read as "the rhs may reference `r`", so the definite-assignment
        // gate rejected the assignment that was staring right at it and blamed `r`.
        K::NamedArg { value, .. } => value.as_ref().is_none_or(|v| sub(v)),
        K::New { size, src } => sub(size) && src.as_ref().is_none_or(|s| sub(s)),
        // PkgScoped / ClassNew / ArrayMethodWith / RandomizeWith / Dist / Error —
        // not vetted → conservatively "may reference".
        _ => false,
    }
}

/// The POSITIVE twin of [`expr_no_ref`]: "expression `e` DEFINITELY references
/// `name`". Same enumerated forms, opposite default — an unvetted node answers
/// `false` ("no definite reference") instead of `true` ("may reference").
///
/// The two exist because polarity is a property of the GATE, not of the walker.
/// `expr_no_ref` feeds ACCEPT gates, where "unknown ⇒ may reference" is the safe
/// answer. A REJECT gate needs the opposite: with `expr_no_ref`'s polarity every
/// unvetted initializer becomes a rejection, and §4.5.250's review found exactly
/// that — `int idx = pkg::BASE;` beside any `automatic` sibling was rejected with a
/// message naming a variable it never mentions.
///
/// Under-detection here means a REJECT gate can miss a hazard and fall back to the
/// behavior it would have had anyway; over-detection would break working designs.
pub(crate) fn expr_definitely_refs(e: &ast::Expr, name: &str) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::Ident(p) => p.segments.first().is_some_and(|s| s.name == name),
        K::Unary { operand, .. } => expr_definitely_refs(operand, name),
        K::Binary { lhs, rhs, .. } => {
            expr_definitely_refs(lhs, name) || expr_definitely_refs(rhs, name)
        }
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_definitely_refs(cond, name)
                || expr_definitely_refs(then_e, name)
                || expr_definitely_refs(else_e, name)
        }
        K::BitSelect { base, index } => {
            expr_definitely_refs(base, name) || expr_definitely_refs(index, name)
        }
        K::PartSelect { base, msb, lsb } => {
            expr_definitely_refs(base, name)
                || expr_definitely_refs(msb, name)
                || expr_definitely_refs(lsb, name)
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            expr_definitely_refs(base, name)
                || expr_definitely_refs(offset, name)
                || expr_definitely_refs(width, name)
        }
        K::Concat { parts } | K::AssignPattern(parts) => {
            parts.iter().any(|x| expr_definitely_refs(x, name))
        }
        K::Replicate { count, value } => {
            expr_definitely_refs(count, name) || value.iter().any(|x| expr_definitely_refs(x, name))
        }
        K::Call { name: cn, args } => {
            cn.segments.first().is_some_and(|s| s.name == name)
                || args.iter().any(|x| expr_definitely_refs(x, name))
        }
        K::SysCall { args, .. } => args.iter().any(|x| expr_definitely_refs(x, name)),
        K::MethodCall { recv, args, .. } => {
            expr_definitely_refs(recv, name) || args.iter().any(|x| expr_definitely_refs(x, name))
        }
        K::Paren { inner } => expr_definitely_refs(inner, name),
        K::MinTypMax { min, typ, max } => {
            expr_definitely_refs(min, name)
                || expr_definitely_refs(typ, name)
                || expr_definitely_refs(max, name)
        }
        K::Cast { target, expr } => {
            expr_definitely_refs(expr, name)
                || match target {
                    ast::CastTarget::Size(s) => expr_definitely_refs(s, name),
                    _ => false,
                }
        }
        K::New { size, src } => {
            expr_definitely_refs(size, name)
                || src.as_ref().is_some_and(|s| expr_definitely_refs(s, name))
        }
        K::NamedArg { value, .. } => value
            .as_ref()
            .is_some_and(|v| expr_definitely_refs(v, name)),
        _ => false,
    }
}

/// R17 §3.3: CONSERVATIVE "this `@(…)` sensitivity list certainly does NOT reference
/// `name`". The accept-gate twin of [`sensitivity_reads_ident`], built on
/// [`expr_no_ref`] so an unvetted event expression answers "may reference".
/// `@(*)` names no expression at all and is trivially ref-free.
pub(crate) fn sensitivity_no_ref(s: &ast::Sensitivity, name: &str) -> bool {
    match s {
        ast::Sensitivity::Star => true,
        ast::Sensitivity::List(evs) => evs.iter().all(|ev| expr_no_ref(&ev.expr, name)),
    }
}

/// R17 §3.3: CONSERVATIVE "this intra-assignment event control (`= @(ev) rhs` /
/// `= repeat(n) @(ev) rhs`) certainly does NOT reference `name`". Both the repeat
/// count and the event expressions are reads.
pub(crate) fn intra_event_no_ref(e: &ast::IntraEvent, name: &str) -> bool {
    e.repeat.as_ref().is_none_or(|r| expr_no_ref(r, name)) && sensitivity_no_ref(&e.ctrl, name)
}

/// CONSERVATIVE lvalue counterpart of [`expr_no_ref`] — the lvalue (write target
/// and any select index) certainly does NOT reference `name`. Uses `expr_no_ref`
/// for index sub-exprs (so a read hidden in an index is not a blind spot).
pub(crate) fn lvalue_no_ref(lv: &ast::Lvalue, name: &str) -> bool {
    lvalue_no_ref_with(lv, name, false)
}

/// R16 §3.2: the lvalue twin of [`expr_no_ref_deep`] — an all-segments path rule, so
/// a callee's hierarchical self-write `t.a = 99;` is seen as touching `a`.
pub(crate) fn lvalue_no_ref_deep(lv: &ast::Lvalue, name: &str) -> bool {
    lvalue_no_ref_with(lv, name, true)
}

fn lvalue_no_ref_with(lv: &ast::Lvalue, name: &str, deep: bool) -> bool {
    use ast::Lvalue as L;
    let sub_l = |x: &ast::Lvalue| lvalue_no_ref_with(x, name, deep);
    let sub_e = |x: &ast::Expr| expr_no_ref_with(x, name, deep);
    match lv {
        // As in `expr_no_ref`: any path headed by `name` references it (a whole
        // write `name`, a field/hier write `name.f`). Only another head is ref-free.
        L::Ident(p) => {
            if deep {
                p.segments.iter().all(|s| s.name != name)
            } else {
                p.segments.first().is_none_or(|s| s.name != name)
            }
        }
        L::BitSelect { base, index, .. } => sub_l(base) && sub_e(index),
        L::PartSelect { base, msb, lsb, .. } => sub_l(base) && sub_e(msb) && sub_e(lsb),
        L::IndexedPart {
            base,
            offset,
            width,
            ..
        } => sub_l(base) && sub_e(offset) && sub_e(width),
        L::Concat { parts, .. } => parts.iter().all(sub_l),
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
            // R17 §3.3: a TIMING prefix is just more expressions. `#1 y = 3;` references
            // `name` exactly as much as `y = 3;` does — not at all — but the timed form
            // used to answer `false` here purely because `delay`/`event` were present,
            // and then fell to `da_stmt`'s catch-all and aborted the whole walk. Vetting
            // the prefix expressions is both more precise and what the predicate's own
            // contract ("certainly does NOT reference `name`") already claimed.
            delay
                .as_ref()
                .is_none_or(|d| d.values.iter().all(|e| expr_no_ref(e, name)))
                && event.as_ref().is_none_or(|e| intra_event_no_ref(e, name))
                && lvalue_no_ref(lhs, name)
                && expr_no_ref(rhs, name)
        }
        S::SysTaskCall { args, .. } => args.iter().all(|a| expr_no_ref(a, name)),
        // A CONTAINER-METHOD statement (`q.delete();`, `s.putc(i,c);`) — a 2-segment
        // enable whose head is the receiver. It touches only that receiver and its
        // arguments, so one naming neither cannot read or write `name`.
        //
        // Leaving this unvetted was a misdiagnosis engine: one container-method
        // statement anywhere in an `if` arm (`if (c) begin d = 0; q.delete(); end`)
        // aborted the whole definite-assignment walk, and the error named `d` — a
        // variable assigned two tokens earlier — instead of the queue.
        //
        // Deliberately NOT extended to a plain `show(x);` user task enable (review F5):
        // v1 publishes block-locals as MODULE nets, so a callee body can name the
        // flattened bare name and read it with neither the head nor an argument
        // mentioning it — "a task writes only through an output actual" is an IEEE
        // scoping argument, and the flatten is precisely where IEEE scoping does not
        // hold. A single-segment enable stays unvetted, exactly as before.
        S::UserTaskCall { name: cn, args, .. } if cn.segments.len() == 2 => {
            cn.segments.first().is_none_or(|s| s.name != name)
                && args.iter().all(|a| expr_no_ref(a, name))
        }
        _ => false,
    }
}

/// R16 §3.2: CONSERVATIVE "this whole statement TREE certainly does not touch
/// `name`" — the callee-body question, as opposed to [`stmt_no_ref`]'s
/// single-statement one.
///
/// Needed because v1 publishes block-locals as MODULE nets: a callee body can name
/// the flattened bare name (or reach it hierarchically) with neither the call's head
/// nor any argument mentioning it. That is why a plain `show(x);` statement stayed
/// unvetted and aborted the definite-assignment walk — the walk then blamed whatever
/// local it was tracking, several lines away from the call.
///
/// Sound by construction: every leaf uses the all-segments [`expr_no_ref_deep`] /
/// [`lvalue_no_ref_deep`], every composite requires ALL children to be clean, and any
/// form not enumerated answers `false` ("may touch"). A nested user call is delegated
/// to `call_inert`, which resolves the callee and recurses with a depth budget — an
/// unresolvable or too-deep callee answers `false`.
pub(crate) fn stmt_no_ref_deep(
    s: &ast::Stmt,
    name: &str,
    call_inert: &dyn Fn(&ast::HierPath, &[ast::Expr], &str) -> bool,
) -> bool {
    use ast::Stmt as S;
    let sub = |st: &ast::Stmt| stmt_no_ref_deep(st, name, call_inert);
    let e_ok = |x: &ast::Expr| expr_no_ref_deep(x, name);
    match s {
        S::Null(_) => true,
        // A `disable` names a BLOCK, never a variable.
        S::Disable { .. } => true,
        S::Block { stmts, decls, .. } | S::Fork { stmts, decls, .. } => {
            decls
                .iter()
                .flat_map(|d| d.names.iter())
                .all(|n| n.init.as_ref().is_none_or(e_ok))
                && stmts.iter().all(sub)
        }
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
            delay.is_none()
                && event.is_none()
                && lvalue_no_ref_deep(lhs, name)
                && expr_no_ref_deep(rhs, name)
        }
        S::If {
            cond,
            then_s,
            else_s,
            ..
        } => e_ok(cond) && sub(then_s) && else_s.as_ref().is_none_or(|x| sub(x)),
        S::Case {
            scrutinee, items, ..
        } => {
            e_ok(scrutinee)
                && items.iter().all(|it| match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        labels.iter().all(e_ok) && sub(body)
                    }
                    ast::CaseItem::Default { body, .. } => sub(body),
                })
        }
        S::For {
            init,
            cond,
            step,
            body,
            ..
        } => sub(init) && e_ok(cond) && sub(step) && sub(body),
        S::While { cond, body, .. } => e_ok(cond) && sub(body),
        S::Repeat { count, body, .. } => e_ok(count) && sub(body),
        S::Forever { body, .. } => sub(body),
        S::Return { value, .. } => value.as_ref().is_none_or(e_ok),
        S::SysTaskCall { args, .. } => args.iter().all(e_ok),
        S::UserTaskCall { name: cn, args, .. } => call_inert(cn, args, name),
        // Timing / `assign` / `force` / SVA / anything else — not vetted.
        _ => false,
    }
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
