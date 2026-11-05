(§2 nested-select-of-bit · OPEN · in Ranges / bounds / selects) (a nested part-select read of a bit-select is accepted):
* A nested part-select READ of a bit-select is accepted: `logic [1:0][15:2] b; … b[1][9][0:0]` (rd_b1p_c) reads `R 0` on PRE 719e37d2, where iverilog 13.0 ("the number of indices (3) is greater than the number of dimensions (2)"), sv2v → iverilog and verilator 5.052 ("Illegal bit or array select; type does not have a bit range") refuse the select (a part-select of a scalar)
* row 29 plan, PRE = r1–r3
* lenient accept
* status: OPEN
