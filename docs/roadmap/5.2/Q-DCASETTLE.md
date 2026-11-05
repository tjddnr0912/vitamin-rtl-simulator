(§5.2 Q-DCASETTLE · rank ① · source Q-RELDELAY r1 probes O3a O3d; r1 lens-diff F2 (L26 M18)) (a delayed continuous assign's right-hand side is evaluated on every settle):
* symptom: a delayed continuous assignment's right-hand side is evaluated on every settle, not on an operand change (`schedule_delayed_cas`), so an effectful callee in it runs once per settle: `assign #3 y = f(a);` with a `$display` in `f` and an `always @(y)` prints `F` 11 times on PRE (O3a), iverilog and sv2v once, verilator 3 times
* witness: an unrelated wake adds a call (O3d, no force)
* witness: since Q-RELDELAY the release's restore event (`W t=7 y=0001`, all three oracles) is one more such settle (12; E39 `--obs-procs` assign evals 13 → 14)
* witness: the evaluation also moves values: a `$random` in the delayed right-hand side draws once per settle, so a later procedural draw differs (M18, no force: vita `S r=1e8dcd3d`, iverilog = sv2v `c0895e81`; L26 with the release: PRE `d513d2aa`, Q-RELDELAY `72aff7e5`)
* note: Q-NANDCA is the same evaluation seen through NaN
* status: pre-existing
