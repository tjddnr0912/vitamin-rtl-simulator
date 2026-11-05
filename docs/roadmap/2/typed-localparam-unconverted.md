(§2 typed-localparam-unconverted · OPEN · in Constant domain (i64)) (a typed localparam whose range does not fold at parse time stays unconverted):
* A typed localparam whose declared range does not fold at parse time is stored unconverted in the parser's constant table: `module t #(parameter int N = 3); localparam logic [N:0] X = 32; typedef struct packed { logic [X:0] m; } c_t;` gives `X=0 bits=33`, iverilog, sv2v → iverilog and verilator `bits=1` (found in row 23's review)
* the T stage (§5.2 row 68) or V (row 48) decides where it is converted
* 3 oracles
* OPEN
