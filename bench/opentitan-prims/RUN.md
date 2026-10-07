# bench/opentitan-prims — seven OpenTitan primitives

The corpus row `opentitan-prims`: eleven unmodified upstream files from lowRISC's OpenTitan
driving seven cryptographic and integrity primitives with a pseudo-random vector stream,
reduced to one `DIGEST=` line. It is constant-heavy SystemVerilog — packages of packed-array
parameters, `automatic` functions over them, generate loops of cipher rounds — and the
corpus's second SystemVerilog row after `ibex`. Like `ibex`, its oracle is verilator, because
iverilog 13 cannot parse it. The design came out of the row-9 new-design census (ROADMAP §5.2
row 16), which ran it first.

## Provenance

| | |
|---|---|
| Repo | https://github.com/lowRISC/opentitan |
| Pinned SHA | `a3490b428e30cde95ad7a4d9072517bfbd03fc02` |
| Licence | Apache-2.0 (`src/LICENSE`). Every compiled file and every header they include (`prim_assert.sv`, `prim_assert_*_macros.svh`, `prim_assert_sec_cm.svh`, `prim_flop_macros.sv`) carries `SPDX-License-Identifier: Apache-2.0` |
| Clone path | `bench/opentitan-prims/src/` (gitignored), a sparse checkout of `hw/ip/prim/rtl` and `hw/ip/prim_generic/rtl` plus the tree's top-level files: 346 files, 60 MB, 54 MB of it the blob-less history |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Testbench | `tb.sv`, written for this bench, not upstream |
| Top module | `tb` |
| Defines | `SYNTHESIS`, as for `ibex`; every tool gets it |
| Lines fed to the simulators | 1879 = 1836 upstream RTL in the listed files + 43 local `tb.sv`, plus the headers `prim_assert.sv` includes |

Reconstruct the clone with `cargo run -p corpus-runner -- fetch --run`, or by hand:

```sh
cd bench/opentitan-prims
git clone --filter=blob:none --no-checkout https://github.com/lowRISC/opentitan src
git -C src sparse-checkout set --cone hw/ip/prim/rtl hw/ip/prim_generic/rtl
git -C src fetch --depth 1 origin a3490b428e30cde95ad7a4d9072517bfbd03fc02
git -C src checkout --detach a3490b428e30cde95ad7a4d9072517bfbd03fc02
```

## File list (`files.txt`, exact, in order)

```
src/hw/ip/prim/rtl/prim_cipher_pkg.sv             397
src/hw/ip/prim/rtl/prim_count_pkg.sv               15
src/hw/ip/prim/rtl/prim_count.sv                  313
src/hw/ip/prim/rtl/prim_crc32.sv                  324
src/hw/ip/prim_generic/rtl/prim_flop.sv            25
src/hw/ip/prim/rtl/prim_gf_mult.sv                184
src/hw/ip/prim/rtl/prim_present.sv                158
src/hw/ip/prim/rtl/prim_prince.sv                 242
src/hw/ip/prim/rtl/prim_secded_inv_39_32_dec.sv    62
src/hw/ip/prim/rtl/prim_secded_inv_39_32_enc.sv    24
src/hw/ip/prim/rtl/prim_subst_perm.sv              92
tb.sv                                              43
                                                 ----
                                                 1879
```

## Commands (from `bench/opentitan-prims/`)

```sh
F=$(cat files.txt)            # /bin/sh — zsh does not word-split $F
D="-DSYNTHESIS"
I="-Isrc/hw/ip/prim/rtl"

# vita
../../target/release/vita --top tb $D $I $F +N=32

# verilator (the oracle)
verilator --binary --timing --timescale 1ns/1ns -Wno-fatal -Wno-lint -Wno-style -Wno-MULTIDRIVEN \
  -j 0 -o v --top-module tb $D $I $F
./obj_dir/v +N=32
```

`./run.sh vita`, `./run.sh verilator`, `./run.sh verilator-x` (the x check below),
`./run.sh sv2v` (the 4-state cross-check below; needs `sv2v` on `PATH`) and
`./run.sh iverilog` wrap these; `N=<vectors>` overrides the size.

## Expected output

verilator 5.052 at `+N=32` (the pinned size), one checkpoint line and the digest:

```
P n=0 pres=784502bd3911c170 presd=b22af4077ebadd1d sp=97e4da0b0bd1933b spd=0123456789abcdef pr=af9d53d031ab32eb prd=0123456789abcdef enc=3e89abcdef dec=89abcdef syn=00 err=00 crc=efb8d5c7 gf=89abcdef cnt=00000000
DIGEST=8387a22d0531655d
- tb.sv:41: Verilog $finish
```

vita prints the same two lines, then `simulation ended (Finish) at time 350` and `errors=0
warnings=3 notes=1`, exit 0. The warnings are `VITA-W1017` (no `` `timescale `` in the design,
so the 1ns/1ns base is assumed) and two `VITA-W3056` for the decrypting PRESENT instance's
unconnected `key_o` and `idx_o`; the note is `VITA-I2021`, printed once, that `unique` case
overlaps are not checked (`prim_crc32.sv:32`). Two runs of each tool are byte-identical.

## What the workload exercises

| Instance | Configuration | Fed with |
|---|---|---|
| `prim_present` ×2 | 64-bit block, 128-bit key, 31 rounds; the second decrypts (`Decrypt=1`) the first's output with the first's final key and round index | the vector `x` and key `k` |
| `prim_subst_perm` ×2 | 64-bit, 31 rounds; the second inverts the first | `x`, `k[63:0]` |
| `prim_prince` ×2 | 64-bit block, 128-bit key, no halfway registers (combinational here); the second decrypts the first's output | `x`, `k` |
| `prim_secded_inv_39_32_enc` / `_dec` | the inverted 39/32 SECDED code | `x[31:0]`; the decoder sees the codeword with a single-bit error injected on vectors 5, 13, 21 and 29 and a double-bit error on vectors 9 and 25 |
| `prim_crc32` | 4 bytes per word | `x[31:0]` every cycle, re-seeded once on vector 3 |
| `prim_gf_mult` | 32-bit GF(2^32) multiply, 8 stages per cycle | `x[31:0]` × `x[63:32]` |
| `prim_count` | 32-bit hardened counter | `x`: increment and decrement enables, a step, a load on vector 7 |

Each vector `x` is an LFSR step of the previous one xored with a multiple of the vector index;
`k` is a 128-bit LFSR. After reset (three clocks, released on a falling edge), every falling
edge folds every primitive output — data, keys, round index, valid bits, syndrome, error
flags, CRC, product, both counter outputs and the counter's error — into a 64-bit rotate-xor
digest, three folds per vector. Final state is never compared. One `P` line prints every 50
vectors (one at `+N=32`).

## Sizing: 32 vectors

The census ran this testbench with a fixed 200 vectors: verilator 0.38 s, vita 54.1 s, sv2v
→ iverilog 148.7 s. The vector count is now a `+N=` plusarg and the row runs 32, so vita
takes about a sixth of that (study/03 §7 has the measured median); 32 still covers every
injected SECDED error and the CRC and counter loads above. Verilator at `+N=200` reproduces
the census's digest (`DIGEST=2958736d2b4a80a9`), so the plusarg changed nothing else. The
testbench is the census's `tb_prims.sv` with that one change and a longer header comment.

## The oracle is verilator, under one condition

iverilog 13 cannot parse the design. It stops in the first package, twelve times:

```
src/hw/ip/prim/rtl/prim_cipher_pkg.sv:23: sorry: packed array parameters are not supported yet.
```

so contract rule 2's exception applies: verilator stands as the oracle only if the digest does
not move when every uninitialised bit is randomised. `./run.sh verilator-x` builds with
`--x-assign unique --x-initial unique` and runs 66 times:

| run-time option | `+N=32` |
|---|---|
| `+verilator+rand+reset+0` (all zeros) and `+1` (all ones) | `8387a22d0531655d` |
| `+verilator+rand+reset+2`, seeds 1–64 | `8387a22d0531655d`, all 64 |
| the same build with `--x-initial-edge`, `+rand+reset+0`, `+1`, seeds 1–32 | `8387a22d0531655d`, all 34 |

### A 4-state cross-check

sv2v v0.0.13 converts the design to Verilog and iverilog 13 — a 4-state simulator — runs it
(`./run.sh sv2v`): `DIGEST=8387a22d0531655d` at `+N=32`, and the census measured
`2958736d2b4a80a9` at 200 vectors the same way. The digest accumulates every vector through
XOR, so one x would leave x digits in it; the 4-state run prints none.

The conversion keeps `always_comb`. With `-E Always` (the `ibex` recipe) sv2v rewrites
`always_comb` as `always @*`. iverilog then prints `sorry: constant selects in always_*
processes are not fully supported` 569 times, among them for `prim_subst_perm`'s round loop
over its own `data_state[2047:0]`; both `prim_subst_perm` outputs stay x, and the run prints
`DIGEST=xxxxxxxxxxxxxxxx` at `+N=32` and at `+N=200`. Kept as `always_comb`, the design
compiles without that message and every output is defined.

## The digest moves with the design

Contract rule 5, on copies of upstream files, with the clone untouched. Each change is one
line. The oracle column is the x-randomising verilator build at `+verilator+rand+reset+0`;
`+rand+reset+2` with seed 1 gives the same digest on every mutant.

| mutation | verilator | vita |
|---|---|---|
| none (the pin) | `8387a22d0531655d` | `8387a22d0531655d` |
| `prim_cipher_pkg.sv:144`, two entries of `PRESENT_SBOX4` swapped (`S(0)` and `S(1)`) | `9434e7b996d316cf` | `9434e7b996d316cf` |
| `prim_cipher_pkg.sv:44`, bit 0 of PRINCE's first round constant flipped | `94bd9aede625ffab` | `94bd9aede625ffab` |
| `prim_crc32.sv:309`, the seed loaded by `set_crc_i` no longer inverted | `000978e291fc7165` | `000978e291fc7165` |
| `prim_gf_mult.sv:38`, the reduction polynomial's `x^9` term moved to `x^8` | `d1c36b0015509523` | `d1c36b0015509523` |
| `prim_secded_inv_39_32_dec.sv:16`, data bit 0 dropped from syndrome bit 0 | `944d8b9c1b7203b5` | `944d8b9c1b7203b5` |
| `prim_count.sv:114`, the counter also counts when both enables are set (`^` → `\|`) | `28f64e3f03281e0d` | `28f64e3f03281e0d` |
| control: `prim_cipher_pkg.sv:215`, the 64-bit-key PRESENT schedule (this row uses the 128-bit one) | the pin | the pin |

Every mutant is asymmetric: it changes one primitive, or one direction of one (the SBOX swap
changes PRESENT's encryption and the substitution-permutation network's forward sbox, not their
inverse tables).

## vita today

The whole design runs to the pin with the upstream text unmodified, and vita prints the
oracle's digest on every mutant above.
