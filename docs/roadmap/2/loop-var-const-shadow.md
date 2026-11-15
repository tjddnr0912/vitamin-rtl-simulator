(§2 loop-var-const-shadow · OPEN · in Scoping / imports / block-locals) (a loop variable does not shadow a parse-time constant in a range):
* symptom: a `for` or `foreach` loop variable named like a module `localparam` leaves the parser's constant table bound, so a typedef range in the loop body that names it folds with the localparam's value, where both oracles refuse a variable in a constant expression
* witness (row 68 + 23 plan pass 2, W/r68/plan2/m1/cells/b_forinit_dec): `localparam int N = 4;` and `for (int N = 2; N < 3; N++) begin typedef struct packed { logic [N-1:0] a; } s_t; s_t v; $display("A b=%0d", $bits(v)); end`: PRE 4d1d84bb rc 0 `A b=4`
  * oracles: iverilog ``t.sv:5: error: A reference to a net or variable (`N') is not allowed in a constant expression.`` ``t.sv:5: error: Dimensions must be constant.``
  * verilator ``%Error: t.sv:5:38: Expecting expression to be constant, but variable isn't const: 'N'`` ``%Error: t.sv:5:39: left side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1)``
* witness (b_foreach_dec): `foreach (arr[N])` over `logic [7:0] arr [3];` with a member `logic [N:0] a;`: PRE rc 0 `A b=5`
  * oracles: the same lines, iverilog at t.sv:6, verilator at t.sv:6:38
* note: pre-existing for every decimal table entry, and a new entry inherits it as loud → accepts-illegal (W/r68/plan2/PLAN2.txt §1 B; row 23's premise judge 2 j6_forinit_mix, j6_foreach_mix, §5.2 23)
* status: OPEN
