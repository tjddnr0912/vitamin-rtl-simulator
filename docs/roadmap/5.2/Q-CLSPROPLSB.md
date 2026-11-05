(§5.2 Q-CLSPROPLSB · rank ① · source Q-MEMBSRC plan probe clsk/lsb) (a class property with a non-zero LSB is selected as if declared from bit 0):
* symptom: a class property with a non-zero LSB is selected as if declared `[w-1:0]`: `class K; logic [15:8] vv = 8'hA5;` and `vv[9:8] vv[15] vv[1:0] vv[1]` in a method print `xx x 01 0` where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12351–12353, 12384–12388) gives `01 1 xx x`
* witness: the module-scope twin prints `01 1 xx`
* oracles: verilator 5.052 `01 1` on the in-range pair
* evidence: PRE 9d60771d
* status: silent-wrong
