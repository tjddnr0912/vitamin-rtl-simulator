(§2 low-negative-formal · OPEN · in Ranges / bounds / selects) (`$low` of a function formal with a negative lsb reads 0):
* symptom: `$low` of a function formal typed by a module typedef whose lsb is negative reads 0, after a warning that clamps the net
* witness: h06b_low_module_ctl `localparam int N = -2;` `typedef logic [N+5:N] t_t;` `function automatic int f(t_t v); return $bits(v) * 100 + $low(v) + 10; endfunction` `$display("A f=%0d", f(0));`: PRE b0 9870e7ba `A f=610` at exit 0
* PRE's warning: ``t.sv:5:20: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: negative packed-range low bound (e.g. `[3:-2]`) is not yet supported; net clamped to width 1 [in top]``
* oracles: iverilog, sv2v → iverilog and verilator `A f=608` (W/row46/plan/run/h06b_low_module_ctl/RES.txt)
* note: the negative-bound class of §0 11; found by row 46's plan (F2), outside its page forms
* status: OPEN
