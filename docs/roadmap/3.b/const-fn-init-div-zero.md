(§3.b const-fn-init-div-zero · OPEN · in Constants / parameters) (a loop local initialised by a division by zero declines):
* symptom: LOUD: a loop-body declaration re-run per iteration whose initializer divides by zero on the second pass (`10 / (1 - i)`) is E3009
* witness: k2_redecl_init_unfolds `for (int i = 0; i < n; i++) begin int t = 10 / (1 - i); s = s + t; end`, `localparam int P = f(2);`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog `A P=10`; verilator 5.054 `--lint-only` accepts, its value not run: one value oracle, no direction written
* note: batch 4 lens-sound N-S4b (W/batch4/RULINGS-lens-sound-r1.txt); k2 is BIND-2a's named killer of M3, pinned as PRE prints (`frame_pins.rs`); PRE = POST, not batch 4's
* status: OPEN
