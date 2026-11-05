(§2 task-local-assoc-word · OPEN · in Delays / events) (an associative array declared in a static task's body is one word):
* An associative array declared in a static task's body is one word: `task t; bit [7:0] mem [int]; mem[16] = 8'h5a; mem[17] = 8'ha5; mem[3] = 8'h33;` reads `v=xx xx xx` with an E4002 warning per access (`array word index of top.$itask$t$L.mem … out of range`), and `$readmemh("p.hex", mem)` there warns W4023 `address 16 outside the load range` and reads `v=xx xx` (n2_task_local_ew, n2_task_local = lens e8_static_task_local2; verilator `v=5a a5 33`, `v=5a a5`; the same on PRE 77231d42)
* the static task's body-local takes the inline-task lane (`inline_task_locals.rs` `hoist_one_inline_local`), which has no dynamic-storage handle arm and sizes the `[int]` dimension through `array_dim_extents` as one word (`array_geom.rs`: `Dim::Dyn | Dim::Queue(_) | Dim::Assoc(_) => (0, 1)`, commented "dyn dims never reach the static-extent path")
* verilator
* OPEN
