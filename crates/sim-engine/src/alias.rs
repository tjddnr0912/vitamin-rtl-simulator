//! The RENAME set: continuous drivers that MOVE bits instead of computing them.
//!
//! `assign n = m;` between two whole nets of the same width is not a
//! computation — it is a second name for `m`. `n` has no state of its own, so
//! there is no instant at which it holds a value `m` never held, and in
//! particular no instant at which it is still `z`/`x` because a driver "has not
//! run yet". The same is true one slice at a time: `assign n[1] = a;
//! assign n[0] = b;` gives `n` no state either, and a bus assembled that way is
//! how every vector port in a fabric design is wired.
//!
//! vita builds `n` as an ordinary net and drives it from the t0 structural
//! settle, which runs BEFORE the declaration initializers (`arm_t0` /
//! `arm_processes_after_seed`). So the settle reads the source while it is still
//! at its declared default, and the run loop's first delta then moves `n` again
//! once the initializer has landed — **a transition the source never made**.
//! Measured against iverilog 13:
//!
//! ```text
//! module sub (input wire p, output wire o);
//!   reg r; always @* r = p; assign o = r;
//! endmodule
//! reg  pr = 1'b0;             sub u1 (.p(pr), .o(o1));   // ivl x  vita 0  ← the defect
//! wire pw; assign pw = 1'b0;  sub u2 (.p(pw), .o(o2));   // ivl 0  vita 0  ← must keep firing
//! ```
//!
//! The second row is why "suppress the port bind's dirt" is the WRONG fix: a
//! constant driver really does move its net off `z` at time zero, and both tools
//! wake the level waiter for it (`run.rs` carries the measurement — 49 of 270
//! generated cont-assign designs diverged when that dirt was dropped wholesale).
//! The separating property is not "is it a port bind" but **did the SOURCE
//! move**.
//!
//! So a copy net is repaired after static initialization, and its time-zero event
//! is then SUPPRESSED if no source of it moved. A driver that COMPUTES — an
//! operator, a concatenation, a replication, a width change, a runtime index — is
//! not a copy; its output has an initial state of its own and the settle's first
//! evaluation is a real transition out of it.
//!
//! ⚠️⚠️ **Suppression only, and TRANSITIVE.** Both halves were found by adversarial
//! review, one per round, and they pull in opposite directions.
//!
//! The first version wrote `dirty[n] := OR over its sources`, which reads as the
//! same rule and is not: in vita `n` and `m` are two nets with their OWN storage
//! defaults (a driven `wire` starts `z`, a `logic`/`reg` starts `x`), where
//! iverilog collapses them into one net with one default. So a source can move
//! while the destination provably never does, and the OR arm then invented an
//! event on a net that holds `x` for the whole run:
//!
//! ```text
//! wire [1:0] m; assign m = {1'b1, 1'bx};
//! module sub (input logic p); always @(p) $display("sub woke"); endmodule
//! sub u (.p(m[0]));            // iverilog silent · the OR arm woke the child
//! ```
//!
//! Sixteen of fifty-six generated cells regressed on that. But plain suppression
//! — asking only the IMMEDIATE sources' dirt — is wrong in the other direction,
//! because a copy can stay put for a reason that has nothing to do with its
//! source: its own default already equals the copied value.
//!
//! ```text
//! wire [1:0] vv; assign vv = 2'b1z;   // vv MOVES, zz -> 1z
//! wire  s; assign s = vv[0];          // z default already matches: s never moves
//! logic d; assign d = s;              // x default does not: d moves x -> z
//! always @(d) …                       // iverilog fires · plain suppression did not
//! ```
//!
//! So `moved` is carried along the chain: a copy forwards its sources' movement
//! whether or not it moved itself, and only a net with NOTHING moving behind it is
//! suppressed. ⚠️ Its `x` twin (`2'b1x`, so `d` never moves either) is the control
//! that stops this from becoming the OR arm again — iverilog is silent there, and
//! so is vita. Both spellings are pinned in
//! `cli/tests/copy_net_no_t0_transition.rs`.
//!
//! The settle's own record still decides whether `n` moved; this pass only decides
//! whether it was allowed to.
//!
//! ⚠️ That boundary is where iverilog stops being self-consistent: it collapses
//! `assign w = pr & 1'b1;` (its elaborator folds the identity away) but not
//! `assign w = pr | 1'b0;`, two spellings of the same function of the same
//! variable, and it reports a t0 event for one and not the other. vita takes the
//! half that is a tautology rather than the half that is an elaborator artifact,
//! so those folded-identity spellings stay as they are — see ROADMAP §2.

use sim_ir::SimIr;
use std::collections::BTreeMap;

/// A net whose every continuous driver moves bits: `cas` are those drivers (in
/// declaration order) and `srcs` the distinct nets they read.
pub(crate) struct CopyNet {
    pub dst: u32,
    pub cas: Vec<usize>,
    pub srcs: Vec<u32>,
}

/// A `LvalChunk`/`Select` width or offset edge, folded. Both are ExprIds in the
/// frozen IR (`Select.width` is a const-expr edge such as `Add(Sub(msb,lsb),1)`),
/// so this is the same fold `eval_select` and `write_chunk` run.
fn const_of(ir: &SimIr, eid: u32) -> Option<u32> {
    crate::width::const_u32_of_expr(ir, eid)
}

/// Flat packed storage of one element — asked of the DESTINATION, and of a source
/// that is reached through a `Select` (a slice of a heap handle is not a slice).
/// Heap kinds are string / queue / dynamic and associative arrays / class handles
/// / events.
///
/// ⚠️ A whole-net source is NOT asked, deliberately: `assign w = <real net>;`
/// then rides the repair, and it should — the store's `coerce_assign` makes the
/// destination a pure function of the source with no state of its own, which is
/// the whole predicate. Measured: six real-source cells, POST matches iverilog on
/// all six and PRE matched two. Adding the check here would move four of them
/// AWAY from the oracle.
fn flat(ir: &SimIr, n: u32) -> bool {
    let nv = &ir.nets[n as usize];
    matches!(
        nv.kind,
        sim_ir::NetKind::Wire
            | sim_ir::NetKind::Reg
            | sim_ir::NetKind::Logic
            | sim_ir::NetKind::Integer
    ) && nv.array_len <= 1
}

/// The `(lsb, width)` a constant-offset slice of `net_w` bits covers, or `None`
/// if the offset is not a literal or the slice leaves the net.
///
/// In-range is required for two separate reasons. The stated one: a slice that
/// runs off the end FABRICATES `x` bits rather than moving them. The load-bearing
/// one: the repair pass re-evaluates each admitted driver, and an out-of-range
/// read re-emits its `E4002` on every visit — `native/dirty.rs` records the
/// measurement (picorv32 6 errors → 9). Refusing them keeps the diagnostic stream
/// byte-identical.
fn const_slice(
    ir: &SimIr,
    kind: sim_ir::SelKind,
    offset: Option<u32>,
    width: Option<u32>,
    net_w: u32,
) -> Option<(u32, u32)> {
    let Some(off) = offset else {
        // No offset edge ⇒ the whole net, whatever `kind` nominally says.
        return width.is_none().then_some((0, net_w));
    };
    let w = match width {
        Some(e) => const_of(ir, e)?,
        None => net_w,
    };
    let (lsb, w) = crate::eval::binops::select_lsb_width(kind, const_of(ir, off)? as i64, w);
    (lsb >= 0 && (lsb as u64) + (w as u64) <= net_w as u64).then_some((lsb as u32, w))
}

/// The source this driver's rhs copies, if it copies one: a whole-net read, a
/// constant, in-range slice of one, or a constant, in-range WORD of a flat
/// unpacked array (`assign c = m[1];`, §2 🆕 I ⓒ — both oracles read the word
/// through). `want` is the bit count the lvalue takes, so a widening or
/// truncating driver is refused — padding and truncation are computed, not moved.
/// The second half is the word's index expression when the source is an array
/// word (`None` for a whole net or a slice).
fn copied_source(ir: &SimIr, rhs: u32, want: u32) -> Option<(u32, Option<u32>)> {
    match ir.exprs.get(rhs as usize)? {
        sim_ir::Expr::Signal { net, word: None } => {
            (want == ir.nets[*net as usize].width).then_some((*net, None))
        }
        sim_ir::Expr::Signal {
            net,
            word: Some(weid),
        } => {
            // An array WORD: the element is flat packed storage like any scalar,
            // but the array net itself is not (`flat` refuses it as a destination
            // and as a slice base). A runtime index computes; an out-of-range
            // constant fabricates `x` (and re-emits its E4002 on every repair).
            let nv = &ir.nets[*net as usize];
            let flat_kind = matches!(
                nv.kind,
                sim_ir::NetKind::Wire
                    | sim_ir::NetKind::Reg
                    | sim_ir::NetKind::Logic
                    | sim_ir::NetKind::Integer
            );
            if !flat_kind || nv.array_len <= 1 || want != nv.width {
                return None;
            }
            let idx = const_of(ir, *weid)?;
            (idx < nv.array_len).then_some((*net, Some(*weid)))
        }
        sim_ir::Expr::Select {
            base,
            offset,
            width,
            kind,
        } => {
            let net = match ir.exprs.get(*base as usize)? {
                sim_ir::Expr::Signal { net, word: None } => *net,
                _ => return None,
            };
            if !flat(ir, net) {
                return None;
            }
            let (_, w) = const_slice(
                ir,
                *kind,
                Some(*offset),
                Some(*width),
                ir.nets[net as usize].width,
            )?;
            (w == want).then_some((net, None))
        }
        _ => None,
    }
}

/// A driver that contributes NOTHING to its net's resolution: no delay, the whole
/// net, and a constant that is `z` in every bit at the net's own width (`assign c
/// = 8'hzz;` beside `assign c = v;`, §2 🆕 I ⓑ). `z` is the identity of every
/// resolution kind (wire / wand / wor), so the net is a copy of its other driver
/// exactly as if this one were not written — both oracles read `v` through it.
/// A narrower constant is NOT one: `4'hz` on an 8-bit net zero-extends and the
/// high half drives 0 (iverilog `X5` against an `a5` source). A partial
/// `8'hzx` computes a conflict. Both stay computed, as they were.
fn null_driver(ir: &SimIr, ca: &sim_ir::ContAssign) -> bool {
    if ca.delay.is_some() || ca.lhs.chunks.len() != 1 {
        return false;
    }
    let c = &ca.lhs.chunks[0];
    if c.word.is_some() || c.offset.is_some() || c.width.is_some() {
        return false;
    }
    let Some(sim_ir::Expr::Const { val }) = ir.exprs.get(ca.rhs as usize) else {
        return false;
    };
    let Some(k) = ir.consts.get(*val as usize) else {
        return false;
    };
    let w = ir.nets[c.net as usize].width;
    if k.width != w || w == 0 || !matches!(k.repr, sim_ir::ConstRepr::Numeric) {
        return false;
    }
    (0..w as usize).all(|b| {
        let (wi, bi) = (b / 64, b % 64);
        let v = (k.bits.val.get(wi).copied().unwrap_or(0) >> bi) & 1;
        let u = (k.bits.unk.get(wi).copied().unwrap_or(0) >> bi) & 1;
        v == 1 && u == 1
    })
}

/// Every copy net in `ir`, in DEPENDENCY ORDER — a copy net whose source is
/// itself a copy net comes after it, so one forward pass both propagates values
/// and mirrors event status along a chain.
///
/// A driver is admitted only when it is a bit move, which needs all of:
///
/// * no delay — `assign #d n = m;` has its own inertial register, iverilog-pinned
///   to hold `x` over `[0, d)`;
/// * a single-chunk lvalue on this net with no array word, and either the whole
///   net or a CONSTANT-offset slice;
/// * an rhs that is a whole-net read or a constant-offset slice of one, of
///   exactly the width the lvalue takes;
/// * flat packed storage on both sides.
///
/// The net must have at least one driver, ALL of them must be admitted, and it
/// must not be a multi-driver group (≥2 whole-net drivers is a resolution, and
/// vita's `E3001` already refuses overlapping partial ones, so the surviving
/// multi-driver copy nets are disjoint slices of one bus). A whole-net all-`z`
/// constant driver ([`null_driver`]) is not counted on either side: it is neither
/// a move nor a computation, and a net with one move beside it is still a copy.
///
/// A cycle has no source to order from, so its own members are dropped. A copy
/// net READING a cycle member is kept — whatever value the settle's fixpoint left
/// there, the copy of it made no independent transition.
///
/// ⚠️ That last sentence is a property of the code, not a hope, and it was not
/// until review: the back-edge arm marked the WHOLE stack, so a reader was
/// dropped whenever its own walk happened to be in flight when the cycle was
/// found — which made coverage depend on net-id order. Two independent lenses
/// found it by swapping two `wire` declarations and watching the same cell go
/// from fixed to not fixed. Only the members from the back-edge target upwards
/// are the cycle.
pub(crate) fn copy_nets(ir: &SimIr) -> Vec<CopyNet> {
    if ir.cont_assigns.is_empty() {
        return Vec::new();
    }
    let nulls: std::collections::BTreeSet<usize> = ir
        .cont_assigns
        .iter()
        .enumerate()
        .filter(|(_, ca)| null_driver(ir, ca))
        .map(|(ci, _)| ci)
        .collect();
    // A multi-driver group whose members are one real driver plus null ones is
    // not a resolution of anything.
    let md: BTreeMap<u32, Vec<usize>> = crate::sched::multi_driver_groups(ir)
        .into_iter()
        .filter(|(_, cis)| cis.iter().filter(|ci| !nulls.contains(ci)).count() >= 2)
        .collect();
    // net → (its move drivers, the distinct nets they read, "every driver that
    // touches this net is a move"). A net touched by one driver that computes is
    // out whatever its other drivers look like.
    let mut by_dst: BTreeMap<u32, (Vec<usize>, Vec<u32>, bool)> = BTreeMap::new();
    for (ci, ca) in ir.cont_assigns.iter().enumerate() {
        if nulls.contains(&ci) {
            continue;
        }
        // `Some(src)` ⇒ this whole driver is one bit move from `src`. Only a
        // single-chunk lvalue can be, so the multi-chunk case falls to the loop
        // below and disqualifies every net it touches.
        let moved = (ca.delay.is_none() && ca.lhs.chunks.len() == 1)
            .then(|| {
                let c = &ca.lhs.chunks[0];
                if c.word.is_some() || !flat(ir, c.net) {
                    return None;
                }
                let (_, took) =
                    const_slice(ir, c.kind, c.offset, c.width, ir.nets[c.net as usize].width)?;
                copied_source(ir, ca.rhs, took)
                    .map(|(s, _)| s)
                    .filter(|&s| s != c.net)
            })
            .flatten();
        match moved {
            Some(src) => {
                let e =
                    by_dst
                        .entry(ca.lhs.chunks[0].net)
                        .or_insert((Vec::new(), Vec::new(), true));
                e.0.push(ci);
                if !e.1.contains(&src) {
                    e.1.push(src);
                }
            }
            None => {
                for c in &ca.lhs.chunks {
                    by_dst
                        .entry(c.net)
                        .or_insert((Vec::new(), Vec::new(), true))
                        .2 = false;
                }
            }
        }
    }
    by_dst.retain(|dst, e| e.2 && !e.0.is_empty() && !md.contains_key(dst));
    if by_dst.is_empty() {
        return Vec::new();
    }
    // DEPENDENCY ORDER, depth-first with an explicit stack. `state`: 1 = on the
    // stack, 2 = placed, 3 = placed but NOT emitted. A back-edge to `s` means the
    // stack from `s` UPWARDS is a cycle; a cycle has no source to order from, so
    // those are placed unemitted and simply keep the behaviour they had before
    // this pass existed. Everything BELOW `s` on the stack merely reads the cycle
    // and is emitted normally when its own walk finishes — marking the whole
    // stack instead made that depend on which net id the walk started from.
    let mut state: BTreeMap<u32, u8> = BTreeMap::new();
    let mut ordered: Vec<CopyNet> = Vec::new();
    let mut stack: Vec<(u32, usize)> = Vec::new();
    for root in by_dst.keys().copied().collect::<Vec<u32>>() {
        if state.contains_key(&root) {
            continue;
        }
        state.insert(root, 1);
        stack.push((root, 0));
        while let Some(top) = stack.len().checked_sub(1) {
            let (net, next) = stack[top];
            if next < by_dst[&net].1.len() {
                let s = by_dst[&net].1[next];
                stack[top].1 = next + 1;
                if by_dst.contains_key(&s) {
                    match state.get(&s).copied() {
                        None => {
                            state.insert(s, 1);
                            stack.push((s, 0));
                        }
                        Some(1) => {
                            // `rposition`, not `position`: with nested cycles the
                            // innermost occurrence of `s` is the one this edge
                            // closes.
                            let at = stack.iter().rposition(|e| e.0 == s).unwrap_or(0);
                            for e in stack[at..].iter() {
                                state.insert(e.0, 3);
                            }
                        }
                        _ => {}
                    }
                }
                continue;
            }
            if state.insert(net, 2) != Some(3) {
                let e = &by_dst[&net];
                ordered.push(CopyNet {
                    dst: net,
                    cas: e.0.clone(),
                    srcs: e.1.clone(),
                });
            }
            stack.pop();
        }
    }
    ordered
}

/// `net → the net it is a WHOLE-NET copy of` (its root source), identity elsewhere —
/// the runtime half of the rename set (ROADMAP §2 row 33) — and, beside it, `net →
/// the index expression of the array WORD it copies` (`u32::MAX` = a whole net),
/// for a copy of a constant word (`assign c = m[1];`, §2 🆕 I ⓒ). A chain through
/// such a copy carries the word down (`d = c` reads `m[1]` too).
///
/// Only the strictest members of [`copy_nets`] qualify: ONE driver, ONE source, both
/// sides the whole net, and an rhs that is a bare read (of a net or of a constant
/// word of an array). A chain resolves to its root (`copy_nets` is in dependency
/// order, so `alias[src]` is final when `dst` asks).
/// Excluded, deliberately: a 2-state destination (the settle's write coerces x/z
/// to 0 and a read-through would not), any net that is ever a `force` / `release`
/// target (a forced destination holds a value its source never held), and a
/// destination whose declared SIGN differs from its source's. The read substitutes
/// the NET, and a read carries the net's declared sign into its extension: `wire
/// signed [7:0] c; assign c = v;` with an unsigned `v` then `r = c;` (32-bit) is
/// 4294967295 in both oracles, and the source's unsigned flag gave 255 on the
/// interpreter and the VM while the native compiled path (which keeps the node's
/// own sign) gave the oracle's answer — a backend split, found by review. Width is
/// already required equal by `copied_source`; sign is the other property a read
/// takes from the net rather than from the bits.
pub(crate) fn copy_alias(ir: &SimIr, two_state: &[bool]) -> (Vec<u32>, Vec<u32>) {
    let mut alias: Vec<u32> = (0..ir.nets.len() as u32).collect();
    let mut alias_word: Vec<u32> = vec![u32::MAX; ir.nets.len()];
    let forced: std::collections::BTreeSet<u32> = ir
        .stmts
        .iter()
        .filter_map(|st| match st {
            sim_ir::Stmt::Force { lhs, .. } | sim_ir::Stmt::Release { lhs } => Some(lhs),
            _ => None,
        })
        .flat_map(|lv| lv.chunks.iter().map(|c| c.net))
        .collect();
    for cn in copy_nets(ir) {
        let [ci] = cn.cas.as_slice() else { continue };
        let [src] = cn.srcs.as_slice() else { continue };
        let ca = &ir.cont_assigns[*ci];
        let [c] = ca.lhs.chunks.as_slice() else {
            continue;
        };
        if c.word.is_some() || c.offset.is_some() || c.width.is_some() {
            continue;
        }
        let Some(sim_ir::Expr::Signal { word, .. }) = ir.exprs.get(ca.rhs as usize) else {
            continue;
        };
        if two_state.get(cn.dst as usize).copied().unwrap_or(false) || forced.contains(&cn.dst) {
            continue;
        }
        let root = alias[*src as usize];
        if ir.nets[cn.dst as usize].signed != ir.nets[root as usize].signed {
            continue;
        }
        alias[cn.dst as usize] = root;
        // An array word's index is validated by `copied_source` (constant, in
        // range); an array net is never itself a copy, so the chain's word is
        // either this driver's or the source's.
        alias_word[cn.dst as usize] = match word {
            Some(weid) => *weid,
            None => alias_word[*src as usize],
        };
    }
    (alias, alias_word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(kind: sim_ir::NetKind, width: u32) -> sim_ir::NetVar {
        sim_ir::NetVar {
            kind,
            width,
            msb: width - 1,
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: sim_ir::PortDir::Internal,
            init: sim_ir::BitPacked {
                val: vec![0],
                unk: vec![0],
            },
        }
    }

    fn whole(n: u32) -> sim_ir::Lvalue {
        sim_ir::Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net: n,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        }
    }

    fn empty_ir() -> SimIr {
        SimIr {
            instances: Vec::new(),
            nets: Vec::new(),
            processes: Vec::new(),
            cont_assigns: Vec::new(),
            funcs: Vec::new(),
            exprs: Vec::new(),
            stmts: Vec::new(),
            blocks: Vec::new(),
            consts: Vec::new(),
        }
    }

    /// nets `0..n` all 1-bit `Wire`; `exprs[i] = Signal(i)` for `i < n`.
    fn ir_with(n: u32, cas: Vec<(u32, u32, Option<u32>)>) -> SimIr {
        let mut ir = empty_ir();
        for _ in 0..n {
            ir.nets.push(net(sim_ir::NetKind::Wire, 1));
        }
        for i in 0..n {
            ir.exprs.push(sim_ir::Expr::Signal { net: i, word: None });
        }
        for (dst, src, delay) in cas {
            ir.cont_assigns.push(sim_ir::ContAssign {
                lhs: whole(dst),
                rhs: src,
                delay,
            });
        }
        ir
    }

    /// A literal `k` as an ExprId — `const_u32_of_expr` folds through the pool,
    /// so a bare `Expr::Const` with no `ConstVal` behind it folds to nothing.
    fn konst(ir: &mut SimIr, k: u64) -> u32 {
        let val = ir.consts.len() as u32;
        ir.consts.push(sim_ir::ConstVal {
            width: 32,
            signed: false,
            repr: sim_ir::ConstRepr::Numeric,
            bits: sim_ir::BitPacked {
                val: vec![k],
                unk: vec![0],
            },
        });
        let e = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Const { val });
        e
    }

    fn shape(r: &[CopyNet]) -> Vec<(u32, Vec<u32>)> {
        r.iter().map(|c| (c.dst, c.srcs.clone())).collect()
    }

    #[test]
    fn chain_is_ordered_source_first() {
        // assign 2 = 1;  assign 1 = 0;   ⇒ net 1 must be repaired before net 2.
        let ir = ir_with(3, vec![(2, 1, None), (1, 0, None)]);
        assert_eq!(shape(&copy_nets(&ir)), vec![(1, vec![0]), (2, vec![1])]);
    }

    /// ⭐ The reader's own net id must not decide whether it is emitted. Both
    /// spellings put a cycle `{1,2}` next to a reader of it; in the first the
    /// reader is walked FIRST (so the back edge is found with the reader on the
    /// stack), in the second it is walked last. Marking the whole stack on a back
    /// edge dropped the reader in the first spelling only — two review lenses
    /// found that by swapping two `wire` declarations in one design.
    #[test]
    fn a_reader_of_a_cycle_is_kept_whichever_end_the_walk_starts_from() {
        // reader is net 0, walked first: 0 -> 1 -> 2 -> 1 (back edge).
        let below = ir_with(3, vec![(0, 1, None), (1, 2, None), (2, 1, None)]);
        assert_eq!(shape(&copy_nets(&below)), vec![(0, vec![1])]);
        // reader is net 2, walked last: 0 -> 1 -> 0 (back edge), then 2.
        let above = ir_with(3, vec![(0, 1, None), (1, 0, None), (2, 0, None)]);
        assert_eq!(shape(&copy_nets(&above)), vec![(2, vec![0])]);
    }

    #[test]
    fn cycle_members_are_dropped_but_a_later_reader_is_kept() {
        // assign 0 = 1; assign 1 = 0; — neither has a source to order from, so
        // neither is emitted. `assign 2 = 0` still is: net 2 copies whatever the
        // settle left on net 0, and copies its event status with it.
        let ir = ir_with(3, vec![(0, 1, None), (1, 0, None), (2, 0, None)]);
        assert_eq!(shape(&copy_nets(&ir)), vec![(2, vec![0])]);
    }

    /// `assign c = m[IDX];` — a constant, in-range word of a flat array is a
    /// copy (§2 🆕 I ⓒ); the alias carries the word's index expression.
    fn ir_word_copy(idx: u64, array_len: u32, c_width: u32) -> (SimIr, u32) {
        let mut ir = empty_ir();
        let mut m = net(sim_ir::NetKind::Reg, 8);
        m.array_len = array_len;
        ir.nets.push(m);
        ir.nets.push(net(sim_ir::NetKind::Wire, c_width));
        let k = konst(&mut ir, idx);
        let rhs = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Signal {
            net: 0,
            word: Some(k),
        });
        ir.cont_assigns.push(sim_ir::ContAssign {
            lhs: whole(1),
            rhs,
            delay: None,
        });
        (ir, k)
    }

    #[test]
    fn a_constant_in_range_array_word_is_a_copy_and_the_alias_names_the_word() {
        let (ir, k) = ir_word_copy(1, 4, 8);
        assert_eq!(shape(&copy_nets(&ir)), vec![(1, vec![0])]);
        let (alias, words) = copy_alias(&ir, &[false, false]);
        assert_eq!(alias, vec![0, 0]);
        assert_eq!(words, vec![u32::MAX, k]);
    }

    #[test]
    fn an_out_of_range_word_a_widened_word_and_a_scalar_word_are_not_copies() {
        assert!(copy_nets(&ir_word_copy(4, 4, 8).0).is_empty());
        assert!(copy_nets(&ir_word_copy(1, 4, 9).0).is_empty());
        // `word: Some(_)` on a non-array net is a class field / assoc read.
        assert!(copy_nets(&ir_word_copy(0, 1, 8).0).is_empty());
    }

    /// `assign c = v; assign c = <z>;` — an all-`z` whole-net constant driver at
    /// the net's width contributes nothing (§2 🆕 I ⓑ) and the net stays a copy;
    /// a narrower or partial `z` constant computes, as before.
    fn ir_with_z(width: u32, val: u64, unk: u64) -> SimIr {
        let mut ir = ir_with(2, vec![(1, 0, None)]);
        for n in &mut ir.nets {
            n.width = 8;
            n.msb = 7;
        }
        let cv = ir.consts.len() as u32;
        ir.consts.push(sim_ir::ConstVal {
            width,
            signed: false,
            repr: sim_ir::ConstRepr::Numeric,
            bits: sim_ir::BitPacked {
                val: vec![val],
                unk: vec![unk],
            },
        });
        let e = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Const { val: cv });
        ir.cont_assigns.push(sim_ir::ContAssign {
            lhs: whole(1),
            rhs: e,
            delay: None,
        });
        ir
    }

    #[test]
    fn an_all_z_whole_net_constant_driver_is_ignored() {
        let ir = ir_with_z(8, 0xff, 0xff);
        assert_eq!(shape(&copy_nets(&ir)), vec![(1, vec![0])]);
        let (alias, words) = copy_alias(&ir, &[false, false]);
        assert_eq!(alias, vec![0, 0]);
        assert_eq!(words, vec![u32::MAX, u32::MAX]);
        // narrower (`4'hz` zero-extends), partial (`8'hzx`) and `x` all compute
        assert!(copy_nets(&ir_with_z(4, 0xf, 0xf)).is_empty());
        assert!(copy_nets(&ir_with_z(8, 0xf0, 0xff)).is_empty());
        assert!(copy_nets(&ir_with_z(8, 0x00, 0xff)).is_empty());
    }

    #[test]
    fn delayed_and_multidriven_are_not_copies() {
        assert!(copy_nets(&ir_with(2, vec![(1, 0, Some(7))])).is_empty());
        // two WHOLE-net drivers on net 1 ⇒ a resolution, not a copy.
        assert!(copy_nets(&ir_with(3, vec![(1, 0, None), (1, 2, None)])).is_empty());
    }

    #[test]
    fn width_change_is_not_a_copy() {
        let mut ir = ir_with(2, vec![(1, 0, None)]);
        ir.nets[1].width = 4;
        ir.nets[1].msb = 3;
        assert!(copy_nets(&ir).is_empty());
    }

    #[test]
    fn computed_rhs_is_not_a_copy() {
        let mut ir = ir_with(2, vec![(1, 0, None)]);
        let e = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Unary {
            op: sim_ir::UnOp::BitNot,
            operand: 0,
        });
        ir.cont_assigns[0].rhs = e;
        assert!(copy_nets(&ir).is_empty());
    }

    /// `assign bus[1] = a; assign bus[0] = b;` — the bus has no state either, and
    /// its event status is the OR of both sources'.
    #[test]
    fn disjoint_constant_slices_of_one_bus_are_a_copy() {
        let mut ir = ir_with(2, vec![]);
        let bus = ir.nets.len() as u32;
        ir.nets.push(net(sim_ir::NetKind::Wire, 2));
        let k0 = konst(&mut ir, 0);
        let k1 = konst(&mut ir, 1);
        let w1 = konst(&mut ir, 1);
        for (off, src) in [(k0, 0u32), (k1, 1u32)] {
            ir.cont_assigns.push(sim_ir::ContAssign {
                lhs: sim_ir::Lvalue {
                    chunks: vec![sim_ir::LvalChunk {
                        net: bus,
                        word: None,
                        offset: Some(off),
                        width: Some(w1),
                        kind: sim_ir::SelKind::PartConst,
                    }],
                },
                rhs: src,
                delay: None,
            });
        }
        assert_eq!(shape(&copy_nets(&ir)), vec![(bus, vec![0, 1])]);
    }

    /// A RUNTIME offset selects different bits at different times and leaves the
    /// rest behind — that is state, so it is not a copy.
    #[test]
    fn runtime_offset_slice_is_not_a_copy() {
        let mut ir = ir_with(2, vec![]);
        let bus = ir.nets.len() as u32;
        ir.nets.push(net(sim_ir::NetKind::Wire, 2));
        ir.cont_assigns.push(sim_ir::ContAssign {
            lhs: sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net: bus,
                    word: None,
                    offset: Some(1), // exprs[1] = Signal(1), not a literal
                    width: Some(1),
                    kind: sim_ir::SelKind::PartIdxUp,
                }],
            },
            rhs: 0,
            delay: None,
        });
        assert!(copy_nets(&ir).is_empty());
    }

    /// `assign w = v[0];` is a copy; one computed driver anywhere on the net
    /// disqualifies the whole net.
    #[test]
    fn constant_slice_rhs_is_a_copy_until_a_sibling_computes() {
        let mut ir = ir_with(1, vec![]);
        let v = ir.nets.len() as u32;
        ir.nets.push(net(sim_ir::NetKind::Wire, 2));
        let sig_v = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Signal { net: v, word: None });
        let k0 = konst(&mut ir, 0);
        let w1 = konst(&mut ir, 1);
        let sel = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Select {
            base: sig_v,
            offset: k0,
            width: w1,
            kind: sim_ir::SelKind::Bit,
        });
        ir.cont_assigns.push(sim_ir::ContAssign {
            lhs: whole(0),
            rhs: sel,
            delay: None,
        });
        assert_eq!(shape(&copy_nets(&ir)), vec![(0, vec![v])]);

        // …now give net 0 a second, computed driver on a slice of it.
        let not_v = ir.exprs.len() as u32;
        ir.exprs.push(sim_ir::Expr::Unary {
            op: sim_ir::UnOp::BitNot,
            operand: sig_v,
        });
        ir.cont_assigns.push(sim_ir::ContAssign {
            lhs: sim_ir::Lvalue {
                chunks: vec![sim_ir::LvalChunk {
                    net: 0,
                    word: None,
                    offset: Some(k0),
                    width: Some(w1),
                    kind: sim_ir::SelKind::PartConst,
                }],
            },
            rhs: not_v,
            delay: None,
        });
        assert!(copy_nets(&ir).is_empty());
    }
}
