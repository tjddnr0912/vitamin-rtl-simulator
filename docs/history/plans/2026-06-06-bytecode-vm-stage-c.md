# Stage C — Bytecode VM Compiled Backend: Spec & Implementation Plan

> **Status:** Review-incorporated (2026-06-06). Adversarial review `w7a0hlp1z` (4 grounded
> lenses → verdict **REVISE**) applied: 6 must-fix + 10 should-fix folded in. Stage B
> (11/11 prerequisites) complete; this is the `구현` design the user's sequence calls for,
> written against the seams Stage B installed. Substrate: **bytecode VM (P0a)**, in-process,
> no runtime toolchain (P0b = N/A) — see `docs/preview/18-acceleration-analysis.md`.
>
> **IMPLEMENTATION PROGRESS:** **C1 ✅ + C2 ✅ (2026-06-06) — MVP terminus reached.** The VM
> now actually executes the codegen-able (P9) class byte-identically to the interpreter
> (P5 gate live over 72 corpus designs + mixed P9b + new prologue/runaway/blocking-index
> teeth); 439 tests green, clippy/fmt clean, golden `SimIr` root unflipped. **Next = C3**
> (native value registers — the first phase that can be *faster*).

**Goal:** Replace `Scheduler::vm_run_body`'s interpreter-delegation with a real bytecode
compiler + register VM that executes a codegen-able (P9 suspend-free) process body,
calling the SAME kernel through the `Kernel` trait — provably byte-identical to the
interpreter (the P5 gate) and faster on the suspend-free class once native eval lands.

**Non-goals (stay interpreted, by design):** suspend-bearing bodies (Delay/Wait/Fork/
Call terminators), continuous assigns (until C8), the `$display`/`$strobe`/`$monitor`
format engine (C7 — a documented codegen *boundary*, not a target), and Windows.
**In-scope but subtle:** a codegen-able body may still contain `Expr::Call` in its expr
arena (only `Terminator::Call` is excluded by P9); the interpreter's `eval` treats
`Expr::Call` as a defensive 1-bit `X` (eval.rs:173) and the VM MUST reproduce that exact
arm verbatim (so a future elaborate emitting `Expr::Call` cannot silently misbehave on
the VM path).

---

## 1. What Stage B already gives us (the substrate)

| Seam | Where (verified) | Stage C uses it for |
|---|---|---|
| `Backend{Interpreter,Bytecode}` + `SimOpts.backend` | lib.rs | selecting the VM; P5 runs both |
| `run_body` dispatch + `vm_run_body` delegation | sched.rs:366 | THE entry point to fill in |
| `is_codegen_able(stmts, body)` (P9) | backend.rs:33 | which bodies the VM may claim |
| `Kernel` trait, 5 methods (P7b) | exec.rs:40-51 | the body→kernel ABI: `k_eval_for_lvalue(&Lvalue,u32)->Value`, `k_resolve_lvalue_offsets(&Lvalue)->Vec<(u32,u32)>`, `k_write_lvalue(&Lvalue,Value,&[(u32,u32)])`, `k_schedule_nba(Lvalue,Value)`, `k_dispatch_systask(SysTaskId,Option<u32>,&[u32])->Ctl` |
| `StmtEffect` read/write split (P7a) | exec.rs:160 | the per-statement shape the VM mirrors |
| P5 gate + P6 corpus | tests/ | byte-identity proof, every phase |
| P8 sampling-moment contract | doc-18 §P8 | order the VM must preserve (only #2/#3 are body-side) |
| P3 float contract | doc-18 §P3 | VM reuses the frozen formatters verbatim |

**Critical inheritance:** the VM does NOT reimplement net I/O, VCD, scheduling, NBA, or
formatting — it calls the kernel for all of those (via `Kernel`), so determinism, VCD
bytes, and float output reproduce *by construction*. The VM's only new code is **control
flow + (eventually) native expression eval**. **But two pieces of body-execution state the
interpreter maintains in `run_process` are NOT in the kernel and MUST be reproduced by the
VM itself from C2 (review must-fix #1/#2):** `st.cur_time_mult` (set per activation at
exec.rs:80-87; the ONLY writer) and the per-activation delta guard (exec.rs:57/176-180).

---

## 2. VM architecture

A **register bytecode VM** (not tree-walk, not native codegen): removes the two measured
hot spots (eval tree-walk dispatch, `Value` heap-alloc) while staying pure Rust with zero
toolchain dependency (preserves all determinism pins). **Honest expectation:** the *win*
is C3+ (native value registers). C2 — compiled control flow with eval still delegated to
the kernel — is a *structural* milestone, expected **at-or-below interpreter speed**
(it adds a compile pass + op-dispatch loop on top of unchanged eval cost). Small bodies
stay interpreted so C2 overhead is never paid where it can't pay back.

### 2.1 Compiled artifact (per codegen-able **template**, not per activity)

```
struct CompiledBody {
    blocks: Vec<CompiledBlock>, // 1:1 with the frozen Process.body BBs (SAME indices, for P16)
    lvalues: Vec<Lvalue>,       // cloned LHS side table (LvalIdx -> &lvalues[idx])
    arglists: Vec<Vec<u32>>,    // cloned SysTask arg ExprId lists (ArgsIdx)
    nregs: u32,
}
struct CompiledBlock { ops: Vec<Op>, term: CompiledTerm }
enum CompiledTerm { Goto(u32), Branch { cond: ExprId, then_bb: u32, else_bb: u32 }, Return }
```

`CompiledTerm` covers ONLY the P9 allow-list — `is_codegen_able` guaranteed nothing else.
Block indices match the frozen IR's (debugger mapping, P16). **`Branch` carries the
condition `ExprId`, not a register** — truthiness is a tri-valued control-flow rule, not a
register boolean (see §2.4).

### 2.2 Opcode set (MVP — C2 ops delegate eval to the kernel)

`Op` stays `Copy`-small; `Lvalue`/arg vectors live in side tables, referenced by index:

```
enum Op {
    // C2: delegate eval to the SAME kernel the interpreter uses (no native eval yet)
    EvalForLval { dst: Reg, lhs: LvalIdx, rhs: ExprId }, // dst = k_eval_for_lvalue(&lvalues[lhs], rhs)
    ResolveOff  { dst: OffReg, lhs: LvalIdx },           // dst = k_resolve_lvalue_offsets(&lvalues[lhs])
    // write phase: direct Kernel calls (resolve LvalIdx -> &lvalues[idx] at the call)
    WriteLval   { lhs: LvalIdx, val: Reg, off: OffReg }, // k_write_lvalue(&lvalues[lhs], take(val), &off)
    ScheduleNba { lhs: LvalIdx, val: Reg },              // k_schedule_nba(lvalues[lhs].clone(), take(val))
    SysTask     { which: SysTaskId, fmt: Option<ExprId>, args: ArgsIdx }, // k_dispatch_systask(.., &arglists[args])
}
```

Per the P8 contract, **a blocking assign emits `ResolveOff` immediately before its
`WriteLval`** so the dynamic index is sampled at statement time (moment #3); statements
lower in textual order so `ScheduleNba` calls preserve `nba_seq` (moment #2). The register
file is `RegFile = Vec<Option<Value>>` (Value has no `Default`/`take`; the `Option` lets
`WriteLval`/`ScheduleNba` `mem::take` the produced value without a clone) — `OffReg` holds
the `Vec<(u32,u32)>`. The interpreter's `compute_effect`/`apply_effect` already call exactly
these `k_*` methods, so C2 reuses the kernel verbatim and introduces zero new value logic.

### 2.3 Execution + borrow protocol + cache

The cache and a scratch RegFile are owned **out-of-band on `SimState`** (never enter the
frozen `SimIr`): `vm_cache: Vec<Option<CompiledBody>>` indexed by **template**
(`activity_template(proc)`, length `ir.processes.len()`) — so fork children that share a
template share one `CompiledBody` (they differ only by entry BB), and the O(blocks)
`is_codegen_able` scan is **decided once** (the slot's presence == codegen-able).

**Borrow protocol (review must-fix #4):** `Scheduler` is `&mut SimState`, and
`impl Kernel for Scheduler` makes the kernel `&mut Scheduler`. We therefore CANNOT hold
`&self.cache[tmpl]` while passing `&mut self` as the kernel. So: in `vm_run_body`,
`Rc::clone` the cached `Rc<CompiledBody>` out (or `mem::take` + restore) BEFORE calling
`vm_exec`, then pass the owned `Rc` alongside `&mut sched`. `vm_exec` borrows nothing from
the scheduler except through the `Kernel` trait.

```
fn vm_run_body(&mut self, proc: u32, block: u32) -> Step {
    let tmpl = self.activity_template(proc) as usize;
    let body = self.st.vm_cache_get_or_compile(tmpl);     // Rc<CompiledBody>, compiled once
    // PROLOGUE (must-fix #1): the VM bypasses run_process — the ONLY cur_time_mult writer.
    self.st.cur_time_mult =
        self.st.proc_multipliers.get(tmpl).copied().unwrap_or(1).max(1) as u64;
    let mut regs: Vec<Option<Value>> = vec![None; body.nregs as usize];
    vm_exec(self, &body, &mut regs, proc, block)
}

fn vm_exec(k: &mut impl Kernel, body: &CompiledBody, regs: &mut RegFile,
           proc: u32, mut bb: u32) -> Step {
    let mut guard: u64 = 0;                                 // must-fix #2: termination guard
    loop {
        for op in &body.blocks[bb as usize].ops { /* EvalForLval/ResolveOff/WriteLval/
            ScheduleNba/SysTask -> k_* ; SysTask Finish/Stop/Fatal -> return */ }
        match body.blocks[bb as usize].term {
            Goto(t)              => bb = t,
            Branch{cond,th,el}   => bb = if k.k_truthy(cond) { th } else { el }, // §2.4
            Return               => { k.k_rearm(proc); return Step::Done }       // must-fix #6
        }
        guard += 1;
        if guard > k.k_max_deltas() { k.k_mark_fatal(); return Step::Fatal } // mirror exec.rs:176-180
    }
}
```

### 2.4 Truthiness contract (review must-fix #3)

Branch truthiness is a **tri-valued control-flow rule on `EvalCtx`** (eval.rs:179-183),
NOT a register boolean: `X`/`Z` → **false** (`if(x)` takes the `else`). The VM routes it
through a new `k_truthy(eid) -> bool` (C1) that forwards VERBATIM to `Scheduler::truthy`
(builds `EvalCtx` with the now-correct `cur_time_mult`, calls `EvalCtx::truthy`) — exactly
what `Terminator::Branch` uses today (exec.rs:120). There is no `Value::truthy()` and no
`-> bool-in-reg` opcode; `Branch` reads `k_truthy(cond)` directly.

---

## 3. Phased implementation

Every phase gated by **the full `cargo test --workspace --locked` suite green, zero new
failures** (NOT a frozen test-count literal — review should-fix), plus the P5 differential
gate. The `Kernel` trait grows by exactly two FORWARDING methods in C1; nothing else in
Stage B changes; ZERO frozen-IR change in any phase.

| Phase | Maps to | Deliverable | Gate |
|---|---|---|---|
| **C1** ✅ Terminator-phase ABI | P7b follow-on | DONE 2026-06-06. Added `k_truthy(&self,eid)->bool` (→ `Scheduler::truthy`) and `k_rearm(&mut self,proc:u32)` (→ `rearm(proc)`, preserving the Edge/Level/Initial asymmetry — NOT reimplemented), plus `k_max_deltas`/`k_mark_fatal` forwarders (exec.rs `trait Kernel` + sched.rs impl). **Invariant stated:** a codegen-able template can never be entered as a fork child (its body has no `Fork` terminator, and `is_codegen_able` scans the whole body), so `k_rearm` is total. | suite green (landed with C2, its first caller) |
| **C2** ✅ Structural VM (delegating eval) | scaffolding | DONE 2026-06-06 (MVP terminus). `CompiledBody`/`Op`/`compile_body` (1:1 BB lowering) + out-of-band template-keyed `vm_cache` (`VmSlot`/`SimState::vm_compiled`, decide-once, `Rc` owned-handle borrow protocol) + `vm_exec` with the **cur_time_mult prologue (#1)** in `vm_run_body` and **delta guard (#2)** in the exec loop. **Compile-pass unit tests (2):** `blocks.len()==body.len()` + terminator 1:1 (P16) and NBA/systask op shapes (independent of P5). **New teeth tests (3):** `$time` under distinct non-unit multipliers (prologue, adversarially confirmed to fail with a broken prologue), a `forever` runaway (guard → Fatal), and `a[i]=K;i=i+1` (P8 #3 blocking-index sample). | **P5 byte-identical** ✅ over corpus(72) + mixed (P9b) + prologue + runaway + blocking-index; 439 green |
| **C3** Native value registers | P12a | ⏳ Perf harness DONE (`tests/perf_baseline.rs`, `#[ignore]`d). **MEASURED FINDING (2026-06-06, reshapes C3):** an A/B of inline-≤128 `Value` storage vs the `Vec<u64>` heap rep showed **~0 net change** (eval-heavy best-of-5: 2802 vs 2781 ms interp; 2714 vs 2699 ms VM — within noise) → **`Value` heap-alloc is NOT the bottleneck** (doc-18's premise contradicted by measurement; tiny short-lived `Vec<u64>` are cheap). The inline experiment was REVERTED (no benefit = pure complexity, YAGNI). Also measured: **the C2 VM is already 0.97x interp on eval-heavy** — it pre-compiles the body and avoids `run_process`'s per-block `stmt_ids/term/stmt` **clones** (exec.rs:88-104). **PROFILED (2026-06-06, `/usr/bin/sample`, eval-heavy, debuginfo release).** Self-time histogram **upends the whole C3 premise**: the **net-write path dominates ~50%** — `write_lvalue` 5707 + `set_vu` 3970 + `slice_word` 1392 + `resize`/`mask_top` 844 — and it is **BIT-SERIAL** (`write_lvalue`/`slice_word`/`write_chunk` loop one bit at a time via `get_vu`/`set_vu`/`bit_of`/`set_bit`, plus a `Value::zeros` alloc per chunk). malloc/free ≈ ~17%. **The eval tree-walk dispatch (`eval_ctx`) is only ~1.5%** and all eval value-ops ~15%. ⇒ **native eval (C3-C6) would chase ~1.5–15% with the project's highest-risk reimplementation; the real ~50% win is WORD-PARALLEL net read/write** (state.rs `write_lvalue`/`write_chunk`/`slice_word` + `Value::set_vu`/`get_vu`/`bit_of`/`set_bit`) — contained, lower-risk, and it helps BOTH backends (shared net path, the VM can't out-run it via eval). **PIVOT → word-parallel net write. ✅ DONE (2026-06-06):** word-ized the three bit-serial hot paths (state.rs `slice_word` read, `write_lvalue` single-chunk piece-skip, `write_chunk` whole-element store) — copy/mask whole u64 words instead of per-bit `get_vu`/`set_vu`/`bit_of`/`set_bit`; bit-serial kept as fallback for part-select / unaligned / OOR. Guard `array_len ≤ 1 \|\| net_w % 64 == 0` so a masked top-word write can't clobber a neighbour element. **Result: ~2x on BOTH backends** (eval-heavy interp 2781→1274 ms = 2.18x; codegen-heavy 196→99 ms = 1.98x). **Then re-profiled → next hot spot = bit-serial value ops (`set_vu` #1, fed by `resize`+shift loops); word-ized `Value::resize` + `shr_fill`/`shl_grow` (multi-word shifts, `shift_word_vs_bit_parity` locks bit-exactness): eval-heavy 1274→948 ms.** **Then re-profiled again → with the bit-serial paths gone, `Value` heap alloc was now the single dominant cost (~25-30%). Retried inline-`Value` (`Words` enum: `Inline{[u64;2],len}` ≤128 alloc-free, `Heap(Vec)` >128) — THIS TIME a clear win** (the same change measured ~0 earlier; the net-write/shift per-bit `set_vu` loops that masked the alloc + hammered the per-access Deref are now word-parallel). A/B same-machine: eval-heavy 959→618 ms (1.55x), VM 864→537 (0.87x interp). **4th re-profile → residual hot spots: resize/`mask_top` normalization + a transient `BitPacked` alloc per net read. Guarded `mask_top`'s resize on length (skip the no-op) + made `read_net` build the inline `Value` directly from the store (dropped `net_word_packed`→`slice_word`→`BitPacked`→`from_packed`, two Vec allocs/read).** eval-heavy 618→461 ms. **Cumulative from C2 baseline: eval-heavy 2781→461 = ~6.0x, codegen-heavy 196→61 = ~3.2x, VM eval-heavy 2699→385 (0.84x interp).** 441 green + iverilog differential 11 bit-exact. Remaining (lower ROI): select/concat word化, `eval_ctx`/`eval_binary_ctx` dispatch (~native-eval territory, still small). | ✅ P5 + differential bit-exact, **~6.0x** measured |
| **C4** Static width/sign + poison | P10 | Precompute (width,signed,poison-regime) per (ExprId,context). **Asymmetric poison contract (oracle eval.rs:439-444, value.rs:193):** UNSIGNED 128-bit lane (poison `X` only at width>128), SIGNED 64-bit lane (poison `X` at width>64). A uniform threshold diverges. | P5 + instrumented-eval cross-check |
| **C5** Index classification | P11 | Static vs runtime index/width/count; reproduce `const_u32_of_expr` **SHALLOW** fold + each site's EXACT fallback + OOR sentinel arms (read & write). Do NOT "improve" the fold. | P5 |
| **C6** Real lane | P12b | f64 register lane (is_real, NaN/±inf/±0.0/partial_cmp), reusing P3-frozen formatters. | P5 + float golden |
| **C7** Prologue nuance + format boundary | P12c/P12d | The **stale-during-postponed** `cur_time_mult` nuance (flush_postponed renders `$strobe`/`$monitor` with the last process's multiplier, sched.rs:570) — the simple per-activation prologue is already in C2; C7 handles the postponed subtlety. Format engine stays interpreted (explicit boundary). | P5 |
| **C8** Cont-assign codegen | P13 | **Scheduler-surface change, not a body phase:** `settle_cont_assigns` is a scheduler loop over `cont_assigns` in DECLARATION order (sched.rs:190-237), not `run_body`. Preserve decl-order fixpoint + delayed `assign #d` `last_ca` keying. Not a correctness prereq (cont-assigns interpret fine) — a speedup for cont-assign-heavy designs. | P5 + P9b |
| **C9** Perf + cache + debug | P14/P15/P16 | Speedup baseline + **threshold gate** (the perf harness from C3 promoted); content-addressed codegen cache key + a **NEW** `kernel_abi_version` header field (does NOT exist yet — vita-artifact has only `format_version` + `composite_input_hash`, header.rs:50-52) gated independently of `format_version`; ExprId→SourceLoc sidecar (P16). | P14 threshold met |

**MVP terminus = C2:** the VM actually *executes* bodies (compiled control flow, cached
compile, correct cur_time_mult + termination), byte-identical to the interpreter — but
expected at-or-below its speed. **C3 is the first phase that can be faster.**

---

## 4. Risks & mitigations

| Risk | Mitigation |
|---|---|
| cur_time_mult / delta-guard leak (the two run_process-only states) | reproduced in C2's vm_exec prologue + guard, byte-mirroring exec.rs:80-87/176-180; timescale + runaway corpus cases give P5 teeth |
| Runaway codegen-able loop hangs (P5 can't diff a non-terminating run) | per-activation guard returns `Step::Fatal` exactly like the interpreter; runaway corpus case asserts identical DeltaLimit/Fatal |
| Compiled body reorders kernel calls → nba_seq divergence (P8 #2) | vm_exec runs ops in compile order = statement order; ResolveOff-before-WriteLval enforces #3; P9b + a new `a[i]=x;i=i+1` blocking-index test have teeth |
| Native value rep drifts from `Value` bit-for-bit | C3/C4 reuse value.rs ops as the oracle; asymmetric-poison contract pinned; P5 over wide-arith corpus |
| const-fold "improvement" computes a wider width than the shallow interpreter fold | C5 reuses `const_u32_of_expr` verbatim + each site's fallback |
| `Expr::Call` on a codegen-able body | VM reproduces eval's defensive 1-bit-`X` arm (eval.rs:173) verbatim; listed in non-goals |
| C2 overhead paid on small bodies with no payback | small-body guard keeps them interpreted; per-template decide-once cache removes the per-fire predicate scan |
| Borrow infeasibility (cache + &mut kernel) | out-of-band `Rc<CompiledBody>` cloned out before vm_exec (§2.3) |
| Frozen IR pressure | ZERO IR change in any phase; SchemaHash root pinned (existing schema_hash test); `format_version` 3; `kernel_abi_version` is a separate NEW gate |

---

## 5. Acceptance for "Stage C done"

1. `Backend::Bytecode` executes the P9 class on the VM; everything else interprets.
2. P5 gate byte-identical (stdout + VCD + summary) over the full P6 corpus + P9b mixed +
   the new timescale/runaway cases, on all CI legs, no skip.
3. P14 speedup threshold met on the suspend-free class vs the interpreter baseline.
4. `schema_hash::<SimIr>()` root unflipped; `format_version` still 3; a NEW
   `kernel_abi_version` (added in C9) gates the VM artifact independently.
5. Docs: doc-18 → SHIPPED; the format-engine boundary (C7), the interpreter-only
   constructs (cont-assign pre-C8, suspend-bearing), and the `Expr::Call` 1-bit-X arm
   explicitly stated.
