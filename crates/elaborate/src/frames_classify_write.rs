//! §3.b `frame-body-outside-write`: where a frame body's blocking-assign lvalue lands,
//! and which frame FUNCTIONS that answer routes to the `&mut` statement executor.
//!
//! Split out of `frames_classify.rs` (over the 1000-line cap). The route is decided in two
//! halves that must both say yes: [`Elaborator::func_body_writes_outside_name`] on the AST
//! (asked before any body is lowered, because the set is read while OTHER bodies lower) and
//! [`Elaborator::frame_outside_writes_are_user_nets`] on the lowered blocks, which vetoes a
//! net vita minted for itself. [`Elaborator::frame_chunk_site`] is the per-chunk rule both
//! `classify_frame_body` and the veto read, so the gate and the veto cannot disagree.

use super::*;

/// Where one blocking-assign lvalue CHUNK of a frame body lands, relative to that
/// body's own frame window `[lo, hi)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameChunkSite {
    /// The synchronous `&self` frame executor can perform it: a whole in-frame slot
    /// write, a bit/part-select of an in-frame scalar net (the engine
    /// read-modify-writes the slot), an in-frame dyn-array element (a heap op), or a
    /// class-field HEAP write through a handle (interior-mutable, wherever the handle
    /// lives).
    InFrame,
    /// A net OUTSIDE the window — a module / instance net. Only the `&mut` statement
    /// executor can perform it, which is what `compute_suspendable_tasks` routes a
    /// carrier of one to. `whole` is the shape, because the two spellings have always
    /// had different diagnostics and the distinction survives here.
    Outside { whole: bool },
    /// In-frame but a shape the frame write path cannot route: an ARRAY-element write
    /// (`word`), or a select of an unpacked-array local (reserved as a 1-element net,
    /// where `mem[k]` mis-lowers to a bit-select).
    InFrameLoud,
}

impl Elaborator<'_> {
    /// The per-chunk rule shared by the frame-body GATE (`classify_frame_body`) and the
    /// §3.b route VETO ([`Self::frame_outside_writes_are_user_nets`]).
    ///
    /// One home, because the two consumers must agree chunk for chunk: a chunk the gate
    /// admits must not make the route fire (it would change a working call's shape), and
    /// a chunk the route claims must be one the gate then accepts (else the function is
    /// rerouted and still refused).
    pub(crate) fn frame_chunk_site(&self, c: &ir::LvalChunk, lo: u32, hi: u32) -> FrameChunkSite {
        // N7: a class field write (`this.f = v` / `obj.f = v`) is a HEAP write through a
        // class handle, not a frame-slot write — allowed regardless of word/in-frame (the
        // handle that carries it is itself a frame-local or module net). It must be
        // answered BEFORE the window test: the handle can be a module net, and reporting
        // `Outside` for it would route a body the `&self` executor already runs.
        if self.class_handle_nets.contains(&c.net) && c.word.is_some() {
            return FrameChunkSite::InFrame;
        }
        let whole = c.offset.is_none() && c.word.is_none() && c.width.is_none();
        let in_frame = c.net >= lo && c.net < hi;
        // V5: an in-frame DYN-ARRAY element write (`loc[i] = v`) routes to the HEAP
        // (`dyn_write`), not a frame-slot write — allowed (the frame dyn-array net holds
        // the current activation's array).
        if in_frame && self.is_dyn_handle_net(c.net) {
            return FrameChunkSite::InFrame;
        }
        if !in_frame {
            return FrameChunkSite::Outside { whole };
        }
        // EXT2-H: a bit/part-select write to an IN-FRAME scalar net (`r[7:0] = x`,
        // `r[i] = b`, an md-packed `p[0] = ..`) is supported — the engine
        // read-modify-writes the frame slot. An ARRAY-element write (`c.word`), or a
        // select of an UNPACKED-array local (a 1-elem net — `frame_array_local` — where
        // `mem[k]` mis-lowers to a bit-select), is not.
        //
        // MEASURED, not assumed: exempting the `$sformatf` hoist's `$sfmt_tmp$` scratch
        // nets from the window test (they are written and read back within one statement
        // sequence, so they LOOK frame-local) makes the engine panic `frame lvalue net is
        // routed` — the SYNCHRONOUS frame executor genuinely cannot write a module net.
        // That is why the exemption is not here; the §3.b answer is a ROUTE to the other
        // executor, not a claim that this one can do it.
        if whole || !(c.word.is_some() || self.frame_array_local.contains(&c.net)) {
            FrameChunkSite::InFrame
        } else {
            FrameChunkSite::InFrameLoud
        }
    }

    /// §3.b, AST half: does this FUNCTION's body assign a name the routine does not own —
    /// a module / instance net?
    ///
    /// `true` makes the function a STATEMENT-EXECUTOR function: `lower_frame_funcs` joins
    /// it to `inout_func_names`, so its calls are emitted as a `Terminator::Call` (the only
    /// call shape `run_process` can route), and `classify_frame_body` is told to accept the
    /// write instead of refusing it. The RUNTIME route is not decided here —
    /// `ir::compute_suspendable_tasks` classifies functions too and already reads the same
    /// out-of-window lhs chunk, so elaborate and the engine reach the set independently.
    ///
    /// ⚠️ ASKED ON THE AST, and it has to be. The set is read while OTHER functions' bodies
    /// are lowered — a caller is lowered before its callee whenever its name sorts first —
    /// so an answer derived from the callee's own lowered blocks is not available yet.
    /// `function automatic int f2(input int v); return fw(v) + 1;` beside `fw` was lowered
    /// first, saw an empty set, emitted a plain `Expr::Call`, and the engine reached
    /// `frame write targets a frame-local net` (a panic, rc=101). The IR half is still
    /// asked, as a VETO, by [`Self::frame_outside_writes_are_user_nets`].
    ///
    /// The own-name set is `pkg_body_scope::rtn_declared_names` — formals, top-level and
    /// block-local declarations, body-local enum labels and the function's own name — the
    /// one construction every other consumer of that question uses.
    ///
    /// Two declines, each keeping a refusal that is right today:
    /// * a body that ENABLES A TASK. Both oracles reject a task call in a function outright
    ///   (iverilog "Functions cannot enable/call tasks", verilator `FUNCTIMECTL`), and an
    ///   inlined task body's formal binding is itself an out-of-window write, so without
    ///   this the construct both oracles refuse would start running.
    /// * a MULTI-SEGMENT lvalue (`u.x = v`, `pkg::s = v`). The hierarchical write has its
    ///   own routing question and its own queue row; nothing here measured it.
    pub(crate) fn func_body_writes_outside_name(&self, func: &ast::FunctionDef) -> bool {
        if stmt_enables_task(&func.body) {
            return false;
        }
        let own = pkg_body_scope::rtn_declared_names(
            &func.ports,
            &func.body_decls,
            &func.body_enums,
            &func.body,
            Some(&func.name.name),
        );
        stmt_writes_outside_name(&func.body, &own)
    }

    /// §3.b, IR half: is every out-of-window write in this lowered body one the SOURCE
    /// wrote, rather than a net vita minted for itself?
    ///
    /// The veto on [`Self::func_body_writes_outside_name`]. A COMPILER-GENERATED net
    /// (`$ia_tmp$`, `$sfmt_tmp$`, a capture net handed to the wrong frame) must keep
    /// `classify_frame_body`'s own wording — that case is a vita bug, and routing it would
    /// make the bug run instead of report.
    pub(crate) fn frame_outside_writes_are_user_nets(
        &self,
        entry: u32,
        net_base: u32,
        locals_len: u32,
    ) -> bool {
        let (lo, hi) = (net_base, net_base + locals_len);
        // `net_is_compiler_internal` scans `symbols`, so ask it once per DISTINCT net.
        let mut checked: BTreeMap<u32, bool> = BTreeMap::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![entry];
        while let Some(bi) = stack.pop() {
            if !seen.insert(bi) {
                continue;
            }
            let Some(blk) = self.func_blocks.get(bi as usize) else {
                continue;
            };
            for &sid in &blk.stmts {
                let ir::Stmt::BlockingAssign { lhs, .. } = &self.stmts[sid as usize] else {
                    continue;
                };
                for c in &lhs.chunks {
                    if !matches!(
                        self.frame_chunk_site(c, lo, hi),
                        FrameChunkSite::Outside { .. }
                    ) {
                        continue;
                    }
                    let internal = *checked
                        .entry(c.net)
                        .or_insert_with(|| self.net_is_compiler_internal(c.net));
                    if internal {
                        return false;
                    }
                }
            }
            match &blk.term {
                ir::Terminator::Goto { target } => stack.push(*target),
                ir::Terminator::Branch {
                    then_bb, else_bb, ..
                } => {
                    stack.push(*then_bb);
                    stack.push(*else_bb);
                }
                ir::Terminator::Call { ret_bb, .. } => stack.push(*ret_bb),
                ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                    stack.push(*resume)
                }
                ir::Terminator::Fork { .. } | ir::Terminator::Return => {}
            }
        }
        true
    }

    /// §3.b: refuse a call to a body-writing frame FUNCTION in a position the hoist could
    /// not turn into a statement.
    ///
    /// The write is carried by the call's `Terminator::Call`, which only a STATEMENT can
    /// hold. Where the hoist declines — a continuously re-evaluated expression, a `force`
    /// rhs, a package-scoped call, another frame function's body — the call falls through
    /// to `emit_frame_call`, which would otherwise emit a plain `Expr::Call`.
    ///
    /// MEASURED, by building this refusal out: every one of those positions then reaches
    /// `sim-engine state/frame_eval.rs` and panics `frame write targets a frame-local net`
    /// (rc 101, no diagnostic, no source location) — an `assign`, a `force`, a call in
    /// another function body and a `pkg::f()` call all four. So this is not a
    /// conservatism: it is the difference between a named E3009 and a crash.
    /// `rtn_key` is the frame table key — `pkg::f` for a package-SCOPED call, the bare name
    /// otherwise. It is the one position this function can IDENTIFY rather than list, so
    /// when it names a package the message leads with that case and quotes the user's own
    /// package, never a stand-in `pkg`.
    pub(crate) fn refuse_body_write_call(&mut self, fname: &str, rtn_key: &str) -> u32 {
        let (scoped, also_scoped) = if rtn_key.contains("::") {
            (
                format!(
                    "This call is package-SCOPED (`{rtn_key}(...)`), and a scoped call \
                     builds its frame at the call site, after the statement that would have \
                     carried the write: `import {rtn_key};` and call `{fname}` by its bare \
                     name, which does perform it. "
                ),
                "",
            )
        } else {
            (
                String::new(),
                "a package-SCOPED call, whose frame is built at the call site — `import` it \
                 and use the bare name instead; ",
            )
        };
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "function `{fname}` assigns a module net from its body, so the write has to \
                 be emitted as a statement before the expression that calls it. {scoped}That \
                 works in any position evaluated ONCE per statement, but not here. The \
                 remaining cases are: a CONTINUOUSLY re-evaluated expression (`assign`, \
                 `force`, a `wait` condition) — the write cannot re-fire on every change; an \
                 intra-assignment delay (`x = #1 {fname}(...)`); {also_scoped}and any \
                 position inside another FUNCTION body lowered as a call frame (a \
                 function is entered from the expression that calls it, so it has no call \
                 statement of its own to carry the write) — the same call in a `task` body, \
                 or in a module process, does work. In a module process or a `task` body, \
                 assign the call to a temporary first (`t = {fname}(...);`) and use `t`."
            ),
        );
        self.placeholder_expr()
    }
}

/// Does `s` (recursively) BLOCKING-ASSIGN a name outside `own`?
///
/// The lvalue must be a single-segment `Ident` — a hierarchical or package-scoped write is
/// a different routing question and answers `false` here, so it keeps its pre-slice
/// refusal. A concatenation or select target is walked through `lvalue_root_outside`,
/// which reports the root name it can see and `false` for a shape with no single root; a
/// concat target is refused by `classify_frame_body` on its own count in any case.
///
/// Only a BLOCKING assign counts: an NBA in a function body is refused by both this
/// elaborator and iverilog, and a `force`/`release`/system task is refused by
/// `classify_frame_body`, so routing may not depend on them.
pub(crate) fn stmt_writes_outside_name(s: &ast::Stmt, own: &BTreeSet<String>) -> bool {
    use ast::Stmt::*;
    match s {
        Blocking { lhs, .. } => lvalue_root_outside(lhs, own),
        Block { stmts, .. } | Fork { stmts, .. } => {
            stmts.iter().any(|st| stmt_writes_outside_name(st, own))
        }
        If { then_s, else_s, .. } => {
            stmt_writes_outside_name(then_s, own)
                || else_s
                    .as_deref()
                    .is_some_and(|e| stmt_writes_outside_name(e, own))
        }
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_writes_outside_name(body, own)
            }
        }),
        For { body, init, .. } => {
            stmt_writes_outside_name(init, own) || stmt_writes_outside_name(body, own)
        }
        While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            stmt_writes_outside_name(body, own)
        }
        DelayCtrl { body, .. } | EventCtrl { body, .. } | Wait { body, .. } => body
            .as_deref()
            .is_some_and(|b| stmt_writes_outside_name(b, own)),
        NonBlocking { .. }
        | Return { .. }
        | SysTaskCall { .. }
        | UserTaskCall { .. }
        | RandomizeWith { .. }
        | EventTrigger { .. }
        | Disable { .. }
        | WaitFork { .. }
        | ConcurrentAssert { .. }
        | Assign { .. }
        | Deassign { .. }
        | Force { .. }
        | Release { .. }
        | DeferredAssert { .. }
        | CoverProperty { .. }
        | Null(_)
        | Error(_) => false,
    }
}

/// Is this lvalue's single root name absent from `own`? A shape with no single root name
/// (a concatenation, a streaming target) answers `false`.
fn lvalue_root_outside(lv: &ast::Lvalue, own: &BTreeSet<String>) -> bool {
    match lv {
        ast::Lvalue::Ident(p) => matches!(p.segments.as_slice(), [seg] if !own.contains(&seg.name)),
        ast::Lvalue::BitSelect { base, .. }
        | ast::Lvalue::PartSelect { base, .. }
        | ast::Lvalue::IndexedPart { base, .. } => lvalue_root_outside(base, own),
        _ => false,
    }
}

/// Does `s` (recursively) ENABLE A TASK?
///
/// `_`-free: this is the precondition of a ROUTE, and a statement kind nobody classified
/// must not default to the quiet side. Every arm that can hold a statement descends; a
/// statement that cannot hold one answers `false` by its own name.
pub(crate) fn stmt_enables_task(s: &ast::Stmt) -> bool {
    use ast::Stmt::*;
    match s {
        UserTaskCall { .. } => true,
        Block { stmts, .. } | Fork { stmts, .. } => stmts.iter().any(stmt_enables_task),
        If { then_s, else_s, .. } => {
            stmt_enables_task(then_s) || else_s.as_deref().is_some_and(stmt_enables_task)
        }
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_enables_task(body)
            }
        }),
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            stmt_enables_task(body)
        }
        DelayCtrl { body, .. } | EventCtrl { body, .. } | Wait { body, .. } => {
            body.as_deref().is_some_and(stmt_enables_task)
        }
        Blocking { .. }
        | NonBlocking { .. }
        | Return { .. }
        | SysTaskCall { .. }
        | RandomizeWith { .. }
        | EventTrigger { .. }
        | Disable { .. }
        | WaitFork { .. }
        | ConcurrentAssert { .. }
        | Assign { .. }
        | Deassign { .. }
        | Force { .. }
        | Release { .. }
        | DeferredAssert { .. }
        | CoverProperty { .. }
        | Null(_)
        | Error(_) => false,
    }
}
