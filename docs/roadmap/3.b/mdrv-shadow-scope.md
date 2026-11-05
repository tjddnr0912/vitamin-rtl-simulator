(§3.b mdrv-shadow-scope · OPEN · in Constants / parameters) (a same-named block local hides a whole process from the multidriver check):
* the multidriver shadow guard skips a whole PROCESS when any block in it declares a local of the variable's name (`declares_local_named`, Rules A, B and D), so the process's real writes outside that block go uncounted: `assign y = 4'h1;` + `initial begin y = 4'h2; begin : inner logic [3:0] y; y = 4'h3; end … end` prints `y=2`, a `fork` whose sibling branch declares `y` prints `y=7`, and `always @* begin y = b; assert (…) else begin logic [7:0] y; y = '0; end end` prints `y=11` (sv2v → iverilog and verilator `y=22`)
* oracles: iverilog "Cannot perform procedural assignment to variable 'y' because it is also continuously assigned." on all three, verilator CONTASSINIT / MULTIDRIVEN
* scope the guard to the declaring block (writes outside it count)
* oracles: 2 oracles
* status: OPEN
