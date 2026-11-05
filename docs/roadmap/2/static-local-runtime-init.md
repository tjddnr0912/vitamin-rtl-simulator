(§2 static-local-runtime-init · OPEN · in Scoping / imports / block-locals) (a static task's static local from a run-time value initialises at first call):
* A static task's static block-local initialised from a run-time value (an argument, a module variable) runs the initializer at the first call, not once before time 0 (lt10_ctl `y=10 11 12`, iverilog `0 1 2`; si_mod_ctl `y=50 51`, verilator `10 11`, iverilog `0 1`)
* the frame lane's twin runs it at each call (rec_si_ctl `y=20 10 0`, iverilog `0 1 2`; the HELD line `frame_static_init_t0_safe` below)
* 2 oracles
* OPEN
