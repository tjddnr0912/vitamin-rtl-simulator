# bench/serv — the `servant` SoC, bit-serial RV32I

The corpus row `serv`: olofk's bit-serial RISC-V core inside its `servant` SoC, running
a real program out of a `$readmemh` memory for 500 000 clock cycles and reducing the
wishbone traffic to one `DIGEST=` line. The core takes about 35 clocks per instruction,
so this row measures scheduler throughput rather than datapath width — it is the corpus
row where vita and Icarus Verilog come out closest. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter serv --compare` reproduces it.

## Provenance

| | |
|---|---|
| Repo | https://github.com/olofk/serv |
| Pinned SHA | `41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2` |
| Licence | ISC (`src/LICENSE`, Copyright 2019 Olof Kindgren) — permissive, redistribution permitted with notice |
| Clone path | `bench/serv/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Committed here | `tb.v`, `files.txt`, `run.sh`, this file |
| Firmware | `src/sw/blinky.hex` — upstream's own, read in place; nothing is copied or regenerated |
| Top module | `tb` (must be pinned: `--top tb` / `-s tb`) |
| Lines fed to the simulators | 4318 across 27 files |
| Workload | `+N=500000` cycles |
| Expected | `DIGEST=f3f45af36093b2b1` |

Reconstruct the clone:

```sh
cd bench/serv
git clone https://github.com/olofk/serv src
git -C src checkout 41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2
```

`corpus-runner fetch --run` performs exactly this; there is no `prepare.sh` for this
row. `src/sw/blinky.hex` is upstream's shipped image, 11 lines: a real RV32I program of
`lui`/`addi`/`sb`/`xori`/`and` around a `0x100000`-iteration `addi`/`bne` delay loop.
`tb.v` passes its path to `servant`'s `memfile` parameter, so the working directory has
to be `bench/serv/` when the simulators run.

## File list (exact, in order — this is `files.txt`)

Order matters only in that `tb.v` comes first; that is the order actually measured.

```
tb.v
src/rtl/serv_bufreg.v
src/rtl/serv_bufreg2.v
src/rtl/serv_alu.v
src/rtl/serv_csr.v
src/rtl/serv_ctrl.v
src/rtl/serv_decode.v
src/rtl/serv_immdec.v
src/rtl/serv_mem_if.v
src/rtl/serv_rf_if.v
src/rtl/serv_rf_ram_if.v
src/rtl/serv_rf_ram.v
src/rtl/serv_state.v
src/rtl/serv_debug.v
src/rtl/serv_top.v
src/rtl/serv_rf_top.v
src/rtl/serv_aligner.v
src/rtl/serv_compdec.v
src/servile/servile_arbiter.v
src/servile/servile_mux.v
src/servile/servile_rf_mem_if.v
src/servile/servile.v
src/servant/servant_timer.v
src/servant/servant_gpio.v
src/servant/servant_mux.v
src/servant/servant_ram.v
src/servant/servant.v
```

Derive any variant of this list from `files.txt`, not from a
`src/rtl/*.v src/servile/*.v src/servant/*.v` glob — that glob pulls in 43 extra FPGA
board wrappers.

## Commands

Run with the working directory set to `bench/serv/`. `$FILES` below is
`$(cat files.txt)` in that order; use `/bin/sh` rather than zsh, which does not
word-split an unquoted `$FILES` and hands the whole list to the tool as one filename.

```sh
# vita (one-shot, release binary)
../../target/release/vita --top tb $FILES +N=500000

# iverilog (the oracle)
iverilog -g2012 -s tb -o x.vvp $FILES && vvp x.vvp +N=500000
```

`./run.sh iverilog` and `./run.sh vita` wrap the two, with the cycle count overridable
as `N=<cycles> ./run.sh …`.

### Why `--top tb` / `-s tb` is not optional

This file list has three uninstantiated roots — `tb`, `serv_rf_top` and
`servile_rf_mem_if`. Left to pick for itself, vita reports
`VITA-W3057 W-ELAB-AUTOTOP-AMBIGUOUS: auto-top selected 3 uninstantiated roots` and
elaborates all three as independent tops, while iverilog drops the extras. The digest is
unaffected — the extra roots drive nothing — but the two tools are then no longer
elaborating the same design, so pin the top on both sides.

## Expected output

Both tools, verbatim at `+N=500000`:

```
vita:
Preloading tb.dut.ram from src/sw/blinky.hex
CYCLES=500000
DIGEST=f3f45af36093b2b1
simulation ended (Finish) at time 5000155
errors=0 warnings=11 notes=0

iverilog:
Preloading tb.dut.ram from src/sw/blinky.hex
WARNING: src/servant/servant_ram.v:45: $readmemh(src/sw/blinky.hex): Not enough words in the file for the requested range [0:2047].
CYCLES=500000
DIGEST=f3f45af36093b2b1
tb.v:66: $finish called at 5000155 (1s)
```

Same digest, same cycle count, same finish time, exit 0 on both. Both tools also report
that `blinky.hex` under-fills the 2048-word range; it is harmless, because the tail
stays at its reset value in both, which is why the digests agree.

vita's 11 warnings:

| Code | Count | What |
|---|---:|---|
| `VITA-W1017` | 1 | No `` `timescale `` in the design; the 1ns/1ns base is assumed. That is iverilog's default here too, which is why the two finish times line up |
| `VITA-W4023` | 1 | `$readmem('src/sw/blinky.hex')`: 11 of 2048 words; the rest unchanged — the same fact iverilog's `WARNING:` line reports |
| `VITA-W4029` | 9 | Unknown array word index on `tb.dut.ram.mem` and `tb.dut.rf_ram.memory` during the first fetches — "read X / write ignored", ending in a suppression line. The design genuinely indexes with x there, and both tools agree on the outcome |

## The agreement is not N-specific

The digest is a function of the run length, so a single matching size could be a fixed
point. Checked at a second one:

| `+N=` | vita | iverilog |
|---|---|---|
| 123457 | `5047014c6f49c607` | `5047014c6f49c607` |
| 500000 | `f3f45af36093b2b1` | `f3f45af36093b2b1` |

Both tools also end at the same simulated time at both sizes.

## Determinism

Repeated runs of each tool produce a byte-identical digest, including across
independent iverilog compiles of the same file list.

## Verilator is not an oracle here

Verilator 5.050 runs this design and reports `DIGEST=e7e8b5e6c1276563`, which does not
match — and that is expected rather than a finding. `serv_rf_ram` drives
`rdata <= i_ren ? memory[i_raddr] : {width{1'bx}}` and both the register file and the
RAM start uninitialised, so the workload genuinely depends on 4-state semantics that a
2-state model approximates away. The manifest records iverilog as the sole oracle for
this row.

## What the digest observes, and why `tb.v` looks the way it does

`tb.v` accumulates, on every `posedge clk` after reset, a rotate-xor over the memory-bus
and external-bus taps inside `servant`. Three decisions in it are load-bearing:

- **The wishbone taps are masked by `stb`/`ack`.** SERV drives adr/rdt to x whenever
  `stb` is low; an ungated accumulator returns `xxxxxxxxxxxxxxxx`, which compares equal
  to nothing.
- **`wb_mem_sel` is excluded from the digest.** It is still x on ten cycles during the
  first fetch even after gating.
- **`` `define SERV_CLEAR_RAM ``** sits at the top of `tb.v` and carries across the
  whole file list in every tool, so the register-file zeroing needs no `-D` flag on
  either command line.

## Workload tuning

`+N=500000` puts the iverilog reference near the middle of the 3–15 s band. The dial is
linear, so roughly `+N=200000` is the floor and `+N=1000000` the ceiling if a different
point is needed — but the digests recorded here are for `+N=500000` only.
