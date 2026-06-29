//! Bytecode backend (P0a) — Stage B scaffolding.
//!
//! Today this houses the **P9 scope predicate** that classifies a process body as
//! codegen-able on the bytecode VM. Stage C grows this module into the bytecode
//! compiler + register VM; the predicate already gates which bodies that VM may
//! claim (everything else permanently uses the reference interpreter).

use std::rc::Rc;

use sim_ir::{
    BasicBlock, Expr, LvalChunk, Lvalue, SelKind, SimIr, Stmt, SysFuncId, SysTaskId, Terminator,
};

use crate::builtins::Ctl;
use crate::exec::{Kernel, Step};
use crate::native_eval::NativeProg;
use crate::value::Value;
use crate::width::WidthTable;

/// **P9 scope predicate.** Is this process `body` codegen-able on the bytecode VM?
///
/// A POSITIVE allow-list — NOT a `Fork`/`Call` deny-list (a deny-list would wrongly
/// admit `Delay`/`Wait`). A body qualifies iff **every** terminator is
/// `Goto`/`Branch`/`Return` and **no** statement is `Disable`.
///
/// Loops are fine: `for`/`while`/`forever` lower to `Branch` back-edges (and a bare
/// self-timed `always` wraps in an implicit `forever`), so a qualifying body still
/// runs to `Return` *atomically*, with no suspension. The excluded terminators are
/// exactly the suspend / spawn points a single straight native call cannot express:
///
/// - `Delay` / `Wait` — the true suspend points; resuming them needs a saved
///   resume-PC state machine. (`Wait` also carries `WaitCause::Named`, which the
///   interpreter parks but never wakes — it must never be "compiled" into a hang.)
/// - `Fork` — spawns child activities and a join barrier (the activity arena).
/// - `Call` — an integer-frame call (v1 inlines tasks, so this should not appear,
///   but it is excluded defensively).
///
/// `Stmt::Disable` is excluded as well: a no-op today, but a Phase-2 control-flow
/// change we will not silently bake into compiled code.
///
/// A `BlockingAssign` whose rhs is a queue POP (v5 ④) is also excluded: the pop
/// is side-effecting, so the interpreter intercepts it as a statement-level
/// effect (`StmtEffect::QPop`) — the VM's `EvalForLval` funnel would X-poison
/// instead of popping and silently diverge. (Queue PUSHES stay codegen-able:
/// they are SysTasks riding the shared kernel dispatch.)
///
/// B1 frame-call: a body that REACHES an `Expr::Call` (a user-function call) is
/// also excluded. The frame evaluator runs ONLY on the `&self` interpreter read
/// path (re-entrant frame arena + the left-to-right operand order that static
/// recursion depends on); the VM's native/`EvalForLval` funnels must never
/// reorder or short-circuit it. `expr_has_call` walks the RHS / cond / arg
/// subtrees (the arena is post-order, so the recursion is bounded by ExprId
/// depth). The interpreter is then the SOLE executor of any Call, and the P5
/// differential gate passes vacuously for frame designs.
///
/// Anything not on the allow-list falls back to the interpreter, so an unknown or
/// future terminator/statement variant is safe by default.
pub(crate) fn is_codegen_able(stmts: &[Stmt], exprs: &[Expr], body: &[BasicBlock]) -> bool {
    body.iter().all(|block| {
        let term_ok = match block.term {
            Terminator::Goto { .. } | Terminator::Return => true,
            // A Branch condition can itself be a Call (`if (fact(n) > 5)`).
            Terminator::Branch { cond, .. } => !expr_has_call(exprs, cond),
            _ => false,
        };
        let stmts_ok = block.stmts.iter().all(|&sid| {
            // v5 ④ pops + v6 assoc-iteration calls: both WRITE state from an
            // rhs position (queue shrink / ref key arg), so both are
            // statement-level intercepts the VM's pure EvalForLval funnel
            // cannot reproduce — interpreter-only.
            let pop_rhs = matches!(
                &stmts[sid as usize],
                Stmt::BlockingAssign { rhs, .. } if matches!(
                    exprs.get(*rhs as usize),
                    Some(Expr::SysFunc {
                        which: SysFuncId::QPopBack
                            | SysFuncId::QPopFront
                            | SysFuncId::AssocFirst
                            | SysFuncId::AssocNext
                            | SysFuncId::AssocLast
                            | SysFuncId::AssocPrev,
                        ..
                    })
                )
            );
            // v7: a SEEDED $random rhs writes the seed variable back, and
            // $value$plusargs writes its ref var — the same statement-level
            // intercept family (the NO-ARG $random form stays codegen-able:
            // the draw rides the shared eval kernel).
            let seeded_random_rhs = matches!(
                &stmts[sid as usize],
                Stmt::BlockingAssign { rhs, .. } if matches!(
                    exprs.get(*rhs as usize),
                    Some(Expr::SysFunc { which: SysFuncId::Random, args }) if !args.is_empty()
                )
            );
            let value_plusargs_rhs = matches!(
                &stmts[sid as usize],
                Stmt::BlockingAssign { rhs, .. } if matches!(
                    exprs.get(*rhs as usize),
                    Some(Expr::SysFunc {
                        which: SysFuncId::ValuePlusargs
                            | SysFuncId::Fopen
                            | SysFuncId::Sformatf
                            // v9 SYS-READ: the file-read family advances/mutates
                            // fd state via a statement-level intercept — the VM's
                            // pure EvalForLval funnel would X-poison them.
                            | SysFuncId::Fgetc
                            | SysFuncId::Feof
                            | SysFuncId::Ungetc
                            | SysFuncId::Fgets
                            | SysFuncId::Fread
                            | SysFuncId::Fscanf
                            | SysFuncId::Sscanf
                            // v9 rank 6: $dist_uniform writes the ref seed; $cast
                            // writes the ref dst — both statement-level intercepts.
                            | SysFuncId::DistUniform
                            | SysFuncId::Cast,
                        ..
                    })
                )
            );
            let pop_rhs = pop_rhs || seeded_random_rhs || value_plusargs_rhs;
            // B1: any expr position that can REACH a frame Call excludes the body.
            let has_call = match &stmts[sid as usize] {
                Stmt::BlockingAssign { rhs, .. } | Stmt::Force { rhs, .. } => {
                    expr_has_call(exprs, *rhs)
                }
                Stmt::NonblockingAssign { rhs, delay, .. } => {
                    expr_has_call(exprs, *rhs) || delay.is_some_and(|d| expr_has_call(exprs, d))
                }
                Stmt::SysTask { fmt, args, .. } => {
                    fmt.is_some_and(|f| expr_has_call(exprs, f))
                        || args.iter().any(|&a| expr_has_call(exprs, a))
                }
                Stmt::Disable { .. } | Stmt::Release { .. } => false,
            };
            !pop_rhs
                && !has_call
                && !matches!(
                    stmts[sid as usize],
                    // Disable: Phase-2 control flow we will not bake into compiled
                    // code. Force/Release: format_version-4 shape reserve — keep
                    // compiled bodies away until the semantics increment lands.
                    // NBA transport delay (v5): interp-only until increment (A)
                    // wires the value-carrying delayed event into the VM path.
                    Stmt::Disable { .. }
                        | Stmt::Force { .. }
                        | Stmt::Release { .. }
                        | Stmt::NonblockingAssign { delay: Some(_), .. }
                )
        });
        term_ok && stmts_ok
    })
}

/// B1: does the expr subtree rooted at `eid` REACH an `Expr::Call`? Walks all
/// child ExprId edges; the frozen arena is post-order (every child < its
/// parent), so the recursion depth is bounded by the expression nesting.
fn expr_has_call(exprs: &[Expr], eid: u32) -> bool {
    match exprs.get(eid as usize) {
        Some(Expr::Call { .. }) => true,
        None | Some(Expr::Const { .. } | Expr::Signal { .. } | Expr::ArrayItem { .. }) => false,
        Some(Expr::Select {
            base,
            offset,
            width,
            ..
        }) => {
            expr_has_call(exprs, *base)
                || expr_has_call(exprs, *offset)
                || expr_has_call(exprs, *width)
        }
        Some(Expr::Concat { parts }) => parts.iter().any(|&p| expr_has_call(exprs, p)),
        Some(Expr::Replicate { count, value }) => {
            expr_has_call(exprs, *count) || expr_has_call(exprs, *value)
        }
        Some(Expr::Unary { operand, .. }) => expr_has_call(exprs, *operand),
        Some(Expr::Binary { lhs, rhs, .. }) => {
            expr_has_call(exprs, *lhs) || expr_has_call(exprs, *rhs)
        }
        Some(Expr::Ternary {
            cond,
            then_e,
            else_e,
        }) => {
            expr_has_call(exprs, *cond)
                || expr_has_call(exprs, *then_e)
                || expr_has_call(exprs, *else_e)
        }
        Some(Expr::SysFunc { args, .. }) => args.iter().any(|&a| expr_has_call(exprs, a)),
    }
}

// ── Stage C: bytecode VM (P0a) ─────────────────────────────────────────────
//
// The compiled artifact + register VM that executes a codegen-able (P9 suspend-free)
// process body by calling the SAME `Kernel` the interpreter uses — so net I/O, VCD,
// NBA, scheduling and float formatting reproduce BY CONSTRUCTION (the P5 gate proves
// byte-identity). C2 delegates expression eval to the kernel (`k_eval_for_lvalue`);
// native value registers are C3. The VM's only new code is control flow + cache.
// See `docs/superpowers/plans/2026-06-06-bytecode-vm-stage-c.md`.

/// Per-activation scratch register file: one value slot per (blocking+nonblocking)
/// assign. `Value` has no `Default`/`take`, so the `Option` lets `WriteLval`/`ScheduleNba`
/// `take` the produced value without a clone. Each slot is written (by its `EvalForLval`)
/// before it is read, so reuse across activations would be sound — C2 allocates per
/// activation (structural-milestone simplicity; pooling is a C9 perf item).
pub(crate) type RegFile = Vec<Option<Value>>;
/// Per-activation offset register file: one slot per blocking assign, holding the
/// `(bit-offset, array-word)` pairs `ResolveOff` sampled at statement time (P8 #3).
pub(crate) type OffFile = Vec<Option<crate::exec::Offsets>>;

/// A compiled process body, built ONCE per codegen-able **template** and cached
/// out-of-band on `SimState` (never in the frozen `SimIr`). Block indices are 1:1 with
/// the frozen `Process.body` (SAME indices — the P16 debugger mapping).
pub(crate) struct CompiledBody {
    blocks: Vec<CompiledBlock>,
    /// Cloned LHS side table; `Op`s reference an `Lvalue` by index into this.
    lvalues: Vec<Lvalue>,
    /// Cloned `$systask` arg-ExprId lists; `Op::SysTask` references one by index.
    arglists: Vec<Vec<u32>>,
    /// Pre-compiled native expression programs (VM-only fast path); `Op::EvalNative`
    /// references one by index. Empty when native compilation was disabled (the
    /// `None` ctx) or no RHS qualified.
    natives: Vec<NativeProg>,
    /// How many value / offset registers a single activation needs.
    pub(crate) nregs: u32,
    pub(crate) noffs: u32,
}

struct CompiledBlock {
    ops: Vec<Op>,
    term: CompiledTerm,
}

/// The P9 allow-list terminators ONLY — `is_codegen_able` guaranteed nothing else
/// reaches the compiler. `Branch` carries the condition `ExprId` (truthiness is a
/// tri-valued control-flow rule routed through `k_truthy`, NOT a register boolean).
#[derive(Clone, Copy)]
enum CompiledTerm {
    Goto(u32),
    Branch {
        cond: u32,
        then_bb: u32,
        else_bb: u32,
    },
    Return,
}

/// One VM instruction. `Copy`-small: `Lvalue`/arg vectors live in `CompiledBody` side
/// tables, referenced by index. C2 ops delegate eval to the kernel (no native eval yet).
#[derive(Clone, Copy)]
enum Op {
    /// `regs[dst] = k_eval_for_lvalue(&lvalues[lhs], rhs)` — RHS context-sized to LHS.
    EvalForLval { dst: u32, lhs: u32, rhs: u32 },
    /// `regs[dst] = k_eval_native(&natives[native])` — VM-only native fast path
    /// (byte-identical to `EvalForLval` for the subset `try_compile` accepts).
    EvalNative { dst: u32, native: u32 },
    /// `offs[dst] = k_resolve_lvalue_offsets(&lvalues[lhs])` — dynamic index NOW (P8 #3).
    ResolveOff { dst: u32, lhs: u32 },
    /// `k_write_lvalue(&lvalues[lhs], take(regs[val]), take(offs[off]))` — blocking write.
    WriteLval { lhs: u32, val: u32, off: u32 },
    /// `k_schedule_nba(lvalues[lhs].clone(), take(regs[val]))` — LHS index sampled in NBA.
    ScheduleNba { lhs: u32, val: u32 },
    /// `k_dispatch_systask(which, fmt, &arglists[args], sid)`. `sid` is the source
    /// StmtId — it keys the severity side table (`$fatal`/`$error`/…, P1-1).
    SysTask {
        which: SysTaskId,
        fmt: Option<u32>,
        args: u32,
        sid: u32,
    },
}

/// Lower one codegen-able `body` (statements resolved through `stmts`) to a
/// `CompiledBody`. Mirrors the interpreter's `compute_effect`/`apply_effect`
/// statement-shape EXACTLY (same kernel calls, same order), so the VM introduces zero
/// new value logic. Per the P8 contract a blocking assign emits `EvalForLval → ResolveOff
/// → WriteLval` (RHS, then dynamic index, then write); statements lower in textual order
/// so `ScheduleNba` calls preserve `nba_seq` (moment #2). MUST be called only on a body
/// `is_codegen_able` accepted (the `unreachable!` arms assert that contract).
/// Replicates `SimState::lvalue_width` from the IR alone (no runtime net table) so
/// `compile_body` can compute the RHS eval context (`ctx_w = max(lvalue_w,
/// self_w(rhs))`) the same way `eval_for_lvalue` does. The runtime `NetSlot.width`
/// is seeded verbatim from `ir.nets[n].width` and never mutated, so reading the IR
/// is byte-equivalent.
fn lvalue_width_of(ir: &SimIr, lhs: &Lvalue) -> u32 {
    lhs.chunks
        .iter()
        .map(|c| chunk_width_of(ir, c))
        .sum::<u32>()
        .max(1)
}
fn chunk_width_of(ir: &SimIr, c: &LvalChunk) -> u32 {
    match c.kind {
        SelKind::Bit => {
            if c.offset.is_none() && c.width.is_none() {
                ir.nets[c.net as usize].width
            } else {
                1
            }
        }
        SelKind::PartConst | SelKind::PartIdxUp | SelKind::PartIdxDown => c
            .width
            .and_then(|eid| crate::width::const_u32_of_expr(ir, eid))
            .unwrap_or_else(|| ir.nets[c.net as usize].width),
    }
}

/// Choose the RHS eval op for an assignment: a native program (VM fast path) when
/// `native` ctx is present AND `try_compile` accepts the whole tree, else the
/// kernel-delegating `EvalForLval`. The native and delegated paths are byte-identical
/// (the P5 gate enforces it), so this choice never changes observable behaviour.
fn eval_rhs_op(
    native: Option<(&SimIr, &WidthTable)>,
    lhs: &Lvalue,
    rhs: u32,
    dst: u32,
    li: u32,
    natives: &mut Vec<NativeProg>,
) -> Op {
    if let Some((ir, wt)) = native {
        let ctx_w = lvalue_width_of(ir, lhs).max(wt.width(rhs));
        let ctx_signed = wt.signed(rhs);
        if let Some(prog) = crate::native_eval::try_compile(ir, wt, rhs, ctx_w, ctx_signed) {
            let ni = natives.len() as u32;
            natives.push(prog);
            return Op::EvalNative { dst, native: ni };
        }
    }
    Op::EvalForLval { dst, lhs: li, rhs }
}

pub(crate) fn compile_body(
    stmts: &[Stmt],
    body: &[BasicBlock],
    native: Option<(&SimIr, &WidthTable)>,
) -> CompiledBody {
    let mut lvalues: Vec<Lvalue> = Vec::new();
    let mut arglists: Vec<Vec<u32>> = Vec::new();
    let mut natives: Vec<NativeProg> = Vec::new();
    let mut nregs: u32 = 0;
    let mut noffs: u32 = 0;
    let mut blocks = Vec::with_capacity(body.len());
    for block in body {
        let mut ops = Vec::new();
        for &sid in &block.stmts {
            match &stmts[sid as usize] {
                Stmt::BlockingAssign { lhs, rhs } => {
                    let li = lvalues.len() as u32;
                    lvalues.push(lhs.clone());
                    let v = nregs;
                    nregs += 1;
                    let o = noffs;
                    noffs += 1;
                    ops.push(eval_rhs_op(native, lhs, *rhs, v, li, &mut natives));
                    ops.push(Op::ResolveOff { dst: o, lhs: li });
                    ops.push(Op::WriteLval {
                        lhs: li,
                        val: v,
                        off: o,
                    });
                }
                Stmt::NonblockingAssign { lhs, rhs, delay } => {
                    // delay: Some(_) is excluded by `is_codegen_able` above.
                    debug_assert!(delay.is_none());
                    let li = lvalues.len() as u32;
                    lvalues.push(lhs.clone());
                    let v = nregs;
                    nregs += 1;
                    ops.push(eval_rhs_op(native, lhs, *rhs, v, li, &mut natives));
                    ops.push(Op::ScheduleNba { lhs: li, val: v });
                }
                Stmt::SysTask { which, fmt, args } => {
                    let ai = arglists.len() as u32;
                    arglists.push(args.clone());
                    ops.push(Op::SysTask {
                        which: *which,
                        fmt: *fmt,
                        args: ai,
                        sid,
                    });
                }
                // `is_codegen_able` rejects any body containing these, so they
                // are unreachable for a compiled body; mirror the interpreter's
                // `StmtEffect::Nop` (emit no op) for totality.
                Stmt::Disable { .. } | Stmt::Force { .. } | Stmt::Release { .. } => {}
            }
        }
        let term = match &block.term {
            Terminator::Goto { target } => CompiledTerm::Goto(*target),
            Terminator::Branch {
                cond,
                then_bb,
                else_bb,
            } => CompiledTerm::Branch {
                cond: *cond,
                then_bb: *then_bb,
                else_bb: *else_bb,
            },
            Terminator::Return => CompiledTerm::Return,
            // `is_codegen_able` guarantees only the P9 allow-list reaches here.
            other => unreachable!("non-codegen-able terminator in compile_body: {other:?}"),
        };
        blocks.push(CompiledBlock { ops, term });
    }
    CompiledBody {
        blocks,
        lvalues,
        arglists,
        natives,
        nregs,
        noffs,
    }
}

/// Execute a `CompiledBody` from entry block `bb` to `Return`, calling `k` (the SAME
/// kernel the interpreter drives) for every eval / write / systask / branch / rearm.
/// The `cur_time_mult` prologue is the CALLER's job (it bypasses `run_process`, the only
/// writer — see `Scheduler::vm_run_body`); this function owns ONLY the body's control
/// flow plus the per-activation termination guard (a byte-mirror of exec.rs:176-180).
/// Byte-identical to `run_process` on the codegen-able class — the P5 gate enforces it.
// VM-REGPOOL: `regs`/`offs` are leased from the Scheduler's pool by the caller
// (`vm_run_body`) and returned afterwards, so a per-activation `vec![None; n]`
// pair is no longer allocated on every process step. The caller pre-sizes them.
pub(crate) fn vm_exec(
    k: &mut impl Kernel,
    body: &CompiledBody,
    proc: u32,
    mut bb: u32,
    regs: &mut RegFile,
    offs: &mut OffFile,
) -> Step {
    let mut guard: u64 = 0;
    loop {
        let block = &body.blocks[bb as usize];
        for op in &block.ops {
            match *op {
                Op::EvalForLval { dst, lhs, rhs } => {
                    let v = k.k_eval_for_lvalue(&body.lvalues[lhs as usize], rhs);
                    regs[dst as usize] = Some(v);
                }
                Op::EvalNative { dst, native } => {
                    let v = k.k_eval_native(&body.natives[native as usize]);
                    regs[dst as usize] = Some(v);
                }
                Op::ResolveOff { dst, lhs } => {
                    let o = k.k_resolve_lvalue_offsets(&body.lvalues[lhs as usize]);
                    offs[dst as usize] = Some(o);
                }
                Op::WriteLval { lhs, val, off } => {
                    let value = regs[val as usize]
                        .take()
                        .expect("WriteLval before EvalForLval");
                    let offsets = offs[off as usize]
                        .take()
                        .expect("WriteLval before ResolveOff");
                    k.k_write_lvalue(&body.lvalues[lhs as usize], value, &offsets);
                }
                Op::ScheduleNba { lhs, val } => {
                    let value = regs[val as usize]
                        .take()
                        .expect("ScheduleNba before EvalForLval");
                    k.k_schedule_nba(body.lvalues[lhs as usize].clone(), value);
                }
                Op::SysTask {
                    which,
                    fmt,
                    args,
                    sid,
                } => match k.k_dispatch_systask(which, fmt, &body.arglists[args as usize], sid) {
                    Ctl::Finish => return Step::Finish,
                    Ctl::Stop => return Step::Stop,
                    Ctl::Fatal => return Step::Fatal,
                    Ctl::Continue => {}
                },
            }
        }
        match block.term {
            CompiledTerm::Goto(t) => bb = t,
            CompiledTerm::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                bb = if k.k_truthy(cond) { then_bb } else { else_bb };
            }
            CompiledTerm::Return => {
                k.k_rearm(proc);
                return Step::Done;
            }
        }
        guard += 1;
        if guard > k.k_max_deltas() {
            k.k_mark_fatal();
            return Step::Fatal;
        }
    }
}

/// One `vm_cache` slot: the decide-once codegen-ability + compiled body for a template.
pub(crate) enum VmSlot {
    /// Not yet examined.
    Unchecked,
    /// `is_codegen_able` said no — always interpret this template.
    NotCodegenable,
    /// Codegen-able; the compiled body shared via `Rc` so `vm_run_body` can take an
    /// owned handle out BEFORE the `&mut self` kernel call (the §2.3 borrow protocol).
    Compiled(Rc<CompiledBody>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_ir::{DelayRegion, DisableKind, EdgeKind, Lvalue, WaitCause};

    /// stmts arena: index 0 = a benign blocking assign, index 1 = a `disable`.
    fn arena() -> Vec<Stmt> {
        vec![
            Stmt::BlockingAssign {
                lhs: Lvalue { chunks: vec![] },
                rhs: 0,
            },
            Stmt::Disable {
                scope_kind: DisableKind::Scope,
                target: 0,
            },
        ]
    }

    fn block(stmts: Vec<u32>, term: Terminator) -> BasicBlock {
        BasicBlock { stmts, term }
    }

    /// Straight-line AND looping bodies (Branch back-edge) over Goto/Branch/Return
    /// with only assigns are codegen-able — this is the `always_ff @(posedge clk)`
    /// shape (the edge wait is the process *sensitivity*, not a body terminator).
    #[test]
    fn straight_line_and_loops_are_codegen_able() {
        let a = arena();
        let body = vec![
            block(
                vec![0],
                Terminator::Branch {
                    cond: 0,
                    then_bb: 0, // back-edge: a runtime loop
                    else_bb: 2,
                },
            ),
            block(vec![0], Terminator::Goto { target: 0 }),
            block(vec![0], Terminator::Return),
        ];
        assert!(is_codegen_able(&a, &[], &body));
    }

    #[test]
    fn delay_terminator_is_not_codegen_able() {
        let a = arena();
        let body = vec![block(
            vec![],
            Terminator::Delay {
                amount: 1,
                region: DelayRegion::Active,
                resume: 0,
            },
        )];
        assert!(!is_codegen_able(&a, &[], &body));
    }

    #[test]
    fn wait_terminator_is_not_codegen_able() {
        let a = arena();
        for cond in [
            WaitCause::Edge {
                net: 0,
                kind: EdgeKind::Posedge,
            },
            WaitCause::Level { nets: vec![0] },
            WaitCause::Expr { expr: 0 },
            WaitCause::Named { ev: 0 }, // the never-waking variant — must be excluded
            WaitCause::Fork,            // v8: wait fork — suspend-bearing, excluded
        ] {
            let body = vec![block(vec![], Terminator::Wait { cond, resume: 0 })];
            assert!(!is_codegen_able(&a, &[], &body), "Wait must exclude");
        }
    }

    #[test]
    fn fork_and_call_are_not_codegen_able() {
        let a = arena();
        let fork = vec![block(
            vec![],
            Terminator::Fork {
                children: vec![],
                join: 0,
                resume_bb: 0,
            },
        )];
        assert!(!is_codegen_able(&a, &[], &fork));
        let call = vec![block(
            vec![],
            Terminator::Call {
                target: 0,
                ret_bb: 0,
            },
        )];
        assert!(!is_codegen_able(&a, &[], &call));
    }

    /// B1 frame-call: a process body that REACHES an `Expr::Call` (a user
    /// function call) in a RHS, a Branch cond, or a $systask arg is
    /// interpreter-only — the VM must never run the re-entrant frame evaluator
    /// (it would reorder the operand eval that static recursion depends on).
    #[test]
    fn frame_call_in_body_is_not_codegen_able() {
        // RHS: `r = 1 + fact(n)` (the Call is NESTED under a Binary).
        let exprs = vec![
            Expr::Signal { net: 0, word: None }, // 0: n
            Expr::Call {
                func: 0,
                args: vec![0],
            }, // 1: fact(n)
            Expr::Const { val: 0 },              // 2: 1
            Expr::Binary {
                op: sim_ir::BinOp::Add,
                lhs: 2,
                rhs: 1,
            }, // 3: 1 + fact(n)
        ];
        let rhs_assign = vec![Stmt::BlockingAssign {
            lhs: Lvalue { chunks: vec![] },
            rhs: 3,
        }];
        let body = vec![block(vec![0], Terminator::Return)];
        assert!(
            !is_codegen_able(&rhs_assign, &exprs, &body),
            "a frame Call nested in an RHS must exclude the body"
        );

        // Branch cond: `if (fact(n)) ...` — the Call rides the terminator.
        let cond_body = vec![block(
            vec![],
            Terminator::Branch {
                cond: 1, // fact(n)
                then_bb: 0,
                else_bb: 0,
            },
        )];
        assert!(
            !is_codegen_able(&arena(), &exprs, &cond_body),
            "a frame Call in a Branch cond must exclude the body"
        );

        // $display arg: `$display(fact(n))`.
        let task = vec![Stmt::SysTask {
            which: SysTaskId::Display,
            fmt: None,
            args: vec![1], // fact(n)
        }];
        assert!(
            !is_codegen_able(&task, &exprs, &body),
            "a frame Call in a $systask arg must exclude the body"
        );

        // Negative control: the same arena WITHOUT a Call stays codegen-able.
        let no_call = vec![Stmt::BlockingAssign {
            lhs: Lvalue { chunks: vec![] },
            rhs: 2, // the Const
        }];
        assert!(
            is_codegen_able(&no_call, &exprs, &body),
            "a Call-free body must stay codegen-able"
        );
    }

    #[test]
    fn disable_statement_is_not_codegen_able() {
        let a = arena();
        // a Goto/Return body, but one block runs the `disable` statement (arena idx 1).
        let body = vec![
            block(vec![1], Terminator::Goto { target: 1 }),
            block(vec![0], Terminator::Return),
        ];
        assert!(!is_codegen_able(&a, &[], &body));
    }

    /// v5 ④: a BlockingAssign whose rhs is a queue pop (side-effecting
    /// SysFunc) is interpreter-only — the VM's `EvalForLval` funnel cannot pop
    /// (pure READ phase), so compiling it would silently diverge. Queue PUSH
    /// SysTask bodies stay codegen-able (shared kernel dispatch).
    #[test]
    fn queue_pop_rhs_is_not_codegen_able() {
        for which in [SysFuncId::QPopBack, SysFuncId::QPopFront] {
            let exprs = vec![
                Expr::Signal { net: 0, word: None },
                Expr::SysFunc {
                    which,
                    args: vec![0],
                },
            ];
            let a = vec![Stmt::BlockingAssign {
                lhs: Lvalue { chunks: vec![] },
                rhs: 1,
            }];
            let body = vec![block(vec![0], Terminator::Return)];
            assert!(
                !is_codegen_able(&a, &exprs, &body),
                "{which:?} must exclude"
            );
        }
        let push = vec![Stmt::SysTask {
            which: SysTaskId::QPushBack,
            fmt: None,
            args: vec![0, 0],
        }];
        let body = vec![block(vec![0], Terminator::Return)];
        assert!(
            is_codegen_able(&push, &[], &body),
            "pushes stay codegen-able"
        );
    }

    /// One suspend-bearing block anywhere disqualifies the whole body (the predicate
    /// is `all`, not `any`).
    #[test]
    fn one_bad_block_disqualifies_the_body() {
        let a = arena();
        let body = vec![
            block(vec![0], Terminator::Goto { target: 1 }),
            block(
                vec![],
                Terminator::Delay {
                    amount: 0,
                    region: DelayRegion::Active,
                    resume: 0,
                },
            ),
            block(vec![0], Terminator::Return),
        ];
        assert!(!is_codegen_able(&a, &[], &body));
    }

    /// [C2] The compile pass maps blocks 1:1 (P16 debugger correspondence) and lowers
    /// each terminator onto the P9 allow-list verbatim — checked WITHOUT running the VM
    /// (independent of the P5 differential gate, which proves *behaviour*).
    #[test]
    fn compile_pass_maps_blocks_and_terminators_one_to_one() {
        let a = arena(); // stmt 0 = a blocking assign
        let body = vec![
            block(
                vec![0],
                Terminator::Branch {
                    cond: 0,
                    then_bb: 0, // back-edge
                    else_bb: 2,
                },
            ),
            block(vec![0], Terminator::Goto { target: 0 }),
            block(vec![0], Terminator::Return),
        ];
        assert!(is_codegen_able(&a, &[], &body));
        let cb = compile_body(&a, &body, None);

        // 1:1 block count + per-index terminator mapping.
        assert_eq!(cb.blocks.len(), body.len(), "block count must be 1:1");
        assert!(matches!(
            cb.blocks[0].term,
            CompiledTerm::Branch {
                cond: 0,
                then_bb: 0,
                else_bb: 2
            }
        ));
        assert!(matches!(cb.blocks[1].term, CompiledTerm::Goto(0)));
        assert!(matches!(cb.blocks[2].term, CompiledTerm::Return));

        // Each blocking assign lowers to exactly Eval → Resolve → Write (3 ops), and the
        // register counts equal the number of blocking assigns (3 across the 3 blocks).
        for b in &cb.blocks {
            assert_eq!(b.ops.len(), 3, "blocking assign ⇒ 3 ops");
            assert!(matches!(b.ops[0], Op::EvalForLval { .. }));
            assert!(matches!(b.ops[1], Op::ResolveOff { .. }));
            assert!(matches!(b.ops[2], Op::WriteLval { .. }));
        }
        assert_eq!(cb.nregs, 3);
        assert_eq!(cb.noffs, 3);
    }

    /// [C2] A nonblocking assign lowers to Eval → ScheduleNba (no `ResolveOff` — the NBA
    /// path samples the LHS index itself at schedule time, P8), and `$systask` to one
    /// `SysTask` op referencing a cloned arg list.
    #[test]
    fn compile_pass_nonblocking_and_systask_shapes() {
        use sim_ir::SysTaskId;
        let stmts = vec![
            Stmt::NonblockingAssign {
                lhs: Lvalue { chunks: vec![] },
                rhs: 7,
                delay: None,
            },
            Stmt::SysTask {
                which: SysTaskId::Finish,
                fmt: None,
                args: vec![1, 2, 3],
            },
        ];
        let body = vec![block(vec![0, 1], Terminator::Return)];
        let cb = compile_body(&stmts, &body, None);
        assert_eq!(cb.blocks[0].ops.len(), 3); // Eval, ScheduleNba, SysTask
        assert!(matches!(
            cb.blocks[0].ops[0],
            Op::EvalForLval { rhs: 7, .. }
        ));
        assert!(matches!(cb.blocks[0].ops[1], Op::ScheduleNba { .. }));
        assert!(matches!(cb.blocks[0].ops[2], Op::SysTask { args: 0, .. }));
        assert_eq!(cb.nregs, 1, "one value reg for the NBA");
        assert_eq!(cb.noffs, 0, "NBA does not allocate an offset reg");
        assert_eq!(cb.arglists, vec![vec![1, 2, 3]]);
    }
}
