(§2 bits-hier-placeholder · OPEN · in Ranges / bounds / selects) (`$bits` of a hierarchical name reads a 32 placeholder before patching):
* `$bits(u.X)` lowers to a `Const 32` placeholder until `resolve_deferred_hier_bits` patches it (`expr_special.rs:532`), and a reader of its VALUE before then takes 32: a `case` label (x05 `r=2`), `$bits({($bits(u.X)){1'b1}})` (x24 `r=20`) (verilator and iverilog `1`, `7`)
* index sealing also reads it as 32 (k04_bx `r = mem[$signed(f[$bits(u.X)-2:0]) + 7'sd5]` `r=000000xx`; verilator and iverilog `r=00000004`; the generate-block twin is right)
* §4.5.601 treats it as undecided only for widths and counts
* fix: a placeholder with a recorded `int` shape and no value (`Signal{POISON_NET}` + `hier_placeholder_shape`), after censusing its value readers
* 2 oracles
* OPEN
