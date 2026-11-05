(§2 formal-shadows-parameter · OPEN · in Constant domain (i64)) (a formal shadowing a parameter is selected as the parameter):
* A formal shadowing a parameter is selected as the parameter (`const_param_select_env`), and so is a LOCAL beside a same-named constant, seeded or written: the select arm (`const_fn.rs:1124-1142`) reaches `lookup_scoped` at `const_select.rs:541` with no `envw` guard (the Ident arm has it, `const_fn.rs:942`), so vita reads the constant's word and no env read happens.
* witness: Beside `localparam logic [3:0] t = 4'hA;`, a local `logic [3:0] t; t[0]=1'b1;`: h52_local_part `return t[1:0];` `00000010`, iverilog, sv2v and verilator `000000x1`
* h52_local_idx `return t[1 +: 2];` `00000001`, all three `000000xx`
* h52_cmp `if (t[1] == 1'b1) return 8'd1; return 8'd2;` `00000001`, all three `00000010`, a wrong arm
* h52_index_shadow `u = 4'hF; u[t[1]] = 1'b0; return u;` `00001101`, iverilog and sv2v `00001110`, verilator `Internal Error: ../V3Number.cpp:1063: toUInt with 4-state 2'bx`.
* With `return t[1];` and the constant elsewhere: `$unit`, h52_unit `00000001`, all three `0000000x`
* a package import, h52_import `00000001`, all three `0000000x`
* the generate block holding the call, h52_gen `Q=00000001`, all three `Q=0000000x`
* a parameter overridden to `4'h0`, h52_param_ov `00000000`, all three `0000000x`
* a package function called beside the constant, h53_pkg_mod `00000001`, iverilog and sv2v `0000000x`, verilator no value (`Specified --top-module ... was not found in design`, the harness).
* With no seed: h52_local_written `t = 4'h5; return t[1];` `00000001`, all three `00000000`
* witness: the formal, h52_formal `input logic [3:0] b` `return b[1];`, `f(4'h5)`, `00000001`, all three `00000000`.
* A local of another name beside the same constant is E3009 (h52_local_ctl `u`, all three `0000000x`).
* evidence: PRE 719e37d2 = §4.5.633's binary on every witness, and its set is empty on all of them: the read is the constant's word, not a seeded one.
* note: A fix moves output, so §4.5.633 left it
* oracles: 3 oracles
* status: OPEN
