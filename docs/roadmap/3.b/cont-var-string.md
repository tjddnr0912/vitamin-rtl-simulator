(§3.b cont-var-string · SPLIT · in Constants / parameters) (a continuous `assign` to a `string` or class handle is refused):
* symptom: a continuous `assign` to a `string` or a class handle stays E3018 (`check_lvalue_kind`): `string v; assign v = a ? "ab" : "c";` iverilog prints one byte, 0x01 then a space (`%s` after `a = 0`, then `a = 1`), verilator `c` / `ab`
* a class handle `assign b = a;` aborts iverilog (`vvp_fun_bufz: recv_object(...) not implemented`)
* status: SPLIT
