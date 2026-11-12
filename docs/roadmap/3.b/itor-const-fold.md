(§3.b itor-const-fold · OPEN · in Constants / parameters) (`$itor` in a constant expression has no fold):
* symptom: LOUD: `$itor` in a parameter's value is E3009 at any operand width
* witness: g11_itor `localparam real T = $itor(5);` `localparam real U = $itor(70'd5);` `localparam real V = $itor(64'd5 >> 1);`
* witness: PRE = POST rc 1 ``t.sv:2:23: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `T` value is not a constant: `$itor(…)` has no constant-fold arm [in top]`` and the same for `U` and `V`
* oracles: iverilog, sv2v → iverilog and verilator `A 5.000000 5.000000 2.000000` (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G11 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; only a comment in `hdl-ast/src/sysfn.rs` names it; §0 ⑥ is `$itor` at run time
* status: OPEN
