# lens:soundness round 2 — REPORT (live; updated per question)
status: started
binaries: (pending md5 check)
## Q1 wide_entry_is_stale census — pending
## Q2 bare_ident_route consumers — pending
## Q3 gen_case_region cache — pending
## Q4 value-keyed refusal — pending
## Q5 mutants — pending

## Q1 finding F1 (BLOCKING, fix 1, NEW instance): wide write LATER than narrow at one key
- census: imports processed in SOURCE order (instance.rs:535-580 `for (i, imp) in import_list`); wildcard arm skips a name only if `explicit_imports` already holds it (package.rs:1210); explicit-wide arm (package.rs:1371-1386) inserts `wide_param_bits` and never removes `params` → `import pa::*; import pb::W;` leaves params[top.W]=pa's narrow + wide[top.W]=pb's wide; wide is CURRENT (explicit wins, §26.3) but `wide_entry_is_stale` → true.
- X1 (probes/q1/X1.sv): pre/post `W=10000000000000007 D=10000000000000008 bits=65 hi=1`; post2 `W=00000000000000005 D=00000000000000006 bits=65 hi=0`; iv+sv2v = pre; vl `W=00000005 ... bits=32 hi=1` (self-contradicting: 32 bits yet W[64]=1 → disqualified)
- X3 ($unit wildcard + module explicit): pre/post `W=10000000000000007 eq=1`, post2 `W=00000000000000005 eq=0`, iv/sv2v/vl `W=10000000000000007 eq=1`
- X2 gen-case: pre `arm five` (pre-existing wrong), post `arm a` (= iv, sv2v), post2 `arm five` (back to PRE); vl `arm five`
- X4 (package pc does `import pa::*; import pb::W;`): pre/post/iv/sv2v `Z=10000000000000007 Y=10000000000000008`; post2 `Z=00000000000000005 Y=00000000000000006`; vl = post2 (vl disqualified: X1 self-contradiction)
- X6 multi-consumer: post2 bare W/LI/LW/n/sel/sub.P all read pa's 5, while hierarchical `X6.W` reads 10000000000000007 (routes DISAGREE inside one design); pre/post all wide. iv/sv2v refuse `X6.W` (imported name not hierarchical).

## Q2 (route consumers)
- G1 (round-1 genvar stale class, probes/q2/G1.sv): post2 every consumer (bare, L, LW, i[0], u.P, i+1, gen-if) reads the genvar = sv2v/vl; pre split (display wide, gen-if/range narrow). So in the narrow-current class POST2 is consistent.
- In the F1 class (wide current), POST2 splits: bare/select/param-override narrow vs hierarchical wide (X6); `$bits(W)`=65 from pb's param_meta beside pa's value 5 (X1).

## Q3 region cache
- key uniqueness measured: macro twice in one scope (M1), two includes at identical in-file offsets (I1), named/unnamed loops (L1/L2/L3): no collision (post2 second construct `def 9 bits=4` consistent; a collision would show post's `a 8 bits=4`). Map never cleared (only driver.rs:184 init); only 4 phase calls instance.rs:1040/1167/1217/1355.
- freeze-by-design: M1 second/I0/L1/L2 post2 = PRE `def 9 bits=4`, oracles `a 200 bits=8` (documented forward-ref trade, not a regression vs PRE).
- NEW (non-blocking vs PRE): C1 — label available in EVERY phase but its VALUE changes (P's initializer forward-refs gb.Q; P wide in Nets, then narrow+wide = stale → narrow later): pre `C1 p 8 bits=4 P=10000000000000005` (mix), post `C1 def 9 bits=4 P=10000000000000005` (consistent = iv), post2 `C1 p 8 bits=4 P=00000000000000003` (mix again); iv `def 9 bits=4 P=1..5`, sv2v `p 200 bits=8 P=3`. The cache keys availability, not the arm; fix 1's predicate is itself phase-dependent.
- X5 (narrow in Nets, wide later at top.gb.P — premise violated): post2 `P=3 bits_w=4` consistent (= iv); pre/post `P=1..7 bits_w=4` (mix); sv2v `P=1..7 bits_w=8`.

## Q1 census (grep in src@9bd5cd65)
- wide writers: 8 insert/remove lines: wide_param_range.rs:56 (bind_wide_param ← params.rs 2334/2369/2408, instance.rs 722, generate.rs 783/820), package.rs 388, 657, 998(restore), 1001(remove), 1272(remove, ambiguity), 1277, 1386.
- bind_param_value call sites: 23 (generate 264/384/394/742/803; frames_reserve 1130/1138; instance 206/698/743; package 369/616/687/797/939/1234/1413; net_util 618; params 1742/1983/2032/2463/2530). Direct params writers: frames_reserve 1141 remove, params 897/903.
- str_param_raw writers 6 (generate 751, instance 679, package 624/962/965, params 1910); real_param_val writers 11.
- REMOVERS of params/str/real at a wide write: 0. bind_wide_param touches hier_param_range + wide_param_bits only; package 1277/1386 touch param_meta, pkg_var_aliases/symbols only. "returns before bind_param_value" is true but removes nothing written EARLIER.
- ordered pairs (later writer decides "current"):
  | earlier | later | current | stale→reads | measured |
  | wildcard wide 1277 | enum label instance.rs:206 | narrow | narrow ✓ | B2E (r1) |
  | wildcard wide 1277 | explicit narrow 1413 | narrow | narrow ✓ | B3X (r1) |
  | module wide (params/instance) | genvar generate.rs:264 | narrow (in loop) | narrow ✓ | A1S, G1 |
  | **wildcard narrow 1234** | **explicit wide 1386** | **wide** | **narrow ✗** | **X1 X3 X4 X6 (F1)** |
  | gen-scope wide 783/820 (Nets) | gen-scope narrow 803 (VarInit, fwd-ref init) | narrow | narrow; flips between phases | C1 |
  | gen-scope narrow 803 (Nets) | gen-scope wide 783/820 (VarInit, fwd-ref init) | wide (later) | narrow ✗ but = Nets value | X5 (benign: = iv) |
  | wildcard narrow 1234 (pa) | wildcard wide 1277 (pb) | ambiguous | wide removed, narrow kept (pre-existing, not stale) | — |
  | header/body params vs wildcard | — | local_names skip (package.rs:1204/1262) | no pair | — |
- 252-cell census POST→POST2 raw-output moves: 0/252 (cmp252.py) — CONFIRMED.

## Q4 value-keyed refusal
- check_const_range_bound call sites: 8 (array_geom 312/313/321/572/573, packed 191/192, params 1696/1697 = 9 lines incl. both bounds; 8 per brief counts msb/lsb pairs+Size). const_range_bound_fold callers: 57 lines; all others bypass the refusal (return None → caller default).
- guard reached only when the i64 lane returned None (early return line 972), so the bound's VALUE is the catch-all; the guard only adds loudness.
- B1-B9 masked/derived x (`&&1'b0`, `!==1'bx`, `===1'b0`, `>>1`, `||1'b1 -1`, `x^x`, inside `&&`, `===1'bx +1`): post2 = PRE or better on all 9; zero right→loud. B4 (`>>1`) post loud → post2 1 = oracles. B7 `x^x` not refused (bit domain declines) = PRE (split iv 1/sv2v x/vl error).
- R2 bypass/declining shapes (`(x==?p)|1'b0` in localparam range, function return, typedef): post2 = PRE `P=32 fr=1 t=1` (post refused); iv `1/1/1`, sv2v `x/x/x`. Residue class (masked-x 4-state ops), not a regression.
- stale names: wide_name_bits returns None for a stale key (no fallback to narrow_param_bits) → fold_self_bits declines → never refuses → PRE catch-all.

## Claims re-measured
- md5 PRE 4ace7617… POST 1e41978f… POST2 ed69d5ae… CONFIRMED.
- 0/252 census POST→POST2 (raw output byte compare) CONFIRMED (cmp252.py).
- 14/666 harness CONFIRMED (cmp666.py): 5 =PRE (S05A/B S07A/B K02q); 9 !=PRE: S08A/B S14A/B (refused, wording), K02 (fewer refusal lines), Q3 FNRX/TDX ×(±IV) loud→`FNRX 1` (PRE loud for other reasons).
- 0/174 matrix, 32/410 lens probes, velab 15/15: NOT MEASURED.

## Q5 mutants — running (mut_run.sh, sequential, CARGO_TARGET_DIR=lens_snd/target)
- m5 key without prefix; m2 drop ident_route skip; m8 invert unknown; m1 stale→false
- m5 kill-cell candidate: probes/q3/U1.sv (uA string P → region unavailable, cached first; uB numeric P → region 'a'). post2 `U1.uA.d def|U1.uB.a a`.
- m5 (key without prefix): SURVIVED — `Summary [41.990s] 8933 tests run: 8933 passed, 15 skipped`, rc 0, elaborate recompiled (1 Compiling line), vita md5 8ee72e2269b2db3f4452bac56dd6d69a. Killing cell (unpinned): U1 — m5 `U1.uA.d def|U1.uB.d def`, post2 `U1.uA.d def|U1.uB.a a`, iv/sv2v `uA a / uB a`. M1/L1/I1 unchanged under m5.
- m2 (drop ident_route skip only): KILLED — `cli::generate_case_label_domain a_stale_wide_entry_under_a_narrow_rebinding_is_not_read` (generate_case_label_domain.rs:83, stdout `A1D i=18446744073709551625 K=0`), 8932 passed 1 failed; md5 789af358ba37c599c7df15c8ac042bc0. m2 on X1: `W=10000000000000007 D=00000000000000006` (bare route wide, constant fold narrow: the split the skip prevents).
- m8 (invert unknown test): KILLED — `cli::inside_wildcard a_bound_refusal_is_keyed_on_an_x_value` + `cli::inside_wildcard shapes_without_a_constant_value_are_loud`, 8931 passed 2 failed; md5 201476e4d4eb8732df44ef4d7f0ea188.
- m1 (stale → false): KILLED — `cli::inside_wildcard a_wide_wildcard_fold_does_not_read_a_stale_wide_entry` + `cli::generate_case_label_domain a_stale_wide_entry_under_a_narrow_rebinding_is_not_read`, 8931 passed 2 failed; md5 5afeba76a81b3dab8673eb2d5ca45166.
- m8 on B4 (`>>1`, value 0): loud E3009 (post2 `bits=1` = oracles).
- attribution: m1 binary on X1/X3/X4/X6 = PRE/POST answers (X1 `W=10000000000000007 D=10000000000000008 bits=65 hi=1`), X2 `arm a`, C1 `def 9 bits=4 P=10000000000000005` → F1 and the C1 flip are caused by `wide_entry_is_stale` (fix 1).
- leftovers after run: cargo 0, vita 0.

## VERDICT: FINDINGS
- F1 BLOCKING (fix 1, NEW instance): explicit wide import after a same-name wildcard narrow import → right→wrong silent (X1 X3 X4 X6).
- N1 NON-BLOCKING vs PRE (fix 1 × fix 2): C1 phase mix returns (PRE mixes too; POST was consistent).
- N2 missing pin: m5 survives; U1 kills it.
- Q4: no finding (B1-B9, R2; zero right→loud).
status: completed
