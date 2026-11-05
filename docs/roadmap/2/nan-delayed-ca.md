(§2 nan-delayed-ca · in Oracle splits (recorded, not chased)) (a delayed continuous assign re-evaluated with identical NaN bits):
* a delayed continuous assignment re-evaluated with identical NaN bits after another operand moved (Q-NANDCA, ENG-1 r2 D13) — vita lands at the old time (`t=7`), iverilog and sv2v reschedule (`t=8`), verilator does not converge: vita re-decides a delayed right-hand side on settle passes without an operand change, so identical NaN bits are one value (`value::real_values_same`, ENG-1c) where IEEE 754 says `NaN != NaN`
* the same at step (c) (ENG-1c mutant N5, pin `d13c`: the right-hand side returns to the landed NaN bits while `1.0` is pending — vita drops the pending value, `W t=3 | F t=12`; iverilog = sv2v `W t=3 | W t=13 | F t=13`; verilator does not converge; the non-NaN control k1: vita = iverilog = sv2v t=12)
* following iverilog needs per-assign operand-change tracking
* recorded split
