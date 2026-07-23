//! AST read-set queries — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Substitute SVA formal identifiers with their bound actual expressions (slice A1).
/// `map` is `formal-name → actual-expr`; a single-segment `Ident` whose name is a
/// formal is replaced by a clone of the actual (carrying the actual's span). Every
/// other node is rebuilt structurally so the substitution reaches nested operands.
/// Pure AST rewrite — used to inline `sequence s(x,y); …` at `s(a,b)` (IR-0).
pub(crate) fn subst_expr(e: &ast::Expr, map: &BTreeMap<String, ast::Expr>) -> ast::Expr {
    use ast::ExprKind as K;
    // A formal occurrence (a bare single-segment name that is a key) → the actual.
    if let K::Ident(p) = &e.kind {
        if p.segments.len() == 1 {
            if let Some(actual) = map.get(&p.segments[0].name) {
                return actual.clone();
            }
        }
    }
    let sp = e.span;
    let kind = match &e.kind {
        K::Unary { op, operand } => K::Unary {
            op: *op,
            operand: Box::new(subst_expr(operand, map)),
        },
        K::Binary { op, lhs, rhs } => K::Binary {
            op: *op,
            lhs: Box::new(subst_expr(lhs, map)),
            rhs: Box::new(subst_expr(rhs, map)),
        },
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => K::Ternary {
            cond: Box::new(subst_expr(cond, map)),
            then_e: Box::new(subst_expr(then_e, map)),
            else_e: Box::new(subst_expr(else_e, map)),
        },
        K::BitSelect { base, index } => K::BitSelect {
            base: Box::new(subst_expr(base, map)),
            index: Box::new(subst_expr(index, map)),
        },
        K::PartSelect { base, msb, lsb } => K::PartSelect {
            base: Box::new(subst_expr(base, map)),
            msb: Box::new(subst_expr(msb, map)),
            lsb: Box::new(subst_expr(lsb, map)),
        },
        K::IndexedPart {
            base,
            offset,
            width,
            dir,
        } => K::IndexedPart {
            base: Box::new(subst_expr(base, map)),
            offset: Box::new(subst_expr(offset, map)),
            width: Box::new(subst_expr(width, map)),
            dir: *dir,
        },
        K::Concat { parts } => K::Concat {
            parts: parts.iter().map(|x| subst_expr(x, map)).collect(),
        },
        K::Replicate { count, value } => K::Replicate {
            count: Box::new(subst_expr(count, map)),
            value: value.iter().map(|x| subst_expr(x, map)).collect(),
        },
        K::Call { name, args } => K::Call {
            name: name.clone(),
            args: args.iter().map(|x| subst_expr(x, map)).collect(),
        },
        K::SysCall { name, args } => K::SysCall {
            name: name.clone(),
            args: args.iter().map(|x| subst_expr(x, map)).collect(),
        },
        K::Paren { inner } => K::Paren {
            inner: Box::new(subst_expr(inner, map)),
        },
        K::MinTypMax { min, typ, max } => K::MinTypMax {
            min: Box::new(subst_expr(min, map)),
            typ: Box::new(subst_expr(typ, map)),
            max: Box::new(subst_expr(max, map)),
        },
        // literals, pkg-scoped, multi-segment ident, new, dollar, error: no formal
        // occurrence to rewrite — clone verbatim.
        _ => e.kind.clone(),
    };
    ast::Expr { kind, span: sp }
}

/// Substitute every bare single-segment identifier named `name` in `e` with a clone
/// of `repl` (slice N2c — substitute a local-variable READ with its data register
/// read / captured value). Structural rebuild so the substitution reaches nested
/// operands. A hierarchical `u.name` is NOT a match (a different signal).
pub(crate) fn subst_ident_expr(e: &ast::Expr, name: &str, repl: &ast::Expr) -> ast::Expr {
    let mut map = BTreeMap::new();
    map.insert(name.to_string(), repl.clone());
    subst_expr(e, &map)
}

/// Substitute SVA formals (slice A1) through a clocking event's expressions (a
/// formal may name the clock signal of a parameterized property).
pub(crate) fn subst_sensitivity(
    s: &ast::Sensitivity,
    map: &BTreeMap<String, ast::Expr>,
) -> ast::Sensitivity {
    match s {
        ast::Sensitivity::Star => ast::Sensitivity::Star,
        ast::Sensitivity::List(evs) => ast::Sensitivity::List(
            evs.iter()
                .map(|ev| ast::EventExpr {
                    edge: ev.edge,
                    expr: subst_expr(&ev.expr, map),
                    iff: ev.iff.as_ref().map(|e| subst_expr(e, map)),
                    span: ev.span,
                })
                .collect(),
        ),
    }
}

/// B1 frame-call: collect single-segment user-function/task call names reachable
/// from a statement (for recursion detection). Incomplete coverage is loud-safe
/// (a missed edge leaves the function on the inline path → `inline_stack` rejects).
pub(crate) fn collect_callee_stmt(s: &ast::Stmt, out: &mut std::collections::BTreeSet<String>) {
    use ast::Stmt::*;
    match s {
        Blocking { rhs, .. } => collect_callee_expr(rhs, out),
        NonBlocking { rhs, .. } => collect_callee_expr(rhs, out),
        If {
            cond,
            then_s,
            else_s,
            ..
        } => {
            collect_callee_expr(cond, out);
            collect_callee_stmt(then_s, out);
            if let Some(e) = else_s {
                collect_callee_stmt(e, out);
            }
        }
        Case {
            scrutinee, items, ..
        } => {
            collect_callee_expr(scrutinee, out);
            for it in items {
                match it {
                    ast::CaseItem::Match { labels, body, .. } => {
                        for g in labels {
                            collect_callee_expr(g, out);
                        }
                        collect_callee_stmt(body, out);
                    }
                    ast::CaseItem::Default { body, .. } => collect_callee_stmt(body, out),
                }
            }
        }
        For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            collect_callee_stmt(init, out);
            collect_callee_expr(cond, out);
            collect_callee_stmt(step, out);
            collect_callee_stmt(body, out);
        }
        While { cond, body, .. } => {
            collect_callee_expr(cond, out);
            collect_callee_stmt(body, out);
        }
        Repeat { count, body, .. } => {
            collect_callee_expr(count, out);
            collect_callee_stmt(body, out);
        }
        Forever { body, .. } => collect_callee_stmt(body, out),
        Block { stmts, .. } | Fork { stmts, .. } => {
            for st in stmts {
                collect_callee_stmt(st, out);
            }
        }
        SysTaskCall { args, .. } => {
            for a in args {
                collect_callee_expr(a, out);
            }
        }
        UserTaskCall { name, args, .. } => {
            if name.segments.len() == 1 {
                out.insert(name.segments[0].name.clone());
            }
            for a in args {
                collect_callee_expr(a, out);
            }
        }
        _ => {}
    }
}

/// B1 frame-call: collect call names reachable from an expression (companion to
/// [`collect_callee_stmt`]).
pub(crate) fn collect_callee_expr(e: &ast::Expr, out: &mut std::collections::BTreeSet<String>) {
    use ast::ExprKind::*;
    match &e.kind {
        Call { name, args } => {
            if name.segments.len() == 1 {
                out.insert(name.segments[0].name.clone());
            }
            for a in args {
                collect_callee_expr(a, out);
            }
        }
        Unary { operand, .. } => collect_callee_expr(operand, out),
        Binary { lhs, rhs, .. } => {
            collect_callee_expr(lhs, out);
            collect_callee_expr(rhs, out);
        }
        Ternary {
            cond,
            then_e,
            else_e,
        } => {
            collect_callee_expr(cond, out);
            collect_callee_expr(then_e, out);
            collect_callee_expr(else_e, out);
        }
        BitSelect { base, index } => {
            collect_callee_expr(base, out);
            collect_callee_expr(index, out);
        }
        PartSelect { base, msb, lsb } => {
            collect_callee_expr(base, out);
            collect_callee_expr(msb, out);
            collect_callee_expr(lsb, out);
        }
        IndexedPart {
            base,
            offset,
            width,
            ..
        } => {
            collect_callee_expr(base, out);
            collect_callee_expr(offset, out);
            collect_callee_expr(width, out);
        }
        Concat { parts } => {
            for p in parts {
                collect_callee_expr(p, out);
            }
        }
        Replicate { count, value } => {
            collect_callee_expr(count, out);
            for v in value {
                collect_callee_expr(v, out);
            }
        }
        SysCall { args, .. } => {
            for a in args {
                collect_callee_expr(a, out);
            }
        }
        Paren { inner } => collect_callee_expr(inner, out),
        MinTypMax { min, typ, max } => {
            collect_callee_expr(min, out);
            collect_callee_expr(typ, out);
            collect_callee_expr(max, out);
        }
        New { size, src } => {
            collect_callee_expr(size, out);
            if let Some(s) = src {
                collect_callee_expr(s, out);
            }
        }
        _ => {}
    }
}

impl Elaborator<'_> {
    /// Conservative aliasing walk: does `e` read net `target` through any
    /// name it mentions? (Names resolve exactly like the lowering would —
    /// scoped lookup incl. dotted interface aliases — so an index variable
    /// `i` never false-positives.)
    pub(crate) fn expr_reads_net(&self, e: &ast::Expr, target: u32) -> bool {
        match &e.kind {
            ast::ExprKind::Ident(p) => {
                let name = p
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                self.lookup_net_scoped(&name) == Some(target)
            }
            ast::ExprKind::BitSelect { base, index } => {
                self.expr_reads_net(base, target) || self.expr_reads_net(index, target)
            }
            ast::ExprKind::Paren { inner } => self.expr_reads_net(inner, target),
            ast::ExprKind::Unary { operand, .. } => self.expr_reads_net(operand, target),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.expr_reads_net(lhs, target) || self.expr_reads_net(rhs, target)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_reads_net(cond, target)
                    || self.expr_reads_net(then_e, target)
                    || self.expr_reads_net(else_e, target)
            }
            ast::ExprKind::PartSelect { base, msb, lsb } => {
                self.expr_reads_net(base, target)
                    || self.expr_reads_net(msb, target)
                    || self.expr_reads_net(lsb, target)
            }
            ast::ExprKind::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.expr_reads_net(base, target)
                    || self.expr_reads_net(offset, target)
                    || self.expr_reads_net(width, target)
            }
            ast::ExprKind::Concat { parts } => parts.iter().any(|p| self.expr_reads_net(p, target)),
            ast::ExprKind::Replicate { count, value } => {
                self.expr_reads_net(count, target)
                    || value.iter().any(|p| self.expr_reads_net(p, target))
            }
            // A user function body could read anything — conservative TRUE
            // keeps the guard sound (loud beats a silently moved index).
            ast::ExprKind::Call { .. } => true,
            // V2005-compat: with a net literally named `new` in scope,
            // `new[i]` lowers as a READ of that net (adversarial find #3) —
            // check the fallback target and walk the children.
            ast::ExprKind::New { size, src } => {
                self.lookup_net_scoped("new") == Some(target)
                    || self.expr_reads_net(size, target)
                    || src.as_ref().is_some_and(|s| self.expr_reads_net(s, target))
            }
            ast::ExprKind::MinTypMax { min, typ, max } => {
                self.expr_reads_net(min, target)
                    || self.expr_reads_net(typ, target)
                    || self.expr_reads_net(max, target)
            }
            ast::ExprKind::SysCall { args, .. } => {
                args.iter().any(|a| self.expr_reads_net(a, target))
            }
            _ => false,
        }
    }

    /// True if a replication-count expression reads a runtime VARIABLE net (a
    /// `logic`/`reg`/`int` signal) — i.e. it is NOT a compile-time constant.
    /// IEEE §11.4.12.2 requires the count to be a constant expression; a runtime
    /// count otherwise lowers to a net the engine folds to 0 → silent 0-width.
    /// A param / localparam / genvar / `$clog2` / const-function / literal reads
    /// NO net (params live in `self.params`, not the net table), so this returns
    /// false and the existing lowering is kept byte-identical. CONSERVATIVE
    /// toward false: an unhandled shape keeps the old lowering (pre-existing
    /// behavior — never a new over-reject of valid code).
    pub(crate) fn count_reads_runtime_net(&self, e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Ident(p) => {
                let name = p
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                // Mirror lower_expr's resolution precedence (subst → out_subst →
                // params → net): a name bound as an inline-function local/formal,
                // or a constant (param / localparam / genvar / enum-label), is NOT
                // a runtime net — even when an outer-scope net of the same name is
                // shadowed by it. Only a genuine runtime-variable read is flagged
                // (else a valid `{N{…}}` with a shadowing inner localparam is
                // wrongly loud-rejected — adversarial soundness find).
                self.subst_lookup(&name).is_none()
                    && self.out_subst_lookup(&name).is_none()
                    && self.lookup_scoped(&name).is_none()
                    && self.lookup_net_scoped(&name).is_some()
            }
            ast::ExprKind::Paren { inner } => self.count_reads_runtime_net(inner),
            ast::ExprKind::Unary { operand, .. } => self.count_reads_runtime_net(operand),
            ast::ExprKind::Binary { lhs, rhs, .. } => {
                self.count_reads_runtime_net(lhs) || self.count_reads_runtime_net(rhs)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.count_reads_runtime_net(cond)
                    || self.count_reads_runtime_net(then_e)
                    || self.count_reads_runtime_net(else_e)
            }
            ast::ExprKind::BitSelect { base, index } => {
                self.count_reads_runtime_net(base) || self.count_reads_runtime_net(index)
            }
            ast::ExprKind::PartSelect { base, msb, lsb } => {
                self.count_reads_runtime_net(base)
                    || self.count_reads_runtime_net(msb)
                    || self.count_reads_runtime_net(lsb)
            }
            ast::ExprKind::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.count_reads_runtime_net(base)
                    || self.count_reads_runtime_net(offset)
                    || self.count_reads_runtime_net(width)
            }
            ast::ExprKind::Concat { parts } => {
                parts.iter().any(|p| self.count_reads_runtime_net(p))
            }
            ast::ExprKind::Replicate { count, value } => {
                self.count_reads_runtime_net(count)
                    || value.iter().any(|p| self.count_reads_runtime_net(p))
            }
            ast::ExprKind::Cast { expr, .. } => self.count_reads_runtime_net(expr),
            ast::ExprKind::MinTypMax { min, typ, max } => {
                self.count_reads_runtime_net(min)
                    || self.count_reads_runtime_net(typ)
                    || self.count_reads_runtime_net(max)
            }
            // A CONSTANT function's result is constant iff its ARGUMENTS are (a
            // runtime-net argument makes the count runtime — iverilog rejects
            // it). Unlike `expr_reads_net`'s conservative `Call => true`, a
            // const-function call with constant args (`{f(3){…}}`) must NOT be
            // flagged, so walk the arguments only (a body reading a module net
            // stays the pre-existing silent-0 — a rare, no-regression miss).
            ast::ExprKind::Call { args, .. } => {
                args.iter().any(|a| self.count_reads_runtime_net(a))
            }
            ast::ExprKind::SysCall { name, args } => {
                // The type/shape-query system functions are ELABORATION CONSTANTS
                // regardless of a net operand (they read the operand's TYPE/shape,
                // not its runtime value): `{$bits(net){…}}` is a constant count.
                // Every other sysfunc (`$clog2`, `$signed`, …) is runtime iff its
                // arguments are, so recurse into those. (adversarial differential
                // find: `$bits`/`$size`/… of a net were wrongly loud-rejected.)
                let is_type_query = matches!(
                    name.name.as_str(),
                    "$bits"
                        | "$size"
                        | "$high"
                        | "$low"
                        | "$left"
                        | "$right"
                        | "$increment"
                        | "$dimensions"
                        | "$unpacked_dimensions"
                        | "$isunbounded"
                        | "$typename"
                );
                !is_type_query && args.iter().any(|a| self.count_reads_runtime_net(a))
            }
            _ => false,
        }
    }

    /// Read-set of a lowered process body: every net referenced on a RHS or a
    /// branch condition (LHS write targets are NOT reads). Drives implicit
    /// `@*`/`always_comb` sensitivity. Deterministic ascending net order.
    pub(crate) fn comb_read_set(&self, body: &[ir::BasicBlock]) -> Vec<u32> {
        let mut reads = std::collections::BTreeSet::new();
        for bb in body {
            for &sid in &bb.stmts {
                match &self.stmts[sid as usize] {
                    ir::Stmt::BlockingAssign { lhs, rhs }
                    | ir::Stmt::NonblockingAssign { lhs, rhs, .. } => {
                        // The LHS dynamic INDEX sub-exprs (`mem[sel] = …`,
                        // `mask[idx*8 +: 8] = …`) are reads: the block must
                        // re-fire when the index changes, not only when a RHS
                        // signal does. The written base net is NOT a read.
                        self.collect_lval_reads(lhs, &mut reads);
                        self.collect_expr_reads(*rhs, &mut reads);
                    }
                    ir::Stmt::SysTask { fmt, args, .. } => {
                        if let Some(f) = fmt {
                            self.collect_expr_reads(*f, &mut reads);
                        }
                        for &a in args {
                            self.collect_expr_reads(a, &mut reads);
                        }
                    }
                    ir::Stmt::Disable { .. } => {}
                    // shape-reserved at format_version 4 (never lowered yet); a
                    // force RHS / LHS-index would be a read when force lands. The
                    // `Release` LHS index is symmetric (latent — both loud-rejected).
                    ir::Stmt::Force { lhs, rhs } => {
                        self.collect_lval_reads(lhs, &mut reads);
                        self.collect_expr_reads(*rhs, &mut reads);
                    }
                    ir::Stmt::Release { lhs } => {
                        self.collect_lval_reads(lhs, &mut reads);
                    }
                }
            }
            if let ir::Terminator::Branch { cond, .. } = &bb.term {
                self.collect_expr_reads(*cond, &mut reads);
            }
        }
        reads.into_iter().collect()
    }

    /// Recursively collect every `Signal` net read by expression `eid`.
    pub(crate) fn collect_expr_reads(&self, eid: u32, reads: &mut std::collections::BTreeSet<u32>) {
        match &self.exprs[eid as usize] {
            ir::Expr::Const { .. } => {}
            ir::Expr::Signal { net, word } => {
                reads.insert(*net);
                // The array WORD index is itself a read: `always_comb y = mem[sel]`
                // must re-fire when `sel` (or any signal in a multi-dim flat index
                // `i*ncols+j`) changes, not only when the memory changes. Symmetric
                // with the `Select` arm recursing into its offset.
                if let Some(weid) = word {
                    self.collect_expr_reads(*weid, reads);
                }
            }
            ir::Expr::Select {
                base,
                offset,
                width,
                ..
            } => {
                self.collect_expr_reads(*base, reads);
                self.collect_expr_reads(*offset, reads);
                self.collect_expr_reads(*width, reads);
            }
            ir::Expr::Concat { parts } => {
                for &p in parts {
                    self.collect_expr_reads(p, reads);
                }
            }
            ir::Expr::Replicate { count, value } => {
                self.collect_expr_reads(*count, reads);
                self.collect_expr_reads(*value, reads);
            }
            ir::Expr::Unary { operand, .. } => self.collect_expr_reads(*operand, reads),
            ir::Expr::Binary { lhs, rhs, .. } => {
                self.collect_expr_reads(*lhs, reads);
                self.collect_expr_reads(*rhs, reads);
            }
            ir::Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.collect_expr_reads(*cond, reads);
                self.collect_expr_reads(*then_e, reads);
                self.collect_expr_reads(*else_e, reads);
            }
            ir::Expr::SysFunc { args, .. } => {
                for &a in args {
                    self.collect_expr_reads(a, reads);
                }
            }
            // A frame function CALL: its argument expressions are evaluated in the
            // caller's scope, so their nets belong in an implicit `@(*)`/`always_comb`
            // sensitivity list — same as a `SysFunc`. Without this, a comb block whose
            // only reads reach a framed function through its args (`y = f(a, b)`) never
            // re-fires when the args change (silent stale/X). (`func` is a callee index,
            // not an expression.)
            ir::Expr::Call { args, .. } => {
                for &a in args {
                    self.collect_expr_reads(a, reads);
                }
            }
            // ⓑ-breadth (v17): the with-clause iterator reads the engine scratch,
            // not a net — no sensitivity contribution.
            ir::Expr::ArrayItem { .. } => {}
        }
    }
}
