# SERV — vitamin benchmark recipe (hardened)

Bit-serial RV32I SoC (`servant`) running a real program for 500 000 clock cycles.
Reproduce everything below from this file alone. `bench/*` is **gitignored**, so
this file carries the full reconstruction, including the testbench source.

| | |
|---|---|
| Upstream repo | <https://github.com/olofk/serv> |
| Pinned SHA | `41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2` (2026-07-10) |
| License | **ISC** (permissive; `Copyright 2019, Olof Kindgren`) — redistribution OK with notice |
| Top module | `tb` (must be given explicitly: `--top tb`) |
| Workload | `+N=500000` |
| Expected digest | `DIGEST=f3f45af36093b2b1` |
| RTL fed to the simulators | 27 files, **4318 lines** |
| Measured on | macOS 26.6.2, arm64 · Icarus Verilog 13.0 (stable) · vita 0.1.0 (release) · Verilator 5.050 |

---

## 1. Reconstruct the tree

```sh
cd <repo>/bench/serv
git clone https://github.com/olofk/serv src
git -C src checkout 41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2
cp src/sw/blinky.hex .          # firmware, loaded by $readmemh via the `memfile` parameter
```

`blinky.hex` is upstream's `sw/blinky.hex` byte-for-byte (11 lines). It is a real
RV32I program: `lui/addi/sb/xori/and` plus a 0x100000-iteration `addi`/`bne` delay
loop. The bit-serial core takes ~35 clocks per instruction, so 500 k cycles is a
scheduler-throughput workload, not a startup measurement.

Then write `tb.v` (section 5) and `files.txt` (section 2) next to this file.

## 2. File list — `files.txt`, exact order

Order matters only in that `tb.v` comes first; it is the order actually measured.

```
tb.v
src/rtl/serv_bufreg.v
src/rtl/serv_bufreg2.v
src/rtl/serv_alu.v
src/rtl/serv_csr.v
src/rtl/serv_ctrl.v
src/rtl/serv_decode.v
src/rtl/serv_immdec.v
src/rtl/serv_mem_if.v
src/rtl/serv_rf_if.v
src/rtl/serv_rf_ram_if.v
src/rtl/serv_rf_ram.v
src/rtl/serv_state.v
src/rtl/serv_debug.v
src/rtl/serv_top.v
src/rtl/serv_rf_top.v
src/rtl/serv_aligner.v
src/rtl/serv_compdec.v
src/servile/servile_arbiter.v
src/servile/servile_mux.v
src/servile/servile_rf_mem_if.v
src/servile/servile.v
src/servant/servant_timer.v
src/servant/servant_gpio.v
src/servant/servant_mux.v
src/servant/servant_ram.v
src/servant/servant.v
```

## 3. Commands

All runs are from `bench/serv/` (the `$readmemh` path `blinky.hex` is relative to cwd).

**iverilog (the oracle)** — compile and run are timed together:

```sh
/opt/homebrew/bin/iverilog -g2012 -o x.vvp $(cat files.txt) && \
/opt/homebrew/bin/vvp x.vvp +N=500000
```

**vita** — one-shot, unmodified upstream RTL:

```sh
<repo>/target/release/vita --top tb $(cat files.txt) +N=500000
```

**vita** — probe copy (see section 6), the variant that actually simulates:

```sh
<repo>/target/release/vita --top tb $(cat pf.txt) +N=500000
```

`--top tb` is required. Without it vita's auto-top picks three uninstantiated
roots (`tb`, `serv_rf_top`, `servile_rf_mem_if`) and elaborates all of them,
tripling the diagnostic noise.

## 4. Results

Protocol: **4 interleaved rounds** in the order iverilog → vita → iverilog(probe)
→ vita(probe); **round 0 discarded** (cache warming); median of rounds 1–3.
Never A-then-B.

| run | median | all four rounds | exit | digest |
|---|---|---|---|---|
| iverilog, unmodified list | **7.36 s** | 7.26 / 7.46 / 7.36 / 7.25 | 0 | `f3f45af36093b2b1` |
| vita, unmodified list | **0.02 s** | 0.02 / 0.02 / 0.02 / 0.02 | **1 (loud)** | none — dies in elaborate |
| iverilog, probe list | 7.41 s | 7.21 / 7.41 / 7.47 / 7.35 | 0 | `f3f45af36093b2b1` |
| vita, probe list | **9.10 s** | 9.00 / 9.21 / 9.08 / 9.10 | 0 | `f3f45af36093b2b1` |

iverilog compile is 0.04–0.05 s of the 7.4 s; the rest is `vvp`.
**vita = 0.81× iverilog** on this workload (7.36 / 9.10).

**Determinism: confirmed.** Every tool produced a byte-identical digest on all four
of its runs (four independent iverilog compiles included). No variance.

**Cross-tool match: verified here, not inherited.** Verbatim stdout:

```
                        # iverilog, unmodified list
Preloading tb.dut.ram from blinky.hex
WARNING: src/servant/servant_ram.v:45: $readmemh(blinky.hex): Not enough words in the file for the requested range [0:2047].
CYCLES=500000
DIGEST=f3f45af36093b2b1
tb.v:66: $finish called at 5000155 (1s)
```
```
                        # vita, probe list
Preloading tb.dut.ram from blinky.hex
CYCLES=500000
DIGEST=f3f45af36093b2b1
simulation ended (Finish) at time 5000155
```

Both `CYCLES=` and `DIGEST=` lines are identical, and both end at time 5000155.
(iverilog additionally warns that `blinky.hex` under-fills the 2048-word range;
vita stays quiet. Harmless — the tail stays at its reset value in both, which is
why the digests agree.)

**The agreement is not N-specific.** The digest is a function of the run length,
so a fixed point would be invisible. Checked at three sizes:

| N | iverilog | vita (probe) | |
|---|---|---|---|
| 123457 | `5047014c6f49c607` | `5047014c6f49c607` | match |
| 500000 | `f3f45af36093b2b1` | `f3f45af36093b2b1` | match |
| 1000000 | `7380a1a2e0e749e0` | `7380a1a2e0e749e0` | match |

**Workload tuning.** `+N=500000` gives an iverilog median of 7.36 s — inside the
3–15 s target, near the middle. N scales linearly (`+N=2000000` → ~29 s), so
`+N=200000` (~3 s) is the floor and `+N=1000000` (~15 s) the ceiling if you need
a different point. **Keep `+N=500000`**; the digests above are for that value only.

**Do not use verilator as an oracle for this design.** Re-verified here (Verilator
5.050): it runs in 0.18 s and reports `DIGEST=e7e8b5e6c1276563`, which does **not**
match. That is expected, not a vita bug:
`serv_rf_ram` drives `rdata <= i_ren ? memory[i_raddr] : {width{1'bx}}` and the
RF/RAM start uninitialized, so the design genuinely depends on 4-state semantics.
iverilog and vita agree; verilator's 2-state approximation cannot.

## 5. `tb.v` (written for this corpus — reproduce verbatim)

Notes on why it looks the way it does:
- `` `define SERV_CLEAR_RAM `` at the top carries across the whole file list in all
  three tools, so the RF zeroing works with no `-D` flag.
- The digest **must** mask the wishbone taps with `stb`/`ack`. SERV drives adr/rdt
  X whenever stb is low; an ungated accumulator returns `xxxxxxxxxxxxxxxx`.
  `wb_mem_sel` is X on 10 cycles during the first fetch even after gating, so it is
  excluded from the digest.
- `plus_ok = $value$plusargs(...)` is split from the `if` deliberately: vita
  supports `$value$plusargs` only as the direct rhs of a blocking assignment
  (see section 7).
- `memfile` is passed as a parameter rather than `$readmemh(f, dut.ram.mem)`,
  which is what upstream's own `bench/servant_sim.v` does — vita cannot do a
  hierarchical reference to an unpacked array (section 7).

```verilog
// Benchmark testbench for SERV (olofk/serv) -- written for vitamin bench corpus.
// Deterministic, self-terminating, prints exactly one DIGEST line.
// Workload scales with +N=<cycles>.
`define SERV_CLEAR_RAM
`default_nettype none

module tb;

   reg          clk = 1'b0;
   reg          rst = 1'b1;
   wire         q;

   integer      n;
   integer      i;
   reg          plus_ok;
   integer      cyc;
   reg [63:0]   acc;

   servant
     #(.memfile  ("blinky.hex"),
       .memsize  (8192),
       .sim      (0),
       .debug    (1'b0),
       .with_csr (1),
       .compress (1'b0),
       .width    (1))
   dut (clk, rst, q);

   always #5 clk = ~clk;

   // Bus taps.  SERV's wishbone lines are only meaningful while stb/ack is
   // asserted; mask them off otherwise so the digest never eats an X.
   wire        ms   = dut.wb_mem_stb;
   wire        ma   = dut.wb_mem_ack;
   wire        es   = dut.wb_ext_stb;
   wire [31:0] madr = ms ? dut.wb_mem_adr : 32'd0;
   wire [31:0] mrdt = ma ? dut.wb_mem_rdt : 32'd0;
   wire        mwe  = ms ? dut.wb_mem_we  : 1'b0;
   wire [31:0] eadr = es ? dut.wb_ext_adr : 32'd0;
   wire [31:0] edat = es ? dut.wb_ext_dat : 32'd0;

   initial begin
      cyc = 0;
      acc = 64'd0;
      plus_ok = $value$plusargs("N=%d", n);
      if (!plus_ok)
        n = 500000;

      rst = 1'b1;
      repeat (16) @(posedge clk);
      rst = 1'b0;
      repeat (4*n + 4000) @(posedge clk);
      $display("WATCHDOG");
      $finish;
   end

   always @(posedge clk) if (!rst) begin
      cyc <= cyc + 1;
      acc <= {acc[62:0], acc[63]}
             ^ {mrdt, madr}
             ^ {edat, eadr}
             ^ {62'd0, mwe, 1'b0};
      if (cyc >= n) begin
         $display("CYCLES=%0d", cyc);
         $display("DIGEST=%h", acc);
         $finish;
      end
   end

endmodule
```

## 6. The probe copy — `probe_rtl/` and `pf.txt`

`probe_rtl/` is a **flat** copy of the 26 RTL files with exactly **13 mechanical,
semantics-preserving substitutions**. It exists to separate "vita's loud gate" from
"vita's simulation correctness and throughput". Regenerate it with:

```sh
mkdir -p probe_rtl
for f in $(tail -n +2 files.txt); do cp "$f" probe_rtl/; done   # 26 RTL files, skip tb.v
sed -i '' \
  -e 's/(|WITH_CSR)/(WITH_CSR != 0)/g' \
  -e 's/RESET_STRATEGY (RESET_STRATEGY)/RESET_STRATEGY ("MINI")/g' \
  -e 's/reset_strategy (RESET_STRATEGY)/reset_strategy ("MINI")/g' \
  -e 's/RESET_STRATEGY (reset_strategy)/RESET_STRATEGY ("MINI")/g' \
  -e 's/reset_strategy (reset_strategy)/reset_strategy ("MINI")/g' \
  -e 's/memfile (memfile)/memfile ("blinky.hex")/g' \
  probe_rtl/*.v
{ echo tb.v; ls probe_rtl/*.v; } > pf.txt
```

Derive the copy from `files.txt`, **not** from `src/rtl/*.v src/servile/*.v
src/servant/*.v` — that glob pulls in 43 extra FPGA board wrappers.
This recipe was run and its output diffed: it reproduces `probe_rtl/`
**byte-identically**.

The complete edit set, verified against upstream:

| file | line | upstream | probe |
|---|---|---|---|
| `serv_ctrl.v` | 73 | `if (\|WITH_CSR)` | `if (WITH_CSR != 0)` |
| `serv_rf_if.v` | 65 | `if (\|WITH_CSR)` | `if (WITH_CSR != 0)` |
| `serv_top.v` | 553 | `if (\|WITH_CSR)` | `if (WITH_CSR != 0)` |
| `serv_rf_top.v` | 114, 155 | `(RESET_STRATEGY)` | `("MINI")` |
| `serv_top.v` | 232, 434, 555 | `(RESET_STRATEGY)` | `("MINI")` |
| `servant.v` | 92, 106 | `(reset_strategy)` | `("MINI")` |
| `servant.v` | 90 | `.memfile (memfile)` | `.memfile ("blinky.hex")` |
| `servile.v` | 156, 205 | `(reset_strategy)` | `("MINI")` |

**Proof the edits do not change the design:** iverilog gives the identical
`f3f45af36093b2b1` on both file lists (measured in every round of section 4).

`pf.txt` = `tb.v` followed by all 26 `probe_rtl/*.v` in `ls` order.

## 7. What vita does on the unmodified RTL — the loud gate

`exit 1`, `errors=10 warnings=8`, 0.02 s, dies in elaborate. Verbatim headline:

```
src/servant/servant.v:93:4: error[VITA-E3009] E-ELAB-UNSUPPORTED: the override of parameter `RESET_STRATEGY` is not a constant, so the declared default would be used instead — that is a different design, not a smaller one [in tb.dut.ram]
```

Two independent constant-domain gaps, each with a 12-line repro beside this file:

1. **A string-valued parameter used as a parameter-override argument is not a
   constant.** 7 sites — `servant.v:93` ×2, `servant.v:108`, `servile.v:158`,
   `servile.v:210`, `serv_top.v:237`, `serv_top.v:437` — every
   `.RESET_STRATEGY (reset_strategy)` / `.memfile (memfile)` passthrough down the
   `servant → servile → serv_top → serv_ctrl/serv_state` chain.
   Repro: `repro_string_param_override.v`. A literal `.STRAT("MINI")` works and an
   integer param passthrough `.WIDTH(w)` works; it is specifically a **string**
   parameter *reference* in an override position.

2. **A unary reduction operator applied to a parameter is not const-foldable in a
   `generate if` condition.** 3 sites, all `generate if (|WITH_CSR)` —
   `serv_top.v:553`, `serv_ctrl.v:72`, `serv_rf_if.v:64` — surfacing as
   `VITA-E3010 E-ELAB-UNRESOLVED-NAME: generate-if condition is not a constant`.
   Repro: `repro_reduction_in_generate_if.v`, four-way: `|WITH_CSR` fails with a
   ternary override, with a literal `1` override, and with a plain int param
   override, while `WITH_CSR != 0` passes all three. The override is irrelevant —
   the constant evaluator simply has no reduction-operator arm on this path. This
   is the more general of the two; `|PARAM` is a common idiom.

Fixing gap (2) alone, or both, moves this candidate to **vita-match**.

Two smaller gaps found while writing `tb.v` (worked around on the tb side only;
they never touched the RTL):

- `$value$plusargs` is accepted only as the direct rhs of a blocking assignment.
  `if (!$value$plusargs("N=%d", n))` is rejected with `VITA-E3009`; iverilog and
  verilator both accept the `if` form.
- Hierarchical access to an unpacked array is unsupported: both
  `dut.ram.mem[i] = ...` and `$readmemh(f, dut.ram.mem)` give `VITA-E3009`
  "hierarchical read of `dut.ram.mem` is unsupported". Upstream's own
  `bench/servant_sim.v` uses exactly `$readmemh(firmware_file, dut.ram.mem)`, so
  this will recur across SoC candidates.

## 8. Corpus role

Keep **both** lists:

- `files.txt` — a **loud-gate regression**. Unmodified upstream ISC RTL that vita
  refuses; the day it stops refusing, it must print `f3f45af36093b2b1`.
- `pf.txt` — a **throughput + differential workload**. 4318 lines of real
  bit-serial RISC-V RTL, 500 k cycles, hierarchical taps, `$readmemh`, generate,
  wishbone, byte-exact against iverilog at 0.81×.

## 9. `run.sh`

A thin wrapper over section 3 lives beside this file:

```sh
./run.sh iverilog        # oracle, unmodified list
./run.sh vita            # loud gate, unmodified list
./run.sh iverilog-probe  # oracle, probe list
./run.sh vita-probe      # the throughput/differential number
./run.sh verilator       # mismatches by design, see section 4
N=1000000 ./run.sh iverilog
```

Tool paths are overridable via `$VITA` / `$IVERILOG` / `$VVP`.

## 10. Verification performed for this file

- 4 interleaved rounds, first pair discarded, medians in section 4 — never A-then-B.
- Digest byte-identical across all 4 runs of each of the 4 configurations.
- vita/iverilog digest agreement re-measured here, and confirmed at three
  different `N` so it cannot be a fixed point.
- The section 6 regeneration recipe was executed and `diff -r`'d against the real
  `probe_rtl/`: byte-identical.
- The `tb.v` and `files.txt` embedded above were extracted back out of this file,
  built in a clean directory, and run: `DIGEST=f3f45af36093b2b1`. This file alone
  is sufficient.
- The verilator mismatch was re-measured, not inherited from the scout.
