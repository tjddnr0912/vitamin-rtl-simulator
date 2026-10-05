# DELTA_R1 — review round 1 fixes (POST → POST2)

Binaries: PRE `pre/vita` 4ace7617440041d0d16aee5309fded10 · POST-T `postT/vita` 37d2b613f937979ade730b6808175e22 ·
POST `post/vita` 1e41978f78724cc11b691cf6babfe911 (= snapshot e350ef79) · **POST2 `post2/vita` ed69d5ae6fa11e12d4220550f6391b42** (release, 7304768 B).
Delta vs the e350ef79 snapshot: `r1_delta_src.patch` (12 hunks, 8 source files) and `r1_delta_tests.patch`.

## Changed hunks
| file | change |
|---|---|
| const_wide.rs | `wide_entry_is_stale(key)` = `wide_param_bits` has key AND (`params` or `str_param_raw` or `real_param_val`) has it; `wide_name_bits` returns None on such a key |
| ident_route.rs | `bare_ident_route` skips the `Wide` route on a stale key, so the narrow route that follows reads the current binding |
| lib.rs, driver.rs | new field `gen_case_region: BTreeMap<(prefix, span.lo, span.hi), bool>` |
| generate.rs | GenItem::Case: look up the key, call `gen_case_choose(.., use_region = cached.unwrap_or(true))`, insert the first answer |
| gen_case.rs | `use_region` parameter; returns `(body, region_ok)`; `region.filter(use_region)`; header rewritten (dF4) |
| const_eval.rs | range-bound refusal = `holds_xz_wildcard(e)` AND `fold_self_bits(e, wide_name_bits)` has an unknown bit; message "… makes this bound's value x …" |
| wildcard_eq.rs | `holds_xz_wildcard` doc: one caller (W1) |
| tests | generate_case_label_domain.rs +2 (A1S+A1D, B2E+B2D; d1w); inside_wildcard.rs +3 (Sxf+Sxg, x bound refused; MC4; st1) and the compound-bound pins changed from refused to PRE values |

## BLOCKING 1 — census
Writers of `wide_param_bits`, and what else the SAME binding writes at that key:
| writer | binding | also writes params/str/real/symbols at the key? |
|---|---|---|
| package.rs:388 | package-function prefix install of the package's wide consts (formals skipped) | no |
| package.rs:657 (restored at :998) | intra-package wide sibling during the package fold | no (string/real/narrow consts are other names) |
| package.rs:1277 | wildcard import of a wide const at the module key (removed at :1272 on a second package) | no (param_meta only) |
| package.rs:1386 | explicit import of a wide const | no (param_meta only) |
| wide_param_range.rs:56 `bind_wide_param` ← params.rs 2334/2369/2408, generate.rs 773/810, instance.rs 722 | wide declaration or override | no: each caller returns or continues before `bind_param_value` |

So no single binding puts a wide entry beside another value at one key. A key holds both only after a later narrow rebinding that does not clear the wide entry: a genvar (A1*, A2S, A3S; the setup suspends only `real_param_val`), a local enum label under a wildcard-imported wide constant (B2E), an explicit import (B3X). In each of these the narrow binding is the current one.

Cells (iverilog / sv2v / verilator texts from the lens_snd report; POST2 re-run):
| cell | PRE | POST | POST2 | oracles |
|---|---|---|---|---|
| A1S | zero 1 / one 2 | def / def | zero 1 / one 2 | zero/one (3) |
| A1L | def / a | def / def | def / a | def/a (3) |
| A2S | zero / one | def / def | zero / one | zero/one (iv, vl) |
| A3S | zero / one | def / def | zero / one | 3 oracles zero/one |
| B2E | a 1 | def 99 | a 1 | a (3) |
| B3X | def / a | def / def | def / a | def/a (iv, vl) |
| B4P | def / odd | def / def | def / odd | def/odd (iv, sv2v) |
| st1 (`me/`) | E3009×2 E3010×2 | Q=0 K=9 ×2 | E3009×2 E3010×2 (= PRE) | Q=0/1 K=0/1 |
| A1D (pre-existing twin) | i=18446744073709551625 K=9 | same | **i=0 K=0 / i=1 K=1** | 0/1 (3) |
| B2D (pre-existing twin) | E1=18446744073709551616 K=0 | same | **E1=1 K=1** | 1/1 (3) |
A1D and B2D move because `bare_ident_route` is the run-time twin of the same first-asking resolver. The producer row (clear the wide entry in the binders) is still open.

## BLOCKING 2 — phase order evidence
- Phase order per instance: instance.rs:1040 `GenPhase::Nets`, then :1167 VarInit, :1217 Logic, :1355 Instances. Those four are the only calls that elaborate a generate region, so the Nets phase writes the cache entry first.
- The key is `(cur_prefix, span.lo, span.hi)`. The prefix carries the instance path and every generate segment (`label[idx]`), and generate.rs's header guarantees the same sequence in every phase.
- Measured consistency: d1w, D1, D2, D4, D5, N01, NA08 (nested loops), NA12/NA12S (instance arrays) print each arm's own net value with no x. The 252-cell census is unchanged against POST.

| cell | PRE | POST | POST2 | oracles |
|---|---|---|---|---|
| D1 | def 99 | E3010 ×2 (mix) | def 99 | a 1 |
| D2 | def 99 | E3010 ×2 | def 99 | a 1 |
| D3 | def 99 | a 1 | a 1 | a 1 |
| D4 | def 99 | a 1 (hidden mix: same-width nets) | def 99 | a 1 |
| D5 | o=99 | o=1 (hidden mix) | o=99 | o=1 |
| d1w (`me/`) | def 9 bits=4 | a 8 bits=4 (mix) | def 9 bits=4 | a 200 bits=8 |
| d1p (`me/`) | k 8 bits=4 (mix) | same | same (residue) | sv2v k 200 bits=8; iverilog refuses |
| NA01F–NA04F | E3010 | E3010 | E3010 | iv def / sv2v,vl a |

D4 and D5 were right on POST only through a phase mix whose nets happened to agree; they go back to PRE's consistent answer.
Simpler, provably phase-stable alternative, not built: cache the chosen ARM per key. It also fixes d1p, but that moves a pre-existing silent-wrong (PRE's mix) to a consistent wrong `def` and was not asked for.

## BLOCKING 3 — bound cells
Rule: refuse iff the bound holds an x/z wildcard AND its own bit-domain fold has an unknown bit.
| cell | shape | PRE | POST | POST2 | oracles | class |
|---|---|---|---|---|---|---|
| Sxf | `((x==?)&1'b0)` | 1 | loud | 1 | 1 (3) | right restored |
| Sxg | `?0:0` | 1 | loud | 1 | 1 (3) | right restored |
| Sxk | inside `&1'b0` | 1 | loud | 1 | vl 1 | right restored |
| Sy2 / Sy3 / Sy5 | port / wire / function return | 1 | loud | 1 | 1 | right restored |
| Sxc | `?3:3` | 1 / x | loud | 1 / x | 4 / 4 | back to PRE (wrong, residue) |
| Sxj | `\|1'b1` | 1 | loud | 1 | 2 | back to PRE (wrong) |
| Sxe | dim `(..&1'b0)+1` | x | loud | x | 1 | back to PRE (wrong) |
| Sy1 | parameter-type range | 32 | loud | 32 | — | back to PRE |
| Sy6 | unpacked `[X:0]` | 8 | loud | 8 | — | back to PRE |
| G6 / MD1 / S05A,B / S07A,B / K02:15,16 / K02q | compound pattern | 4 / 1 / x,1 / x,1 / 1 | loud | = PRE | 2 | back to PRE (wrong, residue) |
| G5 G9 G11 G12 G13 / Q3 -DFNRX -DTDX | `1 + x`, `x*3` (arithmetic with x: the fold declines) | 1 (G13 32) | loud | = PRE | iv 1 / sv2v x | back to PRE (split) |
| S08A,B / S14A,B / AD_X / RB_X / K02:18 | x-valued compare as the bound | x / 1 | loud | loud | iv 1 / sv2v x | refused (split; LRM: an x bound is illegal) |
- Zero right→loud.
- The value-keyed rule cannot reach the `1 + x` shapes: the bit domain declines arithmetic on an unknown, so "x reaches the value" is indistinguishable from the masked `& 1'b0` without a 4-state arm. Those cells keep PRE.

## Moved cells vs POST (POST2 − POST)
- 252-cell generate-case census: 0.
- S2 matrix (174): 0.
- 666-variant harness: 14.
  - 4 change wording only: S08A/B, S14A/B, still refused.
  - 5 go back to PRE: S05A/B, S07A/B, K02q.
  - K02 has fewer refusal lines and is still loud on line 18.
  - Q3 -DFNRX, -DTDX (each also with -DIV) go loud → 1 bit, which is iverilog's reading.
- Lens probes (410): 32.
  - 30 equal PRE: A1L A1S A2S A3S B2E B3X B4P D1 D2 D4 D5 G5 G6 G9 G11 G12 G13 MD1 Sxc Sxe Sxf Sxg Sxj Sxk Sy1–Sy6.
  - 2 differ from PRE and now equal the oracles: A1D, B2D.
- velab, 11 corpus designs + 4 examples: 15/15 .velab and .vu identical PRE vs POST2.

## Mutant suggestions
1. `wide_entry_is_stale` → `false`: the A1S and B2E pins should die.
2. Remove only the `ident_route.rs` skip: the A1D/B2D lines in those pins should die.
3. Remove only the `wide_name_bits` decline: A1S should die, and A1D/B2D should stay.
4. `use_region` ignored (always true): the d1w pin should die.
5. Key without `cur_prefix` (span only): no pin is known to kill it; build a loop where iteration 0's label refers forward and iteration 1's does not.
6. Drop the value condition (node-keyed refusal): `a_bound_refusal_is_keyed_on_an_x_value` (Sxf) should die.
7. Drop the refusal entirely: the x-bound half of the same test should die.
8. `bp_any_unknown` → `!bp_any_unknown`: both halves should die.

## Residues (for the docs step)
- dF2: a parameter-width cast label `W6'(P8)` keeps its i64 answer (T30: PRE/POST/POST2 def, iverilog/sv2v/verilator hit), plus 17 random cases. Class: "a label no bit fold reads keeps PRE", like L07, S44, S48.
- sF3: an indexed part-select WIDTH with an x-valued `==?` is silent on every build (G2b `y=0001`; iverilog and sv2v refuse). Append to PROBE_CATALOG line 83, "an x-valued compare used as a size".
- dF3: `localparam T TP = '1` under `#(.T(logic signed [3:0]))` / `#(.T(byte))` reads 15 / 255 where all three oracles read -1. grep of ROADMAP, PROBE_CATALOG and REMAINING_WORK found no entry; ROADMAP:224–226 is the enum-base 2-state axis, a different shape. NOT recorded.
- d1p: a forward-referenced label that is itself the match mixes arms between phases (PRE already does). Pre-existing.
- B1 producer row: genvar setup, enum-label binding and explicit import should clear a same-key `wide_param_bits` entry (cells A1S A1L A2S A3S B2E B3X B4P st1; the twins A1D and B2D are closed by the resolver decline).
- Bound residues back to PRE: compound pattern (G6 MD1 S05 S07 K02:15/16), masked-x 4-state operators (Sxc Sxj Sxe Sy1 Sy6), `1 + x` arithmetic (G5 G9 G11 G12 G13, Q3 FNRX/TDX).
