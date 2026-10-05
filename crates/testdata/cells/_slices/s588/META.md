# META s588 (phase 1: manifest only, nothing copied)

## Slice
- §4.5.588, ROADMAP row §2 🆕 AE (§5.2 row 1 at the time).
- Outcome as recorded: "§4.5.588 took §2 🆕 AE (§5.2 row 1) and built nothing: grounding, plan and the implementer's step 0 refuted every shape, so this commit is its record. No code changed." (s588/COMMIT_DOCS.txt:3-5). Title line: "🆕 AE not built: every cut descends, BLOCKED on row 15, 🆕 AC's sinks, a 4-state result channel and two enum lines" (s588/COMMIT_DOCS.txt:1).
- No POST binary exists (no code was built). Review: none.

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s588/pre/vita | e1e7e57148bb83ad35d2c4f68247a290 | release | HEAD 2f2d3f2d | s588/GROUNDING.md:4, s588/IMPL.md:5 |
| PRE-sep | s588/pre/sep/{vcmp,velab,vita,vrun} | vcmp f6c729fc39891ff9099cf84042e688f8, velab 2f45610b3b1baa571b95b28f2d8f4f60, vita d0e1cf9dff9b16fbff5f489f6b35cc15, vrun 3f42b9d96d8b7fd741758cac34e77c9e | release, separate-bins (profile unrecorded in s588 docs) | unrecorded in s588 docs (same md5s as s587 G0, HEAD 7b33a009 source) | md5_at_preservation |
| PROBE-a | path unrecorded (not kept under s588) | 2249f265256c275bcf5f3eda7b0d10f0 | release | "HEAD + logging only" (patch s588/g/probe.patch, excluded) | s588/GROUNDING.md:329 |
| PROBE-m | path unrecorded | 4d5e17eba66eab7172f25593a4a8cb88 | release | HEAD + logging + call counter | s588/GROUNDING.md:329 |
Tag use: `.PRE`, `.PRE0` (earlier stored run, byte-identical to `.PRE` on all 13 pairs), `.PRE1` (vita run of a `_rt`/`_hs` twin) -> PRE; `.PROBE` in g/a, g/a2, g/b, g/l, g/c -> PROBE-a, in g/m, g/m2, g/s -> PROBE-m (per GROUNDING.md:329); `.staged` -> PRE-sep.

## Oracles
- As captured: "iverilog 13.0 -g2012; verilator 5.052 --binary --timing; sv2v 0.0.13 -> iverilog" (s588/GROUNDING.md:108). Harness s588/g/run4.sh (manifested, role runner): vvp and the verilator sim wrapped in `perl -e "alarm 30"`, rc=142 = hang (s588/PLAN.md:5).
- sv2v binary used: s588/g/tools/sv2v (excluded as a binary; md5 ca3f9b7138513e09a88be96aa7b5ea19 = v0.0.13 per preserve/versions_at_preservation.txt).

## Verdict sources
- g/a, g/a2, g/b: GROUNDING Q3 table, column "PRE class" (s588/GROUNDING.md:127-268; same table in g/classify.txt).
- g/l, g/s: GROUNDING Q4 A lane table, columns F and D, quoted as `F: ...; D: ...` (s588/GROUNDING.md:284-297). The row names a short id (l01) and covers both `_d` and `_p` twins.
- g/m, g/m2: GROUNDING Q4 B table, column F (s588/GROUNDING.md:302-327).
- plan: PLAN §2 table column "F (by construction)" (s588/PLAN.md:31-51); the repeat cells p06/p07/p13/p14/p15/p16/p19 from the commit-message line "It would move 8 cells silent -> correct (...)" (s588/COMMIT_DOCS.txt:33).
- s0, s0e base cells: IMPL step-0 table, last column "under the planned guard" (s588/IMPL.md:43-58).
- g/c, docs, `_rt` twins, p04/p05/p08/p09/p12/p17/p18/p20, c03/c04/c05/c15/c16/r03/r04: no class word in the slice docs -> `unrecorded`; notes give the first doc line that mentions the cell.

## Layout
One flat dir per set; a cell = `<stem>.sv` with sibling outputs `<stem>.<suffix>`.
| suffix | role | tag |
|---|---|---|
| .sv | sv | - |
| .PRE / .PRE0 / .PRE1 | vita | PRE |
| .PROBE | vita | PROBE-a / PROBE-m |
| .staged | vita | PRE-sep (vcmp->velab->vrun) |
| .ivl | oracle_iverilog | iverilog |
| .vl | oracle_verilator | verilator |
| .s2v | oracle_sv2v (sv2v -> iverilog run + sv2v stderr) | sv2v |
| .s2v.v | oracle_sv2v_translation | sv2v |
| .s2v.err | oracle_sv2v_translation (sv2v stderr, mostly 0 bytes) | sv2v |
| .vu / .velab | excluded (staged artefact) | |
Example: s588/g/a/a01_x2a_ret4.{sv,PRE,PROBE,ivl,vl,s2v,s2v.v,s2v.err}.
Sets: g/a (40), g/a2 (23), g/b (79) interpreter cells; g/l (39 lanes x `_d`/`_p`), g/s (12 x 2), g/c (18 storage cells); g/m (24), g/m2 (3) moved-result lanes; g/staged (4 staged reruns, aliases of g/a, g/b); plan (planner cells p01-p21); s0 (31 seed-kind census cells + 31 `_rt` twins); s0e (10 enum cells + `_rt`/`_hs` twins); docs (8 cells measured during the docs step: d01rt..d12rt).
Generators kept: g/gen_ab.py (g/a, g/b), gen_a2.py, gen_l.py, gen_m.py, gen_s.py. No generator found for g/c, plan, s0, s0e, docs.

## Counts
- Cells (non-alias): 360 = g/a 40, g/a2 23, g/b 79, g/c 18, g/l 78, g/m 24, g/m2 3, g/s 24, plan 22, s0 31, s0e 14, docs 4.
- Aliases: 9 = g/staged 4 (of g/a/a01, g/a/a07, g/b/b61, g/b/b84), docs 4 (d01rt, d05rt, d07rt, d09rt = s0e `_rt` twins e01/e05/e07/e09; the oracle outputs live on the docs copy, the kept s0e copy has only `.PRE1`), s0e/m19_hs0 (= g/m/m19_repeat).
- Field recovery over 369 CELLS rows: verdict_class recovered 319, unrecorded 50; verdict_source same; vita_tags unrecorded 1 (s0e/m19_hs0, no outputs); oracles: 360 rows with manifested oracle files, 9 rows `none-on-this-copy` (aliases or kept copies whose oracle outputs sit on the other copy), 0 doc-quote; binary md5: 4 of 4 tags recovered (PRE-sep from md5_at_preservation); base commit: PRE recovered, PRE-sep and probes partly (see table).
- CELLS columns oracles_present / vita_tags list the outputs of that copy, including outputs that are MANIFEST `alias` rows (byte-identical to the kept copy). Generated helper files in this work dir (build.py, scan*, stats.txt, noora_cells.txt, run_stats.json; s588 also pc_lib.py / pc_engine.py / pc_stats.py) are not part of the manifest.

## Not recoverable / excluded cells
- Vita-only (no oracle output anywhere), excluded as `no-oracle-output`: s0 31 `_rt` twins, s0e 10 twins (`_rt` of e02/e03/e04/e06/e08/e10, `_hs` of e01/e04/e05, m19_hs), plan/p21_rt_display_x.sv (no output files on disk; PLAN.md:51 quotes PRE text and "(hang)" for iverilog). The IMPL step-0 table's "vita run time (`_rt`)" column and the commit message's `_hs` evidence (e01 n=2, e05 n=0 on the hand-spelled POST) rest on these vita-only twins.
- PROBE binaries are not on disk under s588 (outputs kept, binary not).

## Size
- Manifest: 2838 files, 585233 bytes (excluding 13 alias rows).
- By role: vita 666 / 137460; oracle_sv2v_translation 720 / 119110; sv 360 / 106998; oracle_verilator 360 / 80796; verdict_table 6 / 56420; oracle_sv2v 360 / 29033; generator 5 / 27520; oracle_iverilog 360 / 26819; runner 1 / 1077.
- By set: g/l 112050; g/b 106071; set-level g/ tables+scripts 85017; g/a 51783; s0 44380; g/s 36386; g/m 34393; plan 34330; g/a2 28231; g/c 21547; s0e 15786; docs 9492; g/m2 4846; g/staged 921.
- 10 largest: g/l_summary.txt 14755; g/classify.txt 13860; g/gen_ab.py 11981; g/b_summary.txt 10085; g/a_summary.txt 7523; g/s_summary.txt 6946; g/gen_l.py 5400; g/gen_a2.py 4743; g/gen_m.py 3265; g/a2_summary.txt 3251.
- No file > 1 MB; no set > 5 MB. No VCD files.
- Third-party screen: 0 hits (module names checked against preserve/thirdparty_modnames.txt, ignoring the generic `top`; no license/upstream markers).

## Excluded (see EXCLUDED.tsv)
binary 6 / 45421840 (pre/vita, pre/sep x4, g/tools/sv2v); log 31 / 815977 (g/suite.log nextest, corpus, build, .rc markers, empty run logs); doc-draft 10 / 286677 (docs/ROADMAP*, REMAINING_WORK, edit scripts); probe-log 4 / 181250; slice-record-doc 4 / 84609 (GROUNDING, PLAN, IMPL, COMMIT_DOCS: the verdict sources, parent decides); no-oracle-output 83 / 24811; script 3 / 9888 (classify.py, summ.py, runall.sh); rust-patch 1 / 6963 (g/probe.patch); vita-staged-artefact 8 / 5612; iverilog-image 1 / 4690. Accounting: 3002 files walked = 2851 manifest rows + 151 excluded.
