(§2 🆕 AI · BLOCKED · in Start-order rows) (a signed constant of unknown width sign-extends into an unsigned region):
* a signed constant in a constant region whose width the walk cannot give folds sign-extended into an unsigned region (§4.5.593): `logic signed [7:0] X = -4; int N = 2;` with `X + {N{1'b0}}` (Ev12 `L=4294967292`, `== 8'hFC` `M=0`, `> 8'd100` `K=0`, a bound `vb=4`; iverilog, sv2v, verilator `L=252 M=1 K=1 vb=5`; the literal count `{2{1'b0}}`, Ev12c, is right)
* beside an unpacked-array-parameter element `(X | A[1]) == 8'hFE`, `(C ? X : A[1]) == 8'hFC` (Eu7–Eu9 `L=0 GI=else GC=def vb=4`; sv2v and verilator `1 then item 5`)
* under `==?` E3009 / E3010 / `vb=1` / generate-case `default` (Eu1, Eu2, Eu4)
* a signed array element `A[0] ==? …` E3009 and `default` (arrE; verilator `L=1`)
* `const_fn_width.rs` `const_self_width`: `K::Replicate` sizes its count with the literal-only `const_eval_u32` while the value path folds it with `fold_count`, a whole constant-array element has no width or sign arm (`const_signed_env`, `const_expr_signed` read it unsigned), and an unknown width turns masking off in `eval_const_env_at`'s leaf arm
* holds 🆕 W (its T-typed twin Tv12 is right on PRE and goes wrong under W)
* evidence: §4.5.594 sized both (the count by `fold_count` through the scope resolver, the element by its declared width and sign) behind one region rule (one entry per evaluation; a region holding a user call, and the constant-function interpreter as a whole, kept PRE's leaf rules): WRONG→OK 295, LOUD→OK 20 over 3082 cells, corpus byte-identical
* reverted whole after two BLOCKING rounds, 🆕 AE's 0 entering a newly sized region each time (`fl` reads a never-assigned local): round 1 through a nested region root that re-decided the rule under a call-bearing root (C1 `localparam int L = fl(0) + ((X + {N{1'b0}}) == 8'hFC)` PRE `L=0` = iverilog, cut `L=1`; C2, a generate-if over the same sum, `else` → `then`; C8 E3009 → `L=2`), round 2 through a constant that stored it first, named from a call-free region (Nq_launder `localparam Q = fl(0)`, Apg `localparam logic [7:0] A = fl(0)`: PRE `GI=else` = iverilog = sv2v, cut `GI=then`), which no structural "contains a call" rule can see
* fix: re-derive that sizing (ER §3.6) once 🆕 AE lands
* before it §5.2's 4-state rule holds, a region that holds a call keeps PRE's leaf rules, and no constant carries a mark of its own (Q-AESINK's mark is a fact about the elaboration)
* the other `None` arms with a signed sibling are "Constant domain" lines (a call count, `S.len()`, a >64-bit neighbour)
* 3 oracles (Ev12), 2 (Eu7–Eu9)
* a NAME-FREE arithmetic count beside it is sized the same way (row 40 plan H5): `localparam int L = (({ {(1+1){1'b1}} } + 2'd1) == 2'd0);` `l=0`, `if (({ {(1+1){1'b1}} } + 2'd1) == 2'd0)` `gi=else`, iverilog 13.0 = sv2v = verilator 5.052 `l=1` / `gi=then` (the literal `{2'b11}` control is right), so a name-free narrowing does not close this line
* row 40's rule waits in the i64 walks for this line's re-derive
* status: BLOCKED (🆕 AE)
