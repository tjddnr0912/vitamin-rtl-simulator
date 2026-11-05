(§2 gen-case-i64-compare · OPEN · in Constant domain (i64)) (generate `case` compares two i64 values):
* Generate `case` compares two i64 values
* size the case and items once (`fold_bits_at`), not pairwise (§4.5.555; §4.5.594's D21: `case (X + {N{1'b0}}) 8'hFC:` over a signed `X` `GC=def`, iverilog, sv2v, verilator `GC=fc`, its literal-count twins too, and 🆕 AI's fix leaves it, so the same expression's generate-if then disagrees)
* 2 oracles
* OPEN
