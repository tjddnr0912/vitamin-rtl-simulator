(§2 signed-param-reads-unsigned · BLOCKED · in Index sealing) (a `parameter signed` with an unsigned value reads unsigned):
* `parameter signed A = 4'd10` reads unsigned (`param_decl_width_opt`; §4.5.559): `parameter signed X = 4'hf` prints `R 15`, oracles -1 (v_s, W/row33/lens-sound/c6; PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt))
* status: BLOCKED (the alias line below)
