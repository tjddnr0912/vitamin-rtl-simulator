(§2 override-recovery-reread · OPEN · in Diagnostics / artifacts) (a refused override's recovery value is read by a second diagnostic):
* symptom: an override that is not a constant leaves a recovery value that a later declaration reads, so a second line names a width the design does not have
* witness: c4_gate_ovr `module m #(parameter F1 = '1) (); logic [F1*1100000:0] v; endmodule` with `m #(.F1(n)) u();` over a net `n`: PRE d5ac3b05 the override's W3056 and E3009 lines, then `t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 4294967296 exceeds the v1 cap (1048576) [in top.u]`
* note: the default alone gives 1100001; VAL-2 F3 (W/val2/plan/PLAN.txt, W/val2/plan/probe/cells2/c4_gate_ovr)
* status: OPEN
