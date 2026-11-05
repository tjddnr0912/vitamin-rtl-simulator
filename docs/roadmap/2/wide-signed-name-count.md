(§2 wide-signed-name-count · OPEN · in Constant domain (i64)) (a >64-bit signed neighbour of a name-count replication folds wrong):
* A >64-bit signed neighbour beside a name-count replication folds wrong: `localparam logic signed [64:0] X = -4;` with `(X + {N{1'b0}}) > 65'd100` (K_w65) `L=0`, iverilog, sv2v, verilator `L=1`
* the literal twins `X + 65'd0`, `X + 2'b00` are right, and 🆕 AI's i64 count arm does not reach it (§4.5.594)
* site not traced
* 3 oracles
* OPEN
