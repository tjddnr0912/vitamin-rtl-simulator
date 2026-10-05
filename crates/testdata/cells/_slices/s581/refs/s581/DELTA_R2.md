# DELTA_R2 — S's i64 half alone (POST3b), for the round-3 lenses

Binaries: PRE `pre/vita` 4ace7617440041d0d16aee5309fded10 (= e54fa74a) · POST2 `post2/vita` ed69d5ae6fa11e12d4220550f6391b42 ·
POST3 `s581/post3/vita` 33067d7744033c67c45ad43be49d95da (superseded: has the blocker below) ·
**POST3b `s581/post3b/vita` 6aab206f39960ecfd78a576159ed811c** (release, 7304736 B) = POST3 + the width fallback. Not to be confused with `s580/post3` (eab81ef2).

## Tree vs e54fa74a (`r2_src.patch`, `r2_tests.patch`, new file `r2_new_generate_case_label_residue.rs`)
Byte-identical to HEAD (`git diff --quiet`): generate.rs, ident_route.rs, driver.rs, lib.rs, const_wide.rs, const_bound.rs, const_eval.rs, param_default_takes_declared_type.rs, string_literal_condition.rs. gen_case.rs and generate_case_label_domain.rs are deleted.
| file | change |
|---|---|
| wildcard_eq.rs | `const_wildcard_i64` (i64 `==?` / `!=?` / x/z `inside` element at w = max(L(lhs), L(pattern)) ≤ 64, the pair's sign, `pattern_ext_fill`); private `wildcard_match` (moved here from const_wide, its only caller); module and `pattern_ext_fill` docs say the wide domain has no `==?` arm |
| wildcard_eq.rs (POST3b) | **fallback**: at `w > 64` `const_wildcard_i64` returns `const_wildcard_masked_pre(op, lhs, rhs)`, HEAD's own-width masked compare reproduced line for line (WildEq/WildNe, sized x/z literal pattern, non-negative left value, single-word bit-63-clear pattern, else None). Keyed on the width decline only, so the ≤64-bit cells the routine fixed never reach it; the doc comment names the cells it serves (`localparam logic [67:0] P68 = 68'hC; P68 ==? 4'b1?00`) and the ones it must not reach (`(4'd15+4'd1) ==? 5'b1?000`, `S ==? 4'sb?100`, `inside`, MC4) |
| const_fn_width.rs | the width-aware comparison arm consults `const_wildcard_i64` after `const_compare_special` |
| const_str.rs | `const_compare_special` loses the own-width masked `==?` compare (string equality only) |
| const_fn.rs | comment |
| tests | inside_wildcard.rs converted to the observed values; const_domain_semantics.rs = eab81ef2's conversions (all pass); new generate_case_label_residue.rs (T1 T2 S06 at PRE, X13 item) |
`holds_xz_wildcard` and its const_eval call are removed. **The removal was measured, not assumed.** A build with the guard kept and no wide arm (`vita_guard_noarm`) still refuses S05A/B, S08A/B and K02 lines 14, 15, 18: HEAD's wide fold reads an `inside` element as `==`, which is x. So the guard was not inert; without it those cells equal PRE.

## Moved-cell tables (POST3 vs PRE)
- **252-cell generate-case census**: 2 moved, X13 and X23 (`inside` label, PRE `def` → `a`, = sv2v and iverilog's `==?` twin). Every other cell equals PRE: all 🆕 T cells, the stale-key cells (G01, NA06*), and the phase cells (NA0xF).
- **S2 matrix (174)**: unchanged 134 · loud→value 15 · value→value 25 · value→loud 0.
  - loud→value by consumer: localparam untyped LP_I, typed LT_I, package PK_I, size cast SC_I, $clog2 CL_I, enum value EN_I, generate-if GI_I, generate-case scrutinee GS_I, override OT_I and OV_I (OV_I: iverilog/verilator `1 1`, sv2v `1 32` split), generate-case label binders BL_I BS_I BP_I BO_I, signed region SR1. 14 match the oracles.
  - value→value: AD_I AD_W BL_W BO_W BP_W BS_W CL_W EN_W GC_I GC_W GI_W GS_W LP_W LT_W OT_W OV_W PK_W PS_I PS_W RB_I RB_W RP_I RP_W SC_W SR2. 23 match the oracles; OV_W (sv2v `$bits` 32) and SR2 (verilator 0, iverilog and sv2v 1) are splits.
  - Not moved (= PRE): every >64-bit / wide-arm cell (LW_*, BW_*, BG_*, BOW_*), GF_* (the generate-for condition goes through the wide domain), and AD_X / RB_X (no guard).
- **666 harness**: both 452 · NEW=PRE≠eab81ef2 85 (the wide half and the T lane not landed) · NEW=eab81ef2≠PRE 103 · neither 26.
  Oracle check of the 214 NEW≠PRE-or-eab81ef2 variants:
  - match the oracles 68 · same stdout as PRE 114
  - no oracle 14: the P06/P06v family, where verilator on the `-DNOGC` twin prints POST3's lines exactly; P7 L4q 0, recorded in s580 as verilator 0 and iverilog `==?` 0; Q3 -DREPX at file level.
  - NEW≠oracle 18:
    - prior-ruled oracle artifacts: CD1, P08b, Q5, Q9, P2c, Q3 PSEL ×2
    - residue lines equal to PRE's value: H03, K02, P4a ×2
    - Q3 -DFNRX / -DTDX ×4: an `x*3` bound line, iverilog 1 / sv2v x; the file was loud on PRE because of other lines
    - **the BLOCKER below**: lens_snd3 p3 -DG2 / -DG7
- **Lens probes (499 files: lens_snd/probes, lens_diff/p, lens_snd2 q1–q4, lens_diff2/p, me/)**: 71 move from PRE.
  - 70 have every moved line oracle-confirmed.
  - Si23 equals verilator and iverilog's `==?` twin; sv2v rewrites the unsized x pattern.
  - No 🆕 T cell, stale cell (A1S B2E X1 q1g …) or phase cell (d1w r1 r1b C1 d1p) moves.
- **velab**: 15/15 .velab and .vu identical PRE vs POST3.

## BLOCKER found on POST3, closed in POST3b by the fallback
A left operand whose declared width is over 64 bits but whose value fits, against an x/z pattern, moved away from PRE on POST3. The cause: PRE's own-width masked compare (in `const_compare_special`) answered it, `const_wildcard_i64` declined at w > 64, and the wide domain has no `==?` arm. POST3b falls back to that compare exactly there.
| cell | PRE | POST3 | POST3b | iverilog | sv2v |
|---|---|---|---|---|---|
| `case (1) (P68 ==? 4'b1?00)` (s580 lens_snd3 p3 -DG2) | item | default | item | item | item |
| `case (0) (P68 !=? 4'b1?00)` (p3 -DG7) | item | default | item | item | item |
| p3 -DLV | — | ≠PRE | = PRE | | |
| blk/b2.sv: localparam L, LN, generate-if, generate-case | then / item | E3009 ×2, E3010 | then / item | then / item | then / item |
| blk/b1.sv (with an `inside` localparam, loud on PRE) | E3009 ×1 | E3009 ×3, E3010 | E3009 ×1 | | |

Width ladder (`ladder/`, 18 cells): left operand declared 65 / 68 / 128, holding `'hC` (fits) or `'h1_0000_0000_0000_000C` (does not fit); `==?`, `!=?`, `inside {4'b1?00}`; each cell exercises a localparam, a range bound, a generate-if and a generate-case label. **POST3b = PRE on 18/18.**
- POST3 had regressed the fits-in-64 `==?` / `!=?` cells, 6 of them, to E3009/E3010.
- Where PRE is loud, POST3b is loud the same way: the `inside` fits cells (E3009; sv2v item) and the non-fit `==?` / `!=?` cells (E3009 ×2, E3010; iverilog/sv2v default and item). These are wide-half residues.
- Where PRE answers, POST3b equals iverilog and sv2v.

POST3b vs POST3, everything re-run:
- 252-cell census: 0 moved
- S2 matrix: 0 moved of 174
- lens probes (520 files incl. blk/ and ladder/): 9 moved, all back to PRE (blk pin/b1/b2, the 6 regressed ladder cells)
- 666 harness: 3 moved, p3 -DG2 / -DG7 / -DLV, all back to PRE
- velab: 15/15 .velab and .vu identical PRE vs POST3b
POST3's moved-cell tables above therefore hold for POST3b, minus the blocker rows. New pin: `inside_wildcard.rs::a_wide_declaration_with_a_fitting_value_keeps_the_pre_compare`, checked against iverilog, sv2v and PRE.

## Mutant suggestions
1. `const_wildcard_i64` returns None always: MC4 and the operator-and-sets pins should die.
2. Drop `.max(pw)` (left operand at its own width): Q1 / L4 (`(4'd15+4'd1) ==? 5'b1?000`) should die.
3. `sg = false` (sign never pushed): MC4 should die (`mut_c` analogue).
4. `pattern_ext_fill` → `Some(false)` always: the signed-pattern pins (Q4/I1) should die.
5. `w > 64` → `w > 32`: the 33..64-bit ladder cells should die.
6. Fallback removed (`if w > 64 { return None; }` as on POST3): `a_wide_declaration_with_a_fitting_value_keeps_the_pre_compare` should die (gen-case item → default, L/LN loud).
7. Fallback widened to every decline of `const_wildcard_i64`, or called before it: the ≤64-bit fixes regress to PRE's own-width answer — Q1 `(4'd15+4'd1) ==? 5'b1?000` back to 0 and MC4 — so the operator-and-sets and MC4 pins should die.
