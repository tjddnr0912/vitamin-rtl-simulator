(§5.2 ENG-4 · rank ① F · source engine plan for row 73 (P4)) (an `iff` guard is evaluated at resume, not at the store that makes the event):
* slot: owner ruling, D7
* symptom: an `iff` guard (`@(e iff g)`, `always @(e iff g)`, `always_ff @(posedge c iff g)`) is evaluated when vita resumes the waiter, not at the store that makes the event of `e` (F01 `W t=5`, hand `W t=8`; F02, F03 `q=2` for `q=1`, F05, F06)
* IEEE 1800-2017 §9.4.2.3
* oracles: single source: verilator = vita, iverilog and sv2v reject `iff` in an event control
* status: held for an owner ruling (D7)
* note: row 73 must refuse an `iff` wait beside assertion control until then
