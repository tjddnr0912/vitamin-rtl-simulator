//! §5.2 row 22 (§4.5.621): a concatenation-target assignment `{a, b} = e;` in a FRAME
//! body — a function or task that runs in its own activation window.
//!
//! The frame write path (`frame_write_lvalue`) writes ONE chunk, so a multi-chunk
//! lvalue was refused in a frame body (E3009, `frames_classify.rs`), and a suspendable
//! task that lifted past that refusal wrote it through the flat store at exit 0
//! (`{hi, lo} = 8'hb6` printed `r=21` where all three oracles print `r=b6`). Here the
//! assignment is split at lowering into `tmp = e;` — a frame-local capture net, so `e`
//! evaluates once and `{a, b} = {b, a}` swaps — and one single-chunk write per target,
//! MSB first, each taking its slice of `tmp`: the slicing and the order the
//! module-scope engine write uses (`native/write.rs`, the multi-chunk funnel).

use super::*;

impl Elaborator<'_> {
    /// Reserve one frame-local capture net per concatenation-target blocking
    /// assignment in `body`, keyed by the statement's span and the frame's `owner`.
    ///
    /// It has to be reserved here, before the frame's window of net ids closes, at a
    /// placeholder width the lowering patches — the same handoff, for the same
    /// reasons, as `reserve_frame_case_tmps`. An assignment the lowering then declines
    /// costs one unused slot.
    pub(crate) fn reserve_frame_concat_tmps(&mut self, body: &ast::Stmt, owner: u32) {
        let mut spans = Vec::new();
        Self::collect_concat_assign_spans(body, &mut spans);
        for sp in spans {
            let name = format!("$ia_tmp${}", self.nets.len());
            let nv = ir::NetVar {
                kind: ir::NetKind::Reg,
                width: 1,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, 1),
            };
            self.add_net(&name, nv);
            let net = (self.nets.len() - 1) as u32;
            self.frame_concat_tmp
                .entry(sp)
                .or_default()
                .insert(owner, net);
        }
    }

    /// Spans of every plain `{…} = e;` (no intra-assignment timing) in `s`, at any
    /// depth, over the same statement containers `collect_case_spans` walks.
    fn collect_concat_assign_spans(s: &ast::Stmt, out: &mut Vec<(u32, u32)>) {
        use ast::Stmt as S;
        match s {
            S::Blocking {
                lhs: ast::Lvalue::Concat { .. },
                delay: None,
                event: None,
                span,
                ..
            } => out.push((span.lo, span.hi)),
            S::Case { items, .. } => {
                for it in items {
                    let (ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. }) =
                        it;
                    Self::collect_concat_assign_spans(body, out);
                }
            }
            S::Block { stmts, .. } | S::Fork { stmts, .. } => {
                for st in stmts {
                    Self::collect_concat_assign_spans(st, out);
                }
            }
            S::If { then_s, else_s, .. } => {
                Self::collect_concat_assign_spans(then_s, out);
                if let Some(e) = else_s {
                    Self::collect_concat_assign_spans(e, out);
                }
            }
            S::For { body, .. }
            | S::While { body, .. }
            | S::Repeat { body, .. }
            | S::Forever { body, .. } => Self::collect_concat_assign_spans(body, out),
            S::DelayCtrl { body, .. } | S::EventCtrl { body, .. } | S::Wait { body, .. } => {
                if let Some(b) = body {
                    Self::collect_concat_assign_spans(b, out);
                }
            }
            _ => {}
        }
    }

    /// Lower `lv = rhs_id` — a multi-chunk lvalue in a frame body, `rhs_id` already
    /// sized to the lvalue — as `tmp = rhs_id` plus one single-chunk write per chunk,
    /// returning the statements in order. `None` leaves the multi-chunk write, which
    /// the frame validator keeps loud:
    ///
    /// * no reserved capture for this statement in this frame (owner-checked, as
    ///   `hoist_case_scrutinee` checks its capture);
    /// * a chunk without a decided constant width;
    /// * a `real` or string target, which has no bit slice to take;
    /// * a chunk whose index, offset or width reads a target of the same
    ///   concatenation (`{i, loc[i]} = e`), a net outside this frame, or anything
    ///   through a call: split into sequential writes it could read the NEW value,
    ///   where vita's module path and iverilog read it first (verilator does not —
    ///   an oracle split this lane must not pick).
    pub(crate) fn split_frame_concat_write(
        &mut self,
        span: ast::Span,
        lv: &ir::Lvalue,
        rhs_id: u32,
    ) -> Option<Vec<u32>> {
        if !self.in_frame_body || lv.chunks.len() < 2 {
            return None;
        }
        let tmp = *self
            .cur_frame_owner
            .and_then(|o| self.frame_concat_tmp.get(&(span.lo, span.hi))?.get(&o))?;
        let widths: Vec<u32> = lv
            .chunks
            .iter()
            .map(|c| self.chunk_width(c).and_then(|w| u32::try_from(w).ok()))
            .collect::<Option<_>>()?;
        if widths.contains(&0) {
            return None;
        }
        if lv.chunks.iter().any(|c| {
            self.is_non_bit_addressable_target(c.net)
                || self.nets.get(c.net as usize).map(|n| n.kind) == Some(ir::NetKind::Real)
        }) {
            return None;
        }
        // A chunk's index, offset or width may read only this frame's own locals —
        // never a target of the same concatenation, and never through a call: a
        // module net can be the same storage as a target under another spelling
        // (`{m, mv[top.m]}`), and a called function can read a target (`mv[getm()]`),
        // both of which the read set does not show (§4.5.621 review).
        let owner = self.cur_frame_owner?;
        let hi = owner
            + self
                .func_metas
                .iter()
                .find(|m| m.base_net == owner)
                .map(|m| m.locals_len)?;
        let targets: std::collections::BTreeSet<u32> = lv.chunks.iter().map(|c| c.net).collect();
        for c in &lv.chunks {
            let mut reads = std::collections::BTreeSet::new();
            let mut calls = Vec::new();
            for e in [c.word, c.offset, c.width].into_iter().flatten() {
                self.collect_expr_reads_calls(e, &mut reads, &mut calls);
            }
            if !calls.is_empty()
                || reads
                    .iter()
                    .any(|&n| targets.contains(&n) || n < owner || n >= hi)
            {
                return None;
            }
        }
        let total: u32 = widths.iter().sum();
        if let Some(nv) = self.nets.get_mut(tmp as usize) {
            nv.width = total;
            nv.msb = total - 1;
            nv.signed = false;
            nv.init = default_init(ast::NetVarKind::Reg, total);
        }
        let mut out = vec![self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: whole_net_lvalue(tmp),
            rhs: rhs_id,
        })];
        let mut off = total;
        for (c, &w) in lv.chunks.iter().zip(&widths) {
            off -= w;
            let base = self.push_expr(ir::Expr::Signal {
                net: tmp,
                word: None,
            });
            let offset = self.const_u32_expr(off, 32);
            let width = self.const_u32_expr(w, 32);
            let slice = self.push_expr(ir::Expr::Select {
                base,
                offset,
                width,
                kind: ir::SelKind::PartConst,
            });
            out.push(self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: ir::Lvalue {
                    chunks: vec![c.clone()],
                },
                rhs: slice,
            }));
        }
        Some(out)
    }
}
