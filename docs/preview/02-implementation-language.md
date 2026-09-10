# 02 · Implementation language

The implementation language is Rust, and the choice carries rules: which crates may be depended on,
what each one is there for, what a candidate dependency must satisfy, how the minimum supported Rust
version is set, and why one dependency is vendored into the tree. This document is that contract.
The build recipe itself is [03](03-build-and-portability.md).

## The decision

Rust. Five properties of the implementation language are load-bearing here, and the alternatives
were weighed against them.

| Requirement | Why the simulator needs it |
|---|---|
| Deterministic execution with no garbage collector | the kernel is a tight event loop, and a collector pause costs both throughput and the ability to reason about the run as a pure function of the input |
| Sum types with exhaustive pattern matching | the front end is a large BNF and the IR is an arena of tagged nodes. An exhaustive `match` over a frozen enum is how a new IR variant becomes a compile error at every consumer instead of a silent fall-through |
| Memory safety | a simulator defect does not surface as a crash; it surfaces as a plausible wrong waveform. Removing the whole class of memory defects removes the hardest half of that search |
| A first-class source build on every supported target | one command, no generated build system, no host toolchain beyond a linker |
| A permissive-licensed ecosystem with pure-source crates | anything pulled in must build from source everywhere the simulator does |

| Candidate | For | Against | Verdict |
|---|---|---|---|
| Rust | all five above; `cargo` gives reproducible multi-target source builds; `enum` plus matching fits the AST and IR directly | learning curve; compile times above C++ on a cold tree | chosen |
| C / C++ | the proven EDA path (Verilator is C++, Icarus is C/C++), maximum portability and ecosystem, familiar to EDA engineers | keeping a parser and simulator of this size memory-safe is manual work that never ends; cross-platform builds carry header and linker friction | not chosen |
| Go | simple language model, fast builds, easy cross-compilation | collector pauses work against deterministic timing and throughput; the absence of sum types makes an AST and its traversals awkward | not chosen |

Rust HDL and EDA tooling is established rather than experimental — `veryl`, `spade`, `sv-parser` and
the Surfer waveform viewer are all Rust — so adopting it is not a bet on the ecosystem appearing.

## What a dependency must satisfy

Every rule here is enforced by something: a lockfile entry, a CI job, or a test.

1. Pure Rust. A dependency that needs a C or C++ compiler, a vendored C library, cmake or make is
   not admissible. No vitamin crate has a `build.rs`; the only `build.rs` in the tree belongs to the
   vendored libm and emits target-detection `cfg`s only.
2. Deterministic output. A dependency on the path from source to stdout or to waveform bytes must
   produce the same bytes on every supported target. This is why the float functions are vendored
   with hardware intrinsics disabled, and why no hash-map iteration order reaches an execution
   decision.
3. Buildable from source on every supported target, with no prebuilt artifact and no network step
   beyond the crates.io fetch.
4. A permissive licence compatible with `MIT OR Apache-2.0`.
5. A recorded version constraint. An exact pin (`=x.y.z`) is used when a specific release is
   load-bearing, a minor line when a range is; either way the reason is written down beside it, and
   every build and test command passes `--locked` so all three CI environments resolve identically.
6. Feature discipline. A dependency is taken with `default-features = false` when its default
   features would pull in terminal, colour or platform code that a leaf crate must not have. The
   same applies inside the workspace: `cli` depends on `sim-engine` with `default-features = false`,
   which is what makes a `--no-default-features` build genuinely drop the oracle executors instead of
   silently getting them back through feature unification.
7. Layer discipline. A crate that performs IO or rendering does not become a dependency of a leaf
   crate. The diagnostic model crate `diag` has no dependencies at all; rendering and log routing
   live one layer up in `vita-log`.

## The direct dependencies

Eighteen external crates are named directly; everything else in the lockfile is transitive.

| Crate | Constraint | Used by | Kind | What it is there for |
|---|---|---|---|---|
| `logos` | `0.16`, `default-features = false` plus `export_derive` and `std` | `hdl-lexer` | build | the tokenizer: a `#[derive(Logos)]` token table compiles to one DFA, which is faster and far less error-prone than a hand-written scanner. Defaults are stripped so exactly the two needed features are on |
| `serde` | `1.0` with `derive` | `hdl-ast`, `sim-ir`, `elaborate`, `vita-artifact`, `cli` | build | the serialization derives on the AST, the frozen IR and the artifact types |
| `postcard` | `1.1` with `use-std` | `vita-artifact`, `cli` | build | the single wire encoder for `.vu` and `.velab`. One encoder, so a byte layout cannot drift between producers |
| `blake3` | `=1.8.2` | `vita-schema`, `cli` | build | the schema and artifact digests. Pinned exactly: 1.8.3 and later move to edition 2024, which would raise the floor for no gain |
| `libm` | path `third_party/libm`, `default-features = false` | `sim-engine` | build | the real-math functions and the non-uniform `$dist_*`. Vendored — see below |
| `fst-writer` | `=0.3.1` | `vcd-writer` | build | FST waveform emission. Pinned exactly: the 0.2 line writes a time table that GTKWave tolerates and Surfer rejects. This crate is edition 2024, and is therefore what sets the MSRV floor |
| `fst-reader` | `=0.10.2` | `vcd-writer`, `cli` | test | an independent reader by a different code path, used as the transcode's correct-or-loud oracle: read back what was written and assert the round trip |
| `syn` | `2.0` with `full` and `extra-traits` | `vita-artifact-derive` | build | parsing the input of the `#[derive(SchemaHash)]` macro |
| `quote` | `1.0` | `vita-artifact-derive` | build | emitting that macro's output |
| `proc-macro2` | `1.0` | `vita-artifact-derive` | build | the token types both of the above are written against |
| `hex` | `0.4` | `sim-ir`, `vita-artifact` | test | rendering a digest for a golden comparison |
| `serde-reflection` | `0.6` | `sim-ir` | test | structural reflection over the frozen types, which is how the shape gate detects a field added, removed or reordered |
| `ron` | `0.8` | `sim-ir` | test | the golden registry format for those shapes |
| `cranelift-jit`, `cranelift-module`, `cranelift-codegen`, `cranelift-frontend`, `cranelift-native` | `0.120`, `optional = true` | `sim-engine` | build, `jit` only | the machine-code experiment. Held at the 0.120 line because it is the newest that builds on the pinned toolchain; the feature is off by default, so none of these is compiled, resolved or shipped in a default build |

## Deliberately not depended on

Each of these is a component another project of this kind would take from crates.io. The reason for
writing it here instead is recorded so the question is not reopened by default.

| Not used | Instead | Why |
|---|---|---|
| A parser generator or combinator library | `hdl-parser` is a hand-written recursive-descent parser with a Pratt expression parser | the SystemVerilog grammar is roughly 1,800 Annex A rules with context sensitivity — type versus identifier, net versus variable — that needs lexer and symbol-table feedback, which an LR or PEG generator cannot express cleanly. Every production SystemVerilog front end is hand-written recursive descent for this reason. The parser also has to recover and keep going after an error, because the diagnostic corpus asserts several codes from one run; per-rule panic-mode recovery against a synchronizing set is a first-class part of the parser rather than an unstable feature of a library. `winnow` appears in the lockfile only as a transitive dependency of a proc-macro helper |
| A diagnostic rendering library | `diag` carries the model (`Diagnostic`, `MsgCode`, `Severity`, `SourceLoc`, `LogSink`) with zero dependencies, and `vita-log` renders | the code catalogue is compiled into the binary and gated 1:1 against the `MsgCode` enum ([15](15-error-code-reference.md)), so the model has to be owned here. Keeping the model crate dependency-free also keeps terminal, colour and platform code out of the bottom of the graph |
| `sv-parser` | the front end above | it returns a concrete syntax tree tracking Annex A rule names, which is a different shape from the AST this pipeline elaborates. It is still useful as a cross-check when confirming a grammar rule's name |
| The `vcd` crate | `vcd-writer` | waveform bytes are a golden compared byte for byte across platforms, so the writer sits on the determinism-critical path and is owned here |

## MSRV and toolchain

| Rule | Value at HEAD |
|---|---|
| The MSRV is a floor, not a ceiling | `rust-version = "1.85"`. There is no upper bound; the project follows new Rust releases |
| The floor is set by the maximum requirement of the adopted crates | `fst-writer` 0.3.x is edition 2024, which requires rustc 1.85. Every other adopted crate needs less |
| Raising the floor is a deliberate change | it names the crate that forced it, here and in [CONTRIBUTING.md](../../CONTRIBUTING.md) |
| vitamin's own crates stay on edition 2021 | the pinned toolchain permits edition 2024; adopting it would be a separate decision with no benefit to the contracts in this document |
| The toolchain is pinned in the repository | `rust-toolchain.toml` pins the channel exactly, requests `rustfmt`, `clippy` and `rust-src`, and lists the four supported targets, so a local build and a CI build use the same compiler |
| Every command passes `--locked` | resolution is identical in all three CI environments |

The lockfile contains `windows-sys` and related entries. They are configuration-gated transitive
rows reached through build helpers; Windows is not a toolchain target and has no CI runner.

## The vendored libm

`third_party/libm` is a copy of `libm` 0.2.16 (MIT). It is a path dependency and is deliberately not
a workspace member: the workspace `exclude` list keeps `cargo clippy --workspace -- -D warnings` from
linting third-party code and keeps `cargo test --workspace` from running its test suite.

| Property | Value |
|---|---|
| Why vendored at all | the 21 IEEE §20.8.2 real-math functions and the non-uniform `$dist_*` need a `f64` implementation whose bits do not depend on the host |
| How it is consumed | `default-features = false`, which turns off the `arch` feature, so no hardware float intrinsic is used and every result is bit-identical on any IEEE-754 target |
| What was changed in the copy | the `no-panic` dev-dependency is dropped, because the upstream test suite is never built here |
| Its `build.rs` | kept: it emits target-detection `cfg`s only, which is deterministic per target |
| Its own edition and floor | edition 2021, rustc 1.63 — independent of the workspace |
| Its only consumer | `sim-engine` |
| What holds the property | a determinism test in `sim-engine` that pins the function results |

This is the one exception to depending on crates.io directly, and it exists because the alternative —
taking `libm` from the registry with default features — would make a transcendental's low bits a
property of the machine that ran the simulation, which the portability goal forbids
([01](01-goals-and-scope.md)).
