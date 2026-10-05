# META s580

## Slice
- Slice: §4.5.580. ROADMAP row: §2 🆕 S (commit subjects cf2f9ffc / 789053ba / eab81ef2 / 3711e0e5 end "(§2 🆕 S …)"; record commit e54fa74a "docs: record §4.5.580 — `inside` compares an x/z element with `==?` at run time; §2 🆕 S keeps two halves, §2 🆕 T queued"). The slice REPORT itself does not name the row tag.
- Outcome as recorded: "Round 3 found two BLOCKING findings on the constant axis (lens_snd3 R3-A, lens_diff3 F1/F2) with the round budget spent (ER §3.6): ship the run-time half, constant domain back to PRE semantics." (s580/REPORT.md:547); "Constant contexts compare an `inside` element with `==` exactly as before the slice." (s580/REPORT.md:562).
- Census status: complete (manifest only, nothing copied). Helper scripts used for this census: preserve/work/s580/_tools/ (not part of the manifest).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s580/pre/vita | 9d37b3cdbd7f325fe273241dfbb3f465 | release | 00c3d76d963f28e4ecc403bd5eba0807abcdbb40, branch fix/inside-wildcard | s580/REPORT.md:7 (md5_at_preservation identical) |
| POST | s580/post/vita | 65a8ab183a63664b3b26ac4607967a2c | release | e5147442 (round-1 tree; s580/REPORT.md:389) | s580/REPORT.md:343 (md5_at_preservation identical) |
| POST-interp, POST-vm | s580/post/vita `--backend interp/vm` (lens_diff be_P*_<backend>.txt; backend read from the file name) | 65a8ab183a63664b3b26ac4607967a2c | release | e5147442 | s580/lens_diff/REPORT.md:4,49 |
| POST2 | s580/post2/vita | 09ab4bd248b02f4675f532e2d342282e | release | cf5076ef (s580/REPORT.md:490) | s580/REPORT.md:441 (md5_at_preservation identical) |
| POST2-interp, POST2-vm | s580/post2/vita `--backend interp/vm` (lens_snd2/r.sh) | 09ab4bd248b02f4675f532e2d342282e | release | cf5076ef | s580/REPORT.md:441; runner s580/lens_snd2/r.sh |
| POST3 (+ POST3-interp, POST3-vm) | s580/post3/vita (lens_snd3/h3.sh `--backend native/interp/vm`) | 76ad4738a0ec5a26cc7718c675467a9e | release | e6c9cc8d (s580/REPORT.md:546) | s580/REPORT.md:509 (md5_at_preservation identical) |
| POST4 | s580/post4/vita | 4ace7617440041d0d16aee5309fded10 | release | round-4 tree on e6c9cc8d (git log: 3711e0e5 "… (§2 🆕 S, round 4)") | s580/REPORT.md:608 (md5_at_preservation identical); no manifested output carries this tag |
| dbg-e5147442 | s580/vita_debug_post.bak | e429b343c801e1138fba2d05a9c68d28 | debug | e5147442 | s580/REPORT.md:430; s580/vita_debug.md5; s580/lens_snd/REPORT.md:60 |
| post-dbg-r1 | unrecorded (debug build of the round-1 working tree) | unrecorded | debug | round-1 working tree on 00c3d76d | s580/REPORT.md:77 ("POST = debug build of the working tree at the time of the run") |
| postrel | unrecorded (suffix `.postrel.txt`) | unrecorded | release (per REPORT wording) | unrecorded | s580/REPORT.md:345 ("Census re-run on POST release"); suffix-to-binary link not written down |
| post2dbg | unrecorded (suffix `.post2dbg.txt`, files dated 2026-10-01 17:58) | unrecorded | unrecorded | unrecorded | unrecorded |
| r2dbg | unrecorded (r2/lens `*.dbg*.txt`, written by r2/rerun_lens.py `<new-vita> dbg`) | unrecorded | debug | round-2 tree | s580/REPORT.md:442 ("byte-identical to the debug build"); on disk s580/r2/vita_dbg_new.bak md5_at_preservation ce162b9da5071558e4bfa9c7ac8e91fc (= s580/r2/vita_dbg_new.md5), link to these files not recorded |
| r26-new, r26-dbg2 | unrecorded (r2/r26/run26.sh takes NEW=$1; `.dbg2.txt` producer not recorded) | unrecorded | unrecorded | unrecorded | unrecorded |
| fx-out, fx-preout, fx-relout, fx-err | unrecorded (fx `*.out/*.preout/*.relout`, F14.err) | unrecorded | unrecorded | unrecorded | unrecorded |
| fx2-new, fx2-post_dbg | unrecorded (r2/fx2 `*.new`, H06.post_dbg) | unrecorded | unrecorded | round-2 tree | s580/REPORT.md:429 ("PRE / e5147442 / new outputs: r2/fx2/H0*.{sv,pre,post,new}") |
| r4pre, r4post3, r4new | unrecorded (fx, r2/fx2 `*.r4*`; no producer script on disk) | unrecorded | unrecorded | unrecorded | unrecorded |
| obs-unrecorded | unrecorded (census/obs/C*/run.json, results.jsonl from `--obs-dir`) | unrecorded | unrecorded | unrecorded | s580/REPORT.md:33 names the run.json route read |
| lens-s8, lens-st | unrecorded (vcmp/velab/vrun text of lens_snd/s8 and lens_diff/st) | unrecorded | unrecorded | PRE/POST per s580/lens_snd/REPORT.md:62, s580/lens_diff/REPORT.md:50 | per-file binary not recorded |
Tags used in MANIFEST: 28 (lens-s8 and POST4 rows above are not used). Binary md5 recorded for 11 (PRE, POST, POST-interp, POST-vm, POST2, POST2-interp, POST2-vm, POST3, POST3-interp, POST3-vm, dbg-e5147442), `unrecorded` for 17. Base commit recorded as a commit id for the same 11; post-dbg-r1, r2dbg, fx2-new name only a working tree; 14 `unrecorded`.
Other debug binaries on disk, no manifested output tagged with them: s580/r3/vita_dbg_new.bak 2085bd87ec7d778b3a55af45bbe7d255, s580/r4/vita_dbg_new.bak 2cbd77b8c7cb264ba8f7dd9d83fce8e1 (md5_at_preservation = the .md5 files beside them).

## Oracles
- verilator: "Verilator 5.052 2026-09-05 rev vUNKNOWN-built20260905" (captured in logs, e.g. s580/census/c/C01.vlc.txt:8, s580/r2/fx2/H01.vlc:8).
- iverilog: no `-V` line captured in any log; docs say "iverilog 13.0" (s580/REPORT.md:12, :18).
- sv2v: "sv2v v0.0.13 (s580/sv2v/sv2v-macOS/sv2v, md5 ca3f9b7138513e09a88be96aa7b5ea19)" (s580/REPORT.md:12).

## Verdict sources
- census/* cells: the 4-way divergence classification s580/REPORT.md:313-321 (sub-cell ids expanded; a multi-cell file lists each class with its sub-cell ids); notes carry the matrix row(s) s580/REPORT.md:80-308. Cells absent from 313-321 (no divergence) are `unrecorded`.
- lens cells: hand-mapped quotes from s580/lens_snd/REPORT.md, lens_snd2/REPORT.md, lens_snd3/REPORT.md, lens_diff/REPORT.md, lens_diff2/REPORT.md, lens_diff3/REPORT.md (finding headers and CLEAN lines naming the probe file).
- r2/r26 S09A/B, S18A/B: s580/REPORT.md:426. Other r2/r26 cells: `unrecorded`, notes give the R2-6 decline-table row (s580/REPORT.md:407-424).
- notes column also quotes the round-4 bucket of every cell file from s580/r4/cmp3_post4.txt (both / pre / post3 / neither = POST4 equal to PRE and POST3 / PRE only / POST3 only / neither).
- verdict_table rows in MANIFEST: census/table_d.md, census/table_ecz.md, r2/census_*.txt, r2/lens_*.txt, lens_diff3/q4.out, r4/cmp3_*.txt, r4/rtonly_*.txt, r4/why_*.txt.

## Layout
dst = path relative to s580/. A cell = one .sv file (several sets hold many sub-cells per file, e.g. census/A.sv = A01-A37). Outputs belong to the .sv whose stem is the longest prefix of the output's stem in the same dir (define variants append `_<DEF>` or a raw TAG to the stem).
- census, census/c, census/e, census/cq, census/z (runner census/run.sh; generators census/gen*.py). Example: census/c/C01.sv. Suffixes: `.sv` sv; `_2s.sv` aux (2-state twin fed to verilator; its output is written under the base stem); `.pre.txt` vita PRE; `.post.txt` vita post-dbg-r1; `.postrel.txt` vita postrel; `.post2dbg.txt` vita post2dbg; `.iv.txt` oracle_sv2v (sv2v→iverilog run); `.sv2v.err` oracle_sv2v (sv2v stderr + rc); `.sv2v.v` oracle_sv2v_translation; `.ivd.txt` oracle_iverilog (iverilog direct on `==?` twins); `.vlc.txt` oracle_verilator (build); `.vl.txt` oracle_verilator (run); census/obs/C*/run.json,results.jsonl vita obs-unrecorded for census/c/C*.sv.
- probes: probes/repro_inside_op.sv (`.sv2v.v` translation, obj_r.err verilator stderr (empty), `.post2dbg.txt`, `.postrel.txt`; oracle values quoted s580/REPORT.md:11-12).
- fx (test fixtures; fx/expect.json aux): `.pre.txt` PRE, `.post.txt` post-dbg-r1, `.postrel.txt`, `.post2dbg.txt`, `.out` fx-out, `.preout` fx-preout, `.relout` fx-relout, `.r4pre/.r4post3/.r4new`, `.iv.txt` oracle_sv2v, `.vl.txt` oracle_verilator, `obj_<cell>.err` oracle_verilator (build stderr), `<cell>.v` oracle_sv2v_translation. Example fx/F01.sv.
- lens_snd/probes (runner lens_snd/run.sh): `.pre.txt` PRE, `.post.txt` POST, `.dbg.txt` dbg-e5147442, `.ivd.txt` oracle_iverilog (direct, -DNO_INSIDE), `.iv.txt` oracle_sv2v, `.sv2v.v/.sv2v.err`, `.vlb.txt`/`.vl.txt` oracle_verilator. Example lens_snd/probes/P1.sv.
- lens_snd2/probes (runner lens_snd2/r.sh): as lens_snd plus `.post2.txt` POST2, `.p2interp.txt`/`.p2vm.txt` POST2 backends; TAG variants e.g. P9_SFMT.ivd.txt.
- lens_snd3/p (runner lens_snd3/h3.sh): `.p3native/.p3interp/.p3vm.txt` POST3 lanes, `.p2.txt` POST2, `.pre.txt` PRE, `.iv.txt` oracle_iverilog (DIRECT here), `.sv.txt` oracle_sv2v, `.sv2v.v`, `.vlb.txt/.vl.txt`. Example lens_snd3/p/p1.sv (variants p3_G1 …).
- lens_diff (runner lens_diff/h.sh): `.pre/.post(.q).txt` PRE/POST (q = -DIV), `.iv.txt` oracle_iverilog (direct -DIV), `.sv.txt` oracle_sv2v, `.sv2v.v/.sv2v.err`, `.vl.txt/.vlc.txt`, `be_P0x_<backend>.txt` POST lanes. Example lens_diff/P01.sv.
- lens_diff2 (runner h2.sh): + `.post2(.q).txt`; `<cell>s.v/.sverr` sv2v; CD1.vlb.txt / CD1s.* belong to r2/CD1.sv. Example lens_diff2/Q1.sv.
- lens_diff3 (runner run3.py): `<cell>_<DEF>.{pre,post,post2,post2q,post3,post3q,iv,sv}.txt`, `.sv2v.v`, `.vlb.txt/.vl.txt`. Example lens_diff3/G1.sv.
- r2: r2/CD1.sv (oracle files in lens_diff2/, values s580/REPORT.md:433); r2/lens/<lens>_<stem>[_DEF].{post,postq,post2,post2q,dbg,dbgq}.txt = vita re-runs of lens cells (runner r2/rerun_lens.py), attached to the lens cell.
- r2/fx2: `.pre` PRE, `.post` POST, `.new` fx2-new, `.post_dbg`, `.r4*`, `.ivc` (sv2v→iverilog compile log, all empty), `.sv2verr` (empty), `.e` (empty), `.vlc` oracle_verilator (build log), `.v` translation; run outputs of iverilog/verilator were not saved.
- r2/r26 (runner run26.sh): `S<nn><pos>.sv` (pos A/B/G/L/O), `.pre/.post/.post2/.new/.dbg2.txt`, `.iv.txt` = sv2v→iverilog when `<cell>.v` exists else iverilog direct, `.sv2v.err`, `.vlc(.txt)/.vl.txt`.
- ident, irtest, lens_diff/st: IR byte-identity designs; only copies of fx/census/lens cells are kept, as aliases.

## Not recoverable
- r2/OR1.sv, r2/OR2.sv: "every cell printed iverilog's own `==?` text" (s580/REPORT.md:399), no oracle output file and no raw quote → excluded (no-oracle-output).
- r2/SL1.sv, SL1q.sv: the string-pin oracle values are quoted at s580/REPORT.md:432 without naming the file; SL1.v translation only → excluded.
- r2/fx2/H02q, H05, H05w: "oracles r2/fx2 (sv2v→iv, verilator, iverilog direct H02q, H05w)" (s580/REPORT.md:429) but no output file → excluded. The other r2/fx2 cells keep only compile logs (`.vlc`, empty `.ivc/.sv2verr/.e`) and translations; run values were not captured.
- fx F06, F06n, F07, F07n, F15 (translation only), F16, F18, Ffill, G2, G2q, L1-L8 (no oracle file); r4/fx H07C, I12, K02, K02q, K07 (K02.e/K07.e empty, translations); census/c/C28b (matrix row s580/REPORT.md:295 has no oracle value); census/e/Q.sv; r2/r26 X1, X2 (vita only); r3/irc (234 per-cell IR designs, vita only); r4/rt (26 run-time twins, vita only); ident I02-I13 and irtest N01 (IR identity); lens_snd/s8/T8.sv.
- Outside this slice dir: s580/r4/cmp3.py also ran $S/f2/w65.sv (LPA cell, s580/REPORT.md:565); f2/ is not under s580/ and is not in this manifest.

## Counts
| set | cells (incl. aliases) | aliases | verdict_class recorded | vita_tags recorded | oracle doc-quote |
|---|---|---|---|---|---|
| census | 10 | 0 | 7 | 10 | 1 |
| census/c | 34 | 0 | 33 | 34 | 0 |
| census/e | 57 | 0 | 44 | 57 | 1 |
| census/cq | 10 | 0 | 0 | 10 | 0 |
| census/z | 7 | 0 | 7 | 7 | 0 |
| probes | 1 | 0 | 0 | 1 | 1 |
| fx | 16 | 0 | 0 | 16 | 3 |
| lens_snd | 11 | 0 | 9 | 11 | 0 |
| lens_diff | 16 | 0 | 14 | 13 | 2 |
| lens_diff/st | 1 | 1 | 0 | 1 | 0 |
| r2 | 1 | 0 | 0 | 0 | 0 |
| r2/fx2 | 7 | 0 | 0 | 7 | 0 |
| r2/r26 | 92 | 0 | 4 | 92 | 0 |
| lens_diff2 | 9 | 0 | 5 | 8 | 0 |
| lens_snd2 | 10 | 0 | 9 | 9 | 0 |
| lens_diff3 | 7 | 0 | 6 | 7 | 0 |
| lens_snd3 | 5 | 0 | 5 | 5 | 0 |
| ident | 12 | 12 | 0 | 1 | 0 |
| irtest | 1 | 1 | 0 | 0 | 0 |
| total | 307 | 14 | 143 | 289 | 8 |

Per field, recovered vs `unrecorded` over 307 CELLS rows: verdict_class 143/164; verdict_source 143/164; vita_tags 289/18; oracles_present 307/0 (of which doc-quote only: 6; `via-alias_of` = alias row whose oracle files sit with alias_of).
Binary md5 and base commit: see the Binaries table (rows with `unrecorded` are counted there).

## Size (manifest, role=alias rows excluded)
- total: 2592587 bytes in 4023 files (MANIFEST rows incl. alias: 4038).
- by role: oracle_verilator 985493 B/453 f, vita 796357 B/2199 f, verdict_table 317658 B/15 f, sv 189580 B/293 f, oracle_sv2v_translation 161246 B/304 f, oracle_iverilog 40395 B/249 f, oracle_sv2v 34312 B/491 f, aux 31966 B/5 f, generator 19785 B/4 f, runner 15795 B/10 f
- by set: r2/r26 496918 B/929 f, (set-level) 376424 B/30 f, lens_diff2 304421 B/160 f, census/e 292139 B/560 f, lens_diff 206193 B/351 f, census/c 202913 B/354 f, lens_diff3 156652 B/540 f, lens_snd3 111129 B/302 f, lens_snd2 107918 B/172 f, lens_snd 96219 B/134 f, census 72019 B/82 f, r2/fx2 62667 B/72 f, fx 48960 B/178 f, census/z 42292 B/77 f, census/cq 6839 B/60 f, r2 4101 B/4 f, probes 3443 B/5 f, lens_diff/st 788 B/7 f, ident 552 B/6 f
- 10 largest: r2/lens_r3_post3.txt 72693; r2/lens_r3_dbg.txt 72693; r4/cmp3_post4.txt 39998; r4/cmp3_dbg.txt 39998; fx/expect.json 23186; r4/why_post3.txt 22021; r4/why_pre.txt 19954; lens_diff2/Q1_w.pre.txt 15051; lens_diff2/Q1_w.post.txt 14832; lens_diff2/Q1_w.preq.txt 14551
- files > 1 MB: none; sets > 5 MB: none.
- VCD rows (role vcd; repo .gitignore has `*.vcd`, parent decides): 0

## Excluded (EXCLUDED.tsv), bytes by category
build-dir 4287255750 B/17960 f, binary 180655136 B/10 f, repo-copy 84393888 B/4236 f, gate-log 7372787 B/93 f, build-artefact 1813927 B/1 f, rust-source-or-patch 1593608 B/24 f, vvp-image 1379570 B/494 f, vita-staged 1296539 B/1613 f, no-oracle-output 171053 B/981 f, slice-doc-verdict-source 123430 B/7 f, test-log 89293 B/7 f, tool 83462 B/4 f, lens-harness 50474 B/16 f, script 18844 B/13 f, build-or-corpus-log 16085 B/28 f, example-aux 13856 B/2 f, example-vcd 4688 B/4 f, scratch-output 103 B/2 f, run-log 4 B/5 f, empty-marker 0 B/1 f
