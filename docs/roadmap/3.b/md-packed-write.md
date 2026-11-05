(§3.b md-packed-write · OPEN · in Parser accept) (nested part-select writes into multi-dimensional packed arrays):
* scope: md packed nested part-select write: ascending / non-zero-LSB leaf, genvar index, silent OOB
* corpus witnesses at the nested-lvalue refusal (`elaborate/src/lvalue.rs:577`, reached when `packed_lval.rs:345–370` declines): a variable element index plus a part-select (row-9 O7, `b[i][7:0] |= …`, EL2: verilator and sv2v `b=2211 u0=16 u1=27`) and a non-zero-LSB packed element plus a part-select in a continuous assign, an output actual or a port (row-9 O8, `[1:0][15:2]`, EL2 / EH1: all three `r=0018005 w=400002380001`)
* one funnel with `nested-lval-select`
* status: OPEN (§5.2 row 29)
