//! The constant-edge decision (§4.5.601): a part-select width, an indexed part-select
//! width and a replication count each have ONE value — the IEEE value of the bounds as
//! written — decided here once, and the engine reads only decided values.
//!
//! The frozen IR carries the three as expression EDGES (`Expr::Select.width`,
//! `Expr::Replicate.count`, `LvalChunk.width`), and every engine reader folds them with
//! one SHALLOW fold, [`sim_ir::selfwidth::const_u32_of_expr_ctx`]: sign-blind, clamping
//! every leaf at `WIDTH_MAX`, saturating a subtraction at zero, and defaulting (one bit,
//! zero copies, the whole net) where it has no answer. So an edge is never left for the
//! readers to interpret:
//!
//! - **Decided** while lowering, when no leaf is a deferred placeholder: the value is the
//!   constant domain's where it answers (a `[m:l]` bound by bound; an indexed width or a
//!   count only when its signed fold is not negative), else
//!   [`sim_ir::selfwidth::edge_value`] over the lowered tree — one self-determined region
//!   per bound, wrapping at its width and read at its sign. The tree stays when the
//!   engine's fold and elaborate's mirror both read that value; otherwise a fresh `Const`
//!   holds it.
//! - **Late**, when the tree holds a placeholder (a generate-block or instance name, a
//!   `$bits(u.X)`): decided after the deferred passes patch it
//!   ([`Elaborator::decide_late_edges`], before the multidriver scan reads a
//!   continuous-assign chunk). A disagreeing reader gets a fresh `Const`, and every edge
//!   holder of the root is repointed to it; the root itself is never rewritten, because
//!   an inline formal's root can also be read as a value.
//! - **Kept**: a NEGATIVE LITERAL bound (`x[-1:0]`) keeps its old tree for read and write —
//!   an oracle split (iverilog and sv2v refuse it as out of order, verilator reads two
//!   bits).
//! - **Refused** (`E3009`): a negative or out-of-order width, a negative count, a zero
//!   indexed width (IEEE §11.5.1), an x/z value, a tree outside the engine fold's arms (a
//!   declined call, a variable, `*` over a late name, arithmetic over a >64-bit
//!   parameter's bits), a >64-bit parameter declared with a non-zero low bound, and a
//!   `[m:l]` width over vita's net limit from bounds that are not two literals.
//!
//! A consumer that SIZES something from a target's or an operand's width while lowering
//! sees an undecided width as unknown, never as 1 or 32: an all-ones or all-zeros fill
//! becomes `~1'b0` or a one-bit zero, which the run sizes at the final context; an
//! intra-assignment capture holds the right-hand side at its own width; and where the
//! value the consumer built depends on a width it could not know, it records that width,
//! and the decision refuses the design when the decided width is not the one assumed.
//!
//! Nothing here asks whether the code is ever executed: a width in an uncalled function,
//! a dead branch or an auto-elaborated root module is decided, or refused, like any other.

use super::*;
use sim_ir::selfwidth::{EdgeFault, WIDTH_MAX};

/// Which constant edge a funnel built — the noun the refusal names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EdgeKind {
    /// `[m:l]`: the `(m - l) + 1` width tree (`(l - m) + 1` on an ascending net).
    PartWidth,
    /// `[b +: w]` / `[b -: w]`: `w`.
    IndexedWidth,
    /// `{n{…}}`: `n`.
    RepCount,
}

impl EdgeKind {
    fn noun(self) -> &'static str {
        match self {
            EdgeKind::PartWidth => "the width of this part-select",
            EdgeKind::IndexedWidth => "the width of this indexed part-select",
            EdgeKind::RepCount => "this replication count",
        }
    }
}

/// One bound of a `[m:l]` select: the constant domain's value where it answered while
/// lowering, else the lowered tree.
#[derive(Clone, Copy, Debug)]
enum Bound {
    Ast(u32),
    Tree(u32),
}

/// What an edge's value is computed from.
#[derive(Clone, Copy, Debug)]
enum Shape {
    Part { msb: Bound, lsb: Bound, desc: bool },
    Indexed(u32),
    Count { n: u32, zero_ok: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Decided(u64),
    Late,
    Kept,
    Refused,
}

/// Why an edge is refused.
#[derive(Clone, Debug)]
enum Refusal {
    Fault(EdgeFault),
    Text(String),
}

/// A funnel edge and its decision.
pub(crate) struct EdgeRec {
    /// The edge as the engine reads it (the root every holder points at).
    root: u32,
    shape: Shape,
    kind: EdgeKind,
    state: State,
    refusal: Option<Refusal>,
    span: Option<ast::Span>,
    /// The instance prefix the funnel ran under, so a refusal names the instance.
    prefix: String,
    /// The reason read off the bound's source text while it was lowered.
    msg: String,
    /// A hierarchical name in the bound (`gb.L`, `u.W`): resolved by a deferred pass, so
    /// the reason is re-read off the patched tree.
    hier: Option<String>,
}

/// What a bound or a shape evaluates to now.
enum Decision {
    Value(u64),
    Late,
    /// A leaf is a constant an error left behind: nothing is decided from it.
    ErrLeaf,
    Fault(EdgeFault),
}

/// A width a lowering-time consumer built something for.
#[derive(Clone, Debug)]
pub(crate) enum Measure {
    /// An assignment target's total width.
    Target(ir::Lvalue),
    /// An operand's self-determined width (a fill's sibling).
    Expr(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Need {
    /// The value built is right only for this exact width.
    Assumed(u32),
    /// The value built is right for any width up to this one.
    AtMost(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UseKind {
    Fill,
    Capture,
    Stream,
}

/// A consumer's record, verified once the widths are decided.
pub(crate) struct UseRec {
    measure: Measure,
    need: Need,
    kind: UseKind,
    span: Option<ast::Span>,
    prefix: String,
}

/// The fill context comes from widths not decided yet (F2): each measure with the width
/// the lowering assumed for it.
#[derive(Clone, Debug)]
pub(crate) struct F2Ctx {
    measures: Vec<(Measure, u32)>,
}

impl F2Ctx {
    pub(crate) fn with(f2: &Option<F2Ctx>, m: Measure, assumed: u32) -> Option<F2Ctx> {
        let mut out = f2.clone().unwrap_or(F2Ctx {
            measures: Vec::new(),
        });
        out.measures.push((m, assumed));
        Some(out)
    }
}

fn is_negative_literal(e: &ast::Expr) -> bool {
    match &Elaborator::peel_parens(e).kind {
        ast::ExprKind::Unary {
            op: ast::UnOp::Minus,
            operand,
        } => matches!(
            Elaborator::peel_parens(operand).kind,
            ast::ExprKind::IntLit { .. }
        ),
        _ => false,
    }
}

fn is_plain_literal(e: &ast::Expr) -> bool {
    matches!(
        &Elaborator::peel_parens(e).kind,
        ast::ExprKind::IntLit { kind, raw } if !literal::is_fill_literal(raw, *kind)
    )
}

fn span_of(bounds: &[&ast::Expr]) -> Option<ast::Span> {
    let first = bounds.first()?;
    let last = bounds.last().unwrap_or(first);
    Some(ast::Span {
        lo: first.span.lo.min(last.span.lo),
        hi: first.span.hi.max(last.span.hi),
    })
}

impl Elaborator<'_> {
    fn edge_ctx(&self) -> sim_ir::selfwidth::ExprCtx<'_> {
        sim_ir::selfwidth::ExprCtx {
            exprs: &self.exprs,
            consts: &self.consts,
            nets: &self.nets,
        }
    }

    // ── what a tree holds ─────────────────────────────────────────────────────────

    /// Does the tree under `eid` hold a deferred placeholder (`late`) or a constant an
    /// error left behind (`err`)?
    fn tree_marks(&self, eid: u32, late: &mut bool, err: &mut bool) {
        if self.error_placeholders.contains(&eid) {
            *err = true;
            return;
        }
        if self.undecided_placeholders.contains(&eid) {
            *late = true;
            return;
        }
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Signal { net, .. }) => {
                if *net == POISON_NET {
                    *late = true;
                }
            }
            Some(ir::Expr::Call { func, args }) => {
                if *func == POISON_FID {
                    *late = true;
                }
                for a in args {
                    self.tree_marks(*a, late, err);
                }
            }
            Some(ir::Expr::SysFunc { args, .. }) => {
                for a in args {
                    self.tree_marks(*a, late, err);
                }
            }
            Some(ir::Expr::Concat { parts }) => {
                for a in parts {
                    self.tree_marks(*a, late, err);
                }
            }
            Some(ir::Expr::Select {
                base,
                offset,
                width,
                ..
            }) => {
                for a in [*base, *offset, *width] {
                    self.tree_marks(a, late, err);
                }
            }
            Some(ir::Expr::Replicate { count, value }) => {
                self.tree_marks(*count, late, err);
                self.tree_marks(*value, late, err);
            }
            Some(ir::Expr::Unary { operand, .. }) => self.tree_marks(*operand, late, err),
            Some(ir::Expr::Binary { lhs, rhs, .. }) => {
                self.tree_marks(*lhs, late, err);
                self.tree_marks(*rhs, late, err);
            }
            Some(ir::Expr::Ternary {
                cond,
                then_e,
                else_e,
            }) => {
                for a in [*cond, *then_e, *else_e] {
                    self.tree_marks(a, late, err);
                }
            }
            Some(ir::Expr::Const { .. } | ir::Expr::ArrayItem { .. }) | None => {}
        }
    }

    fn bound_decision(&self, b: Bound) -> Result<i128, Decision> {
        match b {
            Bound::Ast(v) => Ok(i128::from(v)),
            Bound::Tree(id) => {
                let (mut late, mut err) = (false, false);
                self.tree_marks(id, &mut late, &mut err);
                if err {
                    return Err(Decision::ErrLeaf);
                }
                if late {
                    return Err(Decision::Late);
                }
                sim_ir::selfwidth::edge_bound_value(self.edge_ctx(), id).map_err(Decision::Fault)
            }
        }
    }

    /// The value of `shape` now, or why there is none.
    fn decide_shape(&self, shape: Shape) -> Decision {
        match shape {
            Shape::Part { msb, lsb, desc } => {
                let m = self.bound_decision(msb);
                let l = self.bound_decision(lsb);
                let (m, l) = match (m, l) {
                    (Ok(m), Ok(l)) => (m, l),
                    (Err(Decision::ErrLeaf), _) | (_, Err(Decision::ErrLeaf)) => {
                        return Decision::ErrLeaf
                    }
                    (Err(Decision::Fault(f)), _) | (_, Err(Decision::Fault(f))) => {
                        return Decision::Fault(f)
                    }
                    _ => return Decision::Late,
                };
                match sim_ir::selfwidth::part_width(m, l, desc) {
                    Ok(v) => Decision::Value(v),
                    Err(f) => Decision::Fault(f),
                }
            }
            Shape::Indexed(id) | Shape::Count { n: id, .. } => {
                match self.bound_decision(Bound::Tree(id)) {
                    Ok(v) if v < 0 => Decision::Fault(EdgeFault::Negative),
                    Ok(v) => Decision::Value(u64::try_from(v).unwrap_or(u64::MAX)),
                    Err(d) => d,
                }
            }
        }
    }

    /// The decided value of edge `eid` if a funnel decided it, else `None`.
    pub(crate) fn decided_edge_value(&self, eid: u32) -> Option<u64> {
        match self.edge_recs.get(*self.edge_index.get(&eid)?)?.state {
            State::Decided(v) => Some(v),
            _ => None,
        }
    }

    /// Does the lowered tree under `eid` hold a constant with x or z bits?
    fn tree_has_unknown(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Const { val }) => self
                .consts
                .get(*val as usize)
                .is_some_and(|c| c.bits.unk.iter().any(|&u| u != 0)),
            Some(ir::Expr::Unary { operand, .. }) => self.tree_has_unknown(*operand),
            Some(ir::Expr::Binary { lhs, rhs, .. }) => {
                self.tree_has_unknown(*lhs) || self.tree_has_unknown(*rhs)
            }
            Some(ir::Expr::Ternary {
                cond,
                then_e,
                else_e,
            }) => [*cond, *then_e, *else_e]
                .iter()
                .any(|&x| self.tree_has_unknown(x)),
            Some(ir::Expr::Concat { parts }) => parts.iter().any(|&x| self.tree_has_unknown(x)),
            Some(ir::Expr::SysFunc { args, .. } | ir::Expr::Call { args, .. }) => {
                args.iter().any(|&x| self.tree_has_unknown(x))
            }
            _ => false,
        }
    }

    /// Is count `eid` decided as zero now (a funnel's decision, or a plain `Const`)?
    pub(crate) fn count_is_zero_now(&self, eid: u32) -> bool {
        match self.decided_edge_value(eid) {
            Some(v) => v == 0,
            None => !self.edge_is_late(eid) && self.const_of_expr_u32(eid) == Some(0),
        }
    }

    fn edge_is_late(&self, eid: u32) -> bool {
        self.edge_index
            .get(&eid)
            .and_then(|&i| self.edge_recs.get(i))
            .is_some_and(|r| r.state == State::Late)
    }

    // ── the funnels' decision ─────────────────────────────────────────────────────

    fn push_edge(
        &mut self,
        root: u32,
        shape: Shape,
        kind: EdgeKind,
        state: State,
        refusal: Option<Refusal>,
        bounds: &[&ast::Expr],
    ) {
        let (msg, hier) = if state == State::Refused || state == State::Late {
            (
                self.unfolded_edge_message(kind, bounds),
                bounds.iter().find_map(|b| Self::hier_name_in(b)),
            )
        } else {
            (String::new(), None)
        };
        self.edge_index.insert(root, self.edge_recs.len());
        self.edge_recs.push(EdgeRec {
            root,
            shape,
            kind,
            state,
            refusal,
            span: span_of(bounds),
            prefix: self.cur_prefix.clone(),
            msg,
            hier,
        });
    }

    /// The edge for a value decided while lowering: the tree itself when the engine's fold
    /// and elaborate's mirror both read `v` from it, else a fresh `Const`.
    fn decided_now(
        &mut self,
        tree: u32,
        v: u64,
        shape: Shape,
        kind: EdgeKind,
        bounds: &[&ast::Expr],
    ) -> u32 {
        let root = if self.readers_agree(tree, v) {
            tree
        } else {
            self.const_edge_expr(v)
        };
        self.push_edge(root, shape, kind, State::Decided(v), None, bounds);
        root
    }

    /// Do the engine's fold (which clamps at `WIDTH_MAX`) and elaborate's mirror both read
    /// `v` off `tree`?
    fn readers_agree(&self, tree: u32, v: u64) -> bool {
        let engine = sim_ir::selfwidth::const_u32_of_expr_ctx(self.edge_ctx(), tree);
        let mirror = self.const_of_expr_u32(tree);
        engine.map(u64::from) == Some(v.min(u64::from(WIDTH_MAX)))
            && mirror.map(u64::from) == Some(v)
    }

    fn const_edge_expr(&mut self, v: u64) -> u32 {
        match u32::try_from(v) {
            Ok(n) => self.const_u32_expr(n, 32),
            Err(_) => {
                let cid = self.intern_const(make_const_i64(v as i64, 64, false));
                self.push_expr(ir::Expr::Const { val: cid })
            }
        }
    }

    /// The `[m:l]` funnel's decision, for the width `tree` it built over the lowered bounds
    /// `msb_id` / `lsb_id`; `folded` is the constant domain's value of each bound.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn decide_part_edge(
        &mut self,
        tree: u32,
        msb_ast: &ast::Expr,
        lsb_ast: &ast::Expr,
        msb_id: u32,
        lsb_id: u32,
        ascending: bool,
        folded: (Option<u32>, Option<u32>),
    ) -> u32 {
        let bounds = [msb_ast, lsb_ast];
        let kind = EdgeKind::PartWidth;
        let msb = folded.0.map_or(Bound::Tree(msb_id), Bound::Ast);
        let lsb = folded.1.map_or(Bound::Tree(lsb_id), Bound::Ast);
        let shape = Shape::Part {
            msb,
            lsb,
            desc: !ascending,
        };
        if is_negative_literal(msb_ast) || is_negative_literal(lsb_ast) {
            // The documented split: the old tree stands for read and write.
            let state = if folded.0.is_some() && folded.1.is_some() {
                State::Kept
            } else {
                State::Refused
            };
            let refusal = (state == State::Refused).then_some(Refusal::Fault(EdgeFault::Unreduced));
            self.push_edge(tree, shape, kind, state, refusal, &bounds);
            return tree;
        }
        match self.decide_shape(shape) {
            Decision::ErrLeaf => tree,
            Decision::Late => {
                self.push_edge(tree, shape, kind, State::Late, None, &bounds);
                tree
            }
            Decision::Fault(f) => {
                self.push_edge(
                    tree,
                    shape,
                    kind,
                    State::Refused,
                    Some(Refusal::Fault(f)),
                    &bounds,
                );
                tree
            }
            Decision::Value(v) => {
                if v > MAX_NET_WIDTH && !(is_plain_literal(msb_ast) && is_plain_literal(lsb_ast)) {
                    let text = Self::over_limit_text(v);
                    self.push_edge(
                        tree,
                        shape,
                        kind,
                        State::Refused,
                        Some(Refusal::Text(text)),
                        &bounds,
                    );
                    return tree;
                }
                self.decided_now(tree, v, shape, kind, &bounds)
            }
        }
    }

    fn over_limit_text(v: u64) -> String {
        format!(
            "the width of this part-select ({v} bits) exceeds vita's limit of {MAX_NET_WIDTH} \
             bits for a width it folds while elaborating"
        )
    }

    fn zero_width_text() -> String {
        "the width of this indexed part-select is zero; an indexed part-select width must be \
         a positive constant (IEEE §11.5.1)"
            .to_string()
    }

    fn zero_count_text() -> String {
        "a replication count of zero is only legal as a direct operand of a concatenation \
         (IEEE §11.4.12.1)"
            .to_string()
    }

    /// The indexed-width / replication-count funnel's decision for bound `e`, lowered to
    /// `id`. `zero_ok`: a count that is a direct operand of a concatenation may be zero
    /// (§11.4.12.1) — the count's own fact, passed with its edge.
    pub(crate) fn decide_width_edge(
        &mut self,
        e: &ast::Expr,
        id: u32,
        kind: EdgeKind,
        zero_ok: bool,
    ) -> u32 {
        let bounds = [e];
        let shape = match kind {
            EdgeKind::RepCount => Shape::Count { n: id, zero_ok },
            _ => Shape::Indexed(id),
        };
        let (mut late, mut err) = (false, false);
        self.tree_marks(id, &mut late, &mut err);
        if err {
            return id;
        }
        // A negative signed fold — a `wrapping_neg` literal included (`f[0 +: -1]`,
        // `f[0 +: -4'sd2]`, `f[7 -: -1]`) — is refused. A count's is reported by the
        // caller's §11.4.12.2 check, with its own text.
        if self.const_bound_signed(e).is_some_and(|s| s < 0) {
            if kind != EdgeKind::RepCount {
                self.push_edge(
                    id,
                    shape,
                    kind,
                    State::Refused,
                    Some(Refusal::Fault(EdgeFault::Negative)),
                    &bounds,
                );
            }
            return id;
        }
        let v = match self.const_bound_u32(e) {
            Some(n) => u64::from(n),
            None => match self.decide_shape(shape) {
                Decision::Value(v) => v,
                Decision::ErrLeaf => return id,
                Decision::Late => {
                    self.push_edge(id, shape, kind, State::Late, None, &bounds);
                    return id;
                }
                Decision::Fault(f) => {
                    self.push_edge(
                        id,
                        shape,
                        kind,
                        State::Refused,
                        Some(Refusal::Fault(f)),
                        &bounds,
                    );
                    return id;
                }
            },
        };
        if v == 0 && kind == EdgeKind::IndexedWidth {
            // The constant domain reads an unknown as 0: a lowered constant with x/z bits
            // says so, and a zero computed by a call may be one (the constant interpreter
            // reads a variable the function never assigns as 0, ROADMAP §2 🆕 AE).
            let refusal = if self.tree_has_unknown(id) {
                Refusal::Fault(EdgeFault::Unknown)
            } else if Self::ast_contains_call(e) {
                Refusal::Text(
                    "the width of this indexed part-select has x or z bits, or is zero: it is \
                     computed by a call, and vita's constant interpreter reads a variable the \
                     function never assigns (x) as 0; an indexed part-select width must be a \
                     positive known constant (IEEE §11.5.1)"
                        .to_string(),
                )
            } else {
                Refusal::Text(Self::zero_width_text())
            };
            self.push_edge(id, shape, kind, State::Refused, Some(refusal), &bounds);
            return id;
        }
        self.decided_now(id, v, shape, kind, &bounds)
    }

    // ── the decision pass ─────────────────────────────────────────────────────────

    /// Decide every late edge, now that the deferred passes have patched their
    /// placeholders; report every refusal the finished IR still holds; verify every
    /// consumer's record. Runs before the multidriver scan, which reads continuous-assign
    /// chunk widths.
    pub(crate) fn decide_late_edges(&mut self) {
        for i in 0..self.edge_recs.len() {
            if self.edge_recs[i].state != State::Late {
                continue;
            }
            let shape = self.edge_recs[i].shape;
            match self.decide_shape(shape) {
                // a leaf an error left behind, or a name no pass resolved (both reported)
                Decision::ErrLeaf | Decision::Late => {}
                Decision::Fault(f) => {
                    self.edge_recs[i].state = State::Refused;
                    self.edge_recs[i].refusal = Some(Refusal::Fault(f));
                }
                Decision::Value(v) => {
                    let refusal = match shape {
                        Shape::Part { .. } if v > MAX_NET_WIDTH => Some(Self::over_limit_text(v)),
                        Shape::Indexed(_) if v == 0 => Some(Self::zero_width_text()),
                        Shape::Count { zero_ok: false, .. } if v == 0 => {
                            Some(Self::zero_count_text())
                        }
                        _ => None,
                    };
                    if let Some(text) = refusal {
                        self.edge_recs[i].state = State::Refused;
                        self.edge_recs[i].refusal = Some(Refusal::Text(text));
                        continue;
                    }
                    self.materialize(i, v);
                }
            }
        }
        let used = self.used_edges();
        let mut reported: BTreeSet<(u32, u32, String)> = BTreeSet::new();
        for i in 0..self.edge_recs.len() {
            if self.edge_recs[i].state != State::Refused || !used.contains(&self.edge_recs[i].root)
            {
                continue;
            }
            let msg = self.refusal_message(i);
            let (span, prefix) = (self.edge_recs[i].span, self.edge_recs[i].prefix.clone());
            self.report_at(span, &prefix, &msg, &mut reported);
        }
        self.verify_use_recs(&mut reported);
        self.hier_resolved_chunk.clear();
    }

    /// Record `v` for late edge `i`: keep its tree when both readers read `v`, else intern
    /// a fresh `Const` and repoint every edge holder of the root to it. The root's slot is
    /// never written — an inline formal's root can be read as a value too.
    fn materialize(&mut self, i: usize, v: u64) {
        let root = self.edge_recs[i].root;
        self.edge_recs[i].state = State::Decided(v);
        if self.readers_agree(root, v) {
            return;
        }
        let fresh = self.const_edge_expr(v);
        let outs = (self.task_calls_proc.values_mut())
            .chain(self.task_calls_func.values_mut())
            .flat_map(|info| info.out_binds.iter_mut().map(|(_, lv)| lv));
        sim_ir::selfwidth::for_each_constant_edge_mut(
            &mut self.exprs,
            &mut self.stmts,
            &mut self.cont_assigns,
            outs,
            |e| {
                if *e == root {
                    *e = fresh;
                }
            },
        );
        self.edge_index.insert(fresh, i);
    }

    /// Every edge the finished IR holds.
    fn used_edges(&self) -> BTreeSet<u32> {
        let mut used = BTreeSet::new();
        sim_ir::selfwidth::for_each_constant_edge(
            &self.exprs,
            &self.stmts,
            &self.cont_assigns,
            self.task_call_outs(),
            |e| {
                if let Some(eid) = e {
                    used.insert(eid);
                }
            },
        );
        used
    }

    fn refusal_message(&self, i: usize) -> String {
        let r = &self.edge_recs[i];
        let what = r.kind.noun();
        match &r.refusal {
            Some(Refusal::Text(t)) => t.clone(),
            Some(Refusal::Fault(EdgeFault::Negative)) => match r.kind {
                EdgeKind::PartWidth => format!(
                    "{what} is negative: its bounds are out of order for the declared \
                     direction of what it selects"
                ),
                EdgeKind::IndexedWidth => format!("{what} is negative"),
                EdgeKind::RepCount => format!("{what} is negative (IEEE §11.4.12.2)"),
            },
            Some(Refusal::Fault(EdgeFault::Unknown)) => {
                format!("{what} does not fold to a constant: its value has x or z bits")
            }
            Some(Refusal::Fault(EdgeFault::Unreduced)) | None => {
                // A hierarchical name the deferred pass has since resolved is no longer
                // what blocks the tree: an operator over it is.
                if let (Some(name), Some(op)) = (&r.hier, self.edge_blocking_operator(r.root)) {
                    return format!(
                        "{what} does not fold to a constant: it applies `{op}` to `{name}`, a \
                         hierarchical name vita resolves only after elaboration, and the \
                         simulator evaluates only `+` and `-` over such a name in a constant \
                         width or count (give the value a localparam in this scope)"
                    );
                }
                r.msg.clone()
            }
        }
    }

    fn report_at(
        &mut self,
        span: Option<ast::Span>,
        prefix: &str,
        msg: &str,
        reported: &mut BTreeSet<(u32, u32, String)>,
    ) {
        let (lo, hi) = span.map_or((0, 0), |s| (s.lo, s.hi));
        // One bound lowered twice under one instance (a statement lowered for two routes)
        // is one defect.
        if !reported.insert((lo, hi, prefix.to_string())) {
            return;
        }
        let saved_prefix = std::mem::replace(&mut self.cur_prefix, prefix.to_string());
        match span {
            Some(span) => self.error_at(MsgCode::ElabUnsupported, span, msg),
            None => self.error(MsgCode::ElabUnsupported, msg),
        }
        self.cur_prefix = saved_prefix;
    }

    /// The caller lvalues every task call copies its outputs to (`t(a[gb.L-1:0])`): they
    /// sit in the task-call side tables, not in the statement arena, and their part
    /// chunks are read by the same fold.
    fn task_call_outs(&self) -> impl Iterator<Item = &ir::Lvalue> {
        (self.task_calls_proc.values())
            .chain(self.task_calls_func.values())
            .flat_map(|info| info.out_binds.iter().map(|(_, lv)| lv))
    }

    /// The backstop, after the whole design is built: every edge the engine will read is a
    /// `Const` without x/z bits or a funnel's decided or kept edge; a part chunk always has
    /// a width edge. Every producer outside the funnels writes a `Const` (§4.5.601's
    /// census), so this finds nothing in any design measured; it is here so a producer
    /// added later cannot hand the engine an edge nobody decided. It has no source
    /// location to give, and a design already refused needs no second, vaguer line.
    pub(crate) fn edge_backstop(&mut self) {
        if self.had_error {
            return;
        }
        let mut stray: u32 = 0;
        sim_ir::selfwidth::for_each_constant_edge(
            &self.exprs,
            &self.stmts,
            &self.cont_assigns,
            self.task_call_outs(),
            |e| {
                let ok = e.is_some_and(|eid| {
                    let plain = matches!(
                        self.exprs.get(eid as usize),
                        Some(ir::Expr::Const { val }) if self
                            .consts
                            .get(*val as usize)
                            .is_some_and(|c| c.bits.unk.iter().all(|&u| u == 0))
                    );
                    plain
                        || self
                            .edge_index
                            .get(&eid)
                            .and_then(|&i| self.edge_recs.get(i))
                            .is_some_and(|r| matches!(r.state, State::Decided(_) | State::Kept))
                });
                if !ok {
                    stray = stray.saturating_add(1);
                }
            },
        );
        if stray > 0 {
            let span = self.cur_span.take();
            let prefix = std::mem::take(&mut self.cur_prefix);
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "{stray} part-select width(s) or replication count(s) in this design were \
                     not decided, and no source location was recorded for them (the simulator \
                     would read one bit, repeat zero times or write the whole net)"
                ),
            );
            self.cur_span = span;
            self.cur_prefix = prefix;
        }
    }

    // ── consumers that size something before the widths are decided ──────────────

    /// Total bit width of a LOWERED lvalue as the engine reads it (`lvalue_width`): a
    /// whole-net chunk reads the net (element) width, a bit select is 1, a part select
    /// reads its width edge — 1 where it does not fold (a kept negative-literal split, as
    /// before). Private: every consumer goes through a wrapper below, which knows whether
    /// the width is decided.
    fn ir_lvalue_width(&self, lv: &ir::Lvalue) -> u32 {
        lv.chunks
            .iter()
            .map(|c| match c.kind {
                ir::SelKind::Bit => {
                    if c.offset.is_none() && c.width.is_none() {
                        // `.get()` (not index) so an error-recovery lvalue with an
                        // out-of-range net id degrades to 1 instead of panicking.
                        self.nets.get(c.net as usize).map(|n| n.width).unwrap_or(1)
                    } else {
                        1
                    }
                }
                _ => c
                    .width
                    .and_then(|eid| self.const_of_expr_u32(eid))
                    .unwrap_or(1),
            })
            .sum::<u32>()
            .max(1)
    }

    /// Is `lv`'s width undecided now: a part chunk whose width edge is late, or a chunk
    /// written through a deferred hierarchical sentinel (its width is the resolved
    /// chunk's)?
    pub(crate) fn target_undecided(&self, lv: &ir::Lvalue) -> bool {
        lv.chunks.iter().any(|c| {
            (HIER_SEL_WRITE_SENTINEL_BASE..POISON_NET).contains(&c.net)
                || (c.kind != ir::SelKind::Bit && c.width.is_some_and(|w| self.edge_is_late(w)))
        })
    }

    /// The width of `lv` with every late edge evaluated now (the deferred passes are done):
    /// what a pending fill is re-lowered at.
    pub(crate) fn decided_lvalue_width(&self, lv: &ir::Lvalue) -> u32 {
        let late_value = |eid: u32| -> Option<u32> {
            let i = *self.edge_index.get(&eid)?;
            let r = self.edge_recs.get(i)?;
            match r.state {
                State::Late => match self.decide_shape(r.shape) {
                    Decision::Value(v) => u32::try_from(v).ok(),
                    _ => None,
                },
                State::Decided(v) => u32::try_from(v).ok(),
                _ => None,
            }
        };
        lv.chunks
            .iter()
            .map(|c| match (c.kind, c.width) {
                (ir::SelKind::Bit, _) | (_, None) => self.ir_lvalue_width(&ir::Lvalue {
                    chunks: vec![c.clone()],
                }),
                (_, Some(w)) => late_value(w)
                    .or_else(|| self.const_of_expr_u32(w))
                    .unwrap_or(1),
            })
            .sum::<u32>()
            .max(1)
    }

    fn push_use(&mut self, measure: Measure, need: Need, kind: UseKind) {
        self.use_recs.push(UseRec {
            measure,
            need,
            kind,
            span: self.cur_span,
            prefix: self.cur_prefix.clone(),
        });
    }

    /// The context `resize_rhs_for_lvalue` lowers a fill-bearing right-hand side in, and
    /// the F2 state to lower it under: a decided target gives its exact width; an
    /// undecided one gives the width the lowering can see (each late chunk read as 1) and
    /// an F2 state, so an all-ones or all-zeros fill is spelled for the run to size.
    pub(crate) fn fill_context(&self, lv: &ir::Lvalue) -> (u32, Option<F2Ctx>) {
        let w = self.ir_lvalue_width(lv);
        if !self.target_undecided(lv) {
            return (w, None);
        }
        (w, F2Ctx::with(&None, Measure::Target(lv.clone()), w))
    }

    /// An `'x` / `'z` fill lowered at `w` under F2: the value is right for the assumed
    /// width, or — when a sibling widened it — for any width up to `w`.
    pub(crate) fn record_unknown_fill(&mut self, f2: &F2Ctx, w: u32) {
        for (m, assumed) in &f2.measures {
            let need = if w > *assumed {
                Need::AtMost(w)
            } else {
                Need::Assumed(*assumed)
            };
            self.push_use(m.clone(), need, UseKind::Fill);
        }
    }

    /// RC2: is `sibling`'s self-determined width not decided yet — does it depend on a late
    /// edge (a select's width, a replication's count over a generate-block or instance
    /// name or a `$bits(u.X)`)? A fill beside it is then lowered for the run to size,
    /// instead of at `sibling_ctx`'s 32 (or at the placeholder's 32).
    pub(crate) fn sibling_undecided(&self, sibling: u32) -> bool {
        self.width_is_late(sibling)
    }

    /// Does the self-determined width of `eid` depend on a late edge?
    fn width_is_late(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Select { width, kind, .. }) => {
                *kind != ir::SelKind::Bit && self.edge_is_late(*width)
            }
            Some(ir::Expr::Replicate { count, value }) => {
                self.edge_is_late(*count) || self.width_is_late(*value)
            }
            Some(ir::Expr::Concat { parts }) => parts.iter().any(|&p| self.width_is_late(p)),
            Some(ir::Expr::Unary { op, operand }) => {
                matches!(op, ir::UnOp::Plus | ir::UnOp::Minus | ir::UnOp::BitNot)
                    && self.width_is_late(*operand)
            }
            Some(ir::Expr::Binary { op, lhs, rhs }) => match op {
                // a 1-bit result
                ir::BinOp::Eq
                | ir::BinOp::Ne
                | ir::BinOp::Lt
                | ir::BinOp::Le
                | ir::BinOp::Gt
                | ir::BinOp::Ge
                | ir::BinOp::CaseEq
                | ir::BinOp::CaseNe
                | ir::BinOp::LogAnd
                | ir::BinOp::LogOr => false,
                // the left operand's width
                ir::BinOp::Shl
                | ir::BinOp::Shr
                | ir::BinOp::AShl
                | ir::BinOp::AShr
                | ir::BinOp::Pow => self.width_is_late(*lhs),
                _ => self.width_is_late(*lhs) || self.width_is_late(*rhs),
            },
            Some(ir::Expr::Ternary { then_e, else_e, .. }) => {
                self.width_is_late(*then_e) || self.width_is_late(*else_e)
            }
            _ => false,
        }
    }

    /// The width of an intra-assignment capture temp. A decided target gives its exact
    /// width (the right-hand side is evaluated in that context, as before). An undecided
    /// one gives the right-hand side's own width R (at least 1): the capture is then
    /// exact under the write's zero-extension unless the right-hand side's top region is
    /// signed, holds a fill other than an all-zero one (`'0` zero-extends exactly), or has
    /// an operator whose value depends on the width it is evaluated at
    /// (`+ - * ** << <<< ~^ ^~`, unary `~` or `-`) — then the record makes
    /// the decision refuse a target wider than R. An R the lowering cannot tell is 1,
    /// and right only for a one-bit target.
    pub(crate) fn ia_capture_width(&mut self, lv: &ir::Lvalue, rhs_id: u32) -> u32 {
        if !self.target_undecided(lv) {
            return self.ir_lvalue_width(lv);
        }
        let Some(r) = self.trusted_self_width(rhs_id) else {
            self.push_use(
                Measure::Target(lv.clone()),
                Need::Assumed(1),
                UseKind::Capture,
            );
            return 1;
        };
        let signed = self.canonical_self_width(rhs_id).is_none_or(|sw| sw.signed);
        if signed || self.region_needs_width(rhs_id) {
            self.push_use(
                Measure::Target(lv.clone()),
                Need::AtMost(r),
                UseKind::Capture,
            );
        }
        r.max(1)
    }

    /// Does the top region of `eid` hold a fill that is not all zeros (an all-ones fill,
    /// spelled `~1'b0`, or an `'x` / `'z` one), or an operator whose value depends on the
    /// width it is evaluated at? A `'0` fill is a known zero, which the write's
    /// zero-extension reproduces at any width. Operands of `& | ^ / %`, a shift's left
    /// operand and the ternary arms are in the region; comparisons, logical and reduction
    /// operators, a shift amount and every self-determined node (a select, a
    /// concatenation, a call…) start another.
    fn region_needs_width(&self, eid: u32) -> bool {
        if self.fill_eids.contains(&eid) {
            return !self.is_known_zero_const(eid);
        }
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Binary { op, lhs, rhs }) => match op {
                ir::BinOp::Add
                | ir::BinOp::Sub
                | ir::BinOp::Mul
                | ir::BinOp::Pow
                | ir::BinOp::Shl
                | ir::BinOp::AShl
                | ir::BinOp::BitXnor => true,
                ir::BinOp::BitAnd
                | ir::BinOp::BitOr
                | ir::BinOp::BitXor
                | ir::BinOp::Div
                | ir::BinOp::Mod => self.region_needs_width(*lhs) || self.region_needs_width(*rhs),
                ir::BinOp::Shr | ir::BinOp::AShr => self.region_needs_width(*lhs),
                _ => false,
            },
            Some(ir::Expr::Unary { op, operand }) => match op {
                ir::UnOp::BitNot | ir::UnOp::Minus => true,
                ir::UnOp::Plus => self.region_needs_width(*operand),
                _ => false,
            },
            Some(ir::Expr::Ternary { then_e, else_e, .. }) => {
                self.region_needs_width(*then_e) || self.region_needs_width(*else_e)
            }
            _ => false,
        }
    }

    /// Is `eid` a constant whose bits are all known zeros?
    fn is_known_zero_const(&self, eid: u32) -> bool {
        matches!(
            self.exprs.get(eid as usize),
            Some(ir::Expr::Const { val }) if self.consts.get(*val as usize).is_some_and(|c| {
                c.bits.val.iter().all(|&v| v == 0) && c.bits.unk.iter().all(|&u| u == 0)
            })
        )
    }

    /// The target width a streaming right-hand side is padded against (§11.4.14.3). An
    /// undecided target is read as the lowering can see it; with one chunk the stream is
    /// then left unpadded, which stays right for any final width up to the stream's.
    pub(crate) fn stream_target_width(&mut self, lv: &ir::Lvalue, stream_width: u32) -> u32 {
        let w = self.ir_lvalue_width(lv);
        if !self.target_undecided(lv) {
            return w;
        }
        let need = if lv.chunks.len() == 1 && w <= stream_width {
            Need::AtMost(stream_width)
        } else {
            Need::Assumed(w)
        };
        self.push_use(Measure::Target(lv.clone()), need, UseKind::Stream);
        w
    }

    /// The decided width of a measure, once every edge is decided.
    fn measure_width(&self, m: &Measure) -> Option<u64> {
        match m {
            Measure::Expr(eid) => self.ir_bits_of(*eid).map(u64::from),
            Measure::Target(lv) => {
                let mut total: u64 = 0;
                for c in &lv.chunks {
                    total += self.chunk_width(c)?;
                }
                Some(total.max(1))
            }
        }
    }

    fn chunk_width(&self, c: &ir::LvalChunk) -> Option<u64> {
        if (HIER_SEL_WRITE_SENTINEL_BASE..POISON_NET).contains(&c.net) {
            let resolved = self.hier_resolved_chunk.get(&c.net)?;
            if resolved.net == POISON_NET {
                return None;
            }
            return self.chunk_width(resolved);
        }
        match (c.kind, c.width) {
            (ir::SelKind::Bit, _) if c.offset.is_none() && c.width.is_none() => {
                self.nets.get(c.net as usize).map(|n| u64::from(n.width))
            }
            (ir::SelKind::Bit, _) => Some(1),
            (_, None) => None,
            (_, Some(w)) => self
                .decided_edge_value(w)
                .or_else(|| self.const_of_expr_u32(w).map(u64::from)),
        }
    }

    fn verify_use_recs(&mut self, reported: &mut BTreeSet<(u32, u32, String)>) {
        let recs = std::mem::take(&mut self.use_recs);
        for u in &recs {
            let Some(f) = self.measure_width(&u.measure) else {
                continue; // refused or reported where the width failed
            };
            let ok = match u.need {
                Need::Assumed(a) => f == u64::from(a),
                Need::AtMost(a) => f <= u64::from(a),
            };
            if ok {
                continue;
            }
            let assumed = match u.need {
                Need::Assumed(a) | Need::AtMost(a) => a,
            };
            let late = "is known only after elaboration (a bound reads a name resolved then)";
            let subject = match &u.measure {
                Measure::Target(lv)
                    if lv
                        .chunks
                        .iter()
                        .any(|c| (HIER_SEL_WRITE_SENTINEL_BASE..POISON_NET).contains(&c.net)) =>
                {
                    format!(
                        "the width of this hierarchical assignment target ({f} bits) is known \
                         only after elaboration"
                    )
                }
                Measure::Target(_) => {
                    format!("the width of this assignment target ({f} bits) {late}")
                }
                Measure::Expr(_) => {
                    format!("the width of the operand beside this fill ({f} bits) {late}")
                }
            };
            let reason = match u.kind {
                UseKind::Fill => {
                    format!("an `'x` or `'z` fill was sized before that, as {assumed} bit(s)")
                }
                UseKind::Capture => format!(
                    "its intra-assignment capture holds the right-hand side at {assumed} \
                     bit(s), and that value depends on the width it is evaluated at"
                ),
                UseKind::Stream => format!(
                    "the streaming concatenation on its right-hand side was padded before \
                     that, for {assumed} bit(s)"
                ),
            };
            let msg = format!("{subject}, but {reason}");
            let (span, prefix) = (u.span, u.prefix.clone());
            self.report_at(span, &prefix, &msg, reported);
        }
    }

    /// The operator that stops the engine's fold of edge `eid` once every leaf is a
    /// constant, if that is what stops it (rather than a read of a variable, a call, or
    /// an x/z value).
    fn edge_blocking_operator(&self, eid: u32) -> Option<&'static str> {
        match self.exprs.get(eid as usize)? {
            ir::Expr::Binary {
                op: ir::BinOp::Add | ir::BinOp::Sub,
                lhs,
                rhs,
            } => self
                .edge_blocking_operator(*lhs)
                .or_else(|| self.edge_blocking_operator(*rhs)),
            ir::Expr::Binary { op, lhs, rhs } => {
                let leaves_known =
                    self.edge_leaves_are_constants(*lhs) && self.edge_leaves_are_constants(*rhs);
                leaves_known.then(|| ir_binop_text(*op))
            }
            ir::Expr::Unary { op, operand } => self
                .edge_leaves_are_constants(*operand)
                .then(|| ir_unop_text(*op)),
            _ => None,
        }
    }

    /// Does the tree under `eid` hold only known constants and operators (no read, no
    /// call, no x/z value)?
    fn edge_leaves_are_constants(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Const { val }) => self
                .consts
                .get(*val as usize)
                .is_some_and(|c| c.bits.unk.iter().all(|&u| u == 0)),
            Some(ir::Expr::Binary { lhs, rhs, .. }) => {
                self.edge_leaves_are_constants(*lhs) && self.edge_leaves_are_constants(*rhs)
            }
            Some(ir::Expr::Unary { operand, .. }) => self.edge_leaves_are_constants(*operand),
            _ => false,
        }
    }

    /// The refusal's text read off the bound's source while it is lowered: a >64-bit
    /// parameter with a non-zero low bound (read by position), an x or z value, a declined
    /// call, a >64-bit parameter, else the first sub-expression the constant domain has
    /// no answer for, else the plain fact.
    fn unfolded_edge_message(&self, kind: EdgeKind, bounds: &[&ast::Expr]) -> String {
        let what = kind.noun();
        if let Some((name, range)) = bounds.iter().find_map(|b| self.wide_lsb_param_in(b)) {
            return format!(
                "{what} does not fold to a constant: it reads `{name}`, a parameter wider than \
                 64 bits declared {range}; vita reads a select of such a parameter by position \
                 from bit 0, so it does not evaluate one in a constant width or count"
            );
        }
        if bounds.iter().any(|b| self.wide_domain_has_unknown(b)) {
            return format!("{what} does not fold to a constant: its value has x or z bits");
        }
        let reason = bounds.iter().find_map(|b| self.unfoldable_reason(b));
        if let Some(r) = &reason {
            if bounds.iter().any(|b| Self::ast_contains_call(b)) {
                return format!("{what} does not fold to a constant: {r}");
            }
        }
        if let Some(v) = bounds.iter().find_map(|b| self.variable_named_in(b)) {
            return format!(
                "{what} does not fold to a constant: it reads `{v}`, a variable, where a \
                 constant expression is required"
            );
        }
        if let Some(name) = bounds.iter().find_map(|b| self.wide_param_named_in(b)) {
            return format!(
                "{what} does not fold to a constant: it reads {name}, a parameter wider \
                 than 64 bits, and in a constant width or count vita evaluates only a bare \
                 select of such a parameter, not an expression over its bits (assign the bits \
                 you need to a narrower localparam and use that)"
            );
        }
        match reason {
            Some(r) => format!("{what} does not fold to a constant: {r}"),
            None => format!("{what} does not fold to a constant"),
        }
    }

    /// The first `f(node)` answer over `e` and every sub-expression, parent first —
    /// every child `hdl_ast::walk` enumerates (casts and concatenations included), not
    /// only the constant domain's. Diagnostic and decline walks only.
    fn first_in<T>(e: &ast::Expr, f: &mut impl FnMut(&ast::Expr) -> Option<T>) -> Option<T> {
        if let Some(t) = f(e) {
            return Some(t);
        }
        let mut found = None;
        e.for_each_child(|_, c| {
            if found.is_none() {
                found = Self::first_in(c, f);
            }
        });
        found
    }

    /// Does `e`'s value, or a parameter (or a select of one) it reads, hold an x or z bit
    /// in the wide bit domain? A literal's own `?` / `z` digits are not asked about: in a
    /// wildcard pattern (`==? 4'b1?00`) they are don't-cares. Diagnostic-only.
    fn wide_domain_has_unknown(&self, e: &ast::Expr) -> bool {
        let unknown = |r: Option<WideBits>| matches!(r, Some((b, w, _)) if bp_any_unknown(&b, w));
        unknown(fold_self_bits(e, &|n, _| self.wide_name_bits(n)))
            || Self::first_in(e, &mut |x| unknown(self.wide_name_bits(x)).then_some(())).is_some()
    }

    /// The first plain (one-segment) name in `e` bound to a net or variable rather than a
    /// constant — a run-time value in a width or count. Diagnostic-only.
    fn variable_named_in(&self, e: &ast::Expr) -> Option<String> {
        Self::first_in(e, &mut |x| {
            let ast::ExprKind::Ident(p) = &x.kind else {
                return None;
            };
            let [seg] = p.segments.as_slice() else {
                return None;
            };
            let n = &seg.name;
            // the classification `nonconst_bound_reason` makes for a declaration bound
            let constant = self.lookup_scoped(n).is_some()
                || self
                    .walk_scopes_key(n, |k| self.wide_param_bits.contains_key(k))
                    .is_some();
            // a scalar net only: an unpacked array here may be a constant array
            // parameter, which this classification cannot tell from a variable
            let scalar_net = self
                .lookup_net_scoped(n)
                .and_then(|id| self.nets.get(id as usize))
                .is_some_and(|nv| nv.array_len <= 1);
            (!constant
                && self.subst_lookup(n).is_none()
                && scalar_net
                && self.const_array_vals_of_base(x).is_none())
            .then(|| source_name(n).to_string())
        })
    }

    /// The first hierarchical (dotted) name in `e`. Diagnostic-only.
    fn hier_name_in(e: &ast::Expr) -> Option<String> {
        Self::first_in(e, &mut |x| match &x.kind {
            ast::ExprKind::Ident(p) if p.segments.len() > 1 => Some(
                p.segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            _ => None,
        })
    }

    /// The first name in `e` bound to a parameter DECLARED wider than 64 bits — one whose
    /// value lives in the wide map, or one whose value fits the i64 lane under a wider
    /// declared range (`parameter prm_t pt = 80'h…05_02`) — reached through a cast or a
    /// concatenation too. Diagnostic-only.
    fn wide_param_named_in(&self, e: &ast::Expr) -> Option<String> {
        Self::first_in(e, &mut |x| {
            if let ast::ExprKind::Ident(p) = &x.kind {
                if let [seg] = p.segments.as_slice() {
                    if self
                        .walk_scopes_key(&seg.name, |k| self.wide_param_bits.contains_key(k))
                        .is_some()
                    {
                        return Some(format!("`{}`", seg.name));
                    }
                }
            }
            self.param_sel_range(x)
                .is_some_and(|(_, width, _)| width > 64)
                .then(|| Self::expr_brief(x))
        })
    }

    /// The first name in `e` bound to a parameter declared wider than 64 bits whose
    /// declared range does not start at bit 0 (or whose range vita does not record), with
    /// that range's text. A select of such a parameter is read by POSITION from bit 0 in
    /// the constant and the run-time lanes alike (ROADMAP §2 "A bare >64-bit non-zero-LSB
    /// parameter select reads positionally"), so the bound funnel declines it and the gate
    /// refuses the width or count.
    pub(crate) fn wide_lsb_param_in(&self, e: &ast::Expr) -> Option<(String, String)> {
        Self::first_in(e, &mut |x| {
            let ast::ExprKind::Ident(p) = &x.kind else {
                return None;
            };
            let [seg] = p.segments.as_slice() else {
                return None;
            };
            let n = &seg.name;
            if self.subst_lookup(n).is_some() || self.out_subst_lookup(n).is_some() {
                return None;
            }
            self.wide_lsb_range_of(n).map(|range| (n.clone(), range))
        })
    }

    fn wide_lsb_range_of(&self, n: &str) -> Option<String> {
        let key = self.walk_scopes_key(n, |k| {
            self.wide_param_bits.contains_key(k)
                || self.params.contains_key(k)
                || self.symbols.contains_key(k)
        })?;
        let text = |(lo, w, asc): DeclRange| {
            let hi = lo + i64::from(w) - 1;
            if asc {
                format!("[{lo}:{hi}]")
            } else {
                format!("[{hi}:{lo}]")
            }
        };
        if self.wide_param_bits.contains_key(&key) {
            return match self.hier_param_range.get(&key).copied() {
                None => Some("with a range vita does not record".to_string()),
                Some(r) if r.0 != 0 || r.2 => Some(text(r)),
                Some(_) => None,
            };
        }
        if !self.params.contains_key(&key) {
            return None;
        }
        match self.param_range.get(&key).copied() {
            Some(r) if r.1 > 64 && (r.0 != 0 || r.2) => Some(text(r)),
            _ => None,
        }
    }
}

/// The name the source wrote: the parser renames a `for (int k …)` loop variable to
/// `__forvar_<name>_<offset>` (`hdl-parser` `stmt_ctl.rs`) so it never aliases another.
fn source_name(n: &str) -> &str {
    n.strip_prefix("__forvar_")
        .and_then(|rest| rest.rsplit_once('_'))
        .map_or(n, |(name, _)| name)
}

fn ir_binop_text(op: ir::BinOp) -> &'static str {
    use ir::BinOp as B;
    match op {
        B::Add => "+",
        B::Sub => "-",
        B::Mul => "*",
        B::Div => "/",
        B::Mod => "%",
        B::Pow => "**",
        B::BitAnd => "&",
        B::BitOr => "|",
        B::BitXor => "^",
        B::BitXnor => "~^",
        B::LogAnd => "&&",
        B::LogOr => "||",
        B::Lt => "<",
        B::Le => "<=",
        B::Gt => ">",
        B::Ge => ">=",
        B::Eq => "==",
        B::Ne => "!=",
        B::CaseEq => "===",
        B::CaseNe => "!==",
        B::Shl => "<<",
        B::Shr => ">>",
        B::AShl => "<<<",
        B::AShr => ">>>",
        _ => "an operator",
    }
}

fn ir_unop_text(op: ir::UnOp) -> &'static str {
    use ir::UnOp as U;
    match op {
        U::Minus => "-",
        U::BitNot => "~",
        U::LogNot => "!",
        _ => "an operator",
    }
}

#[cfg(test)]
mod tests {
    //! The backstop is reached by no producer today (§4.5.601's census), so it is pinned
    //! here by pushing what no producer pushes; and the late materialization is pinned on
    //! a root that is both an edge and a value.
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Collect(RefCell<Vec<String>>);
    impl LogSink for Collect {
        fn emit(&self, event: LogEvent) {
            if let LogEvent::Diagnostic(d) = event {
                self.0
                    .borrow_mut()
                    .push(format!("{:?} {}", d.code, d.message));
            }
        }
    }

    fn errors(sink: &Collect) -> Vec<String> {
        sink.0.borrow().clone()
    }

    fn select(el: &mut Elaborator<'_>, width: u32) -> u32 {
        let base = el.const_u32_expr(0xff, 8);
        let offset = el.const_u32_expr(0, 32);
        el.push_expr(ir::Expr::Select {
            base,
            offset,
            width,
            kind: ir::SelKind::PartConst,
        })
    }

    #[test]
    fn backstop_refuses_an_edge_no_funnel_decided() {
        let sink = Collect::default();
        let mut el = Elaborator::new(&sink);
        let a = el.const_u32_expr(2, 32);
        let b = el.const_u32_expr(3, 32);
        // `*` is outside the engine's shallow fold: the reader would select one bit.
        let width = el.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Mul,
            lhs: a,
            rhs: b,
        });
        select(&mut el, width);
        // An exact `(6 - 2) + 1` tree no funnel recorded is refused too: only decided
        // edges reach the engine.
        let m = el.const_u32_expr(6, 32);
        let l = el.const_u32_expr(2, 32);
        let d = el.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Sub,
            lhs: m,
            rhs: l,
        });
        let one = el.const_u32_expr(1, 32);
        let exact = el.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: d,
            rhs: one,
        });
        select(&mut el, exact);
        el.edge_backstop();
        let e = errors(&sink);
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].contains("ElabUnsupported 2 part-select"), "{e:?}");
        assert!(e[0].contains("no source location was recorded"), "{e:?}");
    }

    #[test]
    fn backstop_refuses_an_unknown_const_and_a_part_chunk_with_no_width() {
        let sink = Collect::default();
        let mut el = Elaborator::new(&sink);
        let cid = el.intern_const(ir::ConstVal {
            width: 32,
            signed: false,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val: vec![4],
                unk: vec![1],
            },
        });
        let xw = el.push_expr(ir::Expr::Const { val: cid });
        select(&mut el, xw);
        let off = el.const_u32_expr(0, 32);
        let rhs = el.const_u32_expr(1, 1);
        el.push_stmt(ir::Stmt::BlockingAssign {
            lhs: ir::Lvalue {
                chunks: vec![ir::LvalChunk {
                    net: 0,
                    word: None,
                    offset: Some(off),
                    width: None,
                    kind: ir::SelKind::PartConst,
                }],
            },
            rhs,
        });
        el.edge_backstop();
        let e = errors(&sink);
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(e[0].starts_with("ElabUnsupported 2 part-select"), "{e:?}");
    }

    #[test]
    fn backstop_accepts_constants_and_funnel_edges() {
        let sink = Collect::default();
        let mut el = Elaborator::new(&sink);
        let c = el.const_u32_expr(4, 32);
        select(&mut el, c);
        // a decided tree and a kept one (the negative-literal split)
        let m = el.const_u32_expr(6, 32);
        let l = el.const_u32_expr(2, 32);
        let decided = el.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Sub,
            lhs: m,
            rhs: l,
        });
        el.push_edge(
            decided,
            Shape::Indexed(decided),
            EdgeKind::IndexedWidth,
            State::Decided(4),
            None,
            &[],
        );
        select(&mut el, decided);
        let neg = el.push_expr(ir::Expr::Unary {
            op: ir::UnOp::Minus,
            operand: l,
        });
        el.push_edge(
            neg,
            Shape::Part {
                msb: Bound::Ast(u32::MAX),
                lsb: Bound::Ast(0),
                desc: true,
            },
            EdgeKind::PartWidth,
            State::Kept,
            None,
            &[],
        );
        select(&mut el, neg);
        el.edge_backstop();
        assert!(errors(&sink).is_empty(), "{:?}", errors(&sink));
    }

    /// RC4 / R4: a late root that is both a replication count and a value in a wider
    /// context (an inline formal bound verbatim to `gb.A + gb.B`). The decision repoints
    /// the count to a fresh `Const`; the value keeps the tree, so its 64-bit context sum
    /// keeps the carry the 32-bit count drops.
    #[test]
    fn a_late_decision_repoints_edges_and_leaves_values_on_the_root() {
        let sink = Collect::default();
        let mut el = Elaborator::new(&sink);
        // gb.A = gb.B = 32'h8000_0002 (unsigned): the 32-bit sum wraps to 4
        let a = el.const_u32_expr(0x8000_0002, 32);
        let b = el.const_u32_expr(0x8000_0002, 32);
        let root = el.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: a,
            rhs: b,
        });
        el.push_edge(
            root,
            Shape::Count {
                n: root,
                zero_ok: false,
            },
            EdgeKind::RepCount,
            State::Late,
            None,
            &[],
        );
        let one = el.const_u32_expr(1, 1);
        let value = el.push_expr(ir::Expr::Concat { parts: vec![one] });
        let rep = el.push_expr(ir::Expr::Replicate { count: root, value });
        let wide = el.const_u32_expr(0, 32);
        let wide = el.push_expr(ir::Expr::Concat {
            parts: vec![wide, wide],
        });
        let sum = el.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: root,
            rhs: wide,
        });
        let root_before = el.exprs[root as usize].clone();
        el.decide_late_edges();
        assert!(errors(&sink).is_empty(), "{:?}", errors(&sink));
        let ir::Expr::Replicate { count, .. } = el.exprs[rep as usize].clone() else {
            panic!("replicate moved");
        };
        assert_ne!(count, root, "the count is repointed");
        assert_eq!(el.const_of_expr_u32(count), Some(4));
        assert_eq!(
            el.exprs[root as usize], root_before,
            "the root slot is untouched"
        );
        let ir::Expr::Binary { lhs, .. } = el.exprs[sum as usize].clone() else {
            panic!("sum moved");
        };
        assert_eq!(lhs, root, "the value keeps the tree");
        el.edge_backstop();
        assert!(errors(&sink).is_empty(), "{:?}", errors(&sink));
    }
}
