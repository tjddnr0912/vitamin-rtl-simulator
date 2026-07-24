//! Stage-1 fork-in-frame classifier — the `fork … join[_any|_none]` arm-admission
//! machinery, split out of `frames_classify.rs` (mechanical move; keeps that file under
//! the 1000-line module-size cap). `ForkAdmit` + [`Elaborator::fork_arms_self_contained`]
//! decide whether a `fork` inside a suspendable task body is runnable on the owned-window
//! model (Case A) or must stay LOUD (Case B / nested / disable-fork); `classify_one_arm`
//! and the `expr_reads_range` / `chunk_touches_range` helpers are the per-arm workers.
//! The caller (`frame_body_is_leaf_nonsuspending`) and the `frame_task_has_unsafe_construct`
//! backstop stay in `frames_classify.rs` — they consume `expr_reads_range` (`pub(crate)`)
//! and the `ForkAdmit` verdict.

use super::*;

/// Stage-1 fork-in-frame admission verdict for a `fork … join[_any|_none]` inside a
/// suspendable task body. The gate ([`Elaborator::fork_arms_self_contained`]) walks
/// every arm's reachable blocks and folds them into the worst verdict:
/// - `CaseA`: no arm reads/writes a net in the enclosing task's frame-local range
///   `[base_net, base_net+locals_len)` → runnable on the existing owned-window model
///   (the single-threaded scheduler + stash/restore keep the concurrent children and
///   the parked parent from co-residing on the shared `frame_stack`).
/// - `CaseB`: some arm touches a parent frame-local → needs the shared-window arena
///   (Stage 2/3). LOUD in Stage 1 (correct-or-loud).
/// - `Loud`: a NESTED fork, a `disable fork`, or any construct the arm walk cannot
///   prove self-contained. LOUD every stage.
///
/// `CaseB`/`Loud` both keep the task loud in Stage 1; the split exists so a later
/// stage can admit `CaseB` (join-all) without touching the `Loud` structural rejects.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ForkAdmit {
    CaseA,
    CaseB,
    Loud,
}

impl Elaborator<'_> {
    /// Stage-1 fork-in-frame: true if evaluating expression `e` reads any net in
    /// `[lo,hi)` — a parent frame-local of the enclosing suspendable task. Reuses the
    /// shared `collect_expr_reads` net-collection walk (Signal/Select/Concat/… children).
    pub(crate) fn expr_reads_range(&self, e: u32, lo: u32, hi: u32) -> bool {
        let mut reads = std::collections::BTreeSet::new();
        self.collect_expr_reads(e, &mut reads);
        reads.iter().any(|&n| n >= lo && n < hi)
    }

    /// Stage-1 fork-in-frame: does an lvalue CHUNK touch a net in `[lo,hi)` — either its
    /// TARGET net, its array-WORD index expr (`mem[d]`), or its part-select OFFSET expr
    /// (`r[d +: 8]`)? The two index exprs are evaluated in the ARM's context, so a parent
    /// frame-local read there is Case B exactly like a write to the target net — otherwise
    /// the arm runs on the empty owned window and the index read panics / mis-reads the
    /// static slab. (`width` is a compile-time constant, never a frame-local read.)
    fn chunk_touches_range(&self, c: &ir::LvalChunk, lo: u32, hi: u32) -> bool {
        (c.net >= lo && c.net < hi)
            || c.word.is_some_and(|e| self.expr_reads_range(e, lo, hi))
            || c.offset.is_some_and(|e| self.expr_reads_range(e, lo, hi))
    }

    /// Stage-1 fork-in-frame: classify ONE fork arm subtree, from `arm_entry` up to the
    /// shared `join_bb` sentinel (a fork arm is sealed with `goto(join_bb)`; the join
    /// block is never-executed, so the walk stops there). Returns `Loud` on a nested
    /// fork / `disable fork` / any unrecognized statement; `CaseB` if the arm reads or
    /// writes a net in `[lo,hi)` (a parent frame-local); else `CaseA`. Does NOT descend
    /// into a called task via a `Call` — the callee has its own frame and cannot touch
    /// this task's locals — but DOES inspect that `Call`'s in-/out-binds (evaluated in
    /// the ARM context), where a parent-local actual would otherwise slip past the walk.
    ///
    /// CONSERVATIVE by construction: under-detecting Case B is silent-wrong (a Case-B
    /// fork would run on the empty owned window and read garbage), so any stmt kind not
    /// proven self-contained, and any `Call` whose bind table is not yet populated (a
    /// not-yet-resolved deferred hier enable), escalates to `CaseB` (stays loud).
    fn classify_one_arm(&self, arm_entry: u32, join_bb: u32, lo: u32, hi: u32) -> ForkAdmit {
        let in_range = |n: u32| n >= lo && n < hi;
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![arm_entry];
        let mut admit = ForkAdmit::CaseA;
        while let Some(bi) = stack.pop() {
            if bi == join_bb || !seen.insert(bi) {
                continue; // stop at the join sentinel; skip already-visited
            }
            let Some(blk) = self.func_blocks.get(bi as usize) else {
                continue;
            };
            // A nested fork (fork inside a fork arm) is loud every stage.
            if let ir::Terminator::Fork { .. } = &blk.term {
                return ForkAdmit::Loud;
            }
            for &sid in &blk.stmts {
                match &self.stmts[sid as usize] {
                    // `disable fork` in a frame body — a fork-family construct with no
                    // in-frame engine support (design §8) → loud.
                    ir::Stmt::Disable {
                        scope_kind: ir::DisableKind::Fork,
                        ..
                    } => return ForkAdmit::Loud,
                    // `disable <named block>` (break/continue idiom) is a no-op marker +
                    // Goto — self-contained, safe.
                    ir::Stmt::Disable {
                        scope_kind: ir::DisableKind::Scope,
                        ..
                    } => {}
                    ir::Stmt::BlockingAssign { lhs, rhs } => {
                        if lhs
                            .chunks
                            .iter()
                            .any(|c| self.chunk_touches_range(c, lo, hi))
                            || self.expr_reads_range(*rhs, lo, hi)
                        {
                            admit = ForkAdmit::CaseB;
                        }
                    }
                    ir::Stmt::NonblockingAssign { lhs, rhs, .. } => {
                        if lhs
                            .chunks
                            .iter()
                            .any(|c| self.chunk_touches_range(c, lo, hi))
                            || self.expr_reads_range(*rhs, lo, hi)
                        {
                            admit = ForkAdmit::CaseB;
                        }
                    }
                    ir::Stmt::SysTask { args, .. } => {
                        if args.iter().any(|&a| self.expr_reads_range(a, lo, hi)) {
                            admit = ForkAdmit::CaseB;
                        }
                    }
                    // Force/Release (and any future stmt kind) inside an arm — not proven
                    // self-contained → conservatively Case B (correct-or-loud).
                    _ => admit = ForkAdmit::CaseB,
                }
            }
            match &blk.term {
                ir::Terminator::Goto { target } => stack.push(*target),
                ir::Terminator::Branch {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    if self.expr_reads_range(*cond, lo, hi) {
                        admit = ForkAdmit::CaseB;
                    }
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                ir::Terminator::Wait { cond, resume } => {
                    if self.wait_cond_reads_frame_local(cond, &in_range) {
                        admit = ForkAdmit::CaseB;
                    }
                    stack.push(*resume);
                }
                ir::Terminator::Delay { amount, resume, .. } => {
                    // The delay VALUE (`#(d)`) is evaluated in the arm's context — a parent
                    // frame-local there is Case B, else the empty owned window panics.
                    if self.expr_reads_range(*amount, lo, hi) {
                        admit = ForkAdmit::CaseB;
                    }
                    stack.push(*resume);
                }
                ir::Terminator::Call { ret_bb, .. } => {
                    // The arm's nested task-call args are evaluated in the ARM (caller)
                    // context and live in the side table (keyed by the GLOBAL block id of
                    // this Call), NOT the arm's block stmts — inspect them for a parent
                    // frame-local read (in-bind) or write-back (out-bind). A MISSING entry
                    // means a deferred hier enable not yet resolved at this pass → its
                    // actuals are unknown, so classify Case B (conservative).
                    match self.task_calls_func.get(&bi) {
                        Some(info) => {
                            if info
                                .in_binds
                                .iter()
                                .any(|&(_, arg)| self.expr_reads_range(arg, lo, hi))
                                || info.out_binds.iter().any(|(_, lv)| {
                                    lv.chunks
                                        .iter()
                                        .any(|c| self.chunk_touches_range(c, lo, hi))
                                })
                            {
                                admit = ForkAdmit::CaseB;
                            }
                        }
                        None => admit = ForkAdmit::CaseB,
                    }
                    stack.push(*ret_bb);
                }
                ir::Terminator::Fork { .. } => return ForkAdmit::Loud,
                // A `return` (or a `disable`-escape) inside a fork arm diverts the arm to
                // the task's SHARED exit block (which carries `Terminator::Return`). At
                // runtime the synthetic ARM frame — spawned by `exec_fork`, which BYPASSES
                // `enter_task_frame` — would hit the in-frame `Return` handler and run
                // `exit_task_frame`/`frame_dyn_free` on the PARENT task's FuncId, popping the
                // parent's frame scope and freeing its dyn-array locals while the parent is
                // parked (silent data corruption). iverilog also REJECTS `return` inside a
                // fork-join block (IEEE 1800 §9.3/§13.4.1) → LOUD. A well-formed arm instead
                // terminates via `goto(join_bb)` and the walk stops at the join sentinel
                // above, never reaching a `Return` terminator — so this is non-false-rejecting.
                ir::Terminator::Return => return ForkAdmit::Loud,
            }
        }
        admit
    }

    /// Stage-1 fork-in-frame admission for a suspendable task `[lo,hi)` = its frame-local
    /// net range. Walks the task's reachable blocks (following its OWN CFG edges; a `Call`
    /// stays in-function via `ret_bb`) and, for every `Terminator::Fork`, classifies each
    /// arm with [`classify_one_arm`], folding into the worst verdict. `Loud` short-circuits;
    /// otherwise the worst of `CaseA`/`CaseB` wins. Called ONCE per task (memoized by the
    /// caller), so multiple forks in one body still cost a single whole-body walk.
    pub(crate) fn fork_arms_self_contained(&self, entry: u32, lo: u32, hi: u32) -> ForkAdmit {
        let mut worst = ForkAdmit::CaseA;
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![entry];
        while let Some(bi) = stack.pop() {
            if !seen.insert(bi) {
                continue;
            }
            let Some(blk) = self.func_blocks.get(bi as usize) else {
                continue;
            };
            if let ir::Terminator::Fork { children, join, .. } = &blk.term {
                // Per-fork verdict: fold this fork's own arms first (Loud short-circuits).
                let mut this_fork = ForkAdmit::CaseA;
                for &arm in children {
                    match self.classify_one_arm(arm, *join, lo, hi) {
                        ForkAdmit::Loud => return ForkAdmit::Loud,
                        ForkAdmit::CaseB => this_fork = ForkAdmit::CaseB,
                        ForkAdmit::CaseA => {}
                    }
                }
                // Stage-3: a Case-B fork of ANY join mode is admitted (the shared-window arena
                // is now REFCOUNTED — `frame_window_rc`). Under `join_any`/`join_none` a
                // surviving arm can outlive the parent's resume; the parent releases its
                // reference at `Return` and the arm keeps the window alive until it completes,
                // so the slot is freed only when the LAST referencing frame drops. (Stage 2
                // gated `join_any`/`join_none` to `Loud` here for lack of that refcount; Stage
                // 3 removes the gate.) The remaining `Loud` cases — a NESTED fork, a
                // `disable fork`, or any arm construct not proven self-contained — are handled
                // by `classify_one_arm` above (its `Loud` short-circuits this loop). A Case-A
                // fork of any mode is unaffected (the owned-window model isolates it).
                if this_fork == ForkAdmit::CaseB {
                    worst = ForkAdmit::CaseB;
                }
            }
            // Follow this task's own CFG edges (a `Call` follows `ret_bb`, never into the
            // callee), plus a fork's children/join/resume so every reachable Fork is seen.
            match &blk.term {
                ir::Terminator::Goto { target } => stack.push(*target),
                ir::Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                    stack.push(*resume)
                }
                ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
                ir::Terminator::Fork {
                    children,
                    join,
                    resume_bb,
                } => {
                    stack.extend(children.iter().copied());
                    stack.push(*join);
                    stack.push(*resume_bb);
                }
                ir::Terminator::Return => {}
            }
        }
        worst
    }
}
