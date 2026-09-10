# bench/biriscv — pinned benchmark recipe

The corpus row `biriscv`: a dual-issue RV32 core (`riscv_core`) running a real RISC-V
program image out of a TCM memory model, free-running for a caller-chosen number of
cycles. Big, branchy, structural RTL — about 8.4 k lines of `always` blocks across 20
core files, the largest design in the corpus. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter biriscv --compare` reproduces it.

## Provenance

| | |
|---|---|
| Repo | https://github.com/ultraembedded/biriscv |
| Pinned SHA | `6af9c4be5a0807d368eaad5e49af52322e31d073` |
| Licence | Apache-2.0 (`src/LICENSE`, verbatim upstream text) |
| Upstream RTL | unmodified — every `src/...` file is upstream byte-for-byte |
| Committed here | `tb.v` (derived from upstream's `tb/tb_core_icarus/tb_top.v`), `files.txt`, `run.sh`, `prepare.sh`, `mkhex.py`, this file |
| Not committed | `src/` (the clone) and `prog.hex` (extracted from upstream's `test.elf`) |
| Lines fed to the simulators | 8830 = 8431 upstream core + 218 upstream tb memory model + 181 local `tb.v` |
| Top module | `tb_top` |

Reconstruct the tree:

```sh
cd bench/biriscv
git clone https://github.com/ultraembedded/biriscv src
git -C src checkout 6af9c4be5a0807d368eaad5e49af52322e31d073
sh prepare.sh                       # regenerates prog.hex — 11856 lines
```

`corpus-runner fetch --run` performs exactly these steps. `prepare.sh` is a thin
wrapper over `python3 mkhex.py src/tb/tb_core_icarus/test.elf > prog.hex`, and the
image is byte-reproducible from that command (sha256 prefix
`e725f66eda2afe796729ef22b2381fc7`). It is upstream's shipped `test.elf`: a single
`PT_LOAD` at vaddr `0x80000000`, `filesz` 7672 and `memsz` 11856, flattened to one byte
per line and zero-filled to `memsz`. `tb.v` reads it with `$readmemh` and pushes it
into the TCM through upstream's own `u_mem.write(addr, byte)` task, which is what
upstream's testbench does. Building the image the upstream way would need
`riscv32-unknown-elf-objcopy` at fetch time, which is why it is extracted in Python
here.

## File list (exact, in order — this is `files.txt`)

```
src/src/core/biriscv_alu.v
src/src/core/biriscv_csr_regfile.v
src/src/core/biriscv_csr.v
src/src/core/biriscv_decode.v
src/src/core/biriscv_decoder.v
src/src/core/biriscv_defs.v
src/src/core/biriscv_divider.v
src/src/core/biriscv_exec.v
src/src/core/biriscv_fetch.v
src/src/core/biriscv_frontend.v
src/src/core/biriscv_issue.v
src/src/core/biriscv_lsu.v
src/src/core/biriscv_mmu.v
src/src/core/biriscv_multiplier.v
src/src/core/biriscv_npc.v
src/src/core/biriscv_pipe_ctrl.v
src/src/core/biriscv_regfile.v
src/src/core/biriscv_trace_sim.v
src/src/core/biriscv_xilinx_2r1w.v
src/src/core/riscv_core.v
src/tb/tb_core_icarus/tcm_mem.v
src/tb/tb_core_icarus/tcm_mem_ram.v
tb.v
```

The first 20 are upstream's own `$(wildcard src/core/*.v)` — nothing is cut down or
added. `biriscv_trace_sim.v` is in that wildcard but is instantiated only under
`` `ifdef verilator ``, which makes it an uninstantiated second root; see the `-s` /
`--top` pin below.

## Commands

Run with the working directory set to `bench/biriscv/` — `prog.hex` is opened relative
to it. `$FILES` below is `$(cat files.txt)`, in that order.

iverilog (compile plus run; the compile is a fraction of a second, about 1% of the
total):

```sh
iverilog -g2012 -s tb_top -I src/src/core -D TRACE=0 -o x.vvp $FILES
vvp x.vvp +N=50000
```

vita (one-shot, release binary):

```sh
../../target/release/vita --top tb_top -I src/src/core -D TRACE=0 $FILES +N=50000
```

`./run.sh 50000` runs both of the above.

`+N=50000` is the pinned workload size, in cycles. The dial is linear, and this value
puts the iverilog reference mid-band in the 3–15 s target, so startup is fully
amortised and the run is still cheap to repeat.

### Why each flag

| Flag | Why |
|---|---|
| `-s tb_top` / `--top tb_top` | Required for a fair comparison. Without it the two tools elaborate different designs: vita reports `VITA-W3057 W-ELAB-AUTOTOP-AMBIGUOUS` and elaborates `biriscv_trace_sim` as a second independent top, which iverilog drops. The digest happens not to change — the extra root drives nothing — but the elaboration work is not the same, so pin it |
| `-D TRACE=0` | Upstream's trace hooks off |
| No `-D verilog_sim` | Upstream's makefile passes it. With it, `biriscv_csr_regfile.v`'s `HAS_SIM_CTRL` block turns the program's exit-CSR write into an RTL-side `$finish` at about 3122 cycles, which caps the workload and makes `+N` meaningless; it also enables a `$write("%c")` putc path that spams the banner. Without it the core free-runs and `+N` is the real dial. Both tools get the identical flag set either way |

## Expected output

Both tools, verbatim:

```
CYCLES=50000
DIGEST=22481d1cacf87584
```

iverilog then prints `tb.v:108: $finish called at 500055 (1s)`; vita prints
`simulation ended (Finish) at time 500055` and `errors=0 warnings=16 notes=0`. Same
finish time, same digest, exit 0 on both.

A cheap smoke point: `./run.sh 3000` gives `CYCLES=3000` and
`DIGEST=e4c202c497e78e7d` on both tools, finishing at 30055.

## Determinism

Repeated runs produce the same digest, and the entire stdout of each tool is
byte-identical across its runs — one hash per tool, not just per digest line.

## Verilator is deliberately not run

This design is materially X-dependent at reset (see the `VITA-W4029` diagnostics
below). Verilator's 2-state zeroing would change the digest by construction, so it
would be a non-oracle rather than a third opinion. The manifest records iverilog as
the sole oracle for this row.

## What the digest observes

`tb.v` accumulates, on each `posedge clk` after reset, a rotate-xor over a 64-bit
observation vector built from every signal `riscv_core` drives: `mem_i_pc`,
`mem_d_addr`, `mem_d_data_wr`, `mem_d_data_rd`, `mem_d_req_tag`, and all the
rd/wr/cacheable/invalidate/writeback/flush strobes.

The core boots with X in the register file and the fetch FIFO, so a naive xor poisons
the whole digest to x. Instead the testbench accumulates two masks per cycle using
`===` — a "bit is 1" mask and a "bit is known (0 or 1)" mask — rotated by different
amounts and combined at the end. That keeps the digest a real number while leaving it a
genuine 3-state observer: 0, x and 1 each produce a distinct digest contribution,
because x is 0 in the known-mask where 0 is 1. A `#200000000` watchdog prints
`WATCHDOG` and a fixed sentinel digest if the run ever wedges.

## vita diagnostics — 16 warnings, errors=0, none blocking

| Code | Count | What |
|---|---:|---|
| `VITA-W1017` | 1 | No `` `timescale `` in the design; 1ns/1ns assumed. This is iverilog's default here too, which is why the 500055 finish times line up |
| `VITA-W3056` | 6 | Unconnected output ports in `u_lsu_request` and `u_pipe1_ctrl`. Upstream leaves them dangling; iverilog is silent about it |
| `VITA-W4029` | 9 | Unknown array word index at time 0 on `ras_stack_q`, `bht_sat_q`, and the decode FIFO's `valid0_q` / `valid1_q` / `pc_q` / `ram_q` — "read X / write ignored", ending in a suppression line. The design genuinely indexes arrays with X during the first cycles, and both tools agree on the outcome |

## Testbench requirement for this corpus

Any candidate testbench in this corpus must deassert reset away from the active clock
edge. A blocking write to `rst` on the same posedge the digest `always` block samples
is an unresolved Verilog race: it produces a clean one-clock offset between two
simulators that reads exactly like a scheduling defect in one of them and is not.
`tb.v` deasserts with `#1 rst = 0;` for that reason, and the two tools then agree
bit-for-bit at every N.
