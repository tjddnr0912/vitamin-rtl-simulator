(§2 🆕 AD · OPEN · in Start-order rows) (routine text folded at the caller binds the caller's same-named names):
* routine text folded or lowered at the caller's prefix binds a name in it to the caller's same-named function or constant
* §4.5.589 closed the package-routine half where PRE's fold answered (`elaborate/src/decl_scope.rs`: return, formal and local ranges, defaults, the frame reserve, the inline lane, `$bits(q::h())`, call typing and a constant call in a package routine's body bind the declaring package's)
* left here: a static package task's body-locals are not armed (`inline_task_locals.rs` `hoist_inline_task_locals`: m10_itask_par `logic [W:0] t` `v=232`, c23_task_blk, a block-local, `v=255`; iverilog, verilator, sv2v `v=8`, `v=15`)
* arm them as `inline_task` arms the formals
* a routine-local enum label in the routine's own range keeps PRE's caller binding (`decl_scope.rs` never probes a declared name: m5_enum_lbl_rt `v=232`; verilator `v=0`, the label; iverilog `v=8`, the package's; sv2v refuses; a split, not chased)
* the other halves are rows: `$unit` routines 🆕 AF, generate routines 🆕 AG, the instance-array prepass 🆕 AH, an imported routine's origin (the "Scoping" import line), the text PRE refuses (§3.b `pkg-text-open`)
* rv558_ovr's `$bits(P)` 32 (oracles 4) is the "Index sealing" override-leaf line, w05615 / w05639's `P=1000` (oracles 8) 🆕 AC's interpreter ranges
* a package in another staged compilation unit is refused (PROBE_CATALOG §4.5.589)
* ① a package function's local array dimension folds in the calling instance's scope, a legal program (row 33 r3, PLAN4 F4: the frame lowering of a package routine's local, shared code; filed here because Q-FRAMEDEFAULT, row 33's residue row in its record docs.patch, does not exist while row 33 is parked; W/row33/plan4/p4; PRE 79814020 and main's e0888973 alike): g_dim `package p; parameter int unsigned V = 5; function automatic int g(); bit a [V]; return $size(a); endfunction endpackage module t; import p::*; localparam int unsigned V = 4; initial begin #1 $display("R %0d", g()); $finish; end endmodule` `R 4`, sv2v → iverilog, verilator and iverilog `R 5`
* over enum labels, p's `MT = 4'h5` under t's `MT = 4'h6` (g_diml) `R 6`, all three `R 5`
* `p::g()` with no import under t's `V = 4` (g_dimsc) `R 4`, all three `R 5`
* no shadow (g_dimctl) `R 5` on all
* body reads are right (g_called, g_calledl, g_ctl `R 5`)
* k22b (a package function writing a module's variable, illegal) runs `R ok` on the same route
* silent-wrong, pre-existing
* 3 oracles
* status: OPEN (the task half)
