(§2 🆕 H · OPEN · in Start-order rows) (constant reductions that stay loud: parameters, function locals, x/z bits):
* loud constant reductions: ⓑ `|P` over an ascending / lo≠0 parameter (`narrow_param_bits`)
* ⓔ after a const-function local assignment (`envw`)
* ⓕ a bitwise op over x/z (`const_wide.rs`: per-bit 4-state tables; §5.2 Q-XCORE) — as a declaration range bound PRE keeps the one-bit default (a Q-DECLSELBOUND witness): right by default `(4'bx & 4'b0)`, `(4'b0x00 >> 3)`, T/s581 Sxf Sxg Sxk Sxl Sy2 Sy3 Sy5 q4__B1 B4 r1v__sxfg
* silent-wrong `(1'bx ? 1 : 1)` (IEEE and 3 oracles n=2, PRE 1), `(4'bx & 4'b1)` (IEEE illegal)
* the 4-state fold of a whole bound in its context would also claim a divisor zero only in its region (`((4'bx & 4'b0) / (2'b00 + 2'b00))`, `/ (4'b1000 << 1)`, `/ (3'd4 + 3'd4)` in a 3-bit region), `((4'b0x00 << 1) + 2)`, `((1 ? 4'bx : 4'd2) + 1)`, `(4'sbx000 >>> 4) / 1`, `(4'bx << 4) + 2`, `(1'bx ? 4'd1 : 4'd3)` (all IEEE illegal, PRE n=1, the one-bit default; a prototype, W/row24/declsel/plan2 `4s`, measured +8 right claims, 0 false on 1314 cells)
* ⓓ held
* 2 oracles
* OPEN
