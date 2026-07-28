//! The statement-level `$sformatf` / `'{…}`-actual hoist — split out of `hoist/mod.rs`
//! for the ≤1000-line module policy.
//!
//! `$sformatf` renders a value the engine can only produce as the rhs of a blocking
//! assignment, so reaching it in any other position means rendering into a temp first
//! and reading the temp. That is a MOVE of an evaluation, and §4.5.250's adversarial
//! review found five silent-wrongs in the first cut of exactly that move. The rules
//! the traversal below encodes, each paid for:
//!
//!   * how many times — not into a ternary arm, a short-circuited `&&`/`||` operand,
//!     a replication value, or a `$monitor`/`$strobe` argument list (those re-render);
//!   * in what order — not past a sibling that is not inert (`expr_is_inert`);
//!   * where — not inside a frame FUNCTION body, whose executor cannot write the
//!     module net a temp would be.
//!
//! Everything it declines keeps the honest loud it had before the hoist existed.

use super::*;

impl Elaborator<'_> {
    /// §4.5.248: hoist EVERY `$sformatf(…)` inside `e` that sits in an
    /// unconditionally-evaluated position out to a fresh string temp, returning the
    /// rewritten expression (`None` ⇒ nothing to hoist, so the caller stays on its
    /// byte-identical path). Post-order, so a `$sformatf` nested in another one's
    /// arguments is rendered first — the same order a left-to-right evaluation gives.
    ///
    /// `$sformatf` is PURE (it renders a value; it mutates nothing), so moving its
    /// evaluation earlier within the same statement is unobservable — the ONE thing
    /// that matters is that it still runs exactly as many times as before. That is why
    /// the descent stops at every conditional or repeating position:
    ///
    ///   * a ternary's ARMS (only one runs) — the condition is descended, the arms are
    ///     not, so `c ? $sformatf(…) : s` keeps its existing loud rather than becoming
    ///     a render that happens on both branches;
    ///   * anything not enumerated below (a `with` clause, a constraint, a randomize
    ///     body, a method call on a result) — unvetted ⇒ left alone ⇒ still loud.
    ///
    /// The CALLER is responsible for only invoking this on statement positions that
    /// run once (a blocking / non-blocking rhs, a task-enable argument list) — never on
    /// a loop condition, which re-evaluates.
    pub(crate) fn hoist_nested_sformatf(
        &mut self,
        b: &mut ProcessBuilder,
        e: &ast::Expr,
    ) -> Option<ast::Expr> {
        if self.frame_fn_lowering {
            return None; // see `lower_frame_func_body` — the temp would be a module net
        }
        let inner = self.hoist_sformatf_children(b, e);
        let node = inner.clone().unwrap_or_else(|| e.clone());
        if let Some(t) = self.hoist_sformatf_arg(b, &node) {
            return Some(t);
        }
        inner
    }

    /// Conservatively: evaluating `e` has no observable side effect, so moving a
    /// LATER sibling's evaluation ahead of it is unobservable. Anything not enumerated
    /// answers `false`.
    ///
    /// The hoist moves a render EARLIER, which is only safe when everything it moves
    /// past is inert. `show($urandom, {"<", $sformatf("%0d", $urandom), ">"})` proved it
    /// is not free: hoisting the nested render handed argument 1 the SECOND draw and the
    /// format the FIRST — the two arguments swapped values (review F3, caught against
    /// live iverilog).
    fn expr_is_inert(e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        match &e.kind {
            K::IntLit { .. }
            | K::RealLit { .. }
            | K::StrLit { .. }
            | K::Null
            | K::Dollar
            | K::Ident(_) => true,
            K::Paren { inner } => Self::expr_is_inert(inner),
            K::Unary { operand, .. } => Self::expr_is_inert(operand),
            K::Binary { lhs, rhs, .. } => Self::expr_is_inert(lhs) && Self::expr_is_inert(rhs),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                Self::expr_is_inert(cond)
                    && Self::expr_is_inert(then_e)
                    && Self::expr_is_inert(else_e)
            }
            K::BitSelect { base, index } => Self::expr_is_inert(base) && Self::expr_is_inert(index),
            K::PartSelect { base, msb, lsb } => {
                Self::expr_is_inert(base) && Self::expr_is_inert(msb) && Self::expr_is_inert(lsb)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                Self::expr_is_inert(base)
                    && Self::expr_is_inert(offset)
                    && Self::expr_is_inert(width)
            }
            K::MinTypMax { min, typ, max } => {
                Self::expr_is_inert(min) && Self::expr_is_inert(typ) && Self::expr_is_inert(max)
            }
            K::Cast { target, expr } => {
                Self::expr_is_inert(expr)
                    && match target {
                        ast::CastTarget::Size(sz) => Self::expr_is_inert(sz),
                        _ => true,
                    }
            }
            K::Concat { parts } | K::AssignPattern(parts) => parts.iter().all(Self::expr_is_inert),
            K::Replicate { count, value } => {
                Self::expr_is_inert(count) && value.iter().all(Self::expr_is_inert)
            }
            // `$sformatf` itself renders and mutates nothing; a QUERY sysfunc is a pure
            // read. Everything else — `$random`/`$urandom`/`$fgetc`/… and every USER
            // call (which may carry an output formal) — is assumed to have an effect.
            K::SysCall { name, args } => {
                matches!(
                    name.name.as_str(),
                    "$sformatf"
                        | "$time"
                        | "$stime"
                        | "$realtime"
                        | "$bits"
                        | "$clog2"
                        | "$signed"
                        | "$unsigned"
                        | "$size"
                        | "$left"
                        | "$right"
                        | "$high"
                        | "$low"
                        | "$dimensions"
                        | "$increment"
                        | "$typename"
                        | "$countones"
                        | "$onehot"
                        | "$onehot0"
                        | "$isunknown"
                ) && args.iter().all(Self::expr_is_inert)
            }
            _ => false,
        }
    }

    /// [`Self::hoist_nested_sformatf`] restricted to STRICTLY NESTED occurrences — the
    /// root node itself is left alone. Used where the root has its own handling that
    /// must not be disturbed: a `$display` VALUE argument, whose top-level hoist is
    /// gated on the format string (§4.5.127) because replacing a surplus arg with a
    /// string temp changes how it renders. A `$sformatf` buried inside such an argument
    /// (`$display("%0d", len($sformatf(…)))`) is not an argument at all — the enclosing
    /// expression keeps its own type — so it carries none of that hazard.
    pub(crate) fn hoist_sformatf_children(
        &mut self,
        b: &mut ProcessBuilder,
        e: &ast::Expr,
    ) -> Option<ast::Expr> {
        if self.frame_fn_lowering {
            return None;
        }
        use ast::ExprKind as K;
        // Rewrite the children first (post-order), then this node.
        let rebuilt: Option<K> = match &e.kind {
            K::Paren { inner } => self
                .hoist_nested_sformatf(b, inner)
                .map(|i| K::Paren { inner: Box::new(i) }),
            K::Unary { op, operand } => self.hoist_nested_sformatf(b, operand).map(|o| K::Unary {
                op: *op,
                operand: Box::new(o),
            }),
            // §11.4.7: `&&` / `||` MAY SKIP their right operand, so hoisting out of it
            // makes the render happen 0 -> 1 times — and while `$sformatf` is pure its
            // ARGUMENTS are not: `c && (s == $sformatf("%0d", $random))` advanced the
            // seed with `c` false (review F2, caught against live iverilog). The LEFT
            // operand is always evaluated, so it still descends.
            K::Binary { op, lhs, rhs } => {
                let l = self.hoist_nested_sformatf(b, lhs);
                let r = if matches!(op, ast::BinOp::LogAnd | ast::BinOp::LogOr)
                    || !Self::expr_is_inert(lhs)
                {
                    None // short-circuit, or a left operand whose effects must come first
                } else {
                    self.hoist_nested_sformatf(b, rhs)
                };
                (l.is_some() || r.is_some()).then(|| K::Binary {
                    op: *op,
                    lhs: Box::new(l.unwrap_or_else(|| (**lhs).clone())),
                    rhs: Box::new(r.unwrap_or_else(|| (**rhs).clone())),
                })
            }
            K::Cast { target, expr } => self.hoist_nested_sformatf(b, expr).map(|x| K::Cast {
                target: target.clone(),
                expr: Box::new(x),
            }),
            K::Concat { parts } => self
                .hoist_expr_list(b, parts)
                .map(|parts| K::Concat { parts }),
            // A replication evaluates its value `count` times — ZERO for `{0{…}}`. Not
            // "exactly as many times as before" for any count but 1, so it is not
            // descended into (review F2).
            // A nested call gets BOTH rewrites: its arguments' own `$sformatf`s, and any
            // non-empty `'{…}` actual bound to an `input` dyn-array formal (which must be
            // materialized into a temp — see `hoist_dyn_pattern_actuals`). Both are
            // statement-level for the same reason, so they ride one traversal.
            K::Call { name, args } => {
                let inner = self.hoist_expr_list(b, args);
                let cur: &[ast::Expr] = inner.as_deref().unwrap_or(args);
                match self.hoist_dyn_pattern_actuals(b, name, cur) {
                    Some(args) => Some(K::Call {
                        name: name.clone(),
                        args,
                    }),
                    None => inner.map(|args| K::Call {
                        name: name.clone(),
                        args,
                    }),
                }
            }
            K::SysCall { name, args } if name.name != "$sformatf" => {
                self.hoist_expr_list(b, args).map(|args| K::SysCall {
                    name: name.clone(),
                    args,
                })
            }
            // Only the CONDITION — the arms are conditionally evaluated.
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => self.hoist_nested_sformatf(b, cond).map(|c| K::Ternary {
                cond: Box::new(c),
                then_e: then_e.clone(),
                else_e: else_e.clone(),
            }),
            _ => None,
        };
        rebuilt.map(|kind| ast::Expr { kind, span: e.span })
    }

    /// [`Self::hoist_nested_sformatf`] over a statement's actual-argument list.
    pub(crate) fn hoist_expr_list_pub(
        &mut self,
        b: &mut ProcessBuilder,
        list: &[ast::Expr],
    ) -> Option<Vec<ast::Expr>> {
        self.hoist_expr_list(b, list)
    }

    /// [`Self::hoist_nested_sformatf`] over a list; `None` ⇒ no element changed.
    fn hoist_expr_list(
        &mut self,
        b: &mut ProcessBuilder,
        list: &[ast::Expr],
    ) -> Option<Vec<ast::Expr>> {
        // LEFT-PREFIX RULE (review F3): element `i` may be hoisted only when every
        // element before it is inert, because the hoist moves its render ahead of all
        // of them. The moment one is not, this list stops hoisting — the remaining
        // `$sformatf`s keep their existing loud rather than silently reordering.
        let mut prefix_inert = true;
        let rewritten: Vec<Option<ast::Expr>> = list
            .iter()
            .map(|x| {
                let r = if prefix_inert {
                    self.hoist_nested_sformatf(b, x)
                } else {
                    None
                };
                prefix_inert = prefix_inert && Self::expr_is_inert(x);
                r
            })
            .collect();
        rewritten.iter().any(|x| x.is_some()).then(|| {
            rewritten
                .into_iter()
                .zip(list)
                .map(|(new, old)| new.unwrap_or_else(|| old.clone()))
                .collect()
        })
    }
}
