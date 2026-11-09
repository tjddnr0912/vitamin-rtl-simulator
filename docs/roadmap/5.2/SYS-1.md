(§5.2 SYS-1 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G1 SYS) (one record per system function, in sim-ir):
* slot: R2.1 · L1, the first row of the record-first order, after batch 1 lands (owner, 2026-11-09: the order approved with the new rows SYS-1, PARSE-1, VAL-1–3, BIND-1 and BIND-2)
* slot: its grounding may start before batch 1 lands (its files, `sim-ir` `names.rs` and 8 classifier sites, miss batch 1's diff); it builds after, PRE = main then
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* scope: system-function facts keyed on the name only, a bool or an id per property: runtime id, folds into the integer domain, a dimension query, width-neutral in the edge gate, a folded real conversion, exact in i64, inert for a hoist
* evidence: members `sys_fn_is_integer` (`const_fn_width.rs:64`), `is_dim_query_name`, `is_other_sys_call` (`edge_gate.rs:631`), `real_conversion_is_folded`'s set (`param_query.rs:112`), `expr_exact`'s `$clog2` arm (`expr_size_hier_exact.rs:27`), `expr_is_inert`'s list (`hoist/sformatf.rs:66`), `map_sysfunc` (`systask.rs:301`), `sim-ir` `sysfunc_name` (`names.rs:89`): 41 sites in 15 files
* evidence: one name is spelled at 4–16 sites: `"$clog2"` 16 (elaborate 11, parser 4, sim-ir 1), `"$dimensions"` 11, `"$signed"` 9, `"$bits"` 8 (census at bb99f548, W/program/reorder2-2026-11-09/census)
* fix: one record per system function in `sim-ir/src/sysfn.rs` beside `names.rs`, which elaborate and the engine both reach (ER §5.1: a question beside its twin in `sim-ir`), 1 new file, 8 fns re-pointed
* fix: one home is right because every member keys on the same name set; each classifier's set is a different question, so each stays a named column copied verbatim from its PRE set
* note: the columns: folds_integer {`$clog2` `$bits` `$rtoi`}, width_neutral {`$clog2` `$signed` `$unsigned`}, real_conversion {`$clog2` `$rtoi`}, exact_i64 {`$clog2`}, inert (`expr_is_inert`'s list), runtime (`map_sysfunc`'s map), dim_query
* note: outside by rule: the system task lists (`systask.rs` display, format, file I/O: statements, no value) and the parser's `$clog2` fold arms (PARSE-1)
* pins: a structure test, no `$name` set in a constant, width or hoist classifier outside `sysfn.rs` (named exceptions: the fold cores until VAL-1, `systask.rs`'s task lists, the parser), and a name ↔ `SysFuncId` round trip
* note: order and cost: a name match, no evaluation; risk low; it frees one entry per new `$name` (Q-SYMMIX (C), Q-RS30FOLD)
* note: D12 never together: VAL × SYS 9 shared files, TYPE × SYS 9
* note: trip wires every home row names before build (ER §3.6; PLAN G6): TW-BYTE, one moved byte on the cells of record or the 26,858 sweep rows, is BLOCKING; a second on one class goes to a fresh opus-plan, a third parks the row with the rows stacked on it (D8)
* note: TW-BYTE's cells of record: fix (d)'s 2,877 + 49, row 46's 70 + 112, row 24's 113, S1v's 133 / 430, Q-SYMMIX's counterexamples and row 29's page cells
* note: TW-ORDER, EVALUATION-ORDER (per-site order re-derived): a new stack overflow or signal, more than 2x wall on a cell whose PRE wall is at least 0.5 s (median of 3), or past ±3 % / ER §8.1 instructions on the 27 cost cells
* note: TW-REFUSE: any new diagnostic, refusal, or decline turned value in a home row is BLOCKING; TW-REACH: a member the home cannot reach is named OUTSIDE, and if that leaves a rule spelled twice in one crate the row re-plans
* note: TW-RATCHET: a `structure_ratchet` key rises (a 159/23, b 102, c 98/10/27, d 85, e 24), so the move is respelled; TW-LATCH (VAL-2): a named latch question answering unlike `had_error` on any cell is BLOCKING
* note: the home rows take none of the parked rows' cells as acceptance, only byte-equality (three blockers on one axis: Q-SYMMIX 5 rounds, row 24 three parks, row 29's page half three lens rounds)
* note: honest cost: seven home rows before the first value row; fix (d) slice A, the one precedent (~28 accessor sites, 34 files), took ~2.8 days and seven premise rounds; these homes span 41 to 357 call sites
* note: so about three weeks with no page or real-design payoff, then four batches of value rows and re-entries before VeeR EH1 on unmodified RTL (batch 8, two to three weeks more) and OpenTitan (batch 9); per-row token cost unmeasured (U6)
* source: W/program/reorder2-2026-11-09/PLAN.txt G1 SYS, G2 SYS-1, G6 (approved by the owner, 2026-11-09)
