(§2 typed-signed-bound-oracle-split · in Oracle splits (recorded, not chased)) (verilator and sv2v read a typed signed negative range bound as unsigned):
* witness: q33e_mod_typed_mdnet `localparam logic [3:0] U = 4'd1;` `localparam logic signed [3:0] S = -4'sd2;` `logic [U:S][1:0] n;`: PRE = iverilog `A bn=8 sz=4 l1=1 r1=-2 n1=11 nm2=00 s=-2 sn=1`
* verilator `A bn=28 sz=14 l1=1 r1=14 n1=00 nm2=00 s=-2 sn=1`
* sv2v → iverilog `A bn=28 sz=14 l1=1 r1=-2 n1=00 nm2=xx s=-2 sn=1` (W/qsymmix/plan2/cells; q33a q33d q33f the same split)
* note: each tool contradicts itself in one run (verilator: `S` is -2 as a value and 14 as a bound; sv2v: 14 elements over `[1:-2]`), so neither is an oracle on this axis (ER §7.3)
* IEEE 1800-2017 §6.9.1 (ref :4258-4259) makes each bound its own constant expression
