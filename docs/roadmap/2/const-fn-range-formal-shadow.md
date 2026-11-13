(§2 const-fn-range-formal-shadow · OPEN · in Scoping / imports / block-locals) (a local's range naming a formal or local reads the module parameter):
* symptom: SILENT as measured: a constant-function local's range that names a formal (c20b) or another local (c38) reads the module parameter of that name, at rc 0, where the oracles refuse
* witness: c20b `localparam int n = 9; function automatic int f(input int n); logic [n-1:0] t; t = '1; return t; endfunction localparam int P = f(4);`: PRE `A P=511` rc 0; without the module `n` PRE is E3009 (c20a)
* oracles: c20b iverilog ``t.sv:3: error: A reference to a net or variable (`n') is not allowed in a constant expression.`` / `Dimensions must be constant.`, verilator `%Error: t.sv:3:49: Expecting expression to be constant, but variable isn't const: 'n'`
* witness: c38_local_shadow_param_in_range `localparam int N = 5;` and in `f` `int N; logic [N-1:0] t; N = 3; t = '1; return t;`, `localparam int P = f(1);`: PRE = POST `A P=31` rc 0 (main's re-run on the frozen binaries)
* oracles: c38 iverilog and sv2v → iverilog ``A reference to a net or variable (`N') is not allowed in a constant expression.`` / `Dimensions must be constant.`, verilator `%Error: t.sv:3:56: Expecting expression to be constant, but variable isn't const: 'N'` (W/batch4/lens-diff/oracle/c38_local_shadow_param_in_range)
* cause: the plan's lane L8: a local's range folds at module scope, never in the frame (W/bind2/plan/PLAN.txt); near §2 const-domain-shadow
* note: BIND-2 plan F2, amended by batch 4 lens-diff S2 with the function-local variant (W/batch4/RULINGS-lens-diff-r1.txt); c20b is pinned as PRE prints (`frame_pins.rs`); PRE = POST
* status: OPEN
