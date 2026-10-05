# §4.5.594 start decision + implementation plan — §2 🆕 AI ("row X")
status: DONE (every number below is from a run in this session or quoted from GROUNDING.md with its cell name)

Binaries (frozen, never rebuilt):
- PRE    $S/s592/pre/vita     md5 86a2d84a21d0329c57541e51ae9eb4d2 (release, 75f46453)
- post3r $S/s594/post3r/vita  md5 ca572e0c4873c3903ceb48545e4d814b (release, 75f46453 + g/p123.patch = P1+P2+P3)
HEAD moved during this run: 1da347bc "docs: record §4.5.592 — 🆕 V reverted whole" (docs/ only). §4.5.592 has no code on main,
so there is no file or semantic overlap with p123. `git diff --stat 75f46453 1da347bc -- crates/elaborate crates/hdl-parser
crates/hdl-ast crates/ir` = `crates/hdl-parser/src/lib.rs | 13 +++++++------` (doc comment only) → PRE is elaborate-identical
to HEAD. crates/ overall differs by §4.5.590 (sim-engine t0 hold + 2 cli tests) → the slice's review PRE must be rebuilt from
1da347bc (`git archive`), and run-time-lane cells (Arpt_*, Cca_*, Mdl_*) re-run on it.
At 1da347bc 🆕 AI is ROADMAP:146 and §5.2 row 6 (🆕 W + 🆕 S (a) is row 7); rows 1–5 precede it.
Audit harness: $S/s594/a/{a1gen.py→cells (140), a2gen.py→cells2 (105), a3gen.py→cells3 (52)}, cells4 (10), cells5 (6),
ivd/ (11 iverilog probes); runners g/run4.sh, a/run4w.sh (same + 15 s perl alarm on vvp/verilator sim: iverilog hangs on
`repeat (x)`); summaries a/cells*.summ (tags pre, p3r, ivl, s2v, vl); classifier g/xt.py (hand-corrected where verilator's
2-state reading was the only "oracle": x/z is not a verilator axis, RULES.md).

## Intent
Make a constant region's width and sign knowable for two leaves the walk answers "unknown" today — a replication whose count
is a name (the value fold already reads it) and a whole element of a captured constant array parameter (its element type is
recorded) — so a signed neighbour stops folding sign-extended into an unsigned region, without letting any consumer that
newly gets an answer turn a loud or correct cell into a silent wrong.

## Decision: NARROWER — GO for P1+P2+P3 behind one region rule; the full p123 cut is NO-GO
Why not the full cut: over 313 new audit cells post3r has 13 descents (4 LOUD→WRONG, 9 OK→WRONG) and 6 WRONG→WRONG′, every
one in a region that holds a user call or names a constant-function variable — 🆕 AE's never-assigned 0 (and 🆕 AC's declined
call) read through a newly sized region. 🆕 AE's row already states the rule: "until it lands, a lane a row opens from loud
to a value declines a run that reads a never-assigned 4-state variable"; ER §2.2 "never widen a loud into a silent", §2.3
"breaking a cancelling pair" (Kfn_cpR, Mcmp_RAE_int: PRE right only because width and 🆕 AE's 0 cancelled).
The rule (structural, ER §2.4 "a routing arm excludes by structure"; ER §2.5 "decline per consumer for the delta only, as a
documented delta-limiter"): a REGION ROOT that contains a user `Call` anywhere (complete walk: `param_query::ast_any`, which
descends Concat/Replicate parts and count) or names an `envw` entry (constant-function formal / local / return variable) is
sized and signed by PRE's leaf rules — literal-only count (`const_eval_u32`), no whole-element width, element unsigned — so
its route is PRE's line verbatim. A comparison's two operands are ONE region (Mcmp_RAE_gi proves a per-operand rule leaks).
Effect, by construction: the 13 descents and 6 WRONG→WRONG′ take PRE's answer; kept: grounding 273/275 WRONG→OK (Bfa_lp,
Bfr_lp name a local), 20/20 LOUD→OK, 22 noref→verilator, the 8 WRONG→WRONG′ of residue V; audit 32/36 WRONG→OK; lost (stay
PRE, filed as R5): 11 audit LOUD→OK + 4 audit WRONG→OK + 2 grounding WRONG→OK.
Start condition (ER §10.2): every lane measured on post3r (grounding Q3 table + Audits 1–2 below); the narrowed build only
removes answers (its Literal rule IS PRE's code line), so it needs no new lane — but it must be re-measured on every set.

## Audit 1 — consumers that newly get an answer, varied into 🆕 AC / 🆕 AE
Consumers (a52e1a66 = HEAD lines). G = PRE's None was a guard (decline / other route); V = value only.
| # | consumer | on None (PRE) | cells | post3r result |
|---|---|---|---|---|
| 1 | `eval_const_env_at` comparison arm const_fn_width.rs:568 (+ cs :578) | V: w=0, unmasked | c1 lp lg gi rb; Kfn_cp*, Mcmp_*, Mfn_cpR, Bred_*_gi | WRONG→OK; with 🆕 AE OK→WRONG ×4, WRONG→WRONG′ ×2 |
| 2 | leaf arm :721 (+ ls :724) | V: raw leaf | c1 | WRONG→OK |
| 3 | `eval_const_env_self` :755 (+ sg :760) | V: ctx 0 | c2 Scond Sidx Spow Slnot | WRONG→OK, LOUD→OK (Spow/Sshc) |
| 4 | `eval_const_shift_count` :840 | gated by `ctx_width_names_are_evident` (no Replicate/BitSelect arm) | — | opted out |
| 5 | `eval_const_assign` :993 (+ cs :1000) | V: ctx 0 | c1 Bfa Bfr Bfn | WRONG→OK |
| 6 | reduction arm in a constant function :674 | G: `?` decline (E3009) | Bred_*, Kfn_rd* | LOUD→OK ×7; with 🆕 AE LOUD→WRONG ×2 |
| 7 | `const_unsigned_selfdet` const_fn.rs:675 (`$clog2`, CA/net delay, time-literal operands) | G: non-negative kept, negative declines (→ run-time lane / loud) | c2 Sclog; Cca_*, Cnet_*, Cct_*, Cpt_*, Kcl_*, Kdl_*, Kfn_cl*, Mdl_* | WRONG→OK ×4 (delay), LOUD→OK ×2; with 🆕 AE LOUD→WRONG ×2, OK→WRONG ×2 |
| 8 | tier-3 `ast_selfwidths_all_known` const_bound.rs:119 (→ `lower_const_width_expr` 🆕 AC sink, `repeat_unroll_count`) | G: tier-3 declines → `unwrap_or(0/1)` default / run-time counter | c2 C*; Arep/Apsw/Arng/Arpt_*, Kpw_*, Krp_* | WRONG→OK ×5; with 🆕 AE OK→WRONG ×2, WRONG→WRONG′ ×3 |
| 9 | `param_decl_width_opt` ternary/operator arms params.rs:632/:695 | width: value-inferred tail | c1 lv, Bpkg Binst Bovr, c3 K_ovr3, Klv_* | 60+1 WRONG→OK, 8 V, 1 🆕 AE WRONG→WRONG′ |
| 10 | `override_self_meta` param_query.rs:590/:599 | `?` | c2 Eovu_R/Eovt_R, Diu_R/Dit_R/Dii_R | opted out (PRE = post3r) |
| 11 | `const_ctx_within_i64(_context)` param_query.rs:811/:851 | only `Some(w>64)` refuses (router) | c1 Wz64/Wz80, c2 Dw80*/Dfw80* | E3009→right, no value→loud |
| 12 | `untyped_fill_init` param_query.rs:900/:903 | `unwrap_or(0).max(1)` | c1 Fil_*, c2 Dfw2u Dfe8u | WRONG→OK |
| 13 | `param_init_kept_loud` param_query.rs:978 | `is_none_or(w<32)` | c2 Ekl_*, Ekl_R_d/_o5/_o8 | Ekl_R_d WRONG→OK (`B=32`→`B=40`), rest = PRE |
| 14 | `override_bits` fill arm const_wide.rs:1695 | `?`, name-free trees only | — | opted out (unreachable) |
| 15 | `const_signed_env` override channel instance.rs:1514/:1668, iface_inst.rs:32/:121 (:1303 defparam gated by `sign_is_syntactically_evident`) | element unsigned | Audit 2 | 6 WRONG→OK, 0 descents |
| 16 | `const_expr_signed` params.rs:130/:768/:799, param_query.rs:903 | element unsigned | c3 M_*_lp, Ov_lp_*, c1 Sel_s_lv, c2 Fcanss Fints | WRONG→OK / noref→verilator |
The 20 LOUD→OK of the grounding (c1 Pweq_Eel_gi/lp/rb, Pweq_Rlp_gi/lp, Wz80_lv; c2 Cps_R, Dfw80u, Spow_E/R, Sshc_E/R; s593
Eu1/Eu2 lp gi rb, Eu4 lp gi) land on consumers 1, 3, 8, 9, 11; none holds a call or a constant-function variable.
Descents found (post3r vs PRE vs oracles, raw lines; all removed by the region rule):
| cell | region | PRE | post3r | oracles |
|---|---|---|---|---|
| Bred_aeR_lp | `fr = \|{N{r}}`, `logic [3:0] r` never assigned, `localparam L = fr(2)` | `E3009 … fr(…) has no constant-fold arm` | `L=0` | iverilog, sv2v, verilator `L=x` |
| Bred_aeR_rb | `logic [fr(2) + 2:0] v` | `E3009 … a function call that does not fold` | `vb=3` | verilator `%Error: … left side of bit range isn't a two-state constant`; iverilog `vb=1`; sv2v `vb=x` |
| Kcl_RAE | `localparam L = $clog2((X + {N{1'b0}}) + fl(2))` | `E3009 … $clog2(…) has no constant-fold arm` | `L=8` | 3 oracles `L=x` |
| Kfn_clR_lp | `fk = $clog2((X + {N{1'b0}}) + r)` in fk | `E3009` | `L=8` | iverilog `L=0`; sv2v, verilator `L=x` |
| Arpt_RAE / Arpt_EAE | `repeat ({N{1'b0}} + fl(2))` / `repeat ((A[1] - 8'd2) + fl(2))` | `k=0` | `k=2` | hand-IEEE §12.7.2 (x count → 0); iverilog hangs (Krp_rtx: run-time `repeat (r)` hangs too); verilator `k=2` (2-state) |
| Mdl_RAE | `assign #((X + {N{1'b0}}) + fl(2)) w = r;` r 0→1 at 10 | `W4030 … a call in a delayed continuous assign: S3b` then `w=1 t=10` | `w=1 t=264` | iverilog, sv2v `w=1 t=10` |
| Kdl_RAE | same delay, constant driver | `w=1 t=0` | `w=1 t=254` | hand-IEEE x delay → 0 (iverilog gives `t=0` for every call delay on a constant driver, ivd p1–p6) |
| Kfn_cpR_lp | `int fk = ((X + {N{r}}) == 8'hFC)` | `L=0` | `L=1` | iverilog `L=0` (int is 2-state, x→0); sv2v, verilator `L=X` |
| Kfn_cfR_lp | `int fk = ((X + {N{1'b0}}) + fl(a)) == 8'hFE` | `L=0` | `L=1` | iverilog `L=0` |
| Mcmp_RAE_int | `localparam int L = ((X + {N{1'b0}}) == (fl(0) + 8'd252))` | `L=0` | `L=1` | iverilog `L=0` |
| Mcmp_EAE_int | `((X \| A[1]) == (fl(0) + 8'd254))` | `L=0` | `L=1` | hand-IEEE (int); sv2v, verilator `L=X` |
| Mcmp_RAE_gi | `if ((X + {N{1'b0}}) == (fl(0) + 8'd252))` | `GI=else` | `GI=then` | iverilog, sv2v `GI=else` (verilator `then`) |
WRONG→WRONG′ (both silent): Arep_RAE `rep=0`→`rep=ff`, Kpw_RAE `pw=0`→`pw=2`, Kpw_EAE (3 oracles refuse the x count/width),
Klv_RAE `L=4294967294 B=32`→`L=254 B=8`, Mcmp_RAE_lg `L=0`→`L=1`, Mfn_cpR_lg `L=0`→`L=1` (3 oracles `x`).
🆕 AC unchanged (declined call `fcase` = const-fn-case): Arep_RAC `rep=0`, Apsw_RAC `pw=1`, Arng_RAC E3009, Arpt_RAC `k=3` —
PRE = post3r in all 12 AC cells. R1 (call count) unchanged: grounding Rfn_* 27 cells PRE = P3; Arep/Apsw/Arng/Arpt_R1c
(`{f2(2){1'b0}} + 2'd3`: `rep=0 pw=1 rb=1 k=3`, 3 oracles `fff 5 4 3`), Kdl_R1 `t=5` = iverilog — PRE = post3r.
Element type census (cells2, ER §4.5 "each `signed` spelling"): 16 WRONG→OK (byte, shortint, integer, int-typedef struct
packed signed, typedef signed vector, 1-bit bit, time: Tbyt_w_lp `L=0`→`L=1` sv2v+verilator; Ttim_s_lp `L=1`→`L=0`
verilator + hand-IEEE), 0 other movers; enum, real, string, multi-packed (E2002), >64-bit elements (W65/W128, not captured,
E3009 both) and the shadow cells Hfml/Hloc/Hgen/Hidx unchanged.
Totals post3r vs PRE over the 313 audit cells: WRONG→OK 36, LOUD→OK 11, LOUD→WRONG 4, OK→WRONG 9, WRONG→WRONG′ 6.
Under the region rule: descents 0, WRONG→WRONG′ 0; lost to R5: Bred_okR ×3, Bred_andR ×3, Bred_aeR_gi, Kcl_RG, Kfn_clG,
Kfn_rdG, Kfn_rdR (LOUD→OK), Kfn_cpG, Klv_RG, Kpw_RG, Mcmp_RG_int (WRONG→OK); grounding Bfa_lp `L=65532`, Bfr_lp `L=0`.

## Audit 2 — interface override sign; delay position (cells a/cells)
Interface override sign (iface_inst.rs:32/121) and its module twin (instance.rs:1514/1668): the `signed` channel is read
only by `override_at_declared_width` when the override has no `ovr_bits` and no fill (an operator tree), i.e. a >64-bit
declared target; ≤64-bit targets take the element's sign from `ovr_bits` (PRE already right).
| cells | PRE | post3r | oracles (raw) |
|---|---|---|---|
| Diw_se/Diw_st (interface, `parameter logic [127:0] P`, `#(.P(AS[0] + 8'sd0))` / `C ? AS[0] : AS[1]`), Dipw_se/st (positional) | `P=0000000000000000fffffffffffffffc` | `P=fffffffffffffffffffffffffffffffc` | verilator `P=fffffffffffffffffffffffffffffffc`; sv2v `P=000000000000000000000000000000fc` (drops the element sign, disqualified); iverilog `sorry: unpacked array parameters` |
| Dmw_se/Dmw_st (module twin) | same | same | same → 6 WRONG→OK, interface = module (ER §3.3) |
| Diw/Dmw `_s _u _sm`; Diu/Dit/Dii/Dipu/Dmu × {s, se, st} | = post3r | = PRE | verilator agrees |
| `_R` (`X + {N{1'b0}}`) and Diw_L/Dmw_L (`X + 2'b00`) onto every target | wrong | = PRE | 3 oracles `P=252 lt0=0 B=8` / `252` / `000…00fc` — pre-existing, not moved (ROADMAP:200; P-b below) |
Delay position (`const_unsigned_selfdet` → `delay_ticks_in_scope` / `delay_units_in_scope`):
| cells | PRE | post3r | oracles (raw) |
|---|---|---|---|
| Cca_R5 `assign #((X + {N{1'b0}}) / 8'd50) w = 1'b1;` | `w=1 t=0` | `w=1 t=5` | iverilog, sv2v `w=1 t=5` |
| Cnet_R5 `wire #(…same…) w = 1'b1;` | `t=0` | `t=5` | iverilog `t=5` (sv2v parse error) |
| Cca_E5 `(X \| A[1]) / 8'd50`, Cca_S5 `(AS[0] + 8'd0) / 8'd50` | `t=0` | `t=5` | sv2v `t=5`; iverilog `sorry` |
| Cca_L5/R2/E2/AS/X, Cnet_L5 | right | = PRE | iverilog/sv2v agree |
| Cct_* (CA) and Cpt_* (procedural) `… * 1ns` operands, 14 cells | — | = PRE in all 14 | — |
verilator prints `t=0` for every constant-driver CA delay incl. the literal twin Cca_L5: not a CA-delay oracle there; sv2v
prints `fired t=0` for every `* 1ns` cell incl. Cpt_L (iverilog, verilator `t=252`): not a time-literal oracle.
Result: 4 WRONG→OK, 0 descents for call-free delays; call-bearing delays are Audit 1's Mdl_RAE/Kdl_RAE (region rule).
param_init_kept_loud at ≥32 bits: Ekl_R_d `P=1 B=32` → `P=1 B=40` (sv2v, verilator `B=40`; iverilog `B=41`); o5/o8 = PRE.

## Audit 3 — the 8 WRONG→WRONG′ cells (PRE wrong in every one)
| cell | expression | PRE | post3 | ref (raw) |
|---|---|---|---|---|
| Adiv_lv | `localparam L = X / {N{1'b1}};` | `L=4294967295 B=32` | `L=255 B=8` | iverilog, sv2v, verilator `L=84 B=8` |
| Amod_lv | `X % {N{1'b1}}` | `L=4294967295 B=32` | `L=255 B=8` | 3 oracles `L=0 B=8` |
| Aediv_lv | `X / A[1]` | `L=4294967294 B=32` | `L=254 B=8` | sv2v, verilator `L=126 B=8` (iverilog `sorry`) |
| Wz34_lv | `X + {N{17'h0}}` | `L=4294967292 B=32` | `L=17179869180 B=34` | sv2v, verilator `L=252 B=34` (iverilog `B=35`) |
| Wz64_lv | `X + {N{32'h0}}` | `L=4294967292 B=32` | `L=18446744073709551612 B=64` | sv2v, verilator `L=252 B=64` (iverilog `B=65`) |
| K_ovr3 | `V = X + {N{3'b000}}`, `#(.N(3))` | `V=4294967292 B=32` | `V=508 B=9` | sv2v, verilator `V=252 B=9` (iverilog `B=10`) |
| M_divu_lp | `AS[0] / A[1]` | `L=4294967294 B=32` | `L=254 B=8` | sv2v, verilator `L=126 B=8` |
| M_shr_su_lp | `(AS[0] + 8'd0) >>> 1` | `L=4294967294 B=32` | `L=254 B=8` | sv2v, verilator `L=126 B=8` |
All untyped, module scope, no call → the region rule keeps them moved (width right, value still the unlimited
`const_eval_in_scope` coerced to it): the width-aware value lane admits a name only through `ctx_width_names_are_evident` /
`declared_override_widths`, which have no Replicate and no whole-element arm; literal twins right on PRE (Tardiv_lv `84 B=8`).
ROADMAP at 1da347bc: :200 is "An unreadable override leaf (element, replication, call, `$rtoi`, prim cast, `SZ_0'(B8)`) keeps
the default's type"; :199 is "A fill in an override tree keeps the default's type; region width via `declared_override_widths`".
The grounding's ":199" is one line off; residue V joins :200 (override twin Eovu_R / Diu_R `P=-4 lt0=1 B=32`, verilator
`P=252 lt0=0 B=8`) and names :199's `declared_override_widths`.

## Implementation plan (each step with its verification)
0. Freeze PRE = release build of `git archive 1da347bc`; record md5. Verify: c1/c2/c3/cells*/ivd outputs on it = $S/s592/pre
   outputs except run-time-lane cells, which are re-recorded.
1. const_array.rs: `const_elem_wsign(&self, e) -> Option<(u32, bool)>` = `const_array_elem_read(e)` → `(elem_w, elem_signed)`
   when `elem_w > 0` (no envw shadow check: the region rule covers a constant-function shadow, so a check there could not be
   killed — ER §7.4). Verify: cells2 element types (16 WRONG→OK), W65/W128 still E3009.
2. const_fn.rs + const_wide.rs: factor `const_placement_wide`'s resolver closure into one method and add
   `const_placement_count(count, envw) -> Option<u32>` = `fold_count` (made `pub(crate)`) through that resolver with
   `is_count = true` (locals refused, literal first, no call arm — §4.5.371 ⓸). Replaces p123's synthesized `{count{1'b0}}`.
   Verify: c1 count binders (lp lg lv gi rb ×9 spellings) = post3r.
3. const_fn_width.rs: `enum LeafRule { Literal, Scope }`; `fn leaf_rule(&self, e, envw)` = Literal iff
   `ast_any(e, Call)` || `ast_names_any(e, |n| envw.contains_key(n))` (doc: delta-limiter for 🆕 AE/🆕 AC, delete when 🆕 AE
   carries an x plane); `const_self_width(e, envw)` = `const_self_width_in(e, envw, leaf_rule(e, envw))` (decide lazily, only
   when a Scope-only arm would answer); Replicate: `const_eval_u32(count)` else, in Scope only, `const_placement_count`;
   select arm: `const_select_self_width(e)` else, in Scope only, `const_elem_wsign(e)` width; `const_signed_env(_in)` gets a
   Scope-only BitSelect arm → `elem_signed`; the comparison arm (:568–578) computes ONE rule over the comparison node and
   passes it to both operands' width and sign queries. Verify: Literal arms are PRE's lines (diff review), Mcmp_RAE_gi.
4. const_eval.rs: `const_expr_signed(_in)` the same Scope-only BitSelect arm (rule = call check; no envw at module scope).
   P2 and P3 ship together (grounding: P2 alone lands 8 cells on sv2v's sign-dropped value).
5. Convert the marker pin crates/cli/tests/selfdet_bound_lanes.rs:212–231 `width_unknown_wrap_bound_keeps_preslice_decline`
   (`CW=1`) to a value pin `CW=fffffffffff` (verilator and sv2v text; iverilog `sorry: unpacked array parameters`), renamed,
   doc rewritten. Fix every "no arm for a const-array element / name count" claim: const_bound.rs:72–78, const_fn.rs:~705,
   const_wide.rs:258 (`fold_count` "no twin to forget"), ROADMAP:100 WALL line, docs/REMAINING_WORK.md:19.
6. New test file (crates/cli/tests/, e.g. `const_width_count_and_element.rs`), value pins with raw oracle lines, one property
   per assertion: Ev12 (`L=252 M=1 K=1 vb=5`, 3 oracles), Eu7–Eu9 (`L=1 GI=then vb=5`, sv2v+verilator), count binders
   (localparam, header, untyped, pkg, genvar, instance override K_ovr), consumers (Apsw_R0 `pw=5`, Arep_R0 `rep=fff`,
   Kpw_R0 `pw=2`, Cps_R `ps=1`, Spow_R `L=4`, Cca_R5 `t=5`, Cnet_R5, Dfw2u `U=3 B=2`, Dfw80u, Ekl_R_d `B=40`), element
   types (Tbyt_w, Tsps_w, Tts6_w, Tbit1_w, Ttim_s), signs (M_div_lp `L=-2 B=8`, Ov_lp_se `L=-4 lt0=1 B=8`, M_cmp_ss,
   Dmw_se/Diw_se `ff…fc`), region-rule pins at PRE's answer with oracle text (Bred_aeR_lp E3009 / 3 oracles `L=x`;
   Arpt_RAE `k=0` hand-IEEE §12.7.2; Mdl_RAE `w=1 t=10` iverilog+sv2v; Kcl_RAE E3009 / `L=x`; Kfn_cpR `L=0` iverilog;
   Mcmp_RAE_gi `GI=else` iverilog+sv2v), residue pins with markers (Rfn_a_s_lp E3009, Arep_R1c `rep=0`, K_w65 `L=0`,
   Bfa_lp `L=65532`, Kpw_RG `pw=0`, Adiv_lv `L=255 B=8`). Cell sources: $S/s594/g/c1..c3, $S/s594/a/cells..cells5.
7. Mutants (expected outcome written first; `cargo nextest run --workspace --locked --no-fail-fast` each, ER §7.4):
   M1 Replicate Scope fallback off → Apsw_R0/Arep_R0/Ev12 die; M2 element width off → Tbyt_w/Eu7/Cca_E5 die;
   M3 `const_signed_env_in` element arm false → M_cmp_ss_lp / M_tern_ss_lt_lp (`L=1`, right on PRE by the raw compare,
   wrong once the width is known and the sign is not) and Dmw_se die; M4 `const_expr_signed_in` element arm false →
   M_div_lp/Ov_lp_se die; M5 element sign forced true → Ten4_s_lp (`(TA[0] + 1'sb0) < 0` over an enum
   `logic [3:0]` element 12: `L=0`, sv2v and verilator `L=0`) dies;
   M6 `leaf_rule` always Scope → Bred_aeR_lp/Arpt_RAE/Mdl_RAE/Kcl_RAE/Kfn_cpR/Mcmp_RAE_gi die; M7 ignores calls →
   Arpt_RAE/Mdl_RAE/Kcl_RAE/Mcmp_RAE_gi die; M8 ignores envw names → Bred_aeR_lp/Kfn_cpR die; M9 per-operand rule in the
   comparison arm → Mcmp_RAE_gi/Mcmp_RAE_int die. Budget ≈ 9 × (8 min relink + 30 s) plus first-run binary checks (D9).
8. Byte-identity: Literal = PRE's code for any root with a call or an envw name; Scope changes an answer only where PRE
   returned None (`const_eval_u32(count)` None; `const_select_self_width` None and `const_array_elem_read` Some; a whole
   element's sign). Verify: release corpus `.vu`/`.velab` 15/15 byte-identical (post3r was), with a firing trace (post3r: P1
   24× in verilog-axi, P2 0× — element arm vacuous on corpus, covered by cells), 666-variant harness 0 movers, full gate.
9. Re-measure with POST: c1/c2/c3/s593 old/cells*/ivd → expected = post3r except the 19 region-rule cells and the 17 R5
   cells, which must equal PRE; elab time A/B release, both orders interleaved (D5), `leaf_rule` lazily evaluated.
Risks: (a) a consumer that combines two `const_self_width`/`const_signed_env` answers into one context other than the
comparison arm would reopen M9's class — the soundness lens must census every such pair; (b) residue V (8) ships as
WRONG→WRONG′ (width right, value wrong) — if the reviewer rules that a trade, exclude `param_decl_width_opt`'s two arms and
lose 60 lv WRONG→OK; (c) const_fn_width.rs is already 1065 lines (CONTRIBUTING ~1000): keep new helpers in const_array.rs /
const_fn.rs, split under §5.2 row 11, not here; (d) no format bump, no frozen type (elaborate-local).

## Rows to file
- §2 🆕 AI → closed except its residues (one line each):
  - R1 call count: `{f2(2){1'b0}}` — `fold_count` has no call arm: lp/lv/gi E3009/E3010 (Rfn_a_s_lp; 3 oracles `L=1`), a bound
    `vb=1` and `{f2(2){1'b0}} + 2'd3` counts `rep=0 pw=1 rb=1` (🆕 AC sinks; 3 oracles `vb=5`, `fff 5 4`), the unselected
    arm `C ? X : {f2(2){1'b0}}` `L=0 GI=else vb=4` (3 oracles `1 then 5`); BLOCKED (§4.5.371 ⓸ call-depth, 🆕 AE, 🆕 AC).
  - R2 constant-function local with a call-bounded width (envw width 0): Bfl_lp `L=0`, 3 oracles `1`; OPEN.
  - R3 `S.len()` width: Oslen_o_s `L=0 GI=else vb=4`, iverilog + verilator `1 then 5`; OPEN.
  - R4 65-bit signed neighbour beside a name count: K_w65 `(X + {N{1'b0}}) > 65'd100` `L=0`, 3 oracles `1`; OPEN.
  - R5 (new) a region holding a user call or a constant-function variable keeps PRE's leaf widths: Bfa_lp `L=65532`,
    Bfr_lp `L=0`, Kpw_RG `pw=0`, Klv_RG `L=4294967294 B=32`, Mcmp_RG_int `L=0`, Kfn_cpG `L=0` (3 oracles 252/1/2/`254 B=8`/1/1),
    Bred_okR/andR, Kcl_RG, Kfn_clG/rdG E3009 (3 oracles fold); without the rule 13 descents (Audit 1); BLOCKED (🆕 AE x plane;
    🆕 AC for declined calls).
  - V value lane: an untyped initializer with a name-count replication or a whole element binds the unlimited fold at its
    (right) width — Adiv_lv `L=255 B=8` (3 oracles `84 B=8`), Wz34_lv, Aediv_lv, K_ovr3, M_divu_lp, M_shr_su_lp —
    `ctx_width_names_are_evident` / `declared_override_widths` have no Replicate / whole-element arm; joined with ROADMAP:200
    (override twin Diu_R/Eovu_R `P=-4 lt0=1 B=32`, verilator `P=252 lt0=0 B=8`) and :199; a 4-caller predicate → own slice.
- PROBE_CATALOG (PRE = post3r, outside the moved set):
  - P-a a signed operand of a real-valued delay reads unsigned: Cpt_X `#(X * 1ns)` and Cpt_AS fire at 252 where iverilog and
    verilator never fire (−4 ns); Cct_X CA twin (iverilog never) — `delay_plain_units` → `const_unsigned_selfdet`.
  - P-b an operator-tree override onto a TYPED parameter binds the unlimited fold: Kot_L `#(.P(X + 2'b00))` onto
    `parameter logic [15:0] P` `P=65532`, Koi_L onto `int P` `P=-4` (3 oracles `252`); Dmw_L/Diw_L onto `logic [127:0]`
    `…fffffffffffffffc` (3 oracles `…fc`).
  - oracle notes: iverilog 13 hangs on a run-time `repeat (x)` (Krp_rtx; hand-IEEE §12.7.2 → 0); iverilog fires a
    constant-driver CA delay holding a call at t=0 (ivd p1–p6; with a changing driver p11 delays 2, p12 254 correctly).

## Effect on 🆕 W's retry (§5.2 row 7)
- Prerequisite met once this lands: post4w (P1+P2+P3 + s593 W proto) took p11's 17 correct→wrong and p8's 12 to 0 and kept
  Tv12 right; W3/W6 (array container) moved WRONG→OK with P2 (s593 W3_psw/W6_psw).
- The region rule does not touch W's cells as far as measured (T-typed module-scope constants, no calls) — unmeasured on the
  narrowed build: the retry re-runs §4.5.593's sets (666, p3, p7, p8, p11) on the post-AI PRE, as row 7 already says.
- W still needs 🆕 S (a)'s i64 half: its remaining descents are the constant `==?` sinks (p3 W1/W2/W10 13, c8 4, c9 20,
  c11 3, c12 6 — grounding numbers).

## Could not determine
- Whether the reviewer accepts residue V's WRONG→WRONG′ (width right, value wrong) as not a "trade"; the plan ships it as the
  grounding proposed and names the exclusion that would avoid it.
- Why §5.2 row 6 is taken before rows 1–5 (not stated in the brief).
- `leaf_rule`'s elab-time cost on the corpus (lazy evaluation proposed; unmeasured).
