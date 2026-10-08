# study/03 — The workload corpus

Fifteen real designs, run under `vita` and checked against a digest an external oracle
produced. This document states what the corpus is for, the contract a workload must
satisfy, the fifteen rows and what each exercises, how `corpus-runner` is invoked and how to
read its table, and what the corpus can and cannot gate.

The tool is [`crates/corpus-runner`](../../crates/corpus-runner); the directory it measures
is [`bench/`](../../bench/README.md). Companion studies: the performance axis in
[study/01](01-interpreted-vs-compiled.md), terminology and native-backend coverage in
[study/02](02-v1-native-coverage.md).

---

## 1. What it is for

A measurement made on one design is a property of that design. Before the corpus, this
project's performance and coverage judgements rested on two designs — `bench/picorv32`, and
`bench/keccak`, which was written here to be measured and therefore leans toward the
bottlenecks already known. A ceiling computed from `keccak_f_arr` alone is a fact about
`keccak_f_arr`.

The corpus exists so that the next claim of the form "this is worth N weeks" is priced
against RTL nobody here wrote, and so that a defect nobody here suspected has somewhere to
show up. Both halves pay: the largest single finding it has produced — that every
continuous assign whose right-hand side reaches a user call was re-evaluated on every settle
pass — came from a third-party workload rather than from any internal probe.

The RTL is never redistributed. `bench/*/src/` is not committed; the repository carries a
pinned commit SHA per workload and `corpus-runner fetch` clones it, so any number in this
document can be reproduced against exactly the source that produced it.

---

## 2. The contract

Canonical text: `crates/corpus-runner/src/lib.rs`. A workload is admitted only if all five
hold.

1. **Permissive licence** — MIT, BSD-2, BSD-3, ISC or Apache-2.0. A test enforces it.
2. **An oracle ran it first.** No oracle, no admission, however interesting the design.
   Icarus Verilog 13.0 is the reference; Verilator 5.050 is a second opinion on 2-state
   arithmetic only. The oracle runs *before* vita: running vita first leads to trimming the
   testbench toward what vita accepts, and that destroys the measurement. One exception: a
   SystemVerilog design Icarus Verilog cannot parse may take Verilator as its oracle, but only
   if its digest is x-invariant — identical under `--x-assign unique --x-initial unique` with
   `+verilator+rand+reset+0`, `+1`, and `+2` over at least 64 seeds. Five seeds are not
   enough: they once passed an ibex testbench that failed to print its pin on 29 of 62. A
   2-state oracle cannot answer for a design that reads x (picorv32 and serv do, §6.2), and
   randomising x does not show that a 4-state simulator's x-pessimism never reaches the
   digest, so a 4-state cross-check is recorded beside it where one can be built (for ibex,
   sv2v converts the design and iverilog runs it). A manifest test holds the row's `oracle`
   field to naming the check. `ibex` and `opentitan-prims` are the rows admitted this way. A second exception
   (owner ruling): where Verilator is not x-invariant on a design, that 4-state
   cross-check may stand alone as its oracle, on three conditions — vita's digest is
   byte-identical to the 4-state oracle's; the failed x-invariance check is recorded with
   the row (the seeds and the distinct digests or cycle counts they printed); and rule 5's
   asymmetric mutations move the 4-state oracle's digest as well as vita's. VeeR EL2
   (ROADMAP §5.2) is the row it was ruled for; the runner's contract text and its manifest
   test take the exception with that row.
3. **One digest line, accumulated over the whole run** — not final state, which is blind to
   a divergence the design later overwrites. The pinned digest is the *oracle's* answer, so
   `corpus-runner run` is a differential gate even on a machine with no other simulator
   installed.
4. **Deterministic and self-terminating** — an explicit `$finish`, fixed seeds, a watchdog.
   A workload must not be able to stall the harness.
5. **The digest must move when the design changes.** Mutate one line of the upstream RTL
   and re-run: if the digest survives, the workload gates nothing, and it looks exactly like
   one that does. Every row is checked this way. A *symmetric* mutation can be dead
   honestly — in the `verilog-ethernet` loopback, TX and RX share one `lfsr` instance, so a
   CRC-polynomial change cancels at both ends and only an RX-side datapath mutation moves
   the digest. A mutation that fails to move a digest and a workload that cannot measure are
   different conclusions, and asymmetry is what separates them.

Rule 3 also shapes what the corpus can gate at all: a digest accumulated over a whole run
selects for designs that simulate for seconds and elaborate for milliseconds. §7 is the
consequence.

---

## 3. The fifteen workloads

Manifest: `static CORPUS: &[Workload]` in `crates/corpus-runner/src/corpus.rs`. It is a Rust
`const` table rather than a data file, because the workspace builds `--locked` for
cross-platform reproducibility and a TOML or JSON manifest would mean a parser dependency
for a file that changes a few times a year. The compiler checks it instead.

| # | Name | Origin | Shape | Licence | Pinned SHA | Directory | Plusargs | Expect | Oracle |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `sha256` | github.com/secworks/sha256 | crypto | BSD-2 | `837c5cc396f001d18f2c765721c585716eb439ae` | `sha256` | `+N=2000` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 2 | `aes` | github.com/secworks/aes | crypto | BSD-2 | `80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc` | `aes` | `+N=200` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 3 | `picorv32` | github.com/YosysHQ/picorv32 | cpu | ISC | `a473fc8fca393771d83b0ffcf0b14db3393339d8` | `picorv32` | `+N=400000` | `Runs { exit: 0 }` | iverilog 13.0 only |
| 4 | `darkriscv` | github.com/darklife/darkriscv | cpu | BSD-3 | `4aa437997cd35253c9111f10a449de13ccaeee78` | `darkriscv/src/sim` | `+N=600000` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 5 | `biriscv` | github.com/ultraembedded/biriscv | cpu | Apache-2.0 | `6af9c4be5a0807d368eaad5e49af52322e31d073` | `biriscv` | `+N=50000` | `Runs { exit: 0 }` | iverilog 13.0 |
| 6 | `serv` | github.com/olofk/serv | cpu | ISC | `41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2` | `serv` | `+N=500000` | `Runs { exit: 0 }` | iverilog 13.0 only |
| 7 | `verilog-axi` | github.com/alexforencich/verilog-axi | fabric | MIT | `516bd5dadc3365b7f9e225d2af8fe0b8d804fe53` | `verilog-axi` | `+N=5000` | `Split { … }` | iverilog 13.0 |
| 8 | `verilog-ethernet` | github.com/alexforencich/verilog-ethernet | stream | MIT | `77320a9471d19c7dd383914bc049e02d9f4f1ffb` | `verilog-ethernet` | `+N=1000` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 9 | `keccak` | first-party, in this repository | crypto | ours | — | `keccak` | `+N=2000` | `Runs { exit: 0 }` | iverilog 13.0, verilator 5.050 and a Python reference all agree |
| 10 | `keccak-arr` | first-party, in this repository | crypto | ours | — | `keccak` | `+N=2000` | `Runs { exit: 0 }` | iverilog 13.0, verilator 5.050 and a Python reference all agree |
| 11 | `ibex` | github.com/lowRISC/ibex | cpu | Apache-2.0 | `8b8ee086aef72e0833b7f0493d9d33f1e4d3c8e2` | `ibex` | `+N=20000` | `Runs { exit: 0 }` | verilator 5.052, x-invariant (iverilog 13 cannot parse it) |
| 12 | `verilog-axis` | github.com/alexforencich/verilog-axis | fabric | MIT | `48ff7a7e2ef782cf778d47910cf85835c64b1bce` | `verilog-axis` | `+N=3000` | `Runs { exit: 0 }` | iverilog 13.0 |
| 13 | `verilog-i2c` | github.com/alexforencich/verilog-i2c | stream | MIT | `a65be4045e898a52e791c6ee71f8f79a7cd2e129` | `verilog-i2c` | `+N=900000` | `Runs { exit: 0 }` | iverilog 13.0 |
| 14 | `verilog-uart` | github.com/alexforencich/verilog-uart | stream | MIT | `1b867e53af738e4a8bc7c839ca2f1c07f40382dc` | `verilog-uart` | `+N=20000` | `Runs { exit: 0 }` | iverilog 13.0 |
| 15 | `opentitan-prims` | github.com/lowRISC/opentitan (sparse: two directories) | crypto | Apache-2.0 | `a3490b428e30cde95ad7a4d9072517bfbd03fc02` | `opentitan-prims` | `+N=32` | `Runs { exit: 0 }` | verilator 5.052, x-invariant (iverilog 13 cannot parse it) |

Shapes, from `enum Shape`, and what each exercises:

| Shape | Rows | Exercises |
|---|---|---|
| `cpu` | picorv32, darkriscv, biriscv, serv, ibex | fetch/decode/execute, branchy control, a register file, a bus |
| `crypto` | sha256, aes, keccak, keccak-arr, opentitan-prims | a wide fixed datapath, few branches, heavy bit manipulation |
| `stream` | verilog-ethernet, verilog-i2c, verilog-uart | streaming and handshake pipelines: many small always blocks, high event churn |
| `fabric` | verilog-axi, verilog-axis | parameterised interconnect: elaboration- and generate-heavy |

Distribution is five crypto, five cpu, three stream, two fabric. `ibex` and
`opentitan-prims` are the SystemVerilog designs. Rows 12–15 came out of the row-9
new-design census (§4.5.603), which ran each of them under its oracle first.

### 3.1 Per-row mechanics

**`sha256`** — four files, the corpus's widest vita margin.

**`aes`** — seven files. `aes_key_mem.v:182` reads one word past `key_mem`, which IEEE
1364-2005 §5.2.1 defines (read x, write ignored) and both oracles pass silently. vita reports
it as the warning `VITA-E4002` and exits 0 (`Expect::Runs { exit: 0 }`). Until §4.5.576 it was
an error: the row printed the correct digest and exited 1, pinned `Runs { exit: 1 }`.

**`picorv32`** — the reference RISC-V workload; two files. It uses `tbd.v`, not the older
`tb.v`, because that one prints final state only and is blind to a divergence the core later
overwrites. Its oracle is Icarus Verilog only: picorv32's register file starts
uninitialised, so the design genuinely depends on 4-state semantics and Verilator's 2-state
approximation produces a different answer (`17b6f447736ac50d`).

**`darkriscv`** — core only; the full SoC (darksocv, darkuart and the rest) runs and agrees
with Icarus Verilog, but its UART's `$fgetc` path depends on host file state, so it is not a
corpus row (`bench/darkriscv/RUN.md`). This is the one workload whose `dir`
differs from its `root`: it runs from `bench/darkriscv/src/sim` because upstream's
`darkram.v` opens `../src/darksocv.mem` relative to the working directory. Flags are
`--top tb2 -DSIMULATION=1 -D__WAITSTATE__=7 -I ../rtl` for vita and `-s tb2` with the same
defines and includes for Icarus Verilog; files `../../tb2.v ../rtl/darkriscv.v
../rtl/darkram.v`; data `../src/darksocv.mem`.

**`biriscv`** — dual-issue, 23 source files, the largest design that runs at 8.8k lines.
`--top tb_top -I src/src/core -D TRACE=0`. Its firmware `prog.hex` is upstream content
extracted from `test.elf`, so it is not committed; `bench/biriscv/prepare.sh` regenerates
it.

**`serv`** — bit-serial, roughly 35 clocks per instruction, which makes it a
scheduler-throughput workload. 27 source files, data `src/sw/blinky.hex` (byte-identical to
upstream's own copy, so also not committed). `--top tb` / `-s tb` is not optional: the file
list has three uninstantiated roots (`tb`, `serv_rf_top`, `servile_rf_mem_if`), and without
the flag the oracle elaborates three designs where vita elaborates one, so the comparison
stops being between the same thing. Its oracle is Icarus Verilog only: SERV reads an
uninitialised register file and drives x deliberately, so Verilator's 2-state result
(`e7e8b5e6c1276563`) is a different design's answer.

**`verilog-axi`** — a 2×2 crossbar, elaboration- and generate-heavy, ten files. It
elaborates and runs, and its digest is not the oracle's. §6.1 covers the ruling.

**`verilog-ethernet`** — GMII loopback, five files, the only streaming shape.

**`keccak` / `keccak-arr`** — first-party RTL committed in this repository, sharing
`bench/keccak` and differing in exactly one thing: `keccak_f_arr.sv`'s `rho` builds a
25-element array on every call where `keccak_f.sv` does not. Same expected digest. That one
difference is worth 2.0×, which makes `keccak-arr` the corpus worst case and the only design
from which the frame-arena estimates were computed. The oracle is three-way and anchored
rather than merely mutual: the all-zero-state first lane is the published Keccak value
`f1258f7940e1dde7`. Recipe: [`bench/keccak/RUN.md`](../../bench/keccak/RUN.md).

`bench/keccak/keccak_f_flat.sv`, generated by `gen_flat.py`, is deliberately not a corpus
row: it exists to measure the call regime (study/01 §2.4).

**`ibex`** — lowRISC's RV32 core through `ibex_top` (`RV32M=RV32MFast`, flop register file,
no icache, no security hardening), 62 upstream files and 30,272 lines, the corpus's only
SystemVerilog design. A 15-instruction RV32IM loop runs N times and stores its accumulator
every iteration. Flags are `--top tb -DSYNTHESIS -DDV_FCOV_DISABLE` plus three `-I`
directories on every tool: `SYNTHESIS` removes the DPI-C exports and `DV_FCOV_DISABLE`
empties the coverage macros. Icarus Verilog 13 cannot parse `ibex_pkg.sv` (`:350`, a keyed
assignment pattern on a packed-struct `localparam`), so its oracle is Verilator under the
x-invariance condition of rule 2: 66 runs with randomised x initialisation print the same
digest, and sv2v converted and run under Icarus Verilog, a 4-state simulator, prints it
too. Eight asymmetric mutations across the ALU, the multiplier, the load-store unit, the
register file, the prefetch buffer and the core's clock gate move it. The testbench asserts reset with a falling edge: the core runs on a gated clock
whose enable is itself reset state. vita refuses
it at elaboration: 1 error, ROADMAP §3.a ⑤ⓚ, behind one prerequisite at the head of the §5.2
queue. §4.5.571's builds (the parser) and §4.5.572's (elaborate) that resolved it ran the whole
design to Verilator's digest at both sizes, §4.5.572's also eight of the mutations, and both were
reverted: the parser could not certify which declaration the construct's target reaches, and the
elaborate design met the parser's wildcard-import binding and a block-local check that skips
`force`. (The parse error in
front of it — two `prim_lfsr.sv` functions returning a generate-local multi-dimensional packed
typedef — closed in §4.5.564, the twelve undeclared labels of a generate block's `typedef enum`
in §4.5.565, the eighteen errors of nine whole-array continuous assigns in §4.5.566, five
`'{default: v}` on a packed target in §4.5.567, two string-literal generate-if conditions in
§4.5.568, two packed-array parameters written as `'{…}` in §4.5.569, and the copy of a
`pmp_cfg_t` array in §4.5.570.) Recipe, x runs and mutation:
[`bench/ibex/RUN.md`](../../bench/ibex/RUN.md).

**`verilog-axis`** — a 4×4 `axis_switch` whose outputs feed four frame-mode `axis_fifo`
instances, six files: routing, round-robin arbitration, register slices and frame drop. 189
of its 86,322 cycles carry an x on a FIFO's `tlast` output (uninitialised RAM, before the
first beat) in Icarus Verilog and in vita alike; the digest counts them (`XC=189`) and both
tools agree on every line. Three one-line mutations (FIFO data, switch routing, arbiter
mask) move the digest and a control on an unconnected status output does not. Recipe:
[`bench/verilog-axis/RUN.md`](../../bench/verilog-axis/RUN.md).

**`verilog-i2c`** — `i2c_master` and `i2c_slave` on one open-drain bus, three files. The
digest folds every change of twelve bus and status bits and every byte either side receives,
stamped with its cycle, so it is the census's 187,187-line trace in one line (`+TRACE` prints
the trace, byte-identical to the census's in both tools). Recipe:
[`bench/verilog-i2c/RUN.md`](../../bench/verilog-i2c/RUN.md).

**`verilog-uart`** — the `uart` with `txd` looped into `rxd`, four files. A timing-only
mutation (the receiver samples its start bit one cycle later; every byte still arrives intact)
moves the digest through the per-cycle status fold alone. Recipe:
[`bench/verilog-uart/RUN.md`](../../bench/verilog-uart/RUN.md).

**`opentitan-prims`** — eleven unmodified OpenTitan files driving seven primitives (two
instances each of `prim_present`, `prim_subst_perm` and `prim_prince`, the 39/32 inverted
SECDED encoder and decoder, `prim_crc32`, `prim_gf_mult` and `prim_count`) under a 32-vector
stream; the fetch checks out two directories of the OpenTitan tree (`sparse`, §5). Icarus
Verilog 13 stops at `prim_cipher_pkg.sv:23` (`sorry: packed array parameters are not
supported yet`, twelve times), so the oracle is Verilator under rule 2: 66 runs with
randomised x initialisation and 34 more of an `--x-initial-edge` build print one digest, and
sv2v + Icarus Verilog (4-state) prints it too. The census ran 200 vectors (vita 54.1 s against
Verilator's 0.38 s); the row runs 32 so it fits the corpus step, and Verilator reproduces the
census's 200-vector digest at `+N=200`. Recipe:
[`bench/opentitan-prims/RUN.md`](../../bench/opentitan-prims/RUN.md).

### 3.2 Pinned digests

| Workload | Pinned digest |
|---|---|
| `sha256` | `DIGEST=e75e29e81cff3c66de9e0f419baa516ea08e6414fa1f9f62a757538288351724` |
| `aes` | `DIGEST=cfaa46dd896b2275ade662d344f5e251` |
| `picorv32` | `DIGEST=68d30f61bf9bf1d4` |
| `darkriscv` | `DIGEST=59370cf8b1d0503d` |
| `biriscv` | `DIGEST=22481d1cacf87584` |
| `serv` | `DIGEST=f3f45af36093b2b1` |
| `verilog-axi` | `DIGEST=3b9321d5ea42f302` (oracle) and `DIGEST=fd90a1407928ebc8` (vita) |
| `verilog-ethernet` | `DIGEST=ca4945d0044f74d8` |
| `keccak`, `keccak-arr` | `perms=2000 lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde` |
| `ibex` | `DIGEST=13b2ddfcd551ba2f` (verilator; sv2v + iverilog agrees; vita prints it since §4.5.574) |
| `verilog-axis` | `DIGEST=d24b621c2e3346ba` |
| `verilog-i2c` | `DIGEST=e8bac662acedfaec` |
| `verilog-uart` | `DIGEST=7ba8527cf36dd903` |
| `opentitan-prims` | `DIGEST=8387a22d0531655d` (verilator; sv2v + iverilog agrees) |

The manifest's `note` fields deliberately carry no timings. A number written in two places
rots in one of them; the timings live in §7 of this document, and
`corpus-runner run --compare` reproduces them.

---

## 4. Running it

`corpus-runner` has no external dependencies (std only), carries `#![forbid(unsafe_code)]`
and is `publish = false`.

```
corpus-runner — the vitamin workload corpus

    list                       what the corpus contains, and what is on this machine
    fetch [--run]              show (or perform) the clones the corpus needs
    run [--filter S] [--reps N] [--compare]
                               run each present workload and check its pinned digest
                               --reps N = N TIMED samples (N+1 rounds; the first is
                               discarded as cache warm-up). Default 3.
                               --compare also times iverilog on the same workloads.

exit: 0 = every present workload matched  ·  1 = a mismatch or crash
      2 = nothing present (run `fetch` first)  ·  3 = usage
```

Invoke as `cargo run -p corpus-runner -- <cmd>`. With no subcommand, `list` is the default;
`-h`, `--help` and `help` print the usage text and exit 0. `run` requires a release binary at
`target/release/vita`, so build first:

```bash
cargo build --release -p cli --locked
cargo run -p corpus-runner -- fetch --run
cargo run -p corpus-runner -- run --compare
```

### 4.1 Exit codes

| Code | Every path that returns it |
|---:|---|
| **0** | `list`; `fetch` (with or without `--run`) where no command failed; `run` where at least one workload is present and no row is a failure |
| **1** | `fetch --run` where a clone or a `prepare.sh` exits non-zero or fails to spawn, or a stale clone is left in place (local changes, an unreadable status) or cannot be removed; `run` where any row is a failure (stderr: `corpus-runner: {n} failing`) |
| **2** | `run` where `--filter` matched no workload; `run` where every job was `Absent` (`no workload is present on this machine — run 'corpus-runner fetch --run'`) |
| **3** | no repository root found (no `bench/` above the crate); an unknown subcommand; `--filter` with no value; `--reps` with an unparseable value; no `vita` binary at `target/release/vita` or `target/debug/vita` |

### 4.2 Reading the `run` table

The table is fixed-width. Parse it by column offset, not by whitespace — the detail column
contains spaces.

```
{workload:<18} {tool:<10} {grade:<11} {median:>9}  {detail}
```

`median` renders as `{s:.3}s`, or `-` when nothing was timed. Tool labels are `vita`,
`iverilog` and `verilator`; `Tool::Verilator` exists in the enum but no verilator job is ever
built, because Verilator is a half oracle (§6.2). The phase split is printed *below* the
table rather than as a column, deliberately: a new column moves every consumer's parse.

**The eight grades.** `is_failure()` is exactly `Regression | Drifted | OracleDrifted` —
three strings, and nothing else in the output means failure.

| Variant | Printed | Failure? | Meaning |
|---|---|---|---|
| `Ok` | `ok` | no | the pinned digest matched |
| `KnownGap` | `known-gap` | no | a pinned refusal produced its pinned diagnostic — the ladder is working |
| `Regression(why)` | `REGRESSION` | **yes** | see the grading table below |
| `Promoted` | `PROMOTED` | no | a refused or split row now matches the oracle. Uppercase, and not a failure |
| `RuledSplit` | `ruled-split` | no | a split row reproduced vita's own pinned answer. Neither a pass nor a failure |
| `Drifted { got }` | `DRIFTED` | **yes** | a refused row produced a *different* refusal, so the pin stops describing the design |
| `Absent` | `absent` | no | the sources are not on this machine |
| `OracleDrifted { got }` | `ORACLE-DRIFT` | **yes** | the oracle stopped reproducing the pin |

**The non-determinism marker.** `is_nondeterministic()` is `digests.len() > 1`. When true,
the detail column is overwritten regardless of grade:

```
*** NON-DETERMINISTIC *** {digest1}  |  {digest2}[  |  …]
```

The check runs *before* the grade match, deliberately: two distinct digests from one tool is
a bigger fact than whichever one the last round happened to produce, and gating the marker
behind `Grade::Ok` makes it unreachable, since a differing digest retires the job as a
mismatch. A flapping tool would then be reported as a consistently wrong one.

**Otherwise the detail column is:**

| Grade or outcome | Detail |
|---|---|
| `Regression(why)` | `why`, verbatim |
| `Drifted { got }` | `expected a different refusal; got {got}` |
| `Promoted` | `now runs — move its manifest row to Expect::Runs` |
| `RuledSplit` | `ruled split — {why}` |
| `KnownGap` with `Refused { diag }` | `diag` |
| `OracleDrifted { got }` | `the ORACLE no longer reproduces the pin: {got}` (verbatim tool output) |
| `Absent` | `fetch first` |
| anything else | empty |

**After the table**, `run` prints the phase split (one line per matched vita row:
`{name:<18} elab {elab_s:.3}s  sim {sim_s:.3}s  ({pct:.0}% front end)`); with `--compare`,
one comparison line per workload
(`{name:<18} vita {v:.3}s  iverilog {i:.3}s  = {ratio:.2}x {faster|SLOWER}`, where
`ratio = iverilog / vita` and the verdict word is `faster` at 1.0 or above); one line per
promoted row; and always
`coverage: {runs}/{total} of the corpus runs under vita`.

`list` prints `{workload:<18} {shape:<7} {origin:<9} {licence:<12} {vita:<9} note`, the same
coverage line, and one indented line per row that is not plain `Runs`. `origin` renders as
`in-repo` or `upstream`; the `vita` column renders `runs`, `refused` or `split`.

### 4.3 The grading table

For any tool other than vita the only question is whether the oracle still reproduces the
pin: `Absent` grades `Absent`, `Match` grades `Ok`, and every other outcome — a mismatch, a
refusal, a non-zero exit, a timeout — grades `ORACLE-DRIFT`.

For vita:

| Manifest `Expect` | Outcome | Grade |
|---|---|---|
| any | `Absent` | `absent` |
| `Runs` | `Match` | `ok` |
| `Runs` | `Mismatch { got }` | `REGRESSION` — *digest changed: {got}* |
| `Runs` | `Refused { diag }` | `REGRESSION` — *newly refused: {diag}* |
| `Runs` | `Crashed { code, tail }` | `REGRESSION` — *exit {code}: {tail}* |
| `Runs` | `Timeout` | `REGRESSION` — *timed out* |
| `Refused` | `Match` | `PROMOTED` |
| `Refused` | `Refused { got }` containing the pinned fragment | `known-gap` |
| `Refused` | `Refused { got }` not containing it | `DRIFTED` |
| `Refused` | `Mismatch { got }` | `REGRESSION` — *was loud, now silently wrong: {got}* |
| `Refused` | `Crashed` | `REGRESSION` — *was loud, now crashes* |
| `Refused` | `Timeout` | `REGRESSION` — *was loud, now hangs* |
| `Split` | `Match` (the oracle digest) | `PROMOTED` — the split closed |
| `Split { vita }` | `Mismatch { got }` where `got == vita` | `ruled-split` |
| `Split { vita }` | `Mismatch { got }` where `got != vita` | `REGRESSION` — *digest changed* |
| `Split` | `Refused` / `Crashed` / `Timeout` | `REGRESSION` |

The `Refused → Mismatch` cell is pinned by name in a unit test,
`loud_becoming_silently_wrong_is_a_regression`, because loud-to-silently-wrong is the one
move the accuracy ladder forbids. A refused design that starts answering, and answers
wrongly, must not grade as a promotion.

The exit code lives inside `Expect::Runs { exit }` rather than beside it as a free field, so
a refused row has nowhere to write one. A refused workload is expected to exit **0** if it
ever starts running — that is the promotion, and it has to be observable. A correct digest
with the wrong exit code grades `Crashed` with the tail
*"digest correct but exit {c}, expected {want}"*, not a mismatch.

### 4.4 Measurement discipline built into the harness

`measure(jobs, reps, budget)` is called once by `main` with **all** jobs and a 600-second
budget, so round-robin interleaving is the default shape rather than an option. The full
protocol these implement is study/01 §5.

- The loop is `for round in 0..=reps.max(1)`, so there are `reps + 1` rounds, and a sample is
  recorded only when `round > 0`. The first round is discarded as cache warm-up. `--reps`
  defaults to 3, giving 4 rounds and 3 timed samples. `--reps N` means N *samples*.
- `median()` sorts and returns the middle element, or the average of the middle pair for an
  even count, and `None` for an empty set.
- A job that did not produce `Match` is retired and not re-run: repeating it cannot change
  the verdict and would delay the jobs still being timed. A `ruled-split` row is therefore
  never timed and shows `-` in the median column.
- Presence is tested on the **sources** (`missing_source()`), not on the directory, because
  `fetch` creates `bench/<root>/src` and that makes `bench/<root>` exist. Testing the
  directory turns a `cannot read 'tb.v'` into a refusal and grades it *newly refused*.
- `run_bounded()` enforces the wall-clock budget by hand, since macOS ships no `timeout(1)`,
  and drains both pipes on their own threads for the child's whole life. The common
  `try_wait` plus `wait_with_output` shape deadlocks the moment a workload outruns the pipe
  buffer — the child blocks in `write`, so it never exits, and the parent never reads because
  it has not exited — which then reports an honest, immediate refusal as a hang.
  `verilog-axi` emits 19 238 bytes of diagnostics and a macOS pipe starts at 16 KiB.
- `digest_line()` scans **stdout only**, in reverse, for a line containing `DIGEST=` or
  ` acc=`; the contract is that the digest is the last such line. stdout and stderr are kept
  apart so a stderr line containing `DIGEST=` can never outrank the real one.
- `refusal()` prefers the pinned diagnostic wherever it appears in stderr and falls back to
  the first `error[` or `error:` line only when the pin is absent. Grading on emission order
  would make a harmless reordering read as a drift: `verilog-ethernet` emits 24 warnings
  before its pinned error, and `verilog-axi` emits 54 errors of which the pinned one is
  merely first.
- `prepare_iverilog()` compiles the `.vvp` **before** the timed rounds
  (`iverilog -g2012 {args} -o {vvp} {files}` in the workload's directory), so what is timed
  on each side is simulation rather than vita paying for elaboration while Icarus Verilog
  pays for nothing. A failure prints `corpus-runner: no iverilog comparison for {name}: {e}`
  and is not a corpus failure. `vvp_path()` writes the `.vvp` outside `dir` so `darkriscv`,
  which runs from inside its clone, does not dirty it.
- `probe_phases()` runs one **extra** vita invocation with `--obs-dir` and reads `elab_s` and
  `sim_s` out of `run.json`. One sample, and a separate run: folding `--obs-dir` into the
  timed command would add its file writes to every vita wall time and make the headline
  numbers incomparable with the ones pinned in this file and the README.
- `vita_binary()` prefers `target/release/vita`, falls back to `target/debug/vita` and warns
  on that path — `corpus-runner: WARNING measuring a debug binary; timings are not
  comparable` — and with no binary at all errors `run 'cargo build --release' first` and
  exits 3.

Interleaving is a property of the call site, not of the type. Passing all jobs at once makes
round-robin the default, but calling `measure(&jobs[0..1])` and then `measure(&jobs[1..2])`
is block-sequential measurement again.

---

## 5. Fetching, and what is committed

`plan_fetch(root)` produces one `FetchStep` per upstream workload. `fetch` prints the plan
and executes it only with `--run`:

```
git clone --filter=blob:none --no-checkout {repo} bench/{root}/src
[git -C bench/{root}/src sparse-checkout set --cone {dirs}]
git -C bench/{root}/src fetch --depth 1 origin {sha}
git -C bench/{root}/src checkout --detach {sha}
[sh bench/{root}/prepare.sh]
```

The `sparse-checkout` line is emitted only for a row whose `Origin::Upstream` names `sparse`
directories. `opentitan-prims` is the one: it compiles eleven files of the OpenTitan tree,
and the two directories it checks out (`hw/ip/prim/rtl`, `hw/ip/prim_generic/rtl`, plus the
tree's top-level files) leave its clone at 60 MB, 54 MB of which is the blob-less history.
Because the clone is `blob:none` and the cone is set before the checkout, no blob outside the
cone is downloaded. A manifest test (§8) holds every listed file and `-I` directory of a
sparse row inside its cone.

A clone counts as present only when its HEAD resolves to the pinned SHA and every file the
row lists inside it exists (`clone_state()`). HEAD is read from the files git keeps
(`resolve_head()`): the SHA itself for the detached checkout `fetch --run` leaves, otherwise
the target of `ref: <r>` from the loose ref file or from `packed-refs`, so a clone made by
hand on a branch at the pin is present too; a `ref:` that resolves nowhere is a clone whose
checkout never ran. A directory that is anything else — a clone whose fetch, cone or
checkout failed half-way, another commit, a cone missing a listed file — is reported with
the reason, and `fetch --run` removes it and clones again. Testing the directory alone would
print "already present" on every later fetch while `run` grades the row `absent`.

Removal is guarded (`clear_stale()`). The path is rebuilt from the row's root, which a
manifest test holds to one plain path component unique among the upstream rows, and the
state is measured again right before anything is removed. A stale clone that holds anything
besides `.git` is removed only when `git status --porcelain --ignored=no`, run on the
clone's own repository, succeeds and prints nothing: an edited or untracked file, or a
status git cannot produce, leaves the clone in place, `fetch --run` reports it and goes on
with the other rows, and exits 1. Files the clone's own `.gitignore` covers (build products)
do not count as local changes. A clone holding only `.git` is removed without asking git.

A workload that is already present still re-runs its `prepare.sh` under `--run`: those
scripts regenerate deliberately uncommitted artifacts and are idempotent.
`bench/biriscv/prepare.sh` is the only one, regenerating `prog.hex` from upstream's
`test.elf`.

Two kinds of file live under `bench/`, treated oppositely. Testbenches, file lists,
`RUN.md`, `run.sh`, `prepare.sh` and the first-party `bench/keccak/*.sv` are committed: they
are this project's work product, and a pinned SHA reconstructs the upstream RTL but not the
harness — and the harness is what produced the digest. The upstream clone, the firmware
images extracted from it, and every build product are not. `.gitignore` implements this as
an **allow-list**: everything under `bench/` is ignored, with explicit re-adds for `tb*.v`,
`tb*.sv`, `files.txt`, `RUN.md`, `README.md`, `run.sh`, `prepare.sh`, `*.py` and
`bench/keccak/*.sv`, and outright ignores for `/bench/*/src/` and `/bench/*/obj_dir*/`. A stray binary or
scratch probe cannot be committed by accident.

---

## 6. Current state

`coverage()` counts rows that are not `Expect::Refused`. No row is refused since §4.5.574, so
`list` and `run` print `coverage: 15/15`. A clean run grades fourteen rows `ok` and `verilog-axi`
`ruled-split`. A slice that makes a row refused again fails the run (the runner grades a moved
refusal `DRIFTED`, a runs-row that stops running a regression).
Since the real-design direction the corpus is also the loop's pre-push gate:
`corpus-runner run` runs before every push, and CI runs it on every push to `main` and every
pull request, on all three platforms (§8).

### 6.1 The ruled split

`verilog-axi` elaborates and runs. Its digest is not the oracle's, and the whole of the
difference is `XC=29` against `XC=0`: 29 cycles out of 123 166 in which the crossbar's
registered `valid` outputs are `x` in Icarus Verilog and definite in vita. Everything else
matches — the same cycle count, the same handshake, the same per-master data digests.

Two states were not enough for this row. `Expect::Refused` grades it *"was loud, now
silently wrong"* — the right shape for a refused row that starts answering wrongly, and
permanently red here. `Expect::Runs` with vita's own digest would be self-certifying, which
is what the corpus exists to prevent.

The axis is measured and ruled. The residual is time-zero continuous-assign event ordering
(ROADMAP §2-N), where **the oracle answers two ways to the same question**: with identical
operands and identical values, `wire w = a | b;` fires the `always @*` that reads it at time
zero and `wire w = a & b;` does not. Verilator settles everything and has no vote. There is
no oracle to match, only a ruling.

`Expect::Split { vita, why }` pins **both** digests and names the ruling. It is not a pass:
the row reads `ruled-split` on every run with its reason on the line. vita's own answer
moving is still a `REGRESSION`, and the day the two agree the row grades `PROMOTED`, which
is the event it waits for. The state exists for a divergence the oracle cannot arbitrate and
for nothing else — a digest that merely fails to match is a finding, not a split.

### 6.2 Which oracle applies to which row

Verilator is a half oracle and the manifest records per row which tool arbitrates it. It
agrees on sha256, aes, verilog-ethernet and darkriscv. It disagrees on picorv32
(`17b6f447736ac50d`) and serv (`e7e8b5e6c1276563`), and in both cases the design reads an
uninitialised register file on purpose, so a 2-state approximation is answering a different
question. `--compare` therefore invokes Icarus Verilog only; on `ibex` and `opentitan-prims` it
prints the Icarus Verilog error and is not a failure. Those two are the rows Verilator
arbitrates, under rule 2's x-invariance condition, with sv2v + Icarus Verilog as their 4-state
cross-check; the commands are in `bench/ibex/RUN.md` and `bench/opentitan-prims/RUN.md`
(`./run.sh verilator-x`, `./run.sh sv2v`). Neither is automated, so the runner cannot report
`ORACLE-DRIFT` for these rows. Verilator was not run on `verilog-axis`, `verilog-i2c` or
`verilog-uart`: Icarus Verilog arbitrates them, and `verilog-axis` reads x on 189 cycles,
which a 2-state tool would flatten.

The two `keccak` rows have the strongest oracle in the corpus: three independent
implementations agree, and the agreement is anchored to a published constant rather than
being mutual.

---

## 7. Performance, and the limits of what the corpus gates

Median of three timed samples, round-robin interleaved with the first round discarded,
release binaries, all fifteen rows in one run (2026-10-07, a 10-core Apple-silicon machine;
other processes held the load average at 3–4 throughout). Reproduce with
`cargo run -p corpus-runner -- run --compare`. Ratio is `iverilog / vita`; above 1 means
vitamin is faster.

| Workload | vita | iverilog | Ratio |
|---|---:|---:|---:|
| sha256 | 1.22 s | 4.01 s | 3.28× |
| verilog-ethernet | 2.36 s | 7.50 s | 3.18× |
| aes | 2.50 s | 5.72 s | 2.29× |
| verilog-uart | 3.89 s | 6.25 s | 1.61× |
| darkriscv | 4.01 s | 6.86 s | 1.71× |
| keccak | 4.01 s | 8.86 s | 2.21× |
| biriscv | 4.04 s | 8.97 s | 2.22× |
| picorv32 | 4.57 s | 6.73 s | 1.47× |
| verilog-axis | 5.02 s | 7.69 s | 1.53× |
| verilog-i2c | 5.51 s | 8.69 s | 1.58× |
| serv | 7.38 s | 7.10 s | 0.96× |
| keccak-arr | 12.33 s | 8.81 s | 0.71× |
| verilog-axi | — | — | ruled split; retired from timing |

Geometric mean **1.74×** over the twelve timed rows, **1.86×** over the ten third-party
rows, **2.00×** with `serv` excluded. `ibex` (29.95 s) and `opentitan-prims` (9.07 s) have no
Icarus Verilog time, because Icarus Verilog cannot parse them. This table is the only place
these numbers live.

Two rows are losses and their causes are recorded. `keccak-arr` builds a 25-element array on
every subroutine call and is the frame-path row. `serv` is the x-heavy row, and its cost is
not compiled-lane coverage: it is the corpus's *most* compiled design at 98.6% of compile
requests admitted, the highest rate of the ten.

### 7.1 Every row is at least 99% simulation

From the separate `--obs-dir` probe run of the same session (one sample per row, not the
timed rounds):

| Workload | elab | sim | Front end |
|---|---:|---:|---:|
| ibex | 0.230 s | 29.923 s | 1% |
| opentitan-prims | 0.051 s | 9.048 s | 1% |
| picorv32 | 0.031 s | 4.556 s | 1% |
| biriscv | 0.027 s | 4.047 s | 1% |
| verilog-ethernet | 0.013 s | 2.342 s | 1% |
| verilog-axis | 0.024 s | 4.982 s | 0% |
| serv | 0.012 s | 7.531 s | 0% |
| aes | 0.007 s | 2.509 s | 0% |
| verilog-i2c | 0.004 s | 5.571 s | 0% |
| sha256 | 0.003 s | 1.218 s | 0% |
| darkriscv | 0.003 s | 4.017 s | 0% |
| verilog-uart | 0.001 s | 3.833 s | 0% |
| keccak / keccak-arr | 0.001 s | 4.027 / 12.528 s | 0% |

That table is a property of the corpus, and it follows directly from contract rule 3: a
digest accumulated over a whole run selects for designs that simulate for seconds and
elaborate for milliseconds.

**What the corpus can gate.** Simulation correctness and simulation speed. Every pinned
digest is an accumulated trace, so a divergence anywhere in the run is caught even if the
design later overwrites it, and the timings are dominated by exactly the phase the digests
check.

**What it cannot gate.** Front-end cost. An elaboration cost that triples on a
declaration-heavy module moves every median in the table by less than the noise floor; the
same cost measures +36% on `biriscv` and +193% on a module of 20 000 plain `wire [31:0]`
declarations when elaboration is timed on its own. Printing the phase split makes the number
readable; it does not make the gate see it. A threshold needs a front-end-bound row — many
declarations, a short simulation — with a pinned digest and an oracle, and no workload here
is that. Tracked as `ELAB-PHASE-BLIND` in [ROADMAP](../ROADMAP.md) §5.b.

**Neither can it gate what it does not contain.** A shape absent from the corpus is a shape
the corpus is silent about: an admission rule measured at 1.00× across these ten designs is
worth 2–4× on mixed-sign expression trees, which are simply not in these designs' hot loops.
Until rows 12–14 `stream` and `fabric` had one row each; every shape now rests on at least
two designs.

---

## 8. What CI runs

CI runs the corpus. Two jobs in `.github/workflows/ci.yml` do it on every push to `main` and
every pull request, in parallel with the test jobs: `corpus`, a matrix over ubuntu-latest and
macos-latest, and `corpus-rhel`, in the same `redhat/ubi9` container as `build-rhel`. Three
platforms, so the byte-identity claim is held on real designs and not only on the test suite:
each row's pin is one string on every OS. Both jobs are blocking — no `continue-on-error`.
Each runs:

```bash
cargo build --release -p cli --locked                # `run` requires target/release/vita
cargo run -p corpus-runner --locked -- fetch --run
git status --porcelain --untracked-files=all         # must print nothing
cargo run -p corpus-runner --locked -- run --reps 1
```

- **The fetch** performs §5's plan: the pinned clones, then `bench/biriscv/prepare.sh`. The
  clones under `bench/*/src` (about 180 MB) are cached with `actions/cache`, keyed on the
  runner OS and a hash of `corpus.rs` (the pins), `fetch.rs` (the clone commands) and
  `bench/*/prepare.sh`, so a moved SHA misses the cache; `prepare.sh` re-runs on a cache hit.
  The UBI container installs `git` before checkout — without it `actions/checkout` downloads a
  tarball with no `.git` — and `python3` for `prepare.sh`.
- **The clean-tree step** makes §5 mechanical: third-party RTL is never redistributed, so after
  the fetch `git status --porcelain` must be empty — every path the clones and the prepare
  scripts write is gitignored. A non-empty status fails the job and prints the paths.
- **`run`** grades every row against its pin (§4.3). Any non-zero exit fails the job: 1 for a
  failing row, 2 for nothing present, 3 for misuse or a missing binary. `ok`, `ruled-split`,
  `known-gap` and `PROMOTED` pass. There is no `--compare`: CI has no Icarus Verilog, and the
  pinned digest already is the oracle's answer (contract rule 3). Without `--compare` no oracle
  job is built, so `ORACLE-DRIFT` cannot arise in CI.

The test jobs run the manifest hygiene suite — `crates/corpus-runner/tests/manifest.rs`, fourteen
tests — plus 19 unit tests inside `run.rs` covering the grading table, the reverse digest scan
and the median, and nine inside `fetch.rs`: three hold the clone plan (the whole-tree recipe,
a sparse row's cone set before the checkout, each row's cone carried into its plan), one the
plain-root rule, and five the presence check and the guarded removal: a checkout at the pin
is present, also on a branch
whose loose or packed ref holds the pin; a wrong `HEAD`, a branch at another commit, a branch
that resolves nowhere, a missing listed file or a missing `.git` is stale; a present clone is
never removed, an unfinished one (only `.git`) is, one whose status git cannot read is not,
and on a real repository an untracked file blocks the removal that its absence allows.

| Test | Enforces |
|---|---|
| `every_upstream_workload_is_permissively_licensed` | contract rule 1 |
| `every_upstream_workload_is_pinned_to_a_full_commit_sha` | reproducibility of any quoted number |
| `workload_names_are_unique` | `--filter` cannot be ambiguous |
| `every_pinned_digest_is_one_the_scanner_can_find` | the pin matches `digest_line`'s contract |
| `every_workload_has_sources_and_an_oracle` | contract rule 2 |
| `every_pinned_refusal_names_a_reason` | a `Refused` row cannot pin an empty diagnostic |
| `no_workload_pins_a_known_wrong_answer` | a workload whose digest misses is a regression; `Split`, which names a ruling, is the only way to pin one |
| `a_verilator_oracle_row_records_its_x_invariance_check` | contract rule 2's Verilator exception |
| `coverage_is_reported_over_the_whole_corpus` | the coverage line's denominator |
| `the_corpus_covers_more_than_one_shape` | shape diversity |
| `every_uncommitted_manifest_path_is_gitignored` | §5: a manifest path is committed or gitignored, never committable. Asks git; skips without a work tree |
| `every_sparse_checkout_covers_what_its_row_reads` | §5: a sparse row's listed files, data files and `-I` directories in its clone lie inside its cone |
| `every_upstream_root_is_one_plain_component_and_unique` | §5: `fetch --run` may remove `bench/<root>/src`, so a root is one plain path component and no two upstream rows share one |
| `a_sparse_directory_must_be_a_plain_relative_path` | §5: a sparse directory has no `..`, no leading or trailing `/` and no whitespace (`fetch --run` splits each plan line on whitespace) |

`run --compare`, which times Icarus Verilog beside vita, stays a development-machine step, the
same treatment the Icarus Verilog differential suite gets. Workload sizes are tuned to 3–15
seconds under Icarus Verilog. CI passes `--reps 1`: two rounds whose digests are both
compared, so a non-deterministic row still fails, and the timed samples it drops are not
gated in CI. At the default `--reps 3` — four rounds plus one phase probe per row — the step
took 391 s on a local Apple-silicon machine from a fresh fetch; `ibex` is the longest row, at
a 30.4 s median. The four rows admitted from the row-9 census add about 72 s to CI's
`run --reps 1` on such a machine, measured as each row's whole filtered invocation:
`verilog-axis` 15.8 s, `verilog-i2c` 16.9 s, `verilog-uart` 11.6 s, `opentitan-prims` 27.9 s.

---

## 9. Open items

**The corpus.**

- One row is a ruled split (`verilog-axi`) and one is a loss vita has not closed (`serv`).
- `ibex` runs (§4.5.574) and prints the Verilator digest at both sizes (`+N=2000`
  `32e0e78741376133`, `+N=20000` `13b2ddfcd551ba2f`, 43 warnings). Its page fell from 2 parse
  errors to 0 over §4.5.564–570, §4.5.573 (the parser's wildcard-import binding) and §4.5.574 (a
  keyed `'{…}` as a `?:` arm typed by its packed-struct target); §4.5.571's and §4.5.572's
  reverted builds had printed the same pins, and §4.5.572's Verilator's table for eight of
  RUN.md's mutations.
- New designs, licence and oracle first. The row-9 census admitted four rows that run today
  (`verilog-axis`, `verilog-i2c`, `verilog-uart`, `opentitan-prims`; §4.5.603). The rest are
  ROADMAP §5.2 rows (Solderpad designs — cv32e40p, cva6, the pulp-platform libraries — stay
  outside rule 1, owner ruling):
  - VeeR EH1 (Apache-2.0, row 31): Verilator, x-invariant over 66 runs; after the rows that
    open its page.
  - OpenTitan pattgen, uart, gpio, i2c and rv_timer (row 47): after the T and B stage rows.
  - OpenTitan aon_timer and edn (row 53): after the V stage rows.
  - `verilog-pcie` (MIT, row 63): once a first-party testbench folds a digest.
  - VeeR EL2 (Apache-2.0, row 64): under rule 2's 4-state exception; after its rows.
- The full `darkriscv` SoC runs and agrees with Icarus Verilog (`bench/darkriscv/RUN.md`), also
  with upstream's `__RMW_CYCLE__` on since §4.5.575 accepted its null `$display` argument; it is
  not a corpus row because its UART's `$fgetc` path depends on host file state.
- `aes` exited 1 on an IEEE-defined out-of-range read until §4.5.576 made `VITA-E4002` a
  warning; every row now pins exit 0.

**The tool.**

- `--compare` invokes Icarus Verilog only; Verilator is a half oracle (§6.2) and is not
  automated.
- CI runs the corpus without `--compare` (§8): it gates vita against the pins on three
  platforms but never re-checks that Icarus Verilog still reproduces them, so `ORACLE-DRIFT`
  is a development-machine signal only.
- There is no front-end-bound row, so `ELAB-PHASE-BLIND` stands (§7.1).

---

## 10. Related documents

- [study/01 — the performance axis](01-interpreted-vs-compiled.md) — the class of simulator,
  the standing verdicts on acceleration, and the A/B protocol §4.4 implements.
- [study/02 — terminology and native-backend coverage](02-v1-native-coverage.md) — census,
  coverage and mutation, and the gate a refusal comes from.
- [`bench/README.md`](../../bench/README.md) — what is committed under `bench/` and why.
- [`bench/keccak/RUN.md`](../../bench/keccak/RUN.md) — the first-party rows' recipe, oracle
  and cross-tool table.
- [ROADMAP](../ROADMAP.md) — the open silent-wrong and loud-gap queues the corpus feeds.
- [ENGINEERING_RULES](../ENGINEERING_RULES.md) — the accuracy ladder the grading table
  encodes.
