# §4.5.590 plan — §2 🆕 AB (§5.2 row 2)

status: COMPLETE (audit 37 new cells; decision GO-CONDITIONAL on step 1)

PRE = s590/pre/vita md5 e1e7e57148bb83ad35d2c4f68247a290 (HEAD 303703f9). Probe = s590/probe5/vita (P5; `VITA_PROBE_SIDE=1` = a-side); heap clause emulated by s590/probe3/vita (a-call holds every call CA).
Audit cells: s590/plan/c1 (split / self-contradiction, 4), c2 (held-set predicate, 11), c3 (events / order, 7), c4 (declaration-initializer discriminators, 4), c5 + c6 (hold-window exposure, 6), plan/w (u20b/u20c/u20d, e6plain, nc1). Outputs s590/plan/out_c*/ (r.py: verilator, iverilog, PRE x3 backends, POST x3 backends). Scan: s590/plan/scan.py (normalised whole-output compare, flags moved cells whose PRE equalled an oracle).

## 0. Intent

A function reached from a continuous assign must run at time 0 only once its inputs hold what the time-0 processes wrote, so its reports, `$display` lines and assertion exits match both oracles, without moving any cell an oracle decides and without creating an event that neither PRE nor the oracles have.

## 1. Audit 1 — the 9 split cells (raw lines re-run: a4, a6, a9, v6; rest read from s590/g/out_*)

Self-contradiction cells (one design each):
- T1 `pl_t1_iv_xcall_spelling` — `f1(a)`, `f2(a | 1'b0)`, `f3(av[0])`, initial `a = 0; av = 2'b10;`. iverilog: `f1 t=0 x=0` | `f2 t=0 x=x` + `WARNING: …:10: value is unhandled … Time: 0` | `f3 t=0 x=x` + `WARNING … Time: 0` | `f2 t=0 x=0` | `f3 t=0 x=0`. verilator: `f1 t=0 x=0`, `f2 t=0 x=0`, `f3 t=0 x=0`. PRE: x-run + W4031@0 on all three. POST: `f1/f2/f3 t=0 x=0`, no report. => iverilog's t0 x-run depends on argument spelling for the same bits: no oracle on that axis (ER §7.3 row 1).
- T2a `pl_t2a_order_port_impure` — `logic a = 0, b = 1;` direct `assign y1 = f(a, b)`, port instance `y2`, non-call `wire [1:0] y3 = {a, b}`. verilator: `f …`, `g …`, `i0 y1=01 y2=01 y3=01` (CA first — but a2_declinit alone, same `assign`: `i0 y=00` then `f`). iverilog: `i0 y1=zz y2=zz y3=01` (call after the initial, non-call before, same bits). => both tools self-contradict on the t0 CA-vs-initial order: a §4.7 race with no oracle.
- T3 `pl_t3_const_vs_written` — verilator `f1` (constant args) before `i0 y1=01 y2=00`, `f2` after; iverilog `i0 y1=zz y2=zz`, `f2`, `f1`; PRE `f1`, `f2 x=x`, `i0 y1=01 y2=x1`, `f2`; POST `i0 y1=xx y2=xx`, `f1`, `f2`.

| cell | PRE | POST (a-side) | iverilog | verilator | verdict |
|---|---|---|---|---|---|
| f1_generate | `f t=0 x=x` W4031@0 x2, `x=1`, `x=0` | `f t=0 x=1`, `f t=0 x=0` | x-run + WARNING x2, `x=0`, `x=1` | `x=1`, `x=0` | iverilog spelling-dependent (T1); POST = verilator; not a descent |
| b1p / v4 / v6 | W4031@0 x1 | silent | WARNING `Time: 0` x1 | silent | same (T1: expression / select argument) |
| a4_constargs | `f…`, `i0 y=01`, `i1 y=01` (= verilator exactly) | `i0 y=zz`, `f…` (= iverilog exactly) | `i0 y=zz`, `f…` | `f…`, `i0 y=01` | race only (no x-run possible: empty deps); keep PRE via exemption |
| a9_noarg_const | `f t=0` x2, `i0 y=01 w=01` (= verilator) | `i0 y=xx w=zz`, `f` x2 | `i0 y=zz w=zz`, `f` x2 | = PRE | race only; keep PRE via exemption |
| a6_portconst | `f t=0 x=0 z=1` TWICE, `i0 y=01` | `i0 y=xx`, `f` once | `i0 y=zz`, `f` once | `f` once, `i0 y=01` | count W->C (both oracles 1); i0 = race, POST = hand-IEEE x (§6.8) |
| a7_portdecl | `f x=x z=x`, `f x=0 z=1`, `i0 y=01` | `i0 y=xx`, `f` once | `i0 y=zz`, `f` | `f`, `i0 y=01` | count W->C; i0 race |
| a8_constret | `f x=x` x2, `i0 y=01 w=01` | `i0 y=xx w=zz`, `f x=x` x2 | `i0 y=zz w=zz`, `f x=x` x2 | `i0 y=01 w=01`, `f x=0` x2 | count equal everywhere; race only |

Scan (scan.py over all 116 grounding cells with PRE and P5 outputs): moved 72; flags only a4, a9 (PRE = verilator, POST != verilator) and b1p, v4, v6 (PRE = iverilog, POST = verilator). No moved cell had PRE = iverilog = verilator. The whole-output scan cannot see a one-axis descent (x2b below); step 1 compares the call-line axis and the event axis separately.

## 2. Audit 2 — held-set predicate

`walk_func_body` (levelize/call_deps.rs:439) admits only blocking assigns to the callee's own frame nets, `systask_effect_is_eval_local` tasks, Goto/Branch/Return, Calls to `ir.funcs`, and the count-free system functions; it declines every other write, every other system function (`$random`), Delay/Wait/Fork/task Call terminators, a heap-handle read, `ArrayItem`. With "no SysTask reachable", effect-free is conservative. Measured:
- p1 `$random` in the callee: held (`PROBE side_held_cas=1 of 1`); PRE `y=226`, POST `y=19` (fewer draws; the per-settle family stays).
- p5 `$sscanf`: held; F4004 at time 0 in PRE and POST (loud either way).
- refused before simulation (no lane): DPI `import "DPI-C"` E2002; function->task in a module E3009 (both oracles refuse too); `$fopen`/`$fclose` E3009; `$strobe` in a function E3009; module function with a local class handle `c = new` E3009 (p7, p8); `static int cnt = 0` in an automatic function E2002 (iverilog `sorry`, verilator `DIDNOTCONVERGE`).
- the one miss: a class method body reached through a handle (u20: member write + `new` admitted as effect-free). Heap clause closes it (P3 holds it: u20 `held_cas=1`).
- over-held pure callees reproduce PRE exactly: p9 (dead `if (1'b0) $display`, held 2 of 2) and p10 (pure class method through a handle under P3): stdout identical to PRE and iverilog/verilator.
- W4020 re-pin attribution (u20d: `n` counter + `d == null` print in C::f): PRE `(read X)` + `(write ignored)` at time 0 from the null-handle x-run, then `dnull=1` on every later call; P3 one `(write ignored)` after `dut init` with `obj` constructed, `dnull=1` on every call; verilator `dnull=1` on the first call, `dnull=0` after (iverilog: `recv_object(...) not implemented` abort). The surviving W4020 is a pre-existing ignored member-handle write on a constructed object (PROBE_CATALOG), not the x-run.

## 3. Audit 3 — events, edges, order

| cell | iverilog | verilator | PRE | POST (a-side) | note |
|---|---|---|---|---|---|
| e1_const_edge (const arg, in-body `@(w)` armed in batch 1) | `i0 w=z`, f, P, B, W | f, `i0 w=1`, W | f, `i0 w=1`, P, W | `i0 w=z`, f, P, W, B | B only after a hold: race; exemption keeps PRE |
| e2_decl_edge (decl-init) | `i0 y=zz`, f, B, Y, P | f, `i0 y=01`, P, Y | f x-run, f, `i0 y=01`, P, Y | `i0 y=xx`, f, P, Y, B | count W->C; B = iverilog (race) |
| e3_partial_x | P/W/Y/PY on `x1` | 2-state | P W Y PY | P W Y PY | events = PRE |
| e4_port_tbfirst (DUT via ports, tb first) | f, B, Y, P | f, B, f, P, Y | x-run + W4031@0, Y B P | f, B, P, Y | false W4031 gone |
| e5_batch_vs_release | f, S, W, Z | f, W, S, Z | x-run, S, W, Z | f, S, W, Z | = iverilog = PRE order |
| e6_clkgen_first (assert in a gate fn) | NG t=0, `S t=15 n=1` | `S t=15 n=1` | E4003@0 rc=1, `S t=15 n=2` | rc=0, `S t=15 n=2` | false E4003 gone; `n=2` pre-existing (e6plain, no call: PRE `n=2`, both oracles `n=1`) |
| e7_comb_before_initial | silent | silent | W4031@0 | silent | W->C |
| d1_declinit_overwrite (`a = 0` decl, initial `a = 1`) | `f t=0 x=1 z=1` once | once | 3 lines | once | 2-oracle: refutes any release before batch 1 |
| d2 / d3 (decl-init miss, initial fixes it; d3 via port) | silent | silent | W4031@0 x2 | silent | 2-oracle: refutes value-dependent release |
| d4 (decl-init miss, stays) | WARNING `Time: 0` | `[0] %Error … unique case, but none matched for '2'h0'` | W4031@0 x2 | W4031@0 x1 | report kept |
| x2b (`assign v = (w === 1'bz)`, w held, initial writes a) | f, `V t=0 v=0`, `NV t=0 v=0` | f, `V` | x-run, f, `V`, `NV` | f, `V`, **`PV t=0 v=0`**, `NV` | DESCENT on the event axis (PRE events = iverilog) |
| x2c (`logic w`, `v = (w === 1'bx)`) | f, V | f, V | x-run, f, V, NV | f, V, PV, NV | wrong -> more wrong |
| x1 (w stays x; `v = (w === 1'bz) ? 0 : w`) | `V t=0 v=x` | 2-state | none | V, NV | wrong -> wrong |
| x2d (casez downstream) | f, V | f, V | x-run, f, V, PV | f, V, PV | events = PRE |
| nc1 (no call: `assign v = (a === 1'bx)`, initial `a = 1`) | V, NV | V | V, **PV**, NV | = PRE | pre-existing phantom of the same mechanism |
| x3 (tri-state, held driver) | f, `Q t=0 q=0`, `Q t=0 q=x` | f, Q | f x4 x-run + f, no Q | f x3, no Q | md per-pass residue; Q pre-existing |

Mechanism of x2b: the hold leaves the held net at its default (z) through the seed settle; a non-held CA reading it settles on that default (`z === z` = 1), the t0 edge rebuild delivers `z -> 1` to the header waiter, and the release then moves `v` to 0. It is nc1's pre-existing seed-on-default phantom, reached on a new population (ER §2.2: a routing that interacts with a latent gap is a regression). Fix in-slice: hold the downstream closure (every CA that reads a held net, transitively), so no CA settles on a held net's default.

## 4. Decision — GO-CONDITIONAL (amended shape; step 1 must pass before production code)

Shape S590 = a-side + heap clause + downstream closure + empty-deps exemption:
1. seed set: CA's rhs or lhs index reaches a user call AND (a reached callee is not effect-free OR the CA reads a heap handle) AND NOT (`ca_deps` certified AND its read set is empty AND not a multi-driver member; certification already excludes a delayed CA) — the exempt CA is the one PRE evaluates once, at the seed, plus only `k_release` re-dirties (`levelize/mod.rs` `redirty_targets`), and the exemption keeps PRE's path for it, so no premise about its count is needed beyond "nothing a time-0 process writes reaches it";
2. held = closure of the seed set over CA read deps (a CA reading a net any held CA drives);
3. every settle before the first batch (seed, initializer re-settle, pre-batch loop-top), the md loop, `schedule_delayed_cas` and the copy-net repair skip held CAs; a held delayed CA keeps its initial x drive sized statically;
4. at the first settle after the first batch take (or at run start with none): settle non-held to fixpoint, release held in dependency waves, settle after each; a net first dirtied by the waves with no definite bit leaves the change list (X-DROP);
5. native loop-top settle ends the run on a latched call_fatal (the engine's `check_call_fatal`); no IR, format or JIT change.
Reasons: d1/d2/d3 (2-oracle) need the hold through batch 1, so no earlier release (ER §2.2, §4.6 measure the fix shape); x2b needs the closure (LOOPROMPT §4 new silent = fix now); a4/a9/e1/T3 are race-only moves with no oracle (LOOPROMPT §1: do not touch a split axis), so certified empty-deps CAs stay on PRE's path (nothing a time-0 process writes can reach them, so holding them only reorders a race). The closure and exemption are unmeasured lanes, hence step 1 (ER §10.2). Stop rule: a new descent or a corpus digest move in step 1 that the closure cannot explain = no-go; file "hold window needs a downstream-closure / t0 delivery model" as the prerequisite row and keep the diff.

## 5. Plan (ordered; each step's verification in brackets)

0. Setup: worktree from main AFTER §4.5.589 lands (no source overlap: s589 = crates/elaborate/* + tests/decl_scope_pkg_text.rs; overlap = docs/ROADMAP.md, CHANGELOG.md only); own CARGO_TARGET_DIR; if main moved, freeze PRE' = release build of new main via git archive and re-run the 124 + 37 cells on PRE' [PRE' == PRE on every cell, or list the cells s589 moved (expect only t1_pkg_call / t2_pkg_import_call)].
1. P6 probe = P5 + heap clause + closure + exemption + copy-repair skip (both backends). Measure: 124 grounding + 37 plan cells + fork-at-t0 cell + copy-of-held-`logic` cell, x3 backends; corpus.py (22 files) PRE vs P6; full gate; held counts per corpus row [x2b PV gone and x2c PV gone; a4/a9/e1/T3 byte-identical to PRE; no cell where PRE = iverilog = verilator moves, checked per axis (call/report lines vs always-block lines); every W->C / FL->C of the grounding kept; corpus 22 files byte-identical; gate 9048 with only the u20 re-pin; no BACKEND SPLIT].
2. Production code (rewrite, not paste): new `crates/sim-engine/src/sched/t0_hold.rs` (scan_arm.rs is 1854 lines, ER §10.1): `T0Hold { held, preds, released, active, releasing }`, `build(ir, deps, windows, heap)`, `held_now`, `next_wave`, generic X-DROP over `NetReader`; `levelize/call_deps.rs` `func_effect_free` (+ export in levelize/mod.rs); callee collection as an exhaustive `_`-free walk over `sim_ir::Expr` that holds on any arm it cannot see through (P5's `calls_in` skips `ArrayItem`); Scheduler::new builds it; scan_arm.rs settle loop / md loop / x-drive / `schedule_delayed_cas` / copy repair; run_loop.rs first batch take; native/run.rs mirrors (settle, md, x-drive, `arm_t0` copy repair, release flag, loop-top fatal). Delete t0_mode b/b2, PROBE_NULL_SINK, every `VITA_PROBE_*` read. Hoist `holding` out of the per-CA loop (hottest loop) [cargo build -p cli --locked; P6 cell set re-run == P6].
3. Tests: new `crates/cli/tests/t0_call_cont_assign_hold.rs` (cargo fmt --all first), oracle raw lines in doc comments: k1 (f once), k2b (exit 0), q1caf_tbF/L, d1, d2, d3, d4 (one W4031@0), j1 (stay-x report kept), a10/a11/a12 (report at time 0 kept), x2b (V, NV, no PV), e4, e7, k7_finish (f once), e6 without the counter (exit 0), c2_delay_i0 (`i0 w=xx`), n4e (no t0 events), k3 on 3 backends (end at time 1, Error), T1 (no x-run, iverilog disqualifier cited), race-axis byte pins documented as such: a4 / T3 (PRE order kept), T2b (pure call untouched), copy-of-held-logic i0; residue pins with why: s4 (`f` 2 at t0 + 1 at t1, oracles 1/0), c1_delay. Re-pin unique_if_chain.rs `a_constructor_called_through_new_stays_silent_at_time_0` W4020 2 -> 1 with the u20d attribution; rewrite every `🆕 AB` prose line in that file to POST's measured output [scoped: `cargo nextest run -p cli -p sim-engine --locked`].
4. Pre-review gates: fmt; `cargo clippy --workspace --all-targets --locked -- -D warnings`; release `cargo run -p corpus-runner --locked -- run` right after the scoped gate (corpus = order oracle); mutants with expected kill written first (ER §7.4; `cargo nextest run --workspace --locked --no-fail-fast` each; own CARGO_TARGET_DIR; snapshot/restore by `cp`): M1 seed set empty -> k1, k2b, d1; M2 drop effect-free test -> T2b; M3 drop heap clause -> u20 W4020; M4 drop exemption -> a4, T3; M5 drop closure -> x2b; M6 releasing true at construction -> k1; M7 one wave ignoring preds -> b1_cachain / s5; M8 drop X-DROP -> n4e; M9 drop held x-drive -> c2_delay_i0; M10 drop md skip -> s4; M11 drop delayed-schedule skip -> c1_delay; M12 drop native fatal -> k3 + frame_call test; M13 no hold at initializer re-settle -> d1, d2; M14 no hold at pre-batch loop-top -> n_ca_objf2case_tbL (VL-only count) / u20; M15 copy repair evaluates held -> copy-of-held-logic i0 [corpus failing 0 and byte-identical; every mutant killed; a survivor = unexplained].
5. Review (LOOPROMPT §4, ≤3 rounds): differential + soundness lenses on a frozen POST, PRE three-way; briefing = this audit table + the P6 table; expected race-axis moves listed (i0 reads of held and downstream nets, in-body B lines); soundness open question: edge OR-ing when the waves write one net twice (a held CA cycle) — build the cell before adding an endpoint edge rebuild [findings adopted per ER §3.5; delta re-review after any blocking fix].
6. Final gates: full `cargo nextest run --workspace --locked` + `cargo test --doc --workspace --locked` (0 fail, pass count up vs PRE's log, format_version unchanged, SLOW checked); flip run (ER §7.2, `Bytecode` both spellings) vs PRE's flip run; staged `--features separate-bins` (c6 9/9 + new cells == one-shot); product `--no-default-features` (cells: only the 3 known product refusals differ); JIT `--features jit` + `VITA_JIT=1` (cells x3 backends == non-JIT); corpus once more (failing 0); optional perf A/B on picorv32 interleaved both orders (±3% = no change).
7. Docs (same commit set): ROADMAP — rewrite 🆕 AB to its residue (per-settle re-run of uncertified CAs: s4, c1_delay, u20d, e2 with POST numbers; ER §2.6), drop §5.2 row 2, re-measure unique-if-chain _H cells and rewrite its BLOCKED list, re-measure ⑧ "extra t0 settle" (i3/i4), Summary recount; §0 iverilog disqualifier ⑨; §2 "Oracle splits" line; PROBE_CATALOG lines; manual 006 (:189 reason, :853 row -> residue merged with :852); REMAINING_WORK:12; CHANGELOG; hdl-parser/src/lib.rs:898 comment.

Byte-identity: a design with an empty seed set (no CA reaching a non-effect-free callee or reading a heap handle, or only certified empty-deps ones) has an empty closure; `T0Hold.active` is false from construction, so every settle is the inner settle, the per-CA check is off, md / delayed / copy-repair skips never fire, the release and X-DROP never run. Outside the boundary only the native loop-top fatal check changes behaviour, and only on a latched call_fatal (k3, k4, k6: native now = interp = iverilog). Non-vacuity: held counts per corpus row (ethernet > 0, ibex 0) recorded in step 1.

## 6. Rows to file

- Native loop-top fatal: closed in-slice (fix path), commit message only.
- iverilog x-run on an expression / select argument (T1): ROADMAP §0 "iverilog defects — oracle disqualifiers" item ⑨, worded as a self-contradiction, pinned by the T1 test.
- Held-logic / held-net i0 value (vita z wire / x var, iverilog z, verilator CA-first or 2-state 0): ROADMAP §2 "Oracle splits" one line (§4.7 race; T2a, T3 self-contradictions).
- nc1 seed-on-default phantom edge (non-call CA): PROBE_CATALOG (PRE = POST; iverilog V, NV; vita V, PV, NV).
- u20d ignored member-handle write `d = new(…)` on a constructed object, W4020 text says null/X: PROBE_CATALOG.
- u31 E3009 (function calling a void function with formals, from a CA; both oracles `g t=0 x=0`): PROBE_CATALOG.
- `static … = init` in an automatic function E2002 (no oracle): PROBE_CATALOG.
- e6plain resume/hop order (`S t=15 n=2`, both oracles `n=1`): check §2 row 7 / line 371 text; PROBE_CATALOG only if no line names it.
- AB residue: rewrite in place (not a new row), numbers from the final binary.

## 7. Could not determine

- Whether the closure keeps the corpus byte-identical and the gate at one re-pin (step 1).
- Which PRE W4020 line in the pinned u20 design is the x-run (both read `(write ignored)` there; u20d separates them) — attribute before re-pinning.
- That `expr_is_pure_of_nets` is false whenever a callee's read set is unknown (exemption soundness) — step 1 cell: `assign y = f();` reading `mem[0]` written by the initial must be held.
- IEEE text on a continuous assign's time-0 evaluation and on §12.4.2.1 flush points for a CA was recalled, not quoted; no decision rests on it.
