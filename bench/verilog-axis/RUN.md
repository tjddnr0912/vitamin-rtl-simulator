# bench/verilog-axis — `axis_switch` 4×4 into four `axis_fifo`

The corpus row `verilog-axis`: a 4×4 AXI-Stream switch whose four outputs each feed a
frame-mode FIFO, driven by four pseudo-random frame sources and reduced to one `DIGEST=`
line. It is the corpus's second fabric row: parameterised routing, round-robin
arbitration, register slices and frame drop. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter verilog-axis --compare` reproduces it. The
design came out of the row-9 new-design census (ROADMAP §5.2 row 13), which ran it first.

## Provenance

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-axis |
| Pinned SHA | `48ff7a7e2ef782cf778d47910cf85835c64b1bce` |
| Licence | MIT (`src/COPYING`, Copyright (c) 2014-2018 Alex Forencich) — permissive, redistribution permitted with notice |
| Clone path | `bench/verilog-axis/src/` (gitignored) |
| Upstream RTL | unmodified — `git -C src status --porcelain` is empty at that SHA |
| Testbench | `tb.v`, written for this bench, not upstream |
| Top module | `tb` |
| Lines fed to the simulators | 1598 = 1507 upstream RTL (5 files) + 91 local `tb.v` |

Reconstruct the clone with `cargo run -p corpus-runner -- fetch --run`, or by hand:

```sh
cd bench/verilog-axis
git clone https://github.com/alexforencich/verilog-axis src
git -C src checkout 48ff7a7e2ef782cf778d47910cf85835c64b1bce
```

## File list (exact, in order)

Paths are relative to `bench/verilog-axis/`; `tb.v` comes last.

```
src/rtl/axis_switch.v        410
src/rtl/axis_register.v      280
src/rtl/arbiter.v            159
src/rtl/priority_encoder.v    92
src/rtl/axis_fifo.v          566
tb.v                          91
                            ----
                            1598
```

## Commands (from `bench/verilog-axis/`)

```sh
F="src/rtl/axis_switch.v src/rtl/axis_register.v src/rtl/arbiter.v src/rtl/priority_encoder.v src/rtl/axis_fifo.v tb.v"

# iverilog (the oracle)
iverilog -g2012 -o x.vvp $F && vvp x.vvp +N=3000

# vita (one-shot, release binary)
../../target/release/vita $F +N=3000
```

`./run.sh iverilog` and `./run.sh vita` wrap the two, with the size overridable as
`N=<frames> ./run.sh …`.

> zsh trap. zsh does not word-split an unquoted `$F`, so the five paths arrive as one
> argv entry. Use `/bin/sh`, an array, or `${=F}`.

## Expected output

Both tools print the switch's own five-line address banner, 86 `CP` checkpoint lines (one
per 1000 cycles: the cycle, the running `CYCD`, the beat count), then six lines. Verbatim
tails at `+N=3000`:

```
ACC0=89dcf4f79fc5a23d
ACC1=d09b60154abc1531
ACC2=28d6fe4c789ee6e9
ACC3=666a11e66ad86ed3
CYCD=45268d3005e4fe9d CYCLES=86322 BEATS=83135 XC=189
DIGEST=d24b621c2e3346ba
```

Every stdout line is byte-identical between iverilog and vita. Both end at simulated time
863220000 (iverilog: `tb.v:88: $finish called at 863220000 (1ps)`; vita: `simulation ended
(Finish) at time 863220000`, `errors=0 warnings=24 notes=0`, exit 0). The 24 warnings are
`VITA-W3056`, six per FIFO, one per output port the testbench leaves unconnected
(`pause_ack`, `status_depth`, `status_depth_commit`, `status_overflow`, `status_bad_frame`,
`status_good_frame`); iverilog is silent about them. Two runs of each tool are
byte-identical.

## What the workload exercises

`axis_switch` with `S_COUNT=4`, `M_COUNT=4`, `DATA_WIDTH=64`, keep, id, dest and user
enabled, `UPDATE_TID=1`, `S_REG_TYPE=1` (simple buffer), `M_REG_TYPE=2` (skid buffer),
round-robin arbitration with LSB priority, and default routing (`M_BASE=0`: the top two
`tdest` bits pick the output). Each output feeds an `axis_fifo` with `DEPTH=256`,
`FRAME_FIFO=1`, `DROP_BAD_FRAME=1` (user bit 0 marks a bad frame) and
`DROP_OVERSIZE_FRAME=1`.

Each of the four sources is a 64-bit xorshift generator seeded from a fixed constant. It
offers a beat on about three cycles in four, with random `tkeep`, `tid`, `tdest` and
`tuser`, and ends a frame on one beat in eight; it stops after N frames. The output side
asserts `tready` from the same generators, so every FIFO sees back-pressure.

## The digest

Three accumulators, all over the whole run:

- `ACC0`–`ACC3`: per output, a rotate-xor of every beat that leaves the FIFO — data, keep,
  last, id, dest and user.
- `CYCD`: every falling clock edge, a rotate-xor of 20 handshake bits (source ready, switch
  valid and ready, FIFO valid and last). A cycle whose handshake bits hold an x adds a fixed
  constant instead and is counted in `XC`. Both tools count 189 such cycles, all among the
  first 196, and every x is on a FIFO's `tlast` output, which reads uninitialised RAM until
  the first beat has passed through that FIFO.
- `CYCLES`, `BEATS`, `XC`: counters.

`DIGEST` folds `CYCD`, the four accumulators and the three counters. Final state is never
compared: a beat that differs anywhere moves its accumulator, and a handshake that moves by
one cycle moves `CYCD` (the `arbiter.v` mutation below changes timing and order).

The testbench is the row-9 census's `tb_axis.v` with three changes: the module is named
`tb`, the frame count is a `+N=` plusarg rather than a parameter, and the `DIGEST` line is
added. Every other line it prints is unchanged: with the instance name mapped back, the
census's trace (md5 `4614ce43345864ce`, its first 16 hex digits) is reproduced exactly.

## The digest moves with the design

Contract rule 5, on copies of upstream files, with the clone untouched. Each change is one
line. Both tools agree on every mutant.

| mutation | iverilog | vita |
|---|---|---|
| none (the pin) | `d24b621c2e3346ba` | `d24b621c2e3346ba` |
| `axis_fifo.v:230`, bit 0 of every word leaving the FIFO RAM flipped | `324b6203e633402d` | `324b6203e633402d` |
| `axis_switch.v:236`, default routing compares `tdest` against `k ^ 1` (outputs 0↔1, 2↔3) | `c06d231f1a6ca556` | `c06d231f1a6ca556` |
| `arbiter.v:121`, the round-robin mask no longer advances past the grantee | `918d84342048171c` | `918d84342048171c` |
| control: `axis_fifo.v:295`, the bad-frame status strobe (an unconnected output) stuck at 0 | the pin | the pin |

The arbiter mutation changes the order and timing of frames on each output: `CYCD` and all
four accumulators move, `CYCLES` becomes 86455 and `BEATS` 82697.

## Workload dial

`+N=<frames per source>` scales the run linearly; each size has its own digest. `+N=3000`
puts the iverilog reference in the corpus's 3–15 s band.
