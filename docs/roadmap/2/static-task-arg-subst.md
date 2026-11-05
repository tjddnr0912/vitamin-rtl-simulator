(§2 static-task-arg-subst · OPEN · in Scoping / imports / block-locals) (a static task substitutes an input over its whole body, locals included):
* A static task binds an input argument by substitution over its whole body, so a block-local of the argument's name reads and writes the argument (pk10_ctl `a=7`, pk11_ctl `g=257`, pk14_ctl package task, pk17_ctl generate-block task; iverilog, sv2v → iverilog and verilator `a=3`, `g=16`; `inline_task.rs` `subst`)
* an `automatic` task's block-local named like an input argument shares its slot (pk12_ctl `g=16`, oracles `g=0`; pk8_ctl `in=7`, oracles `0`)
* 3 oracles
* OPEN
