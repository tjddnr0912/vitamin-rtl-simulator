# META r9 (row-9 new-design census, 2026-10-07; unwired)

## Census
- ROADMAP §5.2 row 9 at 1d23be89: "new-design census: OpenTitan IPs (Apache-2.0), VeeR EL2 / EH1 (Apache-2.0),
  alexforencich verilog-axis / -pcie / -uart / -i2c (MIT) — licence and oracle first, then one corpus row each
  ...; report every error page by class (V, B, T, other ...)" (docs/ROADMAP.md:684 at 1d23be89).
- Scratch directory: `$S/row9` (durable copy `~/vita-evidence/2026-10-07-row9/row9/`). Paths below are relative
  to it; `row9/...` in `.expect` headers, `INDEX.tsv` and `FILES.tsv` is the same directory seen from `$S`.
- Families: A = OpenTitan a3490b42 (`A-opentitan/`), B = VeeR EL2 0169d669 / EH1 d04b1c7a (`B-veer/`), C =
  alexforencich axis 48ff7a7e / pcie 25156a9a / uart 1b867e53 / i2c a65be404 (`C-alex/`), and the judge's re-runs
  (`judge/`) (JUDGED_ROOTS.md:3-6). Every cell here is a minimal repro the census wrote; no upstream design is
  kept, and no capture of a run on an upstream design.
- Unwired: `r9` is not in `SEED_ROWS` (crates/corpus-runner/src/cells/mod.rs:50, `["AD", "AE", "AI", "U", "V",
  "W"]`), so admission, `MANIFEST.txt` and every test ignore it. A later `corpus-runner cells pin --rows r9`
  would read it (crates/corpus-runner/src/main.rs:191-195).

## Binaries
| tag | path under $S | md5 | profile | base commit | where it is recorded |
|---|---|---|---|---|---|
| FINAL | accept/vita-final | f4d778f25ab5921a472cf14fefd569e5 | release | 1d23be89 | md5: A-opentitan/REPORT.md:5, B-veer/REPORT.md:4 ("measured at start and end. Never rebuilt."), C-alex/REPORT.md:4, md5_at_preservation 2026-10-07 = same; commit: JUDGED_ROOTS.md:1 "frozen vita f4d778f2, HEAD 1d23be89"; profile: $S/accept/build.log:1 "Finished `release` profile [optimized]" (the build log beside the binary) |

- Commit detail (measured, not inferred): the binary's mtime is 2026-10-06 23:47:27 +0900, 33 s after 751dd35e
  (23:46:54) and before 1d23be89 (23:58:30). 1d23be89 changes docs/ROADMAP.md only, so its code tree is 751dd35e's.
- One binary for every vita capture: A-opentitan/bin/env.sh:5 (`VITA=$S/accept/vita-final`, used by rep.sh:6),
  B-veer/bin/tri:6, judge/bin/run4.sh:5 and :11, C-alex/scripts/drive.sh:4 / hand.sh:5. The C-alex repro captures,
  judge/v1 and judge/grid4 have no runner script; their family records name the same binary (C-alex/REPORT.md:4,
  JUDGED_ROOTS.md:1).
- Invocations differ: A `vita --top t <file>` (rep.sh:6), B `vita t.sv` (tri:6), judge `vita t.sv` (run4.sh:11).

## Oracles
- iverilog 13.0 (/opt/homebrew/bin), verilator 5.052, sv2v v0.0.13 ($S/blog4/sv2v/sv2v-macOS/sv2v, md5
  ca3f9b7138513e09a88be96aa7b5ea19, the same file as the earlier slices' sv2v). Captured quotes in TOOLS.txt (r9).
- A census (A-opentitan/bin/rep.sh:6-12): `iverilog -g2012 -s t && vvp -n` (iv.out), `sv2v f > sv.v && iverilog &&
  vvp` (sv.out), `verilator --binary --timing -Wno-fatal -Wno-lint -Wno-style --x-assign unique --x-initial unique`
  (build log vlb.out, run vl.out with `+verilator+seed+7 +verilator+rand+reset+2`). 16 out dirs have no verilator
  file (rep.sh:9 skips verilator when `NOVL` is set); out_r32_genblk_localparam_cond has a build log and no run.
- B census (B-veer/bin/tri:6-16): the same three chains on `t.sv`; sv2v with `-E Always` and the `(* full_case,
  parallel_case *)` attribute removed before iverilog; verilator `--binary --timing -Wno-fatal -Wno-lint -Wno-style`,
  build stderr to vl.err, and vl.out = `cat vl.err` followed by the run.
- judge (judge/bin/run4.sh:11-21): one capture per step: iv.c.out (iverilog compile), iv.out (vvp), s2v.c.out (sv2v,
  then iverilog compile appended), s2v.out (vvp), vl.c.out (`verilator --binary -Wno-fatal -Wno-lint -Wno-style`,
  no `--timing`), vl.out (run).
- C census repros: no runner script. Bare names (`iv.out`, `ivc.out`, `vl.out`, `vlrun1-3.out`, `vita.out`) and
  `<stem>.<tool>.out`.

## Exit codes (rc)
- A and B: each run's watchdog line (`perl $S/s2-review/sound/p3/wd.pl`, format `WD rc=N sig=S peak_rss_kb=K
  wall_s=T [KILLED...]`) went to a sibling `X.wd` (A rep.sh:6-12 `> X.wd`, B tri:6-16 `> X.wd`).
- judge: the watchdog line was appended to the capture itself (run4.sh:11-21 `>> X.out`); s2v.c.out holds two (sv2v,
  then iverilog compile) when sv2v produced output.
- C-alex repros: none (their scripts keep the watchdog line in shell variables, e.g. scripts/drive.sh:10-12,
  scripts/hand.sh:8-10). Nothing was invented for them: those blocks carry no marker.
- What the `.expect` blocks hold: the capture's bytes unchanged, then (where an exit code was recorded) one marker
  line, as the earlier slices' captures end in one: run steps `rc=N`; a compile step `rc=N` only when N != 0 (the
  README's "the run (or the compile before it) exited N"); a verilator build step `build_rc=N` only when N != 0 (a
  build that exited 0 is a build log, as the earlier slices kept them). N is the first non-zero `WD rc=` of the
  capture's watchdog line(s), else 0. A killed run (sig != 0 or `KILLED...`) gets no marker. Every such block's
  header ends with a field that quotes the watchdog line(s) verbatim, names the `.wd` file (or "the capture's own WD
  line"), and says which marker was appended or why none was; "(after a newline ...)" when one was added first. The
  `.wd` files are listed in FILES.tsv with role `watchdog` and the block they belong to.
- B vl.out's rc is the exit of `verilator ... 2>vl.err; rc=$?; cat vl.err; [ $rc = 0 ] && ./obj_dir/v` (tri:16):
  a failed build records 1 (the `[` test), not verilator's own exit code.
- Counts: 706 `rc=` and 8 `build_rc=` markers appended; 347 `.wd` files kept; 2 judge captures killed (sig=9:
  judge/runs/B-always_comb_shared_vector_pingpong-{t,star}/s2v.out), no marker.

## Verdict sources
- roadmap_row is a JUDGED_ROOTS.md root id; verdict quotes the census or the judge (row9/JUDGED_ROOTS.md,
  row9/A-opentitan/REPORT.md, row9/B-veer/REPORT.md; kept under files/). Rules, applied in this order:
  1. A cell named in the repro column of a census root (A-opentitan/REPORT.md:49-71; B-veer/REPORT.md:26-50, where a
     named dir means its t.sv, B-veer/REPORT.md:5 "repros/<name>/t.sv") whose id is a JUDGED_ROOTS.md id, or is listed
     in its "A other" (:34) / "B other" (:35) row: roadmap_row = that id; verdict = that JUDGED_ROOTS.md row's
     `judged` and `S/L` columns, written `<judged> / <S/L>` (for example `V / S (+L loud)`); verdict_source = that line.
     The census R01 is split by the judge into R01a (:24) and R01b (:20); r01a_shift / r01c_local (member width uses
     `>>`) are R01a, r01d_fncall is R01b by A-opentitan/REPORT.md:74 ("R03 -> R01 (constant function call
     `prim_util_pkg::vbits` ..., r01d)").
  2. A cell named only in a root's mechanism text: roadmap_row = that id, verdict `unrecorded` (grid4 g7-g9 under R04,
     r10c under R10, s2_named under R02, r02g.v under C-R02).
  3. A cell the census names with its own word under an id that is not a JUDGED_ROOTS.md id: roadmap_row
     `unrecorded`, verdict = that word (r15_lval_portsel "Not counted" A-opentitan/REPORT.md:77; r32 "no-oracle split"
     :86; t0_negedge_const_trst "harness-format" B-veer/REPORT.md:53).
  4. C-alex/REPORT.md is a stub ("status: IN PROGRESS"); C cells map through JUDGED_ROOTS.md:36 (C-R01: r01b.v, whose
     captures carry the quoted values) and :37 (C-R02: r02.v, the constant-false procedural `if`). judge/v1/p4.sv:
     JUDGED_ROOTS.md:47 "V1 other silent shapes (probe p4, ...)" -> verdict `silent`.
  5. Everything else: `unrecorded`. The notes field names the line used and why.
- A verdict describes the root the census filed the cell under, not the cell's own outcome; a control named in the
  repro column (r26b "the scoped call ... folds (r26b)") carries its root's verdict, and its notes say so.
- Recovered: 71 cells with a verdict (68 with a root id, 3 with a census word only), 6 with a root id only, 36
  neither.

## Layout
One cell = one census source. Byte-identical copies (all 103 judge runs copy their source to runs/<id>/t.sv or t.v,
run4.sh:9; judge/v1 copies 8 B-veer variants; B-veer nested_lvalue_select/t2.sv = t.sv) are `alias` header lines and
`INDEX.tsv` alias rows; their captures are attached to the kept copy with their own paths. Kept copy preference:
A-opentitan, B-veer, C-alex, judge/v1, judge/probes, judge/runs.

| family | source | captures (block kind) | dropped |
|---|---|---|---|
| A | `repros/<stem>.sv`, `repros/grid4/<g>.sv` | `out_<stem>/`: iv.out (iverilog), sv.out (sv2v), vlb.out (verilator [verilator build]), vl.out (verilator [verilator run]), vita.out; each with `<x>.wd` | `out_<stem>/sv.v` (sv2v translation), iv.vvp, sv.vvp; grid `<g>.vvp` |
| B | `repros/<dir>/t.sv` (+ variants `<v>.sv`, run only by the judge) | iv.out, s2v.out, vl.out (build stderr + run), vl.err ([verilator build stderr]), vita.out; each but vl.err with `<x>.wd`; aux `p.hex` (readmemh cells) | t_sv2v.v, iv.vvp, s2v.vvp |
| C | `repros/<dir>/<stem>.v` | by naming `<stem>.{iv,ivc,vita}.out`, or by a file name inside a bare-named capture, or by pairing with such a capture (see the attribution list below) | `<stem>.vvp` |
| judge | `runs/<id>/t.sv`/`t.v` (copies), `v1/<x>.sv`, `probes/<x>.sv` | runs: iv.c.out, iv.out, s2v.c.out, s2v.out, vl.c.out, vl.out, vita.out (watchdog line inside); v1: `<x>.vita.out`; grid4: `<g>.vita.out` (names A-opentitan/repros/grid4/<g>.sv inside) | runs/<id>/sv.v; v1 `<x>s.v`, `<x>s.vvp`; probes `<x>.s.v`, `<x>.vvp` |

- C attribution (by the census's naming where it names; otherwise by the capture's content; otherwise excluded):
  r01_t0_always_star: iv2.out ("r01.v:14") and ivc2.out (suffix 2, same mtime) -> r01.v; iv.out ("r01b.v:17"),
  ivc.out (pair), vita.out ("t1 e=0 q_next=00" is r01b.v's `$display` format), vl.out (same mtime as vlrun*),
  vlrun1-3.out ("r01b.v:17") -> r01b.v. r02_dead_reversed_psel: vita.out ("r02.v:11:5"), iv.out and ivc.out (no
  suffix, same mtime; "tid=5a" is r02.v's N=1 value), vl.out ("r02.v:11:8"), vlrun1-3.out (mtime pair of vl.out)
  -> r02.v; vlt.out ("r02t.v:10") -> r02t.v; r02g.* -> r02g.v; r02t.* -> r02t.v. Unattributable (excluded):
  vita2.out, iv5.out, ivc5.out, r02b.vvp (no r02b.v in the dir).
- Block order: oracle captures of the census, oracle captures of the judge, then vita (census, judge runs, judge
  v1 / grid4). Block paths are relative to `$S/row9` (`./` = beside the cell).
- files/ (paths as in `$S/row9`): verdict-source documents (JUDGED_ROOTS.md, A/B/C REPORT.md), first-party
  testbenches (A-opentitan/tb/, C-alex/hand/tb_*.v), runners (A-opentitan/bin/ but patches.py, B-veer/bin/ but
  apply_eh1w.py, apply_el2w.py, w20.py, w26.py, wp.py; C-alex/scripts/ but the `vita` symlink; judge/bin/), file
  lists (A-opentitan/ips/<ip>/tb/files{,.p}.txt, A-opentitan/union_files.txt, B-veer/work-el2/files-ahb{,-v,-w}.txt,
  B-veer/work-eh1/files{,-w,-na}.txt, C-alex/*.closure.json), result tables (A-opentitan/patchlog.txt, survey1.txt,
  judge/list{A,B,C}.txt). B-veer/bin/{tri,vita-eh1,vita-el2,vita-run,vlock,wd} are shell scripts (`file`), kept;
  B-veer/bin has no binary or symlink. A-opentitan/bin/mkpatched.py imports the dropped patches.py.
- No cell or file needed truncation (largest capture 5,762 bytes); no VCD exists in the census.

## Counts
- Cells 113: A-opentitan 53 (50 repros + grid4 g7, g8, g9), B-veer 47 (31 t.sv + 16 variants), C-alex 12,
  judge 1 (v1/p4.sv). Aliases 107: judge/runs 98, judge/v1 8, B-veer 1 (nested_lvalue_select/t2.sv).
  INDEX rows 220. (Before the bundle-1 review: aliases 110, INDEX rows 223; see "Licence rewrites" below.)
- Captures kept (blocks) 1031, none identical to another block of its cell: A census 214 (iv 49, sv2v 49, verilator
  67, vita 49), B census 150 (iv 30, sv2v 30, verilator 60, vita 30), C census 22 (iv 9, verilator 9, vita 4),
  judge runs 613 (iverilog 149, sv2v 177, verilator 189, vita 98), judge v1 8 (vita), judge grid4 3 (vita),
  re-captures of the three rewritten cells 21 (iverilog 6, sv2v 6, verilator 6, vita 3). Watchdog files 334
  (A 214, B 120). (Before the review: 1044 blocks, 347 watchdog files.) Aux 2 (+2 byte-identical judge copies listed).
- files/ 91 (verdict_source_doc 4, testbench 13, runner 42, file_list 27, result_table 5).
- DROPPED.tsv (cells kept): 183 sv2v translations, 131 compiled iverilog images (59 iverilog, 72 sv2v), 350,230 bytes.
- Excluded (EXCLUDED.tsv, 148 rows; 145 before the review, plus 3 licence-rewritten): no-output 63 cells (A grid1 17, grid2 17, grid3 3, grid5 5; B
  nested_lvalue_select g1-g13, t1, t3-t9), no-oracle-output 32 cells (vita only: A grid4 a1, g1-g6, g10 via
  judge/grid4; judge/probes 20; judge/v1 p1, p2, p3, row10), licence 10 rows (2 cells, 6 scripts, 2 patches),
  unattributable 4, upstream-run 10 rows, third-party-tree 7, third-party-overlay 7, scratch 1, binary 2, log 9.
- Accounting: 18,972 files walked under `$S/row9` = 113 cell sources + 110 alias sources + 1044 captures + 347
  watchdog files + 4 aux + 91 files/ (1,709 in FILES.tsv or INDEX) + 314 DROPPED + 246 excluded per cell/file +
  16,703 excluded by area.

## Licence screen (every kept file)
- Upstream = pristine trees A-opentitan/ot, B-veer/{eh1,el2}, C-alex/verilog-{axis,i2c,pcie,uart}; census-made copies
  (A-opentitan/patched, B-veer/{eh1,el2}{s,v,w}) were indexed too, and a hit only there is census text.
- Pass 1: lines normalised (strip, collapse whitespace); lines under 25 characters or made only of SV keywords are
  trivial; count of non-trivial lines found verbatim upstream and the longest run. Pass 2: 8-token shingles per line
  (script escapes expanded), longest matched span. Pass 3: comment-stripped HDL token streams, 12-token shingles
  across line breaks, longest covered run. Licence markers: Copyright, SPDX, lowRISC, Western Digital, CHIPS
  Alliance, Forencich, "Licensed under". Flagged = pass-1 hit, a marker, pass-2 span >= 12 or a pass-3 hit; every
  flagged file was read.
- Result: 1,541 files screened, 146 flagged; kept 106 (generic idioms, port-connection lists, file paths, statements
  about licences, identifiers); dropped 10 (EXCLUDED.tsv `licence`, with the upstream file:line); the other 30
  flagged files are out for another reason (no capture, vita only, run on upstream).
- Kept after reading, the closest calls: B frame_fn_concat_lhs/t.sv (one statement of upstream dasm16_ciw with its
  operand renamed, B-veer/eh1/testbench/dasm.svi:60), A r30_gen_strcast_cond.sv (one generate condition naming the
  upstream parameter `LfsrType` and its value, prim_lfsr.sv:283), B std_randomize_with/t.sv and assoc_typed_index/t.sv
  (one reduced statement / declaration each), B-veer/bin/w22.py (its search key is the one-statement tie-off `assign
  jtag_trst_n = 1'b0;`), B-veer/bin/mk_{oracle,eh1}_copies.sh (keys are census text and short declarations),
  C-alex/scripts/upsets.py (parameter names and numeric tuples transcribed from upstream test parametrize lists).
- Dropped repros: nested_lvalue_select/rv.sv (copies beh_lib.sv rvdff/rvdffs) and fn_retvar_partsel_ca/t.sv
  (reproduces the upstream function countones, el2_ifu_bp_ctl.sv:896-903), each with its judge copy and every
  capture. fn_retvar_partsel_ca was the repro of B root O9 (B-veer/REPORT.md:41); O9 has no cell here.

## Licence rewrites (bundle-1 review N2, 2026-10-07)
- A r01d_fncall, r26_pkgparam_impfn and r26b_pkgparam_scopedfn reproduced OpenTitan's `prim_util_pkg::vbits`
  (ot/hw/ip/prim/rtl/prim_util_pkg.sv:43-45) whole with its argument renamed: the same call the countones drop
  above applied to. They are the only cells of roots R01b and R26, so they are rewritten rather than dropped:
  `vbits` becomes the first-party `bits_for` (`return $clog2(n + 1);`), which returns the same value at every
  argument these cells pass (5 -> 3, 21 -> 5); nothing else in the sources changes. The census captures of the
  original sources (census and judge copies, 50 FILES.tsv rows) are no longer kept (EXCLUDED.tsv
  `licence-rewritten`), and the three judge copies stop being aliases.
- Re-captured oracle first, with the judge's commands (judge/bin/run4.sh order changed to iverilog, sv2v ->
  iverilog, verilator under the machine lock, then vita): S/bundle1/tools/n2run.sh, outputs under S/bundle1/n2/
  (block paths `bundle1/n2/<cell>/...` are relative to `$S`, not to `$S/row9`). vita is the census binary FINAL
  (md5 f4d778f2...). Every oracle prints the census's values (`A s=6 e=1 bits=4 L=3`, `A W=5 R=6`, `A W=5`) and
  vita prints the census's diagnostics unchanged (r01d E2002 at 6:27, r26 two E3009 on `W` and `R`, r26b runs and
  prints `A W=5`), so the cells still witness the judged roots.

## Not recoverable
- Exit codes of the C-alex repro captures (never written to a file).
- Oracle captures of the A grids except grid4 g7-g9 (grid1 17, grid2 17, grid3 3, grid4 8, grid5 5 sources; grid4's 8
  have judge/grid4 vita captures only): iverilog compiled most of them (`.vvp` present) but no output was saved; their results exist only as REPORT text
  (A-opentitan/REPORT.md:49 "grid1: 10/17 forms E2002", :51 "grid4 cells 3-10", :52 "grid2", :59 "grid5").
- Captures of B nested_lvalue_select g1-g13, t1, t3-t9 (sources only); oracle captures of judge/probes and judge/v1
  p1-p3, row10 (vita only; sv2v translations and `.vvp` exist, no output).
- Which source produced C r02_dead_reversed_psel vita2.out, iv5.out, ivc5.out (r02b.v is gone).

## Wiring notes (for a later `cells pin --rows r9`; nothing here was changed)
- judge captures keep their watchdog line before the appended marker; normalization does not drop it.
- A cell holds census and judge captures of the same tool (different file names in the text); admission compares
  every capture of a tool.
- C census captures carry no marker, so admission reads them as no run.
- The two readmemh cells have `.aux/` (needs-aux).

## Size
- r9/: 226 files (113 sources, 113 `.expect`) + 2 aux files; _slices/r9/: META.md, FILES.tsv (1,573 rows),
  EXCLUDED.tsv (148 rows), files/ (91). Bytes: r9/ 1,148,132; _slices/r9/ without this file 707,073.
