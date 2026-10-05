# §4.5.585 soundness lens — round 1

PRE  = S/s585/pre/vita  md5 c766b8d59edabaef96436e7b46e75dc6 (HEAD 75f255d4)
POST = S/s585/post/vita md5 6328a971c02b99269f42a1b39f545fda (wt-s585; slice.patch md5 ce8e4ae7879e48cbc652a6ac1fcdd818)
Harness: S/s585/r1/sound/r.py (copy of g/r.py, POST -> post/vita, out -> r1/sound/out)
Cells: S/s585/r1/sound/cells

## Verdict: FAIL (round 1) — F1 BLOCKING, F2 BLOCKING; both from one false premise (Q3): "no function can reach
## an item `function void`" fails for a formal-less, write-free, static void function. wall_s 2290.
## Status
- [x] Q1 else_if_at key soundness — CLEAN (census + 4 cells)
- [x] Q2 Stmt::If producer census — CLEAN (census)
- [x] Q3 first_if_arm_only lifetime census — flag lifetime CLEAN; coverage premise FALSE (F1, F2)
- [x] Q4 reach census of new arms — FAIL: F1 (CA -> fn -> formal-less item fv, 6 cells)
- [x] Q5 downstream consumers — routes CLEAN; one consumer (package-scoped closure walk) refuses the arm: F2

## Findings
### F1 BLOCKING — c2 is not closed: a formal-less static item `function void` is reachable from a CA
New instance (POST-only on the chain spelling); root pre-existing (§2 🆕 AB: PRE-H lone-`if` twin and PRE
`unique case` twin report t0 too). Site: crates/hdl-parser/src/functask.rs:265 predicate `!(item && is_void)` arms
an item `function void`; the claim at lib.rs:889-891 ("every measured route from a function to one is refused (VITA-E3009 …)") and
the pin `refusals_that_keep_a_void_function_unreachable` (cli/tests/unique_if_chain.rs:1278) hold only for g WITH formals (every planner cell t1s/u1s/u3s/t5*/t6*/v* passes
`input logic x, z`). The frame-call subset admits a nested call with no copy-in, so `f` runs `g()` in 🆕 AB's t0 pass.
Cell cells/q6a_fn_vfn_noformal.sv: `function void g(); unique if (a) ; else if (b) ; endfunction`,
`function logic [1:0] f(x,z); g(); return {x,z}; endfunction`, `assign y = f(a,b);`, TB `a=0;b=1; #2 b=0`.
  verilator: no t0 line; `[2] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement
  violated` (x2, again [3],[4]) ; iverilog (q6a_…_case.sv `unique case` twin): `WARNING: …:3:` `Time: 2  Scope: top.u.g`
  only ; PRE: silent, rc 0 ; POST (native=interp=vm): `q6a_fn_vfn_noformal.sv:3:12: warning[VITA-W4031]
  W-RUN-UNIQUE-VIOLATION: … [in top.u.g] [at time 0]` then `[at time 2]`, rc 0 ; PRE-H: same t0+t2 at 3:26.
Same t0 line on POST, both oracles silent at t0 (iverilog `Time: 2` only): q6h (f -> h -> g()), q6d (interface g()
via `i.f(a,b)`, t0 x2), q6j (`wire [1:0] w = f(a,b)`), q6k (port expression `.p(f(a,b))`), q6p (package g() reading
package vars, via import). Refused PRE=POST: q6f (automatic g/f: E3009 13:7). Clean: q6i (always_comb, 🆕 Z waits).
Effect: violates PLAN step-3 acceptance ("the t0 list is exactly aa1_chain, s1s, s3") and the c2 rationale
("🆕 AB … its reach must not grow"). Illegal twin q6b (function -> formal-less task; vl %Error-FUNCTIMECTL) also
gains t0 = the u8/u23 over-acceptance class.

### F2 BLOCKING (loud regression) — a package-scoped call now refuses a legal design PRE ran
Same false premise as F1 (an item `function void` is reachable from a function). New instance; root pre-existing:
the package-scoped closure walk package.rs:144 `pkg_stmt_pure_orig` ends `_ => false` (:183), so the synthesized
`$__vita_unique_violation` statement makes the callee "not self-contained" (inject_pkg_callees package.rs:1621 ->
inline_fn.rs:611 message). PRE-H (lone `unique if`) and PRE `unique case` twin are refused the same way.
Cell cells/q6t_pkg_scoped_const_chain.sv: package `localparam int MODE = 2; function void g(); unique if (MODE == 0)
begin end else if (MODE == 1) begin end endfunction`, `f(x,z)` calls `g()`, module `assign y = pk::f(a, b);`.
  verilator: rc 0, `[0] %Error: q6t_pkg_scoped_const_chain.sv:4: Assertion failed in pk.g: 'unique if' statement
  violated` ; iverilog (case twin): runs, `Time: 0`, `Time: 2` ; PRE: rc 0, runs (t=1 y=1, t=3 y=0), no report ;
  POST: rc 1, `q6t_pkg_scoped_const_chain.sv:16:7: error[VITA-E3009] E-ELAB-UNSUPPORTED: package-scoped call
  `pk::f(...)` reaches `pk::g`, whose body names something outside its own formals/locals, same-package constants,
  variables and subroutines — … `import pk::*` and call `f` by its bare name instead [in top.u]` (all backends).
The message blames a name the user did not write. Following its hint (q6w, import + bare `f`): POST runs, t0 x2 + t2
(= PRE-H; iverilog case twin t0, t2; vl [0]).
Related, loud->loud only (rc 1 both): the E3009 TEXT changes on pkg_vfn_via_f_rt, pkg_vfn2_via_f_rt,
t5s_pkg_fg_scoped, u3s_pkg_fg_nowrite_scoped (PRE "frame function/task `pk::f` body uses an assignment to a net
outside…", POST the inline_fn.rs:611 text) — the only non-W4031 output change in 1005 PRE/POST runs.

## Q1 — else_if_at key domain
- Domain: span.lo = byte offset into ONE expanded buffer per Parser. Census: Parser::new has 1 production caller
  (hdl-parser/src/api.rs:27); cli/src/frontend.rs:338 parses `pp.text` (preprocess_sources concatenates every
  command-line source, every `include and every macro expansion as text; SourceMap maps back). Macro bodies are
  materialized per expansion -> distinct offsets per expansion.
- Token-offset uniqueness: lexer tokens non-overlapping; strip_attribute_instances (hdl-lexer lib.rs:774) only drops
  tokens; parser backtracking sites = 8 (expr_primary.rs:216, functask.rs:427, type_params.rs:473/487/499/905/912/921),
  none wraps parse_statement -> parse_if never runs speculatively; no sub-Parser.
- Stmt::If construction sites in hdl-parser (non-test) = 11: stmt_ctl.rs:96 parse_if (lo = `if` token),
  assertions.rs:298 parse_assert (lo = assert/assume token), type_param_shape.rs:176 (module-item initial),
  udp_table.rs:118/126, udp.rs:619/627/635/643/670 (UDP), monomorph.rs:194/656 (post-parse, in-place, keeps else_s).
  A recorded lo is always an `if` token preceded by `else` [+qualifier]; only parse_if output starts there ->
  no collision is constructible. The walk reads only nodes the same Parser just built.
- Misses: `else`/`if` from a macro body or argument, `include, multi-file, `(* *)` all reach parse_if as written.
- Cells (verilator / PRE / POST, 3 backends identical):
  q1a_macro_twice: vl [1]@6 [2]@8, t3 silent ; PRE silent ; POST 6:8 t1, 8:8 t2 (= vl lines/times)
  q1b_macro_else (ELSE_IF, ELSE IF, BR(if..), BR(else)): vl t1@9 t2@11 t4@15 t5@17, t3 silent ; POST same 4, t3 silent
  q1d_include (chain in included task + main): vl chain.svh:2 t1, main:7 t2 ; POST chain.svh:2:12 t1, :7:15 t2
  q1e_multi (2 files; `else assert (b) else if` in file 2): vl a.sv:3 t1 only ; POST a.sv:3:15 t1 only
## Q2 — producers the walk can meet
- parse_statement dispatch (stmt.rs:150-167): `Kw::If => parse_if()` (always Stmt::If, no wrapper);
  qualifiers -> parse_unique_priority -> parse_if (returns the If; span still at `if`). So a written `else if`
  / `else <qual> if` is always recorded; no parse-time statement hoist exists (grep hoist/pending in hdl-parser: 0
  statement hoists).
- Statement-position Stmt::If producers live during parse = 2 (parse_if, parse_assert); the other 9 construction
  sites are UDP (5+2), module-item initial (type_param_shape.rs:176) or post-parse (monomorph x2, in-place, keep else_s).
  randcase/foreach/do-while/break-continue/wait/labelled/delay/event/`;`/case/fork build no Stmt::If (census = 0)
  -> they end the series (no arm), PRE-equal. do-while clones the armed body (both copies armed, = PRE lone-if).
- hdl-ast has no generic Stmt walker; elaborate consumers in Q5.
## Q3 — first_if_arm_only lifetime
- Writers: 3 (lib.rs:973 init false; functask.rs:265 replace; functask.rs:267 restore); reader 1 (assertions.rs:501).
  replace/tf_body/restore are adjacent lines, no `?`/return between; tf_body callers = 2 (functask.rs:266 fn,
  :315 task — task never touches the flag). parse_function_def callers = 2: classes.rs:285 (false; every class
  `function` incl. `new`), module_items.rs:321 parse_function_item (true; <- :616 $unit, :1340 module/interface/
  package/program/generate items). Class `static/const/pure/extern` members are parse errors (classes.rs ~349);
  no DPI/let/checker parser; covergroups have no bodies. Parse errors are fatal before elaborate
  (frontend.rs `if !parse_errors.is_empty() … return None`), so a leak on a recovery path is unobservable.
- Const interpreter reach: exec_const_stmt entered only via eval_const_call (const_fn.rs:1509, nested :1020);
  bodies from const_fn_def -> pkg_funcs / const_func_table, producers = 5 (instance.rs:527, iface_inst.rs:232,
  package.rs:833->1033, :1695, :1730), every one `ModuleItem::Func` (= non-void item fn, flag true). Unhandled stmt
  (task call / sys task) -> `_ => None` (const_fn.rs ~1719): callee bodies never run.
- .vu identity: q3vu_fn_only (chains in pkg fn, class new/f/void, ifc fn, module fn incl. for-body, case-arm,
  unique0, `else assert`, `(* *)`): PRE .vu == POST .vu (md5 6e22181d…); positive control q3vu_pos (+1 task chain)
  differs (5133 vs 5232 B). => every function body is PRE's tree.
- q3_flag_restore (class new/t/v/t2, module fn/tk/fv, procedural): vl reports all 8; POST reports t,t2,tk,fv,proc
  (armed after a class fn / module fn: restored), silent new,v,fn (documented residue); staged == one-shot.
- CA/decl-init/comb routes into an armed `function void` (new cells, PRE = PRE-H = POST, all rc=1):
  q3g1 class-in-module fn -> module fv: E3010 8:7 ; q3g2 class fn -> vif.g: E2002 8:11 (virtual iface member) ;
  q3g4 module fn -> child.g hier: E3009 17:7,10:5 ; q3g5 decl-init `logic y0 = f(a,b)` -> fv: E3009 15:7 ;
  q3g6 always_comb f -> fv: E3009 14:7 ; q3g20 ctor at decl-init -> module fv: E3010/E3009 x5.
  q3k class fn `fork t(); join_none` -> class task: E3010 7:10 + E3009 (no location) ; q3i interface class: E2002
  1:11 (parse) ; q3l `let` -> fn: runs, g unreferenced, silent = vl.
  q3p parameterized class fn -> `this.t()` class task (ILLEGAL: vl %Error-FUNCTIMECTL, iverilog syntax error): POST
  W4031 t0 x2 through the CA; PRE-H t0 x2 and PRE `unique case` twin t0 x2 -> same root as PLAN u8/u23
  (pre-existing over-acceptance of an illegal design), new instance shape only.
## Q4 — reach of the new arms at t0 / elaboration
- F1: the item `function void` arm is reachable from a CA when the void function has no formals and writes nothing
  (q6a/q6h/q6d/q6j/q6k/q6p POST t0 W4031; both oracles silent at t0). Formal kinds probed (q7a default input,
  q7b ref, q7c const ref, q7d unused input, q7e formal-less with a local write, q6f automatic): all E3009 PRE=POST.
  Constant context through it (q7f localparam): E3009 PRE=PRE-H=POST (vl refuses too).
- Function bodies are PRE's tree (Q3 .vu identity), so every elaboration-time evaluator (const interpreter, param /
  localparam / genvar / header defaults, $bits contexts) and every CA / net-decl / port / decl-init function call sees
  PRE's AST; the routes into an armed item `function void` are refused (Q3 cells + PLAN D1).
- New procedural t0 cells (vl / PRE-H / POST; all 3 backends identical; staged == one-shot):
  q4t1 always_comb -> task(args) ; q4t2 always_comb -> task reading module vars ; q4t3 always @* -> same task ;
  q4t8 always_comb -> task with default args `= a`/`= b` ; q4t11 always_comb fed through `wire x = a` ;
  q4t13 always_comb -> `function void` reading module vars ; q4t14 always_latch -> task:
  every one vl first report t2, PRE-H t2, POST t2, none at t0.
  q4t5 always @(posedge clk) with `initial clk = 1`: vl t2 ; iverilog case twin `Time: 2` ; PRE-H t2 ; POST t2.
  q4t7 class task from a child-module initial + fork, both at t0: vl `[0]` ; iverilog case twin `Time: 0` x2 ;
  PRE-H t0 x2 ; POST t0 x2 (both oracles report at t0: not a split).
- Corpus: 0 of ~1500 bench sources contain `unique|priority|unique0 if` (grep); picorv32 .vu PRE == POST.
## Q5 — downstream consumers of the arm
- Arm = SysTaskCall($__vita_unique_violation, [StrLit]) -> map_severity (systask.rs:195, 1 caller :567).
- AST consumers of SysTaskCall in elaborate = 29 sites / 21 files; classifiers read it as neutral: frames_classify_write
  (no outside write, no task enable), block_local/proofs, multidriver (false), da/loops, da/writes (write-dest args
  only), da/reads (args only -> StrLit reads nothing), const_level_header (no T0 decline), ast_query (callees in args).
  No elaborate/engine predicate keys on `else_s` absence (grep `else_s: None, ..` / is_none / is_some in match
  position: 0). No statement-depth cap past the parser's 256 (q5_long_chain250: 250-deep chain, POST = vl t2/t3
  lines 5/260, 3 backends equal, staged == one-shot).
- run.json routes, PRE vs POST, native/interp/vm, 317 planner cells + 18 lens cells (1005 runs): 6 diffs, all the
  `wprog` asked/declined counter (b02_frame_lanes, ifc_vfn_rt, c17/dif_c17, c20/dif_c20); codegen, native,
  subroutine routes (frame/inlined), backend, exit identical. POST == PRE-H run.json on all 4 distinct cells (obsh.py),
  i.e. the counter moves exactly as a PRE lone-`if` arm moves it; kernel.rs:406-413 documents wprog_why as
  report-only ("cannot change a value, a lane or a diagnostic").
- --obs-procs (335 cells, PRE vs POST native): `processes` evaluation counts equal in every cell; `builtins` differs
  in 93 cells, only the row `unique/priority check` (profile.rs:473) = arm executions.
- Lanes: tgt-jit (md5 34f9d5da…, built 09:00) and tgt-nodef (md5 7eaa30f8…, 09:01) are the step-1 builds; the
  default build of 09:02 equals the 09:49 rebuild (md5 6328a971… both), so no code change after 09:02; that the
  09:00/09:01 sources equal it is UNVERIFIED. J1 (VITA_JIT=1, VITA_JIT_STATS=1): `JITBODY templates_compiled=1
  refused=0`, W4031 t1,t3 = default POST. 34 more cells (8 refused at elaboration in every build): JIT stdout+W4031
  == default on 34 (templates_compiled>=1, refused=0 on 12); product (--no-default-features) == default on 33;
  vl_lines differs = F4004 S3b fatal, identical on the PRE product build (pre-existing: `inc` writes a module var).
- Docs row manual 006 §3.1 (CA t0): k1 vita `f t=0 x=x z=x`,`f t=0 x=0 z=1`; vl and iverilog `f t=0 x=0 z=1` only;
  k2b vita E4003 3:5 t0 rc=1, both oracles silent rc=0; PRE == POST. Row text matches the measurement.
- Staged location side table `stmt_locs` is keyed by StmtId (artifact header v29 note), not span; the arm is the
  only new severity stmt per chain; staged == one-shot on every --staged cell run here.

## Other observations (not findings)
- q3p (parameterized class fn -> `this.t()` class task; illegal: vl FUNCTIMECTL): POST t0 W4031 x2 = PRE-H and PRE
  `unique case` twin -> PLAN u8/u23 over-acceptance class, new shape only.
- q5n nested/cloned arms (do-while clone x2 same span, chain inside a member's then, two macro chains on one line,
  priority/unique/unique0 mix): POST lines/times = vl (inner-qualifier column as documented); staged == one-shot.
- Function-only .vu identity and corpus (0 qualifier-ifs in ~1500 bench sources; picorv32 .vu PRE == POST).
