# PLAN §4.5.587 unique-const-fn

status: COMPLETE (planner)

Intent: a constant function that reaches the parser-synthesized `unique` / `priority if` no-match arm must fold
silently where IEEE requires a constant (both oracles do), and must keep its PRE behavior (run-time call that
reports, or honest E3009) everywhere vita folds a run-time position — so the arm becomes a no-op only under an
opt-in mode set at constant-required consumers, never a global no-op.

## P1 verdict — NARROW (go on shape (b), restricted to measured constant-required groups; not a prerequisite row)

Reasons:
1. Shape (a) (global no-op) is measured BLOCKED: 8 loud→silent (repeat ×5, `#(…ns)`, `assign #`, `wire #`) + 9
   loud→value-minus-report (G5). ER §2.2 "never widen a loud into a silent".
2. Shape (b) needs no prerequisite: the mode is a new Cell beside `const_call_fn`; no IR/sim-ir/format change
   (header 35 unchanged, G3); every consumer not opted in passes no mode and stays byte-identical (ER §10.2
   "opted out (a parameter the lane does not pass, byte-identical)").
3. Lane rule (LOOPROMPT §2 last line, ER §10.2): the opt-in set is cut to groups with measured cells (G1/G2,
   48 cells moved = oracle under (a)); unmeasured constant-required groups, the override/defparam channel,
   covergroup bins, package parameters and every run-time position stay opted out. Wrapper-level opt-ins
   (W1–W6 below) reach a few callers with no cell yet: they are the G6 "to measure" list, measured on PRE
   (plain-if twin + unique-if + both oracles) BEFORE code is written; a G6 lane whose plain-if twin is wrong
   on PRE is dropped from the opt-in (its consumer keeps PRE), never patched in this slice.
4. Equivalence argument that makes every opted-in lane measurable on PRE: under `Required` the arm returns
   `Some(ConstFlow::Normal)`, exactly what the absent `else` of a plain `if` returns, so at any opted-in
   consumer POST(fu: `unique if`, miss) == PRE(fp: plain `if`), except the `steps` counter (+1 per reached
   arm; only changes a fold within one step of MAX_STEPS=100,000, and only to a decline = loud). Under
   `Unstated` the guard is false → catch-all `_ => None` → PRE byte-identical.
5. Row re-scope: the `case` half cannot move with any arm (exec_const_stmt has no `Stmt::Case` arm; plain
   `case` is E3009 too, probe p2) → new row `const-fn-case` (P5). This slice closes the `if` half
   (`unique if` / `priority if`, no `else`) at constant-required positions.

## P2 fix shape — (b): a positive opt-in mode chosen at the consumer

Rejected: (a) global no-op (measured BLOCKED, P1.1); (c) opt-out list at run-time consumers (fails to silent
on any run-time consumer the census missed and on every future consumer; ER §5.5 "key on the POSITIVE set",
§5.1 "shared-machinery semantics only opt-in"; memories shared-machinery-semantics-opt-in,
positive-set-and-list-level-rules, shared-fold-arm-opens-every-lane).

### Mechanism (exact)
- New file `crates/elaborate/src/const_site.rs` (keeps const_fn.rs, already 1,722 lines, to a 4-line arm):
  ```rust
  /// Which kind of position asked the constant interpreter for a value.
  #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
  pub(crate) enum ConstSite {
      /// The consumer has not stated that IEEE requires a constant here: a run-time
      /// position vita folds as an optimisation (a `repeat` count, a delay) or requires
      /// constant itself. A reached simulator report declines the fold (PRE behaviour).
      #[default]
      Unstated,
      /// IEEE 1800-2017 requires a constant expression here (grammar `constant_*`).
      Required,
  }
  impl Elaborator<'_> {
      /// Run `f` as a fold for a constant-required position. `&Self` on purpose:
      /// nothing that lowers (`&mut self`) can run while the mode is set, so no
      /// run-time consumer can inherit it.
      pub(crate) fn const_required<T>(&self, f: impl FnOnce(&Self) -> T) -> T {
          let saved = self.const_site.replace(ConstSite::Required);
          let r = f(self);
          self.const_site.set(saved);
          r
      }
      /// `[N]` unpacked size (IEEE §7.4.2 `[ constant_expression ]`), the twin of
      /// `const_range_bound_fold` for `[m:l]`.
      pub(crate) fn const_dim_size_fold(&self, e: &ast::Expr) -> Option<i64> {
          self.const_required(|s| s.const_eval_in_scope(e))
      }
  }
  ```
- `crates/elaborate/src/lib.rs`: `mod const_site;` and field `const_site: std::cell::Cell<const_site::ConstSite>`
  beside `const_call_fn` (lib.rs:510), doc pointing at const_site.rs. `crates/elaborate/src/driver.rs:59` (the only
  Elaborator constructor that sets `const_call_fn`): `const_site: std::cell::Cell::new(Default::default())`.
- `crates/elaborate/src/const_fn.rs` `exec_const_stmt`, before the catch-all at :1718:
  ```rust
  ast::Stmt::SysTaskCall { name, .. }
      if name.name == ast::UNIQUE_VIOLATION_TASK
          && self.const_site.get() == crate::const_site::ConstSite::Required =>
  {
      Some(ConstFlow::Normal)
  }
  ```
  Update the catch-all comment (:1718, "system-task other than the arm above") and the doc at :1528-1533.
- How the mode reaches the arm through the three `eval_const_call` callers (const_fn.rs:514/:520 in
  `const_eval_in_scope`, :1020 in `eval_const_env`): no signature changes. Every consumer fold entry
  (`const_eval_in_scope`, `const_int_selfdet`, `const_bound_u32`, `eval_const_assign`, `const_range_bound_fold`,
  `cast_size_bits`, `const_truth_in_scope`, `const_str_in_scope`, the param helpers) ends in one of those three
  callers → `eval_const_call` → `exec_const_stmt`, which reads the Cell. Nested calls (argument folds, callee
  bodies, defaults) inherit the mode: inside a constant-required expression every sub-position is constant too.
  The Cell is set only by `const_required`, restored on return; `?`/early returns inside `f` cannot skip the
  restore (it follows `f(self)`); no `catch_unwind` exists in elaborate or cli/src (grepped), so a panic cannot
  resume elaboration with a leaked mode.
- The reached arm under `Required`: silent no-op, `unique if` and `priority if` alike. Basis: both oracles fold
  silently and print no report (G1: verilator P=7 for `unique if` M and `priority if` M; iverilog rejects both
  `if` forms, folds the `case` twins silently); IEEE 1800-2017 §13.4.3 item (i) "all system task calls within a
  constant function shall be ignored" (recalled; applies by analogy — a §12.4.2 violation report is a simulator
  report, deferred to an Observed region that elaboration does not have). `unique0`/`priority0` have no arm
  (already fold, unchanged). User source cannot write `$__vita_unique_violation` (hdl-parser expr.rs:660 rejects
  the `$__vita_` namespace), so the arm sees only the synthesized task.

### Opt-in list (ConstSite::Required) — 6 wrapper-level edits, 7 `const_dim_size_fold` call sites, ~40 call-site wraps
Wrapper-level = position-exact helpers: every caller, nested or not, is the same IEEE constant position.
| id | edit (file:line) | IEEE 1800-2017 constant position | census rows covered | measured cells |
|---|---|---|---|---|
| W1 | const_select.rs:668 `const_range_bound_fold` body | declared range bound, A.2.5 `constant_range` (§7.4 dims, port, return, param, enum base, instance array) | array_geom.rs:308 (range half), :359(+:376,:433), :568, :837/:855, packed.rs:188, netdecl.rs:745, :764, ports.rs:983, inline_task_locals.rs:175, net_util.rs:335, :493, :765, params.rs:303, :860, :1694, inline_fn.rs:120, instance_array.rs:40, :121, :135, array_formal.rs:386, const_eval.rs:696, const_fn_width.rs:1031 (guarded, n/a), range halves of array_geom.rs:262, netdecl.rs:209/:233, string_array_route.rs:108/:357 | b_pr, b_pw, e_td, e_fret, e_fform; rest = G6 |
| W2 | new `const_dim_size_fold`, called at net_util.rs:815, array_geom.rs:262, :320, :845, array_formal.rs:318, string_array_route.rs:103, :353 | unpacked `[N]`, §7.4.2 `[ constant_expression ]` | net_util.rs:815, array_geom.rs:262, :320, :845, array_formal.rs:318, size halves of netdecl.rs:209/:233, string_array_route.rs:212, var_init.rs:74, block_local/hoist.rs:120, frames_reserve.rs:376 | b_ur (`$size` reads :845), e_barr; rest = G6 |
| W3 | const_fn.rs:856 `cast_size_bits` body | size-cast SIZE, `constant_primary ' ( expression )` §6.24.1 | expr_cast.rs:90, expr_size_ctx.rs:636, params.rs:766, const_fn_width.rs:285 (+ nested const_fn.rs:715/:789/:822, const_eval.rs:567, const_fn_width.rs:375) | c_cast |
| W4 | const_real.rs:38 `const_truth_in_scope` body (2 callers) | generate-if / generate-for condition, `if ( constant_expression )`, `genvar_expression` | generate.rs:307, :596 | b_gi, b_gf |
| W5 | const_bound.rs:190 only the `self.const_bound_u32(e)` call in `lower_const_width_expr` (NOT :186 `lower_index_expr`) | replication multiplier (A.8.1 footnote: non-string multiplier shall be constant), `+:` width `constant_expression` §11.5.1 | expr_main.rs:846, :882, :1050, lvalue.rs:484, :503 | c_rep, c_rep2, c_rep3, e_pswr, e_psww; hier :846/:484 = G6 |
| W6 | packed.rs:1889 the `(const_bound_u32(msb), const_bound_u32(lsb))` pair in `width_from_msb_lsb_dir` | part-select `[m:l]`, `part_select_range ::= constant_range` §11.5.1 | packed.rs:1890 (callers expr_main.rs:779, lvalue.rs:451, packed.rs:1826 hier) | h_prd, h_pwr, h_plsb; hier = G6 |

Call-site edits (wrap the `&self` fold call in `self.const_required(|s| …)`; adjacent `.or_else` links share
one closure):
| group | edit sites | IEEE position | census rows | measured cells |
|---|---|---|---|---|
| module-body param binder | instance.rs:677 `param_str_or_folded`, :680 `param_real_value`, :706 `param_decl_width_unoverridden`, :709-716 the fold chain (`untyped_fill_init` … `param_i64_at_declared`), :721 `wide_disagreeing_value` (:672 `check_param_decl_range` is covered by W1) | `constant_param_expression` | instance.rs:677, :680, :708, :710, :711, :712, :715, :721 | a_unique_if_M, a_priority_if_M, b_sf, b_nest, b_loop, b_pkx, b_pki |
| header / body-parameter binder `bind_one_param` (default lane only) | params.rs:1905, :1938-1939, :2098, :2206-2225 chain, :2403; and every other `param_decl_width*` call there that folds the declaration's own text (:2184, :2186, :2297). NOT :2263 `override_at_declared_width` (override channel, deferred) | `constant_param_expression` | params.rs:1905, :1939, :2206, :2210, :2211, :2217, :2225, :2403 | b_pp (header-less body `parameter` routes here, instance.rs:664); header default + interface = G6 |
| generate-scope param binder | generate.rs:754, :766, :774, :780-792 chain, :799, :835 (:773 covered by W1) | `constant_param_expression` | generate.rs:754, :766, :780, :782, :788, :789, :792, :799 | b_gl |
| interface body real param republish | iface_inst.rs:537 | same position as the interface parameter it republishes | iface_inst.rs:537 | G6 |
| enum label (module / generate typedef) | instance.rs:122, :132 (wrap the fold only, not the `self.error` in `unwrap_or_else`) | `enum_name_declaration … = constant_expression` §6.19 | instance.rs:122, :132 | b_en; generate typedef = G6 |
| generate | generate.rs:251 (for init), :358 (for step), :438 (case scrutinee), :459-462 (label, incl. the :460 `const_str_in_scope` fallback, one closure) | `genvar_initialization`, `genvar_iteration`, `case ( constant_expression )`, `case_generate_item ::= constant_expression …` §27.4/§27.5 | generate.rs:251, :358, :438, :459, :460 | b_gf, b_gc, h_gcl |
| streaming slice | stream_concat.rs:132 | `slice_size ::= … constant_expression` §11.4.14 | stream_concat.rs:132 | c_strm |
| elaboration task args | elab_task.rs:27, :68, :92, :95, :119 (each fold call; `elab_task_message` is `&mut`) | elaboration system task arguments, §20.11 (recalled: constant) | same 5 rows | c_elabt (verilator only; iverilog rejects a call there) |
| replication twins | expr_main.rs:975 (`count_lowers_real_param` call), :1081 (`const_bound_signed`), expr_size_ctx.rs:543 | replication multiplier | expr_main.rs:975, :1081, expr_size_ctx.rs:543 | c_rep*; region + negative count = G6 |
| select twins | expr_size_ctx.rs:567, :575, :579, packed.rs:2204, packed_inner.rs:152 | `constant_range` / `+:` width | same 5 rows | h_mdpr (route via :2204 to confirm in G6); :152, region = G6 |
Decline notes: where an opted-in consumer computes its refusal text right after the fold (generate.rs:254/:361/
:441 and the notes after :307/:596 via `unfoldable_note`, instance.rs:132's message), compute it in the same mode
(ER §2.7 "put the reachable cause first"); `param_value_unfoldable` (`&mut`) only if its reason is one `&self`
call, else record the wording residue in the commit message.

Census accounting (scripted over g2_static.tsv): 96 rows opted in, 10 nested, 82 opted out = 188.

### Nested rows (inherit the consumer's mode; no edit)
const_wide.rs:1329, const_bound.rs:230, :371, :566, :664, :792, const_fn_width.rs:285 (via W3), :1031 (guarded),
inline_fn.rs:120 (via W1), expr_cast.rs:145 / expr_main.rs:69 / dynarr.rs:929, :939 (no call reachable).

### Opt-out list (stay `Unstated`, PRE byte-identical) — every row here passes no mode
| class | rows | why |
|---|---|---|
| run-time, fold REPLACES a run-time call | stmt_flow.rs:1119 (`repeat_unroll_count` const_bound.rs:146/:159), frames_classify.rs:816, frames_reserve.rs:685 (same predicate, ONE SPELLING), events.rs:1065, netdecl.rs:876, var_init.rs:127 (via ca_delay_rt.rs:71) | IEEE §12.7.3 / §9.4.1 / structural delay evaluated at execution; measured loud→silent under (a) |
| run-time, loud on decline | events.rs:46, :135, :765, :835, packed.rs:2134, packed_inner.rs:183, expr_main.rs:612, :615, :1142 (+ expr_special.rs:652), lvalue.rs:304, crv.rs:270, :272, :291, :522, :571, :625, cover_synth.rs:194, :199, :202, block_local/hoist.rs:267, :650 | IEEE run-time positions; opting in drops the report verilator / iverilog print (measured: 9 cells) |
| mixed / run-time index | packed.rs:339 (`lower_index_expr`, 38 callers), packed_lval.rs:355 | index and `+:` offset are run time |
| instantiation-time | cover.rs:120 (`option.*`), cover_bins.rs:174, :175 (bin values) | hand-IEEE §19.5 (recalled): bin expressions may name covergroup formal arguments and are evaluated at construction; no oracle (verilator refuses calls in bins, iverilog cannot parse covergroups) → stays E3009 |
| census mislabel, run-time | expr_main.rs:934, stmt_main.rs:216 | string replication: A.8.1 footnote allows a non-constant multiplier for a string |
| deferred channel (ER §4.2: the override channel is its own row) | instance.rs:1244, :1284, :1302, :1303, :1475, :1476, :1486, :1491, :1492, :1493, :1540, :1545, :1547, :1631, :1648, :1649, iface_inst.rs:19, :20, :30, :37, :74, :76, :110, :126, expr_size_hier.rs:613, param_query.rs:578, override_type.rs:127, params.rs:2263 | override / defparam: 8 channels × positional/named × module/interface; b_ov/b_dp stay E3009 |
| masked | package.rs:609, :621, :636, :638, :642, :643, :646, :654, :739 | a package parameter calling its own package's function is E3009 even for `return a + 1` (planner probe pk_simple: iverilog and verilator `P=2`) — unmeasurable for this arm |
| unmeasured constant positions (residue) | net_util.rs:581, netdecl.rs:333, :446, expr_main.rs:996, packed.rs:2120, packed_inner.rs:176, packed_lval.rs:365, cover_bins.rs:112, :117, :129, :145, const_eval.rs:688 | constant by IEEE, no cell; stay PRE (loud or PRE default) until measured |

Covergroup bin decision: opt OUT (stays E3009, honest-loud). No oracle, hand-IEEE reading = instantiation-time,
not constant-required; re-read §19.5 before the commit message states it; pin it as a residue with that note.

## P3 moved-cell table (shape (b) as scoped in P2; derived from the measured (a) run, not yet built)

Measured set = 444 vita cells (g1a 40, g1b 76, g2 152, g2/fix 24, g2b 84, g2c 40, g2d 28). Shape (a) moved 71.
Under (b): 48 move, the other 23 (a)-movers stay at PRE, every other cell stays at PRE.
| direction | n | cells (all if-form M unless noted) | consumer that moves it |
|---|---|---|---|
| loud (E3009/E3010) → value = oracle, no report | 38 | a_unique_if_M, a_priority_if_M; b_{pp,sf,nest,loop,pkx,pki,gl}_{if,pif}_M, b_loop_if_N; b_{pr,pw}_{if,pif}_M, e_td, e_fret, e_fform; b_ur_{if,pif}_M, e_barr; b_{gi,gf,gc}_{if,pif}_M; b_en_{if,pif}_M; c_cast; c_strm; c_elabt | param binders; W1; W2; W4 + generate sites; instance.rs:122/:132; W3; stream_concat.rs:132; elab_task.rs |
| silent-wrong → correct (= both oracles, no report) | 10 | c_rep, c_rep2, c_rep3 (replication), e_pswr, e_psww (`+:` width), h_gcl (generate-case label), h_prd, h_pwr, h_mdpr, h_plsb (`[m:l]`) | W5; generate.rs:459; W6 / packed.rs:2204 |
| loud → silent | 0 | — | no run-time consumer is opted in |
| moved under (a), stay PRE under (b) | 23 | b_ov_{if,pif}_M, b_dp_{if,pif}_M (deferred override channel, E3009); e_cg (cover bin, E3009); c_rpt, e_trpt, e_ktrpt, e_ktrp2, e_frpt, h_tdly, h_cad, h_wd (keep their run-time W4031, as the oracles); c_sizd, e_qsizp, e_qhigh, e_qleft, e_sel3, k_inoff, e_irpt, e_nrpt, k_evlsb (keep E3009); k_evlvl (keeps PRE message) | opt-out list |
Notes: h_plsb keeps its vita-only W4031 at time 1 (the `[m:l]` LSB offset still lowers through packed.rs:339,
which is mixed and opted out; PRE prints it too) — value moves 0→1e, the report is pre-existing (goes to 🆕 AC's
text). Count 38 + 10 + 23 = 71 = (a)'s total.

Re-run obligations for the implementer (POST(b) release separate-bins binary vs $S/pre/sep/vita):
1. All 444 grounding cells, summaries diffed per cell: exactly the 48 above change, each to the (a) POST text
   ($S/*/post_summary.txt rows); the 23 held cells and all 373 others byte-identical to pres_summary.
2. Mechanism-only build (P6 step 2, no opt-in edits): all 444 byte-identical to PRE (non-vacuity of "default is
   Unstated").
3. The G6 cells (P6 step 1) on POST: each = its PRE plain-if twin = oracle.
4. Every case-form cell (G1a 32, G1b 17, probe p2) unchanged (E3009).

## P4 lane table (ER §10.2)

Shared function edited: `exec_const_stmt` (the interpreter; consumers = the 188 census rows). Routed into:
`const_range_bound_fold`, `cast_size_bits`, `const_truth_in_scope`, `lower_const_width_expr`,
`width_from_msb_lsb_dir` (wrapper-level) and the call sites of P2.
| lane | status | evidence / how |
|---|---|---|
| consumer lanes opted in (P2 tables) | measured, except the G6 list | (a)-POST cells per group = oracle (G1, G2 C/D, g2c/g2d); G6 measures the wrapper-level callers and the header/interface/generate-typedef/hier/region/multi-dim twins on PRE (plain-if twin = oracle ⇒ POST(unique) = oracle by the P1.4 equivalence), then on POST(b) |
| consumer lanes opted out (P2 opt-out table + every other census row) | opted out | no `Required` passed ⇒ arm guard false ⇒ catch-all ⇒ byte-identical; checked by P3 re-run item 1 and the mechanism-only build (item 2) |
| native / interp / vm (`--backend`) | measured under (a); re-measure on (b) | g3 harness: the arm lives in elaborate, every executor gets the same IR; (a) 14/14 cells one output across backends. On (b): the 48 moved + 4 opt-out residue cells × 3 backends, one output each |
| staged vcmp → velab → vrun (`--features separate-bins`) | measured under (a); re-measure on (b) | same cells, staged == one-shot after dropping per-stage `errors=` lines (g3 method) |
| .velab | measured under (a); re-measure on (b) | moved cells differ (expected); every non-moved cell `cmp`-identical PRE vs POST (g3 method, extended to all 444 cells' .velab where PRE elaborates) |
| format_version | measured: 35, stays 35 | header bytes `56 45 4c 41 42 00 00 00 23` PRE and POST; vita-artifact/src/header.rs:15; no sim-ir / hdl-ast type touched (ConstSite lives in elaborate, not SchemaHash-ed) |
| JIT (`--features jit`, `VITA_JIT=1`) | measured POST-only under (a); to measure on (b) | build POST(b) with `--features jit` in $S/target_s587_jit; moved set + opt-out residue cells == POST(b) native; record `JITBODY` firing count (non-vacuity, ER §3.2). PRE JIT not needed: elaborate-only change, the JIT reads the same IR |
| product (`--no-default-features`) | n/a by construction; gate still runs it | crates/elaborate, hdl-parser, hdl-ast have no `[features]` and 0 `cfg(feature` (G3 grep); cli's `cfg(feature = "oracle")` only selects backends; run CONTRIBUTING's product-shape commands in the full gate |

## P5 side-finding scope (row drafts are for the main session's docs commit; ROADMAP is not the implementer's)

(i) `case` half → new §3.b row, and `unique-const-fn` becomes a residue line.
- Draft: `const-fn-case — a constant function whose body holds any case / casez / casex / case … inside is E3009
  "f(…) has no constant-fold arm", every qualifier, reached or not (s587 probe p2: plain case g(2), iverilog and
  verilator P=20; G1a 32 + G1b 17 cells); elaborate/src/const_fn.rs exec_const_stmt has no Stmt::Case arm
  (catch-all _ => None); a Case arm that sizes the case expression and every item once at one width and sign
  (ER §2.4), casez/casex don't-care masks, inside per §12.5.4, first match wins; a unique / priority case's
  synthesized default then reaches the §4.5.587 arm (folds only under ConstSite::Required); opening it folds
  plain-case functions at every consumer, run-time replace consumers included (no report there), so census
  those consumers for case bodies that hold other effects; 2 oracles (verilator only for inside); OPEN`.
- ER §2.4 note for that row: the case-sizing rule is the row's main risk (self-width wraparound), not the arm.
- `unique-const-fn` residue draft: `after §4.5.587 a reached unique / priority if arm is a no-op only for
  consumers opted into ConstSite::Required; still E3009 where both oracles fold silently: the parameter-override
  and defparam channel (b_ov, b_dp: verilator W=7, iverilog case twin W=7), the constant positions not yet opted
  in (body-local enum label, queue bound, multi-dim packed +: width, nested packed lvalue [m:l], coverpoint
  sizing, a replication count reading a constant-array element, const array parameter capture), and every case
  form (BLOCKED on const-fn-case); covergroup bin values stay E3009 by hand-IEEE (§19.5, construction time);
  2 oracles; OPEN`.

(ii) Family E → new §2 start-order row 🆕 AC; a queue row, not a same-slice follow-up.
- LOOPROMPT §4 test: in the fix path (its sinks lower_const_width_expr, width_from_msb_lsb_dir, generate.rs:459
  are W5, W6 and a call site here), oracle cells yes (2 oracles), but not S-without-prerequisite: the upward fix
  (ER §2.5 "close a silent default upward by making the shape foldable") needs const-fn-case and a system-task
  arm (p6 holds a $display body and a plain case body), and a loud-at-sink fix descends on right-by-accident
  generate-case labels (every non-matching declined label is right today as "no match"). This slice widens no
  sink and fixes family E's 10 unique-if cells.
- Draft: `🆕 AC — a user call the constant interpreter declines, in a replication count, an indexed part-select
  width, a [m:l] bound or a generate-case label, becomes an empty replication / a 1-bit select / the default arm
  at exit 0 (c_rep_case_M r=00000000, e_pswr_case_M pw=1, h_prd_case_M pr=1, h_plsb_case_M pl=0, h_gcl_case_M
  default; p6: $display body and plain case body; iverilog and verilator r=0000007f pw=7f pr=ff pl=1e and the
  label; 19 cells + p6, PRE = POST §4.5.587); sinks const_bound.rs:185 lower_const_width_expr keeps the lowered
  call when const_bound_u32 declines and the engine's shallow fold reads unwrap_or(0)/(1); packed.rs:1890 /
  :2204 / packed_inner.rs:152; generate.rs:459 reads a non-folding label as no match; also the [m:l] LSB offset
  of a reached-arm call lowers as a run-time call and reports W4031 at time 1 where verilator is silent
  (h_plsb_if_M, packed.rs:339); upward fix = const-fn-case + a system-task arm, then re-measure what still
  declines; 2 oracles; OPEN`.
- §5.2 placement (main session judges): 🆕 AC is ① from the external report's fix path and should NOT precede
  this slice (already grounded; strictly improves family E). Recommend const-fn-case at row 2 and 🆕 AC at row 3,
  ahead of 🆕 AB: both close silent-wrongs at exit 0 (all 19 family-E cells are case-form, so const-fn-case
  moves them upward through this slice's sink opt-ins; p6's $display half needs const-fn-systask),
  while 🆕 AB is a false report with exit 1 (loud). If the owner order keeps 🆕 AB at row 2, put const-fn-case
  and 🆕 AC directly after it, const-fn-case first.

(iii) Triple call in `m[f(2)] = 4'hA` (e_sel2, probe p5: three calls, W4031 ×3, iverilog one call) and k_evlvl's
message → PROBE_CATALOG, one line each. Both are run-time positions this slice keeps at PRE (PRE = POST), so
outside the fix path. Triple call: cite ROADMAP §2 ":311 an operator over a call names it twice" and §3 ":478
inlining names the operand twice" as the possible family, unchecked. k_evlvl: the no-miss twin
(k_evlvl_if_N) on PRE names "a select of a dynamic-storage handle, a string, a subroutine's …" for a plain
vector select with a call index (ER §2.7 misdescribing refusal).

(iv) localparam-array t0 W4031 and the package-parameter E3009 → PROBE_CATALOG.
- b_pa: `localparam int A [2] = '{f(2),1}` is evaluated by a run-time frame call (run.json f:frame:sites=1 also
  for N) and reports W4031 at time 0 where verilator folds silently; iverilog `sorry: unpacked array parameters
  are not supported yet` (1 oracle + hand-IEEE); not the fold path (unmoved under (a)).
- b_pk: a package parameter whose initializer calls a function of the SAME package is E3009 `package parameter P
  value is not a foldable constant` even for `return a + 1` (planner probe $S/plan_probe/pk_simple.sv: iverilog
  and verilator `P=2`; pk_plain plain `if`: both `P=10 Q=7`); not caused by the arm. Flag to the main session:
  likely real-design demand (package-internal constants computed by package functions); promote to §3.b if a
  corpus design hits it.

(v) `unique-if-chain` (ROADMAP:547) blocker list: drop `unique-const-fn`; keep BLOCKED on §2 🆕 AB and
`unique-pkg-closure`; add the measurement the chain slice owes: "an armed function-body chain folds silently at
constant-required consumers after §4.5.587, keeps the run-time call (and its report) at replace consumers, and
turns a PRE value into E3009 at the run-time positions vita requires constant (intra-assignment repeat, array
query dimension, multi-dim +: offset, edge / level event select, string-array index, constraint / dist operand) —
measure those before arming". Same premise update in hdl-parser/src/lib.rs:894-895 (`first_if_arm_only` doc),
crates/cli/tests/unique_if_chain.rs:18-28, :578-581, :676-684, manual 006 §1.4 (lines 185-192) and
docs/REMAINING_WORK.md:12 (this slice's code-comment / manual part is the implementer's; ROADMAP / REMAINING_WORK
the main session's).

(vi) Extra (G4): `$display` / `$info` in a constant function is E3009 where both oracles fold silently and print
nothing (verilator prints `$warning` at build and fails the build on `$error`) → new §3.b row `const-fn-systask`
(IEEE §13.4.3(i); reuses ConstSite::Required, opting in the same consumers; split on $warning / $error / $fatal;
2 oracles for $display / $info; OPEN). Not this slice (a second arm with its own semantics).

## P6 implementation steps (opus-build, xhigh; worktree)

Hard rules for the brief (memory implementer-brief-carries-pre-binary-rule):
- PRE = `$S/pre/vita` (md5 e1e7e57148bb83ad35d2c4f68247a290, release, default features) and `$S/pre/sep/`
  {vita d0e1cf9d…, vcmp, velab, vrun} (separate-bins). Never rebuild PRE, never `git stash`, never build in or
  edit the main checkout; PRE is these files.
- Worktree: `git -C /Users/seongwookjang/project/git/vitamin-rtl-simulator worktree add $S/wt -b s587 7b33a009`;
  every cargo command with `CARGO_TARGET_DIR=$S/target_s587` (JIT build: `$S/target_s587_jit`). Prefix shells
  with `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`. One cargo command at a time; never SIGKILL an
  in-build nextest; no `cargo clean` / `cargo sweep`; no commit (main session commits); ROADMAP / PROBE_CATALOG /
  REMAINING_WORK / LOOPROMPT untouched.
- Return `git -C $S/wt status --short` and `git -C $S/wt diff --stat` with the report.

Step 1 — G6 lane measurement before any code (verify: table cell × {PRE fu, PRE fp, iverilog, verilator}; each
cell holds a `unique if` function fu and a plain-`if` twin fp printed side by side; harness $S/run3.sh):
G6-1 header default `module sub #(parameter int W = f(2))` (module function; instantiated with no override) and
`module top #(parameter int P = f(2))`; G6-2 interface header and body `parameter int P = f(2)`, and
`parameter real R = f(2)` (iface_inst.rs:537); G6-3 untyped `localparam P = f(2)` with `function logic [3:0] f`
printing `$bits(P)`, typed `localparam logic [7:0] P = f(2)`, `localparam real R = f(2)`; G6-4 generate-scope
`typedef enum int {A = f(2), B}`; G6-5 W1 callers: `sub u [f(2):0] ()` (+ a child port range), unpacked-array
formal `input int a [0:f(2)]`, `string s [0:f(2)]` with `'{…}`, `localparam logic [f(2):0] P = '1`,
`typedef enum logic [f(2):0] {A, B}`, descending `logic u [f(2):0]` dumped to VCD (`$var` range label), negative
LSB `logic [f(2):-2] v`, `localparam int A [0:f(2)] = '{default: 1}`, a ranged local in an inlined task and in a
suspendable task; G6-6 W2 callers: `$bits(u)` of `logic [3:0] u [f(2)]`, `$left/$right/$increment(u)`,
`string s [f(2)]`, formal `input int a [f(2)]`; G6-7 W3 nested: `localparam int P = f(2)'(9'h1FF)`,
`$bits(f(2)'(x))`, `repeat (f(2)'(4'hF)) n++;` (cast size inside a run-time count: oracle reports nothing);
G6-8 hier `dut.v[0 +: f(2)]` and `dut.v[f(2):0]`, read and write; G6-9 width-region twins where the region width
decides the value (`logic [7:0] r = ({f(2){1'b1}} + 8'h81) >> 1;`, same for `+:` width, `[m:l]`, hier `[m:l]`);
G6-10 negative count `{f(2)-8{1'b1}}`; G6-11 `m[f(2):0]` on `logic [15:0][3:0]` (confirm by code reading or a probe which of
packed.rs:2204 / :1890 answers it) and `m3[1][f(2):0]` on `logic [3:0][7:0][3:0]`; G6-12 `$info("%s", …)` string arg, an interface
elaboration task, `$fatal(f(2)-7, …)` first argument; G6-13 the same fu in `localparam` and in `repeat (fu(2))`
in one module, both source orders.
Gate: a G6 lane whose PRE fp ≠ oracle is dropped from the opt-in (switch that wrapper caller to an un-opted
spelling or leave the call site unwrapped) and reported; nothing is patched for it in this slice.

Step 2 — mechanism only (P2 "Mechanism"): const_site.rs, lib.rs field + `mod`, driver.rs:59 init, the arm before
const_fn.rs:1718, the catch-all comment and the doc at const_fn.rs:1528-1533. No opt-in yet.
Verify: release separate-bins build; all 444 grounding cells byte-identical to PRE summaries (proves the
default is Unstated and the Cell alone changes nothing).

Step 3 — wrapper-level opt-ins W1–W6 (P2 table), each with a one-line doc naming its IEEE position.
Verify: 444 cells — only declaration / cast / generate-condition / count / width / bound cells move.

Step 4 — call-site opt-ins (P2 call-site table), adjacent `.or_else` links in one closure, decline notes in the
same mode. List every `self.<fold helper>(` call inside the three param binders' default lanes (instance.rs
670-735, params.rs `bind_one_param` 1747-2420, generate.rs 750-845) and wrap each that folds the declaration's
own text; leave params.rs:2263 and every override / defparam / package / opt-out row untouched.
Verify: 444 cells = P3 (48 move to the (a)-POST text, 23 + 373 byte-identical); G6 cells on POST = PRE fp.

Step 5 — comments and user docs (ER §10.3; re-read every sentence of a comment block opened):
const_site.rs module doc (rule, positive set, why the default declines, the `&Self` guarantee, the W1–W6 list);
exec_const_stmt doc; hdl-parser/src/lib.rs:894-895 (`first_if_arm_only` bullet 1: the interpreter now folds an
armed tail only at constant-required consumers; run-time positions vita requires constant would refuse, replace
consumers keep the run-time call; case bodies never fold, ROADMAP §3.b const-fn-case);
crates/cli/tests/unique_if_chain.rs:18-28, :578-581, :676-684 (prose only, assertions unchanged);
docs/manual/006_limitations.md §1.4 (lines 185-192: replace the first held-back reason; add: a constant function
whose `unique` / `priority if` misses folds silently where a constant is required — parameter, declared range,
generate condition or label, size cast, replication count, part-select width or bound, streaming slice,
elaboration-task argument, enum label — as both tools do; at a run-time position the call runs and reports; a
parameter override, `defparam`, covergroup bin and package parameter that reach the miss, and every `case` in a
constant function, stay `VITA-E3009`); docs/manual/003_language-reference.md:763 (one clause, pointer to §1.4);
CHANGELOG.md [Unreleased] "### Fixed — a `unique if` no-match in a constant function folds where a constant is
required" (what changed for a user, the positions, what stays E3009, `unique0` unchanged).

Step 6 — tests: new crates/cli/tests/unique_const_fn.rs (`cargo fmt --all` first). Module header: iverilog 13.0
`-g2012` rejects `unique if` / `priority if`, so the value oracle is verilator 5.052 `--binary --timing --assert`
plus the plain-`if` twin on both tools and the in-design equivalence fu == fp; covergroup and `priority0` have no
oracle (hand-IEEE). Each pin: raw oracle lines in a doc comment above it, exact values asserted, W4031 count
asserted, exit code asserted.
- T1 params_fold_silently (module localparam, header-less body `parameter`, header default, generate-block
  localparam, `$clog2(f(2))`, nested callee, loop, `pk::f(2)`, `import pk::*`, untyped `$bits(P)`, `priority if`,
  `unique0` / `priority0` controls).
- T2 declared_ranges_fold_silently (packed `$bits`, port, typedef, return, formal, `[N]` `$size` + `$left` /
  `$right`, `[m:l]` unpacked, block-local `int arr [f(2)]`).
- T3 generate_constants_fold_silently (if, for bound, case scrutinee, case label — assert the labelled arm runs).
- T4 widths_counts_bounds_fold (`{f(2){1'b1}}` procedural and continuous, `{f(2){2'b01}}`, `v[0 +: f(2)]` read /
  write, `v[f(2):0]` read / write, `m[f(2):0]`, `v[11:f(2)]` = 1e with its one vita-only W4031 pinned as a
  residue naming 🆕 AC, negative count per G6-10).
- T5 cast_stream_elab_enum_fold (`f(2)'(x)`, `{<< f(2) {x}}`, generate `$info` → `I3006 … el=7`, enum `A = f(2)`).
- T6 runtime_positions_keep_the_report (`repeat (f(2)) n++` → W4031 at time 1 once, n=7; task `repeat (f(2))
  @(posedge clk)` → time 0; `#(f(2) * 1ns)`; `assign #(f(2)) w = a;` → W4031 lines exactly as PRE).
- T7 one_function_two_positions (G6-13, both orders: P=7 silent and exactly one run-time W4031).
- T8 runtime_positions_vita_requires_constant_stay_loud (`REFUSED`: intra-assignment `repeat (f(2))`,
  `$size(a2, f(2)-6)`, `m[f(2) +: 2]` multi-dim; each pin names its gate's message, oracle value + report text).
- T9 deferred_channels_stay_loud (`REFUSED`: `#(.W(f(2)))`, `defparam u.W = f(2)`, covergroup bin `{f(2)}` with
  the hand-IEEE §19.5 note).
- T10 case_and_systask_bodies_stay_loud (`REFUSED`: `unique case` M and N, plain `case` control, `$display` in
  the body; oracle P values; name const-fn-case / const-fn-systask).

Step 7 — mutation battery (write the expected killer first; each mutant `cargo nextest run --workspace --locked
--no-fail-fast` in the worktree; byte-snapshot restore under `#!/bin/bash` with `trap restore EXIT`; foreground;
results flushed per line; kills = FAIL / TRY 1 FAIL / TIMEOUT / SIGSEGV / SIGABRT / ABORT / LEAK-FAIL):
| id | mutant | expected killer |
|---|---|---|
| M1 | delete the arm | T1–T5 |
| M2 | drop `&& … == Required` from the guard (= shape (a)) | T6, T7 |
| M3 | `const_required` sets `Unstated` | T1–T5 |
| M4 | `const_required` never restores | T7 (localparam-first order) |
| M7 | unwrap generate.rs:459 | T3 label pin |
| M8 | array_geom.rs:845 back to plain `const_eval_in_scope` | T2 `$left`/`$right`/`$size` pin (survival ⇒ add a discriminating cell before review) |
| M10 | wrap stmt_flow.rs:1119's `repeat_unroll_count` call in `const_required` | T6, T7 |
| M12 | arm matches any `SysTaskCall` | T10 `$display` pin |
Optional if time allows: M5 (W1 removed → T2), M6 (W5 removed → T4), M9 (instance.rs:706 unwrapped → T1 untyped
`$bits(P)`), M11 (wrap events.rs:46 → T8). Record survivors with the "why it did not die" sentence (ER §7.4).

Step 8 — scoped gates (all `--locked`, logs to files, exit codes captured): `cargo fmt --all -- --check`;
`cargo nextest run -p cli --test unique_const_fn`; `cargo nextest run -p cli --test unique_if_chain`;
`cargo nextest run -p cli` (shared fold touched ⇒ whole -p cli, LOOPROMPT §5); `cargo clippy --workspace
--all-targets --locked -- -D warnings`. The full gate, doctests, product-shape axis and corpus run are the main
session's, once, before the commit.

Step 9 — freeze for review: release separate-bins POST(b) in $S/target_s587, copy to $S/post_b/ with md5s; JIT
build to $S/post_b/vita_jit; re-run P3 items 1-4 and P4 (backends, staged, .velab cmp, JIT) into
$S/post_b/*.txt; report the moved-cell table, the G6 table, the mutation table and the gate log paths.

## P7 review targets (two lenses; frozen POST(b) from step 9 vs PRE $S/pre/sep/*; brief opens "attack outside this table")

Soundness lens — attack first:
1. Opt-in list completeness against $S/g2_static.tsv: every REQ row is opted in with its whole position group
   or sits in the P2 opt-out table with a reason; every RT / MIXED / INST row is opted out; `grep -n
   "const_required(\|const_dim_size_fold("` equals the P2 list; no `const_site.set` / `.replace` outside
   const_site.rs.
2. A consumer constant in one call path and run-time in another: the callers of `const_bound_u32` (packed.rs:339
   mixed; :2120 vs :2134 in one function), `const_eval_in_scope` helpers shared by both kinds (`fixed_dim_bounds`
   also feeds block-local definite assignment; `const_str_in_scope`), and every caller of the wrapper-level
   opt-ins: `const_range_bound_fold` (29), `cast_size_bits` (9, some nested in run-time folds),
   `const_truth_in_scope` (2), `lower_const_width_expr` (5), `width_from_msb_lsb_dir` (3 + hier) — position by
   IEEE grammar for each.
3. Wrappers and tables that keep a fold result across contexts: `bits_prescan`, `params` / `hier_params` /
   `wide_param_bits` / `str_param_raw` / `real_param_val`, `pkg_consts`, generate arm decisions per GenPhase,
   const-array captures, enum label maps, `selfw_cache`: show each stores only constant-position results and no
   run-time consumer reads a Required-mode result in place of a run-time call; `repeat_unroll_count`'s ONE
   SPELLING (stmt_flow.rs:1119, frames_classify.rs:816, frames_reserve.rs:685) all Unstated.
4. Mode scope: `const_required` takes `&Self` (nothing lowers inside); W5 / W6 wrap only the fold, never
   `lower_index_expr`; restore on every exit; Required may nest inside Unstated (W1 / W3 inside a run-time fold:
   IEEE-exact), never Unstated inside Required.
5. The arm: producer census of `UNIQUE_VIOLATION_TASK` (hdl-parser assertions.rs:493 only; user spelling refused
   at expr.rs:660); `Normal` equals the plain-if path; the steps edge; `priority` = `unique`.
6. Stale premises: every comment / doc saying the interpreter refuses an armed tail (lib.rs `first_if_arm_only`,
   assertions.rs:468, unique_if_chain.rs, manual 006 §1.4, REMAINING_WORK §D).

Differential lens — attack first:
1. P3 on the frozen binary, three-way: 48 move to oracle text, 23 + 373 PRE byte-identical; the 8 replace cells
   keep their W4031 lines (count, time, location) exactly.
2. Twin consistency inside one opted-in position: one design reading every consumer of it — `$bits`, `$size`,
   `$left` / `$right` / `$increment` / `$dimensions`, VCD `$var` range, a surrounding width region, an untyped
   parameter's `$bits`, hierarchical reads (`dut.P`, `dut.v[f(2):0]`), a net declared after a nested generate.
3. Constant and run-time in one design: the same function in a localparam and a `repeat` / delay / CA delay;
   W1 / W3 nested in run-time counts (`repeat ($bits(logic [f(2):0]))`, `repeat (f(2)'(x))`, a body-local ranged
   by a miss function inside a function used as a `repeat` count) — verilator is the elaboration-time oracle.
4. Outside the table: positional and interface overrides, `defparam` onto a header parameter, `-G`, class
   parameters, `let` with a call, `parameter type` ranged by a call, `$bits(T)` of a call-ranged type, an imported
   package function in a generate-block parameter, `priority if` inside a nested callee of a generate condition,
   a `unique if` miss reached only on a later loop iteration, a miss function called twice in one parameter.
