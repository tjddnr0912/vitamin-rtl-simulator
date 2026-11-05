(§3.b mdrv-ca-call · OPEN · in Constants / parameters) (task-body, hierarchical and system-task writes escape the multidriver rule):
* Rule D reads lvalues only: a task-body write (`task wt; y = 0; endtask` + `initial wt();` beside `assign y = a;`, `y=1 y=0`) and a hierarchical write (`u.y = 0;` onto a child's `assign y = a;`) run
* oracles: iverilog rejects both, verilator runs both
* a system task's output argument is no writer either (`void'($sscanf("5", "%h", y));` beside `assign y = 4'h3;` prints `y=5`; iverilog run-time `$sscanf argument 3 (a vpiNet) is not assignable.`)
* carry `call_body_writes_whole`, the hierarchical lvalue and the system-task destinations into `cont_assign_var_conflict`
* oracles: iverilog + IEEE §6.5
* status: OPEN
