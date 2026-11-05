(§5.2 Q-RANGEMASK · rank ① · source Q-AESINK grounding; review dr1 C4) (interpreter formals and locals with unfoldable ranges are not masked):
* an interpreter formal or body local whose declared range does not fold binds its value unmasked: `const_decl_wsign` declines a bound holding a call (its recursion guard) or an x, `envw` records width 0, "no masking": `h(input logic [gk():0] a)` over `gk() = 2`, `h(8'hFF)`: fr_known, lr_known `ff`, iverilog, sv2v, verilator `07`
* kn__FR, kn__LR `ff`, all three `1f`
* e8b `ff`, all three `0f`
* a seeded bound fr_z1, lr_z1 `ff`, iverilog, sv2v `01`, verilator refuses
* fr_z2, lr_z2 `ff`, pd_z2 `ffff`, all three refuse
* an x bound with no call fr_expr_x, lr_expr_x, e4_fr_x_sized `ff`, iverilog `01`, verilator refuses, sv2v `01` on e4 and dies dividing by zero on the other two
* fix: the bound folds under the depth cap in place of the guard AND the run declines where the width stays unknown: folding alone leaves fr_expr_x `ff`, declining alone turns fr_known (`07`, all three) into a refusal
* controls q1–q6 (packed, typedef, struct, parameter and `$clog2` bounds) are right and q7's run-time path answers `07`: the defect is interpreter-only
* the package-text half stays §3.b `pkg-text-open`
* static real exposure 0 in RTL (Q-AESINK G5)
