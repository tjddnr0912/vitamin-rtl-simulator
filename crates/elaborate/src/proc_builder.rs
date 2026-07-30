//! the per-process CFG BUILDER and the small pure helpers around it — split out of
//! `stmt_flow.rs` (R20) to hold both files under the 1000-line module cap.
//!
//! Nothing here is a serialized type, so the move is free of the `SchemaHash`
//! `module_path!()` constraint that pins `hdl-ast` / `sim-ir` types to their crate root.

use super::*;

/// Index into a `ProcessBuilder::body` (the process-local CFG), NOT the global
/// `SimIr.blocks` arena.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct BlockId(pub(crate) u32);
impl BlockId {
    pub(crate) fn raw(self) -> u32 {
        self.0
    }
}

/// Builds the CFG (`Vec<BasicBlock>`) for ONE process. Owns the process-local
/// block list + the single "unsealed block" cursor.
///
/// INV-1 (sealing): exactly one block — the one `cur` points at — is unsealed
/// at any time. `end_block_with` is the only writer of a real terminator and it
/// CLOSES the cursor (`cur = None`); the caller must `start_block` before the
/// next emit. A freshly-allocated block is pre-filled with `Return`, so even a
/// builder bug degrades to a stray early return, NEVER a dangling index.
///
/// INV-2 (no dangling): a block is allocated (`new_block`) before its id is
/// named in any terminator; `finish` seals the trailing open block with
/// `Return`. Every control-flow form below ends by `start_block`-ing its single
/// "continue point", so on return from `lower_stmt` the cursor is always open
/// and is where control flows next — the caller is structurally unable to leave
/// an arm dangling.
pub(crate) struct ProcessBuilder {
    pub(crate) body: Vec<ir::BasicBlock>,
    pub(crate) cur: Option<BlockId>,
}

impl ProcessBuilder {
    /// Start with one empty block (the entry, id 0) as the open cursor.
    pub(crate) fn new() -> Self {
        let mut pb = ProcessBuilder {
            body: Vec::new(),
            cur: None,
        };
        let entry = pb.new_block();
        pb.cur = Some(entry);
        pb
    }

    /// Allocate a fresh block, provisionally terminated `Return` (overwritten by
    /// `end_block_with` when sealed). Returns its process-local id.
    pub(crate) fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.body.len() as u32);
        self.body.push(ir::BasicBlock {
            stmts: Vec::new(),
            term: ir::Terminator::Return,
        });
        id
    }

    /// Make `b` the open cursor (the caller asserts no other block is open).
    pub(crate) fn start_block(&mut self, b: BlockId) {
        debug_assert!(self.cur.is_none(), "start_block over an open cursor");
        self.cur = Some(b);
    }

    /// B2 frame-call: the process-local id of the currently open block (the block
    /// a `Terminator::Call` will seal — its sidecar key).
    pub(crate) fn cur_id(&self) -> u32 {
        self.cur.expect("cur_id with no open block (INV-1)").raw()
    }

    /// Record an already-built `StmtId` (from the global arena) in the current
    /// block. Stays in the same block (no split).
    pub(crate) fn push_stmt_id(&mut self, sid: u32) {
        let b = self.cur.expect("push_stmt_id with no open block (INV-1)");
        self.body[b.0 as usize].stmts.push(sid);
    }

    /// Seal the current block with `term` and CLOSE the cursor.
    pub(crate) fn end_block_with(&mut self, term: ir::Terminator) {
        let b = self
            .cur
            .take()
            .expect("end_block_with with no open block (double seal?)");
        self.body[b.0 as usize].term = term;
    }

    /// Seal current with `Goto(target)`; cursor closed.
    pub(crate) fn goto(&mut self, target: BlockId) {
        self.end_block_with(ir::Terminator::Goto {
            target: target.raw(),
        });
    }

    /// Final hand-off: seal the trailing open block with `Return`. entry = 0.
    pub(crate) fn finish(mut self) -> (Vec<ir::BasicBlock>, u32) {
        if self.cur.is_some() {
            self.end_block_with(ir::Terminator::Return);
        }
        (self.body, 0)
    }
}

/// A fresh time-0 `SuspendState`. `resume_pc = entry`; everything else default.
/// `wake_key` is a never-armed placeholder the engine overwrites on first
/// suspend — `WakeCond` (the suspend-state type) is DISTINCT from `WaitCause`
/// (the terminator type); a `Level{nets:[]}` (vacuously false) is the minimal
/// valid seed since `WakeCond` has no none-variant.
pub(crate) fn fresh_suspend(entry: u32) -> ir::SuspendState {
    ir::SuspendState {
        resume_pc: entry,
        locals: Vec::new(),
        join_state: ir::JoinState {
            parent: None,
            children: Vec::new(),
            detached: Vec::new(),
            flags: ir::ProcFlags(0),
        },
        wake_key: ir::WakeKey {
            cond: ir::WakeCond::Level { nets: Vec::new() },
            region: ir::RegionTag::Active,
            tie_break: 0,
        },
        call_stack: Vec::new(),
        frame_arena: Vec::new(),
    }
}

/// B3 frame-call: does this body contain a `disable <name>` targeting the frame
/// function/task ITSELF (a self-disable = early return)? Used to lazily add the
/// convergence exit block ONLY when needed (otherwise the body is byte-identical).
pub(crate) fn stmt_disables_name(s: &ast::Stmt, name: &str) -> bool {
    use ast::Stmt::*;
    match s {
        Disable { target, .. } => target.segments.len() == 1 && target.segments[0].name == name,
        Block { stmts, .. } | Fork { stmts, .. } => {
            stmts.iter().any(|st| stmt_disables_name(st, name))
        }
        If { then_s, else_s, .. } => {
            stmt_disables_name(then_s, name)
                || else_s.as_ref().is_some_and(|e| stmt_disables_name(e, name))
        }
        Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_disables_name(body, name)
            }
        }),
        For { body, .. } | While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            stmt_disables_name(body, name)
        }
        _ => false,
    }
}

/// B1 frame-call: can `start` reach `target` over the call-graph `edges`
/// (`start == target` ⇒ "is `start` recursive?", direct OR mutual)? Iterative
/// DFS from `start`'s callees.
pub(crate) fn reaches(
    start: &str,
    target: &str,
    edges: &BTreeMap<String, std::collections::BTreeSet<String>>,
) -> bool {
    let mut stack: Vec<&str> = edges
        .get(start)
        .into_iter()
        .flatten()
        .map(|s| s.as_str())
        .collect();
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    while let Some(n) = stack.pop() {
        if n == target {
            return true;
        }
        if !seen.insert(n) {
            continue;
        }
        if let Some(cs) = edges.get(n) {
            stack.extend(cs.iter().map(|s| s.as_str()));
        }
    }
    false
}

/// B1 frame-call: rebase a process-local terminator's block target(s) by `+base`
/// when the lowered func CFG is appended to the GLOBAL `ir.blocks` arena. A valid
/// frame body only carries `Goto`/`Branch`/`Return`; the suspend/fork variants are
/// rebased defensively (the body validator rejects them) so no index dangles.
pub(crate) fn rebase_terminator(t: &mut ir::Terminator, base: u32) {
    match t {
        ir::Terminator::Goto { target } => *target += base,
        ir::Terminator::Branch {
            then_bb, else_bb, ..
        } => {
            *then_bb += base;
            *else_bb += base;
        }
        ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
            *resume += base
        }
        ir::Terminator::Fork {
            children,
            join,
            resume_bb,
        } => {
            // Stage-1 fork-in-frame: the CHILD arm-entry ids must rebase too (the `..`
            // that skipped `children` was a latent bug — dormant while every in-frame
            // fork was loud-rejected; a fork in a task body reaches here on the flush).
            for c in children.iter_mut() {
                *c += base;
            }
            *join += base;
            *resume_bb += base;
        }
        ir::Terminator::Call { ret_bb, .. } => *ret_bb += base,
        ir::Terminator::Return => {}
    }
}
