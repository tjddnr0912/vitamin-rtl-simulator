# vitamin

vitamin (`vita`) is an open-source RTL simulator written in Rust. It compiles,
elaborates and simulates the Verilog-2005 synthesizable subset plus a large
SystemVerilog subset, writing a `$display` transcript to stdout and a
hierarchical VCD or FST waveform to disk.

```text
preprocess → lex → parse → elaborate → sim-ir → sim-engine → VCD/FST
```

The build is cargo-only: no vita crate has a `build.rs`, and there is no cmake
or make step. Two properties shape everything else in the tool. Output is
deterministic — the same input yields byte-identical stdout, waveform and exit
code on Linux and macOS, at any thread count. Behaviour is correct-or-loud —
see [The correct-or-loud contract](#the-correct-or-loud-contract).

## Quick start

Building needs a Rust toolchain and a C linker. `rust-toolchain.toml` pins
1.85.0 and rustup picks it up automatically.

```sh
cargo build --release --workspace --locked      # builds target/release/vita
./target/release/vita examples/000_counter.sv   # run a bundled design
```

The transcript goes to stdout and ends with a summary line and exit code 0:

```text
t=16  cnt=1 (0x1)
...
done: final cnt=12
simulation ended (Finish) at time 126
errors=0 warnings=0 notes=0
```

The waveform lands in the current working directory under the name the design
gives `$dumpfile` — `counter.vcd` for this example. Open it in
[GTKWave](https://gtkwave.sourceforge.net/) or
[Surfer](https://surfer-project.org/); vitamin ships no GUI of its own.

Waveform format follows the file extension, case-insensitively: a path ending in
`.fst` writes FST, anything else writes VCD. That applies both to the design's
`$dumpfile` argument and to a `-o` override on the command line.

```sh
./target/release/vita examples/000_counter.sv -o waves.fst
```

A design that never calls `$dumpvars` writes no waveform at all, even with `-o`.
That is not an error.

To put the tools on `PATH`, run [`install.sh`](install.sh), which does
`cargo install --path crates/cli --locked` and then links the three staged names
next to the installed binary. [Installation](docs/manual/001_installation.md)
covers the alternatives.

## The four commands

`vita` is a single multicall binary. `vcmp`, `velab` and `vrun` are the same
binary dispatched on the `argv[0]` stem, and each is also reachable as
`vita vcmp …`, `vita velab …`, `vita vrun …`.

| Command | Role | Input | Output |
|---|---|---|---|
| `vita` | one-shot: compile, elaborate and simulate in one invocation | `.sv` / `.v` sources | transcript + waveform |
| `vcmp` | compile — preprocess, lex, parse | sources | `<first-source>.vu` |
| `velab` | elaborate — parameters, generate, instances, lowering | one `.vu` | `<input>.velab` |
| `vrun` | simulate | one `.velab` | transcript + waveform |

The staged flow pays the front end once and re-runs only the simulation:

```sh
vcmp  design.sv          # → design.vu
velab design.vu          # → design.velab
vrun  design.velab       # → transcript + waveform
```

`.vu` and `.velab` carry a structural schema hash. A stale artifact is rejected
at the header gate with exit 2 and a message naming the stage to re-run, never
decoded on a best-effort basis.

`vita explain <CODE>` prints the reference entry for any diagnostic, accepting
the mnemonic (`W-ELAB-FEATURE-LIMIT`), the printed number (`VITA-W3056`) or the
number without the vendor prefix (`W3056`).

Once an invocation lives inside a Makefile, `-v` prints the resolved invocation
at the top of the transcript — every macro value, the files actually compiled
after filelist expansion, the runtime plusargs, and where the thread count came
from — so a `-l/--log` file answers "what did this run use?" on its own. The
[CLI reference](docs/manual/004_cli-reference.md) documents the echo's format.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | clean run; also `--help`, `--version`, `explain`, `--dump-filelist` |
| `1` | design or user error: parse error, elaboration failure, runtime `$fatal`, a `-Werror`-promoted warning |
| `2` | artifact staleness — regenerate the `.vu` / `.velab`; the design is not at fault |
| `3` | usage error: unknown flag, missing value, wrong positional count, unreadable input |
| `101` | panic (deliberately outside the vita exit classes) |
| `141` | terminated by SIGPIPE, as when stdout is closed by `head` |

## What it supports

| Area | Coverage |
|---|---|
| Verilog-2005 synthesizable RTL | modules, ports, parameters and overrides, `generate`/`genvar`, nets and variables, packed and multi-dimensional unpacked arrays, continuous assigns, `initial`/`always`, blocking and nonblocking assignment, `case`/`casex`/`casez`, tasks and functions, `` `timescale `` and delays, named events, `force`/`release`, `disable` |
| Gate level | the built-in gate primitives, plus `primitive … endprimitive` UDPs — combinational and sequential state tables |
| SystemVerilog types | `logic`, `bit`/`byte`/`shortint`/`int`/`longint`, `typedef`, `enum`, packed `struct` and `union`, `string`, `real`/`realtime` |
| SystemVerilog procedural | `always_ff`/`always_comb`/`always_latch`, `fork`/`join`/`join_any`/`join_none`, `foreach`, `do`/`while`, `unique`/`priority`/`unique0`/`priority0`, `final` |
| Structure | packages and `import`, `::` scope resolution, compilation units, work libraries, interfaces and modports, `program`, hierarchical references, automatic and recursive subroutines |
| Dynamic storage | dynamic arrays, queues, associative arrays, and the array and string method families |
| Object-oriented | classes with inheritance and virtual dispatch, parameterized classes, virtual interfaces |
| Verification | immediate and deferred assertions, SVA sequences, property operators, `cover property`, functional coverage (`covergroup`/`coverpoint`/cross), constrained random (`rand`, `randc`, constraint blocks, `randomize() with`, `dist`) |
| System tasks | the display, file-I/O, memory-load, simulation-control, time, conversion, bit-vector, introspection, sampling and `$dump*` families, 21 real-math functions and the `$dist_*` generators |
| Waveforms | VCD and FST, one-shot or staged, plus VCD-to-FST transcode |

Per-construct detail is in the
[language reference](docs/manual/003_language-reference.md); the
[system-task chapter](docs/manual/005_system-tasks.md) lists every `$` name.

Constructs outside the subset are refused with a diagnostic that names the
construct and the reason. The deliberate non-goals are DPI-C, `shortreal`,
`trireg`, implicit nets, UPF and SDF, the UVM ecosystem, synthesis, and a
waveform GUI. Everything that is simplified rather than refused, and every
place where the reference simulators disagree with each other, is listed in
[Limitations](docs/manual/006_limitations.md).

## The correct-or-loud contract

A run either produces the IEEE-correct answer or says out loud that it cannot: a
construct outside the supported subset is a diagnostic with a code, never a
plausible-looking wrong value at exit 0. Where a silent divergence is found it
is treated as the highest-priority defect class, and the open ones are enumerated
in [Limitations](docs/manual/006_limitations.md) with a workaround for each,
rather than left for a user to discover.

Behaviour is held to that standard by differential review against Icarus Verilog
(`iverilog` + `vvp`), by a second opinion from Verilator on 2-state arithmetic,
and — for the areas neither tool accepts, such as SVA, classes, constrained
random, parameterized classes and virtual interfaces — by expected values derived
from the IEEE 1364/1800 text and pinned into the tests with the reason recorded.

## Determinism

| Property | How it is held |
|---|---|
| Cross-platform byte identity | the serialized IR forbids `usize`/`isize`/`f32`/`f64`, uses `BTree` collections only, and carries no source spans; one encoder (serde + postcard) and one digest (blake3, pinned) |
| Thread invariance | `--threads N` moves waveform writing to a dedicated writer thread and nothing else; stdout, waveform and exit code are byte-identical for every N |
| Backend invariance | process bodies run on the compiled `native` executor by default, with `--backend interp` and `--backend vm` available as bisection oracles. All three must produce byte-identical output, and a test suite asserts it |
| Float identity | the transcendental and `$dist_*` math uses a vendored pure-Rust `libm` built without hardware intrinsics, so `f64` results are bit-identical on every IEEE-754 target |
| Artifact staleness | `.vu` and `.velab` carry a structural schema hash and a format version; a mismatch is exit 2, never a silent reinterpretation |

## Performance

vitamin is an event-driven, 4-state simulator — the same class of tool as Icarus
Verilog. Against a compiled, 2-state simulator such as Verilator, or against a
commercial simulator, it is one to two orders of magnitude slower. That gap is
structural, not a tuning defect: Verilator buys its speed by giving up 4-state
semantics and event ordering, which vitamin keeps. The axis is analysed in
[study 01](docs/study/01-interpreted-vs-compiled.md).

Keccak-f[1600], 2000 permutations, macOS arm64, release builds, interleaved
samples with the first round discarded:

| Simulator | Per permutation | Relative |
|---|---:|---:|
| Verilator 5.050 (compiled, 2-state) | 7.0 µs | 1× |
| vitamin, design with no subroutine calls | 295 µs | 42× slower |
| vitamin, same algorithm with function/task calls | 2 035 µs | 291× slower |
| Icarus Verilog 13 | 4 470 µs | 639× slower |

Against Icarus Verilog, vitamin is ahead on most designs: a geometric mean of
1.93× faster across the seven third-party designs in the workload corpus and
1.74× across all nine timed rows, measured by `corpus-runner run --compare` at
the median of three ([study 03](docs/study/03-workload-corpus.md)). Two rows are
losses. Subroutine calls are the largest cost a design can pay — the same
algorithm written without them runs 6.9× faster than the version that calls them.

Elaboration is a rounding error on these workloads: every corpus row is at least
99% simulation time.

### Where it fits in a flow

Running a full regression suite under vitamin is impractical. It pays off as the
quick-check tool in the loop, where determinism and correct-or-loud matter most:

- Pick representative cases — one directed test per feature, a short randomized
  burst, a smoke test of a few hundred cycles — rather than the whole suite.
- Keep the cycle count to what the check needs; simulated time dominates, so a
  10× shorter run really is 10× cheaper.
- Use the staged flow when re-running one design repeatedly: compile and
  elaborate once, then pay only for `vrun`.
- Dump waveforms only when they will be read, and prefer FST for large runs.
- Reach for Verilator or a commercial simulator for soak runs, large randomized
  regressions and full-chip workloads.

## Machine-readable run data

One-shot `vita` can publish a structured record of the run alongside the usual
output, for tooling and agents that consume results rather than read them.

| Flag | Effect |
|---|---|
| `--obs-dir <D>` | writes `run.json` (the run manifest) and `results.jsonl` (one result record), plus `coverage.json` when the design has covergroups. Every other flag here requires it |
| `--probe <path>` / `--probe-file <f>` | records every value change of a hierarchical net to `trace.jsonl`; an unresolved path is a loud error, never a silent skip |
| `--obs-procs` | adds the per-process, builtin and subroutine-call census to `run.json` |
| `--obs-procs-time` | the same, plus wall-clock time per process. This is the one flag whose output is not deterministic, and it perturbs the run it measures |
| `--hier-tree <f>` / `--inst-paths <f>` | writes the elaborated instance tree, or the flat dotted instance-path list |

The schemas are specified in
[preview/19](docs/preview/19-ai-agent-observability.md).

## Project facts

| | |
|---|---|
| Version | 0.2.0. The major stays at 0 because it is a hard artifact-staleness key: bumping it invalidates every `.vu` and `.velab` in existence |
| Toolchain | rustc/cargo 1.85.0 is the floor, pinned in `rust-toolchain.toml`; vita's own crates are edition 2021. `--locked` is required |
| Workspace | 17 member crates, plus a vendored `third_party/libm` held outside the workspace |
| Platforms | Linux and macOS. CI builds and tests on ubuntu-latest, macos-latest and a RHEL 9 / UBI 9 container. Windows is not a target |
| Tests | `cargo nextest run --workspace --locked` runs 7811 tests, all passing, with 15 skipped (they are `#[ignore]`d performance probes, not gates), in about 36 s. CI runs `cargo test --workspace --locked` |
| Artifact format version | 31 |
| Diagnostics | 70 codes, each with a mnemonic, a `VITA-####` number and a reference entry |
| Licence | MIT or Apache-2.0, at your option |

## Where to read next

The user manual is in [`docs/manual/`](docs/manual/), in reading order:

| # | Chapter | Answers |
|---|---|---|
| 0 | [Introduction](docs/manual/000_introduction.md) | what vitamin is and who it is for |
| 1 | [Installation](docs/manual/001_installation.md) | building and installing the binaries |
| 2 | [Quick Start](docs/manual/002_quickstart.md) | a first simulation, end to end |
| 3 | [Language Reference](docs/manual/003_language-reference.md) | which constructs are supported |
| 4 | [CLI Reference](docs/manual/004_cli-reference.md) | every flag of all four commands |
| 5 | [System Tasks](docs/manual/005_system-tasks.md) | every supported `$` task and function |
| 6 | [Limitations](docs/manual/006_limitations.md) | every simplification and divergence |
| 7 | [Error Codes](docs/manual/007_error-codes.md) | reading and looking up a diagnostic |

Beyond the manual:

- [`examples/`](examples/) — four runnable designs: a counter, an ALU, an
  `enum`-based FSM and a shift register.
- [`CHANGELOG.md`](CHANGELOG.md) — what changed between releases.
- [`docs/README.md`](docs/README.md) — the map of the rest of the documentation.
- [`docs/preview/`](docs/preview/) — the design specifications.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) and
  [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md) — the open engineering items.
- [`bench/README.md`](bench/README.md) — the workload corpus and its runner.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — toolchain, gates and workflow.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
