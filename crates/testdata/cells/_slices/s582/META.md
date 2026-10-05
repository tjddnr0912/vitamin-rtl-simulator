# s582 oracle-cell census (phase 1: manifest only)

## Slice
- Slice: §4.5.582. ROADMAP row: §3.b `case-inside` (archive record names it: "§3.b `case-inside` closed and replaced by `case-inside-residue` ... and `inside-name-use`").
- Outcome as recorded: "`case (e) inside` runs where the reference tools' sizing rules agree — each item compared with the `inside` operator against a case expression evaluated once, every other shape one E3009, and every case-inside declined in a design that uses `inside` as a name, after a scope-aware check blocked twice on one axis (2026-10-02, branch wip/s582-case-inside) ✅" — source: `git show acabe991:docs/history/ROADMAP_ARCHIVE.md` line 576 (repo; commit acabe991 "docs: record §4.5.582").

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s582/pre/vita | 4ace7617440041d0d16aee5309fded10 | release | repo HEAD 60c70e05 (main) | s582/plan/PLAN.md:3 (also md5_at_preservation matches) |
| POST | s582/post/vita | f285440f37b22c9cdf32eeb8ebb1e721 | release | branch wip/s582-case-inside, working tree (src status: s582/post/src_status_at_freeze.txt) | s582/post/CENSUS.md:3; s582/lens_diff/REPORT.md:4 |
| POST2 | s582/post2/vita | f491ab7b4921b912baa27b9828641232 | release | wip/s582-case-inside working tree | s582/lens_diff/REPORT.md:67; s582/lens_sound/REPORT.md:88 |
| POST3 | s582/post3/vita | 9b0c1623e3acb71130efe3acb7713a89 | release | wip/s582-case-inside working tree | s582/lens_diff/REPORT.md:111; s582/lens_sound/REPORT.md:116 |
| DBG1 | s582/tests/vita_dbg | 2674c193c5cdb62f66e0a380149d04a7 | debug (33.9 MB) | unrecorded | md5_at_preservation |
| DBG2 | s582/tests/vita_dbg2 | a773c0b4ceb321470aed4d0fc9c0d7bb | debug | unrecorded | md5_at_preservation |
| DBG3 | s582/tests/vita_dbg3 | 159907e8a9b77d85a93b90c2f89bf7e3 | debug | unrecorded | md5_at_preservation |
| unrecorded | (binary not recorded) | unrecorded | unrecorded | unrecorded | unrecorded |
- Tag form in MANIFEST: `<binary>` or `<binary>/<lane>` (lane = default/interp/vm/native/staged/oneshot, or `--top m_ffor`); the binary part is the row above.
- DBG1/2/3 on `tests/sv/out/*/post.out`: inferred from file mtime (each output written 1 s .. minutes after the matching tests/vita_dbg* mtime: 10:13:33, 11:20:26, 11:51:44); `tests/run.sh` takes the binary as `$1` and no log records which. Flagged as inferred.
- `be/*` and `staged/*` (tests/sv lanes, 10:36) and `grounding/be/*` (13 d-spelling lanes): binary not recorded -> tag `unrecorded/<lane>`.
- Source-freeze hashes exist but are not binary md5s: s582/post/src_diff_at_freeze.md5, post2/src_at_freeze.md5, post3/src_at_freeze.md5 (excluded, listed here).
- Not used by any manifested output: s582/docs_probe/vita_dbg md5 0a3747994ee1422de16669b4dcb9a361 (md5_at_preservation).

## Oracles
- Captured in slice text: "iverilog 13.0 (`-g2012`, run on POST's census pass), sv2v 0.0.13 → iverilog 13.0, verilator 5.052 (`--binary --timing`; 2-state)" — s582/post/CENSUS.md:4. verilator raw banner in outputs: "Verilator 5.052 2026-09-05" (e.g. s582/grounding/gen/out_c01_repro_ci/vl.out). sv2v binary used by every runner: s580/sv2v/sv2v-macOS/sv2v (runner scripts line 4/5).
- Lens .v cells run `iverilog -g2005` and `verilator --default-language 1364-2005` (s582/lens_diff/run3.sh line 8); lens_sound `iv` = iverilog default generation (s582/lens_sound/run.sh).

## Verdict sources
- s582/post/CENSUS.md table, last column "expected / verdict" (all census cells: grounding *_ci, b2/b3 hand cells, plan/probe, tests/sv, tests/px).
- s582/grounding/REPORT.md tables, last column "hand-IEEE / verdict" (grounding gen/b2/b3 cells not in the census; row key = cell id prefix such as `c05b`, `m2b`, `f1`; a key with several rows and no discriminating word -> unrecorded).
- s582/lens_diff/REPORT.md and s582/lens_sound/REPORT.md prose: the first line naming only this cell, quoting the first verdict word from a fixed vocabulary (clean, BLOCKING, NON-BLOCKING, loud regression, pre-existing, no slice finding, No gap, over-refusal, disqualified, no-oracle, byte-identical, ...) or that word in the enclosing `###` heading; a line naming several cells is skipped.
- s582/post3/DELTA.md, s582/post2/DELTA.md prose for tests/sv cells absent from the census (same rule).
- Matrix cells (grounding/mx): no per-cell verdict word in any doc (grounding/mx/analysis.txt holds per-row observations; step0/step0b.out the aggregate) -> verdict_class unrecorded.

## Layout
- grounding/{gen,b2,b3,cells}, plan/probe, step0/h, grounding/mx: cell = `<set>/<name>.sv`; outputs in `<set>/out_<name>/`. Runner s582/grounding/run.sh (mx: grounding/mx/runmx.sh). Suffix -> role: `vita.out` vita PRE; `iv.out` oracle_iverilog (`iverilog -g2012`); `sv.out` oracle_sv2v; `sv2v.v` oracle_sv2v_translation; `vl.out` oracle_verilator; `vl_build.log` oracle_verilator only when it holds a `%Warning`/`%Error` line (else excluded as a make/c++ log); `iv.vvp`/`sv.vvp`/`obj_dir` excluded. Example: grounding/gen/c01_repro_ci.sv -> grounding/gen/out_c01_repro_ci/{vita.out,sv.out,sv2v.v,vl.out}. Spellings: `_ci` = true case-inside, `_d`/`_if` = vita-side spellings (cells only where an oracle output exists in their own out_ dir).
- post/census/<path with / as __>/{post.out,iv.out}: POST vita + iverilog on the census cells (runner post/census_run.sh). Example: post/census/grounding__gen__c01_repro_ci/post.out.
- post2/cmp/<path __>.sv/{post.out,post.err,post2.out,post2.err} and post3/cmp/.../{post2.*,post3.*}: re-score vita outputs (tags POST, POST2, POST3) attached to the cell named by the dir.
- tests/sv/<name>.sv -> tests/sv/out/<name>/{pre.out,post.out,iv.out,iv5.out,sv.out,sv2v.v,vl.out,vl_build.log} (runner tests/run.sh; generator tests/designs.py); tests/px/mtm_plain.sv likewise. The two-file pair tests/sv/xa.sv + xb.sv -> tests/sv/out/xab/ and post3/cmp/xab/ (cell xa, xb = aux).
- be/<name>.<lane>, staged/<name>.<lane|c.log|e.log|r.log> -> tests/sv/<name>.sv (tag unrecorded/<lane>); post2/be, post3/be/<file>.<lane> -> cell named by <file>; grounding/be/<name>.<lane> -> grounding cell.
- lens_diff/{d,r2/d,r3/d}/<name>.(sv|v) -> out_<name>/{pre.out,post.out,post2.out,post3.out,iv.out,sv.out,sv2v.v,vl.out,vl_build.log,post.vcd}; lens_diff/be/<name>/<lane>/out.txt (POST/<lane>); lens_diff/r2/rescore/{post,post2}_<file>/out.txt and r3/rescore/{p2,p3}_<file>/out.txt (POST/POST2/POST3).
- lens_sound/{d,r2}/<name> -> out_<name>/...; lens_sound/r2/rerun/<name>.{post,post2}, r3/rerun/<name>.{post2,post3}; lens_sound/r2/vl_<file>.log = verilator.
- Doc-quote cells (no oracle file; oracle text quoted in the lens REPORT line): lens_diff/r3/d/r3f_top.v (+ aux r3f_child.v), r3d_cmdline.v; lens_sound/r3/r3a_macro.v, r3b_marg.v, r3c_inc.v (+ aux r3c_inc.vh), r3d_xa.v (+ aux r3d_xb.v).

## Counts
- Cells: 379 (aliases 0; oracles_present=doc-quote 6)
- Per set: grounding/b2 20; grounding/b3 16; grounding/cells 1; grounding/gen 43; grounding/mx 156; lens_diff/d 29; lens_diff/r2/d 4; lens_diff/r3/d 6; lens_sound/d 15; lens_sound/r2 4; lens_sound/r3 4; plan/probe 4; step0/h 6; tests/px 1; tests/sv 70
- Unrecorded per field: oracles_present 0/379; vita_tags 91/379; verdict_class 219/379; verdict_source 219/379
- binary md5: recorded in slice docs for PRE/POST/POST2/POST3; DBG1-3 md5_at_preservation; `unrecorded` tag for be/staged/grounding-be lanes. base commit: PRE recorded (60c70e05); POST..POST3 = branch only.

## Not recoverable / not cells
- step0 hand-spelled model cells (step0/b 46 SVs, step0/h 13 `*_b.sv`): vita-only; their oracle text is the twin grounding `*_ci` output, compared in step0/cmp0a.out ("cells 59 mismatch 1"). Excluded as no-oracle-output by the SPEC rule; parent may decide otherwise.
- grounding *_d / *_if spellings with only a vita.out (65 gen + 7 b2/b3) and grounding/be lanes: vita-only, excluded.
- lens_diff r2: r2a_c, r2b, r2b2, r2e_unitvar, r2f_unitfn, r2h_loopvar, r2i_blocks, r2k_tdcast and r3: r3g/r3h/r3i/r3j/r3k: no oracle output and no oracle quote (lens_diff/REPORT.md: r2h "no oracle (IEEE 1800 reserves `inside` ...)", line 94). lens_sound r2: r2b_pkg, r2c_cls_let, r2d_unit, r2g_sv, r2i_implicit: PRE/POST2 only. lens_sound/d d03b, d09: vita only. post3st/ca,cb,cc and staged/v01d: staged vita-only.
- lens_diff r3e oracle rerun with -I (lens_diff/REPORT.md:120) has only the doc quote; out_r3e_inc/iv.out holds the captured run without -I ('Include file r3e_decl.vh not found'), manifested as captured.

## Size
- Manifest total (excluding role=alias): 4168 files, 3978413 bytes (3.79 MiB); alias rows: 0
- By role: vita 2589 files / 1167761 B; oracle_sv2v_translation 330 files / 1120274 B; sv 379 files / 947946 B; oracle_verilator 330 files / 270899 B; oracle_iverilog 172 files / 255254 B; verdict_table 6 files / 87082 B; oracle_sv2v 330 files / 60388 B; generator 4 files / 38414 B; vcd 15 files / 18675 B; runner 8 files / 11067 B; aux 5 files / 653 B
- By set:
  - grounding/mx: 624 files / 1917558 B
  - grounding/gen: 576 files / 494057 B
  - tests/sv: 1520 files / 435289 B
  - lens_diff/d: 603 files / 409794 B
  - lens_sound/d: 258 files / 188196 B
  - grounding/b2: 223 files / 177079 B
  - set-level (verdict_table): 6 files / 87082 B
  - grounding/b3: 119 files / 61368 B
  - lens_diff/r2/d: 48 files / 60219 B
  - lens_sound/r2: 43 files / 51239 B
  - set-level (generator): 4 files / 38414 B
  - plan/probe: 58 files / 18595 B
  - set-level (runner): 8 files / 11067 B
  - step0/h: 24 files / 9072 B
  - lens_diff/r3/d: 25 files / 7449 B
  - tests/px: 17 files / 7191 B
  - grounding/cells: 6 files / 3678 B
  - lens_sound/r3: 6 files / 1066 B
- 10 largest manifested files:
  - grounding/mx/analysis.txt (verdict_table) 30023 B
  - post/CENSUS.md (verdict_table) 28668 B
  - tests/designs.py (generator) 25380 B
  - grounding/gen/summary.txt (verdict_table) 18533 B
  - grounding/b2/out_m2_sidefx_d/vl_build.log (oracle_verilator) 15964 B
  - grounding/mx/out_E2s4__I02_s4m2_ci/sv2v.v (oracle_sv2v_translation) 12655 B
  - grounding/mx/out_E2s4__I02_s4m2_if/sv2v.v (oracle_sv2v_translation) 12655 B
  - grounding/mx/out_E1u4__I02_s4m2_ci/sv2v.v (oracle_sv2v_translation) 12648 B
  - grounding/mx/out_E1u4__I02_s4m2_if/sv2v.v (oracle_sv2v_translation) 12648 B
  - grounding/mx/out_E3s8__I02_s4m2_ci/sv2v.v (oracle_sv2v_translation) 12543 B
- Files > 1 MB: 0
- Sets > 5 MB: 0
- VCD rows (role vcd; repo .gitignore has *.vcd, parent decides): 15 files / 18675 B

## Third-party screen
- Manifested .sv/.v module names checked against $S/preserve/thirdparty_modnames.txt: only the generic name `top` matches (every cell's top module); no upstream text markers (lowRISC/Forencich/Copyright/SPDX/ibex_/axi_). third-party-suspect: none.
