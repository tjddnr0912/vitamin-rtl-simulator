(§2 pctm-unit-routine · OPEN · in Scoping / imports / block-locals) (`%m` in a `$unit` or package routine prints the calling instance):
* symptom: `%m` inside a function declared in `$unit` or in a package prints the instance that called it plus the routine's name, where IEEE 1800-2017 §21.2.1.6 prints "the hierarchical name of the design element, subroutine, named block, or labeled statement that invokes the system task", the routine's own; PRE == POST (Q-MDRVMIX's fix, premise judge 2, B-3 and D7, W/wrext/fix1/premise2/REPORT.txt; filed 2026-11-17 at Q-MDRVMIX's park, not caused by batch 5)
* witness: m1_unit_fn_pctm (W/wrext/fix1/premise2/cells/m1_unit_fn_pctm/t.sv), `function automatic void f(); $display("m=%m"); endfunction` in `$unit`, called from `top` and from `top.u`: vita `m=top.f` / `m=top.u.f`
  * oracles: iverilog 13.0 `m=$unit::f` (twice); verilator 5.054 `m=$unit.f` (twice)
* witness: m2_pkg_fn_pctm, the same function in `package p`, imported by `top` and its child: vita `m=top.f` / `m=top.u.f`
  * oracles: iverilog `p::f`; verilator `p.f`
* witness: m3_unit_fn_child_only, a `$unit` function called only from `top.c1`: vita `m=top.c1.f`
* note: the two oracles differ in the separator (`::` against `.`) and agree on the scope, the routine's package or `$unit`; §5.2 Q-PCTM is `%m` after a suspension in an automatic task, another cause; §5.2 Q-MDRVWHO needs the declaring-scope fact this lacks (absent for a `$unit` or package routine)
* oracles: 2 oracles
* status: OPEN
