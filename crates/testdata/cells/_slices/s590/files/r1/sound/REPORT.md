# §4.5.590 round 1 — SOUNDNESS lens REPORT (final)
round: 1 · PRE2 s590/pre2/vita md5 8684cc7c… · POST s590/post_a/vita md5 fab0a3a1… (release, frozen, not rebuilt)
cells: s590/r1/sound/cells/*.sv (13) + cost/*.sv (5); outputs out/*.out, cost.log; runner run.py, cost.py
lens verdict: FAIL = fix in slice (F2 performance), NOT revert. No BLOCKING value regression found.

## Findings (most severe first)
F2 MAJOR (new; performance, not value) t0_hold.rs:158-185 (preds = full transitive upstream held set via BTreeSet DFS through ALL drivers) + :233-256 (next_wave rescans every pending CA's preds every wave).
  Memory O(N^2), time O(N^3) on a held chain; O(N^2) on a shared vector net. Interleaved PRE2,POST,POST,PRE2 (warm-up dropped), wall s:
  chain_1000 interp 0.01/0.14 · chain_2000 0.02/0.89 · chain_4000 0.03/6.3 (native same) · chain_8000 PRE2 0.05 s 41 MB RSS vs POST 47.7 s 189 MB RSS · vec_4000 0.10/1.47.
  Growth x6.4, x7.1, x7.6 per doubling (cubic). Values identical (POST: f t=0 x=0, t1 wN=0). Real-design exposure UNVERIFIED (corpus holds 0 CAs).
  Cheaper equal rule (parent must re-measure, D7): direct held preds suffice — closure invariant: no non-held CA reads a held net, so the DFS through non-held drivers reaches no held CA; Kahn in-degree waves give the same wave sets incl. the cycle fallback.
F1 MINOR (same root class as the native loop-top fatal row; new axis) native/run.rs:403-413 polls call_fatal only AFTER the loop-top settle; engine polls before AND after (run_loop.rs:185, :201). With no first batch, begin_release fires at run start (run.rs:368) so native runs the release before ending.
  sd01a ($finish in exempt f1 -> F4004, no processes): iverilog `f2 t=0 x=0` + `$finish called at 0` · verilator `f2 t=0 x=0` · PRE2 all backends `f2 t=0 x=x`,`f2 t=0 x=0` (native end Quiescent, interp/vm Error) · POST native `f2 t=0 x=0` Error | POST interp/vm no f2 line, Error. BACKEND SPLIT in POST (stdout) where PRE2 split on end reason.
  sd01b ($fatal): iverilog `f2 t=0 x=0` + FATAL · verilator FATAL only (oracle split) · POST native `f2 t=0 x=0` | interp/vm none. Loud (rc=1) on every backend; comment run.rs:813 "at the same loop-top position" overstates parity.
F3 MINOR (test teeth) no pin for: effectful callee via pure wrapper (sd06), two-level downstream closure (sd07), lhs-index call (sd08). POST is right on all three (= oracles / verilator); mutants MS1-MS3 NO_RESULT (ENOSPC, below).
F4 side, pre-existing (PRE2==POST): sd05 static function local `integer c = 0;` -> E3010 "undeclared net/variable `top.c`"; iverilog `t1 y=1`; verilator DIDNOTCONVERGE. Not this slice.

## Cleared (measured)
sd02a $fatal / sd02b $stop in first batch: POST `final y=z` = iverilog; no x-run (PRE2 `f t=0 x=x`, `final y=x`). sd03 always_comb / sd04 always @* / sd07 two-level readers: POST = iverilog (f once, NV, V; no PV); verilator PV,V (2-state). sd06 POST = both oracles. sd08 POST idx once (verilator idx once; iverilog refuses non-constant lhs index). sd09 force/release: t0 x-run gone, t1 re-call pre-existing. sd10 always_comb input: PRE2=POST=iverilog (x then 0), verilator once: split, unchanged. sd01c (with first batch): native=interp=vm.
Source census: settle callers 3+3 (none inside a batch); begin_release 2+2; comb passes cannot be the first batch; uncertified callee => not free => held (closes incomplete read set); certified computed before build on both backends (scan_arm.rs:634 -> :638); X-DROP cannot touch VCD (changes emitted at write, state/changes.rs:41); release once (finish() clears active).

## Mutants (expected in battery.sh before run)
MS1 effect-free callee propagation off (call_deps.rs:208) expect SURVIVE · MS2 closure one pass (t0_hold.rs:159) expect SURVIVE · MS3 lhs index unwalked (t0_hold.rs:94) expect SURVIVE.
Result NO_RESULT: workspace rebuild in cloned target filled the data volume (ENOSPC, 0 B free); MS1 build errors, MS2 rc=101 truncated, MS3 not applied. target_rv_sound + mwt removed; not re-run (D10; 6.2 GiB free). Structural: grep of new test file finds no wrapper call, one-level downstream only (line 424), no lhs-index CA.
