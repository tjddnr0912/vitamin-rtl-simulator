# GROUNDING s584 — §2 🆕 Z (always_comb / always_latch t0 order)

PRE: S/pre/vita md5=2492e1c5c52c7081a6c0997fe194af40 release, built from bf7f3e0d. HEAD 63b9dbb9.

## Q1 re-measure claim — DONE
Cells S/c1/*.sv (+ .out raw), traces S/c1t/*.sv. Summaries via S/summ.py (R@t = W4031 / iv WARNING / vl %Error at time t).
All vita cells: native=interp=vm byte-identical. iv rejects `unique if` (syntax error) -> those cells vl-only.
Every cell carries a real violation at t2 (positive control: all three tools report at t2).

| cell | iv | vl | vita PRE | class |
|---|---|---|---|---|
| a_uc_combfirst (unique case, comb before initial) | t2 | t2 | t0+t2 | real gap (2 oracles) |
| a_uc_initfirst | t2 | t2 | t2 | agree |
| a_pc_combfirst (priority case) | t2 | t2 | t0+t2 | real gap |
| a_pc_initfirst | t2 | t2 | t2 | agree |
| a_lc_combfirst (always_latch unique case) | t2 | t2 | t0+t2 | real gap |
| a_lc_initfirst | t2 | t2 | t2 | agree |
| sa_ui_combfirst (single unique if) | syntax err | t2 | t0+t2 | real gap (vl only) |
| sa_ui_initfirst | - | t2 | t2 | agree |
| sa_li_combfirst (always_latch single unique if) | - | t2 | t0+t2 | real gap (vl only) |
| a_ui_* / a_li_* / b_ui_* / b_li_* (unique if ... else if CHAIN) | - | t2 | silent everywhere (incl. t2) | row 2 defect (chain never reports), not this row |
| b_uc_v_d{1,2,3}_{tbfirst,leaffirst} (input logic ports, depth 1-3) | t2 | t2 | t0+t2 (all 6) | real gap |
| b_uc_w_d{1,2,3}_{tbfirst,leaffirst} (input wire ports) | t2 | t2 | t0+t2 (all 6) | real gap |
| b_pc_v_d1_{tbfirst,leaffirst} | t2 | t2 | t0+t2 | real gap |
| b_lc_v_d1_{tbfirst,leaffirst} (always_latch in child) | t2 | t2 | t0+t2 | real gap |
| sb_ui_v_d1_{tbfirst,leaffirst}, sb_ui_v_d2_tbfirst, sb_li_v_d1_tbfirst | - | t2 | t0+t2 | real gap (vl only) |

vl reports each violation twice per step and keeps re-reporting latches at t3/t4 (re-evaluation count; not an oracle for counts).

WHY (b) fails with the testbench first — traces c1t/tb1_tbfirst.sv, tb1_leaffirst.sv, tb1_samemod_assign.sv (raw):
- vita (both text orders): `I0 t=0 s=xx u.s=xx` / `I1 t=0 s=01 u.s=01` / `C t=0 s=xx` / W4031 [in top.u] [at time 0] / `C t=0 s=01` / `I2 ...`
  -> the initial DOES run first (even with the leaf module written first), and the comb still reads its port at xx.
- iv: `I0 xx` / `I1 01 01` / `C t=0 s=01` / `I2` / `C t=0 s=01` (comb's first run already sees 01). vl: `I0 00` / `I1 01` / `C s=01` / `I2`.
- same-module control tb1_samemod_assign (`assign w = s;` initial written FIRST, comb reads w): vita `C t=0 w=xx` + W4031 at t0; iv `C w=01` silent; vl `C w=01`.
  => (b) is not a hierarchy/ProcId effect: any comb input that reaches it through a continuous assign (a port binding is one) is stale at the comb's t0 run.
- Mechanism (code): t0 seeds Initial|Comb|Latch go into ONE Active batch with one seq (sched/scan_arm.rs:1425-1453; native/run.rs:1110-1145).
  The run loop takes the whole batch and runs its bodies back-to-back; continuous assigns settle (`settle_cont_assigns`) and `propagate_changes`
  run only BETWEEN batches (sched/run_loop.rs:66-75 settle at loop top, :193-196 propagate after the batch). So inside batch 1 a comb reads
  CA/port-driven nets before the initial's writes have propagated, whatever the tie order. (a) is the tie order inside the same batch
  (comb tie < initial tie => comb runs first on x).
- Internal inconsistency recorded: in the same instant the initial reads hierarchical `u.s` = 01 (I1) while the comb reads its port `s` = xx.
## Q2 class width — DONE
Cells S/c2/*.sv (+ .out raw). vita native=interp=vm byte-identical in every cell. iv=iverilog, vl=verilator (2-state: no x/order oracle).
Classes: GAP = real gap (vita wrong, iv=vl agree); SPLIT = iv and vl disagree (vita side noted); AGREE = vita = iv = vl.

| cell | iv raw | vl raw | vita PRE raw | class |
|---|---|---|---|---|
| q_assert_combfirst (immediate assert in always_comb, comb first) | `ERROR: ..:5:` Time 2 only | R@2,R@3 | E4003 `Assertion failed [at time 0]` + t2 | GAP |
| q_assert_initfirst | t2 | t2 | t2 | AGREE |
| q_assert_hier (assert in child comb) | t2 Scope top.u | t2 | E4003 at t0 + t2 | GAP |
| q_assert_noviol_combfirst (no real violation) | rc=0, no ERROR | rc=0 silent | rc=1, `error[VITA-E4003] ... Assertion failed [in top] [at time 0]` | GAP (exit class: clean design fails) |
| q_edge_w_comb_init (watchers; `always_comb y = (s===1)`; initial s=1) | `POS t=0`, neg=0 pos=1 any=1 | WAIT0 released (2-state y=0), POS, neg=0 pos=1 any=1 | `NEG t=0`, `WAIT0 released t=0`, POS, neg=1 pos=1 any=2 | GAP on VALUES (neg, any); WAIT0 SPLIT (vita=vl by 2-state accident) |
| q_edge_comb_init_w | POS, 0/1/1 | POS, 0/1/1 | WAIT0, NEG, POS, neg=1 pos=1 any=1 | GAP (neg) |
| q_edge_w_init_comb / q_edge_init_comb_w | POS 0/1/1 | 0/1/1 | POS 0/1/1 | AGREE |
| q_edge_hier (comb in child) | POS 0/1/1 | WAIT0, POS 0/1/1 | NEG, WAIT0, POS, 1/1/2 | GAP (values) |
| q_edge_latch_comb_init (always_latch) | POS 0/1/1 | WAIT0, POS 0/1/1 | NEG, WAIT0, POS, 1/1/2 | GAP (values) |
| q_chain_B_A_I (B:`b=a` comb, A: unique case on {a,b}, initial a=0) | silent t0 | silent | `A ab=xx` R@0 | GAP |
| q_chain_I_B_A | silent | silent | silent | AGREE |
| q_chain_A_B_I | `A ab=0x` R@0 (1 report) | silent | R@0 x2 (ab=xx, ab=0x) | SPLIT (vita=iv on report, count 2 vs 1) |
| q_chain_I_A_B | R@0 (A 0x) | silent | R@0 (A 0x) | SPLIT (vita=iv) |
| q_chaind_A_B (a=0 by decl init, A before B) | `B; A ab=00` silent | silent | `A ab=0x` R@0 | GAP |
| q_chaind_B_A | `A ab=0x` R@0 (iv runs implicit t0 passes in reverse order) | silent | silent | SPLIT (vita=vl) |
| q_chain3_A_Bc_Bb (A reads c; Bc:c=b; Bb:b=a; decl init) | Bb,Bc,A(00) silent | silent | A(0x) R@0 | GAP |
| q_chain3_A_Bb_Bc | Bc,Bb,A(0x) R@0 | silent | R@0 | SPLIT (vita=iv) |
| q_chain3_Bb_A_Bc | R@0 | silent | R@0 | SPLIT (vita=iv) |
| q_chain3_Bb_Bc_A | A(0x) R@0 | silent | silent | SPLIT (vita=vl) |
| q_chain3_Bc_A_Bb | R@0 | silent | R@0 | SPLIT (vita=iv) |
| q_chain3_Bc_Bb_A | R@0 | silent | R@0 | SPLIT (vita=iv) |
| q_src_nba_{combfirst,srcfirst} (`initial s <= 1`) | `C s=xx` R@0 | R@0 | R@0 | AGREE (all report) |
| q_src_hash0_{combfirst,srcfirst} (`initial #0 s = 1`) | `C s=01` silent | `C s=00` R@0 (comb before the #0, 2-state miss) | `C s=xx` R@0 | SPLIT on order (vita=vl) |
| q_src_hash00_* (`#0 #0`) | R@0 | R@0 | R@0 | AGREE |
| q_src_alwayswr_combfirst (`always begin s=1; #10; end` after comb) | silent t0 | silent | R@0 | GAP |
| q_src_alwayswr_srcfirst | silent | silent | silent | AGREE |
| q_src_alwayshash0_* (`always begin #0 s=1; ..`) | silent | R@0 | R@0 | SPLIT (vita=vl) |
| q_src_ca_init_ca_comb / q_src_ca_comb_ca_init (`assign w = s`) | silent | silent | R@0 (both orders) | GAP |
| q_src_caop_init_comb (`assign w = s ^ 2'b00`) | silent | silent | R@0 | GAP |
| q_src_declinit_comb (`logic [1:0] s = 1`) | silent | silent | silent | AGREE |
| q_read_comb_init (initial: s=1; $display y; #0 ...) | r0 xx, r1..r3 01 | r0 00 ... | R@0, r0 00, r1..r3 01 | r0: vita 00 vs iv xx (vl no x-oracle) + GAP report |
| q_read_init_comb | xx/01/01/01 | 00/01.. | xx/01/01/01 | AGREE (iv) |
| q_read_hier | xx/01.. | 00/01 | xx/01.. + R@0 | GAP (report) |
| q_read2_comb_w_r (comb, writer initial, reader initial) | x0 xx | x0 01 | x0 00 | SPLIT, vita matches neither |
| q_read2_w_r_comb / q_read2_r_w_comb | xx | 01 | xx | SPLIT (vita=iv) |
| q_self_clkgen / _initfirst (`always #5 clk=~clk`) | n=2 | n=2 | n=2 | AGREE (must not move) |
| q_self_atwait_alw_init (`always begin @(a) .. end` before `initial a=1`) | GOT t0, n=1 | n=0 | GOT, n=1 | SPLIT (vita=iv) |
| q_self_atwait_init_alw (initial first) | GOT, n=1 | n=0 | n=0 | SPLIT (vita=vl) — self-timed always position, pre-existing, not this row |
| q_self_atwait_comb / _comb_first (waiter on comb output) | n=1 | n=0 | n=1 | SPLIT (vita=iv) |
| q_gen_comb_first (generate-for always_comb) | t2 | t2 | R@0 x2 + t2 | GAP |
| q_gen_init_first | t2 | t2 | t2 | AGREE |
| q_const_comb_first (`always_comb y = 3`, reader i0..i3 with #0s) | xx/xx/11/11 | 11/11/11/11 | 11/11/11/11 | SPLIT (vita=vl) |
| q_const_init_first | xx/xx/11/11 | 11 x4 | xx/11/11/11 | i1 vita matches neither; i0 vita=iv; i2,i3 AGREE |
| q_const_latch_first | rejects (`always_latch process has no event control`) | 10 x4 | 10 x4 | no iv oracle |
| q_constuc_comb (unique case on a constant, real miss) | R@0 | R@0 | R@0 | AGREE |
| q_hchain (top comb x=s -> port -> child unique case) | t2 | t2 | R@0 + t2 | GAP |

Class width (GAP cells): reports (W4031 unique/priority case and if, E4003 immediate assert, exit class rc 0->1) AND values
(a spurious x->0->1 at t0 seen by `@(negedge)` / `@(y)` counters). Feeding paths that show it: initial or self-timed `always`
written after the comb; any continuous assign (port binding, `assign w = s`, operator CA); comb->comb chains where the
consumer's t0 pass precedes the producer's (decl-init driven, no process write at t0); comb->port->comb; generate scopes; latches.
Feeding paths that do NOT diverge: NBA (`s <= 1`) and `#0 #0` writes (all three report at t0), decl initializers.
## Q3 rule fit — DONE
IEEE 1800-2017 §9.2.2.2: always_comb "is automatically triggered once at time zero, after all initial and always procedures have
been started" (wording from secondary sources: deltahdl issue #3552 quoting the LRM, verilogpro.com; the LRM text itself was not read in
this session). §9.2.2.2.2: "always_comb automatically executes once at time zero, whereas always @* waits until a change occurs".
§9.2.2.3 always_latch: same rules as always_comb (recollection, not re-read).

Method: every candidate is HAND-SPELLED in SV and run on the PRE binary (S/c3/transform.py; spelled cells S/c3/sp/<rule>/*.sv + .out;
originals re-run on all tools in S/c3/orig/*.out; comparison S/c3/cmp.txt, scorer S/c3/score.py). 121 cells (c1, c1t, c2, and 21 s583 cells).
Score = t0 report presence (W4031 / iv WARNING / vl %Error / E4003 / iv ERROR) on cells where iv and vl agree; values read by hand (below).

Corrected scoring (S/c3/score2.py): 121 cells = 84 two-oracle report cells (PRE right on 45, GAP 39), 21 vl-only cells (iv rejects
`unique if` / a constant latch; GAP 6), 16 split cells. (An earlier count that treated iv's syntax errors as "silent" is superseded.)

| rule | spelling (PRE) | 2-oracle GAP fixed /39 | vl-only GAP fixed /6 | descents | split cells moved |
|---|---|---|---|---|---|
| R1 seed initial/always first, comb/latch after, same batch | comb items moved after all other items of the module | 10 | 2 | 0 | 0 |
| R2 comb t0 pass held to the front of Active batch 2 (after batch 1 + CA settle), waiter armed at seeding | `always @*` + `always @(__t0w)` copy, `assign __t0w = 1'b1` (t0 settle wake = held to batch 2) | 35 | 6 | 0 | 0 |
| R2r R2, reverse order | same, copies reversed | 36 (gen unspelled) | 6 | 1 (q_chain_I_B_A) | 5 |
| R3 iverilog: armed + implicit run after the first #0 batch, reverse source order | `always @*` + `initial #0` copies at module END, reversed | 39 | 6 | 0 | 9 (all ->iv: hash0 x2, alwayshash0 x2, a15, o04, a08b, chaind_B_A, chain3_Bb_Bc_A) |
| R3f R3 forward | copies at END, forward | 35 (gen unspelled) | 6 | 0 | 6 (#0-writer cells ->iv) |
| R4 armed + implicit run at the HEAD of the first #0 promotion (= when Active first drains, before any #0 continuation), reverse | `always @*` + `initial #0` copies at module TOP (before every process), reversed | 39 | 6 | 0 | 3 (->iv: a08b, chaind_B_A, chain3_Bb_Bc_A) |
| R4f R4 forward (tie order) | copies at TOP, forward | 36 | 6 | 0 | 0 |
| R4t R4 in topological order (producer comb first) | copies at TOP, producer-first | 39 | 6 | 0 | 4 (->vl: chain3 A_Bb_Bc, Bb_A_Bc, Bc_A_Bb, Bc_Bb_A) |

R1 leaves every CA/port cell wrong (32). R2 leaves q_hchain (comb -> port -> comb: the held copies share batch 2, the child reads its port
before the CA settle) and the 3 comb->comb cells. Only the R3/R4 family (implicit run after the Active region has drained, waiter armed
from seeding so writes before that trigger normal runs) fixes the CA/port and comb->port->comb cells.

Residue of R4f (3 cells, all comb->comb with the consumer written before its producer and no process write at t0):
s583_a08_comb_chain, q_chaind_A_B, q_chain3_A_Bc_Bb. Both oracles silent there by different mechanisms (iv: implicit runs in reverse
source order; vl: topological settle). Fixing them needs an ORDER rule among implicit runs, and every order rule moves split cells
(reverse -> 3 to iv, topological -> 4 to vl). That axis is the recorded never-chased split (iverilog's own order).

Values (hand-read, S/c3/cmp.txt):
- q_edge_* (spurious NEG / extra @(y) at t0): R2, R2r, R3, R3f, R4, R4f, R4t all give iv=vl values (`POS t=0`, neg=0 pos=1 any=1); R1 fixes the
  same-module ones only. WAIT0 (vl releases because 2-state y starts 0): every rule = iv (no-oracle on vl).
- q_read_comb_init r0: iv xx, vl 00 (2-state default, no x-oracle), PRE 00 -> all rules xx. q_read2_comb_w_r x0: iv xx, vl 01, PRE 00
  (matches neither) -> all rules xx (= iv).
- Constant comb read by an initial in its first slice (the row's never-chased split): v01/v03/q_const_comb_first line 1: iv xx, vl 11,
  PRE 11 -> R3/R4/R4f/R4t xx (moves to iv). This move is forced by the fix: the 2-oracle cell a_uc_combfirst needs the comb's t0 pass
  after the initial's first slice, and line 1 is read in that slice. PRE is text-order dependent there (q_const_init_first PRE line 1 = xx);
  R4f is not. Line 2 (`#0`): iv xx, vl 11: R4/R4f keep PRE (11), R3 moves to iv. Line 3 agree everywhere.
- q_const_latch_first: iv rejects the design (`always_latch process has no event control`); vl 10 (2-state, cannot show x); R4f line 1 xx: no oracle.
- Self-timed `always` (q_self_*, s583_v04): byte-identical under every rule (not spelled; keeps Comb seed in batch 1).
- s583_r01_order (t5 report vs #0 / NBA / $monitor / $strobe order): identical to iv under every rule.
- q_assert_noviol_combfirst exit code: PRE rc=1, R4f rc=0 (= iv, vl).

Chosen rule: R4f. always_comb / always_latch: (1) arm the static level waiter at seeding, exactly as `SensKind::Level` is armed
(scan_arm.rs:1452 `arm_sensitivity`; native: `WakeTable::new` level_armed), so a write before the implicit run triggers an ordinary run;
(2) the implicit time-0 run is queued into the Inactive queue at seeding (seed seq, tie order), so it runs at the head of the first #0
promotion: after every initial/always has run its first slice, its writes propagated and continuous assigns / ports settled, before any
#0 continuation and before NBA. A self-timed `always` keeps today's Comb seed in batch 1.
Cells that fix it: 36 two-oracle + 6 vl-only (list in Q4). Descents: 0 (all 45 two-oracle and 15 vl-only cells PRE already got right stay
right under R4f), and no split cell moves on reports. Value moves only on the forced constant-comb line-1 split (above).
Row 7 (parent initial reading a child net at t0): see Q3b.
### Q3b §2 row 7 (S/c4/row7_*.sv, raw .out; spelled under S/c4/sp/<rule>/)
| cell | iv | vl | PRE | R4f / R4 / R3 spelling |
|---|---|---|---|---|
| row7_initial (child `initial s = 8'hEE`, parent `r = u1.s` at t0, then `#0`) | ee / ee | ee / ee | xx / ee | xx / ee (unchanged: no comb in the cell) |
| row7_comb (child `always_comb s = 8'hEE`) | xx / xx | ee / ee | xx / ee | xx / xx (spelling artifact, see Q4) |
| row7_comb_in (child `always_comb s = d ^ 8'hE0`, parent writes d first) | xx / ee | 00 / ee | xx / ee | xx / ee |
No candidate rule moves row 7's headline cell (only initials). R4f-as-implemented keeps row7_comb at PRE (xx / ee); the spelling cannot.

## Q4 PRE spelling — DONE
R4f spelled on PRE: replace each `always_comb` / `always_latch` keyword by `always @*` in place (level waiter armed at seeding, no seed
run: `always @*` is SensKind::Level, events.rs:418) and insert one `initial #0 <same body>` per comb, in source order, BEFORE every other
process of its module (its `#0` executes first in batch 1, so its continuation has the smallest seq and runs at the head of the first
#0 promotion). Generate scopes: copy inside the generate block (hand-spelled q_gen_comb_first).
PRE-on-spelling == both oracles (t0 report presence; all 3 backends byte-identical) on the 36 two-oracle GAP cells:
a_{uc,pc,lc}_combfirst; b_uc_{v,w}_d{1,2,3}_{tbfirst,leaffirst} (12); b_{pc,lc}_v_d1_{tbfirst,leaffirst} (4); q_assert_combfirst, q_assert_hier,
q_assert_noviol_combfirst (rc 1 -> 0); q_chain_B_A_I; q_gen_comb_first; q_hchain; q_read2_comb_w_r; q_read_comb_init; q_read_hier;
q_src_alwayswr_combfirst; q_src_ca_comb_ca_init; q_src_ca_init_ca_comb; q_src_caop_init_comb; s583_a00t_repro_trace; tb1_leaffirst;
tb1_samemod_assign; tb1_tbfirst. Plus the 6 vl-only GAP cells (sa_ui_combfirst, sa_li_combfirst, sb_ui_v_d1_{tbfirst,leaffirst},
sb_ui_v_d2_tbfirst, sb_li_v_d1_tbfirst) and the q_edge_* values (neg=0 pos=1 any=1, `POS t=0` only).
Hierarchy: the same module-local spelling works inside the child (b_*, q_hchain, q_read_hier, q_assert_hier, q_edge_hier fixed), because
the child's comb is triggered by the port's CA settle through its armed waiter. Limitation: the child's `initial #0` copy has a higher
ProcId than every parent process, so a PARENT `#0` continuation runs before the child's implicit run in the spelling, whereas the
implementation (implicit runs queued at seeding with the seed seq) runs every implicit run first. Only row7_comb (split cell) shows it.
## Q5 code census — DONE (HEAD 63b9dbb9; file:line)
ProcId order: elaborate/src/instance.rs:1194-1215 pass (7) lowers THIS module body's processes in source order (`push_process`,
stmt_flow.rs:40-74); instance.rs:1336-1360 pass (8) recurses into child instances afterwards => every parent process has a lower ProcId
than every child process, whatever the module text order (measured: tb1_leaffirst runs the initial before the leaf's comb).

Producers of the kind (elaborate/src/events.rs `lower_sensitivity` :348-428, called from const_level_header.rs:158 `proc_sensitivity`):
| kind | producer | construct |
|---|---|---|
| Comb | events.rs:363-366 | `always_comb` |
| Comb | events.rs:375-381 | self-timed `always` (no header, in-body timing) — body forever-wrapped, stmt_flow.rs:451-453 |
| Comb | events.rs:386-393 | `always` with neither header nor timing (warned, inert; body runs once at t0, never re-armed: empty edges) |
| Comb | events.rs:447-452 | `classify_event_list` Star/None with !force_edge — unreachable today (callers :371 force=true, :423/:439 pass a List) |
| Latch | events.rs:367-370 | `always_latch` (only producer) |
| Level | events.rs:418-421 | `always @*` |
| Level | events.rs:498-503 | `always @(a or b)` (no edge term) |
| Level | const_level_header.rs:190 | header level list naming a constant + t0 pulse net (const_level_header.rs:264 `t0_pulse_net`) |
| (edges) | stmt_flow.rs:470-495 | read-set inference for always_comb/always_latch/`@*` (`comb_inferred_procs`, lib.rs:534; re-done after hier resolve: hier.rs:927-940) |
Source-kind identity already exists in elaborate: `proc_idents[pid].kind == "always_comb"` (tables.rs:169-192, lockstep stmt_flow.rs:66;
used by hier_defer/func_call.rs:201-206 precisely because SensKind::Comb is shared with a self-timed always). It reaches cli obs only
(api.rs:180 -> frontend.rs:676 -> obs.rs:156), NOT sim-engine and NOT the staged artifact.

Consumers (match arms on SensKind), production code:
| site | what it decides | Comb/Latch vs Level today |
|---|---|---|
| sched/scan_arm.rs:1447-1453 (`arm_processes_after_seed`) | t0 seeding, interp + vm | Comb/Latch -> pushed to `cur.active` (seed batch); Level -> `arm_sensitivity` |
| sched/scan_arm.rs:1576-1597 (`arm_sensitivity`) | static waiter registration | Level/Comb/Latch identical (Level waiter over edges; empty edges = none) |
| sched/propagate.rs:661-672 (`rearm`) | re-arm after Return | Comb/Latch/Level identical; Edge/Initial none |
| native/run.rs:1121-1144 (`arm_t0`) | t0 seeding, native tier-3 | Comb/Latch -> `k.active`; Level/Edge nothing (static) |
| native/wake.rs:120, :138-148 (`WakeTable::new`) | static edge / level tables | Level/Comb/Latch identical |
| native/wake.rs:160-168 `level_armed` | t0 arm state | true only for Level with edges (Comb waiter exists only after first run) |
| native/kernel.rs:2329-2334 (`k_rearm`) | native re-arm | Comb/Latch/Level identical |
| levelize/mod.rs:101-104, :550-551 | level/edge read sets (rank, measurement) | Comb/Latch/Level identical |
| levelize/mod.rs:590, :687 `matches!(Comb)` | fusion_candidates (measurement only; perf_baseline.rs) | Comb only |
| state/changes.rs:1601 | edge-target marking | Edge only |
| elaborate/hier.rs:931-934 | read-set recompute `with_fn` | Comb/Latch (only for comb_inferred_procs) |
JIT (`--features jit`): jit.rs has no SensKind/seeding; `run_body_jit` (jit.rs:1291) executes bodies only; scheduling is native/run.rs
(:68, :180) or the Scheduler (lib.rs:1100/1134 stats hooks). Not built, not measured.

Frozen-type status: `sim_ir::SensKind` (sim-ir/src/lib.rs:736-743) derives Serialize/Deserialize/SchemaHash and is reached from
`sim_ir::SimIr` via `Process.sensitivity: Sensitivity { kind, edges }` (lib.rs:753-757) -> covered by the SimIr schema hash
(crates/testdata/sim_ir_canonical.txt, sim_ir_registry.ron) and CURRENT_FORMAT_VERSION (vita-artifact/src/header.rs:15, now 34).
No .velab/.vu is checked in (git ls-files).

Cost of the discriminator:
| option | change | format / goldens | sites to edit | risk |
|---|---|---|---|---|
| (a) new SensKind variant (e.g. `SelfTimed` for the two `always` producers) | sim-ir enum + events.rs:378/:390 | FROZEN-ROOT: schema hash + canonical + RON re-pin, format_version 35, obs.rs field re-pin | every consumer above; exhaustive `match` (scan_arm 1447/1585, propagate 666, run.rs 1124, kernel 2330, levelize 101/550) is compiler-checked; `matches!`/`==` (wake.rs 120/140/167, levelize 590/687, changes.rs 1601, hier.rs 933) are not | lowest semantic risk, highest churn |
| (b) retag the two `always` producers (events.rs:378, :390) as `SensKind::Initial` | value only | no type change -> goldens untouched; bump format_version anyway so a stale artifact's `Comb` (meaning either) is refused | events.rs 2 lines + the R4f arms | Initial vs Comb-with-empty-edges are equivalent at every consumer above (seed batch 1, no rearm, empty read sets); unmeasured: suite + corpus with the flip |
| (c) side table `ProcId set` of always_comb/always_latch, threaded like `final_procs` | elaborate lib.rs:522 twin, stmt_flow.rs insert, api.rs:188, cli frontend.rs:733, sim-engine SimOpts lib.rs:381/:889, st field state/mod.rs:509, staged pipeline.rs:927 + staged.rs:515 (StagedExtraSidecars tail), native/mod.rs:235 + native/tests.rs:426 destructures | artifact wire-shape: format_version 35, `staged_extra_sidecars_wire_shape` fixture re-pin; SimIr goldens untouched | ~10 | STAGED-DROP if the trailer is missed (staged vrun = PRE behaviour) |

Lane table for R4f (both seeding sites must change identically):
| lane | site | measured |
|---|---|---|
| interp | scan_arm.rs:1447-1453 seed + run_loop.rs:224-251 #0 promotion | yes (spelling on PRE; native=interp=vm byte-identical in all 121 + 595 + 119 spelled runs) |
| vm | same Scheduler (lib.rs:1132) | yes (same) |
| native tier-3 (default) | run.rs:1121-1144 seed, wake.rs:160-168 level_armed, run.rs:526-545 #0 promotion | yes (same) |
| JIT | shares native/Scheduler seeding | unmeasured (feature off) |
| staged vcmp->velab->vrun | (a)/(b): kind inside SimIr; (c): new trailer field | unmeasured (needs `--features separate-bins`) |
| take_t0_wakes (run_loop.rs:25-40; native twin) | settle wakes of an armed comb are held to batch 2 | yes (the `always @*` spelling is armed at seeding and takes the same path) |
Precedent for "Level + a time-0 run": const_level_header.rs:177-195 (`t0_pulse_net`, R2-shaped: held to batch 2) — measured insufficient
for this row (R2 misses q_hchain and the comb->comb cells).
## Q6 ibex / corpus — DONE
Corpus checkout: <repo>/bench (gitignored; `resolve_bench_root`, corpus-runner/src/main.rs:22). Row args: corpus.rs:470-500 (ibex:
`--top tb -DSYNTHESIS -DDV_FCOV_DISABLE -I...` + 63 files, plusargs `+N=20000`, pinned `DIGEST=13b2ddfcd551ba2f`). S/ibex_cmd.txt = the same args.
always_comb / always_latch per corpus design (bench/*, .sv/.v, text count): ibex 427 / 17 (113 files in the tree); darkriscv 2 / 0, only in
`src/boards/de10nano_cyclonev_mister/sys/f2sdram_safe_terminator.sv`, which the darkriscv row does not compile (its files: tb2.v, darkriscv.v,
darkram.v); every other row 0 / 0. => ibex is the only corpus row with always_comb / always_latch.
ibex row file list (63 files, comments stripped): always_comb 152, always_latch 8, unique/unique0/priority case|if 122, `assert (` 0,
`ASSERT*( macros 246 (compiled under -DSYNTHESIS). All 160 comb/latch items are `always_comb|always_latch begin [: label] ... end`.

PRE on ibex (S/ibex/pre.out, pre.err; wall 29 s; cwd bench/ibex):
  rc=0 / `DIGEST=13b2ddfcd551ba2f` / `simulation ended (Finish) at time 3000165000`; stderr: 1 W1018, 36 W3056, 6 W4029; 0 W4031, 0 E4003.
  Lines `[at time 0]`: 4, all `warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of
  `tb.u_top.gen_regfile_ff.register_file_i.g_plain_rf.rf_reg` is unknown (x/z); read X / write ignored [at time 0]` (a continuous-assign
  read `rf_reg[raddr]`, vita-only diagnostic). No $display from a comb at t0 (stdout is 2 lines).
R4f spelled on ibex (S/ibex/spell_r4f.py; 160 items -> `initial begin #0; <copy, labels renamed __t0> end` + `always @*` in place; S/ibex/r4f/):
  rc=0, stdout byte-identical to PRE (`cmp` equal: DIGEST=13b2ddfcd551ba2f), stderr identical after normalising line:col (same 6 W4029, 4 at t0).
  Control (unchanged copy, S/ibex/r4ctl/): stdout identical to PRE.
  Caveat: `always @*` infers its read set without callee-function reads (stmt_flow.rs:478-484 `with_fn` only for always_comb/latch), so the
  spelling is not a perfect R4f; the digest still did not move. ibex has no `#0` at t0, so the copy placement (before each comb, not module top)
  is equivalent there.
First spelling attempt (`initial #0 begin : l__t0 ... end`) hit a PRE-EXISTING LOUD defect, outside the fix path:
  a `for (int ...)` loop variable inside the statement of a delay control is E3010 `undeclared net/variable top.__forvar_i_N` —
  S/c5/fv1_delay_block.sv (`initial #0 begin for (int unsigned i...) ... end`) and fv3_delay_for.sv (`initial #0 for (...)`): vita rc=1, iverilog z=5;
  fv2_delay_stmt.sv (`initial begin #0; for (...) ... end`) runs (z=5) on both. Loud, not silent -> PROBE_CATALOG candidate.
Other corpus rows: no always_comb/always_latch, so R4f keyed on those kinds cannot move them. Option (b) (retag the self-timed `always`
as Initial) touches every testbench clock generator: it needs the full suite + corpus run (claimed neutral by census only).
## Open questions
1. Residue under R4f (3 two-oracle cells: s583_a08, q_chaind_A_B, q_chain3_A_Bc_Bb — a consumer comb written before its producer comb, both fed
   only by declaration initializers). Needs an order among implicit runs; reverse (R4) moves 3 split cells to iv, topological (R4t, levelize
   rank exists: levelize/mod.rs `comb_ranks`) moves 4 to vl. Never-chased split axis -> file as a residue row, do not build in this slice.
2. Discriminator: (a) new SensKind variant (frozen root, format 35, golden re-pin), (b) retag the two self-timed `always` producers as Initial
   (value-only; census says neutral; needs suite + corpus; bump format anyway), (c) ProcId side table like final_procs (trailer, format 35).
3. JIT (`--features jit`) and staged (`--features separate-bins`) lanes are unmeasured; both reuse the edited seeding sites (no own site).
4. Spelling cannot express "implicit runs ahead of a PARENT's #0 continuation" (row7_comb split cell differs between spelling and the
   intended implementation): measure row7_comb on POST (expected = PRE: xx / ee).
5. ibex spelling used `always @*` (no callee-function reads in the read set); digest identical anyway. POST must re-run the corpus row.
6. IEEE §9.2.2.2 wording taken from secondary sources; LRM text not read in this session.
7. Pre-existing LOUD find outside the fix path: `for (int i...)` under a delay control (`#0 begin for ... end`, `#0 for ...`) is E3010
   (S/c5/fv1, fv3; fv2 control runs) -> PROBE_CATALOG.
8. Pre-existing split not moved by any rule: self-timed `always begin @(a) ... end` written AFTER `initial a = 1` (q_self_atwait_init_alw:
   iv n=1, vl n=0, vita n=0).
9. Comb evaluation counts at t0 change under R4f (triggered run + implicit run; a00t, tb1, o05 now equal iverilog's count, vl differs):
   not an oracle, but any test pinning a comb's t0 $display count will move — grep the suite on POST.
