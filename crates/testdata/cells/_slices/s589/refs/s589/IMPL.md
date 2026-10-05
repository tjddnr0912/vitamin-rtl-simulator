# §4.5.589 IMPL — package-routine "corrections only" cut (🆕 AD)

Status: COMPLETE through round 3 (post_c frozen). Follow-up not done (step 5 not clean, ibex elab +3.6..3.9%).
Worktree $S/s589/wt (branch s589 @ 303703f9), CARGO_TARGET_DIR=$S/s589/target_g.
PRE $S/s589/pre/vita e1e7e57148bb83ad35d2c4f68247a290 (never rebuilt). Harness: $S/s589/post_a/{runc,runs,cmpc,all}.sh;
cells copied to $S/s589/post_a/cells/{g_c..g_c7,a} (402 grounding + 72 audit = 474; grounding said 401, the dirs hold 402).
Comparison = byte-identity per cell after normalising the PID in an abort line (rc3_selfhdr_mod overflows the stack on PRE, PID varies run to run).

## Controls
- W0 = HEAD unchanged, built in this worktree (release) $S/s589/post_a/w0/vita md5 97f2c210…: 474/474 identical to PRE (path differences in the binary do not reach output).
- PRE rerun vs PRE: 474/474 identical (after PID normalisation).

## Step log
1. machinery (decl_scope.rs, fields, probe in walk_scopes_key_inner + const_fn_def, split in const_range_bound_fold; RtnPkgScope gains rtn/owned; no arming site)
   - S1 $S/s589/post_a/s1/vita md5 00358c0d…; staged $S/s589/post_a/s1/sep (vcmp 8d51ceb1…, velab 4dc94c12…, vrun 80451f6f…)
   - one-shot 474/474 identical to PRE; staged one-CU 474/474 identical to PRE staged.
   - `-p cli` nextest (debug): `Summary [23.367s] 8024 tests run: 8024 passed, 1 skipped` (rc 0)
2. kept `$pkg$<p>.*` bindings (decl_keep_snapshot/restore in decl_scope.rs; 2 call lines in package.rs)
   - S2 md5 1c796316…: one-shot 474/474 = PRE; staged 474/474 = PRE staged.
3. arming L1 (eval_const_call ret/formal ranges, default via decl_split, body cleared), L2 (bind_const_decl), L9 (4 typing sites + const_fn_call_width_safe)
   - S3 md5 6578a075…: vs PRE: wrong->ok 30 (all `*_ce_fn/_par` L1/L2, ty_*, cl4_scoped_nest, r1_flip_wide), wrong->wrong 5 (n_pkgimp, n_twolevel_g, imp_two_pkgs, pi_rpar: P fixed, v waits for L4; rv558_ovr), loud/ok moves 0.
   - a_ret_ce_ctl, ae2_loc_ce, id1_a41_pk, rba1_ce, rc1, rc2, rc4, cl1 = PRE byte-identical.
4. arming L4 (reserve func: return dims + formals/locals/block-locals; task: formals/locals/block-locals), L4b (emit_frame_call formal width), L5 (inline actual width, reduce_function_body formal/return/local + frame_packed_width, inline task formals); L8 rides L4
   - S4 md5 20931a5b…: +64 wrong->ok (all `*_rt_*` ret/fml/loc/frloc, w_*, in*_, pt_task*, pw_wide, pb_bits, b_bits2, n_*, imp_two_pkgs, pi_rpar, r1_flip_wide_rt); ac10-12, ac1, ac2, ac14, rba2, rba3 = PRE.
5. body lane (eval_const_call wrapper: Off + Idle + const_call_fn None + owned cur_rtn_pkg -> split)
   - S5 md5 d2fd35b7…: +14 wrong->ok (a/s/iimp/iwc/simp/swc cast/rep/psel_rt_fn); a_rep_rt_ctl, a_psel_rt_ctl = PRE; L7 `*_dflt_rt_*` unchanged.
6. re-entry guard (in since step 1): rc4 = PRE (`P=8`), no rc 134 other than rc3_selfhdr_mod (PRE's own module-routine overflow, identical on PRE).
7. re-measure on S5: 474 cells vs PRE: wrong->ok 108, wrong->wrong 1 (rv558_ovr `P=255 b=32` -> `P=15 b=32`, oracles `P=15 b=4`), loud->* 0, ok->* 0, split/noora 0.
   staged one-CU: one-shot == staged on 473/474 (rc3 crashes both ways), PRE same.
   st2 two-CU: PRE `top.v=232 c.w=40 c.g.P=5` -> S5 `top.v=8 c.w=40 c.g.P=5`; iverilog `c.w=40 top.v=8 c.g.P=3` (generate half opted out).
   sweep 5756 (PRE .pre from grounding, stable on rerun): 22 movers, all package-lane; 20 wrong->ok, 2 wrong->wrong (w05615 pkloc_p, w05639 pkfml_p: `P=1000 v=232` -> `P=1000 v=8`, oracles `P=8 v=8`; the `P` half is a call-holding local/formal width PRE never folds, 🆕 AC pk_ce_loc/fml).
   P1d corrections not taken (30 of P1d's 138 wrong->ok here): 16 `*_ctl/_pctl` openings, 5 `*_fnpar` (PRE's unit declines), 8 call-holding `*_fml/_frloc/_loc_ce_fn` ranges (item 6 dropped), rv561_pkc (item 7 out) — all by the corrections-only rule.

## Deviations (with reasons)
- D1 window state is a 3-state enum `DeclArm { Off, Cleared, On(Rc<DeclWin>) }` instead of `Option<Rc<DeclWin>>`: the body lane may arm only where NO routine text is armed (Off); a non-owned routine's text nested inside an owned body (a module routine reserved/inlined while a package body lowers) is Cleared and must not take the body window.
- D2 `decl_hdr` holds `Rc<DeclWin>` (pkg, rtn) rather than `usize` addresses: the same routine reaches the interpreter through `pkg_funcs` and through `const_func_table` clones (different addresses).
- D3 re-entry guard built into `decl_enter` at step 1 (inert without arming); PLAN step 6's check (rc4 `P=8`, no rc 134) is measured at every later step.
- D4 window cache `decl_wins: RefCell<BTreeMap<(pkg, rtn), Rc<DeclWin>>>` so the per-routine declared set is computed once (PLAN "cache declared per routine").

## Tests
crates/cli/tests/decl_scope_pkg_text.rs: 31 tests (19 correction pins incl. staged two-CU, 7 PRE-kept, 6 REFUSED with oracle text). `cargo nextest run -p cli --locked --no-fail-fast --test decl_scope_pkg_text`: 31 passed.
New killer cells $S/s589/post_a/cells/k: own8_imp_rba/_ce (M8; PRE = POST = 3 oracles `P=8 v=8`), m9_hdr_nest (PRE `P=235 v=235` -> `P=3 v=3` = 3 oracles), m9_body_imp (PRE `v=0000007f` -> `v=00000003` = 3 oracles), m9_body_cast (PRE `v=-21` -> `v=-1` = 3 oracles), m9_body_nest (scoped spelling: E3009 on PRE and POST, the scoped-call gate), m5_enum_lbl_ce/_rt (PRE = POST `P=232`/`v=232`; iverilog 8, verilator refuses ce / `v=0` rt, sv2v parse error — split, kept at PRE).

## Scoped gate (POST tree)
- `cargo fmt --all` then `cargo fmt --all --check`: rc 0
- `cargo nextest run -p cli --locked --no-fail-fast`: `Summary [24.221s] 8055 tests run: 8055 passed, 1 skipped`; TIMEOUT 0, SLOW 0
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: rc 0

## POST (frozen)
- $S/s589/post_a/vita md5 f13c0978e2ae4ae9ee26979a3e0e5bd6 (release, `cargo build -p cli --release --locked`)
- $S/s589/post_a/sep/ (`--features separate-bins`): vcmp cc532ea6…, velab 1790c2c2…, vrun 1ace9b89…, vita 18074dbe…
- POST one-shot == S5 on 474/474.

## Three-way (PRE vs POST vs oracles)
| set | silent->correct | loud->value | correct->anything | wrong->wrong | loud->loud | split/noora moves |
|---|---:|---:|---:|---:|---:|---:|
| 402 grounding + 72 audit | 108 (pred. ~118) | 0 | 0 | 1 (rv558_ovr) | 0 | 0 |
| 8 killer cells (k) | 3 | 0 | 0 | 0 | 0 | 0 |
| sweep 5756 | 20 | 0 | 0 | 2 (w05615, w05639) | 0 | 0 |
| staged one-CU 482 | one-shot == staged on 481/482 for PRE and for POST (rc3 crashes both) | | | | | |
| st2 two-CU | `top.v=232` -> `top.v=8` (iverilog 8) | | | | | |
Per lane (474+k): L1 18, L2 6, L4/L4b 40, L5 14, L6 15, L8 4, L9 4, L17 3 (+1 wrong->wrong), audit/k 7.

## Elab-time A/B (release, PRE e1e7e571 vs POST f13c0978)
corpus-runner `run --reps 1` (root = worktree, bench sources rsynced from the main checkout), binary swapped at wt/target/release/vita;
order W(discard) A B B A A B B A (A=PRE, B=POST); per-row `phase split` elab ms:
| row | PRE | POST | median POST/PRE |
|---|---|---|---|
| sha256 | 3 3 3 3 | 3 3 3 3 | 1.000 |
| aes | 8 7 7 7 | 7 7 7 7 | 1.000 |
| picorv32 | 28 28 28 28 | 28 29 28 28 | 1.000 |
| darkriscv | 3 3 3 3 | 3 3 3 3 | 1.000 |
| biriscv | 26 26 26 26 | 26 26 26 26 | 1.000 |
| serv | 12 10 10 10 | 10 10 10 10 | 1.000 |
| verilog-ethernet | 12 12 12 12 | 12 12 12 12 | 1.000 |
| ibex | 215 216 220 216 | 229 225 224 222 | 1.039 |
| keccak | 1 1 1 1 | 1 1 1 1 | 1.000 |
| keccak-arr | 1 1 1 1 | 1 1 1 1 | 1.000 |
ibex: POST > PRE in all 4 pairs (both orders) -> +3.9%, outside ±3%. Supplementary ibex elab-only (`--timeout 10 --obs-dir`, run.json elab_s), W + (A B B A)x8:
median PRE 0.2161 s, POST 0.2229 s (1.032); A->B pair median 1.025, B->A pair median 1.034. All other rows unchanged at 1 ms resolution.
Corpus grades every pass: all `ok`, verilog-axi `ruled-split` (pre-existing); no REGRESSION / DRIFTED / ORACLE-DRIFT / NON-DETERMINISTIC; rc 0 x9.

Attribution (ibex elab-only micro A/B, same protocol): null control PRE vs W0 (HEAD rebuilt here) 1.004 (A->B 1.006, B->A 1.002);
V1 = POST with an allocation-free window cache 1.030 (no gain); V3 = POST machinery with nothing armed 1.012 (A->B 1.010, B->A 1.012).
=> about 1/3 of the cost is the always-on checks, 2/3 the armed folds (the double fold of owned package routines' ranges). V1/V3 were experiments only, reverted (elaborate diff re-verified identical to the frozen POST).

## Mutants (debug build via the new test target; killer cells on target_g/debug/vita; tree restored after each, `git diff -- crates/elaborate` md5 1a45aa83… = frozen)
| M | mutation | test-file result | killer cell (POST -> mutant) |
|---|---|---|---|
| M1 | probe active in the PRE half | 9 failed | a_ret_ce_ctl E3009 -> `P=8` |
| M2 | window answer without `.or(pre)` | 1 failed (window_decline_keeps_pre_answer) | rba1_ce `P=8` -> E3009 |
| M3 | skip the PRE half | 8 failed | ae2_loc_ce E3009 -> `P=0000`; ac10_rep_case E3009 -> `w=00000000` |
| M4 | window writes const_call_pkg (proto) | 8 failed | cl1_unit_hdrcall `P=232 v=232` -> E3009 x2 |
| M5 | drop the declared-name exclusion | 1 failed (routine_declared_name_is_not_probed) | m5_enum_lbl_ce `P=232` -> `P=8` |
| M6 | no re-entry guard | 1 failed (header_reentry_keeps_pre_answer) | rc4_shadow_selfhdr `P=8` -> stack overflow rc 134 |
| M7 | no kept bindings | 1 failed (reserve_return_range_binds_package_constant) | a_ret_ce_par `P=8` -> `P=232` |
| M8 | owner = filing (importing) package | 1 failed (routine_imported_into_package_is_not_its_text) | own8_imp_rba `P=8 v=8` -> `P=40 v=40` |
| M9 | interpreter body not cleared | 2 failed (header/body_window_does_not_reach_callee_bodies) | m9_body_imp `v=00000003` -> `v=0000001f`; m9_hdr_nest `P=3 v=3` -> `P=43 v=43` |
| M10 | split applied when unarmed | first run: 31/31 passed, 482 cells = POST; killer found: m10_itask_fn `v=8` -> `v=232`; pinned (inline_task_local_range_binds_package_function), re-run: 1 failed | |
No survivor remains; no `--workspace` survivor run needed.

## Findings
- F1 elab cost: ibex +3.9% (corpus-runner, 4/4 pairs same sign), +3.2% elab-only; outside ±3%. Other rows unchanged at 1 ms resolution.
- F2 missed arming site (not in PLAN §3.2 or proto): inline (static) task LOCALS (`inline_task_locals.rs` `hoist_inline_task_locals`: declared_odd_bound / range_to_dims_opt / record_declared_bounds_for / array dims). m10_itask_par (`logic [W:0] t` in a static package task): PRE = POST `v=232`; iverilog, verilator, sv2v `v=8`. Silent-wrong kept at PRE (no descent).
- F3 the body lane reaches that same hoist for a CALL in the range (cur_rtn_pkg pushed, nothing armed): m10_itask_fn PRE `v=232` -> POST `v=8` = 3 oracles (silent->correct, a lane PLAN did not list).
- F4 M5's killer exists but the oracles split: m5_enum_lbl_ce iverilog `P=8`, verilator refuses (rt twin `v=0`), sv2v parse error; POST keeps PRE `P=232`.
- F5 sweep partial movers: w05615 pkloc_p / w05639 pkfml_p `P=1000 v=232` -> `P=1000 v=8` (oracles `P=8 v=8`): the `P` half is a call-holding local/formal width PRE never folds (item 6 dropped by PLAN).

## Additional deviations
- D5 reduce_function_body takes the window as a new parameter (`hdr_win`) instead of reading `cur_rtn_pkg.last()` (positive owner from the caller, PLAN §3.8).
- D6 L2 and the body lane do not push the re-entry stack (they are not header text); every header arming (L1, L4, L4b, L5, L9) does.
- D7 test file grew a 32nd pin (inline task local, M10 killer) after the scoped gate; re-run: test file 32/32, workspace clippy rc 0, fmt rc 0.

## Final
wt.diff $S/s589/post_a/wt.diff md5 6b25f2ffbff4d3c6214d932d753004cd (18 files, +1306 -27). `git add -N` was used on the two new files so the diff carries them (index intent-to-add only; nothing committed).

## Round-2 delta (review round 1: D1 = S1 BLOCKING, D2/S1m MAJOR, S3 MINOR; S2/F2 left as residue)
Fix: `decl_probe_fn` answers only for a function the window's package DECLARES (`pkg_own_rtns`); an imported callee
(filed in `pkg_funcs[p]` by `elaborate_package`'s import arm) misses and keeps PRE's resolution. No import-origin tag built.
Docs: decl_scope.rs module doc + `decl_probe_fn` doc + const_fn_def probe comment now state that only owned functions answer, and
that the window code never writes `const_call_pkg` but a probed callee runs its body under the probe's tag (its declaring package)
because `eval_const_call` writes every resolved callee's tag there.
Tests (42, was 32): `package_text_binds_its_own_import` -> `imported_callee_in_package_text_keeps_pre_residue` (PRE-kept, `P=232 v=232`,
oracles `P=8 v=8`); new pins: imported_callee_{header,bits,inline,body_lane,default}_runs_in_its_own_package (ifn_hdr_ce `P=255`,
ifn_bits_rt `b=8`, ifn_inl `v=255`, c14 `v=7`, c20 `L=3 v=9`), wildcard_imported_callee_header_runs_in_its_own_package (c04 `v=15 b=4`),
reentry_does_not_block_a_later_correction (k11 `P=8 Q=15`), body_lane_skips_an_imported_body (k12 `v=7`),
body_lane_skips_an_imported_body_over_an_owned_name (new k12e `v=7`, the M12 killer after the fix), kept_binding_carries_declared_width (k13 `v=31 b=5`).
Residue cell for the import-origin row: n_pkgimp ($S/s589/g/c2/n_pkgimp.sv): PRE = post_b `P=232 v=232`; iverilog, verilator `P=8 v=8` (post_a had `P=8 v=8`).
Not armed (residue for docs): S2 c23_task_blk (static package task block-local, PRE = post_b `v=255`, oracles `v=15`), F2 m10_itask_par (`v=232`, oracles `v=8`).

post_b: $S/s589/post_b/vita md5 bea240591388d3ef7c91a47c9c052ac6; sep/ vcmp 5a3ca597…, velab b0518560…, vrun 50bd4f3b…, vita 3988d046…
wt.diff $S/s589/post_b/wt.diff md5 aac91c75b291432bf44ae5c9a5077a58 (18 files, +1542 -27).
Gate: `cargo nextest run -p cli --locked --no-fail-fast` `8065 tests run: 8065 passed, 1 skipped` (41-test file; the 42nd pin added after:
test file 42/42); TIMEOUT 0 SLOW 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` rc 0 (re-run after the 42nd pin); `cargo fmt --all --check` rc 0.

| set | PRE->post_a | PRE->post_b | post_a->post_b |
|---|---|---|---|
| 474 grounding+audit | wrong->ok 108, wrong->wrong 1 | wrong->ok 107, wrong->wrong 1 (rv558_ovr) | ok->wrong 1 (n_pkgimp -> PRE value) |
| 10 killer cells (k) | wrong->ok 4 | wrong->ok 4 | none |
| 73 lens cells | ok->wrong 15, wrong->wrong 6, wrong->ok 8, split 1 | ok->wrong 0, wrong->ok 8, wrong->wrong 1 (k11: `P=8` no oracle kept, `Q` 255->15 = oracles), split 1 (c21: iverilog refuses, verilator/sv2v = post) | wrong->ok 15 (all D1/S1 cells back to PRE = oracles), wrong->wrong 5 (ifn_fml, ifn_dflt, ifn_loc, c05, c20 back to PRE's pre-existing values) |
| sweep 5756 | 22 movers (20 ok, 2 partial) | same 22 | 0 diffs |
| staged | one-shot == staged: lens 73/73 on PRE, post_a, post_b; main 483/484 (rc3 crashes both ways); st2 `top.v=8` |
Lens cells vs PRE after the fix: post_b differs from PRE only on the 8 corrections, k11 and c21; all other 63 byte-identical.
Mutants: M14 (drop the new ownership check) -> 7 failed (all imported_callee_* pins + the residue pin); ifn_hdr_ce `P=7`, c04 `v=1023 b=10`.
Soundness survivors re-run on post_b: M11 -> 1 failed (reentry_does_not_block_a_later_correction); M13 -> 1 failed (kept_binding_carries_declared_width);
M12 -> 0 failed with k12 alone (the ownership check masks it: k12's callee `kk` is imported, so it misses either way; k12b `{kk(K){1'b1}}` PRE = post_b = M12 `v=511`,
oracles `v=7` — the pre-existing c12 class); new k12e (p DECLARES `f`) M12 `v=511` vs post_b `v=7`; pinned; re-run -> 1 failed.
Elab: ibex elab-only micro A/B post_a vs post_b 1.006 (A->B 1.007, B->A 1.003) = unchanged; PRE vs post_b 1.036 (A->B 1.037, B->A 1.034).
corpus-runner `run --reps 1` on post_b: rc 0, all ok, verilog-axi ruled-split, ibex elab 0.222 s; no REGRESSION/DRIFTED/ORACLE-DRIFT.

## Round-3 delta (review round 2: R1 = S4 MAJOR, `pkg_own_rtns` mixed functions and tasks)
Fix at the root: `pkg_own_rtns` split into `pkg_own_funcs` / `pkg_own_tasks` (package.rs Func arm / Task arm), asked only through one
predicate `pkg_owns(pkg, rtn, RtnKind)` (decl_scope.rs; `RtnKind { Func, Task }`). Reader census and the set each takes:
- `decl_win` (header arming): kind of the routine whose text is armed — Func at const_fn.rs eval_const_call_at, const_fn_ret_wsign_in,
  decl_local_win, frames_reserve func reserve, emit_frame_call, inline_fn; Task at frames_reserve task reserve and inline_task formals.
  `DeclWin` carries the kind; its declared-name set is read from the matching table (`decl_names_of` no longer falls from pkg_funcs to pkg_tasks);
  the window cache keys `(pkg, rtn, kind)`; the re-entry check compares the kind too.
- `decl_probe_fn` (callee probe): Func.
- `push_rtn_pkg_scope` `owned` (body lane): kind passed by each push site — frames_body func Func / task Task, inline_fn Func, inline_task Task,
  `with_default_arg_scope` (new `kind` parameter: inline_fn, emit_frame_call, emit_frame_func_out_call Func; inline_task, frame task call Task).
- `pkg_fn_own` (pre-existing reader, the interpreter's running function): Func.
Doc: decl_probe_fn "Only a FUNCTION the package DECLARES answers (a task of that name does not make an imported function its own)" — true:
the probe asks `pkg_owns(.., Func)` and, when the package declares function f, `pkg_funcs[p][f]` is that declaration (import arm only fills an
absent entry, Func arm inserts over it).
Pins (44): declared_task_does_not_own_an_imported_function (task_vs_fn `v=255`) and ..._in_the_reserve (r2c1 `v=15 b=4`), oracle refusal text beside.
Mutant M16 (pkg_owns answers from the merged sets): 2 failed (both new pins).

post_c: $S/s589/post_c/vita md5 e31d955b047ad8bbca74dd6b563d2f6f; sep/ vcmp 41310e56…, velab d7697787…, vrun e1262980…, vita c36f4199…
wt.diff $S/s589/post_c/wt.diff md5 ab107c9ece519a49a5957e6660f87d99 (18 files, +1686 -45).
Gate: nextest -p cli `8068 tests run: 8068 passed, 1 skipped` (TIMEOUT 0, SLOW 0); workspace clippy rc 0; fmt --check rc 0.

| set | post_b -> post_c | PRE -> post_c |
|---|---|---|
| 484 main cells | 0 byte diffs | wrong->ok 111, wrong->wrong 1 (rv558_ovr), loud/correct moves 0 |
| 73 round-1 lens cells | 0 byte diffs | wrong->ok 8, wrong->wrong 1 (k11), split 1 (c21), ok->wrong 0 |
| 50 round-2 lens single-file cells | 2: task_vs_fn `v=7` -> `v=255`, r2c1 `v=1023 b=10` -> `v=15 b=4` (both byte-identical to PRE; all oracles refuse) | wrong->ok 6 (two_mod, nest_pq, fwd_rt, two_pkg_same, wc_redecl, two/ab), ok->wrong 0 |
| 3 two-file designs (cu, dup, sound two) | identical one-shot and staged | cu one-shot `v=31`->`v=255`, sa+sb `u=65535`->`u=255` (= oracles per the lenses); staged two-CU loud identical on all three binaries |
| sweep 5756 | 0 byte diffs | 22 movers (unchanged) |
| staged one-CU | main 483/484 (rc3); lens+r2 123/123 equal one-shot for PRE, post_b, post_c | |
Elab: ibex elab-only post_b vs post_c 0.996 (A->B 0.998, B->A 0.993) = unchanged.

## Round-3 follow-up (S5 MINOR, test-only)
Pin static_task_formal_range_binds_package_constant (r3c3_stask_fml: post_c `v=15` = iverilog, verilator, sv2v; PRE `v=255`). Test file 45/45.
M18 (inline_task.rs static-task formals pass RtnKind::Func): 1 failed (the new pin); source restored (cmp identical).
`cargo fmt --all --check` rc 0; workspace clippy rc 0. post_c not rebuilt: final diff differs from post_c/wt.diff only in crates/cli/tests/decl_scope_pkg_text.rs.
wt_final.diff $S/s589/post_c/wt_final.diff md5 aee03db9a33ca56bd449237f32141fed.
