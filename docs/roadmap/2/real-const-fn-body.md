(§2 real-const-fn-body · LOUD · in Real) (a real-returning constant function folds at no consumer):
* A real-returning constant function body is E3009 (iverilog 0.0), and the call folds at no consumer: under `function automatic real fr(); return 1.5; endfunction`, `localparam int` `fr() * 2`, `(fr() > 1) + 1` and `$bits(fr())` are E3009 "`fr(…)` has no constant-fold arm" on PRE 719e37d2, iverilog 13.0 3 / 2 / 1, sv2v → iverilog 3 / 2 / an error, verilator 5.052 3 / 2 / 64 (row 48 review r4 R4-F11 k_rfn1–k_rfn3; §5.2 Q-REALSPLIT's exit candidates)
* status: LOUD (§3)
