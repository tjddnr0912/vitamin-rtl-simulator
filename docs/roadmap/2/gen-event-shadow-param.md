(§2 gen-event-shadow-param · OPEN · in Scoping / imports / block-locals) (a generate block's `event` shadowing an outer parameter runs silently):
* A generate block's `event N` shadowing an outer `localparam int N = 0` read as a constant width runs silently (t.event_blk: `case (3) P[0 +: N]:` takes `default`, `o=22`) where all three oracles refuse
* the constant fold reads the outer parameter (`lookup_scoped`)
* pre-existing (PRE = row 65)
* 3 oracles
* witness (batch 4 lens-diff S1): a generate `logic` variable shadowing an outer parameter runs with the outer value too: b01_net_shadow_repl `localparam int N = 3;` and in `g` `logic [1:0] N = 2; wire [5:0] w = {N{1'b1}};`: PRE = POST `A 000111` rc 0 (main's re-run on the frozen binaries)
* b01: iverilog and sv2v → iverilog ``A reference to a net or variable (`N') is not allowed in a constant expression.``, verilator `%Error: t.sv:5:22: Replication value isn't a constant.` (W/batch4/lens-diff/oracle/b01_net_shadow_repl)
* witness: b11_indexed_width_shadow_net (`v[0 +: W]` under `logic [2:0] W = 3'd2`, outer `W = 4`) `A 4`, probe q6 (`logic [2:0] N = 3'd5; localparam int R = N + 1; logic [R:0] x;`) `A R=5 bx=6`, and D4 cells e02, e21, each PRE = POST at rc 0
* b11, q6: iverilog ``A reference to a net or variable (`W') is not allowed in a constant expression.`` (`N` for q6), verilator `%Error: t.sv:6:40: Expecting expression to be constant, but variable isn't const: 'W'` (q6 `t.sv:5:24`, `'N'`)
* the consumers seen: a replication count, an indexed width, a range bound, a localparam's value; this line named an `event` only (W/batch4/RULINGS-lens-diff-r1.txt AMEND), PRE = POST, not batch 4's
* OPEN
