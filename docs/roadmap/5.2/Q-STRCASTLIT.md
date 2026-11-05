(§5.2 Q-STRCASTLIT · rank ② · source row 26 r1 lens-sound p04a) (`string'(<literal>)` lowers to a bare literal, not a string):
* slot: in row 74
* `string'(<literal>)` lowers to the bare literal `Const` (`elaborate/src/expr_cast.rs:248–251`), so it is not a string value: `c = 1'bx; u = c ? string'("de") : "x"` ``[`] 1`` (hand IEEE 1800-2017 §11.4.11 `[] 0`; verilator 5.052, 2-state, `[x] 1`; iverilog refuses), `$display("[%s]", c ? string'("de") : "x")` ``[ `]`` (hand `[]`)
* with the cast kept as a string value Q-XNONINT's key fires
* PRE 9d60771d
