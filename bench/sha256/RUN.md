# bench/sha256 — reproducible recipe

Hardened 2026-08-23. Everything below was re-measured from scratch on this
machine; nothing is inherited from the scout report without verification.

## Source

| | |
|---|---|
| Repo | https://github.com/secworks/sha256 |
| Pinned SHA | `837c5cc396f001d18f2c765721c585716eb439ae` (2025-12-15) |
| License | **BSD-2-Clause** — Copyright (c) 2013, Joachim Strömbergson. Permissive, no patent/OHL rider. Full text: `src/LICENSE`. |
| Clone location | `bench/sha256/src/` (`git status --porcelain` is empty — the RTL is byte-identical to upstream, nothing was modified, simplified, or cut down) |
| Top module | `tb` (in `bench/sha256/tb.v`, written for this bench — see "Testbench") |
| LOC fed to the simulators | **1022** = 929 upstream RTL + 93 harness |

```
git clone https://github.com/secworks/sha256 src
git -C src checkout 837c5cc396f001d18f2c765721c585716eb439ae
```

## File list (exact, in order)

```
tb.v
src/src/rtl/sha256_core.v
src/src/rtl/sha256_w_mem.v
src/src/rtl/sha256_k_constants.v
```

Order matters only in that `tb.v` is first; all four must be on one command
line. Paths are relative to `bench/sha256/`.

## Commands (run from `bench/sha256/`)

Icarus Verilog 13.0:

```sh
iverilog -g2012 -o x.vvp tb.v src/src/rtl/sha256_core.v src/src/rtl/sha256_w_mem.v src/src/rtl/sha256_k_constants.v \
  && vvp x.vvp +N=2000
```

vita (release binary, never rebuilt for benching):

```sh
../../target/release/vita tb.v src/src/rtl/sha256_core.v src/src/rtl/sha256_w_mem.v src/src/rtl/sha256_k_constants.v +N=2000
```

Verilator 5.050 (optional third oracle):

```sh
verilator --binary --timing -Wno-fatal -o vsim tb.v src/src/rtl/sha256_core.v src/src/rtl/sha256_w_mem.v src/src/rtl/sha256_k_constants.v \
  && ./obj_dir/vsim +N=2000
```

`run.sh` in this directory wraps all three (`./run.sh iverilog` | `./run.sh
verilator` | `./run.sh`), with `N` overridable from the environment.

> **zsh trap.** Do not put the file list in a plain variable and write
> `vita $F`. zsh does not word-split unquoted parameter expansions, so all four
> paths arrive as one argv entry and vita fails with
> `error[VITA-E8005]: cannot read 'tb.v src/... src/... src/...'`. That reads
> exactly like a silent vita failure and is not one. Use an array, or `${=F}`,
> or `/bin/sh`.

## Expected output

`+N=2000` (the tuned workload) — both simulators, verbatim:

```
iverilog:
DIGEST=e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724
/…/bench/sha256/tb.v:90: $finish called at 536010000 (1ps)

vita:
DIGEST=e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724
simulation ended (Finish) at time 536010000
errors=0 warnings=0 notes=0
```

vita exit code 0, `errors=0 warnings=0 notes=0`. The two tools also agree on the
exact finish time, `536010000` (1 ps precision).

Other workload sizes (all three tools agree at each):

| `+N` | DIGEST |
|---|---|
| 500 | `8391db712c0429b5d50be2ffcc3491d573f000da150bccb47194986d132dd63d` |
| 2000 (default, tuned) | `e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724` |
| 4000 | `dfb4a236ae060ca2a05570faee970a77e9379dba0ee24abe4252191e957e0876` |

Three distinct digests for three `+N` values proves `$value$plusargs("N=%d", N)`
is actually honoured rather than silently ignored in favour of the built-in
default. Running with **no** plusarg reproduces the N=2000 digest, so the
`$value$plusargs` false branch is correct too.

## Timings

Method: interleaved iverilog → vita → iverilog → vita … for 4 rounds, first pair
discarded (cache warming), median of the remaining 3. Both binaries are release
builds. Sequential A-then-B was not used — it manufactures fake deltas.

```
round 1  ivl 4.165  vita 1.454     <- discarded
round 2  ivl 4.119  vita 1.434
round 3  ivl 4.177  vita 1.449
round 4  ivl 4.110  vita 1.415
```

| tool | median (s) | spread over the 3 kept runs |
|---|---|---|
| iverilog (compile + vvp) | **4.119** | 0.067 s (1.6 %) |
| vita (one-shot) | **1.434** | 0.034 s (2.4 %) |
| verilator (run only, excludes ~4.7 s C++ build) | 0.033 | — |

**vita is 2.87× faster than iverilog here.**

The iverilog figure is the whole documented recipe, compile included, so it is
directly comparable to vita's one-shot. Fixed startup is negligible on both
sides and is fully amortised at this size: `iverilog` compile-only is 0.038 s
(0.9 % of 4.119) and vita's parse+elaborate is 0.019 s (1.3 % of 1.434,
measured with `+N=1`).

## Workload tuning

`+N=2000` is the tuned value: iverilog's median of **4.12 s** sits inside the
3–15 s target band, startup is under 1 % of it, and it is cheap enough to run on
every corpus pass. Scaling is linear and was measured, not extrapolated: `+N=4000` costs 8.30 s
(compile + 8.258 s vvp) if a longer run is ever wanted, and `+N=500` costs 1.10 s
(compile + 1.061 s vvp) — below band, do not use as the default.

## Testbench

`tb.v` is written for this bench, not upstream. Upstream `src/src/tb/tb_sha256_core.v`
is a pass/fail counter over ~4 NIST vectors: it terminates in milliseconds and
prints no digest, so it is a conformance check, not a workload.

`tb.v` instantiates `sha256_core` directly and chains N block compressions:
`block = {chain, ~chain}`, `mode` alternates SHA-256/SHA-224 on `i[0]`, and after
each `ready` the digest is xor-accumulated (`acc ^= digest`) while `chain` is
rotated 192 bits and xored with the loop counter. **The feedback is
data-dependent**, so a single wrong bit in any round of any block propagates into
the final `DIGEST=` line — this is a real differential gate, not a stopwatch.
No `$random`, no `$time` in the digest. A watchdog (`#40000000; $display("WATCHDOG");
$finish;`) provides a deterministic hard stop; it never fires (the run ends at
536.01 µs).

## Determinism

Verified, not assumed: the **entire stdout** of each tool is byte-identical
across all 4 runs at `+N=2000`.

```
sha256(iverilog stdout) = da438b0bba0f324d76bdc38ac12bddd0540b4e09c74ded35fd7a0064c3e92822  (×4)
sha256(vita     stdout) = 7fcdf6e93bc16a3d9e65dddf216d3a7d9548df100bdea7ffedd6958e976076fb  (×4)
```

The two hashes differ only because the tools word their `$finish` banner
differently; the `DIGEST=` lines are identical.

## Known quirk (not a vita defect)

Verilator reports `$finish at 528us`, while iverilog and vita both report
536.01 µs. The 8.01 µs gap is exactly 2000 × 4 ns = one clock period per loop
iteration: verilator's `--timing` leaves the `while (!tb_ready) @(posedge clk)`
wait one edge earlier than the two event-driven simulators. The DIGEST is
unaffected at all three N values, so this is a scheduling nuance in verilator's
timing model, not a datapath divergence — vita sides with iverilog exactly.
Recorded because a corpus gate that compares finish *time* (rather than just the
digest) across all three tools would false-positive here.

## Verdict

**vita-match** — clean three-way agreement (iverilog ≡ vita ≡ verilator) on the
digest at every workload size tested, vita exit 0 with zero diagnostics.
