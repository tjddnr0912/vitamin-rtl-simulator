# §4.5.590 grounding — §2 🆕 AB (§5.2 row 2)

status: COMPLETE (Q1-Q6 measured; worktree s590/gwt removed; final patch s590/probe5/P5_final.patch md5 2657ad45...)

PRE = s590/pre/vita md5 e1e7e57148bb83ad35d2c4f68247a290 (HEAD 303703f9). Probes built in worktree s590/gwt (removed at the end), CARGO_TARGET_DIR s590/target_g, release:
- P1 probe/vita 9110afe6...: call CAs (expr_has_call on rhs / lhs index) held until the first batch is taken, then the ordinary declaration-order settle.
- P2 probe2/vita dd968f69...: + release in two phases (non-held CAs settle first, then held CAs in dependency waves through any CA chain) + a released net's first write with no definite bit is not an event (t0 X-DROP).
- P3 probe3/vita 6cecfd45... = candidate (a) "a-call": + held delayed CA keeps its initial x drive, sized statically (no rhs evaluation) + X-DROP over every net the waves dirtied + native loop-top settle ends the run on a latched call_fatal (engine twin run_loop.rs:195).
- P4 probe4/vita c17b310f...: P3 + env VITA_PROBE_MODE=b (pre-batch evaluation muted, no re-evaluation) / b2 (muted + forced re-evaluation in waves). Default mode == P3 on all 104 cells.
- P5 probe5/vita 86da1d4b... (patch probe5/P5.patch, cumulative): P4 + env VITA_PROBE_SIDE=1 = candidate "a-side": hold only CAs reaching a callee not effect-free (levelize call_deps `func_effect_free`: walk_func_body admits it and no SysTask is reachable, transitively). Default env == P3 on all cells.
Cells s590/g/{ab,aa,c1..c8}/*.sv (124); oracle+PRE out_*/; PRE vs probe p1_* p2_* p3_* p4a/b/c_* p5_* (a-side) p5a_* (a); summ.py / summ3.py print them.

## Q1 census — where a call CA runs before the first batch (both backends)
- E1 lib.rs:1132 `settle_t0` -> scan_arm.rs:754 `settle_cont_assigns`; seed = every CA dirty (scan_arm.rs:661); `ca_always` visited every fixpoint pass. Native: native/run.rs:336 (seed `ca_dirty.insert_all`, native/dirty.rs:410).
- E2 initializer re-settle scan_arm.rs:1274 / native/run.rs:1042 (only with declaration initializers): dirty + ca_always.
- E3 copy-net repair: copy nets only (bit moves), no calls.
- E4 loop-top settle before batch 1: run_loop.rs:185 / native/run.rs:398 (ca_always + dirty).
- delayed CAs: `schedule_delayed_cas` (shared, scan_arm.rs:933) evaluates every delayed rhs per settle, and the x-drive evaluates the rhs for its WIDTH (scan_arm.rs ~835, native ~840) — side effects run there too (c1_delay: 4 x prints at t0).
- multi-driver groups: every member every pass (s4_md_call: 3 x + 2 value prints at t0).
- Second run: certified CA -> note_change on its deps (ca_of_net) after batch 1; uncertified (`ca_always`: class-handle/heap dep, $random, count-unsafe static local with deps, delayed, md) -> every settle pass at every step (n_ca_objf2case_tbL PRE: 4 W4031 at t0, 3 at t2, 2 at t3, 2 at t4).
- Side-effect lanes measured: $display (k1), W4031 unique/priority (q1caf, r2), E4003 assert/$error (k2b, r3, k1_error_x), W4007/I4005 (r4), F4004 $fatal (k2, k4), $finish-in-function F4004 (k5), W4020 null handle once per net (u20, u23), static-local carry (i3/i4: ca_always), $random (e1/e2: ca_always), class member mutation (m3: F4016 loud), function->task call (u8/u23, no oracle), module-net write = E3009 at elaborate (not a lane).

## Q2 oracle model (raw lines in out_*/)
- iverilog: a function CA is evaluated once at t0 AFTER the initials' first segments: a2 `i0 y=zz` / `f t=0 x=0 z=1` / `i1 y=01`; inputs never written still get one call on x (a3 `f t=0 x=x z=x`, j1 `WARNING: j1_unique_stayx.sv:5: value is unhandled ... Time: 0`); inputs written later at t0 (NBA a11, #0 a12, always a10) report at Time: 0 (vita PRE same). An argument that is an expression or a functor output gets an extra x call first: v4 `f(a[1],a[0])`, v6 `f(a | 1'b0, b)`, f1 generate `a[i]`, b1p `g(y)` with `assign y = {a,b}`, z3 `f(m)` with always_comb m: `WARNING ... Time: 0`. Self-contradiction: iv/w1_a `f(a)` prints `f t=0 x=0` once, iv/w4_or0 `f(a | 1'b0)` prints `f t=0 x=x` then `f t=0 x=0`. Two function CAs run in reverse source order (d1 `f t=0 x=1 z=0` first; s8 u2 before u1). Non-call CA from declaration initializers settles before the initial (q1 `i0 y=01 c=1 n=10`, all 3 tools); non-call CA from an undriven var stays z (o1n `i0 y=zz w=zz`).
- verilator: initial first for non-constant args (a2 `i0 y=00` then `f t=0 x=0 z=1`); CA first for constant / no args / constant parent port (a4, a6, a7, a8, a9: `f ...` then `i0 y=01`): split with iverilog.

## Q3 candidate shapes — moved cells (104-cell set ab..c5; a-side also on c6/c7/c8 = 20 more)
| candidate | moved | W->C | FL->C | W->W | split VL->IV | C(iv)->VL | i0 logic x | no-oracle | native split closed | correct->wrong |
|---|---|---|---|---|---|---|---|---|---|---|
| a-side (P5 SIDE=1) | 58 | 32 | 5 | 4 | 5 | 4 | 3 | 3 | 2 | 0 by IV/VL text (see notes) |
| a = a-call (P3) | 64 | 33 (+p3) | 5 | 4 | 5 | 4 | 3 (+o1) | 3 | 2 | 0; +5 pure-call cells' wire i0 -> IV zz |
| b mute, no re-eval | 63 | - | - | - | 0 | - | - | - | - | 6: a3, j1, a10, a11, a12 (IV+VL report lost on 4), z3 |
| b2 mute + re-eval | 59 | = a-side on side effects | | | 0 (values stay PRE) | 4 | 0 | | 2 | 0; e1/e2/m1 values shift; mute cannot undo $random draws or static writes |
Notes:
- W->C list (a-side): k1, q1caf_tbF/L, q6a_case, t0f_case; VL-only (iverilog rejects `unique if` / crashes on class CA): n_ca_objf2_tbL_H, n_ca_objf2case_tbL, q6a_H, t0f_if, t2t_H, u20_H; b1, b2, g1, j2, j3 (2->1 = IV), d1 (count; IV reverse order stays), b1r, b1u, b1uf, r1, s1, v12, x1, a13, r2, r4, s2, s5, s6, s3 (VL-consistent), s8 (count; IV reverse order stays). FL->C: k2b, k1_error_x, k2_fatal_x, r3, k5. W->W: c1_delay, h1, s4, e2 (per-settle re-evaluation stays). no-oracle: u23, u8, m1. native: k3 (end time 2 -> 1 = IV `Time: 1`), k4 (native 4 F4004 -> 1 = IV).
- split VL->IV: a4 (wire, = IV exactly), a8/a9 (wire part = IV), a6/a7 (logic target: PRE `i0 y=01` = VL text, POST `i0 y=xx`; IV `i0 y=zz`) — the only cells losing an oracle's literal text; on order they take IV's side.
- i0 logic x: a2, a3d, a5 (+o1 under a-call): a CA-driven `logic` read in batch 1 before its evaluation: POST `xx`, IV `zz`, VL `00`; PRE printed the seed value (`01`, `10`, `x1`) = neither. Hand-IEEE 1800-2017 §6.8 default for logic = x. Wire targets: POST `zz` = IV.
- C(iv)->VL: f1, b1p, v4, v6 lose iverilog's t0 glitch report (`WARNING ... Time: 0`); verilator silent; iverilog contradicts itself on the axis (w1_a vs w4_or0).
- c6/c7/c8 under a-side: aac1-4 (call CA + always_comb, AA-like) = both oracles; f2, g2 (interface port), t2 (import), u30 (automatic) W->C; h2 W->W; k7_finish W->C; k7_stop (PRE neither -> IV), k7_fatal (PRE neither -> VL); n4e/p3e = IV line sets; v14 VCD `$dumpvars` snapshot y 01->xx, w 01->zz (IV dumps end-of-step `b10`): neither either way; m3 F4016 unchanged; u31 E3009 unchanged; k9 unchanged.
- 🆕 AA cells (8: a08, aa1, q_chain3, q_chaind, t3j, tg2, tg5, x13b): 0 moved under every candidate (no call CA).
- (b) refuted: muting the only t0 evaluation of a stay-x CA loses reports both oracles print (j1, a10, a11, a12).

## Q4 lane table (proposed shape = a-side + heap clause)
| lane | shared site | status |
|---|---|---|
| seed settle E1 (engine/native) | settle_cont_assigns x2 | measured 124 cells x 3 backends |
| initializer re-settle E2 | same | measured a2, a7, j2, n3, p2, aac1-3 |
| pre-batch loop-top E4 (ca_always class CAs) | same | measured n_ca_*, m1, m3, m4, u20, u23, t2t, u8 |
| release settle + waves | new wrapper (both) | measured b1/b1u/b1r/s5/a13/x1/z1-z3/aac2/s8 |
| delayed CA x-drive / schedule_delayed_cas | shared | measured c1_delay, c2_delay_i0 (P2 regressed w to zz; P3 fixed) |
| multi-driver group | md loop (both) | measured s4 |
| copy-net repair | not routed (no calls) | opted out by construction; copy of held net measured n7, n4e |
| release X-DROP | new | measured n4, n4e, n7, n8 (P2 woke W2 on a copy; P3 fixed) |
| native post-settle fatal check | native loop | measured k2, k3, k4, k6, frame_call test; non-body source ($strobe arg) k9 unchanged |
| effect-free predicate (walk_func_body reuse) | levelize | measured: pure cells o1/p3/n4/n5/n7/n8/v13/m4 = PRE under SIDE; class method member write + `new` (u20 test) ADMITTED as effect-free -> needs the heap clause (hold when the CA reads a heap handle); with it those cells = P3 (measured) |
| staged vcmp/velab/vrun | same engine | measured c6: 9/9 staged == one-shot, PRE and POST |
| corpus | — | measured: 11 rows stdout+stderr byte-identical PRE vs P3 and vs P5-SIDE; held CAs: ethernet 80/323 (both), ibex 3/1839 (a-call) / 0 (a-side) |
| full gate | — | P2 9048: 2 FAIL; P3 9048: 1 FAIL (unique_if_chain a_constructor_called_through_new_stays_silent_at_time_0, pins W4020 count 2 = x-run artifact, no oracle); P5 SIDE=1: 9048/9048 pass (heap clause would bring the same W4020 re-pin) |
| JIT / product shape | — | see tail |
| Scheduler without run() | test constructors only | hold never released; production callers lib.rs:1078/1123 both reach run() |
Byte-identity (a-side): a design whose CAs reach no call, or only calls to effect-free callees and no heap handle, has t0_hold false for every CA: t0_held_now false => inner settle identical; release wrapper = one inner settle + no wave + empty drop set; native fatal check fires only on a latched call_fatal, which only frame machinery sets (pending at a loop-top only from a CA or a non-body site such as a $strobe/$monitor argument). ibex = 0 held under a-side.
| JIT (`--features jit`, VITA_JIT=1) | none of its own | measured: probe5_jit md5 886ab1cf... vs probe5, SIDE=1, 124 cells x native/interp/vm = 372 runs, 0 diff |
| product (`--no-default-features`) | — | measured: probe5_nodef md5 c20e1535..., 124 cells native: 121 equal; 3 differ = product-shape refusals (c1_delay, c2_delay_i0, i1: "this build carries no other executor"), as on PRE's shape |
Start condition: no lane unmeasured for the proposed shape (the heap clause's cells measured through P3, which holds them).

## Q5 corpus (order oracle)
s590/g/corpus.py runs every corpus.rs row with a given binary in a scratch copy s590/bench. PRE: 10 rows digest_match=True, verilog-axi False (pinned Expect::Split). P2, P3, P5 SIDE=1: all 22 stdout/stderr files byte-identical to PRE (cmp). Held CAs (VITA_PROBE_COUNT): verilog-ethernet 80/323 (a-call and a-side: lfsr_mask holds $error/$finish), ibex 3/1839 (a-call) / 0 (a-side), every other row 0.

## Q6 §2 / PROBE_CATALOG grep
settle_cont_assigns: ROADMAP 4b-r (native scratch pooling, perf only); func_read_deps: ⑧ "system functions in a function body"; "extra t0 settle": ⑧ "a counter-carrying function is one evaluation off (extra t0 settle)" (same root; i3/i4 unchanged: count-unsafe callee stays ca_always — the call_deps.rs note says removing the t0 extra evaluation is what lets that family be certified: follow-up, not this slice); t0 settle: CA-hop t0 Level/Edge sub-line (non-call CAs, untouched: 0 held); AA line 378 (split, 0 moved); manual 006 §3.1 rows `$random` (e1/e2 stay wrong) and 🆕 AB. No row or catalog line for: native post-settle fatal, the iverilog functor-argument x call, the held-logic i0 x/z, the non-call seed-on-x phantom edge, `static ... = init` in a function (E2002), u31 E3009.

## Proposed shape (a-side + heap clause)
1. Scheduler::new: t0_hold[ci] = the CA's rhs or lhs index reaches a user call AND (a reached callee is not effect-free — walk_func_body declines or a SysTask is reachable, transitively — OR the CA reads a heap handle); t0_preds[ci] = held CAs it reads through any CA chain (ca_deps sets x lhs drivers).
2. Every settle before the first batch is taken (seed, initializer re-settle, pre-batch loop-top) skips a held CA and leaves it dirty; a held delayed CA still drives its initial x sized max(lvalue_width, self_width) without evaluating the rhs; schedule_delayed_cas and an md group with a held member skip it.
3. On the first batch take (or at run start when there is none) the next settle: settle non-held CAs to fixpoint, then release held CAs in waves (all preds released; a cycle releases the rest), settling after each; a net first dirtied by the waves with no definite bit leaves the change list (t0 X-DROP).
4. Native loop-top settle ends the run on a latched call_fatal (engine twin run_loop.rs:195): closes native `(Quiescent)` / end-time-2 vs interp `(Error)` at time 1 (k6, k3).
5. Engine + native/run.rs mirror 2-4; JIT none; no IR or format change.
Smallest safe cut = all five (P1 left s5/a13/b1 ordering and a double print; P2 regressed c2_delay_i0 w `zz` vs PRE=iverilog `xx` and woke W2 on a copy in n7; without 4 the release-time fatal ends native Quiescent: frame_call test). Optional narrowing: drop the heap clause -> 0 test re-pins, class-method CAs with effect-free callees keep PRE's x-run (W4020 only, no oracle).
Test re-pin with the heap clause (or a-call): unique_if_chain.rs a_constructor_called_through_new_stays_silent_at_time_0 `VITA-W4020` count 2 -> 1.
