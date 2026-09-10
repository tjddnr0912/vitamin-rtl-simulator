# study/03 — The workload corpus

Ten real designs, run under `vita` and checked against a digest an external oracle
produced. This document states what the corpus is for, the contract a workload must
satisfy, the ten rows and what each exercises, how `corpus-runner` is invoked and how to
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
   testbench toward what vita accepts, and that destroys the measurement.
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

## 3. The ten workloads

Manifest: `static CORPUS: &[Workload]` in `crates/corpus-runner/src/corpus.rs`. It is a Rust
`const` table rather than a data file, because the workspace builds `--locked` for
cross-platform reproducibility and a TOML or JSON manifest would mean a parser dependency
for a file that changes a few times a year. The compiler checks it instead.

| # | Name | Origin | Shape | Licence | Pinned SHA | Directory | Plusargs | Expect | Oracle |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `sha256` | github.com/secworks/sha256 | crypto | BSD-2 | `837c5cc396f001d18f2c765721c585716eb439ae` | `sha256` | `+N=2000` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 2 | `aes` | github.com/secworks/aes | crypto | BSD-2 | `80dc4718e1dcbbdb4b0dd1bdb393d8f7b98981dc` | `aes` | `+N=200` | `Runs { exit: 1 }` | iverilog 13.0; verilator 5.050 agrees |
| 3 | `picorv32` | github.com/YosysHQ/picorv32 | cpu | ISC | `a473fc8fca393771d83b0ffcf0b14db3393339d8` | `picorv32` | `+N=400000` | `Runs { exit: 0 }` | iverilog 13.0 only |
| 4 | `darkriscv` | github.com/darklife/darkriscv | cpu | BSD-3 | `4aa437997cd35253c9111f10a449de13ccaeee78` | `darkriscv/src/sim` | `+N=600000` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 5 | `biriscv` | github.com/ultraembedded/biriscv | cpu | Apache-2.0 | `6af9c4be5a0807d368eaad5e49af52322e31d073` | `biriscv` | `+N=50000` | `Runs { exit: 0 }` | iverilog 13.0 |
| 6 | `serv` | github.com/olofk/serv | cpu | ISC | `41e8aeedfd1e9ad5f95902c5b0dfc83d1c99e5d2` | `serv` | `+N=500000` | `Runs { exit: 0 }` | iverilog 13.0 only |
| 7 | `verilog-axi` | github.com/alexforencich/verilog-axi | fabric | MIT | `516bd5dadc3365b7f9e225d2af8fe0b8d804fe53` | `verilog-axi` | `+N=5000` | `Split { … }` | iverilog 13.0 |
| 8 | `verilog-ethernet` | github.com/alexforencich/verilog-ethernet | stream | MIT | `77320a9471d19c7dd383914bc049e02d9f4f1ffb` | `verilog-ethernet` | `+N=1000` | `Runs { exit: 0 }` | iverilog 13.0; verilator 5.050 agrees |
| 9 | `keccak` | first-party, in this repository | crypto | ours | — | `keccak` | `+N=2000` | `Runs { exit: 0 }` | iverilog 13.0, verilator 5.050 and a Python reference all agree |
| 10 | `keccak-arr` | first-party, in this repository | crypto | ours | — | `keccak` | `+N=2000` | `Runs { exit: 0 }` | iverilog 13.0, verilator 5.050 and a Python reference all agree |

Shapes, from `enum Shape`, and what each exercises:

| Shape | Rows | Exercises |
|---|---|---|
| `cpu` | picorv32, darkriscv, biriscv, serv | fetch/decode/execute, branchy control, a register file, a bus |
| `crypto` | sha256, aes, keccak, keccak-arr | a wide fixed datapath, few branches, heavy bit manipulation |
| `stream` | verilog-ethernet | streaming and handshake pipelines: many small always blocks, high event churn |
| `fabric` | verilog-axi | parameterised interconnect: elaboration- and generate-heavy |

Distribution is four crypto, four cpu, one stream, one fabric.

### 3.1 Per-row mechanics

**`sha256`** — four files, the corpus's widest vita margin.

**`aes`** — seven files. It produces the correct digest and still exits 1: vita reports the
out-of-range array read in `aes_key_mem.v` as an error where IEEE 1364-2005 §5.2.1 defines
the behaviour (read x, write ignored) and both oracles stay silent. `Expect::Runs { exit: 1 }`
pins that rather than grading the workload as a crash, so the over-loud diagnostic stays
visible and closing it will show up as a row that needs its pin moved.

**`picorv32`** — the reference RISC-V workload; two files. It uses `tbd.v`, not the older
`tb.v`, because that one prints final state only and is blind to a divergence the core later
overwrites. Its oracle is Icarus Verilog only: picorv32's register file starts
uninitialised, so the design genuinely depends on 4-state semantics and Verilator's 2-state
approximation produces a different answer (`17b6f447736ac50d`).

**`darkriscv`** — core only; the full SoC (darksocv, darkuart and the rest) is refused, and
that refusal is a queue row rather than a corpus row. This is the one workload whose `dir`
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

`bench/ibex/` exists on disk and is not in the manifest. Icarus Verilog 13 cannot parse
`ibex_pkg.sv` — a syntax error on the named assignment pattern of a struct-typed
`localparam`, and an internal assertion abort in `net_scope.cc` when it is rewritten
positionally — so contract rule 2 excludes it. That is a reason to build a hand-IEEE pin,
not a reason to defer; it is recorded as a queue row, and admitting it would make the
corpus's first SystemVerilog workload.

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
| **1** | `fetch --run` where a clone or a `prepare.sh` exits non-zero or fails to spawn; `run` where any row is a failure (stderr: `corpus-runner: {n} failing`) |
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
git -C bench/{root}/src fetch --depth 1 origin {sha}
git -C bench/{root}/src checkout --detach {sha}
[sh bench/{root}/prepare.sh]
```

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
`bench/keccak/*.sv`, and outright ignores for `/bench/*/src/`, `/bench/*/obj_dir*/` and
`/bench/ibex/`. A stray binary or scratch probe cannot be committed by accident.

---

## 6. Current state

`coverage()` counts rows that are not `Expect::Refused`. No row is refused, so `list` and
`run` print `coverage: 10/10`. A clean run grades nine rows `ok` and one, `verilog-axi`,
`ruled-split`.

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
question. `--compare` therefore invokes Icarus Verilog only.

The two `keccak` rows have the strongest oracle in the corpus: three independent
implementations agree, and the agreement is anchored to a published constant rather than
being mutual.

---

## 7. Performance, and the limits of what the corpus gates

Median of three timed samples, round-robin interleaved with the first round discarded,
release binaries, no other load. Reproduce with `cargo run -p corpus-runner -- run --compare`.
Ratio is `iverilog / vita`; above 1 means vitamin is faster.

| Workload | vita | iverilog | Ratio |
|---|---:|---:|---:|
| sha256 | 1.25 s | 4.06 s | 3.25× |
| verilog-ethernet | 2.24 s | 7.72 s | 3.45× |
| aes | 2.70 s | 6.03 s | 2.23× |
| biriscv | 4.05 s | 9.20 s | 2.27× |
| keccak | 4.24 s | 9.26 s | 2.19× |
| picorv32 | 4.32 s | 7.06 s | 1.64× |
| darkriscv | 6.63 s | 7.10 s | 1.07× |
| serv | 7.42 s | 7.32 s | 0.99× |
| keccak-arr | 13.58 s | 9.14 s | 0.67× |
| verilog-axi | — | — | ruled split; retired from timing |

Geometric mean **1.74×** over the nine timed rows, **1.93×** over the seven third-party
rows, **2.15×** with `serv` excluded. This table is the only place these numbers live.

Two rows are losses and their causes are recorded. `keccak-arr` builds a 25-element array on
every subroutine call and is the frame-path row. `serv` is the x-heavy row, and its cost is
not compiled-lane coverage: it is the corpus's *most* compiled design at 98.6% of compile
requests admitted, the highest rate of the ten.

### 7.1 Every row is at least 99% simulation

From the separate `--obs-dir` probe run (one sample per row, not the timed rounds):

| Workload | elab | sim | Front end |
|---|---:|---:|---:|
| biriscv | 0.022 s | 3.817 s | 1% |
| verilog-ethernet | 0.011 s | 2.123 s | 1% |
| picorv32 | 0.015 s | 4.131 s | 0% |
| serv | 0.008 s | 6.893 s | 0% |
| aes | 0.006 s | 2.598 s | 0% |
| darkriscv | 0.003 s | 6.301 s | 0% |
| sha256 | 0.002 s | 1.193 s | 0% |
| keccak / keccak-arr | 0.001 s | 3.984 / 12.632 s | 0% |

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
Two of the ten shapes have exactly one row each (`stream`, `fabric`), so those axes rest on
a single design.

---

## 8. What CI runs

CI does **not** run `corpus-runner run`: the corpus RTL is not in the repository and CI does
not clone it. What CI runs is the manifest hygiene suite — `crates/corpus-runner/tests/manifest.rs`,
eight tests — plus 17 unit tests inside `run.rs` covering the grading table, the reverse
digest scan and the median.

| Test | Enforces |
|---|---|
| `every_upstream_workload_is_permissively_licensed` | contract rule 1 |
| `every_upstream_workload_is_pinned_to_a_full_commit_sha` | reproducibility of any quoted number |
| `workload_names_are_unique` | `--filter` cannot be ambiguous |
| `every_pinned_digest_is_one_the_scanner_can_find` | the pin matches `digest_line`'s contract |
| `every_workload_has_sources_and_an_oracle` | contract rule 2 |
| `every_pinned_refusal_names_a_reason` | a `Refused` row cannot pin an empty diagnostic |
| `coverage_is_reported_over_the_whole_corpus` | the coverage line's denominator |
| `the_corpus_covers_more_than_one_shape` | shape diversity |

Running the corpus itself is a manual gate on a development machine, the same treatment the
Icarus Verilog differential suite gets. A full `run --compare` takes about two minutes;
workload sizes are tuned to 3–15 seconds under Icarus Verilog.

---

## 9. Open items

**The corpus.**

- One row is a ruled split (`verilog-axi`) and one is a loss vita has not closed (`serv`).
  Reaching 10/10 did not retire the corpus; those two rows are why.
- `ibex` is excluded for want of an oracle (§3.1). It is the entry point to a modern
  SystemVerilog core, and a hand-IEEE pin would make it the corpus's first SystemVerilog
  workload.
- `stream` and `fabric` have one row each. Until that changes, those two shapes rest on a
  single design apiece.
- The full `darkriscv` SoC is refused and is a queue row rather than a corpus row.
- `aes` exits 1 on an IEEE-defined out-of-range read (§3.1); closing that over-loud
  diagnostic will require moving its pin.

**The tool.**

- `--compare` invokes Icarus Verilog only; Verilator is a half oracle (§6.2) and is not
  automated.
- The corpus is not wired into CI (§8).
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
