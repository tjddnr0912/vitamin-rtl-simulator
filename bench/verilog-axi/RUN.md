# bench/verilog-axi — reproducible recipe

The corpus row `verilog-axi`: a 2×2 `axi_crossbar` feeding two `axi_ram` instances,
driven by two synthetic AXI4 master BFMs and reduced to one `DIGEST=` line. It is the
corpus's fabric shape — elaboration- and generate-heavy — and its only **ruled split**:
the design runs, and its digest disagrees with the oracle's on one axis the oracle
cannot arbitrate. See "The ruling" below before reading anything into the mismatch.
The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter verilog-axi` grades the row.

## Provenance

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-axi |
| Pinned SHA | `516bd5dadc3365b7f9e225d2af8fe0b8d804fe53` |
| Licence | MIT (`src/COPYING`, Copyright (c) 2018 Alex Forencich) — permissive, redistribution permitted with notice |
| Clone path | `bench/verilog-axi/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Testbench | `tb.v`, written for this bench, not upstream |
| Top module | `tb` |
| Lines fed to the simulators | 4424 = 3901 upstream RTL (9 files) + 523 local `tb.v` |

Reconstruct the clone:

```sh
cd bench/verilog-axi
git clone https://github.com/alexforencich/verilog-axi src
git -C src checkout 516bd5dadc3365b7f9e225d2af8fe0b8d804fe53
```

## File list (exact, in order)

Paths are relative to `bench/verilog-axi/`; `tb.v` comes last.

```
src/rtl/axi_crossbar.v        391
src/rtl/axi_crossbar_rd.v     569
src/rtl/axi_crossbar_wr.v     678
src/rtl/axi_crossbar_addr.v   418
src/rtl/axi_register_rd.v     530
src/rtl/axi_register_wr.v     691
src/rtl/arbiter.v             159
src/rtl/priority_encoder.v     92
src/rtl/axi_ram.v             373
tb.v                          523
                             ----
                             4424
```

## Commands (from `bench/verilog-axi/`)

```sh
# vita (one-shot, release binary)
../../target/release/vita \
  src/rtl/axi_crossbar.v src/rtl/axi_crossbar_rd.v src/rtl/axi_crossbar_wr.v \
  src/rtl/axi_crossbar_addr.v src/rtl/axi_register_rd.v src/rtl/axi_register_wr.v \
  src/rtl/arbiter.v src/rtl/priority_encoder.v src/rtl/axi_ram.v tb.v \
  +N=5000

# iverilog (the oracle)
iverilog -g2012 -o x.vvp \
  src/rtl/axi_crossbar.v src/rtl/axi_crossbar_rd.v src/rtl/axi_crossbar_wr.v \
  src/rtl/axi_crossbar_addr.v src/rtl/axi_register_rd.v src/rtl/axi_register_wr.v \
  src/rtl/arbiter.v src/rtl/priority_encoder.v src/rtl/axi_ram.v tb.v \
  && vvp x.vvp +N=5000
```

`./run.sh [N]` wraps both, defaulting to `N=5000` and running iverilog first. It
invokes `vita` from `PATH` rather than from `target/release/`.

> zsh trap. Do not put the file list in a plain variable and write `vita $F`. zsh does
> not word-split an unquoted parameter expansion, so all ten paths arrive as one argv
> entry and vita reports that it cannot read a file whose name is the whole list. Use
> an array, `${=F}`, or `/bin/sh`.

## Expected output

Both tools print the crossbar's own 12-line address-decode banner, then four lines.
Verbatim tails at `+N=5000`:

```
iverilog:
OPS=5000
D0=00007df8ab023b59 D1=00007de1f391e75a
CYCD=3b9321bc8ff2b44a CYCLES=123166 XC=29
DIGEST=3b9321d5ea42f302

vita:
OPS=5000
D0=00007df8ab023b59 D1=00007de1f391e75a
CYCD=fd90a12720932ea7 CYCLES=123166 XC=0
DIGEST=fd90a1407928ebc8
```

Both end at simulated time 1231865000 and exit 0. vita's stderr carries 15
`VITA-W3056 W-ELAB-FEATURE-LIMIT` warnings, one per unconnected output port
(`m_axi_arregion`, `m_axi_awregion`, `s_axi_buser`, …); those ports are legitimately
left off the instantiation and iverilog is silent about them.

## The ruling

The two runs agree on the transaction count, on the cycle count, and on both per-master
data accumulators. The whole difference is `XC` — 29 x-cycles out of a 123 166-cycle
run, in the crossbar's registered `valid` outputs — and `CYCD`, which is the same
per-cycle accumulator differing on exactly those 29 samples. The final digest is
`(D0 ^ D1) + CYCD + CYCLES + XC * 1000003`, and both tools' digests reconcile from
their own four reported components, so nothing else diverges.

The residue is time-zero continuous-assign event ordering, on which **iverilog answers
two ways to the same question**: with identical operands and identical values,
`wire w = a | b;` fires the `always @*` that reads it at time zero and
`wire w = a & b;` does not. "Matches iverilog" therefore stops being a statement about
vita here. Verilator settles everything at time zero and has no vote either.

The manifest row records this as `Expect::Split`, which pins **both** digests — the
oracle's `3b9321d5ea42f302` and vita's own `fd90a1407928ebc8` — with the ruling in its
`why` field. A change on either side still fails the gate: vita's answer moving grades
`REGRESSION`, and the two agreeing again grades `PROMOTED` and asks for the row to move
to `Expect::Runs`. That state exists for a divergence the oracle cannot arbitrate and
for nothing else; a digest that merely fails to match is a finding, not a split.

## The split is not N-specific

Rerunning the dial reproduces the same shape rather than a coincidence at one size.
At `+N=2000`, both tools give `OPS=2000`, `D0=000032938cb8a68e D1=000032c4dc48bd3c` and
`CYCLES=49342`, and again differ only in the x-cycle count:

| `+N=` | tool | XC | DIGEST |
|---|---|---:|---|
| 2000 | iverilog | 29 | `f4518d5cc660051d` |
| 2000 | vita | 0 | `5b30184006a803fd` |
| 5000 | iverilog | 29 | `3b9321d5ea42f302` |
| 5000 | vita | 0 | `fd90a1407928ebc8` |

`XC=29` at both sizes is the tell: it is a fixed cost paid once, at time zero, not a
per-transaction divergence.

## Workload dial

`+N=<ops-per-master>` scales essentially linearly, and each size has its own digest.
`N=5000` is the tuned setting: it puts the iverilog reference mid-band in the 3–15 s
target with compile time negligible beside it, and `N=2000` already falls below the 3 s
floor.

## What the design is

A 2×2 `axi_crossbar` (`S_COUNT=2`, `M_COUNT=2`, `DATA_WIDTH=32`, `ADDR_WIDTH=32`,
`S_ID_WIDTH=4`, `M_ID_WIDTH=5`, everything else upstream defaults) feeding two `axi_ram`
instances (`RAM_ADDR_WIDTH=16`) through a generate loop. Default address decode:
slave 0 at `0x00000000/24`, slave 1 at `0x01000000/24`.

`tb.v` instantiates a synthetic AXI4 master BFM twice via generate. Each master runs N
ops: an LFSR-picked slave, a burst length of 1..8, a 32-byte-aligned address inside a
**private** 4 KiB window (master m at offset `m<<13`), an INCR burst write, a wait for
B, a read of the same burst back, and a fold of rdata/rresp/rid/bresp into a 64-bit
accumulator. Private windows make the per-master digests independent of crossbar
arbitration *order*, so they are deterministic without being blind to the crossbar. A
second accumulator, `CYCD`, hashes the master-side bus every clock — a rotate-xor over
wdata/rdata plus ten handshake bits — so the digest has cycle resolution rather than
being an end-state check. The watchdog is cycle-based (`N*512 + 20000`).

## Hazards for anyone re-using this benchmark

1. **Four-state dependence — do not use a 2-state tool as an oracle here.** The
   crossbar drives `m_axi_wvalid` unknown before its first transaction. Masking data by
   its valid bit is not enough, because the valid bit itself is the unknown one.
   `tb.v` substitutes a fixed constant on any x sample and counts those cycles into the
   digest as `XC`. Verilator is deliberately not run: `--binary` is 2-state, so a number
   from it would be misleading rather than informative — and `XC` is precisely the field
   a 2-state tool would flatten.
2. **`XC` is the field to read first on any disagreement.** Every divergence measured on
   this workload so far has been the x-cycle count and nothing else; a divergence in
   `D0`, `D1` or `CYCLES` would be a different and much larger finding.
3. **The unconnected-output warnings are noise, but they are load-bearing noise.** All
   15 are legal dangling outputs. A change in their count means the elaboration of the
   generate loop changed, which is worth looking at even though the warnings themselves
   are harmless.
