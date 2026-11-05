(§5.2 Q-STRCMPCAT · rank ② · source row 26 r1 lens-diff b13 s01) (a one-element concatenation over a string `?:` compares as packed, not string):
* slot: after 74
* symptom: a single-element concatenation `{X}` over a string `?:` compares packed: `c = 1; s = "ab"; b = ({c ? s : "xy"} < "b")` `0`, verilator 5.052 `1` = hand IEEE 1800-2017 §11.4.12.2 (a concatenation with a string operand is a string) and the string compare
* evidence: PRE 9d60771d
