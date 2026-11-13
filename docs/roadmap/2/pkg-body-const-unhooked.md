(§2 pkg-body-const-unhooked · BLOCKED · in Scoping / imports / block-locals) (the constant domain in a package body is unhooked):
* The constant domain in a package body is unhooked (`lookup_scoped` + nine `params` twins; rv561_pkc `P=0000eeee`, iverilog, verilator, sv2v `P=01230123`)
* §4.5.589's prototype measured a strict body lookup: it loses the caller's `$unit` copy (ib1b `P=a5` → E3009) and opens 🆕 AE's 0 (ae8 E3009 → `P=00000000`, oracles `P=xxxxxxxx`)
* a name count in package function text waits on it too (Pkf_cntE `pf = ((PC ? PX : {PN{1'b1}}) == 8'hFC);` `L=0`, iverilog, sv2v, verilator `L=1`): §4.5.594's count arm resolved `PN` through the calling module's scopes (Pkf_cnt, beside a module `PN = 16`: `L=1` → `L=0`)
* witness (BIND-2 plan F3, a candidate: whether rv561_pkc takes the same route is not checked): g3b `package p; localparam logic [3:0] C = 4'd3; function automatic int g(input int a); return {C, 4'h1} + a; endfunction endpackage` beside a module `localparam logic [7:0] C = 8'hFF;`, `P = p::g(0)`: PRE `A P=241`
* g3b: iverilog `A P=49`; verilator lint accepts, its value not run; the plan's lane L3 at `const_fn.rs:939-947`: an env-only name has no `envw` entry, so the concat operand falls to `lookup_scoped` (pinned as PRE prints, `frame_pins.rs`)
* status: BLOCKED (🆕 AF, 🆕 AE)
