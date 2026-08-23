# bench/biriscv — pinned benchmark recipe

A dual-issue RV32 core (`riscv_core`) running a real RISC-V program image out of a
TCM memory model, free-running for a caller-chosen number of cycles. Big, branchy,
structural RTL: ~8.4 k lines of `always` blocks across 20 core files. This is the
opposite shape from `keccak_f_arr` (tight in-function bit loops), and vita's
wall-clock sign flips accordingly — vita **wins** here.

## Provenance

| | |
|---|---|
| Repo | https://github.com/ultraembedded/biriscv |
| Pinned SHA | `6af9c4be5a0807d368eaad5e49af52322e31d073` |
| License | **Apache-2.0** (`src/LICENSE`, verbatim upstream text) |
| RTL modified? | **No.** Every `src/...` file is upstream, byte-for-byte. |
| Local additions | `tb.v` (testbench, derived from upstream `tb/tb_core_icarus/tb_top.v`), `prog.hex` (program image extracted from upstream `test.elf`), `files.txt`, `run.sh`, `mkhex.py`, this file. |
| LOC fed to the simulators | **8830** = 8431 upstream core + 218 upstream tb memory model + 181 local `tb.v` |

`bench/biriscv/src/` is gitignored. To recreate it:

```sh
cd bench/biriscv
git clone https://github.com/ultraembedded/biriscv src
git -C src checkout 6af9c4be5a0807d368eaad5e49af52322e31d073
python3 mkhex.py src/tb/tb_core_icarus/test.elf > prog.hex   # 11856 lines
```

`prog.hex` is byte-reproducible from that command (sha256 prefix
`e725f66eda2afe796729ef22b2381fc7`). It is upstream's shipped `test.elf`, single
`PT_LOAD` at vaddr `0x80000000`, `filesz` 7672 / `memsz` 11856, flattened to one
byte per line and zero-filled to `memsz`. `tb.v` `$readmemh`s it and pushes it into
the TCM via upstream's own `u_mem.write(addr, byte)` task — exactly what upstream's
testbench does. Building it the upstream way instead would need
`riscv32-unknown-elf-objcopy` at build time, which is why the image is extracted in
Python here.

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

The first 20 are upstream's own `$(wildcard src/core/*.v)`; nothing was cut down or
added. `biriscv_trace_sim.v` is in that wildcard but is only instantiated under
`` `ifdef verilator ``, so it is an **uninstantiated second root** — see the
`-s` / `--top` pin below.

## Commands

Run with **CWD = `bench/biriscv/`** (`prog.hex` is opened relative to CWD).
`$FILES` below is `$(cat files.txt)`, in that order.

iverilog (13.x; compile + run — compile is ~0.09 s, i.e. ~1 % of the total):

```sh
iverilog -g2012 -s tb_top -I src/src/core -D TRACE=0 -o x.vvp $FILES
vvp x.vvp +N=50000
```

vita (one-shot; release binary at `target/release/vita`):

```sh
../../target/release/vita --top tb_top -I src/src/core -D TRACE=0 $FILES +N=50000
```

`./run.sh 50000` runs exactly both of the above.

**Tuned plusarg: `+N=50000`** (cycles). The workload dials linearly; `+N=20000`
puts iverilog at ~3.9 s, `+N=50000` at 9.42 s — mid-window for the 3–15 s target,
so startup is fully amortised and the run is still cheap to repeat.

### Why each flag

- `-s tb_top` / `--top tb_top` — **required for a fair comparison.** Without it the
  two tools elaborate different designs: vita reports `VITA-W3057
  W-ELAB-AUTOTOP-AMBIGUOUS` and elaborates `biriscv_trace_sim` as a second
  independent top, which iverilog drops. The digest happens not to change (the
  extra root drives nothing), but the elaborate work is not the same, so pin it.
- `-D TRACE=0` — upstream's trace hooks off.
- **No `-D verilog_sim`** (upstream's makefile passes it). With it,
  `biriscv_csr_regfile.v`'s `HAS_SIM_CTRL` block turns the program's exit-CSR write
  into an RTL-side `$finish` at ~3122 cycles — that caps the workload and makes
  `+N` meaningless, and it also enables a `$write("%c")` putc path that spams the
  banner. Dropping it leaves the core free-running so `+N` is the real dial. Both
  tools get the identical flag set either way.

## Expected output

Both tools, verbatim:

```
CYCLES=50000
DIGEST=22481d1cacf87584
```

iverilog then prints `tb.v:108: $finish called at 500055 (1s)`; vita then prints
`simulation ended (Finish) at time 500055` / `errors=0 warnings=16 notes=0`.
Same finish time, same digest, exit 0 both.

Second workload point, for a cheap smoke test — `./run.sh 3000` gives
`CYCLES=3000` / `DIGEST=e4c202c497e78e7d` on both tools, finish at 30055.

## Measured (2026-08-23, macOS arm64, Darwin 25.6.0)

4 interleaved pairs (iverilog, vita, iverilog, vita, …); **first pair discarded**
as cache-warming; median of the remaining 3.

| tool | rep2 | rep3 | rep4 | **median** |
|---|---|---|---|---|
| iverilog (compile + `vvp`) | 9.42 | 9.38 | 9.47 | **9.42 s** |
| vita (one-shot) | 5.01 | 5.01 | 5.02 | **5.01 s** |

**vita is 1.88× faster than iverilog here.** Spread is ~1 % on both tools.

Determinism: all 8 runs produced `DIGEST=22481d1cacf87584`, and the **entire
stdout** of each tool was byte-identical across its 4 runs (one sha256 per tool,
not just per digest line).

Verilator is deliberately **not** run on this design. It is materially X-dependent
at reset (see `VITA-W4029` below); verilator's 2-state zeroing would change the
digest by construction, so it would be a non-oracle rather than a third opinion.

## What the digest observes

`tb.v` accumulates, per `posedge clk` after reset, a rotate-xor over a 64-bit
observation vector built from **every signal `riscv_core` drives**: `mem_i_pc`,
`mem_d_addr`, `mem_d_data_wr`, `mem_d_data_rd`, `mem_d_req_tag`, and all the
rd/wr/cacheable/invalidate/writeback/flush strobes.

The core boots with X in the regfile and the fetch FIFO, so a naive xor poisons the
whole digest to `x`. Instead the tb accumulates **two** masks per cycle using `===`
— a "bit is 1" mask and a "bit is known (0 or 1)" mask — rotated by different
amounts and combined at the end. This keeps the digest a real number while staying
a genuine **3-state** observer: 0, x and 1 each produce a distinct digest
contribution (verified directly — one bit flipped 0→x→1 gives
`…ffffff5a` / `…fffff7ffffff5a` / `…ffffff52`). An earlier note claiming x-vs-0 is
indistinguishable here is **wrong**; x is 0 in the known-mask where 0 is 1.

## vita diagnostics (all warnings, exit 0, none blocking)

- `VITA-W3056` ×6 — unconnected output ports in `u_lsu_request` and `u_pipe1_ctrl`.
  Upstream leaves them dangling; iverilog is silent about it.
- `VITA-W4029 W-RUN-RANGE-UNKNOWN` ×8 + "further suppressed", all at time 0 —
  unknown array word index on `ras_stack_q`, `bht_sat_q`, and the decode FIFO's
  `valid0_q` / `valid1_q` / `pc_q` / `ram_q`: "read X / write ignored". The design
  genuinely indexes arrays with X during the first cycles and both tools agree on
  the outcome.
- `VITA-W1017` — no `` `timescale ``; 1ns/1ns assumed. Same as iverilog's default
  here, which is why the 500055 finish times line up.
- `VITA-W3057` (auto-top ambiguous) is **gone** now that `--top tb_top` is pinned.

## Trap recorded — do not reintroduce

The one mismatch ever seen on this design was a testbench bug, not a simulator bug.
An earlier `tb.v` deasserted reset with `repeat(5) @(posedge clk); rst = 0;` — a
blocking write to `rst` racing the digest `always` block at the **same** posedge.
iverilog gave `4b5b21d7ddff075d` / finish@200055, vita gave `c210d0540761d80e` /
finish@200045: a clean one-clock offset that reads exactly like a scheduling bug and
is not — it is an unresolved Verilog race in the testbench. Moving the deassert off
the edge (`#1 rst = 0;`, which is what `tb.v` does now) made both tools agree
bit-for-bit at every N. **Any candidate testbench in this corpus must deassert reset
away from the active edge, or the benchmark manufactures fake differentials.**
