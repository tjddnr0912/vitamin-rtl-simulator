(§2 velab-cu-dedupe · OPEN · in Diagnostics / artifacts) (`velab -L` drops a second unit's diagnostic at the same offset):
* symptom: staged `velab -L` drops a second compilation unit's elaboration error when its bound sits at the same offset as the first's, rc 1 kept: `reported_x_bounds` / `reported_bad_bounds` key a bound by `(span.lo, span.hi)` (`elaborate/src/lib.rs:1527-1534`), and composed CUs' spans overlap (`cli/src/staged.rs:133-136`)
* witness: mcu2 (a.sv `logic s; logic [s:0] ya; c u();`, c.sv `module c;   logic s; logic [s:0] yc;`, both bounds at 3:10): PRE vita-main 8c970e9b one-shot prints the a.sv:3:10 line `[in top]` and the c.sv:3:10 line `[in top.u]`, rc 1
* PRE's staged `velab -L` prints one line, ``error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `s` is not allowed in a constant range bound [in top]``, rc 1 (W/s1v/plan2/cells/mcu2)
* witness: mcu (`logic [4'bx:0] ya;` in a.sv, `logic [4'bz:0] yc;` in c.sv, both at 2:10) the same on the S1v-R' prototype pRp: one-shot two lines, staged one, `[in top]` (W/s1v/plan2/p1/mcu)
* note: PRE's own lines share both sets (`const_eval.rs:984-988` Some path, `:1004` None path), so the class is pre-existing
* note: sibling: `velab -L` prints every elaboration diagnostic without a position: PRE `vcmp t.sv ; velab -L` on r08_zero_width_after `error[VITA-E3009] E-ELAB-UNSUPPORTED: the width of this indexed part-select is zero; …` `[in top]`, one-shot `t.sv:6:21: error[VITA-E3009] …`
* note: found by S1v premise round 1 (N-4, W/s1v/RULINGS-premise1.txt CARRY; W/s1v/plan2/filings.txt F-N4)
* status: OPEN
