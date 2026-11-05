(§2 pkg-body-const-unhooked · BLOCKED · in Scoping / imports / block-locals) (the constant domain in a package body is unhooked):
* The constant domain in a package body is unhooked (`lookup_scoped` + nine `params` twins; rv561_pkc `P=0000eeee`, iverilog, verilator, sv2v `P=01230123`)
* §4.5.589's prototype measured a strict body lookup: it loses the caller's `$unit` copy (ib1b `P=a5` → E3009) and opens 🆕 AE's 0 (ae8 E3009 → `P=00000000`, oracles `P=xxxxxxxx`)
* a name count in package function text waits on it too (Pkf_cntE `pf = ((PC ? PX : {PN{1'b1}}) == 8'hFC);` `L=0`, iverilog, sv2v, verilator `L=1`): §4.5.594's count arm resolved `PN` through the calling module's scopes (Pkf_cnt, beside a module `PN = 16`: `L=1` → `L=0`)
* status: BLOCKED (🆕 AF, 🆕 AE)
