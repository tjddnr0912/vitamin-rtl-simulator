(§3.b escaped-name-dot · OPEN · in Subroutine / frame) (an escaped name holding `.` is read as a hierarchical path (E3010)):
* symptom: an escaped identifier that holds a `.` is split there as if it were a hierarchical name, so a legal design is refused with E3010: a routine `\f.x ` (its frame scope `top.$func$\f.x` then misses its own variable) and a variable `\g.h[0] ` beside a generate block `\g.h`; PRE == POST (Q-MDRVMIX's fix, premise judge 2, D7 side findings q06b and q11b, W/wrext/fix1/premise2/REPORT.txt; filed 2026-11-17 at Q-MDRVMIX's park, not caused by batch 5)
* witness: d7_q06b_esc_routine_clean (W/wrext/fix1/premise2/cells/d7_q06b_esc_routine_clean/t.sv), `function automatic void \f.x (input logic d); v[1] = d; endfunction` called as `\f.x (1'b0);`: PRE rc 1 `t.sv:3:49: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.$func$\f.x.v` [in top.$func$\f.x]`
  * oracles: iverilog 13.0 `v=xx0x`; sv2v → iverilog `v=xx0x`; verilator 5.054 `v=0000`
* witness: d7_q11b_esc_net_vs_gen_clean, `logic [3:0] \g.h[0] ;` beside `if (1) begin : \g.h logic q; end`, written `\g.h[0] [1] = 1'b0;`: PRE rc 1 E3010 `undeclared net/variable `top.\g.h[0]`` at t.sv:6:17 and t.sv:6:40
  * oracles: iverilog 13.0 `v=xx0x`; sv2v → iverilog `v=xx0x`; verilator 5.054 `v=0000`
* note: §5.2 Q-ESCSCOPE holds the backslash an escaped identifier keeps (`%m`, the resolvers reading `\x ` and `x` apart); §5.2 Q-ESCTDKEEP an escaped type name holding `::`; this entry is the `.` inside the name, and which producer splits it is not located (UNMEASURED)
* oracles: 2 oracles
* status: OPEN
