//! A reversed constant part-select in a procedural `if` arm the run can never enter
//! (ROADMAP §5.2 row 17, C-R02).
//!
//! IEEE 1800 §11.5.1: "The first expression shall address a more significant bit than
//! the second expression", so `x[7:8]` on a descending `x` is refused (E3009) — in
//! Icarus Verilog too (`part select tid[7:8] is reversed`). A parameterized design
//! writes such a select under a guard that removes it for the values that reverse it:
//! `if (S_COUNT > 1) tid[W-1:W-CL] = g;` with `CL = $clog2(S_COUNT) = 0`. The `if` is
//! decided at elaboration in Icarus Verilog (a constant condition elaborates one arm)
//! and verilator only warns (SELRANGE); both run the design, and so does the generate
//! spelling of the same guard, which elaborates no select at all.
//!
//! What changes, and only this: while lowering the arm of an `if` whose condition is a
//! constant the run can never make true (or, for the `else` arm, never false), the
//! direction check of a `[m:l]` select does not refuse. The select lowers with the
//! width its bounds span as a plain `Const` (never read: the arm never runs) and is
//! recorded. When the arm is lowered, every recorded select must be held by a
//! statement or a terminator of the arm's own blocks; one that is not — lowered for
//! code that runs elsewhere, or held where this walk does not look (a hierarchical
//! select resolved after the deferred passes, an event control's expression) — is
//! refused there and then with the message it would have had.
//!
//! The arm is dead when the LOWERED condition folds: the tree the run's `Branch`
//! evaluates, so every name in it is already bound the way the run binds it (a formal, a
//! block-local or a class property is a read, not a constant). The fold takes only known
//! integral constants of at most 64 bits under `!`, `&&`, `||` and the relational and
//! equality operators, compared at the wider width and signed only when both operands
//! are (§11.8.2); anything else — a variable, a call, `$clog2`, arithmetic, a unary minus,
//! an x or z bit, a real or string constant, and a placeholder (`$bits(u.X)` is a `Const`
//! 32 until a deferred pass writes the real width, row 17 round 1) — leaves both arms
//! live, and the select refused.
//!
//! Every other check in the arm runs as before: a zero indexed width (row 65), an
//! undeclared name, a reversed select on a multi-dimensional packed array, and a
//! reversed select anywhere the statement can run all stay loud.

use super::*;

/// The refusal text of a `[m:l]` select against a descending net.
pub(crate) const ASCEND_ON_DESCENDING: &str =
    "part-select bounds [msb:lsb] ascend but the net is descending [hi:lo] (out of order)";
/// The refusal text of a `[m:l]` select against an ascending net.
pub(crate) const DESCEND_ON_ASCENDING: &str =
    "part-select bounds [msb:lsb] descend but the net is ascending [lo:hi] (out of order)";

/// A reversed select lowered inside a dead arm, with what its refusal would have said.
pub(crate) struct DeadSelect {
    /// The `Const` width the select holds.
    edge: u32,
    span: Option<ast::Span>,
    prefix: String,
    msg: &'static str,
}

/// `v`'s low `w` bits (`1 ≤ w ≤ 64`) read as a two's-complement number.
fn sext(v: u64, w: u32) -> i64 {
    if w >= 64 {
        v as i64
    } else {
        let sh = 64 - w;
        ((v << sh) as i64) >> sh
    }
}

impl Elaborator<'_> {
    /// A lowered `Const` with known bits, at most 64 wide: `(value, width, signed)`. A
    /// placeholder is not one: `$bits(u.X)` lowers to a `Const` 32 that a deferred pass
    /// rewrites to the real width after the arm is lowered, and a constant an error left
    /// behind is not a value (the guard `tree_marks` gives the edge decision).
    fn known_leaf(&self, eid: u32) -> Option<(u64, u32, bool)> {
        if self.undecided_placeholders.contains(&eid) || self.error_placeholders.contains(&eid) {
            return None;
        }
        let ir::Expr::Const { val } = self.exprs.get(eid as usize)? else {
            return None;
        };
        let c = self.consts.get(*val as usize)?;
        if !matches!(c.repr, ir::ConstRepr::Numeric) || c.width == 0 || c.width > 64 {
            return None;
        }
        if c.bits.unk.iter().any(|&u| u != 0) {
            return None;
        }
        let v = c.bits.val.first().copied().unwrap_or(0);
        let mask = if c.width == 64 {
            u64::MAX
        } else {
            (1u64 << c.width) - 1
        };
        Some((v & mask, c.width, c.signed))
    }

    /// The truth of a lowered condition built only of known constants (see the module
    /// doc), else `None`.
    fn lowered_truth(&self, eid: u32, depth: u32) -> Option<bool> {
        if depth > 32 {
            return None;
        }
        match self.exprs.get(eid as usize)? {
            ir::Expr::Const { .. } => self.known_leaf(eid).map(|(v, _, _)| v != 0),
            ir::Expr::Unary {
                op: ir::UnOp::LogNot,
                operand,
            } => self.lowered_truth(*operand, depth + 1).map(|t| !t),
            ir::Expr::Binary {
                op: op @ (ir::BinOp::LogAnd | ir::BinOp::LogOr),
                lhs,
                rhs,
            } => {
                let a = self.lowered_truth(*lhs, depth + 1)?;
                let b = self.lowered_truth(*rhs, depth + 1)?;
                Some(if *op == ir::BinOp::LogAnd {
                    a && b
                } else {
                    a || b
                })
            }
            ir::Expr::Binary { op, lhs, rhs } => {
                let (a, aw, asg) = self.known_leaf(*lhs)?;
                let (b, bw, bsg) = self.known_leaf(*rhs)?;
                // §11.8.2: the operands are compared at the wider width, each extended
                // by its own sign only when both are signed (the expression is unsigned
                // otherwise, and an unsigned operand is zero-extended).
                let ord = if asg && bsg {
                    sext(a, aw).cmp(&sext(b, bw))
                } else {
                    a.cmp(&b)
                };
                match op {
                    ir::BinOp::Lt => Some(ord.is_lt()),
                    ir::BinOp::Le => Some(ord.is_le()),
                    ir::BinOp::Gt => Some(ord.is_gt()),
                    ir::BinOp::Ge => Some(ord.is_ge()),
                    ir::BinOp::Eq | ir::BinOp::CaseEq => Some(ord.is_eq()),
                    ir::BinOp::Ne | ir::BinOp::CaseNe => Some(ord.is_ne()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The constant truth of a procedural `if` condition lowered to `cond_id`, else `None`
    /// (both arms live).
    pub(crate) fn constant_if_truth(&self, cond_id: u32) -> Option<bool> {
        self.lowered_truth(cond_id, 0)
    }

    /// Lower one arm of a procedural `if` that opened at `entry`. A `dead` arm (its
    /// condition can never select it) lowers with reversed `[m:l]` selects recorded
    /// instead of refused, then settles them (module doc).
    pub(crate) fn lower_if_arm(
        &mut self,
        b: &mut ProcessBuilder,
        entry: BlockId,
        s: &ast::Stmt,
        dead: bool,
    ) {
        if !dead {
            self.lower_stmt(b, s);
            return;
        }
        let first_new = b.body.len();
        let mark = self.dead_selects.len();
        self.dead_arm_depth += 1;
        self.lower_stmt(b, s);
        self.dead_arm_depth -= 1;
        self.settle_dead_arm(b, entry, first_new, mark);
    }

    /// Inside a dead arm, a reversed `[m:l]` select spanning `lo..=hi` lowers to a `Const`
    /// width (returned) and is recorded; `None` outside one, or for a span no net can
    /// hold, where the caller refuses as before.
    pub(crate) fn dead_reversed_select(
        &mut self,
        msg: &'static str,
        lo: u32,
        hi: u32,
    ) -> Option<u32> {
        if self.dead_arm_depth == 0 {
            return None;
        }
        let w = (u64::from(hi) - u64::from(lo)).checked_add(1)?;
        if w > MAX_NET_WIDTH {
            return None;
        }
        let edge = self.const_u32_expr(w as u32, 32);
        self.dead_selects.push(DeadSelect {
            edge,
            span: self.cur_span,
            prefix: self.cur_prefix.clone(),
            msg,
        });
        Some(edge)
    }

    /// Keep the selects recorded since `mark` that a statement or terminator of the arm's
    /// blocks (`entry` and every block from `first_new` on) holds; refuse the others.
    fn settle_dead_arm(
        &mut self,
        b: &ProcessBuilder,
        entry: BlockId,
        first_new: usize,
        mark: usize,
    ) {
        if self.dead_selects.len() == mark {
            return;
        }
        let blocks = std::iter::once(entry.raw() as usize).chain(first_new..b.body.len());
        let held = self.edges_held_by(b, blocks);
        let pending: Vec<DeadSelect> = self.dead_selects.drain(mark..).collect();
        for d in pending {
            if held.contains(&d.edge) {
                continue;
            }
            let span = std::mem::replace(&mut self.cur_span, d.span);
            let prefix = std::mem::replace(&mut self.cur_prefix, d.prefix);
            self.error(MsgCode::ElabUnsupported, d.msg);
            self.cur_span = span;
            self.cur_prefix = prefix;
        }
    }

    /// Every part-select width a statement or terminator of `blocks` holds: a select's in
    /// an expression it evaluates, a part chunk's in a target it writes.
    fn edges_held_by(
        &self,
        b: &ProcessBuilder,
        blocks: impl Iterator<Item = usize>,
    ) -> BTreeSet<u32> {
        fn lval(lv: &ir::Lvalue, held: &mut BTreeSet<u32>, roots: &mut Vec<u32>) {
            for c in &lv.chunks {
                if c.kind != ir::SelKind::Bit {
                    held.extend(c.width);
                }
                roots.extend(c.word.iter().chain(&c.offset).chain(&c.width));
            }
        }
        let mut held = BTreeSet::new();
        let mut roots: Vec<u32> = Vec::new();
        for k in blocks {
            let Some(blk) = b.body.get(k) else {
                continue;
            };
            for &sid in &blk.stmts {
                match self.stmts.get(sid as usize) {
                    Some(ir::Stmt::BlockingAssign { lhs, rhs } | ir::Stmt::Force { lhs, rhs }) => {
                        lval(lhs, &mut held, &mut roots);
                        roots.push(*rhs);
                    }
                    Some(ir::Stmt::NonblockingAssign { lhs, rhs, delay }) => {
                        lval(lhs, &mut held, &mut roots);
                        roots.push(*rhs);
                        roots.extend(delay);
                    }
                    Some(ir::Stmt::Release { lhs }) => lval(lhs, &mut held, &mut roots),
                    Some(ir::Stmt::SysTask { fmt, args, .. }) => {
                        roots.extend(fmt);
                        roots.extend(args);
                    }
                    Some(ir::Stmt::Disable { .. }) | None => {}
                }
            }
            match &blk.term {
                ir::Terminator::Branch { cond, .. } => roots.push(*cond),
                ir::Terminator::Delay { amount, .. } => roots.push(*amount),
                ir::Terminator::Wait {
                    cond: ir::WaitCause::Expr { expr },
                    ..
                } => roots.push(*expr),
                _ => {}
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(e) = roots.pop() {
            if !seen.insert(e) {
                continue;
            }
            match self.exprs.get(e as usize) {
                Some(ir::Expr::Select {
                    base,
                    offset,
                    width,
                    kind,
                }) => {
                    if *kind != ir::SelKind::Bit {
                        held.insert(*width);
                    }
                    roots.extend([*base, *offset, *width]);
                }
                Some(ir::Expr::Signal { word, .. }) => roots.extend(word),
                Some(ir::Expr::Concat { parts }) => roots.extend(parts),
                Some(ir::Expr::Replicate { count, value }) => roots.extend([*count, *value]),
                Some(ir::Expr::Unary { operand, .. }) => roots.push(*operand),
                Some(ir::Expr::Binary { lhs, rhs, .. }) => roots.extend([*lhs, *rhs]),
                Some(ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                }) => roots.extend([*cond, *then_e, *else_e]),
                Some(ir::Expr::SysFunc { args, .. } | ir::Expr::Call { args, .. }) => {
                    roots.extend(args)
                }
                Some(ir::Expr::Const { .. } | ir::Expr::ArrayItem { .. }) | None => {}
            }
        }
        held
    }
}
