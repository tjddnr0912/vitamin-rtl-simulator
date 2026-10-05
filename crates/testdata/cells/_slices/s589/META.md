# META s589 (phase 1: manifest only, nothing copied)

## Slice
- §4.5.589, ROADMAP row §2 🆕 AD ("§4.5.589 — ROADMAP §5.2 row 1, §2 🆕 AD, the package-routine half", s589/COMMIT_SLICE.txt:3).
- Outcome as recorded: "elaborate: a package routine's own text binds its package's functions and constants, not the caller's (§2 🆕 AD)" (s589/COMMIT_SLICE.txt:1), landed as corrections only; docs title "🆕 AD's package-routine half closed as corrections; new §2 🆕 AF, AG, AH and §3.b `pkg-text-open`" (s589/COMMIT_DOCS.txt:1). Measured: "484 grounding, audit and killer cells: 111 silent -> correct, 0 loud -> value, 0 correct -> anything" (s589/COMMIT_SLICE.txt:87-88). Review: 3 rounds, round 3 both lenses PASS (s589/COMMIT_SLICE.txt:116, :132).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s589/pre/vita | e1e7e57148bb83ad35d2c4f68247a290 | release | "HEAD 2f2d3f2d, given" (s589/GROUNDING.md:10); "code of 303703f9" (s589/COMMIT_SLICE.txt:85) | s589/GROUNDING.md:10, s589/IMPL.md:5 |
| PRE-sep | s589/pre/sep/{vita,vcmp,velab,vrun} | vita d0e1cf9dff9b16fbff5f489f6b35cc15, vcmp f6c729fc39891ff9099cf84042e688f8, velab 2f45610b3b1baa571b95b28f2d8f4f60, vrun 3f42b9d96d8b7fd741758cac34e77c9e | release, separate-bins | "(given)" | md5_at_preservation |
| P1 | s589/g/post1/vita | 79431e6c00fea58e276a13c071d518b7 | release (probe) | probe P1 first cut, superseded | s589/GROUNDING.md:12 |
| P1b | s589/g/post2/vita (+post2/sep) | 5cc22f1b9af4e7bd984e3c5c901a0a62 | release (probe) | g/proto_p1b.diff | s589/GROUNDING.md:13 |
| P1c | s589/g/post3/vita | 6713f2d3bfb536af8f51224832a86651 | release (probe) | P1c (+ interp body) | s589/GROUNDING.md:14 |
| P1d | s589/g/post4/vita | 4735f1b5e03f5224f7f16b2cec6c4891 | release (probe) | = g/proto_p1d.diff | s589/GROUNDING.md:15 |
| W0 | s589/post_a/w0/vita | 97f2c2108601d29f8e0eabca1c3c9cc3 | release | HEAD unchanged, rebuilt in wt | prefix s589/IMPL.md:10; full md5_at_preservation |
| S1 / S1-sep | s589/post_a/s1/vita, s1/sep | vita 00358c0d5e815c85b2c9c6593868c345; sep vita 3af888da3b185da6a2530c2464fda7e2 (vcmp 8d51ceb1…, velab 4dc94c12…, vrun 80451f6f…) | release | wt s589 @ 303703f9, step 1 | prefix s589/IMPL.md:15; full md5_at_preservation |
| S2 / S2-sep | s589/post_a/s2/vita, s2/sep | 1c79631658602fd72150f6bb8781d34d | release | step 2 | prefix s589/IMPL.md:19; full md5_at_preservation |
| S3 | s589/post_a/s3/vita | 6578a07516815e024ef2a44520750e96 | release | step 3 | prefix s589/IMPL.md:21; full md5_at_preservation |
| S4 | s589/post_a/s4/vita | 20931a5b7b1bbd951f5eef91d9c16119 | release | step 4 | prefix s589/IMPL.md:24; full md5_at_preservation |
| S5 / S5-sep | s589/post_a/s5/vita (+sep) | d2fd35b7d72e417a438d0cb2f75a8c3e | release | step 5 | prefix s589/IMPL.md:26; full md5_at_preservation |
| POST-a / POST-a-sep | s589/post_a/vita, post_a/sep | f13c0978e2ae4ae9ee26979a3e0e5bd6; sep vita 18074dbe315068e20a3766f97dc89e19 (vcmp cc532ea6…, velab 1790c2c2…, vrun 1ace9b89…) | release | wt.diff md5 6b25f2ffbff4d3c6214d932d753004cd | s589/IMPL.md:50-51, :115 |
| POST-b / POST-b-sep | s589/post_b/vita, post_b/sep | bea240591388d3ef7c91a47c9c052ac6; sep vita 3988d046b1787a985c91d6b15d8e18d4 (vcmp 5a3ca597…, velab b0518560…, vrun 50bd4f3b…) | release | wt.diff md5 aac91c75b291432bf44ae5c9a5077a58 | s589/IMPL.md:131-132 |
| POST-c / POST-c-sep | s589/post_c/vita, post_c/sep | e31d955b047ad8bbca74dd6b563d2f6f; sep vita c36f419961fded9592c82346f879429e (vcmp 41310e56…, velab d7697787…, vrun e1262980…) | release | wt.diff md5 ab107c9ece519a49a5957e6660f87d99 (final tree differs only in the test file, wt_final.diff aee03db9…) | s589/IMPL.md:168-169, :186 |
| impl-M1 … impl-M10 | target_g/debug/vita (gone) | unrecorded | debug | POST tree + one mutation each | s589/IMPL.md:87-99 (no md5) |
| r1s-M0, r1s-M11, r1s-M12, r1s-M13 | rv_sound mutant builds (gone) | M0 02edcd23397c9fe43d3116ea216314b0, M11 556c0399360eff94927dd580c8a82873, M12 43bfd115d4aeca3ca7af2fb51dbc1b7d, M13 5be953d427b73d6f86aee91af758765a | debug | 303703f9 + post_a wt.diff + mutation | s589/r1/sound/mut/M*.vita.md5 (excluded files) |
| r2s-M0, M11, M12, M13, M15 | (gone) | M0 80ec856c3292115031dd83d1ba68e72e, M11 1fc36d1080cdb92b0af5c082550136c7, M12 5f7ef6a9aebb32f66aec71959e51dcb0, M13 570349f5ee21efe3a20849cd1033ee95, M15 c67facfda8f87d8270d13fcf6f4ef305 | debug | post_b tree + mutation | s589/r2/sound/mut/M*.vita.md5 |
| r3s-M0, M17, M18 | (gone) | M0 389a7ae14ab9c51368b2f76c84541a13, M17 b344298d0c9bc07f2dbbdd0ac6281d5f, M18 8828fd915159a61afa9c0ca4b5e896b1 | debug | post_c tree + mutation | s589/r3/sound/mut/M*.vita.md5 |
| OFF-LANE | unrecorded | unrecorded | | | `.off_<lane>` in g/c*: runv.sh with `VITA_PROTO_OFF=<lane>`; which probe binary is not recorded |
| STG2 | unrecorded | unrecorded | staged | | `.stg2` in g/c*: stage1.sh with a non-default binary dir, not recorded |
| ROUTE | unrecorded | unrecorded | | | `.route` in g/c*: route.sh route census (default binary PRE; the argument used is not recorded) |
| unrecorded | | | | | g/tmp/o obs output |
Suffix -> tag: `.pre`/`.pre2` PRE; `.p1` P1, `.p2` P1b, `.p3` P1c, `.p4` P1d; `.p1off`/`.p2off`/`.p4off` = that probe with `VITA_PROTO_OFF=all` (inferred from the suffix and runv.sh's 4th argument); `.p4nb` = P1d with `VITA_PROTO_OFF=ibody` (s589/a/run.sh); `.stg`/`.spre`/`.pres`/`.staged` PRE-sep (stage1.sh default; h.sh); `.post`/`.posta`/`.pa` POST-a; `.posts`/`.spost`/`.sposta`/`.spa` POST-a-sep; `.postb`/`.pb` POST-b; `.pbs`/`.spostb` POST-b-sep; `.postc`/`.pc` POST-c; `.pcs`/`.spostc` POST-c-sep; `.w0` W0; `.s1`-`.s5` S1-S5; `.ss1`/`.ss2`/`.ss5` S1/S2/S5-sep; `.pre.route` PRE; `.post.route` POST-a; `.M<n>` = the mutant binary of that dir's lens (r1s/r2s/r3s) or of the implementer (impl-, post_a/cells); post_a/st2 `out.<tag>` = the matching -sep binaries (its run.sh takes a sep dir); r2/sound/c/two `two.<tag>` attached to ab.sv.

## Oracles
- As captured: "Oracles: iverilog 13.0 `-g2012` + vvp; verilator 5.052 `--binary --timing --assert -Wno-fatal`; sv2v 0.0.13 → iverilog" (s589/GROUNDING.md:18); same in s589/COMMIT_SLICE.txt:4-5. sv2v binary: $S/s580/sv2v/sv2v-macOS/sv2v (s589/g/run4.sh, s589/a/run.sh; md5 ca3f9b7138513e09a88be96aa7b5ea19 per preserve/versions_at_preservation.txt).
- Harness keeps only `.ivl`, `.vl`, `.s2v` (+ `.s2v.v` in g/c*, g/sweep/m); `vl_<cell>.build` and `.s2v.err` were deleted by run4.sh / a/run.sh.

## Verdict sources
- g/c … g/c7 and their copies (post_a/cells/g_c*, r2/diff/impl/g_c*, g/tmp): g/preA.tsv column 2, the grounding PRE-vs-oracles class (ok1/ok2/loud1/loud2/WRONG1/WRONG2/SPLIT/NOORACLE); notes add "listed in post_a/s5_ok.txt:N (S5 vs PRE: wrong->ok, s589/IMPL.md:28)" where it applies.
- g/sweep/m movers (+ copies in g/sweep/c, post_a/sweep, post_a/swmv): g/sw_pre.tsv column 2 (PRE vs oracles); notes add the post_a/sw_pre_post.txt DIFF line.
- a (audit), post_a/cells/k, post_b/k2, g/st, g/st2: main docs (GROUNDING, PLAN, IMPL, LENS_BRIEF, COMMIT_*), class word nearest the cell name (the class before a parenthetical list the cell sits in, else the first class word after the name within its list item). 70 of 72 audit cells have no class word (`unrecorded`; a/audit.tsv carries their values, no class column).
- Lens cells: that round's REPORT.md (r1/r2/r3 diff and sound; post_b/lens and post_c/r2 copies use the matching report): "## Findings" table/line naming the cell, then a VERDICT line, then the nearest class word, then a "## FINDING" header with the cell within 3 lines; fallback to the main docs. Notes say which rule fired.

## Layout
Flat dirs; a cell = `<stem>.sv`, outputs `<stem>.<suffix>`.
| suffix | role | tag |
|---|---|---|
| .sv | sv | - |
| .ivl | oracle_iverilog | iverilog |
| .vl | oracle_verilator | verilator |
| .s2v | oracle_sv2v (sv2v -> iverilog run + first 3 sv2v stderr lines) | sv2v |
| .s2v.v | oracle_sv2v_translation | sv2v |
| vita suffixes (above) | vita | per table |
| g/st2/c.sv, g/st/ub.sv, post_a/st2/c.sv | aux (second compilation unit) | - |
| g/st/vlo.err, g/st2/vlo.err | oracle_verilator (two-CU verilator stderr) | verilator |
| .vu .velab .vvp wl/ lib.toml | excluded | |
Example: s589/g/c/a_ret_rt_fn.{sv,pre,ivl,vl,s2v,s2v.v,p1,p1off,p2,p2off,p3,p4,p4off,off_body,…,off_typing,route,stg,stg2}.
Sets: g/c (202 generated), g/c2 (52), g/c3 (3), g/c4 (88), g/c5 (13), g/c6 (28), g/c7 (16) = grounding; a (72 audit); post_a/cells/k (10 killer cells), post_b/k2 (4); g/st, g/st2 (two-CU staged experiments); g/sweep (5756 deduped `.sv` re-collected from s580–s588 and s589's own g/c*; map.tsv maps wNNNNN -> original path; c/ = vita-only runs, m/ = 46 movers with oracles); lens cells r1/diff/c/b1-b3, r1/sound/c (+b3, b4, b5, k, st), r2/diff/c/n, r2/sound/c (+two, k), r3/diff/c/k, r3/sound/c (+k); copies: post_a/cells/*, r2/diff/impl/*, post_b/lens/*, post_c/r2/*, post_c/two, post_a/sweep, post_a/swmv, post_a/st2, docs_probe, g/tmp.
Kept copy for sweep movers: g/sweep/m (neither m/ nor c/ is the authoring location; m/ holds the oracle runs). Lens r2/r3 reused r1's cells (r2/diff/c/b*, r3/diff/c/b* are aliases); r3/diff/c/b1-b3 also hold `.pc`/`.pcs` for 27 r2 cells whose `.sv` is only in post_c/r2/r2_diff_n (attached to that cell).
Generators kept: g/gen.py, g/gen2.py (g/c factorial). Runners kept: g/run4.sh, g/stage1.sh, g/st/stage.sh, g/sweep/run1.sh, a/run.sh, post_a/runc.sh, post_a/runs.sh, post_a/ora.sh, post_a/st2/run.sh, r1/diff/h.sh, r3/diff/hx.sh.
Verdict tables kept (27): a/audit.tsv, g/{pre,preA,p1,p2,p4A,p25,p26,p27,pre5,pre6,pre7,batt1,sw_pre,sw_p2}.tsv, g/redo.txt, g/stgdiff.txt (empty), g/sweep/map.tsv, g/sweep/movers.txt, post_a/{mv_pre_p4,mv_pre_post,mv_pre_post_v,mv_pre_s5,p4_ok,s5_ok,sw_pre_post,sw_pre_s5}.txt.

## Counts
- Cells (non-alias) 618: g/c 202, g/c2 52, g/c3 3, g/c4 86, g/c5 13, g/c6 28, g/c7 16, a 72, post_a/cells/k 10, post_b/k2 4, g/st 1, g/st2 1, g/sweep/m 32, r1/diff/c/b1 20, b2 16, b3 4, r1/sound/c 16, b3 5, b4 2, b5 2, k 4, r2/diff/c/n 9, r2/sound/c 2, r2/sound/c/two 1, r3/diff/c/k 11, r3/sound/c 6.
- Aliases 1375 (post_a/cells/* 474, r2/diff/impl/* 484, sweep copies g/sweep/c 72 + post_a/sweep 72 + post_a/swmv 22 + g/sweep/m 14 (= g/c, g/c4 cells), lens copies, …); 8496 alias rows not copied.
- Field recovery over 1993 CELLS rows: verdict_class recovered 1447, unrecorded 546 (source same); vita_tags unrecorded 11; oracles: 8 rows doc-quote only (cu_a, cu_b and copies: s589/r3/diff/REPORT.md:15 "(= 3 oracles)"), 171 rows `none-on-this-copy`; binary md5: recovered for every built tag except impl-M1..M10, OFF-LANE, STG2, ROUTE; base commit recovered for PRE and POST-a/b/c (diff md5), probes by diff name.
- CELLS columns oracles_present / vita_tags list the outputs of that copy, including outputs that are MANIFEST `alias` rows (byte-identical to the kept copy). Generated helper files in this work dir (build.py, scan*, stats.txt, noora_cells.txt, run_stats.json; s588 also pc_lib.py / pc_engine.py / pc_stats.py) are not part of the manifest.

## Not recoverable / excluded cells
- `no-oracle-output` 68341 files / 68073681 bytes: g/sweep/c 5684 vita-only cells + outputs (28420 files, 29755703 bytes, incl. the 1.1 MB generated stress cells w057xx.sv) and their post_a/sweep copies (39788 files, 38267179 bytes); 24 VCDs those sweep cells wrote; docs_probe/x1 80 vita outputs of s587/x1 cells (no `.sv` in s589); vita-only cells docs_probe bits_body, bits_body_rt, noshadow, pc25; g/st/ub_ctl.sv; r2/diff/c/n dup_a, dup_b (+copies in r3, post_c/two); r2/sound/c/two a.sv, b.sv (+docs_probe two_a/two_b, post_c/two sa/sb; the one-file twin ab.sv is kept). List in noora_cells.txt.
- g/st2 iverilog result exists only as a quote (s589/GROUNDING.md:48: "iverilog `top.v=8 c.w=40 c.g.P=3`"); its `x.vvp` is excluded.
- Mutant binaries and the probe/OFF-LANE/STG2 invocations are not on disk.

## Size
- Manifest: 22092 files, 5660307 bytes (excluding 8496 alias rows).
- By role: vita 19057 / 4465236; verdict_table 27 / 462684; oracle_verilator 659 / 256255; sv 618 / 199442; oracle_sv2v_translation 432 / 165982; oracle_sv2v 642 / 49911; oracle_iverilog 642 / 44599; runner 11 / 8785; generator 2 / 6803; aux 2 / 610.
- By set (largest): g/c 979896; post_a/cells/g_c 794888; g/c4 412332; post_a/cells/a 409405; post_a/cells/g_c4 334218; g/c2 283404; set-level g/sweep 218108; post_a/cells/g_c2 216901; set-level g 192856; post_a/sweep 130618; r2/diff/impl/g_c 129834.
- 10 largest: g/sweep/map.tsv 217576; g/preA.tsv 35103; g/pre.tsv 29886; g/p4A.tsv 28297; g/p1.tsv 26598; post_a/mv_pre_p4.txt 26178; g/p2.tsv 25130; g/batt1.tsv 22251; a/audit.tsv 11624; post_a/mv_pre_post_v.txt 5800.
- No file > 1 MB; no set > 5 MB. No VCD manifested. Third-party screen: 0 hits.

## Excluded (see EXCLUDED.tsv)
binary 67 / 458726672; no-oracle-output 68341 / 68073681; log 74 / 5357819 (gate/, corpus_main, build/nextest logs, .err/.md5/.out); mutation 176 / 3893714 (*/mut/); doc-draft 5 / 3190887 (g/archive.md, docs_probe/*.md); other 1 / 831899 (g/sweep/list.txt); rust-patch 10 / 550352 (wt*.diff, proto_p1*.diff, *.rs); slice-record-doc 12 / 121083 (GROUNDING, PLAN, IMPL, LENS_BRIEF, COMMIT_SLICE, COMMIT_DOCS, 6 lens REPORT.md: the verdict sources, parent decides); perf-ab 45 / 109938 (post_a/ab, abv: ibex elab A/B); build-object 22 / 77665 (g/st/vlo verilator objects); script 34 / 53858; iverilog-image 3 / 10713; vita-staged-artefact 8 / 6223; duplicate-table 1 / 217576 (post_a/sweep/map.tsv, byte-identical to g/sweep/map.tsv). Accounting: 99387 files walked = 30588 manifest rows + 68799 excluded.
