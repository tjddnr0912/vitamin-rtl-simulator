(§3.b bits-negative-msb-net · OPEN · in Constants / parameters) (`$bits` of a net with a negative msb in a localparam):
* symptom: LOUD: `$bits` of a net declared `[-3:0]` in a parameter's value is E3009 naming the net undefined
* witness: p1_negmsb_bits `logic [-3:0] v; logic [-(3):0] w; localparam int A = $bits(v); localparam int B = $bits(w);`
* witness: PRE = POST rc 1 ``t.sv:4:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `A` value is not a constant: undefined name `v` is not a constant [in top]`` and the same for `B` / `w`
* oracles: iverilog, sv2v → iverilog and verilator `A 4 4 4` (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G14 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's; the closest line is §0 11 (negative range bounds on vectors and ports)
* status: OPEN
