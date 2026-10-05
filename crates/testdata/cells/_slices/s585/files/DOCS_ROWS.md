# §4.5.585 docs-commit line texts (ROADMAP / PROBE_CATALOG), round 2 — final rule

Rule shipped: chains are armed only in procedural code outside a subroutine; every function and task body keeps
the lone-`if` rule; only a bare `if` after `else` continues a series (IEEE 1800-2017 Syntax 12-2).
Cells: S/s585/g/w3/<grp>__<cell>/, outputs S/s585/g/out3/<grp>__<cell>.out (grp g = planner/grounding, s = r1/sound,
d = r1/diff); PRE c766b8d5…, POST2 = S/s585/post2/vita.

## ROADMAP §2 start-order rows — new, after 🆕 AA

- 🆕 AB — a function reached from a continuous assign runs once more at t0, on x, before the `initial` that writes its inputs (k1_ca_display: vita `f t=0 x=x z=x` then `f t=0 x=0 z=1`; iverilog and verilator print `f t=0 x=0 z=1` once): a `unique` / `priority` miss in it, or in anything it calls, reports W4031 at t0 (q1caf_case_tbF/L, t0f_ca_case, t0f_ca_if, n_ca_objf2case_tbL ×4 through a class handle, q6a_fn_vfn_noformal_case through a formal-less item `function void`) and an immediate `assert` fails E4003 at t0, so a clean design exits 1 (k2b_ca_assert_clean), where both oracles are silent at t0 and exit 0; `always_comb y = f(a, b);` is the workaround (docs_cells k1w, k2bw: vita = iverilog, exit 0); same family as manual 006 §3.1's `$random` re-drawn per settle pass; documented in manual 006 §3.1; holds `unique-if-chain`'s subroutine-body residue (armed, a CA reaches a class function through a handle, a class void method through `this.`, a constructor through `new`, a formal-less item `function void`, and a task through an accepted function→task call: n_ca_objf2_tbL_H ×4, t2t_c_fg_this_H ×2, u20_c_fnew_member_H ×2, q6a_fn_vfn_noformal_H, u8_c_ft_this_nowrite_H / u23_ctor_cls_task_H ×2 W4031 at t0 on PRE); first: census where the t0 settle evaluates a continuous assign ahead of the first batch, then measure an after-the-first-batch evaluation on k1, k2b, q1caf, q6a; 2 oracles; OPEN

## ROADMAP §2 🆕 AA — cell list gains

`aa1_chain` (an `if` chain, §4.5.585; vita W4031 at t0 and t5, verilator silent; iverilog on the `unique case` twin s583_a08 `Time: 5` only)

## ROADMAP §2 Oracle splits — new line

- a self-timed `always` (no header) written before the `initial` that drives it, at t0 — split (vita = iverilog: source order, a `unique` miss on x reports at t0; verilator starts the `initial` first): s1s_case (iverilog `Time: 0`, `Time: 1`), s3_case (`Time: 0`, `Time: 2`), s1s_selftimed_first, s3_child_selftimed_tbL (the `if` chains, §4.5.585), s583 v04

## ROADMAP §3.b `unique-if-chain` — replaces the row

- unique-if-chain — after §4.5.585 two residues stay silent where verilator reports `'unique if' statement violated`: (1) a chain in any function or task body (every subroutine body keeps the lone-`if` rule, `hdl-parser/src/functask.rs` `tf_body` sets `first_if_arm_only`; 111 lines in the §4.5.585 census by kind: task 44, item `function void` 30, item non-void function 17, class function 7, class void function 7, constructor 6 — e.g. lt_P lines 2 / 5 / 9, b02 fchain / tchain, m_void_rt, ifc_vfn_rt, pkg_vfn3_imp_rt, cls_task_rt, cls_fn_rt, cls_vfn_rt, cls_ctor_rt, n_ca_objf2_tbL, t2t_c_fg_this, u20_c_fnew_member, q6a_fn_vfn_noformal, q6t_pkg_scoped_const_chain); BLOCKED on §2 🆕 AB, `unique-const-fn` and `unique-pkg-closure`; (2) an outer series whose `else` is a qualified `if`: verilator continues it through `else unique0 if` and reports (vl_lines t3 / t5, b01 t1, f1c2_qual C2 / C7, q16_nested_ml t3, q16b_nested_ml2 t3, q23_prio_u0 t3, q5n_nested_spans t4), where IEEE 1800-2017 Syntax 12-2 makes the qualified `if` the final `else` statement (vita = PRE = IEEE; a split, not chased); also recorded: an inner `unique` / `priority if` after `else` reports at its own line where verilator names the outer line (vl_lines t2 / t4, q16_nested_ml, q16b_nested_ml2; PRE the same for a lone inner `if`); pins `crates/cli/tests/unique_if_chain.rs`; verilator; BLOCKED

## ROADMAP §3.b — new row (sound F2)

- unique-pkg-closure — the package-scoped call closure walk `elaborate/src/package.rs` `pkg_stmt_pure_orig` (fn at :143, catch-all `_ => false` at :183) treats the synthesized `$__vita_unique_violation` as impure: a `pk::f(…)` call whose callee reaches a lone `unique if` or a `unique case` with no `default` is refused E3009 `package-scoped call pk::f(...) reaches pk::g, whose body names something outside its own formals/locals …`, naming a statement the user did not write, where both oracles run (q6t_pkg_scoped_const_chain_H, q6t_pkg_scoped_const_chain_case: iverilog `Time: 0`, `Time: 2`; verilator `[0] … pk.g`); `import pk::*` and a bare call run (q6w_pkg_import_const_chain_H: W4031 t0 ×2, t2); PRE the same; keeps `unique-if-chain`'s package bodies unarmed; 2 oracles; OPEN

## ROADMAP §5.2

Delete row 1 (`unique-if-chain`). Add 🆕 AB in the external-report block:

| n | n | §2 🆕 AB — a function reached from a continuous assign runs at t0 on x before the `initial`: W4031 / E4003, exit 1 where both oracles are silent; first: census the t0 CA evaluation, then an after-first-batch evaluation on 2 oracles | §4.5.585's fix path | ① |

## PROBE_CATALOG — new line

| §4.5.585 grounding and review | A function that calls a task is accepted (IEEE 1800-2017 §13.4: a function shall not enable a task): a module `function void` calling a task runs (q18_fv_calls_task, q18p_fv_calls_task_plain: vita `t=1 y=2` / `t=3 y=0`, exit 0, where verilator 5.052 says `%Error-FUNCTIMECTL: q18_fv_calls_task.sv:9:5: Functions cannot invoke tasks (IEEE 1800-2023 13.4)` and iverilog 13.0 `q18p_fv_calls_task_plain.sv:9: error: Functions cannot enable/call tasks.`), and so does a class function or constructor calling a class task (u8_case, u8_c_ft_this_nowrite, u23_ctor_cls_task, q3p_paramcls: verilator `%Error-FUNCTIMECTL`, iverilog `Functions cannot enable/call tasks.`); a `unique` miss in such a task reports at t0 through a continuous assign (u8_case `0@3:12` ×2, q3p_paramcls_case ×2). A NON-void module function calling a task is refused (u2s_m_ft_nowrite, u2a_m_ft_nowrite_auto: E3009 13:7). PRE = POST on every cell | slice grounding (D1), differential lens O3, soundness lens q3p | — |
