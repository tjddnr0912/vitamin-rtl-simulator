(§2 select-write-unfolded-local · OPEN · in Constant domain (i64)) (a select write into an unfolded local reads 0 for the other bits):
* A select write into a local whose initializer did not fold reads 0 for the other bits (`const_fn.rs:1624` at 01cac4b3, `env.get(name).copied().unwrap_or(0)` over the unbound local `bind_const_decl` leaves; the statement 🆕 AE's x2l reads 0 through, here a declined 2-state initializer; §4.5.633's set marks the run): a41 `int t = int'(2.5); t[0] = 1'b0; f = t;` `P=0`, iverilog, verilator and sv2v `P=2`
* declining there makes the twins that read only written bits or overwrite every bit loud (p03 `t[0] = 1'b1; f = {31'b0, t[0]};` and a 32-bit overwrite loop: `P=1 Q=0` = all three oracles)
* close upward: fold the initializer (the `int'(…)` placement / cast residue above) or track the written bits
* 2 oracles
* OPEN
