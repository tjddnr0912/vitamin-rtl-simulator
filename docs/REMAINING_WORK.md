# vitamin — remaining work (snapshot)

One-screen snapshot of what stands between HEAD and the two goals. The detailed rows are in
[ROADMAP.md](ROADMAP.md); finished work is in [history/](history/README.md). Baseline counts at HEAD:
8927 tests passing with 15 skipped, artifact `format_version` 34 (a `.velab` / `.vu` written by an
older build is refused at the header gate with `E9001`), 70 `MsgCode` diagnostic codes; the
canonical table is the fact table in [README.md](../README.md).

- G1 = a correct open-source RTL simulator (correct-or-loud) at the level of icarus, verilator,
  xcelium and vcs.
- G2 = an AI-agent-friendly simulator (the observability rail; SPEC =
  [preview/19](preview/19-ai-agent-observability.md)).

## A. Status

- Direction (owner, 2026-09-28): real-design first. The queue ranks by the errors a slice removes
  from a workload-corpus design and by digest agreement with that design's oracle, `ibex` first.
  ROADMAP §2's synthetic-probe rows are frozen; a review's pre-existing findings outside a slice's
  fix path go to [PROBE_CATALOG.md](PROBE_CATALOG.md), which is neither queued nor counted.
- Default backend `native`; product build `--no-default-features` (one executor); workload corpus
  11/11, fetched and run by CI on all three platforms since §4.5.579. `ibex`, the only SystemVerilog row, runs to verilator's digest at both sizes
  (`+N=2000` `32e0e78741376133`, `+N=20000` `13b2ddfcd551ba2f`) since §4.5.574 typed its last
  refusal — a keyed `'{…}` as a `?:` arm on a packed-struct port — by its target (§3.a ⑤ⓚ). Its
  page closed in eleven slices (parse errors §4.5.564, then §4.5.565–570, the parser's
  wildcard-import binding §4.5.573, the arm §4.5.574; §4.5.571 and §4.5.572 were reverted).
  Its oracle is verilator, under the x-invariance condition of contract rule 2. Every corpus row
  pins exit 0 since §4.5.576 made a known out-of-range array word (`aes_key_mem.v:182`) the
  warning E4002.
- The performance axis is at diminishing returns and ranks below the correctness ladder: codegen
  (cranelift), 2-state storage, cycle-based mode and levelize are all rejected, each with a recorded
  re-entry condition (ROADMAP §5.a).
- The teeth for anything on the G2 rail that REPORTS is an asymmetric mutation — change something
  upstream that must not move the numbers — because a same-input golden lies identically twice.
- An external report reproduced four items at HEAD. §4.5.580 fixed its silent-wrong at run time: an
  `inside` element with x/z bits compares with `==?` (§11.4.13), and `==?` extends a signed or
  unsized pattern by §11.4.5. Two halves of ROADMAP §2 🆕 S remain: a constant expression keeps
  the old `==` comparison (its i64 half behind §2 🆕 W, its wide half behind §2 🆕 T and 🆕 U; 🆕 T
  was found in the fix path: a generate-case label the i64 fold cannot read is skipped), and an
  element whose x/z bits exist only at run time needs a run-time `==?`. The last slice, §4.5.581, built 🆕 T and 🆕 S (a)'s
  constant half and reverted both after three review rounds; the reviews found three
  prerequisites in shared code, now §2 🆕 U (one current binding per key), 🆕 V (a generate-case
  arm decided once per construct instance, not per phase) and 🆕 W (the sign of a constant typed
  by an overridden type parameter), and the attempt's cells are held in
  `generate_case_and_wildcard_prerequisites.rs`. The other three items are queued: `case …
  inside` refused at parse (§3.b `case-inside`; verilator is a 2-state oracle), a time-0 `unique`
  warning both reference tools
  omit (§3.b `unique-glitch-t0`, with `unique-if-chain`: an `else if` chain with no final `else`
  never reports the no-match) and an unannounced `unique` / `unique0` overlap (§3.b
  `unique-overlap-note`; the check stays a §8 non-goal). Incoming reports pre-empt the queue, so
  they are §5.2 rows 1–3, 🆕 U, 🆕 V and 🆕 W rows 4–6, and 🆕 T then 🆕 S (a) row 7.
- `corpus-runner run` prints the elaborate/simulate split per row. Every workload is ≥99%
  simulation, so the corpus cannot GATE a front-end regression (ROADMAP §5.b `ELAB-PHASE-BLIND`).

## B. Queue (canonical = ROADMAP §5.2; one row per loop iteration)

| # | track | item |
|---|---|---|
| 1 | §3.b `case-inside` | accept `case (e) inside`, reusing `inside`'s comparison (§4.5.580) |
| 2 | §3.b `unique-glitch-t0` + `unique-if-chain` | defer the `unique` / `priority` violation report to the Observed region; report the no-match at the end of an `else if` chain |
| 3 | §3.b `unique-overlap-note` | say once per run that `unique` / `unique0` overlaps are not checked (the check stays a §8 non-goal) |
| 4 | §2 🆕 U | keep one current binding per key at the binders (a genvar, a local enum label or an import that rebinds a key clears or suspends the other map's entry) |
| 5 | §2 🆕 V | decide a generate-case arm once per construct instance and reuse it in every phase, or resolve forward labels the same way in every phase |
| 6 | §2 🆕 W | a constant typed by an overridden type parameter keeps the override's sign (measure §4.5.479's and §4.5.483's lanes first) |
| 7 | §2 🆕 T, then §2 🆕 S (a) | §4.5.581's designs: a generate-case label compared in the bit domain two ways, decided only where they agree; then the constant `inside` / `==?` i64 half with every decline falling back to PRE's compare, then its wide half |
| 8 | corpus | new-design census (OpenTitan IPs, VeeR EL2 / EH1, alexforencich axis / pcie / uart / i2c) |

Priority principle: ① silent-wrong with an oracle > ② loud→supported with an oracle > ③ an
honest-loud promotion whose prerequisite holds > ④ G2 OBS. Performance is below the ladder.

## C. Open items by section (counted from ROADMAP at HEAD)

| section | open | startable / blocked | breakdown |
|---|---:|---|---|
| §0 promotion queue (T2 residues) | 14 rows | 9 / 5 | real const-fold residues ⓐ–ⓔ ⓖ ⓗ, enum-label folding ⓐⓑ, negative bounds (part select / port), the `-G` aliases and the `.velab` header field, `case inside` |
| §2-N verilog-axi census | 2 rows + 3 | 0 / 5 | verilog-axi x-cycle promotion, the FST `$dumpvars` snapshot, and three t0-event residues (§4.5.533 closed the x-valued ones) |
| §2 start-order table (frozen) | 26 rows | 4 / 22 | LOUD 4 · BLOCKED 6 (🆕 S since §4.5.580 closed its run-time half; 🆕 T since §4.5.581, on 🆕 U and 🆕 V) · OPEN 8 (🆕 H, 🆕 U, 🆕 V and 🆕 W startable, 🆕 U–W found in §4.5.581's review; row 14 closed and row 30 re-measured stale in §4.5.556, row 25 closed in §4.5.557; row 26 absorbed by row 14 in §4.5.546; 🆕 F and 🆕 R closed) · ORACLE-SPLIT 4 (row 7 since §4.5.541: the `#d` / `#0` / fork kinds landed, the wake-group and time-0 hierarchy orders are splits) · PERF 2 · DO-NOT-START 2 — the six startable rows were taken in one batch (§4.5.519–524): rows 5 and 🆕 L ⓢ closed, 🆕 I ⓖ, 🆕 N's two spelling cells and 🆕 O's eleven-reader class closed, row 32 re-measured and reclassified ORACLE-SPLIT. §4.5.525 then took the §2 declaration-collision cluster out of the mechanism list (six rows deleted) and §4.5.526 the inline-lane store rules (nine rows deleted), not this table. §4.5.527 added 🆕 R (the shared wide walk inside self-determined positions and on the §11.8.2 sign, WALL), the prerequisite for widening its override arm |
| §2 recorded defects by mechanism (frozen) | 196 bullets | 108 / 88 | inline / frame binds 14 · size cast / signedness 12 · constant domain (i64) 20 · scoping / imports / block-locals 32 · delays / events 19 · real 11 · performance 6 · index sealing 15 · ranges / bounds / selects 8 · diagnostics / artifacts 10 · class fields 4 · oracle splits 45 |
| §3 numbered items | 24 rows | 19 / 5 | ⑤ ibex ladder (9, ⑤ⓚ closed in §4.5.574; ⓕ is the unpacked-array typedef residue), ③ file-I/O hoisting (4), ⑧ system functions in function bodies and `$finish` (4), ⑨ package string/real constants (2), ⑬ diagnostic location (3), ⑭ call-tree observability (2) |
| §3 small residues | 126 rows | 107 / 19 | subroutine / frame 28 (md-return-select: §4.5.564's loud edges) · constants / parameters 29 (gen-enum-uncarried, string-literal-condition-residue and md-param-pattern-residue: §4.5.565's, §4.5.568's and §4.5.569's loud edges) · parser accept 17 · system tasks & file I/O 9 · nets / timing 14 (cont-array-residue, packed-default-residue and cont-array-typedef-residue: §4.5.566–567's and §4.5.570's loud edges) · loud shapes surfaced by §4.5.493–495 7 · strings / heap 8 · diagnostics quality 10 (unique-glitch-t0, unique-if-chain, unique-overlap-note) · VCD / real conversion 3 |
| workload corpus (study/03) | 1 item | 1 / 0 | the new-design census (the corpus runs in CI since §4.5.579) |
| §3 intentionally loud | 12 rows | 0 / 12 | not gaps; each has its reason |
| §4 SVA honest-loud | 6 | 0 / 6 | mostly no oracle; hand-IEEE when started; every row states a prerequisite |
| §5 performance / hardening residues | 15 rows | 7 / 8 | frame-body wprog (5c), native scratch pooling (4b-r), array-LHS cliff, inline-fold exponential, memory guard, quiescence / render / eof seams |
| §6 G2 OBS | 6 stages + 10 | 15 / 1 | OBS-2 residue → OBS-1 residue → R-L4 → OBS-4 control → OBS-5 snapshot → OBS-6 X-origin, plus 10 items beside the staged track (call tree, a `void` function filed as `kind: task`, a route decided per spelling, per-call-site builtins, `builtins` rows for primitives the source never wrote, the staged `--hier-tree` accept-and-drop, generate scopes, enum names, R-I1/R-I2, `wprog` keys with no producer) |
| §7 conditional | 4 | 0 / 4 | BACKEND · VHDL · VCD-EXT · MVP-CUT |
| §8 non-goals | 2 | 0 / 2 | IMPLICIT-NET and the out-of-scope list · `defparam` beyond a direct-child constant target |
| total | 447 | 270 / 177 | |

`startable` = two oracles or a hand-IEEE plan and no unmet prerequisite; `blocked` = a stated
prerequisite (§D), WALL, ORACLE-SPLIT, DO-NOT-START, by design, trigger-gated or non-goal.

## D. Prerequisites that block work from starting

None of these blocks a row on a corpus design's current page; they block frozen §2 rows and
synthetic-origin §3 rows, and they wake with the rows they block.

- A generate-scope alias's recorded type — `localparam C = A;` under a generate block records A's
  width and sign, with a GUESSED type followed through forwarding (a child typed from a guessed name
  is a guess) — blocks §2 "Index sealing"'s sign-keyword bullet; the §4.5.559 attempt made the sign
  right and was reverted after three rounds, each finding an alias the fix turned wrong.
- A declaring-scope fold (a module body, a generate block, `$unit`, a package): every fold of one
  scope's code at another scope's prefix — a function's return, formal and local ranges and its
  defaults, a typedef's range, a constant function's body, a package routine's declarations —
  resolves names where they are declared. Blocks the CALL half of §2 "Real"'s integral-override
  bullet (the constant interpreter folds the range at the call site too, §2 "Constant domain"),
  §2 row 10, the bare-name >64-bit select bullet ("Ranges"), the package-routine constant-domain
  bullet and its `$bits` twin ("Scoping"), the `real`-shadow bullet ("Scoping"), §3.b
  `pkg-string-const-select-dir`, §3.b `gen-enum-uncarried` (a generate block's constants bound
  in scope order, per block, not by position once per phase), and §3.b
  `string-literal-condition-residue` (a generate condition's names resolved by scope — the block
  bindings, block-locals, instance-array segments, block imports and `let`s and a genvar's wide
  twin that §4.5.568's review measured reading an outer object), and §3.b
  `cont-array-typedef-residue`'s named bounds (a typedef's bound, a member type's, and a
  `$bits(T)` the parser folds where it reads it, §4.5.570). Folding a call's
  return range at the call site was built in §4.5.558, letting the select resolvers see >64-bit,
  string and real bindings in §4.5.560, and a package routine's own-constant selects (run time,
  interpreter, concatenation, a range bound at the frame scope) in §4.5.561; each was reverted after
  three review rounds, each finding one more lane that folds another scope's code here — in
  §4.5.561 an imported routine's callee resolved in the importing package, a callee's default folded
  under the caller's package, and a frame binding reached another package's function folded in it.
  §4.5.562's two new arms of the shared region walk (a prim cast, the unselected conditional arm)
  met the same class from the other side: a constant function's body and defaults folded in the
  interpreter's concatenation lane with the caller's bindings, and a `real`-shadow decline fired
  while a callee's ranges and defaults folded at the caller's prefix. §4.5.564's review measured the
  same class on a function return with more than one packed dimension (a typedef's dims, and a
  package function's return range, resolve where the function is used; the one-dimensional twins are
  equally wrong on PRE, `docs/PROBE_CATALOG.md`), and it is the prerequisite for selecting from such
  a return variable when its dims are names (§3.b `md-return-select`).
- The override binder's unknown plane and the parameter lane's 2-state identity (§2 row 15: an
  override carrying `x` / `z` binds 0 or 1, and `ParamDecl` drops `bit`) — blocks §3.b
  `md-param-pattern-residue`'s named items and `bit` elements: §4.5.569's review measured
  `#(.N('bx))` into `'{N, 64'h1}` printing `0…01` where verilator and iverilog keep the `x`.
- A tree-wide AST self-width pass — blocks the size-cast cluster in §2 "Size cast / signedness".
- An exact declared-width fold for hierarchical placeholders (`env_fold` negates a narrow literal in
  i64, `-4'd1` → −1 where the binder gives 15) — blocks widening §4.5.528's placeholder record past
  the exact-by-construction shapes (§2 "Inline / frame binds", the inexact-fold bullet).
- A declared width for array-reduction, string and placeholder cast operands (`ir_bits_of` answers
  `None` for `q.sum()`, a `string` net and a deferred hierarchical call) — blocks the single-mention
  sign extension and 2-state coercion of such an operand (§2 "Size cast / signedness", the
  fabricated-width bullet, and the three per-bit `coerce_two_state` sites under "Performance");
  §4.5.530 carried the declared-width cases only, because `TwoState` over a fabricated width lost the
  32 bits the per-bit `Concat` asserted (`$bits(int'(q.sum()))` 32 → E3009).
- One current binding per key (§2 🆕 U, startable, §5.2 row 4): a genvar, a local enum label or an
  import that rebinds a key in one of `params`, `wide_param_bits`, `str_param_raw` and
  `real_param_val` leaves the other map's entry, and the lanes read different ones (A1D prints the
  65-bit constant for the genvar; q1g's i64 lane reads a wildcard import's 3 for the explicit
  import's 2^64 + 9). Blocks §2 🆕 T and the wide half of 🆕 S (a): §4.5.581's bit-domain label
  lane and wide `==?` arm read the stale entry, and a reader-side skip failed the opposite case.
- A generate-case arm decided once per construct instance (§2 🆕 V, startable, §5.2 row 5): the
  four elaboration walks re-decide the construct, and a label referring forward binds differently
  by phase (d1p takes one arm's nets and another's process today). Blocks §2 🆕 T: §4.5.581's
  per-phase bit-domain decision mixed arms on d1w, r1 and r1b, and caching the region's
  availability did not stop it.
- The sign of a constant typed by an overridden type parameter (§2 🆕 W, startable, §5.2 row 6):
  `PV < 0` is 0 and `localparam T TP = '1` is 15 under a signed override, where all three oracles
  say 1 and -1. Blocks 🆕 S (a)'s i64 half, which folded `PV ==? 4'sb1?00` to 0 where HEAD is loud;
  that half also needs every decline of its routine to fall back to PRE's own-width compare.
- A block-scoped CONSTANT binding — blocks §2 🆕 Q; a bare-name hoist makes 6 cells correct and 5
  new silent-wrongs.
- A field-key normalisation map — blocks §2 row 3b; the map is keyed by NetId and a class field is
  not a net.
- Per-instance declarator arity — blocks §3 ⑤ⓕ's dim-COUNT axis; it needs a symbolic arity marker on
  two SchemaHash root types.
- Per-instance class registration — blocks §3 ⑤ⓕ's class-property container; `register_classes` is a
  whole-design prescan, so a property of type `T` is loud with no override at all.
- A binding-resolved scope (which declaration a post-block reference binds, not a name) — blocks
  three §2 "Scoping" bullets (a package routine body's read after a shadowing block, the scoped
  `pk::g()` scope-leak gate, the inline-fold lane's by-name context) and the §3 false-loud row the
  same gate owns; each narrowing keyed on the declaration's properties created a new defect. The
  parser's name-keyed struct bindings are the same gap for assignment patterns (the §4.5.571 CLASS
  row in docs/PROBE_CATALOG.md); §3.a ⑤ⓚ did not wait for it — §4.5.574 records each
  declaration's members and types the arm in elaborate, on the net the name resolves to, since the
  parser's type binding AT a declaration no longer loses to a wildcard import (§4.5.573).
- Type bindings that survive a nearer rebinding of their NAME — a variable's struct / enum type
  (`var_struct`, `var_enum`) and a layout's nested member keys are the bare type name, re-resolved
  at every use, so a nearer scope that rebinds the name retargets them (a block typedef already
  does: `GT0 x5` for `5`) — and a package's complete type twins (the endpackage pass writes a
  struct / enum twin only when the body's table entry DIFFERS from the pre-body one, so a package
  typedef identical to an outer one has none). Block the three PROBE_CATALOG rows §4.5.573 built
  and reverted: a container's wildcard over a unit-scope import or class, a generate block's
  repeated import over the module's own type name, and a replacement that clears the replaced
  binding's other-kind entries (§4.5.573 review round 2: `GV1a x5` for `5`, `GV2 PY` for `B`,
  `s.a` E3010 and `e.name()` E3009 where all three oracles run).
- The adjudication of the diagnostic stream a purity certification moves (a pure RHS reports
  `errors=5`, the same RHS inside a no-op `$unsigned` `errors=9`) — blocks the §2 "Performance"
  certification bullet and the two bullets with its root.
- One oracle and zero corpus demand — blocks clocking (§2 rows 23, 24, 34).
- An oracle split on the boundary itself — blocks §2 row 32 (`$finish` / `$fatal` reached inside a
  function body: iverilog stops where verilator runs on, and neither side can be taken without
  contradicting the other on the sibling spelling).
- Oracle splits are recorded, never chased.
