# r3 (no slice record)

- Slice: unrecorded. No document under $S names `$S/r3` or any of its cell names. The files are dated
  2026-10-01 16:11:32–16:12:40; the oldest entries of $S/s580 are `pre/` (16:11:39) and `sv2v/` (16:11:54).
  The nearest repository commit is 7b804c62 (2026-10-01 16:14:44 +0900, "docs: register the external report's
  fourth item and a unique-if chain gap (§5.2 rows 3–4)"). This is a timestamp fact, not an attribution.
- ROADMAP row: unrecorded.

## Binaries
None. r3 holds no vita output.

## Oracles
- verilator: 5.052, captured in each `obj_<cell>/V<cell>__verFiles.dat` line 3
  (`/opt/homebrew/Cellar/verilator/5.052/bin/verilator_bin`) and the command line on line 2
  (`--binary --timing -Wno-fatal -Wno-lint -Wno-style --Mdir obj_<cell> <cell>.sv`). The .dat file is kept as role `runner`.
- iverilog: compiled images (`repro_unique.vvp`, `u0.vvp`) exist, but no iverilog/vvp stdout was captured: unrecorded.
- sv2v: not run.

## Verdict sources
None: verdict_class and verdict_source are `unrecorded` for all 12 cells.

## Layout
Flat: `<cell>.sv` + `vl_<cell>.err` (verilator build stderr, all 12 are 0 bytes) + `obj_<cell>/V<cell>__verFiles.dat`.
Example: `ui_both.sv`, `vl_ui_both.err`, `obj_ui_both/Vui_both__verFiles.dat`.

## Counts
12 cells, 0 aliases (no byte-identical SV). Recovered vs unrecorded: verdict_class 0/12, verdict_source 0/12,
vita_tags 0/12, oracle outputs = verilator build stderr 12/12 (simulation stdout 0/12), binary md5 n/a, base commit n/a.

## Not recoverable
Simulation output of every oracle (never captured: no `.out`/`.log` next to the cells; the verilator `V<cell>`
executables were built but their run output was not written to a file).

## Size
36 files, 32865 bytes (12 `.sv` 2.9 KB, 12 empty `.err`, 12 `__verFiles.dat` about 30 KB).
