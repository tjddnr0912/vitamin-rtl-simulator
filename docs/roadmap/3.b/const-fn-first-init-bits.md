(§3.b const-fn-first-init-bits · OPEN · in Constants / parameters) (a zero-formal constant function's first `$bits` initializer declines):
* symptom: LOUD: in a constant function with no formal, a first local initialised by `$bits(Q)` of a module parameter runs on a frame with no width yet, and the default arm declines: E3009
* witness: k7_first_init_empty_env `localparam int Q = 5;` `function automatic int f(); int t = $bits(Q); return t; endfunction` `localparam int P = f();`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog `A P=32`; verilator 5.054 `--lint-only` accepts, its value not run: one value oracle, no direction written
* note: the plan's lane L6 (the empty frame); IEEE 1800-2017 §13.4.3 allows `$bits` in a constant function (W/batch4/lens-sound/REPORT.txt N-S4b)
* note: batch 4 lens-sound N-S4b; k7 is BIND-2a's named killer of M7, pinned as PRE prints (`frame_pins.rs`); PRE = POST, not batch 4's
* status: OPEN
