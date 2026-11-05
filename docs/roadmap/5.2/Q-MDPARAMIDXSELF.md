(§5.2 Q-MDPARAMIDXSELF · rank ① · source Q-MEMBSRC plan probe kelem/vw, re-plan O3; Q-DECLSELBOUND r2 lens-sound F3) (an md packed parameter element select reads a 32-bit-widened index):
* slot: in row 70
* a packed multi-dimensional PARAMETER element select reads the 32-bit-widened index: `localparam logic [15:0][3:0] VW = 64'hFEDC_BA98_7654_3210; localparam bit [2:0] C = 3'd7;` `VW[C + 3'd1]` = 8, `VW[C + 3'd2]` = 9 (constant and run-time read) where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12351–12353) gives 0 and 1 and vita's variable twin `rw[C + 3'd1]` reads 0
* the 2-element `VM[C + 3'd1]` on `[1:0][3:0]` is E3009 (IEEE `0110`)
* iverilog `sorry: packed array parameters are not supported yet.`
* sv2v → iverilog 8 / 9 (its variable twin 8: disqualified)
* after Q-MEMBSRC the member md element `P.m[C + 3'd1]` folds `0110` (IEEE; iverilog's variable twin `mm[C + 3'd1]` `0110`) while `VM[…]` stays E3009 until this row (the member leads, 7 cells, ruled 2026-10-27)
* PRE 9d60771d
* silent-wrong
* as a declaration range bound (`logic [VZ[C + 3'd1]:0] d`, IEEE n=1 = PRE) the element declines and PRE keeps the one-bit default
* the widened element is what a bound claim would read: `localparam logic [15:0][3:0] VW = 64'hFEDC_BA90_7654_3212;` `logic [(4'd1 / VW[C + 3'd1]):0] d;` IEEE 1 / 2 = 0, n=1 = verilator = PRE (the default), sv2v → iverilog x, where vita reads element 8 (`0`), a zero divisor (Q-DECLSELBOUND r2 lens m01; m03 m04 m08 o04 o05)
