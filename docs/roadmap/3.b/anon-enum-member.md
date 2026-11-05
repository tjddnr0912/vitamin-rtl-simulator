(§3.b anon-enum-member · OPEN · in Parser accept) (an anonymous `enum` as a struct member is rejected):
* symptom: an anonymous `enum` as a struct member (`enum logic [1:0] {IDLE, RUN, DONE} st;`) is E2002 `expected a net/var type in a struct/union member, found keyword 'enum'` (§4.5.604 took anonymous `struct packed` / `union packed` only)
* iverilog and sv2v → iverilog `E st=2 isrun=1 done=3 r=1010`, verilator refuses a label reference outside the struct (`Can't find definition of variable: 'RUN'`): an oracle split on label scope (IEEE 1800-2017 §6.19)
* status: OPEN
