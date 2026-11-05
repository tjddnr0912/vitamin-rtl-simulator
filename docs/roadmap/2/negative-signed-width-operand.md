(§2 negative-signed-width-operand · in Oracle splits (recorded, not chased)) (a negative signed width operand):
* a negative SIGNED width operand (`f[0 +: -4'sd2]`) — iverilog, sv2v and verilator read its 4-bit pattern as 14 (`0def`)
* vita refuses it as negative (E3009, §4.5.601 RC1), as it refuses `f[0 +: -1]` (iverilog and sv2v read one bit, verilator refuses)
* loud, recorded
