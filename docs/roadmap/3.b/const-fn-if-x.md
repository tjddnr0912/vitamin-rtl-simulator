(§3.b const-fn-if-x · OPEN · in Constants / parameters) (a constant function's `if` on an x condition declines):
* symptom: LOUD: a constant function whose `if` condition holds an x bit is E3009, where the same function written with `?:` folds
* witness: g02_fn_if_x `function automatic int f(input int k); if (4'b100x) return k; else return -k; endfunction`, its twin `g` returning `4'b100x ? k : -k`, `localparam int Q = g(3); localparam int P = f(3);`
* witness: PRE = POST rc 1 ``t.sv:9:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]``; a16_fn_if_x (`f` alone) the same
* oracles: iverilog, sv2v → iverilog and verilator `A 3 3` (g02_fn_if_x), `A 3` (a16_fn_if_x) (W/batch3/lens-diff/D3_TABLE.txt)
* note: batch 3 lens-diff G2 (W/batch3/RULINGS-lens-diff-r1.txt R2), PRE = POST, not batch 3's
* witness (batch 4 lens-diff f27, lens-sound N-S4b k3, k8): an x in a function local declines too: f27_x_literal_cmp `logic [3:0] x; x = 4'b10x1; return (x == 4'b1001);`; k3 `logic [3:0] t = 4'bx; return t + k;` and k8 `return (|t) + k;`, each beside a module `t`: PRE = POST rc 1 E3009
* oracles: f27 iverilog `A P=0`, sv2v → iverilog and verilator `A P=X` (split); k3, k8 iverilog `A P=0`, verilator lint only, its value not run: one value oracle each, no direction written (W/batch4/lens-diff/oracle/f27_x_literal_cmp)
* note: an x plane on a local's value is §5.2 76 (§2 🆕 AE); k3, k8 are BIND-2's named killers of M9, M8, pinned as PRE prints (`frame_pins.rs`)
* status: OPEN
