(§3.b hier-array-query · OPEN · in Constants / parameters) (`$size` and `$left` of a hierarchical name are refused):
* symptom: `$size`, `$left` (and the other array query functions) of a hierarchical name are refused E3009 `unsupported system function in expression`, where both oracles run; the manual's 003 table said "Supported" without that limit (corrected at batch 6's landing); loud; PRE == POST (batch 6's differential lens, W/batch6/lens-diff/REPORT.txt D4 G2; RULINGS L1-13)
* witness: a05_hsize_selidx and a06_hleft_range (W/batch6/lens-diff/cells/A): vita `error[VITA-E3009] E-ELAB-UNSUPPORTED: unsupported system function in expression`; iverilog 13.0 and verilator 5.054 `v=0140 size=6` (a05) and `v=03ff l=9 r=0` (a06)
* note: `$bits` of a hierarchical name is §5.2 Q-HIERBITS's (a provisional constant patched late); this entry is the refusal of the other query functions
* status: OPEN
