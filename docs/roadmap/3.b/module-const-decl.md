(§3.b module-const-decl · OPEN · in Parser accept) (a module-level `const` declaration is a parse error):
* symptom: `const logic [3:0] c = 'z;` as a module item is refused E2002 `t.sv:2:9: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected identifier, found keyword 'logic'`, where IEEE 1800-2017 §6.20.6 allows a `const` variable there and iverilog 13.0 runs it `zzzz xxxx`; loud; PRE == POST (cell f66, W/batch6/lens2-diff/cells/F/f66_static_const; batch 6's differential lens round 2, W/batch6/lens2-diff/REPORT.txt RG5; RULINGS L2-6 FI5)
* note: §3.b tf-decl-lifetime holds only a `static` declaration after a task header
* status: OPEN
