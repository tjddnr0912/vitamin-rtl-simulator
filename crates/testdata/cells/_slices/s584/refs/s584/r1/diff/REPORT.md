# lens:differential round 1 — s584 (A = R7pss, A+B = R5hpss)
verdict: FAIL (4 BLOCKING descents, all in A; B inherits them, adds none)
binaries (frozen; md5 re-checked at end): PRE vita_pre 2492e1c5c52c7081a6c0997fe194af40 | A vita_a caef001a26e755f24510e4ec9f89e443 | A+B vita_ab 5d8d3960ad827868fe41afdfcabc186b (release, 7321504 B each)
oracles: iverilog 13.0 -g2012 ; verilator 5.052 --binary --timing --assert, run +verilator+error+limit+1000
harness: r1/diff/run3.py (iv, vl, vita {pre,a,ab} x {native,interp,vm}); cells b1 (18) b2 (16) b3 (4) b4 (2) = 40 designed + res/ (4 census residue cells re-run)
native = interp = vm on every cell, every binary.

## findings (most severe first)
| id | cell | iv raw | vl raw | PRE | A | A+B | class | NEW? | sev |
|---|---|---|---|---|---|---|---|---|---|
| D2 | b2/u5c_udp_declinit_ff | t=1 q=1 o=1 | t=1 q=1 o=1 | t=1 q=1 o=1 | t=1 q=x o=1 | t=1 q=x o=1 | real gap; x captured by a flop on a t0 posedge, still x at t=1 | NEW (A) | BLOCKING |
| D1 | b2/u5b_udp_declinit_b2 | b2 t=0 o=1 | b2 t=0 o=1 | b2 t=0 o=1 | b2 t=0 o=x | b2 t=0 o=x | real gap | NEW (A) | BLOCKING |
| D3 | b2/u5e_udp_child_declinit_port | b2 t=0 o=0 | b2 t=0 o=0 | o=0 | o=x | o=x | real gap (through a port) | NEW (A) | BLOCKING |
| D4 | b2/u5f_udp_ca_from_declinit | b2 t=0 o=1 | b2 t=0 o=1 | o=1 | o=x | o=x | real gap (through a CA) | NEW (A) | BLOCKING |
| N1 | b2/u5a_udp_declinit_initial | i0 t=0 o=1 | i0 t=0 o=1 | o=x | o=x | o=x | real gap | pre-existing | NOTE |
| N2 | res/q_chaind_A_B, q_chain3_A_Bc_Bb, s583_a08, x13b | no t0 report (B/Bb runs before A) | no t0 report | W4031 t0 | = PRE | = PRE | real gap, 2-oracle (both oracles producer-first at t0) | pre-existing | NOTE |
| S1 | b1/f2, f2b (t0-posedge flop samples implicit-only always_comb) | q=xx / q=x | q=10 / q=6 | = vl | = iv | = iv | split (order) | moved vl->iv | NOTE |
| S2 | b1/r1, r2 (Active-delta-2 reader of implicit-only comb via CA / port) | xx | value | = vl | = iv | = iv | split | moved vl->iv | NOTE |
| S3 | b1/n1 (event triggered from comb, comb first) | E1 t=0 y=01 | (none) | (none) | = iv | = iv | split | moved vl->iv | NOTE |
| S4 | b1/fj1 (fork arm #0 + @(y)) | Z y=xx, F y=01 | Z y=01, no F | = vl | Z y=01, F | = A | split | vl->neither (iv: #0 arm before comb pass) | NOTE |
| S5 | b1/e2, b4/ub1, b4/ub2 (Level settle wake declared/tied before the comb/UDP it reads) | L y=xx / r=x / r=x | y=10 / r=0 / r=1 | = vl | = vl | = iv | split | B moves vl->iv | NOTE |
| G1 | b2/b1_ca_hop_noinit (no initial; Level settle wakes, CA hop) | b=0 | b=0 | b=x | b=x | b=0 | real gap closed by B | pre-existing, fixed | NOTE |
| C1 | b1/f1b W4031 count at t0 | 2 | 0 (2-state) | 1 | 2 | 2 | count (iv only) | = iv | NOTE |
| C2 | b2/cnt1, cnt1b comb display count at t0 | 2 | 1 | 1 / xx+01 | 2 | 2 | count | = iv | NOTE |

Root cause D1-D4 (source): hdl-parser/src/udp_table.rs:132-141 desugars a combinational UDP to `ProcKind::AlwaysComb`; sched/scan_arm.rs:1465-1467 (`SensKind::Comb | SensKind::Latch => { self.arm_sensitivity(aid, 0); push_sorted(&mut self.t0_implicit, ready); }`) and run_loop.rs:244-248 (native/run.rs:538-541, :1155) hold its only time-0 pass to the first Inactive promotion. A UDP input changed only by a declaration initializer (directly, via a CA, via a port) gives no settle-record entry (an initializer is no event), so the UDP is implicit-only and every Active-delta>=2 reader / t0-edge flop reads x. Both oracles evaluate the primitive before them (iv u4_chain2 census line `i0 t=0 o=0 m=1` already showed iverilog evaluates a decl-init-fed UDP before the first initial slice). Not hit: const tie (u1b, u6c), input through a gate primitive (u6d), `#0` reader (u5d).

## other measurements
- corpus: S/corpus/{pre,postA,postAB} made by S/post/vita_a (md5 caef...) / S/post/vita_ab (md5 5d8d...) = frozen bytes; cmp of all 11 rows' .out/.err vs pre: 0 diffs.
- suite (B tree): S/post_suite_b.log `9003 tests run: 9003 passed, 15 skipped`, rc 0. cli udp_comb.rs has no declaration-initialised UDP input (grep), so no pin covers D1-D4.
- PRE = A = AB: f1 fn1 rm1 if1 st1 n1b z1 u5d u6c u6d; vcd1 final t0 values (iv y=10 z=10 s=0 = all); vita's $dumpvars block lists y=xx then `#0 b10` on A/AB (harness-format).
- pre-existing loud, PRE = POST: fr1 E3001 (force on an always_comb variable counted as a second driver; iv runs, vl warns), u6a/u6b E3009 (supply1 / tri1 unsupported).
- no oracle: da1/da2 (`assert #0` / `assert final`: iverilog "sorry: Deferred assertions are not supported"; vl silent = vita all).

## open
- B scored only on 5 Level/UDP cells here (e2 b1 ub1 ub2 + suite/corpus); no B descent found.
- Not run: JIT, staged (claimed identical by census), sequential UDPs (not Comb), always_latch fed by decl-init (iverilog thread -> expected split like r1).

# ROUND 2 (delta) — POST2 = A + fixes (B dropped)
binary: S/r2/vita_post2 md5 28f38a3e306640b2c59907c1ab0c5825 (release); PRE + r1-A unchanged at S/r1/
changes: comb UDP -> `initial <cascade>` + `always @(in1 or ...)`; t0 implicit pass is a TRIGGER (take_t0_trigger), dropped when the block is busy / its static waiter is not live.
## r2 status
verdict r2: FAIL — 2 BLOCKING on the exit-class axis (tg10, tg13), introduced by the trigger rule; D1-D4 fixed.
### Q1 re-score of all r1 cells (40 + 4 res) on PRE / r1-A / POST2 x native,interp,vm (r1 oracle text reused; files */*.out2)
- FIXED: D1 u5b, D2 u5c, D3 u5e, D4 u5f: POST2 = PRE = both oracles (o=1 / q=1 / o=0 / o=1).
- POST2 = r1-A (unchanged since r1): e1a e1b e1c e2 f1b f2 f2b fj1 n1 r1 r2 cnt1 cnt1b cnt2 w1.
- B gone: b1_ca_hop_noinit back to PRE (b=x; iv=vl b=0, pre-existing gap); e2 ub1 ub2 back to the vl side (splits).
- POST2 = PRE = r1-A: the other 23. native = interp = vm everywhere.
### Q2 new cells (23: r2u/ud1-ud9, r2t/tg1-tg14; raw text in *.out2)
| id | cell | iv raw | vl raw | PRE | r1-A | POST2 | class | NEW? | sev |
|---|---|---|---|---|---|---|---|---|---|
| R2-1 | r2t/tg10_assert_chain3_fwd (decl-init src -> p -> m -> immediate assert, topological) | `ERROR: ...:8: A t=0 m=xx` Time 0, run rc=0 | silent, rc=0 | silent rc=0 | silent rc=0 | `error[VITA-E4003] ... A t=0 m=xx [at time 0]`, rc=1 | report: split (iv); exit class: iv=vl=PRE=0, POST2=1 | NEW (trigger rule) | BLOCKING (exit class) |
| R2-2 | r2t/tg13_constcomb_chain3_assert (`always_comb p = CFG;` source) | ERROR at t0, rc=0 | silent rc=0 | silent rc=0 | silent rc=0 | E4003 at t0, rc=1 | as R2-1 | NEW | BLOCKING (exit class) |
| R2-3 | r2t/tg1 tg3 tg4 tg6 tg8 (3-chain gen-if, gen-for, 3 instances via hier refs, 4-chain, latch middle; all topological) | W4031 Time 0 | silent | silent | silent | W4031 [at time 0] | split (report presence) | moved vl->iv | NOTE |
| R2-4 | r2t/tg14_assert_chain2_ctrl (2-chain control) | ERROR at t0 | silent | silent | silent | silent | split | = PRE | NOTE: depth 2 silent, depth >=3 reports (vita self-inconsistent) |
| R2-5 | r2t/tg2 tg5 (consumer-first gen / instances) | silent | silent | W4031 | W4031 | W4031 | real gap (residue class) | pre-existing | NOTE |
| R2-6 | r2u/ud1_seq_comb_udp_t0edge | e t=1 q=1 | e t=1 q=1 | q=0 | q=0 | q=0 | real gap (seq UDP misses t0 0->1 edge) | pre-existing | NOTE |
| R2-7 | r2u/ud4_udp_initial_driven_ff | t=1 q=1 | t=1 q=1 | q=x | q=x | q=x | real gap (same-batch port hop) | pre-existing | NOTE |
| R2-8 | r2u/ud9_udp_instance_delay (`inv #2 u(o,k)`) | runs (w t=2 o=1 ...) | runs | E2002 parse | E2002 | E2002 | loud gap | pre-existing | NOTE |
| R2-9 | r2u/ud8_udp_comb_mutual | b2 c=x u_o=x z=x | c=0 u_o=1 z=1 | c=0 u_o=x z=x | = iv | = iv | split | PRE neither -> iv | NOTE |
| R2-10 | r2t/tg7_woken_once_count | 2 lines (xx, 01) | 1 | 1 | 2 | 1 | count | = PRE | NOTE (woken block runs exactly once) |
Same on PRE / r1-A / POST2: tg9 tg11 (assert #0 chain: vita + vl silent; iv unsupported) tg12 ud3 ud5 ud6; POST2 = PRE: ud2 (i0 line o=x pre-existing gap; b2 = oracles; r1-A had b2 x), ud7 (split, neither).
Mechanism R2-1..R2-3 (run_loop.rs take_t0_trigger + trigger pop loop): the first trigger's pass wakes the middle block, whose trigger is then dropped (waiter consumed) and whose woken run waits in t0_parked behind EVERY later trigger; the consumer two hops down runs its pass on the stale middle value. r1-A ran the middle's pass in place (silent). Needs a source no batch-1 / settle event reaches: declaration initializer or a constant (no-read-set) comb.
obs: run.json processes.items lists a comb UDP as `initial` + `always` (2 items) instead of 1 `always_comb` (u5b: always_comb:1 -> initial:1 + always:0), native = vm — harness-format NOTE.
staged: tgt-post/debug vcmp 603ba824737dd1de3ccbb10d20937a34 / velab 83d85b2cbd2f441ede522ca2cf8c1563 / vrun b6e77beab66827c495f5ceb627d13716 = one-shot POST2 on u5b ud8 ud1 ud2 tg1 tg7; debug vita 732d3ca521eec9a59b01066074cc047f = POST2 release on the same 6 (provenance consistent, not md5-proven).

# ROUND 3 (last, delta) — POST3 = POST2 + slot rule (a woken-not-run block's pending start runs in its trigger's slot, queued start removed; busy drops) + O(1) waiter retirement
binary: S/r3/vita_post3 md5 469298289bfacab02e755ec2d18fa855 (release)
## r3 status
verdict r3: PASS (0 BLOCKING). worktree status md5 869d517263e1f120f2ad3f8158bfadbf at r3 start and end.
### Q1 re-score of all r1 + r2 cells (67) on PRE / POST2 / POST3 x native,interp,vm (files */*.out3)
- moved vs POST2, all to POST3 = PRE: tg10 tg13 (silent, rc 0), tg1 tg3 tg4 tg6 tg8 (silent). Every other cell POST3 = POST2 (D1-D4 stay fixed). No backend split.
### Q2 new cells (10: r3c/t3a b d e f g i j k m)
| id | cell | iv raw | vl raw | PRE | POST2 | POST3 | class | NEW? | sev |
|---|---|---|---|---|---|---|---|---|---|
| R3-1 | t3g_port_chain_slot (top comb -> child comb via port -> child checker) | W4031 Time 0 (top.u_c) | silent | W4031 | W4031 | silent | split | moved iv->vl (away from PRE's oracle side) | NOTE |
| R3-2 | t3k_checker_batch1_woken_reads_implicit | W4031 x2 Time 0 | silent | silent | W4031 | W4031 | split | moved vl->iv (since A; not new in r3) | NOTE |
| R3-3 | t3a_woken_twice_chain (middle woken by batch 1, then superseded) | M p=xx, W4031, M p=xx, M p=01 | M p=01 | M p=01 | M p=xx, W4031, M p=01 | M p=xx, M p=01 (silent) | split (display count / transient) | report = PRE | NOTE |
| R3-4 | t3b_wire0_landing_chain | W4031 Time 0 | 3x "%Error ... none matched for '2'h0'", y=0 | silent y=1 | W4031 | silent y=1 | no oracle (vl 2-state, final y=0 contradicts all) | = PRE | NOTE |
| R3-5 | t3j_assert_each_depth_rev | silent rc 0 | silent rc 0 | W4031 + D3 D2 D1 E4003, rc 1 | same | same | real gap (residue class) | pre-existing | NOTE |
| R3-6 | t3m_mixed5_batch1_middle | W4031 + ERROR A3, rc 0 | silent rc 0 | E4003 + W4031, rc 1 | same | same | split / pre-existing rc | pre-existing | NOTE |
POST3 = PRE (= vl on t0 lines): t3d (counts at t=5/t=10 one line per comb = iv; no stale-waiter refire), t3e (slot reads newer inputs: B/D once, = vl), t3f (superseding slot run calls $finish: A, B, C once; iv also prints its transients), t3i (assert at each depth, topological: silent rc 0; POST2 had D2 D3 E4003 rc 1).
