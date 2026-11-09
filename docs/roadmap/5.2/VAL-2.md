(§5.2 VAL-2 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G2 VAL-2) (one entry and one result record for the elaborate constant evaluator):
* slot: R3.2 · L1, after VAL-1 (owner, 2026-11-09)
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* fix: one canonical entry `fold(e, scope, ctx) -> Folded`: the typed value (`WideBits` as a struct) or Declined carrying only what PRE already tells apart (`CFold` Four / No, `XWhy`), the five cores its lanes
* fix: every PRE entry a delegate with its signature, no consumer edited: `const_eval_in_scope` 98, `eval_const_env` 8 + `_at` / `_self` 22, `fold_ctx` 10, `fold_bits_at` / `_at0` / `fold_self_bits` 37, `env_fold` 11, `const_int_selfdet` 18, `const_unsigned_selfdet` 6, `const_eval_u32` 10 as its literal lane
* evidence: today None means declined and refused (F24), and 8 readers take a decline as a value (`const_fn.rs:1476` `:1484` `:1674`, `inline_fold.rs:79`, `const_fn_width.rs:621` `:640`, `inline_fn.rs:128`, `params.rs:302`): each is named as a lane
* fix: the 22 `had_error` references (13 files) each behind a named question (this fold's own failure, or any error so far), every one answering PRE's latch
* note: one lane per call, never an eager second fold (S1v N-5: a second fold cost ~15 % wall); a check compares the exit code, the signal and the wall
* pins: no fold entry outside `value/` (the delegate list pinned); ratchet c unchanged, b re-aimed at the home, not lowered by a rename
* note: risk high: EVALUATION-ORDER (depth guards `MAX_DEPTH` 64 at `const_fn.rs:1607`, 8 at `const_bound.rs:410`) and Four vs No stickiness; size the 6 entry heads + `value/mod.rs`, 22 refs in 13 files named
* note: TW-LATCH: a named latch question answering unlike `had_error` on any cell is BLOCKING
* note: it frees F24 and S1v's B-2 out of the value channel (§2 zero-width-gate-after-error), the Q-EXECSEM census (after VAL-2) and row 29 r1's `const_eval_u32` literal lane (R7.5)
* note: the home rows' trip wires and their honest cost are written once, in §5.2 SYS-1
* source: W/program/reorder2-2026-11-09/PLAN.txt G2 VAL-2
