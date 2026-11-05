(§2 vl-import-order · in Oracle splits (recorded, not chased)) (import precedence):
* import precedence — verilator binds the first import of a name, explicit or wildcard (D21 `import pb::P; import pa::*;` the explicit `P`; D22 and X1 the wildcard's; Iab_sel, D40 the first of two wildcards)
* iverilog, sv2v and §26.3 let an explicit import win wherever it is written and refuse two wildcards: verilator is no oracle there
