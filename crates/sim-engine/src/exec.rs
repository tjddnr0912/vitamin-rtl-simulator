//! Process executor: the FROZEN basic-block PC state machine. Runs a
//! `Process.body` from a resume block, executing each block's statements then
//! its terminator, suspending on Delay/Wait and completing on Return.
//!
//! Operates on the [`Scheduler`] so a single `&mut` reaches both the net table
//! (`sched.st`) for immediate blocking writes and the scheduler queues for
//! NBA/Delay/Wait scheduling.

use sim_ir::{DelayRegion, Lvalue, Stmt, SysTaskId, Terminator, WaitCause};

use crate::builtins::Ctl;
use crate::sched::Scheduler;
use crate::value::Value;

/// Outcome of one process activation.
pub(crate) enum Step {
    Done,
    Suspended,
    Finish,
    Stop,
    Fatal,
}

/// The body↔kernel ABI seam (P7b): the calls a process body — the tree-walking
/// interpreter OR a Stage-C compiled body — makes to drive the simulation kernel.
/// A READ phase (`k_eval_for_lvalue`/`k_resolve_lvalue_offsets`, no mutation) then a
/// WRITE phase (`k_write_lvalue`/`k_schedule_nba`/`k_dispatch_systask`). The
/// interpreter's statement executor ([`compute_effect`]/[`apply_effect`]) is GENERIC
/// over this trait, so it already runs against the seam — proving the surface is
/// sufficient for a compiled VM to reuse verbatim (the kernel never knows which body
/// drove it; only its control flow differs).
///
/// SCOPE: the STATEMENT-phase ABI for the suspend-free P9 class plus the C1
/// terminator/control surface. Method names are `k_*`-prefixed to stay distinct from
/// `Scheduler`'s inherent methods (the impl just forwards). Suspend / resume
/// (Delay/Wait) and fork are deliberately ABSENT: those bodies stay on the
/// interpreter, which owns the resume-PC state machine (a compiled body runs
/// atomically entry→Return and never suspends — see the P9 predicate).
pub(crate) trait Kernel {
    /// READ: evaluate `rhs` context-sized to `lhs`'s width (IEEE assignment rule).
    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value;
    /// READ: evaluate a pre-compiled native expression program (VM-only fast path,
    /// [C4-lite]). Byte-identical to `k_eval_for_lvalue` for the bounded subset
    /// `native_eval::try_compile` accepts; the compiler only emits this where it does.
    fn k_eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value;
    /// READ: resolve each LHS chunk's `(bit-offset, array-word)` NOW (dynamic index).
    fn k_resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets;
    /// WRITE: blocking write of `value` into `lhs` at the resolved `offsets`
    /// (the full enum, not the pair slice — the assoc lane carries an i64 key).
    fn k_write_lvalue(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets);
    /// WRITE: schedule a nonblocking update (LHS index sampled at schedule time).
    fn k_schedule_nba(&mut self, lhs: Lvalue, value: Value);
    /// READ: evaluate a delay ExprId into global-precision ticks (module-mult
    /// scaled; X/Z → 0 — the shared `Terminator::Delay` rule).
    fn k_delay_ticks(&self, eid: u32) -> u64;
    /// WRITE: schedule a TRANSPORT nonblocking update into the NBA region of
    /// `now + ticks` (v5 increment A; index sampled at schedule time).
    fn k_schedule_nba_at(&mut self, lhs: Lvalue, value: Value, ticks: u64);
    /// WRITE: `force lhs = value` (whole-net, continuous re-eval — §9.3.2). `sid`
    /// keys the assign-rank side table: a marked stmt is a procedural `assign`
    /// (§9.3.1, WEAK rank — a real force overrides it and parks it as latent).
    fn k_force(&mut self, lhs: &Lvalue, value: Value, rhs: u32, sid: u32);
    /// WRITE: `release lhs` (net → driver re-settles; variable → keeps value).
    /// `sid` keys the assign-rank table: a marked stmt is a `deassign` (removes
    /// the assign wherever it lives); a real release removes the FORCE and
    /// re-pins a latent assign if one is parked.
    fn k_release(&mut self, lhs: &Lvalue, sid: u32);
    /// WRITE: run a system task, returning its control outcome. `sid` is the
    /// StmtId — the severity side table (`$fatal`/`$error`/…, P1-1) is keyed by it.
    fn k_dispatch_systask(
        &mut self,
        which: SysTaskId,
        fmt: Option<u32>,
        args: &[u32],
        sid: u32,
    ) -> Ctl;
    /// READ: is `rhs` (the WHOLE expression) a queue-pop SysFunc? Pops are
    /// side-effecting, so the executor intercepts them as a statement-level
    /// effect (`StmtEffect::QPop`) instead of routing them through the pure
    /// eval funnel — the same family as `SysTask` ("its own read+write happen
    /// inside dispatch"). Any OTHER placement of a pop X-poisons in eval.
    fn k_queue_pop_rhs(&self, rhs: u32) -> bool;
    /// WRITE: pop one element (front/back per `rhs`'s SysFuncId) from the
    /// queue behind `rhs`'s handle argument, context-sized to `lhs` exactly as
    /// `k_eval_for_lvalue` sizes an rhs. Empty / non-queue → element-width X
    /// + warn-once (v5 ④; iverilog live: warning + x).
    fn k_queue_pop(&mut self, lhs: &Lvalue, rhs: u32) -> Value;
    /// READ: is `rhs` a SEEDED `$random(seed)` call (v7)? It writes the new
    /// LCG state back into the ref seed variable, so it is a statement-level
    /// effect like the pops; elaborate rejects every other seeded placement.
    fn k_random_seeded_rhs(&self, rhs: u32) -> bool;
    /// WRITE: one Annex-N draw seeded from the ref variable; writes the
    /// updated seed back to its net and returns the 32-bit signed draw.
    fn k_random_seeded(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$dist_uniform(seed, start, end)` (v9, rank 6)? It writes
    /// the advanced seed back into the ref seed variable — the seeded-$random
    /// family; elaborate enforces direct-rhs placement.
    fn k_dist_seeded_rhs(&self, rhs: u32) -> bool;
    /// WRITE: one Annex `rtl_dist_uniform` draw over `[start, end]` seeded from
    /// the ref variable; writes the updated seed back and returns the draw.
    fn k_dist_seeded(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$cast(dst, src)` func-form (v9, rank 6)? It writes the
    /// `dst` ref arg — the `$value$plusargs` family (direct-rhs only).
    fn k_cast_rhs(&self, rhs: u32) -> bool;
    /// WRITE: assign (resize) `src` into the `dst` ref arg and return 1 — an
    /// integral `$cast` always succeeds in this class-free subset (IEEE §6.24.2).
    fn k_cast(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$value$plusargs(fmt, var)` call (v7)? It writes the
    /// ref VAR on a match — statement-level effect, the seeded-$random family.
    fn k_value_plusargs_rhs(&self, rhs: u32) -> bool;
    /// WRITE: search the plusargs, convert the first match's remainder per the
    /// format spec into the ref var, return 1/0 (32-bit signed).
    fn k_value_plusargs(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$fopen(...)` call (v7)? It mutates the file table —
    /// statement-level effect, direct-rhs only (elaborate enforces).
    fn k_fopen_rhs(&self, rhs: u32) -> bool;
    /// WRITE: open the file (mode form → 0x8000_0003… fd; MCD form → channel
    /// bit) and return the descriptor value; failure returns 0.
    fn k_fopen(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$fgetc(fd)` (v9)? Reading a byte advances the fd read
    /// position — a statement-level effect, direct-rhs only (like `Fopen`).
    fn k_fgetc_rhs(&self, rhs: u32) -> bool;
    /// WRITE: read one byte from the fd (honoring `$ungetc` pushback); a 32-bit
    /// signed int, or −1 (0xffff_ffff) at EOF / bad fd.
    fn k_fgetc(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$feof(fd)` (v9)? It reads the lazy-EOF flag a prior
    /// failed read set — routed through the same direct-rhs family so a fd read
    /// and its EOF test share one evaluation order.
    fn k_feof_rhs(&self, rhs: u32) -> bool;
    /// WRITE: nonzero once the fd has hit EOF; a bad/closed fd returns −1
    /// (iverilog parity — NOT 0).
    fn k_feof(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$ungetc(c, fd)` (v9)? It mutates the fd pushback
    /// buffer — statement-level effect, direct-rhs only.
    fn k_ungetc_rhs(&self, rhs: u32) -> bool;
    /// WRITE: push byte `c` back onto the fd (1-deep, last-push-wins) and clear
    /// EOF; 0 on success, −1 if `c` is EOF (−1) or the fd is bad.
    fn k_ungetc(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$fgets(str, fd)` (v9)? It reads a line, advancing the
    /// fd AND writing the str destination — statement-level effect, direct-rhs
    /// only (the `$value$plusargs` family: the worker writes the dest arg and
    /// returns the byte count to `lhs`).
    fn k_fgets_rhs(&self, rhs: u32) -> bool;
    /// WRITE: read a line (up to the dest width in bytes, or through a newline)
    /// into the str destination right-justified; returns the byte count, or 0
    /// at EOF (leaving the destination UNCHANGED).
    fn k_fgets(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$fread(target, fd[, start[, count]])` (v9)? It reads
    /// binary bytes into a reg or memory — statement-level effect, direct-rhs
    /// only (the `$value$plusargs` family).
    fn k_fread_rhs(&self, rhs: u32) -> bool;
    /// WRITE: binary-read into the target reg/memory (big-endian, MSB-first
    /// slot fill; a partial fill leaves the unread LOW bytes at their prior
    /// value); returns the total byte count.
    fn k_fread(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$fscanf(fd, fmt, args...)` (v9)? It parses the fd byte
    /// stream and WRITES the matched ref args — statement-level effect (the
    /// FIRST multi-ref-write intercept), direct-rhs only.
    fn k_fscanf_rhs(&self, rhs: u32) -> bool;
    /// WRITE: run the scanf parser over the fd stream, writing each matched
    /// conversion to its ref arg; returns the conversion count (−1 at EOF
    /// before any input).
    fn k_fscanf(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` a `$sscanf(str, fmt, args...)` (v9)? Like `Fscanf` but the
    /// source is a string VALUE, not an fd.
    fn k_sscanf_rhs(&self, rhs: u32) -> bool;
    /// WRITE: run the scanf parser over the source string; see `k_fscanf`.
    fn k_sscanf(&mut self, rhs: u32) -> Value;
    /// WRITE: `disable fork` — kill every active descendant of the calling
    /// process (P2-E; the activity arena marks them dead, stale queue
    /// entries drop at the dispatch choke).
    fn k_disable_fork(&mut self);
    /// READ: is `rhs` a `$sformatf(fmt, args…)` call (v7)? Formatting needs
    /// the full kernel (the format engine renders through the Scheduler), so
    /// it is a statement-level effect; other placements are loud at elaborate.
    fn k_sformatf_rhs(&self, rhs: u32) -> bool;
    /// WRITE-phase render of a `$sformatf` rhs → the STRING-domain value.
    fn k_sformatf(&mut self, rhs: u32) -> Value;
    /// READ: is `rhs` an assoc-iteration SysFunc (`first`/`next`/`last`/
    /// `prev`)? They WRITE their ref key argument, so like the pops they are
    /// statement-level effects (`StmtEffect::AssocIter`); any other placement
    /// X-poisons in eval (v6).
    fn k_assoc_iter_rhs(&self, rhs: u32) -> bool;
    /// WRITE: run one assoc-iteration step — writes the ref key variable on a
    /// hit and returns the int STATUS (1 found / 0 none / −1 ref-arg too
    /// narrow, key truncated + W4020), context-sized to `lhs` (v6; hand-IEEE
    /// §7.9.4 — no iverilog lane). Dense 0..size-1 walk on dyn/queue handles
    /// (the internal `foreach` desugar target).
    fn k_assoc_iter(&mut self, lhs: &Lvalue, rhs: u32) -> Value;

    // ── terminator / control surface (C1) ──
    // The control-flow ABI a compiled body needs beyond the statement surface above:
    // `Branch` truthiness, `Return` re-arm, and the per-activation termination guard.
    // All FORWARD verbatim to the interpreter's inherent methods (the VM reproduces
    // control flow bit-for-bit through the SAME kernel — it never reimplements it).

    /// CONTROL: tri-valued truthiness of `eid` for a `Branch` (X/Z → false), built on
    /// the same `EvalCtx` the interpreter's `Terminator::Branch` uses (exec.rs:120).
    fn k_truthy(&self, eid: u32) -> bool;
    /// CONTROL: re-arm the process after `Return`, preserving the Edge/Level/Initial
    /// asymmetry (NOT reimplemented). TOTAL on the codegen-able class: such a body has
    /// no `Fork` terminator, so it can never be entered as a fork child (a child's
    /// `Return` is routed to `on_child_complete`, never to `rearm`) — `is_codegen_able`
    /// scans the WHOLE body, so the VM only ever drives top-level activities here.
    fn k_rearm(&mut self, proc: u32);
    /// CONTROL: the infinite-delta termination-guard ceiling (mirror exec.rs:177).
    fn k_max_deltas(&self) -> u64;
    /// CONTROL: flag a fatal (delta-limit) termination (mirror exec.rs:178).
    fn k_mark_fatal(&mut self);
    /// READ (N7): the class_id this StmtId allocates via `new`, or `None` for a
    /// plain blocking assign. A `Some` makes the executor build a `ClassNew`
    /// effect (allocate a heap object) instead of evaluating the placeholder rhs.
    fn k_class_new_site(&self, sid: u32) -> Option<u32>;
    /// WRITE (N7): allocate a fresh object of `class_id` on the heap, returning
    /// its object-id as a 32-bit handle Value to store into the lhs.
    fn k_class_alloc(&mut self, class_id: u32) -> Value;
}

/// Execute activity `pi` starting at body block `start`. `pi` is a runtime
/// ACTIVITY id (index into `Scheduler::activities`), NOT a declaration index —
/// the body/sensitivity are resolved through `activities[pi].template`.
pub(crate) fn run_process(sched: &mut Scheduler, pi: u32, mut bb: u32) -> Step {
    let mut guard: u64 = 0;
    loop {
        // ── CENTRALIZED CHILD-COMPLETION INTERCEPT (terminator-agnostic) ──
        // If this activity is a fork child and the NEXT bb to fetch is its barrier's
        // join_bb, the child has completed. Report + die BEFORE the join_bb block is
        // ever fetched (join_bb is a never-executed sentinel). This catches the child
        // whether it arrives via Goto, Branch, or a resumed Delay/Wait, so a child
        // whose last statement is an if/case/delay/wait into join_bb is handled.
        if sched.activity_is_child(pi) {
            if let Some(jr) = sched.activity_join_ref(pi) {
                if bb == sched.barrier_join_bb(jr) {
                    sched.on_child_complete(jr, pi);
                    return Step::Done; // child dead; rearm skips it (is_child)
                }
            }
        }
        // Defense-in-depth: a non-child must NEVER fetch a live barrier's join_bb.
        #[cfg(debug_assertions)]
        sched.assert_not_parent_at_join(pi, bb);

        // Snapshot the block's stmt ids + terminator (process-local indexing,
        // resolved through this activity's template).
        let tmpl = sched.activity_template(pi) as usize;
        // $time/$realtime evaluated in this process scale by its module multiplier.
        sched.st.cur_time_mult = sched
            .st
            .proc_multipliers
            .get(tmpl)
            .copied()
            .unwrap_or(1)
            .max(1);
        // `%m` scope of this process (P2-11); flat "top" when no sidecar. Skip the
        // String alloc when the scope is already current (the common case for a
        // process resumed many times) — `clone_from` reuses capacity otherwise.
        match sched.st.proc_scopes.get(tmpl) {
            Some(s) => {
                if &sched.st.cur_scope != s {
                    sched.st.cur_scope.clone_from(s);
                }
            }
            None => {
                if sched.st.cur_scope != "top" {
                    sched.st.cur_scope.clear();
                    sched.st.cur_scope.push_str("top");
                }
            }
        }
        // `ir` is `&'ir SimIr` (shared, outliving this `&mut sched` borrow), so the
        // block's stmt list and terminator are read IN PLACE. The previous
        // `stmts.clone()`/`term.clone()`/per-stmt `Stmt::clone()` allocated on every
        // block activation — the second-largest malloc source of clock-bound designs.
        let ir = sched.st.ir;
        let block = &ir.processes[tmpl].body[bb as usize];

        // ── statements (P7a read/write-phase split) ──
        // Each statement executes in two explicit phases: a READ phase
        // (`compute_effect`, pure eval over `&Scheduler` — no mutation) that produces
        // a self-contained [`StmtEffect`], then a WRITE phase (`apply_effect`, the
        // `&mut Scheduler` kernel calls). This is the seam a codegen body needs: it
        // inlines the read phase as native code and routes the write phase through the
        // kernel (P7b puts apply_effect's calls behind a trait). Behaviour is
        // byte-identical to the prior inline form — same evals, same writes, same order.
        for &sid in &block.stmts {
            let stmt = &ir.stmts[sid as usize];
            let effect = compute_effect(&*sched, stmt, sid); // READ phase via Kernel seam
            if let Some(step) = apply_effect(sched, effect) {
                return step; // a SysTask returned Finish/Stop/Fatal
            }
        }

        // ── terminator ──
        match &block.term {
            Terminator::Goto { target } => {
                bb = *target;
            }
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                bb = if sched.truthy(*cond) {
                    *then_bb
                } else {
                    *else_bb
                };
            }
            Terminator::Delay {
                amount,
                region,
                resume,
            } => {
                // format_version 4: `amount` is the ExprId of the RAW delay
                // value in module units — evaluate NOW and scale by this
                // process's multiplier (X/Z → 0; real → round(v×M)).
                let ticks = sched.delay_ticks(*amount);
                let inactive = matches!(region, DelayRegion::Inactive) || ticks == 0;
                let tick = sched.now().saturating_add(ticks);
                sched.schedule_resume(pi, *resume, tick, inactive);
                return Step::Suspended;
            }
            Terminator::Wait { cond, resume } => {
                match cond {
                    WaitCause::Expr { expr } => {
                        if sched.truthy(*expr) {
                            bb = *resume; // already true → fall through
                            guard += 1;
                            if guard > sched.max_deltas_guard() {
                                sched.mark_fatal();
                                return Step::Fatal;
                            }
                            continue;
                        }
                        // Suspending: the one place the cause must be OWNED.
                        sched.suspend_on(pi, *resume, cond.clone());
                    }
                    // `wait fork` (v8): park on the implicit child barrier, or
                    // fall through immediately when there are no live children.
                    WaitCause::Fork => {
                        if sched.exec_wait_fork(pi, *resume) {
                            bb = *resume; // no outstanding children → fall through
                            guard += 1;
                            if guard > sched.max_deltas_guard() {
                                sched.mark_fatal();
                                return Step::Fatal;
                            }
                            continue;
                        }
                        // parked by exec_wait_fork; on_child_complete resumes it.
                    }
                    _ => sched.suspend_on(pi, *resume, cond.clone()),
                }
                return Step::Suspended;
            }
            Terminator::Return => {
                sched.rearm(pi);
                return Step::Done;
            }
            // fork/join/join_any/join_none: register the barrier, spawn each child as
            // a new activity (runnable THIS instant), then either continue at
            // resume_bb (join_none, or zero children) or suspend on the barrier
            // (join/join_any with ≥1 child). The parent is re-enqueued by
            // on_child_complete when the join condition fires.
            Terminator::Fork {
                children,
                join,
                resume_bb,
            } => match sched.exec_fork(pi, children, *join, *resume_bb) {
                Some(cont) => {
                    bb = cont;
                }
                None => return Step::Suspended,
            },
            // B2 frame-call: a recursive/automatic TASK call. The sidecar carries
            // the positional arg↔formal binding (the frozen Terminator has none).
            // Evaluate inputs in THIS (caller) process scope, run the task frame
            // (&self, re-entrant), then write its outputs to the caller lvalues —
            // which here MAY be module nets (the &mut executor can write the flat
            // store). A non-frame Call (no sidecar entry) just advances.
            Terminator::Call { ret_bb, .. } => {
                if let Some(info) = sched.st.task_calls_proc.get(&(tmpl as u32, bb)).cloned() {
                    let cm = sched.st.func_table[info.callee as usize];
                    let in_v: Vec<(u32, Value)> = info
                        .in_binds
                        .iter()
                        .map(|&(slot, e)| {
                            let nv = &sched.st.ir.nets[(cm.base_net + slot) as usize];
                            let sw = sched.st.wt.get(e);
                            let v = sched.eval_ctx_top(e, nv.width.max(1).max(sw.width), nv.signed);
                            (slot, v)
                        })
                        .collect();
                    let out_s: Vec<u32> = info.out_binds.iter().map(|&(s, _)| s).collect();
                    if let Some(outs) = sched.st.run_task_call(info.callee, &in_v, &out_s) {
                        for ((_, lval), val) in info.out_binds.iter().zip(outs) {
                            let offs = sched.resolve_lvalue_offsets(lval);
                            sched.st.write_lvalue(lval, val, &offs);
                        }
                    }
                }
                bb = *ret_bb;
            }
        }

        guard += 1;
        if guard > sched.max_deltas_guard() {
            sched.mark_fatal();
            return Step::Fatal;
        }
    }
}

/// Resolved per-chunk `(bit-offset, array-word)` pairs for an lvalue write.
/// Inline up to 2 chunks — virtually every real lvalue — so the per-statement
/// READ phase does not allocate; a concat wider than 2 chunks spills to a Vec.
/// (The previous `Vec` return allocated once per executed assign, a top
/// malloc source of clock-bound designs.)
#[derive(Clone)]
pub(crate) enum Offsets {
    Inline {
        buf: [(u32, u32); 2],
        len: u8,
    },
    Heap(Vec<(u32, u32)>),
    /// v5 ⑤: a single-chunk assoc-element lvalue. Assoc keys are full SIGNED
    /// i64 domain (negative and beyond-u32 keys are legal), so they cannot
    /// ride the u32 pairs — the key resolves in the READ phase like every
    /// other offset and travels here. `None` = X/Z key (the write degrades
    /// loud + ignored at the funnel).
    AssocKey(Option<i64>),
    /// v6: the string-keyed twin of `AssocKey` (`NetKind::AssocStr`). The key
    /// is the raw byte string (leading-0x00-stripped packed ASCII); `None` =
    /// X/Z key, same degrade.
    AssocStrKey(Option<Vec<u8>>),
}

impl Offsets {
    pub(crate) fn as_slice(&self) -> &[(u32, u32)] {
        match self {
            Offsets::Inline { buf, len } => &buf[..*len as usize],
            Offsets::Heap(v) => v,
            Offsets::AssocKey(_) | Offsets::AssocStrKey(_) => &[],
        }
    }
}

/// The self-contained result of a statement's READ phase — everything the WRITE
/// phase needs, with no further reads of net state. Computing this is pure (reads
/// only, via `&Scheduler`); applying it is where all mutation happens. This is the
/// P7a boundary: a compiled body produces the same effects from native code, and
/// [`apply_effect`]'s kernel calls become the trait surface in P7b.
///
/// `'s` borrows from the (ir-owned) `Stmt`, so building an effect allocates
/// nothing for the lvalue/args themselves — only the NBA arm clones (its lvalue
/// must outlive the activation inside the scheduler's NBA queue).
enum StmtEffect<'s> {
    /// Blocking assign: RHS evaluated context-sized, per-chunk `(offset, word)`
    /// resolved NOW (dynamic-index sample at statement time).
    Blocking {
        lhs: &'s Lvalue,
        value: Value,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is a queue pop (v5 ④): the pop MUTATES the
    /// queue, so it runs in the WRITE phase (`k_queue_pop`), not the pure READ
    /// phase. The lvalue offsets still resolve in the READ phase — i.e. BEFORE
    /// the pop shrinks the queue (deterministic rule pinned in the design doc,
    /// the same family as the NBA apply-time bounds rule).
    QPop {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is an assoc-iteration call (v6): the call
    /// WRITES its ref key argument, so it runs in the WRITE phase — the same
    /// family as `QPop` (lvalue offsets still resolve in the READ phase).
    AssocIter {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is a SEEDED `$random(seed)` (v7): the draw
    /// writes the updated LCG state back into the seed variable — WRITE
    /// phase, same family as `QPop`/`AssocIter`.
    SeededRandom {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$dist_uniform(seed, start, end)` (v9, rank
    /// 6): the draw writes the updated seed back into the seed variable — WRITE
    /// phase, same family as `SeededRandom`.
    SeededDist {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `ok = $cast(dst, src)` (v9, rank 6): the
    /// cast writes the `dst` ref arg (resized `src`) and returns 1 — WRITE phase,
    /// same family as `$value$plusargs`.
    Cast {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$value$plusargs(fmt, var)` (v7): the
    /// match writes the ref var — WRITE phase, same family.
    ValuePlusargs {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$fopen(...)` (v7): opens mutate the
    /// engine file table — WRITE phase, same family.
    Fopen {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$sformatf(...)` (v7): rendering runs
    /// through the kernel-side format engine — WRITE phase, same family.
    Sformatf {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is a v9 file-READ int function ($fgetc/$feof/
    /// $ungetc): the read / pushback mutates the fd read state — WRITE phase,
    /// same family as `Fopen`. The int result writes to `lhs`.
    Fgetc {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    Feof {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    Ungetc {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$fgets(str, fd)` (v9): the read writes the
    /// str destination AND advances the fd — WRITE phase, the `$value$plusargs`
    /// family (the worker writes the dest internally; the byte count → `lhs`).
    Fgets {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$fread(target, fd[, start[, count]])` (v9):
    /// binary-reads into the target reg/memory AND advances the fd — WRITE
    /// phase, the `$value$plusargs` family (the byte count → `lhs`).
    Fread {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
    },
    /// Blocking assign whose rhs is `$fscanf`/`$sscanf` (v9): the scanf parser
    /// WRITES every matched ref arg AND (for `$fscanf`) advances the fd — WRITE
    /// phase, the `$value$plusargs` family (the conversion count → `lhs`). This
    /// is the first MULTI-ref-write intercept; the N writes loop inside the
    /// kernel worker, so the effect still rides the single `{lhs, rhs, offsets}`.
    Scanf {
        lhs: &'s Lvalue,
        rhs: u32,
        offsets: Offsets,
        is_file: bool,
    },
    /// Nonblocking assign: RHS SAMPLED now; the LHS index is sampled inside
    /// `schedule_nba` at schedule time (Active region), so it is NOT resolved here —
    /// preserving `a[i] <= x; i = i + 1;` using the old `i`.
    Nonblocking {
        lhs: &'s Lvalue,
        value: Value,
        /// `<= #d` transport delay in ticks, evaluated in the READ phase
        /// (v5). `None`/`Some(0)` both take the plain same-tick NBA path.
        delay_ticks: Option<u64>,
    },
    /// System task: a kernel call (its own read+write happen inside `dispatch`).
    /// `sid` keys the severity side table (P1-1).
    SysTask {
        which: SysTaskId,
        fmt: Option<u32>,
        args: &'s [u32],
        sid: u32,
    },
    /// `force lhs = value` (RHS sampled in the READ phase; `rhs` rides along
    /// so the kernel can register the IEEE §9.3.2 continuous re-evaluation).
    /// `sid` keys the assign-rank table (§9.3.1 proc-assign = weak rank).
    Force {
        lhs: &'s Lvalue,
        value: Value,
        rhs: u32,
        sid: u32,
    },
    /// `release lhs`. `sid` keys the assign-rank table (deassign vs release).
    Release { lhs: &'s Lvalue, sid: u32 },
    /// `disable fork` (P2-E): kills the caller's active descendants.
    DisableFork,
    /// N7: a `new` allocation site — the tagged blocking-assign allocates a fresh
    /// heap object of `class_id` (WRITE phase) and writes its id to `lhs` (the
    /// placeholder rhs is never evaluated). Same write-phase family as `QPop`.
    ClassNew {
        lhs: &'s Lvalue,
        class_id: u32,
        offsets: Offsets,
    },
    /// `disable <block>`: the Goto desugar did the control-flow work at
    /// elaborate; the statement itself is a marker no-op.
    Nop,
}

/// READ phase: evaluate `stmt` through the read-only half of the [`Kernel`] seam,
/// producing a [`StmtEffect`] that captures everything the write phase will apply. No
/// net state is mutated here. Generic over `K: Kernel`, so the SAME executor serves
/// the interpreter (`Scheduler`) and a Stage-C compiled body.
fn compute_effect<'s, K: Kernel>(k: &K, stmt: &'s Stmt, sid: u32) -> StmtEffect<'s> {
    match stmt {
        Stmt::BlockingAssign { lhs, rhs } => {
            // N7: a `new` allocation site — keyed by StmtId, so check it FIRST so
            // the placeholder rhs (a const 0) is never evaluated. The write phase
            // allocates the heap object and stores its id into `lhs`.
            if let Some(class_id) = k.k_class_new_site(sid) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::ClassNew {
                    lhs,
                    class_id,
                    offsets,
                };
            }
            // v5 ④: a queue-pop rhs is a statement-level EFFECT (it mutates
            // the queue) — defer the pop itself to the write phase.
            if k.k_queue_pop_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::QPop {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v6: an assoc-iteration rhs writes its ref key argument — same
            // statement-level deferral as the pops.
            if k.k_assoc_iter_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::AssocIter {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: a seeded $random(seed) rhs writes the seed back — same family.
            if k.k_random_seeded_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::SeededRandom {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v9 rank 6: a $dist_uniform(seed, ...) rhs writes the seed back.
            if k.k_dist_seeded_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::SeededDist {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v9 rank 6: a $cast(dst, src) func-form writes the dst ref arg.
            if k.k_cast_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Cast {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: $value$plusargs writes its ref var — same family.
            if k.k_value_plusargs_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::ValuePlusargs {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: $fopen mutates the file table — same family.
            if k.k_fopen_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fopen {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v7: $sformatf renders through the kernel — same family.
            if k.k_sformatf_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Sformatf {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            // v9 SYS-READ: $fgetc/$feof/$ungetc read/advance the fd read state —
            // same family. Resolve the lhs offsets in the READ phase but do NOT
            // evaluate the rhs (that read must happen in the WRITE phase, after
            // the dest offsets are pinned — the deterministic-ordering rule).
            if k.k_fgetc_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fgetc {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_feof_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Feof {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_ungetc_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Ungetc {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_fgets_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fgets {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_fread_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Fread {
                    lhs,
                    rhs: *rhs,
                    offsets,
                };
            }
            if k.k_fscanf_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Scanf {
                    lhs,
                    rhs: *rhs,
                    offsets,
                    is_file: true,
                };
            }
            if k.k_sscanf_rhs(*rhs) {
                let offsets = k.k_resolve_lvalue_offsets(lhs);
                return StmtEffect::Scanf {
                    lhs,
                    rhs: *rhs,
                    offsets,
                    is_file: false,
                };
            }
            let value = k.k_eval_for_lvalue(lhs, *rhs); // CONTEXT-SIZED to lhs width
            let offsets = k.k_resolve_lvalue_offsets(lhs); // dynamic index NOW
            StmtEffect::Blocking {
                lhs,
                value,
                offsets,
            }
        }
        Stmt::NonblockingAssign { lhs, rhs, delay } => {
            let value = k.k_eval_for_lvalue(lhs, *rhs); // CONTEXT-SIZED, sampled now
            let delay_ticks = delay.map(|d| k.k_delay_ticks(d));
            StmtEffect::Nonblocking {
                lhs,
                value,
                delay_ticks,
            }
        }
        Stmt::SysTask { which, fmt, args } => StmtEffect::SysTask {
            which: *which,
            fmt: *fmt,
            args,
            sid,
        },
        Stmt::Disable { scope_kind, .. } => match scope_kind {
            sim_ir::DisableKind::Fork => StmtEffect::DisableFork,
            sim_ir::DisableKind::Scope => StmtEffect::Nop,
        },
        Stmt::Force { lhs, rhs } => {
            // Evaluate NOW (context-sized to the target) for the initial pin;
            // the kernel registers `rhs` for continuous re-evaluation
            // (IEEE §9.3.2 — a force with an expression RHS behaves as a
            // continuous assignment until released).
            let value = k.k_eval_for_lvalue(lhs, *rhs);
            StmtEffect::Force {
                lhs,
                value,
                rhs: *rhs,
                sid,
            }
        }
        Stmt::Release { lhs } => StmtEffect::Release { lhs, sid },
    }
}

/// WRITE phase: apply a [`StmtEffect`] through the mutating half of the [`Kernel`]
/// seam. Returns `Some(Step)` only when a `$finish`/`$stop`/fatal system task ends the
/// activation. Generic over `K: Kernel` (same executor for interpreter + compiled VM).
fn apply_effect<K: Kernel>(k: &mut K, effect: StmtEffect<'_>) -> Option<Step> {
    match effect {
        StmtEffect::Blocking {
            lhs,
            value,
            offsets,
        } => {
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::QPop { lhs, rhs, offsets } => {
            let value = k.k_queue_pop(lhs, rhs); // pop + context-size (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::AssocIter { lhs, rhs, offsets } => {
            let value = k.k_assoc_iter(lhs, rhs); // key write + status (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::SeededRandom { lhs, rhs, offsets } => {
            let value = k.k_random_seeded(rhs); // seed write + draw (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::SeededDist { lhs, rhs, offsets } => {
            let value = k.k_dist_seeded(rhs); // seed write + dist draw (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Cast { lhs, rhs, offsets } => {
            let value = k.k_cast(rhs); // dst ref write + status (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::ValuePlusargs { lhs, rhs, offsets } => {
            let value = k.k_value_plusargs(rhs); // var write + status (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fopen { lhs, rhs, offsets } => {
            let value = k.k_fopen(rhs); // file-table mutation (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Sformatf { lhs, rhs, offsets } => {
            let value = k.k_sformatf(rhs); // kernel-side render (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fgetc { lhs, rhs, offsets } => {
            let value = k.k_fgetc(rhs); // byte read + fd advance (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Feof { lhs, rhs, offsets } => {
            let value = k.k_feof(rhs); // lazy-EOF flag read (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Ungetc { lhs, rhs, offsets } => {
            let value = k.k_ungetc(rhs); // pushback mutation (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fgets { lhs, rhs, offsets } => {
            let value = k.k_fgets(rhs); // line read + str dest write (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Fread { lhs, rhs, offsets } => {
            let value = k.k_fread(rhs); // binary read + target write (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Scanf {
            lhs,
            rhs,
            offsets,
            is_file,
        } => {
            // the parser writes every matched ref arg internally (WRITE phase);
            // the conversion count is written to lhs.
            let value = if is_file {
                k.k_fscanf(rhs)
            } else {
                k.k_sscanf(rhs)
            };
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::DisableFork => {
            k.k_disable_fork();
            None
        }
        StmtEffect::Nonblocking {
            lhs,
            value,
            delay_ticks,
        } => {
            // The NBA queue outlives this activation — the one owned clone left.
            match delay_ticks {
                Some(d) if d > 0 => k.k_schedule_nba_at(lhs.clone(), value, d),
                _ => k.k_schedule_nba(lhs.clone(), value),
            }
            None
        }
        StmtEffect::Force {
            lhs,
            value,
            rhs,
            sid,
        } => {
            k.k_force(lhs, value, rhs, sid);
            None
        }
        StmtEffect::Release { lhs, sid } => {
            k.k_release(lhs, sid);
            None
        }
        StmtEffect::SysTask {
            which,
            fmt,
            args,
            sid,
        } => match k.k_dispatch_systask(which, fmt, args, sid) {
            Ctl::Finish => Some(Step::Finish),
            Ctl::Stop => Some(Step::Stop),
            Ctl::Fatal => Some(Step::Fatal),
            Ctl::Continue => None,
        },
        StmtEffect::ClassNew {
            lhs,
            class_id,
            offsets,
        } => {
            let value = k.k_class_alloc(class_id); // allocate heap object (WRITE phase)
            k.k_write_lvalue(lhs, value, &offsets);
            None
        }
        StmtEffect::Nop => None,
    }
}
