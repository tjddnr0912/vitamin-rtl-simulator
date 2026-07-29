# Changelog

All notable changes to **vitamin** are recorded here. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/); dates are ISO-8601.

## [Unreleased] — 2026-07-29

### Fixed
- **Definite-assignment for `automatic` block-locals now understands control
  flow.** A `break`/`continue` placed before the local's first write no longer
  reads as a live path arriving at a later read unwritten — every path that
  reaches that read has already executed the write. `if`/`else` and `case` arms
  that jump drop out of the merge the standard way. A real `disable` is
  deliberately *not* treated as a jump: it may name a block that is not an
  ancestor, so the statement runs on.
- **A statement-position call no longer ends the definite-assignment scan.**
  Calls are now proven harmless instead of assumed harmful: the callee's body is
  walked (transitively, on a depth budget) for any reference to the name,
  including hierarchical self-paths such as `t.a`. A callee that can reach the
  name still makes the call a read.
- **Fixed-size unpacked arrays declared `automatic` in a block.** A `'{…}`
  declaration initializer is supported (it re-runs on each block entry, matching
  the dynamic-dimension form), and an array filled element-by-element with
  literal indices is accepted once every declared index has been written.
- **The same name reused at two nesting levels of sibling block trees.** The
  net-creation and body-lowering phases nested their per-block scopes
  differently, so with both levels scoped the nets were created under one path
  and resolved under another. They agree at any depth now.
- **A comma declaration is treated as independent declarators.** Previously a
  collision on the first name rejected all of them — with a message naming a net
  that did not exist — and dropped the whole declaration, so every later use
  reported "undeclared net/variable" one line below its own declaration.
- **`q.pop_front();` / `void'(q.pop_front());` on a queue of structs** whose
  member width comes from a named parameter. Such a queue is stored per field,
  and the per-field fan-out had no discarding-pop case.
- **`return f(arr);` where `f` takes a dynamic-array formal**, and the same call
  buried in a concatenation.

### Changed
- Diagnostics: the hierarchical-task-call rejection now carries
  `file:line:col`; the dynamic-array-formal message no longer states a
  "module-process level" restriction that no longer exists, and names the actual
  reason instead; the same-name block-local message no longer infers the other
  declaration's lifetime.

## [Unreleased] — 2026-07-17

### Changed
- `format_version` 21 → 22: IEEE **two-stage `#delay` conversion** — in
  mixed-precision designs a delay is now rounded to the declaring module's own
  `` `timescale `` precision first, then converted to the global simulation
  precision (iverilog-verified). The artifact timescale trailers are extended
  accordingly. (The full per-version history lives in the comments of
  `crates/vita-artifact/src/header.rs`.)

### Fixed
- `**` (power): exponent no longer truncated at narrow result widths.
- Conversion of >64-bit **signed** values to `real`.
- Overflow panic on package-enum label ranges.
- u32 overflow panic in indexed part-selects.
- `W-FLIST-OVERRIDE` now flows through the diagnostic gate, so `-Werror=` and
  `-Wno-` apply to it.

### Added
- FST output: case-normalization hardening of the `.fst` extension dispatch.
- The dev-only `separate-bins` Cargo feature now really builds standalone
  `vcmp`/`velab`/`vrun` binaries (thin shims sharing the multicall path).

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
