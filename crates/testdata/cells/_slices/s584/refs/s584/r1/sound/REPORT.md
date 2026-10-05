# s584 lens:soundness round 1 — REPORT (live)
binaries: PRE 2492e1c5 / A caef001a / AB 5d8d3960 (frozen, S/r1)
status: DONE round 1 — verdict FAIL (F1 A, F2 B)

## Findings
F1 BLOCKING (A, NEW): an always_comb/always_latch whose body suspends (vita ACCEPTS `#d`, `wait`, `@` directly or via a task; iverilog rejects, verilator accepts) is activated twice concurrently on ONE activity under A/AB:
 (a) implicit pass queued in t0_implicit regardless of `busy` -> second activation while the first (settle/batch-1 woken) is suspended (cells s2/s3: `C t=0 n=1`,`C t=0 n=2`,`D t=1 n=2` x2; PRE single);
 (b) seeding static Level waiter stays live while the implicit pass is suspended; Level fire has no `busy` gate (propagate.rs:193-216; busy only gates net_to_edge, propagate.rs:96) -> re-entry from entry at t=1 (s5: `C t=1 a=1 n=2` while first in #2);
 (c) with a task frame on the call_stack, run_process ignores bb (process.rs:60 in_frame) -> implicit pass RESUMES the suspended frame: `#1` inside the task completes at t=0 (s1: `D t=0 n=1`; s4 always_latch+@(posedge e): `D t=0 n=1` with no posedge).
 native=interp=vm identical (run.json backend=native, own loop). Cells: cells/s1..s5.
 LEGAL-SV 2-ORACLE DESCENT (f4/f5): `initial a=1;` declared BEFORE `always_comb begin y=a; $display; if (a) $finish; end`: iv=vl=PRE `C t=0 a=1` x1; A/AB x2 (all 3 backends). Control f6 (no $finish): iv x2, vl x1, PRE x1, A x2 (count split) -> the $finish is what makes both oracles agree on x1.
 Earlier LEGAL-SV instance of the same root (implicit pass dispatched ignoring `busy`): f2 `always_comb begin y=a; $display; if (a) $finish; end` + `initial a=1;`
   iv: `C t=0 a=1` x1; vl: x1; PRE: `a=x`,`a=1`; A/AB: `a=1` x2 — the implicit pass re-enters a body that executed $finish, contradicting run_loop.rs:157-162 ("never re-entered"). f3 same (NBA variant).

F2 BLOCKING (B, NEW, 2-oracle descent, Level-only design): b1 `assign w=1; always @(w) s=1; always @(s) $display(v,u); always @(t) u=1; assign v=u; initial t=1;`
   iv=vl=PRE=A `R t=0 v=1 u=1`; AB `R t=0 v=x u=1` (native=interp=vm). B runs the settle wake H alone, its wake WH (fresh wake_seq) rejoins the parked batch-1 wakes [B1] as ONE batch [B1, WH] (rejoin_sorted, run_loop.rs:99-116) -> no settle between B1's write of u and WH's read of v. PRE/A: H led batch 2 with B1, WH ran in batch 3 after the settle.
   Same merge on A's comb passes (b2): iv=vl `v=1 u=1`, PRE `v=x u=x`, A/AB `v=x u=1` (gap kept, not a descent).
N1 NOTE (A, NEW, perf): engine (interp/vm, and native's fallback) t0 cost O(N_passes x N_waiters): propagate scans all waiters (propagate.rs:164-178) once per serial pass.
 measured (single run each, coarse, not D5): p_comb_N interp PRE/AB wall 4096: .13/.13, 8192: .24/.26, 16384: .47/.56 (delta x4 per doubling). native (default) flat: 16384 .48/.47.
N2 NOTE (A, NEW, loud/scale): each serial pass is one delta (run_loop.rs:216; native run.rs batch branch) against the per-timestep max_deltas 1_000_000 (lib.rs:493, no CLI knob) -> >~1e6 comb/latch/UDP (+B: settle wakes) trips DeltaLimit at t0. UNVERIFIED by run.
N3 NOTE (A, NEW, split): GATED-CLOCK reset_edge_seen_marks now runs at t0 in every comb design (run_loop.rs:252 via the t0_implicit clause). g1: E lines iv=`y=x`,`y=1`; vl=`y=1`; PRE=`y=1` (=vl); A/AB=`y=x`,`y=1` (=iv). F lines unchanged (PRE=A=`z=x` x1; iv x2; vl `z=1`): an edge from a WOKEN comb pass is deduped, from an IMPLICIT pass re-fires.
N4 NOTE pre-existing: comb UDP + reader in one batch lags at t>0 (u1: iv=vl `R t=1 a=1 y=0 z=0`, PRE=A=AB `y=1 z=1`) — §2 line-303 class, unchanged.

N5 NOTE (A, NEW, split): implicit-pass WAKES run after the promoted #0 resumes (rejoin order). c3 const comb / c4 decl-init-fed comb + `always @(s) g=s` + `initial #0 $display`: iv `s=x g=x`, vl `s=1 g=1`, PRE `s=1 g=1` (=vl), A/AB `s=1 g=x` (neither).
N6 NOTE: events.rs:383-386 'Every engine consumer treated Comb(empty edges) and Initial alike' — levelize fusion_candidates (mod.rs:590/687, `comb = kind==Comb`) did not; data-only (perf_baseline.rs).
N7 NOTE: rejoin_sorted 'result is push_sorted's whatever the inputs' needs `parked` internally sorted (fast path appends unsorted); holds because cur.active/inactive are only push_sorted-filled (run_loop.rs:461, propagate.rs:541).

## Census tables
static_level_live: set scan_arm.rs:1651; clear propagate.rs:225 (static Level fire), scan_arm.rs:1575 (test consume); init false scan_arm.rs:1169, propagate.rs:958; read scan_arm.rs:1631.
waiters: push scan_arm.rs:1644 (in_body false), propagate.rs:592 (in_body true); remove propagate.rs:193 retain, scan_arm.rs:1569 (test). No disable/kill path removes waiters (dead filtered at run_body scan_arm.rs:1694).
n_level_waiters: +1 scan_arm.rs:1650, propagate.rs:589; -k propagate.rs:228, scan_arm.rs:1574; refresh path no change -> exact.
busy gate: only net_to_edge (propagate.rs:96; native wake.rs:230). Level static fire: no busy test (propagate.rs:164-178; native wake.rs:260-266). Implicit pass: no busy test (run_loop.rs:109-111; native run.rs twin).
SensKind consumers (non-test): scan_arm.rs:1451/1465/1470/1610/1620, propagate.rs:683/685, native run.rs:1150/1154/1161, wake.rs:120/140/172, kernel.rs:2354/2355, levelize 101/104/550/551/590/687, changes.rs:1601, hier.rs:933, events.rs producers 367/371/375/394/407/437/474(dead)/523/525, const_level_header.rs:191. cli: none (obs kind = ProcKind; o1 run.json kinds/evals identical PRE vs AB).
Format pins: obs.rs:73 '35', inline_bind_real_call.rs:219 35; CHANGELOG.md:746-752 still says 34 (release doc).

## Open items
UNVERIFIED: JIT lane (not built); staged vcmp->velab->vrun + v34 refusal (not built); 3-OS identity (macOS only; no hash iteration in diff); N2 by run; perf numbers single-run.

# ROUND 2 (delta) — started
binaries: PRE 2492e1c5 / POST2 S/r2/vita_post2 28f38a3e (frozen)
## r2 Q1: r1 cells on POST2 (native=interp=vm on every cell)
| cell | PRE | r1-A | POST2 | oracle | verdict |
|---|---|---|---|---|---|
| s1 task #1 | C a=x,D t=1,C t=5,D t=6 | D t=0 (zero-time #1) | C a=0,D t=1,C t=5,D t=6 | iv rejects; vl single | F1c fixed |
| s2 / s3 | single | double concurrent | single | iv rejects | F1a fixed |
| s4 latch @(posedge e) | L a=x, D t=7 | D t=0 (no edge) | L a=0, D t=7 | iv rejects | fixed |
| s5 refire while busy | C,D t=2 | C t=1 re-entry | = PRE | iv rejects | F1b fixed |
| f2 / f3 | 2 lines (a=x first) | 2 lines | 1 line | iv=vl 1 line | fixed (now = both) |
| f4 / f5 | 1 | 2 | 1 | iv=vl 1 | 2-oracle descent fixed |
| f6 control | 1 | 2 | 2 | iv 2, vl 1 | count split unchanged |
| b1 | v=1 | v=1 | v=1 | iv=vl v=1 | F2 gone (B dropped) |
| b2 | v=x u=x | v=x u=1 | v=x u=1 | iv=vl v=1 u=1 | gap kept |
| g1 | E y=1 | E y=x,y=1 | = A | split | N3 stands |
| c3 / c4 | = vl | neither | neither | split | N5 stands |
| c1 | u=x | u=1 | u=1 | iv u=1, vl u=0 | split |
| c2, k1, o1 | | | unchanged | | |
| u1 comb UDP | y=x z=x / y=1 z=1 | same | same | iv=vl y=1 z=1 / y=0 z=0 | N4 pre-existing, unchanged by the UDP relowering |
## r2 Q2: trigger predicate census (both backends)
busy writers: engine run_loop.rs:274 (Finish), :302 (Suspended), :307 (Done=false), init scan_arm.rs:1167 / propagate.rs:956; native run.rs:498/:534/:539 (base only, r.proc==tmpl), init wake.rs:178. Only runtime dispatch: engine run_loop.rs:234, native run.rs:478 (others = decl-init scan_arm.rs:1231/run.rs:993, finals scan_arm.rs:1677/run.rs:1519).
flag/armed: engine set scan_arm.rs:1651 (arm push), cleared propagate.rs:225 (fire), run_loop.rs take_t0_trigger, scan_arm.rs:1575 (test); native set wake.rs rearm_level, cleared wake.rs:266 (fire), take_t0_trigger, test setter. rearm only at body end: exec/process.rs:357, exec/mod.rs:220, native kernel.rs:2458.
Every enqueue of a comb's entry Ready is a consuming path (fire / settle fire / trigger); cross-process disable is an elaborate error (stmt_main.rs:918); kill only fork descendants. => (busy & live) and (live & queued) unreachable; predicate exact on S0..S11.
Lost pass: a dropped trigger means the block is busy (it ran) or woken-queued (it will run); no base-activity wake is discarded (dead check = descendants; clock handler = Edge). At break t0_triggers is empty (pass None => trigger loop drained).
Cells on POST2: t1 callee-only read set = iv exactly; t2 landing-woken comb: trigger dropped, 1 line (iv 2, vl weird) count split; t4 const comb + $finish 1 line = both; t6 chain = PRE.
## r2 Q3: UDP lowering (initial <cascade> + always @(inputs))
Single builder: udp.rs:396 -> udp_table.rs build_comb_udp; body [init, always] = sequential precedent order (udp.rs:760-761). No UDP-specific consumer in elaborate/cli/engine (grep is_udp/primitive/udp: none). Seq precedent differs: its initial writes the power-on constant, its always has the z<->x folded-change guard + shadows; comb initial evaluates the table.
Cells: u1 PRE=POST2 (pre-existing lag), u2 decl-init fed = iv=vl=PRE=POST2, u3 chain = PRE (pre-existing t0 gap), u4 settle input changed by batch 1 = iv=PRE=POST2. Warnings unchanged (u1 1 = W1017).
obs: u1 processes 3 -> 4; `always_comb top.g1 line1 evals 3` -> `always top.g1 evals 2` + `initial top.g1 evals 1` (same scope/file/line, kind differs); total_evals 18 = 18. NOTE.
udp_table.rs comment '(no change, no event)' is imprecise when batch 1 changes a settle-record input (held always then reads the new input); per-delta port values still = PRE (u4).
## r2 Q4: twin parity — trigger loop, Inactive extend, predicate order (busy -> no read set -> armed) identical; has_level_nets == !edges.is_empty() for every process (wake.rs:126). All 30 cells native=interp=vm on POST2.
## r2 Q5: comment premises — take_t0_trigger doc, run() doc, Step::Finish note (f4 measured), scan_arm 'a re-arm finds none live' (every activation start consumes), mod.rs flag doc (3 clear sites), rejoin_sorted sortedness (fillers census), header v35 text: hold.
## r2 extra: s6 fork-in-comb single activation; s7 `#0` in comb + `initial a=0; #0 a=1;`: POST2 `C a=0, D` only (the a=1 lands in the same promoted batch before the comb's resume re-arms) = the pre-existing Level semantics (`always @(a)` with in-body #0 does the same on PRE); illegal input. NOTE.
N1 update: interp p_comb (interleaved pre,post2,pre,post2): 8192 .24/.32, 16384 .47/.78 (delta x3.9 per doubling; engine trigger adds a position scan + Vec::remove per trigger). native flat .47/.47.
VERDICT r2: PASS (no BLOCKING).

# ROUND 3 (delta, last) — started
binaries: POST3 S/r3/vita_post3 469298289bfacab02e755ec2d18fa855
## r3 Q1: r1/r2 cells (33) POST2 vs POST3: no change on any cell; native=interp=vm on every POST3 cell.
## r3 new cells (7; POST3 native=interp=vm)
| cell | iverilog | POST2 | POST3 | note |
|---|---|---|---|---|
| r3n1 landing-woken comb + #0 reader of `assign v=y` | C w=1, Z v=1 y=1, C w=1 | C, Z v=x | C, Z v=1 y=1 | = iv first 2 lines (trailing implicit = count split) |
| r3n3 superseding pass hits $finish | C w=1 | C w=1 | C w=1 | P0 swept, no re-entry |
| r3n4 superseding pass suspends (#1, illegal) | rejects | single | single | P0 swept |
| r3n5 superseded block re-woken by a later trigger pass | C z=x, D, C z=1, F y=0 | D, C z=1, F | = iv exactly | later re-wake kept |
| r3n6 stale waiter past t0 (2 combs, pass writes nothing) | 1 line per comb per change | = | = | stale never fires |
| r3n7 two superseded, one sweep | C1, C2, Z v=1 .., C2, C1 | Z v=x | C1, C2, Z v=1 | = iv first 3 lines |
| r3n8 superseded block re-woken by #0 write | C b=x, C b=1, F y=1 | = iv | = iv | |
## r3 census
Entry-start producers for a base comb: propagate retain woken -> push_sorted(cur.active); take_t0_wakes combs -> t0_combs_owed -> t0_serial (drained before the first Inactive branch); seeding -> t0_implicit -> t0_triggers (not a start). cur.inactive fillers: schedule_resume (propagate.rs:564), wheel promotion (run_loop.rs:602); native k_schedule_resume (kernel.rs:2309), run.rs:736 = resumes of busy activities. t0_parked rejoined at loop top (run_loop.rs:199-201) before the sweep (:221-223). Held t0_woken excludes combs (is_t0_comb). => the superseded start is the block's only pending start at the trigger, and it is in cur.active at the sweep; any later start for it is a re-wake after its pass re-armed (newer seq) -> oldest = superseded. Census claim holds.
Stale waiter: stale check precedes fire in the retain (propagate.rs:210-216); counted in n_level_waiters until swept (consistent with the Vec); arm_sensitivity / test helpers filter by gen; n_stale_static exact (+1 per bump only when a live waiter existed; only arm_sensitivity pushes static Level waiters, gen = static_gen; one trigger per activity). Past t0 it lingers only until the next propagate with a change (early return when changed_nets empty); r3n6: no double fire.
Twin: same Drop/Run/RunSuperseding order (busy, no read set, armed); drop_superseded_starts identical (BTreeMap owed, retain first n entry starts); native has no waiter Vec (bool) -> no stale concept, no observable difference.
Perf: interp p_comb (interleaved pre,post3,pre,post3): 8192 .24/.24, 16384 .47/.47 -> r1/r2 N1 quadratic gone.
VERDICT r3: PASS (no BLOCKING).
