# IMPL §4.5.587 unique-const-fn (implementer)

status: ROUND 2 COMPLETE — post_c frozen; verdict PASS (see round-2 tail)
worktree: $S/wt (branch s587 @ 7b33a009); target $S/target_s587

## Step 1 — G6 (PRE e1e7e571 fu/fp, iverilog 13.0 fp, verilator 5.052 fu/fp)
cells: $S/g6/<lane>_{u,p}.sv (u = `unique if`, p = plain `if`, otherwise identical); raw: .vita/.ivl/.vl;
verilator elab-task output in vl_<cell>.build; full table $S/g6/g6_pre_table.txt (68 lanes)
KEEP (PRE fp == oracle): g01a hdr_sub W=7, g01b hdr_top P=7, g02a ifc_hdr P=7, g02b ifc_body P=7, g02c ifc_real R=7.00,
 g02d ifc_lp P=7, g03a untyped P=7 b=4, g03b typed8 P=07 b=8, g03c real R=7.00, g05a inst_arr hit u[7],
 g05c formal_arr s=817 (vl; ivl rejects array actual), g05e lp_range P=ff b=8, g05f enum_base b=8 e=ff (vl; ivl no fn),
 g05g vcd $var [7:0]/[0:7]/[7:-2] (vita = ivl), g05h neg_lsb b=10 l=7 r=-2 v=3fe, g05i asc_lsb, g05j lp_arr n=8 a7=17 q=17 (vl),
 g05k task_local tb=8, g05l task_susp tb=8, g05m port_unp l=7 r=0 i=1, g05n net_unp, g05p md_packed, g05r/g05s string [0:N] dims,
 g05u $bits prescan B=8, g05v task-local unpacked dims, g06a ur_bits b=28, g06b ur_query l=0 r=6 i=-1 s=7 d=2, g06d formal_n s=716 (vl),
 g06f frame_str n=7 s6=z (vl), g06g/g06h string [N] dims, g06i block-local string [N], g06j B=$bits(u)=28,
 g07a cast_lp P=127, g07b cast_bits b=7, g07c cast_rpt n=15, g08a-d hier r=4d/v=ab80/r=cd/v=ab00 (PRE fu silent-wrong r=1/v=0000),
 g09a-e region r=40 (PRE fu silent-wrong), g10a neg count E3009 = both oracles error (PRE fu silent r=00), g10b zero cnt r=05,
 g11a md_pr m=89abcdef, g11b md3 m=fedcba98, g11c md_prw, g12b ifc elab I3006 el=7 (vl -Info el=7), g12c $fatal boom (all),
 g12d module elab el=7, g13a/g13b localparam+repeat P=7 n=7 (vl fu: one run-time violation at [1])
DROPPED (PRE fp != oracle):
 D1 g04/g04b generate-typedef enum label `A = f(2)`: PRE fp E3010 undeclared `top.g[0].A`; vl A=7 B=8; ivl unable to bind.
    No edit needed: gen_enum.rs `literal_only` rejects a Call, so a call label is never carried and never reaches
    instance.rs:122/:132 from generate (the module typedef wrap cannot reach it).
 D2 g05b instance-array child port range (instance_array.rs:121/:135): PRE fp E3009 "a child port width does not const-fold";
    ivl/vl `hit top.u[7] b=4`. Edit: these two callers keep an Unstated spelling of W1.
 D3 g12a/g12f elaboration-task `%s` of a call (elab_task.rs:92/:95): PRE fp E3009 (string-literal body) / `s=   A` (8'h41 body)
    vs vl `s=A`; ivl rejects. Edit: leave both unwrapped.
fp differs from oracle on an unrelated READER, position re-measured by a refined cell (kept): g05d/g06c/g06e `$size` of a string
 array (E3009 unsupported system function) -> g05r/g05s/g06g/g06h/g06i; g05o/g05t task-local `'{}` / foreach -> g05v;
 g05q `$bits(late)` before its declaration (fp E3009 undefined name; vl 8, ivl 0 split) -> g05u.
Residue (not opted in by P2, measured): g11d `m3[1][f(2):0] = '0` PRE fp correct, PRE fu E3009 "nested lvalue select" (packed_lval.rs:365, opt-out residue).
Pre-existing (PRE = POST expected): g6x/ia_shadow: instance-array prepass folds child port range `[f(2):0]` with the PARENT's f
 (child f=3): vita E3009 "port `p` width 8 expects ... got 32"; ivl/vl `hit top.u[7] b=4`.
 elab `$info("%s", g(2))` with 8-bit g: vita `s=   A`, vl `s=A`.
drops 3 / 57 lanes (< 1/3): proceed.

## Step 2 — mechanism only (const_site.rs, lib.rs field+mod, driver.rs init, arm + docs in const_fn.rs)
build: CARGO_TARGET_DIR=$S/target_s587 cargo build -p cli --release --locked --features separate-bins rc=0 (72 s; 1 expected
 dead_code warning: const_required unused) log $S/logs_impl/s2_build.log
binary $S/mech/vita md5=6fe6b4ba1a413517e63c32f5af6cb18d size=7338000 release separate-bins
PRE determinism: pres vs pres2 (re-run of $S/pre/sep/vita): 444/444 identical (text + run.json minus timing keys)
mechanism-only vs PRE: 444/444 identical (text + run.json); G6+g6x 137/137 identical. log $S/logs_impl/s2_mech_vs_pre.txt
harness: $S/rerun444.sh <vita> <sfx>; $S/cmp444.py <sfxA> <sfxB> [dirs]

## Step 3 — W1-W6 (binary $S/s3/vita md5=f72f95389855598bff8713b4ac0779c7)
444 cells: 24 move (b_pr, b_pw, b_ur, b_gi, b_gf x {if,pif}; e_td, e_fret, e_fform, e_barr, c_cast, c_rep, c_rep2, c_rep3,
 e_pswr, e_psww, h_prd, h_pwr, h_mdpr, h_plsb), 420 identical; 23/24 = (a)-POST text; h_mdpr m=ef (intermediate: W6 width
 folded, packed.rs:2204 element scaling not yet opted in) -> fixed in step 4. log $S/logs_impl/s3_vs_pre.txt

## Step 4 — call-site opt-ins (binary $S/s4/vita md5=da2c28ab96a4ccd0052d32a6f0f2b97f)
Deviation D-held: G6 g05w (instance-array child whose HEADER default calls the child's function): PRE fp E3009 (prepass folds the
 child's header params at the parent's prefix, bind_params -> bind_one_param). bind_one_param is opted in for the real binders, so
 the prepass needed a hold: new ConstSite::Held + Elaborator::const_held (&mut self); const_required does not elevate inside Held.
 The instance_array.rs prepass block (bind_params + child port ranges) runs in const_held; this replaces the D2
 const_range_bound_fold_unstated call sites (that fn is now the private W1 body).
Also wrapped (plan said "every fold of the declaration's own text"): param_decl_range_opt in the three binders
 (instance.rs:748, generate.rs:819, params.rs:2450), wide_param_decl_range in bind_wide_param_decl (all callers are the three binders)
 and params.rs:2355; param_value_unfoldable's reason (all 3 callers are opted-in binders naming p.value).
444 cells: 48 moved, all 48 == (a)-POST text, moved set == P3 48-cell list exactly; 396 identical (23 held + 373). log $S/logs_impl/s4_vs_pre.txt
G6 (71 lanes incl. g01c/g01d overridden-default probes, g05w): POST fu == PRE fp in 68; differ 3: g12f (D3 dropped, stays E3009 = PRE fu),
 g13a/g13b (POST fu P=7 n=7 + one W4031 at time 1 = verilator fu's one run-time violation; fp has none). No _p cell changed.
 Dropped lanes unchanged vs PRE fu: g04, g05b, g05w, g12a, g12f. g11d (nested lvalue m3[1][f(2):0]) moved to PRE fp value
 (packed_inner.rs:152 also serves the write path packed_lval.rs:132), so it is not a residue.

## Step 5 — docs: const_site.rs module doc (rule, positive set W1-W6 + call sites, Held, notes in same mode), exec_const_stmt doc
 (moved from atop exec_const_select_write, where it had drifted), hdl-parser lib.rs first_if_arm_only bullet 1,
 unique_if_chain.rs :18-30 / :578-582 / :677-680 prose, manual 006 §1.4 (first held-back reason + new paragraph), manual 003
 `unique`/`priority` row, CHANGELOG [Unreleased] "Fixed — a `unique if` no-match in a constant function folds where a constant is required".
## Step 6 — crates/cli/tests/unique_const_fn.rs (13 tests: T1 params + staged, T1b priority0, T2, T3, T4, T4b lsb residue, T4c negative
 count, T5, T6 (4 cells), T7 (2 orders), T8 (3 REFUSED), T9 (3 REFUSED), T10 (4 REFUSED)); designs + raw oracle outputs in $S/t/
 (t_run.sh: vita fu, verilator fu/fp, iverilog fp). `cargo nextest run -p cli --test unique_const_fn --locked --no-fail-fast`
 rc=0 13 passed. log $S/logs_impl/s6_test.log

## Step 7 — mutation battery (expected outcomes written BEFORE running; each mutant: full `cargo nextest run --workspace --locked --no-fail-fast`)
baseline POST: 9061 passed, 15 skipped, rc=0 ($S/logs_impl/s7_baseline.log)
| id | mutant | expected killer |
| M1 | delete the arm | T1 params, T2 ranges, T3 generate, T4 widths, T4b lsb, T4c negcnt, T5 cast, T7 two_positions |
| M2 | drop `&& const_site == Required` (shape (a)) | T6 runtime_positions_keep_the_report, T7, T8 (t8a value), T9 (ov/dp/cg values), T11 prepass |
| M3 | const_required sets Unstated | same as M1 |
| M4 | const_required never restores | T7 (localparam-first), likely T6/T8 (any earlier Required fold leaks) |
| M7 | unwrap generate.rs case label fold | T3 (gl=2) |
| M8 | array_geom.rs:845 compute_dim_desc back to const_eval_in_scope | T2 $left/$right/$size of `un [f(2)]` (survival => add cell) |
| M10 | wrap stmt_flow.rs:1119 repeat_unroll_count in const_required | T6 t6a, T7 |
| M12 | arm matches any SysTaskCall | T10 t10d ($display body folds) |
| M13 | instance_array prepass without const_held | T11 instance_array_prepass_states_nothing (silent p=a) |
| M5 (opt) | W1 body not wrapped | T2 |
| M6 (opt) | W5 fold not wrapped | T4 |
| M9 (opt) | instance.rs body binder meta unwrapped | T1 (bH=4) |
| M11 (opt) | wrap events.rs:46 intra-assign repeat in const_required | T8 t8a |
results (each: diff verified, restored by cmp, full workspace run; log $S/mut/<id>.log, killers $S/mut/<id>.kills):
| M1 | killed rc=100 9054 pass/8 fail | T1 params, T2 ranges, T3 generate, T4 widths, T4b lsb, T4c negcnt, T5 cast, T7 two_positions (= expected) |
| M2 | killed 5 fail | T6, T7, T8, T9, T11 (= expected) |
| M3 | killed 9 fail | the 8 of M1 + cli::aes_report_r30 run_json_splits_elaborate_from_simulate (wall-clock assertion sim_s > elab_s: elab 0.003265 sim 0.002215 under parallel load; not a mutant effect) |
| M4 | killed 2 fail | T7 one_function_two_positions, T8 runtime_positions_vita_requires_constant_stay_loud |
| M7 | killed 1 fail | T3 generate_constants_fold_silently |
| M8 | killed 1 fail | T2 declared_ranges_fold_silently (mutant: `un size=4 left=3 right=0`, `blk=32`) |
| M10 | killed 2 fail | T6 runtime_positions_keep_the_report, T7 |
| M12 | killed 2 fail | T10 case_and_systask_bodies_stay_loud, cli::const_function_eval system_task_in_body_stays_loud |
| M13 | killed 1 fail | T11 instance_array_prepass_states_nothing |
| M5 | killed 1 fail | T2 declared_ranges_fold_silently |
| M6 | killed 1 fail | T4 widths_counts_bounds_fold |
| M9 | killed 1 fail | T1 params_fold_silently (mutant: `bH=32`) |
| M11 | killed 1 fail | T8 runtime_positions_vita_requires_constant_stay_loud |
survivors: none (13/13 killed). post-battery: git status --short identical to pre-battery; all snapshot files cmp-identical.

## Step 8 — scoped gates (final tree)
fmt --check rc=0 ($S/logs_impl/s8_fmt.log); clippy --workspace --all-targets --locked -D warnings rc=0 (s8_clippy.log);
unique_const_fn rc=0 14/14 (s8_ucf.log); unique_if_chain rc=0 20/20 (s8_uic.log); -p cli rc=0 8038 passed 1 skipped (s8_cli2.log);
(workspace baseline during step 7: 9061 passed 15 skipped, before T11 was added; mutants ran 9062)

## Step 9 — freeze (POST(b)) and lanes
binaries $S/post_b/: vita cad7fe9a7721be97634d808055cb73b8 7338016, vcmp 6a760e183fae684e0ef8d0a44b06b95a, velab d13af1b89d7a30d8532e3483c920aa12, vrun 6568b9520b2243a98452dfd966e9d912
 (release, --features separate-bins); vita_jit f5f5cb52283f4a87701adf9a3b1b8d4d 9197888 (release, --features jit, $S/target_s587_jit)
 built from: git -C $S/wt diff md5 4b1f062d497bfb6cfbbfff004e6487c9 ($S/post_b/wt.diff) + untracked const_site.rs 96fac530cf0ed01f580183afe6187bdd,
 unique_const_fn.rs d314fdcaf3e0ea57f8ad288adfa21338
P3-1: 444 cells: 48 moved (all == (a)-POST text; set == P3 list), 396 identical ($S/post_b/p3_item1_444.txt, p3_item1_vs_aPOST.txt)
P3-2: mechanism-only (step 2) 444/444 identical
P3-3: G6 71 lanes: POST fu == PRE fp in 68; g12f (D3), g13a/g13b (+1 run-time W4031 = verilator) ($S/post_b/p3_item3_g6.txt)
P3-4: 0 case-form cell changed; probe/p2 (plain case) PRE == POST (E3009 x3)
P4 backends: 75 cells (48 moved + 27 held/opt-out) one output across native/interp/vm ($S/post_b/p4_backends.txt)
P4 staged: 75/75 staged == one-shot (sorted value + W4031 + error lines) (p4_staged.txt)
P4 .velab: non-moved 256 identical / 0 differ; moved 10 differ (the silent-wrong->correct cells); 178 without a PRE+POST pair (p4_velab.txt)
P4 format_version: header 56 45 4c 41 42 00 00 00 23 on PRE and POST .velab (35)
P4 JIT: 75 cells jit == POST(b) native (0 mismatches); JITBODY activations>0 in 13/75 (p4_jit.txt)

# ROUND 2 (coordinator: r1 FAILED, 3 BLOCKING of one class; X1 foreign text, X2 4-state default)
## X1(c) census — cells $S/x1/<site>_{u,p}.sv (u: the SHADOW function in the caller scope has the `unique if` miss), table $S/x1/census_final.txt
plain twin p on PRE vs both oracles (iverilog on p, verilator on u and p):
 WRONG -> excluded:
  IA real binder header default `ia_hdr` (p: u[1] p=5, oracles a) / port range `ia_port` (same) / d01, d02
  package fn, run-time inline: return range `pk_rt_ret` v=232 (8), formal `pk_rt_fml` 232 (8), local `pk_rt_loc` 232 (8),
   cast `pk_rt_cast` -24 (0), replication `pk_rt_rep` 7f (7), [m:l] `pk_rt_psel` cd (d)
  package fn, frame lane local `pk_fr_loc` 232 (8)
  package fn, interpreter: return range `pk_ce_ret` P=232 (8), default `pk_ce_dflt` P=1007 (ivl 1003; vl refuses)
  width query `$bits(q::h(0))` `pk_wq` B=8 (4); imported fn `imp_rt_ret` 232, `imp_ce_ret` 232, two packages `imp_two_pkgs` 232 (8)
  generate-scoped routine shadowing the module's: `gen_fn` P=7 bw=8 (ivl 3/4; verilator refuses a constant function under generate)
 WRONG but unreachable by the arm (no exclusion needed): interpreter formal/local ranges `pk_ce_fml`, `pk_ce_loc` P=1000 (8):
  const_decl_wsign declines every bound that holds a call (const_fn_width.rs:1020) -> width unknown -> unmasked; u PRE == POST
 RIGHT -> stays opted in: interpreter body statements of a package fn `pk_ce_body` 1003, `pk_ce_cast` 0; package param range
  `pk_param` b=8 (u: PRE loud -> POST b=8 = oracles); interface `if_rt_ret` 8, `if_own`, `if_port` (vl); hierarchical `hier_rt` 8;
  regular instance `inst_hdr`; IA body `ia_body`; bind `bind_hdr` W=3 (vl), `bind_port` (vl); run-time default `pk_rt_dflt` 1003
 LOUD in both / no data: `cls_rt` (E2002 `q::C c`), `cls_top`, `cls_mod` (E3009 class method return range), `hier_wq`/`if_wq`
  ($bits of hierarchical call unsupported), `gen_fn_ret` (p E3009), `gen_hier` (hierarchical g.h unsupported), `if_ce`
  (hier call in parameter; vl refuses too), `pk_task_loc`/`pk_task_susp` (E2002 `q::tk(...)`)
 bits_prescan: own scope (G6 g05u/g06j right; g05q oracles split). GenPhase: covered by gen_fn/gen_fn_ret/gen_hier.
## implementation (structural keys)
 X1a: instance-array element: `ia_element_next` set by the IA walk, taken by `elaborate_instance` -> `inst_is_ia_element`;
  `bind_params` and `elaborate_ports` of that element run `const_held` (same window as the prepass).
 X1b + X1-gen: `const_required(text, f)` now takes the folded text's span; `text_foreign(span)` = span inside a package
  declaration (unless that package is being elaborated, `cur_prefix == $pkg$<p>`, or the interpreter runs that package's
  routine, `const_call_pkg == p`) OR inside a generate level that declares a function/task (spans collected once in `run`,
  `foreign_text_spans`). Foreign text folds `Held` even inside a stated fold. Interpreter defaults: `const_fold_text(d.span)`.
 X2: `const_arm_hit` / `const_x_read` Cells saved+reset at the outermost stated fold, checked at its exit; both set -> f runs
  again `Held`. 4-state "not fully assigned" = a NUL-suffixed key in the frame env, set (only while Required) when the return
  var is created (`!ret_two_state`) or a 4-state local is declared without a value; cleared by a whole-variable assignment
  (not by a select write) or a re-declaration; reads checked at eval_const_env Ident, const_placement_wide resolver, return at exit.
 dN4: consumer = W5 `lower_const_width_expr` at expr_main.rs (a string LITERAL replicates on the packed path) + the width-region
  twin expr_size_ctx.rs Replicate; both now keep the unstated fold when a replication value is a string literal or string value.
## X2 cells $S/x2 (table census_final.txt): u-form declines (= PRE) in x2a ret4, x2c local, x2f callee arm + caller x, x2g
 sibling call x + arm, x2i integer, x2k range, x2l bit-select partial write; folds (= oracles) in x2b int return 0, x2e assigned
 before read, x2j bit local; x2h no arm: unchanged; every plain twin unchanged.
## regression on s8 (md5 below): 444: 48 moved (= (a)-POST, = P3 list), 396 identical; G6 71: 68 POSTfu==PREfp, g12f/g13a/g13b
 as before, 0 plain changed; lens+census+t (217 cells): 0 plain-twin diffs; u diffs listed in $S/post_c (pending)
## round-2 mutation battery (expected written before running; full workspace each; baseline 9067 pass / 15 skip, $S/logs_impl/r2_ws.log)
| R1 | IA element hold off (`ia_element_next = false`) | instance_array_element_header_and_port_stay_loud |
| R2 | text_foreign ignores package spans | package_routine_text_stays_loud |
| R3 | text_foreign ignores routine-declaring generate levels | generate_block_with_a_routine_stays_loud |
| R4 | X2 rerun removed | unassigned_four_state_read_stays_loud |
| R5 | X2 flags never reset / restored | unassigned_four_state_read_stays_loud (x2m_order: Q's arm leaks into P) |
| R6 | string-literal replication count stated again (dN4 off) | string_literal_replication_keeps_its_count_fold |
| R7 | interpreter default folded in the caller's mode | package_routine_text_stays_loud (pk_ce_dflt) |
| R8 | whole-variable assignment no longer clears the 4-state mark | unassigned_four_state_read_stays_loud (folded controls x2e/x2j... x2e) , params/other folds of logic returns |
results: R1 killed (instance_array_element_header_and_port_stay_loud); R2 killed (package_routine_text_stays_loud); R3 killed
 (generate_block_with_a_routine_stays_loud); R4 killed (unassigned_four_state_read_stays_loud); R5 first SURVIVED (9067 pass,
 $S/mut2/R5_first.log): in the x2m order a leaked ARM fact only re-runs the no-arm fold `Held`, which folds to the same value;
 added x2n_order2 (x-read fold BEFORE the arm fold) -> R5 killed (unassigned_four_state_read_stays_loud); R6 killed
 (string_literal_replication_keeps_its_count_fold); R7 killed (package_routine_text_stays_loud, pk_ce_dflt); R8 killed
 (params_fold_silently, unassigned_four_state_read_stays_loud). Tree restored: status same, 31 snapshot files cmp-identical.
## round-2 final gates (final tree): fmt --check rc=0; clippy --workspace --all-targets -D warnings rc=0 (logs_impl/r2f_clippy.log);
 unique_const_fn 19/19 rc=0; unique_if_chain 20/20 rc=0; -p cli 8043 pass 1 skip rc=0 (r2f_cli.log); workspace (before x2n pin) 9067/15 rc=0
## post_c: vita fcae6604862198c224d5baee33948ec2 7371072, vcmp 01016c9c..., velab e275a1ad..., vrun e86d3174..., vita_jit f1cd245c... 9214448
 built from git diff md5 6103b1f20eb5ac63f36e474a927bf467 + const_site.rs b6a8cc9f..., unique_const_fn.rs b9efb948...
 444: 48 moved (= (a)-POST = P3 list), 396 identical; == post_b on all 444. G6: 68/71 POSTfu==PREfp (g12f, g13a, g13b as before), 0 plain changed.
 lens+census+test cells 262: 0 plain-twin changed; 33 moved back to PRE; 52 still moved (51 identical to post_b, d10 differs: PS folds, PX/PI decline).
 lanes: 75 census cells + 85 lens/census moved/back cells: one output across backends, staged == one-shot, JIT == native (JIT fired 13 + 18).
 .velab: 256 non-moved identical, 10 moved differ; header 35. probe/p2 PRE == POST.
