(§2 union-member-pattern · OPEN · in Scoping / imports / block-locals) (an untyped nested pattern for a packed-union member concatenates all members):
* an untyped nested `'{…}` for a packed-union member concatenates every member of the overlay and is right only where the target truncates the surplus away: b11gu `'{c: 4'h7, u: '{4'h5, 4'h6}}` `r=56`, verilator and sv2v → iverilog `r=76`
* n01_u `r=57` (sv2v → iverilog `57`, iverilog `67`)
* b11u `r=67`, b11f `r=57` and r8a's array initializer are right by truncation (= verilator and sv2v → iverilog)
* PRE = POST
* the typed pattern refuses it (§4.5.609)
* fix: lower the nested value against the member's own width
* OPEN
