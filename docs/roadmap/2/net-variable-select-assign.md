(§2 net-variable-select-assign · OPEN · in Ranges / bounds / selects) (a continuous assign to a net through a variable select runs, sticky):
* A continuous `assign` to a NET with a variable select runs with sticky drivers: `wire [7:0] w; assign w[i +: 4] = a;` (a = 4'h5, i = 0 then 4) prints `R zzzz0101` / `R 01010101` on PRE 719e37d2 (verilator 5.052 `R 00000101` / `R 01010101`), where iverilog 13.0 and sv2v → iverilog refuse ("Indexed part select base expression must be a constant integral value in this context.") and IEEE 1800-2017 A.8.5 `net_lvalue` takes a constant select only (W/row29/park/cells/p3_netvar; row 29 plan P3, PRE = r1–r3)
* lenient accept
* status: OPEN
