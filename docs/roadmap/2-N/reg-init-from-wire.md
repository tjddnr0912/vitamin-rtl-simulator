(§2-N reg-init-from-wire · SPLIT · in t0-event residues (held on purpose)) (a reg initialised at declaration from a constant wire):
* witness: `wire w = 1'b1; reg r = w;`: iverilog `z`, verilator and vita `1` (§6.8)
* status: SPLIT
