(§3.b const-fn-systask · BLOCKED · in Constants / parameters) (`$display` and other system tasks inside a constant function):
* symptom: `$display` / `$info` in a constant function is E3009 `f(…) has no constant-fold arm` where both oracles fold silently and print nothing (§4.5.587 g4, `localparam int P = f(2)`: iverilog and verilator `P=3`)
* `$warning`: iverilog `P=3`, verilator `%Warning-USERWARN: "in f a=2"` at build, then `P=3`
* `$error`: iverilog `P=3`, verilator `%Warning-USERERROR: "in f a=2"` and fails the build (`Expecting expression to be constant, but can't determine constant for FUNCREF 'f'`)
* IEEE 1800-2017 §13.4.3 ignores system task calls in a constant function (recalled)
* `exec_const_stmt` catch-all
* an ignore arm needs `unique-const-fn`'s positive opt-in, since at a consumer that replaces a run-time call (a `repeat` count, a delay) both oracles print the task's output at run time (g4 d_display_rt), and it widens the interpreter like `const-fn-case`, so it obeys §5.2's 4-state rule (§2 🆕 AE)
* oracles: 2 oracles (`$display`, `$info`)
* status: BLOCKED (§2 🆕 AF, AG, AH, the "Scoping" import line)
