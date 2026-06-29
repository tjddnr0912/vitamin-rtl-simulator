# Changelog

All notable changes to **vitamin** are recorded here. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/); dates are ISO-8601.

## [Unreleased] — Phase-1 MVP

vitamin's first milestone: a working, deterministic, 3-OS-reproducible RTL
simulator for the Verilog-2005 synthesizable subset plus a synthesizable
SystemVerilog subset. The full `preprocess → lex → parse → elaborate → sim-ir →
sim-engine → VCD` pipeline drives both the one-shot `vita` flow and the staged
`vcmp → velab → vrun` flow.

### Language & front-end
- Verilog-2005 synthesizable RTL: modules, ANSI/non-ANSI ports, parameters/
  localparam, `generate`/`genvar`, functions/tasks, hierarchy & instances.
- SystemVerilog subset: `logic`, `always_ff`/`always_comb`/`always_latch`, and the
  user-defined types **enum**, **typedef**, and **packed struct**.
- Multi-dimensional **packed** and **unpacked** arrays.
- Full operator precedence; `casez`/`casex`; `fork`/`join`/`join_any`/`join_none`;
  `#delay`, `@event`, `wait`.
- `` `timescale `` (doc-08 model): unit/precision scaling of `#delay`, `$time`,
  `$realtime`.

### Engine & output
- Event-driven IEEE-1364 scheduler (Active / Inactive / NBA regions), 4-state values.
- Word-parallel (u64) 4-state bitwise & reduction evaluation.
- Hierarchical VCD output with real signal names and module scopes.
- `$display`/`$write`/`$monitor`/`$strobe`; `$dumpfile`/`$dumpvars`/`$dumpon`/
  `$dumpoff`/`$dumpall`; `$finish`/`$stop`; `$time`/`$realtime`; real support
  (`$rtoi`/`$itor`/`$realtobits`/`$bitstoreal`).

### Determinism & artifacts
- SchemaHash-frozen `sim-ir` golden root → 3-OS byte-identical output.
- Staged artifacts (`.vu` / `.velab`) with `format_version` staleness gating.
- Stable diagnostic codes (`VITA-Exxxx` / `VITA-Wxxxx`), doc-15 bijection.

### Verification
- 419 workspace tests; differential harness against Icarus Verilog (`iverilog` +
  `vvp`) for representative designs (skips gracefully when not installed).

### Internal artifact-format history
- `format_version` 1 → 2: staged artifact re-rooted at `SimIr` (M3 IR-backbone freeze).
- `format_version` 2 → 3: `real` (f64) type evolution.

### Known limitations
See [docs/manual/006_limitations.md](docs/manual/006_limitations.md). In brief:
arithmetic lane is 128-bit unsigned / 64-bit signed (wider poisons to X, fail-safe);
`casez`/`casex` treat scrutinee-x as don't-care; `$dumpvars(depth,scope)` args are
ignored (full dump); VCD memory dump is word-0 only; `%t` lacks default field width.

### Platforms
Linux and macOS (CI: Ubuntu, macOS, RHEL9). Windows is not currently supported.
