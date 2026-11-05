(§5.2 Q-FORCEDYN · rank ① · source ENG-1a lens-diff r3 (K01_force_dyn_elem)) (a `force` reading a dynamic-array element never re-evaluates on its change):
* symptom: a `force` whose expression reads a dynamic-array element (`force y = d[1] * 1.0`) never re-evaluates when the element changes: vita `S t=6 y=0000… \| S t=9 y=0000…` after `d[1] = 2.0`, verilator `W t=8 y=4000…`, iverilog `sorry … evaluated once` (loud)
* status: silent wrong value, PRE = POST
* IEEE 1800 §10.6.2
* fix: refuse by name or re-seed the force from the heap change
