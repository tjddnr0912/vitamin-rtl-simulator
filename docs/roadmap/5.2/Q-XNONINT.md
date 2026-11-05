(§5.2 Q-XNONINT · rank ② S · source row 26 r2 X1 / F-Q2; Q-STRBYTES re-plan D2) (x-condition `?:` bit-merges string/real arms):
* slot: PARKED behind Q-WITHREAL, 2026-10-28: r2 lens-diff F1 BLOCKING, PLAN2 stop rules S12 / S7'
* slot: its r2 patch stays in W, `W/qxnonint/r2/qxnonint-r2.patch`, and r3 keeps the `merge_ambiguous` hunks keyed on plain realness, dropping `WidthTable::trusted_real`
* slot: its real half — the string half takes slot 13 after row 74
* slot: Q-WITHREAL parked 2026-10-29 behind row 74 and Q-LOCSTALE, so the real half waits on Q-WITHREAL → row 74
* a `?:` whose condition is x / z bit-merges a string or real result (`sim-engine/src/eval/eval_core.rs:706–711`, `Tri::Unknown` → `merge_x` whatever the type), where IEEE 1800-2017 §11.4.11 (ieee1800-2017.txt:11914–11917, 11964–11971) returns an arm when the arms are equivalent and Table 7-1's default otherwise (`""`, 0.0): real `c = 1'bx; rr = c ? ra : ra` (ra = 1.5) prints `4609434218613702656.000000` (1.5's bits read as an integer), iverilog 13.0 = sv2v → iverilog = verilator 5.052 `1.500000` (also `c ? 1.5 : 1.5`, and z; the PROBE_CATALOG ENG-1c row)
* string (no 4-state oracle: iverilog refuses the arm types or aborts in `eval_vec4.c:1360`, verilator is 2-state; hand IEEE) `v = "QQ"; r = c ? v : "PP"` `[PP] 2` (hand `[] 0`), `v = "PP"; r = c ? v : s2` `[] 0` (hand `[PP] 2`), `c ? v : s3.substr(0,1)` `[P] 1`
* 87 of 126 x-lines with a string-valued arm wrong over 19 legal arm kinds, no `$sformatf` involved
* PRE 9d60771d, every lane
* fix: a real result keyed on the node's realness, a string one on an arm VALUE being a string (the other arm converted per §6.16, ieee1800-2017.txt:4464–4465), census first of the Ternary evaluators (eval_core, native_eval/compile, native/wprog, native/frames, backend, state/changes) and of the 25 `from_str_bytes` producers
* declaration-typed pairs with no string value (`c ? P : LQ`, PRE `[PP] 2`, hand `[] 0`) stay for Q-STRTERN
* NOTE: 28 IEEE-illegal integral-arm x-lines (`c ? v : 16'h5050`, `16'("PP")`, a `logic [15:0]` variable, `c ? $sformatf(..) : 8'h41`) change from the bit merge to `""` with no oracle (silent → silent; refusing them is Q-STRTERN's P-5)
* row 26's arm axis (X1, F-Q2) is this row
