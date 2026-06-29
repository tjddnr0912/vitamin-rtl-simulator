# vitamin

**vitamin** is an open-source RTL simulator written in Rust. It simulates the
full **Verilog-2005** synthesizable RTL subset plus a large **SystemVerilog**
subset — `logic`, `always_ff/comb/latch`, `enum` / `typedef` / packed `struct`,
functions/tasks, dynamic / queue / associative arrays, classes, and assertions
(SVA) — producing a textual `$display` transcript and a hierarchical **VCD**
waveform.

Its goals are **determinism** (byte-identical output across Linux and macOS) and
a **clean, source-only `cargo` build** with no C/C++ dependencies.

```text
preprocess → lex → parse → elaborate → sim-ir → sim-engine → VCD
```

> **Status — actively developed (`0.0.0`).** The full pipeline works end-to-end
> in both one-shot and staged modes, spanning Phase-1 RTL, a broad SystemVerilog
> subset, and Phase-3 verification features (SVA, classes, constrained-random).
> **2,600+ tests pass**; behaviour is checked against Icarus Verilog (`iverilog`)
> by live differential review under a strict **correct-or-loud** rule — the
> simulator never produces a silently wrong result, and anything unsupported is
> an explicit diagnostic. Platforms: **Linux and macOS** (Windows is not
> currently supported).

## The four CLIs

`vita` is a single multicall binary; `vcmp`/`velab`/`vrun` are the same binary
dispatched by name (or `vita <sub>`).

| Command | Role |
|---------|------|
| `vita design.sv` | **one-shot** — compile + elaborate + simulate → VCD + stdout |
| `vcmp` | compile sources → `.vu` artifact |
| `velab` | elaborate → `.velab` artifact |
| `vrun` | simulate a `.velab` → VCD + stdout |

## Quick start

Build from a clone of this repository (needs a Rust toolchain — `rust-toolchain.toml`
pins **1.82** automatically):

```sh
cargo build --release --workspace --locked      # builds target/release/vita
./target/release/vita examples/000_counter.sv   # run a sample design
```

You will see the `$display` transcript on stdout and a `counter.vcd` waveform in
the current directory — open it in [GTKWave](https://gtkwave.sourceforge.net/) or
[Surfer](https://surfer-project.org/). To install the tools onto your `PATH`, run
`./install.sh` (or see [Installation](docs/manual/001_installation.md)).

A minimal design:

```systemverilog
module counter #(parameter WIDTH = 4) (input clk, rst, output reg [WIDTH-1:0] cnt);
  always @(posedge clk) if (rst) cnt <= 0; else cnt <= cnt + 1'b1;
endmodule
```

## Documentation

The **user manual** lives in [`docs/manual/`](docs/manual/), in reading order:

| # | Chapter |
|---|---------|
| 0 | [Introduction](docs/manual/000_introduction.md) |
| 1 | [Installation](docs/manual/001_installation.md) |
| 2 | [Quick Start](docs/manual/002_quickstart.md) |
| 3 | [Language Reference](docs/manual/003_language-reference.md) — the supported RTL subset |
| 4 | [CLI Reference](docs/manual/004_cli-reference.md) |
| 5 | [System Tasks](docs/manual/005_system-tasks.md) |
| 6 | [Limitations](docs/manual/006_limitations.md) |
| 7 | [Error Codes](docs/manual/007_error-codes.md) |

- Runnable [`examples/`](examples/) — counter, ALU, an `enum`-based FSM, a shift register.
- The authoritative design specification is in [`docs/preview/`](docs/preview/) (developer-internal).
- The engineering tracker is [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md).

## Building & contributing

```sh
cargo build  --workspace --locked
cargo test   --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the toolchain pins (MSRV 1.82, edition
2021, `--locked`) and the determinism rules that keep builds reproducible.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.
