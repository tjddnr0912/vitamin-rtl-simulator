(§3.b udp-unnamed-instance · OPEN · in Parser accept) (an unnamed UDP instance is a parse error):
* symptom: a UDP instance with no instance name (`pu (q, a, b);`) is refused E2002 `expected identifier, found '('`, where IEEE 1800-2017 §29.8 makes the name optional and iverilog 13.0 and verilator 5.054 run it `1 0`; loud; PRE == POST (cell v28, W/batch6/lens2-diff/cells/V/v28_udp_unnamed_and_named; batch 6's differential lens round 2, W/batch6/lens2-diff/REPORT.txt RG5; RULINGS L2-6 FI5)
* note: §5.2 Q-WEXTPOS's debug check H15v part (ii) (no connection starts at its item's start) rests on this refusal; the row that lifts it re-reads H15v (ii) first (Q-WEXTPOS.md)
* status: OPEN
