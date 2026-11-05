(§3.b string-concat-unsized · RECORD · in Strings / heap) (an unsized constant operand in a string concatenation is accepted):
* (NOTE, not queued) — an unsized constant operand of a string concatenation is accepted: `r = {a, 65}` (a = "xy") is `[xyA] 3` = verilator 5.052, where iverilog 13.0 and sv2v refuse ("Concatenation operand "'sd65" has indefinite width.")
* IEEE 1800-2017 §11.4.12 (ieee1800-2017.txt:11976)
* the nested `{{a}, 65}` is `[yA] 2` on PRE 3e541d67 (Q-NESTCAT) and `[xyA] 3` on Q-NESTCAT r1–r3
* Q-NESTCAT r1 lens-sound b8/K1uns, K1unsd (W/qnestcat/replan4/cells/u)
* status: RECORD
