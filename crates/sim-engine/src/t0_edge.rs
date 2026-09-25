//! The SETTLE-CONSTANT set: nets whose time-0 value is decided by literals alone.
//!
//! The time-0 settle writes every continuous driver before any process is
//! armed, and its record stays on the dirty list so a level waiter runs once
//! (`always @(w)` on `wire w = 1'b1;` prints `W 0 w=1` in both oracles). The
//! write funnel also folds each net's `declared default → settled value` hop
//! into its `slot_edge` mask, and the first delta's edge scan read `z → 1` as a
//! posedge on every settle net: `wire w = 1'b1; always @(posedge w)` printed
//! `P 0`, `always_ff @(posedge w) q <= ~q;` flipped `q` once, `initial
//! @(posedge w)` fell through at time 0 — where neither oracle prints a line.
//!
//! Clearing the mask on EVERY settle net was measured wrong the other way: both
//! oracles DO fire the edge when the driver reads a variable and bit 0 settles
//! definite — `reg r = 0; wire [1:0] w = {r, 1'b1}; always @(posedge w)` prints
//! `P 0 w=01` in both (and `P 0 w=x1` / `P 0 w=01` for an unwritten `r`), as do
//! `r | 2'b01`, `{cnt[7:1], 1'b1}` and `{r, y}` of a port copy `y`. What both
//! oracles are silent on is a driver with NO variable behind it: a literal, a
//! multi-bit literal, a multi-driver constant, a copy of a constant wire
//! (`wire c = w;`), a hierarchical port copy of one, `{1'b0, k}` of a constant
//! `k`, `logic d; assign d = 1'b1;`. So the rule is on the DRIVER's leaves, by
//! fixpoint: a net is settle-constant when every continuous driver of it is
//! undelayed, has no call and no impure system function in it, and reads
//! only settle-constant nets — which carries copies, port binds and concatenations of
//! constants without a copy-net special case. Only those nets lose the settle's
//! edge; every other settle net keeps the mask the funnel built.
//!
//! Between the two the oracles contradict each other and themselves: iverilog
//! fires on `wire n = ~w;` of a constant `w`, on `and g(w, 1'b1, 1'b1)` and on
//! `reg a = 0; wire w = a | 1'b1;` (a functor it did not fold) where verilator
//! folds all three silent; verilator fires on `reg r = 0; wire w = ~r;` and
//! `r !== 1'b1` where iverilog is silent; iverilog fires on `r | 1'b1` and not
//! on `reg [3:0] r = 4'd3; r | 4'd1`. Those stay as they were (ROADMAP §2
//! Oracle splits): a constant-only driver is silent here, anything reading a
//! variable keeps its edge.
//!
//! Computed once per run, in `arm_processes` / `arm_t0`, after the initializer
//! rollback and before the x-drop and the copy-net suppression.
//!
//! THE PHANTOM HOP. The settle runs before the declaration initializers, so a
//! driver that reads an initialised variable settles on the value of the
//! variable's DEFAULT first: `reg r = 1; wire w = (r !== 1'b1);` settled `w`
//! to 1 (`x !== 1`) and the first delta brought it to 0, and the funnel had
//! folded both hops into the mask — `always @(posedge w)` printed `P 0` and
//! `always @(negedge w)` `N 0`, `always_ff @(posedge w) c <= c + 1` counted
//! one, `initial @(posedge w)` fell through, a child's `always @(posedge i)`
//! on that port and a copy `wire c = w;` did the same — where both oracles
//! print only the level line `W 0 w=0` (also for `$isunknown(r)`, `int r =
//! 1; (r !== 1)`, `logic w; assign w = (r !== 1'b1);`, `(r === 1'b0)` of
//! `reg r = 0`, and `v = w ? 1'b0 : 1'b1` chained on it). IEEE 1800 §6.21
//! puts the initializer before any process starts, so both kernels
//! RE-SETTLE after the initializer bodies (the dirty-settle of exactly the
//! drivers those writes marked) and then ASSIGN each dirty edge-target net's
//! mask from the bit it held before the first settle (`edge_b0_snapshot`) to
//! the bit it holds after the re-settle. What that pair yields is the
//! funnel's own rule (`edge_mask`), unchanged: `z → 1` is a posedge, `z → 0`
//! a negedge. The level wake is unchanged too — membership is a union, and a
//! `bit`-typed net driven back to its default still wakes `always @(w)` once
//! (both oracles: `W 0 w=0`, count 1).
//!
//! THE DELIVERY. With the phantom gone the run loop's first settle finds nothing
//! marked, so the settle's record would be delivered by the propagate AFTER the
//! first Active batch, sorted with the batch-write wakes by declaration order,
//! and an in-body wait armed in the batch would see it. Both kernels instead
//! deliver it at the start of the run (`Scheduler::take_t0_wakes`,
//! `native::run::take_t0_wakes`): the arming's dirty list is propagated once
//! before the first batch and the woken processes are held, then queued after
//! the batch and its writes have propagated, ahead of the batch-write wakes —
//! the `initial` bodies, the settle's wakes, the batch-write wakes, which is
//! what both oracles print (`always @(w) x = 5;` runs before an `always @(s)`
//! woken by `initial s = 1;`, which reads `x=5`; `initial begin @(negedge w);
//! … end` armed at time 0 waits). A process the settle and the batch both wake
//! runs once, with the value the batch left; an `always_comb` reading a settled
//! net runs once at time 0 (verilator once, iverilog twice).
//!
//! Left where it was, as one oracle split (ROADMAP §2): whether the settle of
//! a variable-reading driver is an edge AT ALL. On `z → 0` iverilog fires
//! `N 0 w=10` for `{r, 1'b0}` and nothing for `(r !== 1'b1)`, `~r`, `r + 1`
//! (functor-decided) and verilator holds no z; on `z → 1` iverilog fires for
//! a concat, an xor and `~` of a NET and not for `~` of a variable, `r + 1`
//! or a case-equality, verilator fires for `wire w = ~r;` beside an `always
//! @(w)` and not for the same `~clk` beside an `always #5` toggler. vita
//! keeps the value rule for both.

use sim_ir::SimIr;
use std::collections::BTreeSet;

/// Bit 0 of every edge-target net as it stands NOW, dense by net id (`Z`
/// where the net is not an edge target). Taken before the time-0 settle by
/// both kernels (`Scheduler::settle_t0`, `native::run`), so the rebuild in
/// `arm_processes` / `arm_t0` has the bit the net held before any driver
/// wrote it — the declared default, whatever the net kind — rather than a
/// re-derivation of it.
pub(crate) fn edge_b0_snapshot(
    is_edge_target: &[bool],
    b0: impl Fn(u32) -> sim_ir::FourState,
) -> Vec<sim_ir::FourState> {
    is_edge_target
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            if t {
                b0(i as u32)
            } else {
                sim_ir::FourState::Z
            }
        })
        .collect()
}

/// Collect the nets `eid` reads into `out`; `false` when the expression holds a
/// node whose value is not a function of literals and nets alone (a call, a
/// system function, an array-method item). Exhaustive on purpose: a new `Expr`
/// variant is a compile error here, not a silent "constant".
fn closed_reads(ir: &SimIr, eid: u32, out: &mut BTreeSet<u32>) -> bool {
    use sim_ir::Expr as E;
    let Some(e) = ir.exprs.get(eid as usize) else {
        return false;
    };
    match e {
        E::Signal { net, word } => {
            out.insert(*net);
            word.is_none_or(|w| closed_reads(ir, w, out))
        }
        E::Const { .. } => true,
        // A system function is admitted only from the PURE list below — one
        // whose value is a function of its arguments alone — with closed
        // arguments. `$time`, `$random`, the heap and string queries, plusargs
        // and file handles depend on the run, not on literals.
        E::SysFunc { which, args } => {
            pure_sysfunc(*which) && args.iter().all(|&a| closed_reads(ir, a, out))
        }
        E::ArrayItem { .. } | E::Call { .. } => false,
        E::Select {
            base,
            offset,
            width,
            kind: _,
        } => {
            closed_reads(ir, *base, out)
                && closed_reads(ir, *offset, out)
                && closed_reads(ir, *width, out)
        }
        E::Concat { parts } => parts.iter().all(|&p| closed_reads(ir, p, out)),
        E::Replicate { count, value } => {
            closed_reads(ir, *count, out) && closed_reads(ir, *value, out)
        }
        E::Unary { op: _, operand } => closed_reads(ir, *operand, out),
        E::Binary { op: _, lhs, rhs } => closed_reads(ir, *lhs, out) && closed_reads(ir, *rhs, out),
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            closed_reads(ir, *cond, out)
                && closed_reads(ir, *then_e, out)
                && closed_reads(ir, *else_e, out)
        }
    }
}

/// The system functions whose value is a pure function of their arguments —
/// the casts and conversions (`$signed`, `$unsigned`, `$clog2`, `$rtoi`,
/// `$itor`, `$realtobits`, `$bitstoreal`) and the bit queries (`$countones`,
/// `$onehot`, `$onehot0`, `$isunknown`). Measured: `wire w = $clog2(2);`,
/// `$signed(1'b1)`, `$countones(2'b01)`, `1'(2'b01)`, `4'(1)` are silent in
/// both oracles at time 0. A positive list, so a new `SysFuncId` reads as
/// "not constant" until it is measured.
fn pure_sysfunc(which: sim_ir::SysFuncId) -> bool {
    use sim_ir::SysFuncId as F;
    matches!(
        which,
        F::Signed
            | F::Unsigned
            | F::Clog2
            | F::Rtoi
            | F::Itor
            | F::RealToBits
            | F::BitsToReal
            | F::CountOnes
            | F::OneHot
            | F::OneHot0
            | F::IsUnknown
    )
}

/// Per net: is its time-0 settle value decided by literals alone? (See the
/// module doc.) Indexed by net id; a net with no continuous driver, a driver
/// with a delay, a call or a system function, a driver reading a variable or an
/// undriven net, and every member of a driver cycle answer `false`.
pub(crate) fn settle_constant_nets(ir: &SimIr) -> Vec<bool> {
    let nnets = ir.nets.len();
    let ncas = ir.cont_assigns.len();
    let mut is_const = vec![false; nnets];
    if ncas == 0 {
        return is_const;
    }
    // Per driver: how many of its read nets are not yet known constant
    // (`usize::MAX` = never: delayed, impure, or reading past the net table).
    // Per net: how many of its drivers are not yet known constant, and which
    // drivers read it.
    let mut pending_reads: Vec<usize> = vec![usize::MAX; ncas];
    let mut pending_drivers: Vec<usize> = vec![0; nnets];
    let mut readers: Vec<Vec<usize>> = vec![Vec::new(); nnets];
    for (ci, ca) in ir.cont_assigns.iter().enumerate() {
        for c in &ca.lhs.chunks {
            if let Some(p) = pending_drivers.get_mut(c.net as usize) {
                *p += 1;
            }
        }
        if ca.delay.is_some() {
            continue;
        }
        let mut r = BTreeSet::new();
        let closed = closed_reads(ir, ca.rhs, &mut r)
            && ca.lhs.chunks.iter().all(|c| {
                [c.word, c.offset, c.width]
                    .into_iter()
                    .flatten()
                    .all(|e| closed_reads(ir, e, &mut r))
            })
            && r.iter().all(|&n| (n as usize) < nnets)
            && ca.lhs.chunks.iter().all(|c| (c.net as usize) < nnets);
        if !closed {
            continue;
        }
        pending_reads[ci] = r.len();
        for &n in &r {
            readers[n as usize].push(ci);
        }
    }
    let mut ca_const = vec![false; ncas];
    let mut queue: Vec<usize> = (0..ncas).filter(|&ci| pending_reads[ci] == 0).collect();
    while let Some(ci) = queue.pop() {
        if ca_const[ci] {
            continue;
        }
        ca_const[ci] = true;
        for c in &ir.cont_assigns[ci].lhs.chunks {
            let n = c.net as usize;
            pending_drivers[n] -= 1;
            if pending_drivers[n] == 0 && !is_const[n] {
                is_const[n] = true;
                for &rc in &readers[n] {
                    if pending_reads[rc] != usize::MAX {
                        pending_reads[rc] -= 1;
                        if pending_reads[rc] == 0 {
                            queue.push(rc);
                        }
                    }
                }
            }
        }
    }
    is_const
}
