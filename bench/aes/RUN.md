# bench/aes — secworks AES core

The corpus row `aes`: secworks' AES encrypt/decrypt datapath driven through N chained
key-schedule + encrypt + decrypt rounds, reduced to one `DIGEST=` line. This is the
row where vita prints the correct answer and still exits 1. The cross-corpus timing
table is [docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter aes --compare` reproduces it.

## Provenance

| | |
|---|---|
| Repo | https://github.com/secworks/aes |
| Pinned SHA | `80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc` |
| Licence | BSD-2-Clause (`src/LICENSE`, Copyright (c) 2014 Joachim Strömbergson) — redistribution permitted with notice |
| Clone path | `bench/aes/src/` (gitignored) |
| Language | pure Verilog-2005 RTL; every upstream file opens with `` `default_nettype none `` |
| Lines fed to the simulators | 2580 (six upstream files plus the local `tb.v`) |
| Top module | `tb` |

Reconstruct the clone:

```sh
cd bench/aes
git clone https://github.com/secworks/aes src
git -C src checkout 80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc
```

## File list (exact, in order)

Paths are relative to `bench/aes/`. Order matters only in that `tb.v` is last; all
three tools accept this order.

```
src/src/rtl/aes_core.v            342
src/src/rtl/aes_encipher_block.v  487
src/src/rtl/aes_decipher_block.v  526
src/src/rtl/aes_key_mem.v         434
src/src/rtl/aes_sbox.v            327
src/src/rtl/aes_inv_sbox.v        325
tb.v                              139   (written here, not upstream)
                                 ----
                                 2580
```

`src/src/rtl/aes.v`, the 273-line memory-mapped register-file wrapper, is excluded on
purpose: it only adds a bus interface, which would need a bus-transaction testbench.
`aes_core` is the whole crypto datapath.

## Commands

Run from `bench/aes/`, and use `/bin/sh` rather than zsh — zsh does not word-split an
unquoted `$F`, which makes every tool see the whole file list as one filename.

```sh
D=$PWD
F="$D/src/src/rtl/aes_core.v $D/src/src/rtl/aes_encipher_block.v $D/src/src/rtl/aes_decipher_block.v $D/src/src/rtl/aes_key_mem.v $D/src/src/rtl/aes_sbox.v $D/src/src/rtl/aes_inv_sbox.v $D/tb.v"

# iverilog (compile + run)
iverilog -g2012 -o x.vvp $F && vvp x.vvp +N=200

# vita (one-shot)
../../target/release/vita $F +N=200

# verilator (build + run)
verilator --binary --timing -Wno-fatal -o vsim --top-module tb $F && ./obj_dir/vsim +N=200
```

`./run.sh {iverilog|vita|verilator}` does exactly this and honours `N=<count>`.

## Expected output

At `+N=200` the digest is byte-identical in all three tools:

```
DIGEST=cfaa46dd896b2275ade662d344f5e251
```

Verbatim tail of each tool:

```
iverilog : DIGEST=cfaa46dd896b2275ade662d344f5e251
           .../bench/aes/tb.v:134: $finish called at 286060000 (1ps)
vita     : DIGEST=cfaa46dd896b2275ade662d344f5e251
           simulation ended (Finish) at time 286060000
           errors=9 warnings=10 notes=0
verilator: DIGEST=cfaa46dd896b2275ade662d344f5e251
```

All three agree on the end-of-simulation time as well: 286060000 in 1 ps units,
286.06 µs.

## vita exits 1 while printing the correct answer

This is the one thing a harness must special-case here. Gate on the `DIGEST=` line,
not on the exit code. The corpus manifest pins the exit code (`Runs { exit: 1 }`)
rather than grading the workload as a crash, so the day the severity changes the
corpus notices.

vita emits, first of each class:

```
warning[VITA-W1018] W-PP-TIMESCALE-MIXED: some modules have a `timescale and these do not: aes_core, aes_decipher_block, aes_encipher_block, aes_inv_sbox, aes_key_mem, aes_sbox — IEEE 1800 §3.14.2.2 requires all or none, and other tools refuse to elaborate the mixed form (they take the 1ns/1ns base here)
warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `tb.dut.dec_block.inv_sbox_inst.inv_sbox` is unknown (x/z); read X / write ignored [at time 0]
warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `tb.dut.sbox_inst.sbox` is unknown (x/z); read X / write ignored [at time 0]
src/src/rtl/aes_key_mem.v:182:7: error[VITA-E4002] E-RUN-RANGE: array word index of `tb.dut.keymem.key_mem` (out of range; read X / write ignored) [in tb.dut.keymem.key_mem_read] [at time 2175000]
```

Nine errors (eight sites plus a suppression line) and ten warnings. The full stderr is
byte-stable across runs, including the interleaving of the errors and their timestamps.

Beware that piping vita's output — `./run.sh vita | tail` — makes `$?` the pipe tail's
status and hides the 1. `./run.sh vita >/dev/null 2>&1; echo $?` prints `1`; the
iverilog arm prints `0`.

The out-of-range read is real RTL behaviour rather than a vita indexing defect.
`aes_key_mem.v:77` declares `reg [127:0] key_mem [0:14]` and `aes_key_mem.v:182` reads
it as `tmp_round_key = key_mem[round]`, where `round` comes from a 4-bit counter
(`aes_decipher_block.v:201`, `reg [3:0] round_ctr_reg`) decremented at
`aes_decipher_block.v:440` by `round_ctr_reg - 1'b1`. Decrementing past 0 wraps
`4'h0 → 4'hF`, driving index 15 at a memory declared `0:14`. iverilog returns `128'hx`
silently and verilator ignores it; vita calls it an error. The X never reaches the
result — the decipher datapath is idle on that cycle — which is why all three digests
agree.

IEEE 1800 §7.4.6 says an out-of-range *read* returns x and does not require an error.
Whether `VITA-E4002` deserves error severity on a read as opposed to a write is an
open product question, and this design is the standing example.

The `VITA-W1018` timescale warning is legitimate too: `tb.v` carries
`` `timescale 1ns/1ps `` and none of the six upstream RTL files do.

## Testbench

`tb.v` is written for this bench. Upstream's `tb_aes_core.v` is a fixed four-vector
NIST check with no accumulator, far too short to be a workload.

Each of the N iterations re-runs the full key schedule with `keylen` alternating
128-bit and 256-bit on `i[0]`, encrypts a chained block, decrypts the ciphertext
straight back, and folds both results into one 128-bit digest with a rotate between
them so the accumulation is order-sensitive. There is no `$random` and no `$time` in
the digest. Termination is an explicit `$finish`; a `#500000000` watchdog prints
`WATCHDOG` if the run ever hangs, and never fires.

## The digest moves with the workload

Runtime is linear in N and the digest is a function of N, so the plusarg genuinely
scales the run. iverilog and vita agree at every size:

| `+N=` | DIGEST |
|---|---|
| 4 | `ed74b6a3f9e0b82bb8b13a8a307aeacd` |
| 50 | `976b079e54a6b9a5ae6fcaaa31eb8adb` |
| 100 | `2b8a9ff8c9c4f7cb0e5fe9c367564465` |
| 200 (the pinned workload) | `cfaa46dd896b2275ade662d344f5e251` |
| 400 | `c2b9153026200e41982dc57e79417472` |

`+N=200` puts the iverilog reference in the middle of the 3–15 s band. `+N=100` is the
fast alternative if that is too slow for a per-commit gate.

## Determinism

The entire captured stdout and stderr of each tool — not just the digest line — is
byte-identical across repeated runs, vita's diagnostic stream included.

## What this workload covers

- A clean ~2.6 kLOC pure-Verilog-2005 differential workload where vita is byte-correct
  against two independent oracles at five workload sizes.
- The standing correct-or-loud example: vita returns a nonzero exit status on a design
  every other simulator accepts silently, while producing the right answer.
- A different stress profile from the CPU rows: two 256-entry sbox case statements plus
  wide always blocks make it event-heavy per cycle rather than cycle-heavy. iverilog
  needs seconds for only ~28,600 clock cycles.

## Verilator build cost

A clean `rm -rf obj_dir` verilator rebuild dominates the first measurement, and the
first run of the freshly linked binary is several times the warm runs because of cold
page cache. A single verilator timing overstates simulation cost accordingly; time the
warm runs.
