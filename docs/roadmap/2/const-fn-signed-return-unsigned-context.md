(§2 const-fn-signed-return-unsigned-context · OPEN · in Constant domain (i64)) (a signed constant-function return in an unsigned context keeps its sign):
* symptom: SILENT: a `logic signed [7:0]` constant-function return added to an unsigned `8'd0` in a parameter's value is summed signed, at rc 0
* witness: f49_signed_return_ctx `function automatic logic signed [7:0] f(input int a); logic signed [7:0] t; t = -a; return t; endfunction` `localparam int P = f(3); localparam int Q = f(3) + 8'd0;`: PRE = POST `A P=-3 Q=-3` rc 0 (main's re-run on the frozen binaries)
* oracles: iverilog, sv2v → iverilog and verilator `A P=-3 Q=253` (W/batch4/lens-diff/oracle/f49_signed_return_ctx/RES.txt)
* note: the same sum over a parameter, or at run time, is 253 in vita (probe q1 `A Q=253 r=253 d=253`); the nearest line, §2 unsigned-return-read-signed, is the reverse direction
* note: batch 4 lens-diff S6 (W/batch4/RULINGS-lens-diff-r1.txt), PRE = POST, not batch 4's
* status: OPEN
