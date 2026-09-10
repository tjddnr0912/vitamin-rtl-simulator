# 000 · Introduction

This chapter states what vitamin is, who it serves, how its four commands relate, what each
pipeline stage decides, and which platforms it runs on. The supported language surface appears
here at chapter granularity; the construct-by-construct detail is in
[Language Reference](003_language-reference.md).

---

## What vitamin is

vitamin is an open-source RTL simulator written in Rust and distributed as source. It reads
Verilog and SystemVerilog, elaborates the design, runs an event-driven 4-state kernel over it,
writes the `$display` transcript to stdout, and — when the RTL asks for one — writes a VCD or FST
waveform that opens in GTKWave or Surfer.

The command-line surface is deliberately familiar to anyone who has used Icarus Verilog or a
commercial flow: `-D`/`-I` for the preprocessor, `-f` filelists, `-G` parameter overrides,
`-o` for the output path, and a one-shot driver alongside a compile / elaborate / simulate split.

vitamin belongs to the same class of tool as Icarus Verilog: event-driven, 4-state, with a
faithful event queue. A compiled 2-state simulator such as Verilator, and the commercial
simulators, are one to two orders of magnitude faster; that difference is structural, because
4-state values and event ordering are exactly what vitamin keeps. Against Icarus Verilog on the
ten-design workload corpus — `corpus-runner run --compare`, release binaries, median of three
runs — vitamin's geometric mean is 1.74× faster over the nine timed designs, two of which
vitamin loses.

## Who it is for

| Reader | What vitamin offers |
|---|---|
| **RTL designer** | A simulator that installs from source with one `cargo` command, runs a synthesizable design end to end, and produces a standard waveform |
| **Verification engineer** | Assertions, functional coverage, classes, and constrained random, with every unsupported construct refused by a coded diagnostic rather than approximated |
| **CI owner** | The same source produces byte-identical stdout and byte-identical waveform bytes on Linux and macOS, so a diff means a design change |
| **Tooling and agents** | A machine-readable run report, a value-change trace, and a hierarchy dump, all written on request to a directory ([`--obs-dir`](004_cli-reference.md)) |

---

## The four commands

vitamin ships one executable, `vita`. It behaves as four tools: the one-shot driver, and the
three staged tools that split the same pipeline.

| Command | Stage | Consumes | Produces |
|---|---|---|---|
| `vita` | the whole pipeline in one invocation | `.v` / `.sv` sources | RTL stdout, waveform |
| `vcmp` | preprocess, lex, parse | `.v` / `.sv` sources | `.vu` compile snapshot |
| `velab` | elaborate | one `.vu`, or work libraries selected with `-L` | `.velab` elaborated artifact |
| `vrun` | simulate | one `.velab` | RTL stdout, waveform |

The staged split mirrors the compile / elaborate / simulate stages of commercial EDA flows
(`xmvlog`/`xmelab`/`xmsim`, `vlogan`/`vcs`/`simv`). It lets one stage be rebuilt and inspected on
its own, and lets an unchanged stage be skipped by reusing the artifact on disk.

### Dispatch: argv0 and the subcommand token

`vita` is a multicall binary. Which applet runs is decided from the invocation, in this order:

1. The file stem of `argv[0]` is taken — the stem, so an extension is discarded, while a
   decorated name such as `vita-0.2` matches nothing. If the stem is `vcmp`, `velab` or `vrun`,
   that stage runs and every remaining argument belongs to it.
2. Otherwise, if the first argument is exactly `vcmp`, `velab` or `vrun`, that token is
   consumed and selects the stage.
3. Otherwise the one-shot `vita` pipeline runs.

So a stage is reachable either through a link named after it or through the subcommand form, and
the two are the same code path:

```sh
# one-shot
vita design.sv

# staged, subcommand form — no links needed
vita vcmp  design.sv      # -> design.vu
vita velab design.vu      # -> design.velab
vita vrun  design.velab   # -> waveform + stdout

# staged, through links named vcmp / velab / vrun
vcmp design.sv && velab design.vu && vrun design.velab
```

The subcommand token has to be first; a `vita` invocation that names it later runs the one-shot
path and treats the token as a positional argument.

A `.velab` records the digest of everything upstream of it. Running `vrun` on an artifact whose
sources have changed is refused with exit code 2 and a message naming the rebuild, rather than
replaying a stale design.

---

## The pipeline

The one-shot and staged flows run the same stages in the same order. Every stage validates as it
goes; there is no separate checking pass.

```
sources (.v / .sv)
  → preprocess   → lex → parse        [vcmp writes .vu here]
  → elaborate                         [velab writes .velab here]
  → sim-ir
  → simulate                          [vrun starts here]
  → waveform (VCD or FST) + stdout
```

| Stage | What it decides | What it reports |
|---|---|---|
| **preprocess** | Macro definition and expansion, conditional compilation, `` `include `` resolution against the including file's directory then the `-I` list, the `` `timescale `` region table, and a byte-offset source map so later diagnostics point back into the original file | Undefined macro use, macro arity, unbalanced conditionals, a precision coarser than its unit, and the warning raised when a design carries no `` `timescale `` at all |
| **lex** | The token stream; attribute instances `(* … *)` are removed here | Unterminated literals, comments and attributes |
| **parse** | The AST — the last representation that knows the source language. `vcmp` serializes it as `.vu` | Syntax errors, and constructs the front end does not accept |
| **elaborate** | Parameter values and overrides, `generate` unrolling, the instance tree, port and type checking, implicit nets, multiple-driver resolution, and the lowering into `sim-ir`. `velab` serializes the result as `.velab` | Port and type mismatches, unresolvable hierarchical names, unsupported net kinds, and every construct refused above the parser |
| **sim-ir** | Nothing: it is the frozen, language-neutral contract between the front end and the engine, gated by a structural schema hash | — |
| **simulate** | Event ordering on the time wheel, expression evaluation in 4-state, process scheduling, system-task execution, and the reason the run ended | Runtime diagnostics, `$fatal`/`$error`, delta-limit non-convergence, and the closing `simulation ended (…) at time N` line |
| **waveform** | Value-change records for the nets the dump filter selected. An output path ending in `.fst` is transcoded from a sidecar VCD when the run finalizes | A failed open or a failed transcode is a warning; the run still completes |

A waveform appears only when the RTL calls `$dumpvars`. `$dumpfile` on its own records a pending
path and creates no file, and `$dumpon`/`$dumpoff`/`$dumpall` before the first `$dumpvars` do
nothing. The resolved path is `-o` if given, else the `$dumpfile` argument, else `dump.vcd`; the
format follows the extension, `.fst` for FST and anything else for VCD.

---

## Design philosophy

### Determinism

The same sources produce the same bytes, on every supported host and on every run.

| Source of variance | How it is removed |
|---|---|
| Wall clock in the waveform | The VCD `$date` field is the fixed string `vitamin-sim`; no clock is read |
| Serialization drift | One encoder for every artifact — serde plus postcard — with blake3 digests |
| Platform-dependent layout | Frozen IR types are BTree-ordered, span-free, and carry no `usize`, `isize`, `f32` or `f64` |
| Floating-point libraries | Transcendentals come from a vendored pure-Rust libm built without hardware intrinsics, so `f64` results are bit-identical on every IEEE-754 target |
| Thread count | `--threads` moves waveform writing to its own thread and changes wall clock only; the bytes are identical for every value |
| Executor choice | The `native`, `vm` and `interp` executors are required to produce identical stdout and identical waveform bytes; a test suite compares them design by design |

### Correct, or loud

Accuracy is a ladder: a silently wrong answer is worse than a refusal, and a refusal is worse
than correct support. Movement is only ever upward.

- A construct outside the supported set is refused with a source location and a stable
  diagnostic code, never approximated. `vita explain <CODE>` prints the entry for one code.
- Where Icarus Verilog accepts a construct, its behaviour is the differential oracle: a test
  suite runs `iverilog`/`vvp` live and compares the transcript.
- Where Icarus Verilog rejects the construct — assertions, classes, constrained random,
  parameterized classes, virtual interfaces — the expected value is derived from the IEEE 1364
  and 1800 text and pinned as a literal in the test, with the reason recorded beside it.
- Where an argument and a measurement disagree, the measurement decides.

### A source build with cargo and nothing else

`cargo` is the entire build. No vita crate has a `build.rs`, and there is no cmake, no make, and
no shell-out step. The workspace holds 17 member crates; the single vendored third-party
dependency, `third_party/libm`, is excluded from the workspace so a workspace-wide lint does not
reach it.

Artifacts inherit the same discipline. A `.velab` carries a format version, the tool's major
version, and a structural schema hash of the IR shape; any mismatch is a refusal with exit code
2 telling you to regenerate, so a cached artifact can be reused safely.

---

## What the language surface covers

The table below is the chapter-level shape of the supported subset. Each row is expanded, with
the exact refusals, in [Language Reference](003_language-reference.md); constructs that stay
loud are catalogued in [Limitations](006_limitations.md).

| Area | Covered |
|---|---|
| **Design units** | Modules with ANSI and non-ANSI ports, `parameter`/`localparam` and overrides, `generate`/`genvar`, packages and `import`, interfaces with modports and virtual interfaces, programs, classes, clocking blocks, `bind`, user-defined primitives, compilation-unit scope declarations, and work libraries |
| **Preprocessor** | `` `define `` object-like and function-like with default arguments, `` `undef ``, `` `include ``, `` `ifdef ``/`` `ifndef ``/`` `elsif ``/`` `else ``/`` `endif ``, `` `timescale ``, `` `default_nettype ``, `` `__FILE__ ``, `` `__LINE__ `` |
| **Data types** | `wire`/`tri`/`uwire`/`wand`/`wor`, `reg`, `logic`, `integer`, `time`, `event`, `real`/`realtime`, the 2-state atoms `bit`/`byte`/`shortint`/`int`/`longint`, `string`, packed and multi-dimensional unpacked arrays, dynamic arrays, queues, associative arrays, `enum`, `typedef`, packed `struct`, and scalar unpacked structs |
| **Procedural blocks** | `initial`, `final`, `always`, `always_ff`, `always_comb`, `always_latch`, `fork`/`join`/`join_any`/`join_none`, and `disable` |
| **Statements** | Blocking and non-blocking assignment, `if`, `case`/`casez`/`casex` with `unique`/`unique0`/`priority`/`priority0`, `for`/`while`/`repeat`/`forever`/`do…while`/`foreach`, `break`/`continue`/`return`, increment/decrement and compound assignment, `#delay`, `@(event)`, `wait`, `assign`/`deassign`, and `force`/`release` |
| **Expressions** | The full Verilog operator set including reductions, concatenation and replication, part-selects and indexed part-selects, `inside`, casts, streaming operators, and assignment patterns |
| **Functions and tasks** | Static and automatic lifetime, recursion, `input`/`output`/`inout`/`ref` formals, default arguments, and hierarchical calls |
| **Timing** | Per-module `` `timescale `` unit and precision, a global precision tick base taken as the finest precision in the design, and two-stage delay conversion |
| **Verification** | Immediate and deferred assertions, concurrent SVA sequences and properties, `cover property`, covergroups with coverpoints, bins and crosses, classes with single inheritance and virtual dispatch, and constrained random with `rand`/`randc`, constraint blocks, `randomize() with`, and `dist` |
| **System tasks** | The display and write family with full format-specifier support, the severity tasks, simulation control, assertion control, time, conversion, bit-vector queries, the IEEE real-math set, random and distribution functions, plusargs, file I/O, memory load and dump, introspection, and the waveform dump family |

Values are 4-state throughout: `0`, `1`, `x`, `z`.

### Executors

Process bodies run on `native`, the default executor: net values live in a flat arena and
uniform-width expressions run on a specialised evaluator. A build with default features also
carries two alternative executors — `interp`, a tree-walking reference implementation, and `vm`,
a bytecode virtual machine — selectable with `--backend` so a suspected defect can be bisected
against a second implementation of the same semantics. All three are required to print identical
bytes, so the flag moves wall clock only. A build made with `--no-default-features` carries
`native` alone, and then a request for `interp` or `vm` is refused rather than quietly
downgraded. See [CLI Reference](004_cli-reference.md).

### Out of scope

Synthesis, a waveform GUI, FSDB and other vendor wave formats, UVM, UPF, SDF back-annotation,
and DPI-C.

---

## Supported platforms

| Item | Value |
|---|---|
| Operating systems | Linux and macOS |
| Targets | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| Windows | Not supported: not a build target in `rust-toolchain.toml`, and not exercised in CI |
| Rust toolchain | 1.85.0, pinned by `rust-toolchain.toml`; vitamin's own crates are edition 2021 |
| Version | 0.2.0 across all workspace crates |
| Licence | MIT OR Apache-2.0, at your option |

CI runs the full suite on `ubuntu-latest`, on `macos-latest`, and inside a `redhat/ubi9`
container.

---

## Where to go next

- [Installation](001_installation.md) — prerequisites, building from source, and what gets
  installed where.
- [Quickstart](002_quickstart.md) — one worked example from source file to waveform.
- [Language Reference](003_language-reference.md) — the supported subset, construct by
  construct.
- [CLI Reference](004_cli-reference.md) — every command and flag, the staged flow, and the
  observability rail.
- [System Tasks](005_system-tasks.md) — `$display`, `$dumpvars`, file I/O, and the rest.
- [Limitations](006_limitations.md) — what is refused, and how the refusal is spelled.
- [Error Codes](007_error-codes.md) — the diagnostic-code reference.
