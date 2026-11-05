(§2 inlined-body-free-name · OPEN · in Scoping / imports / block-locals) (a routine body lowered in the calling block binds a same-named local):
* A static task's (or `function void`'s), a package routine's (static or automatic, task or function) and a `let`'s body is lowered inside the calling block, so a free name it shares with a block-local of that block binds the local: b7_plain `a=9`, b7d_plain (`#(z)`) `a=2`, b7w_plain (the task writes `z`) `b=7 mz=5`, b7n_plain, b7g (`always @(posedge clk) begin int z;`), b6_plain (an imported package variable `pv=5 a=9`), r8_plain / r8a_plain / r8f_plain (a package task, automatic task or function reading a name its package imports, `a=9`), l2_plain / l1b_plain (a `let`, `a=9`)
* oracles: iverilog, sv2v → iverilog and verilator `a=5`, `a=5`, `b=9 mz=7`, `a=5`, verilator alone `a=5` for `let`.
* Allow-listed as non-capturing for their own body (rounds 5–6, every free-name kind): a module `automatic` task, a module non-void function, a module static task reached only through an `automatic` task, a hierarchical call into a child or an interface.
* Under a timing control the local is E3009 since §4.5.611 wherever its block reaches — by name mention, closed through every routine including the allow-listed ones — a callable that is not allow-listed (module static task, package routine, `let`, class, sequence, property, covergroup) whose definition names it
* the message gives the call path (P1 of round 5's review: b7, b7d, b7w, b7f, b17, b19, b22; F1 / F2 of round 6's: r8, r8a, r8f, r8c, r8x, r8d, r8w, l1, l1b, l2, l3).
* Fail-closed, so names that happened to read right are refused too: K_enum, K_genvar, b8, b8e, b9, K_range_only, K_hier_first_seg
* a package routine's own package variable (L9, A12, b16)
* a `let` reached through an allow-listed lane (A6, A7, A8, l4)
* the round-7 differential review's package and `let` idioms (NOTE 2): a package routine's formal or own local named like the local (p01, p02, p02c, p03), the package's own variable (p01c), a `let` formal (p04) — PRE loud on each except p01c, p01u and p07b (PRE wrong)
* ruled (main): p07a, right before the row only by accident (twin p07b `modi=4`, oracles `modi=2`).
* The path the message prints is a chain of names, each mentioned in the text before it, not a proven call (p05, p06c, p08e).
* Over-refused as well, and refused on PRE too: a local named like a name the static task declares itself (diff lens round 6 NOTE 1: i02, i02a, i02n, i02t, i02u, i02v, i02y, i11, i11t, c6, c6t, c6u, c6v, c6w, c6x) — §5.2 row 72b.
* B stage (§5.2 row 72b)
* 3 oracles (verilator for `let`)
* status: OPEN
