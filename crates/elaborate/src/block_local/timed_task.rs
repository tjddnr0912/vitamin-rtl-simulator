//! §5.2 row 72: a block-local declared under a timing control in a SUBROUTINE body.
//!
//! Row 72 hoists and scopes such a declaration in a PROCESS body (an `initial` / `always`
//! of a module, generate block or interface, a fork arm, a loop) exactly as the same block
//! with no timing control above it. In a task or function body it is refused instead, one
//! E3009 per declaration, wherever the body is elaborated (the inline lane of a static
//! task, the frame lane of an `automatic`, package, interface or hierarchically called
//! task, a function). The no-timing twin of such a block shows why: a subroutine's
//! block-locals have no per-scope storage and no positional name binding yet (a static
//! task's argument substitution over the whole body, the frame-slot coalesce, a static
//! task's once-only `automatic` initializer, a package routine's routine-wide constant
//! skip list), so four review rounds each found another name kind or read position where
//! a name rule let a wrong value through. Before row 72 the declaration was dropped and its
//! name bound the task's or the caller's same-named variable (often a silent wrong value),
//! or was E3010. Lifting the refusal is §5.2 row 72b.

use super::*;

/// Every declaration in a declaring block under a timing control (`#d`, `@(e)`,
/// `wait (c)`) in `s`, at any depth below it, by its span.
pub(crate) fn timing_nested_decls(s: &ast::Stmt) -> BTreeSet<(u32, u32)> {
    fn walk(s: &ast::Stmt, under: bool, out: &mut BTreeSet<(u32, u32)>) {
        if under {
            if let ast::Stmt::Block { decls, .. } | ast::Stmt::Fork { decls, .. } = s {
                out.extend(decls.iter().map(|d| (d.span.lo, d.span.hi)));
            }
        }
        let under = under || is_timing_control(s);
        for_each_nested_stmt(s, |st, _| walk(st, under, out));
    }
    let mut out = BTreeSet::new();
    walk(s, false, &mut out);
    out
}

/// The name the source wrote for a declaration: the parser renames a `for (int i …)`
/// loop variable to `__forvar_i_<offset>` and a `foreach` index to `__foreach_k_<offset>`
/// (`hdl-parser` `stmt_ctl.rs`) so neither aliases another; a diagnostic names `i` / `k`.
pub(crate) fn source_name(n: &str) -> &str {
    n.strip_prefix("__forvar_")
        .or_else(|| n.strip_prefix("__foreach_"))
        .and_then(|rest| rest.rsplit_once('_'))
        .map_or(n, |(name, _)| name)
}

/// The routine whose body a block-local is being reserved in.
pub(crate) struct RoutineView<'a> {
    /// The routine's own name, for the message (a `function void` reaches elaborate as
    /// a task, so the message names the routine, not its kind).
    pub(crate) name: &'a str,
    pub(crate) body: &'a ast::Stmt,
}

impl Elaborator<'_> {
    /// Refuse `d` when it is a declaration under a timing control (`timed`, from
    /// [`timing_nested_decls`] of `view.body`). Once per declaration, anchored at it: a
    /// static task is inlined at every call site, so its declarations are met once per
    /// call.
    pub(crate) fn refuse_timing_block_local_hazard(
        &mut self,
        view: &RoutineView,
        timed: &BTreeSet<(u32, u32)>,
        d: &ast::NetVarDecl,
    ) {
        let key = (d.span.lo, d.span.hi);
        if !timed.contains(&key) || !self.reported_timing_block_locals.insert(key) {
            return;
        }
        let names: Vec<String> = d
            .names
            .iter()
            .map(|n| format!("`{}`", source_name(&n.name.name)))
            .collect();
        let routine = view.name;
        self.error_at(
            MsgCode::ElabUnsupported,
            d.span,
            &format!(
                "declaration of {} under a timing control (`#d`, `@(e)`, `wait (c)`) in the \
                 body of subroutine `{routine}`: a declaration under a timing control is \
                 supported in a process, not yet in a task or function — declare it at the top \
                 of `{routine}`",
                names.join(", ")
            ),
        );
    }
}
