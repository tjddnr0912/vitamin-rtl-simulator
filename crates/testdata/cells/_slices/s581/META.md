# META s581

## Slice
- Slice: §4.5.581. ROADMAP rows: §2 🆕 T and §2 🆕 S (a) ("# s581 — §2 🆕 T stage 1 …", s581/REPORT.md:1; design commits d4dc9c26 "(§2 🆕 T)", e350ef79 "(§2 🆕 S (a))", s581/msg_T.txt:1, s581/msg_S.txt:1).
- Outcome as recorded: "Outcome: §4.5.581 attempted §2 🆕 T and §2 🆕 S (a)'s constant half; three review rounds; reverted whole (§4.5.571/572 precedent). MAIN tree = e54fa74a plus one held-cell test file `crates/cli/tests/generate_case_and_wildcard_prerequisites.rs` (12 tests, all at today's output, oracle lines in comments; oracles re-run on $SCR/fin/*.sv)." (s581/REPORT.md:123).
- Census status: complete (manifest only, nothing copied). Census helpers: preserve/work/s581/_tools/ (config) + preserve/work/s580/_tools/ (shared engine); not part of the manifest.

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s581/pre/vita | 4ace7617440041d0d16aee5309fded10 | release | e54fa74a | s581/COMMON_LENS.md:29 (md5_at_preservation identical; same md5 as s580/post4) |
| POST-T | s581/postT/vita | 37d2b613f937979ade730b6808175e22 | release | d4dc9c26, branch fix/gencase-label-domain | s581/COMMON_LENS.md:30; s581/REPORT.md:62 (md5_at_preservation identical). The first POST-T freeze f70204a92bf3789eca759db0938b485c is kept as s581/postT/vita.refusal-build (s581/REPORT.md:34, :62); census `.post.txt` files are dated 00:19, the re-freeze time |
| POST | s581/post/vita | 1e41978f78724cc11b691cf6babfe911 | release | e350ef79 | s581/COMMON_LENS.md:31; s581/REPORT.md:79 (md5_at_preservation identical) |
| POST2 | s581/post2/vita | ed69d5ae6fa11e12d4220550f6391b42 | release | 9bd5cd65 | s581/COMMON_R2.md:9; s581/REPORT.md:93 (md5_at_preservation identical) |
| POST3 | s581/post3/vita | 33067d7744033c67c45ad43be49d95da | release | eb9d3b69 (s581/REPORT.md:131) | s581/REPORT.md:97; s581/DELTA_R2.md:4 (md5_at_preservation identical) |
| POST3b | s581/post3b/vita | 6aab206f39960ecfd78a576159ed811c | release | eb9d3b69 + w>64 fallback (s581/REPORT.md:131) | s581/REPORT.md:119; s581/DELTA_R2.md:5 (md5_at_preservation identical) |
| dbg-repo | /Users/seongwookjang/project/git/vitamin-rtl-simulator/target/debug/vita at run time (s581/census/run.py VITA['dbg']) | unrecorded | debug | unrecorded (files dated 2026-10-02 00:14) | unrecorded |
Tags used in MANIFEST: 7. Binary md5 recorded for 6 (all but dbg-repo); base commit recorded for 6 (all but dbg-repo).
Other binaries on disk, no manifested output tagged with them: s581/vita_guard_noarm md5_at_preservation f148d07bd7aec1177e2d8872e52c99db (the guard-kept build, s581/DELTA_R2.md:17); lens_snd/vita_mut_d (mutant, s581/lens_snd/REPORT.md:135 54bc2e52…), lens_snd/vita_snap1 (805086ed…, s581/lens_snd/REPORT.md:131). s581/lens_diff/run.py also names `ref` = s580/post3/vita (76ad4738…, s581/COMMON_LENS.md:32); no `.ref` output exists.
Tag source per suffix: census/run.py and s2/run2.py VITA dicts (`.post` = postT/vita in census, = post/vita in s2; `.s2` = post/vita; `.p2` = post2; `.p3` = s581/post3 — the `.p3.txt` files are dated 03:00, after s581/post3/vita was built); lens_diff/run.py (`.pt` = postT); lens_diff2/run2.py (`.post2` = post2); lens_diff/rc `.pre/.postT/.post.txt` = PRE/POST-T/POST ("252 cells copied to lens_diff/rc, run on PRE/POST-T/POST", s581/lens_diff/REPORT.md:29); lens_snd3 out/`P3b_*` = POST3b by suffix name (that lens compares PRE with POST3b, s581/lens_snd3/REPORT.md:10).

## Oracles
- verilator: "Verilator 5.052 2026-09-05 rev vUNKNOWN-built20260905" (captured in logs, e.g. s581/census/cells/L01.vlb.txt:7).
- iverilog: no `-V` line captured; docs say "iverilog 13.0" (s581/COMMON_LENS.md:40, s581/REPORT.md:15).
- sv2v: "sv2v 0.0.13: `$S580/sv2v/sv2v-macOS/sv2v f.sv > f.v`" (s581/COMMON_LENS.md:41); every runner calls the s580 binary (md5 ca3f9b7138513e09a88be96aa7b5ea19, s580/REPORT.md:12).

## Verdict sources
- census (and its alias sets lens_diff/rc, lens_snd/c252, lens_diff3/census_c): the `class` column of s581/T_census.tsv (stage-1 POST-T census); notes add the s581/S_census_gencase.tsv class where it differs.
- s2 (and lens_diff3/s2c): the `class` column of s581/S2_matrix.tsv.
- lens_diff/p: the flags on each cell's `== <id>` line in s581/lens_diff/{r,rab,rcd,si,spg,sq}.out (`<<VITA-MOVES`, `<<POST!=ORACLE`); a line with no flag is `unrecorded` with the line in notes; a few hand cells from s581/lens_diff/REPORT.md.
- lens_diff3/v*: first token of s581/lens_diff3/counts.txt (`split` / `moved_ne_oracle`) where listed; values (exp/pre/post3b/iverilog/sv2v) are in s581/lens_diff3/r3{,b,c,d}.tsv, cited as doc-quote or in notes.
- hand-mapped quotes: lens_snd/probes (s581/lens_snd/REPORT.md), lens_snd2/probes (s581/lens_snd2/REPORT.md), lens_snd3/p (s581/lens_snd3/REPORT.md), lens_diff2/p (s581/lens_diff2/REPORT.md), fin/me/r1v/blk/probes2 (s581/REPORT.md FINAL section, s581/DELTA_R1.md, s581/DELTA_R2.md).
- verdict_table rows in MANIFEST: T_census.tsv, S_census_gencase.tsv, S2_matrix.tsv, S2_p3.txt, classify*.out, cmp4_post*.txt, cmp_post_post2.txt, lens_pp2.txt, lens_pp3*.txt, d3b.txt, lens_diff/*.out, lens_diff/cmp4_mine.txt, lens_diff2/*.out, lens_diff3/r3*.tsv, r3*_summary.txt, counts.txt, ladder/ladder.txt, lens_snd3/p/out/kb.txt, ocheck/post*.txt (the last two quote s580 cells too).

## Layout
dst = path relative to s581/. A cell = one .sv file; outputs belong to the .sv whose stem is the longest prefix of the output's stem in the same dir unless an owner rule below says otherwise.
- census/cells (generator census/gen.py, runner census/run.py, cells/meta.json aux). Example census/cells/C1.sv. `.pre.txt` PRE; `.post.txt` POST-T; `.s2.txt` POST; `.p2.txt` POST2; `.p3.txt` POST3; `.dbg.txt` dbg-repo; `.iv.txt` oracle_iverilog (direct); `.sv.txt` oracle_sv2v (sv2v→iverilog); `.sv2v.v` translation; `.vlb.txt` oracle_verilator (build log); `.vl.txt` oracle_verilator (run or the build's %Error lines).
- s2/cells (generator s2/gen2.py, runner s2/run2.py; meta.json, list.json aux). Example s2/cells/AD_C.sv. `.pre/.post/.p2/.p3.txt` PRE/POST/POST2/POST3; `.iv.txt`, `.sv.txt`, `.sv2v.v`, `.vl.txt`; `.vlb` = verilator build log (text, checked per file).
- lens_diff/p (runner lens_diff/run.py; generators gen_r.py, gen_s.py, gen_sp.py). Example lens_diff/p/R00.sv. `.pre/.pt/.post.txt` PRE/POST-T/POST, `.iv/.sv/.vl/.vlb.txt`, `.sv2v.v`, `<cell>.G` aux (top-parameter overrides).
- lens_diff2/p (runner lens_diff2/run2.py): as lens_diff/p plus `.post2.txt` POST2; `r5a.svh`, `r5b.svh` include files (set-level aux).
- lens_diff3/v, v2, v3, v4 (generators gen_run{,2,3,4}.py): `.sv`, `.s2v.v` translation, `.sv.i.err` oracle_iverilog (compile stderr, failures only), `.s2v.v.s.err` oracle_sv2v; lens_diff3/vl/<cell>.log oracle_verilator for the v*/<cell>.sv of that name. Values per variant in r3*.tsv.
- lens_snd/probes (runner lens_snd/probes/r.sh; run values went to stdout only): `.s.v` translation, `.s.err` sv2v stderr (all empty), `.vlb.txt` verilator build log. Values quoted in s581/lens_snd/REPORT.md.
- lens_snd2/probes/q1..q4: `.s2v.v`, `.s2v.err`, `vl_<cell>.log` (verilator build log); q3 inc_a.svh/inc_b.svh set-level aux.
- lens_snd3/p: b.sv, f.sv, f2.sv, g.sv, k.sv hold `ifdef` sub-cells; p/out/{iv,sv,PRE,P3b}_<sub>.txt = iverilog / sv2v→iverilog / PRE / POST3b per sub-cell, attached to the file that p/run/*/<sub>/ copies; p/run/<tool>/<sub>/<file>.sv are byte copies (aliases) with `o.v` translations; p/kb/kb.sv (+9 copies in p/kb/<bin>-K*/) with values in p/out/kb.txt.
- me, r1v, blk, ladder, fin, probes2, dump2, dump3: `.sv`, `<cell>.v` (or `.v2`) translation, `.vlb` / `vl_<cell>.txt` / blk/vl.log verilator logs; ladder/ladder.txt per-cell value table.
- Alias sets (byte copies of census/s2 cells): lens_diff/rc (with its own PRE/POST-T/POST runs, not byte-identical to census outputs so manifested), lens_snd/c252 (no outputs), lens_diff3/census_c and lens_diff3/s2c (only `.post3b.txt` differs; every other output is an alias row).

## Not recoverable
- fin/ (the 12 held tests' designs): only verilator build logs (`.vlb`) and sv2v translations on disk; the oracle run lines live in the repo test file named at s581/REPORT.md:123, not in the slice dir.
- lens_snd/probes: r.sh printed every tool's values to stdout only; kept files are build logs / empty sv2v stderr / translations. No oracle file and no per-file doc quote → excluded: C1-C12, E1-E12 (summarised "no value->loud, no wrong value", s581/lens_snd/REPORT.md:41), G2, G10, MB1.
- lens_snd2/probes q3 C2, G2, I1, L3, U1 (U1 vita-only line at s581/lens_snd2/REPORT.md:60) → excluded.
- me/ m1, m1q, m2, ov1, ov2, r3a, r3b; probes/ c1-c3, t1-t4, lpa (PRE baseline only, s581/REPORT.md:4-5); probes2/ kw, n09me, na11u (named without values at s581/REPORT.md:64), s, s13p, s2, s47, w06p; r2v/ I2, Q6, res, res_noinside, sx; dump/, most of dump2/ and dump3/ (translations only); lens_diff/p R_eval, S02c, gchk → excluded as no-oracle-output.
- oc3w/ and ocheck/w/: index-numbered sv2v translations of cells held elsewhere (incl. s580 cells), results kept only as tables (lens_pp3_oracle.txt, ocheck/post*.txt, manifested as verdict_table).
- gcx_all.txt: TEMP instrument dump (s581/REPORT.md:52), excluded.
- ex/ (examples stdout+VCD PRE vs POST) and velab/ (11 corpus + 4 examples IR identity, third-party corpus outputs): excluded.

## Counts
| set | cells (incl. aliases) | aliases | verdict_class recorded | vita_tags recorded | oracle doc-quote |
|---|---|---|---|---|---|
| census | 252 | 0 | 252 | 252 | 0 |
| s2 | 174 | 0 | 174 | 174 | 0 |
| lens_diff/p | 318 | 1 | 224 | 318 | 0 |
| lens_diff2/p | 53 | 0 | 6 | 53 | 0 |
| lens_diff3/v | 361 | 0 | 19 | 0 | 287 |
| lens_diff3/v2 | 59 | 0 | 25 | 0 | 29 |
| lens_diff3/v3 | 44 | 0 | 0 | 0 | 40 |
| lens_diff3/v4 | 22 | 0 | 10 | 0 | 0 |
| lens_snd/probes | 62 | 0 | 32 | 0 | 17 |
| lens_snd2/probes | 23 | 0 | 8 | 0 | 7 |
| lens_snd3/p | 5 | 0 | 4 | 5 | 0 |
| lens_snd3/p/kb | 10 | 9 | 0 | 0 | 1 |
| lens_snd3/p/run | 200 | 200 | 0 | 0 | 0 |
| me | 3 | 0 | 3 | 0 | 3 |
| r1v | 5 | 0 | 5 | 0 | 0 |
| blk | 3 | 1 | 3 | 0 | 1 |
| ladder | 18 | 0 | 0 | 0 | 18 |
| fin | 13 | 2 | 9 | 0 | 0 |
| probes2 | 2 | 0 | 1 | 0 | 2 |
| dump2 | 3 | 0 | 0 | 0 | 0 |
| dump3 | 2 | 2 | 0 | 0 | 0 |
| lens_diff/rc | 252 | 252 | 252 | 252 | 0 |
| lens_snd/c252 | 252 | 252 | 252 | 0 | 0 |
| lens_diff3/census_c | 252 | 252 | 252 | 252 | 0 |
| lens_diff3/s2c | 174 | 174 | 174 | 174 | 0 |
| total | 2562 | 1145 | 1705 | 1480 | 405 |

Per field, recovered vs `unrecorded` over 2562 CELLS rows: verdict_class 1705/857; verdict_source 1705/857; vita_tags 1480/1082; oracles_present 2562/0 (of which doc-quote only: 7; `via-alias_of` = alias row whose oracle files sit with alias_of).
Binary md5 and base commit: see the Binaries table (rows with `unrecorded` are counted there).

## Size (manifest, role=alias rows excluded)
- total: 6681972 bytes in 10779 files (MANIFEST rows incl. alias: 16210).
- by role: oracle_verilator 3228465 B/1642 f, vita 1071131 B/4603 f, sv 966665 B/1417 f, verdict_table 653144 B/40 f, oracle_sv2v_translation 483639 B/1324 f, oracle_iverilog 108457 B/918 f, generator 90774 B/9 f, aux 37651 B/13 f, oracle_sv2v 27508 B/807 f, runner 14538 B/6 f
- by set: lens_diff/p 2474392 B/2560 f, census 1244591 B/3024 f, (set-level) 795994 B/63 f, s2 791022 B/1684 f, lens_diff2/p 356117 B/477 f, lens_snd/probes 224314 B/231 f, lens_diff3/v 203172 B/800 f, lens_diff/rc 145334 B/756 f, lens_snd2/probes 80036 B/85 f, lens_diff3/v2 68017 B/153 f, lens_diff3/census_c 48762 B/252 f, fin 48590 B/35 f, lens_diff3/v4 38363 B/73 f, lens_snd3/p 33936 B/200 f, r1v 24051 B/15 f, lens_snd3/p/run 22650 B/50 f, lens_diff3/s2c 22296 B/174 f, ladder 22104 B/36 f, lens_diff3/v3 19668 B/92 f, dump2 12847 B/9 f, blk 2324 B/4 f, probes2 2067 B/2 f, me 1078 B/3 f, lens_snd3/p/kb 247 B/1 f
- 10 largest: lens_diff/rab.out 95963; lens_diff/rcd.out 95480; cmp4_post3.txt 84429; lens_diff/cmp4_mine.txt 36913; cmp4_post.txt 36913; census/gen.py 29655; ocheck/post.txt 28685; T_census.tsv 28468; S_census_gencase.tsv 28418; lens_pp3.txt 21150
- files > 1 MB: none; sets > 5 MB: none.
- VCD rows (role vcd; repo .gitignore has `*.vcd`, parent decides): 0

## Excluded (EXCLUDED.tsv), bytes by category
repo-copy 597076026 B/22973 f, build-dir 463384241 B/15763 f, binary 153227800 B/10 f, third-party-corpus-output 63762337 B/292 f, lens-harness 11117936 B/40 f, gate-log 7001197 B/140 f, vvp-image 5559550 B/2968 f, rust-source-or-patch 261564 B/17 f, harness-translation 255091 B/287 f, instrument-dump 234452 B/1 f, slice-doc-verdict-source 139083 B/20 f, no-oracle-output 106200 B/190 f, script 78747 B/26 f, doc-draft 40447 B/6 f, example-run 11544 B/16 f, scratch-output 2643 B/8 f, orphan-translation 1525 B/2 f, run-log 30 B/10 f, empty-marker 0 B/4 f
