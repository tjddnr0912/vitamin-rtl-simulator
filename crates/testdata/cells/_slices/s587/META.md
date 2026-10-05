# META s587 (phase 1: manifest only, nothing copied)

## Slice
- §4.5.587, ROADMAP row §3.b `unique-const-fn` (§5.2 row at the time).
- Outcome as recorded: "§4.5.587 was reverted whole and nothing of the attempt lands, so this commit is its record." (s587/COMMIT_DOCS.txt:3-4). Title: "`unique-const-fn` reverted whole, BLOCKED on new §2 🆕 AE and 🆕 AD; 🆕 AE is §5.2 row 1" (s587/COMMIT_DOCS.txt:1). Review: "Round 1 killed 13 of 13 mutants, round 2 killed 8 of 8" (s587/COMMIT_DOCS.txt:20); reverted after two BLOCKING rounds (s587/COMMIT_DOCS.txt:21-35).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s587/pre/vita | e1e7e57148bb83ad35d2c4f68247a290 | release (default features) | HEAD 7b33a009 (main) | s587/GROUNDING.md:7 |
| PRE-sep | s587/pre/sep/{vita,vcmp,velab,vrun} | vita d0e1cf9dff9b16fbff5f489f6b35cc15, vcmp f6c729fc39891ff9099cf84042e688f8, velab 2f45610b3b1baa571b95b28f2d8f4f60, vrun 3f42b9d96d8b7fd741758cac34e77c9e | release, separate-bins | HEAD 7b33a009 | s587/GROUNDING.md:9 |
| POST-a | s587/post/{vita,vcmp,velab,vrun} | vita eac2ebb63b51a3fded499002b755b022, vcmp c02b1962618beffe6b8b9b0d3827ee18, velab bf553c27bdc2149f12d878ba5d2024ab, vrun 59a14ecf79bb3533c48b31dd2c8a7d0f | release, separate-bins | `git archive HEAD` + proto.diff md5 f94de040d3dee236a92d50a52b66e1ec (shape (a) prototype) | s587/GROUNDING.md:61,64 |
| POST-a-jit | s587/post/vita_jit | 32fca449f4bbab34b8b2c8f0ebdb48ec | release, --features jit | same as POST-a | s587/GROUNDING.md:110 (G3 JIT row) |
| MECH | s587/mech/vita | 6fe6b4ba1a413517e63c32f5af6cb18d | release, separate-bins | wt branch s587 @ 7b33a009 + step-2 mechanism | s587/IMPL.md:40 |
| S3 | s587/s3/vita | f72f95389855598bff8713b4ac0779c7 | release (separate-bins dir) | step 3 tree | s587/IMPL.md:45 |
| S4 | s587/s4/vita | da2c28ab96a4ccd0052d32a6f0f2b97f | release (separate-bins dir) | step 4 tree | s587/IMPL.md:50 |
| S5 | s587/s5/vita | 9a5525108f2cf61ee3ce0279ba55bd5f | unrecorded | round-2 intermediate (unrecorded) | md5_at_preservation |
| S6 | s587/s6/vita | 8a62233bbe50d76b03c2c28c63e17ae2 | unrecorded | round-2 intermediate (unrecorded) | md5_at_preservation |
| S7 | s587/s7/vita | 623b248e41eb8c68551a6a00e1842b93 | unrecorded | round-2 intermediate (unrecorded) | md5_at_preservation |
| S8 | s587/s8/vita | 804aecd712956ae7ab7bb07cdb4ad9f9 | unrecorded | round-2 tree before post_c ("regression on s8", s587/IMPL.md:162; md5 not written there) | md5_at_preservation |
| POST-b | s587/post_b/{vita,vcmp,velab,vrun} | vita cad7fe9a7721be97634d808055cb73b8, vcmp 6a760e183fae684e0ef8d0a44b06b95a, velab d13af1b89d7a30d8532e3483c920aa12, vrun 6568b9520b2243a98452dfd966e9d912 | release, separate-bins | 7b33a009 + wt.diff md5 4b1f062d497bfb6cfbbfff004e6487c9 + const_site.rs 96fac530… + unique_const_fn.rs d314fdca… | s587/IMPL.md:112-115 |
| POST-b-jit | s587/post_b/vita_jit | f5f5cb52283f4a87701adf9a3b1b8d4d | release, --features jit | as POST-b | s587/IMPL.md:113 |
| POST-c | s587/post_c/vita (+vcmp/velab/vrun) | vita fcae6604862198c224d5baee33948ec2 (vcmp 01016c9c…, velab e275a1ad…, vrun e86d3174… prefixes only) | release | 7b33a009 + git diff md5 6103b1f20eb5ac63f36e474a927bf467 + const_site.rs b6a8cc9f… + unique_const_fn.rs b9efb948… | s587/IMPL.md:181-182 |
| POST-c-jit | s587/post_c/vita_jit | f1cd245ccf95b589f4e8c123ec824a24 | release, jit | as POST-c | prefix s587/IMPL.md:181; full md5_at_preservation |
| VPOST | binary unrecorded | unrecorded | unrecorded | unrecorded | suffix `.vpost` (g2b, g2c, g2d, g2o); byte-identical to the `.post` (POST-a) output in 71/71 cells where both exist (file comparison at preservation, nothing re-run) |
| T-RUN | binary passed to s587/t_run.sh, unrecorded | unrecorded | unrecorded | unrecorded | `.vita` in t/ and t2/; 25 of 50 byte-identical to `.POSTb2`/`.pc`/`.s7`/`.s8` |
| vita-unrecorded | unrecorded | unrecorded | | | 6 VCD rows written by "vitamin-sim 0.2.0" (g6, r1/diff/g6r, r2/diff/re/g6r) and g6/vcdv/out.txt (tag `unrecorded`) |
Suffix -> tag is directory dependent (derived from the harness scripts): run3.sh `.vita`/`obs_` = PRE; vonly.sh `.<sfx>`/`obs<sfx>_` (pres, pres2 = PRE-sep; post = POST-a; mech; s3; s4; s8; pb = POST-b; pc = POST-c); g2 `.pre` = PRE (byte-identical to `.pres` on 152/152); r1/diff run2.sh `.PRE`/`obsPRE_` = PRE-sep, `.POST`/`obsPOST_` = POST-b; r2/diff/re `.POSTC`/`obsC_` = POST-c; r2/diff/{a,b,c} atk3.sh `.PRE` = PRE-sep, `.POSTB` = POST-b, `.POST` = POST-c; x1/run.sh `.PRE` = PRE-sep, `.POSTb` = POST-b, `.s8`; lens_rerun.sh (t, x1, x2, atk, r1/sound/c) `.PRE2` = PRE-sep, `.POSTb2` = POST-b (inferred from the suffix names; the invocation is not recorded), `.pc`, `.s5`-`.s8`; `*.G5` = the same binary run with `-G P=5`; r1/sound/c `.pre.txt`/`.post_b.txt`, r2/sound/c `.pre.txt`/`.post_c.txt` (their run.sh: `$S/pre/vita` = PRE); lanes dirs and post_[bc]/lanes_work `<lane>.out`/`.<lane>` = POST-b (r1) or POST-c (r2, post_c), jit lane = the -jit binary; g3 `.pre.<lane>` = PRE-sep, `.post.<lane>` = POST-a.

## Oracles
- As captured: "iverilog 13.0 fp, verilator 5.052 fu/fp" (s587/IMPL.md:6); harness "verilator --binary --timing --assert -Wno-fatal + `+verilator+error+limit+1000`", iverilog `-g2012` + `vvp -n` (s587/GROUNDING.md:12). sv2v: "not on PATH, not found under / (maxdepth 6). Not used." (s587/GROUNDING.md:11).
- In r1/sound/c and r2/sound/c only the verilator build stderr (`vl_<cell>.build`) and the iverilog stderr (`<cell>_p.ivlerr`) were saved; the run values are quoted in the lens REPORT.md only (33 CELLS rows carry the note "oracle files present but all 0 bytes").

## Verdict sources
- Grounding sets (g1a, g1b, g2, g2/fix, g2b, g2c, g2d + their copies): PLAN P3 moved-cell table, column "direction" (s587/PLAN.md:158-161; names expanded from `b_{pp,sf,…}_{if,pif}_M` and suffixed `_if_M` per the table header "all if-form M unless noted"); family-E lists "NEW pre-existing silent-wrong found" (s587/GROUNDING.md:98) and "More pre-existing silent-wrongs of family E" (s587/GROUNDING.md:141). Every other grounding cell: no per-cell class word -> `unrecorded`.
- g6: IMPL step-1 lane lists KEEP / DROPPED / refined / Residue / Pre-existing (s587/IMPL.md:9-34), by lane id (g01a …).
- x1: IMPL X1(c) census groups "WRONG -> excluded" / "WRONG but unreachable by the arm" / "RIGHT -> stays opted in" / "LOUD in both / no data" (s587/IMPL.md:129-145), by site name; the class is the plain twin on PRE vs both oracles.
- x2: IMPL X2 line (s587/IMPL.md:159-161).
- Lens cells (r1/diff, r1/sound, r2/diff, r2/sound): that lens's REPORT.md, in order: "## Findings" line naming the cell, a VERDICT line naming it, the first class word after the cell's name on a line, a "## FINDING X (…)" header with the cell named within 3 lines below. Notes say which rule fired.
- t, t2, g4, probe, plan_probe: no class word -> `unrecorded`.

## Layout
Flat dirs, a cell = `<stem>.sv`, sibling outputs `<stem>.<suffix>`, verilator build log `vl_<stem>.build`, vita obs dirs `obs<sfx>_<stem>/{run.json,events.jsonl}` (r1/sound/c, r2/sound/c: `obs_<stem>_<pre|post_b|post_c>/`); lanes_work: `<cell>/<lane>.out`.
| suffix | role | tag |
|---|---|---|
| .sv | sv | - |
| .ivl, .ivl.G5, .ivlerr | oracle_iverilog | iverilog |
| .vl, .vl.G5, vl_<stem>.build(.G5) | oracle_verilator | verilator |
| .vita .pre .pres .pres2 .PRE .PRE2 .post .vpost .mech .s3-.s8 .pb .pc .POST .POSTb .POSTb2 .POSTB .POSTC .pre.txt .post_b.txt .post_c.txt .native/.interp/.vm/.staged/.jit, `*.G5` | vita | per the suffix table above |
| obs*/run.json, events.jsonl | vita | per obs prefix |
| .vcd | vcd | iverilog (g6/vcdv) / vita-unrecorded |
| .svh, extra/<cell>_*.sv | aux | - |
| .vu .velab .pre.vu .post.vu .vvp lib.toml | excluded | |
Example: s587/g1a/a_unique_if_M.{sv,ivl,vl,vita,pres,pres2,post,mech,s3,s4,s8,pb,pc} + vl_a_unique_if_M.build + obspb_a_unique_if_M/.
Generators kept: g1a/gen.py, g1b/gen.py, g2/gen.py, g2b/gen.py, g2c/gen.py, g2d/gen.py, g6/gen.py, x1/gen.py, x1/gen2.py, x2/gen.py. Runners kept: run3.sh, t_run.sh, x1/run.sh, r1/diff/run2.sh, r1/sound/c/run.sh, r2/diff/atk3.sh, r2/sound/c/run.sh.
Verdict tables kept (113): per-binary `*_summary.txt` digests of every grounding set, g1_table.txt, g2_static.tsv, g2b/oracle_M.txt, g2c/oracle.txt, g2d/oracle.txt, g6/g6_pre_table.txt, x1/census_final.txt (+ census_pre_postb, census_s5), x2/census_final.txt, post_b/p3_*/p4_*.txt, post_c/p3_*/p4_*/lens_census_moves.txt, r1/diff/atk_lanes.txt, r1/diff/m48/lanes.txt, r1/sound/optin_sites.txt, r2/diff/lanes_c.txt.

## Counts
- Cells (non-alias) 725: g1a 40, g1b 76, g2 24, g2/fix 8, g2b 44, g2c 22, g2d 5, g4 8, g6 141, g6x 1, probe 6, plan_probe 2, x1 80, x2 28, t 30, t2 2, r1/diff/atk/u 36, r1/diff/atk/p 42, r1/diff/atk2/u 4, r1/diff/atk2/p 4, r1/sound/c 27, r2/diff/a/u 23, r2/diff/a/p 23, r2/diff/b/u 2, r2/diff/b/p 2, r2/diff/c/u 2, r2/diff/c/p 2, r2/sound/c 41.
- Aliases 2262 (reruns and copies: r1/diff/c444/*, r2/diff/re/*, r1/diff/g6r, r2/diff/re/g6r, r1/diff/m48, r2/diff/m48c, post_b/post_c lanes_work + velab_work, r1/diff/velab, r2/diff/velab, post_c/lanes2_work, g2o 32, g3 12, t 21, …); their outputs are manifested where not byte-identical to the kept copy's output of the same role+tag (6662 alias rows not copied).
- Field recovery over 2987 CELLS rows: verdict_class recovered 1542, unrecorded 1445 (source same); vita_tags unrecorded 1059 (alias copies with no output of their own, e.g. velab_work); oracles: 6 rows doc-quote only (r1/sound/c nest_u, nest_p, pkk_p; r2/sound/c pkbody_u; probe/p5; post_c/lanes2_work copy of nest_u), 1480 rows `none-on-this-copy` (aliases); binary md5: 15 of 18 tags recovered (VPOST, T-RUN, vita-unrecorded not); base commit recovered for PRE, PRE-sep, POST-a, MECH, POST-b, POST-c; S3-S8 partly.
- CELLS columns oracles_present / vita_tags list the outputs of that copy, including outputs that are MANIFEST `alias` rows (byte-identical to the kept copy). Generated helper files in this work dir (build.py, scan*, stats.txt, noora_cells.txt, run_stats.json; s588 also pc_lib.py / pc_engine.py / pc_stats.py) are not part of the manifest.

## Not recoverable / excluded cells
- Vita-only, excluded as `no-oracle-output` (238 md5 groups, 9587 files incl. obs dirs, 6421633 bytes; list in noora_cells.txt): g2 128 + g2/fix 16 + g2b 40 + g2c 18 + g2d 23 grounding cells (the G2 run-time census positions had oracle text captured only for moved positions: "oracle text for every moved position: $S/g2o/*.{ivl,vl}, $S/g2b/oracle_M.txt", s587/GROUNDING.md:68) and their c444/re/velab_work copies; plan_probe pk_simple2, pk_simple3, pk_u0; probe/p3b; r1/sound/c pkk_u, pkq_u, pkret0_p, st19_u; r2/sound/c/stg (pkg.sv, top_u, top_p: staged two-CU experiment); t/t1_params_fp2, t/t3b_label; 2 g3 copies.
- G2 A's 36 "keep" position kinds have no oracle file, so their cells are not preserved (s587/GROUNDING.md:71).

## Size
- Manifest: 14371 files, 9271272 bytes (excluding 6662 alias rows). Of these, vita obs-dir files (run.json + events.jsonl) 4419 files / 4185652 bytes (selectable by the `/obs*_*/` path component).
- By role: vita 11377 / 7637643; verdict_table 113 / 897955; oracle_verilator 1430 / 372219; sv 725 / 218638; oracle_iverilog 700 / 81673; generator 10 / 48080; runner 7 / 7070; aux 6 / 6747; vcd 3 / 1247.
- By set (largest): g6 1907583; g1b 1037293; g2b 719826; g2c 528709; x1 397739; g2 387640; g1a 324447; r1/diff/g6r 302494; r1/diff/atk/u 240308; g2/fix 179860; post_b/lanes_work 164140; post_c/lanes_work 164140; r1/sound/c 161277.
- 10 largest: g2_static.tsv 83765; g1_table.txt 25629; g6/g6_pre_table.txt 24094; g1b/summary.txt 22168; x1/census_final.txt 22134; g2b/oracle_M.txt 20410; g6/pres2_summary.txt 17904; g6/mech_summary.txt 17198; g2/pre_summary.txt 16786; post_b/p4_velab.txt 16471.
- No file > 1 MB; no set > 5 MB (total 9.27 MB over all sets).
- VCD (flag: repo .gitignore has `*.vcd`): 3 manifested (g6/g05g_vcd_u.vcd, g6/g05g_vcd_p.vcd vita; g6/vcdv/g05g_vcd_p.vcd iverilog) + 4 alias rows.
- Third-party screen: 0 hits.

## Excluded (see EXCLUDED.tsv)
build-dir 1344 / 536789937 (proto_target, proto_target_jit); binary 48 / 358465360; repo-copy 1392 / 22950015 (proto/); mutation 155 / 21834192 (mut/, mut2/); no-oracle-output 9587 / 6421633; vita-staged-artefact 6544 / 5226370; doc-draft 10 / 4531589 (docs/, rows.txt, cg.json); log 82 / 4525247 (logs/, logs_impl/, markers); rust-patch 8 / 367110 (attempt/, proto.diff, post_[bc]/wt.diff, *.rs); slice-record-doc 8 / 119159 (GROUNDING, PLAN, IMPL, COMMIT_DOCS, 4 lens REPORT.md: the verdict sources, parent decides); script 30 / 49237; iverilog-image 4 / 14924. Accounting: 40245 files walked = 21033 manifest rows + 19212 excluded.
