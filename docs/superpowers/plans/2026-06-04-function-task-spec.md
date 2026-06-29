Now I'll correct the `bind_task_locals` helper to use `map_net_kind_or_wire` and `ir::PortDir::Internal`, and use `add_net` (which handles `fq` and dedup) rather than pushing raw. **REVISED M3 helper**:

```rust
    /// Create a per-call scratch net for each task body-local (the task's `body_decls`
    /// plus a single-level begin-end block's `decls`) and bind the local NAME →
    /// scratch NetId via `out_subst` so the inlined body can read AND write it (M3).
    /// Returns nothing; appends nets to the current instance's slice.
    fn bind_task_locals(&mut self, task: &ast::TaskDef) {
        let mut decls: Vec<&ast::NetVarDecl> = task.body_decls.iter().collect();
        if let ast::Stmt::Block { decls: bd, .. } = task.body.as_ref() {
            decls.extend(bd.iter());
        }
        for d in decls {
            let (width, msb, lsb, signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
            for nm in &d.names {
                // unique synthetic name so it never clashes with a real net.
                let local_name = format!("{}$tflocal", nm.name.name);
                let net_id = self.nets.len() as u32;
                self.add_net(
                    &local_name,
                    ir::NetVar {
                        kind: map_net_kind_or_wire(d.kind),
                        width,
                        msb,
                        lsb,
                        signed,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(d.kind, width),
                    },
                );
                // bind the ORIGINAL formal-local name → the scratch net.
                self.out_subst.push((nm.name.name.clone(), net_id));
            }
        }
    }
```

> `add_net` keys through `fq` (instance-prefixed) and appends in order; `net_id = self.nets.len()` captured *before* the push is the new net's id (matches the existing `add_net` contract — verified it pushes one net unless duplicate). The `$tflocal` suffix guarantees no `fq` collision.

Plus the **net_count recompute** after step 7 (option (a)). In `elaborate_instance`, after the step-7 `for item in &module.body { match item … }` loop (line 525) and **before** step 8, add:

```rust
        // Task-call inlining (step 7) may have appended scratch nets for task locals
        // AFTER the step-5 snapshot. Re-snapshot net_count so the instance's net slice
        // covers them. (Still BEFORE step-8 child recursion, so the slice stays
        // contiguous — child nets are appended after this point.)
        self.instances[inst_id as usize].net_count = self.nets.len() as u32 - first_net;
```

### 1f. `crates/elaborate/src/lib.rs` — n1/n3 fold (cosmetic)

In `inline_function`, rename `inputs` → `formals` (after the all-Input guard they're all inputs; the name now reads true) and unify the arity message. Optional, low-priority; the implementer may skip without correctness impact.

---

## 2. Cargo changes

**None.** No new dependencies; all helpers reuse existing crate APIs (`ir::Expr::Select`, `range_to_dims`, `default_init`, `map_net_kind_or_wire`, `add_net`, `const_eval_in_scope`, `const_u32_expr`). Edition 2021 / MSRV 1.82 preserved. Determinism preserved (BTreeMap point-query, Vec reverse-scan, source-order arena growth; scratch nets appended in decl order).

---

## 3. Test cases (8–10, `#[cfg(test)]`)

Parser tests go in `crates/hdl-parser/src/lib.rs` `mod tests` (use `item_of`); elaborate tests go in `crates/elaborate/src/tests.rs` (use the existing `func_def`/`task_def`/`call`/`task_call`/`elab_ok`/`err_codes`/`elab_with_warnings` builders).

**P-ft5 (parser, M1):** `function reg [7:0] f(input [7:0] x); f = x; endfunction` parses to a `FunctionDef` with `name == "f"`, one Input port `x`, and NO error cascade.
```rust
#[test]
fn ft5_reg_return_type_parses() {
    let (su, errs) = p("module m;\nfunction reg [7:0] f(input [7:0] x); f = x; endfunction\nendmodule");
    assert!(errs.is_empty(), "reg return must parse clean, got {errs:?}");
    let m = first_module(&su.unwrap());
    let ModuleItem::Func(f) = &m.body[0] else { panic!("not a func: {:?}", m.body[0]) };
    assert_eq!(f.name.name, "f");
    assert_eq!(f.ports.len(), 1);
    assert_eq!(f.ports[0].name.name, "x");
}
```

**P-ft6 (parser, M1):** same for `function logic [3:0] g(input a); g = a; endfunction` → `name == "g"`, clean.

**E-ft8 (elaborate, B1 truncation):** `function [3:0] f(input [3:0] x); f=x; endfunction` with 8-bit `a`/`y`, `assign y=f(a)`. Assert the cont-assign RHS is `Select{PartConst, width:4}` (the return coercion) over a `Select{PartConst,width:4}` (the input coercion) over `Signal{a}`. Confirms the width-clamp node is emitted.
```rust
#[test]
fn ft_e8_width_coercion_at_formal_and_return() {
    let unit = module("m", vec![
        wire_vec(7, 0, &["a", "y"]),
        func_def("f", Some((3,0)),
            vec![tf_port(ast::PortDir::Input, Some((3,0)), "x")],
            vec![], bassign("f", id_expr("x"))),
        cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
    ]);
    let s = elab_ok(&unit);
    // RHS root = return-width Select(width 4)
    let root = &s.exprs[s.cont_assigns[0].rhs as usize];
    let ir::Expr::Select { base, width: 4, kind: ir::SelKind::PartConst, .. } = root
        else { panic!("expected return Select width 4, got {root:?}") };
    // its base = input-width Select(width 4) over Signal a
    let inner = &s.exprs[*base as usize];
    let ir::Expr::Select { base: b2, width: 4, kind: ir::SelKind::PartConst, .. } = inner
        else { panic!("expected input Select width 4, got {inner:?}") };
    assert!(matches!(s.exprs[*b2 as usize], ir::Expr::Signal { net: 0, .. }));
}
```

**E-ft9 (elaborate, B1 no-op same width):** `function [7:0] f(input [7:0] x); f=x; endfunction`, 8-bit `a`/`y`. Assert elaborate is clean (the same-width Select is present but the test asserts a clean elaborate + that an end-to-end resize would be identity — value-preserving). This guards the "no value regression on shipped tests" claim.

**E-ft10 (elaborate, M2 return-var readback):** `function [7:0] f(input [7:0] x); f=x; f=f+1; endfunction`, `assign y=f(a)`. Pre-fix this errored `ElabUnresolvedName`. Assert `elab_ok` succeeds and the RHS (under the return-width Select) is `Add(Signal a, 1)`.
```rust
#[test]
fn ft_e10_return_var_readback_folds() {
    let body = blk(vec![
        bassign("f", id_expr("x")),
        bassign("f", binop(ast::BinOp::Add, id_expr("f"), dec("1"))),
    ]);
    let unit = module("m", vec![
        wire_vec(7,0,&["a","y"]),
        func_def("f", Some((7,0)),
            vec![tf_port(ast::PortDir::Input, Some((7,0)), "x")],
            vec![], body),
        cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
    ]);
    let s = elab_ok(&unit); // must NOT error
    // peel the return-width Select (width 8) to reach Add(Signal a, Const 1)
    let mut e = &s.exprs[s.cont_assigns[0].rhs as usize];
    if let ir::Expr::Select { base, .. } = e { e = &s.exprs[*base as usize]; }
    let ir::Expr::Binary { op: ir::BinOp::Add, lhs, .. } = e
        else { panic!("expected Add, got {e:?}") };
    assert!(matches!(s.exprs[*lhs as usize], ir::Expr::Signal { net: 0, .. }));
}
```

**E-ft11 (elaborate, M2 input-formal write rejected cleanly):** `function [7:0] f(input [7:0] x); x=x+1; f=x; endfunction`. Assert `elaborate` returns `None` AND the emitted code is `ElabUnsupported` (NOT `ElabUnresolvedName`).
```rust
#[test]
fn ft_e11_input_formal_write_is_unsupported_not_unresolved() {
    let body = blk(vec![
        bassign("x", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
        bassign("f", id_expr("x")),
    ]);
    let unit = module("m", vec![
        wire_vec(7,0,&["a","y"]),
        func_def("f", Some((7,0)),
            vec![tf_port(ast::PortDir::Input, Some((7,0)), "x")], vec![], body),
        cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
    ]);
    let sink = CollectSink::default();
    assert!(elaborate(&unit, &sink).is_none());
    let codes = err_codes(&sink);
    assert!(codes.contains(&MsgCode::ElabUnsupported));
    assert!(!codes.contains(&MsgCode::ElabUnresolvedName), "must be Unsupported, not Unresolved");
}
```

**E-ft12 (elaborate, M3 task body-local):** `task t(input [7:0] d, output [7:0] q); reg [7:0] tmp; begin tmp=d; q=tmp; end endtask`, `initial t(a,y)`. Assert `elab_ok` succeeds, one process, and the final `BlockingAssign` targets caller net `y` with RHS reading the scratch net (the `tmp` net, id == 2, created after a/y).
```rust
#[test]
fn ft_e12_task_body_local_folds() {
    let body = blk(vec![
        bassign("tmp", id_expr("d")),
        bassign("q", id_expr("tmp")),
    ]);
    let unit = module("m", vec![
        netvar(ast::NetVarKind::Reg, Some((7,0)), false, &["a","y"]),
        ast::ModuleItem::Task(ast::TaskDef {
            automatic: false, name: ident("t"),
            ports: vec![
                tf_port(ast::PortDir::Input, Some((7,0)), "d"),
                tf_port(ast::PortDir::Output, Some((7,0)), "q"),
            ],
            body_decls: vec![netvar_decl_reg("tmp")],
            body: Box::new(body), span: SP,
        }),
        proc_item(ast::ProcKind::Initial, None,
            blk(vec![task_call("t", vec![id_expr("a"), id_expr("y")])])),
    ]);
    let s = elab_ok(&unit);            // must NOT error
    assert_eq!(s.nets.len(), 3);       // a, y, tmp$tflocal
    assert_eq!(s.processes.len(), 1);
    // last stmt writes y (net 1) from the scratch net (net 2)
    let p = &s.processes[0];
    let entry = &p.body[p.entry as usize];
    let last = *entry.stmts.last().unwrap();
    let ir::Stmt::BlockingAssign { lhs, rhs } = &s.stmts[last as usize]
        else { panic!("expected BlockingAssign") };
    assert_eq!(lhs.chunks[0].net, 1);  // caller y
    assert!(matches!(s.exprs[*rhs as usize], ir::Expr::Signal { net: 2, .. })); // scratch tmp
}
```

**E-ft13 (elaborate, m1 task output-actual early-return):** `task t(output [7:0] q); q=8'd1; endtask`, `initial t(y[3:0])` (non-simple output actual). Assert `elaborate` returns `None`, emits exactly ONE `ElabUnsupported` (no cascade from inlining the body with `q` unbound).

**E-ft14 (elaborate, n2 mutual recursion):** `function f(input x); f=g(x); endfunction  function g(input x); g=f(x); endfunction`, `assign y=f(a)`. Assert `elaborate` returns `None` with `ElabUnsupported` (the shared `inline_stack` catches the f→g→f cycle) — no stack overflow / no infinite expansion.

**E-ft15 (elaborate, regression):** re-run the **shipped** `ft_e1` shape but assert the VALUE is unchanged through the new same-width return Select — i.e. the cont-assign RHS, peeled of the width-8 return Select, is still `Add(Signal a, Const 1)`. (Guards "no value regression".) *Update existing `ft_e1`/`ft_e2`/`ft_e3` to peel the return-width Select wrapper, OR keep them as-is if `Some((7,0))` formals with 8-bit actuals make the Select a structural addition the asserts must now account for.*

> **Implementer note on test churn:** adding `coerce_to_width` inserts a `Select` wrapper at the return boundary even for same-width functions, so the shipped `ft_e1`/`ft_e2`/`ft_e3`/`ft_e4` structural asserts on `cont_assigns[0].rhs` will now see a `Select` root, not `Binary`/`Signal`. **The implementer MUST update those four tests to peel one `Select{PartConst}` layer before the existing assertions** (a 3-line change each: `let mut e = &s.exprs[rhs]; if let Select{base,..}=e { e=&s.exprs[*base]; }`). This is expected, not a regression — the e2e VCD values are unchanged (Select width == net width == identity resize). This churn is the cost of B1; it is mechanical and low-risk.

---

## 4. Coverage statement + deferred list

**Covered (with this delta):**
- Parser: function/task definitions (ANSI + non-ANSI formals, sticky directions, empty bodies, optional `: label`), **`reg`/`logic` typed returns (M1)**, Integer/Real/Realtime/Time returns, function/task CALLS (already parsed).
- Elaborate inline: combinational functions reducing to a return expr (single `f=e` or straight-line blocking assigns to locals + return var, **including return-var readback and local readback (M2)**); nested non-recursive function calls; zero-arg functions; **formal-input and return width truncation (B1, exact)**; tasks with input formals, output/inout formals to simple nets, **body-local temps (M3)**, and full control flow inside the task body (if/case/delay via normal `lower_stmt`); per-module table scope; determinism.
- Rejection (clean `ElabUnsupported`/`ElabUnresolvedName`, IR discarded): recursive (direct + **mutual, n2**) and automatic functions/tasks; hierarchical calls; arity mismatch; unknown name; control-flow-in-function-expression; **input-formal write (M2)**; output/inout function formal; **non-simple task output actual (m1, early-return)**.

**Deferred (documented, not silent):**
- **Width WIDENING at a formal/return boundary** pads X (B1 residual): a *narrower* actual into a *wider* formal, or a function returning wider than its body's natural width, fills high bits with X rather than zero/sign-extend. Honest X, not a wrong value. Needs a real width-inference pass + a Resize IR node (out of scope; whole-elaborator gap per `eval.rs:337`).
- Recursive/automatic functions and tasks (frame-call execution — doc 06 SD2 deferred path).
- Functions with loops/if/case that don't reduce to one expression in expression context.
- Function return value of type Real (real literals are already `ElabUnsupported`).
- Task output/inout actual that is a part-select / concat / expression (only a bare net in v1).
- Unassigned function return → 0, not X (m2; documented, acceptable v1).
- Nonblocking/event/delay statements *inside a function* (only tasks may carry timing/control flow).

**Fallback if the M3 `net_count` recompute is deemed risky:** replace `bind_task_locals` + the recompute with a one-line guard in `inline_task` — if `!task.body_decls.is_empty()` (or the body block has decls), emit `ElabUnsupported "task body-local variable (deferred)"` and `return`. This converts M3's misleading `ElabUnresolvedName` into a clean deferral (the verdict's minimum bar) at near-zero risk, sacrificing the body-local *support* but keeping diagnostics honest. Ship the full M3 only if the implementer confirms the `net_count` re-snapshot is sound against the corpus golden hashes.

---

## 5. Residual risks

1. **B1 widening-X (medium):** truncation is now exact, but a narrower-actual-into-wider-formal still yields X in the high bits. If real corpus RTL relies on implicit zero-extension at a function boundary (common: `function [7:0] f(input [3:0] x)` called with a 4-bit signal, expecting the formal's high bits = 0), the result is X, not 0. **Mitigation/decision needed:** if the corpus has such cases, switch `coerce_to_width`'s widening path from PartConst to a `Concat{[Const(0, w-src), value]}` zero-extend — but that requires knowing `src` width at elab time, which v1 lacks. The honest-X behavior is the safe default; revisit when width-inference lands.

2. **M3 `net_count` re-snapshot vs. golden hashes (medium):** appending scratch nets and re-snapshotting `net_count` changes the net arena for any module that calls a task with body-locals. If the corpus golden VCDs/schema-hashes were captured *before* M3, they will shift. **The implementer MUST re-run the corpus-runner and re-bless goldens for affected fixtures**, and confirm the contiguity invariant (lib.rs:392–393) still holds (scratch nets land before step-8 child recursion — verified by construction, but assert via a hierarchical-task corpus case).

3. **Test churn from the return-Select wrapper (low):** `ft_e1`–`ft_e4` need the 3-line peel update. Mechanical; if missed, those four tests fail loudly (not a silent issue).

4. **m2 unassigned-return = 0 not X (low):** a downstream `f() === 1'bx` check would read 0. Documented; matches `placeholder_expr`. Mint a real X-const placeholder when the const pool grows a 4-state X literal.

5. **`opt_ret_var_kw` over-eager eat (low):** it only eats `reg`/`logic`, which cannot legally begin anything else in the return-type position, so no false-eat risk. If a future grammar adds `reg` as a valid following token, revisit.

---

### Files touched
- `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/hdl-parser/src/lib.rs` — M1: `opt_ret_var_kw` helper + two call sites in `parse_function_def` (return-type block, ~lines 1741–1746); tests P-ft5/P-ft6 in `mod tests`.
- `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/elaborate/src/lib.rs` — B1: `tf_decl_width` + `coerce_to_width` (after ~1939), wired into `inline_function` actual-arg loop (~1706) and `reduce_function_body` return (~1744); M2: `input_formals` field (struct ~255 + `new()` ~280), `fold_straight_line` Blocking arm (~1770), `reduce_function_body` frame push/pop (~1725); M3: `bind_task_locals` + `inline_task` rewrite (~1844) + `net_count` re-snapshot after step 7 (~525); m1: early-return in `inline_task`.
- `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl/crates/elaborate/src/tests.rs` — E-ft8…E-ft15 + peel-update to `ft_e1`–`ft_e4`.

All five BLOCKER/MAJOR resolved (B1 scoped-correct, M1/M2/M3 fixed), MINORs folded (m1 early-return; m2 noted), NITs noted (n1 rename optional, n2 mutual-recursion test added, n3 message unify optional). Cargo unchanged. Determinism + MSRV 1.82 + edition 2021 preserved.