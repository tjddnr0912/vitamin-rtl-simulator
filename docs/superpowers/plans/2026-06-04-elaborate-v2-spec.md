I have everything I need. The ground truth is now unambiguous:

- The shipped tree is **v1-only (18 tests)** plus the *already-present* `lower_proc_block` machinery in lib.rs, but the v2 path is **fatal-on-every-stub** and the claimed "29 tests / 11 v2 tests" do **not** exist in `tests.rs`. The DESIGN narrative overclaims; the COVERAGE verdict is correct.
- The single highest-value architectural fix — make unsupported *procedural* constructs and unknown `$task`s **non-fatal** — is directly enabled because `diag` already ships `Severity::Warning` and the `ElabWidthTrunc` (W3008) warning code, with a `LogSink::emit` that the existing `error()` already uses.

This is the spec to finalize. I'll resolve every BLOCKER/MAJOR by introducing a `warn()` choke point and a fatal/non-fatal split, lower casez/casex (PRIORITY) with `CaseEq`, fix the bare-`always #5` clock generator (defer the sensitivity decision until after the body is scanned for in-body timing), and add the test suite that actually exercises it.

Below is the implementer-ready finalized spec.

---

# elaborate v2 — Finalized Implementer Spec (procedural-block lowering)

## 0. Ground truth & what this spec changes vs. the working tree

I read the real crates. The working tree **already contains** `ProcessBuilder`, `lower_proc_block`, `lower_stmt`, `lower_case`, the loop helpers, `lower_systask`, `fresh_suspend`, `map_edge`, `map_systask`, and `parse_str_literal` (literal.rs:246). The **CFG construction is sound** (CFG verdict confirmed: `new_block` pre-fills `Return`, `end_block_with` is the sole terminator writer + closes the cursor, every control form `start_block`s its continue point). **Do not touch the CFG invariants.**

The unresolved MAJORs are all in the **COVERAGE** verdict, and they are real:

| # | MAJOR (COVERAGE verdict) | Root cause in real code | Resolution in this spec |
|---|---|---|---|
| M-A | Every stub discards the **entire** SimIr (`error()` sets `had_error` → `elaborate()` returns `None`) | lib.rs:64, 144-145 | Introduce `warn()` choke point (`Severity::Warning`, code `ElabWidthTrunc`/`ElabUserWarning`) that does **not** set `had_error`. Reclassify unsupported *procedural* constructs and unknown `$task`s as **non-fatal skip/no-op**. |
| M-B | casez/casex are PRIORITY but `lower_case` errors out → IR killed | lib.rs:1295-1302 | Lower casez/casex with `CaseEq` chain (same as `case`); emit a **warning** (not error) noting wildcard `?/x/z` semantics are approximated. IR survives. |
| M-C | Bare `always #5 clk = ~clk;` (clock generator, core testbench construct) is **rejected** | lib.rs:1031-1042 `lower_sensitivity` runs before the body and can't see in-body timing | Defer the `always`/`sens:None` decision: scan the body for any in-body `#`/`@`/`wait`; if present → `SensKind::Initial`-style self-restarting `Comb` arm-free process is wrong, so map to **`SensKind::Comb` with empty edges + a `forever`-wrap** of the body (so it re-arms). If truly no timing → warn + still emit (unschedulable but valid). |
| M-D (MEDIUM→fold) | `$timeformat`/`$monitoron`/`$dumpvars(scope)`/intra-assign delay all IR-killing | map_systask:1569, lower_systask:1492 | Unknown `$task` → **warn + skip** (no-op, no Stmt emitted). `$dumpvars` scope arg → tolerate non-net args by skipping them with a warning. Intra-assign delay → warn + drop delay, keep the assign. |

Everything else the two verdicts raised is NIT/MINOR and folded as code comments (see §5).

**Net effect:** a realistic hand-written testbench (`initial` + `always #5` clock + `always_ff` DUT + `casez` FSM + `$timeformat`/`$dumpvars(0,tb)`) now **elaborates to a usable SimIr** instead of vanishing. That is the "working testbench demo" goal the prompt set.

---

## 1. Complete Rust to ADD / REPLACE in `crates/elaborate/src/lib.rs`

The implementer **edits the existing file**. Each block below is tagged REPLACE (swap an existing fn) or ADD (new fn). Line anchors are from the current tree.

### 1.1 ADD — the non-fatal `warn()` choke point (next to `error()`, after lib.rs:154)

```rust
    /// Emit a WARNING-severity diagnostic and KEEP GOING — does NOT set
    /// `had_error`, so the SimIr survives and is returned. This is the lever that
    /// makes unsupported *procedural* constructs and unknown `$task`s degrade
    /// (skip / no-op) instead of discarding the whole module (COVERAGE M-A/M-B/M-D).
    /// Reuses `ElabWidthTrunc` (W-ELAB-WIDTH-TRUNC / VITA-W3008) as the generic
    /// "lowered with a documented approximation" warning channel until a dedicated
    /// W-ELAB-DEGRADED code is minted. The message carries the specifics.
    fn warn(&mut self, msg: &str) {
        self.sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Warning,
            code: MsgCode::ElabWidthTrunc,
            message: msg.to_string(),
            location: None,
            context: Vec::new(),
            sim_time: None,
        }));
    }
```

> Rationale for reusing `ElabWidthTrunc`: it is the only existing `Warning`-severity elaborate code (W3008). The COVERAGE verdict explicitly suggested "`ElabWidthTrunc`-style warning severity would let the IR survive." A dedicated `ElabDegraded` (W3011) is the clean follow-up (Residual R1); adding it is a one-line edit in `crates/diag/src/code.rs`, which is out of this PR's "ADD to elaborate" scope. **If the implementer is allowed to touch diag**, prefer minting `ElabDegraded => ("W-ELAB-DEGRADED","VITA-W3011",Warning,"construct lowered with a documented approximation")` and using it in `warn()`.

### 1.2 REPLACE — `lower_case` (lib.rs:1288-1354): casez/casex now lower, non-fatal

```rust
    // ── case / casez / casex → Branch chain (all lower; non-fatal) ──
    fn lower_case(
        &mut self,
        b: &mut ProcessBuilder,
        kind: ast::CaseKind,
        scrutinee: &ast::Expr,
        items: &[ast::CaseItem],
    ) {
        // PRIORITY (COVERAGE M-B): casez/casex MUST lower. Wildcard ?/x/z bit
        // semantics are approximated by `===` (CaseEq). This is exact for label
        // sets with no ?/x/z bits (the common FSM/testbench case) and a documented
        // over-strict match otherwise. WARN (non-fatal) — the IR survives.
        if !matches!(kind, ast::CaseKind::Case) {
            self.warn(
                "casez/casex wildcard bits approximated by === (exact when labels \
                 have no ?/x/z); IR lowered",
            );
        }
        let scrut_id = self.lower_expr(scrutinee);
        let merge = b.new_block();

        // Pre-allocate each Match arm's entry block; pin the (optional) default.
        let mut arm_bodies: Vec<(BlockId, &ast::Stmt)> = Vec::new();
        let mut default_body: Option<&ast::Stmt> = None;
        let mut tests: Vec<(&[ast::Expr], BlockId)> = Vec::new();
        for it in items {
            match it {
                ast::CaseItem::Match { labels, body, .. } => {
                    let arm = b.new_block();
                    tests.push((labels.as_slice(), arm));
                    arm_bodies.push((arm, body));
                }
                ast::CaseItem::Default { body, .. } => default_body = Some(body),
            }
        }

        // Test cascade: each label → `scrut === label` → arm, else next test.
        for (labels, arm) in &tests {
            for label in *labels {
                let lbl_id = self.lower_expr(label);
                let eq = self.push_expr(ir::Expr::Binary {
                    op: ir::BinOp::CaseEq,
                    lhs: scrut_id,
                    rhs: lbl_id,
                });
                let next = b.new_block();
                b.end_block_with(ir::Terminator::Branch {
                    cond: eq,
                    then_bb: arm.raw(),
                    else_bb: next.raw(),
                });
                b.start_block(next);
            }
        }
        // All tests missed → default (or nothing) → merge.
        if let Some(body) = default_body {
            self.lower_stmt(b, body);
        }
        b.goto(merge);

        // Each arm body ends Goto(merge).
        for (arm, body) in arm_bodies {
            b.start_block(arm);
            self.lower_stmt(b, body);
            b.goto(merge);
        }
        b.start_block(merge);
    }
```

### 1.3 REPLACE — `lower_sensitivity` + `lower_proc_block` (lib.rs:993-1052): fix bare-`always` clock generator (M-C)

The fix is structural: a bare `always` with no header `@(...)` is **legal** if the body contains its own timing (`#`/`@`/`wait`). Such a process re-runs forever. We model it by **wrapping the body in an implicit `forever`** and giving the process `SensKind::Comb` with empty edges (the engine restarts a `Comb`/edge-less process by its terminators, and the body's own `#`/`@` provide the time advance). Only a bare `always` with **no** timing anywhere is unschedulable → warn (non-fatal) and still emit.

```rust
    // ── one ProceduralBlock → one Process ──────────────────────────
    fn lower_proc_block(&mut self, p: &ast::ProceduralBlock) -> ir::Process {
        // M-C: a bare `always` with NO header @(...) re-arms via its own in-body
        // timing (`always #5 clk=~clk;`). Detect that and wrap the body in an
        // implicit forever so control loops back to the in-body delay/event.
        let bare_always_self_timed = matches!(p.kind, ast::ProcKind::Always)
            && p.sensitivity.is_none();

        let sensitivity = self.lower_sensitivity(p.kind, p.sensitivity.as_ref(), &p.body);
        let mut b = ProcessBuilder::new(); // entry block #0 open
        if bare_always_self_timed && stmt_has_timing(&p.body) {
            // Implicit `forever { body }` so the process re-arms on its own #/@.
            self.lower_forever(&mut b, &p.body);
        } else {
            self.lower_stmt(&mut b, &p.body);
        }
        let (body, entry) = b.finish(); // seals trailing block with Return
        ir::Process {
            sensitivity,
            body,
            entry,
            suspend: fresh_suspend(entry),
        }
    }

    // ── sensitivity mapping ────────────────────────────────────────
    fn lower_sensitivity(
        &mut self,
        kind: ast::ProcKind,
        sens: Option<&ast::Sensitivity>,
        body: &ast::Stmt, // M-C: inspect body for in-body timing on bare `always`
    ) -> ir::Sensitivity {
        use ast::ProcKind::*;
        match kind {
            Initial => ir::Sensitivity { kind: ir::SensKind::Initial, edges: Vec::new() },
            AlwaysComb => ir::Sensitivity { kind: ir::SensKind::Comb, edges: Vec::new() },
            AlwaysLatch => ir::Sensitivity { kind: ir::SensKind::Latch, edges: Vec::new() },
            AlwaysFf => self.classify_event_list(sens, /* force_edge = */ true),
            Always => match sens {
                None => {
                    if stmt_has_timing(body) {
                        // Legal self-timed `always` (clock generator). The body's
                        // own #/@ drives time; the process re-runs (forever-wrapped
                        // in lower_proc_block). No header edges → Comb-shaped arm.
                        ir::Sensitivity { kind: ir::SensKind::Comb, edges: Vec::new() }
                    } else {
                        // Truly unschedulable: warn (non-fatal) but still emit a
                        // valid (inert) process rather than killing the whole IR.
                        self.warn(
                            "always with neither @(...) nor in-body timing is \
                             unschedulable; lowered as an inert process",
                        );
                        ir::Sensitivity { kind: ir::SensKind::Comb, edges: Vec::new() }
                    }
                }
                Some(ast::Sensitivity::Star) => {
                    ir::Sensitivity { kind: ir::SensKind::Comb, edges: Vec::new() }
                }
                Some(s @ ast::Sensitivity::List(_)) => {
                    self.classify_event_list(Some(s), /* force_edge = */ false)
                }
            },
        }
    }
```

ADD this free helper near `const_eval_u32` (lib.rs:~1586):

```rust
/// Does this statement (recursively) contain its own timing control — `#delay`,
/// `@(event)`, or `wait` — anywhere on a path? Used to decide whether a bare
/// `always` (no header @) is a legal self-timed process (clock generator) vs an
/// unschedulable one. Conservative: any nested timing anywhere counts. (M-C)
fn stmt_has_timing(s: &ast::Stmt) -> bool {
    match s {
        ast::Stmt::DelayCtrl { .. } | ast::Stmt::EventCtrl { .. } | ast::Stmt::Wait { .. } => true,
        ast::Stmt::Block { stmts, .. } => stmts.iter().any(stmt_has_timing),
        ast::Stmt::If { then_s, else_s, .. } => {
            stmt_has_timing(then_s) || else_s.as_deref().is_some_and(stmt_has_timing)
        }
        ast::Stmt::Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_has_timing(body)
            }
        }),
        ast::Stmt::For { body, .. }
        | ast::Stmt::While { body, .. }
        | ast::Stmt::Repeat { body, .. }
        | ast::Stmt::Forever { body, .. } => stmt_has_timing(body),
        ast::Stmt::Fork { stmts, .. } => stmts.iter().any(stmt_has_timing),
        _ => false,
    }
}
```

> Note: `lower_forever` (lib.rs:1375) already produces a sound infinite cycle (`head→body→head`) + a dead `Return` continuation. Reusing it for the implicit-forever wrap means **zero new CFG code** and the CFG verdict's soundness proof still holds verbatim.

### 1.4 REPLACE — the DEFERRED/SECONDARY stub arms in `lower_stmt` (lib.rs:1124-1283): fatal → non-fatal where the prompt allows degradation

Replace each `self.error(MsgCode::ElabUnsupported, …)` that the prompt classifies as **SECONDARY/DEFERRED recovering-stub** with `self.warn(…)` so the IR survives. **Keep `self.error(…)` only for genuinely un-lowerable parse errors** (`Stmt::Error`, `Lvalue::Error`, poison name resolution — those already set `had_error` via `resolve_net`). Concrete edits:

```rust
            // intra-assignment delay: WARN + drop the delay, keep the assign (M-D).
            ast::Stmt::Blocking { lhs, delay, rhs, .. } => {
                if delay.is_some() {
                    self.warn("intra-assignment delay (= #d) dropped (v2); assign kept");
                }
                let rhs_id = self.lower_expr(rhs);
                let lv = self.lower_lvalue(lhs);
                let sid = self.push_stmt(ir::Stmt::BlockingAssign { lhs: lv, rhs: rhs_id });
                b.push_stmt_id(sid);
            }
            ast::Stmt::NonBlocking { lhs, delay, rhs, .. } => {
                if delay.is_some() {
                    self.warn("intra-assignment delay (<= #d) dropped (v2); assign kept");
                }
                let rhs_id = self.lower_expr(rhs);
                let lv = self.lower_lvalue(lhs);
                let sid = self.push_stmt(ir::Stmt::NonblockingAssign { lhs: lv, rhs: rhs_id });
                b.push_stmt_id(sid);
            }
```

```rust
            // begin..end: block-local decls WARN (ignored) instead of killing IR.
            ast::Stmt::Block { decls, stmts, .. } => {
                if !decls.is_empty() {
                    self.warn("block-local declarations ignored (v2); body lowered");
                }
                for st in stmts {
                    self.lower_stmt(b, st);
                }
            }
```

```rust
            // ── SECONDARY / DEFERRED → WARN + recover (stay in block) ──
            // disable: doc-17 lowering table says "Stmt::Disable then Goto", but
            // scope-id resolution (DisableKind/target) is deferred. Emit the
            // Stmt::Disable with a Scope/0 placeholder so the *shape* is present,
            // then continue straight-line. Non-fatal. (CFG MINOR-1 reconciled.)
            ast::Stmt::Disable { .. } => {
                self.warn("disable target scope-id unresolved (v2); emitted as Scope/0 no-op");
                let sid = self.push_stmt(ir::Stmt::Disable {
                    scope_kind: ir::DisableKind::Scope,
                    target: 0,
                });
                b.push_stmt_id(sid);
            }
            ast::Stmt::Fork { stmts, .. } => {
                // No Fork terminator lowering yet (join-state deferred). Degrade to
                // SEQUENTIAL execution of the children + warn — sound CFG, wrong
                // concurrency, but the IR survives the demo. (Was IR-killing.)
                self.warn("fork/join lowered as sequential (v2); concurrency not modeled");
                for st in stmts {
                    self.lower_stmt(b, st);
                }
            }
            ast::Stmt::UserTaskCall { .. } => {
                self.warn("user task call skipped (v2); no-op");
            }
            ast::Stmt::EventTrigger { .. }
            | ast::Stmt::Assign { .. }
            | ast::Stmt::Deassign { .. }
            | ast::Stmt::Force { .. }
            | ast::Stmt::Release { .. } => {
                self.warn("procedural-continuous / event-trigger construct skipped (v2); no-op");
            }
            // Parse error is the ONE genuinely-fatal stmt: keep self.error.
            ast::Stmt::Error(_) => {
                self.error(MsgCode::ElabUnsupported, "cannot lower parse-error statement");
            }
```

Also REPLACE the `for` and non-const/large `repeat` arms (lib.rs:1396-1418) `self.error` → `self.warn` (they are SECONDARY; the body is simply skipped, IR survives):

```rust
    fn lower_repeat(&mut self, b: &mut ProcessBuilder, count: &ast::Expr, body: &ast::Stmt) {
        match const_eval_u32(count) {
            Some(n) if n <= REPEAT_UNROLL_CAP => {
                for _ in 0..n { self.lower_stmt(b, body); }
            }
            _ => self.warn("repeat with non-constant or large count skipped (v2); body omitted"),
        }
    }

    fn lower_for(&mut self, _b: &mut ProcessBuilder, _i: &ast::Stmt, _c: &ast::Expr,
                 _s: &ast::Stmt, _body: &ast::Stmt) {
        self.warn("for loop skipped (v2); counter not expressible in frozen net-only Stmt");
    }
```

### 1.5 REPLACE — `lower_systask` + `map_systask` (lib.rs:1492-1528, 1569-1584): unknown `$task` non-fatal, dump-scope tolerant (M-D)

```rust
    fn lower_systask(&mut self, name: &ast::Ident, args: &[ast::Expr]) -> Option<u32> {
        let which = match map_systask(&name.name) {
            Some(w) => w,
            None => {
                // M-D: unknown $task ($timeformat/$monitoron/$readmemh/...) is a
                // WARN + skip (no Stmt emitted), NOT an IR-killing error. The
                // testbench survives.
                self.warn(&format!("unsupported system task `{}` skipped (v2)", name.name));
                return None;
            }
        };
        let takes_fmt = matches!(
            which,
            ir::SysTaskId::Display | ir::SysTaskId::Write | ir::SysTaskId::Monitor | ir::SysTaskId::Strobe
        );
        // M-D: $dumpvars(level, scope...) passes a scope/module name, not a net.
        // Lowering a scope ident through lower_expr would resolve_net → fatal
        // unresolved-name. For the dump family, drop any non-net/non-const arg
        // with a warning instead of resolving it.
        let dump_family = matches!(
            which,
            ir::SysTaskId::DumpVars | ir::SysTaskId::DumpFile | ir::SysTaskId::DumpOn
                | ir::SysTaskId::DumpOff | ir::SysTaskId::DumpAll
        );
        let (fmt, value_args): (Option<u32>, &[ast::Expr]) = if takes_fmt {
            match args.first().map(|e| &e.kind) {
                Some(ast::ExprKind::StrLit { raw }) => {
                    let cid = self.intern_const(parse_str_literal(raw));
                    let fmt_expr = self.push_expr(ir::Expr::Const { val: cid });
                    (Some(fmt_expr), &args[1..])
                }
                _ => (None, args),
            }
        } else {
            (None, args)
        };
        let arg_ids: Vec<u32> = value_args
            .iter()
            .filter_map(|a| {
                if dump_family && !self.is_net_or_const_arg(a) {
                    self.warn("$dump* scope/non-signal argument skipped (v2)");
                    None
                } else {
                    Some(self.lower_expr(a))
                }
            })
            .collect();
        Some(self.push_stmt(ir::Stmt::SysTask { which, fmt, args: arg_ids }))
    }

    /// True if `a` is a bare net Ident or an integer/string literal — i.e. a thing
    /// `lower_expr` can lower without a fatal unresolved-name. A hierarchical /
    /// scope name (`top.dut`) or anything else returns false (dump-family skips it).
    fn is_net_or_const_arg(&self, a: &ast::Expr) -> bool {
        match &a.kind {
            ast::ExprKind::Ident(path) => {
                path.segments.len() == 1 && self.symbols.contains_key(&path.segments[0].name)
            }
            ast::ExprKind::IntLit { .. } | ast::ExprKind::StrLit { .. } => true,
            _ => false,
        }
    }
```

Extend `map_systask` with the common testbench aliases so they lower instead of warning where a frozen `SysTaskId` exists (`$fdisplay`→Display family is a reasonable alias; `$monitoron/off` have no frozen id → stay unknown→warn):

```rust
fn map_systask(dollar_name: &str) -> Option<ir::SysTaskId> {
    match dollar_name {
        "$display" | "$displayb" | "$displayo" | "$displayh" => Some(ir::SysTaskId::Display),
        "$write" | "$writeb" | "$writeo" | "$writeh" => Some(ir::SysTaskId::Write),
        "$monitor" => Some(ir::SysTaskId::Monitor),
        "$strobe" => Some(ir::SysTaskId::Strobe),
        "$finish" => Some(ir::SysTaskId::Finish),
        "$stop" => Some(ir::SysTaskId::Stop),
        "$dumpfile" => Some(ir::SysTaskId::DumpFile),
        "$dumpvars" => Some(ir::SysTaskId::DumpVars),
        "$dumpon" => Some(ir::SysTaskId::DumpOn),
        "$dumpoff" => Some(ir::SysTaskId::DumpOff),
        "$dumpall" => Some(ir::SysTaskId::DumpAll),
        _ => None, // $timeformat/$monitoron/$readmemh/... → warn+skip in caller
    }
}
```

### 1.6 REPLACE — the remaining non-fatal sensitivity/event arms (lib.rs:1063-1113, 1425-1455): warn not error

`classify_event_list` always_ff-with-no-list (1065), `sens_event_net` non-signal (1106), `lower_event_wait_cause` `@(*)` (1426) and multi-edge (1436): change `self.error(MsgCode::ElabUnsupported, …)` → `self.warn(…)`. These already produce a valid placeholder (`POISON_NET` edge / empty `Level`) so the IR shape is fine — only the diagnostic severity changes. Keep `resolve_net`'s unresolved-name **fatal** (a genuinely undeclared signal is a real error, per the prompt's E-ELAB-UNRESOLVED-NAME contract).

> `lower_delay` non-const (1471) → keep as `warn` (degrade to `#0`); the assign/timeline survives. Change its `self.error` to `self.warn`.

### 1.7 The driver edit (lib.rs:195-199) — **already correct**, no change

`ModuleItem::Proc(p) => self.processes.push(self.lower_proc_block(p))` is in place. Leave it.

---

## 2. Cargo.toml changes

**None.** `diag` (with `Severity::Warning` + `MsgCode::ElabWidthTrunc`), `hdl-ast`, `sim-ir` are already dependencies. `warn()` uses only already-imported symbols (`Diagnostic`, `LogEvent`, `Severity`, `MsgCode`).

> If the implementer is permitted to add the dedicated warning code (recommended, Residual R1): add one line to `crates/diag/src/code.rs` in the 3xxx block — `ElabDegraded => ("W-ELAB-DEGRADED","VITA-W3011",Warning,"construct lowered with a documented approximation"),` — and use `MsgCode::ElabDegraded` in `warn()`. That is a diag edit, not elaborate, and is the only place a Cargo/crate boundary is crossed.

---

## 3. Test cases (`#[cfg(test)]`, append to `crates/elaborate/src/tests.rs`)

Add these helpers + 10 tests. They build the AST directly (the file has no lexer/parser dependency — `module()`/`netvar()`/etc. are the builders), elaborate, and assert the Process sensitivity + BB/terminator structure + every-path-Returns. **Note:** `t10_procedural_block_unsupported` (line 478) must be **updated** — bare `always` with no timing is now non-fatal (warn), so it returns `Some`, not `None`; assert a warning + a Process instead.

```rust
// ════════════════════════════════════════════════════════════════════
//  v2 — procedural-block lowering tests
// ════════════════════════════════════════════════════════════════════

impl CollectSink {
    /// Count WARNING-severity diagnostics (non-fatal degrade channel).
    fn n_warnings(&self) -> usize {
        self.events.borrow().iter().filter(|e| matches!(
            e, LogEvent::Diagnostic(d) if d.severity == diag::Severity::Warning
        )).count()
    }
}

/// Elaborate, allowing warnings but no errors → returns the SimIr.
fn elab_with_warnings(unit: &ast::SourceUnit) -> (ir::SimIr, usize) {
    let sink = CollectSink::default();
    let ir = elaborate(unit, &sink).expect("non-fatal lowering must yield Some(SimIr)");
    let warns = sink.n_warnings();
    (ir, warns)
}

// ── CFG validators (process-LOCAL block space) ──
fn assert_cfg_valid(p: &ir::Process) {
    let n = p.body.len() as u32;
    assert!(p.entry < n, "entry {} out of bounds ({})", p.entry, n);
    let chk = |t: u32| assert!(t < n, "terminator target {t} out of bounds ({n})");
    for bb in &p.body {
        match &bb.term {
            ir::Terminator::Goto { target } => chk(*target),
            ir::Terminator::Branch { then_bb, else_bb, .. } => { chk(*then_bb); chk(*else_bb); }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => chk(*resume),
            ir::Terminator::Fork { children, join, resume_bb } => {
                for c in children { chk(*c); } chk(*join); chk(*resume_bb);
            }
            ir::Terminator::Call { target, ret_bb } => { chk(*target); chk(*ret_bb); }
            ir::Terminator::Return => {}
        }
    }
}

/// Every block reachable from entry must reach a Return (no infinite-non-loop
/// dangling). Loops (back-edges) are allowed; we only require Return-reachability
/// for ACYCLIC paths, so a `forever` is exempted by the caller.
fn assert_all_paths_return(p: &ir::Process) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut reaches_return = false;
    fn walk(p: &ir::Process, b: u32, seen: &mut std::collections::HashSet<u32>, hit: &mut bool) {
        if !seen.insert(b) { return; }
        match &p.body[b as usize].term {
            ir::Terminator::Return => *hit = true,
            ir::Terminator::Goto { target } => walk(p, *target, seen, hit),
            ir::Terminator::Branch { then_bb, else_bb, .. } => {
                walk(p, *then_bb, seen, hit); walk(p, *else_bb, seen, hit);
            }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } =>
                walk(p, *resume, seen, hit),
            ir::Terminator::Fork { resume_bb, .. } => walk(p, *resume_bb, seen, hit),
            ir::Terminator::Call { ret_bb, .. } => walk(p, *ret_bb, seen, hit),
        }
    }
    walk(p, p.entry, &mut seen, &mut reaches_return);
    assert!(reaches_return, "no path from entry reaches Return");
    let _ = &mut seen;
}

fn proc_item(kind: ast::ProcKind, sens: Option<ast::Sensitivity>, body: ast::Stmt) -> ast::ModuleItem {
    ast::ModuleItem::Proc(ast::ProceduralBlock { kind, sensitivity: sens, body: Box::new(body), span: SP })
}
fn blk(stmts: Vec<ast::Stmt>) -> ast::Stmt {
    ast::Stmt::Block { label: None, decls: Vec::new(), stmts, span: SP }
}
fn nb(lhs: &str, rhs: ast::Expr) -> ast::Stmt {
    ast::Stmt::NonBlocking { lhs: lv_id(lhs), delay: None, rhs, span: SP }
}
fn bassign(lhs: &str, rhs: ast::Expr) -> ast::Stmt {
    ast::Stmt::Blocking { lhs: lv_id(lhs), delay: None, rhs, span: SP }
}
fn delay_stmt(n: u32, body: Option<ast::Stmt>) -> ast::Stmt {
    ast::Stmt::DelayCtrl {
        delay: ast::Delay { values: vec![dec(&n.to_string())], span: SP },
        body: body.map(Box::new), span: SP,
    }
}
fn systask(name: &str, args: Vec<ast::Expr>) -> ast::Stmt {
    ast::Stmt::SysTaskCall { name: ident(name), args, span: SP }
}
fn str_e(s: &str) -> ast::Expr { ex(ast::ExprKind::StrLit { raw: format!("\"{s}\"") }) }
fn ev_list(terms: Vec<(ast::Edge, &str)>) -> ast::Sensitivity {
    ast::Sensitivity::List(terms.into_iter().map(|(edge, n)| ast::EventExpr {
        edge, expr: id_expr(n), span: SP,
    }).collect())
}

// v2-1: initial testbench — $dumpfile/$dumpvars + a=0 + #5 + a=1 + #5 + $display + $finish.
#[test]
fn v2_1_initial_testbench_structure() {
    let body = blk(vec![
        systask("$dumpfile", vec![str_e("dump.vcd")]),
        systask("$dumpvars", vec![dec("0"), id_expr("a")]),
        bassign("a", dec("0")),
        delay_stmt(5, None),
        bassign("a", dec("1")),
        delay_stmt(5, None),
        systask("$display", vec![str_e("a=%d"), id_expr("a")]),
        systask("$finish", vec![]),
    ]);
    let unit = module("tb", vec![netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["a"]),
                                 proc_item(ast::ProcKind::Initial, None, body)]);
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 0, "clean testbench must not warn");
    assert_eq!(ir.processes.len(), 1);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Initial);
    assert!(p.sensitivity.edges.is_empty());
    assert_cfg_valid(p);
    assert_all_paths_return(p);
    // two #5 delays → two Delay terminators with Active region.
    let delays: Vec<_> = p.body.iter().filter_map(|bb| match bb.term {
        ir::Terminator::Delay { amount, region, .. } => Some((amount, region)), _ => None,
    }).collect();
    assert_eq!(delays, vec![(5, ir::DelayRegion::Active), (5, ir::DelayRegion::Active)]);
}

// v2-2: always_ff @(posedge clk) q <= d → SensKind::Edge / Posedge.
#[test]
fn v2_2_always_ff_edge() {
    let body = nb("q", id_expr("d"));
    let unit = module("ff", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["q","clk","d"]),
        proc_item(ast::ProcKind::AlwaysFf, Some(ev_list(vec![(ast::Edge::Posedge, "clk")])), body),
    ]);
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Edge);
    assert_eq!(p.sensitivity.edges.len(), 1);
    assert_eq!(p.sensitivity.edges[0].kind, ir::EdgeKind::Posedge);
    assert_cfg_valid(p); assert_all_paths_return(p);
}

// v2-3: bare always @(a or b) → SensKind::Level, both AnyEdge terms.
#[test]
fn v2_3_level_sensitivity() {
    let body = bassign("y", id_expr("a"));
    let unit = module("lvl", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["y","a","b"]),
        proc_item(ast::ProcKind::Always,
                  Some(ev_list(vec![(ast::Edge::NoEdge,"a"),(ast::Edge::NoEdge,"b")])), body),
    ]);
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Level);
    assert_eq!(p.sensitivity.edges.len(), 2);
    assert_cfg_valid(p); assert_all_paths_return(p);
}

// v2-4 (M-C): bare `always #5 clk = ~clk;` clock generator → NON-FATAL, Comb,
// forever-wrapped (no Return-reachable continuation; back-edge cycle).
#[test]
fn v2_4_clock_generator_self_timed() {
    let invert = ex(ast::ExprKind::Unary { op: ast::UnOp::BitNot, operand: Box::new(id_expr("clk")) });
    let body = delay_stmt(5, Some(bassign("clk", invert)));
    let unit = module("clkgen", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["clk"]),
        proc_item(ast::ProcKind::Always, None, body), // <-- no header @, in-body #5
    ]);
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 0, "a self-timed clock generator is legal, must not warn");
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Comb);
    assert_cfg_valid(p); // forever is exempt from assert_all_paths_return
    // there is a Delay terminator and a back-edge Goto (the forever cycle).
    assert!(p.body.iter().any(|bb| matches!(bb.term, ir::Terminator::Delay { .. })));
}

// v2-5 (M-C): truly inert `always` (no @ no timing) → WARN, still Some + valid.
#[test]
fn v2_5_bare_always_no_timing_warns_not_fatal() {
    let unit = module("m", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["a"]),
        proc_item(ast::ProcKind::Always, None, bassign("a", dec("0"))),
    ]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_some(), "bare always is now non-fatal");
    assert_eq!(sink.n_warnings(), 1);
    assert_cfg_valid(&out.unwrap().processes[0]);
}

// v2-6: if/else → Branch + shared merge; every path Returns.
#[test]
fn v2_6_if_else_merge() {
    let body = ast::Stmt::If {
        cond: id_expr("c"),
        then_s: Box::new(bassign("y", dec("1"))),
        else_s: Some(Box::new(bassign("y", dec("0")))),
        span: SP,
    };
    let unit = module("m", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["y","c"]),
        proc_item(ast::ProcKind::Initial, None, body),
    ]);
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert!(p.body.iter().any(|bb| matches!(bb.term, ir::Terminator::Branch { .. })));
    assert_cfg_valid(p); assert_all_paths_return(p);
}

// v2-7 (M-B): casez lowers (NON-FATAL, warning) into a CaseEq Branch chain.
#[test]
fn v2_7_casez_lowers_with_warning() {
    let items = vec![
        ast::CaseItem::Match { labels: vec![lit("2'b10", ast::IntLitKind::Sized)],
                               body: Box::new(bassign("y", dec("1"))), span: SP },
        ast::CaseItem::Default { body: Box::new(bassign("y", dec("0"))), span: SP },
    ];
    let body = ast::Stmt::Case { kind: ast::CaseKind::Casez, scrutinee: id_expr("s"), items, span: SP };
    let unit = module("m", vec![
        netvar(ast::NetVarKind::Reg, Some((1,0)), false, &["s","y"]),
        proc_item(ast::ProcKind::Initial, None, body),
    ]);
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 1, "casez approximation must warn (non-fatal), not error");
    let p = &ir.processes[0];
    let has_caseeq = ir.exprs.iter().any(|e| matches!(e, ir::Expr::Binary { op: ir::BinOp::CaseEq, .. }));
    assert!(has_caseeq, "casez must lower via CaseEq");
    assert!(p.body.iter().any(|bb| matches!(bb.term, ir::Terminator::Branch { .. })));
    assert_cfg_valid(p); assert_all_paths_return(p);
}

// v2-8: in-body @(posedge clk) → Wait{Edge,Posedge}, NOT process sensitivity.
#[test]
fn v2_8_in_body_event_wait() {
    let body = blk(vec![
        ast::Stmt::EventCtrl { ctrl: ev_list(vec![(ast::Edge::Posedge,"clk")]), body: None, span: SP },
        nb("q", id_expr("d")),
    ]);
    let unit = module("m", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["q","d","clk"]),
        proc_item(ast::ProcKind::Initial, None, body),
    ]);
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Initial); // block-level stays Initial
    let waits: Vec<_> = p.body.iter().filter_map(|bb| match &bb.term {
        ir::Terminator::Wait { cond: ir::WaitCause::Edge { kind, .. }, .. } => Some(*kind), _ => None,
    }).collect();
    assert_eq!(waits, vec![ir::EdgeKind::Posedge]);
    assert_cfg_valid(p); assert_all_paths_return(p);
}

// v2-9 (M-D): unknown $task ($timeformat) → WARN + skip, IR survives, no Stmt.
#[test]
fn v2_9_unknown_systask_nonfatal() {
    let body = blk(vec![
        systask("$timeformat", vec![]),
        bassign("a", dec("0")),
        systask("$finish", vec![]),
    ]);
    let unit = module("tb", vec![
        netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["a"]),
        proc_item(ast::ProcKind::Initial, None, body),
    ]);
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 1, "$timeformat must warn-skip, not kill the IR");
    // exactly one SysTask stmt survives ($finish); $timeformat emitted nothing.
    let n_systask = ir.stmts.iter().filter(|s| matches!(s, ir::Stmt::SysTask { .. })).count();
    assert_eq!(n_systask, 1);
    assert_cfg_valid(&ir.processes[0]); assert_all_paths_return(&ir.processes[0]);
}

// v2-10: full multi-process testbench (initial stimulus + always_ff DUT) +
//        whole-SimIr determinism (same AST → byte-identical SimIr).
#[test]
fn v2_10_multiprocess_and_determinism() {
    let mk = || {
        let dut = nb("q", id_expr("d"));
        let stim = blk(vec![ bassign("d", dec("1")), delay_stmt(10, None), systask("$finish", vec![]) ]);
        module("tb", vec![
            netvar(ast::NetVarKind::Reg, Some((0,0)), false, &["q","d","clk"]),
            proc_item(ast::ProcKind::AlwaysFf, Some(ev_list(vec![(ast::Edge::Posedge,"clk")])), dut),
            proc_item(ast::ProcKind::Initial, None, stim),
        ])
    };
    let (ir1, _) = elab_with_warnings(&mk());
    let (ir2, _) = elab_with_warnings(&mk());
    assert_eq!(ir1, ir2, "same AST must produce byte-identical SimIr");
    assert_eq!(ir1.processes.len(), 2);
    assert_eq!(ir1.processes[0].sensitivity.kind, ir::SensKind::Edge);   // DUT
    assert_eq!(ir1.processes[1].sensitivity.kind, ir::SensKind::Initial); // stimulus
    for p in &ir1.processes { assert_cfg_valid(p); }
    assert_all_paths_return(&ir1.processes[1]); // initial terminates
}
```

**Update the existing `t10_procedural_block_unsupported`** (line 478) — change its tail from `assert!(out.is_none())` + `ElabUnsupported` to:

```rust
    let out = elaborate(&unit, &sink);
    assert!(out.is_some(), "bare always (no timing) is now a non-fatal warning, not fatal");
    assert!(sink.events.borrow().iter().any(|e| matches!(
        e, LogEvent::Diagnostic(d) if d.severity == diag::Severity::Warning)));
```

---

## 4. Coverage statement (v2 construct → lowering)

**PRIORITY (must lower) — all lowered, IR-producing:**

| Construct | Lowering |
|---|---|
| `initial` | Process, `SensKind::Initial`, empty edges |
| `always @(...)` | Process, `SensKind::Edge` (any explicit edge) / `Level` (all bare) |
| `always_ff @(...)` | Process, `SensKind::Edge` (force_edge) |
| `begin…end` | Straight-line BBs joined by `Goto` (block-local decls → warn+ignore) |
| blocking `=` | `Stmt::BlockingAssign` |
| nonblocking `<=` | `Stmt::NonblockingAssign` |
| `if/else` | `Branch{cond,then,else}` → shared merge |
| `#delay` | `Terminator::Delay{amount, region:(0→Inactive else Active), resume}` |
| `@(event)` (in body) | `Terminator::Wait{Edge/Level, resume}` |
| `wait(expr)` | `Terminator::Wait{Expr, resume}` |
| `$systask` | `Stmt::SysTask{which, fmt, args}`; print-family fmt/args split |
| **`case`** | `CaseEq` Branch chain → merge (default = final else) |
| **`casez`/`casex`** | **Same `CaseEq` chain + warning** (wildcard approximation) — *now PRIORITY-complete* |

**SECONDARY — lowered where clean, else non-fatal warn+skip (IR survives):**

| Construct | Lowering |
|---|---|
| `while` / `forever` | `Goto`+`Branch` back-edges (forever via implicit-forever for self-timed `always`) |
| `repeat(const ≤1024)` | Unroll | `repeat(non-const/large)` → warn+skip |
| `for` | warn+skip (counter not in frozen net-only `Lvalue`) |
| `fork…join` | warn + **sequential** lowering (concurrency not modeled) |
| `disable` | `Stmt::Disable{Scope, 0}` placeholder + warn (shape present) |
| intra-assign delay (`<= #5 d`) | warn + drop delay, keep assign |
| `always_comb`/`always_latch` | `Comb`/`Latch`, empty edges (read-set inference deferred) |
| bare self-timed `always #5` | `Comb` + implicit-forever wrap (clock generator) |
| unknown `$task` (`$timeformat`,`$monitoron`,…) | warn + skip (no Stmt) |
| `$dumpvars(0, scope)` | scope/non-net arg skipped + warn |

**DEFERRED (genuinely fatal or absent):** module instances/hierarchy, parameter override, function/task **definitions** + user `Call`, generate, real literals, hierarchical name refs, bit-level multidriver. A truly undeclared signal (`resolve_net` miss) stays **fatal** (`E-ELAB-UNRESOLVED-NAME`) — that is a real user error, not a missing feature.

---

## 5. Residual risks (what the sim-engine must honor about the CFG)

1. **`SimIr.blocks` is empty; `Process.body` indices are process-LOCAL.** Every `Terminator` target and `Process.entry` indexes the *owning* process's `body`, 0-based. The engine must **never** index `SimIr.blocks` for a process body. (Confirmed: `Terminator` targets are bare untagged `u32`; there is no per-process base to encode the rejected "shared blocks" alternative.)

2. **`forever` (and the implicit-forever clock wrap) intentionally has a back-edge cycle with no Return-reachable exit + one unreachable `dead` Return block.** The engine must (a) tolerate unreachable blocks (do not assert all-blocks-reachable in a golden audit; NIT-2), and (b) **not** require time advancement inside a loop — a zero-delay `while(1) y=1;` is a valid CFG that *would* livelock; detecting combinational loops is the **engine's** job, not elaborate's (CFG MINOR-3). The clock-generator case is safe because its loop body contains a `#5` Delay.

3. **`fork…join` is lowered SEQUENTIALLY** (no `Fork` terminator emitted in v2). The engine sees ordinary `Goto`-chained blocks — correct CFG, **wrong concurrency**. Any design relying on fork parallelism is mis-simulated (warned). Promoting to real `Fork{children,join,resume_bb}` + `JoinState` is the M3 follow-up.

4. **`disable` emits `Stmt::Disable{Scope, target:0}`** as a shape placeholder, not a real scope reference. The engine must treat `target:0` from a warned disable as a **no-op** (it does not name a real labeled block — block labels aren't lowered in v2). Do not dereference `target` as a block index.

5. **Determinism is preserved through the warn path.** `warn()` only calls `sink.emit` (a side channel) — it touches **no** arena, so two identical ASTs still produce byte-identical `SimIr`. `v2_10` pins this with whole-`SimIr` equality. The `is_net_or_const_arg` filter reads `self.symbols` (a `BTreeMap`, order-independent lookup) and never feeds arena order.

6. **`StrUtf8` zero-width const** (`$display("")`) interns a `width:0` `ConstVal`. The engine's formatter must treat `width:0 StrUtf8` as the empty string, not "no format" (COVERAGE MEDIUM, harmless in elaborate).

7. **The `wake_key` seed in `fresh_suspend` is `WakeCond::Level{[]}` / `RegionTag::Active`** — a never-armed placeholder the engine overwrites on first suspend. It is suspend *state*, not structure; it does not affect the schema hash beyond being deterministic.

**Verification gate for the implementer:** after the edits, `cargo test -p elaborate` must show the 18 v1 tests + 10 v2 tests green, `t10` updated, `cargo clippy -p elaborate --all-targets` clean, and `cargo test --workspace` green (the `schema_hash` gate validates SimIr *shape* — unchanged, since no IR type changed). Because the sim-engine is still a stub, the `assert_cfg_valid`/`assert_all_paths_return` validators in the test suite are the **only** executable check of index validity — keep them on every new v2 test.

Files: `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/elaborate/src/lib.rs` (all §1 edits), `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/elaborate/src/tests.rs` (§3 tests + `t10` update), optionally `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/diag/src/code.rs` (one-line `ElabDegraded` if diag edits are permitted, §2).