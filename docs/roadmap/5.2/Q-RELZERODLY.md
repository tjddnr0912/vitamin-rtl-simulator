(§5.2 Q-RELZERODLY · rank ① · source ENG-1c r2 lens-sound N-3 (q2f)) (a `#0` driver of a forced net changing in its release step lands twice):
* symptom: a `#0` continuous driver of a forced net whose right-hand side changes in the step of its `release` lands twice in that step: `logic [7:0] a8 = 1; wire [3:0] y; assign #0 y = a8;`, `force y = 4'd9` at 5, `a8 = 18; release y;` at 10 (q2f): vita on every backend (Q-RELDELAY §4.5.624 and ENG-1c alike) `W t=10 y=1 \| W t=10 y=2` = verilator (`W t=10 y=1 \| W t=10 y=2`)
* witness: iverilog = sv2v `W t=10 y=2`, = PRE (73d22f5e)
* cause: the release restores the last landed value (1) and the `#0` landing of 18 follows in the same step
* oracles: an oracle split (iverilog = sv2v vs verilator)
* fix: decide or record in §2 "Oracle splits"
* status: pre-existing (Q-RELDELAY's)
