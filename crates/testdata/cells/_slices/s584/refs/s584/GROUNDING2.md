# GROUNDING2 s584 — round 2 (after S0 stop: R4f loses one CA/port hop for a batch-2 reader)

PRE: S/pre/vita md5=2492e1c5c52c7081a6c0997fe194af40 (frozen). HEAD 63b9dbb9.

## Q1 mechanism in code + t>0 analog — DONE
Code (HEAD 63b9dbb9):
- Before t0, in order: `settle_t0` (lib.rs:1132 -> sched/scan_arm.rs:735-740 -> `settle_cont_assigns` :752) evaluates every continuous
  assign, so a constant CA (`assign w = 2'd1`) and a constant port tie (a port binding is a CA) land here and their nets go on `st.dirty`;
  then `arm_processes_after_seed`: declaration initializers run as bodies (scan_arm.rs:1208-1228), the initializer re-settle (:1263-1275),
  copy-net repair (:1288-1298), rollback of the initializers' dirt with `last_change_seq = 0` (:1305-1320: an initializer is no event),
  t0 edge rebuild / edge-clear / x-drop (:1321-1421). What stays on `st.dirty` is the "t0 settle record" (constant CA / port-tie nets).
- Seeding (scan_arm.rs:1424-1453): one seq for all seeds; Initial|Comb|Latch -> `cur.active` (batch 1); Edge|Level -> `arm_sensitivity(aid, 0)`
  (static waiter, arm_seq 0 so the settle record reaches it, :1571-1574 doc).
- `take_t0_wakes` (sched/run_loop.rs:25-40, doc :6-24; native twin native/run.rs:1200-1215, called :358): runs `propagate_changes` on the
  settle record with `cur.active` swapped out, so ONLY waiters armed at seeding receive it (in PRE: Edge/Level; a Comb/Latch is unarmed
  until its first Return, propagate.rs:661-672 `rearm`). If batch 1 is non-empty the woken list is held and prepended to the next batch
  taken (run_loop.rs:84-95; native :396-405). Why held (doc :8-22): a wait armed inside batch 1 must not see the settle event
  (`initial begin @(negedge w)...` on a settled w waits, both oracles), and both oracles print the initial bodies first, then the settle's
  wakes, then what batch 1's writes woke.
- CA settle only between batches: loop-top `settle_cont_assigns` + `propagate_changes` (run_loop.rs:66-75; native :382-388) and
  `propagate_changes` after the batch (run_loop.rs:193-196). Inside a batch bodies run back-to-back (:98-187) with no settle, so a body
  reading a CA/port net whose source an earlier body of the SAME batch wrote reads the pre-settle value. Batch order = `push_sorted` by
  (seq, tie); all wakes of one delta share `wake_seq` (sched/mod.rs:40-48, propagate.rs:56 `gseq`), so same-delta wakes run in
  declaration (tie) order, not event order.
- R4f x1/x2: the comb (armed at seeding) gets the constant CA / port-tie settle wake, held to the FRONT of batch 2; the reader woken by
  batch 1 (`initial t = 1`) is in the SAME batch 2; the comb writes y, the reader runs next with no settle between -> v / o stale (x).
  PRE escaped: the comb ran in batch 1, so the loop-top settle before batch 2 carried y through the CA.
- Known class: ROADMAP §2 line 303 "A CA hop lands after the whole batch; yield settle + nonblocking UDP (§4.5.539); BLOCKED (wake-group
  order has no oracle)". §4.5.539 (b4bdf60e) measured the yield settle: 16/19 cells = both oracles, moved picorv32 + serv digests
  (row-7 same-time resume order), reverted.

t>0 analog (S/c7/*.sv, raw .out; gen S/c7/gen.py): at #1 an initial writes the comb input `a` and the reader trigger `t`; comb and reader
are woken in ONE batch; reader prints `v` (= `assign v = y`) / parent `o` (child output port). vita native=interp=vm identical.
| cell | iv | vl | PRE | class |
|---|---|---|---|---|
| t1_ca_combfirst_afirst (always_comb; comb declared first; a written first) | r t=1 v=11 y=11 | r t=1 v=11 y=11 | r t=1 v=00 y=11 | PRE GAP (2-oracle): the same hop lost at t>0 |
| t1_ca_combfirst_tfirst | v=00 y=00 (reader first) | v=11 y=11 | v=00 y=11 | split; PRE matches neither |
| t1_ca_rdfirst_afirst (reader declared first) | v=11 y=11 | v=11 y=11 | v=00 y=00 | PRE GAP (2-oracle): tie order puts the reader first; iv runs in wake order |
| t1_ca_rdfirst_tfirst | v=00 y=00 | v=11 y=11 | v=00 y=00 | split (PRE = iv) |
| t1s_* (`always @*` in place of always_comb; y never ran at t0) | afirst: v=11 y=11 / tfirst: xx xx | 11 11 (all) | combfirst: v=xx y=11; rdfirst: xx xx | same pattern: combfirst_afirst and rdfirst_afirst PRE GAP |
| t2_port_instfirst_afirst (child always_comb o=~a) | r t=1 o=0 | r t=1 o=0 | r t=1 o=1 | PRE GAP (2-oracle): port hop lost at t>0 |
| t2_port_instfirst_tfirst | o=1 | o=0 | o=1 | split (PRE = iv) |
| t2_port_rdfirst_afirst | o=0 | o=0 | o=1 | PRE GAP (2-oracle) |
| t2_port_rdfirst_tfirst | o=1 | o=0 | o=1 | split (PRE = iv) |
| t3_cawoken_{combfirst,rdfirst} (comb woken through `assign w={a,a}`, reader one delta later) | r t=1 v=11 | `r t=0 v=00` (vl fires @(t2) at t0, 2-state) + no t=1 line | r t=1 v=11 | iv = PRE; vl no oracle |
=> PRE already loses the hop at t>0 whenever the comb and the reader share a batch and the comb's trigger is written first (iv = vl there);
the oracles split when the reader's trigger is written first (iv runs the reader first, vl settles). x1/x2 under R4f are the t0 instance
of the recorded §2 line-303 class, exposed because R4f puts the settle-woken comb into the reader's batch.
## Q2 candidate rules scored on the full cell set — DONE
Prototype (throwaway): worktree S/proto (branch proto/s584 off 63b9dbb9, never committed), CARGO_TARGET_DIR=S/tgt-proto, debug.
Rule chosen at run time by env `VITA_T0RULE` (unset = PRE code path). When set, elaborate retags the self-timed / inert `always`
(events.rs:375-393) as Initial (option b). Scheduler (interp + vm) carries every rule; native tier-3 mirrors rules R5hp..R5hpss
(7..12). Diff: S/proto.patch. Runner S/prun.py (outputs S/r2/<rule>/, S/r3/<rule>/ = native,interp,vm); scorer S/score3.py,
split classifier S/split3.py, dump S/dump3.py.
Sanity: prototype with VITA_T0RULE unset == PRE binary on 187/187 cells (byte-equal vita block).
Backends: interp = vm on every rule and cell; native = interp = vm on R5hp R7p R5hps R7ps R5hpss R7pss (all 187 cells).

Cell set (187): c3/orig 121 (= c1 + c1t + c2 + 21 s583 incl. a08/a00t/a15), c4 3, c6 26, c7 14 (t>0 analogs, controls), c8 11
(#0 readers through a CA, Level-only analogs x7*, x9, x10, x11, x12), c9 9 (hierarchy / Level-feeds-comb orders), c10 3 (two
settle-woken combs chained through a CA / port). New cells: raw .out next to each .sv (iv, vl, PRE).
Scoring (as GROUNDING Q3, plus values): rep2 = t0 report presence where iv and vl both ran and agree; val2 = cells whose display
lines iv == vl exactly; vl-only = iv cannot compile (`unique if`, constant latch); descent = PRE == oracle(s), rule != oracle(s)
(report, values or exit class). Split cells (iv lines != vl lines): position of PRE / rule = iv | vl | iv~order | iv~count | neither.

Rules (all: Comb|Latch = always_comb / always_latch / comb-UDP after the retag):
- R4f  armed at seeding; implicit run -> Inactive (head of the first #0 promotion); held t0-settle wakes at the front of batch 2 (PRE).
- R5h  R4f + the whole held settle group (Level/Edge/Comb) runs as its own batch, then settle, then the rest of batch 2.
- R7   R4f + only the Comb/Latch part of the held group runs as its own batch + settle (Level/Edge part keeps the front of batch 2).
- R5   unarmed; every implicit run in one batch after batch 1 + settle, then settle, then batch 2. R5i: + comb-only sub-batches to a
       fixpoint. R5s: R5 one comb per batch (settle after each).
- R5hp / R7p  R5h / R7 + the implicit runs at the first promotion run as their own batch + settle before the promoted #0 resumes.
- R5hps / R7ps  + those implicit runs one per batch (settle after each). R5hpss / R7pss  + the held settle-woken comb group one
       per batch too.

| rule | rep2 fixed /50 | val2 fixed /13 | vl-only /6 | rc fixed | descents | split moves (away from PRE's oracle) | rep2 still wrong |
|---|---|---|---|---|---|---|---|
| R4f | 43 | 1 | 6 | 1 | 5 val: u1b x1 x2 x11 x12 | 39 (12) | chain3_A_Bc_Bb chaind_A_B a08 rh x13 x13b x14 |
| R5h | 43 | 5 | 6 | 1 | 0 | 40 (11) | same 7 |
| R7 | 43 | 2 | 6 | 1 | 0 | 39 (11) | same 7 |
| R5 | 35 | 3 | 6 | 1 | 0 | 24 (4) | + q_hchain rh2 rh4 rh4b lv1 lv1b lv2 p2 |
| R5i | 35 | 3 | 6 | 1 | 0 | 25 (5: + x10 iv->vl) | as R5 |
| R5s | 40 | 3 | 6 | 1 | 0 | 26 (5: + x10 iv->vl) | lv1 lv1b lv2 p2 rh2 rh4 + 3 chains + x13b |
| R5hp | 43 | 5 | 6 | 1 | 0 | 36 (8) | as R4f |
| R7p | 43 | 2 | 6 | 1 | 0 | 35 (8) | as R4f |
| R5hps | 44 | 5 | 6 | 1 | 0 | 42 (8) | 3 chains + x13 x13b x14 |
| R7ps | 44 | 2 | 6 | 1 | 0 | 41 (8) | 3 chains + x13 x13b x14 |
| R5hpss | 46 | 5 | 6 | 1 | 0 | 42 (8) | 3 chains + x13b |
| R7pss | 46 | 2 | 6 | 1 | 0 | 41 (8) | 3 chains + x13b |
val2 fixes: all rules q_edge_comb_init_w; R5h/R5hp/R5hps/R5hpss + x7 x7b x7c (Level-only, PRE gap) + x9; R7* + x9; R5/R5i/R5s + f1 x9.
val2 never fixed (t>0 class, no t0 rule touches them): t1_*afirst x4, t2_*afirst x2, row7_initial; f1_finish under the R4f family
(comb runs twice at t0, both oracles once).
The 8 "away" moves of the p/ps/pss family: q_const_comb_first, s583_v01, s583_v03 (vl -> neither: line i0 becomes iv's xx, the forced
constant-comb-in-first-slice class, every rule); q_read_comb_init, s583_o05, x3_declinit_batch2 (vl -> iv); k1_ca_settled_miss
(vl -> iv: comb runs twice, W4031 x2 = iv); d1b_settle_woken (iv -> neither: count 2, iv 1; vl DIDNOTCONVERGE, no vl oracle).
R4f/R5h/R7 add x5, x5b (vl -> neither: `z t=0 v=x y=1`, the implicit run shares the promoted batch with the #0 reader = the
line-303 hop again) and x6 (vl -> iv); the "p" variants remove those three.
No split cell moves on report presence under any rule (only q_const_latch_first values, iv rejects the design).

R5h on Level-only designs (no always_comb): it changes PRE on x7, x7b, x7c (`always @*` / `always @(w)` settle-woken, batch-2
reader through a CA / port: iv = vl = value, PRE x; R5h = both oracles) and x7d (T reads xv: iv = vl = 1, PRE x; R5h order = iv).
Every Level change measured is toward both oracles; the suite run below is the wider check.
R5 / R5s fail the hierarchy / Level-feeds-comb cells (rh2 rh4 lv1 lv1b lv2 p2 q_hchain: iv = vl silent, PRE and R5 report): an
unarmed comb runs its only t0 pass before a producer that is itself woken later in Active. The armed family (R4f..R7pss) fixes them.
R5i touches none of the 3 residue cells and moves x10 (iv -> vl).
## Q3 winner R7pss: descent proof, machinery, option (b), run counts, ibex — DONE
Rule R7pss (always_comb / always_latch / comb-UDP = SensKind Comb|Latch after the option-(b) retag):
(1) static level waiter armed at seeding (arm_seq 0), at most one live static waiter per activity;
(2) a comb/latch the t0 settle record wakes (take_t0_wakes' held list) runs right after batch 1 and its settle, ONE PER BATCH, each
    followed by a settle, before batch 2 (whose front keeps the held Level/Edge wakes exactly as PRE);
(3) the implicit t0 run of every comb/latch is held for the first INACTIVE promotion at t0 and runs there ONE PER BATCH (tie order),
    each followed by a settle, before the promoted `#0` resumes / `#0` landing wakes, which then run as one batch as today.
No change for a design without always_comb / always_latch / comb UDP except the retag (self-timed / inert always = Initial).

Descent proof (cells, S/r2/R7pss + S/r3/R7pss): 187 + tp1 cells; rep2 46/50 fixed, 0 descents; val2 2/13 fixed, 0 descents; vl-only 6/6, 0 descents;
exit class 1 fixed (q_assert_noviol_combfirst rc 1 -> 0), 0 descents. native = interp = vm on every cell. No split cell moves on report
presence. Split value moves away from PRE's oracle position: 9 = the 8 listed in Q2 + tp1 (vl -> iv, count): 3 forced (constant comb
read in the first slice), 5 vl -> iv, 1 iv -> neither (d1b count, no vl oracle). (188 cells with S/c11/tp1; proto-PRE == PRE 188/188.)
Suite (S/suite/*.log; workspace nextest, debug, prototype tree): env unset 8981 passed / 0 failed; VITA_T0RULE=R7pss 8976 passed, 5 failed:
- elaborate tests::proc_v2::v2_4_clock_generator_self_timed (`left: Initial right: Comb`): the retag itself (structural pin).
- sim-engine native::kernel_tests::s1d4c2a_rearm_matches_the_engine_on_both_halves_of_the_asymmetry (`latch/proc0/Latch: t0 arm state
  diverged left: true right: false`) and native::tests::s1d3_wake_decision_matches_engine (`comb_chain_0_d2_w8_c3/pass0: wake decision
  diverged ... left: [1] right: []`): unit differentials that build `WakeTable::new` directly, whose t0 `level_armed` (wake.rs:160-168)
  the prototype did not change (it arms in arm_t0) — structural.
- cli::t0_phantom_settle an_always_comb_runs_once_at_time_zero... (`left: "C 0 d=1 e=164\nC 0 d=1 e=164\n" right: "C 0 d=1 e=164\n"`)
  and cli::obs_procs obs_procs_counts_are_hand_checkable (total_evals 57 -> 58; always_comb evals 11 -> 12): the documented count split
  (S/c11/tp1_phantom_comb_count.out: iverilog `C 0 d=1 e=164` x2, verilator x1, PRE x1, R7pss x2).
VITA_T0RULE=R5hpss: the same 5 failures, nothing else (its Level held-group change moved no existing pin, incl. always_star_no_t0_start,
t0_phantom_settle's Level cells, zero_delay_cont_assign, same_batch_wait).

t0 comb display counts (lines `<tag> t=0`), iv | vl | PRE | R7pss, cells where R7pss != PRE or != iv:
d1b 1|1*|1|2  f1 1|1|2|2  f2 0|1|1|0  f3 1|0|1|0  k1 2|1|1|2  tp1 2|1|1|2  q_chain_I_A_B 5|2|3|5  q_chain_I_B_A 4|2|2|4  q_chaind_A_B 2|2|3|3
q_chaind_B_A 3|2|2|3  q_chain3_A_Bb_Bc 5|3|4|5  q_chain3_A_Bc_Bb 3|3|5|5  q_chain3_Bb_A_Bc 5|3|4|5  q_chain3_Bb_Bc_A 5|3|3|5
q_chain3_Bc_A_Bb 4|3|5|5  q_chain3_Bc_Bb_A 4|3|5|5  q_src_alwayshash0_* 1|1|2|2  q_src_hash0_* 1|1|2|2  q_src_alwayswr_srcfirst 2|1|1|2
rh_declinit_port_child 1|1|2|2  s583_a00s 2|1|1|2  s583_a08 1|1|2|2  s583_a08b 2|1|1|2  s583_a15 1|1|2|2  s583_o04 1|1|2|2  s583_o05 2|1|1|2
s583_o07 5|3|3|5  (* vl DIDNOTCONVERGE after its first line). R7pss = iv on 14 of these 29 rows (f2 k1 tp1 I_A_B I_B_A chaind_B_A A_Bb_Bc Bb_A_Bc Bb_Bc_A alwayswr_srcfirst a00s a08b o05 o07),
PRE = iv on 2 (d1b f3). The `#0`-writer rows (hash0, alwayshash0, a15, o04) are 2 on PRE and R7pss alike.
Corpus (S/corpus/<mode>/, runner S/corpus/run.py replays corpus.rs's vita args per row in bench/<dir>, read-only; no file written under
bench after S/corpus/marker): modes pre (S/pre/vita), proto_unset, r7pss, r5hpss (S/proto_rel_vita, release, md5 f827e47ff036a68a724a5b401d8e6af8).
All 11 rows: rc=0, stdout AND stderr byte-identical across the four modes; digests = pinned (verilog-axi = its pinned vita split digest
DIGEST=fd90a1407928ebc8). ibex `DIGEST=13b2ddfcd551ba2f`, `simulation ended (Finish) at time 3000165000`, 4 `[at time 0]` lines (W4029) both.
Rule is live on ibex: `--obs-procs` (S/ibex/obs_PRE, obs_R7pss; backend native) total_evals 184104970 -> 184104992 (+22): always_comb
+19 over 20 of 49 processes, always_latch +1 (1 of 1), assign/port/net_init counts shifted on 20 items; digest unchanged.

Option (b) fits: every prototype rule ran with the retag; no self-timed / inert cell moved (q_self_clkgen n=2, q_self_atwait_*, n2, n2b,
n2c, q_src_alwayswr_srcfirst values = iv), every corpus row (each has a clock generator) byte-identical; the only suite pin it moves is
proc_v2 v2_4 (the kind itself).

Machinery for R7pss (prototype sites, S/proto.patch md5 729de8ec42be7292c10f347d689a2e33; 312+/22- incl. the other rules):
- elaborate/src/events.rs:375-393: self-timed / inert `always` -> SensKind::Initial (option b); format_version 35 (header.rs:15).
- interp + vm (one Scheduler): seeding scan_arm.rs:1446-1453 (Comb|Latch: arm_sensitivity(aid, 0) + push the implicit Ready on a new
  t0 list); arm_sensitivity scan_arm.rs:1586-1596 single live static waiter per activity (flag set on push, cleared in the
  propagate.rs:189 retain for `!in_body` Level waiters; on a second arm only arm_seq is refreshed); run_loop.rs: a parked-batch step
  at the loop top (park cur.active, run a one-process batch, `continue` so the loop-top settle + propagate run, then merge the parked
  entries back by push_sorted (seq, tie)), used (a) after batch 1 for the Comb/Latch members of take_t0_wakes' held list and (b) when
  the INACTIVE branch first fires at t0 (its condition also tests the t0 list), for the implicit runs, before the promoted #0 entries.
- native tier-3: arm_t0 run.rs:1121-1144 (wake.rearm_level(pi, 0) + the t0 list), run loop twin run.rs:358-406 and INACTIVE :526-545,
  plus WakeTable::new level_armed (wake.rs:160-168) so the engine/native t0-arm unit differentials agree (the prototype left it, 2 tests).
- JIT: no own scheduling site (native loop); not built (`--features jit` off). Staged: the retag is a SimIr value; not run
  (`--features separate-bins` not built).
- Delta budget: one delta per serial t0 pass (ibex +~50 at t0 against max_deltas 1_000_000).
Alternative R5hpss = R7pss + the held Level/Edge settle wakes also run one per batch with a settle: same cell score plus 4 Level-only
fixes (x7 x7b x7c x7d), same 5 suite failures, corpus byte-identical; it changes Level-only designs, i.e. outside this row.

## Q4 verdict — DONE
START(R7pss). Descents = 0 on all 188 cells (rep2, val2, vl-only, exit class) on native = interp = vm, 0 corpus moves, suite moves = 3
structural pins + 2 pins of the recorded count split (iverilog twice, verilator once). Fixes 46/50 two-oracle report gaps and 6/6
verilator-only gaps; the 4 left (q_chaind_A_B, q_chain3_A_Bc_Bb, s583_a08, x13b) are consumer-before-producer comb chains PRE also gets
wrong, an order axis among comb t0 passes (reverse = iverilog, topological = verilator). No PREREQ line needed for the row; the t>0
hop (t1 / t2 cells) stays the recorded §2 line-303 class.
## Open questions — DONE
1. Residue (2-oracle, PRE also wrong): q_chaind_A_B, q_chain3_A_Bc_Bb, s583_a08 (same-module comb chain, consumer first, decl-init fed)
   and x13b (two settle-woken combs, consumer first through a CA). Needs an order among comb t0 passes; reverse moves split cells to iv,
   topological to vl (round 1).
2. f1_finish (`initial begin s = 1; $finish; end`): both oracles run the comb once at t0; PRE `C s=x` + `C s=1`, R7pss `C s=1` twice
   (2-oracle value gap kept, not a descent). d1b: iverilog once, R7pss twice (no vl oracle).
3. R7pss moves the count split to iverilog on tp1 / obs_procs CLOCKED / k1 (re-pin t0_phantom_settle.rs:102 and obs_procs.rs:159 with
   both oracles quoted).
4. Not measured: JIT lane, staged vcmp -> velab -> vrun, the take_t0_wakes "no batch 1" path (settle wakes become batch 1, so a
   settle-woken comb shares it with Level wakes, not serialised) — no cell exercises it.
5. R5hpss's Level change (4 pre-existing Level 2-oracle gaps x7 x7b x7c x7d fixed, no suite / corpus move) is outside the row: a
   separate §2 row candidate.
6. The constant-comb-read-in-the-first-slice split (q_const_comb_first, s583_v01, s583_v03: line i0 moves vl -> iv's xx) is forced by
   every rule that fixes a_uc_combfirst.
7. IEEE §9.2.2.2 text still from secondary sources (round 1).
