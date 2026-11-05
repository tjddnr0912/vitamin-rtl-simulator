(§3.b decl-var-net-kw · OPEN · in Constants / parameters) (`wire logic`, `var` declarations, drive strengths and `alias` are rejected):
* symptom: `wire logic y;`, `var logic [3:0] y;`, `output var logic [3:0] y`, `output wire logic [3:0] y` and a drive strength on `assign` (`assign (weak0, weak1) y = b;` on a `wire`) are E2002
* oracles: iverilog, verilator and sv2v → iverilog run each (`y=1`, `w=1`, `w=1`, `w=1`, `y=0`)
* `alias a = b;` and `assign p::v = …;` are E2002 too, run by verilator only (iverilog and sv2v syntax errors)
* the typedef spellings `wire s_t w`, `wire q::ps_t w`, `wire T w` are E2002 too (the row-18 matrix's net lane, 12 cells, all three oracles run them; no census page)
* oracles: 3 oracles
* status: OPEN
