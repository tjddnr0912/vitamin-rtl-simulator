# bench/verilog-uart — `uart` in loopback

The corpus row `verilog-uart`: a UART whose transmit line is wired straight back into its
receive line, fed N pseudo-random bytes and reduced to one `DIGEST=` line. It is a
serial-line workload — two small counter-driven state machines and a handshake on each
side. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter verilog-uart --compare` reproduces it. The design
came out of the row-9 new-design census (ROADMAP §5.2 row 15), which ran it first.

## Provenance

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-uart |
| Pinned SHA | `1b867e53af738e4a8bc7c839ca2f1c07f40382dc` |
| Licence | MIT (`src/COPYING`, Copyright (c) 2014-2017 Alex Forencich) — permissive, redistribution permitted with notice |
| Clone path | `bench/verilog-uart/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Testbench | `tb.v`, written for this bench, not upstream |
| Top module | `tb` |
| Lines fed to the simulators | 431 = 370 upstream RTL (3 files) + 61 local `tb.v` |

Reconstruct the clone with `cargo run -p corpus-runner -- fetch --run`, or by hand:

```sh
cd bench/verilog-uart
git clone https://github.com/alexforencich/verilog-uart src
git -C src checkout 1b867e53af738e4a8bc7c839ca2f1c07f40382dc
```

## File list (exact, in order)

```
src/rtl/uart.v      113
src/rtl/uart_rx.v   142
src/rtl/uart_tx.v   115
tb.v                 61
                   ----
                    431
```

## Commands (from `bench/verilog-uart/`)

```sh
F="src/rtl/uart.v src/rtl/uart_rx.v src/rtl/uart_tx.v tb.v"

# iverilog (the oracle)
iverilog -g2012 -o x.vvp $F && vvp x.vvp +N=20000

# vita (one-shot, release binary)
../../target/release/vita $F +N=20000
```

`./run.sh iverilog` and `./run.sh vita` wrap the two (`N=<bytes>` and `TRACE=1` override the
size and add `+TRACE`). zsh does not word-split an unquoted `$F`; use `/bin/sh`.

## Expected output

At `+N=20000` both tools print, byte-identical:

```
SENT=20000 GOT=20000 ACC=5c207af72615bf37 CYCD=70fa3a6cafdf8426 CYC=1620012
DIGEST=7ba8527cf36dd903
```

Both end at simulated time 16200120000 (iverilog: `tb.v:58: $finish called at 16200120000
(1ps)`; vita: `simulation ended (Finish) at time 16200120000`, `errors=0 warnings=0
notes=0`, exit 0). Two runs of each tool are byte-identical.

## What the workload exercises

`uart` with `DATA_WIDTH=8` and `prescale=1` (eight clocks per bit), `txd` driving `rxd`. A
32-bit LFSR offers a byte on about half the cycles the transmitter is ready, and the
receive side takes bytes with random back-pressure (`tready` on three cycles in four). The
run ends when all N bytes have been received, or at a cycle watchdog.

## The digest

- `ACC`: a rotate-xor of every received byte, in order.
- `CYCD`: every falling clock edge, a rotate-xor of seven line and status bits — `txd`,
  `tx_busy`, `rx_busy`, `rx_overrun_error`, `rx_frame_error`, the receiver's `tvalid` and
  the transmitter's `tready`.
- `SENT`, `GOT`, `CYC`: counters.

`DIGEST` folds `CYCD`, `ACC` and the three counters. With `+TRACE` the testbench also prints
every received byte and every change of the seven bits as a line (213,755 lines plus the
counter line at `+N=20000`): that output is byte-identical to the row-9 census's trace
(md5 `43738ff7e0d7d0cd`, its first 16 hex digits) in both tools. The census's `tb_uart.v`
printed those lines unconditionally; this `tb.v` is it with the module named `tb`, the byte
count a `+N=` plusarg, the printing behind `+TRACE`, and the digest added.

## The digest moves with the design

Contract rule 5, on copies of upstream files, with the clone untouched. Each change is one
line. Both tools agree on every mutant.

| mutation | iverilog | vita |
|---|---|---|
| none (the pin) | `7ba8527cf36dd903` | `7ba8527cf36dd903` |
| `uart_tx.v:97`, bit 0 of every transmitted byte flipped | `4ba9ca7e3cc3ff56` | `4ba9ca7e3cc3ff56` |
| `uart_rx.v:133`, the receiver's start-bit sampling point one cycle later | `a7885eaf16e19495` | `a7885eaf16e19495` |
| `uart_rx.v:119`, every received data bit inverted | `7ba852d6f36dd8a9` | `7ba852d6f36dd8a9` |
| control: `uart_rx.v:127`, the frame-error flag never set (no frame error occurs in a clean loopback) | the pin | the pin |

The loopback is not symmetric under these: transmitter and receiver are separate modules,
so each mutation changes one end only. The start-bit mutation changes timing only — every
byte still arrives intact (`ACC` unchanged), and only the per-cycle `CYCD` moves — which is
the case a final-state digest would miss. The receive-data mutation is the converse: `ACC`
moves and `CYCD` does not.

## Workload dial

`+N=<bytes>`; each size has its own digest. `+N=20000` puts the iverilog reference in the
corpus's 3–15 s band.
