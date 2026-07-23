//! Engine state: the net value table (+ previous-delta snapshot for edges),
//! VCD wiring, and the single net-write choke point with change detection.

use std::cell::Cell;
use std::io::Write;
use std::rc::Rc;

use diag::{Diagnostic, LogEvent, LogSink, MsgCode, Severity, TimeStamp};
use sim_ir::{BitPacked, Lvalue, NetKind, SelKind, SimIr};
use vcd_writer::{IdCode, VarType, VcdWriter};

use crate::eval::NetReader;
use crate::value::{nwords, top_mask, Value, Words};

// ---- split parts (mechanical refactor) ----
mod changes;
mod frame_eval;
mod init_diag;
mod netread;
mod task_frames;
pub(crate) use task_frames::*;
#[cfg(test)]
mod tests;

/// A boxed `Write` sink for the VCD. v1 production uses a `File`; tests use an
/// in-memory buffer captured via an `Rc<RefCell<Vec<u8>>>` adapter.
pub(crate) type VcdSink = Box<dyn Write>;

/// One net's runtime storage. The current value occupies `array_len * width`
/// bits laid out word `w` at `[w*width .. w*width+width)`.
pub(crate) struct NetSlot {
    pub cur: BitPacked,
    pub prev: BitPacked,
    pub width: u32,
    pub array_len: u32,
    pub signed: bool,
    /// True for a `NetKind::Real` net — drives the real↔int assignment coercion
    /// in `write_lvalue` and the `is_real` flag on reads.
    pub is_real: bool,
    pub vcd_id: Option<IdCode>,
    /// Per-element VCD ids for an unpacked array (Phase-1.x ⑤): one id per
    /// word, declared as `mem[idx]` vars. EMPTY for scalars (vcd_id is used).
    pub vcd_word_ids: Vec<Option<IdCode>>,
}

/// One captured `$strobe`/`$monitor` argument list. Stores ExprIds (not values)
/// so the args are RE-EVALUATED at postponed-flush time, sampling settled
/// end-of-timestep net values. ExprIds index the immutable `ir.exprs` and remain
/// valid for the whole run, so no value snapshot or scope context is needed:
/// `$timeformat(units, precision, suffix, min_width)` runtime state (IEEE 1800
/// §21.3.2), read by the `%t` format spec at each render (so a mid-sim
/// `$timeformat` re-formats every later `%t`, including strobe/monitor flushes —
/// iverilog-pinned). Arguments are taken arithmetically, unclamped (iverilog
/// accepts out-of-range units); a negative `prec` clamps to 0 at the call site.
#[derive(Clone, Debug)]
pub struct TfState {
    /// Power of 10, in seconds, of the display unit (−9 = ns).
    pub units_exp: i32,
    /// Fraction digits (integer args TRUNCATE to this many; reals round via %f).
    pub prec: u32,
    /// Suffix text, %s-coerced at the `$timeformat` call (string or packed ASCII).
    pub suffix: String,
    /// Minimum field width; negative ⇒ LEFT-justify in |minw| (iverilog-pinned).
    pub minw: i32,
}

/// `EvalCtx` is rebuilt from `Scheduler::st` (ir / nets / now / wt) at flush.
#[derive(Clone)]
pub(crate) struct FmtCapture {
    /// `SysTask.fmt`: Option<ExprId> → a `Const{val}` whose `val` is the
    /// format-string ConstId. `None` ⇒ bare-args (space-joined decimals).
    pub fmt: Option<u32>,
    /// `SysTask.args`: the argument ExprIds, evaluated lazily in `format_args_str`.
    pub args: Vec<u32>,
    /// Time multiplier `M` of the process that REGISTERED this capture (snapshot of
    /// `cur_time_mult` at registration). The postponed flush renders `$time`/
    /// `$realtime` with THIS value, never the scheduler's live `cur_time_mult` —
    /// which by flush time holds whatever process ran LAST in the timestep, a
    /// DIFFERENT module's `M` under mixed `` `timescale ``s.
    pub time_mult: u64,
    /// Default radix for unformatted args (P1-5 b/o/h variants); `None` ⇒ decimal.
    pub radix: Option<u8>,
    /// `%m` scope of the registering process (P2-11) — snapshot, like `time_mult`.
    pub scope: String,
}

/// The single global `$monitor` record (IEEE 1364-2005: at most one active
/// monitor list in the entire simulation). A later `$monitor` REPLACES this.
pub(crate) struct MonitorState {
    pub cap: FmtCapture,
    /// Last evaluated 4-state VALUE list of `cap.args` (one `Value` per arg).
    /// `None` ⇒ never printed yet, so the next postponed flush prints
    /// unconditionally (establishment print). IEEE 1364-2005 §17.1 keys $monitor
    /// reprints off the *monitored expression VALUE* changing, NOT off the
    /// rendered string. `Value` derives `PartialEq`/`Eq` over the `(val, unk)`
    /// bit-planes, so equality is exactly 4-state-aware: an X/Z-collapsing format
    /// spec (`%d` rendering any-unknown to "x", `%h`/`%b` collapsing a
    /// partial-unknown group) can NEVER mask a genuine value transition the way a
    /// rendered-string diff would (e.g. `4'b00xx → 4'b0x00` under `%d`, both
    /// printing "x", is correctly detected as a change here).
    pub last_vals: Option<Vec<Value>>,
}

/// A pending deferred-immediate-assertion report (IEEE 1800-2017 §16.4) captured
/// when the action SysTask was REACHED, held until the maturation region. The
/// message text is RENDERED AT REACH — §16.4.3: a deferred action's input
/// arguments are sampled when the assertion is evaluated, NOT at maturation.
/// (`$time`/`%m` are reach-time too: identical to maturation for `$time` since a
/// deferred assert matures in the SAME time slot, and the registering scope for
/// `%m`.) `action_sid` re-keys the `severities` table at maturation so the text
/// routes correctly — `$error`→diagnostic stream + exit class, `$fatal`→abort, a
/// plain `$display`→stdout.
#[derive(Clone)]
pub(crate) struct DeferredReport {
    /// StmtId of the action SysTask — re-keys `severities` at maturation.
    pub action_sid: u32,
    /// The action's `SysTaskId` (governs the stdout newline for non-severity tasks).
    pub which: sim_ir::SysTaskId,
    /// The action text, fully rendered at REACH (reach-time arg values).
    pub message: String,
}

/// Per-timestep postponed-region queue + the global monitor singleton + the
/// deferred-assert maturation queues (§16.4). The deferred maps are keyed by
/// `(marker StmtId, activity id, activity GENERATION)`: the generation
/// disambiguates a recycled activity slot (`free_activities`) so a re-issued
/// child cannot flush a COMPLETED prior instance's still-pending report
/// (otherwise a real failure would silently vanish).
#[derive(Default)]
pub(crate) struct Postponed {
    /// FIFO of pending strobes for the CURRENT timestep. Drained-and-CLEARED at
    /// every postponed flush (one-shot-per-call semantics).
    pub strobes: Vec<FmtCapture>,
    /// The global monitor (replace-on-redefine). `None` until first `$monitor`.
    pub monitor: Option<MonitorState>,
    /// v9 rank 6: the GLOBAL monitor-enable, modeled as `$monitoroff`-disabled
    /// (IEEE 1364-2005 §17.1). It is independent of monitor (re-)establishment —
    /// `$monitor` does NOT reset it — and persists across re-`$monitor`, so a
    /// `$monitoroff` issued before/around a `$monitor` keeps suppressing
    /// change-reprints. `false` (default) = enabled; the inverted spelling lets
    /// `#[derive(Default)]` give the correct "enabled" default. The ESTABLISHMENT
    /// print (and `$monitoron`-forced reprint, both `last_vals == None`) bypass
    /// this flag — only change-triggered reprints obey it.
    pub monitor_disabled: bool,
    /// `assert #0` pending reports, keyed by `(marker StmtId, activity id,
    /// generation)` so a re-reach of the SAME assertion instance REPLACES its
    /// prior report (flush-on-re-reach). Drained at the Observed region each
    /// settled timestep.
    pub deferred_observed: std::collections::BTreeMap<(u32, u32, u32), DeferredReport>,
    /// `assert final` pending reports — same keying; drained at the Reactive
    /// region, after Observed and before Postponed (IEEE 1800 §4.4 order).
    pub deferred_reactive: std::collections::BTreeMap<(u32, u32, u32), DeferredReport>,
}

/// Per-fd read bookkeeping (v9 SYS-READ): the lazy end-of-file flag and the
/// `$ungetc` pushback stack. The OS file offset lives on the `std::fs::File`;
/// this captures the read-side state that offset cannot.
#[derive(Default)]
pub struct FdReadState {
    /// Set true by the first read that returns zero bytes (lazy EOF: it is the
    /// FAILED read that sets it, not data exhaustion). Cleared by `$ungetc`.
    pub eof: bool,
    /// Bytes pushed back by `$ungetc`, a LIFO stack (iverilog-pinned: it is
    /// NOT 1-deep — pushing A,B,C then reading yields C,B,A before the file
    /// resumes). The next `$fgetc`/`$fgets` byte pops the top of this stack
    /// before touching the file.
    pub pushback: Vec<u8>,
}

pub(crate) struct SimState<'a> {
    pub ir: &'a SimIr,
    pub now: u64,
    pub nets: Vec<NetSlot>,
    /// Dirty-sweep list (scheduler R2): nets that took an ACTUAL bit change
    /// since the last `propagate_changes` sweep, in write order (`dirty_flag`
    /// dedups). Replaces the per-delta full O(nets) `cur != prev` scan — the
    /// sweep sorts this list, so the resulting changed-net order is identical
    /// to the old ascending scan (byte-identity).
    pub dirty: Vec<u32>,
    pub dirty_flag: Vec<bool>,
    /// Per-net `force` flag (IEEE §9.3.2): while set, EVERY normal write path
    /// (procedural, NBA commit, cont-assign settle, delayed CA) is a silent
    /// no-op for that net — only `force_write`/`release` touch it. Whole-net
    /// granularity (bit/part-select force targets are rejected at elaborate).
    pub forced: Vec<bool>,
    /// SVPART: per-net flag — a 2-state variable (`bit`/`int`/…). The write funnel
    /// coerces any X/Z bit to 0 (IEEE §6.11.3: a 2-state var can never hold X).
    /// All-false unless a 2-state net exists ⇒ byte-identical for every prior design.
    pub two_state: Vec<bool>,
    /// WAND/WOR: NetIds resolved by wired-AND / wired-OR when multi-driven
    /// (filled from SimOpts in `simulate`; empty for a from-`new()` state).
    pub wired_and_nets: std::collections::BTreeSet<u32>,
    pub wired_or_nets: std::collections::BTreeSet<u32>,
    /// GLITCH: per-net intra-slot bit0 edge summary (cleared on a net's FIRST
    /// dirtying each slot, OR-accumulated on every later same-slot transition).
    /// bit0 = a `posedge` (→1) occurred, bit1 = a `negedge` (→0), bit2 = bit0
    /// changed at all (AnyEdge). For a net written ONCE per slot it equals the
    /// endpoint `edge_fires(kind, prev, cur)` exactly ⇒ byte-identical. Only
    /// MAINTAINED for `is_edge_target` nets and only READ in `propagate_changes`
    /// for those nets, so a single-write/non-glitch design never diverges; the
    /// accumulator's whole purpose is to recover an A→B→A round-trip that the
    /// endpoint `cur != prev` test loses (IEEE §9: a glitch fires the observer
    /// ONCE).
    pub slot_edge: Vec<u8>,
    /// GLITCH: per-net flag — the net is the target of SOME edge-sensitive wake
    /// (a static `@(posedge/negedge …)` sensitivity OR a procedural `@(edge …)`
    /// wait). Computed once at construction (static); gates the `slot_edge`
    /// maintenance so non-clock nets pay nothing. A net never edge-watched has
    /// its `slot_edge` neither reset nor read.
    pub is_edge_target: Vec<bool>,
    /// SELF-RETRIG: the activity currently executing a PROCEDURAL body, or `None`
    /// outside body execution (NBA-region apply, cont-assign settle, clocking
    /// commit). Set/cleared around `run_body`. Lets the write funnel attribute a
    /// BLOCKING write to its author so the author is not re-triggered by its own
    /// write (IEEE §9: a process suspended on `@(sig)` does not respond to a
    /// blocking change it made before re-arming — `always @(a) a=~a` ticks once,
    /// not forever; an NBA self-write DOES re-fire because it lands post-rearm).
    pub blocking_writer: Option<u32>,
    /// SELF-RETRIG: per-net author of the LAST value change this slot — an
    /// activity id for a blocking proc write, or `u32::MAX` for any other writer
    /// (NBA/cont-assign/clocking/force = not a blocking self-write). Overwritten
    /// on every `note_change`, so it is always fresh for a net in `changed_nets`;
    /// `propagate_changes` suppresses firing a process on a net it itself
    /// blocking-wrote. `u32::MAX` (no real activity id) means "re-fire normally".
    pub last_blocking_writer: Vec<u32>,
    /// IEEE §9.3.2 continuous-force registry: net → (whole-net lvalue, rhs
    /// ExprId, forcing module's time multiplier). BTreeMap ⇒ deterministic
    /// re-evaluation order; empty unless a force is live (zero steady cost).
    pub active_forces: std::collections::BTreeMap<u32, (sim_ir::Lvalue, u32, u64, bool)>,
    /// C-FORCE-REEVAL-p2 sidecar: net (a source the force RHS reads) → the set
    /// of force keys (`active_forces` keys = target nets) whose RHS reads that
    /// net. Rebuilt incrementally at every `active_forces` insert/remove, NEVER
    /// per delta. Lets `reeval_active_forces` re-evaluate only the forces whose
    /// inputs actually changed this delta. BTreeSet preserves the BTreeMap
    /// re-eval order contract (a re-pin can feed another force → order matters).
    pub force_net_to_forces: std::collections::BTreeMap<u32, std::collections::BTreeSet<u32>>,
    /// C-FORCE-REEVAL-p2 sidecar: force keys (target nets) that must ALWAYS
    /// re-evaluate regardless of net changes — a force whose RHS is `volatile`
    /// (reads `$time`/`$realtime`/`$stime`/`$random`/`$urandom`/`$urandom_range`,
    /// which yield a fresh value each delta) OR reads ZERO design nets (e.g.
    /// `force x = 5;` re-pin is cheap and a const RHS never appears in the
    /// net→forces map, so it would otherwise never re-eval). Net-sensitivity
    /// skipping such a force would silently FREEZE it.
    pub force_always_reeval: std::collections::BTreeSet<u32>,
    /// Proc-assigns displaced by an overriding `force` (§9.3.1): net → the
    /// parked (lvalue, rhs ExprId, time-mult). `release` re-pins from here.
    pub latent_assigns: std::collections::BTreeMap<u32, (sim_ir::Lvalue, u32, u64)>,
    /// v5 (C): dynamic-storage heap, NetId-indexed (`None` == missing entry,
    /// the lazy empty-object contract). Flat `Vec<Option<DynObj>>` sized to
    /// `ir.nets.len()` — same memory class as `nets`/`dirty`/`forced`/
    /// `two_state` (per-net), with zero `DynObj` allocs for non-dyn designs.
    /// Ordering is never observed (every access is a point op on a single
    /// HANDLE NetId), so the flat layout is byte-identical to the old BTreeMap.
    /// Dyn objects never live in the flat BitPacked store.
    /// `RefCell` (§4.5.194): the dynamic heap is mutated from the `&self`
    /// synchronous frame executors (`run_frame_call` / `run_task`) — a function
    /// body's `new[]` / dyn element write runs on the expression read-path —
    /// exactly as `class_heap` is already interior-mutable for `&self`
    /// value-returning method field writes. Borrow discipline: every access
    /// scopes its `borrow()`/`borrow_mut()` to the shortest span and never holds
    /// a guard across a nested call that could re-touch the heap.
    pub dyn_heap: std::cell::RefCell<Vec<Option<DynObj>>>,
    /// Warn-once latch for dyn degradations (X-size new[], OOB, …) — one
    /// W-RUN-DYN-DEGRADE per handle net, never a per-iteration spam. RefCell:
    /// the READ path (`read_net` is `&self`) must latch too.
    pub dyn_warned: std::cell::RefCell<std::collections::BTreeSet<u32>>,
    /// Per-net "is a dyn handle" bitmap (DynArray/Queue/Assoc), precomputed so
    /// the hot read/write funnels pay ONE Vec<bool> load — not an `ir.nets`
    /// kind match — per indexed access.
    pub dyn_is_handle: Vec<bool>,
    /// N3 Phase 2: per-net flag — this DynArray handle's ELEMENTS are `string`
    /// (`string s[]`). Element store/read use the `is_str` byte-string path (no
    /// bit-vector resize) and `new[]` fills "". EMPTY/all-false ⇒ golden-neutral.
    pub dyn_str_elem: Vec<bool>,
    /// IEEE 1364-2005 self-width side table — built once, immutable for the run.
    pub wt: crate::width::WidthTable,

    // ── VCD ──
    pub vcd: Option<VcdWriter<VcdSink>>,
    pub vcd_path: Option<String>,
    pub dump_pending_path: Option<String>,
    pub vcd_path_override: Option<String>,
    /// Set when the resolved dump path ends in `.fst`: the VCD is written to a
    /// temp sidecar and transcoded to this FST path at finalize (see
    /// `builtins::dumpvars` and `simulate`). `None` ⇒ a plain VCD target.
    pub fst_target: Option<String>,
    pub dumping: bool,
    pub timescale_unit: String,
    pub vcd_date: String,
    /// Per-NetId hierarchical name (`"top.dut.q"`); empty ⇒ flat `n{i}` fallback.
    pub net_names: Vec<String>,
    /// OBS-2 (`--probe`): per-NetId "is this net traced?" — EMPTY ⇒ no probing (the
    /// per-change tap is a no-op). Sized to the net table when any net is probed.
    pub probed: Vec<bool>,
    /// OBS-2: last emitted value string per probed net (change dedup + `old` field).
    pub probe_prev: Vec<Option<String>>,
    /// OBS-2: accumulated `trace.jsonl` lines (one per probed-net change), time order.
    pub trace_lines: Vec<String>,
    /// Per-ProcId time multiplier (from `SimOpts.proc_multipliers`); empty ⇒ M=1.
    pub proc_multipliers: Vec<u64>,
    /// Per-ProcId `S = 10^(prec − global)` (from `SimOpts.proc_prec_mults`);
    /// empty ⇒ S=1 ⇒ two-stage `#delay` rounding degenerates to `round(d × M)`.
    pub proc_prec_mults: Vec<u64>,
    /// StmtId → severity for `$fatal`/`$error`/`$warning`/`$info` statements
    /// (from `SimOpts.severities`); empty ⇒ no severity tasks in the design.
    pub severities: crate::SeverityTable,
    /// StmtIds of `$timeformat` calls, lowered as no-op `Display` statements
    /// (the `assert_ctl`/severity side-table pattern, from
    /// `SimOpts.timeformat_stmts`); empty ⇒ no `$timeformat` in the design.
    pub timeformat_stmts: std::collections::BTreeSet<u32>,
    /// OBS-3: StmtIds of `$vita_stage(...)` calls — intercepted (never printed).
    pub stage_stmts: std::collections::BTreeSet<u32>,
    /// OBS-3: accumulated `stage.jsonl` lines (one per `$vita_stage` call), in order.
    pub stage_lines: Vec<String>,
    /// OBS-3: monotonic stage-emission counter (the `idx` field), for alignment.
    pub stage_idx: u64,
    /// OBS-3: `true` iff `+STAGE_TRACE` is set — else `$vita_stage` is a pure no-op.
    pub stage_enabled: bool,
    /// Whole-handle copy markers (§7.10, from `SimOpts.handle_copy_stmts`):
    /// no-op Display StmtId → (dst_net, src_net); the dispatch intercept
    /// deep-clones `dyn_heap[src]` into `dyn_heap[dst]`.
    pub handle_copy_stmts: std::collections::BTreeMap<u32, (u32, u32)>,
    /// Queue-slice markers (§7.10.1, from `SimOpts.queue_slice_stmts`):
    /// no-op Display StmtIds whose args are [dst, src, a, b].
    pub queue_slice_stmts: std::collections::BTreeSet<u32>,
    /// Live `$timeformat` state (IEEE 1800 §21.3.2). `None` ⇒ the defaults:
    /// units = the global precision, 0 fraction digits, no suffix, min width 20.
    pub timeformat: Option<TfState>,
    /// Global precision exponent (power of 10, in seconds, of one simulation
    /// tick; from `SimOpts.global_prec_exp`). −9 for the 1ns/1ns base.
    pub global_prec_exp: i8,
    /// StmtId → default radix (2/8/16) for b/o/h print variants (P1-5).
    pub radixes: crate::RadixTable,
    /// Assign-rank table (§9.3.1, from `SimOpts.assign_ranks`): StmtIds of
    /// Force/Release stmts that are procedural assign/deassign (weak rank).
    pub assign_ranks: crate::AssignRankTable,
    /// Bounded-queue bounds (v6 ③, from `SimOpts.queue_bounds`): handle
    /// NetId → N. Empty ⇒ every queue unbounded.
    pub queue_bounds: crate::QueueBoundTable,
    /// Per-ProcId instance path for `%m` (P2-11); empty ⇒ flat `top` fallback.
    pub proc_scopes: Vec<String>,
    /// Unpacked-array dims for per-element VCD naming (Phase-1.x ⑤, from
    /// `SimOpts.net_dims`); an absent array falls back to 1-D 0-based names.
    pub net_dims: crate::NetDimsTable,
    /// ⑤b: net ids selected by the FIRST `$dumpvars` call's depth/scope/net
    /// args. `None` ⇒ dump everything (bare `$dumpvars` / level-only forms).
    pub dump_filter: Option<std::collections::BTreeSet<u32>>,
    /// ⑤b: W4021 once-latch for second-and-later `$dumpvars` calls.
    pub dump_multi_warned: bool,
    /// v7 random state — `Cell`s so the read-phase eval (`&self`) can draw:
    /// each evaluation of `$random`/`$urandom` IS a new draw (matches
    /// iverilog; a re-rendered `$monitor` re-rolls). Net/heap purity (P7a)
    /// is untouched. `$random` seed 0 = the Annex N zero-substitution path
    /// (iverilog default); `$urandom` initial state 0 is the vitamin pin.
    pub rng: RngCells,
    /// v7 runtime plusargs (from `SimOpts.plusargs`, CLI order).
    pub plusargs: Vec<String>,
    /// P2-E `final` ProcIds (from `SimOpts.final_procs`).
    pub final_procs: std::collections::BTreeSet<u32>,
    // ── N4 clocking (from the clocking sidecars; EMPTY ⇒ no clocking) ──
    /// Source NetIds snapshotted into `preponed_buf` at each time advance (the
    /// true preponed value, before any slot activity).
    pub clocking_inputs: Vec<u32>,
    /// Marked commit-handler ProcId → `[(holding_net, source_net)]`. When such a
    /// process fires (its clocking edge), the engine commits `preponed_buf[source]
    /// → holding` (blocking, same-slot — no NBA lag).
    pub clocking_commit: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// N4 clocking output pairs: ProcId → `[(source_net, holding_net)]`. EMPTY ⇒ no outputs.
    pub clocking_outputs: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// S1 gate/assign rise·fall·turnoff delay: cont-assign index → (rise, fall,
    /// turnoff). EMPTY ⇒ uniform/no delays ⇒ byte-identical.
    pub ca_delays: std::collections::BTreeMap<u32, (u32, u32, u32)>,
    /// Preponed snapshot: source NetId → its value at the start of the time slot.
    pub preponed_buf: std::collections::BTreeMap<u32, crate::value::Value>,
    /// SVA-REST: StmtIds of assertion FIRE reports gated by assertion control.
    pub assert_fire: std::collections::BTreeSet<u32>,
    /// SVA-REST: `$assertoff/on/kill` control-site StmtId → kind (0=off,1=on,2=kill).
    pub assert_ctl: std::collections::BTreeMap<u32, u8>,
    /// SVA-REST: global assertion-enable state — `true` while a standing
    /// `$assertoff`/`$assertkill` suppresses gated fires (cleared by `$asserton`).
    pub assert_disabled: bool,
    /// §16.4 deferred-assert flush markers (from `SimOpts.defer_marks`): marker
    /// StmtId → region. Empty ⇒ no deferred asserts in the design.
    pub defer_marks: crate::DeferMarkTable,
    /// §16.4 deferred-assert actions (from `SimOpts.defer_acts`): action StmtId →
    /// (marker StmtId, region).
    pub defer_acts: crate::DeferActTable,
    /// v7 file I/O: fd-form table (`$fopen(name, mode)` → 0x8000_0003…).
    /// 0x8000_0000..=0x8000_0002 are reserved (stdin/stdout/stderr).
    pub files: std::collections::BTreeMap<u32, std::fs::File>,
    /// v7 MCD-form table (`$fopen(name)`): channel BIT index → file.
    /// Bit 0 is stdout; opens take bits from 1 (iverilog parity).
    pub mcd_files: std::collections::BTreeMap<u32, std::fs::File>,
    /// Next fd-form counter (low bits; the returned fd is 0x8000_0000|n).
    pub next_fd: u32,
    /// W4022 once-per-descriptor latch (bad/closed fd writes).
    pub bad_fd_warned: std::collections::BTreeSet<u32>,
    /// v9 per-fd read bookkeeping (Medium-bundle rank 5, SYS-READ): lazy EOF
    /// flag + `$ungetc` pushback stack. Keyed by the FULL fd (0x8000_0000|n),
    /// mirroring `files`. The OS file offset on `std::fs::File` carries the read
    /// position; this side table adds the state that offset cannot express.
    pub read_state: std::collections::BTreeMap<u32, FdReadState>,
    /// v9 fd-form descriptors opened with READ capability (a `$fopen` mode
    /// containing 'r' or '+': r/r+/w+/a+). A plain "w"/"a" fd is write-only and
    /// is NOT in this set — `$fgetc`/`$ungetc` on it return -1 (iverilog parity)
    /// without becoming readable. Keyed by the FULL fd.
    pub readable_fds: std::collections::BTreeSet<u32>,
    /// Instance path of the process CURRENTLY executing — set per `run_process`
    /// (like `cur_time_mult`), read by the `%m` format spec.
    pub cur_scope: String,
    /// N1: stack of active frame-subroutine FuncIds (innermost last), pushed by
    /// `run_frame_call`/`run_task` while a body executes (a `u32` push is alloc-free on
    /// the hot call path). `%m` inside a subroutine body appends each id's name (from
    /// `func_names`) to `cur_scope` (IEEE §21.2.1 → `module.function`). Empty on the
    /// module-level path (byte-identical `%m`). `RefCell` because the runners are `&self`.
    pub frame_scope: std::cell::RefCell<Vec<u32>>,
    /// N1: FuncId → subroutine name, index-aligned to `ir.funcs` (`SimOpts.func_names`).
    /// Consulted only by `%m` inside a frame body. EMPTY ⇒ `%m` = the module scope.
    pub func_names: Vec<String>,
    /// Worker-thread budget (from `SimOpts.threads`); `≥2` ⇒ VCD writer thread.
    pub threads: u32,
    /// Process-body execution backend (from `SimOpts.backend`). Default
    /// `Interpreter`; read at the single `run()` body-dispatch seam.
    pub backend: crate::Backend,
    /// Out-of-band bytecode-VM compile cache (Stage C, P0a), one slot per template
    /// (`ir.processes`). Decides codegen-ability ONCE and memoizes the `CompiledBody`;
    /// fork children sharing a template share its compile. NEVER enters the frozen
    /// `SimIr`. Used only on the `Bytecode` backend (`Unchecked` is a no-cost default).
    pub vm_cache: Vec<crate::backend::VmSlot>,
    /// Multiplier of the process CURRENTLY executing — set per `run_process`, read by
    /// `$time`/`$realtime`. 1 outside any process (the 1ns/1ns base).
    pub cur_time_mult: u64,
    /// `S = 10^(prec − global)` of the ACTIVE process's module (set per
    /// activation, like `cur_time_mult`): one module-precision step in global
    /// ticks — the two-stage `#delay` grain.
    pub cur_prec_mult: u64,

    // ── stdout for $display/$write (boxed sink, deterministic) ──
    pub out: Box<dyn Write + 'a>,

    // ── status flags ──
    pub finished: bool,
    /// r18 (F2): interior-mutable so a `$error` reached inside a synchronous `&self`
    /// frame function / subset-task body (`frame_emit_severity`) can flag it — the same
    /// pattern as `call_fatal`. `$error` in a module process still writes it via the
    /// normal `&mut` path.
    pub had_error: Cell<bool>,
    pub had_fatal: bool,
    /// CLASS-HEAP-CAP: max live class objects before a graceful fatal fires
    /// (see `SimOpts::max_class_objs`). Set by `simulate()` from the opts; the
    /// `SimState::new` default is the same 1M so engine-direct tests are bounded.
    pub max_class_objs: u64,

    // ── runtime diagnostics ──
    /// Direct handle to the diagnostic sink (same `&dyn LogSink` the `out` writer
    /// wraps), so the engine can emit runtime diagnostics (E-RUN-RANGE) — not just
    /// `$display` text. Interior mutability: `emit` takes `&self`.
    pub sink: &'a dyn LogSink,
    /// Rate-limit counter for E-RUN-RANGE (an OOR access in a loop would spam).
    pub run_range_count: Cell<u32>,

    // ── postponed region ($strobe FIFO + global $monitor singleton) ──
    pub postponed: Postponed,

    // ── B1 frame-call (automatic/recursive functions) ──
    /// Frame-call metadata (from `SimOpts.func_table`), index-aligned to
    /// `ir.funcs`. EMPTY ⇒ no frame functions ⇒ the routing tables stay all-false
    /// and every read/width/VCD path is byte-identical.
    pub func_table: crate::FuncTable,
    /// DERIVED per-net "is a frame-local" bitmap (len `ir.nets.len()`), built in
    /// `build_func_routing` from `func_table`. Hot-path sibling of `dyn_is_handle`;
    /// always full-length so `read_net` indexes it directly (no `is_empty` guard).
    pub frame_local: Vec<bool>,
    /// DERIVED net → `(func_idx, slot)` routing (len `ir.nets.len()`); `slot =
    /// net - base_net`. `None` for non-frame nets.
    pub frame_route: Vec<Option<(u32, u32)>>,
    /// AUTOMATIC call windows (LIFO stack); each window is a `Vec<Value>` of
    /// `locals_len` slots. `RefCell` because the function evaluator runs on the
    /// `&self` read path. Empty steady-state.
    pub frame_stack: std::cell::RefCell<Vec<Vec<Value>>>,
    /// STATIC per-function persistent slabs (FuncId → `Vec<Value>`), X-init once,
    /// never restored (shared-slot lifetime). `BTreeMap` = deterministic; never
    /// serialized.
    pub static_store: std::cell::RefCell<std::collections::BTreeMap<u32, Vec<Value>>>,
    /// Live frame-call nesting depth (runaway-recursion guard). `Cell` (no borrow).
    pub call_depth: Cell<u32>,
    /// Latched by `eval_call` when `call_depth` hits the cap (a `&self` path
    /// cannot return a `Step`); the scheduler converts it to `FinishReason::Error`.
    pub call_fatal: Cell<bool>,
    /// B2 frame-call: process-body task-call sites, keyed by `(proc template,
    /// process-local block id)`. The `&mut` executor's `Terminator::Call` arm
    /// runs the task and writes its outputs to the (possibly module-net) caller
    /// lvalues. Empty ⇒ no frame-task calls.
    pub task_calls_proc: crate::TaskCallProc,
    /// Round-14 V3/V4: the set of suspendable TASK `FuncId`s (recomputed at startup via
    /// `sim_ir::compute_suspendable_tasks`). `run_process`'s `Terminator::Call` arm
    /// routes a call to one of these through the call-stack path instead of the
    /// synchronous `run_task_call`. Empty ⇒ no suspendable tasks (byte-identical).
    pub suspendable_tasks: std::collections::BTreeSet<u32>,
    /// B2 frame-call: nested task-call sites (in task bodies), keyed by GLOBAL
    /// `ir.blocks` index. `run_task` consults this; outputs must be frame-local.
    pub task_calls_func: crate::TaskCallFunc,
    /// B4 frame-call: per-net EFFECTIVE-automatic lifetime (len `ir.nets.len()`),
    /// derived in `build_func_routing`. A frame-local net is read/written from the
    /// per-call WINDOW when true, the persistent STATIC slab when false. For a
    /// func with no lifetime overrides this equals its `is_automatic` for every
    /// slot (byte-identical to B1/B2).
    pub frame_slot_auto: Vec<bool>,
    /// B4: per-func "has ≥1 automatic slot" (len `func_table.len()`) — the frame
    /// pushes a per-call window iff true.
    pub func_has_auto: Vec<bool>,
    /// B4: per-func "has ≥1 static slot" — the frame keeps a persistent slab iff true.
    pub func_has_static: Vec<bool>,
    /// V5: per-func "has ≥1 frame-local DYNAMIC-array slot" — gates the reentry
    /// guard + free-at-exit scan (`frame_dyn_reentry_ok`/`frame_dyn_free`) so a frame
    /// with no dyn local pays nothing.
    pub func_has_dyn_local: Vec<bool>,

    // ── ⓑ-breadth (v17): array-method `with`-clause iterator scratch ──────────
    /// The current `(element value, 0-based index)` while a reduction/locator fold
    /// evaluates its `with` expression. `RefCell` because the fold runs on the
    /// `&self` read path. `None` outside a fold → `ArrayItem` reads X (defensive).
    pub array_item_scratch: std::cell::RefCell<Option<(Value, u64)>>,
    // ── N7 class/OOP (sibling of `dyn_heap`; never in the frozen SimIr) ──
    /// Class objects, keyed by a MONOTONIC allocation-id (NOT a net-id — multiple
    /// handles may alias one object and a handle may be re-`new`ed). A handle net
    /// holds `obj_id` in the flat store (0 = `null`). Empty unless classes used.
    /// `RefCell` because a value-returning METHOD evaluates on the `&self` read
    /// path yet may write object fields (sibling of `frame_stack`'s interior
    /// mutability); the borrow is always scoped to a single read/insert.
    pub class_heap: std::cell::RefCell<std::collections::BTreeMap<u32, ClassObj>>,
    /// Next allocation-id (monotonic, never recycled → no use-after-free aliasing
    /// hazard). Starts at 1 so 0 stays reserved for `null`. `Cell`: the alloc
    /// happens in the `&mut` write phase but the counter bump needs no borrow.
    pub class_obj_next: Cell<u32>,
    /// Per-class field layout (indexed by `class_id`), from `SimOpts.class_layouts`.
    /// Drives `new` default-init + field resize-on-write. Empty ⇒ no classes.
    pub class_layouts: Vec<ClassLayout>,
    /// N7-REST: per-class `rand` field bounds (from `SimOpts.class_rand`), indexed
    /// by class_id: `[(field_id, width, signed, lo, hi, ranged)]`. Drives
    /// `randomize()`. Empty ⇒ no rand members.
    pub class_rand: Vec<Vec<crate::RandBound>>,
    /// N7-REST B2: per-class constraint predicates (postfix programs), the
    /// rejection-sampling solver's acceptance test. Indexed by class_id.
    pub class_constraints: Vec<Vec<Vec<sim_ir::COp>>>,
    /// N7-REST B2: per-class `dist` weighted distributions.
    pub class_dist: Vec<Vec<crate::DistField>>,
    /// N7-REST B2: per-class `randc` fields.
    pub class_randc: Vec<Vec<crate::RandcField>>,
    /// N7-REST B-CRV final: per-call inline `randomize() with {…}` constraints,
    /// indexed by the with-id Const arg of each `ClassRandomize`.
    pub randomize_with: Vec<crate::RandWithCall>,
    /// N7-REST B2: per-(object,field) `randc` cyclic state — (permutation, position).
    pub randc_state: std::collections::HashMap<(u32, u32), (Vec<i64>, usize)>,
    /// N7-REST: the deterministic `randomize()` draw seed. A dedicated stream
    /// (separate from `$random`/`$urandom`) so randomize() draws are reproducible
    /// and isolated. `Cell` — advanced in the `&mut` write phase but borrow-free.
    pub randomize_seed: Cell<u32>,
    /// Per-net "is a class handle" bitmap (len `ir.nets.len()`), built in
    /// `simulate()` from `SimOpts.class_handle_nets`. A field-select read/write of
    /// such a net routes to the heap; a bare (word-less) read is the integer id.
    /// EMPTY/all-false ⇒ byte-identical for every prior design.
    pub class_is_handle: Vec<bool>,
    /// `new` allocation sites (StmtId → class_id), from `SimOpts.class_new_sites`.
    /// The tagged blocking-assign allocates a fresh `ClassObj` and writes its id.
    pub class_new_sites: std::collections::BTreeMap<u32, u32>,
    /// Virtual-dispatch table: `class_vtable[class_id][vslot]` = concrete method
    /// FuncId (most-derived override). From `SimOpts.class_vtable`. Empty ⇒ no
    /// virtual methods.
    pub class_vtable: Vec<Vec<u32>>,
    /// Per method-CALL-site dispatch info, indexed by call-ExprId (CLS-CALL-VEC):
    /// `Some((vslot, static_fid))` at a call site. A `Some(vslot)` triggers dynamic
    /// dispatch via `class_vtable[class][vslot]`. EMPTY for non-class designs (an
    /// out-of-range `get` returns `None` ⇒ no dispatch, byte-identical).
    pub class_calls: Vec<Option<(Option<u32>, u32)>>,
    /// Warn-once latch for null/X-handle dereference, per handle net (sibling of
    /// `dyn_warned`). `RefCell`: the READ path (`&self`) must latch too.
    pub class_null_warned: std::cell::RefCell<std::collections::BTreeSet<u32>>,
    /// FMT-CACHE: lazily memoized decode of each format/string `Const` (indexed
    /// by ConstId) so a `$display`/`$write` in a loop unpacks the packed const
    /// once instead of every call. `RefCell` because the format path runs on the
    /// read side (`&self`); byte-identical (`const_string` is a pure fn of cid).
    pub fmt_cache: std::cell::RefCell<Vec<Option<Box<str>>>>,
}

/// A heap-allocated class object (N7). `class_id` is the DYNAMIC type (set at
/// `new`, never changed by handle ref-copy) — it drives virtual dispatch. Fields
/// are a flat `Vec<Value>` indexed by a stable field-id assigned at elaborate
/// (base-class fields FIRST, so a derived object up-cast to its base reads the
/// same field-ids).
#[derive(Debug, Clone)]
pub struct ClassObj {
    pub class_id: u32,
    pub fields: Vec<Value>,
}

/// Per-class field layout (N7): field-id → `(width, signed, four_state)` in
/// stable field-id order (base-class fields first). Drives `new` default-init +
/// field resize-on-write. A 4-state field defaults to X; a 2-state field
/// (`int`/`bit`/handle) defaults to 0 (IEEE §8.8 class-property defaults).
#[derive(Debug, Clone, Default)]
pub struct ClassLayout {
    pub fields: Vec<(u32, bool, bool)>,
    /// SW1: per-field folded initializer (`int x = 42`), parallel to `fields`.
    /// `Some(bits)` overrides the bare type default at `new`; `None` = default.
    pub inits: Vec<Option<sim_ir::BitPacked>>,
}

impl ClassLayout {
    fn field_width(&self, i: u32) -> (u32, bool) {
        self.fields
            .get(i as usize)
            .map(|&(w, s, _)| (w, s))
            .unwrap_or((1, false))
    }
    /// Is field `i` a 4-state type (`logic`/`reg`/…)? A 2-state field (`bit`/`byte`/…)
    /// coerces a written X/Z→0 (§6.11.3). Default `true` (4-state) for an unknown field
    /// id ⇒ NO coercion (conservative — never corrupt an out-of-layout write).
    fn field_four_state(&self, i: u32) -> bool {
        self.fields
            .get(i as usize)
            .map(|&(_, _, fs)| fs)
            .unwrap_or(true)
    }
    fn default_value(&self, i: u32) -> Value {
        // SW1 (IEEE §8.8): a folded declaration initializer wins over the bare
        // type default (2-state→0 / 4-state→X).
        if let Some(Some(bits)) = self.inits.get(i as usize) {
            let (w, s) = self.field_width(i);
            return Value::from_packed(bits, w.max(1), s);
        }
        match self.fields.get(i as usize) {
            Some(&(w, s, four)) if four => Value::xs(w.max(1), s),
            Some(&(w, s, _)) => Value::zeros(w.max(1), s),
            None => Value::xs(1, false),
        }
    }
}

/// Shared element-count cap for dynamic storage (same hazard class as
/// elaborate's `MAX_ARRAY_LEN`, P2-6): a runtime OOM from `new[huge]` or a
/// runaway push loop is as silent-deadly as the t0 one. NO silent caps — every
/// clamp/drop warns (W4020, once per net).
pub(crate) const MAX_DYN_ELEMS: usize = 1 << 24;

/// v5 (C): one dynamic-storage object. Engine-internal RUNTIME state — never
/// serialized, never in the frozen IR (design doc 2026-06-10 §2).
#[derive(Debug, Clone)]
pub enum DynObj {
    /// `int d[]` — element values, length set only by `new[n]`/`delete()`.
    DynArray { elems: Vec<Value> },
    /// `int q[$]` — pushes/pops at both ends, `q[size] = v` appends (§7.10.1).
    Queue {
        elems: std::collections::VecDeque<Value>,
    },
    /// `int a[longint]` — signed-i64 key domain (⑥ elaborate casts the surface
    /// key type before the IR). BTree = deterministic order for any future
    /// iteration/dump surface (first/next land post-MVP for free).
    Assoc {
        map: std::collections::BTreeMap<i64, Value>,
    },
    /// v7 P2-C `string s` — raw bytes (a missing entry IS "" — lazy, like
    /// every dyn object).
    Str { bytes: Vec<u8> },
    /// `int a[string]` (v6) — raw-byte-string keys (leading-0x00-stripped
    /// packed ASCII). BTree byte order = IEEE lexicographic string compare,
    /// so first/next iterate in the §7.9.4 order for free.
    AssocStr {
        map: std::collections::BTreeMap<Vec<u8>, Value>,
    },
}

impl DynObj {
    pub fn len(&self) -> usize {
        match self {
            DynObj::DynArray { elems } => elems.len(),
            DynObj::Queue { elems } => elems.len(),
            DynObj::Assoc { map } => map.len(),
            DynObj::AssocStr { map } => map.len(),
            DynObj::Str { bytes } => bytes.len(),
        }
    }
    /// Clippy pairing for `len`.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Runaway-recursion guard for the frame-call evaluator. It caps the LOGICAL
/// nesting of `eval_call` re-entries; hitting it latches `call_fatal` → the
/// scheduler ends with `FinishReason::Error` (same exit class as user `$fatal`).
///
/// The deep-recursion corpus runs on a large (256 MiB) worker stack — the CLI
/// driver (crates/cli/src/main.rs) and the frame_call test harness both spawn
/// one — so this cap, NOT a host stack overflow, is the guard. For that to hold
/// on EVERY OS the cap must be small enough that `MAX_CALL_DEPTH` re-entries fit
/// that stack on the platform with the FATTEST frames: each `eval_call` frame is
/// ~4 KiB on macOS but ~2.5× that on Windows debug builds, so the old 65536
/// (~0.5 GiB on Windows) overflowed 256 MiB and aborted BEFORE the cap fired
/// (Windows CI stack overflow). 8192 ⇒ ≤ ~80 MiB worst-case (≈3× headroom in
/// 256 MiB), yet still 4× the deepest legal-recursion test (`cnt(2000)`) and far
/// beyond any real RTL recursion. (Raising it requires raising BOTH worker
/// stacks in lockstep.)
pub(crate) const MAX_CALL_DEPTH: u32 = 8192;

/// RAII decrement of `call_depth` on EVERY exit of `eval_call` (normal return,
/// fatal-return, or a panic unwinding through it) so a missed early-return can
/// never permanently inflate the live nesting count. `Cell` (not `RefCell`):
/// `get`/`set` never borrow, so the guard never contends with the read path.
struct DepthGuard<'c>(&'c Cell<u32>);

impl Drop for DepthGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// N1: RAII pop of the `%m` frame-scope stack on EVERY exit of a frame body.
struct FrameScopeGuard<'c>(&'c std::cell::RefCell<Vec<u32>>);

impl Drop for FrameScopeGuard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().pop();
    }
}

// ── free helpers ───────────────────────────────────────────────────────────

/// v7 RNG state cells (see `SimState::rng`).
#[derive(Default)]
pub(crate) struct RngCells {
    /// `$random` Annex-N LCG seed (global generator, like iverilog's).
    pub random: std::cell::Cell<u32>,
    /// `$urandom` splitmix64 state (vitamin-pinned sequence).
    pub urandom: std::cell::Cell<u64>,
}
