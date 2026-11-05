(§2 auto-local-fork-delay · OPEN · in Delays / events) (a loop-body `automatic` read by a `fork` arm's delay takes the last value):
* An `automatic` loop-body local read by a `fork … join_none` arm's delay (`for (…) begin automatic int id = i; fork begin #(id+1) $display(id); end join_none end`, a common testbench idiom) gives every arm the last value (w09 `id=2 t=3` three times; verilator `id=0 t=1 | id=1 t=2 | id=2 t=3`; iverilog refuses the lifetime): one flattened net per `automatic`
* the fork-local form (w08) is E3009
* verilator
* OPEN
