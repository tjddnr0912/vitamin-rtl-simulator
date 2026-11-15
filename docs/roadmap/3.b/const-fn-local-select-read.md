(§3.b const-fn-local-select-read · OPEN · in Constants / parameters) (a select read of a constant-function local declines):
* symptom: LOUD: a part-select or bit-select read of a constant-function local (`b[32:1]`, `x[11:4] + x[a]`) is E3009
* witness: f04_w33 `logic [32:0] b; b = -a; return b[32:1];`, `f(3)`, and f39_select_read_local `logic [15:0] x; x = 16'hABCD; return x[11:4] + x[a];`, `f(0)`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog, sv2v → iverilog and verilator `A P=-2` (f04), `A P=189` (f39) (W/batch4/lens-diff/oracle/f04_w33, f39_select_read_local)
* note: batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* witness (Q-CONSTSELLSB park, W/constsellsb/plan4/m1, m2): c4_for_shift, `for (logic [5:2] i = 4'd0; i < 4'd3; i++) s = s + 8'(i[3:2]);` in a constant function, c4_for_zero_sel (`[3:0] i`, `8'(i[1:0])`) and c4_body_shift_sel (`logic [5:2] i;` declared in the body): PRE rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
  * oracles: iverilog and verilator `V P=00000011` on each
* variation: c4_for_shift_nosel, c4_for_shift with `8'(i)` for `8'(i[3:2])`: PRE rc 0 `V P=00000011`, as iverilog and verilator; so the for-init declaration is not what refuses, the select read of the local is
* witness (Q-CONSTSELLSB premise judge 1, W/constsellsb/premise/q3/cells/q3c_asc_part_read): `bit [0:3] t;` `t = 4'b1100;` `f = {t[0:1], t[2:3], 4'b0000};`: PRE rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
  * oracles: iverilog and verilator `A P=11000000`
* status: OPEN
