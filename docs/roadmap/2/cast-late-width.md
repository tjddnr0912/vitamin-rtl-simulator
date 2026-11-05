(§2 cast-late-width · OPEN · in Ranges / bounds / selects) (a cast over an operand whose width folds late takes 32 bits):
* A size or typed cast over an operand whose width folds only after elaboration takes 32 bits for it (`expr_cast.rs:387` `lower_size_cast`, `:493` the typed cast, `ir_bits_of(e).unwrap_or(32)`) and selects the low bits of a narrower value: k13 `r16 = 16'(f[0 +: gb.L])` (`f = 32'hffff_ff7f`, `L = 7`) `xxXf`, k03 `16'(f[gb.L-1:0])`, k14 `16'({(gb.L){1'b1}})`, k12 `16'(f[u.W-1:0])`, k21 `h16_t'(f[gb.L-1:0])` `xxXf`, k16 `16'(f[gb.L-1:0]) + 16'd1` `xxxx`, k01 `16'(signed'(f[gb.L-1:0]))` `xxXf`
* verilator and sv2v `007f`, `007f`, `007f`, `007f`, `007f`, `0080`, `ffff` (§4.5.601 round-3 soundness review, cells sound/r3/k)
* same root class as the row above and the WALL row's `2'(u1.mem[0] % 4)`: fix: record the width the cast assumed for §4.5.601's decision pass, or cast at the decided width
* 2 oracles
* OPEN
