(§2 inline-constraint-caller-scope · OPEN · in Class fields) (a name in an inline constraint binds at the call, not in the class):
* a name in `h.randomize() with { … }` that `h`'s class record does not bind is read where the call stands, not in the class (IEEE 1800-2017 §18.7): a class `bit [7:0] P` beside a module `localparam logic [1:0][3:0] P` gives `x == P[1]` the module's flat bit (q3.rw_md_param `x=00`; `k.P[1]` = 1), a class `int W` beside a module `localparam int W = 4` gives `x == W` the module's 4 (q3.rw_const; `k.W` = 9)
* no tool oracle here (iverilog has no inline `with`, verilator no constraint solver on this host)
* hand-IEEE
* OPEN
