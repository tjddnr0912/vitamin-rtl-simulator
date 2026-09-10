# bench/verilog-ethernet — `eth_mac_1g` GMII loopback

The corpus row `verilog-ethernet`: a 1G Ethernet MAC with its GMII transmit output
wired straight back into its GMII receive input, driven with N pseudo-random frames and
reduced to one `DIGEST=` line. It is the corpus's only streaming shape — many small
`always` blocks and high event churn, rather than a wide datapath or a branchy core.
The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter verilog-ethernet --compare` reproduces it.

## Provenance

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-ethernet |
| Pinned SHA | `77320a9471d19c7dd383914bc049e02d9f4f1ffb` |
| Licence | MIT (`src/COPYING`, Copyright (c) 2014-2018 Alex Forencich) — permissive, redistribution permitted with notice |
| Clone path | `bench/verilog-ethernet/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Testbench | `tb.v`, written for this bench. Upstream's own `tb/` is cocotb/Python and cannot drive this comparison |
| Top module | `tb` |
| Lines fed to the simulators | 2155 = 1907 upstream RTL + 248 local `tb.v` |
| Language level | Verilog-2005 RTL; `-g2012` only because upstream uses `` `resetall `` / `` `default_nettype none `` |

Reconstruct the clone:

```sh
cd bench/verilog-ethernet
git clone https://github.com/alexforencich/verilog-ethernet src
git -C src checkout 77320a9471d19c7dd383914bc049e02d9f4f1ffb
```

## File list (exact, in order)

Paths are relative to `bench/verilog-ethernet/`. No tool depends on the order; keep it
stable so the digest stays reproducible.

```
tb.v                        248
src/rtl/eth_mac_1g.v        644
src/rtl/axis_gmii_rx.v      358
src/rtl/axis_gmii_tx.v      458
src/rtl/lfsr.v              447
                           ----
                           2155
```

## Commands (from `bench/verilog-ethernet/`)

```sh
# vita (one-shot, release binary)
../../target/release/vita tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v +N=1000

# iverilog (the oracle)
iverilog -g2012 -o x.vvp tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v
vvp x.vvp +N=1000

# verilator (second, independent opinion)
verilator --binary --timing -Wno-fatal -o v --top-module tb tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v
./obj_dir/v +N=1000
```

`./run.sh iverilog`, `./run.sh vita` and `./run.sh verilator` wrap the three, with the
frame count overridable as `N=<frames> ./run.sh …`.

> zsh trap. Do not put the file list in a plain variable and write `vita $F`. zsh does
> not word-split an unquoted parameter expansion, so all five paths arrive as one argv
> entry and vita reports that it cannot read a file whose name is the whole list. Use
> an array, `${=F}`, or `/bin/sh`.

## Expected output

All three tools print the same two lines at `+N=1000`, byte-identical:

```
RX_FRAMES=1000 RX_BYTES=156111
DIGEST=ca4945d0044f74d8
```

vita then prints `simulation ended (Finish) at time 1472980000` and
`errors=0 warnings=26 notes=0`, exiting 0. iverilog prints
`tb.v:243: $finish called at 1472980000 (1ps)` — the same simulated time. Verilator's
own banner says `$finish at 1ms`, which is its time report rounding; its two output
lines match exactly.

The 26 vita warnings are all `VITA-W3056 W-ELAB-FEATURE-LIMIT`, one per unconnected
output port (`stat_*`, `ptp_*`, deliberately left off the instantiation). An
unconnected output is legal and carries no hazard here; iverilog is simply silent about
the same ports.

## What the workload exercises

`eth_mac_1g` is instantiated with `DATA_WIDTH=8`, `ENABLE_PADDING=1`,
`MIN_FRAME_LENGTH=64` and PTP, PFC and PAUSE all disabled, and its GMII TX output is
registered straight back into its GMII RX input. The testbench pushes N frames of
pseudo-random length 20..279 and pseudo-random payload into `tx_axis` with a proper
tvalid/tready handshake. The stimulus comes from a 32-bit LFSR seeded `32'h1234_5678`
in the testbench — there is no `$random`, so it is tool-independent.

Real datapath covered: preamble insert and strip, minimum-frame padding, CRC32
(`lfsr.v`) inserted on TX and checked on RX, the inter-frame gap, and back-pressure.

The digest is cycle-resolution rather than an end-state check: a clocked accumulator
rotate-xors `{rx_axis_tuser, rx_axis_tlast, rx_axis_tdata}` while `rx_axis_tvalid`
holds, plus the five status strobes (`rx_error_bad_fcs`, `rx_error_bad_frame`,
`rx_start_packet`, `tx_error_underflow`, `tx_start_packet`) on every cycle after reset.
A cycle watchdog (`N*800 + 200000`) prints `WATCHDOG` before the digest if the design
ever wedges; `$finish` is explicit on both paths.

## The digest moves with the workload

The dial is linear, and each size has its own digest. All three tools agree at each of
them:

| `+N=` | RX_FRAMES / RX_BYTES | DIGEST |
|---|---|---|
| 500 | 500 / 77577 | `f60f0af7898ae327` |
| 1000 (the pinned workload) | 1000 / 156111 | `ca4945d0044f74d8` |
| 2000 | 2000 / 309719 | `885e3c0a9659c234` |

`+N=1000` is the tuned setting: it puts the iverilog reference mid-band in the 3–15 s
target, far enough above process startup that startup is a fraction of a percent of the
measurement, and `+N=500` falls below the 3 s floor.

## Mutating this workload: use the RX side

The corpus requires that a workload's digest move when its design changes. This one has
a symmetric trap: TX and RX share a single `lfsr` instance, so changing the CRC
polynomial cancels at both ends of the loopback and leaves the digest byte-identical
while the design is genuinely different. Mutate the RX-side datapath instead, and
confirm the digest moves before trusting a run of this row.

## Determinism

Repeated runs of each tool produce byte-identical stdout — the frame and byte counters
as well as the digest line. Two independent engines producing the same cycle-resolution
digest is what makes this workload oracle-clean, and vita reproduces it exactly.
