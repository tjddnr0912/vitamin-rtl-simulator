# bench/keccak — workload corpus recipe

Two corpus rows share this directory, because the two designs they name differ in
exactly one thing. `keccak` compiles `keccak_f.sv`, `keccak-arr` compiles
`keccak_f_arr.sv`, and both print the same line. The RTL itself is described in
[README.md](README.md); the cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md), and
`cargo run -p corpus-runner -- run --filter keccak --compare` reproduces it for both
rows at once.

| | |
|---|---|
| Origin | first-party — written in this repository and committed, so there is nothing to fetch |
| Licence | this repository's |
| Top module | `tb`, in `tb.sv` |
| Workload | `+N=2000` permutations |
| Expected | `perms=2000 lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde` (both rows) |
| Oracle | Icarus Verilog 13.0, Verilator 5.050 and a Python reference all agree |
| Anchor | at `+N=1` the state starts all-zero, so the first lane is the published Keccak value `f1258f7940e1dde7` — the agreement is anchored, not merely mutual |
| Lines fed to the simulators | 134 (`tb.sv` 38 + `keccak_f.sv` 96) · 135 for the `-arr` row |

## Commands (from `bench/keccak/`)

```sh
../../target/release/vita tb.sv keccak_f.sv     +N=2000
../../target/release/vita tb.sv keccak_f_arr.sv +N=2000

iverilog -g2012 -o keccak.vvp     tb.sv keccak_f.sv     && vvp keccak.vvp     +N=2000
iverilog -g2012 -o keccak-arr.vvp tb.sv keccak_f_arr.sv && vvp keccak-arr.vvp +N=2000
```

## Expected output

Both rows, both tools, verbatim:

```
vita:
perms=2000 lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde
simulation ended (Finish) at time 520025000
errors=0 warnings=0 notes=0

iverilog:
perms=2000 lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde
tb.sv:36: $finish called at 520025000 (1ps)
```

Verilator prints the same digest line and `$finish at 520us`.

vita exits 0 with no diagnostics at all — no `` `timescale `` warning either, because
`tb.sv` carries one.

## The digest moves with the workload

Each `+N` has its own lanes and its own accumulator, and all four tools agree at each
of them:

| `+N=` | lane0 | acc |
|---|---|---|
| 1 | `f1258f7940e1dde7` | `584777e2b448a416` |
| 200 | `001a21c22caa6aa0` | `597b2c64edb7b9d7` |
| 2000 (the pinned workload) | `54aa20c46ef0e0f6` | `767c5ab6776c4bde` |

Distinct output at three sizes is the evidence that `$value$plusargs("N=%d", nperm)` is
honoured rather than silently ignored in favour of the built-in default of 100.

## Finish time is part of the agreement

All three tools end at simulated time 520025000 (285000 at `+N=1`, 2625000 at `+N=10`):
one clock to latch `start`, 24 rounds, one clock for the testbench to observe `done`,
per permutation. The testbench drives `start` and `rst_n` with non-blocking assignments
so that a write made in the same time step as a `posedge clk` is never sampled by the
core in that same edge. With blocking writes there the outcome is an IEEE 1800 §4.7
process-ordering race: a simulator that runs the testbench's continuation before the
core's `always` block sees the new `start` one clock earlier than one that runs the
core first, and both orders are conformant. The digest is the same either way; only
the finish time moves, by one clock period per permutation.

## Determinism

Repeated runs of either row produce byte-identical stdout, not merely the same digest
line. The two tools' transcripts differ only in how they word the end of the run.

## Not a corpus row: `keccak_f_flat.sv`

`gen_flat.py` generates `keccak_f_flat.sv` from the same algorithm with every user
call expanded inline. It prints the same line as the other two at every `N` and is
excluded from the manifest on purpose: it exists as the flat half of the call-regime
pair, which is a measurement rather than a workload
([study 01](../../docs/study/01-interpreted-vs-compiled.md) §2.4).
