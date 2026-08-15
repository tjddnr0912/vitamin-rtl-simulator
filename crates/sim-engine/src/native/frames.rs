//! S3a (doc-21 §5 S3) — **which subroutine designs tier-3 can take.**
//!
//! Until this slice `NetArena::buildable` refused every design with a non-empty
//! `func_table`, because a frame-local net's VALUE lives in the activation
//! window rather than in a net slot and both the arena's read path and its write
//! funnel are frame-blind. S3's job is to lift that, and this file is the first
//! half: the predicate that says which frames can be lifted TODAY, and why the
//! lift is byte-identical for exactly those.
//!
//! ## The argument
//!
//! Tier-3 does not restate the frame executor. `NetReader::eval_call` on the
//! tier-3 kernel delegates to the ENGINE's `SimState::run_frame_call` — the same
//! function, the same window/slab, the same `MAX_CALL_DEPTH`, the same
//! `call_fatal`/`had_error` channels. That is sound because the frame window is
//! NOT part of the flat net store: `frame_stack`/`static_store` are their own
//! storage, and a native run leaves them the only live copy of a frame slot
//! exactly as an engine run does.
//!
//! What is NOT shared is the MODULE net store. `run_frame_call` reads module
//! nets through `SimState` (the flat store), which a native run never writes —
//! so a subroutine that reads one would read the t0 value at exit 0: a silent
//! wrong value, the class this repository exists to refuse. The predicate below
//! is precisely the property that makes that unreachable — **a body that
//! references no net outside its own frame window never consults the module
//! store** — so the delegation is byte-identical by construction rather than by
//! measurement, and widening the class (a frame body that READS a module net) is
//! a later slice that has to thread a reader, not a gate edit.
//!
//! ## What each row is for
//!
//! Most refusals below are a shape the delegation would get WRONG; two are
//! DEFENCE, and the difference is stated rather than blurred (the first draft of
//! this list said "every refusal is a shape that would get wrong", and the
//! adversarial review measured two that cannot fire):
//!
//! * a TASK is entered through `Terminator::Call`, whose copy-out writes the
//!   CALLER's lvalues and whose body may suspend — neither exists in the tier-3
//!   walk (`body_is_walkable` refuses the terminator, and this row is what keeps
//!   the two gates from disagreeing about the same design);
//! * a statement other than a blocking assign or `$display`/`$write` is either
//!   dropped by `run_frame_call`'s catch-all or is a heap/effect family the
//!   design gate already counts — refusing it here is what stops a future
//!   elaborate change from turning "rejected upstream" into "silently skipped";
//! * a MODULE body that names a frame-local net would hand a frame slot to the
//!   arena's read path, to `wprog`'s compile-time slot resolution, to
//!   `fast_offsets` and to `write_lvalue` — all four are frame-blind by design.
//!   Measured zero on every design that reaches here, and stated as a row so it
//!   stays zero;
//! * **`contains_shared_fork` is the last of the two flag rows.** Its twin
//!   `has_hier_call` was called DEAD here, then LIVE by A3-ii-a (19 designs hit
//!   only it), and is now GONE — A3-iv measured that its reason was about
//!   elaborate's phase order, not the engine's, and that `frame_suspends`
//!   already answers the question it stood in for. `contains_shared_fork` means
//!   the body has a `fork`, which `frame_suspends` reports as a park one row up;
//!   it is kept as its own row so that if that ever stops being true, this
//!   refuses instead of the design going quiet.
//!
//! ## What this file does NOT close, and who does
//!
//! `Process.sensitivity` carries net ids the walk below never reads, and that is
//! closed by a property of elaborate rather than by this file: an implicit `@*`
//! read set descends into a call's ARGUMENTS only, never the callee body, so a
//! frame-local net cannot enter a sensitivity list without also appearing in the
//! process body — which IS walked. Stated because "the walk did not need to look"
//! and "the walk forgot to look" read the same in a diff.
//!
//! `task_calls_proc` / `task_calls_func` carry net ids OUTSIDE the `Stmt`/`Expr`
//! arenas — a call site's `out_binds` are caller `Lvalue`s, and elaborate's own
//! doc says they may target module nets. The walk below never sees them. They are
//! closed by two OTHER layers: `task_calls_proc` is keyed by a `Terminator::Call`
//! block, which `body_is_walkable` refuses, and `task_calls_func` only exists in
//! task bodies, which the `is_task` row refuses. Saying so here is the point — an
//! unstated closure is how a later widening loses it. Note the two report
//! DIFFERENTLY, measured: a `task_calls_proc` design (a function with output
//! formals) is `buildable: true` and refused by the executor layer, while a
//! `task_calls_func` design is `buildable: false` on the `is_task` row.

use sim_ir::SimIr;

use crate::SimOpts;

/// Can tier-3 run this design's subroutines? `Ok(())` also covers the common
/// case of a design with none (first line — no scan, so a design without
/// subroutines pays nothing).
///
/// `Err` names ONE row; like the eligibility map's `refused`, removing that
/// feature can expose the next.
pub(crate) fn frames_admitted(ir: &SimIr, opts: &SimOpts) -> Result<(), &'static str> {
    if opts.func_table.is_empty() {
        return Ok(());
    }
    // The engine latches a run-fatal on a misaligned table (`build_func_routing`).
    // Tier-3 refuses instead of racing it: an out-of-range window would index the
    // very tables this predicate reads.
    if opts.func_table.len() != ir.funcs.len() {
        return Err("malformed frame sidecar (func_table length)");
    }
    let susp = suspendable_set(ir, opts);
    let mut w = Walk::new(ir);
    for (fi, m) in opts.func_table.iter().enumerate() {
        let fd = ir.funcs[fi];
        if (m.base_net as usize) + (m.locals_len as usize) > ir.nets.len() {
            return Err("malformed frame sidecar (frame window out of range)");
        }
        // The THIRD thing `build_func_routing` fatals on. Checking two of its
        // three conditions and calling that "refuses instead of racing it" was
        // the gap: on this one input the engine's routing tables stay all-false
        // while the arena gets built, which is exactly the race.
        if !fd.is_task && m.locals_len > 0 && m.return_slot >= m.locals_len {
            return Err("malformed frame sidecar (return slot out of range)");
        }
        // A3-i NARROWED THIS ROW. It used to refuse EVERY task, because a task is
        // entered through `Terminator::Call` and the tier-3 walk had no arm for
        // one. It has one now — for the SUBSET half only, the callees
        // `sim_ir::compute_suspendable_tasks` leaves out — so what is refused here
        // is the other half: a task that suspends, forks, prints, or writes
        // outside its own frame window needs the engine's call-stack machinery
        // (`enter_task_frame`, a `FrameRec` per activation, park/resume), which
        // tier-3 does not model at all.
        //
        // The set is computed from the SAME inputs the engine uses (`suspendable_set`
        // is the one spelling), so the gate and `run_process` cannot disagree about
        // which half a task is in.
        //
        // ⚠️ Scoped to `is_task` ON PURPOSE, and that is a ladder question rather
        // than a taste one. A FUNCTION with a `$display` in its body is
        // "suspendable" by the same predicate, and such a design is admitted TODAY
        // (it is reached through `Expr::Call` → `run_frame_call`, never through a
        // `Terminator::Call`). Asking the suspendable question of every func would
        // therefore turn working designs loud — a regression, not a tightening.
        //
        // ⚠️⚠️ **A3-ii-a narrowed it again, and the census is why.** "Suspendable"
        // is the name of the EXECUTOR the engine picks, not a claim that the body
        // suspends: `stmt_signal` counts a `$display`, an NBA, or a write outside
        // the frame as a signal, because the synchronous `&self` executor cannot
        // perform those — not because the body parks. Measured over the whole
        // suite: of 357 designs whose process-body callee is "suspendable",
        // **250 contain no `Delay`/`Wait`/`Fork` at all**. For those the whole
        // ⚠️ A3-ii-b NARROWED THIS ROW, and the sentence it replaced said the
        // quiet part: "tier-3 needs a frame stack and a CFG loop but NO
        // park/resume, no window stash and no scheduler state. That is the half
        // admitted here." This slice built the other half — the walk hands its
        // stack to the kernel at a `Delay`/`Wait` and takes it back on resume,
        // and the window stash is the ENGINE's (`frame_window::stash_windows_in`,
        // extracted so there is one spelling). What is left is `fork` inside a
        // frame, which needs the activity arena tier-3 does not have.
        //
        // Measured before narrowing: of the parking frames the suite reaches,
        // 65 park on a `Wait` edge, 19 on a `Delay`, 13 on a `fork` — and every
        // design in that last group also trips the `fork` DESIGN row, so this
        // row's remaining population is zero-cost today.
        if fd.is_task
            && susp.contains(&(fi as u32))
            && crate::exec::frame_call::frame_forks(ir, &susp, fd.entry)
        {
            return Err("a task frame that FORKS (a `fork` inside the body): S3b");
        }
        // ⚠️⚠️ **`has_hier_call` IS LIVE NOW, and the claim that it was not is one
        // this file made twice.** S3a called both rows dead because the `is_task`
        // row got there first; A3-i repeated it with a new argument — "both flags
        // imply suspendable, and suspendable is refused above". A3-ii-a admits
        // suspendable-but-non-parking frames, so that argument expired with it:
        // measured over the whole suite, this row now fires on **19 designs**,
        // and it is the only thing refusing them.
        //
        // Which is exactly what it is for. `has_hier_call` marks a frame task with
        // a DEFERRED hierarchical enable, whose `Call.target` is a placeholder
        // until the finish-phase resolve — so `frame_suspends` cannot see through
        // it to decide whether the callee parks, and `force_suspend` exists
        // precisely because the two computes would otherwise disagree. Refusing is
        // the fail-closed answer, and lifting it is A3-ii's business, not a
        // comment's.
        //
        // `contains_shared_fork` really is still dead: it means the body has a
        // `fork`, which `frame_suspends` reports as a park one row up. Kept as its
        // own row for the reason it always was — if that ever stops being true,
        // this refuses instead of the design going quiet.
        // ⚠️⚠️ **`has_hier_call` IS GONE (A3-iv), and its reason was about a phase
        // that has already ended.** The row said `frame_suspends` cannot see
        // through a deferred hierarchical enable, because `Call.target` is a
        // PLACEHOLDER until the finish-phase resolve — and that is true where
        // `force_suspend` needs it, inside ELABORATE, whose
        // `compute_suspendable_tasks` runs before the patch. This predicate runs
        // in `simulate`, after it. Measured rather than reasoned: instrumenting
        // the walk over every design that reaches this row counts **zero**
        // unresolvable `Call.target`s.
        //
        // So the question the row was standing in for is answerable, and the row
        // above already asks it — `frame_suspends` refuses a hier callee that
        // parks (measured: a `#1` inside the callee's body is refused as "a task
        // frame that SUSPENDS"), and its `None` arm still fails CLOSED if a
        // target ever does arrive unresolved. Refusing here as well was
        // over-refusal, which is a rung DOWN the ladder.
        //
        // §4.5.338 for the third time in this file: a refusal does not know when
        // its own reason stops being true — and "the target is a placeholder" is
        // a claim about WHEN, which the next phase invalidates.
        if m.contains_shared_fork {
            return Err("a subroutine with a shared fork window");
        }
        // ⭐⭐ **TWO PRECONDITIONS, because there are two executors.** Which one a
        // body must satisfy is decided by WHO RUNS IT, and getting this wrong in
        // either direction is a defect rather than a preference:
        //
        //  * a DELEGATED body — a plain function through `Expr::Call` →
        //    `run_frame_call`, or a synchronous task through `run_task_call` —
        //    runs inside `SimState`'s own `&self` executor, which reads the
        //    ENGINE's flat store. So it must name no net outside its window, or it
        //    reads the t0 value at exit 0. That is S3a's argument, unchanged.
        //
        //  * a DRIVEN body (A3-ii-a) runs through `compute_effect`/`apply_effect`
        //    on THIS kernel. Every read is `k_read_net` and every write is
        //    `k_write_lvalue`, both of which route — so naming a module net is
        //    CORRECT here, and requiring otherwise would refuse the very designs
        //    this slice is for. Measured: applying the delegated precondition to a
        //    driven body left `task automatic show(...); $display(...); g = g + x;`
        //    refused, and an end-to-end check of it silently compared the VM
        //    against itself.
        //
        // Only a TASK can be driven: a task has no return value, so nothing can
        // reach it through `Expr::Call`, and `Terminator::Call` is the only
        // entry. A function — including one with output formals — keeps the
        // delegated precondition, because an expression call to it would take the
        // `run_frame_call` path this walk never sees.
        if fd.is_task && susp.contains(&(fi as u32)) {
            driven_body_is_runnable(ir, opts, &susp, fd.entry, m.base_net, m.locals_len)?;
        } else {
            w.body_is_task = fd.is_task;
            body_stays_in_its_window(&mut w, fd.entry, m.base_net, m.locals_len)?;
        }
    }
    // The frame-blindness of the tier-3 store, stated rather than assumed…
    w.restart();
    for p in &ir.processes {
        for blk in &p.body {
            w.block(blk);
        }
    }
    // ⚠️ `ContAssign.delay` is NOT an ExprId — unlike `Stmt::NonblockingAssign`'s,
    // which is. Elaborate FOLDS a continuous assign's delay to a tick count
    // (`fold_ca_delay`), so the two same-named `Option<u32>` fields live in
    // different spaces. Walking it as an expression read a random node of the
    // arena; measured, on `assign #1 y = f(a);` that node was the FUNCTION's
    // `~x`, and this scan refused the design for naming a frame-local net.
    // Over-refusal rather than a wrong answer — but the next reader of this
    // loop gets to know why there is no `delay` line here.
    for ca in &ir.cont_assigns {
        w.lvalue(&ca.lhs);
        w.expr(ca.rhs);
    }
    for m in opts.func_table.iter() {
        if w.nets
            .iter()
            .any(|&n| n >= m.base_net && n < m.base_net + m.locals_len)
        {
            return Err("a module body that names a frame-local net");
        }
    }
    // …and the TWO module positions whose reader is the bare arena.
    //
    // A call is answered by `NetReader::eval_call`, so it is correct exactly
    // where the evaluation runs through the tier-3 kernel's COMPOSITE reader.
    // Measured, seam by seam, that is: an assignment rhs and its lvalue index
    // expressions (`k_eval_for_lvalue` / `k_resolve_lvalue_offsets`), a branch
    // condition, a `wait(e)` predicate and a delay amount (`k_truthy` /
    // `k_delay_ticks`), and a zero-delay continuous assign — including the
    // multi-driver fold, which evaluates every driver through the same funnel.
    //
    // ⭐ ONE is left, and the other was closed by a slice that was not about
    // calls at all. `k_dispatch_systask` does hand `dispatch` the arena alone —
    // it holds `&mut Scheduler`, so it cannot also lend `&SimState` to a
    // composite — but V1 slice 2 put a composite one level DOWN for a different
    // reason: `SimState::eval_expr_with` wraps the reader in `HeapRouted` to
    // route heap nets, and that wrapper holds BOTH stores. Answering the call
    // family from its `st` half is what makes `$display("%0d", f(x))` native, so
    // the row that used to sit here is gone.
    //
    // Still refused: `schedule_delayed_cas` evaluates a DELAYED assign's rhs and
    // lvalue offsets through the reader it is given, and that path does not go
    // through `eval_expr_with`.
    //
    // `NetArena::eval_call` panics rather than X-poisoning, so a seam this
    // enumeration MISSED is loud in the gate rather than a wrong value in a
    // run; these rows are what keep the two known ones from reaching it.
    let mut c = Walk::new(ir);
    for ca in &ir.cont_assigns {
        if ca.delay.is_none() {
            continue; // the zero-delay settle evaluates through `k_eval_for_lvalue`
        }
        // The delay itself is a folded tick count (see above), so only the rhs
        // and the lvalue's index expressions can carry a call.
        let idx = ca
            .lhs
            .chunks
            .iter()
            .flat_map(|k| [k.word, k.offset, k.width])
            .flatten();
        if std::iter::once(ca.rhs).chain(idx).any(|e| c.has_call(e)) {
            return Err("a call in a delayed continuous assign: S3b");
        }
    }
    Ok(())
}

/// The precondition for a DRIVEN frame body (A3-ii-a) — the one the tier-3 walk
/// executes itself.
///
/// It is deliberately NOT `body_stays_in_its_window`: a driven body's reads and
/// writes go through the kernel, so naming a module net is exactly what works.
/// What it must not contain is anything the WALK cannot run:
///
///  * a system task `k_dispatch_systask` refuses — the same question
///    `body_dispatch_ok` asks of a process body, asked of the frame arena. A
///    `$dumpvars` inside a task would panic mid-run rather than answer;
///  * a NONBLOCKING assign to a FRAME-LOCAL net. The update is filed now and
///    applied at the delta boundary, by which time this activation's window is
///    gone — the write would land in whatever window is live then. An NBA to a
///    MODULE net is fine and common (`n <= m` in a task), because its destination
///    outlives the frame.
///
///    ⚠️ **UNREACHABLE today, measured.** A mutation deleting this clause survives
///    the suite, because elaborate refuses the shape outright (E3009) — as does
///    iverilog, quoting IEEE §10.4.2: "automatically allocated variables may not
///    be assigned values using non-blocking assignments". The clause stays as a
///    backstop on a rule this file does not own; it is recorded as unreachable
///    rather than counted as covered;
///  * a nested call whose own callee is not admitted, which `frames_admitted`
///    checks per func anyway — what is checked HERE is that the site has a
///    `task_calls_func` entry, since a miss means the call silently does not
///    happen.
///
/// The parking terminators are NOT re-checked here: `frame_suspends` above owns
/// that question and names its own row.
fn driven_body_is_runnable(
    ir: &SimIr,
    opts: &SimOpts,
    susp: &std::collections::BTreeSet<u32>,
    entry: u32,
    base_net: u32,
    locals_len: u32,
) -> Result<(), &'static str> {
    let (lo, hi) = (base_net, base_net.saturating_add(locals_len));
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![entry];
    while let Some(b) = stack.pop() {
        if !seen.insert(b) {
            continue;
        }
        let Some(blk) = ir.blocks.get(b as usize) else {
            return Err("malformed frame sidecar (block id out of range)");
        };
        for &sid in &blk.stmts {
            match &ir.stmts[sid as usize] {
                sim_ir::Stmt::SysTask { which, .. } => {
                    if crate::native::kernel::systask_refusal(*which).is_some() {
                        return Err("a system task the tier-3 kernel refuses, inside a task frame");
                    }
                }
                sim_ir::Stmt::NonblockingAssign { lhs, .. } => {
                    if lhs.chunks.iter().any(|c| c.net >= lo && c.net < hi) {
                        return Err("a nonblocking assign to a frame-local net: S3b");
                    }
                }
                _ => {}
            }
        }
        match &blk.term {
            sim_ir::Terminator::Goto { target } => stack.push(*target),
            sim_ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                stack.push(*then_bb);
                stack.push(*else_bb);
            }
            sim_ir::Terminator::Call { target, ret_bb } => {
                // The NESTED table, keyed by this block's GLOBAL id. A miss is a
                // deferred hierarchical enable whose actuals are unresolved; the
                // engine advances past it, which would make the call vanish.
                if !opts.task_calls_func.contains_key(&b) {
                    return Err("a nested call with no sidecar entry: S3b");
                }
                // Its callee's own body is checked by this loop's caller (every
                // func in the table is visited), so only reachability is added
                // here — and the callee must itself be one of the two admitted
                // kinds rather than a parking frame.
                if let Some(cf) = ir.funcs.iter().position(|f| f.entry == *target) {
                    if susp.contains(&(cf as u32))
                        && crate::exec::frame_call::frame_forks(ir, susp, *target)
                    {
                        return Err("a task frame that FORKS (a `fork` inside the body): S3b");
                    }
                } else {
                    return Err("a nested call to an unresolved target: S3b");
                }
                stack.push(*ret_bb);
            }
            sim_ir::Terminator::Return => {}
            // A3-ii-b: the walk PARKS on these two, so the body reaching one is
            // no longer a disagreement — follow the resume edge and keep
            // checking, exactly as the `frame_park_kinds` walk does.
            sim_ir::Terminator::Delay { resume, .. } | sim_ir::Terminator::Wait { resume, .. } => {
                stack.push(*resume)
            }
            // `frame_forks` already refused this with its own row; reaching one
            // here means the two walks disagree, which is worth a row of its own
            // rather than a silent skip.
            sim_ir::Terminator::Fork { .. } => {
                return Err("a task frame that FORKS (a `fork` inside the body): S3b")
            }
        }
    }
    Ok(())
}

/// Which subroutines does the engine drive through its CALL-STACK path rather
/// than the synchronous `&self` frame executor?
///
/// ⭐ **One spelling, reassembled from the same fields `simulate` uses.** The
/// engine builds this set in `lib.rs` right before the run, from `func_table`'s
/// `base_net` / `has_hier_call` and the `task_calls_func` copy-out nets. Asking
/// the same pure function over the same inputs is what makes "tier-3 admits
/// exactly the callees `run_process` will run synchronously" true by
/// construction; a predicate of our own here would be a second classifier, and
/// the two disagreeing means the walk delegates a body the engine parks.
///
/// Called once per gate evaluation. `Ok`-path cost is one pass over the func
/// arena, which the caller is already making.
pub(crate) fn suspendable_set(ir: &SimIr, opts: &SimOpts) -> std::collections::BTreeSet<u32> {
    let base_nets: Vec<u32> = opts.func_table.iter().map(|m| m.base_net).collect();
    let force_suspend: Vec<bool> = opts.func_table.iter().map(|m| m.has_hier_call).collect();
    let call_out_nets = sim_ir::call_out_nets(
        opts.task_calls_func
            .iter()
            .map(|(b, info)| (*b, info.out_binds.as_slice())),
    );
    sim_ir::compute_suspendable_tasks(
        &ir.funcs,
        &ir.blocks,
        &ir.stmts,
        &ir.exprs,
        &base_nets,
        &force_suspend,
        &call_out_nets,
    )
}

/// Can the tier-3 walk run the `Terminator::Call` at process-local block `bb` of
/// process `proc`?
///
/// ONE spelling for two callers — `body_is_walkable` (the gate) and the walk's
/// own `Call` arm (the `debug_assert`) — because the gate saying yes and the arm
/// assuming something else is exactly how a refused shape reaches an executor
/// that cannot run it.
///
/// Two conditions, and neither is redundant:
///  * the call SITE must be in `task_calls_proc`. A missing entry is a deferred
///    hierarchical enable whose actuals elaborate could not resolve; the engine
///    treats it as "advance past the call", which would silently skip the call
///    rather than perform it.
///  * the CALLEE must not PARK. A3-i admitted only the synchronous half; A3-ii-a
///    added the frames that are "suspendable" (the engine's executor choice) but
///    reach no `Delay`/`Wait`/`Fork`, because those run to `Return` inside one
///    `run_body` call. What still refuses is a body that really parks:
///    `run_process` resumes it from `activities[pi].call_stack` with the window
///    stashed, and tier-3 has no such state.
///
/// ⚠️ The two admitted halves take DIFFERENT paths in the walk — a synchronous
/// callee is delegated whole to `SimState::run_task_call`, a non-parking
/// suspendable one is DRIVEN by the walk itself — so the arm has to make the same
/// split this predicate does. `callee_mode` is that split, and it is why this
/// returns a mode rather than a bool.
///
/// What is NOT asked here is whether the callee's BODY stays inside its own frame
/// window — `frames_admitted` already refused the design otherwise, for every
/// func in the table, and it runs first (`runtime_gate` = design ∧ storage, then
/// this layer). Stated rather than assumed: an unstated closure is how a later
/// widening loses it.
pub(crate) fn call_site_runnable(
    ir: &SimIr,
    opts: &SimOpts,
    susp: &std::collections::BTreeSet<u32>,
    proc: u32,
    bb: u32,
) -> bool {
    callee_mode(ir, opts, susp, proc, bb).is_some()
}

/// HOW the walk must run the call at `(proc, bb)`, or `None` when it must not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalleeMode {
    /// A3-i: `compute_suspendable_tasks` leaves it out ⇒ the engine's `&self`
    /// executor runs it ⇒ tier-3 delegates the whole call.
    Synchronous,
    /// A3-ii-a: the engine would drive it from a `FrameRec`, but its body reaches
    /// no suspending terminator ⇒ tier-3 drives the CFG itself, to `Return`,
    /// without leaving this activation.
    DrivenFrame,
}

pub(crate) fn callee_mode(
    ir: &SimIr,
    opts: &SimOpts,
    susp: &std::collections::BTreeSet<u32>,
    proc: u32,
    bb: u32,
) -> Option<CalleeMode> {
    let info = opts.task_calls_proc.get(&(proc, bb))?;
    if !crate::exec::frame_call::site_runnable(ir, susp, Some(info)) {
        return None;
    }
    Some(if susp.contains(&info.callee) {
        CalleeMode::DrivenFrame
    } else {
        CalleeMode::Synchronous
    })
}

/// The delegation precondition for ONE subroutine, as its own function: does its
/// body use only what the engine's `&self` frame executor runs, and does it name
/// only nets inside its own window `[base, base+len)`?
///
/// Named rather than inlined because the two questions are ONE precondition and
/// a caller that asked only the first would delegate a body that reads the module
/// store. `Err` is the row.
fn body_stays_in_its_window(
    w: &mut Walk<'_>,
    entry: u32,
    base_net: u32,
    locals_len: u32,
) -> Result<(), &'static str> {
    w.restart();
    w.func_body(entry)?;
    // ⭐⭐ **A3-iii NARROWED THIS FROM `names` TO `writes`.** S3a's argument was
    // that this `&self` executor reads module nets from `SimState`, which a
    // native run never writes — so an admitted body naming one would read the t0
    // value at exit 0. That is now false for READS: `run_frame_call_with` takes
    // the caller's store and `HeapRouted` splits it, sending a frame slot back to
    // the activation window and a module net to the arena.
    //
    // ⚠️ It stays true for WRITES, and not for want of threading: every
    // destination in that body goes through `SimState::frame_write_lvalue`, which
    // is `&self` on this state and has no way to reach a caller's arena. A body
    // that assigns a module net would land in the dead store, silently.
    //
    // Measured before narrowing rather than after: of the 26 designs this row
    // blocked, **22 only READ out of window and 4 also write**. So the row keeps
    // its place and loses most of its population — and it is re-worded in terms
    // of what it actually refuses, which is the §4.5.338 discipline.
    if w.wnets
        .iter()
        .any(|&n| n < base_net || n >= base_net + locals_len)
    {
        return Err("a subroutine that WRITES a net outside its own frame: S3b");
    }
    Ok(())
}

/// One reachability walk: which nets does a region of the IR name?
///
/// The three stamp buffers are sized to the ARENAS and reused across walks by
/// bumping a generation, not by clearing — the IR has back-edges (a loop's
/// `Goto` re-enters a visited block) and shares expression nodes, so a walk
/// without a visit stamp does not terminate, and one that re-allocates per
/// question is quadratic in the design.
struct Walk<'i> {
    ir: &'i SimIr,
    /// Generation of the current walk; 0 means "never visited".
    gen: u32,
    estamp: Vec<u32>,
    bstamp: Vec<u32>,
    nstamp: Vec<u32>,
    /// The nets this walk named, deduplicated by `nstamp`.
    nets: Vec<u32>,
    /// A3-iii: the subset of `nets` that appear as an lvalue's DESTINATION —
    /// `chunk.net`, not its index expressions. Kept apart because reads and
    /// writes now have different answers: a read routes through the caller's
    /// store, a write cannot.
    wnets: Vec<u32>,
    /// Did this walk descend through an `Expr::Call`? Reset by `restart`.
    saw_call: bool,
    /// Is the body currently being walked a TASK's?
    ///
    /// Set by the caller, because the answer decides which of the two delegated
    /// executors runs it — and they differ on exactly one terminator. See the
    /// `Call` arm in `func_body`.
    body_is_task: bool,
    /// Block-id worklist (`func_body`) and expression worklist (`expr`), kept
    /// apart because the two interleave: a `Branch` walks its condition while
    /// block ids are still pending.
    stack: Vec<u32>,
    estack: Vec<u32>,
}

impl<'i> Walk<'i> {
    fn new(ir: &'i SimIr) -> Self {
        Walk {
            ir,
            gen: 0,
            estamp: vec![0; ir.exprs.len()],
            bstamp: vec![0; ir.blocks.len()],
            nstamp: vec![0; ir.nets.len()],
            nets: Vec::new(),
            wnets: Vec::new(),
            saw_call: false,
            body_is_task: false,
            stack: Vec::new(),
            estack: Vec::new(),
        }
    }

    fn restart(&mut self) {
        self.gen += 1;
        self.nets.clear();
        self.wnets.clear();
        self.saw_call = false;
        self.stack.clear();
        self.estack.clear();
    }

    /// Does this expression tree contain a user subroutine call? A fresh walk
    /// each time (the stamps make repetition cheap, not free — the caller asks
    /// once per candidate position, not once per node).
    fn has_call(&mut self, eid: u32) -> bool {
        self.restart();
        self.expr(eid);
        self.saw_call
    }

    fn add(&mut self, net: u32) {
        match self.nstamp.get_mut(net as usize) {
            Some(s) if *s != self.gen => {
                *s = self.gen;
                self.nets.push(net);
            }
            // An out-of-range net id cannot come from elaborate; it is recorded
            // rather than indexed so nothing panics on a malformed artifact.
            //
            // ⚠️ It makes the FUNCTION caller refuse (its test is "outside my
            // window", which an out-of-range id satisfies) but NOT the module
            // one (whose test is "inside some window", which it does not) — an
            // earlier version of this note claimed both. Only reachable from a
            // corrupt sidecar, and the window-range and return-slot checks above
            // catch the shapes that produce one.
            None => self.nets.push(net),
            _ => {}
        }
    }

    /// Does this FUNCTION body use only what `run_frame_call` executes, and what
    /// nets does it name? A whole-body reachability walk from `entry`, for the
    /// reason `body_is_walkable` gives: a statement behind two `Goto`s runs just
    /// as much as one in the entry block. Blocks unreachable from `entry` are
    /// deliberately not scanned — the same rule the executor itself follows.
    fn func_body(&mut self, entry: u32) -> Result<(), &'static str> {
        self.stack.push(entry);
        while let Some(bb) = self.stack.pop() {
            let Some(blk) = self.ir.blocks.get(bb as usize) else {
                return Err("malformed frame sidecar (block id out of range)");
            };
            match self.bstamp.get_mut(bb as usize) {
                Some(s) if *s != self.gen => *s = self.gen,
                _ => continue,
            }
            for &sid in &blk.stmts {
                match &self.ir.stmts[sid as usize] {
                    sim_ir::Stmt::BlockingAssign { .. } => {}
                    // `run_frame_call`'s Family-D arm renders these through the
                    // shared formatter; a severity (`$error`/`$fatal`) is the
                    // same `Display` id plus a sidecar and reaches the same
                    // reader and the same `call_fatal`/`had_error` latch.
                    sim_ir::Stmt::SysTask {
                        which: sim_ir::SysTaskId::Display | sim_ir::SysTaskId::Write,
                        ..
                    } => {}
                    // Slice #3, and the census is why these three are here and
                    // nothing else is. Instrumented over every design this row
                    // was blocking: **13 are `new[]`** and **2 are `disable`**;
                    // no other statement kind reaches it at all. All three are
                    // arms `run_frame_call` ALREADY EXECUTES, so the row was
                    // conservative about them — with one thing genuinely wrong
                    // underneath, see `frame_dyn_new_with`.
                    //
                    // ⚠️ `DynDelete` has ZERO corpus designs. It is admitted
                    // anyway because it is the same executor arm and the same
                    // family as `DynNew` — refusing half a pair would leave a
                    // row whose stated reason is false — and that is recorded
                    // rather than counted as a gain.
                    sim_ir::Stmt::SysTask {
                        which: sim_ir::SysTaskId::DynNew | sim_ir::SysTaskId::DynDelete,
                        ..
                    } => {}
                    // A plain `disable <named block>` is the break/continue
                    // idiom: elaborate lowers it as a diagnostic-shaped marker
                    // plus a sibling `Goto` that does the control flow, so the
                    // executor's `_ => {}` is the CORRECT execution of the
                    // marker, not a drop. `DisableKind::Fork` stays refused —
                    // a function body cannot fork (elaborate's B1 cut), so this
                    // arm is belt and braces rather than a live distinction.
                    sim_ir::Stmt::Disable {
                        scope_kind: sim_ir::DisableKind::Scope,
                        ..
                    } => {}
                    _ => return Err("a subroutine statement the frame executor drops"),
                }
                self.stmt(sid);
            }
            match &blk.term {
                sim_ir::Terminator::Goto { target } => self.stack.push(*target),
                sim_ir::Terminator::Branch {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    let (t, e) = (*then_bb, *else_bb);
                    self.expr(*cond);
                    self.stack.push(t);
                    self.stack.push(e);
                }
                sim_ir::Terminator::Return => {}
                // `run_frame_call` `break`s defensively on all four (elaborate's
                // B1 cut rejects them in a function body), which would SILENTLY
                // end the call there. Refusing keeps the arena out of any design
                // where that defensive break could run.
                // ⭐ **Slice #7 SPLIT THIS ARM, and the census is the whole
                // argument.** Instrumented over every design the row blocked:
                // **all 7 are a nested `Terminator::Call`, and all 7 are in a
                // TASK body**; `Delay`/`Wait`/`Fork` are zero, because
                // elaborate's B1 cut rejects timing control in a subroutine
                // (measured directly — both spellings of `disable fork` in a
                // function body are E3009).
                //
                // The two delegated executors answer `Call` differently, and
                // only ONE of them is what this walk was written for:
                //
                //  * `run_frame_call` (a plain function through `Expr::Call`)
                //    has no `Call` arm and `break`s — the nested call would be
                //    silently skipped and the function would return early.
                //  * `run_task_with` (a synchronous task, and A3-i's subset
                //    path) RECURSES into it, binds the formals and copies the
                //    outputs back — and A3-iii threaded the caller's store
                //    through it.
                //
                // So the refusal is kept for a function and lifted for a task.
                //
                // ⚠️ And the kept half is UNREACHABLE, measured rather than
                // assumed: the only way a function body acquires a
                // `Terminator::Call` is a nested call to a subroutine with an
                // output formal, and elaborate refuses exactly that — its E3009
                // names this case in so many words ("any position inside a
                // FUNCTION body lowered as a call frame … has no call statement
                // of its own to carry the copy-out — the same call in a TASK
                // body, or in a module process, does work"). iverilog rejects
                // the same source. The arm stays because it is fail-closed and
                // because the reason it encodes is about `run_frame_call`, not
                // about elaborate — if that executor ever grows a `Call` arm,
                // this is the line that has to move.
                // A nested callee that SUSPENDS is not this row's problem: it is
                // refused by its OWN frame's `frame_suspends` row, which
                // `frames_admitted` asks of every subroutine.
                sim_ir::Terminator::Call { .. } if self.body_is_task => {}
                sim_ir::Terminator::Delay { .. }
                | sim_ir::Terminator::Wait { .. }
                | sim_ir::Terminator::Fork { .. }
                | sim_ir::Terminator::Call { .. } => {
                    return Err("a subroutine body that suspends, forks or calls a task")
                }
            }
        }
        Ok(())
    }

    /// Every net ONE module basic block names — statements plus whatever the
    /// terminator samples. Exhaustive over `Terminator` for the same reason
    /// `expr` is exhaustive over `Expr`.
    fn block(&mut self, blk: &sim_ir::BasicBlock) {
        for &sid in &blk.stmts {
            self.stmt(sid);
        }
        match &blk.term {
            sim_ir::Terminator::Goto { .. }
            | sim_ir::Terminator::Return
            | sim_ir::Terminator::Fork { .. }
            | sim_ir::Terminator::Call { .. } => {}
            sim_ir::Terminator::Branch { cond, .. } => self.expr(*cond),
            sim_ir::Terminator::Delay { amount, .. } => self.expr(*amount),
            sim_ir::Terminator::Wait { cond, .. } => match cond {
                sim_ir::WaitCause::Edge { net, .. } | sim_ir::WaitCause::Named { ev: net } => {
                    self.add(*net)
                }
                sim_ir::WaitCause::Level { nets } => {
                    for &n in nets {
                        self.add(n);
                    }
                }
                sim_ir::WaitCause::Expr { expr } => self.expr(*expr),
                sim_ir::WaitCause::Fork => {}
            },
        }
    }

    fn stmt(&mut self, sid: u32) {
        // `self.ir` is a SHARED reference with the walk's own lifetime, so
        // copying it out frees the `&mut self` the helpers below need — the
        // alternative (cloning each lvalue) allocates once per statement.
        let ir = self.ir;
        match &ir.stmts[sid as usize] {
            sim_ir::Stmt::BlockingAssign { lhs, rhs } => {
                self.lvalue(lhs);
                self.expr(*rhs);
            }
            sim_ir::Stmt::NonblockingAssign { lhs, rhs, delay } => {
                self.lvalue(lhs);
                self.expr(*rhs);
                if let Some(d) = delay {
                    self.expr(*d);
                }
            }
            sim_ir::Stmt::SysTask { fmt, args, .. } => {
                if let Some(f) = fmt {
                    self.expr(*f);
                }
                for &a in args {
                    self.expr(a);
                }
            }
            sim_ir::Stmt::Force { lhs, rhs } => {
                self.lvalue(lhs);
                self.expr(*rhs);
            }
            sim_ir::Stmt::Release { lhs } => self.lvalue(lhs),
            sim_ir::Stmt::Disable { .. } => {}
        }
    }

    fn lvalue(&mut self, lhs: &sim_ir::Lvalue) {
        for c in &lhs.chunks {
            self.add(c.net);
            // A3-iii: the DESTINATION is also a write. Its index expressions are
            // not — `local[i]` reads `i`, and a read routes.
            self.wnets.push(c.net);
            for e in [c.word, c.offset, c.width].into_iter().flatten() {
                self.expr(e);
            }
        }
    }

    /// Every net an expression tree can read.
    ///
    /// EXHAUSTIVE, no `_` arm, and that is a rule rather than a style choice:
    /// this walk feeds an ACCEPT decision, so a variant it forgets to descend
    /// into is a net the gate believes is not there.
    ///
    /// A `Call`'s ARGUMENTS are walked; the callee's BODY is not, because every
    /// function in the table is checked against its own window by the caller.
    fn expr(&mut self, eid: u32) {
        let ir = self.ir;
        self.estack.push(eid);
        while let Some(e) = self.estack.pop() {
            let Some(node) = ir.exprs.get(e as usize) else {
                continue;
            };
            match self.estamp.get_mut(e as usize) {
                Some(s) if *s != self.gen => *s = self.gen,
                _ => continue,
            }
            match node {
                sim_ir::Expr::Const { .. } | sim_ir::Expr::ArrayItem { .. } => {}
                sim_ir::Expr::Signal { net, word } => {
                    self.add(*net);
                    if let Some(w) = word {
                        self.estack.push(*w);
                    }
                }
                // ⚠️ `width` is an ExprId too, NOT a literal bit count: the frozen
                // IR stores a part-select's width as a CONST-EXPR edge (`eval`'s
                // own note: `Add(Sub(msb,lsb),1)`). Skipping it was the same
                // field-semantics slip as `ContAssign.delay`, in the opposite
                // direction — a net named ONLY there would be a net this ACCEPT
                // gate believed was not there. Elaborate emits a constant subtree
                // for MOST sites, and the fold (`const_u32_of_expr`) takes no
                // reader, so nothing this walk does can turn into a wrong ANSWER.
                //
                // ⚠️ Two earlier claims here were wrong, both retracted by the
                // reviews: "no design can be written that kills it" (one was), and
                // "elaborate emits a constant subtree for EVERY site" (the same
                // design is the counterexample). A DESCENDING part-select with a
                // non-constant msb (`loc[i:0]`) reaches the module net `i` ONLY
                // through this edge: the offset is `lsb`, and the msb survives just
                // inside `width = Add(Sub(msb,lsb),1)`. Without this push the design
                // is admitted and its frame body reads a module net. Pinned by
                // `s3a_a_non_constant_part_select_msb_is_seen_through_the_width_edge`.
                //
                // ⚠️ That shape is ALSO a pre-existing silent-wrong of its own —
                // vita folds the unfoldable width to 1 bit and answers, where
                // iverilog rejects the construct — recorded in ROADMAP §2
                // (§3 is the loud queue; a silent answer does not belong there, and an
                // earlier version of this line filed it against a section that had
                // no such entry). This
                // walk is not the place to fix it; refusing the design for tier-3
                // is the right answer either way.
                sim_ir::Expr::Select {
                    base,
                    offset,
                    width,
                    ..
                } => {
                    self.estack.push(*base);
                    self.estack.push(*offset);
                    self.estack.push(*width);
                }
                sim_ir::Expr::Concat { parts } => self.estack.extend(parts.iter().copied()),
                sim_ir::Expr::Replicate { count, value } => {
                    self.estack.push(*count);
                    self.estack.push(*value);
                }
                sim_ir::Expr::Unary { operand, .. } => self.estack.push(*operand),
                sim_ir::Expr::Binary { lhs, rhs, .. } => {
                    self.estack.push(*lhs);
                    self.estack.push(*rhs);
                }
                sim_ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                } => {
                    self.estack.push(*cond);
                    self.estack.push(*then_e);
                    self.estack.push(*else_e);
                }
                sim_ir::Expr::SysFunc { args, .. } => self.estack.extend(args.iter().copied()),
                sim_ir::Expr::Call { args, .. } => {
                    self.saw_call = true;
                    self.estack.extend(args.iter().copied())
                }
            }
        }
    }
}
