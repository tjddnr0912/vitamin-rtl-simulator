(§3.b signed-unbased-literal · OPEN · in Parser accept) (a signed unbased unsized literal (`'s1`, `'sx`) is accepted):
* symptom: `a = 'sx;` and `a = 's1;` are accepted and run (c58 `a=xxxxxxxx`, n06 `a=11111111`; W/batch6/lens-diff/cells/B/c58_unsized_sx, MIN/n06_sx_literal), where IEEE 1800-2017 A.8.7 gives `unbased_unsized_literal ::= '0 | '1 | 'z_or_x` with no sign; iverilog 13.0 `t.sv:1: syntax error` / `Malformed statement`; verilator 5.054 runs c58 `a=00000000`; an illegal text accepted; PRE == POST (batch 6's differential lens, W/batch6/lens-diff/REPORT.txt D4 G12; RULINGS L1-13)
* note: §5.2 Q-LOSSYSPELL counts `'sx` / `'sz` in `unsized_xz`, not as fills (its doc words)
  * note (2026-11-21): the record named in the line above (as built in batch 6, pulled out with the row's park on 2026-11-21: nothing of it is on main; its re-entry after §5.2 Q-FILLCTX re-plans the bits, Q-LOSSYSPELL.md)
* status: OPEN
