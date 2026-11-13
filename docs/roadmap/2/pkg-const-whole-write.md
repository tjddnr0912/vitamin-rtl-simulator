(§2 pkg-const-whole-write · OPEN · in Scoping / imports / block-locals) (a whole-name write to a package constant in its function is accepted):
* A whole-name write to a package constant in its function is accepted
* 2 oracles
* witness (BIND-2 plan F1): c18 `package p; localparam logic [3:0] C = 3; function automatic int g(input int a); C = C + a; return C; endfunction endpackage` `localparam int P = p::g(10);`: PRE `A P=13` rc 0; g7 (the same write, then `C[0] = 1'b0;`) is E3009
* c18: iverilog ```t.sv:3: error: Could not find variable ``C'' in ``p.g''```, verilator `%Error: t.sv:3:42: Storing to parameter variable 'C' in a context that is determined only at runtime`
* the plan's lane L3 (W8 seeds `env`, W12 has no declared target); pinned as PRE prints (`frame_pins.rs` c18), PRE = POST
* OPEN
