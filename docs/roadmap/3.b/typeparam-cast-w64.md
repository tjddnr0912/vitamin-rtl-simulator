(§3.b typeparam-cast-w64 · OPEN · in Constants / parameters) (a type-parameter cast at 64 or 65 bits declines):
* symptom: LOUD: `T'(-1)` over a type parameter of 64 or more bits is E3009 ``the cast has no constant-fold arm``; at 63 bits it folds
* witness: g07_typeparam_cast64 `module sub #(parameter type T = logic [63:0]) (); localparam longint A = T'(-1);` with `sub u0();`, `sub #(.T(logic signed [63:0])) u1();`, `sub #(.T(logic signed [62:0])) u2();`
* witness: PRE = POST rc 1 ``t.sv:2:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: the cast has no constant-fold arm [in top.u0]`` and `[in top.u1]`; u2 folds; c_w65_td the same at 65 bits
* oracles: iverilog, sv2v → iverilog and verilator `A top.u0 A=-1`, `A top.u1 A=-1`, `A top.u2 A=-1` (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G7 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's
* status: OPEN
