# 00 · Overview

vitamin is an open-source RTL simulator written in Rust; its command-line tool is `vita`. This
document states what the tool is, the three design goals and what each one commits the
implementation to, the stages of the pipeline and what each stage decides, the two ways a design is
run, and where vitamin sits among the reference simulators. The detailed contracts are in the
numbered documents that follow.

## What vitamin is

| | |
|---|---|
| Project / command | vitamin / `vita` |
| Staged commands | `vcmp` (compile), `velab` (elaborate), `vrun` (simulate) — links onto the same multicall binary, also reachable as `vita vcmp …` |
| Simulator class | batch, event-driven, 4-state (`0`/`1`/`x`/`z`) |
| Input language | the SystemVerilog (IEEE 1800) subset defined in [01](01-goals-and-scope.md), which contains Verilog-2005 RTL in full |
| Platforms | Linux and macOS, x86_64 and aarch64 |
| Build | `cargo build --workspace --locked`; no `build.rs` in any vitamin crate, no C or C++ dependency |
| Licence | `MIT OR Apache-2.0` |
| Version | 0.2.0; artifact `format_version` 31 |

## The three design goals

Each goal is a constraint on the implementation, not an aspiration. The third column is what the
goal forbids, which is the part that decides arguments.

| Goal | What it commits the implementation to | What it forbids |
|---|---|---|
| Precision | Simulated time is a 64-bit integer count of precision ticks; a `#delay` converts through the declaring module's timescale under the two-stage rounding rule of [08](08-timescale-and-timing.md). Values are 4-state throughout, and `z` folds to `x` in every operator except `===`/`!==`. Seven IEEE 1800 scheduling regions are modelled — Preponed, Active, Inactive, NBA, Observed, Reactive, Postponed ([06](06-simulation-engine.md)) | A floating-point time axis. A construct that could observe a region the engine does not model: it is refused with a diagnostic instead. Any approximation that produces a plausible wrong value — the accuracy ladder runs silent-wrong ≪ loud ≪ correct-support, and movement is upward only |
| Portability | One source tree builds with `cargo` alone on every supported target. The same design produces byte-identical stdout and byte-identical waveform bytes on every supported OS: no hash-map iteration order reaches an execution decision, frozen IR types hold no `usize`/`isize`/`f32`/`f64` and use `BTree` collections only, and the real-math functions come from a vendored pure-Rust libm compiled without hardware intrinsics ([02](02-implementation-language.md), [16](16-schema-hash-spec.md)) | A dependency that needs a C or C++ toolchain. Distribution of prebuilt binaries as the supported install path. Any output that varies with allocation addresses, thread scheduling or host ISA |
| Performance | No garbage collector anywhere in the run loop. The default executor `native` holds net values in one flat arena and evaluates uniform-width expressions on a specialised evaluator ([21](21-tier3-native-backend.md)) | Buying speed with semantics. Every executor must print the same bytes, and that equivalence is a gate ([09](09-testing-and-verification.md)); a second spelling of a semantic rule inside a fast path is a defect, not an optimisation. A performance claim measured on a debug binary or on a non-interleaved A/B ([study/01](../study/01-interpreted-vs-compiled.md)) |

## The pipeline

```
HDL source
  → preprocess   (`define / `ifdef / `include / `timescale)
  → lex          (token stream)
  → parse        (AST, grammar checking)
  → elaborate    (parameters, hierarchy, types and ports, multiple drivers → sim-ir + sidecars)
  → sim-engine   (event-driven 4-state kernel, timescale time model)
  → waveform     (VCD, or FST by transcode — only when the RTL calls the dump tasks)
```

| Stage | Crate | What it decides |
|---|---|---|
| preprocess | `hdl-preprocess` | macro expansion, conditional inclusion, include resolution, the `` `timescale `` in force, and the source map every later diagnostic locates against |
| lex | `hdl-lexer` | the token stream, and which words are keywords rather than identifiers |
| parse | `hdl-parser` | grammar, the AST, and the parser-level desugars that add no AST node: packed-struct member selects, `foreach`, `do`-`while`, statement labels, type parameters, parameterized-class monomorphization |
| elaborate | `elaborate` | parameter resolution, generate unrolling, the instance hierarchy, port and type matching, multiple-driver detection, name resolution, and the lowering to `sim-ir` plus the out-of-band sidecar tables |
| sim-ir | `sim-ir` | the frozen IR itself. `sim_ir::SimIr` is the golden root whose structural hash gates artifact staleness ([17](17-sim-ir-ir-backbone-freeze.md)) |
| sim-engine | `sim-engine` | execution: region ordering, delta cycles, the time wheel, net writes, `$` task effects |
| waveform | `vcd-writer` | VCD bytes, and the VCD→FST transcode ([07](07-vcd-format.md)) |

Checking is not a separate stage. A grammar error is a parse error, a connectivity, type or
multiple-driver error is an elaboration error, and an out-of-range select or a non-convergent
timestep is a runtime diagnostic. Every diagnostic carries a stable `MsgCode`
([13](13-diagnostics-and-logging.md), [15](15-error-code-reference.md)).

Language dependence ends at parse: from `sim-ir` onward nothing in the pipeline knows which HDL the
design was written in. Status at HEAD: SystemVerilog is the only front end that exists, so the
language-neutral boundary is a design property rather than a shipped second language
([05](05-strategy-and-roadmap.md)).

## Waveform output

Waveform emission is driven by the RTL, never by the tool alone. `$dumpfile` records a pending path
and creates nothing; `$dumpvars` opens the file. A design that calls neither writes no waveform,
even when `-o` is given, and that is not an error.

| Question | Answer |
|---|---|
| Which path | `-o`/`--out`, else the `$dumpfile` argument, else `dump.vcd` in the current directory |
| Which format | the path's extension, compared case-insensitively: `.fst` selects FST, anything else selects VCD. The rule applies to a `-o` path and a `$dumpfile` argument alike |
| How FST is produced | a sidecar VCD is written during the run and transcoded at finalize, then deleted; the transcode is a pure-Rust writer |
| A second `$dumpvars` | warns once (`W-RUN-DUMP-MULTI` / `VITA-W4021`) and is ignored — the first call fixes both the file and the filter |
| A file that cannot be opened or flushed | a warning, and the run continues without a waveform |

## The two execution modes

One-shot: `vita design.sv` runs compile, elaborate and simulate in one process and writes no
intermediate artifact.

Staged: three commands, each consuming the previous one's artifact.

| Command | Input | Output |
|---|---|---|
| `vcmp` | source files | `<first-source>.vu` — the serialized compilation unit, or a `--work` library entry |
| `velab` | one `.vu`, or `-L` libraries plus `--top` | `<in>.velab` — the golden `SimIr` plus its trailer sidecars |
| `vrun` | one `.velab` | stdout transcript and waveform |

The split exists for four reasons:

- it matches the compile/elaborate/simulate separation of the commercial flows (Cadence
  `xmvlog`/`xmelab`/`xmsim`, Synopsys `vlogan`/`vcs`/`simv`), so an existing project build structure
  carries over;
- each stage is separately buildable and separately debuggable;
- an unchanged stage is skipped by reusing its artifact;
- a work library addresses design units by a `library:unit` logical key, so a large design can be
  compiled once and elaborated many ways.

Reuse is only safe if a stale artifact is refused rather than silently simulated, so every artifact
header is gated on `format_version`, the tool's major version and the structural schema hash, and
`vrun` re-verifies the digests of everything the snapshot consumed. A gate rejection is exit code 2,
which is distinct from a design error (1) and from a usage error (3). The contract is
[14](14-staged-artifacts.md); the hash is [16](16-schema-hash-spec.md).

Status at HEAD: the observability rail is one-shot only. `--obs-dir`, `--probe` and `$vita_stage`
are refused with a diagnostic under `vcmp`, `velab` and `vrun` ([19](19-ai-agent-observability.md)).
The staged commands ship as links onto the one installed `vita` binary, which dispatches on
`argv[0]`; separate `vcmp`/`velab`/`vrun` executables are behind the development-only
`separate-bins` cargo feature.

## The executors

One design runs on one of three interchangeable process-body executors. They exist so that a
suspected defect can be bisected against a second implementation, and their agreement is a gate.

| `--backend` | Role | Compiled in |
|---|---|---|
| `native` (default) | net values in a flat arena; the shipping executor | always |
| `vm` | bytecode compiled per process body | `oracle` feature, on by default |
| `interp` | the readable reference semantics, excluded from performance work by rule | `oracle` feature, on by default |

A build made with `--no-default-features` carries `native` alone; there, an eligibility refusal is a
graceful fatal because no other executor exists to answer. With the oracle executors compiled in, a
refusal falls back to the VM and says so (`W-RUN-BACKEND-FALLBACK`) — a slower answer, not a
different one. The executor that actually ran is reported in `run.json` beside the one requested.
The architecture is [04](04-architecture.md); the arena backend is [21](21-tier3-native-backend.md).

## Where vitamin sits among the reference tools

| Tool | Category | Strength | Role here |
|---|---|---|---|
| Synopsys VCS | commercial, compiled | industry-reference accuracy, full SV and UVM, high throughput | accuracy reference; the equivalence target within the supported scope |
| Cadence Xcelium | commercial, compiled | multi-language (SV/VHDL/e), parallel compile, advanced verification environment | reference for the commercial flow the staged commands mirror |
| Icarus Verilog | open source, event-driven, 4-state | free, light, VCD output, Verilog-2005 and part of SV | the live differential oracle, and the closest comparison by class |
| Verilator | open source, compiled, 2-state | very fast cycle-accurate simulation of large designs | a calibrated second oracle: comparable only outside 2-state, X-init and event-ordering differences |
| vitamin | open source, event-driven, 4-state, Rust | memory safety, no GC, cargo-only source build, byte-identical output across platforms, an agent-readable observability rail | this project |

Measured position, release builds, interleaved runs with the first round discarded:

- picorv32 with its testbench: `native` 0.513 s, `vm` 0.838 s, `interp` 1.319 s, Icarus Verilog 13
  0.585 s.
- The ten-workload corpus, median of three runs: a geometric mean of 1.74× faster than Icarus
  Verilog over the nine timed rows, 1.93× over the seven third-party ones, with two rows slower
  ([study/03](../study/03-workload-corpus.md)).
- Keccak-f[1600]: Verilator 5.050 is 42× faster than vitamin on the call-free spelling of the design
  and 291× faster on the spelling that uses subroutine calls; all four tools agree on the computed
  result.

That last gap is structural, not a tuning deficit. A compiled 2-state simulator and the commercial
simulators are one to two orders of magnitude faster than an event-driven 4-state one, and Verilator
buys its margin by giving up the 4-state values and the event ordering that this project's first
goal requires. The measurement method behind any such claim is [study/01](../study/01-interpreted-vs-compiled.md).

## Related documents

| Question | Document |
|---|---|
| what is in and out of the language scope, and what the two goals commit to | [01-goals-and-scope.md](01-goals-and-scope.md) |
| which crates exist, how they layer, and how the executors are wired | [04-architecture.md](04-architecture.md) |
| the tiers of language scope and which one HEAD satisfies | [05-strategy-and-roadmap.md](05-strategy-and-roadmap.md) |
| every flag of the four commands | [../manual/004_cli-reference.md](../manual/004_cli-reference.md) |
| what is still open | [../ROADMAP.md](../ROADMAP.md) and [../REMAINING_WORK.md](../REMAINING_WORK.md) |
| where the external references come from | [11-sources-and-citations.md](11-sources-and-citations.md) |
