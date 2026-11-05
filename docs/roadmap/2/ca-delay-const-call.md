(§2 ca-delay-const-call · BLOCKED · in Constant domain (i64)) (a continuous-assign delay over a constant call folds an unset variable as 0):
* A continuous-assign or net-declaration delay over a constant-function call that reads a never-assigned 4-state variable folds 🆕 AE's 0 (`const_eval.rs` `fold_ca_delay`): `assign #(fd(2)) w = r;` and `wire #(fd(2)) w = r;` delay 1 where iverilog and verilator delay 2 (p05b, p08b)
* `#(fx1(2))` (`xxx1`) delays 1 where iverilog and sv2v take the x delay as 0 (p10), as vita's run-time lane does (p12, `#(dv)` with `dv = 4'bxxx1`)
* that lane (`ca_delay_rt.rs`) shares 🆕 AE's `repeat` cut's enum prerequisites (d12, an `enum bit [3:0]` local: folded delay 1 = iverilog and verilator, run-time twin d12rt delay 2), and routing there must count the routed assign as delayed in `demote_runtime_delay_on_resolved_nets`, which otherwise drops its delay on a resolved net where today's folded delay is E3001 ("Delays": a resolved multi-driven `wire` drops a runtime delay)
* oracles: 2 oracles
* status: BLOCKED (the "Size cast" enum lines)
