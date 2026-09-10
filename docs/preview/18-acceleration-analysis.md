# 18 · Acceleration paths — the standing verdict on each

Every acceleration path that has been evaluated for the vita engine, the measurement that
decides it, the verdict that stands, and the condition that reopens it. A path with no
measurement behind it has no verdict here. Where a path is shipped, this document also
carries the contract it had to satisfy to ship: the determinism surface it may not
re-implement, the sampling moments it may not move, and the equivalence gate that proves it
did neither.

Companion documents: [study/01](../study/01-interpreted-vs-compiled.md) treats the same axis
in narrative form and holds the full A/B protocol; [preview/21](21-tier3-native-backend.md)
surveys the direction a further native backend would take;
[preview/20](20-cycle-mode-feasibility.md) holds the separate-mode question — cycle-based
scheduling and 2-state values.

---

## 1. Verdict summary

| Path | Verdict | Deciding measurement | Reopens when |
|---|---|---|---|
| Word-parallel 4-state bit operations | shipped | ~6× cumulative on an expression-bound design, with the per-bit formulas kept as a test oracle | — |
| Bytecode VM (`--backend vm`) | shipped; compiled only in an `oracle` build | expression-bound ~2.2×, structure-bound ~2.8×, wide 100-bit ~1.7×, clock-bound ~1.0× | — |
| Flat net arena + width-specialised evaluator (`--backend native`) | shipped; the default executor | picorv32 0.513 s against the VM's 0.838 s and the interpreter's 1.319 s | — |
| Machine-code generation through cranelift (`jit` feature) | rejected; off by default, kept building and measured | 14–47% slower on the default backend; ~38% of a run is boundary shim against a ceiling of 8.9–11.3% | leaf loads and 2-state arithmetic can be inlined into generated code with zero calls back into Rust, and without a second spelling of the expression semantics |
| Flat storage alone, without the rest of the bundle | rejected | 0.57 s → 0.57 s | — |
| Widening the suspend-free allow-list | rejected | refused activations are 18.1% of activations but 4.3% of time; a measured ceiling of 2.84–4.24× realised 0.2–0.3% | stimulus bodies become compute-heavy — eight or more statements per activation |
| Levelization of the Active batch | discarded | 1.00× across combinational depths 1–24; the quadratic term belonged to the continuous-assign settle, and the dirty settle closes it | no standing trigger — real RTL measures inter-process depth 1, and a design at depth ≥ 6 would be the first observation worth re-measuring |
| Process fusion in the default mode | not adopted | value divergence against Icarus Verilog, on a pinned counterexample | only as a declared mode with a hazard detector — [preview/20](20-cycle-mode-feasibility.md) |
| Global 2-state values (dropping the `unk` plane) | rejected | removing the work the second plane causes measures ~7% against a 30% bar | never as a default — [preview/20](20-cycle-mode-feasibility.md) |
| Multicore PDES within a timestep | conditional no-go | the parallelisable share is 78–82%, so the Amdahl ceiling is ≈2.5× at T=4 and ≈3.3× at T=8; the corpus's active batch width is W = 1–8 | a real design sustains batch width W ≥ ~64 with per-activation grain ≥ ~200 ns |
| GPU for the core engine | not viable | structural (§6), not a tuning gap | — |
| Stimulus-parallel GPU (Monte-Carlo regression) | a separate product | requires a branch-free cycle-based engine, which is a different engine | — |

The live queue that carries these rows, with their identifiers, is
[ROADMAP §5](../ROADMAP.md) (standing verdicts and open residues) and
[ROADMAP §7](../ROADMAP.md) (conditional items and their triggers).

---

## 2. Two orthogonal axes, and where vita sits

"Compiled simulators are fast" conflates two independent choices, and the verdicts above are
only readable once they are separated.

| | Interpreted | Compiled |
|---|---|---|
| Event-driven, full 4-state | Cadence Verilog-XL; vita's reference executor | Synopsys VCS, Cadence Xcelium, Siemens Questa |
| Cycle-based, usually 2-state | rare | Verilator |

VCS — *Verilog Compiled-code Simulator* — is the origin of the compiled line and displaced
the interpreted reference on speed alone; Cadence made the same move through NC-Verilog to
Xcelium. A sign-off simulator is compiled **and still** event-driven, 4-state and
timing-accurate. Verilator is compiled **and** cycle-based: it drops intra-cycle scheduling
and evaluates once per clock, buying one to two orders of magnitude and giving up fine
timing, part of 4-state, and sign-off standing with them.

vita is event-driven and 4-state by design — see [01-goals-and-scope](01-goals-and-scope.md),
G1. Every path in §1 either stays inside that class or is explicitly a separate mode.

### 2.1 The three executors at HEAD

| `--backend` | Availability | What it is |
|---|---|---|
| `native` | always compiled; the default | net values in a flat `u32`-indexed arena; uniform-width expressions on a specialised evaluator |
| `vm` / `bytecode` | `oracle` feature, on by default in a workspace build | each body compiled once to a bytecode op stream, then an op loop |
| `interp` / `interpreter` | `oracle` feature | walks `SimIr` on every activation; the readable reference |

In a `--no-default-features` build only `native` exists, and the two oracle spellings are a
loud rejection rather than a silent downgrade. Backend mechanics in full are in
[06-simulation-engine](06-simulation-engine.md).

### 2.2 The ceiling, measured

The compiled-versus-cycle distinction is not theoretical here. Keccak-f[1600] on macOS
arm64, release builds, interleaved samples with the first round discarded, all four tools
agreeing on the digest and anchored to the published Keccak reference value
(`bench/keccak/RUN.md`):

| Simulator | Design | Per permutation | Relative to Verilator |
|---|---|---:|---:|
| Verilator 5.050, `--binary --timing` | `keccak_f.sv` | 7.0 µs | 1× |
| vita | `keccak_f_flat.sv` (calls expanded) | 295 µs | 42× slower |
| vita | `keccak_f.sv` (function/task calls) | 2 035 µs | 291× slower |
| Icarus Verilog 13 | `keccak_f.sv` | 4 470 µs | 639× slower |

42× is the gap between an event-driven 4-state engine at its best and a compiled 2-state
cycle engine, measured on one machine and one design. It is an optimistic bound on what a
compiled backend is worth, because Verilator traded away 4-state and intra-cycle ordering to
get it, and a compiled 4-state simulator is slower than that. VCS and Xcelium are not
measured by this project; their advantage over an open-source 4-state simulator is treated as
a given, and no number here is a measurement of it.

Against the same class of tool — Icarus Verilog on the ten-workload corpus — vita's
geometric mean is 1.74× over the nine timed rows and 1.93× over the seven third-party rows.
Full table: [study/03](../study/03-workload-corpus.md).

The two readings do not conflict, and both matter for ranking an acceleration: the gap to a
cycle-based compiled tool is a **constant factor**, not a complexity class, and the paths
that would close it are exactly the ones §7 and §11 price.

---

## 3. Shipped: word-parallel 4-state bit operations

The 4-state bitwise operators and the six reductions work on `u64` words of the `val` and
`unk` planes rather than bit by bit. For AND the word formula is

```text
known0 = (~av & ~au) | (~bv & ~bu)
known1 = (~au &  av) & (~bu &  bv)
rv     = known1
ru     = ~known0 & ~known1
```

`or_w`, `xor_w`, `xnor_w` and `not_w` in `crates/sim-engine/src/value.rs` have the same
shape, and `reduce_word` with `RedKind` in `crates/sim-engine/src/eval/` covers the
reductions. The final partial word is masked with the low mask, because `not_w` and `xnor_w`
map the unused high `0 & 0` region to 1.

| Property | Contract |
|---|---|
| Oracle | the per-bit formulas stay under `#[cfg(test)]`, and `value.rs::word_vs_bit_parity` compares them bit-exactly against the word forms |
| Effect | a 64-bit AND is one word operation instead of 64, and the loop is branchless, so LLVM auto-vectorises it (NEON / AVX). Wide buses gain; a narrow design is one word and loses nothing |
| Out of scope | relational and equality comparison run on the arithmetic lanes (64/128-bit integers), a different path that is not word-ised |

`std::simd` is deliberately not used. `portable_simd` is nightly-only, which conflicts with
the stable MSRV 1.85 pin and with `--locked` byte-identical output across the supported
platforms. The stable `u64` word loop is already 64 lanes wide per word and LLVM vectorises
it, so explicit SIMD buys nothing that would justify either a nightly toolchain or the `wide`
crate; each breaks a core invariant.

### 3.1 What the word representation is worth

The operators are only part of it: the same move applies to net access, shifts, resizing and
value storage. Measured against a per-bit baseline on an expression-bound design, each step
timed against the state before it:

| Step | What became word-parallel | Time | Cumulative |
|---|---|---:|---:|
| baseline | evaluation delegated to the kernel, per-bit throughout | 2781 ms | — |
| net I/O | the net-access funnel — `slice_word`, `write_lvalue`, `write_chunk`: per-bit → `u64` words | 1274 ms | 2.18× |
| shift and resize | `value.rs` `shr_fill`, `shl_grow`, `Value::resize`: per-bit → multi-word | 948 ms | 2.9× |
| value storage | `Value.val` / `.unk` from `Vec<u64>` to `Words` — inline up to 128 bits, heap above | 618 ms | 4.5× |
| read and mask | a length guard on `mask_top`'s resize, and `read_net` reading inline rather than through a transient `BitPacked` | 461 ms | **~6.0×** |

A scheduler-bound design over the same steps goes 196 ms → 61 ms, ~3.2×.

Two properties of that ladder are the reason it is recorded here rather than in a change
log. First, **every one of these wins is on the path the interpreter and the VM share**, so
none of them is backend-specific and none of them costs a second spelling. Second, the
dominant cost was found by measurement and was not the predicted one: the two bottlenecks
predicted before the first profile — evaluation tree-walk dispatch, and `Value` heap
allocation — were each *masked* by bit-serial net I/O, and a `Value` inlining experiment
that measured ~0 before the net-write loop was word-ised measured 1.55× after it. A
bottleneck is layered; one profile does not finish the job.

---

## 4. Shipped: the bytecode VM, and the substrate rule behind it

### 4.1 Target form

Three target forms were evaluated against the project's hard constraints: cargo-only, no
`build.rs` in vita's own crates, a pinned MSRV, and `--locked` byte-identical output across
platforms.

| Target form | Speed | New dependency | Determinism pins | Verdict |
|---|---|---|---|---|
| Bytecode VM | ~2–5× | none; pure Rust | all preserved | chosen |
| Emitted native Rust | 10–100× headline | a runtime `rustc` or `cc`, plus `libloading` | the host LLVM would have to be re-proved | rejected — it puts a runtime host toolchain into a repository that forbids even a `build.rs`, and collides head-on with cargo-only, the hermetic `.velab → VCD` contract and cross-platform byte identity |
| Typed IR-2 | ~3–8× | none | preserved | second choice; the gain is small for the work |

Determinism is the first goal, so the substrate is the one that removes interpretation
overhead without touching it.

### 4.2 Compile-and-load mechanism: none

An in-process bytecode interpreter generates no code and loads none. The `.vu` and `.velab`
artifacts, the `vita` and `vrun` execution paths and the hermetic contract are all unchanged,
so the "runtime rustc / cdylib+dlopen / static dispatch" question never arises.

The determinism corollary is structural rather than tested-for: a VM opcode **dispatches**
the same 4-state and f64 primitives the interpreter calls; it does not re-implement them. The
float format and arithmetic axes therefore agree byte for byte by construction.

### 4.3 Compile-time constants in bytecode

Static widths and signedness, and folded index, width and count values, are encoded as
**immediate operands or const-pool indices on the op** — not as a separate node type and not
as an emitted literal. Shallow folds and per-site fallbacks are computed once at
bytecode-compile time and frozen into the immediate.

### 4.4 Golden impact: none

Bytecode, VM state and the whole backend seam live outside `sim_ir::SimIr` — in `SimOpts`
sidecars or in separate modules — so `schema_hash::<SimIr>()` is unaffected by any of it, and
a backend change never needs a `format_version` bump. See
[16-schema-hash-spec](16-schema-hash-spec.md) and
[17-sim-ir-ir-backbone-freeze](17-sim-ir-ir-backbone-freeze.md).

### 4.5 What the VM is worth

Release build, best of five, `crates/sim-engine/tests/perf_baseline.rs`:

| Workload shape | VM against the interpreter |
|---|---|
| Expression-bound | ~2.2× |
| Structure-bound (select / concat / replicate) | ~2.8× |
| Wide, 100-bit | ~1.7× |
| Clock- or scheduler-bound | ~1.0× — evaluation is not the bottleneck there |

The VM's allow-list is a positive list over terminators (`backend::is_codegen_able`), so a
body holding `#delay`, `@`, `fork` or a call terminator stays on the walk. Its reject-reason
keys — `"delay"`, `"wait"`, `"fork"`, `"frame_call"` — are stable and appear in `run.json`'s
`codegen` histogram.

The interpreter is **excluded from performance work by rule**: making the reference faster is
how a reference stops being readable, and a second spelling of a rule is this repository's own
defect class.

---

## 5. Shipped: the flat arena and specialised evaluator

The default backend owns net storage — a single flat `u32`-indexed word buffer,
`native::arena::NetArena` — so it is all-or-nothing per design rather than per body.
Measured on picorv32, release, interleaved, best of five:

| Executor | Time | Against native |
|---|---:|---:|
| `--backend interp` | 1.319 s | 2.57× slower |
| `--backend vm` | 0.838 s | 1.63× slower |
| `--backend native` | **0.513 s** | 1.00× |
| Icarus Verilog 13 | 0.585 s | 1.14× slower |

Corpus eligibility is 6,470 / 6,470 = 100.00% with zero refusals.

The load-bearing lesson for any future acceleration is that this backend pays because four
things land **together**: static net allocation, width-specialised operations, erasure of the
schedule lookup into direct calls or static bits, and a specialised NBA record. None of them
pays alone, which §7 and §8 measure directly.

---

## 6. Not viable: GPU for the core engine

Event-driven RTL simulation is hostile to a GPU for six independent reasons, and none of them
is a tuning gap.

1. **Branch divergence.** Every process is data-dependent `if` / `case` / loop, which is SIMT
   warp divergence, which is a throughput collapse.
2. **Sparse activity.** Only the nets and processes that changed in a timestep are
   re-evaluated, and that is a very small fraction. A GPU wants dense, uniform work.
3. **Temporal causality.** Time T depends on T−1, so there is no parallelism *across*
   timesteps — only independent processes *within* one, which is exactly the sparse,
   divergent set above.
4. **Pointer chasing.** The IR is an index-edge arena and evaluation is a recursive tree
   walk, so memory access is uncoalesced.
5. **Fine-grained synchronisation.** NBA, delta cycles and the layered regions are barriers
   and atomics.
6. **The industry position agrees.** VCS, Xcelium and Questa are all CPU tools. Published GPU
   simulation work targets *different* problems — large cycle-based gate netlists, and
   stimulus-parallel regression farms.

Stimulus-parallel GPU — many independent Monte-Carlo runs — is a coherent product, but it
requires a branch-free cycle-based engine. That is a different engine, not an acceleration of
this one.

---

## 7. Rejected: machine-code generation through cranelift

The `jit` Cargo feature is present, builds, is wired into the default backend, is measured
and is correct. It is off by default and stays off.

### 7.1 Status at HEAD

| | |
|---|---|
| Cargo feature | `jit`, OFF by default, declared on `sim-engine` and forwarded by `cli` |
| Runtime gate | additionally requires the `VITA_JIT` environment variable; `VITA_JIT_STATS` prints per-run codegen statistics |
| Dependency | cranelift 0.120, the newest line that builds on rustc 1.85 (0.134 requires 1.94); ~29 crates |
| Unsafe surface | the call boundary — the only `unsafe` in the workspace besides `cli/src/frontend.rs`'s `signal(2)` |
| Correctness | the CLI and sim-engine suites pass with the JIT enabled |
| Determinism | not the obstacle it was assumed to be: cranelift IR masks shift counts itself, so `ushr(x, 64) == ushr(x, 0)` on both aarch64 and x86-64. What remains is that cranelift's definition is not Verilog's definition, which is one architecture-independent guard |

### 7.2 Three measurements, all negative and all agreeing

**Per expression.** 0.58 s → 0.67 s, +23.5 ns per call. Isolated with a callback-free
`Const`-only program — machine code whose entire body returns two constants — the boundary
alone is +32.6 ns. At 6,509,189 `eval_native` calls, a 33 ns boundary is 215 ms of cost
against a 130 ms target, so the sign was arithmetically settled before any code-quality work.

**Per body.** Raising the compilation unit to the body calls the boundary 12× less often —
542,883 times, 18 ms of boundary. Coverage is 16 of 22 templates, 382,877 of 542,883
activations = 70.5%. Result: 0.57 s → 0.61 s, +104 ns per activation. Cutting the boundary
count by 12× does not change the sign, because a compiled body turns every kernel call and
shim op into a non-inlinable `extern "C"` call, where the VM path has them all inlined in
Rust.

**On the default backend**, with the flat arena and width specialisation already in place:

| Shape | JIT off | JIT on | Δ |
|---|---:|---:|---|
| struct-heavy | 55.1 ms | 81.2 ms | +47.4% |
| eval-heavy | 62.9 ms | 87.5 ms | +39% |
| mem-heavy | 81.6 ms | 107.4 ms | +32% |
| expr-heavy | 116.1 ms | 148.6 ms | +28% |
| picorv32 | 514 ms | 587 ms | +14.1% |

### 7.3 The verdict is arithmetic

The profile names the cost: about **38% of the run is shim** — `s_load` at 13.7%, a call back
into Rust for every leaf, and `jit::mk` at 12.4%, which rebuilds a 72-byte `Value` on every
write. That marshalling is the exact cost the flat arena exists to remove, and it reappears at
the boundary.

What perfect code generation can remove is **op dispatch only**, and that is 8.9–11.3% of the
run, while the boundary is ~38% — 25% even if `jit::mk` disappeared entirely. It is paying
25% to win 11%. On top of the arithmetic sits the structural cost: a **second implementation
of expression semantics** beside the interpreter and the VM, which is this engine's documented
defect class.

An independent census closes it from the other side: 56–86% of executed compiled-lane programs
are a **single** `Load` or `Const` op. There is no dispatch loop worth compiling away.

### 7.4 The reopening condition, and it is one condition

Generated code must **inline the leaf load and the 2-state arithmetic** so that it makes zero
calls back into Rust, and it must do so without spelling the expression semantics a second
time. Two of the three prerequisites exist: a leaf is a pair of words at a compile-time index,
and the arithmetic is ordinary integer arithmetic. The missing piece is the "without a second
spelling" half.

### 7.5 What the experiment leaves behind

- `Select`, `Reduce` and `Ternary` are extracted into `native_eval::op_select`, `op_reduce`
  and `op_ternary`, so the VM arm and any compiled body call one function rather than growing
  a third spelling of a bit-loop rule.
- Running the whole suite under `VITA_JIT=1` found a real defect in the compile-time
  specialisation of `Op::WriteScalar`: the specialised op carries no `ResolveOff`, and a
  failed RHS inline routed to a shim that consumes an offset.

---

## 8. Rejected: flat storage without the rest of the bundle

A probe placed word 0 of every net into one flat vector and served leaf reads from it, with
the three write sites synchronised and a `debug_assert` on every read so the debug suite
proved the synchronisation. Its memory-access profile matches a real arena: one indexed load
instead of two pointer hops.

| | |
|---|---|
| Baseline | 0.57 s |
| Flat mirror | 0.57 s |

No difference, and the reason generalises: `read_scalar_words` already loads the `NetSlot` to
read `is_real`, `array_len`, `width` and `signed`, so that cache line is hot and one more
pointer hop does not register.

**Storage layout alone is worth nothing.** The arena pays only as part of the bundle in §5.
This measurement is why "flatten the store first, then decide" is the wrong order and "the
four together, or not at all" is the right one.

---

## 9. Rejected: widening the suspend-free allow-list

The bodies the compiled path refuses are those holding `#delay`, `@` or `fork`, which are
overwhelmingly stimulus. Sweeping work per activation on bodies that are *already* admitted
answers the value question without first building the resume-PC state machine that widening
would require:

| Statements per activation | Interpreter | VM | VM / interpreter |
|---:|---:|---:|---:|
| 1 | 81.8 ms | 81.3 ms | 0.99× |
| 2 | 94.9 ms | 93.6 ms | 0.99× |
| 4 | 117.7 ms | 113.9 ms | 0.97× |
| 8 | 159.7 ms | 145.0 ms | 0.91× |
| 16 | 242.8 ms | 203.8 ms | 0.84× |
| 32 | 408.1 ms | 319.7 ms | 0.78× |
| 64 | 740.4 ms | 554.8 ms | 0.75× |

The compiled path never loses, and below about eight statements it gives nothing: the
per-activation fixed cost — register-file lease, prologue, dispatch loop — is not amortised.
Stimulus bodies are one to three statements, so the bodies this axis would absorb sit exactly
in the tie region.

On a real design and testbench the refused activations are 18.1% of activations but only
**4.3% of wall time**, 257 ns each against 709 ns for a compiled body, and that is this axis's
ceiling. The Amdahl ceiling computed before the sweep was 2.84–4.24×; the realised value is
0.2–0.3%. A ceiling bounds the reachable range and does not predict it. Widening also buys a
resume-PC state machine, which is new silent-wrong surface.

Reopens when stimulus bodies become compute-heavy.

---

## 10. Discarded: levelization. Not adopted: process fusion

**Levelization** — static combinational ranks, an Active batch drained in rank order, a settle
between ranks — measures **1.00× across depths 1 through 24** once built. A depth sweep at
fixed cycle count explains it: a pure combinational chain is linear in depth (3.3 ms at depth
1, 31.5 ms at depth 24 within one module) and the wake chain carries one process per delta, so
there is no batch to sort. The quadratic term appeared only when the chain ran through
continuous assigns (7.8 ms at depth 1, 814.4 ms at depth 24), and its root was that every
settle pass re-evaluated every continuous assign. Evaluating only the assigns whose
dependencies moved removes it — 814.4 ms to 57.9 ms at depth 24, 14.1× — and changes no
process execution order, because a skipped visit recomputes the same value and the write
funnel discards a same-value write.

Real RTL puts its combinational work inside large `always @*` blocks rather than between
processes: picorv32 has inter-process depth 1 and four fusion candidates out of 43 processes.
There is no standing trigger on this row; a real design measuring inter-process combinational
depth ≥ 6 would be the first observation worth re-measuring against.

**Process fusion** — running a connected chain of combinational processes in a single
activation — measures 1.7–2.5× and passes the 72-design backend-equivalence gate on stdout,
VCD bytes and run summary. It is still not adopted, because it diverges on **value**: with a
`clk = ~clk; #1` stimulus the fused build prints `0000017c` where Icarus Verilog and the
unfused build print `xxxxxxxx`. Unfused, a depth-D chain propagates across D deltas and a
process waking in the same batch reads a partially propagated output; fused, it reads a fully
propagated one. A safety condition on the chain's *interior* nets says nothing about *when its
output becomes fresh*, and the reader of that output is the flop the cone exists to drive, so
"no concurrent reader of the output" empties the safe set. Both values are IEEE-legal; what is
violated is vita's own pin to Icarus Verilog, which puts this on the silent-wrong rung of
vita's own ladder.

Pinned as `sim-engine::backend_equiv::a_comb_chain_output_is_sampled_mid_propagation`. The
same transform is legitimate as a **declared mode** with a hazard detector, which is
[preview/20](20-cycle-mode-feasibility.md).

---

## 11. Conditional no-go: multicore PDES within a timestep

Byte-identical determinism is **not** the blocker — a preserving design exists and is sketched
below. The blockers are workload width, the serial residue, and the engineering cost.

### 11.1 The three probes

All three live in `crates/sim-engine/tests/perf_baseline.rs` as permanent instruments —
`perf_pdes_sync_cost`, `perf_pdes_engine_grain`, `perf_pdes_bsp_mock` — measured on a 10-core
Apple Silicon machine.

**τ, the per-delta dispatch round trip.**

| Dispatch shape | T=2 | T=4 | T=8 |
|---|---:|---:|---:|
| naive `thread::scope` spawn/join | 31 µs | — | 93 µs |
| resident pool + spin barrier (generation scatter, countdown gather) | 294 ns | 471 ns | 2.0 µs |

Naive spawn/join is fatal on its own: a delta's work is measured in microseconds and the
dispatch costs tens of them. The T=8 surge is the spill past the performance-core count onto
efficiency cores, so a real design has to clamp T to the P-core count.

**g, the engine activation grain.** W independent four-NBA flop `always` blocks, the ideal
maximum-parallelism case: marginal cost **~700 ns per activation** (W=16→1024 converges
707→686 ns; W=1 is 1351 ns and includes fixed slot overhead).

Self-time classification of one such run:

| Class | Share | Where |
|---|---|---|
| Parallelisable | ~78–82% | `eval_binary_ctx`, `mask_top`, `eval_ctx`, `resize`, `read_net`, the NBA capture in `k_schedule_nba`, `Value` allocation — all per-process work |
| Serial residue | ~18–22% | `write_chunk` and `write_lvalue` on the NBA apply side, `propagate_changes`, the sort — the commit and propagate phase |

**BSP mock, the dispatch-side speedup matrix.** Resident pool, static chunk ownership, serial
commit pass — the same deterministic dispatch shape as the design sketch. At T=4, corrected to
the measured grain:

| grain | W=8 | W=64 | W=512 | W=4096 |
|---|---:|---:|---:|---:|
| ~28 ns | 0.39× | 1.56× | 2.96× | 3.51× |
| ~195 ns | 1.59× | 3.09× | 3.61× | 3.69× |
| ~890 ns | 2.93× | 3.61× | 3.72× | 3.95× |

T=8 wins only when W×g is large — peak 5.1× at g≈890 ns, W=4096. Under light load the
efficiency-core spin is counterproductive, down to 0.11×.

### 11.2 The ceiling

A serial residue of ~20% gives an Amdahl ceiling of **≈2.5× at T=4, ≈3.3× at T=8, 5× at
T=∞**. Taking the minimum of that and the dispatch-side matrix, an ideal wide-synchronous
design at g≈700 ns reaches ≈2–2.5× at T=4 and ≈3× at T=8, once W ≥ 64. At W ≤ 8 it is ≤1.6× or
a loss, and at W = 1 — every testbench-shaped workload in the corpus — there is nothing to
parallelise at all.

### 11.3 The byte-identical design sketch

This is the evidence for "possible, but expensive", not a plan.

| Element | Design |
|---|---|
| Eligible class | suspend-free processes whose writes are all NBA — the `is_codegen_able` classification, reused. NBA-pure processes cannot observe each other within a delta, because their writes land in the NBA region, so they are parallel-safe with no read/write-set analysis |
| Run splitting | a process that writes with blocking assignment, forks, or touches the dynamic heap splits the batch. Only the pure-eval spans between splitters scatter; splitters run serially in batch order, so sequential visibility is preserved by construction |
| NBA ordering | capture keyed on `(batch_idx, intra_seq)` merges to exactly the total order the global sequence number gives today |
| Output ordering | `$display` / `$strobe` / `$monitor` registration goes to a per-process buffer flushed in batch order. Sequential output is per-process contiguous, so the bytes are identical |
| `$finish` and friends | the lowest `batch_idx` wins and later index logs are discarded, reproducing the sequential mid-batch stop. A pure-eval process writes no state directly, so discarding is free |
| Dirty list | per-thread collection, then merge, then the existing sort — the settle is already sort-based, so this joins naturally |
| Time wheel | not involved: pure-eval bodies are suspend-free, so they hold no mid-body delay or `@` |

The engineering cost is what makes it expensive: dismantling `!Send` — `Box<dyn Write>`,
`LogSink`, `Cell`, the `Rc` VM cache, nine sites, each becoming per-worker — per-worker
`EvalCtx` and native evaluation stacks, and a corpus-wide `--threads 1` against N byte-diff
gate. A v2 that parallelises the NBA apply itself over disjoint nets could lower the serial
share toward ~10% and open a ceiling near 10×, but propagation — edge detection, waker
ordering, region control — is order-sensitive by nature and falls outside that scope.

### 11.4 Verdict and re-entry

Not implemented. A ceiling of 2–3× is realised only by a sustained "W ≥ 64 plus
evaluation-dominated bodies" workload, which is absent from the corpus and from the Phase-1
user workloads; the machine cost — threading work, deterministic merge, gate maintenance —
exceeds that conditional gain.

**Re-entry condition**: a real design shows, over most of its simulated time, (a) an active
batch width **W ≥ ~64** and (b) a parallel-phase grain of **≥ ~200 ns** per activation. Below
either, the mock says ≤1.6× and the Amdahl and merge constants eat it. On entry, the sketch
above plus a byte-diff gate is the starting point.

### 11.5 `--threads` is not this

`--threads` / `-j` is the waveform writer's thread budget, not simulation parallelism. At
N ≥ 2 the VCD **file writes** move to a dedicated writer thread behind an order-preserving
bounded FIFO; the simulation thread still performs all deterministic work, including VCD
encoding and record ordering, and hands over finished byte chunks. Output is byte-identical
for every thread count, pinned by `crates/sim-engine/tests/threads.rs`. A run that writes no
waveform is unaffected by the flag.

---

## 12. Contracts an acceleration must satisfy

Any backend, lane or mode added to this engine inherits all four of the following. They are
not advice; three of them are gates.

### 12.1 The float-path determinism surface

The justification for a second execution path is that cross-platform byte identity survives
it, and the float path is the least pinned axis. These functions are **reused verbatim** by
every path — never re-implemented, never compiled with fast-math.

| Function | File | Determinism basis |
|---|---|---|
| `dec_field_width(n, signed)` | `crates/diag/src/fmt.rs`, forwarded by `builtins/render.rs` | exact `u128` integer arithmetic to 128 bits; only above 128 does it use `n · LOG10_2` in f64, and that result is a column-alignment hint |
| `fmt_dec`, real arm | `crates/sim-engine/src/builtins/render.rs` | `x.round() as i64`, saturating; NaN → 0 |
| `fmt_real` (`%f`) | `builtins/render.rs` | Rust `{:.*}`, not libm |
| `fmt_real_e` (`%e`) | `builtins/render.rs` | Rust `{:.p$e}` plus two-digit exponent padding |
| `format_g` (`%g`) | `builtins/render.rs` | the exponent comes from Rust `{:e}`; `log10` is deliberately avoided because libm transcendentals are not byte-identical across platforms; ±0.0 canonicalised |
| `Value::from_f64`, `Value::to_f64`, `real_to_int_round` | `crates/sim-engine/src/value.rs` | int↔real through `as f64` and round-half-away |

Because every path calls the *same instance*, `%f`, `%e`, `%g`, `%t`, `%d`-on-real and `%d`
above 128 bits agree byte for byte. The backend differential gate enforces it, and the
checked-in golden `sim-engine::end_to_end::float_format_determinism_golden` locks
reproducibility across platforms: every platform matches the same literal, which is equivalent
to a cross-platform diff, and CI runs the same golden on each leg so a divergent platform fails
its own leg.

Real-math (`$ln` … `$atanh`) and the non-uniform `$dist_*` functions run on the vendored
pure-Rust libm — `third_party/libm`, `default-features = false`, so no hardware intrinsics —
for the same reason.

### 12.2 The sampling-moment contract

For a second execution path to be byte-identical, "when is what read and written" has to be a
contract. These seven moments are frozen. Most are **kernel-side** (`write_lvalue`,
`schedule_nba`, `propagate_changes`, `emit_vcd_change`), and a compiled body calls the *same*
kernel methods through the `Kernel` trait, so it reproduces them for free. The one body-side
invariant a compiled path must actively preserve is **statement execution order**.

| # | Moment | Site | Class | Obligation on a compiled body |
|---|---|---|---|---|
| 1 | continuous-assign fixpoint in declaration order | `sched::settle_cont_assigns` | kernel-side | a continuous assign is not a process body; none |
| 2 | NBA: LHS index sampled at schedule time, `nba_seq` applied in order | `sched::propagate::schedule_nba*`, `apply_nba` | kernel-side plus **body order** | call the NBA kernel entry in statement text order; text order *is* `nba_seq` |
| 3 | blocking-assign offsets resolved at statement time | `exec::compute_effect` | body, read phase | resolve offsets before the write; already captured in `StmtEffect` |
| 4 | in-body `@(sig)` arm snapshot, Level arms only | `sched::propagate::suspend_on` | kernel-side | `@` suspends, so the body is outside the allow-list; none |
| 5 | delayed `assign #d` keyed on the last continuous-assign change | `sched` | kernel-side | delayed continuous assigns are not bodies; none |
| 6 | `propagate_changes` refreshes `prev` last | `sched::propagate::propagate_changes` | kernel-side | none — backend-neutral |
| 7 | eager per-write VCD emission, for glitch fidelity | `state::write_chunk` → `emit_vcd_change` | kernel-side | none — shared funnel |

Of the seven, a compiled body is actively responsible for **#2 and #3 only**, and both are
satisfied by executing statements in the same order the reference does. Breaking any of the
seven turns the equivalence gate red immediately.

Corpus coverage of the moments: #2 by `nba_sample` (`a[i] <= v; i = i + 1`), #3 by array and
out-of-bounds writes, #7 by `multi_write_glitch` (three writes in one delta), #1 and #5 by
`cont_assign_mixed`. #4 is outside the compiled path by construction and is covered by the
interpreter suites.

### 12.3 The equivalence gates

| Gate | File | What it compares |
|---|---|---|
| Backend differential | `crates/sim-engine/tests/backend_equiv.rs` | 72 generated corpus designs built once into a `SimIr`, then run on the interpreter and the VM concurrently; identical stdout, VCD bytes, `sim_time`, `finish_reason` and `exit_class`, plus hand-written shapes the generator cannot emit. Anti-vacuity: `gate_actually_compares_vcd_bytes` |
| Tier-3 differential | `crates/sim-engine/src/native/run_tests.rs` | the same IR on the VM and on the default backend, with per-design VCD targets; asserts the native backend actually ran, first, so a fall-back cannot make the gate compare the VM against itself |
| Design gate | `crates/sim-engine/tests/native_gate.rs` | each reject family actually fires; corpus eligibility is an exact pinned count |
| iverilog differential | `crates/sim-engine/tests/differential.rs` | vita against `iverilog` + `vvp`; skips gracefully when the tools are absent, and the design still runs through vita, so a vita-side crash is still caught |

The invariant the whole scheme rests on: the shared net-write and VCD choke point
(`state::write_lvalue`, `emit_vcd_change`) stays on the **shared** side across backends, so
only process-body control flow differs and VCD or stdout bytes cannot diverge in a
backend-specific way.

A stronger check than running the corpus on two `--backend` values is **flipping the default
and running the whole suite**: it costs one line and about ten minutes, and it recruits every
shape the whole test suite already encodes, rather than only the shapes the corpus generator
emits.

### 12.4 The A/B protocol

No performance number enters this document without it. The full protocol, with the artifact
behind each rule, is [study/01 §5](../study/01-interpreted-vs-compiled.md); the rules
themselves:

| Rule | The artifact it answers |
|---|---|
| Release binaries only | a debug binary produced a fake +88% picorv32 regression |
| Interleave A and B; never block-sequential | 5×PRE then 5×POST gave a fake +12.5% where interleaving gave −0.9% |
| Run both orders, A→B and B→A | interleaving alone leaves a ±1% position bias that flips the sign |
| Discard the first round | cache and thermal warm-up |
| ±3% is "no change" | the standing no-change band |
| No concurrent load while measuring | a hard rule |
| The pair must compute the same thing | a drifted pair still yields two timings and still divides them |
| A digest or golden gate must move under a mutation of its own design | a gate that survives one is measuring nothing and looks exactly like one that is |
| Freeze the binary under review | a lens measuring a binary that keeps being rebuilt retracts its findings |
| Prefer ablation to instrumentation | per-body timers reported `bodies = 3762 ms` on a run whose uninstrumented wall clock was 1234 ms |

---

## 13. Reproducing the cross-tool measurements

`bench/keccak/` is first-party RTL carried in this repository. Verilator and Icarus Verilog
are installed separately.

```bash
cargo build --release -p cli --locked

cd bench/keccak
vita tb.sv keccak_f.sv      +N=200                 # the call spelling
vita tb.sv keccak_f_flat.sv +N=200                 # the expanded spelling
vita --backend interp tb.sv keccak_f.sv +N=200     # executor A/B (oracle build only)

iverilog -g2012 -o k.vvp tb.sv keccak_f.sv && vvp k.vvp +N=200
verilator --binary --timing -Wno-fatal -o vk tb.sv keccak_f.sv && ./obj_dir/vk +N=200000
```

The permanent probes behind §7, §9 and §11 are `#[ignore]`d and are data, not gates:

```bash
cargo test --release -p sim-engine --test perf_baseline -- --ignored --nocapture
```

---

## 14. Related documents

- [study/01 — the performance axis](../study/01-interpreted-vs-compiled.md): the same
  verdicts in narrative form, the value-representation census, the flat profile, and the full
  A/B protocol.
- [study/03 — the workload corpus](../study/03-workload-corpus.md): the ten designs and the
  harness every corpus figure here comes from.
- [preview/20 — cycle-mode feasibility](20-cycle-mode-feasibility.md): the separate-mode
  question, both halves of it.
- [preview/21 — tier-3 native backend](21-tier3-native-backend.md): the direction a further
  native backend would take.
- [preview/06 — simulation engine](06-simulation-engine.md): scheduler regions, backend
  mechanics, the `Kernel` ABI.
- [ROADMAP §5 and §7](../ROADMAP.md): the live queue, the standing verdicts, and the re-entry
  triggers.
