# vitamin — remaining work (snapshot)

One-screen snapshot of what stands between HEAD and the two goals. The detailed rows are in
[ROADMAP.md](ROADMAP.md); finished work is in [history/](history/README.md). Baseline counts at HEAD:
7928 tests passing with 15 skipped, artifact `format_version` 31, 70 `MsgCode` diagnostic codes; the
canonical table is the fact table in [README.md](../README.md).

- G1 = a correct open-source RTL simulator (correct-or-loud) at the level of icarus, verilator,
  xcelium and vcs.
- G2 = an AI-agent-friendly simulator (the observability rail; SPEC =
  [preview/19](preview/19-ai-agent-observability.md)).

## A. Status

- Default backend `native`; product build `--no-default-features` (one executor); workload corpus
  10/10 with 0 rejections.
- The performance axis is at diminishing returns and ranks below the correctness ladder: codegen
  (cranelift), 2-state storage, cycle-based mode and levelize are all rejected, each with a recorded
  re-entry condition (ROADMAP §5.a).
- The teeth for anything on the G2 rail that REPORTS is an asymmetric mutation — change something
  upstream that must not move the numbers — because a same-input golden lies identically twice.
- `corpus-runner run` prints the elaborate/simulate split per row. Every workload is ≥99%
  simulation, so the corpus cannot GATE a front-end regression (ROADMAP §5.b `ELAB-PHASE-BLIND`).

## B. Queue (canonical = ROADMAP §5.2)

| # | track | item |
|---|---|---|
| 1 | §2 silent-wrong | `$size(q)` / `$size(da)` of a queue or dynamic array answers the ELEMENT width (`32` against both oracles' `3` / `2`); the introspection fold sees only the packed element dim of a handle net and folds a constant where the answer is a runtime size |
| 2 | §2 silent-wrong | A user CALL as a leaf of a §11.6.1 region stands the whole region down (`id8(a8) * b8` is `00000001` against both oracles' `0000fe01`); the size-cast sign walk's `Call` arm is the last `_ => None` leaf after §4.5.495 |
| 3 | §3 loud → correct-support | A user-call ACTUAL in an `always_comb` rhs beside a declaration initializer is a false-loud E3001 (`id8(u8) * b8`; both oracles print `f609`); Rule A counts every actual as a write where verilator counts `output`/`inout` actuals only |
| 4 | §6 OBS | Give the static `subroutines` rows a declaration site, so the two subroutine objects can be joined |
| 5 | §6 OBS | `WPROG-WHY`: nothing says why an EXPRESSION left the compiled lane, so a reader infers the boundary from builtin call counts and gets it wrong |
| 6 | §3 loud → correct-support | ⑤ⓕ residue: the 2-state axis of a `T'(e)` cast and a packed struct member (the sign follows the override since §4.5.483), the enum base (blocked behind a §2 enum-storage row), the union member's parse gate, a multi-dimensional packed type-param default or override, a mixed-caller callee, `m #(8)` / `defparam u.T$w`, the VCD `$scope` spelling, a `genblk<N>` collision (split) |
| next | — | the scoped lane's ungated scope leak and the name-keyed inline context beneath it (one prerequisite, a binding-resolved geometry), a package static shared across importing modules, the static initializer that reads a formal (1-oracle), §2 🆕 L ⓦ residue, §2 🆕 N residue, a labelled concurrent `assert property` action block's `%m` |

Priority principle: ① silent-wrong with an oracle > ② loud→supported with an oracle > ③ an
honest-loud promotion whose prerequisite holds > ④ G2 OBS. Performance is below the ladder.

## C. Open items by section (counted from ROADMAP at HEAD)

| section | open | breakdown |
|---|---:|---|
| §0 promotion queue (T2 residues) | 14 rows | real const-fold residues ⓐ–ⓔ ⓖ ⓗ, enum-label folding ⓐⓑ, negative bounds (part select / port), the `-G` aliases and the `.velab` header field, `case inside` |
| §2-N verilog-axi census | 2 rows + 5 | verilog-axi x-cycle promotion, the FST `$dumpvars` snapshot, and five t0-event residues |
| §2 start-order table | 27 rows | LOUD 6 · BLOCKED 6 · WALL 5 (declared-width provenance / §11.8.1 region sign) · OPEN 4 · PERF 2 · ORACLE-SPLIT 2 · DO-NOT-START 2 |
| §2 recorded defects by mechanism | 111 bullets | inline / frame binds 16 · size cast / signedness 16 · constant domain (i64) 14 · scoping / imports / block-locals 17 · ranges / bounds / selects 5 · oracle splits 10 · performance 7 · real 7 · index sealing 6 · delays / events 6 · ranges / bounds / selects 4 · diagnostics / artifacts 4 · class fields 3 |
| §3 numbered items | 24 rows | ③ file-I/O hoisting (4), ⑤ ibex ladder residues (9, including ⓕ the unpacked-array typedef residue), ⑧ system functions in function bodies and `$finish` (4), ⑨ package string/real constants (2), ⑬ diagnostic location (3), ⑭ call-tree observability (2) |
| §3 small residues | 87 rows | subroutine / frame 23 · constants / parameters 16 · parser accept 12 · system tasks & file I/O 9 · nets / timing 6 · strings / heap 6 · loud shapes surfaced by §4.5.493–495 6 · diagnostics quality 5 · VCD / real conversion 3 |
| §3 intentionally loud | 12 rows | not gaps; each has its reason |
| §4 SVA honest-loud | 6 | mostly no oracle; hand-IEEE when started |
| §5 performance / hardening residues | 18 rows | frame-body wprog (5c), native scratch pooling (4b-r), array-LHS cliff, inline-fold exponential, memory guard, CI nextest, MSRV ceiling, quiescence / render / eof seams |
| §6 G2 OBS | 6 stages + 8 | OBS-2 residue → OBS-1 residue → R-L4 → OBS-4 control → OBS-5 snapshot → OBS-6 X-origin, plus 8 items beside the staged track (call tree, subroutine join key, per-call-site builtins, the staged `--hier-tree` accept-and-drop, generate scopes, enum names, R-I1/R-I2, `WPROG-WHY`) |
| §7 conditional | 4 | BACKEND · VHDL · VCD-EXT · MVP-CUT |
| §8 non-goals | 2 | IMPLICIT-NET and the out-of-scope list · `defparam` beyond a direct-child constant target |

## D. Prerequisites that block work from starting

- §11.8.1 region sign in the wide constant fold — blocks §2 rows 14, 15, 16, 25, 26, 30, 🆕 F and
  every widening of the fold's accept set, which includes 🆕 H ⓐ (a bound whose operator is DEFINITE
  despite an x operand clamps to one bit, and the fix site is that accept set).
- A wide resolver that reads a SELECT — blocks §2 row 10's surviving half; an unguarded bound
  fallback moves 0 of 18 cells.
- A tree-wide AST self-width pass — blocks the size-cast cluster in §2 "Size cast / signedness".
- A per-resumption-kind ordering model — blocks §2 row 7; one ordering key cannot express what the
  oracles do in a single run of a single design.
- A block-scoped CONSTANT binding — blocks §2 🆕 Q; a bare-name hoist makes 6 cells correct and 5
  new silent-wrongs.
- A field-key normalisation map — blocks §2 row 3b; the map is keyed by NetId and a class field is
  not a net.
- Per-instance declarator arity — blocks §3 ⑤ⓕ's dim-COUNT axis; it needs a symbolic arity marker on
  two SchemaHash root types.
- Per-instance class registration — blocks §3 ⑤ⓕ's class-property container; `register_classes` is a
  whole-design prescan, so a property of type `T` is loud with no override at all.
- One oracle and zero corpus demand — blocks clocking (§2 rows 23, 24, 34).
- Oracle splits are recorded, never chased.
