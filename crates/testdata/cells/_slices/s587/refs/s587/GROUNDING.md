# GROUNDING §4.5.587 unique-const-fn

status: COMPLETE (G0-G5); all cells measured on PRE e1e7e571 / d0e1cf9d and POST prototype eac2ebb6

## G0 PRE freeze
- HEAD 7b33a009 (main, clean but `?? .DS_Store`). `cargo build -p cli --release --locked` rc=0 ("Finished ... in 0.06s": tree already built from HEAD sources).
- $S/pre/vita md5=e1e7e57148bb83ad35d2c4f68247a290 size=7338016 profile=release (default features)
- `cargo build -p cli --release --locked --features separate-bins` rc=0 (46.81s), copied to $S/pre/sep/:
  - vita f6c... see table: vcmp f6c729fc39891ff9099cf84042e688f8 7338000; velab 2f45610b3b1baa571b95b28f2d8f4f60 7338000; vita d0e1cf9dff9b16fbff5f489f6b35cc15 7338000; vrun 3f42b9d96d8b7fd741758cac34e77c9e 7338000
- logs: $S/logs/g0_build.log, $S/logs/g0_sep.log
- sv2v: not on PATH, not found under / (maxdepth 6). Not used.
- harness: $S/run3.sh (vita PRE --obs-dir, iverilog -g2012 + vvp -n, verilator --binary --timing --assert -Wno-fatal + `+verilator+error+limit+1000`)

## Early finding (probe/p2.sv)
- A PLAIN `case` (no qualifier) in a constant function is already E3009 on PRE: `exec_const_stmt` (const_fn.rs:1588) has no `Stmt::Case` arm (catch-all `_ => None` at const_fn.rs:1718). iverilog + verilator fold `g(2)` = 20.
  vita: `p2.sv:10:22: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `g(…)` has no constant-fold arm [in top]`
  ivl: `P=20 Q=20 R=7`; vl: `P=20 Q=20 R=7`
  => the `unique case` half of the row cannot move with a no-op arm alone.

## G1 reproduce the row on HEAD (PRE e1e7e571), 3 tools — DONE
Raw per-cell outputs: $S/g1a/*.{vita,ivl,vl}, $S/g1b/*.{vita,ivl,vl}; flattened table $S/g1_table.txt (116 cells).
Function shape: `f = 7; <q> if (a == 1) f = 10;` / `<q> case (a) 1: f = 10; 3: f = 30; endcase` (casez/casex/`case inside` twins). M = arg 2 (miss reached), N = arg 1 (no miss).

G1a qualifier x form, localparam context (40 cells):
- `unique if` M / `priority if` M: vita `error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `P` value is not a constant: `f(…)` has no constant-fold arm [in top]`; verilator `P=7` rc=0 (no report); iverilog `syntax error` rc=2 (rejects `unique if` / `priority if`).
- `unique if` N / `priority if` N: vita P=10 = verilator P=10.
- `unique0 if` M / `priority0 if` M: vita P=7 (no arm synthesized); verilator P=7 for unique0, rejects `priority0` (`syntax error, unexpected if`); iverilog rejects all.
- every `case`/`casez`/`casex`/`case inside` cell, all 4 qualifiers, M AND N (32 cells): vita E3009 `f(…)` has no constant-fold arm. iverilog/verilator: case/casez/casex M P=7, N P=10 (iverilog adds `vvp.tgt sorry: Case unique/unique0 qualities are ignored.` for unique/unique0); `case inside`: iverilog `Incomprehensible case expression` (rejects); verilator N P=10, M `%Error: ...: Expecting expression to be constant, but can't determine constant for FUNCREF 'f'` (even for unique0 — a verilator limit). `priority0` rejected by both oracles.
- Root for case: `exec_const_stmt` has no `Stmt::Case` arm (plain-case control probe/p2.sv also E3009). A no-op arm for the synthesized task moves NO case cell.

G1b constant contexts (if = `unique if`, pif = `priority if`, case = `unique case`):
| ctx | vita PRE M (if/pif/case) | vita PRE if-N | verilator M | iverilog case M |
|---|---|---|---|---|
| parameter init (pp) | E3009 `f(…)` has no constant-fold arm | P=10 | P=7 | P=7 |
| packed range (pr) | E3009 `a function call that does not fold to a constant is not allowed in a c…` | bits=11 | bits=8 | bits=8 |
| unpacked range (ur) | same E3009 | size=10 | size=7 | size=7 |
| generate-if (gi) | E3010 `f(…)` has no constant-fold arm | gi=other | gi=7 | gi=7 |
| generate-for bound (gf) | E3010 `generate-for condition is not a constant: f(…) has no constant-fold arm` | g6=6 | g6=6 | g6=6 |
| generate-case expr (gc) | E3010 `f(…)` has no constant-fold arm | gc=other | gc=7 | gc=7 |
| $clog2 arg in localparam (sf) | E3009 | P=4 | P=3 | P=3 |
| port width in sub (pw) | E3009 `a function call that does not fold …` | pw=11 | pw=8 | pw=8 |
| instance override #(.W(f())) (ov) | E3009 `the override of parameter W is not a constant, so the declared defau…` | W=10 | W=7 | W=7 |
| defparam (dp) | E3009 `a non-constant override value is unsupported` | W=10 | W=7 | W=7 |
| package localparam, pkg-local fn (pk) | E3009 `package parameter P value is not a foldable constant [in pk]` — ALSO for N (pre-existing, unrelated loud) | E3009 | P=7 | P=7 |
| `pk::f()` from module (pkx) | E3009 `pk.f(…)` has no constant-fold arm | P=10 | P=7 | P=7 |
| `import pk::*; f()` (pki) | E3009 `f(…)` has no constant-fold arm | P=10 | P=7 | P=7 |
| enum label value (en) | E3009 `enum label A value is not a foldable constant` | A=10 | A=7 | iverilog `No function named f found in this context` |
| localparam in gen block (gl) | E3009 [in top.gb] + E3010 gb.P | P=10 | P=7 | P=7 |
| nested callee g has the miss (nest) | E3009 | P=11 | P=8 | P=8 |
| miss inside a for loop, 2-3 misses (loop, both cells M) | E3009 | (no N) | P=7 / P=10 | P=7 |
| class param `class C #(parameter int P = f(2))` (cl) | E2002 parse (vita has no parameterized class here) | E2002 | P=7 / P=10 | iverilog syntax error |
| localparam ARRAY init `'{f(2),1}` (pa) | NOT a fold: run-time frame call (run.json f:frame:sites=1 also for N); M prints `warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION ... [in top.f] [at time 0]`, A0=7 rc=0 | A0=10 | A0=7, no report | iverilog `sorry: unpacked array parameters are not supported yet.` |
- No oracle prints any violation report for any constant-context cell (verilator: 0 `%Error`/`Assertion failed` lines in all G1 M cells that built; iverilog: none on case cells). vita's localparam-array cell (pa) is the only one that reports, at time 0, because vita evaluates that initializer at run time.

IEEE 1800-2017 (RECALLED, not re-read):
- §13.4.3 constant functions, constraint list item (i): "All system task calls within a constant function shall be ignored." (recalled; 1364-2005 §10.4.5 has the same rule). Item (h): system FUNCTIONS limited to those legal in a constant_expression.
- §12.4.2 / §12.5.3: `unique`/`priority` if/case with no match and no explicit else/default "shall issue a violation report"; `unique0` shall not. §12.4.2.1 / §12.5.3.1: violation reports are deferred to the Observed region of the time step (like deferred immediate assertions) and flushed by a later re-evaluation in the same time step (recalled). There is no time step during elaboration-time constant-function evaluation; the text names no rule for a violation inside a constant function (recalled: no such sentence).
- Recalled reading: a violation report is a simulator-produced report, not a system task call, so §13.4.3(i) does not literally cover it; both oracles behave as "no report, fold the value" (measured above).

## POST prototype (measurement only, not a proposal)
- $S/proto = `git archive HEAD`, one hunk in crates/elaborate/src/const_fn.rs `exec_const_stmt` (diff $S/proto.diff md5 f94de040d3dee236a92d50a52b66e1ec):
  `ast::Stmt::SysTaskCall { name, .. } if name.name == ast::UNIQUE_VIOLATION_TASK => { Some(ConstFlow::Normal) }` before the catch-all.
- `CARGO_TARGET_DIR=$S/proto_target cargo build -p cli --release --locked --features separate-bins` rc=0 (1m09s), log $S/logs/proto.log.
- $S/post/vita md5=eac2ebb63b51a3fded499002b755b022 size=7338000 (release, separate-bins build); vcmp c02b1962618beffe6b8b9b0d3827ee18, velab bf553c27bdc2149f12d878ba5d2024ab, vrun 59a14ecf79bb3533c48b31dd2c8a7d0f.
- PRE for every PRE/POST diff below = $S/pre/sep/vita (d0e1cf9d…, same HEAD source; `pre==pres on g2` checked: identical summaries to $S/pre/vita).

## G2 consumer census — measured (static enumeration of every call site: see "G2 static" below)
Cells: $S/g2 (152) + $S/g2/fix (24) + $S/g2b (84) = 260 run-time-position cells, if-form and case-form, M and N. Summaries: */pres_summary.txt, */post_summary.txt; oracle text for every moved position: $S/g2o/*.{ivl,vl}, $S/g2b/oracle_M.txt.
Witness rule: `sites` in run.json counts LOWERED call sites, not executions (c_rep_if_N is sites=1 yet folded) — so the witness used is W4031 at run time on the M cell, PRE vs POST.

A. 65 position kinds measured (x {if, case} x {M, N}); 25 kinds move on POST (if-form M only, lists B-D); 4 kinds are loud on vita for unrelated reasons in all 4 cells (blk2: nested named-block local E3010; cons: class-handle decl init E3009; evix: edge event bit-select E3009; stm: struct member width E2002); the other 36 kinds keep a RUN-TIME call on PRE and POST (summary lines identical; M reports W4031 at run time, as verilator does for if and both oracles for case): acomb arith asrt blk bsel ca case2 casee casei clog dflt disp dly ffx finit forb genca ifc insd lbit lidx nb nda port proc psel sel2 sfmt sidx sinit tern uidx vinit vinit2 waitc wcond (continuous assign, net-decl assign, module variable init, procedural RHS, always_comb, always_ff/NBA, port connection, $display/$sformatf arg, if/case expr/case item, ternary, operand, bit-select/1-D part-select offset/unpacked/string-array index read, lvalue index, multi-dim packed element lvalue, `#(f())`, for/while/wait condition, inside member, assert condition, function default arg, automatic/static function-local init, $clog2 arg at run time, block-local init, generate-block assign).

B. Positions where the POST fold REPLACES the run-time call and DROPS a report an oracle prints (loud→silent for the violation report) — 12 if-form M cells:
| cell | position | PRE | POST | verilator (if) | iverilog (case twin) |
|---|---|---|---|---|---|
| c_rpt_if_M | `repeat (f(2)) n++;` in initial | W4031 [at time 1], n=7 | n=7, no report | `[1] %Error: c_rpt_if_M.sv:4: Assertion failed in top.f: 'unique if' statement violated` n=7 | `WARNING: c_rpt_case_M.sv:4: value is unhandled for priority or unique case statement` Time: 1, n=7 |
| e_trpt_if_M | `repeat (f(2)) @(posedge clk);` | W4031 [at time 0], t=13 | t=13, no report | `[0] %Error: ... 'unique if' statement violated` t=13 | WARNING ... Time: 0, t=13 |
| e_ktrpt_if_M | same inside `task automatic` | W4031 [at time 0] | no report | `[0] %Error: ... violated` | WARNING ... Time: 0 |
| e_ktrp2_if_M | `repeat (f(2)) n++;` inside task | W4031 [at time 1] | no report | `[1] %Error: ... violated` | WARNING ... Time: 1 |
| e_frpt_if_M | `repeat (f(2)) g++;` inside a function | W4031 [at time 1] | no report | `[1] %Error: ... violated` | WARNING ... Time: 1 |
| c_sizd_if_M | `$size(a2, f(2)-6)` unpacked | E3009 `a whole unpacked array has no value in this context` | sz=4, no report | `[1] %Error: ... violated` sz=4 | iverilog rejects (`$size has too many arguments`) |
| e_qsizp_if_M | `$size(p2, f(2)-6)` packed | E3009 `unsupported system function in expression` | s=4 | `[1] %Error: ... violated` s=4 | rejects |
| e_qhigh_if_M | `$high(p2, f(2)-6)` | E3009 | h=3 | `[1] %Error: ... violated` h=3 | rejects |
| e_qleft_if_M | `$left(a2, f(2)-6)` | E3009 | l=0 | `[1] %Error: ... violated` l=0 | rejects |
| e_sel3_if_M | `m[f(2) +: 2]` on `logic [15:0][3:0]` | E3009 `variable indexed part-select on a multi-dim packed array is unsupported` | m=78 | `[1] %Error: ... violated` m=78 | iverilog assert-crash (rc=134) |
| e_irpt_if_M | `x = repeat (f(2)) @(posedge clk) y;` | E3009 `a runtime (non-constant) repeat(n) count in an intra-assignment event control is unsupp…` | x=5 t=13 | `%Error-UNSUPPORTED: Unsupported: repeat event control` | WARNING ... Time: 0, x=5 t=13 |
| e_nrpt_if_M | `x <= repeat (f(2)) @(posedge clk) y;` | E3009 (non-blocking twin) | x=5 | unsupported | WARNING ... Time: 0, x=5 |
- rpt family (5): PRE is CORRECT incl. the report (fallback: `repeat_unroll_count` declines → runtime `$repeat_cnt$` path with a run-time call); POST drops it = **loud→silent** (vs both oracles).
- query/offset/intra-repeat family (7): PRE loud (E3009), POST value without the report verilator (or iverilog's case twin) prints = loud→value-minus-report.

C. Positions that are constant expressions by IEEE where POST folds and NO oracle reports (loud→value, value = oracle) — 9 if-form M cells + G1's 35:
c_cast_if_M (`f(2)'(x)` cw=7 = vl 7, ivl case 7), c_strm_if_M (`{<< f(2) {x}}` st=6890 = vl), c_elabt_if_M (`$info` in generate: POST `info[VITA-I3006] I-ELAB-USER-INFO: el=7 [in top.g]`; verilator build log `-Info: c_elabt_if_M.sv:7:5: el=7`; iverilog rejects any call there), e_barr_if_M (block-local `int arr [f(2)]` s=7), e_td_if_M (typedef width b=8), e_fret_if_M (return width b=8), e_fform_if_M (formal width b=8), e_cg_if_M (covergroup bin `{f(2)}` cov=100.0 — NO ORACLE: verilator `Non-constant expression in bin range; values must be constants` for if AND case, iverilog cannot parse covergroup).
G1: a_unique_if_M, a_priority_if_M, 33 b_* cells (pp, pr, ur, gi, gf, gc, sf, pw, ov, dp, en, gl, pkx, pki, nest, loop x {if, pif} + loop_if_N) — all POST value = verilator value, no report.

D. silent-wrong → correct (PRE wrong value at exit 0; POST = both oracles' value; no oracle reports) — 5 if-form M cells:
c_rep_if_M `{f(2){1'b1}}` PRE r=00000000 POST 0000007f; c_rep2_if_M (continuous assign) same; c_rep3_if_M `{f(2){2'b01}}` PRE 0 POST 00001555; e_pswr_if_M `v[0 +: f(2)]` PRE pw=1 POST 7f; e_psww_if_M `v[0 +: f(2)] = '1` PRE 00000001 POST 0000007f. verilator same values, no report.

E. NEW pre-existing silent-wrong found (independent of this row; PRE = POST): any user-function call the constant interpreter declines, used as a replication count or an indexed part-select WIDTH, lowers to an empty replication / 1-bit select at exit 0 (`const_bound.rs:185` `lower_const_width_expr` keeps the lowered call when `const_bound_u32` declines; the engine's shallow fold then reads `unwrap_or(0)` / `unwrap_or(1)`, as that function's own doc says). 12 cells: c_rep_case_{M,N}, c_rep2_case_{M,N}, c_rep3_case_{M,N}, e_pswr_case_{M,N}, e_psww_case_{M,N} + probe/p6.sv (`$display` in body, and a PLAIN `case` body): vita `r1=00000000 r2=00000000 pw=1` vs iverilog and verilator `r1=0000007f r2=0000007f pw=7f`. No existing ROADMAP row found (grep `lower_const_width_expr|const_bound_u32|0-width replication|replication count|part-select width` in ROADMAP/PROBE_CATALOG/REMAINING_WORK/manual 006: only ROADMAP:127 ⓩ and :176 negative count).

F. Other pre-existing observations (PRE = POST, not moved): (1) `m[f(2)] = 4'hA` on `logic [15:0][3:0]` calls f three times (probe/p5.sv: `call a=2` x3; iverilog once) — W4031 x3 in e_sel2_*_M vs one report in each oracle; a global-write body is called once (p4.sv cnt=1, with W4030 fallback to vm); possibly the ROADMAP:311 / :478 double-mention family, not checked. (2) `localparam int A [2] = '{f(2),1}` is evaluated by a run-time frame call (b_pa_*: sites=1 also for N), so vita reports W4031 [at time 0] where verilator folds silently (iverilog: unpacked array parameters unsupported). (3) `parameter` in a package whose initializer calls a package-local function is E3009 even with no miss (b_pk_if_N). (4) `$size(a2, 4)` out-of-range dim: vita x, verilator 0 (c_sizd_if_N).

## G3 lane table — DONE
Cells $S/g3 (14: 10 moved, 4 unchanged incl. a case control). Outputs $S/g3/<cell>.<pre|post>.<native|interp|vm|staged|jit>.
| lane | PRE | POST | how |
|---|---|---|---|
| native / interp / vm (`--backend`) | 14/14 cells: one distinct output across native, interp, vm, staged (after dropping per-stage `errors=` summary lines) | 14/14 same | measured |
| staged vcmp→velab→vrun (separate-bins) | same output as one-shot | same | measured |
| .velab content | unchanged cells (c_proc_if_M, c_ca_if_M, e_sel2_if_M, c_rpt_case_M): PRE .velab == POST .velab byte-identical (`cmp`) | moved cells differ (c_rpt_if_M 711→701 bytes; e_trpt 781→698; e_ktrpt 827→733; e_frpt 816→795; c_rep 621→630; e_pswr 666→675); PRE-loud cells have no PRE .velab | measured |
| format_version | header bytes `56 45 4c 41 42 00 00 00 23` (35) PRE and POST; `CURRENT_FORMAT_VERSION: u32 = 35` (vita-artifact/src/header.rs:15). No sim-ir type touched by the arm | 35 | measured |
| JIT (`--features jit`, `VITA_JIT=1`) | not built for PRE | $S/post/vita_jit md5 32fca449f4bbab34b8b2c8f0ebdb48ec size 9181376 (build rc=0, 1m06s, $S/logs/proto_jit.log): 14/14 cells output == POST native; JIT fired (`JITBODY templates_compiled=1 ... activations=1`) in 5/14 (c_cast, e_ktrpt, e_pswr, e_sel2, e_trpt) | measured (POST only) |
| product (`--no-default-features`) | not built | not built | opted out by construction: crates/elaborate, hdl-parser, hdl-ast have no `[features]` and 0 `cfg(feature` lines (grep); cli's `cfg(feature = "oracle")` (frontend.rs:841/843, stage_args.rs:88/90/518/520) only gate backend selection; the fold runs in elaborate |
- The arm lives in elaborate, so every executor sees the folded IR; no lane measured differently on POST.

## G4 sibling handling and queue rows — DONE
- Sibling precedent: NO ignore-arm exists for any system task in `exec_const_stmt` (const_fn.rs:1588-1720; catch-all `_ => None` with comment "NonBlocking / timing / fork / system-task / case / disable / … → loud."). Measured ($S/g4, PRE):
  | body task | localparam P = f(2): vita | iverilog | verilator |
  |---|---|---|---|
  | `$display` | E3009 `f(…)` has no constant-fold arm | P=3, nothing printed | P=3, nothing printed |
  | `$info` | E3009 | P=3 | P=3 |
  | `$warning` | E3009 | P=3 | build log `%Warning-USERWARN: "in f a=2"`, P=3 |
  | `$error` | E3009 | P=3 | `%Error: ... Expecting expression to be constant, but can't determine constant for FUNCREF 'f'` (build_rc=1) |
  Run-time twin (`x = f(2); repeat (f(1)) x++;` with the task in f): all three tools print the task output twice (`in f a=2`, `in f a=1`) — i.e. both oracles evaluate a `repeat` count's call at RUN time (g4/d_display_rt).
  Since the display-family declines, it never reaches a "replace" consumer today; an ignore-arm for it would land on the same B-list consumers (repeat count etc.) and drop the run-time `$display` output there.
- ROADMAP/PROBE_CATALOG/REMAINING_WORK rows touching the interpreter (grep `const_fn.rs|exec_const_stmt|constant function|constant-fold arm|interpreter`): ROADMAP:547 (unique-if-chain, BLOCKED on this row), :548 (this row), :549 (unique-pkg-closure), :173 (`int unsigned` return read signed, `const_fn_ret_wsign`), :217 (real-returning constant function E3009), :341 (cross-package recursion, hand-IEEE), :163 (placement/cast fold residue), :114 🆕 H ⓔ (const-function local assignment `envw`), PROBE_CATALOG:49 (string compare in a constant function E3009). manual 003:175 (generate-scoped routine loud), 006:187-191 (the three holds on arming chains in subroutines). No row for a `case` arm in the interpreter; no row for system tasks in constant functions.


## G2 follow-up cells from the static census ($S/g2c 40 cells, $S/g2d 28 cells; oracle text $S/g2c/oracle.txt, $S/g2d/oracle.txt)
| cell (if-form M unless noted) | position / consumer | PRE | POST | oracle text |
|---|---|---|---|---|
| h_tdly | `#(f(2) * 1ns)` events.rs:1065 FALLBACK | W4031 [at time 0], t=7 | t=7, no report | vl `[0] %Error: h_tdly_if_M.sv:5: Assertion failed in top.f: 'unique if' statement violated` t=7; ivl case twin `WARNING ... Time: 0` t=7 |
| h_cad | `assign #(f(2)) w = a;` netdecl.rs:876 | W4030 (vm fallback) + W4031 x4 (2 at t0, 2 at t1), w at 8 | w at 8, no report, no W4030 | vl `[0] ... violated`, `[1] ... violated`; ivl case twin one WARNING Time: 0 |
| h_wd | `wire #(f(2)) w = a;` var_init.rs:127 | same as h_cad | w at 8, no report | same as h_cad |
| h_gcl | generate-case LABEL `f(2):` generate.rs:459 SILENT | gcl=default (silent-wrong) | gcl=label | vl gcl=label |
| h_prd / h_pwr | `v[f(2):0]` read / write packed.rs:1890 | pr=1 / 00000001 | ff / 000000ff | vl ff / 000000ff |
| h_mdpr | `m[f(2):0]` on `logic [15:0][3:0]` | m=1 | 89abcdef | vl 89abcdef |
| h_plsb | `v[11:f(2)]` | pl=0 + W4031 [at time 1] | pl=1e + W4031 [at time 1] | vl pl=1e, no report |
| k_evlsb | `@(posedge v[f(2)-7])` events.rs:765 | E3009 `edge event-control bit-select must select the net's LSB with a constan…` | ev at 1, no report | ivl case twin `WARNING ... Time: 0` ev at 1; vl `Unsupported: Impure function calls in sensitivity lists` |
| k_inoff | `m[1][f(2)-7 +: 2]` packed_inner.rs:183 | E3009 `variable indexed part-select on a multi-dim packed array…` | m=ef, no report | vl `[1] ... 'unique if' statement violated` m=ef |
| k_evlvl | `@(v[f(2)-7+3])` events.rs:835 | E3009 `a level (non-edge) event control on a select with a variable index is…` | E3009 `a level (non-edge) event control on a select of a dynamic-storage handle, a string, a subroutine's…` (= PRE text of k_evlvl_if_N) | vl unsupported (impure call in sensitivity list) |
| k_cons, k_dlo, k_dhi, k_dwt (constraint, dist bound / weight) | crv.rs:625, cover_synth.rs:194/199/202 | E3009 for if/case, M/N | same | not run |
- More pre-existing silent-wrongs of family E (PRE = POST, both oracles agree): h_gcl_case_M, h_gcl10_case_N (non-folding generate-case label silently takes `default`; oracles take the label), h_prd_case_{M,N}, h_pwr_case_M, h_mdpr_case_{M,N} (`[f():0]` = 1 bit / 1 element), h_plsb_case_{M,N} (`v[11:f()]` pl=0 / pl=1 vs oracles 1e / 3). Family E total: 19 cells + probe/p6.

## G2 static census (opus-build helper, read-only; $S/g2_static.tsv, 188 consumer rows + header)
- `eval_const_call` is called only at const_fn.rs:514, :520 (`const_eval_in_scope`) and :1020 (`eval_const_env`). No call site in crates/cli/src, sim-engine/src, hdl-parser/src.
- decline behaviour: LOUD 60, LOUD-after-chain 25, LOUD-per-comment 2, SKIP 54, SILENT 18, ANALYSIS 16, FALLBACK-RUNTIME 13. REPLACE: yes 12, conditional 1 (packed.rs:339 `lower_index_expr`: only when the lowered tree also reduces and disagrees), no 168, n/a 7.
- IEEE run-time positions that REPLACE a call on a successful fold (fallback = run-time call): stmt_flow.rs:1119 `repeat` (via `repeat_unroll_count` const_bound.rs:146; same predicate read by frames_classify.rs:816 and frames_reserve.rs:685), events.rs:1065 `#(…time literal…)`, netdecl.rs:876 `assign #d`, var_init.rs:127 `wire #d` (via ca_delay_rt.rs:71). All four measured: loud→silent on POST (c_rpt family, h_tdly, h_cad, h_wd).
- IEEE run-time positions that are LOUD on decline (POST turns them into a value, no report): events.rs:46/:135 (measured e_irpt/e_nrpt), events.rs:765 (k_evlsb), events.rs:835 (k_evlvl: still loud), packed.rs:2134 (e_sel3), packed_inner.rs:183 (k_inoff), expr_main.rs:1142 / expr_special.rs:652 array query dim (c_sizd, e_qsizp, e_qhigh, e_qleft), expr_main.rs:612/615 + lvalue.rs:304 string-array index (sidx cells: run-time call PRE=POST, no move), crv.rs:625 / cover_synth.rs:194/:202 (E3009 PRE=POST), cover_synth.rs:199 dist high bound SILENT skip (k_dhi: E3009 from :194 first).
- Constant-required positions with FALLBACK to a run-time tree on decline: expr_main.rs:1050 (replication count), expr_main.rs:846/:882 + lvalue.rs:484/:503 (`+:` width), packed.rs:1890, :2204, packed_inner.rs:152 (`[m:l]`) — these are family E's sinks.
- Other elaborate-time body walkers: none has a `$__vita_unique_violation` arm (`systask.rs:195` is the only elaborate reference). const_bound.rs:441 `const_stmt_width_safe` `_ => false` at :488 (so `const_bound_u32` tier 2 declines any call whose body holds a SysTaskCall; tier 3 `const_int_selfdet` const_bound.rs:84 still reaches `eval_const_call`); inline_fold.rs:11 `fold_straight_line` `_ => false` at :174; package.rs:144 `pkg_stmt_pure_orig` `_ => false` at :183; const_level_header.rs:337 treats SysTaskCall as no suspension blocker.

## G5 fix shapes with moved-cell counts (measured set: 444 vita cells = g1a 40 + g1b 76 + g2 152 + g2/fix 24 + g2b 84 + g2c 40 + g2d 28)
Shape (a) — no-op arm everywhere (= the prototype, MEASURED): 71 cells change (g1a 2, g1b 33, g2 6, g2/fix 2, g2b 17, g2c 8, g2d 3), all if-form; 0 case cells change.
| direction | n | cells |
|---|---|---|
| loud → value = oracle, no oracle report | 42 | G1 35 (a_unique_if_M, a_priority_if_M, b_{pp,pr,ur,gi,gf,gc,sf,pw,ov,dp,en,gl,pkx,pki,nest,loop}_{if,pif}_M, b_loop_if_N); c_cast, c_strm, c_elabt, e_barr, e_td, e_fret, e_fform |
| loud → value, no oracle | 1 | e_cg (verilator refuses a call in a bin for if and case; iverilog cannot parse covergroup) |
| silent-wrong → correct | 10 | c_rep, c_rep2, c_rep3, e_pswr, e_psww, h_gcl, h_prd, h_pwr, h_mdpr, h_plsb (h_plsb keeps its vita-only W4031) |
| **loud → silent** (PRE reports W4031 at run time like the oracles; POST drops it) | 8 | c_rpt, e_trpt, e_ktrpt, e_ktrp2, e_frpt (`repeat`), h_tdly (`#(…ns)`), h_cad (`assign #`), h_wd (`wire #`) |
| loud (E3009) → value WITHOUT the report an oracle prints | 9 | c_sizd, e_qsizp, e_qhigh, e_qleft, e_sel3, k_inoff (verilator reports), e_irpt, e_nrpt, k_evlsb (iverilog case twin reports; verilator unsupported) |
| loud → loud, message changes | 1 | k_evlvl |
=> shape (a) is BLOCKED by 8 loud→silent + 9 loud→value-minus-report (ER §2.2 "Never widen a loud into a silent").

Shape (b) — the arm is a no-op only inside a fold requested by a consumer whose position IEEE requires to be constant (opt-in mode set at that consumer's call site, e.g. a Cell on the Elaborator saved/restored like `const_call_fn`); a fold requested by any other consumer that reaches the arm declines exactly as PRE. NOT prototyped. Derived from the (a) measurement with the 18 non-constant-position cells held at PRE: 53 move (42 loud→value=oracle, 1 no-oracle, 10 silent-wrong→correct), 0 loud→silent. Fails safe: a consumer not opted in keeps PRE. The mode must be set at the consumer, not inside a wrapper: `const_bound_u32` serves required-constant widths (packed.rs:2120, `lower_const_width_expr` const_bound.rs:185, packed.rs:1890) and run-time positions (packed.rs:2134 offset, `repeat_unroll_count` const_bound.rs:159).

Shape (c) — no-op arm that records "arm reached" and the 7 measured run-time consumer families decline when it is set (opt-out: const_bound.rs:146 repeat; events.rs:1065; ca_delay_rt.rs:71 (netdecl.rs:876, var_init.rs:127); events.rs:46/:135; expr_special.rs:652 / expr_main.rs:1142; packed.rs:2134 / packed_inner.rs:183; events.rs:765/:835). Same 53 on the measured set; any run-time consumer missing from the list becomes loud→silent, so completeness rests on the static census (188 rows).

Not movable by any no-op arm (E3009 PRE = POST): every `case`/`casez`/`casex`/`case inside` constant function, any qualifier, reached or not — G1a 32 + G1b 17 case cells (b_cl_case_M is E2002 parse, b_pa_case_M a run-time call) + plain-case control probe/p2 — root: `exec_const_stmt` has no `Stmt::Case` arm.

Recommendation: (b). Re-scope the row to the `if` form (lone `unique if` / `priority if` with no `else`); file the `case` half as its own row (a `Stmt::Case` arm, which also opens every plain-case constant function; ER §2.4 case-sizing rule applies); file family E as a new §2 row.
