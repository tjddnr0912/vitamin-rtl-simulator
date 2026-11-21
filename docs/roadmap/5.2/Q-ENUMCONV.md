(§5.2 Q-ENUMCONV · rank — · source row 33 lens2-diff (W/row33/lens2-diff/d5)) (an implicit conversion to an enum is accepted):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* an implicit conversion to an enum is accepted (IEEE 1800-2017 §6.19.3): b_mod_nat `R 1 1`, b_scalar `R 1`, b_frame_scalar `R 1`
* verilator %Error-ENUMVALUE, iverilog refuses
* PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt)
* face (2026-11-19, batch 6, §5.2 Q-LOSSYSPELL's premise judge 2): a fill and a sized x literal assigned to an enum variable: p8b_enumstr (W/lossy/premise2/cells/p8b_enumstr/t.sv), `typedef enum logic [69:0] { E0 = 70'd0, E1 = 70'd1 } e_t;`, `e = 'x; ec = 70'bx;`: vita 54bc2791 rc 0 `A e=xxxxxxxxxxxxxxxxxx ec=xxxxxxxxxxxxxxxxxx ...`; verilator 5.054 `%Error-ENUMVALUE: t.sv:5:9: Implicit conversion to enum 'enum t.e_t' from 'logic' (IEEE 1800-2023 6.19.3)` and `%Error-ENUMVALUE: t.sv:5:18: Implicit conversion to enum 'enum t.e_t' from 'logic[69:0]' (IEEE 1800-2023 6.19.3)`; iverilog 13.0 `t.sv:5: error: This assignment requires an explicit cast.` (twice); the same cell's string assignments are §3.b string-integral-assign-no-cast
