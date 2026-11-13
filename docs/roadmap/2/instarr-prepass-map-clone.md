(§2 instarr-prepass-map-clone · PERF · in Performance (open, recorded)) (the instance-array prepass clones eight whole parameter maps per array):
* symptom: the instance-array prepass (U3 SIDE8: `side_snapshot` / `side_restore`, `bound/undo.rs`) clones eight whole maps per array, so its cost grows with the arrays times the names alive
* witness: 1,500 arrays 0.64 s, 3,000 2.15 s, 12,000 not done in 30 s, with 400 names alive; one run each (W/bind1/plan/PLAN.txt F-c)
* note: BIND-1 plan F-c: PRE's cost, kept by BIND-1a, which moves the snapshot into `bound/undo.rs` unchanged; cost_bind1's y_inst_array heats 3000 SIDE8 pairs; no oracle (cost)
* status: PERF
