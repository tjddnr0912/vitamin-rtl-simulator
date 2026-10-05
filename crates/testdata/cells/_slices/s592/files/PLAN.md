# §4.5.592 start decision + implementation plan — §2 🆕 V
status: completed (Q0–Q5 answered; every number below is from a run in this session or quoted from GROUNDING.md)

Binaries (frozen, never rebuilt):
- PRE   $S/s592/pre/vita            md5 86a2d84a21d0329c57541e51ae9eb4d2 (main 75f46453, release)
- P6    $S/s592/probe_p6/vita  = bin_p6 md5 f6052d7ce91fc7e2a9d5632ee56204ce + P6_DECIDE=1 P2_NOPREBIND=1 P5_NOCACHE=1 P2_NOVERIFY=1
- D1    $S/s592/probe_d1/vita  = same bin_p6 + P2_NOPREBIND=1 only (no view; Nets record + verify-loud + replay of the record; = ER §2.6 "second pass verifies the recorded first result")
- cache $S/s592/probe_cache/vita = bin_p5 + P2_NOPREBIND=1 P2_NOVERIFY=1 (Nets decision replayed silently = positional decisions)
Harness: $S/s592/g/run.py; new cells $S/s592/g/c/Z*.sv (43); raw output c/<id>.<tool>.txt.
Classifier: $S/s592/g/model_cmp.py (PRE→X transition per oracle model), residue.py, mdrows.py.
IEEE text: $S/s592/lrm/lrm.txt (pypdf extraction of IEEE 1800-2017, 1315 pages).

## Intent
Every elaboration walk must agree on each generate construct's decision, so a forward-referenced name can never silently
mix arms (the prerequisite §4.5.581 named for 🆕 T); each forward-reference cell lands on the IEEE answer or an honest
refusal, never on a new silent value.

## Decision
- P6 (scope-wide decision view): NO-GO. It descends under BOTH oracle models (table "Model tallies"): under the
  grounding's own sv2v+verilator model 4 cells go loud→silent-wrong (Z1a, Z1b, Z1g, Z1h: 🆕 AE / 🆕 AC through the
  newly decided arm) and it removes a PRE error iverilog AND verilator report (Z2a, Z2b); under IEEE §26.3 it has
  0 rungs up, 18 descents (16 correct-loud→accept, F1 + Z1g loud→silent-wrong) and 34 sideways moves.
- NARROWER = D1 (record the Nets decision per construct instance, verify it in the three later walks, refuse loudly on
  a mismatch and replay the record): GO. Under IEEE: 45 rungs up, 0 down, 7 sideways (loud→loud with a correct
  message), 97 unchanged. Under sv2v+verilator: 0 silent descents; 7 cells go OK→loud (F3 I4 M4 V04 V24s Z1f Z5m),
  each a cell where PRE printed the sv2v/verilator value while vita's own Nets walk had chosen iverilog's arm.
- Not "closing by refusing" (ER §2.2): the refusal is the IEEE outcome for an illegal forward reference (no outer
  object); for a legal shadowed one the value half is filed as its own row with its prerequisite (🆕 AE, Z1g).
- Start condition (ER §10.2) for D1: MET — every lane measured (lane table in Q5); staged / corpus byte-identity /
  elab-time are re-measurements of a by-construction identity, owed in the implementation steps.
- The one judgment the main session must make: adopt the Q0 ruling (iverilog + IEEE §26.3 decide forward references;
  sv2v and verilator are not oracles there). Without it, D1's 7 OK→loud cells are regressions under the old model;
  no variant measured (P6, P4, cache, D1) has zero descents under the old model.

## Q0 — oracle split: is iverilog-lexical vs sv2v/verilator-scope-wide a genuine split?
No. IEEE 1800-2017 decides it, for iverilog.
- §26.3 (p. 778, normative): "An identifier is locally visible at some point within a scope if a) The identifier
  denotes a nested scope within the current scope, or b) The identifier is declared as an identifier prior to that
  point within the current scope, or c) The identifier is visible from an explicit import prior to that point within
  the current scope." … "For a reference to an identifier other than function or task call, the locally visible
  identifiers defined at the point of the reference in the current scope shall be searched. If the reference is a
  function or task call, all of the locally visible identifiers to the end of the current scope shall be searched."
  … "If the reference is not bound within the current scope, the next outer lexical scope shall be searched" …
  "For a reference to an identifier other than function or task call, it shall be illegal if no identifier can be
  found that matches the reference."
- §26.3 Example 1 is this exact shape (a generate block reading `x` before its own `int x`): "line 2 initializes
  p::x. Line 4 initializes top.b.x."
- §6.5 (p. 86): "Data shall be declared before they are used, apart from implicit nets (see 6.10)."; §6.2: "A data
  object is a named entity that has a data value and a data type associated with it, such as a parameter, a
  variable, or a net."; §6.20: "Constants are named data objects"; §27.2: "Parameters declared in generate blocks
  shall be treated as localparams"; §27.4: "The loop index variable shall be declared in a genvar declaration prior
  to its use". §6.20.2 and §27 state no exception.
- Measured on the LRM's own example (Z0a, verbatim minus the illegal line 5; LRM answer px=1 bx=2):
  iverilog `px=1 bx=2` · sv2v `px=z bx=2` · verilator `px=0 bx=2` · PRE `px=0 bx=2`.
  Parameter twin Z0b: iverilog `l2 X=1 / l4 X=2`; sv2v, verilator, PRE `l2 X=2 / l4 X=2`.
  Decision twin Z0c: iverilog `pkg 9 bits=4`; sv2v, verilator `local 200 bits=8`; PRE `local 8 bits=4` (mix).
- Ruling: binding is positional. No outer object → illegal (iverilog: "Unable to bind parameter `K' … Check for
  declaration after use"); an outer object → it binds (iverilog's value). sv2v and verilator implement a scope-wide
  model the standard excludes and contradict its example: disqualified on this axis (ER §7.3: the spec decides,
  never a majority). Repo precedent agrees: `iface_subr_scope.rs` answers the same split with honest-loud.
- Consequences: the row's "2 oracles" (sv2v + verilator) is a majority of disqualified tools; d1p's correct outcome
  is a refusal; the grounding's "global pre-bind" residue is inverted (positional Nets widths are IEEE-correct:
  N2p PRE `bits=4 w=15` = iverilog); C1's `P=10000000000000005` token is the IEEE value (iverilog prints it).

## Q1 — P6's 8 loud→value cells, varied (arm contents reaching 🆕 AE / 🆕 AC / hierarchy / instances)
| cell | iverilog | sv2v | verilator | PRE | P6 | D1 | cache |
|---|---|---|---|---|---|---|---|
| Z1a scr fwd, arm `localparam P = fx(2)` | ERR "Unable to bind parameter `S'" | k P=xxxx bits=8 | k P=xxxx bits=8 | ERR E3010 | k P=0000 bits=8 | ERR | ERR E3010 |
| Z1a2 same, backward | k P=xxxx bits=8 | k P=xxxx | k P=xxxx | k P=0000 | k P=0000 | k P=0000 | k P=0000 |
| Z1b scr fwd, arm `{fc(2){1'b1}}` | ERR | k r=7f | k r=7f | ERR E3010 | k r=00 | ERR | ERR |
| Z1b2 same, backward | k r=7f | k r=7f | k r=7f | k r=00 | k r=00 | k r=00 | k r=00 |
| Z1c scr fwd, module reads `gb.g.w`, `$bits(gb.g.w)` | ERR | h w=200 bits=4 | h w=200 bits=8 | ERR E3010 | h w=200 bits=8 | ERR | ERR |
| Z1d lbl fwd, arm `localparam Q = K + 1` + instance | ERR | k o=100 | k o=100 | ERR E3010 | ERR E3009 (Q) | ERR | def |
| Z1e lbl fwd, arm instance `#(.P(7))` | ERR | k o=7 | k o=7 | ERR E3010 | k o=7 | ERR | def |
| Z1f for shadow, instances only | L[0] P=10 | L[0..2] | L[0..2] | L[0..2] | L[0..2] | ERR | L[0..2] |
| Z1g for shadow, body `localparam P = fx(i)` | L0 w=10 P=xxxx | L0..L2 P=xxxx | L0..L2 P=xxxx | ERR E3010 | L0..L2 P=0000 | ERR | ERR E3010 |
| Z1h if fwd, then-arm `{fc(2){1'b1}}` | ERR | then r=7f | then r=7f | ERR E3010 | then r=00 | ERR | ERR |
| Z1i V14b + arm `fx(1)` | ERR | k P=xxxx | k P=xxxx | k P=0000 | k P=0000 | ERR | NONE |
Finding: P6's decision view hands the newly chosen arm to the constant interpreter and to 🆕 AC's sinks; the
backward twins (Z1a2, Z1b2) are wrong on PRE, so P6 inherits them as loud→silent-wrong under every model. The view
also stops at the decision: the arm's own `Q = K + 1` stays unbound (Z1d), so P6 chooses an arm with a `K` the arm
cannot read. D1 keeps every one of these loud (and Z1i goes silent-wrong→loud).

## Q2 — quiet binder: can P6 hide a PRE-loud error the oracles also refuse?
| cell | iverilog | sv2v | verilator | PRE | P6 | D1 |
|---|---|---|---|---|---|---|
| Z2a shadow fwd label; IEEE arm has `wire [Nope:0]` | ERR "Unable to bind parameter `Nope'" | k 200 | ERR "Can't find definition of variable: 'Nope'" | ERR E3009 | k 200 (exit 0) | ERR |
| Z2b same, IEEE arm declares `w` twice | ERR "'w' has already been declared" | k 200 | ERR "Duplicate declaration of signal" | ERR E3009 | k 200 (exit 0) | ERR |
| Z2c fwd `localparam logic [7:-1] K` | ERR (fwd) | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | def 9 bits=4 | same |
| Z2d shadow fwd `K = 4'b10x1` | def 9 bits=4 | def | def | ERR E3009 | ERR E3009 | same |
| Z2e shadow fwd `K = 8'd99 / 0` | def 9 bits=4 | def | def | ERR E3009 | ERR E3009 | same |
Finding: the binder itself hides nothing — the positional registration still runs `check_param_decl_range` and the
unfoldable report in Nets (Z2c–Z2e identical to PRE). But P6 changes which arm Nets builds, so an error inside the
arm iverilog and verilator elaborate disappears: correct-loud → silent on Z2a, Z2b (2 oracles refuse). D1 keeps them.
(Z2d/Z2e's E3009 is §2 row 15's x-plane line, pre-existing.)

## Q3 — the exact set at the i64 / wide / sign boundary
| cell | iverilog | sv2v | verilator | PRE | P6 |
|---|---|---|---|---|---|
| Z3a fwd `K = 4'h1 << 4` / Z3a2 backward | ERR / def 9 bits=4 | def / def | def / def | def / def | def / def |
| Z3b fwd `K = 32'hFFFF_FFFF + 1` / Z3b2 bwd | ERR / def 9 bits=4 | def / def | zero / zero | ERR E3010 / zero | zero / zero |
| Z3c fwd `logic signed [71:0] K = -72'sd3` / bwd | ERR / k 200 | k 200 / k 200 | k 200 / k 200 | k 8 bits=4 / k 200 | k 200 / k 200 |
| Z3d fwd `case (K[64:0])`, 128-bit K / bwd | ERR / k 200 | k 200 / k 200 | k 200 / k 200 | ERR E3010 / ERR E3010 | ERR / ERR |
| Z3e fwd `K = 4'sb1000 >>> 1; if (K < 0)` / bwd | ERR / neg 200 | neg / neg | neg / neg | ERR E3010 / neg 200 | neg / neg |
| V06_C1 (wide outer Q, narrow inner Q, `P = Q`) | def 9 bits=4 P=10000000000000005 | p 200 P=…03 | p 200 P=…03 | p 8 bits=4 P=…05 | p 200 bits=8 P=10000000000000005 |
Finding: P6's forward value equals its backward twin in every cell (twin-equal: Z3b inherits verilator's side of an
iverilog lossless-unsized split, Z3d inherits PRE's wide-select loud). No operator broke beyond the twin, but V06_C1
shows P6 is internally inconsistent: the arm is chosen from the scope-wide `P = 3`, the printed token is the outer
wide value — a combination no tool produces. Moot for D1 (no binder, no exact set).

## Q4 — elaboration time
- P6: the view re-runs a fixpoint binder per decision (per for-iteration condition); unoptimised suite 68.2 s vs
  51.9 s (GROUNDING). Safe caching if ever revived: the exact set is a pure function of (level AST, bound genvar
  names) → once per level entry; the fixpoint values depend on genvar values → per level entry per walk, shared by
  every decision in the level; skip the view when a decision input names no forward exact parameter (structural,
  per AST node). Not pursued (NO-GO).
- D1: one key (prefix clone + kind + span) and one map insert per construct instance in Nets, one lookup in each later
  walk; no fixpoint, no view. P1 (the same record/compare on the PRE tree, debug, with logging) ran the suite in
  51.933 s — the baseline the grounding set P6's 68.2 s against.
  Obligation: release A/B of corpus elaboration time, PRE vs POST interleaved, both orders, first run discarded
  (ER §8.1); ±3% = no change. Expected: no change.

## Model tallies over 149 cells (106 grounding + 43 audit), PRE → X
IEEE model (iverilog decides; a refusal of an illegal design is the correct outcome):
- P6: same 97 · DOWN LOUDok→ACCEPT 16 (F2 I1 V03 V08c V12f V14b V17 Z1a Z1b Z1c Z1e Z1h Z2a Z2b Z3b Z3e) ·
  DOWN LOUD→WRONG 2 (F1 Z1g) · SIDE ACCEPT→ACCEPT 21 · SIDE WRONG→WRONG 12 · SIDE 1 (Z1d) · UP 0
- D1: same 97 · UP ACCEPT→LOUDok 27 (AE3 V01 V08 V08b V09 V09b V10 V10e V12a V12b V12e V13 V16 V18 V19 V20 V21 V22
  V23 V24 V24c V24s V26 V30 V31 Z1i Z3c) · UP WRONG→LOUD 18 (E09 F3 I2 I3 I4 M4 V04 V04x V05c V05d V06 V06n W1 Z0c
  Z1f Z5h Z5m Z5n) · SIDE loud→loud 7 (AE7 V14b V17 Z1d Z1e F1 Z1g) · DOWN 0
- cache: same 100 · UP WRONG→OK 11 (E09 I2 I3 I4 V04 V04x V05c V05d V06 Z0c Z5h) · DOWN LOUDok→ACCEPT 5 (AE7 V14b V17
  Z1d Z1e: accidental louds become a silent default) · SIDE 30 · n/a 3
sv2v+verilator model (the grounding's):
- P6: UP 40 · DOWN LOUD→WRONG 4 (Z1a Z1b Z1g Z1h) · n/a 6 (incl. Z2a Z2b, where iverilog + verilator refuse) · SIDE 2
- D1: UP WRONG→LOUD 35 · DOWN OK→LOUD 7 (F3 I4 M4 V04 V24s Z1f Z5m) · SIDE 6 · n/a 4 · silent descents 0
Raw D1 lines: V01 `P2-MISMATCH generate case decided differently by two elaboration walks (Nets Some(2), now Some(1))`;
F1 `… generate for … (Nets [0], now [0, 1, 2])` plus 4 cascaded E3010 (probe does not replay a for-loop);
M4 `(Nets Some(0), now Some(1))` then `now Some(2)` (the chain converges one link per walk).
P1 census of the same comparison (GROUNDING): suite 1088 DECIDE / 3260 CMPOK / 3 MISMATCH = one design (d1p pin);
corpus 343 / 1029 / 0; examples 0 decisions.

## Q5 — implementation plan (D1)
### Files and functions
- New `crates/elaborate/src/gen_decision.rs` (generate.rs is 1,045 lines; ER §10.1 sibling-module precedent):
  `enum GenDecision { Case(Option<usize>), If(bool), For(Vec<i64>) }`; key `(String /*cur_prefix*/, kind, span.lo,
  span.hi)`; `fn gen_decision_check(&mut self, kind, span, phase, now: GenDecision) -> GenDecision`:
  Nets → insert, return `now`; later walk → no record ⇒ `now` (PRE); equal ⇒ `now`; different ⇒ report once per key,
  return the record. Record only a decision Nets completed without a diagnostic (an arm index, a branch, an iteration
  list); an unfoldable scrutinee/condition/init/step, the unroll cap or the stall guard is reported in Nets as today
  and leaves no record, so those designs keep PRE's later-walk behaviour (the probe recorded an unfoldable `if` but
  not an unfoldable `case`; unify on "no record").
- `crates/elaborate/src/lib.rs` / `driver.rs`: fields `gen_decided: BTreeMap<Key, GenDecision>`,
  `gen_decision_reported: BTreeSet<Key>`.
- `crates/elaborate/src/generate.rs`: `GenItem::Case` → factor PRE's scan verbatim into `gen_case_scan` (keep from
  P6) returning the item index, then `gen_decision_check`; `elaborate_gen_if` → check the truth per chain link;
  `GenItem::For` → in Nets record the genvar values iterated; in a later walk compare step by step and, on the first
  divergence, report and iterate the recorded values (no E3010 cascade).
- Diagnostic: E3010 (ElabUnresolvedName), the module lane's phrase so both lanes read alike: "`K` is used before it
  is declared in this generate block (declared at L:C), so vita's elaboration passes choose different
  {case items|branches|iterations} here; IEEE 1800-2017 §26.3 binds a name only to declarations before the
  reference — declare it above the generate construct." The name: bare identifiers of the decision input that an
  enclosing generate level (or module-level region) declares as a parameter after the construct, found structurally
  from the module AST by span containment, computed only on a mismatch. None found (a chain such as M4) → "this
  generate {kind} depends on a parameter declared after it …". Caret on the name's use, else the construct.
- Keep from P6: `gen_case_scan`; the record/verify idea of `gen_decide` (rewritten typed, deduped, replaying for-loops).
  Drop: `with_decision_view`, `gen_exact_expr`, `gen_has_call`, `gen_prebind`, `take/restore_param_key`,
  `ParamKeySaved`, `gen_prebinding` (and its gate on `check_param_decl_range`), `gen_prebound`, `gen_levels`,
  `gen_module_region_decls`, `vprobe_fwd`, the instance.rs region block, every env var.
### Lane table (ER §10.2)
| shared function / lane | consumers | status |
|---|---|---|
| `GenItem::Case` record + verify | label, scrutinee, per prefix / iteration / instance | measured: D1 probe 149 cells; P1 suite + corpus census |
| `elaborate_gen_if` (+ else-if links) | condition | measured: I1–I4 V12f V28c Z1h Z3e |
| `GenItem::For` iteration list | unroll count, genvar values | measured: F1 F2 F3 Z1f Z1g V09 V21 E06 C01 Z5n |
| key (prefix, kind, span) | instances, arrays, loop iterations | measured: V07 V10 V10e V12a–f E06 C01 (E06/C01 differ per iteration) |
| no-record path (Nets returned early) | PRE behaviour | measured: V03 F2 I1 V08c (identical to PRE) |
| new E3010 text | stderr, diagnostics rail | pinned by the new tests |
| staged vcmp → velab → vrun | same function in velab | owed: step 5 |
| corpus / examples byte identity | | by construction + P1 (0 mismatches); owed: step 7 |
| elaboration time | | owed: step 8 |
### Byte-identity argument
Where every later walk's decision equals the Nets record (P1: 3260/3263 suite comparisons, 1029/1029 corpus), the
check returns the computed decision, emits nothing and writes only `gen_decided` (never serialized): traversal, IR,
nets, processes and diagnostics are PRE's. Non-vacuity: 1088 suite / 343 corpus records written, 1 suite design fires.
`format_version` unchanged (no IR change).
### Tests (deciding oracle: IEEE 1800-2017 §26.3 + iverilog; sv2v / verilator lines kept as the disqualified side)
New file `crates/cli/tests/generate_decision_walks.rs` (name free), each pin with the three raw oracle lines:
1. V04x shadow case with nets → refused E3010 naming `S`; iverilog `one 1 bits=6`; PRE `k 8 bits=6`.
2. I4 shadow if, no nets → refused; iverilog `else`; sv2v/verilator/PRE `then`.
3. F3 shadow for, no nets → refused; iverilog `L0`; PRE `L0 L1 L2`.
4. F1 shadow for with nets → refused with exactly ONE error (no E3010 cascade); iverilog `L0 w=10`.
5. M4 chain through a backward parameter → refused (fallback wording); iverilog `seven P=7`; PRE `five P=5`.
6. Z0c (§26.3 Example 1, decision form) → refused; iverilog `pkg 9 bits=4`; PRE `local 8 bits=4`.
7. V24 module-level region → refused; iverilog refuses; PRE `k 8 bits=4`.
8. V10e instance array → refused per element.
9. Z1g → refused (witness that the positional value half reaches 🆕 AE: iverilog `L0 w=10 P=xxxx`).
Controls, byte-identical to PRE: 10. V05a `def 9 bits=4` (= iverilog; the later declaration does not change the
decision) · 11. E06 `L0 def / L1 two / L2 def` (all three; per-iteration decisions) · 12. V12c (two instances,
different arms; all three) · 13. Z5f lazy forward label after the match `one 1 bits=8` (all three) · 14. V14
`def 9 bits=4` (consistent forward label: iverilog refuses, sv2v/verilator accept — reject-vs-accept, out of scope).
### Pin conversions
- `generate_case_and_wildcard_prerequisites.rs` `p2_a_forward_referenced_matching_label_mixes_arms`: `D1P k 8 bits=4`
  → refused (exit 1, E3010, `K`, no `D1P` line); keep the three oracle lines; rewrite the header's P2 bullet ("P2 a
  generate-case arm chosen identically in every elaboration phase": closed by refusal, §4.5.592; the value half is the
  new row). `p2_a_forward_label_does_not_change_the_other_labels` stays `D1W def 9 bits=4` (🆕 T; phase-consistent).
### Mutants (expected outcome first; battery on the full workspace, ER §7.4)
M1 no Nets record → pins 1–9 silent (killer 1) · M2 verify replays without reporting (= cache) → pin 1 prints
`D1P def 9 bits=4`, pin 2 `else` silent (killers 1, 2) · M3 report but keep the later decision → pin 4 shows the
E3010 cascade (killer 4) · M4 key without prefix → pin 11 false refusal (killer 11; E06's iterations decide
differently) · M5 skip If → killer 2 · M6 skip For → killers 3, 4 · M7 compare Some/None only → killer 1 (both Some)
· M8 no dedupe → killer 4 (error count) · M9 record in VarInit → pin 1 silent `k 8 bits=4` (killer 1).
### Steps (each with its verification)
1. Worktree from main (own CARGO_TARGET_DIR), implement. Verify: `cargo build -p cli --locked`; D1 harness column.
2. Re-run the 149 cells as tool `post`. Verify: `model_cmp.py '.*' post` = PRE→POST IEEE UP 45, DOWN 0, same 97,
   SIDE 7; F1 and Z1g print one error each.
3. Tests + pin conversion; `cargo fmt --all`. Verify: `cargo nextest run -p cli --test generate_decision_walks
   --test generate_case_and_wildcard_prerequisites`.
4. Mutant battery M1–M9. Verify: each dies on its named pin; survivors re-run at `--workspace`.
5. Staged (`--features separate-bins`). Verify: the 52 changed cells and the five control pins give the same exit
   code and diagnostics through vcmp → velab → vrun as one-shot.
6. Gate: `cargo nextest run --workspace --locked` + `cargo test --doc --workspace --locked`; fmt; `cargo clippy
   --workspace --all-targets --locked -- -D warnings`. Verify: 0 fail, pass count +new pins, format_version unchanged.
7. Corpus: `cargo run -p corpus-runner --locked -- run`. Verify: no REGRESSION/DRIFTED/ORACLE-DRIFT; corpus 11 +
   examples 4 `.vu`/`.velab` byte-identical PRE vs POST (30 artifacts).
8. Elab-time A/B (release, interleaved, both orders). Verify: within ±3%.
9. Two lenses on a frozen POST. Soundness premises to census: every arm decider is `elaborate_gen_item` /
   `elaborate_gen_if` (GROUNDING Q1: only those); the four walks traverse one tree when decisions agree; key
   uniqueness per instance and iteration; the no-record path is PRE.
10. Docs (rows below), commit with the `§4.5.592` anchor. §4.5.590 is sim-engine only: disjoint, bundle-able.
### Risks
- Owner sign-off on the Q0 ruling (7 cells move OK→loud under the old model).
- False refusal where two arms differ only in Nets-phase content and print the same (contrived; 0 in suite/corpus).
- For-loop replay must not cascade (pin 4); a Nets-unrecorded construct keeps PRE (cascades there are PRE's).
- 🆕 T's re-attempt must decide in Nets or from walk-invariant inputs; D1 turns a T-induced mix into a refusal.

## Rows to file
- §2 🆕 V: delete (every mix is refused). §5.2 row 7: delete; recount. §2 🆕 T: BLOCKED (🆕 U's held half) only; note
  that its chooser is verified per walk by §4.5.592.
- New §2 line (next id, "Scoping"): later walks bind a generate-scope name declared after the reference to the
  previous walk's declaration — reads (N2 `K=8 bits=4`, B1 `K=8`, Q65 `P=11 bits=3`, P1c `K=8 v=8 P=8`, W2
  `K=10000000000000007`, Z0b `l2 X=2`; accepted where iverilog refuses: Z5a `K=8`, Z5b `P=8`, Z5c `v=8`, Z5d `w=8`)
  and, refused since §4.5.592, decisions (I4, V04, F1, M4, Z0c); IEEE §26.3 binds by position (iverilog `K=4 bits=4
  w=15`, `K=4`, `P=2 bits=3`, `K=4 v=4 P=4`, `K=05`, `l2 X=1`; refuses Z5a–d; `else`, `one 1 bits=8`, `L0 w=10`,
  `seven P=7`, `pkg 9 bits=4`); fix: restore each generate level's own parameter keys at its entry in every walk after
  Nets (module-region keys only where the pre-Nets entry was empty: V24s keeps an import); must stay: Z5i, Z5j
  hierarchical reads (all three `K=5`); reads half OPEN (iverilog + hand-IEEE); decisions half BLOCKED (🆕 AE: Z1g).
- PROBE_CATALOG (outside the fix path):
  · module scope binds a later parameter: V01m `k 200 bits=8` (iverilog refuses), N2m `bits=8 w=255` / N2m2 `def`
    (iverilog `bits=4 w=15` / `four`, the `$unit` K), Z5l `K=5` (iverilog refuses); body params bound before all walks
    (`instance.rs:612-700`).
  · generate-scope VARIABLE twin of §26.3 Example 1: Z0a PRE `px=0 bx=2` (= verilator), LRM + iverilog `px=1 bx=2`.
  · E02: `$bits(v)` in a block localparam reads the outer `wire [3:0] v` though the block declares `wire [7:0] v`
    first: vita `def 9 bits=4 P=4`, all three `eight 200 bits=8 P=8`.
  · V15: hierarchical label into a later block `gb.K` accepted `def 9 bits=4`; iverilog and sv2v "A hierarchical
    reference (`gb.K') is not allowed in a constant expression", verilator internal error.
  · Z5g: a generate-case label naming an undeclared identifier is skipped `def 9 bits=4`; iverilog/sv2v "Unable to
    bind parameter `NOPE'", verilator "Can't find definition of variable: 'NOPE'" (🆕 T / 🆕 AC label sink).
  · Z3d: `case (K[64:0])` over a 128-bit generate localparam is E3010; all three `k 200 bits=8` (loud, safe).
  · consistent forward references accepted where §26.3 and iverilog refuse (V14 `def`, V27 `def`, V30b `def`, Z2c,
    Z3a; sv2v/verilator accept): reject-vs-accept, recorded with the ruling.
- Not filed: the grounding's "global pre-bind (P4) closes Nets-only widths / chains, BLOCKED on 🆕 AE" (under §26.3
  positional Nets widths are correct and the pre-bind would descend); M1 (its E3009 = iverilog = §26.3); C1 with
  🆕 U-b (C1's token is the IEEE value; C1 is refused as a mix).

## Not determined
- IEEE 1800-2023's text (only 2017 read); commercial simulators' behaviour (not runnable here).
- A D1-mode workspace suite run (P1's census is the evidence; step 6 measures it).
- Staged and corpus byte identity for D1 (steps 5, 7).
