# round-19 residual families → correct-support — Design Spec

**Date:** 2026-07-24
**Base:** `5dd897b` (round19-families branch), 4262 green, format_version 23.
**Goal:** Bring the round-19 reviewer report's residual gaps to correct-support (no defer), no `format_version` bump, correct-or-loud throughout.

## Origin

External reviewer report round-19 (base v0.15). The round-18 8 families are already FIXED at `5dd897b`. Round-19 lists 7 residual "families" + `tb_sha3 m.name`. Fresh-probe triage at HEAD confirmed every repro AND **corrected two of the report's diagnoses** (see §2). iverilog 13.0 and verilator both REJECT all these constructs (`sorry: Overriding the default variable lifetime`, `Unpacked structs not supported`) → **no external oracle** → verification is hand-IEEE (§6.21 lifetime, §13.5.1/2 pass-by-value) cross-checked against the **passing boundaries** that already work.

## 1. Families (4 actionable + m.name subsumed)

| # | Family | Repro (minimal) | HEAD result |
|---|---|---|---|
| BL1 | const block-local under fork | `fork begin automatic int t=5_000_000; #(t*1ns); end join_none` | E3009 lifetime |
| BL2 | single-block dyn `'{}` init | `begin automatic string f[]='{"a","b"}; end` | E3009 lifetime |
| BL3 | same-name dyn `'{}` init ×N | two blocks each `automatic byte msg[]='{…}'` | E3009 ×2 |
| BL4 | block-local via output-actual | `automatic logic[4:0] m; setmode(3,m); if(m==3)…` | E3009 read-before-write |
| Q | param-width record whole-copy | `pkt_t cur; next=cur;` (member `[ADDR_W-1:0]`) | E3010 ×4 |
| F-struct | struct input formal, string member | `run_kat(kats[0])`, `kat_t{enum;string}` | E3009 missing `$unp$k$name` |
| F-record-out | output formal call in `&&`/`||` | `while(n<5 && rsp_next(n,r)==1)` | E3009 output formal illegal |

**m.name (tb_sha3:243/247)** — verified NOT a separate bug. Enum `.name()` in the exact 2-arg `$display`/if-else shape works with scalar formals. The tb_sha3 failure is the F-struct gap (run_test takes `kat_t` struct-with-string-member input formal → binding fails before `.name()` is reached). **Fixing F-struct resolves it.**

## 2. Report corrections (verified — do NOT implement the report's framing)

- **Q**: report proposed making struct member-width eval param-aware. **UNSOUND** — a parse-time frozen width is silent-wrong under `#(.ADDR_W())` override (verified `$bits(cur.addr)` = 8 vs 48 per-instance today; the parser deliberately keeps param widths out of the packed path, `structs.rs:726-731`). The record net, queue, `size()`, `pop_front()`, and field access **already work** via the per-member SoA representation. The ONLY gap is **whole-record scalar copy**.
- **F-record-out**: report claimed a "≥2 string members" boundary. **It does not exist** — both reporter files fail identically regardless of string count; the 2-string copy-out is value-correct. The real trigger is **call context**: an output-formal call in a short-circuit `&&`/`||` RHS. Not a records issue; a hoist issue.

## 3. Per-family design

### BL family — `crates/elaborate/src/block_local.rs` (+ da.rs, frames_body.rs)

The v1 model flattens each procedural block-local to ONE module net per bare name (`hoist_block_local_nets`). Correct when at most one activation is live AND the value is constant-across-entries or definitely-reassigned-before-read. §4.5.180/189/213-D added the loud gates + the `$blk$` distinct-net scoping (`compute_scoped_block_locals`) + per-entry scalar init (`compute_per_entry_block_locals` / `emit_per_entry_block_inits`).

- **BL1 — const-immune under-fork.** `compute_per_entry_block_locals` excludes `under_fork` blocks (concurrency: one net can't hold per-child storage). But an `automatic` block-local whose init **folds to a constant** and which is **never reassigned** is byte-identical to a static net holding that constant — concurrency-immune (every activation reads the same constant). Fix: in the two loud gates (`hoist_block_local_nets` scoped path ~line 372, non-scoped path ~line 570), add an exception: skip the loud when `n.init` folds via `const_eval_in_scope`/`fold_init` AND `name` is never written in `stmts` (new helper `stmt_never_assigns_ident` or reuse). Then it flattens to a static net with the folded constant. Verified: P5 (static form) and P6 (assigned-not-init form) already run; only the automatic+initializer form is loud.
  - Stays LOUD: non-const init under fork, or reassigned under fork (genuinely needs per-activation storage — module-process forks have no frame arena).

- **BL2 + BL3 — per-entry dyn-storage init at block entry (unified).** A dyn-array/queue/string block-local with a `'{}`/`new[]` init, not under fork. Mid-body `x='{…}'` **statement** is loud (verified P1), but the decl-init EXPANSION (`dyn_decl_init_stmts` → `d=new[N]; d[i]=e;`) is fully supported mid-body (verified P8/P9). Fix:
  1. Extend `compute_per_entry_block_locals` (or a parallel `compute_per_entry_dyn_block_locals`) to record dyn-storage `'{}`/`new[]`-init block-locals (not under fork). Currently it excludes strings/arrays (`netvar_kind_is_var` + `n.unpacked.is_empty()`).
  2. In `emit_per_entry_block_inits`, for a dyn-storage local emit `dyn_decl_init_stmts(name, kind, elems)` (new[N]+element writes) instead of the raw `x=init` Blocking. This re-inits on each block entry.
  3. Skip the `automatic`+init loud gate for these (per-entry).
  4. Skip the t0 pre-sweep push for these (per_entry already skips it) — else double-init.
  5. **BL3 same-name**: relax the coalesce guard (`hoist_block_local_nets` ~line 430, "dynamic-storage local … in another block is unsupported") to ALLOW same-name dyn coalesce when both blocks are per-entry-dyn. Shared net; block 2 re-allocs on entry (verified P9: new[2]→new[3] shared net is correct). Boundary preserved: a same-name dyn coalesce where the later block does NOT definitely re-init before read stays loud (leak hazard).
  - Stays LOUD: under-fork dyn (concurrency); assoc; multi-dim; non-`'{}`/`new[]` dyn init.

- **BL4 — signature-aware definite-assignment.** `automatic_local_definitely_assigned` (da.rs) treats any reference to `name` as a read → `f(…, name)` where `name` is an OUTPUT actual is misread as read-before-write → loud. Fix: thread a callee-signature resolver into the DA walk so a statement whose ONLY reference to `name` is an output/inout actual (and no input-position read) counts as a definite ASSIGNMENT of `name`. The Elaborator has port directions (`func_table`/tf_ports). Verified P7: direct assign works; only the output-call path is the gap.
  - Stays LOUD: genuine read-before-write; `name` used as both an input read and output in the same call.

### Q — `crates/hdl-parser/src/soa.rs` (parser, no format bump)

A scalar non-packable record (`var_unpacked_struct`) has per-member nets `$unp$v$field` but **no whole-value copy**. `try_soa_assign` (`soa.rs:189`) handles pop-into-record (`soa.rs:201-252`) and array-copy (`soa.rs:296-315`, gated on `record_soa_vars` = arrays only) but not scalar record `a=b`. Fix: add a branch — `a=b` / `a<=b` where both are scalar `var_unpacked_struct` of the **SAME type name** → emit a `Block` of per-member `$unp$a$field <op> $unp$b$field` (thread `blocking` flag). Both `=` and `<=` already dispatch through `try_soa_assign` (`stmt.rs:301/323`). Covers Q's module net + block-local, and generalizes to literal-width mixed-state / string-member scalar records.
  - Correctness gate: SAME type name (identical raw member ranges resolved in the same module → identical per-instance width/sign by construction). All-or-loud (never partial fan-out). By-value preserved (independent member nets).
  - Stays LOUD: cross-type whole copy.

### F-struct — `crates/hdl-parser/src/structs.rs::expand_struct_call_args` (parser, no format bump)

A non-packable struct tf-port decomposes into per-member formals `$unp$k$mode`, `$unp$k$name` (`typedefs.rs:818`, `structs.rs:228-264`). The call-site actual expander `expand_struct_call_args` (`structs.rs:275-308`) only matches a bare `Ident` naming a `var_unpacked_struct` — `kats[0]` is a `BitSelect` → falls through unexpanded → 1 actual for N formals → `missing actual for $unp$k$name`. Fix: extend `expand_struct_call_args` to also match `BitSelect{base:Ident(arr), index}` (and `IndexedPart`) where `arr ∈ record_soa_vars`, expanding to per-member element actuals `$unp$arr$field_k[index]` (mirror `parse_record_array_member` / `soa_member_field`). N element actuals line up with N per-member formals. Guard: only record SoA arrays (leave packable `record_array_vars` — whole-vector element already works).
  - Deep-copy: each member is its own scalar/string formal → the existing per-member string-formal copy-in deep-copies (IEEE §13.5.1). Adversarial test: caller mutates `kats[0].name` while callee holds `k.name` across a suspend.
  - Resolves m.name.
  - Stays LOUD: whatever `expand_struct_call_args` can't line up (arity still checked in `fill_default_args`).

### F-record-out — `crates/elaborate/src/hoist.rs` + `stmt_flow.rs` (elaborate, no format bump)

`emit_frame_call` (`frames_call.rs:218-228`) rejects any function with a non-Input formal on the plain-expression path. Reached because `hoist_inout_calls` (`hoist.rs:276-280`) deliberately leaves a `&&`/`||` RHS inout-call un-hoisted (must not make a conditional call unconditional). The copy-out synthesis itself (`emit_frame_func_out_call`, `frames_call.rs:735-865`) already works and is value-correct for ≥2 strings on direct/plain-condition paths. Fix (the deep one): lower an output-formal call in a short-circuit position by emitting the copy-out `Terminator::Call` **inside the short-circuit branch** — a guarded conditional (`if (left) { r = f_with_copyout(…); rhs = … }`), not a flat pre-statement hoist. Concretely, in `lower_while`/`lower_for` head lowering (and the general expression lowering for `&&`/`||`), split the condition at the short-circuit operator and emit the inout-call as a statement-context `Call` in the taken branch.
  - Also: make the blanket `frames_call.rs:225` message more specific (it currently reads as if output formals are never supported, which misled the reporter).
  - Adversarial verification hardest here: short-circuit semantics (call must NOT fire when the guard is false; copy-out must NOT write `r` then), value-correctness of the loop, ≥2-string copy-out.
  - Stays LOUD (unless included): `?:` arm output-formal call (scope decision — include if tractable within the same mechanism).

## 4. Format invariance

No `format_version` bump. Q + F-struct are parser-desugar (`$unp$`/SoA machinery — parser-only, `.vu` AST hash only, per the "struct member desugar is parser-only" memory). BL is elaborate-transient (`per_entry_block_locals`/`scoped_block_locals` maps, IR-0 synthesized statements). F-record-out is elaborate CFG lowering + reuses existing `TaskCallInfo` copy-out. SimIr golden root unchanged.

## 5. Verification (no external oracle)

Every family has a passing boundary as its reference behavior:
- BL1: static form (P5) / assigned-not-init form (P6).
- BL2/BL3: static single-block dyn `'{}` (P2), mid-body new[]+writes (P8/P9).
- BL4: direct assign (P7).
- Q: field-wise copy (verified PASS); packable whole-vector copy.
- F-struct: packable struct actual (`run_kat(kd[0])` with `{int;int}` works).
- F-record-out: direct-rhs / plain-condition output-formal call (value-correct, ≥2 strings).

Adversarial hand-IEEE tests per family: pass-by-value/aliasing (string members deep-copy), same-name sequential isolation, under-fork concurrency stays loud, cross-type copy stays loud, short-circuit non-evaluation.

## 6. Staging

Feature branch `round19-families`. Subagent-driven: one implementer + task-review per task, whole-branch review at end.
- Task 1: BL1 const-immune under-fork.
- Task 2: BL2+BL3 per-entry dyn-storage block-entry init (+ same-name coalesce relax).
- Task 3: BL4 signature-aware definite-assignment.
- Task 4: Q whole-record scalar copy (parser).
- Task 5: F-struct SoA-element actual expansion (+ verify m.name resolved).
- Task 6: F-record-out short-circuit conditional-call hoist (deep).

Gates each task: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`, format_version stays 23. Then docs (ROADMAP/ARCHIVE/CLAUDE.md/memory) + push + final report.
