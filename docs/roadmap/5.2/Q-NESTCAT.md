(§5.2 Q-NESTCAT · rank ② · source row 26 r1 F2 (n04); Q-NESTCAT r1–r3 lenses; W/qnestcat/replan4/cells) (a string concatenation nested in a concatenation drops bytes):
* slot: in row 74
* slot: PARKED 2026-10-30 from slot 4 after three blocking review rounds on two axes — a replicated operand evaluated once per copy (Q-STRREPEVAL; r3 lens-sound F1, a `let` call) and a string route that lands on consumers wrong for strings on PRE (r3 F2 / DF1: a `?:` or `let` node in a packed concatenation, the integral cast, a select, a `wait`) — and DF2 (§3.b string-door-pairs)
* slot: W/qnestcat/replan4/PLAN4.txt
* slot: the r1–r3 patches stay in W/qnestcat and are not restored, ER §3.6
* a string concatenation nested in a concatenation or replication drops bytes (a = "a"): `{{8'h41, a}}` `[a] 1`, iverilog 13.0 = sv2v = verilator 5.052 `[Aa] 2`
* `{{2{a}}}` `[a] 1` (`[aa] 2`)
* `{"<", {8'h41, a}, ">"}` `[Aa>] 3` (`[<Aa>] 4`)
* `{{"A", a}}` `[a] 1` (`[Aa] 2`)
* `{$size(arr){{s}}}` (s = "ab") `[bbb] 3` (`[ababab] 6`)
* IEEE 1800-2017 §11.4.12.2 (ieee1800-2017.txt:12041–12058)
* PRE 9d60771d
* silent-wrong
* fix: `expr_is_string_ast`'s concatenation and replication arms (a string leaf at any depth), landing with the string route's `check_handle_assign` (Q-HANDLERET; r1 without it: `h = {{8'h41, a}}` E3009 → `H1 0`, H9alias `h == o` 1) and zero grant (Q-ZEROREPL; r1 without it: `r = {{a}, {0{8'h41}}}` `[a] 1` = all three → E3009)
* with every prerequisite in, nothing is excluded (r2b, which excluded nothing, = the oracles on n12 n13 n14 n15 t13 t14 f22 f23)
