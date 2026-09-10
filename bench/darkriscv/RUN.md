# bench/darkriscv — vita differential benchmark

The corpus row `darkriscv`: a 3-stage RISC-V (RV32E/I) soft core. This harness runs the
core plus BRAM only — no UART, IO or PLL — boots the bundled firmware image, and
reduces the whole run to a single 64-bit `DIGEST` that three independent simulators
agree on bit-for-bit. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter darkriscv --compare` reproduces it.

## Provenance

| | |
|---|---|
| Repo | https://github.com/darklife/darkriscv |
| Pinned SHA | `4aa437997cd35253c9111f10a449de13ccaeee78` |
| Licence | BSD-3-Clause (`src/LICENSE`, Copyright (c) 2018 Marcelo Samsoniuk) |
| Clone path | `bench/darkriscv/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty |
| Top module | `tb2` |

Reconstruct the clone:

```sh
cd bench/darkriscv
git clone https://github.com/darklife/darkriscv src
git -C src checkout 4aa437997cd35253c9111f10a449de13ccaeee78
```

## Files fed to the simulators (in order)

Paths relative to `bench/darkriscv/`:

| File | Lines | Origin |
|---|---:|---|
| `tb2.v` | 89 | this harness, committed here |
| `src/rtl/darkriscv.v` | 965 | upstream, unmodified |
| `src/rtl/darkram.v` | 199 | upstream, unmodified |
| | 1253 | |

Two further inputs are required, are not counted in the line total, and are not passed
on the command line:

- `src/rtl/config.vh` — 577 lines, pulled in by `` `include "../rtl/config.vh" ``.
  This is why `-I ../rtl` and the `src/sim` working directory are both mandatory.
- `src/src/darksocv.mem` — a 1991-word firmware image loaded by
  `$readmemh("../src/darksocv.mem", ...)` in `darkram.v`.

## Commands

All three commands run from `bench/darkriscv/src/sim/` — the `` `include `` and
`$readmemh` paths above are relative to that directory. The corpus manifest records
this as the workload's working directory for the same reason.

iverilog (the oracle):

```sh
cd bench/darkriscv/src/sim
iverilog -g2012 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl -o core.vvp \
         ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v
vvp core.vvp +N=600000
```

vita:

```sh
cd bench/darkriscv/src/sim
../../../../target/release/vita --top tb2 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl \
     ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v +N=600000
```

verilator (third leg):

```sh
cd bench/darkriscv/src/sim
verilator --binary -j 4 -Wno-fatal -DSIMULATION=1 -D__WAITSTATE__=7 +incdir+../rtl \
          --top-module tb2 -o vcore ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v
./obj_dir/vcore +N=600000
```

verilator wants `+incdir+../rtl` or `-I../rtl` with no space. A spaced `-I ../rtl`
makes it read `../rtl` as a source file and it dies with a misleading
`obj_dir/../rtl.sv` not-found.

## Expected result

All three simulators print exactly this line:

```
DIGEST=59370cf8b1d0503d
```

iverilog and vita produce byte-identical digest lines, both exit 0, and both report the
run ending at simulation time `6000990000`.

## Required flags

| Flag | Why |
|---|---|
| `-DSIMULATION=1` | Required for determinism, not cosmetics. `darkram.v` zeroes `MEM` only under that `ifdef`, and `$readmemh` fills 1991 of 2048 words, so without it the top 57 words stay X |
| `-D__WAITSTATE__=7` | The knob that scales the workload: it takes the same instruction stream from CPI 1.75 to 13.15. Changing the UART baud does not scale it, because the core never blocks on TX in simulation |
| `--top tb2` / `-s tb2` | `darkcache` and `darkmac` are uninstantiated in this configuration. Without an explicit top, vita reports `W-ELAB-AUTOTOP-AMBIGUOUS` and elaborates three independent roots |
| Do not pass `-q` | vita's `-q` silences `$display` and therefore swallows the `DIGEST` line |

Both `-DSIMULATION=1` and `-D__WAITSTATE__=7` are upstream `config.vh` options driven
from the command line. No RTL is edited.

## Why the digest is trustworthy

It is not a single sampled point. The tools agree at five distinct workload sizes, each
producing a different digest, so the digest is demonstrably workload-sensitive rather
than a constant:

| `+N=` | DIGEST | agreeing tools |
|---|---|---|
| 30000 | `a0dbce6fd1dc52ec` | iverilog, vita |
| 60010 | `20489ab144f3a216` | iverilog, vita |
| 150000 | `f97d97c9887a0b4a` | iverilog, vita |
| 300000 | `fbac6e254aed6b46` | iverilog, vita, verilator |
| 600000 (the pinned workload) | `59370cf8b1d0503d` | iverilog, vita, verilator |

verilator agreeing matters independently: it is 2-state, so its match proves the
post-reset digest carries no X dependence. This is a value differential, not an X-map
comparison.

Construction, in `tb2.v`: on each `negedge CLK` after reset, rotate-left a 64-bit
accumulator by 1 and xor in
`{IADDR,DATAO} ^ {DADDR,DATAI} ^ {IDREQ,DDREQ,DRD,DWR,DBE}`. At the end, fold all 32
architectural registers (`core0.REGS[k]`) through the same rotate-xor. Sampling on
negedge is deliberate: it removes any race against the DUT's posedge nonblocking
updates. A `san()` helper maps any word containing X to a fixed constant via
`(^v === 1'bx)`, which keeps the digest 2-state and deterministic while staying
sensitive to *where* X appears.

## Workload tuning

`+N=600000` is the pinned value; it puts the iverilog reference mid-band in the 3–15 s
target, so per-run startup is well amortised without the run being tedious. Scaling is
linear and unbounded here: in this harness `ESIMREQ` is tied to `1'b0`, so the core
cannot call `$finish` on itself and there is no natural cycle ceiling. The only upper
bound is the `#40000000` ns watchdog in `tb2.v`, about 4M cycles.

Caveat when picking a different N: two digests whose N differ by a multiple of 64 can
collide — N=60000 and N=300000 do. That is not a defect. Once the firmware reaches its
idle spin the sampled bus is constant, and a rotate-by-1 accumulator enters a period-64
orbit; early history is still fully retained, since rotation is invertible. Avoid N
values 64 apart when using this as a gate.

## Determinism

Repeated runs of every tool print the same digest. `-DSIMULATION=1` carries the whole
determinism argument, as described in the flag table above.

## vita diagnostics — 12 warnings, errors=0

Both simulators agree through all of them.

| Code | Count | What |
|---|---:|---|
| `VITA-W3056` | 2 | `DLEN` and `ESIMACK` output ports left unconnected |
| `VITA-W4029` | 9 | Unknown array word index on `core0.REGS` at time 0, before reset deasserts, ending in a suppression line |
| `VITA-W4023` | 1 | `$readmem` short image — 1991 of 2048 words, rest unchanged |

## The full-SoC harness

`bench/darkriscv/tb.v` (top `tb`) drives the whole SoC — darksocv, darkbridge,
darkuart, darkriscv, darkpll, darkram, darkio, darkcache and darkmac, 3115 lines with
the testbench. It is committed beside this file and is not a corpus row: the corpus
measures the core, which is the part with a clean three-tool agreement, and the SoC
adds a UART whose `$fgetc` path makes the workload depend on host file state.

Run it from the same directory, with `--top tb` / `-s tb` and the same defines:

```sh
../../../../target/release/vita --top tb -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl \
     ../../tb.v ../rtl/darksocv.v ../rtl/darkbridge.v ../rtl/darkuart.v \
     ../rtl/darkriscv.v ../rtl/darkpll.v ../rtl/darkram.v ../rtl/darkio.v \
     ../rtl/darkcache.v ../rtl/darkmac.v +N=1000
```

vita and iverilog agree on it, at `+N=500`, `+N=1000` and `+N=5000`:

| `+N=` | DIGEST |
|---|---|
| 500 | `73607907b6755e07` |
| 1000 | `40c2edba15a45106` |
| 5000 | `4c251ba45c48151c` |

## A construct in the SoC tree that vita rejects

Enabling upstream's `__RMW_CYCLE__` option (`config.vh:298`) reaches `darkram.v:72`,
`$display("dpram: RMW cycle enabled.",);` — a null argument, which IEEE 1364-2005
§17.1.1.2 explicitly permits (it prints a space) and iverilog accepts:

```
../rtl/darkram.v:72:50: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found ')'
```

The option is off in both harnesses above, so neither run reaches it.
