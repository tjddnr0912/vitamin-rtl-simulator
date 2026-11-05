(§2 respell-drops-const-type · OPEN · in Size cast / signedness) (a package md parameter's use site reads an imported constant as a 32-bit integer):
* symptom: `respell_pkg_expr` (`hdl-parser/src/packed_md.rs:198`, `dec_lit` at `:225` and `:228`) replaces a constant the package imported by `dec_lit(v)`, dropping the width and sign `ConstVal` holds (`hdl-parser/src/lib.rs:314-318`)
* witness: n3b_respell_wrap (p2 imports p1's `localparam logic [3:0] U = 4'd15;`, `localparam logic [U + 4'd1 : 0][1:0] P`; the module imports p2): PRE `A bp=34 sz=17 p0=10` = sv2v → iverilog
* verilator `A bp=2 sz=1 p0=10`
* iverilog `sorry: packed array parameters are not supported yet` (W/qsymmix/premise1/cells5)
* note: spelling only the declaration exactly gave `A bp=2 sz=17` (Q-SYMMIX re-plan 1, W/qsymmix/plan2/pair_f1t.txt), so the declaration and every use site move together
* fix: the respell keeps `ConstVal`'s width and sign (a sized literal)
* measure the package type-parameter caller (`hdl-parser/src/pkg_type_param.rs:113`, `:116`) with it
* oracles: verilator and hand IEEE: a parameter with a type keeps it (IEEE 1800-2017 §6.20.2), and a range bound is self-determined (§11.6.1), so U + 4'd1 is 4 bits and 15 + 1 wraps to 0: [0:0][1:0], verilator's bp=2 sz=1
* iverilog refuses a packed array parameter
* sv2v → iverilog sides with vita
* status: OPEN
