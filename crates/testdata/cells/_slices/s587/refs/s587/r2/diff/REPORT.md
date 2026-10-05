# lens:differential round 2 — §4.5.587 delta (post_c)
status: STARTED
## Q0 re-measure implementer numbers on post_c (r2/diff/re, my round-1 cells + 444; capture = shell-style interleave)
444: post_c == post_b on all 444 (text); 48 != PRE (= P3 list), 396 == PRE.
plain twins (atk p 33, atk2 p 4, g6r _p 71 + d14 etc. = 117): post_c == PRE == post_b in 117/117.
unique forms: post_c != post_b only in d01, d02, e02 (each == PRE) and d10 (PX/PI decline, PS folds; != PRE, != post_b); 91 others == post_b.
(first attempt compared stdout+stderr concatenation vs shell interleave -> 100% "diff": harness-format, discarded)
(first atk3 run invalid: nullglob+ls listed cwd as extra files -> E8005; rerun)
## Q3b x2m/x2n (implementer cells, re-run r2/diff/x2r)
x2m u: PRE E3009 (Q) rc=1 -> post_b = post_c `Q=7 P=0000`; verilator `Q=7 P=xxxx`; p twin PRE = post_b = post_c `Q=7 P=0000`; iverilog(p) refuses P.
x2n u: PRE E3009 (Q) -> post_c `P=0000 Q=7`; verilator `P=xxxx Q=7`; p twin PRE `P=0000 Q=7`.
Verdict: pre-existing exposure, not a slice regression (reasoning in final report).
## Q4 docs (post_c wt.diff: CHANGELOG, manual 006 §1.4, 003 row; const_site.rs module doc)
D1 "a package routine's own declarations or body folded from a module ... keeps its refusal": the census movers b_pkx/b_pki
 (package fn body with the miss, called from a module parameter) fold silently on post_c (in the 48) -> the body clause overclaims.
D2 "anything inside a generate block that declares a function or task" -> held regardless of whether the text binds to the routine (see r15/r16/r17).
D3 "A replication of a string literal keeps its old count fold": the old fold is a silent-wrong in the unique form (e02 `s=` vs `s=abab`, no W4031) and
 the code also covers string VALUES (implementer note) -> doc hides a pre-existing silent-wrong and under-states scope.
D4 "a function that also reads a 4-state variable it never fully assigned": the hold is per outermost fold (x2f callee arm + caller x, x2g sibling)
 -> wider than "a function"; conservative, wording only.
## Q5 lanes on post_c (lanes_c.py = round-1 lanes.py with PB=post_c): 48 census movers + 23 round-2 designs: native == interp == vm == JIT == staged in 71/71 (JIT fired 11).
## Q1/Q2/Q3 round-2 designs (r2/diff/a/{u,p}; 5-way PRE / post_b / post_c / iverilog(p) / verilator(u,p)) — partial, r15-r22 pending rerun
r01 genfor single instances + shadow: u.C = oracles (g[0] p=5, g[1] p=a); p right. r04 bind into IA element type: u.C = verilator; r05 grandchild in IA element: u.C = oracles.
r02 2-D instance array, r03 interface array: loud in vita (u = p = PRE), no slice data.
r06 `fa += 1` / `fb++` after miss: post_b 0001/0001 (vl xxxx) -> post_c declines both (= PRE). r09 t*0, loop never assigns, `return t`, recursion, x passed as arg: post_b 0000/0000/0000/0010/0000 (vl+ivl xxxx) -> post_c declines all 5.
r07 9 probes (bit/part-select read, struct member, block-local, reg local, time / implicit [3:0] / implicit 1-bit return, unpacked array local): post_c declines all 9 (post_b folded 8).
r08 t&0, $bits(t), t===t: post_c declines all 3 where oracles give Q1=0000 Q2=4 Q3=1 (over-decline, = PRE; plain twin loud on $bits(t) anyway).
r10 loop-assigned, bit return, integer assigned before read: post_c S1=0010 S2=0000 S3=5 = verilator.
r11 import q::* + q::g / g(2) (g's return range [f(2):0] calls q's own f, no shadow): P1/P3 + g range decline (= PRE), P2 = f(2) folds; plain twin folds them (values hidden by a pre-existing E3009 on the package parameter range PR) -> over-decline, loud.
r12 module fn calling q::f: u.C only the pre-existing `package-scoped call ... needs a body` E3009 (run-time call), plain right.
r13 package importing package (q2::g -> f): u.C P=4 = oracles. r14 `export q::f`: E2002 parse (pre-existing).
r23 package and module in two files (module text offsets inside the package's per-file span): u.C P=7 R=3 bw=8 = oracles -> no cross-file span collision.
r15 gen level WITH a routine (h) whose nested level calls the MODULE's f: post_b P=7 (= ivl = vl, plain right) -> post_c declines (= PRE) -> over-decline, loud.
r16 routine only in the inner level, outer text calls module f: post_c P=7 bw=8 = oracles.
r17 generate-if condition / generate-for condition of a construct whose begin-end block declares a routine: post_b correct (g taken, blk 0, blk 1) ->
 post_c E3010 both (= PRE) -> the construct header (outside the block per IEEE scoping) is held too; over-decline, loud.
r18 function declared after use: post_c P=7 bv=8 = oracles.
r19 macro text (`define outside the package) in a package routine's return range; r20 macro in a routine-declaring gen level; r21 gen-level body from
 `include; r22 package body from `include: post_b silent-wrong (232/232; P=7 bw=8) -> post_c declines all four (= PRE). Span rule holds for macros/includes.
F3 NEW BLOCKING r24_unit_shadow ($unit-scope h with return range [f(2):0], $unit f returns 3; module top has a same-named f with the miss):
 u.PRE E3009 rc=1 -> post_b = post_c `v=232 P=232` rc=0; verilator(u,p) + iverilog(p) `v=8 P=8`; plain twin PRE = post_c `v=232 P=232`.
 $unit text is neither a package nor a gen level -> text_foreign false -> the X1b class (routine text resolved in the caller's scope) leaks.
r25 block-local re-declared per loop iteration: post_b PR=0001 (vl, ivl xxxx) -> post_c declines (= PRE). X2 marks the re-declaration.
r26 module fn calling q::f at P / [mf(2):0]: u loud on the pre-existing unique-pkg-closure E3009 (PRE same); B=$bits(mr(0)) E3009 in plain too.
r27 `t[a-2]` (plain loud too) and `^t` after a miss: post_b folded X (vl/ivl 000x) -> post_c declines.
## Q6 .velab on post_c (velab_c.py): nonmoved 256 same / 0 differ / 140 no pair; moved 10 differ + 38 post-only; header ...23 (35) both.
## Systematic (27 round-2 designs + plain twins)
plain twins changed PRE->post_c: 0. W4031 dropped: 0. unique movers PRE loud -> post_c rc=0: 9 = 8 match oracle + r24 MISMATCH (F3).
post_b moved but post_c back to PRE: 12 = 8 were post_b silent-wrong (r06 r07 r09 r19 r20 r21 r22 r25, r27) / 4 over-declines where post_b was right
 (r08 Q1/Q3, r15, r17; plus r11 P1/P3 in the import case).
## Findings round 2 (most severe first)
F3 BLOCKING NEW instance, same root class as X1b / round-1 F1-F2 pattern: r24_unit_shadow ($unit routine text resolved in the caller's scope)
N6 NON-BLOCKING: over-declines r08 (t&0, t===t), r11 (import q::*: g's range), r15 (gen level with routine, call binds to module f), r17 (gen-if/for header) - PRE-identical, loud
D1-D5 docs (D5: $unit routines are neither held nor listed; with F3 the CHANGELOG's "folds silently, as both reference tools do" is false for a $unit routine's range)
x2m/x2n: pre-existing exposure (ER §3.6), not a slice regression
status: COMPLETE
