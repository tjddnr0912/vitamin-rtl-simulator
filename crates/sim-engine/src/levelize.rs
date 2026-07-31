//! Combinational levelization — a static rank per process template.
//!
//! **The problem.** A chain of `D` combinational stages costs `≈ D²/2` process
//! activations per cycle, not `D`. Each stage runs once on STALE inputs, then again
//! on the next delta once its upstream settled, so the per-cycle activation profile
//! of a depth-6 chain is the triangle `7 6 5 4 3 2 1 1 1 1`. Sorting the active batch
//! does not fix it — measured identical at 4.86 / 4.85 s ascending vs descending
//! (§4.5.278) — because the stage-to-stage propagation crosses the DELTA boundary
//! (`settle_cont_assigns`), not merely a within-batch ordering.
//!
//! **The fix that was measured, and REJECTED (2026-08-01).** A rank-ordered Active
//! drain with inter-rank settle was built on top of this rank and measured at **1.00x**
//! on exactly the shape it targets, at every depth from 1 to 24. It was reverted rather
//! than shipped as a knob that does nothing (same call as the `Value::resize` one-word
//! fast path in §4.5.278).
//!
//! The reason is that the depth cost is not where §4.5.278 said it was. Holding total
//! cycles fixed and varying only depth (`perf_depth_cost_shape`):
//!
//! | depth | one module, no cont-assigns | instances chained through port cont-assigns |
//! |---|---|---|
//! | 1  |  3.3 ms |   7.8 ms |
//! | 6  |  8.4 ms |  71.2 ms |
//! | 12 | 15.3 ms | 229.6 ms |
//! | 24 | 31.7 ms | 814.4 ms |
//!
//! A pure `always_comb` chain is **linear** in depth — 24 stages of work cost 9.6x one
//! stage, i.e. no depth penalty at all — and its wake chain is naturally one process per
//! delta, so there is no batch to rank-order in the first place. The quadratic appears
//! only once the stages are chained through CONTINUOUS ASSIGNS: 104x for 24x the depth.
//!
//! That points at `settle_cont_assigns`, which makes a FULL pass over every continuous
//! assign, to fixpoint, on EVERY delta. A depth-D chain needs D deltas to propagate and
//! carries O(D) assigns, so the settle work is paid D times over D assigns = O(D²) — and
//! no amount of reordering the process drain touches it. The lever is a dirty-driven
//! settle (re-evaluate only assigns whose RHS nets changed), the same shape as the
//! dirty-list that took `propagate_changes` from 305 ms to 15.5 ms.
//!
//! **This matters for more than performance.** A dirty-driven settle preserves
//! declaration order among the assigns it does evaluate and never touches process
//! execution order — so unlike levelization it needs NO golden re-adjudication.
//!
//! What remains here is the rank itself, which is correct, cheap, and reports a design's
//! combinational depth — the number this whole line of investigation was plotted against.
//!
//! **The edge that matters is not "who writes what I read".** It is:
//!
//! > `p → q` iff `p` writes a net with a BLOCKING write that `q` is LEVEL-sensitive to.
//!
//! Both qualifiers are load-bearing, and dropping either reintroduces a silent-wrong:
//!
//! - **NBA writes make no edge.** A nonblocking write lands in the NBA region, which
//!   is already a delta boundary levelization does not cross. Treating `q <= d` as a
//!   combinational edge would rank the stages of a shift register and run them in
//!   dependency order within one delta — which is exactly how a shift register
//!   collapses (measured: `q0=0 q1=1` becomes `q0=0 q1=0` when process order moves).
//! - **Edge-sensitive readers make no edge.** `@(posedge clk)` IS the sequential
//!   boundary. A clocked process must not be ranked behind whoever last wrote its
//!   inputs; it runs when the edge says so.
//!
//! So `SensKind::{Comb, Latch, Level}` readers take edges and `SensKind::Edge` readers
//! do not, and only `Stmt::BlockingAssign` contributes writes.
//!
//! **Cycles.** A combinational loop (or a mutually-sensitive pair) has no topological
//! rank. Kahn's algorithm leaves exactly those nodes unranked, and they keep rank 0 —
//! they drain first, which is the pre-levelize behaviour, so a cyclic design is no
//! worse off than it is today. The cont-assign fixpoint continues to handle its own
//! convergence independently.
//!
//! Determinism: derived purely from the frozen `SimIr`, computed at engine init, and
//! built over ordered containers only. It never enters `SimIr`, so the golden hash and
//! `format_version` are untouched.

use std::collections::{BTreeMap, BTreeSet};

use sim_ir::{SensKind, SimIr, Stmt};

/// Nets a process writes with a BLOCKING assignment (the only writes that propagate
/// within the current delta). `Force`/`Release` are shape-reserved no-ops today and
/// deliberately contribute nothing.
fn blocking_writes(ir: &SimIr, tmpl: usize) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for block in &ir.processes[tmpl].body {
        for &sid in &block.stmts {
            if let Stmt::BlockingAssign { lhs, .. } = &ir.stmts[sid as usize] {
                for c in &lhs.chunks {
                    out.insert(c.net);
                }
            }
        }
    }
    out
}

/// Nets a process is LEVEL-sensitive to. For `Comb`/`Latch` the `edges` list holds the
/// elaborate-inferred read set; for `Level` it is the written `@(a, b)` list. An
/// `Edge` process returns empty — it is the sequential boundary, not a rank consumer.
fn level_reads(ir: &SimIr, tmpl: usize) -> BTreeSet<u32> {
    let s = &ir.processes[tmpl].sensitivity;
    match s.kind {
        SensKind::Comb | SensKind::Latch | SensKind::Level => {
            s.edges.iter().map(|e| e.net).collect()
        }
        SensKind::Edge | SensKind::Initial => BTreeSet::new(),
    }
}

/// Nets read by the expression rooted at `eid`.
///
/// The match is EXHAUSTIVE with no `_` arm on purpose. A walker that silently drops a
/// variant under-detects, and an under-detecting walker is how three silent-wrongs got
/// in during round-19 — here it would only cost levelization (a missed edge lowers a
/// rank, which degrades to the pre-levelize order), but the habit is the point: a new
/// `Expr` variant must fail to compile here rather than quietly read as "reads nothing".
fn expr_nets(ir: &SimIr, eid: u32, out: &mut BTreeSet<u32>) {
    use sim_ir::Expr as E;
    match &ir.exprs[eid as usize] {
        E::Signal { net, .. } => {
            out.insert(*net);
        }
        // `word` on a Signal is a CONSTANT word index, not an ExprId — nothing to walk.
        E::Const { .. } | E::ArrayItem { .. } => {}
        E::Select {
            base,
            offset,
            width: _,
            kind: _,
        } => {
            expr_nets(ir, *base, out);
            expr_nets(ir, *offset, out);
        }
        E::Concat { parts } => {
            for &p in parts {
                expr_nets(ir, p, out);
            }
        }
        E::Replicate { count, value } => {
            expr_nets(ir, *count, out);
            expr_nets(ir, *value, out);
        }
        E::Unary { op: _, operand } => expr_nets(ir, *operand, out),
        E::Binary { op: _, lhs, rhs } => {
            expr_nets(ir, *lhs, out);
            expr_nets(ir, *rhs, out);
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_nets(ir, *cond, out);
            expr_nets(ir, *then_e, out);
            expr_nets(ir, *else_e, out);
        }
        E::SysFunc { which: _, args } | E::Call { func: _, args } => {
            for &a in args {
                expr_nets(ir, a, out);
            }
        }
    }
}

/// Static combinational rank per process template, indexed by template id.
///
/// Public because the combinational DEPTH of a design (`ranks.iter().max()`) is the
/// number that predicts how a design behaves under an event-driven scheduler, and it
/// is not derivable from any other exported fact.
///
/// Rank 0 = no combinational predecessor (a source, an edge-triggered process, or a
/// node Kahn could not order because it sits in a combinational cycle). Rank `k` = the
/// longest combinational path reaching it.
pub fn comb_ranks(ir: &SimIr) -> Vec<u32> {
    let n = ir.processes.len();
    let mut rank = vec![0u32; n];
    if n == 0 {
        return rank;
    }

    let reads: Vec<BTreeSet<u32>> = (0..n).map(|p| level_reads(ir, p)).collect();
    let writes: Vec<BTreeSet<u32>> = (0..n).map(|p| blocking_writes(ir, p)).collect();
    // Continuous assigns must CARRY rank without ADDING a level: they settle to a
    // fixpoint at the top of every delta, so a value crossing one is already stable
    // when the next rank runs. Skipping them was measured to leave every rank at 0 on
    // the instance-chained shape (stage -> `assign y = r` -> port -> next stage), which
    // is precisely the shape whose depth cost this exists to remove.
    let cas: Vec<(BTreeSet<u32>, BTreeSet<u32>)> = ir
        .cont_assigns
        .iter()
        .map(|c| {
            let lhs: BTreeSet<u32> = c.lhs.chunks.iter().map(|k| k.net).collect();
            let mut rhs = BTreeSet::new();
            expr_nets(ir, c.rhs, &mut rhs);
            (lhs, rhs)
        })
        .collect();

    let mut net_rank: BTreeMap<u32, u32> = BTreeMap::new();
    let nr = |m: &BTreeMap<u32, u32>, s: &BTreeSet<u32>| -> u32 {
        s.iter()
            .map(|x| m.get(x).copied().unwrap_or(0))
            .max()
            .unwrap_or(0)
    };
    // Monotone relaxation. It converges in `combinational depth` rounds, which is small;
    // the cap only binds on a combinational CYCLE, where it stops the ranks growing
    // without bound. Either way the result is a deterministic function of the SimIr.
    let cap = n + cas.len() + 1;
    for _ in 0..cap {
        let mut changed = false;
        for p in 0..n {
            // A process with no level reads (edge-triggered, `initial`) is a rank-0
            // source: it runs when its own trigger says so, never "after" a producer.
            if reads[p].is_empty() {
                continue;
            }
            let r = nr(&net_rank, &reads[p]);
            if r > rank[p] {
                rank[p] = r;
                changed = true;
            }
        }
        for p in 0..n {
            for &w in &writes[p] {
                let e = net_rank.entry(w).or_insert(0);
                if *e < rank[p] + 1 {
                    *e = rank[p] + 1;
                    changed = true;
                }
            }
        }
        for (lhs, rhs) in &cas {
            let r = nr(&net_rank, rhs);
            for &l in lhs {
                let e = net_rank.entry(l).or_insert(0);
                if *e < r {
                    *e = r;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    rank
}
