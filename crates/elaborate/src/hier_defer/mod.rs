//! deferred hier resolution — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// A concurrent assertion (`assert property(@(clk) ante |-> cons)`) collected
/// A hierarchical READ reference (`tb.dut.x`) deferred during expression lowering
/// (N3). The placeholder `Signal` at `eid` is patched to the resolved NetId after
/// all instances are elaborated; `prefix` is the scope path in effect at lowering
/// (so the resolver can walk downward → outward → absolute), `path` the dotted
/// segments.
pub(crate) struct DeferredHier {
    pub(crate) eid: u32,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    /// R17 §4.1: the statement/expression this reference was written in. Resolution
    /// happens long after lowering, in a pass with no ambient `cur_span`, so every
    /// diagnostic these passes raise printed no location at all.
    pub(crate) span: Option<ast::Span>,
}

/// Family D (r17): a hierarchical FUNCTION call `u1.f(args)` whose callee lives in a
/// child instance not yet elaborated when the call site is lowered. Mirrors
/// [`DeferredHier`] (the net-read defer): the placeholder `Expr::Call { func: POISON_FID }`
/// at `eid` is patched to the callee's per-instance FuncId after all instances exist.
/// `prefix` = the caller's scope at lowering, `path` = the dotted segments (`[u1, f]`);
/// the last segment is the function name, the rest resolve to the callee instance scope.
pub(crate) struct DeferredHierCall {
    pub(crate) eid: u32,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    pub(crate) argc: usize,
}

/// Family D (r18): a hierarchical TASK enable `u1.tk(args);` whose callee lives in a
/// child instance not yet elaborated when the call site is lowered. The statement path
/// twin of [`DeferredHierCall`]. At the call site a placeholder `Terminator::Call` seals
/// the process block `(proc, call_block)` and the args are lowered in the CALLER scope;
/// `resolve_deferred_hier_task_call` builds the `TaskCallInfo{callee: per-instance fid}`
/// into `task_calls_proc[(proc, call_block)]` once every instance's frame tasks are in
/// `hier_tasks`. Restricted to SCALAR formals. §4.5.201: each arg is lowered BOTH as a
/// value (`arg_ids[i]`, the copy-IN for an input/inout formal) AND — when it is an lvalue
/// (a bare var / select) — as a caller-side lvalue (`arg_lvals[i]`, the copy-OUT target for
/// an output/inout formal); the callee port DIRECTION is unknown at the call site (the
/// instance isn't elaborated yet), so `resolve_deferred_hier_task_call` picks between them
/// per port using `hier_task_port_dirs[fid]`.
pub(crate) struct DeferredHierTaskCall {
    /// R16 §4-1: the enable's own source span, captured at DEFER time.
    ///
    /// These diagnostics fire in a resolve pass that runs long after the statement was
    /// lowered, so `cur_span` no longer points at anything the user wrote — and the
    /// hierarchical-task-call reject was the ONLY diagnostic in the round-16 report with
    /// no `file:line:col` at all (84 errors, 83 located). In the `TB=partial` run it was
    /// the only diagnostic, so that log had no position anywhere in it.
    pub(crate) span: Option<ast::Span>,
    pub(crate) proc: u32,
    pub(crate) call_block: u32,
    /// §4.5.208: `Some(func_block)` when the enable is NESTED inside a frame TASK body (the
    /// placeholder `Call` lives in `func_blocks`, not a process) — the resolver patches
    /// `func_blocks[func_block].term` and inserts the `TaskCallInfo` into `task_calls_func`
    /// (not `task_calls_proc`). `None` ⇒ a top-level process enable (`proc`/`call_block`).
    /// The block id is process-LOCAL at emit and rebased by `+base` at frame-body finish
    /// (`lower_frame_task_body`), exactly like `pending_task_calls`.
    pub(crate) func_block: Option<u32>,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    pub(crate) arg_ids: Vec<u32>,
    pub(crate) arg_lvals: Vec<Option<ir::Lvalue>>,
    /// §4.5.207: per-arg, `Some(net)` when the actual is a bare whole-array Ident (a static
    /// array net, resolved in the CALLER scope at defer time — a whole array cannot be lowered
    /// to a value). At resolve, if the callee formal is an INPUT array (`frame_arr_formal_meta`)
    /// the net is packed into the md-packed slot (`pack_hier_array_actual`); `None` otherwise.
    /// An array formal fed a non-array actual (or vice versa), or an OUTPUT/INOUT array formal,
    /// stays loud there (correct-or-loud).
    pub(crate) arg_arrays: Vec<Option<u32>>,
}

/// A hierarchical WRITE target (`tb.dut.x = …`) whose net does not exist when the
/// lvalue is lowered. The `LvalChunk` is emitted with the sentinel net
/// `HIER_WRITE_SENTINEL_BASE + index-in-this-Vec`; `resolve_deferred_hier_write`
/// resolves `prefix`/`path` (same IEEE §23.6 walk as the read side, via
/// `hier_lookup`), applies the write-context guards (no event/dyn/array/packed
/// whole-net, no procedural write to a `wire`), and patches every matching chunk
/// in the statement arena to the real NetId (or `POISON_NET` on error). Whole-net
/// only — a hierarchical element/part-select write stays a loud follow-on.
pub(crate) struct DeferredHierWrite {
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    /// R17 §4.1: see [`DeferredHier::span`].
    pub(crate) span: Option<ast::Span>,
}

/// §4.5.355: a fill literal (`'0`/`'1`/`'x`/`'z`) whose ASSIGNMENT WIDTH was not
/// knowable when it was lowered, because the target is a deferred hierarchical write
/// and its chunk still carried a sentinel net.
///
/// The literal is lowered self-determined (one bit) as usual and this record says
/// where to go back and widen it: `expr_id` is the arena slot holding the `Const`
/// (`push_expr` appends, so the slot is this literal's alone and overwriting it
/// cannot disturb another expression), and `sentinel` names which deferral the width
/// is waiting on. `resolve_pending_fill_widths` rebuilds the constant from
/// `(raw, kind, resolved-width)` — no lowering scope required, which is why only a
/// BARE fill is admitted here (see `bare_fill_literal`).
pub(crate) struct PendingFillWidth {
    pub(crate) expr_id: u32,
    pub(crate) sentinel: u32,
    /// The right-hand side AST, kept so the resolve pass can re-lower it at the width
    /// that finally exists. Admitted only by `scope_free_fill_expr`.
    pub(crate) rhs: ast::Expr,
    /// `cur_prefix` at lowering time — the whole of the "scope" a plain name needs,
    /// because resolution is an outward walk from this string over tables that outlive
    /// elaboration. Restored around the re-lowering (§4.5.360).
    pub(crate) prefix: String,
}

pub(crate) struct DeferredHierSelWrite {
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    /// R17 §4.1: see [`DeferredHier::span`].
    pub(crate) span: Option<ast::Span>,
    pub(crate) idx_eids: Vec<u32>,
    /// `Some` for a hierarchical PART-select write (`dut.v[3:0]=…`, `dut.v[o+:w]=…`,
    /// or array-element `dut.mem[i][3:0]=…`): the (already-lowered) raw offset edge,
    /// the width edge, and the select kind. `None` for an element / bit-select write.
    pub(crate) part: Option<HierPart>,
}

/// A hierarchical INDEXED read `base[i]…[k]` whose `base` is a 2-segment hierarchical
/// reference (slice N3.1 + multi-dim follow-on). Deferred like [`DeferredHier`] — the
/// base net does not exist at lowering time — but resolution must choose the SELECT
/// KIND and arity from the resolved net's shape (a single-/multi-dim unpacked array
/// element word, a multi-dim packed bit-slice, or a vector bit-select), which is only
/// known after elaboration. EVERY index is LOWERED AT LOWERING TIME (with the full
/// param/genvar/function-formal context) into `idx_eids`; the fixup only builds the
/// flat-word/offset arithmetic and select around those eids (review N3.1: re-lowering
/// an index at fixup lost that context — a function-formal index silently resolved to
/// a shadowing outer net).
pub(crate) struct DeferredHierSelect {
    pub(crate) eid: u32,
    pub(crate) prefix: String,
    pub(crate) path: Vec<String>,
    /// Indices in SOURCE order (`grid[i][j]` → `[i, j]`). Length 1 = the N3.1 scalar/
    /// vector/single-dim-array case; ≥2 = a multi-dim unpacked/packed element select.
    pub(crate) idx_eids: Vec<u32>,
    /// A trailing PART-select (`dut.mem[i][m:l]`, scalar `dut.v[m:l]`, `[b+:w]`) whose
    /// offset is normalized against the (element/net) LSB at resolution time — the
    /// READ twin of [`DeferredHierSelWrite`]'s `part`. `None` = a bit-select / whole
    /// element read (the pre-existing lanes), so those stay byte-identical.
    pub(crate) part: Option<HierPart>,
}

mod read;
mod task_call;
mod write;
