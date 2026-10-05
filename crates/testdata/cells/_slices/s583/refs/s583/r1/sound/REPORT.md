# §4.5.583 soundness lens — round 1
status: DONE round 1 (~52 tool calls)
binaries: PRE md5 2492e1c5c52c7081a6c0997fe194af40, POST md5 41566ca6a6c8eaf9a232bf5741a7058c

## Q1 Stmt::If producer census (what the walk can meet in else position)
hdl-parser constructors of Stmt::If:
- stmt_ctl.rs:83 parse_if — else = parse_statement() (stmt_ctl.rs:78-80)
- assertions.rs:298 parse_assert, plain immediate `assert`/`assume` (stmt.rs dispatch `Kw::Assert | Kw::Assume`):
  Stmt::If{cond, then_s=pass action, else_s=Some(fail action or synthesized $error)} — a Stmt::If that is NOT an `if` statement
- type_param_shape.rs:176 (module-item Initial, not statement position), udp_table.rs:118/126, udp.rs:619-670 (UDP), monomorph.rs:194/656 (in-place subst patterns, no rebuild)
- assertions.rs:462 parse_unique_priority itself (inner `unique/priority if` returns an If with its own arm)
Non-If wrappers ending the series: parse_labeled_stmt (stmt.rs:232 → Block), parse_seq_block, parse_delay_stmt/parse_event_stmt, `;` Null, DeferredAssert (`assert #0/final`).
elaborate-side Stmt::If constructors (sva_*, cover*, events.rs:333, const_fn.rs:1652 is a match arm) run after parse; none re-runs the walk.
SUSPECT H1: walk follows into the immediate-assert desugar and through its fail action if that is an `if`.

## Q2 function-body / const-interpreter census
- function bodies: parse_function_def only (functask.rs:81); callers classes.rs:285 (class methods), module_items.rs:321 parse_function_item ← module_items.rs:615 ($unit), :1340 (module/interface/package items). tf_body callers functask.rs:259 (function, flag set), :308 (task, flag not set). flag save/restore functask.rs:258-260, no early exit between.
- parse_function_item (module_items.rs:320-334): `function void` becomes ModuleItem::Task — body parsed under in_function_body=true.
- exec_const_stmt (const_fn.rs:1588) entered only from eval_const_call (const_fn.rs:1509) + self-recursion; eval_const_call callers const_fn.rs:514 (pkg::f), :520 (f), :1020 (nested call inside a const fn expression).
- body lookup const_fn_def (const_fn.rs:1379-1398): pkg_funcs / const_func_table only. Populated from ast::ModuleItem::Func only: instance.rs:527, iface_inst.rs:232, package.rs:833→1033, imports package.rs:1695/1730.
- exec_const_stmt arms: Null, Block, Blocking, If, Return, While, For, Repeat; `_ => None` (statement-level calls incl. TaskCall / void call / SysTaskCall refuse BEFORE any callee body runs).
=> bodies the interpreter can execute = non-void functions declared at module/interface/package/$unit top level. Void functions (→Task) and class methods are never in the tables.

## Q3 span-keyed consumers
(u32,u32)-keyed tables in elaborate: lib.rs:833 frame_repeat_cnt (repeat span), lib.rs:842 frame_case_tmp (case span), lib.rs:1563/driver.rs:556 escape_warned (StrLit escapes), tables.rs:21 ForkModeTable (fork span), tables.rs:423 TaskCallProc (task-call span), block_local_feed.rs:25/block_local_class.rs:343 branch_of (block-local decl spans), const_eval.rs:979, packed_pattern.rs:123, gen_enum.rs:216, pkg_scoped_frames.rs:285, decl_collide.rs:648 (expr/decl spans).
None keys an If statement; the arm (SysTaskCall + StrLit) shares a span with its container in PRE already (lone-if arm, case default arm). Arm consumer: systask.rs:195 → SeverityKind::UniqueViolation; location via stmt_locs (v29).
## Q4 routing until-when
Nothing re-runs parse_unique_priority; monomorph mutates in place. Staleness = format_version + semver_major only (vita-artifact gate.rs:51; header.rs:17 tool_version "NEVER a staleness key").
## Findings (cells in r1/sound/cells; vita outputs native=interp=vm identical unless noted)
### F1 BLOCKING (new): walk follows the immediate-assert desugar → W4031 on a path with an else
Census: only two parser producers put a Stmt::If in statement position: parse_if (stmt_ctl.rs:83) and parse_assert's plain
immediate assert/assume (assertions.rs:298, else_s always Some). The walk's test `matches!(**e, Stmt::If{..})` (assertions.rs diff)
treats the assert node as an `else if` and descends into its FAIL action; if that action is an `if` without else, the arm lands there.
Cell c1_assert_if.sv line 4: `#1 unique if (a) x = 1; else assert (b) else if (c) $display(...);` a=b=c=0.
- verilator 5.052 --assert: `t=1 A1 done` / `t=2 c-branch` / `t=3 A3 done` / `t=4 A4 done` (no 'unique if' violation)
- PRE: same 4 lines, no W4031 (rc=0)
- POST: `c1_assert_if.sv:4:15: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for priority or unique case statement [in top] [at time 1]`
  and `c1_assert_if.sv:12:14: warning[VITA-W4031] ... [at time 4]` (line 12 = `priority if … else assume (b) else if (c)`)
- POST staged vcmp→velab→vrun identical to one-shot; POST-jit (VITA_JIT=1) identical, `JITBODY templates_compiled=1 refused=0 activations=1`.
- Boundary c8 (`else assert #0 (b) else if (c)`, DeferredAssert): PRE=POST=verilator `t=1 D1 done`, no report.
Control c6 (`else assert (c);`, default fail): PRE=POST=E4003 Assertion failed, no W4031; verilator `'assert' failed.`
Root: predicate keyed on node kind, not on the written `else if`. Fix direction: follow only an If whose span begins at an `if` keyword
(assert desugar spans begin at `assert`/`assume`) or decide the series in parse_if.
### F2 NON-BLOCKING (pre-existing silent, over-narrowing): flag covers bodies the interpreter can never run
Coordinator question. exec_const_stmt runs only FunctionDefs found by const_fn_def (const_fn.rs:1379-1398) in pkg_funcs/const_func_table,
filled only from ast::ModuleItem::Func (instance.rs:527, iface_inst.rs:232, package.rs:832-833→1033, imports package.rs:1695/1730).
`function void` → ModuleItem::Task at parse (module_items.rs:322-331); class methods → ClassItem (classes.rs:285): never in the tables.
A void function called as a statement in a constant function hits exec_const_stmt `_ => None` before any body runs:
- c2a (fv with `else unique if`) PRE=POST: `c2a_fv_in_cf.sv:9:26: error[VITA-E3009] ... parameter `P` value is not a constant: `cf(…)` has no constant-fold arm`
- c2b (fv with NO unique at all) PRE=POST: identical E3009 → refusal is the call, not an arm. c2c (no call): P=4.
- verilator c2a: `%Error: c2a_fv_in_cf.sv:9:26: Expecting expression to be constant, but can't determine constant for FUNCREF 'cf'`
Residue cell c3p (plain chains in class `function void m`, class `function integer f`, module `function void mv`):
- verilator: `[1] %Error: c3p_class_fn.sv:4: Assertion failed in top.C.m: 'unique if' statement violated`, `[2] … :8: … top.C.f …`, `[4] … :12: … top.mv …`
- PRE=POST: no W4031 (only `t=2 r=0`, `t=3 m-z`)
Lane check c3h (H spelling, lone inner unique if, armed on PRE and POST): W4031 at :4:56 [in top.C.m] t1, :8:33 [in top.C.f] t2, :12:54 [in top.mv] t4, 3 backends identical.
Decision: keying the opt-out on "body can be constant-evaluated" = non-void function outside a class (parse_function_def already returns
is_void; the class caller is classes.rs:285) is SOUND for E3009. Generate-scope functions are also never in the tables (instance.rs:527
walks top-level module.body) but stay opted out conservatively.
### F3 NON-BLOCKING (pre-existing class, note): the arm is frozen into the .vu at vcmp
.vu = postcard hdl_ast::SourceUnit; gate = schema_hash::<SourceUnit> (cli/src/pipeline.rs:756-757); tool_version never a key (vita-artifact header.rs:17).
c7_chain.sv: PRE vcmp .vu → POST velab/vrun: accepted rc=0, `t=1 done`, NO W4031; POST .vu → POST: W4031 :4:15; POST .vu → PRE velab/vrun: W4031 :4:15.
Same class as every parser-desugar slice; no format bump is consistent with policy.
### F4 clean (positive): attribute after else follows the series
c5_attr.sv `else (* foo *) if (b)`: verilator `[1] %Error: c5_attr.sv:4: Assertion failed in top: 'unique if' statement violated`; POST W4031 :4:15; PRE silent.
### Q3 verdict: clean by census (no If-keyed span table); Q4: nothing re-desugars after parse (F3 is the only "until when").

Q3 addendum: span.lo-keyed tables (block_local_class.rs:675/839, block_local/hoist.rs:75-795 per_entry/scoped_block_locals) are keyed by Block spans; frames_classify.rs:817 / stmt_flow.rs:561,1157 by repeat/case spans. No If- or SysTaskCall-keyed table.
Q2 addendum: only one Parser constructor (api.rs:27), so no sub-parser can reset in_function_body; enums.rs:233 synthesizes a Case body (no arm).
exec_const_stmt catch-all at const_fn.rs:1719.

## Lens verdict
Product wobbles: YES, on one new shape (F1): `unique/priority if … else assert/assume (c) else if (d) …` prints W4031 where PRE and verilator print nothing (3 backends, JIT, staged).
Everything else censused is stable: flag restored on the only exit, no span-keyed collision, no artifact/format change, no constant-fold regression (exec_const_stmt cannot reach task, void-function or class-method bodies).
