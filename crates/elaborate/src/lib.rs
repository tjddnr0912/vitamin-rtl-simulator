//! elaborate — lowers a parsed hdl-ast `SourceUnit` into the frozen `sim-ir`.
//!
//! Pipeline position: preprocess → lex → parse → **ELABORATE** → sim-ir →
//! engine → VCD.
//!
//! ## v1 slice (this PR)
//! INPUT: a `SourceUnit` with ONE top `ModuleDecl`, no hierarchy/instances.
//! OUTPUT: a `SimIr` populated with `nets` (from decls), `consts`/`exprs` (from
//! lowered expressions), `cont_assigns` (lowered), and one self-`Instance` for
//! the top. `processes`/`stmts`/`blocks`/`funcs` stay EMPTY — procedural-block →
//! Process/BasicBlock lowering is the NEXT slice.
//!
//! ## What v1 lowers
//! - net/var declarations (wire/reg/logic/integer + ranges/signed/arrays)
//! - 4-state integer literals (see [`literal`])
//! - continuous `assign` statements (incl. concat-LHS, bit/part selects)
//!
//! ## Deferred (NOT v1 — error path + slot noted at each site)
//! - parameter override / module instances / hierarchy flattening
//! - procedural blocks (`always`/`initial`) → Process/SuspendState/BasicBlock
//! - width/type inference + context-determined sizing
//! - generate, function/task, user `Call`
//!
//! ## Determinism (feeds the velab golden hash — see module-level note §end)
//! Nets are appended in declaration order; exprs in a fixed post-order via the
//! single [`Elaborator::push_expr`] choke point; consts are deduped through a
//! lookup-only map that never reorders the arena. No HashMap iteration ever
//! feeds arena order.

mod literal;

use std::collections::{BTreeMap, BTreeSet};

use diag::{Diagnostic, LogEvent, LogSink, MsgCode, Severity};
use hdl_ast as ast;
use literal::{
    make_const_i64, make_const_real, make_const_u32, parse_int_literal, parse_real_f64,
    parse_real_literal, parse_str_literal, parse_str_literal_text,
};
use sim_ir as ir;

/// Const-bounded `repeat`/`for` are UNROLLED (the loop counter cannot live in a
/// `SuspendState.locals` slot — `Stmt`'s `Lvalue` only addresses nets, not
/// locals, and `Stmt` is frozen). This caps the unroll so a `repeat(1_000_000)`
// ---- split modules (mechanical refactor; see the module-size policy note in the crate docs) ----
mod api;
mod array_formal;
mod array_geom;
mod arrays;
mod ast_query;
mod block_local;
mod block_local_class;
mod class_lower;
mod classes;
mod const_bound;
mod const_eval;
mod const_fn;
mod const_fn_width;
mod const_real;
mod cover;
mod cover_bins;
mod cover_synth;
mod crv;
mod da;
mod driver;
mod dynarr;
mod dynarr_method;
mod events;
mod expr_cast;
mod expr_ctx;
mod expr_main;
mod expr_special;
mod frames_body;
mod frames_call;
mod frames_classify;
mod frames_classify_fork;
mod frames_reserve;
mod generate;
mod hier;
mod hier_defer;
mod hoist;
mod iface_inst;
mod inline_fn;
mod inline_task;
mod instance;
mod instance_array;
mod limits;
mod lvalue;
mod net_util;
pub(crate) use limits::*;
// The deferred-print task list lives with the hoister that must skip those arguments;
// `lower_stmt`'s own `$sformatf` child hoist needs the same answer.
pub(crate) use hoist::is_deferred_print_task;
mod netdecl;
mod package;
pub mod packed;
mod packed_lval;
mod params;
mod ports;
mod proc_builder;
mod scope;
mod stmt_flow;
mod stmt_main;
mod string_array_route;
mod strings;
mod sva_ast;
mod sva_check;
mod sva_clocking;
mod sva_decl;
mod sva_fsm;
mod sva_liveness;
mod sva_prop;
mod sva_seq;
mod sys_special;
mod systask;
mod tables;
mod toplevel;
mod var_init;
pub use api::*;
pub(crate) use array_formal::*;
pub(crate) use ast_query::*;
pub(crate) use block_local::*;
pub(crate) use classes::*;
pub(crate) use const_eval::*;
pub(crate) use const_fn::*;
pub(crate) use const_fn_width::*;
pub(crate) use cover::*;
pub(crate) use cover_synth::*;
pub(crate) use crv::*;
pub(crate) use da::*;
pub(crate) use dynarr::*;
pub(crate) use expr_cast::*;
pub(crate) use expr_ctx::*;
pub(crate) use frames_classify::*;
pub(crate) use frames_classify_fork::*;
pub(crate) use generate::*;
pub(crate) use hier::*;
pub(crate) use hier_defer::*;
pub(crate) use inline_fn::*;
pub(crate) use lvalue::*;
pub(crate) use net_util::*;
pub(crate) use package::*;
pub(crate) use packed::*;
pub(crate) use ports::*;
pub(crate) use proc_builder::*;
pub(crate) use string_array_route::*;
pub(crate) use sva_ast::*;
pub(crate) use sva_prop::*;
pub(crate) use sva_seq::*;
pub(crate) use systask::*;
pub use tables::*;
pub use toplevel::instantiated_names;
pub(crate) use toplevel::*;
/// Public entry point. Returns `Some(SimIr)` iff no hard error was emitted;
/// every error path still produces valid placeholder arena edges so the partial
/// IR is never structurally broken (the result is simply discarded on error).
/// A recorded declaration-time pre-size: `(owner rank path, lvalue, `new[n]`)`.
pub(crate) type PresizeEntry = (Vec<u32>, ast::Lvalue, ast::Expr);
/// A recorded block-local initializer: `(declaration offset, owner rank path, lvalue, rhs)`.
pub(crate) type BlockLocalInit = (u32, Vec<u32>, ast::Lvalue, ast::Expr);

struct Elaborator<'s> {
    sink: &'s dyn LogSink,
    /// §4.5.249: resolves an AST byte span back to `file:line:col` for diagnostics.
    /// `None` on the paths that have no `SourceMap` (unit tests, `.velab` replay) —
    /// diagnostics then read exactly as before.
    span_resolver: Option<&'s dyn diag::SpanResolver>,
    /// The span of the construct currently being elaborated. Set by the statement /
    /// declaration walkers and consulted by `error`, so a diagnostic raised deep in a
    /// helper still points at the source the user wrote.
    cur_span: Option<ast::Span>,
    had_error: bool,
    /// ELAB-ERR-CAP: count of error diagnostics emitted so far. Emission is soft-
    /// capped at `MAX_ELAB_ERRORS` so a broken construct unrolled across a large
    /// generate cannot flood unbounded stderr/memory (`had_error` still latches,
    /// so the run stays loud). Parser-side precedent: `error_limit = 50`.
    error_count: usize,
    /// GEN-NET-CAP: latched once the total net arena hits `MAX_TOTAL_NETS`, so the
    /// loud "too many nets" diagnostic fires exactly once (not per subsequent
    /// `add_net`). Past it, `add_net` is a no-op → the arena stops growing.
    net_budget_blown: bool,

    // ── growing sim-ir arenas (insertion-ordered → deterministic) ──
    nets: Vec<ir::NetVar>,
    exprs: Vec<ir::Expr>,
    consts: Vec<ir::ConstVal>,
    cont_assigns: Vec<ir::ContAssign>,
    /// WAND/WOR: NetIds declared `wand`/`wor` — the engine resolves a MULTI-driven
    /// such net by wired-AND / wired-OR instead of the default wire resolution.
    wired_and_nets: BTreeSet<u32>,
    wired_or_nets: BTreeSet<u32>,
    instances: Vec<ir::Instance>,
    /// Static per-instance hierarchy info (path/module/parent) for the design-structure
    /// exports (`--hier-tree` / `--inst-paths`). Parallel to `instances`, index-aligned.
    instances_info: Vec<InstanceInfo>,

    // ── v2: procedural lowering arenas ──
    // `processes` is one Process per ProceduralBlock (module-body order).
    // `stmts` is the GLOBAL straight-line Stmt arena (SimIr.stmts); a
    // `BasicBlock.stmts` holds indices into it. The CFG basic blocks themselves
    // live INLINE in each `Process.body` (process-LOCAL indices; SimIr.blocks
    // stays empty — it is reserved for funcs, deferred past v2).
    processes: Vec<ir::Process>,
    stmts: Vec<ir::Stmt>,

    // ── lookup-only maps (NEVER feed arena order) ──
    symbols: BTreeMap<String, u32>, // fully-qualified net/var NAME → NetId
    /// Round-9: top-level `bind` decls indexed by TARGET module name → the
    /// checker instances to attach inside every instantiation of that target.
    /// Populated once in `run` (target/checker existence-validated); consumed at
    /// step (8) of `elaborate_instance`. Owned clones keep it independent of the
    /// AST borrow.
    bind_targets: BTreeMap<String, Vec<ast::ModuleInstance>>,
    const_dedup: BTreeMap<ConstKey, u32>,
    // NetId → per-dimension `(lo, size)` extents (source order) for unpacked arrays
    // whose addressing is NOT plain 0-based (`reg [7:0] g[0:1][0:2]` ⇒ [(0,2),(0,3)];
    // `mem[4:7]` ⇒ [(4,4)]). elaborate-LOCAL only — NEVER in the frozen sim-ir (NetVar
    // keeps a scalar `array_len`); a multi-index `g[i][j]` lowers to the row-major flat
    // word `(i-lo0)*s1 + (j-lo1)`, so the IR backbone is untouched. Plain 0-based 1-D
    // arrays are absent (the access path falls back to `[(0, array_len)]`).
    array_dims: BTreeMap<u32, Vec<(i64, u32)>>,
    /// Nets declared with a NEGATIVE packed low bound (`logic [3:-2] x`) → that bound.
    /// `NetVar.msb`/`lsb` are frozen `u32`, so such a net is stored NORMALIZED as
    /// `[w-1:0]` and this is what recovers the declared numbering for a bit/part select
    /// (`norm_offset_for_net`). SPARSE — absent for every ordinary net, so the common
    /// path is one `BTreeMap` miss and the lowering is byte-identical.
    ///
    /// Written ONLY where `range_to_dims_opt(.., allow_neg_lsb = true)` was used: the
    /// width and this record must be turned on together, or the net is wide while its
    /// selects address the wrong bits.
    net_decl_neg_lsb: BTreeMap<u32, i64>,
    /// The DECLARED `(msb, lsb)` of those same nets, exported as the `net_decl_ranges`
    /// sidecar so the VCD `$var` line can print `x [3:-2]` instead of the normalized
    /// `[5:0]`. Same keys as `net_decl_neg_lsb`; kept separate because that one drives
    /// select normalization (needs only the low bound) and this one drives labelling.
    net_decl_range: BTreeMap<u32, (i64, i64)>,
    /// StmtIds of `$fmonitor`/`$fstrobe` calls — the FILE-directed twins of
    /// `$monitor`/`$strobe`, which share their frozen `SysTaskId`. The engine reads
    /// `args[0]` as a descriptor for these and routes the postponed render through
    /// `file_write`. EMPTY for every design that uses neither.
    file_directed_stmts: std::collections::BTreeSet<u32>,
    /// v7 `$bits` prescan: name → (element bits, unpacked dim lengths) for the
    /// CURRENT module's body decls, recorded in declaration order during the
    /// body param-binding walk (3b) — a `localparam X = $bits(mem[0])` binds
    /// before nets lower, so the real net table can't serve it. Unfoldable
    /// decls are silently skipped (the `$bits` SITE goes loud instead).
    bits_prescan: BTreeMap<String, (u64, Vec<u64>)>,
    /// GAP-G (round-4 shadow guard): the bare names DECLARED locally by the
    /// current module — header (`#(...)`) params, ports, and top-level body
    /// nets/params — gathered from the AST ONCE before any body const-eval. A
    /// local declaration SHADOWS a same-named wildcard-imported package array
    /// (local-wins), so `const_array_vals_of_base` must NOT fold the imported
    /// array for a name in this set (→ loud, correct-or-loud). Gathered upfront
    /// (not decl-order like `bits_prescan`) so it catches a forward reference and
    /// a PORT — declaration forms a decl-order prescan would miss. A pure import
    /// is not a local declaration → absent → the intended fold proceeds.
    /// Saved/restored per module (like `bits_prescan`).
    local_decl_names: std::collections::BTreeSet<String>,
    /// DUP (round-5): per-block-local scoping decision for `automatic` block-locals
    /// whose bare name COLLIDES across DISJOINT procedural blocks (two `always`
    /// blocks each declaring `automatic int idx;`). v1 flattens block-locals to the
    /// module namespace by bare name, so such a pair would alias → the second was
    /// rejected E3009. Keyed by the declaring `Stmt::Block` span (`span.lo`) → the
    /// set of local NAMES that must be given their own `$blk$<span.lo>` scope
    /// segment (so each block gets a distinct net instead of aliasing). Computed
    /// ONCE per module by `compute_scoped_block_locals` as a pure function of the
    /// AST (both the Nets-phase hoist and the Logic-phase body lowering read it, so
    /// they derive the SAME segment). Tightly guarded (see the compute fn): only
    /// disjoint blocks, no module-net collision, no nested scoped blocks — every
    /// uncovered edge falls through to the pre-existing loud E3009 (correct-or-loud).
    /// Empty for every design with no such collision → byte-identical. Saved/
    /// restored per module.
    scoped_block_locals: BTreeMap<u32, std::collections::BTreeSet<String>>,
    /// R18-X1: bare block-local names that share ONE flattened net across two or more
    /// disjoint blocks (see `compute_coalesced_block_locals`). Read wherever the
    /// definite-assignment gate needs to know whether THIS block is the only writer of
    /// the net — the coalesce guard in `hoist` can only answer that for the second and
    /// later declaring blocks, because it keys on the net already existing. Computed
    /// ONCE per module as a pure function of the AST, so every declaring block gets the
    /// same answer regardless of hoist order. Empty for every design with no such
    /// collision ⇒ byte-identical. Saved/restored per module.
    coalesced_block_locals: std::collections::BTreeSet<String>,
    /// r18 (family D): module-process block-locals that are `automatic` WITH AN
    /// INITIALIZER and are safe to give per-entry (IEEE §6.21) semantics on the single
    /// flattened net — the initializer re-runs at BLOCK ENTRY instead of once at t0. Keyed
    /// by the declaring block's `span.lo` → the set of qualifying NAMES. Computed ONCE per
    /// module by `compute_per_entry_block_locals` (both the Nets-phase hoist and the
    /// Logic-phase Block arm read it, deriving the SAME decision). SAFE iff at most one
    /// activation of the block is live at a time: a module process's loops are sequential,
    /// so ONLY a `fork` ancestor can spawn a concurrent copy — those blocks are EXCLUDED
    /// (kept loud), as are name/module-net collisions and dyn/string decls. Empty for every
    /// design without such a decl → byte-identical. Saved/restored per module.
    per_entry_block_locals: BTreeMap<u32, std::collections::BTreeSet<String>>,
    /// v7 P2-D: package name → its const symbols (params/localparams + enum
    /// labels), folded EAGERLY in declaration order at `run()` entry.
    pkg_consts: BTreeMap<String, BTreeMap<String, i64>>,
    /// Package name → its TYPE names (every `typedef`). The parser resolves an
    /// `import p::my_t` / `p::my_t` type at parse time (it copies the scoped type twin
    /// to the bare name), so elaborate has nothing to BIND for a type import — this set
    /// exists only so `apply_import_consts` can tell a legal type import from an
    /// unknown-symbol error.
    pkg_types: BTreeMap<String, std::collections::BTreeSet<String>>,
    /// Declared `(width, signed)` of each package PARAM const (the package twin
    /// of `param_meta`). A `pkg::x` / bare-imported read materializes at this
    /// DECLARED width (`logic [3:0] x` → 4 bits) instead of the value-inferred
    /// 32 bits, so it carries the right self-width inside a concat/replication
    /// (`{4'h5, p::x}` — otherwise the 32-bit const shoves the high operand out).
    pkg_const_meta: BTreeMap<String, BTreeMap<String, (u32, bool)>>,
    /// v7 P2-D: package name → its function/task definitions (clones — the
    /// same inline-expansion tables modules use).
    pkg_funcs: BTreeMap<String, BTreeMap<String, ast::FunctionDef>>,
    pkg_tasks: BTreeMap<String, BTreeMap<String, ast::TaskDef>>,
    /// v7 P2-D: compilation-unit-scope `import` items — applied to every
    /// module elaboration (IEEE visibility is decl-order; TBs put them first).
    cu_imports: Vec<ast::ImportDecl>,
    /// P2-E: ProcIds of `final` blocks — engine side table (never the IR):
    /// skipped at arming, run once at end of simulation.
    pub final_procs: std::collections::BTreeSet<u32>,
    /// §4.5.166 HIER twin: ProcIds whose implicit `@(*)`/`always_comb`/
    /// `always_latch` read-set was inferred by `comb_read_set` (NOT a bare
    /// self-timed `always`). Recomputed after the deferred hierarchical
    /// indexed read/write resolvers patch real net+index into the arenas, so a
    /// hierarchical index (`y = dut.mem[idx]` / `dut.mem[idx] = v`) — invisible
    /// behind a sentinel at lowering time — enters the sensitivity list.
    comb_inferred_procs: Vec<u32>,
    /// B1 frame-call metadata, index-aligned to `self.funcs`/`ir.funcs`. Pushed
    /// in `lower_frame_func`; drained into `Sidecars.func_table`. EMPTY until a
    /// frame (automatic/recursive) function lowers.
    func_metas: Vec<FuncMeta>,
    /// N1: subroutine name per FuncId, parallel to `func_metas` (pushed together).
    /// Drained into `Sidecars.func_names` for `%m` inside a frame body.
    frame_func_names: Vec<String>,
    /// B1 frame-call: the GLOBAL `FuncDef` arena (→ `ir.funcs`). Accumulates
    /// across instances; index-aligned to `func_metas`. EMPTY for designs with
    /// no frame functions (golden-neutral: `ir.funcs` stays empty).
    funcs: Vec<ir::FuncDef>,
    /// B1 frame-call: the GLOBAL func-body block arena (→ `ir.blocks`). Each
    /// frame function's lowered CFG is appended here with its `Goto`/`Branch`
    /// targets rebased; `FuncDef.entry` is a global index into it.
    func_blocks: Vec<ir::BasicBlock>,
    /// B1 frame-call: names of functions that need a frame (automatic OR on a
    /// recursion cycle), and their reserved FuncId. PER-INSTANCE (saved/restored
    /// like `func_table`) so a sibling module never diverts to a stale id. The
    /// call-site divert (`inline_function`) consults this; empty ⇒ pure inline.
    frame_idx: BTreeMap<String, u32>,
    /// B2 frame-call: names of recursive TASKS needing a frame → reserved FuncId.
    /// PER-INSTANCE (saved/restored). The task-call divert (`inline_task`) emits a
    /// `Terminator::Call` + a `TaskCallInfo` when the callee is here.
    task_frame_idx: BTreeMap<String, u32>,
    /// Round-14 V3/V4: frame-TASK bodies whose reject decision is DEFERRED to a
    /// post-pass — `(fid, name, block_base, net_base, locals_len)`. Once every task is
    /// lowered, `sim_ir::compute_suspendable_tasks` gives the transitive suspendable set,
    /// so a leaf non-suspending task is lifted (the engine routes it) while a
    /// timing/nested/transitively-suspendable one stays loud (E3009). The trailing bool
    /// = the AST had a `repeat(...)` with a timing body (a shared-counter hazard — keep
    /// loud), captured from the AST before it is lost to the IR desugar.
    frame_task_pending: Vec<(u32, String, u32, u32, bool)>,
    /// B2: accumulated process-body task-call sites (→ `Sidecars.task_calls_proc`).
    task_calls_proc: TaskCallProc,
    /// B2: accumulated nested (task-body) task-call sites (→ `task_calls_func`).
    task_calls_func: TaskCallFunc,
    /// B2: true while lowering a frame-task BODY (so a nested task call registers
    /// into `pending_task_calls` keyed by its process-LOCAL block, rebased to a
    /// global `task_calls_func` key on append; false ⇒ a process body → register
    /// `task_calls_proc` directly).
    frame_task_lowering: bool,
    /// B2: nested task-call sites collected during the current task-body lowering,
    /// keyed by process-LOCAL block id (rebased by +base when the body appends).
    pending_task_calls: Vec<(u32, TaskCallInfo)>,
    /// Stage-1 fork-in-frame: fork join-modes collected during the current task-body
    /// lowering, keyed by process-LOCAL `join_bb` (rebased by +base and inserted into
    /// `fork_modes` under `FRAME_FORK_KEY` when the body appends). Empty for a task body
    /// with no fork ⇒ byte-identical to before.
    pending_fork_modes: Vec<(u32, JoinMode)>,
    /// §4.5.208: deferred hierarchical enables (`u1.tk(...)`) collected during the current
    /// frame-task body lowering (`func_block` = process-LOCAL block, rebased by +base at
    /// `lower_frame_task_body` finish, then moved to `deferred_hier_task_calls`).
    pending_hier_task_calls: Vec<DeferredHierTaskCall>,
    // NetId → per-unpacked-dimension DESCENDING flag (`mem[3:0]` ⇒ [true]).
    // Recorded only when some dim is descending (absent = all ascending);
    // array ASSIGNMENT pairs elements positionally left-to-right in DECLARED
    // index order (IEEE 1800 §7.6), so the copy expansion needs the declared
    // direction that `(lo, size)` extents erase. elaborate-LOCAL only.
    array_dim_desc: BTreeMap<u32, Vec<bool>>,
    // NetId → SYS-INTRO dimension descriptor: ordered `(left, right)` endpoints
    // per dimension (UNPACKED dims in declaration order, THEN packed dims), plus
    // the count of leading unpacked dims. Computed from the AST ranges at decl
    // time so `$size`/`$left`/`$right`/`$low`/`$high`/`$increment`/`$dimensions`/
    // `$unpacked_dimensions` const-fold (IR-0). A true scalar (`reg s`) has an
    // EMPTY descriptor (0 dims), distinguishing it from `reg [0:0] x` (1 dim) —
    // which `nv.msb`/`nv.lsb` alone cannot. elaborate-LOCAL only.
    dim_desc: BTreeMap<u32, (Vec<(i64, i64)>, usize)>,
    // SYS-INTRO잔여: the declared base type-kind per net, for `$typename`'s
    // canonical string (reg/logic/wire ⇒ "logic", integer ⇒ "integer", …).
    intro_kind: BTreeMap<u32, ast::NetVarKind>,
    // §13.4.1: a STATIC (non-automatic) task's formals are a SINGLE instance,
    // retained across calls. Inline-path tasks (all static — automatic/recursive
    // divert to the frame path) therefore share ONE formal-local net per formal,
    // allocated at the first call site and reused at every later site (keyed by
    // task name → per-formal net ids, in port order). elaborate-LOCAL only.
    task_arg_locals: std::collections::HashMap<String, Vec<u32>>,
    // Heap-handle nets (dyn array/queue/assoc) whose ELEMENT type is 2-state
    // (int/bit/byte/shortint/longint). These skip the regular `record_dim_desc`
    // path (and thus `intro_kind`), so the `two_state_nets` sidecar would miss
    // them — making `new[]` default elements to X instead of the IEEE §7.5.2
    // 2-state default of 0. Recorded separately to avoid leaking the element
    // kind into `$typename`/`$size` for the handle net. elaborate-LOCAL only.
    two_state_heap_handles: std::collections::BTreeSet<u32>,
    // N3 Phase 2: DynArray handle nets whose ELEMENT type is `real` / `string`
    // (`real r[]` / `string s[]`) — the engine needs the element kind for the
    // `new[]` default and the (non-bit-vector) element store. elaborate-LOCAL.
    real_elem_dyn_nets: std::collections::BTreeSet<u32>,
    string_elem_dyn_nets: std::collections::BTreeSet<u32>,
    // Every net DECLARED with static unpacked dims — including 1-element
    // arrays (`reg x [0:0]`), which `array_len > 1` cannot distinguish from
    // scalars (adversarial find #5). elaborate-LOCAL only.
    unpacked_array_nets: BTreeSet<u32>,
    // NetId → per-PACKED-dimension `(lo, width, ascending)` for multi-dim packed
    // arrays (`logic [3:0][7:0]` ⇒ [(0,4,false),(0,8,false)]). The net is a flat
    // `product(width)`-bit vector; a select `m[i]` is the bit-SLICE
    // `[coord*stride +: elem_width]`. `ascending` records a little-endian `[lo:hi]`
    // dim (msb<lsb): the source index maps to `coord = hi - i` instead of `i - lo`
    // (N3.3 — mirrors `norm_offset_for_net` for plain vectors). elaborate-LOCAL —
    // NEVER in the frozen sim-ir.
    packed_dims: BTreeMap<u32, Vec<(i64, u32, bool)>>,
    // v5 ⑥: the active `$` substitution while lowering a QUEUE element index —
    // the ExprId of `size(handle)-1`. Save/restore around each queue index so
    // nested selects (`q[$ - r[$]]`) bind each `$` to ITS OWN queue. `None`
    // outside a queue index ⇒ a bare `$` is loud-rejected.
    dollar_subst: Option<u32>,
    // ⓑ-breadth (v17): the active array-method `with`-clause iterator name
    // (`item` by default, or the named `find(x)` variable). When set, a matching
    // single-segment Ident lowers to `Expr::ArrayItem{index:false}` and `iter.index`
    // to `{index:true}`. `None` outside a with-expr.
    array_iter: Option<String>,
    // The (width, signed) of the iterated array's ELEMENT type, so a bare `item`
    // is sized correctly in the with-expr. `None` ⇒ default int (32, signed).
    array_iter_elem: Option<(u32, bool)>,
    // v5 ⑥ (D): interface declarations (OWNED clones — avoids threading the
    // unit lifetime) + the registry of elaborated interface INSTANCES
    // (FQ path → interface name) consulted by interface-port binding.
    ifaces: BTreeMap<String, ast::ModuleDecl>,
    iface_insts: BTreeMap<String, String>,
    /// ⓑ-breadth (§25.9): fully-qualified names of `virtual interface` handles, so
    /// the static binding assignment `vif = inst;` is skipped at statement lowering
    /// (the binding is resolved to a member alias at net-elaboration time).
    vif_handles: std::collections::BTreeSet<String>,
    /// v6 ②: symbol keys aliased through a modport whose direction is INPUT —
    /// writes through these names are loud (§25.5). Keyed at alias-copy time;
    /// empty unless a modport binding is live (zero steady cost).
    modport_readonly: BTreeSet<String>,

    // ── v3 hierarchy state ──
    // `cur_prefix` is the dotted instance path of the instance currently being
    // lowered ("tb", then "tb.dut", …). The symbol table is keyed by the FQ name
    // `cur_prefix + "." + local`, so `tb.q` and `tb.dut.q` never collide. Empty
    // only transiently (the top is always given its module name as the root path).
    cur_prefix: String,
    // FQ param-name → const value, visible while lowering an instance scope.
    // Re-points the v1 free `const_eval_u32` SLOT so `[W-1:0]` folds to a width.
    params: BTreeMap<String, i64>,
    // FQ param-name → (DECLARED width, signed), for params with a determinate
    // declared width (an explicit range or `integer`/`int`). A typed-param READ
    // materializes at THIS width — not the value-inferred 32 bits — so
    // `localparam logic [63:0] P = '1` is a 64-bit all-ones const, not `ffffffff`
    // (IEEE §6.20.2). Parallel-keyed to `params`; absent ⇒ value-inference.
    // elaborate-LOCAL (golden-neutral — only changes a typed param's const width).
    param_meta: BTreeMap<String, (u32, bool)>,
    // FQ param-name → its DECLARED packed range `(lo, width, ascending)` — for a
    // NON-zero-LSB param (`localparam [15:8] P`), so a bit/part-select `P[15:12]`
    // normalizes its offset against the declared LSB like a net (the param twin of
    // the struct/interface `dbase`). Only NON-zero-LSB params are inserted — an
    // absent entry means the classic `[N:0]`/zero-LSB raw offset (byte-identical).
    // Parallel-keyed to `param_meta` and resolved by the SAME `walk_scopes`, so the
    // offset range can never drift from the value/meta lookups. elaborate-LOCAL.
    param_range: BTreeMap<String, (u32, u32, bool)>,
    // N5: FQ param-name → RAW string literal for a `string`-typed / string-valued
    // parameter (`localparam string S = "abc"` and the untyped `localparam S = "abc"`).
    // A string param has NO i64 value, so it is kept out of `self.params` and stored
    // here; an Ident that resolves to such a param re-emits the SAME `StrUtf8` const
    // `parse_str_literal(raw)` would (so `S == "abc"` is byte-identical). elaborate-LOCAL.
    str_param_raw: BTreeMap<String, String>,
    /// r19: a REAL-valued parameter/localparam, keyed FQ, holding the FOLDED f64.
    /// Mirrors `str_param_raw`'s side-map design: a real has no i64 value, so it is
    /// kept out of `params` and resolved before the numeric param path. Stores the
    /// VALUE, not the raw text — building raw text by concatenation manufactured
    /// non-grammar strings (`-(-1.25)` became `"--1.25"`), which `parse_real_literal`
    /// silently turns into 0.0.
    real_param_val: BTreeMap<String, f64>,
    /// FQ keys of nets created by the v1 procedural block-local FLATTEN
    /// (`hoist_block_local_nets`). Such a net is one process's private variable
    /// published under the enclosing prefix's bare name, so it must NOT count as a
    /// scope shadow of an outer constant — inside a generate block its key differs
    /// from the module constant's, which would otherwise make every other reader in
    /// that scope resolve to it.
    hoisted_block_local: BTreeSet<String>,
    /// §4.5.248: the per-entry (`automatic`) block-local NAMES currently in scope
    /// during the Nets-phase hoist walk, so a NESTED block's STATIC initializer can be
    /// checked against an OUTER block's automatic (see
    /// `deny_static_init_reading_per_entry`). Pushed/popped around each block's stmts.
    per_entry_in_scope: BTreeSet<String>,
    /// §4.5.250: the `$sfmt_tmp$<n>` scratch nets the `$sformatf` hoist creates. They
    /// are written and read back within ONE statement sequence with no timing control
    /// between, so no other process can observe or clobber one — which is why the
    /// frame-call subset validator may treat a whole write to one as frame-local.
    /// §4.5.250: true while a FRAME FUNCTION body is being lowered. The `$sformatf`
    /// hoist is suppressed there — its temp is a module net, and the frame-function
    /// executor cannot write one (measured: the engine panics).
    frame_fn_lowering: bool,
    /// §4.5.252: true while lowering an expression that is GUARANTEED to be rendered by
    /// the format-aware evaluator (`format_args_str`) rather than the degenerate `eval`
    /// arm — the direct rhs of a string blocking assign, and a string `return`. Only
    /// there may a `$sformatf` lower as a plain expression node.
    sformatf_expr_ok: bool,
    // N6: FQ name of a FIXED `string` ARRAY (`string files[0:1]`) → (lo, hi, element
    // net ids in index order). A string is heap-backed with no packed width, so a fixed
    // array desugars to N scalar `NetKind::String` element nets; `files[K]` (CONST K)
    // resolves to the K-th net (read + write + `.method()`). A RUNTIME index / dynamic
    // (`string s[]`) / init-pattern is loud (correct-or-loud). elaborate-LOCAL.
    string_array_elems: BTreeMap<String, (i64, i64, Vec<u32>)>,
    // T1: DynArray net id → declared GEOMETRY, for a FIXED string array that was
    // ROUTED to the dynamic representation (`string s[n]`, `string s[1:3]`,
    // `string s[3:1]`, `string s[2][2]` … at a scope that runs the t0 var-init flush).
    // The routing is what gives a fixed string array a RUNTIME index and `foreach`,
    // which the per-element-net form above cannot express (the index would have to
    // select among N distinct nets).
    //
    // Membership means "fixed-size storage that merely happens to be dyn-backed", so
    // `new[]` on it stays LOUD — a fixed array is not resizable, and without this the
    // routing would turn today's honest reject into a SILENT resize. elaborate-LOCAL
    // (the engine only needs `string_elem_dyn_nets`, which the routed net also joins).
    fixed_string_dyn: BTreeMap<u32, StrArrayGeom>,
    // T1: FQ name of a routed fixed string array → its DynArray net. The net is
    // registered under a MANGLED name (`<name>$sad`), never the declared one, so the
    // bare name stays FREE in the module namespace — exactly as the per-element-net
    // form leaves it free by storing elements as `<name>$sae$<i>`.
    //
    // That is load-bearing, not cosmetic: v1 flattens block-locals onto module nets by
    // BARE NAME, so putting the array on its own name made a block-local `logic [7:0]
    // sa` collide with a module `string sa[2]` and hit the dynamic-storage collision
    // reject — two designs iverilog runs, and vita ran correctly before the routing,
    // went loud. Keeping the name free restores that namespace exactly.
    fixed_string_dyn_key: BTreeMap<String, u32>,
    // PERSISTENT FQ param-name → value, NEVER restored (unlike `params`). Lets a
    // post-elaboration hierarchical READ (`dut.WIDTH`) fold to the sibling
    // instance's param value. Out-of-band (golden-free).
    hier_params: BTreeMap<String, i64>,
    // Family D (r17): PERSISTENT FULLY-QUALIFIED `<instance-path>.<fname>` → per-instance
    // FuncId, NEVER restored. Only HIER-CALLABLE framed functions (input-only scalar
    // formals, non-string return) are registered — a call resolving to anything else
    // finds no entry → loud (correct-or-loud). Lets a hierarchical call `u1.f(x)` bind to
    // the callee instance's FuncId (which already baked in that instance's nets/params).
    // Reused by `hier_resolve` (the §23.6 commit-to-scope walk) exactly like `symbols`.
    hier_funcs: BTreeMap<String, u32>,
    // Family D (r18): the TASK twin of `hier_funcs` — PERSISTENT `<inst-path>.<tname>` →
    // per-instance frame-TASK FuncId. Only HIER-CALLABLE framed tasks (input-only scalar
    // formals) are registered; a hier task enable `u1.tk(x)` resolves through this via
    // `hier_resolve`. An out/inout/array/string formal or a non-framed (static) task gets
    // no entry → loud (correct-or-loud).
    hier_tasks: BTreeMap<String, u32>,
    // §4.5.200: task NAMES that appear as the target of a hierarchical enable (`u1.tk(...)`
    // — the last segment of a 2+-segment `UserTaskCall`) ANYWHERE in the design. Such a task
    // must be FORCE-FRAMED (`build_task_frame_set`): a STATIC (non-automatic) task is
    // otherwise inlined and has no per-instance FuncId for the §4.5.197 hier defer/resolve
    // to bind, so a hier call to it stays loud. Collected ONCE (pre-scan in `run`) before any
    // per-module framing; frame ⊇ inline (§4.5.198/199) means force-framing never regresses
    // the task's LOCAL callers, and name-based over-collection is harmless. NEVER restored.
    hier_called_task_names: std::collections::BTreeSet<String>,
    // §4.5.201: per hier-callable frame-TASK FuncId, the declared port DIRECTIONS (parallel
    // to the formals). `resolve_deferred_hier_task_call` reads it to route each deferred arg
    // to an in-bind (input/inout copy-in) and/or an out-bind (output/inout copy-out) — the
    // direction is unknown at the call site (the callee instance isn't elaborated yet).
    hier_task_port_dirs: BTreeMap<u32, Vec<ast::PortDir>>,
    // `defparam top.u.N = 7;` overrides, keyed by the FULLY-QUALIFIED target
    // instance path → [(param-name, const value)]. Collected in pass 7 (when the
    // parent's FQ prefix is current) and consumed by the child's `bind_params` in
    // pass 8 — so the override is registered before the child binds. v1 supports a
    // DIRECT-child `inst.param` only (deeper paths / non-const values are loud).
    defparams: BTreeMap<String, Vec<(String, i64)>>,
    // module names on the active instantiation path — the recursion cycle guard.
    inst_stack: Vec<String>,
    // Instance id of the instance whose body is currently being lowered. Set in
    // `elaborate_instance` step (2) (saved/restored like `cur_prefix`), so a
    // child instance created from *inside* a generate block (`elaborate_generate`
    // → `lower_gen_module_item`) can record the correct `Instance.parent` without
    // threading the id through every generate-walk call.
    cur_inst: u32,

    // ── user function/task inlining (SD2 inline path) ──
    // name → def (OWNED clone), populated per-module from ModuleItem::Func/Task in
    // `elaborate_instance` BEFORE lowering that module's logic. Cleared/restored
    // per instance scope so a child module never sees a parent's functions by bare
    // name (matches the per-instance net isolation of `walk_scopes`). Cloning the
    // small defs sidesteps threading an AST lifetime through the whole driver; the
    // tables are point-queried only (BTreeMap), never iterated into arena order.
    func_table: BTreeMap<String, ast::FunctionDef>,
    // §4.5.186 constant-function evaluation: the SAME per-module function defs as
    // `func_table`, but populated EARLIER (before the module's parameter fold) so a
    // `localparam W = f(N)` can interpret `f` at compile time (`func_table` itself is
    // populated in pass 3.5, AFTER the param fold in pass 3). Saved/restored per
    // instance scope alongside `func_table`.
    const_func_table: BTreeMap<String, ast::FunctionDef>,
    task_table: BTreeMap<String, ast::TaskDef>,
    // R19-X1: the scope prefix in force when `func_table`/`task_table` were collected —
    // i.e. the scope in which every function/task in them is DECLARED. Saved/restored
    // with those tables. Read only by `default_binding_matches_decl_scope`: a filled
    // DEFAULT argument value is lowered in the CALLER's scope, but IEEE 1800 §13.5.4
    // evaluates it in the subroutine's own, and the two can name different objects.
    tf_decl_scope: String,
    // R5-B: names of FRAME functions that have an output/inout formal. A call to
    // one carries copy-out (like a task) plus a return value, so it is lowered as a
    // `Terminator::Call` (statement context) via `emit_frame_func_out_call` rather
    // than a pure `Expr::Call`. EMPTY for any design without such a function, so the
    // hoist pre-pass in `lower_stmt` is skipped and all other code is byte-identical.
    inout_func_names: std::collections::BTreeSet<String>,
    // §4.5.179: names of FRAMED functions with an `input` dynamic-array formal (the set
    // §4.5.177 blesses on the direct-rhs `x = f(arr)` path). A call to one BURIED in a
    // larger expression (`$display(f(a))`, `r = f(a)+1`, `if (f(a) > 0)`) is hoisted to a
    // temp `__t = f(a)` — itself a direct-rhs blocking assign that re-triggers §4.5.177's
    // snapshot marker. EMPTY for any design without such a function → the hoist pre-pass in
    // `lower_stmt` is skipped and all other code is byte-identical.
    dyn_formal_func_names: std::collections::BTreeSet<String>,
    // Named SVA declarations (Phase-3 named-SVA slice): bare name → decl, collected
    // per-instance like func_table/task_table (saved/restored so siblings don't
    // inherit). Kept SEPARATE from the net symbol table so a net and a sequence of
    // the same name coexist — only the assert-property-instance position and
    // `expand_sequence`'s bare-ident leaves consult these (the func/task-name
    // namespace precedent). Inlined at use sites → pure IR-0, golden untouched.
    seq_table: BTreeMap<String, ast::SeqDecl>,
    prop_table: BTreeMap<String, ast::PropDecl>,
    /// SVA-REST `let NAME [(formals)] = expr;` declarations, inlined at use sites
    /// (the named-sequence precedent). Pure IR-0.
    let_table: BTreeMap<String, ast::LetDecl>,
    // Recursion guard for named-sequence inlining (separate from `inline_stack`,
    // which is empty by the time SVA checkers materialize, but a dedicated stack
    // keeps SVA correctness independent of func/task inline state).
    sva_inline_stack: Vec<String>,
    /// SEQ-DEPTH: live `expand_sequence` recursion depth, capped so a pathological
    /// 35k-deep nested `##`/`[*]` sequence is a loud reject, not a stack overflow.
    sva_seq_depth: u32,

    // ── N7 class/OOP ──
    // All declared classes, name → resolved metadata. Built by a whole-design
    // prescan (forward-reference safe, like seq_table) before any lowering, so a
    // class used before its textual decl still resolves. `class_order[id]` is the
    // inverse (stable class_id → name), keeping the engine sidecars (layouts/
    // vtable, indexed by id) aligned.
    class_table: BTreeMap<String, ClassInfo>,
    class_order: Vec<String>,
    // A class-handle NetId → its declared (STATIC) class name. Drives `obj.field`
    // / method resolution (the layout + method set come from the static type;
    // virtual dispatch refines to the dynamic type at run time).
    net_class: BTreeMap<u32, String>,
    // While lowering a method body: the `this` handle net + its class name, so a
    // bare field / `this.field` resolves against the enclosing object.
    cur_this: Option<(u32, String)>,
    // While lowering a method/function body: `(Some(return-var net) | None for a
    // void task, exit BB)`. A `return [expr]` assigns the return var and jumps to
    // the exit block. `None` outside any method body (a stray `return` is loud).
    cur_return: Option<(Option<u32>, BlockId)>,
    // Frame-local discard slot for the method body currently lowering (the net a
    // nested void call writes its result into). `None` in a process body, where a
    // fresh module net is used instead.
    cur_discard: Option<u32>,
    // True while lowering a FRAME function/task/method body — executed by
    // `run_frame_call`, which cannot honor the interpreter's sys-read StmtEffect.
    // A bare `$sscanf`/`$fscanf`/`$fgets`/`$fread` in such a body must NOT route
    // through `emit_discarded_call` (it would loud-reject on the module discard
    // net, or silently no-op); it keeps the pre-existing `lower_systask`
    // (warn+skip) path. `false` in a process body AND an INLINE task body (both
    // lowered into the process statement stream, where the sys-read DOES execute).
    in_frame_body: bool,
    // §4.5.177: set ONLY while lowering the rhs of a blessed direct-rhs call
    // `x = f(dynarr)` at module-process level — where `lower_stmt` has already emitted the
    // `handle_copy` snapshot marker that fills the callee's dyn-array formal heap slot.
    // `emit_frame_call` binds a function's `input` dyn-array formal ONLY when this is set;
    // every other call context (nested in a bigger expr, inside a `&self` subroutine body,
    // etc.) has no marker → the formal would read empty → loud (correct-or-loud by
    // construction: no marker ⇒ no support).
    dyn_formal_call_ok: bool,
    // Frame-local nets that came from an UNPACKED-array decl (`reg [7:0] mem [0:3]`).
    // These reserve as a 1-elem net (the array is outside the frame-call subset), so a
    // `mem[k]` select mis-lowers to a bit-select of that 1-elem net — `validate_frame_
    // body` must keep rejecting any such select/write (EXT2-H allows part-selects of a
    // genuine scalar, NOT a collapsed array). Elaborate-transient (not a Sidecar).
    frame_array_local: std::collections::BTreeSet<u32>,
    // §13.3 UARR: nets that are an unpacked-array FUNCTION FORMAL lowered as an
    // md-packed frame slot (`input logic [63:0] words [0:7]` → `[7:0][63:0]`) → its
    // classified `ArrayFormal` shape. A WHOLE read of such a formal (`arr` not
    // `arr[i]`) would return the flat vector — silently wrong for a scalar context —
    // so the whole-name read choke point loud-rejects it (only element reads `arr[i]`
    // are supported). The shape is retained so a SIBLING array-formal ACTUAL (round-7
    // UARR2: `f(a,i)` forwarding `f`'s own formal `a` to `g`) can pass the whole
    // md-packed value through when the caller/callee formals match. Elaborate-transient.
    frame_arr_formal_meta: std::collections::BTreeMap<u32, ArrayFormal>,
    // Sidecar accumulators (drained into `Sidecars`):
    class_handle_nets: std::collections::BTreeSet<u32>,
    class_new_sites: std::collections::BTreeMap<u32, u32>,
    class_vtable: Vec<Vec<u32>>,
    class_calls: std::collections::BTreeMap<u32, (Option<u32>, u32)>,
    class_field_widths: std::collections::BTreeMap<u32, (u32, bool)>,
    /// How far `index_self_width`'s placeholder scan has got. Everything below
    /// it is known placeholder-free FOREVER: a resolved node is never turned
    /// back into a `POISON_*` placeholder (the deferred-hierarchy passes only
    /// ever patch a placeholder INTO a resolved node), so "clean" is permanent
    /// and the scan is amortized across the whole elaboration instead of being
    /// re-run per indexed select. Measured: without it picorv32 paid 4.3%.
    selfw_scan: u32,
    /// Scratch for `index_self_width`'s PROVISIONAL path (§4.5.310) — reused
    /// across calls so a design with a hierarchical reference does not allocate
    /// an arena-sized buffer per indexed select. Only the queried subtree's ids
    /// are meaningful in it; every other slot is stale by design.
    selfw_scratch: Vec<sim_ir::selfwidth::SelfWidth>,
    /// Memo for `index_self_signed` (§4.5.309) — one `SelfWidth` per ExprId,
    /// filled forward and never invalidated, because an expression's
    /// self-width cannot change once it is pushed.
    selfw_cache: Vec<sim_ir::selfwidth::SelfWidth>,
    // N7-REST B-CRV final: per-call inline `randomize() with` constraints, pushed
    // (and indexed) as each `obj.randomize() with {…}` is lowered.
    randomize_with: Vec<RandWithCall>,
    // SVA-REST assertion control: StmtIds of synthesized assertion-fire reports +
    // `$assertoff/on/kill` control sites. `in_assert_synth` is true while a
    // synthesized SVA checker body is lowering, so each fire `$error`'s StmtId is
    // captured into `assert_fire`.
    assert_fire: std::collections::BTreeSet<u32>,
    assert_ctl: std::collections::BTreeMap<u32, u8>,
    in_assert_synth: bool,
    // N4 clocking: source NetIds to snapshot in the preponed buffer + marked
    // commit-handler ProcId → [(holding_net, source_net)]. `clocking_events` maps a
    // clocking-block name to its clocking event so `@(cb)` lowers to `@(clk)`.
    clocking_inputs: std::collections::BTreeSet<u32>,
    clocking_commit: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// N4 clocking output pairs collected per-clocking-block commit proc.
    /// Flushed into `SimOpts::clocking_outputs` at elaboration end.
    clocking_outputs: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    /// S1 gate/assign rise·fall·turnoff delay: cont-assign index → (rise, fall,
    /// turnoff). Populated only when the folded values are not all equal.
    ca_delays: std::collections::BTreeMap<u32, (u32, u32, u32)>,
    clocking_events: std::collections::BTreeMap<String, ast::Sensitivity>,
    /// Holding NetIds of clocking INPUTs (`cb.sig`) — read-only; an lvalue write
    /// to one is loud (you cannot drive a clocking input, §14.3).
    clocking_hold_nets: std::collections::BTreeSet<u32>,
    /// R17: NetIds created by FLATTENING an `automatic` procedural block-local into
    /// the module namespace. IEEE 1800 §23.9 forbids a hierarchical reference to an
    /// automatic variable — it has no static address to name — but v1's flatten gives
    /// it one, so `other.tb.a` silently resolved to the block-local's net and read or
    /// wrote per-entry storage from outside. Measured: iverilog rejects the same
    /// program ("Hierarchical reference to automatically allocated item"), vita printed
    /// the poked value. Elaborate-local (never serialized).
    automatic_local_nets: std::collections::BTreeSet<u32>,
    /// A2a: NetIds of DESUGARED array parameters (`localparam int RHO[0:4] =
    /// '{…}` — stored as an ordinary variable array) → the source name. Any
    /// write to one (assignment / force / $readmem / SYS-READ dest / task
    /// output actual) is a loud error: a parameter is an elaboration constant
    /// and must never be silently mutable. Elaborate-local (never serialized).
    const_param_nets: std::collections::BTreeMap<u32, String>,
    /// GAP-G: fq name of a DESUGARED 1-D unpacked array parameter (`localparam
    /// int ROT[0:3] = '{0,1,3,5}`) → its const-folded element values, in
    /// declared index order. Populated when the const-param net is created (its
    /// `'{…}` init elements are all foldable scalars); lets `const_eval_in_scope`
    /// fold an element read `ROT[i]` in a constant context — e.g. a generate-scope
    /// `localparam R = ROT[g]`. A multi-dim / non-foldable array is simply absent
    /// (its element reads stay loud — correct-or-loud). Elaborate-local only.
    array_const_vals: std::collections::BTreeMap<String, Vec<i64>>,
    /// GAP-G (round-4): package name → (const array parameter name → element
    /// values), the package-scope twin of `array_const_vals`. A package array
    /// param (`package p; localparam int ROT[0:3]='{…}`) lowers to a `$pkg$p`
    /// net, so its elements are not in the module-scope `array_const_vals`;
    /// captured here during `elaborate_package` so a const read of an element
    /// folds whether the array is named by an explicit `p::ROT[i]` or by a bare
    /// `ROT[i]` made visible via `import p::*`. Same shape rules as
    /// `array_const_vals` (0-based ascending single-dim, all-foldable init);
    /// anything else is absent → loud. Elaborate-local only.
    pkg_array_const_vals:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<i64>>>,
    /// A2a: true while lowering the synthesized §6.8 decl-init `initial` —
    /// a const param's own initializer is legitimate, so the deny is off.
    lowering_decl_init: bool,
    /// A2b-prereq: package name → (var name → NetId) for package-level
    /// VARIABLES (IEEE §26: one storage instance per elaboration, shared by
    /// every module). The nets live under the reserved `$pkg$<pkg>` scope
    /// (not a spoofable user path — an escaped identifier keeps its leading
    /// backslash in the AST name, so `\$pkg$p` never string-matches) and are
    /// excluded from the VCD in v1 (iverilog parity: it dumps no package
    /// vars either). Elaborate-local (never serialized).
    pkg_vars: std::collections::BTreeMap<String, std::collections::BTreeMap<String, u32>>,
    /// A2b-prereq: fq alias key → (origin package, is_explicit) for names
    /// bound into a module scope by an `import pkg::*`/`import pkg::name`
    /// VARIABLE import (the alias itself lives in `symbols`, interface-alias
    /// precedent). Consulted by `add_net`: a LOCAL declaration shadows a
    /// WILDCARD import (iverilog-pinned), colliding with an EXPLICIT import
    /// is an error (iverilog-pinned).
    pkg_var_aliases: std::collections::BTreeMap<String, (String, bool)>,
    /// A2b-prereq (adversarial sound S2): fq keys of DECLARED genvar names.
    /// A genvar binds into `params` only transiently (during loop unroll), so
    /// the constant-shadow guard needs this persistent record — otherwise a
    /// procedural reference to a genvar-named import alias would silently
    /// resolve to the package variable outside the unroll.
    genvar_decls: std::collections::BTreeSet<String>,
    /// Every clocking-block name in the whole design (never cleared) — diagnostic
    /// only: lets a cross-hierarchy `@(inst.cb)` event control emit an accurate
    /// "unsupported clocking-event" message instead of a generic hier-name error.
    all_clocking_names: std::collections::BTreeSet<String>,
    /// Auto-increment counter for anonymous clocking blocks (N4-v2b).
    anon_clocking_count: u32,

    // Substitution scope: a formal-param NAME currently bound to an actual ExprId
    // (a function/task INPUT formal during inlining). `lower_expr`'s Ident arm
    // consults this FIRST: a bare single-segment Ident matching a key lowers to the
    // bound ExprId, not a net — exactly like `Paren` unwrapping (no new IR node). A
    // Vec used as a stack so nested inlining + shadowing resolve innermost-wins via
    // reverse linear scan. Empty in steady state (one `is_empty`/scan on the hot
    // path costs nothing).
    subst: Vec<(String, u32)>,
    // Output/inout task formal NAME → caller NetId. Consulted in BOTH `lower_expr`
    // (read) and `collect_lval_chunks` (write) so a formal resolves to the caller's
    // net in either position. Symmetric Vec stack with `subst`.
    out_subst: Vec<(String, u32)>,
    // R2: a READ-ONLY `input` dynamic-array formal NAME → the caller's DynArray NetId
    // (an alias). Consulted ONLY on read paths (`.size()`/`b[i]`) so `b.size()` reads
    // the caller's `dyn_heap[a]` directly. WRITE paths (`b[i]=`/`b=new[]`/`push_back`)
    // never consult this — they miss `symbols` and stay loud, so a read-only input
    // aliases for free while writes/inout/output remain correct-or-loud. Vec stack
    // (pushed at the inline call, popped after the body), mirroring `out_subst`.
    dyn_subst: Vec<(String, u32)>,
    // v7 P2-C: FORMAL name → declared `string`-ness, for the FUNCTION body being lowered
    // (inline OR frame). A `string` relational compare (`a < b`) routes through `StrCmp`
    // only if an operand is string-domain; a formal is not otherwise detectable (an
    // inline literal actual is not a string net; a frame formal is not in `subst`), so a
    // `string`-declared formal compare silently did a PACKED compare. A symmetric Vec
    // stack (innermost-wins via `rev`) so nested inline calls / a shadowing inner
    // non-string formal resolve to the correct declared type. (Not populated for TASKs:
    // an inline task copies its formals to `__taskarg_` locals whose string-compare is a
    // SEPARATE pre-existing gap; a frame task would be a clean mirror — a follow-on.)
    formal_str: Vec<(String, bool)>,
    // Recursion guard: function/task names on the active inline-expansion stack. A
    // name found here when we try to inline it = direct or mutual recursion =
    // E-ELAB-UNSUPPORTED (SD2: recursive ⇒ frame-call, deferred). Mirrors
    // `inst_stack`.
    inline_stack: Vec<String>,

    // ── fork/join concurrency state (engine-facing side channel, NOT in SimIr) ──
    // `fork_modes` maps (cur_proc, join_bb) → JoinMode for every fork lowered; it
    // is threaded into the engine via SimOpts.fork_modes (never the golden root).
    fork_modes: ForkModeTable,
    // `cur_proc` is the ProcId the process currently being lowered WILL occupy when
    // the caller pushes it (== self.processes.len() at lower_proc_block entry). Any
    // record_fork_mode during that body is keyed by exactly this id, so the engine's
    // (template, join_bb) lookup is guaranteed to hit.
    cur_proc: u32,
    // Nesting guard: true while lowering a fork CHILD body. A `Stmt::Fork` seen with
    // `in_fork == true` is the nested case → hard ElabUnsupported error (v1 MVP cut).
    in_fork: bool,
    // `disable` lowering state: (label, exit-BB) per lexically-enclosing NAMED
    // begin-block of the statement being lowered. Exit BBs are allocated LAZILY
    // (pre-scan: only labels some `disable` in the block body actually targets),
    // so designs without disable lower to byte-identical CFGs (golden corpus
    // untouched). `disable_fork_floor` is the stack depth at the current
    // fork-child boundary: a child may only disable blocks INSIDE its own body —
    // a Goto across the fork boundary would bypass the join barrier.
    disable_stack: Vec<(String, BlockId)>,
    disable_fork_floor: usize,

    // ── timescale state (engine-facing side channel, NOT in SimIr) ──
    // module NAME → its delay-unit exponent (base-10 of seconds), and the design-wide
    // finest precision exponent (the global tick base). Both supplied by the glue from
    // `hdl_preprocess::resolve_module_timescales`. Empty/`-9` ⇒ the `1ns/1ns` base
    // (multiplier 1 everywhere → byte-identical to the pre-timescale lowering).
    mod_unit_exp: std::collections::BTreeMap<String, i8>,
    // module NAME → its OWN precision exponent (two-stage delay conversion —
    // empty for the legacy 4-arg entry points ⇒ every module's prec == global
    // ⇒ stage-1 is the identity, byte-identical to the single-round behavior).
    mod_prec_exp: std::collections::BTreeMap<String, i8>,
    global_prec_exp: i8,
    // `--top` root override (worklib / multi-top selection): when `Some`, these
    // units are the roots — in the given order — instead of `pick_roots`.
    root_override: Option<Vec<String>>,
    /// Is `` `default_nettype none `` in effect for the module being elaborated?
    /// Saved/restored around each `elaborate_instance`, because the directive governs
    /// the module DECLARATION site, not the instantiation site — a `none` module may
    /// instantiate a `wire` one and vice versa.
    pub(crate) cur_nettype_none: bool,
    /// FQ names of nets created by [`Elaborator::declare_implicit_net`]. Only these get
    /// the width-truncation warning — an explicitly declared 1-bit net driven by a wider
    /// expression is the author's choice, not a §3.5 surprise.
    pub(crate) implicit_nets: std::collections::BTreeSet<String>,
    // Delay multiplier `M = 10^(unit_exp − global_prec_exp)` of the module CURRENTLY
    // being lowered (saved/restored around each `elaborate_instance`, like cur_prefix).
    // `#delay` literals scale by this; `$time`/`$realtime` divide by it (per process).
    cur_time_mult: u64,
    // `S = 10^(prec_exp − global_prec_exp)` of the module CURRENTLY being lowered:
    // one step of its OWN precision in global ticks (1 when prec == global).
    cur_prec_mult: u64,
    // Per-ProcId multiplier table (parallel to `processes`), threaded to the engine via
    // `SimOpts.proc_multipliers` for `$time`/`$realtime` scaling. NEVER in the golden root.
    proc_multipliers: Vec<u64>,
    // Parallel per-process `S = 10^(prec − global)` table (two-stage `#delay`
    // rounding: real delays = `round(d × M/S) × S`). NEVER in the golden root.
    proc_prec_mults: Vec<u64>,

    // ── severity-task state (engine-facing side channel, NOT in SimIr) ──
    // StmtId → SeverityKind for every `$fatal`/`$error`/`$warning`/`$info` lowered
    // (each as a `SysTaskId::Display` stmt). Threaded via `SimOpts.severities`.
    severities: SeverityTable,
    // StmtIds of `$timeformat` calls (each a no-op `SysTaskId::Display` stmt —
    // the assert_ctl/severity pattern). Threaded via `SimOpts.timeformat_stmts`.
    timeformat_stmts: std::collections::BTreeSet<u32>,
    stage_stmts: std::collections::BTreeSet<u32>,
    // Whole-handle copy markers (§7.10 `dst = src` deep copy): no-op Display
    // StmtId → (dst_net, src_net). Threaded via `SimOpts.handle_copy_stmts`.
    handle_copy_stmts: std::collections::BTreeMap<u32, (u32, u32)>,
    // Family C (r17): SUBSET of `handle_copy_stmts` that are dyn-array-formal snapshot
    // markers living inside a frame body — the only handle-copy markers the `&self`
    // frame executors run and the subset validator allows. Threaded via SimOpts.
    dyn_formal_marker_stmts: std::collections::BTreeSet<u32>,
    // Family D (r17): StmtIds of GENUINE `$display`/`$write` prints (NOT a severity /
    // timeformat / stage / marker Display — those return early with their own table).
    // `classify_frame_body` admits only these in a subset function/task body, and the
    // `&self` executors render them. Validator-only (no serialization): the engine
    // render arm keys on the plain Display/Write shape reaching it after the marker arm.
    frame_print_stmts: std::collections::BTreeSet<u32>,
    // Queue-slice markers (§7.10.1 `dst = src[a:b]`): no-op Display StmtIds
    // whose args are [dst, src, a, b]. Threaded via `SimOpts.queue_slice_stmts`.
    queue_slice_stmts: std::collections::BTreeSet<u32>,
    // StmtId → default radix (2/8/16) for the b/o/h print-task variants (P1-5).
    radixes: RadixTable,
    // StmtIds of Force/Release stmts that are procedural assign/deassign (§9.3.1
    // weak rank — see [`AssignRankTable`]).
    assign_ranks: AssignRankTable,
    // Bounded-queue bounds (v6 ③): handle NetId → N.
    queue_bounds: QueueBoundTable,
    // NetIds of named events (v5 batch B): each `event e` is a 64-bit counter
    // Reg (init 0). `->e` increments it; `@(e)` is plain AnyEdge sensitivity.
    // The set guards the VALUE surface — an event cannot be read or written.
    event_nets: std::collections::BTreeSet<u32>,
    // Per-ProcId instance path for `%m` (P2-11); lockstep with `processes`.
    proc_scopes: Vec<String>,
    // §6.8: a VARIABLE declaration initializer whose value is NON-constant
    // (`logic [7:0] b = a;`) is a one-time assignment at time 0, equivalent to
    // `initial b = a;`. A constant init folds into the net's `init` field; a
    // non-constant one is collected here (in declaration order) and drained into
    // ONE synthesized `initial` process after the module's item loop, so `b`
    // sees `a`'s value instead of silently keeping its X/0 default. (lvalue, rhs).
    pending_var_inits: Vec<(ast::Lvalue, ast::Expr)>,
    // A routed string array's decl-time `new[n]` pre-size, which must precede the element
    // writes. Pushed at DECLARATION time with a BARE name, so a declaration inside a
    // GENERATE scope emitted an lvalue that resolved to `t.s` instead of `t.gb[0].s`: the
    // generate walk only isolates `pending_var_inits` during its VarInit phase, and this is
    // pushed during Nets. Keyed by the `cur_prefix` in effect at the declaration, so each
    // scope drains its own at its own flush point (module scope is the `""` key, and its
    // drain order is unchanged — byte-identical for every design without a generate-scope
    // string). Spliced to the FRONT of that scope's collected inits.
    /// `Vec<u32>` = the OWNING scope's rank path. A `case` arm and an unlabeled `if`/`begin`
    /// body share the enclosing prefix, so the key alone cannot say which scope's flush owns
    /// the entry, and a pre-size drained by the WRONG one runs `new[n]` after the element
    /// writes and wipes them. §4.5.265: this was a `bool` (inside a generate body or not),
    /// which cannot separate two NESTED generate scopes that both share a prefix — the rank
    /// path can, and is stable across the elaboration phases by construction.
    pending_scoped_presize: BTreeMap<String, Vec<PresizeEntry>>,
    /// §4.5.254: EVERY block-local declaration initializer, keyed by the FULL prefix it
    /// lives under — the instance/generate prefix for a flattened one, plus a `$blk$<lo>`
    /// segment for one that earned its own scope. `u32` is the declaring name's source
    /// offset, the DECLARATION ORDER key.
    ///
    /// Measured against iverilog: every module-scope static initializer runs before every
    /// block-local one (`int m = $random;` before a `begin int a = $random;` regardless of
    /// which is written first), and block-locals then run in declaration order among
    /// themselves. Both halves need this one list: `hoist_block_local_nets` runs BEFORE
    /// `collect_var_init_drivers`, so a block-local pushed straight into `pending_var_inits`
    /// landed ahead of every module-scope init; and holding only the STRING ones back (what
    /// r19 did) reordered a block against its own non-string declarations.
    /// `Vec<u32>` = the OWNING scope's rank path (see `pending_scoped_presize`).
    pending_block_local_inits: BTreeMap<String, Vec<BlockLocalInit>>,
    /// Is the walk currently inside a generate body? A generate body is a scope to
    /// iverilog even when vita mints no prefix segment for it (`case` arms and unlabeled
    /// `if`/`begin` bodies share the enclosing prefix), and its initializers run BEFORE
    /// the enclosing scope's own. Prefix alone cannot distinguish those, so the flag does.
    in_generate_body: bool,
    /// §4.5.256 — where the scope being elaborated sits in the STATIC-INITIALIZATION
    /// order, as a path of `(slot, seq)` pairs compared lexicographically. Measured
    /// against live iverilog, that order is NOT the order vita creates the processes in:
    ///
    /// - a MODULE initializes ① its generate scopes ② its child instances ③ its own
    ///   variables ④ its own block-locals;
    /// - a GENERATE scope initializes ① its child instances ② its own variables
    ///   ③ its own block-locals ④ its nested generate scopes.
    ///
    /// Expressing it as data is what lets the two axes separate: OWNERSHIP (which flush
    /// drains which pending entry) stays innermost-first, because a scope must claim its
    /// own before an ancestor sees it, while INITIALIZATION order is this path. Trying to
    /// carry both on the elaboration pass order is what made the earlier attempts trade
    /// one wrong order for another.
    rank_path: Vec<u32>,
    /// Which BAND the instance scope being entered belongs to: 0 = declared in the
    /// enclosing scope's own body (or a root), 1 = injected by a `bind`. A bind directive
    /// has no position inside the target module, so the two cannot share a key space.
    rank_band: u32,
    /// Per-SLOT monotonic counters for the CURRENT scope, saved/restored on entry so each
    /// scope numbers its own children. One counter per slot, not one per scope: the four
    /// generate walks visit the same generates, but only the Instances walk visits
    /// `Instance` items, so a shared counter handed a generate a different number in the
    /// VarInit and Instances phases — and a child instance was then filed under a rank
    /// path that no longer matched its own generate's flush.
    rank_seq: [u32; 4],
    /// ProcId → its initialization rank. Only synthesized decl-init processes appear;
    /// everything else runs after all of them (IEEE 1800 §6.21 — a static initializer is
    /// assigned "before any initial or always block starts", which vita had been
    /// approximating with "gets a lower ProcId" and losing across an instance boundary).
    init_ranks: BTreeMap<u32, Vec<u32>>,
    // v8 SVA: concurrent assertions collected during statement lowering, drained
    // into synthesized clocked checker processes after each module's process loop.
    pending_sva: Vec<PendingSva>,
    // SVA-REST: `cover property` statements collected during lowering, drained into
    // synthesized counter + end-of-sim `$display` processes (golden-free).
    pending_cover: Vec<PendingCover>,
    // N3: hierarchical READ references (`tb.dut.x`) collected during expression
    // lowering. A downward ref cannot resolve at lowering time (the child instance's
    // nets are created in pass 8, AFTER the parent body is lowered in pass 7), so each
    // is recorded with a PLACEHOLDER `Signal` expr id + the lowering-time scope prefix
    // and dotted path, then resolved against the now-complete `symbols` table after all
    // instances are elaborated (`resolve_deferred_hier`). Out-of-band (golden-free).
    deferred_hier: Vec<DeferredHier>,
    // Family D (r17): deferred hierarchical FUNCTION calls (`u1.f(x)`), resolved to a
    // per-instance FuncId after all instances exist (mirrors `deferred_hier`).
    deferred_hier_calls: Vec<DeferredHierCall>,
    // Family D (r18): deferred hierarchical TASK enables (`u1.tk(x);`), resolved to a
    // per-instance frame-TASK FuncId + `TaskCallInfo` after all instances exist.
    deferred_hier_task_calls: Vec<DeferredHierTaskCall>,
    // N3.1: hierarchical INDEXED reads `dut.mem[i]` — resolved (with the lowering
    // scope restored) into an array element / bit select after all instances.
    deferred_hier_sel: Vec<DeferredHierSelect>,
    /// Deferred hierarchical WRITE targets (`tb.dut.x = …`); see [`DeferredHierWrite`].
    /// Out-of-band (golden-free) — patched into the statement arena post-elaboration.
    deferred_hier_write: Vec<DeferredHierWrite>,
    /// HIER-REST①: deferred hierarchical ELEMENT/bit-select WRITE targets
    /// (`dut.mem[i] = …`); see [`DeferredHierSelWrite`]. Out-of-band (golden-free).
    deferred_hier_sel_write: Vec<DeferredHierSelWrite>,
    /// N5 functional coverage: covergroup TYPE name → its declaration (registered in
    /// the prescan so an instance can be lowered regardless of source order).
    cover_types: std::collections::BTreeMap<String, ast::CovergroupDecl>,
    /// N5: FQ instance name → its per-coverpoint trackers (hit-bitmap reg + sampled
    /// expr + auto-bin count). `sample()`/`get_coverage()` synthesize against these —
    /// pure IR-0 (the bitmap regs are ordinary nets; no sim-ir change).
    cover_insts: std::collections::BTreeMap<String, Vec<CoverpointTracker>>,
    /// N5 slice C: FQ instance name → its cross trackers (product hit-bitmap + the
    /// constituent coverpoints' match data). Sampled/averaged alongside `cover_insts`.
    cross_insts: std::collections::BTreeMap<String, Vec<CrossTracker>>,
    /// OBS-1b: per covergroup-instance coverage manifest, built at each `new` site
    /// (resolved bitmap net ids in the correct scope). Threaded to `SimOpts` for the
    /// end-of-run `coverage.json` export. Out-of-band, golden-free.
    coverage_manifest: Vec<CovgInstMeta>,
    // §16.4 deferred immediate asserts (out-of-band, engine-facing): marker
    // StmtId → region, and action StmtId → (marker StmtId, region). See
    // [`DeferMarkTable`]/[`DeferActTable`].
    defer_marks: DeferMarkTable,
    defer_acts: DeferActTable,
    // Set while lowering a deferred assert's pass/fail arms: (marker StmtId,
    // region). The `push_stmt` hook records every SysTask emitted under it into
    // `defer_acts`, path-independently (severity OR plain $display).
    cur_defer: Option<(u32, DeferRegion)>,
    // One-shot W-note: a deferred-assert arm contained no deferrable action
    // (only side-effecting statements), so it ran inline (evaluate-when-reached).
    defer_inline_warned: bool,
}

// ────────────────────────────── Tests ──────────────────────────────
#[cfg(test)]
mod tests;
