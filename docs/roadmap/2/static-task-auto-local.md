(§2 static-task-auto-local · OPEN · in Scoping / imports / block-locals) (a static task's `automatic` block-local is one copy for every call):
* A static task's `automatic` block-local is one copy for every call: its initializer runs once, at the first call (`inline_task_locals.rs` `first_call`; no per-entry lane for subroutine bodies): lt17_ctl `c=6 7 8`, lt2b_ctl `y=10 10 10`, lt13_ctl package task, lt15_ctl generate-block task (sv2v → iverilog and verilator `6 6 6`, `10 20 30`; iverilog refuses the lifetime), and a read before its write sees the previous call's value (lt11_ctl `c=0 5`, verilator `0 0`)
* the module lane refuses the same shapes (E3009)
* oracles: 2 oracles
* status: OPEN
