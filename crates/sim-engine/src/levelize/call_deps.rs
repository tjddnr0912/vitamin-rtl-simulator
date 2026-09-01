//! The call-purity half of the dirty settle — "may a continuous assign whose RHS reaches
//! a user FUNCTION be skipped when none of its dependency nets moved?".
//!
//! Split out of `levelize.rs` at the 1000-line policy boundary; the entry point is
//! [`func_read_deps`], and its only consumer is [`super::ca_deps`], which owns the
//! decision this file only informs. Nothing here is serialized, so the move carries no
//! SchemaHash consequence.

use super::*;
use std::collections::BTreeMap;

/// Per `FuncDef`: `Some(nets)` = the value a call to this function returns is a
/// function of its ARGUMENTS and of `nets` — the module nets its body reads, through
/// every function it calls in turn — and of nothing else. `None` = decline; a
/// continuous assign whose RHS reaches it must stay in `ca_always`.
///
/// **What has to be true, and what does not.** iverilog and verilator both re-evaluate
/// a continuous assign exactly when a net in its sensitivity list moves, so the
/// obligation here is *not* that the RHS is a mathematical function — it is that the
/// DEPENDENCY SET IS COMPLETE. Measured on `assign m = f();` where `f` reads module net
/// `z` through no argument: iverilog freezes `m` at its t0 value (empty sensitivity
/// list), verilator and vita track `z`. Collecting `z` into the set is what keeps vita
/// on verilator's side of that split while paying the cost only when `z` moves.
///
/// ⚠️ That sentence is measured, and review measured its LIMIT: make the callee
/// RECURSIVE and verilator changes sides, freezing the value with iverilog while vita
/// still tracks. So the justification above is "vita keeps the answer it already gave",
/// not "vita follows verilator" — for a recursive callee both oracles agree and vita
/// does not. PRE and POST are identical there (the transitive closure converges on the
/// self-edge and re-evaluates on the same set the old always-path did), so it is
/// pre-existing rather than something this certification chose.
///
/// The same measurement disposes of the hazard that looks worst from the outside — a
/// static (non-`automatic`) function's locals persist between calls, so its value can
/// depend on the window it was left. That is true of iverilog's storage too, and
/// iverilog still only calls it when a dependency moves; `bump(a)` carrying a local
/// across calls agrees in all three tools today, and evaluating on dependency changes
/// is what reproduces it. What must not happen is a dependency the set does not name.
///
/// So the walk declines, positively, on everything it cannot attribute to a net:
///
/// * any statement but a blocking assign or a `SysTask` that
///   [`sim_ir::systask_effect_is_eval_local`] admits, and any terminator but
///   `goto`/branch/return;
/// * a `SysFunc` anywhere in the body — `$random` advances a seed other readers draw
///   from and `$fgetc` advances a file position, neither of which is a net, so changing
///   how many times the body runs changes values this set cannot mention;
/// * a read of a HEAP handle net, whose contents move while the handle does not — the
///   condition `ca_deps` already applies to its own directly-read nets;
/// * a read or a write of a net in ANOTHER function's frame window, whose lifetime is a
///   call rather than a net whose changes are noted;
/// * a write to any net outside this function's own window, whose timing this analysis
///   does not model (`assign m = f(a)` with `f` writing a module net is E3009 at
///   elaborate today — the gate is here for the paths that are not).
///
/// ⚠️ The SysTask carve-out is the one that looks reckless and is the same theorem: an
/// effect whose whole result is over when its statement is over happens as many times as
/// the body is evaluated, and this makes the body's evaluation count the ORACLES'
/// evaluation count. Measured on a callee with NO dependency: a `$display` in such a
/// body printed 30 times over five cycles and prints once now, which is what iverilog
/// and verilator print. Give the same callee a dependency and the count is one per
/// change of it (26 → 3 on review's probe) — the same rule, a different number, and not
/// one either oracle can arbitrate, since iverilog's sensitivity list is empty and
/// verilator aborts on the first `$error`. It is also
/// load-bearing rather than a bonus — `verilog-ethernet`'s `lfsr_mask` contains an
/// `$error`, so refusing the whole family would leave the design correct and eighty
/// times slow. What is NOT admitted is anything that leaves state behind for a later
/// evaluation to read, and anything that writes storage through an ARGUMENT, which the
/// "every write lands in my own window" check above cannot see.
///
/// A read of the function's OWN window is not a dependency: it is a formal, the return
/// slot, or a body local, all of which this call establishes or carries itself.
pub(crate) fn func_read_deps(
    ir: &SimIr,
    windows: &[(u32, u32)],
    is_heap: &dyn Fn(u32) -> bool,
) -> (Vec<Option<BTreeSet<u32>>>, Vec<bool>) {
    let n = ir.funcs.len();
    if windows.len() != n {
        // The sidecar is the only description of where a frame window lives; without
        // one, "is this net a local?" has no answer and every call declines.
        return (vec![None; n], vec![false; n]);
    }
    // Nets belonging to SOME frame window. A read of one from another function is a
    // read of storage whose changes `note_change` does not report to `ca_of_net`.
    let mut framed = vec![false; ir.nets.len()];
    for &(base, len) in windows {
        let lo = (base as usize).min(framed.len());
        let hi = lo.saturating_add(len as usize).min(framed.len());
        framed[lo..hi].iter_mut().for_each(|f| *f = true);
    }
    let mut reads: Vec<BTreeSet<u32>> = vec![BTreeSet::new(); n];
    let mut callees: Vec<BTreeSet<u32>> = vec![BTreeSet::new(); n];
    let mut ok = vec![true; n];
    let mut safe = vec![true; n];
    for fi in 0..n {
        let (base, len) = windows[fi];
        let mine = |net: u32| net >= base && net < base.saturating_add(len);
        let mut r = BTreeSet::new();
        let mut c = BTreeSet::new();
        ok[fi] = walk_func_body(ir, fi, &mine, &framed, is_heap, &mut r, &mut c);
        safe[fi] = own_reads_are_definitely_assigned(ir, fi, &mine);
        reads[fi] = r;
        callees[fi] = c;
    }
    // Transitive closure. Declining propagates UP the call graph and reads propagate up
    // with it; a recursive cycle converges, since both lattices only grow.
    //
    // ⚠️ FAIL-CLOSED, deliberately. Reads travel one call-graph edge per round, so a
    // chain of `n` functions converges in at most `n` of them and the `+ 1` is the round
    // that observes it. But "at most n" is an ARGUMENT, and the failure mode if it is
    // wrong is a set that is `Some` and INCOMPLETE — a certified assign that stops
    // tracking a net, at exit 0. Rather than rest on the bound, notice non-convergence
    // and decline everything.
    let mut settled = false;
    for _ in 0..=n {
        let mut moved = false;
        for fi in 0..n {
            for ci in callees[fi].clone() {
                let ci = ci as usize;
                if !ok[ci] && ok[fi] {
                    ok[fi] = false;
                    moved = true;
                }
                if !safe[ci] && safe[fi] {
                    safe[fi] = false;
                    moved = true;
                }
                let add: Vec<u32> = reads[ci].difference(&reads[fi]).copied().collect();
                if !add.is_empty() {
                    moved = true;
                    reads[fi].extend(add);
                }
            }
        }
        if !moved {
            settled = true;
            break;
        }
    }
    if !settled {
        return (vec![None; n], vec![false; n]);
    }
    (
        reads
            .into_iter()
            .zip(ok)
            .map(|(r, k)| k.then_some(r))
            .collect(),
        safe,
    )
}

/// One function's own body, without its callees: collect the module nets it reads and
/// the functions it calls; return `false` on anything [`func_read_deps`] declines.
/// Is every read of one of this function's OWN window slots preceded, on every path from
/// entry, by a write of that slot? `false` ⇒ the value can depend on how many times the
/// body has run, which is exactly what certifying a call changes.
///
/// ⚠️⚠️ **This gate did not exist in the first version of the slice, and the review built
/// the design that needs it.** A plain Verilog function's locals are static, so a read
/// that is not definitely assigned reads what the LAST call left there. Measured, on a
/// counter local incremented once per call and returned, with the assign depending on a
/// net that moves:
///
/// | | vita PRE | ungated POST | iverilog | verilator |
/// |---|---|---|---|---|
/// | saturating at 3 | `3 3 3` | `2 3 3` | `1 2 3` | `3 3 3` |
/// | non-saturating | F4016, exit 1 | `2 3 4`, exit 0 | `1 2 3` | did not converge |
///
/// PRE agreed with verilator exactly on the first row and refused the second loudly;
/// ungated POST agreed with NOBODY on the first and answered silently on the second —
/// correct → silent-wrong and loud → silent-wrong, both of which the ladder forbids.
///
/// ⭐ The residual is one evaluation, not a rule: vita's extra t0 settle pass puts POST
/// exactly one ahead of iverilog on both rows. Closing THAT is what would let this family
/// be certified honestly; until then it declines.
///
/// ⭐⭐ **It is a disjunct, not a veto, and the other arm is what saves the motivating
/// design.** `ca_deps` certifies when this holds OR when the assign's dependency set is
/// EMPTY — because an assign with no dependencies is evaluated once, at the settle seed,
/// and never again, so "how many times" is one and cannot vary. That arm is not a
/// weakening: it is what both oracles do with an empty sensitivity list, and it is
/// measured to give iverilog's answer exactly on the same counter function with its
/// dependency removed (`1 1` where PRE was F4016).
///
/// ⚠️ Without the disjunct this gate alone reverts the slice's headline. `lfsr_mask`
/// clears its mask arrays in a `for` loop before reading them, and a loop that might run
/// zero times is not definite assignment — proving otherwise needs a trip count this IR
/// does not carry. Its dependency set is empty (the argument is a genvar), so it takes
/// the other arm.
///
/// The analysis is textbook definite-assignment over the body's CFG: `in[entry]` = the
/// input formals (the call binding writes them), `in[b]` = the intersection over
/// predecessors, and a block's statements are walked in order so a write earlier in the
/// block covers a read later in it. It is applied to `automatic` functions too — a fresh
/// window has no carry, so the check over-refuses there, but keying a second predicate on
/// `FuncMeta.is_automatic`/`auto_override` would put a mirror of the storage policy in
/// this file for it to drift against.
///
/// ⚠️ ARRAY-WORD IMPRECISION, stated because it is the seam: a slot is a NET, so writing
/// `t[0]` marks all of `t` assigned. What that can produce is a value depending on an
/// unwritten WORD of an array the body does write — narrower than the counter above, and
/// a shape no corpus design or probe has produced.
fn own_reads_are_definitely_assigned(ir: &SimIr, fi: usize, mine: &dyn Fn(u32) -> bool) -> bool {
    use sim_ir::{Stmt, Terminator};
    let Some(fd) = ir.funcs.get(fi) else {
        return false;
    };
    // Reachable blocks, and the formals that are live on entry by construction.
    let mut blocks: Vec<u32> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![fd.entry];
    while let Some(b) = stack.pop() {
        if !seen.insert(b) {
            continue;
        }
        let Some(blk) = ir.blocks.get(b as usize) else {
            return false;
        };
        blocks.push(b);
        succs(&blk.term, &mut stack);
    }
    let writes_of = |b: u32, out: &mut BTreeSet<u32>| {
        if let Some(blk) = ir.blocks.get(b as usize) {
            for &sid in &blk.stmts {
                if let Some(Stmt::BlockingAssign { lhs, .. }) = ir.stmts.get(sid as usize) {
                    for k in &lhs.chunks {
                        if mine(k.net) {
                            out.insert(k.net);
                        }
                    }
                }
            }
        }
    };
    // `None` = TOP (this block has no reachable predecessor state yet), which is the
    // identity for the meet. Entry starts at the formals.
    let base_formals: BTreeSet<u32> = {
        let mut f = BTreeSet::new();
        for b in &blocks {
            if let Some(blk) = ir.blocks.get(*b as usize) {
                let _ = blk;
            }
        }
        // The formals occupy the first `n_params` slots of the window by the layout
        // convention `FuncMeta` documents; `mine` is the only thing that knows where the
        // window starts, so recover the base by scanning for the lowest owned net id the
        // body mentions. Cheaper and exact: ask `mine` at the ids the IR uses.
        for (net, _) in ir.nets.iter().enumerate() {
            let net = net as u32;
            if mine(net) {
                f.insert(net);
                if f.len() as u32 >= ir.funcs[fi].n_params {
                    break;
                }
            }
        }
        f
    };
    let mut state: BTreeMap<u32, Option<BTreeSet<u32>>> =
        blocks.iter().map(|&b| (b, None)).collect();
    state.insert(fd.entry, Some(base_formals));
    for _ in 0..=blocks.len() {
        let mut moved = false;
        for &b in &blocks {
            let Some(Some(cur)) = state.get(&b).cloned().map(Some) else {
                continue;
            };
            let Some(cur) = cur else { continue };
            let mut out = cur.clone();
            writes_of(b, &mut out);
            let Some(blk) = ir.blocks.get(b as usize) else {
                return false;
            };
            let mut sub = Vec::new();
            succs(&blk.term, &mut sub);
            for t in sub {
                let e = state.entry(t).or_insert(None);
                let next = match e.as_ref() {
                    None => Some(out.clone()),
                    Some(prev) => {
                        let meet: BTreeSet<u32> = prev.intersection(&out).copied().collect();
                        (meet != *prev).then_some(meet)
                    }
                };
                if let Some(n) = next {
                    *e = Some(n);
                    moved = true;
                }
            }
        }
        if !moved {
            // Converged: now check every read against the state that reaches it.
            for &b in &blocks {
                let Some(Some(entry_set)) = state.get(&b) else {
                    continue; // unreachable in the meet — nothing reads there
                };
                let mut have = entry_set.clone();
                let Some(blk) = ir.blocks.get(b as usize) else {
                    return false;
                };
                for &sid in &blk.stmts {
                    match ir.stmts.get(sid as usize) {
                        Some(Stmt::BlockingAssign { lhs, rhs }) => {
                            for e in lhs
                                .chunks
                                .iter()
                                .flat_map(|k| [k.word, k.offset, k.width])
                                .flatten()
                            {
                                if !own_reads_ok(ir, e, mine, &have) {
                                    return false;
                                }
                            }
                            if !own_reads_ok(ir, *rhs, mine, &have) {
                                return false;
                            }
                            for k in &lhs.chunks {
                                if mine(k.net) {
                                    have.insert(k.net);
                                }
                            }
                        }
                        Some(Stmt::SysTask { fmt, args, .. }) => {
                            for &e in fmt.iter().chain(args.iter()) {
                                if !own_reads_ok(ir, e, mine, &have) {
                                    return false;
                                }
                            }
                        }
                        _ => return false,
                    }
                }
                if let Terminator::Branch { cond, .. } = &blk.term {
                    if !own_reads_ok(ir, *cond, mine, &have) {
                        return false;
                    }
                }
            }
            return true;
        }
    }
    // Did not converge: fail closed, exactly as `func_read_deps`'s own fixpoint does.
    false
}

/// Every own-window net the expression reads is in `have`.
fn own_reads_ok(ir: &SimIr, eid: u32, mine: &dyn Fn(u32) -> bool, have: &BTreeSet<u32>) -> bool {
    let mut ok = true;
    let mut out = BTreeSet::new();
    collect_own_reads(ir, eid, mine, &mut out);
    for n in out {
        if !have.contains(&n) {
            ok = false;
        }
    }
    ok
}

fn collect_own_reads(ir: &SimIr, eid: u32, mine: &dyn Fn(u32) -> bool, out: &mut BTreeSet<u32>) {
    use sim_ir::Expr as E;
    let Some(e) = ir.exprs.get(eid as usize) else {
        return;
    };
    match e {
        E::Const { .. } | E::ArrayItem { .. } => {}
        E::Signal { net, word } => {
            if mine(*net) {
                out.insert(*net);
            }
            if let Some(w) = word {
                collect_own_reads(ir, *w, mine, out);
            }
        }
        E::Select { base, offset, .. } => {
            collect_own_reads(ir, *base, mine, out);
            collect_own_reads(ir, *offset, mine, out);
        }
        E::Concat { parts } => parts
            .iter()
            .for_each(|&p| collect_own_reads(ir, p, mine, out)),
        E::Replicate { count, value } => {
            collect_own_reads(ir, *count, mine, out);
            collect_own_reads(ir, *value, mine, out);
        }
        E::Unary { operand, .. } => collect_own_reads(ir, *operand, mine, out),
        E::Binary { lhs, rhs, .. } => {
            collect_own_reads(ir, *lhs, mine, out);
            collect_own_reads(ir, *rhs, mine, out);
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            collect_own_reads(ir, *cond, mine, out);
            collect_own_reads(ir, *then_e, mine, out);
            collect_own_reads(ir, *else_e, mine, out);
        }
        E::SysFunc { args, .. } | E::Call { args, .. } => args
            .iter()
            .for_each(|&a| collect_own_reads(ir, a, mine, out)),
    }
}

/// The successor blocks of a terminator, for the walks above.
fn succs(term: &sim_ir::Terminator, out: &mut Vec<u32>) {
    use sim_ir::Terminator as T;
    match term {
        T::Goto { target } => out.push(*target),
        T::Branch {
            then_bb, else_bb, ..
        } => {
            out.push(*then_bb);
            out.push(*else_bb);
        }
        T::Call { ret_bb, .. } => out.push(*ret_bb),
        T::Delay { resume, .. } | T::Wait { resume, .. } => out.push(*resume),
        T::Fork {
            children,
            resume_bb,
            ..
        } => {
            out.extend(children.iter().copied());
            out.push(*resume_bb);
        }
        T::Return => {}
    }
}

fn walk_func_body(
    ir: &SimIr,
    fi: usize,
    mine: &dyn Fn(u32) -> bool,
    framed: &[bool],
    is_heap: &dyn Fn(u32) -> bool,
    reads: &mut BTreeSet<u32>,
    callees: &mut BTreeSet<u32>,
) -> bool {
    use sim_ir::{Stmt, Terminator};
    let Some(fd) = ir.funcs.get(fi) else {
        return false;
    };
    let mut seen = BTreeSet::new();
    let mut stack = vec![fd.entry];
    while let Some(b) = stack.pop() {
        if !seen.insert(b) {
            continue;
        }
        let Some(blk) = ir.blocks.get(b as usize) else {
            return false;
        };
        for &sid in &blk.stmts {
            match ir.stmts.get(sid as usize) {
                Some(Stmt::BlockingAssign { lhs, rhs }) => {
                    for k in &lhs.chunks {
                        if !mine(k.net) {
                            return false;
                        }
                        for e in [k.word, k.offset, k.width].into_iter().flatten() {
                            if !expr_func_reads(ir, e, mine, framed, is_heap, reads, callees) {
                                return false;
                            }
                        }
                    }
                    if !expr_func_reads(ir, *rhs, mine, framed, is_heap, reads, callees) {
                        return false;
                    }
                }
                Some(Stmt::SysTask { which, fmt, args })
                    if sim_ir::systask_effect_is_eval_local(*which) =>
                {
                    // Its arguments are READS, and the format string is an ExprId like
                    // any other — a `$display("%0d", z)` inside the body makes `z` a
                    // dependency exactly as an operand would.
                    for &e in fmt.iter().chain(args.iter()) {
                        if !expr_func_reads(ir, e, mine, framed, is_heap, reads, callees) {
                            return false;
                        }
                    }
                }
                _ => return false,
            }
        }
        match &blk.term {
            Terminator::Goto { target } => stack.push(*target),
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                if !expr_func_reads(ir, *cond, mine, framed, is_heap, reads, callees) {
                    return false;
                }
                stack.push(*then_bb);
                stack.push(*else_bb);
            }
            Terminator::Return => {}
            Terminator::Delay { .. }
            | Terminator::Wait { .. }
            | Terminator::Fork { .. }
            | Terminator::Call { .. } => return false,
        }
    }
    true
}

/// The expression half of [`walk_func_body`]: an `_`-free walk that collects reads and
/// call targets, returning `false` on any node whose value this analysis cannot
/// attribute to nets and arguments.
fn expr_func_reads(
    ir: &SimIr,
    eid: u32,
    mine: &dyn Fn(u32) -> bool,
    framed: &[bool],
    is_heap: &dyn Fn(u32) -> bool,
    reads: &mut BTreeSet<u32>,
    callees: &mut BTreeSet<u32>,
) -> bool {
    use sim_ir::Expr as E;
    let Some(e) = ir.exprs.get(eid as usize) else {
        return false;
    };
    let go = |sub: u32, reads: &mut BTreeSet<u32>, callees: &mut BTreeSet<u32>| {
        expr_func_reads(ir, sub, mine, framed, is_heap, reads, callees)
    };
    match e {
        E::Const { .. } => true,
        E::Signal { net, word } => {
            if !mine(*net) {
                // Not ours: a module net is a dependency, another frame's local is not
                // something `note_change` reports, and a heap handle hides its contents.
                if framed.get(*net as usize).copied().unwrap_or(false) || is_heap(*net) {
                    return false;
                }
                reads.insert(*net);
            }
            word.is_none_or(|w| go(w, reads, callees))
        }
        E::Select { base, offset, .. } => go(*base, reads, callees) && go(*offset, reads, callees),
        E::Concat { parts } => parts.iter().all(|&p| go(p, reads, callees)),
        E::Replicate { count, value } => go(*count, reads, callees) && go(*value, reads, callees),
        E::Unary { operand, .. } => go(*operand, reads, callees),
        E::Binary { lhs, rhs, .. } => go(*lhs, reads, callees) && go(*rhs, reads, callees),
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            go(*cond, reads, callees) && go(*then_e, reads, callees) && go(*else_e, reads, callees)
        }
        E::Call { func, args } => {
            if ir.funcs.get(*func as usize).is_none() {
                return false;
            }
            callees.insert(*func);
            args.iter().all(|&a| go(a, reads, callees))
        }
        E::SysFunc { .. } | E::ArrayItem { .. } => false,
    }
}

/// The nets a continuous assign's RHS reads THROUGH a call, from
/// [`func_read_deps`]'s per-function summary. Kept out of `expr_nets` on purpose:
/// that walk also feeds `comb_ranks`/`fusion_candidates`, which are about the
/// combinational graph between processes, and a callee's reads are not edges there.
pub(crate) fn expr_call_reads(
    ir: &SimIr,
    eid: u32,
    fdeps: &[Option<BTreeSet<u32>>],
    fsafe: &[bool],
    out: &mut BTreeSet<u32>,
    all_safe: &mut bool,
) {
    use sim_ir::Expr as E;
    match &ir.exprs[eid as usize] {
        E::Const { .. } | E::ArrayItem { .. } => {}
        E::Signal { word, .. } => {
            if let Some(w) = word {
                expr_call_reads(ir, *w, fdeps, fsafe, out, all_safe);
            }
        }
        E::Select { base, offset, .. } => {
            expr_call_reads(ir, *base, fdeps, fsafe, out, all_safe);
            expr_call_reads(ir, *offset, fdeps, fsafe, out, all_safe);
        }
        E::Concat { parts } => {
            for &p in parts {
                expr_call_reads(ir, p, fdeps, fsafe, out, all_safe);
            }
        }
        E::Replicate { count, value } => {
            expr_call_reads(ir, *count, fdeps, fsafe, out, all_safe);
            expr_call_reads(ir, *value, fdeps, fsafe, out, all_safe);
        }
        E::Unary { operand, .. } => expr_call_reads(ir, *operand, fdeps, fsafe, out, all_safe),
        E::Binary { lhs, rhs, .. } => {
            expr_call_reads(ir, *lhs, fdeps, fsafe, out, all_safe);
            expr_call_reads(ir, *rhs, fdeps, fsafe, out, all_safe);
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_call_reads(ir, *cond, fdeps, fsafe, out, all_safe);
            expr_call_reads(ir, *then_e, fdeps, fsafe, out, all_safe);
            expr_call_reads(ir, *else_e, fdeps, fsafe, out, all_safe);
        }
        E::SysFunc { args, .. } => {
            for &a in args {
                expr_call_reads(ir, a, fdeps, fsafe, out, all_safe);
            }
        }
        E::Call { func, args } => {
            if let Some(Some(d)) = fdeps.get(*func as usize) {
                out.extend(d.iter().copied());
            }
            if !fsafe.get(*func as usize).copied().unwrap_or(false) {
                *all_safe = false;
            }
            for &a in args {
                expr_call_reads(ir, a, fdeps, fsafe, out, all_safe);
            }
        }
    }
}
