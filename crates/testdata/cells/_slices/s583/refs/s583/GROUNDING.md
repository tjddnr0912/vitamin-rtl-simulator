# §4.5.583 grounding — unique-glitch-t0 + unique-if-chain

HEAD bf7f3e0d. Scratch root S = this directory. Cells: S/c/*.sv, raw outputs S/c/*.out, harness S/run.py
(iverilog -g2012 + vvp; verilator --binary --timing --assert, run with +verilator+error+limit+1000;
sv2v 0.0.13 -> iverilog; vita PRE --backend native|interp|vm; a single "vita (native=interp=vm)" block means all
three backends printed identical bytes — true for every cell below).

PRE binary: S/pre/vita  md5=2492e1c5c52c7081a6c0997fe194af40  size=7321504  (cargo build -p cli --locked --release, release)

## Verdict first

- A as recorded is refuted. Neither oracle defers the report. iverilog 13 and verilator 5.052 both report a zero-delay
  glitch at t>0 the moment the case executes, exactly as vita PRE does (9 two-oracle cells). IEEE §12.4.2.1 deferral
  would make vita silent on all 9 — a descent against two agreeing oracles.
- The recorded t0 silence comes from the order processes start at time 0, not from deferral. iverilog runs an
  always_comb's implicit time-0 execution after the initial procedures. vita seeds Initial|Comb|Latch at t0 in
  ProcId (source) order, so an always_comb written before the initial that drives it runs first, on x inputs.
- B is real (verilator-only oracle; iverilog rejects `unique if`). It is a parser-only fix and reaches every lane identically.

## Cell table (A)

iv = iverilog, vl = verilator, s2v = sv2v->iverilog (sv2v rewrites unique to `(* full_case, parallel_case *)`, so it
never reports: NO_ORACLE in every cell; run on a00, a01, b00 only). vl is not an oracle for x/z or event order (2-state,
optimizer-dependent order) — flagged per cell.

| cell (S/c/) | iv | vl | vita PRE | verdict |
|---|---|---|---|---|
| a00_repro.sv (recorded) | report t2 only | report t2 only | report t0 + t2 | vita-wrong at t0 (2 oracles silent) — cause: order (a00t) |
| a00t_repro_trace.sv | init; eval 01; eval 01; report t2 | init; eval 01; report t2 | eval xx; REPORT t0; init; eval 01; report t2 | vita-wrong: comb ran before initial |
| a00s_repro_initfirst.sv (same, initial textually first) | report t2 | report t2 | report t2 (init; eval 01) | agree -> vita depends on source order |
| a00p_repro_initfirst_pure.sv | t2 | t2 | t2 | agree |
| a01_comb_hash0_glitch.sv (always_comb, `#5 r=00; #0 r=01`) | eval 00, report t5, eval 01 | report t5 | report t5 | agree (IEEE-deferral: silent) |
| a02_star_hash0_glitch.sv (always @*) | report t5 | report t5 | report t5 | agree |
| a03_atr_hash0_glitch.sv (always @(r)) | report t5 | report t5 | report t5 | agree |
| a04_latch_hash0_glitch.sv (always_latch) | report t5 | report t5 | report t5 | agree |
| a05_posedge_double.sv (two posedges in one step) | report t5 | report t5 | report t5 | agree |
| a06_initial_two_writes.sv (`r=00; r=10;` one process) | silent | silent | silent | agree (control) |
| a07_nba_chain.sv (NBA settle glitch ab=10->11) | report t5 | report t5 | report t5 | agree |
| a08_comb_chain.sv (case comb before `b=a` comb) | t5 only (no t0) | silent (vl evaluates b first; order) | t0 (ab=0x) + t5 | t0: vita-wrong vs iv; t5: iv=vita, vl order-flagged |
| a08b_comb_chain_rev.sv (`b=a` comb first) | REPORT t0 (ab=0x) | silent | silent | split iv/vl (x+order); vita=vl |
| a09_comb_persist.sv (real no-match persists) | report t5 | report t5 (repeats each eval) | report t5 | agree |
| a10_resume_event.sv (violation, then `@(ev)` resumes same step) | report t5 | report t5 | report t5 | agree (IEEE flush: silent) |
| a10w_resume_wait.sv (same with `wait(go)`) | report t5 | report t5 | report t5 | agree (IEEE flush: silent) |
| a11_same_proc_hash0.sv (violation, `#0`, fix, same process) | report t5 | report t5 | report t5 | agree (IEEE too: #0 is not a flush point) |
| a12_func_two_procs.sv (fn with unique case; P1 real miss, P2 always_comb glitch) | 2 reports t5 `Scope: top.f` | 2 reports t5 | 2 reports t5 `[in top.f]` | agree (IEEE: P1 only) |
| a13_initial_x_t0.sv (initial, r=x at t0) | report t0 | report t0 | report t0 | agree |
| a14_comb_t0_nba_init.sv (`initial r <= 01`) | eval xx, REPORT t0, eval 01 | eval 01 only | eval xx, REPORT t0, eval 01 | iv=vita; vl not x-oracle |
| a15_comb_t0_hash0_init.sv (`initial #0 r = 01`) | eval 01 once, silent | report t0 (2-state r=00 is a real miss) | eval xx, REPORT t0 | vita-wrong vs iv (order); vl not x-oracle |
| o01_t0_hash0hash0.sv (`#0 #0 r=01`) | REPORT t0 | — | REPORT t0 | agree (iv only) |
| o02_t0_hash0x3.sv | REPORT t0 | — | REPORT t0 | agree (iv only) |
| o03_t0_hash0_then_nba.sv | REPORT t0 | — | REPORT t0 | agree (iv only) |
| o04_t0_always_hash0.sv (`always begin #0 r=01; #10; end`) | silent | — | REPORT t0 | vita-wrong vs iv (order) |
| o05_t0_assign.sv (`assign r=01`) | eval 01 x2, silent | — | eval 01 x1, silent | no report either; eval count differs |
| o06_t0_initial_display.sv | init0, init1, eval xx+REPORT, init2, init3, eval 01 | — | eval xx+REPORT, init0..init3, eval 01 | report in both; order differs |
| o07_three_combs.sv | I, I#0, P3(abc=0xx), P2, P1, P2, P3(000) | — | P1, P2, P3(000), I, I#0 | order: iv runs combs after first #0 batch, reverse source order |
| v01_comb_const_t0_read.sv (stdout VALUE) | init y=xx z=x / #0 xx x / #0#0 11 1 | 11 1 x3 | 11 1 x3 | split iv/vl; vita=vl |
| v02_comb_const_initial_first.sv | xx / xx / 11 | 11 x3 | xx / 11 / 11 | split; vita line 2 matches neither |
| v03_latch_t0.sv | xx / xx / 10 | 10 x3 | 10 x3 | split; vita=vl |
| v04_selftimed_always_order.sv | gen; init clk=0 | init; gen | gen; init clk=0 | vita=iv |
| r01_order.sv (ORDER) | comb-eval 00; WARNING t5; A-after-#0; after-NBA; monitor; strobe | same plus an extra `monitor t=5 r=00 q=0` before A-after-#0 (order-flagged) | identical order to iv | agree with iv. Streams: iv WARNING on stdout, vl stdout, vita stderr |
| f01_finish_same_step.sv (`r=00; $finish;` same step) | `$finish called at 5`, eval 00, NO report | report x2 | report t5, ended Finish at 5 | split |
| f02_finish_after_hash0.sv | report t5 then $finish | report then $finish | report t5 | agree |

Fitted rule (all three tools): the report prints when the violating statement executes, in every process kind,
at t0 and t>0. No tool flushes on always_comb re-trigger, @/wait resume or #0. Counterexamples to IEEE §12.4.2.1
deferral on BOTH oracles: a01, a02, a03, a04, a05, a07, a10, a10w, a12-P2. On iv only: a08@t5, a14, o01, o02, o03.

iverilog's time-0 always_comb order, fitted on o06/o07/a15/o01/a00t/v01: always_comb's waiter is armed from the
start (an active-region write from an initial triggers it: a00t prints eval twice). Its implicit t0 run happens in
the inactive region, after the first batch of `#0` continuations (o06: after init1, before init2; a15: after
`#0 r=01`; o01: before the second `#0`), and multiple always_combs run in REVERSE source order (o07, a08/a08b).
IEEE 1800 §9.2.2.2.2: always_comb runs once at time zero; the clause that it runs "after all initial and always
procedures have been started" is my recollection and was not checked against the text in this session. Order
among always_combs is unspecified by IEEE.

## Message text, severity, exit (m_case_*.sv, m_if_*.sv, b00)

- iv: compile `vvp.tgt sorry: Case unique/unique0 qualities are ignored.` (unique and unique0 case, not priority). Run (stdout):
  `WARNING: <file>:<line>: value is unhandled for priority or unique case statement` + `         Time: N  Scope: top`. unique0 case
  no-match: silent. `unique if`/`unique0 if`/`priority if`: syntax error, compile rc=8 / 50. Run rc=0.
- vl: `[N] %Error: <file>:<line>: Assertion failed in top: unique case, but none matched for '2'h0'`; priority case:
  `priority case, but non-match found for '2'h0'`; if: `'unique if' statement violated`; `priority if`: silent (single and
  chain); unique0: silent. Default: `%Error: ...: Verilog $stop` + `Aborting...`, rc=1; with +verilator+error+limit it continues, rc=0.
- vita: `<file>:<line>:<col>: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for priority or unique case statement [in top] [at time N]`
  (stderr), rc=0; same "case statement" text on `if` forms; unique0/priority0 silent (IEEE: unique0 no-match is not a violation — all agree).

## Cell table (B)

| cell | vl | vita PRE | verdict |
|---|---|---|---|
| m_if_unique.sv t1 `unique if (a)` | report | report | agree |
| t2 chain2 / t3 chain3 | report / report | silent / silent | vita-wrong |
| t4 chain with final else | silent | silent | agree |
| t5 `else begin if (b) … end` | silent | silent | agree (walk must not enter a block) |
| t6 `if (a) begin if (c) … end else if (b)` | report | silent | vita-wrong |
| t7 dangling `if (a) if (c) … else …` | report | report | agree |
| t8/t9 chain matches | silent | silent | agree |
| m_if_unique0.sv (all 9) | silent | silent | agree |
| m_if_priority.sv | silent everywhere | t1, t7 | hand-IEEE §12.4.2; vl not an oracle for priority-if; chain2/3/t6 missing under hand-IEEE |
| b00_repro.sv | chain t1, single t2; priority silent | single t2, priority t3 | vita-wrong (chain) |
| b01_nested_qual.sv t1 `unique if … else unique0 if (b)` | report | silent | vita-wrong (naive walk fixes it) |
| b01 t2 `else unique if` / t3 `else priority if` / t4 `unique0 if … else unique if` | 1 report each | 1 report each (inner's arm, col 41/43/42) | agree on count |
| b01 t5 `else L1: if (b)` | silent | silent | agree (labeled stmt parses to a Block) |
| b01 t6 chain3 with repeated cond | report | silent | vita-wrong |
| b02_frame_lanes.sv fchain / fsingle / tchain / tsingle | report x4 | -, report, -, report | vita-wrong on chains in function and task bodies |
| b03_chain_comb_t0.sv (chain in always_comb before its driving initial) | t2 (glitch), t3 (+t4 artifact) | silent | vita-wrong; after the fix vita would also report t0 (eval ab=xx: the order residue) — prediction, unmeasured |
| b04_single_comb_t0.sv | t2, t3 (+t4) | t0 (eval a=x), t2, t3 | t0 vita-only (same order residue, already present for single if) |

Corpus reach: no `unique/priority if` in any bench/*/src (0 sites). `unique|priority case` only in ibex (182 lines);
PRE on ibex (corpus args): rc=0, DIGEST=32e0e78741376133, 0 x W4031 (6 x W4029). The corpus digest is scanned from stdout only.

## Code census

Producer (parser):
- crates/hdl-parser/src/assertions.rs:451 `parse_unique_priority`; warn_stmt :462; if arm :482–489 (injects only when
  the FIRST if's `else_s.is_none()`: the B defect); case arm :491–506 (default pushed if none); suppress for unique0/priority0 :456.
- crates/hdl-parser/src/stmt_ctl.rs:70 `parse_if` (else-if = `else_s: Some(Box<Stmt::If>)` directly); stmt.rs:166 caller;
  stmt.rs:232 `parse_labeled_stmt` (a labeled non-block becomes a Block, so a walk over direct `Stmt::If` stops there, matching vl).
- crates/hdl-ast/src/lib.rs:39 `UNIQUE_VIOLATION_TASK = "$__vita_unique_violation"`; hdl-parser/src/expr.rs:656 (the name is reserved).

Lowering:
- crates/elaborate/src/systask.rs:195 name -> `SeverityKind::UniqueViolation`; elaborate/src/tables.rs:71, :83 `diag_class` -> (Warning, RunUniqueViolation);
  diag/src/code.rs:130 `RunUniqueViolation => W-RUN-UNIQUE-VIOLATION / VITA-W4031`; elaborate/src/frames_classify.rs:1311 admits severities in frame bodies.

Consumers (engine lanes):
- builtins/dispatch.rs:251 (dispatch_body) -> builtins/queues_io.rs:167 `run_severity_with` -> :183 `emit_severity_message` (sink, sim_time; :218 UniqueViolation => Continue).
  Reached by: interp+vm via sched/kernel.rs:76; native tier-3 via native/kernel.rs:2716/2749 `k_dispatch_systask`; JIT via jit.rs:766 `k_dispatch_systask`.
- Frame lanes bypass dispatch: state/frame_eval.rs:1705 (function frames) and state/task_frames.rs:211 (task frames) -> frame_eval.rs:1360 `frame_emit_severity` (:1393).
- profile.rs:473 label.

Tests pinning W4031/unique text: crates/cli/tests/round29_report.rs (13 hits; :339 `a_runtime_diagnostic_says_when_it_fired`, :374),
unique0_priority0.rs (5; :40 `unique0 if … else if`, suppressed so not moved by B), severity_in_frame_body.rs (2), case_inside.rs (2; :464–531),
runtime_diag_location.rs:188 (`d.sv:16:12: warning[VITA-W4031]`), procedural_adv.rs:97, :128 (single `priority if` and `unique if`, "one warning").
None has a non-unique0 `if … else if` chain, so by reading no pin moves under B; the suite has not been run.

Deferred machinery (`assert #0` / `assert final`):
- parser assertions.rs:223 -> `Stmt::DeferredAssert`; elaborate stmt_main.rs:723 (region), lib.rs:1818 `cur_defer`, stmt_flow.rs:84 `prune_deferred_actions`;
  tables.rs:218 `DeferRegion`, :232 `DeferMarkTable` (marker sid -> region), :239 `DeferActTable` (action sid -> (marker, region)).
- engine state/mod.rs:188/:191 `deferred_observed` / `deferred_reactive`: BTreeMap<(marker_sid, aid, gen), Vec<DeferredReport>>.
- sched/run_loop.rs:510 `try_defer_with` (marker = remove key; action = render text at reach, push); :606 `mature_deferred`
  (emits via emit_severity_message, in key order); :670 `drain_deferred_on_finish`; maturation call sites run_loop.rs:276/:299
  (Scheduler) and native/run.rs:578–579, :1477; finish drains run_loop.rs:159/166/363/480, native/run.rs:469/476/617;
  sched/kernel.rs:389–410 disable-fork cancels killed activities' entries.
- What it implements: flush-on-re-reach of the SAME marker within the SAME activation (aid, gen) — a re-reach replaces the entry —
  plus cancel-on-kill. Not implemented: §16.4.2 flush on @/wait resume or always_comb re-trigger (other than by re-reaching the marker),
  flush of the whole per-process queue, or two failures before a flush point both queued (vita replaces).
- Process identity: Scheduler `cur_aid`/`cur_gen` (sched/mod.rs:542; `set_cur_activity` wait_fork.rs:13; native/run.rs:85–91), available
  at dispatch_body. NOT available in `frame_emit_severity` (SimState, &self): a report from a function/task frame has no process key.
- Flush-point hooks that would be needed: waiter wake from @/wait (sched waiters; native/wake.rs), static Level waiter re-fire for
  always_comb/always_latch (scan_arm.rs:1586, native/wake.rs:140). None touches the deferred queues today.

Time-0 seeding (the actual root of the recorded t0 cell):
- sched/scan_arm.rs:1448 (Scheduler: interp/vm) and native/run.rs:1125 (native): `Initial | Comb | Latch` pushed to `cur.active` in (seq, tie) order.
- elaborate/src/events.rs:364–417: a self-timed `always` with no sensitivity is also `SensKind::Comb` (empty edges), and so is
  `always_comb y = 2'd3;` (empty inferred read set). The engine cannot tell always_comb/always_latch from a clock generator, so a t0-order
  fix needs a discriminator (new SensKind = frozen IR + format bump, or a side table). Other Comb consumers: native/kernel.rs:2331,
  native/wake.rs:140, levelize/mod.rs:101/551/590/687, sched/propagate.rs:669, elaborate/src/hier.rs:933.

## Lane table for B (parser desugar; no shared engine function edited)

| lane | runs the injected arm | measured on PRE |
|---|---|---|
| native tier-3 (default) | yes, dispatch_with | yes (m_if_*, b00–b04) |
| interp | yes, sched/kernel.rs:76 | yes (byte-identical to native) |
| vm | yes | yes (byte-identical) |
| JIT (`jit` feature, off) | yes, k_dispatch_systask | no (feature not built) |
| function frame | yes, frame_eval.rs:1705 | b02 fchain/fsingle (no instrumentation of which executor ran) |
| task frame | yes, task_frames.rs:211 | b02 tchain/tsingle (same caveat) |
| staged vcmp/velab/vrun | same AST/IR | no (needs `--features separate-bins`) |

## Recommendation

1. Ship B: in the if arm of `parse_unique_priority`, follow `else_s` while it is directly a `Stmt::If` (never into a Block
   or labeled statement) and inject at the last `if` whose `else_s` is None; skip when suppress_no_match. Keep the first if's span
   (vl reports the qualifier's line). Predicted moves: m_if_unique t2/t3/t6, b01 t1/t6, b02 fchain/tchain, b03 t2/t3 -> report
   (vl agrees); priority chains -> report (hand-IEEE; vl silent on all priority-if); unique0 chains, labeled and `else begin if` stay silent.
   Known widening: b03 gains a t0 report from the time-0 order residue that single `unique if` (b04) and `unique case` (a00) already
   show. That is a spurious diagnostic with no value change. Record it under the order row.
2. A: do not build the deferral. It silences 9 two-oracle report cells and moves the report after later Active/NBA prints
   (r01: vita's order matches iv byte-for-byte today). Rewrite the ROADMAP row and manual 006 §1.4: the "Icarus Verilog and
   Verilator print nothing" sentence is false at t>0 (a01). File the real residue as a new row "always_comb t0 order": vita
   starts always_comb/always_latch in source order at t0. Two oracles agree only on the a00 shape (comb written before an initial that
   writes its input with no delay). Values split iv/vl (v01, v03), and iv's exact rule (inactive region, after the first #0 batch,
   reverse order) is implementation-specific. Prerequisite: an always_comb/always_latch discriminator distinct from a self-timed `always`.
   Lanes: scan_arm.rs:1448 and native/run.rs:1125. Corpus digests are at risk if it moves values.
3. If deferral is ever built anyway, these could go wrong. A report from a function/task frame has no process key. No hook exists
   for an @/wait resume or an always_comb re-trigger. Reusing the assert #0 queue gives a10/a10w a report (no re-reach) but makes
   a01–a05 silent, which matches neither IEEE nor the oracles. The finish drain prints a report from the step where $finish is
   reached, which is a split cell (f01: iv no, vl yes). The rendered text is sampled at reach, so it is not lost.
