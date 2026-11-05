(§3.b bits-select-rest · OPEN · in Constants / parameters) (`$bits` of a select in the shapes the constant domain still declines):
* after §4.5.605 a constant-position `$bits` of a select stays E3009 for a multi-dimensional packed array (`logic [3:0][7:0] p; $bits(p[1])`: all three 8), a string index (`$bits(s[0])`: iverilog and verilator 8), an unpacked-array slice (`$bits(m[1:2])`: all three 16), a typedef'd array port, a non-ANSI port in a `localparam` or range (iverilog and sv2v 4), an interface or hierarchical select, and every use inside a nested scope — a generate block, a routine's header or body, an interface body, a package function — or over a name a nested scope declares (an enum label included), which the rule declines rather than resolve
* note: vita's runtime `$bits` answers the runtime positions
* witness: no corpus witness
* status: OPEN
