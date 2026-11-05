(§2 overcap-typedef-bits · in Oracle splits (recorded, not chased)) (`$bits` of a typedef wider than the v1 cap gives a value):
* witness: hz2_wide33td `parameter [32:0] PW = 33'h1_0000_0003;` `typedef logic [PW : 0] t_t;` `$display("A bt=%0d", $bits(t_t));`
  * PRE d59629fa `A bt=4294967300` at exit 0
  * iverilog `A bt=4`
  * sv2v → iverilog `A bt=4294967300` (sv2v spells the `$bits` value itself)
  * verilator `%Error: t.sv:3:18: Value too wide for 32-bits expected in this context 33'h100000003`
* note: a net of that width is loud (hz1n_wide33net, `declared net width 4294967296 exceeds the v1 cap (1048576)`)
* only the typedef read by `$bits` passes
* note: no Q-SYMMIX design changed it (P3 kept PRE's spelling), so it waits on Q-VALRULES fix (d)
* source: W/qsymmix/plan/cells/hz2_wide33td, W/qsymmix/plan3/filings.txt section 3
