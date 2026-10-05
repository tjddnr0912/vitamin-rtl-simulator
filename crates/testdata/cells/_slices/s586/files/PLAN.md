# PLAN §4.5.586 — §3.b `unique-overlap-note` (ROADMAP §5.2 row 1), HEAD fc1fc07e

## Verdict: GO (full scope, no narrowing, no prerequisite row)

Intent: vita silently skips the IEEE 1800-2017 §12.4.2 / §12.5.3 multiple-match ("overlap") check for
`unique` / `unique0`, so a clean log reads as "checked". Keep the check a §8 non-goal and say so: one
Info line per parse, at the first written `unique` / `unique0` qualifier, changing nothing else.

Every lane in ER §10.2's table is measured (G7 plus four cells the planner added, below). The design
differs from the grounding probe in four places (Info not Note, mnemonic, message text, emitted
after the parse-error gate). Each difference is re-measured as an implementation cell (T-rows). None
of them is a lane.

Cells the planner added (S/g/, PRE = S/pre, probe = S/probe_bin):
- c50_worklib: `vcmp a.sv --work L=…` + `vcmp b.sv --work L=…` + `velab -L L=… --top t` + `vrun`.
  Probe: `a.sv:4:5` note on the first vcmp, `b.sv:6:5` on the second, velab and vrun print
  `errors=0 warnings=0 notes=0`, vrun stdout `y=1 z=1` (cmp-identical to PRE). The library `.vu`
  units are md5-identical PRE vs probe. The `.velab` files differ only in the run-specific trailer at
  bytes 1018–1075; a PRE-vs-PRE control twin differs in the same region.
- c52_include: `unique0 casez` in an `` `include ``d task. Probe `inc.svh:4:3`; iverilog
  `./inc.svh:4: vvp.tgt sorry: Case unique/unique0 qualities are ignored.` plus `t.sv:7: …`;
  verilator `[0] %Error: inc.svh:4: Assertion failed in t.chk: unique0 case, but multiple matches found for '2'h3'`.
- c53_log: `--log` carries the note line; stdout cmp-identical to PRE.
- c51 (negative controls): `q.unique()` → no note in PRE or probe. A constraint `unique {a, b}` is a
  pre-existing E2002 in both, with no note.

## Decisions

| # | decision | reason (cell) |
|---|---|---|
| D1 sites | A: the first `unique`/`unique0` qualifier token, in token order, recorded in `parse_unique_priority` | The row names that function. A is measured on every lane. B (first elaborated site) needs an AST carrier because `unique0` and `unique`+`default` leave no trace after parsing (c31: .velab identical except the 32-byte digest). That costs a schema_hash re-pin and makes every `.vu` stale. B was never probed, and its order would follow instance DFS rather than argv (c43) |
| D1b unreached code | still announced (c13b `--top`, c14 untaken generate, c17 uncalled fn) | The message claims nothing about reachability. iverilog prints for c17 but not for c13b/c14. Recorded in the test header and manual 006 |
| D1c qualifiers | `unique` and `unique0`, on case/casez/casex/case-inside/if. Never `priority`/`priority0` | IEEE defines no overlap rule for priority (c07/c08/c22: both oracles silent). `unique0` counts even with no other effect, because overlap is its only check (c02/c04/c06) |
| D2 severity | Info: `info[VITA-I2021]` | Manual 007 defines Note as a follow-on that carries its parent error's code, so a standalone note contradicts it, and doc15 has no `N` letter. Warning would let `-Werror` fail every design with a qualifier: ibex has 123 `unique case` lines (G4), a ladder descent. Info = counted under `notes=`, `-Wno-` drops it, never promoted, exit unchanged. The gate and counters treat Info and Note alike (vita-log lib.rs:130, cli lib.rs:305-309), so the probe's measurements transfer apart from the token |
| D3 code | `I-PARSE-UNIQUE-OVERLAP-UNCHECKED`, `VITA-I2021`, variant `ParseUniqueOverlapUnchecked`, after `ParseSelectBase` | The parse stage emits it, so the band is 2xxx. 2004–2020 are reserved in doc15 Appendix A, so 2021 is free in both the body and the appendix. The probe's `…-UNIQUE-UNCHECKED` overclaims: the no-match check does exist. The reserved W3030 `W-ELAB-CASE-OVERLAP` names a *detection*, which is wrong here. 4032 sits in the runtime band and would print no `[at time]` |
| D4 location | file:line:col of the qualifier token through `loc_from_span(&pp.map, …)` | ER §2.7 (anchor + caret on the operand). iverilog names the qualifier line (c25). The location resolves through macros (c46 call site), includes (c52) and argv files (c43). W1017 is location-less because it reports an absence; this note reports a written construct |
| D5 emission point | the pre-gate loop records the span; the line is emitted right after the parse-error gate (`frontend.rs` after the `!parse_errors.is_empty()` return) | A design that does not parse never runs. iverilog prints its sorry only for a design it compiles. A failed parse then prints exactly PRE's stderr (c44 PRE: `t.sv:5:9: error[VITA-E2002] … found ';'` + `errors=1 warnings=0 notes=0`). W2004 stays before the gate (manual 007 says so). This changes the probe, which printed the note on c44 |
| D6 test util | add `I-PARSE-UNIQUE-OVERLAP-UNCHECKED` to the `diags()` filter (`always_comb_t0_util/mod.rs:30-46`), next to W1017 | Re-pinning the 12 tests would break `staged_matches` (:85-98): the parse-stage line prints at one-shot but not at staged `vrun` (c30/c41/c48/c50). The note's own presence is pinned in its own file |
| D7 work-library lane | measured (c50): one line per `vcmp` invocation that contains a qualifier | That path calls the same `frontend_sources_mapped` (pipeline.rs:519). No separate test row: no mutant would survive T12 and die only there |
| D8 count | once per parse (per `vita` / `vcmp` process), first site only | Row text. ER §2.7's "oracle count as standard" targets defect reports. iverilog prints per elaborated site (c24: ×2). Flagged for the lenses |
| D9 release docs | CHANGELOG, docs/manual/{003,004,006,007}, README count — slice commit. doc15 entry + Appendix A count — slice commit (bijection gate + `include_str!`, reviewed by both lenses) | LOOPROMPT §6 / CONTRIBUTING Workflow |
| D10 SPEC docs | doc13 :223 and :259 count `68` → 71: docs commit after review. preview 01:136 ("Multi-match is not checked") stays true: unchanged. hdl-reference 03-procedural.md:90-92 describes IEEE, not vita: unchanged | LOOPROMPT §6 (SPEC after two reviews) |

Message text (one line, no final period, house style of W2004/W1017). It must not contain `error`,
`warning[`, `W4031` or `unhandled`: corpus-runner's refused-row fallback greps `error[`/`error:`, and
existing tests count W4031 lines by those needles.

```
`unique` / `unique0` overlaps are not checked: when more than one case item or `if` condition matches, the first one runs and no violation is reported, where IEEE 1800-2017 §12.4.2 and §12.5.3 require one. Printed once, at the first such statement in source order
```

How each claim was measured. "The first one runs": c01–c06 y=1 on native, interp and vm (vita = iverilog = IEEE; verilator's
`unique if` y=0 is its quirk). "No violation is reported": G1 vita column. "Once" and "first in source order": c16, c23, c42, c43.
msgcodes title: "`unique`/`unique0` overlaps are not checked (IEEE 1800 §12.4.2/§12.5.3); said once per parse".

## Lane table (ER §10.2) — 0 unmeasured

Shared code edited or routed into: `parse_unique_priority` (1 caller, stmt.rs:165); `ParseWarnKind` (1
consumer, frontend.rs:343); `frontend_pp_to_unit_mapped` (one-shot frontend.rs:590 and vcmp
pipeline.rs:519; test wrappers route here); GatedSink and StderrSink unchanged.

| lane | PRE | measured cell | POST re-run (impl) |
|---|---|---|---|
| native | no note on any G1 cell | c41/c48 note ×1 | T1–T11 |
| interp, vm | — | c41 stdout+stderr identical to native | T12 |
| staged multicall (`vita vcmp/velab/vrun`) | W2004 proxy at vcmp only (c30) | c41 | T12 |
| staged separate-bins | c30 | c48 (vcmp 1, velab 0, vrun 0) | manual cell, `--features separate-bins` |
| work library (`--work` / `-L`) | c50 | c50 (above) | manual cell |
| JIT (`--features jit`, `VITA_JIT=1`) | — | c48 identical | manual cell |
| product (`--no-default-features`) | — | c48 identical | manual cell + CI axis |
| `-q` `-Wno-` `-Werror` `-Werror=` `--log` | — | c41, c47, c53 | T11 (+ `--log` manual) |
| `--obs-dir` | — | c41: run.json differs only in timings | manual cell |
| `hdl_parser::parse` (44 callers) | — | opted out: api.rs:14 drops warnings | full gate |
| corpus ibex (only workload with a qualifier) | c40 DIGEST=13b2ddfcd551ba2f rc0 | c40 stdout identical, +1 stderr line | corpus-runner run |
| no `unique`/`unique0` | c49 examples ×4, c22 | stdout, stderr, rc and VCD cmp-identical | re-run the c49 cmp vs PRE |
| include / macro / `ifdef` / multiline | — | c52, c46, c45, c25 | T7, T8 |
| parse failure | c44 PRE stderr | probe printed the note (D5 changes this) | T10 exact = PRE |

## Implementation steps

1. `crates/hdl-parser/src/lib.rs`: add `ParseWarnKind::UniqueOverlapUnchecked`. Its doc: the first
   `unique`/`unique0` of the parse, at most one per parse, and why. Add the helper
   `record_unique_qualifier(&mut self, span)` beside `warn_select_base`. It pushes only if no warning
   of that kind exists, so the first wins. It is keyed on kind, not span, which makes it the single
   home of "once". Rewrite the `ParseWarn` doc, which today says only "other tools read it
   differently" (ER §10.3: re-verify the whole block you open).
2. `crates/hdl-parser/src/assertions.rs` `parse_unique_priority`: before `self.bump()`, if
   `peek` is `Kw::Unique | Kw::Unique0`, call the helper with `qspan`. Add one sentence to the fn doc
   (priority has no overlap rule).
3. `crates/hdl-parser/src/api.rs`: widen the `parse_with_warnings` doc sentence to match.
4. `crates/diag/src/code.rs`: add the row (D3). `crates/diag/tests/bijection.rs`: 70 → 71 (number and message).
5. `crates/cli/src/frontend.rs` `frontend_pp_to_unit_mapped`: make the loop an exhaustive `match`.
   `NonStandardSelectBase` emits W2004 as today; `UniqueOverlapUnchecked` stores the span (no
   `unreachable!`, no second dedup). After the parse-error gate, emit `Severity::Info`,
   `MsgCode::ParseUniqueOverlapUnchecked`, the message, `loc_from_span`, no context, no sim_time.
   Comment why it goes after the gate and why W2004 stays before.
6. `crates/cli/tests/always_comb_t0_util/mod.rs`: filter (D6) and update the `Run.diags` doc
   (both lines are once-per-parse, parse-stage, absent from staged `vrun`).
7. New file `crates/cli/tests/unique_overlap_note.rs` (below). Run `cargo fmt --all` first.
8. `docs/preview/15-error-code-reference.md`: entry after W2004, header
   ``### VITA-I2021 · `I-PARSE-UNIQUE-OVERLAP-UNCHECKED` (Info)``. Cover: what is and is not checked;
   no-match is still W4031 for `unique`/`priority` and is suppressed for `unique0`/`priority0`; once per
   parse at the first written site, reached or not; printed by vita and vcmp, never by velab or vrun;
   not printed on a failed parse; iverilog's sorry text and verilator's text verbatim; an example
   whose `->` output is pasted from a POST run; Fix: none needed, `-Wno-I2021`. Appendix A intro
   "70 codes" → 71.
9. Docs in the slice commit:
   - manual 006 §1.4: rewrite the last paragraph (:209-213). Name the code, once per parse, first
     written site reached or not, vcmp-only in the staged flow, iverilog per elaborated case, verilator
     checks.
   - manual 003 :763 and :764: add the code to the "documented cut" sentence and to the `unique0` row.
   - manual 007: add a 2xxx row; :6 "70" → 71; :180 "70 … 2 to Info … 33" → "71 … 3 to Info … 34";
     :335 and :605 "68" → 71 (already stale; fix every count sentence the slice moves).
   - manual 004 :366 "68" → 71; README :226 "70" → 71.
   - CHANGELOG `[Unreleased]` `### Added — …`.
10. Docs commit, after review:
    - ROADMAP: delete :548; delete §5.2 row 1, renumber, and fix the prose row references ("rows 1–3",
      "row 3 is ①", "rows 1–7", "Rows 4–7").
    - ROADMAP §8 :714 → "`unique` / `unique0` multiple-match checking (`I-PARSE-UNIQUE-OVERLAP-UNCHECKED`
      says so once per parse)". "priority" there was wrong: priority has no multiple-match rule.
    - ROADMAP Summary: recount by census (expected §3.b 127/110/17, total 458/271/187, `next` shifts by −1).
    - doc13 counts (D10). LOOPROMPT NEXT is untracked dev-meta.

## Tests (`unique_overlap_note.rs`): oracle lines verbatim above each; the module header names iverilog 13.0 (`-g2012`, `vvp -n`), verilator 5.052 (`--binary --timing --assert`) and IEEE 1800-2017 §12.4.2 / §12.5.3

- T1 c01: the whole stderr line pinned exactly (`t.sv:6:5: info[VITA-I2021] I-PARSE-UNIQUE-OVERLAP-UNCHECKED: <msg>`), stdout `y=1`, rc 0,
  epilogue `errors=0 warnings=1 notes=1`. iverilog `t.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`, `y=1`;
  verilator `[1] %Error: t.sv:6: Assertion failed in t: unique case, but multiple matches found for '2'h3'`. Kills M7 M8 M9 M12.
- T2 c04, c05, c06: `6:5 I2021` once each, y=1. iverilog: sorry on c04; c05/c06 `t.sv:6: syntax error`.
  verilator: `unique0 case, but multiple matches found for '2'h1'`; `'unique if' statement violated` (its y=0, recorded). Kills M2 M13 M14.
- T3 c07, c08, c22: no `[VITA-I2021]`, `notes=0`. Both oracles silent; iverilog prints no sorry. Kills M1.
- T4 c42: the line is at `9:5` (the `unique0`), not `5:5` (the `priority`). Kills M1′ (first qualifier of any kind).
- T5 c16, c23: exactly one `[VITA-I2021]`, at `6:5`. iverilog prints the sorry twice (t.sv:6, t.sv:10). Kills M4 M5.
- T6 c43: `a.sv b.sv` → `a.sv:2:15`; `b.sv a.sv` → `b.sv:6:5`. Kills M5 and any order not taken from argv.
- T7 c52 → `inc.svh:4:3`. c46 → `t.sv:6:5`. c45 → none; with `-DNOPE` → `t.sv:5:5`.
- T8 c25: `t.sv:6:5` (qualifier line), where iverilog says 6 and verilator 7. Kills M10.
- T9 c13b `--top t`, c14, c17: each announced at its own line, y=5. Records that iverilog prints only for c17.
- T10 c44: stderr == PRE's two lines exactly, rc 1. Plus an order cell: `unique` at line 4 and a W2004
  select-base at a later line → W2004 line, then the I2021 line, then W1017. Kills M6.
- T11 a design with a `` `timescale `` (so W1017 cannot be promoted): `-Werror` gives rc 0, an `info[` line
  and `errors=0 warnings=0 notes=1`. `-Wno-I2021`, `-Wno-VITA-I2021` and the mnemonic each give no line and
  `notes=0`. `-q` keeps the line. Kills M7.
- T12: native, interp and vm give identical stdout+stderr. `vita vcmp` has exactly one line; `vita velab` and
  `vita vrun` have none. Staged stdout == one-shot stdout. Kills M15.
- T13: `q.unique()` → none. Kills M16 (a token scan instead of the statement funnel).

Teeth: run the file against M12 (frontend arm deleted). Every positive test must fail; T3, T10 and T13 pass.

## Mutants (write each expected outcome before running; narrow `-p cli --test unique_overlap_note` first, then re-confirm survivors at `--workspace --no-fail-fast`)

| id | mutation | expected killer |
|---|---|---|
| M1 | latch on `Unique\|Unique0\|Priority\|Priority0` | T3, T4 |
| M2 | latch on `Unique` only | T2 (c04, c06) |
| M4 | no once-latch (push every site) | T5 |
| M5 | last site wins (replace span) | T5, T6 |
| M6 | emit inside the pre-gate loop | T10 |
| M7 | `Severity::Warning` | T1 token, T11 rc |
| M8 | `Severity::Note` | T1 token |
| M9 | `location: None` | T1 |
| M10 | span of the `case`/`if` keyword, not the qualifier | T8 |
| M12 | arm deleted (the feature does nothing) | T1–T9, T11, T12 |
| M13 / M14 | latch only in the case arm / only in the if arm | T2 / T1 |
| M15 | emit only in the one-shot driver (`run_vita_str_gated`) | T12 |
| M16 | first `Kw::Unique` token found by a token scan | T13 |

## Byte-identity

- No `unique`/`unique0` written in statement position: the helper is never called, `parse_with_warnings`
  returns the same `Vec<ParseWarn>`, and the frontend stores no span and emits nothing. The AST is
  untouched (no field, no item), so `.vu`, `.velab`, SimIr, schema hashes and format_version 35 are
  unchanged. stdout, stderr, VCD and exit code are byte-identical. `priority`/`priority0`-only designs
  are covered by the same argument (the latch keys on Unique|Unique0).
- Any design that fails to parse: stderr byte-identical to PRE (D5).
- With a qualifier: one extra stderr line from `vita`/`vcmp`, `notes` +1, nothing else.
  Measured: ibex digest, c49, the c50 `.vu` md5. Non-vacuity: the arm fires exactly once on ibex and on every positive T-cell.

## Gates

- Iterate with `cargo nextest run -p cli --locked` (shared frontend, so the whole `-p cli`), plus `-p diag` and `-p hdl-parser`.
- Full gate once before commit: `cargo nextest run --workspace --locked --no-fail-fast` (PRE: 9032 run,
  15 skipped; POST: 9032 + new, 0 FAIL/TIMEOUT), then `cargo test --doc --workspace --locked`, clippy
  `--workspace --all-targets`, `fmt --check`, and the product axis (3 commands).
- No flip run: native and backend are untouched.
- `cargo run -p corpus-runner --locked -- run` before push: failing 0, ibex `ok`.

## Review targets (lenses: hit hardest)

1. The four deltas from the probe on a frozen POST release binary, every G7 lane re-run. Above all the
   after-gate move: is a successful parse ever left without the line, or is the line ever printed twice
   in one process?
2. Each message and doc clause against a cell (the claims list under the message, the manual 006 and
   doc15 entry wording, every count sentence 68/70 → 71).
3. Site funnel completeness: stmt.rs:165 is the only caller. Check `Kw::Unique` in other positions,
   error recovery, include, macro and `ifdef`; first-site determinism across argv and `-f` files.
4. Gating and harness scope: the `diags()` filter hides only I2021, and the 12 tests still assert every
   other diagnostic; `staged_matches` still compares; corpus refused-row needles.
5. Outside the table: any second parse per process, and any sink that treats Info differently from Note
   (census: only `token()`, cli lib.rs:339).

## Risks and what I could not determine

- The printed token is `info`, while the row and the external report say "note". If the owner
  wants `note[`, manual 007's Note row and doc15's letter set (E/W/I/F) have to change first. I did
  not decide this for the owner; D2 records the reasons for Info.
- A test that asserts on `info[` is invisible to the probe, which printed `note[`. Census: the only
  `info[` test is sim-engine `severity_tasks.rs`, which never parses. The full gate settles it.
- D1b over-reports against iverilog (c13b, c14) and D8 counts once where iverilog counts each site.
  Both are deliberate (row text) and both are stated in manual 006 and doc15. A lens may still file
  them; re-measure before adopting.
- The IEEE wording is recalled; there is no local copy of the standard. The repo cites the same
  clauses at manual 006:174 and doc15:1288.
- "2 reviews" for SPEC docs is read as: after both lenses close. doc15 has to ride the code commit
  anyway (bijection gate).
- Out of path, not fixed here (one PROBE_CATALOG line each):
  - doc15 Appendix A reserves `E2004` while body `W2004` holds that number.
  - manual 007's "two codes emitted at a severity other than their default" sentence and table omit
    `E4002`, which doc15 lists since §4.5.576.
