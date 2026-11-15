(§2 typed-localparam-unconverted · OPEN · in Constant domain (i64)) (a typed localparam whose range does not fold at parse time stays unconverted):
* A typed localparam whose declared range does not fold at parse time is stored unconverted in the parser's constant table: `module t #(parameter int N = 3); localparam logic [N:0] X = 32; typedef struct packed { logic [X:0] m; } c_t;` gives `X=0 bits=33`, iverilog, sv2v → iverilog and verilator `bits=1` (found in row 23's review)
* the T stage (§5.2 row 68) or V (row 48) decides where it is converted
* 3 oracles
* consumers (Q-CONSTSELLSB park, premise judge 2 F1): `type_param_literal_range` folds a type parameter's bounds through this table, `msb: Self::folded_bound(self.const_bound(&r.msb)?, r.span)?,` (`hdl-parser/src/pkg_type_param.rs:71`), for a non-overridable type parameter (`hdl-parser/src/type_params.rs:252`)
* consumers: `respell_pkg_expr`, the `pkg::t` typedef twin of a name the package imported, writes the table's value as a literal, `return Self::dec_lit(v as u32, span);` (`hdl-parser/src/packed_md.rs:158`)
* witness (W/constsellsb/plan4/m1/cells/c1_tp_mod): `module top #(parameter int W = 4);` `localparam logic [W-1:0] L = 16;` `localparam type T = logic [L+3:L];` `T x;` with `x = '0; x[2] = 1'b1; x[3] = 1'b1;`: PRE 4d1d84bb rc 0 `V x=0000 L=0 lo=16 hi=19`
  * oracles: iverilog and verilator `V x=1100 L=0 lo=0 hi=3`
* witness (c1_tp_width): the same with `localparam type T = logic [L+3:0];` and `x = '1;`: PRE rc 0 `V b=20 x=11111111111111111111 L=0`
  * oracles: iverilog and verilator `V b=4 x=1111 L=0`
* witness (c1_twin_mod): `package q;` `localparam int M = 19;` `localparam logic [(1 ? 3 : 2):0] L = 16;`, `package p;` `import q::*;` `typedef logic [M:L] o_t;`, `o_t x;` in the module: PRE rc 0 `V x=0000 b=4 lo=16 hi=19`
  * oracles: iverilog and verilator `V x=00000000000000001100 b=20 lo=0 hi=19`
* control (c1_tp_mod_fit): c1_tp_mod with `L = 2`, which fits: PRE `V x=0011 L=2 lo=2 hi=5`, as iverilog and verilator
* OPEN
