# §4.5.590 implementation — §2 🆕 AB

status: FINAL = post_d (own-wave reverted, see "Round-4 outcome"); nothing committed

wt = s590/wt (branch s590 @ 303703f9), CARGO_TARGET_DIR = s590/target_g.
PRE = s590/pre/vita md5 e1e7e57148bb83ad35d2c4f68247a290 (staged s590/pre/sep).

## Step 1 — P6 probe gate
P6 = P5_final.patch + heap clause + downstream closure + certified-empty-deps exemption + copy-repair skip (both backends); default-on (no env).
binary s590/probe6/vita md5 698f7070c7eaeed0f56bb3cc93895ff1; patch s590/probe6/P6.patch md5 afb9a662a423248e51eb20f0a33dbafd.
Runs: s590/p6/p6_<grp>/ (124 grounding + 37 plan + plan/w, PRE x3 + P6 x3); s590/p6/out_new (6 new cells q1-q6 with verilator/iverilog/sv2v); s590/p6/full_pl* (plan cells re-run with fresh oracles: 3 plan c2 cells had been edited after the plan's run: p4, p5, p9); s590/p6/p5ref_* (P5 SIDE=1 on current plan cells).
Scanner: s590/p6/axscan.py (per-axis call/rep/evt/val three-way).

- gate (full nextest, P6): `cargo nextest run --workspace --locked --no-fail-fast` -> 9048 run: 9047 passed, 1 failed (unique_if_chain a_constructor_called_through_new_stays_silent_at_time_0: W4020 count left 1 right 2 = the predicted u20 re-pin), 15 skipped. log s590/gate_p6.log.
- x2b: P6 = iverilog exactly (f, V, NV; PV gone). x2c: PV gone (f, V, NV; PRE f x-run, f, V, NV). x2d: P6 = both oracles (PV gone).
- a4, a9, e1: PRE==POST all backends True. T3: f1 (exempt, constant args) keeps PRE's position `f1` before `i0 y1=01`; f2 (deps {a}, held) loses its x-run (`f2 t=0 x=x z=1`) and `i0 y2=x1` -> `xx`. Whole-cell byte identity is impossible for T3 under the plan's own shape (f2 is not exempt); the exempt CA's axis is byte-identical.
- per-axis scan: 0 cells with PRE=IV=VL on an axis that moved. PRE=VL->neither only on the documented race axis (i0 value: a6, a7, a8, T2a, e2; e2 event order B line = race). IV->VL: f1, b1p, v4, v6 (T1 disqualifier). 
- BACKEND SPLIT: only the pre-existing W4030 fallback note on c1_delay, c2_delay_i0, i1_static_cnt (PRE same); k2/k3/k4 PRE split closed in P6 (native = interp).
- aa (8 cells): PRE==POST True all.
- P6 vs P5 (fresh P5 SIDE=1 refs on the same cell files, s590/p6/cmp_p5p6.py): 19 cells differ, every one on an amendment: exemption a4, a9, e1, T3 (f1 half) -> PRE; heap clause m3 (t0 W4020 read-X gone), m4 (w i0 xx->zz), u20, u20b, u20d; closure n4e (v i0 xx->zz = IV), x2b/x2c/x2d (PV gone), x1 (P5 V+NV -> none = PRE events; i0 v=z = IV), e4 (i0 y xx->zz = IV), x3 (f t=0 count 3 -> 4; PRE 5, both oracles 1: md per-pass residue, one more pass from the second wave), q3 (copy-repair skip: c i0 zz->xx, race axis, IV zz / VL 00).
- corpus.py P6: 22/22 files byte-identical to g/corpus_pre (cmp). Held counts P6 (VITA_PROBE_COUNT): 0 in EVERY row (ethernet's 80 lfsr_mask CAs are certified empty-deps -> exempt; P5 a-side held 80). The corpus is vacuous for the hold path.
- corpus-runner in wt (cargo run -p corpus-runner --locked -- run --reps 1, wt/target/release/vita -> P6): rc=0, 10 ok + verilog-axi ruled-split (pinned); log s590/p6/crun.log.
- new cells q1 (fork join_none at t0), q2 (fork-join), q4 (exempt soundness: f() reading mem[0] is held, deps {mem}), q5 (same with unique case: W4031@0 gone, both oracles silent), q3/q6 copy-of-held wire/logic: call axes = both oracles' t0 count.
VERDICT step 1: PASS on every item except the literal whole-cell "T3 byte-identical": T3's exempt CA (f1) is byte-identical on its axis; its held CA (f2 reads a) moves as the plan's held class (x-run gone; t0 call axis = verilator; i0 y2 x1->xx). No new descent, no corpus move -> proceed.
- incident: an unquoted heredoc ran a backticked `cargo run -p corpus-runner -- run` in the MAIN checkout cwd (PID 5586, main's target/debug/corpus-runner); killed by PID. corpus-runner has no file writes. Main shows M docs/ROADMAP.md, docs/REMAINING_WORK.md mtime 08:04 = not from this run (parent/s589 work).

## Step 2 — production code
- new crates/sim-engine/src/sched/t0_hold.rs (T0Hold: build / active / held_now / begin_release / release_due / next_wave / finish / x_drop; reached_callees exhaustive walk; Scheduler::hold_t0_pass, settle_releasing_t0); levelize/call_deps.rs func_effect_free + body_has_systask + frame_nets (shared with func_read_deps); levelize/mod.rs export; sched/mod.rs field t0_hold + mod; scan_arm.rs (build, settle wrapper/inner, hoisted `holding`, per-pass hold step, md skip, delayed-schedule skip, copy-repair skip); run_loop.rs (begin_release at start/first batch); native/run.rs mirror (wrapper/inner, hold_t0_pass, settle_releasing_t0, md skip, copy-repair skip, begin_release, loop-top check_call_fatal). No env reads, no probe modes.
- hot loop: no per-CA hold test; one `holding` bool read per settle; held CAs removed from the pass before the per-CA loop (held delayed x-drive moved from in-loop position to the pass head: unobservable to CAs because every CA reading a held net is held by the closure).
- build: cargo build --release -p cli --locked OK (no warnings). binary s590/post_dev/vita md5 81a9dea8c79208f82ef1f62cde6da8cf.
- re-run of the full P6 cell set (172 cells x PRE/POST x 3 backends): POST == P6 on 172/172 (s590/p6/cmp_dev.py dev p6 -> 0 diffs).

## Step 3 — tests
- new crates/cli/tests/t0_call_cont_assign_hold.rs: 21 tests, 3 backends each (+ staged k1), oracle raw lines in doc comments. Cells: k1(+staged), k2b, q1caf tbF/tbL, d1, d2+d3, d4+j1+a11+a12+a10, x2b, e4, e7, k7_finish, q8 (= e6 without the counter: gate fn assert, both oracles PG t=5/15/25, PRE E4003@0 rc=1), q7 (held delayed i0 xx; replaces c2_delay_i0, whose f is pure so it is NOT held under a-side and cannot pin the held x-drive), n4e, k3 (3-backend stdout incl. end time), T1, a4+T3 (race), T2b (race, pure), q3 (race, copy of held), b1+s5 (waves), q5 (exemption soundness: f() reading mem[0] held), residue s4 + c1_delay + n_ca_objf2case_tbL.
- unique_if_chain.rs: u20 W4020 2 -> 1 with attribution (u20d twin: PRE both W4020 from the x-run ahead of `dut init`; POST one (write ignored) behind it, the pre-existing ignored member-handle write); 🆕 AB prose rewritten to POST's measured output (header, b02, n_ca_objf2_tbL, t2t, u20, q6a).
- cargo fmt --all; quick: nextest -p cli --test t0_call_cont_assign_hold --test unique_if_chain -> 41/41 pass.

## Rebase (coordinator message: §4.5.589 landed, main = cb2f7f1d)
- temp WIP commit on s590 -> `git rebase main` -> `git reset --soft main` + `git reset`: tree uncommitted again; `git diff` identical to the pre-rebase diff (index lines aside), new files byte-identical.
- PRE2: detached worktree s590/prewt @ cb2f7f1d, CARGO_TARGET_DIR s590/target_pre2 -> s590/pre2/vita + s590/pre2/sep (separate-bins). (building)
- test file gained q9 (closure-held delayed assign that native runs itself: native twin of the held x-drive).

## Step 7 — mutants (expected killers written before running; s590/mut/battery.py)
M1 seed empty -> k1,k2b,d1 | M2 every call held -> T2b | M3 no heap clause -> u20 W4020 | M4 no exemption -> a4/T3 | M5 no closure -> x2b (+n4e,e4) | M6 releasing at construction -> k1 | M7 one wave -> b1 | M8 no X-DROP -> n4e | M9 no held x-drive (engine) -> q7 | M9n native twin -> q9 | M10 no md skip (engine) -> s4 | M10n native -> s4 | M11 no delayed-schedule skip -> c1_delay | M12 no native fatal -> k3 + sim-engine frame_call cont_assign_originated_runaway_terminates | M13 hold off at initializer re-settle (engine) -> d1,d2/d3 | M13n native -> d1,d2/d3 | M14 release before first batch (engine) -> n_ca_objf2case_tbL/u20/k1 | M14n native | M15 copy repair evaluates held (engine) -> q3 | M15n native -> q3.
Narrow run per mutant: nextest -p cli --test t0_call_cont_assign_hold --test unique_if_chain (+ -p sim-engine --test frame_call for M12); survivors re-run --workspace.

## Step 4 — scoped gate (on the rebased tree)
- cargo nextest run -p cli --locked --no-fail-fast: 8090 run, 8090 passed, 1 skipped (rc 0). TIMEOUT 0, SLOW 0.
- cargo nextest run -p sim-engine --locked --no-fail-fast: 714 run, 714 passed, 14 skipped (rc 0).
- cargo clippy --workspace --all-targets --locked -- -D warnings: rc 0. cargo fmt --all -- --check: rc 0.

## Step 5 — freezes, flip, product, JIT, staged (PRE2 = main cb2f7f1d)
- PRE2 s590/pre2/vita md5 8684cc7c872b0af01f295797911c1998; sep: vcmp 8a0071061125402b62bfc9d4614a4809 velab f2d42caac90637bfaf610e1c3fb27c9d vita c555292fcf4a6f6269246689a1eed156 vrun 45bb94130a99361ddfe588b938e4beac. prewt worktree removed after the PRE2 flip run.
- POST s590/post_a/vita md5 fab0a3a1d8b5e8c90e967e89563bb825; sep: vcmp 40aee022e1a3da5d6d7c8333711c3ce3 velab 031afb5349aa424761fbbc0cde3c63ed vita 6bfa9a6d22f153ae49a38b3a23fa3bb1 vrun 15a80bdd23609d12f94489b6578844ee; nodef 2c48b6af3c1e88143d734e12a42656ec; jit 1cf307acd1157acc08ed29d6cd70bba3.
- final gate POST: nextest --workspace 9114 run, 9114 passed, 15 skipped; SLOW 0; doctests rc 0 (17 crates, 0 doctests).
- flip (Bytecode, both spellings, s590/flip.py): PRE2 9093 run / 9 failed; POST 9114 run / 9 failed; failure sets identical (backend_equiv the_default_backend_is_native, obs_wprog x5, obs x2, cont_assign_runtime_delay a_call_in_a_runtime_delay_falls_back_instead_of_panicking).
- final sweep s590/fin/f_* (175 cells: PRE2 x3, POST x3, staged sep PRE2/POST, nodef native, JIT x3): PRE2 == old PRE on 175/175; POST == pre-rebase POST on 175/175; JIT == POST 175/175; nodef == POST native except c1_delay, i1_static_cnt, c2_delay_i0, q7 (product-shape refusals "this build carries no other executor", same class as PRE's); staged runtime lines == one-shot except 12 cells refused at vcmp/velab (elaborate errors; staged PRE == staged POST and PRE == POST on all 12).

## Step 6 — three-way (PRE2 vs POST vs oracles), s590/fin/axscan_fin.txt
cells with oracle 167, moved 106; PRE=IV=VL axis moved: 0.
| axis | ->oracle | neither->neither | PRE=VL->neither | IV->VL | VL->IV |
| call | 31 | 17 | 0 | 0 | 0 |
| rep | 39 | 12 | 0 | 5 (f1, b1p, v4, v6x2) | 0 |
| evt | 2 (e4, x2d) | 1 (p3e) | 1 (e2) | 0 | 0 |
| val | 6 | 12 | 6 (a6 x2, a7, a8, T2a, e2) | 0 | 1 (e4) |
59 cells: every moved axis ->oracle. Every mover was predicted in GROUNDING/PLAN tables or the P6 analysis (POST == P6 on every cell); no unpredicted mover.
- corpus: corpus.py PRE2 vs POST 22/22 files byte-identical; PRE2 vs old PRE 22/22 identical; corpus-runner (wt, target/release/vita -> post_a/vita) rc 0: 10 ok + verilog-axi ruled-split. log s590/fin/crun_post.log.
- product/JIT CI axes: cargo clippy -p cli -p sim-engine --no-default-features -D warnings rc 0; nextest -p sim-engine --no-default-features --lib 178/178; cargo clippy -p sim-engine --features jit -D warnings rc 0.

## Step 7 — mutant results (narrow: nextest -p cli --test t0_call_cont_assign_hold --test unique_if_chain; M12 also -p sim-engine --test frame_call)
20/20 KILLED (s590/mut/results_narrow.txt; git status unchanged after battery: True). No survivor, so no --workspace confirmation run was needed.
M1 20 tests | M2 a_pure_call_is_not_held only | M3 unique_if_chain u20 only | M4 a_constant_argument_call_keeps_its_time_0_place only | M5 x2b, n4e, e4, q3 | M6 20 | M7 a_chain_of_held_assigns only | M8 a_release_landing_x_wakes_nothing only | M9 q7 test | M9n q7/q9 test | M10 s4 residue | M10n s4 residue | M11 c1_delay residue + q7 | M12 k3 only — sim-engine frame_call cont_assign_originated_runaway_terminates PASSED under M12 (expected-killer miss: that design's callee is effect-free, so under a-side the assign is not held and the fatal is caught as before; P2's failure was under a-call) | M13 d1, d2/d3 | M13n d1, d2/d3 | M14 18 | M14n 19 | M15 q3, e4 | M15n q3, e4.

## Step 8
- s590/post_a/wt.diff (git diff + the two new files as /dev/null diffs) md5 18844699c76d6b22a45ebc1bbbe982b8, 2179 lines. Release rebuild of the final tree = post_a/vita md5 fab0a3a1… (identical).

## Deviations / notes
- T3 whole-cell byte identity (step-1 wording) not met by construction; exempt half identical.
- c2_delay_i0 replaced by q7 for the held x-drive pin (c2's callee is pure: not held under a-side); q9 added as the native twin (closure-held delayed assign).
- e6 pinned as q8 (no counter).
- 5 native-twin mutants added to the 15 (M9n, M10n, M13n, M14n, M15n): 20 total.
- corpus holds 0 CAs under the final shape (ethernet's 80 lfsr_mask are exempt): corpus byte identity is vacuous for the hold.
- x3 (md + held downstream): f t=0 count P5 3 -> POST 4 (PRE 5, oracles 1), md per-pass residue.
- crates/hdl-parser/src/lib.rs:898 still carries the pre-slice 🆕 AB sentence (PLAN step 7 docs item); left untouched so the frozen POST matches the tree.
- new test file is 1171 lines (unique_if_chain.rs, the precedent, is ~1400).

## Round-2 delta (coordinator: r1 lenses diff F1/F2, sound F1-F3, MS1-MS3)
- F1 perf: T0Hold rebuilt — closure by worklist over per-net readers (was a whole-design fixpoint loop, O(N) passes on a reversed chain); waves by Kahn counting per net (unreleased held drivers per net; per held CA the count of read nets still waiting for another held driver), linear; held set + graph stored only when something is held; no call anywhere in the IR -> inactive without walking (no allocation).
- wave/closure identity probe (s590/probe_r2/vita md5 e11aca53…, temporary: post_a transitive preds + post_a fixpoint closure computed beside the new ones, assert_eq on every wave and on the held set): 215 cells (172 + q7-q9 + r1 diff d/e + r1 sound sd) + perf chain/arr/vec/wires_1000 and _4, x3 backends = 666 runs, 0 mismatches; full nextest --workspace with the probe: 9114/9114 passed, 0 PROBE lines. Probe removed (file restored from snapshot).
- round-2 battery expected killers (s590/mut/battery2.py, written before running): MS1 -> sd06 | MS2 (worklist: newly held not re-queued = one level) -> sd07/sd07r | MS3 -> sd08 | M1-M4 as before | M5 (closure off) -> x2b,n4e,e4,sd07 | M6 -> k1 | M7 (one wave) -> b1, r2k | K1 (rem==0 step off) -> sd07r, r2k | K2 (rem==1 step off) -> r2k | M8 -> n4e | M9 -> q7 | M9n -> q9 | M10/M10n -> s4 | M11 -> c1_delay | M12 (native post-settle check off) -> k3, possible survivor now that the pre-settle check exists | M16 (native pre-settle check off) -> sd01a | M13/M13n -> d1,d2/d3 | M14/M14n -> k1 | M15/M15n -> q3.
- incident (my error, caught): one release build ran from the Bash default cwd = the MAIN checkout into target_g (relative dep-info paths made the later worktree build look Fresh: binary md5 f42989f8… = main's code, x-run back). Detected by k1 on that binary; fixed by touching the worktree's changed sources and rebuilding (dep-info now lists t0_hold.rs). post_a/probe_r2 builds were all from the worktree (post_a re-verified earlier by rebuild md5).
- round-2 battery: 26/26 KILLED (s590/mut/results_narrow_r2.txt; git status unchanged). MS1 sd06 only; MS2 sd07/sd07r only; MS3 sd08 only; K1 sd07r; K2 r2k only; M7 b1 + r2k + sd07; M12 still killed by k3; M16 sd01a only. No survivor -> no --workspace run needed.
- battery restore uses shutil.copy2 (keeps the snapshot mtime): the mutated sim-engine sources were touched afterwards so the next debug build recompiles from disk (a restored file older than the mutant build would leave the last mutant in the debug units). Round 1: the next sim-engine debug build after its battery was the round-2 probe edit (whole-crate recompile), so no stale result was used.
- post_b (s590/post_b/vita md5 73c96dee…; sep vcmp f253f199 velab 487a1d79 vita ad40d0b3 vrun 82a8e91b; nodef 296e9af6; jit a129c89d): scoped -p cli 8095/8095, -p sim-engine 714/714, clippy 0, fmt 0; full nextest 9119/9119, doctests rc 0; flip 9119 run / 9 failed = PRE2 flip set; corpus.py 22/22 = PRE2; corpus-runner rc 0; sweep 217 cells: post_a->post_b delta = sd01a + sd01b native only (`f2 t=0 x=0` line gone, = interp/vm); JIT == POST 217/217; nodef differs only on the delayed-call product refusals (c1_delay, i1, c2_delay_i0, q7, lens d19); staged mismatches only on 18 elaborate-refused cells (staged PRE == staged POST, PRE == POST on all).
- post_b scaling (interleaved, s590/r2/perf/perf_table.txt) showed chain/wires still superlinear: chain 1k/4k/8k/16k POST/PRE2 = 1.50/3.92/7.22/13.74 (POST 0.018/0.115/0.386/1.41 s). Cause: every unreleased held assign stayed on the dirty worklist (hold_t0_pass re-dirtied it), so each of the N wave settles visited all pending ones. Fix: the hold step no longer re-dirties (the wave that releases an assign marks it dirty; a held uncertified assign is in ca_always anyway). Identity post_b vs fix (post_dev3 md5 = see post_c): 222 cells x 3 backends, stdout+stderr+rc+VCD bytes: 0 diffs. post_b is superseded (kept, not overwritten); the final freeze is post_c.

### Round-2 final (post_c = the tree as it stands)
- post_c s590/post_c/vita md5 2607d4ced62f799b233e901bd70bed3a (= post_dev3); sep vcmp b5fe0867 velab 8a99fbe1 vita b78e8b79 vrun 99019fa8; nodef 21789e11; jit 6c028fe3. wt.diff s590/post_c/wt.diff md5 eb92a05babc3594efb2ba12fc27f24d8 (7 files +327/-35; t0_hold.rs 470 lines, test file 1351 lines, 26 tests).
- gates on the final tree: nextest --workspace 9119/9119 (SLOW 0, TIMEOUT 0); doctests rc 0; clippy --workspace --all-targets -D warnings rc 0; fmt --check rc 0; product clippy rc 0 + nextest -p sim-engine --no-default-features --lib 178/178; jit clippy rc 0.
- flip (Bytecode, both spellings): 9119 run / 9 failed, set = PRE2 flip set.
- sweep 217 cells (172 + q7-q9 + r1 diff d/e + r1 sound sd + r2 sd07r/r2k) x PRE2 / post_c x3 / post_a x3 / staged / nodef / JIT: post_a->post_c delta = sd01a native + sd01b native only (`f2 t=0 x=0` gone; = interp/vm); post_b->post_c 0; PRE2 unchanged; JIT == POST 217/217; nodef only the 5 delayed-call product refusals; staged mismatch = the same 18 elaborate-refused cells (staged PRE == staged POST).
- corpus.py post_c 22/22 = PRE2; corpus-runner rc 0 (10 ok + verilog-axi ruled-split).
- round-3 battery (post_c lines, s590/mut/results_narrow_r3.txt): 26/26 KILLED, git status unchanged; sources touched afterwards.
- scaling (s590/r2/perf/perf_table_postc.txt; interleaved A,B,B,A,A,B,B,A after a discarded warm-up pair; release; A = PRE2, B = post_c; RSS MB max):
  chain 1k 1.03 (0.0121/0.0124 s) | 4k 1.04 | 8k 1.02 | 16k 1.06 (0.1056/0.1122 s), RSS 76/79
  wires 1k 1.02 | 4k 1.04 | 8k 1.05 | 16k 1.05 (0.0990/0.1037 s), RSS 74/76
  arr 1k 1.00 | 4k 1.00 | 8k/16k refused by both (generate-for unroll cap, E3009)
  vec 1k 0.99 | 4k 0.99 | 8k/16k refused by both
  vchain (sound lens vec chain) 1k 0.99 | 4k 0.98 | 8k/16k refused by both
  AB-order and BA-order ratios agree (no sign flip). Before (post_a, sound lens): chain_8000 47.7 s / 189 MB; post_b chain_16000 1.41 s (13.7x).
- residue recorded: per-NET wave granularity (d18 generate chain over `o[k]`: PRE2 22 calls, POST 18, both oracles 6; e02 `c[k+1] = f(c[k])`: PRE2 18, POST 9, iverilog 10, verilator 11) — module doc now states it exactly.

## Round-3 delta (coordinator: r2 sound F1-F4, diff doc findings; main 75f46453)
- rebased onto main 75f46453 (WIP commit / rebase / reset --soft + reset): diff identical (index lines aside), new files byte-identical. PRE3 = s590/pre3/vita (detached worktree s590/prewt3 @ 75f46453, target_pre2) md5 9b616218bc6893e44e6aeca46841e1eb; sep vcmp 11864a40 velab ae8c17ef vita 78439b42 vrun cf87d373.
- F1 lanes: inside the release's waves (`T0Hold::in_waves`, set after the release's first settle) the always-visited set is the held members of ca_always, the multi-driver resolution only the groups with a held member, `schedule_delayed_cas` only the held delayed assigns (both backends; lane lists built once by `T0Hold::index_lanes`). Identity vs post_c on 248 cells (172 + q7-q9 + r1/r2 lens cells + r2/cells + perf/cost shapes) x 3 backends incl. VCD bytes: 0 diffs.
- F1 held uncertified (HU) assigns, measured three ways against post_c on the same 248 cells:
  per wave (post_c semantics, a released HU assign is re-run in every later wave's passes): 0 diffs; r_chain stays quadratic (the evaluation count itself is quadratic).
  final wave (HU and everything waiting on them released together last): 8 cells differ (x3, u20b, u20d, d19, e01, r2a, r2b, r2h2); r2a gains `g t=0 x=z` (a call on a held net's default, iverilog/verilator never) = a descent -> rejected; r2h2 fixed (`f2 w1=0` only).
  own wave (HU re-run only in the wave that releases it): 7 cells differ (x3, u20b, u20d, d19, e01, r2a, r2b), every difference a removed repeat call on a settled value (u20d counter n=5 -> 3 at t=1; verilator n=1); r_chain linear (~PRE3).
  CHOSEN: per wave — the brief's criterion is identical values and call counts on every cell. own wave is the measured linear alternative (needs a coordinator decision on the 7-cell delta).
- round-4 battery expected killers (s590/mut/battery4.py, lines remapped to the r3 tree): as round 3, plus MK1 (build threshold `hn.rem > 0`) -> r2e pin, MKA (release processes only the first driven net) -> r2c pin. The wave-lane lists (held_always / md_held / held_delayed) are cost-only specialisations: replacing them by the full lanes is post_c's behaviour, measured output-identical on 248 cells x 3 backends (no mutant can kill an equivalence; cost pinned by the scaling table).
- F4 / nothing-held path: `settle_cont_assigns` (both backends) now tests `T0Hold::active` once per call and runs `settle_cont_assigns_inner::<false>` — the inner fixpoint monomorphised over `const HOLD`, so with nothing held no hold test remains in the pass or md loops; `schedule_delayed_cas` reads the flag once per call; `T0Hold` is boxed in `Scheduler` (one pointer in the hot struct instead of ~200 bytes).
- F2 pins added: r2c `an_assign_driving_two_nets_releases_the_readers_of_both` (both oracles' counts; order = race), r2e `a_held_self_reading_sole_driver_is_released_at_once` (no oracle on counts). Round-4 battery 28/28 KILLED incl. MK1 (r2e only) and MKA (r2c only); git status unchanged; sources touched after.
- doc claims reworded (t0_hold.rs module doc): first paragraph no longer says "never on the declared defaults"; the release paragraph lists the three exceptions (cycle of held assigns incl. its downstream — k01 `f6 t=0 v=z`; per-net granularity — e02 9 calls, iverilog 10, verilator 13; d18 18, oracles 6; declined callee's body reads not in the read set — r2h2 `f2 w1=z`, PRE2 also wrong) and the held-uncertified O(N²); the cost paragraph states exactly what is linear.
- residue recorded (not fixed): k06 — a `$fatal` latched by a continuous assign's call does not stop the rest of that time-0 settle (later waves and the fixpoint still run before the loop-top check ends the run); pre-existing class (PRE2 also evaluates further assigns after the latch inside one settle).
- post_d (s590/post_d/vita md5 c68477f5e3a54ca9ea1eedc513a45575 = dev2; sep vcmp aabb5764 velab 265ed9c4 vita 2f97756f vrun b90ec256; nodef 4c5b7cd8; jit 30fc88ea); wt.diff s590/post_d/wt.diff md5 ef230e1e6f3e84eab712717f1645c02e (7 files +393/-41; t0_hold.rs 563 lines; test file 1432 lines, 28 tests).
- gates (tree = post_d): -p cli 8121/8121; -p sim-engine 714/714; --workspace 9145 passed (1 LEAK: cli::sva_paren_property ibex_assert_macro_shape under load average ~49 from a concurrent agent; isolated re-run 5/5, no leak); doctests rc 0; clippy --workspace --all-targets rc 0; fmt rc 0; product clippy rc 0 + lib 178/178; jit clippy rc 0.
- flip: post_d 9145 run / 9 failed = PRE3 flip (9117 run / 9 failed; prewt3 + target_pre2) = PRE2 flip set.
- sweep 238 cells (172 + q7-q9 + r1 diff/sound + r2/cells + r2 sound newcells + r2 diff k-cells) x PRE3 / post_d x3 / post_c x3 / staged / nodef / JIT: post_c -> post_d delta 0 (all 3 backends); PRE3 == PRE2 on every cell with a PRE2 run; JIT == POST 238/238; nodef only the 5 delayed-call product refusals; staged mismatch only on 20 elaborate-refused cells (18 + r2h, r2i; staged PRE == staged POST, PRE == POST).
- corpus: corpus.py PRE3 vs post_d 22/22 files byte-identical (PRE3 == PRE2 22/22); corpus-runner rc 0 (10 ok + verilog-axi ruled-split).
- scaling, PRE3 (A) vs post_d (B), release, discarded warm-up pair then ABBAABBA, load gated < 3 at batch start (uptime recorded in s590/r4/perf_cost_mine.txt, perf_diffr2.txt):
  sound cost (interp | native): r_chain 250/500/1000 = 2.70/6.80/16.99 | 2.78/6.52/16.09 (per-wave HU kept: quadratic, identical call counts to post_c; own-wave would be ~1.0); sd_chain_1000_1000 0.27 | 0.32; sm_chain_1000_250 0.28 | 0.34 (lane fix; post_c 2.23/2.40, 1.80/1.41); s_chain_1000 0.98 | 1.05; u_chain 250/500/1000 1.03/1.00/1.01 | 1.02/0.97/0.98 (refused by both, E3009); pure_4000 1.00 | 1.01. Loads 2.95 -> 3.12.
  own shapes (native): chain 1k/4k/8k/16k 1.02/1.04/1.04/1.05; wires 1.03/1.04/1.05/1.07; arr/vec/vchain 1k,4k 0.98-1.00. Load 2.86 -> 3.27.
  diff r2 shapes (native, 1k/4k/8k/16k): arr 1.01/1.00/1.00/1.00; callchain 0.99/0.98/0.99/0.99; fanin_cat 1.01/1.00/1.00/1.01; fanin_vec 1.00/0.97/0.98/0.97; fanout 1.00/1.01/1.02/1.02; ichain 1.00/1.03/1.04/1.05; ring 1.02/1.03/1.03/1.03; ringtail 1.01/1.02/1.04/1.03; wires 1.03/1.03/1.05/1.05. Load 2.94 -> 3.52. AB and BA ratios agree everywhere (no sign flip).
- clk_200k (no call, nothing held), two 8-run ABBAABBA blocks each: interp 0.90/0.91, vm 0.90/0.91, native 1.02/1.03 (loads 2.94 -> 2.80); post_c vs post_d interp 0.87. Diagnostic variant with `#[inline(never)]` on the engine inner settle: = post_d (1.00), so the wrapper/inlining is not the cause; the remaining interp/vm difference vs PRE3 (faster) is unattributed (code layout suspected, not measured).
- corpus timing, PRE3 (A) vs post_d (B), native default, per row: discarded warm-up pair, then ABBAABBA wall (front end <= 1%), each row started at 1-min load < 3 (s590/r4/corpus_time.txt): sha256 0.999 | aes 0.999 | picorv32 1.001 | darkriscv 1.017 | biriscv 0.998 | serv 1.001 | verilog-axi 1.000 | verilog-ethernet 0.997 | ibex 1.002 | keccak 0.994 | keccak-arr 1.002; AB/BA order ratios within 0.991-1.019; stdout identical A/B on every run; loads 2.19-3.31.
- prewt3 removed. status: COMPLETE (round 3); nothing committed.

## Round-4 delta (coordinator decision: held uncertified assigns re-run only in their own wave)
- production: `T0Hold` keeps per-assign lane flags (held member of ca_always / held delayed / held multi-driver member -> its group) from `index_lanes`; `next_wave` fills the wave's lanes (its ca_always members, its groups, its delayed members); inside the waves both settle loops and `schedule_delayed_cas` visit only those. No env var. Build s590/r5/dev/vita md5 8c800916aaa11d72c15fbd5031167f33; identical to the r3 probe2 (VITA_PROBE_HU=ownwave) on 261 cells x 3 backends incl. VCD (0 diffs).
- movers vs post_d (261 cells x 3 backends; raw outputs s590/r5/movers/<cell>.txt; every mover native=interp=vm except the W4030 fallback note on d19/f06, as before): post_e only DELETES lines of post_d on every mover except f07/u20d, where the deleted repeat calls change a `$random` draw count / a counter (values below); no added call, no call on z/x (f07 post_e: 0 `x=x|z` lines).
  | cell | PRE3 | post_d | post_e | iverilog | verilator | (t0 calls)
  | x3 tristate (f) | 5 | 4 | 2 | 1 | 1 |
  | u20b (f) | 5 | 4 | 2 | (aborts) | 1 |
  | u20d (f) | 5 | 4 | 2 | (aborts) | 1 | printed counter t=1 n: PRE3 4, post_d 5, post_e 3, verilator 1
  | d19 delay chain (f) | 11 | 8 | 6 | 1 | 3 |
  | e01 tri md (f) | 7 | 5 | 3 | 1 | 1 |
  | r2a (f; g) | 14; 3 | 8; 1 | 4; 1 | 2; 1 | 1; 1 |
  | r2b (f; g) | 7; 3 | 4; 1 | 2; 1 | 1; 1 | 1; 1 |
  | f06 held delayed (f) | 7 | 5 | 2 | 1 | 3 |
  | f07 $random chain (g) | 30 | 42 | 12 | 6 | 6 | printed rnd: PRE3 -887079274, post_d -445445430, post_e 1924134885, iverilog -1295874971 (= vita's 7th draw), verilator -1959092748
  | r3e held lanes vcd (f) | 17 | 11 | 5 | 3 | 2 |
- new pin r5h `an_uncertified_held_chain_runs_each_link_in_its_own_wave_only` (4-link reverse-declared $random chain: post_e 8 calls at t0, post_d 20, PRE3 28, both oracles 4); no existing pin moved (t0_call_cont_assign_hold + unique_if_chain 48/48 before the new pin).
- round-5 battery (lines remapped; + MOW: wave lane = every released held always-member, i.e. post_d per-wave) 29/29 KILLED; MOW only by the r5h pin; git status unchanged; sources touched after.
- module doc: the held-uncertified paragraph and the cost paragraph rewritten (own-wave visit; f07 numbers 42 -> 12, PRE3 30, both oracles 6; the release is linear in the held graph times the passes of a wave's fixpoint).
- post_e (s590/post_e/vita md5 e776d356844bcd7d5d1bd9079b6756f4; sep vcmp 67b6e634 velab 497e482c vita e8bb66e6 vrun 34de508c; nodef 222fe5c0; jit f0aeb0ba); = r5 dev on 261 cells x 3 backends (0 diffs). wt.diff s590/post_e/wt.diff md5 b6bf2f5cea1acbc6d083029e3380c3b2 (7 files +396/-41; t0_hold.rs 599 lines; test file 1474 lines, 29 tests).
- gates (tree = post_e): -p cli 8122/8122; -p sim-engine 714/714; --workspace 9146/9146 (SLOW/TIMEOUT/LEAK 0); doctests rc 0; clippy --workspace --all-targets rc 0; fmt rc 0; product clippy rc 0 + lib 178/178; jit clippy rc 0.
- flip: post_e 9146 run / 9 failed = PRE3 flip set.
- sweep 252 cells x PRE3 / post_e x3 / post_d x3 / staged / nodef / JIT: post_d -> post_e delta = exactly the 10 movers above + the new r5h cell, all 3 backends; POST backend split only the W4030 note cells; JIT == POST 252/252; nodef only the 6 delayed-call product refusals (+ f06); staged mismatch only the 20 elaborate-refused cells.
- corpus: corpus.py post_e vs PRE3 22/22 byte-identical; corpus-runner rc 0 (10 ok + verilog-axi ruled-split).
- HU scaling, PRE3 (A) vs post_e (B), release, discarded warm-up pair then ABBAABBA (s590/r6/perf_hu_poste.txt): native r_chain 1k/2k/4k/8k 0.98/0.97/0.97/0.98, hu_urandom 1.00/0.98/0.98/0.98 (load 2.63 -> 2.66); interp r_chain 0.98/0.96/0.97/0.97, hu_urandom 1.00/0.97/0.98/0.97 (load 2.66 -> 2.66); RSS within 2 MB; AB/BA agree.
  Reference post_d (native; a batch first launched by mistake against post_d through a half-applied sed, started at load 5.64, interp half killed; s590/r6/perf_hu_WRONG_postd.txt): r_chain 15.3/38.9/91.4/198.4x, hu_urandom 14.4/35.9/81.0x.
- status: COMPLETE through round 4 (post_e); nothing committed.

## Round-4 outcome (coordinator decision per D8: own-wave reverted; ship exactly post_d)
- own-wave attempt (post_e, md5 e776d356, kept frozen, not shipped): f07 `$random` 6-link chain t0 calls 42 -> 12 (PRE3 30, both oracles 6); r_chain / hu_urandom 1k-8k = PRE3 (0.96-1.00, was 15-198x on post_d); 10 movers, deletions only (table in "Round-4 delta").
- BLOCKING (r4 soundness, s590/r4/sound/REPORT.md): a held uncertified assign whose DECLINED callee's body reads a net a later wave writes re-ran only in its own wave, so it stayed stale past t0 (X-DROP: nothing wakes):
  r4b final: POSTE `final w1=0 w2=x`; PRE3/POSTD `w2=0`; iverilog `w2=x`, verilator `w2=0`.
  r4a: POSTE `W2 t=1 w2=0`; PRE3/POSTD/verilator `W2 t=0 w2=0`; iverilog no W2 event.
  r4c (held delayed): POSTE `D t=2 d=0`; PRE3/POSTD `D t=1 d=0`; verilator `D t=0 d=0`; iverilog no D event.
  r4d (multi-driver member): POSTE `N t=3 n=0`; PRE3/POSTD/verilator `N t=0 n=0`; iverilog `N t=0 n=x`.
  r4g (stale value feeding a held reader): POSTE `g t=0 x=x`, `g t=3 x=0`; POSTD `g t=0 x=x`, `g t=0 x=0`; verilator `g t=0 x=0`; iverilog `g t=0 x=x` only.
  Every POSTE event at t>0 is in neither oracle. Controls r4e/r4f POSTE == POSTD. Also MINOR: MWM (multi-driver lane accumulating) survived 9146/9146.
- queue-row fix shape: own-wave only for held uncertified assigns whose read set is complete (no declined callee anywhere in their calls); per-wave otherwise. Prerequisite: a declined callee's body reads entering `ca_deps`' read set (r2 F3; r2h2 `f2 t=0 x=1 w1=z` then `w1=0`, both oracles once on `0`, PRE2 also wrong).
- residues kept in the shipped post_d state:
  held uncertified chains are O(N^2) during the release (post_d native: r_chain 1k/2k/4k/8k 15.3/38.9/91.4/198.4x PRE3, hu_urandom 1k/2k/4k 14.4/35.9/81.0x; sound r3 r_chain_1000 17x);
  f07 call count PRE3 30, post_d 42, both oracles 6 (t0 `g` calls); the next printed `$random` moves with the draw count (PRE3 -887079274, post_d -445445430, iverilog -1295874971);
  k06 (a `$fatal` latched by an assign's call does not stop the rest of that time-0 settle; PRE3 too);
  d18 / e02 per-net wave granularity (d18 18 calls, oracles 6; e02 9, iverilog 10, verilator 13);
  cycle-fed chains released with the cycle (k01 `f6 t=0 v=z`);
  repeat calls on settled values in x3 (4; oracles 1), d19 (8; iverilog 1, verilator 3), e01 (5; oracles 1) — and u20b/u20d/r2a/r2b/f06/r3e per the Round-4 delta table.
- restore: `git apply -R s590/post_e/wt.diff` (tree clean) then `git apply s590/post_d/wt.diff`; diff + new-file diffs md5 ef230e1e6f3e84eab712717f1645c02e == post_d/wt.diff (byte-identical incl. index lines); release rebuild md5 c68477f5e3a54ca9ea1eedc513a45575 == post_d/vita. Module doc: no own-wave text (post_d's).
- restored tree gates: -p cli 8121/8121, -p sim-engine 714/714 (FAIL/TIMEOUT/SLOW/LEAK 0); clippy --workspace --all-targets -D warnings rc 0; fmt --check rc 0. s590/post_d/wt_final.diff (plain `git diff`, new files untracked so not included; post_d/wt.diff carries them) md5 dca2dbd4b7c482d84ac58b6a7496026f. status: SHIPPABLE STATE = post_d; nothing committed.
