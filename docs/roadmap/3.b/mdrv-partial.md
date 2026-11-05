(§3.b mdrv-partial · RECORD · in Constants / parameters) (the multidriver check ignores partial writes):
* the multidriver check ignores partial writes (`stmt_writes_whole_ident`)
* Rule D too: `assign y = {a, a}; always @* y[0] = b;` runs `y=11` (iverilog "Cannot perform procedural assignment to bit select", verilator MULTIDRIVEN), two `assign y[0]` on an unpacked element run `y0=5` (iverilog multiple drivers, verilator MULTIDRIVEN)
* xcelium unmeasured
* status: RECORD
