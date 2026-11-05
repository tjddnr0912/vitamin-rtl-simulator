(§2 dyn-oob-reads-x · OPEN · in Ranges / bounds / selects) (an out-of-range read of a 2-state dynamic array or queue reads x):
* An out-of-range or x index read of a dynamic array or queue of a 2-state element reads `x`, not the element type's default (q3_dynq `d=xx q=xxxxxxxx dneg=xx qneg=xxxxxxxx`, iverilog and verilator `d=00 q=00000000 dneg=00 qneg=00000000`; IEEE 1800-2017 §7.4.6, Table 7-1)
* an associative array reads the default since §4.5.616 (`netread.rs` `assoc_missing_read`)
* a 4-state element reads `x` (iverilog `dl=xx ql=xxxxxxxx`)
* 2 oracles
* OPEN
