(§5.2 Q-STRTERN · rank ② · source row 26 r1 elaborate half, r2 re-plan) (a `?:` with a string arm is not typed as a string):
* slot: in row 74
* a `?:` with a string arm is not string-typed in elaborate (`expr_is_string_ast` / `ir_expr_is_string` have no Ternary arm) and its literal / parameter arms are not string values: `c = 1; r = {c ? s : "xy", "!"}` (s = "abc") `[bc!] 3`, verilator 5.052 `[abc!] 4` (iverilog refuses the arm types)
* `{c ? s : string'(16'h7879), "!"}` with c = 0 `[y!] 2` (verilator `[xy!] 3`)
* `$display(c ? s : "L=%0d", x)` with c = 0 prints ` 327443361892          7` (verilator `L=%0d          7`)
* a numeric arm `{c ? s : 8'h41, "!"}` with c = 1 `[b!] 2` (verilator `[ab!] 3`)
* IEEE 1800-2017 §11.4.11, §11.4.12.2
* PRE 9d60771d
* fix: the classifier types a `?:` with a string arm as string and makes its literal / parameter arms string values (C7, which also closes Q-XNONINT's residue: `c ? P : LQ` at x `[PP] 2`, hand `[] 0`), and refuses a known-integral arm opposite a string arm (P-5)
* classifier gaps (concatenation, replication, hierarchical, parameter, class member, package call) are missed climbs, not refusals
* row 26's elaborate half (F1, S2)
* witness (batch 4 lens-diff, a candidate): b17_str_mentions_real `localparam string S = (R > 1.0) ? "big" : "small";` over `localparam real R = 1.5`: PRE = POST E3009 ``parameter `S` value is not a constant: `R` is a real, which has no integral constant value here``; sv2v → iverilog `A   big two`, verilator `A big two`, iverilog ``cannot be implicitly cast to the target type``
