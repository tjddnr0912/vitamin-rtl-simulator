(§5.2 Q-FRAMEREAL · rank ① · source row 33 probe e4) (a real assigned to an element of a frame-local array is not rounded):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* a real assigned to an element of a frame-local array is not rounded: in `function automatic int f(); int v [2]; v[1] = 2.5; …` `v[1]` reads 0 (W/row33/probe/e/e4)
* verilator, sv2v → iverilog and iverilog 3, as a module element and a frame-local scalar 3
* IEEE 1800-2017 §6.12.2
* silent wrong on PRE
* PRE 79814020 and main's e0888973 alike (W/row33/park/REMEASURE-addendum.txt)
