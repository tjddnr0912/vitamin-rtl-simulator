# Changelog

All notable changes to **vitamin** are recorded here. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/); dates are ISO-8601.

## [Unreleased] — 2026-07-29

### Fixed

- **A callee that advances time no longer ends the caller's definite-assignment
  scan.** The deep (callee-body) reference walker had no arm for `@(posedge clk)`,
  `#1`, `wait`, or `wait fork`, and a `_ => false` in a walker keyed on a name means
  "may reference ANY name". One timing control anywhere in a task therefore made every
  block-local declared after a call to it unusable — which is the standard clocked
  driver (`preload()`, `run_scenario()`), and was 11 of the 12 diagnostics in the
  round-18 report. The same mistake had been fixed in the shallow twin a round earlier.
- **SILENT-WRONG: a shared flattened net plus a call that suspends.** Two same-named
  block-locals share one net; one block writes it, calls a one-line helper that does
  `@(posedge clk)`, and reads back — observing the sibling block's write. vita printed
  `A v=99` where iverilog prints `A v=1`, at exit 0, identically at `c8ad2b4` and
  `46b9816`. The shared-net rule read only *syntactic* timing, so a wrapper hid the
  suspend; and the top-level walk returned early once the local was assigned, so the
  rule was never consulted at all. Both are closed.
- **A struct member write that covers the whole variable is a whole write.**
  `rm.c = 5;` on a single-member struct left the walk believing `rm` was unwritten.
  Members are constant part-selects after the parser's desugar, so the rule is bit
  coverage — which also accepts field-by-field writes and hand-written
  `x[31:16] = a; x[15:0] = b;`. Partial coverage stays loud.
- **`automatic` is no longer silently dropped from an unpacked-struct block-local.**
  `automatic rec_t r;` could not be resolved by the automatic-lifetime parse helper
  (an unpacked struct is not in the typedef table), so the member fan-out parsed it
  with no lifetime at all and the declaration became static. Two same-named struct
  locals in disjoint blocks then shared one net, while the identical `automatic int`,
  enum, and typedef-alias forms each got their own scope.
- **The first block declaring a shared name is now asked the same question as the
  rest.** The coalesce guard keys on the net already existing, so it only ever saw the
  second and later declarations — an ordering artefact, not a semantic distinction.
  The decision is now a pure AST pre-pass.
- **A call actual naming an unpacked-struct record is seen as touching its member
  nets.** The SoA fan-out renames the variable but not the call arguments, so a write
  through a formal was invisible (a false-loud) and an `inout` copy-in read was too.

### Added

- **`-v` now prints what actually ran.** Driven from a Makefile or a wrapper
  script, the arguments you read and the arguments the process received are
  different texts: the shell has already substituted `$(WIDTH)`, the filelist
  expander has spliced `-f` frames away, and `VITA_THREADS` never appears in the
  command line at all. `-v` prints the resolved form as a block at the top of the
  transcript — the invocation (shell-quoted, paste-able), cwd, every filelist
  opened, the sources actually compiled, incdirs, defines, runtime plusargs,
  output/log paths, elaboration roots and libraries, timeout, and the thread count
  *with its provenance* (`--threads`, `VITA_THREADS`, or `auto`). Empty rows are
  omitted, long lists wrap at the value column, and a flag never wraps away from
  its value. Because it goes through the normal progress stream, `-l/--log`
  captures it in the same file and the same order as the diagnostics it explains.
  Every applet echoes its own stage. Pure reporting: nothing about the run changes
  and nothing is hashed into an artifact.

### Fixed

- **A flag's value inside a filelist is no longer treated as a source path.** The
  expander's list of value-taking flags had not been updated since the original
  five, so every flag added later had its value rewritten against the frame's base
  directory inside a `-F` filelist: `--top top` became `--top /abs/ip/top` and the
  run died with "top module not found", while `--hier-tree h.txt` silently wrote
  `ip/h.txt` instead of the path the caller named. Affected `--top`, `-L`,
  `--work`, `--workdir`, `--upstream`, `--obs-dir`, `--hier-tree`, `--inst-paths`,
  `--probe` and `--probe-file`. Source positionals still resolve, which is the
  whole point of `-F`.

- **A chained method call no longer ends the definite-assignment scan.** The
  expression walker had no arm for `s.substr(a, b).atoi()` (IEEE §8.13 chaining),
  so it answered "may reference" for *every* name — one chain anywhere in a block
  rejected the chain's own assignment target and every local declared after it. It
  also blamed the wrong file: a chain inside a callee body was reported against a
  local in the caller.
- **The definite-assignment walk no longer forgets that a local is already
  written.** Its catch-all rejected any unmodelled statement outright, even on a
  path where the local was definitely assigned. Once written this execution the
  per-entry reset it would have observed is already overwritten, and nothing
  un-assigns it. This is why a dummy write *before* a loop cleared the diagnostic
  while the same write *inside* the loop did not.
- **A timing-controlled first write is supported.** `#1 x = 7;`,
  `@(posedge clk) x = 7;`, `x = #1 7;`, `#1 begin x = 7; end` and
  `wait (c) x = 7;` are blocking writes — the process does not continue until the
  write has happened. A timing prefix that reads the local is still a genuine
  read-before-write and stays loud, and on a *shared* flattened net any
  time-advancing statement stays loud (the scheduler can hand the one net to the
  other block in between).
- **A local that is never written is accepted.** `automatic byte exp[];` passed
  to an `input` formal is not a read-before-write — there is no first write for the
  read to be before. With no writer the flattened static net holds the type default
  at every entry, which is exactly what `automatic` supplies. Proving "no writer"
  consults the callee resolver, so a task that pokes the flattened net through a
  hierarchical self-path still counts as one.
- **SILENT-WRONG: a method call was not seen as a read of its receiver.** The
  shared reference walker checked a call's arguments but not its path head, so
  `s.atoi()` did not count as reading `s`. The block-local scope-leak detector is
  built on that walker: a block-local referenced outside its block only through a
  method call went undetected, coalesced onto the outer binding's net, and the
  outside read returned the block's value (vita printed `1234` where iverilog
  prints `9999`).
- **SILENT-WRONG: `atoi`/`atohex`/`atooct`/`atobin` parsed like `strtol`.** They
  skipped leading whitespace, honored a sign, and stopped at an underscore. IEEE
  1800 §6.16.9 says the conversion "scans all leading digits and underscore
  characters (`_`) and stops as soon as it encounters any other character" — so
  `" 3".atoi()` is 0 (not 3), `"-7".atoi()` is 0 (not -7) and `"1_0".atoi()` is 10
  (not 1). Verified byte-identical to iverilog 13 over 17 inputs. The old code
  carried a comment asserting iverilog's stricter reading was "its bug".
- **SILENT-WRONG: a hierarchical reference could reach an `automatic` block-local.**
  IEEE 1800 §23.9 forbids naming an automatic variable hierarchically — it has no
  static address — but v1's flatten publishes one as a module net, so `tb.a = 99`
  from another module silently wrote per-entry storage. Now rejected on read,
  write and select-write, as iverilog rejects it.

### Added

- **`note:` telling you where the definite-assignment walk stopped.** E3009's
  lifetime message names two possible causes; the third — "the analyzer stopped
  here" — was the real one for most sites and was invisible, because only the
  declaration's location was printed. The note carries the construct's own
  file:line:col and one of six reasons.
- The block-local scope-leak diagnostic now has a location at all: the error
  points at the declaration, a note at the reference that makes it illegal. The
  deferred hierarchical read/write/select-write passes likewise carry the
  originating statement's span, so every diagnostic they raise is locatable.

## [2026-07-29 · round-16] 

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
