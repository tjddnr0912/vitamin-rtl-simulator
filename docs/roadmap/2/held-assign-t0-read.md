(§2 held-assign-t0-read · in Oracle splits (recorded, not chased)) (a time-0 read of a held continuous assign's output before release):
* a t0 read of a held continuous assign's output, or of an assign below it, before the release (an `initial`'s first slice): vita `z` on a net, `x` on a variable (hand-IEEE §6.8)
* iverilog `z`
* verilator the value from the declared defaults when it runs the assign first (a6, a7 `i0 y=01`) or 2-state `0` when it runs the `initial` first (a2 `i0 y=00`) — split
* both tools order a continuous assign and an `initial` both ways in other designs (T2a, a2, T3), and an assign with only constant arguments keeps PRE's place before the processes (a4 = verilator; iverilog after) — `t0_call_cont_assign_hold.rs` race pins
