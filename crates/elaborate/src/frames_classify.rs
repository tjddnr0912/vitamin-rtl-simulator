//! frame subset classification — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

// ── free helpers (pure, no &self) ──────────────────────────────────

/// Does this statement (recursively) contain its own timing control — `#delay`,
/// `@(event)`, or `wait` — anywhere on a path? Used to decide whether a bare
/// `always` (no header @) is a legal self-timed process (clock generator) vs an
/// unschedulable one. Conservative: any nested timing anywhere counts. (M-C)
pub(crate) fn stmt_has_timing(s: &ast::Stmt) -> bool {
    match s {
        ast::Stmt::DelayCtrl { .. }
        | ast::Stmt::EventCtrl { .. }
        | ast::Stmt::Wait { .. }
        | ast::Stmt::WaitFork { .. } => true,
        ast::Stmt::Block { stmts, .. } => stmts.iter().any(stmt_has_timing),
        ast::Stmt::If { then_s, else_s, .. } => {
            stmt_has_timing(then_s) || else_s.as_deref().is_some_and(stmt_has_timing)
        }
        ast::Stmt::Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_has_timing(body)
            }
        }),
        ast::Stmt::For { body, .. }
        | ast::Stmt::While { body, .. }
        | ast::Stmt::Repeat { body, .. }
        | ast::Stmt::Forever { body, .. } => stmt_has_timing(body),
        ast::Stmt::Fork { stmts, .. } => stmts.iter().any(stmt_has_timing),
        _ => false,
    }
}

/// B1 frame-call: does this function body need a frame (is it NOT straight-line
/// foldable by the inline path)? True for any control flow (if/case/loop/fork),
/// non-blocking assign, $systask, or timing — exactly the constructs
/// `fold_straight_line` rejects. A straight-line body (`Null`/`Block`/`Blocking`)
/// stays on the byte-identical inline path unless it is automatic/recursive.
pub(crate) fn body_needs_frame(s: &ast::Stmt) -> bool {
    use ast::Stmt::*;
    match s {
        Null(_) | Blocking { .. } => false,
        Block { stmts, .. } => stmts.iter().any(body_needs_frame),
        _ => true,
    }
}

/// Is a STATIC function body SELF-CONTAINED — reading only its own formals,
/// (block-)locals and return var, and calling no other user function? Then routing it
/// to the frame path is sensitivity-safe: an implicit `always_comb`/`@(*)` call site's
/// read-set is exactly the call's ARG expressions (added by `collect_expr_reads`), so
/// no body read is lost. A body that reads a module-level / hierarchical signal (or
/// calls another function whose body might) must stay INLINE, else that read vanishes
/// from the comb sensitivity list (stale/X). Fail CLOSED: any construct not proven
/// local-only ⇒ `false`.
pub(crate) fn body_reads_only_locals(f: &ast::FunctionDef) -> bool {
    let mut locals: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in &f.ports {
        locals.insert(p.name.name.clone());
    }
    let mut decls = f.body_decls.clone();
    collect_block_local_decls(&f.body, &mut decls);
    for d in &decls {
        for n in &d.names {
            locals.insert(n.name.name.clone());
        }
    }
    locals.insert(f.name.name.clone());
    stmt_reads_only_locals(&f.body, &locals)
}

/// Straight-line-body walker for [`body_reads_only_locals`]. A control-flow statement
/// fails closed (such a body is already routed by `body_needs_frame`, so this arm never
/// decides a route; it only guards the ㊁ context-width addition).
pub(crate) fn stmt_reads_only_locals(
    s: &ast::Stmt,
    locals: &std::collections::HashSet<String>,
) -> bool {
    use ast::Stmt::*;
    match s {
        Block { stmts, .. } => stmts.iter().all(|st| stmt_reads_only_locals(st, locals)),
        Blocking { lhs, rhs, .. } => {
            lvalue_reads_only_locals(lhs, locals) && expr_reads_only_locals(rhs, locals)
        }
        _ => false,
    }
}

/// An lvalue is local-only iff it is a plain single-segment local name. A concat /
/// indexed / hierarchical lvalue fails closed (its index sub-exprs, or the target
/// itself, could reach a non-local signal).
pub(crate) fn lvalue_reads_only_locals(
    lv: &ast::Lvalue,
    locals: &std::collections::HashSet<String>,
) -> bool {
    match lv {
        ast::Lvalue::Ident(p) => {
            matches!(p.segments.as_slice(), [seg] if locals.contains(&seg.name))
        }
        _ => false,
    }
}

/// Does `e` read only local names (and no user function call)? Fail CLOSED on any
/// variant not proven read-local (user `Call`, package/hierarchical ref, `new`, cast
/// targets, method/randomize/dist, …) so a routed body can never hide a non-local read.
pub(crate) fn expr_reads_only_locals(
    e: &ast::Expr,
    locals: &std::collections::HashSet<String>,
) -> bool {
    use ast::ExprKind::*;
    match &e.kind {
        IntLit { .. } | RealLit { .. } | StrLit { .. } | Null | Dollar => true,
        Ident(p) => matches!(p.segments.as_slice(), [seg] if locals.contains(&seg.name)),
        Paren { inner } => expr_reads_only_locals(inner, locals),
        Unary { operand, .. } => expr_reads_only_locals(operand, locals),
        Binary { lhs, rhs, .. } => {
            expr_reads_only_locals(lhs, locals) && expr_reads_only_locals(rhs, locals)
        }
        Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_reads_only_locals(cond, locals)
                && expr_reads_only_locals(then_e, locals)
                && expr_reads_only_locals(else_e, locals)
        }
        BitSelect { base, index } => {
            expr_reads_only_locals(base, locals) && expr_reads_only_locals(index, locals)
        }
        PartSelect { base, msb, lsb } => {
            expr_reads_only_locals(base, locals)
                && expr_reads_only_locals(msb, locals)
                && expr_reads_only_locals(lsb, locals)
        }
        IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            expr_reads_only_locals(base, locals)
                && expr_reads_only_locals(offset, locals)
                && expr_reads_only_locals(width, locals)
        }
        Concat { parts } => parts.iter().all(|p| expr_reads_only_locals(p, locals)),
        Replicate { count, value } => {
            expr_reads_only_locals(count, locals)
                && value.iter().all(|v| expr_reads_only_locals(v, locals))
        }
        // `$signed`/`$unsigned`/… read only their (local) argument expressions.
        SysCall { args, .. } => args.iter().all(|a| expr_reads_only_locals(a, locals)),
        Cast { expr, .. } => expr_reads_only_locals(expr, locals),
        _ => false,
    }
}

/// True if the function's body ELEMENT-SELECTS a multi-dim PACKED local
/// (`logic [1:0][7:0] p; … = p[0]`). The inline path binds a local by SUBSTITUTION
/// (a flat value carrying no packed dims), so `p[k]` folds as a BIT-select — silently
/// wrong (`p[0]` yields bit 0, not byte `[7:0]`). `build_frame_set` routes such a
/// function to the (correct) frame path when self-contained; a non-routable one is
/// loud-rejected in `reduce_function_body`. Only MULTI-dim packed (`d.packed`
/// non-empty) counts — a 1-D `logic [7:0] p; p[0]` is a legitimate bit-select. A
/// whole-use md-packed local (no select) folds correctly (flat value) → NOT flagged.
pub(crate) fn body_selects_mdpacked_local(f: &ast::FunctionDef) -> bool {
    let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut decls = f.body_decls.clone();
    collect_block_local_decls(&f.body, &mut decls);
    for d in &decls {
        if !d.packed.is_empty() {
            for n in &d.names {
                names.insert(n.name.name.clone());
            }
        }
    }
    !names.is_empty() && stmt_selects_name(&f.body, &names)
}

/// Straight-line-body walker for [`body_selects_mdpacked_local`] (an inline body is
/// Block+Blocking; control flow is already framed by `body_needs_frame`). True if any
/// RHS reads `<name>[…]` for a name in `names`. (An element-select LHS write is
/// separately loud — not reducible / outside the frame subset — so only the RHS read
/// is the silent path checked here.)
pub(crate) fn stmt_selects_name(s: &ast::Stmt, names: &std::collections::HashSet<String>) -> bool {
    use ast::Stmt::*;
    match s {
        Block { stmts, .. } => stmts.iter().any(|st| stmt_selects_name(st, names)),
        Blocking { rhs, .. } => expr_selects_name(rhs, names),
        _ => false,
    }
}

/// True if `e` contains a bit/part/indexed select whose BASE is a bare name in
/// `names` (`p[k]` / `p[a:b]` / `p[a+:w]`). Recurses every operand position.
pub(crate) fn expr_selects_name(e: &ast::Expr, names: &std::collections::HashSet<String>) -> bool {
    use ast::ExprKind as K;
    let named_base = |b: &ast::Expr| matches!(&b.kind, K::Ident(p) if p.segments.first().is_some_and(|s| names.contains(&s.name)));
    let r = |x: &ast::Expr| expr_selects_name(x, names);
    match &e.kind {
        K::BitSelect { base, index } => named_base(base) || r(base) || r(index),
        K::PartSelect { base, msb, lsb } => named_base(base) || r(base) || r(msb) || r(lsb),
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => named_base(base) || r(base) || r(offset) || r(width),
        K::Unary { operand, .. } => r(operand),
        K::Binary { lhs, rhs, .. } => r(lhs) || r(rhs),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => r(cond) || r(then_e) || r(else_e),
        K::Concat { parts } => parts.iter().any(r),
        K::Replicate { count, value } => r(count) || value.iter().any(r),
        K::Call { args, .. } | K::SysCall { args, .. } => args.iter().any(r),
        K::Paren { inner } => r(inner),
        K::MinTypMax { min, typ, max } => r(min) || r(typ) || r(max),
        K::Cast { target, expr } => r(expr) || matches!(target, ast::CastTarget::Size(s) if r(s)),
        _ => false,
    }
}

/// Does any statement in `s` lower a `return`? Gates the return-exit-block setup
/// in the frame-function/task and inline-task paths, so a body WITHOUT `return`
/// keeps its exact prior IR (byte-identical golden output) and only return-using
/// bodies pay for the extra exit block. Coverage mirrors [`stmt_disables_name`].
pub(crate) fn body_has_return(s: &ast::Stmt) -> bool {
    use ast::Stmt::*;
    match s {
        Return { .. } => true,
        Block { stmts, .. } | Fork { stmts, .. } => stmts.iter().any(body_has_return),
        If { then_s, else_s, .. } => {
            body_has_return(then_s) || else_s.as_ref().is_some_and(|e| body_has_return(e))
        }
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                body_has_return(body)
            }
        }),
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            body_has_return(body)
        }
        _ => false,
    }
}

/// Is `e` REPEATABLE — does evaluating it twice give the same answer and the same
/// observable effects as evaluating it once?
///
/// The inline fold substitutes a body-local's defining RHS at every REFERENCE to that
/// local, and the arena keeps one node that the evaluator then walks once per reference.
/// For a pure RHS that is only wasted work. For an impure one it is a defect, measured
/// against iverilog on the path that exists at HEAD:
///
/// ```text
///   reg [31:0] u; u = $random;  g = u ^ u;        vita 3533466533   ivl 0
///   u = $urandom; g = u ^ u;                      vita 2354591315   ivl 0
///   u = sf(a);    g = u ^ u ^ u;   (sf $displays) vita prints 3x    ivl 1x
///   u = mem[99];  g = u ^ u ^ u;   (mem[0:3])     vita E4002 x3     ivl silent
/// ```
///
/// ⚠️ FAIL CLOSED. Every construct not listed as repeatable answers `false`, because the
/// cost of a wrong `true` is a silent-wrong and the cost of a wrong `false` is one
/// function taking the frame path — which is strictly more capable since §4.5.198/199.
/// In particular an ARRAY ELEMENT read is NOT repeatable: it is the one shape above whose
/// impurity is a diagnostic rather than a value, and vita reports it once per reference.
///
/// ⚠️⚠️ Only the FIRST TWO rows above are actually reached. The routing is gated on
/// `body_reads_only_locals`, which answers `false` for a user `Call` and for a module-net
/// read — so rows 3 and 4 stay exactly as they were. They are listed because they are the
/// same defect and the same evidence, not because this closes them; closing them needs
/// the sensitivity-list prerequisite that gate exists for.
fn expr_is_repeatable(e: &ast::Expr) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        // Literals and a plain name: no effect, and nothing can change between two
        // reads inside ONE expression evaluation (a frame body cannot write a module
        // net, and no net write interleaves inside a single `eval_ctx`).
        K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::TimeLit { .. } => true,
        K::PkgScoped { .. } | K::Null => true,
        K::Ident(_) => true,
        K::Paren { inner } => expr_is_repeatable(inner),
        K::Unary { operand, .. } => expr_is_repeatable(operand),
        K::Binary { lhs, rhs, .. } => expr_is_repeatable(lhs) && expr_is_repeatable(rhs),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => expr_is_repeatable(cond) && expr_is_repeatable(then_e) && expr_is_repeatable(else_e),
        // A part-select of a NAME is repeatable; a part-select whose base is an array
        // element is not, and that is decided by recursing into `base`.
        K::PartSelect { base, msb, lsb } => {
            expr_is_repeatable(base) && expr_is_repeatable(msb) && expr_is_repeatable(lsb)
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => expr_is_repeatable(base) && expr_is_repeatable(offset) && expr_is_repeatable(width),
        K::Concat { parts } => parts.iter().all(expr_is_repeatable),
        K::Replicate { count, value } => {
            expr_is_repeatable(count) && value.iter().all(expr_is_repeatable)
        }
        // ⚠️ `a[i]` is deliberately NOT repeatable. An out-of-range read REPORTS, and
        // vita files that report once per evaluation — so a duplicated element read
        // turns one `E4002` into N and an exit 0 into an exit 1. Measured.
        K::BitSelect { .. } => false,
        // Everything else fails closed: `$random` and `$urandom` draw, a user call runs
        // a body that can print, `new`/`randomize`/`with`-clause carry state.
        _ => false,
    }
}

/// Does this body DUPLICATE a non-repeatable value — assign a local from an RHS that
/// cannot be evaluated twice, and then name that local more than once?
///
/// That is exactly the shape the inline fold turns into repeated evaluation, so such a
/// function is routed to the frame path, which binds the RHS to a slot ONCE and reads the
/// slot thereafter. The frame path is already linear in the reference count where the
/// inline path is exponential — `(refs)^(chained statements)`, measured — so the routing
/// closes a performance cliff as a side effect, but the reason it exists is the four
/// measured silent-wrongs above.
///
/// Straight-line only, because a body with control flow is already framed by
/// `body_needs_frame` before this is ever asked.
fn body_duplicates_a_non_repeatable_local(f: &ast::FunctionDef) -> bool {
    // ⚠️⚠️ BAIL on an INTRA-ASSIGNMENT DELAY, body-wide.
    //
    // `body_needs_frame` answers `false` for a `Blocking` regardless of its `delay`, and
    // `fold_straight_line` deliberately ACCEPTS one with a warning ("intra-assignment
    // delay in inlined function dropped"). `classify_frame_body` does NOT — it refuses
    // the body outright. So routing a delayed body hands a design the inline path RAN to
    // a path that rejects it: `u = #3 $signed(a); fn = u + u;` was `r=42` (iverilog's
    // answer) plus a warning, and became E3009 exit 1. That is correct → loud, which the
    // accuracy ladder forbids however non-conforming the design is (IEEE 1364 §10.4.4
    // bans time control in a function; vita's accept was deliberate and warned).
    //
    // ⚠️ BODY-WIDE, not per-statement. The delayed statement need not be the
    // non-repeatable one: `u = #3 a;` (repeatable) followed by `v = $signed(a[7:0]);
    // fn = u+v+v;` routes on `v` and is refused for `u`'s delay.
    let mut has_delay = false;
    walk_straight_line(&f.body, &mut |st| {
        if let ast::Stmt::Blocking { delay, event, .. } = st {
            if delay.is_some() || event.is_some() {
                has_delay = true;
            }
        }
    });
    if has_delay {
        return false;
    }

    // (name → the count of REFERENCES seen since its non-repeatable definition).
    let mut pending: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut hit = false;
    walk_straight_line(&f.body, &mut |st| {
        if let ast::Stmt::Blocking { lhs, rhs, .. } = st {
            // Count references in the RHS *before* re-defining, so `u = u + f()` counts
            // the read of `u` that happens on the way in.
            let mut names_a_pending = false;
            for (nm, n) in pending.iter_mut() {
                let c = expr_ref_count(rhs, nm);
                if c > 0 {
                    names_a_pending = true;
                    *n += c;
                    if *n >= 2 {
                        hit = true;
                    }
                }
            }
            if let Some(target) = lvalue_bare_name(lhs) {
                // ⚠️⚠️ TRANSITIVITY. Repeatability has to be judged AFTER substitution,
                // not on the surface syntax: `fold_straight_line` binds each local to the
                // arena node its RHS lowered to UNDER the current substitution, so
                // `u = $random; v = u; fn = v ^ v;` gives `v` the SAME impure node and
                // duplicates it exactly as `fn = u ^ u` would. Judging `v = u` on its own
                // text calls it repeatable and loses the whole chain — measured: seven of
                // twenty-six defective cells survived the first version of this, through
                // `+`, `{}`, `&`, `[m:l]` and a bare copy.
                //
                // So a definition inherits pending status from any pending name it reads.
                if expr_is_repeatable(rhs) && !names_a_pending {
                    pending.remove(&target);
                } else {
                    pending.insert(target, 0);
                }
            }
        }
    });
    hit
}

/// The bare NAME an lvalue writes, when it is a whole scalar. A select/concat lvalue
/// returns `None` — this predicate only needs the simple `u = <rhs>;` shape the inline
/// fold substitutes through.
fn lvalue_bare_name(l: &ast::Lvalue) -> Option<String> {
    match l {
        ast::Lvalue::Ident(p) if p.segments.len() == 1 => Some(p.segments[0].name.clone()),
        _ => None,
    }
}

/// How many times `name` is NAMED in `e`. Counts occurrences rather than answering a
/// yes/no, because one reference is exactly the case the inline fold handles correctly.
fn expr_ref_count(e: &ast::Expr, name: &str) -> u32 {
    let mut n = 0;
    walk_expr_refs(e, &mut |id| {
        if id == name {
            n += 1;
        }
    });
    n
}

fn walk_expr_refs(e: &ast::Expr, f: &mut impl FnMut(&str)) {
    use ast::ExprKind as K;
    match &e.kind {
        K::Ident(p) => {
            if p.segments.len() == 1 {
                f(&p.segments[0].name);
            }
        }
        K::Paren { inner } => walk_expr_refs(inner, f),
        K::Unary { operand, .. } => walk_expr_refs(operand, f),
        K::Binary { lhs, rhs, .. } => {
            walk_expr_refs(lhs, f);
            walk_expr_refs(rhs, f);
        }
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            walk_expr_refs(cond, f);
            walk_expr_refs(then_e, f);
            walk_expr_refs(else_e, f);
        }
        K::BitSelect { base, index } => {
            walk_expr_refs(base, f);
            walk_expr_refs(index, f);
        }
        K::PartSelect { base, msb, lsb } => {
            walk_expr_refs(base, f);
            walk_expr_refs(msb, f);
            walk_expr_refs(lsb, f);
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            walk_expr_refs(base, f);
            walk_expr_refs(offset, f);
            walk_expr_refs(width, f);
        }
        K::Concat { parts } => parts.iter().for_each(|p| walk_expr_refs(p, f)),
        K::Replicate { count, value } => {
            walk_expr_refs(count, f);
            value.iter().for_each(|p| walk_expr_refs(p, f));
        }
        K::Call { args, .. } | K::SysCall { args, .. } => {
            args.iter().for_each(|a| walk_expr_refs(a, f))
        }
        // ⚠️ `Cast` is the ONE variant that `expr_reads_only_locals` admits and this walk
        // could miss, so a reference hiding in `int'(u)` counted as ZERO and the body was
        // not routed. Both siblings in this file already have the arm
        // (`expr_reads_only_locals`, `expr_selects_name`); this is the same spelling.
        // Every other unwalked variant — `MinTypMax`, `MethodCall`, `New`, `ClassNew`,
        // `Dist`, the assignment patterns, `RandomizeWith`, `ArrayMethodWith`,
        // `NamedArg` — is refused by that gate before this walk is reached.
        K::Cast { target, expr } => {
            walk_expr_refs(expr, f);
            if let ast::CastTarget::Size(sz) = target {
                walk_expr_refs(sz, f);
            }
        }
        _ => {}
    }
}

/// Visit every statement of a STRAIGHT-LINE body in source order.
///
/// ⚠️ The body IS straight-line here — `body_needs_frame` sits earlier in the same
/// short-circuiting `||` chain — but that is an invariant carried by DISJUNCT ORDER, not
/// by this function, and reordering the chain would silently make this walk under-visit.
/// The callback sees every statement including ones this walk cannot descend into, so a
/// caller that must fail closed can check the kind itself; `debug_assert` states the
/// invariant so a reorder fails loudly in CI rather than quietly routing less.
fn walk_straight_line(s: &ast::Stmt, f: &mut impl FnMut(&ast::Stmt)) {
    match s {
        ast::Stmt::Block { stmts, .. } => stmts.iter().for_each(|st| walk_straight_line(st, f)),
        other => {
            debug_assert!(
                matches!(other, ast::Stmt::Null(_) | ast::Stmt::Blocking { .. }),
                "walk_straight_line reached a control-flow statement;                  `body_needs_frame` is supposed to have routed this body already"
            );
            f(other)
        }
    }
}

impl Elaborator<'_> {
    /// AST-side companion to `expr_is_real`: does `e` evaluate to real *because* it
    /// reaches a real-returning user function call through a real-propagating
    /// position (the same operators `expr_is_real` recurses through)?
    pub(crate) fn ast_has_real_call(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.ast_has_real_call(inner),
            ast::ExprKind::Unary { op, operand } => {
                matches!(op, ast::UnOp::Plus | ast::UnOp::Minus) && self.ast_has_real_call(operand)
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                // `**` is real-propagating in SV (§11.4.4) and was missing from
                // this list and from its IR-side twin `expr_is_real`, which carries the
                // identical four-op list — so `fa(2) ** 1` was not seen as real.
                matches!(
                    op,
                    ast::BinOp::Add
                        | ast::BinOp::Sub
                        | ast::BinOp::Mul
                        | ast::BinOp::Div
                        | ast::BinOp::Pow
                ) && (self.ast_has_real_call(lhs) || self.ast_has_real_call(rhs))
            }
            ast::ExprKind::Ternary { then_e, else_e, .. } => {
                self.ast_has_real_call(then_e) || self.ast_has_real_call(else_e)
            }
            ast::ExprKind::Call { name, .. } => {
                name.segments.len() == 1
                    && self
                        .func_table
                        .get(&name.segments[0].name)
                        .is_some_and(|f| {
                            matches!(f.ret_type, ast::ParamType::Real | ast::ParamType::Realtime)
                        })
            }
            _ => false,
        }
    }

    /// Round-14 V3/V4: decide the DEFERRED frame-TASK reject now that every task body is
    /// lowered. `compute_suspendable_tasks` gives the transitive suspendable set:
    /// - a task in that set that is LEAF and NON-SUSPENDING (no `Delay`/`Wait`/`Fork`/
    ///   `Call` terminator in its own body) is LIFTED — the engine routes it through
    ///   `run_process` (V3/V4 Phase 2). Its `$display`/NBA run and it needs no suspend.
    /// - a task in the set that is otherwise (has timing, or a nested/transitive call to a
    ///   suspendable task) stays LOUD (E3009) — the per-activity-window + nested-frame
    ///   phases are a follow-on; running it now would drop it onto the synchronous
    ///   executor (silent-wrong), so correct-or-loud keeps it a clean reject.
    /// - a task NOT in the set is a plain subset task (or an lvalue-kind non-subset one) —
    ///   the classic `validate_frame_body` (a subset task calling only subset tasks runs
    ///   synchronously, byte-identical to before this feature).
    pub(crate) fn resolve_frame_task_rejects(&mut self) {
        // r18: frame-aware suspend classification — `base_nets[fi]` (from `func_metas`,
        // threaded verbatim to the engine's `func_table`) lets a task that writes an
        // out-of-frame module/instance net lift to the suspendable path instead of being
        // loud-rejected as a subset. Elaborate and the engine pass the SAME `base_nets`.
        let base_nets: Vec<u32> = self.func_metas.iter().map(|m| m.base_net).collect();
        // §4.5.208: `has_hier_call` forces a frame task with a deferred hier enable
        // suspendable — the SAME derivation the engine uses (both from `FuncMeta`).
        let force_suspend: Vec<bool> = self.func_metas.iter().map(|m| m.has_hier_call).collect();
        // R23 §3.1: the nested call sites' COPY-OUT destinations. Reduced through the
        // shared `ir::call_out_nets` (the engine calls the same function over the same
        // table, threaded verbatim), so a call whose output actual is a module net makes
        // the CALLING task suspendable instead of aborting the `&self` executor.
        let call_out_nets = ir::call_out_nets(
            self.task_calls_func
                .iter()
                .map(|(b, info)| (*b, info.out_binds.as_slice())),
        );
        let full = ir::compute_suspendable_tasks(
            &self.funcs,
            &self.func_blocks,
            &self.stmts,
            &self.exprs,
            &base_nets,
            &force_suspend,
            &call_out_nets,
        );
        let pending = std::mem::take(&mut self.frame_task_pending);
        for (fid, name, base_net, locals_len, unsafe_repeat) in pending {
            if full.contains(&fid) {
                // ⚠️ The three rejecting predicates are evaluated INTO NAMES before the
                // decision, so the diagnostic can say WHICH one fired. It used to list
                // all of them and let the reader guess — and the guess was wrong for the
                // commonest case: a `repeat` with a non-constant count is neither a
                // `fork` nor anything frame-LOCAL, so an external reporter re-wrote a
                // correct `localparam` twice chasing a cause the message had invented.
                let unsafe_ctor = self.frame_task_has_unsafe_construct(fid, base_net, locals_len);
                let leaf =
                    self.frame_body_is_leaf_nonsuspending(fid, base_net, base_net + locals_len);
                if leaf && !unsafe_repeat && unsafe_ctor.is_none() {
                    // lifted — the engine's suspendable-frame path runs it.
                    // Stage-2 fork-in-frame: if the body has an admitted Case-B `join` fork
                    // (an arm touches this task's frame-local range), flag the task so the
                    // engine's `enter_task_frame` gives it a shared-window ARENA (a
                    // `WindowSlot::Shared`) that the parent and its fork arms reference by
                    // handle. `fork_arms_self_contained` returns `CaseB` ONLY for the
                    // mode-gated `join` case (join_any/join_none is `Loud` → not lifted), so
                    // this is exactly the set that needs the arena. A no-fork / Case-A task
                    // returns `CaseA` here → flag stays false → byte-identical owned window.
                    let entry = self.funcs[fid as usize].entry;
                    if self.func_metas[fid as usize].is_automatic
                        && self.fork_arms_self_contained(entry, base_net, base_net + locals_len)
                            == ForkAdmit::CaseB
                    {
                        self.func_metas[fid as usize].contains_shared_fork = true;
                    }
                } else {
                    // Most specific first: `unsafe_repeat` and `unsafe_ctor` name an exact
                    // construct, while `!leaf` is the general backstop and can only describe
                    // the class.
                    let why = if unsafe_repeat {
                        "a `repeat` around timing control whose count is not a constant this \
                         elaborator folds — such a count desugars to a module-scope counter net \
                         that concurrent activations would share. A CONSTANT count is unrolled \
                         and IS supported, including a folded one (`repeat (64)`, `repeat (4*16)`, \
                         `repeat (LP)`, `repeat ($clog2(LP))`); a count read from a variable is not"
                    } else {
                        unsafe_ctor.unwrap_or(
                            "a construct outside the supported subset (the body is not a leaf \
                             non-suspending frame body — it forks, or calls something that does)",
                        )
                    };
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "frame task `{name}` body uses {why}. Supported here: `$display`, \
                             a nonblocking assign to a MODULE net, `@`/`#`/`wait` on a module \
                             net, and nested task calls"
                        ),
                    );
                }
            } else {
                // V2A-dyn (§4.5.194): a subset (non-suspendable) task with a dynamic-array
                // input formal is now SUPPORTED. `dyn_heap` is interior-mutable, so the
                // engine deep-copies the caller's array into the formal's per-activation heap
                // slot right before the synchronous `run_task_call` and frees it after (the
                // exec.rs subset arms), exactly like the suspendable path's frame-entry
                // snapshot (§4.5.173). Validate the body as an ordinary subset task — a dyn
                // formal READ is fine; a WRITE to it still hits the `&self` heap-write fatal.
                let entry_bb = self.funcs[fid as usize].entry;
                self.validate_frame_body(&name, entry_bb, base_net, locals_len, true);
            }
        }
    }

    /// Round-14 V3/V4 (adversarial-review guard): a suspendable task carries a construct
    /// the engine's frame machinery cannot yet run CORRECTLY, so it must stay LOUD rather
    /// than silently mis-run. Covers the differential-review findings:
    /// - a frame-local UNPACKED ARRAY (`int a[0:2]`) — reserved as a 1-element net, so
    ///   element access collapses to a 1-bit select (silent-wrong, even with no suspend);
    /// - a nonblocking assign whose LHS is a frame-local (illegal per LRM — iverilog
    ///   rejects — and the engine's arg-bind path panics on it);
    /// - a `Wait` whose condition reads a frame-local (the level-wait re-eval runs without
    ///   the frame window restored → panic);
    /// - a `disable fork` statement (a fork-family construct with no in-frame guard);
    /// - a `wait fork` statement (ditto — the engine's `WaitCause::Fork` in-frame arm has
    ///   no implicit child-barrier tracking per activation, so it `mark_fatal`s at RUNTIME
    ///   with a misleading delta-limit diagnostic; catching it here turns that into a clean
    ///   compile-time E3009, matching `disable fork`'s treatment).
    ///
    /// The `repeat(<non-const>) @(edge)` shared-counter case is caught separately, at the
    /// AST level (`ast_has_repeat_with_timing`), before the desugar loses it.
    pub(crate) fn frame_task_has_unsafe_construct(
        &self,
        fid: u32,
        base_net: u32,
        locals_len: u32,
    ) -> Option<&'static str> {
        let (lo, hi) = (base_net, base_net + locals_len);
        let is_frame_local = |n: u32| n >= lo && n < hi;
        // (1) a frame-local unpacked array declared by this task.
        if (lo..hi).any(|n| self.frame_array_local.contains(&n)) {
            return Some("an unpacked ARRAY declared local to this task");
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![self.funcs[fid as usize].entry];
        while let Some(b) = stack.pop() {
            if !seen.insert(b) {
                continue;
            }
            let Some(blk) = self.func_blocks.get(b as usize) else {
                continue;
            };
            for &sid in &blk.stmts {
                match &self.stmts[sid as usize] {
                    // (2) a nonblocking assign to a frame-local LHS.
                    ir::Stmt::NonblockingAssign { lhs, .. } => {
                        if lhs.chunks.iter().any(|c| is_frame_local(c.net)) {
                            return Some(
                                "a nonblocking assign (`<=`) to a variable local to this task",
                            );
                        }
                    }
                    // (4) `disable fork` — a fork-family construct the engine's in-frame
                    // path can't run; `compute_suspendable_tasks` counts it as a signal
                    // (so the engine WOULD route it), but the DisableFork apply has no
                    // in-frame guard, so it must stay loud (soundness-review finding).
                    ir::Stmt::Disable {
                        scope_kind: ir::DisableKind::Fork,
                        ..
                    } => return Some("a `disable fork`"),
                    _ => {}
                }
            }
            match &blk.term {
                // (3) a `wait`/`@` whose condition expr reads a frame-local net.
                ir::Terminator::Wait { cond, resume } => {
                    if self.wait_cond_reads_frame_local(cond, &is_frame_local) {
                        return Some(
                            "a `wait`/`@` whose condition reads a variable local to this task",
                        );
                    }
                    // (5) `wait fork` — see the doc comment above; no in-frame child-
                    // barrier support, so reject at compile time rather than let the
                    // engine's runtime `mark_fatal` backstop fire.
                    if matches!(cond, ir::WaitCause::Fork) {
                        return Some("a `wait fork`");
                    }
                    stack.push(*resume);
                }
                // A `#(d)` delay is NOT a frame-hazard here: the `Delay` terminator
                // evaluates `amount` with this task's frame window live (a variable
                // delay reading a frame-local is legal and iverilog-matching in an
                // ordinary suspendable task, and a post-join continuation resumes with
                // the parent window restored). The ONLY hazard — a `#(d)` amount read on
                // a fork ARM's empty owned window — is caught precisely by
                // `classify_one_arm` in frames_classify_fork.rs, not by this general
                // backstop. So just follow the continuation.
                ir::Terminator::Delay { resume, .. } => stack.push(*resume),
                ir::Terminator::Goto { target } => stack.push(*target),
                ir::Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
                // Stage-1 fork-in-frame: a `Fork` is now reachable here (a Case-A fork is
                // admitted by `frame_body_is_leaf_nonsuspending`). Walk its arms AND its
                // post-join continuation so a frame-local ARRAY / NBA-to-a-frame-local /
                // `disable fork` / `wait`-reading-a-frame-local anywhere in the fork or
                // after it still forces this task loud (correct-or-loud backstop).
                ir::Terminator::Fork {
                    children,
                    join,
                    resume_bb,
                } => {
                    stack.extend(children.iter().copied());
                    stack.push(*join);
                    stack.push(*resume_bb);
                }
                ir::Terminator::Return => {}
            }
        }
        None
    }

    /// Does a `Wait` condition read a frame-local net? (`@`/`wait` re-evaluates its
    /// condition on wake without the frame window restored, so a frame-local read there
    /// panics — such a task stays loud.) An `@(posedge module_clk)` / `wait(module_net)`
    /// reads only module nets and is fine.
    pub(crate) fn wait_cond_reads_frame_local(
        &self,
        cond: &ir::WaitCause,
        is_frame_local: &impl Fn(u32) -> bool,
    ) -> bool {
        match cond {
            ir::WaitCause::Edge { net, .. } => is_frame_local(*net),
            ir::WaitCause::Level { nets } => nets.iter().any(|&n| is_frame_local(n)),
            ir::WaitCause::Named { ev } => is_frame_local(*ev),
            ir::WaitCause::Expr { expr } => {
                let mut reads = std::collections::BTreeSet::new();
                self.collect_expr_reads(*expr, &mut reads);
                reads.iter().any(|&n| is_frame_local(n))
            }
            ir::WaitCause::Fork => false,
        }
    }

    /// Round-14 V3/V4 (adversarial guard): does `stmt` contain a `repeat(...)` whose body
    /// has a timing control (@/#/wait)? A non-const `repeat`'s HIDDEN loop counter is a
    /// shared net (not per-activation — by design; a const `repeat` unrolls instead), so
    /// concurrent activations of a suspendable task corrupt each other's iteration count
    /// across a suspend (differential-review silent-wrong). Keep such a task loud.
    /// Conservative (a const `repeat` is unrolled and safe, but rejecting it too is
    /// correct-or-loud — and the KAT-driver pattern uses no `repeat`).
    ///
    /// ⚠️ Takes `&self` ONLY so the unroll question is asked with the lowering's own
    /// spelling (`repeat_unroll_count`); it reads no other elaborator state.
    pub(crate) fn ast_has_repeat_with_timing(&self, stmt: &ast::Stmt) -> bool {
        use ast::Stmt as S;
        match stmt {
            S::Repeat { count, body, span } => {
                // r18 (C1): a const-small `repeat(N)` is straight-UNROLLED by `lower_repeat`
                // (no runtime counter), so its timing is safe across suspends — only a
                // NON-const / large count desugars to the shared `$repeat_cnt$` net whose
                // value would corrupt across concurrent activations. So flag the timing only
                // when the count is NOT a const the unroller would consume.
                // A non-unrolled count now gets a FRAME-LOCAL counter
                // (`reserve_frame_repeat_counters`), which is per activation — so the
                // cross-activation hazard this flag existed for is gone and the only
                // remaining question is whether the counter net is frame-local at all.
                // It is, for every `repeat` the reservation pass saw, which is every
                // one in this body.
                let unrolled = self.repeat_unroll_count(count).is_some()
                    || self.frame_repeat_cnt.contains_key(&(span.lo, span.hi));
                (!unrolled && Self::ast_stmt_has_timing(body))
                    || self.ast_has_repeat_with_timing(body)
            }
            S::Block { stmts, .. } | S::Fork { stmts, .. } => {
                stmts.iter().any(|x| self.ast_has_repeat_with_timing(x))
            }
            S::If { then_s, else_s, .. } => {
                self.ast_has_repeat_with_timing(then_s)
                    || else_s
                        .as_deref()
                        .is_some_and(|x| self.ast_has_repeat_with_timing(x))
            }
            S::For { body, .. } | S::While { body, .. } | S::Forever { body, .. } => {
                self.ast_has_repeat_with_timing(body)
            }
            S::DelayCtrl { body, .. } | S::EventCtrl { body, .. } | S::Wait { body, .. } => body
                .as_deref()
                .is_some_and(|x| self.ast_has_repeat_with_timing(x)),
            S::Case { items, .. } => items
                .iter()
                .any(|it| self.ast_has_repeat_with_timing(case_item_body(it))),
            _ => false,
        }
    }

    /// Any @/#/wait timing control anywhere in `stmt` (recursively).
    pub(crate) fn ast_stmt_has_timing(stmt: &ast::Stmt) -> bool {
        use ast::Stmt as S;
        match stmt {
            S::DelayCtrl { .. } | S::EventCtrl { .. } | S::Wait { .. } | S::WaitFork { .. } => true,
            S::Block { stmts, .. } | S::Fork { stmts, .. } => {
                stmts.iter().any(Self::ast_stmt_has_timing)
            }
            S::If { then_s, else_s, .. } => {
                Self::ast_stmt_has_timing(then_s)
                    || else_s.as_deref().is_some_and(Self::ast_stmt_has_timing)
            }
            S::For { body, .. }
            | S::While { body, .. }
            | S::Forever { body, .. }
            | S::Repeat { body, .. } => Self::ast_stmt_has_timing(body),
            S::Case { items, .. } => items
                .iter()
                .any(|it| Self::ast_stmt_has_timing(case_item_body(it))),
            _ => false,
        }
    }

    /// Round-14 V3/V4 Phase 2/3: is (suspendable) task `fid` LIFTABLE onto the engine's
    /// suspendable-frame path — a LEAF task (no nested task `Call`) with no non-Case-A
    /// `Fork`. The engine's `run_process` handles its `$display`/NBA (Phase 2) and its
    /// `@`/`#`/`wait` suspend via the per-activity window stash/restore (Phase 3). A
    /// nested-task-call (`Call`) is supported (the engine pushes a nested frame, or runs a
    /// subset callee synchronously). Stage-1 fork-in-frame: a `Fork` whose arms are all
    /// self-contained (`ForkAdmit::CaseA`, decided over `[lo,hi)` = the task's frame-local
    /// range) is ALSO liftable — the concurrent children ride owned windows and the parked
    /// parent's window stays stashed in its `FrameRec`; a Case-B / nested / disable-fork
    /// fork stays LOUD (correct-or-loud, follow-on stages). Walks the REACHABLE blocks from
    /// the task's entry (a range walk would spill into later tasks' blocks — every task's
    /// body sits in the shared `func_blocks` arena).
    pub(crate) fn frame_body_is_leaf_nonsuspending(&self, fid: u32, lo: u32, hi: u32) -> bool {
        // fork-in-frame verdict is a whole-body property — compute it at most ONCE
        // (lazily, only if this body actually contains a `Fork`).
        let mut fork_admit: Option<ForkAdmit> = None;
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![self.funcs[fid as usize].entry];
        while let Some(b) = stack.pop() {
            if !seen.insert(b) {
                continue;
            }
            let Some(blk) = self.func_blocks.get(b as usize) else {
                continue;
            };
            match &blk.term {
                ir::Terminator::Fork { resume_bb, .. } => {
                    let adm = *fork_admit.get_or_insert_with(|| {
                        self.fork_arms_self_contained(self.funcs[fid as usize].entry, lo, hi)
                    });
                    // Stage 2: a Case-B `join` fork is now liftable (the engine gives the task
                    // a shared-window arena — see `contains_shared_fork` in
                    // `resolve_frame_task_rejects`). Only a `Loud` verdict — a nested fork, a
                    // `disable fork`, or a Case-B `join_any`/`join_none` (mode-gated to Loud in
                    // `fork_arms_self_contained`) — keeps the task loud (correct-or-loud).
                    if adm == ForkAdmit::Loud {
                        return false;
                    }
                    // The shared-window ARENA exists only for an AUTOMATIC task (its locals are
                    // per-activation windows). A STATIC task's locals live in the single shared
                    // `static_store` slab, so a Case-B arm touching them would clobber across
                    // (recursive) activations — silent-wrong. Keep static Case-B LOUD.
                    if adm == ForkAdmit::CaseB && !self.func_metas[fid as usize].is_automatic {
                        return false;
                    }
                    // Case A / Case-B-join: keep walking the CONTINUATION after the join (the
                    // arm children are classified by `fork_arms_self_contained` above and run
                    // as their own activities, so they need not be walked here).
                    stack.push(*resume_bb);
                }
                // Phase 4: a nested task `Call` is supported (the engine pushes a nested
                // frame, or runs a subset callee synchronously). Follow `ret_bb` (the
                // continuation in THIS task); the callee lives in another func.
                ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
                ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                    stack.push(*resume)
                }
                ir::Terminator::Goto { target } => stack.push(*target),
                ir::Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                ir::Terminator::Return => {}
            }
        }
        true
    }

    /// B2: THIS module's recursive TASKS (self or mutual). A non-recursive task
    /// (even automatic) is still handled by the byte-identical `inline_task` path;
    /// only recursion needs a frame.
    pub(crate) fn build_task_frame_set(&self) -> std::collections::BTreeSet<String> {
        let mut edges: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
        for (name, t) in &self.task_table {
            let mut callees = std::collections::BTreeSet::new();
            collect_callee_stmt(&t.body, &mut callees);
            callees.retain(|c| self.task_table.contains_key(c));
            edges.insert(name.clone(), callees);
        }
        let mut set = std::collections::BTreeSet::new();
        for (name, t) in &self.task_table {
            // automatic OR recursive (self/mutual). A static non-recursive task —
            // even with control flow OR body-locals — still inlines (the inline
            // path hoists its body-locals under a per-call scope; see
            // `inline_task`). §4.5.198/199 closed the frame ⊂ inline capability gaps
            // ($display, module-net writes incl. array elements, multi-dim locals), so
            // framing a static task no longer loses any capability its LOCAL callers rely on.
            // V2B (§4.5.194): an OUTPUT/INOUT dynamic-array formal MUST be framed — the
            // deep-copy-OUT at exit needs the frame's per-activation heap slot (the inline
            // path has no dyn binding). (An INPUT dyn formal works inline via the R11 alias,
            // so it does not force a frame.)
            // §4.5.200: a task that is the TARGET of a hierarchical enable (`u1.tk(...)`)
            // anywhere in the design MUST be framed so it has a per-instance FuncId for the
            // §4.5.197 hier defer/resolve to bind (a hier call to a still-inlined static task
            // is otherwise loud). Name-based (framing precedes instance elaboration), so an
            // unrelated same-named task is also framed — harmless now that frame ⊇ inline.
            // §4.5.203: a task with a FIXED unpacked-array formal (`byte b[4]`, `int m[2][2]`)
            // MUST be framed — the array formal is an md-packed value slot that only exists on
            // the frame path (the inline static-task binding has no slot, so it was loud). Same
            // frame ⊇ inline safety as the hier force-frame; a rejected array shape stays loud
            // on the frame path (correct-or-loud).
            if t.automatic
                || reaches(name, name, &edges)
                || self.hier_called_task_names.contains(name)
                || t.ports
                    .iter()
                    .any(|p| self.is_output_or_inout_dyn_array_formal(p))
                || t.ports
                    .iter()
                    .any(|p| self.is_fixed_unpacked_array_formal(p))
            {
                set.insert(name.clone());
            }
        }
        set
    }

    /// The set of THIS module's functions needing a frame: every `automatic`
    /// function, plus every function on a recursion cycle (direct or mutual). A
    /// missed call edge is loud-safe (the function stays inline → `inline_stack`
    /// rejects), but the walk covers the common AST shapes.
    pub(crate) fn build_frame_set(&self) -> std::collections::BTreeSet<String> {
        let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        // Callee return widths (a call operand's self-width) for the ㊁ context-width
        // detection — built once so `f = g(x) + 1` sees `g(x)`'s width, not `None`.
        let func_widths: std::collections::HashMap<String, u32> = self
            .func_table
            .iter()
            .filter_map(|(n, f)| ast_func_return_width(f).map(|w| (n.clone(), w)))
            .collect();
        for (name, f) in &self.func_table {
            // R4: a `string`-returning function (IEEE §6.16) is materialized through a
            // frame `NetKind::String` return slot — the frame body writes it (a
            // `$sformatf(...)` render via the engine's `frame_rhs_value`) and
            // `run_frame_call` copies out the string Value; the call is recognized as a
            // string operand (`ir_expr_is_string`). An inline straight-line fold has no
            // string slot, so it MUST be framed regardless of lifetime — force it in.
            if f.ret_string {
                set.insert(name.clone());
                continue;
            }
            // R2: a read-only dyn-array-input function is routed to the INLINE path
            // (where the formal aliases the caller's DynArray net) — do NOT frame it,
            // even though it is `automatic` / `ret_two_state` / has a `Return` / has an
            // unpacked formal (the frame ABI is value-only and cannot carry a dyn
            // handle). The inline fold accepts its straight-line body; a write to the
            // dyn formal / control flow / recursion is excluded by `func_r2_inlinable`
            // (→ stays framed → loud) or by `inline_stack` (recursion → loud).
            if self.func_r2_inlinable(f) {
                continue;
            }
            // A function is framed when it is `automatic`, recursive (below), its
            // body is NOT straight-line foldable (has control flow / a construct the
            // inline path rejects), OR its return type is 2-state (so the frame
            // return slot coerces X/Z→0, §6.11.3 — the inline fold has no slot to
            // coerce). Framing handles all of these with one machine.
            // The ㊁ context-width route additionally requires a SELF-CONTAINED body
            // (`body_reads_only_locals`): a framed call contributes only its ARG
            // expressions to an implicit `always_comb`/`@(*)` sensitivity list, so a
            // body that reads a module-level signal would lose that read from the list
            // if routed (stale/X). Such a function stays INLINE (main's behavior — its
            // reads survive via inlining); the pre-existing inline width bug there is
            // left untouched rather than traded for a sensitivity regression.
            // Both the ㊁ context-width route and the multi-dim-packed-local
            // element-select route (`p[k]` folds as a bit-select on the inline
            // subst value — silently wrong) need the frame path AND a SELF-CONTAINED
            // body (a framed call contributes only its args to an `always_comb`/`@(*)`
            // sensitivity list; a global-reading body would lose that read → stale).
            if f.automatic
                || f.ret_two_state
                || body_needs_frame(&f.body)
                // §13.3 UARR: an unpacked-array formal is lowered ONLY on the frame
                // path (md-packed slot + call-site concat); the inline substitution
                // path has no array binding. Force it to a frame regardless of
                // lifetime so `arr[i]` reads / control flow both work.
                || f.ports.iter().any(|p| !p.unpacked.is_empty())
                || ((body_needs_context_width(f, &func_widths)
                    || body_selects_mdpacked_local(f)
                    // §11.4.7-adjacent, and the same defect class as the shared `case`
                    // scrutinee: the inline fold substitutes a body-local's defining RHS
                    // at every REFERENCE, and one arena node walked N times re-runs its
                    // effects N times: `$random` and `$urandom` drew TWICE where iverilog
                    // draws once, measured, at exit 0. The frame path binds the RHS to a
                    // slot once, so routing is the fix.
                    //
                    // ⚠️ `body_reads_only_locals` below is what makes this safe AND what
                    // limits it: a framed call contributes only its ARGUMENTS to an
                    // implicit `always_comb` sensitivity list, so a body reading a module
                    // net cannot be routed without dropping that read. The two remaining
                    // shapes of this defect — a side-effecting callee firing once per
                    // reference, and an out-of-range element read reporting once per
                    // reference — both sit behind that gate and are unchanged.
                    || body_duplicates_a_non_repeatable_local(f))
                    && body_reads_only_locals(f))
            {
                set.insert(name.clone());
            }
        }
        // call-graph edges restricted to known function names.
        let mut edges: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
        for (name, f) in &self.func_table {
            let mut callees = std::collections::BTreeSet::new();
            collect_callee_stmt(&f.body, &mut callees);
            callees.retain(|c| self.func_table.contains_key(c));
            edges.insert(name.clone(), callees);
        }
        // f needs a frame iff f can reach f (direct or mutual recursion).
        for name in self.func_table.keys() {
            if reaches(name, name, &edges) {
                set.insert(name.clone());
            }
        }
        set
    }

    /// Loud-reject any construct unsupported in a frame function/task body: a
    /// timing/suspend/fork terminator, a non-blocking-assign statement
    /// (`$display`/NBA/force/release), or a blocking-assign lvalue that is not a
    /// WHOLE frame-local net (a module-net write or a part/array select). For
    /// tasks (`allow_call`), a `Terminator::Call` (a nested frame-task call) is
    /// permitted. One diagnostic per offending function (it fails elaboration).
    pub(crate) fn validate_frame_body(
        &mut self,
        name: &str,
        entry: u32,
        net_base: u32,
        locals_len: u32,
        allow_call: bool,
    ) {
        if let Some((what, detail)) =
            self.classify_frame_body(entry, net_base, locals_len, allow_call)
        {
            // R26 §3: the subset clause must attach to the CONSTRUCT, so the construct
            // (`what`) is a short noun phrase and every explanation (`detail`) is
            // appended AFTER it as its own sentence. When R25 §3.2 grew the StrPutC
            // reason into a multi-clause explanation ending "…the same assignment in a
            // `task` body, or in a module process, does work", this `, which is outside
            // the frame-call subset` landed on THAT — i.e. the message said the WORKING
            // form was the unsupported one. Splitting the two makes the mis-attachment
            // unreachable by construction rather than relying on each author to wrap
            // their explanation in parentheses.
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "frame function/task `{name}` body uses {what}, which is outside the \
                     frame-call subset (only blocking assigns to its own locals, \
                     if/else/case/loops, and nested task calls are supported){detail}"
                ),
            );
        }
    }

    /// Is `net` one vita minted for itself (a `$…` scratch: `$ia_tmp$`,
    /// `$repeat_cnt$`, `$sfmt_tmp$`) rather than one the source declared?
    ///
    /// ⚠️ Asked only on an ERROR path, so the scan over `symbols` costs nothing
    /// that matters. It exists so a recurrence of the round-35 P0 names vita
    /// instead of the user's code.
    fn net_is_compiler_internal(&self, net: u32) -> bool {
        self.symbols.iter().any(|(fq, &id)| {
            id == net
                && fq
                    .rsplit('.')
                    .next()
                    .is_some_and(|seg| seg.starts_with('$'))
        })
    }

    /// Round-14 V3/V4: classify a frame body — `None` = SUBSET (blocking-assigns to
    /// its own locals + control flow + nested subset calls, executable synchronously
    /// by `run_task_call`); `Some(reason)` = NON-SUBSET (a timing/suspend/fork
    /// terminator, an NBA/$systask/force/release statement, or an out-of-frame /
    /// part-select / array-element blocking assign). `validate_frame_body` turns
    /// `Some` into the loud E3009 (behaviour unchanged this phase); a later phase
    /// routes a `Some` TASK to the suspendable-frame engine path instead of rejecting.
    pub(crate) fn classify_frame_body(
        &self,
        entry: u32,
        net_base: u32,
        locals_len: u32,
        allow_call: bool,
    ) -> Option<(&'static str, &'static str)> {
        let (lo, hi) = (net_base, net_base + locals_len);
        // `(what, detail)` — `what` is a SHORT NOUN PHRASE naming the rejected construct,
        // with no relative clause of its own (the caller appends ", which is outside the
        // frame-call subset (…)" directly to it), and `detail` is either "" or one or
        // more further sentences starting with ". ". Never grow `what` into an
        // explanation: the caller's clause would then attach to the tail of that
        // explanation instead of to the construct (R26 §3).
        let mut why: Option<(&'static str, &'static str)> = None;
        // Walk only the blocks REACHABLE from this body's entry via its OWN CFG edges
        // (a `Call` terminator follows `ret_bb`, never the callee's entry — the walk
        // stays inside this function). A linear `block_base..func_blocks.len()` scan
        // over-reads into LATER-lowered functions' blocks when this runs in the
        // post-pass (`resolve_frame_task_rejects` fires after every body is lowered, so
        // `len()` is the GLOBAL end): their out-of-frame writes get mis-attributed here,
        // false-rejecting a valid subset task not defined last (ROADMAP §3). Reachable-
        // only is also correct-or-loud: a block skipped here is either another
        // function's or dead code (never executed), so it can only DROP a false reject.
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![entry];
        while let Some(bi) = stack.pop() {
            if !seen.insert(bi) {
                continue;
            }
            let Some(blk) = self.func_blocks.get(bi as usize) else {
                continue;
            };
            let term_ok = matches!(
                blk.term,
                ir::Terminator::Goto { .. }
                    | ir::Terminator::Branch { .. }
                    | ir::Terminator::Return
            ) || (allow_call && matches!(blk.term, ir::Terminator::Call { .. }));
            if !term_ok {
                // R23 §4: name the terminator that actually failed. This arm answered
                // "a timing/suspend/fork control (#delay, @, wait, fork)" for EVERY
                // rejected terminator, including a `Call` in a body lowered with
                // `allow_call = false` — so `function automatic int outer(input int a);
                // r = nxt(a, loc); …` (no timing control anywhere in it) was reported as
                // carrying one. A diagnostic that names a cause the source does not
                // contain sends the reader looking for something that is not there.
                why = Some(if matches!(blk.term, ir::Terminator::Call { .. }) {
                    (
                        "a call to a subroutine with an output/inout formal",
                        ". A FUNCTION body is entered from the expression that calls it, so \
                         it has no call statement of its own to carry the callee's copy-out; \
                         the same call in a `task` body, or in a module process, does work.",
                    )
                } else {
                    ("a timing/suspend/fork control (#delay, @, wait, fork)", "")
                });
            }
            for &sid in &blk.stmts {
                match &self.stmts[sid as usize] {
                    ir::Stmt::BlockingAssign { lhs, .. } => {
                        // A concatenation-target lvalue (`{a,b} = x`) is multi-chunk; the
                        // frame write path (`frame_write_lvalue`) handles a SINGLE chunk
                        // only, so a concat must stay loud (EXT2-H's per-chunk relax must
                        // not admit it — it would panic / silently write chunk 0).
                        if lhs.chunks.len() > 1 {
                            why = Some(("a concatenation-target assignment", ""));
                        }
                        for c in &lhs.chunks {
                            // N7: a class field write (`this.f = v` / `obj.f = v`) is a
                            // HEAP write through a class handle, not a frame-slot write —
                            // allowed regardless of word/in-frame (the handle that
                            // carries it is itself a frame-local or module net).
                            if self.class_handle_nets.contains(&c.net) && c.word.is_some() {
                                continue;
                            }
                            let whole = c.offset.is_none() && c.word.is_none() && c.width.is_none();
                            let in_frame = c.net >= lo && c.net < hi;
                            // V5: an in-frame DYN-ARRAY element write (`loc[i] = v`) routes
                            // to the HEAP (`dyn_write`), not a frame-slot write — allowed
                            // (the frame dyn-array net holds the current activation's array).
                            if in_frame && self.is_dyn_handle_net(c.net) {
                                continue;
                            }
                            // EXT2-H: a bit/part-select write to an IN-FRAME scalar net
                            // (`r[7:0] = x`, `r[i] = b`, an md-packed `p[0] = ..`) is now
                            // supported — the engine read-modify-writes the frame slot.
                            // A WHOLE write must target an in-frame net; a non-whole write
                            // to a net OUTSIDE the function, an ARRAY-element write
                            // (`c.word`), or a select of an UNPACKED-array local (a 1-elem
                            // net — `frame_array_local` — where `mem[k]` mis-lowers to a
                            // bit-select), stays rejected.
                            // MEASURED, not assumed: exempting the `$sformatf` hoist's
                            // `$sfmt_tmp$` scratch nets here (they are written and read
                            // back within one statement sequence, so they LOOK frame-local)
                            // makes the engine panic `frame lvalue net is routed` — the
                            // frame-function executor genuinely cannot write a module net.
                            // The gate is right; the hoist is what must not run there.
                            if whole {
                                if !in_frame {
                                    // ⚠️ Which HALF of this sentence is true matters. The
                                    // P0 this guard was added for (aes_top round-35) was a
                                    // COMPILER-generated capture net handed to the wrong
                                    // frame, and the message blamed the user's `case` — a
                                    // construct the same sentence lists as supported. The
                                    // reporter spent thirty minutes on the wrong half.
                                    // The source cannot avoid an internal net, so say so.
                                    why = Some(if self.net_is_compiler_internal(c.net) {
                                        (
                                            "an assignment to a COMPILER-GENERATED net                                              outside the function",
                                            ". That net is vita's, not the source's — this                                              is a vita bug; please report it",
                                        )
                                    } else {
                                        ("an assignment to a net outside the function", "")
                                    });
                                }
                            } else if c.word.is_some()
                                || !in_frame
                                || self.frame_array_local.contains(&c.net)
                            {
                                why = Some(("a part-select / array-element assignment", ""));
                            }
                        }
                    }
                    // B3: `disable <enclosing named block>` (the break/continue
                    // idiom) lowers to a no-op `Disable{Scope}` marker PLUS a `Goto`
                    // to the block's exit — the Goto does the unwinding and is
                    // honored by the frame BB loop; the marker is a pure no-op in
                    // `run_frame_call`/`run_task` (only BlockingAssign mutates). A
                    // `disable fork` (`Fork`) has no fork to target in a frame body,
                    // so it falls through to the loud arm.
                    ir::Stmt::Disable {
                        scope_kind: ir::DisableKind::Scope,
                        ..
                    } => {}
                    // V5 (§4.5.194): a `new[]` / `delete()` on an IN-FRAME dyn-array net
                    // (`loc = new[n]`, `loc.delete()`) is a heap op the frame executors now
                    // perform (args[0] is the handle Signal). A target OUTSIDE the frame or a
                    // non-dyn handle falls through to the loud arm below (correct-or-loud).
                    ir::Stmt::SysTask {
                        which: ir::SysTaskId::DynNew | ir::SysTaskId::DynDelete,
                        args,
                        ..
                    } if args.first().is_some_and(|&a| {
                        matches!(
                            &self.exprs[a as usize],
                            ir::Expr::Signal { net, .. }
                                if *net >= lo && *net < hi && self.is_dyn_handle_net(*net)
                        )
                    }) => {}
                    // Family C (r17): a dyn-array-formal SNAPSHOT marker (a no-op
                    // Display whose sid is in `dyn_formal_marker_stmts`) emitted for a
                    // `x = f(arr)` call inside this frame body — the `&self` executors
                    // run it (heap→heap deep-copy via the interior-mutable `dyn_heap`).
                    // Restricted to the dyn-formal set, so a §7.10 whole-handle copy
                    // Display stays loud here (it is NOT in the set).
                    ir::Stmt::SysTask {
                        which: ir::SysTaskId::Display,
                        ..
                    } if self.dyn_formal_marker_stmts.contains(&sid) => {}
                    // Family D (r17): a genuine `$display`/`$write` print in a subset
                    // function/task body — the `&self` executors render it (iverilog
                    // runs a $display in a function). A severity/timeformat/stage/marker
                    // Display is NOT in `frame_print_stmts` → stays loud (correct-or-loud).
                    ir::Stmt::SysTask {
                        which: ir::SysTaskId::Display | ir::SysTaskId::Write,
                        ..
                    } if self.frame_print_stmts.contains(&sid) => {}
                    // r18 (F2): a SEVERITY task (`$info`/`$warning`/`$error`/`$fatal`, lowered
                    // as a `Display` + a `severities` sidecar). The `&self` frame executors
                    // render it to the diag stream (`frame_emit_severity`; `$error`/`$fatal`
                    // latch `had_error`/`call_fatal` via interior mutability). This makes a
                    // `unique`/`priority case` — whose synthesized no-match arm is a
                    // `$warning` — usable in a frame function/subset-task body, and any
                    // explicit severity call there. A timeformat/stage/assert-ctl Display is
                    // NOT in `severities`, so it still hits the loud arm (correct-or-loud).
                    ir::Stmt::SysTask {
                        which: ir::SysTaskId::Display,
                        ..
                    } if self.severities.contains_key(&sid) => {}
                    // R25 §3.2: name the statement that ACTUALLY failed. This arm
                    // answered with a four-way list for every rejected statement, and
                    // for the commonest one — `s[i] = v` on a `string`, which lowers to
                    // a `StrPutC` system task — the source contains NONE of the four.
                    // The reader is told the body "uses a $systask / nonblocking / force
                    // / release statement" while looking at a line the same message
                    // calls supported ("blocking assigns to its own locals"). Same
                    // defect class as R23 §4 on the terminator arm, and the same fix:
                    // report what is there.
                    ir::Stmt::SysTask {
                        which: ir::SysTaskId::StrPutC,
                        ..
                    } => {
                        why = Some((
                            "a string element assignment (`s[i] = v`)",
                            ". It lowers to a system task, and a FUNCTION body is entered from \
                             the expression that calls it, so it has no call statement of its \
                             own to carry the write out; \
                             the same assignment in a `task` body, or in a module process, \
                             does work.",
                        ))
                    }
                    ir::Stmt::NonblockingAssign { .. } => {
                        why = Some(("a nonblocking assignment (`<=`)", ""))
                    }
                    ir::Stmt::Force { .. } => why = Some(("a `force` statement", "")),
                    ir::Stmt::Release { .. } => why = Some(("a `release` statement", "")),
                    ir::Stmt::SysTask { .. } => why = Some(("a system task call", "")),
                    _ => why = Some(("a $systask / nonblocking / force / release statement", "")),
                }
            }
            // Follow this block's OWN successor edges (a `Call` stays in-function via
            // `ret_bb`; a `Fork`/`Return` has no in-body successor to classify — and a
            // `Fork` already set `why` above, so not descending into it can't flip the
            // verdict from reject to accept).
            match &blk.term {
                ir::Terminator::Goto { target } => stack.push(*target),
                ir::Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
                ir::Terminator::Delay { resume, .. } => stack.push(*resume),
                ir::Terminator::Wait { resume, .. } => stack.push(*resume),
                ir::Terminator::Fork { .. } | ir::Terminator::Return => {}
            }
        }
        why
    }

    /// Is `net` a FRAME-LOCAL (an automatic/frame task or function's local slot,
    /// in some `FuncMeta`'s `[base_net, base_net+locals_len)` range)? The engine's
    /// frame copy-out (`frame_write_lvalue`) can only write a WHOLE frame-local
    /// net, so a part/bit/indexed SELECT of a frame-local as an output/inout actual
    /// must be loud-rejected rather than fed to a select lvalue it cannot route.
    pub(crate) fn net_is_frame_local(&self, net: u32) -> bool {
        self.func_metas
            .iter()
            .any(|m| net >= m.base_net && net < m.base_net + m.locals_len)
    }
}
