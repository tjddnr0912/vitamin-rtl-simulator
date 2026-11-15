(§5.2 Q-CONSTSELLSB · rank ① S · source Q-AESINK review r1 (dC1); row 29 page half r3 lens-sound C1) (constant interpreter select writes ignore the declared LSB and direction):
* slot: R7.1 · L1 (batch 7, after batch 6), on BIND-2: a local's `RangeFacts`, read by select writes and the read twin (owner, 2026-11-09, the record-first order; it was any · L2)
* slot: PARKED 2026-11-15 with Q-CONSTAGG, keeping R7.1 · L1, where it ran as batch 5's first row (owner O1, 2026-11-15): its premise class broke twice, at the build's c2 and at premise judge 2, nothing landed, and the next row is the owner's (the park below)
* the constant interpreter's select write numbers from 0 whatever the declaration says: `ConstWidths` holds a width and a sign, no LSB or direction, so `exec_const_select_write` (`elaborate/src/const_fn.rs`) stores `t[i]` at storage bit `i`: k_off_b `bit [5:2] t; t[2] = 1; t[3] = 1;` `00001100`, verilator `00000011`, iverilog and sv2v `0000xx11`
* k_asc_b `bit [0:3] t` `00001101`, all three `00001011`
* a 4-state local is the same value under Q-AESINK's `numbered` flag: v1_loop0_w1 `11111111`, all three `x1111111`
* IEEE 1800-2017 §11.5.1
* fix: record the LSB and the direction beside the width, and the read twin (§2 "A non-zero-LSB local in a concat") reads the same record
* the record's LSB is the declaring scope's, not the call's (sc_lsb_gif)
* a child-local function in a port range reaches a row 29 page-shape output actual (row 29 page half r3 lens-sound C1, W/row29/page/lens3-sound/c/f/g10_formal_fn; a source inside the child is a filing, RULINGS-plan3b O4 (a), beside y01–y05 of W/row29/page/r3/docs.patch): g10_formal_fn `module c (output logic [g()-1:0] o); function automatic int g(); bit [5:2] t; t = 0; t[2] = 1; t[3] = 1; return t; endfunction assign o = '1; endmodule` with `logic [1:0][15:2] r; c u(.o(r[1][9:6]));` (`g()` 12 in vita, 3 by hand) PRE `t.sv:7:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: nested lvalue select (v1: single-level) [in top]`, r3 `R xxxxxx1111xxxxxxxxxxxxxxxxxx`, iverilog 13.0 `R zzzzzz0111zzzzzzzzzzzzzzzzzz` (`t.sv:7: warning: Port 1 (o) of module c expects 3 bit(s), given 4.`), sv2v → iverilog the same, verilator 5.052 `R 0000000111000000000000000000`
* the 1-D twin `R xxxxxx1111xxxx` on PRE
* the page shape stays loud on main (row 29's page half parked)
* note: BIND-2 landed without the merged per-local record (§4.5.640): its first commit adds the lsb and direction as a private `ConstFrame` field filled by `bind_shape` (W1 W6 W9); P0-c's pair lookups per iteration (2.0 on 16 cells, 3.0 `z_sel_write_loop`, 4.0 `z_concat_local`, 6.0 `y_env`) bound a merge's saving loosely, not a saving (W/bind2/RULINGS-plan.txt R8)
* note: tier S (W/constsellsb/plan/PLAN.txt Q1 TIER: the row edits shared code, `bound/frame.rs`'s writer and `const_placement_wide`'s resolver; ER §3.7)
* note: the declaring-scope line's cells (:9) sc_lsb_gif, sc_lsb_gif2, sc_w_gif, sc_lsb_mod and sc_rw_gif printed PRE's text on P0's p0b (W/constsellsb/p0/SUMMARY.txt `B_PRETEXT stay`); their shape, a body-local range folded at the call prefix, goes to §5.2 row 3 / §2 🆕 AG (PLAN.txt F1)
* note: the x half goes to §5.2 76 (PLAN.txt F2): on v1_loop0_w1, v2_loop0_ret, nz_mod_off and n17_4st_off P0's p0b printed the bits the oracles define and 0 where they print x (v1_loop0_w1 `A P=01111111`, all three `x1111111`; `B_NAMED_N checked=4 match=4`)
* PARKED 2026-11-15 with Q-CONSTAGG (W/constsellsb/plan4/PLAN4.txt V2): the build stopped in c2 on a broken premise, plan pass 3's re-derived table failed premise judge 2 on the same class (F1) and a new one (F2), and ER §3.6 names revert and prerequisite for a root returning through another door
* nothing landed: T is back at dfa5c625 (`git diff dfa5c625 -- crates/` empty); c1 and c2 are kept as records, not bases (W/constsellsb/build/c1.patch md5 307baa3e, build/c2_wip.patch 8e970a8b, park/T_final.patch 6e021275 with the untracked files)
* P0 (W/constsellsb/p0/SUMMARY.txt): p0a, the record filled inside `bind_shape`, tripped TW-b (`TW-b=1`) on y_env, 13,390,664,364 → 14,039,466,661 instructions (+4.845 %; +4.851 / +4.841 % by order), output equal; TW-b allowed one cheaper spelling (PLAN.txt TRIP WIRE)
* spelling L (W/constsellsb/plan2/PLAN2.txt D1, "a frame with no Shifted declaration runs PRE's code plus one text test per declaration and one null test per frame"; p0L/SUMMARY.txt): y_env +0.688 %, and R-b tripped on e5_paren0, a `[(3):0]` formal parsed twice, at +6.891 % instructions
* e5_paren0's cost: +848.8 instructions per frame over e1 (premise/REPORT.txt Q2; p0L/SUMMARY.txt `PERUSE_DIFF` +848.7)
* the owner accepted e5 on 2026-11-15 as the cost PLAN2.txt:42 had named (``e5 `[(3):0]` keeps two parses (+825 per bind, named)``), on the condition of premise judge 1's census (W/program/state.txt, W/constsellsb/BRIEF-build.txt)
* premise judge 1 PASS (W/constsellsb/premise/REPORT.txt Q1): a text census of every function, 334 in the 51 real designs (587 files) and 1024 in the corpus (1456 files), found 1 shifted declaration, none in the corpus
* real designs: that one, `logic [31:7] unused_insn_data` in `insn_is_branch` (otbn_instruction_fetch.sv:76, two copies), has one call, at :447 in an `always` block, and is never bound in a constant call (constant-context binds 0, REPORT.txt Q1)
* build (W/constsellsb/build/NOTES.txt): c1, structure only, verified: 152 cells print as PRE (cs 76, bind2_lane 53, b4pins_lane 23), masked disassembly base → c1 34 changed, 3 new, 0 outside the advance list, clippy rc 0, narrow 21 of 21 (vita-post 57b14b37)
* it stopped in c2: the narrow gate moved 10 pins of `const_seed_mark.rs` (build/moved_pins.txt) where the plan's premise named one, c12; the parser writes a vector or enum typedef's range into the declaration's `dtype.range` (`hdl-parser/src/decls.rs:210-211`), and `ast::DataType` keeps no type name (`hdl-ast/src/lib.rs:1008-1014`)
* plan pass 3 (W/constsellsb/plan3/PLAN3.txt §1): the class "the decision reads a merged field; the premise was checked on the source text", three instances: K1 a typedef's range carried into the declaration, K2 a parser-folded `$bits` bound, K3 a lexer list of the pins that move
* plan pass 3's ruling (PLAN3.txt §5, §8): admit the carried range and continue c2 behind trip wire TW3-a
* plan pass 3's sweep (plan3/sweep/DIFF.txt): 22,898 sources on PRE and p0b2, SAME 22,867, DIFF 28 (`const_seed_mark.rs` 10, `frame_pins.rs` 1, the row's own `decl_range_pins.rs` 17), CAP 3, rc changes 0
* premise judge 2 FAIL (W/constsellsb/premise2/REPORT.txt): of 102 cells, 21 print PRE = iverilog = verilator where the prototype p0b2 differs, each at rc 0; rc changes 0 (recounted from b1, b1b, b2, b3 and b4's SUMMARY.txt)
* F1, 12 cells, the class's second break: a decimal bound in `dtype.range` is also a fold through the parser's constant table, at `type_param_literal_range` (`hdl-parser/src/pkg_type_param.rs:71`) and `respell_pkg_expr` (`hdl-parser/src/packed_md.rs:158`), which plan pass 3 had not censused
  * tp_w0_local, tp_w0fn_local, tp_w0_formal, tp_w0_return (b1), tp_tern_local, tp_cast_local, tp_hdrless_param, pkgtp_tern_local, pkgtwin_tern_local, pkgtwin_tern_formal, pkgtwin_tern_bare (b1b), k2_td_of_tp (b4)
* witness (F1, premise2/b1): tp_w0_local, in `module top #(parameter int W = 4);` `localparam logic [W-1:0] L = 16;` `localparam type T = logic [L+3:L];` and `function automatic logic [7:0] f(input int x); T t; t = '0; t[2] = 1'b1; t[3] = 1'b1; return t; endfunction`, `P = f(1)`: PRE `V P=00001100 L=0`, p0b2 `V P=00000000 L=0`
  * oracles: iverilog and verilator `V P=00001100 L=0`
* F2, 9 cells, a new class: the prototype's record lives until the name's next declaration, while PRE's value slot is reset by the block's own declaration, so a select write after the block lands at the inner declaration's positions (premise2/REPORT.txt F2, from the prototype's code)
  * package function bodies only: the module twins (b4 ltm_*) are E3009 on all three binaries, byte-equal
  * lt_inner_noread_after_bitw, lt_inner_whole_then_outer_rewrite, lt_formal_shadow, lt_formal_shadow_rewrite (b2), lt_if_branch_decl, lt_nested_inner_twice, lt4_inner_whole_outer_rewrite, lt4_formal_shadow_rewrite, lt4_return_shadow (b3)
* witness (F2, premise2/b2): lt_formal_shadow, in `package q`, `function automatic logic [7:0] f(input bit [3:0] a);` `begin bit [5:2] a; end` `a[1] = 1'b1;` `f = a;`, `P = q::f(4'd0)`: PRE `V P=00000010`, p0b2 `V P=00000000`
  * oracles: iverilog and verilator `V P=00000010`
* plan pass 4 (W/constsellsb/plan4/PLAN4.txt §1, cells plan4/m1): F1's input is wrong on PRE at module scope with no constant function, §2 typed-localparam-unconverted, filed before this row:
  * c1_tp_mod, `localparam logic [W-1:0] L = 16;` `localparam type T = logic [L+3:L];` `T x;` with bits 2 and 3 written: PRE `V x=0000 L=0 lo=16 hi=19`, iverilog and verilator `V x=1100 L=0 lo=0 hi=3`
  * c1_tp_width, the type `logic [L+3:0]`: PRE `V b=20 x=11111111111111111111 L=0`, iverilog and verilator `V b=4 x=1111 L=0`
  * c1_twin_mod, a package typedef `logic [M:L]` over an imported `localparam logic [(1 ? 3 : 2):0] L = 16` and M = 19: PRE `V x=0000 b=4 lo=16 hi=19`, iverilog and verilator `V x=00000000000000001100 b=20 lo=0 hi=19`
* so on F1's 12 cells PRE prints both oracles' line over a range it holds as `[19:16]` (c1_tp_mod `lo=16 hi=19`); plan pass 4 reads that as two errors cancelling (ER §2.3, REVIEW-ONLY), and a record taken from the elaborated range would read the same `[19:16]` (PLAN4.txt §1)
* ruling V2 (PLAN4.txt §3, §4): park both rows; V1 (a record only for a range written in the declaration's own text, and a one-declaration rule for F2) left the origins of 99 span variables NOT_MEASURED and needed c1 rebuilt and re-verified
* named costs as measured per site (premise/REPORT.txt Q2, Q3; p0L/SUMMARY.txt): instructions PRE → p0a2 (p0b2 for the select read and write), median of 6 runs (q3a, q3e: of 3)
  * W1, a local with a shifted range: z_decl_loop_off +8.685 %, +1265.8 per bind (800,000 binds)
  * W6, a formal: e4_shift +15.562 %, +2104.8 per bind (1,199,760 calls)
  * W9, a return: z_ret_off_loop +3.623 %, +2133.5 per call (800,000 calls)
  * e5_paren0, a `[(3):0]` formal: +6.891 %, +933.4 per call, +848.8 over e1 (1,199,760 calls)
  * a select of a record local in a concat: c0 +24.266 %, +8520.1 per read; c3 +17.750 %, +4858.2 per read (800,000 reads each)
  * a select write: z_sel_write_off +0.484 %, +120.4 per write (800,000 writes)
  * no record: y_env +0.688 %, +38.4 per `bind_shape` use (2,399,782 uses); q3a +0.482 %, +70.2 per W1 bind; q3e +0.401 %, +260.3 per call (800,000 each)
* re-entry (PLAN4.txt §3 PARK RECORD), re-derived from the row and the code of that day (ER §3.6 last row):
  * RE-1: §2 typed-localparam-unconverted converted at its producer (c1_tp_mod, c1_tp_width and c1_twin_mod print both oracles' lines), or the record declines on a fact the parser records
  * RE-2: one record per declaration, from the fold that gives the width, no text classifier; a function with no shifted declaration at most +1.00 % (y_env and its twins e0–e6, q3a, q3e); W1, W6, W9 and e5 re-measured per unit
  * RE-3: the record lives as long as the binding: premise judge 2's lt cells (b2, b3) and PLAN3's b1–b11 at 0 right → wrong
  * RE-4: a P0 over the acceptance set, premise judge 2's 21 cells each printing PRE's line or both oracles', with right → wrong 0, rc change 0 and kept refusals byte-equal, before any build
  * the conversion's row: typed-localparam-unconverted names §5.2 row 68 or row 48 (its line 3); which one converts the table is not determined (PLAN4.txt U4)
* kept (W/constsellsb): the page's cells (ground/cells.tsv) and plan/PLAN.txt's lane table; plan3/mx–mx5 (84 cells) and plan3/sweep; premise2/b1, b1b, b2, b3, b4 (102 cells, the 21 the acceptance set); plan4/m1, m2 (28 cells); the named-cost cells (p0L, premise/q3); build/moved_pins.txt
* kept: `decl_range_pins.rs` as expected values only (park/T_final.patch); c1.patch and c2_wip.patch are records, not bases (ER §3.6)
* note: Q-CONSTAGG parks with it (Q-CONSTAGG.md:10, the same frame record)
