# bench/aes — secworks AES core (Verilog-2005) differential benchmark

Hardened 2026-08-23. Everything below was re-measured, not inherited from the scout.

## Provenance

| | |
|---|---|
| repo | https://github.com/secworks/aes |
| pinned SHA | `80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc` |
| license | **BSD-2-Clause** (`src/LICENSE`, Copyright (c) 2014 Joachim Strömbergson) — redistribution OK with notice |
| checkout path | `bench/aes/src/` (gitignored) |
| language | pure Verilog-2005 RTL; every RTL file opens with `` `default_nettype none `` |
| RTL LOC fed to the simulators | **2580** (6 upstream files + local `tb.v`) |

Re-clone from scratch:

```sh
cd bench/aes
git clone https://github.com/secworks/aes src
git -C src checkout 80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc
```

## File list (exact, in order)

Paths are relative to `bench/aes/`. Order matters only in that `tb.v` is last; all three tools accept this order.

```
src/src/rtl/aes_core.v            342
src/src/rtl/aes_encipher_block.v  487
src/src/rtl/aes_decipher_block.v  526
src/src/rtl/aes_key_mem.v         434
src/src/rtl/aes_sbox.v            327
src/src/rtl/aes_inv_sbox.v        325
tb.v                              139   (written here, NOT upstream)
                                 ----
                                 2580
```

`src/src/rtl/aes.v` (the 273-line memory-mapped register-file wrapper) is deliberately **excluded** — it only adds a bus interface that would need a bus-transaction testbench. `aes_core` is the whole crypto datapath.

Top module: **`tb`**.

## Testbench

`tb.v` is ours, not upstream (upstream `tb_aes_core.v` is a fixed 4-vector NIST check with no accumulator — far too short). It runs `N` iterations; each iteration re-runs the full key schedule with `keylen` alternating 128/256-bit (`i[0]`), encrypts a chained block, decrypts the ciphertext straight back, and folds both results into one 128-bit digest with a rotate between them so the accumulation is order-sensitive. Fully deterministic: no `$random`, no `$time` in the digest. Terminates via explicit `$finish`; a `#500000000` watchdog prints `WATCHDOG` if it ever hangs (never reached).

## Workload size (tuned)

**`+N=200`** — iverilog median **6.18 s**, inside the 3–15 s target window.

Runtime is linear in N and the digest is a function of N, so the plusarg genuinely scales the run. Measured (single runs, vvp-only vs vita, interleaved):

| N | iverilog (vvp) | vita | ratio | digest |
|---|---|---|---|---|
| 4 | 0.169 s | 0.119 s | 1.42× | `ed74b6a3f9e0b82bb8b13a8a307aeacd` |
| 50 | 1.571 s | 0.930 s | 1.69× | `976b079e54a6b9a5ae6fcaaa31eb8adb` |
| 100 | 3.103 s | 1.801 s | 1.72× | `2b8a9ff8c9c4f7cb0e5fe9c367564465` |
| **200** | **6.117 s** | **3.537 s** | **1.73×** | **`cfaa46dd896b2275ade662d344f5e251`** |
| 400 | 12.210 s | 7.122 s | 1.71× | `c2b9153026200e41982dc57e79417472` |

**iverilog and vita agree at all five sizes.** N=100 (~3.1 s) is the fast alternative if 6 s is too slow for a per-commit gate.

## Exact command lines

Run from `bench/aes/`. Use `/bin/sh`, **not zsh** — zsh does not word-split an unquoted `$F`, which makes every tool see the whole file list as one filename.

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

`./run.sh {iverilog|vita|verilator}` (honours `N=<count>`) does exactly this.

## Expected output

**DIGEST — byte-identical in all three tools:**

```
DIGEST=cfaa46dd896b2275ade662d344f5e251
```

Verbatim tail of each tool at `+N=200`:

```
iverilog : DIGEST=cfaa46dd896b2275ade662d344f5e251
           .../bench/aes/tb.v:134: $finish called at 286060000 (1ps)
vita     : DIGEST=cfaa46dd896b2275ade662d344f5e251
           simulation ended (Finish) at time 286060000
           errors=9 warnings=10 notes=0
verilator: DIGEST=cfaa46dd896b2275ade662d344f5e251
```

All three also agree on end-of-sim time: **286060000** in 1 ps units = 286.06 µs.

### ⚠ vita exits **1** while printing the correct answer

This is the single thing a harness must special-case. **Gate on the DIGEST line, not on the exit code.**

vita emits, verbatim (first of each class):

```
warning[VITA-W1018] W-PP-TIMESCALE-MIXED: some modules have a `timescale and these do not: aes_core, aes_decipher_block, aes_encipher_block, aes_inv_sbox, aes_key_mem, aes_sbox — IEEE 1800 §3.14.2.2 requires all or none, and other tools refuse to elaborate the mixed form (they take the 1ns/1ns base here)
warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `tb.dut.dec_block.inv_sbox_inst.inv_sbox` is unknown (x/z); read X / write ignored [at time 0]
warning[VITA-W4029] W-RUN-RANGE-UNKNOWN: array word index of `tb.dut.sbox_inst.sbox` is unknown (x/z); read X / write ignored [at time 0]
error[VITA-E4002] E-RUN-RANGE: array word index of `tb.dut.keymem.key_mem` (out of range; read X / write ignored) [at time 2175000]
```

9 errors (8 shown + a suppression line), 10 warnings. The full stdout is byte-stable across runs.

Confirmed directly: `./run.sh vita >/dev/null 2>&1; echo $?` prints `1`; the iverilog arm prints `0`. Beware that piping vita's output (e.g. `./run.sh vita | tail`) makes `$?` the *pipe tail's* status and silently hides the 1.

**The out-of-range read is real RTL behaviour, not a vita indexing bug.** `aes_key_mem.v:77` declares `reg [127:0] key_mem [0:14]` and reads it at `:182` as `tmp_round_key = key_mem[round]`, where `round` comes from a 4-bit counter (`aes_decipher_block.v:201` `reg [3:0] round_ctr_reg`, decremented at `:440` by `round_ctr_reg - 1'b1`). Decrementing past 0 wraps `4'h0 → 4'hF`, driving index 15 at a memory declared `0:14`. iverilog silently returns `128'hx`; verilator ignores it; vita calls it an error. The X never reaches the result (the decipher datapath is idle on that cycle), which is why all three digests agree.

IEEE 1800 §7.4.6 says an out-of-range **read** returns x and does **not** require an error. Whether E4002 deserves error severity on a read (as opposed to a write) is an open product question — this design is the standing example.

The W1018 timescale warning is also legitimate: `tb.v` has `` `timescale 1ns/1ps `` and none of the six upstream RTL files do.

## Median timings (2026-08-23, this machine)

Method: **interleaved** iverilog → vita, four rounds, **first pair discarded**, median of rounds 2–4. Both binaries are release builds. (Sequential A-then-B measurement is what produces fake deltas from cache warming — do not do it that way.)

| round | iverilog compile | iverilog run | iverilog total | vita |
|---|---|---|---|---|
| 1 (discarded) | 0.064 | 6.010 | 6.074 | 3.468 |
| 2 | 0.057 | 5.985 | 6.042 | 3.496 |
| 3 | 0.058 | 6.124 | 6.182 | 3.529 |
| 4 | 0.057 | 6.136 | 6.193 | 3.550 |
| **median (2–4)** | **0.057** | **6.124** | **6.182** | **3.529** |

**iverilog 6.18 s vs vita 3.53 s → vita is 1.75× faster.** Spread across rounds 2–4 is under 2.5 % for both tools, so the ratio is solid.

verilator, separately (clean `rm -rf obj_dir` rebuild): **build 8.39 s**, then run 0.387 s / 0.052 s / 0.048 s → **median warm run 0.050 s**. The first run being 8× the warm runs is cold page-cache on the freshly-linked binary; a single verilator timing will overstate sim cost by ~8×.

## Determinism

Verified, not assumed. Across the four interleaved rounds the **entire captured stdout+stderr** (not just the digest line) is byte-identical:

- iverilog: `md5 = b503b59dcd1f5ae256dea756e8bc7af0` × 4
- vita: `md5 = 6c0c156b282e442f4b190489c9e9ef01` × 4

vita's diagnostic stream — including the interleaving of the 9 errors, their timestamps, and the suppression lines — is stable run to run.

## What this benchmark is good for

1. A clean ~2.6 kLOC pure-Verilog-2005 differential workload where vita is byte-correct against two independent oracles at five workload sizes.
2. A standing correct-or-loud example: vita returns a nonzero exit status on a design every other simulator accepts silently, while producing the right answer.
3. A different stress profile from keccak/picorv32 — two 256-entry sbox case statements plus wide always blocks make it event-heavy per cycle rather than cycle-heavy. iverilog needs 6 s for only ~28,600 clock cycles.
