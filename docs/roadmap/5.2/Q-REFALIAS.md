(§5.2 Q-REFALIAS · rank ① · source ENG-2 r3 B2 re-plan OP-2 (RTWO, RTWO_tw), r4 design review N-REFJN) (two `ref` formals bound to one variable keep only the last copy-out):
* symptom: two `ref` formals bound to one variable keep only the last copy-out: `t(g, g)` whose task writes `x[1]` and `z[2]` leaves `g=0100` where passing by reference (the local twin, verilator) leaves `g=0110`
* status: value-level, pre-existing (PRE = POST)
* IEEE 1800-2017 §13.5.2 :14775-14776 ("a reference to the original argument is passed")
* fix: alias the formals of one actual, or copy out per written region
* symptom: a `ref` formal written after a timing control in a `join_none` arm (illegal, §9.3.2 :9277-9278; verilator refuses by name) is refused only through the frame classify's E3009 (N12_refjn)
* fix: refuse it by name
