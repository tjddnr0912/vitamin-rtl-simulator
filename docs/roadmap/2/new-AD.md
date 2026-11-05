(§2 🆕 AD · OPEN · in Start-order rows) (routine text folded at the caller binds the caller's same-named names):
* routine text folded or lowered at the caller's prefix binds a name in it to the caller's same-named function or constant
* §4.5.589 closed the package-routine half where PRE's fold answered (`elaborate/src/decl_scope.rs`: return, formal and local ranges, defaults, the frame reserve, the inline lane, `$bits(q::h())`, call typing and a constant call in a package routine's body bind the declaring package's)
* left here: a static package task's body-locals are not armed (`inline_task_locals.rs` `hoist_inline_task_locals`: m10_itask_par `logic [W:0] t` `v=232`, c23_task_blk, a block-local, `v=255`; iverilog, verilator, sv2v `v=8`, `v=15`)
* arm them as `inline_task` arms the formals
* a routine-local enum label in the routine's own range keeps PRE's caller binding (`decl_scope.rs` never probes a declared name: m5_enum_lbl_rt `v=232`; verilator `v=0`, the label; iverilog `v=8`, the package's; sv2v refuses; a split, not chased)
* the other halves are rows: `$unit` routines 🆕 AF, generate routines 🆕 AG, the instance-array prepass 🆕 AH, an imported routine's origin (the "Scoping" import line), the text PRE refuses (§3.b `pkg-text-open`)
* rv558_ovr's `$bits(P)` 32 (oracles 4) is the "Index sealing" override-leaf line, w05615 / w05639's `P=1000` (oracles 8) 🆕 AC's interpreter ranges
* a package in another staged compilation unit is refused (PROBE_CATALOG §4.5.589)
* 3 oracles
* status: OPEN (the task half)
