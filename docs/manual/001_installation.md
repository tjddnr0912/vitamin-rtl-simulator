# 1. Installation

vitamin is an open-source RTL simulator written in Rust. It ships as a single
binary, `vita`, which acts as the one-shot driver and also dispatches the staged
commands `vcmp` / `velab` / `vrun` by the name it is invoked under.

> **Supported platforms: Linux and macOS only.**
> Windows is **not** currently supported — it is not a build target in
> `rust-toolchain.toml` and is not exercised in CI. There are no Windows install
> steps; do not expect a Windows build to work.

The supported targets are:

- `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`, `aarch64-apple-darwin` (Intel + Apple Silicon)

vitamin is **built from source on the target machine** — there is no vendor
prebuilt binary. `cargo` is the only build entry point: no `cmake`, no `make`, no
`build.rs` shell-out.

---

## 1.1 Prerequisites

You need a Rust toolchain. vitamin pins **Rust 1.82** (the MSRV) via
`rust-toolchain.toml` at the repository root, so once Rust is installed through
`rustup`, the correct version is selected **automatically** when you run any
`cargo` command inside the checkout — you do not need to pick a version by hand.

Install `rustup` (same command on Linux and macOS):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Then restart your shell (or `source "$HOME/.cargo/env"`) so `cargo` is on your
`PATH`. The first `cargo` invocation in the repo downloads and uses the pinned
1.82.0 toolchain on its own.

On a fresh RHEL/Fedora box you may also need a C linker for the final link step:

```sh
sudo dnf install -y gcc
```

---

## 1.2 Build from source

Clone the repository and build the whole workspace in release mode. `--locked`
uses the committed `Cargo.lock` so the build is reproducible across machines:

```sh
git clone https://github.com/your-org/vitamin
cd vitamin
cargo build --release --workspace --locked
```

This produces the single multicall binary at `target/release/vita`.

To run the test suite (the deterministic golden gate is part of it):

```sh
cargo test --workspace --locked
```

`cargo run` from the checkout is a contributor workflow, not the end-user path —
the intended flow is to **install** the binary (next section) and invoke it as a
terminal command.

---

## 1.3 Install

Install `vita` into `~/.cargo/bin` (which `rustup` already added to your `PATH`):

```sh
cargo install --path crates/cli --locked
```

Only the `cli` crate produces an installable binary, and the default build emits
exactly one `[[bin]]` (`vita`), so this is unambiguous — no `--bin` selector is
needed.

If you install straight from git instead of a local checkout, select the package
explicitly:

```sh
cargo install --git https://github.com/your-org/vitamin -p cli --locked
```

### The multicall link farm — `vcmp` / `velab` / `vrun`

`vita` is a **multicall** binary: it inspects the basename of `argv[0]` and
dispatches accordingly — invoked as `vcmp`/`velab`/`vrun` it runs that stage;
invoked as anything else (`vita`) it runs the one-shot pipeline. So the three
staged commands are just **links to the same `vita` binary** under different
names. After `cargo install`, create them next to the installed `vita`:

```sh
VITA="$(command -v vita)"
BIN="$(dirname "$VITA")"
for s in vcmp velab vrun; do
  ln -sf "$VITA" "$BIN/$s"   # symlink; falls back to a copy if your FS rejects links
done
```

Hardlinks (`ln -f`) are equivalent and let the linked names share one signed
binary; symlinks are simpler and portable across filesystems. Either works — the
dispatch is driven purely by the invocation name.

The bundled [`install.sh`](../../install.sh) automates both steps: it runs the
`cargo install` above, then creates the `vcmp`/`velab`/`vrun` links (symlink
first, copy as a fallback).

> If the binary is ever renamed or its `argv[0]` is otherwise mangled so the
> basename is no longer recognized, you can still reach a stage explicitly:
> `vita vcmp …`, `vita velab …`, `vita vrun …`.

---

## 1.4 PATH

`rustup` puts `~/.cargo/bin` on your `PATH`, so a `cargo install` build needs no
extra setup. If you installed Rust another way and the command is not found, add
it yourself:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

(Put that line in your shell profile to make it permanent.) A system-wide
`/usr/local/bin` install is not required.

---

## 1.5 Verify

A quick smoke test — write a trivial testbench and run it one-shot:

```sh
cat > hello.sv <<'EOF'
module tb;
  initial begin
    $display("hello from vitamin");
    $finish;
  end
endmodule
EOF

vita hello.sv
```

You should see `hello from vitamin` on stdout and a clean exit (code 0). The
staged flow over the same design:

```sh
vcmp  hello.sv -o hello.vu        # compile  → .vu
velab hello.vu -o hello.velab     # elaborate → .velab
vrun  hello.velab                 # simulate → VCD + stdout
```

---

Next: see the sibling chapters for usage of the one-shot driver and the staged
`vcmp → velab → vrun` flow. Back to [Installation](001_installation.md).
