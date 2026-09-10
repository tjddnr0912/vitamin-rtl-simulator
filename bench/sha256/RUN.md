# bench/sha256 — reproducible recipe

The corpus row `sha256`: secworks' SHA-256 core driven through N chained block
compressions, reduced to one `DIGEST=` line that three simulators agree on.
The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter sha256 --compare` reproduces it.

## Source

| | |
|---|---|
| Repo | https://github.com/secworks/sha256 |
| Pinned SHA | `837c5cc396f001d18f2c765721c585716eb439ae` |
| Licence | BSD-2-Clause — Copyright (c) 2013 Joachim Strömbergson. Permissive, no patent or OHL rider. Full text at `src/LICENSE` |
| Clone path | `bench/sha256/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty |
| Top module | `tb`, in `bench/sha256/tb.v` (written for this bench, not upstream) |
| Lines fed to the simulators | 1022 = 929 upstream RTL + 93 harness |

Reconstruct the clone:

```sh
cd bench/sha256
git clone https://github.com/secworks/sha256 src
git -C src checkout 837c5cc396f001d18f2c765721c585716eb439ae
```

## File list (exact, in order)

Paths are relative to `bench/sha256/`. Order matters only in that `tb.v` comes first;
all four go on one command line.

```
tb.v
src/src/rtl/sha256_core.v
src/src/rtl/sha256_w_mem.v
src/src/rtl/sha256_k_constants.v
```

## Commands (from `bench/sha256/`)

Icarus Verilog (the oracle):

```sh
iverilog -g2012 -o x.vvp tb.v src/src/rtl/sha256_core.v src/src/rtl/sha256_w_mem.v src/src/rtl/sha256_k_constants.v \
  && vvp x.vvp +N=2000
```

vita (one-shot, release binary):

```sh
../../target/release/vita tb.v src/src/rtl/sha256_core.v src/src/rtl/sha256_w_mem.v src/src/rtl/sha256_k_constants.v +N=2000
```

Verilator (optional third opinion):

```sh
verilator --binary --timing -Wno-fatal -o vsim tb.v src/src/rtl/sha256_core.v src/src/rtl/sha256_w_mem.v src/src/rtl/sha256_k_constants.v \
  && ./obj_dir/vsim +N=2000
```

`./run.sh` wraps all three — `./run.sh iverilog`, `./run.sh verilator`, or `./run.sh`
for vita — with `N` overridable from the environment.

> zsh trap. Do not put the file list in a plain variable and write `vita $F`. zsh does
> not word-split an unquoted parameter expansion, so all four paths arrive as one argv
> entry and vita reports `error[VITA-E8005]: cannot read 'tb.v src/... src/... src/...'`.
> That reads exactly like a silent vita failure and is not one. Use an array, `${=F}`,
> or `/bin/sh`.

## Expected output

At `+N=2000`, both simulators, verbatim:

```
iverilog:
DIGEST=e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724
/…/bench/sha256/tb.v:90: $finish called at 536010000 (1ps)

vita:
DIGEST=e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724
simulation ended (Finish) at time 536010000
errors=0 warnings=0 notes=0
```

vita exits 0 with no diagnostics at all. Both tools end at the same simulated time,
`536010000` in 1 ps units.

## The digest moves with the workload

All three tools agree at each size, and each size has its own digest:

| `+N=` | DIGEST |
|---|---|
| 500 | `8391db712c0429b5d50be2ffcc3491d573f000da150bccb47194986d132dd63d` |
| 2000 (the pinned workload) | `e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724` |
| 4000 | `dfb4a236ae060ca2a05570faee970a77e9379dba0ee24abe4252191e957e0876` |

Three distinct digests for three `+N` values is the evidence that
`$value$plusargs("N=%d", N)` is honoured rather than silently ignored in favour of the
built-in default. Running with no plusarg reproduces the N=2000 digest, so the
`$value$plusargs` false branch is correct too.

## Workload tuning

`+N=2000` puts the iverilog reference inside the 3–15 s band with startup under 1% of
the run, which is what makes it cheap enough for every corpus pass. Scaling is linear
and measured rather than extrapolated: `+N=4000` roughly doubles the run and `+N=500`
falls below the 3 s floor, so it is not a usable default.

## Testbench

`tb.v` is written for this bench. Upstream's `src/src/tb/tb_sha256_core.v` is a
pass/fail counter over four NIST vectors: it terminates in milliseconds and prints no
digest, so it is a conformance check rather than a workload.

`tb.v` instantiates `sha256_core` directly and chains N block compressions. Each
iteration presents `block = {chain, ~chain}`, alternates SHA-256/SHA-224 on `i[0]`,
waits for `ready`, xor-accumulates the digest into `acc`, then rotates the digest by
64 bits and xors in the loop counter to form the next `chain`. The feedback is
data-dependent, so a single wrong bit in any round of any block propagates into the
final `DIGEST=` line: this is a differential gate, not a stopwatch. There is no
`$random` and no `$time` in the digest. A watchdog (`#40000000; $display("WATCHDOG");
$finish;`) gives a deterministic hard stop; it never fires, since the run ends at
536.01 µs.

## Determinism

The entire stdout of each tool is byte-identical across repeated runs at `+N=2000`,
not just the `DIGEST=` line. The two tools' full transcripts differ only in how they
word their `$finish` banner.

## Verilator finish-time quirk

Verilator reports `$finish at 528us` where iverilog and vita both report 536.01 µs.
The 8.01 µs gap is exactly 2000 × 4 ns, one clock period per loop iteration:
Verilator's `--timing` leaves the `while (!tb_ready) @(posedge clk)` wait one edge
earlier than the two event-driven simulators. The digest is unaffected at every size
tested, so this is a scheduling nuance in Verilator's timing model rather than a
datapath divergence, and vita sides with iverilog exactly. Recorded because a gate
comparing finish *time* across all three tools would false-positive here; the corpus
compares the digest.
