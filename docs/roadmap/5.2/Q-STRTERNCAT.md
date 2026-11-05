(§5.2 Q-STRTERNCAT · rank ① · source Q-NESTCAT r1 lens-sound b2/U37tern, U37ternd; r3 lens-diff b13) (a string `?:` as a concatenation operand drops the arm):
* slot: in row 74, with Q-STRTERN
* a `?:` with string arms as a string-concatenation operand drops the arm: `r = {{c ? a : t}, "!"}` and `r = {c ? a : t, "!"}` (a = "xy", c = 1) print `[!]`, verilator 5.052 `[xy!]`
* iverilog 13.0 and sv2v vvp syntax error
* IEEE 1800-2017 §11.4.12.2
* PRE 01cac4b3 = Q-NESTCAT r2
* a `?:` whose arms are string concatenations the same: b13 `{"<", (k ? {a, "t"} : {a, "e"})}` (k = 1) `[at] 2` on PRE 3e541d67 and Q-NESTCAT r1–r3, verilator 5.052 `[<at] 3` (iverilog 13.0 and sv2v vvp syntax error)
* the `?:` door of §3.b string-door-pairs
