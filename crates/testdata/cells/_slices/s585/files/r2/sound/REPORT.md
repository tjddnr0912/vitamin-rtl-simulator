# §4.5.585 soundness lens — round 2 (delta)
PRE   S/s585/pre/vita   md5 c766b8d59edabaef96436e7b46e75dc6
POST2 S/s585/post2/vita md5 6f9c776f1e234a5dd04609f67302000f (slice2.patch md5 4a3813a5…)
JIT   S/s585/tgt-jit/release/vita md5 3f20e9a0… ; product S/s585/tgt-nodef/release/vita md5 f97f67cf…
## Status
- [x] D1 re-run round-1 cells on POST2: F1/F2 cells PRE = POST2; run.json, obs-procs, non-W4031 = PRE
- [x] D2 census: every subroutine body under the flag; save/restore exact; no procedural block under the flag
- [x] D3 procedural t0 routes: none beyond 🆕 AA / self-timed (5 new cells + t0 census of 370 cells)
## Findings
none — round-1 F1 and F2 closed (PRE = POST2); no new finding

## D1 — round-1 cells on POST2 (tab_r1.txt; 49 lens cells, native table + 3-backend equality)
- F1 cells q6a q6h q6d q6j q6k q6p: POST t0 W4031 -> POST2 silent = PRE (rc 0). q6b (illegal) silent = PRE.
- F2 cell q6t: POST rc 1 E3009@16 -> POST2 rc 0 silent = PRE; q6w (import): POST t0 x2, t2 -> POST2 silent = PRE.
- Refusal cells (q3g*, q3i, q3k, q5p, q6f, q6q, q6s, q6u, q6v, q7a-f): PRE = POST = POST2 (same code and line).
- Procedural chains armed on POST2 = vl lines/times: q1a 6@1,8@2; q1b 9@1,11@2,15@4,17@5; q1d 7@2;
  q3_flag_restore 45@8; q4t11 4@2; q4t5 4@2; q5_long_chain250 260@3; q5n 11@2,13@3 x2. No POST2 t0 line here.
- Silent on POST2 where vl reports (subroutine-body residue by design): q1d chain.svh:2, q3_flag_restore
  7/13/24/27, q4t1/2/3/8/13/14, q4t7, q5_long 5@2, q5n 7@1; q5n t4 `priority if .. else unique if .. else if ..
  else unique0 if` (vl [4]@15) silent by the F3 rule — the differential lens's call, not a soundness finding.
- 365 cells (317 planner + 48 lens) x 3 backends = 1095 runs: non-W4031 output diffs 0 (round 1's 4 E3009 text
  changes gone); run.json route diffs 0 (round 1's 6 wprog-counter diffs gone); --obs-procs: processes equal in
  every cell, builtins differ only in the row `unique/priority check` (57 row diffs).

## D2 — census of the delta
- Flag: writers 3 (lib.rs:976 init false; functask.rs:655 replace true; :855 restore), reader 1 (assertions.rs:503).
  tf_body spans functask.rs:650-857: no `return`, no `?`, no nested function/task/class/module/procedural parse
  between 655 and 855; parse errors are fatal before elaborate.
- Subroutine bodies: tf_body callers 2 (functask.rs:257 parse_function_def, :305 parse_task_def); their callers
  classes.rs:285/:292, module_items.rs:321/:619/:1343. FunctionDef/TaskDef constructions 4: functask.rs:266, :309
  (tf_body results), module_items.rs:323 (TaskDef from the tf_body-parsed void function), enums.rs:233 (synthetic
  enum method: Stmt::Case of returns, no qualifier). DPI 0, interface class / pure / extern = parse errors (classes.rs),
  `let` = expression (module_items.rs:1421), covergroup / property / sequence / constraint / randomize-with carry no
  statement bodies.
- Statement roots (19 parse_statement call sites, 14 fns): only 4 start a context — tf_body (flag true),
  parse_procedural_block (module_items.rs:1336), module-level concurrent assert action (module_items.rs:1393 ->
  parse_assert_action_block), module-level cover property (:1409); the rest nest inside a statement. parse_statement
  has no Initial/Always/Final/Class/Module/Function/Task arm, so no procedural block parses under the flag; a `fork`
  in a subroutine inherits flag true (first-if-only).
- .vu identity: sub_only.sv (chains in package task / void / non-void fn, class new / task / void, interface void fn /
  task with fork, module task with do-while, formal-less module fv reached from a function on a net-decl init):
  PRE .vu == POST2 .vu (md5 160b5c70…); positive control (+1 procedural chain) 4659 vs 4758 B.
- Lanes on POST2: 58 cells, JIT (VITA_JIT=1) == default on all (JITBODY templates_compiled>=1 on 19 cells, refused!=0 on 0); product ==
  default on 57; vl_lines F4004 S3b = pre-existing (identical on the PRE product build, round 1).

## D3 — procedural t0 routes
- New cells (vl / iverilog case twin / PRE-H / POST2): d3a two initials + always @(a or b): t2 / Time 2 / t2 / t2;
  d3b program initial: [0],[2] / Time 0, Time 2 / t0,t2 / t0,t2 (both oracles report t0); d3c always_comb via
  `assign w = f(a,b)` + always_latch: t2 / Time 2 x2 / t2 / t2; d3d child `initial #0`: silent everywhere;
  d3e module-level SVA action block, posedge at t0: `act t=0 r=2`, [2] / iverilog rejects / t2 / t2.
- t0 census over 370 cells: POST2 t0 cells not in PRE = aa1_chain (vl silent; 🆕 AA), s1s_selftimed_first (vl [1];
  iverilog s1s_case Time 0,1), s3_child_selftimed_tbL (vl [2]; iverilog Time 0,2) — recorded splits; s1i, s2 tbF,
  s2 tbL (vl [0] re-measured) and d3b (vl [0], iverilog Time 0) — oracles report t0 too. PRE t0 cells lost: none.
wall_s: 414
