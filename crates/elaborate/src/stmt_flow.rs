//! statement control flow — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

// ════════════════════════════════════════════════════════════════════
//  v2 — procedural-block lowering (ProceduralBlock → ir::Process)
// ════════════════════════════════════════════════════════════════════
//
// BLOCK-INDEX SPACE (the load-bearing decision):
//   `ir::Process.body: Vec<BasicBlock>` is INLINE per-process. Every Terminator
//   target (`Goto.target`, `Branch.then_bb`/`else_bb`, `Delay.resume`,
//   `Wait.resume`, …) and `Process.entry` is an index INTO THAT process's own
//   `body` Vec — process-LOCAL, 0-based, reset per process. `SimIr.blocks`
//   (the top-level arena) is NOT referenced by `Process`; v2 leaves it empty
//   (it is reserved for func/task bodies, deferred). `BlockId` below is a
//   newtype over that process-local index so it can never be confused with a
//   StmtId/ExprId/NetId (all bare u32 elsewhere).
//
//   `BasicBlock.stmts: Vec<u32>` hold indices into the GLOBAL `self.stmts`
//   arena (shared across processes), appended via `push_stmt`.

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

impl Elaborator<'_> {
    /// Append a process AND its time multiplier in lockstep (invariant:
    /// `proc_multipliers.len() == processes.len()`). The engine reads the table from
    /// `SimOpts.proc_multipliers` to scale `$time`/`$realtime` per calling module.
    pub(crate) fn push_process(&mut self, p: ir::Process) {
        // Full-width u64: a `10us/1ps`-class ratio (diff ≥ 10) used to SATURATE at
        // u32::MAX = a non-power-of-10 M → wrong `$time` scaling and a wrong `%t`
        // decade (soundness review Q6). The staged trailer stays wire-compatible
        // (postcard varint encodes the same bytes for values < 2^32).
        self.proc_multipliers.push(self.cur_time_mult);
        self.proc_prec_mults.push(self.cur_prec_mult);
        // `%m` scope: the instance path of the module being lowered ("tb.u1");
        // an empty prefix (single top) renders as the top module's own name —
        // but cur_prefix is ALWAYS the instance path incl. the top ("m" / "m.u1").
        self.proc_scopes.push(self.cur_prefix.clone());
        self.processes.push(p);
    }

    /// THE deterministic stmt append point (mirror of [`Self::push_expr`]).
    #[inline]
    pub(crate) fn push_stmt(&mut self, s: ir::Stmt) -> u32 {
        // §16.4: while lowering a deferred assert's pass/fail arm, every SysTask
        // emitted (the $error/$display/$fatal/… action — any lowering path) is
        // recorded as a deferred ACTION so the engine enqueues it under the
        // assertion's marker instead of firing it inline. Path-independent: both
        // `lower_severity_task` and the plain `$display` path funnel through here.
        if let Some((marker, region)) = self.cur_defer {
            if matches!(s, ir::Stmt::SysTask { .. }) {
                let id = self.stmts.len() as u32;
                self.defer_acts.insert(id, (marker, region));
            }
        }
        let id = self.stmts.len() as u32;
        self.stmts.push(s);
        id
    }

    /// Synthesize a private capture temp for an intra-assignment delay site
    /// (`$ia_tmp$<n>` — `$` keeps it collision-proof against user identifiers).
    pub(crate) fn fresh_ia_tmp(&mut self, width: u32) -> u32 {
        let name = format!("$ia_tmp${}", self.nets.len());
        let nv = ir::NetVar {
            kind: ir::NetKind::Reg,
            width,
            msb: width.saturating_sub(1),
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: default_init(ast::NetVarKind::Reg, width),
        };
        self.add_net(&name, nv);
        (self.nets.len() - 1) as u32
    }

    /// A2: synthesize ONE step of a fixed-size-array `foreach` walk. The parser
    /// desugar emits `__st = arr.first/next(__foreach_i)` uniformly; a static array
    /// has no `.first/.next`, so walk a plain index over the FIRST dim `[lo, lo+size)`:
    ///   first → `__i = lo; __st = 1;`   (a fixed array always has ≥1 element)
    ///   next  → `__i = __i + 1; __st = (__i <= hi) ? 1 : -1;`
    /// The caller's `while (__st == 1)` then visits every element exactly once. The
    /// index variable carries the DECLARED index value (`a[2:5]` walks 2..5), so the
    /// body's `a[i]` selects the right element. Returns true (statement consumed).
    pub(crate) fn lower_fixed_foreach_step(
        &mut self,
        b: &mut ProcessBuilder,
        st_lhs: &ast::Lvalue,
        arr_net: u32,
        method: &str,
        key_arg: &ast::Expr,
        dim: u32,
    ) -> bool {
        let key_net = match &key_arg.kind {
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.lookup_net_scoped(&p.segments[0].name)
            }
            _ => None,
        };
        let Some(knet) = key_net else {
            return false; // not a plain index — let the normal path diagnose
        };
        // `dim` is the 1-indexed unpacked dimension this index walks (a multi-dim
        // `foreach (a[i,j])` tags each index with its dimension). A reference past
        // the array's actual unpacked dimensions is loud (e.g. `a[i,j]` on a 1-D
        // array), matching iverilog's reject.
        //
        // IEEE 1800 §12.7.3: `foreach` traverses the declared bounds left-to-right.
        // A descending range (`a[hi:lo]`) walks hi DOWN to lo; ascending walks lo up
        // to hi. All walk arithmetic is SIGNED (the index is `integer`) so a descending
        // walk's transient `lo-1` (e.g. -1 when lo=0) terminates instead of wrapping.
        //
        // T1: a ROUTED fixed string array carries its geometry in `fixed_string_dyn`
        // instead of `array_dims`/`array_dim_desc` — it is a `DynArray` handle, so the
        // flat-word-store descriptors do not describe it. Asked FIRST, and the two
        // sources are disjoint by construction (a net is a routed string array or a
        // static array, never both), so this cannot change what any existing array walks.
        let (lo, size, desc) = match self.routed_foreach_dim(arr_net, dim) {
            Some(g) => g,
            None => {
                let extents = self.net_dim_extents(arr_net);
                match extents.get((dim - 1) as usize) {
                    Some(&(lo, size)) => (
                        lo,
                        size,
                        self.array_dim_desc
                            .get(&arr_net)
                            .and_then(|v| v.get((dim - 1) as usize).copied())
                            .unwrap_or(false),
                    ),
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a foreach index dimension exceeds the array's unpacked dimensions",
                        );
                        return true;
                    }
                }
            }
        };
        // `lo` is the DECLARED minimum and may be negative (`int a[-1:1]`); the walk
        // already runs on signed 32-bit constants (`const_s32_expr`), so the bounds only
        // have to arrive intact. Saturating conversion keeps a pathological declaration
        // from wrapping into a bogus range.
        let hi = lo.saturating_add(i64::from(size.saturating_sub(1)));
        let clamp_i32 = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let (lo_i, hi_i) = (clamp_i32(lo), clamp_i32(hi));
        let st_lv = self.lower_lvalue(st_lhs);
        self.check_lvalue_kind(&st_lv, true);
        if method == "first" {
            let start = self.const_s32_expr(if desc { hi_i } else { lo_i });
            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: whole_net_lvalue(knet),
                rhs: start,
            });
            b.push_stmt_id(sid);
            let one = self.const_s32_expr(1);
            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: st_lv,
                rhs: one,
            });
            b.push_stmt_id(sid);
        } else {
            // next: __i = __i ± 1 (sub when descending, add when ascending)
            let cur = self.push_expr(ir::Expr::Signal {
                net: knet,
                word: None,
            });
            let one = self.const_s32_expr(1);
            let step = self.push_expr(ir::Expr::Binary {
                op: if desc { ir::BinOp::Sub } else { ir::BinOp::Add },
                lhs: cur,
                rhs: one,
            });
            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: whole_net_lvalue(knet),
                rhs: step,
            });
            b.push_stmt_id(sid);
            // __st = (descending ? __i >= lo : __i <= hi) ? 1 : -1
            let cur2 = self.push_expr(ir::Expr::Signal {
                net: knet,
                word: None,
            });
            let bound = self.const_s32_expr(if desc { lo_i } else { hi_i });
            let cmp = self.push_expr(ir::Expr::Binary {
                op: if desc { ir::BinOp::Ge } else { ir::BinOp::Le },
                lhs: cur2,
                rhs: bound,
            });
            let one = self.const_s32_expr(1);
            let neg1 = self.const_s32_expr(-1);
            let tern = self.push_expr(ir::Expr::Ternary {
                cond: cmp,
                then_e: one,
                else_e: neg1,
            });
            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: st_lv,
                rhs: tern,
            });
            b.push_stmt_id(sid);
        }
        true
    }

    pub(crate) fn lower_proc_block(&mut self, p: &ast::ProceduralBlock) -> ir::Process {
        // G7: desugar a single-term `iff` event guard into a body `if` (the IR
        // sensitivity carries no guard). Shadow `p` with the rewritten block; the rest
        // of the function is unchanged.
        let desugared = self.desugar_event_iff(p);
        let p = desugared.as_ref().unwrap_or(p);
        // The ProcId this process WILL occupy when the caller pushes it. Stable for
        // the whole body lowering (lower_proc_block is non-reentrant: it fully
        // builds one Process and returns BEFORE processes.push). Any fork mode
        // recorded below is keyed by this id; the caller debug_asserts the match.
        self.cur_proc = self.processes.len() as u32;
        // Reset the nesting guard at every top-level body entry (a process body is
        // never lowered while already inside a fork child of another process).
        self.in_fork = false;

        // P2-E `final`: zero-time one-shot — ANY timing control in the body
        // (#/@/wait, anywhere, fork branches included) is illegal (§9.2.3).
        if matches!(p.kind, ast::ProcKind::Final) {
            if stmt_has_timing(&p.body) {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a final block executes in zero time — timing controls \
                     (#/@/wait) are illegal inside it (IEEE §9.2.3)",
                );
            }
            self.final_procs.insert(self.cur_proc);
        }

        // M-C: a bare `always` with NO header @(...) re-arms via its own in-body
        // timing (`always #5 clk=~clk;`). Detect that and wrap the body in an
        // implicit forever so control loops back to the in-body delay/event.
        let bare_always_self_timed =
            matches!(p.kind, ast::ProcKind::Always) && p.sensitivity.is_none();

        let sensitivity = self.lower_sensitivity(p.kind, p.sensitivity.as_ref(), &p.body);
        let mut b = ProcessBuilder::new(); // entry block #0 open
        if bare_always_self_timed && stmt_has_timing(&p.body) {
            // Implicit `forever { body }` so the process re-arms on its own #/@.
            self.lower_forever(&mut b, &p.body);
        } else {
            self.lower_stmt(&mut b, &p.body); // recursive body lowering
        }
        let (body, entry) = b.finish(); // seals trailing block with Return

        // Implicit-sensitivity inference for `@*` / `always_comb` / `always_latch`:
        // lower_sensitivity leaves these `Comb`/`Latch` with EMPTY edges. Infer the
        // read-set (every net read on a RHS / branch condition in the lowered body)
        // and record it as level-sensitive edges so the engine re-fires the block
        // when any input changes. EXCLUDES a bare self-timed `always` (re-arms via
        // its own in-body #/@, no data read-set).
        let is_comb_inferred = matches!(
            p.kind,
            ast::ProcKind::AlwaysComb | ast::ProcKind::AlwaysLatch
        ) || matches!(
            (p.kind, p.sensitivity.as_ref()),
            (ast::ProcKind::Always, Some(ast::Sensitivity::Star))
        );
        let sensitivity = if is_comb_inferred && sensitivity.edges.is_empty() {
            // Record this ProcId so the post-hier-resolution pass can recompute
            // its read-set (a bare self-timed `always` is NOT is_comb_inferred, so
            // its intentionally-empty sensitivity is never touched — clocks safe).
            self.comb_inferred_procs.push(self.cur_proc);
            let nets = self.comb_read_set(&body);
            ir::Sensitivity {
                kind: sensitivity.kind,
                edges: nets
                    .into_iter()
                    .map(|net| ir::EdgeTerm {
                        net,
                        kind: ir::EdgeKind::AnyEdge,
                    })
                    .collect(),
            }
        } else {
            sensitivity
        };

        ir::Process {
            sensitivity,
            body,
            entry,
            suspend: fresh_suspend(entry),
        }
    }

    // ── case → Branch chain ────────────────────────────────────────
    pub(crate) fn lower_case(
        &mut self,
        b: &mut ProcessBuilder,
        kind: ast::CaseKind,
        scrutinee: &ast::Expr,
        items: &[ast::CaseItem],
    ) {
        // casez/casex wildcard semantics are realized per-label by masking the
        // label's unknown (`?`/`z`/`x`) bits out of the compare (see
        // `case_cmp`). Plain `case` is an exact 4-state `===`.
        let scrut_id0 = self.lower_expr(scrutinee);
        let merge = b.new_block();

        // Pre-allocate each Match arm's entry block; pin the default body.
        // Allocation order (deterministic): merge, then each Match arm block in
        // source order, then per-label miss-blocks during the cascade. Every
        // label is lowered ONCE here (ids reused below) so the case's collective
        // signedness can be computed from them.
        let mut arm_bodies: Vec<(BlockId, &ast::Stmt)> = Vec::new();
        let mut default_body: Option<&ast::Stmt> = None;
        let mut tests: Vec<(Vec<u32>, BlockId)> = Vec::new();
        for it in items {
            match it {
                ast::CaseItem::Match { labels, body, .. } => {
                    let arm = b.new_block();
                    let ids: Vec<u32> = labels
                        .iter()
                        .map(|l| self.lower_case_label(scrut_id0, l))
                        .collect();
                    tests.push((ids, arm));
                    arm_bodies.push((arm, body));
                }
                ast::CaseItem::Default { body, .. } => default_body = Some(body),
            }
        }

        // §12.5 / §11.8.1 COLLECTIVE signedness: the case comparison is signed
        // ONLY when the scrutinee AND every case-item label are signed; if ANY
        // participant is unsigned the WHOLE comparison is unsigned. vita's engine
        // sizes each `CaseEq(scrut,label)` pair independently
        // (`signed(l)&&signed(r)`), so a signed scrutinee would sign-extend
        // against a signed label even when an unsigned sibling label makes the
        // SET collectively unsigned — the wrong branch, silently (e.g.
        // `case(s) -1: ; 4'hF: ;` with signed `s=4'hF`: vita matched `-1`,
        // iverilog matches `4'hF`). Force the scrutinee UNSIGNED once when the
        // set is collectively unsigned: then every pair-signedness is false, so
        // the scrutinee AND each label zero-extend — exactly the collective rule
        // (iverilog-pinned). No-op when the scrutinee is already unsigned (its
        // pairs are already unsigned → byte-identical) or when all labels are
        // signed (correctly stays signed). `$unsigned` preserves width and x/z.
        let scrut_signed = self.expr_self_signed(scrut_id0);
        let collective_signed = scrut_signed
            && tests
                .iter()
                .all(|(ids, _)| ids.iter().all(|&id| self.expr_self_signed(id)));
        let scrut_id = if scrut_signed && !collective_signed {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Unsigned,
                args: vec![scrut_id0],
            })
        } else {
            scrut_id0
        };

        // Test cascade: for each label, a wildcard-aware match → arm else next.
        for (labels, arm) in &tests {
            for &lbl_id in labels {
                let eq = self.case_cmp(scrut_id, lbl_id, kind);
                let next = b.new_block();
                b.end_block_with(ir::Terminator::Branch {
                    cond: eq,
                    then_bb: arm.raw(),
                    else_bb: next.raw(),
                });
                b.start_block(next);
            }
        }
        // All tests missed → the default (or empty) → merge.
        if let Some(body) = default_body {
            self.lower_stmt(b, body);
        }
        b.goto(merge);

        // Lower each arm body, each ending Goto(merge).
        for (arm, body) in arm_bodies {
            b.start_block(arm);
            self.lower_stmt(b, body);
            b.goto(merge);
        }
        b.start_block(merge);
    }

    /// Shared tail for the seeded / ref-writing blocking intercepts (`$random`,
    /// `$dist_uniform`, `$cast`): push the `lhs = <intercept rhs>` blocking
    /// assign, honoring an intra-assignment delay with capture-now/write-later
    /// (the draw / ref write happens at the capture; only the lhs write delays).
    /// The IR is byte-identical to the prior inline `$random` lowering (IR-0).
    pub(crate) fn emit_blocking_intercept(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs_id: u32,
    ) {
        let lv = self.lower_lvalue(lhs);
        self.check_lvalue_kind(&lv, true);
        if let Some(d) = delay {
            // capture-now/write-later: the DRAW (and seed/ref update) happens at
            // the capture statement; only the lhs write is delayed.
            let w = self.ir_lvalue_width(&lv);
            let tmp = self.fresh_ia_tmp(w);
            let cap = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: whole_net_lvalue(tmp),
                rhs: rhs_id,
            });
            b.push_stmt_id(cap);
            let (amount, region) = self.lower_delay(d);
            let resume = b.new_block();
            b.end_block_with(ir::Terminator::Delay {
                amount,
                region,
                resume: resume.raw(),
            });
            b.start_block(resume);
            let tmp_read = self.push_expr(ir::Expr::Signal {
                net: tmp,
                word: None,
            });
            let wr = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: lv,
                rhs: tmp_read,
            });
            b.push_stmt_id(wr);
        } else {
            let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                lhs: lv,
                rhs: rhs_id,
            });
            b.push_stmt_id(sid);
        }
    }

    /// Lower an `if`-CONDITION expression. `$value$plusargs(fmt, var)` (B1, IEEE
    /// §21.6) is a function that writes `var` (a side effect) and returns 1/0; vita
    /// supports it only as a blocking-assign RHS because of that write. The common
    /// idiom is the condition form `if ($value$plusargs("LIMIT=%d", n)) …`, so
    /// desugar it to a synthetic `__tmp = $value$plusargs(…)` (the supported
    /// statement form — the write gets a controlled placement BEFORE the branch),
    /// then branch on `__tmp`. Any other condition lowers normally (a nested
    /// `$value$plusargs` deeper in an expression stays loud — `lower_expr`).
    pub(crate) fn lower_branch_cond(&mut self, b: &mut ProcessBuilder, cond: &ast::Expr) -> u32 {
        // Peel redundant parens — `if (($value$plusargs(…)))` is the same idiom.
        // A COMPOUND condition (`($value$plusargs(…) && x)`) peels to a Binary,
        // not the bare call, so it falls through to `lower_expr` and stays loud.
        let mut inner = cond;
        while let ast::ExprKind::Paren { inner: i } = &inner.kind {
            inner = i;
        }
        if let ast::ExprKind::SysCall { name, args } = &inner.kind {
            if name.name == "$value$plusargs" {
                return match self.value_plusargs_rhs(args) {
                    Some(rhs_id) => {
                        let tmp = self.fresh_ia_tmp(32);
                        let sid = self.push_stmt(ir::Stmt::BlockingAssign {
                            lhs: whole_net_lvalue(tmp),
                            rhs: rhs_id,
                        });
                        b.push_stmt_id(sid);
                        self.push_expr(ir::Expr::Signal {
                            net: tmp,
                            word: None,
                        })
                    }
                    None => self.placeholder_expr(), // diagnostic already emitted
                };
            }
        }
        self.lower_expr(cond)
    }

    // ── loops (SECONDARY) ──────────────────────────────────────────

    /// F-record-out (§4.5.215) / r18 (F1): lower ONE loop-condition operand to a bool
    /// expr id, hoisting an eval-order-safe inout/output-formal call in it to a copy-out
    /// temp at the CURRENT open block (`head`, or a short-circuit `eval_b`), which is
    /// re-entered on every iteration. An un-hoistable or eval-order-unsafe call lowers in
    /// place and loud-rejects at `emit_frame_call` (correct-or-loud). With no inout-call
    /// this is exactly `lower_expr` (byte-identical).
    pub(crate) fn lower_loop_cond_operand(&mut self, b: &mut ProcessBuilder, e: &ast::Expr) -> u32 {
        if self.expr_has_inout_call(e) && self.hoist_is_safe(e) {
            let hoisted = self.hoist_inout_calls(b, e);
            self.lower_expr(&hoisted)
        } else {
            self.lower_expr(e)
        }
    }

    /// F-record-out (§4.5.215): does the condition `cond` (a while/for loop condition OR an
    /// `if` condition — §4.5.215 follow-on) require the explicit short-circuit branch split?
    /// True iff it is a TOP-LEVEL `&&`/`||` carrying an inout/output-formal call that
    /// `hoist_inout_calls` cannot hoist (an un-hoistable `&&`/`||`-RHS call — see
    /// `has_unhoistable_inout_call`). Gated so a condition with no such call is byte-identical
    /// to before (a single Branch via `lower_loop_cond_operand`); only the reported class (a
    /// conditionally-evaluated output-formal call in a while/for/if condition) is diverted to
    /// the branch chain. DEEPER nesting (`(A||B) && Ccall`) or an eval-order-unsafe operand
    /// still loud-rejects there (correct-or-loud).
    pub(crate) fn cond_needs_shortcircuit_split(&self, cond: &ast::Expr) -> bool {
        matches!(
            &cond.kind,
            ast::ExprKind::Binary {
                op: ast::BinOp::LogAnd | ast::BinOp::LogOr,
                ..
            }
        ) && self.has_unhoistable_inout_call(cond)
    }

    /// F-record-out (§4.5.215): lower a TOP-LEVEL short-circuit condition `A && B` / `A || B`
    /// (a while/for loop condition OR an `if` condition — the `true_bb`/`false_bb` targets are
    /// the loop's `body_bb`/`exit` or the if's `then_bb`/`else_bb`; guarded by
    /// `cond_needs_shortcircuit_split`) as an explicit branch chain, so
    /// the operand carrying an output/inout-formal call is evaluated ONLY on the
    /// short-circuit path that reaches it — its copy-out `Terminator::Call` never fires (and
    /// the output actual is never written) when the other operand already decides the result.
    /// The current open block is `head`; on return the cursor is CLOSED (both `head` and the
    /// synthesized `eval_b` are sealed) and control has been routed to `body_bb` / `exit`.
    ///
    /// ```text
    ///   &&:  head:   cond_a = A;  branch cond_a -> eval_b, exit   (A false ⇒ whole false)
    ///        eval_b: cond_b = B;  branch cond_b -> body,   exit
    ///   ||:  head:   cond_a = A;  branch cond_a -> body,   eval_b (A true  ⇒ whole true)
    ///        eval_b: cond_b = B;  branch cond_b -> body,   exit
    /// ```
    ///
    /// Because `B` is the WHOLE expression of `eval_b`, `lower_loop_cond_operand` hoists its
    /// (now unconditionally-evaluated-within-this-block) inout-call there — reusing
    /// `emit_frame_func_out_call`, the same copy-out the direct-rhs / plain-condition paths
    /// use. A deeper-nested or eval-order-unsafe call in either operand still loud-rejects
    /// (correct-or-loud). `true_bb`/`false_bb` are process-local block ids (`BlockId::raw`):
    /// the whole-condition-TRUE and whole-condition-FALSE targets (a loop's `body_bb`/`exit`,
    /// or an if's `then_bb`/`else_bb`).
    pub(crate) fn lower_shortcircuit_cond(
        &mut self,
        b: &mut ProcessBuilder,
        cond: &ast::Expr,
        true_bb: u32,
        false_bb: u32,
    ) {
        let (op, lhs, rhs) = match &cond.kind {
            ast::ExprKind::Binary { op, lhs, rhs } => (*op, lhs.as_ref(), rhs.as_ref()),
            _ => unreachable!("cond_needs_shortcircuit_split guarantees a top-level && / ||"),
        };
        let is_and = matches!(op, ast::BinOp::LogAnd);
        // head (current open block): evaluate A, branch per the short-circuit rule. Any
        // eval-order-safe inout-call in A is hoisted here (unconditional — the LHS is always
        // evaluated), which is correct.
        let cond_a = self.lower_loop_cond_operand(b, lhs);
        let eval_b = b.new_block();
        if is_and {
            // A false ⇒ whole `&&` false ⇒ false_bb; A true ⇒ evaluate B.
            b.end_block_with(ir::Terminator::Branch {
                cond: cond_a,
                then_bb: eval_b.raw(),
                else_bb: false_bb,
            });
        } else {
            // A true ⇒ whole `||` true ⇒ true_bb; A false ⇒ evaluate B.
            b.end_block_with(ir::Terminator::Branch {
                cond: cond_a,
                then_bb: true_bb,
                else_bb: eval_b.raw(),
            });
        }
        // eval_b: reached ONLY when A did not short-circuit. Evaluate B — its inout-call is
        // hoisted here (emitting the copy-out `Terminator::Call` in this block), so it fires
        // exactly once per path it is reached, and never when A short-circuits.
        b.start_block(eval_b);
        let cond_b = self.lower_loop_cond_operand(b, rhs);
        b.end_block_with(ir::Terminator::Branch {
            cond: cond_b,
            then_bb: true_bb,
            else_bb: false_bb,
        });
    }

    pub(crate) fn lower_while(
        &mut self,
        b: &mut ProcessBuilder,
        cond: &ast::Expr,
        body: &ast::Stmt,
    ) {
        let head = b.new_block();
        let body_bb = b.new_block();
        let exit = b.new_block();
        b.goto(head);
        b.start_block(head);
        // F-record-out (§4.5.215): a TOP-LEVEL `A && B` / `A || B` whose short-circuit-only
        // operand carries an output/inout-formal call `hoist_inout_calls` cannot hoist (it
        // must not be made unconditional) is lowered as an explicit short-circuit branch
        // chain (`lower_shortcircuit_cond`), so the call fires only on the path that
        // reaches it. Otherwise (the common case) the condition is a single Branch, with an
        // eval-order-safe inout-call hoisted at `head` (r18/F1 — see `lower_loop_cond_operand`).
        if self.cond_needs_shortcircuit_split(cond) {
            self.lower_shortcircuit_cond(b, cond, body_bb.raw(), exit.raw());
        } else {
            let c = self.lower_loop_cond_operand(b, cond);
            b.end_block_with(ir::Terminator::Branch {
                cond: c,
                then_bb: body_bb.raw(),
                else_bb: exit.raw(),
            });
        }
        b.start_block(body_bb);
        self.lower_stmt(b, body);
        b.goto(head); // back-edge
        b.start_block(exit);
    }

    pub(crate) fn lower_forever(&mut self, b: &mut ProcessBuilder, body: &ast::Stmt) {
        let head = b.new_block();
        b.goto(head);
        b.start_block(head);
        self.lower_stmt(b, body);
        b.goto(head); // unconditional back-edge
                      // No natural continue point; open a fresh (unreachable) block so the
                      // post-condition (cursor open) holds. It gets Return at finish.
        let dead = b.new_block();
        b.start_block(dead);
    }

    /// `repeat(N)` with a const, small `N` → straight unroll (no runtime counter,
    /// which `Stmt`'s net-only Lvalue cannot express). Non-const/large → reject.
    pub(crate) fn lower_repeat(
        &mut self,
        b: &mut ProcessBuilder,
        count: &ast::Expr,
        body: &ast::Stmt,
    ) {
        // §11.6: a fill literal count is self-determined to 1 bit, so `repeat('1)`
        // is ONE iteration (the generic const-eval would read a 32-bit all-ones
        // value and skip the loop); `'0`/`'x`/`'z` ⇒ zero iterations.
        if let Some((raw, kind)) = fill_literal_ast(count) {
            let once = literal::fill_literal_const(raw, kind, 1)
                .map(|cv| {
                    cv.bits.unk.iter().all(|&u| u == 0)
                        && (cv.bits.val.first().copied().unwrap_or(0) & 1) == 1
                })
                .unwrap_or(false);
            if once {
                self.lower_stmt(b, body);
            }
            return;
        }
        match const_eval_u32(count) {
            Some(n) if n <= REPEAT_UNROLL_CAP => {
                // small constant ⇒ straight unroll (byte-identical to before).
                for _ in 0..n {
                    self.lower_stmt(b, body);
                }
            }
            _ => {
                // A RUNTIME (or large-constant) count: desugar to a signed
                // down-counter while-loop — `cnt = count; while (cnt > 0) { body;
                // cnt = cnt - 1; }`. The count is evaluated ONCE (IEEE §12.7.3); a
                // negative/zero count runs the body zero times. (Previously this
                // warned and OMITTED the body — `repeat(n)` with a variable `n`
                // silently ran zero times.)
                let cnt_net = {
                    let name = format!("$repeat_cnt${}", self.nets.len());
                    let nv = ir::NetVar {
                        kind: ir::NetKind::Integer, // signed 32-bit
                        width: 32,
                        msb: 31,
                        lsb: 0,
                        signed: true,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(ast::NetVarKind::Integer, 32),
                    };
                    self.add_net(&name, nv);
                    (self.nets.len() - 1) as u32
                };
                let count_id = self.lower_expr(count);
                let init = self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: whole_net_lvalue(cnt_net),
                    rhs: count_id,
                });
                b.push_stmt_id(init);
                let head = b.new_block();
                let body_bb = b.new_block();
                let exit = b.new_block();
                b.goto(head);
                b.start_block(head);
                let cnt_rd = self.push_expr(ir::Expr::Signal {
                    net: cnt_net,
                    word: None,
                });
                let zero = self.const_s32_expr(0);
                let cond = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Gt,
                    lhs: cnt_rd,
                    rhs: zero,
                });
                b.end_block_with(ir::Terminator::Branch {
                    cond,
                    then_bb: body_bb.raw(),
                    else_bb: exit.raw(),
                });
                b.start_block(body_bb);
                self.lower_stmt(b, body);
                let cnt_rd2 = self.push_expr(ir::Expr::Signal {
                    net: cnt_net,
                    word: None,
                });
                let one = self.const_s32_expr(1);
                let dec = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::Sub,
                    lhs: cnt_rd2,
                    rhs: one,
                });
                let dec_stmt = self.push_stmt(ir::Stmt::BlockingAssign {
                    lhs: whole_net_lvalue(cnt_net),
                    rhs: dec,
                });
                b.push_stmt_id(dec_stmt);
                b.goto(head);
                b.start_block(exit);
            }
        }
    }

    /// `for (init; cond; step) body` desugars to `init; while (cond) { body; step }`.
    /// The counter is an ordinary declared net (classic Verilog `integer i`), the
    /// same runtime-counter shape `lower_while` already handles — there is no
    /// special suspend-surviving state, so the net-only `Stmt` Lvalue suffices.
    /// (A C99-style `for (int i = 0; …)` block-local counter would need an
    /// automatic per-process frame, which v1 lacks; the parser produces an
    /// assignment `init`, not a declaration, so that case does not arise here.)
    pub(crate) fn lower_for(
        &mut self,
        b: &mut ProcessBuilder,
        init: &ast::Stmt,
        cond: &ast::Expr,
        step: &ast::Stmt,
        body: &ast::Stmt,
    ) {
        self.lower_stmt(b, init); // run the initializer once, before the loop head
        let head = b.new_block();
        let body_bb = b.new_block();
        let exit = b.new_block();
        b.goto(head);
        b.start_block(head);
        // r18 (F1) + F-record-out (§4.5.215): hoist an inout/output-function call out of the
        // for-loop condition — same rationale as `lower_while` (the condition is re-tested at
        // `head` after each step, so the call runs once per iteration). A top-level `&&`/`||`
        // with an un-hoistable short-circuit-RHS call is split into a branch chain; otherwise
        // a single eval-order-safe hoist at `head`.
        if self.cond_needs_shortcircuit_split(cond) {
            self.lower_shortcircuit_cond(b, cond, body_bb.raw(), exit.raw());
        } else {
            let c = self.lower_loop_cond_operand(b, cond);
            b.end_block_with(ir::Terminator::Branch {
                cond: c,
                then_bb: body_bb.raw(),
                else_bb: exit.raw(),
            });
        }
        b.start_block(body_bb);
        self.lower_stmt(b, body);
        self.lower_stmt(b, step); // step runs at the END of each iteration
        b.goto(head); // back-edge to re-test the condition
        b.start_block(exit);
    }
}
