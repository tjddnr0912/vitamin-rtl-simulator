(§5.2 Q-SCALARSEL · rank ① · source Q-AESINK review r2 (diff N4), r3 differential lens D4) (a bit select of a scalar folds instead of being an error):
* a bit select of a scalar folds: sw_w1 `logic t; t[0] = 1'b1; f = t;` `00000001`, iverilog `sw_w1.sv:5: error: can not select part of scalar: t`, sv2v → iverilog `s2out.v:6: error: can not select part of scalar: t`, verilator `%Error: sw_w1.sv:5:6: Illegal bit or array select; type does not have a bit range, or bad dimension: data type is 'logic'`
* the run-time lane UNVERIFIED
* fix: refuse with the oracles
