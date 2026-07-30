//! r19 follow-on — the general hoister's QUERY helpers, split from `hoist/general.rs` to
//! hold it under the 1000-line module cap.
//!
//! Nothing here transforms anything. These answer the questions the walkers ask: which roots
//! a call WRITES (named-argument aware), whether a hierarchical read can ALIAS a bare local,
//! which actuals of a call statement are reads, and whether we are inside a frame body.

use super::general::{shape, Shape};
use super::*;

impl Elaborator<'_> {
    /// Are we lowering the body of a frame function or a frame task? Writes there must
    /// target frame-local nets, so no copy-out to a module net may be emitted.
    pub(crate) fn in_frame_body(&self) -> bool {
        self.frame_fn_lowering || self.frame_task_lowering
    }
}

impl Elaborator<'_> {
    /// Every root an output/inout ACTUAL of any output-formal call in `e` writes — the
    /// `shape`-driven twin of `collect_inout_mutated`, which only descends
    /// Unary/Binary/Paren/Ternary and so misses a call reached through a concat, another
    /// call's argument list, a select index, or a `?:` arm.
    ///
    /// Named arguments are resolved to their formals (IEEE 1800 §13.5.4) before the
    /// direction check: keying on POSITION made a `.formal(o)` output actual invisible, so
    /// `o` was neither a hazard candidate nor counted by the two-calls guard.
    pub(crate) fn collect_mutated_deep(&self, e: &ast::Expr, out: &mut BTreeSet<String>) {
        if self.inout_call_target(e).is_some() {
            if let ast::ExprKind::Call { name, args } = &e.kind {
                match self.callee_arg_dirs(name, args) {
                    Some(binds) => {
                        for (dir, a) in binds {
                            if !matches!(dir, ast::PortDir::Input) {
                                if let Some(root) = expr_root_ident(named_arg_value(a)) {
                                    out.insert(root);
                                }
                            }
                        }
                    }
                    // Directions unresolvable ⇒ assume every actual may be written, so the
                    // opacity checks that consume `candidates` still get something to ask
                    // about (the alternative, an empty set, is the hole this fixes).
                    None => {
                        for a in args {
                            if let Some(root) = expr_root_ident(named_arg_value(a)) {
                                out.insert(root);
                            }
                        }
                    }
                }
                // A nested output-formal call in an argument writes too.
                for a in args {
                    self.collect_mutated_deep(a, out);
                }
            }
            return;
        }
        for c in shape_all_children(e) {
            self.collect_mutated_deep(c, out);
        }
    }
}

/// Unwrap a `.formal(value)` named argument to the value it carries; any other expression is
/// itself. Without this, `expr_root_ident` sees a `NamedArg` node and answers `None`.
pub(crate) fn named_arg_value(e: &ast::Expr) -> &ast::Expr {
    match &e.kind {
        ast::ExprKind::NamedArg { value: Some(v), .. } => v,
        _ => e,
    }
}

/// Every child `shape` models, INCLUDING the ones that are not hoist sites — for walks that
/// only need to find things (a mutated root, a call), never to rewrite.
pub(crate) fn shape_all_children(e: &ast::Expr) -> Vec<&ast::Expr> {
    match shape(e) {
        Shape::Uncond(cs) | Shape::NoHoist(cs) | Shape::Unevaluated(cs) => cs,
        Shape::ShortCircuit { lhs, rhs, .. } => vec![lhs, rhs],
        Shape::Ternary {
            cond,
            then_e,
            else_e,
        } => vec![cond, then_e, else_e],
    }
}

impl Elaborator<'_> {
    /// The root names an output-formal call `e` WRITES through its output/inout actuals.
    ///
    /// Named-argument aware (IEEE 1800 §13.5.4): `collect_inout_mutated` zips formals with
    /// actuals by POSITION, so a `.formal(o)` output actual contributed nothing — `o` was
    /// neither a hazard candidate nor counted by the two-calls-one-root guard, and the
    /// eval-order analysis silently read the post-call value.
    pub(crate) fn mutated_roots_of_call(&self, e: &ast::Expr) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let ast::ExprKind::Call { name, args } = &e.kind else {
            return out;
        };
        match self.callee_arg_dirs(name, args) {
            Some(binds) => {
                for (dir, a) in binds {
                    if !matches!(dir, ast::PortDir::Input) {
                        if let Some(root) = expr_root_ident(named_arg_value(a)) {
                            out.insert(root);
                        }
                    }
                }
            }
            // Directions unresolvable ⇒ assume any actual may be written (conservative: it
            // can only add hazards, never hide one).
            None => {
                for a in args {
                    if let Some(root) = expr_root_ident(named_arg_value(a)) {
                        out.insert(root);
                    }
                }
            }
        }
        out
    }

    /// Does the hierarchical path `segs` name something in the CURRENT scope (a self-path
    /// like `t.o` / `t.blk.o`), rather than reaching into a child instance?
    ///
    /// Only a self-path can alias a bare block-local: v1 flattens block-locals to module
    /// nets by bare name, so `t.o` and `o` are one net, while `sub.o` is a different module's
    /// net and cannot be.
    pub(crate) fn hier_path_is_self(&self, segs: &[ast::Ident]) -> bool {
        let Some(head) = segs.first() else {
            return false;
        };
        self.cur_prefix
            .split('.')
            .map(|c| c.split('[').next().unwrap_or(c))
            .any(|c| c == head.name)
    }
}

impl Elaborator<'_> {
    /// The `input` actuals of a user task/function call STATEMENT, in order — the ones that
    /// are reads and may therefore be hoisted from. `None` when the callee's directions
    /// cannot be resolved, which stands the statement down (an unresolved direction could be
    /// an output, and rewriting a write destination loses the write).
    pub(crate) fn task_call_input_args<'a>(
        &self,
        name: &ast::HierPath,
        args: &'a [ast::Expr],
    ) -> Option<Vec<&'a ast::Expr>> {
        let binds = self.callee_arg_dirs(name, args)?;
        // `callee_arg_dirs` may reorder / default-fill, so map back by identity of the actual
        // rather than by position.
        Some(
            args.iter()
                .enumerate()
                .filter(|(i, _)| self.arg_is_input_at(&binds, args, *i))
                .map(|(_, a)| a)
                .collect(),
        )
    }

    /// Is `args[i]` bound to an `input` formal?
    pub(crate) fn task_call_arg_is_input(
        &self,
        name: &ast::HierPath,
        args: &[ast::Expr],
        i: usize,
    ) -> bool {
        match self.callee_arg_dirs(name, args) {
            Some(binds) => self.arg_is_input_at(&binds, args, i),
            // Unresolvable ⇒ treat as NOT an input, so it is never rewritten.
            None => false,
        }
    }

    /// Shared body of the two above: find `args[i]` among the resolved bindings (by pointer
    /// identity of the actual) and report whether its formal is an `input`.
    fn arg_is_input_at(
        &self,
        binds: &[(ast::PortDir, &ast::Expr)],
        args: &[ast::Expr],
        i: usize,
    ) -> bool {
        let target = &args[i] as *const ast::Expr;
        binds
            .iter()
            .find(|(_, a)| std::ptr::eq(*a as *const ast::Expr, target))
            .is_some_and(|(dir, _)| matches!(dir, ast::PortDir::Input))
    }
}

impl Elaborator<'_> {
    /// Does any `inout` actual of this call statement name a root that one of the `input`
    /// actuals' calls WRITES? An `inout` copy-in reads the actual, and that read sits at the
    /// call — after every hoisted copy-out — so it would see the post-call value; and being
    /// the write destination too, it cannot be redirected to a snapshot.
    pub(crate) fn task_call_inout_root_written(
        &self,
        name: &ast::HierPath,
        args: &[ast::Expr],
        inputs: &[&ast::Expr],
    ) -> bool {
        let Some(binds) = self.callee_arg_dirs(name, args) else {
            return true;
        };
        let mut written = BTreeSet::new();
        for e in inputs {
            self.collect_mutated_deep(e, &mut written);
        }
        binds.iter().any(|(dir, a)| {
            matches!(dir, ast::PortDir::Inout)
                && expr_root_ident(named_arg_value(a)).is_some_and(|r| written.contains(&r))
        })
    }
}
