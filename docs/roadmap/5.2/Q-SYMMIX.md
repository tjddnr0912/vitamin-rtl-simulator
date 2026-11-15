(§5.2 Q-SYMMIX · rank ① · source Q-DECLTYPE plan round 2 (W/qdecltype/plan2/PLAN-r2.txt R7 F12)) (a symbolic struct over a signed and an unsigned parameter is laid out wrong):
* slot: PARKED 2026-11-09 behind Q-VALRULES (b) → row 74, to re-enter with spelling (C), keeping C0 · L1 while parked (owner decision O3, W/qsymmix/replan3/PLAN.txt OWNER DECISIONS)
* slot: S1v (Q-PARAM2STATE's value half) runs next at C0 · L1 as batch 0 (owner, 2026-11-09)
* slot: the two lines above supersede the slot lines below (the re-entry at C0 · L1 of 2026-11-08, the order of 2026-11-05 and 2026-11-06)
* slot: it re-enters at R5.1 · L1 (batch 5), after batch 4 lands, on VAL-1–3, SYS-1 and PARSE-1, with a premise judge and a design review, in place of "Q-VALRULES (b) → row 74" (owner, 2026-11-09, the record-first order: (b) is VAL-1–3 and no longer waits on 74)
* slot: PARKED 2026-11-14 a third time, keeping R5.1 · L1: the batch 5 P0 of spelling `{W}` (owner option A, W/qsymmix5/plan/PLAN.txt Q2) tripped the wire named before it ran, nothing landed, and batch 5's next rows are the owner's (the third park below)
* slot: re-enters at C0 · L1, next, since Q-VALRULES fix (d) slice A landed (§4.5.636; owner's order 2026-11-06: fix (d), then this row's re-entry, then S1v, then batch 1)
* slot: after Q-DECLTYPE's S12 lands, before Q-PARAM2STATE's value half S1v, which it blocks (owner, 2026-11-05)
* slot: alone or in one batch with S1v, ahead of the first batch, the owner's call
* symptom: a symbolic struct (`hdl-parser/src/sym_struct.rs`) whose member MSBs name a signed and an unsigned narrow parameter is laid out wrong at exit 0
* witness: sy9_mixgeom `parameter NS = 4'shF;` `parameter L3 = 4'd15;` `typedef struct packed { logic [L3 - 5'sd14 : 0] a; logic [NS + 5'sd2 : 0] b; } s_t;` `s_t v;` `v = 4'b1001;` `$display("A bt=%0d ba=%0d bb=%0d va=%b vb=%b", $bits(v), $bits(v.a), $bits(v.b), v.a, v.b);`: PRE `A bt=20 ba=18 bb=2 va=000000000000000010 vb=01`, iverilog and verilator `A bt=4 ba=2 bb=2 va=10 vb=01`, sv2v → iverilog `A bt=20 ba=2 bb=2 va=00 vb=01` (20 bits from two 2-bit members: it contradicts itself)
* witness: sy7_noul the same `s_t` over the same NS and L3 with plain twins `logic [L3 - 5'sd14 : 0] na;` `logic [NS + 5'sd2 : 0] nb;` and `$display("A sa=%0d sb=%0d na=%0d nb=%0d", $bits(v.a), $bits(v.b), $bits(na), $bits(nb));`: PRE `A sa=18 sb=2 na=2 nb=2`, iverilog, sv2v → iverilog and verilator `A sa=2 sb=2 na=2 nb=2`
* witness: x1_ovr_sym_twin through an instance override, `module m #(parameter P = 4'd0, parameter Q = 4'd15) ();` `typedef struct packed { logic [Q - 5'sd14 : 0] a; logic [P + 5'sd2 : 0] b; } s_t;` `s_t v;` `$display("A bv=%0d ba=%0d bb=%0d p=%0d", $bits(v), $bits(v.a), $bits(v.b), P);` with `parameter NS2 = 4'shF;` `m #(.P(NS2)) u();`: PRE `A bv=20 ba=18 bb=2 p=-1`, iverilog and verilator `A bv=4 ba=2 bb=2 p=-1`, sv2v → iverilog `A bv=20 ba=2 bb=2 p=-1`
* note: right on PRE with one parameter name per struct (sy1–sy5) and with unsigned members only (sy9's `typedef struct packed { logic [L3 - 5'sd14 : 0] a; logic [5'sd1 : 0] b; } u_t;` prints `B bt=4 ba=2 va=10 ub=01` on PRE and all three tools)
* witness: b08_onename one parameter name, wrong: `parameter NS = 4'shF;` `typedef struct packed { logic [NS + 5'd0 : 0] a; logic [NS + 5'sd2 : 0] b; } s_t;` `s_t v;` `logic [NS + 5'sd2 : 0] nb;` `$display("A bv=%0d ba=%0d bb=%0d nb=%0d", $bits(v), $bits(v.a), $bits(v.b), $bits(nb));`: PRE `A bv=34 ba=32 bb=2 nb=2`, iverilog and verilator `A bv=18 ba=16 bb=2 nb=2`, sv2v → iverilog `A bv=34 ba=16 bb=2 nb=2` (W/qsymmix/probe/g3/b08_onename)
* note: so the one-name note above holds for sy1–sy5 only: the axis is the sign of each member width, not the name count (W/qsymmix/GROUND.txt G3)
* cause: measured (W/qsymmix/GROUND.txt G2): `register_sym_struct` composes the member widths into one tree, each width `(E < 0) ? 1 - E : E + 1` over the member's MSB `E` as written (`hdl-parser/src/sym_struct.rs:116` `sym_member_width`), each offset the sum of the widths below it (`:48`), the total the sum of all (`:62`).
* So an unsigned width makes the sum unsigned (IEEE 1800-2017 §11.8.1) and a signed narrow MSB is zero-extended inside it, where elaborate folds each bound alone (`elaborate/src/array_geom.rs:531`)
* cause: elaborate is right on the tree it is given: p7_usertree, the same tree written on a plain net (`logic [((L3 - 5'sd14) + 1) + ((NS + 5'sd2) + 1) - 1 : 0] n1;`), is `n1=20` on PRE, iverilog, sv2v and verilator (W/qsymmix/probe/g2/p7_usertree)
* cause: the wrong member is the one whose bounds sum across the sign boundary, not the unsigned one: p1_swaporder (the signed member first) PRE `A bt=20 bb=18 ba=2 vb=000000000000000010 va=01`, iverilog and verilator `A bt=4 bb=2 ba=2 vb=10 va=01` (W/qsymmix/probe/g2/p1_swaporder)
* scope: the same composition in md packed parameters (P2, `hdl-parser/src/packed_md.rs:122` `packed_dim_width`, `:129` `packed_stride`): m01_mdparam `parameter logic [NS + 5'sd2 : 0][L3 - 5'sd14 : 0] PM = 4'b1001;` PRE `A bp=36 p1=10 p0=01`, verilator `A bp=4 p1=10 p0=01`, iverilog `t.sv:4: sorry: packed array parameters are not supported yet.` (W/qsymmix/probe/g3/m01_mdparam)
* scope: and in typedef and type-parameter widths (P3, `hdl-parser/src/type_params.rs:628` `sym_range_width`, read by `$bits(T)` at `:740` and by `T$w` at `hdl-parser/src/type_param_packed.rs:93`): t02_tdbits_dim `typedef logic [NS + 5'sd2 : L3 - 5'sd14] t_t;` `$bits(t_t)` PRE `A bt=17`, iverilog and verilator `A bt=1` (W/qsymmix/probe/g3/t02_tdbits_dim)
* note: sites `hdl-parser/src/sym_struct.rs:14`, `:100-136`, `hdl-parser/src/struct_sel.rs:42-140`, `hdl-parser/src/lvalue.rs:460`, `hdl-parser/src/type_rec.rs:274`, `hdl-parser/src/decl_key.rs:143`
* note: under Q-PARAM2STATE's value-half rule (C) the cells t4_symbolic, x1_ovr_sym_kw and x2_ovr_sym_unrec descend without this fix (W/qdecltype/plan2/PLAN-r2.txt R4), so it goes first
* note: a first reader of the declared-kind record (`TypeKind`, §4.5.635) matches it exhaustively, with no `_` arm, and pins `NotRecorded` (Q-DECLTYPE S12 design review N-6)
* PARKED 2026-11-06 behind Q-VALRULES fix (d) after three premise rounds (premise judge r3 FAIL, BLOCKING F1 F2 F3, W/qsymmix/RULINGS-premise3.txt), nothing landed: `git diff 777e0f25 -- crates/` is empty
* note: three premise rounds measured three spellings of the member width in the parser: r1 `64'(e)` over a non-plain bound turned n4c_mdret_wrap32 from `A r=1001 bh=4` (iverilog and verilator the same) into `declared net width 4294967296 exceeds the v1 cap (1048576)`
* r2 `64'(($bits(e))'(e))` for every literal kind turned op1_uu (`logic [N - '1 : 0] a;`, PRE = iverilog = verilator `A bv=8 ba=5 bb=3`) into `a cast that is not a constant here is not allowed in a constant range bound`, mixed an exact piece into a PRE-spelled typedef product (mx7 `A b0=3 b1=-2147483642`, mx12, mx13) and copied a substituted width 9x (gl_11 killed at 2 GB where PRE peaks 370 MB)
* r3 F1z (plain-only, P1 alone) met three: F1, the exact total leaves P1 through a typedef alias, which copies no Sym slot (`hdl-parser/src/typedefs.rs:617-631`)
* F2, it admits a name whose declared range is ascending or not 0-based, which elaborate's `$bits` fold refuses
* F3, an exact struct's PRE twin is built from a 1-bit default (`unwrap_or_else(|| Self::dec_lit(1, m.span))`) (W/qsymmix/premise1, premise2, premise3 REPORT.txt)
* witness: F1 c01_alias_bits_lp `module top #(parameter N = 4);` `typedef struct packed { logic [N-1:0] a; logic [N-1:0] b; } s_t;` `typedef s_t a_t;` `localparam X = $bits(a_t) + 32'hFFFF_FFFF >> 1;` `$display("A x=%0d b=%0d", X, $bits(a_t));`: PRE `A x=3 b=8` = iverilog = sv2v → iverilog = verilator, F1z `A x=2147483651 b=8`
* witness: the same silent on c02_alias_bits_disp, c04_alias_ltp, c05_alias_override, c12_alias2_bits, c13_alias_tdvec, d01_gen_alias, d02_inst_alias, d04_alias_lp_member and d05_alias_unpdim (W/qsymmix/premise3/cells, cells2)
* witness: the same loud on c03_alias_bits_range, c07_alias_mem_nonexact and d03_alias_member_carry
* witness: F2 e01_asc_lp `module top #(parameter N = 4);` `localparam logic [0:2] LA = 3'd5;` `typedef struct packed { logic [LA + N : 0] a; logic [N-1:0] b; } s_t;` `s_t v;` `$display("A bv=%0d ba=%0d", $bits(v), $bits(v.a));`: PRE `A bv=14 ba=10` = iverilog = sv2v → iverilog = verilator, F1z `t.sv:3:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: a cast that is not a constant here is not allowed in a constant range bound [in top]` and four more errors
* witness: the same loud on e02_nz_lp, e03_asc_hdr, e13_ubrange_hdr, e14_pkg_asc, e15_td_asc_hdr and e17_lp_signed_asc (W/qsymmix/premise3/cells2, cells4)
* witness: F3 c20_twin_bogus `typedef s_t a_t;` (s_t as in c01) `typedef struct packed { logic [$bits(a_t) - 1 : 0] x; logic [N - 1 : 0] y; } m_t;` `typedef struct packed { m_t z; logic [N - '1 : 0] w; } o_t;` `o_t o;` `$display("A bo=%0d bz=%0d bw=%0d", $bits(o), $bits(o.z), $bits(o.w));`: PRE `A bo=18 bz=12 bw=6` = iverilog = sv2v → iverilog = verilator, F1z `A bo=11 bz=5 bw=6` (W/qsymmix/premise3/cells3)
* note: main's D7 re-measure (W/main_d7/2026-11-06/qsymmix, PRE d59629fa, F1z 1c616c6f) reads the same c01, e01 and c20 texts
* note: four instances on the bound-context axis across three plans (r1 n4c, F1v q3 in re-plan 1, r2 mx7 / mx12 / mx13, r3 F1 and F3): a parser respelling of a width rule cannot be made exact without the declared types elaborate holds, so the rule needs its one home first (ER §5.1)
* note: the plain-only spelling measured in re-plan 2 (W/qsymmix/plan3/PLAN3.txt, prototype F1z W/qsymmix/plan3/bin/vita-f1z md5 1c616c6f) is the evidence fix (d) starts from, not a patch to restore (ER §3.6: re-derive from the row and current code once (d) lands)
* re-entry: when Q-VALRULES fix (d) lands, re-derive the width rule from this row on the then-current code
* the owner's order (2026-11-06): fix (d) at C0 · L1, then this row's re-entry, then S1v, then batch 1
* first cells the premise3 cells above
* then the 46 of 427 cells PLAN3's prototype F1z (W/qsymmix/plan3/bin/vita-f1z md5 1c616c6fb5ea0d2168b25d88558c0cdc) moves (41 wrong -> right, 0 descents, 0 outputs moved over 5572 sweep rows)
* then sy9_mixgeom, sy7_noul, x1_ovr_sym_twin, c05_nested, c13_defparam, b07_neg, w03_selfwrap_total, then W/qsymmix/plan3/cells (s1_*, h1_*, q2_*) and W/qsymmix/plan3/chain
* re-entry: on 2026-11-08 fix (d) slice A gave the three parked spellings one home without moving a value: the member width `(E < 0) ? 1 - E : E + 1` is `range_rules.rs` `sym_member_width` (`crates/hdl-ast/src/range_rules.rs:857`, called from `hdl-parser/src/sym_struct.rs:135`)
* re-entry: after slice A the offsets and the total sums are `sym_sum_of` (`range_rules.rs:882`, called from `sym_struct.rs:212`), and the 1-bit width an errored member is laid out with is `sym_member_default_width` (`:875`, `sym_struct.rs:39`)
* re-entry: after slice A P3's `sym_range_width` is `sym_typedef_width` with its `[X-1:0] -> X` shortcut (`range_rules.rs:817`, `:819`, called from `hdl-parser/src/type_params.rs:634`) and P2's builders are the `md_*` functions (`range_rules.rs:729`-`:793`, called from `hdl-parser/src/packed_md.rs`)
* outside the file by its rule, so the re-entry meets them where they were: F1's alias slot (`hdl-parser/src/typedefs.rs`, a table: `range_rules.rs` OUTSIDE list) and F2's `$bits` fold in elaborate (an evaluator, the same list)
* the record's count carries no `integer` limit since R1c moved to S1v, so an exact struct total past 2^31-1 meets no limit on that path: the re-entry censuses such totals or keeps its lane's PRE cap (UNMEASURED, W/qvalrulesd/plan7/filings.txt N7-2)
* evidence: slice A leaves the row's cells as PRE: c01_alias_bits_lp `A x=3 b=8`, e01_asc_lp `A bv=14 ba=10`, c20_twin_bogus `A bo=18 bz=12 bw=6` byte-equal to PRE, and the 56 rows of W/qsymmix/premise3 in the cells of record show no difference (W/qvalrulesd/plan8/PLAN.txt D5, prototype p7c ae4ce705, the build r4 = p7c on all cells)
* PARKED 2026-11-09 after the re-entry's evidence round and premise rounds 4 and 5 each met a BLOCKING on the minted route (W/qsymmix/RULINGS-reentry-ev.txt, RULINGS-premise4.txt, RULINGS-premise5.txt, each D7-confirmed by main)
* nothing landed: `git diff cc4d0257 -- crates/` is empty
* note: this park supersedes the re-entry lines above (the re-entry when fix (d) lands, the owner's order of 2026-11-06), and the re-entry bullets of 2026-11-08 stay as the sites the next re-entry starts from
* re-entry route: the minted one, where the parser mints each symbolic member's bounds as a localparam pair that elaborate's body binder binds per instance (`s_t$1$0$sr` / `s_t$1$0$sw` on V1u, `sym 2 0 width` on TK5)
* re-entry route: V1u (9f6e58ac, W/qsymmix/replan/PLAN.txt), then V1u-M3+DR (24be002e, re-plan 1, W/qsymmix/replan2/PLAN.txt), then TK5 (50c180b9, re-plan 2, W/qsymmix/replan3/PLAN.txt), PRE vita-main 8c970e9b (= cc4d0257)
* route x scope (evidence round B-1): h15h_unit_hdr, a `$unit` typedef after `endmodule` naming module m's L3 under a top header `parameter L3 = 4'd7`
  * PRE rc 1 (7 x E3009, a false cause)
  * V1u rc 0 `A bv=28 ba=26 ...` (loud → value) (W/qsymmix/replan/ev/cells2/h15h_unit_hdr)
  * oracles: iverilog `` Unable to bind parameter `L3' in `$unit' ``, verilator `Can't find definition of variable: 'L3'`
  * IEEE 1800-2017 §23.9 (ieee1800-2017.txt:33529-33535) makes it an error
* note: the minted pair binds in the instantiating module's scope, after top's header parameters, not where the typedef is: the caller-scope class (F25, W/qvalrulesd/RULINGS-premise5.txt B1), a root class seen before (ER §3.6)
* note: PRE has the same scope leak (h15t, h15h read top's L3) and is loud by accident
* witness (2026-11-14, W/qsymmix5/ground/var/v2_h15h_nomix): the `$unit` `typedef struct packed { logic [L3 - 5'd1 : 0] a; logic [1:0] b; } s_t;` after module m's `endmodule`, no sign mix, read by `s_t v;` in `module top #(parameter L3 = 4'd7);`: PRE 4d1d84bb rc 0 `A bv=9 ba=7`
  * oracles: iverilog ``t.sv:4: error: Unable to bind parameter `L3' in `$unit'``, verilator ``%Error: t.sv:4:32: Can't find definition of variable: 'L3'`` (W/qsymmix5/ground/run_var/SUMMARY.txt)
  * P0 (below) prints it as PRE; filed at §2 typedef-dims-reresolve
* route x scope (premise round 4 B-1, the second BLOCKING on this axis): a typedef in a blockless `generate ... endgenerate` region is minted
* `parse_gen_items_until` (`hdl-parser/src/generate.rs:54`) raises no `gen_depth` (only `:358` block and `:395` branch do), so the pair drains at `generate.rs:58-62` and the generate binder, which has no DR, binds it
* witness: a1_genregion_out PRE rc 0 `A bv=20 ba=18 bb=2`
  * oracles: iverilog = verilator `A bv=4 ba=2 bb=2`
  * V1u-M3+DR rc 1 `` t.sv:3:5: error[VITA-E3009] ... undefined name `s_t$1$0$sw` is not allowed in a constant range bound [in top.u] `` (wrong → loud, a false cause, a minted name printed)
* witness: a12_genregion_twoinst PRE `A top.u1 bv=20 ..` `A top.u2 bv=5 ba=1 bb=4`
  * oracles: iverilog = verilator u1 `bv=4 ba=2 bb=2`, u2 `bv=5 ba=1 bb=4`
  * V1u-M3+DR rc 1, the same E3009 (u2 right → loud)
* witness: a4_genregion_unused_str (an unused struct, a string override) PRE rc 0 `A N=97` = iverilog = sv2v → iverilog = verilator
  * V1u-M3+DR rc 1 `` generate-scope parameter `s_t$1$0$sr` value is not a constant: undefined name `N` ... `` and `` ... `s_t$1$0$sw` value is not a constant: undefined name `s_t$1$0$sr` ... `` (an added refusal) (W/qsymmix/premise4/cells)
* note: the anonymous member-type route (`hdl-parser/src/structs/mod.rs:144`) was outside the same census (F1b: a10 moves right, a13 prints `o_t$4$0$sw`)
* note: the root both times: the mint was decided by a predicate over parser state (`in_package`, `gen_depth`, `mint_container`) the walk does not guarantee, and the items went down the shared channel `pending_module_items`, which four loops drain (W/qsymmix/replan3/PLAN.txt P1 K1)
* binder arm (evidence round B-3): the minted pair does not reach `bind_one_param` (`elaborate/src/params.rs:1780`) as the re-entry plan's P1 said
* its route is `elaborate/src/instance.rs:706-792` and `generate.rs:836-890` (`instance.rs:703` `targets` is false for a minted name)
* binder arm (premise round 4 B-2): the body binder's string arm (`elaborate/src/instance.rs:717`) runs before the DR point (`:766`) and takes `$sr` over a bare string name, so the refusal prints the minted name
* census: the arms `check_param_decl_range` `:712`, string `:717`, real `:719`, wide `:762-765`, fold / DR `:766`
* witness: c7_r_bare_str (`logic [N : 0] a`, `m #(.N("a")) u();`) PRE `` undefined name `N` is not allowed in a constant range bound ``
  * V1u-M3+DR `` t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: undefined name `s_t$1$0$sr` is not a constant is not allowed in a constant range bound [in top.u] `` (rc 1 on both)
  * oracles: verilator `A bv=100 ba=98`, sv2v → iverilog `A bv=116 ba=98` (W/qsymmix/premise4/cells/c7_r_bare_str)
* note: the wide shape (c8, a bare 65-bit N) loses PRE's true cause `` `N` is wider than 64 bits, so it has no integral constant value for a width / range bound ... ``
* name space (premise round 4 B-3): `$` is legal in a simple identifier
* witness: b5_plain_collide_after (`localparam s_t$1$0$sw = 7;` after the typedef) PRE rc 0 `A bv=20 ba=18 u=7`
  * oracles: iverilog = verilator `u=7`
  * V1u-M3+DR rc 1 `` duplicate declaration of parameter `s_t$1$0$sw`: this `localparam` repeats ... `` (an added refusal on a PRE rc 0 cell)
* note: escaped names do not collide (b1, b2), and PRE has the class itself on its own generated names (b8_pre_carrier_collide, §2 generated-name-collision)
* evaluation phase (premise round 5 B-A, I17): a body constant function enters `const_func_table` (`elaborate/src/instance.rs:537-542`) before `bind_params` (`:656`)
* so a header default, a body parameter or an enum label calling it sizes the struct while the minted names are unbound, where PRE's composed tree reads a header parameter already bound
* witness: j4c_hdr_cfunc_local (header `parameter P = f()`, f's local `s_t x; x = '1; return x;`) PRE `A P=511 bv=9`
  * oracles: iverilog = sv2v → iverilog = verilator `A P=511 bv=9`
  * TK5 `A P=1 bv=9` at rc 0 (right → wrong, silent) (W/qsymmix/premise5/cells)
* witness: j4j_body_lp_cfunc_local_before_td (`localparam P = f();` before the typedef, the same f): the same three texts
* witness: j4k `P=511` / `P=8191` → `P=1` / `P=1`, j4m right → loud, j4p right → loud with a false cause (`folds to two different constants`)
* note: main's D7 (W/main_d7/2026-11-09/qsymmix-p5) reads the same j4c and j4j texts
* note: the route trace saw no read there (j4c: no DECLSRC, DECLMISS or SINK event), so "0 hazard reads over 627" was measured by a probe that cannot see it
* note: under it fires PRE's own silent default for a constant-function local whose width does not fold (§2 const-fn-local-unbound-width, j4r)
* census: 3 caller kinds (a header default, a body parameter before the typedef, an enum label before the typedef) x 3 sizing sites (`const_fn.rs:1645` formal, `:1455-1471` local, `:1709` return)
* name-keyed record (premise round 5 B-C, I18): TK5's run-start source record `sym_mint_decl` is keyed by the bare name, and REG (`next_type_reg`) counts per Parser, so a `velab -L` compose of separately compiled units repeats a name
* witness: j5b_mcu_class_diffspan (a.sv module m reads N, b.sv module k reads M) PRE `` undefined name `M` is not allowed in a constant range bound `` and the same for `N`, errors=2
  * TK5 `` undefined name `sym 2 0 width` is not allowed in a constant range bound `` twice, errors=2 (a minted name printed)
* note: round 4's D1 ruling had named it: the refusal's record per declaration, never name-keyed (W/qsymmix/RULINGS-premise4.txt D1)
* note: round 5 also met B-B (the funnel keyed on message text rewrote the user's `-G` text, j3c) and B-D (added cascade lines with a false cause on rc 1 cells, j3e, j3f, j5d), each seen before
* trip wire: named before round 5 (W/qsymmix/replan3/PLAN.txt VERDICT): a minted item drained off the body loop's own typedef item, a minted name printed by any diagnostic, or a pass reading a minted name between the header-parameter binding and the body binder where PRE folds (I17)
* trip wire: in round 5 or the build it ends the minted route, the fallback the owner's O3
* trip wire: tripped on two of its three parts, I17 by B-A (j4c, j4j, j4k, j4m, j4p) and the printed minted name by B-C (j5b), so the minted route ends (W/qsymmix/RULINGS-premise5.txt)
* re-entry: spelling (C), a per-member count node folded where PRE folds, with no minted names
* so the name space (K8), the name reach and phase (K9, I17), the text funnel (I7, I9) and the source record (I18) vanish, and the refusal text is PRE's by construction (W/qsymmix/replan3/PLAN.txt R5)
* re-entry: (C) fixes the sign composition only, not caller scope (q1a, q1j stay PRE: the node folds in the consumer's scope), and it is not prototyped (U4)
* re-entry: (C) waits on Q-VALRULES (b) → row 74: as a node today it costs one arm in each of the 11 elaborate evaluators and `sim-ir/src/names.rs:96`, after (b) one arm in its one evaluator (W/qsymmix/replan2/PLAN.txt P3)
* note: closing I17 by construction inside the minted route (a minted name read unbound folds its source in place) needs the same arm in every evaluator, the same cost as (C) (W/qsymmix/replan3/PLAN.txt R5)
* evidence: the 627-cell set, W/qsymmix/replan2/all.lst (536 unique) + W/qsymmix/replan3/p4.lst (54, premise round 4's cells) + new.lst, new2.lst, new3.lst, new4.lst (38, n01-n44)
* evidence: compared by `W/qsymmix/replan3/cmp3.py` (38b504aa), premise round 5's cells beside them (W/qsymmix/premise5/cells, j1-j6)
* evidence: on the 627 TK5 measured SAME 529, w→r 62, l→r 2, k→v? 4, l→v? 3, v?→v? 14, l→l 13
* 0 r→w, 0 r→l, 0 v?→loud, 0 added errors, 0 new kills, 0 WALL>2x (W/qsymmix/replan3/cmp3_tk5.tsv f55ec769, PLAN.txt P3)
* note: the 627 claims were measured with the minted route's probes, and round 5's j4 cells moved PRE-right designs wrong outside them, so (C) re-measures every claim
* evidence: the prototypes are evidence, not patches to restore (ER §3.6)
  * TK5 `W/qsymmix/replan/x/bin/vita-tk5` (50c180b9, its diff `W/qsymmix/replan3/proto/tk5.diff` fb1ac8a8)
  * V1u-M3+DR `W/qsymmix/replan/x/bin/vita-v1um3dr` (24be002e)
  * V1u-T (20e6f743, 534 of 536 SAME, two claims)
* evidence: Q6 (static, W/qsymmix/replan2/q6/q6.txt), 31 unique OpenTitan structs: 14 pass PRE's gate, 13 of them at module-item level, tlul_sram_byte.sv:150 in a block, none run (U3)
* evidence: the 13 are tlul_adapter_sram.sv:231 and :236, otbn_loop_controller.sv:52, prim_arbiter_tree_dup.sv:52, prim_sram_arbiter.sv:46, kmac_msgfifo.sv:97, rv_core_ibex_addr_trans.sv:22, prim_generic_flash_bank.sv:109
* and i3c_buffer.sv:65, i3c_target_trx.sv:313, i3c_controller_trx.sv:216, ibex_lockstep.sv:243 and :357
* re-entry: the first cells for (C) are the minted route's counterexamples (h15h_unit_hdr, a1_genregion_out, a12_genregion_twoinst, a4_genregion_unused_str, c7_r_bare_str, b5_plain_collide_after, j4c, j4j, j4k, j4m, j4p, j5b)
* then the 627 and Q6's 13 structs
* note: S1v runs first now, so the cells needing both rows (t4_symbolic, x1_ovr_sym_kw, x2_ovr_sym_unrec) stay wrong and are claimed on S1v's page (Q-PARAM2STATE)
* note: (C) on the homes (W/program/reorder2-2026-11-09/PLAN.txt G3): the count node's arm once (VAL-1), folded through the one entry (VAL-2), its bounds through the router (VAL-3); the node's facts one SYS-1 entry if spelled as a reserved `$__vita_` name
* note: and the parser emits the node where `range_rules` part 5's respell builders write today (PARSE-1): one arm in place of 11 + `names.rs:96`; (C) is not prototyped, and the 627 claims were measured on the minted route (U8)
* note: VAL-2 landed without the one entry (§4.5.639): (C)'s count node is the first rule written for cores of different types, so (C) builds it, `fold_in(lane, e) -> Folded` in `value/lanes.rs`, called by arms only; P0-b measured a runtime-match entry +6.25 % instructions (FAIL) and a generic `fold<L: FoldLane>` −0.045..+0.046 % (PASS) (§5.2 VAL-2)
* PARKED 2026-11-14 after the batch 5 P0 of spelling `{W}`, the one-operand concat around each member width, tripped the wire named before it ran (W/qsymmix5/plan/PLAN.txt Q3 TRIP WIRE), so the row parks with no second P0 and no patch round
* nothing landed: P0 was a scratch diff on f0d87432, never committed (W/qsymmix5/p0/p0.patch md5 0e08b65e, 3 files +11 −1; binaries P0 f0c53487, PRE 4d1d84bb)
* P0's three sites: `sym_member_width` returns `{W}` around PRE's ternary width (`crates/hdl-ast/src/range_rules.rs:859`)
  * `names_an_overridable` gains `ExprKind::Concat { parts } if parts.len() == 1 => self.names_an_overridable(&parts[0]),` before its `_ => None,` (`hdl-parser/src/sym_struct.rs:205`, K1)
  * `const_fold_descends` gains `| K::Concat { .. }` after `| K::Cast { .. }` (`elaborate/src/const_bound.rs:302`, K2)
* trip wire, named before the run: on the cells of record any right → wrong, right → loud, loud → value where both oracles refuse, or any diagnostic line that differs outside the claimed cells ends the parser-respelling axis (PLAN.txt Q3, :145-147)
* run: W/qsymmix5/p0/SUMMARY.txt (run_p0.sh 80087e52), 819 cell rows and the 255 moved sweep rows, 941 unique sources: `TRIP right_to_wrong=8 right_to_loud=15 loud_to_value=0 stray_diag=90`
  * stray_diag counts moved sources whose class is not wrong → right or loud → right and whose diagnostic lines differ (W/qsymmix5/p0/tools/cls_p0.py:30-31)
* premises broken, each a SUMMARY.txt `PREMISE` line: P11 (the TRIP line), P12 (c20 right → loud), P14 (g8a, g8d other), P16 (b07, h10 other; 11 P0 sources print ``the concatenation `{…}` has no constant-fold arm``), P20 (2 sources without `struct` moved `.vu` bytes), P21 (g5 +122.830 %)
* premises held: P4 (h15h and v3 loud on P0 with other lines, v2 the same), P6, P13, P15, P17, P18, P19, P22
* right → wrong, 8 cells (W/qsymmix/plan3/chain), each PRE = iverilog = verilator at rc 0, P0 `A b=1` at rc 0: g3_ssmix_4 `A b=16`, g3_ssmix_6 `A b=22`, g3_ssmix_8 `A b=28`, g3_ssmix_10 `A b=34`, g3_ssfill_4 `A b=20`, g3_ssfill_6 `A b=28`, g3_ssfill_8 `A b=36`, g3_ssfill_10 `A b=44`
* witness: g3_ssfill_4 `module top #(parameter N = 2, parameter L3 = 4'd15);` `typedef struct packed { logic [N - 1 : 0] a; logic [L3 - 5'd14 : 0] b; } s0;`, then s1 to s4 each `typedef struct packed { s<k-1> x; logic [N - '1 : 0] y; } s<k>;`, `s4 v;` `$display("A b=%0d", $bits(v));`
* right → loud, 15 cells, PRE = iverilog = verilator at rc 0 on each: 9 at rc 1, whose first line is ``error[VITA-E3009] E-ELAB-UNSUPPORTED: $bits argument shape unsupported (nets, array views, and self-determined expressions fold; v7)``, and 6 killed
  * c07_alias_mem_nonexact and c09_alias_typed_nonexact `A bo=14 bx=8 by=6`, c20_twin_bogus `A bo=18 bz=12 bw=6`, e14_pkg_asc `A bv=14 ba=10` (W/qsymmix/premise3/cells, cells3, cells4), op1_uu `A bv=8 ba=5 bb=3` (W/qsymmix/premise2/cells2)
  * q2_ss_exact2 `A bo=6 bp=10 bw=6 bz=4 x=1111 w=111111`, q2_ss_fill `A bo=5 bx=4 bz=1 x=1111 z=0`, q2_ss_fillcarry `A bo=9 bx=4 bz=4 by=1 x=1111 z=0000 y=1`, q2_ss_twin3 `A bp=10 bw=8 bu=2 w=11111111` (W/qsymmix/plan3/cells)
  * each of the 9 also prints 1 to 5 lines ``the width of this part-select does not fold to a constant: the concatenation `{…}` has no constant-fold arm`` (P0's out.out)
  * killed: g3_ss_7 `A b=18`, g3_ss_8 `A b=20`, g3_ss_9 `A b=22`, g3_ss_10 `A b=24`, g3_td_11 `A b=5` (W/qsymmix/plan3/chain), g4_alias_6 `A b=512` (W/qsymmix/premise3/cells3)
  * P0 hit the 30 s cap on each and, run alone, the 120 s / 8 GB cap, where PRE ran 0.13 to 4.62 s (W/qsymmix5/p0/out/capedge); g3_td_10 printed PRE's `A b=5` in 72.13 s where PRE took 1.59 s
* sources: 8 of the 8 right → wrong and 8 of the 9 rc 1 right → loud hold an unbased unsized `'1` (grep `'1`; e14_pkg_asc does not), and the 6 killed hold 8 to 13 `typedef` lines; what P0 does on them is REVIEW-ONLY (not measured)
* moved the right way: wrong → right 91 and loud → right 7 of the 941 (SUMMARY.txt `CLASS_N`); the rest: same 730, text-only 73, other 14, apart (one oracle) 3
* the 7 loud → right are P12's user forms p12a, p12b, p12c, p12d, p12g, p12i and p12j (W/qsymmix5/p0/cells/new): a user-written `{N}` member width (p12a `logic [{N} - 1 : 0] a;`), PRE rc 1 E2002 at parse, P0 = iverilog = verilator
* witness (P14, W/qsymmix5/p0/cells/new): g8a `module top #(parameter L7 = 4'd7);` `typedef struct packed { logic [L7 - 5'sd14 : 0] a; logic [3 : 0] b; } s_t;` `typedef s_t a_t;` `localparam [63:0] X = $bits(a_t) + 64'd0;` `localparam Y = $bits(a_t);`: PRE `A x=18446744073709551614 y=4294967294` at rc 0, P0 `A x=4294967294 y=4294967294`
  * oracles: iverilog = verilator `A x=30 y=30`
* witness: g8d, g8a's struct with `localparam [63:0] X = ($bits(a_t) + 64'd0) >> 1;`: PRE `A x=9223372036854775807` at rc 0, P0 `A x=2147483647`
  * oracles: iverilog = verilator `A x=15`
* witness (loud, P4: `{W}` keeps PRE's wrap): w01_selfwrap, a member and a net `[L3 - 5'd16 : 0]` over `parameter L3 = 4'd15;`, and w02_selfwrap_mix: PRE = P0 rc 1 ``part-select bounds [msb:lsb] ascend but the net is descending [hi:lo] (out of order)`` (W/qsymmix/probe/g3)
  * oracles: iverilog = verilator `A bv=36 ba=32 na=32` (w01), `A bv=33 bb=2 ba=31 na=31` (w02) (W/qsymmix5/ground/run/SUMMARY.txt)
* split (no IEEE reading given; replan3 U5 records q1d's): b07_neg and h10_neg_mixed PRE rc 1, P0 rc 0 with iverilog's line; k11_unary_mul_clog PRE rc 1 (13 errors), P0 rc 1 (3 errors)
  * oracles: b07_neg iverilog `A bv=6 ba=2 bb=4 nb=4`, verilator `A bv=32 ba=2 bb=30 nb=30`; h10_neg_mixed iverilog `A bv=6 ba=2 bb=4`, verilator `A bv=32 ba=2 bb=30`
  * oracles: k11_unary_mul_clog iverilog `A top.u2 bv=245 a=1 b=2 d=4 e=3 f=6 g=229`, verilator `A top.u2 bv=259 a=1 b=16 d=4 e=3 f=6 g=229` (W/qsymmix5/ground/run/SUMMARY.txt)
* cost: g5_cost_inst400_mem8 (400 instances × 8 members × 3 reads, W/qsymmix5/p0/cells/new), median of 6 runs, PRE 82,871,277,133 → P0 184,662,319,559 instructions (+122.830 %), wall 4.495 → 10.025 s (SUMMARY.txt `COST_P0`)
  * control k_net_ctl (cost30's w__k_net, no struct) 100,723,373,953 → 100,630,531,510 (−0.092 %), wall 5.650 → 5.615 s
* real designs: 51 of 51 the same (`REAL_SUM designs=51 same=51 diff=0`)
  * of Q6's 13 structs, 5 sit in files a real66 design lists (tlul_adapter_sram.sv twice, otbn_loop_controller.sv, prim_sram_arbiter.sv, kmac_msgfifo.sv) and 8 in files none lists (W/qsymmix5/p0/NOTES.txt step 6); whether the 5 build a symbolic layout there is not measured
* sweep: 255 of 26,858 rows moved (`SWEEP_CMP`): 192 in `.vu` bytes only, each source holding `struct`; 61 text-only (rc and display lines equal, a diagnostic line moved), 54 of them without `struct`; 2 other, without `struct`
  * witness (text-only): s589__g__c7__ty_cat_ctl.sv PRE ``parameter `P` value is not a constant: the concatenation `{…}` has no constant-fold arm [in top]``, P0 ``parameter `P` value is not a constant: `q.h(…)` has no constant-fold arm [in top]``, rc 1 both
  * witness (other): s593__a__p7__xCAT_ad.sv `logic [3:0] arr [0:({fx(2), 4'b0000} inside {8'b0?00_0000})+2];` (fx returns an unset local) PRE rc 0 `asz=4`, P0 rc 1 ``a function call that does not fold to a constant is not allowed in a constant range bound ...``; xCAT_rb the same as a packed range
  * oracles: iverilog ``t.sv:4: sorry: "inside" expressions not supported yet.``, verilator ``%Error: t.sv:4:62: right side of bit range isn't a two-state constant (IEEE 1800-2023 6.9.1)`` (xCAT_ad; xCAT_rb `t.sv:4:50: left side`)
  * which of the three edits moves the 56 rows without `struct` is REVIEW-ONLY: one build holds all three (PLAN.txt Q1 (c) PR-B asked for K2 as its own diagnostic row if the list edit moved a cell beyond c8's class)
* re-entry (2026-11-14): this replaces the (C) lines above (:111-114, :131-133), which do not hold on f0d87432: an E-alone node turns h15h / v3 loud → value, and the cause walker names a SysCall or prints nothing for an unknown kind (W/qsymmix5/ground/REPORT.txt G1 V-1, V-2)
* re-entry: the entry `fold_in` is not built (VAL-2.md:26, :29), and a node meets 5 core dispatch sites, at least 4 metadata sites, 1 to 3 cause walkers and 1 parser reader with or without it (ground REPORT.txt G2)
* re-entry: Q-SYMMIX re-enters only on a spelling whose P0 run shows, on the cells of record, 0 right → wrong, 0 right → loud, 0 loud → value where both oracles refuse and 0 changed diagnostic line outside the claimed cells, with K1's opened forms listed against both oracles (PLAN.txt Q1)
* re-entry: an E-alone spelling also waits on the `$unit` leak's own row (§2 typedef-dims-reresolve → row 68) and on a measured count of its arms (PLAN.txt Q1 (c) PR-C)
* re-entry: not on a spelling the parser emits for the member width: one was measured per park and each tripped, and this park's wire ends that axis (PLAN.txt Q3)
  * park 1 (2026-11-06, cfc17636): `64'(e)`, `64'(($bits(e))'(e))` and F1z (W/qsymmix/premise1, premise2, premise3)
  * park 2 (2026-11-09, bb99f548): the minted localparam pair, V1u → V1u-M3+DR → TK5 (W/qsymmix/replan, replan2, replan3)
  * park 3 (2026-11-14): `{W}` (W/qsymmix5/p0)
* re-entry: what the grounding names as needing code outside the struct lane: K1 (the parser reader `names_an_overridable`), K2 (the cause list `const_fold_descends`), K3 (a 64-bit consumer through the alias `$bits`, g8a and g8d above) (ground REPORT.txt G5)
* re-entry: its cells of record are PLAN.txt Q3's CELLS OF RECORD with this park's 23 trip cells and the 56 sweep rows without `struct` (W/qsymmix5/p0/out/CLASS.tsv)
* waits on it (pointers; the order is the owner's, batch 5's next rows are not decided):
  * Q-PARAM2STATE S1v-V: "runs at R5.2 · L1 after Q-SYMMIX (C) (R5.1)" (Q-PARAM2STATE.md:8), "S1v may land only lanes no symbolic struct, md packed parameter or symbolic typedef / type parameter reaches" (:36)
  * R6.1: "so BIND-1 and S1v-V land before R6.1 reads it" (BIND-1.md:19)
  * R7.4 Q-OVRXPLANE: "R7.4 · L1, after S1v-V (R5.2), on BIND-1" (Q-OVRXPLANE.md:3)
  * row 29's page half (R8.2), its r3 signing: "after S1v-V, Q-OVRXPLANE and the `const_eval_u32` literal lane's retirement (R5.2, R7.4, R7.5)" (29.md:6), "no binder reads it until S1v-V" (29.md:54)
  * EH1's row 31 (R8.3): "after row 24 and 29's page half re-enter (R8.1, R8.2)" (31.md:4)
* status: silent-wrong, pre-existing (PRE 92805863)
* source: W/qdecltype/plan2/cells/sy1_param_based … sy9_mixgeom, x1_ovr_sym_twin, x1_ovr_sym_kw, x2_ovr_sym_unrec, t4_symbolic
