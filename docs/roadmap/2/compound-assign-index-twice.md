(§2 compound-assign-index-twice · OPEN · in Ranges / bounds / selects) (a compound assignment evaluates its element index twice):
* A compound assignment evaluates its element index twice: `a[f()] += 1` on `int a [8]` calls `f` twice (q6_compound_fixed `cnt=2 v=1`, iverilog and verilator `cnt=1 v=1`; IEEE 1800-2017 §11.4.1 evaluates the left operand once), on every array kind
* 2 oracles
* OPEN
