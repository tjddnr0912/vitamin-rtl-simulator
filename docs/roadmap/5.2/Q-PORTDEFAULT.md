(§5.2 Q-PORTDEFAULT · rank ① · source row 33 design review (W/row33/design/b5, W/row33/design/b9)) (an ANSI port default is parsed and never lowered):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* an ANSI port default is parsed and never lowered, so a legal §23.2.2.4 program reads the wrong value at exit 0, W3056 `floats at z` the only sign on the input forms, no sign at all on o0 o1: `input bit i = 1'b1` (cp3) `R 0`
* `input logic [3:0] i = 4'h5` (i1) `R x`
* an interface (i0) `R 0`
* `output bit o = 1'b1` (o0) `R 0`, and o1 `R x`, against `R 1`, `R 5`, `R 1`, `R 1`, `R 1` on sv2v → iverilog, verilator and iverilog
* a program (p0) `R 0` against verilator and iverilog `R 1`, sv2v does not parse `program`
* PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt)
* source: W/row33/RULINGS-dr1.txt B2
