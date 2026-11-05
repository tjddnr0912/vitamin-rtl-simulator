(§3.b frame-array · OPEN · in Subroutine / frame) (multi-dimensional, offset, copied and `foreach` frame-local arrays):
* multi-dim / offset / copied / `foreach` frame-local arrays
* a local whose dimension folds negative with no assignment is accepted: `bit u [((4'h6 == ~4'h8) ? 1 : -1)]` (D_as_false_bare) `R ok`, verilator a size error
* `foreach` on a frame-local is E3009 ``enum method `u.first` is unavailable`` (ds5k, W/row33/lens-sound/c12; oracles `R a a`)
* ① a frame-local dimension that does not fold reads as a scalar, a silent wrong: `function automatic int g(); int a [h()+2]; return $size(a); endfunction` (q1, q2, q3; W/row33/lens-diff/p4) `R 32`, iverilog, sv2v and verilator `R 2` (row 33 r1 F7 and lens r1 d 1; filed here because Q-FRAMEDEFAULT, row 33's residue row in its record docs.patch, does not exist while row 33 is parked)
* PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt)
* oracles: iverilog
* status: OPEN
