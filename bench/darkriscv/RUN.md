# darkriscv — vita differential benchmark

A 3-stage RISC-V (RV32E/I) soft core. This harness runs the **core + BRAM only**
(no UART/IO/PLL), boots the bundled firmware image, and reduces the whole run to a
single 64-bit `DIGEST` that three independent simulators agree on bit-for-bit.

## Provenance

| | |
|---|---|
| Repo | https://github.com/darklife/darkriscv |
| Pinned SHA | `4aa437997cd35253c9111f10a449de13ccaeee78` (2026-05-12) |
| License | **BSD-3-Clause** (`src/LICENSE`, Copyright (c) 2018 Marcelo Samsoniuk) |
| Clone path | `bench/darkriscv/src/` (the clone root; `bench/` is gitignored) |
| Upstream RTL | **unmodified** — verified `git status --porcelain` clean |

## Files fed to the simulators (in order)

Paths relative to `bench/darkriscv/`:

1. `tb2.v` — 88 lines, **our harness** (not upstream)
2. `src/rtl/darkriscv.v` — 965 lines, upstream, unmodified
3. `src/rtl/darkram.v` — 199 lines, upstream, unmodified

**LOC = 1252.**

Two further inputs are *required* but are not counted in LOC and are not passed on
the command line:

- `src/rtl/config.vh` — 577 lines, pulled in by `` `include "../rtl/config.vh" ``
  (this is why `-I ../rtl` and the `src/sim` cwd are both mandatory)
- `src/src/darksocv.mem` — 1991-word firmware image, loaded by
  `$readmemh("../src/darksocv.mem", ...)` in `darkram.v`

## Commands

**Both commands must be run from `bench/darkriscv/src/sim/`** — the `` `include ``
and `$readmemh` paths above are relative to that directory.

### iverilog (reference oracle)

```sh
cd bench/darkriscv/src/sim
iverilog -g2012 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl -o core.vvp \
         ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v
vvp core.vvp +N=600000
```

### vita

```sh
cd bench/darkriscv/src/sim
../../../../target/release/vita --top tb2 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl \
     ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v +N=600000
```

### verilator (optional third leg)

```sh
cd bench/darkriscv/src/sim
verilator --binary -j 4 -Wno-fatal -DSIMULATION=1 -D__WAITSTATE__=7 +incdir+../rtl \
          --top-module tb2 -o vcore ../../tb2.v ../rtl/darkriscv.v ../rtl/darkram.v
./obj_dir/vcore +N=600000
```

Note verilator wants `+incdir+../rtl` (or `-I../rtl`, no space). A spaced
`-I ../rtl` makes it read `../rtl` as a *source file* and it dies with a
misleading `obj_dir/../rtl.sv` not-found.

## Expected result

All three simulators print exactly this line:

```
DIGEST=59370cf8b1d0503d
```

iverilog and vita digest lines are byte-identical (`md5` of the grepped line matches:
`6c0c97eeb36f917dcb38bd1f03fb852d`). Both exit **0**. Both report the run ending at
simulation time `6000990000`.

## Median timings

Measured **interleaved** (iverilog, vita, iverilog, vita, ...) over 4 rounds; the
first round is discarded as cache-warming and the median of the remaining 3 is
reported. All binaries are release builds. macOS / Darwin 25.6.0, Apple silicon.
iverilog 13.0, verilator 5.050.

| Tool | Median | Raw (rounds 2/3/4) | vs iverilog |
|---|---|---|---|
| iverilog | **7.27 s** | 7.31 / 7.24 / 7.27 | 1.00x |
| vita | **9.22 s** | 9.15 / 9.25 / 9.22 | **0.79x** (vita slower) |
| verilator | 0.16 s | 0.16 / 0.16 | 45x |

Spread within each tool is under 1.5%, so the 1.27x vita-vs-iverilog gap is far
outside the noise.

## Determinism

- **8/8** runs across the battery printed `59370cf8b1d0503d` — every iverilog run,
  every vita run.
- **3/3** verilator runs agreed too.
- `-DSIMULATION=1` is **required for determinism, not cosmetics**: `darkram.v` zeroes
  `MEM` only under that ifdef, and `$readmemh` fills only 1991 of 2048 words, so
  without it the top 57 words stay X.
- Pass `--top tb2`. `darkcache`/`darkmac` are uninstantiated in this config; without
  an explicit top, vita's `W-ELAB-AUTOTOP-AMBIGUOUS` elaborates three independent roots.
- Do **not** pass vita's `-q` — it silences `$display` and therefore swallows the
  `DIGEST` line.

## Why the digest is trustworthy

It is not a single sampled point. iverilog and vita agree at **five** distinct
workload sizes, each producing a *different* digest, so the digest is demonstrably
workload-sensitive rather than a constant:

| N | DIGEST | agreeing tools |
|---|---|---|
| 30000 | `a0dbce6fd1dc52ec` | iverilog, vita |
| 60010 | `20489ab144f3a216` | iverilog, vita |
| 150000 | `f97d97c9887a0b4a` | iverilog, vita |
| 300000 | `fbac6e254aed6b46` | iverilog, vita, verilator |
| **600000** | **`59370cf8b1d0503d`** | **iverilog, vita, verilator** |

verilator agreeing matters independently: it is 2-state, so its match proves the
post-reset digest carries no X dependence — this is a genuine *value* differential,
not an X-map comparison.

**Construction** (`tb2.v`): on each `negedge CLK` after reset, rotate-left-1 a 64-bit
accumulator and xor in `{IADDR,DATAO} ^ {DADDR,DATAI} ^ {IDREQ,DDREQ,DRD,DWR,DBE}`.
At the end, fold all 32 architectural registers (`core0.REGS[k]`) through the same
rotate-xor. Sampling on **negedge** is deliberate: it removes any race against the
DUT's posedge NBA updates. A `san()` helper maps any word containing X to a fixed
constant via `(^v === 1'bx)`, keeping the digest 2-state and deterministic while
staying sensitive to *where* X appears.

## Workload tuning

`+N=600000` is the tuned value: it puts iverilog's median at 7.27 s, mid-window for
a 3-15 s target, so per-run startup is well amortised without the run being tedious.

The knob that actually scales this workload is the **upstream wait-state option**
`-D__WAITSTATE__=7`: it takes the same instruction stream from CPI 1.75 to 13.15.
Changing the UART baud does *not* scale it (the core never blocks on TX in
simulation). Both `-DSIMULATION=1` and `-D__WAITSTATE__=7` are upstream `config.vh`
options driven from the command line; **no RTL was edited**.

Scaling is linear and unbounded here: N=300000 -> 3.66 s, N=600000 -> 7.31 s. In
*this* harness `ESIMREQ` is tied to `1'b0`, so the core cannot call `$finish` on
itself and there is no natural ~385k-cycle ceiling. (That ceiling is real in the
full-SoC harness `tb.v`, where `darkuart.v` raises `ESIMREQ` at the firmware prompt.)
The only upper bound is the `#40000000` ns watchdog in `tb2.v`, i.e. ~4M cycles.

**Caveat when picking a different N:** two digests whose N differ by a multiple of 64
can collide (N=60000 == N=300000). That is not a bug — once the firmware reaches its
idle spin the sampled bus is constant, and a rotate-by-1 accumulator enters a
period-64 orbit. Early history is still fully retained (rotation is invertible).
Just avoid N values that are 64 apart when using this as a gate.

## Known-benign vita diagnostics (12 warnings, errors=0)

Both simulators agree through all of them:

- 2x `W-ELAB-FEATURE-LIMIT` — `DLEN` / `ESIMACK` output ports left unconnected
- 9x `W-RUN-RANGE-UNKNOWN` on `core0.REGS` at time 0 (X index before reset deasserts;
  further diagnostics suppressed)
- 1x `W-RUN-READMEM` — the 1991-of-2048 short firmware image

## Related finding: vita refuses the full SoC

`bench/darkriscv/tb.v` (top `tb`, 3115 lines: darksocv + darkbridge + darkuart +
darkriscv + darkpll + darkram + darkio + darkcache + darkmac) runs fine under
iverilog (`DIGEST=b4a7bb6d411fea85`, 5.16 s) but **vita exits 1**:

```
../rtl/darkuart.v:303:13: error[VITA-E3009] E-ELAB-UNSUPPORTED: $fgetc/$ungetc/$fgets/$fread/$fscanf/$sscanf are supported only as the direct rhs of a blocking assignment (v9) [in tb.soc0.io0.uart0]
```

The upstream line is `UART_RFIFO <= $fgetc(32'h8000_0000);` — a **nonblocking**
assignment whose rhs is `$fgetc`. vita implemented the blocking arm and not the
nonblocking twin. This is the single blocker; with it removed, elaboration reports
`errors=0` on everything else in the SoC.

A second, independent parser gap: enabling upstream `__RMW_CYCLE__` (config.vh:298)
reaches `darkram.v:72`, `$display("dpram: RMW cycle enabled.",);` — a **null argument**,
which IEEE 1364-2005 §17.1.1.2 explicitly permits (it prints a space) and iverilog
accepts:

```
../rtl/darkram.v:72:50: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found ')'
```

Neither affects the reported core run.
