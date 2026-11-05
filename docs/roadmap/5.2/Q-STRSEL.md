(§5.2 Q-STRSEL · rank ① · source Q-RELDELAY r5 re-plan Q6 (s04), r5 design review d3 / C1) (a string literal as a constant select bound or count is refused):
* slot: after Q-STRWIDTH and 74
* a string literal as a constant select bound, indexed width, replication count or lvalue width is E3009 (`a string literal has no integral constant value`) where IEEE 1800-2017 §5.9 reads it as an unsigned integer constant and iverilog = sv2v = verilator run it: s04 `wire [127:0] y; assign y["b":"a"] = a; assign y[97] = b;` (iverilog = sv2v `S y=zz1xzzz`)
* e01 `x[0 +: "\002"]` (`S r=00000010`), e02 `{"\002"{2'b10}}` (`S r=00001010`), e03 `x["\003":"\000"]` (`S r=1010`), e15 `x[0 +: "\001" + 1]` (`S r=00000010`), e18 `assign y[0 +: "\002"] = 2'b11;` (`S y=zzzzzz11`), e20 `r[1 +: "\002"] = 2'b11;` (`S r=00000110`)
* the width fold (`sim_ir::selfwidth::edge_bound_value`, `Leaves::Width`) refuses the leaf that the offset fold (`offset_value`) reads — the sim-ir unit test `offset_value_reads_a_string_literal_the_width_fold_does_not` pins that refusal and flips with this row
* pre-existing
* a declaration range bound `logic [("x"):0] d` declines to PRE's one-bit default (n=1, iverilog = sv2v = verilator 121; a Q-DECLSELBOUND witness)
