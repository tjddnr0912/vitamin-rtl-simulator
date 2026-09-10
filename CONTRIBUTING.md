# Contributing to vitamin

How to build, gate and land a change in this repository: the pinned toolchain,
the commands that must pass, the determinism and frozen-type rules a change has
to respect, and the workflow for pushing it.

The method behind the rules — the correct-or-loud ladder, the census and
adversarial-review procedure — is in
[docs/ENGINEERING_RULES.md](docs/ENGINEERING_RULES.md). Read it before
implementing.

## Toolchain

`rust-toolchain.toml` is canonical and rustup picks it up automatically; no
manual override is needed.

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

| Pin | Value | Reason |
|---|---|---|
| rustc / cargo | 1.85.0 | `fst-writer` 0.3.x is edition 2024, which requires 1.85. 1.85 is a floor, not a ceiling — there is no upper MSRV bound |
| Edition | 2021 for every vita crate | the toolchain permits 2024; vita's crates stay on 2021 deliberately |
| `blake3` | `=1.8.2` | 1.8.3 and later move to edition 2024. Keep the exact pin |
| `fst-writer` | `=0.3.1` | the 0.2.x line writes an FST time table that GTKWave tolerates and wellen (Surfer) rejects |
| `fst-reader` | `=0.10.2` | an independent reader by another author, used as the transcode oracle: read back what was written and assert it round-trips |
| `cranelift-*` | `0.120` | the newest line that builds on 1.85; only compiled under the off-by-default `jit` feature |
| `libm` | vendored at `third_party/libm`, `default-features = false` | the `arch` feature off means no hardware float intrinsics, so `f64` results are bit-identical on every IEEE-754 target |

`--locked` is mandatory on every cargo invocation: the lockfile is what makes
three operating systems agree.

`third_party/libm` is a path dependency held outside the workspace by
`exclude`, so `clippy --workspace` does not lint it and `cargo test --workspace`
does not run its tests. Do not edit it.

### Cargo features

| Feature | Crate | Default | What it does |
|---|---|---|---|
| `oracle` | `sim-engine`, forwarded by `cli` | on | compiles the `interp` and `vm` executors, which makes `--backend interp\|interpreter\|vm\|bytecode` an accepted value. Without it those spellings are a loud CLI error |
| `jit` | `sim-engine`, forwarded by `cli` | off | the cranelift native-codegen experiment. It pulls in about 29 crates and needs `unsafe` at the call boundary; keeping it off keeps the default lock resolution and the byte-identical multi-OS contract untouched |
| `separate-bins` | `cli` | off | emits standalone `vcmp` / `velab` / `vrun` binaries for debugging one stage in isolation. Each is a shim over the same multicall entry point |

`cli` depends on `sim-engine` with `default-features = false`, and that is
load-bearing: without it, feature unification re-enables `oracle` and a
`--no-default-features` build silently compiles the oracle backends anyway.

## The gate

These four must pass before a push.

```sh
cargo build  --workspace --locked
cargo test   --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Clippy has to be `--workspace --all-targets`. A crate-scoped `-p <crate>` run
skips `crates/cli/tests/*.rs`, which CI lints.

Locally, `cargo nextest run --workspace --locked` runs the same suite: 7352
tests, all passing, 15 skipped, in about 36 s. The 15 are the `#[ignore]`d
performance probes in `crates/sim-engine/tests/perf_baseline.rs` and
`crates/cli/tests/perf_call_regime.rs` — data, not gates.

The two runners are not interchangeable. `cargo test` is what CI runs and is
much slower; only nextest reads `.config/nextest.toml`, which caps a single test
at 60 s with four retries — a hard four-minute ceiling, far above the slowest
real test at about 18 s. Without that cap, "the suite is still running" and "the
machine is dying" look the same. Switching between the two runners inside one
session forces a full rebuild, so pick one and stay on it.

### The product-shape axis

CI also builds the shape a release would ship: one executor, so a gate refusal
has nowhere to fall back to and is therefore loud.

```sh
cargo build  -p cli -p sim-engine --locked --no-default-features
cargo clippy -p cli -p sim-engine --locked --no-default-features -- -D warnings
cargo test   -p sim-engine        --locked --no-default-features --lib
```

`-p cli -p sim-engine` rather than `--workspace`, because the workspace's
dev-dependencies pull `sim-engine` with default features for the test targets
and would re-enable `oracle`. `--lib` for the same reason: the integration
targets carry those dev-dependencies. An axis written without both restrictions
goes green while testing nothing.

## Determinism rules

Byte-identical output across Linux and macOS is a guarantee, not an aspiration,
and it is enforced by tests that will reject a change that breaks it.

| Rule | Enforced by |
|---|---|
| No `usize`, `isize`, `f32` or `f64` in a serialized type | `crates/sim-ir/tests/no_float_usize.rs` |
| Collections are `BTree`-only, and types carry no source spans | the `SchemaHash` canonical string |
| No serde attributes on frozen types | `crates/sim-ir/tests/no_serde_attrs.rs` |
| `sim-ir` cross-type fields use the fully-qualified `sim_ir::Foo` spelling (`extern crate self as sim_ir`) | `crates/sim-ir/tests/body_refs.rs` rejects a bare reference |
| Byte goldens and the embedded diagnostic reference check out verbatim on every OS | `.gitattributes` marks `crates/testdata/**`, `crates/**/testdata/**` and `docs/preview/15-error-code-reference.md` as `-text` |
| Vendored `libm` produces the same bits everywhere | `crates/sim-engine/tests/libm_determinism.rs` |

One more rule has no test behind it, so it has to be respected by hand: a type
that derives `SchemaHash` must not move between modules. The canonical key
embeds `module_path!()`, so moving a type flips the pinned hash and invalidates
every `.vu` and `.velab` in existence. That is why `hdl-ast`'s types and
`sim-ir`'s frozen types live at their crate roots. Code that is never serialized
can move freely.

## Frozen types and `format_version`

`sim_ir::SimIr` is the golden root. Adding, removing or reordering a field in
any type reachable from it changes the structural root hash.

The container version is `CURRENT_FORMAT_VERSION` in
`crates/vita-artifact/src/header.rs`, currently 31. Three kinds of change touch
it, and they cost different amounts:

| Change | Consequence |
|---|---|
| A field added, removed or reordered in a frozen `sim-ir` type | the root hash flips. Bump `format_version`, re-pin `crates/testdata/sim_ir_canonical.txt` and `sim_ir_registry.ron`, and regenerate every `.velab`, all in the same commit |
| A staged trailer sidecar appended or its wire shape changed | an artifact wire-shape change. Bump `format_version`; the `SimIr` goldens stay untouched, and old artifacts fail loudly at the header gate rather than mis-decoding |
| An engine-facing side table threaded through `SimOpts` or synthesized during elaboration | out of band. The golden is unaffected and no bump is needed |

Every bump carries a comment on the constant saying what changed and why, so
that a version number can be read back to a wire shape. Add to that comment;
do not replace it.

The gate order at read time is `format_version`, then `tool_semver_major`, then
`schema_hash`, lowest first, so the most specific message wins. All three
rejections exit 2 and name the stage to re-run. The workspace major version
stays at 0 because `tool_semver_major` is a staleness key: a 0-to-1 bump would
invalidate every artifact on disk.

## File size and module splits

Keep a source file under about 1000 lines. When one approaches the limit, split
it rather than letting it grow:

- Add a submodule and give it a `use super::*` prelude.
- Mark the moved items `pub(crate)` and re-export them from the crate root, so
  no caller outside the crate changes.
- Leave the types at the crate root. A child module can then reach a type's
  private fields, which is what makes the split mechanical.
- Keep a single `trait impl` whole; do not split one across files.

Types that derive `SchemaHash` are the exception and must stay where they are,
for the reason above.

## Diagnostics

Every user-visible message has a code, and the code is defined in exactly one
place. Adding or changing one is a three-part edit:

1. Add or edit the row in the `msgcodes!` table in `crates/diag/src/code.rs`.
   A row is `(mnemonic, VITA-number, severity, title)`.
2. Add or edit the matching entry in
   [`docs/preview/15-error-code-reference.md`](docs/preview/15-error-code-reference.md),
   in the numbered body sections. The parser requires this header shape:

   ```text
   ### VITA-E0001 · `E-CLI-BAD-FLAG` (Error)
   ```

   Appendix A holds reserved numbers that are not yet enum variants and is
   excluded from the gate.
3. If the change is user-facing, update
   [`docs/manual/007_error-codes.md`](docs/manual/007_error-codes.md) in the
   same change.

`crates/diag/tests/bijection.rs` gates parts 1 and 2 against each other. It
asserts the enum has exactly 68 body variants, that the mnemonic sets are 1:1,
that each documented `VITA-####` number matches the enum's, that each documented
severity matches the enum's default, and that no mnemonic or number repeats.
Adding a code without a doc entry fails, and so does the reverse.

The reference file is compiled into the binary: `crates/cli/src/lib.rs` embeds
it with `include_str!`, and `vita explain <CODE>` prints the entry straight out
of the executable. That is why the file is `-text` in `.gitattributes` and why
its `###` header shape is load-bearing.

A user reaches a code through three spellings — the mnemonic, `VITA-W3056`, and
`W3056` — and `MsgCode::resolve` is the single function that accepts all three.
`vita explain`, `-Wno-<CODE>` and `-Werror=<CODE>` all call it. Do not add a
second resolver.

Adding a CLI flag has its own gate: `crates/cli/tests/help_covers_flags.rs`
parses the literal flag spellings out of the match arms in
`crates/cli/src/stage_args.rs` and `filelist.rs` and fails if any is missing
from the `--help` text.

## Tests

- Write the failing test first, make it pass, run the full gate, then commit.
- Every fix gets a regression test; every feature gets a focused test.
- Record the oracle in the test's module header: which tool and version produced
  the expected value. Most tests in `crates/cli/tests/` carry such a note.
- `crates/sim-engine/tests/differential.rs` runs a live diff against Icarus
  Verilog (`iverilog -g2012` plus `vvp`) for 27 designs, comparing `$display`
  stdout. It skips with a printed notice when neither tool is on `PATH`, and CI
  has neither — so it is a developer-machine gate, and a design still runs
  through vita when it skips.
- Where no external tool accepts the construct — SVA, classes, constrained
  random, parameterized classes, virtual interfaces — pin the expected value
  from the IEEE 1364/1800 text and state in the test header why there is no tool
  oracle.
- `crates/sim-engine/tests/backend_equiv.rs` asserts the three executors produce
  byte-identical output. A change to one executor's semantics belongs in all of
  them, or in none.

## CI

`.github/workflows/ci.yml` is the only workflow. It defines three jobs that
expand to four runs, triggered on pushes to `main` and on every pull request.

| Job | Runner | Steps |
|---|---|---|
| `build-native` | ubuntu-latest and macos-latest | fmt, clippy, build, test — the four canonical commands |
| `build-no-oracle` | ubuntu-latest | the product-shape build, clippy and lib tests, then a smoke script that simulates a small design, greps its output, and asserts `--backend vm` is rejected rather than silently ignored |
| `build-rhel` | ubuntu-latest in a `redhat/ubi9` container | `dnf install -y gcc` for a C linker, then build and test |

There is no Windows runner and no release or publish workflow.

## Build-tree hygiene

The workspace carries 613 integration-test targets. Cargo writes a new hashed
binary per test target per build and never reclaims the superseded ones, so
`target/` grows quickly under repeated full-workspace test runs — on the order
of tens of gigabytes a month on a development machine.

```sh
cargo install cargo-sweep
cargo sweep --time 2        # delete build artifacts untouched for 2+ days
cargo sweep --time 2 -d     # dry run: report what would be reclaimed
```

Two days of retention keeps everything currently in use: a sweep is followed by
a `cargo build --workspace --locked` that finds nothing to recompile. Prefer it
to `cargo clean`, which throws away current artifacts too and forces a full
rebuild. Do not sweep while a build or test run is in flight, and do not
hand-roll a `find -delete`, which misses the nested `build/` and `examples/`
directories.

This affects the build tree only, never a released binary or a simulation
result.

## Workflow

The repository owner pushes to `main` directly. Everyone else forks, opens a
pull request, and needs one approval. Force-pushing to `main` is not permitted.

- Keep the full gate green: build, test, clippy, fmt, all `--locked`.
- A commit subject states what is true of the tree after the change, in present
  tense, naming the area it touches.
- Keep a change scoped to one thing. A design change gets re-reviewed, not
  patched forward.
- A user-facing change to the CLI, the supported language, a system task or a
  diagnostic lands in `docs/manual/` in the same change, and in
  [`CHANGELOG.md`](CHANGELOG.md).
- New text in the repository is written in English.
- Do not commit secrets, local `.env` files, or generated `.vcd` output.

## Where things live

| Path | Contents |
|---|---|
| `crates/` | the 17-crate workspace: front end, elaborate, engine, artifacts, CLI |
| `third_party/libm/` | the vendored pure-Rust math library; do not edit |
| `examples/` | four runnable designs used by the manual |
| `bench/` | the workload corpus and its runner; upstream RTL is cloned at a pinned commit, never redistributed |
| [`docs/ENGINEERING_RULES.md`](docs/ENGINEERING_RULES.md) | the canonical method: the accuracy ladder, census and review procedure |
| [`docs/preview/`](docs/preview/) | the design specifications |
| [`docs/manual/`](docs/manual/) | the user manual |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | every open work item |
| [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md) | the one-screen snapshot and the queue |
| [`docs/history/`](docs/history/) | completed work, the development log, review documents and dated plans |
| [`docs/README.md`](docs/README.md) | the documentation map |

## Platforms

Linux and macOS are supported and CI-tested. Windows is not a target; the code
carries some Windows-aware paths, but nothing builds or tests it. A patch adding
Windows support is welcome and would need a CI runner to go with it.

## Licence

Contributions are dual licensed under MIT and Apache-2.0, matching the
repository. Submitting a contribution means agreeing to that.
