//! Procedural read-through of whole-net copies (ROADMAP §2 row 33 and its 🆕 I
//! residue) — the per-expression table [`proc_read_alias`] builds.
//!
//! Split out of `levelize/mod.rs` when the walk grew a second body kind: a process
//! that calls a FRAME task or function (a `Terminator::Call` in its body, or an
//! `Expr::Call` in one of its expressions) also executes the callee's blocks, which
//! live in the GLOBAL `ir.blocks` arena and used to be invisible to both halves of the
//! rule — the writer predicate did not see a blocking write inside the callee
//! (`task automatic wr(input x); v = x; endtask` then `$display(c)`) and the reader
//! side did not mark a read inside the callee (`y = c;` in the body, or the actual `c`
//! of `tk(c, r2)`, whose expression lives in the `TaskCallProc` sidecar rather than in
//! a statement). Both oracles print the fresh `a5`; vita printed the settle's `xx`.

use std::collections::{BTreeMap, BTreeSet};

use super::expr_signals;
use crate::{TaskCallFunc, TaskCallProc};
use sim_ir::{BasicBlock as Block, SimIr, Stmt, Terminator};

/// Nets the blocking assignments of ONE block write (the only writes that propagate
/// within the current delta). `Force`/`Release` are shape-reserved no-ops today and
/// deliberately contribute nothing.
pub(super) fn block_blocking_writes(ir: &SimIr, block: &Block, out: &mut BTreeSet<u32>) {
    for &sid in &block.stmts {
        if let Stmt::BlockingAssign { lhs, .. } = &ir.stmts[sid as usize] {
            for c in &lhs.chunks {
                out.insert(c.net);
            }
        }
    }
}

/// The global blocks of a frame body entered at `entry`, in DFS order — every block
/// reachable through the terminators, `Call` included (its `ret_bb` continues the
/// SAME body; the callee's own entry is collected separately by `callee_entries`).
fn frame_blocks(ir: &SimIr, entry: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![entry];
    while let Some(b) = stack.pop() {
        if !seen.insert(b) {
            continue;
        }
        let Some(blk) = ir.blocks.get(b as usize) else {
            continue;
        };
        out.push(b);
        match &blk.term {
            Terminator::Goto { target } => stack.push(*target),
            Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                stack.push(*then_bb);
                stack.push(*else_bb);
            }
            Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
            Terminator::Delay { resume, .. } | Terminator::Wait { resume, .. } => {
                stack.push(*resume)
            }
            Terminator::Fork {
                children,
                resume_bb,
                ..
            } => {
                stack.extend(children.iter().copied());
                stack.push(*resume_bb);
            }
            Terminator::Return => {}
        }
    }
    out
}

/// Every `Expr::Call` callee under `eid` (a frame FUNCTION reached from an
/// expression), by FuncId.
fn expr_callees(ir: &SimIr, eid: u32, out: &mut BTreeSet<u32>) {
    use sim_ir::Expr as E;
    match &ir.exprs[eid as usize] {
        E::Signal { word, .. } => {
            if let Some(w) = word {
                expr_callees(ir, *w, out);
            }
        }
        E::Const { .. } | E::ArrayItem { .. } => {}
        E::Select { base, offset, .. } => {
            expr_callees(ir, *base, out);
            expr_callees(ir, *offset, out);
        }
        E::Concat { parts } => {
            for &p in parts {
                expr_callees(ir, p, out);
            }
        }
        E::Replicate { count, value } => {
            expr_callees(ir, *count, out);
            expr_callees(ir, *value, out);
        }
        E::Unary { operand, .. } => expr_callees(ir, *operand, out),
        E::Binary { lhs, rhs, .. } => {
            expr_callees(ir, *lhs, out);
            expr_callees(ir, *rhs, out);
        }
        E::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            expr_callees(ir, *cond, out);
            expr_callees(ir, *then_e, out);
            expr_callees(ir, *else_e, out);
        }
        E::SysFunc { args, .. } => {
            for &a in args {
                expr_callees(ir, a, out);
            }
        }
        E::Call { func, args } => {
            out.insert(*func);
            for &a in args {
                expr_callees(ir, a, out);
            }
        }
    }
}

/// Walk one block's statements and terminator, handing every expression it
/// evaluates to `f` (an `ExprId`). The `Call` terminator's in-binds are NOT in the
/// block — they are in the task-call sidecar, and the caller adds them.
fn block_exprs(ir: &SimIr, block: &Block, f: &mut dyn FnMut(u32)) {
    let lv_exprs = |lv: &sim_ir::Lvalue, f: &mut dyn FnMut(u32)| {
        for c in &lv.chunks {
            for e in [c.word, c.offset, c.width].into_iter().flatten() {
                f(e);
            }
        }
    };
    for &sid in &block.stmts {
        match &ir.stmts[sid as usize] {
            Stmt::BlockingAssign { lhs, rhs } => {
                f(*rhs);
                lv_exprs(lhs, f);
            }
            Stmt::NonblockingAssign { lhs, rhs, delay } => {
                f(*rhs);
                if let Some(d) = delay {
                    f(*d);
                }
                lv_exprs(lhs, f);
            }
            Stmt::SysTask {
                which: _,
                fmt,
                args,
            } => {
                if let Some(fm) = fmt {
                    f(*fm);
                }
                for &a in args {
                    f(a);
                }
            }
            Stmt::Force { lhs, rhs } => {
                f(*rhs);
                lv_exprs(lhs, f);
            }
            Stmt::Release { lhs } => lv_exprs(lhs, f),
            Stmt::Disable { .. } => {}
        }
    }
    match &block.term {
        Terminator::Branch { cond, .. } => f(*cond),
        Terminator::Delay { amount, .. } => f(*amount),
        Terminator::Wait { cond, .. } => {
            if let sim_ir::WaitCause::Expr { expr } = cond {
                f(*expr);
            }
        }
        Terminator::Goto { .. }
        | Terminator::Fork { .. }
        | Terminator::Call { .. }
        | Terminator::Return => {}
    }
}

/// The frame bodies a process runs, as the ENTRY block of each callee (a
/// `Terminator::Call` target in the process body or, transitively, in a callee; an
/// `Expr::Call` in any expression either evaluates, resolved through `ir.funcs`).
/// Also returns, per process-local block id, the in-bind expressions of that
/// block's `Call` (from the sidecar) so the caller can mark them.
fn callee_entries(
    ir: &SimIr,
    tmpl: usize,
    calls_proc: &TaskCallProc,
    calls_func: &TaskCallFunc,
) -> (BTreeSet<u32>, Vec<u32>) {
    let mut entries: BTreeSet<u32> = BTreeSet::new();
    let mut in_binds: Vec<u32> = Vec::new();
    let mut funcs: BTreeSet<u32> = BTreeSet::new();
    // ⚠️ A `Terminator::Call`'s `target` is the callee's entry AS KNOWN WHEN THE
    // CALLER LOWERED: a callee whose body lowers later still had its reservation
    // placeholder (`entry: 0`), so a nested call to a task declared after its caller
    // pointed at block 0 (review B-1: the walk was name-order dependent). The
    // sidecar's `callee` FuncId is what the engine dispatches on; resolve through it
    // and use `target` only where no sidecar row exists.
    let entry_of = |target: u32, info: Option<&crate::TaskCallInfo>| -> u32 {
        info.and_then(|i| ir.funcs.get(i.callee as usize))
            .map_or(target, |d| d.entry)
    };
    for (bi, block) in ir.processes[tmpl].body.iter().enumerate() {
        block_exprs(ir, block, &mut |e| expr_callees(ir, e, &mut funcs));
        let info = calls_proc.get(&(tmpl as u32, bi as u32));
        if let Terminator::Call { target, .. } = &block.term {
            entries.insert(entry_of(*target, info));
        }
        if let Some(info) = info {
            for &(_, e) in &info.in_binds {
                in_binds.push(e);
                expr_callees(ir, e, &mut funcs);
            }
        }
    }
    // Transitive closure over the callee bodies: a nested `Call` (sidecar keyed by
    // the GLOBAL block index) and an `Expr::Call` inside a callee.
    let mut stack: Vec<u32> = entries.iter().copied().collect();
    stack.extend(
        funcs
            .iter()
            .filter_map(|&f| ir.funcs.get(f as usize).map(|d| d.entry)),
    );
    entries.extend(stack.iter().copied());
    let mut seen_entry: BTreeSet<u32> = BTreeSet::new();
    while let Some(entry) = stack.pop() {
        if !seen_entry.insert(entry) {
            continue;
        }
        entries.insert(entry);
        for b in frame_blocks(ir, entry) {
            let blk = &ir.blocks[b as usize];
            let mut fs = BTreeSet::new();
            block_exprs(ir, blk, &mut |e| expr_callees(ir, e, &mut fs));
            let info = calls_func.get(&b);
            if let Terminator::Call { target, .. } = &blk.term {
                stack.push(entry_of(*target, info));
            }
            if let Some(info) = info {
                for &(_, e) in &info.in_binds {
                    expr_callees(ir, e, &mut fs);
                }
            }
            stack.extend(
                fs.iter()
                    .filter_map(|&f| ir.funcs.get(f as usize).map(|d| d.entry)),
            );
        }
    }
    (entries, in_binds)
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
///
/// §2 🆕 I ⓓ/ⓖ: the "process" is the process AND the frame bodies it runs. Its
/// write set includes a blocking write inside a callee, and its reads include the
/// callee's statements and the call's in-bind actuals (the sidecar). A callee body
/// is ONE set of expressions shared by every calling process, so a read inside it
/// is marked only for the roots EVERY calling process writes — a mixed population
/// (one caller writes `v`, another does not) keeps the settle's value in both, which
/// is what PRE gave both.
pub(crate) fn proc_read_alias(
    ir: &SimIr,
    alias: &[u32],
    alias_word: &[u32],
    calls_proc: &TaskCallProc,
    calls_func: &TaskCallFunc,
) -> (Vec<u32>, Vec<u32>) {
    // (net table, word table), both by ExprId.
    let mut table = (
        vec![u32::MAX; ir.exprs.len()],
        vec![u32::MAX; ir.exprs.len()],
    );
    if alias.iter().enumerate().all(|(i, &a)| a == i as u32) {
        return table;
    }
    let mark = |eid: u32, writes: &BTreeSet<u32>, table: &mut (Vec<u32>, Vec<u32>)| {
        expr_signals(ir, eid, &mut |sig, net| {
            let root = alias[net as usize];
            if root != net && writes.contains(&root) {
                table.0[sig as usize] = root;
                table.1[sig as usize] = alias_word[net as usize];
            }
        });
    };
    // Callee entry block → the roots every calling process writes (`None` = no
    // caller seen yet, the identity of the intersection).
    let mut callee_roots: BTreeMap<u32, Option<BTreeSet<u32>>> = BTreeMap::new();
    for tmpl in 0..ir.processes.len() {
        let (entries, in_binds) = callee_entries(ir, tmpl, calls_proc, calls_func);
        let mut writes = BTreeSet::new();
        for block in &ir.processes[tmpl].body {
            block_blocking_writes(ir, block, &mut writes);
        }
        for &entry in &entries {
            for b in frame_blocks(ir, entry) {
                block_blocking_writes(ir, &ir.blocks[b as usize], &mut writes);
            }
        }
        for &entry in &entries {
            let slot = callee_roots.entry(entry).or_insert(None);
            *slot = Some(match slot.take() {
                None => writes.clone(),
                Some(prev) => prev.intersection(&writes).copied().collect(),
            });
        }
        if writes.is_empty() {
            continue;
        }
        for block in &ir.processes[tmpl].body {
            block_exprs(ir, block, &mut |e| mark(e, &writes, &mut table));
        }
        for &e in &in_binds {
            mark(e, &writes, &mut table);
        }
    }
    for (entry, roots) in &callee_roots {
        let Some(roots) = roots else { continue };
        if roots.is_empty() {
            continue;
        }
        for b in frame_blocks(ir, *entry) {
            let blk = &ir.blocks[b as usize];
            block_exprs(ir, blk, &mut |e| mark(e, roots, &mut table));
            if let Some(info) = calls_func.get(&b) {
                for &(_, e) in &info.in_binds {
                    mark(e, roots, &mut table);
                }
            }
        }
    }
    table
}
