//! Frame/suspend analysis over the frozen arenas — split out of lib.rs (no SchemaHash types here;
//! the frozen serialized types stay in lib.rs because the registry key embeds `module_path!()`).

/// Round-14 V3/V4: the set of TASK `FuncId`s that must run on the suspendable-frame
/// path (executed by the scheduler's `run_process` with a call-stack) instead of the
/// synchronous `run_task`. A task is suspendable iff its reachable body carries a
/// "suspend signal" — a `Delay`/`Wait`/`Fork` terminator, or a `NonblockingAssign`/
/// `SysTask`/`Force`/`Release`/`disable fork` statement — OR it (transitively) calls a
/// suspendable task. Functions are never suspendable (IEEE forbids timing controls in a
/// function), so `is_task == false` funcs are skipped.
///
/// This is a pure function of the `funcs`/`blocks`/`stmts` arenas, so ELABORATE (to lift
/// the E3009 reject for exactly these tasks) and the ENGINE (to route them at run time)
/// compute the *same* set independently — no serialized sidecar, no `format_version`
/// impact, and the two can never disagree. Deterministic (result is order-independent;
/// the returned `BTreeSet` is sorted).
pub fn compute_suspendable_tasks(
    funcs: &[sim_ir::FuncDef],
    blocks: &[sim_ir::BasicBlock],
    stmts: &[sim_ir::Stmt],
    base_nets: &[u32],
    force_suspend: &[bool],
) -> std::collections::BTreeSet<u32> {
    use sim_ir::{DisableKind, Stmt, Terminator};
    // A statement is a suspend signal unless it is a blocking assign or a `disable
    // <scope>` marker (the two things the synchronous `&self` frame executor can run)
    // — EXCEPT a blocking assign that writes a net OUTSIDE this task's own frame
    // window `[lo, hi)` (a module / instance net, INCLUDING an `mem[i]=v` array element
    // or an `{a,b}=x` concat chunk). Such a write needs `&mut`, which only the suspendable
    // process path has, so it IS a signal (r18/r19 infra): it lets an `automatic` /
    // hierarchically-called task mutate instance state (whole-net, part-select, OR array
    // element), matching iverilog, instead of being loud-rejected as "an assignment to a
    // net outside the function" / "a part-select / array-element assignment". A `word`-
    // indexed IN-FRAME write (a frame-local array element, or a class-field HEAP write
    // through an in-frame handle) is NOT marked — it stays a subset the `&self` executor
    // runs. A class-field write through a MODULE-scope handle (out-of-frame, `word=Some`)
    // is over-marked to the suspendable path, which is harmless: the `&mut` executor does
    // the same class-heap write, just not synchronously. `base_nets[fi]` is this func's
    // frame base (== engine `func_table[fi].base_net`, threaded verbatim from elaborate
    // `func_metas`, so both callers classify identically — the pure-function contract
    // holds with `base_nets` now part of the input).
    let stmt_signal = |s: &Stmt, lo: u32, hi: u32| match s {
        Stmt::Disable {
            scope_kind: DisableKind::Scope,
            ..
        } => false,
        Stmt::BlockingAssign { lhs, .. } => lhs.chunks.iter().any(|c| c.net < lo || c.net >= hi),
        _ => true,
    };
    // A `Call` terminator's `target` is the callee's entry block ⇒ callee FuncId.
    let mut entry_to_func = std::collections::HashMap::new();
    for (i, f) in funcs.iter().enumerate() {
        entry_to_func.insert(f.entry, i as u32);
    }
    let n = funcs.len();
    let mut direct = vec![false; n];
    let mut calls: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (fi, f) in funcs.iter().enumerate() {
        if !f.is_task {
            continue;
        }
        // This task's frame window `[lo, hi)` (r18): a blocking assign writing a net
        // outside it is a module/instance-net write → a suspend signal (see above).
        let lo = base_nets.get(fi).copied().unwrap_or(0);
        let hi = lo.saturating_add(f.locals_len);
        // Reachability walk over THIS task's own blocks: intra-func edges are followed;
        // a `Call.target` jumps into another func and is recorded (for transitivity),
        // not followed.
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![f.entry];
        while let Some(b) = stack.pop() {
            if !seen.insert(b) {
                continue;
            }
            let Some(blk) = blocks.get(b as usize) else {
                continue;
            };
            if blk.stmts.iter().any(|&sid| {
                stmts
                    .get(sid as usize)
                    .is_some_and(|s| stmt_signal(s, lo, hi))
            }) {
                direct[fi] = true;
            }
            match &blk.term {
                Terminator::Goto { target } => stack.push(*target),
                Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                Terminator::Call { target, ret_bb } => {
                    if let Some(&cf) = entry_to_func.get(target) {
                        calls[fi].push(cf);
                    }
                    stack.push(*ret_bb);
                }
                Terminator::Delay { resume, .. } | Terminator::Wait { resume, .. } => {
                    direct[fi] = true;
                    stack.push(*resume);
                }
                Terminator::Fork {
                    children,
                    resume_bb,
                    ..
                } => {
                    direct[fi] = true;
                    for &c in children {
                        stack.push(c);
                    }
                    stack.push(*resume_bb);
                }
                Terminator::Return => {}
            }
        }
    }
    // §4.5.208: FORCE a frame task with a deferred hier enable suspendable. The callee's
    // suspend status is invisible through the placeholder `Call.target` when elaborate runs
    // this (per-instance, pre-resolve), so both computes over-approximate CONSISTENTLY from
    // the same `FuncMeta.has_hier_call` flag (threaded verbatim to both callers) — a sound
    // over-approximation (the suspendable `&mut` path is a superset that also runs a
    // non-suspending callee). Applied to `direct` BEFORE the transitive closure so a caller
    // of the forced task is lifted too.
    for (i, d) in direct.iter_mut().enumerate() {
        if force_suspend.get(i).copied().unwrap_or(false) {
            *d = true;
        }
    }
    // Transitive closure: a task that calls a suspendable task is itself suspendable.
    let mut susp = direct;
    loop {
        let mut changed = false;
        for i in 0..n {
            if !susp[i] && calls[i].iter().any(|&c| susp[c as usize]) {
                susp[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (0..n as u32).filter(|&i| susp[i as usize]).collect()
}
