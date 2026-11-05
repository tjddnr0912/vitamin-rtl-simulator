(§3.b E3009-anchor · OPEN · in Diagnostics quality) (some elaboration errors lack file:line or name the module header):
* symptom: some E3010 / E3009 sites lack file:line (`diag::SpanResolver`), and an import arm's E3009 names the module header (§4.5.591's §26.3 conflicts: `Ewn.sv:2:8`, `Iee_gif.sv:3:8`, `D41m.sv:1:8`; iverilog the second import, `Ewn.sv:4`, `Iee_gif.sv:5`, `D41m.sv:3`)
* status: OPEN
