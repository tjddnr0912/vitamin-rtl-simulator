(§2 pkg-param-no-cause · OPEN · in Diagnostics / artifacts) (a package parameter that does not fold prints no cause or position):
* symptom: a package parameter that does not fold says only that it is not a foldable constant, with no position and no cause, where the module body names both
* witness: d_pkg_div0 `package p; localparam int D = 7 / 0; endpackage`: PRE d5ac3b05 ``error[VITA-E3009] E-ELAB-UNSUPPORTED: package parameter `D` value is not a foldable constant [in p]``
* witness: d_body_div0, the same in a module: ``t.sv:2:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `D` value is not a constant: the `/` operation divides by zero, which is `x` and not a constant [in top]``; `4'b1x00 + 1` the same pair
* note: VAL-2 F2 (W/val2/plan/PLAN.txt, cells W/val2/plan/probe/cells); `param_decl_fold` kept each site's own text
* status: OPEN
