# bench/verilog-axi — reproducible recipe

**Verdict: `vita-loud`.** iverilog runs the workload cleanly and emits a deterministic digest.
vita refuses the design at elaborate (exit 1, empty stdout, no digest). This is an honest
`correct-or-loud` refusal, not a wrong answer — see "The vita refusal" below.

## Provenance

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-axi |
| Pinned SHA | `516bd5dadc3365b7f9e225d2af8fe0b8d804fe53` |
| License | **MIT** (`src/COPYING`, "Copyright (c) 2018 Alex Forencich") — redistribution OK |
| Upstream RTL | 3901 lines (9 files, unmodified) |
| Testbench | `tb.v`, 523 lines, **written for this bench** (not upstream) |
| Total LOC fed to simulators | **4424** |

Restore the RTL from scratch:

```sh
cd <repo>/bench/verilog-axi
git clone https://github.com/alexforencich/verilog-axi src
git -C src checkout 516bd5dadc3365b7f9e225d2af8fe0b8d804fe53
```

`tb.v` is not in the upstream repo; it lives beside this file and must be kept.

## Tool versions used for the numbers below

- `Icarus Verilog version 13.0 (stable) (v13_0)` at `/opt/homebrew/bin/iverilog` + `/opt/homebrew/bin/vvp`
- `vita 0.1.0` at `../../target/release/vita` (release build)
- macOS arm64 (Darwin 25.6.0)

## File list — EXACT, IN ORDER

```
src/rtl/axi_crossbar.v
src/rtl/axi_crossbar_rd.v
src/rtl/axi_crossbar_wr.v
src/rtl/axi_crossbar_addr.v
src/rtl/axi_register_rd.v
src/rtl/axi_register_wr.v
src/rtl/arbiter.v
src/rtl/priority_encoder.v
src/rtl/axi_ram.v
tb.v
```

## Commands

All commands are run from `bench/verilog-axi/`. Tuned workload size: **`+N=5000`**.

### iverilog (the oracle)

```sh
/opt/homebrew/bin/iverilog -g2012 -o x.vvp \
  src/rtl/axi_crossbar.v src/rtl/axi_crossbar_rd.v src/rtl/axi_crossbar_wr.v \
  src/rtl/axi_crossbar_addr.v src/rtl/axi_register_rd.v src/rtl/axi_register_wr.v \
  src/rtl/arbiter.v src/rtl/priority_encoder.v src/rtl/axi_ram.v tb.v \
  && /opt/homebrew/bin/vvp x.vvp +N=5000
```

Compile is silent (zero warnings, zero bytes on stdout/stderr) and costs 0.07 s.

### vita

```sh
../../target/release/vita \
  src/rtl/axi_crossbar.v src/rtl/axi_crossbar_rd.v src/rtl/axi_crossbar_wr.v \
  src/rtl/axi_crossbar_addr.v src/rtl/axi_register_rd.v src/rtl/axi_register_wr.v \
  src/rtl/arbiter.v src/rtl/priority_encoder.v src/rtl/axi_ram.v tb.v \
  +N=5000
```

Convenience wrapper: `./run.sh [N]` (defaults to `N=5000`, runs iverilog then vita).

## Expected output

iverilog's stdout ends with these four lines (verbatim, after 12 lines of the crossbar's
own address-decode banner):

```
OPS=5000
D0=00007df8ab023b59 D1=00007de1f391e75a
CYCD=3b9321bc8ff2b44a CYCLES=123166 XC=29
DIGEST=3b9321d5ea42f302
```

**Expected DIGEST (iverilog): `3b9321d5ea42f302`**

**vita prints no DIGEST line at all** — stdout is 0 bytes. It exits 1 after 98 lines of
diagnostics on stderr, whose first line is verbatim:

```
tb.v:300:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `S_THREADS` value is not a foldable constant expression [in tb.xbar]
```

(Path prefix follows however you spelled the file on the command line.)
Diagnostic census, stable across runs: **54x `VITA-E3009` + 43x `VITA-W3056`**, then
`errors=54 warnings=43 notes=0`.

So **there is no vita-vs-iverilog digest comparison to make yet.** Do not record a match.

## Median timings

Method: 3 pairs run **interleaved** (iverilog, vita, iverilog, vita, iverilog, vita),
first pair discarded as cache-warming, median of the remaining two reported.

| | rep2 | rep3 | **median** |
|---|---|---|---|
| iverilog, full recipe (compile + vvp) | 6.59 s | 6.55 s | **6.57 s** |
| iverilog, `vvp` only | 6.52 s | 6.48 s | **6.50 s** |
| vita (elaborate, refuses) | 0.06 s | 0.06 s | **0.06 s** |

Discarded first pair: iverilog 6.74 s total / 6.68 s vvp, vita 0.06 s. Spread across all
three iverilog runs is 0.19 s (2.9 %), so the warm/cold effect here is small but real.

### Determinism — verified, holds

All three iverilog runs produced **byte-identical stdout** (`md5 4cc56b17ca2caf2660ceac7d3738f08c`),
each from a freshly compiled `.vvp`. All three vita runs produced byte-identical stdout
(empty) and byte-identical stderr (`md5 c6f38e445e6459f138acce409b025d08`).

### Workload dial

`+N=<ops-per-master>` scales essentially linearly. Each N has its own digest:

| `+N=` | vvp secs | CYCLES | DIGEST |
|---|---|---|---|
| 2000 | 2.71 | 49342 | `f4518d5cc660051d` |
| **5000** | **6.50** | **123166** | **`3b9321d5ea42f302`** |
| 10000 | 13.31 | 246245 | `d5910bf753af400b` |

`N=5000` is the tuned setting: it sits mid-band in the 3–15 s target, compile time is
negligible next to it (1 %), and `N=2000` already falls below the 3 s floor.

## What the design is

A 2x2 `axi_crossbar` (S_COUNT=2, M_COUNT=2, DATA_WIDTH=32, ADDR_WIDTH=32, S_ID_WIDTH=4,
M_ID_WIDTH=5; everything else upstream defaults) feeding two `axi_ram` instances
(ADDR_WIDTH=16) through a generate loop. Default decode: slave0 = `0x00000000/24`,
slave1 = `0x01000000/24`.

`tb.v` instantiates a synthetic AXI4 master BFM twice via generate. Each master runs N ops:
LFSR-picked slave, burst length 1..8, 32-byte-aligned address inside a **private** 4 KiB
window (master m uses offset `m<<13`), writes an INCR burst, waits for B, reads the same
burst back, and folds rdata/rresp/rid/bresp into a 64-bit accumulator. Private windows make
the digest independent of crossbar arbitration *order*, so it is deterministic without being
blind to the crossbar. A second accumulator (`CYCD`) hashes the master-side bus every clock
(rotate-xor over wdata/rdata plus 10 handshake bits), so the digest is sensitive to
cycle-level behaviour, not just final data. Watchdog is cycle-based (`N*512+20000`).

## Known hazards for anyone re-using this benchmark

1. **Four-state dependence — do not use a 2-state tool as an oracle here.** `axi_crossbar`
   drives `m_axi_wvalid = X` for the first 29 cycles after reset. Masking data by its valid
   bit is not enough, because the valid bit itself is X. `tb.v` substitutes a fixed constant
   on any X sample and counts X cycles into the digest as `XC` (= 29 at every N measured).
   Verilator is **deliberately not run**: `--binary` is 2-state, so a number from it would be
   misleading rather than informative. If a 2-state tool is ever compared, `XC` is the field
   that will differ first.
2. **The vita refusal narrows to one construct: `{n{expr}}` is missing from the constant
   domain.** `probe.v` / `probe2.v` in this directory isolate it:
   - `parameter P_CONCAT = {32'd1, 32'd2};` -> vita OK
   - `parameter P_REPL = {4{32'd2}};` -> **E3009 "not a foldable constant expression"**
   - `localparam L_REPL = {4{32'd2}};` -> **E3009**
   - `parameter C = S_COUNT * 32'd2;` -> vita OK
   - `r = {4{16'd2}};` (runtime, procedural) -> vita OK

   So it is not about the repeat count being a parameter, and not about parameter defaults
   vs localparams: a fully literal `{4{32'd2}}` is refused while the same replication at
   runtime is fine. `const_eval_in_scope` has a Concat arm but no Replication arm. iverilog
   folds all of them (`P_REPL=00000002000000020000000200000002`). verilog-axi's whole
   parameter convention is `parameter S_THREADS = {S_COUNT{32'd2}}` and
   `parameter M_ADDR_WIDTH = {M_COUNT{{M_REGIONS{32'd24}}}}`, so this one gap blocks the
   entire library, not just the crossbar.
3. **The 43 `VITA-W3056` warnings are unconnected output ports** (`s_axi_buser`,
   `m_axi_awuser`, `m_axi_awregion`, ...). They are legitimate in this design and will become
   pure warning noise the moment the E3009s are fixed.
4. When `{n{...}}` folding lands in the constant domain, this entry converts from a
   `vita-loud` record into a 4424-line generate/parameter-elaboration stress with a real
   differential digest. Re-run this file's recipe unchanged and compare against
   `DIGEST=3b9321d5ea42f302`.
