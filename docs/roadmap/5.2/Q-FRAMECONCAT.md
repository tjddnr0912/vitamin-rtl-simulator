(§5.2 Q-FRAMECONCAT · rank ① · source row 33 probes c5, f3, f5 (W/row33/probe/c, W/row33/probe/d)) (a concatenation assigned to a frame-local unpacked array fills it in reverse):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* a concatenation assigned to a frame-local unpacked array fills it in reverse: `int u [2]; u = {32'd5, 32'd6};` prints `R 6 5` (c5), sv2v → iverilog and verilator `R 5 6`, iverilog `R 0 0`, the module twin f3 E3009
* a silent wrong on a legal §10.10 program
* `u = 64'h0000_0005_0000_0006` (f5) is the same order `R 6 5`, verilator %Error CONST, iverilog `cannot be implicitly cast`, sv2v → iverilog `R 5 6`
* PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt)
