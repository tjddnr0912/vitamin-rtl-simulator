(§3.b dyn-md-elem-select · OPEN · in Strings / heap) (selects into a multi-dimensional element of a dynamic container):
* symptom: a select into a md element of a dyn array is E3009
* fix: record the handle's packed extents (`netdecl.rs`), route via `lower_packed_read`
* oracles: verilator
* status: OPEN.
* A part-select WRITE into a md element of a dynamic container is not E3009 (row 29 plan P3, PRE 719e37d2, W/row29/park/cells p3_dyn*): a queue `t_ex q[$]; q[0][1:0] = 2'b00;` and an associative array `t_ex a[int]; a[3][1:0] = 2'b00;` (`typedef logic [1:0][3:0] t_ex`) warn W4020 `unsupported dyn lvalue shape (write ignored)` and print `R a5`, verilator 5.052 `R 00`
* symptom: a dynamic array `d[1][1:0] = 2'b01;` (d[1] = 8'h5A) writes the two flat low bits, `R a5 59`, verilator `R a5 01` (silent-wrong)
* oracles: iverilog 13.0 and sv2v refuse every cell
