# s588 PLAN — §2 🆕 AE start decision + implementation plan

status: COMPLETE
PRE = $S/s588/pre/vita md5 e1e7e57148bb83ad35d2c4f68247a290 (release, HEAD 2f2d3f2d)
Planner cells: $S/s588/plan/p01..p21 (harness $S/s588/g/run4.sh; iverilog/vvp wrapped in a 30 s alarm, rc=142 = hang)

## 0. Intent

Row 1 exists so later rows that make the constant interpreter fold more (🆕 AD, `const-fn-case`, `const-fn-systask`,
`unique-const-fn`) stop inheriting "a never-assigned 4-state variable reads as 0". The grounding showed the row's literal
fix (decline at exit, D) descends and recommended F (4-state re-run, frozen x-bearing exit). This plan decides what can
ship now without a descent and re-files the rest.

## 1. Spot-check of GROUNDING code claims (HEAD 2f2d3f2d) — all confirmed

- const_fn.rs:908 Ident arm `match env.get(n)`; :1213 `(*env.get(&seg.name)?, w, s)`; :1335 `env.insert(name, 0)` (no
  init); :1507 `env.entry(name).or_insert(0)`; :1518 `*env.get(name)?`; :1582 `env.get(name).copied().unwrap_or(0)`.
- These four are the only env value reads (grep env.get/insert/remove/entry over const_fn.rs, const_fn_width.rs,
  const_select.rs, const_eval.rs).
- hdl-ast lib.rs:682 `ParamDecl {kind, signed, ty, range, name, value, span}`: no 2-state field; `ParamType {Implicit,
  Integer, Real, Realtime, Time}`; hdl-parser params.rs:521-524 says `var_kind` is recorded and then dropped.
- The interpreter emits no diagnostic; its only state is the `const_call_pkg` / `const_call_fn` RefCells (restored).
- const_wide.rs `fold_region`: `== != === !==` (wide_eq_with_unknowns), `!`, `&& ||`, reductions are x-aware; bitwise, `~`,
  unary `-`, an ambiguous ternary decline on an unknown; no Call arm (`_ => None`).

## 2. New measurements (PRE + oracles; raw lines)

F's POST values below are by construction (per-call replacement: `fd(2)` known -> 2; `fx1(2)` x-bearing -> frozen legacy 1).
| cell | shape | PRE | iverilog | verilator | sv2v | F (by construction) |
|---|---|---|---|---|---|---|
| p01 | `localparam int Q = fd(2) - fx1(2); P = fd(2); R = P - fx1(2);` | `Q=0 P=1 R=0` | `Q=0 P=2 R=0` | `Q=x P=2 R=x` | `Q=x P=2 R=x` | Q=1 P=2 R=1: Q, R correct->silent |
| p02 | `if (fd(2) - fx1(2))` generate | `E` | `E` | `T` | `E` | T: correct->silent |
| p03 | a41 twins: `int t = int'(2.5); t[0]=1'b1; f={31'b0,t[0]}` / 32-bit overwrite loop | `P=1 Q=0` | `P=1 Q=0` | `P=1 Q=0` | `P=1 Q=0` | W7 decline: both correct->loud |
| p04 | run time `v = 4'bxxx1; repeat (v)` | `n=0` | hang rc=142 | `n=1` | hang rc=142 | — |
| p05b | `assign #(fd(2)) w = r;` r 0->1 at 5 | `t6 w=1` | `t6 w=0` | `t6 w=0` | `t6 w=0` | — (filed, §8) |
| p08b | `wire #(fd(2)) w = r;` | `t6 w=1` | `t6 w=0` | `t6 w=0` | parse error | — (filed) |
| p10 | `assign #(fx1(2)) w = r;` ($strobe at 5) | `t5 w=0` | `t5 w=1` | `t5 w=0` | `t5 w=1` | — (filed) |
| p11 | `assign #(fx(2)) w = r;` | `t5 w=1` | `t5 w=1` | `t5 w=1` | `t5 w=1` | keep |
| p12 | run time `assign #(dv) w = r;` dv=xxx1 | `t5 w=1` | `t5 w=1` | `t5 w=0` | `t5 w=1` | — |
| m19 | `repeat (fd(2))` (grounding) | `n=1` | `n=2` | `n=1` | `n=2` | |
| p06 | same inside an automatic task | `n=1` | `n=2` | `n=1` | `n=2` | |
| p07 | `repeat (fx1(2))` (xxx1) | `n=1` | hang | `n=1` | hang | |
| p13 | `repeat (fr(2))`, `fr[0]=1'b1` on a never-assigned return var | `n=1` | hang | `n=1` | hang | |
| p14 | `repeat (fs(2))`, `t = t + 4'd1; fs = t;` | `n=1` | hang | `n=1` | hang | |
| p15 | `repeat (h(2))`, `h = fd(a)` (nested) | `n=1` | `n=2` | `n=1` | `n=2` | |
| p16 | `repeat (fb(2))`, `t` a block-local in `begin : blk` | `n=1` | `n=2` | `n=1` | `n=2` | |
| p17 | `repeat (fm(2))`, `fm = (t & 4'b0000) + 4'd2` | `n=2` | `n=2` | `n=2` | `n=2` | |
| p18 | `localparam int N = fd(2); repeat (N)` | `n=1 N=1` | `n=2 N=2` | `n=2 N=2` | `n=2 N=2` | |
| p19 | task `repeat (fd(2)) begin @(posedge clk); … end` | `n=1 t=1` | `n=2 t=3` | `n=1 t=1` | `n=2 t=3` | |
| p20 | controls: `t` assigned before read; `int t;` never assigned | `n=1 k=1` | `n=1 k=1` | `n=1 k=1` | `n=1 k=2` | |
| p21 | run time `$display(fr(2), fs(2), fx1(2))` + `repeat` of each | `fr=xxx1 fs=xxxx fx1=xxx1` / `n1=0 n2=0 n3=0` | (hang) | | | |

Readings: verilator runs 2-state at run time (m19/p06 n=1) and keeps x in an `int` localparam (p01) — not an oracle for x
or x->0 (RULES); sv2v turns `int` 4-state (p01 Q=x, p20 k=2) — not an oracle for 2-state. iverilog (and sv2v->iverilog)
hangs on `repeat` of an x count (p04, p07, p13, p14; also grounding l23). vita's run time is x-correct for these functions
and runs an x count 0 times (p04, p21; grounding m24/m25), = IEEE 1800-2017 §12.7.2 (recalled, not re-read).

## 3. Decision: A, smaller cut ("AE-rt") + B for the binder half

Build: the AE guard (a sound record that a constant-function fold read a never-assigned 4-state variable) and its first
consumer, the statement `repeat` count (`repeat_unroll_count`): a fold that read one declines, so the count runs on the
run-time loop, which is already x-correct. Every other lane opts out. File the binder half BLOCKED; downstream rows carry
the guard instead of waiting for the binder half; re-order §5.2.

Why (rules):
- F (F0/F1) is not shippable. Its per-call replacement breaks a cancellation with a still-collapsed x-bearing call:
  p01 Q and R, p02 go correct->silent (PRE = iverilog/sv2v by cancellation). memory `fixing-one-site-unmasks-the-axis`
  (§4.5.366) scores exactly this BLOCKING; ER §2.3 "look for two errors that were cancelling", ER §2.2 "never trade".
  The grounding's lane table missed this lane (m26 measured only the 4-state-binder case, a lateral move), so ER §10.2's
  start condition does not hold for F. No opt-in narrowing contains it: the corrected value escapes through a parameter
  (p01 R = P - fx1(2)), so only an all-or-nothing gate works (ER §2.5 "gate interacting properties on one precondition"):
  the known half waits for the x-bearing half, which needs P1 + P2 + P3 (§7).
- D descends (grounding Q4/Q5: 13 correct->loud, s01 loud->silent, 4 silent->silent').
- C (D where the binder's 4-state-ness is known): no binder knows it (ParamDecl, §1); ranges are a split (l06/l07); only
  the enum-label lane is binder-independent (l16/l17, verilator + hand-IEEE, iverilog refuses the designs): 2 cells, a new
  out-channel, and a refusal (ER §2.2 "do not close a queue item by refusing"). Not worth a slice.
- The W7 decline (a41) descends on its own twins (p03), so it is filed, not built (ER §2.5 "close a silent default
  upward ... with cells whose true value equals the default").
- The cut is the subset of the row that needs no prerequisite (ER §10.2 row 1): `repeat_unroll_count` is a fold that
  predicts run-time behaviour, and PRE admits a word the engine contradicts (m19 n=1 vs vita's own run time n=2 for the
  same call, m25) — ER §2.5 row 2 "keep a fold predicting runtime behaviour admission-only". Declining it routes the count
  to an existing mechanism (ER §10.2 risk order), closes upward (value right, nothing loud), and moves only silent->correct.
- Cost vs value: ~60 lines + pins; 8 cells silent->correct, 0 predicted descents; the guard is what 🆕 AD, `const-fn-case`,
  `const-fn-systask`, `unique-const-fn` need (a newly opened arm declines a guarded run, keeping PRE's loud), so it is
  reviewed once here, in isolation, instead of inside 🆕 AD. F would cost a 4-state re-run for 22 synthetic cells it cannot
  ship. Corpus/suite population of guarded reads = 0 (grounding probe), so both stay byte-identical.

## 4. Implementation plan (cut "AE-rt")

Step 0 — lane table before code (ER §10.2; verify: every row below marked measured or opted out, cells on PRE + oracles)
- Guard readers: `repeat_unroll_count` (const_bound.rs:146) only. Its callers stmt_flow.rs:1119 (`lower_repeat`: unroll vs
  the `$repeat_cnt$` down-counter desugar), frames_classify.rs:816 (`ast_has_repeat_with_timing`), frames_reserve.rs:685
  (`collect_runtime_repeat_spans`) all key on its `is_none()`, so they agree. Re-grep `Stmt::Repeat` / `S::Repeat`
  (done here: const_bound.rs:482 is a constant-body width walk, block_local/proofs.rs:163 `fold_i32` is literal-only,
  da/*, hoist/* fold no count) and record it.
- Opted out (byte-identical, the guard is not read): every parameter/override/default binder, generate if/case/for,
  ranges and dims, enum labels, §2 🆕 AC sinks (replication, `+:`, `[m:l]`, generate-case label), module-scope composites,
  `$bits`, the intra-assignment `repeat (n) @…` count (events.rs:44, :133 — no run-time lane: a decline there is E3009-style
  loud, which would descend on masked cells), CA / net-declaration delays (filed, §8 row b).
- Seed-kind census (soundness premise): the `NetVarDecl.kind` the parser records for packed struct/union, enum (with and
  without a base), typedef'd and type-parameter locals, and `FunctionDef.ret_two_state` for typedef'd and type-parameter
  returns (does it follow an override?). Rule: seed unless the per-instance kind is provably 2-state (`shape_kind` +
  `net_kind_is_two_state`). Under-seeding is the only unsound direction.
- Cells to (re)measure on PRE before code: m19, p06, p07, p13, p14, p15, p16, p17, p18, p19, p20, l23; plus a struct, an
  enum and a type-parameter local count cell from the census.

Step 1 — the guard (crates/elaborate/src/lib.rs `struct Elaborator` fields + driver.rs `Elaborator::new`; const_fn.rs)
- Fields: `const_x_frames: RefCell<Vec<BTreeSet<String>>>` (names seeded-and-not-wholly-assigned, one set per call) and
  `const_x_reads: Cell<u64>` (monotonic count of value reads of such a name, any depth).
- `eval_const_call`: push an empty set after the formal loop and package-constant seeding, before `body_decls`; pop after
  `body()`, next to the `const_call_pkg/fn` restore (the closure already funnels every early return).
- Seed (W3/W8): `bind_const_decl`, name with no initializer and 4-state kind -> insert; any other name -> remove.
  Seed (W5): before :1507, if `!f.ret_two_state && !env.contains_key(name)` -> insert the return name.
- Count a read: R1 (:908 `Some(v)` arm), R2 (:1213), R3 (:1518 exit read) -> if the top set holds the name, bump the
  counter. R4 (:1582 RMW) is not a read: the unwritten bits stay seeded and any later value read counts.
- Clear (W6): blocking whole-name assign (:1649) -> remove after `eval_const_assign(rhs)` (so `t = t + 1` counts first).
  A select write (W7) does not clear.
- API: `pub(crate) fn const_fold_x_tainted<T>(&self, f: impl FnOnce() -> T) -> (T, bool)` (snapshot / compare); doc it as
  the AE guard contract downstream rows cite.
- Verify: no value, control flow or decline inside the interpreter changes (diff is bookkeeping only); clippy has no
  dead-code warning (the consumer reads it).

Step 2 — the consumer (const_bound.rs:146)
- Keep the fill-literal arm first; then `let (n, tainted) = self.const_fold_x_tainted(|| self.const_bound_u32(count));`
  and return `None` when tainted, else the old `match`.
- Verify: m19/p06/p15/p16/p19 n=2, p07/p13/p14 n=0, l23 n=0, p17 n=2, p18 n=1 (opted-out binder residue), p20 n=1 k=1.

Step 3 — pins (new crates/cli/tests/const_fn_x_repeat.rs; `cargo fmt --all` first)
- Values: m19 `n=2`, p06 `n=2`, p15 `n=2`, p16 `n=2`, p19 `n=2 t=3` (iverilog + sv2v; verilator 2-state n=1 recorded);
  p07, p13, p14 `n=0` (hand-IEEE §12.7.2; iverilog/sv2v hang rc=142 and verilator `n=1` recorded in the comment);
  keeps: l23 `n=0`, p17 `n=2`, p20 `n=1 k=1`; p18 `n=1 N=1` as a REFUSED-style residue pin with `n=2 N=2` (all three
  oracles) beside it (ER §2.6 "keep the refused shape's oracle text"); census cells from step 0.
- Shape pin (the only teeth for over-seeding, M5): untainted controls (p20 both) keep the straight unroll — no
  `$repeat_cnt$` net (stmt_flow.rs:1172 / frames_reserve.rs:565) — and m19 gets one; via an elaborate unit test
  (crates/elaborate/src/tests, `elab_ok` -> SimIr nets) or an existing IR/obs dump if the CLI has one.
- Gates: scoped `-p cli --test const_fn_x_repeat` + the repeat/frame test files; `-p elaborate`; then the full gate once
  (`cargo nextest run --workspace --locked` + `cargo test --doc --workspace --locked`), workspace clippy, fmt, corpus.

## 5. Byte-identity argument (to keep, by code path)
1. The guard only writes its own side tables; no interpreter value, branch or decline reads them.
2. Only `repeat_unroll_count` reads the counter; every other lane is byte-identical by construction.
3. In it, an unguarded fold takes the old path verbatim.
4. Population: the grounding probe logged 0 reads of a never-assigned 4-state variable over the whole suite (9048 tests)
   and the corpus (11 workloads, 4 interpreter calls), and this guard's read set is a subset of the probe's (R4 excluded),
   so the guarded branch never fires there: suite and corpus (IR, digests, grades) byte-identical.
5. Non-vacuity (ER §3.2): the moved cells fire the guard (count it in the review build), the controls do not.

## 6. Mutants for the lenses (each must die on a pin; survivors documented)
- M1 drop the R2 count (placement resolver): a count cell reading the seeded var through a concat (`{3'b000, t[0]}`).
- M2 drop the R3 count: p13 (n=1 instead of 0).
- M3 no propagation through nested calls (per-frame flag): p15.
- M4 clear on a select write: p07.
- M5 seed 2-state locals too (over-seed): value-invisible; the `$repeat_cnt$` shape pin on p20.
- M6 consumer ignores the guard: m19.
- M7 seed only locals, not the return variable: p13.
- M8 seed only function-level decls, not block decls: p16.
- M9 clear before evaluating the rhs: p14.
- M10 push the frame before the formal loop (arguments read the callee's set): an argument naming a caller's seeded var.

## 7. Risks (GROUNDING open risks + new) and mitigation
- G1/G2 (walk divergence, 2^depth re-runs), G4 (F1 vs 🆕 H ⓕ), G5 (a42): not applicable — no 4-state re-run is built.
- G3 (m26) generalises to p01/p02 (correct->silent): the reason F is not built; recorded in AE's row.
- G6 IEEE recalled: p07/p13/p14 rest on §12.7.2 alone plus vita's own run time (p04, p21). If hand-IEEE is not accepted,
  drop those three from the moved set by keeping them as pins of POST = vita's run time (the decline cannot be scoped to
  known results without the 4-state re-run).
- G7 iverilog hang on `repeat (x)`: keep the alarm wrapper in every harness.
- G8 W7 decline: measured descending (p03) -> filed, not built.
- N1 seed-kind under-seeding (struct / enum / typedef / type-parameter locals, type-parameter returns): step-0 census and
  cells; seed when unsure.
- N2 run-time fallback must lower and run the function: a tainted count whose function the run-time path refuses or runs
  differently (recursion, `static` locals keeping values across run-time calls, package / `$unit` functions, a function
  with a `unique` / `$display` arm now running at run time as both oracles do) — lens targets; a refusal there is a
  correct->loud the slice must not ship.
- N3 split lanes (ER §2.5 last row): after the slice `repeat (fd(2))` = 2 but `localparam N = fd(2); repeat (N)` = 1 (p18),
  where PRE answered 1 for both. PRE already splits run time (m25 = 2) from the binders (m03 = 1); the slice moves the
  statement count to the run-time side its semantics belong to (ER §2.5 row 2). The p18 pin keeps the residue visible.
- N4 frames: p19 changes a suspendable task's repeat from unrolled to a frame-local `$repeat_cnt$`; the three callers share
  one predicate; measure p19 and a function-body twin.

## 8. B — documents (docs commit; recount the Summary by line census, no accumulation)

ROADMAP §2 start-order rows
- 🆕 AE (line 141) -> "- 🆕 AE — the constant interpreter reads a never-assigned 4-state variable as 0 (env
  `BTreeMap<String, i64>`): a return variable (`const_fn.rs:1507`; x2a `P=0000`, d10 `PX=0000 PI=0`; verilator `xxxx` /
  `x`, iverilog refuses the design) or a local declared without a value (`:1335`; x2c `P=0000`, x2i `integer` `P=0`, x2l
  `P=0001`; iverilog and verilator `xxxx`, `x`, `xxx1`), wherever a binder takes the value (§4.5.588 closed the statement
  `repeat` count, which now runs on the run-time loop); IEEE 1800-2017 §6.8 gives x; both binder fixes measured descend
  (§4.5.588): declining an x-bearing result makes 13 correct cells loud (2-state binders' x->0, `& 4'b0000`, `int'()`,
  `$clog2`, generate-if truth) and §2 🆕 AC's sinks turn s01 silent, and keeping only a fully known 4-state re-run (b61
  `if (t == 4'd0)`: PRE `0001`, all oracles `0010`; 18 binder lanes) breaks its cancellation with a still-collapsed
  x-bearing call (p01 `localparam int Q = fd(2) - fx1(2);` PRE and iverilog `0`, re-run `1`; p02 generate-if PRE,
  iverilog, sv2v `E`, re-run `T`), so both halves land together (ER §2.5); fix: a per-bit x plane carried to the binder,
  each binder deciding (2-state x->0, <=64-bit 4-state loud, wide hold); 2 oracles (locals), verilator + hand-IEEE (return
  variable); BLOCKED (§2 row 15's 2-state identity; §2 🆕 AC's sinks refusing an x-valued call; a 4-state result channel
  from the interpreter to module-scope folds)"
- 🆕 AD (line 140): insert before "; first:" — "; a lane this row opens from loud to a value declines a call the AE guard
  marks (`const_fold_x_tainted`, §4.5.588; l22: the instance-array prepass folding `fx(2)` in the child would bind `0000`
  where both oracles print `xxxx`)"; "(§5.2 row 2)" -> "(§5.2 row 1)".
- 🆕 AB (138) row 3 -> 2; 🆕 U (132) 5 -> 4; 🆕 V (133) 6 -> 5; 🆕 W (134) 7 -> 6; 🆕 S retry (129) "(§5.2 row 8)" -> 7.

ROADMAP §2 Constant domain (i64)
- delete "`integer x; g = x + 1;` is 1 (iverilog x); iverilog; OPEN" (DUP: 🆕 AE's x2i).
- add (a) "A select write into a local whose initializer did not fold reads 0 for the other bits (`const_fn.rs:1582`
  `unwrap_or(0)`): a41 `int t = int'(2.5); t[0] = 1'b0;` `P=0`, iverilog and verilator `P=2`; declining there makes the
  twins that read only written bits or overwrite every bit loud (p03 `P=1 Q=0`, PRE = all three oracles); close upward
  (fold the initializer — the `int'(…)` residue above — or track the written bits); 2 oracles; OPEN"
- add (b) "A continuous-assign or net-declaration delay over a constant-function call that read a never-assigned 4-state
  variable folds the 0 (`fold_ca_delay`): `assign #(fd(2)) w = r;` and `wire #(fd(2)) w = r;` delay 1 where iverilog and
  verilator wait 2 (p05b, p08b); `#(fx1(2))` delay 1 where iverilog and sv2v take the x delay as 0, as vita's run-time lane
  does (p10, p12); route it to the run-time lane with the AE guard (§4.5.588), first counting a guard-routed assign as
  delayed in `demote_runtime_delay_on_resolved_nets`, which otherwise drops a resolved net's delay where PRE's folded
  delay is E3001; 2 oracles; OPEN"

ROADMAP §3.b
- `const-fn-case` (470): "so it inherits §2 🆕 AE (a `case` with no `default` leaves the return variable unassigned) and
  🆕 AD (not yet measured on case bodies)" -> "so it inherits §2 🆕 AD (not yet measured on case bodies), and its arm
  declines a run the AE guard marks (§4.5.588; a `case` with no `default` that misses leaves the return variable
  unassigned)"; "BLOCKED (§2 🆕 AE, 🆕 AD)" -> "BLOCKED (§2 🆕 AD)".
- `const-fn-systask` (471): "BLOCKED (§2 🆕 AE, 🆕 AD)" -> "BLOCKED (§2 🆕 AD; the ignore arm carries the AE guard,
  §4.5.588)".
- `unique-if-chain` (553): "(itself BLOCKED on §2 🆕 AE and 🆕 AD)" -> "(itself BLOCKED on §2 🆕 AD)".
- `unique-const-fn` (554): "once §2 🆕 AE and 🆕 AD land: a positive opt-in at constant-required consumers" -> "once §2
  🆕 AD lands: a positive opt-in at constant-required consumers whose arm declines a run the AE guard marks (§4.5.588;
  round 1's d10)"; "BLOCKED (§2 🆕 AE, 🆕 AD; …)" -> "BLOCKED (§2 🆕 AD; …)".

ROADMAP §5.2
- Preamble sentence 3-4 -> "Rows 1–3, what the external report's fix path found (§4.5.583, §4.5.585, §4.5.587), go
  before the ① rows 4–7 because the owner's order of 2026-10-02 keeps the report's items first. §4.5.587 reverted §3.b
  `unique-const-fn` (now BLOCKED): folding the `unique` form like its plain-`if` twin inherits the twin's silent-wrongs.
  Row 1 closes the prefix-binding half; the 4-state half is a guard, not a row: §4.5.588 measured that the interpreter's 0
  for a never-assigned 4-state variable cannot be fixed at the binders yet (§2 🆕 AE, BLOCKED), so every arm or lane a
  later row opens declines a run the AE guard marks; row 3 widens the interpreter the same way and waits on row 1;
  `unique-const-fn` is re-derived from its row once row 1 lands (ER §3.6). Row 2, with `unique-const-fn` and §3.b
  `unique-pkg-closure`, blocks §3.b `unique-if-chain`'s subroutine-body residue. Rows 4–7 are §4.5.581's prerequisites:
  none has a corpus witness and each redesigns shared code."
- Table: delete row 1 (AE). New order: 1 🆕 AD, 2 🆕 AB, 3 `const-fn-case` ("after row 1: a `Stmt::Case` arm in
  `exec_const_stmt` that sizes the case expression and every item once (ER §2.4) and declines a run the AE guard marks,
  then census the newly folded consumers against their plain-`if` twins"), 4 🆕 U, 5 🆕 V, 6 🆕 W, 7 🆕 T then S (a),
  8 new-design census, 9 hygiene (slot "hygiene").

REMAINING_WORK §D
- line 12: "(§2 🆕 AB, §5.2 row 3)" -> "row 2"; "(§3.b `unique-const-fn`, blocked by the x seed below and the
  declaring-scope fold)" -> "(§3.b `unique-const-fn`, blocked by the declaring-scope fold below; its arm carries the AE
  guard)".
- line 13 -> "- A 4-state result channel from the constant interpreter to module-scope folds — `const_eval_in_scope`
  answers `Option<i64>` and the wide walk (`const_wide.rs` `fold_region`) has no call arm, so an x a constant function
  returns reaches every binder as a value; with row 15's 2-state identity (below) and §2 🆕 AC's sinks refusing an
  x-valued call, it lets each binder hold, convert or refuse the x; blocks §2 🆕 AE's binder half (§4.5.588 closed the
  statement `repeat` count and added the AE guard, which §2 🆕 AD and §3.b `const-fn-case`, `const-fn-systask`,
  `unique-const-fn` use instead of waiting on it)."
- line 14 "(§3.b `const-fn-case`, §5.2 row 4" -> "row 3"; lines 15-17 U/V/W rows 5/6/7 -> 4/5/6; line 20 AD row 2 -> 1.
- line 22: "— blocks row 15 and" -> "— blocks row 15, §2 🆕 AE's binder half and".

Summary (recount by census): §2 start-order: AE OPEN -> BLOCKED (startable -1, blocked +1, named prerequisite +1); rung
"frozen; 🆕 AD, AB, U–W, T, S queued"; next "1, 2, 4, 5, 6, 7". §2 recorded defects: +2 lines (a), (b), -1 DUP (open +1,
startable +1). §3.b next 3; study/03 next 8. LOOPROMPT NEXT index: main session (§8).

## 9. Next slice
§5.2 row 1 = §2 🆕 AD (declaring-scope fold over routine text and the instance-array prepass). Its grounding must list
every lane it opens loud -> value and measure the AE guard there (l22, m18), plus its silent -> value lanes (d01/d02:
binding the parent's same-named function) where a guard decline could make a right-by-accident cell loud.

## 10. Not determined
- Whether hand-IEEE §12.7.2 alone is accepted for p07/p13/p14 (recalled clause; iverilog/sv2v hang, verilator 2-state).
- The parser's recorded kind for struct / enum / typedef / type-parameter locals and type-parameter returns (step 0).
- Whether a lens accepts the N3 split-lane argument (p18 vs m19).
- Exact Summary numbers (docs-step recount); intra-assignment `repeat` count lane cells (opted out, not measured).
