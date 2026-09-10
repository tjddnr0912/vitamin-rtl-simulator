# study/01 — The performance axis

What class of simulator vitamin is, what that class determines, where it measures against
the reference tools, where its time goes, the standing verdict on every acceleration that
has been evaluated, and the A/B protocol any future performance claim has to follow.
Every figure below carries the method that produced it.

Companion studies: terminology and native-backend coverage in
[study/02](02-v1-native-coverage.md); the workload corpus and its harness in
[study/03](03-workload-corpus.md).

---

## 1. What class of simulator this is

### 1.1 Two orthogonal axes

"Compiled simulators are fast" conflates two independent choices.

| Axis | Option A | Option B |
|---|---|---|
| **What executes the design** | *interpreted* — a data structure is walked on every activation | *compiled* — the design is lowered once to bytecode or machine code and then called |
| **When execution order is decided** | *dynamic / event-driven* — at run time, from which nets actually changed | *static / levelized* — at compile time, by topological sort of the dependency graph |

The first axis costs re-deciding "this node is an Add, evaluate left, evaluate right" on
every visit. The second decides whether a combinational chain of depth *D* costs *D* delta
cycles or one pass.

### 1.2 Where the tools sit

| | Interpreted | Compiled |
|---|---|---|
| **Event-driven, 4-state** | Verilog-XL, Icarus Verilog, **vitamin** | VCS, Xcelium, Questa |
| **Cycle-based, usually 2-state** | (rare) | Verilator |

VCS and Xcelium are compiled but not cycle-based: they keep event ordering, 4-state values
and timing, and compile only the execution. Verilator changes both axes at once and gives
up sign-off suitability for the speed. vitamin is event-driven and 4-state, which places it
in the same class as Icarus Verilog, and against a compiled 2-state simulator or a
commercial simulator it is one to two orders of magnitude slower. That distance is
structural, not a tuning defect.

### 1.3 What the class determines

**4-state values cost two planes.** A value is stored as two bit planes, `val` and `unk`,
one machine word each per 64 bits, so one instruction still processes 64 bits at a time.
Every operation, every mask, every width conversion and every net read therefore does two
sets of work. Section 3 quantifies this.

**Sparse activity is an algorithmic advantage.** A large design toggles 1–5% of its nets
per cycle, and an event-driven kernel evaluates only what a changed net wakes. A
levelized kernel evaluates the whole cone.

**A combinational chain of depth D propagates across D deltas.** A process that wakes in an
intermediate delta and reads the chain's output sees a partially propagated value. IEEE
1800 §4 leaves intra-region process order implementation-defined; vitamin pins its order to
Icarus Verilog, because the differential against that tool is what gives correct-or-loud
its teeth. Any acceleration that reorders or coalesces process execution changes observable
values, which is why fusion and levelization are ruled out of the default mode (§4).

**Sign-off suitability follows from 4-state.** Dropping the `unk` plane makes an
uninitialised signal read as 0, so a reset defect passes in simulation and diverges on
silicon. That is the trade Verilator advertises. vitamin's stated goal is correct-or-loud
accuracy at the level of Icarus Verilog, Verilator, Xcelium and VCS, so a global 2-state
mode is excluded by the goal independently of what it would be worth.

---

## 2. Measured position

### 2.1 Against three tools on one algorithm

Keccak-f[1600], macOS arm64, release builds, interleaved samples with the first round
discarded. All four tools produce
`lane0=54aa20c46ef0e0f6 lane1=b19e9f995e1f41d3 acc=767c5ab6776c4bde`, and the first lane of
the all-zero state is the published Keccak value `f1258f7940e1dde7`, so the agreement is
anchored to an external reference rather than mutual. Recipe and raw table:
[`bench/keccak/RUN.md`](../../bench/keccak/RUN.md).

| Simulator | Design | Wall | Per permutation | Relative to Verilator |
|---|---|---:|---:|---:|
| Verilator 5.050, `--binary --timing`, N=200000 | `keccak_f.sv` | 1.39 s | **7.0 µs** | 1× |
| vitamin | `keccak_f_flat.sv` (calls expanded) | 0.59 s | 295 µs | 42× slower |
| vitamin | `keccak_f.sv` (function/task calls) | 4.07 s | 2 035 µs | 291× slower |
| Icarus Verilog 13 | `keccak_f.sv` | 8.94 s | 4 470 µs | 639× slower |

### 2.2 Against Icarus Verilog on the workload corpus

Median of three timed samples per row, round-robin interleaved with the first round
discarded, release binaries, no other load. Reproduce with
`cargo run -p corpus-runner -- run --compare`. Ratio is `iverilog / vita`; above 1 means
vitamin is faster.

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
| verilog-axi | — | — | ruled split, not timed |

Geometric mean **1.74×** over the nine timed rows, **1.93×** over the seven third-party
rows, **2.15×** with `serv` excluded. Two rows are losses: `keccak-arr` builds a
25-element array on every subroutine call and is the corpus worst case for the frame path;
`serv` is bit-serial and reads an uninitialised register file, so it is the x-heavy row.

### 2.3 The three executors on one design

picorv32 with its testbench, release build, interleaved, best of five. All three executors
and Icarus Verilog end at simulated time `399995000`, i.e. the same workload.

| Executor | Time | vs `native` | What it does |
|---|---:|---:|---|
| `--backend interp` | 1.319 s | 2.57× slower | walks the IR tree on every activation |
| `--backend vm` | 0.838 s | 1.63× slower | compiles each body to bytecode once, runs an op loop |
| `--backend native` (default) | **0.513 s** | 1.00× | net values in a flat arena; uniform-width expressions on a specialised evaluator |
| Icarus Verilog 13 | 0.585 s | 1.14× slower | — |

Reproduce: `cargo build --release -p cli --locked`, then from `bench/picorv32`,
`vita --backend <b> tb.v picorv32.v`.

The bytecode VM measured against the interpreter alone (release, best of five,
`crates/sim-engine/tests/perf_baseline.rs`): expression-bound ~2.2×, structure-bound ~2.8×,
wide 100-bit ~1.7×, clock- or scheduler-bound ~1.0×, because evaluation is not the
bottleneck there.

The interpreter is a test instrument and is permanently excluded from performance work.
Making the reference faster is how a reference stops being readable, and a second
specialised spelling of a semantic rule is this repository's defect class. Its numbers
above are for scale, not as a target.

### 2.4 The call regime

The same algorithm in two spellings — `bench/keccak/keccak_f.sv` with three looping
functions, and `keccak_f_flat.sv` with the calls expanded by `gen_flat.py` — produces a
byte-identical digest and differs by **6.9×** (4.07 s against 0.59 s). This is the
standing measurement of what writing a round behind `function` costs.

The pair is a gate, not just a probe. `crates/cli/tests/perf_call_regime.rs` asserts, in
tests that are not `#[ignore]`d, that the two spellings print the same digest and that the
digest is not all-X, that the caller body is admitted to the compiled backend for both, and
that `flat.frame_bodies == 0` while `called.frame_bodies == 2`. A drifted pair still
produces two timings and still divides them, so the equality assertion is what makes the
ratio a measurement.

The gap closes more slowly than each improvement suggests, because it narrows at both ends:
the flat design is exempt from the call, not from a whole-net read or a scheduler
allocation.

Two layers make up the cost. The **caller** layer is closed: a process body holding a user
call is admitted to the compiled backend, and only the one expression holding the call
declines to the generic evaluator. The **callee** layer is open: a function body runs on
`SimState::run_frame_call`, the generic `Value` tree-walk, on every backend.

### 2.5 What is not measured

VCS and Xcelium have never been run by this project. Their speed advantage over an
open-source 4-state simulator is a given and is not hedged, but no number in this
repository is a measurement of it. Any numeric target against them needs a licensed
single-core run of the corpus.

Verilator is measured, because it is free and it is a genuinely compiled backend, which
makes it a usable ceiling gauge even though its 2-state cycle semantics disqualify it as a
sign-off oracle.

---

## 3. Where the time goes

### 3.1 Simulation, not elaboration

`corpus-runner run` prints a phase split under its grade table, from a separate
`--obs-dir` probe run — one sample per row, not the timed rounds, because the flag's file
writes would otherwise be charged to every timed median. `elab_s` and `sim_s` are internal
timers written into `run.json`, so the probe measures the same phases the timed rounds ran.

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

Every row is at least 99% simulation. Simulation speed is therefore the whole of the
performance axis for these designs, and a front-end cost is arithmetically invisible in
the corpus medians: an elaboration cost that triples on a declaration-heavy module moves
every median in the table by less than the noise floor, while the same cost measures +36%
on `biriscv` and +193% on a module of 20 000 plain `wire [31:0]` declarations when
elaboration is timed on its own. Printing the split makes the number readable; it does not
make the corpus gate it. A threshold needs a front-end-bound row — many declarations, a
short simulation — with a pinned digest and an oracle, which no workload currently is.
Tracked as `ELAB-PHASE-BLIND` in [ROADMAP](../ROADMAP.md) §5.b.

### 3.2 The runtime value representation

`sim_engine::Value` is 72 bytes: two 32-byte `Words` (each an inline `[u64; 2]` or a heap
`Vec`, plus a discriminant), a width, a signedness and the `is_real` / `is_str` flags.
**16 of those bytes are the 4-state data.** The other 56 are metadata — exactly what an
interpreter needs at run time and what a compiled simulator bakes into generated code as
literals.

An execution-weighted census of every value returned by `eval_ctx`, over the eight
then-running corpus workloads (instrumentation measured and reverted, not committed):

```text
                    definite   <=64 bits   BOTH     heap
  picorv32           100.00%    100.00%    100.00%   0.00%
  keccak             100.00%     99.95%     99.95%   0.05%
  keccak-arr         100.00%     99.72%     99.72%   0.28%
  biriscv             99.91%     99.94%     99.86%   0.00%
  aes                 99.99%     97.49%     97.49%   0.00%
  darkriscv           98.49%     97.89%     96.38%   0.00%
  serv                89.66%    100.00%     89.66%   0.00%
  sha256             100.00%     83.93%     83.93%  16.07%
```

83.9% to 100% of evaluated values are simultaneously definite and at most 64 bits —
geometric mean 95.7%, median 99.7%. That is the shape the compiled `wprog` lane already
carries: `W = (val, unk)` is 16 bytes and its 2-state lane is a bare `u64` at 8. The
representation the workloads need is already in the tree; what limits it is how much of a
design reaches it.

Three readings follow, and each rules out a candidate lever:

- **The heap is not the cost.** `Value`'s `Vec` spill fires on 0.00% of evaluations in six
  of eight designs; `sha256` is the exception at 16%, from its 512-bit blocks. The 72 bytes
  move by value, inline.
- **A lazy unknown plane is not the prize.** At ≤64 bits both planes are inline, so the
  `unk` plane costs no allocation — 16 of the 72 bytes and some ALU work. The 56 metadata
  bytes dominate, and a compiled program does not carry them.
- **A global 2-state mode with an x trip-wire is the wrong granularity.** `serv` is the
  floor at 89.66% definite and its x is real, so such a mode would trip immediately after
  reset and stay tripped. The per-operation lane is the right granularity and already
  exists.

One column of the wider census is a biased subsample and must not be read as a design-wide
x/z rate: `genpath_reads` (aes 11.65% definite, sha256 39.54%) counts only `read_net`, the
general `Value`-returning path, which is reached exactly when the fast `read_scalar_words`
path declines. Those nets have already fallen off the fast lane, so they over-represent
x/z by construction. The `eval_ctx` column has no such bias — it sees every value.

### 3.3 What two planes cost per operation

A 4-state bitwise AND over one word is thirteen machine operations, against one for
2-state:

```rust
pub(crate) fn and_w(av: u64, au: u64, bv: u64, bu: u64) -> (u64, u64) {
    let known0 = (!au & !av) | (!bu & !bv);
    let known1 = (!au & av) & (!bu & bv);
    (known1, !known0 & !known1)
}
```

`!au` reads as "known", `!av` as "the value bit is 0", so `!au & !av` is "definitely 0".
`or_w`, `xor_w`, `xnor_w` and `not_w` have the same shape.

Arithmetic is cheaper, not dearer, because a partially known sum is impossible — any `x` in
either operand poisons the whole result, so the implementation is one branch plus a 2-state
add. Operations are not uniformly expensive.

Three further sites double because the planes do:

| Site | Why it is two sets of work |
|---|---|
| `mask_top` | a signal width is rarely a multiple of 64, so the unused high bits of the last word must be cleared after every operation — once for `val`, once for `unk` |
| `resize` | widening sign-extends the top bit of `val` *and* the top bit of `unk`, so an `x` sign bit widens to `x`; narrowing truncates both |
| whole-net read | two loads, two masks, and one `Value` construction |

An instruction count is not a time measurement, and this is where that matters most:
removing the work the `unk` plane causes measures about **7%**, far below the 30% bar set
for taking the trade, because the second plane is usually all zeros, stays in cache, and
`& 0` retires almost free on a superscalar core.

### 3.4 The profile is flat

Self-time profile of a real design (picorv32), before the specialised lane existed:

```
eval          26.7%
resize        16.6%
netread       13.1%
mask_top      13.0%
eval_binary   12.5%
```

By Amdahl's law, making a fraction `f` of a run infinitely fast bounds the whole speedup at
`1 / (1 − f)`:

| Item | `f` | Ceiling if it became free |
|---|---:|---:|
| `eval` | 26.7% | 1.36× |
| `resize` | 16.6% | 1.20× |
| `netread` | 13.1% | 1.15× |
| `mask_top` | 13.0% | 1.15× |
| `eval_binary` | 12.5% | 1.14× |

No single item is worth more than about 1.2×, and only part of each is removable — a
one-word fast path in `resize` measures about 1%. That is what "the cost is evenly spread"
means, and it is an observation about where cost sits, not a verdict that nothing can be
done: the same profile says 4-state arithmetic is roughly a quarter of the run and the
other three quarters are elsewhere.

Three ways to misread such a profile, each of which has produced a wrong number here:

1. **Symbol attribution absorbs inlined code.** `resize 16.6%` means "machine code
   attributed to the name `resize`", including everything inlined into it. What can be
   removed is duplicated *work*, not a function.
2. **Instrumentation can cost more than its subject.** Per-body region timers reported
   `bodies = 3762 ms` on a run whose uninstrumented wall clock was 1234 ms — 4.8 million
   timestamps outweighed the subject. A timer on `write_lvalue` reported 121 ms of which
   about 64 ms was the timer pair itself, at four million calls. Use ablation instead:
   make the engine do the work twice and take the wall-clock difference, which adds no
   instrumentation.
3. **A ceiling is an upper bound, not a prediction.** See §4, where a 4.24× ceiling
   realised 0.2%.

### 3.5 Compiled-lane admission

Inside the default backend, an expression compiles to a `wprog` program when its shape and
width are in the specialised lane; otherwise it declines to the generic evaluator.
An execution-weighted census of `compile` requests (instrumentation measured and reverted,
not committed) gives the per-design admission rate. The compile cache runs `compile` once
per `(eid, w, signed)`, so these counts are execution weight rather than distinct
expressions.

| Design | Admitted | Declined | Decline rate |
|---|---:|---:|---:|
| keccak | 1 712 105 | 1 546 018 | 47.5% |
| darkriscv | 10 527 654 | 7 025 920 | 40.0% |
| aes | 592 296 | 387 885 | 39.6% |
| picorv32 | 7 889 493 | 1 022 255 | 11.5% |
| biriscv | 5 015 105 | 196 393 | 3.8% |
| sha256 | 3 312 068 | 50 003 | 1.5% |
| serv | 72 500 547 | 1 043 305 | 1.4% |

A decline count is meaningless without the admitted count beside it: `serv`'s 500 015
requests on a single expression are enormous next to `aes`'s whole decline budget and
negligible against `serv`'s own 72.5 million admissions. `serv` is the least promising
target for lane coverage and is also one of the two designs vitamin loses on, which
locates its cost somewhere other than lane coverage.

The declines that remain are documented and deliberate, or are filed axes:

- A `Select` with a runtime offset (`x[i +: 4]`) is not in the lane, and a bit-serial core
  does it constantly.
- A `Ternary` evaluates both branches in the compiled lane, which has no control flow, so a
  branch holding an out-of-range array read would report an access the generic path never
  performs. The lane declines rather than report it.
- `Call` at width 64 is the frame axis (§2.4).
- A root context width above 64 bits is the wide lane; `aes` is 387k requests of it.

### 3.6 What a compiled simulator does differently

For one nonblocking assignment `a <= b`, vitamin performs: construction of a 72-byte
`Value`; resolution of an `Lvalue` through a chunk array and an `Offsets` table; a write
funnel that asks, at run time, whether the destination is real, frame-local, a handle,
2-state, an array, and how wide — eleven branches and six side tables; a push of a ~112-byte
`NbaUpdate`; and a run-time lookup of the wake set through `net_to_edge`, the dirty list
and waiter vectors.

Code generated by a compiled simulator performs two stores. Width, signedness, kind,
array-ness and the wake set were all answered when the code was generated, and the
generated code does not contain the questions. A net is a variable, not a table entry, and
can live in a register.

The relevant difference is not the instruction count but the presence of the questions.
vitamin's branches are the price of run-time generality, and the limit reached by adding a
code generator on top of the existing representation is that generality, not the executor.
A real tier-3 backend is a second engine, and it requires four things together — static
net allocation, width-specialised operations, erasure of the schedule lookup into direct
calls or static bits, and a specialised NBA record — none of which pays alone. The
direction is surveyed in
[preview/21 — tier-3 native backend](../preview/21-tier3-native-backend.md).

---

## 4. Accelerations evaluated, and the standing verdict on each

Each row states what was measured, the verdict that stands today, and the condition that
reopens it.

| Acceleration | Verdict | Reopens when |
|---|---|---|
| Bytecode VM reachable from the CLI | shipped; `--backend vm` in an oracle build | — |
| Flat net storage + specialised evaluator (`native`) | shipped; the default executor | — |
| Widening the compiled lane's admission | partly shipped; see below | a design whose hot loop is mixed-sign |
| Widening the suspend-free allow-list | rejected — ceiling 4.3% of run time | stimulus bodies become compute-heavy |
| Levelization of the Active batch | rejected — 1.00× | a design shows inter-process combinational depth ≥ 6 |
| Process fusion in the default mode | rejected — value divergence | only as a declared cycle mode with a hazard detector |
| JIT / machine-code generation (cranelift) | rejected — 14–47% slower on tier-3 | leaf loads and 2-state arithmetic can be inlined into generated code with no second spelling of the semantics |
| Flat mirror for leaf reads | rejected — 0.0% | — |
| Global 2-state (dropping the `unk` plane) | rejected — ~7%, and it conflicts with the accuracy goal | never, as a default |
| Frame arena for callee bodies | open; per-design ceilings measured (§4, last block) | it is the open half of the call axis (§2.4) |

**Compiled-lane admission.** Two admissions are shipped and one is measured and declined.
A leaf `Signal` at equal width is admitted regardless of the sign gate, because no exit of
that arm reads `signed`: corpus effect picorv32 −3.4%, darkriscv −1.6%, biriscv −1.6%,
serv −1.2%, sha256 −0.9%, keccak −0.4%, aes and keccak-arr flat, every pinned digest
unchanged. Admitting a node narrower than its context is shipped and is classified by the
LRM's sizing rule rather than by width — a self-determined node (a leaf, a select, a
concat, every one-bit result) compiles at its own width and converts, a context-determined
operator computes at the context width, truncation still declines; corpus effect darkriscv
−6.2%, serv −2.7%, picorv32 −1.8%, the three call-bound rows flat, and darkriscv moves
from parity to 1.08× ahead. The sizing classification is an `_`-free match over the
operator enums because "narrower than the context" is two different rules: folding
`v[8:11] + 4'd1` at four bits yields 0 where 16 is correct.

Removing the sign half of the admission gate outright is built, measured sound and
declined. It is sound — the module's battery grows to 8 260 admitted trees and 48 660
widening programs, all value-identical to the generic evaluator, with about 330 000
adversarial cells byte-identical and all ten corpus workloads byte-identical — and it fires
hard where it applies, 13 of 14 hot shape families at 2.1×–4.7×, a 24-assign mixed-sign
design at 1.26 s against 0.27 s. On the corpus it is **1.00×**, because mixed-sign
expression trees are not in these designs' hot loops. The queue line records that, rather
than recording it as worthless.

**Suspend-free allow-list widening.** Bodies the compiled path refuses are those holding
`#delay`, `@` or `fork`, which are mostly stimulus. Sweeping work per activation on bodies
already admitted answers the value question without building the resume-PC state machine
that widening needs: 1 statement per activation is 0.99×, 2 is 0.99×, 8 is 0.91×, 64 is
0.75×. Per-activation fixed cost is not amortised below about eight statements, and
stimulus bodies are one to three. On a real design and testbench the fallback activations
are 18.1% of activations but only **4.3% of time** (257 ns each against 709 ns for a
compiled body), which is the ceiling for this axis. The measured ceiling of 2.84–4.24×
realised 0.2–0.3%: a ceiling bounds the reachable range and does not predict it.

**Levelization.** Static combinational ranks, an Active batch drained in rank order and a
settle between ranks measure 1.00× across depths 1 through 24. A depth sweep at fixed cycle
count shows why: a pure combinational chain is linear in depth (3.3 ms at depth 1, 31.5 ms
at depth 24 in a single module), and the wake chain carries one process per delta, so there
is no batch to sort. The quadratic term appeared only when the chain ran through continuous
assigns (7.8 ms at depth 1, 814.4 ms at depth 24), and its cause was that every settle pass
re-evaluated every continuous assign. Evaluating only assigns whose dependencies moved
removes the quadratic term — 71.2 ms to 13.3 ms at depth 6, 814.4 ms to 57.9 ms at depth 24
(14.1×) — and changes no process execution order, because the skipped visits recompute the
same value and the write funnel discards a same-value write. Real RTL puts its
combinational work inside large `always @*` blocks rather than between processes: picorv32
has inter-process depth 1 and 4 fusion candidates out of 43 processes.

**Process fusion.** Running a connected chain of combinational processes in one activation
measures 1.7–2.5× and passes a 72-design backend-equivalence gate on stdout, VCD bytes and
run summary. It is rejected because it diverges on value. With a stimulus of
`clk = ~clk; #1`, the fused build prints `0000017c` where Icarus Verilog and the unfused
build print `xxxxxxxx`. The mechanism is §1.3: unfused, a depth-D chain propagates across D
deltas and a process waking in the same batch reads a partially propagated output; fused,
it reads a fully propagated one. A safety condition on the chain's *interior* nets does not
cover *when its output becomes fresh*, and the reader of that output is the flop the cone
exists to drive, so requiring "no concurrent reader of the output" empties the safe set.
Both values are IEEE-legal; what is violated is vitamin's own pin to Icarus Verilog. The
counterexample is pinned as
`sim-engine::backend_equiv::a_comb_chain_output_is_sampled_mid_propagation`. The equivalence
gate did not catch it because every corpus design used `#1 clk = 1`, which puts
initialisation and the first edge in different timesteps; the hazard requires both in one
activation. A gate is only as strong as the shapes inside it.

The same transform is legitimate as an advertised mode rather than a default, which is what
Verilator does. Its design, including the hazard detector whose completeness gate is
"zero candidates implies the two modes are byte-identical", is
[preview/20 — cycle-mode feasibility](../preview/20-cycle-mode-feasibility.md).

**JIT / machine-code generation.** Present in the tree behind the `jit` Cargo feature,
which is off by default, and additionally gated at run time by the `VITA_JIT` environment
variable; `VITA_JIT_STATS` prints per-run codegen statistics. cranelift is pinned at 0.120,
the newest line that builds on rustc 1.85, and adds ~29 crates. Determinism is not the
obstacle it was assumed to be: cranelift IR masks shift counts itself, so
`ushr(x, 64) == ushr(x, 0)` on both aarch64 and x86-64, which makes the determinism pin a
specification the generated code must satisfy rather than a wall.

Three measurements, all negative, and they agree:

- **Per expression.** 0.58 s to 0.67 s, +23.5 ns per call. Isolated with a callback-free
  `Const`-only program — machine code whose entire body returns two constants — the boundary
  alone is +32.6 ns over 1 228 796 runs. At 6 509 189 `eval_native` calls, a 33 ns boundary
  is 215 ms against a 130 ms target.
- **Per body.** `run_body` is called 12× less often, 542 883 times, so the same boundary
  is 18 ms. Coverage 16 of 22 templates, 382 877 of 542 883 activations = 70.5%. Result:
  0.57 s to 0.61 s, +104 ns per activation. Reducing the boundary count by 12× does not
  change the sign, because a compiled body turns every kernel call and shim op into a
  non-inlinable `extern "C"` call, where the VM path has them all inlined in Rust.
- **On tier-3, after two of the three reopen conditions were met.** 14–47% slower. About
  38% of the run is shim, half of that `jit::mk`, which rebuilds a 72-byte `Value` on every
  write — the exact representation cost tier-3 exists to avoid.

The finding that generalises: an interpreter's advantage is inlining, not dispatch, and
every boundary a JIT introduces costs more than the dispatch it removes. Correctness is not
the obstacle — the CLI and sim-engine suites pass with the JIT enabled, and running the
whole suite under `VITA_JIT=1` found a real defect in compile-time specialisation of
`Op::WriteScalar`. Two by-products are kept: `Select`, `Reduce` and `Ternary` are extracted
into `native_eval::op_*` so the VM arm and any compiled body call the same function rather
than growing a third spelling of a bit-loop rule.

**Flat mirror for leaf reads.** A probe placed word 0 of every net in one flat vector and
served leaf reads from it, with three write sites synchronised and a `debug_assert` on
every read so the debug suite proved the synchronisation. Baseline 0.57 s, flat mirror
0.57 s. `read_scalar_words` already loads the `NetSlot` to read `is_real`, `array_len`,
`width` and `signed`, so that cache line is hot and the extra pointer hop does not show.
Storage layout alone is worth nothing; representation erasure only pays as a bundle (§3.6).

**Frame arena for callee bodies.** Leaf-attributed profiles (`/usr/bin/sample`, idle thread
excluded) give the share of the run inside a frame call, and within that the share in the
generic evaluator and `Value` — the part a compiled frame body would replace:

| Design | Inside a frame call | Generic-evaluator share | Ceiling if removed |
|---|---:|---:|---:|
| aes | 88.8% | 68.0% | **3.13×** |
| keccak-arr | 82.5% | 60.4% | 2.52× |
| keccak_f | 44.8% | 39.3% | 1.65× |

The demand is bounded by which designs make frame calls at all: `frame_bodies` is **0** on
sha256, picorv32 and darkriscv, whose 38, 33 and 9 functions are all inlined by elaborate,
so a frame arena does nothing for them. Its demand is aes (18), biriscv (7) and keccak (3).

### 4.1 Measured costs of individual engine behaviours

Each figure is the wall-time share attributable to one behaviour, measured by the A/B
protocol in §5 with every pinned digest unchanged.

| Behaviour | Cost |
|---|---|
| A whole-net read at equal width copying a 72-byte `Value` in and out to perform two field writes | 17.2% of `keccak_f`, 13.7% of `keccak_f_arr`, 11–18% across the corpus |
| Two `Vec` allocations per delta, plus discarding `ca_dirty`'s capacity on every continuous-assign fixpoint pass | 14.6% of `serv`, 9.8% `sha256`, 5.0% `picorv32`, 4.9% `darkriscv`; 0 on `keccak` and `aes` |
| Rebuilding a callee's local window from the IR on every frame call | 7.7% of `keccak`, 6.8% of `aes`, 3.5% `keccak-arr`, 3.3% `sha256`, 2.2% `picorv32`, 1.4% `biriscv`, 0.5% `serv` |
| Refusing a whole process body because one expression in it holds a user call | `keccak_f.sv` 8.11 s against 6.91 s |
| Re-evaluating a `case` scrutinee once per arm | with the above, `keccak_f.sv` 8.11 s against 5.41 s (−33%) |
| Treating every `Expr::Call` as impure in the continuous-assign dirty settle, so any assign reaching a call re-evaluates forever | `verilog-ethernet` ~38 hours of simulation against 2.24 s |
| Full re-evaluation of all continuous assigns per settle pass | 814.4 ms against 57.9 ms at combinational depth 24 |
| Net-order fixpoint traversal in a fixed direction rather than alternating | 0.56 s against 0.17 s on a 3 000-link reverse chain |

---

## 5. The A/B protocol

Any performance claim in this repository is produced by this procedure. Each rule answers a
measured artifact, and the magnitude of that artifact is given so the rule is not taken on
authority.

1. **Build both ends with `--release`.** Timing a debug binary reports a fake **+88%**
   regression. `debug` is 25.9 MB against `release` 5.7 MB; check the size.
   `corpus-runner` warns when it falls back to `target/debug/vita`.
2. **Freeze both binaries.** Copy each out to a fixed path before measuring. A measurement
   against a binary that is still being rebuilt retracts its own findings.
3. **Verify the pair computes the same thing.** A drifted pair still produces two timings
   and still divides them; the failure mode looks exactly like a measurement. Assert the
   digest equality, and assert it is not the degenerate value — an all-X pair "matches".
4. **Interleave A and B; never run one block then the other.** Five PRE runs then five POST
   runs on a 0.45 s design reports a fake **+12.5%** where interleaving reports −0.9%.
   `corpus_runner::measure` takes every job at once so round-robin is the default shape.
   Interleaving is a property of the call site, not of the type: calling
   `measure(&jobs[0..1])` and then `measure(&jobs[1..2])` is block-sequential again.
5. **Run both orders, A→B and B→A.** Interleaving alone leaves a ±1% position bias that
   flips the sign of a small delta. Measured on one such change: PRE first gives POST 0.2%
   slower; POST first gives POST 1.1% faster; the true delta is 1.00×.
6. **Discard the first round.** `measure` records a sample only when `round > 0`, so
   `--reps N` means N timed samples over N+1 rounds. The default is 3.
7. **Take the median, not the mean**, and report the sample count.
8. **Run nothing else.** No parallel agents, no concurrent builds, no other load.
9. **Treat ±3% as no change.**
10. **Report coverage beside the ratio.** A body-level JIT at 7.4% coverage measuring
    "0.58 to 0.59" is not a result; the same experiment at 70.5% coverage is.
11. **Normalise before ranking.** A decline, refusal or activation count means nothing
    without the admitted count beside it (§3.5).
12. **Prove the gate can move.** A digest or golden gate that survives a mutation of its own
    design is measuring nothing and looks exactly like one that is measuring something.
    Every corpus workload is checked by mutating one line of its RTL, and a *symmetric*
    mutation can be dead honestly — in a loopback design where TX and RX share one LFSR
    instance, a CRC-polynomial change cancels at both ends, so the mutation must be
    asymmetric.

Instrumentation rules, from §3.4:

- Do not put a timer on a function called millions of times; use ablation.
- Report the observer effect of any instrument that stays in place (the region timers used
  here report ±3%), and remove the instrument after measuring — the engine's hottest path
  does not keep two clock reads.
- A profile's symbol shares are attribution, not a work breakdown.

---

## 6. Reproducing

```bash
# The pre-build probes: VM allow-list hit rate, work-per-activation crossover, depth cost
cargo test -p sim-engine --test perf_baseline --release -- --ignored --nocapture perf_p9_coverage
cargo test -p sim-engine --test perf_baseline --release -- --ignored --nocapture perf_work_per_body_crossover
cargo test -p sim-engine --test perf_baseline --release -- --ignored --nocapture perf_depth_cost_shape

# Fusion spike and fusion opportunity
cargo test -p sim-engine --test perf_baseline --release -- --ignored --nocapture perf_fusion_spike
cargo test -p sim-engine --test perf_baseline --release -- --ignored --nocapture perf_fusion_opportunity

# The call regime: the gates, then the timed row
cargo test -p cli --test perf_call_regime
cargo test --release -p cli --test perf_call_regime -- --ignored --nocapture

# The counterexample the default mode must keep answering
cargo test -p sim-engine --test backend_equiv -- a_comb_chain_output_is_sampled_mid_propagation

# Backend equivalence
cargo test -p sim-engine --test backend_equiv

# Cross-tool and cross-design timings
cargo build --release -p cli --locked
cargo run -p corpus-runner -- run --compare

# The JIT experiment: behind a feature, off by default
cargo build --release -p cli --bin vita --features jit
VITA_JIT=1 VITA_JIT_STATS=1 ./target/release/vita <design.sv>
VITA_JIT=1 cargo test -p cli --features jit
```

`crates/sim-engine/tests/perf_baseline.rs` holds fourteen `#[ignore]`d probes over fixed
designs (`CODEGEN_HEAVY`, `EVAL_HEAVY`, `EXPR_HEAVY`, `STRUCT_HEAVY`, `WIDE_HEAVY`,
`WIDE_STRUCT_HEAVY`, `REAL_HEAVY`, `CONT_ASSIGN_HEAVY`, `CONT_ASSIGN_ELEM`, `HEAP_HEAVY`,
`MEM_HEAVY`, `DUMP_HEAVY`, `SHA256_INLINE`, `SHA256_FUNCS`, `STIM_LIKE`). They time through
a `NullSink` so wall time reflects the engine rather than the sink, and they assert
`finish_reason == Finish` on every repetition. They are data, not gates: `#[ignore]` keeps
them out of the normal suite so timing variance can never fail CI, and none of them asserts
a ratio.

Writing `function` is not the same as reaching the frame path. `SHA256_INLINE` against
`SHA256_FUNCS` does not measure the call regime: those transforms are straight-line,
`body_needs_frame` is false, elaborate folds every call, and the two designs produce a
byte-identical `CodegenReport`. The keccak pair in §2.4 differs by one `for` loop, which is
the whole of what forces a frame.

---

## 7. Glossary

| Term | Meaning |
|---|---|
| **4-state** | a signal carries `0`/`1`/`x`/`z`. 2-state carries `0`/`1` only |
| **ablation** | measuring a cost by making the engine do the work twice and taking the wall-clock difference; adds no instrumentation |
| **Amdahl's law** | accelerating a fraction `f` of a run bounds total speedup at `1/(1−f)` |
| **ceiling** | an upper bound on a reachable range, not a prediction |
| **combinational depth** | how many levels of combinational logic are chained; sets the number of deltas |
| **compiled** | the design is lowered to bytecode or machine code before execution |
| **continuous assign** | `assign y = a & b;`. Module port connections lower to these |
| **cycle-based** | evaluation batched per clock, with no intra-cycle timing model |
| **delta cycle** | a step in which computation advances but simulation time does not |
| **event-driven** | execution order decided at run time from which nets changed |
| **`f`** | the wall-clock fraction eligible for an acceleration; the input to an Amdahl ceiling |
| **FFI boundary** | the call between generated machine code and Rust; inlining stops there, and it measures ≈33 ns |
| **golden** | an expected output pinned as a literal in a test |
| **levelize** | order combinational logic by dependency rank at compile time |
| **NBA** | nonblocking assignment (`<=`): all right-hand sides read, then all left-hand sides written |
| **oracle** | an external tool that supplies the expected answer; here Icarus Verilog 13, with Verilator 5.050 as a second opinion on 2-state arithmetic |
| **plane** | one of the two word arrays (`val`, `unk`) that encode a 4-state value |
| **process** | one `always` or `initial` block; the scheduler's unit of execution |
| **representation erasure** | fixing width, signedness, kind and wake set at compile time so the generated code does not contain the question |
| **self-time** | profile time attributed to a symbol; includes code inlined into it |
| **sensitivity list** | the signals that wake a process |
| **shim** | an `extern "C"` wrapper through which generated code calls Rust |
| **sign-off** | final verification before tape-out; requires 4-state and timing accuracy |
| **sparse activity** | only a small fraction of nets toggle per cycle; the algorithmic advantage of event-driven execution |
| **stratified regions** | the IEEE 1800 §4 execution phases within one time step |
| **teeth** | a test that proves a gate actually checks something |
| **trivial-shape shortcut** | skipping the VM loop when a compiled program is a single op; 46.3% of executions |
| **X-optimism** | reading `x` as `0`; hides reset defects, and is a property of 2-state simulation |

---

## 8. Related documents

- [study/02 — terminology and native-backend coverage](02-v1-native-coverage.md) — what
  the tier-3 backend accepts, how a refusal is reported, and how equivalence is gated.
- [study/03 — the workload corpus](03-workload-corpus.md) — the ten workloads, the harness,
  and the timings quoted in §2.2.
- [preview/18 — acceleration analysis](../preview/18-acceleration-analysis.md).
- [preview/20 — cycle-mode feasibility](../preview/20-cycle-mode-feasibility.md) — the
  advertised-mode form of process fusion, with its hazard detector.
- [preview/21 — tier-3 native backend](../preview/21-tier3-native-backend.md) — the survey
  of what a real machine-code backend requires.
- [preview/06 — simulation engine](../preview/06-simulation-engine.md) — the scheduler and
  region model this axis is measured against.
- [ENGINEERING_RULES](../ENGINEERING_RULES.md) — the accuracy ladder and the review method.
- [`bench/keccak/RUN.md`](../../bench/keccak/RUN.md) — the cross-tool recipe and raw table
  behind §2.1.
