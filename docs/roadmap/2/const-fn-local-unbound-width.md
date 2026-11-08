(§2 const-fn-local-unbound-width · OPEN · in Scoping / imports / block-locals) (a header default's constant function sizes a local by an unbound body localparam):
* symptom: a header parameter default that calls a body constant function whose local is sized by a body localparam folds the local's width before the localparam binds, silently: the value reads as a 1-bit one
* cause: elaborate fills `const_func_table` from the body's functions (`elaborate/src/instance.rs:537-542`) before `bind_params` (`:656`), and the constant-function evaluator sizes a local from its declared type (`const_fn.rs:1455` `bind_const_decl`)
* cause: a local whose width does not fold is taken silently as width 0, so `'1` reads 1 (W/qsymmix/premise5/REPORT.txt J4)
* note: the same evaluator reads a missing width as a value at `const_fn.rs:1484` and `:1674` (`unwrap_or((0, false))`, §5.2 Q-VALRULES note for (b)), and which reader j4r takes is not traced
* witness: j4r_pre_ctl_body_lp_after_hdr `module m #(parameter P = f()) ();` `localparam W = 9;` `function automatic integer f(); logic [W - 1 : 0] x; x = '1; return x; endfunction` `$display("A P=%0d", P);`: PRE vita-main 8c970e9b `A P=1` at exit 0
* oracles: iverilog, sv2v → iverilog and verilator `A P=511` (W/qsymmix/park2/m/j4r_pre_ctl_body_lp_after_hdr/RES.txt, W/qsymmix/premise5/ORACLES6.txt)
* note: found by Q-SYMMIX premise round 5 (W/qsymmix/RULINGS-premise5.txt CARRY), where the minted route moved PRE-right designs onto this default (j4c, j4j, §5.2 Q-SYMMIX)
* status: OPEN
