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
    pub dyn_heap: Vec<Option<DynObj>>,
    /// Warn-once latch for dyn degradations (X-size new[], OOB, …) — one
    /// W-RUN-DYN-DEGRADE per handle net, never a per-iteration spam. RefCell:
    /// the READ path (`read_net` is `&self`) must latch too.
    pub dyn_warned: std::cell::RefCell<std::collections::BTreeSet<u32>>,
    /// Per-net "is a dyn handle" bitmap (DynArray/Queue/Assoc), precomputed so
    /// the hot read/write funnels pay ONE Vec<bool> load — not an `ir.nets`
    /// kind match — per indexed access.
    pub dyn_is_handle: Vec<bool>,
    /// IEEE 1364-2005 self-width side table — built once, immutable for the run.
    pub wt: crate::width::WidthTable,

    // ── VCD ──
    pub vcd: Option<VcdWriter<VcdSink>>,
    pub vcd_path: Option<String>,
    pub dump_pending_path: Option<String>,
    pub vcd_path_override: Option<String>,
    pub dumping: bool,
    pub timescale_unit: String,
    pub vcd_date: String,
    /// Per-NetId hierarchical name (`"top.dut.q"`); empty ⇒ flat `n{i}` fallback.
    pub net_names: Vec<String>,
    /// Per-ProcId time multiplier (from `SimOpts.proc_multipliers`); empty ⇒ M=1.
    pub proc_multipliers: Vec<u64>,
    /// StmtId → severity for `$fatal`/`$error`/`$warning`/`$info` statements
    /// (from `SimOpts.severities`); empty ⇒ no severity tasks in the design.
    pub severities: crate::SeverityTable,
    /// StmtIds of `$timeformat` calls, lowered as no-op `Display` statements
    /// (the `assert_ctl`/severity side-table pattern, from
    /// `SimOpts.timeformat_stmts`); empty ⇒ no `$timeformat` in the design.
    pub timeformat_stmts: std::collections::BTreeSet<u32>,
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

    // ── stdout for $display/$write (boxed sink, deterministic) ──
    pub out: Box<dyn Write + 'a>,

    // ── status flags ──
    pub finished: bool,
    pub had_error: bool,
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

impl<'a> SimState<'a> {
    pub fn new(
        ir: &'a SimIr,
        out: Box<dyn Write + 'a>,
        sink: &'a dyn LogSink,
        timescale_unit: String,
        vcd_date: String,
        vcd_path_override: Option<String>,
    ) -> Self {
        let nets = ir
            .nets
            .iter()
            .map(|nv| {
                let alen = nv.array_len.max(1);
                let total = (nv.width as usize) * (alen as usize);
                let init = expand_init(&nv.init, nv.width, alen, total);
                NetSlot {
                    cur: init.clone(),
                    prev: init,
                    width: nv.width,
                    array_len: alen,
                    signed: nv.signed,
                    is_real: nv.kind == NetKind::Real,
                    vcd_id: None,
                    vcd_word_ids: Vec::new(),
                }
            })
            .collect();
        // Built with an EMPTY func table here; `simulate()` rebuilds with the real
        // `func_table` once it is installed (a from-`new()` SimState keeps today's
        // behavior since no `Expr::Call` is reachable without a frame func).
        let wt = crate::width::WidthTable::build(ir, &crate::FuncTable::new());
        let nnets = ir.nets.len();
        // GLITCH: a net is an edge target if it appears in a STATIC edge
        // sensitivity (`@(posedge clk …)` on an `always`) OR in a PROCEDURAL
        // edge wait (`@(posedge x)` inside a body → `Terminator::Wait` carrying
        // `WaitCause::Edge`). Both are compile-time-fixed net ids, so one scan
        // at construction yields the complete set; `slot_edge` is then only
        // maintained/consulted for these nets (every other net byte-identical).
        let mut is_edge_target = vec![false; nnets];
        let mark_edge = |net: u32, set: &mut Vec<bool>| {
            if (net as usize) < nnets {
                set[net as usize] = true;
            }
        };
        for p in &ir.processes {
            // Static edge sensitivity (`always @(posedge clk …)`).
            if p.sensitivity.kind == sim_ir::SensKind::Edge {
                for et in &p.sensitivity.edges {
                    mark_edge(et.net, &mut is_edge_target);
                }
            }
            // Procedural edge wait (`@(posedge x)` in this process's body — its
            // blocks are PROCESS-LOCAL in `body`, NOT in the global `ir.blocks`
            // arena, which holds only func/task bodies).
            for blk in &p.body {
                if let sim_ir::Terminator::Wait {
                    cond: sim_ir::WaitCause::Edge { net, .. },
                    ..
                } = &blk.term
                {
                    mark_edge(*net, &mut is_edge_target);
                }
            }
        }
        // Func/task bodies (global arena) may also carry an `@(edge …)` wait.
        for blk in &ir.blocks {
            if let sim_ir::Terminator::Wait {
                cond: sim_ir::WaitCause::Edge { net, .. },
                ..
            } = &blk.term
            {
                mark_edge(*net, &mut is_edge_target);
            }
        }
        SimState {
            ir,
            now: 0,
            nets,
            dirty: Vec::new(),
            dirty_flag: vec![false; nnets],
            forced: vec![false; nnets],
            two_state: vec![false; nnets],
            wired_and_nets: std::collections::BTreeSet::new(),
            wired_or_nets: std::collections::BTreeSet::new(),
            slot_edge: vec![0u8; nnets],
            is_edge_target,
            blocking_writer: None,
            last_blocking_writer: vec![u32::MAX; nnets],
            active_forces: std::collections::BTreeMap::new(),
            force_net_to_forces: std::collections::BTreeMap::new(),
            force_always_reeval: std::collections::BTreeSet::new(),
            latent_assigns: std::collections::BTreeMap::new(),
            dyn_heap: (0..nnets).map(|_| None).collect(),
            dyn_warned: std::cell::RefCell::new(std::collections::BTreeSet::new()),
            dyn_is_handle: ir
                .nets
                .iter()
                .map(|nv| {
                    matches!(
                        nv.kind,
                        NetKind::DynArray
                            | NetKind::Queue
                            | NetKind::Assoc
                            | NetKind::AssocStr
                            | NetKind::String
                    )
                })
                .collect(),
            wt,
            vcd: None,
            vcd_path: None,
            dump_pending_path: None,
            vcd_path_override,
            dumping: false,
            timescale_unit,
            vcd_date,
            net_names: Vec::new(),
            net_dims: crate::NetDimsTable::new(),
            dump_filter: None,
            dump_multi_warned: false,
            rng: RngCells::default(),
            plusargs: Vec::new(),
            final_procs: std::collections::BTreeSet::new(),
            clocking_inputs: Vec::new(),
            clocking_commit: std::collections::BTreeMap::new(),
            clocking_outputs: std::collections::BTreeMap::new(),
            ca_delays: std::collections::BTreeMap::new(),
            preponed_buf: std::collections::BTreeMap::new(),
            assert_fire: std::collections::BTreeSet::new(),
            assert_ctl: std::collections::BTreeMap::new(),
            assert_disabled: false,
            defer_marks: crate::DeferMarkTable::new(),
            defer_acts: crate::DeferActTable::new(),
            files: std::collections::BTreeMap::new(),
            mcd_files: std::collections::BTreeMap::new(),
            next_fd: 3,
            bad_fd_warned: std::collections::BTreeSet::new(),
            read_state: std::collections::BTreeMap::new(),
            readable_fds: std::collections::BTreeSet::new(),
            proc_multipliers: Vec::new(),
            severities: crate::SeverityTable::new(),
            timeformat_stmts: std::collections::BTreeSet::new(),
            handle_copy_stmts: std::collections::BTreeMap::new(),
            queue_slice_stmts: std::collections::BTreeSet::new(),
            timeformat: None,
            global_prec_exp: -9,
            radixes: crate::RadixTable::new(),
            assign_ranks: crate::AssignRankTable::new(),
            queue_bounds: crate::QueueBoundTable::new(),
            proc_scopes: Vec::new(),
            cur_scope: "top".to_string(),
            threads: 1,
            backend: crate::Backend::Interpreter,
            vm_cache: (0..ir.processes.len())
                .map(|_| crate::backend::VmSlot::Unchecked)
                .collect(),
            cur_time_mult: 1,
            out,
            finished: false,
            had_error: false,
            had_fatal: false,
            max_class_objs: 1_000_000,
            sink,
            run_range_count: Cell::new(0),
            postponed: Postponed::default(),
            func_table: crate::FuncTable::new(),
            frame_local: vec![false; nnets],
            frame_route: vec![None; nnets],
            frame_stack: std::cell::RefCell::new(Vec::new()),
            static_store: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            call_depth: Cell::new(0),
            call_fatal: Cell::new(false),
            task_calls_proc: crate::TaskCallProc::new(),
            task_calls_func: crate::TaskCallFunc::new(),
            frame_slot_auto: vec![false; nnets],
            func_has_auto: Vec::new(),
            func_has_static: Vec::new(),
            array_item_scratch: std::cell::RefCell::new(None),
            class_heap: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            class_obj_next: Cell::new(1),
            class_layouts: Vec::new(),
            class_rand: Vec::new(),
            class_constraints: Vec::new(),
            class_dist: Vec::new(),
            class_randc: Vec::new(),
            randomize_with: Vec::new(),
            randc_state: std::collections::HashMap::new(),
            // A fixed nonzero start so the first draw is well-defined and the
            // sequence is reproducible on every OS (dist_uniform substitutes 0
            // anyway, but pinning it keeps the contract explicit).
            randomize_seed: Cell::new(1),
            class_is_handle: vec![false; nnets],
            class_new_sites: std::collections::BTreeMap::new(),
            class_vtable: Vec::new(),
            class_calls: Vec::new(),
            class_null_warned: std::cell::RefCell::new(std::collections::BTreeSet::new()),
            fmt_cache: std::cell::RefCell::new(vec![None; ir.consts.len()]),
        }
    }

    /// FMT-CACHE: decode a format/string const, memoized by ConstId. The
    /// expensive bit-unpack + UTF-8 validation (`const_string`) runs once per
    /// const; later calls clone the cached text. Byte-identical to calling
    /// `const_string(self.ir, cid)` directly.
    pub(crate) fn fmt_const_string(&self, cid: u32) -> String {
        let mut cache = self.fmt_cache.borrow_mut();
        let slot = &mut cache[cid as usize];
        if slot.is_none() {
            *slot = Some(crate::builtins::const_string(self.ir, cid).into_boxed_str());
        }
        slot.as_deref().unwrap().to_string()
    }

    /// B1: derive the per-net frame-local routing tables from `func_table` (the
    /// `dyn_is_handle` build pattern). VALIDATES the hand-built/elaborated sidecar
    /// (index alignment + in-range slots) and latches a loud fatal rather than
    /// risking an out-of-bounds panic on a malformed table. No-op for an EMPTY
    /// table (every existing design byte-identical).
    pub fn build_func_routing(&mut self) {
        let nnets = self.ir.nets.len();
        self.frame_local = vec![false; nnets];
        self.frame_route = vec![None; nnets];
        // B4: per-net EFFECTIVE-automatic flag + per-func storage needs.
        self.frame_slot_auto = vec![false; nnets];
        self.func_has_auto = vec![false; self.func_table.len()];
        self.func_has_static = vec![false; self.func_table.len()];
        if self.func_table.is_empty() {
            return;
        }
        if self.func_table.len() != self.ir.funcs.len() {
            self.fatal_run("frame-call func_table length != ir.funcs length");
            return;
        }
        for (fi, m) in self.func_table.iter().enumerate() {
            // The return-slot check applies only to FUNCTIONS (a task has no
            // func-named return var; `return_slot` is unused/0 for `is_task`).
            let is_task = self.ir.funcs.get(fi).map(|f| f.is_task).unwrap_or(false);
            let return_bad = !is_task && m.locals_len > 0 && m.return_slot >= m.locals_len;
            if return_bad || (m.base_net as usize) + (m.locals_len as usize) > nnets {
                self.fatal_run("frame-call FuncMeta slot range out of bounds");
                return;
            }
            for slot in 0..m.locals_len {
                let net = (m.base_net + slot) as usize;
                self.frame_local[net] = true;
                self.frame_route[net] = Some((fi as u32, slot));
                // B4: a slot is EFFECTIVE-automatic iff its override bit is set OR
                // the function/task default is automatic (bit 0 for slots ≥ 64).
                let overridden = slot < 64 && (m.auto_override >> slot) & 1 == 1;
                let auto = overridden || m.is_automatic;
                self.frame_slot_auto[net] = auto;
                if auto {
                    self.func_has_auto[fi] = true;
                } else {
                    self.func_has_static[fi] = true;
                }
            }
        }
    }

    /// Stage-C VM dispatch (P0a): return the cached `CompiledBody` for template `tmpl`
    /// if it is codegen-able, compiling + caching on first sight; `None` ⇒ interpret.
    /// The decision is made ONCE per template (the per-fire `is_codegen_able` scan is
    /// removed). The returned `Rc` is an OWNED handle (cloned out of the cache) so the
    /// caller can pass `&mut self` as the `Kernel` to `vm_exec` without aliasing the
    /// cache — the §2.3 borrow protocol.
    pub(crate) fn vm_compiled(&mut self, tmpl: usize) -> Option<Rc<crate::backend::CompiledBody>> {
        use crate::backend::VmSlot;
        match &self.vm_cache[tmpl] {
            VmSlot::Compiled(rc) => return Some(Rc::clone(rc)),
            VmSlot::NotCodegenable => return None,
            VmSlot::Unchecked => {}
        }
        // `self.ir` is a `&SimIr` field — copy the reference out so the immutable read
        // of the IR does not borrow `self` across the `self.vm_cache` write below.
        let ir: &SimIr = self.ir;
        if !crate::backend::is_codegen_able(&ir.stmts, &ir.exprs, &ir.processes[tmpl].body) {
            self.vm_cache[tmpl] = VmSlot::NotCodegenable;
            return None;
        }
        let compiled = Rc::new(crate::backend::compile_body(
            &ir.stmts,
            &ir.processes[tmpl].body,
            Some((ir, &self.wt)),
        ));
        self.vm_cache[tmpl] = VmSlot::Compiled(Rc::clone(&compiled));
        Some(compiled)
    }

    /// Emit a rate-limited `E-RUN-RANGE` (VITA-E4002) runtime diagnostic for an
    /// out-of-range array word / select. The OOR is RECOVERED (read X / drop write),
    /// so the run still finishes; this only surfaces it. Capped at `CAP` per run with
    /// a final "further suppressed" note so a loop of OOR accesses can't spam.
    pub fn warn_run_range(&self, what: &str) {
        const CAP: u32 = 8;
        let n = self.run_range_count.get();
        let msg = match n.cmp(&CAP) {
            std::cmp::Ordering::Less => {
                Some(format!("{what} (out of range; read X / write ignored)"))
            }
            std::cmp::Ordering::Equal => {
                Some("further out-of-range diagnostics suppressed".to_string())
            }
            std::cmp::Ordering::Greater => None,
        };
        if let Some(message) = msg {
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Error,
                code: MsgCode::RunRange,
                message,
                location: None,
                context: Vec::new(),
                sim_time: Some(diag::TimeStamp { ticks: self.now }),
            }));
        }
        self.run_range_count.set(n.saturating_add(1));
    }

    /// Emit a loud `RunFatal` (F-RUN-FATAL / exit class Fatal — same code as user
    /// `$fatal`) and latch `had_fatal`/`finished`. Used for malformed engine input
    /// that must not silently mis-route (e.g. a corrupt frame `func_table`), as a
    /// hardened alternative to a raw out-of-bounds panic.
    pub fn fatal_run(&mut self, what: &str) {
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Fatal,
            code: MsgCode::RunFatal,
            message: what.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(diag::TimeStamp { ticks: self.now }),
        }));
        self.had_fatal = true;
        self.finished = true;
    }

    /// CLASS-HEAP-CAP: the class-object budget (`max_class_objs`) was exceeded.
    /// Emit a single loud `F-RUN-CLASS-LIMIT` and latch `had_fatal`/`finished`
    /// for a graceful `$finish` (exit class Fatal) instead of an OOM — the class
    /// heap is never garbage-collected, so an unbounded `new()` in a loop would
    /// grow without limit. Single-shot (guarded by `had_fatal`).
    pub fn fatal_class_limit(&mut self) {
        if !self.had_fatal {
            self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                severity: Severity::Fatal,
                code: MsgCode::RunClassLimit,
                message: format!(
                    "class object budget ({}) exceeded — likely an unbounded `new()` \
                     (the class heap is not garbage-collected); raise \
                     SimOpts::max_class_objs if intended",
                    self.max_class_objs
                ),
                location: None,
                context: Vec::new(),
                sim_time: Some(diag::TimeStamp { ticks: self.now }),
            }));
        }
        self.had_fatal = true;
        self.finished = true;
    }

    // ── reads ────────────────────────────────────────────────────────────

    // ── writes (single choke point) ──────────────────────────────────────

    /// Write `value` (any width) into the LHS chunks of `lhs`, MSB-first source
    /// consumption (Verilog concat-LHS). Returns true if ANY bit changed.
    ///
    /// `offsets[i]` is the already-EVALUATED bit offset of `lhs.chunks[i]` — the
    /// runtime value of a dynamic index (`a[i]`) or the const for `a[3]`; ignored
    /// for a whole-net/`None` chunk. The caller resolves these at the correct
    /// sampling moment (statement time for blocking, SAMPLE time for NBA so
    /// `a[i] <= x; i = i+1;` uses the OLD `i`, settle time for cont-assign),
    /// because this `&mut self` path has no read-only `EvalCtx`.
    pub fn write_lvalue(
        &mut self,
        lhs: &Lvalue,
        value: Value,
        offsets: &crate::exec::Offsets,
    ) -> bool {
        // ── real↔int assignment coercion (IEEE 1364 §6.2) ──
        // Only a WHOLE-NET lvalue (single Bit chunk, no offset/width) can be a
        // real destination: a real is dimensionless and never bit/part-selected
        // (§6.2 makes r[i]/r[hi:lo] illegal at elaborate). Detect the whole-net
        // case and consult NetSlot.is_real.
        let dest_is_real = lhs.chunks.len() == 1
            && matches!(lhs.chunks[0].kind, SelKind::Bit)
            && lhs.chunks[0].offset.is_none()
            && lhs.chunks[0].width.is_none()
            && self.nets[lhs.chunks[0].net as usize].is_real;

        let value = match (dest_is_real, value.is_real) {
            // real net ← real value: store verbatim (already 64 IEEE bits).
            (true, true) => value,
            // real net ← integer value (int→real CONVERT): exact for ≤53-bit.
            (true, false) => Value::from_f64(value.to_f64().unwrap_or(0.0)),
            // integer net ← real value (real→int ASSIGNMENT: ROUND half-away).
            // A real RHS only legally targets a whole scalar int net (concat-LHS
            // of a real is illegal §6.2). Round to that net's width; for the rare
            // multi-chunk case round to the total LHS width.
            (false, true) => {
                let w = if lhs.chunks.len() == 1 {
                    self.nets[lhs.chunks[0].net as usize].width
                } else {
                    lhs.chunks.iter().map(|c| self.chunk_width(c)).sum()
                };
                let signed = lhs.chunks.len() == 1 && self.nets[lhs.chunks[0].net as usize].signed;
                crate::value::real_to_int_round(value.to_f64().unwrap_or(0.0), w.max(1), signed)
            }
            // integer net ← integer value: unchanged legacy path.
            (false, false) => value,
        };

        // ── v5 ⑤: assoc-element lane (single chunk, i64 key) ──
        // The key cannot ride the u32 pairs; `resolve_lvalue_offsets` claims
        // the shape and the funnel splits here, AFTER the real coercion (an
        // assoc element gets the same real→int rounding as a dyn element).
        if let crate::exec::Offsets::AssocKey(key) = offsets {
            if let Some(c) = lhs.chunks.first() {
                self.assoc_write(c.net, *key, &value);
            }
            return false; // dyn content never enters the dirty channel
        }
        // v6: the string-keyed twin lane.
        if let crate::exec::Offsets::AssocStrKey(key) = offsets {
            if let Some(c) = lhs.chunks.first() {
                self.assoc_str_write(c.net, key, &value);
            }
            return false;
        }
        let offsets = offsets.as_slice();
        debug_assert_eq!(
            offsets.len(),
            lhs.chunks.len(),
            "one (offset,word) per chunk"
        );

        // v7 P2-C: a STRING destination takes the WHOLE source value — its
        // net-table width is 0 (dynamic), so the total-width resize below
        // would chop the value to 1 bit; §6.16 conversion is dyn_write's
        // byte-strip, never a bit resize.
        if let [c] = lhs.chunks.as_slice() {
            if self.ir.nets[c.net as usize].kind == NetKind::String {
                let (raw_off, raw_word) = offsets.first().copied().unwrap_or((0, 0));
                return self.write_chunk(c, raw_off, raw_word, &value);
            }
        }
        // Total destination bit width = sum of chunk widths.
        let total: u32 = lhs.chunks.iter().map(|c| self.chunk_width(c)).sum();
        let src = value.resize(total.max(1));

        // Single-chunk LHS — the dominant case (a whole net, or a single bit-/part-
        // select). With one chunk, `take_lo == 0` and `cw == total`, so the sliced
        // "piece" IS `src` exactly; skip the per-bit slice entirely and hand `src`
        // straight to `write_chunk` (which word-writes the whole-net/aligned case). The
        // per-bit slice loop below is only for a multi-chunk concat LHS (`{a,b} = x`).
        if lhs.chunks.len() == 1 {
            let (raw_off, raw_word) = offsets.first().copied().unwrap_or((0, 0));
            return self.write_chunk(&lhs.chunks[0], raw_off, raw_word, &src);
        }

        let mut changed = false;
        let mut src_hi = total; // next source bit (exclusive top)
        for (idx, chunk) in lhs.chunks.iter().enumerate() {
            let cw = self.chunk_width(chunk);
            let take_lo = src_hi.saturating_sub(cw);
            // slice src[take_lo .. src_hi) → low-aligned chunk value
            let mut piece = Value::zeros(cw.max(1), false);
            piece.width = cw;
            for i in 0..cw {
                let (v, u) = src.get_vu(take_lo + i);
                piece.set_vu(i, v, u);
            }
            src_hi = take_lo;
            let (raw_off, raw_word) = offsets.get(idx).copied().unwrap_or((0, 0));
            changed |= self.write_chunk(chunk, raw_off, raw_word, &piece);
        }
        changed
    }

    /// Width (in bits) a single lvalue chunk writes.
    fn chunk_width(&self, c: &sim_ir::LvalChunk) -> u32 {
        match c.kind {
            // whole-net write: offset/width None.
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    self.nets[c.net as usize].width
                } else {
                    1
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp | SelKind::PartIdxDown => {
                // `c.width` is an ExprId (frozen IR: a const-expr edge like
                // `Add(Sub(msb,lsb),1)`), NOT a literal — fold it to its value.
                c.width
                    .and_then(|eid| crate::width::const_u32_of_expr(self.ir, eid))
                    .unwrap_or_else(|| self.nets[c.net as usize].width)
            }
        }
    }

    /// Total destination bit-width of an lvalue (Σ chunk widths). Used to seed
    /// the RHS context width. Does NOT compute a sign — lhs sign never
    /// propagates (IEEE 1364-2005 assignment rule, §5.5).
    pub(crate) fn lvalue_width(&self, lhs: &Lvalue) -> u32 {
        lhs.chunks
            .iter()
            .map(|c| self.chunk_width(c))
            .sum::<u32>()
            .max(1)
    }

    /// Write a low-aligned `piece` into the destination chunk. `raw_off` is the
    /// already-EVALUATED `c.offset` (the runtime index for `a[i]`, the const for
    /// `a[3]`; ignored for a whole-net chunk). Returns changed.
    fn write_chunk(
        &mut self,
        c: &sim_ir::LvalChunk,
        raw_off: u32,
        raw_word: u32,
        piece: &Value,
    ) -> bool {
        let net = c.net as usize;
        // v5 (C)-3b: dyn-handle element write → heap (never the flat store).
        if self.dyn_is_handle[net] {
            return self.dyn_write(c, raw_word, piece);
        }
        // N7: a class-handle field-select write (`obj.f = v`, `word = field-id`)
        // goes to the heap; a bare word-less write (the handle id itself —
        // ref-copy / `h = null` / `h = new` placeholder) falls through to the
        // flat store below.
        if self.class_is_handle[net] && c.word.is_some() {
            return self.class_field_write(c, raw_word, piece);
        }
        // A forced net ignores every normal driver until release (§9.3.2).
        if self.forced[net] {
            return false;
        }
        // SVPART: a 2-state variable can never hold X/Z (IEEE §6.11.3) — coerce any
        // unknown bit of the incoming value to 0 before it lands. Only fires for the
        // (new) 2-state nets, so every prior design is byte-identical.
        let coerced;
        let piece = if self.two_state[net] && piece.unk.iter().any(|&u| u != 0) {
            let mut v = piece.clone();
            for k in 0..v.unk.len() {
                v.val[k] &= !v.unk[k]; // X (val0/unk1) & Z (val1/unk1) → 0
                v.unk[k] = 0;
            }
            coerced = v;
            &coerced
        } else {
            piece
        };
        // `c.word` is an ExprId; `raw_word` is the caller-evaluated array index
        // (the runtime `k` of `mem[k] = …`). None ⇒ index 0. An out-of-range word
        // write is IGNORED (spec E-RUN-RANGE) — clamping to the last element would
        // silently corrupt a valid neighbor.
        let word = if c.word.is_some() {
            if raw_word >= self.nets[net].array_len {
                self.warn_run_range("array word index");
                return false;
            }
            raw_word
        } else {
            0
        };
        let net_w = self.nets[net].width;
        let base = word * net_w; // bit offset of this array element

        // `c.width` is a const-expr edge (part-select bounds are constant); fold
        // it. `c.offset` was evaluated by the caller (dynamic-index capable) and
        // arrives as `raw_off`, symmetric with the runtime offset eval that
        // `eval_select` already does on the READ side.
        let ir = self.ir;
        let fold = |eid: u32| crate::width::const_u32_of_expr(ir, eid);
        // P0-IPU: the low (LSB) net-bit position is SIGNED — an underflowing indexed
        // part-select (`v[-2+:4]`, `v[1-:3]`) extends below bit 0, and only the
        // in-range bits are written (the low OOB bits are dropped, NOT shifted up or
        // wrapped). Mirror the READ side (`eval_select`): keep `lsb` an `i64` and let
        // the bit loop clamp at both ends. `raw_off` arrives as the offset's 32-bit
        // 2's-complement; sign-extend it (bit positions never exceed i32 range).
        let off_i = raw_off as i32 as i64;
        let (lsb, width) = match c.kind {
            SelKind::Bit => {
                if c.offset.is_none() && c.width.is_none() {
                    (0i64, net_w) // whole net
                } else {
                    (off_i, 1)
                }
            }
            SelKind::PartConst | SelKind::PartIdxUp => {
                (off_i, c.width.and_then(fold).unwrap_or(net_w))
            }
            SelKind::PartIdxDown => {
                let w = c.width.and_then(fold).unwrap_or(net_w);
                (off_i - (w as i64) + 1, w)
            }
        };

        // GLITCH: capture scalar bit0 BEFORE the mutation so a same-slot re-write
        // (A→B→A) records its real `B→A` transition (the endpoint compare would
        // see only A==A). Gated on `is_edge_target`, so non-clock nets pay nothing.
        let track_edge = self.is_edge_target[net];
        let old_b0 = if track_edge {
            scalar_bit0(&self.nets[net].cur)
        } else {
            sim_ir::FourState::Zero
        };

        let slot = &mut self.nets[net];

        // WORD-PARALLEL fast path: a whole-element write to a 64-aligned destination
        // (every scalar whole-net assign, plus array elements whose width is a multiple
        // of 64). Copy `piece`'s words into the store with word-granular change detection
        // + a top-word mask, replacing the per-bit `set_bit` loop. Guard:
        // `array_len <= 1 || net_w % 64 == 0` guarantees the element occupies WHOLE store
        // words, so masking the top word cannot clobber a neighbouring element packed in
        // the same word. Everything else (part/bit-select, unaligned base, OOR) falls
        // through to the proven bit-serial path below — byte-identical by construction.
        if lsb == 0 && width == net_w && base % 64 == 0 && (slot.array_len <= 1 || net_w % 64 == 0)
        {
            let wbase = (base / 64) as usize;
            let nw = nwords(net_w).max(1);
            let m = top_mask(net_w);
            let mut changed = false;
            for k in 0..nw {
                let mut nv = piece.val.get(k).copied().unwrap_or(0);
                let mut nu = piece.unk.get(k).copied().unwrap_or(0);
                if k == nw - 1 {
                    nv &= m;
                    nu &= m;
                }
                let idx = wbase + k;
                if slot.cur.val.len() <= idx {
                    slot.cur.val.resize(idx + 1, 0);
                    slot.cur.unk.resize(idx + 1, 0);
                }
                if slot.cur.val[idx] != nv || slot.cur.unk[idx] != nu {
                    slot.cur.val[idx] = nv;
                    slot.cur.unk[idx] = nu;
                    changed = true;
                }
            }
            if changed {
                self.note_change(net as u32, word);
                if track_edge {
                    self.accumulate_edge(net, old_b0);
                }
            }
            return changed;
        }

        let mut changed = false;
        for i in 0..width {
            // P0-IPU: the destination net-bit is `lsb + i` in a SIGNED domain — a bit
            // below 0 (underflow) OR at/above `net_w` (overflow) is dropped cleanly
            // (matching the READ side and iverilog), so only in-range bits are written.
            let dst = lsb + i as i64;
            if dst < 0 || dst as u32 >= net_w {
                continue;
            }
            let (v, u) = piece.get_vu(i);
            if set_bit(&mut slot.cur, base + dst as u32, v, u) {
                changed = true;
            }
        }
        if changed {
            self.note_change(net as u32, word);
            if track_edge {
                self.accumulate_edge(net, old_b0);
            }
        }
        changed
    }

    /// N4 clocking: commit a whole-net SCALAR value into a holding net (the
    /// preponed sample → `cb.sig`), blocking + same-slot, marking it changed for
    /// propagation. Mirrors `write_chunk`'s word-parallel whole-net fast path
    /// (holding nets are scalar `Reg`s: `array_len == 1`, never forced/2-state).
    pub fn commit_clocking_sample(&mut self, net: u32, v: &Value) -> bool {
        let i = net as usize;
        let net_w = self.nets[i].width;
        let nw = nwords(net_w).max(1);
        let m = top_mask(net_w);
        // GLITCH: a holding net can itself be an edge target (`@(posedge cb.sig)`),
        // so this write must feed the same intra-slot edge accumulator as the
        // normal funnel — `note_change` alone RESETS `slot_edge`, so without the
        // matching `accumulate_edge` a posedge on the committed sample would be
        // silently lost (the pre-fix endpoint compare caught it via `cur != prev`).
        let track_edge = self.is_edge_target[i];
        let old_b0 = if track_edge {
            scalar_bit0(&self.nets[i].cur)
        } else {
            sim_ir::FourState::Zero
        };
        let slot = &mut self.nets[i];
        let mut changed = false;
        for k in 0..nw {
            let mut nv = v.val.get(k).copied().unwrap_or(0);
            let mut nu = v.unk.get(k).copied().unwrap_or(0);
            if k == nw - 1 {
                nv &= m;
                nu &= m;
            }
            if slot.cur.val.len() <= k {
                slot.cur.val.resize(k + 1, 0);
                slot.cur.unk.resize(k + 1, 0);
            }
            if slot.cur.val[k] != nv || slot.cur.unk[k] != nu {
                slot.cur.val[k] = nv;
                slot.cur.unk[k] = nu;
                changed = true;
            }
        }
        if changed {
            self.note_change(net, 0);
            if track_edge {
                self.accumulate_edge(i, old_b0);
            }
        }
        changed
    }

    /// N4 clocking: take the PREPONED snapshot of every clocking-input source net
    /// at the start of a time slot (called right after time advances, BEFORE any
    /// slot activity — so each source holds the value it had ENTERING the slot, the
    /// true preponed value). EMPTY `clocking_inputs` ⇒ no-op ⇒ byte-identical.
    pub(crate) fn snapshot_preponed(&mut self) {
        if self.clocking_inputs.is_empty() {
            return;
        }
        for idx in 0..self.clocking_inputs.len() {
            let src = self.clocking_inputs[idx];
            let v = self.read_net(src, None);
            self.preponed_buf.insert(src, v);
        }
    }

    /// N4 clocking: a marked commit-handler proc fired on its clocking edge —
    /// commit `preponed_buf[source] → holding` for each of its inputs (blocking,
    /// same-slot), then drive `source = holding` for each of its outputs.
    /// Returns `true` iff this proc was a clocking handler (so the scheduler
    /// skips the no-op body dispatch).
    pub(crate) fn commit_clocking(&mut self, proc: u32) -> bool {
        let is_input_handler = self.clocking_commit.contains_key(&proc);
        let is_output_handler = self.clocking_outputs.contains_key(&proc);
        if !is_input_handler && !is_output_handler {
            return false;
        }
        // INPUT phase: preponed_buf[source] → holding_net (existing).
        if let Some(pairs) = self.clocking_commit.get(&proc).cloned() {
            for (hold, src) in pairs {
                if let Some(v) = self.preponed_buf.get(&src).cloned() {
                    self.commit_clocking_sample(hold, &v);
                }
            }
        }
        // OUTPUT phase: current_value(holding_net) → source_net (new).
        // Simplified synchronous model: drive in Active region at edge detection,
        // after INPUT commits. Hand-IEEE (no Reactive region; covers typical TB use).
        // NOTE: tuple is (source_net, holding_net) — REVERSED from clocking_commit.
        if let Some(out_pairs) = self.clocking_outputs.get(&proc).cloned() {
            for (src, hold) in out_pairs {
                let v = self.read_net(hold, None);
                self.commit_clocking_sample(src, &v);
            }
        }
        true
    }

    /// Record an ACTUAL bit change on `net`: mark it for the next
    /// `propagate_changes` dirty sweep, then emit the VCD record. This is the
    /// single funnel both `write_chunk` exit paths use — any future mutation
    /// path MUST route through it or the sweep goes blind.
    fn note_change(&mut self, net: u32, word: u32) {
        let i = net as usize;
        if !self.dirty_flag[i] {
            self.dirty_flag[i] = true;
            self.dirty.push(net);
            // GLITCH: first dirtying this slot — start the bit0 edge accumulator
            // fresh. (Later same-slot writes OR into it via `accumulate_edge`.)
            if self.is_edge_target[i] {
                self.slot_edge[i] = 0;
            }
        }
        // SELF-RETRIG: record who authored this change. A BLOCKING write by an
        // executing procedural body (`blocking_writer = Some`) tags the net so the
        // author isn't re-fired on it; every other writer (NBA/cont-assign/
        // clocking/force, `blocking_writer = None`) tags `u32::MAX` = re-fire
        // normally. Overwritten each change, so it is fresh for the next sweep.
        self.last_blocking_writer[i] = self.blocking_writer.unwrap_or(u32::MAX);
        self.emit_vcd_change(net, word);
    }

    /// GLITCH: OR this write's bit0 transition (`old_b0 → current bit0`) into the
    /// net's intra-slot edge accumulator. Called AFTER `note_change` (so the
    /// first-dirty reset has already run), only for `is_edge_target` whole-net /
    /// element-0 writes. `old_b0` is the net's scalar bit0 captured BEFORE the
    /// write.
    fn accumulate_edge(&mut self, net: usize, old_b0: sim_ir::FourState) {
        let new_b0 = scalar_bit0(&self.nets[net].cur);
        let mut m = 0u8;
        if fs_is_posedge(old_b0, new_b0) {
            m |= 1;
        }
        if fs_is_negedge(old_b0, new_b0) {
            m |= 2;
        }
        if old_b0 != new_b0 {
            m |= 4;
        }
        self.slot_edge[net] |= m;
    }

    /// Emit a VCD value_change for the net word that changed. Arrays carry one
    /// id PER ELEMENT (Phase-1.x ⑤ — the v1 VCD only ever showed word 0);
    /// scalars keep the single `vcd_id`.
    fn emit_vcd_change(&mut self, net: u32, word: u32) {
        if !self.dumping {
            return;
        }
        let i = net as usize;
        let width = self.nets[i].width;
        let id = if self.nets[i].vcd_word_ids.is_empty() {
            match self.nets[i].vcd_id {
                Some(id) => id,
                None => return,
            }
        } else {
            match self.nets[i].vcd_word_ids.get(word as usize) {
                Some(Some(id)) => *id,
                _ => return,
            }
        };
        let packed = slice_word(&self.nets[i].cur, width, word);
        if let Some(w) = self.vcd.as_mut() {
            let _ = w.set_time(self.now);
            let _ = w.value_change(id, &packed, width);
        }
    }

    // ── edge support ─────────────────────────────────────────────────────

    // (R2) The former `snapshot_prev` full-net cur→prev copy at each time
    // advance was DELETED: at the settled point `prev == cur` holds for every
    // net by induction — the only `prev` writers are `propagate_changes`
    // step (c) and the constructor, both setting prev = cur — so the pass was
    // a provable no-op costing O(nets) per timestep. Byte-compare suites
    // (staged/threads/corpus/differential) pin the equivalence.

    // ── force / release (IEEE 1364 §9.3.2; expression forces re-evaluate
    //    continuously via the scheduler's active_forces registry) ─────────

    /// Apply `force lhs = value`: write THROUGH the force flag (a re-force on
    /// an already-forced net must land), then pin the net. `lhs` is a single
    /// whole-net chunk (elaborate-validated).
    pub fn force_write(&mut self, lhs: &Lvalue, value: Value) -> bool {
        let net = lhs.chunks[0].net as usize;
        self.forced[net] = false;
        let offs = crate::exec::Offsets::Inline {
            buf: [(0, 0); 2],
            len: 1,
        };
        let changed = self.write_lvalue(lhs, value, &offs);
        self.forced[net] = true;
        changed
    }

    /// `release lhs`: unpin. A NET target snaps back to its driver at the next
    /// cont-assign settle (same timestep — the run loop settles every delta);
    /// a VARIABLE keeps the forced value until the next procedural assignment
    /// (no settle entry exists for it) — both fall out of just clearing the flag.
    pub fn release(&mut self, lhs: &Lvalue) {
        self.forced[lhs.chunks[0].net as usize] = false;
    }

    // ── C-FORCE-REEVAL-p2: force-RHS net-sensitivity sidecar ─────────────────

    /// Walk a force RHS expression, collecting every design net it READS (so a
    /// per-delta reeval can skip a force whose inputs are unchanged) and whether
    /// it is `volatile` — i.e. it contains a `$time`/`$realtime`/`$stime` or
    /// `$random`/`$urandom`/`$urandom_range` leaf, which yields a DIFFERENT value
    /// each delta even with frozen net inputs. The walk recurses every child
    /// ExprId (children are all `u32` arena indices). A `Signal{net, word}` reads
    /// `net` AND (recursively) its `word` index expr. Defensive on a malformed /
    /// out-of-range ExprId (treat as a volatile leaf so it is always re-evaluated
    /// — never silently dropped).
    pub fn collect_force_reads(&self, eid: u32) -> (Vec<u32>, bool) {
        let mut nets = Vec::new();
        let mut volatile = false;
        self.walk_force_reads(eid, &mut nets, &mut volatile);
        nets.sort_unstable();
        nets.dedup();
        (nets, volatile)
    }

    fn walk_force_reads(&self, eid: u32, nets: &mut Vec<u32>, volatile: &mut bool) {
        use sim_ir::Expr;
        let Some(e) = self.ir.exprs.get(eid as usize) else {
            // Unresolvable node → be conservative: force always re-evaluates.
            *volatile = true;
            return;
        };
        match e {
            Expr::Const { .. } | Expr::ArrayItem { .. } => {}
            Expr::Signal { net, word } => {
                nets.push(*net);
                if let Some(w) = word {
                    self.walk_force_reads(*w, nets, volatile);
                }
            }
            Expr::Select { base, offset, .. } => {
                self.walk_force_reads(*base, nets, volatile);
                self.walk_force_reads(*offset, nets, volatile);
            }
            Expr::Concat { parts } => {
                for &p in parts {
                    self.walk_force_reads(p, nets, volatile);
                }
            }
            Expr::Replicate { count, value } => {
                self.walk_force_reads(*count, nets, volatile);
                self.walk_force_reads(*value, nets, volatile);
            }
            Expr::Unary { operand, .. } => self.walk_force_reads(*operand, nets, volatile),
            Expr::Binary { lhs, rhs, .. } => {
                self.walk_force_reads(*lhs, nets, volatile);
                self.walk_force_reads(*rhs, nets, volatile);
            }
            Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.walk_force_reads(*cond, nets, volatile);
                self.walk_force_reads(*then_e, nets, volatile);
                self.walk_force_reads(*else_e, nets, volatile);
            }
            Expr::SysFunc { which, args } => {
                use sim_ir::SysFuncId as F;
                if matches!(
                    which,
                    F::Time | F::Realtime | F::Stime | F::Random | F::Urandom | F::UrandomRange
                ) {
                    *volatile = true;
                }
                for &a in args {
                    self.walk_force_reads(a, nets, volatile);
                }
            }
            Expr::Call { args, .. } => {
                // A user function call could read state the net-sensitivity map
                // cannot see (statics, side effects). Conservatively volatile so
                // it always re-evaluates (never silently frozen).
                *volatile = true;
                for &a in args {
                    self.walk_force_reads(a, nets, volatile);
                }
            }
        }
    }

    /// Register (or refresh) a force's net-sensitivity in the sidecar. Called at
    /// every `active_forces` insert so the per-delta reeval can target only the
    /// affected forces. `key` is the target net (the `active_forces` map key).
    pub fn register_force_sensitivity(&mut self, key: u32, rhs: u32) {
        // Refresh: a re-force on the same key may change the RHS — drop the old
        // sensitivity first so stale net→force edges never linger.
        self.unregister_force_sensitivity(key);
        let (reads, volatile) = self.collect_force_reads(rhs);
        if volatile || reads.is_empty() {
            // Volatile RHS ($time/$random/…) or a const/zero-net RHS: ALWAYS
            // re-evaluate. A const RHS reads no net, so the net→forces map would
            // never trigger it; treating it as always-reeval preserves today's
            // unconditional behavior (a same-value re-pin is dropped downstream).
            self.force_always_reeval.insert(key);
        } else {
            for n in reads {
                self.force_net_to_forces.entry(n).or_default().insert(key);
            }
        }
    }

    /// Drop a force's net-sensitivity from the sidecar (on release/displace).
    pub fn unregister_force_sensitivity(&mut self, key: u32) {
        self.force_always_reeval.remove(&key);
        // Remove `key` from every net's trigger set; prune emptied entries so
        // the map stays minimal (and the per-delta union loop stays cheap).
        self.force_net_to_forces.retain(|_, set| {
            set.remove(&key);
            !set.is_empty()
        });
    }

    // ── VCD lifecycle (driven by $dumpfile/$dumpvars) ────────────────────

    pub fn open_vcd(&mut self, sink: VcdSink) {
        self.vcd = Some(VcdWriter::new(sink));
    }

    pub fn finalize_vcd(&mut self) {
        if let Some(w) = self.vcd.as_mut() {
            // P2-2: a failed final flush means a truncated waveform — say so
            // (was: `let _ =` swallowed it; exit stayed 0 with no message).
            if let Err(e) = w.flush() {
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Warning,
                    code: MsgCode::RunVcdWriteFail,
                    message: format!("VCD flush failed: {e}"),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(diag::TimeStamp { ticks: self.now }),
                }));
            }
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

impl<'a> SimState<'a> {
    /// One W-RUN-DYN-DEGRADE per handle net, callable from `&self` (read path).
    pub(crate) fn dyn_warn_once_at(&self, net: u32, msg: &str) {
        if !self.dyn_warned.borrow_mut().insert(net) {
            return;
        }
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::RunDynDegrade,
            message: msg.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
    }

    /// v5 (C)-3b: indexed READ of a dyn handle. `idx` is the caller-resolved
    /// word (X/Z or >u32 already mapped to the `u32::MAX` sentinel — the same
    /// rule as static arrays). OOB / X-index / empty / whole-handle reads are
    /// element-width X + warn-once (IEEE: the element default; our elements
    /// are 4-state).
    fn dyn_read(&self, net: u32, idx: Option<u32>) -> Value {
        let nv = &self.ir.nets[net as usize];
        let (w, signed) = (nv.width.max(1), nv.signed);
        let xs = || Value::xs(w, signed);
        let Some(i) = idx else {
            // v7 P2-C: a STRING handle's whole-value read IS its packed
            // materialization (8×len, is_str — context resizing bypassed).
            if nv.kind == NetKind::String {
                let bytes: &[u8] = match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
                    Some(DynObj::Str { bytes }) => bytes,
                    _ => &[],
                };
                return Value::from_str_bytes(bytes);
            }
            // a handle has no scalar value surface (elaborate guards at ⑥;
            // defensive here — e.g. a hand-built IR or future regression).
            self.dyn_warn_once_at(net, "dyn handle read without an index");
            return xs();
        };
        match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
            Some(DynObj::DynArray { elems }) if (i as usize) < elems.len() => {
                elems[i as usize].clone()
            }
            Some(DynObj::Queue { elems }) if (i as usize) < elems.len() => {
                elems[i as usize].clone()
            }
            _ => {
                self.dyn_warn_once_at(net, "dyn index out of range or X (read X)");
                xs()
            }
        }
    }

    // ── N7 class/OOP heap accessors (sibling of the dyn_* family) ──────────
    /// Warn-once (per handle net) for a null/X dereference or a stale-object
    /// access. Never escalates to a fatal — IEEE makes null deref a runtime
    /// error, but vita degrades to X (read) / no-op (write) + this warning so a
    /// faulty testbench does not abort the whole run.
    pub(crate) fn class_warn_null(&self, net: u32, msg: &str) {
        if !self.class_null_warned.borrow_mut().insert(net) {
            return;
        }
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::RunDynDegrade,
            message: msg.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: Some(TimeStamp { ticks: self.now }),
        }));
    }

    /// The object-id a handle net currently points to, or `None` if it is
    /// `null` (id 0) or holds X/Z. Reads the handle's own integer value from the
    /// flat store (word-less read falls through the class branch in `read_net`).
    fn read_handle_id(&self, net: u32) -> Option<u32> {
        let v = self.read_net(net, None);
        if v.unk.iter().any(|&u| u != 0) {
            return None; // X/Z handle ⇒ null-like
        }
        match v.val.first().copied().unwrap_or(0) {
            0 => None, // null
            id => Some(id as u32),
        }
    }

    /// The field `(width, signed)` for field-id `field` of the object `id`
    /// belongs to (its DYNAMIC type). `(1,false)` fallback if unknown.
    fn class_field_width(&self, id: u32, field: u32) -> (u32, bool) {
        let cid = self.class_heap.borrow().get(&id).map(|o| o.class_id);
        cid.and_then(|c| self.class_layouts.get(c as usize))
            .map(|l| l.field_width(field))
            .unwrap_or((1, false))
    }

    /// N7: read field `field` of the object the handle points to. Null/X handle,
    /// a stale object, or a field-id past the layout ⇒ warn-once + X (never a
    /// panic). Returned at the field's natural width; `eval_ctx` resizes to ctx.
    fn class_field_read(&self, net: u32, field: u32) -> Value {
        match self.read_handle_id(net) {
            Some(id) => {
                let heap = self.class_heap.borrow();
                match heap.get(&id) {
                    Some(obj) if (field as usize) < obj.fields.len() => {
                        obj.fields[field as usize].clone()
                    }
                    _ => {
                        // CLS-FIELD-RD: fw (heap borrow + layout lookup) is only
                        // used on this cold stale/short arm — compute it here, not
                        // on the hot happy path above.
                        drop(heap);
                        let fw = self.class_field_width(id, field);
                        self.class_warn_null(net, "class field read of a stale/short object (X)");
                        Value::xs(fw.0.max(1), fw.1)
                    }
                }
            }
            None => {
                self.class_warn_null(net, "null/X class handle dereference (read X)");
                Value::xs(1, false)
            }
        }
    }

    /// N7: write field `field` of the object the handle points to. `&self` (the
    /// `RefCell` heap) so a value-method's body — running on the read path — can
    /// still mutate fields. Null/X handle or stale object ⇒ warn-once + no-op
    /// (not a panic). The value is resized to the field's declared width/sign.
    fn class_field_write(&self, c: &sim_ir::LvalChunk, field: u32, piece: &Value) -> bool {
        let net = c.net;
        let Some(id) = self.read_handle_id(net) else {
            self.class_warn_null(net, "null/X class handle dereference (write ignored)");
            return false;
        };
        let fw = self.class_field_width(id, field);
        let resized = piece.clone().resize_keep_sign(fw.0.max(1), fw.1);
        let mut heap = self.class_heap.borrow_mut();
        match heap.get_mut(&id) {
            Some(obj) if (field as usize) < obj.fields.len() => {
                obj.fields[field as usize] = resized;
            }
            _ => {
                drop(heap);
                self.class_warn_null(net, "class field write to a stale/short object (ignored)");
            }
        }
        false
    }

    /// N7: allocate a fresh object of `class_id`, default-init its fields per the
    /// layout, and return its monotonic object-id (≥1; never recycled). `&self`
    /// (interior-mutable heap) so a ctor invoked on the read path can allocate.
    pub(crate) fn class_alloc(&self, class_id: u32) -> u32 {
        let id = self.class_obj_next.get();
        self.class_obj_next.set(id + 1);
        let fields = match self.class_layouts.get(class_id as usize) {
            Some(layout) => (0..layout.fields.len() as u32)
                .map(|i| layout.default_value(i))
                .collect(),
            None => Vec::new(),
        };
        self.class_heap
            .borrow_mut()
            .insert(id, ClassObj { class_id, fields });
        id
    }

    /// v5 (C)-3b/④: indexed WRITE of a dyn handle. Shared rules: X-index /
    /// bit-select within an element → IGNORED + warn-once (clamping or
    /// auto-grow would silently corrupt). Kind split (iverilog live):
    /// dyn array — any OOB → IGNORED + warn; queue — `q[size] = v` is
    /// push_back-equivalent (IEEE §7.10.1, legal and SILENT, grows by one),
    /// beyond that → IGNORED + warn.
    /// Returns false ALWAYS: dyn content changes do not participate in the net
    /// dirty channel (design §4 — no sensitivity on handles, no VCD records).
    /// `dyn_heap` lazy-create accessor — the `BTreeMap::entry(net).or_insert_with`
    /// replacement for the flat `Vec<Option<DynObj>>` layout. Sets the slot to
    /// `Some(f())` only if it is currently `None`, then hands back `&mut DynObj`.
    /// `net` is always a valid HANDLE NetId (`< ir.nets.len()`), so the slot
    /// exists; the `expect` is unreachable by construction.
    pub(crate) fn dyn_entry(&mut self, net: u32, f: impl FnOnce() -> DynObj) -> &mut DynObj {
        let slot = &mut self.dyn_heap[net as usize];
        if slot.is_none() {
            *slot = Some(f());
        }
        slot.as_mut().expect("dyn_entry: slot just set to Some")
    }

    fn dyn_write(&mut self, c: &sim_ir::LvalChunk, raw_word: u32, piece: &Value) -> bool {
        let net = c.net;
        let w = self.ir.nets[net as usize].width.max(1);
        // v7 P2-C: STRING whole-handle assignment — strip leading NULs from
        // the packed value (§6.16) and store the bytes. The only legal
        // string lvalue shape; anything narrower falls to the loud arm.
        if self.ir.nets[net as usize].kind == NetKind::String
            && c.word.is_none()
            && c.offset.is_none()
            && c.width.is_none()
        {
            let bytes = piece.to_str_bytes();
            self.dyn_heap[net as usize] = Some(DynObj::Str { bytes });
            return false; // no net dirty channel (design §4, dyn precedent)
        }
        if c.word.is_none() || c.offset.is_some() || c.width.is_some() {
            self.dyn_warn_once_at(net, "unsupported dyn lvalue shape (write ignored)");
            return false;
        }
        // ⑤/v6: an assoc element on the u32 pair funnel = a shape the
        // AssocKey/AssocStrKey lane did not claim (a concat chunk, …) —
        // outside the MVP, IGNORED loud. The single-chunk lane
        // (`write_lvalue`) never reaches here.
        if matches!(
            self.ir.nets[net as usize].kind,
            NetKind::Assoc | NetKind::AssocStr
        ) {
            self.dyn_warn_once_at(
                net,
                "assoc element write in an unsupported lvalue shape (ignored)",
            );
            return false;
        }
        let i = raw_word as usize;
        if self.ir.nets[net as usize].kind == NetKind::Queue {
            // A missing entry IS the empty queue: the append lane must be
            // reachable on a never-touched handle (`q[0] = v` creates it).
            let DynObj::Queue { elems } = self.dyn_entry(net, || DynObj::Queue {
                elems: std::collections::VecDeque::new(),
            }) else {
                return false; // kind-mismatched entry: unreachable by construction
            };
            let len = elems.len();
            match i.cmp(&len) {
                std::cmp::Ordering::Less => elems[i] = piece.clone().resize(w),
                // The u32::MAX X-sentinel can never land in the Equal arm:
                // len ≤ the cap, far below the sentinel.
                std::cmp::Ordering::Equal if len < MAX_DYN_ELEMS => {
                    elems.push_back(piece.clone().resize(w));
                    self.enforce_queue_bound(net); // v6 ③ (no-op when unbounded)
                }
                std::cmp::Ordering::Equal => self.dyn_warn_once_at(
                    net,
                    "queue exceeds the element cap (1<<24); write-append dropped",
                ),
                std::cmp::Ordering::Greater => {
                    self.dyn_warn_once_at(net, "queue index beyond size or X (write ignored)");
                }
            }
            return false;
        }
        match self.dyn_heap.get_mut(net as usize).and_then(|o| o.as_mut()) {
            Some(DynObj::DynArray { elems }) if i < elems.len() => {
                elems[i] = piece.clone().resize(w);
                false
            }
            _ => {
                self.dyn_warn_once_at(net, "dyn index out of range or X (write ignored)");
                false
            }
        }
    }

    /// v5 ⑤: assoc-element WRITE (`a[k] = v`) — the `Offsets::AssocKey` lane.
    /// `None` key = X/Z (invalid index, IEEE §7.8.6): IGNORED + warn-once. A
    /// missing key CREATES the element (§7.8); the value is cast to the
    /// element type (the same `resize(w)` as every other dyn store). Inserts
    /// past the shared cap warn + drop (no silent caps).
    pub(crate) fn assoc_write(&mut self, net: u32, key: Option<i64>, value: &Value) {
        let w = self.ir.nets[net as usize].width.max(1);
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (write ignored)");
            return;
        };
        // Cap BEFORE the entry borrow (the warn latch needs `&self` while the
        // map borrow holds `&mut self`); replacing an existing key is exempt.
        let (len, exists) = match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
            Some(DynObj::Assoc { map }) => (map.len(), map.contains_key(&k)),
            _ => (0, false),
        };
        if !exists && len >= MAX_DYN_ELEMS {
            self.dyn_warn_once_at(net, "assoc exceeds the element cap (1<<24); write dropped");
            return;
        }
        // A missing entry IS the empty assoc (lazy, like every dyn object).
        let entry = self.dyn_entry(net, || DynObj::Assoc {
            map: std::collections::BTreeMap::new(),
        });
        if let DynObj::Assoc { map } = entry {
            map.insert(k, value.clone().resize(w));
        }
    }

    /// v6: string-keyed assoc WRITE — the `Offsets::AssocStrKey` lane (the
    /// byte-string twin of `assoc_write`; same X-key / cap / create rules).
    pub(crate) fn assoc_str_write(&mut self, net: u32, key: &Option<Vec<u8>>, value: &Value) {
        let w = self.ir.nets[net as usize].width.max(1);
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (write ignored)");
            return;
        };
        let (len, exists) = match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
            Some(DynObj::AssocStr { map }) => (map.len(), map.contains_key(k)),
            _ => (0, false),
        };
        if !exists && len >= MAX_DYN_ELEMS {
            self.dyn_warn_once_at(net, "assoc exceeds the element cap (1<<24); write dropped");
            return;
        }
        let entry = self.dyn_entry(net, || DynObj::AssocStr {
            map: std::collections::BTreeMap::new(),
        });
        if let DynObj::AssocStr { map } = entry {
            map.insert(k.clone(), value.clone().resize(w));
        }
    }

    /// v6 ③: bounded-queue post-op rule (iverilog live, IEEE §7.10):
    /// whatever the op left beyond size N+1 falls off the TAIL — one rule
    /// reproduces push_back-on-full (= skip), push_front-on-full (back
    /// drops) and insert-on-full (back drops). Loud (W4020 once per net).
    pub(crate) fn enforce_queue_bound(&mut self, net: u32) {
        let Some(&b) = self.queue_bounds.get(&net) else {
            return;
        };
        let cap = b as usize + 1;
        let mut dropped = false;
        if let Some(DynObj::Queue { elems }) =
            self.dyn_heap.get_mut(net as usize).and_then(|o| o.as_mut())
        {
            while elems.len() > cap {
                elems.pop_back();
                dropped = true;
            }
        }
        if dropped {
            self.dyn_warn_once_at(
                net,
                "bounded queue exceeded its bound; tail element(s) dropped",
            );
        }
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

impl<'a> SimState<'a> {
    /// Build a read-only `EvalCtx` over `&self` holding ZERO frame borrow
    /// (mirrors the scheduler's `eval`/`truthy` ctor). An `EvalCtx` can live
    /// across a nested `Expr::Call` because it touches the frame arena only
    /// transitively through `read_net`, which clones-and-releases. `$time`/
    /// `$realtime` inside a frame body run on the ambient `cur_time_mult` (a
    /// frame func is rejected at elaborate if it reads them — B1 cut).
    fn mk_eval_ctx(&self) -> crate::eval::EvalCtx<'_, SimState<'a>> {
        crate::eval::EvalCtx {
            ir: self.ir,
            nets: self,
            now: self.now,
            wt: &self.wt,
            time_mult: self.cur_time_mult,
            rng: &self.rng,
            plusargs: &self.plusargs,
        }
    }

    /// Read frame slot `slot` of function `func`: the top AUTOMATIC window, or
    /// the shared STATIC slab. Borrow → clone → drop-at-return (never held
    /// across a nested eval — §borrowDiscipline rule 1).
    fn frame_slot_read(&self, func: u32, automatic: bool, slot: u32) -> Value {
        if automatic {
            self.frame_stack
                .borrow()
                .last()
                .expect("frame read: no active call window")[slot as usize]
                .clone()
        } else {
            self.static_store
                .borrow()
                .get(&func)
                .expect("static read: no storage slab")[slot as usize]
                .clone()
        }
    }

    /// Store `v` into frame slot `slot` (arg binding). `v` is an already-owned
    /// Value (computed before any borrow); the `borrow_mut` is scoped to the
    /// single index-store — NO eval inside (§borrowDiscipline rule 3).
    fn frame_slot_write(&self, func: u32, automatic: bool, slot: u32, v: Value) {
        // A 2-state frame slot (byte/int/shortint/longint/bit) can never hold X/Z
        // (IEEE §6.11.3). Frame slot writes bypass `write_chunk`, so the coercion
        // it applies (val &= !unk; unk = 0) must be repeated here — for the arg
        // copy-IN, body-local assignments, and the return slot alike.
        let v = self.coerce_two_state_frame(func, slot, v);
        if automatic {
            let mut g = self.frame_stack.borrow_mut();
            g.last_mut().expect("arg bind: no active call window")[slot as usize] = v;
        } else {
            let mut g = self.static_store.borrow_mut();
            g.get_mut(&func).expect("arg bind: no storage slab")[slot as usize] = v;
        }
    }

    /// Coerce X/Z bits of `v` to 0 when the frame slot `(func, slot)` is a 2-state
    /// net (registered in `two_state`). The slot's flat net id is
    /// `func_table[func].base_net + slot`. A non-2-state slot returns `v` unchanged.
    fn coerce_two_state_frame(&self, func: u32, slot: u32, mut v: Value) -> Value {
        let Some(net) = self
            .func_table
            .get(func as usize)
            .map(|m| (m.base_net + slot) as usize)
        else {
            return v;
        };
        if net < self.two_state.len() && self.two_state[net] && v.unk.iter().any(|&u| u != 0) {
            for k in 0..v.unk.len() {
                v.val[k] &= !v.unk[k]; // X (val0/unk1) & Z (val1/unk1) → 0
                v.unk[k] = 0;
            }
        }
        v
    }

    /// Store the fully-evaluated `v` into a whole-net frame-local lvalue. The
    /// value is resized to the slot net's declared width/sign in a LOCAL first,
    /// THEN a scoped `borrow_mut` does the index-store with no eval inside
    /// (§borrowDiscipline rule 2). Part-select / array / module-net lvalues are
    /// rejected at ELABORATE, so the engine only ever sees a whole-net chunk;
    /// the `debug_assert` is a release-stripped backstop.
    fn frame_write_lvalue(&self, lhs: &Lvalue, v: Value) {
        debug_assert_eq!(
            lhs.chunks.len(),
            1,
            "frame lvalue is a single whole-net chunk"
        );
        let c = &lhs.chunks[0];
        let net = c.net as usize;
        debug_assert!(
            self.frame_local[net] && c.offset.is_none() && c.word.is_none() && c.width.is_none(),
            "frame write must target a whole frame-local net"
        );
        let (fidx, slot) = self.frame_route[net].expect("frame lvalue net is routed");
        let nv = &self.ir.nets[net];
        let val = v.resize_keep_sign(nv.width.max(1), nv.signed);
        // B4: route by this slot's EFFECTIVE lifetime (window vs static slab).
        self.frame_slot_write(fidx, self.frame_slot_auto[net], slot, val);
    }

    /// N7: store a frame-body blocking-assign value, routing a class FIELD write
    /// (`this.f = v` / `obj.f = v`) to the heap (`class_field_write`) and every
    /// other (whole frame-local net) write to `frame_write_lvalue`. `&self` —
    /// both targets are interior-mutable (the `RefCell` heap / the frame arena).
    fn frame_or_class_write(&self, lhs: &Lvalue, v: Value) {
        if lhs.chunks.len() == 1 {
            let c = &lhs.chunks[0];
            if self.class_is_handle[c.net as usize] && c.word.is_some() {
                let field = c
                    .word
                    .and_then(|w| crate::width::const_u32_of_expr(self.ir, w))
                    .unwrap_or(0);
                self.class_field_write(c, field, &v);
                return;
            }
        }
        self.frame_write_lvalue(lhs, v);
    }

    /// B1 frame-call evaluator. Runs user function `func`'s lowered body (in the
    /// GLOBAL `ir.blocks` arena from `FuncDef.entry`) against a per-invocation
    /// frame, returning its return-var Value resized to the declared return
    /// width/sign. `&self` (read path) + interior-mutable frame arena; the body
    /// BB loop is iterative, native recursion occurs ONLY on a nested
    /// `Expr::Call` (heap-bounded by the window stack, capped at
    /// `MAX_CALL_DEPTH`). `None` ⇒ a corrupt/empty sidecar (the eval arm
    /// X-poisons) — but `build_func_routing` validates the table, so a populated
    /// table reaching here is well-formed.
    fn run_frame_call(&self, func: u32, args: &[Value]) -> Option<Value> {
        use sim_ir::{Stmt, Terminator};
        if self.func_table.is_empty() {
            return None; // non-frame Call → the eval arm X-poisons
        }
        let m = self.func_table[func as usize];
        let (rw, rsig) = (m.ret_width.max(1), m.ret_signed);

        // ── runaway-recursion guard (heap depth, NOT host stack) ──
        let d = self.call_depth.get();
        if d >= MAX_CALL_DEPTH {
            if !self.call_fatal.get() {
                self.call_fatal.set(true);
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Fatal,
                    code: MsgCode::RunFatal,
                    message: format!(
                        "frame-call recursion exceeded the depth limit ({MAX_CALL_DEPTH})"
                    ),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(TimeStamp { ticks: self.now }),
                }));
            }
            // Finish THIS in-flight eval cleanly with all-X; the scheduler will
            // convert the latched `call_fatal` to FinishReason::Error.
            return Some(Value::xs(rw, rsig));
        }
        self.call_depth.set(d + 1);
        let _g = DepthGuard(&self.call_depth); // decrements on EVERY exit

        let fd = self.ir.funcs[func as usize];
        debug_assert!(
            !fd.is_task,
            "tasks are rejected at elaborate (B2 frame-call)"
        );
        debug_assert!(
            self.ir.nets[(m.base_net + m.return_slot) as usize].width == m.ret_width
                && self.ir.nets[(m.base_net + m.return_slot) as usize].signed == m.ret_signed,
            "return-var net width/sign must equal the declared ret_width/ret_signed"
        );
        let base = m.base_net;
        let nloc = m.locals_len;
        let np = fd.n_params;
        // B4: per-func storage needs (window for automatic slots, slab for static).
        let has_auto = self.func_has_auto[func as usize];
        let has_static = self.func_has_static[func as usize];

        // ── FRAME SETUP: build the fresh window in a LOCAL, then install it.
        //    The per-call WINDOW holds automatic slots (push/pop); the persistent
        //    STATIC slab (X-init ONCE, never reset) holds static slots. A func with
        //    no lifetime overrides uses exactly one of them (byte-identical to B1). ──
        // Each fresh frame slot defaults to its NET's declared init — X for 4-state
        // (reg/logic/integer), 0 for 2-state (bit/byte/int/shortint/longint) per IEEE
        // §6.4. (Using `Value::xs` unconditionally mis-defaulted 2-state locals to X,
        // silently corrupting reads/branches of an unassigned 2-state local.)
        let fresh: Vec<Value> = (0..nloc)
            .map(|s| {
                let nv = &self.ir.nets[(base + s) as usize];
                Value::from_packed(&nv.init, nv.width.max(1), nv.signed)
            })
            .collect();
        match (has_auto, has_static) {
            (true, true) => {
                self.frame_stack.borrow_mut().push(fresh.clone());
                self.static_store.borrow_mut().entry(func).or_insert(fresh);
            }
            (true, false) => self.frame_stack.borrow_mut().push(fresh),
            (false, _) => {
                self.static_store.borrow_mut().entry(func).or_insert(fresh);
            }
        }

        // ── BIND ARGS into the formal slots (resize to the formal's width). ──
        for i in 0..np {
            let nv = &self.ir.nets[(base + i) as usize];
            let v = args
                .get(i as usize)
                .cloned()
                .unwrap_or_else(|| Value::xs(nv.width.max(1), nv.signed))
                .resize_keep_sign(nv.width.max(1), nv.signed);
            self.frame_slot_write(func, self.frame_slot_auto[(base + i) as usize], i, v);
        }

        // ── BB LOOP over the GLOBAL func arena from `fd.entry`. Process bodies
        //    live in a SEPARATE `Process.body` space and are never touched. ──
        let mut cur = fd.entry;
        loop {
            debug_assert!(
                (cur as usize) < self.ir.blocks.len(),
                "frame CFG target in range (rebase complete)"
            );
            let blk = &self.ir.blocks[cur as usize];
            for &sid in &blk.stmts {
                if let Stmt::BlockingAssign { lhs, rhs } = &self.ir.stmts[sid as usize] {
                    let lw = self.lvalue_width(lhs);
                    let sw = self.wt.get(*rhs);
                    // OWNED Value FIRST — its nested Calls may recurse into
                    // run_frame_call, fine: THIS frame holds NO live borrow now.
                    let v = self
                        .mk_eval_ctx()
                        .eval_ctx(*rhs, lw.max(sw.width), sw.signed);
                    // THEN store (borrow scoped to the index-store only). N7: a
                    // class field write routes to the heap, not a frame slot.
                    self.frame_or_class_write(lhs, v);
                }
                // SysTask/NBA/delay/event in a func body are rejected at
                // ELABORATE (B1 cut) → never reach here.
            }
            match &blk.term {
                Terminator::Goto { target } => cur = *target,
                Terminator::Branch {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    let taken = self.mk_eval_ctx().truthy(*cond); // X/Z cond → else
                    cur = if taken { *then_bb } else { *else_bb };
                }
                Terminator::Return => break,
                // Delay/Wait/Fork/Call are illegal in a pure func body (rejected
                // at elaborate); break defensively (rebase keeps targets valid).
                _ => break,
            }
        }

        // ── READ the return slot (clone + release), resize to declared width. ──
        let ret_auto = self.frame_slot_auto[(base + m.return_slot) as usize];
        let rv = self
            .frame_slot_read(func, ret_auto, m.return_slot)
            .resize_keep_sign(rw, rsig);
        if has_auto {
            self.frame_stack.borrow_mut().pop(); // static: leave the slab (persistence)
        }
        Some(rv) // _g drops here → call_depth decremented
    }

    /// B2 frame-call: execute TASK `callee` against a fresh frame. `in_vals` are
    /// `(callee input-formal slot, value)` pairs already evaluated in the CALLER
    /// context; `out_slots` are the callee output-formal slots to read back at
    /// `Return`. Returns those slots' values (the caller writes them to its
    /// lvalues). Same `&self` frame arena + borrow discipline as `run_frame_call`;
    /// a NESTED task call (`Terminator::Call` in the task body) recurses and writes
    /// its outputs to the CALLING task's frame-local lvalues. `None` ⇒ empty
    /// sidecar (no frame tasks).
    fn run_task(
        &self,
        callee: u32,
        in_vals: &[(u32, Value)],
        out_slots: &[u32],
    ) -> Option<Vec<Value>> {
        use sim_ir::{Stmt, Terminator};
        if self.func_table.is_empty() {
            return None;
        }
        let m = self.func_table[callee as usize];
        // runaway guard (shared with run_frame_call).
        let d = self.call_depth.get();
        if d >= MAX_CALL_DEPTH {
            if !self.call_fatal.get() {
                self.call_fatal.set(true);
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Fatal,
                    code: MsgCode::RunFatal,
                    message: format!(
                        "frame-task recursion exceeded the depth limit ({MAX_CALL_DEPTH})"
                    ),
                    location: None,
                    context: Vec::new(),
                    sim_time: Some(TimeStamp { ticks: self.now }),
                }));
            }
            return Some(
                out_slots
                    .iter()
                    .map(|&s| {
                        let nv = &self.ir.nets[(m.base_net + s) as usize];
                        Value::xs(nv.width.max(1), nv.signed)
                    })
                    .collect(),
            );
        }
        self.call_depth.set(d + 1);
        let _g = DepthGuard(&self.call_depth);
        let fd = self.ir.funcs[callee as usize];
        debug_assert!(fd.is_task, "run_task on a non-task FuncDef");
        let base = m.base_net;
        let nloc = m.locals_len;
        // B4: per-func storage needs (window for automatic slots, slab for static).
        let has_auto = self.func_has_auto[callee as usize];
        let has_static = self.func_has_static[callee as usize];

        // ── FRAME SETUP (window for automatic slots; persistent slab for static). ──
        // Each fresh frame slot defaults to its NET's declared init — X for 4-state
        // (reg/logic/integer), 0 for 2-state (bit/byte/int/shortint/longint) per IEEE
        // §6.4. (Using `Value::xs` unconditionally mis-defaulted 2-state locals to X,
        // silently corrupting reads/branches of an unassigned 2-state local.)
        let fresh: Vec<Value> = (0..nloc)
            .map(|s| {
                let nv = &self.ir.nets[(base + s) as usize];
                Value::from_packed(&nv.init, nv.width.max(1), nv.signed)
            })
            .collect();
        match (has_auto, has_static) {
            (true, true) => {
                self.frame_stack.borrow_mut().push(fresh.clone());
                self.static_store
                    .borrow_mut()
                    .entry(callee)
                    .or_insert(fresh);
            }
            (true, false) => self.frame_stack.borrow_mut().push(fresh),
            (false, _) => {
                self.static_store
                    .borrow_mut()
                    .entry(callee)
                    .or_insert(fresh);
            }
        }

        // ── COPY-IN the input args (resize to each formal's width). ──
        for (slot, v) in in_vals {
            let nv = &self.ir.nets[(base + *slot) as usize];
            let bound = v.clone().resize_keep_sign(nv.width.max(1), nv.signed);
            self.frame_slot_write(
                callee,
                self.frame_slot_auto[(base + *slot) as usize],
                *slot,
                bound,
            );
        }

        // ── BB LOOP over the GLOBAL func arena from fd.entry. ──
        let mut cur = fd.entry;
        loop {
            debug_assert!(
                (cur as usize) < self.ir.blocks.len(),
                "task CFG target in range"
            );
            let blk = &self.ir.blocks[cur as usize];
            for &sid in &blk.stmts {
                if let Stmt::BlockingAssign { lhs, rhs } = &self.ir.stmts[sid as usize] {
                    let lw = self.lvalue_width(lhs);
                    let sw = self.wt.get(*rhs);
                    let v = self
                        .mk_eval_ctx()
                        .eval_ctx(*rhs, lw.max(sw.width), sw.signed);
                    self.frame_or_class_write(lhs, v);
                }
            }
            match &blk.term {
                Terminator::Goto { target } => cur = *target,
                Terminator::Branch {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    cur = if self.mk_eval_ctx().truthy(*cond) {
                        *then_bb
                    } else {
                        *else_bb
                    };
                }
                // NESTED task call — keyed by THIS block's global index.
                Terminator::Call { ret_bb, .. } => {
                    let Some(info) = self.task_calls_func.get(&cur).cloned() else {
                        break; // malformed sidecar: stop defensively
                    };
                    let cm = self.func_table[info.callee as usize];
                    let in_v: Vec<(u32, Value)> = info
                        .in_binds
                        .iter()
                        .map(|&(slot, e)| {
                            let nv = &self.ir.nets[(cm.base_net + slot) as usize];
                            let sw = self.wt.get(e);
                            let v = self.mk_eval_ctx().eval_ctx(
                                e,
                                nv.width.max(1).max(sw.width),
                                nv.signed,
                            );
                            (slot, v)
                        })
                        .collect();
                    let out_s: Vec<u32> = info.out_binds.iter().map(|&(s, _)| s).collect();
                    let outs = self
                        .run_task(info.callee, &in_v, &out_s)
                        .unwrap_or_default();
                    // callee popped → top frame is the calling task again; write
                    // its frame-local output lvalues.
                    for ((_, lval), val) in info.out_binds.iter().zip(outs) {
                        self.frame_write_lvalue(lval, val);
                    }
                    cur = *ret_bb;
                }
                Terminator::Return => break,
                _ => break,
            }
        }

        // ── COPY-OUT the requested output slots (resize to slot width). ──
        let outs: Vec<Value> = out_slots
            .iter()
            .map(|&s| {
                let nv = &self.ir.nets[(base + s) as usize];
                self.frame_slot_read(callee, self.frame_slot_auto[(base + s) as usize], s)
                    .resize_keep_sign(nv.width.max(1), nv.signed)
            })
            .collect();
        if has_auto {
            self.frame_stack.borrow_mut().pop();
        }
        Some(outs)
    }

    /// B2 frame-call: public &self entry for the executor's `Terminator::Call`
    /// (the `run_task` core is private to the frame-eval impl).
    pub(crate) fn run_task_call(
        &self,
        callee: u32,
        in_vals: &[(u32, Value)],
        out_slots: &[u32],
    ) -> Option<Vec<Value>> {
        self.run_task(callee, in_vals, out_slots)
    }
}

impl<'a> NetReader for SimState<'a> {
    fn dyn_size(&self, net: u32) -> Option<u64> {
        // Only a dyn HANDLE answers; a missing heap entry IS the empty object
        // (size 0 — IEEE: a declared dynamic array/queue/assoc starts empty).
        // Assoc included: `size()` is the IEEE alias of `num()` (§7.9.1). Any
        // other net kind returns None → the eval arm X-poisons defensively.
        match self.ir.nets.get(net as usize).map(|n| n.kind) {
            Some(
                sim_ir::NetKind::DynArray
                | sim_ir::NetKind::Queue
                | sim_ir::NetKind::Assoc
                | sim_ir::NetKind::AssocStr,
            ) => Some(
                self.dyn_heap
                    .get(net as usize)
                    .and_then(|o| o.as_ref())
                    .map(|o| o.len() as u64)
                    .unwrap_or(0),
            ),
            _ => None,
        }
    }
    /// ⓑ-breadth (v15): snapshot the element VALUES of a dyn handle in
    /// deterministic order, for the array reduction/ordering/locator methods.
    /// dyn array / queue iterate insertion order; assoc iterates its BTree key
    /// order (the IEEE iteration order). A non-handle (or a string handle, which
    /// is a byte sequence, not an element array) returns None → the caller
    /// X-poisons. A missing heap entry IS the empty array (`Some(vec![])`).
    fn dyn_values(&self, net: u32) -> Option<Vec<Value>> {
        match self.ir.nets.get(net as usize).map(|n| n.kind) {
            Some(
                sim_ir::NetKind::DynArray
                | sim_ir::NetKind::Queue
                | sim_ir::NetKind::Assoc
                | sim_ir::NetKind::AssocStr,
            ) => Some(
                match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
                    Some(DynObj::DynArray { elems }) => elems.clone(),
                    Some(DynObj::Queue { elems }) => elems.iter().cloned().collect(),
                    Some(DynObj::Assoc { map }) => map.values().cloned().collect(),
                    Some(DynObj::AssocStr { map }) => map.values().cloned().collect(),
                    _ => Vec::new(),
                },
            ),
            _ => None,
        }
    }
    fn array_item(&self, index: bool) -> Value {
        match &*self.array_item_scratch.borrow() {
            Some((val, idx)) => {
                if index {
                    let mut v = Value::zeros(32, true);
                    v.val[0] = (*idx).min(i32::MAX as u64);
                    v
                } else {
                    val.clone()
                }
            }
            None => Value::xs(32, true),
        }
    }
    fn swap_array_item(&self, v: Option<(Value, u64)>) -> Option<(Value, u64)> {
        self.array_item_scratch.replace(v)
    }
    fn dyn_warn(&self, net: u32, msg: &str) {
        // The eval-side degradation hook (e.g. a pop outside its statement
        // intercept) — same W4020 once-per-net latch as every other lane.
        self.dyn_warn_once_at(net, msg);
    }
    fn is_assoc(&self, net: u32) -> bool {
        // Bitmap first: the static-array hot path short-circuits on one bit
        // load (the same budget `read_net` already pays), only real handles
        // pay the kind lookup.
        self.dyn_is_handle
            .get(net as usize)
            .copied()
            .unwrap_or(false)
            && matches!(
                self.ir.nets.get(net as usize).map(|n| n.kind),
                Some(sim_ir::NetKind::Assoc)
            )
    }
    fn assoc_read(&self, net: u32, key: Option<i64>) -> Value {
        let (w, signed) = self
            .ir
            .nets
            .get(net as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((1, false));
        let Some(k) = key else {
            // X/Z key = invalid index (IEEE §7.8.6): element-width X + warn.
            self.dyn_warn_once_at(net, "assoc key is X/Z (read X)");
            return Value::xs(w, signed);
        };
        match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
            Some(DynObj::Assoc { map }) => map.get(&k).cloned().unwrap_or_else(|| {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }),
            // A missing entry IS the empty assoc — every key is "not found".
            _ => {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }
        }
    }
    fn assoc_exists(&self, net: u32, key: Option<i64>) -> Option<bool> {
        if !self.is_assoc(net) {
            return None; // not an assoc handle → the eval arm X-poisons
        }
        let Some(k) = key else {
            // exists() with an X/Z key matches nothing — 0, but LOUD (the
            // same invalid-index family as read/write, policy pin).
            self.dyn_warn_once_at(net, "assoc key is X/Z (exists 0)");
            return Some(false);
        };
        Some(
            match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
                Some(DynObj::Assoc { map }) => map.contains_key(&k),
                _ => false,
            },
        )
    }
    fn str_bytes(&self, net: u32) -> Option<Vec<u8>> {
        if self.ir.nets.get(net as usize).map(|n| n.kind) != Some(NetKind::String) {
            return None;
        }
        Some(
            match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
                Some(DynObj::Str { bytes }) => bytes.clone(),
                _ => Vec::new(),
            },
        )
    }
    fn is_assoc_str(&self, net: u32) -> bool {
        self.dyn_is_handle
            .get(net as usize)
            .copied()
            .unwrap_or(false)
            && matches!(
                self.ir.nets.get(net as usize).map(|n| n.kind),
                Some(sim_ir::NetKind::AssocStr)
            )
    }
    fn assoc_str_read(&self, net: u32, key: &Option<Vec<u8>>) -> Value {
        let (w, signed) = self
            .ir
            .nets
            .get(net as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
            .unwrap_or((1, false));
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (read X)");
            return Value::xs(w, signed);
        };
        match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
            Some(DynObj::AssocStr { map }) => map.get(k).cloned().unwrap_or_else(|| {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }),
            _ => {
                self.dyn_warn_once_at(net, "assoc key not found (read X)");
                Value::xs(w, signed)
            }
        }
    }
    fn assoc_str_exists(&self, net: u32, key: &Option<Vec<u8>>) -> Option<bool> {
        if !self.is_assoc_str(net) {
            return None;
        }
        let Some(k) = key else {
            self.dyn_warn_once_at(net, "assoc key is X/Z (exists 0)");
            return Some(false);
        };
        Some(
            match self.dyn_heap.get(net as usize).and_then(|o| o.as_ref()) {
                Some(DynObj::AssocStr { map }) => map.contains_key(k),
                _ => false,
            },
        )
    }
    fn eval_call(&self, func: u32, args: &[Value]) -> Option<Value> {
        self.run_frame_call(func, args)
    }
    fn resolve_virtual_call(&self, call_eid: u32, static_fid: u32, args: &[Value]) -> u32 {
        // Only a virtual call site (sidecar `vslot = Some`) redirects. CLS-CALL-VEC:
        // O(1) ExprId index; out-of-range (non-class / smaller Vec) ⇒ None.
        let Some(&(Some(vslot), _)) = self
            .class_calls
            .get(call_eid as usize)
            .and_then(|o| o.as_ref())
        else {
            return static_fid;
        };
        // args[0] = the receiver handle's object-id; X / null → static target
        // (the body then null-derefs to X — never a vtable OOB).
        let Some(this_v) = args.first() else {
            return static_fid;
        };
        if this_v.unk.iter().any(|&u| u != 0) {
            return static_fid;
        }
        let id = this_v.val.first().copied().unwrap_or(0) as u32;
        if id == 0 {
            return static_fid;
        }
        let class_id = match self.class_heap.borrow().get(&id) {
            Some(o) => o.class_id,
            None => return static_fid,
        };
        self.class_vtable
            .get(class_id as usize)
            .and_then(|t| t.get(vslot as usize))
            .copied()
            .filter(|&f| f != u32::MAX)
            .unwrap_or(static_fid)
    }
    fn formal_width(&self, func: u32, i: usize) -> Option<(u32, bool)> {
        // The i-th formal is `base_net + i` (port order, [0..n_params)); its
        // NetVar carries the declared (width, signed). Out of range (the call
        // passes MORE actuals than the func has formals) ⇒ None → the eval arm
        // falls back to the actual's self-width.
        let m = self.func_table.get(func as usize)?;
        if (i as u32) >= m.n_params {
            return None;
        }
        let nv = &self.ir.nets[(m.base_net + i as u32) as usize];
        Some((nv.width.max(1), nv.signed))
    }
    fn read_net(&self, net: u32, word: Option<u32>) -> Value {
        // N7: a class-handle FIELD-select read (`obj.f` / `this.f`, word = field-id)
        // goes to the heap. Checked FIRST so a method's `this` slot — which is also
        // a frame-local net — routes to the field, not the frame window. A bare
        // (word-less) handle read falls through (frame window or flat store = the id).
        if self.class_is_handle[net as usize] {
            if let Some(field) = word {
                return self.class_field_read(net, field);
            }
        }
        // B1 frame-call: a frame-local net reads from the ACTIVE call window
        // (automatic) or the shared static slab — never the flat store. One
        // bitmap load on the hot path (sibling of `dyn_is_handle`, always
        // full-length so it indexes directly). EMPTY func_table ⇒ all-false ⇒
        // byte-identical. The read clones the slot Value and releases the frame
        // borrow at the `return` BEFORE control re-enters any evaluator.
        if self.frame_local[net as usize] {
            let (fidx, slot) = self.frame_route[net as usize].expect("frame-local net is routed");
            debug_assert!(
                word.is_none(),
                "B1 frame-local nets are scalar (no array word)"
            );
            // B4: route by this slot's EFFECTIVE lifetime (window vs static slab).
            return self.frame_slot_read(fidx, self.frame_slot_auto[net as usize], slot);
        }
        // v5 (C)-3b: a dyn HANDLE never reads the flat store — its elements
        // live in the heap. One bitmap load on the hot path.
        if self.dyn_is_handle[net as usize] {
            return self.dyn_read(net, word);
        }
        let slot = &self.nets[net as usize];
        let width = slot.width;
        let w = word.unwrap_or(0);
        // Out-of-range array word reads all-X (spec E-RUN-RANGE) — NOT a clamp to the
        // last element (which would silently return a neighbour's value).
        if w >= slot.array_len {
            self.warn_run_range("array word index");
            let mut v = Value::xs(width.max(1), slot.signed);
            v.width = width;
            v.is_real = slot.is_real;
            return v;
        }
        // Read the element DIRECTLY into an inline `Value` — no transient `BitPacked`
        // Vec (the prior `net_word_packed → slice_word → BitPacked → from_packed` path
        // allocated two Vecs per read just to copy them into the inline planes). The
        // word-aligned fast path (every scalar net + 64-aligned array element) copies
        // whole store words; an unaligned base (non-64 array element) falls to bit-serial.
        let base = w * width;
        let n = nwords(width).max(1);
        let mut v = if base % 64 == 0 {
            let wbase = (base / 64) as usize;
            let mut val = Words::zeros(n);
            let mut unk = Words::zeros(n);
            for k in 0..n {
                val[k] = slot.cur.val.get(wbase + k).copied().unwrap_or(0);
                unk[k] = slot.cur.unk.get(wbase + k).copied().unwrap_or(0);
            }
            let m = top_mask(width);
            val[n - 1] &= m;
            unk[n - 1] &= m;
            Value {
                val,
                unk,
                width,
                signed: slot.signed,
                is_real: false,
                is_str: false,
            }
        } else {
            let mut tmp = Value::zeros(width.max(1), slot.signed);
            tmp.width = width;
            for i in 0..width {
                let (vv, uu) = bit_of(&slot.cur, base + i);
                tmp.set_vu(i, vv, uu);
            }
            tmp
        };
        v.is_real = slot.is_real; // flag the read-back as real (val[0] = IEEE bits)
        v
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

/// NetKind → VCD VarType.
pub(crate) fn vcd_var_type(kind: NetKind) -> VarType {
    match kind {
        NetKind::Reg => VarType::Reg,
        NetKind::Integer => VarType::Integer,
        NetKind::Real => VarType::Real, // VCD `$var real`
        NetKind::Wire | NetKind::Logic => VarType::Wire,
        // v5/v6/v7 dyn + string handles are NEVER declared to the VCD (design
        // doc: variable length has no $var form) — filtered upstream; defensive map.
        NetKind::DynArray
        | NetKind::Queue
        | NetKind::Assoc
        | NetKind::AssocStr
        | NetKind::String => VarType::Wire,
    }
}

/// Build a net's storage from its declared init. For scalars the init IS the
/// value; for arrays the init plane is replicated per word (elaborate emits one
/// init plane; v1 broadcasts it to every element).
fn expand_init(init: &BitPacked, width: u32, array_len: u32, total_bits: usize) -> BitPacked {
    let total_words = nwords(total_bits as u32).max(1);
    if array_len <= 1 {
        let mut val = init.val.clone();
        let mut unk = init.unk.clone();
        val.resize(total_words, 0);
        unk.resize(total_words, 0);
        return BitPacked { val, unk };
    }
    // broadcast the width-wide init to each element
    let mut out = BitPacked {
        val: vec![0; total_words],
        unk: vec![0; total_words],
    };
    let elem = Value::from_packed(init, width, false);
    for w in 0..array_len {
        let base = w * width;
        for i in 0..width {
            let (v, u) = elem.get_vu(i);
            set_bit(&mut out, base + i, v, u);
        }
    }
    out
}

/// Slice `width` bits starting at word `word`*width from a packed store.
fn slice_word(store: &BitPacked, width: u32, word: u32) -> BitPacked {
    let base = word * width;
    // WORD-PARALLEL fast path: a 64-aligned element read — copy whole store words and
    // mask the top partial word (which discards any neighbouring element's bits that
    // share that word). Covers every scalar net (base 0) and 64-aligned array elements;
    // an unaligned base (array element with a non-64-aligned offset) falls to bit-serial.
    if base % 64 == 0 {
        let n = nwords(width).max(1);
        let wbase = (base / 64) as usize;
        let mut val = vec![0u64; n];
        let mut unk = vec![0u64; n];
        for k in 0..n {
            val[k] = store.val.get(wbase + k).copied().unwrap_or(0);
            unk[k] = store.unk.get(wbase + k).copied().unwrap_or(0);
        }
        let m = top_mask(width);
        val[n - 1] &= m;
        unk[n - 1] &= m;
        return BitPacked { val, unk };
    }
    let mut tmp = Value::zeros(width.max(1), false);
    tmp.width = width;
    for i in 0..width {
        let (v, u) = bit_of(store, base + i);
        tmp.set_vu(i, v, u);
    }
    tmp.into_bitpacked(width)
}

#[inline]
fn bit_of(b: &BitPacked, i: u32) -> (u64, u64) {
    let w = (i / 64) as usize;
    let s = i % 64;
    let v = b.val.get(w).map_or(0, |x| (x >> s) & 1);
    let u = b.unk.get(w).map_or(0, |x| (x >> s) & 1);
    (v, u)
}

/// Set bit `i` of a packed store to (v,u); grow as needed; return true if changed.
#[inline]
fn set_bit(b: &mut BitPacked, i: u32, v: u64, u: u64) -> bool {
    let w = (i / 64) as usize;
    let s = i % 64;
    while b.val.len() <= w {
        b.val.push(0);
    }
    while b.unk.len() <= w {
        b.unk.push(0);
    }
    let ov = (b.val[w] >> s) & 1;
    let ou = (b.unk[w] >> s) & 1;
    if ov == v && ou == u {
        return false;
    }
    b.val[w] = (b.val[w] & !(1 << s)) | ((v & 1) << s);
    b.unk[w] = (b.unk[w] & !(1 << s)) | ((u & 1) << s);
    true
}

/// Scalar (bit0) 4-state of a net's current value — for edge detection.
pub(crate) fn scalar_bit0(b: &BitPacked) -> sim_ir::FourState {
    let v = b.val.first().copied().unwrap_or(0) & 1;
    let u = b.unk.first().copied().unwrap_or(0) & 1;
    match (v, u) {
        (0, 0) => sim_ir::FourState::Zero,
        (1, 0) => sim_ir::FourState::One,
        (0, 1) => sim_ir::FourState::X,
        _ => sim_ir::FourState::Z,
    }
}

/// WIDE-EDGE: bit0 edge predicates per IEEE 1364 §5.1.2 (Table — confirmed vs
/// iverilog 13.0 across all 12 four-state transitions). A `posedge` is a
/// transition TOWARD 1: `0→1`, `0→x`, `0→z`, `x→1`, `z→1`. A `negedge` is a
/// transition TOWARD 0: `1→0`, `1→x`, `1→z`, `x→0`, `z→0`. (`x→z`/`z→x` are
/// neither.) The old narrow form (`new==1 && prev!=1`) missed the `0→x`/`0→z`
/// rises and `1→x`/`1→z` falls — a silent-wrong for any edge net that goes to an
/// UNKNOWN value mid-sim. The write funnel ORs these per transition into the
/// `slot_edge` mask, the single source of procedural `@(posedge/negedge)` firing.
#[inline]
fn fs_is_posedge(prev: sim_ir::FourState, new: sim_ir::FourState) -> bool {
    use sim_ir::FourState::{One, Zero};
    (prev == Zero && new != Zero) || (new == One && prev != One)
}
#[inline]
fn fs_is_negedge(prev: sim_ir::FourState, new: sim_ir::FourState) -> bool {
    use sim_ir::FourState::{One, Zero};
    (prev == One && new != One) || (new == Zero && prev != Zero)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P2-2: a VCD flush failure at finalize must surface as W-RUN-VCD-WRITE-FAIL
    /// (was: `let _ =` silently swallowed it — truncated VCD, zero diagnostics).
    #[test]
    fn finalize_vcd_flush_error_warns() {
        struct FailWriter;
        impl Write for FailWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len()) // accept records...
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("disk full")) // ...but fail the flush
            }
        }
        #[derive(Default)]
        struct DiagSink(std::cell::RefCell<Vec<String>>);
        impl LogSink for DiagSink {
            fn emit(&self, e: LogEvent) {
                if let LogEvent::Diagnostic(d) = e {
                    self.0
                        .borrow_mut()
                        .push(format!("{}: {}", d.code.code_num(), d.message));
                }
            }
        }

        let (toks, _) = hdl_lexer::lex("module t; endmodule");
        let (su, _) = hdl_parser::parse(&toks, "module t; endmodule");
        let sink = DiagSink::default();
        let ir = elaborate::elaborate(&su.expect("unit"), &sink).expect("elaborate");
        let mut st = SimState::new(
            &ir,
            Box::new(std::io::sink()),
            &sink,
            "1ns".to_string(),
            "test".to_string(),
            None,
        );
        st.open_vcd(Box::new(FailWriter));
        st.finalize_vcd();
        let diags = sink.0.borrow();
        assert!(
            diags.iter().any(|d| d.starts_with("VITA-W4019")),
            "flush failure must emit W-RUN-VCD-WRITE-FAIL; got {diags:?}"
        );
    }
}
