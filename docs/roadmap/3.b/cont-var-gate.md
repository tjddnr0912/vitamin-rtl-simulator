(§3.b cont-var-gate · SPLIT · in Constants / parameters) (a built-in gate driving a non-`logic` variable is refused):
* symptom: a built-in gate's output on a variable that is not `logic` stays E3018 (§4.5.600, `cont_var.rs`): IEEE §10.3.2 allows a variable one primitive output
* iverilog 13.0 and sv2v → iverilog refuse it ("Variable 'y' cannot be driven by a primitive or continuous assignment with non-default strength."), verilator runs `and g(y, a, b);` on `reg y` `y=1` / `y=0`, as vita runs the `logic` twin
* oracles: no 4-state tool
* status: SPLIT
