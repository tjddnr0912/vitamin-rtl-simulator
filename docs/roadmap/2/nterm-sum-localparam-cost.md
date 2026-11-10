(§2 nterm-sum-localparam-cost · PERF · in Performance (open, recorded)) (an n-term `1+…+1` localparam costs superlinear time):
* symptom: a localparam whose value is an n-term sum `1+1+…+1` takes superlinear time: 2000 terms 0.14 s, 8000 3.4 s, 30000 killed at 30 s; the same sums as a struct member width stay at 0.07 s
* witness: e_add_lp_8000 `localparam P = 1+1+…+1;` (8000 terms), `$display("L P=%0d", P);`: PRE `L P=8000` in 3.45 s, POST 3.39 s; PRE = POST at every n (W/batch2/lens-sound/probe3.SUMMARY.txt)
* note: batch 2 lens-sound N6 (W/batch2/RULINGS-lens-sound-r1.txt), not batch 2's
* status: PERF
