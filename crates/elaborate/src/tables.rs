//! public sidecar table types — split out of the original `elaborate` lib.rs (mechanical move).

/// Join mode for a `fork … join`/`join_any`/`join_none`. NOT part of `SimIr`
/// (the frozen `Terminator::Fork` carries no mode field): it rides out-of-band in
/// the [`ForkModeTable`] so the golden root stays byte-identical. The engine
/// consults it when executing the `Fork` terminator (total-or-fatal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JoinMode {
    /// `join` — parent blocks until ALL children reach the join.
    All,
    /// `join_any` — parent unblocks at the FIRST child; surplus run on.
    Any,
    /// `join_none` — parent never blocks; children run as background activities.
    None,
}

/// Join-mode side table: `(template ProcId, join_bb)` → [`JoinMode`]. A
/// deterministic `BTreeMap` so it is 3-OS byte-stable when serialized; it NEVER
/// enters the golden `SimIr` root. The key is globally unique because each
/// process body is a private BB arena and `join_bb` is unique within it.
pub type ForkModeTable = std::collections::BTreeMap<(u32, u32), JoinMode>;

/// Stage-1 fork-in-frame: the sentinel "template" key for a fork that lives inside a
/// suspendable TASK body (its blocks are in the GLOBAL `func_blocks` arena, so its
/// `join_bb` is globally unique and its mode is independent of which process runs the
/// task). Recorded as `(FRAME_FORK_KEY, global join_bb)` at the task-body flush and
/// queried under the same key by the engine's `exec_fork` when the parent is in-frame.
/// `u32::MAX` can never collide with a real `ProcId` (dense `0..nproc`).
pub const FRAME_FORK_KEY: u32 = u32::MAX;

/// Per-NetId fully-qualified hierarchical name (`"top.dut.q"`), source order.
/// An engine-facing SIDE TABLE for the VCD writer — like [`ForkModeTable`] it
/// rides out-of-band in `SimOpts` and NEVER enters the frozen `SimIr` root (which
/// carries no name field). Threaded by the simulate path so `$dumpvars` emits real
/// hierarchical `$scope`/`$var` instead of a flat `top` + synthetic `n0..nN`.
pub type NetNameTable = Vec<String>;

/// Severity class of a lowered `$fatal`/`$error`/`$warning`/`$info` statement.
/// NOT part of `SimIr` (the frozen `SysTaskId` has no severity variants): a
/// severity task lowers to a plain `SysTaskId::Display` stmt, and this kind rides
/// out-of-band in the [`SeverityTable`] so the golden root stays byte-identical.
/// The engine consults it per-StmtId to route the text to the DIAGNOSTIC stream
/// (doc-13 tokens `fatal[VITA-F4004]`/`error[VITA-E4003]`/…) instead of stdout,
/// and to abort (`$fatal`) or flag the exit class (`$error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SeverityKind {
    /// `$info` — diagnostic only; exit class untouched.
    Info,
    /// `$warning` — diagnostic only; exit class untouched.
    Warning,
    /// `$error` — diagnostic + `ExitClass::HadErrors`; run continues.
    Error,
    /// `$fatal` — diagnostic + implicit `$finish` with `ExitClass::Fatal`.
    Fatal,
}

/// Severity side table: StmtId → [`SeverityKind`]. A deterministic `BTreeMap`
/// (3-OS byte-stable when serialized); like [`ForkModeTable`] it rides in
/// `SimOpts` / the `.velab` trailer and NEVER enters the golden `SimIr` root.
pub type SeverityTable = std::collections::BTreeMap<u32, SeverityKind>;

/// Default-radix side table (P1-5): StmtId → radix (2/8/16) for the
/// `$displayb/o/h`, `$writeb/o/h`, `$strobeb/o/h`, `$monitorb/o/h` variants —
/// the b/o/h changes only how UNFORMATTED arguments render (IEEE §17.1.1.1).
/// Out-of-band like the other tables; the frozen `SysTaskId` is unchanged.
pub type RadixTable = std::collections::BTreeMap<u32, u8>;

/// Maturation region of a deferred immediate assertion (IEEE 1800 §16.4 / §4.4),
/// mirrored at the engine. NOT part of `SimIr` and deliberately NOT
/// `sim_ir::RegionTag` (which is golden-reachable via `WakeKey`→`Process`→`SimIr`
/// and would flip the root hash) — a fresh out-of-band enum that rides `SimOpts`
/// and the `.velab` trailer like every other sidecar, so the golden root stays
/// byte-identical and `format_version` is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeferRegion {
    /// `assert #0` — matures in the Observed region.
    Observed,
    /// `assert final` — matures in the Reactive region.
    Reactive,
}

/// Deferred-assert FLUSH-MARKER side table (§16.4): the StmtId of the synthesized
/// no-op marker emitted just before each deferred assertion's `Branch` → its
/// region. The marker StmtId IS the assertion-instance identity; reaching it (in
/// the Active region) cancels any prior pending report for `(marker_sid,
/// activity)` — this is flush-on-re-reach, the defining deferred-assert
/// behavior. Out-of-band; the frozen IR is unchanged (the marker is an ordinary
/// `SysTaskId::Display` stmt the engine suppresses via this table).
pub type DeferMarkTable = std::collections::BTreeMap<u32, DeferRegion>;

/// Deferred-assert ACTION side table (§16.4): the StmtId of each pass/fail action
/// SysTask (the `$error`/`$display`/`$fatal`/… inside a deferred assert's arms) →
/// `(owning marker StmtId, region)`. Reaching the action ENQUEUES a report under
/// `(marker_sid, activity)` for region maturation instead of firing inline.
/// Out-of-band like the other tables.
pub type DeferActTable = std::collections::BTreeMap<u32, (u32, DeferRegion)>;

/// Assign-rank side table (IEEE 1364 §9.3.1): the StmtIds of `Stmt::Force` /
/// `Stmt::Release` statements that are really procedural `assign`/`deassign`
/// (the frozen `Stmt` has no Assign/Deassign variants — they reuse the force
/// machinery at a WEAKER rank: a real `force` overrides an active assign and
/// `release` hands control back to it). Out-of-band like the other tables.
pub type AssignRankTable = std::collections::BTreeSet<u32>;

/// Bounded-queue side table (v6 ③): HANDLE NetId → declared bound N
/// (`[$:N]`, max size N+1 — iverilog live). EMPTY by default (every queue
/// unbounded). Never enters the golden IR — rides `SimOpts` + a `.velab`
/// trailer segment like every other sidecar.
pub type QueueBoundTable = std::collections::BTreeMap<u32, u32>;

/// Unpacked-dimension side table (Phase-1.x ⑤): array NetId → per-dim
/// `(lo, size)` in declared order. SPARSE — exactly the elaborate-local
/// `array_dims` map, so a plain 0-based 1-D array is ABSENT and the engine
/// falls back to `[(0, array_len)]`. Drives per-element VCD naming
/// (`mem[4]`, `g[1][2]`). Out-of-band like every other sidecar.
pub type NetDimsTable = std::collections::BTreeMap<u32, Vec<(i64, u32)>>;

/// Frame-call metadata (B1, automatic/recursive functions), INDEX-ALIGNED to
/// `ir.funcs[i]` by construction (pushed in the same `lower_frame_func` that
/// writes `ir.funcs[idx]`). The frozen `FuncDef` carries only `entry/n_params/
/// locals_len/is_task`; everything the engine needs to ROUTE a frame call rides
/// here, out-of-band, so the golden `SimIr` root (and `schema_hash`/
/// `format_version`) is byte-unchanged. EMPTY by default ⇒ no frame functions ⇒
/// every existing design byte-identical.
///
/// SLOT LAYOUT (contiguous `ir.nets` ids from `base_net`, declaration order):
/// `[0..n_params)` = input formals (port order); `[n_params]` = the func-named
/// RETURN var (allocated at exactly the declared return range/sign); `[n_params+
/// 1..locals_len)` = `body_decls` (source order). All are REAL `ir.nets` entries
/// (so `width.rs`/`read_net`/lvalue lowering see correct width/signed); they are
/// flagged frame-local ONLY here (a `NetVar` has no spare bit).
///
/// CONTRACT (engine debug-asserts): `return_slot == n_params`, and
/// `ir.nets[base_net + return_slot].width == ret_width &&  .signed == ret_signed`
/// — the return-var net is allocated at exactly the declared width/sign so the
/// engine's read-slot-then-resize is idempotent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FuncMeta {
    /// First `ir.nets` id of this function's frame window.
    pub base_net: u32,
    /// Number of input formals (== `FuncDef.n_params`).
    pub n_params: u32,
    /// LOCAL slot index (0..`locals_len`) of the func-named return var
    /// (== `n_params` by the layout convention above).
    pub return_slot: u32,
    /// Total frame slots (formals + return-var + body_decls; == `FuncDef.locals_len`).
    pub locals_len: u32,
    /// `true` ⇒ fresh window per call (push/pop). `false` ⇒ ONE shared static
    /// slab (no restore → the deepest write clobbers = the iverilog-faithful
    /// static-lifetime corruption). The only storage-policy discriminator.
    pub is_automatic: bool,
    /// Declared return self-width (`FunctionDef.range`; `integer` ⇒ 32, signed).
    /// Hoisted because `Expr::Call` has no net id of its own — `width.rs` sizes
    /// the call from this.
    pub ret_width: u32,
    /// Declared return signedness.
    pub ret_signed: bool,
    /// B4: per-slot AUTOMATIC lifetime override bitmask (bit `i` set ⇒ slot `i`
    /// was declared `automatic` regardless of the function/task default). A slot's
    /// EFFECTIVE lifetime is automatic iff `(auto_override >> i) & 1 == 1` OR
    /// `is_automatic`. `0` (the common case) ⇒ every slot follows `is_automatic`
    /// (byte-identical to B1/B2). Slots ≥ 64 always follow the default.
    pub auto_override: u64,
    /// v7: per-FORMAL `string`-type bitmask (bit `i` set ⇒ input formal `i` was
    /// declared `input string`). A string formal lowers to a 1-bit `Wire` net (a
    /// string is a dynamic handle, not a fixed width), so the engine cannot tell
    /// it from a scalar by width/kind — this mask lets `Expr::Call` arg binding
    /// materialise a string LITERAL actual as a heap-string value instead of
    /// truncating it to the 1-bit slot. `0` (the common case) ⇒ no string formals
    /// ⇒ byte-identical. Formals ≥ 64 are not marked (frame funcs never have that
    /// many params). `#[serde(default)]` so a trailer written by an older binary
    /// still deserialises (missing ⇒ 0 ⇒ prior behaviour).
    #[serde(default)]
    pub str_params: u64,
    /// §4.5.208: this frame TASK's body contains a DEFERRED hierarchical enable
    /// (`u1.tk(...)` nested inside the body). The hier call's placeholder `Call.target`
    /// is patched to the callee entry only at the finish-phase resolve — AFTER the
    /// per-instance `resolve_frame_task_rejects` runs `compute_suspendable_tasks`, so the
    /// two computes (elaborate pre-resolve, engine post-resolve) would classify the caller
    /// DIFFERENTLY (breaking the §4.5.197 pure-function contract). This flag forces the
    /// caller SUSPENDABLE in BOTH computes (`compute_suspendable_tasks`'s `force_suspend`) —
    /// sound (over-approximation: a hier callee may suspend) and consistent (both derive it
    /// from this serialized field). `false` (the common case) ⇒ byte-identical. `#[serde(
    /// default)]` so an older trailer still deserialises (missing ⇒ false ⇒ prior behaviour;
    /// old artifacts never had a frame-body hier call, so the default is exactly correct).
    #[serde(default)]
    pub has_hier_call: bool,
    /// Stage-2 fork-in-frame: this frame TASK has an admitted Case-B `fork … join` — an arm
    /// reads/writes the enclosing task's frame-local range `[base_net, base_net+locals_len)`.
    /// Threaded verbatim to the engine's `func_table` (like `has_hier_call`) so
    /// `build_func_routing` fills `SimState.func_contains_shared_fork`, and
    /// `enter_task_frame` allocates an interior-mutable ARENA window (a `WindowSlot::Shared`)
    /// for the callee so its fork arms and the parked parent reference ONE window by handle.
    /// Set by the classifier ONLY for a Case-B fork whose join mode is `join` (all) — a
    /// Case-B `join_any`/`join_none` stays LOUD until Stage 3 (refcount). `false` (the common
    /// case) ⇒ the byte-identical `WindowSlot::Owned` path. `#[serde(default)]` so an older
    /// trailer still deserialises (missing ⇒ false ⇒ prior behaviour; old artifacts never had
    /// an admitted Case-B fork, so the default is exactly correct).
    #[serde(default)]
    pub contains_shared_fork: bool,
}

/// Frame-call sidecar (B1): `Vec<FuncMeta>` index-aligned to `ir.funcs`.
/// `Default` (empty) ⇒ golden-neutral.
pub type FuncTable = Vec<FuncMeta>;

/// B2 frame-call (tasks): one task-call site's argument↔formal binding. The
/// frozen `Terminator::Call` carries only `{target, ret_bb}`, so the positional
/// mapping rides this sidecar. `in_binds[i] = (callee INPUT formal slot, arg
/// ExprId)` — evaluated in the CALLER context, written into the fresh frame.
/// `out_binds[j] = (callee OUTPUT formal slot, caller Lvalue)` — read from the
/// frame at `Return`, written back to the caller. (The `Lvalue` is the frozen
/// sim-ir type, but this whole table is out-of-band, never the golden root.)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskCallInfo {
    /// FuncId of the called task (`ir.funcs[callee].is_task`).
    pub callee: u32,
    /// (callee input-formal slot, arg ExprId) — positional copy-in.
    pub in_binds: Vec<(u32, u32)>,
    /// (callee output-formal slot, caller Lvalue) — positional copy-out.
    pub out_binds: Vec<(u32, sim_ir::Lvalue)>,
}

/// B2: task-call sites in PROCESS bodies, keyed by `(process template id,
/// process-local block id of the Call terminator)`. Consulted by the `&mut`
/// executor; outputs may target module nets. Empty ⇒ no frame-task calls.
pub type TaskCallProc = std::collections::BTreeMap<(u32, u32), TaskCallInfo>;

/// B2: NESTED task-call sites (a task call inside a task body), keyed by the
/// GLOBAL `ir.blocks` index of the Call terminator. Consulted by `&self`
/// `run_task`; outputs must be frame-local nets of the calling task.
pub type TaskCallFunc = std::collections::BTreeMap<u32, TaskCallInfo>;

/// Engine-facing side tables produced by one elaboration — ALL out-of-band
/// (`SimOpts` fields / `.velab` trailers, each serialized as its OWN postcard
/// segment for append-only compatibility); none ever enters the golden `SimIr`.
/// N7-REST: one rand field's resolved draw spec — `(field_id, width, signed, lo,
/// hi, constrained)`. `constrained` ⇒ `randomize()` draws uniformly in [lo, hi];
/// else a full-width draw over the whole field.
pub type RandBound = (u32, u32, bool, i64, i64, bool);

/// N7-REST B2: one rand field's `dist` distribution — `(field_id, [(lo,hi,weight)])`.
pub type DistField = (u32, Vec<(i64, i64, i64)>);

/// N7-REST B2: a `randc` (cyclic) field — `(field_id, lo, hi)`.
pub type RandcField = (u32, i64, i64);

/// N7-REST B-CRV final: one inline `randomize() with {…}` call's per-call extra
/// constraints — `(domain overrides, predicates)`. Indexed by a with-id passed
/// as a `Const` arg of the `ClassRandomize` SysTask. Each domain override
/// `(field_id, lo, hi)` further-narrows the class `[lo,hi]` for that call;
/// predicates are ANDed with the class predicates (IEEE §18.7 — inline
/// constraints are ADDED to the class constraints).
pub type RandWithCall = (Vec<(u32, i64, i64)>, Vec<Vec<sim_ir::COp>>);

/// OBS-1b coverage-manifest entry: one covergroup INSTANCE and its coverage items
/// (coverpoints + crosses), each with the resolved 64-bit hit-bitmap net id, bin
/// count, and weight. Engine-facing, out-of-band (golden-free) — the engine reads
/// each bitmap's final value at end-of-run to compute `coverage.json`. Built during
/// elaboration (at the covergroup `new` site, where the bitmap-name scope is correct).
#[derive(Debug, Clone)]
pub struct CovgInstMeta {
    pub inst: String,
    pub items: Vec<CovItem>,
}

/// One coverage item (a coverpoint or a cross) inside a [`CovgInstMeta`]. Mirrors
/// what `synth_cover_get` reduces: `countones(bitmap)*100/num_bins`, weighted.
#[derive(Debug, Clone)]
pub struct CovItem {
    pub name: String,
    /// `false` = coverpoint (skipped from the average when `num_bins == 0`);
    /// `true` = cross (always counted, implicit weight 1 — matches `synth_cover_get`).
    pub is_cross: bool,
    pub bitmap_net: u32,
    pub num_bins: u32,
    pub weight: u32,
}

/// One elaborated instance's static structure, for the design-hierarchy exports
/// (`--hier-tree` module tree + `--inst-paths` full-path list). Out-of-band (not in
/// the frozen sim-ir), populated at `elaborate_instance` in instance-index order.
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// The full dotted instance path from the top (`top.u_cpu.u_alu`). Empty for the
    /// top instance's own root name is avoided — the top is its module/instance name.
    pub path: String,
    /// The module name this instance elaborates.
    pub module: String,
    /// Parent instance index (into the same `Vec`), or `None` for a top root.
    pub parent: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Sidecars {
    pub fork_modes: ForkModeTable,
    pub net_names: NetNameTable,
    /// Static design hierarchy (one entry per elaborated instance, in index order) for
    /// the `--hier-tree` / `--inst-paths` exports. EMPTY ⇒ not requested / no instances.
    pub instances_info: Vec<InstanceInfo>,
    /// OBS-1b: per covergroup-instance coverage manifest (see [`CovgInstMeta`]).
    /// EMPTY ⇒ no covergroups ⇒ no `coverage.json` payload. Golden-neutral.
    pub coverage_manifest: Vec<CovgInstMeta>,
    pub proc_multipliers: Vec<u64>,
    /// Parallel per-process `S = 10^(prec − global)` (two-stage `#delay`
    /// rounding); rides `SimOpts.proc_prec_mults`. EMPTY ⇒ S = 1 everywhere.
    pub proc_prec_mults: Vec<u64>,
    pub severities: SeverityTable,
    /// StmtIds of `$timeformat` calls (no-op `Display` stmts, §21.3.2).
    pub timeformat_stmts: std::collections::BTreeSet<u32>,
    /// OBS-3: StmtIds of `$vita_stage(...)` calls (no-op `Display` stmts the engine
    /// intercepts to emit `stage.jsonl`, gated on `+STAGE_TRACE`). EMPTY ⇒ no
    /// `$vita_stage` in the design. One-shot `vita` only (velab loud-rejects it).
    pub stage_stmts: std::collections::BTreeSet<u32>,
    /// Whole-handle copy markers (§7.10): StmtId → (dst_net, src_net).
    pub handle_copy_stmts: std::collections::BTreeMap<u32, (u32, u32)>,
    /// Queue-slice markers (§7.10.1): StmtIds with args [dst, src, a, b].
    pub queue_slice_stmts: std::collections::BTreeSet<u32>,
    pub radixes: RadixTable,
    /// Per-ProcId hierarchical instance path (`"tb.u1"`) — drives `%m` (P2-11).
    /// Parallel to `processes`, like `proc_multipliers`.
    pub proc_scopes: Vec<String>,
    /// StmtIds of Force/Release stmts that are procedural assign/deassign.
    pub assign_ranks: AssignRankTable,
    /// Bounded-queue bounds (v6 ③): handle NetId → N.
    pub queue_bounds: QueueBoundTable,
    /// Unpacked-array dims for per-element VCD naming (Phase-1.x ⑤).
    pub net_dims: NetDimsTable,
    /// P2-E: ProcIds of `final` blocks (skip arming; run at end of sim).
    pub final_procs: std::collections::BTreeSet<u32>,
    /// §16.4 deferred-assert flush markers: marker StmtId → region.
    pub defer_marks: DeferMarkTable,
    /// §16.4 deferred-assert actions: action StmtId → (marker StmtId, region).
    pub defer_acts: DeferActTable,
    /// B1 frame-call metadata, index-aligned to `ir.funcs`. EMPTY ⇒ no
    /// automatic/recursive functions ⇒ golden-neutral.
    pub func_table: FuncTable,
    /// N1: FuncId → subroutine name, index-aligned to `func_table` / `ir.funcs`.
    /// Consulted only by `%m` rendered inside a frame body. EMPTY ⇒ module scope.
    pub func_names: Vec<String>,
    /// B2 frame-call: task-call sites in process bodies (executor-facing).
    pub task_calls_proc: TaskCallProc,
    /// B2 frame-call: nested task-call sites in task bodies (`run_task`-facing).
    pub task_calls_func: TaskCallFunc,
    /// SVPART: NetIds of 2-state variables (`bit`/`byte`/`shortint`/`int`/
    /// `longint`). The engine coerces X/Z→0 on every write to these (IEEE §6.11.3
    /// — a 2-state var can never hold X). EMPTY ⇒ no 2-state nets ⇒ golden-neutral.
    pub two_state_nets: std::collections::BTreeSet<u32>,
    /// N3 Phase 2 heterogeneous heap: DynArray handle NetIds whose ELEMENTS are `real`
    /// (`real r[]`). The engine flags the net `is_real` (so element read/write use the
    /// real value path) and fills `new[]` with 0.0. EMPTY ⇒ golden-neutral.
    pub real_elem_dyn_nets: std::collections::BTreeSet<u32>,
    /// N3 Phase 2 heterogeneous heap: DynArray handle NetIds whose ELEMENTS are `string`
    /// (`string s[]`). The engine stores/reads elements as `is_str` byte-strings (no
    /// bit-vector resize) and fills `new[]` with "". EMPTY ⇒ golden-neutral.
    pub string_elem_dyn_nets: std::collections::BTreeSet<u32>,
    /// WAND/WOR: NetIds declared `wand`/`wor` (wired-AND / wired-OR resolution).
    /// EMPTY ⇒ golden-neutral. Only consulted for MULTI-driven nets.
    pub wired_and_nets: std::collections::BTreeSet<u32>,
    pub wired_or_nets: std::collections::BTreeSet<u32>,
    // ── N7 class/OOP sidecars (out-of-band, golden-free) ──
    /// NetIds that are class handles (engine `class_is_handle` bitmap).
    pub class_handle_nets: std::collections::BTreeSet<u32>,
    /// `new` allocation sites: StmtId → class_id.
    pub class_new_sites: std::collections::BTreeMap<u32, u32>,
    /// Per-class field layout: `[class_id]` → `[(width, signed, four_state)]` in
    /// stable field-id order (base-class fields first).
    pub class_layouts: Vec<Vec<(u32, bool, bool)>>,
    /// SW1: per-class folded field initializers, parallel to `class_layouts`
    /// (`[class_id][field_id]` → `Some(bits)` if `= const`, else `None`).
    pub class_field_inits: Vec<Vec<Option<sim_ir::BitPacked>>>,
    /// N7-REST: per-class `rand` fields with folded constraint bounds.
    /// `[class_id]` → `[(field_id, width, signed, lo, hi, ranged)]`. `ranged` ⇒
    /// draw `dist_uniform(lo, hi)`; else a full-width seeded draw.
    pub class_rand: Vec<Vec<RandBound>>,
    /// N7-REST B2: per-class general constraint PREDICATES (`[class_id]` → list of
    /// postfix programs over candidate rand-field values). The `randomize()`
    /// rejection-sampling solver draws from `class_rand`'s `[lo,hi]` domains, then
    /// keeps a candidate only when EVERY predicate evaluates true. Inter-variable
    /// (`x < y`), `inside`, and implication all lower to these (no `[lo,hi]` fold).
    pub class_constraints: Vec<Vec<Vec<sim_ir::COp>>>,
    /// N7-REST B2: per-class `dist` weighted distributions (`[class_id]` → fields).
    pub class_dist: Vec<Vec<DistField>>,
    /// N7-REST B2: per-class `randc` (cyclic) fields (`[class_id]` → fields).
    pub class_randc: Vec<Vec<RandcField>>,
    /// N7-REST B-CRV final: per-call inline `randomize() with {…}` constraints,
    /// indexed by the with-id `Const` arg of each `ClassRandomize` SysTask. EMPTY
    /// ⇒ no inline `with` calls ⇒ golden-neutral.
    pub randomize_with: Vec<RandWithCall>,
    /// Virtual dispatch table: `[class_id][vslot]` → concrete FuncId.
    pub class_vtable: Vec<Vec<u32>>,
    /// Per method-call-site dispatch: key (StmtId/ExprId) → `(vslot, static_fid)`.
    pub class_calls: std::collections::BTreeMap<u32, (Option<u32>, u32)>,
    /// Class field-read Signal ExprId → `(field_width, field_signed)` — patches
    /// the engine width table (a field Signal's net is the 32-bit handle).
    pub class_field_widths: std::collections::BTreeMap<u32, (u32, bool)>,
    // ── SVA-REST assertion-control sidecars (out-of-band, golden-free) ──
    /// StmtIds of synthesized SVA-checker FIRE reports (`$error`/action). Gated by
    /// a standing `$assertoff`/`$assertkill` (the engine suppresses these when
    /// assertions are disabled). EMPTY ⇒ no assertions ⇒ golden-neutral.
    pub assert_fire: std::collections::BTreeSet<u32>,
    /// `$assertoff`/`$asserton`/`$assertkill` control sites: StmtId → kind
    /// (0 = off, 1 = on, 2 = kill). Lowered to a no-op `Display`; the engine
    /// flips the global assertion-enable on reach. EMPTY ⇒ no control tasks.
    pub assert_ctl: std::collections::BTreeMap<u32, u8>,
    // ── N4 clocking sidecars (out-of-band, golden-free) ──
    /// Source NetIds sampled into the PREPONED buffer at each time advance (a
    /// clocking input's bound signal). The engine snapshots these BEFORE any
    /// slot activity (the true preponed value). EMPTY ⇒ no clocking ⇒ byte-identical.
    pub clocking_inputs: std::collections::BTreeSet<u32>,
    /// Marked clocking-commit handler procs: ProcId → `[(holding_net, source_net)]`.
    /// When such a process fires (on its clocking event), the engine commits
    /// `preponed_buf[source] → holding` (blocking, same-slot — no NBA lag).
    pub clocking_commit: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// N4 clocking output pairs (staged velab→vrun must carry them or
    /// OUTPUT direction is silently dropped). ProcId → `[(source_net, holding_net)]`.
    pub clocking_outputs: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// S1 gate/assign rise·fall·turnoff delay: cont-assign index → (rise, fall,
    /// turnoff) in scaled ticks. Populated ONLY when the folded delay values are
    /// NOT all equal (so `#5`, `#(3,3)`, and no-delay stay byte-identical). The
    /// frozen `ContAssign.delay` keeps `Some(rise)` so the uniform path is
    /// untouched. EMPTY ⇒ no differing rise/fall/turnoff ⇒ byte-identical.
    pub ca_delays: std::collections::BTreeMap<u32, (u32, u32, u32)>,
}
