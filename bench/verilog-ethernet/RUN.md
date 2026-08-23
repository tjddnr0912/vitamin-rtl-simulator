# Benchmark: verilog-ethernet — `eth_mac_1g` GMII loopback

Status as of 2026-08-23: **READY workload, vita REFUSES it (vita-loud).**
Two independent oracles agree byte-for-byte; vita exits 1 at elaborate with 6 errors.
Admit to the corpus the moment the two gaps in §"Why vita refuses" close.

---

## 1. Source

| | |
|---|---|
| Repo | https://github.com/alexforencich/verilog-ethernet |
| Pinned SHA | `77320a9471d19c7dd383914bc049e02d9f4f1ffb` |
| License | **MIT** (`src/COPYING`, "Copyright (c) 2014-2018 Alex Forencich") — permissive, OK to vendor |
| Clone path | `bench/verilog-ethernet/src/` (upstream tree, unmodified — `git status` clean at that SHA) |
| Testbench | `bench/verilog-ethernet/tb.v` — **written for this benchmark**, not upstream (upstream `tb/` is cocotb/Python and unusable here) |

Re-create the clone:

```sh
cd bench/verilog-ethernet
git clone https://github.com/alexforencich/verilog-ethernet src
git -C src checkout 77320a9471d19c7dd383914bc049e02d9f4f1ffb
```

## 2. File list (exact, in order — order matters to none of the three tools, but keep it stable for digest reproducibility)

```
tb.v
src/rtl/eth_mac_1g.v
src/rtl/axis_gmii_rx.v
src/rtl/axis_gmii_tx.v
src/rtl/lfsr.v
```

2155 lines of RTL+TB total (`tb.v` 248, `eth_mac_1g.v` 644, `axis_gmii_rx.v` 358, `axis_gmii_tx.v` 458, `lfsr.v` 447).
Top module: **`tb`**. Language level: Verilog-2005 RTL (`-g2012` only because upstream uses `` `resetall ``/`` `default_nettype none ``).

## 3. What the workload exercises

`eth_mac_1g` (DATA_WIDTH=8, ENABLE_PADDING=1, MIN_FRAME_LENGTH=64, PTP/PFC/PAUSE all 0) with its
GMII TX output registered straight back into its GMII RX input. The TB pushes `N` frames of
pseudo-random length 20..279 and pseudo-random payload (fixed 32-bit LFSR seed `0x12345678`; **no
`$random`**, so the stimulus is tool-independent) into `tx_axis` with a proper tvalid/tready
handshake. A clocked accumulator rotate-xors `{rx_axis_tuser, tlast, tdata}` plus the five status
strobes on **every** cycle after reset, so `DIGEST` is a cycle-accurate differential gate, not an
end-state check.

Real datapath covered: preamble insert/strip, min-frame padding, CRC32 (`lfsr.v`) insert on TX and
check on RX, IFG, and back-pressure. A cycle watchdog (`N*800 + 200000`) prints `WATCHDOG` before
the digest if the design ever wedges; `$finish` is explicit on both paths.

## 4. Commands (verbatim — run from `bench/verilog-ethernet/`)

**iverilog (primary oracle, Icarus Verilog 13.0):**

```sh
iverilog -g2012 -o x.vvp tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v
vvp x.vvp +N=1000
```

**vita (one-shot):**

```sh
../../target/release/vita tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v +N=1000
```

**verilator (second, independent oracle — Verilator 5.050):**

```sh
verilator --binary --timing -Wno-fatal -o v --top-module tb tb.v src/rtl/eth_mac_1g.v src/rtl/axis_gmii_rx.v src/rtl/axis_gmii_tx.v src/rtl/lfsr.v
./obj_dir/v +N=1000
```

## 5. Tuned workload size: `+N=1000`

Measured scaling of the iverilog `vvp` run (same `x.vvp`, plusarg only):

| plusargs | iverilog run | DIGEST |
|---|---|---|
| `+N=500` | 3.94 s | `f60f0af7898ae327` |
| **`+N=1000`** | **7.82 s** | **`ca4945d0044f74d8`** |
| `+N=2000` | 15.36 s | `885e3c0a9659c234` |

The knob is linear. **`+N=1000` is the tuned setting** — it sits mid-band of the 3–15 s target, far
enough above process startup (~40 ms) that startup is 0.5 % of the measurement.

## 6. Expected output

iverilog and verilator both print, **byte-identical**:

```
RX_FRAMES=1000 RX_BYTES=156111
DIGEST=ca4945d0044f74d8
```

(iverilog then adds `tb.v:243: $finish called at 1472980000 (1ps)`; verilator's own `$finish` line
says `1ms` — it is rounding its time report. Cosmetic only; the digest and both counters match.)

**Expected DIGEST at `+N=1000`: `ca4945d0044f74d8`**

## 7. Median timings (2026-08-23, macOS arm64, Darwin 25.6.0)

Method: four interleaved iverilog/vita pairs (iverilog, vita, iverilog, vita, …) — never A-then-B —
first pair discarded as cache warm-up; median of the remaining three. Every run wrapped in a hard
timeout.

| | run 2 | run 3 | run 4 | **median** |
|---|---|---|---|---|
| `iverilog` compile | 0.039 | 0.040 | 0.042 | **0.040 s** |
| `vvp` run (`+N=1000`) | 7.836 | 7.823 | 7.822 | **7.823 s** |
| iverilog full recipe (compile+run) | 7.875 | 7.863 | 7.864 | **7.864 s** |
| `vita` one-shot | 0.025 | 0.025 | 0.024 | **0.025 s** |
| `verilator` run (build 24.3 s one-off) | — | — | — | 0.071 s |

Spread on the `vvp` run across the three post-warm-up samples is 14 ms (0.18 %) — the measurement is
stable enough that a real speed change of a few percent would be visible.

**vita's 0.025 s is not a simulation time.** vita never simulates: it exits 1 at elaborate. Do not
quote it as a speed number.

## 8. Determinism / oracle checks performed

- **iverilog determinism: PASS.** All four runs printed `DIGEST=ca4945d0044f74d8` and
  `RX_FRAMES=1000 RX_BYTES=156111`, byte-identical.
- **vita determinism: PASS.** All four runs printed exactly the same 6 errors + 26 warnings and
  `errors=6 warnings=26 notes=0`, exit 1.
- **Two-oracle agreement: PASS (re-verified here, not taken on trust).** verilator 5.050 printed the
  same two lines as iverilog 13.0. Two independent engines producing the same cycle-accurate digest
  is what makes this workload oracle-clean.
- **vita-vs-oracle digest comparison: NOT POSSIBLE.** vita emits no `DIGEST` line at all — it
  refuses before simulating. This is correct-or-loud behaving correctly, but it means the design is
  **not yet a benchmark**, only a candidate.

## 9. Why vita refuses (verbatim, all 6 errors)

```
src/rtl/axis_gmii_rx.v:161:1: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `STYLE_INT` value is not a foldable constant expression [in tb.uut.axis_gmii_rx_inst.eth_crc_8]
src/rtl/axis_gmii_rx.v:161:1: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: generate-if condition is not a constant [in tb.uut.axis_gmii_rx_inst.eth_crc_8]
src/rtl/axis_gmii_rx.v:161:1: error[VITA-E3009] E-ELAB-UNSUPPORTED: frame function/task `lfsr_mask` body uses a system task call, which is outside the frame-call subset (only blocking assigns to its own locals, if/else/case/loops, and nested task calls are supported) [in tb.uut.axis_gmii_rx_inst.eth_crc_8]
src/rtl/axis_gmii_tx.v:171:1: error[VITA-E3009] ... (same three, tx side, tb.uut.axis_gmii_tx_inst.eth_crc_8)
```

Both roots are in **`src/rtl/lfsr.v`**, and both must close before this design runs. Isolated probes
are kept at `bench/verilog-ethernet/probe/`.

**GAP 1 — the constant domain has no string VALUES.** `lfsr.v:352` is
`parameter STYLE_INT = (STYLE == "AUTO") ? "REDUCTION" : STYLE;` and `lfsr.v:362` is
`generate if (STYLE_INT == "REDUCTION")`. `probe/p3.v` splits this four ways and the split is sharp:
`parameter A = (STYLE == "AUTO")` **folds** (string *comparison* to an integer already works), but
`parameter B = STYLE` (string propagated from another parameter) and
`parameter C = 1 ? "REDUCTION" : "LOOP"` (string selected by a ternary) do **not**. A string
*literal* initializer is fine; a folded value that *is* a string is missing. E3009 on `STYLE_INT`
then cascades into E3010 on the generate-if, which is why two errors point at one line.

**GAP 2 — a function whose body contains a system task call is outside the frame-call subset.**
`lfsr.v:204`'s `function lfsr_mask` has `$error(...); $finish;` at `lfsr.v:306` (and again at `:437`)
in a **dead** else-branch. `probe/p2.v` is a 16-line Verilog-2005 reduction of this: iverilog prints
`DIGEST=6`, vita rejects. `$display`/`$error` inside a function is explicitly legal Verilog-2005 and
the branch is unreachable, so no value semantics are at stake.

Closing GAP 1 alone would **not** unblock this design: `lfsr_mask` is called from the genvar loop in
the REDUCTION arm, so GAP 2 fires on whichever arm is selected.

**Wider payoff:** `rtl/lfsr.v` is Forencich's shared CRC/scrambler generator, vendored verbatim into
verilog-ethernet, verilog-axis, verilog-pcie, verilog-uart and corundum. The `STYLE`/`STYLE_INT`
string-parameter idiom and the `$error` guard inside the mask function are in all of them.

## 10. Noise worth fixing separately

vita emits **26 `W-ELAB-FEATURE-LIMIT` warnings for unconnected *output* ports** (`stat_*`, `ptp_*`,
deliberately left off the instantiation). An unconnected output is unambiguously legal and carries no
hazard; these 26 lines bury the 6 real errors — a `tail -13` on stderr shows only warnings. Consider
demoting to a note, or suppressing for outputs.

Everything else in the design went through cleanly: the 130-port `eth_mac_1g` instantiation, the
`TX_USER_WIDTH` ternary parameter chains, `` `default_nettype none ``/`` `resetall ``, and the
multi-file elaboration. The refusal is narrowly the two `lfsr.v` constructs.
