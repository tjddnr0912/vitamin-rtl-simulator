# bench/verilog-i2c — `i2c_master` and `i2c_slave` on one bus

The corpus row `verilog-i2c`: an I2C master and an I2C slave sharing one open-drain bus,
driven by a pseudo-random command and data stream and reduced to one `DIGEST=` line. It is
a bit-level protocol workload — two state machines, a clock prescaler and input glitch
filters, many small events and few wide datapaths. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter verilog-i2c --compare` reproduces it. The design
came out of the row-9 new-design census (ROADMAP §5.2 row 14), which ran it first.

## Provenance

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-i2c |
| Pinned SHA | `a65be4045e898a52e791c6ee71f8f79a7cd2e129` |
| Licence | MIT (`src/COPYING`, Copyright (c) 2015-2017 Alex Forencich) — permissive, redistribution permitted with notice |
| Clone path | `bench/verilog-i2c/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Testbench | `tb.v`, written for this bench, not upstream |
| Top module | `tb` |
| Lines fed to the simulators | 1490 = 1400 upstream RTL (2 files) + 90 local `tb.v` |

Reconstruct the clone with `cargo run -p corpus-runner -- fetch --run`, or by hand:

```sh
cd bench/verilog-i2c
git clone https://github.com/alexforencich/verilog-i2c src
git -C src checkout a65be4045e898a52e791c6ee71f8f79a7cd2e129
```

## File list (exact, in order)

```
src/rtl/i2c_master.v   897
src/rtl/i2c_slave.v    503
tb.v                    90
                      ----
                      1490
```

## Commands (from `bench/verilog-i2c/`)

```sh
F="src/rtl/i2c_master.v src/rtl/i2c_slave.v tb.v"

# iverilog (the oracle)
iverilog -g2012 -o x.vvp $F && vvp x.vvp +N=900000

# vita (one-shot, release binary)
../../target/release/vita $F +N=900000
```

`./run.sh iverilog` and `./run.sh vita` wrap the two (`N=<cycles>` and `TRACE=1` override
the size and add `+TRACE`). zsh does not word-split an unquoted `$F`; use `/bin/sh`.

## Expected output

At `+N=900000` both tools print, byte-identical:

```
NCMD=3148 NRD=813 NWR=1846
DIGEST=e8bac662acedfaec
```

Both end at simulated time 9000000000 (iverilog: `tb.v:87: $finish called at 9000000000
(1ps)`; vita: `simulation ended (Finish) at time 9000000000`, `errors=0 warnings=6 notes=0`,
exit 0). The six warnings are `VITA-W2003`: `i2c_slave.v` drives six nets with `assign`
without declaring them (`scl_posedge`, `scl_negedge`, `sda_posedge`, `sda_negedge`,
`start_bit`, `stop_bit`, lines 232–238), which IEEE 1364-2005 §3.5 makes implicit 1-bit
wires; iverilog is silent. Two runs of each tool are byte-identical.

## What the workload exercises

`i2c_master` with `prescale=2` and `stop_on_idle=0`; `i2c_slave` with `FILTER_LEN=4`,
device address `0x50` (mask `0x7f`) and `release_bus=0`. The bus is the wired AND of both
devices' `scl`/`sda` outputs, each released (high) while its `_t` enable is set.

A 32-bit LFSR issues a command whenever the master is ready and one LFSR bit is set: the
address is `0x50` (the slave) or `0x51` (nobody, so the master sees a missed ACK) with equal
probability, the operation is a read, a write, a multiple write or a stop, and start and
stop flags are random. The master's write-data stream and the slave's transmit stream are
LFSR bytes with random `tlast`; both receive sides take bytes with random back-pressure.

## The digest

The testbench folds three kinds of event into one 64-bit rotate-xor, each tagged and
stamped with the cycle number:

- every change of a 12-bit status vector sampled at the falling clock edge — `scl`, `sda`,
  the master's `busy`, `bus_control`, `bus_active` and `missed_ack`, the slave's `busy`,
  `bus_addressed` and `bus_active`, and the three stream `ready` outputs — together with
  the slave's latched bus address;
- every byte the master reads (data and `tlast`);
- every byte the slave receives (data and `tlast`).

The three counters are folded in last. With `+TRACE` the testbench also prints each event
as a line (`C`, `MRD`, `SWR`, 187,186 lines plus the counter line at `+N=900000`): that
output is byte-identical to the row-9 census's trace (md5 `726994eae6ef80b9`, its first 16
hex digits) in both tools. The census's `tb_i2c.v` printed those lines unconditionally; this
`tb.v` is it with the module named `tb`, the cycle count a `+N=` plusarg, the printing
behind `+TRACE`, and the digest added.

## The digest moves with the design

Contract rule 5, on copies of upstream files, with the clone untouched. Each change is one
line. Both tools agree on every mutant.

| mutation | iverilog | vita |
|---|---|---|
| none (the pin) | `e8bac662acedfaec` | `e8bac662acedfaec` |
| `i2c_master.v:540`, every data bit the master transmits inverted | `141f524e455625b6` | `141f524e455625b6` |
| `i2c_slave.v:345`, bit 0 of every byte the slave receives flipped on its output | `d480e9cef85ece23` | `d480e9cef85ece23` |
| `i2c_master.v:568`, every bit the master reads inverted | `4553fd818db5131f` | `4553fd818db5131f` |
| `i2c_master.v:655`, one extra prescale cycle per transmitted data bit | `aa8edaf7c97b5f07` | `aa8edaf7c97b5f07` |
| control: `i2c_master.v:401`, the stop issued under `stop_on_idle` (tied 0 here) | the pin | the pin |

The slave mutation is asymmetric by construction: it changes the slave's output stream
only, not the bus. The prescale mutation changes only the bus timing; fewer transfers then fit in the run
(`NCMD=3009 NRD=804 NWR=1792`).

## Workload dial

`+N=<cycles>`; each size has its own digest. `+N=900000` puts the iverilog reference in the
corpus's 3–15 s band.
