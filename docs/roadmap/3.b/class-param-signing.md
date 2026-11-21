(§3.b class-param-signing · OPEN · in Parser accept) (a class value parameter with a `signed` keyword is a parse error):
* symptom: `class C #(parameter signed P = 1);` is refused E2002 `expected identifier, found keyword 'signed'` (and three follow-on E2002 lines); verilator 5.054 runs it `g=2`; iverilog 13.0 `t.sv:1: syntax error`; IEEE 1800-2017 §8.25 with §6.20.2's parameter declaration (a data type or implicit type with a signing) allows it; loud; PRE == POST (cell c40, W/batch6/lens-diff/cells/B/c40_class_signed; batch 6's differential lens, W/batch6/lens-diff/REPORT.txt D4 G7; RULINGS L1-13)
* note: §5.2 Q-CLSPARAMTYPE holds a class parameter's dropped type, a neighbour
* status: OPEN
