//! S1d-4a (doc-21 §5 S1 분해) — **the arena as a second `Kernel` implementor.**
//!
//! The grounding that reshaped this plan: tier-3's executor is not a second
//! executor, it is the `Kernel` trait's second implementor. `compute_effect` and
//! `apply_effect` are already generic over `K: Kernel`, so every statement
//! MEANING — context-sizing, offset sampling, the NBA sample-at-schedule rule,
//! the real→int coercion — is shared code that runs unchanged over this store.
//! Byte-identity with the engine is therefore not a property to test for and
//! hope holds; it is structural, and the differential gate exists to catch the
//! places where THIS file breaks it, not the places the shared executor might.
//!
//! ## What "honest" means for the methods that are not implemented, in TWO kinds
//!
//! 52 declarations (51 without the `jit` feature). They divide four ways:
//! **13 store core** · **17 classification predicates** · **18 gate-refused
//! workers** · **4 NOT BUILT**. The counts are counted, not estimated — the
//! first draft said "20 refused" and "16 file methods" and both were wrong
//! (there are 18 and 8).
//!
//! The two-kind split is the correction both reviewers of this slice forced, and
//! it matters more than the arithmetic:
//!
//! - **Gate-refused (18)**: `native::design_eligibility` refuses every design
//!   that can reach them — force/release, queue pop, assoc iteration, class
//!   alloc, `disable fork`, and the eleven funnel-outside workers §4.5.291's
//!   `stmt_effect` row covers (seeded `$random`/`$dist_*`, `$cast`,
//!   `$value$plusargs`, the 8 file methods). `gate_refused!` names the row.
//! - **NOT BUILT (4)**: `k_dispatch_systask`, `k_sformatf`, `k_schedule_nba_at`,
//!   `k_rearm`. An ELIGIBLE design reaches every one of these; what keeps them
//!   out of production is `native::runtime_gate` choosing the VM, one layer below
//!   eligibility (§4.5.288's two-layer verdict). `not_built!` says so and names
//!   the slice that builds it. Using the gate-refused wording here — which the
//!   first draft did for `k_sformatf` — points a maintainer at a gate widening
//!   that never happened.
//!
//! Unreachable is not the same as harmless in either kind: a silent no-op would
//! be a wrong answer waiting for the day the layer above it moves.
//!
//! **The predicates are NOT stubbed, and that asymmetry is the whole design.**
//! A `k_*_rhs` predicate answers a CLASSIFICATION question that decides which
//! effect `compute_effect` builds. Hard-coding `false` — the obvious "it can't
//! happen" stub — would not make those statements loud; it would silently route
//! them down the pure-eval path and produce a different statement. So every
//! predicate is answered for real, through `exec::kpred`, the same functions the
//! `Scheduler` impl now calls. One spelling ⇒ the two backends cannot disagree
//! about what a statement IS; only the workers are loud, and only where the gate
//! already guarantees nothing arrives.
//!
//! ## What is deliberately NOT here
//!
//! - **`k_dispatch_systask` is loud** (with `k_sformatf`, which shares its
//!   blocker: both need the format engine, which renders through `&SimState`).
//!   The first draft sized 4b at "exactly 4 read sites … a parameterisation, not
//!   a rewrite". **That number was at least 3× low and the soundness review
//!   measured it.** The original grep looked for `eval_expr`/`read_net`/
//!   `write_lvalue` and so missed every access spelled another way. The reachable
//!   set in an ELIGIBLE design is at least: `render.rs:260,377` (format args),
//!   `render.rs:676,695` (`$timeformat` non-literal args), `queues_io.rs:690,756`,
//!   `dispatch.rs:528,541,558` (`$dumplimit`, the `$fdisplay`/`$fwrite` fd,
//!   `$fclose`), and `crv_draw.rs:591,592,617` (`$writemem*` — which reads the
//!   MEMORY itself, a direct store read, not a formatter read).
//!   And "zero write sites" was true only of net VALUES: `queues_io.rs:488,491`
//!   write `st.nets[i].vcd_id`/`vcd_word_ids` from inside `$dumpvars`, and
//!   `arena::Slot` has no such field — a categorically different problem, and one
//!   S1d-4d's VCD byte-identity gate runs straight into. 4b must re-measure from
//!   the store side (`&st.nets`, `st.nets[i]`), not from a name list.
//! - **The NBA queue here is a flat `Vec`, not the region machinery.** Entries
//!   are `NbaUpdate` — the ENGINE's type, not a parallel one — so the gate can
//!   compare them field by field, and so 4c inherits a queue whose shape it does
//!   not have to migrate. Draining/regions/delta are 4c.
//! - **`k_rearm` is loud, not a no-op.** Re-arming reads the activity arena's
//!   `is_child` flag and the process's sensitivity kind; that state belongs to
//!   the scheduler 4c builds. A silent no-op would leave a Level process asleep
//!   forever — a hang, not an error — which is precisely the class this file
//!   refuses to introduce.

use std::collections::BTreeMap;

use sim_ir::{Lvalue, SimIr, SysTaskId};

use crate::builtins::Ctl;
use crate::exec::{Kernel, Offsets};
use crate::native::arena::NetArena;
use crate::sched::{NbaLhs, NbaUpdate};
use crate::value::Value;

/// A method the S0 DESIGN gate makes unreachable. `row` names the eligibility
/// row that refuses it, so a future widening of the gate reads as an instruction
/// rather than a mystery. 18 methods qualify (counted, not estimated).
macro_rules! gate_refused {
    ($m:literal, $row:literal) => {
        panic!(
            "tier-3 native kernel: {} is unreachable — `native::design_eligibility` \
             refuses every design that can call it ({}). Reaching this means the gate \
             widened without wiring the method; wire it or restore the row.",
            $m, $row
        )
    };
}

/// A method that is NOT gate-refused — an ELIGIBLE design reaches it — and is
/// simply not built yet. What keeps it out of production is `native::runtime_gate`
/// choosing the VM, one layer below eligibility (§4.5.288's two-layer verdict).
///
/// The distinction is the whole reason this is a second macro. Both reviewers of
/// this slice found the first version using the gate-refused wording for methods
/// no row refuses, which points a future maintainer at a gate widening that never
/// happened — the exact opposite of what the message is for.
macro_rules! not_built {
    ($m:literal, $slice:literal, $why:literal) => {
        panic!(
            "tier-3 native kernel: {} is NOT built yet ({} builds it) — {}. No \
             eligibility row refuses a design that calls it; `native::runtime_gate` \
             keeping such designs on the VM is what makes this unreachable today, so \
             reaching it means a backend was selected before its kernel was finished.",
            $m, $slice, $why
        )
    };
}

/// The tier-3 `Kernel`: the arena store plus the cold context an `EvalCtx` needs.
///
/// Borrows the IR-derived tables rather than owning them — they are the SAME
/// tables the engine reads, so a width or plusarg can never differ between the
/// two backends by construction.
#[allow(dead_code)] // S1d-4b's body walk is the production constructor; today
                    // only the shared-executor differential builds one. Saying that is more
                    // honest than a fake call site or a widened visibility.
pub(crate) struct NativeKernel<'i> {
    pub(crate) ir: &'i SimIr,
    pub(crate) arena: NetArena,
    pub(crate) wt: &'i crate::width::WidthTable,
    /// `class_new_sites`, the one classification question that is not a function
    /// of `ir.exprs` (see `exec::kpred`'s module doc). Shared by reference for
    /// the same single-spelling reason the predicates are shared by call.
    pub(crate) class_new_sites: &'i BTreeMap<u32, u32>,
    pub(crate) plusargs: &'i [String],
    pub(crate) rng: crate::state::RngCells,
    pub(crate) now: u64,
    pub(crate) time_mult: u64,
    pub(crate) prec_mult: u64,
    /// The in-body step ceiling. A CONSTRUCTOR ARGUMENT, not a default: it was
    /// briefly `u64::MAX`, which is not "no opinion" but "no termination guard",
    /// and a runaway combinational body would have spun forever instead of
    /// reporting `F4027`. The gate caught it by comparing against the engine's.
    pub(crate) max_body_steps: u64,
    /// The NBA queue in engine shape. 4c gives it regions and a drain.
    pub(crate) nba: Vec<NbaUpdate>,
    pub(crate) nba_seq: u64,
    pub(crate) fatal: bool,
}

#[allow(dead_code)] // ditto — `new`/`ctx` have exactly one caller, the gate.
impl<'i> NativeKernel<'i> {
    pub(crate) fn new(
        ir: &'i SimIr,
        arena: NetArena,
        wt: &'i crate::width::WidthTable,
        class_new_sites: &'i BTreeMap<u32, u32>,
        plusargs: &'i [String],
        max_body_steps: u64,
    ) -> NativeKernel<'i> {
        NativeKernel {
            ir,
            arena,
            wt,
            class_new_sites,
            plusargs,
            rng: crate::state::RngCells::default(),
            now: 0,
            time_mult: 1,
            prec_mult: 1,
            max_body_steps,
            nba: Vec::new(),
            nba_seq: 0,
            fatal: false,
        }
    }

    /// The read context, built exactly as `SimState::mk_eval_ctx` builds its own
    /// — same fields, same order — with the arena as the reader. Every read in
    /// this file goes through it, so there is one place where the two stores can
    /// differ and it is the `nets` field.
    pub(crate) fn ctx(&self) -> crate::eval::EvalCtx<'_, NetArena> {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets: &self.arena,
            now: self.now,
            wt: self.wt,
            time_mult: self.time_mult,
            rng: &self.rng,
            plusargs: self.plusargs,
        }
    }
}

impl Kernel for NativeKernel<'_> {
    #[cfg(feature = "jit")]
    fn k_nets(&self) -> &dyn crate::eval::NetReader {
        &self.arena
    }

    // ── READ: store-backed, shared rules ──

    fn k_eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
        // `Scheduler::eval_for_lvalue`, restated over this store: the IEEE
        // assignment rule is width = max(lhs, self(rhs)), sign = rhs self-sign.
        let lw = self.arena.lvalue_width(self.ir, lhs);
        let sw = self.wt.get(rhs);
        self.ctx().eval_ctx(rhs, lw.max(sw.width), sw.signed)
    }

    fn k_eval_native(&self, prog: &crate::native_eval::NativeProg) -> Value {
        // `native_eval::run` already takes `&dyn NetReader`, so the tier-2
        // compiled-expression VM runs over the arena with no second copy. The
        // scratch is per-call here (the engine reuses one behind a RefCell —
        // an allocation choice, not a semantic one).
        let mut scratch = crate::native_eval::NativeScratch::default();
        crate::native_eval::run(prog, &self.arena, &mut scratch)
    }

    fn k_resolve_lvalue_offsets(&self, lhs: &Lvalue) -> Offsets {
        crate::eval::resolve_offsets(&self.ctx(), lhs)
    }

    fn k_delay_ticks(&self, eid: u32) -> u64 {
        // The RULE is `eval::delay_ticks_of`, shared with the engine. It was
        // restated here first, and the gate caught that the restatement had
        // dropped the X/Z guard and the `u64::MAX` saturation sentinel — an
        // unbounded delay fired at t+0. Sharing it is what makes that class of
        // divergence unrepresentable rather than merely fixed.
        let v = self.ctx().eval(eid);
        crate::eval::delay_ticks_of(&v, self.time_mult, self.prec_mult)
    }

    fn k_truthy(&self, eid: u32) -> bool {
        self.ctx().truthy(eid)
    }

    fn k_truthy_value(&self, v: &Value) -> bool {
        matches!(self.ctx().truthiness(v), crate::eval::Tri::True)
    }

    fn k_max_deltas(&self) -> u64 {
        self.max_body_steps
    }

    fn k_mark_fatal(&mut self) {
        self.fatal = true;
    }

    // ── WRITE: store-backed ──

    fn k_write_lvalue(&mut self, lhs: &Lvalue, value: Value, offsets: &Offsets) {
        let ir = self.ir;
        self.arena.write_lvalue(ir, lhs, value, offsets);
    }

    fn k_write_scalar(&mut self, lhs: &Lvalue, _net: u32, value: Value) {
        // The compiler proved this destination is a plain whole-net scalar, so
        // the general funnel's offset argument is the constant it would have
        // resolved. The engine's twin takes the same shortcut; `_net` is the id
        // it already knows, and the funnel re-reads it from the chunk — keeping
        // ONE resolution path is worth more here than skipping one index.
        let ir = self.ir;
        let off = Offsets::Inline {
            buf: [(0, 0); 2],
            len: 1,
        };
        self.arena.write_lvalue(ir, lhs, value, &off);
    }

    fn k_schedule_nba(&mut self, lhs: &Lvalue, value: Value) {
        // Sample the dynamic LHS index NOW, before the push — the Active-region
        // rule that makes `a[i] <= x; i = i+1;` use the OLD `i`.
        let offsets = self.k_resolve_lvalue_offsets(lhs);
        let seq = self.nba_seq;
        self.nba_seq += 1;
        self.nba.push(NbaUpdate {
            seq,
            lhs: NbaLhs::of(lhs),
            sampled: value,
            offsets,
        });
    }

    fn k_schedule_nba_scalar(&mut self, lhs: &Lvalue, value: Value) {
        debug_assert_eq!(
            self.k_resolve_lvalue_offsets(lhs).as_slice(),
            &[(0, 0)],
            "specialised NBA destination must resolve to the constant offsets"
        );
        let seq = self.nba_seq;
        self.nba_seq += 1;
        let chunk = lhs.chunks[0].clone();
        self.nba.push(NbaUpdate {
            seq,
            lhs: NbaLhs::One(chunk),
            sampled: value,
            offsets: Offsets::Inline {
                buf: [(0, 0); 2],
                len: 1,
            },
        });
    }

    fn k_schedule_nba_at(&mut self, lhs: &Lvalue, value: Value, ticks: u64) {
        // TRANSPORT delay: the engine files this under `now + ticks` in a
        // separate map. 4c owns the time-bucketed queue; until it exists the
        // entry cannot be filed anywhere that would honour the delay, and
        // dropping it into the same-step queue would run it EARLY — a wrong
        // answer, not a missing feature.
        let _ = (lhs, value, ticks);
        not_built!(
            "k_schedule_nba_at (transport-delay NBA)",
            "S1d-4c",
            "there is no time-bucketed queue, and filing the entry same-step would \
             fire it EARLY — a wrong answer, not a missing feature. The S0 gate does \
             not inspect `NonblockingAssign.delay`, so `a <= #3 b` is eligible"
        )
    }

    // ── CLASSIFICATION: one spelling, shared with the engine's impl ──
    //
    // Answered for real, never stubbed. See the module doc: a `false` here does
    // not make a statement loud, it makes it a DIFFERENT statement.

    fn k_queue_pop_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::queue_pop_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_random_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::random_seeded_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_dist_seeded_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::dist_seeded_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_cast_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::cast_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_value_plusargs_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::value_plusargs_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fopen_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fopen_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fgetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgetc_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_feof_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::feof_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_ungetc_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::ungetc_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fgets_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fgets_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fread_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fread_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_fscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::fscanf_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_sscanf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sscanf_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_sformatf_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::sformatf_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_assoc_iter_rhs(&self, rhs: u32) -> bool {
        crate::exec::kpred::assoc_iter_rhs(self.ir.exprs.as_slice(), rhs)
    }
    fn k_rhs_is_stmt_effect_family(&self, rhs: u32) -> bool {
        sim_ir::rhs_is_stmt_effect(self.ir.exprs.as_slice(), rhs)
    }
    fn k_class_new_site(&self, sid: u32) -> Option<u32> {
        self.class_new_sites.get(&sid).copied()
    }

    // ── REFUSED WORKERS: loud, each naming the row that makes it unreachable ──

    fn k_dispatch_systask(
        &mut self,
        which: SysTaskId,
        _fmt: Option<u32>,
        _args: &[u32],
        _sid: u32,
    ) -> Ctl {
        // NOT gate-refused — this one is CORE and simply not wired yet. The
        // measured scope is in the module doc: `builtins::dispatch` renders
        // through `&SimState` at 4 read sites, which 4b parameterises. Until
        // then the runtime gate keeps every design on the VM, and arriving here
        // says the wiring order was broken.
        panic!(
            "tier-3 native kernel: k_dispatch_systask({which:?}) is not wired — \
             `builtins::dispatch` reads nets through `&SimState` (4 sites; S1d-4b). \
             `native::runtime_gate` must keep eligible designs on the VM until it is."
        )
    }

    fn k_force(&mut self, _lhs: &Lvalue, _value: Value, _rhs: u32, _sid: u32) {
        gate_refused!("k_force", "statement scan rejects `force`/`release`")
    }
    fn k_release(&mut self, _lhs: &Lvalue, _sid: u32) {
        gate_refused!("k_release", "statement scan rejects `force`/`release`")
    }
    fn k_queue_pop(&mut self, _lhs: &Lvalue, _rhs: u32) -> Value {
        gate_refused!("k_queue_pop", "NetKind scan rejects queue storage")
    }
    fn k_assoc_iter(&mut self, _lhs: &Lvalue, _rhs: u32) -> Value {
        gate_refused!("k_assoc_iter", "the assoc NetKind row; the `foreach`-over-dyn-array form is caught by the `dyn_array`/`stmt_effect` rows instead")
    }
    fn k_disable_fork(&mut self) {
        gate_refused!("k_disable_fork", "statement scan rejects `disable fork`")
    }
    fn k_class_alloc(&mut self, _class_id: u32) -> Value {
        gate_refused!("k_class_alloc", "the `class` sidecar row — `class_new_sites`, the very table `k_class_new_site` reads; `NetKind` has no class variant")
    }
    fn k_rearm(&mut self, _proc: u32) {
        not_built!(
            "k_rearm",
            "S1d-4c",
            "re-arming reads the activity arena's `is_child` flag and the process's \
             sensitivity kind, which the scheduler 4c builds. A silent no-op would \
             leave a Level process asleep forever — a hang, not an error"
        )
    }

    // The funnel-outside family (§4.5.291): every one of these writes through
    // its own dispatch instead of `write_lvalue`, and the `stmt_effect` reject
    // row refuses any design containing one. Wiring them is what LIFTS that row.

    fn k_random_seeded(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_random_seeded", "`stmt_effect` row (§4.5.291)")
    }
    fn k_dist_seeded(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_dist_seeded", "`stmt_effect` row (§4.5.291)")
    }
    fn k_cast(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_cast", "`stmt_effect` row (§4.5.291)")
    }
    fn k_value_plusargs(&mut self, _rhs: u32) -> Value {
        gate_refused!(
            "k_value_plusargs",
            "`stmt_effect` row (§4.5.291) — the row both keccak variants lose to"
        )
    }
    fn k_fopen(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fopen", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fgetc(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fgetc", "`stmt_effect` row (§4.5.291)")
    }
    fn k_feof(&mut self, _rhs: u32) -> Value {
        gate_refused!(
            "k_feof",
            "`stmt_effect` row (§4.5.291) — over-marked, ROADMAP §5.1"
        )
    }
    fn k_ungetc(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_ungetc", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fgets(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fgets", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fread(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fread", "`stmt_effect` row (§4.5.291)")
    }
    fn k_fscanf(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_fscanf", "`stmt_effect` row (§4.5.291)")
    }
    fn k_sscanf(&mut self, _rhs: u32) -> Value {
        gate_refused!("k_sscanf", "`stmt_effect` row (§4.5.291)")
    }
    fn k_sformatf(&mut self, _rhs: u32) -> Value {
        // NOT the `stmt_effect` row, and the first version of this file said it was.
        // `sysfunc_is_stmt_effect` answers FALSE for `Sformatf` on purpose
        // (sim-ir/src/analysis.rs:27) — the &self frame executor has its own
        // intercept — so the S0 row built on it does not refuse this. Tier-2 hit
        // the identical hole and patched it with an explicit extra reject
        // (`backend.rs`, "ONE documented delta"); tier-3 needed the same delta and
        // did not have it. It is `not_built` rather than gate-refused because the
        // blocker is the RENDERER, not the write: `$sformatf` writes its value
        // through the ordinary funnel, but producing that value needs the format
        // engine, which renders through `&SimState` exactly as `k_dispatch_systask`
        // does. One blocker, two methods, one slice (4b).
        not_built!(
            "k_sformatf",
            "S1d-4b",
            "rendering needs the format engine, which reads through `&SimState` — \
             the same blocker as `k_dispatch_systask`. NOTE the reachable set is \
             wider than the `$sformatf` spelling: elaborate desugars string \
             CONCATENATION into a synthetic Sformatf node"
        )
    }
}
