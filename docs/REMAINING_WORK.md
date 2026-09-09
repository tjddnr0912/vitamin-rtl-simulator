# vitamin — remaining work (snapshot)

Top-level snapshot of what stands between HEAD and the two goals. Rewritten whole at every re-plan; the detailed rows are in [ROADMAP.md](ROADMAP.md), completed work in [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md), history in [DEVLOG.md](DEVLOG.md). Baseline counts (tests, format_version, MsgCode) live in CLAUDE.md only.

- G1 = a correct open-source RTL simulator (correct-or-loud) at the level of icarus / verilator / xcelium / vcs.
- G2 = an AI-agent-friendly simulator (observability rail; SPEC = [preview/19](preview/19-ai-agent-observability.md)).

## A. Status

- Default backend `native`; product build `--no-default-features` (one executor); workload corpus 10/10 with 0 rejections.
- Performance axis: diminishing returns reached; codegen (cranelift), 2-state storage, cycle-based mode, levelize all rejected with recorded re-entry conditions (ROADMAP §5).
- External reports (round 1–39): closed except the residues filed into ROADMAP §2 / §3.
- ⚠️ The G2 rail is now self-checked as well as emitted: `run.json`'s subroutine census was found
  reporting swapped/dropped counts for any design containing a class, invisible to a same-input
  determinism golden. The teeth for a reporting rail is an ASYMMETRIC MUTATION (change something
  upstream that must not move the numbers), not a repeat run.
- `corpus-runner run` prints the elaborate / simulate split per row. Every workload is ≥99% simulation, so the corpus still cannot GATE a front-end regression (ROADMAP §5.b `ELAB-PHASE-BLIND`).

## B. Queue (canonical = ROADMAP §5.2)

| # | track | item |
|---|---|---|
| 1 | §2 silent-wrong | **Block-local shadow MIS-ROUTE** — a block-local that shadows a module net, and whose declaring span the scoping pass drops from candidacy, leaves its write ON the shadowed module net (`MOD=41`, both oracles `0`). It is the measured PREREQUISITE for §3.b `blocal-flatten`, which was built and reverted because every widening of the scope set uncovers more of it |
| 2 | §2 silent-wrong | **A 33..64-bit override VALUE is cut at bit 32** on an untyped target while `$bits` reports 33/64 — vita contradicting itself in one run, on the literal spelling too. `defparam` cuts further. The declared-width target lane is correct |
| 3 | §2 silent-wrong | **Forwarding**: an untyped parent parameter overridden at a width other than its default literal's forwards as 32. Root = the sized-literal arm of `param_decl_width_opt` is not gated on `default_binds` |
| 4 | §6 OBS | Give the static `subroutines` rows a declaration site, so the two subroutine objects can be joined. ⓑ (`SubProfile` → `subroutine_calls`) and ⓒ (per-FuncId declaration site) SHIPPED, and the census they read was itself repaired — it had been reporting swapped or dropped counts for any design containing a class |
| 5 | §6 OBS | `WPROG-WHY`: nothing says why an EXPRESSION left the compiled lane, so a reader infers the boundary from builtin call counts and gets it wrong — an external report did, and so did this repo's refutation of it (two rounds) |
| 6 | §3 loud → correct-support | ⑤ⓕ residue: the non-arity axis of `shape_flags`' F4004 (signedness, 2-state kind), multi-dimensional packed type-param default/override, mixed-caller callee, `m #(8)` / `defparam u.T$w`, VCD `$scope` spelling, `genblk<N>` collision (split) |
| next | — | 🆕 L ⓦ residue, §2 🆕 N residue, a labelled concurrent `assert property` action block's `%m`, the §2 static-task-frame twin of the block-local class |

Priority principle: ① silent-wrong with an oracle > ② loud→supported with an oracle > ③ honest-loud promotion whose prerequisite holds > ④ G2 OBS. Performance is below the ladder.

## C. Open items by section (counts from ROADMAP at HEAD)

| section | open | breakdown |
|---|---:|---|
| §0 promotion queue (T2/T3 residues) | 14 rows | real const-fold residues ⓐ–ⓔ ⓖ ⓗ (ⓕ `time` closed by §4.5.462), enum-label folding ⓐⓑ, negative bounds (part select / port), `-G` aliases and `.velab` header field, `case inside` |
| §2 start-order table | 27 rows | LOUD 6 · BLOCKED 6 · WALL 5 (declared-width provenance / §11.8.1 region sign) · OPEN 4 · PERF 2 · ORACLE-SPLIT 2 · DO-NOT-START 2 |
| §2 recorded defects by mechanism | 93 bullets | inline / frame binds 16 · size cast / signedness 15 · constant domain (i64) 14 · index sealing 8 (row 25's operator half CLOSED; the name-leaf and >64-bit declines plus the `unsigned`-keyword sign are the new rows) · delays / events 7 · performance 7 · oracle splits 6 · real 5 · ranges 4 · scoping 4 · diagnostics 4 · class fields 3 |
| §3 numbered items | 24 rows | ③ file-I/O hoisting (4), ⑤ ibex ladder residues (9, incl. ⓕ the unpacked-array typedef residue), ⑧ system functions in function bodies / `$finish` (4), ⑨ package string/real constants (2), ⑬ diagnostic location (3), ⑭ call-tree observability (2) |
| §3 small residues | 66 rows | subroutine / frame 17 · constants / parameters 11 · parser accept 9 (`iface-blocal` closed by §4.5.464; its flatten-model residue `blocal-flatten` replaces it) · system tasks & file I/O 9 · nets / timing 6 · strings / heap 6 · diagnostics quality 5 · VCD / real conversion 3 |
| §3 intentionally loud | 12 rows | not gaps; each has its reason |
| §4 SVA honest-loud | 6 | mostly no oracle; hand-IEEE when started |
| §5 performance / hardening residues | 18 rows | frame-body wprog (5c), native scratch pooling (4b), array-LHS cliff, inline-fold exponential, memory guard, CI nextest, MSRV ceiling, quiescence / render / eof seams |
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
