(§2 gen-scope-alias-width · OPEN · in Index sealing) (a generate-scope alias `localparam C = A;` binds 32 bits):
* A generate-scope alias `localparam C = A;` binds 32 bits (`param_decl_width_opt` alias arm)
* 2 oracles
* OPEN
* witness (2026-11-05, Q-DECLTYPE plan round 2): a module-scope alias `localparam A = C;` is recorded at parse time as 32 bits, signed, whatever C's shape (`hdl-parser/src/params.rs` `param_const_shape`, its (Implicit, None) arm), so a struct member sized through it differs from its plain twin
* witness: t1_alias_neg `localparam C = 4'shF;` `localparam A = C;` `typedef struct packed { logic [A + 5'd1 : 0] m; } s_t;` `s_t v;` `logic [A + 5'd1 : 0] n;` `$display("A sm=%0d n=%0d a=%0d", $bits(v.m), $bits(n), A);`: PRE `A sm=1 n=17 a=-1`, iverilog, sv2v → iverilog and verilator `A sm=17 n=17 a=-1`
* witness: t3_alias_wrap `localparam C = 4'hF;` `localparam A = C;` `typedef struct packed { logic [A + 4'd1 : 0] m; logic [C + 4'd1 : 0] k; } s_t;` `s_t v;` `logic [A + 4'd1 : 0] n;` `logic [C + 4'd1 : 0] nk;` `$display("A sm=%0d n=%0d sk=%0d nk=%0d", $bits(v.m), $bits(n), $bits(v.k), $bits(nk));`: PRE `A sm=17 n=1 sk=1 nk=1`, iverilog and verilator `A sm=1 n=1 sk=1 nk=1`, sv2v → iverilog `A sm=17 n=17 sk=17 nk=17`
* note: a prerequisite of Q-TABLEKW (filed at S1v's landing)
* source: W/qdecltype/plan2/cells/t1_alias_neg, t3_alias_wrap, W/qdecltype/plan2/PLAN-r2.txt R7 F10
