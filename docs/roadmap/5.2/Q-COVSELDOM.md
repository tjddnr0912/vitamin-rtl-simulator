(§5.2 Q-COVSELDOM · rank ① · source row 24 r4 cell cov_oneside (plain twin cov_oneside_p)) (a coverpoint on a constant part-select gets the wrong bin domain width):
* symptom: a coverpoint on a constant part-select gets the wrong domain width: `logic [40:8] w; localparam bit [3:0] X = 4'd15; coverpoint w[X + 4'd12 : 9]` after one sample of 0 reads `cov=1.5625` (one of 64 bins) where IEEE 1800-2017 §12.5 (`X + 4'd12` = 11 at its own width: `w[11:9]`, 8 auto bins) gives `12.5` (iverilog refuses the covergroup)
* evidence: PRE = r4 on native, vm and interp
* status: silent-wrong, pre-existing
