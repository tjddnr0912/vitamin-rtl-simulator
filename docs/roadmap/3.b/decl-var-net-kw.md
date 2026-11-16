(§3.b decl-var-net-kw · OPEN · in Constants / parameters) (`wire logic`, `var` declarations, drive strengths and `alias` are rejected):
* symptom: `wire logic y;`, `var logic [3:0] y;`, `output var logic [3:0] y`, `output wire logic [3:0] y` and a drive strength on `assign` (`assign (weak0, weak1) y = b;` on a `wire`) are E2002
* oracles: iverilog, verilator and sv2v → iverilog run each (`y=1`, `w=1`, `w=1`, `w=1`, `y=0`)
* `alias a = b;` and `assign p::v = …;` are E2002 too, run by verilator only (iverilog and sv2v syntax errors)
* the typedef spellings `wire s_t w`, `wire q::ps_t w`, `wire T w` are E2002 too (the row-18 matrix's net lane, 12 cells, all three oracles run them; no census page)
* oracles: 3 oracles
* witness (Q-CONSTSELLSB park, W/constsellsb/plan4/m1): c3_var_module, `var logic [3:0] t;` at module scope and `t = 4'd5;`: PRE rc 1 ``t.sv:2:7: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected identifier, found keyword 'logic'`` and three more E2002 lines
  * oracles: iverilog and verilator `V t=0101`
* witness: in a function body, c3_var_zero `var logic [3:0] t;` and c3_var_shift `var logic [5:2] t;` in a constant function, c3_var_runtime and c3_var_first called at run time: PRE rc 1 ``expected '=' or '<=' after lvalue, found keyword 'logic'`` on each
  * oracles: iverilog and verilator `V P=00000101` (c3_var_zero), `V P=00000011` (c3_var_shift), `V r=00000101` (c3_var_runtime, c3_var_first)
* witness (2026-11-16, §5.2 Q-MDRVMIX's grounding C3, W/mdrv/ground/cells/m/m_p_invar_acomb_part): `module mi (input var logic [7:0] v, input logic [7:0] s, input logic clk);`: PRE rc 1 `t.sv:2:22: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected ')', found keyword 'logic'` and six more E2002 lines (errors=7); the matrix's 8 m_p_invar_* cells stop here, before any driver rule
  * oracles (the cell also writes the input with `always_comb v[3:0] = ~s[3:0];`, illegal for that reason): iverilog runs `A v=00001010` `B v=00000011`; verilator `%Error-ASSIGNIN: t.sv:3:15: Assigning to input/const variable: 'v'`; sv2v → iverilog `s2v.v:13: error: 'v['sd3:'sd0]' is not a valid l-value for a procedural assignment.`
* status: OPEN
