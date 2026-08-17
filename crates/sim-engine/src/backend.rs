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
///
/// T0 (doc-21 §7.3): this is a thin wrapper over [`reject_reasons_into`] — the
/// REAL gate and the obs histogram share ONE walk, so the run.json reasons can
/// never drift from what the VM actually refused (a wrong log is a silent-wrong,
/// doc-19 §3).
pub(crate) fn is_codegen_able(
    stmts: &[Stmt],
    exprs: &[Expr],
    body: &[BasicBlock],
    class_new_sites: &std::collections::BTreeMap<u32, u32>,
) -> bool {
    let mut reasons = std::collections::BTreeSet::new();
    reject_reasons_into(stmts, exprs, body, class_new_sites, &mut reasons);
    reasons.is_empty()
}

/// The single-source P9 walk: record every DISTINCT rejection cause of `body`
/// into `out` (empty ⇒ codegen-able). Stable snake_case strings — these are the
/// `reject_reasons` keys of run.json's `codegen` object, so an obs consumer may
/// pin them; rename = breaking change. Unlike the old bool `.all()` this does
/// NOT short-circuit: the histogram must name every cause a body exhibits, and
/// the walk runs once per process TEMPLATE (not per activation), so the extra
/// visits cost nothing measurable.
///
/// The terminator match is `_`-free on purpose (accept-gate rule): `Terminator`
/// is a frozen sim-ir type, so a new variant is a format bump AND a forced
/// decision here — it cannot default onto the silent (accepted) side.
fn reject_reasons_into(
    stmts: &[Stmt],
    exprs: &[Expr],
    body: &[BasicBlock],
    class_new_sites: &std::collections::BTreeMap<u32, u32>,
    out: &mut std::collections::BTreeSet<&'static str>,
) {
    for block in body {
        match block.term {
            Terminator::Goto { .. } | Terminator::Return => {}
            // A Branch condition can itself be a Call (`if (fact(n) > 5)`).
            Terminator::Branch { cond, .. } => {
                if expr_has_call(exprs, cond) {
                    out.insert("user_call_in_expr");
                }
            }
            Terminator::Delay { .. } => {
                out.insert("delay");
            }
            Terminator::Wait { .. } => {
                out.insert("wait");
            }
            Terminator::Fork { .. } => {
                out.insert("fork");
            }
            Terminator::Call { .. } => {
                out.insert("frame_call");
            }
        }
        for &sid in &block.stmts {
            // STATEMENT-LEVEL INTERCEPTS. `compute_effect` turns certain
            // `BlockingAssign`s into something other than "evaluate rhs, write lhs" —
            // a queue pop shrinks the queue, an assoc-iteration writes its ref key, a
            // seeded `$random`/`$dist_*` writes the seed back, `$cast` writes its dst,
            // the file family advances fd state. The VM's `EvalForLval` + `WriteLval`
            // funnel reproduces none of that, so such a body stays on the interpreter.
            //
            // The membership question is answered by `sim_ir::rhs_is_stmt_effect`, the
            // SAME predicate `k_rhs_is_stmt_effect_family` uses — not by a copy of it.
            // This used to be a hand-written list here, and it had drifted: it named
            // `DistUniform` and none of the other six seeded `$dist_*` ids, so
            // `$dist_normal(seed, …)` compiled onto the VM, never wrote the seed back,
            // and every subsequent draw repeated. That list is exhaustive with no `_`
            // arm precisely so a new `SysFuncId` cannot default to the silent side;
            // duplicating it here threw that guarantee away.
            //
            // ONE documented delta: `$sformatf`. The canonical predicate answers "can
            // the &self FRAME executor run this", and it says yes for `$sformatf`
            // because the frame path has its own intercept for it. The VM has no such
            // intercept — `compute_effect` still routes it to `StmtEffect::Sformatf` —
            // so it is excluded here and only here.
            if let Stmt::BlockingAssign { rhs, .. } = &stmts[sid as usize] {
                if sim_ir::rhs_is_stmt_effect(exprs, *rhs) {
                    out.insert("stmt_effect_rhs");
                }
                // Reported apart from the canonical stmt-effect family because it
                // is a VM-only delta (the frame path DOES run `$sformatf`) — the
                // histogram should not blur the two. NOTE the key reads "the body
                // reaches a $sformatf-shaped IR node", not "the source spells
                // $sformatf": elaborate desugars string CONCATENATION (`{s, "!"}`)
                // into a synthetic Sformatf node (strings.rs), and that body is
                // refused for exactly the same reason (differential-review find).
                if matches!(
                    exprs.get(*rhs as usize),
                    Some(Expr::SysFunc {
                        which: SysFuncId::Sformatf,
                        ..
                    })
                ) {
                    out.insert("sformatf");
                }
            }
            // N7 `c = new`: an allocation site is a plain `BlockingAssign` in the IR
            // whose MEANING lives in a StmtId-keyed sidecar — `compute_effect` checks
            // `class_new_sites` FIRST and never evaluates the placeholder rhs. Nothing
            // in the statement itself reveals this, so an IR-only classifier cannot see
            // it: the VM compiled the placeholder, the handle stayed X, and every later
            // field write was dropped with a null-dereference warning while the run
            // exited 0.
            if class_new_sites.contains_key(&sid) {
                out.insert("class_new");
            }
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
            if has_call {
                out.insert("user_call_in_expr");
            }
            // Disable: Phase-2 control flow we will not bake into compiled
            // code. Force/Release: format_version-4 shape reserve — keep
            // compiled bodies away until the semantics increment lands.
            // NBA transport delay (v5): interp-only until increment (A)
            // wires the value-carrying delayed event into the VM path.
            match stmts[sid as usize] {
                Stmt::Disable { .. } => {
                    out.insert("disable");
                }
                Stmt::Force { .. } | Stmt::Release { .. } => {
                    out.insert("force_release");
                }
                Stmt::NonblockingAssign { delay: Some(_), .. } => {
                    out.insert("nba_transport_delay");
                }
                _ => {}
            }
        }
    }
}

/// How much of a design the bytecode VM can claim — the answer to "is
/// [`Backend::Bytecode`](crate::Backend::Bytecode) worth selecting here?".
///
/// A body outside the P9 allow-list runs on the interpreter under EITHER backend, so a
/// design at `0/N` cannot speed up no matter how good the VM gets. That makes this the
/// first thing to measure before reaching for the VM — or before investing in a faster
/// one, since a native backend would inherit exactly the same allow-list.
///
/// Counted over process TEMPLATES (`ir.processes`) — what the VM compiles and caches —
/// not over runtime activations. It is a static property of the elaborated design, so it
/// costs one allow-list walk and touches no simulation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodegenCoverage {
    /// Process templates the VM can compile.
    pub codegen_able: usize,
    /// Process templates in the design.
    pub total: usize,
}

impl CodegenCoverage {
    /// Fraction of templates the VM can claim, in `0.0..=1.0`. A design with no
    /// processes reports `0.0` (nothing to accelerate), not a division by zero.
    pub fn ratio(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.codegen_able as f64 / self.total as f64
        }
    }
}

/// Compute [`CodegenCoverage`] for an elaborated design.
///
/// IR-only census: `class_new_sites` is a `SimOpts` sidecar this signature cannot
/// reach, so a `c = new` body counts as codegen-able here and is refused at the
/// real gate. Over-counts on class designs — the run.json instrument uses
/// [`codegen_report`] with the REAL sidecar instead.
pub fn codegen_coverage(ir: &SimIr) -> CodegenCoverage {
    codegen_report(ir, &std::collections::BTreeMap::new()).coverage
}

/// T0 (doc-21 §7.3): the per-design VM-coverage instrument behind run.json's
/// `codegen` object. Before this, the ONLY way to observe "does the bytecode VM
/// claim anything here" was a `--backend interp` vs `bytecode` A/B timing run —
/// which is how a design whose VM contribution was exactly 0% went unnoticed
/// until an external round-26 measurement (`bench/keccak` 호출형).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenReport {
    /// Process templates the VM claims / total (the existing P9 census).
    pub coverage: CodegenCoverage,
    /// Distinct rejection cause → how many process TEMPLATES exhibit it. A
    /// template with two causes counts under both, so the column sum may exceed
    /// the rejected-template count — each row answers "how many templates does
    /// this cause touch", not a partition. Keys are the stable strings of
    /// `reject_reasons_into`.
    pub reject_reasons: std::collections::BTreeMap<&'static str, u32>,
    /// Function/task templates (`ir.funcs`) — NONE of which are compile
    /// candidates today. The round-26 blind spot made visible: a design whose
    /// work lives in subroutine bodies can report 100% process coverage and
    /// still run 0% of its time on the VM, and `frame_call`/`user_call_in_expr`
    /// rows in the histogram plus a non-zero count here are exactly that shape.
    pub frame_bodies: usize,
}

/// Compute [`CodegenReport`] with the REAL `class_new_sites` sidecar — the same
/// walk (`reject_reasons_into`) the VM's compile gate runs, so the report cannot
/// disagree with what the executor actually did (doc-19 §3: a wrong log is a
/// silent-wrong).
pub fn codegen_report(
    ir: &SimIr,
    class_new_sites: &std::collections::BTreeMap<u32, u32>,
) -> CodegenReport {
    let mut reject_reasons = std::collections::BTreeMap::new();
    let mut codegen_able = 0usize;
    let mut reasons = std::collections::BTreeSet::new();
    for p in &ir.processes {
        reasons.clear();
        reject_reasons_into(&ir.stmts, &ir.exprs, &p.body, class_new_sites, &mut reasons);
        if reasons.is_empty() {
            codegen_able += 1;
        } else {
            for r in &reasons {
                *reject_reasons.entry(*r).or_insert(0u32) += 1;
            }
        }
    }
    CodegenReport {
        coverage: CodegenCoverage {
            codegen_able,
            total: ir.processes.len(),
        },
        reject_reasons,
        frame_bodies: ir.funcs.len(),
    }
}

/// B1: does the expr subtree rooted at `eid` REACH an `Expr::Call`? Walks all
/// child ExprId edges; the frozen arena is post-order (every child < its
/// parent), so the recursion depth is bounded by the expression nesting.
pub(crate) fn expr_has_call(exprs: &[Expr], eid: u32) -> bool {
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

pub(crate) struct CompiledBlock {
    ops: Vec<Op>,
    term: CompiledTerm,
}

/// The P9 allow-list terminators ONLY — `is_codegen_able` guaranteed nothing else
/// reaches the compiler.
///
/// `Branch` keeps the condition `ExprId` for the interpreter path AND, when the
/// condition compiles, an index into `CompiledBody::natives`. Truthiness stays a
/// tri-valued control-flow rule (`Tri::True` only), so the native path computes the
/// VALUE natively and then routes it through the SAME `truthiness` the interpreter
/// uses — the rule is not reimplemented, only the value production moves.
///
/// This matters more than it looks: real RTL writes combinational logic as if/case
/// trees, so a decoder-shaped design spends most of its body time in these conditions.
/// Measured on PicoRV32 + testbench: 316 of 316 branch conditions are natively
/// compilable, and every one of them was interpreted.
#[derive(Clone, Copy)]
pub(crate) enum CompiledTerm {
    Goto(u32),
    Branch {
        cond: u32,
        /// Index into `CompiledBody::natives`, when the condition compiled.
        native: Option<u32>,
        then_bb: u32,
        else_bb: u32,
    },
    Return,
}

/// One VM instruction. `Copy`-small: `Lvalue`/arg vectors live in `CompiledBody` side
/// tables, referenced by index. C2 ops delegate eval to the kernel (no native eval yet).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Op {
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
    /// COMPILE-TIME SPECIALISATION of `ScheduleNba` for a destination the compiler has
    /// already proved is a plain whole-net scalar: no dynamic index to sample, so the
    /// `resolve_lvalue_offsets` call and the `Offsets` it returns are both statically
    /// known (`Inline{[(0,0)], len:1}`) and the queue entry is unconditionally
    /// `NbaLhs::One`. Nothing about that decision can change during a run — an lvalue's
    /// SHAPE is fixed at elaboration and `plain_scalar` is a property of the net, not of
    /// its value — so it is decided once per template instead of 2.5 million times.
    ScheduleNbaScalar { lhs: u32, val: u32 },
    /// COMPILE-TIME SPECIALISATION of `ResolveOff` + `WriteLval` into ONE op, same
    /// proof. Two live conditions remain and are tested here rather than baked, because
    /// both CAN change during a run: `forced` (a `force`/`release` re-targets the net)
    /// and the incoming value's `is_real` (which selects the real→int rounding arm).
    /// Either one falls back to the general funnel.
    WriteScalar { lhs: u32, net: u32, val: u32 },
    /// FUSED `EvalForLval` + `WriteScalar`: one op for a whole blocking assign
    /// whose destination `plain_scalar_dest` proved, and whose RHS goes through
    /// the kernel (no `Op::EvalNative`).
    ///
    /// The register file is an ABI, and for these two ops it is a pure cost: the
    /// produced `Value` is written into `regs[dst]` by one op and `take`n back out
    /// by the very next one. A `Value` is 72 bytes, so that is a round trip
    /// through memory per assignment — and it is why simply running a
    /// `CompiledBody` on tier-3 was measured a WASH against the tier-3 walk,
    /// whose `compute_effect` hands its result straight to `apply_effect`.
    /// Fusing removes the round trip and one dispatch, and changes nothing else:
    /// the same two kernel methods are called with the same arguments in the same
    /// order.
    EvalWriteScalar { lhs: u32, net: u32, rhs: u32 },
    /// FUSED `EvalForLval` + `ScheduleNbaScalar`, same proof and same reason.
    ///
    /// ⚠️ Routing this to `k_schedule_nba` instead is an EQUIVALENT mutation and
    /// no test kills it, which is the honest reading rather than a hole:
    /// `NbaLhs::of` returns `One(c.clone())` for a single-chunk lvalue — the same
    /// thing the specialised method builds — and `k_resolve_lvalue_offsets` on a
    /// destination `plain_scalar_dest` accepted yields exactly the `(0, 0)` pair
    /// it bakes, with no index to report out of range. The two differ in COST,
    /// not in what they queue, so the pin that protects the specialisation is the
    /// op-mix census, not a value differential.
    EvalNbaScalar { lhs: u32, rhs: u32 },
    /// `k_dispatch_systask(which, fmt, &arglists[args], sid)`. `sid` is the source
    /// StmtId — it keys the severity side table (`$fatal`/`$error`/…, P1-1).
    SysTask {
        which: SysTaskId,
        fmt: Option<u32>,
        args: u32,
        sid: u32,
    },
}

impl CompiledBody {
    /// The op MIX of this body, for census pins.
    ///
    /// `compile_body`'s specialisations are silent by construction — a
    /// `plain_scalar_dest` that quietly stopped matching would leave every other
    /// test green and only show up as a lost few percent. Counting the ops is how
    /// that becomes an assertion.
    #[cfg(test)]
    pub(crate) fn op_census(&self) -> std::collections::BTreeMap<&'static str, usize> {
        let mut m = std::collections::BTreeMap::new();
        for b in &self.blocks {
            for o in &b.ops {
                let k = match o {
                    Op::EvalForLval { .. } => "EvalForLval",
                    Op::EvalNative { .. } => "EvalNative",
                    Op::ResolveOff { .. } => "ResolveOff",
                    Op::WriteLval { .. } => "WriteLval",
                    Op::ScheduleNba { .. } => "ScheduleNba",
                    Op::ScheduleNbaScalar { .. } => "ScheduleNbaScalar",
                    Op::WriteScalar { .. } => "WriteScalar",
                    Op::EvalWriteScalar { .. } => "EvalWriteScalar",
                    Op::EvalNbaScalar { .. } => "EvalNbaScalar",
                    Op::SysTask { .. } => "SysTask",
                };
                *m.entry(k).or_insert(0usize) += 1;
            }
        }
        m
    }
}

impl Op {
    /// Is this op the LAST one of the statement it was lowered from — i.e. does a
    /// STATEMENT BOUNDARY fall immediately after it?
    ///
    /// The op stream does not record statement boundaries, and two of the
    /// interpreter's per-statement obligations (`k_call_fatal`, `k_drain_diags`)
    /// are defined AT one, so `vm_exec` has to recover them. It can, exactly:
    /// `compile_body` lowers every statement to a run of ops whose final member is
    /// one of the five below — a blocking assign ends in `WriteLval`/`WriteScalar`,
    /// a nonblocking one in `ScheduleNba`/`ScheduleNbaScalar`, a system task in
    /// `SysTask`. The three that answer `false` (`EvalForLval`, `EvalNative`,
    /// `ResolveOff`) only ever appear BEFORE one of the five.
    ///
    /// ⚠️ Recovering the boundary is not decoration, and "check after every op"
    /// is not an acceptable approximation of it. `run_body` consumes `call_fatal`
    /// AFTER the write, so a fatal latched while resolving a dynamic index
    /// (`mem[f(i)] = 1`, the one call position `is_codegen_able` does not exclude)
    /// still performs its write and stops after it. Checking per-op would return
    /// between `ResolveOff` and `WriteLval` and silently drop that write — and
    /// with it the `E4002` that write owes. `fatal_in_an_lvalue_index_call`
    /// separates the two (measured: making `ResolveOff` a boundary loses the
    /// diagnostic).
    ///
    /// `_`-free on purpose (accept-gate rule): a new `Op` must be a forced
    /// decision here, not default onto the "mid-statement" side.
    pub(crate) fn ends_statement(&self) -> bool {
        match self {
            Op::EvalForLval { .. } | Op::EvalNative { .. } | Op::ResolveOff { .. } => false,
            Op::WriteLval { .. }
            | Op::ScheduleNba { .. }
            | Op::ScheduleNbaScalar { .. }
            | Op::WriteScalar { .. }
            | Op::EvalWriteScalar { .. }
            | Op::EvalNbaScalar { .. }
            | Op::SysTask { .. } => true,
        }
    }
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
/// Can this destination take the specialised whole-net-scalar path?
///
/// Exactly the shape `SimState::write_lvalue`'s fast path claims, decided ONCE per
/// template here instead of per write at run time: a single `Bit` chunk with no
/// word/offset/width, onto a net `plain_scalar` accepts. Both halves are immutable for
/// the run — an lvalue's shape is fixed at elaboration, and `plain_scalar` describes the
/// net's STORAGE (real / frame / handle / two-state / array / width), not its value.
///
/// The two conditions that are NOT immutable — `forced`, and whether the incoming value
/// is real — are deliberately excluded and stay live tests inside the ops.
fn plain_scalar_dest(ctx: Option<&CompileCtx>, lhs: &Lvalue) -> Option<u32> {
    let plain = ctx?.plain;
    match lhs.chunks.as_slice() {
        [c] if matches!(c.kind, sim_ir::SelKind::Bit)
            && c.word.is_none()
            && c.offset.is_none()
            && c.width.is_none()
            && plain.get(c.net as usize).copied().unwrap_or(false) =>
        {
            Some(c.net)
        }
        _ => None,
    }
}

/// What `compile_body` is allowed to specialise with. `None` ⇒ the portable form:
/// every RHS through `Op::EvalForLval`, every write through the general funnel.
///
/// This replaced a four-tuple because the two specialisations are INDEPENDENT and
/// tier-3 wants exactly one of them:
///
///   - `plain` drives `Op::WriteScalar`/`Op::ScheduleNbaScalar`. It describes net
///     STORAGE, so it is true for whichever kernel executes the body.
///   - `natives` drives `Op::EvalNative`, the tier-2 expression VM.
///     `NativeKernel::k_eval_native` builds a fresh `NativeScratch` per call and
///     runs the tier-2 program, whereas `k_eval_for_lvalue` routes to `wprog` —
///     the width-specialised evaluator S2 built, which is where tier-3's speed
///     lives. Emitting natives there would swap the faster path for the slower one
///     **on every RHS both accept**.
///
/// ⚠️⚠️ **THAT QUALIFIER WAS LOAD-BEARING AND THE CODE IGNORED IT (D1.5).** The
/// rule used to be "tier-3 must not take natives", full stop, and it was correct
/// about the RHSs both evaluators accept and silent about a third category: the
/// ones `wprog` REFUSES. `wprog` admits uniform width ≤ 64 bits only, so every
/// wide (>64-bit) expression fell to the generic `eval_ctx` tree walk while tier-2
/// ran it on `native_eval`. Measured: tier-3 was **1.71× slower than the VM** on
/// 100-bit arithmetic and **2.52×** on wide select/concat (ROADMAP §5.1-av).
///
/// ⭐ And the old rule was right that turning natives on unconditionally is worse
/// — measured too: expr-heavy 123 → 478 ms, mem-heavy 86 → 237 ms. So the two
/// evaluators must PARTITION the space rather than compete for it, which is what
/// `natives_when` expresses.
///
/// The table `try_compile` needs (`nonint`) lives INSIDE the option rather than
/// beside a `bool`, so "no natives" cannot carry a stale or wrong-tier table.
pub(crate) struct CompileCtx<'a> {
    pub(crate) ir: &'a SimIr,
    pub(crate) wt: &'a WidthTable,
    pub(crate) plain: &'a [bool],
    /// `Some(nonint)` ⇒ emit `Op::EvalNative` where `try_compile` accepts AND
    /// `natives_when` allows.
    pub(crate) natives: Option<&'a [bool]>,
    /// WHICH accepted RHSs actually get `Op::EvalNative`.
    pub(crate) natives_when: NativesWhen<'a>,
}

/// Which RHSs a kernel wants routed to the tier-2 expression VM.
#[derive(Clone, Copy)]
pub(crate) enum NativesWhen<'a> {
    /// Tier-2: every RHS `try_compile` accepts. It has no second evaluator to
    /// lose to.
    ///
    /// ⚠️ Gated because tier-2 is gated: with the `oracle` feature off there is no
    /// VM, so nothing constructs this — and the no-oracle CI axis is what caught
    /// that (a dead variant is a `-D warnings` error there). This is the axis
    /// working: an enum arm that only one executor uses should disappear with it.
    #[cfg(feature = "oracle")]
    Always,
    /// Tier-3: only the RHSs its OWN evaluator refuses. `wprog` is faster on
    /// everything it takes, and `native_eval` is faster than the generic tree
    /// walk on everything `wprog` leaves — so the two partition the RHS space and
    /// neither is ever displaced by the other.
    ///
    /// ⚠️⚠️ **THE PREDICATE IS PASSED IN, and D1.6 is why.** The first version of
    /// this partition asked `wprog::width_admits` — the width refusal, which is
    /// `compile`'s FIRST line. That is necessary and not sufficient: `wprog` also
    /// declines on node kinds (a runtime-offset part-select, a `SysFunc`, …), and
    /// a census counted **75 such RHSs at ≤64 bits** across the perf shapes. Every
    /// one went to NEITHER evaluator — the generic tree walk — while tier-2 ran it
    /// on `native_eval`.
    ///
    /// So the caller answers the real question `(rhs, w, signed) -> declines?` by
    /// running `wprog::compile`. That needs the arena, which is tier-3's and has
    /// no business in this struct; a closure keeps the layering intact and keeps
    /// the answer a single spelling — `compile` itself, not a re-derivation of
    /// what it accepts.
    OnlyWhereWprogDeclines(&'a dyn Fn(u32, u32, bool) -> bool),
}

fn eval_rhs_op(
    ctx: Option<&CompileCtx>,
    lhs: &Lvalue,
    rhs: u32,
    dst: u32,
    li: u32,
    natives: &mut Vec<NativeProg>,
) -> Op {
    if let Some((c, nonint)) = ctx.and_then(|c| c.natives.map(|n| (c, n))) {
        let (ir, wt) = (c.ir, c.wt);
        let ctx_w = lvalue_width_of(ir, lhs).max(wt.width(rhs));
        let ctx_signed = wt.signed(rhs);
        // The partition. `width_admits` is `wprog::compile`'s own first line, asked
        // The partition. The predicate is the caller's because only tier-3 can
        // answer it — `wprog::compile` needs the arena — and asking `compile`
        // itself is what keeps this a single spelling.
        let wanted = match c.natives_when {
            #[cfg(feature = "oracle")]
            NativesWhen::Always => true,
            NativesWhen::OnlyWhereWprogDeclines(declines) => declines(rhs, ctx_w, ctx_signed),
        };
        if !wanted {
            return Op::EvalForLval { dst, lhs: li, rhs };
        }
        if let Some(prog) = crate::native_eval::try_compile(ir, wt, nonint, rhs, ctx_w, ctx_signed)
        {
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
    ctx: Option<&CompileCtx>,
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
                    // REGISTER REUSE: slot 0, always. A value register's entire live
                    // range is the CONTIGUOUS op triple emitted just below — written by
                    // the eval op, `take`n by `WriteLval` two ops later — and nothing
                    // emitted by this compiler holds a register across statements or
                    // across blocks (the terminator arms use none). So one slot serves
                    // the whole body.
                    //
                    // The counter used to increment per statement, giving a 40-assignment
                    // `always` block 40 registers of which one was ever live. That cost
                    // is paid per ACTIVATION, not per compile: `vm_run_body` clears and
                    // refills the leased file every time the body runs, and each slot is
                    // an `Option<Value>` (~64 B, non-trivial Drop). Measured by ablation
                    // on picorv32 + testbench: the lease cycle was 92 ns of the 709 ns
                    // activation — 7.6% of the whole run — for slots that were all None.
                    //
                    // `compiled_bodies_write_every_register_before_reading_it` pins the
                    // liveness property this rests on.
                    let v = 0;
                    // COMPILE-TIME SPECIALISATION: a destination the compiler can prove
                    // is a plain whole-net scalar has no dynamic index to sample, so the
                    // `ResolveOff` op and the `Offsets` value it produces both vanish —
                    // one fewer op in the stream and one fewer kernel call per assign.
                    //
                    // And when the eval half is the KERNEL call (no `EvalNative`), the
                    // pair fuses into a single op, which is what removes the register
                    // round trip — see `Op::EvalWriteScalar`. Asked in this order because
                    // the fused form must not be emitted for a natively-compiled RHS:
                    // that value comes from `k_eval_native`, a different kernel method.
                    let eval = eval_rhs_op(ctx, lhs, *rhs, v, li, &mut natives);
                    match (
                        plain_scalar_dest(ctx, lhs),
                        matches!(eval, Op::EvalForLval { .. }),
                    ) {
                        (Some(net), true) => ops.push(Op::EvalWriteScalar {
                            lhs: li,
                            net,
                            rhs: *rhs,
                        }),
                        (Some(net), false) => {
                            nregs = 1;
                            ops.push(eval);
                            ops.push(Op::WriteScalar {
                                lhs: li,
                                net,
                                val: v,
                            })
                        }
                        (None, _) => {
                            nregs = 1;
                            ops.push(eval);
                            let o = 0;
                            noffs = 1;
                            ops.push(Op::ResolveOff { dst: o, lhs: li });
                            ops.push(Op::WriteLval {
                                lhs: li,
                                val: v,
                                off: o,
                            });
                        }
                    }
                }
                Stmt::NonblockingAssign { lhs, rhs, delay } => {
                    // delay: Some(_) is excluded by `is_codegen_able` above.
                    debug_assert!(delay.is_none());
                    let li = lvalues.len() as u32;
                    lvalues.push(lhs.clone());
                    // REGISTER REUSE — see the blocking-assign arm. Written here, taken
                    // by the very next op.
                    let v = 0;
                    let eval = eval_rhs_op(ctx, lhs, *rhs, v, li, &mut natives);
                    match (
                        plain_scalar_dest(ctx, lhs),
                        matches!(eval, Op::EvalForLval { .. }),
                    ) {
                        // FUSED — see the blocking arm.
                        (Some(_), true) => ops.push(Op::EvalNbaScalar { lhs: li, rhs: *rhs }),
                        (Some(_), false) => {
                            nregs = 1;
                            ops.push(eval);
                            ops.push(Op::ScheduleNbaScalar { lhs: li, val: v });
                        }
                        (None, _) => {
                            nregs = 1;
                            ops.push(eval);
                            ops.push(Op::ScheduleNba { lhs: li, val: v });
                        }
                    }
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
            } => {
                // Self-width context: a condition is self-determined (§11.6.1 does not
                // propagate a context width into it), which is exactly what the
                // interpreter's `eval(cond)` produces before `truthiness`.
                let native = ctx
                    .and_then(|c| c.natives.map(|n| (c, n)))
                    .and_then(|(c, nonint)| {
                        let (ir, wt) = (c.ir, c.wt);
                        let w = wt.width(*cond).max(1);
                        crate::native_eval::try_compile(ir, wt, nonint, *cond, w, wt.signed(*cond))
                            .map(|p| {
                                let ni = natives.len() as u32;
                                natives.push(p);
                                ni
                            })
                    });
                CompiledTerm::Branch {
                    cond: *cond,
                    native,
                    then_bb: *then_bb,
                    else_bb: *else_bb,
                }
            }
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

#[cfg(feature = "jit")]
impl CompiledBody {
    pub(crate) fn blocks(&self) -> &[CompiledBlock] {
        &self.blocks
    }
    pub(crate) fn lvalue(&self, i: u32) -> &Lvalue {
        &self.lvalues[i as usize]
    }
    pub(crate) fn arglist(&self, i: u32) -> &[u32] {
        &self.arglists[i as usize]
    }
    pub(crate) fn native(&self, i: u32) -> &NativeProg {
        &self.natives[i as usize]
    }
    pub(crate) fn op_at(&self, blk: u32, opi: u32) -> Op {
        self.blocks[blk as usize].ops[opi as usize]
    }
}

#[cfg(feature = "jit")]
impl CompiledBlock {
    pub(crate) fn ops(&self) -> &[Op] {
        &self.ops
    }
    pub(crate) fn term(&self) -> CompiledTerm {
        self.term
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
                    k.k_schedule_nba(&body.lvalues[lhs as usize], value);
                }
                Op::ScheduleNbaScalar { lhs, val } => {
                    let value = regs[val as usize]
                        .take()
                        .expect("ScheduleNbaScalar before EvalForLval");
                    // The compiler proved the destination is a plain whole-net scalar, so
                    // there is no dynamic index to sample: the offsets are the constant
                    // `(0, 0)` pair the general path would have computed.
                    k.k_schedule_nba_scalar(&body.lvalues[lhs as usize], value);
                }
                Op::WriteScalar { lhs, net, val } => {
                    let value = regs[val as usize]
                        .take()
                        .expect("WriteScalar before EvalForLval");
                    k.k_write_scalar(&body.lvalues[lhs as usize], net, value);
                }
                Op::EvalWriteScalar { lhs, net, rhs } => {
                    k.k_eval_write_scalar(&body.lvalues[lhs as usize], net, rhs);
                }
                Op::EvalNbaScalar { lhs, rhs } => {
                    k.k_eval_nba_scalar(&body.lvalues[lhs as usize], rhs);
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
            // THE STATEMENT BOUNDARY — `run_body`'s two per-statement obligations,
            // in its order (consume the fatal FIRST, then drain; a fatal returns
            // without draining, and whoever owns the run loop drains after the
            // body).
            //
            // Both were absent until S3-1, and the two have very different
            // standing, which is worth stating because an earlier version of this
            // comment claimed the same weight for both:
            //
            //  - `k_call_fatal` is REAL and OBSERVABLE. `call_fatal` is latched by
            //    frame machinery, which `is_codegen_able` excludes from every
            //    expression position it scans — but it does NOT scan an lvalue's
            //    INDEX expression, so `mem[f(i)] = 1` with a `$fatal` inside `f`
            //    reaches a compiled body. Without this line the body runs on past
            //    the fatal. That was a live VM-vs-interpreter divergence before
            //    this slice and is pinned by
            //    `fatal_in_an_lvalue_index_call_stops_the_body`.
            //  - `k_drain_diags` is a BACKSTOP and is NOT observable today, and
            //    saying otherwise would be repeating a claim the tier-3 walk
            //    already measured false (`body.rs`: deleting its copy leaves the
            //    suite green and 25 out-of-range designs byte-identical). The
            //    reason is that `format_args_str_with` drains before every
            //    `$display`/`$error` line and the run loop drains after every
            //    body, so no design has been found where the boundary drain is
            //    the one that decides the order. It is here because the walk has
            //    it and the two executors should not differ in what they promise;
            //    treat it as unproven. (For `K = Scheduler` it is a documented
            //    no-op outright.)
            if op.ends_statement() {
                if k.k_call_fatal() {
                    return Step::Fatal;
                }
                k.k_drain_diags();
            }
        }
        match block.term {
            CompiledTerm::Goto(t) => bb = t,
            CompiledTerm::Branch {
                cond,
                then_bb,
                else_bb,
                native,
            } => {
                let t = match native {
                    Some(ni) => {
                        let v = k.k_eval_native(&body.natives[ni as usize]);
                        k.k_truthy_value(&v)
                    }
                    None => k.k_truthy(cond),
                };
                bb = if t { then_bb } else { else_bb };
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

/// How many assign right-hand sides inside codegen-able bodies native-eval can compile.
///
/// This is the number that explains a VM speedup, and it must be measured through the
/// REAL `try_compile` rather than inferred from a list of supported operators. Reading
/// `classify_binop`'s seven-op table as "the supported set" produced a census claiming
/// real RTL bails on 48% of binary nodes; the lowering in `native_eval::compile` in fact
/// handles comparisons, equality, case-equality and the logical binaries through
/// separate arms, so that census was wrong.
///
/// Returns `(compiled, total)` over assign statements in bodies that clear the P9
/// allow-list. Bodies outside it are excluded because their RHS never reaches
/// `try_compile` at all.
pub fn native_eval_coverage(ir: &SimIr) -> (usize, usize) {
    native_eval_coverage_split(ir).0
}

/// `((assign_ok, assign_total), (branch_ok, branch_total))`.
///
/// Branch conditions are counted separately because they are compiled DIFFERENTLY: a
/// `CompiledTerm::Branch` stores the raw ExprId and evaluates it through the
/// interpreter's `k_truthy`, so a natively-compilable condition is still interpreted
/// today. Real RTL writes its combinational logic as if/case trees, so this is where a
/// decoder-shaped design actually spends its time.
pub fn native_eval_coverage_split(ir: &SimIr) -> ((usize, usize), (usize, usize)) {
    let wt = crate::width::WidthTable::build(ir, &crate::FuncTable::new());
    // IR-only view of the native type guard. A class-handle / real-or-string dyn-element
    // net is flagged out of band (`SimState::native_ineligible`), so this census can only
    // OVER-count those; it is a measurement, not a gate.
    let nonint = crate::native_eval::ineligible_nets(ir);
    let (mut ok, mut total) = (0usize, 0usize);
    let (mut bok, mut btotal) = (0usize, 0usize);
    for p in &ir.processes {
        // IR-only census — see `codegen_coverage` on the `class_new_sites` caveat.
        if !is_codegen_able(
            &ir.stmts,
            &ir.exprs,
            &p.body,
            &std::collections::BTreeMap::new(),
        ) {
            continue;
        }
        for block in &p.body {
            for &sid in &block.stmts {
                let (lhs, rhs) = match &ir.stmts[sid as usize] {
                    Stmt::BlockingAssign { lhs, rhs } => (lhs, *rhs),
                    Stmt::NonblockingAssign { lhs, rhs, .. } => (lhs, *rhs),
                    _ => continue,
                };
                total += 1;
                // Context width/sign as the lowering sees it: the lvalue's own width.
                let ctx_w = lhs
                    .chunks
                    .iter()
                    .map(|c| c.width.map_or(wt.width(rhs), |_| wt.width(rhs)))
                    .max()
                    .unwrap_or_else(|| wt.width(rhs));
                if crate::native_eval::try_compile(ir, &wt, &nonint, rhs, ctx_w, wt.signed(rhs))
                    .is_some()
                {
                    ok += 1;
                }
            }
            if let Terminator::Branch { cond, .. } = block.term {
                btotal += 1;
                let w = wt.width(cond).max(1);
                if crate::native_eval::try_compile(ir, &wt, &nonint, cond, w, wt.signed(cond))
                    .is_some()
                {
                    bok += 1;
                }
            }
        }
    }
    ((ok, total), (bok, btotal))
}

// ⚠️ `oracle` as well as `test`: this module's subject is the tier-2 compile
// path (`Op::EvalNative`, `NativesWhen::Always`, the VM's op census), and in a
// product-shape build there is no tier-2 to compile for. The no-oracle CI axis
// is what surfaced it — a dead enum arm there is a `-D warnings` error.
#[cfg(all(test, feature = "oracle"))]
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
        assert!(is_codegen_able(
            &a,
            &[],
            &body,
            &std::collections::BTreeMap::new()
        ));
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
        assert!(!is_codegen_able(
            &a,
            &[],
            &body,
            &std::collections::BTreeMap::new()
        ));
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
            assert!(
                !is_codegen_able(&a, &[], &body, &std::collections::BTreeMap::new()),
                "Wait must exclude"
            );
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
        assert!(!is_codegen_able(
            &a,
            &[],
            &fork,
            &std::collections::BTreeMap::new()
        ));
        let call = vec![block(
            vec![],
            Terminator::Call {
                target: 0,
                ret_bb: 0,
            },
        )];
        assert!(!is_codegen_able(
            &a,
            &[],
            &call,
            &std::collections::BTreeMap::new()
        ));
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
            !is_codegen_able(
                &rhs_assign,
                &exprs,
                &body,
                &std::collections::BTreeMap::new()
            ),
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
            !is_codegen_able(
                &arena(),
                &exprs,
                &cond_body,
                &std::collections::BTreeMap::new()
            ),
            "a frame Call in a Branch cond must exclude the body"
        );

        // $display arg: `$display(fact(n))`.
        let task = vec![Stmt::SysTask {
            which: SysTaskId::Display,
            fmt: None,
            args: vec![1], // fact(n)
        }];
        assert!(
            !is_codegen_able(&task, &exprs, &body, &std::collections::BTreeMap::new()),
            "a frame Call in a $systask arg must exclude the body"
        );

        // Negative control: the same arena WITHOUT a Call stays codegen-able.
        let no_call = vec![Stmt::BlockingAssign {
            lhs: Lvalue { chunks: vec![] },
            rhs: 2, // the Const
        }];
        assert!(
            is_codegen_able(&no_call, &exprs, &body, &std::collections::BTreeMap::new()),
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
        assert!(!is_codegen_able(
            &a,
            &[],
            &body,
            &std::collections::BTreeMap::new()
        ));
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
                !is_codegen_able(&a, &exprs, &body, &std::collections::BTreeMap::new()),
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
            is_codegen_able(&push, &[], &body, &std::collections::BTreeMap::new()),
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
        assert!(!is_codegen_able(
            &a,
            &[],
            &body,
            &std::collections::BTreeMap::new()
        ));
    }

    /// [C2] The compile pass maps blocks 1:1 (P16 debugger correspondence) and lowers
    /// each terminator onto the P9 allow-list verbatim — checked WITHOUT running the VM
    /// (independent of the P5 differential gate, which proves *behaviour*).
    /// The VM's intercept rule IS `sim_ir::rhs_is_stmt_effect`, plus exactly one
    /// documented addition. This pins both halves.
    ///
    /// It used to be a hand-written copy of that list, and the copy had drifted: it
    /// named `DistUniform` and none of the other six seeded `$dist_*` ids. A
    /// `v = $dist_normal(seed, …)` body therefore compiled onto the VM, where the
    /// `EvalForLval` funnel evaluates the rhs and writes the lhs and nothing writes
    /// the SEED back — so every later draw repeated the first, silently, at exit 0.
    ///
    /// Written over the real `is_codegen_able` rather than over the predicate alone,
    /// because the defect was in the classifier, not in the predicate.
    /// The compile-time specialisation must actually FIRE, and must NOT fire on a shape
    /// it cannot prove.
    ///
    /// `Op::WriteScalar`/`Op::ScheduleNbaScalar` are chosen once per template by
    /// `plain_scalar_dest`. If that predicate silently stopped matching, every other test
    /// would stay green — the general ops are correct, only slower — and the
    /// specialisation would be dead code nobody noticed. If it matched a shape it cannot
    /// prove (an array element with a dynamic index, a net `plain_scalar` rejects), the
    /// specialised handler would write the wrong place. Both directions are pinned.
    #[test]
    fn the_scalar_specialisation_fires_only_on_a_provable_destination() {
        let src = "module t; reg [31:0] a; reg [31:0] b; initial begin a = 0; b = 0; end endmodule";
        let (toks, _) = hdl_lexer::lex(src);
        let (su, _) = hdl_parser::parse(&toks, src);
        struct S;
        impl diag::LogSink for S {
            fn emit(&self, _e: diag::LogEvent) {}
        }
        let ir = elaborate::elaborate(&su.expect("unit"), &S).expect("elaborate");
        let wt = crate::width::WidthTable::build(&ir, &crate::FuncTable::new());
        let nn = ir.nets.len();
        let nonint = vec![false; nn];
        // Claim only net 0 as a plain scalar; every other destination must stay general.
        let mut plain = vec![false; nn];
        plain[0] = true;

        let whole = |net: u32| Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        // An ARRAY ELEMENT — `word` present, a dynamic index the specialised op cannot
        // sample — on the SAME net the table claims, so the only thing separating it from
        // the specialised case is the shape.
        let elem = Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net: 0,
                word: Some(0),
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let stmts = vec![
            Stmt::BlockingAssign {
                lhs: whole(0),
                rhs: 0,
            },
            Stmt::NonblockingAssign {
                lhs: whole(0),
                rhs: 0,
                delay: None,
            },
            Stmt::BlockingAssign {
                lhs: elem.clone(),
                rhs: 0,
            },
            Stmt::NonblockingAssign {
                lhs: elem,
                rhs: 0,
                delay: None,
            },
            // net 1 is NOT in the plain-scalar table.
            Stmt::BlockingAssign {
                lhs: whole(1),
                rhs: 0,
            },
        ];
        let body = vec![BasicBlock {
            stmts: vec![0, 1, 2, 3, 4],
            term: Terminator::Return,
        }];
        let cb = compile_body(
            &stmts,
            &body,
            Some(&CompileCtx {
                ir: &ir,
                wt: &wt,
                plain: &plain,
                natives: Some(&nonint),
                natives_when: NativesWhen::Always,
            }),
        );
        let ops = &cb.blocks[0].ops;
        let n = |f: &dyn Fn(&Op) -> bool| ops.iter().filter(|o| f(o)).count();

        assert_eq!(
            n(&|o: &Op| matches!(o, Op::WriteScalar { .. })),
            1,
            "the whole-net scalar blocking assign must specialise: {ops:?}"
        );
        assert_eq!(
            n(&|o: &Op| matches!(o, Op::ScheduleNbaScalar { .. })),
            1,
            "the whole-net scalar NBA must specialise: {ops:?}"
        );
        assert_eq!(
            n(&|o: &Op| matches!(o, Op::WriteLval { .. })),
            2,
            "array element and non-plain net keep the general write: {ops:?}"
        );
        assert_eq!(
            n(&|o: &Op| matches!(o, Op::ScheduleNba { .. })),
            1,
            "the array-element NBA keeps the general form: {ops:?}"
        );
        // A specialised blocking assign drops its `ResolveOff` — two general blocking
        // assigns remain, so two `ResolveOff` remain.
        assert_eq!(n(&|o: &Op| matches!(o, Op::ResolveOff { .. })), 2);

        // ── S3-1: the TIER-3 context — same `plain`, no natives. ──
        //
        // The eval half is then the kernel call, so the two specialised pairs
        // FUSE. This is the only place the fused arms are chosen, and pinning
        // the whole op mix is how a `plain_scalar_dest` that quietly stopped
        // matching (or a fusion that quietly stopped firing) becomes a failure
        // instead of a few lost percent.
        let cb3 = compile_body(
            &stmts,
            &body,
            Some(&CompileCtx {
                ir: &ir,
                wt: &wt,
                plain: &plain,
                natives: None,
                natives_when: NativesWhen::Always,
            }),
        );
        let census = cb3.op_census();
        assert_eq!(
            census,
            [
                ("EvalForLval", 3),
                ("EvalNbaScalar", 1),
                ("EvalWriteScalar", 1),
                ("ResolveOff", 2),
                ("ScheduleNba", 1),
                ("WriteLval", 2),
            ]
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>(),
            "tier-3 op mix moved: {:?}",
            cb3.blocks[0].ops
        );
        // The register file is what the fusion is FOR: five statements, three of
        // which still round-trip a `Value` through `regs`, and two of which no
        // longer do. `nregs` stays 1 because the general ops still need it — the
        // saving is per EXECUTION, not per compile, so this asserts the op mix
        // above rather than a smaller `nregs`.
        assert_eq!((cb3.nregs, cb3.noffs), (1, 1));

        // Without a native context NOTHING specialises: the fallback is the general form,
        // never a guess.
        let cb2 = compile_body(&stmts, &body, None);
        assert_eq!(
            cb2.blocks[0]
                .ops
                .iter()
                .filter(|o| matches!(o, Op::WriteScalar { .. } | Op::ScheduleNbaScalar { .. }))
                .count(),
            0,
            "no native context ⇒ no specialisation"
        );
    }

    /// The liveness property register reuse rests on: within a block, every register
    /// and offset a op READS was WRITTEN by an earlier op in that same block.
    ///
    /// Slot 0 now serves the whole body, so a compiler change that made a register live
    /// across statements — or emitted a read before its write — would no longer be caught
    /// by `Option::take().expect(...)`: it would silently pick up the value the PREVIOUS
    /// statement (or the previous activation) left there. That is a loud-to-silent
    /// trade, so the property is pinned here rather than left to the runtime `expect`.
    ///
    /// Checked over a body mixing both assign kinds and a system task, in a
    /// three-block loop, so the walk sees blocks that start mid-flow rather than a
    /// single straight line.
    #[test]
    fn compiled_bodies_write_every_register_before_reading_it() {
        let lv = |net: u32| Lvalue {
            chunks: vec![sim_ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: sim_ir::SelKind::Bit,
            }],
        };
        let stmts = vec![
            Stmt::BlockingAssign { lhs: lv(0), rhs: 0 },
            Stmt::NonblockingAssign {
                lhs: lv(1),
                rhs: 0,
                delay: None,
            },
            Stmt::SysTask {
                which: SysTaskId::Display,
                fmt: None,
                args: vec![],
            },
            Stmt::BlockingAssign { lhs: lv(2), rhs: 0 },
        ];
        let body = vec![
            BasicBlock {
                stmts: vec![0, 1, 2, 3],
                term: Terminator::Branch {
                    cond: 0,
                    then_bb: 1,
                    else_bb: 2,
                },
            },
            BasicBlock {
                stmts: vec![1, 0],
                term: Terminator::Goto { target: 2 },
            },
            BasicBlock {
                stmts: vec![3],
                term: Terminator::Return,
            },
        ];
        let cb = compile_body(&stmts, &body, None);
        assert_eq!(cb.nregs, 1, "one live register serves the whole body");
        assert_eq!(cb.noffs, 1);

        let mut seen = 0usize;
        for (bi, b) in cb.blocks.iter().enumerate() {
            let mut reg_written: std::collections::BTreeSet<u32> = Default::default();
            let mut off_written: std::collections::BTreeSet<u32> = Default::default();
            for (oi, op) in b.ops.iter().enumerate() {
                match *op {
                    Op::EvalForLval { dst, .. } | Op::EvalNative { dst, .. } => {
                        reg_written.insert(dst);
                    }
                    Op::ResolveOff { dst, .. } => {
                        off_written.insert(dst);
                    }
                    Op::WriteLval { val, off, .. } => {
                        assert!(
                            reg_written.contains(&val),
                            "block {bi} op {oi}: reads register {val} never written in this block"
                        );
                        assert!(
                            off_written.contains(&off),
                            "block {bi} op {oi}: reads offset {off} never written in this block"
                        );
                        seen += 1;
                    }
                    Op::ScheduleNba { val, .. }
                    | Op::ScheduleNbaScalar { val, .. }
                    | Op::WriteScalar { val, .. } => {
                        assert!(
                            reg_written.contains(&val),
                            "block {bi} op {oi}: reads register {val} never written in this block"
                        );
                        seen += 1;
                    }
                    // The FUSED ops carry their value in a temporary that never
                    // reaches the register file, so they have no register to
                    // check — and that IS the property this test is about: a
                    // fused op is one whose value never round-trips through
                    // `regs`. Counted separately so the `seen` floor below
                    // cannot be met by them.
                    Op::EvalWriteScalar { .. } | Op::EvalNbaScalar { .. } => {}
                    Op::SysTask { .. } => {}
                }
            }
        }
        // Not vacuous: the walk must actually have inspected reads.
        assert!(seen >= 6, "only {seen} register reads walked");
    }

    #[test]
    fn the_vm_intercept_rule_is_the_canonical_predicate_plus_sformatf() {
        use sim_ir::SysFuncId as S;
        // A seeded arg list (`args` non-empty) — what makes `$random`/`$dist_*`
        // effectful. Expr 0 is the seed operand, expr 1 the call under test.
        let seeded = |which: S| -> bool {
            let exprs = vec![
                Expr::Const { val: 0 },
                Expr::SysFunc {
                    which,
                    args: vec![0],
                },
            ];
            let stmts = vec![Stmt::BlockingAssign {
                lhs: Lvalue { chunks: vec![] },
                rhs: 1,
            }];
            let body = vec![BasicBlock {
                stmts: vec![0],
                term: Terminator::Return,
            }];
            is_codegen_able(&stmts, &exprs, &body, &std::collections::BTreeMap::new())
        };

        // Every seeded `$dist_*` is excluded — the six that the hand-written copy
        // silently admitted, and the one it named.
        for which in [
            S::DistUniform,
            S::DistNormal,
            S::DistExponential,
            S::DistPoisson,
            S::DistChiSquare,
            S::DistT,
            S::DistErlang,
            S::Random,
        ] {
            assert!(
                !seeded(which),
                "{which:?} writes its seed back and must not reach the VM"
            );
        }

        // The one delta: `$sformatf` is `false` in the canonical predicate (the FRAME
        // executor has its own intercept for it) but must still be excluded here,
        // because the VM has none and `compute_effect` routes it to a `StmtEffect`.
        assert!(
            !sim_ir::sysfunc_is_stmt_effect(S::Sformatf, &[0]),
            "canonical predicate changed its mind about $sformatf — re-check the VM"
        );
        assert!(!seeded(S::Sformatf), "$sformatf must not reach the VM");

        // Not vacuous: a pure sysfunc rhs still compiles.
        assert!(
            seeded(S::Clog2),
            "a pure sysfunc rhs must stay codegen-able"
        );
    }

    /// `c = new` is a plain `BlockingAssign` in the IR whose meaning lives in the
    /// StmtId-keyed `class_new_sites` sidecar, so an IR-only classifier cannot see it.
    /// Compiled, the VM evaluated the placeholder rhs, the handle stayed X, and every
    /// later field write was dropped at exit 0.
    #[test]
    fn a_class_new_site_is_not_codegen_able() {
        let exprs = vec![Expr::Const { val: 0 }];
        let stmts = vec![Stmt::BlockingAssign {
            lhs: Lvalue { chunks: vec![] },
            rhs: 0,
        }];
        let body = vec![BasicBlock {
            stmts: vec![0],
            term: Terminator::Return,
        }];
        assert!(
            is_codegen_able(&stmts, &exprs, &body, &std::collections::BTreeMap::new()),
            "without the sidecar this is an ordinary assign"
        );
        let sites: std::collections::BTreeMap<u32, u32> = [(0u32, 7u32)].into_iter().collect();
        assert!(
            !is_codegen_able(&stmts, &exprs, &body, &sites),
            "the same statement, once `class_new_sites` claims it, must stay interpreted"
        );
    }

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
        assert!(is_codegen_able(
            &a,
            &[],
            &body,
            &std::collections::BTreeMap::new()
        ));
        let cb = compile_body(&a, &body, None);

        // 1:1 block count + per-index terminator mapping.
        assert_eq!(cb.blocks.len(), body.len(), "block count must be 1:1");
        assert!(matches!(
            cb.blocks[0].term,
            CompiledTerm::Branch {
                cond: 0,
                native: None,
                then_bb: 0,
                else_bb: 2
            }
        ));
        assert!(matches!(cb.blocks[1].term, CompiledTerm::Goto(0)));
        assert!(matches!(cb.blocks[2].term, CompiledTerm::Return));

        // Each blocking assign lowers to exactly Eval → Resolve → Write (3 ops).
        for b in &cb.blocks {
            assert_eq!(b.ops.len(), 3, "blocking assign ⇒ 3 ops");
            assert!(matches!(b.ops[0], Op::EvalForLval { .. }));
            assert!(matches!(b.ops[1], Op::ResolveOff { .. }));
            assert!(matches!(b.ops[2], Op::WriteLval { .. }));
        }
        // ONE register and ONE offset, for three assigns across three blocks. The count
        // used to equal the number of assigns; a register's live range is the contiguous
        // op triple above, so they all share slot 0. This is what makes the leased file
        // cheap to reset per activation.
        assert_eq!(cb.nregs, 1);
        assert_eq!(cb.noffs, 1);
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
