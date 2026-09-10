# 20 · A separate cycle mode — feasibility and standing verdict

A cycle-based mode alongside the default event-driven engine is the one acceleration that
would move vita out of its performance class rather than within it. This document states
what such a mode is, what it would require, what the two deciding measurements show, the
verdict that stands, and the condition that reopens each half. It also carries the design
the mode would have to satisfy if it were ever built, because that design is what makes the
mode honest rather than merely fast.

Status at HEAD: **not implemented.** There is no mode flag, no second value representation
and no static schedule. The default engine is event-driven and 4-state on every backend.

Preceding measurement: [preview/18](18-acceleration-analysis.md). Background on the two
axes: [study/01](../study/01-interpreted-vs-compiled.md). The queue rows are
[ROADMAP §5.a and §7](../ROADMAP.md).

---

## 1. The mode is two independent halves

"Cycle-based" bundles two changes that are orthogonal, and each has its own measurement and
its own verdict.

| Half | What it changes | Verdict | Reopens when |
|---|---|---|---|
| **Scheduling** | evaluate every combinational block once per cycle in a static level order, instead of running an event queue with delta settling | rejected | a real design evaluates each combinational block at least once per cycle |
| **Value domain** | drop the `unk` plane and carry two-state values only | rejected | never as a default; as a mode, only with a way that trades no correctness |

Either half can be taken without the other. Verilator takes both. The rest of this document
treats them separately, then states what a mode taking either would still owe.

---

## 2. The scheduling half

### 2.1 The arithmetic

A cycle mode erases the event queue, delta settling, change detection and process wake, and
replaces them with a single static-level pass over every combinational block, once per
cycle. Writing `E_act` for the activations an event-driven run actually performs and
`C_blk` for the block evaluations a cycle pass would perform:

```text
event = E_act × (eval + sched)
cycle = C_blk ×  eval

cycle wins  ⟺  C_blk / E_act  <  1 + sched/eval
```

The measured `sched/eval` ratio is ≈ 0.84, so the gate is **1.84**.

### 2.2 The measurement

An activation census — pure counting, no correctness impact:

| Design | Combinational blocks | Cycle-mode evaluations | Current evaluations | Ratio | Evaluations per block per cycle | Verdict |
|---|---:|---:|---:|---:|---:|:--|
| picorv32, 200k cycles | 230 = 31 processes + 199 assigns | 46,000,000 | 4,457,425 | **10.32** | **0.097** | FAIL |
| keccak | 7 = 1 + 6 | 18,221 | 11,021 | 1.65 | 0.605 | marginal, 11% |

picorv32's combinational blocks are evaluated **0.097 times per cycle**: event-driven
scheduling is already skipping 90.3% of the combinational work. A cycle mode would do all of
it in exchange for erasing at most 0.84 of an evaluation's worth of scheduling — ten times
the work to save 0.84. In a control-dominated CPU-shaped design, event-driven scheduling
*is* the optimisation. Only a dense datapath (keccak) reaches the gate at all, and its 11%
margin is inside the model's own error.

### 2.3 Verdict and re-entry

Rejected. The ratio is a function of the design's **activity rate**, not of the engine, so
the re-entry condition is a workload fact rather than an engineering one: a corpus with real
demand whose combinational blocks average **at least one evaluation per cycle**. picorv32 is
at 0.097.

### 2.4 The related transform, and why it is not a shortcut

Process fusion — collapsing a connected chain of combinational processes into one activation
— is the cheap approximation of a static level order, and it measures 1.7–2.5×. It is not
adopted in the default mode because it changes **values**, not only speed: unfused, a
depth-D chain propagates across D deltas and a process waking in the same batch reads a
partially propagated output; fused, it reads a fully propagated one. With a
`clk = ~clk; #1` stimulus the fused build prints `0000017c` where Icarus Verilog and the
unfused build print `xxxxxxxx`. Both values are IEEE-legal, and what is violated is vita's
own pin to Icarus Verilog, which is exactly the silent-wrong rung of the ladder.

A safety condition on the chain's *interior* nets does not cover *when its output becomes
fresh*, and the reader of that output is the flop the cone exists to drive, so requiring "no
concurrent reader of the output" empties the safe set. The counterexample is pinned as
`sim-engine::backend_equiv::a_comb_chain_output_is_sampled_mid_propagation`.

That is the precise sense in which the transform is legal only as a **declared mode**: a
mode announces the semantic change, so a value difference is a documented mode property
rather than a wrong answer. §5 is the design that would have to carry that announcement, and
a fusion-based cycle mode owes the same hazard detector with the same completeness gate.

Opportunity is also small in real RTL: picorv32 has inter-process combinational depth 1 and
four fusion candidates out of 43 processes, because real designs put their combinational work
inside large `always @*` blocks rather than between processes.

---

## 3. The value half — dropping the `unk` plane

### 3.1 What it would remove

A vita value carries two word planes.

| `val` | `unk` | Meaning |
|---|---|---|
| 0 | 0 | `0` |
| 1 | 0 | `1` |
| 0 | 1 | `x`, unknown |
| 1 | 1 | `z`, high impedance |

A 2-state mode would carry `val` alone. Storage, copying and masking halve, and the
operators shrink by much more than half, because a 4-state AND computes known-0 and known-1
separately (`value.rs::and_w`):

```text
4-state AND :  known0 = (~av & ~au) | (~bv & ~bu)
               known1 = (~au &  av) & (~bu &  bv)
               rv = known1 ;  ru = ~known0 & ~known1     ≈ 10 word operations
2-state AND :  rv = av & bv                              =  1 word operation
```

| Item | 4-state | 2-state |
|---|---|---|
| Bitwise operator | ~10 word operations | 1 |
| `mask_top` | two planes masked, two length checks | one |
| `resize` | two planes copied and sign-extended | one |
| Net storage | `{val, unk}` | half the memory, half the cache pressure |
| `from_packed` / `get_vu` / `set_vu` | two planes | one |

Arithmetic is the exception and is already cheap: a partially known sum is impossible, so any
`x` in either operand poisons the whole result, which is one branch plus a two-state add.
Operations are not uniformly expensive.

### 3.2 The measurement

An instruction count is not a time measurement. A scratch build short-circuited `and_w`,
`or_w`, `xor_w`, `xnor_w` and `not_w` to their 2-state forms and removed the `unk` handling
from `mask_top` and `resize`, buying the upper bound directly and reverting afterwards.
Self-validation: the scratch build produced **identical output** to the normal build
(`trap=0 addr=00000014`), so the measured region is x-free, control flow is the same, and the
timing comparison is valid.

| Executor | 4-state | 2-state scratch | Ceiling |
|---|---:|---:|---:|
| Interpreter | 1230.7 ms | 1149.6 ms | **1.071×** |
| Bytecode VM | 1067.0 ms | 1025.8 ms | **1.040×** |

Widening the patch from `and`/`or` alone to `xor`/`xnor`/`not` plus `resize` moved it from
1.067× to 1.071× — essentially not at all. The only unpatched item left is net storage, and
the design has 238 nets, so there is nothing there either.

**The `unk` plane is about 7% of the cost, against the 30% bar set for taking the trade.**
The second plane is usually all zeros, stays in cache, and `& 0` retires nearly free on a
superscalar core.

An execution-weighted census of every evaluated value on the corpus workloads says the same
thing from the other side: 83.9% to 100% of values are simultaneously definite and at most 64
bits, geometric mean 95.7%. The 2-state shape the workloads need is already reachable per
operation, and the compiled lane already carries it — what limits it is how much of a design
reaches that lane, not the presence of the plane.

### 3.3 Where the cost actually is

The profile of a real design is flat, which is what "no single lever" means:

```text
eval          26.7%
resize        16.6%
netread       13.1%
mask_top      13.0%
eval_binary   12.5%
```

Every one of these bounds the whole run at 1.14×–1.36× even if it became free, and only part
of each is removable — a one-word fast path in `resize` measures about 1%, reproducibly, on
two independent attempts. The five share a cause, but the cause is the interpretation
structure (a `Value` constructed and moved per node, a tree walk, indirection), not the fact
that there are two planes; the 7% measurement is what separates those two readings.

### 3.4 Verdict and re-entry

Rejected. As a **default** it is rejected permanently, because it is a step down the accuracy
ladder (§4) and G1 is the whole point of the tool. As a **mode** it is rejected on the
number: a 7% ceiling does not pay for a second value representation threaded through the
engine.

Re-entry requires a way to reach the gain with no correctness trade at all — not a better
argument for taking the trade.

---

## 4. What a 2-state mode gives up

This is a semantic reduction, not a performance trade-off, and the mode's honesty depends on
saying so.

| Lost | Consequence |
|---|---|
| X propagation | an uninitialised register reads as `0`, not `x` |
| `===` / `!==` | a comparison that distinguishes x and z becomes meaningless |
| `casez` / `casex` | don't-care matching loses its basis |
| `z` and tri-state | wired-and/or resolution and bus multi-driving are inexpressible |
| X-checking assertions | `assert(!$isunknown(x))` and its family always pass |

### X-optimism is the real hazard

When an uninitialised signal propagates as `x`, a bug becomes visible. When it reads as `0`,
the bug passes silently. That is how a 2-state simulation passes while the real chip is
wrong; the industry calls it X-optimism, and it is why 2-state tools are not used for
sign-off.

This is the reason such a mode can only ever be a **separate, declared mode**. Making it the
default would remove vita's G1 claim.

---

## 5. How a mode would keep correct-or-loud

Declaring the limitation in documentation is not enough: a user does not know whether their
design is in the affected set. The rule vita would apply is the same one it applies
everywhere — **refuse, rather than differ quietly**.

| Construct | 2-state mode |
|---|---|
| `===`, `!==`, `$isunknown`, `casez`, `casex` | refuse, with an error code, naming the default mode as where to run it |
| Explicit `'x` / `'z` literals | refuse |
| Tri-state driving (`assign y = en ? d : 1'bz`), `wand` / `wor` | refuse |
| Reading an uninitialised signal | defined as `0`, with a warning, behind an explicit option |

That keeps the mode on the **loud** rung: what it supports is exact, and what it cannot do is
noisy. The refusal list is also the mode's own proof obligation — it is the statement "this
design runs identically in either mode".

### The equivalence gate

> A design the mode does not refuse must produce byte-identical output in both modes.

If x and z never participate, the 4-state and 2-state computations agree. That makes the gate
a completeness check on the refusal list: refusal set and gate verify each other, and a
missing refusal shows up as a byte difference rather than as a silent wrong answer. The
structure already exists in `crates/sim-engine/tests/backend_equiv.rs` and would be reused
directly.

The same shape applies to a fusion- or level-scheduled mode: the hazard detector's
completeness gate is "zero candidates implies the two modes are byte-identical". Without a
gate of that shape, a declared mode is just a second engine with unmeasured differences.

---

## 6. The prerequisite measurements, and what they returned

The rule that produced these verdicts is that a payoff is measured before the machine that
depends on it is built. Each row states the pass bar it was given and what it returned.

| # | Measurement | Pass bar | Method | Result |
|---|---|---|---|---|
| M1 | the share of run cost the `unk` plane actually causes | ≥ 30% | a scratch patch pinning `unk` to zero, buying the ceiling directly; correctness breaks but the speed bound is exact; reverted afterwards | **7%** — fail |
| M2 | whether real designs hit the §5 refusal list | none, or few | count `===`, `casez`, `'x` and tri-state uses in the corpus designs | not reached; M1 closed the axis |
| M3 | whether halving the net table matters to cache | design-size dependent | net storage bytes | not reached; the measured design has 238 nets |
| M4 | the scheduling half — the extra evaluations a cycle mode pays to erase the event queue | `C_blk / E_act < 1.84` | activation census, pure counting, no correctness impact | **10.32** on picorv32 — fail; 1.65 on keccak, inside model error |

Both halves are closed by probes costing hours, against the multi-session build priced in §7.
That ordering is the standing rule for this axis, and it is why this document is a
specification rather than a mode.

---

## 7. Cost and risk, if a mode were ever built

| Stage | Size | Note |
|---|---|---|
| Prerequisite measurements | XS | scratch patch, reverted |
| A value representation without `unk` | L | `Value`, `BitPacked` and net storage are all involved |
| The §5 refusal set | M | the whole of the design's honesty |
| Mode wiring plus the equivalence gate | S | the `--backend` selection and gate structure are the precedent |
| Mode contract and limits documentation | S | |
| Total | ~4–6 sessions | |

The largest risk is that the value representation reaches everywhere: `Value` is present
throughout the engine. A separate 2-state type duplicates code; a runtime branch spends part
of the gain on the branch. The 7% ceiling does not fund either.

---

## 8. Open decisions, if the verdict is ever revisited

1. **Whether to build it at all.** A passing prerequisite measurement is necessary and not
   sufficient — a dual value representation carries a permanent maintenance cost after it
   lands.
2. **Refuse, or assign a value.** §5 recommends refusing, which is what correct-or-loud
   requires. Industry practice is to proceed quietly under an `--x-assign 0` style option.
   Refusing is the choice that matches this tool's identity.
3. **Whether 2-state and the executor selection stay orthogonal.** Keeping them orthogonal is
   recommended, since their effects multiply.
4. **Whether to go on to cycle-based scheduling.** The two halves are orthogonal (§1): the
   value half can be taken alone, at much lower risk. The scheduling half is separately
   rejected in §2, and levelization and fusion are separately closed in
   [preview/18 §10](18-acceleration-analysis.md), so nothing pulls the scheduling half along.

---

## 9. Related documents

- [preview/18 — acceleration paths](18-acceleration-analysis.md): the standing verdict on
  every acceleration, including levelization and fusion.
- [study/01 — the performance axis](../study/01-interpreted-vs-compiled.md): the two
  orthogonal axes, the value-representation census, and the cost of two planes per operation.
- [preview/01 — goals and scope](01-goals-and-scope.md): G1, the correctness goal a 2-state
  default would remove.
- [preview/06 — simulation engine](06-simulation-engine.md): the event regions and the delta
  settle a cycle mode would replace.
- [ROADMAP §5.a and §7](../ROADMAP.md): the standing verdict rows and their triggers.
