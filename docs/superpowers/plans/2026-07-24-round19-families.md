# round-19 residual families — Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development. Design spec: `docs/superpowers/specs/2026-07-24-round19-families-design.md`.

**Goal:** Round-19 report's 4 residual families (BL1-4, Q, F-struct, F-record-out) → correct-support, correct-or-loud, no `format_version` bump.

**Architecture:** BL = block-local gate relaxations in `crates/elaborate/src/block_local.rs`/`da.rs`/`frames_body.rs`. Q + F-struct = parser `$unp$`/SoA desugar (`crates/hdl-parser/src/soa.rs`/`structs.rs`). F-record-out = elaborate short-circuit hoist (`crates/elaborate/src/hoist.rs`/`stmt_flow.rs`).

## Global Constraints

- **No `format_version` bump** (stays 23). All changes are parser-desugar (`.vu` AST hash only) or elaborate-transient/IR-0/engine-side. If any change flips the SimIr golden root, STOP — the approach is wrong.
- **Correct-or-loud**: every unsupported form must be a loud E-code, never silent-wrong. No external oracle (iverilog/verilator reject these) → verify hand-IEEE (§6.21, §13.5.1/2) against the passing boundaries named per task.
- Gates each task: `cargo test --workspace --locked` green; `cargo clippy --workspace --all-targets --locked -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- Files ≤1000 lines (split if a file nears it). SchemaHash/frozen types must not move to submodules.
- Commit trailers: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` + `Claude-Session: https://claude.ai/code/session_011teDGcVPTUGkX5ByoG35HV`.

---

## Task 1: BL1 — const-immune block-local under a fork

**Files:** Modify `crates/elaborate/src/block_local.rs` (the two loud gates in `hoist_block_local_nets`: scoped path ~line 372, non-scoped path ~line 570). Maybe add a helper `stmt_never_assigns_ident` in `da.rs` or reuse existing write-scan. Test: `crates/cli/tests/block_local_const_fork.rs` (new).

**Approach:** An `automatic` block-local whose initializer folds to a CONSTANT (`self.const_eval_in_scope(init).is_some()` or `fold_init(init, w).is_some()`) AND which is never written in the block's `stmts` is byte-identical to a static net holding that constant — concurrency-immune. In BOTH loud gates, before firing E3009, add: `if init_folds_const && !stmt_writes_name { skip loud (let it flatten to the const net); }`. The net's `.init` already carries the folded constant via `elaborate_netvar_decl`. Do NOT mark per-entry (a constant needs no re-init).

**Test cases (TDD — write failing first):**
- `const_fork_watchdog_now_runs`: the BL1 repro (`fork begin automatic int unsigned timeout_ns=5_000_000; #(timeout_ns*1ns); $display("late"); end join_none #1 $finish`) → elaborates + simulates, `$finish` at time 1 (watchdog does not fire — correct). Assert no E3009, sim ends.
- `const_fork_value_used`: `fork begin automatic int W=8; logic[W-1:0] x; x=3; if(x==3)$display("PASS"); end join_none #1 $finish` → PASS (const used in a width/value).
- Regression `nonconst_fork_stays_loud`: `fork begin automatic int x=some_net; ... end join_none` with `some_net` a module net → E3009 (non-const init under fork).
- Regression `reassigned_const_fork_stays_loud`: `fork begin automatic int c=0; c=c+1; ... end join_none` → E3009 (reassigned).
- Regression: the existing non-fork const-init automatic (§4.5.213-D) still works.

**Gate + commit.**

---

## Task 2: BL2 + BL3 — per-entry dyn-storage block-local init at block entry

**Files:** Modify `crates/elaborate/src/block_local.rs` (`compute_per_entry_block_locals` to include dyn; the automatic loud gate to skip per-entry-dyn; the coalesce guard ~line 430 to allow per-entry-dyn same-name; the t0 push to skip per-entry-dyn). Modify `crates/elaborate/src/frames_body.rs` (`emit_per_entry_block_inits` to emit `dyn_decl_init_stmts` for dyn locals). Reuse `crates/elaborate/src/dynarr.rs::dyn_decl_init_stmts`. Test: `crates/cli/tests/block_local_dyn_init.rs` (new).

**Approach:**
1. Record dyn-storage `'{}`/`new[]`-init block-locals (not under fork) as per-entry (extend the map or add a parallel `per_entry_dyn_block_locals`). Kind = DynArray/Queue (+ string dyn-array). Must have `'{}` (`AssignPattern`) init and a single `Dim::Dyn`/`Dim::Queue` unpacked dim.
2. `emit_per_entry_block_inits`: for a per-entry-dyn local, emit `self.dyn_decl_init_stmts(name, kind, elems)` (which produces `d=new[N]; d[i]=e;`) and lower each — instead of the raw `x=init` Blocking (which is loud for dyn).
3. Skip the `automatic`+init loud gate (line ~570 and ~372) for per-entry-dyn.
4. Skip the t0 pending-var-init push for per-entry-dyn (the per_entry check already skips; ensure dyn is covered — currently the `scalar_string`/dyn push at ~594-654 must not double-fire).
5. Relax the coalesce guard (~line 430): allow a same-name dyn coalesce when THIS block's decl is per-entry-dyn (it re-allocs on entry → no leak). Keep loud when the later same-name block does NOT re-init before read.

**Test cases:**
- `single_block_string_dyn_init` (BL2): `begin automatic string files[]='{"a.rsp","b.rsp"}; if(files.size()==2)$display("PASS"); end` → PASS.
- `single_block_byte_dyn_init` (BL2): `begin automatic byte msg[]='{8'h0,8'h1}; if(msg.size()==2)$display("PASS"); end` → PASS.
- `same_name_dyn_init_two_blocks` (BL3): the BL3 repro (`begin automatic byte msg[]='{0,1}; …"A" end begin automatic byte msg[]='{0,1,2}; …"PASS" end`) → prints A then PASS (each block's size correct).
- `same_name_dyn_three_scenarios`: 3 blocks with different-size `msg[]='{…}'`, each reads its own size — models tb_hash_top ×49.
- `dyn_init_in_loop_reinits`: `for(i=0;i<2;i++) begin automatic byte m[]='{i,i}; ... end` — re-inits each iteration (per-entry semantics).
- Adversarial `dyn_init_by_value`: block A `msg[]='{…}'` then a task reads a snapshot — mutation isolation.
- Regression `under_fork_dyn_stays_loud`: `fork begin automatic byte m[]='{0,1}; end join_none` → E3009 (under-fork dyn concurrency).
- Regression `same_name_dyn_no_reinit_stays_loud`: two blocks share `msg[]` where block 2 READS `msg` before re-init → stays loud (leak hazard).
- Regression: `ok_bl_samename_new` (new[] same-name) still passes; static single-block dyn `'{}` (P2) still passes.

**Gate + commit.**

---

## Task 3: BL4 — signature-aware definite-assignment (output-actual = write)

**Files:** Modify `crates/elaborate/src/da.rs` (`automatic_local_definitely_assigned` + `da_stmt` to accept an output-actual-write resolver). Modify the caller in `block_local.rs` to pass the resolver. Test: `crates/cli/tests/block_local_output_actual.rs` (new).

**Approach:** Thread a signature-aware predicate into the DA walk: for a statement (or expression) containing a call `f(…, name, …)` where `name` is at an OUTPUT/INOUT actual position of `f` (per `func_table`/tf_port directions) AND `name` appears at no input-read position in that statement, treat the statement as a definite ASSIGNMENT of `name`. The predicate needs callee port directions — pass a closure `&dyn Fn(&ast::Expr /*call*/, &str) -> OutActualKind` (or precompute the set of statements that output-write `name`). Keep the existing conservative behavior for everything else. Handles `if (!setmode(3, cavp_mode))` (call in the If cond writes cavp_mode via output before any read).

**Test cases:**
- `output_actual_da` (BL4): the BL4 repro (`automatic logic[4:0] cavp_mode; if(!setmode(3,cavp_mode))$display("fail"); if(cavp_mode==3)$display("PASS");`) → PASS.
- `output_actual_stmt_position`: `automatic int r; getval(r); if(r==42)$display("PASS");` (call in statement position) → PASS.
- `inout_actual_da`: inout actual counts as assignment.
- Regression `input_actual_stays_loud`: `automatic int x; f(x); if(x==…)` where `x` is an INPUT actual (read-before-write) → stays loud E3009.
- Regression `mixed_read_and_output_stays_loud`: `automatic int x; g(x, x)` where x is both input and output in the same call → stays loud (conservative).
- Regression: existing block-local DA cases unaffected (byte-identical).

**Gate + commit.**

---

## Task 4: Q — whole-record scalar copy (parser)

**Files:** Modify `crates/hdl-parser/src/soa.rs` (`try_soa_assign` ~line 189, add scalar-record branch; mirror pop path ~201-252 and array-copy ~296-315). Test: extend `crates/cli/tests/record_array_soa.rs`.

**Approach:** In `try_soa_assign`, add a branch: LHS and RHS are both scalar `var_unpacked_struct` of the SAME type name → emit a `Block` of per-member assigns `$unp$lhs$field <op> $unp$rhs$field` (one per member, thread the `blocking` flag). All-or-loud (emit all members or fall through). Both `=` and `<=` dispatch here already (`stmt.rs:301/323`).

**Test cases:**
- `param_width_record_whole_copy` (Q): the Q repro (`pkt_t cur_ar; automatic pkt_t next_cur_ar; next_cur_ar=cur_ar; …; cur_ar<=next_cur_ar;` with member `[ADDR_W-1:0]`) → PASS.
- `param_width_record_copy_override`: instantiate `#(.ADDR_W(8))` and `#(.ADDR_W(48))` — verify per-instance widths correct (no frozen-width silent-wrong).
- `mixed_state_record_whole_copy`: literal-width `{int a; logic[7:0] b}` (non-packable) whole copy → PASS.
- `record_whole_copy_by_value`: `b=a; a.x=99;` → `b.x` unchanged (by-value).
- `record_nba_whole_copy`: `a<=b;` fidelity (all fields land together).
- Regression `cross_type_copy_stays_loud`: `a=b` where a,b different record types → loud.
- Regression: packable whole-vector copy (`queue_of_record.rs::whole_struct_copy_is_by_value`) still passes.

**Gate + commit.**

---

## Task 5: F-struct — SoA-record-array element actual expansion (parser); resolves m.name

**Files:** Modify `crates/hdl-parser/src/structs.rs::expand_struct_call_args` (~275-308) to match `BitSelect{base:Ident(arr), index}` (and `IndexedPart`) where `arr ∈ record_soa_vars`. Reuse `soa.rs::soa_member_field` / `struct_sel.rs::parse_record_array_member`. Test: `crates/cli/tests/struct_tf_port_soa_actual.rs` (new).

**Approach:** When an actual is `arr[i]` with `arr` a record SoA array, expand to N per-member element actuals `$unp$arr$field_k[i]` — one per member, lining up with the N per-member formals. Guard to record SoA arrays only (leave packable `record_array_vars` — whole-vector element already works). Arity still checked in `fill_default_args` (correct-or-loud).

**Test cases:**
- `struct_input_formal_string_member` (F-struct): the F-struct repro (`run_kat(kats[0])`, `kat_t{m_e mode; string name}`, body reads `k.mode`) → PASS.
- `mname_resolved` (m.name): the tb_sha3-shape replica (`run_test(kats[0])` with `$display("%-40s mode=%s", name, m.name())` in if/else) → prints correctly. Confirms m.name subsumed.
- `struct_formal_both_members_used`: body reads BOTH `k.mode` and `k.name` → correct values.
- `struct_formal_fixed_array`: `kat_t kats[2]; run_kat(kats[1])` (fixed SoA array) → correct.
- Adversarial `struct_input_by_value`: caller mutates `kats[0].name` after the call reads it → callee saw the pre-call value (pass-by-value deep-copy, §13.5.1). If a suspend is involved, snapshot immunity.
- Regression: packable struct actual (`run_kat(kd[0])` with `{int;int}`) still works; bare-Ident struct actual (`run_kat(structvar)`) still works.

**Gate + commit.**

---

## Task 6: F-record-out — output-formal call in a short-circuit position (elaborate, deep)

**Files:** Modify `crates/elaborate/src/hoist.rs` (`hoist_inout_calls` / `hoist_stmt_top` / `has_unhoistable_inout_call`) and `crates/elaborate/src/stmt_flow.rs` (`lower_while`/`lower_for` head lowering). Improve the message at `crates/elaborate/src/frames_call.rs:225`. Test: `crates/cli/tests/output_formal_shortcircuit.rs` (new).

**Approach:** Lower an output-formal call in a short-circuit `&&`/`||` RHS by emitting the copy-out `Terminator::Call` INSIDE the short-circuit branch (guarded conditional), not a flat pre-statement hoist. Split the condition `A && f(out r)` into: eval `A` → branch on `A` → in the taken branch emit `r = f_with_copyout(…)` (statement-context Call, reusing `emit_frame_func_out_call`) then combine → the call does not fire and `r` is not written when `A` is false. Apply in `lower_while`/`lower_for` head and general `&&`/`||` lowering. **This is the deep task — stage carefully.**

**Test cases:**
- `output_formal_in_while_and` (F-record-out): the repro (`while (n<5 && rsp_next(n,r)==1) n++;`) → PASS, loop runs correct # of iterations, `r` populated each call.
- `output_formal_and_shortcircuit_no_call`: `if (0 && rsp_next(n,r))` — assert `rsp_next` NEVER called and `r` NOT written (short-circuit correctness). Use a side-effect counter (module net incremented in the callee).
- `output_formal_or_shortcircuit`: `while (done || step(r))` — `||` variant.
- `output_formal_2strings_valcorrect`: ≥2-string output record via the `&&` path → values correct (locks in "member count irrelevant").
- `output_formal_0strings`: all-int output record via `&&` path → PASS.
- Regression: direct-rhs (`n=rsp_next(n,r)`) and plain-condition (`while(rsp_next(n,r)==1)`) output-formal calls still work (byte-identical).
- Regression: R5-B inout hoist (`crates/cli/tests/` inout-function tests) unaffected.

**Gate + commit.** If `?:` arm is tractable within the same mechanism, include it + a test; else note as a documented follow-on (correct-or-loud, still loud).

---

## After all tasks

- Whole-branch code review (most-capable model).
- Docs: ROADMAP.md (§0/§3 update), ROADMAP_ARCHIVE.md (new §4.5.215 section), CLAUDE.md (status + bullet, git-ignored — edit not commit), memory (block-local + new lessons).
- Verify format_version still 23; full green; clippy/fmt clean.
- Merge to main (owner drives push).
- Final report.
