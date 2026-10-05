# s581 — §2 🆕 T stage 1 (generate-case label in 4-state domain)
status: step 0 started (reading canon)
## step 0 (done): canon read (CONTRIBUTING, ER 2/4/5/6/7/10, LOOPROMPT), ROADMAP S/T rows, archive 4.5.580, revert diff.
- PRE baseline re-run: t1..t4 default, c1 default, c2 item, c3 default (matches brief).
- LPA binds at HEAD (65-bit 1).
## step 1 site census (code)
- DECIDES an arm: elaborate/src/generate.rs GenItem::Case (~432) ONLY.
- WALKS all arms (Match|Default bodies, never read labels): generate.rs:1014 collect_gen_sva_decls; gen_enum.rs:66,170;
  block_local_class.rs:123,197; expr_size_hier.rs:189 gen_has_defparam, :403 census_gen; toplevel.rs:225,285;
  cond_names.rs:164; md_return.rs:87; decl_collide.rs:497 (span only); generate.rs:1041 gen_items_anchor (span only).
- No GenItem::Case arm: param_dup.rs (Item/Block only), instance.rs:483 (Item only), ast_query.rs, package.rs (no GenItem).
- hdl-parser/src/generate.rs: builds only. => no routing split mirror.
## step 1 oracle census (census/gen.py, run.py, table.py; cells/*.sv; 205 cells)
- KEY FINDING: generate-case SIZING is an oracle split. verilator 5.052 gen-case = whole-case §12.5 (context-determined
  at longest width, unsigned if any unsigned) = all 3 tools' PROCEDURAL case = vita runtime. iverilog 13.0 gen-case (and
  sv2v->iverilog) = each expr at its OWN width, pairwise extend, pair sign -> contradicts its own procedural case
  (probes2/w01p.sv: W01p b vs gen W01 a). verilator quirk: W06 (4'sb1111 vs 4'sb1111 + 8'd0) def procedurally too.
- genvar arith: iverilog $bits(i-1)=3, runtime (i-1)===32'hFFFFFFFF =1 but gen-case def (self-contradiction) (probes2/n09q.sv).
## step 2 design (implemented, crates/elaborate/src/gen_case.rs)
- fold every expr of the case in the bit domain at own width (fold_selfdet_operand, param_leaf_bits names).
- fully-wide case (scrutinee known + every label folds): per label compute PAIR reading (iverilog) and WHOLE reading
  (§12.5/verilator, via const_wide::fold_in_region = extracted comparison-region operand rule). agree -> decide;
  disagree -> keep i64 answer (PRE) else refuse SizingSplit.
- otherwise legacy: i64 answer (PRE) ; else x-valued own fold -> non-match ; else known -> refuse NoCaseWidth ; else Unfoldable.
- scrutinee gate unchanged (i64). const_wide.rs: pure extraction of `at` closure -> fold_in_region; fold_selfdet_operand pub(crate).
## debug-POST census: changes vs PRE listed in T_census.tsv (class column) — see step 5
## step 3 implemented: gen_case.rs (new), generate.rs (call site), lib.rs (mod), const_wide.rs (extract fold_in_region),
   tests: cli/tests/generate_case_label_domain.rs (new, 9 tests; composite designs re-run on iv/sv2v/vl/PRE: dump verified),
   cli/tests/inside_wildcard.rs (XB/XD x-left ==? labels now REFUSED pin; XI/FOLD kept).
## step 4 scoped gate: running (gate/*.log)
## step 4 scoped gate: build -p cli rc=0; nextest -p cli --no-fail-fast 7917/7917 pass (1 skipped); nextest -p elaborate 65/65.
   first cli run (fail-fast) failed param_default_takes_declared_type::a_generate_case_string_label_compares_as_an_i64:
   KNOWN-WRONG rows (signed -1 vs 16'hffff / "\377\377", both oracles hit) now hit -> pin converted (oracle re-measured).
## step 5: POST-T frozen postT/vita md5 f70204a92bf3789eca759db0938b485c size 7304752 (release). T_census.tsv written.
## census final (T_census.tsv, 215 cells): unchanged 148 | skip->match 10 | skip->refuse 21 | decided->changed 36
- decided->changed agree(3-oracle) 35; SPLIT 1 = N09 (genvar i-1 vs 32'hFFFFFFFF: vl a_0, iv/sv def_0; iverilog self-contradicts)
- skip->refuse: PRE-wrong 10; illegal(all oracles refuse) 2; PRE accidentally right 8 (C1 X19 X21 S44 M06 M10 M11 S48); split S34 (PRE=iverilog)
- residue unchanged wrong: T4 X13 X23 (constant InsideEq-as-Eq -> stage 2)
- mutants M1(always pair) M2(always whole) M3(no legacy x-skip) M4(pair always unsigned): all killed (mut/*.log)
- examples: stdout+VCD identical PRE vs POST. release rebuild on final tree md5 identical to postT.
## final scoped gate: gate2/*.log (running)
## final scoped gate (gate2): build rc0; nextest -p cli --no-fail-fast 7918/7918 pass (1 leaky: func_ret_packed_md::a_select_on_a_call_is_loud_on_every_route, not LEAK-FAIL; absent in gate/cli2 run); nextest -p elaborate 65/65; clippy -p elaborate -p cli --all-targets -D warnings rc0; fmt --check ok.
status: stage 1 DONE, awaiting stage 2 brief.
# ROUND 2 (coordinator message: stage-1 revision then stage 2)
## A2 done: refusals removed (GenCaseChoice/GenCaseRefusal/gen_case_refusal_note deleted); a label the bit domain cannot
   decide keeps PRE exactly (i64 answer else non-match). inside_wildcard.rs restored to HEAD.
## A3 name-axis census (cells NA*, 31 cells, label+scrutinee, arm nets printed => phase-consistent: no x, no missing arm)
- moved (all closures, oracle-agree): NA07S NA11M NA11G(sv2v+vl; iverilog cannot bind gen-enum label) NA11S(sv2v+vl; iverilog reads outer EM) NA12S NA13S
- unchanged right: NA01-04 L/S, NA06 L/S, NA07L, NA08 L/S, NA12, NA13L, NA13I
- unchanged pre-existing: NA0xF forward ref E3010 (iv def, sv2v/vl a); NA09L/S net-shadow (all oracles refuse; vita reads param, ROADMAP 'Constant domain' bullet);
  NA10L/E block import E3009 (oracles a); NA10T block `let` (iv/sv refuse, vl a; PRE/POST def)
- cross-check instrument (TEMP VITA_GCX) over all 252 cells: NAME leaves where both domains answer: 0 disagreements
  (wide-only: 65-bit params T1 L18 L28-30 M08 O08 P04 P05 S48; i64-only: NA09 net-shadow).
  EXPRESSION level EXTDIFF only from the i64 lane having no width: fill '1 read as 32 ones (F01 F02 F05 F07) and
  unlimited arithmetic (W01 W02 W03 W08 W09 W10 W11 W12 W18 W19 W20: i64 16/300 vs self-width 0/44). No LOWDIFF anywhere.
  => cross-check NOT added (a literal expression-level check would revert 3-oracle closures W03 W11 W19 to PRE's wrong def;
     the name-level check fires nowhere). Flagged open.
## A4 split pins: W01 W04 W08 W09 F01 S13 S15 S17 S27 O15 (both oracle texts) in a_split_sizing_keeps_the_i64_answer.
## A1 N09 pin with iverilog self-contradiction + procedural twin (N09p) in the same block.
## A5 velab PRE vs POST-T (vcmp+velab, corpus-runner args): 11 corpus + 4 examples: 15/15 .velab SAME, 15/15 .vu SAME.
   generate-case constructs reached: 0 in every corpus design and example (TEMP instrument; positive control T1=4 calls, N01 velab=12).
## A6 POST-T re-frozen: postT/vita md5 37d2b613f937979ade730b6808175e22 size 7304736 (old refusal build kept as postT/vita.refusal-build)
   census (252 cells): unchanged 200 | skip->match 10 | decided->changed 42 (39 agree incl. NA11G/NA11S 2-oracle with iverilog disqualified by
   unshadowed control probes2/na11u.sv; N09 split ruled) | value->loud 0 | loud->value 0
   gate3: build 0; nextest -p cli 7917/7917 (1 skipped); elaborate 65/65; clippy 0; fmt ok.
## CHECKPOINT written: T.patch + T_new_files/ (gen_case.rs, generate_case_label_domain.rs)
# STAGE 2 (constant half re-landed: git diff 3711e0e5 eab81ef2 minus generate.rs; const_wide wildcard arm hand-applied onto fold_in_region)
- holds_xz_wildcard: only remaining use = const_eval.rs range-bound refusal (kept). Measured: r26 S05A/B S07A/B S08A/B S14A/B, K02, matrix AD_X RB_X:
  PRE silent (`$size` x / 1 bit), POST E3009; oracles 2 (compound pattern) or split iv 1 / sv2v x (x-valued). value->loud only on x-valued (LRM-illegal) or PRE-wrong cells.
- cmp4 (666 variants, PRE=HEAD, post3=eab81ef2, NEW=post/vita): both 427 | NEW=post3!=PRE 181 | NEW=PRE!=post3 7 (X1 X4 p3G9 p5H4 p6J1-3: post3's gen-case refusal) | neither 51 (all gen-case label lane: NEW item where PRE/post3 default; K02 differs only by post3's gen-case refusal line)
- ocheck on the 238 NEW!=PRE-or-post3 variants: NEW=oracle 171, NEW=PRE(stdout) 28, no-oracle 21, NEW-loud 9 (holds_xz_wildcard cells above), NEW!=oracle 10 — all NEW=post3 and prior-ruled in s580 (sv2v literal rewrite: CD1 P08b Q1 Q5 with iverilog=NEW; iverilog untyped-param width outlier Q9 UEV/UEB, verilator=vita; P2c run-time OVX (sv2v x); Q3 PSEL pre-existing residue G3)
- S2 matrix (s2/cells, 174 cells, S2_matrix.tsv): unchanged 112 | loud->value 30 (29 POST=oracle, OV_I split: iv/vl 1 1, sv2v 1 32) | value->value 30 (28 POST=oracle, OV_W split, SR2 iv/sv 1 vl 0) | value->loud 2 (AD_X RB_X, x bound, split) ; POST!=post3 only BG_*/BW_* (T's 65-bit label lane)
- gen-case census on post/vita: moved T2 T3 X13 X23 M04 R08 (skip->match, oracle-confirmed); C1 X21 M06 M10 unchanged output (now decided: x non-match); T4 unchanged (compound pattern refused by the fold) -> residue
- binder census B* (module untyped/65-bit, package, generate-scope 65-bit, override untyped/65-bit, scrutinee) x forms: every newly bound value read by the label lane (no non-match default).
- gate4 running
- inside_wildcard::shapes_without_a_constant_value_are_loud (eab81ef2 pin) expected the gen-case compound-label refusal; converted to a RESIDUE value pin (`default`, sv2v item) per decision A2.
- generate_case_label_domain.rs: new test a_constant_wildcard_label_is_read_in_the_bit_domain (T2 T3 X13 X23 M04 R08 move; C1 X21 M06 M10 decided), T4 moved into the residue pin. Composite re-run on sv2v (all lines) and iverilog (==? cells).
- gate5 (final): build 0; nextest -p cli 7920/7920 (1 skipped); elaborate 65/65; sim-engine const_domain_semantics 15/15; clippy 0; fmt ok.
- POST frozen: post/vita md5 1e41978f78724cc11b691cf6babfe911 size 7304768 (release rebuild on final tree = measured binary)
- velab PRE vs POST: 15/15 .velab SAME, 15/15 .vu SAME
- S_census_gencase.tsv (gen-case census, s2 column): unchanged 194 | skip->match 16 | decided->changed 42
status: stage 1 revision + stage 2 DONE.
# REVIEW ROUND 1 FIXES (POST2)
- B1: const_wide.rs wide_entry_is_stale(key) = wide entry AND (params|str_param_raw|real_param_val) at key; wide_name_bits declines, ident_route.rs bare_ident_route skips Wide.
  census of wide_param_bits writers: package.rs 388 (pkg-fn prefix install, skip formals), 657 (intra-package sibling, restored 998), 1277 (wildcard import), 1386 (explicit import), wide_param_range.rs:56 bind_wide_param (callers params.rs 2334/2369/2408, generate.rs 773/810, instance.rs 722 — each returns/continues before bind_param_value). No binding writes wide + another map at one key.
  cells: A1S A1L A2S A3S B2E B3X B4P -> PRE (=oracles); A1D i=0/1 K=0/1 and B2D E1=1 K=1 now = oracles (pre-existing twins fixed); st1 loud (=PRE), no wrong value.
- B2: lib.rs gen_case_region (prefix, span.lo, span.hi) -> region available on first elaboration (Nets; phase order instance.rs:1040 Nets, 1167 VarInit, 1217 Logic, 1355 Instances). gen_case_choose(.., use_region) -> (body, region_ok).
  cells: D1 D2 -> def 99 (=PRE, no E3010); D3 a 1; D4 def 99, D5 o=99 (=PRE; POST's a/o=1 was a hidden phase mix); d1w def 9 bits=4 (=PRE); d1p k 8 bits=4 (=PRE, residue).
- B3: const_eval.rs refusal = holds_xz_wildcard(e) && fold_self_bits(e) has unknown bits. right->loud 0. refused: S08A/B S14A/B AD_X RB_X K02:18 (x-valued). back to PRE: Sxf Sxg Sxk Sy2 Sy3 Sy5 (right), Sxc Sxj Sxe Sy1 Sy6 G6 MD1 S05 S07 K02:15/16 (PRE wrong; compound or masked-x fold declines), G5 G9 G11 G12 G13 (`1 + x`: fold declines on arithmetic with x; split iv 1 / sv2v x; G13 PRE 32).
- W1 doc, dF4 header reworded; pins: A1S+A1D, B2E+B2D, d1w, Sxf+Sxg, x-valued bound refusal, MC4, st1 (oracle-checked r1v/).
- gate6 running
- gate6: build 0; nextest -p cli 7925/7925 (1 skipped); elaborate 65/65; const_domain_semantics 15/15; clippy 0; fmt ok; release 0.
- POST2 frozen post2/vita md5 ed69d5ae6fa11e12d4220550f6391b42 size 7304768.
- POST2 vs POST: census 0/252, S2 matrix 0/174, 666 harness 14 (listed in DELTA_R1.md), lens probes 32/410 (30 = PRE, A1D B2D = oracles), velab 15/15 SAME vs PRE.
- DELTA_R1.md written (hunks, cell tables, census, phase evidence, mutants, residues). r1_delta_src.patch / r1_delta_tests.patch.
# ROUND 2 DECISION: revert T + S wide half; POST3 = S i64 half only
- POST3 s581/post3/vita md5 33067d7744033c67c45ad43be49d95da size 7304736; gate7: build 0, cli 7913/7913 (1 skipped), elaborate 65/65, const_domain_semantics 15/15, clippy 0, fmt ok.
- moved vs PRE: census X13 X23 only; S2 matrix loud->value 15, value->value 25, value->loud 0; 666: 103+26 moved (oracle-checked); lens 71 (all oracle); velab 15/15 SAME.
- BLOCKER: >64-bit-width left operand with a fitting value vs an x/z pattern (P68 = 68'hC): p3 G2/G7 gen-case label right->wrong, localparam/gen-if right->loud (blk/). Not guarded. Proposed fix: PRE masked compare as the fallback when w > 64.
- holds_xz_wildcard removal measured: with the guard and no arm it still fired on inside x bounds (S05A/B S08A/B K02:14/15/18), because HEAD's wide fold reads inside as == (x).
## PREREQUISITE ROWS (for docs)
P1 one current binding per key (stale same-key wide/narrow entries; resolvers ask one map first):
  cells A1S A1L A2S A3S B2E B3X B4P st1 (genvar/enum/explicit import over a stale WIDE entry: bit domain read the stale wide value),
  A1D B2D (run-time twins: `$display(i)` 18446744073709551625, `E1` 18446744073709551616 on PRE),
  X1 X3 X4 X6 q1g q1g2 q1n (round 2: `import pa::*; import pb::W;` leaves a narrow W beside the CURRENT wide W; skipping the wide entry read 5 where PRE and the oracles read 2^64+7).
  Both readings of "which entry is current" fail one family, so only the binders can make it one binding per key.
P2 phase-stable generate-case label resolution (maps leak across GenPhase walks; Nets runs first, instance.rs:1040):
  cells d1w (first build: Nets default net + Logic arm-a process `a 8 bits=4`), d1p (PRE already mixes `k 8 bits=4`), D1 D2 (first build E3010), D4 D5 (hidden mix),
  r1 r1b r1p C1 U1 (round 2: a forward label resolves to an outer same-named constant in Nets and to the inner one later while the region exists in both phases; right->wrong against PRE).
🆕 T measured design (for the row; built and reverted):
  - compare each label two ways: PAIR (iverilog's generate-case: each expression at its own width, pair sign) and WHOLE (§12.5 = every tool's procedural case: longest width, unsigned if any unsigned); decide only where they agree, else keep the i64 answer.
  - sizing split: W01–W20 (iverilog gen-case contradicts its procedural case; verilator sign-extends a signed item in a wider unsigned case, W06/T07p).
  - 42 decided->changed closures (T_census.tsv: L24 S06 S07 S11 S16 S21 S23 S29 S30 S32 S36 S37 S39 O01 O02 O03 O11 O12 O21 N09(split, ruled) N10 N11 W03 W11 W16 W19 E03 E05 E06 R04 R06 R07 P01 P02 P03 P06 NA07S NA11M NA11G NA11S NA12S NA13S), 10 skip->match (L06 L26 L30 T1 X09 M08 N08 R05 P04 P05).
  - with S's wide half also: T2 T3 X13 X23 M04 R08 skip->match (X13 X23 already move with the i64 half alone).
S's wide half (held, needs P1 and P2): LP `localparam [64:0] LP = {64'd0, (4'b1100 ==? 4'b1?00)}` (+ `case (1) LP` item), st1 (stale genvar read through the wide arm), BW_* BG_* BOW_* LW_* (65-bit binders), GF_* (generate-for condition), two compares under || / two-element sets (Q6 I2 rb2 P4a rb2q H03), x-valued bound refusal (S08 S14 AD_X RB_X, oracle split).
Residues still open (from round 1, for docs): dF2 W6'(P8) label; sF3 indexed part-select width with x ==? (append to PROBE_CATALOG line 83); dF3 type-param `'1` (not recorded anywhere); d1p.
# POST3b (fallback adopted)
- wildcard_eq.rs: at w > 64 const_wildcard_i64 -> const_wildcard_masked_pre (HEAD's masked compare verbatim). gate8: build 0, cli 7914/7914 (1 skipped), elaborate 65/65, cds 15/15, clippy 0, fmt ok, release 0.
- POST3b post3b/vita md5 6aab206f39960ecfd78a576159ed811c size 7304736.
- vs POST3: census 0/252, S2 0/174, lens 9/520 (blk + 6 ladder cells, all = PRE), 666 3 (p3 G2 G7 LV, = PRE); ladder 18/18 = PRE; velab 15/15 SAME.

# FINAL (reverted whole)
Outcome: §4.5.581 attempted §2 🆕 T and §2 🆕 S (a)'s constant half; three review rounds; reverted whole (§4.5.571/572 precedent). MAIN tree = e54fa74a plus one held-cell test file `crates/cli/tests/generate_case_and_wildcard_prerequisites.rs` (12 tests, all at today's output, oracle lines in comments; oracles re-run on $SCR/fin/*.sv).

## Design commits (branch fix/gencase-label-domain)
| commit | held | binary |
|---|---|---|
| d4dc9c26 | 🆕 T: generate-case label in the 4-state bit domain (gen_case.rs, both-readings compare, const_wide::fold_in_region extraction), refusals dropped (unreadable label keeps PRE) | postT 37d2b613 |
| e350ef79 | + 🆕 S (a) constant half re-landed from eab81ef2 (const_wildcard_i64, fold_region wildcard arm, wildcard_match, holds_xz_wildcard range-bound refusal) | post 1e41978f |
| 9bd5cd65 | round-1 fixes: wide_entry_is_stale (wide_name_bits + bare_ident_route skip), gen_case_region first-phase cache, value-keyed bound refusal | post2 ed69d5ae |
| eb9d3b69 | round-2 decision: T and S's wide half reverted, S's i64 half alone + w>64 fallback to PRE's masked compare (const_wildcard_masked_pre) | post3 33067d77 / post3b 6aab206f |

## Blockers by round
- Round 1:
  - F1 stale same-key wide entry: A1S A1L A2S A3S B2E B3X; B4P on POST; st1 on the S side (Q=0 K=9).
  - F2 phase-dependent region: D1, D2 (E3010); d1w (`a 8 bits=4`).
  - dF1 node-keyed bound refusal made right bounds loud: Sxf Sxg Sxk Sy2 Sy3 Sy5.
  - mut_c survived; MC4 added.
- Round 2:
  - F-A / X1, the opposite stale case: X1 X3 X4 X6 q1g q1g2 q1n. `import pa::*; import pb::W;` leaves a narrow W beside the CURRENT wide W; skipping the wide entry read 5 where PRE and the oracles read 2^64+7.
  - F-B forward label resolving to an outer constant in Nets and the inner one later, while the region exists in both phases: r1 r1b C1 (right→wrong vs PRE).
  - Both axes blocking twice → T and the wide half reverted.
  - i64-half blocker found by me: P68 width > 64 (p3 G2/G7, blk/b1, blk/b2) → the w>64 fallback (POST3b).
- Round 3, POST3b:
  - S3-1 / dF1 unknown-width decline: `const_self_width(lhs)?` returns None before the fallback. A parameter-count replication, a select or an array element as the left operand goes right→loud or wrong (`({N2{4'b1100}} ==? 8'b1?00_1100)`: PRE 1, POST3b E3009, oracles 1; G1 takes `default`).
  - dF2 / PT5 loud→wrong through a pre-existing sign loss: `PV ==? 4'sb1?00` over a parameter typed by an overridden signed type, PRE E3009, POST3b 0, oracles 1 (PT5c `PV < 0` already wrong on PRE).
  - Budget spent → reverted whole.

## Prerequisite rows
- P1, one current binding per key:
  - Binders: genvar setup, enum label, explicit and wildcard imports rebind a key without clearing the other map's entry.
  - Cells: A1S A1L A2S A3S B2E B3X B4P st1 (stale WIDE read first); A1D B2D (run-time twins, wrong on PRE: `$display(i)` 18446744073709551625 K=9, `E1` 18446744073709551616 K=0); X1 X3 X4 X6 q1g q1g2 q1n (stale NARROW beside the current wide).
  - A resolver-side "skip the other entry" fails one family either way.
  - Held cells: A1D (wrong), A1S (right), X1 (right).
- P2, phase-stable generate-case label resolution:
  - Mechanism: the GenPhase walks (Nets first, instance.rs:1040; then VarInit, Logic, Instances) re-resolve labels through maps that leak across phases.
  - Cells: d1w d1p D1 D2 D4 D5 r1 r1b r1p C1 U1. Held: d1p (PRE mixes, `k 8 bits=4`), d1w (PRE consistent wrong `def 9 bits=4`).
- P3, the sign of a constant typed by an overridden type parameter:
  - PT5c `PV < 0` = 0 (oracles 1).
  - dF3 `localparam T TP = '1` under `#(.T(logic signed [3:0]))` = 15 (oracles -1). It was in no queue (grep of ROADMAP, PROBE_CATALOG, REMAINING_WORK).
  - PT5 `PV ==? 4'sb1?00` loud on PRE, 0 through the i64 half. Held: PT5c, dF3, PT5 (loud).

## 🆕 T measured design (for its row)
- Compare each label two ways and decide only where they agree; otherwise keep the i64 answer (else a non-match). Nothing refuses.
  - PAIR: iverilog's generate-case — each expression at its own width, the pair's sign.
  - WHOLE: §12.5, which is every tool's procedural case — longest width, unsigned if any is unsigned.
- Sizing oracle split, W01–W20:
  - iverilog's generate-case contradicts its procedural case.
  - verilator sign-extends a signed item inside a wider unsigned case (W06, T07p).
- Census, T_census.tsv (252 cells):
  - 42 decided→changed: L24 S06 S07 S11 S16 S21 S23 S29 S30 S32 S36 S37 S39 O01 O02 O03 O11 O12 O21 N09 (split, ruled) N10 N11 W03 W11 W16 W19 E03 E05 E06 R04 R06 R07 P01 P02 P03 P06 NA07S NA11M NA11G NA11S NA12S NA13S
  - 10 skip→match: L06 L26 L30 T1 X09 M08 N08 R05 P04 P05
  - with S's wide half also: T2 T3 X13 X23 M04 R08
  - velab: 0 generate-case constructs in the corpus and examples.
- Held cells: T1 L06 S06 X13 N09.

## S (a) i64 half (const_wildcard_i64, common width ≤ 64, pair sign, pattern_ext_fill)
- Moved, POST3 vs PRE: S2 matrix loud→value 15, value→value 25; census X13 X23; 666 harness 129 moved; lens probes 71 moved. All oracle-confirmed except the two failing classes below.
- Class 1, a width decline: the routine returns None where PRE's own-width masked compare answered.
  - w > 64: P68. Closed by the fallback in POST3b.
  - Unknown width, `const_self_width(lhs)` None: `{N2{…}}`, a select, an array element. Not closed.
- Class 2, loud→wrong over a value whose sign is already lost (P3): PT5.
- Uniform-fallback measurement that would decide a re-attempt: make EVERY decline of const_wildcard_i64 fall back to PRE's masked compare, keyed on the decline, never ahead of it. Then re-run:
  - the round-3 cells: r3a, G1, select / array-element left operands;
  - the four cells the routine fixed: `(4'd15+4'd1) ==? 5'b1?000` = 1, `SP ==? 4'sb?100` = 1, `localparam L = 4'b1100 inside {4'b1?00}`, MC4;
  - the S2 matrix, cmp4, the lens dirs and velab.
  
  If every moved cell is oracle-confirmed and PT5 stays loud (needs P3, or a decline when the left operand's type comes from a type parameter), the i64 half can land alone.
- Held cells: r3a (right), P68 (right), PT5 (loud).

## Residues (for docs)
- dF2: a parameter-width cast label `W6'(P8)` keeps its i64 answer (T30: vita default, iverilog / sv2v / verilator hit), plus 17 random cases. Class "a label no bit fold reads keeps PRE", with L07, S44, S48.
- sF3: an indexed part-select WIDTH with an x-valued `==?` is silent on every build (G2b `y=0001`; iverilog and sv2v refuse) → append to PROBE_CATALOG line 83.
- Bound classes (PRE's one-bit catch-all, unchanged):
  - compound pattern: G6 MD1 S05 S07 K02:15/16
  - masked x in 4-state & | ?: — Sxc Sxj Sxe Sy1 Sy6
  - arithmetic with x: G5 G9 G11 G12 G13, Q3 FNRX/TDX
  - x-valued compare as a bound: S08 S14 AD_X RB_X (oracles split; LRM illegal)
  - two-element set / `||` under the wide domain: Q6 I2 rb2 P4a H03
- 65-bit signed fallback cells H10/H36 (round-3 lens; recorded by the coordinator).
- d1p (pre-existing phase mix); the label-lane residues X05 X19 X20 L07 L12 L13 L14 M11 S44 S48 (4-state ops, reals, string-parameter arithmetic, hierarchical names, call scrutinee); scrutinee-still-loud S01 S03 S04 S19 S28.
