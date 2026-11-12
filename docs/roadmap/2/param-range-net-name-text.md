(§2 param-range-net-name-text · OPEN · in Diagnostics / artifacts) (a localparam's range naming a net calls it an undefined name):
* symptom: a `localparam`'s declared range that names module nets says ``undefined name `n` ``, where a net's range over the same names says it is a net
* witness: o_param `logic n, k; localparam logic [n:k] P = 0;`: PRE d5ac3b05 ``t.sv:1:43: error[VITA-E3009] E-ELAB-UNSUPPORTED: undefined name `n` is not allowed in a constant range bound [in top]`` and the same for `k`
* witness: o_net `logic n, k; logic [n:k] v;`: ``t.sv:1:32: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `n` is not allowed in a constant range bound [in top]``; both rc 1
* note: VAL-3 F-b (W/val3/plan/PLAN.txt, cells W/val3/plan/probe/cells2); oracles not run; `range_bounds_checked` kept both texts
* status: OPEN
