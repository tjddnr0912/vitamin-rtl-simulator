# s584 oracle-cell census (phase 1: manifest only)

## Slice
- Slice: §4.5.584. ROADMAP row: §2 🆕 Z ("§4.5.584 — ROADMAP §5.2 row 1, §2 🆕 Z (the external report, re-diagnosed by §4.5.583)." s584/COMMIT_SLICE.txt:3-4).
- Outcome as recorded: slice commit "sim-engine, elaborate, parser: an `always_comb` / `always_latch` time-0 pass waits for its inputs (§2 🆕 Z)" (s584/COMMIT_SLICE.txt:1; repo 75af8f1c) and docs commit "docs: record §4.5.584 — §2 🆕 Z closed, its residue is §2 🆕 AA; `unique-if-chain` is §5.2 row 1" (s584/COMMIT_DOCS.txt:1; repo 75f255d4).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s584/pre/vita (= s584/r1/vita_pre) | 2492e1c5c52c7081a6c0997fe194af40 | release | built from bf7f3e0d; HEAD 63b9dbb9 (code-identical per PLAN.md:4) | s584/GROUNDING.md:3 |
| A | s584/r1/vita_a (= post/vita_a) | caef001a26e755f24510e4ec9f89e443 | release (r1 frozen) | slice tree A = R7pss (r1/diff/REPORT.md:1); branch unrecorded | s584/r1/diff/REPORT.md:3 |
| AB | s584/r1/vita_ab (= post/vita = post/vita_ab) | 5d8d3960ad827868fe41afdfcabc186b | release | A + patch B (s584/slice_b.patch, later dropped: B_DROPPED.md) | s584/r1/diff/REPORT.md:3; s584/B_DROPPED.md:3 |
| POST2 | s584/post2/vita (= r2/vita_post2) | 28f38a3e306640b2c59907c1ab0c5825 | release | slice tree round 2 | s584/r1/diff/REPORT.md:40 |
| POST3 | s584/post3/vita (= r3/vita_post3) | 469298289bfacab02e755ec2d18fa855 | release | slice tree round 3 | s584/r1/diff/REPORT.md:68 |
| POST4 | s584/post4/vita | 7868871ac14c9a58eaa6f51b6f980ca5 | release (7338016 B, same size as POST3) | final tree (r4run/run.py BINS post3, post4) | md5_at_preservation |
| PROTO | s584/tgt-proto/debug/vita (build dir; not manifested) | 2768dfd2810664ab9442143655cd9b65 | debug | worktree S/proto, "branch proto/s584 off 63b9dbb9, never committed" (GROUNDING2.md:50) | md5_at_preservation; the file's mtime (01:01) is later than r2/ (00:31) and r3/ (00:20), so it may not be the binary that wrote them |
| PROTO-rel | s584/proto_rel_vita | f827e47ff036a68a724a5b401d8e6af8 | release | prototype | s584/GROUNDING2.md:138 (used for corpus runs only; corpus excluded) |
| unrecorded(<dir>) | s3b, s3dbg, s3dbg2, s3rel, s3relb, staged, staged_b, staged2, staged3, r1/diff/r2stg, c12/post | unrecorded | unrecorded | unrecorded | unrecorded (prun.py PVITA override / staged runs; no doc names the binary) |
- Tag forms: `PROTO/<RULE>` = prun.py run of the prototype with VITA_T0RULE=<RULE> (PRE = rule unset) — r2/<RULE>/, r3/<RULE>/, c4/<RULE>/, c12/proto/<RULE>/, c14/proto/<RULE>/ (r2/POSTA and r2/POSTB keep their dir names as rule labels); `<BIN>/<be>` and `<BIN>/obs-<be>` = per-backend run / observability files (r1/diff/r2obs, w_*/v_<bin>_<be>.vcd); `unrecorded(<dir>)/<lane>` = binary not recorded.
- Combined files (`<cell>.out`, `.out2`, `.out3`): tag lists the sections present, e.g. `iverilog+verilator+PRE+A+AB` (r1/diff/run3.py), `+A+POST2` (run4.py), `+POST2+POST3` (run5.py; .out2/.out3 copy the iv/vl text from the r1 .out per run4.py:2 / run5.py:2).
- Other binaries present, not used by any manifested output: exp_bin/vita_e1 d24e533a35e2c19ceaa5fcbea7e942f7, vita_e3 79c129493e8e6df15e86b19a36c4fb19, vita_e4 bcaabc1efb542c6da4f979e82605b619 (perf experiments), tgt-post/{debug,release}/vita (build dir).

## Oracles
- Captured: "Oracles: Icarus Verilog version 13.0 (stable) (v13_0) `-g2012`; Verilator 5.052 2026-09-05 `--binary --timing --assert`." (s584/S0.md:3); "Oracles: iverilog 13.0 -g2012; verilator 5.052 --binary --timing --assert (no oracle for an x or a run count: 2-state, re-evaluates to settle)" (s584/COMMIT_SLICE.txt:4-6). No sv2v run in this slice's cells (no sv2v section or sv2v.v found).

## Verdict sources
- Header-aware tables (column `cell` + column `class` or `verdict`; brace/glob/short-id keys expanded, e.g. `b_uc_v_d{1,2,3}_{tbfirst,leaffirst}`, `a_ui_*`, `f4 / f5`): s584/GROUNDING.md, GROUNDING2.md, S0.md, PLAN.md, r1/diff/REPORT.md (class column + `[sev: …]` from the sev column, both verbatim), r1/sound/REPORT.md (verdict column).
- r1/diff and r1/sound prose fallback (first line naming only this cell; first verdict word: real gap, NON-BLOCKING, BLOCKING, no-oracle, GAP, AGREE, SPLIT, agree, pre-existing, descent, fixed, ...; else the `#` heading's word).
- Verdict tables manifested (set-level, not used for verdict_class): c3/cmp.txt, split_all.txt, split_r.txt, r2run/diff_vs_r1a.txt, r3run/diff_vs_post2.txt, r3run/diff_vs_pre.txt, r4run/diff_vs_post2.txt.
- Alias cells take the kept cell's verdict / oracles (notes say so).

## Layout
- c1, c1t, c2, c4..c14 (grounding, run.py): cell `<set>/<name>.sv`, output `<set>/<name>.out` (role `combined`: `=== iverilog` (-g2012 + vvp), `=== verilator` (--binary --timing --assert, +verilator+error+limit+1000), `=== vita (native=interp=vm)` = PRE). `<set>/w_<name>/` holds a.vvp / obj_vl only (excluded). Example: c1/a_uc_combfirst.sv -> c1/a_uc_combfirst.out.
- c3/orig = copies of grounding cells (aliases; their .out byte-identical to the kept cell's become alias rows); c3/sp/<RULE>/, c4/sp/<RULE>/, c6/sp/ = hand-spelled rule transforms (c3/transform.py) run on PRE: a transform identical to its source is an alias; a changed one is vita-only (excluded).
- r1/diff/{b1,b2,b3,b4,res,r2t,r2u,r3c}/<name>.sv -> `<name>.out` (run3.py: iv, vl, PRE/A/AB), `.out2` (run4.py: PRE/A/POST2), `.out3` (run5.py: PRE/POST2/POST3); w_<name>/v_<bin>_<be>.vcd and iv.vcd (iverilog VCD, tag iverilog); r1/diff/r2obs/<name>.<bin>.<be>/ run.json+results.jsonl; r1/diff/r2stg staged logs.
- r1/sound/cells/<name>.sv -> r1/sound/w/<name>/{iv.out, iv_c.txt (oracle_iverilog run / compile), vl.out, vl_c.txt (oracle_verilator run / compile)} written by r1/sound/r.sh; vita runs were printed to the console only (vita_tags `unrecorded` for these cells unless a re-score dir holds them).
- Re-score dirs (vita only, attached to the cell they name): r2/<RULE>/<stem>.out and r3/<RULE>/<stem>.out (cells listed in r2/cells.txt, r3/cells.txt -> c3/orig/... paths), r2run|r3run|r4run/<bin>/<set>__<stem>.out (cells.tsv), s3*/PRE/<stem>.out (resolved by stem, c3/orig first), c4/<RULE>/, c12/proto/<RULE>/, c12/post/PRE/, c14/proto/<RULE>/.
- staged, staged_b, staged2, staged3: copies of cells (aliases) with `<name>.one.<be>.out`, `.vrun.<be>.out`, `.vcmp.log`, `.velab.log` vita outputs.

## Counts
- Cells: 533 (aliases 224; oracles_present=doc-quote 0)
- Per set: c1 34; c10 3; c11 1; c12 7; c13 1; c14 4; c1t 11; c2 55; c3/orig 121; c3/sp 89; c4 3; c4/sp/R1 3; c4/sp/R2 1; c4/sp/R2r 1; c4/sp/R3 1; c4/sp/R3f 1; c4/sp/R4 1; c4/sp/R4f 1; c4/sp/R4t 1; c5 3; c6 26; c6/sp 3; c7 14; c8 11; c9 9; obsudp 1; r1/diff/b1 18; r1/diff/b2 16; r1/diff/b3 4; r1/diff/b4 2; r1/diff/r2t 14; r1/diff/r2u 9; r1/diff/r3c 10; r1/diff/res 4; r1/sound/cells 33; staged 3; staged2 5; staged3 4; staged_b 4; tmp1 1
- Unrecorded per field: oracles_present 0/533; vita_tags 8/533; verdict_class 155/533; verdict_source 155/533
- binary md5: PRE/A/AB/POST2/POST3/PROTO-rel recorded in slice docs; POST4 and PROTO md5_at_preservation; s3*/staged*/c12-post unrecorded. base commit: PRE recorded (bf7f3e0d / HEAD 63b9dbb9); prototype branch recorded (proto/s584 off 63b9dbb9); POST* branch unrecorded.

## Not recoverable / not cells
- c3/sp, c4/sp, c6/sp changed transforms (907 SVs with their .out): vita-only PRE spellings (c3/transform.py: "Hand-spell candidate t0 rules in SV, for the PRE binary"). Excluded as no-oracle-output; parent may decide otherwise.
- c5/forvar_init_only.sv, c5/forvar_star.sv; r1/sound/cells p_comb_* (perf-size cells, "timed separately", r2run/run.py:11) and others without w/<name>/ oracle files: vita-only.
- perf/ (timing cells and ibex A/B timing), jitlane/, suite/, gate/, mut/, corpus/, ibex/: excluded (perf/ibex* and corpus/ibex = third-party corpus).
- r1/sound vita outputs: never written to files (r.sh prints them).

## Size offender (parent decides)
- s584/r1/sound/w/s3_comb_wait/vl.out = 624410641 B (wc -c), raw verilator capture of cell r1/sound/cells/s3_comb_wait.sv. Inspected with head -c / tail -c only: it starts "vl compile rc=0 / C t=0 a=0 n=1 / D t=7 n=1 / C t=7 a=1 n=2 ..." and ends mid-line at "C t=7 a=1 n=1701" (no closing rc line; r.sh runs Vtop under `perl -e 'alarm 20'`). md5 column = `not-computed-size-offender` (the SPEC forbids reading it whole). Manifested with its true size; it alone makes set r1/sound/cells exceed 5 MB.

## Size
- Manifest total (excluding role=alias): 8088 files, 626658141 bytes (597.63 MiB); alias rows: 1156
- By role: oracle_verilator 66 files / 624554330 B; vita 7094 files / 1539977 B; combined 519 files / 349293 B; sv 309 files / 100429 B; verdict_table 7 files / 60378 B; runner 9 files / 21539 B; oracle_iverilog 66 files / 21437 B; vcd 16 files / 5718 B; generator 1 files / 5028 B; aux 1 files / 12 B
- By set:
  - r1/sound/cells: 366 files / 624610003 B  ** SET > 5 MB **
  - c3/orig: 3226 files / 803991 B
  - c2: 550 files / 188028 B
  - c6: 910 files / 155577 B
  - c1: 340 files / 121735 B
  - c9: 315 files / 85907 B
  - r1/diff/b2: 206 files / 84627 B
  - r1/diff/b1: 233 files / 72831 B
  - c7: 490 files / 66804 B
  - r1/diff/r2u: 99 files / 64360 B
  - set-level (verdict_table): 7 files / 60378 B
  - c8: 385 files / 54568 B
  - r1/diff/r2t: 130 files / 43983 B
  - c1t: 110 files / 40378 B
  - r1/diff/r3c: 30 files / 27895 B
  - c12: 133 files / 27646 B
  - set-level (runner): 9 files / 21539 B
  - c3/sp: 89 files / 21156 B
  - c10: 90 files / 20116 B
  - r1/diff/res: 21 files / 18211 B
  - c4: 105 files / 12356 B
  - r1/diff/b3: 48 files / 10845 B
  - c14: 68 files / 10088 B
  - obsudp: 4 files / 8824 B
  - r1/diff/b4: 24 files / 5405 B
  - set-level (generator): 1 files / 5028 B
  - staged: 24 files / 4782 B
  - c11: 21 files / 3194 B
  - c5: 6 files / 2518 B
  - c13: 16 files / 2330 B
  - staged2: 10 files / 775 B
  - c6/sp: 3 files / 627 B
  - staged3: 8 files / 620 B
  - staged_b: 8 files / 620 B
  - c4/sp/R3: 1 files / 132 B
  - c4/sp/R4: 1 files / 132 B
  - c4/sp/R4f: 1 files / 132 B
- 10 largest manifested files:
  - r1/sound/w/s3_comb_wait/vl.out (oracle_verilator) 624410641 B  ** FILE > 1 MB **
  - c3/cmp.txt (verdict_table) 31181 B
  - split_all.txt (verdict_table) 18189 B
  - r1/diff/r2obs/ud8_udp_comb_mutual.post2.native/run.json (vita) 6235 B
  - r1/diff/r2obs/ud8_udp_comb_mutual.a.native/run.json (vita) 5952 B
  - r1/diff/r2obs/ud8_udp_comb_mutual.pre.native/run.json (vita) 5952 B
  - r1/diff/r2obs/ud8_udp_comb_mutual.post2.vm/run.json (vita) 5881 B
  - r1/sound/w/s2_comb_direct_delay/vl_c.txt (oracle_verilator) 5833 B
  - r1/sound/w/s1_comb_task_delay/vl_c.txt (oracle_verilator) 5827 B
  - r1/sound/w/s3_comb_wait/vl_c.txt (oracle_verilator) 5813 B
- Files > 1 MB: 1 -> r1/sound/w/s3_comb_wait/vl.out
- Sets > 5 MB: 1 -> r1/sound/cells
- VCD rows (role vcd; repo .gitignore has *.vcd, parent decides): 16 files / 5718 B

## Third-party screen
- Manifested .sv/.v module names vs $S/preserve/thirdparty_modnames.txt: no match other than generic names; no upstream text markers. third-party-suspect: none. Excluded corpus: ibex/, ibex_cmd.txt, corpus/, perf/ibex_*.

## Parent addendum
- r1/sound/w/s3_comb_wait/vl.out: md5_at_preservation 75427aa9b52e2f2563a792f9836b49ab (computed by the parent with `md5 -q`, 624410641 bytes, mtime 2026-10-03 02:22:34); MANIFEST md5 column updated from `not-computed-size-offender`.
