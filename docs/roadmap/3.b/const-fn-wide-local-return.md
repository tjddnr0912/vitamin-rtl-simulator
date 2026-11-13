(§3.b const-fn-wide-local-return · OPEN · in Constants / parameters) (a constant function's local or return over 64 bits declines):
* symptom: LOUD: a constant function computing in a local or a return wider than 64 bits (128, 100) is E3009
* witness: f23_wide_ret_shift `function automatic logic [127:0] f(input int a); logic [127:0] r; r = 1; r = r << (a * 8); return r - 1; endfunction`, `localparam logic [127:0] P = f(13);`: PRE = POST rc 1 E3009 (`f(…)` has no constant-fold arm)
* witness: d17_ctx_wide_fn_in_range_and_init, a 100-bit `w(90)` and `logic [$bits(w(1)) - 1:0] z;`: PRE = POST rc 1, E3009 for `P` and ``a `$bits` that is not a constant here is not allowed in a constant range bound``
* oracles: iverilog, sv2v → iverilog and verilator `A P=000000ffffffffffffffffffffffffff` (f23), `A 0040000000000000000000000 100` (d17) (W/batch4/lens-diff/oracle/)
* note: near §3.b const-fn-wide-local-truth (a wide local as a `?:` condition); batch 4 lens-diff (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
