(§3.b E3001-overlap · DEFERRED · in Nets / timing) (a part-select driver beside a whole or same-range driver is a false E3001):
* same-range part-select drivers
* witness: corpus witness row-9 O10: a wire with a part-select continuous driver beside a whole driver is E3001 (`net … ifu_axi_bresp driven by multiple overlapping continuous assignments`; EL2 `el2_veer_wrapper` ties `ifu_axi_bresp` / `ifu_axi_bid` twice; iverilog and sv2v → iverilog `w=00 v=x1`, verilator `v=01`, 2-state)
* a net declaration assignment is such a whole driver too (Q-RELDELAY r3 lens-diff m10: `wire [5:0] y = 6'b0;` beside `assign y[3 -: 2] = d0;` or `y[3:2]`, E3001, iverilog = sv2v `S t=1 y=00xx00 \| S t=2 y=00x000 \| S t=3 y=000x00`, verilator `000000`)
* row 60 builds the per-bit driver map
* oracles: 2 oracles
* status: DEFERRED (§7 row 60)
