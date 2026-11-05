(§5.2 Q-MEMBSYSFNBOUND · rank ② · source row 24 r2 lens-diff N4 (d/bound_sysfn), r2 k/high_pc, r3 cells t/high_bound) (`$high`, `$size` and the other array queries of a struct member are refused):
* symptom: an array query function of a packed-struct member is E3009 `unsupported system function in expression` (`elaborate/src/expr_main.rs:1369`): `typedef struct packed { logic [15:8] a; logic [3:0] b; } h_t; h_t h; … $display("%0d", $high(h.a));`, also `$low`, `$left`, `$right`, `$size` and `$increment` of `h.a`, where IEEE 1800-2017 §20.7 (ieee1800-2017.txt:27464–27475) and iverilog 13.0 give `15`, `8`, `15`, `8`, `8`, `1` (sv2v → iverilog `11 4 11 4 8 1`: the member's position in the flattened struct)
* witness: as a select base (`h.a[$high(h.a) -: 4]`, IEEE `1011` = vita's literal `h.a[15 -: 4]`; sv2v `0101`, iverilog asserts `elab_expr.cc:2372`) and in a constant (`localparam int H = $high(h.a);` E3009 "parameter `H` value is not a constant: undefined name `h` is not a constant", iverilog `15`)
* pins: `$bits(h.a)` runs (`8`), the plain twin `$high(a)` on `logic [15:8] a` runs (`15`), as does `$high(h)` (`11`)
* evidence: PRE 9d60771d, every backend
* note: the bound spelling `h.a[$high(h.a):$high(h.a)-3]` is row 24's E2002 (iverilog `A hb y=1010`, sv2v `0101`)
* symptom: a select as the argument takes the same arm (Q-SIZESEL, Q-PKRDLSB plan R-7; W/qpkrdlsb/park/nine d_size_dec, size_ctl; PRE 719e37d2): `logic [3:0][5:2] q [4]; int i = 1; $size(q[i + 1][1][4:3])` E3009 `unsupported system function in expression`, sv2v = verilator `R 0 2`, iverilog exits 139
* witness: `$size(q[1][1])` and `$size(v[4:3])` (`logic [7:0] v`) the same, verilator `R 0 4 2`, iverilog exits 139
* status: loud
