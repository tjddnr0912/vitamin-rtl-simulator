# vitamin

**vitamin** is an open-source RTL simulator written in Rust. It simulates the
full **Verilog-2005** synthesizable RTL subset plus a large **SystemVerilog**
subset — `logic`, `always_ff/comb/latch`, `enum` / `typedef` / packed `struct`,
functions/tasks, dynamic / queue / associative arrays, classes, and assertions
(SVA) — producing a textual `$display` transcript and a hierarchical **VCD**
(or **FST**) waveform.

Its goals are **determinism** (byte-identical output across Linux and macOS) and
a **clean, source-only `cargo` build** with no C/C++ dependencies.

```text
preprocess → lex → parse → elaborate → sim-ir → sim-engine → VCD
```

> **Status — `0.2.0`; actively developed.** The full pipeline works end-to-end in
> both one-shot and staged modes, spanning Phase-1 RTL, a broad SystemVerilog
> subset, and Phase-3 verification features (SVA, classes, constrained-random).
> Process bodies run on the **compiled `native` backend by default**; the
> interpreter and the bytecode VM remain behind the `oracle` feature as
> bisection oracles. The major version stays at `0` while
> [docs/ROADMAP.md](docs/ROADMAP.md) §2/§3 carry open correctness items — see
> [CHANGELOG.md](CHANGELOG.md).
> **6,000+ tests pass**; behaviour is checked against Icarus Verilog (`iverilog`)
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
pins **1.85** automatically):

```sh
cargo build --release --workspace --locked      # builds target/release/vita
./target/release/vita examples/000_counter.sv   # run a sample design
```

You will see the `$display` transcript on stdout and a `counter.vcd` waveform in
the current directory — open it in [GTKWave](https://gtkwave.sourceforge.net/) or
[Surfer](https://surfer-project.org/). To get an **FST** waveform instead, give
the output an `.fst` extension — `-o waves.fst` on the command line, or
`$dumpfile("waves.fst")` in the RTL (any other extension writes VCD). To install
the tools onto your `PATH`, run
`./install.sh` (or see [Installation](docs/manual/001_installation.md)).

Once the command lives in a Makefile, add `-v` and vitamin prints the *resolved*
invocation — every macro value, the files actually compiled, the runtime
plusargs, and where the thread count came from — at the top of the transcript,
so a `--log` file can answer "what did this run actually use?" on its own. See
[What actually ran](docs/manual/004_cli-reference.md#what-actually-ran--v).

A minimal design:

```systemverilog
module counter #(parameter WIDTH = 4) (input clk, rst, output reg [WIDTH-1:0] cnt);
  always @(posedge clk) if (rst) cnt <= 0; else cnt <= cnt + 1'b1;
endmodule
```

## Performance — what to expect

vitamin is an **event-driven, 4-state** simulator, the same class of tool as
Icarus Verilog. Against a **compiled, 2-state** simulator such as Verilator — or
a commercial simulator — it is **one to two orders of magnitude slower**. That
gap is structural rather than a tuning bug: Verilator buys its speed by giving
up 4-state semantics and event ordering, and vitamin deliberately does not (see
[docs/study/01](docs/study/01-interpreted-vs-compiled.md)).

Measured on Keccak-f[1600], 2 000 permutations, macOS arm64, release builds,
interleaved samples with the first round discarded:

| Simulator | Per permutation | Relative |
|---|---|---|
| Verilator 5.050 (compiled, 2-state) | **7.0 µs** | 1× |
| vitamin — design with no subroutine calls | 305 µs | 44× slower |
| vitamin — design with function/task calls | 2 225 µs | 320× slower |
| Icarus Verilog 13 | 4 670 µs | 672× slower |

Against Icarus Verilog specifically, vitamin is ahead — a geometric mean of
**1.9× faster** across five third-party designs in the workload corpus, and
**1.5×** across all eight that run, with one design still meaningfully slower
([docs/study/03](docs/study/03-workload-corpus.md)). Function and task calls
remain the biggest cost a design can pay: the same algorithm written without
subroutines runs **7.3× faster** than the version that calls them, and closing
that gap is an active work item.

### Use it for fast verification, not for regression farms

Running a full regression suite under vitamin is generally impractical today.
Get value out of it the other way around — as the **quick-check** tool in the
loop, where its determinism and its correct-or-loud rule matter most:

- Pick **representative cases** rather than the whole suite — one directed test
  per feature, a short randomized burst, a smoke test of a few hundred cycles.
- Keep the cycle count to what the check actually needs; simulated time is the
  dominant cost, so a 10× shorter run really is 10× cheaper.
- Use the staged flow (`vcmp` → `velab` → `vrun`) when you re-run one design
  repeatedly: compile and elaborate once, then only pay for `vrun`.
- Dump waveforms only when you intend to look at them, and prefer FST
  (`-o waves.fst`) for large runs.
- Reach for Verilator or a commercial simulator for long soak runs, large
  randomized regressions, and full-chip workloads.

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

See [CONTRIBUTING.md](CONTRIBUTING.md) for the toolchain pins (MSRV 1.85, edition
2021, `--locked`) and the determinism rules that keep builds reproducible.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any additional
terms or conditions.
