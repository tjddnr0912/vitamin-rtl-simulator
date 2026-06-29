# Introduction

**vitamin** is an open-source RTL simulator written in Rust. You feed it
Verilog / SystemVerilog RTL, it runs the design, and — when your RTL asks for it —
writes a VCD waveform you can open in GTKWave, Surfer, or any standard viewer.

If you already write RTL and have used a simulator like Icarus Verilog or VCS,
vitamin will feel familiar: it compiles, elaborates, and simulates your design,
and emits the same waveform and `$display` output you expect. What is different
is *how* it is built — for **determinism and reproducibility** rather than for
the broadest possible language coverage.

## What problem it solves

Most RTL simulators are either commercial (VCS, Xcelium — closed, license-gated)
or distributed as prebuilt binaries whose results can subtly differ across hosts.
vitamin targets three things instead:

- **Deterministic, byte-reproducible results.** The same source builds and runs
  to the *same* result on Linux and macOS. Run it on your laptop, run it in CI,
  get identical output.
- **Source-only builds.** No prebuilt binaries, minimal/zero C dependencies.
  `cargo build` is the whole story.
- **Timing precision without a GC.** An event-driven core with a faithful
  `timescale` model, written in Rust so that deterministic timing accuracy does
  not fight a garbage collector.

vitamin uses Icarus Verilog as its **golden reference** for differential
verification: signal values and transition times are checked to match `iverilog`.

## The four CLIs

vitamin ships as a single multicall binary, `vita`, that behaves as four tools
depending on how it is invoked. You can run the whole flow in one shot, or split
it into stages.

| Command | Role | Consumes | Produces |
|---------|------|----------|----------|
| `vita`  | one-shot: compile → elaborate → simulate | `.sv` / `.v` source | VCD (+ stdout) |
| `vcmp`  | compile (preprocess + lex + parse)       | source              | `.vu` |
| `velab` | elaborate                                | `.vu`               | `.velab` |
| `vrun`  | simulate                                 | `.velab`            | VCD (+ stdout) |

The staged tools mirror the compile / elaborate / simulate split of commercial
EDA flows (Cadence `xmvlog`/`xmelab`/`xmsim`, Synopsys `vlogan`/`vcs`/`simv`).
They let you rebuild and debug one stage at a time and **skip stages whose inputs
have not changed** by reusing the on-disk artifact.

The staged applets are dispatched by the program's basename, so they can be
installed as symlinks to `vita`, or invoked explicitly as subcommands:

```sh
# one-shot
vita design.sv

# staged — explicit subcommand form (no symlinks needed)
vita vcmp  design.sv      # -> design.vu
vita velab design.vu      # -> design.velab
vita vrun  design.velab   # -> design.vcd (if the RTL calls $dumpvars)

# staged — via symlinks named vcmp / velab / vrun
vcmp design.sv && velab design.vu && vrun design.velab
```

A `.velab` is gated against the exact front-end shape it was elaborated from: if
an upstream source changes, running `vrun` on a stale artifact is **refused**
(by content hash, not mtime) rather than silently producing wrong results.

## The pipeline

Both the one-shot and staged flows run the same stages in the same order:

```
HDL source
  → preprocess   (`define / `ifdef / `include / `timescale)
  → lex          (token stream)
  → parse        (AST, syntax checks)
  → elaborate    (params, hierarchy, type/port checks, multiple-driver checks)
  → sim-ir       (language-neutral intermediate representation)
  → sim-engine   (event-driven IEEE-1364 kernel, timescale time model)
  → VCD          (emitted only when the RTL calls $dumpfile / $dumpvars / …)
```

Checking is not a separate pass — each stage validates as it goes. Syntax errors
surface at **parse**; connectivity, type/port, and multiple-driver errors surface
at **elaboration**, each reported with a source location and a stable error code.

Note the last line: **VCD is not dumped automatically.** A waveform is written
only when your RTL explicitly calls a dump system task (`$dumpfile`, `$dumpvars`,
`$dumpon`/`$dumpoff`/`$dumpall`). This matches Icarus/VCS behaviour and keeps the
RTL in control of what gets recorded.

## Design philosophy

- **Deterministic and reproducible across OSes.** The same source produces the
  same VCD bytes on Linux and macOS. Determinism is enforced structurally — a
  schema hash gates artifact staleness, and the IR avoids platform-dependent
  encodings — not by convention.
- **Frozen IR.** `sim-ir` is the golden contract between the front end and the
  engine. Its serialized shape is frozen behind a schema hash; changing it is a
  deliberate, versioned act that invalidates old artifacts. This is what makes
  cached `.velab` reuse safe.
- **Language-dependent until parse, neutral after.** Everything up to and
  including `parse` knows about Verilog/SystemVerilog. From `sim-ir` onward the
  representation is language-neutral. That boundary keeps the engine simple, and
  leaves room to add more source languages — or a compiled/JIT backend — later
  without rewriting the simulator core.

## Phase-1 scope

Phase 1 (the current MVP) implements the **synthesizable SystemVerilog RTL
subset**, which includes **all of Verilog-2005 RTL**: modules, ports,
`parameter`/`localparam`, `generate`/`genvar`; `wire`/`reg`/`logic`/`integer`,
packed and (multi-dimensional) unpacked arrays; `initial`/`always` and
`always_ff`/`always_comb`/`always_latch`; blocking/non-blocking assignment,
`if`/`case`/`casez`/`casex`, the loop family, and `fork`/`join`; functions and
tasks; the `#delay`/`@(event)`/`wait` timing constructs and `assign`; plus the
SystemVerilog data types **`enum`, `typedef`, and packed `struct`**. The core
system tasks for display/output, time, simulation control, and VCD dump are
supported. The backend is a deterministic IR-walking interpreter; a compiled
backend is reserved for a later phase behind the `sim-ir` boundary. Values are
4-state (`0`/`1`/`x`/`z`).

Out of scope for now: synthesis itself, a waveform GUI, non-VCD wave formats
(FST), and advanced verification features (UPF, SDF back-annotation, DPI-C,
coverage, UVM).

## Supported platforms

vitamin runs on **Linux and macOS** (Apple Silicon and Intel). It builds from
source with `cargo` against a pinned toolchain (MSRV **1.82**, edition 2021); use
`--locked` for reproducible builds. **Windows is not supported.**

## Where to go next

- [Installation](001_installation.md) — building from source with `cargo`.
- [Quick Start](002_quickstart.md) — your first one-shot and staged runs.
- [Language Reference](003_language-reference.md) — the supported RTL subset in detail.
- [CLI Reference](004_cli-reference.md) — `vita` / `vcmp` / `velab` / `vrun` usage.
- [System Tasks](005_system-tasks.md) — `$display`, `$dumpvars`, and friends.
- [Limitations](006_limitations.md) — known v1 simplifications.
- [Error Codes](007_error-codes.md) — the diagnostic-code reference.
