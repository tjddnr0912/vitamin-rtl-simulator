(§5.2 Q-CVARPATEXT · rank — · source row 24 r2 lens-sound S2) (refusal text: a procedural assign beside a continuous one is named as a force):
* symptom: refusal text: a procedural continuous `assign` or a `deassign` of a `reg` / `integer` variable that a continuous `assign` also drives is refused with the force text: `reg [3:0] r; wire [1:0] x = 2'b01; assign r[1:0] = x; initial begin #1 assign r = 4'h0; …` (also `integer r; … assign r = 0;`, and `#1 deassign r;`) prints "error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `top.r` is driven by a continuous `assign` and also by a `force` or `release`; IEEE 1800 §10.6.2 allows that, but v1 keeps a continuous `assign` to a variable that is not `logic` only as its sole writer — declare it `logic`" (`elaborate/src/cont_var.rs:221`)
* the statement is a procedural continuous assignment (IEEE 1800-2017 §10.6.1, ieee1800-2017.txt:10689), whose mixture with a continuous assignment §6.5 makes an error (:3483–3484, :3495–3497), and a `deassign` writes nothing
* oracles: iverilog refuses (`Cannot perform procedural assignment to variable 'r' because it is also continuously assigned.`), sv2v → iverilog runs `r=0000` / `r=xx01` (and refuses the `integer` cell)
* pins: the `logic` twin is accepted (Q-MIXCAPROC)
* evidence: PRE 9d60771d, every backend
* fix: a text naming the procedural continuous assignment or the `deassign`
* status: loud → loud
* source: r3 re-plan OP2 (probe s2t reg_pca_pa, int_pca_pa, reg_pca_dea)
