# §4.5.594 implementation — §2 🆕 AI (region rule cut)
status: DONE (post_a frozen; wt.diff md5 41d06f63eab3029a98adaf380ffd5708 = freeze; nothing committed)
worktree: $S/s594/wt (branch s594 @ 1da347bc); CARGO_TARGET_DIR=$S/s594/target
PRE: $S/s594/pre/ (to be built from 1da347bc)

## Step log
- setup: started
- setup: worktree $S/s594/wt (branch s594 @ 1da347bc) created; prewt @ 1da347bc built and removed with target_pre.

## Step 0 — PRE frozen ($S/s594/pre, built from 1da347bc in a separate checkout, never rebuilt)
- `cargo build --release --locked -p cli` → pre/vita md5 fc478eb9778be05e6d4ef7afa884443e (release, 7453936 B)
- `cargo build --release --locked -p cli --features separate-bins` → pre/sep/: vita 8b7832c45cdbbd746fa2f9fa9fc9b6a7,
  vcmp 1209038e2c99b54b33d6dff603bde85e, velab 9fdd51f69859c07a36e3a42d5c206212, vrun aa83118e5d6d5cd3863e1d7d4afc3118
- vs old PRE ($S/s592/pre/vita 86a2d84a…): i/runsets.sh (runv.py, vita only) over g/c1 c2 c3 c4, g/old/* (22 sets),
  a/cells..cells5: total=3082 same=3078 movers=4 — the 4 (a/cells3 Kdl_RAE Kdl_RG, a/cells4 Mdl_RAE Mdl_RG) are
  harness-format only (their `.pre` came from run4w.sh 2>&1 interleaving); re-run of old PRE in runv format: 4/4
  byte-identical to the new PRE. Run-time-lane difference from §4.5.590 on these sets: none observed.

## Steps 1–4 — code (debug build d1 md5 203a0a0da96b27168d2feeb5996c0a72, `cargo build --locked -p cli` rc 0, no warnings)
- const_array.rs `const_elem_wsign` (no shadow check, per PLAN).
- const_fn.rs: resolver closure → `const_placement_name`; `const_placement_count` = `fold_count` (now pub(crate)) through it.
- NEW const_leaf_rule.rs: `LeafRule {Literal, Scope}`, lazy `RegionRule` (root + Cell), `leaf_rule`, `region_rep_count`,
  `region_elem_wsign`. const_fn_width.rs: `const_self_width`/`const_signed_env` = wrappers over `_in(e, envw, &RegionRule)`;
  comparison arm: one RegionRule over the comparison node for both operands' width + sign; eval_const_env_self,
  eval_const_assign, leaf arm: one RegionRule per entry. const_eval.rs `const_expr_signed_in` + BitSelect Scope arm.
- DEVIATIONS from PLAN (both only remove answers; measured):
  D1 `leaf_rule` is also Literal inside a package function body (`const_call_pkg` set): both Scope resolvers walk the
     CALLING module's scopes (lookup_scoped / const_array_ref_of_base) — ER §5.3. Probes (i/probe):
     Pkf_cnt `pf = ((PC ? PX : {PN{1'b1}}) + 8'd4) == 8'h00` in package p, module `localparam int PN = 16`:
       PRE L=1, post3r L=0 (DESCENT), d1 L=1; iverilog, sv2v, verilator `L=1`.
     Pkf_el same with bare `PA[1]` and a module `logic [15:0] PA [0:1]`: PRE 1, post3r 0 (DESCENT), d1 1; sv2v, verilator 1
       (iverilog `sorry: unpacked array parameters`).
     Pkf_cntE (package fn, `== 8'hFC`): PRE 0, post3r 0, d1 0 (3 oracles 1) — no loss vs post3r; module twin Mfn_cntE
       PRE 0 → d1 1 (3 oracles 1).
  D2 tier-3 `ast_selfwidths_all_known` asks every node under the ROOT's RegionRule (PLAN's per-call wrapper = per node).
     T3n_rpt `repeat (A8[{N{1'b0}}] + fl(2))` (fl reads never-assigned r): PRE k=0, post3r k=3, d1 k=0; iverilog + sv2v
       hang (alarm 15 s, repeat(x)); verilator k=3 (2-state); hand-IEEE §12.7.2 x count → 0. T3e_rpt (`A8[CA[0]]`) same.
     T3n_rep `{(A8[{N{1'b0}}] + fl(2)){4'hF}}`: PRE rep=0, post3r fff, d1 0; iverilog/sv2v
       `error: Concatenation repeat may not be undefined (8'bxxxxxxxx)`; verilator `Replication value of < 0`.
  D3 (design consequence of PLAN step 2, not a choice) direct `fold_count` answers a zero name count `Some(0)` like the
     literal twin, where p123's synthesized `{count{1'b0}}` declined: Z0n_lt `(C ? X : {1'b0, {N{1'b0}}}) == 8'hFC`, N=0:
     PRE 0, post3r 0, d1 1 = literal twin Z0l_lt (PRE 1) = iverilog, sv2v, verilator `L=1`. Z0n/Z0l lp/lv: E3009 on all builds.

## Step-3/4 verification on d1 (i/cls3.py: every cell of g/c1–c4, g/old/*, a/cells–cells5 = 3082)
- d1 vs post3(r) and PRE: =both 2689, =post3r 355, =PRE 38, NEW 0.
- the 38 =PRE: the 19 region-rule cells (13 descents + 6 WRONG→WRONG′ of PLAN Audit 1) + the 17 R5 cells (Bred_okR×3,
  Bred_andR×3, Bred_aeR_gi, Kcl_RG, Kfn_clG, Kfn_rdG, Kfn_rdR, Kfn_cpG, Klv_RG, Kpw_RG, Mcmp_RG_int, g/c1 Bfa_lp, Bfr_lp)
  + Kdl_RG, Mdl_RG (value `t=254`/`t=264` same on all builds; post3r dropped PRE's W4030 backend-fallback warning
  because it folded the call-bearing delay; d1 keeps PRE's warning).

## Step 5 — pin conversion + stale claims
- selfdet_bound_lanes.rs `width_unknown_wrap_bound_keeps_preslice_decline` (`CW=1` marker) → value pin
  `const_array_element_bound_wraps_at_the_element_width` `CW=fffffffffff` (i/pin/Wcw: PRE `CW=1`, post3r/d1
  `CW=fffffffffff`, verilator + sv2v `CW=fffffffffff`, iverilog `sorry: unpacked array parameters are not supported yet.`);
  module-header oracle note extended.
- comment claims fixed: const_bound.rs (tier-3 gate "a const-array element"), const_fn.rs:545 (Pow exponent residual
  list), const_select.rs (`const_select_self_width` doc "A const-array element has no recorded width here"),
  const_wide.rs `fold_count` ⓵ ("there is no twin to forget here"). NOT edited (docs, left to the commit):
  docs/ROADMAP.md:100 WALL line (makes no element/count claim), :146 🆕 AI row, docs/REMAINING_WORK.md:19.

## Step 6 — tests: NEW crates/cli/tests/const_width_count_and_element.rs (22 tests)
- every design was dumped once (temporary hook, removed) and run on PRE / d1 / iverilog / sv2v / verilator (i/dump, 59
  designs); every oracle sentence in the file is that run's raw line. 2 doc corrections came from it (package count:
  iverilog runs it `L=1`; tier-3 cell: iverilog refuses the CA array, sv2v hangs).
- `cargo nextest run -p cli --locked --no-fail-fast --test const_width_count_and_element --test selfdet_bound_lanes`:
  Summary 30 tests run: 30 passed (debug).

## Gate (debug, CARGO_TARGET_DIR=$S/s594/target)
- `cargo nextest run -p cli --locked --no-fail-fast`: rc 0, Summary [23.195s] 8143 tests run: 8143 passed, 1 skipped
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: rc 0
- `cargo fmt --all -- --check`: rc 0

## POST frozen ($S/s594/post_a; source = post_a/wt_at_freeze.diff md5 41d06f63eab3029a98adaf380ffd5708)
- `cargo build --release --locked -p cli` → post_a/vita md5 4b3afc99cfeb6a2bc5bb94cfdeca3596 (release, 7453952 B)
- `--features separate-bins` → post_a/sep: vita c0610ca6bbfa6b51cb28048fa419707e, vcmp 6373c0b466be297343fc35bcc6379663,
  velab 3de6d22494291d2a2c70d76fd0606e94, vrun 30bc6f26dc235f526c699d4fd315790f

## Re-measure on PRE / POST (release) / oracles
- one-shot, 3082 set cells: POST release == debug d1 on 3082/3082; vs post3(r)/PRE: =both 2689, =post3r 355, =PRE 38, NEW 0.
- staged (i/staged.py vcmp→velab→vrun, sm.py-normalized vs one-shot of the same build), 3158 cells (sets + i/probe 16
  + i/dump 59 + i/pin 1): PRE 3158/3158, POST 3158/3158.
- PRE→POST transition (g/xt.py over i/summ/*.summ, ref = iverilog==sv2v, else ≥2 agreeing; sets + i/probe, i/dump
  excluded as duplicates): WRONG→OK 298 (3or 128, s2v=vl 161, 2or 2, 1or 6, iv=s2v!vl 1); LOUD→OK 20 (3or 5,
  s2v=vl 14, 2or 1); noref moves 31 (all land on verilator, or on iverilog+sv2v for the CA delays where verilator
  prints t=0 for every constant driver); WRONG→WRONG′ 8 (residue V: Adiv_lv Amod_lv Aediv_lv Wz34_lv Wz64_lv K_ovr3
  M_divu_lp M_shr_su_lp); OK→anything 0; LOUD→WRONG 0.
- a/ivd 11 probes: PRE == POST 11/11.
- 666-variant harness (i/h594a.py = g/h594.py with PRE=s594/pre, CUT=post_a): `variants 666 files 343`, `movers 0`.
- corpus `.vu`/`.velab` (g/corpus_velab.py): 30/30 byte-identical (15 workloads+examples × 2), vcmp=0 velab=0 all.
  Non-vacuity (temporary AI_TRACE build i/vita_trace md5 2cf033222a0c3d683313530363c94660, source restored, diff md5
  re-checked = freeze): verilog-axi `AI-TRACE-P1 Scope n=64` ×8, `AI-TRACE-P1 Literal n=32` ×8; P2 0× on all 15.
- `cargo run -p corpus-runner --locked -- run` in wt (bench rsynced from main excluding obj_dir/*.vvp;
  wt/target/release/vita = post_a/vita): rc 0; 10 `ok`, verilog-axi `ruled-split` (ROADMAP §2-N); coverage 11/11.
- elab A/B (i/elab_ab.py = s592's, `vita velab <row>.vu`, interleaved, order alternating per rep, rep 0 discarded,
  n=30): A=PRE B=POST geomean B/A 0.9967, rows 0.986..1.007, loadavg {1.79 2.49 3.31}→{2.05 2.50 3.30};
  reversed A=POST B=PRE geomean 0.9999, rows 0.971..1.016 (keccak 0.971: per-order 0.888 / 1.124, sign flips = noise;
  5.7 vs 5.5 ms), loadavg {1.96 2.47 3.28}→{1.83 2.40 3.23}. Verdict: no change (±3%).
- NOTE (not changed, to keep post_a = final source): `select_init_meta` (const_array.rs, untyped select initializer)
  spells the same element rule `(m.elem_w > 0).then_some((m.elem_w, m.elem_signed))` as `const_elem_wsign` — ER §5.1
  "one rule, one home" says route it through the helper; a byte-identical 4-line follow-up (tried, reverted).

## Mutant battery — expected outcomes (written before running)
command per mutant: `cargo nextest run --workspace --locked --no-fail-fast` (debug, CARGO_TARGET_DIR=$S/s594/target).
File: T = crates/cli/tests/const_width_count_and_element.rs, SB = selfdet_bound_lanes.rs.
- M1 leaf_rule.rs:101 placement count → None: T ev12, every_count_spelling, a_count_from_every_binder,
  a_name_count_reaches_every_width_consumer, a_zero_name_count die.
- M2 fn_width.rs:295 element width → None: T a_whole_element_sizes_the_region, every_element_type (4 widths),
  a_signed_element_keeps_its_sign (B=8 cells), SB const_array_element_bound_wraps_at_the_element_width die.
- M3 fn_width.rs:369 const_signed_env_in element sign → false: T a_signed_element_keeps_its_sign (cmp, tern),
  a_signed_element_override_extends_by_its_sign die.
- M4 const_eval.rs:519 const_expr_signed_in element sign → false: T a_signed_element_keeps_its_sign (div, sum) dies.
- M5 const_array.rs:188 element sign → true: T an_unsigned_element_stays_unsigned dies (time cell may too).
- M6 leaf_rule.rs:85 Literal → Scope: T rule_* (8) and known_wrong_regions_the_rule_keeps die.
- M7 leaf_rule.rs:82 call check → false: T rule_a_clog2, rule_a_call_bearing_repeat, rule_a_call_bearing_delay,
  rule_a_comparison_is_one_region, rule_a_tier3_bound, known_wrong_regions (call-bearing width) die.
- M8 leaf_rule.rs:83 envw check → false: T rule_a_reduction, rule_a_region_naming_a_function_local,
  known_wrong_regions (function local) die.
- M9 fn_width.rs:616/617/624/625 per-operand rule: T rule_a_comparison_is_one_region dies.
- M10 leaf_rule.rs:81 package-body check → false: T rule_a_package_function_body_keeps_pre_s_leaf_rules dies.
- M11 const_bound.rs:132 tier-3 per-node rule: T rule_a_tier3_bound_takes_its_root_s_rule dies.
- M12 const_array.rs:188 drop `elem_w > 0`: expected SURVIVOR (no element has width 0: `const_array_elem_geom`
  multiplies `w.max(1)`, `range_to_dims` gives ≥1) — recorded as unreachable, not killed.

## Mutant battery — results (i/mut/battery.sh: byte-snapshot restore + trap, apply by line with anchor count check;
## each `cargo nextest run --workspace --locked --no-fail-fast`; status before == after; diff md5 after = freeze)
| id | result | killers (all in const_width_count_and_element unless noted) |
|---|---|---|
| M0 | 9167 passed, 15 skipped (baseline) | — |
| M1 | 6 failed | ev12, every_count_spelling, a_count_from_every_binder, a_name_count_reaches_every_width_consumer, a_zero_name_count, known_wrong_an_untyped_value_over_a_name_count |
| M2 | 4 failed | a_signed_element_keeps_its_sign, a_whole_element_sizes_the_region, every_element_type, selfdet_bound_lanes::const_array_element_bound_wraps_at_the_element_width |
| M3 | 2 failed | a_signed_element_keeps_its_sign, a_signed_element_override_extends_by_its_sign |
| M4 | 1 failed | a_signed_element_keeps_its_sign |
| M5 | 2 failed | an_unsigned_element_stays_unsigned, every_element_type (time) |
| M6 | 9 failed | known_wrong_regions + all 8 rule_* |
| M7 | 6 failed | known_wrong_regions, rule_a_call_bearing_delay, rule_a_call_bearing_repeat, rule_a_clog2, rule_a_comparison_is_one_region, rule_a_tier3_bound |
| M8 | 3 failed | known_wrong_regions, rule_a_reduction, rule_a_region_naming_a_function_local |
| M9 | 1 failed | rule_a_comparison_is_one_region |
| M10 | 1 failed | rule_a_package_function_body_keeps_pre_s_leaf_rules |
| M11 | 1 failed | rule_a_tier3_bound_takes_its_root_s_rule |
| M12 | SURVIVED (9167 passed) | expected: unreachable — `range_to_dims_opt` returns |msb−lsb|+1 or a fixed atom width or (1,0,0) on decline; `const_array_elem_geom` folds packed dims with `w.max(1)`; so `elem_w ≥ 1` and the `elem_w > 0` guard (copied from `select_init_meta`'s spelling) never declines. Not edited (would move post_a). |
Wall per mutant 93–108 s.

## Final
- $S/s594/post_a/wt.diff md5 41d06f63eab3029a98adaf380ffd5708 (= wt_at_freeze.diff); new files `git add -N`.
- wt `git status --short`: A const_width_count_and_element.rs, A const_leaf_rule.rs, M ×9 (cli test, elaborate ×8).
  main: `?? .DS_Store`, `?? AGENTS.md` (pre-existing, untouched). wt also holds ignored bench/ (rsynced) and
  target/release/vita (= post_a/vita) for corpus-runner.

# Round-2 delta (round-1: differential PASS, soundness FAIL F1)
status: DONE (post_b frozen; nothing committed)

## Fix 1 (F1) — one rule per evaluation, narrowing only; the interpreter Literal whole
- `Elaborator.region_literal: Cell<bool>` (lib.rs field, driver.rs init). `region_rule(root)` (const_leaf_rule.rs):
  Literal if a Literal region is open, else `leaf_rule(root)`; a Literal decision sets the flag until the returned
  `RegionRule` drops (Drop clears it only in the region that set it). `literal_region()` opens a Literal region
  unconditionally. `leaf_rule` = call check only (`ast_any(Call)`).
- region entries (every `RegionRule` is opened through `region_rule`): const_self_width / const_signed_env /
  const_expr_signed wrappers, eval_const_env_self, eval_const_assign, comparison arm, leaf arm, tier-3 root, and NEW
  `const_eval_in_scope` (now an entry over `const_eval_in_scope_walk`, whose own 8 recursive calls stay in the walk).
- nested roots then inherit through the dynamic extent: ternary condition / shift amount / `**` exponent /
  `$clog2` arg (eval_const_env_self, const_unsigned_selfdet), cast operand (const_size_cast → eval_const_assign),
  select bound / index (select_span → eval_const_env_self), element index (const_array_elem_read →
  const_eval_in_scope), the comparison redirect inside const_eval_in_scope, call arguments.
- `literal_region()` held for the whole of `eval_const_call` (args, header, body) and `const_fn_ret_wsign_in`
  (header text; keeps the caller's call width = the callee's ret width). Reason (measured on post_a, i/r2):
  C11 `u = (cmp); fk = r + u;` (r never assigned): PRE 0, post_a 1, iverilog 0, sv2v/verilator x — the cross-statement
  twin of C3, which no per-region test sees; C10 `localparam int L = fl(0) + f(0)` with f's body = (cmp): PRE 0,
  post_a 1, iverilog 0, sv2v/verilator x; C10g as generate-if: PRE else, post_a then, iverilog/sv2v else, verilator then.
- removed (now subsumed, so unkillable): the `envw` name arm and the D1 package-body arm of `leaf_rule` — `envw` entries
  exist only inside eval_const_call (Literal) and on the declared-width lanes, which admit no replication
  (`ctx_width_names_are_evident` has no Replicate arm) and no array-element base (`declared_override_widths`).
- debug d3 (md5 59edf134e9810dd210bc3f4622d104be) on lens cells: C1 0, C2 else, C3 0, C8 E3009 (`the `+` operation has no
  constant-fold arm`), C9 vb=4 — all = PRE; C0 1 (= oracles). C3c 0 = PRE (NOT post_a's 1, see deviation R2-D1).
- delta post_a → d3 over 3268 cells (all sets + probes + r2 + lens copies): 20 movers, all to PRE: C1 C2 C3 C3c C8 C9
  (r2 + lens copies), C10 C10f C10g C11 C11c, and 3 lost gains g/c1/Bfn_lp, g/c2/Grl_lp, i/probe/Mfn_cntE
  (function body holding only the comparison; 3 oracles `L=1`).

## Fix 2 — docs
- const_elem_wsign: "whatever the index" → "for an index the element read resolves — None, like
  const_array_elem_read, for an unfoldable, negative or out-of-range index".
- const_leaf_rule.rs module doc rewritten (rule = call check; narrowing inheritance; interpreter Literal whole; why no
  envw arm). "PRE's line verbatim" now stated as "Inside an open Literal region every width and sign answer is PRE's:
  the two Scope arms answer None, and nothing else in the walks reads the rule" — code path: region_rep_count /
  region_elem_wsign return None on Literal; every other arm of const_self_width_in / const_signed_env_in /
  const_expr_signed_in / ast_selfwidths_all_known_in is PRE's code; `rr` is read nowhere else.
- comments updated: comparison arm, leaf arm, const_fn.rs const_placement_count doc and the Pow residual list,
  const_array.rs shadow-check sentence, lib.rs field doc.

## Fix 3 — pins (const_width_count_and_element.rs, 22 → 28 tests; every new design dumped and oracle-run, i/dump2)
- rule_a_comparison_nested_in_a_call_bearing_region_inherits_it: C1 `L=0` (iverilog `L=0`; sv2v, verilator `L=x`);
  C2 `GI=else` (iverilog, sv2v `GI=else`; verilator `GI=then`).
- a_comparison_beside_a_constant_keeps_its_width: C0 `L=1` (3 oracles `L=1`).
- rule_a_self_determined_position_nested_in_a_call_bearing_region_inherits_it: C8 loud `the `+` operation has no
  constant-fold arm` (iverilog `L=0`; sv2v, verilator `L=x`); C9 `vb=4` (iverilog `vb=1`, sv2v `vb=x`, verilator
  `%Error: … left side of bit range isn't a two-state constant`).
- rule_the_constant_function_interpreter_keeps_pre_s_leaf_rules: C3, C11, C10 `L=0` (iverilog `L=0`; sv2v, verilator
  `L=x`), C10g `GI=else` (iverilog, sv2v else; verilator then).
- known_wrong_the_interpreter_gives_up_its_gains: C3c, C11c, Bfn_lp `L=0` (3 oracles `L=1`).
- a_signed_element_leaf_enters_an_unsigned_region_zero_extended: Mx1d `(AS[0] / 8'd2) == 8'd126` `L=1` (sv2v, verilator
  `L=1`; iverilog sorry; PRE `L=0`) — the cell separating round-1's Mx1 (see residues).
- `cargo nextest run -p cli --locked --no-fail-fast --test const_width_count_and_element --test selfdet_bound_lanes`:
  36 passed.

## Deviations from the round-2 brief
- R2-D1 C3c stays at PRE (`L=0`), not post_a's `L=1`: C3c and C3 differ only by the statement `r = 8'd0;`; a rule
  that is structural cannot separate an assigned local from a never-assigned one (that is §2 🆕 AE), and the same holds
  for C11/C11c. Pinned KNOWN-WRONG with the oracles' `L=1`.
- R2-D2 the rule also covers called functions (C10, C11 above) — 3 extra cells move back to PRE (Bfn_lp, Grl_lp,
  Mfn_cntE), all WRONG→OK on post_a, back to PRE's WRONG (R5 widens: "a constant-function body").

## Round-2 mutant battery — expected outcomes (written before running; T = const_width_count_and_element)
- M1 leaf_rule.rs:135 count → None: T ev12, count spellings, binders, consumers, zero count, residue V die.
- M2 fn_width.rs:295 element width → None: T element tests + SB pin die. M3 fn_width.rs:369, M4 const_eval.rs:518 sign →
  false: T signed-element tests die. M5 const_array.rs:189 sign → true: T unsigned / time die.
- M6 leaf_rule.rs:117 call check off: T module-scope call-region rules die (clog2, repeat, delay, comparison_is_one_region,
  tier3, nested comparison, nested self-determined, known_wrong_regions call-bearing width); interpreter rules survive.
- M13 leaf_rule.rs:87 nested root ignores the open Literal region (widen Literal→Scope): T nested comparison (C1/C2),
  nested self-determined (C8/C9), interpreter (C3/C11/C10) die.
- M14 const_fn.rs:1479 interpreter not forced: T interpreter rule, reduction, function-local, package body,
  known_wrong interpreter gains die.
- M15 decl_scope.rs:430 header fold not forced: T known_wrong_a_return_range dies (untyped: width from the unforced
  header fold, value from the forced one).
- M16 const_fn.rs:243 const_eval_in_scope opens no region: T nested comparison (localparam C1), C8 loud, C10 die.
- Mx1 fn_width.rs:772 leaf arm always Literal: T a_signed_element_leaf_enters_an_unsigned_region dies.
- M9 fn_width.rs:615 comparison rule over lhs only: expected SURVIVOR, equivalent by construction (eval_const_env_at
  runs only under an entry whose root contains the comparison, so its rule = the enclosing one).
- M11 const_bound.rs:132 per-node tier-3 rule: expected SURVIVOR, equivalent (the root region is open for the walk).
- M12 const_array.rs:189 drop `elem_w > 0`: expected SURVIVOR (elem_w ≥ 1, round 1).
- extra pin known_wrong_a_return_range_keeps_pre_s_width (i/r2h Hr1/Hr2): return range `[({N{1'b1}} + 2'd1):0]`
  PRE/post_b `L=31 B=5`, `K=31 vb=33`; post_a `L=1 B=1`, `K=1 vb=3`; iverilog, verilator `L=1 B=1` (sv2v `L=1 B=5`),
  `K=1 vb=3` (3 oracles) — a header-text gain post_a had, given up with the interpreter (kills M15).

## Gate (final source; product code = post_b + comment-only edits, binaries verified byte-identical)
- `cargo nextest run -p cli --locked --no-fail-fast`: rc 0, Summary [22.773s] 8150 tests run: 8150 passed, 1 skipped
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: rc 0; `cargo fmt --all -- --check`: rc 0
- workspace baseline (battery M0): 9174 passed, 15 skipped.

## post_b frozen ($S/s594/post_b)
- vita md5 511dffa2f0fdcb6953a4aa9b241ec74b (release, 7470464 B); sep: vita 44363b080b936bfbb3a23c0ba4cd5ff4,
  vcmp f8ebe5dec98aaefa36481f87c6757c21, velab 715e142adb119bfe9eb184d400817108, vrun dfa8fe360a6da66d3f18cceabd588435.
- post_b/wt_at_freeze.diff md5 aea609ff0d7d2db1652a9f105d722476; after it: 4 comment edits (const_eval.rs 1 line,
  const_fn.rs 5→5, const_fn_width.rs 6→6, const_bound.rs 3→3 lines) and one test (Hr pin). Rebuilt release + staged
  after them: all five binaries byte-identical to post_b. Final post_b/wt.diff saved (md5 in the return).

## Re-measure PRE vs post_b
- one-shot over 3269 cells (all sets, a/ivd, i/probe, i/dump, i/pin, i/r2, lens copies diff_c..c5 + sound): post_b
  release == debug d3 on every cell present in both (C8k new).
- staged == one-shot: PRE 3269/3269, post_b 3269/3269.
- delta post_a → post_b: 21 movers — C1 C2 C3 C3c C8 C9 (r2 + lens), C10 C10f C10g C11 C11c, Bfn_lp Grl_lp Mfn_cntE all
  back to PRE; C8k new (= PRE `K=2`, 3 oracles `K=2`). Every other cell byte-equal to post_a.
  vs oracles (int localparams read hand-IEEE: x → 0 in a 2-state int, iverilog's line): to-PRE OK→… restorations
  C1 C3 C10 C11 (`L=0` = iverilog), C2 C10g (`GI=else` = iverilog, sv2v), C8 (loud = PRE; iverilog `L=0`), C9 (`vb=4`,
  oracles split); lost gains (WRONG on PRE, OK on post_a): Bfn_lp Grl_lp Mfn_cntE C3c C11c C10f (3 oracles `L=1`).
- PRE → post_b over the round-1 sets + probes (g/xt.py): WRONG→OK 295 (3or 125, s2v=vl 161, 2or 2, 1or 6,
  iv=s2v!vl 1), LOUD→OK 20, noref moves 31, WRONG→WRONG′ 8 (residue V), OK→anything 0, LOUD→WRONG 0.
- 666 harness PRE vs post_b: `movers 0`. a/ivd 11/11 equal.
- corpus .vu/.velab PRE vs post_b 30/30 byte-identical. Firing trace (temporary i/vita_trace2, source restored, diff
  md5 = freeze): verilog-axi `AI-TRACE-P1 Literal n=Some(32)` ×8, `Literal n=Some(64)` ×4, P1 Scope 0×, P2 0× — the
  corpus no longer reaches a Scope answer (post_a's 8 Scope sites are now inside call-bearing regions / the interpreter):
  byte-identity is non-vacuous for the Literal path only; the Scope path is covered by cells.
- corpus-runner (wt/target/release/vita = post_b): rc 0, 10 ok + verilog-axi ruled-split, coverage 11/11.
- elab A/B (n=30, interleaved, rep 0 discarded; machine loaded, loadavg 9.6→7.6 and 7.6→5.6): A=PRE B=post_b geomean
  0.9979 (rows 0.946..1.013; keccak 0.946 per-order 0.883/1.126 = noise); A=post_b B=PRE geomean 0.9971 (rows
  0.990..1.012). ibex 1.007 / 0.990 and verilog-axi 1.013 / 0.990 agree in sign (post_b ~1% slower), inside ±3%.
  Verdict: no change.

## Round-2 mutant battery — results (i/mut2; `cargo nextest run --workspace --locked --no-fail-fast` each;
## status before == after; elaborate diff restored)
| id | result | killers (T = const_width_count_and_element) |
|---|---|---|
| M0 | 9174 passed, 15 skipped | — |
| M1 | 6 failed | T ev12, count spellings, binders, consumers, zero count, residue V |
| M2 | 5 failed | T signed-element sign walks, signed element leaf (Mx1d), whole element, element types; SB pin |
| M3 | 2 failed | T signed element sign walks, override sign |
| M4 | 1 failed | T signed element sign walks |
| M5 | 2 failed | T unsigned element, element types (time) |
| M6 call check off | 8 failed | T known_wrong_regions, call-bearing delay/repeat/clog2, comparison_is_one_region, nested comparison (C1/C2), nested self-determined (C8/C9), tier3 |
| M13 nested root ignores the open Literal region | 9 failed | T nested comparison (C1/C2), nested self-determined (C8/C9), interpreter (C3/C11/C10), reduction, function local, package body, interpreter gains, regions, return range |
| M14 interpreter not forced | SURVIVED | equivalent by construction: eval_const_call's two callers (const_eval_in_scope_walk's Call arm, eval_const_env's Call arm) run inside an open region whose root holds the call; kept as a caller-independent guard, documented in the code comment |
| M15 header fold not forced | 1 failed | T known_wrong_a_return_range_keeps_pre_s_width |
| M16 const_eval_in_scope opens no region | 2 failed | T nested comparison (C1), nested self-determined (C8) |
| Mx1 leaf arm always Literal | 1 failed | T a_signed_element_leaf_enters_an_unsigned_region_zero_extended (Mx1d) |
| M9 comparison rule over lhs | SURVIVED | equivalent: eval_const_env_at runs only under an entry whose root holds the comparison (comment says so) |
| M11 tier-3 per-node | SURVIVED | equivalent: the root region stays open for the walk (doc says so) |
| M12 drop `elem_w > 0` | SURVIVED | unreachable (elem_w ≥ 1, round 1) |

## Residues (no code)
- E1 (soundness): type-parameter element `localparam T AT [0:1]` captured with the DEFAULT T: `u2 #(.T(logic [7:0]))`
  PRE = post_a = post_b `t.u2 L=1 M=0`; verilator `t.u2 L=0 M=0`, sv2v `t.u2 L=0 M=0` (sv2v also drops u1's sign:
  `t.u1 L=0`; verilator `t.u1 L=1`); iverilog `sorry: unpacked array parameters`. Pre-existing, not moved.
- Mx1 (soundness, survived round 1): SEPARATED — Mx1d `localparam L = ((AS[0] / 8'd2) == 8'd126);` over
  `logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2}`: PRE 0, post_a/post_b 1, verilator 1, sv2v 1, iverilog sorry; the
  Mx1 mutant (leaf arm Literal) gives the raw −4 into the division → killed (round-2 battery). A replication leaf is
  equivalent under Mx1 (its value is the placement fold's, already zero-extended at its width).
- m1 generate-case (differential D21; existing ROADMAP "Generate `case` compares two i64 values" row):
  `case (X + {N{1'b0}})` PRE/post_a/post_b `GC=def`, iverilog/sv2v/verilator `GC=fc`; the same cell's generate-if
  moved `GI=else`→`GI=then` (= oracles), so vita now splits internally (GI=then, GC=def).
- m2 residue V (differential D14 L4): `L4` PRE `4294967295`, post `255`, 3 oracles `63` — width right, value the
  unlimited fold coerced.
- m3 R5 (differential D30): `localparam L = g(1);` PRE = post `L=0`, 3 oracles `L=1`; R5 now also covers every
  constant-function body and header (Bfn_lp, Grl_lp, Mfn_cntE, C3c, C11c, C10f, Hr1/Hr2 above).
