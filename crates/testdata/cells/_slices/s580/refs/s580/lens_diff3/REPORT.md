# lens:differential round 3 — REPORT (in progress)
task: lens:differential round 3 (delta cf5076ef..e6c9cc8d)
status: COMPLETED (13 tool calls, 7 designs: P1 H1 G1 H2 G2 G3 P3)
## Q1 R3-1 sign region
(pending)
## Q2 R3-2 OR form deleted
(pending)
## Q3 R3-3 generate-case
(pending)
## Q4 regression
(pending)
## log
- probes written: P1.sv (Q1 signed region: runtime B01-B40, CONSTS K*, KF const-function), H1.sv (Q2/R3-1 hierarchical shapes H1-H8), G1.sv (Q3 generate-case labels L1-L12). Runner run3.py (pre/post/post2/post3 + iverilog -DIV + sv2v->iverilog + verilator).
- P1 base/CONSTS: POST3 = iverilog = sv2v = verilator on all 40 runtime cells B01-B40 and 14 const cells (raw P1*.txt). POST2 differed on 30 runtime cells (intended R3-1). KF const-function: E3009 on all vita (PRE-identical loud).
- H1: H2/H2I/H3/H4 PRE/POST/POST2 = oracle 1, POST3 E3009 "with a signed pattern needs the left operand's signedness" (loud regression, NON-BLOCKING). H8 PRE/POST/POST2 0 (oracle 1) -> POST3 E3009 (improvement). H6 0->1 (= oracle). H1 POST2 1 -> POST3 E3009 (intended R3-2).
- G1: L1 `case (1) {64'd0, (4'b1100 ==? 4'b1?00)}` PRE dflt / POST dflt / POST2 E3010 / POST3 dflt / iverilog item / sv2v item -> CANDIDATE BLOCKING (POST2 loud -> POST3 silent). Plain twin L1P dflt on all vita (PRE-identical class). L2-L6 L8 L10-L12 POST3 = oracle; L7 x-valued dflt = oracle; L9 string E3010 POST2/POST3 (PRE dflt, oracle item).
- Q4 (q4.py, lens_diff + lens_diff2, 170 variants / 1747 cells): 136 identical, 34 changed, all 34 in the intended R3 list (R3-1 8, R3-2 18, R3-3 8); raw q4.out.
- H2 (relative hier): R01-R05 PRE/POST/POST2 0 -> POST3 1 = iverilog = sv2v (fixed by R3-1). R07 uL.P / R10 g.s8 (local generate block!) / R11 uL.g2.s8: POST2 1 (OR form) -> POST3 E3009 (= POST, intended R3-2). R06 loud all. R08/R09 0 all = oracle.
- G2: wide known wildcard labels M1 M2 M3 M4 M6 M7: POST2 E3010 -> POST3 dflt (oracle item); M10 POST3 `one` (oracle `item`, wrong arm); M8 x-valued dflt = oracle. PRE column polluted by unconditional LP param (PRE/POST refuse it) -> re-run as G3.

## FINDINGS (round 3)
F1 BLOCKING (rule: POST2 loud -> POST3 silent). New instance from R3-3; values PRE-identical; root class = PRE-existing wide generate-case label skipped as non-match.
  `case (1) {64'd0, (4'b1100 ==? 4'b1?00)} : item; default : dflt` -> PRE dflt / POST dflt / POST2 E3010 / POST3 dflt / iverilog item / sv2v item (G1_L1).
  Same on G3 M1 (inside), M2 (!=?), M4 (65'(...) cast), M6 (value 0, case (0)), M7 (101 bits); G3 M10 (wide label then `1:`) POST3 `one`, oracle `item`.
  Plain twins L1P / M4P / M9P: dflt on PRE..POST3, oracle item (PRE-existing).
  Mechanism: generate.rs:472-478 (e6c9cc8d) skips whenever fold_self_bits is Some, i.e. also a KNOWN value that the i64 path cannot read; const_wide.rs:1686 selfdet_bits_i64 `if w > 64 || bp_any_unknown(&b, w) { return None; }`. The comment claims "x-VALUED" but no bp_any_unknown test.
F2 BLOCKING (rule: PRE loud -> POST3 silent). Not in the R3 delta (POST2 = POST3); introduced by cf5076ef (round-2 constant ==? fold).
  `localparam [64:0] LP = {64'd0, (4'b1100 ==? 4'b1?00)}; case (1) LP : item; default : dflt` -> PRE E3009 (LP not a constant) / POST E3009 / POST2 dflt / POST3 dflt / iverilog item / sv2v item (G3_M9). Plain twin M9P dflt PRE..POST3.
F3 NON-BLOCKING (correct -> loud). New instances from R3-1 (sign asked at aw == pw); same mechanism as POST2's aw != pw loud.
  `((t.uL.k != 0) ? s4 : s4) ==? 4'sb1?00` H2, inside twin H2I, `mem[t.uL.k] ==? 8'sb1?00_0000` H3, `s8[t.uL.k*4 +: 4] ==? 4'sb1?00` H4: PRE/POST/POST2 1 (H2I PRE x) = iverilog 1 = sv2v 1 -> POST3 E3009 "with a signed pattern needs the left operand's signedness".
  Mechanism: wildcard_eq.rs:281-288 asks canonical_self_width for every signed pattern; packed.rs:463-466 returns None for ANY placeholder in the subtree (index, ternary condition), although neither affects the result's sign.
## Q1: CLEAN on values. P1 40 runtime + 14 const cells, H2 R01-R05/R08/R09, P3 C01-C03: POST3 = iverilog = sv2v (= verilator on P1).
## Q2: OR form structurally gone (wildcard_eq.rs:266-273 single call site -> E3009). Measured loud on POST3: lens_diff P08/P09, lens_diff2 Q4/Q8, H1 H1, H2 R07 R10 (local generate block g.s8) R11. POST(value)->POST3 differences: all = oracle (P03 P07 P08b P10b P10c Q2 Q7) except K5b (no oracle; round-2 hand-IEEE) and F3 cells.
## Q3: x-valued before/after/instead/several labels = oracle (L7 L11 L12 M8, Q3 -DXBEFORE, Q6 XB/XI). F1, F2 above.
## Q4: 170 variants / 1747 cells; identical 136 variants (1249 cells); changed 34 variants (288 cells) all intended (R3-1 8 var / 46 cells, R3-2 18 / 180, R3-3 8 / 62); unintended 0.
