# 03 — Build and portability

The build contract: how vitamin is built, what may and may not participate in that build,
which Cargo features exist and what each one changes, which versions are pinned and why,
which platforms are supported, and what CI enforces on every push.

Companion contracts: the crate graph and layering rules in
[04-architecture.md](04-architecture.md); the crate-selection rationale in
[02-implementation-language.md](02-implementation-language.md); the structural hash the
build produces in [16-schema-hash-spec.md](16-schema-hash-spec.md).

---

## 1. The build contract

Every installation compiles from source on the target operating system. The project
distributes no prebuilt binary, and nothing in the build depends on one.

| Rule | What it forbids | Why |
|---|---|---|
| `cargo` is the only entry point | `cmake`, `make`, wrapper scripts, a code-generation step run before the compiler | one command builds and tests the whole workspace on every supported OS, with no second toolchain to install or version-match |
| No vita crate has a `build.rs` | build scripts, shell-out during compilation, generated source under `target/` | a build script is a second, unversioned program whose output the compiler trusts; without one, what rustc sees is exactly what is in the repository |
| Pure Rust, no C library dependency | `*-sys` crates, vendored C, pkg-config probing | C dependencies are where multi-OS builds break. A C linker is still required, which is the one non-Rust prerequisite |
| `--locked` on every invocation | resolving a fresh dependency graph at build time | the committed `Cargo.lock` is what makes three operating systems agree on one dependency set |
| Structural hashes come from a proc-macro | a codegen pass, a build-script-emitted table | `#[derive(SchemaHash)]` runs inside rustc, so the hash machinery obeys the no-build-script rule ([16-schema-hash-spec.md](16-schema-hash-spec.md)) |

The single build script in the tree belongs to the vendored `third_party/libm`. It emits
target-detection `cfg`s and nothing else, so its output is a deterministic function of the
target triple.

"Built from source" describes where a binary comes from, not how it is run. The flow is
`cargo build --release` or `cargo install` to produce a binary, then the installed terminal
commands `vita` / `vcmp` / `velab` / `vrun` — the same model as ripgrep, fd or
uutils-coreutils. `cargo run` is a contributor workflow, not the end-user path.

---

## 2. The workspace

Seventeen member crates in one Cargo workspace, `resolver = "2"`. Each crate has one
responsibility and a stated interface, so it can be tested on its own.

| Crate | Owns | Publishable |
|---|---|---|
| `hdl-preprocess` | compiler directives, macro expansion, `` `include ``, `` `timescale `` regions, the source map | yes |
| `hdl-lexer` | tokenization (logos-based) | yes |
| `hdl-parser` | hand-written recursive-descent parser: tokens to AST | yes |
| `hdl-ast` | the AST types; `serde` + `SchemaHash` derived; the `.vu` root type | yes |
| `elaborate` | parameters, generate, instances, hierarchy flattening, type and connectivity checks, lowering to the IR | yes |
| `sim-ir` | the frozen language-neutral IR and the rules shared with elaborate | yes |
| `sim-engine` | the event-driven 4-state kernel, the three executors, the `$` task and function handlers | yes |
| `hdl-builtins` | reserved as the extraction target for those handlers. A one-line stub at HEAD | no |
| `vcd-writer` | VCD serialization and the VCD-to-FST transcode | yes |
| `diag` | the diagnostic data model and the shared `$display` field rules. No IO | yes |
| `vita-artifact` | the `.vu` / `.velab` container header and the staleness gates | yes |
| `vita-artifact-derive` | the `#[derive(SchemaHash)]` proc-macro | yes |
| `vita-schema` | the `SchemaShape` trait, the shape registry, the blake3 hash | yes |
| `vita-log` | the `-Wno-` / `-Werror=` gate policy layered over a `LogSink` | yes |
| `cli` | the driver: argv, filelists, pipeline wiring, artifact bodies, the observability rail, the `vita` binary | yes |
| `vcd-diff` | reserved for a normalized VCD diff. A one-line stub at HEAD; no CLI and no caller exist | no |
| `corpus-runner` | the workload-corpus tool | no |

`crates/testdata/` is a data directory, not a crate: it holds the byte goldens
`sim_ir_canonical.txt` and `sim_ir_registry.ron`.

### 2.1 Workspace package metadata

| Key | Value |
|---|---|
| `version` | `0.2.0`, shared by every member |
| `edition` | `2021` for every vita crate |
| `rust-version` | `1.85` |
| `license` | `MIT OR Apache-2.0`, with `LICENSE-MIT` and `LICENSE-APACHE` at the repository root |
| `repository` | `https://github.com/tjddnr0912/vitamin-rtl-simulator` |

The major version stays at `0` by contract. `tool_semver_major` is one of the three hard
staleness keys in the artifact header, and the header gate compares the major only, so a
`0` to `1` bump would invalidate every `.velab` and `.vu` in existence while a minor bump
leaves them valid. See [14-staged-artifacts.md](14-staged-artifacts.md).

`Cargo.lock` is committed, lockfile format `version = 4`, resolving 131 packages: the 17
members, the vendored `libm`, and 113 from crates.io.

### 2.2 The excluded vendored library

`third_party/libm` is a path dependency and deliberately not a workspace member —
`exclude = ["third_party/libm"]`.

| Fact | Value |
|---|---|
| Upstream | `libm` 0.2.16, MIT, from `rust-lang/compiler-builtins` |
| Why excluded | `cargo clippy --workspace -- -D warnings` must not lint third-party code, and `cargo test --workspace` must not run its tests |
| Consumed with | `default-features = false`, which turns the `arch` feature off |
| What that buys | no hardware float intrinsics, therefore bit-identical `f64` results on every IEEE-754 target |
| Consumer | `sim-engine` only, for the real-math transcendentals and the non-uniform `$dist_*` functions |
| Enforced by | `crates/sim-engine/tests/libm_determinism.rs` |

Its own `edition` (2021) and `rust-version` (1.63) are independent of the workspace. Its
`no-panic` dev-dependency is dropped from the vendored copy, because its tests are never
built here. The vendored copy is not edited.

---

## 3. Cargo features

Three feature names exist in the workspace. There are no others.

| Feature | Owner | Default | Effect |
|---|---|---|---|
| `oracle` | `sim-engine`, forwarded by `cli` | on | compiles the `Backend::Interpreter` and `Backend::Bytecode` variants and their dispatch, which makes `--backend interp`, `interpreter`, `vm` and `bytecode` accepted values, and compiles the fallback arm a native-gate refusal lands on |
| `jit` | `sim-engine`, forwarded by `cli` | off | adds the five `cranelift-*` crates and the machine-code generation path |
| `separate-bins` | `cli` | off | emits standalone `vcmp`, `velab` and `vrun` binaries in addition to `vita` |

### 3.1 What `oracle` gates

`oracle` gates the choice of executor, not the semantics. Statement semantics live in code
generic over the `Kernel` trait and are shared by every executor; the compiled-body
machinery in the VM is also the native backend's own fast path. The feature therefore
removes two enum variants and their dispatch, and deletes nothing else — the VM-only
surface is about 95 lines, and `exec/` holds one interpreter-only function.

What turning it off changes is the outcome of a gate refusal. With a second executor
compiled in, a refusal is a slower answer; with none, the only remaining outcomes are loud
or wrong, so the refusal is promoted to a fatal.

| | `cargo build` | `cargo build --no-default-features` |
|---|---|---|
| `oracle` | on | off |
| Executors compiled | `interp`, `vm`, `native` | `native` only |
| Default executor | `native` | `native` |
| `--backend vm` / `interp` | accepted | `error[VITA-E0001]`, exit 3 |
| A native-gate refusal | `warning[VITA-W4030]`, falls back to the VM, exit 0 | `fatal[VITA-F4004]`, exit 1 |
| Purpose | development and verification, with the differential oracles alive | the shape a release ships |

`--backend` is a debugging control. `native` is the default and runs every design; the
other two exist so a suspected defect can be bisected against a second implementation of
the same semantics. `interp` is permanently excluded from performance work. See
[04-architecture.md](04-architecture.md).

### 3.2 Gating the product shape

```sh
cargo build  -p cli -p sim-engine --locked --no-default-features
cargo clippy -p cli -p sim-engine --locked --no-default-features -- -D warnings
cargo test   -p sim-engine        --locked --no-default-features --lib
```

Three rules make that axis mean something. Each failure mode looks the same from outside —
a green build that tests nothing.

1. Feature unification. A dependency declared without `default-features = false` carries
   its own defaults in, and a `--no-default-features` build of the consumer then compiles
   them anyway. `cli`'s `sim-engine` dependency carries `default-features = false`, and
   that line is load-bearing. Check with
   `cargo tree -p cli --no-default-features -e features`.
2. `--lib` is required. Integration-test targets pull `sim-engine` through
   dev-dependencies with default features, which re-enables `oracle`. This is also why the
   axis is `-p cli -p sim-engine` rather than `--workspace`.
3. The two configurations share `target/debug/vita`. After changing configuration, rebuild
   before measuring; a stale binary reports that `--backend vm` was accepted in a build
   that has no VM.

### 3.3 `jit`

`jit` is machine-code generation through cranelift. It is off by default and stays off. It
is orthogonal to `oracle`: `oracle` decides whether the oracle executors ship, `jit`
decides whether a third spelling of expression semantics is compiled at all.

| Property | Value |
|---|---|
| Default | off |
| Cost when on | about 29 additional crates, and `unsafe` at the generated-code call boundary |
| Second gate | the `VITA_JIT` environment variable, checked at run time; `VITA_JIT_STATS` prints per-run codegen statistics |
| Measured verdict | 14–47% slower than the native backend. About 38% of a run is shim — a call back into Rust per leaf load, and a 72-byte value rebuilt per write — against an op-dispatch ceiling of 8.9–11.3% |
| Status at HEAD | present in the tree, not a CI axis, not enabled in any shipped configuration |

Being behind a feature is not an exemption from verification. Building and checking it by
hand:

```sh
cargo build  -p sim-engine --features jit --locked
cargo clippy -p sim-engine --features jit --all-targets --locked -- -D warnings
VITA_JIT=1 cargo nextest run --workspace --features sim-engine/jit --locked
VITA_JIT=1 VITA_JIT_STATS=1 vita --backend native design.sv
```

The measurement and its method are in [study/01](../study/01-interpreted-vs-compiled.md);
the feasibility analysis is in [18-acceleration-analysis.md](18-acceleration-analysis.md).

### 3.4 `separate-bins`

The default build emits exactly one binary, `vita`, which dispatches on the `argv[0]` stem.
`separate-bins` adds three `[[bin]]` targets — `vcmp`, `velab`, `vrun` — each declared
`required-features = ["separate-bins"]`. Each is a shim whose whole body calls the same
`cli::driver_main()` entry point, so a standalone binary and the multicall path share one
code path with no branch between them. It exists to debug one stage in isolation.

---

## 4. Toolchain

`rust-toolchain.toml` at the repository root is canonical; rustup reads it automatically
and no manual override is needed.

```toml
[toolchain]
channel = "1.85.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
```

| Fact | Value |
|---|---|
| Channel | `1.85.0`, an exact stable release, not a range |
| Components | `rustfmt`, `clippy`, `rust-src` |
| Targets | x86_64 and aarch64 Linux-gnu, x86_64 and aarch64 Apple Darwin. This list is complete: no Windows target |
| Why 1.85 | `fst-writer` 0.3.x is edition 2024, which requires rustc 1.85 |

MSRV policy:

- 1.85 is a floor, not a ceiling. There is no upper bound, and a newer toolchain is
  followed rather than pinned away from.
- The floor is the maximum over the requirements of the adopted crates. `fst-writer` 0.3.x
  sets it; every other dependency requires less.
- vita's own crates stay on edition 2021 even though the toolchain permits 2024. Raising
  the floor does not move the edition.
- An MSRV increase is a minor version bump. It never lands in a patch release.

---

## 5. Dependency pins

Every pin below is a decision with a reason. A pin without one is a pin that will be
removed by the next person who reads it.

| Dependency | Spec | Kind | Reason |
|---|---|---|---|
| `blake3` | `=1.8.2` | exact | 1.8.3 and later move to edition 2024 |
| `fst-writer` | `=0.3.1` | exact | the 0.2.x line writes a malformed FST time table that GTKWave and `fst-reader` tolerate and wellen (Surfer) rejects. 0.3.x fixes it, and its edition-2024 requirement is what sets the workspace MSRV |
| `fst-reader` | `=0.10.2` | exact | an independent reader by another author, used as the transcode oracle: read back what was written and assert it round-trips |
| `cranelift-{jit,module,codegen,frontend,native}` | `0.120` | minor line | the newest line that builds on rustc 1.85; 0.134 requires 1.94. Optional, absent unless `jit` is on |
| `logos` | `0.16`, `default-features = false` | feature pin | strips the default `export_derive` and `std`; `hdl-lexer` re-enables exactly those two |
| `libm` | `path = "third_party/libm"`, `default-features = false` | vendored, feature pin | the `arch` feature off means no hardware float intrinsics, therefore bit-identical `f64` everywhere |
| `postcard` | `1.1`, `use-std` | — | the single artifact wire encoder. There is no second encoder and no fallback |
| `serde` | `1.0`, `derive` | — | the serialization boundary for AST, IR and artifact types |
| `syn` | `2.0`, `full` + `extra-traits` | — | proc-macro input parsing in `vita-artifact-derive` |
| `quote`, `proc-macro2` | `1.0` | — | proc-macro output and token types |
| `hex` | `0.4` | dev only | hash rendering in tests |
| `serde-reflection` | `0.6` | dev only | structural reflection over the IR for the frozen-shape gate |
| `ron` | `0.8` | dev only | the golden shape registry format |

Every direct external dependency, with where it is used:

| Crate | Where | Kind | Purpose |
|---|---|---|---|
| `logos` | `hdl-lexer` | prod | tokenizer derive and engine |
| `serde` | `hdl-ast`, `sim-ir`, `elaborate`, `vita-artifact`, `cli` | prod | serialization derives |
| `postcard` | `vita-artifact`, `cli`; `hdl-ast` (dev) | prod + dev | the artifact wire encoder |
| `blake3` | `vita-schema`, `cli`; `sim-ir` (dev) | prod + dev | schema and artifact digests |
| `libm` | `sim-engine` | prod | deterministic transcendentals |
| `fst-writer` | `vcd-writer` | prod | FST waveform emission |
| `fst-reader` | `vcd-writer`, `cli` | dev | the independent FST reader oracle |
| `syn`, `quote`, `proc-macro2` | `vita-artifact-derive` | prod | the proc-macro |
| `hex` | `sim-ir`, `vita-artifact` | dev | hash rendering |
| `serde-reflection`, `ron` | `sim-ir` | dev | the frozen-shape reflection gate and its golden registry |
| `cranelift-*` | `sim-engine` | prod, `jit` only | machine-code generation |

Dependency policy:

1. A pure-Rust crate is preferred whenever one satisfies the requirement. A C-binding
   crate is not adopted where a pure-Rust crate exists.
2. Adopting a C dependency would require the same build to be proved on every supported
   platform, and static linking to be available. No such dependency exists at HEAD.
3. `Cargo.lock` is committed and `--locked` is used everywhere.
4. A dependency audit step is not part of CI at HEAD.

Two `windows-sys`, `windows_*` and `find-msvc-tools` families appear in the lockfile as
cfg-gated transitive entries. They are lock rows, not evidence of Windows support: no
Windows target is listed in the toolchain file and no Windows runner exists in CI.
`winnow` likewise appears transitively; `hdl-parser` does not use it and is hand-written.

---

## 6. Profiles

```toml
[profile.release]
opt-level     = 3
lto           = "thin"
codegen-units = 1
strip         = "symbols"

[profile.dev.package."sim-engine"]
opt-level = 2
```

| Setting | Reason |
|---|---|
| `opt-level = 3` | the event loop is throughput-critical. Size-oriented levels are excluded |
| `lto = "thin"` | cross-crate inlining across the pipeline at a link cost that stays acceptable |
| `codegen-units = 1` | maximum optimization of the hot loops, paired with LTO |
| `strip = "symbols"` | a small binary and a clean macOS signing surface; there is one binary to strip |
| `sim-engine` at `opt-level = 2` in dev | the engine's inner loops stay usable in a debug build while the rest keeps full debug info |

`panic` is deliberately unset, so unwinding is in force. Two things depend on it: the whole
CLI runs on a spawned worker thread with a 256 MiB stack, and a panic there is joined and
re-raised so the process exits with the conventional panic code rather than a vita exit
class; and RAII guards in the engine, such as the recursion-depth decrement, must run
during unwind. A `catch_unwind` boundary that flushes a partial waveform on an engine
panic is not implemented.

No `[profile.dist]` exists. Fat LTO is not used.

---

## 7. What the build guarantees about output

The same input produces byte-identical stdout, waveform and exit code on every supported
platform and at every thread count. Four build-level facts carry that guarantee.

| Mechanism | What it prevents |
|---|---|
| `--locked` everywhere | two platforms resolving different dependency versions |
| Vendored `libm` with `arch` off | a hardware intrinsic producing a different `f64` on one target |
| `.gitattributes` marks `crates/testdata/**`, `crates/**/testdata/**` and `docs/preview/15-error-code-reference.md` as `-text` | a checkout rewriting line endings under a byte-compared golden or an `include_str!` source |
| No build script | generated source differing between machines |

Build provenance is stamped into every artifact header from `option_env!("VITA_GIT_SHA")`,
`option_env!("VITA_GIT_DIRTY")`, `env!("CARGO_PKG_VERSION")` and `cfg!(debug_assertions)`.
A CI or installer wrapper injects the git values; a plain `cargo build` records them as
absent. Provenance is traceability only and is never a staleness key, so a dirty tree does
not force a rebuild. If a git SHA ever becomes mandatory, a `vergen-gix` build script —
pure Rust, no shell-out — is the one admissible exception to the no-build-script rule.

The serialization rules the build has to respect — no `usize`, `isize`, `f32` or `f64` in a
frozen type, `BTree`-only collections, span-free types, the fully-qualified `sim_ir::`
spelling, and the module-path sensitivity of `SchemaHash` — are stated in
[16-schema-hash-spec.md](16-schema-hash-spec.md) and
[17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md). The container version
is `CURRENT_FORMAT_VERSION`, currently 31.

---

## 8. Platforms

Linux and macOS are supported and CI-tested. Windows is not a target: it is absent from the
toolchain target list, absent from CI, and absent from the install script.

| Platform | Rust install | Notes |
|---|---|---|
| Ubuntu LTS (22.04 / 24.04) | rustup | the reference platform, exercised on every push by the `ubuntu-latest` runner |
| RHEL 8/9 and derivatives | rustup | glibc 2.28 (RHEL 8) and 2.34 (RHEL 9). CI proves the RHEL 9 axis in a `redhat/ubi9` container, which needs `dnf install -y gcc` for a linker |
| macOS, Apple Silicon and Intel | rustup | both `aarch64-apple-darwin` and `x86_64-apple-darwin` are toolchain targets. A universal binary is not produced |

Common setup:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
git clone https://github.com/tjddnr0912/vitamin-rtl-simulator
cd vitamin-rtl-simulator
cargo build --workspace --locked
cargo test  --workspace --locked
```

---

## 9. CI

`.github/workflows/ci.yml` is the only workflow file. It triggers on pushes to `main` and
on every pull request, with concurrency grouped per workflow and ref and
`cancel-in-progress` on. Three job definitions expand to four job runs.

| Job | Runner | What it holds |
|---|---|---|
| `build-native` | `ubuntu-latest` and `macos-latest`, `fail-fast: false` | the four canonical commands, on both native platforms |
| `build-no-oracle` | `ubuntu-latest` | the product shape: one executor, and a refusal that has nowhere to fall back to |
| `build-rhel` | `ubuntu-latest` in a `redhat/ubi9` container | the same build and test on the glibc and RHEL axis |

Shared actions: `actions/checkout@v6`, `dtolnay/rust-toolchain@1.85.0`,
`Swatinem/rust-cache@v2` keyed per job.

`build-native` steps, in order:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
cargo test --workspace --locked
```

`build-rhel` steps: `dnf install -y gcc`, then `cargo build --workspace --locked` and
`cargo test --workspace --locked`.

`build-no-oracle` steps: the three product-shape commands of §3.2, then a smoke script that
proves both halves of the shape at once — a design runs, and an executor that is not
compiled in is refused rather than silently ignored.

```sh
set -euo pipefail
# writes a small design, then:
./target/debug/vita smoke.sv | tee out.txt
grep -q 'q=7' out.txt
if ./target/debug/vita --backend vm smoke.sv; then
  echo "FAIL: --backend vm was accepted in a build without the VM"; exit 1
fi
```

`build-no-oracle` is a separate job rather than an extra step on the others because of
feature unification: if any crate in a workspace build enables `oracle`, every crate gets
it, so the product shape exists only in a build that never enables it anywhere.

Gate summary:

| Gate | Enforced in | Command |
|---|---|---|
| Formatting | `build-native` ×2 | `cargo fmt --all -- --check` |
| Lint, zero warnings, all targets | `build-native` ×2 | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Lint, product shape | `build-no-oracle` | `cargo clippy -p cli -p sim-engine --locked --no-default-features -- -D warnings` |
| Workspace build | `build-native` ×2, `build-rhel` | `cargo build --workspace --locked` |
| Product-shape build | `build-no-oracle` | `cargo build -p cli -p sim-engine --locked --no-default-features` |
| Full test suite | `build-native` ×2, `build-rhel` | `cargo test --workspace --locked` |
| Product-shape lib tests | `build-no-oracle` | `cargo test -p sim-engine --locked --no-default-features --lib` |
| End-to-end smoke and loud refusal | `build-no-oracle` | the script above |
| Lockfile pinning | every build, test and clippy step | `--locked` |

Clippy has to be `--workspace --all-targets`. A crate-scoped `-p <crate>` run skips
`crates/cli/tests/*.rs`, which CI lints.

Not in CI at HEAD: any Windows runner, any release or publish workflow, `cargo audit`, and
`cargo nextest`.

### 9.1 The local gate

| | `cargo nextest run --workspace --locked` | `cargo test --workspace --locked` |
|---|---|---|
| Role | the local full gate | what CI runs |
| Reads `.config/nextest.toml` | yes | no |
| Per-test timeout | `slow-timeout = { period = "60s", terminate-after = 4 }`, a hard four-minute ceiling | none |
| Result at HEAD | 7352 tests, 7352 passed, 15 skipped, exit 0, about 36 s | the same suite, far slower |

The 15 skipped tests are the `#[ignore]`d performance probes; they are data, not gates.

The timeout is the reason `.config/nextest.toml` exists. Without a per-test cap, "the suite
is still running" and "the machine is dying" are indistinguishable, and a non-terminating
loop in a mutated design can take the machine down. The cap sits far above every real test:
the whole workspace is about half a minute of wall clock and the slowest single test is
about 18 s, so a test that reaches four minutes is hung rather than slow.

The two runners are not interchangeable, and switching between them inside one session
forces a full rebuild. Pick one and stay on it.

---

## 10. Install and invocation

```sh
# build from source; --locked uses the committed lockfile
cargo build --release --workspace --locked      # -> target/release/vita

# install into ~/.cargo/bin, which rustup already puts on PATH
cargo install --path crates/cli --locked
cargo install --git https://github.com/tjddnr0912/vitamin-rtl-simulator -p cli --locked
```

The default `cli` build emits exactly one `[[bin]]`, so neither form needs a `--bin`
selector. `-p cli` is unambiguous because `cli` is the only crate with an installable
binary. A `--bin vita` selector is needed only when installing with `separate-bins` on.

`install.sh` does the whole job: it runs `cargo install --path crates/cli --locked`,
locates the installed binary through `PATH` or `${CARGO_HOME:-$HOME/.cargo}/bin`, and
creates the three staged names beside it — a symlink first, a copy if the filesystem
rejects links, and a loud failure if both fail. It supports Linux and macOS only.

```sh
vita  top.sv                                  # one-shot: nothing is written to disk between stages
vcmp  -y ./rtl --work work=./work *.sv        # compile
velab -s top -o top.velab                     # elaborate
vrun  top.velab +SEED=1                       # simulate, re-verifying the upstream chain
```

If `~/.cargo/bin` is not on `PATH`, add it: `export PATH="$HOME/.cargo/bin:$PATH"`.

---

## 11. Distribution

| Channel | Status at HEAD |
|---|---|
| Source build from the repository, via `install.sh` or `cargo install --path` | the supported channel |
| `cargo install --git … -p cli --locked` | supported; same source-build semantics. macOS attaches no quarantine attribute to a locally built binary, so Gatekeeper does not prompt |
| crates.io | not published. 14 of the 17 crates are publishable; `hdl-builtins`, `vcd-diff` and `corpus-runner` carry `publish = false` |
| Prebuilt release archives | not produced. No release or publish workflow exists. A downloaded macOS archive would carry the quarantine attribute, so an unsigned binary would need Developer ID signing and notarization |
| OS packages: deb, rpm, Homebrew | not provided |

Source build remaining the primary channel is what keeps any future convenience archive a
cached build rather than a vendor-supplied binary.

---

## 12. Build-tree hygiene

The workspace carries 613 integration-test targets. Cargo writes a new hashed binary per
test target per build and never reclaims the superseded ones, so `target/` grows on the
order of tens of gigabytes a month under repeated full-workspace test runs.

```sh
cargo install cargo-sweep
cargo sweep --time 2        # delete artifacts untouched for two or more days
cargo sweep --time 2 -d     # dry run
```

Two days of retention keeps everything in current use: a sweep followed by
`cargo build --workspace --locked` finds nothing to recompile. Prefer it to `cargo clean`,
which discards current artifacts and forces a full rebuild. Do not sweep while a build or
test run is in flight, and do not hand-roll a `find -delete`, which misses the nested
`build/` and `examples/` directories. This affects the build tree only, never a released
binary or a simulation result.

---

## Related documents

- [02-implementation-language.md](02-implementation-language.md) — why these crates and this
  language
- [04-architecture.md](04-architecture.md) — the crate graph and the layering rules
- [09-testing-and-verification.md](09-testing-and-verification.md) — the test-layer contract
  behind the gate
- [14-staged-artifacts.md](14-staged-artifacts.md) — the artifact container and the
  staleness gates the version policy protects
- [16-schema-hash-spec.md](16-schema-hash-spec.md) — the proc-macro hash the build produces
- [../../CONTRIBUTING.md](../../CONTRIBUTING.md) — the same rules as a contributor workflow
- [../manual/001_installation.md](../manual/001_installation.md) — installation for users
- [../history/specs/2026-05-26-vitamin-rtl-simulator-design.md](../history/specs/2026-05-26-vitamin-rtl-simulator-design.md)
  — the original whole-project specification, superseded by this set
