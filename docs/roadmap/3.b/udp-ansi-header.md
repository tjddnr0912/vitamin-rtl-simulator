(§3.b udp-ansi-header · OPEN · in Parser accept) (a UDP header with ANSI port declarations is a parse error):
* symptom: `primitive pu(output q, input a, input b);` is refused E2002 `expected identifier, found keyword 'output'`, where IEEE 1800-2017 §29.3.1 allows the ANSI form (`udp_ansi_declaration`) and verilator 5.054 runs it `10`; loud; PRE == POST (cell n11, W/batch6/lens2-diff/cells/N/n11_udp_arr; batch 6's differential lens round 2, W/batch6/lens2-diff/REPORT.txt RG5; RULINGS L2-6 FI5)
* status: OPEN
