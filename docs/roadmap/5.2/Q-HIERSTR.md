(§5.2 Q-HIERSTR · rank ② · source row 26 r1 lens-sound p08) (a hierarchical string inside a concatenation or relational prints wrong):
* slot: after 74 and 75
* symptom: a hierarchical string in a concatenation or relational: `{c ? u.s : t, "!"}` with c = 1 prints `[\x01!] 2` (verilator 5.052 `[U!] 2`), with c = 0 `[!] 1` (verilator `[T!] 2`)
* witness: `(c ? u.s : t) < "TT"` with c = 1 `1` (verilator `0`)
* oracles: iverilog aborts (`eval_vec4.c:1360`)
* IEEE 1800-2017 §11.4.12.2
* note: its x lines (`== "U"` `x x`, hand `0 1`) are Q-XNONINT's, the typing of `u.s` is Q-STRTERN's classifier
* evidence: PRE 9d60771d
