(§3.b cont-var-force · OPEN · in Constants / parameters) (`force` / `release` over a continuous `assign` to a non-`logic` variable):
* symptom: a `force` / `release` over a continuous `assign` to a variable that is not `logic` stays E3018: `other_writers` (`cont_array.rs`) counts it as a writer
* IEEE §10.6.2 allows it
* witness: iverilog and verilator run `assign y = a;` + `force y = 0; … release y;` `y=1 y=0 y=1 y=0`, as vita runs the `logic` twin (mdrv-ca-force)
* a `bit` variable the same (Q-RELDELAY r2 lens-diff D07, a delayed concatenation `{bv, z}` with a force on `bv`, and D7 D07d, a whole `assign bv = a;` with a force: E3018, iverilog = verilator `bv=10`; the `logic` twin D07c runs)
* oracles: 2 oracles
* status: OPEN
