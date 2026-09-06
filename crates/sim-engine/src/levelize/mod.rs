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

mod call_deps;
use call_deps::expr_call_reads;
pub(crate) use call_deps::func_read_deps;

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

/// Per-expression READ-THROUGH table (ROADMAP §2 row 33): for a `Signal` read of a
/// whole-net copy `c = v` inside a process that BLOCKING-writes `v`, the net to read
/// instead (`v`); `u32::MAX` everywhere else. Indexed by ExprId.
///
/// `wire [7:0] c; assign c = v;` then `v = 8'hA5; cap = c;` in one process latched
/// `cap = 00` where both oracles latch `a5`: the copy is driven by the settle, which
/// runs between process batches, so the process's own read one statement later saw
/// the value from before its own write. iverilog COLLAPSES such a copy into its
/// source (its VCD gives both one identifier); this table gives the same answer at
/// the one place it is defined — a read that follows the writer's own write in
/// program order. A read in ANOTHER process in the same delta is a §5.4.1 race
/// whose outcome depends on process order in every tool, and it keeps the value it
/// had (the settle's), so nothing order-dependent moves.
///
/// ⚠️ A store-side forward (update `c` inside the write of `v`) was built and
/// measured first: it moved picorv32's oracle-pinned digest, made a UDP chain sample
/// its fresh input in the same delta (iverilog: the old one) and split native from
/// the VM on keccak. The settle's consumers are order-sensitive; procedural reads
/// after one's own write are not.
pub(crate) fn proc_read_alias(
    ir: &SimIr,
    alias: &[u32],
    alias_word: &[u32],
) -> (Vec<u32>, Vec<u32>) {
    // (net table, word table), both by ExprId.
    let mut table = (
        vec![u32::MAX; ir.exprs.len()],
        vec![u32::MAX; ir.exprs.len()],
    );
    if alias.iter().enumerate().all(|(i, &a)| a == i as u32) {
        return table;
    }
    for tmpl in 0..ir.processes.len() {
        let writes = blocking_writes(ir, tmpl);
        if writes.is_empty() {
            continue;
        }
        let mark = |eid: u32, table: &mut (Vec<u32>, Vec<u32>)| {
            expr_signals(ir, eid, &mut |sig, net| {
                let root = alias[net as usize];
                if root != net && writes.contains(&root) {
                    table.0[sig as usize] = root;
                    table.1[sig as usize] = alias_word[net as usize];
                }
            });
        };
        let lv_exprs = |lv: &sim_ir::Lvalue, f: &mut dyn FnMut(u32)| {
            for c in &lv.chunks {
                for e in [c.word, c.offset, c.width].into_iter().flatten() {
                    f(e);
                }
            }
        };
        for block in &ir.processes[tmpl].body {
            for &sid in &block.stmts {
                match &ir.stmts[sid as usize] {
                    Stmt::BlockingAssign { lhs, rhs } => {
                        mark(*rhs, &mut table);
                        lv_exprs(lhs, &mut |e| mark(e, &mut table));
                    }
                    Stmt::NonblockingAssign { lhs, rhs, delay } => {
                        mark(*rhs, &mut table);
                        if let Some(d) = delay {
                            mark(*d, &mut table);
                        }
                        lv_exprs(lhs, &mut |e| mark(e, &mut table));
                    }
                    Stmt::SysTask {
                        which: _,
                        fmt,
                        args,
                    } => {
                        if let Some(f) = fmt {
                            mark(*f, &mut table);
                        }
                        for &a in args {
                            mark(a, &mut table);
                        }
                    }
                    Stmt::Force { lhs, rhs } => {
                        mark(*rhs, &mut table);
                        lv_exprs(lhs, &mut |e| mark(e, &mut table));
                    }
                    Stmt::Release { lhs } => lv_exprs(lhs, &mut |e| mark(e, &mut table)),
                    Stmt::Disable { .. } => {}
                }
            }
            match &block.term {
                sim_ir::Terminator::Branch { cond, .. } => mark(*cond, &mut table),
                sim_ir::Terminator::Delay { amount, .. } => mark(*amount, &mut table),
                sim_ir::Terminator::Wait { cond, .. } => {
                    if let sim_ir::WaitCause::Expr { expr } = cond {
                        mark(*expr, &mut table);
                    }
                }
                sim_ir::Terminator::Goto { .. }
                | sim_ir::Terminator::Fork { .. }
                | sim_ir::Terminator::Call { .. }
                | sim_ir::Terminator::Return => {}
            }
        }
    }
    table
}

/// Every `Signal` node under `eid`, as `(that node's ExprId, its net)`. Same
/// exhaustive walk as [`expr_nets`], one level richer.
fn expr_signals(ir: &SimIr, eid: u32, f: &mut dyn FnMut(u32, u32)) {
    use sim_ir::Expr as E;
    match &ir.exprs[eid as usize] {
        E::Signal { net, word } => {
            f(eid, *net);
            if let Some(w) = word {
                expr_signals(ir, *w, f);
            }
        }
        E::Const { .. } | E::ArrayItem { .. } => {}
        E::Select {
            base,
            offset,
            width: _,
            kind: _,
        } => {
            expr_signals(ir, *base, f);
            expr_signals(ir, *offset, f);
        }
        E::Concat { parts } => {
            for &p in parts {
                expr_signals(ir, p, f);
            }
        }
        E::Replicate { count, value } => {
            expr_signals(ir, *count, f);
            expr_signals(ir, *value, f);
        }
        E::Unary { op: _, operand } => expr_signals(ir, *operand, f),
        E::Binary { op: _, lhs, rhs } => {
            expr_signals(ir, *lhs, f);
            expr_signals(ir, *rhs, f);
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_signals(ir, *cond, f);
            expr_signals(ir, *then_e, f);
            expr_signals(ir, *else_e, f);
        }
        E::SysFunc { which: _, args } | E::Call { func: _, args } => {
            for &a in args {
                expr_signals(ir, a, f);
            }
        }
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
        E::Signal { net, word } => {
            out.insert(*net);
            // `word` IS an ExprId, not a constant index — `eval_core` evaluates it
            // (`assoc_key(*weid)` / the u32 word funnel). Missing it only costs a rank
            // here, but the same walk feeds the dirty settle, where a dropped index net
            // means `assign y = mem[idx]` never re-evaluates when `idx` moves.
            if let Some(w) = word {
                expr_nets(ir, *w, out);
            }
        }
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

    let writes: Vec<BTreeSet<u32>> = (0..n).map(|p| blocking_writes(ir, p)).collect();
    // A process that reads a net IT ITSELF blocking-writes is reading its own
    // intermediate value inside one activation — `always_comb begin y = a; z = y+1; end`
    // — not waiting on another producer. Counting that as a dependency makes the
    // relaxation climb forever: `rank[p] >= net_rank[n]` and `net_rank[n] >= rank[p]+1`
    // cannot both hold. The climb is bounded only by the iteration cap, so the result
    // looks like a combinational CYCLE that is not there.
    //
    // Measured on PicoRV32: 4 of its 43 processes do this over 6 nets, and that alone
    // made `comb_depth` report "cyclic" for a CPU that has no combinational loop.
    let reads: Vec<BTreeSet<u32>> = (0..n)
        .map(|p| level_reads(ir, p).difference(&writes[p]).copied().collect())
        .collect();
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

/// The design's combinational DEPTH, or `None` when the dependency graph did not
/// converge — i.e. it contains a combinational cycle (or a bit-level-acyclic loop that
/// looks cyclic at net granularity, which is the common case in real RTL).
///
/// `comb_ranks` relaxes under a hard iteration cap so it always terminates. On a cyclic
/// graph the ranks grow until that cap and the largest one is a SATURATION ARTIFACT, not
/// a depth — PicoRV32 reports 195 from 43 processes, which is simply the cap. Reading
/// that number as a depth is how a measurement lies, so callers that want the depth must
/// go through here and handle the `None`.
pub fn comb_depth(ir: &SimIr) -> Option<u32> {
    let cap = (ir.processes.len() + ir.cont_assigns.len() + 1) as u32;
    let max = comb_ranks(ir).iter().copied().max().unwrap_or(0);
    // A converged relaxation cannot produce a rank at or above the number of rounds it
    // was allowed; reaching it means the fixpoint was still moving when the cap hit.
    if max + 1 >= cap {
        None
    } else {
        Some(max)
    }
}

/// Nets an `Lvalue` DEPENDS ON (its dynamic word/offset/width index expressions) —
/// not the nets it writes. `resolve_lvalue_offsets` evaluates these at settle time, so
/// a change to one of them changes where the assign lands.
fn lvalue_index_nets(ir: &SimIr, lv: &sim_ir::Lvalue, out: &mut BTreeSet<u32>) {
    for c in &lv.chunks {
        for e in [c.word, c.offset, c.width].into_iter().flatten() {
            expr_nets(ir, e, out);
        }
    }
}

/// Is every node of the expression rooted at `eid` a PURE function of the nets it
/// reads? A positive allow-list, never a deny-list: an unrecognised node must read as
/// "not pure" so a future `Expr` variant cannot quietly opt itself into being skipped.
///
/// `SysFunc` is excluded because `$random`/`$time` do not depend on their inputs alone,
/// and `ArrayItem` because it is an array-method iterator value with no net of its own.
///
/// `Call` is answered by `fdeps` — [`func_read_deps`]'s verdict for the callee, which
/// is `Some` exactly when the callee's reads have all been collected into the caller's
/// dependency set. Before that analysis existed this arm was an unconditional `false`,
/// which is why `assign w = f(CONST);` — eighty of them in `verilog-ethernet`'s
/// `lfsr.v`, one per generated mask bit — was re-evaluated on every settle pass for the
/// whole run instead of once.
fn expr_is_pure_of_nets(ir: &SimIr, eid: u32, fdeps: &[Option<BTreeSet<u32>>]) -> bool {
    use sim_ir::Expr as E;
    let expr_is_pure_of_nets = |ir: &SimIr, e: u32| expr_is_pure_of_nets(ir, e, fdeps);
    match &ir.exprs[eid as usize] {
        E::Const { .. } => true,
        E::Signal { word, .. } => word.is_none_or(|w| expr_is_pure_of_nets(ir, w)),
        E::Select { base, offset, .. } => {
            expr_is_pure_of_nets(ir, *base) && expr_is_pure_of_nets(ir, *offset)
        }
        E::Concat { parts } => parts.iter().all(|&p| expr_is_pure_of_nets(ir, p)),
        E::Replicate { count, value } => {
            expr_is_pure_of_nets(ir, *count) && expr_is_pure_of_nets(ir, *value)
        }
        E::Unary { operand, .. } => expr_is_pure_of_nets(ir, *operand),
        E::Binary { lhs, rhs, .. } => {
            expr_is_pure_of_nets(ir, *lhs) && expr_is_pure_of_nets(ir, *rhs)
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_is_pure_of_nets(ir, *cond)
                && expr_is_pure_of_nets(ir, *then_e)
                && expr_is_pure_of_nets(ir, *else_e)
        }
        E::Call { func, args } => {
            fdeps.get(*func as usize).is_some_and(Option::is_some)
                && args.iter().all(|&a| expr_is_pure_of_nets(ir, a))
        }
        E::SysFunc { .. } | E::ArrayItem { .. } => false,
    }
}

/// Per continuous assign: the nets its value depends on, and whether it may be skipped
/// when none of them changed.
///
/// **Why skipping is sound when it is.** An assign whose dependency nets are unchanged
/// recomputes the same value, and the write funnel drops a same-value write without
/// noting a change — so evaluating it is observationally a no-op. The whole risk is
/// therefore in the DEPENDENCY SET being complete and the value being a pure function
/// of it, which is what `dirty_ok` certifies.
///
/// `dirty_ok` is false for an impure RHS, and for any dependency that is a heap handle
/// (dynamic array / queue / associative array / class), whose CONTENTS can change while
/// the handle net itself does not — a change no `note_change` would report.
///
/// `windows` is the per-`FuncId` frame layout (`FuncMeta`'s `base_net`/`locals_len`),
/// which [`func_read_deps`] needs to tell a callee's own locals from the module nets it
/// reads. A caller with no sidecar passes an empty slice, and every call declines.
pub(crate) fn ca_deps(
    ir: &SimIr,
    windows: &[(u32, u32)],
    is_heap: &dyn Fn(u32) -> bool,
) -> Vec<(BTreeSet<u32>, bool)> {
    let (fdeps, fsafe) = func_read_deps(ir, windows, is_heap);
    // ⚠️⚠️ THE NETS A `force`/`release` CAN RE-DIRTY, and the reason the empty-dependency
    // arm below is not enough on its own.
    //
    // That arm rests on "no dependencies ⇒ evaluated once, at the settle seed". Round-2
    // adversarial review measured it FALSE: `k_release` calls `redirty_drivers_of` on its
    // target UNCONDITIONALLY — deliberately, so a released wire snaps back in the same
    // settle instead of at the next input change — so a certified assign is evaluated
    // `1 + (number of releases)` times. Both oracles evaluate it once whatever the release
    // count, and a `release` with no matching `force` is a no-op in both. Measured on a
    // counter callee: PRE `3 3 3` (verilator's answer exactly) → certified POST `1 2 3`
    // (nobody's), and the non-saturating twin went from a loud `F4016 did not converge` to
    // a silent answer — the exact ladder drop the count-safety gate exists to prevent,
    // arriving through the OTHER arm of the disjunct.
    //
    // ⭐ The census that settles it: `ca_dirty_flag[..] = true` has exactly three
    // producers — the seed in `Scheduler::new`, `note_change` (a dependency really moved,
    // which is the rule this certification implements), and `redirty_drivers_of`, whose
    // only callers are the two `k_release` twins. So naming the force/release targets
    // names every evaluation that is not caused by a dependency change.
    let redirty_targets: BTreeSet<u32> = ir
        .stmts
        .iter()
        .filter_map(|st| match st {
            Stmt::Release { lhs } | Stmt::Force { lhs, .. } => Some(lhs),
            _ => None,
        })
        .flat_map(|lv| lv.chunks.iter().map(|c| c.net))
        .collect();
    ir.cont_assigns
        .iter()
        .map(|c| {
            let mut deps = BTreeSet::new();
            expr_nets(ir, c.rhs, &mut deps);
            // …and, for a certified call, the nets its BODY reads: `expr_nets` sees the
            // arguments only, so without this an `assign y = f();` reading a module net
            // inside `f` would be certified with an empty set and freeze.
            let mut count_safe = true;
            expr_call_reads(ir, c.rhs, &fdeps, &fsafe, &mut deps, &mut count_safe);
            lvalue_index_nets(ir, &c.lhs, &mut deps);
            for e in c
                .lhs
                .chunks
                .iter()
                .flat_map(|k| [k.word, k.offset, k.width])
                .flatten()
            {
                expr_call_reads(ir, e, &fdeps, &fsafe, &mut deps, &mut count_safe);
            }
            let dirty_ok = c.delay.is_none()
                && expr_is_pure_of_nets(ir, c.rhs, &fdeps)
                && c.lhs.chunks.iter().all(|k| {
                    [k.word, k.offset, k.width]
                        .into_iter()
                        .flatten()
                        .all(|e| expr_is_pure_of_nets(ir, e, &fdeps))
                })
                && !deps.iter().copied().any(is_heap)
                // ⭐ THE DISJUNCT. A callee whose value can depend on how many times it
                // has run is safe in exactly two situations, and one evaluation is one
                // of them: with an EMPTY dependency set and no way to be re-dirtied, the
                // assign is evaluated once (at the settle seed) and never again, which is
                // what both oracles do with an empty sensitivity list. See
                // `own_reads_are_definitely_assigned` for the other, and `redirty_targets`
                // above for why "empty" alone was not the right question.
                && (count_safe
                    || (deps.is_empty()
                        && !c.lhs.chunks.iter().any(|k| redirty_targets.contains(&k.net))));
            (deps, dirty_ok)
        })
        .collect()
}

/// Nets a process writes NONBLOCKING (they land in the NBA region, a delta boundary).
fn nba_writes(ir: &SimIr, tmpl: usize) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for block in &ir.processes[tmpl].body {
        for &sid in &block.stmts {
            if let Stmt::NonblockingAssign { lhs, .. } = &ir.stmts[sid as usize] {
                for c in &lhs.chunks {
                    out.insert(c.net);
                }
            }
        }
    }
    out
}

/// Nets a process is EDGE-sensitive to (`@(posedge n)`), which is a wake nobody may
/// reorder around.
fn edge_reads(ir: &SimIr, tmpl: usize) -> BTreeSet<u32> {
    let s = &ir.processes[tmpl].sensitivity;
    match s.kind {
        SensKind::Edge => s.edges.iter().map(|e| e.net).collect(),
        SensKind::Comb | SensKind::Latch | SensKind::Level | SensKind::Initial => BTreeSet::new(),
    }
}

/// A fusable pair: running `producer`'s body immediately before `consumer`'s, in ONE
/// activation, is observationally identical to running them in separate deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FusionPair {
    pub producer: usize,
    pub consumer: usize,
    /// The net that connects them — written by `producer`, read by `consumer` alone.
    pub net: u32,
}

/// Which combinational process pairs may be fused without moving any process order.
///
/// Fusion is what raises work per activation, and work per activation is what decides
/// whether a compiled backend pays at all (the C-GAIN crossover: 0.99x at one statement
/// per activation, 0.75x at 64). The measured payoff of doing it is large — 3.6x on the
/// interpreter and 1.19x -> 2.44x on the VM at depth 48 — so the only question is when
/// it is SAFE.
///
/// It is safe exactly when nothing can observe the delta between the two bodies. The
/// connecting net `n` must therefore have `consumer` as its ONLY consumer of any kind
/// and `producer` as its only producer:
///
/// - no other process level-reads `n` — it would have run between them
/// - no process edge-reads `n` — `@(posedge n)` is a wake nobody may reorder around
/// - no continuous assign reads `n` (RHS or lvalue index) — the settle between the two
///   bodies would have seen the new value and could feed it back into `consumer`
/// - no continuous assign writes `n`, and no other process writes it, blocking or NBA
/// - `producer` writes NOTHING BUT `n`, since another write of its would carry its own
///   observers whose ordering would move
///
/// The VCD is unaffected either way: the write to `n` still goes through the same funnel
/// at the same simulation time, so its recorded value sequence is unchanged. That is why
/// fusing does not cost the G2 observability rails.
pub fn fusion_candidates(ir: &SimIr) -> Vec<FusionPair> {
    let n = ir.processes.len();
    let comb = |p: usize| matches!(ir.processes[p].sensitivity.kind, SensKind::Comb);
    let lreads: Vec<BTreeSet<u32>> = (0..n).map(|p| level_reads(ir, p)).collect();
    let ereads: Vec<BTreeSet<u32>> = (0..n).map(|p| edge_reads(ir, p)).collect();
    let bwrites: Vec<BTreeSet<u32>> = (0..n).map(|p| blocking_writes(ir, p)).collect();
    let nwrites: Vec<BTreeSet<u32>> = (0..n).map(|p| nba_writes(ir, p)).collect();

    // Everything the continuous assigns touch, on either side.
    let mut ca_reads: BTreeSet<u32> = BTreeSet::new();
    let mut ca_writes: BTreeSet<u32> = BTreeSet::new();
    for c in &ir.cont_assigns {
        expr_nets(ir, c.rhs, &mut ca_reads);
        lvalue_index_nets(ir, &c.lhs, &mut ca_reads);
        for k in &c.lhs.chunks {
            ca_writes.insert(k.net);
        }
    }

    let mut out = Vec::new();
    for p in 0..n {
        if !comb(p) || bwrites[p].len() != 1 {
            continue;
        }
        let net = *bwrites[p].iter().next().expect("len checked");
        if ca_reads.contains(&net) || ca_writes.contains(&net) {
            continue;
        }
        // Exactly one level reader, no edge reader, no other writer of any kind.
        let mut consumer = None;
        let mut ok = true;
        for q in 0..n {
            if ereads[q].contains(&net) || nwrites[q].contains(&net) {
                ok = false;
                break;
            }
            if q != p && bwrites[q].contains(&net) {
                ok = false;
                break;
            }
            if lreads[q].contains(&net) {
                if consumer.is_some() || q == p {
                    ok = false;
                    break;
                }
                consumer = Some(q);
            }
        }
        let Some(q) = consumer.filter(|_| ok) else {
            continue;
        };
        if comb(q) {
            out.push(FusionPair {
                producer: p,
                consumer: q,
                net,
            });
        }
    }
    out
}

/// A continuous assign that is a WHOLE-NET COPY (`assign y = r`) — the shape a module
/// port connection lowers to, and the reason `fusion_candidates` fires on nothing real:
/// a chain of instances is `always_comb r` → `assign y = r` → next stage, so the
/// producer's net is always read by a continuous assign.
///
/// Returns `(rhs_net, lhs_net)` for each such copy.
fn whole_net_copies(ir: &SimIr) -> Vec<(u32, u32, usize)> {
    ir.cont_assigns
        .iter()
        .enumerate()
        .filter_map(|(ci, c)| {
            if c.delay.is_some() || c.lhs.chunks.len() != 1 {
                return None;
            }
            let k = &c.lhs.chunks[0];
            if k.word.is_some() || k.offset.is_some() || k.width.is_some() {
                return None;
            }
            match &ir.exprs[c.rhs as usize] {
                sim_ir::Expr::Signal { net, word: None } => Some((*net, k.net, ci)),
                _ => None,
            }
        })
        .collect()
}

/// How many fusable pairs appear if a chain is allowed to cross a whole-net copy
/// continuous assign (`assign y = r`) — i.e. what an instance-chained design, the shape
/// real RTL actually has, would offer.
///
/// This is a MEASUREMENT of the opportunity, not a transform: crossing a copy means the
/// fused body must also perform that copy and the settle must stop doing so, which is
/// strictly more machinery than fusing two adjacent processes. Reported so the decision
/// to build that machinery rests on the count, not on the assumption that real designs
/// look like the synthetic chain.
pub fn fusion_candidates_across_copies(ir: &SimIr) -> usize {
    let n = ir.processes.len();
    let comb = |p: usize| matches!(ir.processes[p].sensitivity.kind, SensKind::Comb);
    let lreads: Vec<BTreeSet<u32>> = (0..n).map(|p| level_reads(ir, p)).collect();
    let ereads: Vec<BTreeSet<u32>> = (0..n).map(|p| edge_reads(ir, p)).collect();
    let bwrites: Vec<BTreeSet<u32>> = (0..n).map(|p| blocking_writes(ir, p)).collect();
    let copies = whole_net_copies(ir);

    let copy_cis: BTreeSet<usize> = copies.iter().map(|&(_, _, ci)| ci).collect();
    let mut other_ca: BTreeSet<u32> = BTreeSet::new();
    for (ci, c) in ir.cont_assigns.iter().enumerate() {
        if copy_cis.contains(&ci) {
            continue;
        }
        expr_nets(ir, c.rhs, &mut other_ca);
        lvalue_index_nets(ir, &c.lhs, &mut other_ca);
        for k in &c.lhs.chunks {
            other_ca.insert(k.net);
        }
    }

    let mut count = 0usize;
    for (p, bw) in bwrites.iter().enumerate() {
        if !comb(p) || bw.len() != 1 {
            continue;
        }
        // A port connection lowers to a CHAIN of whole-net copies (`r` -> module output
        // `y` -> parent wire `w` -> child input `a`), not a single one, so the walk has
        // to follow the chain to its end before asking who reads it. Crossing only one
        // copy found nothing on exactly the shape this exists for.
        let mut cur = *bw.iter().next().expect("len checked");
        let mut hops = 0usize;
        let end = loop {
            if other_ca.contains(&cur) || ereads.iter().any(|s| s.contains(&cur)) {
                break None;
            }
            let outs: Vec<&(u32, u32, usize)> =
                copies.iter().filter(|(rn, _, _)| *rn == cur).collect();
            let read_here = (0..n).any(|q| lreads[q].contains(&cur));
            match (outs.as_slice(), read_here) {
                // Read by a process here and by nothing else: this is the chain end.
                ([], true) => break Some(cur),
                // Feeds exactly one copy and no process: keep walking.
                ([one], false) => {
                    cur = one.1;
                    hops += 1;
                    if hops > copies.len() + 1 {
                        break None; // a copy cycle; refuse rather than spin
                    }
                }
                _ => break None,
            }
        };
        let Some(y) = end else { continue };
        let readers: Vec<usize> = (0..n).filter(|&q| lreads[q].contains(&y)).collect();
        if let [q] = readers.as_slice() {
            if comb(*q) && *q != p {
                count += 1;
            }
        }
    }
    count
}

/// DIAGNOSTIC: processes that both level-READ and blocking-WRITE the same net, plus the
/// nets involved. Used to tell a genuine combinational loop from an artifact of the
/// rank analysis.
pub fn self_read_write_processes(ir: &SimIr) -> Vec<(usize, Vec<u32>)> {
    (0..ir.processes.len())
        .filter_map(|p| {
            let r = level_reads(ir, p);
            let w = blocking_writes(ir, p);
            let both: Vec<u32> = r.intersection(&w).copied().collect();
            if both.is_empty() {
                None
            } else {
                Some((p, both))
            }
        })
        .collect()
}
