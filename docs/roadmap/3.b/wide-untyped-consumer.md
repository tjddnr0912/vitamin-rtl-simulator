(§3.b wide-untyped-consumer · OPEN · in Constants / parameters) (an untyped parameter copied from a parameter wider than 64 bits):
* symptom: untyped `Q = P` over >64-bit `P` is E3009
* `fold_self_bits`
* oracles: 2 oracles
* witness (batch 4 lens-diff): a17_untyped_wide_ovr `module c #(parameter P = 1); localparam Q = P;` under `.P(100'h1_0000_0000_0000_0000_0005)`, and h08_wide_untyped_lp_chain `localparam Q = P; localparam R = Q + 1;` over a 100-bit `P`: PRE = POST rc 1 ``parameter `Q` value is not a foldable constant expression``
* a17: iverilog and verilator `A bP=100 bQ=100 P=0000100000000000000000005`, sv2v → iverilog `A bP=32 bQ=32 P=0000100000000000000000005`; h08: iverilog `A 100 101 00000100000000000000000006`, verilator `A 100 100 0000100000000000000000006`, sv2v → iverilog `A 100 82 100000000000000000006` (W/batch4/lens-diff/oracle/)
* status: OPEN
