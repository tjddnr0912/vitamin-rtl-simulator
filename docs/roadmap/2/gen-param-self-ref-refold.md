(§2 gen-param-self-ref-refold · OPEN · in Scoping / imports / block-locals) (a generate parameter naming an outer one of its name reads its own value):
* symptom: SILENT as measured: a generate-scope parameter whose value or range names an outer parameter of its own name is re-folded each phase and reads its own earlier value, at rc 0, where iverilog and verilator refuse
* witness: p02_gen_selfref `localparam X = 1;` and in `g` `localparam X = X + 1; logic [X:0] w;` `$display("A X=%0d bw=%0d", X, $bits(w));`: PRE 66617835 `A X=4 bw=3` rc 0
* witness: p18_selfref_range `localparam logic [7:0] P = 8'd5;` and in `g` `localparam logic [P:0] P = 3;`: PRE `A 3 4` rc 0; j10_t2_window_nets2, in `g` `localparam logic [P:0] P = 8'hFF; localparam int Z = P[5:2] + 1;`: PRE `A 255 256 16 17` rc 0, oracles not run
* oracles: p02 iverilog `t.sv:4: error: Recursive parameter reference found involving X.`, verilator `%Error: t.sv:4:16: Variable's initial value is circular: 'X'` (W/bind1/ground/probe/DIGEST.txt)
* oracles: p18 iverilog ``t.sv:4: error: Circular type definition found involving `[P:'sd0]`.``, verilator `%Error: t.sv:4:28: Variable's initial value is circular: 'P'`
* note: the range fold of DECL-ORDER runs between the meta write and the value write (`bind_decl`, §5.2 BIND-1); a built mutant that moves it after the value write (M1) moves j09 to E3009
* note: BIND-1 plan F-a and RULINGS-plan R11 (W/bind1/plan/PLAN.txt, W/bind1/RULINGS-plan.txt); p02, p18 and j09 are pinned as PRE prints (`bind_pins.rs`); PRE = POST
* status: OPEN
