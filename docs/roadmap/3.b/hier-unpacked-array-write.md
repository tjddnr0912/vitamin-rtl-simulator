(§3.b hier-unpacked-array-write · DEFERRED · in Nets / timing) (a hierarchical write of a whole unpacked array is refused):
* symptom: a hierarchical write of a whole unpacked array (`u.ram_core = '{default: v};`, EL2 tb `tb_top.sv:3388` `init_iccm`) is E3009 (`hierarchical write of Gen_iccm_enable.iccm_loop[0].iccm.iccm_bank.ram_core is unsupported`, and the keyed pattern)
* a cross-instance write: routing and ordering census first (ER §4.4)
* oracles: verilator `11 22 11` (iverilog and sv2v syntax errors)
* witness: row-9 O14
* status: DEFERRED (§7 row 59)
