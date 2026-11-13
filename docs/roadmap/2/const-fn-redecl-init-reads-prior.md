(§2 const-fn-redecl-init-reads-prior · OPEN · in Constant domain (i64)) (a constant-function local's initializer that names the local itself):
* symptom: a block declaration's initializer that names its own local reads the last loop iteration's value (g5, rc 0), and on a first pass, or beside a module parameter of that name, is E3009 (c29, k1: LOUD)
* witness: g5 `for (int i = 0; i < n; i++) begin int t = t + 1; if (i == 0) t = 10; s = s + t; end`, `f(3)`: PRE `A P=33` rc 0 (W/bind2/plan/PLAN.txt F4)
* oracles: g5 iverilog `A P=12`; verilator lint accepts, its value not run: one value oracle, so no severity word
* witness: c29_init_reads_self_redecl `for (int i = 0; i < k; i++) begin int t = t + 5; acc = acc + t; end`, `f(3)`, and k1_shape_before_init `localparam int t = 7;` beside `int t = t + 1; return t;` in `f()`: PRE = POST rc 1 ``parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``
* oracles: iverilog `A P=15` (c29), `A P=1` (k1); verilator 5.054 `--lint-only` accepts both, its value not run: one value oracle each, no direction written (W/batch4/RULINGS-lens-sound-r1.txt N-S4b)
* cause: the plan's lanes L2 (a declared, unbound local's read is loud), L5 and L9 (W/bind2/plan/PLAN.txt)
* note: BIND-2 plan F4 and batch 4 lens-sound N-S4b (c29, k1 pinned as PRE prints in `frame_pins.rs`; k1 is BIND-2a's named killer of M2); PRE = POST, not batch 4's
* status: OPEN
