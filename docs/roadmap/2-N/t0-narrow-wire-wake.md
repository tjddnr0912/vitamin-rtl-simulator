(§2-N t0-narrow-wire-wake · HELD · in t0-event residues (held on purpose)) (a narrowing wire initialiser wakes an event control at time 0):
* symptom: `wire [3:0] w = r8` of `reg [7:0] r8 = 8'h12` wakes `@(w)` at t0 (iverilog not)
* status: HELD
