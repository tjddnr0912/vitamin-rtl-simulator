# lens:differential round 2 (delta cf5076ef)

status: complete (see Findings at end)

## Binaries

## DD1

## DD2

## DD3

## DD4

## DD5

## DD6

## DD7

### Q1 (Q1.sv, 42 L const + 42 R runtime twins + 3 multi-element; h2.sh SV=1 VL=1)
- POST2 const L01-L42, M01-M03 = iverilog `==?` twin on every cell (carry, borrow, <<, ~, unary -, concat, replication, ?:, $signed, $unsigned, size cast, $clog2, $bits, 63/64/65/127/128/129, signed >>> at 4/64/65/66, unsized 'b1?010 / 'bx_1?010, !=?). PRE/POST: whole design E3009 (L01.. / L09..).
- verilator differs on L20 L23 L32 L33 (sign in `inside`) and M02 (sizes all elements to the max): iverilog + sv2v agree with vita; sv2v differs on L12 (cast) only.
- CANDIDATE: R41 `sa >>> 1 inside {4'sb111?}` (sa=4'sb1110) POST2 0 (inside and ==?), iverilog 1, sv2v 1, verilator 1; const twin L41 = 1. R42 same at 64 bits. Runtime-only; attribution pending (Q2).
- W cells (x literal in a known-valued lhs, `-DWX`): POST2 E3009 "4'b111x has no constant-fold arm" (loud).

### Q2 (Q2.sv) — runtime signed wildcard compare, lhs width >= pattern width
| cell | PRE in | POST in | POST2 in | PRE ==? | POST ==? | POST2 ==? | iverilog ==? | sv2v->iv | verilator | const twin POST2 |
|---|---|---|---|---|---|---|---|---|---|---|
| R41 `sa>>>1` vs `4'sb111?` (sa=4'sb1110) | x | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | C41 1 |
| R42 `s64>>>1` vs `64'shFFFF_FFFF_FFFF_FFF?` | x | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | C42 1 |
| R43 `sa+s8` vs `8'sb1111_111?` | x | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | C43 1 |
| R44 `s8n/8'sd2` vs `8'sb1111_111?` (s8n=-4) | x | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | C44 1 |
| R46 `s8n>>>1` vs `4'sb111?` | x | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | C46 1 |
| R47 `sa+s8` vs `4'sb111?` | x | 0 | 0 | 1 | 0 | 0 | 1 | 1 | 1 | C47 1 |
| R48 `(sa>>>1) !=? 4'sb111?` | 1 | 1 | 1 | 1 | 1 | 1 | 0 | 0 | 0 | C48 0 |
| R49 `s8n%8'sd3` vs `8'sb1111_111?` | x | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | C49 1 |
| R52 `s5>>>1` vs `4'sb111?` (s5=5'sb11110) | x | 0 | 0 | 1 | 0 | 0 | 1 | 1 | 1 | - |
| controls R45 (lhs narrower), R50 (u4), R51 (unsigned pattern), R53 (`==`) | | right | right | | | | | | | |

### Q3 (Q3.sv) — DD1 positions + DD5
- POST2 = iverilog on all: generate-if (G1, GS signed `SA>>>1 inside {4'sb111?}`), generate-case scrutinee (GC1) and label (GL), override (OV1/OVS/OV0), param-port default (PD), package localparam (PK1/PK0), const-fn argument (LF1/LF0), typed localparam (LT1), `-G TP4=15` (LG 1; iverilog -P 1), range bound (RB1 4, RB0 2, RBS 4), array dim (AD1 4, ADS 4).
- DD5 no false refusal: x-valued wildcard in unselected `?:` arm of a range bound (RBT 4) / array dim (ADT 4), inside `$bits` (RBB 2), generate-case x label after the match (GA first).
- DD5 loud: XBEFORE (x-valued wildcard label BEFORE the matching label) POST2 E3010, iverilog `GB one` (case equality skips the x label) -> correct->loud candidate, attribution pending. typedef range (TDX) / function return range (FNRX) E3009 (iverilog 1). const-fn body (FBODY) E3009 (recorded residue). generate-for (GFOR) E3010 = iverilog error.
- DD5 silent: REPX `{((4'bx100 ==? 4'b1?00) + 1){1'b1}}` POST2 `REPX 0` rc=0, iverilog "Concatenation repeat may not be undefined" rc=1. PSEL `v8[(4'bx100 ==? 4'b1?00)*3+1:0]` POST2 `PSEL 1`, iverilog `PSEL x`. Same class as recorded R2 residue 2 (x-valued compare in a replication count / part-select), direct shape. Attribution pending.
- STX: `struct packed {…} sx;` at module level is a parse error in PRE/POST/POST2 (unrelated, pre-existing).

### Q4 (Q4.sv) — DD3 OR form, 36 absolute-path cells + 2 upward
- POST2 = iverilog `==?` twin on every cell (net, reg, signed reg, part-select through path, array element, generate-block signal, `t.u.P`, signed `t.u.PS`, x/z left bits under wildcard and compared bits -> x, definite mismatch -> 0, `!=?`, 2-element inside, 70-bit, signed pattern on unsigned path and vice versa, `(t.u.s8b >>> 1)` vs `8'sb1111_111?` and `4'sb111?` (H35/H37 = 1), `t.u.s4 + 8'sd0`). PRE/POST: E3009 "left operand of unsizable width".
- Note: H37 `(t.u.s8b >>> 1) inside {4'sb111?}` = 1 via the OR form, while the same shape on a local (Q2 R46) = 0 via the AND form: vita self-inconsistent across lowerings.
- `t.u.st.a` (struct member through a path): E3010 unresolved in PRE/POST/POST2 (unrelated).

### Q5 (Q5.sv) — DD4 unsized side table
- POST2 = iverilog on all: `'bx1` vs `32'bx1` vs `2'bx1` in mixed order (U13 0, U01 1, U12 0, U14 0), `('bx1)`, let / nested let / let with args (1; iverilog twin = literal, hand-IEEE §11.12), sized let 0, `'hx_0000_0001` 1, `'h?` 1, `'h0?` 0, function / package function / task bodies with unsized (1) and sized (0) patterns, localparam K01 0 / K02 1 / K03 1.
- Compound patterns `(C1 ? 'bx1 : 'b0)`, `$signed('bx1)`, `32'('bx1)` at run time and K04 in a localparam: loud (E3009) in PRE/POST/POST2. No id-collision route found (arena `self.exprs` append-only; no truncate/take sites in crates/elaborate/src).

### Q6 (Q6.sv) — attribution + DD6
- REPX `{((4'bx100 ==? 4'b1?00)+1){1'b1}}` / REPE (plain `==`): PRE/POST/POST2 `0` rc=0; iverilog "Concatenation repeat may not be undefined". PRE-identical, not wildcard-specific (recorded R2 residue 2 class, direct shape).
- PSEL `v8[(4'bx100 ==? 4'b1?00)*3+1:0]` / PSEE (plain `==`): PRE/POST/POST2 `1`; iverilog `x`. PRE-identical, same class.
- XB generate-case `==?` x-valued label before the matching label: PRE/POST `XB one` (= iverilog), POST2 E3010. XI (inside) same: PRE/POST `XI one`, POST2 E3010 (hand-IEEE §27.5/§12.5: x label does not match 1'b1 -> one). XE (plain `==`): `one` everywhere. -> correct->loud introduced by R2-6.
- DD6 string pin: POST2 `G0 1 G1 0 G2 1` = iverilog direct = verilator = sv2v->iverilog; PRE/POST E3010.
- DD6 CD1: POST2 `A=1 A2=0 M=1 M2=0` = iverilog = verilator; sv2v->iverilog `M=0` (sv2v rewrites `'hx`; hand-IEEE §5.7.1 x-extends -> 1). PRE/POST E3009.

### DD7 (dd7.py: every recorded lens_diff variant, define set validated by reproducing the recorded POST text with post/vita, then POST2 vs recorded POST)
- 30 stems, 0 unvalidated, 37 variants identical; 559 cells identical, 156 cells changed — every changed cell is in the implementer's "Intended changes" list (P03, P03 -DB, P05 -DK5, P07, P07 -DLETT, P08, P08 -DIV, P08b -DIV, P09, P09 -DNOP, P10b) and equals iverilog where iverilog runs (K5b / P07_l: let, hand-IEEE). Unintended changes: 0.

### Q7 (Q7.sv) — realistic always_comb shapes of the Q2 class
- Z1 `(acc + d) inside {8'sb1111_111?, 8'sb0000_000?}` (acc=0, d=4'sb1110), Z3 `(q >>> 2) inside {8'sb1111_11??}` (q=-16), Z4 `if ((acc + d) inside {8'sb1111_11?0})`: POST/POST2 0 (inside and `==?`), iverilog 1, sv2v->iverilog 1, verilator 1. PRE inside x/x/0, PRE `==?` 0/0/0.
- Q2 cells on POST2 `--backend interp|vm|native`, inside and `==?`: all 0 -> elaboration (IR) defect, not a backend.

### Q8/Q9 (Q8.sv, Q9.sv)
- POST2 = iverilog: untyped 36-bit localparam vs `32'bx1` 0 / `'bx1` 1; 64-bit `F64` vs `'bx1` 1, `'b0?1` 0, `(F64+1'b1)` vs `'bx0` 1, `!=? 'bx1` 1; OR form on `t.u.i32` (`'b1?` 0, `32'hFFFF_FFF?` 1, `4'sb111?` 1, `36'hF_FFFF_FFF?` 0, `36'shF_FFFF_FFF?` 1); `N=-6` vs `4'sb1x1x` 1 / `4'b1x1x` 0 / `36'sh?_FFFF_FFFA` 1, const = runtime.
- K3 `UE = 4'd15 + 4'd1` (untyped) then `UE inside {5'b1?000}`: POST2 0, iverilog 1 — UE itself is 0 / $bits 4 in vita, 16 / 5 in iverilog (verilator 5.052: UE 0, $bits 4 = vita; iverilog is the outlier). Parameter typing, not wildcard-specific.
- RR1 `t.u.rr inside {4'b1?00}` (real 12.0): POST2 0 rc=0; iverilog refuses `==?` on real; no oracle for `inside` (§11.4.13 `==` for non-integral -> 12.0 vs 8.0 -> 0 under x/z->0). UNVERIFIED.
- IU1 `t.u.i32 inside {'bx1}`: POST2 E3009 (R2-3 by design), iverilog `==?` 1; POST also loud.

## Findings (round 2)
- G1 BLOCKING: run-time AND form, both operands signed, lhs width >= pattern width, lhs holding a sign-dependent operator (`>>>`, `/`, `%`, narrower signed sub-operand): POST/POST2 0, all three oracles 1; PRE `==?` right on R47/R52; POST2 const twins right. wildcard_eq.rs:269 + :334.
- G2 NON-BLOCKING: generate-case x-valued wildcard label before the matching label: PRE/POST right, POST2 E3010 (R2-6).
- G3 NON-BLOCKING pre-existing: x-valued wildcard (or `==`) as a replication count (0) / part-select bound (1), PRE-identical, recorded R2 residue-2 class.
- DD7: 0 unintended changes.
status: complete
