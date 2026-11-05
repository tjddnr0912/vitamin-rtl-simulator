(§2 negative-signed-count · OPEN · in Ranges / bounds / selects) (a negative two-bit signed replication count reads unsigned):
* A negative signed replication count of two bits is read unsigned in a constant: `localparam logic [15:0] L = { 4'hA, {2'sb11{4'hF}}, 8'h5C };` is `ff5c` at exit 0, where iverilog 13.0 and sv2v refuse ("Concatenation repeat may not be negative (-1).") and verilator 5.052 reports "Replication value of < 0 or X/Z not legal"
* IEEE 1800-2017 §11.4.12.1
* silent
* pre-existing (row 40 plan b_negnarrow, W/row40/probe/b2)
* OPEN
