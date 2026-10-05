# §4.5.594 grounding — §2 🆕 AI (planner's "row X"): a signed constant beside a node `const_self_width` cannot size
status: DONE
PRE = $S/s592/pre/vita (release, md5 86a2d84a21d0329c57541e51ae9eb4d2, main 75f46453). main has since moved to a52e1a66
(§4.5.590 sim-engine + docs; the row is filed there as §2 🆕 AI, §5.2 row 7 — 🆕 X was taken by the class-method `case` row);
75f46453..a52e1a66 touches none of const_fn_width.rs / const_fn.rs / const_eval.rs / selfdet_bound_lanes.rs.
ref = iverilog = sv2v (both run), else ≥2 agreeing running oracles; "s2v=vl" = iverilog refuses (`sorry: unpacked array
parameters are not supported yet` on every element cell). sv2v is disqualified on a SIGNED array element (drops the sign:
Sel_b_rv `$display("%0d", AS[0])` over `logic signed [7:0] AS[0:1] = '{-4, 2}` sv2v `RV=252`, verilator and vita `-4`;
also drops element widths in bounds: Cebn_E sv2v `eb=0`), so element cells where sv2v and verilator split are verilator + hand-IEEE.

## Artifacts
cells g/c1 (733, gen1.py), g/c2 (225, gen2.py), g/c3 (143, gen3.py), g/c4 (twins), g/old (s593's 1658 cells, copied), g/pin;
summaries g/c1.summ g/c2.summ g/c3.summ (tags p0 = PRE, p1 p2 p3 = prototypes, ivl s2v vl); classifiers g/xt.py g/cls.py;
harness g/run4.sh g/runall.sh g/runv.py g/h594.py g/corpus_velab.py g/fire.py; patches g/p1.patch g/p12.patch g/p123.patch.
Prototype binaries (worktree $S/s594/gwt @75f46453, removed at the end):
- post1 = P1 (debug md5 dacc7564ecc78ac928fbf9e333392537): Replicate count fold.
- post2 = P1+P2 (debug 486ea9c8753d10ba904873b618814856): + whole-element width (select arm) and sign (`const_signed_env`).
- post3 = P1+P2+P3 (debug 147bbb984b327e4e83aa5a8346c73470; release post3r ca572e0c4873c3903ceb48545e4d814b): + the same
  element sign in `const_expr_signed`. post3t = post3 + `ROWX_TRACE` firing counters. post4w = post3 + s593 g/proto1.patch (W)
  (debug bd67d6ad0eebff88baea13024fdba5a2).

## Q1 code census (read at 75f46453) — `const_self_width` None arms and what consumes a None
`const_fn_width.rs:163 const_self_width(e, envw)` returns None for:
- N1 `K::Replicate` with a count `const_eval_u32` (literal / paren / unary only, const_fn.rs:190) cannot fold — any NAME count
  (localparam, header param, untyped, genvar, pkg const), a call, `$bits`/`$clog2`, `N-0`; also count×width u32 overflow.
  The VALUE twin folds that count (`const_placement_wide` → `fold_self_bits` → `const_wide.rs:269 fold_count`: literal first,
  then the placement resolver, locals refused, NO call arm by design — §4.5.371 blocker ⓸). `fold_count`'s doc says "there is no
  twin to forget here"; `const_self_width` is that forgotten width twin.
- N2 selects via `const_select_self_width` (const_select.rs:721 → `const_select_resolved`): a WHOLE element `A[i]` of a constant
  array parameter (module, `p::A[i]`, imported), a multi-packed element's select, an untyped value-inferred base, an
  out-of-range index. A select OF an element (`A[i][3:0]`, `A[i][7]`) resolves (`const_select_base` element arm).
- N3 `K::Ident` with an `envw` entry of width 0 (constant-function local whose declared width does not fold, e.g. a bound
  that calls a function) — function bodies only.
- N4 `K::Ident` multi-segment (hier `u.P`) → catch-all.
- N5 `K::Cast` Prim with no `cast_prim_wsign` (real kinds); `Named` cast whose name is not a constant (`cast_size_bits`).
- N6 `K::Call` with no `const_fn_def` / `const_fn_ret_wsign_in` (two-segment `S.len()`; unfoldable return range).
- N7 catch-all: RealLit, StrLit, TimeLit, MethodCall, MinTypMax, AssignPattern, Null, Dollar, … (not i64-folded or not integral).
- Propagating: Paren, unary `+ - ~`, context-determined binaries, shifts/`**` (left), Ternary arms, Concat/Replicate parts,
  Signing casts. Never None: PkgScoped (32 fallback), SysCall (32, every name; the i64 lane folds only 32-bit-int ones),
  compares/reductions/`!` (1), IntLit fill (0).
`const_signed_env` (:321) and `const_expr_signed` (const_eval.rs:491): Replicate/Concat/selects → `_ => false` (right per
§11.4.12/§11.5.1); a WHOLE element `A[i]` is also `false` — wrong for a signed array (§7.4: the element has the element type).
Consumers of a None (every caller of `const_self_width`):
| site | on None |
|---|---|
| `eval_const_env_at` comparison arm (:568) | `w = 0` → both operands at ctx 0, no masking, a signed leaf stays sign-extended (the row) |
| `eval_const_env_at` leaf arm (:721) | leaf not reinterpreted (raw i64) |
| `eval_const_env_self` (:755) | ctx 0 — ternary cond, select index, shift count, `**` exponent, `!` |
| `eval_const_assign` (:993) | ctx 0, only the final coercion (const-fn assignment, `override_self_value`, width-aware init lane) |
| `eval_const_shift_count` (:840) | value unchanged |
| reduction arm in a const fn (:674) | `?` decline (loud) |
| `const_unsigned_selfdet` (const_fn.rs:675; `$clog2`, delay) | non-negative kept, negative declines |
| `param_decl_width_opt` ternary / operator arms (params.rs:632/:695) | untyped param falls to the value-inferred tail (32+) |
| `override_self_meta` (param_query.rs:590) | `?` decline (reached only past `ctx_width_names_are_evident`, which has no Replicate arm) |
| `const_ctx_within_i64(_context)` (param_query.rs:811/:851) | not a hazard (only `Some(w>64)` refuses) |
| `untyped_fill_init` (param_query.rs:900) | `unwrap_or(0).max(1)` → 1-bit width |
| `param_init_kept_loud` (param_query.rs:978) | `is_none_or(w<32)` → kept loud (overridden lane) |
| `ast_selfwidths_all_known` (const_bound.rs:119; tier-3 bound/count fold) | tier-3 declines → consumer default |
| `override_bits` fill arm (const_wide.rs:1695) | `?` (name-free trees only; unreachable for a name count) |
Not consumers (unaffected by any fix here): generate-case scrutinee and labels (`generate.rs:438` `const_eval_in_scope`, the
unlimited walk, two i64 values compared — ROADMAP §2 "Generate `case` compares two i64 values"); typed/untyped initializer VALUE
when `param_init_at_declared_width` declines (`eval_param_init` → `const_eval_in_scope`, then coerced).

### Q1 measured per None kind (signed neighbour `logic signed [7:0] X = -4` in an unsigned region; PRE vs 3 oracles)
| kind | spellings | PRE | live? |
|---|---|---|---|
| N1 name count | `{N{}}` localparam int / header param / untyped / `(N-0)` / nested / in concat / `p::PN` / `$clog2(4)` / `$bits(2'b00)` / genvar (Bgv) / gen-block, pkg, instance, override, const-fn binders | lp lg lv gi rb wrong in every one (e.g. Rlp_a_s_lp `L=0`, Rlp_a_s_lv `L=4294967292 B=32`, rb `vb=4`; 3 oracles `1`, `252 B=8`, `5`); literal count Rlit right | LIVE |
| N1 call count | `{f2(2){1'b0}}` (Rfn) | value position E3009/E3010 (fold_count has no call arm; oracles `1`); bound `vb=1` silent (🆕 AC sink); unselected ternary arm `C ? X : {f2(2){1'b0}}` `L=0 GI=else vb=4` (3 oracles `1 then 5`) | LIVE, not fixed (residue R1) |
| N1 + fill / wrap | `'1 ^ {N{1'b0}}` lv `U=1 B=1` (3 oracles `3 B=2`); `8'hFF+8'd1+{N{1'b0}}` lp `0` lg `1` (oracles `1`/`0`) | LIVE (no sign: overflow) |
| N2 element | `A[1]`, `p::PA[1]`, imported `PA[1]`, 1-bit / int / int-unsigned / 4-bit / desc / `[8:1]` / 33 / 64-bit elements, header array default + override, signed element as the signed operand | lp lg lv gi rb wrong (Eel_o_s_lp `L=0`, lv `4294967294 B=32`; sv2v + verilator `1`, `254 B=8`); selects of an element (Eesl/Eepa) right | LIVE |
| N2 multi-packed | `logic [1:0][3:0] AM [0:1]` | E2002 PRE (§3 ⑤ⓐ); sv2v + verilator run | loud |
| N3 | `logic [h()-1:0] z` in a const fn (Bfl) | `L=0`, 3 oracles `1`; literal-width twin (c4/flc) right | LIVE, not fixed (residue R2) |
| N4 | `X \| u.P` | E3009; iverilog, sv2v, verilator HIERPARAM refuse | not live |
| N5 | `u8_t'(8'd2)` (parser desugars to `unsigned'(W'(e))`, §3 ⑤ⓓ) | right | not live |
| N6 | `fz()` typedef return, `p::pf()` | right (except typed `lt`: the wide walk's missing call arm, 🆕 AE family) | not live |
| N6 | `S.len()` (Oslen) | `(X\|8'd0)+S.len() == 32'd254` `L=0 GI=else vb=4`; iverilog + verilator `1 then 5` (sv2v refuses); `+ 32'sd2` twin right | LIVE, not fixed (residue R3) |
| N2 untyped base | `W[1:0]` over `localparam W = 5+3` | right | not live |
| (>64) | `logic signed [64:0] X`: `(X + {N{1'b0}}) > 65'd100` | `0`, 3 oracles `1`; literal twins `X+65'd0`, `X+2'b00` right; P3 unchanged | LIVE, not fixed (residue R4) |

## Q2 live cells — PRE → P3 by consumer (c1, signed neighbour, 9 count spellings + 2 element spellings × forms `+`/`|`/`?:` = 22 cells per consumer)
| consumer | PRE | P3 |
|---|---|---|
| lp `localparam L = (E) == K` | 22 WRONG | 22 OK |
| lg `(E) > 8'd100` | 22 WRONG | 22 OK |
| lv untyped `localparam L = E` (value, `$bits`) | 22 WRONG (`B=32`) | 22 OK |
| lt `localparam logic [15:0] L = E` | 22 OK (declared lane) | 22 OK |
| gi generate-if | 22 WRONG | 22 OK |
| rb range bound `[(E)==K + 3:0]` | 22 WRONG (`vb=4`) | 22 OK |
| gc generate-case `case (E) K:` | 22 WRONG (`def`) | 22 WRONG — not this row: literal twins Tl2_a_gc/Tl8_o_gc/Tl8_t_gc wrong on PRE (3 oracles `item`) |
| rt run-time `(E) == K` | 22 OK | 22 OK |
| rv run-time `%0d` of E | 11 OK, 8 split, 3 noref | same — `?:` form: iverilog/sv2v `-4`, verilator/vita `252` (IEEE: unsigned result); iverilog folds the same text to 252 as a constant |
Run-time path: right on PRE for every live leaf. Operators (Rlp, Eel × lp gi rb rt): `!=`, `<`, `===`, `inside` 3 WRONG→OK each
(rt OK=); `==?` E3009/E3010 → OK (Pweq_Rlp lp gi: 3 oracles; Pweq_Eel lp gi rb: sv2v + verilator). Widths: neighbour w32/w33
4–5 →OK, w64 2 →OK (the i64 is the width), w65 1 →OK (lg stays: R4); replication z32/z34/z64 3 →OK, z80 lv E3009 →
`252 B=80` (verilator; iverilog `B=81`, its untyped-param self-contradiction). Binders: gen-block 4, genvar 4, pkg, instance,
override, const-fn (Bfn, Bfr, Bfa) all WRONG→OK (3 oracles). Arithmetic in an untyped value: see Q4 residue V.

## Q3 lane table (ER §10.2) for P1+P2+P3
| shared function / table | lanes reached | status |
|---|---|---|
| `const_self_width` Replicate arm, count `const_eval_u32` cannot fold (P1; literal counts keep PRE's line verbatim) | (a) comparison arm → lp lg gi rb, `==?`/`inside`, compare-topped gen-case | measured c1 (P*, 11 leaves), p10, p12 |
| | (b) leaf reinterpretation | measured c1 |
| | (c) `eval_const_env_self`: cond, index, shift count, `**`, `!` | measured c2 Scond Sidx Sshc Spow Slnot Sred Scast (R, E, literal twin) |
| | (d) `eval_const_assign`: const-fn assignment | measured c1 Bfa Bfr Bfn |
| | (e) `const_unsigned_selfdet`: `$clog2`, delay | `$clog2` measured (c2 Sclog); delay cells refused by all three oracles (CONTASSINIT / procedural) — vita PRE = P3 |
| | (f) `param_decl_width_opt` operator/ternary arms (untyped width) | measured c1 lv, Bpkg Binst Bovr, c3 K_ovr3: 60 WRONG→OK, 8 WRONG→WRONG′ (Q4 V) |
| | (g) `override_self_meta` | opted out by construction (`ctx_width_names_are_evident` declines a Replicate first); measured c2 Eovu_R/Eovt_R PRE = P3 |
| | (h) `const_ctx_within_i64(_context)` fences | measured c2 Dfw80t Dfw80u Dw80v Dw80c Dw80t Dw80r Dw80s, c1 Wz64 Wz80 |
| | (i) `untyped_fill_init` | measured c1 Fil_lv, c2 Dfw2u Dfw2uL Dfw80u |
| | (j) `param_init_kept_loud` | measured c2 Ekl_L Ekl_R Ekl_Rd Ekl_Ld (PRE = P3) |
| | (k) tier-3 `ast_selfwidths_all_known` | measured c2 Cbnd Crep Cps Cdim Crpt Cgfr (_R, _L, _E, _Ee), Cebn |
| | (l) `override_bits` fill arm | opted out (name-free trees only) |
| `const_self_width` select arm, whole element (P2; `const_select_self_width` untouched, so `ast_has_param_select` / `const_range_bound_fold_at` routing unchanged) | (a)–(k) | measured c1 Eel Epel, c2 F* C*_E C*_Ee S*_E, c3 M_*, g/pin |
| `const_signed_env` whole-element arm (P2) | comparison cs, leaf ls, self sg, assign cs, `**` exponent sign, `override_self_meta` sign (:599), override channel `ResolvedOverride.signed` (instance.rs:1303/1514/1668), params.rs:128 | measured c3 Ov_nu/pu/nt/ns/df × 6 (PRE = P3 = verilator), Fpexp, M_*; iface_inst.rs:32/121 (interface override) unmeasured |
| `const_expr_signed` whole-element arm (P3) | params.rs:130 default-lane operator/ternary sign, :768 cast arm, :799 value-inferred tail, param_query.rs:903 | measured c3 M_*_lp Ov_lp_*, c1 Sel_s_lv, c2 Fcanss Fints |
| `const_placement_wide` / `fold_count`, `const_array_elem_read` | read-only reuse | no edit |
Byte-identity for an expression whose width PRE knows: P1 runs only where `const_eval_u32(count)` returned None (PRE's arm returned
None there), P2 only where `const_select_self_width` returned None, and the element sign arms only for a BitSelect that
`const_array_elem_read` resolves — a node whose whole-element width PRE never knew, so no region whose width PRE knew contains it
in a context-determined position. Measured: release post3r `.vu` 15/15 and `.velab` 15/15 byte-identical to PRE (11 workloads + 4
examples); non-vacuous for P1 (trace: 24 firings in verilog-axi, `Some(32)`/`Some(64)`), vacuous for P2 (0 firings).
§4.5.580/581 666-variant harness: 0 movers (post2, post3). s593's 1658 cells: post1 9 / post2 27 movers, P2→P3 0.

## Q4 fix shape, moved cells, loud→value audit
Moved cells PRE → post3 (correct→anything = 0 in every set):
| set | cells | WRONG→OK | LOUD→OK | noref moves (onto verilator) | WRONG→WRONG′ | OK→other |
|---|---:|---:|---:|---:|---:|---:|
| c1 | 733 | 200 (118 3or + 82 s2v=vl) | 6 | 4 | 5 | 0 |
| c2 | 225 | 42 | 6 | 10 | 0 | 0 |
| c3 | 143 | 14 | 0 | 8 | 3 | 0 |
| s593 g/old | 1658 | 19 (p10 Eu1/Eu2 gc, Eu4 rb/gc, Eu7–Eu9 ×4; Ev12 3or; p3 W3_psw/W6_psw; Eu1/Eu2/Eu4 gc on sv2v alone) | 8 (Eu1/Eu2 lp gi rb, Eu4 lp gi) | 0 | 0 | 0 |
| 666 harness | 666 | 0 | 0 | 0 | 0 | 0 |
| cli tests (113 targets, 1522) | | 1 pin (`selfdet_bound_lanes::width_unknown_wrap_bound_keeps_preslice_decline`, a KNOWN-WRONG marker for this row: `CW=1` → `CW=fffffffffff` = verilator, sv2v) | | | | 0 other fails; elaborate + sim-engine 779/779 |
loud→value (each audited against the ref): Pweq_Rlp lp/gi (3or), Pweq_Eel lp/gi/rb (s2v=vl), Cps_R `ps=1` (iverilog+verilator;
sv2v `5`), Dfw80u `U=ff…ff B=80` (3or), Spow_R/Sshc_R `L=4` (3or), Spow_E/Sshc_E (s2v=vl), Wz80_lv `252 B=80`, Dw80v (verilator;
iverilog `B=81`), Cps_E `ps=3`, Cps_Ee `ps=1`, Fpexp_lv `2 ** AS[0]` `0 B=32` (verilator + hand-IEEE §11.4.3; sv2v 2^... junk),
p10 Eu1/Eu2/Eu4. None lands on a value the oracles reject.
Newly knowable → consumers it lands on: tier-3 bounds/counts (Cbnd_R `bb=6`→`2`, Cdim_R, Cebn_E `eb=2`→`256` verilator, Cbnd_Ee
`258`→`2`), fill sizing (Dfw2u, Dfe8u `U=1 B=1`→`fd B=8`), the >64 fences (Dfw80u, Dw80*: E3009 → right), untyped widths (lv),
self positions (Scond `5`→`7`, Spow/Sshc E3009→`4`). P2 without P3 lands 8 signed-element lv cells on sv2v's sign-dropped value
(M_div_lp `AS[0]/AS[1]` `254 B=8`, verilator `-2`; Ov_lp_se `252 lt0=0`, verilator `-4 lt0=1`) — P2 ships only with P3.
Residue V (WRONG→WRONG′, 8 cells, all untyped `localparam L = E` with a replication or element): Adiv_lv `X/{N{1'b1}}` PRE
`4294967295 B=32`, P3 `255 B=8`, 3 oracles `84 B=8`; Amod_lv `255 B=8` (oracles `0`); Aediv_lv `254` (s2v+vl `126`); Wz34_lv
`17179869180 B=34`, Wz64_lv `18446744073709551612 B=64` (`252`); K_ovr3 `508 B=9` (`252`); M_divu_lp, M_shr_su_lp `254` (`126`).
The width moves onto the oracles; the value is the unlimited `const_eval_in_scope` coerced to it, because the declared-width
value lane (`param_init_at_declared_width` → `param_decl_width_declared` → `declared_env_for` → `ctx_width_names_are_evident` /
`declared_override_widths`) has no Replicate / element arm. The literal twins are right on PRE (Tardiv_lv `X/2'b11` `84 B=8`,
Tarmod `0`, Tarshr `126`, Tarediv `126`, Tareshr `127`). Same gate holds the override twin: Eovu_R `#(.P(X + {N{1'b0}}))` onto
untyped `P` PRE = P3 `P=-4 B=32` (verilator `252 B=8`; literal twin Eovu_L right) = ROADMAP §2 "An unreadable override leaf
(element, replication, …) keeps the default's type" (ROADMAP:199 at a52e1a66).
Proposed shape (≤15 lines):
1. const_fn_width.rs `const_self_width` Replicate: `let n = const_eval_u32(count).or_else(|| self.const_placement_count(count, envw))?;`
   — `const_placement_count` = the count `fold_count` folds through `const_placement_wide`'s count resolver (factor that closure
   into one method both call; literal path first, locals in `envw` refused, no call arm — §4.5.371 ⓸ stays excluded).
2. same fn, select arm: `self.const_select_self_width(e).or_else(|| self.const_elem_wsign(e, envw).map(|(w, _)| w))`, where
   `const_elem_wsign` = `const_array_elem_read(e)` → `(elem_w, elem_signed)` (whole element, any packed_dims; a base an `envw`
   local shadows declines).
3. `const_signed_env` AND `const_expr_signed`: a BitSelect that `const_elem_wsign` answers takes `elem_signed` (both sign models
   together, ER §2.2 "route only with the whole type").
4. Pin: convert `selfdet_bound_lanes.rs::width_unknown_wrap_bound_keeps_preslice_decline` to `CW=fffffffffff` (verilator + sv2v
   text; iverilog refuses); new oracle-pinned file over Ev12/Eu7–Eu9 and the c1/c2/c3 families; record residues R1–R4, V, gc.
5. No frozen type, no format bump (elaborate-local; corpus `.vu`/`.velab` byte-identical).

## Q5 §2 / PROBE_CATALOG grep per code site (a52e1a66 line numbers)
- `const_self_width`: ROADMAP:100 WALL(AST self-width) line; :146 🆕 AI (this row); :672 §5.2 row 7; REMAINING_WORK:19.
- `const_signed_env`: ROADMAP:100. `const_expr_signed`, `const_placement_*`, `const_array_elem_read`, `ctx_width_names_are_evident`,
  `param_init_at_declared_width`, `eval_param_init`, `ast_selfwidths_all_known`: no line.
- `const_eval_u32` / `fold_count`: :146; :186 "A negative replication count is accepted" (K_neg/K_negu: E3009 PRE = P3, oracles
  refuse — not moved).
- `const_select_self_width`: :146, :672. `declared_override_widths`: :199 (override leaf keeps default type — residue V's twin).
- `param_decl_width_opt`: :201, :202, :494 (untyped/alias/dimquery arms; none is the operator arm). `const_bound`: :128 :140 (🆕 AC)
  :182 :463 :565; PROBE_CATALOG:85. Generate `case`: :178 (the gc column). `.len()`: :35 :515 :576, PROBE_CATALOG:45 (none is R3).
- Test marker: crates/cli/tests/selfdet_bound_lanes.rs:212–231 (RESIDUAL MARKER naming "const_self_width has no arm" for elements).

## Verdict
Start condition: GO for P1+P2+P3 (lanes measured or opted out; only iface_inst's override-sign read and the delay position lack
an oracle cell). Smallest safe cut: P1 alone is also safe (c1 0 OK→other, WRONG→WRONG′ 4 lv, c3 1) but leaves the element half;
P2 must not ship without P3. Prerequisite rows: none. Residues to file: R1 call count (`fold_count` has no call arm;
unselected-ternary silent + 🆕 AC `vb=1`), R2 const-fn local with a call-bound width (envw width 0), R3 `S.len()` width,
R4 65-bit signed neighbour beside a name count, V declared-width value lane has no Replicate/element arm (+ ROADMAP:199 override
twin, Eovu_R), gc = existing "Generate `case`" row. Effect on 🆕 W (post4w = fix + W): p11 17 OK→WRONG → 0, p8 12 → 0, Tv12 OK;
W's remaining descents are constant `==?` sinks only (p3 W1/W2/W10 13, c8 4, c9 20, c11 3, c12 6) — W still needs 🆕 S (a)'s i64 half.
