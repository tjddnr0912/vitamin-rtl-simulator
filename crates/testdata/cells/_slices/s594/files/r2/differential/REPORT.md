# §4.5.594 lens DIFFERENTIAL round 2 (delta) — REPORT
status: completed. verdict FAIL (product shakes): 1 BLOCKING (new door, same root class as round-2 F1 / plan's Mcmp_RAE_gi).
binaries: PRE fc478eb9…; post_a 4b3afc99…; post_b 511dffa2f0fdcb6953a4aa9b241ec74b (sep vcmp f8ebe5de… velab 715e142a… vrun dfa8fe36…)
r1 59 cells on post_b: post_b == post_a 59/59 (no movers), staged == one-shot 59/59.
new: n/ 12 cells (R2a–f call-far, Nq_launder, O17 O23 O4 O5 O6) + m/ 4 mutations (O17m O23m O4m discriminating twins; Nq_lit literal twin). staged == one-shot 16/16.
BLOCKING B1 Nq_launder: `localparam Q = fl(0);` (fl reads never-assigned r) then `if ((X + {N{1'b0}}) == (Q + 8'd252))`:
  PRE GI=else · post_a GI=then · post_b GI=then · iverilog GI=else · sv2v GI=else · verilator GI=then (2-state).
  Literal twin Nq_lit (`X + 2'b00`): PRE then → PRE's else was cancellation; the rule "root holds a user call" cannot see a call
  one declaration away. Same root class (🆕 AE 0 through a newly sized region), new door (a constant NAME). Present since post_a.
call-far (must = PRE): R2a sib, R2d cast, R2e ternary cond, R2f $clog2: PRE 0 / post_a 1 / post_b 0 / 3 oracles 1 — = PRE (R5 by design);
  R2b repl count: E3009 all three vita (R1) — = PRE; R2c element index: 1 all — = PRE.
opposite (must keep fix): O17m header default beside pk::fz(1) default, O23m gen-if body + else-if chain, O4m child of call-bearing
  override, O5 $bits(L0) inside call-bearing M: post_b = post_a = fix = verilator (+sv2v; iverilog lossless B=9). O17/O23/O4/O6 non-discriminating (PRE already right).
