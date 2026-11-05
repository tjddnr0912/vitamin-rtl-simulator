(§2 runtime-time-literal · OPEN · in Inline / frame binds) (a time literal in a run-time operation keeps the integral context):
* A time literal in a run-time operation keeps the integral context: `a8 + b8 + 0ns` (a8 = b8 = 8'hff) 510, iverilog 13.0 = sv2v 254 (IEEE 1800-2017 §5.8, ieee1800-2017.txt:3126–3127: a time literal is `realtime`; §11.8.2, :12689–12691: the non-real operand is self-determined)
* PRE 01cac4b3 (Q-WITHREAL r2 lens-diff N3, TL_add2)
* 2 oracles
* status: OPEN (§5.2 Q-TIMELITCONST, or row 74 if its census shows the run-time kind there)
