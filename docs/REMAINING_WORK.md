# vitamin — remaining work (snapshot)

Top-level snapshot of what stands between HEAD and the two goals. Rewritten whole at every re-plan; the detailed rows are in [ROADMAP.md](ROADMAP.md), completed work in [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md), history in [DEVLOG.md](DEVLOG.md). Baseline counts (tests, format_version, MsgCode) live in CLAUDE.md only.

- G1 = a correct open-source RTL simulator (correct-or-loud) at the level of icarus / verilator / xcelium / vcs.
- G2 = an AI-agent-friendly simulator (observability rail; SPEC = [preview/19](preview/19-ai-agent-observability.md)).

## A. Status

- Default backend `native`; product build `--no-default-features` (one executor); workload corpus 10/10 with 0 rejections.
- Performance axis: diminishing returns reached; codegen (cranelift), 2-state storage, cycle-based mode, levelize all rejected with recorded re-entry conditions (ROADMAP §5).
- External reports (round 1–39): closed except the residues filed into ROADMAP §2 / §3.
- `corpus-runner run` prints the elaborate / simulate split per row. Every workload is ≥99% simulation, so the corpus still cannot GATE a front-end regression (ROADMAP §5.b `ELAB-PHASE-BLIND`).

## B. Queue (canonical = ROADMAP §5.2)

| # | track | item |
|---|---|---|
| 1 | §3 loud → correct-support | ⑤ⓕ residue: the tf-port formal (1-oracle for the fixed shape) · a `parameter` of an unpacked-array type · `parameter type T = a_t` · ⑤ ⓓ/ⓔ residues · CU-scope items in a class body · ibex DPI export |
| 2 | §2 silent-wrong | row 7: a parent `initial` reading a child net at t0 sees X where both oracles read the child's own init. ⚠️ needs a `sim_ir::Process` sidecar ⇒ format bump — price it at grounding |
| 3 | §2 silent-wrong | 🆕 I ⓒ: the word-alias residue — a RUNTIME index `m[k]` reads `xx` where both oracles read `a5`, and a full-range select of an array word stays `x`. ⚠️ its neighbours are oracle splits; census the 2-oracle cells only |
| 4 | §6 OBS | R2 call tree: task/function rows in `processes.items[]`. Prerequisite ⓐ **shipped** (`run.json`'s `subroutines` route census, doc-19 §4.10) — what is left is ⓑ the `SubProfile` at the three frame seams and ⓒ a decl `file:line:col`, and ⓑ is no longer blocked: with ⓐ in the file a 0-call row can be told from an inlined one — ROADMAP §6 |
| 5 | §6 OBS | `WPROG-WHY`: nothing says why an EXPRESSION left the compiled lane, so a reader infers the boundary from builtin call counts and gets it wrong — an external report did, and so did this repo's refutation of it (two rounds) — ROADMAP §5.b |
| next | — | mixed-caller callee, `m #(8)` / `defparam u.T$w`, VCD `$scope` spelling, `genblk<N>` collision (split), 🆕 L ⓦ residue (package constants outside the i64 interpreter), a labelled concurrent `assert property` action block's `%m` |

Priority principle: ① silent-wrong with an oracle > ② loud→supported with an oracle > ③ honest-loud promotion whose prerequisite holds > ④ G2 OBS. Performance is below the ladder.

## C. Open items by section (counts from ROADMAP at HEAD)

| section | open | breakdown |
|---|---:|---|
| §0 promotion queue (T2/T3 residues) | 15 rows | real const-fold residues ⓐ–ⓗ, enum-label folding ⓐⓑ, negative bounds (part select / port), `-G` aliases and `.velab` header field, `case inside` |
| §2 start-order table | 28 rows | OPEN 8 · BLOCKED 3 · WALL 5 (declared-width provenance / §11.8.1 region sign) · ORACLE-SPLIT 2 · PERF 2 · LOUD 6 · DO-NOT-START 2 |
| §2 recorded defects by mechanism | 86 bullets | size cast / signedness 15 · constant domain (i64) 14 · inline / frame binds 16 · index sealing 4 · real 5 · ranges 4 · class fields 3 · scoping 4 · delays / events 6 · diagnostics 4 · performance 6 · oracle splits 5 |
| §3 numbered items | 24 rows | ③ file-I/O hoisting (4), ⑤ ibex ladder residues (9, incl. ⓕ the unpacked-array typedef residue), ⑧ system functions in function bodies / `$finish` (4), ⑨ package string/real constants (2), ⑬ diagnostic location (3), ⑭ call-tree observability (2) |
| §3 small residues | 65 rows | parser accept 8 · constants / parameters 11 · subroutine / frame 17 · system tasks & file I/O 9 · nets / timing 6 · diagnostics quality 5 · strings / heap 6 · VCD / real conversion 3 |
| §3 intentionally loud | 12 rows | not gaps; each has its reason |
| §4 SVA honest-loud | 6 | mostly no oracle; hand-IEEE when started |
| §5 performance / hardening residues | 16 rows | frame-body wprog (5c), native scratch pooling (4b), array-LHS cliff, inline-fold exponential, memory guard, CI nextest, MSRV ceiling, quiescence / render / eof seams |
| §6 G2 OBS | 6 stages | OBS-2 residue → OBS-1 residue → R-L4 → OBS-4 control → OBS-5 snapshot → OBS-6 X-origin |
| §7 conditional | 4 | BACKEND · VHDL · VCD-EXT · MVP-CUT |
| §8 non-goals | 1 | DEFPARAM · IMPLICIT-NET · out-of-scope list |

## D. Walls (do not start until the prerequisite stands)

- Declared-width / sign provenance in the wide constant fold (§11.8.1 region sign): §2 rows 14 · 15 · 16 · 25 · 26 · 30 · 🆕 F, and every widening of the fold's accept set.
- §2 🆕 H ⓐ joined that wall (measured 2026-09-07): a bound whose operator is DEFINITE despite an x operand — `&` with a 0, `|` with a 1, `~&`, `~|`, `===`, `&&` — clamps to one bit, and the fix site is `fold_self_bits`'s reduction arm, i.e. the accept set itself.
- §2 row 10's surviving half (a >64-bit parameter SELECT in a range bound, one bit vs both oracles' 221) needs a wide resolver that reads a select: `selfdet_bits_unsigned` declines it today, so an unguarded bound fallback moved 0 of 18 cells.
- Tree-wide AST self-width pass: the size-cast cluster in §2 "Size cast / signedness".
- Clocking (rows 23 / 24 / 34): one oracle, zero corpus demand.
- Block-scoped CONSTANT binding (§2 🆕 Q): a `localparam` in a procedural block. The bare-name hoist was built and reverted — 6 cells correct, 5 new silent-wrongs (ROADMAP §2 🆕 Q carries the measured cells).
- Oracle splits are recorded, never chased.
