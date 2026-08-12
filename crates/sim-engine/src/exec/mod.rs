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

// ---- split parts (mechanical refactor) ----
pub(crate) use frame_window::*;
pub(crate) use process::*;
pub(crate) mod frame_call;
mod frame_window;
pub(crate) mod kpred;
pub(crate) mod plusargs;
mod process;
pub(crate) mod stmt_effect;

/// Outcome of one process activation.
#[derive(Debug)]
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
    /// READ (jit): the net table, so a compiled body's leaf loads go through exactly the
    /// reader every other path uses.
    #[cfg(feature = "jit")]
    fn k_nets(&self) -> &dyn crate::eval::NetReader;
    /// READ: evaluate `rhs` context-sized to `lhs`'s width (IEEE assignment rule).
    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value;
    /// READ: SELF-DETERMINED evaluation of `eid` through THIS kernel's store.
    ///
    /// A1-ii. The `stmt_effect` family reads operands that are not the rhs of the
    /// enclosing assignment — a `$random(seed)` seed variable, a `$dist_*`
    /// parameter, the key of an assoc iteration step — and until this seam those
    /// bodies reached `Scheduler::eval`, i.e. the ENGINE's nets, which is the one
    /// store a native run never writes. Threading them through here is what lets
    /// the bodies be shared verbatim instead of restated per backend.
    ///
    /// A whole-net `Signal` evaluated here equals `read_net(net, None)`: the
    /// self-determined context width and sign ARE the net's own, so the
    /// `resize_keep_sign` at the end of the `Signal` arm is a no-op.
    fn k_eval(&self, eid: u32) -> Value;
    /// READ: the frozen IR. Both implementors hold one; a shared `stmt_effect`
    /// body needs `ir.exprs` to unpack its own argument shape.
    fn k_ir(&self) -> &sim_ir::SimIr;
    /// READ: total destination bit-width of `lhs` (Σ chunk widths), the seed of the
    /// rhs context width. Store-INDEPENDENT — every term is a declared width or a
    /// const-folded part-select width — but each implementor already computes it
    /// over its own net table, so the seam keeps that single spelling.
    fn k_lvalue_width(&self, lhs: &Lvalue) -> u32;
    /// READ: `(self-width, self-signed)` of `eid` from the width table.
    fn k_self_width(&self, eid: u32) -> (u32, bool);
    /// READ: CONTEXT-sized evaluation of `eid` through THIS kernel's store.
    ///
    /// A3-i. The self-determined `k_eval` above is not enough for a subroutine
    /// copy-in: IEEE §13.4.3 makes a formal a variable of its DECLARED type, so
    /// the actual is evaluated in the formal's width and signedness. The engine
    /// spells this `Scheduler::eval_ctx_top`, which reads its own nets — the one
    /// store a native run never writes.
    fn k_eval_ctx(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value;
    /// READ: the base net of subroutine `func`'s frame window, i.e. the net id of
    /// its slot 0. IR-derived (it is `func_table[func].base_net`, threaded
    /// verbatim from elaborate), so both implementors answer identically — the
    /// seam exists because only they hold the table.
    fn k_frame_base(&self, func: u32) -> u32;
    /// READ: the call-site binding for the `Terminator::Call` at process-local
    /// block `bb` of process `proc`, or `None` when the sidecar has no entry
    /// (a deferred hierarchical enable elaborate could not resolve).
    ///
    /// CLONED rather than borrowed, as the engine's own arm clones it: the
    /// caller goes on to take `&mut self`, and a borrow of the sidecar would
    /// keep the kernel immutably borrowed across the call it is describing.
    fn k_task_call_site(&self, proc: u32, bb: u32) -> Option<crate::TaskCallInfo>;
    /// READ: is the `Terminator::Call` at process-local block `bb` of process
    /// `proc` one the SUBSET path can run — a resolved site with a synchronous
    /// callee?
    ///
    /// The runtime twin of the gate's `native::frames::call_site_runnable`, and
    /// it exists so the walk asserts its own precondition instead of trusting a
    /// check made three layers away. Both ask the same two questions of the same
    /// two tables; the gate must compute the suspendable set itself because it
    /// runs before `simulate` installs `SimState::suspendable_tasks`, which is
    /// what this reads.
    fn k_call_site_runnable(&self, proc: u32, bb: u32) -> bool;
    /// Run subroutine `callee` SYNCHRONOUSLY against a fresh frame, and return
    /// the caller copy-outs that still need writing.
    ///
    /// The operation, not the struct (A1-iv-a): everything it touches — the frame
    /// window, the dyn heap, the static slab, the depth guard, the `%m` scope
    /// stack — is `SimState` state that both kernels borrow, so there is one
    /// implementation and both impls delegate to it in a line. What does NOT
    /// happen inside is any read or write of a NET, which is why this can be a
    /// single seam at all: the inputs arrive already evaluated and the scalar
    /// outputs leave unwritten.
    ///
    /// DYN out-formals are copied out INSIDE (they land in the shared heap);
    /// scalar ones come back in `out_binds` order for `k_write_lvalue`.
    fn k_run_subset_task(
        &mut self,
        callee: u32,
        in_vals: &[(u32, Value)],
        dyn_snaps: &[(u32, u32)],
        out_binds: &[(u32, Lvalue)],
    ) -> Vec<(Lvalue, Value)>;
    /// READ: the ExprId of an assoc iteration step's CURRENT-key argument, or
    /// `None` when the step reads none. The caller evaluates it through `k_eval`,
    /// i.e. through its own store.
    fn k_assoc_iter_cur_key(&self, rhs: u32) -> Option<u32>;
    /// The FILE TABLE, as two operations rather than as the struct that holds
    /// it (A1-iv). Both implementors reach the same `SimState` — the table is a
    /// shared object exactly as `dyn_heap` is — so the file family needs no new
    /// storage, only a way to say "read a byte" from a shared body.
    ///
    /// Narrow on purpose. Returning `&mut Scheduler` instead was tried and does
    /// not compile (`Scheduler<'a, 'ir>` is invariant in `'a`), and it would
    /// hand a shared body `sched.st.read_net` / `sched.eval` /
    /// `sched.st.write_lvalue` — the ENGINE's nets, which is the exact defect
    /// A1-ii and A1-iii each measured. Two methods cannot be misused that way.
    fn k_file_read_byte(&mut self, fd: u32) -> Option<u8>;
    /// Push one over-read byte back onto `fd`'s pushback stack.
    fn k_file_unget(&mut self, fd: u32, b: u8);
    /// `$fopen`'s table work: open `name` in `mode` (`None` = the MCD form) and
    /// return the descriptor, 0 on failure. A1-iv-b.
    fn k_file_open(&mut self, name: &str, mode: Option<&str>) -> u32;
    /// `$feof`'s table work: `Some(eof)` for an open descriptor, `None` for a
    /// bad or closed one (which also emits the bad-fd warning).
    fn k_file_eof(&mut self, fd: u32) -> Option<bool>;
    /// READ: net `net` (optional array word) through THIS kernel's store.
    /// A1-iv-c: `$fread` merges each element with its PRIOR value, so it is the
    /// one member of the family that reads its own destination.
    fn k_read_net(&self, net: u32, word: Option<u32>) -> Value;
    /// The declared LOW index of unpacked array `net`, or `None` for a negative
    /// base (which `$fread` refuses loudly).
    fn k_array_base(&self, net: u32) -> Option<u64>;
    /// Emit one `RunReadmem` warning at the current simulation time. The sink and
    /// the clock live on the scheduler for both backends, so this is a forward,
    /// not a second spelling.
    fn k_warn_readmem(&mut self, msg: String);
    /// `$ungetc`'s table work: push `byte` back onto a READ-CAPABLE open `fd`
    /// and clear its EOF latch. `false` when the descriptor is bad or closed
    /// (which also warns) or is write-only (which does not — a write stream is
    /// not pushable, and iverilog says so silently).
    fn k_file_ungetc(&mut self, fd: u32, byte: u8) -> bool;
    /// READ: locate one assoc iteration step against the SHARED heap, given the
    /// current key the caller just read. Returns `(key write, status)`; the caller
    /// performs the write through `k_write_lvalue`.
    fn k_assoc_iter_compute(&self, rhs: u32, cur: Option<Value>) -> (Option<(u32, Value)>, i32);
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
    fn k_schedule_nba(&mut self, lhs: &Lvalue, value: Value);
    /// WRITE: the compile-time-specialised twin of `k_schedule_nba` for a destination
    /// already proved to be a plain whole-net scalar — no dynamic index to sample.
    fn k_schedule_nba_scalar(&mut self, lhs: &Lvalue, value: Value);
    /// WRITE: the compile-time-specialised twin of `k_write_lvalue` for the same shape.
    /// `net` is the destination net id the compiler resolved.
    fn k_write_scalar(&mut self, lhs: &Lvalue, net: u32, value: Value);

    /// WHOLE ASSIGNMENT: evaluate `rhs` in `lhs`'s context and write it into the
    /// plain whole-net scalar `net`. `Op::EvalWriteScalar`'s meaning.
    ///
    /// The DEFAULT is exactly the two calls it replaces, so an implementor that
    /// does not override is byte-identical by construction — this is a seam, not
    /// a new rule, and `K = Scheduler` takes the default.
    ///
    /// It exists because for `K = NativeKernel` the two halves cost more than the
    /// work between them. Measured on picorv32: **47.2% of all expression
    /// evaluations are a ONE-OP program** (a single net read, or a constant), and
    /// around those two memory reads the split path pays a `wcache` borrow plus
    /// an `Rc` clone, an `lvalue_width` walk of the IR, a 72-byte `Value`
    /// construction, a `resize`, and then the write funnel re-deriving the chunk
    /// width it was already told. Every one of those is a compile-time constant
    /// for the op. The override collapses them; the default keeps the meaning.
    fn k_eval_write_scalar(&mut self, lhs: &Lvalue, net: u32, rhs: u32) {
        let value = self.k_eval_for_lvalue(lhs, rhs);
        self.k_write_scalar(lhs, net, value);
    }
    /// WHOLE ASSIGNMENT: the nonblocking twin — `Op::EvalNbaScalar`'s meaning.
    /// Same contract, same default.
    fn k_eval_nba_scalar(&mut self, lhs: &Lvalue, rhs: u32) {
        let value = self.k_eval_for_lvalue(lhs, rhs);
        self.k_schedule_nba_scalar(lhs, value);
    }
    /// READ: evaluate a delay ExprId into global-precision ticks (module-mult
    /// scaled; X/Z → 0 — the shared `Terminator::Delay` rule).
    fn k_delay_ticks(&self, eid: u32) -> u64;
    /// WRITE: schedule a TRANSPORT nonblocking update into the NBA region of
    /// `now + ticks` (v5 increment A; index sampled at schedule time).
    fn k_schedule_nba_at(&mut self, lhs: &Lvalue, value: Value, ticks: u64);
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
    /// WRITE: read a line into the str destination; returns the byte count, or 0
    /// at EOF (leaving the destination UNCHANGED). A fixed-width reg dest reads
    /// up to its width in bytes (or through a newline), right-justified. A `string`
    /// dest (NetKind::String) reads the WHOLE line uncapped and stores it via the
    /// §6.16 packed-string path.
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
    /// READ (R22, drift pin only): does `sim_ir::sysfunc_is_stmt_effect` — the
    /// canonical statement-effect family, which the suspend classifier routes
    /// bodies onto this executor because of — claim `rhs`? Used by a
    /// `debug_assert!` at `compute_effect`'s fall-through so the family and this
    /// executor's arms cannot silently drift apart.
    fn k_rhs_is_stmt_effect_family(&self, rhs: u32) -> bool;
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
    /// Verilog control-flow truthiness of an ALREADY-COMPUTED value.
    ///
    /// Exists so a natively-evaluated branch condition routes through the SAME
    /// tri-valued rule `k_truthy` uses (`Tri::True` only — x/z takes the else branch).
    /// Reimplementing the rule on the compiled side is exactly how a backend silently
    /// diverges on X, so the value production moves and the decision does not.
    fn k_truthy_value(&self, v: &Value) -> bool;
    /// CONTROL: re-arm the process after `Return`, preserving the Edge/Level/Initial
    /// asymmetry (NOT reimplemented). TOTAL on the codegen-able class: such a body has
    /// no `Fork` terminator, so it can never be entered as a fork child (a child's
    /// `Return` is routed to `on_child_complete`, never to `rearm`) — `is_codegen_able`
    /// scans the WHOLE body, so the VM only ever drives top-level activities here.
    fn k_rearm(&mut self, proc: u32);
    /// CONTROL: install the per-process execution context before running its
    /// body — `$time`'s multiplier, the precision multiplier, and the `%m` scope.
    ///
    /// These are exactly the fields an earlier tier-3 slice found the kernel must
    /// not keep its own copies of, and they are written PER PROCESS, so a body
    /// walk that skips this renders `$time` and `%m` from whatever process ran
    /// last. Both implementors forward to the same `exec::enter_body`.
    fn k_enter_body(&mut self, tmpl: u32);
    /// CONTROL: the current simulation time, in global-precision ticks.
    ///
    /// The `Terminator::Delay` rule is `now + delay_ticks(amount)`, and both
    /// halves have to come from the same clock: a kernel that read `now` from
    /// somewhere other than where its nets live would file a resume under a tick
    /// that has already passed, and a wheel key below `now` is never drained.
    fn k_now(&self) -> u64;
    /// CONTROL: the DELTA budget — how many region-cascade iterations one
    /// timestep may take before it is declared an oscillator.
    ///
    /// Distinct from `k_max_deltas`, which despite its name is the IN-BODY step
    /// budget. Both implementors read the field the CLI's `--max-deltas`
    /// populates, so the two backends report the same oscillation at the same
    /// point rather than at two thresholds that happen to be equal today.
    fn k_delta_budget(&self) -> u64;
    /// CONTROL: stop advancing time past this tick (`SimOpts::time_limit`).
    fn k_time_limit(&self) -> Option<u64>;
    /// CONTROL: park this activation on an in-body EVENT (`@(posedge x)`,
    /// `@(sig)`, `wait(expr)`), to be resumed when the cause is satisfied.
    ///
    /// The implementor owns the waiter list AND the arm snapshot an
    /// `@(sig)` needs — the values of the watched nets at suspend time, so a
    /// change that completed BEFORE the wait armed does not spuriously fire it.
    /// Snapshotting is therefore not something the walk can do for it: the two
    /// stores hold net values in different shapes.
    fn k_suspend_on(&mut self, proc: u32, block: u32, cause: &sim_ir::WaitCause);
    /// CONTROL: park this activation and schedule its resume.
    ///
    /// `tick == now` lands in the CURRENT timestep's Active (or Inactive, for
    /// `#0`) region; a later tick goes on the time wheel. Both implementors own
    /// their own region queues, which is exactly why this is a kernel call: it
    /// lets `run_body` decide WHEN to suspend — the IEEE rule — while leaving
    /// WHERE the resume is filed to whoever owns the scheduler.
    fn k_schedule_resume(&mut self, proc: u32, block: u32, tick: u64, inactive: bool);
    /// CONTROL: has a fatal been latched from a `&self` eval context?
    ///
    /// A fatal raised inside an expression (a frame body, a cont-assign rhs) has
    /// no way to return `Step::Fatal` — it can only set a `Cell`. The body walk
    /// consumes it at the next statement boundary so the process STOPS where the
    /// fatal happened instead of running the rest of its body on state the fatal
    /// just declared invalid. Added to the seam by S1d-4c-2b: the tier-3 walk
    /// needs the same check, and reading the flag through its own path would be a
    /// second spelling of "when does a latched fatal take effect".
    fn k_call_fatal(&self) -> bool;
    /// CONTROL: report any diagnostic the STORE could only record, not emit.
    ///
    /// Asymmetric on purpose, and the asymmetry is the whole content: the engine
    /// store owns the diagnostic sink, so it emits at the access and this is a
    /// no-op for it. The tier-3 arena is read through `&self` (`NetReader`) and
    /// the sink lives on the scheduler its owner borrows mutably, so an
    /// out-of-range array access can only be COUNTED there and reported here.
    /// Called at the statement boundary — the finest granularity the seam has —
    /// so an access and the `$display` in the NEXT statement stay in order.
    fn k_drain_diags(&mut self);
    /// CONTROL: the IN-BODY step budget — how many blocks one activation may run
    /// without suspending.
    ///
    /// NOT the delta limit, despite the name: both implementors return
    /// `max_body_steps`. Conflating the two is the round-25 defect that reported
    /// an ordinary `for` loop as a combinational oscillation and produced
    /// `F4027`, and the previous wording here ("the infinite-delta
    /// termination-guard ceiling") was that same conflation written down.
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
pub(crate) enum StmtEffect<'s> {
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
