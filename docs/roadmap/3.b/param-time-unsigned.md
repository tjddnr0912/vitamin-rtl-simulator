(§3.b param-time-unsigned · OPEN · in Parser accept) (a signing keyword after `time` on a parameter is refused):
* witness (2026-11-05, Q-DECLTYPE premise judge round 2, k8_time_sign): `parameter time unsigned T1 = 5;` `$display("A t1=%0d", T1);`: PRE `t.sv:2:18: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected identifier, found keyword 'unsigned'` `t.sv:2:18: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected ';', found keyword 'unsigned'` (exit 1), iverilog, sv2v → iverilog and verilator `A t1=5`
* note: legal: `time` is an `integer_atom_type`, which takes a signing (IEEE 1800-2017 A.2.2.1, ieee1800-2017.txt:4173-4193)
* note: PRE takes `int unsigned`, `integer unsigned` and `byte unsigned` (Q-PARAM2STATE's d41u note), so the `time` arm is the odd one
* status: OPEN
* source: W/qdecltype/premise2/REPORT.txt n3, W/qdecltype/premise2/probe/k8_time_sign, o_k8_time_sign
