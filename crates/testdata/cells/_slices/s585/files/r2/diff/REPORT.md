# §4.5.585 r2 (delta) DIFFERENTIAL lens — REPORT
VERDICT r2: PASS (product does not shake). One doc nit (N1). wall_s ~900.
PRE  S/s585/pre/vita md5 c766b8d59edabaef96436e7b46e75dc6 | POST2 S/s585/post2/vita md5 6f9c776f1e234a5dd04609f67302000f
(worktree `git diff | md5` = 4a3813a5b19e49d893826896d55c0ccc = slice2.patch) | JIT md5 3f20e9a0… | product md5 f97f67cf…
Harness: S/s585/r2/diff/h2.py, eq2.py (= r1 h.py/eq.py with POST -> post2). Table: S/s585/r2/diff/table_r1cells.txt
## Status
- [x] D0 delta read: stmt_ctl.rs:81 `written_if = self.at_kw(Kw::If)` only; functask.rs:655/855 tf_body sets/restores
  first_if_arm_only=true (only call sites functask.rs:257 function, :305 task; no early exit between 655 and 855).
- [x] D1 r1 cell set (40 cells, 3 backends, staged, JIT): PRE(r2) == PRE(r1) all; PRE lines lost 0; JIT == POST2 43/43.
  STAGED-DIFF rows = elaboration refusals (stop at velab, as in r1) + p1d/p3f (my staged() drops -D/-f: harness-format).
  F3 closed: q23_prio_u0 POST2 silent at t1/t2 (POST had 5:17, 6:17), q23b silent.
  POST -> POST2 drops are all judged residue: task/void-fn/class-task chains (p2_inc t1@2, p3_multi t3@4 t4@8, q9_ifc 5 lines,
  q9b 3, s1_struct t0@11 + t8@7x3, s2/s2b task+fv lines, s3a t3@7 t6@12, e2 priority-in-task, q18 t2@5) and
  `else unique0 if` (q16 t3@9, q16b t3@12, q23 t3@9; verilator reports them).
- [x] D2 invariants eq2.py over 314 cells (all census + r1 + r2): VCD, stdout minus W4031, --obs-procs evals, JIT == POST2:
  314/314 OK (r1's four E3009-text STDOUT-DIFF cells are back to PRE text). Product build == POST2 14/14.
- [x] D3 bare-if (r2a_bare_forms): `else (* m *) if` 6:15, `else \`IFM` 7:15, ifdef-guarded `if` 8:15, plain 3-chain 26:15,
  `if … else unique if … else if` 30:20, `else (* m *) unique if … else if` 32:49 = verilator lines 6/7/8/26/30/32;
  `priority if … else unique0 if` silent = verilator. Residue (judged): t4 POST2 16:20 vs vl :14, t5 20:22 vs vl :18
  (inner-line), t6 silent vs vl :22 (`else unique0 if`).
- [x] D4 procedural-only: r2b_gen (generate-for always x2 t1@6, generate-if initial t6@17, final t9@20 = verilator; priority
  initials t3/t4@11 no oracle), r2c_proc_ctx (interface initial t1@2, program initial t2@6, fork join_any t3@19,
  join_none t5@23, chain calling a task t6@25, final t10@32 = verilator; task chain t7/t8@14 = residue; join_any priority
  branch t4@20 no oracle), r2e_restore (always_comb after module fn/task + class: t0@22, t4@22, named block t1@29 =
  verilator; class/module subroutine chains residue), r2d_pkg_initial: E3009 PRE = POST2, verilator syntax error.
- [x] D5 t0: POST2-only t0 lines over 314 cells = aa1_chain:5, s1i_initial_first:5, s2_child_initial_tbF:17,
  s2_child_initial_tbL:5, s1s_selftimed_first:5, s3_child_selftimed_tbL:5, q26_comb_xreg:11 = the judge's list exactly.
  Outside it only r2e_restore:22 t0 (always_comb, decl-init inputs a=b=0, a real t0 miss) = verilator `[0] … r2e_restore.sv:22`.
## Findings
- N1 NON-BLOCKING doc nit: manual 006 / CHANGELOG say "Only a bare `if` right after `else` continues the series.
  Anything else after `else` ends it", but `else (* m *) if` continues (lexer strips attributes; r2a t1 POST2 6:15 =
  verilator :6; pinned in unique_if_chain_shape.rs). Say attributes are transparent.
