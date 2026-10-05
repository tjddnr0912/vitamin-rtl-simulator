# META s586 (phase 1 manifest; nothing copied)

Status: COMPLETE. Files: MANIFEST.tsv, CELLS.tsv, EXCLUDED.tsv (this dir). Helpers kept here: inv.txt (+.xdirs)
= file inventory with md5 (excluded dirs not walked), build586.py, rules586.py, size.txt. EXCLUDED.tsv was written
by ../s585/excl.py with rules586.py.

## Slice
- §4.5.586. ROADMAP row: "§4.5.586 — ROADMAP §5.2 row 1, §3.b unique-overlap-note (the external report)"
  (s586/COMMIT_SLICE.txt:3).
- Outcome (slice commit title): "parser, cli: one Info line says `unique` / `unique0` overlaps are not checked
  (§3.b unique-overlap-note)" (s586/COMMIT_SLICE.txt:1). Docs commit: "`unique-overlap-note` closed by VITA-I2021;
  `unique-const-fn` is §5.2 row 1" (s586/COMMIT_DOCS.txt:1).
- Start timestamp: no s586_start file; first cell run g/c01_ucasez 2026-10-03 12:35 (file mtimes).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s586/pre/vita | 8d33349579ca773389265f7a888fd17e | release | fc1fc07e | s586/GROUNDING.md:5 (= md5_at_preservation) |
| PRE-SEP | s586/pre/sep/{vcmp,velab,vrun,vita} | vcmp a6c25fc88ce6ab87472cd82f8a7f404b, velab fed435ff34b58ec96a74b7d23d4d6e25, vrun f607ba50817ad0e1d1a5a91cc461dee6, vita f758e6dce74035669645bd0d0251110c | release, separate-bins | fc1fc07e | md5_at_preservation |
| PROBE | s586/probe_bin/vita_default | 9d6736b579a0bb7e6c27a7e58aeaeda1 | release | probe_src = git archive HEAD (fc1fc07e) + patch | s586/GROUNDING.md:123 (= md5_at_preservation) |
| PROBE-PRODUCT | s586/probe_bin/vita_product | 67f32dbd169f3fa977c9c3a0341dd82f | release, --no-default-features | as PROBE | s586/GROUNDING.md:203 prefix; full = md5_at_preservation |
| POST | s586/post/vita (review round 1) | 9c9c64f700f72ccdf4410659484ff720 | release | wt-s586 = fc1fc07e + slice.patch (md5 a113ece08d43ff96fb839352c352089f now) | s586/r1/diff/REPORT.md:3 (= md5_at_preservation) |
| POST-JIT | s586/post/vita_jit | 470b2c97068c96af87e4f2295aafaa74 | release, --features jit | as POST | s586/r1/diff/REPORT.md:3 |
| POST-PRODUCT | s586/post/vita_product | cbd9d0ed66fc183f4075646a5b04bc34 | release, --no-default-features | as POST | s586/r1/diff/REPORT.md:3 |
| POST-SEP | s586/post/sep/{vcmp,velab,vrun,vita} | vcmp 4b7be41258306bd5496f30549dc1b7a8, velab f6b6fb64ed68bc7992c3b1980aa58853, vrun 777f21a490828b3446a9e180de285dde, vita ba3b9b9776b52368f5fe80bef7199679 | release, separate-bins | as POST | md5_at_preservation |
| POST2 | s586/post2/vita (review round 2) | 9d5134826d5c2e971c6ca7bf1e841a86 | release | wt-s586 + slice2.patch (md5 71224f4b…) | s586/r1/diff/REPORT.md:24, s586/r1/sound/REPORT.md:38 (= md5_at_preservation) |
| POST2-JIT | s586/post2/vita_jit | d3bdc20c00d464fb4d9c58bf6817270c | release, --features jit | as POST2 | s586/r1/diff/REPORT.md:24 prefix; full = md5_at_preservation |
| POST2-PRODUCT | s586/post2/vita_product | 4abff645289421a05c6e5f9ae8b5274b | release, --no-default-features | as POST2 | s586/r1/diff/REPORT.md:24 prefix; full = md5_at_preservation |
| POST2-SEP | s586/post2/sep/{vcmp,velab,vrun,vita} | vcmp 21c9036cd17e51f31be849a099384eb1, velab 74421968714b389e393a3154e1b05bc1, vrun ee826236cb00a3335c920284f879cadb, vita f5b1562229df228d2a21b4f587846e59 | release, separate-bins | as POST2 | s586/r1/sound/REPORT.md:38 prefixes; full = md5_at_preservation |
| POST3 | s586/post3/vita (docs-phase build) | ef8fa1e020501546ba751e111d1e0f95 | release (post3/build.log) | unrecorded (slice3.patch present, md5 3066170e…; link to the build not recorded) | s586/DOCS_ROWS.md:538 prefix; full = md5_at_preservation |
| unrecorded | r2/p0/*/vita.*, r2/prog/*/vita.* (run3.sh default VITA=$S/pre/vita, run3.sh:4; override unrecorded; r2/prog/plain/vita.err is byte-identical to docs_cells/plain/pre.err); g/c41_lanes/* and g/c48_lanes2/{def,jit,jits,prod,prodi,vc,ve,vr}.* (G7 lane table measured on the probe, s586/sec/G7.md:7-15; binary per file unrecorded); r1/diff/w nothing | unrecorded | unrecorded | unrecorded | unrecorded |
Tag rules: g/cNN `vita.*` = PRE (GROUNDING.md:10 "Runner: S/run3.sh (vita PRE; …)"), `probe.*` = PROBE
(GROUNDING.md:123), `pre.*` = PRE; r1/diff v/ and v2/ prefixes `pre`/`post`/`post2`/`jit`/`prod` from run.sh/run2.sh;
v2/stg vcmp/velab/vrun = POST2-SEP, `w.*` = POST2 (run2.sh:14-15); docs_cells `pre`/`post3` (DOCS_ROWS.md:537-538);
lanes/<side>/<case>: side pre/post/post2, `sep`/`wl` = <side>-SEP, `jit` = <side>-JIT, `prod`/`prodi` = <side>-PRODUCT
(lanes.sh); r3/<run>.{pre,post2}.* = PRE / POST2. g/c48_lanes2 `p_*` files are attributed to p.sv by file prefix.

## Oracles
- iverilog: "iverilog 13.0 -g2012, vvp -n" (s586/COMMIT_SLICE.txt:4); table header "iverilog 13.0" (s586/GROUNDING.md:13).
- verilator: "verilator 5.052 --binary --timing --assert, run with +verilator+error+limit+1000" (s586/COMMIT_SLICE.txt:6-7);
  banner "Verilator 5.052 2026-09-05 rev vUNKNOWN-built20260905" (s586/g/c01_ucasez/vl.cout:8, an excluded build log).
- sv2v, xcelium: not used.

## Verdict sources
Hand-mapped from lens finding lines (labels verbatim): s586/r1/diff/REPORT.md:15 (N1, c01_elabfail), :16 (N2,
c03_w2004), :17 (P1, b15_line), :18 (P2, e02_uif, b12_nested, e01 = r1/diff/cells/e01/m.sv), :19 (P3, e04_nomatch),
:20 (P4, b05_program), :28 (R2-1, a02dup = a01_x), :29 (R2-2, n02/n03/n06); s586/r1/sound/REPORT.md:30 (F1,
c02_cu_func_only); s586/DOCS_ROWS.md:556 ("an oracle split, not a defect", n06); DOCS_ROWS.md:543/545/547
(PROBE_CATALOG: p0/a_case, p0/a_if; p0/b_ident; prog/autonq, prog/modauto). The G1 table (GROUNDING.md:13-38) and
G5 table (sec/G5.md) record per-cell tool outputs but no class word: those cells are `unrecorded`.

## Layout
| set | dir | how a cell was identified | suffix → role | example |
|---|---|---|---|---|
| g (G1/G5) | g/cNN_*/ (t.sv; c52 + inc.svh) | dir holds iv.* or vl.* | iv.{cout,cerr,crc,out,err,rc} oracle_iverilog; vl.{cerr,crc,out,err,rc} oracle_verilator (vl.cout = c++ build stdout, excluded when it has no `%` line); vita.* PRE, probe.* PROBE, pre.* PRE | g/c01_ucasez/t.sv, g/c01_ucasez/iv.cerr, g/c01_ucasez/vl.out, g/c01_ucasez/vita.err |
| r1/diff | r1/diff/cells/ (+e01/, r2/, sub/*.f) | run name → source from run.sh / run2.sh / vl.sh (hand table in build586.py) | iv/<n>/{cout,crc,out} oracle_iverilog; vl/<n>/{cout,crc,out,rc} oracle_verilator (cout = merged compile output; excluded when no `%` line); v/<n>/, v2/<n>/ `<bin>.{out,err,rc,cerr,crc,log}` vita, `iv.cout`/`iv.out` oracle_iverilog, `vl.cout`/`vl.out` oracle_verilator | r1/diff/cells/b07_genfor.sv; r1/diff/iv/b07_genfor/cout; r1/diff/vl/b07_genfor/out; r1/diff/v2/b07_genfor/post2.err |
| r2/p0, r2/prog | r2/{p0,prog}/<case>/t.sv | run3.sh layout as g | as g; vita.* tag unrecorded | r2/p0/b_ident/iv.cerr |
| docs_cells | docs_cells/<case>/ (aliases of r2/* and r1/diff cells) | pre/post3 + iv/vl | pre.* PRE, post3.* POST3, iv.* / vl.* oracles | docs_cells/modauto/iv.out |
| doc_ex | doc_ex/m.sv | vl.cerr (verilator compile warnings) | vl.cerr oracle_verilator | doc_ex/vl.cerr |
| r1/sound/cells | c15_priority0.sv (doc-quote); m.sv, c02_cu_func_only.sv by md5 | oracle text quoted in REPORT only | - | r1/sound/cells/c15_priority0.sv |
| lanes, r3 | harness copies | outputs attached to the kept cell by md5 of the copied source (lanes) or by the single file named in stderr (r3) | out/err/rc, <step>.{out,err,rc} vita | lanes/post2/c52/err |
Kept copy of a duplicated source = earliest mtime among non-harness copies (doc_ex/m.sv 13:19 before r1/diff/cells/e01/m.sv
13:47; r1/sound/cells/c02_cu_func_only.sv 13:47:14 before r1/diff/cells/r2/n01_cu_func.sv 14:15). No VCD manifested
(VCDs exist only beside vita-only cells: g/c49_examples, lanes/*/ex_*).

## Counts
- Cells: 94 rows = 70 kept + 24 aliases. Kept per set: g 26 (c01-c25, c13b is an alias, + c52), r1/diff/cells 24
  (+ e01/m is an alias), r1/diff/cells/r2 11 (+ n01 alias), r2/p0 3, r2/prog 3 (+ b05 alias), doc_ex 1,
  r1/sound/cells 2 (c02_cu_func_only by md5, c15_priority0 doc-quote). Aliases: docs_cells 8, g 9 (c13b, c31_trace x4,
  c41_lanes, c48_lanes2 t/p, c53_log), r1/diff/cells 2 (e01/m, r2/n01), r1/sound/cells/m, r2/c02, r2/cells/c01e,
  r2/cells/c02, r2/prog/b05.
- oracles_present (kept): iverilog,verilator 55; iverilog 11 (r1/diff/cells/r2); verilator 1 (doc_ex/m); doc-quote 1;
  unrecorded 2 (r1/sound/cells/c02_cu_func_only, r2/prog/modauto: their oracle outputs sit under the aliases
  r1/diff/cells/r2/n01_cu_func and docs_cells/modauto/t, noted per row). Aliases: 6 with oracle outputs of their own.
- verdict_class / verdict_source: recovered 17 kept + 7 alias; unrecorded 53 kept + 17 alias.
- vita_tags: unrecorded 8 kept (doc_ex/m, c15_priority0, r2/p0 x3, r2/prog autonq/modauto/plain), 3 aliases.
- binary md5: full for every tag (POST2-JIT/PRODUCT/SEP and PROBE-PRODUCT, POST3 full only by md5_at_preservation).
- base commit: recorded for PRE, PRE-SEP, PROBE*, POST*, POST2*; unrecorded for POST3 and the `unrecorded` tag.

## Excluded (EXCLUDED.tsv, bytes)
build-dir 26,916,917,303 (probe_target 17.9 GB, r1/sound/mut_latch/target 9.0 GB, verilator obj_dir/obj/vl trees);
built-binary 203,056,592 (27 Mach-O); repo-copy 91,762,784 (probe_src, mut/src, r1/sound/mut2/src, mut_latch/src);
cargo-gate-corpus-mutation-log 4,226,403; doc-draft 337,259 (docs_edit, sec/*.md, GROUNDING.head, r3/explain*);
rust-patch-or-test-draft 292,382; verilator-build-log 165,869 (41); vvp 108,015; no-oracle-output 99,549 (426 files:
g/c30, c31 (3 non-alias designs), c42-c47, c49 examples, c50, c51; r1/sound/cells 16; r2/cells/c05; lanes runs of
those; r3 class_only/syntax); vita-staged 83,704; harness-copy 21,943 (89); third-party 13,429 (g/c40_ibex: vita
outputs of the ibex corpus run); script 9,790; run-metadata 9,485; vita-obs-json 7,765.

## Not recoverable
- r1/sound/cells oracle runs: only quoted or paraphrased in s586/r1/sound/REPORT.md (Q7 :22-26 c12 / m.sv, F2 :31
  c01/c08 "iverilog no sorry", Q-e :43 priority0); outputs never saved (c12.vvp, c12b.vvp, m.vvp are compiled images).
  Kept: c15_priority0 as doc-quote; m.sv via its md5 twins (doc_ex, r1/diff/cells/e01). c12_iv_shapes, c01_elab_err,
  c08_elab_err2 excluded (paraphrase, no raw text).
- r2/prog/modauto own run outputs were never captured in its dir (DOCS_ROWS.md:503); the draft-step re-run lives in
  docs_cells/modauto (alias, manifested).
- doc_ex/m.sv: iverilog compiled (a.vvp) but its vvp output was not saved; verilator run not saved (compile stderr only).
- docs_cells/n06 verilator: compile stderr only (vl.cerr); no run (verilator refused, DOCS_ROWS.md:554-555).
- Binary of r2/*/vita.* and of the c41/c48 lane files: unrecorded.

## Size
- Manifest (excluding role=alias): 1,694 files, 419,311 B. Alias rows: 51 (not copied).
- By role: vita 1117 / 139,362; oracle_verilator 244 / 121,710; verdict_table 7 / 120,749; sv 70 / 17,346;
  runner 5 / 9,996; oracle_iverilog 246 / 9,736; aux 5 / 412.
- By set: g/* 696 files / 72,092 B (c16_two_sites 153 / 15,167 incl. lane runs on its c41/c48 copies); r1/diff/cells 556 / 164,782
  (+e01 32 / 8,326, +r2 234 / 22,549); r2/p0 33 / 2,799; r2/prog 39 / 4,536; docs_cells 46 / 9,864; doc_ex 2 / 1,102;
  r1/sound/cells 20 / 1,538; set-level 36 / 131,723 (runners, verdict docs, r3 runs naming no source).
- 10 largest: DOCS_ROWS.md 41,007; GROUNDING.md 37,140; PLAN.md 19,140; COMMIT_SLICE.txt 8,860;
  r1/sound/REPORT.md 7,482; r1/diff/vl/a01/cout 5,990; r1/diff/REPORT.md 5,761; r1/diff/vl/e03_iv_sites/cout 5,481;
  r1/diff/vl/b08_casegen/cout 5,340; r1/diff/vl/b09_attr/cout 5,338.
- No file > 1 MB; no set > 5 MB.
- Third-party: none manifested (modname screen hits only `top`; no license text). g/c40_ibex excluded third-party.
