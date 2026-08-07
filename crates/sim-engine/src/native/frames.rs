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
//! * **`has_hier_call` and `contains_shared_fork` are DEAD rows today.** Both
//!   flags are written only inside elaborate's frame-TASK lowering
//!   (`frames_body.rs`, `frames_classify.rs`), so they are always false for the
//!   functions this predicate reaches — the `is_task` row above gets there first.
//!   They stay because S3b admits tasks, and on that day they are the two shapes
//!   that must NOT come with them.
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
        if fd.is_task {
            return Err("task frames (entered by `Terminator::Call`): S3b");
        }
        // DEAD today — subsumed by the `is_task` row above (see the module doc).
        if m.has_hier_call {
            return Err("a subroutine with a hierarchical call (forced suspendable)");
        }
        if m.contains_shared_fork {
            return Err("a subroutine with a shared fork window");
        }
        w.restart();
        w.func_body(fd.entry)?;
        if w.nets
            .iter()
            .any(|&n| n < m.base_net || n >= m.base_net + m.locals_len)
        {
            return Err("a subroutine that names a net outside its own frame: S3b");
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
    // The two that are NOT: `k_dispatch_systask` hands `dispatch` the arena
    // alone (it holds `&mut Scheduler`, so it cannot also lend `&SimState` to a
    // composite), and `schedule_delayed_cas` evaluates a DELAYED assign's rhs
    // and lvalue offsets through the reader it is given. Both are S3b, when the
    // split reader is threaded through the `(st, nets)` helpers.
    //
    // `NetArena::eval_call` panics rather than X-poisoning, so a seam this
    // enumeration MISSED is loud in the gate rather than a wrong value in a
    // run; these rows are what keep the two known ones from reaching it.
    let mut c = Walk::new(ir);
    for p in &ir.processes {
        for blk in &p.body {
            for &sid in &blk.stmts {
                if let sim_ir::Stmt::SysTask { fmt, args, .. } = &ir.stmts[sid as usize] {
                    if fmt.iter().chain(args.iter()).any(|&e| c.has_call(e)) {
                        return Err("a call in a system-task argument: S3b");
                    }
                }
            }
        }
    }
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
    /// Did this walk descend through an `Expr::Call`? Reset by `restart`.
    saw_call: bool,
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
            saw_call: false,
            stack: Vec::new(),
            estack: Vec::new(),
        }
    }

    fn restart(&mut self) {
        self.gen += 1;
        self.nets.clear();
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
