(§5.2 Q-MDRVMIX · rank ① F · source W/mdrv/ground/REPORT.txt Q1, row 24's F5 / D1 (owner, 2026-11-16)) (overlapping partial and mixed writes of a packed variable are not refused):
* purpose: a continuous and a procedural writer on overlapping bits of a packed variable that elaboration decides (IEEE 1800-2017 §6.5, §10.3.2), and an `always_comb` and another process on overlapping bits (§9.2.2.2), are E3001
* why: G-EH1, before row 24 (EH1 cut), whose K7 inputs it decides (24.md); the owner put the multidriver rule for partial and mixed writes before row 24 (2026-11-16)
* needs: Q-WREXT, Q-WEXTPOS
* readers: 24
* state: PARKED Q-WEXTPOS (2026-11-17)
* record: as filed below ; superseded order lines are kept for the record
* slot: parked 2026-11-17 at R8.1 · L1 behind §5.2 Q-WEXTPOS (owner, 2026-11-17, option A): its two commits are pulled out of batch 5 by a33df0e0 and batch 5 goes on to its boundary as PARSE-1's provenance piece and Q-WREXT; nothing of this row is on main or on the batch 5 branch
* park (2026-11-17): the class, in ER's words: a reader takes a value from a container filled for another purpose, or a context's value where the write's own was expected (ER §3.6 root class; the class premise judge 1 named for the first plan, B1 below); at the boundary the soundness lens found a legal design refused because the view's geometry constant and a kernel extent read a provisional constant as final (a hierarchical `$bits` lowered by `lower_bits_fold` in `expr_special.rs` and patched by `resolve_deferred_hier_bits` in `hier_defer/bits.rs` after the view read it: lens-sound F1), the differential lens found the E3001 text's bit range read from the normalized bounds `mdrv_bits_note` took from the net, not from the declared range (lens-diff B1), and the fix failed its premise judge three times on the same class in the text's sentences (the `.*` port name, the declaration initializer's position, a `$unit` routine's name, a gate list's and an enclosing statement's span); the verdicts loud and quiet were right on every cell, the sentences were not
  * lenses: W/batch5b/lens-sound/REPORT.txt, W/batch5b/lens-diff/REPORT.txt (with D4_TABLE.txt)
  * plans: W/wrext/fix1/PLAN.txt, W/wrext/fix1/PLAN2.txt, W/wrext/fix1/PLAN3.txt
  * premise judges: W/wrext/fix1/premise/REPORT.txt (with LINES.txt), W/wrext/fix1/premise2/REPORT.txt, W/wrext/fix1/premise3/REPORT.txt
* built and pulled: C1 0da863d1 (rule X) and C2 92579c89 (rule Y) on the batch 5 integration branch, reverted together by a33df0e0, so `crates/` is again 74b2bbe7's tree (Q-WREXT's row end); the code as built is W/mdrv/build/c1.patch and W/mdrv/build/c2.patch; what left with it: `elaborate/src/mdrv_mix.rs` and `check_mixed_drivers` with its call in `driver.rs`, the `cont_var.reported` insert in `cont_var.rs`, `WriteView::for_test`, `wext_home.rs`'s H10, the 130 pins of `crates/cli/tests/mdrv_mix_pins.rs`, the three rewritten `implicit_read_set.rs` pins (PRE's designs again) and `wext_view.rs`'s w01 (PRE's design again)
* re-entry: a fresh plan on the code as it stands then, once §5.2 Q-WEXTPOS has landed (the view stands down on a provisional constant and carries each write's own position), with as its input:
  * (a) `check_mixed_drivers` returns at its head when `had_error` is set, and the `cont_var.reported` stand-down is deleted (premise judge 2 of the fix, W/wrext/fix1/premise2/REPORT.txt D3 / D4: sound; 0 of the 130 pins and 0 of the 9923 record dirs go quiet)
  * (b) the text reduced to the variable's name, the bits clause "on bits [HI:LO] of its packed value (bit 0 is the least significant bit)", the writers' kind words and each write's own position from Q-WEXTPOS's `at` (PLAN3.txt's templates; premise judge 3's literal lines 1–12, W/wrext/fix1/premise3/REPORT.txt); the words that text loses are §5.2 Q-MDRVWHO's
  * (c) the differential lens's two filings, as this row's known silences, each PRE == POST: "target set by declaration site", interface members and package variables written by overlapping mixed writers stay silent (i48, rule X's shape, iverilog refuses it: `Cannot perform procedural assignment to bit select 'i0.v['sd0]' because it is also continuously assigned.`; n08 and n07, rule Y's shape), and "constant select left Undecided", a constant select the kernel's fold leaves `Undecided` (a negative index i61, n02, n04, n06; a multiply n21; a genvar expression i63) keeps both rules silent, iverilog refusing i61, n02, n06 and i63 (W/batch5b/lens-diff/REPORT.txt F1, F2)
  * (d) the open pair question of §3.b mdrv-ff-latch-pairs (which `always_ff` / `always_latch` sub-forms to refuse is the owner's)
* note: the manual's text for the two rules (b75c6d28, §4.5.648) left the manual with the park, since the manual says what the tree does; it stands at the end of this doc verbatim as it stood at a33df0e0, to be put back, with the re-plan's text, at this row's landing
* note: line 1 as filed at b75c6d28 (§4.5.648), kept as written: "(§5.2 Q-MDRVMIX · rank ① F · source W/mdrv/ground/REPORT.txt Q1, row 24's F5 / D1 (owner, 2026-11-16)) (a packed variable's overlapping mixed writes and always_comb pairs are E3001):"; the title is again the one it had before that commit, the defect the row closes
* note: the lines from here to the manual record are b75c6d28's built-state text, kept verbatim; where they say "built", "as built", "POST 92579c89" or name a pin of `mdrv_mix_pins.rs`, they describe the build a33df0e0 pulled out, not the tree
* slot: built 2026-11-17 on the batch 5 integration branch (§4.5.648): C1 0da863d1, rule X, and C2 92579c89, rule Y, landing with batch 5; no code on main; batch 5 closes at this row (owner, 2026-11-17)
* slot: R8.1 · L1, after Q-WREXT, the record it reads, and before row 24, which waited on it (owner, 2026-11-16, "2번이 좋을것 같음": the multidriver rule for partial and mixed writes first, a prerequisite row before row 24; main: the record first, ER §5.1)
* note: superseded slot lines (2026-11-16), kept as written: "planned, not built (2026-11-16): R8.1 · L1, after Q-WREXT, the record it reads, and before row 24, which waits on it" and "premise judge 1 failed (B1, below), the re-plan is W/mdrv/plan2/PLAN2.txt, and premise judge 2 waits on the owner's answer on rule Y (the open question below)"
* symptom (PRE, before this row): on a packed variable, a continuous driver beside a procedural write, or an `always_comb` write beside another process's write, to the same statically decided bits runs at rc 0: `multidriver.rs` rules A–D read whole writes by name only, `check_whole_net_multidriver` reads continuous rows only, and `cont_var.rs`'s sole-writer check over `other_writers` covers only variables that are not `logic` (kinds Reg, Integer, Real; W/mdrv/ground/REPORT.txt Q2)
* witness (X): m_a_ca_acomb_ppo, `logic [7:0] v; assign v[5:2] = s[3:0]; always_comb v[3:0] = ~s[3:0];`: PRE cdf0c457 rc 0 `A v=xx011010` `B v=xx110011`
  * oracles: iverilog 13.0 rc 1 `t.sv:6: error: Cannot perform procedural assignment to part select 'v['sd3:'sd0]' because it is also continuously assigned.`; verilator 5.054 `%Warning-MULTIDRIVEN: t.sv:4:15: Bits [3:2] of signal 't.v' have multiple combinational drivers. This can cause performance degradation.`, runs `A v=00011010` `B v=00110011`; sv2v → iverilog `A v=xx010110` `B v=xx110011`; IEEE 1800-2017 §6.5 (txt:3494-3497)
  * POST 92579c89 (pin m_a_ca_acomb_ppo): rc 1, `error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `t.v` is driven by an `assign` or gate output at t.sv:5:3 in `t` AND written by `always_comb` at t.sv:6:3 in `t` on bits [3:2], and a variable driven by a continuous assignment takes no other writer on those bits (IEEE 1800-2017 §6.5, §10.3.2)`
* witness (Y): m_a_acomb_acomb_ppo, `always_comb v[5:2] = s[3:0]; always_comb v[3:0] = ~s[3:0];`: PRE rc 0 `A v=xx011010` `B v=xx110011`
  * oracles: iverilog runs `A v=xx010110` `B v=xx110011`; verilator `%Warning-MULTIDRIVEN: t.sv:4:15: Bits [3:2] of signal 't.v' have multiple combinational drivers. This can cause performance degradation.`, runs `A v=00011010` `B v=00110011`; sv2v → iverilog `A v=xx011010` `B v=xx110011`; IEEE §9.2.2.2 (txt:9036-9038)
  * POST 92579c89 (pin m_a_acomb_acomb_ppo): rc 1, `error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `t.v` is written by `always_comb` at t.sv:5:3 in `t` AND by `always_comb` at t.sv:6:3 in `t` on bits [3:2], and the bits an `always_comb` writes take no write from another process (IEEE 1800-2017 §9.2.2.2)`
* evidence: the matrix W/mdrv/ground/matrix.tsv, 374 cells on PRE cdf0c457 with iverilog 13.0, verilator 5.054 (`--lint-only -Wall` and `--binary`) and sv2v → iverilog, each with its IEEE label (W/mdrv/ground/cells/m/SUMMARY.txt): correct-loud 63, correct-quiet 63, SILENT-ACCEPT 164, FALSE-LOUD 12, value-wrong 7, no-oracle-agreement 25, WARN-ONLY 3, legal-race 37
* note: premise judge 1 (W/mdrv/premise/REPORT.txt) failed the first plan (W/mdrv/plan/PLAN.txt) on B1: its target set read `intro_kind`, a container filled for the 2-state default (14 insert sites by the judge's count, 13 by PLAN2's), two of which (`inline_task.rs:440`, `inline_task_locals.rs:186` at b9cb67a1) put a static task's 2-state formals and locals in it, so rule Y refused a legal design; the class is "a positive set reading a container filled for another purpose" (row 29's park), the second instance on that axis the 6 m_p_innet_* input-port nets
* witness (B1): q8_task_static_bit_comb_ff, `task tk(input bit [3:0] x, output logic [3:0] y); bit [3:0] tt; tt = x ^ 4'h5; y = tt; endtask` called by `always_comb tk(a, v);` and `always_ff @(posedge clk) tk(b, w);`: PRE rc 0 `A v=0110 w=0000`
  * oracles: iverilog `t.sv:4: warning: user task (tk) must be automatic to be synthesized in an always_ff process.` (and the same for `always_comb`), vvp `A v=0110 w=0000`; verilator `--lint-only -Wall` no line, runs `A v=0110 w=0000`; sv2v → iverilog `A v=0110 w=0000`; the judge found no IEEE clause that refuses it (§9.2.2.2.2 extends the rule to a called function only, §13.3.1 txt:14399-14400 shares a static task's items)
  * as built: quiet, POST prints PRE's text (pin q8_task_static_bit_comb_ff): a static task's formals and locals carry no declaration fact
* record: the premise history: premise judge 1 FAIL on B1 → the re-plan W/mdrv/plan2/PLAN2.txt (T2 by declaration fact, attribution by `via`, rules X and Y, the 442-cell expectation W/mdrv/plan2/expected2.tsv) → premise judge 2 PASS, 0 BLOCKING (W/mdrv/premise2/REPORT.txt: K1–K9 by methods other than PLAN2's, the literal lines L1–L11 for the build; its population sweep, 16400 cells, 8693 elaborate, X 0, Y 0, 56 no observation) → the owner's answer on rule Y (below) → build rounds 1 (C1) and 2 (C2) (W/mdrv/build/NOTES.txt)
* status: DECIDED 2026-11-17 (owner): "always_comb에서의 결정은 오류로 막는걸로. IEEE 규격이 최우선이야." — option (a), rule Y is E3001, and IEEE first where it says "shall not", even where the oracles warn or run the design (W/program/state.txt 2026-11-17); the question it answered, rule Y, an `always_comb` and another process writing the same decided bits of a packed variable, 27 matrix cells (PLAN2 section 3, its facts and options as written there):
  * always_comb × always_comb, 11 cells: illegal by §9.2.2.2 (txt:9036-9038); iverilog runs, a value; verilator `%Warning-MULTIDRIVEN: .. Bits [..] of signal 't.v' have multiple combinational drivers` (lint, runs); the whole twin m_a_acomb_acomb_ww E3001 on PRE
  * always_comb × always, 3: the same clause; iverilog runs; verilator's MULTIDRIVEN warning (runs); m_a_acomb_alw_ww E3001
  * always_comb × always_ff, 3 (+2 through a task): the same clause; iverilog runs; verilator `%Error-BLKANDNBLK: .. Unsupported: Blocking and non-blocking assignments to potentially overlapping bits of same packed variable`; m_a_acomb_aff_ww E3001
  * always_comb × always_latch, 3: the same clause; iverilog runs; verilator `%Warning-COMBDLY` (not a driver line; runs); m_a_acomb_alatch_ww E3001
  * always_comb × initial, 3 (+2 through a task): the same clause; iverilog runs; verilator no line, runs; m_a_acomb_init_ww E3001
  * no tool refuses any of the 27 as a legality error (MULTIDRIVEN 14, COMBDLY 3, no line 5, "Unsupported" 5); xcelium 0 observations; on 13 of the 27 the tools that run the design print different values from each other, so there is no oracle value to match; every whole-variable twin is already E3001 on PRE (rules A and B)
  * real designs: X 0 and Y 0 on the 14 of 51 that elaborate (premise judge 1 K4, an over-approximating probe); row 24's K7 #1 and #11 are Y, their literal twins rc 0 on PRE
  * (a) Y in as E3001, on IEEE's "shall not", the partial twin of an error PRE already gives on the whole variable (PLAN2's recommendation; it overrides the 2026-09-11 guide that a cell the second tool accepts can only be a warning, as the 2026-10-06 ruling #4 did) — chosen
  * (b) Y out, rule X only: K7 #1 and #11 stay open as filed, row 24's named spelling printing what its literal twin prints (rc 0 on an IEEE-illegal design)
  * (c) Y as a warning (a W code, the design runs with PRE's value): what verilator does on 14 of 27, the partial pair W where its whole twin is E
* fix (PLAN2, as planned before the build): T2, the nets a declaration fact names (Q-WREXT's `decl`: site Module, Generate, AnsiPort or NonAnsiPort, kind `logic`, `reg` or `bit`, direction Internal or Output, not typed by a type parameter, no unpacked dimension), with no subtraction and no read of `intro_kind` or a net's name
* fix (PLAN2, as planned): writer attribution by Q-WREXT's `via`: rule X, a continuous writer (assign, gate, port) beside a procedural one (`always_comb`, `always_ff`, `always_latch`, `always`, `initial` or `var_init`, Direct or TaskInline, or a frame body); rule Y, an `always_comb` Direct write beside another process (those kinds but `var_init`) writing Direct or TaskInline; decided extents intersecting, on a T2 net, E3001; a deferred assertion's action (`DeferAction`), `final`, `sva` and force / release are not writers of either rule
* evidence: the expectation W/mdrv/plan2/expected2.tsv, 442 cells (374 matrix, premise judge 1's 56, PLAN2's 12): loud under option (a) X 75 + Y 32 = 107, under (b) 75, under (c) 75 errors and 32 warnings; in the matrix SILENT-ACCEPT → loud 97 under (a), 70 under (b), untouched 277 / 304
* evidence: P0-b on Q-WREXT C2 3bc88674: the recorded facts joined with expected2.tsv under T2, X and Y as written give its class on 442 / 442 cells, 0 mismatches (X 75, Y 32; W/wrext/build/p0b/P0B.txt)
* record: one home, `elaborate/src/mdrv_mix.rs`, a reader of Q-WREXT's write view through one entry, `check_mixed_drivers` (called once at the end of `run`, `driver.rs:1045`, after `check_var_cont_assign_sole_writer`); it decides with the view's facts only (`WriteView::writes`, `WriteView::decl`) and reads no `intro_kind` and no net name to decide (the name is read once, for the message); its home rules are H3, H4, H8, H9 and H10 of `crates/elaborate/tests/wext_home.rs`, its pins `crates/cli/tests/mdrv_mix_pins.rs`
* record: the target set T2 (PLAN2 §1, premise judge 2 K1): a net with a declaration fact whose site is a module body, a generate scope, a module's ANSI header or its non-ANSI port, whose declared keyword is `logic`, `reg` or `bit` (not `ir::NetKind`, which folds `event` and the atom types onto `Reg`), whose direction is internal or output (an input or inout port is a net, IEEE 1800-2017 §23.2.2.3), and which a type parameter does not type; every net with no fact (task and function formals and locals, loop variables, temporaries, implicit nets, interface and package members, block locals) is outside
* record: PLAN2's fifth term, "no unpacked dimension", is not built: the view gives `Word` to every chunk with a word index and a `Word` never compares, and over the 442 lane views the 48 nets with `array_len > 1` carry no `Bits` write (premise judge 2 L6: its mutant N12 equivalent); a 1-element unpacked array is quiet by the same `Word` (k7_arr_whole); a UDP's own nets carry facts and are targets by the fact's definition, with no pair reachable today (k3_udp_seq_disjoint quiet; k3_udp_out_comb, a parent's variable on a primitive's output, loud; premise judge 2 L9)
* record: rule X's sides: a continuous writer, a `cont_assigns` row of kind `assign` (a user `assign`, or a gate output, which arrives with the same kind) or `port` (an instance port hookup), and a procedural writer, a blocking or non-blocking statement (never a `force` or a `release`) of a process of kind `always_comb`, `always_ff`, `always_latch`, `always`, `initial` or `var_init` (P6), `via` Direct or TaskInline (a static task's body or copy-out, in its caller's process), or a statement of a frame body (a function, an automatic task, a task called hierarchically)
* record: rule Y's sides: an `always_comb`'s own blocking or non-blocking statement (`via` Direct) and a statement of another process (another `pid`) of kind `always_comb`, `always_ff`, `always_latch`, `always` (an `always @*` among them) or `initial` (P5, P6 without `var_init`), `via` Direct or TaskInline; the two arms of a fork are one process (c08_fork_arm), one `always_comb` elaborated in two instances is two (k3_two_inst_hier); two statements of one process are never a pair
* record: neither rule's writer: a statement `via` DeferAction (a deferred assertion's action), the kinds `final`, `sva`, `covergroup`, `clocking` and `net_init`, a `force` / `release`, and on rule Y a declaration initializer and a frame body; `Word`, `Undecided` and `Poison` extents never compare
* record: the pair test: per target net, the decided (`Bits`) intervals of both sides sorted by `(lo, view order)` and swept once, keeping the two largest `hi` of distinct writers per side (one maximum held by the incoming writer's own interval hides a narrower interval of another: k6_y_sweep_second_best); the unit test `the_sweep_finds_a_pair_iff_brute_force_does` checks it against brute force on random intervals (premise judge 2 K6); the pair reported is the sweep's (premise judge 2 L5)
* record: X before Y and one E3001 per variable: a net rule X reports is not asked rule Y (y_x_first_one_net), the two rules' errors come out together, lowest net first (y_x_and_y_two_nets); PLAN2 did not order the rules, this order is the build's
* record: the stand-down: a net an older checker reported (`cont_var.reported`: rules A, B and D's E3001, `check_whole_net_multidriver`'s E3001, the whole-array E3009 and, since C1, `cont_var.rs`'s own E3018) gets neither rule (m_a_acomb_acomb_ww, m_a_acomb_aff_ww print PRE's whole-variable E3001); a `multidriver.rs` W3060 records no net, so rule Y adds its E3001 there (y_w3060_and_y prints the W3060 line and then rule Y's E3001)
* record: the text has no location prefix and names both writers, each with what is located and how (premise judge 2 L4): (a) a `.*` connection has no source position and says so (k4_wild_port: ``driven by the `.*` connection of instance `t.u` to its port `v` (a `.*` connection has no source position)``); (b) a frame body is named by its routine and that routine's declaration (m_c_func_comb_ca_ppo: ``written by the body of subroutine `t.fA` declared at t.sv:5:28``); (c) two instances of one module share one `file:line:col` and read apart by scope (l4_two_inst: ``at t.sv:5:3 in `t.u1` `` and ``at t.sv:5:3 in `t.u2` ``); (d) the kind `assign` cannot tell an `assign` from a gate output, so the text says "an `assign` or gate output" (m_g_gate_acomb_bp)
* record: the bits are storage bits `[hi:lo]` of the overlap, and the text says "(counted from its least significant bit as 0)" when the declared range is not `[w-1:0]` (l4_range_note: `logic [8:1] v`, ``on bits [5:4] (counted from its least significant bit as 0)``); a write through a static task says " (through a subroutine it calls)" (m_c_task_init_ca_ppo)
* evidence: IEEE 1800-2017 (W/ref/ieee1800-2017.txt): rule X §6.5 txt:3495-3497 "it shall be an error to have multiple continuous assignments or a mixture of procedural and continuous assignments writing to any term in the expansion of a written longest static prefix of a variable", txt:3522-3523 a declared variable initialization "is considered a procedural assignment", txt:3527-3529 a continuous assignment implied by an instance output port ("This makes additional procedural or continuous assignments to a variable connected to the output port of an instance illegal"), §10.3.2 txt:10283-10285; rule Y §9.2.2.2 txt:9036-9038 "The variables written on the left-hand side of assignments shall not be written to by any other process. However, multiple assignments made to independent elements of a variable are allowed as long as their longest static prefixes do not overlap", §9.2.2.2.2 txt:9082-9084
* evidence: P0-b, count only, on probe builds of the rules as written (C1 W/mdrv/build/p0b/bin md5 0ecabdbe, C2 W/mdrv/build/p0b2/bin md5 5e1b7bcc; W/mdrv/build/out/p0b, p0b2, p0b2rt): the lane wext_lane, 442 cells, X 75 of 75 and Y 32 of 32 against expected2.tsv, 0 mismatches; the eleven other lanes, 1138 cells, X 0, Y 0; the record population, 8368 cells, X 1 (W1 below), Y 0; testdata, 8832 cells, X 0 (3 stand-downs: td_AI/s594__g__c2__Sdly_E, _L, _R, a net an older check already reports, PRE err=1), Y 0; the 51 real designs: 18 reach the end phase, 0 X and 0 Y lines, the other 33 stop before it (no observation)
* note: named differences against PRE (each illegal by the clause it cites): the 107 wext_lane cells (expected2.tsv axes X 75, Y 32; W/mdrv/build/expected_moved.tsv), each now E3001 where PRE ran it; two of them, m_c_func_comb_ca_ppo and m_c_func_comb_ca_ww, also lose PRE's run-phase line `warning[VITA-W4030] W-RUN-BACKEND-FALLBACK: requested backend `native` cannot run this design (a subroutine that WRITES a net outside its own frame: S3b); ran on `vm` instead — the result is unaffected, the speed is` because the design is refused at elaboration, before any run (vrun skipped; main's ruling 2026-11-17: listed_ok, the row-end comparator was too strict)
* note: W1, the record cell row24_113/D1.d1.mdrv_ca_hs (W/homes/cells/row24/design/c/d1.mdrv_ca_hs/t.sv: a packed struct `h`, `assign h[K-1:K-4] = x;` with `K = 9` beside `initial begin h = '0; ...`): PRE rc 0 `A mc h=000`; POST rc 1 `error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `top.h` is driven by an `assign` or gate output at t.sv:2:3 in `top` AND written by `initial` at t.sv:3:3 in `top` on bits [8:5], and a variable driven by a continuous assignment takes no other writer on those bits (IEEE 1800-2017 §6.5, §10.3.2)`; illegal by §6.5 txt:3495-3497
* note: three `crates/cli/tests/implicit_read_set.rs` pins whose old design is illegal by §9.2.2.2 (txt:9036-9038), `initial begin g = 0;` writing the `g[1]` an `always_comb` writes, so rule Y's E3001: rp1_local_twin_bit_write_covers_that_bit, rcond_local_twin and rtwo_local_twin, rewritten (C2) so the `initial` writes only bits the `always_comb` does not (`g[0]`, `g[3:2]`; in rtwo `g[0]`, `g[3]`), keeping what they pin (the implicit read set); PRE prints the pinned lines on the new designs; old and new designs with their oracle text: W/mdrv/build/pins/irs/rp1_orig, rp1_new, rcond_orig, rcond_new, rtwo_orig, rtwo_new
* note: `crates/cli/tests/wext_view.rs` w01_cont_and_comb_parts made disjoint (C1: `always_comb v[1:0]` for `v[3:0]`): its overlapping pair is rule X's E3001 and has no IR to render
* record: pins: `crates/cli/tests/mdrv_mix_pins.rs`, the whole stdout, stderr and exit code of `vita t.sv` on POST; each loud pin's doc carries PRE's text (W/wrext/build/bin2 aa1604e9), the IEEE clause with its line and each oracle's line with lint or error stated; each quiet pin's doc says why neither rule fires (outside T2, not a writer, no overlap, an older check reported the variable, or a named residue); C2 flipped C1's rule Y pins from PRE's text to loud and k3_two_inst_hier from quiet to loud (C1's note "disjoint terms: no pair" was false: premise judge 2 K3 lists it as Y); `crates/cli/tests/multidriver_process_pairs.rs`'s silent partial pins keep their verdicts (none has an `always_comb` side); a unit test over `WriteView::for_test` (test only) for rule Y's sides
* record: mutants, each named killer run on its built mutant (W/mdrv/build/mutants.tsv): C1 battery a (25 rows) FAILED on four: M2 killed by an unnamed test (`cont_assign_variable_drivers` force_over_an_assign_is_not_a_driver, not k3_force_over_assign: the named killer corrected), M7 survived (the report re-read both extents as `Bits` and skipped the pair, a second guard masking the sweep: fixed, the sweep's pair now carries the two compared intervals and the report reads no extent; killer p7_loopidx), N4 survived (no rule X pin had an atom kind: new quiet pin k8_int_port_x), N10 no result (the anchor, fixed); battery b: 21 killed, 0 survived, 3 controls agree (K8, C1a, C1b)
* record: C2 (D8 delta: rule Y's M5, M6, M13-Y, N1, N6, N7-Y, N9, P1 X before Y, P2 net order, O1 orientation, M11-Y the stand-down, and C1's rows in the files C2 touched; K8, M9 and U2 not re-run, their files untouched): 30 killed, 0 survived, 2 controls agree (C1a, C1b)
* record: N2 (a static task's formals and locals get a fact) is equivalent at the rule level: the only continuous writer of a task item, `assign tk.x[3:0]`, is E3010 on POST (probe W/mdrv/build/mutants/n2_probe/t.sv: "undeclared hierarchical name `tk.x`"), so no X pair reaches a task item, and every write to a task item is TaskInline, never rule Y's `always_comb` side (premise judge 2 L6); N12 has no code (the term is not built)
* record: row-end runs on 92579c89 (bin2 vita-post 929bd081, PRE W/wrext/build/bin2 aa1604e9; W/mdrv/build/out): gate PASS (re-run bare; the first try's NO_RESULT was a launch-line deadlock); record: 8341 cells compared, 8339 same, W1 as listed, 1 at the record's cap edge (fixd_all7/ones1092__qvalrulesd__plan__chain__md_12, re-run PASS); lanes sys1 175, sys1b 148, parse1 123, val1 265, val2 85, val3 107, b3pins 6, bind1 61, bind2 53, b4pins 23 and parse1b 92 same; wext_lane 442: 335 same, 107 listed, of which the two W4030 cells above by main's ruling; 51 real designs same (EH1 `TEST_PASSED`, minstret 443, mcycle 1362; 0 E3001 lines)
* record: cost (W/mdrv/build/out/cost/SUMMARY.txt, instructions and wall, PRE against POST): cost30 PASS, 0 flagged, worst +0.61 % instructions (k_str, 20,875,635 → 21,003,227); cost_wext PASS, 0 flagged, worst +0.75 % (y_wext_procs); cost_mdrv PASS (y_mdrv_one_net, y_mdrv_mix, y_mdrv_512), 0 flagged, worst +0.87 % (y_mdrv_one_net)
* note: stays out (owner option (a) covers the `always_comb` rule only), each keeping PRE's output: the `always_ff` / `always_latch` / plain pairs (R-FF, 32 cells), as illegal by IEEE as Y's, kept out as a filed residue because a measured counter-population exists there (the memory-initialisation idiom, `initial` or `$readmemh` beside `always_ff`, 12 pins, and an error built once and reverted because it rejected working RTL, `multidriver.rs:62-63`, line corrected 2026-11-17, was :50-51), because §9.2.2.4 (txt:9105-9107) has no independent-element sentence, so a rule from its text also refuses the disjoint pairs real designs spell (spid_status.sv:165 / :176 / :189, otbn_rf_bignum_ff.sv:56 / :62, el2_veer_lockstep.sv:447 / :457), and because no tool prints a driver line on any ff × ff partial pair; filed as §3.b mdrv-ff-latch-pairs, open
* note: also out: unpacked words (15 cells, §5.2 Q-ARRWORDMD), continuous × continuous (6 cells, `check_whole_net_multidriver`, §5.2 Q-MDCONSTOFF), input-port nets (the 6 m_p_innet_* cells: a net, IEEE §23.2.2.3; a procedural write to an input-port net is filed as §3.b proc-write-input-net, 2026-11-17), run-time prefixes (5 cells, R-RT: p7_loopidx splits the oracles), the channels with no statement (Q-WREXT's OUT list), nets, unresolved hierarchical names (W/mdrv/plan/PLAN.txt B2)
* note: not built, each keeping PRE's text in a quiet pin, with its filing (W/mdrv/build/RESIDUES.txt, each block with PRE's and each oracle's text):
  * 1 R-RT, a run-time index beside another writer (p7_loopidx, k4_loop_comb_ff): §3.b mdrv-runtime-prefix
  * 2 R-ATOM, an atom-typed variable (k8_int_port_x, c10_int_kind_y): §3.b mdrv-target-kinds
  * 3 L8a, a type-parameter-typed variable (k7_tparam_overlap): §3.b mdrv-target-kinds
  * 4 L8b, an unpacked array written whole and per element (k7_arr_whole): §5.2 Q-ARRWORDMD
  * 5 R-VOIDFN, an `always_comb` writing only through a void function called as a statement (k2_voidfn_comb_ff): §3.b mdrv-subroutine-sides
  * 6 L2, a task enabled hierarchically from another scope (k2_hier_task): §3.b mdrv-subroutine-sides
  * 7 L3, a clocking-block output drive (k3_clocking_out): §3.b mdrv-ca-call
  * 8 an input-port net written procedurally (m_p_innet_acomb_part, m_p_innet_init_whole, y_q_innet): §3.b proc-write-input-net
  * 9 R-FF, `always_ff` / `always_latch` pairs with no `always_comb` side (m_a_aff_aff_ppo, m_a_aff_alatch_ppo, k3_latch_ff): §3.b mdrv-ff-latch-pairs
  * 10 R-TASKA, an `always_comb` writing the bits only through a static task it enables (c01_acomb_viatask_init, c02_acomb_viatask_acomb): §3.b mdrv-subroutine-sides
  * 11 R-FN, a function's frame body on rule Y (m_c_func_comb_acomb_ppo, k3_fn_two_comb, k2_fnframe_comb_ff): §3.b mdrv-subroutine-sides
  * 12 R-FINAL, `final` on either rule (c09_final_x, k3_final_comb): §3.b mdrv-unlisted-writers
  * 13 `Word` on rule Y, an unpacked element written by `always_comb` and `initial` (y_q_word): §5.2 Q-ARRWORDMD
  * 14 a deferred assertion's action writing a module variable (c12_assert_action_y): §3.b mdrv-unlisted-writers
* note: inputs to premise judge 2, PLAN2 statements the Q-WREXT C2 build measured false (W/wrext/build/NOTES.txt round 2): `elaborate_netvar_decl` has 6 call lines, not 7 (instance.rs:955, generate.rs:705, iface_inst.rs:604, package.rs:1272, block_local/hoist.rs:305 and :714, the same at 3bc88674); `elaborate_ports` has an interface caller PLAN2's row 13 does not name (iface_inst.rs:576, `DeclScope::Iface`, no fact); a type parameter's kind follows the instance's type, not the default's (an instance overriding `T = bit [7:0]` records `Module/Bit/Internal/tparam`)
* note: EH1's six false E3018 (ahb_sif.sv:216 / :219 / :220, imem and lmem) are not this row: row 29's nested select is refused at `lval_base_net` (`elaborate/src/lvalue.rs:551`, E3009) and returns `POISON_NET` (`limits.rs:49`), whose chunk sets `other_writers`' design-wide `unknown` (`cont_array.rs:367` / `:450`), so `cont_var.rs:204` (line corrected 2026-11-17, was :202) refuses every recorded variable `assign` in the design (`Refusal::Unresolved`; W/mdrv/ground/REPORT.txt Q3, cells q3_portnest and q3_portnest_ctl); §3.b nested-lval-select owns it
* note (2026-11-17, roadmap review W/program/roadmap-review-2026-11-17/FIXLIST.txt G2): §5.2 Q-MIXCAPROC owns this row's rule X mechanism on its cells (`assign y[3 +: 2]` beside `always_comb y[4]`, IEEE §6.5); compared by design text with `mdrv_mix_pins.rs` (no run), no pin spells any of its eight cells, so a re-measure on POST is owed for all eight (Q-MIXCAPROC.md lists them and what rule X's text says of each)
* source: W/mdrv/ground/REPORT.txt (Q1–Q8), W/mdrv/plan/PLAN.txt, W/mdrv/premise/REPORT.txt, W/mdrv/plan2/PLAN2.txt, W/mdrv/premise2/REPORT.txt, W/mdrv/build/NOTES.txt (rounds 1, 2), W/mdrv/build/RESIDUES.txt
* record (manual, at a33df0e0): `docs/manual/003_language-reference.md`, the multiple-driver paragraph as b75c6d28 wrote it (the manual now reads as before b75c6d28: "The rule reads", "are not checked."), verbatim:
```text
Multiple drivers are legal on a net, not on a variable (IEEE 1800 §6.5). A
variable driven by a continuous `assign` or a gate output — `logic y;`, or an
`output logic y` port — takes no other driver: a second whole `assign` or gate
output on it, a procedural write in an `always`, `always_latch`, `initial` or
`final` block, a procedural `assign`, or a declaration initializer is
`E-ELAB-MULTIDRIVER` / `VITA-E3001`, as Icarus Verilog refuses it. Declare the
signal a net (`wire`) to have its drivers resolved. A `force` is not a driver
(§10.6.2). That rule reads whole-variable writes at module scope only: a select,
element or member write, a write through a task, a hierarchical name or a system
task's output argument, and a port binding, a UDP instance, a generate block, a
`generate … endgenerate` region or an interface body driving the same variable
are not checked by it. On a variable that is not `logic` — `reg`, `integer`, `time`,
`real`, `realtime`, a 2-state type — they are: a continuous `assign` there is kept
only as the variable's sole writer, driving all of it, and is otherwise
`E-ELAB-LVALUE-KIND` / `VITA-E3018` (§9.4).
A second rule reads the bits: on a `logic`, `reg` or `bit`
variable declared in a module (not an input port), a continuous driver — an
`assign`, a gate output or an instance's output port, on the whole, a select or a
member — and a procedural write in an `always`-family or `initial` block, a declaration initializer or a
function or task body on overlapping bits fixed at elaboration is `VITA-E3001`
(§6.5; on a `reg` or `bit` variable an `assign` beside another writer is that
`VITA-E3018`); and an `always_comb` and another process writing overlapping bits is
`VITA-E3001` (§9.2.2.2). Writes to disjoint bits are legal. A select with a run-time
index, an unpacked array element and the other cases listed under
`E-ELAB-MULTIDRIVER` in [007](007_error-codes.md) are not read by it.
```
* record (manual, at a33df0e0): `docs/manual/007_error-codes.md`, `E-ELAB-MULTIDRIVER` in detail, the opening sentence as b75c6d28 wrote it (the manual now reads "Four shapes reach this code."), verbatim:
```text
Six shapes reach this code; the last two read the bits a write to a variable
covers.
```
* record (manual, at a33df0e0): `docs/manual/007_error-codes.md`, the paragraphs from the gate-output sentence to the end of the two new shapes as b75c6d28 wrote them (the manual now reads as before b75c6d28 there, but for the Verilator sentence, which stays as b75c6d28 corrected it: "Verilator's lines on the overlapping partial pairs measured vary: a `MULTIDRIVEN` lint warning, a `BLKANDNBLK` \"Unsupported\" error, or none."), verbatim:
````text
A gate output counts as a continuous driver, and an `always_latch` beside the
`assign` is this error too, not the latch warning below. A `force` over the
`assign` is not a second driver (IEEE §10.6.2). An `always_comb` or `always_ff`
beside the `assign` is the previous shape, with its own message. The three variable shapes above read only whole-variable writes at module
scope: a select, element or member write, a write through a task or a
hierarchical name, a port binding, a generate-block `assign`, an interface body
and an unpacked array (which is `VITA-E3009` when its whole-array `assign` has
another writer) are outside them. The fifth and sixth shapes, below, read a select,
a member write and a port binding by the bits they cover.

In those three only a *direct*, *whole-variable* write counts. Direct means an assignment
(including `force` and a procedural `assign`), not a write that reaches the
variable through a task or function call — `initial wt();` where
`task wt(); v = 3; endtask` is not a driver here, because a variable merely passed
to an `input` formal would otherwise be reported too. Whole means the lvalue is the
bare identifier: a struct member (`s.x`), an array element (`mem[a]`), a bit
(`bits[0]`) or a part select is a partial write and does not make a pair there.
Verilator's lines on the overlapping partial pairs measured vary: a `MULTIDRIVEN`
lint warning, a `BLKANDNBLK` "Unsupported" error, or none. Xcelium's behaviour on a partial write has not been measured.

A **variable** declared `logic`, `reg` or `bit` in a module body, a generate block
or a module's `output` port, driven on some bits by a continuous driver (an
`assign`, a gate output or an instance's output port) and written on overlapping
bits by a blocking or non-blocking assignment — in an `always_comb`, `always_ff`,
`always_latch`, `always` or `initial` block, a static task such a block calls, a
function or automatic task body, or a declaration initializer — is two writers on
those bits under IEEE 1800 §6.5 and §10.3.2. Writes to disjoint bits are legal and
run. On a `reg` or `bit` variable an `assign` beside another writer is
`VITA-E3018` (`E-ELAB-LVALUE-KIND`) instead, reported first; an output port bound to
it is this shape:

```systemverilog
// t.sv
module t;
  logic clk; logic [7:0] s;
  logic [7:0] v;
  assign v[5:2] = s[3:0];
  always_comb v[3:0] = ~s[3:0];     // bits [3:2] have two writers
endmodule
```
```
error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `t.v` is driven by an `assign` or gate output at t.sv:5:3 in `t` AND written by `always_comb` at t.sv:6:3 in `t` on bits [3:2], and a variable driven by a continuous assignment takes no other writer on those bits (IEEE 1800-2017 §6.5, §10.3.2)
```

A **variable** of the same kinds whose bits an `always_comb` writes, written on
overlapping bits by another process (`always_comb`, `always_ff`, `always_latch`,
`always`, `always @*` or `initial`, directly or through a static task it calls), is
two writers under IEEE 1800 §9.2.2.2: "The variables written on the left-hand side
of assignments shall not be written to by any other process", while independent
elements whose longest static prefixes do not overlap may be. No tool measured
refuses these as illegal (Icarus Verilog runs them; Verilator warns, runs them or
calls some unsupported); vita follows the standard:

```systemverilog
// t.sv
module t;
  logic clk; logic [7:0] s;
  logic [7:0] v;
  always_comb v[5:2] = s[3:0];
  always_comb v[3:0] = ~s[3:0];     // bits [3:2] are written by two processes
endmodule
```
```
error[VITA-E3001] E-ELAB-MULTIDRIVER: variable `t.v` is written by `always_comb` at t.sv:5:3 in `t` AND by `always_comb` at t.sv:6:3 in `t` on bits [3:2], and the bits an `always_comb` writes take no write from another process (IEEE 1800-2017 §9.2.2.2)
```

These two messages have no location prefix and name both writers. Two instances of
one module read apart by their scope (`in t.u1`, `in t.u2`); a `.*` connection, which
has no source position, says so; a function body is named by the function and where
it is declared. The bits are counted from the variable's least significant bit as 0,
and the message says so when the declared range does not end at 0 (`logic [8:1] v`).
One error is reported per variable, and a variable one of the first four shapes
(or `VITA-E3018`) already reports gets no second error.

Both read only bits fixed at elaboration (a constant select, a struct member, a port
binding). They do not read a select with a run-time index (`v[i]` in a loop), an
unpacked array element, an `int`-family or type-parameter-typed variable, an input
port, a `final` block, a deferred assertion's action, a clocking-block drive, a
function's writes inside an `always_comb` (and a static task or void function an
`always_comb` calls, which is not its own side of the second rule), or a pair of
`always_ff` / `always_latch` / `always` / `initial` writers with no `always_comb`
and no continuous driver among them; those partial writes run as before.
````
