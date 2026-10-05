# §4.5.590 round 4 (delta) — SOUNDNESS lens REPORT
round: 4 · POSTE s590/post_e/vita md5 e776d356… · POSTD s590/post_d/vita · PRE3 s590/pre3/vita
status: IN PROGRESS
## Pre-registration (before any r4 run)
cells (7): r4a held-uncertified (HU, `$random` callee) w2=h(b) whose BODY reads w1 written in a later wave, observer always @(w2) + t1 + final · r4b same, final only (no activity after t0) · r4c HU delayed · r4d HU multi-driver member · r4e control: HU reading the later-wave net as an ARGUMENT (Kahn orders it) · r4f two HU in one wave, mutual body reads · r4g stale HU output feeding a held reader. Old cells r1/r2/r3 (31) without oracles for movers POSTD -> POSTE.
expect: r4a/b/c/d/g may keep a stale value after the release (own-wave re-run only; HU read sets omit body reads); stale persists past t0 when no settle follows the release (r4b final). r4e/r4f POSTE == POSTD.
mutant MWM: next_wave `self.wave_md = wave...collect()` -> accumulate (never cleared): multi-driver groups of earlier waves re-resolve in later waves. Predict narrow SURVIVE (no pin counts multi-driver calls across waves), killer r2a (f count).
## Results (final)
verdict: FAIL — BLOCKING F1 (value regression introduced by the round-4 own-wave rule). Round 4 = D8 scope signal.
F1 BLOCKING, new instance of the r2 F3 root class (a declined callee's body reads are not in ca_deps' read set). A held uncertified assign re-runs only in its own wave, so when its callee BODY reads a net a later wave writes, its value stays computed on that net's default after the release. The X-DROP removes the `z->x` hop, nothing wakes, and the value is corrected only by the next unrelated settle, or never:
  r4b final: POSTE `final w1=0 w2=x`; PRE3/POSTD `w2=0`; iverilog `w2=x`, verilator `w2=0`.
  r4a: POSTE `W2 t=1 w2=0` (h never runs with w1=0 at t0); PRE3/POSTD/verilator `W2 t=0 w2=0`; iverilog no W2 event (w2=x throughout).
  r4c (held delayed): POSTE `D t=2 d=0`; PRE3/POSTD `D t=1 d=0`; verilator `D t=0 d=0`; iverilog no D event (d=x).
  r4d (multi-driver member): POSTE `N t=3 n=0`; PRE3/POSTD/verilator `N t=0 n=0`; iverilog `N t=0 n=x`.
  r4g (stale value feeding a held reader): POSTE `g t=0 x=x`, `g t=3 x=0`; POSTD `g t=0 x=x`, `g t=0 x=0`; verilator `g t=0 x=0`; iverilog `g t=0 x=x` only.
  Every POSTE event at t>0 above is in neither oracle. Controls: r4e (later-wave net read as an argument: Kahn orders it) and r4f (two uncertified assigns in one wave) give POSTE == POSTD.
  Candidate fix (re-measure, D7): own-wave only for held uncertified assigns whose read set is complete (no declined callee); per-wave, or one closing pass over held_always after the last wave, for the rest.
F2 MINOR, new instance (test teeth): MWM (wave_md accumulates = post_d's multi-driver lane) survives --workspace 9146/9146; r2a t0 f 4 -> 8 and r3e 5 -> 11 (= POSTD counts). The r5h pin covers only the always lane.
Q2: wave_always / wave_md / wave_delayed are reassigned on every next_wave call (together wave included); unused after finish() clears in_waves. Determinism: 7 cells x 3 backends x 3 runs, 0 non-deterministic.
Movers POSTD -> POSTE (31 old cells, 3 backends): r2a t0 f 8 -> 4 (PRE3 14), r2b 4 -> 2 (7), r3e 11 -> 5 (17), deletions only; 28 unchanged.
