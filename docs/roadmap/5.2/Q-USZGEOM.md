(§5.2 Q-USZGEOM · rank ① · source Q-DECLSELBOUND plan (Q4, O4)) (an unpacked dimension with a declined bound gets size and storage from two folds):
* slot: after Q-DECLSELBOUND
* symptom: an unpacked dimension whose bound the declaration route declines gets `$size` from one fold and storage from another: `localparam logic [7:0] U = 8'h0C; logic [3:0] arr [0:(U inside {4'b1?00})+2];` `arr[3] = 4'h9; arr[0] = 4'h5;` PRE `asz=4 a3=x a0=5` + E4002, sv2v = verilator `asz=4 a3=9 a0=5` (iverilog: `inside` unsupported)
* witness: W/s593 p3 / p7 `_ad` runs cells (13)
* evidence: PRE 9d60771d
* status: silent-wrong, pre-existing
