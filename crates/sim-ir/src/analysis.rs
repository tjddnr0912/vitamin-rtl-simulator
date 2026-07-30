//! Frame/suspend analysis over the frozen arenas — split out of lib.rs (no SchemaHash types here;
//! the frozen serialized types stay in lib.rs because the registry key embeds `module_path!()`).

/// R22 §3.1-3.3 ROOT: does evaluating this `SysFuncId` DO something — mutate the file
/// table, advance an fd, write a ref argument, advance a seed — rather than just compute a
/// value from state it only reads?
///
/// This is the single canonical statement-level-effect family. The `&mut` process executor
/// implements exactly this set as [`crate::…::StmtEffect`] variants (sim-engine
/// `exec::process::compute_effect`, whose dispatch order this mirrors); the synchronous
/// `&self` frame executor implements NONE of them and reaches them only through the pure
/// `eval` path, which returns 0/X and touches nothing. So membership here is precisely
/// "a statement the frame executor must not be asked to run", which is what
/// [`compute_suspendable_tasks`] needs to route the enclosing task correctly.
///
/// EXHAUSTIVE by construction — no `_` arm. A new `SysFuncId` fails to compile until
/// someone decides which side it is on, because the failure mode of a wrong default here
/// is silent: an effect that lands on the frame path returns 0 and writes nothing, at
/// exit 0, with no diagnostic (that is exactly how R22 §3.2/§3.3 escaped).
///
/// `args` matters for two ids: `$random`/`$dist_*` are effectful ONLY in their seeded form
/// (`$random()` with no argument is a pure draw off the global generator), and `$cast` only
/// in its 2-arg function form. Both conditions are copied from the kernel predicates
/// (`k_random_seeded_rhs` / `k_dist_seeded_rhs` / `k_cast_rhs`) so the classifier and the
/// executor agree on the same shapes — `stmt_effect_family_agrees` in sim-engine pins that.
///
/// `Sformatf` is deliberately NOT in the set. It is a `StmtEffect` on the process path, but
/// the frame executor has its own working intercept for it (`frame_rhs_value`'s
/// `NetKind::String` arm renders through the shared formatter), so marking it would move
/// working designs onto a path they do not need and could be loud-rejected on. Measured, not
/// assumed: a `$sformatf`-only frame task is correct at HEAD in all five subroutine shapes.
pub fn sysfunc_is_stmt_effect(which: sim_ir::SysFuncId, args: &[u32]) -> bool {
    use sim_ir::SysFuncId as S;
    match which {
        // ── writes a ref argument, mutates a container, or advances file/seed state ──
        S::QPopBack | S::QPopFront => true,
        S::AssocFirst | S::AssocNext | S::AssocLast | S::AssocPrev => true,
        // seeded forms only (a bare `$random()`/`$dist_*()` draw is pure).
        S::Random => !args.is_empty(),
        S::DistUniform
        | S::DistNormal
        | S::DistExponential
        | S::DistPoisson
        | S::DistChiSquare
        | S::DistT
        | S::DistErlang => !args.is_empty(),
        // function form `$cast(dst, src)` writes `dst`; the task form is a SysTaskId.
        S::Cast => args.len() == 2,
        S::ValuePlusargs => true,
        S::Fopen => true,
        S::Fgets | S::Fscanf | S::Sscanf | S::Fread | S::Feof | S::Fgetc | S::Ungetc => true,

        // ── pure: computed from state the evaluator only reads ──
        // `Sformatf` is a process-path StmtEffect but has a working frame intercept — see
        // the doc comment above.
        S::Sformatf => false,
        S::Time | S::Realtime | S::Stime => false,
        S::Signed | S::Unsigned | S::Clog2 => false,
        S::Rtoi | S::Itor | S::RealToBits | S::BitsToReal => false,
        S::DynSize | S::AssocExists | S::AssocNum => false,
        S::Urandom | S::UrandomRange => false,
        S::CountOnes | S::OneHot | S::OneHot0 | S::IsUnknown => false,
        S::TestPlusargs => false,
        S::StrLen | S::StrGetC | S::StrSubstr | S::StrToUpper | S::StrToLower | S::StrCmp => false,
        S::ArrSum | S::ArrProduct | S::ArrAnd | S::ArrOr | S::ArrXor => false,
        S::StrAtoi | S::StrAtohex | S::StrAtooct | S::StrAtobin | S::StrAtoreal => false,
        S::Ln | S::Log10 | S::Exp | S::Sqrt | S::Pow | S::Floor | S::Ceil => false,
        S::Sin | S::Cos | S::Tan | S::Asin | S::Acos | S::Atan | S::Atan2 | S::Hypot => false,
        S::Sinh | S::Cosh | S::Tanh | S::Asinh | S::Acosh | S::Atanh => false,
    }
}

/// R22: of the statement-effect family, the subset the synchronous `&self` frame
/// executor can NEVER perform — whatever the operands, whatever the enclosing body.
///
/// This is NOT the same question as [`sysfunc_is_stmt_effect`], and conflating them is a
/// mistake with teeth in both directions:
/// * The family is the right input to the SUSPEND classifier, where over-marking is free
///   (the `&mut` path is a superset of the `&self` path).
/// * It is the wrong input to anything that CHANGES A CALL'S SHAPE, where over-marking
///   costs. Measured: `function automatic logic [63:0] packk (input byte b[]);
///   foreach (b[i]) packk += b[i];` is in the family — `foreach` desugars to
///   `b.first(i)` / `b.next(i)` — yet the `&self` executor runs it correctly, because the
///   iteration key is a body-local. Routing it away from its working path on the family's
///   say-so broke a dyn-array input formal that only the direct-call path can bind.
///
/// The one carve-out is the assoc-iteration step, whose answer depends on the KEY rather
/// than on the id: a body-local key works on `&self`, and only a non-local key does not.
/// `fatal_frame_assoc_iter` asks that question where the key is actually known.
pub fn sysfunc_frame_executor_cannot_perform(which: sim_ir::SysFuncId, args: &[u32]) -> bool {
    use sim_ir::SysFuncId as S;
    match which {
        S::AssocFirst | S::AssocNext | S::AssocLast | S::AssocPrev => false,
        other => sysfunc_is_stmt_effect(other, args),
    }
}

/// R22: direct-rhs form of [`sysfunc_frame_executor_cannot_perform`].
pub fn rhs_frame_executor_cannot_perform(exprs: &[sim_ir::Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(sim_ir::Expr::SysFunc { which, args })
            if sysfunc_frame_executor_cannot_perform(*which, args)
    )
}

/// R22: does this func's OWN reachable body carry an rhs the synchronous `&self` frame
/// executor cannot perform ([`rhs_frame_executor_cannot_perform`])?
///
/// Elaborate uses this to decide that a framed FUNCTION's calls must be emitted as a
/// `Terminator::Call` (the copy-out shape) rather than an `Expr::Call` — that terminator
/// is the only call shape the engine's `run_process` router can look at, so without it a
/// function's statement-level effect silently does nothing.
///
/// No transitive closure is needed, and none would be sound to skip: a frame FUNCTION body
/// cannot contain a `Terminator::Call` at all (`classify_frame_body` runs with
/// `allow_call = false` over every one of them), so there is no nested callee to reach.
pub fn func_body_needs_stmt_executor(
    funcs: &[sim_ir::FuncDef],
    blocks: &[sim_ir::BasicBlock],
    stmts: &[sim_ir::Stmt],
    exprs: &[sim_ir::Expr],
    fid: u32,
) -> bool {
    use sim_ir::{Stmt, Terminator};
    let Some(f) = funcs.get(fid as usize) else {
        return false;
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![f.entry];
    while let Some(b) = stack.pop() {
        if !seen.insert(b) {
            continue;
        }
        let Some(blk) = blocks.get(b as usize) else {
            continue;
        };
        for &sid in &blk.stmts {
            if let Some(Stmt::BlockingAssign { rhs, .. }) = stmts.get(sid as usize) {
                if rhs_frame_executor_cannot_perform(exprs, *rhs) {
                    return true;
                }
            }
        }
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
                for &c in children {
                    stack.push(c);
                }
                stack.push(*resume_bb);
            }
            Terminator::Return => {}
        }
    }
    false
}

/// R22: is `rhs` the DIRECT rhs form of a statement-level effect (see
/// [`sysfunc_is_stmt_effect`])?
///
/// Direct-only is not an approximation, it is the lowering's own rule: elaborate rejects
/// every one of these in a nested position (`E3009 "… supported only as the direct rhs of a
/// blocking assignment"`), and the executor's `compute_effect` likewise matches only the
/// direct rhs. Keying the classifier on the same shape the lowering keys on is what stops
/// the two from disagreeing.
pub fn rhs_is_stmt_effect(exprs: &[sim_ir::Expr], rhs: u32) -> bool {
    matches!(
        exprs.get(rhs as usize),
        Some(sim_ir::Expr::SysFunc { which, args }) if sysfunc_is_stmt_effect(*which, args)
    )
}

/// Round-14 V3/V4: the set of TASK `FuncId`s that must run on the suspendable-frame
/// path (executed by the scheduler's `run_process` with a call-stack) instead of the
/// synchronous `run_task`. A task is suspendable iff its reachable body carries a
/// "suspend signal" — a `Delay`/`Wait`/`Fork` terminator, or a `NonblockingAssign`/
/// `SysTask`/`Force`/`Release`/`disable fork` statement — OR it (transitively) calls a
/// suspendable task.
///
/// R22: FUNCTIONS are classified too. They used to be skipped on the grounds that "IEEE
/// forbids timing controls in a function", but that reasoning conflated two things. What
/// membership in this set actually buys is the `&mut` PROCESS executor, and a function
/// needs that for exactly the same reason a task does — a statement-level effect in its
/// body (`rc = $fgets(line, fd)` in a `function automatic`, R22 §3.1). Suspending is not
/// implied: a function reached here can never carry a timing control, because
/// `validate_frame_body` runs `classify_frame_body(allow_call = false)` over EVERY framed
/// function body and rejects any `Delay`/`Wait`/`Fork`/`Call` terminator, every
/// out-of-frame write, and every `$systask` other than a plain print or a severity. A
/// framed function that reaches the engine is therefore already leaf and non-suspending —
/// the same property `frame_body_is_leaf_nonsuspending` verifies for a task before lifting
/// it — so routing it through `run_process` changes which executor performs its statements
/// and nothing else. Only functions that carry a signal are added, so a function with no
/// statement-level effect keeps its byte-identical synchronous path.
///
/// This is a pure function of the `funcs`/`blocks`/`stmts`/`exprs` arenas, so ELABORATE (to
/// lift the E3009 reject for exactly these tasks) and the ENGINE (to route them at run time)
/// compute the *same* set independently — no serialized sidecar, no `format_version`
/// impact, and the two can never disagree. Deterministic (result is order-independent;
/// the returned `BTreeSet` is sorted).
pub fn compute_suspendable_tasks(
    funcs: &[sim_ir::FuncDef],
    blocks: &[sim_ir::BasicBlock],
    stmts: &[sim_ir::Stmt],
    exprs: &[sim_ir::Expr],
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
    //
    // R22 §3.1-3.3 ROOT: a blocking assign is ALSO a signal when its rhs is a statement-
    // level effect ([`rhs_is_stmt_effect`]) — `rc = $fgets(line, fd)`, `$value$plusargs`,
    // a seeded `$random`, `$fopen`, `$cast`, a queue pop, an assoc-iteration step. The
    // question this predicate answers is "can the synchronous `&self` frame executor run
    // this statement", and for those the answer is no REGARDLESS of where the lhs lives:
    // the effect is in the RHS, not the destination. Before this arm the walk only looked
    // at the lhs, so `rc = $fgets(…)` writing an in-frame OUTPUT formal read as "yes,
    // subset" and the task stayed on the frame executor — where the same call reaches the
    // pure `eval` path, returns 0 and writes nothing. That is why an unrelated
    // `$display("x")` in the same body FIXED the read: `Stmt::SysTask` fell to the `_`
    // arm, marked the task suspendable, and moved the whole body onto the `&mut` executor
    // that can perform the effect. One line of an unrelated statement changing whether a
    // file read works is not a condition any user can predict.
    let stmt_signal = |s: &Stmt, lo: u32, hi: u32| match s {
        Stmt::Disable {
            scope_kind: DisableKind::Scope,
            ..
        } => false,
        Stmt::BlockingAssign { lhs, rhs } => {
            rhs_is_stmt_effect(exprs, *rhs) || lhs.chunks.iter().any(|c| c.net < lo || c.net >= hi)
        }
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
        // This func's frame window `[lo, hi)` (r18): a blocking assign writing a net
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
