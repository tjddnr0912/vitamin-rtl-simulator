# §4.5.585 plan — unique-if-chain (§5.2 row 1) — DECISION: START, narrowed to c2

HEAD 75f255d4. PRE S/s585/pre/vita md5 c766b8d59edabaef96436e7b46e75dc6 (frozen, never rebuild). POST-cand
S/s585/postc/vita md5 17b7d988696c0f5b38b7e94b845e2df9 (wt-s585, patch S/s585/postc/cand.patch).
New cells S/s585/p/*.sv (+ `_H.sv` = armed tree on PRE), outputs S/s585/g/out/<cell>.out (harness S/s585/g/r.py).
Counting scripts S/s585/p/residue.py, gain.py (option = set of body kinds whose chains keep the first-`if` arm;
POST-option predicted from POST-cand with those kinds' chain lines set back to PRE). Pin sources S/s585/p/PINS.txt.
Oracles: verilator 5.052 (`unique if`), iverilog 13 (only `unique case`; rejects `unique if`).

## Status
- [x] D1 transitive CA reach (61 new cells)
- [x] D2 options weighed, residue counted
- [x] D3 plan delta (below)

## Decision

START, with narrowing c2: a chain keeps PRE's first-`if`-only arm in EVERY function body except a `function void`
declared outside a class (which `parse_function_item` turns into a `TaskDef`). Armed: procedural code, every task
(module, interface, package, $unit, program, class), item `function void`, `fork` branches, `final`.
Record the CA t0 row as new §2 🆕 AB; it holds the function residue together with `unique-const-fn`.

- The brief's N1 (first-`if` in every non-void function, ctor armed) is NOT closed: a CA reaches a class void
  method through `this.` (t2t_c_fg_this, u7_c_fg_this_nowrite) — W4031 at t0, verilator and PRE silent.
- c1 (N1 + class void methods) is NOT closed either: a class function reaches a constructor through a member
  handle `d = new(x, z)` (u20_c_fnew_member, u21_c_fnew_this) — W4031 at t0 ×2, verilator and PRE silent.
- c2 is closed for every legal shape vita runs (census D1): each remaining route from a CA to an armed body is
  refused (E3009 / E3010 / E2002) or illegal (a function calling a task: both oracles refuse).
- (b) rejected: c2 needs no prerequisite (ER §10.2 row 1), ships 78 verilator-matching lines in 52 cells now,
  and leaves PRE behaviour (silent) in the 33 residue lines; (b) would hold every chain behind a scheduler change
  whose fix is not grounded. N2 (all functions, s583's original) is equally closed but leaves 11 more lines.

Deciding raw lines (t2t_c_fg_this: `assign y = obj.f(a, b)`, `C::f` calls `this.g(x, z)`, chain in void `C::g`):
- verilator: `[2] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated`
  (×2 at t2, t3, t4; nothing at t0)
- PRE: `t=1 y=1` / `t=3 y=0`, rc=0, no W4031
- POST-cand: `t2t_c_fg_this.sv:4:12: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for priority
  or unique case statement [in top.C.g] [at time 0]` ×2, then t2 ×3, t3 ×2, t4 ×2; PRE-H the same at 4:31
  (the root is pre-existing: PRE reports a lone `unique if` there).
u20_c_fnew_member (`C::f` does `d = new(x, z)`, chain in `D::new`): verilator `[2] … $unit.D.new: 'unique if'
statement violated` (t2–t4, none at t0); PRE silent; POST-cand `u20_c_fnew_member.sv:5:12: … [in top.D.new]
[at time 0]` ×2; PRE-H the same.
Root (k1_ca_display, k2b_ca_assert_clean): iverilog and verilator print `f t=0 x=0 z=1` once; PRE prints
`f t=0 x=x z=x` then `f t=0 x=0 z=1`. k2b: both oracles silent, rc 0; PRE `k2b_ca_assert_clean.sv:3:5:
error[VITA-E4003] E-RUN-USER-ERROR: Assertion failed [in top.u.f] [at time 0]`, rc=1.

## D1 — routes from a continuous assign to an armed body (PRE / PRE-H / POST-cand, verilator)

A CA calls only non-void functions. Under c2 those are unarmed; the question is what they reach.
| route | cells | vita (PRE = PRE-H = POST) |
|---|---|---|
| item non-void fn → item `function void` (static, automatic, output formal, no writes; module, $unit, package scoped / imported, interface local / hierarchical, generate, program, net decl, port expression; f→h→g) | t1s t1a t1o t8s t5s t5i t5a t6l t6h t7u t9g t10p t11w t12p u1s u1a u3 u3s u4 u5 u6 | E3009 (frame-call subset), all |
| item non-void fn → task (illegal: both tools refuse) | u2s u2a | E3009 |
| item fn → `fork t(); join_none` (the legal function→task route) | t13k | E3009 |
| item fn → class method through a handle (formal, module variable) | t4h t4o | E2002 / E3009 |
| class fn → class void method `this.g()` | t2t u7 | RUNS: t0 W4031 (opted out by c2) |
| class fn → class void method bare `g()` | t2b | E3010 |
| class fn → constructor via member `d = new(…)` / `this.d = new(…)` | u20 u21 | RUNS: t0 W4031 (opted out by c2) |
| class fn → constructor via local handle; reading `d.r` | t3c t3m u10 u11 | E3009 / E3010 |
| class fn → member handle's method `d.g()` | t4d | E3009 |
| class fn / ctor → $unit or package `function void` | v1w v1n v2w v2n v3w v3n u22 u24 | E3010 / E2002 |
| class fn / ctor → class task `this.t()` (illegal; verilator `%Error-FUNCTIMECTL … Functions cannot invoke tasks`, iverilog `Functions cannot enable/call tasks.`) | u8 u23 (u8_case: PRE already 0@3:12 ×2) | RUNS (no oracle; a pre-existing over-acceptance → PROBE_CATALOG line below) |
| out-of-block `extern` class method | w1 w2 | E2002 |
Other time-0 contexts (H cells on PRE vs oracles): `initial` before the writer (s1i), child `initial` both module
orders (s2 tbF/tbL), ctor from `initial` (s4): verilator reports at t0 too. Self-timed `always` before the
`initial` (s1s, s3): verilator silent at t0, iverilog reports at t0 on the `unique case` twins (s1s_case 0@5,
s3_case 0@5) = vita: a split. always_comb consumer-first quiet chain (aa1_chain): 🆕 AA (SPLIT).

## D2 — options (183 cells vita runs with a verilator run; overlap-only c23 and the forbidden-qualifier vl_lines
lines 11 / 17 excluded)
| option | first-`if` kinds | gained lines (cells) | residue lines (cells) | t0 W4031 verilator+PRE lack | closed |
|---|---|---|---|---|---|
| cand | item non-void | 95 (65) | 16 (15) | 9: CA row ×6 (n_ca_objf2 tbF/tbL, t2t, u7, u20, u21) + splits ×3 | no |
| N1 | + class non-void | 89 (60) | 22 (21) | 7: CA row ×4 + splits | no |
| c1 | + class void | 83 (57) | 28 (25) | 5: CA row ×2 (u20, u21) + splits | no |
| c2 | + constructor | 78 (52) | 33 (29) | 3: splits only (aa1_chain 🆕 AA; s1s, s3 vita = iverilog) | yes |
| N2 | + item void | 67 (43) | 44 (37) | 3: splits only | yes |
| (b) prerequisite | no slice | 0 | all | 0 | — |
c2 residue (33): fn ×16 b02_frame_lanes:7 dif_c05_long50:3 dif_c11b:4 dif_c11b:21 dif_c21:5 fold_P:5 gen_fn_rt
ifc_fn_rt l07f_P m_auto_rt m_static_rt n_ca_mf_tbF n_ca_mf_tbL pkg_fn_rt prog_fn_rt unit_fn_rt; class non-void ×6
cls_fn_rt n_ca_objf2_tbF/L n_comb_objf2_tbF/L snd_c3p:8; class void ×6 cls_vfn_rt dif_c11b:11 dif_c18:4 snd_c3p:4
t2t u7; ctor ×5 cls_ctor_rt dif_c21:12 s4_ctor_initial u20 u21. All are PRE-silent (no regression).
POST lines no verilator line names, same in every option: `priority if` chains (dif_c05:9, dif_c07:26, dif_c17:8,
f1c2:27, m_if_priority:7/9/15; hand-IEEE §12.4.2, verilator checks no `priority if`), an x/z condition (dif_c08:7,
hand-IEEE; verilator is 2-state), aa1_chain (🆕 AA).

Assumption for main to confirm: extending §2 🆕 AA (SPLIT) and the self-timed-`always` split to chains is not
blocking — every option arms procedural chains, 🆕 AA's remedy is ruled out (every order moves split cells), and
§4.5.584 re-opened this row with 🆕 AA recorded. 🆕 AB is different: both oracles agree in every measured order
(k1, q1caf tbF/tbL), so its reach must not grow, and c2 is the narrowing that keeps it from growing.

## D3 — plan delta against S/s583/PLAN.md

### Predicate (file:line in wt-s585 = POST-cand)
- crates/hdl-parser/src/functask.rs:263: `std::mem::replace(&mut self.in_const_fn_body, item && !is_void)` becomes
  `…, !(item && is_void))` — true for every function body except an item `function void`.
- Rename the field (lib.rs:884 doc :876-883, init :964; read at assertions.rs:501) to `first_if_arm_only`; rewrite
  its doc and the comments at functask.rs:81-84, :261-262 and assertions.rs:462-464 to state BOTH reasons: the
  constant-function interpreter refuses an armed tail (`unique-const-fn`), and a continuous assign runs a function
  at t0 on x before the driving `initial` (§2 🆕 AB) — a class function through a handle, a class void method
  through `this.`, a constructor through `new`. An item `function void` is armed because no non-void function can
  call one: the frame-call subset refuses it (E3009; pinned).
- Unchanged from POST-cand: `else_if_at` (lib.rs:891, stmt_ctl.rs:79-92), the walk (assertions.rs:491-520),
  classes.rs:285 `parse_function_def(false)`, module_items.rs:321 `parse_function_def(true)`. No hdl-ast change,
  no SchemaHash move, format_version unchanged; nothing under native/ or backend.rs (no flip run).

### Lane table (ER §10.2)
| lane | site | status |
|---|---|---|
| native tier-3 body | native/kernel.rs `k_dispatch_systask` | measured (Q1, Q4, D1; 3 backends every cell) |
| interp, vm | sched/kernel.rs:76 | measured (same cells) |
| automatic task frame | state/task_frames.rs | measured (b02 tchain) |
| inlined static / package / interface task | inline_task.rs | measured (lt_P, dif_c17, pkg_vfn3_imp_rt line 8) |
| item `function void` (TaskDef lanes) | task machinery | measured (m_void_rt, ifc_vfn_rt, pkg_vfn3_imp_rt, n_comb_vfn_one/tbF/tbL, n_latch_vfn_tbF/tbL, dif_c18:13, dif_c11b:17, snd_c3p:12) |
| class task, `fork` branch, `final` | class_lower, run_finals | measured (cls_task_rt, lt_P K.m, lt_P t4/t5) |
| every other function body (item non-void, every class function incl. `new`) | flag | opted out: PRE's tree (POST-cand = PRE on item non-void cells; POST-c2 = PRE on class cells to confirm, step 3) |
| staged vcmp→velab→vrun | artifact | measured on POST-cand (Q4 `POSTst 000 True`); re-measure POST-c2 |
| JIT (`--features jit`) | jit.rs `k_dispatch_systask` | NOT measured → step 1 |
| `--no-default-features` (product) | native only | NOT measured → step 1 |

### Implementation (opus-build xhigh; PRE frozen at S/s585/pre/vita, never rebuilt; H spelling = PRE's armed tree)
1. Apply the predicate (above) in wt-s585; build POST-c2 three ways, each in its own CARGO_TARGET_DIR: default
   release (→ S/s585/postc2/vita), `-p cli --features jit`, `-p cli --no-default-features`; build PRE (git archive
   75f255d4) with `--features jit` and with `--no-default-features` too. Measure the two unmeasured lanes:
   - JIT: J1 (S/s583/j/J1.sv; PRE on J1_H.sv) with `VITA_JIT_STATS=1 vita --backend native`; verilator reports t1 and
     t3 (S/s583/j/vl_J1.log). Confirm from the stats that the `always` body compiled; if not, add a bounded hot loop
     (`repeat (2000)` with `#1`); if no body holding the arm ever compiles, record the lane unreachable (measured).
   - product: m_if_unique, b02_frame_lanes, lt_P, m_void_rt, n_comb_vfn_tbL, dif_c01, dif_c02, cls_fn_rt,
     n_ca_objf2_tbL, u20_c_fnew_member on PRE-product (H spellings) and POST-product; equal to default POST-c2.
   Any lane difference stops the slice (narrow to opt-in or file the lane as a prerequisite, ER §10.2).
2. Field rename + comments (above). `cargo fmt --all`.
3. Re-measure POST-c2 on every cell (S/s585/g/c, c3, c4, c5, c6, c7, S/s585/p, S/s583/plan, S/s583/r1/*/cells) ×
   native/interp/vm + `--staged` (r.py with POST → postc2). Accept only: POST-c2 = PRE on every class-function
   chain line, POST-c2 = POST-cand elsewhere; residue.py (option c2) on the real outputs lists exactly the 33 lines
   above; the t0 list is exactly aa1_chain, s1s_selftimed_first, s3_child_selftimed_tbL.
4. Tests (step 4 table below), pins from PINS.txt, verilator text verbatim above each pin.
5. Docs (texts below) in the slice commit; ROADMAP texts below in the docs commit.
6. Mutation battery (`--workspace`, own CARGO_TARGET_DIR per mutant; ER §7.4):
   M1 walk removed → chain pins fail; M2 predicate `false` → l08_P E3009 and the opted-out class pins gain t0 W4031;
   M3 predicate `item && !is_void` (POST-cand) → n_ca_objf2_tbL / t2t / u20 pins fail; M4 predicate excludes
   `new` (c1) → u20 / cls_ctor_rt pins fail; M5 predicate `true` → item `function void` pins fail; M6 walk by node
   kind (drop the `else_if_at` test) → dif_c01 fails; M7 `written_if` without the qualifier look-ahead → b01 t1,
   vl_lines t3 / t5 fail; M8 flag not restored after a body → the "block after a function / class" shape pin fails.
7. Gates (main, after review): `cargo nextest run --workspace --locked` + `cargo test --doc --workspace --locked`,
   `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, corpus-runner
   0 failing with every row byte-identical (no `unique if` / `priority if` in any bench/*/src: grep count 0;
   ibex DIGEST 13b2ddfcd551ba2f).

### Tests
`crates/cli/tests/unique_if_chain.rs` (from S/s583/attempt/unique_if_chain.rs; every design on `--backend native`,
`interp`, `vm` with identical W4031 lines; one design also through `vita vcmp`/`velab`/`vrun`):
- module doc: replace "Inside a function body only a lone `if` is armed …" with the c2 rule and both reasons
  (`unique-const-fn`, §2 🆕 AB); keep the oracle paragraph.
- keep as attempted: m_if_unique, vl_lines, b01, timing-control enders, `priority if` hand-IEEE, unique0 /
  priority0 silent, lt_P (tasks, `fork`, `final`), b02 (tchain `b02:18:12 [in top.tchain]`; fchain silent — comment
  now BLOCKED on `unique-const-fn` and §2 🆕 AB), l08_P `P0=0 P1=1`.
- rewrite the b03 test: §2 🆕 Z is closed, so no t0 report: POST `b03.sv:6:12` at t2 and t3 (verilator `[2]`, `[3]`;
  its `[4]` runs after `$finish`). Drop the t0-residue pin.
- new `an_assert_or_attribute_after_else`: dif_c01 silent (verilator no assertion line), dif_c15 `5:15 t1`,
  `11:64 t4` ×2, dif_c02 `5:15 t1`, `7:26 t2` (verilator lines 5, 7), f1c2_qual full list (PINS.txt).
- new `void_functions_outside_a_class_report`: m_void_rt `4:12 [in top.fv] t1`, ifc_vfn_rt `4:12 [in top.i.fv] t1`,
  pkg_vfn3_imp_rt `4:12 [in top.fv] t1` + `8:12 [in top.t] t3`, n_comb_vfn_tbL t2 (no t0; verilator t2–t4).
- new `other_function_bodies_keep_the_first_if_rule`: cls_fn_rt, cls_vfn_rt, cls_ctor_rt silent; n_ca_objf2_tbL,
  t2t_c_fg_this, u20_c_fnew_member silent at every time (verilator t2–t4 quoted); comment: armed, each prints W4031
  at t0 through the continuous assign (their `_H` spellings on PRE do) — §2 🆕 AB; `unique-if-chain` BLOCKED.
- new `refusals_that_keep_a_void_function_unreachable` (code, location, rc=1 only — no message text): t1s_m_fg
  E3009 14:7, u1s_m_fg_nowrite E3009 13:7, t13k_m_fork_task E3009 14:7, v1n_unit_vg_from_cls E3010 6:5,
  u22_ctor_unit_vg E3010 8:5; comment: if one starts to run, re-decide `parse_function_def`'s predicate against
  §2 🆕 AB before re-pinning.
- new `time0_splits_reach_chains`: aa1_chain `5:12` t0 and t5 (verilator silent; iverilog on the `unique case` twin
  s583 a08: `Time: 5` only) — §2 🆕 AA; s1s_selftimed_first `5:12` t0 and t1 (verilator `[1]` only; iverilog on
  s1s_case `Time: 0`, `Time: 1`) — the self-timed split. Pinned so an order change moves them.
`crates/hdl-parser/tests/unique_if_chain_shape.rs` (from the attempt): task chain armed at its tail with the first
span; item `function void` chain armed (inside the `ModuleItem::Task`); item `function int`, class `function int`,
class `function void`, class `function new` chains keep the first-`if` shape; a lone `unique if` is armed in every
body; a procedural block after a function and after a class is armed (flag restored); `else assert (b) else if (c)`
is not entered; `else (* a *) if` is followed; `else unique0 if` is followed; `else unique if` stops the walk.

### Docs (slice commit)
manual 006 §1.4 — replace "On `if`, the arm is injected only when … (ROADMAP §3.b `unique-if-chain`)." with:
> On `if`, the arm goes on the last `if` of the `else if` series and reports at the first `if`, as Verilator does.
> An `else` followed by anything but `if` — a `begin … end` block, a labelled statement, a delay or event control,
> `;`, an immediate `assert` — ends the series. Inside a function body only a lone `if` is armed, except in a
> `function void` declared outside a class, so a chain in any other function reports nothing where Verilator
> reports (ROADMAP §3.b `unique-if-chain`): the constant-function interpreter would refuse a non-void function at
> elaboration (`VITA-E3009`) once the miss is reached (ROADMAP §3.b `unique-const-fn`), and a continuous assign
> that calls a function — and through a class handle, every class function and constructor that function calls —
> would run the armed miss at time 0 on `x`, where both tools are silent (§3.1, ROADMAP §2 🆕 AB). The message
> says `case statement` on `if` forms too; Verilator says `'unique if' statement violated` (ROADMAP §3.b
> `unique-if-text`).
manual 006 §3.1 — new row:
> | A function reached from a continuous assign whose inputs an `initial` writes at time 0: `assign y = f(a, b);`
> beside `initial begin a = 0; b = 1; … end` | `f` runs once more at time 0, before the `initial`, on `x`: a
> `$display` in it prints `x`, a `unique` / `priority` miss reports `VITA-W4031`, an immediate `assert` fails with
> `VITA-E4003` and the run exits 1 | both tools run `f` only after the `initial`'s writes: no time-0 line, exit 0 |
> Call `f` from an `always_comb`, whose time-0 pass waits for its inputs |
manual 003:763 — append to the notes: "On `if`, the report covers the whole `else if` series, outside the function
bodies Limitations §1.4 names."
CHANGELOG [Unreleased]:
> ### Fixed — a `unique if … else if` chain reports its no-match
> - `unique if (a) … else if (b) …` with no final `else` now reports `VITA-W4031` when no condition is true, at the
>   first `if`, as Verilator does; it reported nothing. An `else` followed by anything but `if` — a `begin … end`
>   block, a labelled statement, a delay or event control, `;`, an immediate `assert` — ends the series.
>   `priority if` chains report the same way (IEEE 1800-2017 §12.4.2); `unique0` / `priority0` chains stay silent.
>   This holds in module code, tasks (module, package and class), `function void` outside a class, `fork` branches
>   and `final` blocks. A chain in any other function body — a non-void function, or any class function, the
>   constructor included — still reports nothing (manual 006 §1.4).

### ROADMAP texts (docs commit)
§2 start-order rows, after 🆕 AA (new):
> - 🆕 AB — a function reached from a continuous assign runs once more at t0, on x, before the `initial` that writes
>   its inputs (k1: vita `f t=0 x=x z=x` then `x=0 z=1`; iverilog and verilator run it only after the writes): a
>   `unique` / `priority` miss in it reports W4031 at t0 (q1caf_case_tbF/L, t0f_ca_case, t0f_ca_if,
>   n_ca_objf2case_tbL ×4 through a class handle) and an immediate `assert` fails E4003 at t0, so a clean design
>   exits 1 (k2b), where both oracles are silent and exit 0; same family as manual 006 §3.1's `$random` re-drawn per
>   settle pass; holds `unique-if-chain`'s function residue (a CA reaches a class function through a handle, a class
>   void method through `this.`, a constructor through `new`: n_ca_objf2_tbL_H, t2t_c_fg_this_H,
>   u20_c_fnew_member_H); first: census where the t0 settle evaluates a continuous assign ahead of the first batch,
>   then measure an after-the-first-batch evaluation on k1, k2b, q1caf; 2 oracles; OPEN
PROBE_CATALOG (new line, §4.5.585 grounding): a class function or constructor calling a class task runs (u8_case,
u23_ctor_cls_task; a `unique` miss in the task reports at t0 through a continuous assign) where iverilog
(`Functions cannot enable/call tasks.`) and verilator (`%Error-FUNCTIMECTL … Functions cannot invoke tasks (IEEE
1800-2023 13.4)`) refuse the design; the module twin is refused (u2s_m_ft_nowrite, u2a: E3009).
§3.b `unique-if-chain` (replaces the row):
> - unique-if-chain — after §4.5.585 a chain in a function body other than a `function void` outside a class keeps
>   the first-`if` arm and is silent where verilator reports `'unique if' statement violated`: a non-void function
>   outside a class (m_static_rt, m_auto_rt, unit_fn_rt, ifc_fn_rt, prog_fn_rt, gen_fn_rt, b02 fchain, n_ca_mf_tbL;
>   BLOCKED on `unique-const-fn` and §2 🆕 AB) and every class function, the constructor included (cls_fn_rt,
>   cls_vfn_rt, cls_ctor_rt, snd_c3p lines 4 / 8, n_ca_objf2_tbL, t2t_c_fg_this, u20_c_fnew_member; BLOCKED on §2
>   🆕 AB); predicate `hdl-parser/src/functask.rs` `parse_function_def`; the refusals that keep a continuous assign
>   from reaching an armed `function void` are pinned (`unique_if_chain.rs`); verilator; BLOCKED
§2 🆕 AA: add `aa1_chain` (an `if` chain, §4.5.585) to its cell list.
§2 Oracle splits (new line):
> - a self-timed `always` (no header) written before the `initial` that drives it, at t0 — split (vita = iverilog:
>   source order, a `unique` miss on x reports at t0; verilator starts the `initial` first): s1s_case, s3_case,
>   s583 v04
§5.2: delete row 1; add 🆕 AB in the external-report block. Proposed by risk order (ER §10.2): 1 unique-overlap-note,
2 unique-const-fn, 3 §2 🆕 AB ("a function reached from a CA runs at t0 on x before the `initial`: W4031 / E4003,
exit 1 where both oracles are silent; first: census the t0 CA evaluation, then an after-first-batch evaluation on 2
oracles"; source "§4.5.585's fix path"; rank ①), rows 4–9 as now. Rank ① and its exit-1 effect argue for row 1:
main's call.

### Review targets (lenses get this table plus D1/D2; attack outside it)
- Closure: a route from a continuous assign (or a variable declaration initializer, a port expression, a net
  declaration assignment) to an armed body that D1 missed — any legal way a function reaches a task or an item
  `function void` (interface class, parameterized class, `let`, virtual interface, DPI export, checker).
- Predicate: `!(item && is_void)` at the only `FunctionDef` parse site; synthetic bodies (enums.rs:233) carry no
  `unique`; the flag restored on every exit of `parse_function_def` (parse error inside a body, missing
  `endfunction`, a class body after a method).
- Series: F1/F2 regressions (assert / assume after `else`, attributes, qualifiers, labelled / delay / event / `;`
  enders, dangling `else`, then-branch never entered, double report, condition evaluation count n=2 PRE = POST);
  `else_if_at` keyed on `span.lo` is sound because the preprocessor emits one expanded text per Parser.
- Inherited splits only: the t0 lines POST adds are exactly 🆕 AA's and the self-timed split's shapes.
- Loud regressions: l08_P runs; run.json routes PRE = POST; product build gains no refusal; staged and JIT parity.

wall_s: 1758 (planning session, 123 cell files in S/s585/p).
