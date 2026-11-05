(§3.b cont-var-partial · OPEN · in Constants / parameters) (a continuous `assign` to part of a non-`logic` variable is refused):
* a continuous `assign` to part of a variable that is not `logic` stays E3018 (§4.5.600, `cont_var.rs` `cont_var_cover`): a part select or member (`reg [3:0] r; assign r[1:0] = a;`, iverilog `r=zz10`; `assign d.a = v;` on a 2-state struct, `d=1010`), disjoint parts covering it (`r[1:0]` + `r[3:2]`, iverilog and verilator `r=0110`), some elements of an array (`10 zz`; an `int` element `16 z`), a multi-dimensional array's elements (the flattened index's bounds guard does not fold; iverilog `0 1 1 2`), one element continuous beside another procedural (`10 55`)
* note: IEEE §6.5 allows each
* witness: the undriven part reads `z` in iverilog (even in an `int` element), `x` in vita's `logic` twin, `0` in verilator
* oracles: covering shapes 2 oracles
* status: OPEN
