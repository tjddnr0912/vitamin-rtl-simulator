# §4.5.589 round 3 (final delta) — SOUNDNESS lens REPORT (final)
round: 3 · POST_c $S/s589/post_c/vita e31d955b… (staged post_c/sep) · PRE e1e7e571… · post_b bea24059… · new cells $S/s589/r3/sound/c (6 of 12)

## Census: every kind-carrying call site vs what it folds (POST_c tree $S/s589/wt)
Old merged reader `pkg_own_rtns`: 0 hits. Raw `pkg_own_funcs`/`pkg_own_tasks`: writers package.rs:834 (Func arm) / :841 (Task arm), init driver.rs:58-59, only reader decl_scope.rs:132-133 inside `pkg_owns`.
| site (enclosing fn) | call | kind | folds | verdict |
|---|---|---|---|---|
| const_fn.rs:1417 pkg_fn_own | pkg_owns | Func | interpreter's running function | match |
| decl_scope.rs:341 decl_probe_fn | pkg_owns | Func | bare constant FUNCTION callee | match |
| decl_scope.rs:149 decl_win / pkg_body_scope.rs:162 push_rtn_pkg_scope | pkg_owns | param | — | pass-through |
| const_fn.rs:1461 eval_const_call_at | decl_win | Func | callee header | match |
| decl_scope.rs:171 decl_local_win | decl_win | Func | interpreter body locals | match |
| decl_scope.rs:428 const_fn_ret_wsign_in | decl_win | Func | call return range (typing) | match |
| frames_reserve.rs:781 reserve_frame_func | decl_win | Func | function frame | match |
| frames_reserve.rs:1245 reserve_frame_task | decl_win | Task | task frame | match |
| frames_call/emit.rs:79 emit_frame_call | decl_win | Func | `func: FunctionDef` formals | match |
| inline_fn.rs:441 inline_resolved_func_in_pkg | decl_win | Func | inline function | match |
| inline_task.rs:394 inline_task | decl_win | Task | static task formals | match |
| inline_fn.rs:504 / frames_body.rs:359 | push_rtn_pkg_scope | Func | function bodies | match |
| frames_body.rs:535 / inline_task.rs:755 | push_rtn_pkg_scope | Task | task bodies | match |
| inline_fn.rs:484, emit.rs:207 emit_frame_call, emit.rs:912 emit_frame_func_out_call | with_default_arg_scope | Func | function defaults | match |
| inline_task.rs:494, emit.rs:521 emit_frame_task_call | with_default_arg_scope | Task | task defaults | match |
decl_body_win takes `sc.kind` from the pushed scope; window cache key and re-entry compare (pkg, rtn, kind). 23 sites, 0 mismatches.
Doc sentence (decl_scope.rs:324-325 "Only a FUNCTION the package DECLARES answers … a task of that name does not make an imported function its own"): true — decl_probe_fn asks pkg_owns(.., Func) = pkg_own_funcs (Func arm only); when p declares function f, pkg_funcs[p][f] is that declaration (package.rs:832 `funcs.insert` overwrites; import arm :878/:883 `entry().or_insert` fills only an absent name).

## Cells
r1+r2 (38 cells) on post_c, one-shot + staged: post_b -> post_c movers = 1 (r2c1 `v=1023 b=10` -> `v=15 b=4` = PRE; all three oracles refuse the design); one-shot == staged 38/38; PRE -> post_c movers all silent->correct or split (k11 Q, k11b, k13, c18, c19, c22, c24, two/ab u, c21 split; c15b top-level is a missing-.pre artefact, its st/ copy PRE = post_c).
New (kind lanes): PRE | post_b | post_c (= staged) | iverilog / verilator / sv2v
- r3c1_ptask_loc (automatic package task local): `v=255` | `v=15` | `v=15` | `v=15` x3
- r3c2_ptask_fml (automatic task formal): `v=255` | `v=15` | `v=15` | `v=15` x3
- r3c3_stask_fml (static task formal): `v=255` | `v=15` | `v=15` | `v=15` x3
- r3c4/r3c5 task default `= g()` (automatic/static): `v=3` on all | iverilog, verilator `v=3`; sv2v `Missing argument 2 of call to task`
- r3c6 function with output + default: `v=3 r=3` on all | verilator `v=3 r=3`; iverilog, sv2v refuse (`port o is not an input port`)

## Mutants ($S/s589/r3/sound/mut)
Worktree $S/s589/rv_sound3 = 303703f9 + post_c/wt.diff (diff -rq vs $S/s589/wt: identical), CARGO_TARGET_DIR $S/s589/target_rv_sound3 (debug). M0: test file 44/44; killer cells = post_c release values.
| M | mutation | test file (44) | --workspace | killer cell post_c -> mutant |
|---|---|---|---|---|
| M17 | frames_reserve.rs:1248 task reserve passes `RtnKind::Func` | 1 failed (task_local_range_binds_package_function) | not needed | r3c1 `v=15` -> `v=255`; r3c2 `v=15` -> `v=255` |
| M18 | inline_task.rs:397 static-task formals pass `RtnKind::Func` | 44/44 pass | 9092 passed (1 leaky), 15 skipped, rc 0 — SURVIVOR | r3c3_stask_fml `v=15` -> `v=255` (oracles `v=15` x3) |
Source restored (diff -rq empty); worktree and target removed.

## Findings (round 3)
| id | sev | new / same class | file:line | evidence | cell |
|---|---|---|---|---|---|
| S5 | MINOR | new instance of S3's class (missing pin) | inline_task.rs:394-398 (static-task formal arming, L5) | M18 survives the whole workspace suite | r3c3_stask_fml PRE `v=255` / post_c `v=15` / oracles `v=15` x3: pin it |
| S2 | MINOR | residue, unchanged (F2 class) | static package task block-local, unarmed | | c23 `v=255`, oracles `v=15` |
S4: closed (r2c1 back to PRE; pins declared_task_does_not_own_an_imported_function{,_in_the_reserve}).
Lens verdict: PASS — 23 kind sites match what they fold, no reader of the merged notion left, doc sentence true, post_b -> post_c moves only r2c1 (to PRE), 0 correct->wrong vs PRE on 44 lens cells, staged == one-shot 44/44, no crash.

