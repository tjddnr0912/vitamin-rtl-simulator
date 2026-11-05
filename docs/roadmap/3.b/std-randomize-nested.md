(§3.b std-randomize-nested · DEFERRED · in Constrained random) (`std::randomize() with` used as a value expression):
* symptom: `std::randomize(v) with {v dist {…}}` as a value expression (`ok = std::randomize(…) with {…};`, EL2 tb `ahb_sif.sv:87`, its VERILATOR branch) is E3009 (`inline randomize() with {…} is supported as a statement or a direct assignment rhs`; `std::randomize()` is absent)
* oracles: verilator runs no solver here (`ok=0`), iverilog and sv2v refuse: hand-IEEE (§18.12)
* witness: row-9 O15
* status: DEFERRED (§7 row 58)
