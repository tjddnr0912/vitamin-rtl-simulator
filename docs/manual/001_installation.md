# 001 · Installation

This chapter covers the prerequisites and why each version is what it is, building from source,
what the build produces, installing, reaching the staged commands, `PATH`, and verifying the
result.

vitamin is built from source on the target machine. There is no vendor prebuilt binary, and
`cargo` is the only build entry point — no `cmake`, no `make`, no `build.rs` in any vitamin
crate.

---

## 1.1 Platforms

| Item | Value |
|---|---|
| Operating systems | Linux and macOS |
| Targets | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| Windows | Not supported. It is not a target in `rust-toolchain.toml`, it has no CI runner, and there are no Windows install steps |

CI builds and runs the full suite on `ubuntu-latest`, on `macos-latest`, and inside a
`redhat/ubi9` container.

---

## 1.2 Prerequisites

### Rust toolchain

A Rust toolchain installed through `rustup`. The exact version is pinned in the repository, so
once `rustup` is present the correct compiler is selected automatically by any `cargo` command
run inside the checkout.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Restart the shell, or `source "$HOME/.cargo/env"`, so `cargo` is on `PATH`. The first `cargo`
invocation inside the checkout downloads the pinned toolchain on its own.

`rust-toolchain.toml` at the repository root is the authority:

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

### Why these versions

| Requirement | Version | Reason |
|---|---|---|
| **rustc / cargo** | 1.85.0 | The FST writer, `fst-writer 0.3.x`, is edition 2024, which requires rustc 1.85. The workspace declares `rust-version = "1.85"` as a floor with no upper bound — newer toolchains are fine, and vitamin's own crates stay on edition 2021 |
| **`fst-writer`** | `=0.3.1` | The 0.2.x line writes a malformed FST time table that GTKWave tolerates and wellen — the reader behind Surfer — rejects. 0.3.x fixes it. This pin is what sets the 1.85 floor |
| **`fst-reader`** | `=0.10.2` | A development dependency only: an independent FST reader used as the oracle for the VCD-to-FST transcode, so an emitted FST is read back and compared |
| **`blake3`** | `=1.8.2` | Digest for schema hashes and artifact headers. 1.8.3 and later move to edition 2024 |
| **`libm`** | vendored 0.2.16, `default-features = false` | Hardware float intrinsics are switched off so `f64` transcendentals are bit-identical on every IEEE-754 target. It lives at `third_party/libm` and is excluded from the workspace |
| **`cranelift-*`** | 0.120 | Only reachable through the `jit` feature, which is off by default. 0.120 is the newest line that builds on rustc 1.85 |

### A C linker

The final link step needs a system C linker. Most desktop installs already have one. On a bare
RHEL, UBI or Fedora image:

```sh
sudo dnf install -y gcc
```

---

## 1.3 Build from source

```sh
git clone https://github.com/tjddnr0912/vitamin-rtl-simulator
cd vitamin-rtl-simulator
cargo build --release --workspace --locked
```

`--locked` uses the committed `Cargo.lock`, which is what makes the build reproducible across
machines. Use it for every command.

A debug build of just the driver is faster to produce and is what the examples use:

```sh
cargo build -p cli
```

---

## 1.4 What the build produces

| Artifact | Built by | Notes |
|---|---|---|
| `target/release/vita` | `cargo build --release --workspace --locked` | The multicall binary. This is the whole product |
| `target/debug/vita` | `cargo build -p cli` | Same binary, debug profile. Considerably slower to run |
| `target/<profile>/{vcmp,velab,vrun}` | `cargo build --features separate-bins --locked` | Standalone per-stage executables, behind the development-only `separate-bins` feature. Each is a shim over the same multicall entry point, for debugging one stage in isolation |
| `target/<profile>/corpus-runner` | `cargo build -p corpus-runner` | A development tool that fetches, runs and grades the workload corpus. Not part of the product |

The default build emits exactly one executable, `vita`. The three staged names are reached
either through links to it or through its subcommand form; see §1.6.

---

## 1.5 Install

### With `install.sh`

The bundled [`install.sh`](../../install.sh) does both steps — install the binary, then create
the staged names next to it:

```sh
./install.sh
```

It runs from any working directory, because it resolves the repository root from its own
location. What it does, in order:

1. `cargo install --path crates/cli --locked`, which places `vita` in `~/.cargo/bin` (or
   `$CARGO_HOME/bin`).
2. Locates the installed binary with `command -v vita`, falling back to
   `${CARGO_HOME:-$HOME/.cargo}/bin/vita`. If neither is executable it prints
   `error: could not locate the installed 'vita' binary.` and exits 1.
3. For each of `vcmp`, `velab` and `vrun`, creates a symbolic link to `vita` in the same
   directory, printing `    linked  <name> -> vita`. If the filesystem rejects links it copies
   the binary instead and prints `    copied  <name> (link not supported on this filesystem)`.
   If both fail it prints `error: failed to create '<path>'.` and exits 1.
4. Prints `Done. Installed: vita, vcmp, velab, vrun  (in <dir>)` and a `PATH` hint.

The result is one real binary plus three names for it:

| Path | What it is |
|---|---|
| `~/.cargo/bin/vita` | The executable |
| `~/.cargo/bin/vcmp` | Link to `vita` |
| `~/.cargo/bin/velab` | Link to `vita` |
| `~/.cargo/bin/vrun` | Link to `vita` |

### By hand

Install the binary on its own:

```sh
cargo install --path crates/cli --locked
```

Only the `cli` crate produces an installable binary and the default build emits exactly one
`[[bin]]`, so no `--bin` selector is needed.

Straight from git, without a local checkout, name the package explicitly:

```sh
cargo install --git https://github.com/tjddnr0912/vitamin-rtl-simulator -p cli --locked
```

Then create the staged names yourself:

```sh
VITA="$(command -v vita)"
BIN="$(dirname "$VITA")"
for s in vcmp velab vrun; do
  ln -sf "$VITA" "$BIN/$s"
done
```

Symbolic links and hard links both work; dispatch reads the invocation name, not the inode.

---

## 1.6 The staged commands without links

`vita` decides which applet runs from the file stem of `argv[0]`, and failing that from the
first argument. So every stage is reachable from the plain `vita` name:

```sh
vita vcmp  design.sv      # -> design.vu
vita velab design.vu      # -> design.velab
vita vrun  design.velab   # -> waveform + stdout
```

`vita <stage>` consumes the stage token and forwards the remaining arguments unchanged. The
token has to be the first argument. This form is the fallback whenever links are unavailable or
the binary has been renamed.

---

## 1.7 PATH

`rustup` puts `~/.cargo/bin` on `PATH`, so a `cargo install` needs no further setup. If Rust was
installed another way and the command is not found:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Put that line in a shell profile to keep it. A system-wide `/usr/local/bin` install is not
required.

---

## 1.8 Verify

Check the binary answers:

```console
$ vita --version
vita 0.2.0
```

Run a design end to end:

```sh
cat > hello.sv <<'EOF'
`timescale 1ns/1ns
module tb;
  initial begin
    $display("hello from vitamin");
    $finish;
  end
endmodule
EOF

vita hello.sv
```

stdout carries the transcript and the closing summary line:

```text
hello from vitamin
simulation ended (Finish) at time 0
```

stderr carries the diagnostic counts:

```text
errors=0 warnings=0 notes=0
```

The exit code is 0. Dropping the `` `timescale `` line still runs, and adds one warning to
stderr:

```text
warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
```

The same design through the staged flow:

```sh
vita vcmp  hello.sv  -o hello.vu
vita velab hello.vu  -o hello.velab
vita vrun  hello.velab
```

The staged flow and the one-shot flow produce byte-identical stdout and byte-identical waveform
bytes for the same design; a test suite gates that equality.

### The test suite

Two runners cover the same tests and are not interchangeable — switching between them in one
session forces a rebuild, so pick one and stay on it.

| Command | Role | Result at HEAD |
|---|---|---|
| `cargo test --workspace --locked` | The suite CI runs | — |
| `cargo nextest run --workspace --locked` | The local full gate; reads `.config/nextest.toml`, which caps any single test at 60 s with four attempts | 7352 tests run, 7352 passed, 15 skipped, exit 0, 35.8 s |

The lint and format gates, which CI also enforces:

```sh
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for the contributor workflow.

---

## 1.9 Keeping `target/` small

Cargo writes a fresh hashed executable per test target per build and never reclaims the
superseded ones. The workspace carries 613 integration-test targets, so a repeated
`cargo test --workspace` accumulates a full set each time. One development machine measured
59 GiB across 357,247 files over two months, about 25 GiB per month.

```sh
cargo install cargo-sweep
cargo sweep --time 2      # -d for a dry run
```

A two-day retention keeps every current artifact: after a sweep that took one tree to 3.5 GiB,
the next `cargo build --workspace --locked` finished in 0.08 s with nothing to recompile.

Prefer this to `cargo clean`, which also discards artifacts that are still current and forces a
full rebuild. Do not sweep while a build or test run is in flight. Sweeping affects only the
build tree; installed binaries and simulation results are untouched.

---

Next: [Quickstart](002_quickstart.md) walks one design from source file to waveform.
