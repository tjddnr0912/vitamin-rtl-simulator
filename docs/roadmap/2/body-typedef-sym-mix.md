(§2 body-typedef-sym-mix · OPEN · in Size cast / signedness) (a function-body symbolic struct over mixed-sign parameters is laid out wrong):
* symptom: a symbolic packed struct typedef in a function body, its members over a signed and an unsigned parameter, is laid out wrong at exit 0: Q-SYMMIX's composition on the body-typedef route (`in_body_typedef`), which emits no module item (W/qsymmix/replan/PLAN.txt F-4)
* witness: h13_body_func `module top #(parameter NS = 4'shF, parameter L3 = 4'd15);` `function automatic int f();` `typedef struct packed { logic [L3 - 5'sd14 : 0] a; logic [NS + 5'sd2 : 0] b; } s_t;` `s_t v;` `return $bits(v);` `$display("A f=%0d", f());`
  * PRE vita-main 8c970e9b `A f=20` at exit 0
  * oracles: iverilog = verilator `A f=4`, sv2v `` t.sv:3:5: Parse error: missing expected `endfunction` `` (W/qsymmix/park2/m/h13_body_func/RES.txt)
* note: every minted candidate kept PRE's bytes here (TK5: no ticket for a body typedef, h13 and n07), so it waits on Q-SYMMIX's re-entry with spelling (C) (§5.2 Q-SYMMIX)
* status: OPEN
