(§5.2 notes) (5.2 Queue (start order)):
* One iteration takes one row, or one bundle of up to three rows whose vita source diffs are file-disjoint and whose lane tables do not meet (ER §3.7), plus at most one review-free hygiene item;
* a row whose files are disjoint from the rows under way may start before its slot, but a stage row's implementation starts only after the previous stage row lands.
* A row is a single root or one funnel's roots (an oracle, a corpus row's pinned oracle counting; outside walls and oracle splits).
* A row that needs a format bump takes it.
* Same-root rows are one slice, and their order is a measurement (a value lane lands before a guard over an unlimited fold is deleted).
* State (2026-11-04): Q-ALWINTRA parked 2026-11-02 behind Q-ALWBUDGET, L1 took C1 33, which parked 2026-11-04 behind Q-TYPEOVERVAL, so L1 empties;
* Q-AESINK landed (§4.5.633);
* row 41's page half landed (§4.5.634; its residue keeps slot 41 · L2 after 68), so L2 empties and is not refilled;
* Q-DECLTYPE landed (§4.5.635, its one slice S12: every declaration's type facts in one `hdl_ast::DataType`, with Q-PARAM2STATE's record half), so C0 · L1 runs Q-SYMMIX, then Q-PARAM2STATE's value half S1v, then the first batch (Q-MEMBSRC's carrier, 46, 24, 31), and Q-NETSHAPE, its elaborate half, follows beside Q-SIDETABLES (owner, 2026-11-05);
* Q-SYMMIX parked 2026-11-06 behind Q-VALRULES fix (d) after three premise rounds, nothing landed (W/qsymmix/RULINGS-premise3.txt), keeping C0 · L1, and S1v's rule (C) lanes a symbolic struct, an md packed parameter or a symbolic typedef / type parameter reaches wait on fix (d);
* Q-PKRDLSB parked 2026-11-02, T3 took C3 row 29's page half, which parked 2026-11-04 behind Q-PARAM2STATE;
* T3 empties and is not refilled (owner, 2026-11-04, replacing the refill rule of 2026-11-02): rows 33 (L1) and 41 (L2) finished in their slots (row 41 ended 2026-11-04, §4.5.634; row 33 parked 2026-11-04), an emptied slot is not refilled;
* rows 29, 33 and 41 have ended, so one slot runs the queue one row at a time, batch verify kept, with auto-fill off: the next row waits for the owner (owner, 2026-11-05).
* The owner named the next rows (2026-11-06): Q-VALRULES fix (d) as a code row at C0 · L1, then Q-SYMMIX's re-entry, then S1v, then batch 1.
* State (2026-11-08): Q-VALRULES fix (d) slice A landed (§4.5.636: the range record and every pair-of-bounds rule in `crates/hdl-ast/src/range_rules.rs`, every output PRE's), so Q-SYMMIX re-enters at C0 · L1, next, then S1v, which takes the four refusals slice A dropped beside R1v and the bound range rule, then batch 1;
* Q-VALRULES keeps its census and design (a)–(c), measure only, at 10 · L1 just before row 74.
* State (2026-11-09, owner decision O3): Q-SYMMIX parked behind Q-VALRULES (b) → row 74, to re-enter with spelling (C), nothing landed (W/qsymmix/RULINGS-premise5.txt), so S1v (Q-PARAM2STATE's value half) runs next at C0 · L1 as batch 0, then batch 1.
* State (2026-11-09, owner): batch 0 merged into batch 1 and S1v ended at S1v-Q (S1v-R's refusals and the Lane track to Q-VALRULES (b)); rows 46 and 24 parked behind Q-VALRULES (b) at their premise rounds 1, row 31 with 24; row 29 stayed parked; batch 1 = S1v-Q alone, landed (§4.5.637)
* State (2026-11-09, owner): the record-first order approved (W/program/reorder2-2026-11-09/PLAN.txt): the homes first, each structure only (every output PRE's, PRE's differences as named lanes), then the value rows that land in one place, then the parked rows' re-entries
* its homes are justified groups, not one forced home (the owner's condition): SYS-1, PARSE-1 with OPS, VAL-1–3 (Q-VALRULES (b), off row 74), BIND-1 and BIND-2 (new), and 74-S (row 74's structure half after Q-NETSHAPE, O1); arithmetic in two homes, hdl-ast 64-bit and `sim-ir` wide (O2)
* its batches (D8, at most five rows per goal): 2 SYS-1, PARSE-1; 3 VAL-1–3; 4 BIND-1, BIND-2; 5 Q-SYMMIX (C), S1v-V; 6 Q-VALRULES (b)'s value rows; 7 Q-CONSTSELLSB, Q-CONSTAGG, const-fn-local-unbound-width, Q-OVRXPLANE, the `const_eval_u32` literal lane; 8 rows 24, 29, 31 (EH1); 9 rows 68 + 23, the F25 row, 46, 39 (OpenTitan)
* then Q-NETSHAPE, 74-S, row 74's value half after its sinks, Q-SIDETABLES alone at a boundary, the Q-EXECSEM census any time after VAL-2, and the C rows left in their order; the D14 structure review falls at batch 5's boundary
* the honest cost: seven home rows before the first value row, about three weeks with no page or real-design payoff, then four batches before EH1 on unmodified RTL (batch 8) and OpenTitan (batch 9) (PLAN G6, §5.2 SYS-1)
* State (2026-11-10): batch 2 landed (§4.5.638): SYS-1 in two halves, SYS-1a and SYS-1b, and PARSE-1 with OPS, each structure only (every output PRE's, PRE's differences as named lanes); both keep their table rows, marked landed, for their records; next VAL-1–3 (batch 3)
* TW5 at row 33's park: five rows filed, Q-TYPEOVERVAL, Q-FRAMEREAL, Q-FRAMECONCAT, Q-ENUMCONV and Q-PORTDEFAULT (176–180 · any), within the cap of 6;
* Q-FRAMECAWAKE, measured only inside row 33's frame, and Q-FRAMEDEFAULT, row 33's residue in its record, are not filed (W/row33/park/REMEASURE-addendum.txt).
* Parked, each keeping its slot: Q-WITHREAL, Q-XNONINT and Q-INLITEM (1 · L1), P-PCTU (6 · T3), Q-DLYCAT (7 · L1), row 29's page half (C3 · T3), row 33 (C1 · L1), row 29's full form (9 · T3), Q-NESTCAT (in row 74), Q-TPCARRIER (29 · L2), Q-MEMBSRC's read half (32 · L2), Q-BITSTYPE, Q-REALSPLIT and Q-REALTWIN (42 · T3), Q-ALWINTRA (153 · any), Q-PKRDLSB (163 · any).

* Real-design first: every defect row has a corpus witness except rows 1–8, which an external report or its fix path reproduced.
* A slice that moves a corpus page re-pins that row's refusal in the same commit (`DRIFTED` otherwise) and reports the page before and after.

---

* Real-design program (owner, 2026-10-07; replaces the structural hold of 2026-10-05; re-scoped 2026-10-15): row 9's census ran OpenTitan (19 IPs, a3490b42), VeeR EL2 / EH1 and alexforencich verilog-axis / -pcie / -uart / -i2c and judged 50 roots on 28 blocked pages — structural 12 (T a declared type re-derived per consumer 6, B name binding during lowering 2, V the constant-value domain 4), local 10, other 28 — and every root is a row below or a deferred row in §7 except C-R01 (a time-0 `always @*` wake whose two values IEEE §4.7 and §9.2.2.2.2 both allow; §2-N-3).
* Scope (owner, 2026-10-15): (A) the structural stages T, B and V (row 69, which reads row 66's record, inside T; V1 and V3, rows 51 and 52, whose only witness is EL2, inside V) with the held rows 1–8 merging into a stage where they meet it, and the rows then under way, 22 (landed, §4.5.621) and 73 (parked behind ENG-3) (row 67 landed: §4.5.610, §4.5.614; row 25: §4.5.615);
* (B) the local and other roots of EH1 and OpenTitan with their corpus rows: EH1's rows 20, 22 and 25–30 and its corpus row 31, OpenTitan's rows 33–35, 37, 38 and 40 and its corpus rows 47 and 53;
* deferred to §7, each with its re-entry trigger: the EL2-only rows 54–60, EL2's corpus row 64, DPI-C's rows 61 and 62 (§3.a ⑮, not a non-goal) and verilog-pcie's corpus row 63.
* The program is complete when every row in scopes A and B has landed or is explicitly parked into a stage that is itself in scope.
* Order (2026-11-01, the page cut; W/program/q6-2026-10-31/REPLAN.txt, the Q6 check of the 2026-10-28 order below): under the rows' texts EH1 waited on 27 rows and the OpenTitan twelve on 39, of which a page line reads 4 and 18, and page-first alone moves neither date, because the parks' chains set them;
* so the C slots run first — Q-MEMBSRC's carrier, 29, 24 with corpus row 31 for EH1, then 33, 35, 37, 39, 46, 41 (its page half landed, §4.5.634), 68 with 23, 43, 42, 38, 70, 69, 29 and Q-VARBASEWR with corpus row 47 for the twelve — and the numbered slots keep the order below.
* Order (owner, 2026-10-28; W/refactor/REORDER.txt): each stage owns every shared producer of its class — T′ the declared-type record and its carriers (Q-MEMBSRC, Q-TPCARRIER, row 70 with Q-MDPARAMIDXSELF, Q-PARAM2STATE, Q-LETCAST, rows 66, 41, 68, 69, 42, 43, 38, 8, 44, 71 and 52), V′ the constant evaluators, their stores and the interpreter's environment (rows 48, 40, 49–51 and 76, Q-UNSZXPAD, Q-OVRXPLANE, Q-STRWIDTH, Q-TIMELITCONST, Q-IDXVALUE and the fold-parity rows), B′ the declaring-scope binding of lowered text (rows 45 with 2, 46, 39, 1, 3, 4 and 77) — and every producer is slotted before the rows that trust it;
* the two funnels no stage owns get a row each (74 expression type, 75 deferred hierarchical references);
* three tracks run it: lane 1 the EH1 set (Q-NESTCAT, Q-DLYCAT, rows 74 and 26, corpus row 31), the OpenTitan stack's local rows, then row 75;
* lane 2 T′ then B′, the only track that runs a T′ row, page-first since 2026-10-28 (W/refactor/REORDER.txt Q6's trip-wire fired on Q-MEMBSRC's park: row 24 slipped more than five slots, EH1 6 → 19 rounds stage-first, 12 page-first): the producer chain to row 24 first — 66 (landed, §4.5.631), Q-XFALLBACK (landed, §4.5.632) and Q-AESINK (landed, §4.5.633; row 48's design, filed from its round 3), 8, 70 (Q-PARAM2STATE inside it if one re-pin), Q-PARAM2STATE, Q-OVRXPLANE, 76, Q-LETCAST, Q-TPCARRIER (parked 2026-10-29 behind its door rows Q-WITHREAL, Q-PARAM2STATE, Q-LETCAST and lane 1's Q-2STINVSEL), Q-MEMBSRC and 24 — then the OpenTitan twelve's rows 38, 41 (its page half landed, §4.5.634), 68, 69, 42, 46, 43 and 39, then 44 and the rest of T′ and B′;
* lane 1 and track 3 keep their order except rows that wait on a moved row (31, 45 with 2, 47, 53 and 5–7 follow it);
* track 3 V′ from its no-code census and design row (row 48; T's census and design are delivered, row 18), whose design re-derives the V′ order;
* the pages keep their order — EH1 (row 31), the OpenTitan twelve (row 47; rows 71, 44 and 45 open none of its pages), V's seven (row 53);
* row 23 ships inside T as one slice with row 68 and row 39 inside B after row 46 (each parked: its build routed new values onto the stage's root class).
* A page that runs becomes a corpus row.
* Held rows 1 and 3–8 hold slots in their stages since 2026-10-28 (T′ row 8; B′ rows 1, 3 and 4; V′ rows 5, 6 and 7, rows 1–5 still before rows 6–7);
* row 2 ships with row 45;
* row 72b stays held and joins B′ only if a stage's exit re-census finds it on an opened page.
* For this program a blocking finding is fixed, never reverted for its round count (ER §3.6's two-round revert is suspended; from the second blocking round on one axis — the same root class — the axis is re-planned by a fresh planner, not patched; each round is a fresh agent, and a delta round re-runs only the cells its delta reaches, the full re-run once in the final round), and every round is reported.
* The tier letter after the rank is the review tier (ER §3.7).

* The `rank` column is a row's defect class — ① a silent-wrong, ② loud→supported (§1) — not its place in the queue: the `slot` column is the start order, and the row number is a stable name.
* Since 2026-10-28 a slot reads `<start order> · <track>` — L1 lane 1, L2 lane 2, T3 track 3, `any` the first free track whose rows under way are file-disjoint from it — and a row joined into another shows its host's slot;
* the numbered order comes from W/refactor/reorder/scripts/order.py (a stage's rows one at a time, known shared files never side by side).
* Since 2026-11-01 a slot `C<n> · <track>` (the page cut, the head of the table; W/program/q6-2026-10-31/order.tsv) goes before every numbered slot: a C row is a row a page line needs or a mechanism such a row builds on, its opening covers the shapes the pages spell while every other shape of its class keeps PRE's refusal, and a new `after` on a C row needs a page line (`file:line`);
* an `any` row starts only on a track whose next C row cannot start (its `after`, or a D12 file).
* Since 2026-11-09 a slot `R<batch>.<n> · L1` is the record-first order (W/program/reorder2-2026-11-09/PLAN.txt G6) and goes before every C and numbered slot; an R number with no row (R6, R7.3, R7.5, R9.2) is named in the row docs that hold its items
* Q-MEMBSRC's carrier left the page cut (2026-11-02, W/membsrc-carrier/RULINGS-ground.txt), so Q-MEMBSRC shows 32 · L2 with its read half
* Rows 1–5, what the external report's fix path found (§4.5.585, §4.5.587, §4.5.589), go before the ① rows 6–7 because the owner's order of 2026-10-02 keeps the report's items first.
* §2 🆕 AD's measured half landed (§4.5.589), and the four prerequisites that half left follow in ER §10.2's risk order: routing into an existing mechanism (rows 1–2) before new infrastructure (row 3 a per-scope constant table, row 4 an AST change and a format bump).
* §4.5.587 reverted §3.b `unique-const-fn` (now BLOCKED): folding the `unique` form like its plain-`if` twin inherits the twin's silent-wrongs;
* §4.5.589 closed the package-routine half of that prefix binding and rows 1–4 close the rest (§2 re-entry: that row's fix path).
* The 4-state half is row 76 (§2 🆕 AE; row 48's order: Q-AESINK (landed, §4.5.633), the result channel, the call arm, then the x plane behind row 8 and Q-PARAM2STATE);
* until the x plane lands it is a rule: §4.5.588 measured that no narrower cut fixes the interpreter's 0 for a never-assigned 4-state variable, so no arm or lane a later row opens from loud to a value may answer from an evaluation that reads one — directly, through a nested run, or through a binding stored from such an evaluation — and PRE's value stays where PRE answered.
* Q-AESINK's mark (§4.5.633, `Sidecars::const_seed_reads`: one design-wide set, no store carries it, read by `crates/cli/tests/const_seed_mark.rs` only) says an elaboration read one, in vita's own run: a premise folded at the call's scope is its own row's (rows 3, 4, 68, Q-RTNBODYSCOPE);
* it over-marks (§4.5.633 lists the shapes), the first reader's false-alarm price, 0 of 32 observed (14 corpus, 18 real), 51 not observed, and 124 of the 4,820 rows stop at E2002 before elaboration, so 0 marks there is not observed;
* it is a whole-elaboration fact, never a per-step stand-down, in any lane: PRE's loud holds in the first generate walk only, so a step falling back on the mark drops an arm at exit 0 and instance order flips the outcome (review dr1: r40b_genif_seed, e8, e9).
* The first lane's design review proves its decision is not re-made per walk;
* the candidate is a restart with every lane closed and first-run diagnostics buffered, UNVERIFIED, not prototyped;
* row 5 widens the interpreter that way and waits on rows 1–4;
* `unique-const-fn` is re-derived from its row once rows 1–4 land (ER §3.6).
* `unique-const-fn`, §3.b `unique-pkg-closure` and §2 🆕 AB's residue (§4.5.590 closed its t0 run on x; the residue is BLOCKED) block §3.b `unique-if-chain`'s subroutine-body residue.
* Rows 6–7 are §4.5.581's prerequisites: none has a corpus witness and each redesigns shared code;
* row 6 ships 🆕 W with 🆕 S (a)'s i64 half (each alone descends), and both meet §2 🆕 AI (§4.5.593), which waits on 🆕 AE (row 76; §4.5.594 reverted it whole), so row 6 waits on it;
* two more of §4.5.581's wait on 🆕 AE: §2 🆕 U's wide→narrow half (§4.5.591 closed its import half) and §2 🆕 V (§4.5.592 reverted it; row 77 takes it with the §2 "Scoping" later-walk reads line), so row 7's 🆕 T waits on rows 76 and 77.
* Row 8 (§4.5.588) unblocks 🆕 AE's `repeat` cut (row 76) and row 44's `enum bit` override, so it is a T′ producer slotted before both (2026-10-28).

---

* note: - §3.b `pkg-type-param-import`, `scoped-call-comb-arg`, the mixed-caller callee, `m #(8)` / `defparam u.T$w`, the VCD `$scope` `[0]` spelling, the `genblk<N>` collision, §2 🆕 L ⓦ / 🆕 N residues — synthetic origin, no corpus witness.

---

* note: - §3 ⑤ⓕ CLASS PROPERTY — needs per-instance class registration (`register_classes` prescans); a property typed by a module alias over a module or `$unit` localparam opens with §5.2 row 68 (rv.cprop_lp, rv.cprop_unit: iverilog and verilator `f/4`); one over a header parameter or a type parameter stays here (s3.cprop, t1.cprop: E3009 "undefined name `W` / `T$w`"), as does a property reached through a handle (`h.s.a`, r2.cls_handle: E3010; iverilog `hsa=20`, verilator `hsa=16`, a split).
