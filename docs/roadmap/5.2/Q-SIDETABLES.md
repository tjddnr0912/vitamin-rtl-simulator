(§5.2 Q-SIDETABLES · rank — F · source W/review/structure-2026-11-04/REPORT.txt S7 items 1 + 4) (one engine side-table record, below sim-engine):
* slot: right after the first batch's boundary verify (D8: Q-MEMBSRC's carrier, 46, 24, 31, with Q-DECLTYPE's S12, Q-SYMMIX and Q-PARAM2STATE's value half landed before it) (owner, 2026-11-05)
* slot: beside Q-NETSHAPE, which goes first is the owner's call
* slot: no row waits on it, and it spans elaborate, cli and sim-engine, so it runs at a batch boundary, with no row under way
* slot: a `.velab` `format_version` bump, so it runs alone (D12)
* symptom: the engine side tables are declared 3–5 times and carried by hand: `elaborate/src/tables.rs:501`, `sim-engine/src/lib.rs:231`, `cli/src/lib.rs:433`, `sim-engine/src/state/mod.rs:247`, `elaborate/src/lib.rs:277`
* evidence: `..opts.sim_opts()` at `cli/src/frontend.rs:810` and `cli/src/staged.rs:758`
* evidence: 8 silent staged-drop format bumps (`vita-artifact/src/header.rs:15`), 02a9c3e0, cfe87bad
* evidence: parallel Vecs keyed by one FuncId / ProcId / class_id: 7986884a (one producer pushed 3 of 4 FuncId tables, so counts swapped or vanished), `elaborate/src/tables.rs:501-720` (16 parallel docs)
* fix: ONE `SideTables` struct (serde, Default) in a crate below sim-engine (a sim-ir module that derives no SchemaHash, or a new small crate)
* fix: the Elaborator owns a `SideTables` being filled (its 41 mirror fields go)
* fix: `SimOpts = { tables: SideTables, knobs }` with no `..` spread
* fix: `SimState` borrows `&SideTables` instead of ~28 clones
* fix: `.velab` carries it as ONE serde trailer (field order = struct order, one place)
* fix: sim-engine then needs sim-ir only (it drops the elaborate dependency, ratchet e. → 0)
* fix: one record per id — `ProcMeta {multiplier, prec_mult, scope, inst_scope, ident}`, `FuncMeta` + `{name, decl_loc}`, `ClassMeta {layout, field_inits, rand, constraints, dist, randc, vtable}` — each minted by one push (generalise `push_func`, `elaborate/src/frames_reserve.rs:31`)
* note: size (grep counts, not effort): ~12 files (elaborate `lib.rs` / `driver.rs` / `api.rs` / `tables.rs`, cli `lib.rs` / `frontend.rs` / `staged.rs` / `pipeline.rs`, sim-engine `lib.rs` / `state/mod.rs` / `init_diag.rs` / `native/mod.rs`)
* note: 294 field references in 23 sim-engine files and 142 `self.<field>` in 46 elaborate files re-pointed, one `format_version` bump
* note: the per-id records take 15 Sidecars fields to 3 and fold into the `SideTables` change
* note: a refactor row is verified by unchanged output — the full gate, corpus and cells byte-identical to PRE — not by page cells
* note: Q-VALRULES fix (d) slice A (§4.5.636) left the engine's range spellings over what sim-ir records to this row, named in `crates/hdl-ast/src/range_rules.rs`'s OUTSIDE list and its structure test's exceptions (W/qvalrulesd/plan5/filings.txt F21)
* note: those spellings are `sim-engine/src/state/task_frames.rs:1078` and `:1091` (the VCD header's declared span, a copy of `Lane::PartSelectSub`: sim-engine has no hdl-ast edge) and `sim-engine/src/builtins/pattern.rs` (a runtime pattern's stride)
* note: the same list also holds sim-ir `sel_written_bits` and `selfwidth.rs` `part_width`, and `sim-engine/src/state/changes.rs` and `state/frame_eval.rs` (a `[off -: w]` low index)
